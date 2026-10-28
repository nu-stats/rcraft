//! Statistics menu descriptives (P4.1) end to end against a real R (skipped
//! with a note when none is found): build each dialog's code from its spec and
//! the table's columns, run it, and check the numbers R prints or returns.

use std::time::{Duration, Instant};

use rcraft_catalog::stats::{CORRELATION, CROSSTAB, FREQUENCY, MISSING, NORMALITY, SUMMARY};
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

fn run(e: &mut Engine, source: Source, code: &str) -> (Outcome, String) {
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
                    Item::Warning(w) => format!("warning: {w}"),
                    Item::Error { message, .. } => format!("error: {message}"),
                    Item::Notice(n) => n.clone(),
                    Item::Plot { .. } => String::new(),
                })
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default();
    (outcome, text)
}

const MAKE: &str = "d <- data.frame(age = c(20, 30, 40, NA, 50), \
     g = factor(c('a', 'b', 'a', 'b', 'a')), x = c(1, 2, 3, 4, 5), y = c(2, 4, 5, 4, 5), \
     grp = c('m', 'f', 'm', 'f', 'm'), z = c(NA, 1, NA, 2, NA), flat = c(1, 1, 1, 1, 1))";

/// Makes `d`, builds the spec's code for it with these values and runs it.
fn dialog(
    e: &mut Engine,
    spec: &Spec,
    set: &[(&str, &str)],
    lists: &[(&str, &[&str])],
    flags: &[&str],
) -> (Outcome, String) {
    let (made, _) = run(e, Source::Typed, MAKE);
    assert_eq!(made, Outcome::Ok);
    let ctx = Context {
        dataset: "d".into(),
        columns: [
            ("age", VarKind::Number),
            ("g", VarKind::Factor),
            ("x", VarKind::Number),
            ("y", VarKind::Number),
            ("grp", VarKind::Text),
            ("z", VarKind::Number),
            ("flat", VarKind::Number),
        ]
        .into_iter()
        .map(|(name, kind)| ColumnInfo {
            name: name.into(),
            kind,
        })
        .collect(),
    };
    let mut values = spec.defaults(&ctx);
    for (id, text) in set {
        values.set_text(id, *text);
    }
    for (id, names) in lists {
        values.set_list(id, names.iter().map(|n| n.to_string()).collect());
    }
    // exactly the named boxes are ticked, whatever the dialog starts with
    for w in spec.widgets {
        if let Widget::Check { id, .. } = w {
            values.set_flag(id, flags.contains(id));
        }
    }
    let code = spec.generate(&ctx, &values, Style::Base);
    assert!(code.is_ok(), "{code:?}");
    let code = code.unwrap_or_default();
    let (outcome, text) = run(e, Source::Menu(spec.menu.into()), &code);
    assert_eq!(outcome, Outcome::Ok, "{code}\n{text}");
    (outcome, text)
}

/// Runs R that stops unless every one of `exprs` holds.
fn holds(e: &mut Engine, exprs: &str) {
    let (outcome, text) = run(e, Source::Typed, &format!("stopifnot({exprs})"));
    assert_eq!(outcome, Outcome::Ok, "{exprs}\n{text}");
}

#[test]
fn summary_statistics_are_right_and_can_be_saved_and_split() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);

    // saved, so the numbers can be checked against R's own functions
    dialog(
        &mut e,
        &SUMMARY,
        &[("save", "tab")],
        &[("vars", &["age", "x"])],
        &[],
    );
    holds(
        &mut e,
        "identical(rownames(tab), c('age', 'x')), tab['age', 'n'] == 4, tab['age', 'missing'] == 1, \
         tab['age', 'mean'] == 35, isTRUE(all.equal(tab['age', 'sd'], sd(c(20, 30, 40, 50)))), \
         tab['age', 'median'] == 35, tab['x', 'max'] == 5, tab['x', 'q25'] == 2",
    );

    // none chosen: every number variable, and a constant one is fine
    let (_, text) = dialog(&mut e, &SUMMARY, &[("save", "all")], &[], &[]);
    holds(
        &mut e,
        "identical(rownames(all), c('age', 'x', 'y', 'z', 'flat')), all['flat', 'sd'] == 0, \
         all['z', 'n'] == 2",
    );
    assert!(text.contains("median"), "{text}");

    // split by a group: one table each, rows with a missing group left out
    let (_, text) = dialog(&mut e, &SUMMARY, &[("by", "g")], &[("vars", &["x"])], &[]);
    assert!(text.contains("$a") && text.contains("$b"), "{text}");
    dialog(
        &mut e,
        &SUMMARY,
        &[("by", "g"), ("save", "bytab")],
        &[("vars", &["x"])],
        &[],
    );
    holds(
        &mut e,
        "identical(names(bytab), c('a', 'b')), bytab$a['x', 'mean'] == 3, bytab$b['x', 'n'] == 2",
    );
}

#[test]
fn frequency_and_cross_tabulation_count_the_rows() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);

    let (_, text) = dialog(
        &mut e,
        &FREQUENCY,
        &[],
        &[("vars", &["g", "age"])],
        &["percent", "missing"],
    );
    assert!(text.contains("count") && text.contains("percent"), "{text}");
    assert!(text.contains("60") && text.contains("40"), "{text}");
    // the missing age is its own row
    assert!(text.contains("<NA>"), "{text}");
    let (_, plain) = dialog(&mut e, &FREQUENCY, &[], &[("vars", &["g"])], &[]);
    assert!(
        !plain.contains("percent") && !plain.contains("<NA>"),
        "{plain}"
    );

    let (_, text) = dialog(
        &mut e,
        &CROSSTAB,
        &[("row", "g"), ("col", "grp"), ("share", "row")],
        &[],
        &["test"],
    );
    assert!(text.contains("Sum"), "totals are added: {text}");
    assert!(text.contains("Row percentages"), "{text}");
    assert!(text.contains("Pearson's Chi-squared test"), "{text}");
    for share in ["column", "total", "none"] {
        let (_, text) = dialog(
            &mut e,
            &CROSSTAB,
            &[("row", "g"), ("col", "grp"), ("share", share)],
            &[],
            &[],
        );
        assert_eq!(
            text.to_lowercase().contains("percentages"),
            share != "none",
            "{share}: {text}"
        );
    }
}

#[test]
fn correlation_missing_patterns_and_normality_give_what_r_gives() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);

    let (_, text) = dialog(&mut e, &CORRELATION, &[], &[("vars", &["x", "y"])], &[]);
    assert!(text.contains("0.775"), "cor(x, y) is 0.7746: {text}");
    assert!(text.contains("Rows used by each pair"), "{text}");
    let (_, text) = dialog(
        &mut e,
        &CORRELATION,
        &[("method", "spearman"), ("use", "complete")],
        &[("vars", &["x", "age"])],
        &["p"],
    );
    assert!(text.contains("P-values"), "{text}");
    // complete rows only: the row with a missing age is out for both variables
    assert!(text.contains(" 4"), "{text}");
    dialog(
        &mut e,
        &CORRELATION,
        &[("method", "kendall")],
        &[("vars", &["x", "y"])],
        &["p"],
    );

    let (_, text) = dialog(&mut e, &MISSING, &[], &[("vars", &["age", "z"])], &[]);
    assert!(text.contains("Patterns"), "{text}");
    // rows: z missing in 3 (".X"), none missing in 1 (".."), age missing in 1 ("X.")
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    assert!(
        lines
            .iter()
            .any(|l| l.starts_with(".X") && l.ends_with('3')),
        "{text}"
    );
    assert!(
        lines
            .iter()
            .any(|l| l.starts_with("X.") && l.ends_with('1')),
        "{text}"
    );
    // none chosen: every variable
    dialog(&mut e, &MISSING, &[], &[], &[]);

    let (_, text) = dialog(&mut e, &NORMALITY, &[], &[("vars", &["x", "flat"])], &[]);
    assert!(text.contains("W") && text.contains("p.value"), "{text}");
    // a constant variable cannot be tested: NA, not an error
    let (_, text) = dialog(&mut e, &NORMALITY, &[], &[("vars", &["flat"])], &[]);
    assert!(text.contains("NA"), "{text}");
}
