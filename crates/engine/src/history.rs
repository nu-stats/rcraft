//! P2.1: the command history and how the Command window steps through it.
//!
//! Page Up and the up arrow go back through earlier commands, Page Down and
//! the down arrow go forward again, and clicking an entry in the History pane
//! puts it back in the Command window. The text being typed when the user
//! first goes back is kept and returned to when they come forward past the
//! newest entry.

/// Entries kept; older ones are dropped.
pub const MAX_ENTRIES: usize = 10_000;

pub type CommandId = u64;

/// Where a command came from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Source {
    /// Typed in the Command window.
    Typed,
    /// A menu or dialog entry; carries its catalog name.
    Menu(String),
    /// Part of a script run from the Script Editor.
    Script,
}

/// How a command ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Ok,
    Error,
    Interrupted,
    Crashed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistoryEntry {
    pub id: CommandId,
    pub source: Source,
    pub code: String,
    /// `None` while the command is still running.
    pub outcome: Option<Outcome>,
}

#[derive(Clone, Debug, Default)]
pub struct History {
    entries: Vec<HistoryEntry>,
    next_id: CommandId,
}

impl History {
    pub fn new() -> History {
        History::default()
    }

    /// Adds a command and returns its id. Ids are never reused.
    pub fn push(&mut self, source: Source, code: &str) -> CommandId {
        self.next_id += 1;
        let id = self.next_id;
        self.entries.push(HistoryEntry {
            id,
            source,
            code: code.to_string(),
            outcome: None,
        });
        if self.entries.len() > MAX_ENTRIES {
            self.entries.remove(0);
        }
        id
    }

    pub fn set_outcome(&mut self, id: CommandId, outcome: Outcome) {
        if let Some(entry) = self.entries.iter_mut().rev().find(|e| e.id == id) {
            entry.outcome = Some(outcome);
        }
    }

    pub fn entries(&self) -> &[HistoryEntry] {
        &self.entries
    }

    pub fn get(&self, id: CommandId) -> Option<&HistoryEntry> {
        self.entries.iter().rev().find(|e| e.id == id)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The commands worth recalling: not blank, and with runs of the same
    /// command collapsed to one.
    fn recallable(&self) -> Vec<&str> {
        let mut out: Vec<&str> = Vec::new();
        for entry in &self.entries {
            let code = entry.code.trim_end();
            if code.trim().is_empty() || out.last() == Some(&code) {
                continue;
            }
            out.push(code);
        }
        out
    }
}

/// Where the user is while stepping through the history.
#[derive(Clone, Debug, Default)]
pub struct Recall {
    /// Index into the recallable list; `None` when not stepping.
    index: Option<usize>,
    /// What was in the box when stepping began.
    draft: String,
}

impl Recall {
    pub fn new() -> Recall {
        Recall::default()
    }

    /// Call when the user runs a command or edits the text by hand, so the
    /// next step back starts from the newest entry.
    pub fn reset(&mut self) {
        self.index = None;
        self.draft.clear();
    }

    pub fn is_stepping(&self) -> bool {
        self.index.is_some()
    }

    /// Page Up or the up arrow. `current` is what is in the box now. Returns
    /// the text to put there, or `None` if there is nothing earlier.
    pub fn previous(&mut self, history: &History, current: &str) -> Option<String> {
        let list = history.recallable();
        if list.is_empty() {
            return None;
        }
        let index = match self.index {
            None => {
                self.draft = current.to_string();
                list.len() - 1
            }
            Some(i) => i.saturating_sub(1).min(list.len() - 1),
        };
        self.index = Some(index);
        list.get(index).map(|s| (*s).to_string())
    }

    /// Page Down or the down arrow. Past the newest entry the text that was
    /// being typed comes back.
    pub fn next(&mut self, history: &History) -> Option<String> {
        let list = history.recallable();
        let i = self.index?;
        if i + 1 >= list.len() {
            self.index = None;
            return Some(std::mem::take(&mut self.draft));
        }
        self.index = Some(i + 1);
        list.get(i + 1).map(|s| (*s).to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn history(codes: &[&str]) -> History {
        let mut h = History::new();
        for c in codes {
            h.push(Source::Typed, c);
        }
        h
    }

    #[test]
    fn ids_are_unique_and_outcomes_are_recorded() {
        let mut h = History::new();
        let a = h.push(Source::Typed, "1");
        let b = h.push(Source::Menu("Histogram".into()), "hist(x)");
        assert_ne!(a, b);
        assert_eq!(h.get(a).and_then(|e| e.outcome), None);
        h.set_outcome(a, Outcome::Ok);
        assert_eq!(h.get(a).and_then(|e| e.outcome), Some(Outcome::Ok));
        h.set_outcome(999, Outcome::Error); // unknown id: nothing happens
        assert_eq!(h.len(), 2);
    }

    #[test]
    fn stepping_back_and_forward_restores_the_draft() {
        let h = history(&["a", "b", "c"]);
        let mut r = Recall::new();
        assert_eq!(r.previous(&h, "half typed"), Some("c".into()));
        assert_eq!(r.previous(&h, "ignored"), Some("b".into()));
        assert_eq!(r.previous(&h, ""), Some("a".into()));
        // at the oldest entry, going back again stays there
        assert_eq!(r.previous(&h, ""), Some("a".into()));
        assert_eq!(r.next(&h), Some("b".into()));
        assert_eq!(r.next(&h), Some("c".into()));
        assert_eq!(r.next(&h), Some("half typed".into()));
        assert!(!r.is_stepping());
        assert_eq!(r.next(&h), None);
    }

    #[test]
    fn blank_and_repeated_commands_are_skipped() {
        let h = history(&["a", "a", "  ", "", "b", "b", "a"]);
        let mut r = Recall::new();
        let got: Vec<String> = std::iter::from_fn(|| r.previous(&h, "")).take(3).collect();
        assert_eq!(got, ["a", "b", "a"]);
    }

    #[test]
    fn an_empty_history_has_nothing_to_recall() {
        let h = History::new();
        let mut r = Recall::new();
        assert_eq!(r.previous(&h, "x"), None);
        assert!(!r.is_stepping());
    }

    #[test]
    fn reset_starts_again_from_the_newest() {
        let h = history(&["a", "b"]);
        let mut r = Recall::new();
        r.previous(&h, "");
        r.previous(&h, "");
        r.reset();
        assert_eq!(r.previous(&h, ""), Some("b".into()));
    }

    #[test]
    fn history_is_capped() {
        let mut h = History::new();
        for i in 0..(MAX_ENTRIES + 5) {
            h.push(Source::Typed, &i.to_string());
        }
        assert_eq!(h.len(), MAX_ENTRIES);
        assert_eq!(h.entries().first().map(|e| e.code.as_str()), Some("5"));
    }
}
