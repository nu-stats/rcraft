//! Phase 2C: an existing do-file, unedited, run on R.
//!
//! [`Run`] goes down a do-file the way Stata does: it replaces each line's
//! macros, sets `local` and `global` macros, runs `foreach`, `forvalues`,
//! `while` and `if` blocks, follows `do`, `run` and `include` into other
//! files and keeps track of `cd`; every command it reaches goes to the
//! translator ([`Translator`]), in order, as the do-file would run it. What
//! it cannot do is refused with the reason, as everywhere in this crate.
//!
//! Some values are known only when the do-file runs: `local m = r(mean)`,
//! `if _N > 0 {`, `local s : display %9.2f r(sd)`. There [`Run::next_event`]
//! stops with [`Event::Ask`], the R that prints the value; the caller runs
//! it after the lines before it have run and gives back what it printed
//! ([`Run::give`]). A dry run ([`dry_run`]) gives no values, so what depends
//! on one is reported as known only when the do-file runs.
//!
//! The rules (how Stata keeps a number in a macro, how `forvalues` steps,
//! what `: list` does) were observed in Stata 19 with made-up values; see
//! `format`, `macros` and `plan/STATUS.md`.

mod eval;
mod format;
mod macros;
mod scan;
mod varlist;

use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use crate::data_r::{Line, RCode, Translation, Translator, ValueCode};
use crate::reader::{self, Command, ForeachList, Group, Item, Opener, Prefix, Span};

pub use format::g as stata_g;
pub use macros::{expand, words, Env, Need, Stop};
pub use scan::{
    command_word as command_name, quarto, r_script, report, scan, status, CommandUse, FileRole,
    FileStatus, Ran, Scan, Status,
};

/// The do-files a do-file runs (`do`, `run`, `include`), read from where
/// they are. Nothing else is read: data files are R's to read.
pub trait Files {
    /// A do-file's text, or `None` if it is not there.
    fn read(&self, path: &str) -> Option<String>;
    /// Whether a file or folder is there.
    fn exists(&self, path: &str) -> bool;
}

/// No files: `do`, `run` and `include` say the file is not there.
pub struct NoFiles;

impl Files for NoFiles {
    fn read(&self, _: &str) -> Option<String> {
        None
    }
    fn exists(&self, _: &str) -> bool {
        false
    }
}

/// Where the do-file runs.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// The do-file's name as shown (`analysis.do`).
    pub name: String,
    /// The folder it starts in: its own, which is Stata's working folder when
    /// it is run from there. Forward slashes; empty when not known.
    pub folder: String,
    /// Folders the do-file names that are somewhere else on this machine:
    /// in every line, each `from` (case and `\` or `/` aside) becomes its `to`.
    pub map: Vec<(String, String)>,
    /// A folder for `tempfile` files; without one `tempfile` is refused.
    pub temp_folder: String,
}

/// What a line of the run is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum What {
    /// A command, given to the translator.
    Command,
    /// A macro set: nothing for R to do.
    Macro,
    /// A loop, an `if` or a block, another file run, the end of the do-file:
    /// nothing for R to do.
    Control,
    /// Not run: it needs a value known only when the do-file runs (in a dry
    /// run), or one R could not work out.
    Later,
}

/// Where a line of the run comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Origin {
    /// Which file: 0 is the do-file, then the ones it runs, in [`DoFile::files`].
    pub file: usize,
    pub span: Span,
    /// The line as written, before its macros were replaced.
    pub written: String,
    /// The loops it is in, outermost first (`v = price`).
    pub loops: Vec<String>,
    pub what: What,
    /// For a macro or a block, what happened (`controls is "mpg weight"`).
    pub info: Option<String>,
}

/// A file the do-file names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileRef {
    /// As the do-file wrote it (macros replaced).
    pub written: String,
    /// Where that is, from the folder the do-file was in then.
    pub path: String,
    pub role: FileRole,
    pub status: FileStatus,
    /// The line of the run that names it.
    pub line: usize,
}

/// A do-file run through, with what each line became.
#[derive(Debug, Clone, Default)]
pub struct DoFile {
    pub translation: Translation,
    /// One for each line of `translation`.
    pub origins: Vec<Origin>,
    /// The do-file, then each file it ran.
    pub files: Vec<String>,
    pub refs: Vec<FileRef>,
}

/// What [`Run::next_event`] has for the caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// One or more lines were added to [`Run::lines`] (a block that is not run
    /// adds its first line and each command in it).
    Line,
    /// Run this R, after the lines before it, and give what it prints to
    /// [`Run::give`].
    Ask(ValueCode),
    /// The do-file is done.
    Done,
}

/// A dry run's answer to every question for R.
pub const KNOWN_WHEN_RUN: &str = "known only when the do-file runs";

/// Most lines a run makes (a loop that goes on and on is stopped).
pub const MAX_LINES: usize = 200_000;
/// Most times a loop goes round.
pub const MAX_ROUNDS: usize = 100_000;

/// The commands whose main part is a varlist (with factor variables for the
/// models): there a pattern, a range or `_all` is made the data's names.
const VARLIST: &[&str] = &[
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
    "xtreg",
    "ivregress",
    "stcox",
    "clogit",
    "robreg",
    "keep",
    "drop",
    "order",
    "sort",
    "list",
    "describe",
    "destring",
    "tostring",
    "histogram",
    "kdensity",
    "scatter",
    "line",
    "levelsof",
];

/// Commands that leave the data's variable names as they were.
const NAMES_STAY: &[&str] = &[
    "display",
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
    "xtreg",
    "ivregress",
    "anova",
    "stcox",
    "clogit",
    "robreg",
    "test",
    "lincom",
    "margins",
    "estat",
    "list",
    "describe",
    "count",
    "histogram",
    "kdensity",
    "scatter",
    "line",
    "twoway",
    "graph",
    "lowess",
    "set",
    "version",
    "log",
    "cmdlog",
    "pwd",
    "cd",
    "matrix",
    "scalar",
    "ereturn",
    "return",
    "ssc",
    "levelsof",
    "xtset",
    "label",
    "sort",
    "gsort",
    "browse",
    "pause",
    "more",
];

/// Deepest nesting of files run from files.
const MAX_FILES_DEEP: usize = 32;

#[derive(Debug, Clone)]
enum Macro {
    Text(String),
    /// Not known: why, in words.
    Later(String),
    /// Not known even when the do-file runs: a line that sets it was not
    /// translated. Why, in words.
    Lost(String),
}

#[derive(Debug, Clone)]
enum Kind {
    /// The do-file, or one run with `do`/`run`: its own locals.
    File {
        file: usize,
        locals: BTreeMap<String, Macro>,
    },
    /// A file run with `include`: the locals of the one that includes it.
    Include {
        file: usize,
    },
    Body,
    Foreach {
        name: String,
        values: Vec<String>,
        next: usize,
    },
    While {
        condition: String,
        rounds: usize,
    },
}

#[derive(Debug, Clone)]
struct Frame {
    items: Rc<Vec<Item>>,
    at: usize,
    kind: Kind,
    /// `capture {`, `quietly foreach ...`: prefixes for every command inside.
    prefixes: Vec<Prefix>,
}

/// A do-file being run, a line at a time.
pub struct Run<'f> {
    files: &'f dyn Files,
    opts: Options,
    names: Vec<String>,
    frames: Vec<Frame>,
    globals: BTreeMap<String, Macro>,
    t: Translator,
    lines: Vec<Line>,
    origins: Vec<Origin>,
    refs: Vec<FileRef>,
    programs: BTreeSet<String>,
    /// A line before that was not translated runs a program or an add-on,
    /// which may set any global; the globals the do-file has not set are not
    /// known after it.
    globals_unknown: Option<String>,
    /// What the last one-line `if exp command` came to, for an `else` after it.
    one_line_if: Option<bool>,
    temp: usize,
    cwd: String,
    written: BTreeSet<String>,
    /// Answers for the item being done, in the order it asks.
    answers: Vec<Result<String, String>>,
    asked: usize,
    /// Locals set while a line's macros are replaced (`` `++i' ``).
    pending: Vec<(String, String)>,
    waiting: Option<ValueCode>,
    done: bool,
    /// The data's variable names, as R last gave them, while they hold.
    vars: Option<Vec<String>>,
    /// A local the line just added sets from what it left (`levelsof, local()`).
    then: Option<(String, Need)>,
}

/// What doing an item came to.
enum Flow {
    /// Done; nothing was added.
    Next,
    /// Done; a line was added.
    Line,
    /// R must work out this first.
    Ask(Need),
}

impl<'f> Run<'f> {
    pub fn new(text: &str, files: &'f dyn Files, opts: Options) -> Run<'f> {
        let program = reader::read(text);
        let cwd = opts.folder.clone();
        Run {
            files,
            names: vec![opts.name.clone()],
            opts,
            frames: vec![Frame {
                items: Rc::new(program.items),
                at: 0,
                kind: Kind::File {
                    file: 0,
                    locals: BTreeMap::new(),
                },
                prefixes: Vec::new(),
            }],
            globals: BTreeMap::new(),
            t: Translator::new(),
            lines: Vec::new(),
            origins: Vec::new(),
            refs: Vec::new(),
            programs: BTreeSet::new(),
            globals_unknown: None,
            one_line_if: None,
            temp: 0,
            cwd,
            written: BTreeSet::new(),
            answers: Vec::new(),
            asked: 0,
            pending: Vec::new(),
            waiting: None,
            done: false,
            vars: None,
            then: None,
        }
    }

    /// The lines so far, with their origins.
    pub fn lines(&self) -> &[Line] {
        &self.lines
    }

    pub fn origins(&self) -> &[Origin] {
        &self.origins
    }

    /// The helpers the lines so far call: name and R.
    pub fn helpers(&self) -> Vec<(&'static str, &'static str)> {
        self.t.helpers()
    }

    /// The next line, a question for R, or the end.
    pub fn next_event(&mut self) -> Event {
        loop {
            if let Some(w) = &self.waiting {
                return Event::Ask(w.clone());
            }
            if self.done {
                return Event::Done;
            }
            if self.lines.len() >= MAX_LINES {
                self.done = true;
                let at = self.span_here();
                let line = self.t.refused(
                    at,
                    String::new(),
                    format!("RCraft stops here: the do-file has run {MAX_LINES} lines, which looks like a loop that does not end"),
                );
                self.push(line, What::Command, None, String::new());
                return Event::Line;
            }
            match self.advance() {
                Flow::Next => {}
                Flow::Line => return Event::Line,
                Flow::Ask(need) => {
                    let code = match &need {
                        Need::Value(exp) => self.t.value_code(exp),
                        Need::Display(rest) => self.t.display_code(rest),
                        Need::Names => match self.t.data() {
                            Some(d) => Ok(ValueCode {
                                code: format!("cat(names({d}), sep = \"\n\")"),
                                text: true,
                            }),
                            None => Err(crate::expr_r::Untranslated {
                                reason: "there is no data in memory".into(),
                            }),
                        },
                        Need::R(code, _) => Ok(ValueCode {
                            code: code.clone(),
                            text: true,
                        }),
                    };
                    match code {
                        Ok(code) => {
                            self.waiting = Some(code.clone());
                            return Event::Ask(code);
                        }
                        // R cannot be asked: the item is done again without it
                        Err(e) => self.answers.push(Err(format!(
                            "{} is not translated: {}",
                            need.what().trim(),
                            e.reason
                        ))),
                    }
                }
            }
        }
    }

    /// What R printed for the last [`Event::Ask`], or why it could not.
    pub fn give(&mut self, printed: Result<String, String>) {
        let text = self.waiting.take().is_some_and(|w| w.text);
        self.answers.push(printed.map(|p| {
            if text {
                p.strip_suffix('\n').unwrap_or(&p).to_string()
            } else {
                p.trim().to_string()
            }
        }));
    }

    /// The run so far, without ending it.
    pub fn snapshot(&self) -> DoFile {
        DoFile {
            translation: self.t.clone().finish(self.lines.clone()),
            origins: self.origins.clone(),
            files: self.names.clone(),
            refs: self.refs.clone(),
        }
    }

    /// The run as it stands, ended.
    pub fn finish(self) -> DoFile {
        DoFile {
            translation: self.t.finish(self.lines),
            origins: self.origins,
            files: self.names,
            refs: self.refs,
        }
    }

    // ---------------- going down the do-file ----------------

    fn advance(&mut self) -> Flow {
        let depth = self.frames.len();
        // a local the line before set from what it left in R, now it has run
        if let Some((name, need)) = self.then.clone() {
            self.asked = 0;
            match self.ask(need) {
                Ok(v) => self.set_local_now(&name, Macro::Text(v)),
                Err(Stop::Ask(n)) => return Flow::Ask(n),
                Err(e) => self.set_local_now(
                    &name,
                    Macro::Later(format!("is set from what a line left, {}", stop_reason(&e))),
                ),
            }
            self.then = None;
            self.answers.clear();
            return Flow::Next;
        }
        let Some(top) = self.frames.last() else {
            self.done = true;
            return Flow::Next;
        };
        if top.at >= top.items.len() {
            return self.frame_end();
        }
        let item = top.items.get(top.at).cloned();
        self.asked = 0;
        self.pending.clear();
        let flow = match &item {
            Some(Item::Command(c)) => self.command(c, depth - 1),
            Some(Item::Group(g)) => self.group(g, depth - 1),
            Some(Item::Problem(p)) => {
                self.done_item(depth - 1);
                let line = self.t.refused(
                    p.span,
                    String::new(),
                    format!("the do-file reader could not read this line: {}", p.message),
                );
                self.push(line, What::Command, None, String::new());
                Flow::Line
            }
            None => Flow::Next,
        };
        if !matches!(flow, Flow::Ask { .. }) {
            self.answers.clear();
        }
        flow
    }

    /// The item at `depth` is done: the next one is due, and the locals its
    /// macros set are kept.
    fn done_item(&mut self, depth: usize) {
        if let Some(f) = self.frames.get_mut(depth) {
            f.at += 1;
        }
        for (name, value) in std::mem::take(&mut self.pending) {
            self.set_local_now(&name, Macro::Text(value));
        }
    }

    fn frame_end(&mut self) -> Flow {
        let Some(kind) = self.frames.last().map(|f| f.kind.clone()) else {
            return Flow::Next;
        };
        match kind {
            Kind::Foreach { name, values, next } => {
                if let Some(v) = values.get(next).cloned() {
                    if let Some(top) = self.frames.last_mut() {
                        top.at = 0;
                        top.kind = Kind::Foreach {
                            name: name.clone(),
                            values,
                            next: next + 1,
                        };
                    }
                    self.set_local_now(&name, Macro::Text(v));
                } else {
                    // Stata 19: the loop's macro is gone after the loop
                    self.frames.pop();
                    self.remove_local(&name);
                }
                Flow::Next
            }
            Kind::While { condition, rounds } => {
                self.asked = 0;
                match self.condition(&condition) {
                    Ok(Some(true)) if rounds < MAX_ROUNDS => {
                        if let Some(top) = self.frames.last_mut() {
                            top.at = 0;
                            top.kind = Kind::While {
                                condition,
                                rounds: rounds + 1,
                            };
                        }
                        self.answers.clear();
                        Flow::Next
                    }
                    Err(Stop::Ask(need)) => Flow::Ask(need),
                    other => {
                        self.frames.pop();
                        self.answers.clear();
                        let why = match other {
                            Ok(Some(true)) => Some(format!(
                                "RCraft stops this while loop after {MAX_ROUNDS} rounds"
                            )),
                            Ok(_) => None,
                            Err(e) => Some(stop_reason(&e)),
                        };
                        if let Some(why) = why {
                            let at = self.span_here();
                            let line =
                                self.t
                                    .refused(at, format!("while {condition}"), why.clone());
                            self.push(line, What::Later, None, format!("while {condition}"));
                            return Flow::Line;
                        }
                        Flow::Next
                    }
                }
            }
            _ => {
                self.frames.pop();
                if self.frames.is_empty() {
                    self.done = true;
                }
                Flow::Next
            }
        }
    }

    fn command(&mut self, c: &Command, depth: usize) -> Flow {
        let written = c.text.clone();
        let expanded = match macros::expand(&written, self) {
            Ok(t) => self.map_paths(&t),
            Err(Stop::Ask(need)) => return Flow::Ask(need),
            Err(e) => {
                self.done_item(depth);
                let what = if matches!(e, Stop::Later(_)) {
                    What::Later
                } else {
                    What::Command
                };
                // a macro this line sets does not keep the value it had before
                if matches!(c.name.as_str(), "local" | "global") {
                    let rest = after_word(&c.text, &c.written);
                    let name = rest.split_whitespace().next().unwrap_or("");
                    let name = name.split(['=', ':']).next().unwrap_or("");
                    if macros::is_name(name) {
                        let why = format!("was not set: {}", stop_reason(&e));
                        let m = match &e {
                            Stop::Later(_) => Macro::Later(why),
                            _ => Macro::Lost(why),
                        };
                        self.set_macro(name, c.name == "global", m);
                    }
                }
                let line = self.t.refused(c.span, written.clone(), stop_reason(&e));
                self.push(line, what, None, written);
                return Flow::Line;
            }
        };
        if expanded.trim().is_empty() {
            self.done_item(depth);
            return Flow::Next;
        }
        // a one-line `else command` reads as a line only after its if
        let program = if c.name == "else" {
            let mut c2 = c.clone();
            c2.text = expanded.clone();
            reader::Program {
                items: vec![Item::Command(c2)],
            }
        } else {
            reader::read(&expanded)
        };
        let mut c2 = match program.items.as_slice() {
            [Item::Command(c2)] => c2.clone(),
            [Item::Problem(p)] => {
                self.done_item(depth);
                let line = self.t.refused(
                    c.span,
                    expanded,
                    format!("the do-file reader could not read this line: {}", p.message),
                );
                self.push(line, What::Command, None, written);
                return Flow::Line;
            }
            _ => {
                self.done_item(depth);
                let line = self.t.refused(
                    c.span,
                    expanded,
                    "with its macros replaced this is not one command, which is not translated",
                );
                self.push(line, What::Command, None, written);
                return Flow::Line;
            }
        };
        c2.span = c.span;
        let mut prefixes = self
            .frames
            .get(depth)
            .map(|f| f.prefixes.clone())
            .unwrap_or_default();
        prefixes.append(&mut c2.prefixes);
        c2.prefixes = prefixes;
        self.run_command(&c2, depth, written)
    }

    fn run_command(&mut self, c: &Command, depth: usize, written: String) -> Flow {
        let rest = after_word(&c.text, &c.written);
        match c.name.as_str() {
            "local" | "global" => {
                let global = c.name == "global";
                match self.define(&rest, global) {
                    Err(Stop::Ask(need)) => Flow::Ask(need),
                    Ok(info) => {
                        self.done_item(depth);
                        self.push(nothing(c), What::Macro, Some(info), written);
                        Flow::Line
                    }
                    Err(e) => {
                        self.done_item(depth);
                        let what = if matches!(e, Stop::Later(_)) {
                            What::Later
                        } else {
                            What::Command
                        };
                        let line = self.t.refused(c.span, c.text.clone(), stop_reason(&e));
                        self.push(line, what, None, written);
                        Flow::Line
                    }
                }
            }
            "macro" => {
                self.done_item(depth);
                let ws = words(&rest);
                let line = match ws.first().map(String::as_str) {
                    Some("drop") => {
                        for n in ws.iter().skip(1) {
                            if n == "_all" {
                                self.globals.clear();
                            } else if let Some(local) = n.strip_prefix('_') {
                                self.remove_local(local);
                            } else {
                                self.globals.remove(n);
                            }
                        }
                        return self.line_with(
                            nothing(c),
                            What::Macro,
                            Some("dropped".into()),
                            written,
                        );
                    }
                    Some("shift") => {
                        self.shift_locals();
                        return self.line_with(
                            nothing(c),
                            What::Macro,
                            Some("shifted".into()),
                            written,
                        );
                    }
                    _ => self.t.refused(
                        c.span,
                        c.text.clone(),
                        "macro list and macro dir are not translated yet",
                    ),
                };
                self.push(line, What::Command, None, written);
                Flow::Line
            }
            // the first token of a macro, and what is left, as Stata 19 takes them
            "gettoken" => {
                self.done_item(depth);
                let g = match gettoken_parts(&rest) {
                    Ok(g) => g,
                    Err(why) => {
                        let names = rest.split_once(':').map(|(n, _)| n).unwrap_or("");
                        let lost = format!("{} was not translated", c.text.trim());
                        for (n, global) in qualified_names(names) {
                            self.set_macro(&n, global, Macro::Lost(lost.clone()));
                        }
                        let line = self.t.refused(c.span, c.text.clone(), why);
                        return self.line_with(line, What::Command, None, written);
                    }
                };
                let (src, src_global) = g.source.clone();
                let value = if src_global {
                    self.globals.get(&src).cloned()
                } else {
                    self.locals().and_then(|l| l.get(&src).cloned())
                };
                let text = match value {
                    None => String::new(),
                    Some(Macro::Text(t)) => t,
                    Some(Macro::Later(why) | Macro::Lost(why)) => {
                        let unknown = |why: &str| Macro::Lost(why.to_string());
                        for (n, global) in g.targets.iter().chain(g.matched.iter()) {
                            self.set_macro(n, *global, unknown(&why));
                        }
                        let line = self.t.refused(c.span, c.text.clone(), why);
                        return self.line_with(line, What::Command, None, written);
                    }
                };
                let (token, left, paren) = gettoken(&text, &g.parse, g.matched.is_some());
                let mut info = Vec::new();
                if let Some((n, global)) = g.targets.first() {
                    info.push(format!("`{n}' = {token}"));
                    self.set_macro(n, *global, Macro::Text(token));
                }
                if let Some((n, global)) = g.targets.get(1) {
                    info.push(format!("`{n}' = {left}"));
                    self.set_macro(n, *global, Macro::Text(left));
                }
                if let Some((n, global)) = &g.matched {
                    self.set_macro(n, *global, Macro::Text(paren));
                }
                self.line_with(nothing(c), What::Macro, Some(info.join(", ")), written)
            }
            // the positional locals `1', `2', ... from text, as Stata 19 sets
            // them: every numbered local is cleared first
            "tokenize" => {
                self.done_item(depth);
                let (text, parse) = match tokenize_parts(&rest) {
                    Ok(x) => x,
                    Err(why) => {
                        let line = self.t.refused(c.span, c.text.clone(), why);
                        return self.line_with(line, What::Command, None, written);
                    }
                };
                let tokens = tokenize(&text, &parse);
                if let Some(l) = self.locals_mut() {
                    l.retain(|k, _| !k.bytes().all(|b| b.is_ascii_digit()));
                }
                for (i, t) in tokens.iter().enumerate() {
                    self.set_local_now(&(i + 1).to_string(), Macro::Text(t.clone()));
                }
                let info = if tokens.is_empty() {
                    "no tokens: `1' and the rest are empty".to_string()
                } else {
                    tokens
                        .iter()
                        .enumerate()
                        .map(|(i, t)| format!("`{}' = {t}", i + 1))
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                self.line_with(nothing(c), What::Macro, Some(info), written)
            }
            "tempvar" | "tempname" | "tempfile" => {
                self.done_item(depth);
                let names = words(&rest);
                if c.name == "tempfile" && self.opts.temp_folder.is_empty() {
                    let lost = format!("{} was not translated", c.text.trim());
                    for n in &names {
                        self.set_local_now(n, Macro::Lost(lost.clone()));
                    }
                    let line = self.t.refused(
                        c.span,
                        c.text.clone(),
                        "tempfile needs a folder for the files, which this run was not given",
                    );
                    self.push(line, What::Command, None, written);
                    return Flow::Line;
                }
                let mut made = Vec::new();
                for n in &names {
                    // Stata 19: __000000, __000001, ... for both; tempfile a file of its own
                    let value = if c.name == "tempfile" {
                        format!("{}/ST_{:06}.tmp", self.opts.temp_folder, self.temp)
                    } else {
                        format!("__{:06}", self.temp)
                    };
                    self.temp += 1;
                    made.push(format!("{n} is {value}"));
                    self.set_local_now(n, Macro::Text(value));
                }
                let mut line = nothing(c);
                if c.name == "tempfile" {
                    line.result = Ok(RCode {
                        code: format!(
                            "dir.create({}, showWarnings = FALSE, recursive = TRUE)",
                            crate::expr_r::r_text(&self.opts.temp_folder)
                        ),
                        ..RCode::default()
                    });
                }
                self.push(line, What::Macro, Some(made.join(", ")), written);
                Flow::Line
            }
            "unab" => {
                let (name, list) = rest.split_once(':').unwrap_or(("", ""));
                let name = name.trim();
                let list = list.split(',').next().unwrap_or("").trim();
                if !macros::is_name(name) || list.is_empty() {
                    self.done_item(depth);
                    let line = self.t.refused(
                        c.span,
                        c.text.clone(),
                        "Stata stops here with an error: unab name : varlist",
                    );
                    self.push(line, What::Command, None, written);
                    return Flow::Line;
                }
                let vars = match self.vars_now() {
                    Err(Stop::Ask(need)) => return Flow::Ask(need),
                    Err(e) => Err(stop_reason(&e)),
                    Ok(None) => Err(format!("the data's variable names are {KNOWN_WHEN_RUN}")),
                    Ok(Some(v)) => varlist::expand(list, &v)
                        .map_err(|e| format!("Stata stops here with an error: {e}")),
                };
                self.done_item(depth);
                match vars {
                    Ok(v) => {
                        let info = show(name, &v);
                        self.set_local_now(name, Macro::Text(v));
                        self.push(nothing(c), What::Macro, Some(info), written);
                    }
                    Err(why) => {
                        self.set_local_now(name, Macro::Later(format!("is set by unab: {why}")));
                        let what = if why.contains(KNOWN_WHEN_RUN) {
                            What::Later
                        } else {
                            What::Command
                        };
                        let line = self.t.refused(c.span, c.text.clone(), why);
                        self.push(line, what, None, written);
                    }
                }
                Flow::Line
            }
            "do" | "run" | "include" => {
                self.done_item(depth);
                self.run_file(c, &rest, written)
            }
            // `if exp command` on one line: the expression runs until what
            // cannot continue it, and the command after it runs if it is true
            "if" => {
                self.one_line_if = None;
                let refuse = |s: &mut Self, why: &str| {
                    s.done_item(depth);
                    let line = s.t.refused(c.span, c.text.clone(), why);
                    s.line_with(line, What::Command, None, written.clone())
                };
                let Ok((_, end)) = crate::lang::expr::parse_prefix(&rest) else {
                    return refuse(
                        self,
                        "the condition after if cannot be read (Stata: invalid syntax)",
                    );
                };
                let (condition, then) = rest.split_at(end);
                if then.trim().is_empty() {
                    return refuse(
                        self,
                        "if needs a command or { after its condition (Stata: invalid syntax)",
                    );
                }
                let go = match self.condition(condition) {
                    Err(Stop::Ask(need)) => return Flow::Ask(need),
                    Err(e) => {
                        self.done_item(depth);
                        let what = if matches!(e, Stop::Later(_)) {
                            What::Later
                        } else {
                            What::Command
                        };
                        let line = self.t.refused(c.span, c.text.clone(), stop_reason(&e));
                        return self.line_with(line, what, None, written);
                    }
                    Ok(None) => {
                        return refuse(self, "the condition is text (Stata: type mismatch)")
                    }
                    Ok(Some(go)) => go,
                };
                self.one_line_if = Some(go);
                if !go {
                    self.done_item(depth);
                    self.push_control(
                        c.span,
                        c.text.clone(),
                        "false: the command after it is skipped".into(),
                        written,
                    );
                    return Flow::Line;
                }
                let program = reader::read(then.trim());
                let [Item::Command(inner)] = program.items.as_slice() else {
                    return refuse(
                        self,
                        "the command after if is not one command, which is not translated",
                    );
                };
                let mut inner = inner.clone();
                inner.span = c.span;
                let mut prefixes = c.prefixes.clone();
                prefixes.append(&mut inner.prefixes);
                inner.prefixes = prefixes;
                self.run_command(&inner, depth, written)
            }
            // `else command` after a one-line if: the command runs if that was false
            "else" => {
                let then = rest.trim();
                match self.one_line_if.take() {
                    Some(false) => {
                        let program = reader::read(then);
                        let [Item::Command(inner)] = program.items.as_slice() else {
                            self.done_item(depth);
                            let line = self.t.refused(c.span, c.text.clone(), "the command after else is not one command, which is not translated");
                            return self.line_with(line, What::Command, None, written);
                        };
                        let mut inner = inner.clone();
                        inner.span = c.span;
                        let mut prefixes = c.prefixes.clone();
                        prefixes.append(&mut inner.prefixes);
                        inner.prefixes = prefixes;
                        self.run_command(&inner, depth, written)
                    }
                    Some(true) => {
                        self.done_item(depth);
                        self.push_control(
                            c.span,
                            c.text.clone(),
                            "the if before it was true: skipped".into(),
                            written,
                        );
                        Flow::Line
                    }
                    None => {
                        self.done_item(depth);
                        let line = self.t.refused(
                            c.span,
                            c.text.clone(),
                            "else with no one-line if just before it, or one that was not run",
                        );
                        self.line_with(line, What::Command, None, written)
                    }
                }
            }
            "exit" => {
                self.done_item(depth);
                self.push(
                    nothing(c),
                    What::Control,
                    Some("the do-file ends here".into()),
                    written,
                );
                while let Some(f) = self.frames.pop() {
                    if matches!(f.kind, Kind::File { .. }) {
                        break;
                    }
                }
                if self.frames.is_empty() {
                    self.done = true;
                }
                Flow::Line
            }
            "continue" => {
                self.done_item(depth);
                let brk = rest.contains("break") || rest.contains("brea");
                let in_loop = self
                    .frames
                    .iter()
                    .rposition(|f| matches!(f.kind, Kind::Foreach { .. } | Kind::While { .. }));
                let Some(at) = in_loop else {
                    let line = self.t.refused(
                        c.span,
                        c.text.clone(),
                        "continue outside a loop (Stata: not in a loop)",
                    );
                    self.push(line, What::Command, None, written);
                    return Flow::Line;
                };
                self.frames.truncate(at + 1);
                let info = if brk {
                    if let Some(Frame {
                        kind: Kind::Foreach { name, .. },
                        ..
                    }) = self.frames.pop()
                    {
                        self.remove_local(&name);
                    }
                    "the loop ends here"
                } else {
                    if let Some(f) = self.frames.last_mut() {
                        f.at = f.items.len();
                    }
                    "on to the loop's next round"
                };
                self.push(nothing(c), What::Control, Some(info.into()), written);
                Flow::Line
            }
            "program" => {
                self.done_item(depth);
                let ws = words(&rest);
                if ws.first().is_some_and(|w| w == "drop") {
                    for n in ws.iter().skip(1) {
                        if n == "_all" || n == "_allado" {
                            self.programs.clear();
                        } else {
                            self.programs.remove(n);
                        }
                    }
                    return self.line_with(
                        nothing(c),
                        What::Control,
                        Some("programs dropped".into()),
                        written,
                    );
                }
                let line = self.t.refused(
                    c.span,
                    c.text.clone(),
                    "program dir and program list are not translated yet",
                );
                self.push(line, What::Command, None, written);
                Flow::Line
            }
            name if self.programs.contains(name) => {
                self.done_item(depth);
                let line = self.t.refused(
                    c.span,
                    c.text.clone(),
                    format!("this runs {name}, a program the do-file defines; programs are not translated yet"),
                );
                self.globals_unknown = Some(c.text.trim().to_string());
                self.push(line, What::Command, None, written);
                Flow::Line
            }
            _ => {
                // a pattern, range or _all in a varlist: the data's names
                let mut c = c.clone();
                let mut info = None;
                let main = c.parts.main.clone();
                if VARLIST.contains(&c.name.as_str()) && varlist::needs_names(&main) {
                    match self.vars_now() {
                        Err(Stop::Ask(need)) => return Flow::Ask(need),
                        Ok(Some(vars)) => match varlist::expand(&main, &vars) {
                            Ok(m) => {
                                if m != main {
                                    info = Some(format!("the variables: {m}"));
                                    c.parts.main = m;
                                }
                            }
                            Err(e) => {
                                self.done_item(depth);
                                let line = self.t.refused(
                                    c.span,
                                    c.text.clone(),
                                    format!("Stata stops here with an error: {e}"),
                                );
                                self.push(line, What::Command, None, written);
                                return Flow::Line;
                            }
                        },
                        // as written: the translator takes what it can
                        Ok(None) | Err(_) => {}
                    }
                }
                let c = &c;
                self.done_item(depth);
                if c.name == "clear" && matches!(rest.trim(), "all" | "*") {
                    self.programs.clear();
                }
                let line = self.t.command(c);
                if let Err(e) = &line.result {
                    // the latest one is named: it is the likeliest to have set it
                    if may_set_globals(&c.name, &e.reason) {
                        self.globals_unknown = Some(c.text.trim().to_string());
                    }
                }
                // a local() it would have set is not known after it
                if line.result.is_err() {
                    for o in c.options.iter().filter(|o| o.is("local", 3)) {
                        if let Some(n) = o
                            .arg
                            .as_deref()
                            .map(str::trim)
                            .filter(|n| macros::is_name(n))
                        {
                            let why = format!("{} was not translated", c.text.trim());
                            self.set_local_now(n, Macro::Lost(why));
                        }
                    }
                }
                // levelsof, local(name): the local is set from r(levels) once the line has run
                if c.name == "levelsof" && line.result.is_ok() {
                    let local = c.options.iter().find(|o| o.is("local", 3));
                    if let Some(name) = local
                        .and_then(|o| o.arg.as_deref())
                        .map(str::trim)
                        .filter(|n| macros::is_name(n))
                    {
                        self.then = Some((name.to_string(), Need::Value("r(levels)".into())));
                    }
                }
                let index = self.lines.len();
                self.file_refs(c, index);
                if c.name == "cd" && line.result.is_ok() {
                    let dir = clean_path(&c.parts.main);
                    self.cwd = join(&self.cwd, &dir);
                }
                self.push(line, What::Command, info, written);
                Flow::Line
            }
        }
    }

    /// The data's variable names, from R (asked again only after a line
    /// that may have changed them); `None` when there are no data, or no R
    /// to ask (a dry run).
    fn vars_now(&mut self) -> Result<Option<Vec<String>>, Stop> {
        if self.t.data().is_none() {
            return Ok(None);
        }
        if let Some(v) = &self.vars {
            return Ok(Some(v.clone()));
        }
        match self.ask(Need::Names) {
            Ok(text) => {
                let v: Vec<String> = text
                    .lines()
                    .map(|l| l.trim().to_string())
                    .filter(|l| !l.is_empty())
                    .collect();
                self.vars = Some(v.clone());
                Ok(Some(v))
            }
            Err(Stop::Later(_)) => Ok(None),
            Err(e) => Err(e),
        }
    }

    fn line_with(&mut self, line: Line, what: What, info: Option<String>, written: String) -> Flow {
        self.push(line, what, info, written);
        Flow::Line
    }

    /// `local`/`global` and what follows the word: the name, then text,
    /// `= exp` or `: function`. Returns what the macro holds now, in words.
    fn define(&mut self, rest: &str, global: bool) -> Result<String, Stop> {
        let rest = rest.trim_start();
        for (pre, step) in [("++", 1.0), ("--", -1.0)] {
            if let Some(name) = rest.strip_prefix(pre) {
                let name = name.trim();
                if !macros::is_name(name) || global {
                    return Err(Stop::Stata(format!("{pre}{name}: invalid name")));
                }
                let old = self.local(name)?;
                let x: f64 = if old.trim().is_empty() {
                    0.0
                } else {
                    old.trim()
                        .parse()
                        .map_err(|_| Stop::Stata(format!("`{name}' is not a number")))?
                };
                let value = format::g(x + step, 18);
                self.set_local_now(name, Macro::Text(value.clone()));
                return Ok(show(name, &value));
            }
        }
        let len = rest
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .count();
        let name = rest.get(..len).unwrap_or("");
        let after = rest.get(len..).unwrap_or("");
        if name.is_empty() {
            return Err(Stop::Stata("a macro name is needed".into()));
        }
        let trimmed = after.trim_start();
        let value = if let Some(exp) = trimmed.strip_prefix('=') {
            match eval::eval_text(exp) {
                Ok(v) => v.macro_text(),
                Err(eval::NotHere::Stata(m)) => return Err(Stop::Stata(m)),
                Err(eval::NotHere::Later) => match self.ask(Need::Value(exp.trim().to_string())) {
                    Ok(v) => v,
                    Err(Stop::Later(why)) => {
                        self.set_macro(
                            name,
                            global,
                            Macro::Later(format!("holds {}, {why}", exp.trim())),
                        );
                        return Err(Stop::Later(format!("{} is {why}", exp.trim())));
                    }
                    Err(Stop::Not(why)) => {
                        self.set_macro(name, global, Macro::Lost(format!("was not set: {why}")));
                        return Err(Stop::Not(why));
                    }
                    Err(e) => return Err(e),
                },
            }
        } else if let Some(f) = trimmed.strip_prefix(':') {
            match self.extended(f.trim()) {
                Ok(v) => v,
                Err(Stop::Later(why)) => {
                    self.set_macro(name, global, Macro::Later(why.clone()));
                    return Err(Stop::Later(why));
                }
                Err(Stop::Not(why)) => {
                    self.set_macro(name, global, Macro::Lost(format!("was not set: {why}")));
                    return Err(Stop::Not(why));
                }
                Err(e) => return Err(e),
            }
        } else {
            macros::definition_text(after)
        };
        self.set_macro(name, global, Macro::Text(value.clone()));
        Ok(show(name, &value))
    }

    fn set_macro(&mut self, name: &str, global: bool, m: Macro) {
        if global {
            self.globals.insert(name.to_string(), m);
        } else {
            self.set_local_now(name, m);
        }
    }

    /// A value from R for the item being done: the next answer given, or a
    /// question if it has not been asked yet.
    fn ask(&mut self, need: Need) -> Result<String, Stop> {
        if let Some(a) = self.answers.get(self.asked).cloned() {
            self.asked += 1;
            return a.map_err(Stop::Later);
        }
        Err(Stop::Ask(need))
    }

    fn group(&mut self, g: &Group, depth: usize) -> Flow {
        let parent = self
            .frames
            .get(depth)
            .map(|f| f.prefixes.clone())
            .unwrap_or_default();
        let mut prefixes = parent;
        prefixes.extend(g.prefixes.iter().cloned());
        match &g.opener {
            Opener::Foreach { name, list } => {
                let (header, text) = match list {
                    ForeachList::In(ws) => ("in".to_string(), ws.join(" ")),
                    ForeachList::Of { kind, list } => (format!("of {kind}"), list.clone()),
                };
                let written = format!("foreach {name} {header} {text} {{");
                let expanded = match macros::expand(&text, self) {
                    Ok(t) => t,
                    Err(Stop::Ask(need)) => return Flow::Ask(need),
                    Err(e) => return self.refuse_block(g, depth, written, &e),
                };
                let values = match list {
                    ForeachList::In(_) => Ok(words(&expanded)),
                    ForeachList::Of { kind, .. } => self.foreach_values(kind, &expanded),
                };
                let values = match values {
                    Ok(v) => v,
                    Err(Stop::Ask(need)) => return Flow::Ask(need),
                    Err(e) => return self.refuse_block(g, depth, written, &e),
                };
                let shown = format!("foreach {name} {header} {} {{", expanded.trim());
                let info = rounds_info(&values);
                self.enter_loop(g, depth, prefixes, name, values, shown, info, written)
            }
            Opener::Forvalues { name, range } => {
                let written = format!("forvalues {name} = {range} {{");
                let expanded = match macros::expand(range, self) {
                    Ok(t) => t,
                    Err(Stop::Ask(need)) => return Flow::Ask(need),
                    Err(e) => return self.refuse_block(g, depth, written, &e),
                };
                let values = match forvalues(&expanded) {
                    Ok(v) => v,
                    Err(why) => return self.refuse_block(g, depth, written, &Stop::Stata(why)),
                };
                let shown = format!("forvalues {name} = {} {{", expanded.trim());
                let info = rounds_info(&values);
                self.enter_loop(g, depth, prefixes, name, values, shown, info, written)
            }
            Opener::While { condition } => {
                let written = format!("while {condition} {{");
                match self.condition(condition) {
                    Err(Stop::Ask(need)) => Flow::Ask(need),
                    Err(e) => self.refuse_block(g, depth, written, &e),
                    Ok(None) => self.refuse_block(
                        g,
                        depth,
                        written,
                        &Stop::Stata("the condition is text (Stata: type mismatch)".into()),
                    ),
                    Ok(Some(go)) => {
                        self.done_item(depth);
                        let info = if go {
                            "true: the loop starts"
                        } else {
                            "false: the loop does not run"
                        };
                        self.push_control(g.span, written.clone(), info.into(), written);
                        if go {
                            self.frames.push(Frame {
                                items: Rc::new(g.body.clone()),
                                at: 0,
                                kind: Kind::While {
                                    condition: condition.clone(),
                                    rounds: 1,
                                },
                                prefixes,
                            });
                        }
                        Flow::Line
                    }
                }
            }
            Opener::If { condition } => {
                let written = format!("{}if {condition} {{", if g.else_if { "else " } else { "" });
                match self.condition(condition) {
                    Err(Stop::Ask(need)) => Flow::Ask(need),
                    Err(e) => self.refuse_block(g, depth, written, &e),
                    Ok(None) => self.refuse_block(
                        g,
                        depth,
                        written,
                        &Stop::Stata("the condition is text (Stata: type mismatch)".into()),
                    ),
                    Ok(Some(go)) => {
                        self.done_item(depth);
                        let info = match (go, g.otherwise.is_some()) {
                            (true, _) => "true: its block runs",
                            (false, true) => "false: the else block runs",
                            (false, false) => "false: its block is skipped",
                        };
                        self.push_control(g.span, written.clone(), info.into(), written);
                        let body = if go {
                            Some(&g.body)
                        } else {
                            g.otherwise.as_ref()
                        };
                        if let Some(items) = body {
                            self.frames.push(Frame {
                                items: Rc::new(items.clone()),
                                at: 0,
                                kind: Kind::Body,
                                prefixes,
                            });
                        }
                        Flow::Line
                    }
                }
            }
            Opener::Prefixed => {
                self.done_item(depth);
                self.frames.push(Frame {
                    items: Rc::new(g.body.clone()),
                    at: 0,
                    kind: Kind::Body,
                    prefixes,
                });
                Flow::Next
            }
            Opener::Program { name } => {
                self.done_item(depth);
                self.programs.insert(name.clone());
                let text = format!("program define {name}");
                let line = self.t.refused(
                    g.span,
                    text.clone(),
                    format!("programs (program define {name} ... end) are not translated yet; the lines that run {name} are not run"),
                );
                self.push(line, What::Command, None, text);
                Flow::Line
            }
            Opener::Raw { kind, header, .. } => {
                self.done_item(depth);
                let why = match kind.as_str() {
                    "mata" => "Mata is not translated: it is outside what RCraft runs".to_string(),
                    "python" => "python blocks are not translated".to_string(),
                    "input" => {
                        "input (data typed into the do-file) is not translated yet".to_string()
                    }
                    other => format!("{other} blocks are not translated"),
                };
                let text = if header.starts_with(kind.as_str()) {
                    header.trim().to_string()
                } else {
                    format!("{kind} {header}").trim().to_string()
                };
                let line = self.t.refused(g.span, text.clone(), why);
                self.push(line, What::Command, None, text);
                Flow::Line
            }
            Opener::Frame { name } => {
                let written = format!("frame {name} {{");
                self.refuse_block(
                    g,
                    depth,
                    written,
                    &Stop::Not("frames (frame name { }) are not translated yet".into()),
                )
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn enter_loop(
        &mut self,
        g: &Group,
        depth: usize,
        prefixes: Vec<Prefix>,
        name: &str,
        values: Vec<String>,
        shown: String,
        info: String,
        written: String,
    ) -> Flow {
        if values.len() > MAX_ROUNDS {
            return self.refuse_block(
                g,
                depth,
                written,
                &Stop::Not(format!(
                    "the loop goes round more than {MAX_ROUNDS} times, which RCraft does not run"
                )),
            );
        }
        self.done_item(depth);
        self.push_control(g.span, shown, info, written);
        if let Some(first) = values.first().cloned() {
            self.set_local_now(name, Macro::Text(first));
            self.frames.push(Frame {
                items: Rc::new(g.body.clone()),
                at: 0,
                kind: Kind::Foreach {
                    name: name.to_string(),
                    values,
                    next: 1,
                },
                prefixes,
            });
        }
        Flow::Line
    }

    /// A block that is not run: its first line says why, and each command in
    /// it is listed as not run.
    fn refuse_block(&mut self, g: &Group, depth: usize, written: String, e: &Stop) -> Flow {
        self.done_item(depth);
        let why = stop_reason(e);
        let what = if matches!(e, Stop::Later(_)) {
            What::Later
        } else {
            What::Command
        };
        let line = self.t.refused(g.span, written.clone(), why.clone());
        self.push(line, what, None, written);
        let inside = format!("inside a block that is not run: {why}");
        let mut commands = Vec::new();
        collect_commands(&g.body, &mut commands);
        if let Some(o) = &g.otherwise {
            collect_commands(o, &mut commands);
        }
        for c in commands {
            let line = self.t.refused(c.span, c.text.clone(), inside.clone());
            self.push(line, what, None, c.text.clone());
        }
        Flow::Line
    }

    /// Whether `condition` holds (`None`: it is text), with its macros
    /// replaced; R is asked when the value is not known here.
    fn condition(&mut self, condition: &str) -> Result<Option<bool>, Stop> {
        let text = macros::expand(condition, self)?;
        match eval::eval_text(&text) {
            Ok(eval::Val::Num(x)) => Ok(Some(x != 0.0)),
            Ok(eval::Val::Str(_)) => Ok(None),
            Err(eval::NotHere::Stata(m)) => Err(Stop::Stata(m)),
            Err(eval::NotHere::Later) => {
                let v = self.ask(Need::Value(text.trim().to_string()))?;
                Ok(Some(v.trim() != "0"))
            }
        }
    }

    fn foreach_values(&mut self, kind: &str, list: &str) -> Result<Vec<String>, Stop> {
        match kind {
            "local" | "global" => {
                let name = list.trim();
                let v = if kind == "local" {
                    self.local(name)?
                } else {
                    self.global(name)?
                };
                Ok(words(&v))
            }
            "numlist" => numlist(list).map_err(Stop::Stata),
            "newlist" => Ok(words(list)),
            "varlist" => {
                if !varlist::needs_names(list) {
                    return Ok(words(list));
                }
                match self.vars_now()? {
                    Some(vars) => varlist::expand(list, &vars)
                        .map(|t| words(&t))
                        .map_err(Stop::Stata),
                    None => Err(Stop::Later(format!(
                        "foreach over {}: the data's variable names are {KNOWN_WHEN_RUN}",
                        list.trim()
                    ))),
                }
            }
            other => Err(Stop::Stata(format!(
                "foreach ... of {other}: Stata has no such list"
            ))),
        }
    }

    fn run_file(&mut self, c: &Command, rest: &str, written: String) -> Flow {
        let ws = macros::tokens(rest);
        let Some(first) = ws.first() else {
            let line = self.t.refused(
                c.span,
                c.text.clone(),
                format!("{} needs a file name", c.name),
            );
            self.push(line, What::Command, None, written);
            return Flow::Line;
        };
        let file = clean_path(first);
        let file = if has_extension(&file) {
            file
        } else {
            format!("{file}.do")
        };
        let path = join(&self.cwd, &file);
        let deep = self
            .frames
            .iter()
            .filter(|f| matches!(f.kind, Kind::File { .. } | Kind::Include { .. }))
            .count();
        let index = self.lines.len();
        let text = self.files.read(&path);
        self.refs.push(FileRef {
            written: macros::unquote(first),
            path: path.clone(),
            role: FileRole::Run,
            status: if text.is_some() {
                FileStatus::There
            } else {
                FileStatus::Missing
            },
            line: index,
        });
        let Some(text) = text else {
            let line = self.t.refused(
                c.span,
                c.text.clone(),
                format!("the file is not there: {path}"),
            );
            self.push(line, What::Command, None, written);
            return Flow::Line;
        };
        if deep >= MAX_FILES_DEEP {
            let line = self.t.refused(
                c.span,
                c.text.clone(),
                format!("files run from files more than {MAX_FILES_DEEP} deep are not run"),
            );
            self.push(line, What::Command, None, written);
            return Flow::Line;
        }
        let number = self.names.len();
        self.names.push(path.clone());
        let program = reader::read(&text);
        let mut prefixes = c.prefixes.clone();
        if c.name == "run" {
            prefixes.push(Prefix::Quietly);
        }
        let kind = if c.name == "include" {
            Kind::Include { file: number }
        } else {
            // `do file a b`: `0' is "a b", `1' is a, `2' is b
            let mut locals = BTreeMap::new();
            let args: Vec<String> = ws.iter().skip(1).cloned().collect();
            locals.insert("0".to_string(), Macro::Text(args.join(" ")));
            for (i, a) in args.iter().enumerate() {
                locals.insert((i + 1).to_string(), Macro::Text(macros::unquote(a)));
            }
            Kind::File {
                file: number,
                locals,
            }
        };
        let info = format!("runs {path}");
        self.push(nothing(c), What::Control, Some(info), written);
        self.frames.push(Frame {
            items: Rc::new(program.items),
            at: 0,
            kind,
            prefixes,
        });
        Flow::Line
    }

    // ---------------- macros ----------------

    /// The frame whose locals the line being done sees.
    fn locals_mut(&mut self) -> Option<&mut BTreeMap<String, Macro>> {
        self.frames
            .iter_mut()
            .rev()
            .find_map(|f| match &mut f.kind {
                Kind::File { locals, .. } => Some(locals),
                _ => None,
            })
    }

    fn locals(&self) -> Option<&BTreeMap<String, Macro>> {
        self.frames.iter().rev().find_map(|f| match &f.kind {
            Kind::File { locals, .. } => Some(locals),
            _ => None,
        })
    }

    fn set_local_now(&mut self, name: &str, m: Macro) {
        if let Some(l) = self.locals_mut() {
            l.insert(name.to_string(), m);
        }
    }

    fn remove_local(&mut self, name: &str) {
        if let Some(l) = self.locals_mut() {
            l.remove(name);
        }
    }

    /// `macro shift`: `2' becomes `1', and so on.
    fn shift_locals(&mut self) {
        if let Some(l) = self.locals_mut() {
            let mut i = 1;
            while let Some(next) = l.get(&(i + 1).to_string()).cloned() {
                l.insert(i.to_string(), next);
                i += 1;
            }
            l.remove(&i.to_string());
        }
    }

    // ---------------- lines ----------------

    fn push(&mut self, line: Line, what: What, info: Option<String>, written: String) {
        // a line that may have changed the data's variables: R is asked their names again
        if matches!(what, What::Command | What::Later) {
            let word = scan::command_word(&line.stata);
            let full = crate::lang::commands::resolve(&word);
            if line.result.is_err() || !NAMES_STAY.contains(&full) {
                self.vars = None;
            }
        }
        let file = self
            .frames
            .iter()
            .rev()
            .find_map(|f| match f.kind {
                Kind::File { file, .. } | Kind::Include { file } => Some(file),
                _ => None,
            })
            .unwrap_or(0);
        let loops = self
            .frames
            .iter()
            .filter_map(|f| match &f.kind {
                Kind::Foreach { name, values, next } => values
                    .get(next.saturating_sub(1))
                    .map(|v| format!("{name} = {v}")),
                Kind::While { rounds, .. } => Some(format!("while, round {rounds}")),
                _ => None,
            })
            .collect();
        self.origins.push(Origin {
            file,
            span: line.span,
            written,
            loops,
            what,
            info,
        });
        self.lines.push(line);
    }

    fn push_control(&mut self, span: Span, shown: String, info: String, written: String) {
        let line = Line {
            span,
            stata: shown,
            result: Ok(RCode::default()),
        };
        self.push(line, What::Control, Some(info), written);
    }

    fn span_here(&self) -> Span {
        self.lines.last().map(|l| l.span).unwrap_or_default()
    }

    /// The folders the user said are elsewhere, put where they are.
    fn map_paths(&self, text: &str) -> String {
        let mut out = text.to_string();
        for (from, to) in &self.opts.map {
            if from.is_empty() {
                continue;
            }
            out = replace_path(&out, from, to);
        }
        out
    }

    fn file_refs(&mut self, c: &Command, line: usize) {
        let name = c.name.as_str();
        let main = c.parts.main.trim();
        let (word, _) = main.split_once(char::is_whitespace).unwrap_or((main, ""));
        let writes = matches!(
            name,
            "save" | "saveold" | "export" | "outsheet" | "log" | "cmdlog"
        ) || (name == "graph" && word == "export")
            || (name == "estimates" && word == "save");
        let role = if writes {
            FileRole::Write
        } else {
            FileRole::Read
        };
        let (written, ext) = match (&c.using, name) {
            (Some(u), _) => (
                u.clone(),
                if matches!(name, "use" | "merge" | "append") {
                    "dta"
                } else {
                    ""
                },
            ),
            (None, "use" | "save" | "saveold") if !main.is_empty() => (main.to_string(), "dta"),
            (None, "graph") if word == "export" => {
                (main.get(word.len()..).unwrap_or("").trim().to_string(), "")
            }
            (None, "cd") if !main.is_empty() => {
                let dir = clean_path(main);
                let path = join(&self.cwd, &dir);
                let status = if self.files.exists(&path) {
                    FileStatus::There
                } else {
                    FileStatus::Missing
                };
                self.refs.push(FileRef {
                    written: dir,
                    path,
                    role: FileRole::Folder,
                    status,
                    line,
                });
                return;
            }
            _ => return,
        };
        // the file name, without the options after it
        let first = macros::tokens(written.split(',').next().unwrap_or(""))
            .into_iter()
            .next()
            .unwrap_or_default();
        if first.is_empty() {
            return;
        }
        let mut file = clean_path(&first);
        if !ext.is_empty() && !has_extension(&file) {
            file = format!("{file}.{ext}");
        }
        let path = join(&self.cwd, &file);
        let key = path.to_ascii_lowercase();
        let status = if role == FileRole::Write {
            self.written.insert(key);
            FileStatus::Written
        } else if self.written.contains(&key) {
            FileStatus::MadeHere
        } else if self.files.exists(&path) {
            FileStatus::There
        } else {
            FileStatus::Missing
        };
        self.refs.push(FileRef {
            written: file,
            path,
            role,
            status,
            line,
        });
    }
}

impl Env for Run<'_> {
    fn local(&mut self, name: &str) -> Result<String, Stop> {
        if let Some((_, v)) = self.pending.iter().rev().find(|(n, _)| n == name) {
            return Ok(v.clone());
        }
        match self.locals().and_then(|l| l.get(name)) {
            Some(Macro::Text(t)) => Ok(t.clone()),
            Some(Macro::Later(why)) => Err(Stop::Later(format!("`{name}' {why}"))),
            Some(Macro::Lost(why)) => Err(Stop::Not(format!("`{name}' is not known: {why}"))),
            // macval(), a class member (`.a.b'): forms of macro RCraft does not have
            None if name.contains(['(', '.']) => Err(Stop::Not(format!(
                "`{name}': this form of macro (macval(), class members) is not translated yet"
            ))),
            None => Ok(String::new()),
        }
    }

    fn global(&mut self, name: &str) -> Result<String, Stop> {
        match self.globals.get(name) {
            Some(Macro::Text(t)) => Ok(t.clone()),
            Some(Macro::Later(why)) => Err(Stop::Later(format!("${name} {why}"))),
            Some(Macro::Lost(why)) => Err(Stop::Not(format!("${name} is not known: {why}"))),
            // the S_ globals Stata sets itself (S_level, ...) are not kept
            None if name.starts_with("S_") => Err(Stop::Not(format!(
                "${name} is one of the globals Stata sets itself, which RCraft does not keep"
            ))),
            None => match &self.globals_unknown {
                Some(line) => Err(Stop::Not(format!(
                    "${name} is not known: {line}, which was not translated, may have set it"
                ))),
                None => Ok(String::new()),
            },
        }
    }

    fn set_local(&mut self, name: &str, value: String) {
        self.pending.push((name.to_string(), value));
    }

    fn inline(&mut self, exp: &str) -> Result<String, Stop> {
        match eval::eval_text(exp) {
            Ok(v) => Ok(v.macro_text()),
            Err(eval::NotHere::Stata(m)) => Err(Stop::Stata(m)),
            Err(eval::NotHere::Later) => self.ask(Need::Value(exp.trim().to_string())),
        }
    }

    fn extended(&mut self, text: &str) -> Result<String, Stop> {
        let (word, rest) = split_word(text);
        let macro_text = |run: &mut Self, kind: &str, name: &str| -> Result<String, Stop> {
            match kind {
                "local" => run.local(name),
                "global" => run.global(name),
                _ => Err(Stop::Stata(format!("{kind}: local or global expected"))),
            }
        };
        match word {
            "word" => {
                let (second, after) = split_word(rest);
                if second == "count" {
                    return Ok(words(after).len().to_string());
                }
                let n: usize = second
                    .parse()
                    .map_err(|_| Stop::Stata(format!(": word {second}: a number is expected")))?;
                let after = after.trim_start();
                let list = after
                    .strip_prefix("of")
                    .ok_or_else(|| Stop::Stata(": word # of: of is expected".into()))?;
                Ok(words(list)
                    .get(n.wrapping_sub(1))
                    .cloned()
                    .unwrap_or_default())
            }
            "list" => self.list_function(rest),
            "length" | "strlen" | "ustrlen" => {
                let (kind, name) = split_word(rest);
                let v = macro_text(self, kind, name.trim())?;
                Ok(if word == "ustrlen" {
                    v.chars().count()
                } else {
                    v.len()
                }
                .to_string())
            }
            "subinstr" => {
                let (kind, after) = split_word(rest);
                let (name, after) = split_word(after);
                let v = macro_text(self, kind, name)?;
                let (args, opts) = after.split_once(',').unwrap_or((after, ""));
                let parts = macros::tokens(args);
                let [from, to] = parts.as_slice() else {
                    return Err(Stop::Stata(
                        ": subinstr: two quoted texts are expected".into(),
                    ));
                };
                let (from, to) = (macros::unquote(from), macros::unquote(to));
                let all = opts.split_whitespace().any(|o| o == "all");
                let word_only = opts.split_whitespace().any(|o| o == "word");
                let (out, count) = subinstr(&v, &from, &to, all, word_only);
                if let Some(c) = opts.split("count(").nth(1) {
                    let inner = c.split(')').next().unwrap_or("");
                    let (k, n) = split_word(inner);
                    if k != "local" {
                        return Err(Stop::Not(
                            ": subinstr with count(global ...) is not translated yet".into(),
                        ));
                    }
                    self.set_local(n.trim(), count.to_string());
                }
                Ok(out)
            }
            "display" | "di" | "dis" | "disp" | "displ" | "displa" => self.ask(Need::Display(rest.trim().to_string())),
            "variable" if split_word(rest).0 == "label" => {
                let var = split_word(rest).1.trim();
                let d = self
                    .t
                    .data()
                    .ok_or_else(|| Stop::Stata("no variables defined".into()))?
                    .to_string();
                let v = crate::expr_r::r_text(var);
                self.ask(Need::R(
                    format!("cat({{ l <- attr({d}[[{v}]], \"label\", exact = TRUE); if (is.null(l)) \"\" else l }})"),
                    format!("the label of {var}"),
                ))
            }
            // `: label (var) #` from the data's labels; `: label set #` from the do-file's label define
            "label" => {
                let arg = rest.trim();
                let number = |value: &str| -> Result<f64, Stop> {
                    value
                        .parse()
                        .map_err(|_| Stop::Stata(format!(": label: {value} is not a number")))
                };
                if let Some(inner) = arg.strip_prefix('(') {
                    let (var, value) = inner.split_once(')').unwrap_or((inner, ""));
                    let value = value.split(',').next().unwrap_or("").trim();
                    let x = number(value)?;
                    let d = self
                        .t
                        .data()
                        .ok_or_else(|| Stop::Stata("no variables defined".into()))?
                        .to_string();
                    let v = crate::expr_r::r_text(var.trim());
                    return self.ask(Need::R(
                        format!(
                            "cat({{ l <- attr({d}[[{v}]], \"labels\", exact = TRUE); k <- names(l)[!is.na(l) & l == {}]; if (length(k)) k[1] else {} }})",
                            crate::expr_r::r_number(x),
                            crate::expr_r::r_text(value)
                        ),
                        format!("the label of {value} in {}", var.trim()),
                    ));
                }
                let (set, value) = split_word(arg);
                let x = number(value.split(',').next().unwrap_or("").trim())?;
                self.t.label_text(set, x).ok_or_else(|| {
                    Stop::Not(format!(
                        ": label {set}: the value label {set} is not defined in the do-file (a value label read with the data has no name in R)"
                    ))
                })
            }
            "type" | "value" | "format" => Err(Stop::Not(format!(
                "the macro function : {word} is not translated: R does not keep Stata's storage types, value-label names or display formats"
            ))),
            _ => {
                let name: Vec<&str> = text.split_whitespace().take(2).collect();
                Err(Stop::Not(format!(
                    "the macro function : {} is not translated yet",
                    name.join(" ")
                )))
            }
        }
    }
}

impl Run<'_> {
    fn list_function(&mut self, rest: &str) -> Result<String, Stop> {
        use macros::ListOp;
        let ws: Vec<&str> = rest.split_whitespace().collect();
        let get = |run: &mut Self, name: &str| run.local(name);
        match ws.as_slice() {
            [op @ ("uniq" | "dups" | "sort" | "sizeof" | "retokenize" | "clean"), name] => {
                let a = get(self, name)?;
                let op = match *op {
                    "uniq" => ListOp::Uniq,
                    "dups" => ListOp::Dups,
                    "sort" => ListOp::Sort,
                    "sizeof" => ListOp::Sizeof,
                    _ => ListOp::Retokenize,
                };
                Ok(macros::list_op(op, &a, ""))
            }
            ["posof", ..] => {
                let after = rest.trim_start().strip_prefix("posof").unwrap_or("");
                let (elt, tail) = after
                    .rsplit_once(" in ")
                    .ok_or_else(|| Stop::Stata(": list posof: in is expected".into()))?;
                let a = get(self, tail.trim())?;
                let want = macros::unquote(elt.trim());
                let at = macros::tokens(&a)
                    .iter()
                    .position(|t| macros::unquote(t) == want)
                    .map_or(0, |i| i + 1);
                Ok(at.to_string())
            }
            [a, op @ ("-" | "|" | "&" | "in" | "==" | "==="), b] => {
                let (x, y) = (get(self, a)?, get(self, b)?);
                let op = match *op {
                    "-" => ListOp::Minus,
                    "|" => ListOp::Union,
                    "&" => ListOp::Intersect,
                    "in" => ListOp::In,
                    "==" => ListOp::Same,
                    _ => ListOp::SameSet,
                };
                Ok(macros::list_op(op, &x, &y))
            }
            _ => Err(Stop::Not(format!(
                "the macro function : list {} is not translated yet",
                ws.first().unwrap_or(&"")
            ))),
        }
    }
}

/// A dry run: the do-file gone through without R, so every value known only
/// when it runs is left unknown (P2C.1).
pub fn dry_run(text: &str, files: &dyn Files, opts: Options) -> DoFile {
    let mut run = Run::new(text, files, opts);
    loop {
        match run.next_event() {
            Event::Line => {}
            Event::Ask(_) => run.give(Err(KNOWN_WHEN_RUN.into())),
            Event::Done => break,
        }
    }
    run.finish()
}

// ---------------- helpers ----------------

/// `tokenize`'s text and its `parse()` characters.
fn tokenize_parts(rest: &str) -> Result<(String, String), String> {
    let chars: Vec<char> = rest.chars().collect();
    let at = |k: usize| chars.get(k).copied();
    let mut i = 0;
    let mut comma = None;
    while let Some(ch) = at(i) {
        match ch {
            '`' if at(i + 1) == Some('"') => i = compound_end(&chars, i),
            '"' => i = closing_quote(&chars, i + 1) + 1,
            ',' => {
                comma = Some(i);
                break;
            }
            _ => i += 1,
        }
    }
    let Some(comma) = comma else {
        return Ok((macros::unquote(rest.trim()), String::new()));
    };
    let text: String = chars.iter().take(comma).collect();
    let opts: String = chars.iter().skip(comma + 1).collect();
    let opts = opts.trim();
    let inner = opts
        .strip_prefix("parse(")
        .or_else(|| opts.strip_prefix("p("))
        .and_then(|o| o.strip_suffix(')'))
        .ok_or_else(|| format!("tokenize with the option {opts} is not translated"))?;
    Ok((macros::unquote(text.trim()), macros::unquote(inner.trim())))
}

/// What `gettoken` names: the macros it sets (with whether each is
/// global), the macro it reads, its `parse()` characters and its `match()`
/// macro.
struct GetToken {
    targets: Vec<(String, bool)>,
    source: (String, bool),
    parse: String,
    matched: Option<(String, bool)>,
}

/// `name`, `(global) name`, `(local) name`, in order.
fn qualified_names(text: &str) -> Vec<(String, bool)> {
    let mut out = Vec::new();
    let mut global = false;
    for w in text.split_whitespace() {
        match w {
            "(local)" => global = false,
            "(global)" => global = true,
            n if macros::is_name(n) => {
                out.push((n.to_string(), global));
                global = false;
            }
            _ => {}
        }
    }
    out
}

/// `gettoken a [b] : c [, parse("...") match(m)]`.
fn gettoken_parts(rest: &str) -> Result<GetToken, String> {
    let (names, after) = rest
        .split_once(':')
        .ok_or("gettoken needs `:` before the macro it reads")?;
    let targets = qualified_names(names);
    if targets.is_empty() || targets.len() > 2 {
        return Err("gettoken takes one or two macro names before `:`".into());
    }
    let (src, opts) = match after.split_once(',') {
        Some((s, o)) => (s, o.trim()),
        None => (after, ""),
    };
    let source = match qualified_names(src).as_slice() {
        [one] => one.clone(),
        _ => return Err("gettoken reads one macro after `:`".into()),
    };
    let mut parse = " ".to_string();
    let mut matched = None;
    let mut o = opts;
    while !o.is_empty() {
        let (word, arg, next) = match o.find('(') {
            Some(open) if !o[..open].contains(char::is_whitespace) => {
                let close = o[open..]
                    .find(')')
                    .map(|k| k + open)
                    .ok_or("a ( without its ) in gettoken's options")?;
                (&o[..open], Some(&o[open + 1..close]), o[close + 1..].trim())
            }
            _ => match o.split_once(char::is_whitespace) {
                Some((w, n)) => (w, None, n.trim()),
                None => (o, None, ""),
            },
        };
        match (word, arg) {
            (w, Some(a)) if !w.is_empty() && "parse".starts_with(w) => {
                parse = macros::unquote(a.trim());
            }
            (w, Some(a)) if !w.is_empty() && "match".starts_with(w) => {
                matched = Some(
                    qualified_names(a)
                        .first()
                        .cloned()
                        .ok_or("match() needs a macro name")?,
                );
            }
            _ => {
                return Err(format!(
                    "gettoken with the option {word} is not translated yet"
                ))
            }
        }
        o = next;
    }
    Ok(GetToken {
        targets,
        source,
        parse,
        matched,
    })
}

/// The first token of `text` and the rest, as Stata 19's gettoken takes
/// them: leading blanks skipped; a quoted token without its quotes; a
/// `parse` character a token of its own (`<=` and `==` stay together);
/// otherwise up to the next `parse` character, which the rest keeps. With
/// `paren`, a token in parentheses is what is inside them, and the third
/// value is `(` (empty otherwise).
fn gettoken(text: &str, parse: &str, paren: bool) -> (String, String, String) {
    let chars: Vec<char> = text.chars().collect();
    let at = |k: usize| chars.get(k).copied();
    let from =
        |a: usize, b: usize| -> String { chars.iter().skip(a).take(b.saturating_sub(a)).collect() };
    let mut i = 0;
    while at(i) == Some(' ') {
        i += 1;
    }
    let Some(first) = at(i) else {
        return (String::new(), String::new(), String::new());
    };
    if paren && first == '(' {
        let mut depth = 0usize;
        let mut k = i;
        while let Some(ch) = at(k) {
            if ch == '(' {
                depth += 1;
            } else if ch == ')' {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return (from(i + 1, k), from(k + 1, chars.len()), "(".into());
                }
            }
            k += 1;
        }
    }
    if first == '`' && at(i + 1) == Some('"') {
        let end = compound_end(&chars, i);
        return (
            from(i + 2, end.saturating_sub(2)),
            from(end, chars.len()),
            String::new(),
        );
    }
    if first == '"' {
        let end = closing_quote(&chars, i + 1);
        return (from(i + 1, end), from(end + 1, chars.len()), String::new());
    }
    if parse.contains(first) {
        let len = if "=><!~".contains(first) && at(i + 1) == Some('=') {
            2
        } else {
            1
        };
        return (from(i, i + len), from(i + len, chars.len()), String::new());
    }
    let end = (i..chars.len())
        .find(|&k| at(k).is_some_and(|ch| parse.contains(ch)))
        .unwrap_or(chars.len());
    (from(i, end), from(end, chars.len()), String::new())
}

/// Where the `"` closing a quote is, from `from` on (the end if none).
fn closing_quote(chars: &[char], from: usize) -> usize {
    (from..chars.len())
        .find(|&k| chars.get(k) == Some(&'"'))
        .unwrap_or(chars.len())
}

/// Where a compound quote opened at `i` ends (just after its `"'`).
fn compound_end(chars: &[char], i: usize) -> usize {
    let at = |k: usize| chars.get(k).copied();
    let mut depth = 0;
    let mut k = i;
    while let Some(ch) = at(k) {
        if ch == '`' && at(k + 1) == Some('"') {
            depth += 1;
            k += 2;
        } else if ch == '"' && at(k + 1) == Some('\'') {
            depth -= 1;
            k += 2;
            if depth == 0 {
                return k;
            }
        } else {
            k += 1;
        }
    }
    chars.len()
}

/// The tokens of `text`: spaces separate, quotes hold words together and
/// are taken off, and each `parse` character is a token of its own.
fn tokenize(text: &str, parse: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let at = |k: usize| chars.get(k).copied();
    let between =
        |a: usize, b: usize| -> String { chars.iter().skip(a).take(b.saturating_sub(a)).collect() };
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut i = 0;
    let flush = |cur: &mut String, out: &mut Vec<String>| {
        if !cur.is_empty() {
            out.push(std::mem::take(cur));
        }
    };
    while let Some(ch) = at(i) {
        if ch == '`' && at(i + 1) == Some('"') {
            flush(&mut cur, &mut out);
            let end = compound_end(&chars, i);
            out.push(between(i + 2, end.saturating_sub(2)));
            i = end;
        } else if ch == '"' {
            flush(&mut cur, &mut out);
            let end = closing_quote(&chars, i + 1);
            out.push(between(i + 1, end));
            i = end + 1;
        } else if ch.is_whitespace() {
            flush(&mut cur, &mut out);
            i += 1;
        } else if parse.contains(ch) {
            flush(&mut cur, &mut out);
            // ==, >=, <=, != and ~= stay one token, = a parse character or not
            if "=><!~".contains(ch) && at(i + 1) == Some('=') {
                out.push(format!("{ch}="));
                i += 2;
            } else {
                out.push(ch.to_string());
                i += 1;
            }
        } else {
            cur.push(ch);
            i += 1;
        }
    }
    flush(&mut cur, &mut out);
    out
}

fn nothing(c: &Command) -> Line {
    Line {
        span: c.span,
        stata: c.text.clone(),
        result: Ok(RCode::default()),
    }
}

fn stop_reason(e: &Stop) -> String {
    match e {
        Stop::Later(why) => why.clone(),
        Stop::Not(why) => why.clone(),
        Stop::Stata(m) => format!("Stata stops here with an error: {m}"),
        Stop::Ask(need) => format!("{} is {KNOWN_WHEN_RUN}", need.what()),
    }
}

/// `name is "value"`, cut short when long.
fn show(name: &str, value: &str) -> String {
    let mut v: String = value.chars().take(120).collect();
    if v.len() < value.len() {
        v.push_str("...");
    }
    format!("{name} is \"{v}\"")
}

fn rounds_info(values: &[String]) -> String {
    match values.len() {
        0 => "runs no times".to_string(),
        n => {
            let shown: Vec<&str> = values.iter().take(8).map(String::as_str).collect();
            let more = if n > 8 { ", ..." } else { "" };
            let times = if n == 1 { "time" } else { "times" };
            format!("runs {n} {times}: {}{more}", shown.join(", "))
        }
    }
}

/// The text after the command word (`local`, `loc`) in a line.
fn after_word(text: &str, written: &str) -> String {
    let mut search = 0;
    while let Some(rel) = text.get(search..).and_then(|t| t.find(written)) {
        let at = search + rel;
        let before_ok = at == 0
            || text
                .get(..at)
                .and_then(|b| b.chars().last())
                .is_some_and(|c| c.is_whitespace() || c == ':');
        let end = at + written.len();
        let after_ok = text
            .get(end..)
            .and_then(|a| a.chars().next())
            .is_none_or(|c| !(c.is_ascii_alphanumeric() || c == '_'));
        if before_ok && after_ok {
            return text.get(end..).unwrap_or("").to_string();
        }
        search = end;
    }
    String::new()
}

fn split_word(text: &str) -> (&str, &str) {
    let t = text.trim_start();
    let end = t.find(char::is_whitespace).unwrap_or(t.len());
    (t.get(..end).unwrap_or(""), t.get(end..).unwrap_or(""))
}

fn collect_commands<'a>(items: &'a [Item], out: &mut Vec<&'a Command>) {
    for i in items {
        match i {
            Item::Command(c) => out.push(c),
            Item::Group(g) => {
                collect_commands(&g.body, out);
                if let Some(o) = &g.otherwise {
                    collect_commands(o, out);
                }
            }
            Item::Problem(_) => {}
        }
    }
}

/// `subinstr` on a macro's text: the first `from` (or `all`), or whole
/// words only (`word`); and how many were changed.
fn subinstr(text: &str, from: &str, to: &str, all: bool, word_only: bool) -> (String, usize) {
    if from.is_empty() {
        return (text.to_string(), 0);
    }
    if word_only {
        let mut count = 0;
        let mut out = Vec::new();
        for w in text.split(' ') {
            if w == from && (all || count == 0) {
                count += 1;
                out.push(to.to_string());
            } else {
                out.push(w.to_string());
            }
        }
        return (out.join(" "), count);
    }
    if all {
        let count = text.matches(from).count();
        (text.replace(from, to), count)
    } else if text.contains(from) {
        (text.replacen(from, to, 1), 1)
    } else {
        (text.to_string(), 0)
    }
}

/// The values of `forvalues`: `a/b`, `a(d)b`, `a d to b` and `a d : b`,
/// each added on to the one before, as Stata 19 does
/// (`0(.1)1` reaches `.9999999999999999`), kept as %18.0g.
fn forvalues(range: &str) -> Result<Vec<String>, String> {
    let r = range.trim();
    let num = |s: &str| -> Result<f64, String> {
        s.trim()
            .parse::<f64>()
            .map_err(|_| format!("{s}: a number is expected (Stata: invalid syntax)"))
    };
    let (start, step, end) = if let Some((a, b)) = r.split_once('/') {
        (num(a)?, 1.0, num(b)?)
    } else if let (Some(open), Some(close)) = (r.find('('), r.find(')')) {
        let a = r.get(..open).unwrap_or("");
        let d = r.get(open + 1..close).unwrap_or("");
        let b = r.get(close + 1..).unwrap_or("");
        (num(a)?, num(d)?, num(b)?)
    } else {
        let ws: Vec<&str> = r.split_whitespace().collect();
        match ws.as_slice() {
            [a, t, "to" | ":", b] => {
                let a = num(a)?;
                (a, num(t)? - a, num(b)?)
            }
            _ => return Err(format!("forvalues {r}: Stata would not read the range")),
        }
    };
    if step == 0.0 || !step.is_finite() {
        return Err("forvalues: the step is zero".into());
    }
    let mut out = Vec::new();
    let mut v = start;
    while (step > 0.0 && v <= end) || (step < 0.0 && v >= end) {
        out.push(format::g(v, 18));
        if out.len() > MAX_ROUNDS {
            break;
        }
        v += step;
    }
    Ok(out)
}

/// A numlist's numbers, as `foreach ... of numlist` keeps them (%15.0g):
/// `1/3`, `10(5)20`, `1[2]5`, `1 3 to 9`, `1 2: 6`, single numbers.
fn numlist(text: &str) -> Result<Vec<String>, String> {
    let bad = |s: &str| format!("{s}: invalid numlist");
    let num = |s: &str| -> Result<f64, String> { s.trim().parse::<f64>().map_err(|_| bad(s)) };
    let mut out: Vec<String> = Vec::new();
    let push_range = |out: &mut Vec<String>, a: f64, d: f64, b: f64| -> Result<(), String> {
        if d == 0.0 || !d.is_finite() {
            return Err(bad("0"));
        }
        let n = ((b - a) / d + 1e-7).floor();
        if n < 0.0 {
            return Ok(());
        }
        if n > MAX_ROUNDS as f64 {
            return Err(format!("the numlist has more than {MAX_ROUNDS} numbers"));
        }
        for i in 0..=(n as usize) {
            out.push(format::g(a + i as f64 * d, 15));
        }
        Ok(())
    };
    let text = text.replace(',', " ").replace(':', " : ");
    let ws: Vec<&str> = text.split_whitespace().collect();
    let mut i = 0;
    while let Some(w) = ws.get(i) {
        // a b to c, a b : c
        if let (Some(t), Some(b)) = (ws.get(i + 2), ws.get(i + 3)) {
            if matches!(*t, "to" | ":") {
                if let (Ok(a), Ok(next), Ok(b)) =
                    (num(w), num(ws.get(i + 1).unwrap_or(&"")), num(b))
                {
                    push_range(&mut out, a, next - a, b)?;
                    i += 4;
                    continue;
                }
            }
        }
        if let Some((a, b)) = w.split_once('/') {
            let (a, b) = (num(a)?, num(b)?);
            push_range(&mut out, a, if b >= a { 1.0 } else { -1.0 }, b)?;
        } else if let Some(open) = w.find(['(', '[']) {
            let close = w.find([')', ']']).ok_or_else(|| bad(w))?;
            let a = num(w.get(..open).unwrap_or(""))?;
            let d = num(w.get(open + 1..close).unwrap_or(""))?;
            let b = num(w.get(close + 1..).unwrap_or(""))?;
            push_range(&mut out, a, d, b)?;
        } else {
            out.push(format::g(num(w)?, 15));
        }
        i += 1;
    }
    Ok(out)
}

/// Whether a command that was not translated may have set globals: one
/// outside the tiers of Stata's commands RCraft knows (an add-on, an ado
/// program) or a program-tier command that runs other code. A line Stata
/// stops at, or one only R can judge, sets nothing new.
fn may_set_globals(command: &str, reason: &str) -> bool {
    use crate::coverage::{tier, Tier};
    if reason.contains("Stata stops here")
        || reason.contains("(Stata: ")
        || reason.contains(KNOWN_WHEN_RUN)
    {
        return false;
    }
    // Stata's own commands that set up the session set no globals of their own
    if matches!(
        command,
        "sysdir" | "adopath" | "which" | "log" | "cmdlog" | "cd" | "help" | "discard" | "pwd"
    ) {
        return false;
    }
    match tier(command) {
        Tier::Other => true,
        Tier::Programs => !matches!(
            command,
            "args" | "syntax" | "tokenize" | "gettoken" | "end" | "program" | "ssc" | "net" | "ado"
        ),
        _ => false,
    }
}

/// A path as written: quotes off, `\` as `/`.
fn clean_path(written: &str) -> String {
    macros::unquote(written.trim()).trim().replace('\\', "/")
}

fn has_extension(file: &str) -> bool {
    let name = file.rsplit('/').next().unwrap_or(file);
    name.contains('.') && !name.ends_with('.')
}

fn is_absolute(path: &str) -> bool {
    let b = path.as_bytes();
    path.starts_with('/')
        || path.starts_with('~')
        || (b.len() >= 2
            && b.first().is_some_and(u8::is_ascii_alphabetic)
            && b.get(1) == Some(&b':'))
}

/// `file` from the folder `cwd`, with `.` and `..` worked out.
fn join(cwd: &str, file: &str) -> String {
    let full = if is_absolute(file) || cwd.is_empty() {
        file.to_string()
    } else {
        format!("{}/{file}", cwd.trim_end_matches('/'))
    };
    let lead = if full.starts_with('/') { "/" } else { "" };
    let mut parts: Vec<&str> = Vec::new();
    for p in full.split('/') {
        match p {
            "" | "." => {}
            ".." if parts
                .last()
                .is_some_and(|l| *l != ".." && !l.ends_with(':')) =>
            {
                parts.pop();
            }
            p => parts.push(p),
        }
    }
    format!("{lead}{}", parts.join("/"))
}

/// Every `from` in `text` (letters' case and `\` or `/` aside) made `to`.
fn replace_path(text: &str, from: &str, to: &str) -> String {
    let same = |a: char, b: char| {
        let norm = |c: char| {
            if c == '\\' {
                '/'
            } else {
                c.to_ascii_lowercase()
            }
        };
        norm(a) == norm(b)
    };
    let t: Vec<char> = text.chars().collect();
    let f: Vec<char> = from.trim_end_matches(['/', '\\']).chars().collect();
    let to = to.trim_end_matches(['/', '\\']);
    let mut out = String::new();
    let mut i = 0;
    while i < t.len() {
        let hit = !f.is_empty()
            && t.get(i..i + f.len())
                .is_some_and(|w| w.iter().zip(&f).all(|(a, b)| same(*a, *b)));
        if hit {
            out.push_str(to);
            i += f.len();
        } else if let Some(c) = t.get(i) {
            out.push(*c);
            i += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests;
