//! P2.4: the session log and the session script.
//!
//! The **log** is a transcript of the session: each command after a prompt,
//! then what it printed. The **script** is only the code that ran without
//! error, as plain R a user can keep and run elsewhere. Menu actions are in
//! both, as the R they ran.

use std::fs::File;
use std::io::Write;
use std::path::Path;

use crate::history::{Outcome, Source};
use crate::results::Item;

/// The log text kept in memory; older text is dropped past this.
const MAX_LOG_BYTES: usize = 32 * 1024 * 1024;

const PROMPT: &str = "> ";
const CONTINUATION: &str = "+ ";

#[derive(Debug, Default)]
pub struct Transcript {
    log: String,
    /// Commands that ran without error, in order, ready to join.
    script: Vec<String>,
    file: Option<File>,
}

impl Transcript {
    pub fn new() -> Transcript {
        Transcript::default()
    }

    pub fn log_text(&self) -> &str {
        &self.log
    }

    /// The script: a header, then each successful command. A command that
    /// came from a menu is preceded by a comment naming the menu entry.
    pub fn script_text(&self) -> String {
        let mut out = String::from("# Script from an RCraft session\n");
        for part in &self.script {
            out.push('\n');
            out.push_str(part);
            if !part.ends_with('\n') {
                out.push('\n');
            }
        }
        out
    }

    /// Also write the log to `path` from now on (appending), starting with
    /// what has been logged so far. A second call switches files.
    pub fn log_to(&mut self, path: &Path) -> std::io::Result<()> {
        let mut file = File::options().create(true).append(true).open(path)?;
        file.write_all(self.log.as_bytes())?;
        self.file = Some(file);
        Ok(())
    }

    pub fn stop_logging(&mut self) {
        self.file = None;
    }

    pub fn is_logging_to_file(&self) -> bool {
        self.file.is_some()
    }

    /// A command is about to run.
    pub fn command(&mut self, code: &str) {
        let mut text = String::new();
        for (i, line) in code.trim_end().lines().enumerate() {
            text.push_str(if i == 0 { PROMPT } else { CONTINUATION });
            text.push_str(line);
            text.push('\n');
        }
        self.append(&text);
    }

    /// Something the command produced.
    pub fn item(&mut self, item: &Item) {
        let text = match item {
            Item::Text { stream: _, text } => text.clone(),
            Item::Warning(message) => format!("Warning: {message}\n"),
            Item::Error { message, call } if call.is_empty() => format!("Error: {message}\n"),
            Item::Error { message, call } => format!("Error in {call} : {message}\n"),
            Item::Notice(note) => format!("({note})\n"),
        };
        self.append(&text);
    }

    /// A command ended. Keeps it for the script if it ran without error.
    pub fn finished(&mut self, source: &Source, code: &str, outcome: Outcome) {
        if outcome != Outcome::Ok {
            return;
        }
        let mut part = String::new();
        if let Source::Menu(name) = source {
            part.push_str(&format!("# {name}\n"));
        }
        part.push_str(code.trim_end());
        self.script.push(part);
    }

    fn append(&mut self, text: &str) {
        self.log.push_str(text);
        if self.log.len() > MAX_LOG_BYTES {
            let mut cut = self.log.len() - MAX_LOG_BYTES;
            while !self.log.is_char_boundary(cut) {
                cut += 1;
            }
            self.log.drain(..cut);
        }
        if let Some(file) = self.file.as_mut() {
            // a full disk must not stop the session: drop the file and go on
            if file.write_all(text.as_bytes()).is_err() {
                self.file = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rcraft_rhost::Stream;

    #[test]
    fn logs_commands_with_prompts() {
        let mut t = Transcript::new();
        t.command("x <- 1\nprint(x)");
        t.item(&Item::Text {
            stream: Stream::Stdout,
            text: "[1] 1\n".into(),
        });
        assert_eq!(t.log_text(), "> x <- 1\n+ print(x)\n[1] 1\n");
    }

    #[test]
    fn logs_warnings_errors_and_notices() {
        let mut t = Transcript::new();
        t.item(&Item::Warning("careful".into()));
        t.item(&Item::Error {
            message: "boom".into(),
            call: "f(x)".into(),
        });
        t.item(&Item::Error {
            message: "bare".into(),
            call: String::new(),
        });
        t.item(&Item::Notice("output cut off".into()));
        assert_eq!(
            t.log_text(),
            "Warning: careful\nError in f(x) : boom\nError: bare\n(output cut off)\n"
        );
    }

    #[test]
    fn the_script_keeps_only_what_worked() {
        let mut t = Transcript::new();
        t.finished(&Source::Typed, "a <- 1\n", Outcome::Ok);
        t.finished(&Source::Typed, "stop('x')", Outcome::Error);
        t.finished(&Source::Menu("Histogram".into()), "hist(a)", Outcome::Ok);
        t.finished(&Source::Typed, "Sys.sleep(9)", Outcome::Interrupted);
        let script = t.script_text();
        assert!(script.contains("a <- 1\n"));
        assert!(!script.contains("stop"));
        assert!(!script.contains("Sys.sleep"));
        assert!(script.contains("# Histogram\nhist(a)\n"));
    }

    #[test]
    fn an_empty_session_has_a_header_only() {
        assert_eq!(
            Transcript::new().script_text(),
            "# Script from an RCraft session\n"
        );
    }

    #[test]
    fn the_log_is_capped_on_a_character_boundary() {
        let mut t = Transcript::new();
        let piece = "é".repeat(1024 * 1024);
        for _ in 0..40 {
            t.item(&Item::Notice(piece.clone()));
        }
        assert!(t.log_text().len() <= MAX_LOG_BYTES);
        assert!(t.log_text().is_char_boundary(0));
    }

    #[test]
    fn writes_the_log_to_a_file() {
        let dir = std::env::temp_dir().join(format!("rcraft-log-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap_or_default();
        let path = dir.join("session.log");
        let mut t = Transcript::new();
        t.command("before");
        t.log_to(&path).unwrap_or_default();
        t.command("after");
        t.stop_logging();
        t.command("not written");
        let text = std::fs::read_to_string(&path).unwrap_or_default();
        assert_eq!(text, "> before\n> after\n");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
