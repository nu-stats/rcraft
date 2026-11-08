//! Stata typed a line at a time (P2B.7): the Command window and the Script
//! Editor in Stata mode.
//!
//! A line means what it does in the context of the lines before it (which
//! data is in memory, which model left e(), which scalars there are), so
//! [`Live`] keeps the Stata run so far and translates each new line after it.
//! It hands back that line's R together with any helper R has not been given
//! yet, and says what the session holds for Tab (variables come from R; the
//! data's R name, macros and value labels from here).
//!
//! Macros and loops work as in a do-file ([`crate::dofile`], without R).
//!
//! Each step translates everything again, which keeps one translator and no
//! second copy of its state: a do-file is small next to what R does with it.
//! Text the reader cannot take as commands of its own (a block left open, a
//! line it cannot read) is answered but not kept, so it cannot swallow the
//! lines typed after it.

use std::collections::BTreeSet;

use crate::data_r::{Line, Translation, ValueCode};
use crate::dofile::{dry_run, Event, NoFiles, Options, Run};
use crate::reader::{read, Item};

/// The Stata so far, run as a do-file is (macros set, loops run) but without
/// values from R: what needs one says it is known only when the do-file runs.
fn run(text: &str) -> Translation {
    dry_run(text, &NoFiles, Options::default()).translation
}

/// The first and last lines around the helpers in the R a step runs, so the
/// Results window can fold them to one line (the session script keeps them).
pub const HELPERS_BEGIN: &str = "# RCraft's Stata helpers, defined once:";
pub const HELPERS_END: &str = "# end of RCraft's Stata helpers";

/// What a step made.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Step {
    /// Each Stata command of the text and what it became.
    pub lines: Vec<Line>,
    /// The R to run: helpers R does not have yet, then each line's R with its
    /// Stata as a comment and its notes.
    pub code: String,
    /// The packages its `ssc install` lines ask for, for the window to offer
    /// the R package of the same name (P2B.11).
    pub ssc: Vec<String>,
    /// R must work a value out first (`local m = r(mean)`): run this out of
    /// sight, give what it printed to [`Live::give`], and step the same text
    /// again. Nothing else is in the step, and nothing was kept.
    pub ask: Option<String>,
    /// The helpers `code` (or `ask`) gives R. If R stops there (a line R
    /// cannot read runs nothing), hand them to [`Live::unsend`] so the next
    /// step gives them again.
    pub helpers: Vec<String>,
}

/// The Stata of one session so far.
#[derive(Debug, Clone, Default)]
pub struct Live {
    /// Every command kept, one after another.
    source: String,
    /// How many translated lines `source` makes.
    done: usize,
    /// The helpers already sent to R.
    sent: BTreeSet<String>,
    data: Option<String>,
    value_labels: Vec<String>,
    locals: Vec<String>,
    globals: Vec<String>,
    /// How many `ssc install` lines have been seen.
    ssc: usize,
    /// What R printed for each value it was asked for, in order; each step
    /// runs the Stata so far again with them.
    answers: Vec<Result<String, String>>,
}

/// Why a value from R is not asked for in the middle of several lines.
const ON_ITS_OWN: &str = "in the Command window and the Script Editor a value only R knows is worked out on a line of its own, after the lines before it have run (run the lines before it first, or the whole file with File > Run do-file)";

impl Live {
    pub fn new() -> Live {
        Live::default()
    }

    /// Translates `text` after what came before, and keeps it.
    pub fn step(&mut self, text: &str) -> Step {
        let own = read(text);
        let keep = own.items.iter().all(|i| !matches!(i, Item::Problem(_)));
        if !keep {
            // a block left open, or a line the reader cannot take: answered on its own
            let t = run(text);
            let code = t.lines.iter().map(Line::script).collect();
            return Step {
                lines: t.lines,
                code,
                ..Step::default()
            };
        }
        let mut whole = self.source.clone();
        whole.push_str(text.trim_end_matches(['\n', '\r']));
        whole.push('\n');
        let program = read(&whole);
        let t = loop {
            let (t, ask) = self.replay(&whole);
            match ask {
                None => break t,
                // the value is read now, before any line of this text has run
                Some(v) if t.lines.len() <= self.done => {
                    let (mut code, helpers) = self.helpers_code(&t.helpers);
                    code.push_str(&v.code);
                    return Step {
                        ask: Some(code),
                        helpers,
                        ..Step::default()
                    };
                }
                // lines of this text come first and have not run: R would give
                // the value from before them
                Some(_) => self.answers.push(Err(ON_ITS_OWN.into())),
            }
        };
        let lines: Vec<Line> = t.lines.get(self.done..).unwrap_or_default().to_vec();
        let (mut code, helpers) = self.helpers_code(&t.helpers);
        for l in &lines {
            code.push_str(&l.script());
        }
        self.done = t.lines.len();
        self.source = whole;
        self.data = t.data;
        self.value_labels = t.value_labels;
        for c in program.commands() {
            let name = c.parts.main.split_whitespace().next().unwrap_or("");
            let list = match c.name.as_str() {
                "local" => &mut self.locals,
                "global" => &mut self.globals,
                _ => continue,
            };
            if !name.is_empty() && !list.iter().any(|n| n == name) {
                list.push(name.to_string());
            }
        }
        let ssc = t.ssc.get(self.ssc..).unwrap_or_default().to_vec();
        self.ssc = t.ssc.len();
        Step {
            lines,
            code,
            ssc,
            ask: None,
            helpers,
        }
    }

    /// The helpers of a step R stopped at, to be given again.
    pub fn unsend(&mut self, names: &[String]) {
        self.sent
            .retain(|h| !helper_name(h).is_some_and(|n| names.iter().any(|m| m == n)));
    }

    /// What R printed for the last [`Step::ask`], or why it could not; then
    /// the same text is stepped again.
    pub fn give(&mut self, printed: Result<String, String>) {
        self.answers.push(printed);
    }

    /// `text` run with the answers R has given so far, in the order they were
    /// asked; and the next question, if it needs another.
    fn replay(&self, text: &str) -> (Translation, Option<ValueCode>) {
        let mut run = Run::new(text, &NoFiles, Options::default());
        let mut used = 0;
        loop {
            match run.next_event() {
                Event::Line => {}
                Event::Ask(v) => match self.answers.get(used) {
                    Some(a) => {
                        run.give(a.clone());
                        used += 1;
                    }
                    None => return (run.finish().translation, Some(v)),
                },
                Event::Done => return (run.finish().translation, None),
            }
        }
    }

    /// The helpers R does not have yet, between the lines Results folds,
    /// and their names; they count as sent.
    fn helpers_code(&mut self, helpers: &[String]) -> (String, Vec<String>) {
        let new: Vec<&String> = helpers.iter().filter(|h| !self.sent.contains(*h)).collect();
        if new.is_empty() {
            return (String::new(), Vec::new());
        }
        let names: Vec<&str> = new.iter().filter_map(|h| helper_name(h)).collect();
        let mut code = format!("{HELPERS_BEGIN} {}\n", names.join(", "));
        for h in &new {
            code.push_str(h);
            code.push('\n');
        }
        code.push_str(HELPERS_END);
        code.push('\n');
        for h in new {
            self.sent.insert(h.clone());
        }
        (code, names.iter().map(|n| n.to_string()).collect())
    }

    /// The R name of the data in memory, as the translator follows it.
    pub fn data(&self) -> Option<&str> {
        self.data.as_deref()
    }

    pub fn value_labels(&self) -> &[String] {
        &self.value_labels
    }

    pub fn locals(&self) -> &[String] {
        &self.locals
    }

    pub fn globals(&self) -> &[String] {
        &self.globals
    }

    /// The Stata run so far.
    pub fn source(&self) -> &str {
        &self.source
    }
}

/// The name a helper defines (`stata_fmt <- function` is `stata_fmt`).
fn helper_name(code: &str) -> Option<&str> {
    let (name, _) = code.split_once(" <- ")?;
    let name = name.lines().next()?.trim();
    (!name.is_empty() && !name.contains(' ')).then_some(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A step with the data's names given when it asks for them, as the
    /// window does (a name in a varlist may abbreviate a variable).
    fn answered(live: &mut Live, text: &str) -> Step {
        loop {
            let s = live.step(text);
            match &s.ask {
                Some(code) if code.contains("cat(names(") => {
                    live.give(Ok("make\nprice\nmpg\nweight\n".into()))
                }
                _ => return s,
            }
        }
    }

    #[test]
    fn each_line_means_what_it_does_after_the_ones_before() {
        let mut live = Live::new();
        let a = live.step("use auto");
        assert_eq!(a.lines.len(), 1);
        assert_eq!(live.data(), Some("auto"));
        let b = answered(&mut live, "summarize price");
        // the helper comes once, before the line that first needs it
        assert!(b.code.starts_with(HELPERS_BEGIN), "{}", b.code);
        assert!(b.code.contains("stata_summarize <- function"));
        assert!(b.code.contains(HELPERS_END));
        assert!(b.code.contains("# Stata: summarize price\n"));
        let c = answered(&mut live, "summarize mpg");
        assert!(!c.code.contains(HELPERS_BEGIN), "{}", c.code);
        assert!(c.code.starts_with("# Stata: summarize mpg\n"), "{}", c.code);
        // a line that reads e() after a model typed earlier
        answered(&mut live, "regress price mpg");
        let d = live.step("display e(N)");
        assert!(d.lines.iter().all(|l| l.result.is_ok()), "{:?}", d.lines);
        assert!(d.code.contains("stata_eget(\"N\")"), "{}", d.code);
    }

    #[test]
    fn what_cannot_be_kept_is_answered_and_left_out() {
        let mut live = Live::new();
        live.step("use auto");
        let open = live.step("foreach v of varlist price mpg {");
        assert!(open.lines.iter().any(|l| l.result.is_err()));
        // the open block did not swallow what follows
        let next = answered(&mut live, "summarize price");
        assert_eq!(next.lines.len(), 1, "{:?}", next.lines);
        assert!(next.lines.iter().all(|l| l.result.is_ok()));
    }

    #[test]
    fn a_value_only_r_knows_is_asked_for_on_a_line_of_its_own() {
        let mut live = Live::new();
        live.step("use auto");
        // the data's names first: price may abbreviate a variable
        let names = live.step("summarize price");
        assert!(
            names
                .ask
                .as_deref()
                .is_some_and(|c| c.starts_with("cat(names(")),
            "{names:?}"
        );
        live.give(Ok("make\nprice\nmpg\n".into()));
        assert!(live.step("summarize price").ask.is_none());
        let ask = live.step("local m = r(mean)");
        let code = ask.ask.clone().unwrap_or_default();
        assert!(code.contains("%18.0g") && ask.lines.is_empty(), "{ask:?}");
        // R's answer, then the same line again
        live.give(Ok("  6165.256756756757\n".into()));
        let set = live.step("local m = r(mean)");
        assert!(set.ask.is_none() && set.lines.len() == 1, "{set:?}");
        // price is a name R gave already: no asking again
        let used = live.step("generate d = price - `m'");
        assert!(used.ask.is_none(), "{used:?}");
        assert_eq!(
            used.lines[0].stata,
            "generate d = price - 6165.256756756757"
        );
        assert!(used.lines[0].result.is_ok());
        // asked again later, the answer before is replayed and not asked for
        assert!(live.step("display `m'").ask.is_none());
        // in the middle of several lines R would answer too early: said so
        let both = live.step("summarize mpg\nlocal k = r(mean)\ndisplay `k'");
        assert!(both.ask.is_none());
        let reasons: Vec<String> = both
            .lines
            .iter()
            .filter_map(|l| l.result.as_ref().err().map(|e| e.reason.clone()))
            .collect();
        assert!(
            reasons.iter().any(|r| r.contains("on a line of its own")),
            "{reasons:?}"
        );
    }

    #[test]
    fn macros_and_labels_are_followed() {
        let mut live = Live::new();
        live.step("use auto");
        live.step("local controls mpg weight");
        live.step("global root \"c:/data\"");
        live.step("label define yesno 0 \"no\" 1 \"yes\"");
        assert_eq!(live.locals(), ["controls".to_string()]);
        assert_eq!(live.globals(), ["root".to_string()]);
        assert_eq!(live.value_labels(), ["yesno".to_string()]);
        assert_eq!(helper_name("stata_fmt <- function(x) x"), Some("stata_fmt"));
        // ssc install: the package asked for, once
        assert_eq!(
            live.step("ssc install estout, replace").ssc,
            ["estout".to_string()]
        );
        assert!(answered(&mut live, "summarize price").ssc.is_empty());
    }
}
