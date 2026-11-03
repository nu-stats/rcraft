//! P2C.9, whole replication packages run on R: each package (a copy made for
//! testing, outside the repository) runs from its entry do-file the way the
//! do-file window runs one, line by line in a fresh R, past any problem, so
//! the whole package is tried. What R printed goes to `rcraft.log` in the
//! package's folder and every line's fate to `rcraft_lines.tsv` (file, line,
//! command, `ok`, `refused` or `r-error`, and why), and the R it ran to
//! `rcraft.R`, to set against the package's run in Stata 19 (its `@E` lines
//! in particular, which the copy prints after each model).
//!
//! Set `RCRAFT_E2E_DIR` to the folder of the copies; each package's folder
//! has `entry.txt`: the folder to start in (relative to the package) on the
//! first line and the do-file to run (relative to that folder) on the
//! second. `RCRAFT_E2E_ONLY` picks the packages whose names hold its text.
//! Skipped without them. It reports; it fails only if the runner panics.

#![allow(clippy::panic)]

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::Path;
use std::time::{Duration, Instant};

use rcraft_engine::{Engine, Item, Outcome, Source};
use rcraft_rhost::{find_installs, Env, SessionConfig};
use rcraft_statatr::dofile::{command_name, Event, Files, Options, Run};

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

/// Runs one package; writes its log and its table of lines beside it.
fn run_package(e: &mut Engine, pkg: &Path, budget: Duration) -> String {
    let entry = std::fs::read_to_string(pkg.join("entry.txt")).unwrap_or_default();
    let mut parts = entry.lines().map(str::trim);
    let (Some(start), Some(dofile)) = (parts.next(), parts.next()) else {
        return format!("{}: no entry.txt", pkg.display());
    };
    let folder = pkg.join(start);
    let folder_text = folder.display().to_string().replace('\\', "/");
    let Ok(text) = std::fs::read_to_string(folder.join(dofile)) else {
        return format!("{}: no {dofile}", pkg.display());
    };
    let started = Instant::now();
    let mut log = String::new();
    let mut script = String::new();
    // each line: the index of its file (named at the end) and the rest
    let mut rows: Vec<(usize, String)> = Vec::new();
    let (mut ok, mut refused, mut errors) = (0, 0, 0);
    if run(e, &format!("setwd(\"{folder_text}\")"), budget).0 != Outcome::Ok {
        return format!("{}: R could not start in {folder_text}", pkg.display());
    }
    let opts = Options {
        name: dofile.to_string(),
        folder: folder_text.clone(),
        temp_folder: format!("{folder_text}/rcraft_temp"),
        ..Options::default()
    };
    let _ = std::fs::create_dir_all(folder.join("rcraft_temp"));
    let mut r = Run::new(&text, &Disk, opts);
    let mut sent = BTreeSet::new();
    let mut handled = 0;
    let mut timed_out = false;
    loop {
        if started.elapsed() > budget {
            timed_out = true;
            break;
        }
        match r.next_event() {
            Event::Line => {
                let lines = r.lines().get(handled..).unwrap_or_default().to_vec();
                let origins = r.origins().get(handled..).unwrap_or_default().to_vec();
                handled += lines.len();
                for (line, origin) in lines.iter().zip(&origins) {
                    let command = command_name(line.stata.trim());
                    let (status, why) = match &line.result {
                        Err(e) => {
                            refused += 1;
                            ("refused", e.reason.clone())
                        }
                        Ok(_) => {
                            let (mut code, names) = fresh(&r, &sent);
                            code.push_str(&line.script());
                            script.push_str(&code);
                            script.push('\n');
                            let left = budget.saturating_sub(started.elapsed());
                            let (o, t) = run(e, &code, left.max(Duration::from_secs(1)));
                            log.push_str(&t);
                            if !t.ends_with('\n') && !t.is_empty() {
                                log.push('\n');
                            }
                            if o == Outcome::Ok {
                                sent.extend(names);
                                ok += 1;
                                ("ok", String::new())
                            } else {
                                errors += 1;
                                let said: String = t
                                    .lines()
                                    .filter(|l| l.contains("rror"))
                                    .collect::<Vec<_>>()
                                    .join(" ");
                                ("r-error", said)
                            }
                        }
                    };
                    rows.push((
                        origin.file,
                        format!(
                            "{}\t{command}\t{status}\t{}",
                            origin.span.line,
                            why.replace(['\t', '\n'], " ")
                        ),
                    ));
                }
            }
            Event::Ask(v) => {
                let (mut code, names) = fresh(&r, &sent);
                code.push_str(&v.code);
                script.push_str(&code);
                script.push('\n');
                let (o, t) = run(e, &code, Duration::from_secs(120));
                if o == Outcome::Ok {
                    sent.extend(names);
                }
                r.give(if o == Outcome::Ok { Ok(t) } else { Err(t) });
            }
            Event::Done => break,
        }
    }
    let files = r.finish().files;
    let mut table = String::from("file\tline\tcommand\tstatus\twhy\n");
    for (i, rest) in &rows {
        let name = files
            .get(*i)
            .map_or("?", |f| if f.is_empty() { dofile } else { f });
        let _ = writeln!(table, "{name}\t{rest}");
    }
    let _ = std::fs::write(pkg.join("rcraft.log"), &log);
    let _ = std::fs::write(pkg.join("rcraft.R"), &script);
    let _ = std::fs::write(pkg.join("rcraft_lines.tsv"), &table);
    format!(
        "{}: {ok} ran, {refused} refused, {errors} stopped in R{} ({:.0} s)",
        pkg.file_name().unwrap_or_default().to_string_lossy(),
        if timed_out { ", timed out" } else { "" },
        started.elapsed().as_secs_f64()
    )
}

#[test]
fn replication_packages_run_on_r() {
    let Ok(dir) = std::env::var("RCRAFT_E2E_DIR") else {
        eprintln!("skipped: set RCRAFT_E2E_DIR to the folder of the package copies");
        return;
    };
    let only = std::env::var("RCRAFT_E2E_ONLY").unwrap_or_default();
    let budget = Duration::from_secs(
        std::env::var("RCRAFT_E2E_SECONDS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1800),
    );
    let mut names: Vec<_> = std::fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.join("entry.txt").exists())
        .filter(|p| p.to_string_lossy().contains(&only))
        .collect();
    names.sort();
    for pkg in names {
        let Some(mut e) = engine() else {
            eprintln!("skipped: no supported R on this machine");
            return;
        };
        let summary = run_package(&mut e, &pkg, budget);
        eprintln!("{summary}");
    }
}
