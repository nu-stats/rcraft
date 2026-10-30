//! P2B.4, `margins` translated to R and run in a real R, and the quadratic
//! `c.x##c.x` that `margins, dydx(x)` is so often used after.
//!
//! Every margin is checked against the same number worked out by hand from
//! its formula, never against the helper's own numerical derivatives:
//!
//! - after logit, the average predicted probability (which for a logit with
//!   a constant is exactly the share of successes) and its delta-method
//!   error from the analytic gradient `mean(p (1 - p) x)`;
//! - the average marginal effect of a continuous variable,
//!   `mean(p (1 - p)) b`, and its gradient worked out by hand;
//! - a factor variable's effect as the change from the base level, with
//!   `ib1.g` turning the sign;
//! - after `regress y c.x##c.x`, a real quadratic (R would have read `x * x`
//!   as `x`), whose marginal effect is `b1 + 2 b2 mean(x)`, on t tests;
//! - `at()`, `atmeans`, `predict(xb)`, the exposure after poisson, an `if`,
//!   the robust variance, and the estimation sample only; a command that may
//!   have changed the data since the fit makes `margins` refuse, because
//!   Stata would average the changed rows.
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
        "rcraft-stata-margins-{}-{}",
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

/// What the `margins` line of the script returns: the margins, then their
/// standard errors.
fn margins(e: &mut Engine, code: &str) -> Vec<f64> {
    numbers(
        e,
        &format!("res <- {}", line_of(code, "stata_margins(")),
        "res$b, sqrt(diag(res$V))",
    )
}

/// The logit fit of `yb x w` by hand: design, probabilities and the
/// observed-information variance.
const LOGIT_PIECES: &str = "X <- cbind(1, models$x, models$w); b <- unname(stata_b); \
     p <- stats::plogis(drop(X %*% b)); V <- solve(t(X) %*% (X * (p * (1 - p))))";

#[test]
fn margins_after_logit_is_the_average_probability_with_its_delta_method_error() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    let code = script(&format!(
        "use \"{dir}/models.dta\"\nlogit yb x w\nmargins\n"
    ));
    let text = ok(&mut e, &code);
    assert!(text.contains("Predictive margins"), "{text}");
    assert!(text.contains("Pr(yb)"), "{text}");
    let got = margins(&mut e, &code);
    let want = numbers(
        &mut e,
        &format!("{LOGIT_PIECES}; J <- colMeans(X * (p * (1 - p)))"),
        "mean(models$yb), sqrt(drop(t(J) %*% V %*% J))",
    );
    all_close(&got, &want, 1e-8, "average probability and its error");
}

#[test]
fn dydx_of_a_continuous_variable_is_the_average_derivative() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    let code = script(&format!(
        "use \"{dir}/models.dta\"\nlogit yb x w\nmargins, dydx(x)\nmargins, dydx(*)\n"
    ));
    let text = ok(&mut e, &code);
    assert!(text.contains("Average marginal effects"), "{text}");
    let lines: Vec<&str> = code
        .lines()
        .filter(|l| l.starts_with("stata_margins("))
        .collect();
    let one = numbers(
        &mut e,
        &format!("res <- {}", lines.first().copied().unwrap_or("")),
        "res$b, sqrt(diag(res$V))",
    );
    // the gradient of mean(p (1 - p)) b_x in b, by hand
    let want = numbers(
        &mut e,
        &format!(
            "{LOGIT_PIECES}; s <- p * (1 - p); \
             J <- colMeans(X * (s * (1 - 2 * p))) * b[2] + c(0, mean(s), 0)"
        ),
        "mean(s) * b[2], sqrt(drop(t(J) %*% V %*% J))",
    );
    all_close(&one, &want, 1e-7, "average marginal effect of x");
    // dydx(*) is every covariate: x, then w
    let all = numbers(
        &mut e,
        &format!("res <- {}", lines.get(1).copied().unwrap_or("")),
        "res$b",
    );
    let want = numbers(
        &mut e,
        &format!("{LOGIT_PIECES}; s <- p * (1 - p)"),
        "mean(s) * b[2], mean(s) * b[3]",
    );
    all_close(&all, &want, 1e-7, "every covariate's effect");
}

#[test]
fn a_factor_variables_effect_is_the_change_from_its_base_level() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    let code = script(&format!(
        "use \"{dir}/models.dta\"\nlogit yb x i.g\nmargins g\nmargins, dydx(g)\n"
    ));
    ok(&mut e, &code);
    let lines: Vec<&str> = code
        .lines()
        .filter(|l| l.starts_with("stata_margins("))
        .collect();
    let pieces = "b <- unname(stata_b); X0 <- cbind(1, models$x, 0); X1 <- cbind(1, models$x, 1); \
         p0 <- stats::plogis(drop(X0 %*% b)); p1 <- stats::plogis(drop(X1 %*% b)); \
         X <- cbind(1, models$x, models$g); p <- stats::plogis(drop(X %*% b)); \
         V <- solve(t(X) %*% (X * (p * (1 - p)))); \
         J0 <- colMeans(X0 * (p0 * (1 - p0))); J1 <- colMeans(X1 * (p1 * (1 - p1))); D <- J1 - J0";
    let at_levels = numbers(
        &mut e,
        &format!("res <- {}", lines.first().copied().unwrap_or("")),
        "res$b, sqrt(diag(res$V))",
    );
    let want = numbers(
        &mut e,
        pieces,
        "mean(p0), mean(p1), sqrt(drop(t(J0) %*% V %*% J0)), sqrt(drop(t(J1) %*% V %*% J1))",
    );
    all_close(&at_levels, &want, 1e-8, "the margin at each level of g");
    let change = numbers(
        &mut e,
        &format!("res <- {}", lines.get(1).copied().unwrap_or("")),
        "res$b, sqrt(diag(res$V))",
    );
    let want = numbers(
        &mut e,
        pieces,
        "mean(p1) - mean(p0), sqrt(drop(t(D) %*% V %*% D))",
    );
    all_close(&change, &want, 1e-8, "the change from level 0 to level 1");

    // with ib1.g the base is 1, so the change is the other way round
    let code = script(&format!(
        "use \"{dir}/models.dta\"\nlogit yb x ib1.g\nmargins, dydx(g)\n"
    ));
    ok(&mut e, &code);
    let flipped = margins(&mut e, &code);
    close(
        at(&flipped, 0),
        -at(&change, 0),
        1e-8,
        "the change from level 1 to 0",
    );
    close(at(&flipped, 1), at(&change, 1), 1e-8, "the same error");
}

#[test]
fn a_quadratic_is_a_quadratic_and_its_effect_is_b1_plus_2_b2_x() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    let code = script(&format!(
        "use \"{dir}/models.dta\"\nregress y c.x##c.x w\nmargins, dydx(x)\n"
    ));
    ok(&mut e, &code);
    // the square is really in the model, as Stata's c.x##c.x puts it there
    let got = numbers(&mut e, "0", "unname(stata_b)");
    let want = numbers(
        &mut e,
        "X <- cbind(1, models$x, models$x^2, models$w); bh <- solve(t(X) %*% X, t(X) %*% models$y)",
        "as.numeric(bh)",
    );
    all_close(&got, &want, 1e-9, "the quadratic's coefficients");

    let m = numbers(
        &mut e,
        &format!("res <- {}", line_of(&code, "stata_margins(")),
        "res$b, sqrt(diag(res$V)), res$table[[4]]",
    );
    let hand = numbers(
        &mut e,
        "b <- unname(stata_b); V <- stats::vcov(stata_estimates); J <- c(0, 1, 2 * mean(models$x), 0); \
         est <- b[2] + 2 * b[3] * mean(models$x); se <- sqrt(drop(t(J) %*% V %*% J))",
        "est, se, 2 * stats::pt(-abs(est / se), 76)",
    );
    all_close(
        &m,
        &hand,
        1e-8,
        "the effect of x, its error and its t test on 76 df",
    );
}

#[test]
fn at_and_atmeans_set_the_covariates_before_averaging() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    let code = script(&format!(
        "use \"{dir}/models.dta\"\nlogit yb x w\nmargins, at(x=(0 1))\nmargins, at(x=0) at(x=1)\n"
    ));
    ok(&mut e, &code);
    let lines: Vec<&str> = code
        .lines()
        .filter(|l| l.starts_with("stata_margins("))
        .collect();
    let want = numbers(
        &mut e,
        "b <- unname(stata_b)",
        "mean(stats::plogis(b[1] + b[3] * models$w)), mean(stats::plogis(b[1] + b[2] + b[3] * models$w))",
    );
    for l in &lines {
        let got = numbers(&mut e, &format!("res <- {l}"), "res$b");
        all_close(&got, &want, 1e-9, "the margin with x set to 0 and to 1");
    }

    let code = script(&format!(
        "use \"{dir}/models.dta\"\nregress y x w\nmargins, atmeans\n"
    ));
    ok(&mut e, &code);
    let got = margins(&mut e, &code);
    // Stata sets each covariate to the float of its mean (checked in Stata 19)
    let want = numbers(
        &mut e,
        "fl <- function(v) readBin(writeBin(as.numeric(v), raw(), size = 4), \"double\", size = 4, n = length(v)); \
         b <- unname(stata_b); a <- c(1, fl(mean(models$x)), fl(mean(models$w))); V <- stats::vcov(stata_estimates)",
        "sum(a * b), sqrt(drop(t(a) %*% V %*% a)), mean(models$y)",
    );
    close(
        at(&got, 0),
        at(&want, 0),
        1e-12,
        "the prediction at the float means",
    );
    close(at(&got, 1), at(&want, 1), 1e-12, "its error");
    // so it is not quite mean(y), which the exact means would give
    let gap = (at(&got, 0) - at(&want, 2)).abs();
    assert!(
        gap > 1e-10 && gap < 1e-7,
        "the float means move it slightly: {gap}"
    );
}

/// Stata 19 SE on this same data, 2026-10-30: `atmeans` and `at()` with
/// values that are not exact floats.
#[test]
fn atmeans_and_at_agree_with_stata_19() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    let cases: [(&str, &[f64], f64); 6] = [
        (
            "regress y x w\nmargins, atmeans",
            &[1.1357311297706145, 0.22896587017807754],
            1e-12,
        ),
        (
            "logit yb x w\nmargins, atmeans",
            &[0.51397811967737173, 0.069298316784833891],
            1e-9,
        ),
        (
            "logit yb x w\nmargins, dydx(x) atmeans",
            &[0.3405334589629081, 0.089436156016400661],
            1e-9,
        ),
        (
            "regress y x w\nmargins, at(x=0.1)",
            &[1.5624242246427948],
            1e-12,
        ),
        (
            "regress y x w\nmargins, at(x=0.1 w=0.3)",
            &[1.2698690966304336],
            1e-12,
        ),
        // Stata's own logit after i.g stops a little earlier than R's
        (
            "logit yb x i.g\nmargins, at(x=0.1)",
            &[0.57381421771106866],
            1e-6,
        ),
    ];
    for (cmd, stata, tol) in cases {
        let code = script(&format!("use \"{dir}/models.dta\"\n{cmd}\n"));
        ok(&mut e, &code);
        let got = margins(&mut e, &code);
        for (i, s) in stata.iter().enumerate() {
            let g = at(&got, i);
            assert!(
                (g - s).abs() <= tol * s.abs().max(1.0),
                "{cmd}: {g} against Stata's {s}"
            );
        }
    }
}

#[test]
fn margins_after_poisson_counts_the_exposure_and_keeps_to_an_if() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    let code = script(&format!(
        "use \"{dir}/models.dta\"\npoisson cnt x, exposure(expo)\nmargins\nmargins, predict(xb)\nmargins if g == 1\n"
    ));
    ok(&mut e, &code);
    let lines: Vec<&str> = code
        .lines()
        .filter(|l| l.starts_with("stata_margins("))
        .collect();
    let got: Vec<f64> = lines
        .iter()
        .map(|l| at(&numbers(&mut e, &format!("res <- {l}"), "res$b"), 0))
        .collect();
    let want = numbers(
        &mut e,
        "b <- unname(stata_b); k <- models$g == 1",
        "mean(models$expo * exp(b[1] + b[2] * models$x)), \
         mean(b[1] + b[2] * models$x + log(models$expo)), \
         mean(models$expo[k] * exp(b[1] + b[2] * models$x[k]))",
    );
    all_close(
        &got,
        &want,
        1e-9,
        "events, the linear prediction, and the rows the if keeps",
    );
}

#[test]
fn margins_after_robust_uses_the_robust_variance() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    let code = script(&format!(
        "use \"{dir}/models.dta\"\nlogit yb x w, robust\nmargins, dydx(x)\n"
    ));
    ok(&mut e, &code);
    let got = margins(&mut e, &code);
    let want = numbers(
        &mut e,
        &format!(
            "{LOGIT_PIECES}; s <- p * (1 - p); sc <- X * (models$yb - p); n <- nrow(X); \
             Vr <- V %*% crossprod(sc) %*% V * n / (n - 1); \
             J <- colMeans(X * (s * (1 - 2 * p))) * b[2] + c(0, mean(s), 0)"
        ),
        "mean(s) * b[2], sqrt(drop(t(J) %*% Vr %*% J)), sqrt(drop(t(J) %*% V %*% J))",
    );
    close(at(&got, 0), at(&want, 0), 1e-7, "the effect");
    close(at(&got, 1), at(&want, 1), 1e-7, "its robust error");
    assert!(
        (at(&got, 1) - at(&want, 2)).abs() > 1e-6,
        "the robust error is not the ordinary one"
    );
}

#[test]
fn margins_averages_the_estimation_sample_only() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    // a command that leaves the data alone may come between
    let code = script(&format!(
        "use \"{dir}/models.dta\"\nregress y x if g == 1\nsummarize y\nmargins\n"
    ));
    let text = ok(&mut e, &code);
    assert!(text.contains("Number of obs = 40"), "{text}");
    let got = margins(&mut e, &code);
    let want = numbers(
        &mut e,
        "b <- unname(stata_b); k <- models$g == 1",
        "mean(b[1] + b[2] * models$x[k])",
    );
    close(
        at(&got, 0),
        at(&want, 0),
        1e-9,
        "the average over the forty rows of the fit",
    );
}

#[test]
fn what_margins_does_not_translate_says_why() {
    assert!(
        refusal("use \"d.dta\"\nregress y x\nmargins, vce(unconditional)\n")
            .contains("allowed only after estimation with robust"),
        "vce(unconditional) after the model's own errors, in Stata's words"
    );
    assert!(
        refusal("use \"d.dta\"\nlogit y x i.g\nmargins g, contrast(effects)\n")
            .contains("suboptions"),
        "contrast() with suboptions"
    );
    assert!(
        refusal("use \"d.dta\"\nlogit y x\nmargins, at((asobserved) x)\n").contains("asobserved"),
        "an at() statistic that is not translated"
    );
    assert!(
        refusal("use \"d.dta\"\nlogit y x\nmargins x\n").contains("factor variable"),
        "a continuous variable in the marginlist"
    );
    assert!(
        refusal("use \"d.dta\"\nlogit y x i.g\nmargins g, dydx(x) pwcompare\n")
            .contains("marginal effects"),
        "contrasts of effects"
    );
    assert!(
        refusal("use \"d.dta\"\nmargins\n").contains("needs a model"),
        "margins with no model"
    );
    // Stata averages the rows e(sample) marks, which a drop moves (a sort does not matter)
    assert!(
        refusal("use \"d.dta\"\nregress y x\ndrop if x > 1\nmargins\n")
            .contains("changed the rows"),
        "rows dropped after the fit"
    );
    assert!(
        refusal("use \"d.dta\"\nregress y x\nfoo bar\nmargins\n").contains("is not translated yet"),
        "an untranslated line is refused itself"
    );
    let after = translate(&read("use \"d.dta\"\nregress y x\nfoo bar\nmargins\n"));
    assert!(
        after
            .lines
            .last()
            .and_then(|l| l.result.as_ref().err())
            .is_some_and(|e| e.reason.contains("changed the rows")),
        "and counts as a change, since it still runs in Stata"
    );
}

/// The rest of margins against Stata 19 on `models` (with `h = mod(id, 3)`):
/// over(), at() statistics, elasticities, a marginlist with #, atmeans with a
/// factor variable, pwcompare, contrast, r., post, the outcomes after ologit
/// and mlogit, and the current values after the data changed. Margins and
/// then standard errors; after logit, ologit and mlogit to 6 digits (Stata's
/// own fits stop earlier than R's, see the notes), after regress to 9 (the
/// Jacobian of the margins in b is numerical).
#[test]
fn the_rest_of_margins_agrees_with_stata_19() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    let use_ = format!("use \"{dir}/models.dta\"\ngen h = mod(id, 3)\n");
    let l = "logit yb x w i.g\n";
    let r = "regress y x i.g i.h\n";
    let cases: Vec<(String, Vec<f64>, f64)> = vec![
        (
            format!("{l}margins, over(g)"),
            vec![
                0.47500000000044607,
                0.54999999999682758,
                0.063053993863915428,
                0.064401873213277616,
            ],
            1e-6,
        ),
        (
            format!("{l}margins, at((mean) x w=0.5)"),
            vec![0.29906261156329061, 0.082675811726279608],
            1e-6,
        ),
        (
            format!("{l}margins, at((median) x (p25) w)"),
            vec![0.70036813871337988, 0.077537066337290614],
            1e-6,
        ),
        (
            format!("{l}margins, eyex(x)"),
            vec![-0.42376061655653424, 0.16157893662109019],
            1e-6,
        ),
        (
            format!("{l}margins, dyex(x)"),
            vec![-0.025767667584057186, 0.016951261222154856],
            1e-6,
        ),
        (
            format!("{l}margins, eydx(x)"),
            vec![0.66610229520473663, 0.18819605537535844],
            1e-6,
        ),
        (
            format!("{l}margins g, atmeans"),
            vec![
                0.51730752087508891,
                0.51077191918759945,
                0.10029653170111062,
                0.09840772724847878,
            ],
            1e-6,
        ),
        (
            format!("{l}margins, atmeans"),
            vec![0.51404032022408286, 0.069308336264685216],
            1e-6,
        ),
        (
            format!("{l}margins, dydx(x) atmeans"),
            vec![0.34132156853525353, 0.091160864359315308],
            1e-6,
        ),
        (
            format!("{l}margins g, pwcompare"),
            vec![0.51042053664913012 - 0.51466813532433708, 0.0924826],
            1e-5,
        ),
        (
            format!("{r}margins g#h"),
            vec![
                0.53223261122581178,
                1.1339681494993243,
                0.53880585599388087,
                1.3286212323603368,
                1.9303567706338494,
                1.3351944771284057,
                0.48070645886583846,
                0.4626586117378565,
                0.4707662941978667,
                0.47214316851828109,
                0.47465090862734288,
                0.46715921588117898,
            ],
            1e-9,
        ),
        (
            format!("{r}margins r.h"),
            vec![
                0.60173553827351256,
                0.0065732447680690864,
                0.57658675175521656,
                0.57726962658350467,
            ],
            1e-9,
        ),
        (
            "ologit ord x w\nmargins".to_string(),
            vec![
                0.37353847000299228,
                0.28721357182919444,
                0.33924795816781328,
                0.052360240711368078,
                0.050468238267759397,
                0.050315501722370676,
            ],
            1e-6,
        ),
        (
            "ologit ord x w\nmargins, dydx(x)".to_string(),
            vec![
                -0.1152024309051382,
                0.003994061864939653,
                0.11120836904019855,
                0.039933733396693219,
                0.012028948592611648,
                0.038784455659196582,
            ],
            1e-6,
        ),
        (
            "ologit ord x w\nmargins, predict(outcome(2))".to_string(),
            vec![0.28721357182919444, 0.050468238267759397],
            1e-6,
        ),
        (
            "mlogit cat3 x w\nmargins".to_string(),
            vec![
                0.20000000050428998,
                0.43749999983893434,
                0.36249999965677565,
                0.044403112347908454,
                0.049267543823752216,
                0.048471674092946851,
            ],
            1e-6,
        ),
        (
            "mlogit cat3 x w\nmargins, dydx(w) predict(outcome(3))".to_string(),
            vec![-0.10219746700157813, 0.080350356839752085],
            1e-6,
        ),
        // the data changed after the fit: Stata averages the current values
        (
            "regress y x w\nreplace x = x + 1\nmargins".to_string(),
            vec![3.0623282043722764, 0.30995184856485197],
            1e-9,
        ),
    ];
    for (cmd, stata, tol) in cases {
        let code = script(&format!("{use_}{cmd}\n"));
        ok(&mut e, &code);
        let got = margins(&mut e, &code);
        assert_eq!(got.len(), stata.len(), "{cmd}: {got:?}");
        for (g, s) in got.iter().zip(&stata) {
            assert!(
                (g - s).abs() <= tol * s.abs().max(1.0),
                "{cmd}: {g} against Stata's {s}"
            );
        }
    }
    // contrast: Stata's joint F that the three margins of h are equal
    let code = script(&format!("{use_}{r}margins h, contrast\n"));
    ok(&mut e, &code);
    let v = numbers(
        &mut e,
        &format!("res <- {}", line_of(&code, "stata_margins(")),
        "res$F, res$df, res$df_r, res$p",
    );
    all_close(
        &v,
        &[0.73140167233399322, 2.0, 75.0, 0.48463442521357269],
        1e-10,
        "contrast",
    );
    // post: the margins are e(b) and e(V), and lincom reads them by Stata's names
    let code = script(&format!("{use_}{r}margins h, post\nlincom 2.h - 1.h\n"));
    ok(&mut e, &code);
    let v = numbers(
        &mut e,
        &format!("res <- {}", line_of(&code, "stata_lincom(")),
        "stata_b[[\"1.h\"]], sqrt(stata_vcov[\"1.h\", \"1.h\"]), res[1, 1], res[1, 2]",
    );
    all_close(
        &v,
        &[
            1.5321624600665869,
            0.40252848417697484,
            -0.59516229350544347,
            0.56907600437296579,
        ],
        1e-9,
        "margins, post then lincom",
    );
}

/// margins, vce(unconditional) against Stata 19: each row's part of the
/// margin and its part through b, scaled by the model's own small-sample
/// factor (HC1's N / (N - k) after regress, N / (N - 1) after logit, and the
/// cluster factors), decoded from these very numbers.
#[test]
fn unconditional_margins_agree_with_stata_19() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    let cases: [(&str, &[f64]); 7] = [
        (
            "regress y x w i.g, vce(robust)\nmargins, vce(unconditional)",
            &[1.1357311360696081, 0.33855976838003143],
        ),
        (
            "regress y x w i.g, vce(robust)\nmargins g, vce(unconditional)",
            &[
                0.70401534344298999,
                1.5674469286962263,
                0.3975323387431754,
                0.39344753641863667,
            ],
        ),
        (
            "regress y x w i.g, vce(robust)\nmargins, dydx(x) vce(unconditional)",
            &[1.8379643645175932, 0.26285938918129997],
        ),
        (
            "regress y x w i.g, vce(cluster cl)\nmargins, vce(unconditional)",
            &[1.1357311360696081, 0.42528032413685596],
        ),
        (
            "logit yb x w, vce(robust)\nmargins, vce(unconditional)",
            &[0.51249999999849805, 0.056236812800193887],
        ),
        (
            "logit yb x w, vce(robust)\nmargins, dydx(x) vce(unconditional)",
            &[0.22145818978797666, 0.042608756204886376],
        ),
        (
            "logit yb x w, vce(cluster cl)\nmargins, vce(unconditional)",
            &[0.51249999999849805, 0.058056315025031599],
        ),
    ];
    for (cmd, stata) in cases {
        let code = script(&format!("use \"{dir}/models.dta\"\n{cmd}\n"));
        ok(&mut e, &code);
        let got = margins(&mut e, &code);
        assert_eq!(got.len(), stata.len(), "{cmd}: {got:?}");
        for (g, s) in got.iter().zip(stata) {
            assert!(
                (g - s).abs() <= 1e-8 * s.abs().max(1.0),
                "{cmd}: {g} against Stata's {s}"
            );
        }
    }
}

/// A number no other call in this run has had: the tests in a file run at
/// once, and each writing its data to the same folder let one read a file
/// another was still writing.
fn fresh() -> usize {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}
