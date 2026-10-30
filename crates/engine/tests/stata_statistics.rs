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
    assert!(text.contains("total 10"), "{text}");
    assert!(text.contains("Pearson chi2"), "{text}");
    assert!(text.contains("row percentages"), "{text}");

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
        refusal("use \"d.dta\"\nsummarize x, meanonly\n").contains("meanonly"),
        "an option that is not translated should be named"
    );
    assert!(
        refusal("use \"d.dta\"\ntabulate a b c\n").contains("one or two variables"),
        "tab1 and tab2 are not this command"
    );
    assert!(
        refusal("use \"d.dta\"\nbysort g: summarize x\n").contains("by with summarize"),
        "by is not translated yet"
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
