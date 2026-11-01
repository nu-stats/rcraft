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

mod anova;
mod graph;
mod iv;
mod margins;
mod matrices;
mod ml;
mod model;
mod more;
mod post;
mod random;
mod returned;
mod scalars;
mod st;
mod stats;
mod tables;
mod weights;
mod xt;

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
            self.s.esample = false;
            self.s.changed_since_fit = true;
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
        Opener::Program { name } => format!("program {name}"),
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
    "stcox",
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

const LEAVE_DATA_ALONE: &[&str] = &[
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

/// R's version of Stata's float storage: a double rounded to the nearest
/// single-precision value, missing kept missing.
const FLOAT_HELPER: &str = "stata_float <- function(x) {\n  y <- readBin(writeBin(as.numeric(x), raw(), size = 4), \"double\", size = 4, n = length(x))\n  y[is.na(x) | !is.finite(y)] <- NA\n  y\n}";

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

const LEVELSOF_HELPER: &str = r#"stata_levelsof <- function(x, clean = FALSE, sep = " ", missing = FALSE) {
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
    if (missing && anyNA(x)) s <- c(s, ".")
  }
  list(levels = paste(s, collapse = sep), r = length(s), N = sum(keep))
}"#;

/// Names for Excel columns, A to Z, AA, AB ..., from the `first`-th column.
const EXCEL_NAMES_HELPER: &str = "stata_excel_names <- function(n, first = 1) {\n  vapply(seq_len(n) + first - 1, function(i) {\n    s <- \"\"\n    while (i > 0) {\n      s <- paste0(LETTERS[(i - 1) %% 26 + 1], s)\n      i <- (i - 1) %/% 26\n    }\n    s\n  }, \"\")\n}";

fn helper_code(name: &str) -> &'static str {
    match name {
        "stata_excel_names" => EXCEL_NAMES_HELPER,
        "stata_stats" => more::STAT_HELPERS,
        "stata_destring" => more::DESTRING_HELPER,
        "stata_tostring" => more::TOSTRING_HELPER,
        "stata_summarize" => stats::SUMMARIZE_HELPER,
        "stata_tab_values" => stats::TAB_VALUES_HELPER,
        "stata_tabulate" => stats::TABULATE_HELPER,
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
        "stata_estat" => post::ESTAT_HELPER,
        "stata_margins" => margins::MARGINS_HELPER,
        "stata_unconditional" => margins::UNCONDITIONAL_HELPER,
        "stata_anova" => anova::ANOVA_HELPER,
        "stata_xtset" => xt::XTSET_HELPER,
        "stata_xtreg" => xt::XTREG_HELPER,
        "stata_xt_predict" => xt::XT_PREDICT_HELPER,
        "stata_xt_ic" => xt::XT_IC_HELPER,
        "stata_xt_other" => xt::XT_OTHER_HELPER,
        "stata_ivregress" => iv::IV_HELPER,
        "stata_stset" => st::STSET_HELPER,
        "stata_graph" => graph::GRAPH_HELPER,
        "stata_stcox" => st::STCOX_HELPER,
        "stata_phtest" => st::PHTEST_HELPER,
        "stata_returned" => returned::RETURNED_HELPER,
        "stata_fmt" => returned::FORMAT_HELPER,
        "stata_ereturn" => returned::ERETURN_HELPER,
        "stata_ttest_r" => returned::TTEST_R_HELPER,
        "stata_eb" => matrices::EB_HELPER,
        "stata_scalar" => scalars::SCALAR_HELPER,
        "stata_tables" => tables::TABLES_HELPER,
        "stata_import_float" => IMPORT_FLOAT_HELPER,
        "stata_levelsof" => LEVELSOF_HELPER,
        "stata_set_obs" => SET_OBS_HELPER,
        "stata_expand" => EXPAND_HELPER,
        "stata_matrix_list" => matrices::MATRIX_LIST_HELPER,
        "stata_rng" => random::RNG_HELPER,
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
    /// Scalars defined so far, number or text.
    scalars: BTreeMap<String, Kind>,
    /// The scalars the command being translated reads by their bare names.
    scalars_read: std::cell::RefCell<Vec<String>>,
    /// Value label sets defined so far: name -> (value, text).
    label_sets: BTreeMap<String, Vec<(f64, String)>>,
    /// The model last fitted, for the commands that read it afterwards.
    estimates: Option<model::Estimates>,
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
    /// And its time variable.
    panel_time: Option<String>,
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

/// The data and what the translator knows of it, as `preserve` left them.
#[derive(Default, Clone)]
struct Preserved {
    data: Option<String>,
    file: Option<String>,
    kinds: BTreeMap<String, Kind>,
    label_sets: BTreeMap<String, Vec<(f64, String)>>,
    panel: Option<String>,
    panel_time: Option<String>,
    stset: Option<st::StSet>,
}

/// Collects one command's R.
#[derive(Default)]
struct Out {
    code: Vec<String>,
    caveats: Vec<String>,
    packages: Vec<&'static str>,
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
fn file_name(written: &str, ext: &str) -> String {
    let f = written
        .trim()
        .trim_matches('"')
        .trim_matches('`')
        .trim_matches('\'');
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
        let r = expr_r::translate_with(text, &kinds, &ret, use_)?;
        for c in r.caveats {
            out.caveat(c);
        }
        Ok(r.code)
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
        let r = expr_r::translate_with(text, &kinds, &ret, Use::Value)?;
        for c in r.caveats {
            out.caveat(c);
        }
        Ok((r.code, r.kind))
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
    fn not_translated(&mut self, stata: &str, reason: &str) {
        // a line Stata stops at with an error leaves the data as it was
        if reason.contains("Stata stops here") || reason.contains("(Stata: ") {
            return;
        }
        if let Some(c) = crate::reader::read(stata.trim()).commands().first() {
            for v in new_vars(c) {
                self.lost_vars.insert(v, stata.trim().to_string());
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
    fn by(&self, c: &Command) -> Result<Option<(Option<String>, String)>, Untranslated> {
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
        let arrange = if sort {
            let all = format!("{group} {within}");
            Some(format!(
                "dplyr::arrange(dplyr::pick({}))",
                select(all.trim())?
            ))
        } else if within.is_empty() {
            None
        } else {
            Some(format!(
                "dplyr::arrange(dplyr::pick({}))",
                select(&format!("{group} {within}"))?
            ))
        };
        Ok(Some((
            arrange,
            format!("dplyr::group_by(dplyr::pick({group_sel}))"),
        )))
    }

    /// `data <- data |> step` with the `by` prefix's grouping around it.
    fn step(&self, c: &Command, step: String, out: &mut Out) -> Result<(), Untranslated> {
        let d = self.data()?;
        let mut steps = Vec::new();
        match self.by(c)? {
            Some((arrange, group)) => {
                steps.extend(arrange);
                steps.push(group);
                steps.push(step);
                steps.push("dplyr::ungroup()".into());
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
        if sort {
            out.package("dplyr");
            let all: Vec<String> = group.iter().chain(&within).map(|v| r_name(v)).collect();
            out.line(format!("{d} <- dplyr::arrange({d}, {})", all.join(", ")));
        }
        self.helpers.insert("stata_by_groups");
        let mut args = vec![d.clone(), vector(&group)];
        if !within.is_empty() {
            args.push(format!("within = {}", vector(&within)));
        }
        if sort {
            args.push("sorted = TRUE".into());
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
        if let Some(why) = self.uses_lost(c) {
            return Err(no(why));
        }
        if c.prefixes.iter().any(|p| matches!(p, Prefix::Frame(_))) {
            self.esample = false;
            self.changed_since_fit = true;
            return Err(no("frames (frame name:) are translated in a later step"));
        }
        if c.parts.weight.is_some() && !WEIGHTED.contains(&c.name.as_str()) {
            self.esample = false;
            self.changed_since_fit = true;
            return Err(no(format!("{} with weights is not translated yet", c.name)));
        }
        let mut out = Out::default();
        let by = c.prefixes.iter().any(|p| matches!(p, Prefix::By { .. }));
        self.esample_new = false;
        self.r_new = None;
        self.e_new = false;
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
        if done.is_ok() && MODELS.contains(&c.name.as_str()) && !self.e_new {
            self.e_ret =
                returned::Returned::Lost(format!("e() after {} is not kept in R yet", c.name));
        }
        // a line that is not translated still runs in Stata, so it counts too
        if done.is_err() || !LEAVE_DATA_ALONE.contains(&c.name.as_str()) {
            self.changed_since_fit = done.is_err() || !MODELS.contains(&c.name.as_str());
        }
        // e(sample) holds while the rows stay where the model found them
        let name = c.name.as_str();
        let bysort = c
            .prefixes
            .iter()
            .any(|p| matches!(p, Prefix::By { sort: true, .. }));
        self.esample = if MODELS.contains(&name) {
            done.is_ok() && self.esample_new && !by
        } else {
            // values may change (Stata's e(sample) is a mark on the rows, so
            // it stays); rows may not be sorted, added or dropped
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
            ) || (matches!(name, "drop" | "keep")
                && c.parts.if_exp.is_none()
                && c.parts.in_range.is_none());
            self.esample
                && done.is_ok()
                && (LEAVE_DATA_ALONE.contains(&name) || values_only)
                && !matches!(name, "sort" | "gsort")
                && !bysort
        };
        done?;
        if self.drew.get() {
            self.helpers.insert("stata_rng");
        }
        for v in new_vars(c) {
            self.lost_vars.remove(&v);
        }
        if MODELS.contains(&c.name.as_str()) {
            self.weighted_fit = c.parts.weight.is_some();
            self.fit_fweight = matches!(&c.parts.weight, Some(w) if w.kind == crate::lang::syntax::WeightKind::Frequency);
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
            // whether there was one
            let body = out.code.join("\n");
            out.code = vec![format!(
                "stata_rc <- tryCatch({{\n{body}\n0\n}}, error = function(e) NA_real_)"
            )];
        }
        out.done()
    }

    fn dispatch(&mut self, c: &Command, out: &mut Out) -> Result<(), Untranslated> {
        let opts = Opts(&c.options);
        match c.name.as_str() {
            "use" => self.use_(c, &opts, out),
            "save" | "saveold" => self.save(c, &opts, out),
            "import" => self.import(c, &opts, out),
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
            "list" => self.list(c, &opts, out),
            "describe" => self.describe(c, out),
            "set" => self.set(c, out),
            "clear" => {
                if let Some(d) = self.data.take() {
                    out.line(format!("rm({d})"));
                }
                self.data_lost = None;
                self.kinds.clear();
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
            "duplicates" => self.duplicates(c, &opts, out),
            "encode" => self.encode(c, &opts, out),
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
            "xtreg" => self.xtreg(c, &opts, out),
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
            "display" => self.display(c, out),
            "ereturn" | "return" => self.list_returned(c, out),
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
        self.file = Some(path);
        self.kinds.clear();
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
                // empty cells are missing, "NA" is text, as in Stata
                args.push("na = \"\"".into());
                args.push("show_col_types = FALSE".into());
                steps.push(format!("readr::read_delim({})", args.join(", ")));
                steps.push("as.data.frame()".into());
                // Stata stores a column with decimals as float, unless asdouble
                if !opts.has("asdouble", 8) {
                    self.helpers.insert("stata_float");
                    self.helpers.insert("stata_import_float");
                    steps.push("stata_import_float()".into());
                    out.caveat(
                        "numbers with decimals are stored as floats, as Stata's import delimited \
                         stores them (asdouble keeps them double)",
                    );
                }
                if opts.arg("varnames", 7).as_deref() == Some("nonames") {
                    steps.push("stats::setNames(paste0(\"v\", seq_along(.)))".into());
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
        let own = format!("{var}[");
        if rhs.contains(&own) || c.parts.if_exp.as_deref().is_some_and(|e| e.contains(&own)) {
            return Err(no(format!(
                "replace that reads {var}'s own earlier rows ({var}[...]) runs row by row in Stata \
                 (each row sees the rows above already replaced); not translated yet"
            )));
        }
        let (cond, value, kind) = self.drawing_value(c, rhs, out)?;
        let value = match cond {
            Some(cond) => format!("dplyr::if_else({cond}, {value}, {})", r_name(var)),
            None => value,
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
        out.line(format!(
            "{d} <- {d} |> dplyr::arrange(dplyr::pick({}))",
            select(&c.parts.main)?
        ));
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
            if minus {
                // descending: missing values last unless mfirst
                if mfirst {
                    keys.push(format!("!is.na({n}) & {n} != \"\""));
                    keys.push(format!("dplyr::desc({n})"));
                } else {
                    keys.push(format!("dplyr::desc({n})"));
                }
            } else {
                keys.push(n);
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
                if !set.is_empty() && !add && !modify && !opts.has("replace", 7) {
                    return Err(no(format!("label define {name}: the label already exists (Stata wants add, modify or replace)")));
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
                        "{d} <- {d} |> dplyr::mutate(dplyr::across({sel}, \\(v) structure(haven::zap_labels(v), label = attr(v, \"label\"))))"
                    ));
                } else {
                    if !self.label_sets.contains_key(lbl) {
                        out.caveat(format!("the label {lbl} is not defined earlier in this do-file; it is expected from the data file"));
                    }
                    out.line(format!(
                        "{d} <- {d} |> dplyr::mutate(dplyr::across({sel}, \\(v) haven::labelled(as.numeric(v), labels = {}, label = attr(v, \"label\"))))",
                        label_object(lbl)
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
        out.line(format!(
            "stata_r <- stata_levelsof({rows}[[{}]], clean = {}, sep = {}, missing = {})",
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
                "print(data.frame(variable = names({d}), type = vapply({d}, \\(v) class(v)[1], \"\"), label = vapply({d}, \\(v) {{ l <- attr(v, \"label\"); if (is.null(l)) \"\" else l }}, \"\")), row.names = FALSE)"
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
            label_sets: self.label_sets.clone(),
            panel: self.panel.clone(),
            panel_time: self.panel_time.clone(),
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
        self.label_sets = p.label_sets;
        self.panel = p.panel;
        self.panel_time = p.panel_time;
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
                    }
                }
                Ok(())
            }
            "more" | "linesize" | "varabbrev" | "trace" | "rmsg" | "type" | "maxvar"
            | "matsize" => {
                out.caveat(format!(
                    "nothing to do in R: set {what} only changes how Stata runs or prints"
                ));
                Ok(())
            }
            other => Err(no(format!("set {other} is not translated yet"))),
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reader::read;

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
            "data <- readr::read_delim(\"data.csv\", na = \"\", show_col_types = FALSE)"
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
        assert_eq!(out[1], "s <- s |> dplyr::mutate(y = dplyr::if_else((!is.na(g) & g == 1), stata_float((x * 2)), NA_real_))");
        assert_eq!(out[2], "s <- s |> dplyr::mutate(z = 0.1)");
        // missing x counts as larger than 1; str5 keeps 5 characters
        assert_eq!(out[3], "s <- s |> dplyr::mutate(t = dplyr::if_else((is.na(x) | x > 1), substr(paste0(\"a\", s), 1, 5), \"\"))");
        assert_eq!(
            out[4],
            "s <- s |> dplyr::mutate(y = dplyr::if_else(is.na(y), 0, y))"
        );
        assert_eq!(out[5], "s <- s |> dplyr::arrange(dplyr::pick(g)) |> dplyr::group_by(dplyr::pick(g)) |> dplyr::mutate(n = stata_float(dplyr::row_number())) |> dplyr::ungroup()");
        assert!(out[6].contains("runs row by row"), "{}", out[6]);
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
        assert_eq!(out[6], "s <- s |> dplyr::arrange(dplyr::pick(c(g, x)))");
        assert_eq!(out[7], "s <- s |> dplyr::arrange(dplyr::desc(x), g)");
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
        assert!(r("label define a 1 \"x\"\nlabel define a 2 \"y\"")[1].starts_with("NOT:"));
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
        let out = r("set obs 2\ngenerate a = rbeta(1, 2)\ngenerate b = runiform()\nset seed 3\ngenerate c = runiform()");
        assert!(out[2].contains("not Stata's: a line before it that draws was not translated (generate a = rbeta(1, 2))"), "{out:?}");
        assert!(!out[4].starts_with("NOT"), "{out:?}");
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
        assert!(out[2].contains("stata_rc"), "{out:?}");
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
        let out = r("use a\nkeep if rbeta(1, 1) < .5\nsummarize x\nlabel variable x \"X\"");
        assert!(
            out[2].starts_with("NOT: the data in memory is not known here"),
            "{out:?}"
        );
        // .a to .z are all NA in R: how they compare is not translated
        let out = r("display . < .a
display . == .");
        assert!(
            out[0].starts_with("NOT: a comparison with an extended missing value"),
            "{out:?}"
        );
        assert!(!out[1].starts_with("NOT"), "{out:?}");
        // a variable whose line was not translated: the lines that use it say so
        let out = r(concat!(
            "use a\ngenerate double y = rbeta(1, 2)\ngenerate z = y * 2\nregress z x\n",
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
