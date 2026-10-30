//! Custom contrasts (P4.3) against a real R (skipped with a note when none is
//! found). Every figure is checked against R computed another way:
//!
//! * one factor: a comparison of group means has the textbook standard error
//!   `sqrt(MSE * sum(w^2 / n))`, with the residual degrees of freedom;
//! * additive models with other factors and a covariate: a comparison of levels
//!   is a difference of treatment-coded coefficients, whose estimate and
//!   standard error `summary()` gives;
//! * an interaction model: the estimated means equal the average of `predict()`
//!   over the other factor's levels, with the covariate at its mean.

use std::time::{Duration, Instant};

use rcraft_catalog::postest::CONTRASTS;
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

fn ok(e: &mut Engine, code: &str) -> String {
    let (outcome, text) = run(e, code);
    assert_eq!(outcome, Outcome::Ok, "{code}\n{text}");
    text
}

/// The code the dialog writes for the model object `model`.
fn dialog_code(model: &str, factor: &str, contrasts: &str, set: &[(&str, &str)]) -> String {
    let ctx = Context {
        dataset: model.into(),
        frames: Vec::new(),
        columns: Vec::new(),
    };
    let mut values = CONTRASTS.defaults(&ctx);
    values.set_text("factor", factor);
    values.set_text("contrasts", contrasts);
    for (id, v) in set {
        values.set_text(id, *v);
    }
    let code = CONTRASTS.generate(&ctx, &values, Style::Base);
    assert!(code.is_ok(), "{code:?}");
    code.unwrap_or_default()
}

/// A comparison done inside R: passes, or says what differed.
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

/// Unequal groups, no random numbers.
const ONE_WAY: &str = "\
    d <- data.frame(\
      g = factor(rep(c('a', 'b', 'c'), c(6, 5, 4))), \
      y = c(10.2, 11.1, 9.8, 10.9, 10.5, 11.4, 12.3, 13.1, 12.7, 13.8, 12.2, 9.1, 8.7, 9.9, 9.4)); \
    fit <- lm(y ~ g, data = d); \
    mse <- sum(resid(fit)^2) / df.residual(fit); \
    mu <- tapply(d$y, d$g, mean); n <- table(d$g)";

#[test]
fn one_factor_comparisons_have_the_textbook_standard_error() {
    let Some(mut e) = engine() else { return };
    ok(&mut e, ONE_WAY);
    let text = ok(
        &mut e,
        &format!(
            "res <- {}",
            dialog_code("fit", "g", "a = 1, b = -1; a = 1, b = -0.5, c = -0.5", &[])
        ),
    );
    assert!(text.contains("Estimated means of g"), "{text}");
    assert!(
        text.contains("Comparisons (intervals are not adjusted)"),
        "{text}"
    );
    // the estimates
    compare(
        &mut e,
        "res$Estimate",
        "c(mu['a'] - mu['b'], mu['a'] - 0.5 * mu['b'] - 0.5 * mu['c'])",
    );
    // the standard errors: sqrt(MSE * sum(w^2 / n))
    compare(
        &mut e,
        "res[['Std. Error']]",
        "c(sqrt(mse * (1 / n['a'] + 1 / n['b'])), \
           sqrt(mse * (1 / n['a'] + 0.25 / n['b'] + 0.25 / n['c'])))",
    );
    // the test and the interval, on the residual degrees of freedom
    compare(&mut e, "res$df", "c(12, 12)");
    compare(&mut e, "res$t", "res$Estimate / res[['Std. Error']]");
    compare(
        &mut e,
        "res$p",
        "2 * pt(-abs(res$Estimate / res[['Std. Error']]), 12)",
    );
    compare(
        &mut e,
        "res[['lower 95%']]",
        "res$Estimate - qt(0.975, 12) * res[['Std. Error']]",
    );
    // the means table: group means, with the standard error sqrt(MSE / n)
    let means = ok(&mut e, "cat(format(mu, digits = 6))");
    assert!(means.contains("10.65"), "{means}");
    // another level changes the interval only
    ok(
        &mut e,
        &format!(
            "res90 <- {}",
            dialog_code("fit", "g", "a = 1, b = -1", &[("conf", "0.9")])
        ),
    );
    compare(
        &mut e,
        "res90[['lower 90%']]",
        "res$Estimate[1] - qt(0.95, 12) * res[['Std. Error']][1]",
    );
}

/// Two factors and a covariate, unbalanced.
const TWO_FACTORS: &str = "\
    d <- data.frame(\
      A = factor(c(rep('a1', 11), rep('a2', 13))), \
      B = factor(c(rep(c('b1', 'b2', 'b3'), c(4, 4, 3)), rep(c('b1', 'b2', 'b3'), c(3, 4, 6)))), \
      x = round(seq(2, 9, length.out = 24)^1.05, 3)); \
    d$y <- round(5 + 1.2 * (d$A == 'a2') + 0.7 * as.integer(d$B) + 0.4 * d$x + \
                 1.5 * (d$A == 'a2' & d$B == 'b3') + 1.3 * sin(1:24), 3)";

#[test]
fn comparisons_in_an_additive_model_are_coefficient_differences() {
    let Some(mut e) = engine() else { return };
    ok(&mut e, TWO_FACTORS);
    ok(
        &mut e,
        "add <- lm(y ~ A + B + x, data = d); co <- summary(add)$coefficients",
    );
    ok(
        &mut e,
        &format!(
            "res <- {}",
            dialog_code("add", "B", "b2 = 1, b1 = -1; b3 = 1, b1 = -1", &[])
        ),
    );
    // treatment coding: b2 - b1 and b3 - b1 are the Bb2 and Bb3 coefficients
    compare(&mut e, "res$Estimate", "co[c('Bb2', 'Bb3'), 'Estimate']");
    compare(
        &mut e,
        "res[['Std. Error']]",
        "co[c('Bb2', 'Bb3'), 'Std. Error']",
    );
    compare(&mut e, "res$p", "co[c('Bb2', 'Bb3'), 'Pr(>|t|)']");
    // and a comparison on the other factor
    ok(
        &mut e,
        &format!(
            "resA <- {}",
            dialog_code("add", "A", "a2 = 1, a1 = -1", &[])
        ),
    );
    compare(&mut e, "resA$Estimate", "co['Aa2', 'Estimate']");
    compare(&mut e, "resA[['Std. Error']]", "co['Aa2', 'Std. Error']");
}

#[test]
fn estimated_means_average_over_the_other_factors_and_hold_numbers_at_their_means() {
    let Some(mut e) = engine() else { return };
    ok(&mut e, TWO_FACTORS);
    ok(&mut e, "inter <- lm(y ~ A * B + x, data = d)");
    ok(
        &mut e,
        &format!(
            "res <- {}",
            dialog_code("inter", "A", "a2 = 1, a1 = -1", &[])
        ),
    );
    // predict() over every B level at the mean of x, averaged, for each A level
    ok(
        &mut e,
        "avg <- function(a) mean(predict(inter, data.frame(A = factor(a, levels = levels(d$A)), \
                                B = factor(levels(d$B), levels = levels(d$B)), x = mean(d$x)))); \
         m1 <- avg('a1'); m2 <- avg('a2')",
    );
    compare(&mut e, "res$Estimate", "m2 - m1");
    // a weighted comparison of three levels of the other factor, the same way
    ok(
        &mut e,
        &format!(
            "resB <- {}",
            dialog_code("inter", "B", "b3 = 1, b1 = -0.5, b2 = -0.5", &[])
        ),
    );
    ok(
        &mut e,
        "avgb <- function(b) mean(predict(inter, data.frame(B = factor(b, levels = levels(d$B)), \
                                 A = factor(levels(d$A), levels = levels(d$A)), x = mean(d$x)))); \
         want <- avgb('b3') - 0.5 * avgb('b1') - 0.5 * avgb('b2')",
    );
    compare(&mut e, "resB$Estimate", "want");
}

#[test]
fn several_comparisons_can_be_adjusted_and_bad_ones_are_refused_in_words() {
    let Some(mut e) = engine() else { return };
    ok(&mut e, ONE_WAY);
    let three = "a = 1, b = -1; a = 1, c = -1; b = 1, c = -1";
    ok(
        &mut e,
        &format!("raw <- {}", dialog_code("fit", "g", three, &[])),
    );
    for method in ["holm", "bonferroni"] {
        ok(
            &mut e,
            &format!(
                "adj <- {}",
                dialog_code("fit", "g", three, &[("adjust", method)])
            ),
        );
        compare(&mut e, "adj[[6]]", &format!("p.adjust(raw$p, '{method}')"));
        // estimates and intervals are not adjusted
        compare(&mut e, "adj$Estimate", "raw$Estimate");
    }
    // weights that do not add up to zero
    let (outcome, text) = run(&mut e, &dialog_code("fit", "g", "a = 1, b = -0.5", &[]));
    assert_eq!(outcome, Outcome::Error);
    assert!(text.contains("add up to 0.5, not 0"), "{text}");
    // a level the factor does not have names the levels it has
    let (outcome, text) = run(&mut e, &dialog_code("fit", "g", "a = 1, z = -1", &[]));
    assert_eq!(outcome, Outcome::Error);
    assert!(
        text.contains("'z' is not a level of g") && text.contains("a, b, c"),
        "{text}"
    );
    // something that is not a weight
    let (outcome, text) = run(&mut e, &dialog_code("fit", "g", "a = one, b = -1", &[]));
    assert_eq!(outcome, Outcome::Error);
    assert!(
        text.contains("a level, an equals sign and a weight"),
        "{text}"
    );
    // a variable that is not a factor of the model
    let (outcome, text) = run(&mut e, &dialog_code("fit", "y", "a = 1, b = -1", &[]));
    assert_eq!(outcome, Outcome::Error);
    assert!(text.contains("is not a factor in the model"), "{text}");
    // a curved number variable, a generalised model
    ok(
        &mut e,
        "d$x <- seq_len(15); curve <- lm(y ~ g + log(x), data = d)",
    );
    let (outcome, text) = run(&mut e, &dialog_code("curve", "g", "a = 1, b = -1", &[]));
    assert_eq!(outcome, Outcome::Error);
    assert!(text.contains("enter as they are"), "{text}");
    ok(&mut e, "gl <- glm(y > 10 ~ g, family = binomial, data = d)");
    let (outcome, text) = run(&mut e, &dialog_code("gl", "g", "a = 1, b = -1", &[]));
    assert_eq!(outcome, Outcome::Error);
    assert!(text.contains("Linear model dialog"), "{text}");
    ok(&mut e, "cat(1 + 1)");
}

#[test]
fn the_dialog_asks_for_a_factor_and_a_comparison() {
    let ctx = Context {
        dataset: "fit".into(),
        frames: Vec::new(),
        columns: Vec::new(),
    };
    let mut values = CONTRASTS.defaults(&ctx);
    assert!(CONTRASTS.generate(&ctx, &values, Style::Base).is_err());
    values.set_text("factor", "g");
    assert!(CONTRASTS.generate(&ctx, &values, Style::Base).is_err());
    values.set_text("contrasts", "a = 1, b = -1");
    assert!(CONTRASTS.generate(&ctx, &values, Style::Base).is_ok());
}
