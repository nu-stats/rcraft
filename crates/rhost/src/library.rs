//! RCraft's own R package library: where packages that RCraft installs go,
//! kept apart from the packages of any other app's R (STATUS.md: never
//! install into another app's environment).
//!
//! Packages built for one R minor version may not load in another (R keeps
//! its own user libraries per minor version, `win-library/4.4`, for the same
//! reason), so the library has one folder per minor version:
//! `<app data>/RCraft/R-library/4.4`. A session on R 4.4.x sees only that one.
//!
//! RCraft used to keep every package straight in `R-library`. Those packages
//! are moved, one by one, into the folder of the R version that built them
//! (the `Built:` line of each package's DESCRIPTION) the next time a session
//! starts. Nothing is deleted: a package whose version cannot be read, or
//! whose place in the new folder is taken, or that cannot be moved (Windows
//! holds the files of a package a running R has loaded), stays where it was.

use std::ffi::{OsStr, OsString};
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::find::{Env, Platform, RVersion};

/// The name of the folder that holds the per-version libraries.
const LIBRARY_FOLDER: &str = "R-library";
/// At most this many entries of the old flat library are looked at.
const MAX_FLAT_ENTRIES: usize = 20_000;
/// At most this much of a DESCRIPTION file is read.
const MAX_DESCRIPTION_BYTES: u64 = 256 * 1024;

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

/// The folder that holds one package library per R minor version. It may
/// not exist yet; nothing is created here.
pub fn library_root(env: &Env) -> Option<PathBuf> {
    app_data_dir(env).map(|d| d.join(LIBRARY_FOLDER))
}

/// The name of the library folder for an R version: `4.4` for R 4.4.3.
pub fn version_folder(version: RVersion) -> String {
    format!("{}.{}", version.major, version.minor)
}

/// The folder for packages RCraft installs for R `version` (any patch
/// release of the same minor version shares it). It may not exist yet;
/// nothing is created here.
pub fn user_library_dir(env: &Env, version: RVersion) -> Option<PathBuf> {
    library_root(env).map(|d| d.join(version_folder(version)))
}

/// Gets RCraft's library at `library` ready for a session: when it is one of
/// RCraft's per-version folders, packages left in the old flat library are
/// sorted into their versions' folders and the folder is created, so that R
/// has it on its search path from the start and a package installed during
/// the session loads without a restart. Any other folder is left as it is.
/// Failures are ignored: at worst R starts without RCraft's packages.
pub fn prepare(library: &Path) {
    let Some(root) = library.parent() else {
        return;
    };
    let ours = root.file_name() == Some(OsStr::new(LIBRARY_FOLDER))
        && library
            .file_name()
            .and_then(OsStr::to_str)
            .is_some_and(is_version_folder);
    if !ours {
        return;
    }
    sort_flat_library(root);
    let _ = std::fs::create_dir_all(library);
}

/// What [`sort_flat_library`] did.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Sorted {
    /// Packages moved, with the folder they went to (`("dplyr", "4.6")`).
    pub moved: Vec<(String, String)>,
    /// Packages left in the flat library, and why.
    pub left: Vec<(String, &'static str)>,
}

/// Moves each package kept straight in `root` (RCraft's old flat library)
/// into `root/<major.minor>`, the folder of the R version that built it.
/// Never deletes and never overwrites.
pub fn sort_flat_library(root: &Path) -> Sorted {
    let mut sorted = Sorted::default();
    let Ok(entries) = std::fs::read_dir(root) else {
        return sorted;
    };
    for entry in entries.flatten().take(MAX_FLAT_ENTRIES) {
        let path = entry.path();
        let Some(name) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        if is_version_folder(&name) || !path.is_dir() {
            continue;
        }
        let Some(description) = read_capped(&path.join("DESCRIPTION")) else {
            // not a package (a lock folder left by a failed install, say)
            continue;
        };
        let Some(built) = built_version(&description) else {
            sorted
                .left
                .push((name, "its DESCRIPTION does not say which R built it"));
            continue;
        };
        let folder = version_folder(built);
        let target_dir = root.join(&folder);
        let target = target_dir.join(&name);
        if target.exists() {
            sorted.left.push((name, "the new folder already has it"));
            continue;
        }
        if std::fs::create_dir_all(&target_dir).is_err() {
            sorted.left.push((name, "the new folder could not be made"));
            continue;
        }
        match std::fs::rename(&path, &target) {
            Ok(()) => sorted.moved.push((name, folder)),
            Err(_) => sorted.left.push((name, "it could not be moved (in use?)")),
        }
    }
    sorted
}

/// A folder name like `4.4`: a per-version library, not a package.
fn is_version_folder(name: &str) -> bool {
    let mut parts = name.split('.');
    let digits = |p: Option<&str>| {
        p.is_some_and(|p| !p.is_empty() && p.len() <= 4 && p.bytes().all(|b| b.is_ascii_digit()))
    };
    digits(parts.next()) && digits(parts.next()) && parts.next().is_none()
}

/// The R version on a DESCRIPTION's `Built:` line (`Built: R 4.6.1; ...`).
fn built_version(description: &str) -> Option<RVersion> {
    let line = description
        .lines()
        .find_map(|l| l.strip_prefix("Built:"))?
        .trim_start();
    let rest = line.strip_prefix("R ")?;
    RVersion::parse(rest.split(';').next()?)
}

fn read_capped(path: &Path) -> Option<String> {
    let file = std::fs::File::open(path).ok()?;
    let mut bytes = Vec::new();
    file.take(MAX_DESCRIPTION_BYTES)
        .read_to_end(&mut bytes)
        .ok()?;
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

/// The value for `R_LIBS`: RCraft's library first, then whatever the
/// environment already had. R searches these folders, where they exist, before
/// the user's own library (`R_LIBS_USER`), which is left to R so that packages
/// the user installed the usual way are still found.
pub fn libs_value(existing: Option<&OsStr>, library: &Path) -> Option<OsString> {
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

    fn v(text: &str) -> RVersion {
        RVersion::parse(text).unwrap_or(RVersion {
            major: 0,
            minor: 0,
            patch: 0,
        })
    }

    #[test]
    fn each_system_has_its_usual_place() {
        assert_eq!(
            user_library_dir(&env(Platform::Windows), v("4.4.3")),
            Some(
                PathBuf::from("/lad")
                    .join("RCraft")
                    .join("R-library")
                    .join("4.4")
            )
        );
        assert!(user_library_dir(&env(Platform::MacOs), v("4.4.3"))
            .is_some_and(|p| p.ends_with("Application Support/RCraft/R-library/4.4")));
        assert!(user_library_dir(&env(Platform::Linux), v("4.4.3"))
            .is_some_and(|p| p.ends_with(".local/share/rcraft/R-library/4.4")));
        let none = Env {
            platform: Some(Platform::Windows),
            ..Env::default()
        };
        assert_eq!(user_library_dir(&none, v("4.4.3")), None);
    }

    #[test]
    fn two_minor_versions_get_two_folders_and_patches_share_one() {
        for platform in [Platform::Windows, Platform::MacOs, Platform::Linux] {
            let e = env(platform);
            let a = user_library_dir(&e, v("4.4.3"));
            let b = user_library_dir(&e, v("4.6.2"));
            assert!(a.is_some() && b.is_some());
            assert_ne!(a, b, "{platform:?}");
            assert_eq!(
                a.as_ref().and_then(|p| p.parent()),
                b.as_ref().and_then(|p| p.parent())
            );
            assert_eq!(a, user_library_dir(&e, v("4.4.0")), "{platform:?}");
            assert!(b.is_some_and(|p| p.ends_with("R-library/4.6")));
        }
        assert_eq!(version_folder(v("10.12.1")), "10.12");
    }

    #[test]
    fn reads_the_r_version_that_built_a_package() {
        let d = "Package: dplyr\nVersion: 1.1.4\nBuilt: R 4.6.1; x86_64-w64-mingw32; 2026-01-01 \
                 00:00:00 UTC; windows\n";
        assert_eq!(built_version(d), Some(v("4.6.1")));
        assert_eq!(
            built_version("Built: R 4.4.0; ; 2024; unix"),
            Some(v("4.4.0"))
        );
        assert_eq!(built_version("Package: x\n"), None);
        assert_eq!(built_version("Built: S 4.4.0;"), None);
        assert!(is_version_folder("4.4") && is_version_folder("10.12"));
        for name in ["dplyr", "4", "4.4.1", "4.", ".4", "a.b", "00LOCK-x"] {
            assert!(!is_version_folder(name), "{name}");
        }
    }

    fn scratch(name: &str) -> PathBuf {
        let d = std::env::temp_dir()
            .join(format!("rcraft-library-{name}-{}", std::process::id()))
            .join(LIBRARY_FOLDER);
        let _ = std::fs::remove_dir_all(&d);
        let _ = std::fs::create_dir_all(&d);
        d
    }

    fn package(dir: &Path, name: &str, built: Option<&str>) {
        let p = dir.join(name);
        let _ = std::fs::create_dir_all(&p);
        let built = built
            .map(|b| format!("Built: R {b}; ; 2026; windows\n"))
            .unwrap_or_default();
        let _ = std::fs::write(p.join("DESCRIPTION"), format!("Package: {name}\n{built}"));
    }

    #[test]
    fn old_flat_packages_move_to_the_folder_of_the_r_that_built_them() {
        let root = scratch("sort");
        package(&root, "dplyr", Some("4.6.1"));
        package(&root, "rlang", Some("4.4.3"));
        package(&root, "odd", None);
        let _ = std::fs::create_dir_all(root.join("00LOCK-x"));
        // already in the new place: the old copy stays put
        package(&root.join("4.6"), "tibble", Some("4.6.2"));
        package(&root, "tibble", Some("4.6.1"));

        let sorted = sort_flat_library(&root);
        let mut moved = sorted.moved.clone();
        moved.sort();
        assert_eq!(
            moved,
            vec![
                ("dplyr".into(), "4.6".into()),
                ("rlang".into(), "4.4".into())
            ]
        );
        assert!(root.join("4.6").join("dplyr").join("DESCRIPTION").is_file());
        assert!(root.join("4.4").join("rlang").join("DESCRIPTION").is_file());
        assert!(!root.join("dplyr").exists());
        assert!(root.join("odd").join("DESCRIPTION").is_file());
        assert!(root.join("tibble").join("DESCRIPTION").is_file());
        assert!(root.join("00LOCK-x").is_dir());
        let mut left: Vec<&str> = sorted.left.iter().map(|(n, _)| n.as_str()).collect();
        left.sort_unstable();
        assert_eq!(left, vec!["odd", "tibble"]);
        // a second pass has nothing more to do
        assert_eq!(sort_flat_library(&root).moved, Vec::new());
        let _ = std::fs::remove_dir_all(root.parent().unwrap_or(&root));
    }

    #[test]
    fn prepare_makes_only_our_own_version_folders() {
        let root = scratch("prepare");
        package(&root, "dplyr", Some("4.6.1"));
        let lib = root.join("4.4");
        prepare(&lib);
        assert!(lib.is_dir());
        assert!(root.join("4.6").join("dplyr").is_dir());
        // somewhere else: left alone
        let other = root.parent().map(|p| p.join("mine")).unwrap_or_default();
        prepare(&other);
        assert!(!other.exists());
        prepare(Path::new(""));
        assert!(sort_flat_library(Path::new("/no/such/folder/here")) == Sorted::default());
        let _ = std::fs::remove_dir_all(root.parent().unwrap_or(&root));
    }

    #[test]
    fn the_library_goes_first_and_the_old_value_is_kept() {
        let lib = Path::new("/rc/lib");
        let one = libs_value(None, lib);
        assert_eq!(one, Some(OsString::from("/rc/lib")));
        let joined = std::env::join_paths(["/other"]).unwrap_or_default();
        let both = libs_value(Some(&joined), lib).unwrap_or_default();
        let parts: Vec<PathBuf> = std::env::split_paths(&both).collect();
        assert_eq!(
            parts,
            vec![PathBuf::from("/rc/lib"), PathBuf::from("/other")]
        );
        assert_eq!(libs_value(Some(OsStr::new("")), lib), one);
    }
}
