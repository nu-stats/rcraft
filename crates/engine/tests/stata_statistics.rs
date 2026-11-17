//! P2B.4: Stata's statistics commands translated to R and run in a real R.
//!
//! Each case is a short do-file over one made-up dataset whose numbers are
//! chosen so the right answer is known in advance (`x` is 1 to 10, the two
//! groups are 1-5 and 6-10, `y` is `2 * x`). Every line must translate, the
//! script must run, and the figures must be the ones Stata gives:
//!
//! - `summarize, detail` percentiles follow Stata's rule, so the 25th
//!   percentile of 1..10 is 3, not R's default 3.25, and the 75th is 8, not
//!   7.75;
//! - its kurtosis is the population one, so 1..10 gives about 1.776;
//! - `correlate` drops a whole row when any of its variables is missing, which
//!   is checked against the same correlation worked out a second way;
//! - `tabulate, chi2` has no continuity correction, so the 2x2 table here
//!   gives 5/3, worked out by hand in the comment beside it, and R's corrected
//!   figure is shown to be a different number;
//! - `ttest x, by(g)` on these numbers has t exactly -5 on 8 degrees of
//!   freedom.
//!
//! Without a supported R, or without dplyr and haven, the test is skipped with
//! a note.

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
        "stopifnot(requireNamespace(\"dplyr\", quietly = TRUE), requireNamespace(\"haven\", quietly = TRUE))",
    );
    if have != Outcome::Ok {
        eprintln!("skipped: the R packages dplyr and haven are needed");
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

/// Runs `setup` with everything it prints thrown away, then reads the numbers
/// `show` prints. The helpers print a table of their own, which would
/// otherwise be read back instead of the answer.
fn numbers(e: &mut Engine, setup: &str, show: &str) -> Vec<f64> {
    let text = ok(
        e,
        &format!(
            "invisible(utils::capture.output({setup}))\ncat(format(c({show}), digits = 15), \"\\n\")"
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

/// A folder of this test's own, with the dataset written into it.
fn fixture(e: &mut Engine) -> String {
    let dir = std::env::temp_dir().join(format!(
        "rcraft-stata-stats-{}-{}",
        std::process::id(),
        fresh()
    ));
    let _ = std::fs::create_dir_all(&dir);
    let d = dir.display().to_string().replace('\\', "/");
    ok(
        e,
        &format!(
            "local({{
  haven::write_dta(data.frame(
    id = 1:10,
    x = as.numeric(1:10),
    y = as.numeric(2 * (1:10)),
    z = c(5, 3, NA, 8, 1, 9, 2, 7, 4, 6),
    g = c(0, 0, 0, 0, 0, 1, 1, 1, 1, 1),
    b = c(1, 1, 1, 1, 0, 0, 0, 0, 1, 1),
    s = c('a', 'b', 'a', 'b', 'a', 'b', 'a', 'b', 'a', 'b')
  ), '{d}/study.dta')
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

/// The reason the first refused command gives.
fn refusal(dofile: &str) -> String {
    translate(&read(dofile))
        .lines
        .iter()
        .find_map(|l| l.result.as_ref().err().map(|e| e.reason.clone()))
        .unwrap_or_default()
}

#[test]
fn summarize_gives_statas_statistics_and_statas_percentiles() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    let code = script(&format!("use \"{dir}/study.dta\"\nsummarize x y\n"));
    let text = ok(&mut e, &code);
    assert!(text.contains("x") && text.contains("5.5"), "{text}");

    let v = numbers(
        &mut e,
        "s <- stata_summarize(study[c(\"x\")])",
        "s$obs, s$mean, s$sd, s$min, s$max",
    );
    assert_eq!(at(&v, 0), 10.0, "obs");
    close(at(&v, 1), 5.5, 1e-9, "mean");
    // the sample standard deviation: sqrt(82.5 / 9)
    close(at(&v, 2), (82.5f64 / 9.0).sqrt(), 1e-9, "sd");
    close(at(&v, 3), 1.0, 1e-9, "min");
    close(at(&v, 4), 10.0, 1e-9, "max");

    // detail: Stata's percentile rule, not R's default
    let v = numbers(
        &mut e,
        "s <- stata_summarize(study[c(\"x\")], detail = TRUE)",
        "s$p25, s$p50, s$p75, s$skewness, s$kurtosis, s$variance",
    );
    close(at(&v, 0), 3.0, 1e-9, "p25");
    close(at(&v, 1), 5.5, 1e-9, "p50");
    close(at(&v, 2), 8.0, 1e-9, "p75");
    close(at(&v, 3), 0.0, 1e-9, "skewness");
    // 120.8625 / 8.25^2, from the deviations of 1..10
    close(at(&v, 4), 120.8625 / (8.25f64 * 8.25), 1e-9, "kurtosis");
    close(at(&v, 5), 82.5 / 9.0, 1e-9, "variance");

    // R's own default percentiles are the other ones, so the rule really differs
    let v = numbers(&mut e, "0", "unname(stats::quantile(1:10, c(0.25, 0.75)))");
    close(at(&v, 0), 3.25, 1e-9, "R's p25");
    close(at(&v, 1), 7.75, 1e-9, "R's p75");
}

#[test]
fn summarize_counts_only_the_rows_an_if_keeps_and_a_string_has_none() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    let code = script(&format!(
        "use \"{dir}/study.dta\"\nsummarize x if g == 1\nsummarize s\n"
    ));
    ok(&mut e, &code);
    let v = numbers(
        &mut e,
        "s <- stata_summarize(dplyr::filter(study, g == 1)[c(\"x\")])",
        "s$obs, s$mean",
    );
    assert_eq!(at(&v, 0), 5.0, "the five rows of the second group");
    close(at(&v, 1), 8.0, 1e-9, "the mean of 6..10");
    // a string variable has no observations, as in Stata
    let v = numbers(&mut e, "s <- stata_summarize(study[c(\"s\")])", "s$obs");
    assert_eq!(at(&v, 0), 0.0, "a string variable");
}

#[test]
fn tabulate_counts_and_its_chi_squared_has_no_continuity_correction() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    let code = script(&format!(
        "use \"{dir}/study.dta\"\ntabulate g\ntabulate g b, chi2 row\n"
    ));
    let text = ok(&mut e, &code);
    assert!(
        text.contains("      Total |         10      100.00"),
        "{text}"
    );
    assert!(text.contains("Pearson chi2"), "{text}");
    // Stata 19's table of percentages, with its Key box
    assert!(text.contains("| row percentage |"), "{text}");

    let v = numbers(
        &mut e,
        "t <- stata_tabulate(stata_tab_values(study$g))",
        "t$freq, t$percent, t$cum",
    );
    assert_eq!(at(&v, 0), 5.0, "the first group");
    assert_eq!(at(&v, 1), 5.0, "the second group");
    close(at(&v, 2), 50.0, 1e-9, "percent");
    close(at(&v, 5), 100.0, 1e-9, "the cumulative percent ends at 100");

    // the table is 1 and 4 over 3 and 2, so Pearson's chi2 is
    // 10 * (1 * 2 - 4 * 3)^2 / (5 * 5 * 4 * 6) = 5/3
    let v = numbers(
        &mut e,
        "r <- stats::chisq.test(table(study$g, study$b), correct = FALSE)",
        "unname(r$statistic)",
    );
    close(at(&v, 0), 5.0 / 3.0, 1e-9, "chi2");
    // R's corrected figure is a different number, so the choice matters
    let v = numbers(
        &mut e,
        "r <- suppressWarnings(stats::chisq.test(table(study$g, study$b)))",
        "unname(r$statistic)",
    );
    assert!(
        (at(&v, 0) - 5.0 / 3.0).abs() > 0.1,
        "the corrected chi2 should differ: {}",
        at(&v, 0)
    );
}

#[test]
fn tabulate_shows_value_labels_and_leaves_missing_values_out_unless_asked() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    let code = script(&format!(
        "use \"{dir}/study.dta\"\nlabel define yn 0 \"no\" 1 \"yes\"\nlabel values g yn\ntabulate g\ntabulate z\ntabulate z, missing\n"
    ));
    let text = ok(&mut e, &code);
    assert!(text.contains("no") && text.contains("yes"), "{text}");

    // z misses one of its ten values
    let v = numbers(
        &mut e,
        "t <- stata_tabulate(stata_tab_values(study$z))",
        "sum(t$freq)",
    );
    assert_eq!(at(&v, 0), 9.0, "the missing value is left out");
    let v = numbers(
        &mut e,
        "t <- stata_tabulate(stata_tab_values(study$z), missing = TRUE)",
        "sum(t$freq)",
    );
    assert_eq!(at(&v, 0), 10.0, "missing asks for it back");

    // nolabel shows the numbers underneath
    let text = ok(
        &mut e,
        "invisible(utils::capture.output(t <- stata_tabulate(stata_tab_values(study$g, nolabel = TRUE))))\ncat(t$value)",
    );
    assert!(
        text.contains('0') && text.contains('1') && !text.contains("yes"),
        "{text}"
    );
}

#[test]
fn correlate_drops_a_whole_row_when_any_of_its_variables_is_missing() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    let code = script(&format!(
        "use \"{dir}/study.dta\"\ncorrelate x y\ncorrelate x z\ncorrelate x y, covariance\n"
    ));
    let text = ok(&mut e, &code);
    // x and y are 1..10 and twice that, so they agree exactly
    assert!(text.contains("(obs=10)"), "{text}");
    assert!(
        text.contains("(obs=9)"),
        "the row with a missing z goes: {text}"
    );

    let v = numbers(
        &mut e,
        "r <- stata_correlate(study[c(\"x\", \"y\")])",
        "r[1, 2]",
    );
    close(at(&v, 0), 1.0, 1e-9, "cor(x, y)");

    // z misses its third value, so that row goes for both variables: the same
    // number, worked out a second way
    let got = numbers(
        &mut e,
        "r <- stata_correlate(study[c(\"x\", \"z\")])",
        "r[1, 2]",
    );
    let want = numbers(&mut e, "0", "stats::cor(study$x[-3], study$z[-3])");
    close(at(&got, 0), at(&want, 0), 1e-9, "casewise cor(x, z)");
    // and it is not R's pairwise answer over all ten rows
    let pairwise = numbers(
        &mut e,
        "0",
        "stats::cor(study$x, study$z, use = \"pairwise.complete.obs\")",
    );
    close(
        at(&pairwise, 0),
        at(&want, 0),
        1e-9,
        "one missing value only",
    );

    // covariance of x and y is twice the variance of x
    let v = numbers(
        &mut e,
        "r <- stata_correlate(study[c(\"x\", \"y\")], covariance = TRUE)",
        "r[1, 2]",
    );
    close(at(&v, 0), 2.0 * 82.5 / 9.0, 1e-9, "cov(x, y)");
}

/// Stata 19 leaves a string variable out with a note and goes on; only when
/// every variable is a string is it an error (checked in Stata on
/// 2026-10-30).
#[test]
fn correlate_leaves_a_string_variable_out_as_stata_does() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    let code = script(&format!("use \"{dir}/study.dta\"\ncorrelate x s y\n"));
    let text = ok(&mut e, &code);
    assert!(
        text.contains("(s ignored because string variable)"),
        "{text}"
    );
    let line = code
        .lines()
        .rfind(|l| l.starts_with("stata_correlate("))
        .unwrap_or_default();
    let v = numbers(&mut e, &format!("res <- {line}"), "nrow(res), res[2, 1]");
    assert_eq!(at(&v, 0), 2.0, "x and y are left");
    close(at(&v, 1), 1.0, 1e-12, "y = 2x, so their correlation is 1");
    let code = script(&format!("use \"{dir}/study.dta\"\ncorrelate s\n"));
    let (outcome, text) = run(&mut e, &code);
    assert_eq!(outcome, Outcome::Error, "{text}");
    assert!(text.contains("too few variables specified"), "{text}");
}

#[test]
fn ttest_one_sample_two_sample_and_paired() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    let code = script(&format!(
        "use \"{dir}/study.dta\"\nttest x == 5.5\nttest x, by(g)\nttest x == y\nttest x, by(g) unequal\nttest x == 5.5, level(90)\n"
    ));
    let text = ok(&mut e, &code);
    assert!(text.contains("t-test"), "{text}");
    assert!(text.contains("90 percent"), "level(90): {text}");

    // the mean of 1..10 is exactly the value tested, so t is 0
    let v = numbers(
        &mut e,
        "r <- stats::t.test(study$x, mu = 5.5)",
        "unname(r$statistic)",
    );
    close(at(&v, 0), 0.0, 1e-9, "one-sample t");

    // two groups of five: the difference is 5 and the standard error is 1
    let v = numbers(
        &mut e,
        "r <- stats::t.test(x ~ g, data = study, var.equal = TRUE)",
        "unname(r$statistic), unname(r$parameter)",
    );
    close(at(&v, 0), -5.0, 1e-9, "two-sample t");
    close(at(&v, 1), 8.0, 1e-9, "degrees of freedom");

    // paired: every difference is -x, so t is its mean over its standard error
    let v = numbers(
        &mut e,
        "r <- stats::t.test(study$x, study$y, paired = TRUE)",
        "unname(r$statistic)",
    );
    let want = -5.5 / ((82.5f64 / 9.0).sqrt() / 10f64.sqrt());
    close(at(&v, 0), want, 1e-6, "paired t");
}

#[test]
fn the_statistics_commands_say_why_they_refuse_what_they_refuse() {
    let welch = refusal("use \"d.dta\"\nttest x, by(g) welch\n");
    assert!(
        welch.contains("Welch") && welch.contains("Satterthwaite"),
        "{welch}"
    );
    assert!(
        refusal("use \"d.dta\"\nttest x\n").contains("needs == or by()"),
        "a bare ttest should say what is missing"
    );
    assert!(
        refusal("use \"d.dta\"\nttest x == 1, unpaired\n").contains("nothing to unpair"),
        "a one-sample ttest cannot be unpaired"
    );
    assert!(
        refusal("use \"d.dta\"\nsummarize x, vsquish\n").contains("vsquish"),
        "an option that is not translated should be named"
    );
    assert!(
        refusal("use \"d.dta\"\ntabulate a b c\n").contains("tab1 and tab2 take"),
        "tab1 and tab2 are not this command"
    );
    assert!(
        refusal("use \"d.dta\"\nbysort g: margins\n")
            .contains("margins may not be combined with by"),
        "by with margins, in Stata's words"
    );
    assert!(
        refusal("summarize x\n").contains("no data is in memory"),
        "a command before any use should say so"
    );
}

/// The figures on this small data set against Stata 19 SE, run in batch mode
/// on 2026-10-30: casewise correlation, a string variable, percentiles with
/// a missing value, and the 2 x 2 chi-squared.
#[test]
fn the_statistics_agree_with_stata_19() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    let mut rerun = |cmd: &str, start: &str, show: &str| -> Vec<f64> {
        let code = script(&format!("use \"{dir}/study.dta\"\n{cmd}\n"));
        ok(&mut e, &code);
        let line = code
            .lines()
            .rfind(|l| l.starts_with(start))
            .unwrap_or_else(|| panic!("no {start} line:\n{code}"))
            .to_string();
        numbers(&mut e, &format!("res <- {line}"), show)
    };
    let near = |got: &[f64], stata: &[f64], what: &str| {
        assert_eq!(got.len(), stata.len(), "{what}: {got:?} against {stata:?}");
        for (g, s) in got.iter().zip(stata) {
            assert!(
                (g - s).abs() <= 1e-12 * s.abs().max(1.0),
                "{what}: {g} against Stata's {s}"
            );
        }
    };
    let got = rerun(
        "correlate x z y",
        "stata_correlate(",
        "res[2, 1], res[3, 2]",
    );
    near(
        &got,
        &[0.11881770515720093, 0.11881770515720093],
        "casewise correlate",
    );
    let got = rerun("summarize s", "stata_summarize(", "res$obs");
    near(&got, &[0.0], "a string variable has no observations");
    let got = rerun(
        "summarize z, detail",
        "stata_summarize(",
        "res$obs, res$mean, res$sd, res$skewness, res$kurtosis, res$p25, res$p50, res$p75",
    );
    near(
        &got,
        &[
            9.0,
            5.0,
            2.7386127875258306,
            0.0,
            1.7699999999999998,
            3.0,
            5.0,
            7.0,
        ],
        "summarize z, detail",
    );
    let got = rerun(
        "tabulate g b, chi2",
        "stata_tabulate(",
        "attr(res, \"chi2\")[c(\"chi2\", \"p\")]",
    );
    near(
        &got,
        &[1.6666666666666665, 0.19670560245894692],
        "the 2 x 2 chi-squared",
    );
}

/// A number no other call in this run has had: the tests in a file run at
/// once, and each writing its data to the same folder let one read a file
/// another was still writing.
fn fresh() -> usize {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

/// `tab1`, `tab2`, `pwcorr`, `summarize, meanonly` and `regress, beta`, run in
/// Stata 19 on this same dataset (`%24.17g`) and worked out a second way here.
#[test]
fn tab1_tab2_pwcorr_meanonly_and_beta_agree_with_stata_19() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    let rerun = |e: &mut Engine, cmd: &str, start: &str, show: &str| -> Vec<f64> {
        let code = script(&format!("use \"{dir}/study.dta\"\n{cmd}\n"));
        ok(e, &code);
        let line = code
            .lines()
            .rfind(|l| l.starts_with(start))
            .unwrap_or_else(|| panic!("no {start} line:\n{code}"))
            .to_string();
        numbers(e, &format!("res <- {line}"), show)
    };
    let near = |got: &[f64], stata: &[f64], what: &str| {
        assert_eq!(got.len(), stata.len(), "{what}: {got:?} against {stata:?}");
        for (g, s) in got.iter().zip(stata) {
            assert!(
                (g - s).abs() <= 1e-12 * s.abs().max(1.0),
                "{what}: {g} against Stata's {s}"
            );
        }
    };

    // tab1: each variable on its own rows, so z has 9 and g 10
    let got = rerun(
        &mut e,
        "tab1 g z",
        "stata_tab1(",
        "length(res), sum(res$g$freq), sum(res$z$freq)",
    );
    near(&got, &[2.0, 10.0, 9.0], "tab1 g z");
    let got = rerun(
        &mut e,
        "tab1 z, missing",
        "stata_tab1(",
        "sum(res$z$freq), nrow(res$z)",
    );
    near(
        &got,
        &[10.0, 10.0],
        "tab1 z, missing: the missing value is a row",
    );

    // tab2: every pair, each on the rows where both are present
    let got = rerun(
        &mut e,
        "tab2 g b z",
        "stata_tab2(",
        "length(res), sum(res[[1]]), sum(res[[2]]), sum(res[[3]]), res[[1]][1, 1], res[[1]][2, 2]",
    );
    near(&got, &[3.0, 10.0, 9.0, 9.0, 1.0, 2.0], "tab2 g b z");
    // firstonly drops b by z; g by z has every column total 1, so chi2 is
    // N (sum of n_ij^2 / (r_i c_j) - 1) = 9 (4/4 + 5/5 - 1) = 9 on 8 df
    let got = rerun(
        &mut e,
        "tab2 g b z, chi2 firstonly",
        "stata_tab2(",
        "length(res), attr(res[[1]], \"chi2\")[\"chi2\"], attr(res[[2]], \"chi2\")[c(\"chi2\", \"df\")]",
    );
    near(
        &got,
        &[2.0, 1.6666666666666665, 9.0, 8.0],
        "tab2, chi2 firstonly",
    );

    // pwcorr: pairwise, so x and z use 9 rows and x and y all 10
    let got = rerun(
        &mut e,
        "pwcorr x z y, sig obs",
        "stata_pwcorr(",
        "res$C[2, 1], res$C[3, 1], res$sig[2, 1], res$sig[3, 1], res$Nobs[2, 1], res$Nobs[3, 1], res$Nobs[2, 2]",
    );
    near(
        &got,
        &[
            0.11881770515720093,
            1.0,
            0.76077618548731318,
            0.0,
            9.0,
            10.0,
            9.0,
        ],
        "pwcorr x z y, sig obs",
    );
    let got = rerun(
        &mut e,
        "pwcorr x z b, sig sidak",
        "stata_pwcorr(",
        "res$sig[2, 1], res$sig[3, 1], res$sig[3, 2]",
    );
    near(
        &got,
        &[
            0.98630969154581571,
            0.81090494048205841,
            0.99461058961983118,
        ],
        "pwcorr, sidak",
    );
    let got = rerun(
        &mut e,
        "pwcorr x z b, sig bonferroni",
        "stata_pwcorr(",
        "res$sig[2, 1], res$sig[3, 1], res$sig[3, 2]",
    );
    near(&got, &[1.0, 1.0, 1.0], "pwcorr, bonferroni is capped at 1");
    let got = rerun(
        &mut e,
        "pwcorr x z b, listwise",
        "stata_pwcorr(",
        "res$C[3, 1], res$C[3, 2]",
    );
    near(
        &got,
        &[-0.22294816068526141, 0.086602540378443893],
        "pwcorr, listwise",
    );
    // the p-value is R's own test of the correlation on the same rows, and
    // Sidak's is 1 - (1 - p)^3 over the three pairs
    let got = rerun(
        &mut e,
        "pwcorr x z b, sig",
        "stata_pwcorr(",
        "res$sig[2, 1], stats::cor.test(study$x, study$z)$p.value, 1 - (1 - res$sig[3, 1])^3",
    );
    near(&got[..1], &got[1..2], "pwcorr's p-value is cor.test's");
    near(&got[2..], &[0.81090494048205841], "Sidak by hand");
    // star() marks, print() blanks, both on the adjusted p
    let code = script(&format!(
        "use \"{dir}/study.dta\"\npwcorr x z b, star(0.5) print(0.6)\n"
    ));
    let text = ok(&mut e, &code);
    assert!(text.contains("-0.2843*"), "{text}");
    assert!(
        !text.contains("0.1188"),
        "print(0.6) leaves out p = 0.76: {text}"
    );

    // summarize, meanonly: r() of the last variable, nothing printed
    let got = rerun(
        &mut e,
        "summarize z, meanonly",
        "stata_r <- ",
        "res$N, res$mean, res$sum, res$min, res$max, res$sum_w",
    );
    near(
        &got,
        &[9.0, 5.0, 45.0, 1.0, 9.0, 9.0],
        "summarize z, meanonly",
    );
    let got = rerun(
        &mut e,
        "summarize x z, meanonly",
        "stata_r <- ",
        "res$N, res$mean",
    );
    near(&got, &[9.0, 5.0], "meanonly keeps the last variable's");
    let code = script(&format!("use \"{dir}/study.dta\"\nsummarize z, meanonly\n"));
    let text = ok(&mut e, &code);
    assert!(!text.contains('5'), "meanonly prints nothing: {text}");

    // regress, beta: e(beta), with the base level of b left out
    let got = rerun(
        &mut e,
        "regress z x i.b, beta",
        "stata_beta <- ",
        "res[2], res[3]",
    );
    near(
        &got,
        &[0.14535034962359536, 0.11900813348198415],
        "regress z x i.b, beta",
    );
    let got = rerun(
        &mut e,
        "regress z x i.b, beta robust",
        "stata_beta <- ",
        "res[2], res[3]",
    );
    near(
        &got,
        &[0.14535034962359536, 0.11900813348198415],
        "beta does not change with robust",
    );
    let got = rerun(
        &mut e,
        "regress z x b, beta noconstant",
        "stata_beta <- ",
        "res[1], res[2]",
    );
    near(
        &got,
        &[0.61886962708309534, 0.44883708634675212],
        "beta, noconstant still uses the centred sd",
    );
    let got = rerun(
        &mut e,
        "regress z x b if g == 1, beta",
        "stata_beta <- ",
        "res[2]",
    );
    near(
        &got,
        &[-0.23408229439226133],
        "beta over the rows an if keeps",
    );
    // and by its definition: b sd(x) / sd(z) over the nine rows with z
    let got = rerun(
        &mut e,
        "regress z x i.b, beta",
        "stata_beta <- ",
        "res[2], local({ k <- !is.na(study$z); stata_b[2] * stats::sd(study$x[k]) / stats::sd(study$z[k]) })",
    );
    near(&got[..1], &got[1..], "beta by hand");
}

#[test]
fn tab2_pwcorr_meanonly_and_beta_say_what_stata_says() {
    assert_eq!(
        refusal("use s\nregress z x, beta vce(cluster g)"),
        "options vce(cluster clustvar) and beta may not be combined"
    );
    assert_eq!(
        refusal("use s\nsummarize z, meanonly detail"),
        "only one of meanonly or detail may be specified"
    );
    assert!(refusal("use s\npwcorr x z, bonferroni sidak").starts_with("invalid syntax"));
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    // in Stata these are errors once the data are seen, so the R stops
    for (cmd, message) in [
        ("tab2 g", "too few variables specified"),
        ("pwcorr s x", "too few variables specified"),
    ] {
        let code = script(&format!("use \"{dir}/study.dta\"\n{cmd}\n"));
        let (outcome, text) = run(&mut e, &code);
        assert_eq!(outcome, Outcome::Error, "{cmd}: {text}");
        assert!(text.contains(message), "{cmd}: {text}");
    }
    // a string variable is left out with Stata's note
    let code = script(&format!("use \"{dir}/study.dta\"\npwcorr x s z\n"));
    let text = ok(&mut e, &code);
    assert!(
        text.contains("(s ignored because string variable)"),
        "{text}"
    );
}

/// `by` before a statistics command or a model, against Stata 19 on the same
/// data: one heading per group in sorted order (value labels shown, a missing
/// value last), and after a model the last group's estimates.
#[test]
fn by_runs_each_group_in_turn_as_stata_19_does() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    let near = |got: &[f64], stata: &[f64], what: &str| {
        assert_eq!(got.len(), stata.len(), "{what}: {got:?} against {stata:?}");
        for (g, s) in got.iter().zip(stata) {
            assert!(
                (g - s).abs() <= 1e-9 * s.abs().max(1.0),
                "{what}: {g} against Stata's {s}"
            );
        }
    };
    let use_ = format!("use \"{dir}/study.dta\"\n");
    let headings = |text: &str| -> Vec<String> {
        text.lines()
            .filter_map(|l| l.strip_prefix("-> "))
            .map(|l| l.trim().to_string())
            .collect()
    };

    // the groups in order, and each group's own figures (by definition: the
    // same command on that group's rows alone)
    let code = script(&format!("{use_}by g: summarize x z\n"));
    let text = ok(&mut e, &code);
    assert_eq!(headings(&text), ["g = 0", "g = 1"], "{text}");
    let v = numbers(
        &mut e,
        "{ a <- stata_summarize(study[study$g == 0, c(\"x\", \"z\")]); b <- stata_summarize(study[study$g == 1, c(\"x\", \"z\")]) }",
        "a$mean, b$mean",
    );
    near(
        &v,
        &[3.0, 4.25, 8.0, 5.6],
        "each group's means, as Stata printed them",
    );
    assert!(text.contains("4.25") && text.contains("5.6"), "{text}");

    // bysort sorts the data; by on rows out of order stops as Stata does
    let code = script(&format!("{use_}by b: summarize x\n"));
    let (outcome, text) = run(&mut e, &code);
    assert_eq!(outcome, Outcome::Error, "{text}");
    assert!(text.contains("not sorted"), "{text}");
    let code = script(&format!("{use_}bysort b: summarize x\nby b: summarize x\n"));
    let text = ok(&mut e, &code);
    assert_eq!(
        headings(&text),
        ["b = 0", "b = 1", "b = 0", "b = 1"],
        "after bysort the rows are in order: {text}"
    );
    assert!(
        text.contains("4.833333"),
        "Stata's mean of x where b = 1: {text}"
    );

    // a missing value is a group of its own, the last
    let code = script(&format!(
        "{use_}replace g = . in 10\nsort g\nby g: summarize x\n"
    ));
    let text = ok(&mut e, &code);
    assert_eq!(headings(&text), ["g = 0", "g = 1", "g = ."], "{text}");

    // value labels in the heading, two variables, and a within-group sort
    let code = script(&format!(
        "{use_}label define gl 0 \"low\" 1 \"high\"\nlabel values g gl\nby g: summarize x\nby g b, sort: summarize x\nbysort g (id): summarize x\n"
    ));
    let text = ok(&mut e, &code);
    assert_eq!(
        headings(&text),
        [
            "g = low",
            "g = high",
            "g = low, b = 0",
            "g = low, b = 1",
            "g = high, b = 0",
            "g = high, b = 1",
            "g = low",
            "g = high"
        ],
        "{text}"
    );

    // a string variable's groups, with tabulate, correlate and ttest inside
    let code = script(&format!(
        "{use_}bysort s: tabulate g b, chi2\nby s: correlate x z\nby s: ttest x, by(g)\n"
    ));
    let text = ok(&mut e, &code);
    assert_eq!(
        headings(&text),
        ["s = a", "s = b", "s = a", "s = b", "s = a", "s = b"]
    );
    // Stata 19: Pearson chi2(1) = 0.1389 and 2.2222; (obs=4) and (obs=5); t = -3 on 3 df
    for want in [
        "chi2(1) =   0.1389",
        "chi2(1) =   2.2222",
        "(obs=4)",
        "(obs=5)",
        "-0.3207",
        "0.3434",
    ] {
        assert!(text.contains(want), "{want} in {text}");
    }
    assert_eq!(text.matches("t = -3, df = 3").count(), 2, "{text}");

    // after a model, e() is the last group's: Stata 19's figures
    let code = script(&format!("{use_}bysort g: regress z x\n"));
    ok(&mut e, &code);
    let v = numbers(
        &mut e,
        "0",
        "stata_b[2], sqrt(stata_vcov[2, 2]), nrow(stata_data)",
    );
    near(
        &v,
        &[-0.40000000000000013, 0.95916630466254393, 5.0],
        "regress after by: the last group",
    );
    let code = script(&format!("{use_}bysort s: logit b x\n"));
    ok(&mut e, &code);
    let v = numbers(
        &mut e,
        "0",
        "stata_b[2], sqrt(stata_vcov[2, 2]), stata_b[1]",
    );
    // to 6 digits: on these five rows Stata stops with a score of about 4e-7,
    // and R's estimates have one of about 1e-15, so the gap is Stata's
    let stata = [
        -0.21974952232028047,
        0.34977355931649418,
        1.7620447717596983,
    ];
    assert_eq!(v.len(), 3, "{v:?}");
    for (g, s) in v.iter().zip(stata) {
        assert!(
            (g - s).abs() <= 1e-6 * s.abs().max(1.0),
            "logit after by: {g} against Stata's {s}"
        );
    }
    // quietly: no headings, the same estimates
    let code = script(&format!("{use_}quietly bysort g: regress z x\n"));
    let text = ok(&mut e, &code);
    assert!(headings(&text).is_empty(), "{text}");
    let v = numbers(&mut e, "0", "stata_b[2]");
    near(&v, &[-0.40000000000000013], "quietly by: regress");

    // Stata's words for in
    assert_eq!(
        refusal("use s\nby g: summarize x in 1/3"),
        "'in' may not be combined with 'by'"
    );
}

/// tabulate, summarize() and test, mtest in Stata 19's layouts
/// (plan/stata/tabulate-summarize.md, plan/stata/lincom-test-lines.md).
#[test]
fn tabulate_summarize_and_test_mtest_print_statas_tables() {
    let Some(mut e) = engine() else { return };
    let code = script(
        "clear\nset obs 8\ngenerate g = 1 + mod(_n, 2)\ngenerate b = _n > 4\ngenerate y = _n * _n / 4\n\
         generate x = mod(_n * 3, 7)\ntabulate g b, summarize(y)\ntabulate g, summarize(y) means\n\
         regress y x b\ntest x b, mtest(bonferroni)\n",
    );
    let text = ok(&mut e, &code);
    for line in [
        "              Means, Standard Deviations and Frequencies of y",
        "         g |         0          1 |     Total",
        "-----------+----------------------+----------",
        "            | Summary of",
        "          g |        Mean",
        "       |     F(df,5)     df      p > F",
        "-------+------------------------------",
        "* Bonferroni-adjusted p-values",
    ] {
        assert!(text.contains(line), "{line:?} in {text}");
    }
}

/// replace's and generate's notes and tsset's Delta line, as Stata 19 prints them
/// (plan/stata/replace-notes.md).
#[test]
fn replace_and_generate_say_what_changed() {
    let Some(mut e) = engine() else { return };
    let code = script(
        "clear\nset obs 10\ngenerate x = _n\ngenerate s = \"a\"\nreplace x = 2*x if _n > 3\nreplace x = . in 1/2\n\
         replace x = x\nreplace x = 5 in 3\nreplace s = \"\" in 1/2\nreplace x = .a in 10\ngenerate y = x/(_n > 5)\n\
         quietly replace y = 2\nbysort s: replace x = _n\ngenerate t = _n\ntsset t\n",
    );
    let text = ok(&mut e, &code);
    for line in [
        "(7 real changes made)",
        "(2 real changes made, 2 to missing)",
        "(0 real changes made)",
        "(1 real change made)",
        "(2 real changes made)\n",
        "(1 real change made, 1 to missing)",
        "(6 missing values generated)",
        "(10 real changes made)",
        "        Delta: 1 unit",
    ] {
        assert!(text.contains(line), "{line:?} in {text}");
    }
    assert_eq!(
        text.matches("real change").count(),
        7,
        "nothing under quietly: {text}"
    );
}

/// table (Stata 17+) in Stata 19's layout, line for line from Stata's own
/// output on the same data (plan/stata/table.md).
#[test]
fn table_prints_statas_layout() {
    let Some(mut e) = engine() else { return };
    let code = script(
        "clear\nset obs 12\ngenerate g = 1 + mod(_n, 3)\nlabel define gl 1 \"one\" 2 \"two\" 3 \"three\"\n\
         label values g gl\ngenerate h = _n > 6\ngenerate x = _n * 1.5\ntable g h\n\
         table g, statistic(mean x) statistic(sd x)\ntable (g h) (), nototals\ntable () (g), statistic(mean x)\n\
         table g h, statistic(frequency) statistic(percent)\n",
    );
    let text = ok(&mut e, &code);
    let lines: Vec<&str> = text.lines().map(str::trim_end).collect();
    for want in [
        "        |        h",
        "        |  0   1   Total",
        "--------+---------------",
        "  Total |  6   6      12",
        "        |   Mean   Standard deviation",
        "  one   |  11.25             5.809475",
        "  Total |   9.75             5.408327",
        "    h   |",
        "      1 |          2",
        "     |    one    two   three   Total",
        "Mean |  11.25   8.25    9.75    9.75",
        "    Frequency |      6       6       12",
        "    Percent   |  50.00   50.00   100.00",
        "---------------------------------------",
    ] {
        assert!(lines.contains(&want), "{want:?} in {text}");
    }
}

/// ranksum, signrank, signtest, kwallis and spearman's matrix in Stata 19's
/// layout, line for line from Stata's own output on the same data
/// (plan/stata/rank-tests.md).
#[test]
fn the_rank_tests_print_statas_layout() {
    let Some(mut e) = engine() else { return };
    let code = script(
        "clear\nset obs 12\ngenerate g = _n > 6\ngenerate k = 1 + mod(_n, 3)\n\
         generate x = mod(_n*7, 11)\ngenerate w = mod(_n*5, 9)\ngenerate z = mod(_n*3, 7)\n\
         ranksum x, by(g)\nsignrank x = w\nsigntest x = w\nkwallis x, by(k)\nspearman x w z\n",
    );
    let text = ok(&mut e, &code);
    let lines: Vec<&str> = text.lines().map(str::trim_end).collect();
    for want in [
        "Two-sample Wilcoxon rank-sum (Mann–Whitney) test",
        "           0 |        6        45.5          39",
        "    Combined |       12          78          78",
        "Adjustment for ties       -0.14",
        "                     ----------",
        "H0: x(g==0) = x(g==1)",
        "         z =  1.043",
        "Exact prob = 0.3312",
        "    Positive |        7          51          39",
        "Adjustment for zeros       0.00",
        "Adjusted variance        161.62",
        "Exact prob = 0.3721",
        "    Negative |           5           6",
        "      Pr(#positive >= 7 or #negative >= 7) =",
        "         min(1, 2*Binomial(n = 12, x >= 7, p = 0.5)) = 0.7744",
        "Kruskal–Wallis equality-of-populations rank test",
        "  | k | Obs | Rank sum |",
        "  | 1 |   4 |    41.50 |",
        "  chi2(2) =  9.548",
        "  chi2(2) with ties =  9.582",
        "               Prob = 0.0083",
        "Number of observations = 12",
        "             |        x        w        z",
        "           z |  -0.0177  -0.9752   1.0000",
    ] {
        assert!(lines.contains(&want), "{want:?} in {text}");
    }
}

/// power in Stata 19's layout, line for line from Stata's own output
/// (plan/stata/power.md): a searched sample size, a power, the paired form's
/// two columns, a total that does not split, and a target value.
#[test]
fn power_prints_statas_layout() {
    let Some(mut e) = engine() else { return };
    let code = script(
        "power onemean 0 0.5\npower pairedmeans 0 0.5, sd(1) corr(0.5)\n\
         power twoproportions 0.3 0.5, nratio(1.5) continuity\npower twomeans 0 0.5, n(101)\n\
         power oneproportion 0.5, n(80) power(0.8)\n",
    );
    let text = ok(&mut e, &code);
    let lines: Vec<&str> = text.lines().map(str::trim_end).collect();
    for want in [
        "Performing iteration ...",
        "Estimated sample size for a one-sample mean test",
        "H0: m = m0  versus  Ha: m != m0",
        "        delta =    0.5000",
        "            N =        34",
        "Paired t test assuming sd1 = sd2 = sd",
        "        delta =    0.5000           sd =    1.0000",
        "           d0 =    0.0000         corr =    0.5000",
        "         sd_d =    1.0000",
        "Pearson's chi-squared test",
        "        delta =    0.2000  (difference)",
        "           N2 =       131",
        "        N2/N1 =    1.5057",
        "Actual sample sizes:",
        "  N per group =        50",
        "        power =    0.6969",
        "H0: p = p0  versus  Ha: p != p0; pa > p0",
        "Estimated effect size and target proportion:",
        "           pa =    0.6543",
    ] {
        assert!(lines.contains(&want), "{want:?} in {text}");
    }
}

/// power's variance, correlation and paired-proportion methods in Stata 19's
/// layout, line for line from Stata's own output (plan/stata/power.md): the
/// sd scale's names, a closed-form sample size whose groups miss nratio(),
/// a searched target's side, and the discordant proportions worked out from
/// their sum.
#[test]
fn power_variances_correlations_and_paired_proportions_print_statas_layout() {
    let Some(mut e) = engine() else { return };
    let code = script(
        "power onevariance 2, sd n(30) power(0.9)\npower twovariances 1 2, nratio(1.3)\n\
         power twocorrelations 0.2 0.5, nratio(1.3) onesided\npower onecorrelation 0.2, n(50) power(0.8)\n\
         power pairedproportions, n(100) prdiscordant(0.3) power(0.8)\n",
    );
    let text = ok(&mut e, &code);
    let lines: Vec<&str> = text.lines().map(str::trim_end).collect();
    for want in [
        "Estimated target standard deviation for a one-sample standard-deviation test",
        "Chi-squared test",
        "H0: s = s0  versus  Ha: s != s0; sa > s0",
        "        delta =    1.5208",
        "           sa =    3.0417",
        "Estimated sample sizes for a two-sample variances test",
        "F test",
        "           N1 =        59",
        "           N2 =        77",
        "        N2/N1 =    1.3051",
        "Fisher's z test",
        "H0: r2 = r1  versus  Ha: r2 > r1",
        "            N =       217",
        "Estimated target correlation for a one-sample correlation test",
        "H0: r = r0  versus  Ha: r != r0; ra > r0",
        "           ra =    0.5451",
        "Large-sample McNemar's test",
        "H0: p21 = p12  versus  Ha: p21 != p12; p21 > p12",
        "    p12 + p21 =    0.3000",
        "Estimated effect size and discordant proportions:",
        "        delta =    0.1516  (difference)",
        "          p12 =    0.0742",
        "          p21 =    0.2258",
    ] {
        assert!(lines.contains(&want), "{want:?} in {text}");
    }
}

/// power's table for lists of values in Stata 19's layout, line for line from
/// Stata's own output (plan/stata/power.md): every combination in Stata's
/// order, the N2/N1 column with nratio(), totals that do not split (with the
/// note under the table), a wide column, and %7.0gc dropping commas that do
/// not fit.
#[test]
fn power_lists_print_statas_table() {
    let Some(mut e) = engine() else { return };
    let code = script(
        "power twomeans 0 0.5, n(50 100) nratio(1 2)\n\
         power pairedproportions, n(100 200) prdiscordant(0.3) power(0.8)\n\
         power twomeans 0 0.2, n(100000)  table\n",
    );
    let text = ok(&mut e, &code);
    let lines: Vec<&str> = text.lines().map(str::trim_end).collect();
    for want in [
        "  +---------------------------------------------------------------------------------+",
        "  |   alpha   power       N      N1      N2   N2/N1   delta      m1      m2      sd |",
        "  |---------------------------------------------------------------------------------|",
        "  |     .05   .4101      50      25      25       1      .5       0      .5       1 |",
        "  |     .05   .3625      50      16      33       2      .5       0      .5       1 |",
        "  |     .05   .6413     100      33      66       2      .5       0      .5       1 |",
        "   Note: N1 and N2 have been rounded down.",
        "H0: p21 = p12  versus  Ha: p21 != p12; p21 > p12",
        "  |   alpha   power       N   delta     p12     p21 prdiscordant |",
        "  |     .05      .8     100   .1516  .07418   .2258           .3 |",
        "  |     .05       1 1.0e+05   50000   50000      .2       0      .2       1 |",
    ] {
        assert!(lines.contains(&want), "{want:?} in {text}");
    }
}

/// power oneway in Stata 19's layout, line for line from Stata's own output
/// (plan/stata/power.md): a searched sample size from the means, unequal
/// groups' average, and the between-group variance.
#[test]
fn power_oneway_prints_statas_layout() {
    let Some(mut e) = engine() else { return };
    let code = script(
        "power oneway 1 2 3\npower oneway 1 2 3, n1(10) n2(20) n3(30)\n\
         power oneway, n(60) ngroups(4) power(0.8)\n",
    );
    let text = ok(&mut e, &code);
    let lines: Vec<&str> = text.lines().map(str::trim_end).collect();
    for want in [
        "Performing iteration ...",
        "Estimated sample size for one-way ANOVA",
        "F test for group effect",
        "H0: delta = 0  versus  Ha: delta != 0",
        "          N_g =         3",
        "        Var_m =    0.6667",
        "Estimated sample sizes:",
        "            N =        18",
        "  N per group =         6",
        "    Average N =   20.0000",
        "           N3 =        30",
        "        power =    0.9995",
        "Estimated effect size and between-group variance:",
        "        delta =    0.4415",
        "        Var_m =    0.1949",
    ] {
        assert!(lines.contains(&want), "{want:?} in {text}");
    }
}

/// cs, cc and ir in Stata 19's layout, line for line from Stata's own output
/// on the same data (plan/stata/epitab.md); replace keeps the case variable's
/// label, which then names ir's row.
#[test]
fn the_epidemiological_tables_print_statas_layout() {
    let Some(mut e) = engine() else { return };
    let code = script(
        "clear\nset obs 60\ngenerate id = _n\ngenerate exposed = mod(id, 3)\n\
         generate case = mod(id, 4) == 0\ngenerate pt = 10 + mod(id, 7)\n\
         label variable case \"Disease\"\nlabel variable pt \"Person-years\"\n\
         cs case exposed\nreplace exposed = exposed > 0\nreplace case = . in 1/3\n\
         ir case exposed pt\ncsi 10 30 90 70\ncci 15 40 60 50, woolf\n",
    );
    let text = ok(&mut e, &code);
    let lines: Vec<&str> = text.lines().map(str::trim_end).collect();
    for want in [
        "                 |        exposed         |",
        "                 |   Exposed   Unexposed  |      Total",
        "           Cases |        10           5  |         15",
        " Risk difference |                0       |   -.2324231    .2324231",
        " Attr. frac. ex. |                0       |    -1.53373     .605325",
        "                               chi2(1) =     0.00  Pr>chi2 = 1.0000",
        "Incidence-rate comparison",
        "         Disease |        10           5  |         15",
        "    Person-years |       492         250  |        742",
        " Inc. rate ratio |          1.01626       |    .3164937    3.789276 (exact)",
        "  Adj Pr(Exposed Disease <= 10) = 0.5003 (lower one-sided)",
        "              Two-sided p-value = 0.9994",
        " Prev. frac. ex. |         .6666667       |    .3551839    .8276856",
        " Prev. frac. pop |         .3333333       |",
        "                                                         Proportion",
        "           Cases |        15          40  |         55       0.2727",
        "      Odds ratio |            .3125       |    .1548526    .6306399 (Woolf)",
        "                 +-------------------------------------------------",
    ] {
        assert!(lines.contains(&want), "{want:?} in {text}");
    }
}

#[test]
fn mvtest_prints_statas_layout() {
    let Some(mut e) = engine() else { return };
    let code = script(
        "clear\nset obs 60\ngenerate double x1 = mod(_n * 37, 101) / 7\n\
         generate double x2 = mod(_n * 13, 17) / 3 + x1 / 4\n\
         generate double x3 = mod(_n * 11, 23) / 4 - x2 / 5\ngenerate int g = 1 + mod(_n, 3)\n\
         mvtest means x1 x2 x3\nmvtest means x1 x2 x3, by(g)\nmvtest covariances x1 x2 x3, by(g)\n\
         mvtest correlations x1 x2 x3\nmvtest normality x1 x2 x3, stats(all) bivariate univariate\n\
         capture noisily mvtest bogus x1\ndisplay _rc\n",
    );
    let text = ok(&mut e, &code);
    let lines: Vec<&str> = text.lines().map(str::trim_end).collect();
    for want in [
        "          Hotelling T2 =     74.00",
        "     Hotelling F(2,58) =     36.37",
        "              Prob > F =    0.0000",
        "              Wilks' lambda |    0.9872     6.0     110.0      0.12   0.9940 e",
        "         Roy's largest root |    0.0114     3.0      56.0      0.21   0.8876 u",
        "                            e = exact, a = approximate, u = upper bound on F",
        "      Modified LR chi2 =  6.919132",
        "     Box F(12,15745.2) =      0.53      Prob > F =  0.8952",
        "          Box chi2(12) =      6.39   Prob > chi2 =  0.8950",
        "        Lawley chi2(2) =     24.86",
        "              x1 |     0.9920         0.0001          12.82       0.0016",
        "    x1            x2           |      7.81     4     0.0987",
        "                  x3           |     10.53     4     0.0324",
        "    Mardia mSkewness =  .2200014    chi2(10) =    2.368   Prob>chi2 =  0.9927",
        "    Henze-Zirkler    =  .7646136     chi2(1) =    0.442   Prob>chi2 =  0.5060",
        "    Doornik-Hansen                   chi2(6) =   12.802   Prob>chi2 =  0.0463",
        "unknown subcommand of mvtest: bogus",
        "198",
    ] {
        assert!(lines.contains(&want), "{want:?} in {text}");
    }
}

#[test]
fn the_stratified_epidemiological_tables_print_statas_layout() {
    let Some(mut e) = engine() else { return };
    let code = script(
        "clear\nset obs 400\ngenerate id = _n\ngenerate byte age = 1 + mod(id * 7, 3)\n\
         generate byte exposed = mod(id * 13, 10) < 4\n\
         generate byte case = mod(id * 37 + 11 * age, 23) < 3 + 5 * exposed + age\n\
         generate double pt = 50 + mod(id * 29, 450)\ngenerate byte dose = mod(id * 11, 4)\n\
         cs case exposed, by(age)\ncc case exposed, by(age) tb\nir case exposed pt, by(age)\n\
         tabodds case dose\n",
    );
    let text = ok(&mut e, &code);
    let lines: Vec<&str> = text.lines().map(str::trim_end).collect();
    for want in [
        "             age |  Risk ratio    [95% conf. interval]   M\u{2013}H weight",
        "               1 |    2.636095     1.460382   4.758341     5.082707",
        "           Crude |         2.1     1.551358   2.842671",
        "    M\u{2013}H combined |    2.098553     1.551099   2.839228",
        "Test of homogeneity (M\u{2013}H)      chi2(2) =    0.856  Pr>chi2 = 0.6517",
        "               1 |   3.835897      1.744069   8.436656     2.932331 (tb)",
        "    M\u{2013}H combined |   2.961914      1.916229   4.578229              (tb)",
        "                                Mantel\u{2013}Haenszel chi2(1) =     23.88",
        "Stratified incidence-rate analysis",
        "               2 |   1.794585      .9246482   3.559075     7.485147 (exact)",
        "Test of homogeneity (M\u{2013}H): chi2(2) = 1.17          Pr>chi2 = 0.5567",
        "          0 |         30           70    0.42857        0.27943   0.65731",
        "Test of homogeneity (equal odds): chi2(3)  =     0.67",
        "Score test for trend of odds:     chi2(1)  =     0.24",
    ] {
        assert!(lines.contains(&want), "{want:?} in {text}");
    }
}

#[test]
fn ktau_prints_statas_layout() {
    let Some(mut e) = engine() else { return };
    let code = script(
        "clear\nset obs 40\ngenerate double x1 = mod(_n * 37, 101) / 7\n\
         generate double x2 = mod(_n * 13, 17) / 3\ngenerate double x3 = mod(_n * 11, 23) / 4\n\
         ktau x1 x2\nktau x1 x2 x3\nktau x1 x2 x3, stats(taub se score)\n",
    );
    let text = ok(&mut e, &code);
    let lines: Vec<&str> = text.lines().map(str::trim_end).collect();
    for want in [
        "  Number of obs =      40",
        "Kendall's tau-a =       0.0474",
        "Kendall's score =      37",
        "    SE of score =      85.637   (corrected for ties)",
        "     Prob > |z| =       0.6742  (continuity corrected)",
        "Number of observations = 40",
        "          x2 |     0.0474     0.9628",
        "|   se of score   |",
        "             |   780.0000",
        "          x3 |    -0.0402    -0.0978     1.0000",
    ] {
        assert!(lines.contains(&want), "{want:?} in {text}");
    }
}

#[test]
fn testnl_prints_statas_layout() {
    let Some(mut e) = engine() else { return };
    let code = script(
        "clear\nset obs 60\ngenerate double z = mod(_n * 37, 101) / 7\n\
         generate double w = mod(_n * 13, 17) / 3\n\
         generate double x = 1 + .4 * z + .9 * w + mod(_n * 11, 23) / 5\nquietly regress x z w\n\
         testnl _b[z] / _b[w] = 0.1\ntestnl (_b[z] = 0.4) (_b[w] = 1), mtest\n",
    );
    let text = ok(&mut e, &code);
    let lines: Vec<&str> = text.lines().map(str::trim_end).collect();
    for want in [
        "  (1)  _b[z] / _b[w] = 0.1",
        "               chi2(1) =       21.74",
        "           Prob > chi2 =        0.0000",
        "  (1)  |        0.37      1     0.5429*",
        "  All  |        0.41      2     0.8160",
        "* Unadjusted p-values",
    ] {
        assert!(lines.contains(&want), "{want:?} in {text}");
    }
}

#[test]
fn rotate_prints_statas_layout() {
    let Some(mut e) = engine() else { return };
    let code = script(
        "clear\nset obs 90\ngenerate double f = mod(_n * 37, 101) / 101 - .5\n\
         generate double g = mod(_n * 53, 97) / 97 - .5\n\
         generate double h = mod(_n * 29, 89) / 89 - .5\n\
         generate double x1 = f + .5 * (mod(_n * 11, 83) / 83 - .5)\n\
         generate double x2 = f + .3 * g + .6 * (mod(_n * 17, 79) / 79 - .5)\n\
         generate double x3 = -f + .7 * (mod(_n * 19, 73) / 73 - .5)\n\
         generate double x4 = .5 * f + g + .5 * (mod(_n * 23, 71) / 71 - .5)\n\
         generate double x5 = g + .4 * (mod(_n * 31, 67) / 67 - .5)\n\
         generate double x6 = h + .3 * g + .5 * (mod(_n * 41, 61) / 61 - .5)\n\
         generate double x7 = h - .4 * f + .5 * (mod(_n * 43, 59) / 59 - .5)\n\
         quietly factor x1-x7, factors(2)\nrotate\nrotate, promax blanks(.3)\n\
         quietly pca x1-x7, components(2)\nrotate\n",
    );
    let text = ok(&mut e, &code);
    let lines: Vec<&str> = text.lines().map(str::trim_end).collect();
    for want in [
        "    Rotation: orthogonal varimax (Kaiser off)    Number of params =         13",
        "         Factor  |     Variance   Difference        Proportion   Cumulative",
        "        Factor1  |      2.60621      0.77299            0.5473       0.5473",
        "Rotated factor loadings (pattern matrix) and unique variances",
        "              x4 |   0.5969    0.6680 |      0.1975",
        "         Factor1 |  0.9499   0.3127",
        "    Rotation: oblique promax (Kaiser off)        Number of params =         13",
        "         Factor  |     Variance   Proportion    Rotated factors are correlated",
        "        Factor2  |      1.84520       0.3875",
        "              x6 |             0.8046 |      0.3500",
        "    (blanks represent abs(loading)<.3)",
        "         Factor2 | -0.2900   0.9433",
        "    Rotation: orthogonal varimax (Kaiser off)    Rho              =     0.7248",
        "           Comp1 |      2.95856      .843423             0.4227       0.4227",
        "              x6 |  -0.0542    0.6119 |        .214",
        "Component rotation matrix",
        "           Comp2 |  -0.2401    0.9707",
    ] {
        assert!(lines.contains(&want), "{want:?} in {text}");
    }
}

#[test]
fn kap_prints_statas_layout() {
    let Some(mut e) = engine() else { return };
    let code = script(
        "clear\nset obs 80\ngenerate int r1 = 1 + mod(_n * 7, 3)\ngenerate int r2 = r1\n\
         replace r2 = 1 + mod(r2, 3) if mod(_n, 6) == 0\n\
         replace r2 = 1 + mod(r2 + 1, 3) if mod(_n, 11) == 0\ngenerate int r3 = r1\n\
         replace r3 = 1 + mod(r3, 3) if mod(_n, 5) == 0\n\
         kap r1 r2, tab\nkap r1 r2, wgt(w)\nkap r1 r2 r3\n",
    );
    let text = ok(&mut e, &code);
    let lines: Vec<&str> = text.lines().map(str::trim_end).collect();
    for want in [
        "        r1 |         1          2          3 |     Total",
        "         1 |        13         12          1 |        26",
        "Agreement   agreement     Kappa   Std. err.         Z      Prob>Z",
        "  77.50%      33.52%     0.6616     0.0764       8.66      0.0000",
        "Ratings weighted by:",
        "   0.5000   1.0000   0.5000",
        "  88.13%      58.52%     0.7137     0.0828       8.62      0.0000",
        "There are 3 raters per subject:",
        "               2 |    0.5803       8.99    0.0000",
        "        combined |    0.6027      13.15    0.0000",
    ] {
        assert!(lines.contains(&want), "{want:?} in {text}");
    }
}

#[test]
fn tetrachoric_prints_statas_layout() {
    let Some(mut e) = engine() else { return };
    let code = script(
        "clear\nset obs 120\ngenerate byte b1 = mod(_n * 7, 11) < 5\n\
         generate byte b3 = (mod(_n * 13, 17) + 2 * b1) > 9\n\
         generate byte b4 = mod(_n * 5, 9) < 4\n\
         tetrachoric b1 b3\ntetrachoric b1 b3 b4, stats(rho se p) star(.05)\n",
    );
    let text = ok(&mut e, &code);
    let lines: Vec<&str> = text.lines().map(str::trim_end).collect();
    for want in [
        "   Number of obs =          120",
        " Tetrachoric rho =       0.2278",
        "       Std error =       0.1386",
        "Test of H0: b1 and b3 are independent",
        " 2-sided exact P =       0.1419",
        "(obs=120)",
        "|   2-sided exact P |",
        "             |       b1       b3       b4",
        "          b4 |  -0.0453  -0.0093   1.0000",
        "             |   0.1442   0.1441   0.0000",
        "             |   0.8538   1.0000",
    ] {
        assert!(lines.contains(&want), "{want:?} in {text}");
    }
}

#[test]
fn icc_prints_statas_layout() {
    let Some(mut e) = engine() else { return };
    let code = script(
        "clear\nset obs 72\ngenerate int target = 1 + mod(_n - 1, 12)\n\
         generate int judge = 1 + floor((_n - 1) / 12)\n\
         generate double score = 10 + target / 3 + judge / 5 + mod(_n * 37, 23) / 11\n\
         icc score target judge\nicc score target, testvalue(.2)\n\
         replace score = . in 5\nreplace score = . in 9\nicc score target judge\n",
    );
    let text = ok(&mut e, &code);
    let lines: Vec<&str> = text.lines().map(str::trim_end).collect();
    for want in [
        "Two-way random-effects model",
        "Absolute agreement",
        "Random effects: target           Number of targets =        12",
        "Random effects: judge            Number of raters  =         6",
        "                 score |        ICC       [95% conf. interval]",
        "            Individual |    .703946       .4888697    .8826017",
        "               Average |   .9344974       .8516032    .9783118",
        "  ICC=0.00: F(11.0, 55.0) = 18.83             Prob > F = 0.000",
        "Note: ICCs estimate correlations between individual measurements",
        "One-way random-effects model",
        "                                 Number of raters  =         6",
        "  ICC(1)=0.20: F(11.0, 60.0) = 6.03           Prob > F = 0.000",
        "  ICC(k)=0.20: F(11.0, 60.0) = 12.05          Prob > F = 0.000",
        "(2 targets omitted from computation because not rated by all raters)",
        "Random effects: target           Number of targets =        10",
    ] {
        assert!(lines.contains(&want), "{want:?} in {text}");
    }
}

#[test]
fn mhodds_prints_statas_layout() {
    let Some(mut e) = engine() else { return };
    let code = script(
        "clear\nset obs 300\ngenerate byte agegrp = 1 + mod(_n, 3)\n\
         generate byte sex = mod(floor(_n / 3), 2)\n\
         generate byte smoke = mod(_n * 7, 10) < 3 + agegrp\n\
         generate byte dose = 1 + mod(_n * 11, 4)\n\
         generate byte case = mod(_n * 13 + 5 * smoke + 3 * dose + agegrp, 17) < 6\n\
         mhodds case smoke agegrp\nmhodds case dose\nmhodds case smoke sex, by(agegrp)\n\
         mhodds case smoke, by(agegrp sex)\n",
    );
    let text = ok(&mut e, &code);
    let lines: Vec<&str> = text.lines().map(str::trim_end).collect();
    for want in [
        "Mantel\u{2013}Haenszel estimate of the odds ratio comparing smoke==1 vs.",
        "  smoke==0",
        "  controlling for agegrp",
        "     Odds ratio    chi2(1)        P>chi2        [95% conf. interval]",
        "       1.144168       0.29        0.5876         0.702979   1.862247",
        "Score test for trend of odds with dose",
        "       0.925559       0.50        0.4783         0.747386   1.146208",
        "Note: The Odds ratio estimate is an approximation to the odds ratio",
        "      for a one-unit increase in dose.",
        "Mantel\u{2013}Haenszel estimate of the odds ratio comparing smoke==1 vs. smoke==0",
        "   agegrp | Odds ratio        chi2(1)         P>chi2       [95% conf. interval]",
        "        1 |   1.256410           0.27         0.6032         0.53034    2.97653",
        "        3 |   1.000000           0.00         1.0000         0.42796    2.33669",
        "Mantel\u{2013}Haenszel estimate controlling for sex and agegrp",
        "       1.145889       0.30        0.5867         0.701086   1.872895",
        "Approximate test of homogeneity of odds ratios: chi2(2)   =     0.15",
        "                                                Pr>chi2   =   0.9254",
        "   agegrp       sex | Odds ratio     chi2(1)      P>chi2   [95% conf. interval]",
        "--------------------+----------------------------------------------------------",
        "        2         1 |   0.928571        0.01      0.9040     0.27853    3.09571",
        "Approximate test of homogeneity of odds ratios: chi2(5)   =     0.63",
    ] {
        assert!(lines.contains(&want), "{want:?} in {text}");
    }
}

#[test]
fn tabodds_or_prints_statas_layout() {
    let Some(mut e) = engine() else { return };
    let code = script(
        "clear\nset obs 300\ngenerate byte dose = 1 + mod(_n * 11, 4)\n\
         generate byte case = mod(_n * 13 + 3 * dose, 17) < 6\n\
         generate byte w = 1 + mod(_n, 3)\n\
         tabodds case dose, or base(3)\ntabodds case dose [fw=w], or\n",
    );
    let text = ok(&mut e, &code);
    let lines: Vec<&str> = text.lines().map(str::trim_end).collect();
    for want in [
        "        dose |  Odds ratio       chi2       P>chi2     [95% conf. interval]",
        "-------------+-------------------------------------------------------------",
        "           1 |    0.841661       0.26       0.6124      0.431667   1.641065",
        "           3 |    1.000000          .           .              .          .",
        "           4 |    0.746450       0.72       0.3946      0.379784   1.467120",
        "Test of homogeneity (equal odds): chi2(3)  =     0.82",
        "           2 |    0.942824       0.06       0.8086      0.585476   1.518283",
        "Test of homogeneity (equal odds): chi2(3)  =     1.55",
        "                                  Pr>chi2  =   0.7596",
    ] {
        assert!(lines.contains(&want), "{want:?} in {text}");
    }
}

#[test]
fn tabodds_adjust_and_binomial_print_statas_layout() {
    let Some(mut e) = engine() else { return };
    let code = script(
        "clear\nset obs 300\ngenerate byte dose = 1 + mod(_n * 11, 4)\n\
         generate byte case = mod(_n * 13 + 3 * dose, 17) < 6\n\
         generate byte w = 1 + mod(_n, 3)\ngenerate byte agegrp = 1 + mod(_n, 3)\n\
         generate byte sex = mod(floor(_n / 3), 2)\n\
         tabodds case dose [fw=w], adjust(agegrp sex) base(2)\ngenerate byte trials = 1\n\
         collapse (sum) case (sum) trials, by(dose agegrp)\ntabodds case dose, binomial(trials)\n",
    );
    let text = ok(&mut e, &code);
    let lines: Vec<&str> = text.lines().map(str::trim_end).collect();
    for want in [
        "Mantel\u{2013}Haenszel odds ratios adjusted for agegrp and sex",
        "           1 |           .          .           .              .          .",
        "           2 |    1.000000          .           .              .          .",
        "           4 |    0.913495       0.13       0.7139      0.563059   1.482033",
        "Score test for trend of odds: chi2(1)  =    0.03",
        "                              Pr>chi2  =  0.8646",
        "          1 |         26           49    0.53061        0.32980   0.85370",
        "          4 |         24           51    0.47059        0.28969   0.76444",
        "Test of homogeneity (equal odds): chi2(3)  =     0.82",
        "                                  Pr>chi2  =   0.9137",
    ] {
        assert!(lines.contains(&want), "{want:?} in {text}");
    }
}

#[test]
fn lroc_prints_statas_layout() {
    let Some(mut e) = engine() else { return };
    let code = script(
        "clear\nset obs 60\ngenerate double x = mod(_n * 7, 13) / 4 - 1.5\n\
         generate byte g = mod(_n, 3)\ngenerate byte w = 1 + mod(_n, 4)\n\
         generate byte y = mod(_n * 11 + 3 * g, 7) < 2 + (x > 0) + g\n\
         replace x = . in 5\n\
         logit y x g\nlroc\nlroc if g == 1, nograph\n\
         probit y x g\nlroc, nograph\n\
         logit y x g [fw=w]\nlroc, nograph\n\
         quietly lroc, nograph\ndisplay r(N)\n",
    );
    let text = ok(&mut e, &code);
    let lines: Vec<&str> = text.lines().map(str::trim_end).collect();
    for want in [
        "Logistic model for y",
        "Number of observations =       59",
        "Area under ROC curve   =   0.6748",
        "Number of observations =       20",
        "Area under ROC curve   =   0.5505",
        "Probit model for y",
        "Number of observations =      148",
        "Area under ROC curve   =   0.6455",
    ] {
        assert!(lines.contains(&want), "{want:?} in {text}");
    }
    assert_eq!(
        lines.iter().filter(|l| l.starts_with("Area under")).count(),
        4,
        "{text}"
    );
}
