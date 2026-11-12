//! Statistics menu panel and fixed-effects models (P6.1) end to end against a
//! real R (skipped with a note when none is found, or when fixest or plm is
//! not installed). Each dialog's code is built from its spec and run on a
//! made-up balanced panel; its coefficients must equal an independent fit
//! (the fixed effects as dummy variables in lm or glm, a between model as lm
//! on the group means, the Hausman statistic worked out from the two fits),
//! and what it prints must equal the package's own report for the call
//! written out by hand.

use std::time::{Duration, Instant};

use rcraft_catalog::panel::{FIXED_EFFECTS_GLM, FIXED_EFFECTS_REGRESSION, HAUSMAN, PANEL_MODEL};
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
    let limit = Instant::now() + Duration::from_secs(120);
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

/// Whether R has the packages these dialogs need.
fn has_packages(e: &mut Engine) -> bool {
    let (outcome, text) = run(
        e,
        Source::Typed,
        "cat(all(vapply(c('fixest', 'plm'), requireNamespace, TRUE, quietly = TRUE)))",
    );
    let ok = outcome == Outcome::Ok && text.trim() == "TRUE";
    if !ok {
        eprintln!("skipped: fixest or plm is not installed");
    }
    ok
}

/// Six units over five years; the units' effects move with hours, so fixed
/// and random effects differ.
const MAKE: &str = "d <- local({ \
     id <- rep(1:6, each = 5); year <- rep(2001:2005, times = 6); \
     a <- c(0.5, -1, 2, 0.3, -0.7, 1.2)[id]; \
     hours <- round(10 + 2 * a + 3 * sin(1:30), 3); \
     wage <- round(1 + 0.4 * hours + a + 0.2 * (year - 2003) + cos(1:30), 3); \
     firm <- c('p', 'q', 'r')[1 + (1:30 %% 3)]; \
     visits <- round(exp(0.5 + 0.08 * hours + 0.3 * a)) + (1:30 * 7) %% 4; \
     days <- rep(c(10, 20, 30), 10); \
     left <- as.numeric(((1:30 * 7) %% 5) < 2); \
     data.frame(id, year, hours, wage, firm, visits, days, left) })";

fn context() -> Context {
    Context {
        dataset: "d".into(),
        frames: Vec::new(),
        columns: [
            ("id", VarKind::Number),
            ("year", VarKind::Number),
            ("hours", VarKind::Number),
            ("wage", VarKind::Number),
            ("firm", VarKind::Text),
            ("visits", VarKind::Number),
            ("days", VarKind::Number),
            ("left", VarKind::Number),
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
    let (made, text) = run(e, Source::Typed, MAKE);
    assert_eq!(made, Outcome::Ok, "{text}");
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
    run(e, Source::Menu(spec.menu.into()), &code)
}

/// A report with spaces tidied, blank lines and the lines that echo the call
/// left out.
fn report(text: &str) -> Vec<String> {
    text.lines()
        .filter(|l| {
            let l = l.trim_start();
            !(l.starts_with("Call:")
                || l.starts_with("plm::plm(")
                || l.starts_with("plm(")
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
fn fixed_effects_regression_is_the_dummy_variable_fit() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !has_packages(&mut e) {
        return;
    }
    let (outcome, text) = dialog(
        &mut e,
        &FIXED_EFFECTS_REGRESSION,
        &[("y", "wage")],
        &[("x", &["hours"]), ("fe", &["id", "year"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let (_, expected) = run(
        &mut e,
        Source::Typed,
        "m <- fixest::feols(wage ~ hours | id + year, data = d, vcov = 'cluster'); \
         print(summary(m)); print(stats::confint(m, level = 0.95))",
    );
    assert!(report(&expected).len() > 6, "{expected}");
    assert_eq!(report(&text), report(&expected), "{text}");
    // the within estimate is the coefficient of hours with a dummy for every
    // unit and year; clustered by unit, as fixest's default
    holds(
        &mut e,
        "inherits(model, 'fixest'), \
         isTRUE(all.equal(unname(coef(model)['hours']), \
           unname(coef(lm(wage ~ hours + factor(id) + factor(year), data = d))['hours']), \
           tolerance = 1e-10)), \
         identical(attr(model$cov.scaled, 'type'), 'Clustered (id)')",
    );

    // conventional errors: the dummy-variable fit's own standard error
    let (outcome, text) = dialog(
        &mut e,
        &FIXED_EFFECTS_REGRESSION,
        &[("y", "wage"), ("errors", "iid"), ("save", "fe1")],
        &[("x", &["hours"]), ("fe", &["id"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    holds(
        &mut e,
        "isTRUE(all.equal(unname(fixest::se(fe1)['hours']), \
           unname(summary(lm(wage ~ hours + factor(id), data = d))$coefficients['hours', 2]), \
           tolerance = 1e-10))",
    );

    // a cluster variable
    let (outcome, text) = dialog(
        &mut e,
        &FIXED_EFFECTS_REGRESSION,
        &[("y", "wage"), ("cluster", "firm"), ("save", "fe2")],
        &[("x", &["hours"]), ("fe", &["id"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    holds(
        &mut e,
        "identical(attr(fe2$cov.scaled, 'type'), 'Clustered (firm)')",
    );
}

#[test]
fn fixed_effects_glm_is_the_dummy_variable_glm() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !has_packages(&mut e) {
        return;
    }
    let (outcome, text) = dialog(
        &mut e,
        &FIXED_EFFECTS_GLM,
        &[("y", "visits"), ("exposure", "days")],
        &[("x", &["hours"]), ("fe", &["id"])],
        &["ratios"],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let (_, expected) = run(
        &mut e,
        Source::Typed,
        "m <- fixest::fepois(visits ~ hours | id, data = d, offset = ~log(days), vcov = 'cluster'); \
         print(summary(m)); cat('\\nRate ratios\\n'); \
         print(exp(cbind(rate_ratio = stats::coef(m), stats::confint(m, level = 0.95))))",
    );
    assert_eq!(report(&text), report(&expected), "{text}");
    holds(
        &mut e,
        "isTRUE(all.equal(unname(coef(model)['hours']), \
           unname(coef(glm(visits ~ hours + factor(id) + offset(log(days)), family = poisson, \
             data = d))['hours']), tolerance = 1e-7))",
    );

    // logit: the units whose outcome never varies are dropped; the rest give
    // the dummy-variable logit's estimate
    let (outcome, text) = dialog(
        &mut e,
        &FIXED_EFFECTS_GLM,
        &[
            ("y", "left"),
            ("family", "logit"),
            ("errors", "iid"),
            ("save", "lg"),
        ],
        &[("x", &["hours"]), ("fe", &["firm"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    holds(
        &mut e,
        "isTRUE(all.equal(unname(coef(lg)['hours']), \
           unname(coef(glm(left ~ hours + factor(firm), family = binomial, data = d))['hours']), \
           tolerance = 1e-6))",
    );
}

#[test]
fn panel_model_gives_the_estimators_it_names() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !has_packages(&mut e) {
        return;
    }
    let (outcome, text) = dialog(
        &mut e,
        &PANEL_MODEL,
        &[("y", "wage"), ("id", "id"), ("time", "year")],
        &[("x", &["hours"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let (_, expected) = run(
        &mut e,
        Source::Typed,
        "m <- plm::plm(wage ~ hours, data = d, index = c('id', 'year'), model = 'within', \
         effect = 'individual'); print(summary(m))",
    );
    assert!(report(&expected).len() > 8, "{expected}");
    assert_eq!(report(&text), report(&expected), "{text}");
    // within is the unit-dummy fit
    holds(
        &mut e,
        "inherits(model, 'plm'), \
         isTRUE(all.equal(unname(coef(model)['hours']), \
           unname(coef(lm(wage ~ hours + factor(id), data = d))['hours']), tolerance = 1e-10))",
    );

    // between: lm on the units' means
    let (outcome, text) = dialog(
        &mut e,
        &PANEL_MODEL,
        &[
            ("y", "wage"),
            ("id", "id"),
            ("time", "year"),
            ("model", "between"),
            ("save", "bm"),
        ],
        &[("x", &["hours"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    holds(
        &mut e,
        "isTRUE(all.equal(unname(coef(bm)), \
           unname(coef(lm(wage ~ hours, data = aggregate(cbind(wage, hours) ~ id, d, mean)))), \
           tolerance = 1e-10))",
    );

    // two-way random effects with errors clustered by unit: the summary uses
    // plm::vcovHC's standard errors
    let (outcome, text) = dialog(
        &mut e,
        &PANEL_MODEL,
        &[
            ("y", "wage"),
            ("id", "id"),
            ("time", "year"),
            ("model", "random"),
            ("effect", "twoways"),
            ("errors", "group"),
            ("save", "re"),
        ],
        &[("x", &["hours"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    holds(
        &mut e,
        "isTRUE(all.equal(summary(re, vcov = plm::vcovHC(re, cluster = 'group'))$coefficients[, 2], \
           sqrt(diag(plm::vcovHC(re, cluster = 'group'))), tolerance = 1e-12)), \
         identical(re$args$effect, 'twoways')",
    );

    // first differences
    let (outcome, text) = dialog(
        &mut e,
        &PANEL_MODEL,
        &[
            ("y", "wage"),
            ("id", "id"),
            ("time", "year"),
            ("model", "fd"),
            ("save", "fdm"),
        ],
        &[("x", &["hours"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    holds(
        &mut e,
        "isTRUE(all.equal(unname(coef(fdm)['hours']), \
           unname(coef(lm(dw ~ dh, data = local({ \
             o <- d[order(d$id, d$year), ]; \
             data.frame(dw = unlist(tapply(o$wage, o$id, diff)), \
                        dh = unlist(tapply(o$hours, o$id, diff))) })))['dh']), tolerance = 1e-10))",
    );
}

#[test]
fn hausman_is_the_statistic_worked_out_by_hand() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !has_packages(&mut e) {
        return;
    }
    let (outcome, text) = dialog(
        &mut e,
        &HAUSMAN,
        &[("y", "wage"), ("id", "id"), ("time", "year")],
        &[("x", &["hours"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(text.contains("Hausman Test"), "{text}");
    // (b_fe - b_re)' (V_fe - V_re)^-1 (b_fe - b_re) on hours, from the printed chisq
    let (outcome, text) = run(
        &mut e,
        Source::Typed,
        "local({ \
           fe <- plm::plm(wage ~ hours, data = d, index = c('id', 'year'), model = 'within'); \
           re <- plm::plm(wage ~ hours, data = d, index = c('id', 'year'), model = 'random'); \
           b <- coef(fe)['hours'] - coef(re)['hours']; \
           v <- vcov(fe)['hours', 'hours'] - vcov(re)['hours', 'hours']; \
           cat(format(unname(b^2 / v), digits = 15)) })",
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let by_hand: f64 = text.trim().parse().unwrap_or(f64::NAN);
    let (_, printed) = dialog(
        &mut e,
        &HAUSMAN,
        &[("y", "wage"), ("id", "id"), ("time", "year")],
        &[("x", &["hours"])],
        &[],
    );
    let chisq = printed
        .split("chisq = ")
        .nth(1)
        .and_then(|s| s.split(',').next())
        .and_then(|s| s.trim().parse::<f64>().ok())
        .unwrap_or(f64::NAN);
    assert!(
        (chisq - by_hand).abs() <= 1e-3 * by_hand.abs(),
        "printed {chisq}, by hand {by_hand}\n{printed}"
    );
}
