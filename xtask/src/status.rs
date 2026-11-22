//! `cargo xtask status-fold`: folds the status fragments into `ROADMAP.md`
//! and `plan/STATUS.md`.
//!
//! A feature or batch PR never edits those two files (every PR that did
//! conflicted with the next merge). It adds `plan/status.d/<sub-id>.md`, for
//! example `plan/status.d/P2C.12at.md`:
//!
//! ```text
//! ## ROADMAP
//!
//! - P2C.12at What was built (done 2026-11-20)
//!   (the rows, written as they would go into ROADMAP.md)
//!
//! ## STATUS
//!
//! ## P2C.12at: What was built, 2026-11-20
//!
//! - the section, written as it would go into plan/STATUS.md
//! ```
//!
//! Either part may be left out. A periodic bookkeeping PR runs
//! `cargo xtask status-fold`, which, for each fragment in file-name order:
//!
//! - puts its ROADMAP rows at the end of its phase's section in `ROADMAP.md`
//!   (the phase is the part of the sub-id between `P` and the first `.`:
//!   `P2C.12at` goes at the end of `## Phase 2C: ...`);
//! - adds its STATUS section at the end of `plan/STATUS.md`;
//! - deletes the fragment (`plan/status.d/README.md` stays).
//!
//! `cargo xtask status-fold --check` only reads the fragments and says what is
//! wrong with them (`cargo xtask ci` runs it), so a bad fragment is found in
//! its own PR and not in the bookkeeping one.

use std::path::{Path, PathBuf};

use crate::{root, Result};

/// One fragment: the rows and the section it carries.
#[derive(Debug, PartialEq, Eq)]
struct Fragment {
    /// The sub-id, from the file name (`P2C.12at`).
    id: String,
    roadmap: Option<String>,
    status: Option<String>,
}

pub fn run(args: &[String]) -> Result {
    let check = args.iter().any(|a| a == "--check");
    if let Some(other) = args.iter().find(|a| a.as_str() != "--check") {
        return Err(format!("status-fold: unknown argument `{other}`"));
    }
    let dir = root().join("plan").join("status.d");
    let files = fragment_files(&dir)?;
    let roadmap_path = root().join("ROADMAP.md");
    let status_path = root().join("plan").join("STATUS.md");
    let mut roadmap = read(&roadmap_path)?;
    let mut status = read(&status_path)?;
    for file in &files {
        let id = file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        let name = file.display();
        let fragment = parse(id, &read(file)?).map_err(|e| format!("{name}: {e}"))?;
        // folding into the copies in memory also checks the phase exists
        roadmap = fold_roadmap(&roadmap, &fragment).map_err(|e| format!("{name}: {e}"))?;
        status = fold_status(&status, &fragment);
    }
    if check {
        println!("status-fold: {} fragment(s) are well formed", files.len());
        return Ok(());
    }
    if files.is_empty() {
        println!("status-fold: no fragments in plan/status.d");
        return Ok(());
    }
    write(&roadmap_path, &roadmap)?;
    write(&status_path, &status)?;
    for file in &files {
        std::fs::remove_file(file).map_err(|e| format!("{}: {e}", file.display()))?;
    }
    println!(
        "status-fold: folded {} fragment(s) into ROADMAP.md and plan/STATUS.md and deleted them",
        files.len()
    );
    Ok(())
}

/// The fragments in `dir`, by file name: the `.md` files but `README.md`.
fn fragment_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        // no folder, no fragments
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(files),
        Err(e) => return Err(format!("{}: {e}", dir.display())),
    };
    for entry in entries {
        let path = entry.map_err(|e| e.to_string())?.path();
        let is_md = path.extension().and_then(|e| e.to_str()) == Some("md");
        let is_readme = path.file_name().and_then(|n| n.to_str()) == Some("README.md");
        if path.is_file() && is_md && !is_readme {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

fn read(path: &Path) -> Result<String> {
    std::fs::read_to_string(path)
        .map(|t| t.replace("\r\n", "\n"))
        .map_err(|e| format!("{}: {e}", path.display()))
}

fn write(path: &Path, text: &str) -> Result {
    std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}

/// Reads a fragment: `## ROADMAP` and `## STATUS` split it (only those two
/// exact lines; a `## P2C.12at: ...` heading inside the STATUS part is its
/// text).
fn parse(id: &str, text: &str) -> Result<Fragment> {
    phase_of(id)?;
    let (mut roadmap, mut status): (Option<Vec<&str>>, Option<Vec<&str>>) = (None, None);
    let mut current: Option<bool> = None; // Some(true) is ROADMAP
    for line in text.lines() {
        match line.trim_end() {
            "## ROADMAP" if roadmap.is_none() => {
                roadmap = Some(Vec::new());
                current = Some(true);
            }
            "## STATUS" if status.is_none() => {
                status = Some(Vec::new());
                current = Some(false);
            }
            _ => match current {
                Some(true) => roadmap.get_or_insert_with(Vec::new).push(line),
                Some(false) => status.get_or_insert_with(Vec::new).push(line),
                None if line.trim().is_empty() => {}
                None => {
                    return Err(
                        "text before the first `## ROADMAP` or `## STATUS` heading".to_string()
                    )
                }
            },
        }
    }
    let body = |part: Option<Vec<&str>>, name: &str| -> Result<Option<String>> {
        match part {
            None => Ok(None),
            Some(lines) => {
                let text = lines.join("\n").trim_matches('\n').trim_end().to_string();
                if text.trim().is_empty() {
                    Err(format!("the `## {name}` part is empty"))
                } else {
                    Ok(Some(text))
                }
            }
        }
    };
    let fragment = Fragment {
        id: id.to_string(),
        roadmap: body(roadmap, "ROADMAP")?,
        status: body(status, "STATUS")?,
    };
    if fragment.roadmap.is_none() && fragment.status.is_none() {
        return Err("it has neither a `## ROADMAP` nor a `## STATUS` part".to_string());
    }
    Ok(fragment)
}

/// The phase of a sub-id: `P2C.12at` is `2C`.
fn phase_of(id: &str) -> Result<&str> {
    id.strip_prefix('P')
        .and_then(|rest| rest.split_once('.'))
        .map(|(phase, _)| phase)
        .filter(|p| !p.is_empty())
        .ok_or_else(|| format!("`{id}` is not a sub-id like P2C.12at (the file name)"))
}

/// `roadmap` with the fragment's rows added at the end of its phase.
fn fold_roadmap(roadmap: &str, fragment: &Fragment) -> Result<String> {
    let Some(rows) = &fragment.roadmap else {
        return Ok(roadmap.to_string());
    };
    let phase = phase_of(&fragment.id)?;
    let heading = format!("## Phase {phase}:");
    let lines: Vec<&str> = roadmap.lines().collect();
    let start = lines
        .iter()
        .position(|l| l.starts_with(&heading))
        .ok_or_else(|| {
            format!(
                "ROADMAP.md has no `{heading} ...` section for {}",
                fragment.id
            )
        })?;
    let end = lines
        .iter()
        .skip(start + 1)
        .position(|l| l.starts_with("## "))
        .map_or(lines.len(), |p| start + 1 + p);
    // after the section's last line that has text
    let at = (start + 1..end)
        .rev()
        .find(|&i| lines.get(i).is_some_and(|l| !l.trim().is_empty()))
        .map_or(start + 1, |i| i + 1);
    let mut out: Vec<&str> = lines.get(..at).unwrap_or_default().to_vec();
    out.extend(rows.lines());
    out.extend(lines.get(at..).unwrap_or_default());
    Ok(out.join("\n") + "\n")
}

/// `status` with the fragment's section added at the end.
fn fold_status(status: &str, fragment: &Fragment) -> String {
    let Some(section) = &fragment.status else {
        return status.to_string();
    };
    format!("{}\n\n{section}\n", status.trim_end_matches('\n'))
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAGMENT: &str = "\
## ROADMAP

- P2C.12zz Something (done 2026-11-22)
  (what it does)

## STATUS

## P2C.12zz: Something, 2026-11-22

- **Done:** it.
";

    const ROADMAP: &str = "\
# Roadmap

## Phase 2B: Stata mode

- P2B.1 first

## Phase 2C: Run my do-file

- P2C.1 one
  more of one
- P2C.2 two

## Phase 3: Data

- P3.1 data
";

    #[test]
    fn a_fragment_splits_on_its_two_headings_only() {
        let f = parse("P2C.12zz", FRAGMENT);
        assert_eq!(
            f,
            Ok(Fragment {
                id: "P2C.12zz".into(),
                roadmap: Some("- P2C.12zz Something (done 2026-11-22)\n  (what it does)".into()),
                status: Some("## P2C.12zz: Something, 2026-11-22\n\n- **Done:** it.".into()),
            })
        );
    }

    #[test]
    fn a_bad_fragment_is_refused() {
        assert!(parse("P2C.1", "stray text\n## ROADMAP\n- x\n").is_err());
        assert!(parse("P2C.1", "## ROADMAP\n\n## STATUS\n- x\n").is_err());
        assert!(parse("P2C.1", "").is_err());
        assert!(parse("zz", "## STATUS\nx\n").is_err());
    }

    #[test]
    fn the_rows_go_at_the_end_of_their_phase() {
        let f = parse("P2C.12zz", FRAGMENT).unwrap_or_else(|e| panic!("{e}"));
        let out = fold_roadmap(ROADMAP, &f).unwrap_or_else(|e| panic!("{e}"));
        let expect = "\
# Roadmap

## Phase 2B: Stata mode

- P2B.1 first

## Phase 2C: Run my do-file

- P2C.1 one
  more of one
- P2C.2 two
- P2C.12zz Something (done 2026-11-22)
  (what it does)

## Phase 3: Data

- P3.1 data
";
        assert_eq!(out, expect);
        // a phase the roadmap lacks is an error
        let g = parse("P9.1", "## ROADMAP\n- x\n").unwrap_or_else(|e| panic!("{e}"));
        assert!(fold_roadmap(ROADMAP, &g).is_err());
    }

    #[test]
    fn the_section_goes_at_the_end_of_status() {
        let f = parse("P2C.12zz", FRAGMENT).unwrap_or_else(|e| panic!("{e}"));
        let out = fold_status("# Status\n\n## Earlier\n\n- x\n", &f);
        assert_eq!(
            out,
            "# Status\n\n## Earlier\n\n- x\n\n## P2C.12zz: Something, 2026-11-22\n\n- **Done:** it.\n"
        );
    }
}
