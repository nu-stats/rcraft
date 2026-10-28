//! Statistics menu logistic and Poisson models (P4.4) end to end against a real
//! R (skipped with a note when none is found). Each dialog's code is built from
//! its spec and run; the model it saves must equal the model R fits for the call
//! written out here by hand, and what it prints must equal R's own report.

use std::time::{Duration, Instant};

use rcraft_catalog::generalised::{LOGISTIC, MULTINOMIAL, NEGATIVE_BINOMIAL, ORDINAL, POISSON};
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

const MAKE: &str = "d <- data.frame(\
     x = 1:12, \
     z = c(2.1, 1.9, 3.2, 2.8, 3.9, 3.1, 4.5, 4.0, 5.2, 4.8, 6.1, 5.5), \
     won = c(0, 0, 0, 1, 0, 0, 1, 0, 1, 1, 1, 1), \
     three = factor(rep(c('a', 'b', 'c'), 4)), \
     n = c(1, 0, 2, 3, 2, 4, 5, 3, 6, 8, 7, 9), \
     nb = c(0, 1, 0, 3, 2, 9, 1, 0, 12, 4, 2, 15), \
     sat = factor(c('low', 'low', 'mid', 'low', 'mid', 'low', 'mid', 'high', 'mid', 'high', \
                    'high', 'high'), levels = c('low', 'mid', 'high')), \
     e = rep(c(10, 20), 6), \
     grp = factor(rep(c('u', 'v'), each = 6)))";

fn context() -> Context {
    Context {
        dataset: "d".into(),
        columns: [
            ("x", VarKind::Number),
            ("z", VarKind::Number),
            ("won", VarKind::Number),
            ("three", VarKind::Factor),
            ("n", VarKind::Number),
            ("nb", VarKind::Number),
            ("sat", VarKind::Factor),
            ("e", VarKind::Number),
            ("grp", VarKind::Factor),
        ]
        .into_iter()
        .map(|(name, kind)| ColumnInfo {
            name: name.into(),
            kind,
        })
        .collect(),
    }
}

/// What the dialog's code does with the made-up data in R.
fn dialog(
    e: &mut Engine,
    spec: &Spec,
    set: &[(&str, &str)],
    lists: &[(&str, &[&str])],
    flags: &[&str],
) -> (Outcome, String) {
    let (made, _) = run(e, Source::Typed, MAKE);
    assert_eq!(made, Outcome::Ok);
    let ctx = context();
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
    run(e, Source::Menu(spec.menu.into()), &code)
}

/// A report with spaces tidied, blank lines and the lines that echo the call
/// (which name the model) left out.
fn report(text: &str) -> Vec<String> {
    text.lines()
        .filter(|l| {
            let l = l.trim_start();
            !(l.starts_with("Call:")
                || l.starts_with("stats::")
                || l.starts_with("glm(")
                || l.starts_with("MASS::glm.nb(")
                || l.starts_with("glm.nb(")
                || l.starts_with("MASS::polr(")
                || l.starts_with("polr(")
                || l.starts_with("nnet::multinom(")
                || l.starts_with("multinom(")
                || l.starts_with("warning:"))
        })
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|l| !l.is_empty())
        .collect()
}

/// Runs R that stops unless every one of `exprs` holds.
fn holds(e: &mut Engine, exprs: &str) {
    let (outcome, text) = run(e, Source::Typed, &format!("stopifnot({exprs})"));
    assert_eq!(outcome, Outcome::Ok, "{exprs}\n{text}");
}

#[test]
fn logistic_model_is_the_glm_r_fits() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);

    let (outcome, text) = dialog(
        &mut e,
        &LOGISTIC,
        &[("y", "won")],
        &[("x", &["x", "z"])],
        &["ratios"],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let (_, expected) = run(
        &mut e,
        Source::Typed,
        "m <- stats::glm(won ~ x + z, family = stats::binomial(link = 'logit'), data = d); \
         print(summary(m)); cat('\\nOdds ratios\\n'); \
         print(exp(cbind(odds_ratio = stats::coef(m), stats::confint(m, level = 0.95))))",
    );
    assert!(report(&expected).len() > 12, "{expected}");
    assert_eq!(report(&text), report(&expected), "{text}");
    holds(
        &mut e,
        "inherits(model, 'glm'), identical(model$family$link, 'logit'), \
         isTRUE(all.equal(coef(model), coef(glm(won ~ x + z, family = binomial, data = d))))",
    );

    // another link, no odds ratios, a categorical predictor, another name
    let (outcome, text) = dialog(
        &mut e,
        &LOGISTIC,
        &[("y", "won"), ("link", "probit"), ("save", "pm")],
        &[("x", &["x", "grp"])],
        &["ratios"],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(
        !text.contains("Odds ratios"),
        "odds ratios are for the logit link: {text}"
    );
    holds(
        &mut e,
        "identical(pm$family$link, 'probit'), \
         isTRUE(all.equal(coef(pm), coef(glm(won ~ x + grp, family = binomial('probit'), data = d))))",
    );

    // a factor outcome with exactly two values is fine, with three it is refused
    // by the dialog's own check before the model is fitted
    let (outcome, text) = dialog(&mut e, &LOGISTIC, &[("y", "grp")], &[("x", &["x"])], &[]);
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let (outcome, text) = dialog(
        &mut e,
        &LOGISTIC,
        &[("y", "three"), ("save", "bad")],
        &[("x", &["x"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Error, "{text}");
    assert!(text.contains("exactly two different values"), "{text}");
    holds(&mut e, "!exists('bad')");
}

#[test]
fn poisson_model_is_the_glm_r_fits() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);

    let (outcome, text) = dialog(
        &mut e,
        &POISSON,
        &[("y", "n")],
        &[("x", &["x"])],
        &["ratios"],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let (_, expected) = run(
        &mut e,
        Source::Typed,
        "m <- stats::glm(n ~ x, family = stats::poisson(), data = d); print(summary(m)); \
         cat('\\nRate ratios\\n'); \
         print(exp(cbind(rate_ratio = stats::coef(m), stats::confint(m, level = 0.95))))",
    );
    assert_eq!(report(&text), report(&expected), "{text}");

    // an exposure becomes an offset, and so changes the coefficients
    let (outcome, text) = dialog(
        &mut e,
        &POISSON,
        &[
            ("y", "n"),
            ("exposure", "e"),
            ("family", "quasipoisson"),
            ("save", "rate"),
        ],
        &[("x", &["x", "grp"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    holds(
        &mut e,
        "inherits(rate, 'glm'), identical(rate$family$family, 'quasipoisson'), \
         isTRUE(all.equal(coef(rate), coef(glm(n ~ x + grp, family = quasipoisson, data = d, offset = log(e))))), \
         !isTRUE(isTRUE(all.equal(coef(rate), coef(glm(n ~ x + grp, family = quasipoisson, data = d)))))",
    );
}

#[test]
fn negative_binomial_model_is_the_glm_nb_r_fits() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    let (have, _) = run(
        &mut e,
        Source::Typed,
        "stopifnot(requireNamespace('MASS', quietly = TRUE))",
    );
    if have != Outcome::Ok {
        eprintln!("skipped: the R package MASS is not installed");
        return;
    }

    let (outcome, text) = dialog(
        &mut e,
        &NEGATIVE_BINOMIAL,
        &[("y", "nb")],
        &[("x", &["x"])],
        &["ratios"],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let (_, expected) = run(
        &mut e,
        Source::Typed,
        "m <- MASS::glm.nb(nb ~ x, data = d); print(summary(m)); cat('\nRate ratios\n'); \
         print(exp(cbind(rate_ratio = stats::coef(m), stats::confint(m, level = 0.95))))",
    );
    assert!(report(&expected).len() > 10, "{expected}");
    assert_eq!(report(&text), report(&expected), "{text}");
    holds(
        &mut e,
        "inherits(model, 'negbin'), is.finite(model$theta), \
         isTRUE(all.equal(coef(model), coef(MASS::glm.nb(nb ~ x, data = d))))",
    );

    // an exposure becomes an offset, a categorical predictor another name
    let (outcome, text) = dialog(
        &mut e,
        &NEGATIVE_BINOMIAL,
        &[("y", "nb"), ("exposure", "e"), ("save", "nbrate")],
        &[("x", &["x", "grp"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    holds(
        &mut e,
        "inherits(nbrate, 'negbin'), \
         isTRUE(all.equal(coef(nbrate), coef(MASS::glm.nb(nb ~ x + grp + offset(log(e)), data = d)))), \
         !isTRUE(isTRUE(all.equal(coef(nbrate), coef(MASS::glm.nb(nb ~ x + grp, data = d)))))",
    );
}

/// Whether R can load `package`; says so and returns false when it cannot.
fn have_package(e: &mut Engine, package: &str) -> bool {
    let (outcome, _) = run(
        e,
        Source::Typed,
        &format!("stopifnot(requireNamespace('{package}', quietly = TRUE))"),
    );
    if outcome != Outcome::Ok {
        eprintln!("skipped: the R package {package} is not installed");
    }
    outcome == Outcome::Ok
}

#[test]
fn ordinal_model_is_the_polr_r_fits() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !have_package(&mut e, "MASS") {
        return;
    }

    let (outcome, text) = dialog(
        &mut e,
        &ORDINAL,
        &[("y", "sat")],
        &[("x", &["x"])],
        &["ratios", "pvalues"],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let (_, expected) = run(
        &mut e,
        Source::Typed,
        "m <- MASS::polr(ordered(sat) ~ x, data = d, method = 'logistic', Hess = TRUE); \
         cat('Outcome order, lowest first: ', paste(levels(ordered(d$sat)), collapse = ' < '), '\n', sep = ''); \
         print(summary(m)); cat('\nCoefficients with p-values\n'); \
         ct <- coef(summary(m)); print(cbind(ct, 'p value' = 2 * pnorm(abs(ct[, 't value']), lower.tail = FALSE))); \
         cat('\nOdds ratios\n'); print(exp(cbind(odds_ratio = coef(m), confint(m, level = 0.95))))",
    );
    assert!(report(&expected).len() > 12, "{expected}");
    assert!(
        text.contains("low < mid < high"),
        "the order used is printed: {text}"
    );
    assert_eq!(report(&text), report(&expected), "{text}");
    holds(
        &mut e,
        "inherits(model, 'polr'), \
         isTRUE(all.equal(coef(model), coef(MASS::polr(ordered(sat) ~ x, data = d, Hess = TRUE))))",
    );

    // a different link and a categorical predictor give no odds ratios
    let (outcome, text) = dialog(
        &mut e,
        &ORDINAL,
        &[("y", "sat"), ("link", "probit"), ("save", "pr")],
        &[("x", &["x", "grp"])],
        &["ratios"],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(!text.contains("Odds ratios"), "{text}");
    holds(
        &mut e,
        "identical(pr$method, 'probit'), \
         isTRUE(all.equal(coef(pr), coef(MASS::polr(ordered(sat) ~ x + grp, data = d, method = 'probit', Hess = TRUE))))",
    );
}

#[test]
fn multinomial_model_is_the_multinom_r_fits() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !have_package(&mut e, "nnet") {
        return;
    }

    let (outcome, text) = dialog(
        &mut e,
        &MULTINOMIAL,
        &[("y", "three")],
        &[("x", &["x", "z"])],
        &["ratios", "pvalues"],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let (_, expected) = run(
        &mut e,
        Source::Typed,
        "m <- nnet::multinom(factor(three) ~ x + z, data = d, trace = FALSE); print(summary(m)); \
         cat('\np-values\n'); \
         print(2 * pnorm(abs(summary(m)$coefficients / summary(m)$standard.errors), lower.tail = FALSE)); \
         cat('\nRelative risk ratios\n'); print(exp(coef(m))); \
         cat('\nConfidence intervals for the ratios\n'); print(exp(confint(m, level = 0.95)))",
    );
    assert!(report(&expected).len() > 12, "{expected}");
    assert_eq!(report(&text), report(&expected), "{text}");
    holds(
        &mut e,
        "inherits(model, 'multinom'), identical(rownames(coef(model)), c('b', 'c')), \
         isTRUE(all.equal(coef(model), coef(nnet::multinom(factor(three) ~ x + z, data = d, trace = FALSE))))",
    );

    // another reference category changes the rows and the coefficients
    let (outcome, text) = dialog(
        &mut e,
        &MULTINOMIAL,
        &[("y", "three"), ("reference", "c"), ("save", "mref")],
        &[("x", &["x"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    holds(
        &mut e,
        "identical(rownames(coef(mref)), c('a', 'b')), \
         isTRUE(all.equal(coef(mref), coef(nnet::multinom(relevel(factor(three), ref = 'c') ~ x, data = d, trace = FALSE))))",
    );

    // a reference that is not a category is refused by R; two categories are refused first
    let (outcome, text) = dialog(
        &mut e,
        &MULTINOMIAL,
        &[("y", "three"), ("reference", "zzz"), ("save", "bad")],
        &[("x", &["x"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Error, "{text}");
    let (outcome, text) = dialog(
        &mut e,
        &MULTINOMIAL,
        &[("y", "grp"), ("save", "two")],
        &[("x", &["x"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Error, "{text}");
    assert!(text.contains("at least three"), "{text}");
}
