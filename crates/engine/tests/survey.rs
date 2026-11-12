//! Statistics menu survey data (P6.4) end to end against a real R (skipped
//! with a note when none is found, or when survey is not installed). Each
//! dialog's code is built from its spec and run on made-up data with three
//! strata of four PSUs, unequal weights and missing values; its estimates,
//! standard errors and intervals are checked three ways:
//!
//! - against the formulas of Stata's [SVY] manual ("Variance estimation":
//!   formula (1) for a total, the score variables of a mean and a ratio, the
//!   sandwich D V{G} D' for a regression, t on PSUs less strata), written out
//!   here in plain R with no survey package;
//! - against Stata 19's own results for the same data and design, recorded
//!   below (`svy: mean`, `total`, `mean, over()`, `proportion`, `regress`,
//!   `logit`, `tabulate`'s design-based Pearson F, and `fpc()`), to 1e-9;
//! - and against survey's report for the call written out by hand.
//!
//! The test records the R and package versions it ran with (AGENTS.md
//! section 5), and checks R is the one `ENVIRONMENT` pins.

use std::time::{Duration, Instant};

use rcraft_catalog::survey::{SURVEY_MEANS, SURVEY_REGRESSION, SURVEY_TABLES};
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

/// The R and survey versions, recorded; false when survey is not installed.
fn versions(e: &mut Engine) -> bool {
    let (outcome, text) = run(
        e,
        Source::Typed,
        "cat(paste(R.version$major, R.version$minor, sep = '.'), \
         if (requireNamespace('survey', quietly = TRUE)) as.character(packageVersion('survey')) else 'none')",
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let mut words = text.split_whitespace();
    let (r, survey) = (words.next().unwrap_or(""), words.next().unwrap_or(""));
    if survey == "none" {
        eprintln!("skipped: survey is not installed");
        return false;
    }
    let pinned = include_str!("../../../ENVIRONMENT")
        .lines()
        .find_map(|l| l.strip_prefix("R_VERSION="))
        .unwrap_or("")
        .trim();
    eprintln!("ran with R {r} and survey {survey} (ENVIRONMENT pins R {pinned})");
    if r != pinned {
        eprintln!("note: this R is not the pinned one, so the run is not a Verified one");
    }
    true
}

/// Three strata of four PSUs (numbered 1 to 4 in each), five rows each,
/// unequal weights, three incomes missing.
const MAKE: &str = "d <- local({ \
     r <- 1:60; stratum <- rep(1:3, each = 20); psu <- rep(rep(1:4, each = 5), 3); \
     wt <- round(10 + 5 * stratum + 3 * psu + (r %% 4), 2); \
     hours <- round(20 + 2 * psu + 3 * stratum + 4 * sin(r), 2); \
     income <- round(100 + 6 * hours + 15 * stratum + 10 * cos(r * 1.3), 1); \
     income[c(4, 27, 51)] <- NA; \
     region <- factor(c('north', 'south', 'west')[1 + (r * 7) %% 3]); \
     smokes <- as.numeric(((r * 11) %% 5) < 2); \
     pop <- c(20, 30, 40)[stratum]; \
     data.frame(income, hours, region, smokes, wt, psu, stratum, pop) })";

/// The [SVY] manual's formulas in plain R: `vtotal(z, fpc)` is formula (1)
/// for the total of the weighted scores `z` (one column per estimate).
const ORACLE: &str = "vtotal <- function(z, fpc = NULL) { \
       z <- as.matrix(z); key <- paste(d$stratum, d$psu); out <- matrix(0, ncol(z), ncol(z)); \
       for (h in unique(d$stratum)) { rows <- d$stratum == h; \
         t <- rowsum(z[rows, , drop = FALSE], key[rows]); n <- nrow(t); \
         f <- if (is.null(fpc)) 0 else n / fpc[rows][1]; c <- sweep(t, 2, colMeans(t)); \
         out <- out + (1 - f) * n / (n - 1) * crossprod(c) }; out }; \
     ok <- !is.na(d$income) & !is.na(d$hours); w <- d$wt; \
     mean_of <- function(x, keep = ok, dom = rep(TRUE, nrow(d)), fpc = NULL) { k <- keep & dom; \
       m <- sum(w[k] * x[k]) / sum(w[k]); z <- ifelse(k, w * (x - m) / sum(w[k]), 0); \
       c(m, sqrt(vtotal(z, fpc))) }; \
     total_of <- function(x, keep = ok) { z <- ifelse(keep, w * x, 0); c(sum(z), sqrt(vtotal(z))) }";

fn context() -> Context {
    Context {
        dataset: "d".into(),
        frames: Vec::new(),
        columns: [
            ("income", VarKind::Number),
            ("hours", VarKind::Number),
            ("region", VarKind::Factor),
            ("smokes", VarKind::Number),
            ("wt", VarKind::Number),
            ("psu", VarKind::Number),
            ("stratum", VarKind::Number),
            ("pop", VarKind::Number),
        ]
        .into_iter()
        .map(|(name, kind)| ColumnInfo {
            name: name.into(),
            kind,
        })
        .collect(),
    }
}

fn dialog(
    e: &mut Engine,
    spec: &Spec,
    set: &[(&str, &str)],
    lists: &[(&str, &[&str])],
    flags: &[&str],
) -> (Outcome, String) {
    let (made, text) = run(e, Source::Typed, MAKE);
    assert_eq!(made, Outcome::Ok, "{text}");
    let ctx = context();
    let mut values = spec.defaults(&ctx);
    for (id, text) in set {
        values.set_text(id, *text);
    }
    for (id, names) in lists {
        values.set_list(id, names.iter().map(|n| n.to_string()).collect());
    }
    for w in spec.widgets {
        if let Widget::Check { id, .. } = w {
            values.set_flag(id, flags.contains(id));
        }
    }
    let code = spec.generate(&ctx, &values, Style::Base);
    assert!(code.is_ok(), "{code:?}");
    let code = code.unwrap_or_default();
    run(e, Source::Menu(spec.menu.into()), &code)
}

fn report(text: &str) -> Vec<String> {
    text.lines()
        .filter(|l| {
            let l = l.trim_start();
            !(l.starts_with("Call:")
                || l.starts_with("survey::")
                || l.starts_with("svyglm(")
                || l.starts_with("data:")
                || l.starts_with("warning:"))
        })
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|l| !l.is_empty())
        .collect()
}

/// Runs R that stops unless every one of `exprs` holds, with the oracle defined.
fn holds(e: &mut Engine, exprs: &str) {
    let code = format!("local({{ {ORACLE}; stopifnot({exprs}) }})");
    let (outcome, text) = run(e, Source::Typed, &code);
    assert_eq!(outcome, Outcome::Ok, "{exprs}\n{text}");
}

/// `all.equal` to 1e-9, unnamed.
fn same(a: &str, b: &str) -> String {
    format!("isTRUE(all.equal(unname(as.vector({a})), unname(as.vector({b})), tolerance = 1e-9))")
}

#[test]
fn means_and_totals_are_the_manuals_and_statas() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    let design = [("weights", "wt"), ("psu", "psu"), ("strata", "stratum")];
    let (outcome, text) = dialog(
        &mut e,
        &SURVEY_MEANS,
        &design,
        &[("x", &["income", "hours"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let (_, expected) = run(
        &mut e,
        Source::Typed,
        "design <- survey::svydesign(ids = ~psu, strata = ~stratum, weights = ~wt, nest = TRUE, data = d); \
         print(local({ est <- survey::svymean(~income + hours, design, na.rm = TRUE); \
         cbind(est = stats::coef(est), se = survey::SE(est), \
         stats::confint(est, level = 0.95, df = survey::degf(design))) }))",
    );
    assert_eq!(report(&text), report(&expected), "{text}");
    // the dialog's numbers, from its own design object
    let est = "local({ est <- survey::svymean(~income + hours, design, na.rm = TRUE); \
               list(b = coef(est), se = survey::SE(est), \
                    ci = confint(est, df = survey::degf(design))) })";
    holds(
        &mut e,
        &[
            same(&format!("{est}$b[1]"), "mean_of(d$income)[1]"),
            same(&format!("{est}$se[1]"), "mean_of(d$income)[2]"),
            same(&format!("{est}$se[2]"), "mean_of(d$hours)[2]"),
            // Stata 19: svy: mean income hours, and its interval on 9 df
            same(&format!("{est}$b"), "c(321.811714975845, 31.6672041062802)"),
            same(
                &format!("{est}$se"),
                "c(5.74996648809755, .933873204758025)",
            ),
            same(
                &format!("{est}$ci[1, ]"),
                "c(308.804387098946, 334.819042852745)",
            ),
            "survey::degf(design) == 9".to_string(),
        ]
        .join(", "),
    );

    // totals
    let (outcome, text) = dialog(
        &mut e,
        &SURVEY_MEANS,
        &[
            ("weights", "wt"),
            ("psu", "psu"),
            ("strata", "stratum"),
            ("stat", "svytotal"),
        ],
        &[("x", &["income"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let tot = "survey::svytotal(~income, design, na.rm = TRUE)";
    holds(
        &mut e,
        &[
            same(
                &format!("coef({tot})"),
                "total_of(d$income, keep = !is.na(d$income))[1]",
            ),
            same(
                &format!("survey::SE({tot})"),
                "total_of(d$income, keep = !is.na(d$income))[2]",
            ),
            // Stata 19: svy: total income
            same(
                &format!("c(coef({tot}), survey::SE({tot}))"),
                "c(532920.2, 35136.334503284)",
            ),
        ]
        .join(", "),
    );

    // a mean for each region (domains)
    let (outcome, text) = dialog(
        &mut e,
        &SURVEY_MEANS,
        &[
            ("weights", "wt"),
            ("psu", "psu"),
            ("strata", "stratum"),
            ("by", "region"),
        ],
        &[("x", &["income"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let by = "survey::svyby(~income, ~region, design, survey::svymean, na.rm = TRUE)";
    holds(
        &mut e,
        &[
            same(
                &format!("coef({by})"),
                "vapply(levels(d$region), function(g) mean_of(d$income, keep = !is.na(d$income), dom = d$region == g)[1], 0)",
            ),
            same(
                &format!("survey::SE({by})"),
                "vapply(levels(d$region), function(g) mean_of(d$income, keep = !is.na(d$income), dom = d$region == g)[2], 0)",
            ),
            // Stata 19: svy: mean income, over(region)
            same(&format!("coef({by})"), "c(319.404624277457, 323.810951526032, 322.045689655172)"),
            same(&format!("survey::SE({by})"), "c(7.09210480782385, 6.14323400240301, 5.86314238490123)"),
        ]
        .join(", "),
    );

    // a finite-population correction
    let (outcome, text) = dialog(
        &mut e,
        &SURVEY_MEANS,
        &[
            ("weights", "wt"),
            ("psu", "psu"),
            ("strata", "stratum"),
            ("fpc", "pop"),
        ],
        &[("x", &["income"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let fpc = "survey::svymean(~income, design, na.rm = TRUE)";
    holds(
        &mut e,
        &[
            same(
                &format!("survey::SE({fpc})"),
                "mean_of(d$income, keep = !is.na(d$income), fpc = d$pop)[2]",
            ),
            // Stata 19: svyset ..., fpc(pop); svy: mean income
            same(
                &format!("c(coef({fpc}), survey::SE({fpc}))"),
                "c(321.811714975845, 5.42274683389105)",
            ),
        ]
        .join(", "),
    );
}

#[test]
fn proportions_and_the_rao_scott_test_are_statas() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    let (outcome, text) = dialog(
        &mut e,
        &SURVEY_TABLES,
        &[
            ("row", "region"),
            ("col", "smokes"),
            ("weights", "wt"),
            ("psu", "psu"),
            ("strata", "stratum"),
        ],
        &[],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(text.contains("Rao & Scott"), "{text}");
    let p = "survey::svymean(~factor(region), design, na.rm = TRUE)";
    let chi = "survey::svychisq(~region + smokes, design)";
    holds(
        &mut e,
        &[
            same(
                &format!("coef({p})"),
                "vapply(levels(d$region), function(g) mean_of(as.numeric(d$region == g), keep = rep(TRUE, 60))[1], 0)",
            ),
            same(
                &format!("survey::SE({p})"),
                "vapply(levels(d$region), function(g) mean_of(as.numeric(d$region == g), keep = rep(TRUE, 60))[2], 0)",
            ),
            // Stata 19: svy: proportion region (north and west)
            same(&format!("coef({p})[c(1, 3)]"), "c(.336206896551724, .333333333333333)"),
            same(&format!("survey::SE({p})[c(1, 3)]"), "c(.0309655843537782, .0313287219873711)"),
            // Stata 19: svy: tabulate region smokes, the design-based Pearson F
            same(&format!("{chi}$statistic"), "0.0056849296588214"),
            same(&format!("{chi}$parameter"), "c(1.993012575533518, 17.93711317980166)"),
            same(&format!("{chi}$p.value"), "0.9942423084959018"),
            // the table is the weights summed in each cell
            same(
                "survey::svytable(~region + smokes, design)",
                "tapply(d$wt, list(d$region, d$smokes), sum)",
            ),
        ]
        .join(", "),
    );
}

#[test]
fn regressions_are_the_sandwich_and_statas() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    let (outcome, text) = dialog(
        &mut e,
        &SURVEY_REGRESSION,
        &[
            ("y", "income"),
            ("weights", "wt"),
            ("psu", "psu"),
            ("strata", "stratum"),
        ],
        &[("x", &["hours", "region"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let (_, expected) = run(
        &mut e,
        Source::Typed,
        "design <- survey::svydesign(ids = ~psu, strata = ~stratum, weights = ~wt, nest = TRUE, data = d); \
         m <- survey::svyglm(income ~ hours + region, design = design, family = stats::gaussian()); \
         print(summary(m)); print(stats::confint(m, level = 0.95, df = survey::degf(design)))",
    );
    assert_eq!(report(&text), report(&expected), "{text}");
    // D V{G} D', D = (X'WX)^-1, the scores w e x, over the complete rows
    let lin = "local({ X <- model.matrix(~ hours + region, d)[ok, ]; y <- d$income[ok]; wk <- w[ok]; \
               D <- solve(crossprod(X, wk * X)); b <- D %*% crossprod(X, wk * y); e <- y - X %*% b; \
               s <- matrix(0, nrow(d), ncol(X)); s[ok, ] <- wk * as.vector(e) * X; \
               list(b = drop(b), V = D %*% vtotal(s) %*% D) })";
    holds(
        &mut e,
        &[
            same("coef(model)", &format!("{lin}$b")),
            same("vcov(model)", &format!("{lin}$V")),
            // Stata 19: svy: regress income hours i.region (hours, _cons, south)
            same(
                "coef(model)[c('hours', '(Intercept)')]",
                "c(7.54805280044545, 83.7836481907653)",
            ),
            same(
                "survey::SE(model)[c('hours', '(Intercept)')]",
                "c(.371265218364744, 12.8175926236954)",
            ),
            same("coef(model)[['regionsouth']]", "-1.55835774561859"),
            same("survey::SE(model)[['regionsouth']]", "1.43112623721216"),
        ]
        .join(", "),
    );

    // logistic: the pseudolikelihood's sandwich, and Stata's svy: logit
    let (outcome, text) = dialog(
        &mut e,
        &SURVEY_REGRESSION,
        &[
            ("y", "smokes"),
            ("family", "quasibinomial"),
            ("weights", "wt"),
            ("psu", "psu"),
            ("strata", "stratum"),
            ("save", "lg"),
        ],
        &[("x", &["hours"])],
        &["ratios"],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(text.contains("Odds ratios"), "{text}");
    let logit = "local({ X <- model.matrix(~ hours, d); y <- d$smokes; b <- coef(lg); \
                 p <- plogis(X %*% b); D <- solve(crossprod(X, as.vector(w * p * (1 - p)) * X)); \
                 s <- as.vector(w * (y - p)) * X; D %*% vtotal(s) %*% D })";
    // everything to 1e-9: with glm's default epsilon (1e-8) svyglm stopped about
    // 2e-8 short of Stata's standard errors; the dialog's tighter control closes it
    holds(
        &mut e,
        &[
            same("sqrt(diag(vcov(lg)))", &format!("sqrt(diag({logit}))")),
            // Stata 19: svy: logit smokes hours (hours, _cons)
            same(
                "coef(lg)[c('hours', '(Intercept)')]",
                "c(-.031503817328669, .592219489416413)",
            ),
            same(
                "survey::SE(lg)[c('hours', '(Intercept)')]",
                "c(.0522228042969489, 1.67673976304993)",
            ),
            same(
                &format!("sqrt(diag({logit}))[c(2, 1)]"),
                "c(.0522228042969489, 1.67673976304993)",
            ),
        ]
        .join(", "),
    );

    // Poisson: the same sandwich with the log link, and Stata's svy: poisson
    let (outcome, text) = dialog(
        &mut e,
        &SURVEY_REGRESSION,
        &[
            ("y", "smokes"),
            ("family", "quasipoisson"),
            ("weights", "wt"),
            ("psu", "psu"),
            ("strata", "stratum"),
            ("save", "po"),
        ],
        &[("x", &["hours"])],
        &["ratios"],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(text.contains("Rate ratios"), "{text}");
    let poisson = "local({ X <- model.matrix(~ hours, d); y <- d$smokes; b <- coef(po); \
                   mu <- exp(X %*% b); D <- solve(crossprod(X, as.vector(w * mu) * X)); \
                   s <- as.vector(w * (y - mu)) * X; D %*% vtotal(s) %*% D })";
    holds(
        &mut e,
        &[
            same("vcov(po)", poisson),
            // the estimates solve the weighted score equations
            "isTRUE(max(abs(crossprod(model.matrix(~ hours, d), w * (d$smokes - exp(model.matrix(~ hours, d) %*% coef(po)))))) < 1e-9)".to_string(),
            // Stata 19: svy: poisson smokes hours (hours, _cons)
            same(
                "coef(po)[c('hours', '(Intercept)')]",
                "c(-.0188613008816544, -.321305745732532)",
            ),
            same(
                "survey::SE(po)[c('hours', '(Intercept)')]",
                "c(.0310075070167696, .987806470944052)",
            ),
        ]
        .join(", "),
    );
}
