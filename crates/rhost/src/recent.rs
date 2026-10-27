//! The files the user opened lately, newest first, kept between runs in a text
//! file (one path per line) in RCraft's own folder. Keeping the list is a
//! convenience, so nothing here fails loudly: a missing or unreadable file is an
//! empty list, and a list that cannot be saved is kept for this run only.

use std::path::PathBuf;

use crate::find::Env;
use crate::library::app_data_dir;

/// How many files are remembered.
pub const MAX_RECENT: usize = 10;

/// Where the list is kept on this machine.
pub fn recent_file(env: &Env) -> Option<PathBuf> {
    app_data_dir(env).map(|d| d.join("recent-files.txt"))
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Recent {
    paths: Vec<String>,
    file: Option<PathBuf>,
}

impl Recent {
    /// The list kept in `file` (empty when there is none yet). With `None` the
    /// list lives in memory only.
    pub fn at(file: Option<PathBuf>) -> Recent {
        let mut recent = Recent {
            paths: Vec::new(),
            file,
        };
        if let Some(text) = recent
            .file
            .as_ref()
            .and_then(|f| std::fs::read_to_string(f).ok())
        {
            for line in text.lines() {
                recent.push_back(line.trim_end_matches('\r'));
            }
        }
        recent
    }

    /// The list for this machine.
    pub fn load(env: &Env) -> Recent {
        Recent::at(recent_file(env))
    }

    /// Newest first.
    pub fn paths(&self) -> &[String] {
        &self.paths
    }

    pub fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }

    /// Puts `path` first (moving it up if it was there) and saves.
    pub fn add(&mut self, path: &str) {
        let path = path.trim();
        if path.is_empty() || path.contains(['\n', '\r']) {
            return;
        }
        self.paths.retain(|p| p != path);
        self.paths.insert(0, path.to_string());
        self.paths.truncate(MAX_RECENT);
        self.save();
    }

    /// Takes `path` off the list (a file that is gone) and saves.
    pub fn forget(&mut self, path: &str) {
        let before = self.paths.len();
        self.paths.retain(|p| p != path);
        if self.paths.len() != before {
            self.save();
        }
    }

    pub fn clear(&mut self) {
        self.paths.clear();
        self.save();
    }

    /// Adds a line read from the file, at the end, unless it is empty, a
    /// repeat, or past the limit.
    fn push_back(&mut self, line: &str) {
        let line = line.trim();
        if !line.is_empty()
            && self.paths.len() < MAX_RECENT
            && !self.paths.iter().any(|p| p == line)
        {
            self.paths.push(line.to_string());
        }
    }

    fn save(&self) {
        let Some(file) = &self.file else { return };
        if let Some(dir) = file.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let mut text = self.paths.join("\n");
        text.push('\n');
        // write beside it and rename, so a crash cannot leave half a list
        let temp = file.with_extension("tmp");
        if std::fs::write(&temp, text).is_ok() && std::fs::rename(&temp, file).is_err() {
            let _ = std::fs::remove_file(&temp);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::find::Platform;

    fn temp_file(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("rcraft-recent-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir.join("recent-files.txt")
    }

    #[test]
    fn newest_first_without_repeats_and_capped() {
        let mut r = Recent::at(None);
        for i in 0..15 {
            r.add(&format!("/data/f{i}.csv"));
        }
        assert_eq!(r.paths().len(), MAX_RECENT);
        assert_eq!(r.paths().first().map(String::as_str), Some("/data/f14.csv"));
        r.add("/data/f10.csv");
        assert_eq!(r.paths().first().map(String::as_str), Some("/data/f10.csv"));
        assert_eq!(
            r.paths().iter().filter(|p| *p == "/data/f10.csv").count(),
            1
        );
        assert_eq!(r.paths().len(), MAX_RECENT);
    }

    #[test]
    fn bad_paths_are_ignored() {
        let mut r = Recent::at(None);
        r.add("  ");
        r.add("a\nb");
        assert!(r.is_empty());
        r.add("  /data/a.csv  ");
        assert_eq!(r.paths(), ["/data/a.csv"]);
    }

    #[test]
    fn the_list_survives_a_restart_and_creates_its_folder() {
        let file = temp_file("survives");
        let mut r = Recent::at(Some(file.clone()));
        assert!(r.is_empty(), "no file yet is an empty list");
        r.add("/data/a.csv");
        r.add("C:\\data\\b é.xlsx");
        assert!(file.is_file());
        let again = Recent::at(Some(file.clone()));
        assert_eq!(again.paths(), ["C:\\data\\b é.xlsx", "/data/a.csv"]);
        let mut again = again;
        again.forget("/data/a.csv");
        again.forget("/not/there");
        assert_eq!(
            Recent::at(Some(file.clone())).paths(),
            ["C:\\data\\b é.xlsx"]
        );
        again.clear();
        assert!(Recent::at(Some(file.clone())).is_empty());
        if let Some(dir) = file.parent() {
            let _ = std::fs::remove_dir_all(dir);
        }
    }

    #[test]
    fn a_damaged_file_is_read_as_far_as_it_goes() {
        let file = temp_file("damaged");
        if let Some(dir) = file.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let mut text = String::from("\n/a\r\n  \n/a\n/b\n");
        for i in 0..20 {
            text.push_str(&format!("/x{i}\n"));
        }
        let _ = std::fs::write(&file, text);
        let r = Recent::at(Some(file.clone()));
        assert_eq!(r.paths().len(), MAX_RECENT);
        assert_eq!(&r.paths()[..3], ["/a", "/b", "/x0"]);
        // bytes that are not text give an empty list, not a crash
        let _ = std::fs::write(&file, [0xff, 0xfe, 0x00]);
        assert!(Recent::at(Some(file.clone())).is_empty());
        if let Some(dir) = file.parent() {
            let _ = std::fs::remove_dir_all(dir);
        }
    }

    #[test]
    fn it_lives_in_rcrafts_folder() {
        let env = Env {
            platform: Some(Platform::Linux),
            home: Some(PathBuf::from("/home/u")),
            ..Env::default()
        };
        assert!(recent_file(&env).is_some_and(|p| p.ends_with("rcraft/recent-files.txt")));
        let none = Env {
            platform: Some(Platform::Windows),
            ..Env::default()
        };
        assert_eq!(recent_file(&none), None);
    }
}
