//! P2C.9, StatCraft's probe do-files run on R: each probe (ours, on made-up
//! data) runs line by line in a fresh R, the way the do-file window runs it,
//! and what it prints is set against what Stata 19 printed for it, from the
//! probe's log in StatCraft's private corpus. The corpus is never read into
//! the repository; set `RCRAFT_STATA_PROBES` to it (`~/statcraft-corpus`) to
//! run this, and `RCRAFT_STATA_PROBE` to part of a name for one probe. It is
//! skipped otherwise.
//! `RCRAFT_PROBE_TRACE` prints each line whose Stata holds that text, with what
//! became of it and what R printed, to follow one probe (`RCRAFT_STATA_PROBE`
//! picks the probes whose folder names hold its text).
//!
//! Compared: each `display`, `return list` and `ereturn list` at the top of a
//! probe (not in a loop or a block, so it ran once, in order, in both), each
//! with the next line Stata echoed with the same text. Numbers agree to
//! 8 significant digits; `%21x` is read back to the double. Text must be the
//! same. Nothing after a probe's first R error is compared: the do-file window
//! stops there. The test reports what it found; it fails only if the runner
//! panics. `docs/stata-coverage.md` has the counts.

#![allow(clippy::panic)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use rcraft_engine::{Engine, Item, Outcome, Source};
use rcraft_rhost::{find_installs, Env, SessionConfig};
use rcraft_statatr::dofile::{command_name, Event, Files, Options, Run};
use rcraft_statatr::reader::{read, Item as RItem};
use rcraft_statatr::values::hex::from_hex;

fn engine() -> Option<Engine> {
    let install = find_installs(&Env::from_process())
        .into_iter()
        .find(|i| i.is_supported())?;
    Engine::start(SessionConfig::new(install)).ok()
}

fn run(e: &mut Engine, code: &str, limit: Duration) -> (Outcome, String) {
    let Ok(id) = e.submit(Source::Typed, code) else {
        return (Outcome::Error, String::new());
    };
    let until = Instant::now() + limit;
    loop {
        e.poll(Duration::from_millis(20));
        if !e.is_working() {
            break;
        }
        if Instant::now() > until {
            e.interrupt();
            let end = Instant::now() + Duration::from_secs(10);
            while e.is_working() && Instant::now() < end {
                e.poll(Duration::from_millis(50));
            }
            return (Outcome::Error, "timed out".into());
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

/// What one top-level line did on R.
struct Ran {
    command: String,
    translated: bool,
    r_ok: bool,
    printed: String,
    /// Why it was not translated.
    refused: String,
}

#[derive(Default)]
struct Probe {
    /// Top-level lines by the do-file's line number.
    top: BTreeMap<usize, Ran>,
    /// Every line: (command, translated, ran in R without an error).
    all: Vec<(String, bool, bool)>,
    /// The lines R stopped at: command, Stata, what R said.
    errors: Vec<(String, String, String)>,
    /// The do-file's line R first stopped at: the do-file window stops there
    /// (or asks), so what comes after is not compared.
    first_error: Option<usize>,
    timed_out: bool,
}

/// Runs the do-file on R as the do-file window does; never stops at a
/// problem, so the lines after it are tried too.
fn run_probe(e: &mut Engine, folder: &str, text: &str, budget: Duration) -> Probe {
    let mut p = Probe::default();
    let started = Instant::now();
    if run(e, &format!("setwd(\"{folder}\")"), budget).0 != Outcome::Ok {
        return p;
    }
    let opts = Options {
        name: "probe.do".into(),
        folder: folder.to_string(),
        temp_folder: folder.to_string(),
        ..Options::default()
    };
    let mut r = Run::new(text, &Disk, opts);
    let mut sent = BTreeSet::new();
    let mut handled = 0;
    loop {
        if started.elapsed() > budget {
            p.timed_out = true;
            return p;
        }
        let event = r.next_event();
        match event {
            Event::Line => {
                let lines = r.lines().get(handled..).unwrap_or_default().to_vec();
                let origins = r.origins().get(handled..).unwrap_or_default().to_vec();
                handled += lines.len();
                for (line, origin) in lines.iter().zip(&origins) {
                    let command = command_name(line.stata.trim());
                    if std::env::var("RCRAFT_PROBE_TRACE").is_ok_and(|t| line.stata.contains(&t)) {
                        eprintln!(
                            "TRACE {} | {:?} | {:?} | {:?}",
                            line.stata,
                            origin.info,
                            origin.what,
                            line.result.as_ref().err()
                        );
                    }
                    let translated = line.result.is_ok();
                    let (ok, printed) = if translated {
                        let (mut code, names) = fresh(&r, &sent);
                        code.push_str(&line.script());
                        let left = budget.saturating_sub(started.elapsed());
                        let (o, t) = run(e, &code, left.max(Duration::from_secs(1)));
                        // a line R cannot read runs nothing, its helpers included
                        if o == Outcome::Ok {
                            sent.extend(names);
                        }
                        (o == Outcome::Ok, t)
                    } else {
                        (false, String::new())
                    };
                    if std::env::var("RCRAFT_PROBE_TRACE").is_ok_and(|t| line.stata.contains(&t)) {
                        eprintln!("{}\n{printed}", line.script());
                    }
                    p.all.push((command.clone(), translated, ok));
                    if translated && !ok {
                        if origin.file == 0 && p.first_error.is_none() {
                            p.first_error = Some(origin.span.line);
                        }
                        p.errors
                            .push((command.clone(), line.stata.clone(), printed.clone()));
                    }
                    if origin.file == 0 && origin.loops.is_empty() {
                        p.top.insert(
                            origin.span.line,
                            Ran {
                                command,
                                translated,
                                r_ok: ok,
                                printed,
                                refused: line
                                    .result
                                    .as_ref()
                                    .err()
                                    .map(|e| e.reason.clone())
                                    .unwrap_or_default(),
                            },
                        );
                    }
                }
            }
            Event::Ask(v) => {
                let (mut code, names) = fresh(&r, &sent);
                code.push_str(&v.code);
                let (o, t) = run(e, &code, Duration::from_secs(60));
                if o == Outcome::Ok {
                    sent.extend(names);
                }
                r.give(if o == Outcome::Ok { Ok(t) } else { Err(t) });
            }
            Event::Done => return p,
        }
    }
}

/// The helpers the run calls that R does not have yet, and their names.
fn fresh(r: &Run<'_>, sent: &BTreeSet<&'static str>) -> (String, Vec<&'static str>) {
    let mut code = String::new();
    let mut names = Vec::new();
    for (name, h) in r.helpers() {
        if !sent.contains(name) {
            code.push_str(h);
            code.push('\n');
            names.push(name);
        }
    }
    (code, names)
}

/// Stata's log as (command line echoed at the top, what it printed).
fn echoes(log: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    let mut blank_before = true;
    for line in log.lines() {
        // an echo follows a blank line; a "." right after one is a missing value printed
        if let (Some(cmd), true) = (line.strip_prefix(". "), blank_before) {
            out.push((cmd.to_string(), String::new()));
        } else if line == "." && blank_before {
            out.push((String::new(), String::new()));
        } else if let Some(last) = out.last_mut() {
            match line.strip_prefix("> ") {
                Some(more) if last.1.is_empty() => last.0.push_str(more),
                _ => {
                    last.1.push_str(line);
                    last.1.push('\n');
                }
            }
        }
        blank_before = line.trim().is_empty();
    }
    out
}

fn full_name(text: &str) -> String {
    let word = command_name(text.trim());
    read(&word)
        .commands()
        .first()
        .map(|c| c.name.clone())
        .unwrap_or(word)
}

/// A printed token as a number: plain, or `%21x`.
fn number(t: &str) -> Option<f64> {
    if t.contains('X') {
        return from_hex(t);
    }
    if t == "." {
        return None;
    }
    t.parse().ok()
}

fn same_number(a: f64, b: f64, tol: f64) -> bool {
    a == b || (a - b).abs() <= tol * a.abs().max(b.abs())
}

/// Whether two printed texts agree: the same words, numbers to 8 digits.
fn same_text(stata: &str, r: &str, tol: f64) -> bool {
    let a: Vec<&str> = stata.split_whitespace().collect();
    let b: Vec<&str> = r.split_whitespace().collect();
    if a == b {
        return true;
    }
    // a word Stata wrapped at its line width is one word in R
    if a.concat() == b.concat() {
        return true;
    }
    a.len() == b.len()
        && a.iter().zip(&b).all(|(x, y)| {
            x == y
                || match (number(x), number(y)) {
                    (Some(p), Some(q)) => same_number(p, q, tol),
                    _ => {
                        // numbers inside text: M|equals|[.333...]
                        let split = |s: &str| -> Vec<String> {
                            s.split(|c: char| "|[]=:,()".contains(c))
                                .map(str::to_string)
                                .collect()
                        };
                        let (sx, sy) = (split(x), split(y));
                        sx.len() == sy.len()
                            && sx.iter().zip(&sy).all(|(u, v)| {
                                u == v
                                    || matches!((number(u), number(v)), (Some(p), Some(q)) if same_number(p, q, tol))
                            })
                    }
                }
        })
}

/// `name = value` of a `return list` / `ereturn list`, scalars only.
fn scalars(text: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let mut in_scalars = false;
    for line in text.lines() {
        let t = line.trim();
        if t.ends_with(':') && !t.contains(' ') {
            in_scalars = t == "scalars:";
            continue;
        }
        if !in_scalars {
            continue;
        }
        if let Some((k, v)) = t.split_once(" = ") {
            out.insert(k.trim().to_string(), v.trim().to_string());
        }
    }
    out
}

#[derive(Default)]
struct Tally {
    lines: usize,
    translated: usize,
    r_errors: usize,
    compared: usize,
    matched: usize,
    /// Agree to 6 significant digits, not 8.
    near: usize,
}

#[test]
fn statcrafts_probes_run_on_r() {
    let Ok(corpus) = std::env::var("RCRAFT_STATA_PROBES") else {
        eprintln!("skipped: set RCRAFT_STATA_PROBES to StatCraft's corpus folder");
        return;
    };
    let only = std::env::var("RCRAFT_STATA_PROBE").unwrap_or_default();
    let budget = Duration::from_secs(
        std::env::var("RCRAFT_STATA_PROBE_SECONDS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(120),
    );
    let mut names: Vec<String> = std::fs::read_dir(&corpus)
        .unwrap_or_else(|e| panic!("{corpus}: {e}"))
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().to_str().map(str::to_string))
        .filter(|n| n.contains(&only))
        .collect();
    names.sort();
    let work: PathBuf = std::env::temp_dir().join(format!("rcraft-probes-{}", std::process::id()));
    let mut by_command: BTreeMap<String, Tally> = BTreeMap::new();
    let mut wrong: Vec<String> = Vec::new();
    let mut near: Vec<String> = Vec::new();
    let mut r_errors: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut all_errors: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut per_probe: Vec<String> = Vec::new();
    let (mut compared, mut matched, mut timed_out) = (0, 0, 0);
    let mut after_error = 0;
    // lines Stata printed that were not translated: reason -> lines
    let mut refused: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for name in &names {
        let dir = Path::new(&corpus).join(name);
        let (Ok(dofile), Ok(log)) = (
            std::fs::read_to_string(dir.join(format!("{name}.do"))),
            std::fs::read_to_string(dir.join(format!("{name}.log"))),
        ) else {
            continue;
        };
        let folder = work.join(name);
        let _ = std::fs::create_dir_all(&folder);
        let Some(mut e) = engine() else {
            eprintln!("skipped: no supported R on this machine");
            return;
        };
        let f = folder.display().to_string().replace('\\', "/");
        let p = run_probe(&mut e, &f, &dofile, budget);
        drop(e);
        if p.timed_out {
            timed_out += 1;
        }
        for (cmd, stata, said) in &p.errors {
            let said = said
                .lines()
                .filter(|l| l.starts_with("error"))
                .collect::<Vec<_>>()
                .join(" ");
            all_errors
                .entry(cmd.clone())
                .or_default()
                .push(format!("{name}: {} => {said}", stata.trim()));
        }
        for (cmd, tr, ok) in &p.all {
            let t = by_command.entry(cmd.clone()).or_default();
            t.lines += 1;
            t.translated += usize::from(*tr);
            if *tr && !*ok {
                t.r_errors += 1;
            }
        }
        // the top-level commands of the do-file, in order, with the echoes
        let program = read(&dofile);
        // each line with the next echo of the same text, in order
        let echoed = echoes(&log);
        let squeeze = |s: &str| -> String {
            s.replace("///", " ")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        };
        let mut at = 0;
        let (mut pc, mut pm) = (0, 0);
        for item in &program.items {
            let RItem::Command(c) = item else { continue };
            let n = full_name(&c.text);
            if !matches!(n.as_str(), "display" | "return" | "ereturn") {
                continue;
            }
            if p.first_error.is_some_and(|l| c.span.line > l) {
                after_error += 1;
                continue;
            }
            let want = squeeze(&c.text);
            let Some(k) = (at..echoed.len()).find(|&k| squeeze(&echoed[k].0) == want) else {
                continue;
            };
            at = k + 1;
            let stata = &echoed[k].1;
            let Some(ran) = p.top.get(&c.span.line) else {
                continue;
            };
            if ran.command.is_empty() {
                continue;
            }
            if !ran.translated {
                let failed = stata
                    .lines()
                    .any(|l| l.starts_with("r(") && l.ends_with(");"));
                if !failed {
                    refused
                        .entry(ran.refused.clone())
                        .or_default()
                        .push(format!("{name}:{}", c.span.line));
                }
                continue;
            }
            if !ran.r_ok {
                r_errors
                    .entry(ran.command.clone())
                    .or_default()
                    .push(format!("{name}:{}: {}", c.span.line, ran.printed.trim()));
                continue;
            }
            // Stata stopped with an error there: nothing to compare
            if stata
                .lines()
                .any(|l| l.starts_with("r(") && l.ends_with(");"))
            {
                continue;
            }
            let t = by_command.entry(n.clone()).or_default();
            t.compared += 1;
            pc += 1;
            // the values that differ at a tolerance (all of them for display)
            let differ = |tol: f64| -> Vec<String> {
                if n == "display" {
                    if same_text(stata, &ran.printed, tol) {
                        return Vec::new();
                    }
                    return vec![format!(
                        "{}\n    Stata: {}\n    R:     {}",
                        c.text.trim(),
                        stata.trim().replace('\n', " / "),
                        ran.printed.trim().replace('\n', " / ")
                    )];
                }
                let (s, r) = (scalars(stata), scalars(&ran.printed));
                if r.is_empty() && !s.is_empty() {
                    return vec!["nothing listed".into()];
                }
                r.iter()
                    .filter_map(|(key, have)| {
                        let want = s.get(key)?;
                        let fine = have == want
                            || matches!((number(have), number(want)), (Some(a), Some(b)) if same_number(a, b, tol));
                        (!fine).then(|| format!("{key} is {have}, Stata's {want}"))
                    })
                    .collect()
            };
            if differ(1e-8).is_empty() {
                t.matched += 1;
                pm += 1;
            } else if differ(1e-6).is_empty() {
                t.near += 1;
                near.push(format!(
                    "{name}:{}: {}",
                    c.span.line,
                    differ(1e-8).join("; ")
                ));
            } else {
                for d in differ(1e-6) {
                    wrong.push(format!("{name}:{}: {d}", c.span.line));
                }
            }
        }
        compared += pc;
        matched += pm;
        let tr = p.all.iter().filter(|x| x.1).count();
        per_probe.push(format!(
            "{name}: {} lines, {tr} translated, {pm}/{pc} compared match{}",
            p.all.len(),
            if p.timed_out { ", timed out" } else { "" }
        ));
    }
    let _ = std::fs::remove_dir_all(&work);
    for l in &per_probe {
        eprintln!("{l}");
    }
    eprintln!("\nby command (lines, translated, R errors, compared, matched, to 6 digits):");
    let mut rows: Vec<_> = by_command.iter().collect();
    rows.sort_by_key(|(_, t)| std::cmp::Reverse(t.lines));
    for (c, t) in rows {
        eprintln!(
            "  {c}: {} {} {} {} {} {}",
            t.lines, t.translated, t.r_errors, t.compared, t.matched, t.near
        );
    }
    eprintln!("\nR errors on lines compared:");
    for (c, v) in &r_errors {
        eprintln!("  {c}: {}", v.len());
        for x in v.iter().take(5) {
            eprintln!("    {}", x.chars().take(300).collect::<String>());
        }
    }
    eprintln!("\nR errors, every line (the first of each kind):");
    let mut rows: Vec<_> = all_errors.iter().collect();
    rows.sort_by_key(|(_, v)| std::cmp::Reverse(v.len()));
    for (c, v) in rows {
        eprintln!("  {c}: {}", v.len());
        let mut seen = BTreeSet::new();
        for x in v {
            let said: String = x
                .split(" => ")
                .nth(1)
                .unwrap_or("")
                .chars()
                .take(90)
                .collect();
            if seen.insert(said) && seen.len() <= 6 {
                eprintln!("    {}", x.chars().take(260).collect::<String>());
            }
        }
    }
    eprintln!("\nagree to 6 significant digits, not 8 ({}):", near.len());
    for w in &near {
        eprintln!("  {}", w.chars().take(400).collect::<String>());
    }
    let mut why: Vec<(&String, &Vec<String>)> = refused.iter().collect();
    why.sort_by_key(|(r, v)| (std::cmp::Reverse(v.len()), r.to_string()));
    eprintln!(
        "\nprinted by Stata, not translated ({}), most lines first:",
        refused.values().map(Vec::len).sum::<usize>()
    );
    for (r, v) in why.iter().take(40) {
        let at: Vec<&str> = v.iter().take(3).map(String::as_str).collect();
        eprintln!(
            "  {} | {} | {}",
            v.len(),
            r.chars().take(200).collect::<String>(),
            at.join(", ")
        );
    }
    eprintln!("\ndiffering ({}):", wrong.len());
    for w in &wrong {
        eprintln!("  {w}");
    }
    eprintln!(
        "\n{} probes ({timed_out} timed out): {matched} of {compared} compared lines match Stata to 8 digits, {} more to 6",
        names.len(),
        near.len()
    );
    eprintln!("{after_error} lines to compare came after an R error and were not compared");
}
