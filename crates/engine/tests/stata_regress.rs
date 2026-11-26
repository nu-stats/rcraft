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
    // Stata's own table, with its confidence interval
    assert!(text.contains("Coefficient  Std. err."), "{text}");
    assert!(text.contains("[95% conf. interval]"), "{text}");

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
    assert!(
        text.contains("Robust") && text.contains("std. err."),
        "{text}"
    );

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
        text.contains("         1.g |"),
        "the base level 0 is left out: {text}"
    );
    assert!(
        !text.contains("0.g |") && !text.contains("0b.g |"),
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
        refusal("use \"d.dta\"\nregress y L.x\n").contains("tsset"),
        "lags need tsset, which a file read here may not carry"
    );
    assert!(
        refusal("use \"d.dta\"\nregress y x, hascons\n").contains("hascons"),
        "an option that is not translated should be named"
    );
    // regress alone shows the last fit again: with none, Stata's own error as the line runs
    assert!(
        script("use \"d.dta\"\nregress\n").contains("last estimates not found"),
        "regress with no fit before it is Stata's error"
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

#[test]
fn a_data_frame_named_e_still_gets_its_estimation_sample() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    // a copy named e.dta, with one y missing: R names the data frame `e`
    ok(
        &mut e,
        &format!(
            "local({{ p <- haven::read_dta('{dir}/panel.dta'); p$y[3] <- NA; haven::write_dta(p, '{dir}/e.dta') }})"
        ),
    );
    let code = script(&format!("use \"{dir}/e.dta\"\nregress y x w\n"));
    assert!(code.contains("e <- haven::read_dta("), "{code}");
    ok(&mut e, &code);
    let got = numbers(
        &mut e,
        "0",
        "sum(stata_esample), which(!stata_esample), nrow(stata_data)",
    );
    assert_eq!(got, [19.0, 3.0, 19.0]);
}

/// SW5: a fit that fails leaves the last model as it was, as in Stata, where a
/// failed command leaves e() alone, so `predict` and `_b[]` still work.
#[test]
fn a_failed_fit_leaves_the_earlier_model_in_place() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    let mut live = rcraft_statatr::live::Live::new();
    let mut step = |e: &mut Engine, line: &str| {
        let mut s = live.step(line);
        // a value only R knows (the data's column names): asked, then stepped again
        while let Some(q) = s.ask.clone() {
            let (o, t) = run(e, &q);
            live.give(if o == Outcome::Ok { Ok(t) } else { Err(t) });
            s = live.step(line);
        }
        assert!(!s.code.trim().is_empty(), "no R for {line}: {s:?}");
        run(e, &s.code)
    };
    let (o, t) = step(&mut e, &format!("use \"{dir}/panel.dta\""));
    assert_eq!(o, Outcome::Ok, "{t}");
    let (o, t) = step(&mut e, "regress y x w");
    assert_eq!(o, Outcome::Ok, "{t}");
    let before = numbers(&mut e, "0", "unname(stats::coef(stata_estimates))");
    // y is not a 0/1 outcome: R stops in the fit (or Stata's own error does)
    let (o, t) = step(&mut e, "logit y x");
    assert_eq!(o, Outcome::Error, "the fit should fail: {t}");
    let after = numbers(&mut e, "0", "unname(stats::coef(stata_estimates))");
    assert_eq!(before, after);
    let (o, t) = step(&mut e, "predict yhat");
    assert_eq!(o, Outcome::Ok, "predict after the failed fit: {t}");
    let (o, t) = step(&mut e, "display _b[x]");
    assert_eq!(o, Outcome::Ok, "{t}");
    close(
        at(
            &numbers(&mut e, "0", "unname(stats::coef(stata_estimates))[2]"),
            0,
        ),
        at(&before, 1),
        1e-12,
        "_b[x]",
    );
    // a fit that works replaces it
    let (o, t) = step(&mut e, "regress y x");
    assert_eq!(o, Outcome::Ok, "{t}");
    let now = numbers(&mut e, "0", "length(stats::coef(stata_estimates))");
    assert_eq!(at(&now, 0), 2.0);
}

/// SW6: a variable the data lack, in `by`, `keep`, `drop`, `collapse`, `egen`'s
/// `by()` or an `if`, says so in Stata's words and changes nothing, instead of
/// a raw dplyr error; lines that name real variables still run.
#[test]
fn a_missing_variable_anywhere_in_a_line_says_so_and_changes_nothing() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    let mut live = rcraft_statatr::live::Live::new();
    let mut step = |e: &mut Engine, line: &str| {
        let mut s = live.step(line);
        while let Some(q) = s.ask.clone() {
            let (o, t) = run(e, &q);
            live.give(if o == Outcome::Ok { Ok(t) } else { Err(t) });
            s = live.step(line);
        }
        assert!(!s.code.trim().is_empty(), "no R for {line}: {s:?}");
        run(e, &s.code)
    };
    let (o, t) = step(&mut e, &format!("use \"{dir}/panel.dta\""));
    assert_eq!(o, Outcome::Ok, "{t}");
    for line in [
        "egen m = mean(x), by(nosuch)",
        "bysort nosuch: summarize x",
        "keep x nosuch",
        "drop nosuch",
        "drop if nosuch > 0",
        "collapse (mean) y, by(nosuch)",
        "count if nosuch > 1",
        "egen m = mean(nosuch)",
        "summarize x if nosuch > 1",
        "regress y x, vce(cluster nosuch)",
    ] {
        let (o, t) = step(&mut e, line);
        assert_eq!(o, Outcome::Error, "{line}: {t}");
        assert!(
            t.contains("variable nosuch not found (Stata: r(111))"),
            "{line}: {t}"
        );
        assert!(
            !t.contains("dplyr") && !t.contains("object '"),
            "{line}: {t}"
        );
    }
    // nothing changed: still twenty rows and the same columns
    let shape = numbers(&mut e, "0", "c(nrow(panel), ncol(panel))");
    assert_eq!(shape, [20.0, 6.0]);
    // lines that name real variables run
    for line in [
        "egen m = mean(x), by(cl)",
        "bysort cl: summarize x",
        "generate q = log(w) * 2 if x > 3 & _n < 15",
        "count if x > 3 & cl != 2",
        "scalar cut = 5",
        "count if x > cut",
        "collapse (mean) y, by(cl)",
        "keep y cl",
        "drop if cl > 4",
    ] {
        let (o, t) = step(&mut e, line);
        assert_eq!(o, Outcome::Ok, "{line}: {t}");
    }
}

/// SW10: `i.g` on a factor or a text column made in R (not opened with `use`)
/// prints Stata's layout, the levels named as a labelled variable's are and the
/// first (or `ib#`'s) left out, and never stops on a raw subscript error.
#[test]
fn a_factor_made_in_r_gets_statas_coefficient_layout() {
    let Some(mut e) = engine() else { return };
    ok(
        &mut e,
        "set.seed(3); rg <- data.frame(y = rnorm(30), x = rnorm(30), \
         g = factor(rep(c('a', 'b', 'c'), 10)), s = rep(c('u', 'v', 'w'), each = 10), \
         stringsAsFactors = FALSE)",
    );
    let mut live = rcraft_statatr::live::Live::new();
    live.adopt("rg");
    let mut step = |e: &mut Engine, line: &str| {
        let mut s = live.step(line);
        while let Some(q) = s.ask.clone() {
            let (o, t) = run(e, &q);
            live.give(if o == Outcome::Ok { Ok(t) } else { Err(t) });
            s = live.step(line);
        }
        assert!(!s.code.trim().is_empty(), "no R for {line}: {s:?}");
        run(e, &s.code)
    };
    for (line, rows, formula) in [
        ("regress y x i.g", ["b", "c"], "y ~ x + g"),
        ("regress y x i.s", ["v", "w"], "y ~ x + s"),
        (
            "regress y x ib2.g",
            ["a", "c"],
            "y ~ x + relevel(g, ref = 'b')",
        ),
    ] {
        let (o, t) = step(&mut e, line);
        assert_eq!(o, Outcome::Ok, "{line}: {t}");
        assert!(
            !t.contains("subscript") && !t.contains("R's layout"),
            "{line}: {t}"
        );
        // Stata's table: the factor's name over its levels, each named by its label
        let var = if line.contains(".s") { "s" } else { "g" };
        assert!(t.contains(&format!("{var:>12} |\n")), "{line}: {t}");
        for r in rows {
            assert!(t.contains(&format!("{r:>11}  |")), "{line}: {t}");
        }
        // and the numbers are lm's
        let got = numbers(&mut e, "0", "unname(stats::coef(stata_estimates))");
        let want = numbers(
            &mut e,
            "0",
            &format!("unname(stats::coef(stats::lm({formula}, data = rg)))"),
        );
        assert_eq!(got.len(), want.len(), "{line}");
        for i in 0..got.len() {
            close(at(&got, i), at(&want, i), 1e-9, line);
        }
    }
}

/// SW11: a number (or a labelled number) compared with a string, or a string
/// with a number, in an `if` is Stata's "type mismatch" (r(109)), not dplyr's
/// error; the same comparison with the right kind still runs.
#[test]
fn comparing_a_number_with_a_string_is_a_type_mismatch() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    ok(
        &mut e,
        &format!(
            "local({{
  d <- haven::read_dta('{dir}/panel.dta')
  d$g <- haven::labelled(as.numeric(d$g), c(a = 0, b = 1))
  d$s <- rep(c('u', 'v'), 10)
  haven::write_dta(d, '{dir}/mixed.dta')
}})"
        ),
    );
    let mut live = rcraft_statatr::live::Live::new();
    let mut step = |e: &mut Engine, line: &str| {
        let mut s = live.step(line);
        while let Some(q) = s.ask.clone() {
            let (o, t) = run(e, &q);
            live.give(if o == Outcome::Ok { Ok(t) } else { Err(t) });
            s = live.step(line);
        }
        assert!(!s.code.trim().is_empty(), "no R for {line}: {s:?}");
        run(e, &s.code)
    };
    let (o, t) = step(&mut e, &format!("use \"{dir}/mixed.dta\""));
    assert_eq!(o, Outcome::Ok, "{t}");
    for line in [
        "summarize x if g == \"a\"",
        "tabulate g if g == \"a\"",
        "list x if \"a\" == g",
        "count if g != \"b\"",
        "regress y x if g == \"a\"",
        "keep if g == \"a\"",
        "drop if g == \"a\"",
        "count if s == 3",
        "summarize x if s > 2",
        "count if x == \"a\"",
    ] {
        let (o, t) = step(&mut e, line);
        assert_eq!(o, Outcome::Error, "{line}: {t}");
        assert!(t.contains("type mismatch (Stata: r(109))"), "{line}: {t}");
        assert!(
            !t.contains("dplyr") && !t.contains("combine"),
            "{line}: {t}"
        );
    }
    // nothing was kept or dropped
    let shape = numbers(&mut e, "0", "nrow(mixed)");
    assert_eq!(shape, [20.0]);
    // the right kinds still run, and a missing variable is still r(111)
    for line in [
        "summarize x if g == 1",
        "count if g == 0 & x > 3",
        "count if s == \"u\"",
        "count if 3 < x",
        "regress y x if s == \"v\"",
        "keep if g == 1",
    ] {
        let (o, t) = step(&mut e, line);
        assert_eq!(o, Outcome::Ok, "{line}: {t}");
    }
    let (o, t) = step(&mut e, "count if nosuch == \"a\"");
    assert_eq!(o, Outcome::Error, "{t}");
    assert!(
        t.contains("variable nosuch not found (Stata: r(111))"),
        "{t}"
    );
}
