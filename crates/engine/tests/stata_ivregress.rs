//! P2B.4, `ivregress` translated to R and run in a real R, on data with two
//! endogenous variables, three instruments and twelve clusters.
//!
//! - 2SLS against two literal stages of least squares (the same
//!   coefficients; the second stage's own standard errors are wrong, and are
//!   shown to be);
//! - 2SLS, LIML and GMM with and without `small`, robust and clustered,
//!   `estat endogenous`, `estat overid`, `predict` and `test`, against Stata
//!   19 SE's own results on this same data (run in batch mode on 2026-10-30,
//!   printed with `%24.17g`). Notably, without `small` Stata's clustered
//!   errors carry no G / (G - 1).
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

/// The data the Stata run used: made the same way, with the same seed.
fn ivdata(e: &mut Engine) -> String {
    let dir = std::env::temp_dir().join(format!(
        "rcraft-stata-iv-{}-{}",
        std::process::id(),
        fresh()
    ));
    let _ = std::fs::create_dir_all(&dir);
    let d = dir.display().to_string().replace('\\', "/");
    ok(
        e,
        &format!(
            "local({{
  set.seed(20261102)
  n <- 120
  z1 <- round(stats::rnorm(n), 3)
  z2 <- round(stats::rnorm(n), 3)
  z3 <- round(stats::runif(n), 3)
  w <- round(stats::rnorm(n), 3)
  v <- stats::rnorm(n)
  x <- round(0.6 * z1 + 0.4 * z2 - 0.3 * z3 + 0.5 * w + v, 3)
  x2 <- round(0.5 * z2 + 0.8 * z3 + 0.5 * v + stats::rnorm(n), 3)
  y <- round(1 + 0.8 * x - 0.5 * w + 0.3 * x2 + 0.6 * v + stats::rnorm(n), 3)
  g <- rep(1:12, each = 10)
  haven::write_dta(data.frame(y = y, x = x, x2 = x2, w = w, z1 = z1, z2 = z2, z3 = z3,
                              g = as.numeric(g)), '{d}/iv.dta')
}})"
        ),
    );
    d
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

#[test]
fn two_sls_is_two_stages_of_least_squares() {
    let Some(mut e) = engine() else { return };
    let d = ivdata(&mut e);
    let code = script(&format!(
        "use \"{d}/iv.dta\"\nivregress 2sls y w (x = z1 z2 z3)\n"
    ));
    let text = ok(&mut e, &code);
    assert!(text.contains("Endogenous: x"), "{text}");
    let got = numbers(
        &mut e,
        "0",
        "unname(stata_b), unname(sqrt(diag(stata_vcov)))",
    );
    let want = numbers(
        &mut e,
        "iv$xhat <- stats::fitted(stats::lm(x ~ w + z1 + z2 + z3, data = iv)); \
         m <- stats::lm(y ~ xhat + w, data = iv)",
        "unname(stats::coef(m)), unname(sqrt(diag(stats::vcov(m))))",
    );
    for i in 0..3 {
        close(
            at(&got, i),
            at(&want, i),
            1e-10,
            "the coefficients of the second stage",
        );
    }
    // the second stage's own standard errors use the wrong residuals
    assert!(
        (at(&got, 4) - at(&want, 4)).abs() > 1e-4,
        "2SLS's errors are not the second stage's"
    );
}

#[test]
fn ivregress_agrees_with_stata_19() {
    let Some(mut e) = engine() else { return };
    let d = ivdata(&mut e);
    let fit = |e: &mut Engine, cmd: &str, show: &str| -> Vec<f64> {
        let code = script(&format!("use \"{d}/iv.dta\"\n{cmd}\n"));
        ok(e, &code);
        numbers(e, "0", show)
    };
    let b_se = "unname(stata_b), unname(sqrt(diag(stata_vcov)))";
    let one = "y w (x = z1 z2 z3)";

    let got = fit(
        &mut e,
        &format!("ivregress 2sls {one}"),
        &format!(
            "{b_se}, stata_estimates$rss, stata_estimates$rmse, stata_estimates$r2, \
             stata_estimates$r2_a, stata_estimates$Wald"
        ),
    );
    near_stata(
        &got,
        &[
            1.0347724534741489,
            0.92916162505063837,
            -0.53510677784680194,
            0.12086243203594065,
            0.1447307400250398,
            0.13931506771882979,
            165.82982376010941,
            1.1755488922205284,
            0.66067979713942815,
            0.65487945179138429,
            41.253642061545293,
        ],
        1e-10,
        "2sls",
    );
    // (command, standard errors, df (0 for z tests), Wald chi2 or F)
    let cases: [(&str, [f64; 3], f64, f64); 6] = [
        (
            "2sls {} , small",
            [0.12240214292427121, 0.1465745180505095, 0.14108985350678582],
            117.0,
            20.11115050500333,
        ),
        (
            "2sls {} , vce(robust)",
            [
                0.12304938787623369,
                0.14858392590059388,
                0.14463572217935614,
            ],
            0.0,
            39.483918227735664,
        ),
        (
            "2sls {} , vce(robust) small",
            [0.1246169591978095, 0.15047679107537387, 0.14647828973761018],
            117.0,
            19.248410136021135,
        ),
        // no G / (G - 1) without small, unlike regress
        (
            "2sls {} , vce(cluster g)",
            [0.12170882273546021, 0.10000730867148581, 0.1088008047239548],
            0.0,
            103.79990045020028,
        ),
        (
            "2sls {} , vce(cluster g) small",
            [
                0.12820261739676159,
                0.10534321540812042,
                0.11460588991812659,
            ],
            11.0,
            46.775375307915851,
        ),
        (
            "gmm {} , small",
            [
                0.12353398042243073,
                0.14873245054610604,
                0.14585844144384202,
            ],
            117.0,
            19.947311244514488,
        ),
    ];
    for (form, se, df, stat) in cases {
        let cmd = format!("ivregress {}", form.replace("{}", one));
        let got = fit(
            &mut e,
            &cmd,
            "unname(sqrt(diag(stata_vcov))), ifelse(is.finite(stata_df), stata_df, 0), \
             ifelse(is.finite(stata_df), stata_estimates$Wald / 2, stata_estimates$Wald)",
        );
        let mut want = se.to_vec();
        want.push(df);
        want.push(stat);
        near_stata(&got, &want, 1e-10, &cmd);
    }
    let got = fit(
        &mut e,
        &format!("ivregress liml {one}"),
        &format!("{b_se}, stata_estimates$kappa, stata_estimates$rss"),
    );
    near_stata(
        &got,
        &[
            1.0328149534592703,
            0.92397029839702527,
            -0.53199811733900393,
            0.12127899342572146,
            0.14567936510334559,
            0.13989493561195127,
            1.0040343458114578,
            166.70958935512948,
        ],
        1e-10,
        "liml",
    );
    let got = fit(
        &mut e,
        &format!("ivregress liml {one}, small"),
        "unname(sqrt(diag(stata_vcov)))",
    );
    near_stata(
        &got,
        &[0.12282401104250933, 0.1475352280119126, 0.14167710854985785],
        1e-10,
        "liml, small",
    );
    let got = fit(
        &mut e,
        &format!("ivregress gmm {one}"),
        &format!("{b_se}, stata_estimates$Wald"),
    );
    near_stata(
        &got,
        &[
            1.042865962637854,
            0.93433305400869993,
            -0.53579601432636303,
            0.1219800320176801,
            0.1468615276350945,
            0.14402367102985544,
            40.917561527209187,
        ],
        1e-10,
        "gmm",
    );
    let got = fit(&mut e, "ivregress 2sls y w (x x2 = z1 z2 z3)", b_se);
    near_stata(
        &got,
        &[
            0.96091875115435554,
            0.89763518060279024,
            0.13646816385002949,
            -0.5011210557423047,
            0.1653568857579934,
            0.14836692329524534,
            0.21657881128638423,
            0.14486478204342268,
        ],
        1e-10,
        "two endogenous variables",
    );

    // estat endogenous and overid, with one and two endogenous variables
    for (cmd, durbin, overid) in [
        (
            format!("ivregress 2sls {one}"),
            [17.851032693466074, 20.271568543890812, 1.0, 116.0],
            [0.48344771310054657, 0.46517813593805402, 2.0],
        ),
        (
            "ivregress 2sls y w (x x2 = z1 z2 z3)".to_string(),
            [15.64416361057596, 8.5449684143703628, 2.0, 114.0],
            [0.1220344720304567, 0.1170687558943279, 1.0],
        ),
    ] {
        let code = script(&format!(
            "use \"{d}/iv.dta\"\n{cmd}\nestat endogenous\nestat overid\n"
        ));
        ok(&mut e, &code);
        let got = numbers(
            &mut e,
            &format!("res <- {}", line_of(&code, "stata_iv_endogenous(")),
            "res",
        );
        near_stata(&got, &durbin, 1e-10, "estat endogenous");
        let got = numbers(
            &mut e,
            &format!("res <- {}", line_of(&code, "stata_iv_overid(")),
            "res",
        );
        near_stata(&got, &overid, 1e-10, "estat overid");
    }

    // predict and test
    let got = fit(
        &mut e,
        &format!("ivregress 2sls {one}\npredict double xb"),
        "iv$xb[1], iv$xb[2]",
    );
    near_stata(
        &got,
        &[1.9615697494415643, 0.69789149703940723],
        1e-10,
        "predict",
    );
    for (cmd, want) in [
        (
            format!("ivregress 2sls {one}"),
            vec![41.253642061545293, 2.0],
        ),
        (
            format!("ivregress 2sls {one}, small"),
            vec![20.11115050500333, 2.0, 117.0],
        ),
    ] {
        let code = script(&format!("use \"{d}/iv.dta\"\n{cmd}\ntest x w\n"));
        ok(&mut e, &code);
        let show = if want.len() == 3 {
            "res[1], res[2], res[3]"
        } else {
            "res[1], res[2]"
        };
        let got = numbers(
            &mut e,
            &format!("res <- {}", line_of(&code, "stata_test(")),
            show,
        );
        near_stata(&got, &want, 1e-10, &cmd);
    }
}

#[test]
fn ivregress_says_what_stata_says() {
    let Some(mut e) = engine() else { return };
    let d = ivdata(&mut e);
    let (outcome, text) = run(
        &mut e,
        &script(&format!(
            "use \"{d}/iv.dta\"\nivregress 2sls y w (x x2 = z1)\n"
        )),
    );
    assert_eq!(outcome, Outcome::Error, "{text}");
    assert!(
        text.contains("equation not identified; must have at least as many instruments not in the regression as there are instrumented variables"),
        "{text}"
    );
    assert!(refusal("use \"d.dta\"\nivregress gmm y (x = z1 z2), vce(hac nw 2)\n").contains("hac"));
    assert!(
        refusal("use \"d.dta\"\nivregress liml y (x = z1 z2), wmatrix(robust)\n").contains("gmm")
    );
    assert!(refusal("use \"d.dta\"\nivregress 3sls y (x = z)\n").contains("2sls, liml and gmm"));
    assert!(
        refusal("use \"d.dta\"\nivregress 2sls y (x = z1) (w = z2)\n").contains("invalid syntax"),
        "one group of instrumented variables, in Stata's words"
    );
    assert!(
        refusal("use \"d.dta\"\nivregress 2sls y (x = z)\nestat ic\n").contains("r(321)"),
        "Stata 19 has no estat ic after ivregress"
    );
}

/// A number no other call in this run has had, so the tests (which run at
/// once) each write their data to a folder of their own.
fn fresh() -> usize {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

/// The rest of ivregress against Stata 19 on the same data: liml with robust
/// and clustered errors, gmm's vce() and wmatrix() and igmm with Hansen's J,
/// estat firststage, the robust and liml forms of the estat tests, margins,
/// noconstant and a factor variable among the instruments.
#[test]
fn the_rest_of_ivregress_agrees_with_stata_19() {
    let Some(mut e) = engine() else { return };
    let d = ivdata(&mut e);
    let start = format!("use \"{d}/iv.dta\"\n");
    let fit = |e: &mut Engine, cmd: &str, show: &str| -> Vec<f64> {
        ok(e, &script(&format!("{start}{cmd}\n")));
        numbers(e, "0", show)
    };
    // b then se, in Stata's order (x, w, _cons)
    let bse = "stata_b[c(\"x\", \"w\", \"(Intercept)\")], sqrt(diag(stata_vcov))[c(\"x\", \"w\", \"(Intercept)\")]";
    let iv = "y w (x = z1 z2 z3)";
    let cases: Vec<(String, [f64; 6])> = vec![
        (
            format!("ivregress liml {iv}, vce(robust)"),
            [
                0.92397029839702527,
                -0.53199811733900393,
                1.0328149534592703,
                0.15008601770880647,
                0.14537530258806741,
                0.12362935279672098,
            ],
        ),
        (
            format!("ivregress liml {iv}, vce(cluster g) small"),
            [
                0.92397029839702527,
                -0.53199811733900393,
                1.0328149534592703,
                0.10631913213536756,
                0.1145142329334701,
                0.12873680391653944,
            ],
        ),
        (
            format!("ivregress gmm {iv}, vce(unadjusted)"),
            [
                0.93433305400869993,
                -0.53579601432636303,
                1.042865962637854,
                0.14729291917426651,
                0.14438573165985885,
                0.12237466900452121,
            ],
        ),
        (
            format!("ivregress gmm {iv}, wmatrix(unadjusted)"),
            [
                0.92916162505063937,
                -0.53510677784680316,
                1.0347724534741487,
                0.14473074002503988,
                0.13931506771882995,
                0.12086243203594052,
            ],
        ),
        (
            format!("ivregress gmm {iv}, wmatrix(cluster g)"),
            [
                0.8981917803237196,
                -0.52343492688600279,
                1.010035939305171,
                0.084991844301869166,
                0.10651937607532559,
                0.11371927539776831,
            ],
        ),
        (
            format!("ivregress gmm {iv}, igmm"),
            [
                0.93446409846537448,
                -0.53576497934445178,
                1.0429216486432991,
                0.14685388892206713,
                0.14401841615322336,
                0.12197079561713048,
            ],
        ),
        (
            "gen zc = mod(_n, 3)\nivregress 2sls y w (x = i.zc z1)".to_string(),
            [
                0.9244244393608726,
                -0.5322700651807567,
                1.0329861969588254,
                0.18005950214616667,
                0.15356039614814684,
                0.12765369048936756,
            ],
        ),
    ];
    for (cmd, stata) in cases {
        near_stata(&fit(&mut e, &cmd, bse), &stata, 1e-9, &cmd);
    }
    // Hansen's J, kept with the fit
    for (cmd, j) in [
        (
            format!("ivregress gmm {iv}, vce(unadjusted)"),
            0.47797647798134735,
        ),
        (
            format!("ivregress gmm {iv}, wmatrix(unadjusted)"),
            0.48344771310053092,
        ),
        (
            format!("ivregress gmm {iv}, wmatrix(cluster g)"),
            0.52783713662730525,
        ),
        (format!("ivregress gmm {iv}, igmm"), 0.47870051609285447),
    ] {
        near_stata(
            &fit(&mut e, &cmd, "stata_estimates$J"),
            &[j],
            1e-9,
            &format!("J after {cmd}"),
        );
    }
    // noconstant: no constant in X or Z, and no R-squared
    let got = fit(
        &mut e,
        &format!("ivregress 2sls {iv}, noconstant"),
        "stata_b[c(\"x\", \"w\")], sqrt(diag(stata_vcov))[c(\"x\", \"w\")], stata_estimates$rss, is.na(stata_estimates$r2)",
    );
    near_stata(
        &got,
        &[
            0.33634927638682505,
            -0.099069769083976189,
            0.2037811456025326,
            0.20571281122879664,
            411.59372511106994,
            1.0,
        ],
        1e-10,
        "noconstant",
    );
    // the estat tests
    let estat = |e: &mut Engine, cmd: &str, begin: &str, show: &str| -> Vec<f64> {
        let code = script(&format!("{start}{cmd}\n"));
        ok(e, &code);
        numbers(e, &format!("res <- {}", line_of(&code, begin)), show)
    };
    let fs = "res$singleresults[1, c(\"r2\", \"r2_a\", \"partial_r2\", \"F\")], res$mineig";
    near_stata(
        &estat(
            &mut e,
            &format!("ivregress 2sls {iv}\nestat firststage"),
            "stata_iv_firststage(",
            fs,
        ),
        &[
            0.45939764533573474,
            0.44059408517349952,
            0.34250195710096465,
            19.968487861927247,
            19.968487861927233,
        ],
        1e-10,
        "estat firststage",
    );
    near_stata(
        &estat(
            &mut e,
            &format!("ivregress 2sls {iv}, vce(robust)\nestat firststage"),
            "stata_iv_firststage(",
            "res$singleresults[1, \"F\"]",
        ),
        &[19.507627293198112],
        1e-10,
        "estat firststage, robust",
    );
    near_stata(
        &estat(
            &mut e,
            &format!("ivregress 2sls {iv}, vce(robust)\nestat endogenous"),
            "stata_iv_endogenous(",
            "res[[\"r_score\"]]",
        ),
        &[13.46496571816544],
        1e-10,
        "the robust score test of endogeneity",
    );
    near_stata(
        &estat(
            &mut e,
            &format!("ivregress 2sls {iv}, vce(robust)\nestat overid"),
            "stata_iv_overid(",
            "res[[\"score\"]]",
        ),
        &[0.4779764779813434],
        1e-10,
        "the robust score test of overidentification",
    );
    near_stata(
        &estat(
            &mut e,
            &format!("ivregress liml {iv}\nestat overid"),
            "stata_iv_overid(",
            "res[[\"ar\"]], res[[\"basmann\"]]",
        ),
        &[0.4841214973749342, 0.2319748841588226],
        1e-9,
        "Anderson-Rubin and Basmann after liml",
    );
    near_stata(
        &estat(
            &mut e,
            &format!("ivregress gmm {iv}, igmm\nestat overid"),
            "stata_iv_overid(",
            "res[[\"HansenJ\"]]",
        ),
        &[0.47870051609285447],
        1e-9,
        "Hansen's J after gmm",
    );
    let (outcome, text) = run(
        &mut e,
        &script(&format!("{start}ivregress liml {iv}\nestat endogenous\n")),
    );
    assert_eq!(
        outcome,
        Outcome::Error,
        "Stata 19 has none after liml: {text}"
    );
    // margins: the linear prediction
    let code = script(&format!("{start}ivregress 2sls {iv}\nmargins\n"));
    ok(&mut e, &code);
    let v = numbers(
        &mut e,
        &format!("res <- {}", line_of(&code, "stata_margins(")),
        "res$b, sqrt(diag(res$V))",
    );
    near_stata(
        &v,
        &[0.68649166666666872, 0.10731244095323232],
        1e-9,
        "margins after ivregress",
    );
    let code = script(&format!("{start}ivregress 2sls {iv}\nmargins, dydx(x)\n"));
    ok(&mut e, &code);
    let v = numbers(
        &mut e,
        &format!("res <- {}", line_of(&code, "stata_margins(")),
        "res$b, sqrt(diag(res$V))",
    );
    near_stata(
        &v,
        &[0.92916162505063837, 0.1447307400250398],
        1e-9,
        "dydx after ivregress",
    );
}

/// P2B.6: `e()` after ivregress, as Stata 19's `ereturn list` shows it.
#[test]
fn e_after_ivregress_is_statas() {
    let Some(mut e) = engine() else { return };
    let d = ivdata(&mut e);
    let e_of = |e: &mut Engine, cmd: &str, names: &[&str]| -> Vec<f64> {
        let code = script(&format!("use \"{d}/iv.dta\"\n{cmd}\n"));
        ok(e, &code);
        let show: Vec<String> = names
            .iter()
            .map(|n| format!("stata_e[[\"{n}\"]]"))
            .collect();
        numbers(e, "0", &show.join(", "))
    };
    let common = [
        "N", "kappa", "mss", "rss", "rmse", "rank", "df_m", "r2", "r2_a", "k_endog",
    ];
    let mut names = common.to_vec();
    names.push("chi2");
    near_stata(
        &e_of(&mut e, "ivregress 2sls y w (x = z1 z2 z3)", &names),
        &[
            120.0,
            1.0,
            322.8820842315572,
            165.8298237601094,
            1.175548892220528,
            3.0,
            2.0,
            0.6606797971394281,
            0.6548794517913843,
            1.0,
            41.25364206154529,
        ],
        1e-9,
        "ivregress 2sls",
    );
    near_stata(
        &e_of(
            &mut e,
            "ivregress 2sls y w (x = z1 z2 z3), small",
            &["rmse", "df_r", "F"],
        ),
        &[1.190524641083323, 117.0, 20.11115050500333],
        1e-9,
        "ivregress 2sls, small",
    );
    near_stata(
        &e_of(
            &mut e,
            "ivregress 2sls y w (x = z1 z2 z3), vce(robust)",
            &["chi2", "r2"],
        ),
        &[39.48391822773566, 0.6606797971394281],
        1e-9,
        "ivregress 2sls, vce(robust)",
    );
    near_stata(
        &e_of(&mut e, "ivregress liml y w (x = z1 z2 z3)", &names),
        &[
            120.0,
            1.004034345811458,
            322.0023186365372,
            166.7095893551295,
            1.178663046828657,
            3.0,
            2.0,
            0.6588796249303339,
            0.6530485074077755,
            1.0,
            40.26511648149859,
        ],
        1e-8,
        "ivregress liml",
    );
    near_stata(
        &e_of(
            &mut e,
            "ivregress gmm y w (x = z1 z2 z3)",
            &["J", "rss", "rmse", "mss", "r2", "r2_a", "chi2"],
        ),
        &[
            0.4779764779813473,
            164.9693082931225,
            1.172494876652355,
            323.7425996985442,
            0.6624405798273786,
            0.6566703333287014,
            40.91756152720919,
        ],
        1e-8,
        "ivregress gmm",
    );
    let v = numbers(
        &mut e,
        "0",
        "as.numeric(stata_e$exog == \"w z1 z2 z3\"), as.numeric(stata_e$exogr == \"w\"), \
         as.numeric(stata_e$endog == \"x\"), as.numeric(stata_e$vce == \"robust\"), \
         as.numeric(stata_e$title == \"Instrumental-variables GMM regression\")",
    );
    assert_eq!(v, vec![1.0; 5], "ivregress's macros");
}
