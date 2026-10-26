//! Layer check: each RCraft crate may depend only on crates in the same
//! or a lower layer, and nothing below L6 may pull in a user interface.
//!
//! Reads the members' `Cargo.toml` files directly. They follow one simple
//! shape (see any crate in `crates/`), so a line reader is enough and keeps
//! this tool free of dependencies.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::{root, Result};

const UI_LAYER: u64 = 6;
const UI_CRATES: &[&str] = &["egui", "eframe", "egui-wgpu", "winit", "rfd", "wgpu"];
const MEMBER_DIRS: &[&str] = &["crates", "apps"];

struct Manifest {
    name: String,
    layer: Option<u64>,
    dependencies: Vec<String>,
}

pub fn check() -> Result {
    let root = root();
    let mut manifests = Vec::new();
    for dir in MEMBER_DIRS {
        let Ok(entries) = fs::read_dir(root.join(dir)) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path().join("Cargo.toml");
            if path.is_file() {
                manifests.push(read_manifest(&path)?);
            }
        }
    }

    let mut layer_of = HashMap::new();
    for m in &manifests {
        let layer = m
            .layer
            .ok_or_else(|| format!("{} has no [package.metadata.rcraft] layer", m.name))?;
        layer_of.insert(m.name.as_str(), layer);
    }

    let mut problems = Vec::new();
    for m in &manifests {
        let layer = layer_of.get(m.name.as_str()).copied().unwrap_or_default();
        for dep in &m.dependencies {
            if let Some(&dep_layer) = layer_of.get(dep.as_str()) {
                if dep_layer > layer {
                    problems.push(format!(
                        "{} (L{layer}) depends on {dep} (L{dep_layer})",
                        m.name
                    ));
                }
            } else if layer < UI_LAYER && UI_CRATES.contains(&dep.as_str()) {
                problems.push(format!(
                    "{} (L{layer}) depends on interface crate {dep}",
                    m.name
                ));
            }
        }
    }

    if problems.is_empty() {
        println!("layers: {} crates checked", manifests.len());
        Ok(())
    } else {
        Err(format!("layer rules broken:\n  {}", problems.join("\n  ")))
    }
}

fn read_manifest(path: &Path) -> Result<Manifest> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut manifest = parse_manifest(&text);
    if manifest.name.is_empty() {
        return Err(format!("{} has no package name", path.display()));
    }
    manifest.dependencies.sort();
    Ok(manifest)
}

fn parse_manifest(text: &str) -> Manifest {
    let mut name = String::new();
    let mut layer = None;
    let mut dependencies = Vec::new();
    let mut section = String::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(header) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            section = header.trim().to_string();
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        match section.as_str() {
            "package" if key == "name" => name = value.trim_matches('"').to_string(),
            "package.metadata.rcraft" if key == "layer" => layer = value.parse().ok(),
            "dependencies" => {
                // `foo = "1"`, `foo.workspace = true` or `foo = { ... }`
                let dep = key.split('.').next().unwrap_or(key).trim_matches('"');
                dependencies.push(dep.to_string());
            }
            _ => {}
        }
    }
    Manifest {
        name,
        layer,
        dependencies,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_name_layer_and_dependencies() {
        let m = parse_manifest(
            "[package]\nname = \"rcraft-dta\"\nversion.workspace = true\n\n\
             [package.metadata.rcraft]\nlayer = 2\n\n[dependencies]\n\
             rcraft-core.workspace = true\nfoo = { version = \"1\" }\nbar = \"2\"\n\n\
             [lints]\nworkspace = true\n",
        );
        assert_eq!(m.name, "rcraft-dta");
        assert_eq!(m.layer, Some(2));
        assert_eq!(m.dependencies, ["rcraft-core", "foo", "bar"]);
    }
}
