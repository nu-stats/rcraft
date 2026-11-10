//! P2B.3: Stata's data commands in R.
//!
//! A do-file read by [`crate::reader`] is translated command by command into R
//! that works on one data frame at a time, the way Stata works on the data in
//! memory: `use study.dta` makes `study`, and the commands after it change
//! `study` (dplyr for rows and columns, haven for Stata files and labels, readr
//! and readxl for text and Excel files, tidyr for `reshape`).
//!
//! As with expressions ([`crate::expr_r`]), nothing is guessed. A command (or
//! an option) that R cannot do the same way is not translated and says why;
//! where the R is right but differs in a detail, a caveat says so. The rules
//! followed are Stata's documented ones as StatCraft implements them (checked
//! there against Stata's own output), for example:
//!
//! - `generate` stores a float unless told otherwise, so `gen x = 0.1` holds
//!   0.100000001490116; the translation rounds to float the same way.
//! - Rows that an `if` leaves out get a missing value (`.` or `""`).
//! - A string variable's missing value is `""`, never `NA`.
//! - `drop if` drops rows where the condition is true, and a missing value
//!   counts as true.
//!
//! The data frame's columns are not known when translating (the file is read
//! only when the R runs), so variable types are tracked where the do-file
//! itself makes them (`generate` of text, `encode`, `tostring`) and are
//! otherwise taken as numbers. Abbreviated variable names cannot be resolved
//! without the data and are taken as written.

mod alpha;
mod anova;
mod center;
mod clogit;
mod cluster2;
mod codebook;
mod coefplot;
mod contract;
mod dates;
mod eclass;
mod estpost;
mod esttab;
mod esttable;
mod filefilter;
mod files;
mod fre;
mod graph;
mod iv;
mod margins;
mod matrices;
mod mexpr;
mod ml;
mod model;
mod more;
mod natscale;
mod outreg;
mod outreg2;
mod parmest;
mod post;
mod postfile;
mod putexcel;
mod random;
mod reghdfe;
mod returned;
mod robreg;
mod scalars;
mod st;
mod stats;
mod suest;
mod survwgt;
mod svy;
mod tables;
mod ts;
mod tsreg;
mod weights;
mod winsor;
mod xt;
mod xtregar;

use std::collections::{BTreeMap, BTreeSet};

use crate::expr_r::{self, r_name, r_number, r_text, Kind, Untranslated, Use};
use crate::lang::syntax::Opt;
use crate::reader::{Command, Item, Opener, Prefix, Program, Span};

/// How a Stata line starts in the R script, as a comment above its R.
pub const STATA_COMMENT: &str = "# Stata: ";

/// R for one Stata command.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RCode {
    /// One or more lines of R; empty when the command has nothing to do in R.
    pub code: String,
    /// Details the R does not keep, in words.
    pub caveats: Vec<String>,
    /// R packages the code calls (offered through the Packages window).
    pub packages: Vec<&'static str>,
}

/// One line of the do-file and what it became.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub span: Span,
    /// The Stata line as read.
    pub stata: String,
    pub result: Result<RCode, Untranslated>,
}

/// A whole do-file in R.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Translation {
    /// Small R functions the lines below call (defined once, first).
    pub helpers: Vec<String>,
    pub lines: Vec<Line>,
    /// The R name of the data in memory after the last line, if any.
    pub data: Option<String>,
    /// The value-label sets defined by then.
    pub value_labels: Vec<String>,
    /// The packages `ssc install` asked for, in order (P2B.11).
    pub ssc: Vec<String>,
}

impl Line {
    /// This line in the R script: the Stata as a comment, then its R and its
    /// notes (or the reason it was not translated).
    pub fn script(&self) -> String {
        let mut out = format!("{STATA_COMMENT}{}\n", self.stata.replace('\n', " "));
        match &self.result {
            Ok(r) => {
                if !r.code.is_empty() {
                    out.push_str(&r.code);
                    out.push('\n');
                }
                for c in &r.caveats {
                    out.push_str(&format!("#   note: {c}\n"));
                }
            }
            Err(e) => out.push_str(&format!("#   not translated: {}\n", e.reason)),
        }
        out
    }
}

impl Translation {
    /// The R script: the helpers, then each Stata line as a comment followed by
    /// its R (or the reason it was not translated).
    pub fn script(&self) -> String {
        let mut out = String::new();
        for h in &self.helpers {
            out.push_str(h);
            out.push('\n');
        }
        if !self.helpers.is_empty() {
            out.push('\n');
        }
        for l in &self.lines {
            out.push_str(&l.script());
        }
        out
    }

    /// The R packages the script needs, each once.
    pub fn packages(&self) -> Vec<&'static str> {
        let mut set = BTreeSet::new();
        for l in &self.lines {
            if let Ok(r) = &l.result {
                set.extend(r.packages.iter().copied());
            }
        }
        set.into_iter().collect()
    }
}

/// Translates every command of `program`, in order, as written: blocks
/// (`foreach`, `forvalues`, `if`, `program`, ...) and macros are not run
/// here. [`crate::dofile`] runs those and hands [`Translator`] the commands
/// one by one.
pub fn translate(program: &Program) -> Translation {
    let mut t = Translator::new();
    let mut lines = Vec::new();
    for item in &program.items {
        match item {
            Item::Command(c) => lines.push(t.command(c)),
            Item::Group(g) => lines.push(t.refused(
                g.span,
                opener_text(&g.opener),
                "loops, if blocks and programs are run by the do-file runner (crate::dofile), not here",
            )),
            Item::Problem(p) => lines.push(t.refused(
                p.span,
                String::new(),
                format!("the do-file reader could not read this line: {}", p.message),
            )),
        }
    }
    t.finish(lines)
}

/// The translator, a command at a time (P2C): what a command means depends
/// on the ones before it (the data in memory, the model that left e(), the
/// scalars), so it keeps that as it goes down the do-file.
#[derive(Clone, Default)]
pub struct Translator {
    s: Session,
}

/// R that prints the value of a Stata expression, as `local name = exp` and
/// `if exp {` need it when the do-file runs (P2C).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueCode {
    pub code: String,
    /// The value is text: what R prints is kept as it is. A number is printed
    /// in %18.0g, to be trimmed, which is how Stata keeps it in a macro.
    pub text: bool,
}

impl Translator {
    pub fn new() -> Translator {
        Translator::default()
    }

    /// The next command.
    pub fn command(&mut self, c: &Command) -> Line {
        let result = self.s.command(c);
        if let Err(e) = &result {
            self.s.not_translated(&c.text, &e.reason);
        }
        if c.prefixes.contains(&Prefix::Capture) {
            self.s.rc_lost = result.as_ref().err().map(|_| {
                format!(
                    "_rc is not known: the line under capture before it was not translated ({})",
                    c.text.trim()
                )
            });
        }
        Line {
            span: c.span,
            stata: c.text.clone(),
            result,
        }
    }

    /// `_rc` is no longer known, for the reason given.
    pub fn lose_rc(&mut self, why: String) {
        self.s.rc_lost = Some(why);
    }

    /// For `use file`: the R that prints the `xtset` or `tsset` settings Stata
    /// saved in the .dta (panel, time and delta, by tabs; nothing when none),
    /// when this do-file did not save the file itself. Stata puts them back
    /// when it reads the file.
    pub fn saved_ts_question(&self, c: &Command) -> Option<String> {
        // `use varlist using f` may leave the panel or time out
        if c.name != "use" || c.using.is_some() || c.parts.main.trim().is_empty() {
            return None;
        }
        let path = file_name(&c.parts.main, "dta");
        if self.s.saved.contains_key(&self.s.file_key(&path)) {
            return None;
        }
        Some(format!(
            "cat(local({{\n{}\nstata_dta_ts({})\n}}))",
            ts::DTA_TS_READER,
            r_text(&path)
        ))
    }

    /// `input`, with the data lines that follow it up to `end`.
    pub fn input(&mut self, c: &Command, lines: &[String]) -> Line {
        self.s.input_lines = Some(lines.to_vec());
        let line = self.command(c);
        self.s.input_lines = None;
        line
    }

    /// A line Stata runs that is not translated, for a reason found before
    /// the translator saw it (a macro whose value is not known, a loop over
    /// variables only the data name). What it may have changed is not known,
    /// so the data, e() and r() before it are no longer relied on; only a
    /// `display` is known to leave them alone.
    pub fn refused(&mut self, span: Span, stata: String, reason: impl Into<String>) -> Line {
        let reason = reason.into();
        let word = stata.split_whitespace().next().unwrap_or("");
        let display = word.len() >= 2 && "display".starts_with(word);
        if word.len() >= 3 && "capture".starts_with(word) {
            self.s.rc_lost = Some(format!(
                "_rc is not known: the line under capture before it was not translated ({})",
                stata.trim()
            ));
        }
        self.s.not_translated(&stata, &reason);
        if !display {
            let why = format!("a line before it was not translated: {}", stata.trim());
            self.s.rows_unknown();
            self.s.e_ret = returned::Returned::Lost(why.clone());
            self.s.r_ret = returned::Returned::Lost(why);
        }
        Line {
            span,
            stata,
            result: Err(no(reason)),
        }
    }

    /// R that prints the value of `exp` by Stata's display rules (a variable
    /// stands for its first row). Worked out on a copy, so the do-file's
    /// state is left as it was; the helpers it calls are kept.
    pub fn value_code(&mut self, exp: &str) -> Result<ValueCode, Untranslated> {
        let (code, copy, text) = match self.trial(&format!("display %18.0g ({exp})")) {
            Ok((r, copy)) => (r.code, copy, false),
            Err(first) if first.reason.contains("given text") => {
                let (r, copy) = self.trial(&format!("display ({exp})"))?;
                (r.code, copy, true)
            }
            Err(e) => return Err(e),
        };
        self.s.helpers.extend(copy.s.helpers.iter().copied());
        Ok(ValueCode { code, text })
    }

    /// R that prints what `display rest` prints, which `local x : display
    /// rest` keeps as it is.
    pub fn display_code(&mut self, rest: &str) -> Result<ValueCode, Untranslated> {
        let (r, copy) = self.trial(&format!("display {rest}"))?;
        self.s.helpers.extend(copy.s.helpers.iter().copied());
        Ok(ValueCode {
            code: r.code,
            text: true,
        })
    }

    /// `line` translated on a copy of this translator.
    fn trial(&self, line: &str) -> Result<(RCode, Translator), Untranslated> {
        let mut copy = self.clone();
        let program = crate::reader::read(line);
        match program.items.as_slice() {
            [Item::Command(c)] => copy.s.command(c).map(|r| (r, copy)),
            _ => Err(no("Stata would not read this expression")),
        }
    }

    /// The R name of the data in memory, if there is any.
    /// The command whose results r() holds now, when the translator knows them.
    pub fn r_from(&self) -> Option<&str> {
        match &self.s.r_ret {
            returned::Returned::Kept { cmd, .. } | returned::Returned::Program { cmd, .. } => {
                Some(cmd.as_str())
            }
            _ => None,
        }
    }

    pub fn data(&self) -> Option<&str> {
        self.s.data.as_deref()
    }

    /// The text a value label set the do-file defined gives `value`: the
    /// value itself when the set has no text for it (as `: label set #`);
    /// `None` when the do-file did not define the set.
    pub fn label_text(&self, set: &str, value: f64) -> Option<String> {
        let pairs = self.s.label_sets.get(set)?;
        Some(
            pairs
                .iter()
                .find(|(v, _)| *v == value)
                .map(|(_, t)| t.clone())
                .unwrap_or_else(|| crate::dofile::stata_g(value, 18)),
        )
    }

    /// The helpers the lines so far call: each one's name and its R.
    pub fn helpers(&self) -> Vec<(&'static str, &'static str)> {
        self.s
            .helpers
            .iter()
            .map(|h| (*h, helper_code(h)))
            .collect()
    }

    /// The R of a helper an expression asks for (`RExpr::helpers`).
    pub fn helper_r(name: &str) -> &'static str {
        helper_code(name)
    }

    /// The translation of `lines`, which this translator made, in order.
    pub fn finish(self, lines: Vec<Line>) -> Translation {
        let s = self.s;
        Translation {
            helpers: s
                .helpers
                .iter()
                .map(|h| helper_code(h).to_string())
                .collect(),
            lines,
            data: s.data.clone(),
            value_labels: s.label_sets.keys().cloned().collect(),
            ssc: s.ssc.clone(),
        }
    }
}

fn opener_text(o: &Opener) -> String {
    match o {
        Opener::Foreach { name, .. } => format!("foreach {name} ..."),
        Opener::Forvalues { name, range } => format!("forvalues {name} = {range}"),
        Opener::While { condition } => format!("while {condition}"),
        Opener::If { condition } => format!("if {condition}"),
        Opener::Prefixed => "{".into(),
        Opener::Frame { name } => format!("frame {name}"),
        Opener::Program { name, .. } => format!("program {name}"),
        Opener::Raw { kind, header, .. } => format!("{kind} {header}"),
    }
}

/// The commands a `by` prefix runs group by group (data commands have their
/// own `by`, through `dplyr::group_by`).
const BY_EACH: &[&str] = &[
    "summarize",
    "tabulate",
    "tab1",
    "tab2",
    "correlate",
    "pwcorr",
    "ttest",
    "regress",
    "logit",
    "probit",
    "poisson",
    "nbreg",
    "ologit",
    "oprobit",
    "mlogit",
    "anova",
    "xtreg",
    "ivregress",
];

/// The commands that take weights here; each reads `c.parts.weight` itself
/// (collapse refuses them in its own words).
const WEIGHTED: &[&str] = &[
    "center",
    "contract",
    "svyset",
    "collapse",
    "summarize",
    "tabulate",
    "tab1",
    "tab2",
    "correlate",
    "pwcorr",
    "regress",
    "logit",
    "probit",
    "poisson",
    "nbreg",
    "ologit",
    "oprobit",
    "mlogit",
];

/// The commands that fit a model.
const MODELS: &[&str] = &[
    "svy",
    "asclogit",
    "cluster2",
    "stcox",
    "clogit",
    "robreg",
    "reghdfe",
    "regress",
    "logit",
    "probit",
    "poisson",
    "nbreg",
    "ologit",
    "oprobit",
    "mlogit",
    "anova",
    "xtreg",
    "ivregress",
];

/// Commands that leave the values of existing variables and the set of rows
/// as they were (a new variable, a new order or a label does not matter to
/// `margins`). Anything else may have changed the data.
/// How a line refused for a lost variable says so.
const LOST_VAR: &str = "is not known here: the line that makes it was not translated";

/// The variables a command changes in place, by its `inplace` or `replace`
/// option (`center x, inplace`, `winsor2 x, replace`): when it is not
/// translated their values are not known after it. A `replace` that
/// overwrites a file says nothing about the variables.
fn in_place_vars(c: &Command) -> Vec<String> {
    let opts = Opts(&c.options);
    let writes_file = c.using.is_some()
        || c.options.iter().any(|o| o.is("saving", 3))
        || matches!(
            c.name.as_str(),
            "save"
                | "saveold"
                | "export"
                | "outsheet"
                | "graph"
                | "esttab"
                | "estout"
                | "outreg"
                | "outreg2"
                | "putexcel"
                | "log"
                | "cmdlog"
                | "copy"
                | "estimates"
                | "eststo"
                | "estadd"
                | "label"
                | "notes"
                | "note"
                | "translate"
                | "file"
                | "postfile"
                | "parmest"
                | "coefplot"
                | "marginsplot"
        );
    let in_place = opts.has("inplace", 5) || (opts.has("replace", 7) && !writes_file);
    if !in_place {
        return Vec::new();
    }
    c.parts
        .main
        .split_whitespace()
        .filter(|w| is_name(w))
        .map(str::to_string)
        .collect()
}

/// The variables (or scalar) a generate, egen, predict, replace or scalar line
/// makes or changes.
fn new_vars(c: &Command) -> Vec<String> {
    let main = c.parts.main.split('=').next().unwrap_or("");
    let mut words: Vec<&str> = main.split_whitespace().collect();
    if words.first().is_some_and(|w| storage(w).is_some()) {
        words.remove(0);
    }
    match c.name.as_str() {
        "generate" | "egen" => words
            .first()
            .map(|w| vec![w.to_string()])
            .unwrap_or_default(),
        "predict" => words.iter().map(|w| w.to_string()).collect(),
        // scalar [define] name = exp
        "scalar" if c.parts.main.contains('=') => words
            .iter()
            .find(|w| !matches!(**w, "define" | "scalar"))
            .map(|w| vec![w.to_string()])
            .unwrap_or_default(),
        // its values after a replace that did not run are not known either
        "replace" => words
            .first()
            .map(|w| vec![w.to_string()])
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

/// Commands that replace the data in memory or change its rows; when one is
/// not translated, the lines after it cannot know the data.
const REPLACE_DATA: &[&str] = &[
    "append",
    "bsample",
    "collapse",
    "contract",
    "cross",
    "cwf",
    "drop",
    "duplicates",
    "expand",
    "fillin",
    "frame",
    "frames",
    "gsort",
    "import",
    "infile",
    "infix",
    "input",
    "insheet",
    "joinby",
    "keep",
    "merge",
    "reshape",
    "restore",
    "sample",
    "set",
    "sort",
    "stack",
    "sysuse",
    "use",
    "webuse",
    "xpose",
];

/// Options of every model that change only how Stata's table looks, with the
/// shortest form Stata 19 takes of each.
const DISPLAY_OPTIONS: &[(&str, usize)] = &[
    ("baselevels", 4),
    ("allbaselevels", 7),
    ("noomitted", 6),
    ("vsquish", 7),
    ("noemptycells", 7),
    ("nofvlabel", 7),
    ("cformat", 2),
    ("pformat", 2),
    ("sformat", 2),
    ("nolstretch", 6),
    ("fvwrap", 6),
    ("fvwrapon", 8),
    ("noci", 4),
    ("nopvalues", 9),
    ("noheader", 8),
    ("notable", 7),
    ("coeflegend", 5),
];

/// And those of `margins`: `asobserved` is its default.
const MARGINS_DISPLAY: &[(&str, usize)] = &[("asobserved", 5), ("noatlegend", 10)];

/// Commands that, translated or not, change neither the data nor its rows nor
/// e(): they write a table or a file, or look something up.
const HARMLESS: &[&str] = &["putexcel", "outreg", "frmttable", "which"];

const LEAVE_DATA_ALONE: &[&str] = &[
    "filefilter",
    "mata",
    "outreg",
    "frmttable",
    "which",
    "query",
    "putexcel",
    "estpost",
    "codebook",
    "grstyle",
    "compress",
    "suest",
    "ereturn",
    "return",
    "display",
    "matrix",
    "parmest",
    "fre",
    "outreg2",
    "eststo",
    "estadd",
    "tsset",
    "xtset",
    "estimates",
    "assert",
    "erase",
    "rm",
    "fvset",
    "histogram",
    "kdensity",
    "scatter",
    "line",
    "connected",
    "twoway",
    "graph",
    "summarize",
    "preserve",
    "tabulate",
    "correlate",
    "tab1",
    "tab2",
    "pwcorr",
    "ttest",
    "list",
    "describe",
    "count",
    "levelsof",
    "test",
    "lincom",
    "estat",
    "margins",
    "predict",
    "generate",
    "sort",
    "gsort",
    "order",
    "label",
    "set",
    "version",
    "save",
    "saveold",
];

fn no(reason: impl Into<String>) -> Untranslated {
    Untranslated {
        reason: reason.into(),
    }
}

/// `import delimited`'s reading: a line that is empty is skipped, as in Stata, but
/// one of delimiters alone is a row of missing values (readr would skip both).
/// A column's type comes from every row, as Stata 19 scans the whole file (a
/// column of numbers with "NA" at row 6,583 is a string); readr would look at
/// the first 1,000 and make the late text missing.
const READ_DELIM_HELPER: &str = r#"stata_read_delim <- function(file, ..., guess_max = NULL) {
  lines <- readLines(file, warn = FALSE)
  lines <- lines[nzchar(sub("\r$", "", lines))]
  if (is.null(guess_max)) guess_max <- max(1000, length(lines))
  readr::read_delim(I(paste0(lines, collapse = "\n")), ..., guess_max = guess_max, skip_empty_rows = FALSE)
}"#;

/// R's version of Stata's float storage: a double rounded to the nearest
/// single-precision value, missing kept as it is (`.a` to `.z` too, as a
/// copy into a float keeps them in Stata 19); too large for a float is `.`.
const FLOAT_HELPER: &str = "stata_float <- function(x) {\n  x <- as.numeric(x)\n  y <- readBin(writeBin(x, raw(), size = 4), \"double\", size = 4, n = length(x))\n  y[!is.finite(y)] <- NA\n  y[is.na(x)] <- x[is.na(x)]\n  y\n}";

/// Stata's order of missing values as a number, for comparisons with one:
/// 0 a number, 1 for `.`, 2 to 27 for `.a` to `.z` (haven's tagged NAs).
const MV_HELPER: &str = r#"stata_mv <- function(x) {
  k <- numeric(length(x))
  m <- is.na(x)
  if (!any(m)) return(k)
  tag <- if (is.double(x)) haven::na_tag(as.vector(unclass(x))[m]) else rep(NA_character_, sum(m))
  k[m] <- ifelse(is.na(tag), 1, match(tag, letters) + 1)
  k
}"#;

/// What Stata's arithmetic and functions give for a missing value: `.`,
/// where R would keep the tag of `.a` to `.z`.
const UNTAG_HELPER: &str = r#"stata_untag <- function(x) {
  if (is.double(x) && anyNA(x)) x[is.na(x)] <- NA_real_
  x
}"#;

/// `x op y` in Stata's order (numbers < `.` < `.a` < ... < `.z`), for two
/// values that may both be missing.
const COMPARE_HELPER: &str = r#"stata_compare <- function(x, op, y) {
  kx <- stata_mv(x)
  ky <- stata_mv(y)
  d <- ifelse(kx == ky, ifelse(kx == 0, sign(as.numeric(x) - as.numeric(y)), 0), kx - ky)
  match.fun(op)(d, 0)
}"#;

/// One value per row that tells apart rows whose missing values differ
/// (`.`, `.a` to `.z`): grouped on beside the columns themselves, as dplyr
/// takes every tagged NA for the same NA, so each is a group of its own as in
/// Stata 19 (`by`, `egen, by()`, `collapse, by()`, `duplicates`).
const TAG_KEY_HELPER: &str = r#"stata_tag_key <- function(d) {
  k <- lapply(Filter(function(v) is.double(v) && anyNA(v), as.list(d)), stata_mv)
  if (length(k) == 0) return(rep("", nrow(d)))
  do.call(paste, unname(k))
}"#;

/// The columns of `d` as keys in Stata's order: each number column is
/// followed by the place of its missing values (`stata_mv`), so `.` comes
/// before `.a` and `.a` before `.z` (Stata 19's sort, gsort, egen group()).
const SORT_KEYS_HELPER: &str = r#"stata_sort_keys <- function(d) {
  k <- list()
  for (v in d) {
    k[[length(k) + 1]] <- v
    if (is.double(v) && anyNA(v)) k[[length(k) + 1]] <- stata_mv(v)
  }
  names(k) <- paste0("k", seq_along(k))
  vctrs::new_data_frame(k, n = nrow(d))
}"#;

/// `strmatch(s, pattern)` as Stata 19: the whole string, `*` any characters
/// (new lines too), `?` one, every other character itself (`\` and `[` too).
const STRMATCH_HELPER: &str = r#"stata_strmatch <- function(s, p) {
  if (!is.character(s) || !is.character(p)) stop("type mismatch (Stata: r(109))", call. = FALSE)
  rx <- function(q) {
    ch <- strsplit(q, "")[[1]]
    one <- vapply(ch, function(c) if (c == "*") ".*" else if (c == "?") "." else sprintf("\\x{%x}", utf8ToInt(c)), "")
    paste0("(?s)^", paste0(one, collapse = ""), "$")
  }
  n <- max(length(s), length(p))
  s <- rep_len(s, n)
  p <- rep_len(p, n)
  as.numeric(vapply(seq_len(n), function(i) grepl(rx(p[i]), s[i], perl = TRUE), TRUE))
}"#;

/// `char(n)` as Stata 19: 1 to 127 that character (n truncated), missing or
/// outside 0 to 255 ""; 0 and 128 to 255 are single bytes R's text does not
/// hold as Stata does, so they stop.
const CHAR_HELPER: &str = r#"stata_char <- function(n) {
  vapply(trunc(n), function(k) {
    if (is.na(k) || k < 0 || k > 255) return("")
    if (k == 0 || k > 127) stop("char(", k, "): a single byte, which is not translated yet", call. = FALSE)
    intToUtf8(k)
  }, "")
}"#;

/// A column with decimals rounded to float, as `import delimited` stores it
/// (whole numbers stay as they are: Stata keeps them exactly).
const IMPORT_FLOAT_HELPER: &str = "stata_import_float <- function(d) {\n  for (n in names(d)) {\n    v <- d[[n]]\n    if (is.numeric(v) && any(!is.na(v) & v != trunc(v))) d[[n]] <- stata_float(v)\n  }\n  d\n}";

/// `levelsof`'s values as Stata 19 writes them: sorted, numbers in %18.0g
/// (missing left out, or `.` last with `missing`), text in compound quotes
/// (`""` first with `missing`; `clean` without quotes); r(r) the number of
/// values and r(N) the rows that count.
/// `expand`: each row's copies after all the rows, in order, as Stata 19 puts
/// them; a count is rounded, and one below 1 or missing leaves the row once.
const EXPAND_HELPER: &str = r#"stata_expand <- function(d, n, gen = NULL) {
  n <- floor(n + 0.5)
  n[is.na(n) | n < 1] <- 1
  extra <- rep(seq_len(nrow(d)), n - 1)
  out <- d[c(seq_len(nrow(d)), extra), , drop = FALSE]
  rownames(out) <- NULL
  if (!is.null(gen)) out[[gen]] <- c(rep(0, nrow(d)), rep(1, length(extra)))
  out
}"#;

/// `set obs n`: the data made longer to n rows, the new ones missing (`""`
/// in a string variable); with no data, n rows and no variables.
const SET_OBS_HELPER: &str = r#"stata_set_obs <- function(d, n) {
  if (is.null(d)) return(structure(list(), names = character(0), row.names = c(NA, -as.integer(n)), class = "data.frame"))
  old <- nrow(d)
  if (n < old) stop("set obs ", n, ": the data has ", old, " rows, and set obs does not drop any")
  if (n == old) return(d)
  d <- d[c(seq_len(old), rep(NA_integer_, n - old)), , drop = FALSE]
  rownames(d) <- NULL
  for (v in names(d)) if (is.character(d[[v]])) d[[v]][(old + 1):n] <- ""
  d
}"#;

/// `levelsof`; with `missing` and `tags`, each of `.` and `.a` to `.z` found
/// is a level of its own, after the numbers (Stata 19: `1 . .a .d`).
const LEVELSOF_HELPER: &str = r#"stata_levelsof <- function(x, clean = FALSE, sep = " ", missing = FALSE, tags = FALSE) {
  if (is.character(x)) {
    x[is.na(x)] <- ""
    keep <- if (missing) rep(TRUE, length(x)) else x != ""
    v <- sort(unique(x[keep]), method = "radix")
    s <- if (clean) v else paste0("`\"", v, "\"'")
  } else {
    x <- as.numeric(x)
    keep <- if (missing) rep(TRUE, length(x)) else !is.na(x)
    v <- sort(unique(x[!is.na(x)]))
    s <- vapply(v, function(z) stata_fmt(z, "%18.0g", trim = TRUE), "")
    if (missing && anyNA(x)) {
      s <- c(s, if (tags) c(".", paste0(".", letters))[sort(unique(stata_mv(x[is.na(x)])))] else ".")
    }
  }
  list(levels = paste(s, collapse = sep), r = length(s), N = sum(keep))
}"#;

/// Names for Excel columns, A to Z, AA, AB ..., from the `first`-th column.
const EXCEL_NAMES_HELPER: &str = "stata_excel_names <- function(n, first = 1) {\n  vapply(seq_len(n) + first - 1, function(i) {\n    s <- \"\"\n    while (i > 0) {\n      s <- paste0(LETTERS[(i - 1) %% 26 + 1], s)\n      i <- (i - 1) %/% 26\n    }\n    s\n  }, \"\")\n}";

/// What `capture` makes of an error in R: Stata's `_rc` and, under
/// `noisily`, Stata's message. A name that is not there is `111` and "name
/// not found" in Stata, as is a helper's "... not found"; "already defined"
/// is `110`, a file not found `601`, one that already exists `602` and a
/// false assertion `9` (read off Stata 19). Any other error keeps R's
/// message and leaves `_rc` missing.
const CAPTURED_HELPER: &str = r#"stata_captured <- function(e, noisily) {
  while (inherits(e$parent, "condition")) e <- e$parent
  m <- sub("^! ", "", conditionMessage(e))
  rc <- NA_real_
  # RCraft's own messages that name Stata's return code: Stata's words alone
  code <- " \\(Stata: r\\(([0-9]+)\\)\\)$"
  if (grepl(code, m)) {
    rc <- as.numeric(sub(paste0("^.*", code), "\\1", m))
    m <- sub(code, "", m)
  } else if (grepl("^file [^\n]* not found$", m)) {
    rc <- 601
  } else if (grepl("^file [^\n]* already exists$", m)) {
    rc <- 602
  } else if (grepl("(^|\n)assertion is false$", m)) {
    rc <- 9
  } else if (m == "not sorted") {
    rc <- 5
  } else if (grepl("^after merge, not all observations matched", m)) {
    rc <- 9
  } else if (grepl("does not uniquely identify observations in the (master|using) data$", m)) {
    rc <- 459
  } else if (grepl("^(repeated time values|time variable must contain only integer|time values with period less than delta)", m)) {
    rc <- 451
  } else if (grepl("^string variables not allowed in varlist", m)) {
    rc <- 109
  } else if (m == "groups (strata) are not nested within clusters") {
    rc <- 459
  } else if (grepl("^object '[^']+' not found$", m)) {
    m <- sub("^object '([^']+)' not found$", "\\1 not found", m)
    rc <- 111
  } else if (grepl("^[^\n]* not found$", m)) {
    rc <- 111
  } else if (grepl("^variable [^\n]* already defined$", m)) {
    rc <- 110
  }
  if (noisily) cat(m, "\n", sep = "")
  rc
}"#;

/// `assert` on the rows it covers: `ok` is TRUE where the expression holds.
const ASSERT_HELPER: &str = r#"stata_assert <- function(ok, rc0 = FALSE) {
  n <- length(ok)
  if (n == 0) {
    cat("(null assertion)\n")
    return(invisible())
  }
  bad <- sum(is.na(ok) | !ok)
  if (bad == 0) return(invisible())
  m <- "assertion is false"
  if (n > 1) {
    m <- paste0(format(bad, big.mark = ","), if (bad == 1) " contradiction" else " contradictions",
      " in ", format(n, big.mark = ","), " observations\n", m)
  }
  if (rc0) cat(m, "\n", sep = "") else stop(m, call. = FALSE)
  invisible()
}"#;

fn helper_code(name: &str) -> &'static str {
    match name {
        "stata_captured" => CAPTURED_HELPER,
        "stata_assert" => ASSERT_HELPER,
        "stata_excel_names" => EXCEL_NAMES_HELPER,
        "stata_stats" => more::STAT_HELPERS,
        "stata_mv_stats" => more::MV_STAT_HELPERS,
        "stata_destring" => more::DESTRING_HELPER,
        "stata_forced" => more::FORCED_HELPER,
        "stata_same_dates" => more::SAME_DATES_HELPER,
        "stata_merge" => more::MERGE_HELPER,
        "stata_label_set" => more::DTA_LABEL_SET_HELPER,
        "stata_xpose" => more::XPOSE_HELPER,
        "stata_coefplot" => coefplot::COEFPLOT_HELPER,
        "stata_estpost" => estpost::ESTPOST_HELPER,
        "stata_esttab" => esttab::ESTTAB_HELPER,
        "stata_read_delim" => READ_DELIM_HELPER,
        "stata_strmatch" => STRMATCH_HELPER,
        "stata_char" => CHAR_HELPER,
        "stata_mv" => MV_HELPER,
        "stata_untag" => UNTAG_HELPER,
        "stata_compare" => COMPARE_HELPER,
        "stata_sort_keys" => SORT_KEYS_HELPER,
        "stata_tag_key" => TAG_KEY_HELPER,
        "stata_insheet" => more::INSHEET_HELPER,
        "stata_split" => more::SPLIT_HELPER,
        "stata_confirm" => more::CONFIRM_HELPER,
        "stata_post" => postfile::POST_HELPER,
        "stata_file" => files::FILE_HELPER,
        "stata_tostring" => more::TOSTRING_HELPER,
        "stata_summarize" => stats::SUMMARIZE_HELPER,
        "stata_tab_values" => stats::TAB_VALUES_HELPER,
        "stata_tabulate" => stats::TABULATE_HELPER,
        "stata_tab_generate" => stats::TAB_GENERATE_HELPER,
        "stata_correlate" => stats::CORRELATE_HELPER,
        "stata_tab1" => stats::TAB1_HELPER,
        "stata_tab2" => stats::TAB2_HELPER,
        "stata_pwcorr" => stats::PWCORR_HELPER,
        "stata_by_groups" => stats::BY_GROUPS_HELPER,
        "stata_wrows" => weights::WROWS_HELPER,
        "stata_wcov" => weights::WCOV_HELPER,
        "stata_lm_wvcov" => weights::LM_WVCOV_HELPER,
        "stata_coefs" => model::COEFS_HELPER,
        "stata_ib" => model::IB_HELPER,
        "stata_dates" => dates::DATES_HELPER,
        "stata_collinear" => model::COLLINEAR_HELPER,
        "stata_slice" => model::SLICE_HELPER,
        "stata_fit_rows" => model::FIT_ROWS_HELPER,
        "stata_glm_vcov" => ml::GLM_VCOV_HELPER,
        "stata_nbreg_vcov" => ml::NBREG_VCOV_HELPER,
        "stata_ml_header" => ml::ML_HEADER_HELPER,
        "stata_multinom_b" => ml::MULTINOM_B_HELPER,
        "stata_probit_newton" => ml::PROBIT_NEWTON_HELPER,
        "stata_perfect" => ml::PERFECT_HELPER,
        "stata_ml_sandwich" => ml::ML_SANDWICH_HELPER,
        "stata_nbreg_lr" => ml::NBREG_LR_HELPER,
        "stata_outcome_probs" => post::OUTCOME_PROBS_HELPER,
        "stata_predict_stat" => post::PREDICT_STAT_HELPER,
        "stata_predict_more" => post::PREDICT_MORE_HELPER,
        "stata_names" => post::NAMES_HELPER,
        "stata_test" => post::TEST_HELPER,
        "stata_lincom" => post::LINCOM_HELPER,
        "stata_zero_term" => post::ZERO_TERM_HELPER,
        "stata_estat" => post::ESTAT_HELPER,
        "stata_margins" => margins::MARGINS_HELPER,
        "stata_unconditional" => margins::UNCONDITIONAL_HELPER,
        "stata_anova" => anova::ANOVA_HELPER,
        "stata_clogit" => clogit::CLOGIT_HELPER,
        "stata_robreg" => robreg::ROBREG_HELPER,
        "stata_reghdfe" => reghdfe::REGHDFE_HELPER,
        "stata_cluster2" => cluster2::CLUSTER2_HELPER,
        "stata_asclogit" => clogit::ASCLOGIT_HELPER,
        "stata_svy" => svy::SVY_HELPER,
        "stata_survwgt" => survwgt::SURVWGT_HELPER,
        "stata_isid" => more::ISID_HELPER,
        "stata_contract" => contract::CONTRACT_HELPER,
        "stata_recast" => contract::RECAST_HELPER,
        "stata_suest" => suest::SUEST_HELPER,
        "stata_est_table" => esttable::EST_TABLE_HELPER,
        "stata_codebook" => codebook::CODEBOOK_HELPER,
        "stata_center" => center::CENTER_HELPER,
        "stata_winsor" => winsor::WINSOR_HELPER,
        "stata_alpha" => alpha::ALPHA_HELPER,
        "stata_outreg" => outreg::OUTREG_HELPER,
        "stata_filefilter" => filefilter::FILEFILTER_HELPER,
        "stata_putexcel" => putexcel::PUTEXCEL_HELPER,
        "stata_outreg2" => outreg2::OUTREG2_HELPER,
        "stata_fre" => fre::FRE_HELPER,
        "stata_parmest" => parmest::PARMEST_HELPER,
        "stata_xtile" => more::XTILE_HELPER,
        "stata_tsset" => ts::TSSET_HELPER,
        "stata_ts" => ts::TS_HELPER,
        "stata_rows_replace" => ts::ROWS_REPLACE_HELPER,
        "stata_xtset" => xt::XTSET_HELPER,
        "stata_xtreg" => xt::XTREG_HELPER,
        "stata_xtregar" => xtregar::XTREGAR_HELPER,
        "stata_tsreg" => tsreg::TSREG_HELPER,
        "stata_xt_predict" => xt::XT_PREDICT_HELPER,
        "stata_xt_ic" => xt::XT_IC_HELPER,
        "stata_xt_other" => xt::XT_OTHER_HELPER,
        "stata_ivregress" => iv::IV_HELPER,
        "stata_stset" => st::STSET_HELPER,
        "stata_graph" => graph::GRAPH_HELPER,
        "stata_combine" => graph::COMBINE_HELPER,
        "stata_stcox" => st::STCOX_HELPER,
        "stata_phtest" => st::PHTEST_HELPER,
        "stata_returned" => returned::RETURNED_HELPER,
        "stata_fmt" => returned::FORMAT_HELPER,
        "stata_ereturn" => returned::ERETURN_HELPER,
        "stata_eclass" => eclass::ECLASS_HELPER,
        "stata_ttest_r" => returned::TTEST_R_HELPER,
        "stata_eb" => matrices::EB_HELPER,
        "stata_scalar" => scalars::SCALAR_HELPER,
        "stata_tables" => tables::TABLES_HELPER,
        "stata_import_float" => IMPORT_FLOAT_HELPER,
        "stata_levelsof" => LEVELSOF_HELPER,
        "stata_set_obs" => SET_OBS_HELPER,
        "stata_expand" => EXPAND_HELPER,
        "stata_matrix_list" => matrices::MATRIX_LIST_HELPER,
        "stata_matrix" => mexpr::MATRIX_HELPER,
        "stata_svmat" => matrices::SVMAT_HELPER,
        "stata_rng" => random::RNG_HELPER,
        "stata_drawnorm" => random::DRAWNORM_HELPER,
        "stata_bootstrap" => random::BOOTSTRAP_HELPER,
        "stata_estat_bootstrap" => random::ESTAT_BOOTSTRAP_HELPER,
        _ => FLOAT_HELPER,
    }
}

/// What the translator knows while going down the do-file.
#[derive(Default, Clone)]
struct Session {
    /// The R name of the data frame "in memory".
    data: Option<String>,
    /// Why `_rc` is not known: the last line under capture was not translated.
    rc_lost: Option<String>,
    /// Why the data is not known: a line that replaces it or changes its rows
    /// was not translated.
    data_lost: Option<String>,
    /// Variables a line that was not translated would have made, and that line.
    lost_vars: BTreeMap<String, String>,
    /// The file it came from (for `save` without a name).
    file: Option<String>,
    /// Variable types made by the do-file itself.
    kinds: BTreeMap<String, Kind>,
    /// The base levels `fvset base` chose: variable -> level.
    fv_base: BTreeMap<String, String>,
    /// What a file this do-file saved keeps with its data, as Stata does:
    /// the `fvset` bases and the value label sets.
    saved: BTreeMap<String, Kept>,
    /// The folder the last `cd` named, when it is known (for `saved`'s keys).
    folder: Option<String>,
    /// Scalars defined so far, number or text.
    scalars: BTreeMap<String, Kind>,
    /// The scalars the command being translated reads by their bare names.
    scalars_read: std::cell::RefCell<Vec<String>>,
    /// Value label sets defined so far: name -> (value, text).
    label_sets: BTreeMap<String, Vec<(f64, String)>>,
    /// The model last fitted, for the commands that read it afterwards.
    estimates: Option<model::Estimates>,
    /// The models `estimates store` kept, by name.
    stored_fits: BTreeMap<String, returned::StoredFit>,
    /// the models eststo stored, in order, for esttab without names
    eststo_names: Vec<String>,
    /// The matrices the do-file made (`matrix A = ...`), kept in R as stata_mat.
    user_mats: BTreeSet<String>,
    /// Matrices `capture ereturn post` may have moved into e(): R says
    /// whether they are still there.
    maybe_moved: BTreeSet<String>,
    /// Matrices a line that was not translated may have changed: name -> that line.
    lost_mats: BTreeMap<String, String>,
    /// outreg tables a line that was not translated may have stored: name -> that line.
    outreg_lost: BTreeMap<String, String>,
    /// The files `file open` opened and not closed: handle -> binary.
    file_handles: BTreeMap<String, bool>,
    /// Handles a `file open` that was not translated may have opened.
    file_lost: BTreeSet<String>,
    /// The postfiles open (`postfile h ...`).
    post_handles: BTreeSet<String>,
    /// The postfiles a declined `post` line leaves open or closed, not known which.
    post_lost: BTreeSet<String>,
    /// The postfiles known to be closed (postclose, postutil clear).
    post_closed: BTreeSet<String>,
    /// After asclogit: its variables, alternatives variable and cluster, for estat summarize.
    asclogit_vars: Option<(Vec<String>, String, Option<String>)>,
    /// svyset: the PSU variable ("" for each row its own) and the pweight variable.
    svy_design: Option<(String, String)>,
    /// While svy: runs its model: the design, the line as Stata keeps it, the model alone.
    svy_call: Option<((String, String), String, String)>,
    /// The model in e() was fitted under svy (margins weight its rows).
    svy_fit: bool,
    /// The storage types of the numeric variables `generate` made, so a
    /// `replace` stores as Stata does (a float stays a float).
    storage: BTreeMap<String, Storage>,
    /// A line not translated may have written files (save, export, using ...):
    /// which files are there is not known after it.
    files_unknown: Option<String>,
    /// The data lines of the `input` being translated.
    input_lines: Option<Vec<String>>,
    /// The programs being run, innermost last: name, class, and the r()
    /// names an r-class one has returned so far (number, text).
    programs: Vec<returned::ProgramRun>,
    /// A line that was not translated may have stored a model too (eststo,
    /// a program), so a name missing from `stored_fits` may be in Stata.
    stores_lost: bool,
    /// Why there is no model to read: a line since the last one was not
    /// translated, and it may have fitted one in Stata.
    estimates_lost: Option<String>,
    /// A command since that model may have changed the data, so the copy of
    /// its estimation sample kept in R is no longer what Stata would use.
    changed_since_fit: bool,
    /// The model last fitted had weights (margins and estat do not take them yet).
    weighted_fit: bool,
    /// And they were frequency weights.
    fit_fweight: bool,
    /// The last regress was plain: no robust, clustered or weighted errors.
    plain_regress: bool,
    /// `stata_esample` holds the last model's e(sample) and the rows have not
    /// moved since, so `e(sample)` can be translated.
    esample: bool,
    /// A model this command fitted left `stata_esample`.
    esample_new: bool,
    /// The last model had robust or clustered errors (Stata allows
    /// margins, vce(unconditional) only then), and which.
    fit_robust: bool,
    fit_cluster: bool,
    /// The panel variable `xtset` named.
    panel: Option<String>,
    /// What stset declared.
    stset: Option<st::StSet>,
    /// A graph was drawn, so `graph export` has one to write.
    graph_drawn: bool,
    /// an extended missing value (.a to .z) was stored (R keeps it as a
    /// tagged NA, which fre and tabulate do not tell apart yet)
    extended_missing: std::cell::Cell<bool>,
    /// The data may hold extended missing values: a `.dta` file was read or
    /// one was stored. Comparisons with a missing value then tell them apart
    /// and computed values are made `.` (`expr_r::translate_drawing`).
    tags: std::cell::Cell<bool>,
    /// the R function that maps the second y axis onto the first, per graph
    y2_fn: String,
    y2_count: usize,
    /// And its time variable.
    panel_time: Option<String>,
    /// The name the active results are stored under (`eststo`, `estimates
    /// store` or `restore`), so `estadd` changes the stored copy too.
    active_store: Option<String>,
    /// e() names `estadd local` made text.
    e_text: BTreeSet<String>,
    /// The `delta()` of `tsset` or `xtset`, as R (`None`: 1).
    ts_delta: Option<String>,
    /// The data came from a file this do-file did not save, so whether it
    /// was `tsset` is not known.
    ts_unknown: bool,
    /// The columns the last model's time-series operators added to its
    /// data (`dplyr::mutate()`'s arguments), for `predict`.
    fit_ts: Option<String>,
    /// Who left the e() and r() an expression would read (P2B.6).
    e_ret: returned::Returned,
    r_ret: returned::Returned,
    /// This command left r(): the names it keeps, and its name.
    r_new: Option<(&'static [&'static str], String)>,
    /// This command left e() (`keep_e`).
    e_new: bool,
    helpers: BTreeSet<&'static str>,
    /// The packages `ssc install` asked for, in order.
    ssc: Vec<String>,
    /// What `preserve` kept, for `restore`.
    preserved: Option<Box<Preserved>>,
    /// The command being translated asked whether it is quiet, so it hides
    /// its own output.
    quiet_asked: std::cell::Cell<bool>,
    /// The command being translated takes Stata's random draws.
    drew: std::cell::Cell<bool>,
    /// Why the generator's state is not Stata's: a line that may have drawn
    /// was not translated (until the next `set seed`).
    rng_lost: Option<String>,
    /// Why the draws are not mt64's: a `version` before 14.
    rng_old: Option<String>,
}

/// Whether a line prints nothing, as Stata decides: the innermost of
/// quietly, noisily and capture (which is quiet) wins, so capture noisily
/// prints and noisily quietly does not.
fn silent(c: &Command) -> bool {
    let mut quiet = false;
    for p in &c.prefixes {
        match p {
            Prefix::Quietly | Prefix::Capture => quiet = true,
            Prefix::Noisily => quiet = false,
            _ => {}
        }
    }
    quiet
}

/// What a saved file keeps beside its values that the translator follows.
#[derive(Default, Clone)]
struct Kept {
    fv_base: BTreeMap<String, String>,
    label_sets: BTreeMap<String, Vec<(f64, String)>>,
    panel: Option<String>,
    panel_time: Option<String>,
    ts_delta: Option<String>,
}

/// The data and what the translator knows of it, as `preserve` left them.
#[derive(Default, Clone)]
struct Preserved {
    data: Option<String>,
    file: Option<String>,
    kinds: BTreeMap<String, Kind>,
    fv_base: BTreeMap<String, String>,
    label_sets: BTreeMap<String, Vec<(f64, String)>>,
    panel: Option<String>,
    panel_time: Option<String>,
    ts_delta: Option<String>,
    ts_unknown: bool,
    stset: Option<st::StSet>,
}

/// Collects one command's R.
#[derive(Default)]
struct Out {
    code: Vec<String>,
    caveats: Vec<String>,
    packages: Vec<&'static str>,
    /// R helpers the line's expressions call.
    helpers: Vec<&'static str>,
}

impl Out {
    fn line(&mut self, l: impl Into<String>) {
        self.code.push(l.into());
    }
    fn caveat(&mut self, c: impl Into<String>) {
        let c = c.into();
        if !self.caveats.contains(&c) {
            self.caveats.push(c);
        }
    }
    fn package(&mut self, p: &'static str) {
        if !self.packages.contains(&p) {
            self.packages.push(p);
        }
    }
    fn done(self) -> Result<RCode, Untranslated> {
        Ok(RCode {
            code: self.code.join("\n"),
            caveats: self.caveats,
            packages: self.packages,
        })
    }
}

/// The options of a command, found by name with Stata's shortest abbreviation.
struct Opts<'a>(&'a [Opt]);

impl Opts<'_> {
    fn has(&self, full: &str, min: usize) -> bool {
        self.0.iter().any(|o| o.is(full, min))
    }
    fn arg(&self, full: &str, min: usize) -> Option<String> {
        self.0.iter().find(|o| o.is(full, min)).map(Opt::arg)
    }
    /// Options other than those listed (to refuse what is not translated).
    fn other(&self, known: &[(&str, usize)]) -> Option<String> {
        self.0
            .iter()
            .find(|o| !known.iter().any(|(f, m)| o.is(f, *m)))
            .map(|o| o.name.clone())
    }
}

fn first_word(s: &str) -> (&str, &str) {
    let s = s.trim();
    match s.find(char::is_whitespace) {
        Some(i) => (s.get(..i).unwrap_or(s), s.get(i..).unwrap_or("").trim()),
        None => (s, ""),
    }
}

/// A file name as written (quotes removed), with `ext` added when it has none.
/// A path from the top: `/x`, `C:/x`, `~/x`.
fn is_absolute(path: &str) -> bool {
    let b = path.as_bytes();
    path.starts_with('/')
        || path.starts_with('~')
        || (b.len() >= 2
            && b.first().is_some_and(u8::is_ascii_alphabetic)
            && b.get(1) == Some(&b':'))
}

impl Session {
    /// The key a file has in `saved`: its path from the top when the folder
    /// `cd` named is known, so `save f` after `cd d` is the `use d/f` later.
    fn file_key(&self, path: &str) -> String {
        let p = match &self.folder {
            Some(f) if !is_absolute(path) => format!("{f}/{path}"),
            _ => path.to_string(),
        };
        // Windows and macOS do not tell case apart in a file name
        p.replace("/./", "/").replace("//", "/").to_lowercase()
    }
}

fn file_name(written: &str, ext: &str) -> String {
    // quotes, and compound quotes (`"a b.dta"') around it
    let f = written
        .trim()
        .trim_matches(|c: char| c == '"' || c == '`' || c == '\'');
    // R takes / as the folder separator everywhere; Stata on Windows takes \ too
    let f = f.trim().replace('\\', "/");
    let has_ext = std::path::Path::new(&f)
        .extension()
        .is_some_and(|e| !e.is_empty());
    if has_ext || ext.is_empty() {
        f
    } else {
        format!("{f}.{ext}")
    }
}

/// An R name for a data frame read from `file` (its name without folders and
/// extension, made syntactic).
fn frame_name(file: &str) -> String {
    let stem = std::path::Path::new(file)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("data");
    let mut n: String = stem
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    // not a name R reserves (`repeat.dta` once became `repeat <- ...`), nor
    // one of the translation's own `stata_` objects
    const RESERVED: &[&str] = &[
        "if", "else", "repeat", "while", "function", "for", "in", "next", "break",
    ];
    if !n.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
        || RESERVED.contains(&n.as_str())
        || n.starts_with("stata_")
    {
        n = format!("data_{n}");
    }
    n
}

/// A varlist as tidyselect: names, `a-c` ranges, `x*`/`x?`/`x~` patterns,
/// `_all` and `*`.
fn select(varlist: &str) -> Result<String, Untranslated> {
    let mut parts = Vec::new();
    for tok in varlist.split_whitespace() {
        if tok == "_all" || tok == "*" {
            parts.push("dplyr::everything()".to_string());
        } else if tok.contains(['*', '?', '~']) {
            let mut re = String::from("^");
            for c in tok.chars() {
                match c {
                    '*' | '~' => re.push_str(".*"),
                    '?' => re.push('.'),
                    c if c.is_ascii_alphanumeric() || c == '_' => re.push(c),
                    c => return Err(no(format!("the variable name pattern {tok} has a `{c}`"))),
                }
            }
            re.push('$');
            parts.push(format!("dplyr::matches({})", r_text(&re)));
        } else if let Some((a, b)) = tok.split_once('-') {
            if a.is_empty() || b.is_empty() {
                return Err(no(format!("{tok} is not a variable range")));
            }
            parts.push(format!("{}:{}", r_name(a), r_name(b)));
        } else if is_name(tok) {
            parts.push(r_name(tok));
        } else {
            return Err(no(format!("{tok} is not a variable name")));
        }
    }
    match parts.len() {
        0 => Err(no("no variables are named")),
        1 => Ok(parts.join("")),
        _ => Ok(format!("c({})", parts.join(", "))),
    }
}

/// Plain names only (no patterns), for commands that need each name.
fn names(varlist: &str) -> Result<Vec<String>, Untranslated> {
    let mut out = Vec::new();
    for tok in varlist.split_whitespace() {
        if !is_name(tok) {
            return Err(no(format!(
                "{tok}: only plain variable names are translated here, not patterns or ranges"
            )));
        }
        out.push(tok.to_string());
    }
    if out.is_empty() {
        return Err(no("no variables are named"));
    }
    Ok(out)
}

fn is_name(t: &str) -> bool {
    t.chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && t.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// `in` as a condition on the row number: `in 5`, `in 1/10`, `in -5/l`, `in f/l`.
fn in_range(text: &str) -> Result<String, Untranslated> {
    let at = |t: &str| -> Result<String, Untranslated> {
        let t = t.trim();
        match t {
            "f" | "F" => Ok("1".into()),
            "l" | "L" => Ok("dplyr::n()".into()),
            _ => match t.parse::<i64>() {
                Ok(k) if k > 0 => Ok(k.to_string()),
                Ok(k) if k < 0 => Ok(format!("(dplyr::n() - {})", -k - 1)),
                _ => Err(no(format!("`in {text}` is not a row range"))),
            },
        }
    };
    match text.split_once('/') {
        Some((a, b)) => Ok(format!(
            "dplyr::between(dplyr::row_number(), {}, {})",
            at(a)?,
            at(b)?
        )),
        None => Ok(format!("dplyr::row_number() == {}", at(text)?)),
    }
}

/// A Stata numeric literal (with `.` and `.a`-`.z` as missing).
fn number_literal(t: &str) -> Option<String> {
    let t = t.trim();
    if t == "."
        || (t.len() == 2
            && t.starts_with('.')
            && t.chars().nth(1).is_some_and(|c| c.is_ascii_lowercase()))
    {
        return Some("NA_real_".into());
    }
    t.parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
        .map(r_number)
}

impl Session {
    fn kind_of(&self, name: &str) -> Option<Kind> {
        self.kinds.get(name).copied()
    }

    fn expr(&self, text: &str, use_: Use, out: &mut Out) -> Result<String, Untranslated> {
        self.esample_ok(text)?;
        self.rc_ok(text)?;
        let kinds = |n: &str| self.kind_of(n);
        let ret = |w: &str, n: &str| self.returned_value(w, n);
        let r = expr_r::translate_drawing(text, &kinds, &ret, use_, None, self.tags.get())?;
        Ok(self.took(r, out).0)
    }

    /// An expression's caveats and helpers into `out`; one that stores an
    /// extended missing value turns on the tags (haven's tagged NAs).
    fn took(&self, r: expr_r::RExpr, out: &mut Out) -> (String, Kind) {
        if r.tagged {
            self.extended_missing.set(true);
            self.tags.set(true);
        }
        if r.tagged || r.helpers.contains(&"stata_mv") {
            out.package("haven");
        }
        for c in r.caveats {
            out.caveat(c);
        }
        out.helpers.extend(r.helpers);
        (r.code, r.kind)
    }

    /// Whether the data may hold `.a` to `.z`; when it may, the R reads their
    /// places with `stata_mv` (haven's tagged NAs).
    fn mv_on(&mut self, out: &mut Out) -> bool {
        if self.tags.get() {
            self.helpers.insert("stata_mv");
            out.package("haven");
        }
        self.tags.get()
    }

    /// `_rc` only while the last line under capture was translated.
    fn rc_ok(&self, text: &str) -> Result<(), Untranslated> {
        let Some(why) = &self.rc_lost else {
            return Ok(());
        };
        let b = text.as_bytes();
        let word = |i: usize| {
            b.get(i)
                .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_')
        };
        let uses = text
            .match_indices("_rc")
            .any(|(i, _)| !(i > 0 && word(i - 1)) && !word(i + 3));
        if uses {
            return Err(no(why.clone()));
        }
        Ok(())
    }

    /// `e(sample)` only while `stata_esample` still lines up with the rows.
    fn esample_ok(&self, text: &str) -> Result<(), Untranslated> {
        let squeezed: String = text.chars().filter(|c| !c.is_whitespace()).collect();
        if squeezed.contains("e(sample)") && !self.esample {
            return Err(no(
                "e(sample) is translated only after regress or a likelihood model, while the \
                 data's rows are still the ones it was fitted on (not sorted, added or dropped \
                 since)",
            ));
        }
        Ok(())
    }

    fn value(&self, text: &str, out: &mut Out) -> Result<(String, Kind), Untranslated> {
        self.esample_ok(text)?;
        self.rc_ok(text)?;
        let kinds = |n: &str| self.kind_of(n);
        let ret = |w: &str, n: &str| self.returned_value(w, n);
        let r = expr_r::translate_drawing(text, &kinds, &ret, Use::Value, None, self.tags.get())?;
        Ok(self.took(r, out))
    }

    fn data(&self) -> Result<String, Untranslated> {
        if let (None, Some(why)) = (&self.data, &self.data_lost) {
            return Err(no(why.clone()));
        }
        self.data
            .clone()
            .ok_or_else(|| no("no data is in memory at this point (no use or import before it)"))
    }

    /// A line Stata runs that is not translated: if it replaces the data or
    /// changes its rows, what is in memory after it is not known.
    /// A line may have moved the rows or changed the data: the model held,
    /// and those `estimates store` kept, no longer mark Stata's e(sample).
    fn rows_unknown(&mut self) {
        self.esample = false;
        self.changed_since_fit = true;
        for f in self.stored_fits.values_mut() {
            f.esample = false;
            f.changed_since_fit = true;
        }
    }

    fn not_translated(&mut self, stata: &str, reason: &str) {
        // in a program: what it returns is no longer known
        if let Some(p) = self.programs.last_mut() {
            p.refused = true;
        }
        // a matrix the line names may have changed: no longer known (before
        // the return below, since a matrix lost earlier can look like an error)
        let named: Vec<String> = self
            .user_mats
            .iter()
            .filter(|m| matrices::may_write(stata, m))
            .cloned()
            .collect();
        // a handle a refused file open names may be open in Stata
        if let Some(h) = files::opened(stata) {
            self.file_handles.remove(&h);
            self.file_lost.insert(h);
        }
        let mut named = named;
        named.extend(matrices::written(stata));
        for m in named {
            self.user_mats.remove(&m);
            // (the reason kept when only the run can tell)
            let why = if reason.contains(crate::dofile::KNOWN_WHEN_RUN) {
                format!("{}: {reason}", stata.trim())
            } else {
                stata.trim().to_string()
            };
            self.lost_mats.insert(m, why);
        }
        // an outreg table a refused outreg or frmttable may have stored
        {
            let word = crate::dofile::command_name(stata.trim());
            if matches!(word.as_str(), "outreg" | "frmttable") {
                if let Some(rest) = stata.split("store(").nth(1) {
                    let name = rest.split(')').next().unwrap_or("").trim();
                    if is_name(name) {
                        self.outreg_lost
                            .insert(name.to_string(), stata.trim().to_string());
                    }
                }
            }
        }
        // a line that may write a file: which files are there is not known
        {
            let word = crate::dofile::command_name(stata.trim());
            let low = stata.to_lowercase();
            let writes = matches!(
                word.as_str(),
                "save"
                    | "sa"
                    | "sav"
                    | "saveold"
                    | "export"
                    | "outsheet"
                    | "outfile"
                    | "file"
                    | "fdasave"
                    | "putexcel"
                    | "putdocx"
                    | "putpdf"
                    | "log"
                    | "cmdlog"
                    | "copy"
                    | "shell"
                    | "erase"
                    | "rm"
                    | "mkdir"
                    | "esttab"
                    | "estout"
                    | "outreg2"
                    | "graph"
                    | "gr"
                    | "estimates"
                    | "est"
            ) || low.contains(" using ")
                || low.contains("saving(")
                || low.starts_with('!');
            if writes && !reason.contains("Stata stops here") {
                self.files_unknown = Some(stata.trim().to_string());
            }
        }
        // a line Stata stops at with an error leaves the data as it was
        if reason.contains("Stata stops here") || reason.contains("(Stata: ") {
            return;
        }
        // it may have stored a model (eststo, a program), so a name
        // `estimates store` did not keep may be stored in Stata
        self.stores_lost = true;
        if let Some(c) = crate::reader::read(stata.trim()).commands().first() {
            let later = reason.contains(crate::dofile::KNOWN_WHEN_RUN);
            for v in new_vars(c).into_iter().chain(in_place_vars(c)) {
                let line = if later {
                    format!("{}: {reason}", stata.trim())
                } else {
                    stata.trim().to_string()
                };
                self.lost_vars.insert(v, line);
            }
        }
        let word = crate::dofile::command_name(stata.trim());
        let full = crate::reader::read(&word)
            .commands()
            .first()
            .map(|c| c.name.clone())
            .unwrap_or(word);
        // (in a dry run a line that waits for a value from R still draws when run)
        if self.rng_lost.is_none()
            && !reason.contains(crate::dofile::KNOWN_WHEN_RUN)
            && random::moves_generator(stata, &full, reason)
        {
            self.rng_lost = Some(format!(
                "the random draws here are not Stata's: a line before it that draws was not \
                 translated ({}); a set seed after it starts them again",
                stata.trim()
            ));
        }
        // a line refused for a variable that is not there changed nothing more
        if reason.contains(LOST_VAR) {
            return;
        }
        if REPLACE_DATA.contains(&full.as_str()) && self.data.is_some() {
            self.data = None;
            self.data_lost = Some(format!(
                "the data in memory is not known here: a line before it that changes the data was not translated ({})",
                stata.trim()
            ));
        }
    }

    /// Why a line cannot run: it names a variable whose making line was not
    /// translated (a name in quotes does not count).
    fn uses_lost(&self, c: &Command) -> Option<String> {
        if self.lost_vars.is_empty() {
            return None;
        }
        let mut word = String::new();
        let mut quoted = false;
        let mut found = None;
        // a scalar set again is known again: only what it is set to counts
        let text = match (c.name.as_str(), c.text.split_once('=')) {
            ("scalar", Some((_, exp))) => exp,
            _ => c.text.as_str(),
        };
        for ch in text.chars().chain(std::iter::once(' ')) {
            if ch == '"' {
                quoted = !quoted;
            }
            if !quoted && (ch.is_alphanumeric() || ch == '_') {
                word.push(ch);
                continue;
            }
            if let Some(line) = self.lost_vars.get(&word) {
                found = Some((word.clone(), line.clone()));
                break;
            }
            word.clear();
        }
        let (v, line) = found?;
        if line.contains(crate::dofile::KNOWN_WHEN_RUN) {
            return Some(format!(
                "{v} is {}: the line that makes it is ({line})",
                crate::dofile::KNOWN_WHEN_RUN
            ));
        }
        Some(format!("{v} {LOST_VAR} ({line})"))
    }

    /// The `if` and `in` of a command as one condition.
    fn condition(&self, c: &Command, out: &mut Out) -> Result<Option<String>, Untranslated> {
        let mut parts = Vec::new();
        if let Some(e) = &c.parts.if_exp {
            parts.push(self.expr(e, Use::Condition, out)?);
        }
        if let Some(r) = &c.parts.in_range {
            parts.push(in_range(r)?);
        }
        Ok(match parts.len() {
            0 => None,
            1 => parts.pop(),
            _ => Some(format!("({})", parts.join(" & "))),
        })
    }

    /// The steps a `by` prefix puts around a step: sorting for `bysort`, then
    /// grouping; `None` when there is no `by`.
    fn by(
        &mut self,
        c: &Command,
        out: &mut Out,
    ) -> Result<Option<(Option<String>, String)>, Untranslated> {
        let Some((varlist, sort)) = c.prefixes.iter().find_map(|p| match p {
            Prefix::By { varlist, sort, .. } => Some((varlist.clone(), *sort)),
            _ => None,
        }) else {
            return Ok(None);
        };
        // `bysort a (b):` sorts by a and b and groups by a
        let (group, within) = match varlist.split_once('(') {
            Some((g, w)) => (
                g.trim().to_string(),
                w.trim_end_matches(')').trim().to_string(),
            ),
            None => (varlist.trim().to_string(), String::new()),
        };
        let group_sel = select(&group)?;
        let order = |sel: String, me: &mut Self, out: &mut Out| {
            if me.mv_on(out) {
                me.helpers.insert("stata_sort_keys");
                out.package("vctrs");
                format!("dplyr::arrange(stata_sort_keys(dplyr::pick({sel})))")
            } else {
                format!("dplyr::arrange(dplyr::pick({sel}))")
            }
        };
        let arrange = if sort || !within.is_empty() {
            let all = format!("{group} {within}");
            Some(order(select(all.trim())?, self, out))
        } else {
            None
        };
        Ok(Some((arrange, self.group_by(&group_sel, out))))
    }

    /// `dplyr::group_by()` on the columns `sel`: where the data may hold `.a`
    /// to `.z`, also on `stata_tag_k` (`stata_tag_key`), so each missing value
    /// is a group of its own; [`Self::ungroup`] drops it again.
    fn group_by(&mut self, sel: &str, out: &mut Out) -> String {
        if self.mv_on(out) {
            self.helpers.insert("stata_tag_key");
            format!(
                "dplyr::group_by(dplyr::pick({sel}), stata_tag_k = stata_tag_key(dplyr::pick({sel})))"
            )
        } else {
            format!("dplyr::group_by(dplyr::pick({sel}))")
        }
    }

    /// What ends a [`Self::group_by`]: `dplyr::ungroup()`, and the key it
    /// may have added dropped.
    fn ungroup(&self) -> String {
        if self.tags.get() {
            "dplyr::ungroup() |> dplyr::select(-stata_tag_k)".into()
        } else {
            "dplyr::ungroup()".into()
        }
    }

    /// `data <- data |> step` with the `by` prefix's grouping around it.
    fn step(&mut self, c: &Command, step: String, out: &mut Out) -> Result<(), Untranslated> {
        let d = self.data()?;
        let mut steps = Vec::new();
        match self.by(c, out)? {
            Some((arrange, group)) => {
                steps.extend(arrange);
                steps.push(group);
                steps.push(step);
                steps.push(self.ungroup());
            }
            None => steps.push(step),
        }
        out.line(format!("{d} <- {d} |> {}", steps.join(" |> ")));
        Ok(())
    }

    /// `by g: command` for the statistics commands and the models: the command
    /// translated once against `stata_d`, then run on each group in turn in a
    /// loop at the top level, so that after a model `stata_b` and the rest are
    /// the last group's, which is Stata's `e()` after `by`.
    fn by_each(&mut self, c: &Command, out: &mut Out) -> Result<(), Untranslated> {
        let Some((varlist, sort, options)) = c.prefixes.iter().find_map(|p| match p {
            Prefix::By {
                varlist,
                sort,
                options,
            } => Some((varlist.clone(), *sort, options.clone())),
            _ => None,
        }) else {
            return self.dispatch(c, out);
        };
        // `sort` is already in `sort`; `rc0` and the rest are not translated
        if let Some(o) = options
            .split([',', ' '])
            .map(str::trim)
            .find(|o| !o.is_empty() && *o != "sort")
        {
            return Err(no(format!("by with the option {o} is not translated yet")));
        }
        if c.parts.in_range.is_some() {
            return Err(no("'in' may not be combined with 'by'"));
        }
        let (group, within) = match varlist.split_once('(') {
            Some((g, w)) => (g.to_string(), w.trim_end_matches(')').to_string()),
            None => (varlist.clone(), String::new()),
        };
        let names = |s: &str| -> Result<Vec<String>, Untranslated> {
            s.split_whitespace()
                .map(|w| {
                    if is_name(w) {
                        Ok(w.to_string())
                    } else {
                        Err(no(format!(
                            "by {w}: only variable names written out are translated in a by prefix"
                        )))
                    }
                })
                .collect()
        };
        let (group, within) = (names(&group)?, names(&within)?);
        if group.is_empty() {
            return Err(no("by needs a variable"));
        }
        let d = self.data()?;
        let mut inner = c.clone();
        inner.prefixes.retain(|p| !matches!(p, Prefix::By { .. }));
        let saved = self.data.replace("stata_d".into());
        let mut body = Out::default();
        let done = self.dispatch(&inner, &mut body);
        self.data = saved;
        done?;
        let vector = |v: &[String]| {
            let v: Vec<String> = v.iter().map(|x| r_text(x)).collect();
            format!("c({})", v.join(", "))
        };
        // each of . and .a to .z a group of its own, in Stata's order
        let tags = self.mv_on(out);
        if sort {
            out.package("dplyr");
            let all: Vec<String> = group.iter().chain(&within).map(|v| r_name(v)).collect();
            if tags {
                self.helpers.insert("stata_sort_keys");
                out.package("vctrs");
                out.line(format!(
                    "{d} <- dplyr::arrange({d}, stata_sort_keys(dplyr::pick({})))",
                    all.join(", ")
                ));
            } else {
                out.line(format!("{d} <- dplyr::arrange({d}, {})", all.join(", ")));
            }
        }
        self.helpers.insert("stata_by_groups");
        let mut args = vec![d.clone(), vector(&group)];
        if !within.is_empty() {
            args.push(format!("within = {}", vector(&within)));
        }
        if sort {
            args.push("sorted = TRUE".into());
        }
        if tags {
            args.push("tags = TRUE".into());
        }
        out.line(format!(
            "for (stata_g in stata_by_groups({})) {{",
            args.join(", ")
        ));
        if !self.quiet(c) {
            out.line("cat(\"\\n-> \", stata_g$label, \"\\n\", sep = \"\")");
        }
        out.line(format!("stata_d <- {d}[stata_g$rows, , drop = FALSE]"));
        if body.code.iter().any(|l| l.contains("stata_perfect(")) {
            // Stata's by goes on to the next group after r(2000)
            out.line("tryCatch({");
            out.code.extend(body.code);
            out.line("}, stata_r2000 = function(e) cat(conditionMessage(e), \"\\n\", sep = \"\"))");
        } else {
            out.code.extend(body.code);
        }
        out.line("}");
        for cv in body.caveats {
            out.caveat(cv);
        }
        for p in body.packages {
            out.package(p);
        }
        out.caveat(
            "by: each group in turn, in sorted order with a missing value last, as in Stata; \
             after a model, the results kept are the last group's",
        );
        if !sort {
            out.caveat(
                "the rows must already be in order of the by variables; R checks the order \
                 itself and stops with Stata's not sorted",
            );
        }
        Ok(())
    }

    fn command(&mut self, c: &Command) -> Result<RCode, Untranslated> {
        // a variable another command makes again: its storage is not known
        if !matches!(c.name.as_str(), "generate" | "replace") {
            for v in new_vars(c) {
                self.storage.remove(&v);
            }
        }
        if let Some(why) = self.uses_lost(c) {
            // Stata has the variable and runs the line: a model there
            // replaces e() (and r()), so neither is R's any more
            self.track_returned(c, false, None);
            self.lose_estimates(c, &why);
            return Err(no(why));
        }
        if c.prefixes.iter().any(|p| matches!(p, Prefix::Frame(_))) {
            self.rows_unknown();
            return Err(no("frames (frame name:) are translated in a later step"));
        }
        if c.parts.weight.is_some() && !WEIGHTED.contains(&c.name.as_str()) {
            self.rows_unknown();
            return Err(no(format!("{} with weights is not translated yet", c.name)));
        }
        if let Some(c2) = self.restated(c) {
            return self.command(&c2);
        }
        let mut out = Out::default();
        let by = c.prefixes.iter().any(|p| matches!(p, Prefix::By { .. }));
        if MODELS.contains(&c.name.as_str()) {
            self.fit_ts = None;
        }
        // under by, each group is a slice of the data: L.x stays right only
        // when the groups are the panels
        let by_vars = c.prefixes.iter().find_map(|p| match p {
            Prefix::By { varlist, .. } => Some(varlist.trim().to_string()),
            _ => None,
        });
        if let Some(v) = by_vars {
            let text = format!(
                "{} {}",
                c.parts.main,
                c.parts.if_exp.as_deref().unwrap_or("")
            );
            if !ts::ts_words(&text).is_empty() && self.panel.as_deref() != Some(v.as_str()) {
                self.rows_unknown();
                return Err(no(
                    "by with time-series operators is translated only when by names the panel \
                     variable (Stata stops with \"not sorted\" when by's sort order is not the \
                     panel and time)",
                ));
            }
        }
        self.esample_new = false;
        self.r_new = None;
        self.e_new = false;
        if MODELS.contains(&c.name.as_str()) {
            self.svy_fit = false;
        }
        self.quiet_asked.set(false);
        self.drew.set(false);
        self.scalars_read.borrow_mut().clear();
        if MODELS.contains(&c.name.as_str()) {
            self.fit_robust = false;
            self.fit_cluster = false;
        }
        let done = if by && BY_EACH.contains(&c.name.as_str()) {
            self.by_each(c, &mut out)
        } else {
            self.dispatch(c, &mut out)
        };
        // a variable comes before a scalar of the same name, as in Stata
        let read = std::mem::take(&mut *self.scalars_read.borrow_mut());
        if let (Ok(()), Some(d), false) = (&done, &self.data, read.is_empty()) {
            let mut names: Vec<String> = Vec::new();
            for n in read {
                let n = r_text(&n);
                if !names.contains(&n) {
                    names.push(n);
                }
            }
            out.code.insert(
                0,
                format!("stata_scalar_clash({d}, c({}))", names.join(", ")),
            );
        }
        // who holds e() and r() now
        let r_left = self.r_new.take();
        self.track_returned(c, done.is_ok(), r_left);
        if let Err(e) = &done {
            self.lose_estimates(c, &e.reason);
        }
        if done.is_ok() && MODELS.contains(&c.name.as_str()) && !self.e_new {
            self.e_ret =
                returned::Returned::Lost(format!("e() after {} is not kept in R yet", c.name));
        }
        // a line that is not translated still runs in Stata, so it counts too
        // (putexcel only writes a spreadsheet, translated or not)
        let harmless = HARMLESS.contains(&c.name.as_str());
        if (done.is_err() && !harmless) || !LEAVE_DATA_ALONE.contains(&c.name.as_str()) {
            self.changed_since_fit = done.is_err() || !MODELS.contains(&c.name.as_str());
            if self.changed_since_fit {
                for f in self.stored_fits.values_mut() {
                    f.changed_since_fit = true;
                }
            }
        }
        // e(sample) holds while the rows stay where the model found them
        let name = c.name.as_str();
        let bysort = c
            .prefixes
            .iter()
            .any(|p| matches!(p, Prefix::By { sort: true, .. }));
        // values may change (Stata's e(sample) is a mark on the rows, so it
        // stays); rows may not be sorted, added or dropped
        let values_only = matches!(
            name,
            "replace"
                | "recode"
                | "egen"
                | "label"
                | "rename"
                | "destring"
                | "tostring"
                | "encode"
                | "decode"
                | "format"
                | "order"
                | "compress"
                | "center"
                | "winsor"
                | "alpha"
                // ereturn post, esample() drops the variable it names
                | "ereturn"
                // a program's own lines are judged one by one
                | "_rcraft_program_begin"
                | "_rcraft_program_end"
        ) || (matches!(name, "drop" | "keep")
            && c.parts.if_exp.is_none()
            && c.parts.in_range.is_none());
        let rows_stay = (done.is_ok() || harmless)
            && !bysort
            && !matches!(name, "sort" | "gsort" | "tsset" | "xtset")
            && (MODELS.contains(&name) || LEAVE_DATA_ALONE.contains(&name) || values_only);
        if !rows_stay {
            for f in self.stored_fits.values_mut() {
                f.esample = false;
            }
        }
        self.esample = if MODELS.contains(&name) {
            done.is_ok() && self.esample_new && !by
        } else {
            self.esample && rows_stay
        };
        done?;
        if self.drew.get() {
            self.helpers.insert("stata_rng");
        }
        for v in new_vars(c) {
            self.lost_vars.remove(&v);
        }
        if MODELS.contains(&c.name.as_str()) {
            // new results, not stored yet
            self.active_store = None;
        }
        // eststo [name]: command, then the results are stored
        let est_prefix = c.prefixes.iter().find_map(|p| match p {
            Prefix::Eststo(n) => Some(n.clone()),
            _ => None,
        });
        if let Some(name) = est_prefix {
            let name = name.unwrap_or_else(|| format!("est{}", self.stored_fits.len() + 1));
            self.eststo_store(&name, &mut out)?;
        }
        if MODELS.contains(&c.name.as_str()) {
            self.weighted_fit = c.parts.weight.is_some();
            self.fit_fweight = matches!(&c.parts.weight, Some(w) if w.kind == crate::lang::syntax::WeightKind::Frequency);
        }
        // a new fit: the last one's results go first, so that if R stops in this
        // one (a model RCraft declines when it sees the data), what reads e() or
        // _b[] afterwards stops too, not shows the model before
        if MODELS.contains(&c.name.as_str()) && self.e_new && !out.code.is_empty() {
            out.code.insert(
                0,
                "rm(list = intersect(c(\"stata_e\", \"stata_b\", \"stata_vcov\", \"stata_df\", \"stata_estimates\"), ls(sorted = FALSE)))\nstata_fit_failed <- TRUE"
                    .to_string(),
            );
            out.code.push("stata_fit_failed <- FALSE".to_string());
        }
        // quietly (or capture): what the line prints is not shown
        if silent(c) && !self.quiet_asked.get() && !out.code.is_empty() {
            let body = out.code.join(
                "
",
            );
            out.code = vec![format!(
                "invisible(utils::capture.output({{
{body}
}}))"
            )];
        }
        if c.prefixes.contains(&Prefix::Capture) {
            // capture: an error is ignored and the do-file goes on; _rc says
            // whether there was one, and noisily shows Stata's message
            self.helpers.insert("stata_captured");
            let body = out.code.join("\n");
            let noisily = if silent(c) { "FALSE" } else { "TRUE" };
            out.code = vec![format!(
                "stata_rc <- tryCatch({{\n{body}\n0\n}}, error = function(e) stata_captured(e, {noisily}))"
            )];
        }
        // the R helpers the line's expressions call
        for h in std::mem::take(&mut out.helpers) {
            self.helpers.insert(h);
        }
        out.done()
    }

    fn dispatch(&mut self, c: &Command, out: &mut Out) -> Result<(), Untranslated> {
        let opts = Opts(&c.options);
        match c.name.as_str() {
            "use" => self.use_(c, &opts, out),
            "save" | "saveold" => self.save(c, &opts, out),
            "import" => self.import(c, &opts, out),
            "insheet" => self.insheet(c, &opts, out),
            "split" => self.split(c, &opts, out),
            "input" => self.input(c, out),
            "confirm" => self.confirm(c, out),
            "format" => self.format(c, out),
            "postfile" | "post" | "postclose" | "postutil" => self.post_cmd(c, &opts, out),
            "file" => self.file(c, out),
            "generate" => self.generate(c, &opts, out),
            "replace" => self.replace(c, out),
            "drop" | "keep" => self.drop_keep(c, c.name == "keep", out),
            "rename" => self.rename(c, &opts, out),
            "sort" => self.sort(c, &opts, out),
            "gsort" => self.gsort(c, &opts, out),
            "order" => self.order(c, &opts, out),
            "label" => self.label(c, &opts, out),
            "count" => self.count(c, out),
            "levelsof" => self.levelsof(c, &opts, out),
            "mkmat" => self.mkmat(c, &opts, out),
            "svmat" => self.svmat(c, &opts, out),
            "_natscale" => self.natscale(c, out),
            "list" => self.list(c, &opts, out),
            "describe" => self.describe(c, out),
            "set" => self.set(c, out),
            "estimates" => self.estimates_cmd(c, out),
            "eststo" => self.eststo(c, &opts, out),
            "esttab" => self.esttab(c, &opts, out),
            "estpost" => self.estpost(c, &opts, out),
            "estadd" => self.estadd(c, &opts, out),
            "erase" | "rm" => self.erase(c, out),
            "assert" => self.assert(c, &opts, out),
            "fvset" => self.fvset(c, out),
            "compress" => {
                if let Some(o) = opts.other(&[("nocoalesce", 10)]) {
                    return Err(no(format!("compress with the option {o} is not translated")));
                }
                out.caveat(
                    "nothing to do in R: R has no storage types to make smaller (Stata lists each variable it changes and the bytes saved)",
                );
                Ok(())
            }
            "bsample" => self.bsample(c, &opts, out),
            "bootstrap" => self.bootstrap(c, out),
            "clear" => {
                if let Some(d) = self.data.take() {
                    out.line(format!("rm({d})"));
                }
                self.data_lost = None;
                self.kinds.clear();
                self.storage.clear();
                self.fv_base.clear();
                self.forget_ts();
                self.label_sets.clear();
                Ok(())
            }
            "version" => {
                self.version(c);
                out.caveat("nothing to do in R: version sets how Stata reads the do-file");
                Ok(())
            }
            "preserve" => self.preserve(c, &opts, out),
            "expand" => self.expand(c, &opts, out),
            "restore" => self.restore(c, &opts, out),
            "recode" => self.recode(c, &opts, out),
            "egen" => self.egen(c, &opts, out),
            "merge" => self.merge(c, &opts, out),
            "append" => self.append(c, &opts, out),
            "reshape" => self.reshape(c, &opts, out),
            "collapse" => self.collapse(c, &opts, out),
            "xpose" => self.xpose(c, &opts, out),
            "duplicates" => self.duplicates(c, &opts, out),
            "isid" => self.isid(c, &opts, out),
            "survwgt" => self.survwgt(c, &opts, out),
            "contract" => self.contract(c, &opts, out),
            "recast" => self.recast(c, &opts, out),
            "isvar" => self.isvar(c, &opts, out),
            "suest" => self.suest(c, &opts, out),
            "codebook" => self.codebook(c, out),
            "center" => self.center(c, &opts, out),
            "winsor" => self.winsor(c, &opts, out),
            "alpha" => self.alpha(c, &opts, out),
            "outreg" => self.outreg(c, &opts, out),
            "frmttable" => self.frmttable(c, &opts, out),
            "mata" => self.mata_line(c, out),
            "filefilter" => self.filefilter(c, &opts, out),
            "putexcel" => self.putexcel(c, &opts, out),
            // query shows Stata's settings, which R does not have
            "query" => {
                out.caveat("nothing to do in R: query only shows Stata's own settings");
                Ok(())
            }
            // grstyle (SSC) only builds a graph scheme, as set scheme chooses one
            "grstyle" => {
                out.caveat("nothing to do in R: grstyle only changes how Stata's graphs look; RCraft's graphs keep their own");
                Ok(())
            }
            "encode" => self.encode(c, &opts, out),
            "decode" => self.decode(c, &opts, out),
            "destring" => self.destring(c, &opts, out),
            "tostring" => self.tostring(c, &opts, out),
            "summarize" => self.summarize(c, &opts, out),
            "tabulate" => self.tabulate(c, &opts, out),
            "correlate" => self.correlate(c, &opts, out),
            "tab1" => self.tab1(c, &opts, out),
            "tab2" => self.tab2(c, &opts, out),
            "pwcorr" => self.pwcorr(c, &opts, out),
            "ttest" => self.ttest(c, &opts, out),
            "regress" => self.regress(c, &opts, out),
            "logit" => self.ml(c, &opts, out, model::Fit::Logit),
            "probit" => self.ml(c, &opts, out, model::Fit::Probit),
            "poisson" => self.ml(c, &opts, out, model::Fit::Poisson),
            "nbreg" => self.ml(c, &opts, out, model::Fit::Nbreg),
            "ologit" => self.ml(c, &opts, out, model::Fit::Ologit),
            "oprobit" => self.ml(c, &opts, out, model::Fit::Oprobit),
            "mlogit" => self.ml(c, &opts, out, model::Fit::Mlogit),
            "predict" => self.predict(c, &opts, out),
            "test" => self.test(c, &opts, out),
            "lincom" => self.lincom(c, &opts, out),
            "estat" => self.estat(c, &opts, out),
            "margins" => self.margins(c, &opts, out),
            "anova" => self.anova(c, &opts, out),
            "xtset" => self.xtset(c, &opts, out),
            "tsset" => self.tsset(c, &opts, out),
            "xtreg" => self.xtreg(c, &opts, out),
            "xtregar" => self.xtregar(c, &opts, out),
            "prais" | "newey" => self.tsreg(c, &opts, out),
            "ivregress" => self.ivregress(c, &opts, out),
            w if crate::lang::syntax::abbrev(w, "histogram", 4) => self.histogram(c, &opts, out),
            w if crate::lang::syntax::abbrev(w, "kdensity", 5) => self.kdensity(c, &opts, out),
            w if crate::lang::syntax::abbrev(w, "scatter", 2) => {
                self.twoway(c, &opts, out, Some("scatter"))
            }
            "line" => self.twoway(c, &opts, out, Some("line")),
            w if crate::lang::syntax::abbrev(w, "connected", 4) => {
                self.twoway(c, &opts, out, Some("connected"))
            }
            w if crate::lang::syntax::abbrev(w, "twoway", 2) => self.twoway(c, &opts, out, None),
            w if crate::lang::syntax::abbrev(w, "graph", 2) => self.graph(c, &opts, out),
            "lowess" => self.lowess(c, &opts, out),
            "stset" => self.stset(c, &opts, out),
            "stcox" => self.stcox(c, &opts, out),
            "clogit" => self.clogit(c, &opts, out),
            "robreg" => self.robreg(c, &opts, out),
            "reghdfe" => self.reghdfe(c, &opts, out),
            "cluster2" => self.cluster2(c, &opts, out),
            "asclogit" => self.asclogit(c, &opts, out),
            "drawnorm" => self.drawnorm(c, &opts, out),
            "_pctile" => self.pctile_r(c, &opts, out),
            "svyset" => self.svyset(c, &opts, out),
            "svy" => self.svy(c, out),
            "outreg2" => self.outreg2(c, &opts, out),
            "fre" => self.fre(c, &opts, out),
            "coefplot" => self.coefplot(c, out),
            "parmest" => self.parmest(c, &opts, out),
            "xtile" => self.xtile(c, &opts, out),
            "display" => self.display(c, out),
            "return"
                if matches!(
                    c.parts.main.split_whitespace().next(),
                    Some("scalar" | "local" | "clear" | "matrix" | "add")
                ) =>
            {
                self.program_return(c, out)
            }
            "_rcraft_program_begin" => self.program_begin(c, out),
            "_rcraft_program_end" => self.program_end(c, out),
            "ereturn" => self.ereturn(c, out),
            "return" => self.list_returned(c, out),
            "matrix" => self.matrix(c, out),
            "scalar" => self.scalar(c, out),
            "ssc" => self.ssc(c, out),
            "net" => Err(no(
                "net fetches Stata add-ons, which are out of scope; an R package can be installed with rc install (P2B.11)",
            )),
            "cd" => {
                let dir = file_name(&c.parts.main, "");
                if dir.is_empty() {
                    // Stata on Windows prints the folder; elsewhere it goes home
                    return Err(no("cd without a folder is not translated: Stata does different things on Windows and elsewhere"));
                }
                out.line(format!("setwd({})", r_text(&dir)));
                self.folder = if is_absolute(&dir) {
                    Some(dir.trim_end_matches('/').to_string())
                } else {
                    self.folder.as_ref().map(|f| format!("{f}/{}", dir.trim_end_matches('/')))
                };
                Ok(())
            }
            "pwd" => {
                out.line("cat(getwd(), \"\n\", sep = \"\")");
                Ok(())
            }
            "log" | "cmdlog" => {
                out.caveat(format!(
                    "nothing to do in R: RCraft keeps what runs itself (Results, and the do-file run's own log), so {} writes no file",
                    c.name
                ));
                Ok(())
            }
            "browse" | "edit" => {
                out.caveat(format!(
                    "nothing to do in R: {} opens Stata's Data Editor; in RCraft the data are in the Data view",
                    c.name
                ));
                Ok(())
            }
            "pause" | "more" | "discard" | "sleep" | "beep" => {
                out.caveat(format!(
                    "nothing to do in R: {} only changes how Stata runs",
                    c.name
                ));
                Ok(())
            }
            other => Err(no(format!("{other} is not translated yet"))),
        }
    }

    /// Whether the line prints nothing; the command then leaves its output out
    /// itself, and the line's R is not wrapped to hide it.
    fn quiet(&self, c: &Command) -> bool {
        self.quiet_asked.set(true);
        silent(c)
    }

    // ---------------- files ----------------

    fn use_(&mut self, c: &Command, opts: &Opts, out: &mut Out) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[("clear", 5), ("nolabel", 5)]) {
            return Err(no(format!("use with the option {o} is not translated yet")));
        }
        let (vars, file) = match &c.using {
            Some(f) => (
                Some(c.parts.main.clone()).filter(|v| !v.trim().is_empty()),
                f.clone(),
            ),
            None => (None, c.parts.main.clone()),
        };
        if file.trim().is_empty() {
            return Err(no("use needs a file name"));
        }
        let path = file_name(&file, "dta");
        let name = frame_name(&path);
        out.package("haven");
        let mut read = format!("haven::read_dta({}", r_text(&path));
        if let Some(v) = &vars {
            read.push_str(&format!(", col_select = {}", select(v)?));
        }
        read.push(')');
        let mut steps = vec![read];
        if opts.has("nolabel", 5) {
            steps.push("haven::zap_labels()".into());
        }
        self.data = Some(name.clone());
        // the label sets and fvset bases are the file's: those it was saved
        // with when this do-file saved it, none known otherwise
        let kept = self
            .saved
            .get(&self.file_key(&path))
            .cloned()
            .unwrap_or_default();
        self.fv_base = kept.fv_base;
        self.label_sets = kept.label_sets;
        self.ts_unknown = !self.saved.contains_key(&self.file_key(&path));
        // a file this do-file did not save may hold .a to .z (tagged NAs)
        if self.ts_unknown {
            self.tags.set(true);
        }
        self.panel = kept.panel;
        self.panel_time = kept.panel_time;
        self.ts_delta = kept.ts_delta;
        self.file = Some(path);
        self.kinds.clear();
        self.storage.clear();
        if let Some(cond) = self.condition(c, out)? {
            steps.push(format!("dplyr::filter({cond})"));
        }
        out.line(format!("{name} <- {}", steps.join(" |> ")));
        Ok(())
    }

    fn save(&mut self, c: &Command, opts: &Opts, out: &mut Out) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[("replace", 7), ("version", 1), ("nolabel", 5)]) {
            return Err(no(format!(
                "save with the option {o} is not translated yet"
            )));
        }
        let d = self.data()?;
        let path = if c.parts.main.trim().is_empty() {
            self.file
                .clone()
                .ok_or_else(|| no("save needs a file name here"))?
        } else {
            file_name(&c.parts.main, "dta")
        };
        out.package("haven");
        if !opts.has("replace", 7) {
            out.line(format!(
                "if (file.exists({p})) stop(\"file {e} already exists\")",
                p = r_text(&path),
                e = path.replace('\\', "/").replace('"', "")
            ));
        }
        out.line(format!("haven::write_dta({d}, {})", r_text(&path)));
        self.saved.insert(
            self.file_key(&path),
            Kept {
                fv_base: self.fv_base.clone(),
                label_sets: self.label_sets.clone(),
                panel: self.panel.clone(),
                panel_time: self.panel_time.clone(),
                ts_delta: self.ts_delta.clone(),
            },
        );
        self.file = Some(path);
        Ok(())
    }

    fn import(&mut self, c: &Command, opts: &Opts, out: &mut Out) -> Result<(), Untranslated> {
        let (kind, rest) = first_word(&c.parts.main);
        let file = c
            .using
            .clone()
            .or_else(|| Some(rest.to_string()).filter(|r| !r.is_empty()))
            .ok_or_else(|| no("import needs a file name"))?;
        if c.using.is_some() && !rest.is_empty() {
            return Err(no(
                "import of chosen variables (varlist using) is not translated yet",
            ));
        }
        let path = file_name(&file, "");
        let name = frame_name(&path);
        let mut steps: Vec<String> = Vec::new();
        match kind {
            k if "delimited".starts_with(k) && k.len() >= 5 => {
                if let Some(o) = opts.other(&[
                    ("clear", 5),
                    ("delimiters", 5),
                    ("varnames", 7),
                    ("case", 4),
                    ("encoding", 8),
                    ("asdouble", 8),
                    ("asfloat", 7),
                    ("stringcols", 7),
                ]) {
                    return Err(no(format!(
                        "import delimited with the option {o} is not translated yet"
                    )));
                }
                out.package("readr");
                let mut args = vec![r_text(&path)];
                match opts.arg("delimiters", 5).as_deref() {
                    Some("tab") | Some("\\t") => args.push("delim = \"\\t\"".into()),
                    Some("whitespace") => {
                        return Err(no(
                            "import delimited with delimiters(whitespace) is not translated yet",
                        ))
                    }
                    Some(d) => args.push(format!("delim = {}", r_text(d))),
                    None => {}
                }
                match opts.arg("varnames", 7).as_deref() {
                    None | Some("1") => {}
                    Some("nonames") => args.push("col_names = FALSE".into()),
                    Some(v) => {
                        return Err(no(format!(
                            "import delimited with varnames({v}) is not translated yet"
                        )))
                    }
                }
                if let Some(e) = opts.arg("encoding", 8) {
                    args.push(format!("locale = readr::locale(encoding = {})", r_text(&e)));
                }
                // every column as text: stringcols(_all)
                let all_text = match opts.arg("stringcols", 7).as_deref().map(str::trim) {
                    None => false,
                    Some("_all" | "*") => true,
                    Some(v) => {
                        return Err(no(format!(
                            "import delimited with stringcols({v}) is not translated yet (stringcols(_all) is)"
                        )))
                    }
                };
                if all_text {
                    args.push("col_types = readr::cols(.default = \"c\")".into());
                }
                // empty cells are missing, "NA" is text, as in Stata
                args.push("na = \"\"".into());
                args.push("show_col_types = FALSE".into());
                self.helpers.insert("stata_read_delim");
                steps.push(format!("stata_read_delim({})", args.join(", ")));
                steps.push("as.data.frame()".into());
                if all_text {
                    // an empty cell of a string is "" in Stata
                    steps.push(
                        "dplyr::mutate(dplyr::across(dplyr::everything(), \\(v) dplyr::coalesce(v, \"\")))"
                            .into(),
                    );
                }
                // Stata stores a column with decimals as float, unless asdouble
                if !opts.has("asdouble", 8) && !all_text {
                    self.helpers.insert("stata_float");
                    self.helpers.insert("stata_import_float");
                    steps.push("stata_import_float()".into());
                    out.caveat(
                        "numbers with decimals are stored as floats, as Stata's import delimited \
                         stores them (asdouble keeps them double)",
                    );
                }
                if opts.arg("varnames", 7).as_deref() == Some("nonames") {
                    steps.push("(\\(d) stats::setNames(d, paste0(\"v\", seq_along(d))))()".into());
                    out.caveat(
                        "columns are named v1, v2, ... as Stata does without variable names",
                    );
                }
                match opts.arg("case", 4).as_deref() {
                    None | Some("lower") => steps.push("dplyr::rename_with(tolower)".into()),
                    Some("upper") => steps.push("dplyr::rename_with(toupper)".into()),
                    Some("preserve") => {}
                    Some(v) => {
                        return Err(no(format!(
                            "import delimited with case({v}) is not translated"
                        )))
                    }
                }
                out.caveat(
                    "column names that are not valid Stata names are made valid by R's rules, \
                     which can differ from Stata's",
                );
            }
            k if "excel".starts_with(k) && k.len() >= 3 => {
                if let Some(o) = opts.other(&[
                    ("clear", 5),
                    ("sheet", 5),
                    ("firstrow", 5),
                    ("cellrange", 4),
                    ("case", 4),
                    ("allstring", 8),
                ]) {
                    return Err(no(format!(
                        "import excel with the option {o} is not translated yet"
                    )));
                }
                out.package("readxl");
                let mut args = vec![r_text(&path)];
                if let Some(s) = opts.arg("sheet", 5) {
                    args.push(format!("sheet = {}", r_text(&s)));
                }
                let first_col = match opts.arg("cellrange", 4) {
                    Some(r) => {
                        let range = if r.contains(':') {
                            r.clone()
                        } else {
                            format!("{r}:XFD1048576")
                        };
                        args.push(format!("range = {}", r_text(&range)));
                        excel_column(&r).unwrap_or(1)
                    }
                    None => 1,
                };
                let firstrow = opts.has("firstrow", 5);
                if !firstrow {
                    args.push("col_names = FALSE".into());
                    args.push(".name_repair = \"minimal\"".into());
                }
                if opts.has("allstring", 8) {
                    args.push("col_types = \"text\"".into());
                }
                steps.push(format!("readxl::read_excel({})", args.join(", ")));
                steps.push("as.data.frame()".into());
                if !firstrow {
                    self.helpers.insert("stata_excel_names");
                    steps.push(format!(
                        "(\\(x) stats::setNames(x, stata_excel_names(ncol(x), {first_col})))()"
                    ));
                }
                match opts.arg("case", 4).as_deref() {
                    None | Some("preserve") => {}
                    Some("lower") => steps.push("dplyr::rename_with(tolower)".into()),
                    Some("upper") => steps.push("dplyr::rename_with(toupper)".into()),
                    Some(v) => {
                        return Err(no(format!("import excel with case({v}) is not translated")))
                    }
                }
            }
            other => return Err(no(format!("import {other} is not translated yet"))),
        }
        // text cells that are empty are "" in Stata, not missing
        steps.push(
            "dplyr::mutate(dplyr::across(dplyr::where(is.character), \\(v) dplyr::coalesce(v, \"\")))".into(),
        );
        out.package("dplyr");
        out.line(format!("{name} <- {}", steps.join(" |> ")));
        self.data = Some(name);
        self.file = None;
        self.kinds.clear();
        self.storage.clear();
        self.fv_base.clear();
        self.forget_ts();
        self.label_sets.clear();
        Ok(())
    }

    // ---------------- new and changed variables ----------------

    fn generate(&mut self, c: &Command, opts: &Opts, out: &mut Out) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[("before", 3), ("after", 1)]) {
            return Err(no(format!(
                "generate with the option {o} is not translated yet"
            )));
        }
        let (lhs, rhs) = c
            .parts
            .main
            .split_once('=')
            .ok_or_else(|| no("generate needs `newvar = exp`"))?;
        let words: Vec<&str> = lhs.split_whitespace().collect();
        let (ty, var) = match words.as_slice() {
            [v] => ("float", *v),
            [t, v] => (*t, *v),
            _ => return Err(no("generate needs one new variable name")),
        };
        if !is_name(var) {
            return Err(no(format!("{var} is not a variable name")));
        }
        if var.contains(':') {
            return Err(no(
                "generate with a value label (newvar:label) is not translated yet",
            ));
        }
        let (cond, value, kind) = self.drawing_value(c, rhs, out)?;
        let storage = storage(ty).ok_or_else(|| no(format!("{ty} is not a storage type")))?;
        let is_text = kind == Kind::Text;
        if is_text != (storage == Storage::Str) && ty != "float" {
            return Err(no(format!(
                "generate {ty} of a {} expression: Stata refuses this (type mismatch)",
                if is_text { "string" } else { "numeric" }
            )));
        }
        let value = if is_text {
            // a declared str# keeps that many characters (strL keeps all)
            match ty.strip_prefix("str").and_then(|w| w.parse::<usize>().ok()) {
                Some(w) => format!("substr({value}, 1, {w})"),
                None => value,
            }
        } else {
            stored(&value, storage, &mut self.helpers, out)
        };
        let value = match cond {
            Some(cond) => {
                let missing = if is_text { "\"\"" } else { "NA_real_" };
                format!("dplyr::if_else({cond}, {value}, {missing})")
            }
            None => value,
        };
        let mut step = format!("dplyr::mutate({} = {value}", r_name(var));
        if let Some(b) = opts.arg("before", 3) {
            step.push_str(&format!(", .before = {}", r_name(b.trim())));
        } else if let Some(a) = opts.arg("after", 1) {
            step.push_str(&format!(", .after = {}", r_name(a.trim())));
        }
        step.push(')');
        out.package("dplyr");
        let d = self.data()?;
        out.line(format!(
            "if ({} %in% names({d})) stop(\"variable {var} already defined\")",
            r_text(var)
        ));
        self.step(c, step, out)?;
        self.kinds.insert(
            var.to_string(),
            if is_text { Kind::Text } else { Kind::Number },
        );
        if !is_text {
            self.storage.insert(var.to_string(), storage);
        }
        Ok(())
    }

    fn replace(&mut self, c: &Command, out: &mut Out) -> Result<(), Untranslated> {
        if !c.options.is_empty() {
            return Err(no("replace with options is not translated yet"));
        }
        let (var, rhs) = c
            .parts
            .main
            .split_once('=')
            .ok_or_else(|| no("replace needs `var = exp`"))?;
        let var = var.trim();
        if !is_name(var) {
            return Err(no(format!("{var} is not a variable name")));
        }
        // Stata replaces row by row when a row refers to the rows above it
        // (`replace x = x[_n-1] if missing(x)` carries values down a run)
        // earlier rows (`x[_n-1]`), the row itself or later ones (`x[_N]`) in the
        // value: then each row
        // depends on rows above it and doing the replace again until nothing
        // changes gives Stata's row-by-row result, as for L.x below
        let own = format!("{var}[");
        let earlier_only = !c.parts.if_exp.as_deref().is_some_and(|e| e.contains(&own))
            && own_subscripts_earlier(rhs, var);
        let own_rows = rhs.contains(&own);
        if own_rows && !earlier_only {
            return Err(no(format!(
                "replace that reads {var}'s own rows other than earlier ones ({var}[_n-1] is) runs \
                 row by row in Stata (each row sees the rows above already replaced); not translated yet"
            )));
        }
        // and so it does for L.var: each row sees the earlier times already replaced
        let read = format!("{rhs} {}", c.parts.if_exp.as_deref().unwrap_or(""));
        let own: Vec<ts::TsOps> = ts::ts_words(&read)
            .iter()
            .filter_map(|w| ts::parse_ts(w).filter(|(_, v)| *v == var).map(|(o, _)| o))
            .collect();
        // only earlier times (L.x, L2.x, LD.x): then each row depends on rows
        // before it, and doing the replace again until nothing changes gives
        // Stata's row-by-row result
        if own.iter().any(|o| o.shift < 1) {
            return Err(no(format!(
                "replace that reads {var} at this or a later time through an operator (D.{var}, \
                 F.{var}) runs row by row in Stata (each row sees the earlier rows already \
                 replaced, the later ones not); not translated yet"
            )));
        }
        let (cond, value, kind) = self.drawing_value(c, rhs, out)?;
        // a variable the do-file made keeps its storage, as in Stata: each row
        // rounded to float before the next reads it
        let value = match (self.storage.get(var).copied(), self.kinds.get(var)) {
            (Some(st), Some(Kind::Number)) if kind != Kind::Text => {
                stored(&value, st, &mut self.helpers, out)
            }
            _ => value,
        };
        let value = match cond {
            Some(cond) => format!("dplyr::if_else({cond}, {value}, {})", r_name(var)),
            None => value,
        };
        let value = if own.is_empty() && !own_rows {
            value
        } else {
            // with draws: each round draws the same numbers (the generator put
            // back before it), so the rows that run come out as Stata's, each
            // with its own draw; the rows the if picks must not move
            let drew = self.drew.get();
            let if_reads_own = c.parts.if_exp.as_deref().is_some_and(|e| {
                ts::ts_words(e)
                    .iter()
                    .any(|w| ts::parse_ts(w).is_some_and(|(_, v)| v == var))
                    || e.split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
                        .any(|w| w == var)
            });
            if drew && if_reads_own {
                return Err(no(format!(
                    "replace that reads L.{var} and draws random numbers, with an if that reads {var}, \
                     is not translated yet"
                )));
            }
            let helper = if drew {
                "stata_rows_replace_draws"
            } else {
                "stata_rows_replace"
            };
            self.helpers.insert("stata_rows_replace");
            let lagged = value
                .replace(
                    &format!("stata_ts({}, ", r_name(var)),
                    "stata_ts(stata_cur, ",
                )
                .replace(
                    &format!("dplyr::lag({}, ", r_name(var)),
                    "dplyr::lag(stata_cur, ",
                );
            format!("{helper}(function(stata_cur) {lagged}, {})", r_name(var))
        };
        out.package("dplyr");
        let d = self.data()?;
        out.line(format!(
            "if (!{} %in% names({d})) stop(\"variable {var} not found\")",
            r_text(var)
        ));
        self.step(c, format!("dplyr::mutate({} = {value})", r_name(var)), out)?;
        if kind == Kind::Text {
            self.kinds.insert(var.to_string(), Kind::Text);
        }
        out.caveat(
            "replace keeps the variable's storage type in Stata (a float stays a float); \
             the R keeps full precision unless the do-file made the variable",
        );
        Ok(())
    }

    // ---------------- rows and columns ----------------

    fn drop_keep(&mut self, c: &Command, keep: bool, out: &mut Out) -> Result<(), Untranslated> {
        let has_rows = c.parts.if_exp.is_some() || c.parts.in_range.is_some();
        let main = c.parts.main.trim();
        out.package("dplyr");
        if has_rows {
            if !main.is_empty() {
                return Err(no(format!("{} takes variables or rows, not both", c.name)));
            }
            let cond = self
                .drawing_condition(c, out)?
                .map(|(cond, _)| cond)
                .unwrap_or_default();
            let step = if keep {
                format!("dplyr::filter({cond})")
            } else {
                format!("dplyr::filter(!{})", wrap(&cond))
            };
            return self.step(c, step, out);
        }
        if main.is_empty() {
            return Err(no(format!("{} needs variables or an if/in", c.name)));
        }
        if c.prefixes.iter().any(|p| matches!(p, Prefix::By { .. })) {
            return Err(no(format!(
                "by with {} of variables: Stata refuses this",
                c.name
            )));
        }
        let sel = select(main)?;
        let d = self.data()?;
        if !keep && (main == "_all" || main == "*") {
            out.line(format!("{d} <- {d}[0, 0]"));
            return Ok(());
        }
        // keep: the variables stay in the data's order, as in Stata
        let step = if keep {
            format!("dplyr::select(-(!{}))", wrap_sel(&sel))
        } else {
            format!("dplyr::select(-{})", wrap_sel(&sel))
        };
        out.line(format!("{d} <- {d} |> {step}"));
        if !keep {
            for n in main.split_whitespace() {
                self.kinds.remove(n);
            }
        }
        Ok(())
    }

    fn rename(&mut self, c: &Command, opts: &Opts, out: &mut Out) -> Result<(), Untranslated> {
        let d = self.data()?;
        out.package("dplyr");
        let main = c.parts.main.trim();
        if opts.has("lower", 5) || opts.has("upper", 5) {
            let f = if opts.has("lower", 5) {
                "tolower"
            } else {
                "toupper"
            };
            out.line(format!(
                "{d} <- {d} |> dplyr::rename_with({f}, {})",
                select(main)?
            ));
            return Ok(());
        }
        if !c.options.is_empty() {
            return Err(no("rename with these options is not translated yet"));
        }
        // rename (a b) (c d), or rename a c
        let (olds, news): (Vec<String>, Vec<String>) = if main.starts_with('(') {
            let (a, b) = main
                .split_once(')')
                .ok_or_else(|| no("rename (old names) (new names) is not closed"))?;
            let b = b.trim().trim_start_matches('(').trim_end_matches(')');
            (
                a.trim_start_matches('(')
                    .split_whitespace()
                    .map(String::from)
                    .collect(),
                b.split_whitespace().map(String::from).collect(),
            )
        } else {
            let w: Vec<String> = main.split_whitespace().map(String::from).collect();
            match w.as_slice() {
                [a, b] => (vec![a.clone()], vec![b.clone()]),
                _ => return Err(no("rename needs an old and a new name")),
            }
        };
        if olds.len() != news.len() {
            return Err(no("rename: the old and new name lists differ in length"));
        }
        if olds.iter().chain(news.iter()).any(|n| !is_name(n)) {
            return Err(no("rename with wildcards is not translated yet"));
        }
        let pairs: Vec<String> = olds
            .iter()
            .zip(news.iter())
            .map(|(o, n)| format!("{} = {}", r_name(n), r_name(o)))
            .collect();
        out.line(format!("{d} <- {d} |> dplyr::rename({})", pairs.join(", ")));
        for (o, n) in olds.iter().zip(news.iter()) {
            if let Some(k) = self.kinds.remove(o) {
                self.kinds.insert(n.clone(), k);
            }
        }
        Ok(())
    }

    fn sort(&mut self, c: &Command, opts: &Opts, out: &mut Out) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[("stable", 6)]) {
            return Err(no(format!(
                "sort with the option {o} is not translated yet"
            )));
        }
        if c.parts.if_exp.is_some() || c.parts.in_range.is_some() {
            return Err(no("sort of part of the data (in) is not translated yet"));
        }
        let d = self.data()?;
        out.package("dplyr");
        let mut keys = format!("dplyr::pick({})", select(&c.parts.main)?);
        // . before .a before .z, as Stata sorts them
        if self.mv_on(out) {
            self.helpers.insert("stata_sort_keys");
            out.package("vctrs");
            keys = format!("stata_sort_keys({keys})");
        }
        out.line(format!("{d} <- {d} |> dplyr::arrange({keys})"));
        if !opts.has("stable", 6) {
            out.caveat(
                "Stata's sort puts rows that tie in no set order (unless stable); \
                 R keeps them in their current order",
            );
        }
        Ok(())
    }

    fn gsort(&mut self, c: &Command, opts: &Opts, out: &mut Out) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[("mfirst", 2), ("generate", 3)]) {
            return Err(no(format!(
                "gsort with the option {o} is not translated yet"
            )));
        }
        if opts.has("generate", 3) {
            return Err(no("gsort with generate() is not translated yet"));
        }
        let d = self.data()?;
        let mfirst = opts.has("mfirst", 2);
        let mut keys = Vec::new();
        let mut desc = false;
        for tok in c.parts.main.split_whitespace() {
            let (minus, name) = match tok.strip_prefix('-') {
                Some(n) => (true, n),
                None => (false, tok.strip_prefix('+').unwrap_or(tok)),
            };
            if name.is_empty() {
                desc = minus;
                continue;
            }
            if !is_name(name) {
                return Err(no(format!(
                    "gsort {tok}: only plain variable names are translated"
                )));
            }
            let n = r_name(name);
            let minus = minus || (desc && !tok.starts_with('+'));
            // the missing values among themselves: . < .a < ... < .z
            let places = self.kinds.get(name) != Some(&Kind::Text) && self.mv_on(out);
            if minus {
                // descending: missing values last unless mfirst
                if mfirst {
                    keys.push(format!("!is.na({n}) & {n} != \"\""));
                }
                keys.push(format!("dplyr::desc({n})"));
                if places {
                    keys.push(format!("dplyr::desc(stata_mv({n}))"));
                }
            } else {
                keys.push(n.clone());
                if places {
                    keys.push(format!("stata_mv({n})"));
                }
            }
        }
        if keys.is_empty() {
            return Err(no("gsort needs variables"));
        }
        out.package("dplyr");
        out.line(format!("{d} <- {d} |> dplyr::arrange({})", keys.join(", ")));
        if mfirst {
            out.caveat(
                "mfirst is translated for numbers; for text the empty string counts as missing",
            );
        }
        Ok(())
    }

    fn order(&mut self, c: &Command, opts: &Opts, out: &mut Out) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[
            ("first", 5),
            ("last", 4),
            ("before", 3),
            ("after", 1),
            ("alphabetic", 5),
        ]) {
            return Err(no(format!(
                "order with the option {o} is not translated yet"
            )));
        }
        let d = self.data()?;
        let sel = select(&c.parts.main)?;
        out.package("dplyr");
        let mut args = vec![sel.clone()];
        if let Some(b) = opts.arg("before", 3) {
            args.push(format!(".before = {}", r_name(b.trim())));
        } else if let Some(a) = opts.arg("after", 1) {
            args.push(format!(".after = {}", r_name(a.trim())));
        } else if opts.has("last", 4) {
            args.push(".after = dplyr::last_col()".into());
        }
        if opts.has("alphabetic", 5) {
            out.line(format!(
                "{d} <- {d} |> dplyr::relocate(dplyr::all_of(sort(names(dplyr::select({d}, {sel})), method = \"radix\")){})",
                args.iter().skip(1).map(|a| format!(", {a}")).collect::<String>()
            ));
        } else {
            out.line(format!(
                "{d} <- {d} |> dplyr::relocate({})",
                args.join(", ")
            ));
        }
        Ok(())
    }

    // ---------------- labels ----------------

    fn label(&mut self, c: &Command, opts: &Opts, out: &mut Out) -> Result<(), Untranslated> {
        let (sub, rest) = first_word(&c.parts.main);
        match sub {
            s if "variable".starts_with(s) && s.len() >= 3 => {
                let d = self.data()?;
                let (var, text) = first_word(rest);
                if !is_name(var) {
                    return Err(no(format!("{var} is not a variable name")));
                }
                let text = text.trim().trim_matches('"');
                // no text clears the label
                let value = if text.is_empty() {
                    "NULL".to_string()
                } else {
                    r_text(text)
                };
                out.line(format!("attr({d}${}, \"label\") <- {value}", r_name(var)));
                Ok(())
            }
            s if "define".starts_with(s) && s.len() >= 3 => {
                let (name, rest) = first_word(rest);
                if !is_name(name) {
                    return Err(no(format!("{name} is not a label name")));
                }
                let pairs = label_pairs(rest)?;
                let set = self.label_sets.entry(name.to_string()).or_default();
                let add = opts.has("add", 3);
                let modify = opts.has("modify", 6);
                // Stata's error, as it stops there (nothing is changed)
                if !set.is_empty() && !add && !modify && !opts.has("replace", 7) {
                    out.line(format!(
                        "stop({}, call. = FALSE)",
                        r_text(&format!("label {name} already defined (Stata: r(110))"))
                    ));
                    return Ok(());
                }
                if opts.has("replace", 7) {
                    set.clear();
                }
                for (v, t) in pairs {
                    match set.iter_mut().find(|(x, _)| *x == v) {
                        Some(slot) if modify => slot.1 = t,
                        Some(_) => return Err(no(format!("label define {name}: value {} already has a label (Stata wants modify)", r_number(v)))),
                        None => set.push((v, t)),
                    }
                }
                set.sort_by(|a, b| a.0.total_cmp(&b.0));
                let body: Vec<String> = set
                    .iter()
                    .map(|(v, t)| format!("{} = {}", r_text(t), r_number(*v)))
                    .collect();
                out.line(format!("{} <- c({})", label_object(name), body.join(", ")));
                Ok(())
            }
            s if "values".starts_with(s) && s.len() >= 3 => {
                let d = self.data()?;
                let w: Vec<&str> = rest.split_whitespace().collect();
                // with two or more words the last is the label name (or `.`)
                let (vars, lbl) = match w.split_last() {
                    Some((l, v)) if !v.is_empty() => (v.join(" "), *l),
                    _ => (rest.to_string(), "."),
                };
                out.package("haven");
                out.package("dplyr");
                let sel = select(&vars)?;
                if lbl == "." {
                    out.line(format!(
                        "{d} <- {d} |> dplyr::mutate(dplyr::across({sel}, \\(v) structure(haven::zap_labels(v), label = attr(v, \"label\", exact = TRUE))))"
                    ));
                } else {
                    let labels = if self.label_sets.contains_key(lbl) {
                        label_object(lbl)
                    } else {
                        // the data file's set; none (numbers, as in Stata) if it has none
                        out.caveat(format!("the label {lbl} is not defined earlier in this do-file; it is taken from the data file, and the values show as numbers if the file has no such label"));
                        self.helpers.insert("stata_label_set");
                        format!(
                            "stata_label_set({}, {})",
                            r_text(lbl),
                            self.file
                                .as_deref()
                                .map(r_text)
                                .unwrap_or_else(|| "NULL".into())
                        )
                    };
                    out.line(format!(
                        "{d} <- {d} |> dplyr::mutate(dplyr::across({sel}, \\(v) haven::labelled(as.numeric(v), labels = {labels}, label = attr(v, \"label\", exact = TRUE))))"
                    ));
                }
                Ok(())
            }
            s if "drop".starts_with(s) && s.len() >= 4 => {
                let names: Vec<&str> = rest.split_whitespace().collect();
                if names.contains(&"_all") {
                    let all: Vec<String> =
                        self.label_sets.keys().map(|n| label_object(n)).collect();
                    self.label_sets.clear();
                    if !all.is_empty() {
                        out.line(format!("rm({})", all.join(", ")));
                    }
                    return Ok(());
                }
                for n in &names {
                    self.label_sets.remove(*n);
                }
                out.line(format!(
                    "rm({})",
                    names
                        .iter()
                        .map(|n| label_object(n))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
                Ok(())
            }
            s if !s.is_empty() && "list".starts_with(s) => {
                let names: Vec<&str> = rest.split_whitespace().collect();
                for n in names {
                    out.line(format!("print({})", label_object(n)));
                }
                Ok(())
            }
            other => Err(no(format!("label {other} is not translated yet"))),
        }
    }

    // ---------------- looking at the data ----------------

    fn count(&mut self, c: &Command, out: &mut Out) -> Result<(), Untranslated> {
        let d = self.data()?;
        if c.prefixes.iter().any(|p| matches!(p, Prefix::By { .. })) {
            return Err(no("by with count is not translated yet"));
        }
        let n = match self.condition(c, out)? {
            Some(cond) => format!("nrow(dplyr::filter({d}, {cond}))"),
            None => format!("nrow({d})"),
        };
        out.package("dplyr");
        // r(N), as Stata leaves it
        out.line(format!("stata_r <- list(N = {n})"));
        self.r_new = Some((returned::R_COUNT, "count".into()));
        if !self.quiet(c) {
            out.line("cat(format(stata_r$N, big.mark = \",\"), \"\\n\")");
        }
        Ok(())
    }

    /// `levelsof`: the distinct values, printed and left in r(levels) as
    /// Stata 19 writes them (numbers in %18.0g, text in compound quotes);
    /// `local()` is set from r(levels) by the do-file runner once it has run.
    fn levelsof(&mut self, c: &Command, opts: &Opts, out: &mut Out) -> Result<(), Untranslated> {
        let d = self.data()?;
        if c.prefixes.iter().any(|p| matches!(p, Prefix::By { .. })) {
            return Err(no("by with levelsof is not translated yet"));
        }
        if let Some(o) = opts.other(&[("clean", 1), ("local", 3), ("missing", 4), ("separate", 3)])
        {
            return Err(no(format!(
                "levelsof with the option {o} is not translated yet"
            )));
        }
        let vars = names(&c.parts.main)?;
        let [var] = vars.as_slice() else {
            return Err(no(
                "levelsof takes one variable (Stata: too many variables specified)",
            ));
        };
        let rows = match self.condition(c, out)? {
            Some(cond) => format!("dplyr::filter({d}, {cond})"),
            None => d.to_string(),
        };
        let sep = opts.arg("separate", 3).unwrap_or_else(|| " ".into());
        out.package("dplyr");
        self.helpers.insert("stata_fmt");
        self.helpers.insert("stata_levelsof");
        let tags = if opts.has("missing", 4) && self.mv_on(out) {
            ", tags = TRUE"
        } else {
            ""
        };
        out.line(format!(
            "stata_r <- stata_levelsof({rows}[[{}]], clean = {}, sep = {}, missing = {}{tags})",
            r_text(var),
            if opts.has("clean", 1) {
                "TRUE"
            } else {
                "FALSE"
            },
            r_text(&sep),
            if opts.has("missing", 4) {
                "TRUE"
            } else {
                "FALSE"
            },
        ));
        self.r_new = Some((returned::R_LEVELSOF, "levelsof".into()));
        if !self.quiet(c) {
            out.line("cat(stata_r$levels, \"\\n\", sep = \"\")");
        }
        Ok(())
    }

    fn list(&mut self, c: &Command, opts: &Opts, out: &mut Out) -> Result<(), Untranslated> {
        let d = self.data()?;
        if self.quiet(c) {
            return Ok(());
        }
        if let Some(o) = opts.other(&[
            ("clean", 1),
            ("noobs", 3),
            ("separator", 3),
            ("sepby", 5),
            ("abbreviate", 2),
            ("nolabel", 3),
            ("table", 3),
        ]) {
            return Err(no(format!(
                "list with the option {o} is not translated yet"
            )));
        }
        let mut steps = vec![d.clone()];
        if let Some(cond) = self.condition(c, out)? {
            steps.push(format!("dplyr::filter({cond})"));
        }
        if !c.parts.main.trim().is_empty() {
            steps.push(format!("dplyr::select({})", select(&c.parts.main)?));
        }
        if !opts.has("nolabel", 3) {
            out.package("haven");
            steps.push("haven::as_factor()".into());
        }
        out.package("dplyr");
        out.line(format!("print(as.data.frame({}))", steps.join(" |> ")));
        out.caveat("printed in R's layout, not Stata's table");
        Ok(())
    }

    fn describe(&mut self, c: &Command, out: &mut Out) -> Result<(), Untranslated> {
        let d = self.data()?;
        let opts = Opts(&c.options);
        if !c.parts.main.trim().is_empty()
            || c.using.is_some()
            || opts.other(&[("short", 1)]).is_some()
        {
            return Err(no(
                "describe of chosen variables, a file or with options other than short is not \
                 translated yet",
            ));
        }
        // Stata's r(), quietly or not
        out.line(format!("stata_r <- list(k = ncol({d}), N = nrow({d}))"));
        self.r_new = Some((returned::R_DESCRIBE, "describe".into()));
        if self.quiet(c) {
            return Ok(());
        }
        if !opts.has("short", 1) {
            out.line(format!(
                "print(data.frame(variable = names({d}), type = vapply({d}, \\(v) class(v)[1], \"\"), label = vapply({d}, \\(v) {{ l <- attr(v, \"label\", exact = TRUE); if (is.null(l)) \"\" else l }}, \"\")), row.names = FALSE)"
            ));
        }
        out.line(format!(
            "cat(\"observations:\", nrow({d}), \" variables:\", ncol({d}), \"\\n\")"
        ));
        out.caveat("printed in R's layout, not Stata's");
        Ok(())
    }

    /// `ssc install name` (P2B.11): Stata's add-ons are out of scope, so
    /// nothing runs in R; the window offers the R package of the same name
    /// (`rc install`) when CRAN has one, and says so when it has none.
    fn ssc(&mut self, c: &Command, out: &mut Out) -> Result<(), Untranslated> {
        let (sub, rest) = first_word(&c.parts.main);
        if sub != "install" {
            return Err(no(format!(
                "ssc {sub} is not translated: ssc's add-ons are Stata's (P2B.11)"
            )));
        }
        let name = rest.trim();
        if !is_name(name) {
            return Err(no("ssc install needs one package name"));
        }
        if let Some(o) = Opts(&c.options).other(&[("replace", 7), ("all", 3)]) {
            return Err(no(format!(
                "ssc install with the option {o} is not translated"
            )));
        }
        self.ssc.push(name.to_string());
        out.caveat(format!(
            "ssc installs Stata's add-on {name}, whose commands are not translated; RCraft offers \
             the R package of the same name instead, when CRAN has one (rc install {name})"
        ));
        Ok(())
    }

    /// `expand [=]exp [if] [in] [, generate(newvar)]`.
    fn expand(&mut self, c: &Command, opts: &Opts, out: &mut Out) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[("generate", 1)]) {
            return Err(no(format!("expand with the option {o} is not translated")));
        }
        if c.prefixes.iter().any(|p| matches!(p, Prefix::By { .. })) {
            return Err(no("expand with by: is not translated yet"));
        }
        let d = self.data()?;
        let exp = c.parts.main.trim().trim_start_matches('=').trim();
        if exp.is_empty() {
            return Err(no("expand needs a count (Stata: invalid syntax)"));
        }
        let (cond, n, _) = self.drawing_value(c, exp, out)?;
        let n = match cond {
            Some(cond) => format!("dplyr::if_else({cond}, as.numeric({n}), 1)"),
            None => n,
        };
        let gen = match opts.arg("generate", 1) {
            Some(g) => {
                let g = g.trim().to_string();
                if self.kinds.contains_key(&g) {
                    return Err(no(format!(
                        "expand: {g} already exists (Stata: variable {g} already defined)"
                    )));
                }
                out.caveat("Stata labels the new variable's 0 and 1 (original, duplicated); here it has no value labels");
                self.kinds.insert(g.clone(), Kind::Number);
                format!(", gen = {}", r_text(&g))
            }
            None => String::new(),
        };
        self.helpers.insert("stata_expand");
        out.package("dplyr");
        out.line(format!(
            "{d} <- stata_expand({d}, dplyr::transmute({d}, stata_n = {n})$stata_n{gen})"
        ));
        Ok(())
    }

    /// `preserve`: a copy of the data in R, and of what is known of it.
    fn preserve(&mut self, c: &Command, opts: &Opts, out: &mut Out) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[]) {
            return Err(no(format!(
                "preserve with the option {o} is not translated"
            )));
        }
        if !c.parts.main.trim().is_empty() {
            return Err(no(
                "preserve takes no words after it (Stata: invalid syntax)",
            ));
        }
        out.line(match &self.data {
            Some(d) => format!("stata_preserved <- {d}"),
            None => "stata_preserved <- NULL".into(),
        });
        out.caveat("if the do-file ends before restore, Stata puts the preserved data back; R keeps the data as it is then");
        self.preserved = Some(Box::new(Preserved {
            data: self.data.clone(),
            file: self.file.clone(),
            kinds: self.kinds.clone(),
            fv_base: self.fv_base.clone(),
            label_sets: self.label_sets.clone(),
            panel: self.panel.clone(),
            panel_time: self.panel_time.clone(),
            ts_delta: self.ts_delta.clone(),
            ts_unknown: self.ts_unknown,
            stset: self.stset.clone(),
        }));
        Ok(())
    }

    /// `restore`: the data `preserve` kept, again; with `preserve` the copy
    /// stays for another restore, with `not` it is dropped unused.
    fn restore(&mut self, c: &Command, opts: &Opts, out: &mut Out) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[("preserve", 8), ("not", 3)]) {
            return Err(no(format!("restore with the option {o} is not translated")));
        }
        if !c.parts.main.trim().is_empty() {
            return Err(no(
                "restore takes no words after it (Stata: invalid syntax)",
            ));
        }
        let Some(p) = self.preserved.clone() else {
            return Err(no(
                "restore: nothing was preserved before it (Stata: no preserve)",
            ));
        };
        if opts.has("not", 3) {
            out.line("rm(stata_preserved)");
            self.preserved = None;
            return Ok(());
        }
        if let Some(now) = self.data.as_ref().filter(|d| Some(*d) != p.data.as_ref()) {
            out.line(format!("rm({now})"));
        }
        if let Some(d) = &p.data {
            out.line(format!("{d} <- stata_preserved"));
        }
        if !opts.has("preserve", 8) {
            out.line("rm(stata_preserved)");
            self.preserved = None;
        }
        self.data = p.data;
        self.file = p.file;
        self.kinds = p.kinds;
        self.fv_base = p.fv_base;
        self.label_sets = p.label_sets;
        self.panel = p.panel;
        self.panel_time = p.panel_time;
        self.ts_delta = p.ts_delta;
        self.ts_unknown = p.ts_unknown;
        self.stset = p.stset;
        // the rows may not be the ones the last model was fitted on
        self.esample = false;
        self.changed_since_fit = true;
        Ok(())
    }

    fn set(&mut self, c: &Command, out: &mut Out) -> Result<(), Untranslated> {
        let (what, rest) = first_word(&c.parts.main);
        match what {
            "seed" => self.set_seed(rest, out),
            "obs" => {
                let n: u64 = rest.trim().parse().map_err(|_| {
                    no(format!(
                        "set obs {}: only a whole number is translated",
                        rest.trim()
                    ))
                })?;
                self.helpers.insert("stata_set_obs");
                match self.data.clone() {
                    Some(d) => out.line(format!("{d} <- stata_set_obs({d}, {n})")),
                    None => {
                        out.line(format!("data <- stata_set_obs(NULL, {n})"));
                        self.data = Some("data".into());
                        self.file = None;
                        self.kinds.clear();
                        self.storage.clear();
                        self.fv_base.clear();
                        self.forget_ts();
                        self.label_sets.clear();
                    }
                }
                Ok(())
            }
            // the width matrix list wraps its columns at
            "linesize" => {
                let n: u32 = rest.trim().parse().map_err(|_| {
                    no(format!(
                        "set linesize {}: only a whole number is translated",
                        rest.trim()
                    ))
                })?;
                if !(40..=255).contains(&n) {
                    return Err(no(format!(
                        "set linesize {n} (Stata: linesize must be between 40 and 255)"
                    )));
                }
                out.line(format!("stata_linesize <- {n}"));
                Ok(())
            }
            "more" | "varabbrev" | "trace" | "rmsg" | "type" | "maxvar" | "matsize"
            | "niceness" | "processors" | "max_memory" | "min_memory" | "segmentsize" => {
                out.caveat(format!(
                    "nothing to do in R: set {what} only changes how Stata runs or prints"
                ));
                Ok(())
            }
            "scheme" => {
                out.caveat("nothing to do in R: set scheme only changes how Stata's graphs look; RCraft's graphs keep their own");
                Ok(())
            }
            // set memory, from before Stata 12: Stata 19 notes it is ignored (a
            // whole number, b k m or g after it), and r(198) for anything else
            m if m.len() >= 3 && "memory".starts_with(m) => {
                let n = rest.trim();
                let digits = n.trim_end_matches(['b', 'k', 'm', 'g', 'B', 'K', 'M', 'G']);
                let line = if n.is_empty() {
                    "stop(\"invalid syntax (Stata: r(198))\", call. = FALSE)".to_string()
                } else if !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()) {
                    "cat(\"set memory ignored.\\n    Memory no longer needs to be set in modern \
                     Statas; memory adjustments are performed on the fly automatically.\\n\")"
                        .to_string()
                } else {
                    format!(
                        "stop({}, call. = FALSE)",
                        r_text(&format!(
                            "'{n}' found where integer expected (Stata: r(198))"
                        ))
                    )
                };
                out.line(line);
                Ok(())
            }
            other => Err(no(format!("set {other} is not translated yet"))),
        }
    }

    /// `erase` (and `rm`): one file deleted; one that is not there stops with
    /// Stata's message and r(601). No extension is added (Stata 19).
    fn erase(&mut self, c: &Command, out: &mut Out) -> Result<(), Untranslated> {
        if !c.options.is_empty() {
            return Err(no(format!("{} takes no options", c.name)));
        }
        let main = c.parts.main.trim();
        let one = match main.strip_prefix('"') {
            Some(rest) => rest.strip_suffix('"').is_some_and(|f| !f.contains('"')),
            None => !main.is_empty() && !main.contains(char::is_whitespace),
        };
        if !one {
            return Err(no(format!("{} needs one file name", c.name)));
        }
        let path = file_name(main, "");
        let p = r_text(&path);
        out.line(format!(
            "if (!file.exists({p})) stop({})",
            r_text(&format!("file {path} not found"))
        ));
        out.line(format!("invisible(file.remove({p}))"));
        Ok(())
    }

    /// `assert`: where it is false, Stata's count of those rows (none for a
    /// single row), then `assertion is false` and r(9); `rc0` prints them
    /// and goes on, and no rows is `(null assertion)` (Stata 19).
    fn assert(&mut self, c: &Command, opts: &Opts, out: &mut Out) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[("rc0", 3)]) {
            return Err(no(format!(
                "assert with the option {o} is not translated yet"
            )));
        }
        if c.prefixes.iter().any(|p| matches!(p, Prefix::By { .. })) {
            return Err(no("by with assert is not translated yet"));
        }
        if c.parts.main.trim().is_empty() {
            return Err(no("assert needs an expression (Stata: invalid syntax)"));
        }
        let d = self.data()?;
        let test = self.expr(&c.parts.main, Use::Condition, out)?;
        let rows = match self.condition(c, out)? {
            Some(cond) => format!("dplyr::filter({d}, {cond})"),
            None => d,
        };
        out.package("dplyr");
        self.helpers.insert("stata_assert");
        out.line(format!(
            "stata_assert(dplyr::pull(dplyr::mutate({rows}, stata_ok = {test}), stata_ok){})",
            if opts.has("rc0", 3) {
                ", rc0 = TRUE"
            } else {
                ""
            }
        ));
        Ok(())
    }

    /// `fvset base #` and `fvset base default`, `fvset clear`: the base level
    /// later models give `i.x`, as if they wrote `ib#.x` (Stata 19). It stays
    /// with the data, so a file the do-file saves keeps it.
    fn fvset(&mut self, c: &Command, out: &mut Out) -> Result<(), Untranslated> {
        if !c.options.is_empty() {
            return Err(no("fvset with options is not translated yet"));
        }
        let (what, rest) = first_word(&c.parts.main);
        let (base, vars) = match what {
            "base" => {
                let (b, vars) = first_word(rest);
                match b {
                    "default" => (None, vars),
                    n if !n.is_empty() && n.chars().all(|ch| ch.is_ascii_digit()) => {
                        (Some(n.to_string()), vars)
                    }
                    other => {
                        return Err(no(format!(
                            "fvset base {other} is not translated yet: only a number or default"
                        )))
                    }
                }
            }
            "clear" => (None, rest),
            other => return Err(no(format!("fvset {other} is not translated yet"))),
        };
        let vars = names(vars)?;
        if vars.is_empty() {
            return Err(no("fvset needs variable names"));
        }
        for v in vars {
            match &base {
                Some(b) => {
                    self.fv_base.insert(v, b.clone());
                }
                None => {
                    self.fv_base.remove(&v);
                }
            }
        }
        out.caveat("nothing runs in R: RCraft writes later models' i. terms with this base level (as ib#.)");
        Ok(())
    }

    /// The line as the translators read it: `cluster(v)` beside a model is the
    /// older `vce(cluster v)`, which `robust` does not change, and `i.x` takes
    /// the base `fvset` chose (both read off Stata 19). `None` when nothing
    /// changes.
    pub(super) fn restated(&self, c: &Command) -> Option<Command> {
        let margins = c.name == "margins";
        if !MODELS.contains(&c.name.as_str()) && !margins {
            return None;
        }
        let mut c2 = c.clone();
        let mut changed = false;
        // options that change only how Stata's table looks (R prints its own),
        // in the shortest forms Stata 19 takes
        let shown = |o: &crate::lang::syntax::Opt| {
            DISPLAY_OPTIONS.iter().any(|(n, m)| o.is(n, *m))
                || (margins && MARGINS_DISPLAY.iter().any(|(n, m)| o.is(n, *m)))
        };
        if c2.options.iter().any(shown) {
            c2.options.retain(|o| !shown(o));
            c2.parts.options = c2
                .options
                .iter()
                .map(|o| match &o.raw {
                    Some(r) => format!("{}({r})", o.name),
                    None => o.name.clone(),
                })
                .collect::<Vec<_>>()
                .join(" ");
            changed = true;
        }
        if margins {
            return changed.then_some(c2);
        }
        let cluster = c.options.iter().find(|o| o.is("cluster", 2));
        if let (Some(cl), false) = (cluster, c.options.iter().any(|o| o.is("vce", 3))) {
            let v = format!("cluster {}", cl.arg().trim());
            c2.options
                .retain(|o| !o.is("cluster", 2) && !o.is("robust", 1));
            c2.options.push(crate::lang::syntax::Opt {
                name: "vce".into(),
                arg: Some(v.clone()),
                raw: Some(v),
            });
            c2.parts.options = c2
                .options
                .iter()
                .map(|o| match &o.raw {
                    Some(r) => format!("{}({r})", o.name),
                    None => o.name.clone(),
                })
                .collect::<Vec<_>>()
                .join(" ");
            changed = true;
        }
        // L(1/2).x is L.x L2.x, l.d.x is LD.x
        if let Ok(main) = ts::expand_ts(&c2.parts.main) {
            if main != c2.parts.main {
                c2.parts.main = main;
                changed = true;
            }
        }
        if !self.fv_base.is_empty() {
            let main = model::with_bases(&c2.parts.main, &self.fv_base);
            if main != c2.parts.main {
                c2.parts.main = main;
                changed = true;
            }
        }
        changed.then_some(c2)
    }
}

/// Where a Stata storage type puts a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Storage {
    Byte,
    Int,
    Long,
    Float,
    Double,
    Str,
}

fn storage(ty: &str) -> Option<Storage> {
    Some(match ty {
        "byte" => Storage::Byte,
        "int" => Storage::Int,
        "long" => Storage::Long,
        "float" => Storage::Float,
        "double" => Storage::Double,
        t if t.starts_with("str") => Storage::Str,
        _ => return None,
    })
}

/// A numeric value as Stata stores it in a variable of `storage`.
fn stored(
    value: &str,
    storage: Storage,
    helpers: &mut BTreeSet<&'static str>,
    out: &mut Out,
) -> String {
    match storage {
        Storage::Float => {
            helpers.insert("stata_float");
            format!("stata_float({value})")
        }
        Storage::Double | Storage::Str => value.to_string(),
        Storage::Byte | Storage::Int | Storage::Long => {
            let (lo, hi) = match storage {
                Storage::Byte => (-127, 100),
                Storage::Int => (-32767, 32740),
                _ => (-2147483647, 2147483620),
            };
            out.caveat("whole-number storage types keep the whole part, and values out of range become missing");
            format!("(\\(v) {{ v <- trunc(v); v[v < {lo} | v > {hi}] <- NA; v }})({value})")
        }
    }
}

fn wrap(code: &str) -> String {
    if code.starts_with('(') && code.ends_with(')') {
        code.to_string()
    } else {
        format!("({code})")
    }
}

fn wrap_sel(sel: &str) -> String {
    if sel.starts_with("c(") || !sel.contains([' ', ':', '(']) {
        sel.to_string()
    } else {
        format!("({sel})")
    }
}

fn label_object(name: &str) -> String {
    format!("label_{name}")
}

/// `1 "Yes" 2 "No"` as (value, text) pairs.
fn label_pairs(text: &str) -> Result<Vec<(f64, String)>, Untranslated> {
    let mut out = Vec::new();
    let mut rest = text.trim();
    while !rest.is_empty() {
        let (num, after) = first_word(rest);
        let v: f64 = num
            .parse()
            .map_err(|_| no(format!("label define: {num} is not a whole number")))?;
        let after = after.trim_start();
        let (t, next) = if let Some(q) = after.strip_prefix('"') {
            let end = q
                .find('"')
                .ok_or_else(|| no("label define: a label's quotes are not closed"))?;
            (q.get(..end).unwrap_or(""), q.get(end + 1..).unwrap_or(""))
        } else {
            first_word(after)
        };
        out.push((v, t.to_string()));
        rest = next.trim();
    }
    Ok(out)
}

/// The 1-based column of an Excel cell reference like `B3` or `AA1:C9`.
fn excel_column(cell: &str) -> Option<usize> {
    let letters: String = cell
        .chars()
        .take_while(|c| c.is_ascii_alphabetic())
        .collect();
    if letters.is_empty() {
        return None;
    }
    Some(
        letters
            .to_ascii_uppercase()
            .bytes()
            .fold(0usize, |n, b| n * 26 + usize::from(b - b'A' + 1)),
    )
}

/// Whether every `var[...]` in `exp` is an earlier row (`_n-k`, k a whole
/// number of 1 or more) or the row itself or a later one, so the replace can
/// be done again until it settles.
fn own_subscripts_earlier(exp: &str, var: &str) -> bool {
    use crate::lang::expr::{BinaryOp, Expr};
    fn walk(e: &Expr, var: &str) -> bool {
        match e {
            // an earlier row (`_n-k`, done again until it settles), or the row
            // itself or a later one (`_n`, `_n+k`, `_N`), which Stata reads
            // before replacing it, as R does
            Expr::Subscript(n, i) if n == var => match i.as_ref() {
                Expr::Name(m) => m == "_n" || m == "_N",
                Expr::Binary(op @ (BinaryOp::Sub | BinaryOp::Add), l, r) => {
                    matches!(l.as_ref(), Expr::Name(m) if m == "_n")
                        && matches!(r.as_ref(), Expr::Number(k)
                            if k.fract() == 0.0 && (*k >= 1.0 || (*op == BinaryOp::Add && *k >= 0.0)))
                }
                _ => false,
            },
            Expr::Subscript(_, i) => walk(i, var),
            Expr::Unary(_, a) => walk(a, var),
            Expr::Binary(_, a, b) => walk(a, var) && walk(b, var),
            Expr::Call(_, args) => args.iter().all(|a| walk(a, var)),
            _ => true,
        }
    }
    // `x = x[1]` alone: row 1 takes its own value, so every row gets the
    // first row's value as it was
    let first_alone = |e: &Expr| matches!(e, Expr::Subscript(n, i) if n == var && matches!(i.as_ref(), Expr::Number(k) if *k == 1.0));
    crate::lang::expr::parse(exp).is_ok_and(|e| first_alone(&e) || walk(&e, var))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reader::read;

    #[test]
    fn extended_missing_values_in_commands_go_by_their_places() {
        // a .dta may hold .a to .z (Stata 19, plan/stata/extended-missing.md)
        let out = r("use s
egen m = rowmax(a b)
egen n = rowmin(a b)
egen f = rowfirst(a b)
egen tg = tag(a), missing
recode a (1/.b = 9) (.d/1 = 4)
collapse (max) x = a (firstnm) y = a, by(g)");
        assert!(out[1].contains("r[a] <- m[[ncol(m)]][a]"), "{out:?}");
        assert!(out[2].contains("stata_untag(do.call(pmin"), "{out:?}");
        assert!(
            out[3].contains(
                r"Reduce(\(r, v) ifelse(is.na(v), r, v), rev(as.list(dplyr::pick(c(a, b)))))"
            ),
            "{out:?}"
        );
        assert!(
            out[4].contains("(stata_sort_keys(dplyr::pick(a)), TRUE)")
                && out[4].contains("& (rep(TRUE, nrow(k)))[s]"),
            "{out:?}"
        );
        assert!(
            out[5].contains(
                "((!is.na(a) & a >= 1) | (stata_mv(a) >= 1 & stata_mv(a) <= 3)) ~ 9, rep_len(FALSE, length(a)) ~ 4"
            ),
            "{out:?}"
        );
        assert!(
            out[6].contains("stata_mv_max(a)") && out[6].contains("stata_mv_firstnm(a)"),
            "{out:?}"
        );
        // without a .dta or a stored .a to .z the R is as before
        let out = r("clear
set obs 3
generate a = _n
generate b = .
egen m = rowmax(a b)
recode a (1/. = 9)
sort a");
        assert!(out.iter().all(|l| !l.contains("stata_mv")), "{out:?}");
        assert!(
            out.iter()
                .any(|l| l.contains("((!is.na(a) & a >= 1) | is.na(a)) ~ 9")),
            "{out:?}"
        );
    }

    #[test]
    fn a_file_name_becomes_a_name_r_can_assign_to() {
        assert_eq!(frame_name("study.dta"), "study");
        assert_eq!(frame_name("C:/data/My Study.dta"), "my_study");
        assert_eq!(frame_name("2024.dta"), "data_2024");
        // reserved in R, and the translation's own objects
        assert_eq!(frame_name("repeat.dta"), "data_repeat");
        assert_eq!(frame_name("function.dta"), "data_function");
        assert_eq!(frame_name("stata_b.dta"), "data_stata_b");
    }

    fn r(source: &str) -> Vec<String> {
        translate(&read(source))
            .lines
            .into_iter()
            .map(|l| match l.result {
                Ok(r) => r.code,
                Err(e) => format!("NOT: {}", e.reason),
            })
            .collect()
    }

    #[test]
    fn a_variable_named_twice_in_a_model_is_declined() {
        let out = r("use auto\nregress price mpg mpg weight");
        assert!(out[1].contains("NOT: mpg named twice"), "{out:?}");
        // a new fit takes the last one's results away first
        let out = r("use auto\nregress price mpg");
        assert!(out[1].starts_with("rm(list = intersect("), "{out:?}");
        assert!(
            out[1].trim_end().ends_with("stata_fit_failed <- FALSE"),
            "{out:?}"
        );
    }
    #[test]
    fn files_make_and_keep_the_data_frame() {
        let out = r("use \"study hours.dta\", clear\nsave results, replace\nsave results");
        assert_eq!(
            out[0],
            "study_hours <- haven::read_dta(\"study hours.dta\")"
        );
        assert_eq!(out[1], "haven::write_dta(study_hours, \"results.dta\")");
        assert!(out[2].starts_with("if (file.exists(\"results.dta\")) stop("));
        let out = r("use id score using panel, clear");
        assert_eq!(
            out[0],
            "panel <- haven::read_dta(\"panel.dta\", col_select = c(id, score))"
        );
        let out = r("import delimited using data.csv, clear\nimport excel survey.xlsx, sheet(\"Wave 2\") firstrow clear");
        assert!(out[0].starts_with(
            "data <- stata_read_delim(\"data.csv\", na = \"\", show_col_types = FALSE)"
        ));
        assert!(out[0].contains("dplyr::rename_with(tolower)"));
        assert!(
            out[1].starts_with("survey <- readxl::read_excel(\"survey.xlsx\", sheet = \"Wave 2\")")
        );
        assert!(r("gen x = 1")[0].starts_with("NOT: no data is in memory"));
    }

    #[test]
    fn ssc_install_asks_for_a_package_and_the_other_add_on_commands_are_refused() {
        let t = translate(&read(
            "ssc install estout, replace\nssc describe estout\nnet install x, from(y)\n",
        ));
        assert_eq!(t.ssc, ["estout".to_string()]);
        let first = t.lines.first().map(|l| l.result.clone());
        assert!(
            matches!(&first, Some(Ok(r)) if r.code.is_empty() && r.caveats.iter().any(|c| c.contains("rc install estout"))),
            "{first:?}"
        );
        let why: Vec<String> = t
            .lines
            .iter()
            .skip(1)
            .map(|l| l.result.clone().err().map(|e| e.reason).unwrap_or_default())
            .collect();
        assert!(
            why.first().is_some_and(|w| w.contains("add-ons are Stata")),
            "{why:?}"
        );
        assert!(
            why.get(1).is_some_and(|w| w.contains("out of scope")),
            "{why:?}"
        );
    }

    #[test]
    fn what_reads_r_back_comes_with_the_command_that_leaves_it() {
        // no model before it: summarize alone must bring stata_rget
        let t = translate(&read("use s\nsummarize y\ndisplay r(mean)\n"));
        assert!(t
            .helpers
            .iter()
            .any(|h| h.contains("stata_rget <- function")));
    }

    #[test]
    fn generate_and_replace_follow_statas_rules() {
        let all = r("use s\ngen y = x * 2 if g == 1\ngen double z = 0.1\ngen str5 t = \"a\" + s if x > 1\nreplace y = 0 if y == .\nbysort g: gen n = _n\nreplace x = x[_n-1] if missing(x)");
        // the last line of each is the change; the first checks the name
        assert_eq!(
            all[1].lines().next(),
            Some("if (\"y\" %in% names(s)) stop(\"variable y already defined\")")
        );
        let out: Vec<&str> = all.iter().map(|s| s.lines().last().unwrap_or("")).collect();
        // the data was read from a file, so .a to .z may be in x: x * 2 is `.` there
        assert_eq!(out[1], "s <- s |> dplyr::mutate(y = dplyr::if_else((!is.na(g) & g == 1), stata_float(stata_untag((x * 2))), NA_real_))");
        assert_eq!(out[2], "s <- s |> dplyr::mutate(z = 0.1)");
        // missing x counts as larger than 1; str5 keeps 5 characters
        assert_eq!(out[3], "s <- s |> dplyr::mutate(t = dplyr::if_else((is.na(x) | x > 1), substr(paste0(\"a\", s), 1, 5), \"\"))");
        assert_eq!(
            out[4],
            "s <- s |> dplyr::mutate(y = dplyr::if_else((stata_mv(y) == 1), stata_float(0), y))"
        );
        // a .dta may hold .a to .z: each is a group of its own
        assert_eq!(out[5], "s <- s |> dplyr::arrange(stata_sort_keys(dplyr::pick(g))) |> dplyr::group_by(dplyr::pick(g), stata_tag_k = stata_tag_key(dplyr::pick(g))) |> dplyr::mutate(n = stata_float(dplyr::row_number())) |> dplyr::ungroup() |> dplyr::select(-stata_tag_k)");
        // carried down row by row: each missing row takes the row above, already filled
        assert!(
            out[6].contains("stata_rows_replace(function(stata_cur)")
                && out[6].contains("dplyr::lag(stata_cur, 1)"),
            "{}",
            out[6]
        );
    }

    #[test]
    fn rows_and_columns() {
        let out = r("use s\ndrop if x > 5\nkeep in 1/10\ndrop a b*\nkeep c-e id\nrename (a b) (c d)\nsort g x\ngsort -x g\norder id, first");
        assert_eq!(out[1], "s <- s |> dplyr::filter(!(is.na(x) | x > 5))");
        assert_eq!(
            out[2],
            "s <- s |> dplyr::filter(dplyr::between(dplyr::row_number(), 1, 10))"
        );
        assert_eq!(
            out[3],
            "s <- s |> dplyr::select(-c(a, dplyr::matches(\"^b.*$\")))"
        );
        assert_eq!(out[4], "s <- s |> dplyr::select(-(!c(c:e, id)))");
        assert_eq!(out[5], "s <- s |> dplyr::rename(c = a, d = b)");
        // a .dta may hold .a to .z: they sort by their places
        assert_eq!(
            out[6],
            "s <- s |> dplyr::arrange(stata_sort_keys(dplyr::pick(c(g, x))))"
        );
        assert_eq!(
            out[7],
            "s <- s |> dplyr::arrange(dplyr::desc(x), dplyr::desc(stata_mv(x)), g, stata_mv(g))"
        );
        assert_eq!(out[8], "s <- s |> dplyr::relocate(id)");
    }

    #[test]
    fn labels_are_kept_as_labelled_columns() {
        let out = r("use s\nlabel define yn 0 \"No\" 1 \"Yes\"\nlabel values smoker yn\nlabel variable smoker \"Smokes daily\"\nlabel define yn 2 \"Maybe\", add");
        assert_eq!(out[1], "label_yn <- c(\"No\" = 0, \"Yes\" = 1)");
        assert!(out[2].contains("haven::labelled(as.numeric(v), labels = label_yn"));
        assert_eq!(out[3], "attr(s$smoker, \"label\") <- \"Smokes daily\"");
        assert_eq!(
            out[4],
            "label_yn <- c(\"No\" = 0, \"Yes\" = 1, \"Maybe\" = 2)"
        );
        // a set that exists: Stata's error, as it stops there
        assert!(r("label define a 1 \"x\"\nlabel define a 2 \"y\"")[1]
            .contains("label a already defined (Stata: r(110))"));
    }

    #[test]
    fn what_is_not_translated_says_why() {
        for (src, why) in [
            (
                "use s\nstreg x, dist(weibull)",
                "streg is not translated yet",
            ),
            (
                "use s\nforeach v of varlist x y {\nsum `v'\n}",
                "do-file runner",
            ),
            ("use s\nanova y g [aw = w]", "with weights"),
            ("use s\nset seed abc", "only a number"),
            ("use s\ngen x", "newvar = exp"),
        ] {
            let out = r(src);
            assert!(out.iter().any(|l| l.contains(why)), "{src}: {out:?}");
        }
        let t = translate(&read("use s\nset seed 42\nset more off"));
        let script = t.script();
        assert!(script.contains("stata_set_seed(42)"), "{script}");
        assert!(!script.contains("set.seed("), "{script}");
    }

    #[test]
    fn bsample_draws_stata_s_sample_in_r() {
        let out = r("use s\nset seed 1\nbsample\nbsample 5\nbsample if x > 1\n\
                     bsample, cluster(g) idcluster(nc)\nsummarize nc");
        assert_eq!(out[2], "s <- stata_bsample(s)");
        assert_eq!(out[3], "s <- stata_bsample(s, n = 5)");
        assert!(
            out[4].starts_with("s <- stata_bsample(s, sel = dplyr::transmute(s, stata_sel = "),
            "{out:?}"
        );
        assert_eq!(
            out[5],
            "s <- stata_bsample(s, cluster = \"g\", idcluster = \"nc\")"
        );
        assert!(!out[6].starts_with("NOT: "), "{out:?}");
        // shapes not observed in Stata, and the draws' state, say why
        for (src, why) in [
            ("use s\nbsample 3 if x > 1", "if or in and a sample size"),
            ("use s\nbsample, strata(g)", "strata"),
            ("use s\nbsample n", "not a number written out"),
            ("use s\nbsample, idcluster(k)", "needs cluster()"),
            ("use s\nversion 13\nbsample", "KISS32"),
        ] {
            let out = r(src);
            assert!(out.iter().any(|l| l.contains(why)), "{src}: {out:?}");
        }
    }

    #[test]
    fn bootstrap_runs_the_command_on_stata_s_samples() {
        let out = r("use s\nbootstrap m=r(mean), reps(5) seed(3): summarize y\ndisplay _se[m]");
        let b = &out[1];
        assert!(b.contains("stata_set_seed(3)"), "{b}");
        assert!(
            b.contains("stata_bs_begin(\"summarize\", FALSE, 5, FALSE)"),
            "{b}"
        );
        assert!(
            b.contains("stata_bsample(stata_bs_full[stata_bs_rows, , drop = FALSE])"),
            "{b}"
        );
        assert!(b.contains("stata_float(stats::setNames(c("), "{b}");
        assert!(!out[2].starts_with("NOT: "), "{out:?}");
        let out = r("use s\nbootstrap _b, reps(4) nodots: regress y x");
        assert!(out[1].contains("stata_bs_rows <- stata_esample"), "{out:?}");
        for (src, why) in [
            (
                "use s\nbootstrap, reps(5): summarize y",
                "expression list required",
            ),
            ("use s\nbootstrap r(mean), saving(f): summarize y", "saving"),
            (
                "use s\nbootstrap _b: anova y g",
                "after regress, logit and probit",
            ),
            ("use s\nbootstrap r(mean)", "a command after a colon"),
        ] {
            let out = r(src);
            assert!(out.iter().any(|l| l.contains(why)), "{src}: {out:?}");
        }
    }

    #[test]
    fn an_if_with_one_value_for_the_data_is_given_each_row() {
        let out = r(
            "use s\ngenerate z = x if _N > 5\nreplace z = 1 if x[1] == 2\ngenerate w = x if x > 1",
        );
        assert!(out[1].contains("dplyr::if_else(rep_len("), "{out:?}");
        assert!(out[2].contains("dplyr::if_else(rep_len("), "{out:?}");
        assert!(!out[3].contains("rep_len("), "{out:?}");
    }

    #[test]
    fn random_draws_are_statas_own() {
        let out = r("set obs 6\nset seed 11\ngenerate g = mod(_n, 2)\n\
                     generate double x = runiform() if g == 1\n\
                     replace x = rnormal(10, 2) in 2/3\n\
                     bysort g: generate y = rgamma(2.5, 1)\n\
                     keep if runiform() < .9\n\
                     display runiform(2, 5)\n\
                     expand 1 + (runiform() < .5) if g == 1");
        assert_eq!(out[1], "stata_set_seed(11)");
        // the rows the if selects draw, one each, in order
        assert!(
            out[3].ends_with("data <- data |> dplyr::mutate(x = dplyr::if_else(rep_len((!is.na(g) & g == 1), dplyr::n()), stata_runiform(rep_len((!is.na(g) & g == 1), dplyr::n())), NA_real_))"),
            "{out:?}"
        );
        assert!(
            out[4].contains("stata_rnormal(rep_len(dplyr::between(dplyr::row_number(), 2, 3), dplyr::n()), 10, 2)"),
            "{out:?}"
        );
        assert!(out[5].contains("dplyr::group_by(dplyr::pick(g)) |> dplyr::mutate(y = stata_float(stata_rgamma(dplyr::n(), 2.5, 1)))"), "{out:?}");
        // the missing rule writes the value twice, so the draw is named once
        assert!(
            out[6].contains("dplyr::filter(local({.stata_draw <- stata_runiform(dplyr::n()); (!is.na(.stata_draw) & .stata_draw < 0.9)}))"),
            "{out:?}"
        );
        assert!(out[7].contains("stata_runiform(1, 2, 5)"), "{out:?}");
        assert!(
            out[8].contains("stata_runiform(rep_len((!is.na(g) & g == 1), dplyr::n()))"),
            "{out:?}"
        );
        assert!(out.iter().all(|l| !l.starts_with("NOT")), "{out:?}");
        // what is not translated says why
        for (src, why) in [
            (
                "set obs 2\ngenerate x = runiform() + rnormal()",
                "second random draw",
            ),
            (
                "set obs 2\ngenerate x = runiform() if runiform() < .5",
                "both the value and the if",
            ),
            ("set obs 2\nsummarize if runiform() < .5", "not here yet"),
            ("set obs 2\ngenerate x = rgamma(2)", "with 1 arguments"),
            (
                "set obs 2\nkeep if runiform() < .5 in 1",
                "together with in",
            ),
            ("set seed 2147483648", "(Stata: '2147483648' found"),
            (
                "version 13\nset obs 2\ngenerate x = uniform()",
                "older generator (KISS32)",
            ),
        ] {
            let out = r(src);
            assert!(out.iter().any(|l| l.contains(why)), "{src}: {out:?}");
        }
        // after a line that drew and was not translated, draws wait for a set seed
        let out = r("set obs 2\ngenerate a = rpoisson(2)\ngenerate b = runiform()\nset seed 3\ngenerate c = runiform()");
        assert!(out[2].contains("not Stata's: a line before it that draws was not translated (generate a = rpoisson(2))"), "{out:?}");
        assert!(!out[4].starts_with("NOT"), "{out:?}");
    }

    /// Forms found in published replication packages (P2C.9).
    #[test]
    fn forms_from_replication_packages() {
        let out = r("erase \"a b.dta\"
rm x.dta
erase two words
compress
set niceness 6");
        assert_eq!(
            out[0],
            "if (!file.exists(\"a b.dta\")) stop(\"file a b.dta not found\")\ninvisible(file.remove(\"a b.dta\"))"
        );
        assert!(out[1].contains("file.remove(\"x.dta\")"), "{out:?}");
        assert!(
            out[2].starts_with("NOT: erase needs one file name"),
            "{out:?}"
        );
        assert!(!out[3].starts_with("NOT"), "{out:?}");
        assert!(!out[4].starts_with("NOT"), "{out:?}");
        // cluster() and cl() are vce(cluster); robust beside them goes
        let out = r("use s
regress y x, robust cl(g)
regress y x, vce(cluster g)");
        // the same R but e(cmdline), which is the line as typed
        assert_eq!(out[1].replace("robust cl(g)", "vce(cluster g)"), out[2]);
        // fvset base: later models write i.x as ib#.x, until default or clear
        let out = r("use s
fvset base 3 g
regress y i.g
regress y g#h
fvset base default g
regress y i.g");
        assert!(out[2].contains("stata_ib(g, \"3\")"), "{out:?}");
        assert!(out[3].contains("stata_cells(g, h, \"3\", \"\")"), "{out:?}");
        assert!(!out[5].contains("stata_ib"), "{out:?}");
        assert!(r("use s\nfvset base frequent g")[1].starts_with("NOT"));
        // and the data keeps it: save and use bring it back, import drops it
        let out = r("use s
fvset base 2 g
save t, replace
import delimited c.csv, clear
regress y i.g
use t, clear
regress y i.g");
        assert!(!out[4].contains("stata_ib"), "{out:?}");
        assert!(out[6].contains("stata_ib(g, \"2\")"), "{out:?}");
        // a label set goes with the data it was defined in
        let out = r("import delimited c.csv, clear
label define yes 1 \"yes\"
import delimited c.csv, clear
label define yes 1 \"yes\"
label define yes 1 \"no\"");
        assert!(!out[3].starts_with("NOT"), "{out:?}");
        assert!(
            out[4].contains("label yes already defined (Stata: r(110))"),
            "{out:?}"
        );
        // reshape long without j() makes _j; append with force
        let out = r("use s
reshape long v, i(id)
append using t, force");
        assert!(out[1].contains("\"_j\""), "{out:?}");
        assert!(
            out[2].contains("stata_forced(s, stata_same_dates(s, haven::read_dta(\"t.dta\")))"),
            "{out:?}"
        );
    }

    #[test]
    fn assert_and_tabulate_generate() {
        let out = r("use s
assert x > 2 if g == 1, rc0
assert x > 2, fast
tabulate g if x > 0, generate(d) missing
tabulate g h, generate(d)");
        assert_eq!(
            out[1],
            "stata_assert(dplyr::pull(dplyr::mutate(dplyr::filter(s, (!is.na(g) & g == 1)), stata_ok = (is.na(x) | x > 2)), stata_ok), rc0 = TRUE)"
        );
        assert!(
            out[2].starts_with("NOT: assert with the option fast"),
            "{out:?}"
        );
        assert!(
            out[3].ends_with("s <- stata_tab_generate(s, \"g\", \"d\", sel = dplyr::pull(dplyr::mutate(s, stata_sel = (is.na(x) | x > 0)), stata_sel), missing = TRUE, tags = TRUE)"),
            "{out:?}"
        );
        assert!(out[4].starts_with("NOT"), "{out:?}");
    }

    #[test]
    fn time_series_operators() {
        let out = r("clear
set obs 5
generate t = _n
generate x = _n
generate L1 = L.x
tsset t
generate a = L.x + D2.x
regress x l.d.x L(1/2).t
predict p
by t: generate b = L.x
replace t = L.x
xtset t
generate c = L.x
tsset t
tsset, clear
generate d = F.x
tsset t
replace x = L.x + 1 if x > 2
replace t = D.t");
        assert!(out[4].contains("time variable not set"), "{out:?}");
        assert!(out[5].contains("stata_tsset(data, \"t\")"), "{out:?}");
        assert!(
            out[6].contains("stata_ts(x, 1, 0, 0, t, NULL, 1)")
                && out[6].contains("stata_ts(x, 0, 2, 0, t, NULL, 1)"),
            "{out:?}"
        );
        assert!(
            out[7].contains("LD.x = stata_ts(x, 1, 1, 0, t, NULL, 1)")
                && out[7].contains("L2.t = stata_ts(t, 2, 0, 0, t, NULL, 1)")
                && out[7].contains("~ LD.x + L.t + L2.t"),
            "{out:?}"
        );
        assert!(out[8].contains("dplyr::mutate(data, LD.x = "), "{out:?}");
        assert!(
            out[9].starts_with("NOT: by with time-series operators"),
            "{out:?}"
        );
        assert!(!out[10].starts_with("NOT"), "{out:?}");
        // xtset with only a panel leaves no time
        assert!(out[12].contains("time variable not set"), "{out:?}");
        assert!(out[15].contains("time variable not set"), "{out:?}");
        assert!(
            out[17].contains("stata_rows_replace(function(stata_cur) dplyr::if_else("),
            "{out:?}"
        );
        assert!(
            out[17].contains("stata_ts(stata_cur, 1, 0, 0, t, NULL, 1)"),
            "{out:?}"
        );
        assert!(
            out[18].starts_with("NOT: replace that reads t at this or a later time"),
            "{out:?}"
        );
    }

    #[test]
    fn a_file_saved_after_cd_keeps_its_settings_when_read_by_its_full_path() {
        let out = r("clear
set obs 3
generate t = _n
generate x = _n
cd \"C:/work/data\"
tsset t
save FOMC, replace
use \"C:/Work/Data/fomc.dta\", clear
generate l = L.x
cd sub
use \"C:/work/data/other\", clear
generate m = L.x");
        assert!(!out[8].starts_with("NOT"), "{out:?}");
        assert!(
            out[8].contains("stata_ts(x, 1, 0, 0, t, NULL, 1)"),
            "{out:?}"
        );
        assert!(out[11].contains("not known here"), "{out:?}");
    }

    #[test]
    fn display_options_change_only_the_table() {
        let out = r("use s
regress y x, base cf(%5.3f) noomit vsquish
xtset id t
xtreg y x, fe vce(robust) baselevel
margins, dydx(x) asobserved cformat(%5.3f)
margins, dydx(x) aso
regress y x, vsq");
        assert!(out[1].contains("stats::lm(y ~ x"), "{out:?}");
        assert!(!out[3].starts_with("NOT"), "{out:?}");
        assert!(!out[4].starts_with("NOT"), "{out:?}");
        // shorter than Stata takes: refused as Stata refuses them
        assert!(
            out[5].starts_with("NOT: margins with the option aso"),
            "{out:?}"
        );
        assert!(
            out[6].starts_with("NOT: regress with the option vsq"),
            "{out:?}"
        );
    }

    #[test]
    fn estimates_store_and_restore() {
        let out = r("use s
estimates store a
regress y x
est sto a
logit d x
est res a
predict p
estimates drop a
estimates restore a
estimates restore zz");
        assert!(
            out[1].starts_with("NOT: estimates store: no model"),
            "{out:?}"
        );
        assert!(out[3].contains("stata_stored[[\"a\"]] <- mget("), "{out:?}");
        assert!(out[5].contains("list2env(stata_stored[[\"a\"]]"), "{out:?}");
        assert!(out[5].contains("(results a are active now)"), "{out:?}");
        // predict reads the model restored: regress's linear prediction
        assert!(!out[6].starts_with("NOT"), "{out:?}");
        assert!(!out[6].contains("type = \"response\""), "{out:?}");
        // a name no longer stored is Stata's error, r(111)
        assert_eq!(out[8], "stop(\"estimation result a not found\")");
        assert_eq!(out[9], "stop(\"estimation result zz not found\")");
        // after a line that was not translated it may be stored in Stata
        let out = r("use s
regress y x
frobnicate m
estimates restore m
predict p");
        assert!(out[3].starts_with("NOT: estimates restore m"), "{out:?}");
        assert!(out[4].starts_with("NOT"), "{out:?}");
    }

    #[test]
    fn set_obs_and_capture_and_rc() {
        let out = r("clear
set obs 3
generate x = _n
set obs 5");
        assert_eq!(out[1], "data <- stata_set_obs(NULL, 3)");
        assert_eq!(out[3], "data <- stata_set_obs(data, 5)");
        assert!(r("set obs many")[0].starts_with("NOT: set obs many"));
        let out = r("use s
capture drop nosuch
display _rc");
        assert!(out[1].starts_with("stata_rc <- tryCatch("), "{out:?}");
        assert!(out[1].ends_with("stata_captured(e, FALSE))"), "{out:?}");
        assert!(out[2].contains("stata_rc"), "{out:?}");
        // capture noisily shows Stata's message for the error
        let out = r("use s
capture noisily display nosuch");
        assert!(out[1].ends_with("stata_captured(e, TRUE))"), "{out:?}");
        // a line under capture that is not translated leaves _rc unknown
        let out = r("use s
capture frobnicate
display _rc
capture drop x
display _rc");
        assert!(out[2].starts_with("NOT: _rc is not known"), "{out:?}");
        assert!(out[4].contains("stata_rc"), "{out:?}");
    }

    #[test]
    fn a_model_that_is_not_translated_hides_the_one_before() {
        let out = r("use s
regress y x
qreg y x, nolog
estat ic
predict yh
display _b[x]
regress y x
estat ic");
        assert!(out[2].starts_with("NOT: qreg"), "{out:?}");
        assert!(
            out[3].starts_with("NOT: estat: the model Stata holds here is not known: qreg y x"),
            "{out:?}"
        );
        assert!(out[4].starts_with("NOT: predict: the model"), "{out:?}");
        assert!(out[5].starts_with("NOT: "), "{out:?}");
        assert!(!out[7].starts_with("NOT: "), "{out:?}");
        // a line that only fails to translate for a display reason keeps it
        let out = r("use s
regress y x
list frobnicate in 1/3
estat ic");
        assert!(!out[3].starts_with("NOT: "), "{out:?}");
        // so do the commands that only read the model, unless they post
        let out = r("use s
regress y x
testparm x
lincom x
estimates store m1
lincom x
margins, dydx(x) post
lincom x");
        assert!(out[2].starts_with("NOT: "), "{out:?}");
        assert!(!out[3].starts_with("NOT: "), "{out:?}");
        assert!(!out[5].starts_with("NOT: "), "{out:?}");
        assert!(out[7].starts_with("NOT: "), "{out:?}");
        // (after a line that was not translated, m1 may have been stored in Stata)
        let out = r("use s
frobnicate
regress y x
estimates restore m1
lincom x");
        assert!(out[3].starts_with("NOT: estimates restore m1"), "{out:?}");
        assert!(out[4].starts_with("NOT: lincom: the model"), "{out:?}");
    }

    #[test]
    fn preserve_and_restore_bring_the_data_back() {
        let out = r("use a
preserve
use b
restore
summarize x
restore");
        assert_eq!(out[1], "stata_preserved <- a");
        assert_eq!(
            out[3],
            "rm(b)
a <- stata_preserved
rm(stata_preserved)"
        );
        assert!(out[4].contains("a"), "{out:?}");
        assert!(
            out[5].starts_with("NOT: restore: nothing was preserved"),
            "{out:?}"
        );
        let out = r("use a
preserve
keep if x > 1
restore, preserve
restore, not");
        assert_eq!(out[3], "a <- stata_preserved");
        assert_eq!(out[4], "rm(stata_preserved)");
    }

    #[test]
    fn quietly_and_capture_hide_what_a_line_prints() {
        let out = r(concat!(
            "use s\nquietly display \"hidden\"\ncapture display \"hidden\"\n",
            "capture noisily display \"shown\"\nnoisily quietly display \"hidden\"\n",
            "quietly noisily display \"shown\"\ndisplay \"shown\"\n",
        ));
        let hidden: Vec<bool> = out
            .iter()
            .skip(1)
            .map(|c| c.contains("capture.output("))
            .collect();
        assert_eq!(hidden, [true, true, false, true, false, false], "{out:?}");
        // a line that changes the data and is not translated: the data after it is not known
        let out = r("use a\nframe change other\ndisplay _N\nuse b\ndisplay _N");
        assert!(
            out[2].starts_with("NOT: the data in memory is not known here"),
            "{out:?}"
        );
        assert!(!out[4].starts_with("NOT"), "{out:?}");
        let out = r("use a\nkeep if rpoisson(1) < .5\nsummarize x\nlabel variable x \"X\"");
        assert!(
            out[2].starts_with("NOT: the data in memory is not known here"),
            "{out:?}"
        );
        // .a to .z compare by their place in Stata's order
        let out = r("display . < .a
display . == .");
        assert!(out[0].contains("as.numeric(TRUE)"), "{out:?}");
        assert!(!out[1].starts_with("NOT"), "{out:?}");
        // a recode by a missing value takes that one only (Stata 19)
        let out = r("use s\nrecode x (.d = 9) (.w = .) (1 = .a) (.a/.w = 5)");
        assert!(
            out[1].contains("stata_mv(x) %in% c(5) ~ 9")
                && out[1].contains("stata_mv(x) %in% c(24) ~ NA_real_")
                && out[1].contains("x %in% c(1) ~ haven::tagged_na(\"a\")")
                && out[1].contains("(stata_mv(x) >= 2 & stata_mv(x) <= 24) ~ 5"),
            "{out:?}"
        );
        // a variable whose line was not translated: the lines that use it say so
        let out = r(concat!(
            "use a\ngenerate double y = rpoisson(2)\ngenerate z = y * 2\nregress z x\n",
            "display \"y is fine in text\"\ngenerate y = 1\nsummarize y\ngenerate w = L.x\n",
        ));
        assert!(
            out[2].starts_with("NOT: y is not known here: the line that makes it"),
            "{out:?}"
        );
        assert!(out[3].starts_with("NOT: z is not known here"), "{out:?}");
        assert!(!out[4].starts_with("NOT"), "{out:?}");
        assert!(out[6].starts_with("NOT: y is not known"), "{out:?}");
        assert!(out[7].contains("time-series operators"), "{out:?}");
        let out = r("use a
scalar s = r(nope)
generate z = x * s
scalar s = 2
generate w = x * s");
        assert!(out[2].starts_with("NOT: s is not known here"), "{out:?}");
        assert!(!out[4].starts_with("NOT"), "{out:?}");
        // a command that hides its own output is not wrapped again
        let out = r("use s\nquietly summarize x");
        assert!(!out[1].contains("capture.output("), "{out:?}");
    }
}
