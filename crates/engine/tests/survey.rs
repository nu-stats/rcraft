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

use rcraft_catalog::postest::{self, MODEL_FIT, PREDICTED, TEST_TERMS};
use rcraft_catalog::survey::{
    SURVEY_MEANS, SURVEY_QUANTILES, SURVEY_RATIOS, SURVEY_REGRESSION, SURVEY_TABLES,
};
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
     ssu <- r; f2 <- 10 + 2 * psu; \
     data.frame(income, hours, region, smokes, wt, psu, stratum, pop, ssu, f2) })";

/// Three strata of two PSUs with six rows each (the shape BRR needs), unequal
/// weights, and replicate weights built here: four BRR half-samples from the
/// Hadamard matrix of order four (its first three columns, as Stata's
/// `svy brr` takes them), 36 delete-one jackknife weights, and 30 bootstrap
/// resamples. The same code built the data Stata 19 ran on.
const MAKE_REP: &str = "d <- local({ \
     k <- 1:36; s <- rep(1:3, each = 12); p <- rep(rep(1:2, each = 6), 3); \
     w <- round(8 + 2 * s + 3 * p + (k %% 3), 2); \
     y <- round(50 + 4 * s - 3 * p + 6 * sin(k * 0.9), 2); \
     x <- round(10 + s + 2 * cos(k * 0.7) + p, 2); \
     H <- kronecker(matrix(c(1, 1, 1, -1), 2, byrow = TRUE), matrix(c(1, 1, 1, -1), 2, byrow = TRUE)); \
     brr <- sapply(1:4, function(r) ifelse(p == ifelse(H[r, 1:3][s] == 1, 1, 2), 2 * w, 0)); \
     colnames(brr) <- paste0('brr', 1:4); \
     jk <- sapply(1:36, function(i) ifelse(k == i, 0, w * 36 / 35)); \
     colnames(jk) <- paste0('jk', 1:36); \
     set.seed(11); \
     bs <- sapply(1:30, function(r) w * tabulate(sample.int(36, 36, replace = TRUE), 36)); \
     colnames(bs) <- paste0('bs', 1:30); \
     data.frame(y, x, w, s, p, brr, jk, bs) })";

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
            ("ssu", VarKind::Number),
            ("f2", VarKind::Number),
        ]
        .into_iter()
        .map(|(name, kind)| ColumnInfo {
            name: name.into(),
            kind,
        })
        .collect(),
    }
}

/// The columns of [`MAKE_REP`].
fn rep_context() -> Context {
    let mut names: Vec<String> = ["y", "x", "w", "s", "p"].map(String::from).to_vec();
    names.extend((1..=4).map(|i| format!("brr{i}")));
    names.extend((1..=36).map(|i| format!("jk{i}")));
    names.extend((1..=30).map(|i| format!("bs{i}")));
    Context {
        dataset: "d".into(),
        frames: Vec::new(),
        columns: names
            .into_iter()
            .map(|name| ColumnInfo {
                name,
                kind: VarKind::Number,
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
    dialog_on(e, MAKE, &context(), spec, set, lists, flags)
}

fn dialog_on(
    e: &mut Engine,
    make: &str,
    ctx: &Context,
    spec: &Spec,
    set: &[(&str, &str)],
    lists: &[(&str, &[&str])],
    flags: &[&str],
) -> (Outcome, String) {
    let (made, text) = run(e, Source::Typed, make);
    assert_eq!(made, Outcome::Ok, "{text}");
    let ctx = ctx.clone();
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

/// The numbers on the first line that starts with the word `name`.
fn row(text: &str, name: &str) -> Vec<f64> {
    text.lines()
        .find_map(|l| {
            let mut words = l.split_whitespace();
            (words.next() == Some(name))
                .then(|| words.filter_map(|t| t.parse::<f64>().ok()).collect())
        })
        .unwrap_or_default()
}

/// `all.equal` to `tol`, unnamed.
fn near(a: &str, b: &str, tol: f64) -> String {
    format!("isTRUE(all.equal(unname(as.vector({a})), unname(as.vector({b})), tolerance = {tol}))")
}

/// The ratio and the weighted-quantile oracles, the replicate variance and the
/// two-stage variance, written out in plain R. They run on `d` of `MAKE` (and
/// the replicate variance on `d` of `MAKE_REP`).
const ORACLE_MORE: &str = "ratio_of <- function(x, y, keep = ok, dom = rep(TRUE, nrow(d)), fpc = NULL) { \
       k <- keep & dom; r <- sum(w[k] * x[k]) / sum(w[k] * y[k]); \
       z <- ifelse(k, w * (x - r * y) / sum(w[k] * y[k]), 0); c(r, sqrt(vtotal(z, fpc))) }; \
     qhand <- function(p) { k <- !is.na(d$income); o <- order(d$income[k]); xs <- d$income[k][o]; \
       cw <- cumsum(w[k][o]) / sum(w[k]); xs[which(cw >= p - 1e-12)[1]] }; \
     ms_se <- function(x, fpc1 = TRUE, fpc2 = TRUE) { k <- !is.na(x); xs <- x[k]; ws <- w[k]; \
       m <- sum(ws * xs) / sum(ws); z <- ws * (xs - m) / sum(ws); key <- paste(d$stratum, d$psu)[k]; \
       st <- d$stratum[k]; out <- 0; \
       for (h in unique(st)) { r <- st == h; t <- tapply(z[r], key[r], sum); n <- length(t); \
         f1 <- if (fpc1) n / d$pop[k][r][1] else 0; \
         out <- out + (1 - f1) * n / (n - 1) * sum((t - mean(t))^2); \
         for (p in unique(key[r])) { rr <- r & key == p; mm <- sum(rr); zz <- z[rr]; \
           f2 <- if (fpc2) mm / d$f2[k][rr][1] else 0; \
           out <- out + f1 * (1 - f2) * mm / (mm - 1) * sum((zz - mean(zz))^2) } }; sqrt(out) }; \
     rep_se <- function(cols, scale, x) { W <- as.matrix(d[cols]); th <- colSums(W * x) / colSums(W); \
       sqrt(scale * sum((th - mean(th))^2)) }";

/// Like `holds`, with the extra oracles defined too.
fn holds_more(e: &mut Engine, exprs: &str) {
    let code = format!("local({{ {ORACLE}; {ORACLE_MORE}; stopifnot({exprs}) }})");
    let (outcome, text) = run(e, Source::Typed, &code);
    assert_eq!(outcome, Outcome::Ok, "{exprs}\n{text}");
}

#[test]
fn survey_ratios_are_statas() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    let design = [("weights", "wt"), ("psu", "psu"), ("strata", "stratum")];
    let mut set = vec![("den", "hours")];
    set.extend(design);
    let (outcome, text) = dialog(
        &mut e,
        &SURVEY_RATIOS,
        &set,
        &[("num", &["income", "smokes"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let (_, expected) = run(
        &mut e,
        Source::Typed,
        "design <- survey::svydesign(ids = ~psu, strata = ~stratum, weights = ~wt, nest = TRUE, data = d); \
         print(local({ est <- survey::svyratio(~income + smokes, ~hours, design, na.rm = TRUE); \
         cbind(ratio = stats::coef(est), se = survey::SE(est), \
         stats::confint(est, level = 0.95, df = survey::degf(design))) }))",
    );
    assert_eq!(report(&text), report(&expected), "{text}");
    // the interval is the estimate plus and minus t(9) standard errors
    let r = row(&text, "income/hours");
    assert_eq!(r.len(), 4, "{text}");
    assert!(
        ((r[3] - r[2]) / 2.0 / r[1] - 2.262157162740992).abs() < 1e-4,
        "{text}"
    );
    let est = "survey::svyratio(~income + smokes, ~hours, design, na.rm = TRUE)";
    holds_more(
        &mut e,
        &[
            same(
                &format!("coef({est})"),
                "c(ratio_of(d$income, d$hours)[1], ratio_of(d$smokes, d$hours)[1])",
            ),
            same(
                &format!("survey::SE({est})"),
                "c(ratio_of(d$income, d$hours)[2], ratio_of(d$smokes, d$hours)[2])",
            ),
            // Stata 19: svy: ratio (income/hours) (smokes/hours), on 9 df
            same(
                &format!("c(coef({est}), survey::SE({est}))"),
                "c(10.1623027374249, .0125665296679747, .120446055701666, .000657480653199729)",
            ),
            "survey::degf(design) == 9".to_string(),
        ]
        .join(", "),
    );

    // a ratio for each group (domains)
    let mut set = vec![("den", "hours"), ("by", "region")];
    set.extend(design);
    let (outcome, text) = dialog(&mut e, &SURVEY_RATIOS, &set, &[("num", &["income"])], &[]);
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let by = "survey::svyby(~income, ~region, design, survey::svyratio, denominator = ~hours, na.rm = TRUE)";
    holds_more(
        &mut e,
        &[
            same(
                &format!("coef({by})"),
                "vapply(levels(d$region), function(g) ratio_of(d$income, d$hours, dom = d$region == g)[1], 0)",
            ),
            same(
                &format!("survey::SE({by})"),
                "vapply(levels(d$region), function(g) ratio_of(d$income, d$hours, dom = d$region == g)[2], 0)",
            ),
            // Stata 19: svy: ratio (income/hours), over(region)
            same(
                &format!("coef({by})"),
                "c(10.232038797198, 10.1170828953406, 10.1447248506152)",
            ),
            same(
                &format!("survey::SE({by})"),
                "c(.155371563684392, .117556453578115, .119524338997218)",
            ),
        ]
        .join(", "),
    );

    // a finite-population correction
    let (outcome, text) = dialog(
        &mut e,
        &SURVEY_RATIOS,
        &[
            ("den", "hours"),
            ("weights", "wt"),
            ("psu", "psu"),
            ("strata", "stratum"),
            ("fpc", "pop"),
        ],
        &[("num", &["income"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let fpc = "survey::svyratio(~income, ~hours, design, na.rm = TRUE)";
    holds_more(
        &mut e,
        &[
            same(
                &format!("survey::SE({fpc})"),
                "ratio_of(d$income, d$hours, fpc = d$pop)[2]",
            ),
            // Stata 19: svyset ..., fpc(pop); svy: ratio (income/hours)
            same(
                &format!("c(coef({fpc}), survey::SE({fpc}))"),
                "c(10.1623027374249, .112989255253847)",
            ),
        ]
        .join(", "),
    );
}

#[test]
fn survey_quantiles_follow_the_weighted_distribution() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    let (outcome, text) = dialog(
        &mut e,
        &SURVEY_QUANTILES,
        &[
            ("y", "income"),
            ("weights", "wt"),
            ("psu", "psu"),
            ("strata", "stratum"),
        ],
        &[],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(text.contains("quantile") && text.contains("se"), "{text}");
    let q = "survey::svyquantile(~income, design, c(.25, .5, .75), ci = TRUE, na.rm = TRUE)$income";
    holds_more(
        &mut e,
        &[
            // the rule: the smallest value whose weighted cumulative share reaches p,
            // worked out by hand on the weighted distribution function
            same(
                &format!("{q}[, 'quantile']"),
                "vapply(c(.25, .5, .75), qhand, 0)",
            ),
            // and the values it gives (recorded; Stata has no command with this rule)
            same(&format!("{q}[, 'quantile']"), "c(295.2, 319.2, 347.6)"),
            // each interval holds its quantile, and se is its half-width over t(9)
            format!("all({q}[, 2] <= {q}[, 1] & {q}[, 1] <= {q}[, 3])"),
            same(
                &format!("{q}[, 'se']"),
                &format!("({q}[, 3] - {q}[, 2]) / (2 * qt(.975, 9))"),
            ),
        ]
        .join(", "),
    );
    // a 90% interval for the median only
    let (outcome, text) = dialog(
        &mut e,
        &SURVEY_QUANTILES,
        &[
            ("y", "income"),
            ("weights", "wt"),
            ("psu", "psu"),
            ("strata", "stratum"),
            ("quantiles", "0.5"),
            ("conf", "0.9"),
        ],
        &[],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let q =
        "survey::svyquantile(~income, design, 0.5, ci = TRUE, alpha = 0.1, na.rm = TRUE)$income";
    holds_more(
        &mut e,
        &[
            same(&format!("{q}[, 'quantile']"), "qhand(.5)"),
            same(
                &format!("{q}[, 'se']"),
                &format!("({q}[, 3] - {q}[, 2]) / (2 * qt(.95, 9))"),
            ),
        ]
        .join(", "),
    );
}

#[test]
fn replicate_weights_match_svy_brr_and_jackknife() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    let ctx = rep_context();
    let brr = ["brr1", "brr2", "brr3", "brr4"];
    let jk: Vec<String> = (1..=36).map(|i| format!("jk{i}")).collect();
    let jk: Vec<&str> = jk.iter().map(String::as_str).collect();
    let bs: Vec<String> = (1..=30).map(|i| format!("bs{i}")).collect();
    let bs: Vec<&str> = bs.iter().map(String::as_str).collect();
    // Stata computes a replicate design's variance from weights it holds in its own
    // way, and agrees with the textbook formula to about 1e-6; the textbook formula
    // (below, in plain R) is matched to 1e-9 and Stata's numbers to 1e-5
    const STATA: f64 = 1e-5;

    // balanced repeated replication: svy brr: mean y x, on 3 df
    let (outcome, text) = dialog_on(
        &mut e,
        MAKE_REP,
        &ctx,
        &SURVEY_MEANS,
        &[("weights", "w"), ("method", "BRR")],
        &[("x", &["y", "x"]), ("repweights", &brr)],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let r = row(&text, "y");
    assert_eq!(r.len(), 4, "{text}");
    // the interval is on 3 df: t(3) = 3.182446
    assert!(
        ((r[3] - r[2]) / 2.0 / r[1] - 3.182446305284263).abs() < 1e-4,
        "{text}"
    );
    let est = "survey::svymean(~y + x, design)";
    holds_more(
        &mut e,
        &[
            same(
                &format!("survey::SE({est})"),
                "c(rep_se(paste0('brr', 1:4), 1 / 4, d$y), rep_se(paste0('brr', 1:4), 1 / 4, d$x))",
            ),
            // Stata 19: svy brr, hadamard(H): mean y x (the means, then the standard errors)
            same(
                &format!("coef({est})"),
                "c(53.7774761904762, 13.6343174603175)",
            ),
            near(
                &format!("survey::SE({est})"),
                "c(.857631180665941, .101583426969287)",
                STATA,
            ),
            "inherits(design, 'svyrep.design')".to_string(),
        ]
        .join(", "),
    );
    // Fay's method takes the coefficient into the variance
    let (outcome, text) = dialog_on(
        &mut e,
        MAKE_REP,
        &ctx,
        &SURVEY_MEANS,
        &[("weights", "w"), ("method", "BRR"), ("fay", "0.3")],
        &[("x", &["y"]), ("repweights", &brr)],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    holds_more(&mut e, "design$fay.rho == 0.3");

    // the delete-one jackknife: svy jackknife: mean y x, on 35 df
    let (outcome, text) = dialog_on(
        &mut e,
        MAKE_REP,
        &ctx,
        &SURVEY_MEANS,
        &[("weights", "w"), ("method", "JK1")],
        &[("x", &["y", "x"]), ("repweights", &jk)],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let r = row(&text, "y");
    assert!(
        ((r[3] - r[2]) / 2.0 / r[1] - 2.030107928).abs() < 1e-4,
        "{text}"
    );
    holds_more(
        &mut e,
        &[
            same(
                &format!("survey::SE({est})"),
                "c(rep_se(paste0('jk', 1:36), 35 / 36, d$y), rep_se(paste0('jk', 1:36), 35 / 36, d$x))",
            ),
            // Stata 19: svy jackknife: mean y x
            near(
                &format!("survey::SE({est})"),
                "c(.89300622837083, .292403481274657)",
                STATA,
            ),
        ]
        .join(", "),
    );

    // bootstrap weights: the variance divides by the replicates (30), as Stata's
    // bsrweight does, and the intervals are normal
    let (outcome, text) = dialog_on(
        &mut e,
        MAKE_REP,
        &ctx,
        &SURVEY_MEANS,
        &[("weights", "w"), ("method", "bootstrap")],
        &[("x", &["y", "x"]), ("repweights", &bs)],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let r = row(&text, "y");
    assert!(
        ((r[3] - r[2]) / 2.0 / r[1] - 1.959963984540054).abs() < 1e-4,
        "{text}"
    );
    holds_more(
        &mut e,
        &[
            same(
                &format!("survey::SE({est})"),
                "c(rep_se(paste0('bs', 1:30), 1 / 30, d$y), rep_se(paste0('bs', 1:30), 1 / 30, d$x))",
            ),
            // Stata 19: svyset [pw=w], bsrweight(bs1-bs30) bsn(1) vce(bootstrap)
            near(
                &format!("survey::SE({est})"),
                "c(.759497433750394, .199552755496812)",
                STATA,
            ),
        ]
        .join(", "),
    );
    // a replicate design takes no strata, and needs two or more replicate columns
    let mut values = SURVEY_MEANS.defaults(&ctx);
    values.set_text("weights", "w");
    values.set_text("method", "JK1");
    values.set_text("strata", "s");
    values.set_list("x", vec!["y".into()]);
    values.set_list("repweights", vec!["jk1".into()]);
    let problems: Vec<&str> = SURVEY_MEANS
        .problems(&ctx, &values)
        .iter()
        .map(|p| p.widget)
        .collect();
    assert!(
        problems.contains(&"strata") && problems.contains(&"repweights"),
        "{problems:?}"
    );
}

#[test]
fn multistage_design_matches_svyset() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    // svyset psu [pw=wt], strata(stratum) fpc(pop) || ssu, fpc(f2)
    let (outcome, text) = dialog(
        &mut e,
        &SURVEY_MEANS,
        &[
            ("weights", "wt"),
            ("psu", "psu"),
            ("ssu", "ssu"),
            ("strata", "stratum"),
            ("fpc", "pop"),
            ("fpc2", "f2"),
        ],
        &[("x", &["income", "hours"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let est = "survey::svymean(~income + hours, design, na.rm = TRUE)";
    holds_more(
        &mut e,
        &[
            // the first stage's variance with its correction, plus the second stage's
            // within each primary unit, weighted by the first stage's sampling fraction
            same(&format!("survey::SE({est})[1]"), "ms_se(d$income)"),
            // Stata 19: svy: mean income hours, on 9 df
            same(
                &format!("coef({est})"),
                "c(321.811714975845, 31.6672041062802)",
            ),
            same(
                &format!("survey::SE({est})"),
                "c(5.49442654860872, .888193416298451)",
            ),
            "survey::degf(design) == 9".to_string(),
        ]
        .join(", "),
    );
    // without corrections the second stage adds nothing: Stata's svyset psu, strata() || ssu
    let (outcome, text) = dialog(
        &mut e,
        &SURVEY_MEANS,
        &[
            ("weights", "wt"),
            ("psu", "psu"),
            ("ssu", "ssu"),
            ("strata", "stratum"),
        ],
        &[("x", &["income", "hours"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    holds_more(
        &mut e,
        &[
            same(
                &format!("survey::SE({est})[1]"),
                "ms_se(d$income, fpc1 = FALSE, fpc2 = FALSE)",
            ),
            same(
                &format!("survey::SE({est})"),
                "c(5.74996648809755, .933873204758025)",
            ),
        ]
        .join(", "),
    );
}

#[test]
fn subpopulation_matches_svy_subpop() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    let design = [("weights", "wt"), ("psu", "psu"), ("strata", "stratum")];
    let mut set = vec![("subpop", "region == \"north\"")];
    set.extend(design);
    let (outcome, text) = dialog(
        &mut e,
        &SURVEY_MEANS,
        &set,
        &[("x", &["income", "hours"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let est = "survey::svymean(~income + hours, design, na.rm = TRUE)";
    holds_more(
        &mut e,
        &[
            // the data is not filtered, and the design keeps all its units: 9 df
            "nrow(d) == 60".to_string(),
            "survey::degf(design) == 9".to_string(),
            same(
                &format!("coef({est})[1]"),
                "mean_of(d$income, keep = !is.na(d$income), dom = d$region == 'north')[1]",
            ),
            same(
                &format!("survey::SE({est})[1]"),
                "mean_of(d$income, keep = !is.na(d$income), dom = d$region == 'north')[2]",
            ),
            // Stata 19: svy, subpop(if region == "north"): mean income hours
            same(
                &format!("coef({est})"),
                "c(319.404624277457, 31.2161271676301)",
            ),
            same(
                &format!("survey::SE({est})"),
                "c(7.09210480782385, 1.15581968524795)",
            ),
        ]
        .join(", "),
    );
    // a regression in a subpopulation
    let mut set = vec![
        ("y", "income"),
        ("subpop", "region == \"north\" & hours > 25"),
    ];
    set.extend(design);
    let (outcome, text) = dialog(&mut e, &SURVEY_REGRESSION, &set, &[("x", &["hours"])], &[]);
    assert_eq!(outcome, Outcome::Ok, "{text}");
    holds_more(
        &mut e,
        &[
            "nrow(d) == 60".to_string(),
            // Stata 19: svy, subpop(if region == "north" & hours > 25): regress income hours
            same(
                "coef(model)[c('hours', '(Intercept)')]",
                "c(7.93320228978366, 70.7949695782238)",
            ),
            same(
                "survey::SE(model)[c('hours', '(Intercept)')]",
                "c(.322188768582832, 11.5433916908677)",
            ),
        ]
        .join(", "),
    );
}

#[test]
fn survey_models_have_the_model_menu() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    assert!(postest::is_model_class("svyglm"));
    let entries: Vec<&str> = postest::model_specs("svyglm")
        .iter()
        .map(|s| s.entry)
        .collect();
    assert_eq!(
        entries,
        ["Model fit", "Test terms", "Predicted values and residuals"]
    );
    let design = [("weights", "wt"), ("psu", "psu"), ("strata", "stratum")];
    let mut set = vec![("y", "income"), ("save", "m1")];
    set.extend(design);
    let (outcome, text) = dialog(
        &mut e,
        &SURVEY_REGRESSION,
        &set,
        &[("x", &["hours", "region"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let mut set = vec![("y", "income"), ("save", "m2")];
    set.extend(design);
    let (outcome, text) = dialog(&mut e, &SURVEY_REGRESSION, &set, &[("x", &["hours"])], &[]);
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let on_model = |e: &mut Engine, spec: &Spec, model: &str, set: &[(&str, &str)]| {
        let ctx = Context {
            dataset: model.into(),
            frames: Vec::new(),
            columns: Vec::new(),
        };
        let mut values = spec.defaults(&ctx);
        for (id, text) in set {
            values.set_text(id, *text);
        }
        let code = spec.generate(&ctx, &values, Style::Base);
        assert!(code.is_ok(), "{code:?}");
        run(e, Source::Menu(spec.menu.into()), &code.unwrap_or_default())
    };
    // Model fit: the rows used (Stata's N) and the design's degrees of freedom
    let (outcome, text) = on_model(&mut e, &MODEL_FIT, "m1", &[]);
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(
        text.contains("Observations: 57")
            && text.contains("Design degrees of freedom: 9")
            && text.contains("Weighted deviance"),
        "{text}"
    );
    // Test terms: Stata's test after svy: regress (the adjusted Wald F)
    let (outcome, text) = on_model(&mut e, &TEST_TERMS, "m1", &[]);
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let close = |got: f64, want: f64| (got - want).abs() <= 1e-5 * want.abs();
    // Stata 19: test 2.region 3.region (F(2, 8)); test hours (F(1, 9))
    let r = row(&text, "region");
    assert_eq!(r.len(), 4, "{text}");
    assert!(
        close(r[0], 0.529097616057079)
            && r[1] == 2.0
            && r[2] == 8.0
            && close(r[3], 0.608405652964901),
        "{text}"
    );
    let r = row(&text, "hours");
    assert!(
        close(r[0], 413.334189168177)
            && r[1] == 1.0
            && r[2] == 9.0
            && close(r[3], 7.85667843989921e-09),
        "{text}"
    );
    let (outcome, text) = on_model(
        &mut e,
        &TEST_TERMS,
        "m1",
        &[("kind", "together"), ("terms", "hours, region")],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(text.contains("hours + region"), "{text}");
    // Predicted values and residuals, to Stata's predict, xb
    let (outcome, text) = on_model(
        &mut e,
        &PREDICTED,
        "m2",
        &[("data", "d"), ("rtype", "response")],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    holds_more(
        &mut e,
        &[
            "is.numeric(d$fitted) && !inherits(d$fitted, 'svystat')".to_string(),
            // Stata 19: svy: regress income hours; predict xb (rows 1 and 60)
            same(
                "d$fitted[c(1, 60)]",
                "c(296.961915779973, 352.808325679405)",
            ),
            // residuals on the rows the model used, missing on the three it left out
            near("d$resid[ok]", "(d$income - d$fitted)[ok]", 1e-9),
            "all(is.na(d$resid[c(4, 27, 51)]))".to_string(),
        ]
        .join(", "),
    );
}
