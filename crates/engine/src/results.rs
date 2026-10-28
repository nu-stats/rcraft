//! P2.3: what the Results window shows, one block per command.

use rcraft_rhost::Stream;

use crate::history::{CommandId, Outcome};

/// Text kept across all blocks; the oldest blocks are dropped past this.
pub const MAX_RETAINED_BYTES: usize = 64 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Item {
    /// Printed output or messages. Consecutive pieces from the same stream
    /// are joined, so a long print is one item.
    Text {
        stream: Stream,
        text: String,
    },
    Warning(String),
    Error {
        message: String,
        call: String,
    },
    /// RCraft's own notes: output cut off, a limit reached, R stopped.
    Notice(String),
    /// A graphics page the command drew, as a PNG file kept for the session.
    Plot {
        path: std::path::PathBuf,
    },
}

impl Item {
    fn bytes(&self) -> usize {
        match self {
            Item::Text { text, .. } => text.len(),
            Item::Warning(s) | Item::Notice(s) => s.len(),
            Item::Error { message, call } => message.len() + call.len(),
            // the picture is a file on disk, not text held here
            Item::Plot { path } => path.as_os_str().len(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResultBlock {
    pub command: CommandId,
    pub items: Vec<Item>,
    /// `None` while the command is running.
    pub outcome: Option<Outcome>,
}

#[derive(Clone, Debug, Default)]
pub struct Results {
    blocks: Vec<ResultBlock>,
    bytes: usize,
}

impl Results {
    pub fn new() -> Results {
        Results::default()
    }

    pub fn blocks(&self) -> &[ResultBlock] {
        &self.blocks
    }

    /// Starts the block for a command.
    pub fn begin(&mut self, command: CommandId) {
        self.blocks.push(ResultBlock {
            command,
            items: Vec::new(),
            outcome: None,
        });
    }

    pub fn add(&mut self, command: CommandId, item: Item) {
        let Some(block) = self.blocks.iter_mut().rev().find(|b| b.command == command) else {
            return;
        };
        self.bytes = self.bytes.saturating_add(item.bytes());
        // join a piece of text onto the one before it from the same stream
        if let (Item::Text { stream, text }, Some(Item::Text { stream: s, text: t })) =
            (&item, block.items.last_mut())
        {
            if stream == s {
                t.push_str(text);
                self.trim();
                return;
            }
        }
        block.items.push(item);
        self.trim();
    }

    pub fn finish(&mut self, command: CommandId, outcome: Outcome) {
        if let Some(block) = self.blocks.iter_mut().rev().find(|b| b.command == command) {
            block.outcome = Some(outcome);
        }
    }

    pub fn clear(&mut self) {
        self.blocks.clear();
        self.bytes = 0;
    }

    /// Drops the oldest blocks while over the cap, always keeping the newest.
    fn trim(&mut self) {
        while self.bytes > MAX_RETAINED_BYTES && self.blocks.len() > 1 {
            let dropped = self.blocks.remove(0);
            let freed: usize = dropped.items.iter().map(Item::bytes).sum();
            self.bytes = self.bytes.saturating_sub(freed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(s: &str) -> Item {
        Item::Text {
            stream: Stream::Stdout,
            text: s.into(),
        }
    }

    #[test]
    fn joins_consecutive_text_from_one_stream() {
        let mut r = Results::new();
        r.begin(1);
        r.add(1, text("a\n"));
        r.add(1, text("b\n"));
        r.add(1, Item::Warning("w".into()));
        r.add(1, text("c\n"));
        let items = &r.blocks()[0].items;
        assert_eq!(items.len(), 3);
        assert_eq!(items[0], text("a\nb\n"));
    }

    #[test]
    fn different_streams_stay_apart() {
        let mut r = Results::new();
        r.begin(1);
        r.add(1, text("out\n"));
        r.add(
            1,
            Item::Text {
                stream: Stream::Stderr,
                text: "msg\n".into(),
            },
        );
        assert_eq!(r.blocks()[0].items.len(), 2);
    }

    #[test]
    fn unknown_commands_are_ignored() {
        let mut r = Results::new();
        r.add(7, text("x"));
        r.finish(7, Outcome::Ok);
        assert!(r.blocks().is_empty());
    }

    #[test]
    fn finish_records_the_outcome() {
        let mut r = Results::new();
        r.begin(1);
        assert_eq!(r.blocks()[0].outcome, None);
        r.finish(1, Outcome::Error);
        assert_eq!(r.blocks()[0].outcome, Some(Outcome::Error));
    }

    #[test]
    fn old_blocks_are_dropped_past_the_cap() {
        let mut r = Results::new();
        let chunk = "x".repeat(MAX_RETAINED_BYTES / 2 + 1);
        for id in 1..=3 {
            r.begin(id);
            r.add(id, text(&chunk));
        }
        assert!(r.blocks().len() < 3);
        assert_eq!(r.blocks().last().map(|b| b.command), Some(3));
    }
}
