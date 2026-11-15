//! The checks that move the Stata mode entries and Vignettes of the catalog
//! to Verified (AGENTS.md section 5). Each goes the way the app goes when the
//! user uses the entry, in a real R, and checks the result against a fixed
//! answer:
//!
//! - Stata mode switch, Translate Stata code, Stata and R side by side, Open
//!   do-file and run, Translate do-file: do-files of the Stata corpus
//!   (`engine/tests/stata_corpus`, made-up data, with Stata 19's own results
//!   recorded beside each) typed line by line in Stata mode, put in the
//!   Script Editor, run from File > Run do-file… and saved as an R script and
//!   a Quarto document; every value printed must be Stata 19's to the file's
//!   tolerance.
//! - Do-file dry-run report: a made-up do-file whose scan is checked against
//!   what then happens when it runs.
//! - Stata differences guide: every command of the corpus has a page, the
//!   page opens from a line, and what the pages say R itself does holds in R.
//! - Vignettes: the list and a vignette's code as R's own files have them.
//!
//! Each test records the R and package versions it ran with and says when R
//! is not the one `ENVIRONMENT` pins. Without R, or without the packages the
//! corpus needs, a test is skipped with a note.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use rcraft_rcodegen::help_docs::{kind_of, FileKind};
use rcraft_statatr::differences;
use rcraft_statatr::dofile::{FileStatus, Ran};
use rcraft_statatr::live::{HELPERS_BEGIN, HELPERS_END};

use super::*;

/// The packages the corpus files used here need.
const NEEDS: &[&str] = &[
    "dplyr", "haven", "readr", "MASS", "sandwich", "survival", "nnet",
];

/// Runs frames until `done` or `secs` seconds have passed.
fn frames_until(
    app: &mut App,
    ctx: &egui::Context,
    secs: u64,
    done: impl Fn(&App) -> bool,
) -> bool {
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1000.0, 700.0));
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(secs) {
        let raw = egui::RawInput {
            screen_rect: Some(screen),
            time: Some(started.elapsed().as_secs_f64()),
            ..Default::default()
        };
        let _ = ctx.run(raw, |ctx| app.ui(ctx));
        if done(app) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    false
}

fn settle(app: &mut App, ctx: &egui::Context) {
    assert!(
        frames_until(app, ctx, 300, |a| a.is_settled()),
        "R did not settle"
    );
}

/// The text of a block of Results.
fn block_text(b: &rcraft_engine::ResultBlock) -> String {
    b.items
        .iter()
        .map(|i| match i {
            Item::Text { text, .. } => text.clone(),
            Item::Warning(w) => format!("warning: {w}\n"),
            Item::Error { message, .. } => format!("error: {message}\n"),
            Item::Notice(n) => n.clone(),
            Item::Plot { .. } => String::new(),
        })
        .collect()
}

/// How many blocks Results holds.
fn blocks(app: &App) -> usize {
    app.engine().map_or(0, |e| e.results().blocks().len())
}

/// The text of the blocks from `from` on.
fn text_since(app: &App, from: usize) -> String {
    app.engine()
        .map(|e| {
            e.results()
                .blocks()
                .iter()
                .skip(from)
                .map(block_text)
                .collect()
        })
        .unwrap_or_default()
}

/// Runs R code as typed; how it ended and what it printed.
fn r(app: &mut App, ctx: &egui::Context, code: &str) -> (Outcome, String) {
    let from = blocks(app);
    app.perform(ctx, Action::Submit(Source::Typed, code.to_string()));
    settle(app, ctx);
    let outcome = app
        .engine()
        .and_then(|e| e.history().entries().last().and_then(|h| h.outcome))
        .unwrap_or(Outcome::Error);
    (outcome, text_since(app, from))
}

/// A started app with R ready and the packages there, or `None` (skipped).
fn start() -> Option<(egui::Context, App)> {
    let ctx = egui::Context::default();
    let mut app = App::new(&ctx);
    if !frames_until(&mut app, &ctx, 120, |a| a.is_settled()) || !app.is_ready() {
        eprintln!("skipped: R did not start");
        return None;
    }
    let need = NEEDS
        .iter()
        .map(|p| format!("requireNamespace(\"{p}\", quietly = TRUE)"))
        .collect::<Vec<_>>()
        .join(", ");
    let (o, _) = r(&mut app, &ctx, &format!("stopifnot({need})"));
    if o != Outcome::Ok {
        eprintln!("skipped: the corpus needs {}", NEEDS.join(", "));
        return None;
    }
    versions(&mut app, &ctx);
    Some((ctx, app))
}

/// The R and package versions, recorded (AGENTS.md section 5).
fn versions(app: &mut App, ctx: &egui::Context) {
    let pkgs = NEEDS
        .iter()
        .map(|p| format!("\"{p}\""))
        .collect::<Vec<_>>()
        .join(", ");
    let (_, text) = r(
        app,
        ctx,
        &format!(
            "cat(paste(R.version$major, R.version$minor, sep = '.'), \
             vapply(c({pkgs}), function(p) paste(p, as.character(utils::packageVersion(p))), \"\"), sep = '\\n')"
        ),
    );
    let mut lines = text.lines();
    let ran = lines.next().unwrap_or("").trim().to_string();
    let pinned = include_str!("../../../../ENVIRONMENT")
        .lines()
        .find_map(|l| l.strip_prefix("R_VERSION="))
        .unwrap_or("")
        .trim();
    eprintln!(
        "ran with R {ran} ({}); ENVIRONMENT pins R {pinned}",
        lines.collect::<Vec<_>>().join(", ")
    );
    if ran != pinned {
        eprintln!("note: this R is not the pinned one, so the run is not a Verified one");
    }
}

fn corpus() -> PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("engine")
        .join("tests")
        .join("stata_corpus");
    std::path::absolute(&dir).unwrap_or(dir)
}

fn slashes(p: &Path) -> String {
    p.display().to_string().replace('\\', "/")
}

fn read(name: &str) -> String {
    std::fs::read_to_string(corpus().join(name)).unwrap_or_default()
}

/// The `@ key value ...` lines printed, every value a number (`.` missing).
fn values(text: &str) -> BTreeMap<String, Vec<f64>> {
    text.lines()
        .filter_map(|l| {
            let mut w = l.trim_start().strip_prefix("@ ")?.split_whitespace();
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

/// What differs from Stata 19's recorded results for corpus file `name`.
fn against_stata(name: &str, printed: &str, how: &str, wrong: &mut Vec<String>) {
    let dofile = read(&format!("{name}.do"));
    let stata = values(&read(&format!("{name}.recorded")));
    if stata.is_empty() {
        wrong.push(format!("{name}: nothing recorded"));
        return;
    }
    let tol: f64 = dofile
        .lines()
        .find_map(|l| l.strip_prefix("* tolerance: "))
        .and_then(|t| t.trim().parse().ok())
        .unwrap_or(1e-8);
    let got = values(printed);
    for (key, want) in &stata {
        let Some(have) = got.get(key) else {
            wrong.push(format!("{how} {name}: {key} was not printed"));
            continue;
        };
        let same = have.len() == want.len()
            && have.iter().zip(want).all(|(h, w)| {
                (w.is_nan() && h.is_nan()) || (h - w).abs() <= tol * w.abs().max(h.abs())
            });
        if !same {
            wrong.push(format!("{how} {name}: {key} is {have:?}, Stata's {want:?}"));
        }
    }
}

/// A do-file cut into what a user types at once: a line, or a block from its
/// `{` to its `}`. Comments and blank lines are left out.
fn units(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut depth = 0i32;
    for line in text.lines() {
        let t = line.trim();
        if depth == 0 && cur.is_empty() && (t.is_empty() || t.starts_with('*')) {
            continue;
        }
        cur.push_str(line);
        cur.push('\n');
        if t.starts_with('}') {
            depth -= 1;
        }
        if t.ends_with('{') {
            depth += 1;
        }
        if depth <= 0 && !t.ends_with("///") {
            out.push(std::mem::take(&mut cur));
            depth = 0;
        }
    }
    if !cur.trim().is_empty() {
        out.push(cur);
    }
    out
}

/// Sets R's working folder to the corpus, as the do-file's own folder.
fn into_corpus(app: &mut App, ctx: &egui::Context) {
    let (o, t) = r(app, ctx, &format!("setwd(\"{}\")", slashes(&corpus())));
    assert_eq!(o, Outcome::Ok, "{t}");
}

/// Simple corpus files: one command a line, no values only R knows (the
/// Script Editor's pane is a dry run: such a value it shows as known only
/// when the do-file runs).
const PLAIN: &[&str] = &["regress", "ranktests", "tabulate_pct", "prtest"];

/// Stata mode switch: each command typed in the Command window in Stata
/// mode, one after another (a loop typed whole), as the app runs it.
#[test]
fn typed_stata_gives_statas_results() {
    let mut wrong = Vec::new();
    for name in PLAIN.iter().chain(&["labels2", "aware"]) {
        let Some((ctx, mut app)) = start() else {
            return;
        };
        into_corpus(&mut app, &ctx);
        app.set_stata_mode(&ctx, true);
        let from = blocks(&app);
        for unit in units(&read(&format!("{name}.do"))) {
            app.submit_stata(&ctx, unit.trim_end());
            settle(&mut app, &ctx);
        }
        against_stata(name, &text_since(&app, from), "typed", &mut wrong);
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// The R a script's Stata becomes, as the Script Editor's R pane shows it:
/// the helpers first, then each line's R under its Stata.
fn pane_r(app: &mut App, script: &str) -> (rcraft_statatr::data_r::Translation, String) {
    let t = app.stata.translation(script).clone();
    let mut code = format!("{HELPERS_BEGIN}\n");
    for h in &t.helpers {
        code.push_str(h);
        code.push('\n');
    }
    code.push_str(HELPERS_END);
    code.push('\n');
    for l in &t.lines {
        code.push_str(&l.script());
    }
    (t, code)
}

/// Translate Stata code: a do-file put in the Script Editor in Stata mode;
/// the R its pane shows, run on its own, gives Stata's results.
#[test]
fn the_r_the_script_editor_shows_gives_statas_results() {
    let mut wrong = Vec::new();
    for name in PLAIN {
        let Some((ctx, mut app)) = start() else {
            return;
        };
        into_corpus(&mut app, &ctx);
        app.set_stata_mode(&ctx, true);
        let text = read(&format!("{name}.do"));
        app.set_script(&text);
        let (t, code) = pane_r(&mut app, &text);
        for l in &t.lines {
            if let Err(e) = &l.result {
                wrong.push(format!(
                    "{name}: not translated: {} ({})",
                    l.stata, e.reason
                ));
            }
        }
        let from = blocks(&app);
        app.perform(&ctx, Action::Submit(Source::Script, code));
        settle(&mut app, &ctx);
        against_stata(name, &text_since(&app, from), "script", &mut wrong);
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// Stata and R side by side: each Stata line of the pane sits at its own line
/// of the script, and its R, run on its own, prints that line's results and
/// no other line's.
#[test]
fn side_by_side_each_stata_line_has_the_r_that_prints_its_results() {
    let Some((ctx, mut app)) = start() else {
        return;
    };
    into_corpus(&mut app, &ctx);
    app.set_stata_mode(&ctx, true);
    let name = "regress";
    let text = read(&format!("{name}.do"));
    let script_lines: Vec<&str> = text.lines().collect();
    let t = app.stata.translation(&text).clone();
    let mut wrong = Vec::new();
    let helpers = t.helpers.join("\n");
    let (o, out) = r(&mut app, &ctx, &helpers);
    assert_eq!(o, Outcome::Ok, "{out}");
    let mut printed = String::new();
    for l in &t.lines {
        // the line the pane marks for the cursor is the line the Stata is on
        let at = l.span.line.checked_sub(1).and_then(|i| script_lines.get(i));
        if at.map(|s| s.trim()) != l.stata.lines().next().map(str::trim) {
            wrong.push(format!(
                "line {} is {at:?}, the pane says {:?}",
                l.span.line, l.stata
            ));
        }
        let from = blocks(&app);
        app.perform(&ctx, Action::Submit(Source::Script, l.script()));
        settle(&mut app, &ctx);
        let own = text_since(&app, from);
        for key in values(&own).keys() {
            if !l.stata.contains(&format!("\"@ {key} ")) {
                wrong.push(format!("{key} printed by the R of {:?}", l.stata));
            }
        }
        for (key, _) in values(&l.stata.replace('"', "")) {
            if !values(&own).contains_key(&key) {
                wrong.push(format!("{:?} did not print {key}", l.stata));
            }
        }
        printed.push_str(&own);
    }
    against_stata(name, &printed, "side by side", &mut wrong);
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// Opens a corpus do-file in the do-file window and runs it to the end; what
/// R printed meanwhile.
fn run_dofile(app: &mut App, ctx: &egui::Context, name: &str) -> Result<String, String> {
    let from = blocks(app);
    app.open_dofile(&corpus().join(format!("{name}.do")), true);
    let ended = frames_until(app, ctx, 300, |a| {
        a.dofile
            .outcome()
            .is_some_and(|(done, _, problem)| done || problem)
    });
    let (done, ran, problem) = app.dofile.outcome().unwrap_or((false, Vec::new(), false));
    if !ended || !done || problem {
        let why: Vec<String> = ran
            .iter()
            .filter_map(|r| match r {
                Ran::Error(e) => Some(e.clone()),
                _ => None,
            })
            .collect();
        return Err(format!("{name}: the run did not finish: {why:?}"));
    }
    settle(app, ctx);
    Ok(text_since(app, from))
}

/// Do-files with loops, macros, an included file and values only R knows.
const RUN: &[&str] = &["aware", "control", "regress", "labels2"];

/// Open do-file and run: File > Run do-file… on corpus do-files.
#[test]
fn a_do_file_run_from_the_window_gives_statas_results() {
    let mut wrong = Vec::new();
    for name in RUN {
        let Some((ctx, mut app)) = start() else {
            return;
        };
        match run_dofile(&mut app, &ctx, name) {
            Ok(printed) => against_stata(name, &printed, "run", &mut wrong),
            Err(why) => wrong.push(why),
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// The R in a Quarto document's chunks, in order.
fn chunks(qmd: &str) -> String {
    let mut out = String::new();
    let mut inside = false;
    for l in qmd.lines() {
        if inside {
            if l.trim() == "```" {
                inside = false;
            } else {
                out.push_str(l);
                out.push('\n');
            }
        } else if l.trim() == "```{r}" {
            inside = true;
        }
    }
    out
}

/// Translate do-file: after a run, Save as R script and Save as Quarto; each
/// file's R, run in a fresh R from the do-file's folder, gives Stata's
/// results.
#[test]
fn a_saved_r_script_and_quarto_document_give_statas_results() {
    let mut wrong = Vec::new();
    for name in RUN {
        let Some((ctx, mut app)) = start() else {
            return;
        };
        if let Err(why) = run_dofile(&mut app, &ctx, name) {
            wrong.push(why);
            continue;
        }
        for quarto in [false, true] {
            let saved = app.dofile.saved(quarto);
            let code = if quarto { chunks(&saved) } else { saved };
            let how = if quarto { "Quarto" } else { "R script" };
            let Some((ctx2, mut fresh)) = start() else {
                return;
            };
            into_corpus(&mut fresh, &ctx2);
            let from = blocks(&fresh);
            fresh.perform(&ctx2, Action::Submit(Source::Script, code));
            settle(&mut fresh, &ctx2);
            against_stata(name, &text_since(&fresh, from), how, &mut wrong);
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// Do-file dry-run report: the scan of a made-up do-file says which lines
/// translate, which are not, which wait for R and which files are there;
/// the run then does just that.
#[test]
fn the_dry_run_report_says_what_the_run_then_does() {
    let Some((ctx, mut app)) = start() else {
        return;
    };
    let dir = std::env::temp_dir().join(format!("rcraft-dry-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let csv = "y,x\n2,1\n4,2\n5,3\n4,4\n6,5\n9,6\n";
    let dofile = "* made-up data for the dry-run report\n\
                  import delimited dry.csv, clear\n\
                  summarize x\n\
                  local m = r(mean)\n\
                  display \"@ m \" %24.17g `m'\n\
                  xtlogit y x\n\
                  regress y x\n\
                  display \"@ b \" %24.17g _b[x]\n\
                  use \"elsewhere/gone.dta\", clear\n";
    assert!(std::fs::write(dir.join("dry.csv"), csv).is_ok());
    assert!(std::fs::write(dir.join("dry.do"), dofile).is_ok());
    app.open_dofile(&dir.join("dry.do"), false);
    let scan = app.dofile.scan().clone();
    eprintln!("{}", app.dofile.scan_text());
    // eight commands; xtlogit is not translated, `m` waits for R
    assert_eq!(scan.lines, 8, "{scan:?}");
    assert_eq!(scan.not, 1, "{scan:?}");
    assert!(
        scan.reasons.iter().any(|(r, _)| r.contains("xtlogit")),
        "{:?}",
        scan.reasons
    );
    assert!(scan.later >= 1, "{scan:?}");
    assert_eq!(
        scan.clean + scan.notes + scan.by_rcraft + scan.not + scan.later,
        scan.lines
    );
    let file = |end: &str| {
        scan.files
            .iter()
            .find(|f| f.path.ends_with(end))
            .map(|f| f.status)
    };
    assert_eq!(file("dry.csv"), Some(FileStatus::There), "{:?}", scan.files);
    assert_eq!(
        file("gone.dta"),
        Some(FileStatus::Missing),
        "{:?}",
        scan.files
    );

    // the run: the missing file stops it at the last line, after the rest ran
    let from = blocks(&app);
    let session = app.session_name.clone();
    app.dofile.run_now(&session);
    assert!(frames_until(&mut app, &ctx, 120, |a| a
        .dofile
        .outcome()
        .is_some_and(|(done, _, problem)| done || problem)));
    if app.dofile.outcome().is_some_and(|(_, _, problem)| problem) {
        app.dofile.go_on(true);
        assert!(frames_until(&mut app, &ctx, 120, |a| a
            .dofile
            .outcome()
            .is_some_and(|(done, _, _)| done)));
    }
    settle(&mut app, &ctx);
    let (_, ran, _) = app.dofile.outcome().unwrap_or((false, Vec::new(), false));
    assert_eq!(ran.len(), 8, "{ran:?}");
    assert_eq!(ran.get(4), Some(&Ran::Skipped), "xtlogit: {ran:?}");
    assert!(matches!(ran.get(7), Some(Ran::Error(_))), "use: {ran:?}");
    let ok = ran.iter().filter(|r| **r == Ran::Ok).count();
    assert_eq!(ok, 6, "{ran:?}");
    // what the lines that waited for R printed is R's own answer
    let printed = values(&text_since(&app, from));
    let (o, want) = r(
        &mut app,
        &ctx,
        "local({ d <- data.frame(y = c(2, 4, 5, 4, 6, 9), x = 1:6); \
         cat(sprintf('%.17g', coef(lm(y ~ x, d))[['x']])) })",
    );
    assert_eq!(o, Outcome::Ok);
    assert_eq!(printed.get("m"), Some(&vec![3.5]), "{printed:?}");
    let b: f64 = want.trim().parse().unwrap_or(f64::NAN);
    let got = printed
        .get("b")
        .and_then(|v| v.first())
        .copied()
        .unwrap_or(f64::NAN);
    assert!((got - b).abs() <= 1e-12 * b.abs(), "{got} against {b}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// What the differences guide says R itself does, with R that prints TRUE
/// when it holds: the page, words of the page, the check.
const R_CLAIMS: &[(&str, &str, &str)] = &[
    (
        "general",
        "R's NA compared with anything is NA",
        "is.na(NA > 5) && is.na(NA == NA) && is.na(NA < -Inf)",
    ),
    (
        "general",
        "order() keeps them as they were",
        "identical(order(c(2, 1, 2, 1)), c(2L, 4L, 1L, 3L))",
    ),
    (
        "general",
        "R's qf() gives 0 at small p and an approximation for df2 above 4e5",
        "local({ f <- function(p, a, b) { x <- qbeta(p, a / 2, b / 2); (b / a) * x / (1 - x) }; \
         qf(1e-300, 3, 5) == 0 && f(1e-300, 3, 5) > 0 && \
         abs(qf(0.3, 3, 5e5) / f(0.3, 3, 5e5) - 1) > 1e-8 })",
    ),
    (
        "regress",
        "R's own lm() and glm() do the same by default (na.omit)",
        "local({ d <- data.frame(y = c(1, 3, NA, 4, 6), x = c(1, 2, 3, NA, 5)); \
         getOption('na.action') == 'na.omit' && nobs(lm(y ~ x, d)) == 3 && \
         nobs(glm(y ~ x, data = d)) == 3 })",
    ),
    (
        "regress",
        "R's lm() would drop the later one",
        "local({ d <- data.frame(y = c(1, 3, 2, 5, 4), x1 = 1:5); d$x2 <- 2 * d$x1; \
         b <- coef(lm(y ~ x1 + x2, d)); !is.na(b[['x1']]) && is.na(b[['x2']]) })",
    ),
    (
        "logit",
        "R's summary() uses the expected information for some models (probit)",
        "local({ set.seed(1); z <- rnorm(200); y <- rbinom(200, 1, pnorm(0.3 + 0.8 * z)); \
         m <- glm(y ~ z, family = binomial('probit')); \
         ll <- function(b) { e <- b[1] + b[2] * z; sum(y * pnorm(e, log.p = TRUE) + (1 - y) * pnorm(-e, log.p = TRUE)) }; \
         obs <- sqrt(diag(solve(-optimHess(coef(m), ll)))); \
         all(abs(obs / sqrt(diag(vcov(m))) - 1) > 1e-4) })",
    ),
    (
        "append",
        "R's rbind() would turn both into text",
        "is.character(rbind(data.frame(a = 'x'), data.frame(a = 1))$a)",
    ),
    (
        "correlate",
        "R's cor() would stop or need use = \"complete.obs\"",
        "is.na(cor(c(1, NA, 3, 4), c(1, 2, 3, 5))) && \
         !is.na(cor(c(1, NA, 3, 4), c(1, 2, 3, 5), use = 'complete.obs'))",
    ),
    (
        "egen",
        "R's mean() would give NA without na.rm",
        "is.na(mean(c(1, NA, 3))) && mean(c(1, NA, 3), na.rm = TRUE) == 2",
    ),
    (
        "histogram",
        "R's hist() uses Sturges' rule",
        "local({ x <- c(1, 2, 2, 3, 3, 3, 9, 11, 4, 5); \
         identical(hist(x, plot = FALSE)$breaks, pretty(range(x), nclass.Sturges(x))) && \
         identical(eval(formals(graphics::hist.default)$breaks), 'Sturges') })",
    ),
    (
        "lowess",
        "R's lowess() adds three robustness steps",
        "formals(stats::lowess)$iter == 3",
    ),
    (
        "stcox",
        "R's coxph() defaults to Efron's",
        "eval(formals(survival::coxph)$ties)[1] == 'efron'",
    ),
    (
        "summarize",
        "not R's default type 7",
        "formals(stats:::quantile.default)$type == 7 && \
         quantile(1:4, 0.5, type = 2, names = FALSE) == 2.5",
    ),
];

/// The programs a do-file defines (`program [define] name`).
fn programs(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|l| {
            let mut w = l.split_whitespace();
            (w.next()? == "program").then_some(())?;
            let name = match w.next()? {
                "define" | "def" => w.next()?,
                "drop" | "dir" | "list" => return None,
                n => n,
            };
            Some(name.trim_end_matches(',').to_string())
        })
        .collect()
}

/// The command a line runs, past `capture`, `quietly` and `noisily`.
fn command_word(line: &str) -> &str {
    let mut rest = line.trim();
    loop {
        let (word, after) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
        let word = word.trim_end_matches(':');
        if matches!(
            word,
            "capture" | "cap" | "quietly" | "qui" | "noisily" | "noi"
        ) {
            rest = after.trim_start().trim_start_matches(':').trim_start();
        } else {
            return word.trim_end_matches(',');
        }
    }
}

/// Every command in the corpus's do-files, as written: comments, the data
/// lines of `input`, Mata, the `end` of a block and calls to the do-file's
/// own programs left out.
fn corpus_commands() -> Vec<(String, String)> {
    let mut out = Vec::new();
    let Ok(dir) = std::fs::read_dir(corpus()) else {
        return out;
    };
    for entry in dir.flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|e| e != "do") {
            continue;
        }
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let text = std::fs::read_to_string(&path).unwrap_or_default();
        let own = programs(&text);
        let mut continued = false;
        let mut in_block = false;
        for line in text.lines() {
            let t = line.trim();
            let was = continued;
            continued = t.ends_with("///");
            if was
                || t.is_empty()
                || t.starts_with('*')
                || t.starts_with("//")
                || t.starts_with('}')
            {
                continue;
            }
            if in_block {
                in_block = t != "end";
                continue;
            }
            let word = command_word(t);
            if t == "end" || own.iter().any(|p| p == word) {
                continue;
            }
            // what follows is data or Mata, up to `end`
            if matches!(word, "input" | "mata") && !t.contains(':') {
                in_block = true;
            }
            out.push((name.clone(), t.to_string()));
        }
    }
    out
}

/// Stata differences guide: a page for every command the corpus runs, opened
/// from a line; and what the pages say of R holds in R.
#[test]
fn the_differences_guide_has_every_page_and_what_it_says_of_r_holds() {
    let commands = corpus_commands();
    assert!(commands.len() > 1000, "{} lines", commands.len());
    let without: Vec<String> = commands
        .iter()
        .filter(|(_, line)| differences::for_line(line).is_none())
        .map(|(file, line)| format!("{file}: {line}"))
        .collect();
    assert!(
        without.is_empty(),
        "no page for {} lines: {without:#?}",
        without.len()
    );

    let Some((ctx, mut app)) = start() else {
        return;
    };
    // a line's note opens its page
    app.show_stata_guide("quietly regress y x, robust");
    assert!(frames_until(&mut app, &ctx, 10, |a| a
        .help_window
        .shown_stata()
        == Some("regress")));

    let mut wrong = Vec::new();
    for (page, words, check) in R_CLAIMS {
        let said = differences::guide(page).is_some_and(|g| {
            g.points.iter().any(|p| {
                p.text
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .contains(words)
            })
        });
        if !said {
            wrong.push(format!("{page}: the page no longer says {words:?}"));
            continue;
        }
        let (o, out) = r(&mut app, &ctx, &format!("cat(isTRUE({check}))"));
        if o != Outcome::Ok || out.trim() != "TRUE" {
            wrong.push(format!("{page}: {words:?} does not hold in this R: {out}"));
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// Vignettes: Help > Vignettes for grid (it ships with R) lists what R's own
/// vignette index has, and a vignette's code opens as its file holds it.
#[test]
fn vignettes_are_listed_and_opened_as_r_has_them() {
    let Some((ctx, mut app)) = start() else {
        return;
    };
    app.perform(
        &ctx,
        Action::HelpRequest(HelpCommand::Vignettes {
            package: Some("grid".into()),
        }),
    );
    assert!(frames_until(&mut app, &ctx, 60, |a| a
        .help_window
        .shown_vignettes()
        .is_some()));
    let list = app
        .help_window
        .shown_vignettes()
        .map(<[_]>::to_vec)
        .unwrap_or_default();
    let (o, index) = r(
        &mut app,
        &ctx,
        "local({ v <- tools::getVignetteInfo('grid'); \
         cat(paste(v[, 'Topic'], v[, 'PDF'], v[, 'R'], v[, 'Title'], sep = '\\t'), sep = '\\n') })",
    );
    assert_eq!(o, Outcome::Ok, "{index}");
    let rows: Vec<Vec<&str>> = index.lines().map(|l| l.split('\t').collect()).collect();
    assert!(rows.len() >= 10, "{index}");
    let mut items: Vec<&str> = list.iter().map(|v| v.item.as_str()).collect();
    let mut topics: Vec<&str> = rows.iter().filter_map(|r| r.first().copied()).collect();
    items.sort_unstable();
    topics.sort_unstable();
    assert_eq!(items, topics);
    for row in &rows {
        let [topic, pdf, code, title] = row.as_slice() else {
            panic!("{row:?}");
        };
        let v = list.iter().find(|v| v.item == *topic);
        assert_eq!(v.map(|v| v.title.as_str()), Some(*title), "{topic}");
        let files: Vec<String> = v
            .map(|v| {
                v.files
                    .iter()
                    .map(|f| f.rsplit(['/', '\\']).next().unwrap_or("").to_string())
                    .collect()
            })
            .unwrap_or_default();
        for f in [pdf, code].into_iter().filter(|f| !f.is_empty()) {
            assert!(
                files.iter().any(|g| g == f),
                "{topic}: {f} not in {files:?}"
            );
        }
        assert!(v.is_some_and(|v| v.package == "grid"));
    }
    // a vignette's R code opens as the file has it
    let code_file = list
        .iter()
        .flat_map(|v| v.files.iter())
        .find(|f| kind_of(f) == Some(FileKind::Code))
        .cloned()
        .unwrap_or_default();
    assert!(!code_file.is_empty());
    app.perform(
        &ctx,
        Action::HelpRequest(HelpCommand::File {
            path: code_file.clone(),
        }),
    );
    assert!(frames_until(&mut app, &ctx, 60, |a| a
        .help_window
        .shown_text()
        .is_some()));
    let shown = app.help_window.shown_text().unwrap_or("").to_string();
    let on_disk = std::fs::read_to_string(&code_file).unwrap_or_default();
    let norm = |s: &str| {
        s.lines()
            .map(str::trim_end)
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string()
    };
    assert_eq!(norm(&shown), norm(&on_disk));
}
