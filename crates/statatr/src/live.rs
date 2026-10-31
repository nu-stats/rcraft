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
//! Each step translates everything again, which keeps one translator and no
//! second copy of its state: a do-file is small next to what R does with it.
//! Text the reader cannot take as commands of its own (a block left open, a
//! line it cannot read) is answered but not kept, so it cannot swallow the
//! lines typed after it.

use std::collections::BTreeSet;

use crate::data_r::{translate, Line};
use crate::reader::{read, Item};

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
}

impl Live {
    pub fn new() -> Live {
        Live::default()
    }

    /// Translates `text` after what came before, and keeps it.
    pub fn step(&mut self, text: &str) -> Step {
        let own = read(text);
        let keep = own.items.iter().all(|i| matches!(i, Item::Command(_)));
        if !keep {
            // a block or a line the reader cannot take: answered on its own
            let t = translate(&own);
            let code = t.lines.iter().map(Line::script).collect();
            return Step {
                lines: t.lines,
                code,
            };
        }
        let mut whole = self.source.clone();
        whole.push_str(text.trim_end_matches(['\n', '\r']));
        whole.push('\n');
        let program = read(&whole);
        let t = translate(&program);
        let lines: Vec<Line> = t.lines.get(self.done..).unwrap_or_default().to_vec();
        let mut code = String::new();
        let new: Vec<&String> = t
            .helpers
            .iter()
            .filter(|h| !self.sent.contains(*h))
            .collect();
        if !new.is_empty() {
            let names: Vec<&str> = new.iter().filter_map(|h| helper_name(h)).collect();
            code.push_str(&format!("{HELPERS_BEGIN} {}\n", names.join(", ")));
            for h in &new {
                code.push_str(h);
                code.push('\n');
            }
            code.push_str(HELPERS_END);
            code.push('\n');
        }
        for l in &lines {
            code.push_str(&l.script());
        }
        for h in new {
            self.sent.insert(h.clone());
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
        Step { lines, code }
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

    #[test]
    fn each_line_means_what_it_does_after_the_ones_before() {
        let mut live = Live::new();
        let a = live.step("use auto");
        assert_eq!(a.lines.len(), 1);
        assert_eq!(live.data(), Some("auto"));
        let b = live.step("summarize price");
        // the helper comes once, before the line that first needs it
        assert!(b.code.starts_with(HELPERS_BEGIN), "{}", b.code);
        assert!(b.code.contains("stata_summarize <- function"));
        assert!(b.code.contains(HELPERS_END));
        assert!(b.code.contains("# summarize price\n"));
        let c = live.step("summarize mpg");
        assert!(!c.code.contains(HELPERS_BEGIN), "{}", c.code);
        assert!(c.code.starts_with("# summarize mpg\n"), "{}", c.code);
        // a line that reads e() after a model typed earlier
        live.step("regress price mpg");
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
        let next = live.step("summarize price");
        assert_eq!(next.lines.len(), 1, "{:?}", next.lines);
        assert!(next.lines.iter().all(|l| l.result.is_ok()));
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
    }
}
