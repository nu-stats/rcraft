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
//!   standard error, the test statistic and its p-value;
//! - (P6.11b) Stata's default Hedges's g (approximate correction) with every
//!   tau² estimator (`random(reml)`, `mle`, `ebayes`, `pmandel`, `hedges`,
//!   `sjonkman`, `dlaird`), `se(khartung)` and `predinterval`;
//!   `esize(cohend)` and `esize(glassdelta2)`; `esize(rdiff)` (no half
//!   counts; Mantel-Haenszel with the Greenland-Robins standard error) and
//!   `esize(lnorpeto)`; each study's effect size and standard error;
//! - (P6.11c) `meta regress year` (REML, DerSimonian-Laird, fixed,
//!   Knapp-Hartung) and `meta bias year, egger`; `subgroup(grp)`,
//!   `cumulative(year)` and `leaveoneout`; `meta bias, harbord`, `peters` and
//!   `begg`; `meta trimfill` (linear and run estimators); `glassdelta1`,
//!   `holkinse`, `se(khartung, truncated)`, Fisher's z through `meta set`;
//!   and the Model menu on `rma.uni` fits;
//! - (P6.11d) `meta bias, harbord` on log risk ratios; `meta bias, begg`
//!   with tied variances; Mantel-Haenszel risk-difference subgroups,
//!   `cumulative()` and `leaveoneout` with the Greenland-Robins error;
//!   Knapp-Hartung subgroups and cumulative analysis; `predict` (xb, stdp,
//!   reffects) and `margins, at()` after `meta regress`; `meta multilevel`;
//!   `meta mvregress` (REML, mle, fixed, a moderator, independent
//!   covariance); the Model menu on `rma.mv` fits;
//! - (P6.11e) `meta mvregress` with outcomes missing in some studies and a
//!   moderator missing in one, `wsevariables()` with `wcorrelations()`,
//!   three outcomes, `random(reml, se(jriley))`, `estat heterogeneity` and
//!   `estat recovariance`; `meta multilevel` with three levels and `mle`;
//!   `meta meregress`; `meta trimfill` refitted where Stata stops.
//!
//! DerSimonian-Laird, Hedges, Sidik-Jonkman, the fixed effects and the tests
//! on them agree to 1e-9. REML, ML, empirical Bayes and Paule-Mandel (by the
//! empirical-Bayes iteration, as Stata's pmandel) are iterated, and the
//! dialog stops where Stata stops (both follow the same iterations; Stata
//! stops once tau² moves by less than 1e-6 relative to 1 + tau²), so they
//! agree to 1e-9 too, as does trim-and-fill, whose filled fit the dialog
//! refits at Stata's stop. rma.mv's fits (found by optim) agree to 1e-6 on one
//! machine, but the optimiser stops at slightly different points on Windows,
//! macOS and Linux (up to 2e-5 apart on a flat likelihood: an uncentred
//! moderator, three outcomes or levels, a small variance), so the P6.11e
//! checks of them use 5e-5.
//!
//! The test records the R and metafor versions it ran with (AGENTS.md
//! section 5), and checks R is the one `ENVIRONMENT` pins.

use std::time::{Duration, Instant};

use rcraft_catalog::meta::{META, MV_META};
use rcraft_catalog::postest::{self, COMPARE_MODELS, MODEL_FIT};
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
/// standard error (4 decimals); a year, two alternating groups, a
/// correlation and its sample size, a lab holding two studies each, standard
/// errors with ties, and a second outcome with its standard error and
/// within-study covariances (correlation 0.5), made without further random
/// draws.
/// `mm` is the same with study 5's estimate missing. The helpers read a
/// fit's figures and a test's, in Stata's order.
const MAKE: &str = "md <- local({ \
     set.seed(611); k <- 14; \
     n1 <- sample(20:120, k, TRUE); n2 <- sample(20:120, k, TRUE); \
     m1 <- round(rnorm(k, 10.4 + rnorm(k, 0, 0.6), 0.3), 3); m2 <- round(rnorm(k, 10, 0.3), 3); \
     sd1 <- round(runif(k, 1.5, 3), 3); sd2 <- round(runif(k, 1.5, 3), 3); \
     a <- rbinom(k, n1, plogis(-1 + rnorm(k, 0, 0.4))); c <- rbinom(k, n2, plogis(-1.5)); \
     a[3] <- 0; b <- n1 - a; dd <- n2 - c; \
     est <- round(rnorm(k, 0.3, 0.35), 4); se <- round(runif(k, 0.08, 0.4), 4); \
     data.frame(study = paste0('S', 1:k), year = 2000 + 1:k, grp = c('A', 'B')[1:k %% 2 + 1], \
       n1, m1, sd1, n2, m2, sd2, a, b, c, d = dd, est, se, r = round(0.3 + 0.25 * sin(1:k), 3), \
       nr = n1 + n2, lab = rep(1:7, each = 2), set = round(c(0.1, 0.1, 0.2, 0.2, 0.3, 0.15, \
       0.15, 0.25, 0.25, 0.3, 0.12, 0.12, 0.35, 0.35), 4), \
       est2 = round(0.7 * est + 0.45 * sin(2 * (1:k)) + 0.1, 4), se2 = round(0.5 * se, 4)) }); \
     md$v11 <- md$se^2; md$v12 <- 0.5 * md$se * md$se2; md$v22 <- md$se2^2; \
     md$reg <- rep(1:3, c(4, 6, 4)); md$est3 <- round(0.5 * md$est2 - 0.3 * cos(1:14) + 0.2, 4); \
     md$se3 <- round(0.8 * md$se, 4); md$v13 <- 0.3 * md$se * md$se3; \
     md$v23 <- 0.2 * md$se2 * md$se3; md$v33 <- md$se3^2; \
     md$e2m <- md$est2; md$e2m[c(3, 8)] <- NA; md$yr <- md$year; md$yr[2] <- NA; \
     mm <- local({ k <- md; k$est[5] <- NA; k }); \
     figs <- function(m) c(m$b[1], m$se, m$tau2, m$I2, m$H2, m$QE, m$ci.lb, m$ci.ub); \
     egger <- function(r) if (inherits(r$fit, 'lm')) { s <- coef(summary(r$fit)); \
       c(s[2, 1], s[2, 2], r$zval, r$pval) } else c(r$fit$beta[2], r$fit$se[2], r$zval, r$pval); \
     near <- function(a, b, tol) isTRUE(length(a) == length(b) && \
       all(abs(a - b) <= tol * pmax(abs(b), 1e-3)))";

fn context(dataset: &str) -> Context {
    let mut columns: Vec<ColumnInfo> = [
        "year", "n1", "m1", "sd1", "n2", "m2", "sd2", "a", "b", "c", "d", "est", "se", "r", "nr",
        "lab", "set", "est2", "se2", "v11", "v12", "v22", "reg", "est3", "se3", "v13", "v23",
        "v33", "e2m", "yr",
    ]
    .into_iter()
    .map(|name| ColumnInfo {
        name: name.into(),
        kind: VarKind::Number,
    })
    .collect();
    for name in ["study", "grp"] {
        columns.push(ColumnInfo {
            name: name.into(),
            kind: VarKind::Text,
        });
    }
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
    ("r", "r"),
    ("nr", "nr"),
];

/// Makes the data, then runs the dialog's code on `dataset` with the inputs
/// filled, the values in `set` (`mods` a comma-separated list), and the check
/// boxes in `flags` set.
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
        if matches!(*id, "mods" | "outer" | "fcols") {
            values.set_list(id, text.split(',').map(|m| m.trim().to_string()).collect());
        } else {
            values.set_text(id, *text);
        }
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

fn define(e: &mut Engine, code: &str) {
    let (outcome, text, _) = run(e, Source::Typed, code);
    assert_eq!(outcome, Outcome::Ok, "{code}\n{text}");
}

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
        // REML, ML, empirical Bayes and Paule-Mandel (by the empirical-Bayes
        // iteration, as Stata's pmandel) stop where Stata stops, to 1e-9
        let tol = "1e-9";
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
    // the REML fit with both plots and Egger's test
    let (outcome, text, plots) = dialog(
        &mut e,
        "md",
        &[("measure", "SMD"), ("label", "study"), ("egger", "model")],
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
             .57109810044714127, .56793314616063151), 1e-9)"
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
             .62900161905248142), 1e-9)"
                .into(),
        ],
    );
}

/// Hedges's g as Stata's default `meta esize` (approximate correction).
const G_REML: &str = ".25881003882646675, .092330886239529425, .081099022652641892, \
     69.934047556771844, 3.3260213588385188, 48.239367508140369, .077844827136324235, \
     .43977525051660926";
const G_DL: &str = ".25947342943252372, .097452474704541922, .094511939320034322, \
     73.051056281353084, 3.7107205775492589, 48.239367508140369, .068470088807320917, \
     .45047677005772646";
/// The same REML fit with Knapp-Hartung errors (`se(khartung)`).
const G_KH: &str = ".25881003882646675, .088359544204031193, .081099022652641892, \
     69.934047556771844, 3.3260213588385188, 48.239367508140369, .067920849028739017, \
     .44969922862419415";

#[test]
fn stata_default_g_and_every_tau2_estimator_match_meta_summarize() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    // meta esize n1 m1 sd1 n2 m2 sd2, then meta summarize, random(...)
    fits_match(
        &mut e,
        "SMDA",
        "ga",
        &[
            ("REML", G_REML),
            (
                "ML",
                ".25839294672029911, .08940278550358427, .073774054333069244, 67.906874580349495, \
                 3.1159321098334147, 48.239367508140369, .083166707015714325, .43361918642488384",
            ),
            (
                "EB",
                ".25827862727834505, .088634520724617819, .071893713190468903, 67.341623592284009, \
                 3.0620015750805556, 48.239367508140369, .084558158871125139, .43199909568556494",
            ),
            // Stata's pmandel and ebayes give the same figures
            (
                "PM",
                ".25827862727834505, .088634520724617819, .071893713190468903, 67.341623592284009, \
                 3.0620015750805556, 48.239367508140369, .084558158871125139, .43199909568556494",
            ),
            (
                "HE",
                ".25753895078988048, .083974379160146759, .060859916569161468, 63.577272643765589, \
                 2.7455384936428491, 48.239367508140369, .092952192011881984, .42212570956787898",
            ),
            (
                "SJ",
                ".2585824152143314, .090707735566910155, .077007568279829047, 68.834471557964847, \
                 3.2086733323322352, 48.239367508140369, .080798520384004618, .4363663100446582",
            ),
            ("DL", G_DL),
        ],
    );
    // each study's g and its standard error, as Stata's _meta_es and _meta_se
    holds(
        &mut e,
        &[
            "near(ga_DL$yi[1:3], c(.113178589846906, .2814112449382859, .37339465497615593), 1e-12)"
                .into(),
            "near(sqrt(ga_DL$vi[1:3]), c(.19399175470240737, .15342990019084943, \
             .16446197700618492), 1e-12)"
                .into(),
        ],
    );
    // se(khartung), predinterval, and both: the pooled figures, then the
    // prediction interval's ends
    let cases: [(&str, bool, bool, &str, &str, &str, &str); 5] = [
        ("REML", true, false, "kh", G_KH, "", "1e-9"),
        (
            "DL",
            true,
            false,
            "dlkh",
            ".25947342943252372, .0880268967397876, .094511939320034322, 73.051056281353084, \
             3.7107205775492589, 48.239367508140369, .069302880790199695, .44964397807484746",
            "",
            "1e-9",
        ),
        (
            "REML",
            false,
            true,
            "gpi",
            G_REML,
            "-.39346704697599749, .91108712462893093",
            "1e-9",
        ),
        (
            "DL",
            false,
            true,
            "gdlpi",
            G_DL,
            "-.44320274135426219, .9621496002193096",
            "1e-9",
        ),
        (
            "REML",
            true,
            true,
            "khpi",
            G_KH,
            "-.39085053876360437, .9084706164165379",
            "1e-9",
        ),
    ];
    for (method, knha, pi, name, figures, ends, tol) in cases {
        let (outcome, text, _) = dialog(
            &mut e,
            "md",
            &[("method", method), ("save", name)],
            &[("forest", false), ("knha", knha), ("pi", pi)],
        );
        assert_eq!(outcome, Outcome::Ok, "{text}");
        let mut checks = vec![format!("near(figs({name}), c({figures}), {tol})")];
        if pi {
            assert!(text.contains("prediction interval"), "{text}");
            checks.push(format!(
                "near(with(predict({name}, pi.type = 'Riley'), c(pi.lb, pi.ub)), c({ends}), {tol})"
            ));
        }
        holds(&mut e, &checks);
    }
}

#[test]
fn cohen_glass_risk_difference_and_peto_match_meta_esize() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    // esize(cohend) and esize(glassdelta2), with random(dlaird)
    fits_match(
        &mut e,
        "COHEN",
        "cd",
        &[(
            "DL",
            ".26110259454547424, .098060256298579951, .095649664516655949, 73.060116160072042, \
             3.7119684922987179, 48.255590399883324, .068908023885490582, .45329716520545793",
        )],
    );
    fits_match(
        &mut e,
        "GLASS",
        "gd",
        &[(
            "DL",
            ".27287046128033893, .098673869898064245, .096748058053474972, 72.93034135062463, \
             3.694172922358129, 48.024247990655667, .079473230064942058, .46626769249573574",
        )],
    );
    holds(
        &mut e,
        &[
            "near(cd_DL$yi[1:3], c(.11401078536048619, .28262422444233021, .37507157707934224), \
             1e-12)"
                .into(),
            "near(sqrt(cd_DL$vi[1:3]), c(.19541816466345449, .15409123596753413, \
             .16520057869932647), 1e-12)"
                .into(),
            "near(gd_DL$yi[1:3], c(.11499193981730275, .3982808022922627, .36981835118770323), \
             1e-12)"
                .into(),
            "near(sqrt(gd_DL$vi[1:3]), c(.19559803965787306, .15700803665443355, \
             .16576284123713728), 1e-12)"
                .into(),
        ],
    );
    // esize(rdiff): no half counts for study 3's empty cell
    fits_match(
        &mut e,
        "RD",
        "rd",
        &[
            (
                "DL",
                ".04194931351007343, .042287860753146223, .019126485758058268, 80.672804977543862, \
                 5.174056550048296, 67.262735150627861, -.040933370549338002, .12483199756948485",
            ),
            (
                "FE",
                "-.012221286867065974, .017498788731174511, 0, 80.672804977543862, \
                 5.1740565500482969, 67.262735150627861, -.046518282553243362, .02207570881911141",
            ),
        ],
    );
    // fixed (Mantel-Haenszel): the estimate, I², H² and Q are metafor's; the
    // fit carries Stata's Greenland-Robins standard error and interval, so
    // the summary shows them; Egger's test uses the inverse-variance model
    let (outcome, text, _) = dialog(
        &mut e,
        "md",
        &[
            ("measure", "RD"),
            ("method", "MH"),
            ("egger", "model"),
            ("save", "rd_MH"),
        ],
        NO_PLOTS,
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(text.contains("Greenland and Robins'"), "{text}");
    // the summary's Model Results line: estimate, se, z, p, interval
    assert!(text.contains("0.0410  0.0194"), "{text}");
    assert!(
        text.contains("Test for Funnel Plot Asymmetry: z =  5.5166"),
        "{text}"
    );
    holds(
        &mut e,
        &[
            "near(figs(rd_MH), c(.041013085231256276, .019434500920009243, 0, 83.01043621087733, \
             5.8859663050338957, 76.517561965440649, .0029221633705276209, .079104007091984924), \
             1e-9)"
                .into(),
            "near(egger(metafor::regtest(metafor::rma(measure = 'RD', ai = a, bi = b, ci = c, \
             di = d, data = md, to = 'none', method = 'FE'))), c(4.2547003162319736, \
             .77125941796116004, 5.516561894932007, 3.4569577408841130e-08), 1e-9)"
                .into(),
        ],
    );
    // esize(lnorpeto): Stata's fixed is inverse variance on Peto's log odds
    // ratios, with no half counts
    fits_match(
        &mut e,
        "PETO",
        "peto",
        &[
            (
                "FE",
                ".25184791494374653, .12240942650699355, 0, 74.971618636173829, \
                 3.9954641311535735, 51.941033704996457, .011929847621836576, .49176598226565649",
            ),
            (
                "DL",
                ".16750462818167408, .25110901994843898, .63860819833483573, 74.971618636173829, \
                 3.9954641311535735, 51.941033704996457, -.3246600071104162, .65966926347376431",
            ),
        ],
    );
    holds(
        &mut e,
        &[
            "near(egger(metafor::regtest(peto_FE)), c(-2.4879003423156396, 1.1931383740822135, \
             -2.0851733515229394, .037053567903594681), 1e-9)"
                .into(),
            "near(predict(peto_FE, transf = exp)$pred, exp(.25184791494374653), 1e-9)".into(),
        ],
    );
}

/// Each of `fit`'s subgroups by `g` refitted alone, as the dialog does, and
/// the checks of their figures and Q_b against Stata's.
const SUBGROUPS: &str = "sub <- function(f, g) lapply(split(seq_along(g), g), \
     function(.rows) stats::update(f, subset = .rows)); \
     qb <- function(fits) metafor::rma(sapply(fits, function(f) f$b[1]), \
       sei = sapply(fits, function(f) f$se), method = 'FE')$QE";

#[test]
fn meta_regression_matches_meta_regress_and_has_the_model_menu() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    // meta set est se; meta regress year, with random(reml), random(dlaird),
    // fixed and se(khartung): coefficients, their variances, tau², I², H²,
    // Q_res and the Wald test (F with Knapp-Hartung)
    for (method, name, knha, b, v, rest, tol) in [
        (
            "REML",
            "mr",
            false,
            ".010953280798438, -21.5379165857404",
            ".000469631229358322, 1891.34424596754",
            "c(mr$tau2, mr$I2, mr$H2, mr$QE, mr$QM), c(.063238416054054294, 58.70461046144041, \
             2.4215778351388346, 26.884601716956038, .25546503884198174)",
            "1e-9",
        ),
        (
            "DL",
            "mr_dl",
            false,
            ".0107030086563275, -21.0391649972868",
            ".000430822636113191, 1734.96211306003",
            "c(mr_dl$tau2, mr_dl$I2, mr_dl$QE, mr_dl$QM), c(.055178045414811298, \
             55.36478417520452, 26.884601716956038, .26589687888944907)",
            "1e-9",
        ),
        (
            "REML",
            "mr_kh",
            true,
            ".010953280798438, -21.5379165857404",
            ".00045888577737397, 1848.06912389209",
            "c(mr_kh$QM, mr_kh$QMdf), c(.26144710985814229, 1, 12)",
            "1e-9",
        ),
    ] {
        let (outcome, text, plots) = dialog(
            &mut e,
            "md",
            &[
                ("measure", "given"),
                ("method", method),
                ("mods", "year"),
                ("save", name),
            ],
            &[("forest", false), ("knha", knha)],
        );
        assert_eq!(outcome, Outcome::Ok, "{text}");
        assert!(text.contains("Test of Moderators"), "{text}");
        // the bubble plot, for its one numeric moderator
        assert_eq!(plots, 1, "{text}");
        holds(
            &mut e,
            &[
                format!("near(coef({name})[c('year', 'intrcpt')], c({b}), {tol})"),
                format!("near(diag(vcov({name}))[c('year', 'intrcpt')], c({v}), {tol})"),
                format!("near({rest}, {tol})"),
            ],
        );
    }
    let (outcome, text, _) = dialog(
        &mut e,
        "md",
        &[
            ("measure", "given"),
            ("method", "FE"),
            ("mods", "year"),
            ("save", "mr_fe"),
        ],
        &[("forest", false), ("bubble", false)],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    holds(
        &mut e,
        &[
            "near(c(mr_fe$QE, mr_fe$QM), c(26.884601716956038, .2536572257198329), 1e-9)".into(),
            // meta bias year, egger: Egger's test adjusted for the moderator
            "near(with(metafor::regtest(mr), c(fit$beta['sei', 1], fit$se[3], zval, pval)), \
             c(2.2679977298555944, 1.1103640491556253, 2.0425712914429193, .041094895442178397), \
             1e-9)"
                .into(),
        ],
    );
    // the Model menu on rma fits: Model fit and Compare models (metafor's
    // likelihood-ratio test), on ML fits with and without the moderator
    let entries: Vec<&str> = postest::model_specs("rma.uni")
        .iter()
        .map(|s| s.entry)
        .collect();
    assert_eq!(entries, ["Model fit", "Compare models"]);
    for (save, mods) in [("ml_full", "year"), ("ml_none", "")] {
        let mut set = vec![("measure", "given"), ("method", "ML"), ("save", save)];
        if !mods.is_empty() {
            set.push(("mods", mods));
        }
        let (outcome, text, _) = dialog(&mut e, "md", &set, &[("forest", false)]);
        assert_eq!(outcome, Outcome::Ok, "{text}");
    }
    let (outcome, text, _) = run(&mut e, Source::Typed, "cat(class(ml_full)[1])");
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert_eq!(text.trim(), "rma.uni");
    let ctx = |name: &str| Context {
        dataset: name.into(),
        frames: Vec::new(),
        columns: Vec::new(),
    };
    let fit_code = MODEL_FIT.generate(
        &ctx("ml_full"),
        &MODEL_FIT.defaults(&ctx("ml_full")),
        Style::Base,
    );
    let (outcome, text, _) = run(
        &mut e,
        Source::Menu(MODEL_FIT.menu.into()),
        &fit_code.unwrap_or_default(),
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(text.contains("Observations: 14"), "{text}");
    assert!(text.contains("-4.04095"), "{text}");
    let mut values = COMPARE_MODELS.defaults(&ctx("ml_full"));
    values.set_text("others", "ml_none");
    let compare = COMPARE_MODELS.generate(&ctx("ml_full"), &values, Style::Base);
    let (outcome, text, _) = run(
        &mut e,
        Source::Menu(COMPARE_MODELS.menu.into()),
        &compare.unwrap_or_default(),
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(text.contains("LRT"), "{text}");
    holds(
        &mut e,
        &["near(metafor::anova.rma(ml_full, ml_none)$LRT, \
           2 * (as.numeric(logLik(ml_full)) - as.numeric(logLik(ml_none))), 1e-9)"
            .into()],
    );
}

#[test]
fn subgroups_cumulative_leave_one_out_and_trim_and_fill_match_stata() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    // meta summarize, subgroup(grp) with random(dlaird), fixed and the
    // default REML: each group's estimate, se, tau², I², Q, and Q_b
    let (outcome, text, _) = dialog(
        &mut e,
        "md",
        &[
            ("measure", "given"),
            ("method", "DL"),
            ("subgroup", "grp"),
            ("cumul", "year"),
            ("trimfill", "L0"),
            ("save", "s_dl"),
        ],
        &[("leave1out", true), ("forest", false)],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(text.contains("Subgroups of grp"), "{text}");
    assert!(text.contains("Q_b = 2.973132 on 1 df"), "{text}");
    assert!(
        text.contains("Estimated number of missing studies"),
        "{text}"
    );
    define(&mut e, SUBGROUPS);
    holds(
        &mut e,
        &[
            "near(sapply(sub(s_dl, md$grp), figs)[c(1, 2, 3, 4, 6), ], c(.294453587090571, \
             .110827746553122, .037233653392947, 46.2608565243747, 11.1650458342743, \
             .566831459543743, .112563731686893, .0352010241759354, 44.0479389940153, \
             10.7234655741425), 1e-9)"
                .into(),
            "near(qb(sub(s_dl, md$grp)), 2.9731321445579333, 1e-9)".into(),
            // meta summarize, cumulative(year) random(dlaird): rows 2 to 4
            "near(unlist(as.data.frame(metafor::cumul(s_dl, order = year))[2:4, c('estimate', \
             'se', 'tau2', 'I2', 'Q')]), c(.26368868, .36200163, .28201649, .14748658, .13455053, \
             .12035189, .02331762, .02893149, .03376857, 44.894548, 52.20674, 60.850468, \
             1.8147025, 4.1846905, 7.6629269), 1e-6)"
                .into(),
            // meta summarize, leaveoneout random(dlaird): studies 1 to 3 left out
            "near(unlist(as.data.frame(metafor::leave1out(s_dl))[1:3, c('estimate', 'se', 'tau2', \
             'I2', 'Q')]), c(.45518838, .4617841, .41308877, .09851711, .08489608, .08661808, \
             .06480863, .04148434, .0437233, 55.721832, 51.486203, 52.202054, 27.101392, \
             24.735231, 25.105682), 1e-6)"
                .into(),
            // meta trimfill, random(dlaird): 4 studies imputed
            "with(metafor::trimfill(s_dl), k0 == 4 && near(c(b[1], ci.lb, ci.ub), \
             c(.30608155, .12851069, .48365241), 1e-6))"
                .into(),
        ],
    );
    let (outcome, text, _) = dialog(
        &mut e,
        "md",
        &[
            ("measure", "given"),
            ("method", "FE"),
            ("subgroup", "grp"),
            ("save", "s_fe"),
        ],
        NO_PLOTS,
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(text.contains("Q_b = 5.249748 on 1 df"), "{text}");
    // the REML fit: subgroups, and trim-and-fill with both estimators
    let (outcome, text, _) = dialog(
        &mut e,
        "md",
        &[
            ("measure", "given"),
            ("subgroup", "grp"),
            ("trimfill", "R0"),
            ("save", "s_reml"),
        ],
        NO_PLOTS,
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    holds(
        &mut e,
        &[
            "near(sapply(sub(s_reml, md$grp), figs)[c(1, 2, 3, 4, 5, 6), ], c(.296139110837349, \
             .112407082465521, .0394164436165783, 47.6797819255571, 1.91130701821076, \
             11.1650458342743, .566775481621335, .112514173623236, .0351383983621339, \
             44.0040576184119, 1.78584368343233, 10.7234655741425), 1e-9)"
                .into(),
            "near(qb(sub(s_reml, md$grp)), 2.895617673812044, 1e-9)".into(),
            // the dialog's trim-and-fill, its filled fit stopped where Stata stops
            "with(s_reml_tf, k0 == 5 && near(c(b[1], ci.lb, ci.ub), c(.287421577753191, \
             .0925710551214481, .482272100384935), 1e-9))"
                .into(),
            // meta summarize, cumulative(year) (REML): rows 3 to 5; with 3 to 5
            // studies Stata stops up to 2e-5 short of the maximum in tau²
            "near(unlist(as.data.frame(metafor::cumul(s_reml, order = year))[3:5, c('estimate', \
             'tau2')]), c(.36198702, .28166399, .30632856, .02901714, .03776144, .02663685), \
             1e-6)"
                .into(),
        ],
    );
    // meta trimfill (linear estimator) after REML
    let (outcome, text, _) = dialog(
        &mut e,
        "md",
        &[("measure", "given"), ("trimfill", "L0"), ("save", "s_tf")],
        NO_PLOTS,
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    holds(
        &mut e,
        &[
            "with(s_tf_tf, k0 == 4 && near(c(b[1], ci.lb, ci.ub), c(.307618260078669, \
           .109446141571852, .505790378585486), 1e-9))"
                .into(),
        ],
    );
    // Mantel-Haenszel subgroups of the log odds ratio: each group's own
    // Mantel-Haenszel fit
    let (outcome, text, _) = dialog(
        &mut e,
        "md",
        &[
            ("measure", "OR"),
            ("method", "MH"),
            ("subgroup", "grp"),
            ("save", "s_mh"),
        ],
        NO_PLOTS,
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(text.contains("Q_b = 0.008664349 on 1 df"), "{text}");
    holds(
        &mut e,
        &[
            "near(sapply(sub(s_mh, md$grp), figs)[c(1, 2, 4, 6), ], c(.260017647996501, \
           .169798971835095, 79.7245406065768, 29.5924244357502, .237475811827043, \
           .172669857142193, 53.0315365567198, 12.7745290353083), 1e-9)"
                .into(),
        ],
    );
}

#[test]
fn small_study_tests_effect_size_variants_and_plots_match_stata() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    // meta esize a b c d, esize(lnoratio); meta bias, harbord (REML and
    // random(dlaird)); meta bias, peters; meta bias, begg
    for (method, egger, shown) in [
        (
            "REML",
            "harbord",
            ["-2.65716", "2.29033", "-1.16016", "0.245981"],
        ),
        (
            "DL",
            "harbord",
            ["-2.64245", "2.21255", "-1.19429", "0.232361"],
        ),
        ("REML", "begg", ["-15", "18.26654", "-0.875918", "0.443421"]),
    ] {
        let (outcome, text, _) = dialog(
            &mut e,
            "md",
            &[("measure", "OR"), ("method", method), ("egger", egger)],
            NO_PLOTS,
        );
        assert_eq!(outcome, Outcome::Ok, "{text}");
        for s in shown {
            assert!(text.contains(s), "{s} in\n{text}");
        }
    }
    let (outcome, text, plots) = dialog(
        &mut e,
        "md",
        &[("measure", "OR"), ("egger", "peters"), ("save", "or_reml")],
        &[("forest", false), ("galbraith", true), ("labbe", true)],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert_eq!(plots, 2, "{text}");
    holds(
        &mut e,
        &[
            "near(with(metafor::regtest(or_reml, predictor = 'ninv'), c(fit$beta[2], fit$se[2], \
           zval, pval)), c(12.31492006964363, 85.42666648941385, .1441577972748089, \
           .885375865672394), 1e-9)"
                .into(),
        ],
    );
    // meta bias, begg on the given estimates and on Stata's default g
    for (measure, shown) in [
        ("given", ["31", "1.64234", "0.100518"]),
        ("SMDA", ["5", "0.218979", "0.82666"]),
    ] {
        let (outcome, text, _) = dialog(
            &mut e,
            "md",
            &[("measure", measure), ("egger", "begg")],
            NO_PLOTS,
        );
        assert_eq!(outcome, Outcome::Ok, "{text}");
        for s in shown {
            assert!(text.contains(s), "{s} in\n{text}");
        }
    }
    // se(khartung, truncated), with REML and random(dlaird)
    for (method, name, values, tol) in [
        (
            "REML",
            "kht",
            ".43818306837285304, .08767735888615226, .24876765035377008, .62759848639193572",
            "1e-9",
        ),
        (
            "DL",
            "kht_dl",
            ".43167100097250477, .086961511083052195, .24380207811003693, .61953992383497236",
            "1e-9",
        ),
    ] {
        let (outcome, text, _) = dialog(
            &mut e,
            "md",
            &[("measure", "given"), ("method", method), ("save", name)],
            &[("forest", false), ("knha", true), ("truncated", true)],
        );
        assert_eq!(outcome, Outcome::Ok, "{text}");
        holds(
            &mut e,
            &[format!(
                "near(figs({name})[c(1, 2, 7, 8)], c({values}), {tol})"
            )],
        );
    }
    // esize(glassdelta1), esize(cohend, holkinse), esize(hedgesg, holkinse)
    // and esize(hedgesg, exact holkinse): each study's effect size and error
    for (measure, holkinse, name, es, se) in [
        (
            "GLASS1",
            false,
            "gd1",
            ".11310782241014822, .24497708847373934, .38693957115009681",
            ".19556854019784597, .1542862572687845, .16811837292847695",
        ),
        (
            "COHEN",
            true,
            "dh",
            ".11401078536048619, .28262422444233021, .37507157707934224",
            ".19542123981843421, .15409960331350805, .16521548634120153",
        ),
        (
            "SMDA",
            true,
            "gh",
            ".113178589846906, .2814112449382859, .37339465497615593",
            ".1954219456580267, .15410149058690154, .16521885221837693",
        ),
        (
            "SMD",
            true,
            "geh",
            ".11317825161162647, .28141095532259036, .37339423785705872",
            ".19542194468885679, .15410148905887891, .16521884937997885",
        ),
    ] {
        let (outcome, text, _) = dialog(
            &mut e,
            "md",
            &[("measure", measure), ("method", "DL"), ("save", name)],
            &[("forest", false), ("holkinse", holkinse)],
        );
        assert_eq!(outcome, Outcome::Ok, "{text}");
        holds(
            &mut e,
            &[
                format!("near({name}$yi[1:3], c({es}), 1e-12)"),
                format!("near(sqrt({name}$vi[1:3]), c({se}), 1e-12)"),
            ],
        );
    }
    // Fisher's z: Stata's meta set atanh(r) 1/sqrt(n - 3), with
    // random(dlaird) and REML, and the pooled correlation
    let (outcome, text, _) = dialog(
        &mut e,
        "md",
        &[("measure", "ZCOR"), ("method", "DL"), ("save", "zc_dl")],
        NO_PLOTS,
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(text.contains("Correlation, pooled"), "{text}");
    let (outcome, text, _) = dialog(
        &mut e,
        "md",
        &[("measure", "ZCOR"), ("save", "zc_reml")],
        NO_PLOTS,
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    holds(
        &mut e,
        &[
            "near(figs(zc_dl), c(.34666867252727884, .056407956301585648, .036190531828433355, \
             82.631398194790677, 5.7575158393007344, 74.847705910909525, .23611110973466182, \
             .45722623531989587), 1e-9)"
                .into(),
            "near(figs(zc_reml), c(.34668478062702091, .056927373424637058, .037009988950375765, \
             82.950398142668149, 5.8652396012987786, 74.847705910909525, .23510917898026967, \
             .45826038227377208), 1e-9)"
                .into(),
            "near(predict(zc_dl, transf = tanh)$pred, tanh(.34666867252727884), 1e-9)".into(),
        ],
    );
}

#[test]
fn harbord_for_risk_ratios_begg_with_ties_and_risk_difference_refits_match_stata() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    // meta esize a b c d, esize(lnrratio); meta bias, harbord
    let (outcome, text, _) = dialog(
        &mut e,
        "md",
        &[("measure", "RR"), ("egger", "harbord")],
        NO_PLOTS,
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    for s in ["-3.39331", "1.80128", "-1.88382", "0.059588"] {
        assert!(text.contains(s), "{s} in\n{text}");
    }
    // meta set est set (seven pairs of tied standard errors); meta bias, begg
    let (outcome, text, _) = dialog(
        &mut e,
        "md",
        &[("measure", "given"), ("se", "set"), ("egger", "begg")],
        NO_PLOTS,
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    for s in ["-2", "18.07392", "-0.165985", "0.955876"] {
        assert!(text.contains(s), "{s} in\n{text}");
    }
    // meta esize a b c d, esize(rdiff) fixed: subgroups, cumulative(year)
    // and leaveoneout, each refit with the Greenland-Robins error
    let (outcome, text, plots) = dialog(
        &mut e,
        "md",
        &[
            ("measure", "RD"),
            ("method", "MH"),
            ("subgroup", "grp"),
            ("cumul", "year"),
            ("save", "rd_sub"),
        ],
        &[("leave1out", true)],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    // the forest plot with the subgroups' diamonds
    assert_eq!(plots, 1, "{text}");
    for s in [
        "Q_b = 0.03760304 on 1 df",
        "0.027874",
        "0.027100",
        "Cumulative meta-analysis, in the order of year",
        "0.046335",
        "0.032129",
        "0.030012",
        "Leave-one-out meta-analysis",
        "0.019963",
        "0.020748",
        "0.021130",
    ] {
        assert!(text.contains(s), "{s} in\n{text}");
    }
    // meta summarize, subgroup(grp) se(khartung): each group's Knapp-Hartung
    // error, and Q_b from the unadjusted ones
    let (outcome, text, _) = dialog(
        &mut e,
        "md",
        &[
            ("measure", "given"),
            ("subgroup", "grp"),
            ("cumul", "year"),
            ("save", "kh_sub"),
        ],
        &[("knha", true), ("forest", false)],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(text.contains("Q_b = 2.895618 on 1 df"), "{text}");
    define(&mut e, SUBGROUPS);
    holds(
        &mut e,
        &[
            "near(sapply(sub(kh_sub, md$grp), function(f) f$se), c(.118115832290721, \
             .110214602570655), 1e-9)"
                .into(),
            // meta summarize, cumulative(year) se(khartung): rows 2 to 4
            "near(unlist(as.data.frame(metafor::cumul(kh_sub, order = year))[2:4, c('estimate', \
             'se', 'ci.lb')]), c(.263688675147378, .361987023420823, .281663993304748, \
             .14748657894908, .150392818576679, .130778539750541, -1.61030599281763, \
             -.28510104796284, -.13453168733476), 1e-9)"
                .into(),
        ],
    );
}

#[test]
fn predictions_multilevel_and_multivariate_fits_match_stata() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    // meta regress year; predict xb, stdp, reffects, fitted; margins,
    // at(year = (2005 2010))
    let (outcome, text, _) = dialog(
        &mut e,
        "md",
        &[
            ("measure", "given"),
            ("mods", "year"),
            ("at", "2005; 2010"),
            ("save", "mp"),
        ],
        &[("studypred", true), ("forest", false), ("bubble", false)],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(text.contains("Study predictions"), "{text}");
    assert!(
        text.contains("Predictions at the moderator values"),
        "{text}"
    );
    holds(
        &mut e,
        &[
            "near(with(predict(mp), c(pred[c(1, 5)], se[c(1, 5)])), c(.37959829193412276, \
             .42341141512787317, .15581480953968349, .099855664237067959), 1e-9)"
                .into(),
            "near(metafor::ranef(mp)$pred[c(1, 5)], c(-.01756230005993482, .016271495929339106), \
             1e-8)"
                .into(),
            // the predictions to 1e-9; their variances to 1e-6, since Stata's
            // margins differs by 1.5e-7 from its own predict, stdp squared
            // (study 5 is the year 2005)
            "near(with(predict(mp, newmods = rbind(c(2005), c(2010))), pred), \
             c(.423411415127873, .478177819120063), 1e-9)"
                .into(),
            "near(with(predict(mp, newmods = rbind(c(2005), c(2010))), se^2), \
             c(.00997115215938305, .0132206825124426), 1e-6)"
                .into(),
        ],
    );
    // meta multilevel est, relevels(lab study) essevariable(se)
    let (outcome, text, plots) = dialog(
        &mut e,
        "md",
        &[("measure", "given"), ("cluster", "lab"), ("save", "ml3")],
        &[("contours", true)],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(text.contains("Standard deviation of each level"), "{text}");
    // the forest plot and the contour-enhanced funnel plot
    assert_eq!(plots, 2, "{text}");
    holds(
        &mut e,
        &[
            "near(c(ml3$b[1], ml3$se^2, sqrt(ml3$sigma2)), c(.443744155435418, \
           .00898826195461914, exp(-2.10710555540641), exp(-1.58542855357435)), 1e-6)"
                .into(),
        ],
    );
    // meta mvregress est est2, wcovvariables(v11 v12 v22), with REML, mle,
    // fixed, a moderator and covariance(independent): coefficients, their
    // covariances, the between-study SDs and correlation (estat sd), Q_M
    let ctx = context("md");
    for (method, structure, mods, name, checks) in [
        (
            "REML",
            "UN",
            "",
            "mv_reml",
            "c(coef(mv_reml), vcov(mv_reml)[c(1, 2, 4)], sqrt(mv_reml$tau2), mv_reml$rho, \
             mv_reml$QE), c(.438439531162747, .457926532903039, .00789113066144299, \
             .00315798442459061, .00866750955155163, exp(-1.43540492622176), \
             exp(-1.12418862308977), tanh(.414615507704636), 195.1458620349403), 1e-6",
        ),
        (
            "ML",
            "UN",
            "",
            "mv_ml",
            "c(coef(mv_ml), vcov(mv_ml)[c(1, 2, 4)], sqrt(mv_ml$tau2), mv_ml$rho), \
             c(.432829765655791, .456341043649659, .00714670773634329, .00281117708306984, \
             .0079789545310019, exp(-1.5243185922843), exp(-1.1714436705672), \
             tanh(.402470632503434)), 1e-6",
        ),
        (
            "FE",
            "UN",
            "",
            "mv_fe",
            "c(coef(mv_fe), vcov(mv_fe)[c(1, 2, 4)], mv_fe$QE), c(.373825866704263, \
             .520532588288754, .00255185614582068, .000637681716110043, .000637399753315889, \
             195.14586203494025), 1e-9",
        ),
        (
            "REML",
            "UN",
            "year",
            "mv_year",
            "c(coef(mv_year), sqrt(mv_year$tau2), mv_year$rho), c(-24.1284780263221, \
             -19.0631614698363, .0122437485672831, .00972609424738155, exp(-1.36028416216164), \
             exp(-1.07698426028502), tanh(.427249601515083)), 5e-6",
        ),
        (
            "REML",
            "DIAG",
            "",
            "mv_ind",
            "c(coef(mv_ind), sqrt(mv_ind$tau2)), c(.432568739315214, .4535264012457, \
             exp(-1.55254573252271), exp(-1.17076930144505)), 1e-6",
        ),
    ] {
        let (made, text, _) = run(&mut e, Source::Typed, MAKE);
        assert_eq!(made, Outcome::Ok, "{text}");
        let mut values = MV_META.defaults(&ctx);
        values.set_list("outcomes", vec!["est".into(), "est2".into()]);
        values.set_list("wcov", vec!["v11".into(), "v12".into(), "v22".into()]);
        if !mods.is_empty() {
            values.set_list("mods", vec![mods.into()]);
        }
        values.set_text("method", method);
        values.set_text("struct", structure);
        values.set_text("save", name);
        let code = MV_META.generate(&ctx, &values, Style::Base);
        assert!(code.is_ok(), "{code:?}");
        let (outcome, text, _) = run(
            &mut e,
            Source::Menu(MV_META.menu.into()),
            &code.unwrap_or_default(),
        );
        assert_eq!(outcome, Outcome::Ok, "{text}");
        // the maximum optim finds: to 1e-6 of Stata's (5e-6 with the uncentred
        // year, where the likelihood is flatter)
        holds(&mut e, &[format!("near({checks})")]);
    }
    // the Model menu on an rma.mv fit
    let fit_ctx = Context {
        dataset: "mv_reml".into(),
        frames: Vec::new(),
        columns: Vec::new(),
    };
    let fit_code = MODEL_FIT.generate(&fit_ctx, &MODEL_FIT.defaults(&fit_ctx), Style::Base);
    let (outcome, text, _) = run(
        &mut e,
        Source::Menu(MODEL_FIT.menu.into()),
        &fit_code.unwrap_or_default(),
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(text.contains("Observations: 28"), "{text}");
    assert_eq!(
        postest::model_specs("rma.mv")
            .iter()
            .map(|s| s.entry)
            .collect::<Vec<_>>(),
        ["Model fit", "Compare models"]
    );
}

/// Runs the multivariate dialog on `md` with `lists` (outcomes, wcov,
/// mods), `texts` and `flags` set.
fn mv(
    e: &mut Engine,
    lists: &[(&str, &[&str])],
    texts: &[(&str, &str)],
    flags: &[(&str, bool)],
) -> (Outcome, String) {
    let (made, text, _) = run(e, Source::Typed, MAKE);
    assert_eq!(made, Outcome::Ok, "{text}");
    let ctx = context("md");
    let mut values = MV_META.defaults(&ctx);
    for (id, cols) in lists {
        values.set_list(id, cols.iter().map(|c| c.to_string()).collect());
    }
    for (id, t) in texts {
        values.set_text(id, *t);
    }
    for (id, on) in flags {
        values.set_flag(id, *on);
    }
    let code = MV_META.generate(&ctx, &values, Style::Base);
    assert!(code.is_ok(), "{code:?}");
    let (outcome, text, _) = run(
        e,
        Source::Menu(MV_META.menu.into()),
        &code.unwrap_or_default(),
    );
    (outcome, text)
}

/// P6.11e: Stata 19's `meta mvregress` with outcomes missing in some studies,
/// `wsevariables()` with `wcorrelations()`, `random(reml, se(jriley))`,
/// `estat heterogeneity` and `estat recovariance`; `meta multilevel` with
/// three levels and `mle`; `meta meregress`; forest plot columns.
#[test]
fn missing_outcomes_correlations_jackson_riley_heterogeneity_and_three_levels_match_stata() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    let w2: &[&str] = &["v11", "v12", "v22"];
    // meta mvregress est e2m (e2m missing in studies 3 and 8): Stata keeps
    // those studies' est, 26 observations
    let (outcome, text) = mv(
        &mut e,
        &[("outcomes", &["est", "e2m"]), ("wcov", w2)],
        &[("save", "mv_miss")],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(text.contains("as Stata's estat heterogeneity"), "{text}");
    holds(
        &mut e,
        &[
            "mv_miss$k == 26 && mv_miss$g.nlevels[2] == 14".into(),
            "near(c(coef(mv_miss), vcov(mv_miss)[c(1, 2, 4)], sqrt(mv_miss$tau2), mv_miss$rho, \
             mv_miss$QE), c(.4377391683662153, .4900803191657542, .0077898572053714523, \
             .0028076182239332812, .0106662966485116875, exp(-1.447930415233669832), \
             exp(-1.088024864743893927), tanh(.3406520114941014032), 165.2029922547711), 5e-5)"
                .into(),
            // estat heterogeneity: Jackson-White-Riley R for est, e2m and
            // jointly; Cochran's H2
            "near(mv_miss_het$jwr$R, c(1.747175970467998551, 3.849609539896764243, \
             2.692898862211889011), 5e-5)"
                .into(),
            "near(mv_miss_het$H2, 6.88345801061546414, 1e-9)".into(),
        ],
    );
    // random(reml, se(jriley)) on the same: t on N - p = 24 degrees of
    // freedom, as Stata's; Stata's scale with outcomes missing (1.0255) is
    // not Jackson and Riley's residual sum over N - p (0.9972), which it
    // equals with every outcome given (small-fixes row 30), so the dialog
    // says so
    let (outcome, text) = mv(
        &mut e,
        &[("outcomes", &["est", "e2m"]), ("wcov", w2)],
        &[("save", "mv_jr")],
        &[("jriley", true)],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(text.contains("as Stata's se(jriley)"), "{text}");
    assert!(text.contains("Some studies miss an outcome"), "{text}");
    holds(
        &mut e,
        &[
            "near(c(mv_jr$seadj, mv_jr$vb[2]), c(.99720710973728, .0028076182239332812), 5e-5)"
                .into(),
            "all(mv_jr$ddf == 24)".into(),
        ],
    );
    // all 14 studies with both outcomes: estat heterogeneity and
    // recovariance (Stata's est row shows its joint R; est2's and the joint
    // are checked), and se(jriley) with 26 degrees of freedom
    let (outcome, text) = mv(
        &mut e,
        &[("outcomes", &["est", "est2"]), ("wcov", w2)],
        &[("save", "mv_full")],
        &[("jriley", true)],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(!text.contains("Some studies miss an outcome"), "{text}");
    holds(
        &mut e,
        &[
            "near(mv_full_het$jwr$R[2:3], c(3.687578030551223129, 2.630659878884500369), 5e-5)"
                .into(),
            "near(mv_full_het$H2, 7.50561007826693416, 1e-9)".into(),
            "near(mv_full_het$cov[c(1, 2, 4)], c(.0566530274082204799, .030345522274438045, \
             .1055704004008428398), 5e-5)"
                .into(),
            "near(c(mv_full$seadj, mv_full$vb[c(1, 2, 4)]), c(.9935732840770839, \
             .0078404166067143916, .0031579844246917574, .00861180592948497), 5e-5)"
                .into(),
            "all(mv_full$ddf == 26)".into(),
        ],
    );
    // = year with se(jriley): Stata's F test of the moderators keeps the
    // model's errors; the fit to 5e-5 on every platform
    let (outcome, text) = mv(
        &mut e,
        &[
            ("outcomes", &["est", "est2"]),
            ("wcov", w2),
            ("mods", &["year"]),
        ],
        &[("save", "mv_jry")],
        &[("jriley", true)],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    holds(
        &mut e,
        &[
            "near(c(mv_jry$seadj, mv_jry$QM, mv_jry$QMdf, mv_jry$QMp), c(.9817169946100938, \
             .1772741240502183, 2, 24, .8386368882683426), 5e-5)"
                .into(),
            "near(diag(mv_jry$vb), c(1901.18314321006892, 2237.671603694001533, \
             .0004720664937154873, .0005553363301929454), 5e-5)"
                .into(),
            "near(c(mv_jry_het$jwr$R[2], mv_jry_het$H2), c(4.02524830632341768, \
             7.421256775699202279), 5e-5)"
                .into(),
        ],
    );
    // est e2m = year: each outcome's R to 5e-5; the joint R, whose
    // determinant of four coefficients magnifies the fit's last digits, to
    // 5e-5
    let (outcome, text) = mv(
        &mut e,
        &[
            ("outcomes", &["est", "e2m"]),
            ("wcov", w2),
            ("mods", &["year"]),
        ],
        &[("save", "mv_my")],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    holds(
        &mut e,
        &[
            "near(mv_my_het$jwr$R[1:2], c(1.880304525587474851, 4.265838462048480828), 5e-5)"
                .into(),
            "near(mv_my_het$jwr$R[3], 2.944426365376759502, 5e-5)".into(),
            "near(mv_my_het$H2, 6.906050154080543457, 1e-9)".into(),
        ],
    );
    // est e2m = yr (yr missing in study 2): that study left out, 24
    // observations in 13 studies
    let (outcome, text) = mv(
        &mut e,
        &[
            ("outcomes", &["est", "e2m"]),
            ("wcov", w2),
            ("mods", &["yr"]),
        ],
        &[("save", "mv_yr")],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    holds(
        &mut e,
        &[
            "mv_yr$k == 24 && mv_yr$g.nlevels[2] == 13".into(),
            // Stata's Wald test of the moderators only
            "near(c(mv_yr$QM, mv_yr$QMdf[1], mv_yr$QMp), c(.4156468640139263648, 2, \
             .812350459139874137), 5e-5)"
                .into(),
            "near(coef(mv_yr), c(-3.368350471727069362, 28.34974135208959822, \
             .0019142971113977933, -.0138406233834085582), 5e-5)"
                .into(),
        ],
    );
    // wsevariables(se se2) wcorrelations(0.3), and three outcomes with
    // wcorrelations(0.3 0.2 0.4) (six between-study parameters, which optim
    // finds to 5e-5 of Stata's on every platform)
    let (outcome, text) = mv(
        &mut e,
        &[("outcomes", &["est", "est2"]), ("wcov", &["se", "se2"])],
        &[("wcorr", "0.3"), ("save", "mv_r2")],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let (outcome, text) = mv(
        &mut e,
        &[
            ("outcomes", &["est", "est2", "est3"]),
            ("wcov", &["se", "se2", "se3"]),
        ],
        &[("wcorr", "0.3 0.2 0.4"), ("save", "mv_r3")],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    holds(
        &mut e,
        &[
            "near(c(coef(mv_r2), sqrt(mv_r2$tau2), mv_r2$rho), c(.4385192232436938564, \
             .4639406075843779398, exp(-1.414680414448238333), exp(-1.118307116859341255), \
             tanh(.4909807938412857342)), 5e-5)"
                .into(),
            "near(c(coef(mv_r3), sqrt(mv_r3$tau2), mv_r3$rho), c(.4393804304146580741, \
             .4653722669375449095, .4348762406799207003, exp(c(-1.423904196520063836, \
             -1.12668447122193105, -1.780411883840742115)), tanh(c(.4764551081660816645, \
             .3070000252538501728, .8892037829868378873))), 5e-5)"
                .into(),
        ],
    );
    // three outcomes with wcovvariables(v11 v12 v13 v22 v23 v33): estat
    // heterogeneity and recovariance (the fit to 5e-5, as above)
    let (outcome, text) = mv(
        &mut e,
        &[
            ("outcomes", &["est", "est2", "est3"]),
            ("wcov", &["v11", "v12", "v13", "v22", "v23", "v33"]),
        ],
        &[("save", "mv3")],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    holds(
        &mut e,
        &[
            "near(mv3_het$jwr$R[2:4], c(3.697929226434258254, 1.628254706317474376, \
             2.144387150207082815), 5e-5)"
                .into(),
            "near(mv3_het$H2, 5.601780882400499983, 1e-9)".into(),
            "near(mv3_het$cov[lower.tri(mv3_het$cov, diag = TRUE)], c(.0550880632887017011, \
             .0285094467516277469, .0082520972090099395, .1062918595239152331, \
             .0422488899549727029, .0298695075329085819), 5e-5)"
                .into(),
        ],
    );
    // meta multilevel est, relevels(reg lab sid) (the lab level's variance at
    // zero in both: Stata's sd 4e-7); meta meregress est year || lab: || sid:
    // and est year set || reg: || lab: || sid: (the uncentred year to 5e-5);
    // meta multilevel, mle (its small lab variance, on a flat likelihood, to
    // 5e-5)
    for (set, name) in [
        (
            &[
                ("measure", "given"),
                ("cluster", "lab"),
                ("outer", "reg"),
                ("save", "ml_reg"),
            ][..],
            "ml_reg",
        ),
        (
            &[
                ("measure", "given"),
                ("cluster", "lab"),
                ("mods", "year"),
                ("save", "mr_lab"),
            ][..],
            "mr_lab",
        ),
        (
            &[
                ("measure", "given"),
                ("cluster", "lab"),
                ("outer", "reg"),
                ("mods", "year, set"),
                ("save", "mr_reg"),
            ][..],
            "mr_reg",
        ),
        (
            &[
                ("measure", "given"),
                ("cluster", "lab"),
                ("method", "ML"),
                ("save", "ml_ml"),
            ][..],
            "ml_ml",
        ),
    ] {
        let (outcome, text, _) = dialog(&mut e, "md", set, NO_PLOTS);
        assert_eq!(outcome, Outcome::Ok, "{name}: {text}");
    }
    holds(
        &mut e,
        &[
            "near(c(ml_reg$b[1], ml_reg$se^2, log(sqrt(ml_reg$sigma2[c(1, 3)]))), \
             c(.4310301937718455378, .0122192325441253646, -2.070107057949499119, \
             -1.582550302328308733), 5e-5) && sqrt(ml_reg$sigma2[2]) < 1e-4"
                .into(),
            "near(c(mr_lab$b[2], mr_lab$b[1], mr_lab$vb[2, 2], mr_lab$vb[1, 1], \
             log(sqrt(mr_lab$sigma2))), c(.0089170982653387684, -17.44554791946580252, \
             .0005948513462875196, 2395.991595410474474, -1.86896443917259214, \
             -1.535839789741080708), 5e-5)"
                .into(),
            "near(c(mr_reg$b[2], mr_reg$b[3], mr_reg$b[1], log(sqrt(mr_reg$sigma2))), \
             c(.0249062069284876168, -1.6324042016968483, -49.20327261914331984, \
             -1.565734471326559962, -2.107356214110507331, -1.640871665495029497), 5e-5)"
                .into(),
            "near(c(ml_ml$b, log(sqrt(ml_ml$sigma2))), c(.4345414096753246747, \
             -2.764583982891404279, -1.596413041384782572), 5e-5)"
                .into(),
        ],
    );
    // forest plot columns (meta forestplot _id year grp ...): it draws, also
    // with a study left out and with subgroups
    for (data, extra) in [("md", ""), ("mm", ""), ("md", "grp")] {
        let mut set = vec![
            ("measure", "given"),
            ("label", "study"),
            ("fcols", "year, grp"),
            ("save", "fc"),
        ];
        if !extra.is_empty() {
            set.push(("subgroup", extra));
        }
        let (outcome, text, plots) = dialog(&mut e, data, &set, &[("funnel", false)]);
        assert_eq!(outcome, Outcome::Ok, "{data} {extra}: {text}");
        assert_eq!(plots, 1, "{data} {extra}: {text}");
    }
}
