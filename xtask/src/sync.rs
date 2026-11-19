//! `cargo xtask sync-statcraft-lang`: keep the vendored copy of StatCraft's
//! do-file front end (`crates/statatr/src/lang` and `values`) in step with
//! StatCraft, and make a hand edit of a vendored file fail CI.
//!
//! StatCraft is our own project but a private one, so RCraft's public CI
//! cannot see it. The sync therefore writes a manifest (the StatCraft commit
//! and a SHA-256 of every file as written), and `cargo xtask ci` only checks
//! that the files still match the manifest. Comparing against StatCraft
//! itself (`--diff`) can only run on a machine that has its checkout.
//!
//! ```text
//! cargo xtask sync-statcraft-lang           copy from StatCraft's origin/main, write the manifest
//! cargo xtask sync-statcraft-lang --check   files against the manifest (what ci runs)
//! cargo xtask sync-statcraft-lang --diff    files against StatCraft's origin/main (local only)
//! ```
//!
//! The StatCraft checkout is `STATCRAFT_DIR`, else `C:/pinokio/api/statcraft`,
//! else `../statcraft` beside the RCraft checkout; `STATCRAFT_REF` replaces
//! `origin/main`. Nothing is fetched: run `git fetch` there first.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{root, Result};

/// Where the manifest lives, relative to the workspace root.
const MANIFEST: &str = "crates/statatr/src/lang/SYNC_MANIFEST";
/// The folder every manifest path is relative to.
const DEST_ROOT: &str = "crates/statatr/src";
const DEFAULT_REF: &str = "origin/main";

/// One vendored file: where it comes from in StatCraft and where it goes.
struct Item {
    source: &'static str,
    dest: &'static str,
}

/// RCraft's own `lang/mod.rs`, `values/mod.rs` and `lang/commands.rs` are not
/// vendored: the first two are RCraft's module lists, and the command table
/// is kept here (StatCraft's lives in its engine, not in a parser crate).
const ITEMS: &[Item] = &[
    Item {
        source: "crates/lang/src/expr.rs",
        dest: "lang/expr.rs",
    },
    Item {
        source: "crates/lang/src/script.rs",
        dest: "lang/script.rs",
    },
    Item {
        source: "crates/lang/src/syntax.rs",
        dest: "lang/syntax.rs",
    },
    Item {
        source: "crates/core/src/missing.rs",
        dest: "values/missing.rs",
    },
    Item {
        source: "crates/core/src/hex.rs",
        dest: "values/hex.rs",
    },
    Item {
        source: "crates/core/src/datetime/mod.rs",
        dest: "values/datetime/mod.rs",
    },
    Item {
        source: "crates/core/src/datetime/parse.rs",
        dest: "values/datetime/parse.rs",
    },
    Item {
        source: "crates/core/src/datetime/leap.rs",
        dest: "values/datetime/leap.rs",
    },
];

pub fn run(args: &[String]) -> Result {
    match args.first().map(String::as_str) {
        None => sync(),
        Some("--check") => check(),
        Some("--diff") => diff(),
        Some(other) => Err(format!(
            "unknown option {other}; use --check, --diff or no option"
        )),
    }
}

/// The file as RCraft keeps it: rewritten, then formatted with the workspace's
/// rustfmt (longer paths wrap lines), so that `cargo fmt` never changes it.
fn render(item: &Item, source: &str, commit: &str) -> std::result::Result<String, String> {
    rustfmt(&rewrite(item, source, commit))
}

/// The path rewrites and struck lines that turn a StatCraft file into the
/// RCraft one, then the header line that says where it came from.
fn rewrite(item: &Item, source: &str, commit: &str) -> String {
    let mut body = String::new();
    for line in source.replace("\r\n", "\n").lines() {
        // The `display` part of the date code (date formats) is not vendored.
        if item.dest == "values/datetime/mod.rs"
            && (line.trim() == "mod display;" || line.trim_start().starts_with("pub use display::"))
        {
            continue;
        }
        body.push_str(
            &line
                .replace("statcraft_core::", "crate::values::")
                .replace("crate::missing::", "crate::values::missing::")
                .replace("crate::hex::", "crate::values::hex::")
                .replace("crate::datetime::", "crate::values::datetime::"),
        );
        body.push('\n');
    }
    let short = commit.get(..12).unwrap_or(commit);
    format!(
        "// Vendored from StatCraft {} at {short}. Do not edit: run\n\
         // `cargo xtask sync-statcraft-lang` (see the `lib.rs` of this crate).\n{body}",
        item.source
    )
}

fn sync() -> Result {
    let dir = statcraft_dir()?;
    let reference = std::env::var("STATCRAFT_REF").unwrap_or_else(|_| DEFAULT_REF.into());
    let commit = git(&dir, &["rev-parse", &reference])?.trim().to_string();
    let dest_root = root().join(DEST_ROOT);
    let mut manifest = format!(
        "# Written by `cargo xtask sync-statcraft-lang`; do not edit.\n\
         # `cargo xtask ci` re-hashes these files and fails if one differs.\n\
         statcraft_commit {commit}\n"
    );
    for item in ITEMS {
        let source = git(&dir, &["show", &format!("{commit}:{}", item.source)])?;
        let text = render(item, &source, &commit)?;
        let path = dest_root.join(item.dest);
        let changed = fs::read_to_string(&path).map_or(true, |old| old != text);
        if changed {
            fs::write(&path, &text).map_err(|e| format!("{}: {e}", path.display()))?;
        }
        manifest.push_str(&format!("{}  {}\n", sha256_hex(&text), item.dest));
        println!(
            "{} {}",
            if changed { "wrote    " } else { "unchanged" },
            item.dest
        );
    }
    let manifest_path = root().join(MANIFEST);
    fs::write(&manifest_path, manifest).map_err(|e| format!("{}: {e}", manifest_path.display()))?;
    println!("sync-statcraft-lang: {} files at {}", ITEMS.len(), &commit);
    Ok(())
}

/// What the manifest says: the StatCraft commit and `(hash, path)` rows.
fn read_manifest() -> std::result::Result<(String, Vec<(String, String)>), String> {
    let path = root().join(MANIFEST);
    let text = fs::read_to_string(&path).map_err(|e| {
        format!(
            "could not read {MANIFEST}: {e}; run `cargo xtask sync-statcraft-lang` on a machine with StatCraft"
        )
    })?;
    let mut commit = String::new();
    let mut rows = Vec::new();
    for line in text.lines() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        if let Some(c) = line.strip_prefix("statcraft_commit ") {
            commit = c.trim().to_string();
        } else if let Some((hash, file)) = line.split_once("  ") {
            rows.push((hash.to_string(), file.trim().to_string()));
        } else {
            return Err(format!("{MANIFEST}: cannot read the line `{line}`"));
        }
    }
    Ok((commit, rows))
}

/// The files against the manifest only (this is what `cargo xtask ci` runs).
pub fn check() -> Result {
    let (commit, rows) = read_manifest()?;
    let dest_root = root().join(DEST_ROOT);
    let mut bad = Vec::new();
    for item in ITEMS {
        if !rows.iter().any(|(_, f)| f == item.dest) {
            bad.push(format!("{}: not in the manifest", item.dest));
        }
    }
    for (hash, file) in &rows {
        match fs::read_to_string(dest_root.join(file)) {
            Ok(text) if sha256_hex(&text) == *hash => {}
            Ok(_) => bad.push(format!("{file}: differs from the manifest")),
            Err(e) => bad.push(format!("{file}: {e}")),
        }
    }
    if bad.is_empty() {
        println!(
            "sync-statcraft-lang: {} vendored files match the manifest (StatCraft {})",
            rows.len(),
            commit.get(..12).unwrap_or(&commit)
        );
        Ok(())
    } else {
        Err(format!(
            "vendored StatCraft files were edited by hand or the manifest is stale:\n  {}\n\
             Fix the bug in StatCraft and run `cargo xtask sync-statcraft-lang`, never in these files.",
            bad.join("\n  ")
        ))
    }
}

/// The files against what a sync of StatCraft's `origin/main` would write.
fn diff() -> Result {
    let dir = statcraft_dir()?;
    let reference = std::env::var("STATCRAFT_REF").unwrap_or_else(|_| DEFAULT_REF.into());
    let commit = git(&dir, &["rev-parse", &reference])?.trim().to_string();
    let (recorded, _) = read_manifest()?;
    let dest_root = root().join(DEST_ROOT);
    let mut differing = Vec::new();
    for item in ITEMS {
        let source = git(&dir, &["show", &format!("{commit}:{}", item.source)])?;
        let want = render(item, &source, &recorded)?;
        let have = fs::read_to_string(dest_root.join(item.dest)).unwrap_or_default();
        if have != want {
            differing.push(item.dest);
        }
    }
    if differing.is_empty() {
        println!("sync-statcraft-lang: in step with StatCraft {reference} ({commit})");
        Ok(())
    } else {
        Err(format!(
            "behind StatCraft {reference} ({commit}); manifest says {recorded}; differing: {}\n\
             Run `cargo xtask sync-statcraft-lang`.",
            differing.join(", ")
        ))
    }
}

fn rustfmt(text: &str) -> std::result::Result<String, String> {
    use std::io::Write;
    use std::process::Stdio;
    let mut child = Command::new("rustfmt")
        .args(["--edition", "2021", "--config-path"])
        .arg(root())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("could not run rustfmt: {e}"))?;
    let mut stdin = child.stdin.take().ok_or("no stdin for rustfmt")?;
    let input = text.to_string();
    let writer = std::thread::spawn(move || stdin.write_all(input.as_bytes()));
    let out = child
        .wait_with_output()
        .map_err(|e| format!("rustfmt failed: {e}"))?;
    let _ = writer.join();
    if out.status.success() {
        String::from_utf8(out.stdout).map_err(|e| format!("rustfmt output is not UTF-8: {e}"))
    } else {
        Err(format!(
            "rustfmt failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ))
    }
}

fn statcraft_dir() -> std::result::Result<PathBuf, String> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(dir) = std::env::var("STATCRAFT_DIR") {
        candidates.push(PathBuf::from(dir));
    }
    candidates.push(PathBuf::from("C:/pinokio/api/statcraft"));
    if let Some(parent) = root().parent() {
        candidates.push(parent.join("statcraft"));
    }
    candidates
        .into_iter()
        .find(|p| p.join(".git").exists())
        .ok_or_else(|| "no StatCraft checkout found; set STATCRAFT_DIR".to_string())
}

fn git(dir: &Path, args: &[&str]) -> std::result::Result<String, String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .map_err(|e| format!("could not run git: {e}"))?;
    if out.status.success() {
        String::from_utf8(out.stdout).map_err(|e| format!("git output is not UTF-8: {e}"))
    } else {
        Err(format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        ))
    }
}

/// SHA-256 of the text, with line ends as LF so a checkout with CR LF hashes
/// the same.
fn sha256_hex(text: &str) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let mut data: Vec<u8> = text.replace("\r\n", "\n").into_bytes();
    let bit_len = (data.len() as u64).wrapping_mul(8);
    data.push(0x80);
    while data.len() % 64 != 56 {
        data.push(0);
    }
    data.extend_from_slice(&bit_len.to_be_bytes());
    for block in data.chunks(64) {
        let mut w = [0u32; 64];
        for (i, word) in block.chunks(4).enumerate() {
            let mut bytes = [0u8; 4];
            bytes.copy_from_slice(word);
            if let Some(slot) = w.get_mut(i) {
                *slot = u32::from_be_bytes(bytes);
            }
        }
        for i in 16..64 {
            let at = |k: usize| w.get(k).copied().unwrap_or(0);
            let s0 = at(i - 15).rotate_right(7) ^ at(i - 15).rotate_right(18) ^ (at(i - 15) >> 3);
            let s1 = at(i - 2).rotate_right(17) ^ at(i - 2).rotate_right(19) ^ (at(i - 2) >> 10);
            let v = at(i - 16)
                .wrapping_add(s0)
                .wrapping_add(at(i - 7))
                .wrapping_add(s1);
            if let Some(slot) = w.get_mut(i) {
                *slot = v;
            }
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for (i, k) in K.iter().enumerate() {
            let wi = w.get(i).copied().unwrap_or(0);
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(*k)
                .wrapping_add(wi);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (slot, add) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
            *slot = slot.wrapping_add(add);
        }
    }
    h.iter().map(|v| format!("{v:08x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_known_answers() {
        assert_eq!(
            sha256_hex(""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        // two blocks
        assert_eq!(
            sha256_hex("abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
        assert_eq!(sha256_hex("a\r\nb"), sha256_hex("a\nb"));
    }

    #[test]
    fn render_rewrites_paths_and_strikes_display() {
        let item = Item {
            source: "crates/core/src/datetime/mod.rs",
            dest: "values/datetime/mod.rs",
        };
        let text = rewrite(
            &item,
            "mod display;\npub use display::{DateFormat, Unit};\nuse crate::missing::is_missing;\nuse statcraft_core::hex::from_hex;\n",
            "0123456789abcdef0123",
        );
        assert!(!text.contains("display"));
        assert!(text.contains("use crate::values::missing::is_missing;"));
        assert!(text.contains("use crate::values::hex::from_hex;"));
        assert!(text.starts_with(
            "// Vendored from StatCraft crates/core/src/datetime/mod.rs at 0123456789ab."
        ));
    }
}
