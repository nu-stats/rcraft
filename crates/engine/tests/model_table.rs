//! Model table, second half (P7.1), end to end against a real R (skipped with
//! a note when none is found, or when a package the models need is missing).
//! The table is built by the dialog's code from the models in the workspace and
//! every cell is checked against the model written out in plain R:
//!
//! - the estimate is `coef()` (`lme4::fixef` for a mixed model, and a
//!   multinomial model's matrix flattened to `outcome:term`), formatted to the
//!   dialog's decimals, and the number below it `sqrt(diag(vcov()))`, for an
//!   `lm`, a `coxph`, a `survreg`, a `polr`, a `multinom`, a `svyglm`, a
//!   `feols`, a `plm` and a `lmer` model in one table;
//! - the statistics are `nobs`, `AIC`, `BIC` and `logLik` where the class has
//!   them and blank where it does not;
//! - the user's own rows go under the statistics, padded with blanks.

use std::time::{Duration, Instant};

use rcraft_catalog::report::MODEL_TABLE;
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

fn define(e: &mut Engine, code: &str) {
    let (outcome, text) = run(e, Source::Typed, code);
    assert_eq!(outcome, Outcome::Ok, "{code}\n{text}");
}

/// Runs R that stops unless every expression holds.
fn holds(e: &mut Engine, exprs: &[&str]) {
    let all = exprs.join(", ");
    let (outcome, text) = run(e, Source::Typed, &format!("stopifnot({all})"));
    assert_eq!(outcome, Outcome::Ok, "{all}\n{text}");
}

/// Sixty rows: a number outcome and two predictors, six groups of ten with a
/// time index, an ordered and an unordered three-level outcome, a survival
/// time and event, and weights.
const MAKE: &str = "d <- local({ i <- 1:60; \
     x <- ((i * 37) %% 23) / 5; z <- ((i * 11) %% 17) / 4; \
     y <- 1 + 0.5 * x - 0.3 * z + sin(i * 1.7); \
     g <- factor(rep(1:6, each = 10)); t <- rep(1:10, 6); \
     ord <- cut(y + 0.4 * cos(i), 3, labels = c('lo', 'mid', 'hi'), ordered_result = TRUE); \
     cls <- cut(y + 0.9 * cos(i * 3), 3, labels = c('a', 'b', 'c')); \
     time <- round(3 + 4 * ((i * 29) %% 31) / 31 + exp(-0.3 * x) * 3, 1); \
     status <- as.numeric((i * 13) %% 7 != 0); w <- 1 + (i %% 4) / 2; \
     data.frame(y, x, z, g, t, ord, cls, time, status, w) })";

fn context(dataset: &str) -> Context {
    Context {
        dataset: dataset.into(),
        frames: Vec::new(),
        columns: Vec::new(),
    }
}

/// Builds the table from the dialog and keeps it as `tab`.
fn table(e: &mut Engine, first: &str, set: &[(&str, &str)], flags: &[&str]) -> (Outcome, String) {
    let spec: &Spec = &MODEL_TABLE;
    let ctx = context(first);
    let mut values = spec.defaults(&ctx);
    values.set_text("save", "tab");
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

/// An lm and a glm, to put rows under.
const TWO_MODELS: &str = "m1 <- lm(y ~ x + z, data = d); \
     g1 <- glm(I(y > 1) ~ x, family = binomial, data = d)";

#[test]
fn custom_rows_go_under_the_statistics() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    define(&mut e, MAKE);
    define(&mut e, TWO_MODELS);
    let rows = "Controls | No | Yes\nCluster | state\n\n  Fixed effects | Yes | Yes  ";
    let (outcome, text) = table(
        &mut e,
        "m1",
        &[("others", "g1"), ("rows", rows)],
        &["stars", "n", "ic"],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    define(
        &mut e,
        "k <- nrow(tab); nm <- c(' ', 'm1', 'g1'); \
         want <- data.frame(c('Controls', 'Cluster', 'Fixed effects'), c('No', 'state', 'Yes'), \
           c('Yes', '', 'Yes'), stringsAsFactors = FALSE); names(want) <- nm",
    );
    holds(
        &mut e,
        &[
            // the statistics block (observations, AIC, BIC) comes first, the
            // three typed rows last, in the order typed, a short row padded
            "identical(tab[[1]][(k - 5):(k - 3)], c('Observations', 'AIC', 'BIC'))",
            "identical(unname(as.matrix(tab[(k - 2):k, ])), unname(as.matrix(want)))",
            "identical(names(tab), nm)",
            // the rows above them are the coefficients and the statistics, as
            // without extra rows
            "nrow(tab) == 2 * 3 + 3 + 3",
            "identical(tab[['m1']][2 * 3 + 1], as.character(nobs(m1)))",
        ],
    );
    // the same table without rows has no row under the statistics
    let (outcome, text) = table(&mut e, "m1", &[("others", "g1")], &["stars", "n", "ic"]);
    assert_eq!(outcome, Outcome::Ok, "{text}");
    holds(
        &mut e,
        &["nrow(tab) == 2 * 3 + 3", "tab[[1]][nrow(tab)] == 'BIC'"],
    );
    // with no statistics the rows follow the coefficients
    let (outcome, text) = table(&mut e, "m1", &[("rows", "Sample | all")], &[]);
    assert_eq!(outcome, Outcome::Ok, "{text}");
    holds(
        &mut e,
        &[
            "tab[[1]][nrow(tab)] == 'Sample'",
            "tab[['m1']][nrow(tab)] == 'all'",
            "nrow(tab) == 2 * 3 + 1",
        ],
    );
}

#[test]
fn other_model_classes_sit_side_by_side() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    for package in [
        "fixest", "plm", "survival", "survey", "lme4", "MASS", "nnet",
    ] {
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
    define(&mut e, MAKE);
    // an lm, a coxph and a feols, as the order has it, then the other classes
    define(
        &mut e,
        "ols <- lm(y ~ x + z, data = d); \
         cox <- survival::coxph(survival::Surv(time, status) ~ x + z, data = d); \
         fe <- fixest::feols(y ~ x + z | g, data = d); \
         aft <- survival::survreg(survival::Surv(time, status) ~ x + z, data = d); \
         ord <- MASS::polr(ord ~ x + z, data = d, Hess = TRUE); \
         mnl <- nnet::multinom(cls ~ x + z, data = d, trace = FALSE); \
         svy <- survey::svyglm(y ~ x + z, survey::svydesign(ids = ~1, weights = ~w, data = d)); \
         pan <- plm::plm(y ~ x + z, data = d, index = c('g', 't'), model = 'within'); \
         mix <- suppressMessages(lme4::lmer(y ~ x + z + (1 | g), data = d))",
    );
    let (outcome, text) = table(
        &mut e,
        "ols",
        &[
            ("others", "cox, fe, aft, ord, mnl, svy, pan, mix"),
            ("digits", "4"),
        ],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    // every cell: the estimate to four decimals, and under it sqrt(diag(vcov))
    // in parentheses, for each class's own names
    define(
        &mut e,
        "fmt <- function(x) formatC(x, format = 'f', digits = 4); \
         terms <- tab[[1]][c(TRUE, FALSE)]; \
         flat <- function(m) { cm <- coef(m); if (!is.matrix(cm)) return(cm); \
           setNames(as.vector(t(cm)), as.vector(t(outer(rownames(cm), colnames(cm), paste, sep = ':')))) }; \
         chk <- function(col, est, se) { est <- est[!is.na(est)]; i <- match(names(est), terms); \
           all(!is.na(i)) && identical(tab[[col]][2 * i - 1], fmt(unname(est))) && \
           identical(tab[[col]][2 * i], paste0('(', fmt(unname(se[names(est)])), ')')) && \
           sum(tab[[col]] != '') == 2 * length(est) }; \
         sd_of <- function(m) sqrt(diag(as.matrix(vcov(m))))",
    );
    holds(
        &mut e,
        &[
            "identical(names(tab), c(' ', 'ols', 'cox', 'fe', 'aft', 'ord', 'mnl', 'svy', 'pan', 'mix'))",
            "chk('ols', coef(ols), sd_of(ols))",
            "chk('cox', coef(cox), sd_of(cox))",
            "chk('fe', coef(fe), sd_of(fe))",
            "chk('aft', coef(aft), sd_of(aft))",
            "chk('ord', coef(ord), sd_of(ord))",
            "chk('mnl', flat(mnl), sd_of(mnl))",
            "chk('svy', coef(svy), sd_of(svy))",
            "chk('pan', coef(pan), sd_of(pan))",
            "chk('mix', lme4::fixef(mix), sd_of(mix))",
            // a term in some models only leaves the others blank: the
            // intercept is in the lm but not the Cox, fixed-effects or panel
            // models, and the multinomial outcomes have their own rows
            "tab[['cox']][match('(Intercept)', terms) * 2 - 1] == ''",
            "all(c('b:x', 'c:z') %in% terms)",
        ],
    );
    // the statistics: nobs, AIC, BIC and logLik where the class has them,
    // blank where it does not (a survey model has no likelihood)
    let (outcome, text) = table(
        &mut e,
        "ols",
        &[("others", "cox, fe, svy, mix")],
        &["n", "ic", "ll"],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    define(
        &mut e,
        "fmt <- function(x) formatC(x, format = 'f', digits = 3); \
         stat <- function(name, col) tab[[col]][match(name, tab[[1]])]",
    );
    holds(
        &mut e,
        &[
            "identical(stat('Observations', 'ols'), '60')",
            // a Cox model's rows are the rows used, not its events
            "identical(stat('Observations', 'cox'), as.character(cox$n))",
            "identical(stat('Observations', 'fe'), as.character(nobs(fe)))",
            "identical(stat('Observations', 'svy'), '60')",
            "identical(stat('Observations', 'mix'), as.character(nobs(mix)))",
            "identical(stat('AIC', 'ols'), fmt(AIC(ols)))",
            "identical(stat('AIC', 'cox'), fmt(AIC(cox)))",
            "identical(stat('BIC', 'fe'), fmt(BIC(fe)))",
            "identical(stat('Log-likelihood', 'mix'), fmt(as.numeric(logLik(mix))))",
            "identical(stat('AIC', 'svy'), '') && identical(stat('Log-likelihood', 'svy'), '')",
        ],
    );
    // a class the table does not know is refused in R's words, naming it
    define(&mut e, "bad <- ts(1:5)");
    let spec: &Spec = &MODEL_TABLE;
    let ctx = context("ols");
    let mut values = spec.defaults(&ctx);
    values.set_text("others", "bad");
    let code = spec
        .generate(&ctx, &values, Style::Base)
        .unwrap_or_default();
    let (outcome, text) = run(&mut e, Source::Menu(spec.menu.into()), &code);
    assert_eq!(outcome, Outcome::Error, "{text}");
    assert!(text.contains("not: bad"), "{text}");
}

/// The models of `other_model_classes_sit_side_by_side`.
const ALL_MODELS: &str = "ols <- lm(y ~ x + z, data = d); \
     cox <- survival::coxph(survival::Surv(time, status) ~ x + z, data = d); \
     fe <- fixest::feols(y ~ x + z | g, data = d); \
     aft <- survival::survreg(survival::Surv(time, status) ~ x + z, data = d); \
     ord <- MASS::polr(ord ~ x + z, data = d, Hess = TRUE); \
     mnl <- nnet::multinom(cls ~ x + z, data = d, trace = FALSE); \
     svy <- survey::svyglm(y ~ x + z, survey::svydesign(ids = ~1, weights = ~w, data = d)); \
     pan <- plm::plm(y ~ x + z, data = d, index = c('g', 't'), model = 'within'); \
     mix <- suppressMessages(lme4::lmer(y ~ x + z + (1 | g), data = d))";

/// Whether the packages the models need are there; a note when not.
fn packages(e: &mut Engine) -> bool {
    for package in [
        "fixest", "plm", "survival", "survey", "lme4", "MASS", "nnet", "sandwich",
    ] {
        let (outcome, _) = run(
            e,
            Source::Typed,
            &format!("stopifnot(requireNamespace('{package}', quietly = TRUE))"),
        );
        if outcome != Outcome::Ok {
            eprintln!("skipped: the R package {package} is not installed");
            return false;
        }
    }
    true
}

#[test]
fn robust_errors_for_every_class_that_has_them() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !packages(&mut e) {
        return;
    }
    define(&mut e, MAKE);
    define(&mut e, ALL_MODELS);
    let (outcome, text) = table(
        &mut e,
        "ols",
        &[
            ("others", "cox, fe, aft, ord, mnl, pan"),
            ("digits", "5"),
            ("vcov", "HC1"),
        ],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    define(
        &mut e,
        "fmt <- function(x) formatC(x, format = 'f', digits = 5); \
         terms <- tab[[1]][c(TRUE, FALSE)]; \
         se_is <- function(col, se) { i <- match(names(se), terms); \
           all(!is.na(i)) && identical(tab[[col]][2 * i], paste0('(', fmt(unname(se)), ')')) }; \
         X <- cbind(1, d$x, d$z); n <- nrow(X); r <- resid(ols); A <- solve(crossprod(X)); \
         hc1 <- sqrt(diag(A %*% crossprod(X * r) %*% A * n / (n - 3))); names(hc1) <- names(coef(ols)); \
         dm <- function(v) v - ave(v, d$g); W <- cbind(dm(d$x), dm(d$z)); u <- resid(pan); \
         B <- solve(crossprod(W)); \
         meat <- Reduce(`+`, lapply(split(seq_len(n), d$g), function(k) \
           tcrossprod(colSums(W[k, , drop = FALSE] * u[k])))); \
         pan_se <- sqrt(diag(B %*% meat %*% B * n / (n - 2))); names(pan_se) <- c('x', 'z'); \
         dfb <- function(m) { D <- residuals(m, type = 'dfbeta'); sqrt(diag(crossprod(D))) }",
    );
    holds(
        &mut e,
        &[
            // lm: the HC1 sandwich written out
            "se_is('ols', hc1)",
            // plm: Arellano's sandwich, clustered on the unit, with n / (n - k)
            "se_is('pan', pan_se)",
            // Cox and survreg: the sum of squared dfbeta residuals (the robust
            // variance a refit with robust = TRUE gives)
            "se_is('cox', setNames(dfb(cox), names(coef(cox))))",
            "se_is('aft', setNames(dfb(aft)[seq_along(coef(aft))], names(coef(aft))))",
            // fixest: its own heteroskedasticity-robust errors
            "se_is('fe', fixest::se(fe, vcov = 'hetero'))",
            // polr and multinom: the sandwich estimator
            "se_is('ord', sqrt(diag(sandwich::sandwich(ord)))[names(coef(ord))])",
            // multinom: each row's score (indicator less probability) times
            // its predictors, for every outcome after the first, sandwiched
            // by the model's covariance
            "se_is('mnl', local({ Xm <- cbind(1, d$x, d$z); Pm <- fitted(mnl); \
               Ym <- outer(as.character(d$cls), mnl$lev, '==') * 1; \
               S <- cbind((Ym[, 2] - Pm[, 2]) * Xm, (Ym[, 3] - Pm[, 3]) * Xm); \
               V <- vcov(mnl); setNames(sqrt(diag(V %*% crossprod(S) %*% V)), colnames(V)) }))",
            // the note says which errors
            "grepl('Robust (HC1)', attr(tab, 'note'), fixed = TRUE)",
        ],
    );
    // HC3 where fixest has none, a survey model and a mixed model: refused
    // with a message, not a crash
    for (others, vcov, words) in [
        ("fe", "HC3", "fixest has the HC1 robust errors only"),
        ("svy", "HC1", "already follow the design"),
        ("mix", "HC1", "not offered for a mixed model"),
    ] {
        let (outcome, text) = table(&mut e, "ols", &[("others", others), ("vcov", vcov)], &[]);
        assert_eq!(outcome, Outcome::Error, "{text}");
        assert!(text.contains(words), "{text}");
    }
}

#[test]
fn r_squared_for_fixest_and_plm_and_labelled_coefficients() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !packages(&mut e) {
        return;
    }
    define(&mut e, MAKE);
    define(&mut e, ALL_MODELS);
    let (outcome, text) = table(
        &mut e,
        "ols",
        &[
            ("others", "fe, pan, cox"),
            ("coefs", "z | Hours\n(Intercept) | Constant\nx"),
        ],
        &["r2"],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    define(
        &mut e,
        "fmt <- function(x) formatC(x, format = 'f', digits = 3); \
         stat <- function(name, col) tab[[col]][match(name, tab[[1]])]; \
         dm <- function(v) v - ave(v, d$g); \
         r2_fe <- 1 - sum(resid(fe)^2) / sum((d$y - mean(d$y))^2); \
         r2_pan <- 1 - sum(resid(pan)^2) / sum(dm(d$y)^2); n <- 60",
    );
    holds(
        &mut e,
        &[
            // R-squared by hand: fixest's is the whole model's (fixed effects
            // included), plm's within one is on the demeaned outcome
            "identical(stat('R-squared', 'fe'), fmt(r2_fe))",
            "identical(stat('R-squared', 'pan'), fmt(r2_pan))",
            // plm's adjustment: (n - 1) over the residual degrees of freedom
            // (rows less units less coefficients)
            "identical(stat('Adjusted R-squared', 'pan'), fmt(1 - (1 - r2_pan) * (n - 1) / (n - 6 - 2)))",
            "identical(stat('R-squared', 'ols'), fmt(summary(ols)$r.squared))",
            "identical(stat('R-squared', 'cox'), '')",
            // the listed coefficients first, in the order typed, under their
            // labels (x keeps its name), each cell still its own model's
            "identical(tab[[1]][c(1, 3, 5)], c('Hours', 'Constant', 'x'))",
            "identical(tab[['ols']][c(1, 3, 5)], fmt(unname(coef(ols)[c('z', '(Intercept)', 'x')])))",
            "identical(tab[['fe']][1], fmt(unname(coef(fe)['z'])))",
        ],
    );
    // a coefficient listed twice is refused before anything runs
    let spec: &Spec = &MODEL_TABLE;
    let ctx = context("ols");
    let mut values = spec.defaults(&ctx);
    values.set_text("coefs", "x\nx | Again");
    let problems = spec.problems(&ctx, &values);
    assert!(problems.iter().any(|p| p.widget == "coefs"), "{problems:?}");
}

#[test]
fn mixed_and_panel_models_reach_the_model_table() {
    use rcraft_catalog::postest::{self, MODEL_FIT};
    for class in [
        "lmerMod", "glmerMod", "fixest", "plm", "coxph", "survreg", "svyglm",
    ] {
        assert!(postest::takes_model_table(class), "{class}");
    }
    assert!(!postest::takes_model_table("pgmm"));
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !packages(&mut e) {
        return;
    }
    define(&mut e, MAKE);
    define(&mut e, ALL_MODELS);
    // Model fit on a mixed model: rows, log-likelihood, AIC and BIC
    let ctx = context("mix");
    let values = MODEL_FIT.defaults(&ctx);
    let code = MODEL_FIT
        .generate(&ctx, &values, Style::Base)
        .unwrap_or_default();
    let (outcome, text) = run(&mut e, Source::Menu(MODEL_FIT.menu.into()), &code);
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(
        text.contains("Observations: 60") && text.contains("AIC"),
        "{text}"
    );
}
