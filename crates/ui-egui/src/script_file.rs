//! The Script Editor's file (P7.3): where the script was opened from or
//! saved to, and the text as it was then, so the pane can say when there are
//! changes not saved and the window can ask before they would be lost.
//!
//! A file is read as UTF-8 (a byte-order mark dropped, bytes that are not
//! UTF-8 replaced) with its line endings made `\n` for the editor; it is
//! written back with the line endings it had (`\r\n` for a Windows file).

use std::path::{Path, PathBuf};

/// Largest script read into the editor.
const MAX_BYTES: u64 = 20 * 1024 * 1024;

/// The script's file, if it has one.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScriptFile {
    path: Option<PathBuf>,
    /// The text when it was last opened or saved.
    saved: String,
    /// The file used `\r\n`.
    crlf: bool,
}

/// A script read from disk: the text for the editor, and the file it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Opened {
    pub text: String,
    pub file: ScriptFile,
}

impl ScriptFile {
    /// A new, unnamed script whose untouched text is `start`.
    pub fn unnamed(start: &str) -> ScriptFile {
        ScriptFile {
            path: None,
            saved: start.to_string(),
            crlf: false,
        }
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Whether `text` differs from the text last opened or saved.
    pub fn is_changed(&self, text: &str) -> bool {
        text != self.saved
    }

    /// What the pane's title says: the file's name (or `Script`), with a dot
    /// while there are changes not saved.
    pub fn title(&self, text: &str) -> String {
        let name = self.path.as_ref().and_then(|p| p.file_name()).map_or_else(
            || "Script".to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        if self.path.is_some() && self.is_changed(text) {
            format!("{name} •")
        } else {
            name
        }
    }

    /// Whether the file is a Stata do-file (it opens with Stata mode on).
    pub fn is_stata(&self) -> bool {
        self.path.as_deref().is_some_and(is_stata_path)
    }

    /// Reads `path` for the editor.
    pub fn open(path: &Path) -> Result<Opened, String> {
        let size = std::fs::metadata(path)
            .map_err(|e| format!("{}: {e}", path.display()))?
            .len();
        if size > MAX_BYTES {
            return Err(format!(
                "{} is {} MB; scripts up to {} MB open in the editor.",
                path.display(),
                size / (1024 * 1024),
                MAX_BYTES / (1024 * 1024)
            ));
        }
        let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(&bytes);
        let raw = String::from_utf8_lossy(bytes);
        let crlf = raw.contains("\r\n");
        let text = raw.replace("\r\n", "\n");
        Ok(Opened {
            file: ScriptFile {
                path: Some(path.to_path_buf()),
                saved: text.clone(),
                crlf,
            },
            text,
        })
    }

    /// Writes `text` to `path` (the file's own line endings kept, `\n` for a
    /// new one), and from then on this is the script's file.
    pub fn save(&mut self, path: &Path, text: &str) -> Result<(), String> {
        let crlf = if self.path.as_deref() == Some(path) {
            self.crlf
        } else {
            std::fs::read(path)
                .map(|b| b.windows(2).any(|w| w == b"\r\n"))
                .unwrap_or(false)
        };
        let body = if crlf {
            text.replace("\r\n", "\n").replace('\n', "\r\n")
        } else {
            text.to_string()
        };
        std::fs::write(path, body).map_err(|e| format!("{}: {e}", path.display()))?;
        self.path = Some(path.to_path_buf());
        self.saved = text.to_string();
        self.crlf = crlf;
        Ok(())
    }
}

/// Whether `path` names a Stata do-file.
pub fn is_stata_path(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| matches!(e.to_ascii_lowercase().as_str(), "do" | "doh" | "ado"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("rcraft-script-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        dir.join(name)
    }

    #[test]
    fn a_windows_file_with_a_mark_opens_clean_and_saves_as_it_was() {
        let path = temp("win.R");
        let bytes = b"\xEF\xBB\xBFx <- 1\r\ny <- 2\r\n";
        assert!(std::fs::write(&path, bytes).is_ok());
        let opened = ScriptFile::open(&path).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(opened.text, "x <- 1\ny <- 2\n");
        let mut file = opened.file;
        assert!(!file.is_changed("x <- 1\ny <- 2\n"));
        assert_eq!(file.title("x <- 1\ny <- 2\n"), "win.R");
        assert_eq!(file.title("x <- 3\n"), "win.R •");
        assert!(file.save(&path, "x <- 3\nz <- 4\n").is_ok());
        assert_eq!(
            std::fs::read(&path).unwrap_or_default(),
            b"x <- 3\r\nz <- 4\r\n".to_vec()
        );
        assert!(!file.is_changed("x <- 3\nz <- 4\n"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_new_script_saves_with_newlines_and_takes_the_name() {
        let path = temp("new.R");
        let _ = std::fs::remove_file(&path);
        let mut file = ScriptFile::unnamed("# start\n");
        assert_eq!(file.title("# start\n"), "Script");
        assert_eq!(file.title("changed"), "Script");
        assert!(file.is_changed("changed"));
        assert!(file.save(&path, "a <- 1\n").is_ok());
        assert_eq!(
            std::fs::read(&path).unwrap_or_default(),
            b"a <- 1\n".to_vec()
        );
        assert_eq!(file.path(), Some(path.as_path()));
        assert_eq!(file.title("a <- 1\n"), "new.R");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn odd_bytes_missing_files_and_do_files() {
        let path = temp("odd.R");
        assert!(std::fs::write(&path, b"x <- \"\xE9\"\n").is_ok());
        let opened = ScriptFile::open(&path).unwrap_or_else(|e| panic!("{e}"));
        assert!(opened.text.starts_with("x <- \""), "{}", opened.text);
        assert!(ScriptFile::open(&temp("no-such-file.R")).is_err());
        assert!(is_stata_path(Path::new("analysis.DO")));
        assert!(!is_stata_path(Path::new("analysis.R")));
        let _ = std::fs::remove_file(&path);
    }
}
