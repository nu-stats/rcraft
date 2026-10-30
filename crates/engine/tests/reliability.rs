//! Fleiss' kappa and the intraclass correlations (P4.2) against a real R
//! (skipped with a note when none is found), on published examples:
//!
//! * Fleiss' kappa: the 10 subjects, 14 raters and 5 categories of Fleiss
//!   (1971), whose kappa is .210;
//! * intraclass correlations: Shrout and Fleiss (1979), 6 targets rated by 4
//!   judges: mean squares 11.24, 32.49, 1.02, 6.26; ICC(1,1) .17, ICC(2,1) .29,
//!   ICC(3,1) .71, ICC(1,k) .44, ICC(2,k) .62, ICC(3,k) .91; and the intervals
//!   the `psych` package prints for them.
//!
//! The Fleiss numbers are also worked out in R from the table of counts by a
//! separate route, so the dialog's arithmetic is checked, not only its
//! agreement with a published figure.

use std::time::{Duration, Instant};

use rcraft_catalog::inference::{FLEISS, ICC};
use rcraft_engine::{Engine, Item, Outcome, Source};
use rcraft_rcodegen::dialog::{ColumnInfo, Context, Spec, Style, VarKind};
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

fn run(e: &mut Engine, code: &str) -> (Outcome, String) {
    let id = e.submit(Source::Typed, code);
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
                    Item::Warning(w) => format!("warning: {w}\n"),
                    Item::Error { message, .. } => format!("error: {message}\n"),
                    Item::Notice(n) => n.clone(),
                    Item::Plot { .. } => String::new(),
                })
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default();
    (outcome, text)
}

fn ok(e: &mut Engine, code: &str) -> String {
    let (outcome, text) = run(e, code);
    assert_eq!(outcome, Outcome::Ok, "{code}\n{text}");
    text
}

/// The code `spec` writes for the data frame `d` with these columns.
fn dialog_code(spec: &Spec, columns: &[&str], kind: VarKind, set: &[(&str, &str)]) -> String {
    let ctx = Context {
        dataset: "d".into(),
        frames: Vec::new(),
        columns: columns
            .iter()
            .map(|name| ColumnInfo {
                name: (*name).into(),
                kind,
            })
            .collect(),
    };
    let mut values = spec.defaults(&ctx);
    values.set_list("raters", columns.iter().map(|c| (*c).to_string()).collect());
    for (id, v) in set {
        values.set_text(id, *v);
    }
    let code = spec.generate(&ctx, &values, Style::Base);
    assert!(code.is_ok(), "{code:?}");
    code.unwrap_or_default()
}

/// The value printed on the line that starts with `what`.
fn value(text: &str, what: &str) -> String {
    text.lines()
        .find_map(|l| l.strip_prefix(what))
        .map(|rest| rest.trim().to_string())
        .unwrap_or_default()
}

/// Fleiss (1971): rows are subjects, columns the 5 categories, 14 raters each.
const FLEISS_COUNTS: &str = "matrix(c(\
    0, 0, 0, 0, 14,  0, 2, 6, 4, 2,  0, 0, 3, 5, 6,  0, 3, 9, 2, 0,  2, 2, 8, 1, 1, \
    7, 7, 0, 0, 0,  3, 2, 6, 3, 0,  2, 5, 3, 2, 2,  6, 5, 2, 1, 0,  0, 2, 2, 3, 7), \
    ncol = 5, byrow = TRUE)";

#[test]
fn fleiss_kappa_matches_the_published_example_and_a_separate_calculation() {
    let Some(mut e) = engine() else { return };
    // one column per rater: each subject's 14 ratings, in any order
    ok(
        &mut e,
        &format!(
            "counts <- {FLEISS_COUNTS}; \
             d <- as.data.frame(t(apply(counts, 1, function(row) rep(1:5, row)))); \
             names(d) <- paste0('r', 1:14)"
        ),
    );
    let names: Vec<String> = (1..=14).map(|i| format!("r{i}")).collect();
    let cols: Vec<&str> = names.iter().map(String::as_str).collect();
    let code = dialog_code(&FLEISS, &cols, VarKind::Number, &[]);
    let text = ok(&mut e, &code);
    assert!(text.contains("14 raters on 10 rows"), "{text}");
    // the published kappa, to three places
    assert_eq!(
        value(&text, "Fleiss' kappa")
            .chars()
            .take(5)
            .collect::<String>(),
        "0.209",
        "{text}"
    );
    assert_eq!(value(&text, "Observed agreement"), "0.3780", "{text}");
    assert_eq!(value(&text, "Agreement expected by chance"), "0.2128");
    // separate route from the counts: kappa, z and the category kappas
    let check = ok(
        &mut e,
        "N <- nrow(counts); m <- sum(counts[1, ]); pj <- colSums(counts) / (N * m); \
         Pi <- apply(counts, 1, function(r) (sum(r^2) - m) / (m * (m - 1))); \
         kap <- (mean(Pi) - sum(pj^2)) / (1 - sum(pj^2)); \
         q <- 1 - pj; \
         v0 <- 2 / (N * m * (m - 1)) * (sum(pj * q)^2 - sum(pj * q * (q - pj))) / sum(pj * q)^2; \
         cat(formatC(kap, digits = 4, format = 'f'), formatC(kap / sqrt(v0), digits = 4, format = 'f'))",
    );
    let mut it = check.split_whitespace();
    assert_eq!(value(&text, "Fleiss' kappa"), it.next().unwrap_or_default());
    assert_eq!(value(&text, "z (kappa = 0)"), it.next().unwrap_or_default());
    // the per-category kappas are in the table
    assert!(text.contains("Category"), "{text}");
}

#[test]
fn fleiss_kappa_needs_complete_rows_and_two_categories() {
    let Some(mut e) = engine() else { return };
    ok(
        &mut e,
        "d <- data.frame(r1 = c('a', 'b', 'a', NA), r2 = c('a', 'b', 'b', 'a'), r3 = c('a', 'b', 'a', 'b'))",
    );
    let code = dialog_code(&FLEISS, &["r1", "r2", "r3"], VarKind::Text, &[]);
    let text = ok(&mut e, &code);
    assert!(text.contains("3 raters on 3 rows"), "{text}");
    ok(
        &mut e,
        "d <- data.frame(r1 = rep('a', 4), r2 = rep('a', 4))",
    );
    let code = dialog_code(&FLEISS, &["r1", "r2"], VarKind::Text, &[]);
    let (outcome, text) = run(&mut e, &code);
    assert_eq!(outcome, Outcome::Error);
    assert!(
        text.contains("every rating falls in one category"),
        "{text}"
    );
}

const SHROUT_FLEISS: &str = "d <- data.frame(\
    J1 = c(9, 6, 8, 7, 10, 6), J2 = c(2, 1, 4, 1, 5, 2), \
    J3 = c(5, 3, 6, 2, 6, 4), J4 = c(8, 2, 8, 6, 9, 7))";

#[test]
fn intraclass_correlations_match_shrout_and_fleiss() {
    let Some(mut e) = engine() else { return };
    ok(&mut e, SHROUT_FLEISS);
    let code = dialog_code(&ICC, &["J1", "J2", "J3", "J4"], VarKind::Number, &[]);
    let text = ok(&mut e, &code);
    assert!(text.contains("4 raters on 6 rows"), "{text}");
    // the mean squares of the paper
    assert!(
        text.contains("subjects 11.24, raters 32.49, residual 1.019, within subjects 6.264"),
        "{text}"
    );
    // each ICC line: type, then the value rounded to two places as in the paper
    let row = |kind: &str| -> Vec<String> {
        text.lines()
            .find(|l| l.contains(kind))
            .map(|l| l.split_whitespace().map(str::to_string).collect())
            .unwrap_or_default()
    };
    let icc = |kind: &str, at: usize| -> String {
        let cells = row(kind);
        // the type, then ICC, F, df1, df2, p, lower, upper
        cells.get(1 + at).cloned().unwrap_or_default()
    };
    for (kind, want) in [
        ("ICC(1,1)", 0.17),
        ("ICC(2,1)", 0.29),
        ("ICC(3,1)", 0.71),
        ("ICC(1,k)", 0.44),
        ("ICC(2,k)", 0.62),
        ("ICC(3,k)", 0.91),
    ] {
        // the paper gives two places, so the value must round to them
        let got: f64 = icc(kind, 0).parse().unwrap_or(f64::NAN);
        assert!(
            (got - want).abs() < 0.0051,
            "{kind}: {got} against {want}\n{text}"
        );
    }
    // the F tests: one-way 1.79 on 5 and 18; the two-way ones 11.03 on 5 and 15
    assert_eq!(icc("ICC(1,1)", 1), "1.79", "{text}");
    assert_eq!(icc("ICC(1,1)", 3), "18");
    assert_eq!(icc("ICC(3,1)", 1), "11.03");
    assert_eq!(icc("ICC(3,1)", 3), "15");
}

#[test]
fn intraclass_intervals_match_the_psych_package_and_are_refused_when_too_small() {
    let Some(mut e) = engine() else { return };
    ok(&mut e, SHROUT_FLEISS);
    let code = dialog_code(&ICC, &["J1", "J2", "J3", "J4"], VarKind::Number, &[]);
    ok(&mut e, &format!("icc <- {code}"));
    // the intervals psych::ICC prints for these data (95%), to two places
    for (row, lower, upper) in [
        (1, -0.13, 0.72),
        (2, 0.02, 0.76),
        (3, 0.34, 0.95),
        (4, -0.88, 0.91),
        (5, 0.07, 0.93),
        (6, 0.68, 0.99),
    ] {
        let got = ok(
            &mut e,
            &format!("cat(round(icc[{row}, 7], 2), round(icc[{row}, 8], 2))"),
        );
        let mut it = got.split_whitespace();
        assert_eq!(
            it.next().and_then(|s| s.parse::<f64>().ok()),
            Some(lower),
            "row {row} lower: {got}"
        );
        assert_eq!(
            it.next().and_then(|s| s.parse::<f64>().ok()),
            Some(upper),
            "row {row} upper: {got}"
        );
    }
    // a different level widens or narrows them
    let code90 = dialog_code(
        &ICC,
        &["J1", "J2", "J3", "J4"],
        VarKind::Number,
        &[("conf", "0.9")],
    );
    let text = ok(&mut e, &code90);
    assert!(text.contains("lower 90%"), "{text}");
    // too few rows
    ok(&mut e, "d <- data.frame(a = c(1, 2), b = c(2, 3))");
    let code = dialog_code(&ICC, &["a", "b"], VarKind::Number, &[]);
    let (outcome, text) = run(&mut e, &code);
    assert_eq!(outcome, Outcome::Error);
    assert!(text.contains("At least three rows"), "{text}");
}
