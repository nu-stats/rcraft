//! P1.1: find installed copies of R.
//!
//! Looks in `R_HOME`, on `PATH` and in the standard places for each system
//! (including the folders `rig` uses), asks each `Rscript` for its version
//! with a short time limit, and returns the usable ones, newest first. Nothing
//! is installed or downloaded here.
//!
//! The search is a pure function of an [`Env`], so tests describe a made-up
//! machine instead of reading the real one.

use std::collections::HashSet;
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// The oldest R that RCraft supports.
pub const MIN_VERSION: RVersion = RVersion {
    major: 4,
    minor: 1,
    patch: 0,
};

/// How long `Rscript --version` may take before it is given up on.
const PROBE_TIMEOUT: Duration = Duration::from_secs(10);

/// Caps on input-sized work (AGENTS.md section 2).
const MAX_DIR_ENTRIES: usize = 200;
const MAX_VERSION_OUTPUT: usize = 4096;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    Windows,
    MacOs,
    Linux,
}

impl Platform {
    pub fn current() -> Platform {
        if cfg!(windows) {
            Platform::Windows
        } else if cfg!(target_os = "macos") {
            Platform::MacOs
        } else {
            Platform::Linux
        }
    }

    fn rscript_name(self) -> &'static str {
        match self {
            Platform::Windows => "Rscript.exe",
            _ => "Rscript",
        }
    }
}

/// An R version, ordered by number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl RVersion {
    /// Reads `4.4.1` (extra parts after a space, hyphen or fourth dot are
    /// ignored, so `4.5.0-patched` and `4.4.1.9000` work).
    pub fn parse(text: &str) -> Option<RVersion> {
        let text = text.trim();
        let end = text
            .find(|c: char| !(c.is_ascii_digit() || c == '.'))
            .unwrap_or(text.len());
        let mut parts = text.get(..end)?.split('.');
        let major = parts.next()?.parse().ok()?;
        let minor = parts.next()?.parse().ok()?;
        let patch = parts.next().map_or(Some(0), |p| p.parse().ok())?;
        Some(RVersion {
            major,
            minor,
            patch,
        })
    }
}

impl fmt::Display for RVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Finds the version in the first line that has one of the forms R prints:
/// `R scripting front-end version 4.4.1 (2024-06-14)` or
/// `R version 4.4.1 (2024-06-14) -- "Race for Your Life"`.
pub fn parse_version_output(output: &str) -> Option<RVersion> {
    output.lines().take(8).find_map(|line| {
        let rest = line.split_once("version")?.1;
        RVersion::parse(rest.trim_start())
    })
}

/// One usable copy of R.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RInstall {
    pub rscript: PathBuf,
    pub version: RVersion,
}

impl RInstall {
    pub fn is_supported(&self) -> bool {
        self.version >= MIN_VERSION
    }
}

/// Why a candidate was not usable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProbeError {
    NotRunnable(String),
    TimedOut,
    NoVersion,
}

impl fmt::Display for ProbeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProbeError::NotRunnable(why) => write!(f, "could not run it: {why}"),
            ProbeError::TimedOut => write!(f, "it did not answer in time"),
            ProbeError::NoVersion => write!(f, "its output has no R version"),
        }
    }
}

/// What the search needs to know about the machine.
#[derive(Clone, Debug, Default)]
pub struct Env {
    pub platform: Option<Platform>,
    pub r_home: Option<PathBuf>,
    pub path: Vec<PathBuf>,
    pub program_files: Vec<PathBuf>,
    pub local_app_data: Option<PathBuf>,
    pub home: Option<PathBuf>,
}

impl Env {
    /// The real machine.
    pub fn from_process() -> Env {
        let var = |name: &str| std::env::var_os(name).filter(|v| !v.is_empty());
        Env {
            platform: Some(Platform::current()),
            r_home: var("R_HOME").map(PathBuf::from),
            path: var("PATH")
                .map(|p| std::env::split_paths(&p).collect())
                .unwrap_or_default(),
            program_files: ["ProgramFiles", "ProgramW6432", "ProgramFiles(x86)"]
                .iter()
                .filter_map(|n| var(n).map(PathBuf::from))
                .collect(),
            local_app_data: var("LOCALAPPDATA").map(PathBuf::from),
            home: var("HOME")
                .or_else(|| var("USERPROFILE"))
                .map(PathBuf::from),
        }
    }

    fn platform(&self) -> Platform {
        self.platform.unwrap_or_else(Platform::current)
    }
}

/// Where to look, most specific first, without duplicates. `list_dirs`
/// returns the sub-folders of a folder (empty when it does not exist).
pub fn candidates(env: &Env, list_dirs: &dyn Fn(&Path) -> Vec<PathBuf>) -> Vec<PathBuf> {
    let platform = env.platform();
    let name = platform.rscript_name();
    let mut out: Vec<PathBuf> = Vec::new();

    if let Some(home) = &env.r_home {
        out.push(home.join("bin").join(name));
    }
    for dir in &env.path {
        out.push(dir.join(name));
    }

    // Folders that hold one sub-folder per R version.
    let mut roots: Vec<PathBuf> = Vec::new();
    match platform {
        Platform::Windows => {
            for pf in &env.program_files {
                roots.push(pf.join("R"));
            }
            if let Some(local) = &env.local_app_data {
                roots.push(local.join("Programs").join("R"));
            }
        }
        Platform::MacOs => {
            roots.push(PathBuf::from("/Library/Frameworks/R.framework/Versions"));
        }
        Platform::Linux => {
            roots.push(PathBuf::from("/opt/R"));
            if let Some(home) = &env.home {
                roots.push(home.join(".local").join("share").join("rig"));
            }
        }
    }
    for root in &roots {
        for version_dir in list_dirs(root).into_iter().take(MAX_DIR_ENTRIES) {
            let resources = match platform {
                Platform::MacOs => version_dir.join("Resources"),
                _ => version_dir,
            };
            out.push(resources.join("bin").join(name));
            if platform == Platform::Windows {
                out.push(resources.join("bin").join("x64").join(name));
            }
        }
    }

    // Fixed places for the system's own R and Homebrew.
    match platform {
        Platform::Windows => {}
        Platform::MacOs => {
            for dir in ["/opt/homebrew/bin", "/usr/local/bin"] {
                out.push(Path::new(dir).join(name));
            }
            out.push(PathBuf::from(
                "/Library/Frameworks/R.framework/Resources/bin/Rscript",
            ));
        }
        Platform::Linux => {
            for dir in ["/usr/bin", "/usr/local/bin"] {
                out.push(Path::new(dir).join(name));
            }
        }
    }

    let mut seen = HashSet::new();
    out.retain(|p| seen.insert(p.clone()));
    out
}

/// Asks one `Rscript` for its version. Runs it with no input and a time
/// limit, and reads at most [`MAX_VERSION_OUTPUT`] bytes of what it prints.
pub fn probe(rscript: &Path) -> Result<RInstall, ProbeError> {
    if !rscript.is_file() {
        return Err(ProbeError::NotRunnable("no such file".into()));
    }
    let mut child = Command::new(rscript)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| ProbeError::NotRunnable(e.to_string()))?;

    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if started.elapsed() >= PROBE_TIMEOUT => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(ProbeError::TimedOut);
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(e) => return Err(ProbeError::NotRunnable(e.to_string())),
        }
    }
    let output = child
        .wait_with_output()
        .map_err(|e| ProbeError::NotRunnable(e.to_string()))?;

    // `Rscript --version` prints to standard error; other front ends use
    // standard output.
    let mut text = String::from_utf8_lossy(&output.stderr).into_owned();
    text.push('\n');
    text.push_str(&String::from_utf8_lossy(&output.stdout));
    text.truncate(floor_boundary(&text, MAX_VERSION_OUTPUT));

    let version = parse_version_output(&text).ok_or(ProbeError::NoVersion)?;
    Ok(RInstall {
        rscript: rscript.to_path_buf(),
        version,
    })
}

/// The largest index at or below `limit` that is a character boundary.
fn floor_boundary(text: &str, limit: usize) -> usize {
    let mut end = limit.min(text.len());
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    end
}

fn list_dirs_on_disk(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut dirs: Vec<PathBuf> = entries
        .flatten()
        .take(MAX_DIR_ENTRIES)
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    dirs
}

/// Every usable R on the machine, newest first. Copies that resolve to the
/// same file are listed once.
pub fn find_installs(env: &Env) -> Vec<RInstall> {
    let mut seen = HashSet::new();
    let mut found: Vec<RInstall> = Vec::new();
    for path in candidates(env, &list_dirs_on_disk) {
        if !path.is_file() {
            continue;
        }
        let key = std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
        if !seen.insert(key) {
            continue;
        }
        if let Ok(install) = probe(&path) {
            found.push(install);
        }
    }
    found.sort_by_key(|i| std::cmp::Reverse(i.version));
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(major: u32, minor: u32, patch: u32) -> RVersion {
        RVersion {
            major,
            minor,
            patch,
        }
    }

    fn no_dirs(_: &Path) -> Vec<PathBuf> {
        Vec::new()
    }

    #[test]
    fn parses_versions() {
        assert_eq!(RVersion::parse("4.4.1"), Some(v(4, 4, 1)));
        assert_eq!(RVersion::parse("4.5.0-patched"), Some(v(4, 5, 0)));
        assert_eq!(RVersion::parse("4.4.1.9000"), Some(v(4, 4, 1)));
        assert_eq!(RVersion::parse("4.4"), Some(v(4, 4, 0)));
        assert_eq!(RVersion::parse("four"), None);
        assert_eq!(RVersion::parse(""), None);
        assert_eq!(RVersion::parse("4"), None);
        assert_eq!(RVersion::parse("99999999999.1.1"), None);
    }

    #[test]
    fn orders_versions_by_number() {
        assert!(v(4, 10, 0) > v(4, 9, 9));
        assert!(v(4, 4, 1) >= MIN_VERSION);
        assert!(v(4, 0, 5) < MIN_VERSION);
    }

    #[test]
    fn reads_both_output_forms() {
        let rscript = "R scripting front-end version 4.4.1 (2024-06-14)\n";
        assert_eq!(parse_version_output(rscript), Some(v(4, 4, 1)));
        let r = "R version 4.3.3 (2024-02-29) -- \"Angel Food Cake\"\nCopyright";
        assert_eq!(parse_version_output(r), Some(v(4, 3, 3)));
        assert_eq!(parse_version_output("command not found"), None);
        assert_eq!(parse_version_output(""), None);
    }

    #[test]
    fn windows_candidates() {
        let env = Env {
            platform: Some(Platform::Windows),
            r_home: Some(PathBuf::from("C:/rhome")),
            path: vec![PathBuf::from("C:/tools")],
            program_files: vec![PathBuf::from("C:/Program Files")],
            local_app_data: Some(PathBuf::from("C:/Users/a/AppData/Local")),
            home: None,
        };
        let list = |dir: &Path| {
            if dir == Path::new("C:/Program Files/R") {
                vec![PathBuf::from("C:/Program Files/R/R-4.4.1")]
            } else {
                Vec::new()
            }
        };
        let got = candidates(&env, &list);
        assert_eq!(
            got.first(),
            Some(&PathBuf::from("C:/rhome/bin/Rscript.exe"))
        );
        assert!(got.contains(&PathBuf::from("C:/tools/Rscript.exe")));
        assert!(got.contains(&PathBuf::from("C:/Program Files/R/R-4.4.1/bin/Rscript.exe")));
        assert!(got.contains(&PathBuf::from(
            "C:/Program Files/R/R-4.4.1/bin/x64/Rscript.exe"
        )));
    }

    #[test]
    fn mac_and_linux_candidates() {
        let mac = Env {
            platform: Some(Platform::MacOs),
            ..Env::default()
        };
        let list = |dir: &Path| {
            if dir == Path::new("/Library/Frameworks/R.framework/Versions") {
                vec![PathBuf::from(
                    "/Library/Frameworks/R.framework/Versions/4.4-arm64",
                )]
            } else {
                Vec::new()
            }
        };
        let got = candidates(&mac, &list);
        assert!(got.contains(&PathBuf::from(
            "/Library/Frameworks/R.framework/Versions/4.4-arm64/Resources/bin/Rscript"
        )));
        assert!(got.contains(&PathBuf::from("/opt/homebrew/bin/Rscript")));

        let linux = Env {
            platform: Some(Platform::Linux),
            home: Some(PathBuf::from("/home/a")),
            ..Env::default()
        };
        let list = |dir: &Path| {
            if dir == Path::new("/opt/R") {
                vec![PathBuf::from("/opt/R/4.3.2")]
            } else {
                Vec::new()
            }
        };
        let got = candidates(&linux, &list);
        assert!(got.contains(&PathBuf::from("/opt/R/4.3.2/bin/Rscript")));
        assert!(got.contains(&PathBuf::from("/usr/bin/Rscript")));
    }

    #[test]
    fn candidates_have_no_duplicates() {
        let env = Env {
            platform: Some(Platform::Linux),
            path: vec![PathBuf::from("/usr/bin"), PathBuf::from("/usr/bin")],
            ..Env::default()
        };
        let got = candidates(&env, &no_dirs);
        let unique: HashSet<_> = got.iter().collect();
        assert_eq!(got.len(), unique.len());
    }

    #[test]
    fn huge_listings_are_capped() {
        let env = Env {
            platform: Some(Platform::Linux),
            ..Env::default()
        };
        let list = |dir: &Path| {
            if dir == Path::new("/opt/R") {
                (0..100_000)
                    .map(|i| PathBuf::from(format!("/opt/R/{i}")))
                    .collect()
            } else {
                Vec::new()
            }
        };
        assert!(candidates(&env, &list).len() < 1000);
    }

    #[test]
    fn a_missing_file_is_not_runnable() {
        let result = probe(Path::new("definitely/not/here/Rscript"));
        assert!(matches!(result, Err(ProbeError::NotRunnable(_))));
    }

    #[test]
    fn an_empty_machine_finds_nothing() {
        let env = Env {
            platform: Some(Platform::Linux),
            path: vec![PathBuf::from("definitely/not/here")],
            ..Env::default()
        };
        // The fixed system folders may hold a real R on the machine running
        // the tests, so only check that the search returns without error.
        let _ = find_installs(&env);
    }

    #[test]
    fn truncation_respects_character_boundaries() {
        let text = "é".repeat(10);
        let end = floor_boundary(&text, 5);
        assert!(text.is_char_boundary(end));
        assert!(end <= 5);
    }
}
