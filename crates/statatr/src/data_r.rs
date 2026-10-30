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
mod margins;
mod ml;
mod model;
mod more;
mod post;
mod stats;

use std::collections::{BTreeMap, BTreeSet};

use crate::expr_r::{self, r_name, r_number, r_text, Kind, Untranslated, Use};
use crate::lang::syntax::Opt;
use crate::reader::{Command, Item, Opener, Prefix, Program, Span};

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
            out.push_str(&format!("# {}\n", l.stata.replace('\n', " ")));
            match &l.result {
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

/// Translates every command of `program`, in order. Blocks (`foreach`,
/// `forvalues`, `if`, `program`, ...) are not translated yet (Phase 2C).
pub fn translate(program: &Program) -> Translation {
    let mut s = Session::default();
    let mut lines = Vec::new();
    for item in &program.items {
        match item {
            Item::Command(c) => lines.push(Line {
                span: c.span,
                stata: c.text.clone(),
                result: s.command(c),
            }),
            Item::Group(g) => lines.push(Line {
                span: g.span,
                stata: opener_text(&g.opener),
                result: Err(no(
                    "loops, if blocks and programs are translated in a later step (Phase 2C)",
                )),
            }),
            Item::Problem(p) => lines.push(Line {
                span: p.span,
                stata: String::new(),
                result: Err(no(format!(
                    "the do-file reader could not read this line: {}",
                    p.message
                ))),
            }),
        }
    }
    Translation {
        helpers: s
            .helpers
            .iter()
            .map(|h| helper_code(h).to_string())
            .collect(),
        lines,
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

/// The commands that fit a model.
const MODELS: &[&str] = &[
    "regress", "logit", "probit", "poisson", "nbreg", "ologit", "oprobit", "mlogit", "anova",
];

/// Commands that leave the values of existing variables and the set of rows
/// as they were (a new variable, a new order or a label does not matter to
/// `margins`). Anything else may have changed the data.
const LEAVE_DATA_ALONE: &[&str] = &[
    "summarize",
    "tabulate",
    "correlate",
    "ttest",
    "list",
    "describe",
    "count",
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
        "stata_coefs" => model::COEFS_HELPER,
        "stata_ib" => model::IB_HELPER,
        "stata_glm_vcov" => ml::GLM_VCOV_HELPER,
        "stata_nbreg_vcov" => ml::NBREG_VCOV_HELPER,
        "stata_ml_header" => ml::ML_HEADER_HELPER,
        "stata_multinom_b" => ml::MULTINOM_B_HELPER,
        "stata_probit_newton" => ml::PROBIT_NEWTON_HELPER,
        "stata_outcome_probs" => post::OUTCOME_PROBS_HELPER,
        "stata_test" => post::TEST_HELPER,
        "stata_lincom" => post::LINCOM_HELPER,
        "stata_estat" => post::ESTAT_HELPER,
        "stata_margins" => margins::MARGINS_HELPER,
        "stata_anova" => anova::ANOVA_HELPER,
        _ => FLOAT_HELPER,
    }
}

/// What the translator knows while going down the do-file.
#[derive(Default)]
struct Session {
    /// The R name of the data frame "in memory".
    data: Option<String>,
    /// The file it came from (for `save` without a name).
    file: Option<String>,
    /// Variable types made by the do-file itself.
    kinds: BTreeMap<String, Kind>,
    /// Value label sets defined so far: name -> (value, text).
    label_sets: BTreeMap<String, Vec<(f64, String)>>,
    /// The model last fitted, for the commands that read it afterwards.
    estimates: Option<model::Estimates>,
    /// A command since that model may have changed the data, so the copy of
    /// its estimation sample kept in R is no longer what Stata would use.
    changed_since_fit: bool,
    helpers: BTreeSet<&'static str>,
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
    let f = f.trim();
    let has_ext = std::path::Path::new(f)
        .extension()
        .is_some_and(|e| !e.is_empty());
    if has_ext || ext.is_empty() {
        f.to_string()
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
    if !n.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) {
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
        let kinds = |n: &str| self.kind_of(n);
        let r = expr_r::translate(text, &kinds, use_)?;
        for c in r.caveats {
            out.caveat(c);
        }
        Ok(r.code)
    }

    fn value(&self, text: &str, out: &mut Out) -> Result<(String, Kind), Untranslated> {
        let kinds = |n: &str| self.kind_of(n);
        let r = expr_r::translate(text, &kinds, Use::Value)?;
        for c in r.caveats {
            out.caveat(c);
        }
        Ok((r.code, r.kind))
    }

    fn data(&self) -> Result<String, Untranslated> {
        self.data
            .clone()
            .ok_or_else(|| no("no data is in memory at this point (no use or import before it)"))
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

    fn command(&mut self, c: &Command) -> Result<RCode, Untranslated> {
        if c.prefixes.iter().any(|p| matches!(p, Prefix::Frame(_))) {
            return Err(no("frames (frame name:) are translated in a later step"));
        }
        if c.parts.weight.is_some() && !matches!(c.name.as_str(), "collapse") {
            return Err(no(format!("{} with weights is not translated yet", c.name)));
        }
        let mut out = Out::default();
        let done = self.dispatch(c, &mut out);
        // a line that is not translated still runs in Stata, so it counts too
        if done.is_err() || !LEAVE_DATA_ALONE.contains(&c.name.as_str()) {
            self.changed_since_fit = done.is_err() || !MODELS.contains(&c.name.as_str());
        }
        done?;
        if c.prefixes.contains(&Prefix::Capture) {
            // capture: an error is ignored and the do-file goes on
            let body = out.code.join("\n");
            out.code = vec![format!("try({{\n{body}\n}}, silent = TRUE)")];
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
            "list" => self.list(c, &opts, out),
            "describe" => self.describe(c, out),
            "set" => self.set(c, out),
            "clear" => {
                if let Some(d) = self.data.take() {
                    out.line(format!("rm({d})"));
                }
                self.kinds.clear();
                Ok(())
            }
            "version" => {
                out.caveat("nothing to do in R: version sets how Stata reads the do-file");
                Ok(())
            }
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
            other => Err(no(format!("{other} is not translated yet"))),
        }
    }

    fn quiet(c: &Command) -> bool {
        c.prefixes.contains(&Prefix::Quietly)
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
        let (value, kind) = self.value(rhs.trim(), out)?;
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
        let value = match self.condition(c, out)? {
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
        let (value, kind) = self.value(rhs.trim(), out)?;
        let value = match self.condition(c, out)? {
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
            let cond = self.condition(c, out)?.unwrap_or_default();
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
        if Self::quiet(c) {
            return Ok(());
        }
        if c.prefixes.iter().any(|p| matches!(p, Prefix::By { .. })) {
            return Err(no("by with count is not translated yet"));
        }
        let n = match self.condition(c, out)? {
            Some(cond) => format!("nrow(dplyr::filter({d}, {cond}))"),
            None => format!("nrow({d})"),
        };
        out.package("dplyr");
        out.line(format!("cat(format({n}, big.mark = \",\"), \"\\n\")"));
        Ok(())
    }

    fn list(&mut self, c: &Command, opts: &Opts, out: &mut Out) -> Result<(), Untranslated> {
        let d = self.data()?;
        if Self::quiet(c) {
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
        if Self::quiet(c) {
            return Ok(());
        }
        if !c.parts.main.trim().is_empty() || !c.options.is_empty() || c.using.is_some() {
            return Err(no(
                "describe of chosen variables, a file or with options is not translated yet",
            ));
        }
        out.line(format!(
            "print(data.frame(variable = names({d}), type = vapply({d}, \\(v) class(v)[1], \"\"), label = vapply({d}, \\(v) {{ l <- attr(v, \"label\"); if (is.null(l)) \"\" else l }}, \"\")), row.names = FALSE)"
        ));
        out.line(format!(
            "cat(\"observations:\", nrow({d}), \" variables:\", ncol({d}), \"\\n\")"
        ));
        out.caveat("printed in R's layout, not Stata's");
        Ok(())
    }

    fn set(&mut self, c: &Command, out: &mut Out) -> Result<(), Untranslated> {
        let (what, rest) = first_word(&c.parts.main);
        match what {
            "seed" => {
                let n: i64 = rest
                    .trim()
                    .parse()
                    .map_err(|_| no(format!("set seed {rest}: only a number is translated")))?;
                out.line(format!("set.seed({n})"));
                out.caveat(
                    "R's random numbers are not Stata's: the same seed does not give the same draws",
                );
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
            ("use s\nxtreg y x", "xtreg is not translated yet"),
            ("use s\nforeach v of varlist x y {\nsum `v'\n}", "Phase 2C"),
            ("use s\nsummarize x [aw = w]", "with weights"),
            ("use s\nset seed abc", "only a number"),
            ("use s\ngen x", "newvar = exp"),
        ] {
            let out = r(src);
            assert!(out.iter().any(|l| l.contains(why)), "{src}: {out:?}");
        }
        let t = translate(&read("use s\nset seed 42\nset more off"));
        let script = t.script();
        assert!(script.contains("set.seed(42)"), "{script}");
        assert!(
            script.contains("#   note: R's random numbers are not Stata's"),
            "{script}"
        );
    }
}
