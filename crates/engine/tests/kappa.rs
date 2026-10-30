//! Cohen's kappa (P4.2) against a real R (skipped with a note when none is
//! found). The answers are worked out by hand from the formulas in the
//! dialog's own comment, on tables small enough to check with a pencil:
//!
//! * a 2 x 2 table with counts 20, 5, 10, 15 (n = 50): observed agreement .7,
//!   chance .5, kappa .4, standard error .1270, z (kappa = 0) 2.8868;
//! * a 3 x 3 table 10 2 0 / 3 12 4 / 1 3 15 (n = 50): kappa .6056 unweighted,
//!   .6732 with linear weights and .7444 with quadratic weights.

use std::time::{Duration, Instant};

use rcraft_catalog::inference::KAPPA;
use rcraft_engine::{Engine, Item, Outcome, Source};
use rcraft_rcodegen::dialog::{ColumnInfo, Context, Style, VarKind};
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

fn context(a: VarKind, b: VarKind) -> Context {
    Context {
        dataset: "d".into(),
        frames: Vec::new(),
        columns: [("a", a), ("b", b)]
            .into_iter()
            .map(|(name, kind)| ColumnInfo {
                name: name.into(),
                kind,
            })
            .collect(),
    }
}

/// The code the dialog writes for the data frame `d` with ratings `a` and `b`.
fn dialog_code(kinds: (VarKind, VarKind), set: &[(&str, &str)]) -> String {
    let ctx = context(kinds.0, kinds.1);
    let mut values = KAPPA.defaults(&ctx);
    values.set_text("a", "a");
    values.set_text("b", "b");
    for (id, v) in set {
        values.set_text(id, *v);
    }
    let code = KAPPA.generate(&ctx, &values, Style::Base);
    assert!(code.is_ok(), "{code:?}");
    code.unwrap_or_default()
}

/// Makes `d` from a table of counts (rows: first rating, columns: second),
/// runs the dialog, and returns what it printed.
fn kappa(e: &mut Engine, counts: &str, kinds: (VarKind, VarKind), set: &[(&str, &str)]) -> String {
    let make = format!(
        "tab <- as.table({counts}); \
         dimnames(tab) <- list(a = c('low', 'mid', 'high')[seq_len(nrow(tab))], \
                               b = c('low', 'mid', 'high')[seq_len(ncol(tab))]); \
         d <- as.data.frame(tab)[rep(seq_len(length(tab)), as.vector(tab)), c('a', 'b')]; \
         rownames(d) <- NULL"
    );
    let (made, text) = run(e, &make);
    assert_eq!(made, Outcome::Ok, "{text}");
    let code = dialog_code(kinds, set);
    let (outcome, text) = run(e, &code);
    assert_eq!(outcome, Outcome::Ok, "{code}\n{text}");
    text
}

/// The value printed on the line that starts with `what`.
fn value(text: &str, what: &str) -> String {
    text.lines()
        .find_map(|l| l.strip_prefix(what))
        .map(|rest| rest.trim().to_string())
        .unwrap_or_default()
}

const TWO: &str = "matrix(c(20, 10, 5, 15), 2)";
const THREE: &str = "matrix(c(10, 3, 1, 2, 12, 3, 0, 4, 15), 3)";

#[test]
fn two_by_two_matches_the_hand_worked_numbers() {
    let Some(mut e) = engine() else { return };
    let kinds = (VarKind::Factor, VarKind::Factor);
    let text = kappa(&mut e, TWO, kinds, &[]);
    assert_eq!(value(&text, "Observed agreement"), "0.7000", "{text}");
    assert_eq!(value(&text, "Agreement expected by chance"), "0.5000");
    assert_eq!(value(&text, "Kappa"), "0.4000");
    assert_eq!(value(&text, "Standard error"), "0.1270");
    assert_eq!(value(&text, "95% confidence interval"), "0.1511 to 0.6489");
    assert_eq!(value(&text, "z (kappa = 0)"), "2.8868");
    assert_eq!(value(&text, "p-value"), "0.00389");
    assert!(text.contains("50 rows with both present"), "{text}");
    // a different level changes the interval, not kappa
    let text = kappa(&mut e, TWO, kinds, &[("conf", "0.9")]);
    assert_eq!(value(&text, "Kappa"), "0.4000");
    assert_eq!(value(&text, "90% confidence interval"), "0.1911 to 0.6089");
}

#[test]
fn three_categories_unweighted_linear_and_quadratic() {
    let Some(mut e) = engine() else { return };
    let kinds = (VarKind::Factor, VarKind::Factor);
    let text = kappa(&mut e, THREE, kinds, &[]);
    assert_eq!(value(&text, "Kappa"), "0.6056", "{text}");
    let text = kappa(&mut e, THREE, kinds, &[("weights", "linear")]);
    assert_eq!(value(&text, "Kappa (linear weights)"), "0.6732", "{text}");
    assert!(text.contains("unweighted kappa only"));
    let text = kappa(&mut e, THREE, kinds, &[("weights", "quadratic")]);
    assert_eq!(
        value(&text, "Kappa (quadratic weights)"),
        "0.7444",
        "{text}"
    );
}

#[test]
fn text_ratings_missing_values_and_refusals() {
    let Some(mut e) = engine() else { return };
    // the same 2 x 2 table with the ratings as text, and two rows missing a rating
    let (made, t) = run(
        &mut e,
        "tab <- matrix(c(20, 10, 5, 15), 2); \
         d <- data.frame(a = rep(c('y', 'n', 'y', 'n'), as.vector(tab)), \
                         b = rep(c('y', 'y', 'n', 'n'), as.vector(tab)), \
                         stringsAsFactors = FALSE); \
         d <- rbind(d, data.frame(a = c(NA, 'y'), b = c('y', NA)))",
    );
    assert_eq!(made, Outcome::Ok, "{t}");
    let code = dialog_code((VarKind::Text, VarKind::Text), &[]);
    let (outcome, text) = run(&mut e, &code);
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(text.contains("50 rows with both present"), "{text}");
    assert_eq!(value(&text, "Kappa"), "0.4000");

    // every rating in one category: kappa is not defined, and R says so
    let (made, t) = run(&mut e, "d <- data.frame(a = rep('x', 5), b = rep('x', 5))");
    assert_eq!(made, Outcome::Ok, "{t}");
    let (outcome, text) = run(&mut e, &code);
    assert_eq!(outcome, Outcome::Error);
    assert!(text.contains("fewer than two different values"), "{text}");

    // the same variable twice is refused before anything runs
    let ctx = context(VarKind::Text, VarKind::Text);
    let mut values = KAPPA.defaults(&ctx);
    values.set_text("a", "a");
    values.set_text("b", "a");
    assert!(KAPPA.generate(&ctx, &values, Style::Base).is_err());
}
