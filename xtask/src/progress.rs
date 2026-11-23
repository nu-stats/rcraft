//! `cargo xtask progress`: the roadmap's "% complete", worked out by the
//! machine from `plan/progress.txt` and the code, never typed by hand.
//!
//! `plan/progress.txt` has one line for each task id in `ROADMAP.md`
//! (`P2C.12a`, `P2C.12b` and so on count as the one task `P2C.12`):
//!
//! ```text
//! P0.1 | done
//! P6.7 | open | 2026-11-27 | entry: Principal components; test: crates/engine/tests/multivariate.rs::pca_matches_stata
//! ```
//!
//! The fields are the id, the state (`done`, `part` or `open`), the day the
//! plan has it done, and what "done" means, as checks separated by `;`:
//!
//! - `entry: <name>`: the catalog has a menu entry of that name, `Verified`;
//! - `test: <file>::<fn>`: that test exists and is not `#[ignore]`d (CI runs
//!   every test, so a merged test passes);
//! - `file: <path>`: the file exists;
//! - `owner: <what>`: the owner has to see it (a manual read, say); the task
//!   is done only once its line says `done` and the other checks hold.
//!
//! A task whose checks all hold counts as done whatever its line says (and
//! the report asks for the line to be changed); a line that says `done` while
//! a check fails is an error, so CI fails. Done counts 1, part done 0.5.
//! Tasks done before the ledger began (22 Nov 2026, from `plan/AUDIT.md` and
//! the roadmap's notes) carry no checks.
//!
//! `cargo xtask progress` prints the figure, the pace against the planned
//! days and every task left; `--check` (which `cargo xtask ci` runs) prints
//! the figure line only. On GitHub Actions the report also goes to the job
//! summary, so every run shows it. `--today YYYY-MM-DD` sets the day.

use std::collections::BTreeSet;
use std::path::Path;

use rcraft_catalog::{Status, ENTRIES};

use crate::{root, Result};

/// 98% of the roadmap, the target.
const TARGET: f64 = 0.98;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Done,
    Part,
    Open,
}

#[derive(Debug, PartialEq, Eq)]
enum Need {
    Entry(String),
    Test { file: String, name: String },
    File(String),
    Owner(String),
}

#[derive(Debug)]
struct Task {
    id: String,
    state: State,
    planned: Option<(i32, u32, u32)>,
    needs: Vec<Need>,
}

/// A task once its checks have been run.
struct Scored<'a> {
    task: &'a Task,
    /// Checks that hold, of the machine checks (owner checks not counted).
    met: usize,
    machine: usize,
    /// What counts: the checks win over the line.
    state: State,
    /// The checks hold but the line does not say `done`.
    relabel: bool,
}

pub fn run(args: &[String]) -> Result {
    let check = args.iter().any(|a| a == "--check");
    let today = match args.iter().position(|a| a == "--today") {
        Some(i) => parse_date(args.get(i + 1).map(String::as_str).unwrap_or(""))
            .ok_or("--today wants YYYY-MM-DD")?,
        None => today(),
    };
    let root = root();
    let roadmap =
        std::fs::read_to_string(root.join("ROADMAP.md")).map_err(|e| format!("ROADMAP.md: {e}"))?;
    let ledger = std::fs::read_to_string(root.join("plan/progress.txt"))
        .map_err(|e| format!("plan/progress.txt: {e}"))?;
    let tasks = parse_ledger(&ledger)?;
    check_ids(&roadmap_ids(&roadmap), &tasks)?;
    let scored = score(&root, &tasks)?;
    let report = report(&scored, today);
    if check {
        println!("{}", report.lines().next().unwrap_or_default());
    } else {
        println!("{report}");
    }
    if let Ok(path) = std::env::var("GITHUB_STEP_SUMMARY") {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new().append(true).open(path) {
            let _ = writeln!(f, "## Roadmap progress\n\n{report}");
        }
    }
    Ok(())
}

/// The distinct task ids of the roadmap: lines starting `- P<phase>.<n>`,
/// with a sub-task's letters dropped.
fn roadmap_ids(roadmap: &str) -> BTreeSet<String> {
    roadmap
        .lines()
        .filter_map(|l| l.strip_prefix("- "))
        .filter_map(task_id)
        .collect()
}

/// `P2C.12ab Something` -> `P2C.12`; `None` when the text is not a task id.
fn task_id(text: &str) -> Option<String> {
    let rest = text.strip_prefix('P')?;
    let (phase, after) = rest.split_once('.')?;
    let mut chars = phase.chars();
    if !chars.next()?.is_ascii_digit() || !chars.all(|c| c.is_ascii_alphanumeric()) {
        return None;
    }
    let digits: String = after.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        return None;
    }
    Some(format!("P{phase}.{digits}"))
}

fn parse_ledger(text: &str) -> Result<Vec<Task>> {
    let mut tasks = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let at = |m: &str| format!("plan/progress.txt line {}: {m}", n + 1);
        let fields: Vec<&str> = line.split('|').map(str::trim).collect();
        let id = fields.first().copied().unwrap_or_default().to_string();
        if task_id(&id).as_deref() != Some(id.as_str()) {
            return Err(at(&format!("`{id}` is not a task id")));
        }
        let state = match fields.get(1).copied() {
            Some("done") => State::Done,
            Some("part") => State::Part,
            Some("open") => State::Open,
            other => return Err(at(&format!("state {other:?}: done, part or open"))),
        };
        let planned = match fields.get(2).copied() {
            None | Some("") => None,
            Some(d) => Some(parse_date(d).ok_or_else(|| at(&format!("date `{d}`")))?),
        };
        let mut needs = Vec::new();
        for need in fields
            .get(3)
            .copied()
            .unwrap_or("")
            .split(';')
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            let (kind, value) = need
                .split_once(':')
                .ok_or_else(|| at(&format!("check `{need}` has no kind")))?;
            let value = value.trim().to_string();
            needs.push(match kind.trim() {
                "entry" => Need::Entry(value),
                "file" => Need::File(value),
                "owner" => Need::Owner(value),
                "test" => {
                    let (file, name) = value
                        .split_once("::")
                        .ok_or_else(|| at(&format!("test `{value}` wants <file>::<fn>")))?;
                    Need::Test {
                        file: file.to_string(),
                        name: name.to_string(),
                    }
                }
                other => return Err(at(&format!("check kind `{other}`"))),
            });
        }
        if state != State::Done && needs.is_empty() {
            return Err(at(&format!(
                "{id} is not done and says nothing about what done means"
            )));
        }
        if fields.len() > 4 {
            return Err(at("more than four fields"));
        }
        tasks.push(Task {
            id,
            state,
            planned,
            needs,
        });
    }
    Ok(tasks)
}

fn check_ids(roadmap: &BTreeSet<String>, tasks: &[Task]) -> Result {
    let mut seen = BTreeSet::new();
    for t in tasks {
        if !seen.insert(t.id.clone()) {
            return Err(format!("plan/progress.txt lists {} twice", t.id));
        }
    }
    let missing: Vec<_> = roadmap.difference(&seen).cloned().collect();
    let extra: Vec<_> = seen.difference(roadmap).cloned().collect();
    if !missing.is_empty() {
        return Err(format!(
            "plan/progress.txt lacks roadmap tasks: {}",
            missing.join(", ")
        ));
    }
    if !extra.is_empty() {
        return Err(format!(
            "plan/progress.txt has tasks the roadmap does not: {}",
            extra.join(", ")
        ));
    }
    Ok(())
}

fn score<'a>(root: &Path, tasks: &'a [Task]) -> Result<Vec<Scored<'a>>> {
    let mut out = Vec::new();
    for task in tasks {
        let mut met = 0;
        let mut machine = 0;
        let mut owner = false;
        for need in &task.needs {
            let holds = match need {
                Need::Owner(_) => {
                    owner = true;
                    continue;
                }
                Need::Entry(name) => ENTRIES
                    .iter()
                    .any(|e| e.name == name && e.status == Status::Verified),
                Need::File(path) => root.join(path).exists(),
                Need::Test { file, name } => test_is_live(root, file, name),
            };
            machine += 1;
            met += usize::from(holds);
        }
        let checks_hold = met == machine && machine > 0;
        if task.state == State::Done && !task.needs.is_empty() && met < machine {
            return Err(format!(
                "plan/progress.txt says {} is done, but {} of its {} checks fail",
                task.id,
                machine - met,
                machine
            ));
        }
        let (state, relabel) = if checks_hold && !owner && task.state != State::Done {
            (State::Done, true)
        } else {
            (task.state, false)
        };
        out.push(Scored {
            task,
            met,
            machine,
            state,
            relabel,
        });
    }
    Ok(out)
}

/// The test `name` is in `file`, marked `#[test]` and not `#[ignore]`d.
fn test_is_live(root: &Path, file: &str, name: &str) -> bool {
    let Ok(text) = std::fs::read_to_string(root.join(file)) else {
        return false;
    };
    let lines: Vec<&str> = text.lines().collect();
    let head = format!("fn {name}(");
    lines.iter().enumerate().any(|(i, l)| {
        if !l.trim_start().starts_with(&head) {
            return false;
        }
        let attrs: Vec<&str> = lines
            .get(i.saturating_sub(4)..i)
            .unwrap_or_default()
            .iter()
            .map(|a| a.trim())
            .collect();
        attrs.iter().any(|a| a.starts_with("#[test]"))
            && !attrs.iter().any(|a| a.starts_with("#[ignore"))
    })
}

fn report(scored: &[Scored<'_>], today: (i32, u32, u32)) -> String {
    let total = scored.len();
    let count = |s: State| scored.iter().filter(|t| t.state == s).count();
    let (done, part, open) = (count(State::Done), count(State::Part), count(State::Open));
    let points = done as f64 + part as f64 * 0.5;
    let pct = points / total as f64 * 100.0;
    let target = (TARGET * total as f64).ceil();
    let mut out = format!(
        "Roadmap: {} of {total} tasks, {pct:.1}% ({done} done, {part} part done, {open} not started); \
         98% needs {target}, {} to go.\n",
        fmt_points(points),
        fmt_points((target - points).max(0.0)),
    );
    let due: Vec<&Scored<'_>> = scored
        .iter()
        .filter(|t| t.task.planned.is_some_and(|d| d <= today))
        .collect();
    let late: Vec<&str> = due
        .iter()
        .filter(|t| t.state != State::Done)
        .map(|t| t.task.id.as_str())
        .collect();
    out.push_str(&format!(
        "\nPace on {}: {} tasks planned done by today, {} of them done.",
        fmt_date(today),
        due.len(),
        due.len() - late.len()
    ));
    if late.is_empty() {
        out.push_str(" On plan.\n");
    } else {
        out.push_str(&format!(" Behind: {}.\n", late.join(", ")));
    }
    // The day the plan's dates reach 98%: the points still owed, task by
    // task in planned order, added to what is done.
    let mut planned: Vec<&Scored<'_>> = scored
        .iter()
        .filter(|t| t.state != State::Done && t.task.planned.is_some())
        .collect();
    planned.sort_by_key(|t| t.task.planned);
    let mut running = points;
    let reach = planned.iter().find_map(|t| {
        running += if t.state == State::Part { 0.5 } else { 1.0 };
        (running >= target).then_some(t.task.planned).flatten()
    });
    match reach {
        _ if points >= target => out.push_str("98% is reached.\n"),
        Some(day) => out.push_str(&format!(
            "At the planned dates, 98% is reached on {}.\n",
            fmt_date(day)
        )),
        None => out.push_str("The planned dates do not reach 98%.\n"),
    }
    let relabel: Vec<&str> = scored
        .iter()
        .filter(|t| t.relabel)
        .map(|t| t.task.id.as_str())
        .collect();
    if !relabel.is_empty() {
        out.push_str(&format!(
            "\nDone by their checks; change their lines to `done`: {}.\n",
            relabel.join(", ")
        ));
    }
    let mut left: Vec<&Scored<'_>> = scored.iter().filter(|t| t.state != State::Done).collect();
    left.sort_by_key(|t| t.task.planned.unwrap_or((9999, 12, 31)));
    if !left.is_empty() {
        out.push_str("\n| Task | State | Planned | Checks met | Owner |\n|---|---|---|---|---|\n");
        for t in left {
            let owner: Vec<&str> = t
                .task
                .needs
                .iter()
                .filter_map(|n| match n {
                    Need::Owner(w) => Some(w.as_str()),
                    _ => None,
                })
                .collect();
            out.push_str(&format!(
                "| {} | {} | {} | {} of {} | {} |\n",
                t.task.id,
                if t.state == State::Part {
                    "part done"
                } else {
                    "not started"
                },
                t.task
                    .planned
                    .map(fmt_date)
                    .unwrap_or_else(|| "not planned".into()),
                t.met,
                t.machine,
                owner.join("; ")
            ));
        }
    }
    out
}

fn fmt_points(p: f64) -> String {
    if p.fract() == 0.0 {
        format!("{p:.0}")
    } else {
        format!("{p:.1}")
    }
}

fn fmt_date((y, m, d): (i32, u32, u32)) -> String {
    format!("{y:04}-{m:02}-{d:02}")
}

fn parse_date(s: &str) -> Option<(i32, u32, u32)> {
    let mut it = s.split('-');
    let y = it.next()?.parse().ok()?;
    let m = it.next()?.parse().ok()?;
    let d = it.next()?.parse().ok()?;
    (it.next().is_none() && (1..=12).contains(&m) && (1..=31).contains(&d)).then_some((y, m, d))
}

/// Today's date in UTC, from the system clock.
fn today() -> (i32, u32, u32) {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    civil((secs / 86_400) as i64)
}

/// Days since 1970-01-01 to a calendar date (Howard Hinnant's algorithm).
fn civil(days: i64) -> (i32, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = (yoe + era * 400 + i64::from(m <= 2)) as i32;
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sub_tasks_count_as_their_task() {
        assert_eq!(task_id("P2C.12ab Foundations").as_deref(), Some("P2C.12"));
        assert_eq!(task_id("P6.7 Multivariate").as_deref(), Some("P6.7"));
        assert_eq!(
            task_id("P1A.10 \"Package needed\"").as_deref(),
            Some("P1A.10")
        );
        assert_eq!(task_id("Packages are third-party"), None);
    }

    #[test]
    fn dates_come_from_days() {
        assert_eq!(civil(0), (1970, 1, 1));
        assert_eq!(civil(20_780), (2026, 11, 23));
        assert_eq!(parse_date("2026-12-05"), Some((2026, 12, 5)));
        assert_eq!(parse_date("2026-13-05"), None);
    }

    #[test]
    fn a_ledger_line_reads_its_checks() {
        let t = parse_ledger(
            "# note\nP0.1 | done\nP6.7 | open | 2026-11-27 | entry: PCA; test: a.rs::b; owner: read it\n",
        );
        let t = t.unwrap_or_default();
        assert_eq!(t.len(), 2);
        let p = t.get(1);
        assert_eq!(p.and_then(|p| p.planned), Some((2026, 11, 27)));
        assert_eq!(
            p.map(|p| p.needs.as_slice()).unwrap_or_default(),
            vec![
                Need::Entry("PCA".into()),
                Need::Test {
                    file: "a.rs".into(),
                    name: "b".into()
                },
                Need::Owner("read it".into()),
            ]
        );
        assert!(parse_ledger("P6.7 | open | 2026-11-27\n").is_err());
        assert!(parse_ledger("P6.7 | maybe\n").is_err());
    }

    #[test]
    fn the_ledger_and_the_roadmap_agree() {
        let root = root();
        let roadmap = std::fs::read_to_string(root.join("ROADMAP.md")).unwrap_or_default();
        let ledger = std::fs::read_to_string(root.join("plan/progress.txt")).unwrap_or_default();
        let tasks = parse_ledger(&ledger);
        assert!(tasks.is_ok(), "{tasks:?}");
        let tasks = tasks.unwrap_or_default();
        assert_eq!(check_ids(&roadmap_ids(&roadmap), &tasks), Ok(()));
        assert!(score(&root, &tasks).is_ok());
    }
}
