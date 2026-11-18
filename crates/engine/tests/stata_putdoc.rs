//! P2C.12x (batch D): the files `putdocx`, `putpdf`, `dyntext`, `dyndoc` and
//! `markdown` write, read back. The corpus files `stata_corpus/putdocx.do` and
//! `stata_corpus/dyndoc.do` compare the messages and return codes with Stata
//! 19's; this test runs the same do-files in a folder of their own (without
//! their `erase` lines) and compares the files they write with the ones Stata 19
//! wrote for them:
//!
//! - Word and PDF: what the files hold, as `stata_putdoc/dump.R` reads it
//!   (paragraph styles and alignment, runs with their bold, italic, underline and
//!   line break, tables with the text, spans and alignment of each cell, the text
//!   of each PDF page), against `stata_putdoc/*.txt`: the structure, the numbers
//!   as written, and any other text only as its length and a checksum, so that
//!   no text Stata wrote is kept (`AGENTS.md`, section 1). Made from made-up data.
//! - Text and HTML: the files exactly, by their MD5 digests (Stata's files are
//!   not kept). The text is compared without carriage returns: Stata 19 on
//!   Windows ends the lines a `dd_do` command printed with one, which RCraft
//!   writes on Windows only (Stata on Linux and macOS was not observed).
//!
//! Skipped with a note without a supported R or the zip package.

#![allow(clippy::panic)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use rcraft_engine::{Engine, Item, Outcome, Source};
use rcraft_rhost::{find_installs, Env, SessionConfig};
use rcraft_statatr::dofile::{Event, Files, Options, Run};

fn engine() -> Option<Engine> {
    let install = find_installs(&Env::from_process())
        .into_iter()
        .find(|i| i.is_supported());
    let Some(install) = install else {
        eprintln!("skipped: no supported R on this machine");
        return None;
    };
    let mut e = Engine::start(SessionConfig::new(install)).ok()?;
    let (have, _) = run(
        &mut e,
        "stopifnot(requireNamespace(\"dplyr\", quietly = TRUE), requireNamespace(\"haven\", quietly = TRUE), \
         requireNamespace(\"zip\", quietly = TRUE))",
    );
    if have != Outcome::Ok {
        eprintln!("skipped: this test needs dplyr, haven and zip");
        return None;
    }
    Some(e)
}

fn run(e: &mut Engine, code: &str) -> (Outcome, String) {
    let Ok(id) = e.submit(Source::Typed, code) else {
        return (Outcome::Error, String::new());
    };
    let limit = Instant::now() + Duration::from_secs(300);
    loop {
        e.poll(Duration::from_millis(50));
        if !e.is_working() || Instant::now() > limit {
            break;
        }
    }
    let outcome = e
        .history()
        .entries()
        .last()
        .and_then(|h| h.outcome)
        .unwrap_or(Outcome::Error);
    let text = e
        .results()
        .blocks()
        .iter()
        .find(|b| b.command == id)
        .map(|b| {
            b.items
                .iter()
                .map(|i| match i {
                    Item::Text { text, .. } => text.clone(),
                    Item::Warning(w) => format!("warning: {w}\n"),
                    Item::Error { message, .. } => format!("error: {message}\n"),
                    _ => String::new(),
                })
                .collect::<String>()
        })
        .unwrap_or_default();
    (outcome, text)
}

struct Disk;

impl Files for Disk {
    fn read(&self, path: &str) -> Option<String> {
        std::fs::read_to_string(path).ok()
    }
    fn exists(&self, path: &str) -> bool {
        Path::new(path).exists()
    }
}

/// Runs a do-file line by line in R, as the do-file window does.
fn run_dofile(e: &mut Engine, folder: &str, text: &str) -> Result<(), String> {
    let (o, t) = run(e, &format!("setwd(\"{folder}\")"));
    if o != Outcome::Ok {
        return Err(t);
    }
    let opts = Options {
        name: "putdoc".into(),
        folder: folder.to_string(),
        ..Options::default()
    };
    let mut r = Run::new(text, &Disk, opts);
    let mut sent = BTreeSet::new();
    let mut handled = 0;
    loop {
        let event = r.next_event();
        let mut code = String::new();
        for (name, h) in r.helpers() {
            if sent.insert(name) {
                code.push_str(h);
                code.push('\n');
            }
        }
        match event {
            Event::Line => {
                let lines = r.lines().get(handled..).unwrap_or_default().to_vec();
                handled += lines.len();
                for line in lines {
                    if let Err(why) = &line.result {
                        return Err(format!("not translated: {}\n{why:?}", line.stata));
                    }
                    code.push_str(&line.script());
                    let (o, t) = run(e, &code);
                    if o != Outcome::Ok {
                        return Err(format!("{}\n{code}\n{t}", line.stata));
                    }
                    code.clear();
                }
            }
            Event::Ask(v) => {
                code.push_str(&v.code);
                let (o, t) = run(e, &code);
                r.give(if o == Outcome::Ok { Ok(t) } else { Err(t) });
            }
            Event::Done => return Ok(()),
        }
    }
}

fn here(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join(name)
}

fn trimmed(text: &str) -> Vec<String> {
    text.lines().map(|l| l.trim_end().to_string()).collect()
}

#[test]
fn the_files_hold_what_statas_hold() {
    let Some(mut e) = engine() else { return };
    let dofile = std::fs::read_to_string(here("stata_corpus/putdocx.do")).unwrap_or_default();
    assert!(!dofile.is_empty(), "stata_corpus/putdocx.do is missing");
    // the files stay for reading back
    let text: String = dofile
        .lines()
        .filter(|l| !l.starts_with("erase "))
        .map(|l| format!("{l}\n"))
        .collect();
    let work = std::env::temp_dir().join(format!("rcraft-putdoc-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&work);
    assert!(std::fs::create_dir_all(&work).is_ok());
    let folder = work.display().to_string().replace('\\', "/");
    let ran = run_dofile(&mut e, &folder, &text);
    let dump = std::fs::read_to_string(here("stata_putdoc/dump.R")).unwrap_or_default();
    let mut wrong = Vec::new();
    if let Err(why) = ran {
        wrong.push(format!("putdocx.do: {why}"));
    } else {
        for (file, reader, expected) in [
            ("pdx_a.docx", "stata_dump_docx", "pdx_a.txt"),
            ("pdx_b.docx", "stata_dump_docx", "pdx_b.txt"),
            ("pdf_a.pdf", "stata_dump_pdf", "pdf_a.txt"),
        ] {
            let want = std::fs::read_to_string(here(&format!("stata_putdoc/{expected}")))
                .unwrap_or_default();
            let (o, got) = run(
                &mut e,
                &format!("{dump}\ncat({reader}(\"{folder}/{file}\"), \"\\n\", sep = \"\")"),
            );
            if o != Outcome::Ok {
                wrong.push(format!("{file}: could not be read back: {got}"));
                continue;
            }
            let (got, want) = (trimmed(&got), trimmed(&want));
            for (i, w) in want.iter().enumerate() {
                if got.get(i) != Some(w) {
                    wrong.push(format!(
                        "{file} line {}: RCraft's {:?}, Stata's {w:?}",
                        i + 1,
                        got.get(i)
                    ));
                    break;
                }
            }
            if got.len() != want.len() {
                wrong.push(format!(
                    "{file}: {} lines, Stata's {}; RCraft's end: {:?}",
                    got.len(),
                    want.len(),
                    got.get(want.len().saturating_sub(2)..)
                ));
            }
        }
    }
    let _ = std::fs::remove_dir_all(&work);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// MD5 digests of the files Stata 19 wrote for `stata_corpus/dyndoc.do`
/// (2026-11-18), the text file without its carriage returns.
const DYNDOC_DIGESTS: &[(&str, &str)] = &[
    ("ddx_out.txt", "7631a0bf16a06f976fa0c3176cbf3d16"),
    ("ddx_out.html", "92d6ae3216c3209ccb5ae7d224255429"),
    ("ddx_md.html", "ada4c6a0d26df5224a43bf4363899998"),
];

#[test]
fn the_text_and_html_files_are_statas() {
    let Some(mut e) = engine() else { return };
    let dofile = std::fs::read_to_string(here("stata_corpus/dyndoc.do")).unwrap_or_default();
    assert!(!dofile.is_empty(), "stata_corpus/dyndoc.do is missing");
    let text: String = dofile
        .lines()
        .filter(|l| !l.starts_with("erase "))
        .map(|l| format!("{l}\n"))
        .collect();
    let work = std::env::temp_dir().join(format!("rcraft-dyndoc-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&work);
    assert!(std::fs::create_dir_all(&work).is_ok());
    let folder = work.display().to_string().replace('\\', "/");
    let mut wrong = Vec::new();
    match run_dofile(&mut e, &folder, &text) {
        Err(why) => wrong.push(format!("dyndoc.do: {why}")),
        Ok(()) => {
            for (file, want) in DYNDOC_DIGESTS {
                let (o, got) = run(
                    &mut e,
                    &format!(
                        "local({{ p <- \"{folder}/{file}\"; b <- readBin(p, \"raw\", file.info(p)$size); \
                         b <- b[b != as.raw(13)]; t <- tempfile(); writeBin(b, t); \
                         cat(unname(tools::md5sum(t)), \"\\n\", sep = \"\"); unlink(t) }})"
                    ),
                );
                let got = got.trim();
                if o != Outcome::Ok || got != *want {
                    wrong.push(format!(
                        "{file}: RCraft's file has the digest {got}, Stata's {want}"
                    ));
                }
            }
        }
    }
    let _ = std::fs::remove_dir_all(&work);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
