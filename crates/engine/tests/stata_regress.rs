//! P2B.4, the models: `regress` translated to R and run in a real R.
//!
//! The point of these cases is not that R can fit a linear model, which it
//! plainly can, but that the translation gives *Stata's* numbers. Each figure
//! is checked against the same figure worked out a second way, from the matrix
//! algebra, so a change in what `sandwich` does would be caught:
//!
//! - the coefficients against the normal equations solved by hand;
//! - `robust` against HC1 worked out from the residuals, and shown to differ
//!   from HC0 and HC3, so the choice of correction is really being made;
//! - `vce(cluster cl)` against the clustered sandwich with Stata's two
//!   corrections, `G / (G - 1)` and `(N - 1) / (N - K)`, worked out by hand,
//!   and its t tests and intervals on `G - 1` degrees of freedom;
//! - `i.g` against a factor whose base level is the lowest value, as Stata's
//!   base is.
//!
//! The standard errors are read from `stata_vcov`, which is what the
//! translation leaves in R for `test` and `lincom` to use.
//!
//! Without a supported R, or without haven, dplyr and sandwich, the
//! test is skipped with a note.

#![allow(clippy::panic)]

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
         requireNamespace(\"sandwich\", quietly = TRUE))",
    );
    if have != Outcome::Ok {
        eprintln!("skipped: the R packages dplyr, haven and sandwich are needed");
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

/// Twenty rows in five clusters, with residuals that are not zero.
fn fixture(e: &mut Engine) -> String {
    let dir = std::env::temp_dir().join(format!(
        "rcraft-stata-regress-{}-{}",
        std::process::id(),
        fresh()
    ));
    let _ = std::fs::create_dir_all(&dir);
    let d = dir.display().to_string().replace('\\', "/");
    ok(
        e,
        &format!(
            "local({{
  x <- c(2, 5, 1, 8, 3, 9, 4, 7, 6, 10, 12, 11, 15, 13, 18, 14, 20, 16, 19, 17)
  w <- c(1, 3, 2, 4, 2, 1, 4, 3, 1, 2, 3, 4, 2, 3, 1, 4, 3, 1, 2, 4)
  off <- c(0.7, -1.3, 2.1, -0.4, 1.8, -2.2, 0.3, 1.1, -1.7, 0.9,
           2.4, -0.8, 1.2, -1.9, 0.6, 2, -0.5, 1.4, -2.3, 0.2)
  haven::write_dta(data.frame(
    id = 1:20,
    x = x,
    w = w,
    y = 3 + 1.5 * x - 2 * w + off,
    cl = as.numeric(rep(1:5, each = 4)),
    g = as.numeric(rep(c(0, 1), 10))
  ), '{d}/panel.dta')
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

fn refusal(dofile: &str) -> String {
    translate(&read(dofile))
        .lines
        .iter()
        .find_map(|l| l.result.as_ref().err().map(|e| e.reason.clone()))
        .unwrap_or_default()
}

/// The design matrix, residuals and sizes, for the checks worked out by hand.
const PIECES: &str = "X <- cbind(1, panel$x, panel$w); yv <- panel$y; \
     bhat <- solve(t(X) %*% X, t(X) %*% yv); ev <- as.numeric(yv - X %*% bhat); \
     n <- nrow(X); k <- ncol(X); xtxi <- solve(t(X) %*% X)";

#[test]
fn regress_gives_the_coefficients_the_normal_equations_give() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    let code = script(&format!("use \"{dir}/panel.dta\"\nregress y x w\n"));
    let text = ok(&mut e, &code);
    assert!(text.contains("Coefficients"), "{text}");
    // Stata always shows a confidence interval, so the translation prints one
    assert!(text.contains("2.5 %") || text.contains("97.5 %"), "{text}");

    let got = numbers(&mut e, "0", "unname(stats::coef(stata_estimates))");
    let want = numbers(&mut e, PIECES, "as.numeric(bhat)");
    for i in 0..3 {
        close(at(&got, i), at(&want, i), 1e-9, "coefficient");
    }
    // the slope on x, which the data were built around
    close(at(&got, 1), 1.5, 0.1, "the slope on x");
}

#[test]
fn robust_standard_errors_are_hc1_and_not_hc0_or_hc3() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    let code = script(&format!("use \"{dir}/panel.dta\"\nregress y x w, robust\n"));
    let text = ok(&mut e, &code);
    assert!(text.contains("Std. err."), "{text}");

    let got = numbers(&mut e, "0", "sqrt(diag(stata_vcov))");
    // HC1 worked out from the residuals: the sandwich, scaled by n / (n - k)
    let want = numbers(
        &mut e,
        &format!(
            "{PIECES}; meat <- t(X) %*% diag(ev^2) %*% X; \
             v1 <- xtxi %*% meat %*% xtxi * n / (n - k)"
        ),
        "sqrt(diag(v1))",
    );
    for i in 0..3 {
        close(at(&got, i), at(&want, i), 1e-9, "HC1 standard error");
    }
    // HC0 has no such scaling, so it is a different number: the correction is real
    let hc0 = numbers(
        &mut e,
        "v <- sandwich::vcovHC(stata_estimates, type = \"HC0\")",
        "sqrt(diag(v))",
    );
    assert!(
        (at(&hc0, 1) - at(&got, 1)).abs() > 1e-6,
        "HC0 should differ from HC1"
    );
    let hc3 = numbers(
        &mut e,
        "v <- sandwich::vcovHC(stata_estimates, type = \"HC3\")",
        "sqrt(diag(v))",
    );
    assert!(
        (at(&hc3, 1) - at(&got, 1)).abs() > 1e-6,
        "R's own default HC3 should differ from Stata's HC1"
    );
}

#[test]
fn clustered_standard_errors_carry_statas_two_small_sample_corrections() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    let code = script(&format!(
        "use \"{dir}/panel.dta\"\nregress y x w, vce(cluster cl)\n"
    ));
    let text = ok(&mut e, &code);
    assert!(text.contains("Std. err."), "{text}");

    let got = numbers(&mut e, "0", "sqrt(diag(stata_vcov))");
    // the clustered sandwich by hand, with G / (G - 1) and (N - 1) / (N - K)
    let want = numbers(
        &mut e,
        &format!(
            "{PIECES}; gs <- unique(panel$cl); \
             meat <- Reduce(`+`, lapply(gs, function(g) {{ \
               i <- panel$cl == g; xu <- t(X[i, , drop = FALSE]) %*% ev[i]; xu %*% t(xu) }})); \
             G <- length(gs); \
             vc <- xtxi %*% meat %*% xtxi * (G / (G - 1)) * ((n - 1) / (n - k))"
        ),
        "sqrt(diag(vc))",
    );
    for i in 0..3 {
        close(at(&got, i), at(&want, i), 1e-9, "clustered standard error");
    }
    // and they are not the plain robust ones
    let plain = numbers(
        &mut e,
        "v <- sandwich::vcovHC(stata_estimates, type = \"HC1\")",
        "sqrt(diag(v))",
    );
    assert!(
        (at(&plain, 1) - at(&got, 1)).abs() > 1e-6,
        "clustering should change the standard errors"
    );

    // Stata's t tests after vce(cluster) have G - 1 = 4 degrees of freedom,
    // not N - K = 17, so the interval is wider than the one R would give
    let df = numbers(&mut e, "0", "stata_df");
    assert_eq!(
        at(&df, 0),
        4.0,
        "five clusters leave four degrees of freedom"
    );
    let half = numbers(
        &mut e,
        "tab <- stata_coefs(stata_b, stata_vcov, stata_df)",
        "tab[[6]] - tab[[1]]",
    );
    for i in 0..3 {
        close(
            at(&half, i),
            stats_qt(&mut e, 4.0) * at(&want, i),
            1e-9,
            "the interval's half width on G - 1 degrees of freedom",
        );
    }
    let p = numbers(
        &mut e,
        "tab <- stata_coefs(stata_b, stata_vcov, stata_df)",
        "tab[[4]]",
    );
    let by_hand = numbers(
        &mut e,
        "tab <- stata_coefs(stata_b, stata_vcov, stata_df)",
        "2 * stats::pt(-abs(tab[[3]]), 4)",
    );
    let r_default = numbers(
        &mut e,
        "tab <- stata_coefs(stata_b, stata_vcov, stata_df)",
        "2 * stats::pt(-abs(tab[[3]]), 17)",
    );
    for i in 0..3 {
        close(at(&p, i), at(&by_hand, i), 1e-12, "p-value on G - 1");
    }
    assert!(
        (at(&p, 1) - at(&r_default, 1)).abs() > 1e-6,
        "the p-value on N - K would be a different number"
    );
}

/// The t quantile for a 95% interval.
fn stats_qt(e: &mut Engine, df: f64) -> f64 {
    at(&numbers(e, "0", &format!("stats::qt(0.975, {df})")), 0)
}

#[test]
fn factor_notation_leaves_out_the_lowest_level_as_statas_base_is() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    let code = script(&format!(
        "use \"{dir}/panel.dta\"\nregress y x i.g\nregress y c.x##i.g\nregress y x, noconstant\n"
    ));
    let text = ok(&mut e, &code);
    assert!(
        text.contains("factor(g)1"),
        "the base level 0 is left out: {text}"
    );
    assert!(
        !text.contains("factor(g)0"),
        "the base level should not appear: {text}"
    );

    // i.g is the same model as a factor fitted directly
    let code = script(&format!("use \"{dir}/panel.dta\"\nregress y x i.g\n"));
    ok(&mut e, &code);
    let got = numbers(&mut e, "0", "unname(stats::coef(stata_estimates))");
    let want = numbers(
        &mut e,
        "m <- stats::lm(y ~ x + factor(g), data = panel)",
        "unname(stats::coef(m))",
    );
    for i in 0..3 {
        close(at(&got, i), at(&want, i), 1e-9, "i.g coefficient");
    }
    // noconstant leaves the intercept out
    let code = script(&format!(
        "use \"{dir}/panel.dta\"\nregress y x, noconstant\n"
    ));
    ok(&mut e, &code);
    let names = ok(&mut e, "cat(names(stats::coef(stata_estimates)), \"\\n\")");
    assert!(!names.contains("Intercept"), "{names}");
}

#[test]
fn regress_only_uses_the_rows_an_if_keeps() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    let code = script(&format!("use \"{dir}/panel.dta\"\nregress y x if g == 1\n"));
    ok(&mut e, &code);
    let v = numbers(&mut e, "0", "stats::nobs(stata_estimates)");
    assert_eq!(at(&v, 0), 10.0, "half the rows have g == 1");
}

#[test]
fn regress_says_why_it_refuses_what_it_refuses() {
    assert!(
        refusal("use \"d.dta\"\nregress y x, vce(bootstrap)\n").contains("vce(bootstrap)"),
        "an unknown vce should be named"
    );
    assert!(
        refusal("use \"d.dta\"\nregress y L.x\n").contains("lags"),
        "lags are not translated"
    );
    assert!(
        refusal("use \"d.dta\"\nregress y x, beta\n").contains("beta"),
        "an option that is not translated should be named"
    );
    assert!(
        refusal("use \"d.dta\"\nregress\n").contains("needs a variable"),
        "regress with nothing should say so"
    );
    assert!(
        refusal("use \"d.dta\"\nregress y x, vce(cluster a b)\n").contains("one variable"),
        "vce(cluster) takes one variable"
    );
}

/// A number no other call in this run has had: the tests in a file run at
/// once, and each writing its data to the same folder let one read a file
/// another was still writing.
fn fresh() -> usize {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}
