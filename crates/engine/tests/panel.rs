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

// ---- P6.1, second half: dynamic panels, panel IV, unit roots, serial correlation ----

use rcraft_catalog::panel::{DYNAMIC_PANEL, PANEL_IV, PANEL_SERIAL, PANEL_UNIT_ROOT};
use rcraft_catalog::postest::{self, MODEL_FIT, ROBUST};

/// Forty units over seven years of `y = 0.5 y(-1) + 0.4 x + a + e`; the same
/// numbers were loaded into Stata 19 for `xtabond`.
const DYNAMIC: &str = "d <- local({ set.seed(61); n <- 40; T <- 7; id <- rep(1:n, each = T); \
     year <- rep(2001:(2000 + T), n); a <- rnorm(n)[id]; x <- round(rnorm(n * T) + 0.3 * a, 3); \
     z <- round(rnorm(n * T) + 0.5 * x, 3); y <- numeric(n * T); y0 <- rnorm(n); \
     for (i in 1:n) { prev <- y0[i]; for (t in 1:T) { r <- (i - 1) * T + t; \
       y[r] <- 0.5 * prev + 0.4 * x[r] + a[r] + rnorm(1); prev <- y[r] } }; \
     data.frame(id, year, x, z, y = round(y, 3)) })";

/// `DYNAMIC` with three rows dropped and one `x` missing: gaps in time.
const GAPPED: &str = "d <- local({ d <- local({ set.seed(61); n <- 40; T <- 7; \
     id <- rep(1:n, each = T); year <- rep(2001:(2000 + T), n); a <- rnorm(n)[id]; \
     x <- round(rnorm(n * T) + 0.3 * a, 3); z <- round(rnorm(n * T) + 0.5 * x, 3); \
     y <- numeric(n * T); y0 <- rnorm(n); \
     for (i in 1:n) { prev <- y0[i]; for (t in 1:T) { r <- (i - 1) * T + t; \
       y[r] <- 0.5 * prev + 0.4 * x[r] + a[r] + rnorm(1); prev <- y[r] } }; \
     data.frame(id, year, x, z, y = round(y, 3)) }); \
     d <- d[-c(10, 11, 60), ]; d$x[100] <- NA; d })";

/// Thirty units over six years, `x2` correlated with the error and with the
/// two instruments `z1` and `z2`; `x1` exogenous.
const INSTRUMENTED: &str = "d <- local({ set.seed(62); n <- 30; T <- 6; \
     id <- rep(1:n, each = T); year <- rep(2001:(2000 + T), n); a <- rnorm(n)[id]; \
     u <- rnorm(n * T); x1 <- round(rnorm(n * T) + 0.4 * a, 3); z1 <- round(rnorm(n * T), 3); \
     z2 <- round(rnorm(n * T) + 0.3 * a, 3); \
     x2 <- round(0.8 * z1 + 0.5 * z2 + 0.6 * u + 0.3 * a + rnorm(n * T), 3); \
     y <- round(1 + 0.5 * x1 + 0.7 * x2 + a + u, 3); data.frame(id, year, y, x1, x2, z1, z2) })";

/// Twenty units over twenty periods: `y` stationary with its own root for
/// each unit, `w` a random walk with a start for each unit.
const ROOTS: &str = "d <- local({ set.seed(63); n <- 20; T <- 20; id <- rep(1:n, each = T); \
     year <- rep(1:T, n); y <- numeric(n * T); mu <- rnorm(n); \
     for (i in 1:n) { prev <- mu[i]; rho <- 0.6 + 0.3 * (i %% 3) / 2; \
       for (t in 1:T) { r <- (i - 1) * T + t; y[r] <- mu[i] * (1 - rho) + rho * prev + rnorm(1); \
         prev <- y[r] } }; y <- round(y, 3); w <- numeric(n * T); \
     for (i in 1:n) { prev <- 0; for (t in 1:T) { r <- (i - 1) * T + t; prev <- prev + rnorm(1); \
       w[r] <- round(prev + mu[i], 3) } }; data.frame(id, year, y, w) })";

/// The context of the second half's data (the first half's columns as well).
fn panel_context() -> Context {
    let mut ctx = context();
    for name in ["x", "z", "y", "x1", "x2", "z1", "z2", "w"] {
        ctx.columns.push(ColumnInfo {
            name: name.into(),
            kind: VarKind::Number,
        });
    }
    ctx
}

/// Like `dialog`, with the second half's columns.
fn dialog2(
    e: &mut Engine,
    make: &str,
    spec: &Spec,
    set: &[(&str, &str)],
    lists: &[(&str, &[&str])],
) -> (Outcome, String) {
    let (made, text) = run(e, Source::Typed, make);
    assert_eq!(made, Outcome::Ok, "{text}");
    let ctx = panel_context();
    let mut values = spec.defaults(&ctx);
    for (id, text) in set {
        values.set_text(id, *text);
    }
    for (id, names) in lists {
        values.set_list(id, names.iter().map(|n| n.to_string()).collect());
    }
    let code = spec.generate(&ctx, &values, Style::Base);
    assert!(code.is_ok(), "{code:?}");
    run(e, Source::Menu(spec.menu.into()), &code.unwrap_or_default())
}

/// The number after `label` in `text` (up to the next comma or space).
fn number_after(text: &str, label: &str) -> f64 {
    text.split(label)
        .nth(1)
        .and_then(|rest| {
            rest.split(|c: char| c == ',' || c.is_whitespace())
                .next()
                .and_then(|n| n.parse::<f64>().ok())
        })
        .unwrap_or(f64::NAN)
}

/// Runs the checks `exprs` (as `holds`, for several at once).
fn all_hold(e: &mut Engine, exprs: &[String]) {
    holds(e, exprs);
}

#[test]
fn dynamic_panel_matches_xtabond() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    let lists: &[(&str, &[&str])] = &[("x", &["x"])];
    // two steps with robust (Windmeijer) errors, the dialog's default
    let (outcome, text) = dialog2(
        &mut e,
        DYNAMIC,
        &DYNAMIC_PANEL,
        &[("y", "y"), ("id", "id"), ("time", "year"), ("save", "ab1")],
        lists,
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    for needle in [
        "Two-steps model Difference GMM",
        "Sargan test: chisq(14)",
        "Autocorrelation test (1)",
        "Autocorrelation test (2)",
    ] {
        assert!(text.contains(needle), "{needle} missing in {text}");
    }
    // one step, robust: no Sargan test is printed
    let (outcome, text) = dialog2(
        &mut e,
        DYNAMIC,
        &DYNAMIC_PANEL,
        &[
            ("y", "y"),
            ("id", "id"),
            ("time", "year"),
            ("steps", "onestep"),
            ("save", "ab2"),
        ],
        lists,
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(
        text.contains("One-step model Difference GMM")
            && text.contains("Sargan test: not shown after one step")
            && !text.contains("Sargan test: chisq"),
        "{text}"
    );
    // two steps, conventional errors
    let (outcome, text) = dialog2(
        &mut e,
        DYNAMIC,
        &DYNAMIC_PANEL,
        &[
            ("y", "y"),
            ("id", "id"),
            ("time", "year"),
            ("errors", "conventional"),
            ("save", "ab3"),
        ],
        lists,
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    // two lags of the outcome
    let (outcome, text) = dialog2(
        &mut e,
        DYNAMIC,
        &DYNAMIC_PANEL,
        &[
            ("y", "y"),
            ("id", "id"),
            ("time", "year"),
            ("lags", "2"),
            ("save", "ab4"),
        ],
        lists,
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let robust = "summary(ab1, robust = TRUE)$coefficients[, 2]";
    all_hold(
        &mut e,
        &[
            // Stata 19: xtabond y x, lags(1) noconstant twostep vce(robust)
            same("coef(ab1)", "c(.10674838203774, .267284198210873)"),
            same(robust, "sqrt(c(.0409124022828614, .00546991211180994))"),
            // estat abond: m1 and m2, as plm::mtest with the robust covariance
            same(
                "c(plm::mtest(ab1, order = 1, vcov = plm::vcovHC(ab1))$statistic, \
                   plm::mtest(ab1, order = 2, vcov = plm::vcovHC(ab1))$statistic)",
                "c(-1.72011985395622, -.0601315407473723)",
            ),
            // xtabond y x, lags(1) noconstant vce(robust)
            same("coef(ab2)", "c(.137591110944501, .303749381281215)"),
            same(
                "summary(ab2, robust = TRUE)$coefficients[, 2]",
                "sqrt(c(.0383260657863139, .00642366898045101))",
            ),
            same(
                "c(plm::mtest(ab2, order = 1, vcov = plm::vcovHC(ab2))$statistic, \
                   plm::mtest(ab2, order = 2, vcov = plm::vcovHC(ab2))$statistic)",
                "c(-2.15400917680742, -.0195494505317342)",
            ),
            // xtabond y x, lags(1) noconstant twostep: conventional errors, m1, m2
            // and Sargan's chi2(14) (Stata prints 11.24771)
            same("coef(ab3)", "c(.10674838203774, .267284198210873)"),
            same(
                "summary(ab3, robust = FALSE)$coefficients[, 2]",
                "sqrt(c(.022210139422862, .00316491037462199))",
            ),
            same(
                "c(plm::mtest(ab3, order = 1)$statistic, plm::mtest(ab3, order = 2)$statistic)",
                "c(-2.48073221558718, -.0660746410542963)",
            ),
            within("plm::sargan(ab3)$statistic", "11.24771", "1e-6"),
            // xtabond y x, lags(2) noconstant twostep vce(robust); estat abond, artests(3)
            same(
                "coef(ab4)",
                "c(.194582466618915, .0562935012693752, .262638919234913)",
            ),
            same(
                "summary(ab4, robust = TRUE)$coefficients[, 2]",
                "sqrt(c(.137141108118089, .0137414714783214, .00739475704252883))",
            ),
            same(
                "c(plm::mtest(ab4, order = 1, vcov = plm::vcovHC(ab4))$statistic, \
                   plm::mtest(ab4, order = 2, vcov = plm::vcovHC(ab4))$statistic, \
                   plm::mtest(ab4, order = 3, vcov = plm::vcovHC(ab4))$statistic)",
                "c(-1.32565721023591, -.729466985882428, .216673661584872)",
            ),
        ],
    );
    // the package's own report for the call written out by hand
    let (_, expected) = run(
        &mut e,
        Source::Typed,
        "m <- local({ plm <- plm::plm; plm::pgmm(y ~ stats::lag(y, 1) + x | stats::lag(y, 2:99), \
           data = d, index = c('id', 'year'), effect = 'individual', \
           model = 'twosteps', transformation = 'd') }); print(summary(m, robust = TRUE))",
    );
    let (_, shown) = dialog2(
        &mut e,
        DYNAMIC,
        &DYNAMIC_PANEL,
        &[("y", "y"), ("id", "id"), ("time", "year")],
        lists,
    );
    // the lines after the call: from "Balanced Panel" on
    let body = |t: &str| -> Vec<String> {
        let lines = report(t);
        lines
            .iter()
            .position(|l| l.starts_with("Balanced Panel"))
            .map(|i| lines.get(i..).unwrap_or_default().to_vec())
            .unwrap_or_default()
    };
    assert!(body(&expected).len() > 8, "{expected}");
    assert_eq!(body(&shown), body(&expected), "{shown}");
}

#[test]
fn panel_iv_matches_xtivreg() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    let lists: &[(&str, &[&str])] = &[("x", &["x1"]), ("endog", &["x2"]), ("inst", &["z1", "z2"])];
    let (outcome, text) = dialog2(
        &mut e,
        INSTRUMENTED,
        &PANEL_IV,
        &[("y", "y"), ("id", "id"), ("time", "year"), ("save", "fe")],
        lists,
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(
        text.contains("Instrumental variable estimation") && text.contains("Within Model"),
        "{text}"
    );
    let (outcome, text) = dialog2(
        &mut e,
        INSTRUMENTED,
        &PANEL_IV,
        &[
            ("y", "y"),
            ("id", "id"),
            ("time", "year"),
            ("model", "random"),
            ("save", "re"),
        ],
        lists,
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(
        text.contains("Random Effect Model") && text.contains("Balestra-Varadharajan-Krishnakumar"),
        "{text}"
    );
    all_hold(
        &mut e,
        &[
            // two-stage least squares on the demeaned data, written out
            same(
                "c(coef(fe), sqrt(diag(vcov(fe))))",
                "local({ n <- length(unique(d$id)); N <- nrow(d); dm <- function(v) v - ave(v, d$id); \
                   X <- cbind(dm(d$x1), dm(d$x2)); Z <- cbind(dm(d$x1), dm(d$z1), dm(d$z2)); \
                   yy <- dm(d$y); Xh <- Z %*% solve(crossprod(Z), crossprod(Z, X)); \
                   b <- solve(crossprod(Xh, X), crossprod(Xh, yy)); e <- yy - X %*% b; \
                   c(b, sqrt(diag(sum(e^2) / (N - n - 2) * solve(crossprod(Xh))))) })",
            ),
            // Stata 19: xtivreg y x1 (x2 = z1 z2), fe: x1, x2 and their errors
            same(
                "c(coef(fe), sqrt(diag(vcov(fe))))",
                "c(.58494433324006, .758761826220583, sqrt(.00781716168448615), \
                   sqrt(.00837930012806866))",
            ),
            // xtivreg y x1 (x2 = z1 z2), re (the default, g2sls); sigma_e, sigma_u
            same(
                "c(coef(re)[c('x1', 'x2')], sqrt(diag(vcov(re)))[c('x1', 'x2')])",
                "c(.791934204721017, .860827501504644, sqrt(.00749753294577665), \
                   sqrt(.00871157019311366))",
            ),
            same(
                "sqrt(plm::ercomp(re)$sigma2)",
                "c(1.03967201756119, .435188502337796)",
            ),
        ],
    );
}

#[test]
fn panel_unit_root_matches_xtunitroot() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    // Levin-Lin-Chu, 20 periods: the adjusted t* of xtunitroot llc, lags 0 to 2
    // with a mean, with a trend and with neither (Stata prints four decimals)
    let llc: &[(&str, &str, &str, f64)] = &[
        ("y", "intercept", "0", -4.3881),
        ("y", "intercept", "1", -3.2430),
        ("y", "intercept", "2", -2.1154),
        ("y", "trend", "0", -4.2999),
        ("y", "trend", "1", -2.4770),
        ("y", "trend", "2", -1.2919),
        ("y", "none", "0", -5.5837),
        ("y", "none", "1", -4.7758),
        ("w", "intercept", "0", -2.0885),
        ("w", "intercept", "1", -2.1256),
        ("w", "trend", "0", -1.4349),
        ("w", "trend", "1", -0.8843),
    ];
    for (var, terms, lags, stata) in llc {
        let (outcome, text) = dialog2(
            &mut e,
            ROOTS,
            &PANEL_UNIT_ROOT,
            &[
                ("var", var),
                ("id", "id"),
                ("time", "year"),
                ("terms", terms),
                ("lags", lags),
            ],
            &[],
        );
        assert_eq!(outcome, Outcome::Ok, "{text}");
        assert!(text.contains("Levin-Lin-Chu"), "{text}");
        let z = number_after(&text, "z = ");
        assert!(
            (z - stata).abs() < 1.5e-4,
            "{var} {terms} lags {lags}: {z} against Stata's {stata}\n{text}"
        );
    }
    // the same call at full precision: Stata's r(tds) and r(p_tds) for y with a
    // mean and one lag
    all_hold(
        &mut e,
        &[
            same(
                "plm::purtest(plm::pdata.frame(d, index = c('id', 'year'))$y, test = 'levinlin', \
                   exo = 'intercept', lags = 1, q = floor(3.21 * 20^(1/3)))$statistic$statistic",
                "-3.242977413477258",
            ),
            same(
                "plm::purtest(plm::pdata.frame(d, index = c('id', 'year'))$y, test = 'levinlin', \
                   exo = 'intercept', lags = 1, q = floor(3.21 * 20^(1/3)))$statistic$p.value",
                ".0005914379402105",
            ),
        ],
    );
    // 12 periods: the bandwidth floor(3.21 T^(1/3)) is 7, whatever the lags
    let short = format!("{ROOTS}; d <- d[d$year <= 12, ]");
    for (var, terms, lags, stata) in [
        ("y", "intercept", "0", -4.7660),
        ("y", "intercept", "1", -3.3619),
        ("y", "intercept", "2", -1.5407),
        ("w", "trend", "0", -6.2927),
        ("w", "trend", "1", -6.7787),
        ("w", "trend", "2", -5.2351),
    ] {
        let (outcome, text) = dialog2(
            &mut e,
            &short,
            &PANEL_UNIT_ROOT,
            &[
                ("var", var),
                ("id", "id"),
                ("time", "year"),
                ("terms", terms),
                ("lags", lags),
            ],
            &[],
        );
        assert_eq!(outcome, Outcome::Ok, "{text}");
        let z = number_after(&text, "z = ");
        assert!(
            (z - stata).abs() < 1.5e-4,
            "T=12 {var} {terms} lags {lags}: {z} against Stata's {stata}\n{text}"
        );
    }
    // Im-Pesaran-Shin: its W-t-bar, standardised with plm's moments, against
    // xtunitroot ips (W-t-bar -2.9180 for y with a mean and one lag): the
    // statistics differ a little (the known gap; plan/small-fixes.md)
    let (outcome, text) = dialog2(
        &mut e,
        ROOTS,
        &PANEL_UNIT_ROOT,
        &[
            ("var", "y"),
            ("id", "id"),
            ("time", "year"),
            ("test", "ips"),
        ],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(text.contains("Im-Pesaran-Shin"), "{text}");
    let w = number_after(&text, "Wtbar = ");
    assert!(
        (w - -2.9180).abs() < 0.05,
        "W-t-bar {w} against Stata's -2.9180\n{text}"
    );
    // an unbalanced panel is refused, not run
    let (outcome, text) = dialog2(
        &mut e,
        &format!("{ROOTS}; d <- d[-5, ]"),
        &PANEL_UNIT_ROOT,
        &[("var", "y"), ("id", "id"), ("time", "year")],
        &[],
    );
    assert_eq!(outcome, Outcome::Error, "{text}");
    assert!(text.contains("must be balanced"), "{text}");
}

#[test]
fn panel_serial_correlation_matches_wooldridge() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    let (outcome, text) = dialog2(
        &mut e,
        GAPPED,
        &PANEL_SERIAL,
        &[("y", "y"), ("id", "id"), ("time", "year")],
        &[("x", &["x", "z"])],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(
        text.contains("Wooldridge test for serial correlation in panel data")
            && text.contains("num df = 1, denom df = 39"),
        "{text}"
    );
    // the same test by another route: the differences matched on (unit, year - 1),
    // regress, residuals on their lag, Stata's cluster sandwich from sandwich::vcovCL
    // with its HC1 factor G/(G-1) (N-1)/(N-K)
    define(
        &mut e,
        "ow <- local({ prev <- d; prev$year <- prev$year + 1; \
           m <- merge(d, prev, by = c('id', 'year'), suffixes = c('', '.l')); \
           m$dy <- m$y - m$y.l; m$dx <- m$x - m$x.l; m$dz <- m$z - m$z.l; \
           fit <- lm(dy ~ 0 + dx + dz, m); m$e <- NA; m$e[as.integer(names(resid(fit)))] <- resid(fit); \
           prev2 <- m[, c('id', 'year', 'e')]; prev2$year <- prev2$year + 1; \
           k <- merge(m[, c('id', 'year', 'e')], prev2, by = c('id', 'year'), suffixes = c('', '.l')); \
           k <- k[!is.na(k$e) & !is.na(k$e.l), ]; \
           f2 <- lm(e ~ 0 + e.l, k); v <- sandwich::vcovCL(f2, cluster = k$id, type = 'HC1'); \
           fs <- unname((coef(f2) + 0.5)^2 / v[1, 1]); \
           c(nrow(k), fs, pf(fs, 1, length(unique(k$id)) - 1, lower.tail = FALSE)) })",
    );
    all_hold(
        &mut e,
        &[
            "ow[1] == 190".to_string(),
            // Stata 19: xtserial y x z (Stata Journal st0039), F(1, 39) and its p-value
            same("ow[2:3]", "c(22.2950973833785, .000030008483145379)"),
        ],
    );
    // the dialog's own F is the same number
    let f = number_after(&text, "F = ");
    assert!((f - 22.2951).abs() < 1.5e-4, "F = {f} in {text}");
}

#[test]
fn panel_models_have_the_model_menu() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    // the menu: Model fit and Robust standard errors on fixest and plm, Model fit on pgmm
    for class in ["fixest", "plm"] {
        assert!(postest::is_model_class(class));
        let entries: Vec<&str> = postest::model_specs(class)
            .iter()
            .map(|s| s.entry)
            .collect();
        assert_eq!(entries, ["Model fit", "Robust standard errors"], "{class}");
    }
    assert!(postest::is_model_class("pgmm"));
    assert_eq!(postest::model_specs("pgmm").len(), 1);
    // a pgmm on the dynamic data, then a feols and a plm on the unbalanced panel
    let (outcome, text) = dialog2(
        &mut e,
        DYNAMIC,
        &DYNAMIC_PANEL,
        &[("y", "y"), ("id", "id"), ("time", "year"), ("save", "gm1")],
        &[("x", &["x"])],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let (outcome, text) = dialog(
        &mut e,
        MAKE,
        &FIXED_EFFECTS_REGRESSION,
        &[("y", "wage"), ("save", "fe1")],
        &[("x", &["hours"]), ("fe", &["id"])],
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
            ("save", "pm1"),
        ],
        &[("x", &["hours"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    // the dialogs of the Objects pane's Model menu, on an object by name
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
    let (outcome, text) = on_model(&mut e, &MODEL_FIT, "fe1", &[]);
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(
        text.contains("Observations: 35") && text.contains("logLik") && text.contains("BIC"),
        "{text}"
    );
    let (outcome, text) = on_model(&mut e, &MODEL_FIT, "pm1", &[]);
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(
        text.contains("Observations: 35") && text.contains("R-squared") && text.contains("rsq"),
        "{text}"
    );
    let (outcome, text) = on_model(&mut e, &MODEL_FIT, "gm1", &[]);
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(text.contains("Observations: 200"), "{text}");
    // Robust standard errors on the feols: fixest's HC1 and a cluster formula
    let (outcome, text) = on_model(&mut e, &ROBUST, "fe1", &[]);
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(text.contains("HC1 robust standard errors"), "{text}");
    let (_, expected) = run(
        &mut e,
        Source::Typed,
        "v <- stats::vcov(fe1, vcov = 'hetero'); \
         cat('Coefficients with HC1 robust standard errors\\n'); \
         print(lmtest::coeftest(fe1, vcov. = v)); cat('\\nConfidence intervals\\n'); \
         print(lmtest::coefci(fe1, vcov. = v, level = 0.95))",
    );
    assert_eq!(report(&text), report(&expected), "{text}");
    let (outcome, text) = on_model(
        &mut e,
        &ROBUST,
        "fe1",
        &[("cluster", "firm"), ("data", "d")],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(text.contains("cluster-robust (by firm)"), "{text}");
    holds(
        &mut e,
        &[same(
            "sqrt(diag(stats::vcov(fe1, vcov = ~firm)))",
            "sqrt(diag(stats::vcov(fixest::feols(wage ~ hours | id, data = d, vcov = ~firm))))",
        )],
    );
    // and on the plm: Arellano's, clustered by the panel unit
    let (outcome, text) = on_model(&mut e, &ROBUST, "pm1", &[("type", "HC3")]);
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(
        text.contains("HC3 robust (Arellano, clustered by panel unit)"),
        "{text}"
    );
    let (_, expected) = run(
        &mut e,
        Source::Typed,
        "v <- plm::vcovHC(pm1, type = 'HC3'); \
         cat('Coefficients with HC3 robust (Arellano, clustered by panel unit) standard errors\\n'); \
         print(lmtest::coeftest(pm1, vcov. = v)); cat('\\nConfidence intervals\\n'); \
         print(lmtest::coefci(pm1, vcov. = v, level = 0.95))",
    );
    assert_eq!(report(&text), report(&expected), "{text}");
    // a plm model has its panel unit as the cluster already, a fixest one only HC1
    let (outcome, text) = on_model(
        &mut e,
        &ROBUST,
        "pm1",
        &[("cluster", "firm"), ("data", "d")],
    );
    assert_eq!(outcome, Outcome::Error, "{text}");
    assert!(
        text.contains("clustered by its panel unit already"),
        "{text}"
    );
    let (outcome, text) = on_model(&mut e, &ROBUST, "fe1", &[("type", "HC3")]);
    assert_eq!(outcome, Outcome::Error, "{text}");
    assert!(text.contains("HC1 correction only"), "{text}");
}
