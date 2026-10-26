//! Asset check: every file under `assets/` needs a row in `ATTRIBUTION.md`.

use std::fs;
use std::path::Path;

use crate::{root, Result};

pub fn check() -> Result {
    let root = root();
    let attribution = fs::read_to_string(root.join("ATTRIBUTION.md"))
        .map_err(|e| format!("could not read ATTRIBUTION.md: {e}"))?;
    let mut files = Vec::new();
    collect(&root.join("assets"), &root, &mut files)?;
    let missing: Vec<&String> = files
        .iter()
        .filter(|f| !attribution.contains(f.as_str()))
        .collect();
    if missing.is_empty() {
        println!("assets: {} files, all attributed", files.len());
        Ok(())
    } else {
        Err(format!(
            "these assets have no row in ATTRIBUTION.md:\n  {}",
            missing
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join("\n  ")
        ))
    }
}

fn collect(dir: &Path, root: &Path, out: &mut Vec<String>) -> Result {
    if !dir.exists() {
        return Ok(());
    }
    let entries = fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for entry in entries {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.is_dir() {
            collect(&path, root, out)?;
        } else if let Ok(rel) = path.strip_prefix(root) {
            out.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
    Ok(())
}
