//! Gathers the per-command files into the tables the translator reads, so a
//! change that adds a command adds files and edits none of the shared ones.
//!
//! Each of these folders under `src/` holds one small file per command (see
//! the `README.md` in each); this script writes one Rust file per folder
//! into `OUT_DIR`, which the module next to the folder `include!`s:
//!
//! | folder | file holds | table |
//! |---|---|---|
//! | `differences/` | a `Guide` | `differences::GUIDES` (the `general` page first, then by file name) |
//! | `differences/` | `<page>.aliases.rs`: command names | `differences::ALIASES` |
//! | `verified/` | the corpus files that check the command | `verified::VERIFIED`, by command name |
//! | `verified/functions/` | the same for a function | `verified::VERIFIED_FUNCTIONS`, by name |
//! | `complete/` | a `Command` (or `NONE`) | `complete::COMMANDS` and the option and subcommand lookups |
//!
//! The file name is the key: `areg.rs` holds the entry of `areg`. Every table
//! is sorted by key here, never by the order the files are listed in, so the
//! result is the same on every platform. A function name is written with each
//! capital letter as `_` and the small letter (`Clock` is `_clock.rs`, `clock`
//! is `clock.rs`), so two names never differ only in case on a case-insensitive
//! file system.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

type Res<T> = Result<T, Box<dyn Error>>;

/// One file of a folder: the name it is keyed by, and where it is.
struct Entry {
    key: String,
    path: PathBuf,
}

fn main() -> Res<()> {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let out = PathBuf::from(std::env::var("OUT_DIR")?);
    let src = manifest.join("src");
    println!("cargo:rerun-if-changed=build.rs");
    for dir in ["differences", "verified", "complete"] {
        println!("cargo:rerun-if-changed=src/{dir}");
    }

    fs::write(
        out.join("differences.rs"),
        differences(&src.join("differences"))?,
    )?;
    fs::write(out.join("verified.rs"), verified(&src.join("verified"))?)?;
    fs::write(out.join("complete.rs"), complete(&src.join("complete"))?)?;
    Ok(())
}

/// The `.rs` files directly in `dir` (not in folders under it), by key.
/// `decode` turns a file name into its key.
fn entries(dir: &Path, decode: fn(&str) -> Res<String>) -> Res<Vec<Entry>> {
    let mut found = Vec::new();
    for item in fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let path = item?.path();
        if !path.is_file() || path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| format!("{}: not a usable file name", path.display()))?;
        found.push(Entry {
            key: decode(stem).map_err(|e| format!("{}: {e}", path.display()))?,
            path,
        });
    }
    found.sort_by(|a, b| a.key.cmp(&b.key));
    // two names that differ only in case cannot live together on Windows or macOS
    let mut lower = BTreeSet::new();
    for e in &found {
        let file = e.path.file_name().and_then(|f| f.to_str()).unwrap_or("");
        if !lower.insert(file.to_lowercase()) {
            return Err(format!(
                "{}: differs only in case from another file",
                e.path.display()
            )
            .into());
        }
    }
    Ok(found)
}

/// A name made of letters, digits and `_`, as a command's is.
fn plain(stem: &str) -> Res<String> {
    if stem.is_empty() || !stem.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(format!("`{stem}` is not a name of letters, digits and _").into());
    }
    Ok(stem.to_string())
}

/// A function name: each `_x` is a capital X, `__` is `_`.
fn function_name(stem: &str) -> Res<String> {
    let mut name = String::new();
    let mut chars = stem.chars();
    while let Some(c) = chars.next() {
        if c == '_' {
            match chars.next() {
                Some('_') => name.push('_'),
                Some(l) if l.is_ascii_lowercase() => name.push(l.to_ascii_uppercase()),
                _ => {
                    return Err(
                        format!("`{stem}`: `_` must be followed by a small letter or `_`").into(),
                    )
                }
            }
        } else if c.is_ascii_lowercase() || c.is_ascii_digit() {
            name.push(c);
        } else {
            return Err(
                format!("`{stem}`: write a capital letter as `_` and the small letter").into(),
            );
        }
    }
    if name.is_empty() {
        return Err("an empty file name".into());
    }
    Ok(name)
}

/// The text of an `include!` of a file, by its absolute path.
fn include(path: &Path) -> String {
    format!("include!({:?})", path.display().to_string())
}

fn differences(dir: &Path) -> Res<String> {
    let all = entries(dir, |stem| match stem.strip_suffix(".aliases") {
        Some(page) => plain(page).map(|p| format!("{p}.aliases")),
        None => plain(stem),
    })?;
    let (aliases, pages): (Vec<&Entry>, Vec<&Entry>) =
        all.iter().partition(|e| e.key.ends_with(".aliases"));
    let mut pages = pages;
    // the general page first
    match pages.iter().position(|e| e.key == "general") {
        Some(at) => {
            let general = pages.remove(at);
            pages.insert(0, general);
        }
        None => return Err(format!("{}: there is no general.rs", dir.display()).into()),
    }
    let mut text =
        String::from("// Written by build.rs from the files in `differences/`; do not edit.\n\n");
    text.push_str(
        "/// Every page, the general one first, then the rest in order of their file names.\n",
    );
    text.push_str("pub const GUIDES: &[Guide] = &[\n");
    for e in &pages {
        writeln!(text, "    {},", include(&e.path))?;
    }
    text.push_str("];\n\n");
    text.push_str("/// The file each page came from (without `.rs`), in the order of `GUIDES`.\n");
    text.push_str("#[cfg(test)]\nconst GUIDE_FILES: &[&str] = &[\n");
    for e in &pages {
        writeln!(text, "    {:?},", e.key)?;
    }
    text.push_str("];\n\n");
    text.push_str("/// The command names that share another command's page: `(page, names)`.\n");
    text.push_str("const ALIASES: &[(&str, &[&str])] = &[\n");
    for e in &aliases {
        let page = e.key.trim_end_matches(".aliases");
        writeln!(text, "    ({page:?}, {}),", include(&e.path))?;
    }
    text.push_str("];\n");
    Ok(text)
}

fn verified(dir: &Path) -> Res<String> {
    let commands = entries(dir, plain)?;
    let functions = entries(&dir.join("functions"), function_name)?;
    let mut text =
        String::from("// Written by build.rs from the files in `verified/`; do not edit.\n\n");
    text.push_str("/// Each Verified command, with the corpus files that check it.\n");
    text.push_str("pub const VERIFIED: &[(&str, &[&str])] = &[\n");
    for e in &commands {
        writeln!(text, "    ({:?}, {}),", e.key, include(&e.path))?;
    }
    text.push_str("];\n\n");
    text.push_str("/// Each Verified function, with the corpus files that check it (the\n");
    text.push_str("/// `functions:` line of each), where a function was the point of a file.\n");
    text.push_str("pub const VERIFIED_FUNCTIONS: &[(&str, &[&str])] = &[\n");
    for e in &functions {
        writeln!(text, "    ({:?}, {}),", e.key, include(&e.path))?;
    }
    text.push_str("];\n");
    Ok(text)
}

fn complete(dir: &Path) -> Res<String> {
    let commands = entries(dir, plain)?;
    let mut text =
        String::from("// Written by build.rs from the files in `complete/`; do not edit.\n\n");
    text.push_str("/// The commands the translator knows, the prefixes and blocks the reader\n");
    text.push_str(
        "/// takes, and what the do-file runner (`crate::dofile`) does itself, offered\n",
    );
    text.push_str("/// in command position.\n");
    text.push_str("pub const COMMANDS: &[&str] = &[\n");
    for e in &commands {
        writeln!(text, "    {:?},", e.key)?;
    }
    text.push_str("];\n\n");
    text.push_str("/// What Tab offers for each of `COMMANDS`, in the same order.\n");
    text.push_str("const COMMAND_TABLE: &[(&str, Command)] = &[\n");
    for e in &commands {
        writeln!(text, "    ({:?}, {}),", e.key, include(&e.path))?;
    }
    text.push_str("];\n");
    Ok(text)
}
