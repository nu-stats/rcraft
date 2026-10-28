//! Graphs menu dialogs (P5.3) end to end against a real R (skipped with a note
//! when none is found): each dialog's code is built from its spec, run, and must
//! draw exactly the plots it should, as real PNG files, without an error.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use rcraft_catalog::graphs::{
    BAR_CHART, BOX_PLOT, DENSITY, HISTOGRAM, LINE_PLOT, QQ_PLOT, SCATTER, SCATTER_MATRIX,
};
use rcraft_engine::{Engine, Item, Outcome, Source};
use rcraft_rcodegen::dialog::{ColumnInfo, Context, Spec, Style, VarKind, Widget};
use rcraft_rhost::{find_installs, Env, SessionConfig};

fn engine() -> Option<Engine> {
    let install = find_installs(&Env::from_process())
        .into_iter()
        .find(|i| i.is_supported());
    let Some(install) = install else {
        eprintln!("skipped: no supported R on this machine");
        return None;
    };
    Engine::start(SessionConfig::new(install)).ok()
}

fn settle(e: &mut Engine) {
    let limit = Instant::now() + Duration::from_secs(60);
    loop {
        e.poll(Duration::from_millis(50));
        if !e.is_working() {
            break;
        }
        assert!(Instant::now() < limit, "the engine did not settle");
    }
}

/// Runs code; returns how it ended, the plots it drew and its messages.
fn run(e: &mut Engine, source: Source, code: &str) -> (Outcome, Vec<PathBuf>, String) {
    let id = e.submit(source, code);
    assert!(id.is_ok(), "{id:?}");
    let id = id.unwrap_or_default();
    settle(e);
    let outcome = e
        .history()
        .entries()
        .last()
        .and_then(|h| h.outcome)
        .unwrap_or(Outcome::Error);
    let mut plots = Vec::new();
    let mut text = String::new();
    for block in e.results().blocks().iter().filter(|b| b.command == id) {
        for item in &block.items {
            match item {
                Item::Plot { path } => plots.push(path.clone()),
                Item::Text { text: t, .. } => text.push_str(t),
                Item::Warning(w) => text.push_str(&format!("warning: {w}\n")),
                Item::Error { message, .. } => text.push_str(&format!("error: {message}\n")),
                Item::Notice(n) => text.push_str(n),
            }
        }
    }
    (outcome, plots, text)
}

const MAKE: &str = "d <- data.frame(\
     x = c(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12), \
     y = c(2.1, 3.9, 6.2, 7.8, 10.1, 12.2, 13.8, 16.1, 18.0, 20.2, 21.9, 24.1), \
     z = c(5, 3, 8, 1, 9, 4, 7, 2, 6, 10, 3, 5), \
     g = factor(rep(c('a', 'b', 'c'), 4)), \
     t = c('u', 'v', 'u', 'v', 'u', 'v', 'u', 'v', 'u', 'v', 'u', NA), \
     day = as.Date('2026-01-01') + 0:11)";

fn context() -> Context {
    Context {
        dataset: "d".into(),
        frames: Vec::new(),
        columns: [
            ("x", VarKind::Number),
            ("y", VarKind::Number),
            ("z", VarKind::Number),
            ("g", VarKind::Factor),
            ("t", VarKind::Text),
            ("day", VarKind::Date),
        ]
        .into_iter()
        .map(|(name, kind)| ColumnInfo {
            name: name.into(),
            kind,
        })
        .collect(),
    }
}

/// Builds the dialog's code with these values and runs it on made-up data.
fn draw(
    e: &mut Engine,
    spec: &Spec,
    set: &[(&str, &str)],
    lists: &[(&str, &[&str])],
    flags: &[&str],
) -> (Outcome, Vec<PathBuf>, String) {
    let (made, _, _) = run(e, Source::Typed, MAKE);
    assert_eq!(made, Outcome::Ok);
    let ctx = context();
    let mut values = spec.defaults(&ctx);
    for (id, text) in set {
        values.set_text(id, *text);
    }
    for (id, names) in lists {
        values.set_list(id, names.iter().map(|n| n.to_string()).collect());
    }
    for w in spec.widgets {
        if let Widget::Check { id, .. } = w {
            values.set_flag(id, flags.contains(id));
        }
    }
    let code = spec.generate(&ctx, &values, Style::Base);
    assert!(code.is_ok(), "{code:?}");
    let code = code.unwrap_or_default();
    let (outcome, plots, text) = run(e, Source::Menu(spec.menu.into()), &code);
    assert_eq!(outcome, Outcome::Ok, "{code}\n{text}");
    assert!(!text.contains("error:"), "{code}\n{text}");
    (outcome, plots, text)
}

fn is_png(path: &std::path::Path) -> bool {
    std::fs::read(path).is_ok_and(|b| {
        b.starts_with(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]) && b.len() > 2000
    })
}

/// One dialog draws exactly one plot, a real picture.
fn one_plot(
    e: &mut Engine,
    spec: &Spec,
    set: &[(&str, &str)],
    lists: &[(&str, &[&str])],
    flags: &[&str],
) {
    let (_, plots, text) = draw(e, spec, set, lists, flags);
    assert_eq!(plots.len(), 1, "{}: {plots:?}\n{text}", spec.entry);
    assert!(
        plots.first().is_some_and(|p| is_png(p)),
        "{}: {plots:?}",
        spec.entry
    );
}

#[test]
fn every_graph_dialog_draws_one_real_picture() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);

    one_plot(&mut e, &HISTOGRAM, &[("x", "y")], &[], &[]);
    one_plot(
        &mut e,
        &HISTOGRAM,
        &[("x", "y"), ("breaks", "FD"), ("title", "My bars")],
        &[],
        &["density"],
    );
    one_plot(&mut e, &HISTOGRAM, &[("x", "y"), ("bins", "4")], &[], &[]);
    one_plot(&mut e, &DENSITY, &[("x", "y")], &[], &[]);
    one_plot(&mut e, &DENSITY, &[("x", "y"), ("g", "g")], &[], &[]);
    one_plot(&mut e, &BOX_PLOT, &[("x", "y")], &[], &[]);
    one_plot(&mut e, &BOX_PLOT, &[("x", "y"), ("g", "g")], &[], &[]);
    one_plot(&mut e, &BAR_CHART, &[("x", "g")], &[], &[]);
    one_plot(&mut e, &BAR_CHART, &[("x", "t")], &[], &["percent"]);
    one_plot(&mut e, &QQ_PLOT, &[("x", "y")], &[], &[]);
    one_plot(&mut e, &SCATTER, &[("x", "x"), ("y", "y")], &[], &[]);
    one_plot(
        &mut e,
        &SCATTER,
        &[("x", "x"), ("y", "z"), ("g", "g")],
        &[],
        &["fit", "smooth"],
    );
    one_plot(&mut e, &LINE_PLOT, &[("x", "day"), ("y", "y")], &[], &[]);
    one_plot(
        &mut e,
        &LINE_PLOT,
        &[("x", "x"), ("y", "z")],
        &[],
        &["points"],
    );
    one_plot(
        &mut e,
        &SCATTER_MATRIX,
        &[],
        &[("vars", &["x", "y", "z"])],
        &[],
    );
}

#[test]
fn a_plot_dialog_run_again_draws_another_and_a_typed_command_can_add_to_it() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    let (_, first, _) = draw(&mut e, &SCATTER, &[("x", "x"), ("y", "y")], &[], &[]);
    assert_eq!(first.len(), 1);
    // abline() in a typed command finds the scatter plot: the device stayed open
    let (outcome, second, text) = run(
        &mut e,
        Source::Typed,
        "abline(lm(y ~ x, data = d), col = 'red')",
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert_eq!(second.len(), 1, "{second:?}");
    assert_ne!(
        std::fs::read(&first[0]).unwrap_or_default(),
        std::fs::read(&second[0]).unwrap_or_default()
    );
}

#[test]
fn missing_values_do_not_stop_a_plot() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    // t has a missing value; a bar chart and a scatter with a missing y still draw
    one_plot(&mut e, &BAR_CHART, &[("x", "t")], &[], &[]);
    let (made, _, _) = run(&mut e, Source::Typed, "d$y[3] <- NA; d$x[5] <- NA; 0");
    assert_eq!(made, Outcome::Ok);
    let ctx = context();
    let mut values = SCATTER.defaults(&ctx);
    values.set_text("x", "x");
    values.set_text("y", "y");
    values.set_flag("fit", true);
    values.set_flag("smooth", true);
    let code = SCATTER
        .generate(&ctx, &values, Style::Base)
        .unwrap_or_default();
    let (outcome, plots, text) = run(&mut e, Source::Menu(SCATTER.menu.into()), &code);
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert_eq!(plots.len(), 1);
}
