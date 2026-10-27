//! RCraft's own R package library: where packages that RCraft installs go,
//! kept apart from the packages of any other app's R (STATUS.md: never
//! install into another app's environment).

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use crate::find::{Env, Platform};

/// RCraft's own folder for what it keeps for the user (packages, the list of
/// recent files), for this machine. It may not exist yet; nothing is created
/// here.
pub fn app_data_dir(env: &Env) -> Option<PathBuf> {
    match env.platform.unwrap_or_else(Platform::current) {
        Platform::Windows => env.local_app_data.as_ref().map(|d| d.join("RCraft")),
        Platform::MacOs => env
            .home
            .as_ref()
            .map(|h| h.join("Library").join("Application Support").join("RCraft")),
        Platform::Linux => env
            .home
            .as_ref()
            .map(|h| h.join(".local").join("share").join("rcraft")),
    }
}

/// The folder for packages RCraft installs, for this machine. It may not
/// exist yet; nothing is created here.
pub fn user_library_dir(env: &Env) -> Option<PathBuf> {
    app_data_dir(env).map(|d| d.join("R-library"))
}

/// The value for `R_LIBS_USER`: RCraft's library first, then whatever the
/// environment already had. R adds the folders that exist to its search path
/// when it starts.
pub fn libs_user_value(existing: Option<&OsStr>, library: &Path) -> Option<OsString> {
    let rest = existing
        .map(std::env::split_paths)
        .into_iter()
        .flatten()
        .filter(|p| !p.as_os_str().is_empty());
    std::env::join_paths(std::iter::once(library.to_path_buf()).chain(rest)).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(platform: Platform) -> Env {
        Env {
            platform: Some(platform),
            local_app_data: Some(PathBuf::from("/lad")),
            home: Some(PathBuf::from("/home/u")),
            ..Env::default()
        }
    }

    #[test]
    fn each_system_has_its_usual_place() {
        assert_eq!(
            user_library_dir(&env(Platform::Windows)),
            Some(PathBuf::from("/lad").join("RCraft").join("R-library"))
        );
        assert!(user_library_dir(&env(Platform::MacOs))
            .is_some_and(|p| p.ends_with("Application Support/RCraft/R-library")));
        assert!(user_library_dir(&env(Platform::Linux))
            .is_some_and(|p| p.ends_with(".local/share/rcraft/R-library")));
        let none = Env {
            platform: Some(Platform::Windows),
            ..Env::default()
        };
        assert_eq!(user_library_dir(&none), None);
    }

    #[test]
    fn the_library_goes_first_and_the_old_value_is_kept() {
        let lib = Path::new("/rc/lib");
        let one = libs_user_value(None, lib);
        assert_eq!(one, Some(OsString::from("/rc/lib")));
        let joined = std::env::join_paths(["/other"]).unwrap_or_default();
        let both = libs_user_value(Some(&joined), lib).unwrap_or_default();
        let parts: Vec<PathBuf> = std::env::split_paths(&both).collect();
        assert_eq!(
            parts,
            vec![PathBuf::from("/rc/lib"), PathBuf::from("/other")]
        );
        assert_eq!(libs_user_value(Some(OsStr::new("")), lib), one);
    }
}
