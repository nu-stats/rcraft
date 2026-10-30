//! P2B.4: Stata's weights in the statistics commands and the models, run in
//! a real R and held to Stata 19 SE's own numbers on the same data (batch
//! mode, 2026-10-30, `%24.17g`), with a few worked out a second way: a
//! frequency weight as that many copies of the row.
//!
//! Without a supported R, or without dplyr and haven, the test is skipped.

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

/// A folder of this test's own, with the dataset written into it.
fn fixture(e: &mut Engine) -> String {
    let dir = std::env::temp_dir().join(format!(
        "rcraft-stata-weights-{}-{}",
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

/// The do-file's start: the data, a frequency weight `f` (1 to 3) and an
/// analytic one `a = id / 3`, which `generate` stores as a float, as Stata
/// does, so its sum is Stata's 17.333333283662796.
fn start(dir: &str) -> String {
    format!(
        "use \"{dir}/study.dta\"\ngen f = mod(id, 3) + 1\ngen a = id / 3\ngen zz = f\nreplace zz = 0 in 2\ngen n = a - 1\n"
    )
}

/// Runs `cmd` after the start, then reruns the line beginning `begin` and
/// reads `show` from what it returns (`res`).
fn rerun(e: &mut Engine, dir: &str, cmd: &str, begin: &str, show: &str) -> Vec<f64> {
    let code = script(&format!("{}{cmd}\n", start(dir)));
    ok(e, &code);
    let line = code
        .lines()
        .rfind(|l| l.starts_with(begin))
        .unwrap_or_else(|| panic!("no {begin} line:\n{code}"))
        .to_string();
    numbers(e, &format!("res <- {line}"), show)
}

/// Runs `cmd` after the start and reads `show`.
fn after(e: &mut Engine, dir: &str, cmd: &str, show: &str) -> Vec<f64> {
    let code = script(&format!("{}{cmd}\n", start(dir)));
    ok(e, &code);
    numbers(e, "0", show)
}

fn near(got: &[f64], stata: &[f64], tol: f64, what: &str) {
    assert_eq!(got.len(), stata.len(), "{what}: {got:?} against {stata:?}");
    for (g, s) in got.iter().zip(stata) {
        assert!(
            (g - s).abs() <= tol * s.abs().max(1.0),
            "{what}: {g} against Stata's {s}"
        );
    }
}

const SE: &str = "stata_b[2], sqrt(stata_vcov[2, 2]), stata_b[1], sqrt(stata_vcov[1, 1])";

#[test]
fn summarize_with_each_kind_of_weight_agrees_with_stata_19() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    let e = &mut e;
    let show = "res$obs, res$sum_w, res$mean, res$sd";
    let got = rerun(e, &dir, "summarize z [fw=f]", "stata_summarize(", show);
    near(
        &got,
        &[19.0, 19.0, 4.6315789473684212, 2.6291216597476783],
        1e-12,
        "fweights",
    );
    let got = rerun(e, &dir, "summarize z [aw=a]", "stata_summarize(", show);
    near(
        &got,
        &[
            9.0,
            17.333333283662796,
            5.1538461863818252,
            2.6831575214897931,
        ],
        1e-12,
        "aweights",
    );
    let got = rerun(e, &dir, "summarize z [iw=a]", "stata_summarize(", show);
    near(
        &got,
        &[
            9.0,
            17.333333283662796,
            5.1538461863818252,
            2.6059947735847397,
        ],
        1e-12,
        "iweights",
    );
    let pcts = "res$p1, res$p5, res$p10, res$p25, res$p50, res$p75, res$p90, res$p95, res$p99";
    let got = rerun(
        e,
        &dir,
        "summarize z [fw=f], detail",
        "stata_summarize(",
        &format!("res$skewness, res$kurtosis, {pcts}"),
    );
    near(
        &got,
        &[
            0.036018791126580449,
            1.6719124429900281,
            1.0,
            1.0,
            1.0,
            2.0,
            5.0,
            7.0,
            8.0,
            9.0,
            9.0,
        ],
        1e-12,
        "fweights, detail",
    );
    let got = rerun(
        e,
        &dir,
        "summarize z [aw=a], detail",
        "stata_summarize(",
        &format!("res$skewness, res$kurtosis, {pcts}"),
    );
    near(
        &got,
        &[
            -0.14465906175359994,
            1.8378042597317654,
            1.0,
            1.0,
            2.0,
            3.0,
            6.0,
            7.0,
            9.0,
            9.0,
            9.0,
        ],
        1e-12,
        "aweights, detail",
    );
    // a frequency weight is that many copies: the same as the rows repeated
    let got = after(
        e,
        &dir,
        "summarize z [fw=f]",
        "local({ k <- !is.na(stata_rows$z); r <- rep(stata_rows$z[k], stata_rows$f[k]); c(mean(r), stats::sd(r), unname(stats::quantile(r, 0.25, type = 2))) })",
    );
    near(
        &got,
        &[4.6315789473684212, 2.6291216597476783, 2.0],
        1e-12,
        "fweights as copies",
    );
    let got = rerun(
        e,
        &dir,
        "summarize z [fw=f], meanonly",
        "stata_r <- ",
        "res$N, res$mean, res$sum",
    );
    near(&got, &[19.0, 4.6315789473684212, 88.0], 1e-12, "meanonly");
    // a zero weight leaves the row out, for both kinds
    let got = rerun(
        e,
        &dir,
        "summarize x [fw=zz]",
        "stata_summarize(",
        "res$obs, res$mean",
    );
    near(&got, &[17.0, 5.9411764705882355], 1e-12, "a zero fweight");
    let got = rerun(
        e,
        &dir,
        "summarize x [aw=zz]",
        "stata_summarize(",
        "res$obs, res$mean",
    );
    near(&got, &[9.0, 5.9411764705882355], 1e-12, "a zero aweight");
}

#[test]
fn tabulate_and_correlate_with_weights_agree_with_stata_19() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    let e = &mut e;
    let got = rerun(e, &dir, "tabulate g [fw=f]", "stata_tabulate(", "res$freq");
    near(&got, &[11.0, 9.0], 1e-12, "tabulate, fweights");
    // aweights: the sums scaled to the ten rows; Stata prints 2.72727275
    let got = rerun(e, &dir, "tabulate g [aw=a]", "stata_tabulate(", "res$freq");
    near(&got, &[2.72727275, 7.27272725], 1e-8, "tabulate, aweights");
    let got = rerun(e, &dir, "tabulate g [iw=a]", "stata_tabulate(", "res$freq");
    near(&got, &[5.00000003, 13.3333333], 1e-8, "tabulate, iweights");
    let got = rerun(
        e,
        &dir,
        "tabulate g b [fw=f], chi2",
        "stata_tabulate(",
        "res[1, 1], res[1, 2], res[2, 1], res[2, 2], attr(res, \"chi2\")[c(\"chi2\", \"p\")]",
    );
    near(
        &got,
        &[3.0, 8.0, 6.0, 3.0, 3.1037649219467407, 0.078111456735083046],
        1e-12,
        "tabulate g b, fweights",
    );
    let got = rerun(e, &dir, "tab1 g [fw=f]", "stata_tab1(", "res$g$freq");
    near(&got, &[11.0, 9.0], 1e-12, "tab1, fweights");

    let got = rerun(
        e,
        &dir,
        "correlate x z [fw=f]",
        "stata_correlate(",
        "res[2, 1]",
    );
    near(&got, &[0.22283200142392168], 1e-12, "correlate, fweights");
    let got = rerun(
        e,
        &dir,
        "correlate x z [aw=a]",
        "stata_correlate(",
        "res[2, 1]",
    );
    near(&got, &[0.085799017723590165], 1e-12, "correlate, aweights");
    let got = rerun(
        e,
        &dir,
        "correlate x z [fw=f], covariance",
        "stata_correlate(",
        "res[2, 1], res[2, 2]",
    );
    near(
        &got,
        &[1.7397660818713454, 6.9122807017543861],
        1e-12,
        "covariance, fweights",
    );
    let got = rerun(
        e,
        &dir,
        "correlate x z [aw=a], covariance",
        "stata_correlate(",
        "res[2, 1], res[2, 2]",
    );
    near(
        &got,
        &[0.56582836300587458, 7.1993342851272484],
        1e-12,
        "covariance, aweights",
    );
    let code = script(&format!("{}correlate x z [fw=f]\n", start(&dir)));
    assert!(
        ok(e, &code).contains("(obs=19)"),
        "the obs are the weights' sum"
    );

    let got = rerun(
        e,
        &dir,
        "pwcorr x z [fw=f], sig obs",
        "stata_pwcorr(",
        "res$sig[2, 1], res$Nobs[2, 1], res$Nobs[1, 1]",
    );
    near(
        &got,
        &[0.35916074228214978, 19.0, 20.0],
        1e-12,
        "pwcorr, fweights",
    );
    let got = rerun(
        e,
        &dir,
        "pwcorr x z [aw=a], sig obs",
        "stata_pwcorr(",
        "res$C[2, 1], res$sig[2, 1], res$Nobs[2, 1]",
    );
    near(
        &got,
        &[0.08579901772359011, 0.826280376868327, 9.0],
        1e-12,
        "pwcorr, aweights",
    );
}

#[test]
fn regress_with_each_kind_of_weight_agrees_with_stata_19() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    let e = &mut e;
    let show = &format!("{SE}, stata_df");
    let fw = [
        0.19728116710875337,
        0.20932661922170717,
        3.5517241379310343,
        1.2957265224528389,
    ];
    let got = after(e, &dir, "regress z x [fw=f]", show);
    near(
        &got,
        &[&fw[..], &[17.0]].concat(),
        1e-12,
        "regress, fweights",
    );
    let got = after(e, &dir, "regress z x [aw=a]", show);
    near(
        &got,
        &[
            0.093663904478437018,
            0.41108959636155801,
            4.4765841097160042,
            3.1214086356863571,
            7.0,
        ],
        1e-12,
        "regress, aweights",
    );
    let pw = [
        0.093663904478437018,
        0.30410167948271544,
        4.4765841097160042,
        2.8307267901900159,
        7.0,
    ];
    let got = after(e, &dir, "regress z x [pw=a]", show);
    near(&got, &pw, 1e-12, "regress, pweights");
    let got = after(e, &dir, "regress z x [aw=a], robust", show);
    near(&got, &pw, 1e-12, "aweights with robust are pweights");
    let got = after(e, &dir, "regress z x [iw=a]", show);
    near(
        &got,
        &[
            0.093663904478437116,
            0.27775841052765571,
            4.4765841097160033,
            2.109023212771862,
            15.0,
        ],
        1e-12,
        "regress, iweights",
    );
    let got = after(e, &dir, "regress z x [fw=f], vce(cluster g)", show);
    near(
        &got,
        &[
            0.19728116710875337,
            0.091904591199941307,
            3.5517241379310343,
            0.10955328991450354,
            1.0,
        ],
        1e-12,
        "fweights, clustered",
    );
    let got = after(e, &dir, "regress z x [pw=a], vce(cluster s)", show);
    near(
        &got,
        &[
            0.093663904478437018,
            0.19427119753280106,
            4.4765841097160042,
            3.5879016908889301,
            1.0,
        ],
        1e-12,
        "pweights, clustered",
    );
    let got = after(
        e,
        &dir,
        "regress z x [fw=zz]",
        "nrow(stata_data), stata_b[2], sum(stata_data$zz)",
    );
    near(
        &got,
        &[8.0, 0.096112311015118801, 16.0],
        1e-12,
        "a zero fweight drops its row",
    );
    // by definition: an fweight fit is least squares on the rows repeated
    let got = after(
        e,
        &dir,
        "regress z x [fw=f]",
        "local({ d <- stata_data[rep(seq_len(nrow(stata_data)), stata_data$f), ]; f <- stats::lm(z ~ x, data = d); c(stats::coef(f)[2], sqrt(stats::vcov(f)[2, 2]), stats::coef(f)[1], sqrt(stats::vcov(f)[1, 1])) })",
    );
    near(&got, &fw, 1e-12, "fweights as copies");
}

#[test]
fn likelihood_models_with_weights_agree_with_stata_19() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    let e = &mut e;
    let cases: [(&str, [f64; 4]); 9] = [
        (
            "logit b x [fw=f]",
            [
                -0.29286501166921369,
                0.18091788396225419,
                1.8000975745715169,
                1.117542845077875,
            ],
        ),
        (
            "logit b x [pw=a]",
            [
                0.097994625145161648,
                0.26104093451557231,
                -0.57584595068027367,
                1.8062127189803843,
            ],
        ),
        (
            "logit b x [iw=a]",
            [
                0.097994625145161648,
                0.19350459639629747,
                -0.57584595068027367,
                1.4319114016787036,
            ],
        ),
        (
            "logit b x [fw=f], vce(robust)",
            [
                -0.29286501166921369,
                0.20231687776589269,
                1.8000975745715169,
                0.96880988859210126,
            ],
        ),
        (
            "logit b x [pw=a], vce(cluster s)",
            [
                0.097994625145161648,
                0.046826687709967924,
                -0.57584595068027367,
                0.31854572421551386,
            ],
        ),
        (
            "poisson z x [fw=f]",
            [
                0.042879430899464104,
                0.037184020267129102,
                1.2905389585768721,
                0.24155973902255021,
            ],
        ),
        (
            "poisson z x [pw=a]",
            [
                0.018457389133135667,
                0.058735681094393671,
                1.5053768346728056,
                0.54947867679547757,
            ],
        ),
        (
            "poisson z x [iw=a]",
            [
                0.018457389133135667,
                0.046377089555635805,
                1.5053768346728056,
                0.35595538394673848,
            ],
        ),
        (
            "probit b x [pw=a]",
            [
                0.061349849462135252,
                0.16150030325155543,
                -0.3574289438084618,
                1.0829185801489305,
            ],
        ),
    ];
    for (cmd, stata) in cases {
        let got = after(e, &dir, cmd, SE);
        // six digits: Stata's iterations stop earlier than R's here (after
        // logit [fw=f] its score is about 6e-7, R's about 1e-15)
        near(&got, &stata, 1e-6, cmd);
    }
    // the header: N is the fweights' sum, and the log likelihood is weighted
    let got = rerun(
        e,
        &dir,
        "logit b x [fw=f]",
        "stata_ml_header(",
        "res[[\"N\"]], res[[\"ll\"]]",
    );
    near(
        &got,
        &[20.0, -12.249802955466865],
        1e-10,
        "logit, fweights: N and ll",
    );
    let got = rerun(
        e,
        &dir,
        "logit b x [pw=a]",
        "stata_ml_header(",
        "res[[\"ll\"]]",
    );
    near(&got, &[-12.550325574457231], 1e-10, "logit, pweights: ll");
    let got = rerun(
        e,
        &dir,
        "poisson z x [fw=f]",
        "stata_ml_header(",
        "res[[\"ll\"]]",
    );
    near(&got, &[-44.695053471319753], 1e-10, "poisson, fweights: ll");
}

#[test]
fn weights_stata_refuses_are_refused_in_its_words() {
    for (cmd, why) in [
        ("summarize z [pw=a]", "pweights not allowed"),
        ("summarize z [iw=a], detail", "iweights not allowed"),
        ("tabulate g [pw=a]", "pweights not allowed"),
        (
            "tabulate g b [aw=a], chi2",
            "chi2 not allowed with aweights",
        ),
        ("tabulate g b [iw=a], chi2", "option chi2 not allowed"),
        ("correlate x z [pw=a]", "pweights not allowed"),
        ("correlate x z [iw=a]", "iweights not allowed"),
        ("logit b x [aw=a]", "aweights not allowed"),
        ("summarize z [weight=a]", "name it as fweight"),
        ("summarize z [fw=f*2]", "an expression"),
        (
            "regress z x [fw=f]\nmargins",
            "margins after a model with weights",
        ),
        (
            "logit b x [pw=a]\nestat ic",
            "estat after a model with weights",
        ),
    ] {
        let why_got = refusal(&format!("use s\n{cmd}\n"));
        assert!(why_got.contains(why), "{cmd}: {why_got}");
    }
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    for (cmd, message) in [
        (
            "summarize x [fw=a]",
            "may not use noninteger frequency weights",
        ),
        ("summarize x [aw=n]", "negative weights encountered"),
        (
            "regress z x [fw=a]",
            "may not use noninteger frequency weights",
        ),
    ] {
        let (outcome, text) = run(&mut e, &script(&format!("{}{cmd}\n", start(&dir))));
        assert_eq!(outcome, Outcome::Error, "{cmd}: {text}");
        assert!(text.contains(message), "{cmd}: {text}");
    }
}
/// A number no other call in this run has had: the tests in a file run at
/// once, and each writing its data to the same folder let one read a file
/// another was still writing.
fn fresh() -> usize {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}
