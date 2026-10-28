//! Post-estimation dialogs (P4.5) end to end against a real R (skipped with a
//! note when none is found). Models are fitted in R by hand; each dialog is
//! then built on the model's name, run, and its output or the columns it adds
//! compared with what R gives for the same question asked directly.

use std::time::{Duration, Instant};

use rcraft_catalog::postest::{
    COMPARE_MODELS, DIAGNOSTICS, MARGINAL_EFFECTS, MODEL_FIT, PREDICTED, ROBUST, TEST_TERMS,
};
use rcraft_engine::{Engine, Item, Outcome, Source};
use rcraft_rcodegen::dialog::{Context, Spec, Style, Widget};
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
                    Item::Warning(w) => format!("warning: {w}\n"),
                    Item::Error { message, .. } => format!("error: {message}\n"),
                    Item::Notice(n) => n.clone(),
                })
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default();
    (outcome, text)
}

/// Two small models fitted to made-up data, one with a missing predictor value.
const SETUP: &str = "d <- data.frame(\
     y = c(10.1, 12.3, 11.8, 14.2, 15.1, 13.9, 17.2, 18.1, 16.5, 19.0, 20.2, 18.8), \
     x = c(1, 2, 3, NA, 5, 6, 7, 8, 9, 10, 11, 12), \
     z = c(2.1, 1.9, 3.2, 2.8, 3.9, 3.1, 4.5, 4.0, 5.2, 4.8, 6.1, 5.5), \
     g = rep(c('a', 'b', 'c', 'd'), each = 3), \
     won = c(0, 0, 0, 1, 0, 0, 1, 0, 1, 1, 1, 1)); \
     m1 <- lm(y ~ x, data = d); m2 <- lm(y ~ x + z, data = d, subset = !is.na(x)); \
     g1 <- glm(won ~ z, data = d, family = binomial)";

/// Runs `spec` on the model called `model`, with these values.
fn on_model(
    e: &mut Engine,
    spec: &Spec,
    model: &str,
    set: &[(&str, &str)],
    flags: &[&str],
) -> (Outcome, String) {
    let ctx = Context {
        dataset: model.into(),
        columns: Vec::new(),
    };
    let mut values = spec.defaults(&ctx);
    for (id, text) in set {
        values.set_text(id, *text);
    }
    for w in spec.widgets {
        if let Widget::Check { id, .. } = w {
            values.set_flag(id, flags.contains(id));
        }
    }
    let code = spec.generate(&ctx, &values, Style::Base);
    assert!(code.is_ok(), "{code:?}");
    run(e, Source::Menu(spec.menu.into()), &code.unwrap_or_default())
}

fn holds(e: &mut Engine, exprs: &str) {
    let (outcome, text) = run(e, Source::Typed, &format!("stopifnot({exprs})"));
    assert_eq!(outcome, Outcome::Ok, "{exprs}\n{text}");
}

fn tidy(text: &str) -> Vec<String> {
    text.lines()
        .filter(|l| !l.starts_with("warning:"))
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|l| !l.is_empty())
        .collect()
}

fn started() -> Option<Engine> {
    let mut e = engine()?;
    settle(&mut e);
    let (outcome, text) = run(&mut e, Source::Typed, SETUP);
    assert_eq!(outcome, Outcome::Ok, "{text}");
    Some(e)
}

#[test]
fn model_fit_reports_the_numbers_r_gives() {
    let Some(mut e) = started() else { return };
    let (outcome, text) = on_model(&mut e, &MODEL_FIT, "m1", &[], &[]);
    assert_eq!(outcome, Outcome::Ok, "{text}");
    // m1 drops the row with the missing x: 11 observations
    assert!(text.contains("Observations: 11"), "{text}");
    let (_, expected) = run(
        &mut e,
        Source::Typed,
        "s <- summary(m1); print(c(logLik = as.numeric(logLik(m1)), df = attr(logLik(m1), 'df'), \
         AIC = AIC(m1), BIC = BIC(m1))); print(c(r.squared = s$r.squared, adj.r.squared = s$adj.r.squared))",
    );
    let shown = tidy(&text);
    for line in tidy(&expected) {
        assert!(shown.contains(&line), "missing {line:?} in {text}");
    }
    // a generalised model shows deviances instead
    let (outcome, text) = on_model(&mut e, &MODEL_FIT, "g1", &[], &[]);
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(
        text.contains("Deviance") && !text.contains("R-squared"),
        "{text}"
    );
}

#[test]
fn compare_models_tables_and_tests_them() {
    let Some(mut e) = started() else { return };
    // m1 and m2 are fitted to the same 11 rows, m2 with one more predictor
    let (outcome, text) = on_model(
        &mut e,
        &COMPARE_MODELS,
        "m1",
        &[("others", "m2")],
        &["nested"],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let (_, expected) = run(
        &mut e,
        Source::Typed,
        "print(data.frame(model = c('m1', 'm2'), df = c(attr(logLik(m1), 'df'), attr(logLik(m2), 'df')), \
         logLik = c(as.numeric(logLik(m1)), as.numeric(logLik(m2))), AIC = AIC(m1, m2)$AIC, \
         BIC = BIC(m1, m2)$BIC)); cat('\\nTest of nested models\\n'); print(anova(m1, m2, test = 'F'))",
    );
    assert_eq!(tidy(&text), tidy(&expected), "{text}");
    // without the test only the table is printed
    let (_, plain) = on_model(&mut e, &COMPARE_MODELS, "m1", &[("others", "m2")], &[]);
    assert!(!plain.contains("Test of nested models"), "{plain}");
    // a name that is not in the workspace is R's error to report
    let (outcome, text) = on_model(&mut e, &COMPARE_MODELS, "m1", &[("others", "nothing")], &[]);
    assert_eq!(outcome, Outcome::Error, "{text}");
}

#[test]
fn predicted_values_and_residuals_line_up_with_the_rows() {
    let Some(mut e) = started() else { return };
    let (outcome, text) = on_model(
        &mut e,
        &PREDICTED,
        "m1",
        &[("data", "d"), ("fitted", "yhat"), ("resid", "e")],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    // row 4 has a missing x: no prediction and no residual there, and the rest match R
    holds(
        &mut e,
        "is.na(d$yhat[4]), is.na(d$e[4]), \
         isTRUE(all.equal(d$yhat[-4], unname(fitted(m1)))), \
         isTRUE(all.equal(d$e[-4], unname(residuals(m1)))), \
         !exists('res')",
    );
    // a generalised model: the scale of the prediction and the kind of residual are options
    let (outcome, text) = on_model(
        &mut e,
        &PREDICTED,
        "g1",
        &[
            ("data", "d"),
            ("fitted", "p"),
            ("resid", "r"),
            ("rtype", "pearson"),
        ],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    holds(
        &mut e,
        "isTRUE(all.equal(d$p, unname(fitted(g1)))), \
         isTRUE(all.equal(d$r, unname(residuals(g1, type = 'pearson'))))",
    );
    let (outcome, _) = on_model(
        &mut e,
        &PREDICTED,
        "g1",
        &[
            ("data", "d"),
            ("fitted", "lp"),
            ("resid", ""),
            ("type", "link"),
        ],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok);
    holds(
        &mut e,
        "isTRUE(all.equal(d$lp, unname(predict(g1, type = 'link'))))",
    );
    // a data frame the model was not fitted to is refused for the residuals
    let (outcome, text) = run(&mut e, Source::Typed, "other <- data.frame(a = 1:3); 0");
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let (outcome, text) = on_model(
        &mut e,
        &PREDICTED,
        "m1",
        &[("data", "other"), ("fitted", ""), ("resid", "e")],
        &[],
    );
    assert_eq!(outcome, Outcome::Error, "{text}");
}

#[test]
fn diagnostics_match_the_tests_r_gives() {
    let Some(mut e) = started() else { return };
    let (outcome, text) = on_model(&mut e, &DIAGNOSTICS, "m1", &[], &[]);
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let (_, expected) = run(
        &mut e,
        Source::Typed,
        "r <- residuals(m1); n <- length(r); s <- shapiro.test(r); \
         bp <- n * summary(lm(r^2 ~ fitted(m1)))$r.squared; \
         cat(signif(s$statistic, 5), signif(s$p.value, 4), signif(bp, 5), \
         signif(pchisq(bp, 1, lower.tail = FALSE), 4), \
         sum(cooks.distance(m1) > 4 / n), sum(hatvalues(m1) > 2 * length(coef(m1)) / n), sep = ' ')",
    );
    let numbers: Vec<&str> = expected.split_whitespace().collect();
    assert_eq!(numbers.len(), 6, "{expected}");
    let lines = tidy(&text);
    assert!(
        lines
            .iter()
            .any(|l| l.contains(&format!("W = {}, p = {}", numbers[0], numbers[1]))),
        "{text}"
    );
    assert!(
        lines.iter().any(|l| l.contains(&format!(
            "chi-squared(1) = {}, p = {}",
            numbers[2], numbers[3]
        ))),
        "{text}"
    );
    assert!(
        lines
            .iter()
            .any(|l| l.ends_with(&format!("above 4/n: {}", numbers[4]))),
        "{text}"
    );
    assert!(
        lines
            .iter()
            .any(|l| l.ends_with(&format!("above 2k/n: {}", numbers[5]))),
        "{text}"
    );
    // for a generalised model only the influence part is shown
    let (outcome, text) = on_model(&mut e, &DIAGNOSTICS, "g1", &[], &[]);
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(
        !text.contains("Shapiro") && text.contains("Cook's distance"),
        "{text}"
    );
    // a model that is not linear or generalised linear is refused
    let (outcome, text) = run(&mut e, Source::Typed, "notmodel <- 1:3; 0");
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let (outcome, _) = on_model(&mut e, &DIAGNOSTICS, "notmodel", &[], &[]);
    assert_eq!(outcome, Outcome::Error);
}

/// Robust standard errors need the sandwich and lmtest packages; skipped
/// with a note where either is missing (CI installs both).
#[test]
fn robust_standard_errors_are_the_ones_sandwich_gives() {
    let Some(mut e) = started() else { return };
    for package in ["sandwich", "lmtest"] {
        let (outcome, _) = run(
            &mut e,
            Source::Typed,
            &format!("stopifnot(requireNamespace('{package}', quietly = TRUE))"),
        );
        if outcome != Outcome::Ok {
            eprintln!("skipped: the R package {package} is not installed");
            return;
        }
    }
    let (outcome, text) = on_model(&mut e, &ROBUST, "m1", &[("type", "HC3")], &[]);
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let (_, expected) = run(
        &mut e,
        Source::Typed,
        "v <- sandwich::vcovHC(m1, type = 'HC3'); cat('Coefficients with HC3 robust standard errors\\n'); \
         print(lmtest::coeftest(m1, vcov. = v)); cat('\\nConfidence intervals\\n'); \
         print(lmtest::coefci(m1, vcov. = v, level = 0.95))",
    );
    assert_eq!(tidy(&text), tidy(&expected), "{text}");
    // a different correction changes the standard errors
    let (_, hc0) = on_model(&mut e, &ROBUST, "m1", &[("type", "HC0")], &[]);
    assert_ne!(tidy(&hc0)[2..], tidy(&text)[2..]);
}

#[test]
fn test_terms_gives_the_tests_r_gives() {
    let Some(mut e) = started() else { return };
    let (outcome, text) = on_model(&mut e, &TEST_TERMS, "m2", &[], &[]);
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let (_, expected) = run(&mut e, Source::Typed, "print(drop1(m2, test = 'F'))");
    assert_eq!(tidy(&text), tidy(&expected), "{text}");

    let (outcome, text) = on_model(
        &mut e,
        &TEST_TERMS,
        "m2",
        &[("kind", "together"), ("terms", "x, z")],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let (_, expected) = run(
        &mut e,
        Source::Typed,
        "reduced <- update(m2, . ~ . - x - z); \
         cat('Model without the terms against the full model\n'); \
         print(anova(reduced, m2, test = 'F'))",
    );
    assert_eq!(tidy(&text), tidy(&expected), "{text}");

    // a generalised model gets a likelihood-ratio test
    let (outcome, text) = on_model(&mut e, &TEST_TERMS, "g1", &[], &[]);
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let (_, expected) = run(&mut e, Source::Typed, "print(drop1(g1, test = 'Chisq'))");
    assert_eq!(tidy(&text), tidy(&expected), "{text}");

    // something that is not a term of the model is refused by the generated R
    let (outcome, text) = on_model(
        &mut e,
        &TEST_TERMS,
        "m2",
        &[("kind", "together"), ("terms", "nothing")],
        &[],
    );
    assert_eq!(outcome, Outcome::Error, "{text}");
    assert!(text.contains("not terms of the model"), "{text}");
}

/// Marginal effects need the marginaleffects package; skipped with a note
/// where it is missing (CI installs it).
#[test]
fn average_marginal_effects_are_the_ones_marginaleffects_gives() {
    let Some(mut e) = started() else { return };
    let (outcome, _) = run(
        &mut e,
        Source::Typed,
        "stopifnot(requireNamespace('marginaleffects', quietly = TRUE))",
    );
    if outcome != Outcome::Ok {
        eprintln!("skipped: the R package marginaleffects is not installed");
        return;
    }
    let (outcome, text) = on_model(&mut e, &MARGINAL_EFFECTS, "g1", &[], &[]);
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let (_, expected) = run(
        &mut e,
        Source::Typed,
        "print(marginaleffects::avg_slopes(g1, conf_level = 0.95))",
    );
    assert_eq!(tidy(&text), tidy(&expected), "{text}");
    // for a logistic model the effect of z is a change in probability: it is
    // not the log-odds coefficient
    holds(
        &mut e,
        "abs(marginaleffects::avg_slopes(g1)$estimate - unname(coef(g1)['z'])) > 1e-6",
    );
}

/// Clustered standard errors need sandwich and lmtest; skipped with a note
/// where either is missing (CI installs both).
#[test]
fn clustered_standard_errors_line_up_with_the_rows_the_model_kept() {
    let Some(mut e) = started() else { return };
    for package in ["sandwich", "lmtest"] {
        let (outcome, _) = run(
            &mut e,
            Source::Typed,
            &format!("stopifnot(requireNamespace('{package}', quietly = TRUE))"),
        );
        if outcome != Outcome::Ok {
            eprintln!("skipped: the R package {package} is not installed");
            return;
        }
    }
    // m1 left row 4 out (a missing x); the cluster variable must be matched on rows
    let (outcome, text) = on_model(
        &mut e,
        &ROBUST,
        "m1",
        &[("type", "HC1"), ("cluster", "g"), ("data", "d")],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let (_, expected) = run(
        &mut e,
        Source::Typed,
        "kept <- d[rownames(model.frame(m1)), ]; v <- sandwich::vcovCL(m1, cluster = kept$g, type = 'HC1'); \
         cat('Coefficients with HC1 cluster-robust (by g) standard errors\n'); \
         print(lmtest::coeftest(m1, vcov. = v)); cat('\nConfidence intervals\n'); \
         print(lmtest::coefci(m1, vcov. = v, level = 0.95))",
    );
    assert_eq!(tidy(&text), tidy(&expected), "{text}");
}
