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
//! of `glm` (a normal(0, 100) prior is nearly flat, so the posterior mean lies
//! close to it), and a model R cannot fit (a 0/1 model for a continuous outcome)
//! comes back as an error without taking R down.
//!
//! P6.9b adds: Student t and Cauchy priors, one for each coefficient or one
//! for all, and the intercept's own prior, checked on a Poisson model whose
//! predictor has mean 0 (so rstanarm's intercept at the centred predictors is
//! Stata's constant and every prior can be the same) against the exact
//! posterior on a grid and Stata's `bayes, prior()` (whose `t()` takes the
//! squared scale); sampling the priors alone against their quantiles; the
//! negative binomial, Gamma and ordered logit fits against maximum likelihood
//! and Stata's `bayes: nbreg | glm | ologit` (their auxiliary and cutpoint
//! priors differ, so with a share of the posterior sd as slack); and the
//! Model menu: posterior predictions by hand from the draws, the trace and
//! density plots, posterior predictive p-values against `bayesstats
//! ppvalues`, and loo against plain importance sampling and WAIC.
//!
//! The test records the R and package versions it ran with (AGENTS.md
//! section 5), and checks R is the one `ENVIRONMENT` pins.

use std::time::{Duration, Instant};

use rcraft_catalog::bayes::{BAYESIAN_REGRESSION, BAYES_CHECKS, BAYES_LOO, POSTERIOR_PREDICTIONS};
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
             autoscale = FALSE), chains = 4, iter = 2000, seed = 1, init_r = 0.5, cores = 1, refresh = 0))))"
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
    // a run too short to converge (50 draws per chain) says so after the table
    let (outcome, text) = dialog(
        &mut e,
        &ctx,
        &BAYESIAN_REGRESSION,
        &[
            ("y", "y"),
            ("chains", "2"),
            ("iter", "100"),
            ("seed", "2"),
            ("save", "bd_short"),
        ],
        &[("x", &["x1", "x2"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(
        text.contains("Warning: the chains have not converged"),
        "{text}"
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
                ("prior_sd", "100"),
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
            text.contains("Rhat")
                && text.contains("n_eff")
                && !text.contains("sigma")
                && !text.contains("Warning: the chains"),
            "{text}"
        );
        // a nearly flat prior: the posterior mean sits near the maximum-likelihood
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

// ---- P6.9b: other families, per-coefficient Student t and Cauchy priors,
// the intercept's prior, sampling the priors, and the Model menu ----

/// `pa`: twenty-five counts on a predictor of mean 0 (so rstanarm's intercept
/// at the centred predictors is Stata's constant); `pb`: forty counts on two
/// predictors of mean 0; `fm`: 120 rows of a negative binomial count, a Gamma
/// outcome and an ordered outcome on two predictors (seed 3).
const MAKE_B: &str = r#"
pa <- data.frame(x = seq(-1.2, 1.2, by = 0.1),
  y = c(0, 1, 2, 0, 0, 1, 1, 2, 3, 0, 0, 1, 1, 1, 1, 1, 1, 4, 2, 3, 4, 3, 1, 1, 1))
pb <- data.frame(x1 = rep(c(-1.5, -1, -0.5, 0, 0.5, 1, 1.5, 0), 5), x2 = rep(c(-1, -0.5, 0, 0.5, 1), each = 8),
  y = c(1, 3, 1, 1, 4, 7, 7, 3, 1, 0, 2, 3, 0, 1, 4, 1, 1, 1, 0, 2, 4, 3, 1, 0, 1, 0, 0, 2, 0, 1,
        2, 0, 2, 1, 0, 2, 0, 0, 4, 1))
fm <- local({
  set.seed(3); n <- 120; x1 <- round(rnorm(n), 3); x2 <- round(rnorm(n), 3)
  mu <- exp(0.5 + 0.4 * x1 - 0.3 * x2)
  data.frame(x1 = x1, x2 = x2, ynb = rnbinom(n, size = 3, mu = mu),
    yg = round(rgamma(n, shape = 4, rate = 4 / mu), 4),
    yo = cut(0.8 * x1 - 0.5 * x2 + rlogis(n), c(-Inf, -0.5, 0.7, Inf),
             labels = c("low", "mid", "high"), ordered_result = TRUE))
})
post_sum <- function(f) unclass(summary(f, probs = c(0.025, 0.975)))
"#;

/// Stata 19 on the same data (`bayes, prior() rseed(1) mcmcsize(): poisson |
/// nbreg | glm, family(gamma) link(log) | ologit`, then `bayesstats summary`):
/// posterior mean, sd and MCSE by row; `bayesmh, likelihood(poisson)` with
/// `bayespredict` and `bayesstats ppvalues` (the replicated mean and
/// P(T >= T_obs) of the mean, minimum and maximum). Stata's `t(mu, s2, df)`
/// takes the squared scale. Only numbers.
const STATA_B: &str = r#"
st <- function(...) { m <- rbind(...); colnames(m) <- c("mean", "sd", "mcse"); m }
s_pat <- st(`(Intercept)` = c(0.267967608067124, 0.180328456241475, 0.00271130441932936),
            x = c(0.408433196737714, 0.226774653777511, 0.00320463031481194))
s_pat_mu1 <- c(0.853936119576476, 0.30541005013626, 0.0044115336495064)
s_pac <- st(`(Intercept)` = c(0.263371018711389, 0.17710704842437, 0.00296159657426889),
            x = c(0.409995961970072, 0.230119147301667, 0.00313454533040702))
s_pbt <- st(`(Intercept)` = c(0.317899386329994, 0.142092513567549, 0.00217119371973662),
            x1 = c(0.453486243543547, 0.133684673645191, 0.00223895912312221),
            x2 = c(-0.591818713452986, 0.182597670542909, 0.00303242533170093))
s_pbn <- st(`(Intercept)` = c(0.300920647421338, 0.146551797708944, 0.00222954902078363),
            x1 = c(0.480372417716277, 0.136254128053065, 0.00212606034150242),
            x2 = c(-0.612327865305596, 0.181920077515514, 0.00351150183562031))
s_nb <- st(`(Intercept)` = c(0.541699762988054, 0.0842810036289391, 0.00210458419687921),
           x1 = c(0.385506319196764, 0.096305806922378, 0.00266469568776031),
           x2 = c(-0.219855859350152, 0.0745020728824299, 0.00164311776688061))
s_gam <- st(`(Intercept)` = c(0.464577432843925, 0.0931283061941038, 0.00241528306397612),
            x1 = c(0.445975584225503, 0.10687500893526, 0.0028892756271101),
            x2 = c(-0.286460190435205, 0.0819017478518964, 0.00214855235902872))
s_ol <- st(x1 = c(0.660954640261799, 0.238488008491925, 0.00666987018538623),
           x2 = c(-0.61827858146765, 0.177360542780059, 0.00619400662958601),
           `low|mid` = c(-0.561835008874621, 0.206361522253323, 0.00631329845594903),
           `mid|high` = c(0.722719566559413, 0.214976180188998, 0.00559952726264971))
s_pp <- c(mean = 0.5034, min = 1, max = 0.7862); s_pp_rep <- c(mean = 1.402834, min = 0.00165, max = 4.44645)
"#;

/// The exact posterior of the Poisson model on `pa` with a normal(0, 2)
/// prior on the constant and the given log prior on the slope, on a 401 by
/// 401 grid: the means of the constant, the slope and exp(constant - 1.2
/// slope), the expected count of the first row.
const EXACT_B: &str = r#"
exact_pa <- function(lp_b) {
  a <- seq(-1.5, 1.8, length = 401); b <- seq(-1.2, 2.2, length = 401)
  G <- outer(a, b, Vectorize(function(a, b) { eta <- a + b * pa$x
    sum(pa$y * eta - exp(eta)) + dnorm(a, 0, 2, log = TRUE) + lp_b(b) }))
  w <- exp(G - max(G)); w <- w / sum(w)
  c(`(Intercept)` = sum(w * a), x = sum(w * rep(b, each = 401)),
    mu1 = sum(w * exp(outer(a, b, function(a, b) a - 1.2 * b))))
}
ex_t <- exact_pa(function(b) dt(b / 0.5, 3, log = TRUE) - log(0.5))
ex_c <- exact_pa(function(b) dcauchy(b, 0, 0.5, log = TRUE))
# within k combined Monte Carlo errors, plus a share `slack` of Stata's sd
agree <- function(s, st, k = 4, slack = 0) {
  r <- rownames(st); all(abs(s[r, "mean"] - st[, "mean"]) < k * sqrt(s[r, "mcse"]^2 + st[, "mcse"]^2) + slack * st[, "sd"]) }
"#;

fn ctx_of(dataset: &str, columns: &[&str]) -> Context {
    Context {
        dataset: dataset.into(),
        frames: Vec::new(),
        columns: columns
            .iter()
            .map(|name| ColumnInfo {
                name: (*name).into(),
                kind: VarKind::Number,
            })
            .collect(),
    }
}

fn fit(e: &mut Engine, ctx: &Context, set: &[(&str, &str)], x: &[&str], flags: &[&str]) -> String {
    let (outcome, text) = dialog(e, ctx, &BAYESIAN_REGRESSION, set, &[("x", x)], flags);
    assert_eq!(outcome, Outcome::Ok, "{text}");
    text
}

#[test]
fn student_t_and_cauchy_priors_match_the_exact_posterior_and_stata() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    define(&mut e, MAKE_B);
    define(&mut e, STATA_B);
    define(&mut e, EXACT_B);
    let pa = ctx_of("pa", &["x", "y"]);
    let poisson_int = [
        ("y", "y"),
        ("family", "poisson"),
        ("int_prior", "normal"),
        ("int_mean", "0"),
        ("int_sd", "2"),
        ("iter", "4000"),
    ];
    let mut t = poisson_int.to_vec();
    t.extend([
        ("prior", "student_t"),
        ("prior_df", "3"),
        ("prior_mean", "0"),
        ("prior_sd", "0.5"),
        ("save", "pt"),
    ]);
    let text = fit(&mut e, &pa, &t, &["x"], &["show_priors"]);
    assert!(
        text.contains("student_t") && text.contains("Intercept"),
        "{text}"
    );
    let mut c = poisson_int.to_vec();
    c.extend([
        ("prior", "cauchy"),
        ("prior_mean", "0"),
        ("prior_sd", "0.5"),
        ("save", "pc"),
    ]);
    fit(&mut e, &pa, &c, &["x"], &[]);
    holds(
        &mut e,
        &[
            // the exact posterior means, within four of the sampler's errors
            "local({ s <- post_sum(pt); all(abs(s[c('(Intercept)', 'x'), 'mean'] - ex_t[1:2]) < 4 * s[c('(Intercept)', 'x'), 'mcse']) })".into(),
            "local({ s <- post_sum(pc); all(abs(s[c('(Intercept)', 'x'), 'mean'] - ex_c[1:2]) < 4 * s[c('(Intercept)', 'x'), 'mcse']) })".into(),
            // Stata's, under the same priors, within four of its errors, and the
            // two programs within four combined errors
            "all(abs(s_pat[, 'mean'] - ex_t[1:2]) < 4 * s_pat[, 'mcse'])".into(),
            "all(abs(s_pac[, 'mean'] - ex_c[1:2]) < 4 * s_pac[, 'mcse'])".into(),
            "abs(s_pat_mu1[1] - ex_t[['mu1']]) < 4 * s_pat_mu1[3]".into(),
            "agree(post_sum(pt), s_pat)".into(),
            "agree(post_sum(pc), s_pac)".into(),
            // the priors are the ones typed
            "rstanarm::prior_summary(pt)$prior$dist == 'student_t' && rstanarm::prior_summary(pt)$prior$scale == 0.5".into(),
            "rstanarm::prior_summary(pc)$prior$dist == 'cauchy' && rstanarm::prior_summary(pc)$prior_intercept$scale == 2".into(),
        ],
    );

    // one prior for each coefficient: Student t, then normal
    let pb = ctx_of("pb", &["x1", "x2", "y"]);
    for (prior, df, loc, scale, save) in [
        ("student_t", "3, 5", "0, 0.5", "0.5, 1", "pbt"),
        ("normal", "3", "0, 1", "1, 2", "pbn"),
    ] {
        let mut set = poisson_int.to_vec();
        set.extend([
            ("prior", prior),
            ("prior_df", df),
            ("prior_mean", loc),
            ("prior_sd", scale),
            ("save", save),
        ]);
        fit(&mut e, &pb, &set, &["x1", "x2"], &[]);
    }
    holds(
        &mut e,
        &[
            "isTRUE(all.equal(as.vector(rstanarm::prior_summary(pbt)$prior$df), c(3, 5)))".into(),
            "isTRUE(all.equal(as.vector(rstanarm::prior_summary(pbn)$prior$scale), c(1, 2)))"
                .into(),
            "agree(post_sum(pbt), s_pbt)".into(),
            "agree(post_sum(pbn), s_pbn)".into(),
        ],
    );

    // the priors alone: the slope's draws follow t(3) * 0.5, the intercept's
    // Cauchy(0, 1), whatever the data
    let mut prior_only = vec![
        ("y", "y"),
        ("prior", "student_t"),
        ("prior_df", "3"),
        ("prior_mean", "0"),
        ("prior_sd", "0.5"),
        ("int_prior", "cauchy"),
        ("int_mean", "0"),
        ("int_sd", "1"),
        ("iter", "4000"),
        ("save", "ppd"),
    ];
    prior_only.push(("seed", "3"));
    define(&mut e, MAKE);
    let text = fit(&mut e, &context("bd"), &prior_only, &["x1"], &["prior_pd"]);
    assert!(
        text.contains("Prior summary (the data are not used)"),
        "{text}"
    );
    // the intercept's prior is on the intercept at the centred predictor
    holds(
        &mut e,
        &[
            "local({ d <- as.matrix(ppd); p <- c(0.1, 0.25, 0.5, 0.75, 0.9); \
             ac <- d[, '(Intercept)'] + mean(bd$x1) * d[, 'x1']; \
             all(abs(sapply(0.5 * qt(p, 3), function(q) mean(d[, 'x1'] <= q)) - p) < 0.03) && \
             all(abs(sapply(qcauchy(p), function(q) mean(ac <= q)) - p) < 0.03) })"
                .into(),
        ],
    );
}

/// `all.equal` to 1e-9, unnamed.
fn same(a: &str, b: &str) -> String {
    format!("isTRUE(all.equal(unname(as.vector({a})), unname(as.vector({b})), tolerance = 1e-9))")
}

#[test]
fn other_families_match_stata_and_maximum_likelihood() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    define(&mut e, MAKE_B);
    define(&mut e, STATA_B);
    define(&mut e, EXACT_B);
    let fm = ctx_of("fm", &["x1", "x2", "ynb", "yg"]);
    let diffuse = [
        ("prior", "normal"),
        ("prior_mean", "0"),
        ("prior_sd", "100"),
        ("int_prior", "normal"),
        ("int_mean", "0"),
        ("int_sd", "100"),
    ];
    for (y, family, save) in [("ynb", "neg_binomial", "fnb"), ("yg", "gamma", "fgam")] {
        let mut set = vec![("y", y), ("family", family), ("save", save)];
        set.extend(diffuse);
        let text = fit(&mut e, &fm, &set, &["x1", "x2"], &[]);
        assert!(
            text.contains(if family == "gamma" {
                "shape"
            } else {
                "reciprocal_dispersion"
            }),
            "{text}"
        );
    }
    let mut fo = fm.clone();
    fo.columns.push(ColumnInfo {
        name: "yo".into(),
        kind: VarKind::Factor,
    });
    let text = fit(
        &mut e,
        &fo,
        &[("y", "yo"), ("family", "ordinal"), ("save", "fol")],
        &["x1", "x2"],
        &[],
    );
    assert!(
        text.contains("low|mid") && text.contains("mid|high"),
        "{text}"
    );
    holds(
        &mut e,
        &[
            // near the maximum-likelihood fits under nearly flat priors
            "local({ g <- MASS::glm.nb(ynb ~ x1 + x2, data = fm); s <- post_sum(fnb)[1:3, ]; \
               all(abs(s[, 'mean'] - coef(g)) < 0.25 * s[, 'sd'] + 4 * s[, 'mcse']) })".into(),
            "local({ g <- glm(yg ~ x1 + x2, data = fm, family = Gamma(link = 'log')); s <- post_sum(fgam)[1:3, ]; \
               all(abs(s[, 'mean'] - coef(g)) < 0.25 * s[, 'sd'] + 4 * s[, 'mcse']) })".into(),
            "local({ g <- MASS::polr(yo ~ x1 + x2, data = fm); s <- post_sum(fol); \
               all(abs(s[c('x1', 'x2'), 'mean'] - g$coefficients) < 0.4 * s[c('x1', 'x2'), 'sd']) && \
               all(abs(s[c('low|mid', 'mid|high'), 'mean'] - g$zeta) < 0.4 * s[c('low|mid', 'mid|high'), 'sd']) })".into(),
            // Stata's bayes: nbreg (its prior on log alpha is not rstanarm's on
            // 1 / alpha), glm gamma (which fixes the scale at 1, so its spread is
            // wider: means only), ologit (flat cutpoints where rstanarm has a
            // Dirichlet prior on the category probabilities)
            "agree(post_sum(fnb), s_nb, slack = 0.1)".into(),
            "agree(post_sum(fgam), s_gam, slack = 0.15)".into(),
            "agree(post_sum(fol), s_ol, slack = 0.4)".into(),
            "all(post_sum(fnb)[, 'Rhat'] < 1.02, na.rm = TRUE)".into(),
            "inherits(fol, 'polr') && identical(levels(rstanarm::get_y(fol)), c('low', 'mid', 'high'))".into(),
        ],
    );
}

/// Runs the code of a Model-menu dialog on the fit `model`; returns the
/// outcome, the text and how many plots came back.
fn model_dialog(
    e: &mut Engine,
    spec: &Spec,
    model: &str,
    set: &[(&str, &str)],
    flags: Option<&[&str]>,
) -> (Outcome, String, usize) {
    let ctx = Context {
        dataset: model.into(),
        frames: Vec::new(),
        columns: Vec::new(),
    };
    let mut values = spec.defaults(&ctx);
    for (id, text) in set {
        values.set_text(id, *text);
    }
    if let Some(flags) = flags {
        for w in spec.widgets {
            if let Widget::Check { id, .. } = w {
                values.set_flag(id, flags.contains(id));
            }
        }
    }
    let code = spec.generate(&ctx, &values, Style::Base);
    assert!(code.is_ok(), "{code:?}");
    let code = code.unwrap_or_default();
    let id = e.submit(Source::Menu(spec.menu.into()), &code);
    assert!(id.is_ok(), "{id:?}");
    let id = id.unwrap_or_default();
    settle(e);
    let outcome = e
        .history()
        .entries()
        .last()
        .and_then(|h| h.outcome)
        .unwrap_or(Outcome::Error);
    let block = e.results().blocks().iter().find(|b| b.command == id);
    let plots = block.map_or(0, |b| {
        b.items
            .iter()
            .filter(|i| matches!(i, Item::Plot { .. }))
            .count()
    });
    let text = block
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
    (outcome, text, plots)
}

#[test]
fn the_model_menu_predicts_checks_and_compares() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    define(&mut e, MAKE_B);
    define(&mut e, STATA_B);
    define(&mut e, EXACT_B);
    let pa = ctx_of("pa", &["x", "y"]);
    let base = [
        ("y", "y"),
        ("family", "poisson"),
        ("int_prior", "normal"),
        ("int_mean", "0"),
        ("int_sd", "2"),
        ("prior_mean", "0"),
        ("prior_sd", "0.5"),
        ("iter", "4000"),
    ];
    let mut t = base.to_vec();
    t.extend([("prior", "student_t"), ("prior_df", "3"), ("save", "pt")]);
    fit(&mut e, &pa, &t, &["x"], &[]);
    let mut c = base.to_vec();
    c.extend([("prior", "cauchy"), ("save", "pc")]);
    fit(&mut e, &pa, &c, &["x"], &[]);
    define(&mut e, "pa2 <- rbind(pa, data.frame(x = NA, y = 1))");

    // ---- posterior predictions ----
    for (kind, prefix) in [("epred", "mu"), ("linpred", "eta"), ("predict", "ynew")] {
        let (outcome, text, _) = model_dialog(
            &mut e,
            &POSTERIOR_PREDICTIONS,
            "pt",
            &[
                ("data", "pa2"),
                ("kind", kind),
                ("prefix", prefix),
                ("seed", "7"),
            ],
            None,
        );
        assert_eq!(outcome, Outcome::Ok, "{text}");
    }
    define(
        &mut e,
        "dr <- as.matrix(pt); X <- cbind(1, pa$x); lin <- dr[, c('(Intercept)', 'x')] %*% t(X); \
         ynew <- rstanarm::posterior_predict(pt, newdata = pa, seed = 7)",
    );
    holds(
        &mut e,
        &[
            "is.na(pa2$mu[26]) && is.na(pa2$mu_lower[26])".into(),
            same("pa2$mu[1:25]", "colMeans(exp(lin))"),
            same(
                "pa2$mu_lower[1:25]",
                "apply(exp(lin), 2, quantile, 0.025, names = FALSE)",
            ),
            same("pa2$eta[1:25]", "colMeans(lin)"),
            same("pa2$ynew[1:25]", "colMeans(ynew)"),
            // the first row's expected count against the exact posterior and Stata
            "abs(pa2$mu[1] - ex_t[['mu1']]) < 4 * sd(exp(lin[, 1])) / sqrt(1000)".into(),
            "abs(pa2$mu[1] - s_pat_mu1[1]) < 4 * sqrt(s_pat_mu1[3]^2 + var(exp(lin[, 1])) / 1000)"
                .into(),
        ],
    );

    // ---- trace plot, density overlay and posterior predictive p-values ----
    let (outcome, text, plots) = model_dialog(
        &mut e,
        &BAYES_CHECKS,
        "pt",
        &[],
        Some(&["trace", "density", "ppc_mean", "ppc_min", "ppc_max"]),
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(plots >= 2, "{plots} plots\n{text}");
    assert!(text.contains("Posterior predictive p-values"), "{text}");
    define(
        &mut e,
        "yr <- rstanarm::posterior_predict(pt, seed = 1); \
         hand <- sapply(list(mean = mean, min = min, max = max), function(f) mean(apply(yr, 1, f) >= f(pa$y)))",
    );
    holds(
        &mut e,
        &[
            same("pt$ppvalues$p", "hand"),
            same(
                "pt$ppvalues$observed",
                "c(mean(pa$y), min(pa$y), max(pa$y))",
            ),
            // Stata's bayesstats ppvalues, within the Monte Carlo error of both
            "all(abs(pt$ppvalues$p - s_pp) < 4 * sqrt(2 * pmax(s_pp * (1 - s_pp), 0.01) / 2000))"
                .into(),
            "all(abs(pt$ppvalues$replicated - s_pp_rep) < c(0.03, 0.01, 0.1))".into(),
        ],
    );

    // ---- loo ----
    let (outcome, text, _) = model_dialog(&mut e, &BAYES_LOO, "pt", &[("others", "pc")], None);
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(
        text.contains("elpd_loo") && text.contains("Ranked by elpd_loo"),
        "{text}"
    );
    define(
        &mut e,
        "ll <- rstanarm::log_lik(pt); is_elpd <- sum(-log(colMeans(exp(-ll)))); \
         lppd <- sum(log(colMeans(exp(ll)))); waic_elpd <- lppd - sum(apply(ll, 2, var)); \
         cmp <- loo::loo_compare(list(pt = pt$loo, pc = pc$loo))",
    );
    holds(
        &mut e,
        &[
            "inherits(pt$loo, 'psis_loo') && inherits(pc$loo, 'psis_loo')".into(),
            // Pareto smoothing moves plain importance sampling little here, and
            // WAIC is near too
            "abs(pt$loo$estimates['elpd_loo', 1] - is_elpd) < 0.5".into(),
            "abs(pt$loo$estimates['elpd_loo', 1] - waic_elpd) < 1".into(),
            same(
                "abs(cmp[2, 'elpd_diff'])",
                "abs(pt$loo$estimates['elpd_loo', 1] - pc$loo$estimates['elpd_loo', 1])",
            ),
        ],
    );

    // ---- an ordered logit: each category's probability from its cutpoints ----
    let mut fo = ctx_of("fm", &["x1", "x2"]);
    fo.columns.push(ColumnInfo {
        name: "yo".into(),
        kind: VarKind::Factor,
    });
    fit(
        &mut e,
        &fo,
        &[
            ("y", "yo"),
            ("family", "ordinal"),
            ("iter", "1000"),
            ("save", "fol"),
        ],
        &["x1", "x2"],
        &[],
    );
    let (outcome, text, _) = model_dialog(
        &mut e,
        &POSTERIOR_PREDICTIONS,
        "fol",
        &[("data", "fm"), ("prefix", "p")],
        None,
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    define(
        &mut e,
        "D <- as.matrix(fol); eta <- D[, c('x1', 'x2')] %*% t(as.matrix(fm[, c('x1', 'x2')])); \
         p1 <- colMeans(plogis(D[, 'low|mid'] - eta)); \
         p3 <- colMeans(1 - plogis(D[, 'mid|high'] - eta))",
    );
    holds(
        &mut e,
        &[
            same("fm$p_low", "p1"),
            same("fm$p_high", "p3"),
            same("fm$p_low + fm$p_mid + fm$p_high", "rep(1, 120)"),
        ],
    );
}

/// Stata 19 with its own default priors (`bayes, rseed(1) mcmcsize(40000)
/// burnin(5000): logit y x1 x2` on `lg`, `regress y x1 x2` on `bd`), and the
/// run on `bd` with normal(0, 25) slopes recorded above: posterior mean, sd
/// and MCSE by row. Only numbers.
const STATA_M: &str = r#"
s_lg <- st(`(Intercept)` = c(-0.528899163967921, 0.309238103978842, 0.00452952153813886),
           x1 = c(1.20652448120493, 0.417920069798189, 0.00708102886238382),
           x2 = c(-0.574872120506271, 0.416227723724512, 0.00853738119417732))
s_bd <- st(`(Intercept)` = c(0.994252285241206, 0.117082898724128, 0.00185027795046497),
           x1 = c(2.01070919612944, 0.144633488354913, 0.00240842076272814),
           x2 = c(-1.52817749774359, 0.166010809506582, 0.00288001219508063),
           sigma2 = c(0.539336072933889, 0.131413422215749, 0.00169865315445797))
s_bd25 <- st(`(Intercept)` = c(0.9928765, 0.1165207, 0.001755), x1 = c(2.013188, 0.1411727, 0.002753),
             x2 = c(-1.53162, 0.1638497, 0.003972), sigma2 = c(0.5391658, 0.129379, 0.002391))
# the dialog's table, from the draws
mh_sum <- function(f) { s <- rstan::monitor(f$draws, warmup = 0, probs = c(0.025, 0.975), print = FALSE)
  s <- as.matrix(as.data.frame(s)[, 1:7]); colnames(s)[2] <- "mcse"; s }
# the exact posterior of the Gaussian model under normal priors of variances V0 on
# the constant and slopes and igamma(.01, .01) on the variance (as exact_stata)
exact_v <- function(d, V0) {
  y <- d$y; Z <- cbind(1, d$x1, d$x2); n <- length(y)
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

#[test]
fn statas_priors_match_stata_and_the_exact_posterior() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    define(&mut e, MAKE);
    define(&mut e, MAKE_B);
    define(&mut e, STATA_B);
    define(&mut e, EXACT_B);
    define(&mut e, STATA_M);
    let long = [("chains", "4"), ("iter", "10000"), ("seed", "1")];
    let bd = ctx_of("bd", &["y", "x1", "x2"]);
    // Stata's default priors on the Gaussian model, and normal(0, 25) slopes
    let mut set = vec![("y", "y"), ("save", "g0")];
    set.extend(long);
    let text = fit(&mut e, &bd, &set, &["x1", "x2"], &["stata"]);
    for word in [
        "Stata's priors, random-walk Metropolis-Hastings, 4 chains of 10000",
        "sigma2: igamma(0.01, 0.01)",
        "mcse",
        "n_eff",
        "Rhat",
        "Acceptance rate",
    ] {
        assert!(text.contains(word), "{word} missing from\n{text}");
    }
    assert!(!text.contains("Warning"), "{text}");
    let mut set = vec![
        ("y", "y"),
        ("prior", "normal"),
        ("prior_sd", "5"),
        ("save", "g25"),
    ];
    set.extend(long);
    fit(&mut e, &bd, &set, &["x1", "x2"], &["stata"]);
    define(
        &mut e,
        "e0 <- exact_v(bd, diag(c(1e4, 1e4, 1e4))); e25 <- exact_v(bd, diag(c(1e4, 25, 25))); \
         m0 <- mh_sum(g0); m25 <- mh_sum(g25)",
    );
    holds(
        &mut e,
        &[
            "inherits(g0, 'bayes_mh') && identical(dim(g0$draws), c(5000L, 4L, 4L))".into(),
            // the exact posterior under Stata's priors, to four MCSEs
            "all(abs(m0[, 'mean'] - e0) < 4 * m0[, 'mcse'])".into(),
            "all(abs(m25[, 'mean'] - e25) < 4 * m25[, 'mcse'])".into(),
            // Stata's own means are that exact posterior too
            "all(abs(s_bd[, 'mean'] - e0) < 4 * s_bd[, 'mcse'])".into(),
            // and the two samplers agree, with no slack for different priors
            "agree(m0, s_bd) && agree(m25, s_bd25)".into(),
            "all(abs(m0[rownames(s_bd), 'sd'] / s_bd[, 'sd'] - 1) < 0.1)".into(),
            "all(m0[, 'Rhat'] < 1.02) && all(m0[, 'n_eff'] > 500)".into(),
            "all(g0$acceptance > 0.15 & g0$acceptance < 0.4)".into(),
        ],
    );
    // the same seed gives the same draws
    let mut set = vec![
        ("y", "y"),
        ("prior", "normal"),
        ("prior_sd", "5"),
        ("save", "g25b"),
    ];
    set.extend(long);
    fit(&mut e, &bd, &set, &["x1", "x2"], &["stata"]);
    holds(&mut e, &["identical(g25$draws, g25b$draws)".into()]);

    // a logit model with Stata's defaults
    let lg = ctx_of("lg", &["y", "x1", "x2"]);
    let mut set = vec![("y", "y"), ("family", "binomial"), ("save", "l0")];
    set.extend(long);
    fit(&mut e, &lg, &set, &["x1", "x2"], &["stata"]);
    // the Poisson model on pa with a Student t slope and a normal(0, 2) constant
    let pa = ctx_of("pa", &["y", "x"]);
    let mut set = vec![
        ("y", "y"),
        ("family", "poisson"),
        ("prior", "student_t"),
        ("prior_sd", "0.5"),
        ("prior_df", "3"),
        ("int_prior", "normal"),
        ("int_sd", "2"),
        ("save", "pt0"),
    ];
    set.extend(long);
    fit(&mut e, &pa, &set, &["x"], &["stata"]);
    // the negative binomial, Gamma and ordered logit models with Stata's defaults
    let mut fm = ctx_of("fm", &["x1", "x2", "ynb", "yg"]);
    for (y, family, save) in [("ynb", "neg_binomial", "n0"), ("yg", "gamma", "ga0")] {
        let mut set = vec![("y", y), ("family", family), ("save", save)];
        set.extend(long);
        let text = fit(&mut e, &fm, &set, &["x1", "x2"], &["stata"]);
        assert!(
            text.contains(if family == "gamma" {
                "scale: fixed at 1"
            } else {
                "lnalpha"
            }),
            "{text}"
        );
    }
    fm.columns.push(ColumnInfo {
        name: "yo".into(),
        kind: VarKind::Factor,
    });
    let mut set = vec![("y", "yo"), ("family", "ordinal"), ("save", "o0")];
    set.extend(long);
    let text = fit(&mut e, &fm, &set, &["x1", "x2"], &["stata"]);
    assert!(
        text.contains("low|mid") && text.contains("cutpoints: flat"),
        "{text}"
    );
    holds(
        &mut e,
        &[
            "agree(mh_sum(l0), s_lg)".into(),
            "local({ s <- mh_sum(pt0); all(abs(s[, 'mean'] - ex_t[1:2]) < 4 * s[, 'mcse']) && agree(s, s_pat) })"
                .into(),
            // within combined MCSEs of Stata, with no slack: the priors are the same
            // (log alpha's mean is not compared: its posterior's long left tail is
            // reached by neither sampler)
            "agree(mh_sum(n0), s_nb)".into(),
            "agree(mh_sum(ga0), s_gam)".into(),
            "agree(mh_sum(o0), s_ol)".into(),
            // the Gamma scale fixed at 1 gives Stata's spread, not glm's
            "local({ s <- mh_sum(ga0); all(abs(s[rownames(s_gam), 'sd'] / s_gam[, 'sd'] - 1) < 0.1) })"
                .into(),
            "local({ s <- mh_sum(o0); all(abs(s[rownames(s_ol), 'sd'] / s_ol[, 'sd'] - 1) < 0.1) })"
                .into(),
            "all(o0$draws[, , 'low|mid'] < o0$draws[, , 'mid|high'])".into(),
        ],
    );
    // an outcome the family cannot take is an error, not a crash
    let mut set = vec![("y", "y"), ("family", "poisson"), ("save", "bad")];
    set.extend(long);
    let (outcome, text) = dialog(
        &mut e,
        &bd,
        &BAYESIAN_REGRESSION,
        &set,
        &[("x", &["x1", "x2"])],
        &["stata"],
    );
    assert_eq!(outcome, Outcome::Error, "{text}");
    assert!(text.contains("must be a count"), "{text}");
}
