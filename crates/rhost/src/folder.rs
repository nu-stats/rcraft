//! The folder R works in, remembered between runs in a one-line text file in
//! RCraft's own folder. It is a convenience, so nothing here fails loudly: a
//! missing, unreadable or stale file means "no remembered folder", and a folder
//! that cannot be saved is used for this run only.

use std::path::{Path, PathBuf};

use crate::find::Env;
use crate::library::app_data_dir;

/// Where the remembered folder is kept on this machine.
pub fn working_folder_file(env: &Env) -> Option<PathBuf> {
    app_data_dir(env).map(|d| d.join("working-folder.txt"))
}

/// The remembered folder, when it is still a folder.
pub fn load(env: &Env) -> Option<PathBuf> {
    let text = std::fs::read_to_string(working_folder_file(env)?).ok()?;
    let line = text.lines().next()?.trim_end_matches('\r').trim();
    if line.is_empty() {
        return None;
    }
    let dir = PathBuf::from(line);
    dir.is_dir().then_some(dir)
}

/// Remembers `dir`. Returns false when it could not be written.
pub fn save(env: &Env, dir: &Path) -> bool {
    let Some(file) = working_folder_file(env) else {
        return false;
    };
    if let Some(parent) = file.parent() {
        if std::fs::create_dir_all(parent).is_err() {
            return false;
        }
    }
    std::fs::write(file, format!("{}\n", dir.display())).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::find::Platform;

    fn env_in(dir: &Path) -> Env {
        Env {
            platform: Some(Platform::Windows),
            local_app_data: Some(dir.to_path_buf()),
            ..Env::default()
        }
    }

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("rcraft-folder-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let _ = std::fs::create_dir_all(&d);
        d
    }

    #[test]
    fn a_folder_is_remembered_and_forgotten_when_it_is_gone() {
        let data = temp("data");
        let work = temp("work");
        let env = env_in(&data);
        assert_eq!(load(&env), None);
        assert!(save(&env, &work));
        assert_eq!(load(&env), Some(work.clone()));
        let _ = std::fs::remove_dir_all(&work);
        assert_eq!(load(&env), None, "a folder that is gone is not offered");
        let _ = std::fs::remove_dir_all(&data);
    }

    #[test]
    fn junk_and_missing_places_mean_no_folder() {
        let data = temp("junk");
        let env = env_in(&data);
        assert!(save(&env, Path::new("")));
        assert_eq!(load(&env), None);
        let none = Env::default();
        assert_eq!(load(&none), None);
        assert!(!save(&none, &data));
        let _ = std::fs::remove_dir_all(&data);
    }
}
