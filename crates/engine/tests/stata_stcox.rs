//! P2B.4, `stset` and `stcox` translated to R and run in a real R, against
//! Stata 19 SE's own results on the same data (batch mode, 2026-10-31,
//! `%24.17g`): Breslow's ties by default (R's own default is Efron's),
//! Efron's and the exact partial likelihood, robust and clustered errors with
//! Stata's factors, factor variables, `test`, `predict`, and `stset`'s
//! `scale()`, `id()` and `enter()`.
//!
//! Without a supported R, or without haven and dplyr, the test is skipped.

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

fn at(v: &[f64], i: usize) -> f64 {
    v.get(i).copied().unwrap_or(f64::NAN)
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

/// Sixty subjects, 42 failures at 16 distinct times (many ties), made with a
/// fixed seed; the same file Stata 19 was run on.
fn survdata(e: &mut Engine) -> String {
    let dir = std::env::temp_dir().join(format!(
        "rcraft-stata-stcox-{}-{}",
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
  n <- 60
  x <- round(stats::rnorm(n), 2)
  w <- round(stats::runif(n), 2)
  g <- rep(1:3, length.out = n)
  cl <- rep(1:10, each = 6)
  tt <- ceiling(stats::rexp(n, exp(-1 + 0.6 * x - 0.4 * w)) * 4)
  cen <- ceiling(stats::runif(n, 2, 30))
  haven::write_dta(data.frame(id = 1:n, time = as.numeric(pmin(tt, cen)), died = as.numeric(tt <= cen),
    x = x, w = w, g = as.numeric(g), cl = as.numeric(cl)), '{d}/surv.dta')
}})"
        ),
    );
    d
}

/// The coefficients and standard errors of x and w, and the log likelihoods.
const XW: &str = "stata_b[c(\"x\", \"w\")], sqrt(diag(stata_vcov))[c(\"x\", \"w\")]";

#[test]
fn stcox_agrees_with_stata_19() {
    let Some(mut e) = engine() else { return };
    let d = survdata(&mut e);
    let start = format!("use \"{d}/surv.dta\"\nstset time, failure(died)\n");
    let fit = |e: &mut Engine, cmd: &str, show: &str| -> Vec<f64> {
        ok(e, &script(&format!("{start}{cmd}\n")));
        numbers(e, "0", show)
    };
    let ll = "stata_estimates$loglik[2], stata_estimates$loglik[1]";
    let cases: Vec<(&str, Vec<f64>)> = vec![
        (
            "stcox x w",
            vec![
                0.44351741558064794,
                -0.60881309743430545,
                0.19069305491832034,
                0.56634156938199287,
                -138.88345222518129,
                -141.51920231333045,
            ],
        ),
        (
            "stcox x w, efron",
            vec![
                0.47907830975925897,
                -0.66179353043896239,
                0.19474741538643384,
                0.57047344311230208,
                -136.27041855303682,
                -139.23403131409907,
            ],
        ),
        (
            "stcox x w, exactp",
            vec![
                0.50202694769229783,
                -0.68475472338296273,
                0.206801142085274,
                0.60564897131166606,
                -105.06254002839883,
                -108.0162060379696,
            ],
        ),
    ];
    for (cmd, stata) in cases {
        near_stata(
            &fit(&mut e, cmd, &format!("{XW}, {ll}")),
            &stata,
            1e-10,
            cmd,
        );
    }
    near_stata(
        &fit(&mut e, "stcox x w, vce(robust)", XW),
        &[
            0.44351741558064794,
            -0.60881309743430545,
            0.1910227615148937,
            0.521836309676739,
        ],
        1e-10,
        "stcox, robust (coxph's sandwich times N / (N - 1))",
    );
    near_stata(
        &fit(&mut e, "stcox x w, vce(cluster cl)", XW),
        &[
            0.44351741558064794,
            -0.60881309743430545,
            0.23313129755502104,
            0.31247459371581715,
        ],
        1e-10,
        "stcox, clustered (times G / (G - 1))",
    );
    // factor variables, nohr, test and predict
    let code = script(&format!(
        "{start}stcox x w i.g, nohr\ntest x w\npredict hr, hr\npredict xb, xb\n"
    ));
    ok(&mut e, &code);
    let v = numbers(
        &mut e,
        "0",
        "stata_b, sqrt(diag(stata_vcov)), surv$hr[1:3], surv$xb[1:3]",
    );
    near_stata(
        &v,
        &[
            0.41099206434694757,
            -0.59120134765836718,
            -0.16437910070568035,
            0.23820708625597806,
            0.19284513153739091,
            0.57862920237615567,
            0.4040636369507361,
            0.37801527206554308,
            0.53543317317962646,
            0.73660928010940552,
            0.73000156879425049,
            -0.62467920780181885,
            -0.30569764971733093,
            -0.31470862030982971,
        ],
        1e-7,
        "stcox x w i.g (predict stores floats)",
    );
    let t = numbers(
        &mut e,
        &format!("res <- {}", line_of(&code, "stata_test(")),
        "res[[\"chi2\"]], res[[\"df\"]]",
    );
    near_stata(
        &t,
        &[4.6980306681581965, 2.0],
        1e-10,
        "test x w after stcox",
    );
    // the header's figures
    let code = script(&format!("{start}stcox x w\n"));
    let text = ok(&mut e, &code);
    assert!(text.contains("No. of failures =         42"), "{text}");
    assert!(
        text.contains("LR chi2(2)      =       5.27"),
        "Stata's 5.271500176298332: {text}"
    );
    // stset's options: scale() changes the time at risk only, id() alone
    // nothing, enter() leaves out the rows that enter at their end
    let v = numbers(
        &mut e,
        &format!(
            "invisible(utils::capture.output(eval(parse(text = {:?}))))",
            script(&format!(
                "use \"{d}/surv.dta\"\nstset time, failure(died == 1) scale(4)\nstcox x w\n"
            ))
        ),
        "stata_b, sum(surv$`_t`)",
    );
    near_stata(
        &v,
        &[0.44351741558064794, -0.60881309743430545, 121.25],
        1e-10,
        "stset, scale(4)",
    );
    near_stata(
        &fit(
            &mut e,
            "stset time, failure(died) id(id)\nstcox x w",
            "stata_b",
        ),
        &[0.44351741558064794, -0.60881309743430545],
        1e-10,
        "stset, id()",
    );
    let v = fit(
        &mut e,
        "gen t0 = cond(mod(id, 4) == 0, 1, 0)\nstset time, failure(died) enter(t0)\nstcox x w",
        &format!("{XW}, {ll}, nrow(stata_data), sum(stata_data$`_d`)"),
    );
    near_stata(
        &v,
        &[
            0.35835367422873587,
            -0.63019816527983918,
            0.20772391745237301,
            0.59104579746527797,
            -127.11013882728246,
            -128.66080448176061,
            57.0,
            39.0,
        ],
        1e-10,
        "stset, enter()",
    );
}

/// strata(), and stset's exit() and origin(), against Stata 19.
#[test]
fn stcox_strata_exit_and_origin_agree_with_stata_19() {
    let Some(mut e) = engine() else { return };
    let d = survdata(&mut e);
    let fit = |e: &mut Engine, cmd: &str, show: &str| -> Vec<f64> {
        ok(e, &script(&format!("use \"{d}/surv.dta\"\n{cmd}\n")));
        numbers(e, "0", show)
    };
    near_stata(
        &fit(
            &mut e,
            "stset time, failure(died)\nstcox x w, strata(g)",
            &format!("{XW}, stata_estimates$loglik[2]"),
        ),
        &[
            0.38547570001870257,
            -0.60394861627941077,
            0.19212492448241264,
            0.59745657094742144,
            -95.330246985158098,
        ],
        1e-10,
        "stcox, strata(g)",
    );
    near_stata(
        &fit(&mut e, "stset time, failure(died) exit(time 20)\nstcox x w", "stata_b[\"x\"], sqrt(stata_vcov[\"x\", \"x\"]), sum(stata_data$`_d`), stata_estimates$loglik[2]"),
        &[0.44248883275836221, 0.19052047268848143, 41.0, -137.74699285124672],
        1e-10,
        "stset, exit(time 20)",
    );
    near_stata(
        &fit(&mut e, "gen o = cond(mod(id, 5) == 0, 2, 0)\nstset time, failure(died) origin(o)\nstcox x w", "stata_b[\"x\"], sqrt(stata_vcov[\"x\", \"x\"]), nrow(stata_data), stata_estimates$loglik[2]"),
        &[0.46805433151008818, 0.19040113976411244, 59.0, -134.04366066889975],
        1e-10,
        "stset, origin(o)",
    );
}

/// estat phtest (the Grambsch-Therneau test, as Stata's), margins after stcox
/// (the relative hazard), and stset with expressions, against Stata 19.
#[test]
fn phtest_margins_and_stset_expressions_agree_with_stata_19() {
    let Some(mut e) = engine() else { return };
    let d = survdata(&mut e);
    let start = format!("use \"{d}/surv.dta\"\nstset time, failure(died)\n");
    let code = script(&format!("{start}stcox x w\nestat phtest, detail\n"));
    ok(&mut e, &code);
    let v = numbers(
        &mut e,
        &format!("res <- {}", line_of(&code, "stata_phtest(")),
        "res$chi2, res$df, res$table$rho, res$table$chi2",
    );
    near_stata(
        &v,
        &[
            0.1505795097985401,
            2.0,
            -0.034496164679680412,
            -0.031618788517091719,
            0.062646177723168966,
            0.043179946173377531,
        ],
        // each covariate's rho and chi2 to 6 digits: Stata works them out from
        // the scaled residuals kept as float variables; the global test, from
        // the matrices, is checked to 9 just below
        1e-6,
        "estat phtest, detail",
    );
    near_stata(
        &[at(&v, 0)],
        &[0.1505795097985401],
        1e-9,
        "estat phtest's global chi2",
    );
    let m = |e: &mut Engine, cmd: &str| -> Vec<f64> {
        let code = script(&format!("{start}stcox x w\n{cmd}\n"));
        ok(e, &code);
        numbers(
            e,
            &format!("res <- {}", line_of(&code, "stata_margins(")),
            "res$b, sqrt(diag(res$V))",
        )
    };
    near_stata(
        &m(&mut e, "margins"),
        &[0.84903142144710519, 0.20505834461113223],
        1e-9,
        "margins after stcox",
    );
    near_stata(
        &m(&mut e, "margins, dydx(x)"),
        &[0.37656022196021333, 0.19072686917382567],
        // 7 digits: both sides differentiate the margin in b numerically
        1e-7,
        "dydx after stcox",
    );
    let fit = |e: &mut Engine, cmd: &str, show: &str| -> Vec<f64> {
        ok(e, &script(&format!("use \"{d}/surv.dta\"\n{cmd}\n")));
        numbers(e, "0", show)
    };
    near_stata(
        &fit(
            &mut e,
            "stset time, failure(died == 1) exit(time 4 * 5)\nstcox x w",
            "stata_b[\"x\"], sum(stata_data$`_d`)",
        ),
        &[0.44248883275836221, 41.0],
        1e-10,
        "stset, exit(time 4 * 5)",
    );
    near_stata(
        &fit(
            &mut e,
            "gen e0 = 2\nstset time, failure(died) enter(time e0 - 1)\nstcox x w",
            "stata_b[\"x\"], nrow(stata_data)",
        ),
        &[0.30034823395909921, 55.0],
        1e-10,
        "stset, enter(time e0 - 1)",
    );
}
#[test]
fn stcox_says_what_it_does_not_translate() {
    assert!(refusal("use s\nstcox x\n").contains("data not st; use stset"));
    assert!(refusal("use s\nstset t, failure(d)\nstcox x, exactm\n").contains("exactm"));
}

/// A number no other call in this run has had, so the tests (which run at
/// once) each write their data to a folder of their own.
fn fresh() -> usize {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

/// P2B.6: `e()` after stcox, as Stata 19's `ereturn list` shows it.
#[test]
fn e_after_stcox_is_statas() {
    let Some(mut e) = engine() else { return };
    let d = survdata(&mut e);
    let start = format!("use \"{d}/surv.dta\"\nstset time, failure(died)\n");
    let names = [
        "N", "ll_0", "ll", "chi2", "r2_p", "rank", "df_m", "N_sub", "N_fail", "risk",
    ];
    let show: Vec<String> = names
        .iter()
        .map(|n| format!("stata_e[[\"{n}\"]]"))
        .collect();
    ok(&mut e, &script(&format!("{start}stcox x w\n")));
    near_stata(
        &numbers(&mut e, "0", &show.join(", ")),
        &[
            60.0,
            -141.5192023133305,
            -138.8834522251813,
            5.271500176298332,
            0.018624681633758,
            2.0,
            2.0,
            60.0,
            42.0,
            485.0,
        ],
        1e-9,
        "stcox x w",
    );
    let v = numbers(
        &mut e,
        "0",
        "as.numeric(stata_e$cmd == \"cox\"), as.numeric(stata_e$cmd2 == \"stcox\"), \
         as.numeric(stata_e$ties == \"breslow\"), as.numeric(stata_e$depvar == \"_t\"), \
         as.numeric(is.null(stata_e[[\"p\"]]))",
    );
    assert_eq!(v, vec![1.0; 5], "stcox's macros, and no e(p), as in Stata");
    ok(&mut e, &script(&format!("{start}stcox x w, vce(robust)\n")));
    near_stata(
        &numbers(&mut e, "0", "stata_e$chi2, stata_e$ll"),
        &[5.531251498470934, -138.8834522251813],
        1e-9,
        "stcox x w, vce(robust): a Wald chi2",
    );
}

/// `matrix list` of e(b) and e(V) after `stcox`, as Stata 19 printed them:
/// no constant, no equation.
#[test]
fn matrix_list_after_stcox_is_statas() {
    let Some(mut e) = engine() else { return };
    let d = survdata(&mut e);
    let text = ok(
        &mut e,
        &script(&format!(
            "use \"{d}/surv.dta\"\nquietly stset time, failure(died)\nquietly stcox x w\nmatrix list e(b)\nmatrix list e(V)\n"
        )),
    );
    let stata = "\ne(b)[1,2]\n            x          w\ny1  .44351742  -.6088131\n\nsymmetric e(V)[2,2]\n            x           w\nx   .03636384\nw  -.03239651   .32074277\n";
    assert!(
        text.ends_with(stata),
        "Stata printed\n{stata}\nRCraft printed\n{text}"
    );
}
