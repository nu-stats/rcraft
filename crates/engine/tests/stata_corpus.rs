//! P2B.10, checked against Stata: the corpus in `stata_corpus/`, made-up
//! data and do-files of our own, whose results Stata 19 printed and
//! `cargo xtask stata-record` kept (`NAME.recorded`). Each do-file is run
//! the way RCraft runs a do-file (P2C, `statatr::dofile`: its macros, loops
//! and included files, R asked for the values only it knows); every line
//! must translate, and each runs in a real R; every value it prints on an
//! `@ key value ...` line is compared with Stata's to the file's
//! tolerance (8 significant digits, or 6 where Stata stops iterating at its
//! tolerance). A command is Verified (`statatr::verified`) only through a
//! file here that passes.
//!
//! Without a supported R, or without haven, dplyr, readr, MASS, nnet,
//! sandwich and survival, the test is skipped with a note.

use std::collections::{BTreeMap, BTreeSet};
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

/// The corpus's own files, for `include`.
struct Corpus;

impl Files for Corpus {
    fn read(&self, path: &str) -> Option<String> {
        std::fs::read_to_string(path).ok()
    }
    fn exists(&self, path: &str) -> bool {
        Path::new(path).exists()
    }
}

/// Runs a do-file line by line in R, as the do-file window does: the
/// helpers a line needs first, then its R; a value only R knows is worked
/// out once the lines before it have run. What R printed, or the first
/// problem.
fn run_dofile(e: &mut Engine, folder: &str, text: &str) -> Result<String, String> {
    let (o, t) = run(e, &format!("setwd(\"{folder}\")"));
    if o != Outcome::Ok {
        return Err(t);
    }
    let opts = Options {
        name: "corpus".into(),
        folder: folder.to_string(),
        ..Options::default()
    };
    let mut r = Run::new(text, &Corpus, opts);
    let mut sent = BTreeSet::new();
    let mut printed = String::new();
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
                // every line the step made, in order
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
                    printed.push_str(&t);
                    code.clear();
                }
            }
            Event::Ask(v) => {
                code.push_str(&v.code);
                let (o, t) = run(e, &code);
                r.give(if o == Outcome::Ok { Ok(t) } else { Err(t) });
            }
            Event::Done => return Ok(printed),
        }
    }
}

/// The `@ key value ...` lines of a do-file's output, every value as a
/// number (`.` is missing).
fn results(text: &str) -> BTreeMap<String, Vec<f64>> {
    text.lines()
        .filter_map(|l| {
            let mut w = l.strip_prefix("@ ")?.split_whitespace();
            let key = w.next()?;
            let v: Option<Vec<f64>> = w
                .map(|v| {
                    if v == "." {
                        Some(f64::NAN)
                    } else {
                        v.parse().ok()
                    }
                })
                .collect();
            let v = v.filter(|v| !v.is_empty())?;
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
    // RCRAFT_CORPUS_ONLY runs the files whose names hold its text
    if let Ok(only) = std::env::var("RCRAFT_CORPUS_ONLY") {
        names.retain(|n| n.contains(only.as_str()));
    }
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
        let folder = dir.display().to_string().replace('\\', "/");
        let text = match run_dofile(&mut e, &folder, &dofile) {
            Ok(t) => t,
            Err(why) => panic!("{name}: {why}"),
        };
        let got = results(&text);
        for (key, want) in &stata {
            let Some(have) = got.get(key) else {
                wrong.push(format!("{name}: {key} was not printed"));
                continue;
            };
            if have.len() != want.len() {
                wrong.push(format!(
                    "{name}: {key} has {} values, Stata's {}",
                    have.len(),
                    want.len()
                ));
                continue;
            }
            for (i, (have, want)) in have.iter().zip(want).enumerate() {
                let same = (want.is_nan() && have.is_nan())
                    || (have - want).abs() <= tol * want.abs().max(have.abs());
                if !same {
                    wrong.push(format!(
                        "{name}: {key} value {} is {have}, Stata's {want} (tolerance {tol})",
                        i + 1
                    ));
                }
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// Verified is earned here: every command (function) `statatr::verified`
/// names is in the `commands:` (`functions:`) of a corpus file, and every
/// one named there is in its list.
#[test]
fn verified_is_what_the_corpus_checks() {
    use rcraft_statatr::verified::{VERIFIED, VERIFIED_FUNCTIONS};
    for (head, list) in [("commands", VERIFIED), ("functions", VERIFIED_FUNCTIONS)] {
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
                for one in header(&text, head).unwrap_or("").split_whitespace() {
                    checked
                        .entry(one.to_string())
                        .or_default()
                        .push(name.clone());
                }
            }
        }
        for files in checked.values_mut() {
            files.sort();
        }
        let verified: BTreeMap<String, Vec<String>> = list
            .iter()
            .map(|(c, f)| (c.to_string(), f.iter().map(|s| s.to_string()).collect()))
            .collect();
        assert_eq!(verified, checked, "{head}");
    }
}
