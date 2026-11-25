//! Statistics menu multilevel models (P6.2) end to end against a real R
//! (skipped with a note when none is found, or when lme4 is not installed).
//! Each dialog's code is built from its spec and run on made-up data (100
//! pupils in 20 classes in 10 schools of 6 to 14, school intercepts and
//! slopes, two hours missing), and its numbers are checked three ways:
//!
//! - against the likelihoods written out here in plain R, with lme4's
//!   estimates put in: for a linear model V = s^2 (I + Z D Z') with Z built
//!   from the data, generalized least squares for the coefficients and their
//!   errors, and the ML or REML log-likelihood, which must be lme4's and flat
//!   at its estimate (so it is the maximum); for a logistic or Poisson model,
//!   the 7-point adaptive Gauss-Hermite likelihood (Golub-Welsch nodes, each
//!   school centred on its mode), likewise;
//! - against Stata 19's own results for the same data (`mixed`, with `reml`,
//!   random slopes independent and unstructured, a nested level; `melogit`
//!   and `mepoisson`), held as tight as Stata's own convergence allows: Stata
//!   stops sooner than lme4 does here (its variance components about 2e-7
//!   from the maximum written out, its errors 2e-8), and its quadrature
//!   centres on each school's posterior mean and variance where lme4's
//!   centres on the mode and curvature, which moves the maximum about 1e-7;
//! - and against lme4's report for the call written out by hand.
//!
//! The test records the R and lme4 versions it ran with (AGENTS.md section
//! 5), and checks R is the one `ENVIRONMENT` pins.

use std::time::{Duration, Instant};

use rcraft_catalog::multilevel::{GENERALIZED_MIXED, ICC, LINEAR_MIXED, TMB_MIXED};
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

/// Whether glmmTMB is installed (its version is recorded).
fn has_glmmtmb(e: &mut Engine) -> bool {
    let (outcome, text) = run(
        e,
        Source::Typed,
        "cat(if (requireNamespace('glmmTMB', quietly = TRUE)) \
         as.character(packageVersion('glmmTMB')) else 'none')",
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    if text.trim() == "none" {
        eprintln!("skipped: glmmTMB is not installed");
        return false;
    }
    eprintln!("ran with glmmTMB {}", text.trim());
    true
}

/// The R and lme4 versions, recorded; false when lme4 is not installed.
fn versions(e: &mut Engine) -> bool {
    let (outcome, text) = run(
        e,
        Source::Typed,
        "cat(paste(R.version$major, R.version$minor, sep = '.'), \
         if (requireNamespace('lme4', quietly = TRUE)) as.character(packageVersion('lme4')) else 'none')",
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let mut words = text.split_whitespace();
    let (r, lme4) = (words.next().unwrap_or(""), words.next().unwrap_or("none"));
    if lme4 == "none" {
        eprintln!("skipped: lme4 is not installed");
        return false;
    }
    let pinned = include_str!("../../../ENVIRONMENT")
        .lines()
        .find_map(|l| l.strip_prefix("R_VERSION="))
        .unwrap_or("")
        .trim();
    eprintln!("ran with R {r} and lme4 {lme4} (ENVIRONMENT pins R {pinned})");
    if r != pinned {
        eprintln!("note: this R is not the pinned one, so the run is not a Verified one");
    }
    true
}

/// 100 pupils in 10 schools of 6 to 14, two classes in each; school
/// intercepts and hours slopes, class intercepts; hours missing in rows 5
/// and 40; a 0/1 pass and counts of absences over 20 to 40 days.
const MAKE: &str = "d <- local({ \
     set.seed(6201); sizes <- c(8, 12, 6, 10, 9, 14, 7, 11, 10, 13); \
     school <- rep(seq_along(sizes), sizes); n <- length(school); \
     class <- paste0(school, c('a', 'b')[1 + (seq_len(n) %% 2)]); \
     u0 <- rnorm(10, sd = 1.2)[school]; u1 <- rnorm(10, sd = 0.4)[school]; \
     c0 <- rnorm(20, sd = 0.6)[match(class, unique(class))]; \
     hours <- round(rnorm(n, 5, 2), 2); girl <- as.numeric((seq_len(n) * 7) %% 3 == 0); \
     score <- round(50 + 2 * hours + 3 * girl + u0 * 4 + u1 * hours + c0 * 2 + rnorm(n, sd = 3), 2); \
     hours[c(5, 40)] <- NA; \
     eta <- -2 + 0.35 * ifelse(is.na(hours), 5, hours) + 0.5 * girl + u0 * 0.9; \
     passed <- as.numeric(runif(n) < plogis(eta)); \
     days <- c(20, 30, 40, 25)[1 + seq_len(n) %% 4]; \
     absences <- rpois(n, days / 30 * exp(0.3 + 0.05 * ifelse(is.na(hours), 5, hours) + 0.5 * u0)); \
     set.seed(6202); rater <- 1 + (seq_len(n) * 5) %% 7; r0 <- rnorm(7, sd = 1.5)[rater]; \
     score2 <- round(score + r0 * 3, 2); passed2 <- as.numeric(runif(n) < plogis(eta + r0 * 0.8)); \
     mu <- days / 30 * exp(0.2 + 0.05 * ifelse(is.na(hours), 5, hours) + 0.6 * u0); \
     hc <- ifelse(is.na(hours), 5, hours) - 5; eta3 <- -1 + 0.3 * hc + 0.5 * girl + u0 * 0.9 + u1 * 2.2 * hc; \
     passed3 <- as.numeric(runif(n) < plogis(eta3)); \
     count3 <- rpois(n, days / 30 * exp(0.3 + 0.1 * hc + 0.4 * u0 + u1 * 2 * hc)); \
     visits <- rnbinom(n, mu = mu * 3, size = 2); zvisits <- ifelse(runif(n) < 0.3, 0, rpois(n, mu * 3)); \
     data.frame(score, hours, girl, school, class, passed, absences, days, rater, score2, passed2, \
       visits, zvisits, passed3, count3) })";

/// 633 pupils in 40 schools of 12 to 20, for the models Stata needs more
/// data to fit (random slopes in a logistic model, a crossed logistic): school
/// intercepts and slopes in `hc` (hours centred at 5), nine raters crossing
/// the schools, counts over 20 to 40 days.
const MAKE_BIG: &str = "big <- local({ \
     set.seed(6203); sizes <- c(14, 18, 12, 20, 16, 13, 19, 15, 17, 12, 20, 14, 16, 18, 13, 15, 19, 12, 17, 16, \
       14, 18, 13, 20, 15, 12, 17, 19, 14, 16, 18, 13, 15, 20, 12, 17, 16, 14, 19, 15); \
     school <- rep(seq_along(sizes), sizes); n <- length(school); \
     u0 <- rnorm(40, sd = 1)[school]; u1 <- rnorm(40, sd = 0.5)[school]; \
     hours <- round(rnorm(n, 5, 2), 2); hc <- hours - 5; girl <- as.numeric((seq_len(n) * 7) %% 3 == 0); \
     days <- c(20, 30, 40, 25)[1 + seq_len(n) %% 4]; rater <- 1 + (seq_len(n) * 5) %% 9; \
     r0 <- rnorm(9, sd = 0.8)[rater]; \
     passed <- as.numeric(runif(n) < plogis(-0.5 + 0.3 * hc + 0.5 * girl + u0 + u1 * hc)); \
     count <- rpois(n, days / 30 * exp(0.3 + 0.1 * hc + 0.4 * u0 + u1 * hc)); \
     passed2 <- as.numeric(runif(n) < plogis(-0.5 + 0.3 * hc + 0.5 * girl + u0 + r0)); \
     data.frame(school, hours, hc, girl, days, rater, passed, count, passed2) })";

/// The likelihoods in plain R on the complete rows `k`. `lmm(Vrel, reml)`:
/// GLS and the profiled log-likelihood for V / s^2 = `Vrel`. `S`, `C` the
/// school and class indicators. `agq(th, family, off)`: the 7-point adaptive
/// Gauss-Hermite log-likelihood of a random-intercept model, `th` = (the
/// coefficients, log sd). `slope(f, th)`: the largest central-difference
/// derivative of `f` at `th`.
const ORACLE: &str = "k <- d[!is.na(d$hours), ]; X <- cbind(1, k$hours, k$girl); n <- nrow(k); \
     S <- model.matrix(~ factor(school) - 1, k); C <- model.matrix(~ factor(class) - 1, k); \
     lmm <- function(Vrel, reml = FALSE, y = k$score) { Vi <- solve(Vrel); \
       XVX <- crossprod(X, Vi %*% X); b <- solve(XVX, crossprod(X, Vi %*% y)); r <- y - X %*% b; \
       q <- drop(crossprod(r, Vi %*% r)); m <- if (reml) n - 3 else n; s2 <- q / m; \
       ll <- -0.5 * (m * log(2 * pi * s2) + determinant(Vrel)$modulus + m + \
         (if (reml) determinant(XVX)$modulus else 0)); \
       list(b = drop(b), se = sqrt(diag(solve(XVX)) * s2), s2 = s2, ll = as.numeric(ll)) }; \
     slope <- function(f, th) max(abs(vapply(seq_along(th), function(j) { h <- 1e-5 * max(1, abs(th[j])); \
       up <- th; up[j] <- up[j] + h; dn <- th; dn[j] <- dn[j] - h; (f(up) - f(dn)) / (2 * h) }, 0))); \
     gh <- local({ J <- matrix(0, 7, 7); for (i in 1:6) J[i, i + 1] <- J[i + 1, i] <- sqrt(i / 2); \
       e <- eigen(J, symmetric = TRUE); list(x = e$values, w = sqrt(pi) * e$vectors[1, ]^2) }); \
     agq <- function(th, family, off = 0) { b <- th[1:3]; su <- exp(th[4]); total <- 0; \
       y <- if (family == 'logit') k$passed else k$absences; \
       for (s in unique(k$school)) { i <- k$school == s; yi <- y[i]; \
         e0 <- as.vector(X[i, , drop = FALSE] %*% b) + (if (length(off) > 1) off[i] else 0); \
         h <- function(u) { e <- e0 + u; (if (family == 'logit') sum(yi * e - log1p(exp(e))) else \
           sum(yi * e - exp(e) - lgamma(yi + 1))) + dnorm(u, 0, su, log = TRUE) }; \
         m <- optimize(function(u) -h(u), c(-20, 20), tol = 1e-14)$minimum; dd <- 1e-4; \
         for (it in 1:6) m <- m - ((h(m + dd) - h(m - dd)) / (2 * dd)) / ((h(m + dd) - 2 * h(m) + h(m - dd)) / dd^2); \
         sdv <- sqrt(-1 / ((h(m + dd) - 2 * h(m) + h(m - dd)) / dd^2)); \
         v <- vapply(gh$x, function(x) h(m + sqrt(2) * sdv * x) + x^2, 0); mx <- max(v); \
         total <- total + mx + log(sum(gh$w * exp(v - mx))) + log(sqrt(2) * sdv) }; total }";

fn context() -> Context {
    Context {
        dataset: "d".into(),
        frames: Vec::new(),
        columns: [
            ("score", VarKind::Number),
            ("hours", VarKind::Number),
            ("girl", VarKind::Number),
            ("school", VarKind::Number),
            ("class", VarKind::Text),
            ("passed", VarKind::Number),
            ("absences", VarKind::Number),
            ("days", VarKind::Number),
            ("rater", VarKind::Number),
            ("score2", VarKind::Number),
            ("passed2", VarKind::Number),
            ("visits", VarKind::Number),
            ("zvisits", VarKind::Number),
            ("passed3", VarKind::Number),
            ("count3", VarKind::Number),
        ]
        .into_iter()
        .map(|(name, kind)| ColumnInfo {
            name: name.into(),
            kind,
        })
        .collect(),
    }
}

fn context_big() -> Context {
    Context {
        dataset: "big".into(),
        frames: Vec::new(),
        columns: [
            ("school", VarKind::Number),
            ("hours", VarKind::Number),
            ("hc", VarKind::Number),
            ("girl", VarKind::Number),
            ("days", VarKind::Number),
            ("rater", VarKind::Number),
            ("passed", VarKind::Number),
            ("count", VarKind::Number),
            ("passed2", VarKind::Number),
        ]
        .into_iter()
        .map(|(name, kind)| ColumnInfo {
            name: name.into(),
            kind,
        })
        .collect(),
    }
}

/// Makes the data, then runs the dialog's code on it. The "ratios" box is on;
/// a `set` entry whose text is "on" turns that check box on.
fn dialog(
    e: &mut Engine,
    spec: &Spec,
    set: &[(&str, &str)],
    lists: &[(&str, &[&str])],
) -> (Outcome, String) {
    dialog_on(e, MAKE, &context(), spec, set, lists)
}

/// The same on the larger data frame `big`.
fn dialog_big(
    e: &mut Engine,
    spec: &Spec,
    set: &[(&str, &str)],
    lists: &[(&str, &[&str])],
) -> (Outcome, String) {
    dialog_on(e, MAKE_BIG, &context_big(), spec, set, lists)
}

fn dialog_on(
    e: &mut Engine,
    make: &str,
    ctx: &Context,
    spec: &Spec,
    set: &[(&str, &str)],
    lists: &[(&str, &[&str])],
) -> (Outcome, String) {
    let (made, text) = run(e, Source::Typed, make);
    assert_eq!(made, Outcome::Ok, "{text}");
    let mut values = spec.defaults(ctx);
    for (id, text) in set {
        if *text == "on" {
            values.set_flag(id, true);
        } else {
            values.set_text(id, *text);
        }
    }
    for (id, names) in lists {
        values.set_list(id, names.iter().map(|n| n.to_string()).collect());
    }
    for w in spec.widgets {
        if let Widget::Check { id, .. } = w {
            if *id == "ratios" {
                values.set_flag(id, true);
            }
        }
    }
    let code = spec.generate(ctx, &values, Style::Base);
    assert!(code.is_ok(), "{code:?}");
    let code = code.unwrap_or_default();
    run(e, Source::Menu(spec.menu.into()), &code)
}

fn report(text: &str) -> Vec<String> {
    text.lines()
        .filter(|l| {
            let l = l.trim_start();
            !(l.starts_with("Formula:")
                || l.starts_with("Data:")
                || l.starts_with("Control:")
                || l.starts_with("lme4::")
                || l.starts_with("warning:"))
        })
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|l| !l.is_empty())
        .collect()
}

fn holds(e: &mut Engine, exprs: &[String]) {
    let exprs = exprs.join(", ");
    let (outcome, text) = run(e, Source::Typed, &format!("stopifnot({exprs})"));
    assert_eq!(outcome, Outcome::Ok, "{exprs}\n{text}");
}

fn define(e: &mut Engine, code: &str) {
    let (outcome, text) = run(e, Source::Typed, code);
    assert_eq!(outcome, Outcome::Ok, "{code}\n{text}");
}

fn within(a: &str, b: &str, tol: &str) -> String {
    format!("isTRUE(all.equal(unname(as.vector({a})), unname(as.vector({b})), tolerance = {tol}))")
}

fn same(a: &str, b: &str) -> String {
    within(a, b, "1e-9")
}

/// lme4's coefficients and errors in Stata's order (hours, girl, _cons).
fn b_se(m: &str) -> String {
    format!("c(lme4::fixef({m})[c(2, 3, 1)], sqrt(diag(as.matrix(vcov({m}))))[c(2, 3, 1)])")
}

/// For a linear fit `m` whose V / s^2 the R function `vrel(th)` builds from
/// `th` (log relative sds, and so on), with `th0` lme4's estimate: the
/// hand-written GLS and log-likelihood are lme4's there, and flat.
fn linear_holds(e: &mut Engine, m: &str, vrel: &str, th0: &str, reml: bool) {
    let reml = if reml { "TRUE" } else { "FALSE" };
    define(
        e,
        &format!("vrel <- {vrel}; th0 <- {th0}; hand <- lmm(vrel(th0), {reml})"),
    );
    holds(
        e,
        &[
            same(&format!("lme4::fixef({m})"), "hand$b"),
            same(&format!("sqrt(diag(as.matrix(vcov({m}))))"), "hand$se"),
            same(&format!("sigma({m})^2"), "hand$s2"),
            within(&format!("logLik({m})"), "hand$ll", "1e-12"),
            format!("slope(function(t) lmm(vrel(t), {reml})$ll, th0) < 1e-5"),
        ],
    );
}

#[test]
fn random_intercepts_are_mixeds() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    let (outcome, text) = dialog(
        &mut e,
        &LINEAR_MIXED,
        &[("y", "score"), ("group", "school")],
        &[("x", &["hours", "girl"])],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let (_, expected) = run(
        &mut e,
        Source::Typed,
        "m <- lme4::lmer(score ~ hours + girl + (1 | school), data = d, REML = FALSE, \
         control = lme4::lmerControl(optCtrl = list(ftol_abs = 1e-14, ftol_rel = 1e-14, \
         xtol_abs = 1e-14, xtol_rel = 1e-14, maxeval = 1e5))); print(summary(m)); \
         print(stats::confint(m, parm = 'beta_', method = 'Wald', level = 0.95))",
    );
    assert!(report(&expected).len() > 12, "{expected}");
    assert_eq!(report(&text), report(&expected), "{text}");
    let (outcome, text) = dialog(
        &mut e,
        &LINEAR_MIXED,
        &[
            ("y", "score"),
            ("group", "school"),
            ("method", "reml"),
            ("save", "rm"),
        ],
        &[("x", &["hours", "girl"])],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    define(&mut e, ORACLE);
    let vrel = "function(t) diag(n) + exp(2 * t[1]) * tcrossprod(S)";
    let th = |m: &str| format!("log(attr(lme4::VarCorr({m})$school, 'stddev') / sigma({m}))");
    linear_holds(&mut e, "model", vrel, &th("model"), false);
    linear_holds(&mut e, "rm", vrel, &th("rm"), true);
    // log sd of the school intercepts and of the residual, as Stata reports
    let logsd = |m: &str| format!("log(c(attr(lme4::VarCorr({m})$school, 'stddev'), sigma({m})))");
    holds(
        &mut e,
        &[
            // Stata 19: mixed score hours girl || school:, and with reml
            within(
                &b_se("model"),
                "c(2.12591225639607, 2.9531996936819, 51.0804688466092, \
                   .165069509513253, .700720558384027, 1.26682052531957)",
                "1e-7",
            ),
            within(
                &logsd("model"),
                "c(1.01718165533462, 1.18086131617354)",
                "1e-6",
            ),
            within("logLik(model)", "-265.077799247152", "1e-12"),
            within(
                &b_se("rm"),
                "c(2.12725158979798, 2.95106045367373, 51.0714976572806, \
                   .16704710000607, .708812700624921, 1.31209969452714)",
                "1e-7",
            ),
            within(
                &logsd("rm"),
                "c(1.07657278659455, 1.19228635687566)",
                "1e-6",
            ),
            within("logLik(rm)", "-264.505120173114", "1e-12"),
        ],
    );
}

#[test]
fn random_slopes_and_nesting_are_mixeds() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    for (set, slopes, save) in [
        (vec![], vec!["hours"], "ind"),
        (vec![("covariance", "unstructured")], vec!["hours"], "un"),
        (vec![("inner", "class")], vec![], "nest"),
    ] {
        let mut all = vec![("y", "score"), ("group", "school"), ("save", save)];
        all.extend(set);
        let (outcome, text) = dialog(
            &mut e,
            &LINEAR_MIXED,
            &all,
            &[("x", &["hours", "girl"]), ("slopes", &slopes)],
        );
        assert_eq!(outcome, Outcome::Ok, "{text}");
    }
    define(&mut e, ORACLE);
    // independent: intercept and slope variances; unstructured: the
    // Cholesky factor of their 2 x 2 covariance; nested: school and class
    linear_holds(
        &mut e,
        "ind",
        "function(t) diag(n) + exp(2 * t[1]) * tcrossprod(S) + exp(2 * t[2]) * tcrossprod(S * k$hours)",
        "local({ v <- lme4::VarCorr(ind); \
           log(c(attr(v$school, 'stddev'), attr(v$school.1, 'stddev')) / sigma(ind)) })",
        false,
    );
    linear_holds(
        &mut e,
        "un",
        "function(t) { L <- matrix(c(exp(t[1]), t[3], 0, exp(t[2])), 2); Z <- cbind(S, S * k$hours); \
           diag(n) + Z %*% kronecker(tcrossprod(L), diag(10)) %*% t(Z) }",
        "local({ L <- t(chol(as.matrix(lme4::VarCorr(un)$school) / sigma(un)^2)); \
           c(log(diag(L)), L[2, 1]) })",
        false,
    );
    linear_holds(
        &mut e,
        "nest",
        "function(t) diag(n) + exp(2 * t[1]) * tcrossprod(S) + exp(2 * t[2]) * tcrossprod(C)",
        "local({ v <- lme4::VarCorr(nest); \
           log(c(attr(v$school, 'stddev'), attr(v[['school:class']], 'stddev')) / sigma(nest)) })",
        false,
    );
    holds(
        &mut e,
        &[
            // Stata 19: mixed score hours girl || school: hours (independent)
            within(
                &b_se("ind"),
                "c(2.04616600763448, 2.78677422828186, 51.602007641715, \
                   .220479849681639, .665403639142335, .952455085780994)",
                "1e-6",
            ),
            within(
                "log(c(attr(lme4::VarCorr(ind)$school.1, 'stddev'), \
                   attr(lme4::VarCorr(ind)$school, 'stddev'), sigma(ind)))",
                "c(-.729452703269641, .208377953429131, 1.12337014662148)",
                // Stata's log sd of the intercepts stops 6e-6 from the
                // maximum written out (lme4's is on it): a direction so flat
                // that Stata's own error for it is 0.92
                "1e-4",
            ),
            within("logLik(ind)", "-260.83865938711", "1e-12"),
            // ..., covariance(unstructured): sds, and atanh of the correlation
            within(
                &b_se("un"),
                "c(2.0089986022313, 2.67291252386332, 51.8655190596525, \
                   .246258333764376, .660824230572537, 1.09237075120649)",
                "1e-6",
            ),
            within(
                "local({ v <- lme4::VarCorr(un)$school; \
                   c(log(attr(v, 'stddev')[2:1]), atanh(attr(v, 'correlation')[1, 2]), log(sigma(un))) })",
                "c(-.526700276058085, .717794626068469, -.555955881338963, 1.10679352429205)",
                "1e-5",
            ),
            within("logLik(un)", "-260.636718901557", "1e-12"),
            // mixed score hours girl || school: || class:
            within(
                &b_se("nest"),
                "c(2.04618839928174, 2.90454403682982, 51.4592463425041, \
                   .165360199220347, .665590506663274, 1.26590540757636)",
                "1e-6",
            ),
            within(
                "log(c(attr(lme4::VarCorr(nest)$school, 'stddev'), \
                   attr(lme4::VarCorr(nest)[['school:class']], 'stddev'), sigma(nest)))",
                "c(.956585117510829, .34634445201459, 1.12763784792523)",
                "1e-6",
            ),
            within("logLik(nest)", "-263.863632719444", "1e-12"),
        ],
    );
}

#[test]
fn logistic_and_poisson_are_melogit_and_mepoisson() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    let (outcome, text) = dialog(
        &mut e,
        &GENERALIZED_MIXED,
        &[("y", "passed"), ("group", "school")],
        &[("x", &["hours", "girl"])],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(text.contains("Odds ratios"), "{text}");
    let (outcome, text) = dialog(
        &mut e,
        &GENERALIZED_MIXED,
        &[
            ("y", "absences"),
            ("family", "poisson"),
            ("group", "school"),
            ("exposure", "days"),
            ("save", "po"),
        ],
        &[("x", &["hours", "girl"])],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(text.contains("Rate ratios"), "{text}");
    define(&mut e, ORACLE);
    define(
        &mut e,
        "th <- function(m) c(lme4::fixef(m), log(attr(lme4::VarCorr(m)$school, 'stddev'))); \
         var_of <- function(m) attr(lme4::VarCorr(m)$school, 'stddev')^2; \
         y <- k$absences; saturated <- sum(ifelse(y > 0, y * log(y), 0) - y - lgamma(y + 1))",
    );
    holds(
        &mut e,
        &[
            // the quadrature written out is lme4's likelihood, and flat at
            // its estimate. For an AGQ fit lme4's logLik leaves out the
            // saturated model's log-likelihood (zero for 0/1 outcomes)
            within("agq(th(model), 'logit')", "logLik(model)", "1e-10"),
            "slope(function(t) agq(t, 'logit'), th(model)) < 1e-5".to_string(),
            within(
                "agq(th(po), 'pois', log(k$days))",
                "logLik(po) + saturated",
                "1e-10",
            ),
            "slope(function(t) agq(t, 'pois', log(k$days)), th(po)) < 1e-5".to_string(),
            // Stata 19: melogit passed hours girl || school: (7 points);
            // mepoisson absences hours girl, exposure(days) || school:.
            // Stata's quadrature centres differently, and its Poisson stops
            // about 2e-5 short in a school variance whose error is larger
            // than itself
            within(
                &b_se("model"),
                "c(.277790171916837, .783077890347478, -1.43541427638319, \
                   .112433358804262, .470485731935358, .631466953638979)",
                "1e-6",
            ),
            within("var_of(model)", ".175245143519442", "1e-6"),
            within("logLik(model)", "-62.9290342942785", "1e-9"),
            within(
                &b_se("po"),
                "c(.0428806194460276, .341048780151274, -3.20838924391485, \
                   .0389995586254279, .160205818866847, .227013739231678)",
                "1e-6",
            ),
            within("var_of(po)", ".0319178387120046", "1e-4"),
            within("logLik(po) + saturated", "-152.60983787822", "1e-9"),
        ],
    );
}

// ---- the second half of P6.2: glmmTMB, intraclass correlation, slopes in
// glmer, crossed effects, random-effect predictions ----

/// The Laplace approximation written out in plain R for a zero-inflated
/// Poisson model with a random intercept on the complete rows `k` (one
/// probability of an excess zero for every row, an offset of log(days)):
/// `zlap(th)`, `th` = (the three coefficients, the logit of the zero
/// probability, the log sd of the school intercepts). Each school's integral
/// over its intercept is exp(h(m)) sqrt(2 pi / -h''(m)) at the mode m of the
/// integrand.
const ZIP_ORACLE: &str = "zlap <- function(th) { b <- th[1:3]; p <- plogis(th[4]); su <- exp(th[5]); \
     off <- log(k$days); total <- 0; \
     for (s in unique(k$school)) { i <- k$school == s; yi <- k$zvisits[i]; \
       e0 <- as.vector(X[i, , drop = FALSE] %*% b) + off[i]; \
       h <- function(u) { mu <- exp(e0 + u); \
         sum(ifelse(yi == 0, log(p + (1 - p) * exp(-mu)), log(1 - p) + dpois(yi, mu, log = TRUE))) + \
           dnorm(u, 0, su, log = TRUE) }; \
       m <- optimize(function(u) -h(u), c(-20, 20), tol = 1e-14)$minimum; dd <- 1e-4; \
       for (it in 1:6) m <- m - ((h(m + dd) - h(m - dd)) / (2 * dd)) / ((h(m + dd) - 2 * h(m) + h(m - dd)) / dd^2); \
       d2 <- (h(m + dd) - 2 * h(m) + h(m - dd)) / dd^2; \
       total <- total + h(m) + 0.5 * log(2 * pi / -d2) }; total }";

/// glmmTMB's coefficients and errors in Stata's order (hours, girl, _cons;
/// for a model with the offset or slopes the same three).
fn tmb_b_se(m: &str) -> String {
    format!(
        "c(glmmTMB::fixef({m})$cond[c(2, 3, 1)], \
         sqrt(diag(as.matrix(vcov({m})$cond)))[c(2, 3, 1)])"
    )
}

/// The variance of the school intercepts of a glmmTMB model.
fn tmb_var(m: &str) -> String {
    format!("attr(glmmTMB::VarCorr({m})$cond$school, 'stddev')[['(Intercept)']]^2")
}

#[test]
fn mixed_model_glmmtmb_matches_stata() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) || !has_glmmtmb(&mut e) {
        return;
    }
    // linear: Stata 19 mixed score hours girl || school: (the numbers of
    // random_intercepts_are_mixeds), by ML
    let (outcome, text) = dialog(
        &mut e,
        &TMB_MIXED,
        &[("y", "score"), ("group", "school"), ("save", "g")],
        &[("x", &["hours", "girl"])],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(text.contains("Conditional model"), "{text}");
    holds(
        &mut e,
        &[
            // (the errors are not compared: glmmTMB's come from the full
            // Hessian, Stata's mixed from the generalised least squares
            // covariance at the estimated variances, 1e-3 apart)
            within(
                "glmmTMB::fixef(g)$cond[c(2, 3, 1)]",
                "c(2.12591225639607, 2.9531996936819, 51.0804688466092)",
                "2e-6",
            ),
            within(
                "log(c(attr(glmmTMB::VarCorr(g)$cond$school, 'stddev'), sigma(g)))",
                "c(1.01718165533462, 1.18086131617354)",
                "1e-6",
            ),
            within("logLik(g)", "-265.077799247152", "1e-9"),
        ],
    );
    // logistic by Laplace: Stata 19 melogit passed hours girl || school:,
    // intmethod(laplace)
    let (outcome, text) = dialog(
        &mut e,
        &TMB_MIXED,
        &[
            ("y", "passed"),
            ("family", "binomial"),
            ("group", "school"),
            ("save", "lg"),
        ],
        &[("x", &["hours", "girl"])],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    holds(
        &mut e,
        &[
            within(
                &tmb_b_se("lg"),
                "c(.277521967242073, .782679267060284, -1.43544634494154, \
                   .112291776480294, .470168713139983, .629888152390397)",
                "1e-5",
            ),
            within(&tmb_var("lg"), ".165253528762341", "1e-5"),
            within("logLik(lg)", "-62.9441702801437", "1e-8"),
        ],
    );
    // negative binomial with an exposure: Stata 19 menbreg visits hours girl,
    // exposure(days) || school:, intmethod(laplace); its ln(alpha) is minus
    // the log of glmmTMB's theta
    let (outcome, text) = dialog(
        &mut e,
        &TMB_MIXED,
        &[
            ("y", "visits"),
            ("family", "nbinom2"),
            ("group", "school"),
            ("exposure", "days"),
            ("save", "nb"),
        ],
        &[("x", &["hours", "girl"])],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    holds(
        &mut e,
        &[
            within(
                "glmmTMB::fixef(nb)$cond[c(2, 3, 1)]",
                "c(.0710557349074445, -.0863522576582768, -2.12311952810286)",
                "1e-5",
            ),
            // the errors are 2e-3 apart: menbreg's against glmmTMB's full
            // Hessian (the Poisson and logistic ones agree to 1e-6)
            within(
                "sqrt(diag(as.matrix(vcov(nb)$cond)))[c(2, 3, 1)]",
                "c(.04666615071737, .196329992644892, .29694379240728)",
                "5e-3",
            ),
            within("-log(sigma(nb))", "-.57405783506231", "1e-5"),
            within(&tmb_var("nb"), ".191769497749425", "1e-4"),
            within("logLik(nb)", "-260.736159801754", "1e-8"),
        ],
    );
    // Poisson with an exposure and independent random slopes, on the larger
    // data: Stata 19 mepoisson count hours girl, exposure(days) ||
    // school: hours, intmethod(laplace)
    let (outcome, text) = dialog_big(
        &mut e,
        &TMB_MIXED,
        &[
            ("y", "count"),
            ("family", "poisson"),
            ("group", "school"),
            ("exposure", "days"),
            ("save", "ps"),
        ],
        &[("x", &["hours", "girl"]), ("slopes", &["hours"])],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    holds(
        &mut e,
        &[
            within(
                &tmb_b_se("ps"),
                "c(.280170418987826, .00923305523835184, -4.63055478633812, \
                   .0701397661704667, .0636317978683384, .356873971131259)",
                "1e-5",
            ),
            within(
                "unname(attr(glmmTMB::VarCorr(ps)$cond$school, 'stddev')[c('hours', '(Intercept)')]^2)",
                "c(.179544492073569, 4.4257257126714)",
                "1e-4",
            ),
            within("logLik(ps)", "-1038.65609871803", "1e-8"),
        ],
    );
    // zero-inflation: the model's log-likelihood is the Laplace
    // approximation of the zero-inflated Poisson likelihood written out here,
    // and flat at its estimate (so it is the maximum)
    let (outcome, text) = dialog(
        &mut e,
        &TMB_MIXED,
        &[
            ("y", "zvisits"),
            ("family", "poisson"),
            ("group", "school"),
            ("exposure", "days"),
            ("zi", "on"),
            ("save", "z"),
        ],
        &[("x", &["hours", "girl"])],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(text.contains("Zero-inflation model"), "{text}");
    define(&mut e, ORACLE);
    define(&mut e, ZIP_ORACLE);
    define(
        &mut e,
        "th <- c(glmmTMB::fixef(z)$cond, glmmTMB::fixef(z)$zi, \
         log(attr(glmmTMB::VarCorr(z)$cond$school, 'stddev')))",
    );
    holds(
        &mut e,
        &[
            within("zlap(th)", "logLik(z)", "1e-7"),
            // higher than 0.02 away along each parameter, both ways (the
            // oracle's own finite differences are too coarse for a slope)
            "all(vapply(seq_along(th), function(j) { s <- numeric(length(th)); s[j] <- 0.02; \
             zlap(th) > zlap(th + s) && zlap(th) > zlap(th - s) }, TRUE))"
                .to_string(),
        ],
    );
    // the dialog refuses zero-inflation for a model that is not for counts
    let ctx = context();
    let mut v = TMB_MIXED.defaults(&ctx);
    v.set_text("y", "score");
    v.set_text("group", "school");
    v.set_flag("zi", true);
    assert!(TMB_MIXED
        .problems(&ctx, &v)
        .iter()
        .any(|p| p.widget == "zi"));
}

/// The rows of the intraclass correlation table as (correlation, standard
/// error, lower and upper limits); the table prints seven digits.
fn icc_rows(text: &str) -> Vec<[f64; 4]> {
    text.lines()
        .skip_while(|l| !l.trim_start().starts_with("level"))
        .skip(1)
        .filter_map(|l| {
            let t: Vec<f64> = l
                .split_whitespace()
                .skip(3)
                .filter_map(|w| w.parse().ok())
                .collect();
            Some([*t.first()?, *t.get(1)?, *t.get(2)?, *t.get(3)?])
        })
        .collect()
}

/// Whether row `row` of the table is Stata's (`estat icc` after the same
/// model): the correlation to 2e-6 (Stata stops a little short of lme4), the
/// error and the interval to 1e-4 (the delta method on a finite-difference
/// Hessian).
fn icc_is(text: &str, row: usize, want: [f64; 4]) {
    let rows = icc_rows(text);
    let got = rows.get(row);
    assert!(got.is_some(), "no row {row} in {text}");
    let got = got.copied().unwrap_or_default();
    for (i, (g, w)) in got.iter().zip(want).enumerate() {
        let tol = if i == 0 { 2e-6 } else { 1e-4 };
        assert!(
            (g - w).abs() < tol,
            "row {row} number {i}: {g} against {w} in {text}"
        );
    }
}

/// Runs the intraclass correlation dialog on the fitted model `name`.
fn icc_of(e: &mut Engine, name: &str) -> (Outcome, String) {
    let mut ctx = context();
    ctx.dataset = name.into();
    let values = ICC.defaults(&ctx);
    let code = ICC.generate(&ctx, &values, Style::Base);
    assert!(code.is_ok(), "{code:?}");
    run(e, Source::Menu(ICC.menu.into()), &code.unwrap_or_default())
}

#[test]
fn intraclass_correlation_matches_estat_icc() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    let x: &[(&str, &[&str])] = &[("x", &["hours", "girl"])];
    let fit = |e: &mut Engine, spec: &Spec, set: &[(&str, &str)], lists: &[(&str, &[&str])]| {
        let (outcome, text) = dialog(e, spec, set, lists);
        assert_eq!(outcome, Outcome::Ok, "{text}");
    };
    fit(
        &mut e,
        &LINEAR_MIXED,
        &[("y", "score"), ("group", "school"), ("save", "m1")],
        x,
    );
    // Stata 19: mixed score hours girl || school:, then estat icc
    let (outcome, text) = icc_of(&mut e, "m1");
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert_eq!(icc_rows(&text).len(), 1, "{text}");
    icc_is(
        &text,
        0,
        [
            0.418883279247883,
            0.132071024971427,
            0.19928555242101,
            0.676130842568928,
        ],
    );
    // ... , reml
    fit(
        &mut e,
        &LINEAR_MIXED,
        &[
            ("y", "score"),
            ("group", "school"),
            ("method", "reml"),
            ("save", "m1r"),
        ],
        x,
    );
    let (outcome, text) = icc_of(&mut e, "m1r");
    assert_eq!(outcome, Outcome::Ok, "{text}");
    icc_is(
        &text,
        0,
        [
            0.442400066277588,
            0.138826628338768,
            0.208424882102919,
            0.705076810649232,
        ],
    );
    // ... || school: || class:, whose class level adds the school's variance
    fit(
        &mut e,
        &LINEAR_MIXED,
        &[
            ("y", "score"),
            ("group", "school"),
            ("inner", "class"),
            ("save", "m2"),
        ],
        x,
    );
    let (outcome, text) = icc_of(&mut e, "m2");
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(text.contains("school>class"), "{text}");
    assert_eq!(icc_rows(&text).len(), 2, "{text}");
    icc_is(
        &text,
        0,
        [
            0.3699597333452,
            0.15367099226625,
            0.138888525962294,
            0.681304491700294,
        ],
    );
    icc_is(
        &text,
        1,
        [
            0.479130447279441,
            0.125610659144746,
            0.255400265071443,
            0.711557679810908,
        ],
    );
    // ... || school: hours: the correlation where hours is zero
    fit(
        &mut e,
        &LINEAR_MIXED,
        &[("y", "score"), ("group", "school"), ("save", "m3")],
        &[("x", &["hours", "girl"]), ("slopes", &["hours"])],
    );
    let (outcome, text) = icc_of(&mut e, "m3");
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert_eq!(icc_rows(&text).len(), 1, "{text}");
    icc_is(
        &text,
        0,
        [
            0.138240133208837,
            0.224631236656824,
            0.0039673015848408,
            0.865962866602705,
        ],
    );
    // ... || school: hours, covariance(unstructured)
    fit(
        &mut e,
        &LINEAR_MIXED,
        &[
            ("y", "score"),
            ("group", "school"),
            ("covariance", "unstructured"),
            ("save", "m3u"),
        ],
        &[("x", &["hours", "girl"]), ("slopes", &["hours"])],
    );
    let (outcome, text) = icc_of(&mut e, "m3u");
    assert_eq!(outcome, Outcome::Ok, "{text}");
    icc_is(
        &text,
        0,
        [
            0.314751567109998,
            0.346945338567363,
            0.0192506069447333,
            0.914883730115681,
        ],
    );
    // melogit passed hours girl || school:, whose residual is pi^2/3
    fit(
        &mut e,
        &GENERALIZED_MIXED,
        &[("y", "passed"), ("group", "school"), ("save", "m4")],
        x,
    );
    let (outcome, text) = icc_of(&mut e, "m4");
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(text.contains("3.289868"), "{text}");
    assert_eq!(icc_rows(&text).len(), 1, "{text}");
    icc_is(
        &text,
        0,
        [
            0.0505741456337743,
            0.0787436265425069,
            0.00213612307456929,
            0.569985118435404,
        ],
    );
    // the residual of the logistic model is the latent variable's pi^2/3
    holds(
        &mut e,
        &[within(
            "attr(lme4::VarCorr(m4)$school, 'stddev')^2 / (attr(lme4::VarCorr(m4)$school, 'stddev')^2 + pi^2 / 3)",
            "0.0505741445635604",
            "2e-5",
        )],
    );
    // what Stata's estat icc refuses is refused here, with the reason: a
    // model for counts, crossed groups, and a model that is not lme4's
    fit(
        &mut e,
        &GENERALIZED_MIXED,
        &[
            ("y", "absences"),
            ("family", "poisson"),
            ("group", "school"),
            ("save", "m5"),
        ],
        x,
    );
    let (outcome, text) = icc_of(&mut e, "m5");
    assert_eq!(outcome, Outcome::Error, "{text}");
    assert!(text.contains("linear, logistic and probit"), "{text}");
    fit(
        &mut e,
        &LINEAR_MIXED,
        &[
            ("y", "score2"),
            ("group", "school"),
            ("crossed", "rater"),
            ("save", "m6"),
        ],
        x,
    );
    let (outcome, text) = icc_of(&mut e, "m6");
    assert_eq!(outcome, Outcome::Error, "{text}");
    assert!(text.contains("crossed groups"), "{text}");
    define(&mut e, "lin <- lm(score ~ hours, data = d)");
    let (outcome, text) = icc_of(&mut e, "lin");
    assert_eq!(outcome, Outcome::Error, "{text}");
    assert!(text.contains("lmer or glmer"), "{text}");
    // without the model's data frame the errors cannot be worked out: the
    // estimates are still printed, with the reason
    define(&mut e, "rm(d)");
    let (outcome, text) = icc_of(&mut e, "m1");
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert_eq!(icc_rows(&text).len(), 0, "{text}");
    assert!(
        text.contains("Standard errors need the model's data frame"),
        "{text}"
    );
    assert!(text.contains("0.41888"), "{text}");
    // the entry is on the Model menu of an lme4 model, and only of those
    assert!(rcraft_catalog::postest::model_specs("lmerMod")
        .iter()
        .any(|s| s.entry == "Intraclass correlation"));
    assert!(rcraft_catalog::postest::model_specs("lm")
        .iter()
        .all(|s| s.entry != "Intraclass correlation"));
}

#[test]
fn random_slopes_in_glmer_match_melogit() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    // Stata 19: melogit passed hc girl || school: hc, intmethod(laplace),
    // independent slopes (its default), and covariance(unstructured); the
    // Laplace approximation is what glmer allows with slopes
    let (outcome, text) = dialog_big(
        &mut e,
        &GENERALIZED_MIXED,
        &[("y", "passed"), ("group", "school"), ("save", "ind")],
        &[("x", &["hc", "girl"]), ("slopes", &["hc"])],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(text.contains("Odds ratios"), "{text}");
    let (outcome, text) = dialog_big(
        &mut e,
        &GENERALIZED_MIXED,
        &[
            ("y", "passed"),
            ("group", "school"),
            ("covariance", "unstructured"),
            ("save", "un"),
        ],
        &[("x", &["hc", "girl"]), ("slopes", &["hc"])],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let (outcome, text) = run(
        &mut e,
        Source::Typed,
        "stopifnot(isTRUE(all.equal(ind@devcomp$dims[['nAGQ']], 1)), \
         isTRUE(all.equal(un@devcomp$dims[['nAGQ']], 1))); cat('Laplace')",
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    // the fit is Poisson on the larger data: Stata 19 mepoisson count hours
    // girl, exposure(days) || school: hours, intmethod(laplace)
    let (outcome, text) = dialog_big(
        &mut e,
        &GENERALIZED_MIXED,
        &[
            ("y", "count"),
            ("family", "poisson"),
            ("group", "school"),
            ("exposure", "days"),
            ("save", "po"),
        ],
        &[("x", &["hours", "girl"]), ("slopes", &["hours"])],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(text.contains("Rate ratios"), "{text}");
    let fixed = |m: &str| {
        format!("c(lme4::fixef({m})[c(2, 3, 1)], sqrt(diag(as.matrix(vcov({m}))))[c(2, 3, 1)])")
    };
    holds(
        &mut e,
        &[
            within(
                &fixed("ind"),
                "c(.465759640805946, .363095244017737, -.802149103678236, \
                   .0964702801474643, .208585495713124, .213790693282985)",
                "1e-4",
            ),
            within(
                "c(attr(lme4::VarCorr(ind)$school.1, 'stddev')^2, attr(lme4::VarCorr(ind)$school, 'stddev')^2)",
                "c(.185052410747099, 1.10966137112068)",
                "1e-4",
            ),
            within("logLik(ind)", "-367.49308096329", "1e-7"),
            within(
                &fixed("un"),
                "c(.462596081540645, .363801404136769, -.785703082925222, \
                   .0969478796813606, .208537230764549, .215307199614322)",
                "1e-4",
            ),
            within(
                "local({ v <- as.matrix(lme4::VarCorr(un)$school); c(v[2, 2], v[1, 1], v[1, 2]) })",
                "c(.188339258532646, 1.09739909765427, .0596523483947688)",
                "1e-4",
            ),
            within("logLik(un)", "-367.380605889837", "1e-7"),
            within(
                &fixed("po"),
                "c(.280170418987826, .00923305523835184, -4.63055478633812, \
                   .0701397661704667, .0636317978683384, .356873971131259)",
                "1e-4",
            ),
            within(
                "c(attr(lme4::VarCorr(po)$school.1, 'stddev')^2, attr(lme4::VarCorr(po)$school, 'stddev')^2)",
                "c(.179544492073569, 4.4257257126714)",
                "1e-4",
            ),
            within("logLik(po)", "-1038.65609871803", "1e-7"),
        ],
    );
}

#[test]
fn crossed_effects_match_mixed() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    // linear: Stata 19 mixed score2 hours girl || _all: R.school || rater:
    let (outcome, text) = dialog(
        &mut e,
        &LINEAR_MIXED,
        &[
            ("y", "score2"),
            ("group", "school"),
            ("crossed", "rater"),
            ("save", "cr"),
        ],
        &[("x", &["hours", "girl"])],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    define(&mut e, ORACLE);
    // V / s^2 = I + a Z_school Z_school' + b Z_rater Z_rater', as written out
    define(
        &mut e,
        "Rr <- model.matrix(~ factor(rater) - 1, k); \
         vrel <- function(t) diag(n) + exp(2 * t[1]) * tcrossprod(S) + exp(2 * t[2]) * tcrossprod(Rr); \
         v <- lme4::VarCorr(cr); \
         th0 <- log(c(attr(v$school, 'stddev'), attr(v$rater, 'stddev')) / sigma(cr)); \
         hand <- lmm(vrel(th0), FALSE, k$score2)",
    );
    holds(
        &mut e,
        &[
            same("lme4::fixef(cr)", "hand$b"),
            same("sqrt(diag(as.matrix(vcov(cr))))", "hand$se"),
            same("sigma(cr)^2", "hand$s2"),
            within("logLik(cr)", "hand$ll", "1e-12"),
            "slope(function(t) lmm(vrel(t), FALSE, k$score2)$ll, th0) < 1e-5".to_string(),
            within(
                &b_se("cr"),
                "c(2.09798759553822, 2.98534678059959, 50.2999611599078, \
                   .170116823691215, .711042458379355, 2.34784493024845)",
                "1e-6",
            ),
            within(
                "log(c(attr(lme4::VarCorr(cr)$school, 'stddev'), \
                   attr(lme4::VarCorr(cr)$rater, 'stddev'), sigma(cr)))",
                "c(1.07076313456896, 1.63746285988173, 1.19154454003963)",
                "1e-6",
            ),
            // (Stata read hours as a float, which moves its log-likelihood 4e-6)
            within("logLik(cr)", "-277.92136003292", "1e-7"),
        ],
    );
    // logistic by Laplace, on the 98 pupils and on the larger data: Stata 19
    // melogit passed2 hours girl || _all: R.school || rater:, intmethod(laplace)
    let (outcome, text) = dialog(
        &mut e,
        &GENERALIZED_MIXED,
        &[
            ("y", "passed2"),
            ("group", "school"),
            ("crossed", "rater"),
            ("save", "cl"),
        ],
        &[("x", &["hours", "girl"])],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    holds(
        &mut e,
        &[
            // (the errors are not compared with crossed groups: glmer's are
            // conditional on the variances, Stata's from the full Hessian,
            // up to 5% apart)
            within(
                "lme4::fixef(cl)[c(2, 3, 1)]",
                "c(.20276953733822, 1.05806257199403, -1.83505573059142)",
                "1e-4",
            ),
            within(
                "c(attr(lme4::VarCorr(cl)$school, 'stddev')^2, attr(lme4::VarCorr(cl)$rater, 'stddev')^2)",
                "c(.382221692921158, 1.6894493722024)",
                "1e-4",
            ),
            within("logLik(cl)", "-56.7170062539611", "1e-7"),
        ],
    );
    let (outcome, text) = dialog_big(
        &mut e,
        &GENERALIZED_MIXED,
        &[
            ("y", "passed2"),
            ("group", "school"),
            ("crossed", "rater"),
            ("save", "cb"),
        ],
        &[("x", &["hours", "girl"])],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    holds(
        &mut e,
        &[
            within(
                "lme4::fixef(cb)[c(2, 3, 1)]",
                "c(.280561977161183, .472513701439089, -1.80614761529358)",
                "1e-4",
            ),
            within(
                "c(attr(lme4::VarCorr(cb)$school, 'stddev')^2, attr(lme4::VarCorr(cb)$rater, 'stddev')^2)",
                "c(.984855843233208, .654903737773256)",
                "1e-4",
            ),
            within("logLik(cb)", "-378.251679091228", "1e-7"),
        ],
    );
}

/// The row of `d` columns `col` by school (first row of each), in school order.
fn by_school(col: &str) -> String {
    format!("as.vector(tapply(d${col}, d$school, function(x) x[1]))")
}

#[test]
fn random_effect_predictions_match_predict_reffects() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    // linear with an independent random slope: Stata 19 mixed score hours
    // girl || school: hours, then predict double u*, reffects (u1 for hours,
    // u2 for the intercept); the two pupils with hours missing get their
    // school's values too
    let (outcome, text) = dialog(
        &mut e,
        &LINEAR_MIXED,
        &[
            ("y", "score"),
            ("group", "school"),
            ("ranef", "on"),
            ("save", "lmod"),
        ],
        &[("x", &["hours", "girl"]), ("slopes", &["hours"])],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    holds(
        &mut e,
        &[
            "!anyNA(d$u_school) && !anyNA(d$u_hours_school)".to_string(),
            within(
                &by_school("u_hours_school"),
                "c(-0.6573902864162008, -0.5962452375833728, -0.1037963390817329, 0.7655124170049711, \
                   -0.0814809530333279, 0.4811285913974183, -0.2330093381937715, 0.0742780403107252, \
                   0.1951391318221626, 0.1558639737731382)",
                "1e-5",
            ),
            within(
                &by_school("u_school"),
                "c(0.0955543039011025, -0.4945899824238849, -0.8626989610537531, 1.6631648644122889, \
                   0.2066843323323358, -0.5434596559877012, -0.0481120190295473, -0.1886437140645636, \
                   0.3704079151945097, -0.1983070832807693)",
                "1e-5",
            ),
        ],
    );
    // the same values are lme4's conditional modes, row by row
    holds(
        &mut e,
        &[same(
            "d$u_school",
            "lme4::ranef(lmod)$school[as.character(d$school), '(Intercept)']",
        )],
    );
    // nested: mixed score hours girl || school: || class:, predict double v*
    let (outcome, text) = dialog(
        &mut e,
        &LINEAR_MIXED,
        &[
            ("y", "score"),
            ("group", "school"),
            ("inner", "class"),
            ("ranef", "on"),
            ("save", "nest"),
        ],
        &[("x", &["hours", "girl"])],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    holds(
        &mut e,
        &[
            within(
                &by_school("u_school"),
                "c(-2.8966785302384181, -2.9620596010254139, -1.5890685579463970, 5.3618135883652540, \
                   0.0949004716474007, 1.2345705812411409, -0.8227704414679136, 0.0543554944315568, \
                   1.0945269105295770, 0.4304100844631510)",
                "1e-5",
            ),
            within(
                "local({ s <- setNames(c(0.2039825473065860, -0.0769735301264023, 0.3567959163906829, \
                   -1.2115721088942550, -0.2524245807672841, -0.6216448063955329, -1.3194591145776160, \
                   0.8505434384301592, 0.1676227036822756, 1.4145862146127199, 0.1376974512323391, \
                   -0.1096934242149325, 1.3930548242239560, -1.0287473838938910, 0.1751457927655053, \
                   -0.4179357944590990, -1.0477260523053551, 1.0637657274321770, -0.0091464709320168, \
                   0.3321286504900032), strsplit('10a 10b 1a 1b 2a 2b 3a 3b 4a 4b 5a 5b 6a 6b 7a 7b 8a 8b 9a 9b', ' ')[[1]]); \
                   tapply(d$u_school_class, d$class, function(x) x[1])[names(s)] - s })",
                "rep(0, 20)",
                "1e-4",
            ),
        ],
    );
    // logistic: melogit passed hours girl || school:, predict double w*,
    // remodes (the conditional modes; its plain reffects are posterior
    // means, which differ by up to 1e-2)
    let (outcome, text) = dialog(
        &mut e,
        &GENERALIZED_MIXED,
        &[
            ("y", "passed"),
            ("group", "school"),
            ("ranef", "on"),
            ("save", "lg"),
        ],
        &[("x", &["hours", "girl"])],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    holds(
        &mut e,
        &[
            "!anyNA(d$u_school)".to_string(),
            within(
                &by_school("u_school"),
                "c(-0.0464632690390434, -0.3234149868856445, 0.0925587502841480, 0.2757946871506283, \
                   0.1085819319575196, -0.3706388728242622, 0.1658726080868514, 0.2799847570649958, \
                   -0.0930563021640964, -0.0994234318712990)",
                "1e-4",
            ),
        ],
    );
}
