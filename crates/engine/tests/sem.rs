//! Statistics > Structural equation model (P6.10) end to end against a real R
//! (skipped with a note when none is found, or when lavaan is not installed).
//! The dialog's code is built from its spec and run on made-up data (300 rows:
//! a latent variable measured by x1, x2 and x3, a second one measured by w1,
//! w2 and w3, an observed predictor z and an outcome y; a copy with 12 values
//! of x2 and 9 of z missing), and its numbers are checked:
//!
//! - against Stata 19's own results for the same data, recorded below
//!   (`sem (L -> x1 x2 x3) (y <- L)`, a model with two latent variables and an
//!   observed predictor, one with two observed predictors on the data with
//!   missing values, and a path model with no latent variable; each with
//!   `estat gof, stats(all)` and the `standardized` option): coefficients,
//!   standard errors, chi-squared, log-likelihood, AIC, BIC, RMSEA, CFI, TLI
//!   and SRMR. Stata's likelihood is flat near its maximum and it stops about
//!   5e-7 short of it, so estimates and errors are held to 5e-6; the
//!   chi-squared, log-likelihood and information criteria to 1e-9;
//! - with lavaan's own defaults, against lavaan::sem written out by hand.
//!
//! The test records the R and lavaan versions it ran with (AGENTS.md
//! section 5), and checks R is the one `ENVIRONMENT` pins.

use std::time::{Duration, Instant};

use rcraft_catalog::postest::{self, COMPARE_MODELS, MODEL_FIT};
use rcraft_catalog::sem::SEM;
use rcraft_engine::{Engine, Item, Outcome, Source};
use rcraft_rcodegen::dialog::{ColumnInfo, Context, Spec, Style, VarKind, Widget};
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

fn run(e: &mut Engine, source: Source, code: &str) -> (Outcome, String) {
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
                    Item::Warning(w) => format!("warning: {w}\n"),
                    Item::Error { message, .. } => format!("error: {message}\n"),
                    Item::Notice(n) => n.clone(),
                    Item::Plot { .. } => String::new(),
                })
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default();
    (outcome, text)
}

/// The R and lavaan versions, recorded; false when lavaan is missing.
fn versions(e: &mut Engine) -> bool {
    let (outcome, text) = run(
        e,
        Source::Typed,
        "cat(paste(R.version$major, R.version$minor, sep = '.'), \
         if (requireNamespace('lavaan', quietly = TRUE)) as.character(packageVersion('lavaan')) \
         else 'none')",
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let mut words = text.split_whitespace();
    let (r, lavaan) = (words.next().unwrap_or(""), words.next().unwrap_or("none"));
    if lavaan == "none" {
        eprintln!("skipped: lavaan is not installed");
        return false;
    }
    let pinned = include_str!("../../../ENVIRONMENT")
        .lines()
        .find_map(|l| l.strip_prefix("R_VERSION="))
        .unwrap_or("")
        .trim();
    eprintln!("ran with R {r} and lavaan {lavaan} (ENVIRONMENT pins R {pinned})");
    if r != pinned {
        eprintln!("note: this R is not the pinned one, so the run is not a Verified one");
    }
    true
}

/// 300 rows: L1 measured by x1, x2, x3; L2 (driven by L1 and z) measured by
/// w1, w2, w3; y driven by L1 and z; all rounded to 6 decimals. dn is the
/// same with 12 values of x2 and 9 of z missing.
const MAKE: &str = "d <- local({ \
     set.seed(610); n <- 300; \
     L1 <- rnorm(n); z <- 0.4 * L1 + rnorm(n); \
     x1 <- L1 + rnorm(n, sd = 0.6); x2 <- 0.8 * L1 + rnorm(n, sd = 0.7); x3 <- 1.2 * L1 + rnorm(n, sd = 0.8); \
     y <- 0.5 * L1 + 0.3 * z + rnorm(n); \
     L2 <- 0.6 * L1 + 0.4 * z + rnorm(n); \
     w1 <- L2 + rnorm(n, sd = 0.5); w2 <- 0.7 * L2 + rnorm(n, sd = 0.6); w3 <- 1.1 * L2 + rnorm(n, sd = 0.9); \
     round(data.frame(x1, x2, x3, y, z, w1, w2, w3), 6) }); \
     dn <- local({ k <- d; set.seed(11); k$x2[sample(300, 12)] <- NA; k$z[sample(300, 9)] <- NA; k })";

fn context(dataset: &str) -> Context {
    Context {
        dataset: dataset.into(),
        frames: Vec::new(),
        columns: ["x1", "x2", "x3", "y", "z", "w1", "w2", "w3"]
            .into_iter()
            .map(|name| ColumnInfo {
                name: name.into(),
                kind: VarKind::Number,
            })
            .collect(),
    }
}

/// Makes the data, then runs the dialog's code on `dataset`. `off` lists the
/// check boxes to turn off; the rest keep their defaults.
fn dialog(
    e: &mut Engine,
    spec: &Spec,
    dataset: &str,
    set: &[(&str, &str)],
    lists: &[(&str, &[&str])],
    off: &[&str],
) -> (Outcome, String) {
    let (made, text) = run(e, Source::Typed, MAKE);
    assert_eq!(made, Outcome::Ok, "{text}");
    let ctx = context(dataset);
    let mut values = spec.defaults(&ctx);
    for (id, text) in set {
        values.set_text(id, *text);
    }
    for (id, names) in lists {
        values.set_list(id, names.iter().map(|n| n.to_string()).collect());
    }
    for w in spec.widgets {
        if let Widget::Check { id, .. } = w {
            if off.contains(id) {
                values.set_flag(id, false);
            }
        }
    }
    let code = spec.generate(&ctx, &values, Style::Base);
    assert!(code.is_ok(), "{code:?}");
    let code = code.unwrap_or_default();
    run(e, Source::Menu(spec.menu.into()), &code)
}

fn holds(e: &mut Engine, exprs: &[String]) {
    let all = exprs.join(", ");
    let (outcome, text) = run(e, Source::Typed, &format!("stopifnot({all})"));
    if outcome != Outcome::Ok {
        // stopifnot names only the first failure; show each check's own
        // result, so a platform-only difference can be read off the CI log
        let mut each = String::new();
        for x in exprs {
            let shown = x
                .strip_prefix("isTRUE(")
                .and_then(|inner| inner.strip_suffix(')'))
                .unwrap_or(x);
            let (_, got) = run(e, Source::Typed, &format!("print({shown})"));
            each.push_str(&format!("\n{x}\n  -> {}", got.trim()));
        }
        assert_eq!(
            outcome,
            Outcome::Ok,
            "{all}\n{text}\nchecks one by one:{each}"
        );
    }
}

fn define(e: &mut Engine, code: &str) {
    let (outcome, text) = run(e, Source::Typed, code);
    assert_eq!(outcome, Outcome::Ok, "{code}\n{text}");
}

fn within(a: &str, b: &str, tol: &str) -> String {
    format!("isTRUE(all.equal(unname(as.vector({a})), unname(as.vector({b})), tolerance = {tol}))")
}

/// Helpers reading a fit: its estimates, errors and standardised ones by
/// "lhs op rhs", and Stata's fit indices as the dialog prints them.
const READ: &str = "est <- function(f, k) { p <- lavaan::parameterEstimates(f); \
       setNames(p$est, paste(p$lhs, p$op, p$rhs))[k] }; \
     ses <- function(f, k) { p <- lavaan::parameterEstimates(f); \
       setNames(p$se, paste(p$lhs, p$op, p$rhs))[k] }; \
     std <- function(f, k, what = 'est.std') { p <- lavaan::standardizedSolution(f); \
       setNames(p[[what]], paste(p$lhs, p$op, p$rhs))[k] }; \
     gof <- function(f) { m <- lavaan::fitMeasures(f); n <- lavaan::lavInspect(f, 'ntotal'); \
       c(chisq = m[['chisq']], df = m[['df']], logl = m[['logl']], aic = m[['aic']], bic = m[['bic']], \
         rmsea = if (m[['df']] > 0) sqrt(max(m[['chisq']] - m[['df']], 0) / (m[['df']] * (n - 1))) else 0, \
         cfi = m[['cfi']], tli = m[['tli']], srmr = m[['srmr_bentler_nomean']], \
         chisq_bs = m[['baseline.chisq']], n = n) }";

/// Checks of one fit against Stata: `b` and `se` pair parameter names with
/// Stata's values, `stdz` the standardised estimates and their errors,
/// `exact` and `loose` the fit indices by name (to 1e-9 and 1e-5).
fn stata_checks(
    fit: &str,
    b: &[(&str, &str)],
    se: &[(&str, &str)],
    stdz: &[(&str, &str, &str)],
    exact: &[(&str, &str)],
    loose: &[(&str, &str)],
) -> Vec<String> {
    let names = |pairs: &[(&str, &str)]| -> (String, String) {
        let k: Vec<String> = pairs.iter().map(|(n, _)| format!("'{n}'")).collect();
        let v: Vec<&str> = pairs.iter().map(|(_, v)| *v).collect();
        (
            format!("c({})", k.join(", ")),
            format!("c({})", v.join(", ")),
        )
    };
    let mut out = Vec::new();
    let (k, v) = names(b);
    out.push(within(&format!("est({fit}, {k})"), &v, "5e-6"));
    let (k, v) = names(se);
    out.push(within(&format!("ses({fit}, {k})"), &v, "5e-6"));
    if !stdz.is_empty() {
        let k: Vec<String> = stdz.iter().map(|(n, _, _)| format!("'{n}'")).collect();
        let k = format!("c({})", k.join(", "));
        let bs: Vec<&str> = stdz.iter().map(|(_, b, _)| *b).collect();
        let ss: Vec<&str> = stdz.iter().map(|(_, _, s)| *s).collect();
        out.push(within(
            &format!("std({fit}, {k})"),
            &format!("c({})", bs.join(", ")),
            "5e-6",
        ));
        out.push(within(
            &format!("std({fit}, {k}, 'se')"),
            &format!("c({})", ss.join(", ")),
            "5e-6",
        ));
    }
    for (name, value) in exact {
        out.push(within(&format!("gof({fit})[['{name}']]"), value, "1e-9"));
    }
    for (name, value) in loose {
        out.push(within(&format!("gof({fit})[['{name}']]"), value, "1e-5"));
    }
    out
}

#[test]
fn one_factor_and_its_outcome_match_stata_sem() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    // sem (L -> x1 x2 x3) (y <- L), built with the helper
    let (outcome, text) = dialog(
        &mut e,
        &SEM,
        "d",
        &[("save", "f1")],
        &[("indicators", &["x1", "x2", "x3"]), ("outcomes", &["y"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(text.contains("Std.all"), "{text}");
    assert!(text.contains("Fit indices as Stata's estat gof"), "{text}");
    define(&mut e, READ);
    let checks = stata_checks(
        "f1",
        &[
            ("L =~ x2", ".653030921294287081"),
            ("L =~ x3", "1.12684563204958899"),
            ("y ~ L", ".592016344019537288"),
            ("x1 ~1 ", "-.075139836666666668"),
            ("x2 ~1 ", "-.019737480000000002"),
            ("x3 ~1 ", ".083553816666666669"),
            ("y ~1 ", "-.023859076666666666"),
            ("x1 ~~ x1", ".344331757867178068"),
            ("x2 ~~ x2", ".463045128684164808"),
            ("x3 ~~ x3", ".720410635668188193"),
            ("y ~~ y", "1.16169428769801564"),
            ("L ~~ L", ".948822194791006712"),
        ],
        &[
            ("L =~ x2", ".0570567666"),
            ("L =~ x3", ".0869370897"),
            ("y ~ L", ".0752874706"),
            ("x1 ~1 ", ".0656544985"),
            ("x2 ~1 ", ".0537794809"),
            ("x3 ~1 ", ".0801084079"),
            ("y ~1 ", ".0705747989"),
            ("x1 ~~ x1", ".0621635948"),
            ("x2 ~~ x2", ".0458122349"),
            ("x3 ~~ x3", ".0922584169"),
            ("y ~~ y", ".1000696885"),
            ("L ~~ L", ".1158955157"),
        ],
        &[
            ("L =~ x1", ".856578760920378657", ".029370936781792138"),
            ("L =~ x2", ".682887133594689333", ".037762015915567104"),
            ("L =~ x3", ".791075829352995474", ".031799880542484379"),
            ("y ~ L", ".471754267730144561", ".050263788171960463"),
            ("x1 ~1 ", "-.066076210931820076", ".057798011276686247"),
            ("x2 ~1 ", "-.021189195587644274", ".057741507049389872"),
            ("x3 ~1 ", ".060218171602508218", ".057787343303117467"),
            ("y ~1 ", "-.019518361450569816", ".057740525431487701"),
            ("x1 ~~ x1", ".266272826340108793", ".050317041271236568"),
            ("x2 ~~ x2", ".533665162770829094", ".051574389614677321"),
            ("x3 ~~ x3", ".374199032213470229", ".050312233746944011"),
            ("y ~~ y", ".777447910878394932", ".047424313164812618"),
        ],
        &[
            ("chisq", ".827229125159192336"),
            ("df", "2"),
            ("logl", "-1683.39407871770163"),
            ("rmsea", "0"),
            ("cfi", "1"),
            ("tli", "1.00913777690117246"),
            ("chisq_bs", "391.029385437393557"),
            ("n", "300"),
        ],
        &[("srmr", ".008181968453766394")],
    );
    holds(&mut e, &checks);
}

#[test]
fn two_factors_with_an_observed_predictor_match_stata_sem() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    // sem (L1 -> x1 x2 x3) (L2 -> w1 w2 w3) (L2 <- L1 z), typed: Stata lets
    // L1 and z covary, which lavaan's defaults would not
    let (outcome, text) = dialog(
        &mut e,
        &SEM,
        "d",
        &[
            (
                "model",
                "L1 =~ x1 + x2 + x3\nL2 =~ w1 + w2 + w3\nL2 ~ L1 + z",
            ),
            ("save", "f2"),
        ],
        &[],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    define(&mut e, READ);
    let checks = stata_checks(
        "f2",
        &[
            ("L2 ~ z", ".48664845154484726"),
            ("L2 ~ L1", ".559789765382546989"),
            ("x1 ~1 ", "-.075139836666601165"),
            ("L1 =~ x2", ".673251270826549786"),
            ("x2 ~1 ", "-.019737479999959194"),
            ("L1 =~ x3", "1.1957480401353251"),
            ("x3 ~1 ", ".083553816666744163"),
            ("w1 ~1 ", ".108041427063709866"),
            ("L2 =~ w2", ".689284258953722584"),
            ("w2 ~1 ", ".079554532576891254"),
            ("L2 =~ w3", "1.14379249358454871"),
            ("w3 ~1 ", ".206451050781990192"),
            ("x1 ~~ x1", ".39758055860297975"),
            ("x2 ~~ x2", ".461735906663844231"),
            ("x3 ~~ x3", ".64470421333344885"),
            ("w1 ~~ w1", ".255024443036739157"),
            ("w2 ~~ w2", ".366929052059867145"),
            ("w3 ~~ w3", ".72328820259704496"),
            ("L2 ~~ L2", "1.34921513428735862"),
            ("L1 ~~ L1", ".895573614795105777"),
            ("L1 ~~ z", ".383580060851922144"),
        ],
        &[
            ("L2 ~ z", ".0722930124"),
            ("L2 ~ L1", ".0954621667"),
            ("x1 ~1 ", ".0624769519"),
            ("L1 =~ x2", ".0574584105"),
            ("x2 ~1 ", ".0520354542"),
            ("L1 =~ x3", ".0909678858"),
            ("x3 ~1 ", ".0763885942"),
            ("w1 ~1 ", ".0785925656"),
            ("L2 =~ w2", ".0307227392"),
            ("w2 ~1 ", ".0612718685"),
            ("L2 =~ w3", ".0462933019"),
            ("w3 ~1 ", ".0968501214"),
            ("x1 ~~ x1", ".0607937011"),
            ("x2 ~~ x2", ".0454181934"),
            ("x3 ~~ x3", ".0910703212"),
            ("w1 ~~ w1", ".0506287126"),
            ("w2 ~~ w2", ".0379851527"),
            ("w3 ~~ w3", ".0847228934"),
            ("L2 ~~ L2", ".1354006369"),
            ("L1 ~~ L1", ".1124149816"),
            ("L1 ~~ z", ".0634036105"),
        ],
        &[
            ("L2 ~ z", ".366476560723788203", ".050252940171744601"),
            ("L2 ~ L1", ".363483991392970773", ".055975415600346037"),
            ("L1 =~ x1", ".832195747302919675", ".029721961800231164"),
            ("L2 =~ w1", ".944885541388280292", ".011940667135022068"),
            ("L2 ~~ L2", ".635185510690273181", ".047617004516032485"),
            ("L1 ~~ z", ".369304169432803642", ".053791598750147503"),
        ],
        &[
            ("chisq", "11.8753305078262201"),
            ("df", "12"),
            ("logl", "-2893.97417578087516"),
            ("aic", "5829.94835156175031"),
            ("bic", "5907.72778352953083"),
            ("rmsea", "0"),
            ("cfi", "1"),
            ("tli", "1.00018470519505076"),
            ("chisq_bs", "1202.18827813291046"),
            ("n", "300"),
        ],
        &[("srmr", ".016756930939046361")],
    );
    holds(&mut e, &checks);
}

#[test]
fn missing_values_and_a_path_model_match_stata_sem() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    // sem (L1 -> x1 x2 x3) (y <- L1 z w1) on the data with missing values,
    // built with the helper: rows missing x2 or z are left out (279 used)
    let (outcome, text) = dialog(
        &mut e,
        &SEM,
        "dn",
        &[("latent", "L1"), ("save", "f3")],
        &[
            ("indicators", &["x1", "x2", "x3"]),
            ("outcomes", &["y"]),
            ("predictors", &["z", "w1"]),
        ],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    define(&mut e, READ);
    let checks = stata_checks(
        "f3",
        &[
            ("y ~ z", ".318448480278568258"),
            ("y ~ w1", "-.063460914914040087"),
            ("y ~ L1", ".507334827410824474"),
            ("y ~1 ", "-.046967053075253895"),
            ("x1 ~1 ", "-.106584440860950064"),
            ("L1 =~ x2", ".64696904791556864"),
            ("x2 ~1 ", "-.038579053762283926"),
            ("L1 =~ x3", "1.17654602885232706"),
            ("x3 ~1 ", ".067344724011169074"),
            ("x1 ~~ x1", ".362430845992336104"),
            ("x2 ~~ x2", ".449594738340484468"),
            ("x3 ~~ x3", ".728062307099241468"),
            ("y ~~ y", "1.13169577976697955"),
            ("L1 ~~ L1", ".897021113395942593"),
            ("L1 ~~ z", ".415448394364556273"),
            ("L1 ~~ w1", ".700292170161259553"),
        ],
        &[
            ("y ~ z", ".0701737675"),
            ("y ~ w1", ".0523943858"),
            ("y ~ L1", ".0931782466"),
            ("y ~1 ", ".0686413362"),
            ("x1 ~1 ", ".0604452091"),
            ("L1 =~ x2", ".0582577597"),
            ("x2 ~1 ", ".0509608488"),
            ("L1 =~ x3", ".0908564375"),
            ("x3 ~1 ", ".0766087831"),
            ("x1 ~~ x1", ".0591919055"),
            ("x2 ~~ x2", ".0453067281"),
            ("x3 ~~ x3", ".0941868872"),
            ("y ~~ y", ".0997995212"),
            ("L1 ~~ L1", ".1121515641"),
            ("L1 ~~ z", ".0613082235"),
            ("L1 ~~ w1", ".0871216616"),
        ],
        &[
            ("y ~ z", ".279244685206597898", ".059282175167220397"),
            ("y ~ w1", "-.078688030948876039", ".064799613891439414"),
            ("y ~ L1", ".386031201207115426", ".065408905985524013"),
            ("y ~~ y", ".73043654592759355", ".047833819458432486"),
            ("L1 ~~ z", ".401880020463000531", ".05168063118612215"),
            ("L1 ~~ w1", ".479073322989834127", ".04908161772520777"),
        ],
        &[
            ("chisq", "3.43146926992722001"),
            ("df", "6"),
            ("logl", "-2423.36092959099506"),
            ("aic", "4878.72185918199011"),
            ("bic", "4936.82124769113216"),
            ("rmsea", "0"),
            ("cfi", "1"),
            ("tli", "1.01385061546205013"),
            ("chisq_bs", "446.705563632960548"),
            ("n", "279"),
        ],
        &[("srmr", ".012215833578977538")],
    );
    holds(&mut e, &checks);

    // sem (y <- z w1) (w2 <- y): no latent variable, a poor fit, so RMSEA (on
    // N - 1, as Stata), CFI and TLI are away from their bounds
    let (outcome, text) = dialog(
        &mut e,
        &SEM,
        "dn",
        &[("model", "y ~ z + w1\nw2 ~ y"), ("save", "f4")],
        &[],
        &["standardized"],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(!text.contains("Std.all"), "{text}");
    assert!(text.contains("0.7047387"), "{text}");
    let checks = stata_checks(
        "f4",
        &[
            ("y ~ z", ".403748371730123123"),
            ("y ~ w1", ".051013383758939712"),
            ("y ~1 ", "-.050355868025343371"),
            ("w2 ~ y", ".198280792478964019"),
            ("w2 ~1 ", ".099210024172747144"),
            ("y ~~ y", "1.29164103144980857"),
            ("w2 ~~ w2", "1.28442229454147783"),
        ],
        &[
            ("y ~ z", ".0697308147"),
            ("y ~ w1", ".0492361182"),
            ("y ~1 ", ".0669118841"),
            ("w2 ~ y", ".0538196852"),
            ("w2 ~1 ", ".0664423918"),
            ("y ~~ y", ".1070804866"),
            ("w2 ~~ w2", ".1064820341"),
        ],
        &[],
        &[
            ("chisq", "290.060884373014233"),
            ("df", "2"),
            ("logl", "-1837.84528178217965"),
            ("aic", "3689.69056356435931"),
            ("bic", "3715.40382643455996"),
            ("rmsea", ".70473874392916136"),
            ("cfi", ".168488612870842713"),
            ("tli", "-1.07877846782289333"),
            ("chisq_bs", "351.430474473189861"),
            ("n", "291"),
        ],
        &[("srmr", ".263347572512613537")],
    );
    holds(&mut e, &checks);
}

#[test]
fn lavaan_defaults_are_lavaan_sem_by_hand_and_the_model_menu_works() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    let model = "L1 =~ x1 + x2 + x3\nL2 =~ w1 + w2 + w3\nL2 ~ L1 + z";
    let (outcome, text) = dialog(
        &mut e,
        &SEM,
        "d",
        &[("model", model), ("save", "own")],
        &[],
        &["stata"],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(text.contains("Fit indices"), "{text}");
    define(
        &mut e,
        "hand <- lavaan::sem('L1 =~ x1 + x2 + x3\\nL2 =~ w1 + w2 + w3\\nL2 ~ L1 + z', data = d)",
    );
    holds(
        &mut e,
        &[
            within("lavaan::coef(own)", "lavaan::coef(hand)", "1e-12"),
            within(
                "lavaan::parameterEstimates(own)$se",
                "lavaan::parameterEstimates(hand)$se",
                "1e-12",
            ),
            within(
                "lavaan::fitMeasures(own, 'chisq')",
                "lavaan::fitMeasures(hand, 'chisq')",
                "1e-12",
            ),
            // lavaan's own default leaves L1 and z uncorrelated, so it fits
            // worse than Stata's model (chi-squared 11.88 on 12)
            "lavaan::fitMeasures(own, 'df') == 13".to_string(),
        ],
    );

    // the Model menu: Model fit and Compare models on a lavaan fit
    let entries: Vec<&str> = postest::model_specs("lavaan")
        .iter()
        .map(|s| s.entry)
        .collect();
    assert_eq!(entries, ["Model fit", "Compare models"]);
    assert!(postest::is_model_class("lavaan") && !postest::takes_model_table("lavaan"));
    let (outcome, text) = run(&mut e, Source::Typed, "cat(class(own)[1])");
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert_eq!(text.trim(), "lavaan");
    // Stata's fit and the same model without z's path (nested, same variables)
    for (save, typed) in [
        ("full", model),
        (
            "nested",
            "L1 =~ x1 + x2 + x3\nL2 =~ w1 + w2 + w3\nL2 ~ L1 + 0*z",
        ),
    ] {
        let (outcome, text) = dialog(
            &mut e,
            &SEM,
            "d",
            &[("model", typed), ("save", save)],
            &[],
            &[],
        );
        assert_eq!(outcome, Outcome::Ok, "{text}");
    }
    let ctx = |name: &str| Context {
        dataset: name.into(),
        frames: Vec::new(),
        columns: Vec::new(),
    };
    let fit_code = MODEL_FIT.generate(&ctx("full"), &MODEL_FIT.defaults(&ctx("full")), Style::Base);
    let fit_code = fit_code.unwrap_or_default();
    let (outcome, text) = run(&mut e, Source::Menu(MODEL_FIT.menu.into()), &fit_code);
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(text.contains("-2893.974"), "{text}");
    assert!(text.contains("5829.948"), "{text}");
    let mut values = COMPARE_MODELS.defaults(&ctx("full"));
    values.set_text("others", "nested");
    let compare = COMPARE_MODELS.generate(&ctx("full"), &values, Style::Base);
    let compare = compare.unwrap_or_default();
    let (outcome, text) = run(&mut e, Source::Menu(COMPARE_MODELS.menu.into()), &compare);
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(text.contains("Chi-Squared Difference Test"), "{text}");
    assert!(text.contains("nested") && text.contains("full"), "{text}");
    // the difference is twice the log-likelihoods' difference, on one df
    holds(
        &mut e,
        &[
            within(
                "lavaan::lavTestLRT(full, nested)[['Chisq diff']][2]",
                "2 * (lavaan::fitMeasures(full, 'logl') - lavaan::fitMeasures(nested, 'logl'))",
                "1e-9",
            ),
            "lavaan::lavTestLRT(full, nested)[['Df diff']][2] == 1".to_string(),
        ],
    );
}
