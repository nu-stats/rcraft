//! `cargo xtask <task>`: build checks.

mod assets;
mod coverage;
mod layers;

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const USAGE: &str = "\
usage: cargo xtask <task>

  ci                   fmt, clippy, tests, layers, assets and coverage (run before every commit)
  layers               check that crates only depend on lower layers
  assets               check that every file in assets/ is listed in ATTRIBUTION.md
  coverage [--check]   regenerate docs/coverage.md from the catalog";

pub type Result<T = ()> = std::result::Result<T, String>;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let task = args.first().map(String::as_str);
    let rest = args.get(1..).unwrap_or_default();
    let result = match task {
        Some("ci") => ci(),
        Some("layers") => layers::check(),
        Some("assets") => assets::check(),
        Some("coverage") => coverage::run(rest.iter().any(|a| a == "--check")),
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

fn ci() -> Result {
    cargo(&["fmt", "--all", "--check"])?;
    cargo(&[
        "clippy",
        "--workspace",
        "--all-targets",
        "--",
        "-D",
        "warnings",
    ])?;
    cargo(&["test", "--workspace", "--quiet"])?;
    layers::check()?;
    assets::check()?;
    coverage::run(true)?;
    println!("ci: all checks passed");
    Ok(())
}

fn cargo(args: &[&str]) -> Result {
    println!("> cargo {}", args.join(" "));
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let status = Command::new(cargo)
        .args(args)
        .current_dir(root())
        .status()
        .map_err(|e| format!("could not run cargo: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("cargo {} failed", args.join(" ")))
    }
}

/// The workspace root (the parent of this crate's folder).
pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}
