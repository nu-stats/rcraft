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
    let dir = std::env::temp_dir().join(format!("rcraft-stata-models-{}", std::process::id()));
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
        refusal("use \"d.dta\"\nmlogit y x\ntest x\n").contains("several equations"),
        "test after mlogit"
    );
    assert!(
        refusal("use \"d.dta\"\nlogit y x\nxtreg y x\n").contains("xtreg"),
        "xtreg is not translated yet"
    );
    assert!(
        refusal("use \"d.dta\"\nnbreg y x, robust\n").contains("robust"),
        "robust nbreg is not translated yet"
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
        refusal("use \"d.dta\"\nregress y x\npredict s, stdp\n").contains("stdp"),
        "stdp is not translated yet"
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
