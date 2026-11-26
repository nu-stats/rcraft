//! Post-estimation dialogs (P4.5) end to end against a real R (skipped with a
//! note when none is found). Models are fitted in R by hand; each dialog is
//! then built on the model's name, run, and its output or the columns it adds
//! compared with what R gives for the same question asked directly.

mod pinned;

use std::time::{Duration, Instant};

use rcraft_catalog::postest::{
    COMPARE_MODELS, DIAGNOSTICS, DIAGNOSTIC_PLOTS, MARGINAL_EFFECTS, MODEL_FIT, PREDICTED, ROBUST,
    TEST_TERMS,
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
        frames: Vec::new(),
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

/// Fit statistics, Average marginal effects and Compare models on an object
/// that is not a model say so in the dialog's own words, not R's
/// "no 'nobs' method is available" (SW4).
#[test]
fn a_non_model_gets_a_clear_message_from_fit_marginal_effects_and_compare() {
    let Some(mut e) = started() else { return };
    for (spec, set) in [
        (&MODEL_FIT, vec![]),
        (&MARGINAL_EFFECTS, vec![]),
        (&COMPARE_MODELS, vec![("others", "m1")]),
    ] {
        let (outcome, text) = on_model(&mut e, spec, "d", &set, &[]);
        assert_ne!(outcome, Outcome::Ok, "{}: {text}", spec.entry);
        assert!(
            text.contains("fitted model"),
            "{}: the message should say what the dialog takes: {text}",
            spec.entry
        );
        assert!(!text.contains("nobs"), "{}: {text}", spec.entry);
    }
    // a real model still runs
    let (outcome, text) = on_model(&mut e, &MODEL_FIT, "m1", &[], &[]);
    assert_eq!(outcome, Outcome::Ok, "{text}");
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

#[test]
fn diagnostic_plots_draw_one_page_of_four_panels() {
    let Some(mut e) = started() else { return };
    for model in ["m1", "g1"] {
        let (outcome, text) = on_model(&mut e, &DIAGNOSTIC_PLOTS, model, &[], &[]);
        assert_eq!(outcome, Outcome::Ok, "{model}: {text}");
        let plots = e
            .results()
            .blocks()
            .last()
            .map(|b| {
                b.items
                    .iter()
                    .filter(|i| matches!(i, Item::Plot { .. }))
                    .count()
            })
            .unwrap_or(0);
        assert_eq!(plots, 1, "{model}: four panels are one page");
    }
    // a plain object is refused
    let (outcome, _) = run(&mut e, Source::Typed, "notmodel <- 1:3; 0");
    assert_eq!(outcome, Outcome::Ok);
    let (outcome, _) = on_model(&mut e, &DIAGNOSTIC_PLOTS, "notmodel", &[], &[]);
    assert_eq!(outcome, Outcome::Error);
    // the layout is put back: the next plot is a single panel
    let (outcome, text) = run(
        &mut e,
        Source::Typed,
        "stopifnot(identical(par('mfrow'), c(1L, 1L)))",
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
}

/// Base R's stats; sandwich, lmtest and marginaleffects for three dialogs.
const PACKAGES: &[&str] = &[
    "sandwich",
    "lmtest",
    "marginaleffects",
    "gt",
    "openxlsx2",
    "flextable",
    "officer",
];

/// The models of [`SETUP`] worked out in plain R, without lm or glm: least
/// squares from the normal equations on the 11 rows with an x, and the
/// logistic model by Newton-Raphson.
const BY_HAND: &str = "ok <- !is.na(d$x); y <- d$y[ok]; n <- sum(ok); \
     X1 <- cbind(1, d$x[ok]); X2 <- cbind(1, d$x[ok], d$z[ok]); \
     ols <- function(X, y) { b <- as.vector(solve(crossprod(X), crossprod(X, y))); \
       r <- as.vector(y - X %*% b); list(b = b, r = r, rss = sum(r^2), p = ncol(X)) }; \
     f1 <- ols(X1, y); f2 <- ols(X2, y); tss <- sum((y - mean(y))^2); \
     llf <- function(f) -n / 2 * (log(2 * pi) + log(f$rss / n) + 1); \
     G <- cbind(1, d$z); w <- d$won; b <- c(0, 0); \
     for (i in 1:100) { p <- as.vector(plogis(G %*% b)); \
       b <- b + as.vector(solve(t(G) %*% (p * (1 - p) * G), t(G) %*% (w - p))) }; \
     p <- as.vector(plogis(G %*% b)); \
     dev <- function(p) -2 * sum(w * log(p) + (1 - w) * log(1 - p))";

/// Runs `spec` on `model` as the menu does and keeps its value as `res`.
fn kept(e: &mut Engine, spec: &Spec, model: &str, set: &[(&str, &str)], flags: &[&str]) {
    let ctx = Context {
        dataset: model.into(),
        frames: Vec::new(),
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

/// What R prints for `code` run after [`BY_HAND`].
fn hand_text(e: &mut Engine, code: &str) -> String {
    let (outcome, text) = run(e, Source::Typed, &format!("local({{ {BY_HAND}; {code} }})"));
    assert_eq!(outcome, Outcome::Ok, "{code}\n{text}");
    text
}

/// Every line of `want` is among the lines of `got`.
fn shows(got: &str, want: &str) {
    let lines = tidy(got);
    for line in tidy(want) {
        assert!(lines.contains(&line), "missing {line:?} in\n{got}");
    }
}

/// The post-estimation dialogs against the models worked out by hand: the fit
/// numbers and the comparison table to the digits printed; the nested F test,
/// the predictions and residuals, Cook's distance and leverage, the robust
/// intervals, the tests of terms and the average marginal effect as numbers.
#[test]
fn post_estimation_numbers_are_the_textbook_formulas() {
    let Some(mut e) = started() else { return };
    for package in ["sandwich", "lmtest", "marginaleffects"] {
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
    let check = |e: &mut Engine, setup: &str, checks: &[(&str, &str)]| {
        holds(e, &pinned::all_of(&format!("{BY_HAND}; {setup}"), checks));
    };

    // model fit, linear: the log-likelihood, AIC, BIC and R-squared
    let (_, text) = on_model(&mut e, &MODEL_FIT, "m1", &[], &[]);
    shows(
        &text,
        &hand_text(
            &mut e,
            "ll <- llf(f1); cat('Observations: ', n, '\\n', sep = ''); \
             print(c(logLik = ll, df = 3, AIC = -2 * ll + 2 * 3, BIC = -2 * ll + log(n) * 3)); \
             r2 <- 1 - f1$rss / tss; \
             print(c(r.squared = r2, adj.r.squared = 1 - (1 - r2) * (n - 1) / (n - 2)))",
        ),
    );
    // and logistic: the deviances
    let (_, text) = on_model(&mut e, &MODEL_FIT, "g1", &[], &[]);
    shows(
        &text,
        &hand_text(
            &mut e,
            "print(c(null = dev(rep(mean(w), length(w))), residual = dev(p)))",
        ),
    );

    // compare models: the table to the digits printed, the F test as numbers
    kept(
        &mut e,
        &COMPARE_MODELS,
        "m1",
        &[("others", "m2")],
        &["nested"],
    );
    let (_, text) = on_model(&mut e, &COMPARE_MODELS, "m1", &[("others", "m2")], &[]);
    shows(
        &text,
        &hand_text(
            &mut e,
            "ll <- c(llf(f1), llf(f2)); \
             print(data.frame(model = c('m1', 'm2'), df = c(3, 4), logLik = ll, \
             AIC = -2 * ll + 2 * c(3, 4), BIC = -2 * ll + log(n) * c(3, 4)))",
        ),
    );
    check(
        &mut e,
        "f <- (f1$rss - f2$rss) / (f2$rss / (n - 3))",
        &[
            ("rss", "all.equal(res$RSS, c(f1$rss, f2$rss), tolerance = 1e-12)"),
            ("f", "all.equal(res$F[2], f, tolerance = 1e-12)"),
            (
                "p",
                "all.equal(res[['Pr(>F)']][2], pf(f, 1, n - 3, lower.tail = FALSE), tolerance = 1e-12)",
            ),
        ],
    );

    // predicted values and residuals, line by line, with none on row 4
    on_model(
        &mut e,
        &PREDICTED,
        "m1",
        &[("data", "d"), ("fitted", "yhat"), ("resid", "e")],
        &[],
    );
    on_model(
        &mut e,
        &PREDICTED,
        "g1",
        &[
            ("data", "d"),
            ("fitted", "pr"),
            ("resid", "pe"),
            ("rtype", "pearson"),
        ],
        &[],
    );
    on_model(
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
    check(
        &mut e,
        "",
        &[
            (
                "fitted",
                "all.equal(d$yhat[ok], as.vector(X1 %*% f1$b), tolerance = 1e-12)",
            ),
            ("resid", "all.equal(d$e[ok], f1$r, tolerance = 1e-10)"),
            ("row4", "is.na(d$yhat[4]) && is.na(d$e[4])"),
            ("prob", "all.equal(d$pr, p, tolerance = 1e-8)"),
            (
                "pearson",
                "all.equal(d$pe, (w - p) / sqrt(p * (1 - p)), tolerance = 1e-8)",
            ),
            (
                "link",
                "all.equal(d$lp, as.vector(G %*% b), tolerance = 1e-8)",
            ),
        ],
    );

    // diagnostics: Shapiro-Wilk of the residuals, Koenker's Breusch-Pagan,
    // Cook's distance and leverage from the hat matrix
    let (_, text) = on_model(&mut e, &DIAGNOSTICS, "m1", &[], &[]);
    let hand = hand_text(
        &mut e,
        &format!(
            "{}; r <- f1$r; s <- sw(r); fit <- as.vector(X1 %*% f1$b); \
             aux <- ols(cbind(1, fit), r^2); bp <- n * (1 - aux$rss / sum((r^2 - mean(r^2))^2)); \
             h <- diag(X1 %*% solve(crossprod(X1)) %*% t(X1)); s2 <- f1$rss / (n - 2); \
             cd <- r^2 / (2 * s2) * h / (1 - h)^2; \
             cat('W = ', signif(s[['W']], 5), ', p = ', signif(s[['p.value']], 4), '\\n', \
             'chi-squared(1) = ', signif(bp, 5), ', p = ', \
             signif(pchisq(bp, 1, lower.tail = FALSE), 4), '\\n', \
             'above 4/n: ', sum(cd > 4 / n), '\\n', 'above 2k/n: ', sum(h > 2 * 2 / n), '\\n', sep = '')",
            pinned::SHAPIRO_BY_HAND
        ),
    );
    for line in hand.lines().filter(|l| !l.trim().is_empty()) {
        assert!(text.contains(line.trim()), "missing {line:?} in\n{text}");
    }

    // robust standard errors: the sandwich by hand, through the intervals
    for (kind, meat) in [
        ("HC3", "r^2 / (1 - h)^2"),
        ("HC1", "r^2 * n / (n - 2)"),
        ("HC0", "r^2"),
    ] {
        kept(&mut e, &ROBUST, "m1", &[("type", kind)], &[]);
        check(
            &mut e,
            &format!(
                "r <- f1$r; h <- diag(X1 %*% solve(crossprod(X1)) %*% t(X1)); \
                 bread <- solve(crossprod(X1)); V <- bread %*% (t(X1) %*% (X1 * ({meat}))) %*% bread; \
                 se <- sqrt(diag(V)); ci <- cbind(f1$b - qt(0.975, n - 2) * se, f1$b + qt(0.975, n - 2) * se)"
            ),
            &[("ci", "all.equal(unname(res), unname(ci), tolerance = 1e-10)")],
        );
    }

    // test terms: each dropped in turn, together, and a likelihood ratio
    kept(&mut e, &TEST_TERMS, "m2", &[], &[]);
    check(
        &mut e,
        "fx <- (ols(X2[, -2], y)$rss - f2$rss) / (f2$rss / (n - 3)); \
         fz <- (ols(X2[, -3], y)$rss - f2$rss) / (f2$rss / (n - 3))",
        &[("f", "all.equal(res$F[2:3], c(fx, fz), tolerance = 1e-12)")],
    );
    kept(
        &mut e,
        &TEST_TERMS,
        "m2",
        &[("kind", "together"), ("terms", "x, z")],
        &[],
    );
    check(
        &mut e,
        "f <- ((tss - f2$rss) / 2) / (f2$rss / (n - 3))",
        &[
            ("f", "all.equal(res$F[2], f, tolerance = 1e-12)"),
            (
                "p",
                "all.equal(res[['Pr(>F)']][2], pf(f, 2, n - 3, lower.tail = FALSE), tolerance = 1e-12)",
            ),
        ],
    );
    kept(&mut e, &TEST_TERMS, "g1", &[], &[]);
    check(
        &mut e,
        "lr <- dev(rep(mean(w), length(w))) - dev(p)",
        &[("lr", "all.equal(res$LRT[2], lr, tolerance = 1e-8)")],
    );

    // the average marginal effect of z: mean(p(1 - p)) b, its error by the
    // delta method on the exact information (marginaleffects takes its own
    // finite differences, about 1e-4 away)
    kept(&mut e, &MARGINAL_EFFECTS, "g1", &[], &[]);
    check(
        &mut e,
        "ame <- function(b) { q <- as.vector(plogis(G %*% b)); mean(q * (1 - q)) * b[2] }; \
         J <- sapply(1:2, function(j) { s <- 1e-6 * c(j == 1, j == 2); \
           (ame(b + s) - ame(b - s)) / 2e-6 }); \
         V <- solve(t(G) %*% (p * (1 - p) * G)); se <- sqrt(as.numeric(J %*% V %*% J))",
        &[
            ("ame", "all.equal(res$estimate, ame(b), tolerance = 1e-6)"),
            ("se", "all.equal(res$std.error, se, tolerance = 1e-3)"),
        ],
    );
}

/// Diagnostic plots: the four panels' points, read back from R's display
/// list, against the linear model worked out by hand: residuals against
/// fitted values, the standardised residuals against their normal scores,
/// the root of their size against fitted values, and against the leverage
/// from the hat matrix.
#[test]
fn diagnostic_plots_draw_the_residuals_worked_out_by_hand() {
    let Some(mut e) = started() else { return };
    let ctx = Context {
        dataset: "m1".into(),
        frames: Vec::new(),
        columns: Vec::new(),
    };
    let values = DIAGNOSTIC_PLOTS.defaults(&ctx);
    let code = DIAGNOSTIC_PLOTS
        .generate(&ctx, &values, Style::Base)
        .unwrap_or_default();
    let (outcome, text) = run(
        &mut e,
        Source::Menu(DIAGNOSTIC_PLOTS.menu.into()),
        &pinned::drawn(&code),
    );
    assert_eq!(outcome, Outcome::Ok, "{code}\n{text}");
    holds(
        &mut e,
        &pinned::all_of(
            &format!(
                "{BY_HAND}; fit <- as.vector(X1 %*% f1$b); r <- f1$r; \
                 h <- diag(X1 %*% solve(crossprod(X1)) %*% t(X1)); s2 <- f1$rss / (n - 2); \
                 sr <- r / sqrt(s2 * (1 - h)); a <- if (n <= 10) 3 / 8 else 1 / 2; \
                 sc <- qnorm((seq_len(n) - a) / (n + 1 - 2 * a))[rank(sr, ties.method = 'first')]"
            ),
            &[
                ("residuals", "has_xy(s, fit, r)"),
                ("qq", "has_xy(s, sc, sr)"),
                ("scale", "has_xy(s, fit, sqrt(abs(sr)))"),
                ("leverage", "has_xy(s, h, sr)"),
            ],
        ),
    );
}

/// Model table (P7.1): a linear and a logistic model side by side, every cell
/// against the table built by hand from the models worked out in plain R
/// (least squares, Newton-Raphson; stars from the p-values computed by hand;
/// observations, R-squared, AIC and BIC from their formulas); then t
/// statistics with two decimals and the table kept under a name.
#[test]
fn the_model_table_is_the_one_built_by_hand() {
    use rcraft_catalog::report::MODEL_TABLE;
    let Some(mut e) = started() else { return };
    kept(
        &mut e,
        &MODEL_TABLE,
        "m1",
        &[("others", "g1")],
        &["stars", "n", "r2", "ic"],
    );
    holds(
        &mut e,
        &pinned::all_of(
            &format!(
                "{BY_HAND}; fmt <- function(x) formatC(x, format = 'f', digits = 3); \
                 st <- function(p) ifelse(p < 0.001, '***', ifelse(p < 0.01, '**', ifelse(p < 0.05, '*', ''))); \
                 s1 <- sqrt(diag(f1$rss / (n - 2) * solve(crossprod(X1)))); \
                 p1 <- 2 * pt(-abs(f1$b / s1), n - 2); \
                 sg <- sqrt(diag(solve(t(G) %*% (p * (1 - p) * G)))); pg <- 2 * pnorm(-abs(b / sg)); \
                 cell <- function(b, s, q) c(paste0(fmt(b), st(q)), paste0('(', fmt(s), ')')); \
                 r2 <- 1 - f1$rss / tss; ll <- llf(f1); dg <- dev(p); \
                 want <- data.frame( \
                   term = c('(Intercept)', '', 'x', '', 'z', '', 'Observations', 'R-squared', \
                     'Adjusted R-squared', 'AIC', 'BIC'), \
                   m1 = c(cell(f1$b[1], s1[1], p1[1]), cell(f1$b[2], s1[2], p1[2]), '', '', '11', \
                     fmt(r2), fmt(1 - (1 - r2) * (n - 1) / (n - 2)), fmt(-2 * ll + 2 * 3), \
                     fmt(-2 * ll + log(n) * 3)), \
                   g1 = c(cell(b[1], sg[1], pg[1]), '', '', cell(b[2], sg[2], pg[2]), '12', '', '', \
                     fmt(dg + 2 * 2), fmt(dg + log(12) * 2)), \
                   check.names = FALSE); names(want)[1] <- ' '"
            ),
            &[
                ("shape", "identical(dim(res), dim(want)) && identical(names(res), names(want))"),
                ("cells", "identical(unname(as.matrix(res)), unname(as.matrix(want)))"),
            ],
        ),
    );

    kept(
        &mut e,
        &MODEL_TABLE,
        "m2",
        &[("paren", "statistic"), ("digits", "2"), ("save", "tab")],
        &["stars"],
    );
    holds(
        &mut e,
        &pinned::all_of(
            &format!(
                "{BY_HAND}; s2 <- sqrt(diag(f2$rss / (n - 3) * solve(crossprod(X2)))); \
                 t2 <- formatC(f2$b / s2, format = 'f', digits = 2)"
            ),
            &[
                ("kept", "identical(tab, res)"),
                ("t", "identical(res$m2[c(2, 4, 6)], paste0('(', t2, ')'))"),
            ],
        ),
    );

    // robust (HC1) standard errors written out as the sandwich, with 95%
    // intervals from them: t for the linear model, normal for the logistic;
    // the stars come from the robust p-values
    kept(
        &mut e,
        &MODEL_TABLE,
        "m1",
        &[("others", "g1"), ("paren", "ci"), ("vcov", "HC1")],
        &["stars"],
    );
    holds(
        &mut e,
        &pinned::all_of(
            &format!(
                "{BY_HAND}; fmt <- function(x) formatC(x, format = 'f', digits = 3); \
                 st <- function(p) ifelse(p < 0.001, '***', ifelse(p < 0.01, '**', ifelse(p < 0.05, '*', ''))); \
                 A1 <- solve(crossprod(X1)); \
                 v1 <- A1 %*% t(X1) %*% (f1$r^2 * X1) %*% A1 * n / (n - 2); s1 <- sqrt(diag(v1)); \
                 q1 <- 2 * pt(-abs(f1$b / s1), n - 2); \
                 AG <- solve(t(G) %*% (p * (1 - p) * G)); \
                 vg <- AG %*% t(G) %*% ((w - p)^2 * G) %*% AG * 12 / (12 - 2); sg <- sqrt(diag(vg)); \
                 qg <- 2 * pnorm(-abs(b / sg)); \
                 ci <- function(b, s, k) paste0('[', fmt(b - k * s), ', ', fmt(b + k * s), ']'); \
                 k1 <- qt(0.975, n - 2); kg <- qnorm(0.975)"
            ),
            &[
                ("names", "identical(names(res), c(' ', 'm1', 'g1'))"),
                (
                    "lm",
                    "identical(res$m1[1:4], c(paste0(fmt(f1$b[1]), st(q1[1])), ci(f1$b[1], s1[1], k1), \
                     paste0(fmt(f1$b[2]), st(q1[2])), ci(f1$b[2], s1[2], k1)))",
                ),
                (
                    "glm",
                    "identical(res$g1[c(1, 2, 5, 6)], c(paste0(fmt(b[1]), st(qg[1])), ci(b[1], sg[1], kg), \
                     paste0(fmt(b[2]), st(qg[2])), ci(b[2], sg[2], kg)))",
                ),
                (
                    "note",
                    "identical(attr(res, 'note'), '95% confidence intervals from robust (HC1) standard errors, \
                     in brackets; * p < 0.05, ** p < 0.01, *** p < 0.001')",
                ),
            ],
        ),
    );

    // also added to the report: the same table as its newest part, with or
    // without a name kept
    run(&mut e, Source::Typed, "rcraft_report <- list()");
    kept(&mut e, &MODEL_TABLE, "m1", &[], &["report"]);
    kept(&mut e, &MODEL_TABLE, "m2", &[("save", "tab2")], &["report"]);
    holds(
        &mut e,
        "length(rcraft_report) == 2, \
         identical(rcraft_report[[2]]$value, res), identical(tab2, res), \
         identical(rcraft_report[[1]]$kind, 'table'), identical(names(rcraft_report[[1]]$value), c(' ', 'm1'))",
    );
}

/// Export table (P7.2): a Model table written as a web page, an RTF document,
/// LaTeX, an Excel workbook and Markdown; each file read back holds every
/// cell, the workbook and the Markdown hold the table exactly, and the
/// Model table's note and the title go with it. A file that is there is not
/// written over unless asked.
#[test]
fn an_exported_table_holds_every_cell() {
    use rcraft_catalog::report::{EXPORT_TABLE, MODEL_TABLE};
    let Some(mut e) = started() else { return };
    for package in ["gt", "openxlsx2", "flextable", "officer"] {
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
    kept(
        &mut e,
        &MODEL_TABLE,
        "m1",
        &[("others", "g1"), ("save", "tab")],
        &["stars", "n", "r2", "ic"],
    );
    let dir = std::env::temp_dir().join(format!("rcraft-tables-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    assert!(std::fs::create_dir_all(&dir).is_ok());
    let file = |name: &str| dir.join(name).to_string_lossy().replace('\\', "/");
    let ctx = Context {
        dataset: "tab".into(),
        frames: Vec::new(),
        columns: Vec::new(),
    };
    let export = |e: &mut Engine, path: &str, replace: bool| {
        let mut values = EXPORT_TABLE.defaults(&ctx);
        values.set_text("file", path);
        values.set_text("title", "Table 1");
        values.set_flag("replace", replace);
        let code = EXPORT_TABLE
            .generate(&ctx, &values, Style::Base)
            .unwrap_or_default();
        run(e, Source::Menu(EXPORT_TABLE.menu.into()), &code)
    };
    for name in ["t.docx", "t.html", "t.rtf", "t.tex", "t.xlsx", "t.md"] {
        let (outcome, text) = export(&mut e, &file(name), false);
        assert_eq!(outcome, Outcome::Ok, "{name}: {text}");
    }
    holds(
        &mut e,
        &pinned::all_of(
            &format!(
                "cells <- unique(c(names(tab), unlist(tab, use.names = FALSE))); cells <- cells[nzchar(cells)]; \
                 has_all <- function(f) {{ x <- paste(readLines(f, warn = FALSE), collapse = '\\n'); \
                   all(vapply(cells, function(c) grepl(c, x, fixed = TRUE), TRUE)) }}; \
                 note <- attr(tab, 'note'); \
                 wb <- openxlsx2::read_xlsx('{xlsx}', start_row = 3, col_names = TRUE); \
                 wb <- wb[seq_len(nrow(tab)), , drop = FALSE]; wb[is.na(wb)] <- ''; \
                 md <- readLines('{md}'); rows <- md[startsWith(md, '| ')][-1]; \
                 split <- t(vapply(rows, function(r) \
                   trimws(strsplit(r, '|', fixed = TRUE)[[1]][seq_len(ncol(tab)) + 1]), character(ncol(tab))))",
                xlsx = file("t.xlsx"),
                md = file("t.md"),
            ),
            &[
                (
                    "docx",
                    &format!(
                        "{{ s <- officer::docx_summary(officer::read_docx('{}'));                          x <- s$text[!is.na(s$text)]; all(cells %in% x) && 'Table 1' %in% x && note %in% x }}",
                        file("t.docx")
                    ),
                ),
                ("html", &format!("has_all('{}')", file("t.html"))),
                ("rtf", &format!("has_all('{}')", file("t.rtf"))),
                ("tex", &format!("has_all('{}')", file("t.tex"))),
                (
                    "html_note",
                    &format!(
                        "any(grepl('in parentheses', readLines('{}', warn = FALSE), fixed = TRUE))",
                        file("t.html")
                    ),
                ),
                (
                    "xlsx",
                    "identical(unname(as.matrix(wb)), unname(as.matrix(tab)))",
                ),
                ("md", "identical(unname(split), unname(as.matrix(trimws(as.matrix(tab)))))"),
                ("md_note", "identical(tail(md, 1), note) && md[1] == 'Table 1'"),
            ],
        ),
    );
    // there now: not written over unless asked
    let (outcome, _) = export(&mut e, &file("t.md"), false);
    assert_eq!(outcome, Outcome::Error);
    let (outcome, text) = export(&mut e, &file("t.md"), true);
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let _ = std::fs::remove_dir_all(&dir);
}
