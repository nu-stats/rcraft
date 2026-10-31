//! P2B.10, checked against Stata: the corpus in `stata_corpus/`, made-up
//! data and do-files of our own, whose results Stata 19 printed and
//! `cargo xtask stata-record` kept (`NAME.recorded`). Each do-file is
//! translated, every line of it must be, and run in a real R; every result
//! it prints as `@ key value` is compared with Stata's to the file's
//! tolerance (8 significant digits, or 6 where Stata stops iterating at its
//! tolerance). A command is Verified (`statatr::verified`) only through a
//! file here that passes.
//!
//! Without a supported R, or without haven, dplyr, readr, MASS, nnet,
//! sandwich and survival, the test is skipped with a note.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use rcraft_engine::{Engine, Item, Outcome, Source};
use rcraft_rhost::{find_installs, Env, SessionConfig};
use rcraft_statatr::data_r::translate;
use rcraft_statatr::reader::read;

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
         requireNamespace(\"readr\", quietly = TRUE), requireNamespace(\"MASS\", quietly = TRUE), \
         requireNamespace(\"nnet\", quietly = TRUE), requireNamespace(\"sandwich\", quietly = TRUE), \
         requireNamespace(\"survival\", quietly = TRUE))",
    );
    if have != Outcome::Ok {
        eprintln!(
            "skipped: the corpus needs dplyr, haven, readr, MASS, nnet, sandwich and survival"
        );
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

fn corpus() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("stata_corpus")
}

/// The `@ key value` lines of a do-file's output, as numbers.
fn results(text: &str) -> BTreeMap<String, f64> {
    text.lines()
        .filter_map(|l| {
            let mut w = l.strip_prefix("@ ")?.split_whitespace();
            let key = w.next()?;
            let v = w.next()?;
            let v = if v == "." { f64::NAN } else { v.parse().ok()? };
            Some((key.to_string(), v))
        })
        .collect()
}

/// A header line of the do-file: `* name: value`.
fn header<'a>(dofile: &'a str, name: &str) -> Option<&'a str> {
    dofile
        .lines()
        .find_map(|l| l.strip_prefix(&format!("* {name}: ")))
}

#[test]
fn the_corpus_gives_statas_results() {
    let Some(mut e) = engine() else { return };
    let dir = corpus();
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .map(|r| {
            r.filter_map(|e| e.ok())
                .filter_map(|e| {
                    let p = e.path();
                    (p.extension()? == "do").then(|| p.file_stem()?.to_str().map(str::to_string))?
                })
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    assert!(names.len() >= 5, "the corpus is in {}", dir.display());
    let mut wrong = Vec::new();
    for name in &names {
        let dofile = std::fs::read_to_string(dir.join(format!("{name}.do"))).unwrap_or_default();
        let recorded =
            std::fs::read_to_string(dir.join(format!("{name}.recorded"))).unwrap_or_default();
        let stata = results(&recorded);
        assert!(
            !stata.is_empty(),
            "{name}: nothing recorded; run cargo xtask stata-record"
        );
        let tol: f64 = header(&dofile, "tolerance")
            .and_then(|t| t.trim().parse().ok())
            .unwrap_or(1e-8);
        let t = translate(&read(&dofile));
        for l in &t.lines {
            assert!(
                l.result.is_ok(),
                "{name}: not translated: {}\n{:?}",
                l.stata,
                l.result
            );
        }
        let folder = dir.display().to_string().replace('\\', "/");
        let code = format!("setwd(\"{folder}\")\n{}", t.script());
        let (outcome, text) = run(&mut e, &code);
        assert_eq!(outcome, Outcome::Ok, "{name}:\n{text}");
        let got = results(&text);
        for (key, want) in &stata {
            match got.get(key) {
                None => wrong.push(format!("{name}: {key} was not printed")),
                Some(have) => {
                    let same = (want.is_nan() && have.is_nan())
                        || (have - want).abs() <= tol * want.abs().max(have.abs());
                    if !same {
                        wrong.push(format!(
                            "{name}: {key} is {have}, Stata's {want} (tolerance {tol})"
                        ));
                    }
                }
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// Verified is earned here: every command `statatr::verified` names is in the
/// `commands:` of a corpus file, and every such command is named there.
#[test]
fn verified_is_what_the_corpus_checks() {
    let dir = corpus();
    let mut checked: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for entry in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
        let p = entry.path();
        if p.extension().is_some_and(|x| x == "do") {
            let name = p
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            let text = std::fs::read_to_string(&p).unwrap_or_default();
            for cmd in header(&text, "commands").unwrap_or("").split_whitespace() {
                checked
                    .entry(cmd.to_string())
                    .or_default()
                    .push(name.clone());
            }
        }
    }
    for files in checked.values_mut() {
        files.sort();
    }
    let verified: BTreeMap<String, Vec<String>> = rcraft_statatr::verified::VERIFIED
        .iter()
        .map(|(c, f)| (c.to_string(), f.iter().map(|s| s.to_string()).collect()))
        .collect();
    assert_eq!(verified, checked);
}
