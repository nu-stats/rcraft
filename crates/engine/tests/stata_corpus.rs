//! P2B.10, checked against Stata: the corpus in `stata_corpus/`, made-up
//! data and do-files of our own, whose results Stata 19 printed and
//! `cargo xtask stata-record` kept (`NAME.recorded`). Each do-file is run
//! the way RCraft runs a do-file (P2C, `statatr::dofile`: its macros, loops
//! and included files, R asked for the values only it knows); every line
//! must translate, and each runs in a real R; every value it prints on an
//! `@ key value ...` line, the n-th time a key is printed against the n-th
//! time Stata printed it, is compared with Stata's token by token: a number
//! to the file's tolerance (8 significant digits, or 6 where Stata stops
//! iterating at its tolerance), anything else (`.`, `.a`, a word) exactly.
//! A recorded line that cannot be read or compared fails the test; none is
//! skipped. A command is Verified (`statatr::verified`) only through a
//! file here that passes.
//!
//! Without a supported R, or without haven, dplyr, readr, MASS, nnet,
//! sandwich, survival and openxlsx2, the test is skipped with a note.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
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
         requireNamespace(\"survival\", quietly = TRUE), \
         requireNamespace(\"openxlsx2\", quietly = TRUE))",
    );
    if have != Outcome::Ok {
        eprintln!(
            "skipped: the corpus needs dplyr, haven, readr, MASS, nnet, sandwich, survival and openxlsx2"
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

/// One value on an `@` line: a number, compared to the tolerance, or any
/// other token (`.`, `.a` to `.z`, a word, a quoted string), compared as
/// exactly the same text.
#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Text(String),
}

impl std::fmt::Display for Tok {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Tok::Num(n) => write!(f, "{n}"),
            Tok::Text(t) => write!(f, "`{t}`"),
        }
    }
}

/// One `@ key value ...` line, with its line number in the text it came from.
#[derive(Debug, Clone, PartialEq)]
struct Row {
    line: usize,
    key: String,
    values: Vec<Tok>,
}

/// A `%21x` hex float as Stata prints it (`-1.8c3c9eecbfb16X-002`: a sign,
/// `1.` or `0.` and hex digits, `X`, a sign and the exponent in hex). `None`
/// if the token does not start that way (`[01].`, then hex digits and an
/// `X`); an error if it does but is not a whole hex float. The values with
/// exponent 3ff are Stata's missing codes (`.`, `.a`, ...): those stay text,
/// compared exactly.
fn hex_float(t: &str) -> Option<Result<Tok, String>> {
    let body = t.strip_prefix(['+', '-']).unwrap_or(t);
    let (int, rest) = (body.get(..2)?, body.get(2..)?);
    if int != "1." && int != "0." {
        return None;
    }
    let x = rest.find('X')?;
    let (frac, exp) = (rest.get(..x)?, rest.get(x + 1..)?);
    if !frac.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let bad = || Some(Err(format!("`{t}` is not a hex float")));
    let Some(exp_digits) = exp.strip_prefix(['+', '-']) else {
        return bad();
    };
    if frac.is_empty()
        || frac.len() > 13
        || exp_digits.is_empty()
        || exp_digits.len() > 3
        || !exp_digits.chars().all(|c| c.is_ascii_hexdigit())
    {
        return bad();
    }
    let (Ok(m), Ok(e)) = (
        u64::from_str_radix(frac, 16),
        i32::from_str_radix(exp_digits, 16),
    ) else {
        return bad();
    };
    if e == 0x3ff && exp.starts_with('+') {
        return Some(Ok(Tok::Text(t.to_string())));
    }
    let e = if exp.starts_with('-') { -e } else { e };
    let lead = if int == "1." { 1.0 } else { 0.0 };
    let value = (lead + m as f64 / 16f64.powi(frac.len() as i32)) * 2f64.powi(e);
    Some(Ok(Tok::Num(if t.starts_with('-') {
        -value
    } else {
        value
    })))
}

/// One token of an `@` line. Text unless it is a number (or a hex float, see
/// `hex_float`): only digits, signs,
/// a point and an exponent, so `inf`, `nan` and `.a` stay text. A token that
/// is made of those characters but is not a number (`1.2.3`, `1e`) cannot be
/// read, and is an error rather than a skipped line.
fn token(t: &str) -> Result<Tok, String> {
    if let Some(hex) = hex_float(t) {
        return hex;
    }
    let numeric_chars = t.chars().all(|c| c.is_ascii_digit() || "+-.eE".contains(c));
    let unsigned = t.trim_start_matches(['+', '-']);
    let digit_led = unsigned.starts_with(|c: char| c.is_ascii_digit())
        || unsigned
            .strip_prefix('.')
            .is_some_and(|r| r.starts_with(|c: char| c.is_ascii_digit()));
    if numeric_chars && digit_led {
        return t
            .parse()
            .map(Tok::Num)
            .map_err(|_| format!("`{t}` is not a number"));
    }
    Ok(Tok::Text(t.to_string()))
}

/// The `@ key value ...` lines of a do-file's output, in order. A line that
/// cannot be read is an error naming its line and token; no line is dropped.
fn results(text: &str) -> Result<Vec<Row>, String> {
    let mut out = Vec::new();
    for (i, l) in text.lines().enumerate() {
        let Some(rest) = l.strip_prefix("@ ").or((l.trim_end() == "@").then_some("")) else {
            continue;
        };
        let mut w = rest.split_whitespace();
        let Some(key) = w.next() else {
            return Err(format!("line {}: `@` line without a key", i + 1));
        };
        let values = w
            .map(|t| token(t).map_err(|why| format!("line {}: token {why}", i + 1)))
            .collect::<Result<Vec<_>, _>>()?;
        out.push(Row {
            line: i + 1,
            key: key.to_string(),
            values,
        });
    }
    Ok(out)
}

/// Compares every line Stata recorded with RCraft's: the n-th time a key was
/// printed with the n-th time RCraft printed it, token by token, numbers to
/// the tolerance and everything else exactly. What differs goes to `wrong`.
fn compare(name: &str, stata: &[Row], got: &[Row], tol: f64, wrong: &mut Vec<String>) {
    let mut seen: BTreeMap<&str, usize> = BTreeMap::new();
    for want in stata {
        let n = seen.entry(want.key.as_str()).or_insert(0);
        let nth = *n;
        *n += 1;
        let Some(have) = got.iter().filter(|g| g.key == want.key).nth(nth) else {
            wrong.push(format!(
                "{name}: {} (recorded line {}) was not printed{}",
                want.key,
                want.line,
                if nth > 0 {
                    format!(" a {}th time", nth + 1)
                } else {
                    String::new()
                }
            ));
            continue;
        };
        if have.values.len() != want.values.len() {
            wrong.push(format!(
                "{name}: {} (recorded line {}) has {} values, Stata's {}",
                want.key,
                want.line,
                have.values.len(),
                want.values.len()
            ));
            continue;
        }
        for (i, (have, want_v)) in have.values.iter().zip(&want.values).enumerate() {
            let same = match (have, want_v) {
                (Tok::Num(h), Tok::Num(w)) => (h - w).abs() <= tol * w.abs().max(h.abs()),
                (h, w) => h == w,
            };
            if !same {
                wrong.push(format!(
                    "{name}: {} (recorded line {}) value {} is {have}, Stata's {want_v} (tolerance {tol})",
                    want.key,
                    want.line,
                    i + 1
                ));
            }
        }
    }
    // a key RCraft printed more often than Stata did is a difference too
    for key in seen.keys() {
        let count = |l: &[Row]| l.iter().filter(|x| x.key == *key).count();
        if count(got) > count(stata) {
            wrong.push(format!(
                "{name}: {key} was printed {} times, Stata's {}",
                count(got),
                count(stata)
            ));
        }
    }
}

/// A header line of the do-file: `* name: value`.
fn header<'a>(dofile: &'a str, name: &str) -> Option<&'a str> {
    dofile
        .lines()
        .find_map(|l| l.strip_prefix(&format!("* {name}: ")))
}

/// Runs one corpus file in `e` and adds what differs from Stata's
/// recorded results to `wrong`.
fn check_file(e: &mut Engine, dir: &Path, name: &str, wrong: &mut Vec<String>) {
    let dofile = std::fs::read_to_string(dir.join(format!("{name}.do"))).unwrap_or_default();
    let recorded =
        std::fs::read_to_string(dir.join(format!("{name}.recorded"))).unwrap_or_default();
    let stata = match results(&recorded) {
        Ok(s) => s,
        Err(why) => {
            wrong.push(format!("{name}.recorded: {why}"));
            return;
        }
    };
    if stata.is_empty() {
        wrong.push(format!(
            "{name}: nothing recorded; run cargo xtask stata-record"
        ));
        return;
    }
    let tol: f64 = header(&dofile, "tolerance")
        .and_then(|t| t.trim().parse().ok())
        .unwrap_or(1e-8);
    let folder = dir.display().to_string().replace('\\', "/");
    let text = match run_dofile(e, &folder, &dofile) {
        Ok(t) => t,
        Err(why) => {
            wrong.push(format!("{name}: {why}"));
            return;
        }
    };
    match results(&text) {
        Ok(got) => compare(name, &stata, &got, tol, wrong),
        Err(why) => wrong.push(format!("{name} (RCraft's output): {why}")),
    }
}

/// How many R sessions share the corpus: `RCRAFT_CORPUS_THREADS`, or the
/// machine's cores up to 4.
fn corpus_threads(files: usize) -> usize {
    std::env::var("RCRAFT_CORPUS_THREADS")
        .ok()
        .and_then(|t| t.trim().parse().ok())
        .unwrap_or_else(|| std::thread::available_parallelism().map_or(1, |n| n.get().min(4)))
        .clamp(1, files.max(1))
}

#[test]
fn the_corpus_gives_statas_results() {
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
    // Several R sessions share the files, each taking the next one in turn.
    // The files write only files named for themselves, so they do not meet.
    let next = AtomicUsize::new(0);
    let threads = corpus_threads(names.len());
    let found: Vec<Option<Vec<String>>> = std::thread::scope(|s| {
        let workers: Vec<_> = (0..threads)
            .map(|_| {
                s.spawn(|| {
                    let mut e = engine()?;
                    let mut wrong = Vec::new();
                    while let Some(name) = names.get(next.fetch_add(1, Ordering::Relaxed)) {
                        check_file(&mut e, &dir, name, &mut wrong);
                    }
                    Some(wrong)
                })
            })
            .collect();
        workers
            .into_iter()
            .map(|w| {
                w.join()
                    .unwrap_or_else(|_| Some(vec!["a corpus thread panicked".into()]))
            })
            .collect()
    });
    // no R session could start: skipped, with the note engine() printed
    if found.iter().all(Option::is_none) {
        return;
    }
    let mut wrong: Vec<String> = found.into_iter().flatten().flatten().collect();
    wrong.sort();
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

// The comparer itself: it must fail on every kind of difference, and drop
// no recorded line.

fn rows(text: &str) -> Vec<Row> {
    results(text).unwrap_or_default()
}

fn differences(stata: &str, got: &str) -> Vec<String> {
    let mut wrong = Vec::new();
    compare("t", &rows(stata), &rows(got), 1e-8, &mut wrong);
    wrong
}

#[test]
fn comparer_passes_lines_that_match() {
    let line = "@ a 1 2.5 -3e-2 . .a word\n@ b 7\n";
    assert!(differences(line, line).is_empty());
    // within 8 significant digits
    assert!(differences("@ a 1.00000000001", "@ a 1.0\n").is_empty());
    // text around the `@` lines is not compared
    assert!(differences("note\n@ a 1\n", "@ a 1\nmore\n").is_empty());
}

#[test]
fn comparer_fails_a_numeric_mismatch() {
    let w = differences("@ a 1 2\n", "@ a 1 2.001\n");
    assert_eq!(w.len(), 1, "{w:?}");
    assert!(w[0].contains("value 2"), "{w:?}");
}

#[test]
fn comparer_fails_an_extended_missing_mismatch() {
    assert_eq!(differences("@ a 1 .a\n", "@ a 1 .b\n").len(), 1);
    assert_eq!(differences("@ a .a\n", "@ a .\n").len(), 1);
    assert_eq!(differences("@ a .a\n", "@ a 0\n").len(), 1);
    assert_eq!(differences("@ a 3\n", "@ a .z\n").len(), 1);
}

#[test]
fn comparer_fails_a_string_mismatch() {
    assert_eq!(differences("@ a abc\n", "@ a abd\n").len(), 1);
    assert_eq!(differences("@ a \"x y\"\n", "@ a \"x z\"\n").len(), 1);
    // `inf` and `nan` are words, not numbers
    assert_eq!(differences("@ a inf\n", "@ a nan\n").len(), 1);
    assert!(differences("@ a abc\n", "@ a abc\n").is_empty());
}

#[test]
fn comparer_fails_a_line_that_cannot_be_read() {
    let why = results("fine\n@ a 1\n@ b 1.2.3\n").unwrap_err();
    assert!(why.contains("line 3") && why.contains("1.2.3"), "{why}");
    assert!(results("@ a 1e\n").is_err());
    assert!(results("@\n").is_err());
}

#[test]
fn comparer_fails_a_line_not_printed_or_printed_short() {
    assert_eq!(differences("@ a 1\n@ b 2\n", "@ a 1\n").len(), 1);
    assert_eq!(differences("@ a 1 2\n", "@ a 1\n").len(), 1);
}

#[test]
fn comparer_compares_every_time_a_key_is_printed() {
    // the earlier line must not be hidden by the later one
    let stata = "@ a 1\n@ a 2\n";
    assert!(differences(stata, stata).is_empty());
    assert_eq!(differences(stata, "@ a 9\n@ a 2\n").len(), 1);
    // the first is 1 against 2, the second is not printed
    assert_eq!(differences(stata, "@ a 2\n").len(), 2);
    assert_eq!(differences(stata, "@ a 1\n@ a 2\n@ a 2\n").len(), 1);
}

#[test]
fn comparer_reads_hex_floats_as_numbers() {
    // the same value with different last hex digits: well within 8 digits
    let a = "@ a -1.8c3c9eecbfb16X-002 +1.0000000000000X+000\n";
    let b = "@ a -1.8c3c9eecbfb18X-002 +1.0000000000001X+000\n";
    assert!(differences(a, b).is_empty());
    // the exponent is hex, the mantissa's sign counts, zero is zero
    assert_eq!(
        rows("@ a +1.2a05f20000000X+021 -1.8000000000000X+000 +0.0000000000000X-3ff\n")
            .first()
            .map(|r| r.values.clone()),
        Some(vec![Tok::Num(1.0e10), Tok::Num(-1.5), Tok::Num(0.0)])
    );
    // off by 2^-25 (3e-8), in the 8th significant digit: fails
    let c = "@ a +1.0000000000000X+000\n";
    let d = "@ a +1.0000008000000X+000\n";
    assert_eq!(differences(c, d).len(), 1);
    // a different value fails, and so does a hex float against a word
    assert_eq!(differences(c, "@ a +1.8000000000000X+000\n").len(), 1);
    assert_eq!(differences(c, "@ a word\n").len(), 1);
}

#[test]
fn comparer_keeps_hex_missing_codes_exact() {
    let dot = "@ a +1.0000000000000X+3ff\n";
    assert!(differences(dot, dot).is_empty());
    // `.` and `.a` are numbers 8 digits apart; they must not pass for one another
    assert_eq!(differences(dot, "@ a +1.0000000000001X+3ff\n").len(), 1);
    assert_eq!(differences(dot, "@ a .\n").len(), 1);
}

#[test]
fn comparer_fails_a_malformed_hex_float() {
    for bad in [
        "+1.8c3cX",
        "+1.8c3cX+",
        "+1.8c3cX+00g",
        "+1.8c3cX+0000",
        "+1.X+002",
        "+1.00000000000000X+002",
    ] {
        let why = results(&format!("@ a {bad}\n")).unwrap_err();
        assert!(why.contains(bad), "{bad}: {why}");
    }
    // plain words and decimals that look a little like one stay as they were
    assert!(results("@ a 1.5 0.25 1.x word\n").is_ok());
}
