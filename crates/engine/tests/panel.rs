//! Statistics menu panel and fixed-effects models (P6.1) end to end against a
//! real R (skipped with a note when none is found, or when fixest or plm is
//! not installed). Each dialog's code is built from its spec and run on
//! made-up panels, and its estimates and standard errors are checked three
//! ways:
//!
//! - against the formulas written out here in plain R, with neither fixest nor
//!   plm: the within transform (and, for two-way effects, its Frisch-Waugh
//!   form), the sandwich with its small-sample factors, the Poisson and logit
//!   scores, the between regression, first differences by time, and the
//!   random-effects GLS transform of the [XT] manual (`xtreg, re`) with the
//!   variance components plm uses;
//! - against Stata 19's own results for the same data, recorded below
//!   (`xtreg, fe`, `areg`, `poisson` and `logit` with dummies, `xtreg, be`,
//!   `regress D.y D.x`, `xtreg, re`, `hausman`), to 1e-9;
//! - and against the package's report for the call written out by hand.
//!
//! The main panel is unbalanced (eight units over three to six years) with
//! two values missing inside units, so the first differences must skip them.
//! Random effects are checked against Stata on a balanced panel: on an
//! unbalanced one plm's Swamy-Arora estimate of the unit variance (Baltagi
//! and Chang's, with the between regression weighted by each unit's rows)
//! differs from both of Stata's, so there it is checked against its own
//! formula only.
//!
//! The test records the R and package versions it ran with (AGENTS.md
//! section 5), and checks R is the one `ENVIRONMENT` pins.

use std::time::{Duration, Instant};

use rcraft_catalog::panel::{FIXED_EFFECTS_GLM, FIXED_EFFECTS_REGRESSION, HAUSMAN, PANEL_MODEL};
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

/// The R, fixest and plm versions, recorded; false when a package is missing.
fn versions(e: &mut Engine) -> bool {
    let (outcome, text) = run(
        e,
        Source::Typed,
        "cat(paste(R.version$major, R.version$minor, sep = '.'), \
         vapply(c('fixest', 'plm'), function(p) if (requireNamespace(p, quietly = TRUE)) \
           as.character(packageVersion(p)) else 'none', ''))",
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let mut words = text.split_whitespace();
    let (r, fixest, plm) = (
        words.next().unwrap_or(""),
        words.next().unwrap_or("none"),
        words.next().unwrap_or("none"),
    );
    if fixest == "none" || plm == "none" {
        eprintln!("skipped: fixest or plm is not installed");
        return false;
    }
    let pinned = include_str!("../../../ENVIRONMENT")
        .lines()
        .find_map(|l| l.strip_prefix("R_VERSION="))
        .unwrap_or("")
        .trim();
    eprintln!("ran with R {r}, fixest {fixest} and plm {plm} (ENVIRONMENT pins R {pinned})");
    if r != pinned {
        eprintln!("note: this R is not the pinned one, so the run is not a Verified one");
    }
    true
}

/// Eight units observed for three to six of the years 2001 to 2006, 37 rows;
/// hours missing in row 9 (unit 2, 2003) and row 27 (unit 6, 2004).
const MAKE: &str = "d <- local({ \
     start <- c(2001, 2001, 2002, 2001, 2003, 2001, 2002, 2001); \
     stop <- c(2006, 2004, 2006, 2005, 2006, 2006, 2005, 2003); \
     id <- rep(1:8, stop - start + 1); year <- unlist(mapply(seq, start, stop)); \
     r <- seq_along(id); a <- c(0.5, -1, 2, 0.3, -0.7, 1.2, -0.2, 0.9)[id]; \
     hours <- round(10 + 2 * a + 3 * sin(r) + 0.5 * cos(3 * r), 3); \
     wage <- round(1 + 0.4 * hours + a + 0.2 * (year - 2003) + cos(r) + 0.3 * sin(5 * r), 3); \
     hours[c(9, 27)] <- NA; \
     firm <- c('p', 'q', 'r')[1 + (r * 5) %% 3]; \
     visits <- round(exp(0.5 + 0.06 * ifelse(is.na(hours), 10, hours) + 0.3 * a)) + (r * 7) %% 4; \
     days <- c(10, 20, 30, 15)[1 + r %% 4]; \
     left <- as.numeric(((r * 7) %% 5) < 2); \
     data.frame(id, year, hours, wage, firm, visits, days, left) })";

/// The formulas in plain R, for `MAKE`: `k` the complete rows, the one-way
/// within fit (`b1`, residuals `e1`), the two-way one by Frisch-Waugh (`b2`,
/// `e2`), and `cl(s, g)`, the sum over clusters of the squared score totals.
const ORACLE: &str = "k <- d[!is.na(d$hours), ]; N <- nrow(k); n <- length(unique(k$id)); \
     cl <- function(s, g) sum(tapply(s, g, sum)^2); \
     xt <- k$hours - ave(k$hours, k$id); yt <- k$wage - ave(k$wage, k$id); \
     b1 <- sum(xt * yt) / sum(xt^2); e1 <- yt - b1 * xt; \
     x2 <- resid(lm(hours ~ factor(id) + factor(year), k)); \
     y2 <- resid(lm(wage ~ factor(id) + factor(year), k)); \
     b2 <- sum(x2 * y2) / sum(x2^2); e2 <- y2 - b2 * x2";

/// Six units over five years. `wage2` adds a shock to each year, so a two-way
/// random-effects model has both components above zero.
const BALANCED: &str = "d <- local({ \
     id <- rep(1:6, each = 5); year <- rep(2001:2005, times = 6); \
     a <- c(0.5, -1, 2, 0.3, -0.7, 1.2)[id]; \
     hours <- round(10 + 2 * a + 3 * sin(1:30), 3); \
     wage <- round(1 + 0.4 * hours + a + 0.2 * (year - 2003) + cos(1:30), 3); \
     wage2 <- wage + 0.5 * c(0.8, -0.5, 0.3, -0.9, 0.6)[year - 2000]; \
     data.frame(id, year, hours, wage, wage2) })";

fn context() -> Context {
    Context {
        dataset: "d".into(),
        frames: Vec::new(),
        columns: [
            ("id", VarKind::Number),
            ("year", VarKind::Number),
            ("hours", VarKind::Number),
            ("wage", VarKind::Number),
            ("wage2", VarKind::Number),
            ("firm", VarKind::Text),
            ("visits", VarKind::Number),
            ("days", VarKind::Number),
            ("left", VarKind::Number),
        ]
        .into_iter()
        .map(|(name, kind)| ColumnInfo {
            name: name.into(),
            kind,
        })
        .collect(),
    }
}

/// Makes the data (`make`), then runs the dialog's code on it.
fn dialog(
    e: &mut Engine,
    make: &str,
    spec: &Spec,
    set: &[(&str, &str)],
    lists: &[(&str, &[&str])],
    flags: &[&str],
) -> (Outcome, String) {
    let (made, text) = run(e, Source::Typed, make);
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

/// A report with spaces tidied, blank lines and the lines that echo the call
/// left out.
fn report(text: &str) -> Vec<String> {
    text.lines()
        .filter(|l| {
            let l = l.trim_start();
            !(l.starts_with("Call:")
                || l.starts_with("plm::plm(")
                || l.starts_with("plm(")
                || l.starts_with("warning:"))
        })
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|l| !l.is_empty())
        .collect()
}

/// Runs R that stops unless every one of `exprs` holds.
fn holds(e: &mut Engine, exprs: &[String]) {
    let exprs = exprs.join(", ");
    let (outcome, text) = run(e, Source::Typed, &format!("stopifnot({exprs})"));
    assert_eq!(outcome, Outcome::Ok, "{exprs}\n{text}");
}

/// Runs R that must succeed (the oracle's objects).
fn define(e: &mut Engine, code: &str) {
    let (outcome, text) = run(e, Source::Typed, code);
    assert_eq!(outcome, Outcome::Ok, "{code}\n{text}");
}

/// `all.equal` to `tol`, unnamed.
fn within(a: &str, b: &str, tol: &str) -> String {
    format!("isTRUE(all.equal(unname(as.vector({a})), unname(as.vector({b})), tolerance = {tol}))")
}

/// `all.equal` to 1e-9.
fn same(a: &str, b: &str) -> String {
    within(a, b, "1e-9")
}

#[test]
fn fixed_effects_regression_is_xtreg_and_areg() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    // unit and year effects, clustered by unit (fixest's default)
    let (outcome, text) = dialog(
        &mut e,
        MAKE,
        &FIXED_EFFECTS_REGRESSION,
        &[("y", "wage")],
        &[("x", &["hours"]), ("fe", &["id", "year"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let (_, expected) = run(
        &mut e,
        Source::Typed,
        "m <- fixest::feols(wage ~ hours | id + year, data = d, vcov = 'cluster'); \
         print(summary(m)); print(stats::confint(m, level = 0.95))",
    );
    assert!(report(&expected).len() > 6, "{expected}");
    assert_eq!(report(&text), report(&expected), "{text}");
    define(&mut e, ORACLE);
    // the sandwich over units, with G/(G-1) (N-1)/(N-K), K counting hours,
    // the year dummies and the constant, as xtreg, fe does
    holds(
        &mut e,
        &[
            same("coef(model)", "b2"),
            same(
                "fixest::se(model)",
                "sqrt(cl(x2 * e2, k$id) * n / (n - 1) * (N - 1) / (N - 1 - length(unique(k$year)))) / sum(x2^2)",
            ),
            // Stata 19: xtreg wage hours i.year, fe vce(cluster id)
            same(
                "c(coef(model), fixest::se(model))",
                "c(.397247281927981, .0649652512254841)",
            ),
        ],
    );

    // conventional errors: s^2 on N - n - 1 degrees of freedom
    let (outcome, text) = dialog(
        &mut e,
        MAKE,
        &FIXED_EFFECTS_REGRESSION,
        &[("y", "wage"), ("errors", "iid"), ("save", "fe1")],
        &[("x", &["hours"]), ("fe", &["id"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    // robust: the sandwich with N / (N - K), K = n + 1
    let (outcome, text) = dialog(
        &mut e,
        MAKE,
        &FIXED_EFFECTS_REGRESSION,
        &[("y", "wage"), ("errors", "hetero"), ("save", "fe2")],
        &[("x", &["hours"]), ("fe", &["id"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    // clustered by firm, which the units are not nested in, so K = n + 1
    let (outcome, text) = dialog(
        &mut e,
        MAKE,
        &FIXED_EFFECTS_REGRESSION,
        &[("y", "wage"), ("cluster", "firm"), ("save", "fe3")],
        &[("x", &["hours"]), ("fe", &["id"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    // two-way clustered by unit and year: Cameron, Gelbach and Miller's
    // V(id) + V(year) - V(id x year), with fixest's min(G) / (min(G) - 1)
    let (outcome, text) = dialog(
        &mut e,
        MAKE,
        &FIXED_EFFECTS_REGRESSION,
        &[("y", "wage"), ("errors", "twoway"), ("save", "fe4")],
        &[("x", &["hours"]), ("fe", &["id", "year"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    define(&mut e, ORACLE);
    holds(
        &mut e,
        &[
            same("coef(fe1)", "b1"),
            same(
                "fixest::se(fe1)",
                "sqrt(sum(e1^2) / (N - n - 1) / sum(xt^2))",
            ),
            same(
                "fixest::se(fe2)",
                "sqrt(sum(xt^2 * e1^2) * N / (N - n - 1)) / sum(xt^2)",
            ),
            same(
                "fixest::se(fe3)",
                "sqrt(cl(xt * e1, k$firm) * 3 / 2 * (N - 1) / (N - n - 1)) / sum(xt^2)",
            ),
            // fixest removes two-way effects by iterating to its own tolerance,
            // which leaves this one about 1.5e-9 (relative) from the exact figure
            within(
                "fixest::se(fe4)",
                "local({ g <- min(n, length(unique(k$year))); \
                   sqrt(g / (g - 1) * (cl(x2 * e2, k$id) + cl(x2 * e2, k$year) - \
                     cl(x2 * e2, paste(k$id, k$year))) * (N - 1) / (N - 2)) / sum(x2^2) })",
                "1e-8",
            ),
            // Stata 19: xtreg wage hours, fe; areg, absorb(id) vce(robust);
            // areg, absorb(id) vce(cluster firm)
            same(
                "c(coef(fe1), fixest::se(fe1))",
                "c(.363860889561733, .075259061265913)",
            ),
            same("fixest::se(fe2)", ".0654844597745196"),
            same("fixest::se(fe3)", ".124260358582127"),
        ],
    );
}

#[test]
fn fixed_effects_glm_is_poisson_and_logit_with_dummies() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    let (outcome, text) = dialog(
        &mut e,
        MAKE,
        &FIXED_EFFECTS_GLM,
        &[("y", "visits"), ("exposure", "days")],
        &[("x", &["hours"]), ("fe", &["id"])],
        &["ratios"],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let (_, expected) = run(
        &mut e,
        Source::Typed,
        "m <- fixest::fepois(visits ~ hours | id, data = d, offset = ~log(days), vcov = 'cluster', \
         ssc = fixest::ssc(adj = FALSE)); \
         print(summary(m)); cat('\\nRate ratios\\n'); \
         print(exp(cbind(rate_ratio = stats::coef(m), stats::confint(m, level = 0.95))))",
    );
    assert_eq!(report(&text), report(&expected), "{text}");
    for (errors, save) in [("iid", "p2"), ("hetero", "p3")] {
        let (outcome, text) = dialog(
            &mut e,
            MAKE,
            &FIXED_EFFECTS_GLM,
            &[
                ("y", "visits"),
                ("exposure", "days"),
                ("errors", errors),
                ("save", save),
            ],
            &[("x", &["hours"]), ("fe", &["id"])],
            &[],
        );
        assert_eq!(outcome, Outcome::Ok, "{text}");
    }
    define(&mut e, ORACLE);
    // the dummy-variable Poisson fitted to its maximum, its information and
    // scores; clusters get G/(G-1), robust N/(N-1), as Stata's ML commands
    define(
        &mut e,
        "gp <- glm(visits ~ hours + factor(id) + offset(log(days)), family = poisson, data = k, \
           control = glm.control(epsilon = 1e-14, maxit = 100)); \
         X <- model.matrix(gp); mu <- fitted(gp); D <- solve(crossprod(X, mu * X)); \
         sc <- (k$visits - mu) * X; \
         v_cl <- (D %*% crossprod(rowsum(sc, k$id)) %*% D)[2, 2] * n / (n - 1); \
         v_hc <- (D %*% crossprod(sc) %*% D)[2, 2] * N / (N - 1)",
    );
    // fixest's GLM standard errors sit about 1.2e-7 (relative) from the exact
    // information whatever its tolerances (its coefficients are glm's to
    // 1e-11), so they are held to 1e-6
    let near = |a: &str, b: &str| within(a, b, "1e-6");
    holds(
        &mut e,
        &[
            same("coef(model)", "coef(gp)['hours']"),
            near("fixest::se(model)", "sqrt(v_cl)"),
            near("fixest::se(p2)", "sqrt(D[2, 2])"),
            near("fixest::se(p3)", "sqrt(v_hc)"),
            // Stata 19: poisson visits hours i.id, exposure(days), with
            // vce(cluster id), the default, and vce(robust)
            same("coef(model)", ".0225502965975261"),
            near("fixest::se(model)", ".0277759713787516"),
            near("fixest::se(p2)", ".0372964390595609"),
            near("fixest::se(p3)", ".025318094996149"),
        ],
    );

    // logit with firm effects: conventional, and clustered by firm
    let (outcome, text) = dialog(
        &mut e,
        MAKE,
        &FIXED_EFFECTS_GLM,
        &[
            ("y", "left"),
            ("family", "logit"),
            ("errors", "iid"),
            ("save", "lg"),
        ],
        &[("x", &["hours"]), ("fe", &["firm"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let (outcome, text) = dialog(
        &mut e,
        MAKE,
        &FIXED_EFFECTS_GLM,
        &[
            ("y", "left"),
            ("family", "logit"),
            ("cluster", "firm"),
            ("save", "lg2"),
        ],
        &[("x", &["hours"]), ("fe", &["firm"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    define(&mut e, ORACLE);
    define(
        &mut e,
        "gl <- glm(left ~ hours + factor(firm), family = binomial, data = k, \
           control = glm.control(epsilon = 1e-14, maxit = 100)); \
         X <- model.matrix(gl); p <- fitted(gl); D <- solve(crossprod(X, p * (1 - p) * X)); \
         v_cl <- (D %*% crossprod(rowsum((k$left - p) * X, k$firm)) %*% D)[2, 2] * 3 / 2",
    );
    holds(
        &mut e,
        &[
            same("coef(lg)", "coef(gl)['hours']"),
            near("fixest::se(lg)", "sqrt(D[2, 2])"),
            near("fixest::se(lg2)", "sqrt(v_cl)"),
            // Stata 19: logit left hours i.firm, and with vce(cluster firm);
            // Stata's logit stops about 1e-9 short of the maximum
            near("coef(lg)", ".0251208887001139"),
            near("fixest::se(lg)", ".130989080355023"),
            near("fixest::se(lg2)", ".208014414937871"),
        ],
    );
}

#[test]
fn panel_model_is_xtreg_and_differences_by_time() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    let (outcome, text) = dialog(
        &mut e,
        MAKE,
        &PANEL_MODEL,
        &[("y", "wage"), ("id", "id"), ("time", "year")],
        &[("x", &["hours"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let (_, expected) = run(
        &mut e,
        Source::Typed,
        "m <- plm::plm(wage ~ hours, data = d, index = c('id', 'year'), model = 'within', \
         effect = 'individual'); print(summary(m))",
    );
    assert!(report(&expected).len() > 8, "{expected}");
    assert_eq!(report(&text), report(&expected), "{text}");

    // clustered by unit: plm's "sss", G/(G-1) (N-1)/(N-K) with K the slopes
    // only (Stata's xtreg, fe counts the constant too, so its clustered error
    // is sqrt((N-1)/(N-2)) larger; the fixed-effects dialog gives Stata's)
    let (outcome, text) = dialog(
        &mut e,
        MAKE,
        &PANEL_MODEL,
        &[
            ("y", "wage"),
            ("id", "id"),
            ("time", "year"),
            ("errors", "group"),
            ("save", "wc"),
        ],
        &[("x", &["hours"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let (outcome, text) = dialog(
        &mut e,
        MAKE,
        &PANEL_MODEL,
        &[
            ("y", "wage"),
            ("id", "id"),
            ("time", "year"),
            ("model", "between"),
            ("save", "bm"),
        ],
        &[("x", &["hours"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    define(&mut e, ORACLE);
    holds(
        &mut e,
        &[
            same("coef(model)", "b1"),
            same("sqrt(vcov(model))", "sqrt(sum(e1^2) / (N - n - 1) / sum(xt^2))"),
            same(
                "summary(wc, vcov = plm::vcovHC(wc, cluster = 'group', type = 'sss'))$coefficients[1, 2]",
                "sqrt(cl(xt * e1, k$id) * n / (n - 1)) / sum(xt^2)",
            ),
            // between: least squares on the units' means of the complete rows
            same(
                "c(coef(bm), sqrt(diag(vcov(bm))))",
                "local({ m <- lm(wage ~ hours, aggregate(cbind(wage, hours) ~ id, k, mean)); \
                   c(coef(m), sqrt(diag(vcov(m)))) })",
            ),
            // Stata 19: xtreg wage hours, fe; xtreg wage hours, be
            same(
                "c(coef(model), sqrt(vcov(model)))",
                "c(.363860889561733, .075259061265913)",
            ),
            same(
                "c(coef(bm), sqrt(diag(vcov(bm))))",
                "c(-4.20330221975148, .93088406586952, 1.00710076155325, .0937248479684603)",
            ),
        ],
    );

    // first differences go by year: the two missing hours take out four
    // differences, leaving 25, where plm's row-wise model = "fd" would
    // difference across the gaps (27)
    let (outcome, text) = dialog(
        &mut e,
        MAKE,
        &PANEL_MODEL,
        &[
            ("y", "wage"),
            ("id", "id"),
            ("time", "year"),
            ("model", "fd"),
            ("errors", "group"),
            ("save", "fdm"),
        ],
        &[("x", &["hours"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    holds(
        &mut e,
        &[
            "nobs(fdm) == 25".to_string(),
            same(
                "c(coef(fdm), vcov(fdm))",
                "local({ o <- d[order(d$id, d$year), ]; \
                   lag1 <- function(v) ave(v, o$id, FUN = function(z) c(NA, diff(z))); \
                   stopifnot(all(ave(o$year, o$id, FUN = function(z) c(1, diff(z))) == 1)); \
                   m <- lm(lag1(o$wage) ~ lag1(o$hours)); c(coef(m), vcov(m)) })",
            ),
            // Stata 19: regress D.wage D.hours, and with vce(cluster id)
            same(
                "c(coef(fdm), sqrt(diag(vcov(fdm))))",
                "c(.322960986024369, .361874811097172, .153590884504173, .0783286884035842)",
            ),
            same(
                "sqrt(plm::vcovHC(fdm, cluster = 'group', type = 'sss')[2, 2])",
                ".0385511591559078",
            ),
        ],
    );

    // random effects on the unbalanced panel: the GLS transform of the [XT]
    // manual with plm's components, sigma_e^2 from the within residuals on
    // N - n - K + 1 and sigma_u^2 Baltagi and Chang's from the between
    // regression weighted by each unit's rows
    let (outcome, text) = dialog(
        &mut e,
        MAKE,
        &PANEL_MODEL,
        &[
            ("y", "wage"),
            ("id", "id"),
            ("time", "year"),
            ("model", "random"),
            ("save", "re"),
        ],
        &[("x", &["hours"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    define(&mut e, ORACLE);
    holds(
        &mut e,
        &[same(
            "c(coef(re), vcov(re), plm::ercomp(re)$sigma2)",
            "local({ Ti <- as.vector(table(k$id)); Tit <- ave(k$hours, k$id, FUN = length); \
               xm <- ave(k$hours, k$id); ym <- ave(k$wage, k$id); se2 <- sum(e1^2) / (N - n - 1); \
               bw <- lm(wage ~ hours, aggregate(cbind(wage, hours) ~ id, k, mean), weights = Ti); \
               X <- cbind(1, k$hours); Z <- model.matrix(~ factor(id) - 1, k); \
               P <- Z %*% diag(1 / Ti) %*% t(Z); \
               ctr <- sum(diag(solve(t(X) %*% P %*% X) %*% t(X) %*% Z %*% t(Z) %*% X)); \
               su2 <- max(0, (sum(Ti * resid(bw)^2) - (n - 2) * se2) / (N - ctr)); \
               th <- 1 - sqrt(se2 / (Tit * su2 + se2)); \
               m <- lm(I(k$wage - th * ym) ~ 0 + I(1 - th) + I(k$hours - th * xm)); \
               c(coef(m), vcov(m), se2, su2) })",
        )],
    );
}

#[test]
fn random_effects_and_hausman_are_statas_on_a_balanced_panel() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    let (outcome, text) = dialog(
        &mut e,
        BALANCED,
        &PANEL_MODEL,
        &[
            ("y", "wage"),
            ("id", "id"),
            ("time", "year"),
            ("model", "random"),
            ("errors", "group"),
            ("save", "re"),
        ],
        &[("x", &["hours"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    holds(
        &mut e,
        &[
            // Stata 19: xtreg wage hours, re (and its sigma_u, sigma_e), and
            // with vce(cluster id)
            same(
                "c(coef(re), sqrt(diag(vcov(re))))",
                "c(-1.14125400866092, .629775877778618, .768101664161492, .0683928049735991)",
            ),
            same(
                "sqrt(plm::ercomp(re)$sigma2)",
                "c(.81448222446206, .112418293755807)",
            ),
            same(
                "sqrt(diag(plm::vcovHC(re, cluster = 'group', type = 'sss')))",
                "c(.765609255463114, .0753381423632523)",
            ),
        ],
    );

    // two-way random effects (Swamy-Arora, Baltagi 2013 section 3.3), which
    // Stata does not fit: the components from the two-way within residuals
    // and the two between regressions, then the GLS transform
    let (outcome, text) = dialog(
        &mut e,
        BALANCED,
        &PANEL_MODEL,
        &[
            ("y", "wage2"),
            ("id", "id"),
            ("time", "year"),
            ("model", "random"),
            ("effect", "twoways"),
            ("save", "re2"),
        ],
        &[("x", &["hours"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    holds(
        &mut e,
        &[same(
            "c(coef(re2), vcov(re2), plm::ercomp(re2)$sigma2)",
            "local({ n <- 6; t <- 5; x <- d$hours; y <- d$wage2; \
               w2 <- function(v) v - ave(v, d$id) - ave(v, d$year) + mean(v); \
               fit <- function(xx, yy) yy - sum(xx * yy) / sum(xx^2) * xx; \
               sv <- sum(fit(w2(x), w2(y))^2) / ((n - 1) * (t - 1) - 1); \
               bi <- function(v) ave(v, d$id) - mean(v); bt <- function(v) ave(v, d$year) - mean(v); \
               s1 <- sum(fit(bi(x), bi(y))^2) / (n - 2); s2 <- sum(fit(bt(x), bt(y))^2) / (t - 2); \
               s3 <- s1 + s2 - sv; \
               th1 <- 1 - sqrt(sv / s1); th2 <- 1 - sqrt(sv / s2); th3 <- th1 + th2 + sqrt(sv / s3) - 1; \
               tr <- function(v) v - th1 * ave(v, d$id) - th2 * ave(v, d$year) + th3 * mean(v); \
               m <- lm(tr(y) ~ 0 + tr(rep(1, n * t)) + tr(x)); \
               c(coef(m), vcov(m), sv, (s1 - sv) / t, (s2 - sv) / n) })",
        )],
    );

    // Hausman: (b_fe - b_re)^2 / (V_fe - V_re), printed as the dialog prints it
    let (outcome, text) = dialog(
        &mut e,
        BALANCED,
        &HAUSMAN,
        &[("y", "wage"), ("id", "id"), ("time", "year")],
        &[("x", &["hours"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let (_, expected) = run(
        &mut e,
        Source::Typed,
        "h <- local({ fixed <- plm::plm(wage ~ hours, data = d, index = c('id', 'year'), model = 'within'); \
           random <- plm::plm(wage ~ hours, data = d, index = c('id', 'year'), model = 'random'); \
           plm::phtest(fixed, random) }); print(h)",
    );
    assert!(text.contains("Hausman Test"), "{text}");
    assert_eq!(report(&text), report(&expected), "{text}");
    holds(
        &mut e,
        &[
            same(
                "h$statistic",
                "local({ fe <- plm::plm(wage ~ hours, data = d, index = c('id', 'year'), model = 'within'); \
                   re <- plm::plm(wage ~ hours, data = d, index = c('id', 'year'), model = 'random'); \
                   (coef(fe)['hours'] - coef(re)['hours'])^2 / \
                     (vcov(fe)['hours', 'hours'] - vcov(re)['hours', 'hours']) })",
            ),
            // Stata 19: hausman fe re
            same("h$statistic", "193.045751189116"),
        ],
    );
}
