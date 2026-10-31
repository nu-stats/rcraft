//! P2B.4, the maximum-likelihood models and what reads a fit afterwards
//! (`predict`, `test`, `lincom`, `estat`), translated to R and run in a real R.
//!
//! As with `regress`, the point is that the translation gives *Stata's*
//! numbers, and each one is checked against the same number worked out a
//! second way, never against the R the translation wrote:
//!
//! - every model's estimates against the likelihood written out by hand
//!   (with `dbinom`, `dnbinom`, `plogis`...) and maximised with `optim`;
//! - every model's standard errors against the Hessian of that hand-written
//!   likelihood, which is the observed information Stata uses; for probit
//!   and nbreg, R's own `summary()` is shown to give different numbers;
//! - robust and clustered errors after `logit` against the sandwich built by
//!   hand with Stata's `N / (N - 1)` and `G / (G - 1)`, and shown to differ
//!   from regress's HC1 scaling;
//! - `mlogit`'s base outcome is the most frequent one;
//! - `test` after `regress` against the F of nested models (`anova`), which
//!   is the same number for least squares, and after `robust` against the
//!   Wald test built by hand;
//! - `predict` against the linear predictor worked out by hand, in rows
//!   outside the estimation sample too, and stored as a float;
//! - `estat ic`, `hettest` and `ovtest` against their textbook formulas.
//!
//! Without a supported R, or without haven, dplyr, MASS and nnet, the test
//! is skipped with a note.

// Stata's numbers are kept exactly as it printed them (%24.17g)
#![allow(clippy::panic, clippy::excessive_precision)]

use std::time::{Duration, Instant};

use rcraft_engine::{Engine, Item, Outcome, Source};
use rcraft_rhost::{find_installs, Env, SessionConfig};
use rcraft_statatr::data_r::translate;
use rcraft_statatr::reader::read;

fn engine() -> Option<Engine> {
    let install = find_installs(&Env::from_process())
        .into_iter()
        .find(|i| i.is_supported());
    let Some(install) = install else {
        eprintln!("skipped: no supported R on this machine");
        return None;
    };
    let mut e = Engine::start(SessionConfig::new(install)).ok()?;
    let (have, _) = run(
        &mut e,
        "stopifnot(requireNamespace(\"dplyr\", quietly = TRUE), requireNamespace(\"haven\", quietly = TRUE), \
         requireNamespace(\"MASS\", quietly = TRUE), requireNamespace(\"nnet\", quietly = TRUE))",
    );
    if have != Outcome::Ok {
        eprintln!("skipped: the R packages dplyr, haven, MASS and nnet are needed");
        return None;
    }
    Some(e)
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
                    Item::Warning(w) => format!("warning: {w}"),
                    Item::Error { message, .. } => format!("error: {message}"),
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

/// The numbers `show` prints, at full precision, with anything `setup` prints
/// thrown away.
fn numbers(e: &mut Engine, setup: &str, show: &str) -> Vec<f64> {
    let text = ok(
        e,
        &format!(
            "invisible(utils::capture.output({{{setup}}}))\ncat(format(c({show}), digits = 15), \"\\n\")"
        ),
    );
    let v: Vec<f64> = text
        .split_whitespace()
        .filter_map(|w| w.parse().ok())
        .collect();
    assert!(!v.is_empty(), "no number came back from {show}: {text}");
    v
}

fn at(v: &[f64], i: usize) -> f64 {
    v.get(i).copied().unwrap_or(f64::NAN)
}

fn close(got: f64, want: f64, by: f64, what: &str) {
    assert!((got - want).abs() < by, "{what}: {got} against {want}");
}

fn all_close(got: &[f64], want: &[f64], by: f64, what: &str) {
    assert_eq!(got.len(), want.len(), "{what}: {got:?} against {want:?}");
    for i in 0..got.len() {
        close(at(got, i), at(want, i), by, what);
    }
}

/// Eighty rows with a binary, a count, an overdispersed count, an ordered
/// and an unordered outcome, made with a fixed seed.
fn fixture(e: &mut Engine) -> String {
    let dir = std::env::temp_dir().join(format!(
        "rcraft-stata-models-{}-{}",
        std::process::id(),
        fresh()
    ));
    let _ = std::fs::create_dir_all(&dir);
    let d = dir.display().to_string().replace('\\', "/");
    ok(
        e,
        &format!(
            "local({{
  set.seed(20261030)
  n <- 80
  x <- round(stats::rnorm(n), 3)
  w <- round(stats::runif(n, -1, 1), 3)
  g <- rep(0:1, length.out = n)
  cl <- rep(1:8, each = 10)
  expo <- round(stats::runif(n, 0.5, 3), 2)
  yb <- as.numeric(stats::runif(n) < stats::plogis(-0.3 + 0.8 * x - 0.6 * w))
  cnt <- stats::rpois(n, expo * exp(0.2 + 0.5 * x))
  nb <- stats::rnbinom(n, size = 1.5, mu = exp(0.6 + 0.4 * x))
  ord <- cut(0.9 * x - 0.5 * w + stats::rlogis(n), c(-Inf, -0.8, 0.6, Inf), labels = FALSE)
  e2 <- exp(0.8 + 0.5 * x)
  e3 <- exp(0.2 - 0.7 * x)
  u <- stats::runif(n)
  p1 <- 1 / (1 + e2 + e3)
  p2 <- e2 / (1 + e2 + e3)
  cat3 <- ifelse(u < p1, 1, ifelse(u < p1 + p2, 2, 3))
  y <- 1 + 2 * x - w + 0.5 * g + stats::rnorm(n) * (1 + abs(x))
  haven::write_dta(data.frame(id = 1:n, x = x, w = w, g = as.numeric(g), cl = as.numeric(cl),
    expo = expo, yb = yb, yb2 = 2 * yb, cnt = as.numeric(cnt), nb = as.numeric(nb),
    ord = as.numeric(ord), cat3 = cat3, y = y), '{d}/models.dta')
}})"
        ),
    );
    d
}

/// Translates a do-file and insists every line became R.
fn script(dofile: &str) -> String {
    let t = translate(&read(dofile));
    for l in &t.lines {
        assert!(
            l.result.is_ok(),
            "not translated: {}\n{:?}",
            l.stata,
            l.result
        );
    }
    t.script()
}

/// The line of the script that starts with `start`, to run again and keep
/// what it returns.
fn line_of(code: &str, start: &str) -> String {
    code.lines()
        .rfind(|l| l.starts_with(start))
        .map(str::to_string)
        .unwrap_or_else(|| panic!("no line starts with {start}:\n{code}"))
}

fn refusal(dofile: &str) -> String {
    translate(&read(dofile))
        .lines
        .iter()
        .find_map(|l| l.result.as_ref().err().map(|e| e.reason.clone()))
        .unwrap_or_default()
}

/// The design matrix for `yb x w` and the logit likelihood by hand.
const LOGIT_BY_HAND: &str = "X <- cbind(1, models$x, models$w); yv <- models$yb; \
     nll <- function(b) -sum(stats::dbinom(yv, 1, stats::plogis(drop(X %*% b)), log = TRUE)); \
     o <- stats::optim(c(0, 0, 0), nll, method = \"BFGS\", control = list(reltol = 1e-15, maxit = 2000))";

#[test]
fn logit_gives_the_estimates_and_errors_of_the_likelihood_written_by_hand() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    let code = script(&format!("use \"{dir}/models.dta\"\nlogit yb x w\n"));
    let text = ok(&mut e, &code);
    assert!(text.contains("Pseudo R2"), "{text}");
    assert!(text.contains("P>|z|"), "logit's tests are z tests: {text}");

    let got = numbers(&mut e, "0", "unname(stata_b)");
    let want = numbers(&mut e, LOGIT_BY_HAND, "o$par");
    all_close(&got, &want, 1e-4, "logit coefficient");
    // the observed information of the hand-written likelihood, at Stata's estimates
    let se = numbers(&mut e, "0", "sqrt(diag(stata_vcov))");
    let hand = numbers(
        &mut e,
        &format!("{LOGIT_BY_HAND}; H <- stats::optimHess(unname(stata_b), nll, control = list(ndeps = rep(1e-4, 3)))"),
        "sqrt(diag(solve(H)))",
    );
    all_close(&se, &hand, 1e-6, "logit standard error");

    // the header's null likelihood is the constant-only model's
    let header = numbers(&mut e, "h <- stata_ml_header(stata_estimates)", "h");
    let ll0 = numbers(
        &mut e,
        "n1 <- sum(models$yb == 1); n0 <- sum(models$yb == 0); nn <- n1 + n0",
        "n1 * log(n1 / nn) + n0 * log(n0 / nn)",
    );
    close(at(&header, 0), 80.0, 0.5, "observations");
    close(
        at(&header, 2),
        at(&ll0, 0),
        1e-9,
        "the constant-only log likelihood",
    );

    // Stata's logit takes any value other than 0 as a success, so 0/2 is 0/1
    let code = script(&format!("use \"{dir}/models.dta\"\nlogit yb2 x w\n"));
    ok(&mut e, &code);
    let two = numbers(&mut e, "0", "unname(stata_b)");
    all_close(&two, &got, 1e-10, "the 0/2 outcome");
}

#[test]
fn probit_errors_are_the_observed_information_not_rs_expected_one() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    let code = script(&format!("use \"{dir}/models.dta\"\nprobit yb x w\n"));
    ok(&mut e, &code);
    let by_hand = "X <- cbind(1, models$x, models$w); yv <- models$yb; \
         nll <- function(b) -sum(stats::dbinom(yv, 1, stats::pnorm(drop(X %*% b)), log = TRUE)); \
         o <- stats::optim(c(0, 0, 0), nll, method = \"BFGS\", control = list(reltol = 1e-15, maxit = 2000)); \
         H <- stats::optimHess(unname(stata_b), nll, control = list(ndeps = rep(1e-4, 3)))";
    let got = numbers(&mut e, "0", "unname(stata_b)");
    let want = numbers(&mut e, by_hand, "o$par");
    all_close(&got, &want, 1e-4, "probit coefficient");
    let se = numbers(&mut e, "0", "sqrt(diag(stata_vcov))");
    let hand = numbers(&mut e, by_hand, "sqrt(diag(solve(H)))");
    all_close(&se, &hand, 1e-6, "probit standard error");
    // R's own summary() gives the expected information's, a different number
    let r_own = numbers(
        &mut e,
        "0",
        "unname(summary(stata_estimates)$coefficients[, 2])",
    );
    assert!(
        (at(&r_own, 1) - at(&se, 1)).abs() > 1e-5,
        "R's summary() should differ for probit: {r_own:?} against {se:?}"
    );
}

#[test]
fn robust_and_clustered_logit_scale_as_stata_scales_a_likelihood_model() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    let pieces = "X <- cbind(1, models$x, models$w); yv <- models$yb; b <- unname(stata_b); \
         p <- stats::plogis(drop(X %*% b)); s <- X * (yv - p); \
         Hi <- solve(t(X) %*% (X * (p * (1 - p)))); n <- nrow(X); k <- ncol(X)";

    let code = script(&format!("use \"{dir}/models.dta\"\nlogit yb x w, robust\n"));
    let text = ok(&mut e, &code);
    assert!(
        text.contains("Wald chi2"),
        "robust shows a Wald test: {text}"
    );
    let got = numbers(&mut e, "0", "sqrt(diag(stata_vcov))");
    let want = numbers(
        &mut e,
        &format!("{pieces}; V <- Hi %*% crossprod(s) %*% Hi * n / (n - 1)"),
        "sqrt(diag(V))",
    );
    all_close(&got, &want, 1e-10, "robust standard error");
    // regress's HC1 scaling, n / (n - k), would be a different number
    let hc1 = numbers(
        &mut e,
        &format!("{pieces}; V <- Hi %*% crossprod(s) %*% Hi * n / (n - k)"),
        "sqrt(diag(V))",
    );
    assert!(
        (at(&hc1, 1) - at(&got, 1)).abs() > 1e-6,
        "HC1's scaling should differ"
    );

    let code = script(&format!(
        "use \"{dir}/models.dta\"\nlogit yb x w, vce(cluster cl)\n"
    ));
    ok(&mut e, &code);
    let got = numbers(&mut e, "0", "sqrt(diag(stata_vcov))");
    let want = numbers(
        &mut e,
        &format!(
            "{pieces}; sg <- rowsum(s, models$cl); G <- nrow(sg); \
             V <- Hi %*% crossprod(sg) %*% Hi * G / (G - 1)"
        ),
        "sqrt(diag(V))",
    );
    all_close(&got, &want, 1e-10, "clustered standard error");
    let df = numbers(&mut e, "0", "as.numeric(is.finite(stata_df))");
    assert_eq!(at(&df, 0), 0.0, "z tests after a likelihood model");
}

#[test]
fn poisson_with_exposure_predicts_every_row_including_those_left_out() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    let code = script(&format!(
        "use \"{dir}/models.dta\"\npoisson cnt x if g == 1, exposure(expo)\npredict double mu\npredict lp, xb\n"
    ));
    ok(&mut e, &code);
    let got = numbers(&mut e, "0", "unname(stata_b)");
    let want = numbers(
        &mut e,
        "k <- models$g == 1; X <- cbind(1, models$x[k]); yv <- models$cnt[k]; lo <- log(models$expo[k]); \
         nll <- function(b) -sum(stats::dpois(yv, exp(drop(X %*% b) + lo), log = TRUE)); \
         o <- stats::optim(c(0, 0), nll, method = \"BFGS\", control = list(reltol = 1e-15, maxit = 2000))",
        "o$par",
    );
    all_close(&got, &want, 1e-4, "poisson coefficient");
    // the number of events for all 80 rows, the 40 left out by the if included
    let mu = numbers(&mut e, "0", "models$mu");
    let hand = numbers(
        &mut e,
        "b <- unname(stata_b)",
        "models$expo * exp(b[1] + b[2] * models$x)",
    );
    assert_eq!(mu.len(), 80, "every row is predicted");
    all_close(&mu, &hand, 1e-9, "predicted count");
    // xb includes ln(exposure), as Stata's does, and is stored as a float
    let lp = numbers(
        &mut e,
        "b <- unname(stata_b); h <- b[1] + b[2] * models$x + log(models$expo)",
        "as.numeric(all(models$lp == stata_float(h))), max(abs(models$lp - h))",
    );
    assert_eq!(at(&lp, 0), 1.0, "xb is the float of the linear prediction");
    assert!(at(&lp, 1) > 0.0, "and the float is not the double");

    let code = script(&format!("use \"{dir}/models.dta\"\npoisson cnt x, irr\n"));
    let text = ok(&mut e, &code);
    assert!(text.contains("IRR"), "{text}");
}

#[test]
fn nbreg_errors_come_from_the_whole_likelihood_alpha_included() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    let code = script(&format!("use \"{dir}/models.dta\"\nnbreg nb x\n"));
    let text = ok(&mut e, &code);
    assert!(text.contains("alpha"), "{text}");
    let by_hand = "X <- cbind(1, models$x); yv <- models$nb; \
         nll <- function(p) -sum(stats::dnbinom(yv, size = exp(-p[3]), mu = exp(drop(X %*% p[1:2])), log = TRUE)); \
         o <- stats::optim(c(0, 0, 0), nll, method = \"BFGS\", control = list(reltol = 1e-15, maxit = 5000)); \
         H <- stats::optimHess(unname(stata_b), nll, control = list(ndeps = rep(1e-4, 3)))";
    let got = numbers(&mut e, "0", "unname(stata_b)");
    let want = numbers(&mut e, by_hand, "o$par");
    all_close(&got, &want, 1e-4, "nbreg coefficient and ln(alpha)");
    let se = numbers(&mut e, "0", "sqrt(diag(stata_vcov))");
    let hand = numbers(&mut e, by_hand, "sqrt(diag(solve(H)))");
    all_close(&se, &hand, 1e-5, "nbreg standard error");
    // glm.nb's own summary holds theta fixed, which is a different number
    let r_own = numbers(
        &mut e,
        "0",
        "unname(summary(stata_estimates)$coefficients[, 2])",
    );
    assert!(
        (at(&r_own, 1) - at(&se, 1)).abs() > 1e-5,
        "glm.nb's summary() should differ: {r_own:?} against {se:?}"
    );
}

#[test]
fn ologit_and_oprobit_give_statas_cut_points_and_probabilities() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    let code = script(&format!(
        "use \"{dir}/models.dta\"\nologit ord x w\npredict p1 p2 p3\n"
    ));
    let text = ok(&mut e, &code);
    assert!(text.contains("/cut1"), "{text}");
    let by_hand = |cdf: &str| {
        format!(
            "X <- cbind(models$x, models$w); yv <- models$ord; \
             nll <- function(p) {{ eta <- drop(X %*% p[1:2]); cuts <- c(-Inf, p[3], p[4], Inf); \
               -sum(log({cdf}(cuts[yv + 1] - eta) - {cdf}(cuts[yv] - eta))) }}; \
             o <- stats::optim(c(0, 0, -0.5, 0.5), nll, method = \"BFGS\", control = list(reltol = 1e-15, maxit = 5000)); \
             H <- stats::optimHess(unname(stata_b), nll, control = list(ndeps = rep(1e-4, 4)))"
        )
    };
    let got = numbers(&mut e, "0", "unname(stata_b)");
    let want = numbers(&mut e, &by_hand("stats::plogis"), "o$par");
    all_close(&got, &want, 1e-4, "ologit coefficient and cut point");
    let se = numbers(&mut e, "0", "sqrt(diag(stata_vcov))");
    let hand = numbers(&mut e, &by_hand("stats::plogis"), "sqrt(diag(solve(H)))");
    all_close(&se, &hand, 1e-4, "ologit standard error");
    // one variable per outcome, in the order of the outcome values
    let p = numbers(&mut e, "0", "models$p1, models$p3");
    let hand = numbers(
        &mut e,
        "b <- unname(stata_b); eta <- b[1] * models$x + b[2] * models$w",
        "stats::plogis(b[3] - eta), 1 - stats::plogis(b[4] - eta)",
    );
    all_close(&p, &hand, 1e-6, "predicted probability");
    let sum = numbers(
        &mut e,
        "0",
        "max(abs(models$p1 + models$p2 + models$p3 - 1))",
    );
    assert!(at(&sum, 0) < 1e-6, "the probabilities add to one");

    let code = script(&format!("use \"{dir}/models.dta\"\noprobit ord x w\n"));
    ok(&mut e, &code);
    let got = numbers(&mut e, "0", "unname(stata_b)");
    let want = numbers(&mut e, &by_hand("stats::pnorm"), "o$par");
    all_close(&got, &want, 1e-4, "oprobit coefficient and cut point");
}

#[test]
fn mlogit_takes_the_most_frequent_outcome_as_its_base() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    let code = script(&format!(
        "use \"{dir}/models.dta\"\nmlogit cat3 x\npredict q1 q2 q3\n"
    ));
    ok(&mut e, &code);
    // the most frequent value, counted by hand
    let counts = numbers(
        &mut e,
        "0",
        "sum(models$cat3 == 1), sum(models$cat3 == 2), sum(models$cat3 == 3)",
    );
    let most = (0..3)
        .max_by(|a, b| at(&counts, *a).total_cmp(&at(&counts, *b)).then(b.cmp(a)))
        .map_or(0.0, |i| i as f64 + 1.0);
    assert!(
        most != 1.0,
        "the fixture should not have 1 as its most frequent outcome"
    );
    let base = numbers(&mut e, "0", "as.numeric(stata_base)");
    assert_eq!(at(&base, 0), most, "the base outcome");

    let by_hand = format!(
        "X <- cbind(1, models$x); yv <- models$cat3; others <- setdiff(1:3, {most}); \
         nll <- function(p) {{ B <- matrix(0, 2, 3); B[, others] <- matrix(p, 2); \
           E <- exp(X %*% B); -sum(log(E[cbind(seq_along(yv), yv)] / rowSums(E))) }}; \
         o <- stats::optim(c(0, 0, 0, 0), nll, method = \"BFGS\", control = list(reltol = 1e-15, maxit = 5000))"
    );
    let got = numbers(&mut e, "0", "unname(stata_b)");
    let want = numbers(&mut e, &by_hand, "o$par");
    all_close(&got, &want, 1e-4, "mlogit coefficient");
    // q1 is outcome 1's probability, whichever outcome is the base
    let q = numbers(&mut e, "0", "models$q1");
    let hand = numbers(
        &mut e,
        &format!(
            "{by_hand}; B <- matrix(0, 2, 3); B[, others] <- matrix(o$par, 2); E <- exp(X %*% B)"
        ),
        "E[, 1] / rowSums(E)",
    );
    all_close(&q, &hand, 1e-5, "outcome 1's probability");
}

#[test]
fn test_after_regress_is_the_f_of_nested_models() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    let code = script(&format!(
        "use \"{dir}/models.dta\"\nregress y x w g\ntest x w\n"
    ));
    ok(&mut e, &code);
    let res = numbers(
        &mut e,
        &format!("res <- {}", line_of(&code, "stata_test(")),
        "res",
    );
    let nested = numbers(
        &mut e,
        "a <- stats::anova(stats::lm(y ~ g, data = models), stats::lm(y ~ x + w + g, data = models))",
        "a$F[2], 2, a$Res.Df[2], a$`Pr(>F)`[2]",
    );
    all_close(&res, &nested, 1e-9, "F, its degrees of freedom and p");

    // one equation, and two in parentheses, against the Wald test by hand
    let code = script(&format!(
        "use \"{dir}/models.dta\"\nregress y x w g\ntest x = w\ntest (x = 2) (w = -1)\n"
    ));
    ok(&mut e, &code);
    let lines: Vec<&str> = code
        .lines()
        .filter(|l| l.starts_with("stata_test("))
        .collect();
    let one = numbers(
        &mut e,
        &format!("res <- {}", lines.first().copied().unwrap_or("")),
        "res[1]",
    );
    let hand = numbers(
        &mut e,
        "b <- stats::coef(stata_estimates); V <- stats::vcov(stata_estimates)",
        "(b[2] - b[3])^2 / (V[2, 2] + V[3, 3] - 2 * V[2, 3])",
    );
    close(at(&one, 0), at(&hand, 0), 1e-9, "test x = w");
    let two = numbers(
        &mut e,
        &format!("res <- {}", lines.get(1).copied().unwrap_or("")),
        "res[1]",
    );
    let hand = numbers(
        &mut e,
        "b <- stats::coef(stata_estimates); V <- stats::vcov(stata_estimates); \
         R <- rbind(c(0, 1, 0, 0), c(0, 0, 1, 0)); d <- R %*% b - c(2, -1)",
        "drop(t(d) %*% solve(R %*% V %*% t(R), d)) / 2",
    );
    close(at(&two, 0), at(&hand, 0), 1e-9, "test (x = 2) (w = -1)");

    // after robust, the Wald test with HC1, which is not the nested F
    let code = script(&format!(
        "use \"{dir}/models.dta\"\nregress y x w g, robust\ntest x w\n"
    ));
    ok(&mut e, &code);
    let robust = numbers(
        &mut e,
        &format!("res <- {}", line_of(&code, "stata_test(")),
        "res[1]",
    );
    let hand = numbers(
        &mut e,
        "X <- cbind(1, models$x, models$w, models$g); ev <- stats::residuals(stata_estimates); \
         n <- nrow(X); k <- ncol(X); xtxi <- solve(t(X) %*% X); \
         V <- xtxi %*% (t(X) %*% (X * ev^2)) %*% xtxi * n / (n - k); \
         b <- stats::coef(stata_estimates); R <- rbind(c(0, 1, 0, 0), c(0, 0, 1, 0)); d <- R %*% b",
        "drop(t(d) %*% solve(R %*% V %*% t(R), d)) / 2",
    );
    close(at(&robust, 0), at(&hand, 0), 1e-9, "robust Wald F");
    assert!(
        (at(&robust, 0) - at(&nested, 0)).abs() > 1e-6,
        "the robust F is not the nested one"
    );

    // after vce(cluster), the denominator degrees of freedom are G - 1
    let code = script(&format!(
        "use \"{dir}/models.dta\"\nregress y x w g, vce(cluster cl)\ntest x\n"
    ));
    ok(&mut e, &code);
    let res = numbers(
        &mut e,
        &format!("res <- {}", line_of(&code, "stata_test(")),
        "res",
    );
    assert_eq!(
        at(&res, 2),
        7.0,
        "eight clusters leave seven degrees of freedom"
    );
    let p = numbers(
        &mut e,
        "0",
        &format!("stats::pf({}, 1, 7, lower.tail = FALSE)", at(&res, 0)),
    );
    close(at(&res, 3), at(&p, 0), 1e-9, "p on G - 1");
}

#[test]
fn lincom_and_test_after_logit_use_z_and_chi_squared() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    let code = script(&format!(
        "use \"{dir}/models.dta\"\nregress y x w\nlincom x + w\n"
    ));
    ok(&mut e, &code);
    let res = numbers(
        &mut e,
        &format!("res <- {}", line_of(&code, "stata_lincom(")),
        "res[[1]], res[[2]]",
    );
    let hand = numbers(
        &mut e,
        "b <- stats::coef(stata_estimates); V <- stats::vcov(stata_estimates)",
        "b[2] + b[3], sqrt(V[2, 2] + V[3, 3] + 2 * V[2, 3])",
    );
    all_close(&res, &hand, 1e-10, "lincom estimate and standard error");

    let code = script(&format!(
        "use \"{dir}/models.dta\"\nlogit yb x w\ntest x w\nlincom x, or\n"
    ));
    ok(&mut e, &code);
    let chi2 = numbers(
        &mut e,
        &format!("res <- {}", line_of(&code, "stata_test(")),
        "res",
    );
    let hand = numbers(
        &mut e,
        "X <- cbind(1, models$x, models$w); b <- unname(stata_b); p <- stats::plogis(drop(X %*% b)); \
         V <- solve(t(X) %*% (X * (p * (1 - p)))); d <- b[2:3]",
        "drop(t(d) %*% solve(V[2:3, 2:3], d)), 2",
    );
    close(at(&chi2, 0), at(&hand, 0), 1e-8, "Wald chi-squared");
    close(at(&chi2, 1), 2.0, 1e-12, "its degrees of freedom");
    let or = numbers(
        &mut e,
        &format!("res <- {}", line_of(&code, "stata_lincom(")),
        "res[[1]], res[[2]]",
    );
    let hand = numbers(
        &mut e,
        "b <- unname(stata_b); se <- sqrt(stata_vcov[2, 2])",
        "exp(b[2]), exp(b[2]) * se",
    );
    all_close(
        &or,
        &hand,
        1e-10,
        "the odds ratio and its delta-method error",
    );
}

#[test]
fn predict_after_regress_fills_every_row_as_a_float() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    let code = script(&format!(
        "use \"{dir}/models.dta\"\nregress y x if g == 1\npredict yhat\npredict double yd\npredict r, residuals\npredict y0 if g == 0\n"
    ));
    ok(&mut e, &code);
    let yd = numbers(&mut e, "0", "models$yd");
    let hand = numbers(&mut e, "b <- unname(stata_b)", "b[1] + b[2] * models$x");
    assert_eq!(yd.len(), 80, "the rows the if left out are predicted too");
    all_close(&yd, &hand, 1e-10, "linear prediction");
    let checks = numbers(
        &mut e,
        "0",
        "as.numeric(all(models$yhat == stata_float(models$yd))), \
         as.numeric(all(models$r == stata_float(models$y - models$yd))), \
         sum(is.na(models$y0)), as.numeric(all(models$y0[models$g == 0] == models$yhat[models$g == 0]))",
    );
    assert_eq!(at(&checks, 0), 1.0, "predict stores a float");
    assert_eq!(
        at(&checks, 1),
        1.0,
        "residuals are the outcome less the prediction"
    );
    assert_eq!(at(&checks, 2), 40.0, "the if leaves the other rows missing");
    assert_eq!(at(&checks, 3), 1.0, "and fills its own rows");

    // a name already in use is refused, as Stata refuses it
    let again = script(&format!(
        "use \"{dir}/models.dta\"\nregress y x\npredict x\n"
    ));
    let (outcome, text) = run(&mut e, &again);
    assert_eq!(outcome, Outcome::Error, "{text}");
    assert!(text.contains("already defined"), "{text}");
}

#[test]
fn estat_gives_statas_information_criteria_and_tests() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    let code = script(&format!(
        "use \"{dir}/models.dta\"\nregress y x w\nestat ic\nestat hettest\nestat hettest, rhs\nestat ovtest\nestat vce\n"
    ));
    ok(&mut e, &code);
    let ic = numbers(
        &mut e,
        &format!("res <- {}", line_of(&code, "stata_estat_ic(")),
        "unlist(res)",
    );
    let hand = numbers(
        &mut e,
        "ll <- as.numeric(stats::logLik(stata_estimates))",
        "80, ll, 3, -2 * ll + 6, -2 * ll + log(80) * 3",
    );
    all_close(&ic, &hand, 1e-9, "N, ll, df, AIC and BIC");
    let r_aic = numbers(&mut e, "0", "stats::AIC(stata_estimates)");
    close(
        r_aic[0] - at(&ic, 3),
        2.0,
        1e-9,
        "R's AIC also counts the variance",
    );

    let het: Vec<&str> = code
        .lines()
        .filter(|l| l.starts_with("stata_hettest("))
        .collect();
    let projection = |z: &str| {
        format!(
            "ev <- stats::residuals(stata_estimates); gg <- ev^2 / mean(ev^2) - 1; Z <- cbind(1, {z}); \
             lm_stat <- 0.5 * drop(t(gg) %*% Z %*% solve(t(Z) %*% Z) %*% t(Z) %*% gg)"
        )
    };
    let fitted = numbers(
        &mut e,
        &format!("res <- {}", het.first().copied().unwrap_or("")),
        "res[1]",
    );
    let hand = numbers(
        &mut e,
        &projection("stats::fitted(stata_estimates)"),
        "lm_stat",
    );
    close(
        at(&fitted, 0),
        at(&hand, 0),
        1e-9,
        "hettest on the fitted values",
    );
    let rhs = numbers(
        &mut e,
        &format!("res <- {}", het.get(1).copied().unwrap_or("")),
        "res[1], res[2]",
    );
    let hand = numbers(&mut e, &projection("models$x, models$w"), "lm_stat");
    close(at(&rhs, 0), at(&hand, 0), 1e-9, "hettest on the regressors");
    close(at(&rhs, 1), 2.0, 1e-12, "two regressors");

    let ov = numbers(
        &mut e,
        &format!("res <- {}", line_of(&code, "stata_ovtest(")),
        "res[1]",
    );
    // Stata rescales the fitted values to [0, 1] and stores each power as a
    // float, so its F is the nested F with those powers...
    let hand = numbers(
        &mut e,
        "m0 <- stats::lm(y ~ x + w, data = models); h <- stats::fitted(m0); \
         u <- (h - min(h)) / (max(h) - min(h)); \
         p2 <- readBin(writeBin(as.numeric(u^2), raw(), size = 4), \"double\", size = 4, n = length(u)); \
         p3 <- readBin(writeBin(as.numeric(u^3), raw(), size = 4), \"double\", size = 4, n = length(u)); \
         p4 <- readBin(writeBin(as.numeric(u^4), raw(), size = 4), \"double\", size = 4, n = length(u)); \
         a <- stats::anova(m0, stats::lm(y ~ x + w + p2 + p3 + p4, data = models)); \
         r <- stats::anova(m0, stats::lm(y ~ x + w + I(h^2) + I(h^3) + I(h^4), data = models))",
        "a$F[2], r$F[2]",
    );
    close(
        at(&ov, 0),
        at(&hand, 0),
        1e-10,
        "RESET F with Stata's float powers",
    );
    // ...which is the exact one to about six digits, no more
    let gap = (at(&ov, 0) - at(&hand, 1)).abs() / at(&hand, 1);
    assert!(
        gap > 1e-8 && gap < 1e-4,
        "the float powers move F slightly: {gap}"
    );
}

#[test]
fn what_is_not_translated_after_a_model_says_why() {
    assert!(
        refusal("use \"d.dta\"\npredict yhat\n").contains("needs a model"),
        "predict with no model before it"
    );
    assert!(
        refusal("use \"d.dta\"\nmlogit y x\ntest x = 1\n").contains("name the equation"),
        "after mlogit an equation needs its names written with equations"
    );
    assert!(
        refusal("use \"d.dta\"\nlogit y x\nstreg x, dist(weibull)\n").contains("streg"),
        "streg is not translated yet"
    );
    assert!(
        refusal("use \"d.dta\"\nnbreg y x, vce(bootstrap)\n").contains("bootstrap"),
        "a vce that is not translated is named"
    );
    assert!(
        refusal("use \"d.dta\"\npoisson y x, exposure(t) offset(o)\n").contains("not both"),
        "exposure and offset together"
    );
    assert!(
        refusal("use \"d.dta\"\nregress y x\nlincom x - 1\n").contains("constant"),
        "lincom with a constant"
    );
    assert!(
        refusal("use \"d.dta\"\nlogit y x\nestat hettest\n").contains("after regress"),
        "hettest is for regress"
    );
    assert!(
        refusal("use \"d.dta\"\nlogit y x\npredict s, dbeta\n").contains("dbeta"),
        "a predict statistic that is not translated is named"
    );
}

/// Each number within `tol` of Stata's, relative to its size.
fn near_stata(got: &[f64], stata: &[f64], tol: f64, what: &str) {
    assert_eq!(
        got.len(),
        stata.len(),
        "{what}: {got:?} against Stata's {stata:?}"
    );
    for (g, s) in got.iter().zip(stata) {
        assert!(
            (g - s).abs() <= tol * s.abs().max(1.0),
            "{what}: {g} against Stata's {s}"
        );
    }
}

/// Stata 19 SE's own results on this same data (`models`), run in batch mode
/// on 2026-10-30 and printed with `%24.17g`. Most agree to ten digits or more.
/// Where Stata's own iterations stop earlier than R's (ologit's gradient is
/// about 5e-6 at Stata's estimates, 2e-7 at R's; logit after `i.g`; the
/// poisson margin, which must equal the mean count exactly and is 2.4125 in R
/// but 2.4125000019 in Stata), the tolerance is six digits and says so.
#[test]
fn the_models_agree_with_stata_19() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    let fit = |e: &mut Engine, cmd: &str, show: &str| -> Vec<f64> {
        let code = script(&format!("use \"{dir}/models.dta\"\n{cmd}\n"));
        ok(e, &code);
        numbers(e, "0", show)
    };
    let rerun = |e: &mut Engine, cmd: &str, start: &str, show: &str| -> Vec<f64> {
        let code = script(&format!("use \"{dir}/models.dta\"\n{cmd}\n"));
        ok(e, &code);
        numbers(e, &format!("res <- {}", line_of(&code, start)), show)
    };
    let b_se = "unname(stata_b), unname(sqrt(diag(stata_vcov)))";

    let got = fit(
        &mut e,
        "regress y x w g, vce(cluster cl)",
        &format!("{b_se}, stata_df"),
    );
    near_stata(
        &got,
        &[
            0.90782238091709189,
            1.8379643645175934,
            -0.96934912606092671,
            0.86343158525323538,
            0.34165245303205355,
            0.3619667718985719,
            0.48641861746295489,
            0.40960119598931632,
            7.0,
        ],
        1e-10,
        "regress, vce(cluster cl): b, se and df_r = G - 1",
    );
    let got = rerun(
        &mut e,
        "regress y x w g, vce(cluster cl)\ntest x w",
        "stata_test(",
        "res[1], res[3]",
    );
    near_stata(
        &got,
        &[56.515593151738031, 7.0],
        1e-10,
        "test after clustering",
    );
    let got = fit(&mut e, "regress y x w, robust", b_se);
    near_stata(
        &got,
        &[
            1.3514155530542686,
            1.9265970683026681,
            -0.9140205174299384,
            0.23905479523371148,
            0.26150004070253402,
            0.39168691077357537,
        ],
        1e-10,
        "regress, robust (HC1)",
    );
    let got = fit(&mut e, "probit yb x w", b_se);
    near_stata(
        &got,
        &[
            0.12362790668735292,
            0.79572554525624506,
            -1.0530699961645282,
            0.1648672654430498,
            0.1923150529540622,
            0.3090964989509753,
        ],
        1e-10,
        "probit, observed information",
    );
    let got = fit(&mut e, "logit yb x w, robust", b_se);
    near_stata(
        &got,
        &[
            0.18644887826476039,
            1.363199246020228,
            -1.7470886873426894,
            0.28017457137732377,
            0.38204137334680699,
            0.49786700439256282,
        ],
        1e-10,
        "logit, robust: N / (N - 1)",
    );
    let got = fit(
        &mut e,
        "logit yb x w, vce(cluster cl)",
        "unname(sqrt(diag(stata_vcov)))",
    );
    near_stata(
        &got,
        &[0.19659074836245186, 0.4506964474614174, 0.43513419898139655],
        1e-10,
        "logit, clustered: G / (G - 1)",
    );
    let got = fit(&mut e, "nbreg nb x", b_se);
    near_stata(
        &got,
        &[
            0.6654924804910296,
            0.44577236830095102,
            -0.73053565022100209,
            0.11410420800909059,
            0.10787436435765344,
            0.34086156471440715,
        ],
        1e-9,
        "nbreg with /lnalpha",
    );
    let got = fit(&mut e, "ologit ord x w", "unname(stata_b)");
    near_stata(
        &got,
        &[
            0.53465541589421406,
            0.33508927778611575,
            -0.62940104652368734,
            0.65545508627858329,
        ],
        1e-6,
        "ologit and its cut points (Stata's own fit stops earlier)",
    );
    let got = fit(
        &mut e,
        "mlogit cat3 x",
        "as.numeric(stata_base), unname(stata_b)",
    );
    near_stata(
        &got,
        &[
            2.0,
            -0.71063529275358917,
            -0.7869965436975902,
            -0.34073709457654067,
            -1.2654174894097019,
        ],
        1e-7,
        "mlogit: base outcome 2, the most frequent",
    );
    let got = rerun(
        &mut e,
        "regress y x w\nestat ic",
        "stata_estat_ic(",
        "unlist(res)[2:5]",
    );
    near_stata(
        &got,
        &[
            -169.33271352580755,
            3.0,
            344.6654270516151,
            351.8115069556367,
        ],
        1e-12,
        "estat ic: ll, df, AIC, BIC",
    );
    let got = rerun(
        &mut e,
        "regress y x w\nestat hettest",
        "stata_hettest(",
        "res[1]",
    );
    near_stata(&got, &[3.037405763040681], 1e-12, "estat hettest");
    let got = rerun(
        &mut e,
        "regress y x w\nestat hettest, rhs",
        "stata_hettest(",
        "res[1], res[2]",
    );
    near_stata(
        &got,
        &[3.4511055574605791, 2.0],
        1e-12,
        "estat hettest, rhs",
    );
    let got = rerun(
        &mut e,
        "regress y x w\nestat ovtest",
        "stata_ovtest(",
        "res[1], res[3]",
    );
    near_stata(
        &got,
        &[0.51465677976399593, 74.0],
        1e-11,
        "estat ovtest, with Stata's float powers",
    );
    let got = rerun(
        &mut e,
        "regress y c.x##c.x w\nmargins, dydx(x)",
        "stata_margins(",
        "res$b, sqrt(diag(res$V)), as.numeric(names(res$table)[3] == \"t\")",
    );
    near_stata(
        &got,
        &[1.9628880322447464, 0.21547004559432981, 1.0],
        1e-10,
        "margins after a quadratic, on t",
    );
    let got = rerun(
        &mut e,
        "logit yb x i.g\nmargins, dydx(x g)",
        "stata_margins(",
        "res$b",
    );
    near_stata(
        &got,
        &[0.22167164928071345, -0.025904504851519128],
        1e-6,
        "average marginal effects after logit",
    );
    let got = rerun(
        &mut e,
        "poisson cnt x, exposure(expo)\nmargins",
        "stata_margins(",
        "res$b, sqrt(diag(res$V))",
    );
    near_stata(
        &got,
        &[2.4125000019148466, 0.17365554993703941],
        1e-8,
        "margins after poisson",
    );
    let got = fit(
        &mut e,
        "ologit ord x w\npredict p\npredict q1 q2 q3",
        "models$p[1], models$q1[1], models$q3[1]",
    );
    near_stata(
        &got,
        &[
            0.51647889614105225,
            0.51647889614105225,
            0.20574034750461578,
        ],
        1e-6,
        "predict after ologit: one variable is the first outcome",
    );
    let got = rerun(
        &mut e,
        "logit yb x w\nlincom x + w",
        "stata_lincom(",
        "res[[1]], res[[2]]",
    );
    near_stata(
        &got,
        &[-0.38388944132246139, 0.52445865610364761],
        1e-10,
        "lincom",
    );
    let got = rerun(
        &mut e,
        "logit yb x w\nmargins, at(x=(0 1))",
        "stata_margins(",
        "res$b, sqrt(diag(res$V))[1]",
    );
    near_stata(
        &got,
        &[
            0.5449587031052433,
            0.78732552200888906,
            0.055959030239695555,
        ],
        1e-10,
        "margins, at()",
    );
}

/// The descriptive commands (P2B.4 part one) on this same data, against
/// Stata 19 SE run in batch mode on 2026-10-30: every figure agrees to
/// about 15 digits.
#[test]
fn the_descriptive_commands_agree_with_stata_19() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    let rerun = |e: &mut Engine, cmd: &str, start: &str, show: &str| -> Vec<f64> {
        let code = script(&format!("use \"{dir}/models.dta\"\n{cmd}\n"));
        ok(e, &code);
        numbers(e, &format!("res <- {}", line_of(&code, start)), show)
    };
    // obs, mean, sd, variance, skewness, kurtosis, then p1 ... p99, min, max
    let pick = "obs, mean, sd, variance, skewness, kurtosis, p1, p5, p10, p25, p50, p75, p90, p95, p99, min, max";
    let got = rerun(
        &mut e,
        "summarize x y, detail",
        "stata_summarize(",
        &format!("unlist(res[1, c(\"{}\")])", pick.replace(", ", "\", \"")),
    );
    near_stata(
        &got,
        &[
            80.0,
            -0.121475,
            1.1034152502829395,
            1.2175252145569622,
            -0.38904249237915856,
            3.7922878609735462,
            -3.891,
            -2.0205000000000002,
            -1.325,
            -0.88349999999999995,
            -0.10200000000000001,
            0.48250000000000004,
            1.2969999999999999,
            1.8115000000000001,
            2.0630000000000002,
            -3.891,
            2.0630000000000002,
        ],
        1e-12,
        "summarize x, detail",
    );
    let got = rerun(
        &mut e,
        "summarize x y, detail",
        "stata_summarize(",
        &format!("unlist(res[2, c(\"{}\")])", pick.replace(", ", "\", \"")),
    );
    near_stata(
        &got,
        &[
            80.0,
            1.1357311360696081,
            2.9701172417179857,
            8.821596429550457,
            0.072917564118979705,
            2.9458007685120333,
            -6.061345365088699,
            -3.8691643052767963,
            -2.9451464084859253,
            -0.9532353901444679,
            1.3842907721004238,
            2.8595645587654399,
            5.0327669469103693,
            6.3554806213730881,
            9.1811303392437047,
            -6.061345365088699,
            9.1811303392437047,
        ],
        1e-12,
        "summarize y, detail",
    );
    let got = rerun(
        &mut e,
        "summarize x if g == 1",
        "stata_summarize(",
        "res$obs, res$mean, res$sd",
    );
    near_stata(
        &got,
        &[40.0, 0.12784999999999999, 0.9820770127512195],
        1e-12,
        "summarize with if",
    );
    let got = rerun(
        &mut e,
        "tabulate ord cat3, chi2",
        "stata_tabulate(",
        "res[1, 1], res[1, 2], res[1, 3], res[2, 1], res[3, 3], attr(res, \"chi2\")[c(\"chi2\", \"p\")]",
    );
    near_stata(
        &got,
        &[
            5.0,
            11.0,
            13.0,
            6.0,
            12.0,
            4.9925778665909837,
            0.28805990065319959,
        ],
        1e-12,
        "tabulate, chi2",
    );
    let got = rerun(
        &mut e,
        "correlate x w y",
        "stata_correlate(",
        "res[2, 1], res[3, 1], res[3, 2]",
    );
    near_stata(
        &got,
        &[
            0.030562992155089359,
            0.71025990044143561,
            -0.15748353526503756,
        ],
        1e-12,
        "correlate",
    );
    let got = rerun(
        &mut e,
        "correlate x w y, covariance",
        "stata_correlate(",
        "res[1, 1], res[2, 1], res[3, 3]",
    );
    near_stata(
        &got,
        &[1.2175252145569624, 0.0196550905063291, 8.821596429550457],
        1e-12,
        "correlate, covariance",
    );
    let tt = "res$statistic, res$parameter, res$p.value, res$stderr";
    for (cmd, stata) in [
        (
            "ttest x == 0.1",
            [
                -1.795273919550332,
                79.0,
                0.076434344499437201,
                0.1233655753521299,
            ],
        ),
        (
            "ttest y, by(g)",
            [
                -2.6370215700362909,
                78.0,
                0.0100900980595278,
                0.64044306061678036,
            ],
        ),
        (
            "ttest y, by(g) unequal",
            [
                -2.6370215700362909,
                77.94905418583474,
                0.010091270326294101,
                0.64044306061678036,
            ],
        ),
        (
            "ttest x == w",
            [
                -0.7361408412039967,
                79.0,
                0.46382524817465443,
                0.13774538012882839,
            ],
        ),
        (
            "ttest x == w, unpaired",
            [
                -0.72678990744809435,
                158.0,
                0.46843008766222288,
                0.13951762257684039,
            ],
        ),
    ] {
        let got = rerun(&mut e, cmd, "print(stats::t.test(", tt);
        near_stata(&got, &stata, 1e-12, cmd);
    }
}

/// A number no other call in this run has had: the tests in a file run at
/// once, and each writing its data to the same folder let one read a file
/// another was still writing.
fn fresh() -> usize {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

/// `ttest`'s confidence intervals at other levels, against the intervals
/// Stata 19 SE prints (to seven figures) for this same data.
#[test]
fn ttest_intervals_at_other_levels_agree_with_stata_19() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    for (cmd, lo, hi) in [
        ("ttest y, by(g) level(90)", -2.754959, -0.6227649),
        ("ttest x == 0.1, level(99)", -0.447099, 0.204149),
        ("ttest x == w, level(80)", -0.2794166, 0.0766166),
    ] {
        let code = script(&format!("use \"{dir}/models.dta\"\n{cmd}\n"));
        ok(&mut e, &code);
        let got = numbers(
            &mut e,
            &format!("res <- {}", line_of(&code, "print(stats::t.test(")),
            "res$conf.int[1], res$conf.int[2]",
        );
        near_stata(&got, &[lo, hi], 5e-7, cmd);
    }
}

/// Twelve rows on which a regressor predicts the outcome perfectly in each of
/// the ways Stata tells apart, as in the Stata 19 probe (see the P2B.4 notes).
fn separation_fixture(e: &mut Engine) -> String {
    let dir = std::env::temp_dir().join(format!(
        "rcraft-stata-perfect-{}-{}",
        std::process::id(),
        fresh()
    ));
    let _ = std::fs::create_dir_all(&dir);
    let d = dir.display().to_string().replace('\\', "/");
    ok(
        e,
        &format!(
            "local({{
  y <- c(1, 1, 1, 1, 0, 0, 0, 1, 0, 0, 1, 0)
  x <- c(1, 2, 3, 4, 4, 5, 6, 7, 8, 9, 10, 11)
  d <- c(0, 0, 1, 1, 1, 0, 0, 0, 0, 0, 1, 0)
  w <- c(3, 1, 4, 1, 5, 9, 2, 6, 5, 3, 5, 8)
  k <- c(1, 2, 3, 1, 2, 3, 1, 2, 3, 1, 3, 3)
  y3 <- as.numeric(x > 4); y3[4] <- 1
  haven::write_dta(data.frame(y, x, d, w, k,
    y2 = as.numeric(x <= 4), y3,
    y4 = ifelse(d == 1, 1, y), y5 = ifelse(d == 1, 0, y), y6 = ifelse(d == 0, 0, y),
    y7 = ifelse(k == 2, 0, y), y8 = as.numeric(x > 6)), '{d}/sep.dta')
}})"
        ),
    );
    d
}

#[test]
fn perfect_prediction_is_dealt_with_as_stata_19_deals_with_it() {
    let Some(mut e) = engine() else { return };
    let dir = separation_fixture(&mut e);
    let use_ = format!("use \"{dir}/sep.dta\"\n");
    let fitted = |e: &mut Engine, cmd: &str| -> (String, Vec<f64>) {
        let text = ok(e, &script(&format!("{use_}{cmd}\n")));
        let v = numbers(e, "0", "stata_b, nobs(stata_estimates)");
        (text, v)
    };

    // a dummy that decides one outcome: omitted with its rows, the rest fitted
    // on the others; Stata 19's e(b) and e(N) (the zero for the omitted
    // variable is left out here)
    let stata = [-0.32934464815365411, 0.90411677071097207];
    for (cmd, note) in [
        ("logit y4 d w", "note: d != 0 predicts success perfectly;"),
        ("logit y5 d w", "note: d != 0 predicts failure perfectly;"),
        ("logit y4 w d", "note: d != 0 predicts success perfectly;"),
        (
            "logit y4 i.d w",
            "note: 0.d != 1 predicts success perfectly;",
        ),
        (
            "logit y4 c.w##i.d",
            "note: 0.d != 1 predicts success perfectly;",
        ),
    ] {
        let (text, v) = fitted(&mut e, cmd);
        assert!(text.contains(note), "{cmd}: {text}");
        assert!(
            text.contains("omitted and 4 obs not used."),
            "{cmd}: {text}"
        );
        let (cons, w) = (at(&v, 0), at(&v, 1));
        near_stata(&[w, cons], &stata, 1e-9, cmd);
        assert_eq!(at(&v, v.len() - 1), 8.0, "{cmd}: e(N)");
    }
    let (text, v) = fitted(&mut e, "logit y7 i.k w");
    assert!(
        text.contains("note: 2.k != 0 predicts failure perfectly;")
            && text.contains("3 obs not used."),
        "{text}"
    );
    // the factor keeps its levels 1 and 3, so 3.k is still estimated
    near_stata(
        &v[..3],
        &[2.9785114719954278, 3.8139418395161799, -1.3030142870170742],
        1e-9,
        "logit y7 i.k w",
    );
    assert_eq!(at(&v, 3), 9.0, "e(N)");

    // the quasi-complete cases: only the rows at the cut are kept
    let (text, v) = fitted(&mut e, "logit y3 x");
    assert!(
        text.contains("note: outcome = x > 4 predicts data perfectly except for")
            && text.contains("x == 4 subsample:")
            && text.contains("x omitted and 10 obs not used."),
        "{text}"
    );
    near_stata(&v, &[0.0, 2.0], 1e-9, "logit y3 x: a constant on two rows");
    let (text, v) = fitted(&mut e, "logit y x if x <= 4 | x >= 5 & y == 0");
    assert!(
        text.contains("note: outcome = x < 4 predicts data perfectly except for")
            && text.contains("x omitted and 8 obs not used."),
        "{text}"
    );
    near_stata(&v, &[0.0, 2.0], 1e-9, "the same with an if");
    let (text, v) = fitted(&mut e, "logit y6 d w");
    assert!(
        text.contains("note: d != 1 predicts failure perfectly;")
            && text.contains("note: w != 5 predicts success perfectly;")
            && text.contains("w omitted and 2 obs not used."),
        "two in turn, the second on the rows the first left: {text}"
    );
    near_stata(&v, &[0.0, 2.0], 1e-9, "logit y6 d w");

    // complete separation stops, in Stata's words, for probit too
    for (cmd, message) in [
        ("logit y2 x", "outcome = x <= 4 predicts data perfectly"),
        ("logit y2 x w", "outcome = x <= 4 predicts data perfectly"),
        ("probit y2 x", "outcome = x <= 4 predicts data perfectly"),
        ("logit y8 w x", "outcome = x > 6 predicts data perfectly"),
        ("logit y3 x w", "outcome = w <= 1 predicts data perfectly"),
    ] {
        let (outcome, text) = run(&mut e, &script(&format!("{use_}{cmd}\n")));
        assert_eq!(outcome, Outcome::Error, "{cmd}: {text}");
        assert!(text.contains(message), "{cmd}: {text}");
    }

    // and the ordinary fits are untouched: no note, the same estimates as glm
    let (text, v) = fitted(&mut e, "logit y w");
    assert!(!text.contains("note:"), "{text}");
    let g = numbers(
        &mut e,
        "0",
        "stats::coef(stats::glm(y ~ w, family = stats::binomial(), data = sep, control = stats::glm.control(epsilon = 1e-12)))",
    );
    near_stata(&v[..2], &g, 1e-9, "logit y w against glm");
}

#[test]
fn by_goes_on_past_a_group_that_stata_cannot_fit() {
    let Some(mut e) = engine() else { return };
    let dir = separation_fixture(&mut e);
    // in Stata 19 `by` reports r(2000) for the group and goes on to the next
    let code = script(&format!(
        "use \"{dir}/sep.dta\"\nsort d\nby d: logit y2 x\nby d: logit y w\n"
    ));
    let text = ok(&mut e, &code);
    assert_eq!(
        text.matches("predicts data perfectly").count(),
        2,
        "both groups, then the next command: {text}"
    );
    assert!(text.contains("-> d = 1"), "{text}");
}

/// Robust and clustered errors after nbreg, ologit, oprobit and mlogit, and the
/// likelihood-ratio test of alpha after nbreg, against Stata 19 on `models`.
/// Six digits: the bread is each model's own variance, which for ologit and
/// mlogit comes from R's numerical Hessian (see the notes above).
#[test]
fn robust_errors_after_the_other_likelihood_models_agree_with_stata_19() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    let se = |e: &mut Engine, cmd: &str, names: &[&str]| -> Vec<f64> {
        ok(e, &script(&format!("use \"{dir}/models.dta\"\n{cmd}\n")));
        let pick: Vec<String> = names
            .iter()
            .map(|n| format!("sqrt(stata_vcov[\"{n}\", \"{n}\"])"))
            .collect();
        numbers(e, "0", &pick.join(", "))
    };
    let nb = ["x", "(Intercept)", "/lnalpha"];
    near_stata(
        &se(&mut e, "nbreg nb x, vce(robust)", &nb),
        &[
            0.096321330899123245,
            0.12072961105109967,
            0.35624427194401809,
        ],
        1e-6,
        "nbreg, robust",
    );
    near_stata(
        &se(&mut e, "nbreg nb x, vce(cluster cl)", &nb),
        &[
            0.11975966068493894,
            0.078489570543055789,
            0.49880799190745001,
        ],
        1e-6,
        "nbreg, clustered",
    );
    let ol = ["x", "w", "/cut1", "/cut2"];
    near_stata(
        &se(&mut e, "ologit ord x w, vce(robust)", &ol),
        &[
            0.20295988781520097,
            0.35591311674491405,
            0.22753245407567338,
            0.25989714190966423,
        ],
        1e-6,
        "ologit, robust",
    );
    near_stata(
        &se(&mut e, "ologit ord x w, vce(cluster cl)", &ol),
        &[
            0.14219638072514956,
            0.40191690471570396,
            0.18552382694791314,
            0.19889956780877471,
        ],
        1e-6,
        "ologit, clustered",
    );
    near_stata(
        &se(&mut e, "oprobit ord x w, robust", &ol),
        &[
            0.12239063028173303,
            0.21816064705598151,
            0.14000675901248208,
            0.15427723266093946,
        ],
        1e-6,
        "oprobit, robust",
    );
    let ml = ["1:x", "1:w", "1:(Intercept)", "3:x", "3:w", "3:(Intercept)"];
    near_stata(
        &se(&mut e, "mlogit cat3 x w, vce(robust)", &ml),
        &[
            0.38351552296724112,
            0.4895574927561735,
            0.32138887171006247,
            0.34250707079036058,
            0.52036120642589179,
            0.29606214123558133,
        ],
        1e-6,
        "mlogit, robust",
    );
    near_stata(
        &se(&mut e, "mlogit cat3 x w, vce(cluster cl)", &ml),
        &[
            0.29665241080478849,
            0.40356906086796074,
            0.34477866338370322,
            0.39273126859588464,
            0.53316684176963314,
            0.31501637141386846,
        ],
        1e-6,
        "mlogit, clustered",
    );
    // the LR test of alpha: chibar2(01), half of chi2(1)'s tail
    let code = script(&format!("use \"{dir}/models.dta\"\nnbreg nb x\n"));
    let text = ok(&mut e, &code);
    assert!(
        text.contains("LR test of alpha=0: chibar2(01) = 26.69"),
        "{text}"
    );
    let v = numbers(
        &mut e,
        "lr <- stata_nbreg_lr(stata_estimates)",
        "lr[[\"chi2_c\"]], lr[[\"p\"]] / (0.5 * stats::pchisq(26.689103748660159, 1, lower.tail = FALSE))",
    );
    near_stata(
        &v,
        // Stata's chi2_c, and p as half of chi2(1)'s tail (Stata's e(p) is the model's test)
        &[26.689103748660159, 1.0],
        1e-6,
        "LR test of alpha",
    );
    let code = script(&format!("use \"{dir}/models.dta\"\nnbreg nb x, robust\n"));
    assert!(
        !ok(&mut e, &code).contains("LR test"),
        "not shown with robust errors, as in Stata"
    );
}

/// `predict`'s other statistics against Stata 19's stored values (floats, so
/// to float precision), for rows 1 and 3, in the estimation sample of
/// `regress y x w if g == 0`, and row 2, which is not.
#[test]
fn predict_statistics_agree_with_stata_19_in_and_out_of_the_sample() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    let stats = [
        "stdp",
        "stdf",
        "stdr",
        "rstandard",
        "rstudent",
        "cooksd",
        "leverage",
    ];
    let mut dofile = format!("use \"{dir}/models.dta\"\nregress y x w if g == 0\n");
    for s in stats {
        dofile.push_str(&format!("predict s_{s}, {s}\n"));
    }
    dofile.push_str(
        "logit yb x w\npredict l_stdp, stdp\npredict l_sc, score\npoisson cnt x, exposure(expo)\npredict p_stdp, stdp\npredict p_ir, ir\npredict p_sc, score\n",
    );
    ok(&mut e, &script(&dofile));
    let cols: Vec<String> = stats.iter().map(|s| format!("models$s_{s}[r]")).collect();
    let stata: [[f64; 7]; 3] = [
        [
            0.42897382378578186,
            2.1067731380462646,
            2.0175371170043945,
            -2.7202763557434082,
            -2.9999756813049316,
            0.1115124449133873,
            0.043252941220998764,
        ],
        [
            0.35639479756355286,
            2.0932013988494873,
            2.0316145420074463,
            1.2987469434738159,
            1.3113147020339966,
            0.017302464693784714,
            0.029854981228709221,
        ],
        [
            1.0440839529037476,
            2.3118360042572021,
            1.7788656949996948,
            0.58660697937011719,
            0.58133512735366821,
            0.039514612406492233,
            0.25622701644897461,
        ],
    ];
    for (r, want) in stata.iter().enumerate() {
        let got = numbers(&mut e, &format!("r <- {}", r + 1), &cols.join(", "));
        near_stata(
            &got,
            want,
            1e-7,
            &format!("predict after regress, row {}", r + 1),
        );
    }
    let got = numbers(
        &mut e,
        "0",
        "models$l_stdp[1], models$l_sc[1], models$l_stdp[2], models$l_sc[2], models$p_stdp[1], models$p_ir[1], models$p_sc[1], models$p_stdp[2], models$p_ir[2], models$p_sc[2]",
    );
    near_stata(
        &got,
        &[
            0.42340770363807678,
            0.59998035430908203,
            0.30726629495620728,
            0.35271617770195007,
            0.11630483716726303,
            0.86206150054931641,
            -0.094818085432052612,
            0.073511473834514618,
            1.3572994470596313,
            -0.33895650506019592,
        ],
        1e-7,
        "predict after logit and poisson",
    );
    // by definition: stdp after regress is predict's own se.fit, and leverage
    // in the sample is R's hatvalues
    let got = numbers(
        &mut e,
        "fit <- stats::lm(y ~ x + w, data = models[models$g == 0, ])",
        "models$s_stdp[1], unname(stats::predict(fit, models[1, ], se.fit = TRUE)$se.fit), models$s_leverage[1], unname(stats::hatvalues(fit)[1])",
    );
    near_stata(&got[..1], &got[1..2], 1e-7, "stdp is se.fit");
    near_stata(&got[2..3], &got[3..4], 1e-7, "leverage is hatvalues");
    assert!(
        refusal("use s\nregress y x, robust\npredict r, rstudent\n")
            .contains("option rstudent not allowed after robust estimation"),
        "residual statistics after robust regress, in Stata's words"
    );
}

/// `test` and `lincom` after ologit and mlogit (equations, cut points) and
/// with interaction coefficients, `estat hettest` with a varlist, and
/// `e(sample)`, against Stata 19 on `models`.
#[test]
fn test_lincom_hettest_and_esample_agree_with_stata_19() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    let use_ = format!("use \"{dir}/models.dta\"\n");
    // the last test or lincom line of the script, run again for its numbers
    let last = |e: &mut Engine, dofile: &str, begin: &str, show: &str| -> Vec<f64> {
        let code = script(&format!("{use_}{dofile}\n"));
        ok(e, &code);
        let line = line_of(&code, begin);
        numbers(e, &format!("res <- {line}"), show)
    };
    let chi2 = |e: &mut Engine, dofile: &str| {
        last(e, dofile, "stata_test(", "res[[\"chi2\"]], res[[\"df\"]]")
    };
    let lincom =
        |e: &mut Engine, dofile: &str| last(e, dofile, "stata_lincom(", "res[1, 1], res[1, 2]");
    // ologit: six digits, as its estimates (see the notes above)
    near_stata(
        &chi2(&mut e, "ologit ord x w\ntest x"),
        &[6.6500532543804587, 1.0],
        1e-6,
        "test x",
    );
    near_stata(
        &chi2(&mut e, "ologit ord x w\ntest x = w")[..1],
        &[0.23236045096730346],
        1e-6,
        "test x = w",
    );
    near_stata(
        &chi2(&mut e, "ologit ord x w\ntest _b[/cut1] = 0")[..1],
        &[6.7023293366669954],
        1e-6,
        "a cut point",
    );
    near_stata(
        &lincom(&mut e, "ologit ord x w\nlincom x + w"),
        &[0.86974469368032981, 0.41575438500015555],
        1e-6,
        "lincom x + w",
    );
    near_stata(
        &lincom(&mut e, "ologit ord x w\nlincom _b[/cut2] - _b[/cut1]"),
        &[1.2848561328022705, 0.24119739466004386],
        1e-6,
        "lincom of cut points",
    );
    // mlogit: the base outcome (2, the most frequent) has no equation
    let m = "mlogit cat3 x w\n";
    near_stata(
        &chi2(&mut e, &format!("{m}test x")),
        &[13.624281484051277, 2.0],
        1e-6,
        "x in every equation",
    );
    near_stata(
        &chi2(&mut e, &format!("{m}test [2]x = [3]x")),
        &[13.52585604224444, 1.0],
        1e-6,
        "[2]x is the base outcome's, a zero",
    );
    near_stata(
        &chi2(&mut e, &format!("{m}test [3]")),
        &[14.415873645763989, 2.0],
        1e-6,
        "an equation but its constant",
    );
    near_stata(
        &lincom(&mut e, &format!("{m}lincom [2]x - [3]x")),
        &[1.2749167694777452, 0.34665658405672817],
        1e-6,
        "lincom with the base outcome's zero",
    );
    near_stata(
        &lincom(&mut e, &format!("{m}lincom x")),
        &[-0.79525353414211297, 0.35450794336819591],
        1e-6,
        "the first equation's x",
    );
    near_stata(
        &lincom(&mut e, &format!("{m}lincom [3]_cons + [3]w")),
        &[-0.9609326129381851, 0.59459953119170661],
        1e-6,
        "a constant",
    );
    // Stata gives a test of nothing for the base outcome's equation; R says why
    let (outcome, text) = run(&mut e, &script(&format!("{use_}{m}test [2]x\n")));
    assert_eq!(outcome, Outcome::Error, "{text}");
    assert!(text.contains("base outcome"), "{text}");

    // interactions, in either order and with or without c.
    let f = |e: &mut Engine, dofile: &str| {
        last(
            e,
            dofile,
            "stata_test(",
            "res[[\"F\"]], res[[\"df\"]], res[[\"df_r\"]]",
        )
    };
    let r = "regress y c.x##i.g\n";
    for t in ["test 1.g#c.x", "test c.x#1.g", "test 1.g#x"] {
        near_stata(
            &f(&mut e, &format!("{r}{t}")),
            &[0.75376122119394684, 1.0, 76.0],
            1e-10,
            t,
        );
    }
    near_stata(
        &lincom(&mut e, &format!("{r}lincom x + 1.g#c.x")),
        &[2.0577600777372527, 0.340094179827649],
        1e-10,
        "lincom with an interaction",
    );
    near_stata(
        &f(&mut e, "regress y c.x##c.w\ntest c.x#c.w")[..1],
        &[0.72707260506675842],
        1e-10,
        "a continuous interaction",
    );

    // estat hettest with a varlist, and with rhs as well
    let h = |e: &mut Engine, dofile: &str| {
        last(
            e,
            dofile,
            "stata_hettest(",
            "res[[\"chi2\"]], res[[\"df\"]]",
        )
    };
    let reg = "regress y x w if g == 0\n";
    near_stata(
        &h(&mut e, &format!("{reg}estat hettest x")),
        &[0.039580253225814488, 1.0],
        1e-10,
        "hettest x",
    );
    near_stata(
        &h(&mut e, &format!("{reg}estat hettest x w, rhs")),
        &[0.16375070954466509, 2.0],
        1e-10,
        "hettest x w, rhs",
    );

    // e(sample): the forty rows of g == 0
    let code = script(&format!(
        "{use_}{reg}summarize x if e(sample)\ngen byte es = e(sample)\n"
    ));
    ok(&mut e, &code);
    let v = numbers(
        &mut e,
        "0",
        "sum(models$es), mean(models$x[models$es == 1])",
    );
    near_stata(&v, &[40.0, -0.37080000000000002], 1e-12, "e(sample)");
    // not once the rows may have moved
    assert!(
        refusal(&format!("{use_}{reg}sort x\nsummarize x if e(sample)\n")).contains("e(sample)")
    );
    assert!(refusal(&format!("{use_}summarize x if e(sample)\n")).contains("e(sample)"));
}

/// Weights in nbreg, ologit, oprobit and mlogit, margins and estat ic after a
/// weighted model, the other predict statistics after ologit, mlogit and
/// nbreg, and Stata's messages for by before the postestimation commands and
/// for residual statistics after robust errors; against Stata 19 on `models`
/// (with `f = mod(id, 3) + 1` and `a = id / 30`). Six digits for the
/// likelihood models (their Hessians are numerical, as noted above).
#[test]
fn the_p2b4_leftovers_agree_with_stata_19() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    let start = format!("use \"{dir}/models.dta\"\ngen f = mod(id, 3) + 1\ngen a = id / 30\n");
    let bse = |e: &mut Engine, cmd: &str, names: &[&str]| -> Vec<f64> {
        ok(e, &script(&format!("{start}{cmd}\n")));
        let b: Vec<String> = names
            .iter()
            .map(|n| format!("stata_b[[\"{n}\"]]"))
            .collect();
        let s: Vec<String> = names
            .iter()
            .map(|n| format!("sqrt(stata_vcov[\"{n}\", \"{n}\"])"))
            .collect();
        numbers(e, "0", &format!("{}, {}", b.join(", "), s.join(", ")))
    };
    let nb = ["x", "(Intercept)", "/lnalpha"];
    let ol = ["x", "w", "/cut1", "/cut2"];
    let ml = ["1:x", "1:w", "1:(Intercept)", "3:x", "3:w", "3:(Intercept)"];
    let cases: Vec<(&str, &[&str], Vec<f64>)> = vec![
        (
            "nbreg nb x [fw=f]",
            &nb,
            vec![
                0.43302236411285205,
                0.56245573347608502,
                -0.8671824759956408,
                0.072573766546981291,
                0.080535521404426144,
                0.26955239259898384,
            ],
        ),
        (
            "nbreg nb x [pw=a]",
            &nb,
            vec![
                0.32460605198270825,
                0.61468818409768577,
                -1.3969692457369167,
                0.10844445998844079,
                0.10658061208616615,
                0.44487508334421361,
            ],
        ),
        (
            "nbreg nb x [iw=a]",
            &nb,
            vec![
                0.32460605198270825,
                0.61468818409768577,
                -1.3969692457369167,
                0.077935622895645956,
                0.086783799678838328,
                0.4187030473503463,
            ],
        ),
        (
            "ologit ord x w [fw=f]",
            &ol,
            vec![
                0.512291515015314,
                0.38411228625408755,
                -0.61802046958406665,
                0.65748750789724697,
                0.14152394287079575,
                0.24956399695425111,
                0.17156713269500687,
                0.17177948814902461,
            ],
        ),
        (
            "ologit ord x w [pw=a]",
            &ol,
            vec![
                0.59113877915432056,
                0.076693555265671273,
                -0.63857503594509113,
                0.45598113699892773,
                0.25592159063886732,
                0.44018505332874475,
                0.26402392894714671,
                0.28608292943156965,
            ],
        ),
        (
            "oprobit ord x w [iw=a]",
            &ol,
            vec![
                0.35835518040024683,
                0.060813885374873838,
                -0.40277756004120585,
                0.26750062490527793,
                0.10580061925310323,
                0.2044738325735691,
                0.12881219396047622,
                0.12547065404859745,
            ],
        ),
        (
            "mlogit cat3 x w [fw=f]",
            &ml,
            vec![
                -0.62082980072421867,
                -0.29068207097250226,
                -0.77926087050975734,
                -1.2642026471867309,
                -0.61915031870820625,
                -0.59103600851653082,
                0.23234653659029145,
                0.36899577477175521,
                0.21911787876592156,
                0.24097161339489673,
                0.34976590283517545,
                0.21864158882499107,
            ],
        ),
        (
            "mlogit cat3 x w [pw=a]",
            &ml,
            vec![
                -0.81892760256469432,
                0.052020189448928313,
                -0.70330718258493596,
                -1.103864322571533,
                -0.58685052525389425,
                -0.35176168204886693,
                0.51546335756321915,
                0.56530478419176622,
                0.38550426671935106,
                0.3791331121749168,
                0.58750343163059149,
                0.31416536461085087,
            ],
        ),
    ];
    for (cmd, names, stata) in cases {
        near_stata(&bse(&mut e, cmd, names), &stata, 1e-6, cmd);
    }
    // the log likelihood and N with frequency weights
    let code = script(&format!("{start}nbreg nb x [fw=f]\n"));
    ok(&mut e, &code);
    let v = numbers(&mut e, "0", "as.numeric(stats::logLik(stata_estimates)), sum(stats::model.weights(stats::model.frame(stata_estimates)))");
    near_stata(
        &v,
        &[-283.23375073926366, 161.0],
        1e-8,
        "nbreg [fw=f]: ll and N",
    );
    // margins after a weighted model: weighted means
    let m = |e: &mut Engine, cmd: &str| -> Vec<f64> {
        let code = script(&format!("{start}{cmd}\n"));
        ok(e, &code);
        numbers(
            e,
            &format!("res <- {}", line_of(&code, "stata_margins(")),
            "res$b, sqrt(diag(res$V))",
        )
    };
    near_stata(
        &m(&mut e, "logit yb x w [pw=a]\nmargins"),
        &[0.48672839477585939, 0.05470685354595644],
        1e-8,
        "margins after logit [pw=a]",
    );
    near_stata(
        &m(&mut e, "logit yb x w [pw=a]\nmargins, dydx(x)"),
        &[0.18794105948402579, 0.042922811808571333],
        1e-8,
        "dydx after logit [pw=a]",
    );
    near_stata(
        &m(&mut e, "logit yb x w [fw=f]\nmargins"),
        &[0.52795031055885067, 0.032187920181026793],
        1e-8,
        "margins after logit [fw=f]",
    );
    near_stata(
        &m(&mut e, "regress y x w [aw=a]\nmargins"),
        &[0.93037541653572009, 0.21842792456267526],
        1e-9,
        "margins after regress [aw=a]",
    );
    // estat ic with frequency weights: N is their sum
    let ic = |e: &mut Engine, cmd: &str| -> Vec<f64> {
        let code = script(&format!("{start}{cmd}\nestat ic\n"));
        ok(e, &code);
        numbers(
            e,
            &format!("res <- {}", line_of(&code, "stata_estat_ic(")),
            "res$N, res[[\"ll(model)\"]], res$AIC, res$BIC",
        )
    };
    near_stata(
        &ic(&mut e, "logit yb x w [fw=f]"),
        &[
            161.0,
            -81.070644539023107,
            168.14128907804621,
            177.38550217299959,
        ],
        1e-9,
        "estat ic after logit [fw=f]",
    );
    near_stata(
        &ic(&mut e, "regress y x w [fw=f]"),
        &[
            161.0,
            -347.11677046274423,
            700.23354092548846,
            709.47775402044181,
        ],
        1e-10,
        "estat ic after regress [fw=f]",
    );
    // predict after ologit, mlogit (one equation) and nbreg (two scores), rows 1 and 2
    let code = script(&format!(
        "{start}ologit ord x w\npredict ox, xb\npredict os, stdp\nmlogit cat3 x w\npredict mx, xb outcome(3)\npredict ms, stdp outcome(3)\nnbreg nb x\npredict ns1 ns2, scores\n"
    ));
    ok(&mut e, &code);
    let v = numbers(&mut e, "0", "models$ox[1:2], models$os[1:2], models$mx[1:2], models$ms[1:2], models$ns1[1:2], models$ns2[1:2]");
    near_stata(
        &v,
        &[
            -0.69534045457839966,
            -0.0074417898431420326,
            0.26706698536872864,
            0.063568271696567535,
            1.2062942981719971,
            -0.37721768021583557,
            0.48310372233390808,
            0.3108803927898407,
            -0.14844171702861786,
            -1.0256229639053345,
            -0.17815661430358887,
            0.38868618011474609,
        ],
        1e-6,
        "predict after ologit, mlogit and nbreg (floats)",
    );
    // Stata's own messages
    for (cmd, why) in [
        (
            "sort g\nregress y x w\nby g: predict p1",
            "predict may not be combined with by",
        ),
        (
            "sort g\nregress y x w\nby g: test x",
            "test may not be combined with by",
        ),
        (
            "sort g\nregress y x w\nby g: margins",
            "margins may not be combined with by",
        ),
        (
            "regress y x w, robust\npredict r1, rstandard",
            "option rstandard not allowed after robust estimation",
        ),
        (
            "regress y x w, robust\npredict c1, cooksd",
            "option cooksd not allowed after robust estimation",
        ),
        ("ologit ord x w [aw=a]", "aweights not allowed"),
    ] {
        let got = refusal(&format!("use s\n{cmd}\n"));
        assert!(got.contains(why), "{cmd}: {got}");
    }
}
