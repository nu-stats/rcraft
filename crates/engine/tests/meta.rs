//! Statistics > Meta-analysis (P6.11) end to end against a real R (skipped
//! with a note when none is found, or when metafor is not installed). The
//! dialog's code is built from its spec and run on made-up data (14 studies,
//! each with two groups' sizes, means and standard deviations, a 2×2 table
//! with one empty cell in study 3, and an estimate with its standard error;
//! a copy with study 5's estimate missing), and its numbers are checked
//! against Stata 19's own results for the same data, recorded below:
//!
//! - `meta esize n1 m1 sd1 n2 m2 sd2, esize(hedgesg, exact)`,
//!   `meta esize a b c d, esize(lnoratio)` and `esize(lnrratio)`, and
//!   `meta set est se`, each followed by `meta summarize` with `random(reml)`,
//!   `random(dlaird)`, `fixed(ivariance)` and, for the tables,
//!   `fixed(mhaenszel)`: the pooled estimate, its standard error and 95%
//!   interval, tau², I², H² and Cochran's Q;
//! - `meta bias, egger` (after REML, after DerSimonian-Laird, and after
//!   Mantel-Haenszel, where Stata uses the inverse-variance model) and
//!   `meta bias, egger traditional`: the slope on the standard error, its
//!   standard error, the test statistic and its p-value.
//!
//! DerSimonian-Laird, the fixed effects and the tests on them agree to 1e-9.
//! REML is iterated: Stata and metafor each stop within about 3e-6 of the
//! maximum, so REML figures are held to 1e-5.
//!
//! The test records the R and metafor versions it ran with (AGENTS.md
//! section 5), and checks R is the one `ENVIRONMENT` pins.

use std::time::{Duration, Instant};

use rcraft_catalog::meta::META;
use rcraft_engine::{Engine, Item, Outcome, Source};
use rcraft_rcodegen::dialog::{ColumnInfo, Context, Widget};
use rcraft_rcodegen::dialog::{Style, VarKind};
use rcraft_rhost::{find_installs, Env, SessionConfig};

fn engine() -> Option<Engine> {
    let install = find_installs(&Env::from_process())
        .into_iter()
        .find(|i| i.is_supported());
    let Some(install) = install else {
        eprintln!("skipped: no supported R on this machine");
        return None;
    };
    Engine::start(SessionConfig::new(install)).ok()
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

/// Runs `code`: its outcome, its text and how many plots it drew.
fn run(e: &mut Engine, source: Source, code: &str) -> (Outcome, String, usize) {
    let id = e.submit(source, code);
    assert!(id.is_ok(), "{id:?}");
    let id = id.unwrap_or_default();
    settle(e);
    let outcome = e
        .history()
        .entries()
        .last()
        .and_then(|h| h.outcome)
        .unwrap_or(Outcome::Error);
    let block = e.results().blocks().iter().find(|b| b.command == id);
    let plots = block.map_or(0, |b| {
        b.items
            .iter()
            .filter(|i| matches!(i, Item::Plot { .. }))
            .count()
    });
    let text = block
        .map(|b| {
            b.items
                .iter()
                .map(|i| match i {
                    Item::Text { text, .. } => text.clone(),
                    Item::Warning(w) => format!("warning: {w}\n"),
                    Item::Error { message, .. } => format!("error: {message}\n"),
                    Item::Notice(n) => n.clone(),
                    Item::Plot { .. } => String::new(),
                })
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default();
    (outcome, text, plots)
}

/// The R and metafor versions, recorded; false when metafor is missing.
fn versions(e: &mut Engine) -> bool {
    let (outcome, text, _) = run(
        e,
        Source::Typed,
        "cat(paste(R.version$major, R.version$minor, sep = '.'), \
         if (requireNamespace('metafor', quietly = TRUE)) as.character(packageVersion('metafor')) \
         else 'none')",
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let mut words = text.split_whitespace();
    let (r, metafor) = (words.next().unwrap_or(""), words.next().unwrap_or("none"));
    if metafor == "none" {
        eprintln!("skipped: metafor is not installed");
        return false;
    }
    let pinned = include_str!("../../../ENVIRONMENT")
        .lines()
        .find_map(|l| l.strip_prefix("R_VERSION="))
        .unwrap_or("")
        .trim();
    eprintln!("ran with R {r} and metafor {metafor} (ENVIRONMENT pins R {pinned})");
    if r != pinned {
        eprintln!("note: this R is not the pinned one, so the run is not a Verified one");
    }
    true
}

/// 14 studies: group sizes, means and standard deviations (3 decimals), a
/// 2×2 table (study 3 has no events in group 1) and an estimate with its
/// standard error (4 decimals). `mm` is the same with study 5's estimate
/// missing. The helpers read a fit's figures and a test's, in Stata's order.
const MAKE: &str = "md <- local({ \
     set.seed(611); k <- 14; \
     n1 <- sample(20:120, k, TRUE); n2 <- sample(20:120, k, TRUE); \
     m1 <- round(rnorm(k, 10.4 + rnorm(k, 0, 0.6), 0.3), 3); m2 <- round(rnorm(k, 10, 0.3), 3); \
     sd1 <- round(runif(k, 1.5, 3), 3); sd2 <- round(runif(k, 1.5, 3), 3); \
     a <- rbinom(k, n1, plogis(-1 + rnorm(k, 0, 0.4))); c <- rbinom(k, n2, plogis(-1.5)); \
     a[3] <- 0; b <- n1 - a; dd <- n2 - c; \
     est <- round(rnorm(k, 0.3, 0.35), 4); se <- round(runif(k, 0.08, 0.4), 4); \
     data.frame(study = paste0('S', 1:k), year = 2000 + 1:k, n1, m1, sd1, n2, m2, sd2, a, b, c, \
       d = dd, est, se) }); \
     mm <- local({ k <- md; k$est[5] <- NA; k }); \
     figs <- function(m) c(m$b[1], m$se, m$tau2, m$I2, m$H2, m$QE, m$ci.lb, m$ci.ub); \
     egger <- function(r) if (inherits(r$fit, 'lm')) { s <- coef(summary(r$fit)); \
       c(s[2, 1], s[2, 2], r$zval, r$pval) } else c(r$fit$beta[2], r$fit$se[2], r$zval, r$pval); \
     near <- function(a, b, tol) isTRUE(length(a) == length(b) && \
       all(abs(a - b) <= tol * pmax(abs(b), 1e-3)))";

fn context(dataset: &str) -> Context {
    let mut columns: Vec<ColumnInfo> = [
        "year", "n1", "m1", "sd1", "n2", "m2", "sd2", "a", "b", "c", "d", "est", "se",
    ]
    .into_iter()
    .map(|name| ColumnInfo {
        name: name.into(),
        kind: VarKind::Number,
    })
    .collect();
    columns.insert(
        0,
        ColumnInfo {
            name: "study".into(),
            kind: VarKind::Text,
        },
    );
    Context {
        dataset: dataset.into(),
        frames: Vec::new(),
        columns,
    }
}

/// Every input picker filled with its column (the dialog reads only those
/// of the chosen effect size).
const COLUMNS: &[(&str, &str)] = &[
    ("n1", "n1"),
    ("m1", "m1"),
    ("sd1", "sd1"),
    ("n2", "n2"),
    ("m2", "m2"),
    ("sd2", "sd2"),
    ("ev1", "a"),
    ("ne1", "b"),
    ("ev2", "c"),
    ("ne2", "d"),
    ("est", "est"),
    ("se", "se"),
];

/// Makes the data, then runs the dialog's code on `dataset` with the inputs
/// filled, the values in `set`, and the check boxes in `flags` set.
fn dialog(
    e: &mut Engine,
    dataset: &str,
    set: &[(&str, &str)],
    flags: &[(&str, bool)],
) -> (Outcome, String, usize) {
    let (made, text, _) = run(e, Source::Typed, MAKE);
    assert_eq!(made, Outcome::Ok, "{text}");
    let ctx = context(dataset);
    let mut values = META.defaults(&ctx);
    for (id, text) in COLUMNS.iter().chain(set) {
        values.set_text(id, *text);
    }
    for w in META.widgets {
        if let Widget::Check { id, .. } = w {
            if let Some((_, on)) = flags.iter().find(|(f, _)| f == id) {
                values.set_flag(id, *on);
            }
        }
    }
    let code = META.generate(&ctx, &values, Style::Base);
    assert!(code.is_ok(), "{code:?}");
    let code = code.unwrap_or_default();
    run(e, Source::Menu(META.menu.into()), &code)
}

const NO_PLOTS: &[(&str, bool)] = &[("forest", false), ("funnel", false)];

/// Fits `measure` with each of `methods` (saved as `<save>_<method>`), and
/// checks each fit's figures against Stata's: theta, se, tau², I², H², Q,
/// the interval's ends.
fn fits_match(e: &mut Engine, measure: &str, save: &str, stata: &[(&str, &str)]) {
    let mut checks = Vec::new();
    for (method, values) in stata {
        let name = format!("{save}_{method}");
        let (outcome, text, _) = dialog(
            e,
            "md",
            &[("measure", measure), ("method", method), ("save", &name)],
            NO_PLOTS,
        );
        assert_eq!(outcome, Outcome::Ok, "{text}");
        assert!(text.contains("Test for Heterogeneity"), "{text}");
        let tol = if *method == "REML" { "1e-5" } else { "1e-9" };
        checks.push(format!("near(figs({name}), c({values}), {tol})"));
    }
    holds(e, &checks);
}

fn holds(e: &mut Engine, exprs: &[String]) {
    let all = exprs.join(", ");
    let (outcome, text, _) = run(e, Source::Typed, &format!("stopifnot({all})"));
    if outcome != Outcome::Ok {
        // stopifnot names only the first failure; show each check's figures,
        // so a platform-only difference can be read off the CI log
        let mut each = String::new();
        for x in exprs {
            let shown = x
                .strip_prefix("near(")
                .and_then(|inner| inner.split(", c(").next())
                .unwrap_or(x);
            let (_, got, _) = run(e, Source::Typed, &format!("print({shown}, digits = 17)"));
            each.push_str(&format!("\n{x}\n  -> {}", got.trim()));
        }
        assert_eq!(
            outcome,
            Outcome::Ok,
            "{all}\n{text}\nchecks one by one:{each}"
        );
    }
}

#[test]
fn hedges_g_matches_stata_meta_summarize_and_meta_bias() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    fits_match(
        &mut e,
        "SMD",
        "g",
        &[
            (
                "REML",
                ".2588093729943497, .092330649890864258, .081098644187181074, 69.934040306715559, \
                 3.3260205568071752, 48.239346395648795, .0778446245390787, .43977412144962069",
            ),
            (
                "DL",
                ".25947274020296268, .097452216762723698, .094511474225331515, 73.051044486845285, \
                 3.7107189535114462, 48.239346395648795, .068469905134433712, .45047557527149162",
            ),
            (
                "FE",
                ".2503496047144364, .049635389515717138, 0, 73.051044486845271, \
                 3.7107189535114458, 48.239346395648795, .15306602890501383, .34763318052385894",
            ),
        ],
    );
    // the default REML fit with both plots and Egger's test
    let (outcome, text, plots) = dialog(
        &mut e,
        "md",
        &[("label", "study"), ("egger", "model")],
        &[("funnel", true)],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert_eq!(plots, 2, "{text}");
    assert!(text.contains("Random-Effects Model"), "{text}");
    assert!(
        text.contains("Test for Funnel Plot Asymmetry: z =  0.5711"),
        "{text}"
    );
    holds(
        &mut e,
        &[
            // meta bias, egger (REML) and meta bias, egger traditional
            "near(egger(metafor::regtest(ma)), c(1.4011073668787877, 2.4533567276476504, \
             .57109810044714127, .56793314616063151), 1e-5)"
                .into(),
            "near(egger(metafor::regtest(ma, model = 'lm')), c(.93471013845995332, \
             2.802447609838262, .33353349235809565, .74448738141395576), 1e-9)"
                .into(),
        ],
    );
}

#[test]
fn odds_and_risk_ratios_match_stata_with_mantel_haenszel() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    fits_match(
        &mut e,
        "OR",
        "or",
        &[
            (
                "REML",
                ".24779768937459418, .25256971186293814, .58041449434555681, 70.340736222456698, \
                 3.371627857995438, 41.633429367825919, -.24722984946242338, .74282522821161168",
            ),
            (
                "DL",
                ".25250570216587037, .24622065229527104, .53904090105087743, 68.775092041669936, \
                 3.2025714898327635, 41.633429367825919, -.23007790858282023, .73508931291456092",
            ),
            (
                "FE",
                ".35320906029425925, .13063386279209246, 0, 68.775092041669936, \
                 3.2025714898327631, 41.633429367825919, .097171394060411032, .60924672652810741",
            ),
            // Mantel-Haenszel with half a count in each cell of study 3, and
            // Q from the Mantel-Haenszel estimate, as Stata
            (
                "MH",
                ".24907476671573511, .12104058273228309, 0, 69.244506569314339, \
                 3.2514516544945775, 42.268871508429505, .011839583892719524, .48630994953875067",
            ),
        ],
    );
    fits_match(
        &mut e,
        "RR",
        "rr",
        &[
            (
                "REML",
                ".20631319571218282, .19058797934516222, .31009608147643652, 67.302344805585321, \
                 3.0583232774771485, 37.905647215878744, -.16723237969059879, .57985877111496442",
            ),
            (
                "DL",
                ".21028713404206084, .18609163548801369, .28862714441812182, 65.704318604658269, \
                 2.9158190166060582, 37.905647215878744, -.15444576933860177, .57502003742272345",
            ),
            (
                "FE",
                ".30066980045438441, .10216152351265136, 0, 65.704318604658269, \
                 2.9158190166060574, 37.905647215878744, .10043689376384585, .50090270714492291",
            ),
            (
                "MH",
                ".19835482106910757, .097206387669139124, 0, 66.588409571392219, \
                 2.9929733579631597, 38.908653653521078, .0078338021703565075, .38887583996785863",
            ),
        ],
    );
    // Mantel-Haenszel with the ratios, a forest plot on the ratio scale and
    // Egger's test, which uses the inverse-variance model as Stata's does
    let (outcome, text, plots) = dialog(
        &mut e,
        "md",
        &[
            ("measure", "OR"),
            ("method", "MH"),
            ("egger", "model"),
            ("save", "mh"),
        ],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert_eq!(plots, 1, "{text}");
    assert!(text.contains("Odds ratio, pooled"), "{text}");
    assert!(
        text.contains("Test for Funnel Plot Asymmetry: z = -2.5095"),
        "{text}"
    );
    holds(
        &mut e,
        &[
            "near(predict(mh, transf = exp)$pred, exp(.24907476671573511), 1e-9)".into(),
            "near(egger(metafor::regtest(metafor::rma(measure = 'OR', ai = a, bi = b, ci = c, \
             di = d, data = md, method = 'FE'))), c(-2.228883165200318, .88819021638756979, \
             -2.5094660176122958, .0120913849058795), 1e-9)"
                .into(),
        ],
    );
}

#[test]
fn given_estimates_and_a_missing_study_match_stata() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    fits_match(
        &mut e,
        "given",
        "gv",
        &[
            (
                "REML",
                ".43818306837285304, .087465406293087758, .053378664951235284, 57.497302609867063, \
                 2.352791849470127, 27.1382589426807, .26675402214523802, .60961211460046805",
            ),
            (
                "DL",
                ".43167100097250477, .082444089164264989, .042913042693651496, 52.097148061496576, \
                 2.0875583802062074, 27.1382589426807, .27008355547233648, .59325844647267301",
            ),
            (
                "FE",
                ".37366495699050339, .050515903003394314, 0, 52.097148061496576, \
                 2.0875583802062079, 27.1382589426807, .27465560645733178, .47267430752367501",
            ),
        ],
    );
    holds(
        &mut e,
        &[
            // meta bias, egger random(dlaird) and meta bias, egger traditional
            "near(egger(metafor::regtest(gv_DL)), c(1.9396076710288135, .92368779494596009, \
             2.0998520080502843, .035741861584999431), 1e-9)"
                .into(),
            "near(egger(metafor::regtest(gv_REML, model = 'lm')), c(1.501949487406347, \
             .82898403403081888, 1.8117954336265414, .095100884763284377), 1e-9)"
                .into(),
        ],
    );
    // study 5's estimate missing: left out, by both
    let (outcome, text, _) = dialog(
        &mut e,
        "mm",
        &[("measure", "given"), ("save", "miss")],
        NO_PLOTS,
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(text.contains("omitted"), "{text}");
    holds(
        &mut e,
        &[
            "miss$k == 13".into(),
            "near(figs(miss), c(.44227868708347678, .095268552607012863, .062916256287157121, \
             61.749767038037561, 2.6143631621654171, 27.010342539367606, .25555575511447215, \
             .62900161905248142), 1e-5)"
                .into(),
        ],
    );
}
