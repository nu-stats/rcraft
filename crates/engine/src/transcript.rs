//! P2.4: the session log and the session script.
//!
//! The **log** is a transcript of the session: each command after a prompt,
//! then what it printed. The **script** is only the code that ran without
//! error, as plain R a user can keep and run elsewhere. Menu actions are in
//! both, as the R they ran. P7.4: the same commands as an R Markdown or
//! Quarto document, one chunk each, with the menu entry above its chunk.

use std::fs::File;
use std::io::Write;
use std::path::Path;

use crate::history::{Outcome, Source};
use crate::results::Item;

/// The log text kept in memory; older text is dropped past this.
const MAX_LOG_BYTES: usize = 32 * 1024 * 1024;

const PROMPT: &str = "> ";
const CONTINUATION: &str = "+ ";

/// A document the session's commands can be written as (P7.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocKind {
    /// `.qmd`, rendered by Quarto.
    Quarto,
    /// `.Rmd`, rendered by rmarkdown.
    RMarkdown,
}

#[derive(Debug, Default)]
pub struct Transcript {
    log: String,
    /// Commands that ran without error, in order: the menu entry each came
    /// from, if one, and its code.
    script: Vec<(Option<String>, String)>,
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
        for (menu, code) in &self.script {
            out.push('\n');
            if let Some(menu) = menu {
                out.push_str(&format!("# {menu}\n"));
            }
            out.push_str(code);
            if !code.ends_with('\n') {
                out.push('\n');
            }
        }
        out
    }

    /// The commands that ran without error as a document: a header with
    /// `title`, then each command in an R chunk, a menu entry's name as a line
    /// of text above its chunk. Rendering it runs the session again.
    pub fn document_text(&self, kind: DocKind, title: &str) -> String {
        let title = title.replace('\\', "\\\\").replace('"', "\\\"");
        let mut out = match kind {
            DocKind::Quarto => format!("---\ntitle: \"{title}\"\nformat: html\n---\n"),
            DocKind::RMarkdown => format!("---\ntitle: \"{title}\"\noutput: html_document\n---\n"),
        };
        out.push_str("\nFrom an RCraft session: the commands that ran without error, in order.\n");
        for (menu, code) in &self.script {
            out.push('\n');
            if let Some(menu) = menu {
                out.push_str(&format!("*{menu}*\n\n"));
            }
            // a chunk's fence must be longer than any run of backticks inside it
            let longest = code.split(|c| c != '`').map(str::len).max().unwrap_or(0);
            let fence = "`".repeat(longest.max(2) + 1);
            out.push_str(&format!("{fence}{{r}}\n{}\n{fence}\n", code.trim_end()));
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
            Item::Text { text, .. } => text.clone(),
            Item::Warning(message) => format!("Warning: {message}\n"),
            Item::Error { message, call } if call.is_empty() => format!("Error: {message}\n"),
            Item::Error { message, call } => format!("Error in {call} : {message}\n"),
            Item::Notice(note) => format!("({note})\n"),
            // a picture has no text; the log says one was drawn
            Item::Plot { .. } => "(a plot was drawn; see the Graph tab)\n".to_string(),
        };
        self.append(&text);
    }

    /// A command ended. Keeps it for the script if it ran without error.
    pub fn finished(&mut self, source: &Source, code: &str, outcome: Outcome) {
        if outcome != Outcome::Ok {
            return;
        }
        let menu = match source {
            Source::Menu(name) => Some(name.clone()),
            _ => None,
        };
        self.script.push((menu, code.trim_end().to_string()));
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
            table: None,
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
    fn the_session_as_quarto_and_r_markdown_has_a_chunk_per_command() {
        let mut t = Transcript::new();
        t.finished(&Source::Typed, "a <- 1\n", Outcome::Ok);
        t.finished(&Source::Typed, "stop('x')", Outcome::Error);
        t.finished(
            &Source::Menu("Graphs > Histogram".into()),
            "hist(a)",
            Outcome::Ok,
        );
        // a Stata line's comment is code, not a menu name
        t.finished(
            &Source::Typed,
            "# Stata: keep if x > 5\nd <- d[d$x > 5, ]",
            Outcome::Ok,
        );
        t.finished(&Source::Typed, "x <- \"```\"", Outcome::Ok);
        let q = t.document_text(DocKind::Quarto, "My \"study\"");
        assert!(
            q.starts_with("---\ntitle: \"My \\\"study\\\"\"\nformat: html\n---\n"),
            "{q}"
        );
        assert!(q.contains("\n```{r}\na <- 1\n```\n"), "{q}");
        assert!(
            q.contains("\n*Graphs > Histogram*\n\n```{r}\nhist(a)\n```\n"),
            "{q}"
        );
        assert!(
            q.contains("```{r}\n# Stata: keep if x > 5\nd <- d[d$x > 5, ]\n```"),
            "{q}"
        );
        // a longer fence round code that holds three backticks
        assert!(q.contains("````{r}\nx <- \"```\"\n````\n"), "{q}");
        assert!(!q.contains("stop"), "{q}");
        let r = t.document_text(DocKind::RMarkdown, "Session");
        assert!(
            r.starts_with("---\ntitle: \"Session\"\noutput: html_document\n---\n"),
            "{r}"
        );
        assert_eq!(r.matches("{r}").count(), 4, "{r}");
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
