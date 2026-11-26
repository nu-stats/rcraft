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
//!   `esize(lnorpeto)`; each study's effect size and standard error.
//!
//! DerSimonian-Laird, Hedges, Sidik-Jonkman, the fixed effects and the tests
//! on them agree to 1e-9. REML, ML, empirical Bayes and Paule-Mandel are
//! iterated: Stata and metafor each stop within about 3e-6 of the solution,
//! so their figures are held to 1e-5.
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
        // the iterated estimators of tau² stop short of the maximum in both
        let tol = if matches!(*method, "REML" | "ML" | "EB" | "PM") {
            "1e-5"
        } else {
            "1e-9"
        };
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
        ("REML", true, false, "kh", G_KH, "", "1e-5"),
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
            "1e-5",
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
            "1e-5",
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
    // standard error and interval are Stata's Greenland-Robins ones, printed
    // after the summary; Egger's test uses the inverse-variance model
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
    assert!(text.contains("Greenland-Robins"), "{text}");
    for shown in ["0.041013085", "0.019434501", "0.002922163", "0.079104007"] {
        assert!(text.contains(shown), "{shown} in\n{text}");
    }
    assert!(
        text.contains("Test for Funnel Plot Asymmetry: z =  5.5166"),
        "{text}"
    );
    holds(
        &mut e,
        &[
            "near(figs(rd_MH)[c(1, 3:6)], c(.041013085231256276, 0, 83.01043621087733, \
             5.8859663050338957, 76.517561965440649), 1e-9)"
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
