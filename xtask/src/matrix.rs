//! `cargo xtask stata-matrix`: the command table shared by RCraft and
//! StatCraft (tandem plan, step 0.2).
//!
//! One row per command: where StatCraft and RCraft each stand, the probes
//! that use it, how many probe lines RCraft refuses, the notes that cover it
//! and who holds its lane. The file is generated, never edited by hand, and
//! lives outside both repositories (`C:/pinokio/api/stata-matrix.csv`, beside
//! the lanes ledger): it is private and not committed.
//!
//! It holds **names and counts only**, never anything Stata printed. StatCraft
//! is read through `git show origin/main:...` and `git archive`, so its
//! working tree (other sessions' unfinished edits) is never read or touched.
//!
//! Sources:
//! - StatCraft: `docs/parity.md` (catalog status), `crates/engine/src/commands/mod.rs`
//!   (the engine's dispatch, for commands outside the catalog), `docs/packages.md`
//!   (SSC packages it runs as ado), `plan/stata/*.md` (note names and headings).
//! - RCraft: the translator's `dispatch` (`data_r.rs`), `verified.rs`, the
//!   per-command counts the `many_dofiles` test writes when
//!   `RCRAFT_COVERAGE_COMMANDS` is set, and `plan/stata/*.md`.
//! - The ledger `stata-lanes.md`.
//!
//! ```text
//! cargo xtask stata-matrix [--statcraft DIR] [--ledger FILE] [--out FILE]
//!                          [--repdo DIR] [--probes-tsv FILE] [--rep-tsv FILE]
//! ```
//!
//! Without `--probes-tsv` the task extracts StatCraft's probe do-files from
//! `origin/main` into `target/stata-matrix/` and runs the test itself (a dry
//! run: no R, no Stata). `--repdo` (or `RCRAFT_REPDO`) names the folder of
//! replication-package do-files for the second measure.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{root, Result};

const DEFAULT_STATCRAFT: &str = "C:/pinokio/api/statcraft";
const DEFAULT_LEDGER: &str = "C:/pinokio/api/stata-lanes.md";
const DEFAULT_OUT: &str = "C:/pinokio/api/stata-matrix.csv";
const STATCRAFT_REF: &str = "origin/main";

pub fn run(args: &[String]) -> Result {
    let opt = |name: &str| -> Option<String> {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    let statcraft = PathBuf::from(opt("--statcraft").unwrap_or_else(|| DEFAULT_STATCRAFT.into()));
    let ledger = PathBuf::from(opt("--ledger").unwrap_or_else(|| DEFAULT_LEDGER.into()));
    let out = PathBuf::from(opt("--out").unwrap_or_else(|| DEFAULT_OUT.into()));
    let repdo = opt("--repdo").or_else(|| std::env::var("RCRAFT_REPDO").ok());
    let work = root().join("target").join("stata-matrix");
    std::fs::create_dir_all(&work).map_err(|e| format!("{}: {e}", work.display()))?;

    // StatCraft, read from its origin/main
    let git = |path: &str| git_show(&statcraft, path);
    let parity = parse_parity(&git("docs/parity.md")?);
    let dispatch_sc = parse_statcraft_dispatch(&git("crates/engine/src/commands/mod.rs")?);
    let packages = parse_packages(&git("docs/packages.md")?);
    let sc_notes = statcraft_notes(&statcraft)?;

    // RCraft
    let rc_text =
        |p: &str| std::fs::read_to_string(root().join(p)).map_err(|e| format!("{p}: {e}"));
    let dispatch_rc = parse_rcraft_dispatch(&rc_text("crates/statatr/src/data_r.rs")?);
    let verified: BTreeSet<String> = rcraft_statatr::verified::VERIFIED
        .iter()
        .map(|(n, _)| (*n).to_string())
        .collect();
    let rc_notes = rcraft_notes()?;

    // the per-command counts of the dry runs
    let probes_tsv = match opt("--probes-tsv") {
        Some(f) => PathBuf::from(f),
        None => {
            let dir = extract_probes(&statcraft, &work)?;
            let file = work.join("probes-commands.tsv");
            dry_run(&dir, &file)?;
            file
        }
    };
    let rep_tsv = match (opt("--rep-tsv"), repdo) {
        (Some(f), _) => Some(PathBuf::from(f)),
        (None, Some(dir)) => {
            let file = work.join("rep-commands.tsv");
            dry_run(Path::new(&dir), &file)?;
            Some(file)
        }
        (None, None) => {
            println!("stata-matrix: no --repdo, the replication-package columns stay empty");
            None
        }
    };
    let probes = read_counts(&probes_tsv)?;
    let reps = match &rep_tsv {
        Some(f) => read_counts(f)?,
        None => Counts::default(),
    };

    let ledger_rows = match std::fs::read_to_string(&ledger) {
        Ok(t) => parse_ledger(&t),
        Err(e) => return Err(format!("{}: {e}", ledger.display())),
    };

    // every command any source names
    let mut names: BTreeSet<String> = BTreeSet::new();
    names.extend(parity.keys().cloned());
    names.extend(packages.keys().cloned());
    names.extend(dispatch_rc.iter().cloned());
    names.extend(verified.iter().cloned());
    // a word a do-file starts a line with is a command only if some table
    // knows it; the rest are the do-files' own programs and locals
    let known: BTreeSet<&str> = rcraft_statatr::complete::COMMANDS
        .iter()
        .copied()
        .chain(
            rcraft_statatr::lang::commands::COMMANDS
                .iter()
                .map(|(n, _)| *n),
        )
        .collect();
    let mut own_programs = 0usize;
    for (name, count) in probes.by_command.iter().chain(reps.by_command.iter()) {
        if known.contains(name.as_str())
            || parity.contains_key(name)
            || packages.contains_key(name)
            || dispatch_sc.contains(name)
            || count.not > 0
        {
            names.insert(name.clone());
        } else {
            own_programs += 1;
        }
    }
    println!("stata-matrix: {own_programs} words the do-files start lines with are in no command table and refuse nothing (their own programs, locals): left out");
    names.retain(|n| is_word(n));

    let mut rows = Vec::new();
    let mut tally = Tally::default();
    for name in &names {
        let sc = if let Some(p) = parity.get(name) {
            p.status.to_string()
        } else if packages.contains_key(name) {
            "package".to_string()
        } else if dispatch_sc.contains(name) {
            "dispatch".to_string()
        } else {
            String::new()
        };
        let probe = probes.by_command.get(name).cloned().unwrap_or_default();
        let rep = reps.by_command.get(name).cloned().unwrap_or_default();
        let translated_somewhere = probe.translated > 0 || rep.translated > 0;
        let rc = if verified.contains(name) {
            "verified"
        } else if dispatch_rc.contains(name) || translated_somewhere {
            "translated"
        } else if probe.not > 0 || rep.not > 0 {
            "refused"
        } else {
            ""
        };
        let sc_has = matches!(sc.as_str(), "matches" | "partial" | "package" | "dispatch");
        let rc_has = matches!(rc, "verified" | "translated");
        let held_rows: Vec<&LedgerRow> = ledger_rows
            .iter()
            .filter(|r| r.commands.contains(name))
            .collect();
        let held = held_rows.iter().any(|r| r.held());
        let lane = held_rows
            .iter()
            .map(|r| format!("{}: {} ({})", r.project, r.session, r.state_short()))
            .collect::<Vec<_>>()
            .join(" | ");
        tally.add(sc_has, rc_has, held, !sc.is_empty() || !rc.is_empty());
        let numbers = probes.numbers_of(name);
        rows.push(Row {
            command: name.clone(),
            sc,
            sc_forms: parity.get(name).map_or(0, |p| p.forms),
            rc: rc.to_string(),
            rc_dispatch: dispatch_rc.contains(name),
            probe_files: probe.files,
            probe_numbers: numbers,
            probe_lines: probe.lines,
            probe_refused: probe.not,
            rep_files: rep.files,
            rep_lines: rep.lines,
            rep_refused: rep.not,
            note_rc: rc_notes.paths_for(name),
            note_sc: sc_notes.paths_for(name),
            lane,
            held,
        });
    }

    let csv = render(&rows);
    std::fs::write(&out, csv).map_err(|e| format!("{}: {e}", out.display()))?;
    println!(
        "stata-matrix: wrote {} ({} commands)",
        out.display(),
        rows.len()
    );
    println!(
        "stata-matrix: both {}, only StatCraft {}, only RCraft {}, neither {}, held {}",
        tally.both, tally.only_sc, tally.only_rc, tally.neither, tally.held
    );
    Ok(())
}

#[derive(Default)]
struct Tally {
    both: usize,
    only_sc: usize,
    only_rc: usize,
    neither: usize,
    held: usize,
}

impl Tally {
    fn add(&mut self, sc: bool, rc: bool, held: bool, known: bool) {
        match (sc, rc) {
            (true, true) => self.both += 1,
            (true, false) => self.only_sc += 1,
            (false, true) => self.only_rc += 1,
            (false, false) if known => self.neither += 1,
            (false, false) => {}
        }
        if held {
            self.held += 1;
        }
    }
}

struct Row {
    command: String,
    sc: String,
    sc_forms: usize,
    rc: String,
    rc_dispatch: bool,
    probe_files: usize,
    probe_numbers: String,
    probe_lines: usize,
    probe_refused: usize,
    rep_files: usize,
    rep_lines: usize,
    rep_refused: usize,
    note_rc: String,
    note_sc: String,
    lane: String,
    held: bool,
}

fn render(rows: &[Row]) -> String {
    let mut s = String::from(
        "command,statcraft_status,statcraft_forms,rcraft_status,rcraft_dispatch,probe_files,probe_numbers,probe_lines,probe_lines_refused,rep_files,rep_lines,rep_lines_refused,note_rcraft,note_statcraft,lane,held\n",
    );
    for r in rows {
        let cells = [
            r.command.clone(),
            r.sc.clone(),
            r.sc_forms.to_string(),
            r.rc.clone(),
            r.rc_dispatch.to_string(),
            r.probe_files.to_string(),
            r.probe_numbers.clone(),
            r.probe_lines.to_string(),
            r.probe_refused.to_string(),
            r.rep_files.to_string(),
            r.rep_lines.to_string(),
            r.rep_refused.to_string(),
            r.note_rc.clone(),
            r.note_sc.clone(),
            r.lane.clone(),
            r.held.to_string(),
        ];
        let line: Vec<String> = cells.iter().map(|c| csv_cell(c)).collect();
        let _ = writeln!(s, "{}", line.join(","));
    }
    s
}

fn csv_cell(c: &str) -> String {
    if c.contains([',', '"', '\n']) {
        format!("\"{}\"", c.replace('"', "\"\""))
    } else {
        c.to_string()
    }
}

/// A command word: lower-case letters, digits and `_`, starting with a
/// letter or `_`.
fn is_word(s: &str) -> bool {
    let mut chars = s.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_lowercase() || c == '_')
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

// ---- StatCraft, read from origin/main ------------------------------------

fn git_show(repo: &Path, path: &str) -> std::result::Result<String, String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .arg("show")
        .arg(format!("{STATCRAFT_REF}:{path}"))
        .output()
        .map_err(|e| format!("git: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "git show {STATCRAFT_REF}:{path} in {}: {}",
            repo.display(),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

struct Parity {
    status: &'static str,
    forms: usize,
}

/// `| \`cmd\` | release | status |` rows; a two-word command (`import
/// delimited`) is folded into its first word, partial beating matches.
fn parse_parity(text: &str) -> BTreeMap<String, Parity> {
    let mut map: BTreeMap<String, Parity> = BTreeMap::new();
    for line in text.lines() {
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        let (Some(name), Some(status)) = (cells.get(1), cells.get(3)) else {
            continue;
        };
        let Some(name) = name.strip_prefix('`').and_then(|n| n.strip_suffix('`')) else {
            continue;
        };
        let status = match *status {
            "matches" => "matches",
            "partial" => "partial",
            "planned" => "planned",
            _ => continue,
        };
        let first = name.split_whitespace().next().unwrap_or(name).to_string();
        let e = map.entry(first).or_insert(Parity { status, forms: 0 });
        e.forms += 1;
        let rank = |s: &str| match s {
            "planned" => 0,
            "partial" => 1,
            _ => 2,
        };
        if rank(status) < rank(e.status) {
            e.status = status;
        }
    }
    map
}

/// The names the engine's `match name {` dispatch has arms for.
fn parse_statcraft_dispatch(text: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut inside = false;
    for line in text.lines() {
        if !inside {
            inside = line.trim() == "match name {";
            continue;
        }
        if line == "    }" {
            break;
        }
        if indent(line) == 8 {
            collect_arm_names(line, &mut names);
        }
    }
    names
}

fn indent(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// The string literals in the head of a match arm (before `=>` or ` if `).
fn collect_arm_names(line: &str, into: &mut BTreeSet<String>) {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('"') {
        return;
    }
    let head = trimmed.split("=>").next().unwrap_or(trimmed);
    let head = head.split(" if ").next().unwrap_or(head);
    for (i, part) in head.split('"').enumerate() {
        if i % 2 == 1 && is_word(part) {
            into.insert(part.to_string());
        }
    }
}

struct Package;

/// `| name | author | installs | runs | notes |` rows of `docs/packages.md`
/// whose package runs at least in part.
fn parse_packages(text: &str) -> BTreeMap<String, Package> {
    let mut map = BTreeMap::new();
    for line in text.lines() {
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        let (Some(name), Some(runs)) = (cells.get(1), cells.get(4)) else {
            continue;
        };
        if !(runs.starts_with("yes") || runs.starts_with("mostly")) {
            continue;
        }
        let word: String = name
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect::<String>()
            .to_ascii_lowercase();
        if is_word(&word) {
            for (package, commands) in PACKAGE_COMMANDS {
                if *package == word {
                    for c in *commands {
                        map.insert((*c).to_string(), Package);
                    }
                }
            }
            map.insert(word, Package);
        }
    }
    map
}

/// The commands a package brings besides the one named after it (its own
/// help lists them; here, names only).
const PACKAGE_COMMANDS: &[(&str, &[&str])] = &[
    ("estout", &["esttab", "eststo", "estadd"]),
    (
        "spost13",
        &["listcoef", "fitstat", "mtable", "mchange", "mgen"],
    ),
];

// ---- notes ------------------------------------------------------------

/// Note files, with the command words their headings name in backticks.
#[derive(Default)]
struct Notes {
    /// file name (with folder prefix) and the command words it covers
    files: Vec<(String, BTreeSet<String>)>,
}

impl Notes {
    fn add(&mut self, path: String, text: &str) {
        let mut words = BTreeSet::new();
        if let Some(stem) = Path::new(&path).file_stem().and_then(|s| s.to_str()) {
            words.insert(stem.to_string());
        }
        for line in text.lines().filter(|l| l.starts_with('#')) {
            for (i, part) in line.split('`').enumerate() {
                if i % 2 == 1 {
                    for w in part.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_')) {
                        if is_word(w) {
                            words.insert(w.to_string());
                        }
                    }
                }
            }
        }
        self.files.push((path, words));
    }

    /// Up to four note paths for a command, the one named after it first.
    fn paths_for(&self, command: &str) -> String {
        let mut own = Vec::new();
        let mut other = Vec::new();
        for (path, words) in &self.files {
            if !words.contains(command) {
                continue;
            }
            let stem = Path::new(path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default();
            if stem == command {
                own.push(path.clone());
            } else {
                other.push(path.clone());
            }
        }
        own.extend(other);
        own.truncate(4);
        own.join(";")
    }
}

fn rcraft_notes() -> std::result::Result<Notes, String> {
    let dir = root().join("plan").join("stata");
    let mut notes = Notes::default();
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "md"))
        .collect();
    files.sort();
    for f in files {
        let name = f
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string();
        if name == "README.md" {
            continue;
        }
        let text = std::fs::read_to_string(&f).unwrap_or_default();
        notes.add(format!("plan/stata/{name}"), &text);
    }
    Ok(notes)
}

fn statcraft_notes(repo: &Path) -> std::result::Result<Notes, String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["ls-tree", "-r", "--name-only", STATCRAFT_REF, "plan/stata"])
        .output()
        .map_err(|e| format!("git: {e}"))?;
    let mut notes = Notes::default();
    for path in String::from_utf8_lossy(&out.stdout).lines() {
        if !path.ends_with(".md") || path.ends_with("README.md") {
            continue;
        }
        let text = git_show(repo, path).unwrap_or_default();
        notes.add(path.to_string(), &text);
    }
    Ok(notes)
}

// ---- RCraft's dispatch --------------------------------------------------

/// The command names `Translator::dispatch` has arms for: the string literals
/// at the head of each arm (indent 12), and `abbrev(w, "name", n)` guards.
fn parse_rcraft_dispatch(text: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut inside = false;
    // an arm whose head runs over several lines (`"a"` then `| "b" =>`)
    let mut open_head = false;
    for line in text.lines() {
        if !inside {
            inside = line.contains("fn dispatch(&mut self, c: &Command, out: &mut Out)");
            continue;
        }
        if line.contains("is not translated yet\")))") && line.contains("other =>") {
            break;
        }
        let trimmed = line.trim_start();
        let level = indent(line);
        if level == 12 && (trimmed.starts_with('"') || trimmed.starts_with("w if ")) {
            // an arm that only refuses (`=> Err(no(...))`) translates nothing
            if trimmed.contains("=> Err(no(") {
                open_head = false;
                continue;
            }
            collect_arm_names(line, &mut names);
            if let Some(i) = trimmed.find("abbrev(w, \"") {
                let rest = trimmed.get(i + "abbrev(w, \"".len()..).unwrap_or_default();
                let word = rest.split('"').next().unwrap_or_default();
                if is_word(word) {
                    names.insert(word.to_string());
                }
            }
            open_head = trimmed.starts_with('"') && !trimmed.contains("=>");
        } else if open_head && trimmed.starts_with('|') {
            collect_arm_names(trimmed.trim_start_matches('|').trim_start(), &mut names);
            open_head = !trimmed.contains("=>") && !trimmed.contains('{');
        } else {
            open_head = false;
        }
    }
    names
}

// ---- the dry runs ----------------------------------------------------------

/// `git archive` of StatCraft's probe do-files from `origin/main`.
fn extract_probes(repo: &Path, work: &Path) -> std::result::Result<PathBuf, String> {
    let dir = work.join("probes");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let archive = work.join("probes.tar");
    let status = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["archive", "--format=tar", "-o"])
        .arg(&archive)
        .args([STATCRAFT_REF, "observe/dofiles"])
        .status()
        .map_err(|e| format!("git archive: {e}"))?;
    if !status.success() {
        return Err("git archive of the probe do-files failed".into());
    }
    let status = Command::new("tar")
        .arg("-xf")
        .arg(&archive)
        .arg("-C")
        .arg(&dir)
        .status()
        .map_err(|e| format!("tar: {e}"))?;
    let _ = std::fs::remove_file(&archive);
    if !status.success() {
        return Err("tar could not unpack the probe do-files".into());
    }
    Ok(dir.join("observe").join("dofiles"))
}

/// Runs the translator over a folder of do-files without R (the
/// `many_dofiles` test) and keeps its per-command counts in `out`.
fn dry_run(dir: &Path, out: &Path) -> Result {
    println!("stata-matrix: dry run over {}", dir.display());
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let output = Command::new(cargo)
        .args([
            "test",
            "-p",
            "rcraft-statatr",
            "--test",
            "many_dofiles",
            "the_share_translated_per_tier",
            "--",
            "--nocapture",
        ])
        .env("RCRAFT_DOFILES_DIR", dir)
        .env("RCRAFT_COVERAGE_COMMANDS", out)
        .current_dir(root())
        .output()
        .map_err(|e| format!("could not run cargo: {e}"))?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        return Err(format!("the dry run failed:\n{stderr}"));
    }
    for line in stderr.lines().filter(|l| l.starts_with("source lines")) {
        println!("stata-matrix: {line}");
    }
    Ok(())
}

#[derive(Default, Clone)]
struct Count {
    files: usize,
    lines: usize,
    translated: usize,
    not: usize,
}

#[derive(Default)]
struct Counts {
    by_command: BTreeMap<String, Count>,
    /// command to the file names it appears in
    files_of: BTreeMap<String, BTreeSet<String>>,
}

impl Counts {
    /// The probe numbers (`NNN` of `NNN-name.do`) of a command, the first
    /// twelve, then how many more.
    fn numbers_of(&self, command: &str) -> String {
        let Some(files) = self.files_of.get(command) else {
            return String::new();
        };
        let numbers: Vec<String> = files
            .iter()
            .filter_map(|f| {
                let n: String = f.chars().take_while(char::is_ascii_digit).collect();
                (!n.is_empty()).then_some(n)
            })
            .collect();
        let shown: Vec<&str> = numbers.iter().take(12).map(String::as_str).collect();
        let mut s = shown.join(" ");
        if numbers.len() > 12 {
            let _ = write!(s, " +{}", numbers.len() - 12);
        }
        s
    }
}

/// `command, file, lines, translated, not, later` rows of the dry run, with
/// abbreviations (`gen`, `gr`) resolved to the full name.
fn read_counts(file: &Path) -> std::result::Result<Counts, String> {
    let text = std::fs::read_to_string(file).map_err(|e| format!("{}: {e}", file.display()))?;
    let mut counts = Counts::default();
    for line in text.lines() {
        let cells: Vec<&str> = line.split('\t').collect();
        let [word, name, lines, translated, not, _later] = cells.as_slice() else {
            continue;
        };
        let word = rcraft_statatr::lang::commands::resolve(word);
        let num = |s: &str| s.parse::<usize>().unwrap_or(0);
        let c = counts.by_command.entry(word.to_string()).or_default();
        c.files += 1;
        c.lines += num(lines);
        c.translated += num(translated);
        c.not += num(not);
        counts
            .files_of
            .entry(word.to_string())
            .or_default()
            .insert((*name).to_string());
    }
    Ok(counts)
}

// ---- the ledger -----------------------------------------------------------

struct LedgerRow {
    commands: BTreeSet<String>,
    project: String,
    session: String,
    state: String,
}

impl LedgerRow {
    /// A row is held while a session may still be working on it: its state
    /// says nothing of being done, released, pushed, merged or idle.
    fn held(&self) -> bool {
        let s = self.state.to_ascii_lowercase();
        !["done", "released", "pushed", "merged", "idle"]
            .iter()
            .any(|w| s.contains(w))
    }

    fn state_short(&self) -> String {
        self.state.chars().take(60).collect()
    }
}

fn parse_ledger(text: &str) -> Vec<LedgerRow> {
    let mut rows = Vec::new();
    for line in text.lines() {
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        let [_, command, project, session, state, ..] = cells.as_slice() else {
            continue;
        };
        if *command == "Command" || command.starts_with("---") || project.is_empty() {
            continue;
        }
        let commands = command
            .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .map(str::to_ascii_lowercase)
            .filter(|w| is_word(w))
            .collect();
        rows.push(LedgerRow {
            commands,
            project: (*project).to_string(),
            session: (*session).to_string(),
            state: (*state).to_string(),
        });
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parity_rows_fold_into_their_first_word() {
        let t = "| `import delimited` | first | matches |\n| `import rdata` | later | partial |\n| `use` | first | matches |\n";
        let p = parse_parity(t);
        assert_eq!(
            p.get("import").map(|p| (p.status, p.forms)),
            Some(("partial", 2))
        );
        assert_eq!(p.get("use").map(|p| p.status), Some("matches"));
    }

    #[test]
    fn dispatch_arms_are_read_with_their_aliases() {
        let t = "    match name {\n        \"display\" => a(),\n        \"stdescribe\" | \"stdes\" => b(),\n        _ => c(),\n    }\n";
        let n = parse_statcraft_dispatch(t);
        assert!(n.contains("display") && n.contains("stdes"), "{n:?}");
    }

    #[test]
    fn a_ledger_row_is_held_until_done_or_released() {
        let rows = parse_ledger(
            "| ttest, list | RCraft | s1 | on hold for the user's go | |\n| stepwise | RCraft | none | done: c101ddb | |\n",
        );
        assert_eq!(rows.len(), 2);
        assert!(rows.first().is_some_and(LedgerRow::held));
        assert!(rows.get(1).is_some_and(|r| !r.held()));
    }

    #[test]
    fn csv_cells_with_commas_are_quoted() {
        assert_eq!(csv_cell("a,b"), "\"a,b\"");
        assert_eq!(csv_cell("plain"), "plain");
    }
}
