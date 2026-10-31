//! `cargo xtask stata-record`: runs the Stata corpus (P2B.10) on a licensed
//! Stata and keeps its results.
//!
//! Each `NAME.do` in `crates/engine/tests/stata_corpus` reads the made-up
//! `corpus.csv` and prints every result it checks as `@ key value`. This
//! task runs each one in Stata's batch mode, keeps only those `@` lines (and
//! Stata's version), and writes them to `NAME.recorded`, which is committed;
//! the log Stata writes, which carries its licence banner, is deleted and
//! never kept. The corpus test then runs the same do-files through RCraft
//! and compares every result with the recorded one.
//!
//! Stata is found from `RCRAFT_STATA`, else in its usual places. Without a
//! Stata the task says so and records nothing; the tests run on what is
//! committed.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::Result;

const CORPUS: &str = "crates/engine/tests/stata_corpus";

pub fn record() -> Result {
    let stata = find_stata()
        .ok_or("no Stata found: set RCRAFT_STATA to its program (StataSE-64.exe, stata-se, ...)")?;
    let dir = Path::new(CORPUS);
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| format!("{CORPUS}: {e}"))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "do"))
        .collect();
    files.sort();
    for file in files {
        let name = file
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or("a do-file without a name")?
            .to_string();
        let batch = if cfg!(windows) { "/e" } else { "-b" };
        let status = Command::new(&stata)
            .args([batch, "do", &format!("{name}.do")])
            .current_dir(dir)
            .status()
            .map_err(|e| format!("{}: {e}", stata.display()))?;
        let log = dir.join(format!("{name}.log"));
        let text = std::fs::read_to_string(&log).map_err(|e| format!("{}: {e}", log.display()));
        // the log is Stata's, with its banner: never kept
        let _ = std::fs::remove_file(&log);
        let text = text?;
        if !status.success() || text.lines().any(is_error) {
            return Err(format!("{name}.do stopped with an error in Stata"));
        }
        let version = text
            .lines()
            .find_map(|l| l.trim().split("  ").find(|w| w.starts_with("Stata ")))
            .unwrap_or("Stata")
            .trim()
            .to_string();
        let results: Vec<&str> = text.lines().filter(|l| l.starts_with("@ ")).collect();
        let mut out =
            format!("* {version}'s results for {name}.do, recorded by cargo xtask stata-record\n");
        for r in &results {
            out.push_str(r.trim_end());
            out.push('\n');
        }
        let recorded = dir.join(format!("{name}.recorded"));
        std::fs::write(&recorded, out).map_err(|e| format!("{}: {e}", recorded.display()))?;
        println!("stata-record: {name}: {} results", results.len());
    }
    Ok(())
}

/// Stata's `r(111);` line, which ends a do-file that failed.
fn is_error(line: &str) -> bool {
    line.strip_prefix("r(")
        .and_then(|r| r.strip_suffix(");"))
        .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
}

fn find_stata() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("RCRAFT_STATA") {
        return Some(PathBuf::from(p));
    }
    let windows = ["StataSE-64.exe", "StataMP-64.exe", "StataBE-64.exe"];
    for version in (17..=20).rev() {
        for exe in windows {
            let p = PathBuf::from(format!("C:\\Program Files\\Stata{version}\\{exe}"));
            if p.is_file() {
                return Some(p);
            }
        }
    }
    for exe in ["stata-se", "stata-mp", "stata"] {
        if let Some(path) = std::env::var_os("PATH") {
            for d in std::env::split_paths(&path) {
                let p = d.join(exe);
                if p.is_file() {
                    return Some(p);
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_failed_do_file_is_seen() {
        assert!(is_error("r(111);"));
        assert!(is_error("r(430);"));
        assert!(!is_error("r(N)"));
        assert!(!is_error("@ r(1);"));
    }
}
