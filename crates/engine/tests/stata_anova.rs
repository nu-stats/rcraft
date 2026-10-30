//! P2B.4, `anova` translated to R and run in a real R.
//!
//! On an unbalanced design, so that the kinds of sums of squares differ:
//!
//! - the partial sums of squares (Stata's default) against SAS's Type III by
//!   its definition: the sum-to-zero design matrix built by hand, and each
//!   term's sum of squares as the rise in the residual sum of squares when its
//!   columns are dropped; and shown to differ from the sequential ones;
//! - `sequential` against Type I by the same model comparisons in order;
//! - a covariate (`c.x`), whose partial sum of squares is its t statistic
//!   squared times the mean square error;
//! - `ib3.g` giving the same table as `g`, since Type III does not depend on
//!   the base;
//! - `test` after `anova` reading the dummy-coded fit, so its F for `g` is
//!   the table's in an additive model;
//! - an empty cell stopping the R with its reason.
//!
//! Without a supported R, or without haven and dplyr, the test is skipped
//! with a note.

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

/// Thirty-two rows in an unbalanced three-by-two design, a covariate, and a
/// variable `k` that leaves the cell g = 3, k = 1 empty.
fn cells(e: &mut Engine) -> String {
    let dir = std::env::temp_dir().join(format!(
        "rcraft-stata-anova-{}-{}",
        std::process::id(),
        fresh()
    ));
    let _ = std::fs::create_dir_all(&dir);
    let d = dir.display().to_string().replace('\\', "/");
    ok(
        e,
        &format!(
            "local({{
  set.seed(20261031)
  g <- rep(c(1, 2, 3), times = c(10, 14, 8))
  h <- c(rep(0:1, length.out = 10), rep(c(0, 0, 1), length.out = 14), rep(c(0, 1, 1, 1), 2))
  x <- round(stats::rnorm(32), 3)
  y <- 2 + (g == 2) * 1.5 - (g == 3) + 0.8 * h + 0.7 * (g == 3) * h + 0.5 * x + stats::rnorm(32)
  k <- ifelse(g == 3, 0, h)
  haven::write_dta(data.frame(g = g, h = h, x = x, y = y, k = k), '{d}/cells.dta')
}})"
        ),
    );
    d
}

/// The anova table the script prints, by row: sums of squares, then df,
/// then F, then p.
fn table(e: &mut Engine, code: &str) -> Vec<f64> {
    numbers(
        e,
        &format!("res <- {}", line_of(code, "stata_anova(")),
        "res[[1]], res[[2]], res[[4]][1:(nrow(res) - 2)], res[[5]][1:(nrow(res) - 2)]",
    )
}

/// The sum-to-zero design for g, h and g#h, and the residual sum of squares
/// of the columns kept.
const TYPE3_BY_HAND: &str =
    "G <- cbind((cells$g == 1) - (cells$g == 3), (cells$g == 2) - (cells$g == 3)); \
     H <- cbind((cells$h == 0) - (cells$h == 1)); GH <- G * H[, 1]; one <- rep(1, nrow(cells)); \
     rss <- function(...) sum(stats::lm.fit(cbind(...), cells$y)$residuals^2); \
     full <- rss(one, G, H, GH); mse <- full / (nrow(cells) - 6); \
     tss <- sum((cells$y - mean(cells$y))^2)";

#[test]
fn partial_sums_of_squares_are_type_iii_by_its_definition() {
    let Some(mut e) = engine() else { return };
    let dir = cells(&mut e);
    let code = script(&format!("use \"{dir}/cells.dta\"\nanova y g h g#h\n"));
    let text = ok(&mut e, &code);
    assert!(text.contains("Partial SS"), "{text}");
    assert!(text.contains("g#h"), "Stata's names for the terms: {text}");
    let got = table(&mut e, &code);
    // rows: Model, g, h, g#h, Residual, Total
    let want = numbers(
        &mut e,
        TYPE3_BY_HAND,
        "tss - full, rss(one, H, GH) - full, rss(one, G, GH) - full, rss(one, G, H) - full, full, tss",
    );
    for i in 0..6 {
        close(at(&got, i), at(&want, i), 1e-9, "sum of squares");
    }
    let df: Vec<f64> = (6..12).map(|i| at(&got, i)).collect();
    all_close(
        &df,
        &[5.0, 2.0, 1.0, 2.0, 26.0, 31.0],
        1e-12,
        "degrees of freedom",
    );
    let f = numbers(
        &mut e,
        TYPE3_BY_HAND,
        "((tss - full) / 5) / mse, ((rss(one, H, GH) - full) / 2) / mse, \
         ((rss(one, G, GH) - full) / 1) / mse, ((rss(one, G, H) - full) / 2) / mse",
    );
    for i in 0..4 {
        close(at(&got, 12 + i), at(&f, i), 1e-9, "F");
    }
    let p = numbers(
        &mut e,
        "0",
        &format!("stats::pf({}, 2, 26, lower.tail = FALSE)", at(&f, 1)),
    );
    close(
        at(&got, 17),
        at(&p, 0),
        1e-12,
        "the p-value of g on (2, 26)",
    );

    // the sequential sum of squares of g is a different number here
    let seq = numbers(&mut e, TYPE3_BY_HAND, "rss(one) - rss(one, G)");
    assert!(
        (at(&seq, 0) - at(&got, 1)).abs() > 1e-6,
        "an unbalanced design: partial and sequential differ"
    );
}

#[test]
fn sequential_sums_of_squares_are_type_i_in_the_order_written() {
    let Some(mut e) = engine() else { return };
    let dir = cells(&mut e);
    let code = script(&format!(
        "use \"{dir}/cells.dta\"\nanova y g h g#h, sequential\n"
    ));
    let text = ok(&mut e, &code);
    assert!(text.contains("Seq. SS"), "{text}");
    let got = table(&mut e, &code);
    let want = numbers(
        &mut e,
        TYPE3_BY_HAND,
        "rss(one) - rss(one, G), rss(one, G) - rss(one, G, H), rss(one, G, H) - full",
    );
    for i in 0..3 {
        close(
            at(&got, i + 1),
            at(&want, i),
            1e-9,
            "sequential sum of squares",
        );
    }
}

#[test]
fn a_covariates_partial_sum_of_squares_is_its_t_squared_times_the_mse() {
    let Some(mut e) = engine() else { return };
    let dir = cells(&mut e);
    let code = script(&format!("use \"{dir}/cells.dta\"\nanova y g c.x\n"));
    ok(&mut e, &code);
    let got = table(&mut e, &code);
    let want = numbers(
        &mut e,
        "m <- stats::lm(y ~ factor(g) + x, data = cells); s <- summary(m); \
         tx <- s$coefficients[\"x\", 3]; mse <- s$sigma^2; \
         rg <- sum(stats::residuals(stats::lm(y ~ x, data = cells))^2) - sum(stats::residuals(m)^2)",
        "rg, tx^2 * mse",
    );
    close(at(&got, 1), at(&want, 0), 1e-9, "g, adjusted for x");
    close(at(&got, 2), at(&want, 1), 1e-9, "x, adjusted for g");
}

#[test]
fn the_base_level_does_not_change_the_table() {
    let Some(mut e) = engine() else { return };
    let dir = cells(&mut e);
    let code = script(&format!("use \"{dir}/cells.dta\"\nanova y g h g#h\n"));
    ok(&mut e, &code);
    let plain = table(&mut e, &code);
    let code = script(&format!(
        "use \"{dir}/cells.dta\"\nanova y ib3.g h ib3.g#h\n"
    ));
    ok(&mut e, &code);
    let based = table(&mut e, &code);
    all_close(&based, &plain, 1e-9, "the table with base 3");
}

#[test]
fn test_after_anova_reads_the_dummy_coded_fit() {
    let Some(mut e) = engine() else { return };
    let dir = cells(&mut e);
    let code = script(&format!(
        "use \"{dir}/cells.dta\"\nanova y g h\ntest 2.g 3.g\n"
    ));
    ok(&mut e, &code);
    let names = ok(&mut e, "cat(names(stata_b), \"\\n\")");
    assert!(
        names.contains("factor(g)2"),
        "base level 1 left out: {names}"
    );
    let f = numbers(
        &mut e,
        &format!("res <- {}", line_of(&code, "stata_test(")),
        "res[1]",
    );
    let got = table(&mut e, &code);
    // five rows (Model, g, h, Residual, Total): the Fs start at 10, g's is 11
    close(
        at(&f, 0),
        at(&got, 11),
        1e-9,
        "test's F for g against the table's",
    );
}

#[test]
fn an_empty_cell_stops_the_r_with_its_reason() {
    let Some(mut e) = engine() else { return };
    let dir = cells(&mut e);
    let code = script(&format!("use \"{dir}/cells.dta\"\nanova y g k g#k\n"));
    let (outcome, text) = run(&mut e, &code);
    assert_eq!(outcome, Outcome::Error, "{text}");
    assert!(text.contains("empty cell"), "{text}");
    // sequential sums of squares have no such problem
    let code = script(&format!(
        "use \"{dir}/cells.dta\"\nanova y g k g#k, sequential\n"
    ));
    ok(&mut e, &code);
}

/// Stata 19 SE's own sums of squares on this same data (`cells`), run in
/// batch mode on 2026-10-30 and printed with `%24.17g`.
#[test]
fn the_sums_of_squares_are_stata_19s_own() {
    let Some(mut e) = engine() else { return };
    let dir = cells(&mut e);
    let stata: [(&str, [f64; 3]); 4] = [
        (
            "anova y g h g#h",
            [
                20.891545388435485,
                5.5182964607521905,
                0.0053058128340021625,
            ],
        ),
        (
            "anova y g h g#h, sequential",
            [
                17.283725139581428,
                5.9267959204930705,
                0.0053058128340026656,
            ],
        ),
        // a continuous variable interacted with a factor: Type III too
        (
            "anova y g c.x g#c.x",
            [20.456665562415004, 7.873750760094401, 2.0988419329224439],
        ),
        (
            "anova y ib3.g h ib3.g#h",
            [
                20.891545388435475,
                5.5182964607521861,
                0.0053058128340020896,
            ],
        ),
    ];
    for (cmd, want) in stata {
        let code = script(&format!("use \"{dir}/cells.dta\"\n{cmd}\n"));
        ok(&mut e, &code);
        let got = table(&mut e, &code);
        for (i, w) in want.iter().enumerate() {
            close(at(&got, i + 1), *w, 1e-10 * w.abs().max(1.0), cmd);
        }
    }
}

#[test]
fn what_anova_does_not_translate_says_why() {
    assert!(refusal("use \"d.dta\"\nanova y g|c.x\n").contains("nested"));
    assert!(refusal("use \"d.dta\"\nanova y g##h / k\n").contains("##"));
    assert!(refusal("use \"d.dta\"\nanova y g, repeated(g)\n").contains("between-subjects"));
    // a covariate interacted with a factor is translated, partial or sequential
    for cmd in ["anova y c.x#g", "anova y c.x#g, sequential"] {
        let t = translate(&read(&format!("use \"d.dta\"\n{cmd}\n")));
        assert!(t.lines.iter().all(|l| l.result.is_ok()), "{cmd}");
    }
}

/// A number no other call in this run has had: the tests in a file run at
/// once, and each writing its data to the same folder let one read a file
/// another was still writing.
fn fresh() -> usize {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

/// The nested and repeated-measures designs of the Stata 19 probe, made the
/// way Stata made them (`round(..., 0.001)` stored as a float).
fn designs(e: &mut Engine) -> String {
    let dir = std::env::temp_dir().join(format!(
        "rcraft-stata-anova-designs-{}-{}",
        std::process::id(),
        fresh()
    ));
    let _ = std::fs::create_dir_all(&dir);
    let d = dir.display().to_string().replace('\\', "/");
    ok(
        e,
        &format!(
            "local({{
  fl <- function(x) readBin(writeBin(as.numeric(x), raw(), size = 4), 'double', size = 4, n = length(x))
  i <- 1:36
  a <- (i - 1) %% 3 + 1
  b <- floor((i - 1) / 3) %% 4 + 1
  r <- floor((i - 1) / 12) + 1
  haven::write_dta(data.frame(a = a, b = b, r = r, y = fl(round(sin(i) * 3 + a + 0.5 * b + cos(3 * i), 3))), '{d}/nest.dta')
  i <- 1:40
  subj <- floor((i - 1) / 4) + 1
  t <- (i - 1) %% 4 + 1
  grp <- subj %% 2
  haven::write_dta(data.frame(subj = subj, t = t, grp = grp, y = fl(round(subj * 0.3 + t * 0.7 + cos(i * 1.7) + grp, 3))), '{d}/rep.dta')
}})"
        ),
    );
    d
}

/// Error terms (`/`), nested terms (`b|a`) and `repeated()`, against Stata 19:
/// a term is tested against the first term after the next `/` (the residual
/// after a trailing one), and the first term of the last group, being an
/// error term, is not tested; the epsilons come from the covariance of the
/// repeated measures within the subjects, pooled over the groups.
#[test]
fn error_terms_nesting_and_repeated_measures_agree_with_stata_19() {
    let Some(mut e) = engine() else { return };
    let d = designs(&mut e);
    let table = |e: &mut Engine, file: &str, cmd: &str| -> Vec<f64> {
        let code = script(&format!("use \"{d}/{file}.dta\"\n{cmd}\n"));
        ok(e, &code);
        numbers(
            e,
            &format!("res <- {}", line_of(&code, "stata_anova(")),
            "res[[1]], res$F",
        )
    };
    // rows: Model, the terms, Residual, Total; the SS then the F
    let near = |got: &[f64], want: &[(usize, f64)], what: &str| {
        for (i, w) in want {
            let g = got.get(*i).copied().unwrap_or(f64::NAN);
            assert!(
                (g - w).abs() <= 1e-9 * w.abs().max(1.0),
                "{what} [{i}]: {g} against Stata's {w}"
            );
        }
    };
    let got = table(&mut e, "nest", "anova y a / b|a /");
    // 4 rows: Model a b|a Residual Total, then 5 F's
    near(
        &got,
        &[
            (1, 17.677635373230093),
            (2, 117.14307905202323),
            (3, 41.417676185496553),
            (5, 7.1021436524181425),
            (6, 0.67907860902484518),
            (7, 7.5422276889623499),
        ],
        "anova y a / b|a /",
    );
    let got = table(&mut e, "nest", "anova y a b|a");
    near(
        &got,
        &[(6, 5.1217654879692258), (7, 7.5422276889623499)],
        "anova y a b|a",
    );
    let got = table(&mut e, "nest", "anova y a / b|a / r");
    near(
        &got,
        &[
            (3, 0.39619997744566338),
            (4, 41.021476208050892),
            (7, 0.67907860902484518),
            (8, 65.703676998477789),
        ],
        "anova y a / b|a / r",
    );
    // r, an error term, is not tested (Stata's F is missing; NA is not read
    // back as a number, so only the nine others come back)
    assert_eq!(got.len(), 9, "r has no F: {got:?}");
    let got = table(&mut e, "nest", "anova y a / r#a / b b#a");
    near(
        &got,
        &[
            (1, 17.677635373230068),
            (2, 0.93452342472244321),
            (3, 95.652101229802966),
            (4, 21.490977822220188),
            (5, 40.483152760774111),
            (8, 56.748610807097919),
            // b, an error term, has no F (NA is not read back), so b#a's is the 11th
            (10, 1.5925867693074338),
        ],
        "anova y a / r#a / b b#a",
    );
    // repeated measures
    let code = script(&format!(
        "use \"{d}/rep.dta\"\nanova y grp / subj|grp t t#grp, repeated(t)\n"
    ));
    ok(&mut e, &code);
    let v = numbers(
        &mut e,
        &format!("res <- {}", line_of(&code, "stata_anova(")),
        "res[[1]][2:5], res$F[c(2, 4, 5)], attr(res, \"epsilon\")",
    );
    let want = [
        4.8587881361442111,
        30.102932344362266,
        23.408814176963325,
        0.05377213614029755,
        1.2912464687658043,
        10.117446683156812,
        0.023240678332792183,
        0.96785136720848786,
        0.65171793275739631,
        0.33333333333333331,
    ];
    assert_eq!(v.len(), want.len(), "{v:?}");
    for (g, w) in v.iter().zip(want) {
        assert!(
            (g - w).abs() <= 1e-9 * w.abs().max(1.0),
            "repeated: {g} against Stata's {w}"
        );
    }
}
