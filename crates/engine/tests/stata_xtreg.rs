//! P2B.4, panel data: `xtset` and `xtreg` translated to R and run in a real
//! R, on an unbalanced panel with gaps (30 panels of 3 to 8 years) and a
//! regressor that does not vary within a panel.
//!
//! - `fe` against least squares with a dummy for every panel (LSDV), which
//!   must give the same slopes and, with the same degrees of freedom, the
//!   same standard errors;
//! - `be` against least squares on the panel means;
//! - every estimator, its robust and clustered errors, R-squareds, sigma_u,
//!   sigma_e, rho, theta, the F test of u_i, `predict` and `test`, against
//!   Stata 19 SE's own results on this same data (run in batch mode on
//!   2026-10-30, printed with `%24.17g`).
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

/// The panel the Stata run used: made the same way, with the same seed.
fn panel(e: &mut Engine) -> String {
    let dir = std::env::temp_dir().join(format!(
        "rcraft-stata-xtreg-{}-{}",
        std::process::id(),
        fresh()
    ));
    let _ = std::fs::create_dir_all(&dir);
    let d = dir.display().to_string().replace('\\', "/");
    ok(
        e,
        &format!(
            "local({{
  set.seed(20261101)
  n <- 30
  Ti <- sample(3:8, n, TRUE)
  id <- rep(1:n, Ti)
  year <- unlist(lapply(Ti, function(t) sort(sample(1:9, t))))
  u <- stats::rnorm(n, sd = 0.8)[id]
  z <- round(stats::rnorm(n), 2)[id]
  x <- round(stats::rnorm(length(id)) + 0.5 * u, 3)
  w <- round(stats::runif(length(id)), 3)
  y <- round(1 + 0.5 * x - 0.3 * w + 0.4 * z + u + stats::rnorm(length(id)), 3)
  cl <- (id - 1) %/% 5 + 1
  haven::write_dta(data.frame(id = as.numeric(id), year = as.numeric(year), x = x, w = w, z = z,
                              y = y, cl = as.numeric(cl)), '{d}/panel.dta')
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

const B_SE: &str = "unname(stata_b), unname(sqrt(diag(stata_vcov))), stata_df";

#[test]
fn fe_is_least_squares_with_a_dummy_for_every_panel() {
    let Some(mut e) = engine() else { return };
    let d = panel(&mut e);
    let code = script(&format!(
        "use \"{d}/panel.dta\"\nxtset id year\nxtreg y x w z, fe\n"
    ));
    let text = ok(&mut e, &code);
    assert!(
        text.contains("note: z omitted because of collinearity."),
        "{text}"
    );
    assert!(text.contains("Panel variable: id (unbalanced)"), "{text}");
    let got = numbers(
        &mut e,
        "0",
        "unname(stata_b[-1]), unname(sqrt(diag(stata_vcov))[-1]), stata_df",
    );
    let want = numbers(
        &mut e,
        "m <- stats::lm(y ~ x + w + factor(id), data = panel); s <- summary(m)$coefficients",
        "s[\"x\", 1], s[\"w\", 1], s[\"x\", 2], s[\"w\", 2], m$df.residual",
    );
    all_close(&got, &want, 1e-10, "fe against LSDV");
}

#[test]
fn be_is_least_squares_on_the_panel_means() {
    let Some(mut e) = engine() else { return };
    let d = panel(&mut e);
    let code = script(&format!(
        "use \"{d}/panel.dta\"\nxtset id\nxtreg y x w z, be\n"
    ));
    ok(&mut e, &code);
    let got = numbers(&mut e, "0", B_SE);
    let want = numbers(
        &mut e,
        "a <- stats::aggregate(cbind(y, x, w, z) ~ id, data = panel, FUN = mean); \
         m <- stats::lm(y ~ x + w + z, data = a); s <- summary(m)$coefficients",
        "s[, 1], s[, 2], m$df.residual",
    );
    all_close(&got, &want, 1e-10, "be against least squares on the means");
}

#[test]
fn xtreg_agrees_with_stata_19() {
    let Some(mut e) = engine() else { return };
    let d = panel(&mut e);
    let run_xt = |e: &mut Engine, cmd: &str, show: &str| -> Vec<f64> {
        let code = script(&format!("use \"{d}/panel.dta\"\nxtset id year\n{cmd}\n"));
        ok(e, &code);
        numbers(e, "0", show)
    };
    let stats =
        "stata_estimates$r2, stata_estimates$sigma_u, stata_estimates$sigma_e, stata_estimates$rho";

    // fe: (Intercept), x, w (z has no within variation and is dropped)
    let got = run_xt(
        &mut e,
        "xtreg y x w z, fe",
        &format!("{B_SE}, {stats}, stata_estimates$F_u, stata_estimates$corr"),
    );
    near_stata(
        &got,
        &[
            1.459501627874757,
            0.45056565462318759,
            -0.53278865025996935,
            0.1888034463063944,
            0.095903152512251802,
            0.30942252979103074,
            121.0,
            0.17549641455699128,
            0.45040805710320159,
            0.23864076070692458,
            1.0457329705868723,
            1.0073400228854048,
            0.51869369795881726,
            4.4686455468108788,
            0.25509094485957001,
        ],
        1e-10,
        "xtreg, fe",
    );
    let got = run_xt(
        &mut e,
        "xtreg y x w z, fe vce(robust)",
        "unname(sqrt(diag(stata_vcov))), stata_df",
    );
    near_stata(
        &got,
        &[
            0.17475284562560539,
            0.086592932822229088,
            0.32443651823319286,
            29.0,
        ],
        1e-10,
        "xtreg, fe vce(robust): clustered on the panel, t on n - 1",
    );
    let got = run_xt(
        &mut e,
        "xtreg y x w, fe vce(cluster cl)",
        "unname(sqrt(diag(stata_vcov))), stata_df",
    );
    near_stata(
        &got,
        &[
            0.073774236668973855,
            0.079510187767408744,
            0.14385794368679131,
            5.0,
        ],
        1e-10,
        "xtreg, fe vce(cluster cl)",
    );
    let got = run_xt(
        &mut e,
        "xtreg y x w z, be",
        &format!("{B_SE}, stata_estimates$r2"),
    );
    near_stata(
        &got,
        &[
            -0.17999186356296693,
            1.8231151060150532,
            1.7700060691367494,
            0.30409105999664993,
            0.59016004328655958,
            0.27688255661904565,
            1.0099149456967309,
            0.15633101118737708,
            26.0,
            0.11261918460528073,
            0.64655311360372836,
            0.23602607626830793,
        ],
        1e-10,
        "xtreg, be",
    );
    let re = [
        1.2863815959162792,
        0.58301363214764412,
        -0.41385933281770765,
        0.33553763070988663,
        0.2277206904992507,
        0.096275428234275415,
        0.31453467596724072,
        0.16123974340811198,
        0.17247084349697669,
        0.42545271702594761,
        0.30168533192643621,
        0.58714056972051265,
        1.0073400228854048,
        0.25358011820680398,
        0.29626056880382057,
        0.48137273334185826,
    ];
    let show_re = format!(
        "unname(stata_b), unname(sqrt(diag(stata_vcov))), {stats}, \
         min(stata_estimates$theta), max(stata_estimates$theta)"
    );
    // re is the default
    for cmd in ["xtreg y x w z, re", "xtreg y x w z"] {
        let got = run_xt(&mut e, cmd, &show_re);
        near_stata(&got, &re, 1e-10, cmd);
    }
    let got = run_xt(
        &mut e,
        "xtreg y x w z, re",
        "as.numeric(is.finite(stata_df))",
    );
    assert_eq!(got, vec![0.0], "re has z tests");
    let got = run_xt(
        &mut e,
        "xtreg y x w z, re vce(robust)",
        "unname(sqrt(diag(stata_vcov)))",
    );
    near_stata(
        &got,
        &[
            0.2457448623800994,
            0.086224875927647979,
            0.30159458521431687,
            0.19073388701707317,
        ],
        1e-10,
        "xtreg, re vce(robust)",
    );

    // predict and test after fe and re
    let got = run_xt(
        &mut e,
        "xtreg y x w z, fe\npredict double xbd\npredict xbf",
        "panel$xbd[1], panel$xbf[1]",
    );
    near_stata(
        &got,
        &[0.42324053130400729, 0.4232405424118042],
        1e-12,
        "predict after fe, double and float",
    );
    let got = run_xt(
        &mut e,
        "xtreg y x w z, re\npredict double xbd",
        "panel$xbd[1]",
    );
    near_stata(&got, &[0.13428038247564178], 1e-10, "predict after re");
    let code = script(&format!(
        "use \"{d}/panel.dta\"\nxtset id year\nxtreg y x w z, fe\ntest x w\n"
    ));
    ok(&mut e, &code);
    let got = numbers(
        &mut e,
        &format!("res <- {}", line_of(&code, "stata_test(")),
        "res[1], res[2], res[3]",
    );
    near_stata(
        &got,
        &[12.877485638820055, 2.0, 121.0],
        1e-10,
        "test after fe",
    );
    let code = script(&format!(
        "use \"{d}/panel.dta\"\nxtset id year\nxtreg y x w z, re\ntest x w\n"
    ));
    ok(&mut e, &code);
    let got = numbers(
        &mut e,
        &format!("res <- {}", line_of(&code, "stata_test(")),
        "res[1], res[2]",
    );
    near_stata(&got, &[39.509518854908066, 2.0], 1e-10, "test after re");
}

#[test]
fn xtreg_says_what_stata_says() {
    // Stata's own words, checked in Stata 19
    assert!(
        refusal("use \"d.dta\"\nxtreg y x, fe\n").contains("must specify panelvar; use xtset"),
        "xtreg before xtset"
    );
    assert!(refusal("use \"d.dta\"\nxtset id\nxtreg y x, be robust\n").contains("be"));
    assert!(refusal("use \"d.dta\"\nxtset id\nxtreg y x, mle robust\n").contains("mle"));
    assert!(refusal("use \"d.dta\"\nxtset id\nxtreg y x, pa corr(ar1)\n").contains("ar1"));
    let Some(mut e) = engine() else { return };
    let d = panel(&mut e);
    ok(
        &mut e,
        &format!(
            "local({{ p <- haven::read_dta('{d}/panel.dta'); p$year[2] <- p$year[1]; \
             haven::write_dta(p, '{d}/repeat.dta') }})"
        ),
    );
    let (outcome, text) = run(
        &mut e,
        &script(&format!("use \"{d}/repeat.dta\"\nxtset id year\n")),
    );
    assert_eq!(outcome, Outcome::Error, "{text}");
    assert!(text.contains("repeated time values within panel"), "{text}");
}

/// A number no other call in this run has had, so the tests (which run at
/// once) each write their data to a folder of their own.
fn fresh() -> usize {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

/// After xtreg: predict u, e, ue, xbu and stdp, estat ic, margins; and the
/// mle and pa estimators; against Stata 19 on the same panel.
#[test]
fn xtreg_postestimation_mle_and_pa_agree_with_stata_19() {
    let Some(mut e) = engine() else { return };
    let d = panel(&mut e);
    let start = format!("use \"{d}/panel.dta\"\nxtset id year\n");
    let run_xt = |e: &mut Engine, cmd: &str, show: &str| -> Vec<f64> {
        let code = script(&format!("{start}{cmd}\n"));
        ok(e, &code);
        numbers(e, "0", show)
    };
    // predict, rows 1 to 3 (panel 1); Stata stores floats
    let rows = |v: &str| format!("panel${v}[1], panel${v}[2], panel${v}[3]");
    let fe = "xtreg y x w z, fe\npredict u, u\npredict e, e\npredict ue, ue\npredict xbu, xbu\npredict sp, stdp";
    let show = [rows("u"), rows("e"), rows("ue"), rows("xbu"), rows("sp")].join(", ");
    near_stata(
        &run_xt(&mut e, fe, &show),
        &[
            0.59583044052124023,
            0.59583044052124023,
            0.59583044052124023,
            0.75192904472351074,
            -0.7682802677154541,
            0.80148983001708984,
            1.347759485244751,
            -0.17244979739189148,
            1.3973202705383301,
            1.0190709829330444,
            1.5072802305221558,
            2.4015102386474609,
            0.19747310876846313,
            0.15167064964771271,
            0.13619007170200348,
        ],
        1e-7,
        "predict after fe",
    );
    let re = "xtreg y x w z, re\npredict u, u\npredict e, e\npredict ue, ue\npredict sp, stdp";
    let show = [rows("u"), rows("e"), rows("ue"), rows("sp")].join(", ");
    near_stata(
        &run_xt(&mut e, re, &show),
        &[
            0.51528650522232056,
            0.51528650522232056,
            0.51528650522232056,
            1.1214331388473511,
            -0.54146319627761841,
            0.93407469987869263,
            1.6367195844650269,
            -0.026176683604717255,
            1.4493612051010132,
            0.23186948895454407,
            0.19724787771701813,
            0.19038988649845123,
        ],
        1e-7,
        "predict after re (u is the BLUP)",
    );
    // estat ic after fe: the within regression's likelihood
    let code = script(&format!("{start}xtreg y x w z, fe\nestat ic\n"));
    ok(&mut e, &code);
    let v = numbers(
        &mut e,
        &format!("res <- {}", line_of(&code, "stata_xt_ic(")),
        "res[[\"ll(model)\"]], res$df, res$AIC, res$BIC, res[[\"ll(null)\"]]",
    );
    near_stata(
        &v,
        &[
            -200.26599338191409,
            3.0,
            406.53198676382817,
            415.62330052800547,
            -215.02848819182748,
        ],
        1e-12,
        "estat ic after fe",
    );
    let (outcome, text) = run(
        &mut e,
        &script(&format!("{start}xtreg y x w z, re\nestat ic\n")),
    );
    assert_eq!(
        outcome,
        Outcome::Error,
        "Stata 19 has no estat ic after re: {text}"
    );
    // margins: the linear prediction
    let m = |e: &mut Engine, cmd: &str| -> Vec<f64> {
        let code = script(&format!("{start}{cmd}\n"));
        ok(e, &code);
        numbers(
            e,
            &format!("res <- {}", line_of(&code, "stata_margins(")),
            "res$b, sqrt(diag(res$V))",
        )
    };
    near_stata(
        &m(&mut e, "xtreg y x w z, fe\nmargins"),
        &[1.2668104575163399, 0.081438614024875192],
        1e-9,
        "margins after fe",
    );
    near_stata(
        &m(&mut e, "xtreg y x w z, fe\nmargins, dydx(x)"),
        &[0.45056565462318765, 0.095903152512251802],
        1e-9,
        "dydx after fe",
    );
    near_stata(
        &m(&mut e, "xtreg y x w z, re\nmargins"),
        &[1.1960437628654388, 0.14534515654557145],
        1e-9,
        "margins after re",
    );

    // mle: the estimates to 8 digits, the standard errors to 4 (Stata's come
    // from a numerical Hessian, about 1e-5 from the exact one used here)
    let got = run_xt(&mut e, "xtreg y x w z, mle", "unname(stata_b), stata_estimates$ll, stata_estimates$chi2_c, stata_estimates$ll_0, stata_estimates$rho");
    near_stata(
        &got,
        &[
            1.2978784107792978,
            0.54324784019211358,
            -0.44992240105067777,
            0.32048660326610856,
            0.79642985941226974,
            1.0081742754063852,
            -239.27223369659271,
            21.532347554347609,
            -257.1259954867117,
            0.38425783903390176,
        ],
        1e-8,
        "xtreg, mle",
    );
    let got = run_xt(
        &mut e,
        "xtreg y x w z, mle",
        "unname(sqrt(diag(stata_vcov)))",
    );
    near_stata(
        &got,
        &[
            0.23837998526221765,
            0.095863469497235912,
            0.30136711160123991,
            0.1879665170907672,
            0.15085528368967502,
            0.065466868062699177,
        ],
        1e-4,
        "xtreg, mle standard errors",
    );
    // pa: Stata's estimator and its stopping rule, so to 12 digits
    let pa = "unname(stata_b), unname(sqrt(diag(stata_vcov))), stata_estimates$phi, stata_estimates$alpha";
    near_stata(
        &run_xt(&mut e, "xtreg y x w z, pa", pa),
        &[
            1.2889617624885081,
            0.57395883488951316,
            -0.42213394222137834,
            0.3320836392882951,
            0.23093264449312431,
            0.096086454230536064,
            0.31362793837166369,
            0.16732382648876165,
            1.570803203240142,
            0.27965291833918154,
        ],
        1e-9,
        "xtreg, pa",
    );
    near_stata(
        &run_xt(
            &mut e,
            "xtreg y x w z, pa vce(robust)",
            "unname(sqrt(diag(stata_vcov)))",
        ),
        &[
            0.24388996098796273,
            0.084851405577160074,
            0.2992428604250329,
            0.19079410304570604,
        ],
        1e-8,
        "xtreg, pa robust",
    );
    near_stata(
        &run_xt(
            &mut e,
            "xtreg y x w z, pa corr(independent)",
            "unname(stata_b), unname(sqrt(diag(stata_vcov)))",
        ),
        &[
            1.2442744322202479,
            0.72928704501182118,
            -0.2669558773569598,
            0.3912273401943851,
            0.21484817874804016,
            0.10339112864550958,
            0.34329352241324645,
            0.11148696107473481,
        ],
        1e-12,
        "xtreg, pa corr(independent)",
    );
    // a factor level absorbed by the panels, named as Stata names it
    let code = script(&format!("{start}gen gg = mod(id, 2)\nxtreg y x i.gg, fe\n"));
    let text = ok(&mut e, &code);
    assert!(
        text.contains("note: 1.gg omitted because of collinearity."),
        "{text}"
    );
    let v = numbers(
        &mut e,
        "0",
        "unname(stata_b), unname(sqrt(diag(stata_vcov)))",
    );
    near_stata(
        &v,
        &[
            1.1683601775202799,
            0.45743547752428476,
            0.084682971483573816,
            0.096588661449641289,
        ],
        1e-10,
        "fe with i.gg absorbed",
    );
    // xtset alone shows the settings again
    let code = script(&format!("{start}xtset\n"));
    assert!(
        ok(&mut e, &code).contains("Panel variable: id"),
        "xtset alone"
    );
}
