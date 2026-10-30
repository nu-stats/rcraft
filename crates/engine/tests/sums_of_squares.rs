//! Type II and Type III sums of squares (P4.3) against a real R (skipped with a
//! note when none is found). The table is checked, number for number, against
//! values R computes another way:
//!
//! * Type II: the term's row of base R's sequential `anova()` with the term
//!   fitted last (after the terms that do not contain it);
//! * Type III: a Wald test of the term's coefficients in a model fitted with
//!   sum-to-zero coding, `b' V^-1 b * sigma^2` from `coef()` and `vcov()`;
//! * with balanced groups, both equal the sequential table.
//!
//! The data are made without random numbers, with unequal group sizes, so the
//! orders of the terms and the codings really do matter.

use std::time::{Duration, Instant};

use rcraft_catalog::postest::SUMS_OF_SQUARES;
use rcraft_engine::{Engine, Item, Outcome, Source};
use rcraft_rcodegen::dialog::{Context, Style};
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

/// The code the dialog writes for the model object `model`.
fn dialog_code(model: &str, kind: &str) -> String {
    let ctx = Context {
        dataset: model.into(),
        frames: Vec::new(),
        columns: Vec::new(),
    };
    let mut values = SUMS_OF_SQUARES.defaults(&ctx);
    values.set_text("type", kind);
    let code = SUMS_OF_SQUARES.generate(&ctx, &values, Style::Base);
    assert!(code.is_ok(), "{code:?}");
    code.unwrap_or_default()
}

/// Runs R code that must succeed.
fn ok(e: &mut Engine, code: &str) -> String {
    let (outcome, text) = run(e, code);
    assert_eq!(outcome, Outcome::Ok, "{code}\n{text}");
    text
}

/// Unequal groups, two factors, a covariate and weights; no random numbers.
const MAKE: &str = "\
    A <- factor(rep(c('a1', 'a2'), times = c(12, 18))); \
    B <- factor(c(rep(c('b1', 'b2', 'b3'), c(5, 4, 3)), rep(c('b1', 'b2', 'b3'), c(4, 6, 8)))); \
    x <- round(seq(1, 9, length.out = 30)^1.1, 3); \
    y <- round(10 + 1.5 * as.integer(A) + 0.8 * as.integer(B) + \
               2 * (A == 'a2' & B == 'b3') + 0.3 * x + 2 * sin(1:30), 3); \
    wts <- rep(c(1, 2, 0.5), 10); \
    d <- data.frame(y, A, B, x, wts); \
    fit <- lm(y ~ A * B, data = d)";

/// A comparison done inside R: prints `match`, or what differed.
fn compare(e: &mut Engine, got: &str, want: &str) {
    let text = ok(
        e,
        &format!(
            "got <- unname({got}); want <- unname({want}); \
             cat(if (isTRUE(all.equal(got, want, tolerance = 1e-8))) 'match' \
             else paste('MISMATCH', toString(got), '|', toString(want)))"
        ),
    );
    assert!(
        text.trim().ends_with("match") && !text.contains("MISMATCH"),
        "{text}"
    );
}

#[test]
fn type_two_with_an_interaction_matches_the_sequential_tables() {
    let Some(mut e) = engine() else { return };
    ok(&mut e, MAKE);
    let before = ok(&mut e, "cat(coef(fit))");
    let text = ok(&mut e, &format!("res <- {}", dialog_code("fit", "2")));
    assert!(text.contains("Anova Table (Type II tests)"), "{text}");
    assert!(text.contains("Response: y"), "{text}");
    // A after B, B after A, and the interaction last
    compare(
        &mut e,
        "res[c('A', 'B', 'A:B', 'Residuals'), 'Sum Sq']",
        "c(anova(lm(y ~ B + A, d))['A', 'Sum Sq'], anova(lm(y ~ A + B, d))['B', 'Sum Sq'], \
           anova(fit)['A:B', 'Sum Sq'], anova(fit)['Residuals', 'Sum Sq'])",
    );
    compare(
        &mut e,
        "res[c('A', 'B', 'A:B', 'Residuals'), 'Df']",
        "c(1, 2, 2, 24)",
    );
    // F and p follow from the sums of squares and the residual line
    compare(
        &mut e,
        "res[c('A', 'B', 'A:B'), 'F value']",
        "res[c('A', 'B', 'A:B'), 'Mean Sq'] / res['Residuals', 'Mean Sq']",
    );
    compare(
        &mut e,
        "res[c('A', 'B', 'A:B'), 'Pr(>F)']",
        "pf(res[c('A', 'B', 'A:B'), 'F value'], c(1, 2, 2), 24, lower.tail = FALSE)",
    );
    // the model is untouched
    assert_eq!(ok(&mut e, "cat(coef(fit))"), before);
    // the order of the terms in the model makes no difference
    ok(&mut e, "fit_ba <- lm(y ~ B * A, data = d)");
    compare(
        &mut e,
        &format!(
            "{{ r2 <- {}; r2[c('A', 'B', 'B:A', 'Residuals'), 'Sum Sq'] }}",
            dialog_code("fit_ba", "2")
        ),
        "res[c('A', 'B', 'A:B', 'Residuals'), 'Sum Sq']",
    );
}

#[test]
fn type_three_matches_a_wald_test_with_sum_to_zero_coding() {
    let Some(mut e) = engine() else { return };
    ok(&mut e, MAKE);
    let before = ok(&mut e, "cat(coef(fit))");
    let text = ok(&mut e, &format!("res <- {}", dialog_code("fit", "3")));
    assert!(text.contains("Anova Table (Type III tests)"), "{text}");
    assert!(text.contains("(Intercept)"), "{text}");
    assert!(text.contains("sum to zero"), "{text}");
    // the same terms as a Wald test of each term's coefficients, in a model
    // fitted with sum-to-zero coding
    ok(
        &mut e,
        "m3 <- lm(y ~ A * B, data = d, contrasts = list(A = 'contr.sum', B = 'contr.sum')); \
         wald <- sapply(0:3, function(t) { i <- which(m3$assign == t); b <- coef(m3)[i]; \
           v <- vcov(m3)[i, i, drop = FALSE]; \
           drop(t(b) %*% solve(v) %*% b) / length(i) * length(i) * sigma(m3)^2 })",
    );
    compare(
        &mut e,
        "res[c('(Intercept)', 'A', 'B', 'A:B'), 'Sum Sq']",
        "wald",
    );
    compare(&mut e, "res['Residuals', 'Sum Sq']", "sum(resid(fit)^2)");
    // the model fitted with R's own default coding is not changed by it
    assert_eq!(ok(&mut e, "cat(coef(fit))"), before);
}

#[test]
fn without_interactions_the_two_types_agree_and_balanced_groups_match_the_sequential_table() {
    let Some(mut e) = engine() else { return };
    ok(&mut e, MAKE);
    // additive: Type II = Type III for every term
    ok(&mut e, "add <- lm(y ~ A + B + x, data = d, weights = wts)");
    ok(&mut e, &format!("t2 <- {}", dialog_code("add", "2")));
    ok(&mut e, &format!("t3 <- {}", dialog_code("add", "3")));
    compare(
        &mut e,
        "t2[c('A', 'B', 'x', 'Residuals'), 'Sum Sq']",
        "t3[c('A', 'B', 'x', 'Residuals'), 'Sum Sq']",
    );
    // each term after the others, with the weights: base R's sequential table
    compare(
        &mut e,
        "t2[c('A', 'B', 'x'), 'Sum Sq']",
        "c(anova(lm(y ~ B + x + A, d, weights = wts))['A', 'Sum Sq'], \
           anova(lm(y ~ A + x + B, d, weights = wts))['B', 'Sum Sq'], \
           anova(lm(y ~ A + B + x, d, weights = wts))['x', 'Sum Sq'])",
    );
    // balanced groups: the sequential table is already Type II and III
    ok(
        &mut e,
        "bal <- data.frame(A = gl(2, 12, labels = c('p', 'q')), B = gl(3, 4, 24, labels = c('u', 'v', 'w'))); \
         bal$y <- round(5 + as.integer(bal$A) + 0.5 * as.integer(bal$B) + sin(seq_len(24)) * 1.7, 3); \
         bfit <- lm(y ~ A * B, bal)",
    );
    for kind in ["2", "3"] {
        ok(&mut e, &format!("tb <- {}", dialog_code("bfit", kind)));
        compare(
            &mut e,
            "tb[c('A', 'B', 'A:B', 'Residuals'), 'Sum Sq']",
            "anova(bfit)[c('A', 'B', 'A:B', 'Residuals'), 'Sum Sq']",
        );
    }
}

#[test]
fn a_model_that_is_not_linear_or_has_redundant_terms_is_refused_in_words() {
    let Some(mut e) = engine() else { return };
    ok(&mut e, MAKE);
    ok(&mut e, "g <- glm(y > 11 ~ A, family = binomial, data = d)");
    let (outcome, text) = run(&mut e, &dialog_code("g", "2"));
    assert_eq!(outcome, Outcome::Error);
    assert!(text.contains("Linear model dialog"), "{text}");
    // a copy of a factor is redundant with it: R gives an NA coefficient
    ok(&mut e, "d$A2 <- d$A; red <- lm(y ~ A + A2, data = d)");
    let (outcome, text) = run(&mut e, &dialog_code("red", "3"));
    assert_eq!(outcome, Outcome::Error);
    assert!(text.contains("redundant"), "{text}");
    // a model with no terms at all is not a crash
    ok(&mut e, "empty <- lm(y ~ 1, data = d)");
    let (outcome, _) = run(&mut e, &dialog_code("empty", "2"));
    let _ = outcome;
    ok(&mut e, "cat(1 + 1)");
}
