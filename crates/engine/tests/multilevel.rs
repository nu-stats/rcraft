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

use rcraft_catalog::multilevel::{GENERALIZED_MIXED, LINEAR_MIXED};
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
     data.frame(score, hours, girl, school, class, passed, absences, days) })";

/// The likelihoods in plain R on the complete rows `k`. `lmm(Vrel, reml)`:
/// GLS and the profiled log-likelihood for V / s^2 = `Vrel`. `S`, `C` the
/// school and class indicators. `agq(th, family, off)`: the 7-point adaptive
/// Gauss-Hermite log-likelihood of a random-intercept model, `th` = (the
/// coefficients, log sd). `slope(f, th)`: the largest central-difference
/// derivative of `f` at `th`.
const ORACLE: &str = "k <- d[!is.na(d$hours), ]; X <- cbind(1, k$hours, k$girl); n <- nrow(k); \
     S <- model.matrix(~ factor(school) - 1, k); C <- model.matrix(~ factor(class) - 1, k); \
     lmm <- function(Vrel, reml = FALSE) { y <- k$score; Vi <- solve(Vrel); \
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
        ]
        .into_iter()
        .map(|(name, kind)| ColumnInfo {
            name: name.into(),
            kind,
        })
        .collect(),
    }
}

/// Makes the data, then runs the dialog's code on it, its check boxes on.
fn dialog(
    e: &mut Engine,
    spec: &Spec,
    set: &[(&str, &str)],
    lists: &[(&str, &[&str])],
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
            values.set_flag(id, true);
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
