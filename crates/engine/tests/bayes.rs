//! Statistics > Bayesian regression (P6.9) end to end against a real R (skipped
//! with a note when none is found, or when rstanarm is not installed). The
//! dialog's code is built from its spec and run on made-up data, and the
//! posterior is checked two ways:
//!
//! - against an exact answer worked out here in plain R. For a Gaussian model
//!   with normal priors on the intercept and the coefficients, the posterior of
//!   the coefficients given the error scale sigma is normal with a known mean and
//!   covariance, and the data's marginal density given sigma is normal too, so the
//!   posterior mean of every coefficient is a one-dimensional integral over sigma
//!   (done on a fine grid). The sampler's mean must lie within four of its own
//!   Monte Carlo standard errors of it. The priors fed to the formula are the
//!   ones rstanarm reports for the fit (`prior_summary`);
//! - and against Stata 19's `bayes: regress` on the same data (recorded below,
//!   numbers only): normal(0, 25) priors on the two slopes, its default
//!   normal(0, 10000) on the constant and igamma(.01, .01) on the variance, 25,000
//!   iterations. Stata's own means are checked against the exact answer under
//!   Stata's priors (within three of Stata's MCSEs), and the dialog's means against
//!   Stata's. The dialog and Stata cannot share every prior: rstanarm puts a
//!   normal prior on the intercept at the centred predictors and an exponential
//!   prior on sigma, where Stata puts normal(0, 10000) on the constant and an
//!   inverse gamma on sigma-squared. The slopes' priors are the same, and the
//!   difference is checked to be small next to the posterior's spread.
//!
//! The logit and Poisson models are checked against the maximum-likelihood fit
//! of `glm` (a normal(0, 10) prior is weak, so the posterior mean lies
//! close to it), and a model R cannot fit (a 0/1 model for a continuous outcome)
//! comes back as an error without taking R down.
//!
//! The test records the R and package versions it ran with (AGENTS.md
//! section 5), and checks R is the one `ENVIRONMENT` pins.

use std::time::{Duration, Instant};

use rcraft_catalog::bayes::BAYESIAN_REGRESSION;
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
    let limit = Instant::now() + Duration::from_secs(300);
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

/// The R and rstanarm versions, recorded; false when rstanarm is missing.
fn versions(e: &mut Engine) -> bool {
    let (outcome, text) = run(
        e,
        Source::Typed,
        "cat(paste(R.version$major, R.version$minor, sep = '.'), \
         if (requireNamespace('rstanarm', quietly = TRUE)) as.character(packageVersion('rstanarm')) \
         else 'none')",
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let mut words = text.split_whitespace();
    let (r, pkg) = (words.next().unwrap_or(""), words.next().unwrap_or("none"));
    if pkg == "none" {
        eprintln!("skipped: rstanarm is not installed");
        return false;
    }
    let pinned = include_str!("../../../ENVIRONMENT")
        .lines()
        .find_map(|l| l.strip_prefix("R_VERSION="))
        .unwrap_or("")
        .trim();
    eprintln!("ran with R {r} and rstanarm {pkg} (ENVIRONMENT pins R {pinned})");
    if r != pinned {
        eprintln!("note: this R is not the pinned one, so the run is not a Verified one");
    }
    true
}

/// The made-up data: `bd`, forty rows of an outcome from two predictors; `lg`,
/// sixty rows of a 0/1 outcome; `ct`, sixty rows of a count.
const MAKE: &str = r#"
bd <- local({
  r <- 1:40
  d <- data.frame(x1 = round(sin(r * 1.7) + 0.5 * cos(r * 0.9), 3), x2 = round(cos(r * 2.3), 3))
  d$y <- round(1 + 2 * d$x1 - 1.5 * d$x2 + 0.8 * sin(r * 3.1) + 0.5 * cos(r * 5.3), 3)
  d
})
lg <- local({
  r <- 1:60
  d <- data.frame(x1 = round(sin(r * 1.7) + 0.5 * cos(r * 0.9), 3), x2 = round(cos(r * 2.3), 3))
  eta <- -0.3 + 1.2 * d$x1 - 0.8 * d$x2
  d$y <- as.numeric(((r * 7919) %% 1000) / 1000 < 1 / (1 + exp(-eta)))
  d
})
ct <- local({
  r <- 1:60
  d <- data.frame(x1 = round(sin(r * 1.7) + 0.5 * cos(r * 0.9), 3), x2 = round(cos(r * 2.3), 3))
  mu <- exp(0.4 + 0.5 * d$x1 - 0.3 * d$x2)
  d$y <- qpois(((r * 7919) %% 1000 + 0.5) / 1000, mu)
  d
})
"#;

/// The exact posterior means of a Gaussian regression with normal priors, as
/// a function of a vector of prior means `m0` and standard deviations `s0` of
/// the intercept and the slopes, with the error scale given an exponential
/// prior of rate `rate` (the intercept is that of the centred predictors, as
/// rstanarm's is).
const EXACT: &str = r#"
exact_centred <- function(d, m0, s0, rate) {
  y <- d$y; X <- cbind(d$x1, d$x2); xb <- colMeans(X)
  Z <- cbind(1, sweep(X, 2, xb)); V0 <- diag(s0^2); n <- length(y)
  sg <- seq(0.01, 8, length.out = 8000)
  lw <- sapply(sg, function(s) {
    R <- chol(s^2 * diag(n) + Z %*% V0 %*% t(Z))
    z <- backsolve(R, y - Z %*% m0, transpose = TRUE)
    -sum(log(diag(R))) - sum(z^2) / 2 + dexp(s, rate, log = TRUE)
  })
  w <- exp(lw - max(lw)); w <- w / sum(w)
  mu <- sapply(sg, function(s) as.vector(solve(crossprod(Z) / s^2 + solve(V0),
                                              crossprod(Z, y) / s^2 + solve(V0, m0))))
  th <- as.vector(mu %*% w)
  c(th[1] - sum(xb * th[-1]), th[-1], sum(sg * w))
}
# Stata's priors: normal(0, 10000) on the constant and normal(0, 25) on the slopes
# (variances), igamma(.01, .01) on the error variance; sigma-squared is integrated
# on a grid even in its logarithm
exact_stata <- function(d) {
  y <- d$y; Z <- cbind(1, d$x1, d$x2); V0 <- diag(c(1e4, 25, 25)); n <- length(y)
  vg <- exp(seq(log(1e-3), log(50), length.out = 6000))
  lw <- sapply(vg, function(v) {
    R <- chol(v * diag(n) + Z %*% V0 %*% t(Z))
    z <- backsolve(R, y, transpose = TRUE)
    -sum(log(diag(R))) - sum(z^2) / 2 - 1.01 * log(v) - 0.01 / v + log(v)
  })
  w <- exp(lw - max(lw)); w <- w / sum(w)
  mu <- sapply(vg, function(v) as.vector(solve(crossprod(Z) / v + solve(V0), crossprod(Z, y) / v)))
  c(as.vector(mu %*% w), sum(vg * w))
}
"#;

/// The context of a dialog working on the data frame `dataset`.
fn context(dataset: &str) -> Context {
    Context {
        dataset: dataset.into(),
        frames: Vec::new(),
        columns: [
            ("y", VarKind::Number),
            ("x1", VarKind::Number),
            ("x2", VarKind::Number),
        ]
        .iter()
        .map(|(name, kind)| ColumnInfo {
            name: (*name).into(),
            kind: *kind,
        })
        .collect(),
    }
}

/// Runs the dialog's code on the data frame of `ctx`.
fn dialog(
    e: &mut Engine,
    ctx: &Context,
    spec: &Spec,
    set: &[(&str, &str)],
    lists: &[(&str, &[&str])],
    flags: &[&str],
) -> (Outcome, String) {
    let mut values = spec.defaults(ctx);
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
    let code = spec.generate(ctx, &values, Style::Base);
    assert!(code.is_ok(), "{code:?}");
    let code = code.unwrap_or_default();
    run(e, Source::Menu(spec.menu.into()), &code)
}

/// Runs R that stops unless every one of `exprs` holds.
fn holds(e: &mut Engine, exprs: &[String]) {
    let exprs = exprs.join(", ");
    let (outcome, text) = run(e, Source::Typed, &format!("stopifnot({exprs})"));
    assert_eq!(outcome, Outcome::Ok, "{exprs}\n{text}");
}

/// Runs R that must succeed (the oracle's objects).
fn define(e: &mut Engine, code: &str) {
    let (outcome, text) = run(e, Source::Typed, code);
    assert_eq!(outcome, Outcome::Ok, "{code}\n{text}");
}

#[test]
fn bayesian_regression_matches_exact_posterior_and_stata() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    define(&mut e, MAKE);
    define(&mut e, EXACT);
    let ctx = context("bd");
    let (outcome, text) = dialog(
        &mut e,
        &ctx,
        &BAYESIAN_REGRESSION,
        &[
            ("y", "y"),
            ("prior", "normal"),
            ("prior_mean", "0"),
            ("prior_sd", "5"),
            ("chains", "4"),
            ("iter", "2000"),
            ("seed", "1"),
        ],
        &[("x", &["x1", "x2"])],
        &["show_priors"],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    for word in [
        "4 chains of 2000 iterations",
        "95% credible interval",
        "mean",
        "sd",
        "2.5%",
        "97.5%",
        "mcse",
        "n_eff",
        "Rhat",
        "(Intercept)",
        "sigma",
        "exponential",
    ] {
        assert!(text.contains(word), "{word} missing from\n{text}");
    }
    assert!(
        !text.contains("mean_PPD") && !text.contains("log-posterior"),
        "{text}"
    );
    define(
        &mut e,
        // the sampler's summary, and the exact posterior under the fit's own priors
        "ps <- rstanarm::prior_summary(bayes); \
         sm <- unclass(summary(bayes, probs = c(0.025, 0.975)))[1:4, ]; \
         ex <- exact_centred(bd, c(as.numeric(ps$prior_intercept$location), as.numeric(ps$prior$location)), \
           c(ps$prior_intercept$adjusted_scale, ps$prior$scale), 1 / ps$prior_aux$adjusted_scale); \
         draws <- as.matrix(bayes)",
    );
    holds(
        &mut e,
        &[
            // the exact posterior mean, to four Monte Carlo standard errors
            "all(abs(sm[, 'mean'] - ex) < 4 * sm[, 'mcse'])".to_string(),
            // the sampler converged: R-hat near 1 and a usable effective sample size
            "all(sm[, 'Rhat'] < 1.02) && all(sm[, 'n_eff'] > 500)".to_string(),
            // the credible interval is the equal-tailed one of the draws
            "isTRUE(all.equal(unname(sm[1:3, c('2.5%', '97.5%')]), \
             unname(t(apply(draws[, 1:3], 2, quantile, c(0.025, 0.975)))), tolerance = 1e-6))"
                .to_string(),
            // the same seed gives the same draws
            "isTRUE(all.equal(draws, as.matrix(rstanarm::stan_glm(y ~ x1 + x2, data = bd, \
             family = stats::gaussian(), prior = rstanarm::normal(location = 0, scale = 5, \
             autoscale = FALSE), chains = 4, iter = 2000, seed = 1, cores = 1, refresh = 0))))"
                .to_string(),
        ],
    );
    // Stata 19: bayes, rseed(1) mcmcsize(20000) burnin(5000)
    //   prior({y:x1}, normal(0,25)) prior({y:x2}, normal(0,25)): regress y x1 x2
    // (means, MCSEs and standard deviations of x1, x2, _cons and sigma2)
    define(
        &mut e,
        "st_mean <- c(x1 = 2.013188, x2 = -1.53162, cons = 0.9928765, sigma2 = 0.5391658); \
         st_mcse <- c(x1 = 0.002753, x2 = 0.003972, cons = 0.001755, sigma2 = 0.002391); \
         st_sd <- c(x1 = 0.1411727, x2 = 0.1638497, cons = 0.1165207, sigma2 = 0.129379); \
         ex_st <- exact_stata(bd); \
         ex_st <- c(ex_st[2], ex_st[3], ex_st[1], ex_st[4]); \
         dr_mean <- c(sm['x1', 'mean'], sm['x2', 'mean'], sm['(Intercept)', 'mean'], mean(draws[, 'sigma']^2)); \
         dr_mcse <- c(sm['x1', 'mcse'], sm['x2', 'mcse'], sm['(Intercept)', 'mcse'], 0.003)",
    );
    holds(
        &mut e,
        &[
            // Stata's own means are the exact posterior under Stata's priors, to three MCSEs
            "all(abs(st_mean - ex_st) < 3 * st_mcse)".to_string(),
            // the dialog's slopes (same prior as Stata's) are Stata's, to three combined MCSEs
            // and a tenth of a posterior standard deviation for the intercept's prior
            "all(abs(dr_mean[1:2] - st_mean[1:2]) < 3 * sqrt(dr_mcse[1:2]^2 + st_mcse[1:2]^2) + 0.1 * st_sd[1:2])"
                .to_string(),
            // the intercept and the error variance have different priors in the two
            // programs, so their means agree only to a tenth of a posterior sd and 3 MCSEs
            "abs(dr_mean[3] - st_mean[3]) < 3 * sqrt(dr_mcse[3]^2 + st_mcse[3]^2) + 0.1 * st_sd[3]"
                .to_string(),
            "abs(dr_mean[4] - st_mean[4]) < 3 * sqrt(dr_mcse[4]^2 + st_mcse[4]^2) + 0.1 * st_sd[4]"
                .to_string(),
        ],
    );
    // the default priors, with the 90% interval, run and print
    let (outcome, text) = dialog(
        &mut e,
        &ctx,
        &BAYESIAN_REGRESSION,
        &[
            ("y", "y"),
            ("chains", "2"),
            ("iter", "400"),
            ("level", "0.9"),
            ("save", "bd_default"),
        ],
        &[("x", &["x1", "x2"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(
        text.contains("90% credible interval") && text.contains("5%") && text.contains("95%"),
        "{text}"
    );
    holds(
        &mut e,
        &["inherits(bd_default, 'stanreg') && nrow(as.matrix(bd_default)) == 400".to_string()],
    );
}

#[test]
fn logit_and_poisson_models_match_maximum_likelihood() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    define(&mut e, MAKE);
    for (data, family, save) in [("lg", "binomial", "post_lg"), ("ct", "poisson", "post_ct")] {
        let ctx = context(data);
        let (outcome, text) = dialog(
            &mut e,
            &ctx,
            &BAYESIAN_REGRESSION,
            &[
                ("y", "y"),
                ("family", family),
                ("prior", "normal"),
                ("prior_mean", "0"),
                ("prior_sd", "10"),
                ("chains", "4"),
                ("iter", "2000"),
                ("seed", "7"),
                ("save", save),
            ],
            &[("x", &["x1", "x2"])],
            &[],
        );
        assert_eq!(outcome, Outcome::Ok, "{family}\n{text}");
        assert!(
            text.contains("Rhat") && text.contains("n_eff") && !text.contains("sigma"),
            "{text}"
        );
        // a weak prior: the posterior mean sits near the maximum-likelihood
        // estimate, within a quarter of the posterior spread plus Monte Carlo error
        // (the posterior is slightly skewed, so it is not the mode)
        holds(
            &mut e,
            &[format!(
                "local({{ g <- stats::glm(y ~ x1 + x2, data = {data}, family = stats::{family}()); \
                 s <- unclass(summary({save}, probs = c(0.025, 0.975)))[1:3, ]; \
                 all(abs(s[, 'mean'] - coef(g)) < 0.25 * s[, 'sd'] + 4 * s[, 'mcse']) && \
                 all(s[, 'Rhat'] < 1.02) && all(s[, 'n_eff'] > 500) }})"
            )],
        );
    }
}

#[test]
fn a_model_r_cannot_fit_is_an_error_not_a_crash() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    define(&mut e, MAKE);
    // a logit model for a continuous outcome
    let ctx = context("bd");
    let (outcome, text) = dialog(
        &mut e,
        &ctx,
        &BAYESIAN_REGRESSION,
        &[("y", "y"), ("family", "binomial"), ("iter", "200")],
        &[("x", &["x1", "x2"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Error, "{text}");
    assert!(text.contains("error"), "{text}");
    // R is still there
    let (outcome, text) = run(&mut e, Source::Typed, "cat(1 + 1)");
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(text.contains('2'), "{text}");
}
