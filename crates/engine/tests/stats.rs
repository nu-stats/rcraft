//! Statistics menu descriptives (P4.1) end to end against a real R (skipped
//! with a note when none is found): build each dialog's code from its spec and
//! the table's columns, run it, and check the numbers R prints or returns.

mod pinned;

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
    let mut e = Engine::start(SessionConfig::new(install)).ok()?;
    pinned::record(&mut e, PACKAGES);
    Some(e)
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
/// The titles of the tables the last command gave (a grid each in Results).
fn last_tables(e: &Engine) -> Vec<String> {
    e.results()
        .blocks()
        .last()
        .map(|b| {
            b.items
                .iter()
                .filter_map(|i| match i {
                    Item::Text { table: Some(t), .. } => Some(t.title.clone()),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default()
}

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
        frames: Vec::new(),
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
    assert_eq!(last_tables(&e), ["g", "age"], "a table for each variable");
    let (_, plain) = dialog(&mut e, &FREQUENCY, &[], &[("vars", &["g"])], &[]);
    assert_eq!(last_tables(&e), ["g"]);
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
    // the test is named as R names it (a 2 by 2 table has Yates' correction)
    let tables = last_tables(&e);
    assert_eq!(tables.len(), 3, "{tables:?}");
    assert_eq!(tables[..2], ["Counts", "Row percentages"]);
    assert!(
        tables[2].starts_with("Pearson's Chi-squared test"),
        "{tables:?}"
    );
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
    assert_eq!(last_tables(&e), ["Correlations", "Rows used by each pair"]);
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

/// Base R's stats only.
const PACKAGES: &[&str] = &[];

/// Runs the dialog's code as the menu does and keeps the tables it gives as
/// `res`.
fn kept(
    e: &mut Engine,
    spec: &Spec,
    set: &[(&str, &str)],
    lists: &[(&str, &[&str])],
    flags: &[&str],
) {
    let (made, _) = run(e, Source::Typed, MAKE);
    assert_eq!(made, Outcome::Ok);
    let ctx = Context {
        dataset: "d".into(),
        frames: Vec::new(),
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
    for w in spec.widgets {
        if let Widget::Check { id, .. } = w {
            values.set_flag(id, flags.contains(id));
        }
    }
    let code = spec
        .generate(&ctx, &values, Style::Base)
        .unwrap_or_default();
    let (outcome, text) = run(
        e,
        Source::Menu(spec.menu.into()),
        &pinned::keep_value(&code, "res"),
    );
    assert_eq!(outcome, Outcome::Ok, "{code}\n{text}");
}

/// The frequency and cross-tabulation counts, shares and chi-square, and the
/// correlations with their p-values, against counts and formulas written out
/// in plain R (no table, chisq.test or cor): rows counted one by one, the
/// chi-square with Yates' correction from the expected counts, Pearson's r
/// from the sums of products and its t, and Kendall's tau-b from the
/// concordant and discordant pairs.
#[test]
fn counts_and_correlations_are_the_textbook_formulas() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);

    kept(
        &mut e,
        &FREQUENCY,
        &[],
        &[("vars", &["g", "age"])],
        &["percent", "missing"],
    );
    holds(
        &mut e,
        &pinned::all_of(
            "ga <- sum(d$g == 'a'); gb <- sum(d$g == 'b'); na <- sum(is.na(d$age))",
            &[
                ("g", "all(res$g[, 'count'] == c(ga, gb))"),
                (
                    "g_pct",
                    "all(res$g[, 'percent'] == c(100 * ga / 5, 100 * gb / 5))",
                ),
                ("age_n", "sum(res$age[, 'count']) == 5"),
                ("age_na", "tail(res$age[, 'count'], 1) == na"),
                ("age_pct", "all(res$age[, 'percent'] == 20)"),
            ],
        ),
    );

    kept(
        &mut e,
        &CROSSTAB,
        &[("row", "g"), ("col", "grp"), ("share", "row")],
        &[],
        &["test"],
    );
    holds(
        &mut e,
        &pinned::all_of(
            "O <- matrix(0, 2, 2, dimnames = list(c('a', 'b'), c('f', 'm'))); \
             for (i in seq_len(nrow(d))) O[as.character(d$g[i]), d$grp[i]] <- \
             O[as.character(d$g[i]), d$grp[i]] + 1; \
             E <- outer(rowSums(O), colSums(O)) / sum(O); \
             x2 <- sum((abs(O - E) - min(0.5, abs(O - E)))^2 / E); \
             test <- res[[3]]",
            &[
                ("counts", "all(res$Counts[1:2, 1:2] == O)"),
                ("totals", "res$Counts[3, 3] == 5"),
                (
                    "row_pct",
                    "all(res[['Row percentages']] == round(100 * O / rowSums(O), 1))",
                ),
                (
                    "x2",
                    "all.equal(test[['X-squared']], x2, tolerance = 1e-12)",
                ),
                (
                    "p",
                    "all.equal(test[['p-value']], pchisq(x2, 1, lower.tail = FALSE), tolerance = 1e-12)",
                ),
            ],
        ),
    );

    kept(&mut e, &CORRELATION, &[], &[("vars", &["x", "y"])], &["p"]);
    holds(
        &mut e,
        &pinned::all_of(
            "x <- d$x; y <- d$y; n <- 5; \
             r <- sum((x - mean(x)) * (y - mean(y))) / \
             sqrt(sum((x - mean(x))^2) * sum((y - mean(y))^2)); \
             t <- r * sqrt(n - 2) / sqrt(1 - r^2)",
            &[
                ("r", "res$Correlations['x', 'y'] == round(r, 3)"),
                ("rows", "all(res[['Rows used by each pair']] == 5)"),
                (
                    "p",
                    "res[['P-values']]['x', 'y'] == round(2 * pt(-abs(t), n - 2), 4)",
                ),
            ],
        ),
    );

    kept(
        &mut e,
        &CORRELATION,
        &[("method", "kendall")],
        &[("vars", &["x", "y"])],
        &[],
    );
    holds(
        &mut e,
        &pinned::all_of(
            "x <- d$x; y <- d$y; s <- 0; tx <- 0; ty <- 0; n0 <- 0; \
             for (i in 1:4) for (j in (i + 1):5) { n0 <- n0 + 1; \
               dx <- sign(x[j] - x[i]); dy <- sign(y[j] - y[i]); s <- s + dx * dy; \
               tx <- tx + (dx == 0); ty <- ty + (dy == 0) }; \
             tau <- s / sqrt((n0 - tx) * (n0 - ty))",
            &[("tau", "res$Correlations['x', 'y'] == round(tau, 3)")],
        ),
    );
}

/// Normality test: W and its p-value against Royston's algorithm written out
/// in plain R, for 20 values (the large-n approximation), the 11 of them that
/// are not missing (the small-n one) and 5 (the shortest list of
/// coefficients).
#[test]
fn the_normality_test_is_roystons_algorithm_by_hand() {
    let shapiro = pinned::SHAPIRO_BY_HAND;
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    let (made, text) = run(
        &mut e,
        Source::Typed,
        "w <- data.frame(big = c(2.3, 3.1, 1.8, 4.9, 2.7, 3.3, 2.9, 6.2, 3.8, 2.2, 3.5, 4.1, 2.6, \
         3.0, 5.4, 3.7, 2.8, 3.2, 4.4, 1.9), \
         some = c(2.3, NA, 1.8, 4.9, NA, 3.3, 2.9, NA, 3.8, 2.2, NA, 4.1, NA, 3.0, NA, 3.7, NA, NA, \
         4.4, NA), five = c(1, 2, 4, 7, 11, rep(NA, 15)))",
    );
    assert_eq!(made, Outcome::Ok, "{text}");
    let ctx = Context {
        dataset: "w".into(),
        frames: Vec::new(),
        columns: ["big", "some", "five"]
            .into_iter()
            .map(|name| ColumnInfo {
                name: name.into(),
                kind: VarKind::Number,
            })
            .collect(),
    };
    let mut values = NORMALITY.defaults(&ctx);
    values.set_list("vars", vec!["big".into(), "some".into(), "five".into()]);
    let code = NORMALITY
        .generate(&ctx, &values, Style::Base)
        .unwrap_or_default();
    let (outcome, text) = run(
        &mut e,
        Source::Menu(NORMALITY.menu.into()),
        &pinned::keep_value(&code, "res"),
    );
    assert_eq!(outcome, Outcome::Ok, "{code}\n{text}");
    holds(
        &mut e,
        &pinned::all_of(
            &format!(
                "{shapiro}; want <- rbind(big = sw(w$big), some = sw(na.omit(w$some)), \
                 five = sw(na.omit(w$five)))"
            ),
            &[
                ("n", "all(res[, 'n'] == c(20, 11, 5))"),
                ("w", "all.equal(unname(res[, 'W']), unname(want[, 'W']), tolerance = 1e-12)"),
                (
                    "p",
                    "all.equal(unname(res[, 'p.value']), unname(want[, 'p.value']), tolerance = 1e-12)",
                ),
            ],
        ),
    );
}
