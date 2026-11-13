//! `cargo xtask <task>`: build checks.

mod assets;
mod corpus;
mod coverage;
mod layers;

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const USAGE: &str = "\
usage: cargo xtask <task>

  ci                   fmt, clippy, tests, layers, assets and coverage (run before every commit;
                       uses cargo-nextest when installed, so the test programs run side by side)
  layers               check that crates only depend on lower layers
  assets               check that every file in assets/ is listed in ATTRIBUTION.md
  coverage [--check]   regenerate docs/coverage.md from the catalog
  stata-record         run the Stata corpus on a licensed Stata and keep its results (P2B.10)";

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
        Some("stata-record") => corpus::record(),
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
    tests()?;
    layers::check()?;
    assets::check()?;
    coverage::run(true)?;
    println!("ci: all checks passed");
    Ok(())
}

/// The workspace's tests. With cargo-nextest installed the test programs
/// run side by side (cargo test runs them one after another), and the doc
/// tests, which nextest does not run, follow; without it, plain cargo test.
fn tests() -> Result {
    let cargo_bin = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let nextest = Command::new(cargo_bin)
        .args(["nextest", "--version"])
        .current_dir(root())
        .output()
        .is_ok_and(|o| o.status.success());
    if nextest {
        cargo(&[
            "nextest",
            "run",
            "--workspace",
            "--no-fail-fast",
            "--status-level",
            "fail",
            "--final-status-level",
            "slow",
        ])?;
        cargo(&["test", "--workspace", "--doc", "--quiet"])
    } else {
        cargo(&["test", "--workspace", "--quiet"])
    }
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
