//! Statistics menu instrumental variables and selection models (P6.6) end to
//! end against a real R (skipped with a note when none is found, or when
//! fixest or sampleSelection is not installed). Each dialog's code is built
//! from its spec and run on made-up data (120 rows in six regions, an
//! endogenous schooling with two instruments, two values of a predictor
//! missing; earnings seen for 63 rows), and its numbers are checked three
//! ways:
//!
//! - against the formulas written out here in plain R: two-stage least squares
//!   with the projection on the instruments, each sandwich with its
//!   small-sample factor, the first-stage F and its robust Wald form,
//!   Wu-Hausman as the control-function test, Sargan as N R^2; Heckman's two
//!   steps (the probit, the inverse Mills ratio, the corrected variance) and
//!   his log-likelihood;
//! - against Stata 19's own results for the same data, recorded below
//!   (`ivregress 2sls` with and without `small`, conventional, robust and
//!   clustered, `estat firststage`, `endogenous` and `overid`, `xtivreg, fe`,
//!   `heckman` by maximum likelihood and `twostep`), to 1e-9;
//! - and against the package's report for the call written out by hand.
//!
//! The test records the R and package versions it ran with (AGENTS.md
//! section 5), and checks R is the one `ENVIRONMENT` pins.

use std::time::{Duration, Instant};

use rcraft_catalog::iv::{HECKMAN, IV_ML, IV_REGRESSION, TREATMENT_EFFECTS};
use rcraft_catalog::postest::{self, MODEL_FIT, ROBUST};
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

/// The R, fixest and sampleSelection versions, recorded; false when a
/// package is missing.
fn versions(e: &mut Engine) -> bool {
    let (outcome, text) = run(
        e,
        Source::Typed,
        "cat(paste(R.version$major, R.version$minor, sep = '.'), \
         vapply(c('fixest', 'sampleSelection'), function(p) if (requireNamespace(p, quietly = TRUE)) \
           as.character(packageVersion(p)) else 'none', ''))",
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let mut words = text.split_whitespace();
    let (r, fixest, selection) = (
        words.next().unwrap_or(""),
        words.next().unwrap_or("none"),
        words.next().unwrap_or("none"),
    );
    if fixest == "none" || selection == "none" {
        eprintln!("skipped: fixest or sampleSelection is not installed");
        return false;
    }
    let pinned = include_str!("../../../ENVIRONMENT")
        .lines()
        .find_map(|l| l.strip_prefix("R_VERSION="))
        .unwrap_or("")
        .trim();
    eprintln!(
        "ran with R {r}, fixest {fixest} and sampleSelection {selection} (ENVIRONMENT pins R {pinned})"
    );
    if r != pinned {
        eprintln!("note: this R is not the pinned one, so the run is not a Verified one");
    }
    true
}

/// 120 rows in six regions. Schooling shares an error with the wage, and z1
/// and z2 move it; x is missing in rows 7 and 55. Earnings are seen where
/// the work index is positive, its error correlated with the earnings'
/// (kids and dist are excluded from the earnings equation). high is the wage
/// above 9; hrs the wage less 8, censored at 0; top the wage censored at 10.
const MAKE: &str = "d <- local({ \
     set.seed(6601); n <- 120; region <- rep(1:6, each = 20); \
     z1 <- round(rnorm(n), 3); z2 <- round(rnorm(n) + 0.3 * z1, 3); x <- round(rnorm(n, 2), 3); \
     u <- rnorm(n); v <- 0.6 * u + rnorm(n, sd = 0.8); \
     educ <- round(10 + 0.8 * z1 + 0.5 * z2 + 0.4 * x + 0.3 * region + v, 3); \
     wage <- round(1 + 0.5 * educ + 0.7 * x + 0.2 * region + u, 3); \
     x[c(7, 55)] <- NA; \
     kids <- (seq_len(n) * 7) %% 4; dist <- round(rnorm(n, 5, 2), 2); \
     e <- rnorm(n); s <- 0.5 * e + rnorm(n, sd = sqrt(0.75)); \
     works <- as.numeric(0.8 + 0.5 * x - 0.6 * kids - 0.15 * dist + 0.4 * z1 + s > 0); \
     earn <- round(2 + 0.6 * ifelse(is.na(x), 2, x) + 0.3 * z1 + e, 3); \
     earn[works == 0 | is.na(works)] <- NA; \
     data.frame(wage, educ, x, z1, z2, region, works, earn, kids, dist, \
       high = as.numeric(wage > 9), hrs = pmax(0, round(wage - 8, 3)), top = pmin(wage, 10)) })";

/// Two-stage least squares in plain R on the complete rows `k`: `Xh` the
/// regressors projected on the instruments, `b`, the residuals `e`, the
/// bread `A`. With `fe`, everything is first demeaned within regions.
const ORACLE: &str = "tsls <- function(fe = FALSE) { k <- d[!is.na(d$x), ]; \
       dm <- function(v) if (fe) v - ave(v, k$region) else v; \
       one <- if (fe) NULL else 1; \
       X <- cbind(one, dm(k$educ), dm(k$x)); Z <- cbind(one, dm(k$z1), dm(k$z2), dm(k$x)); \
       y <- dm(k$wage); Xh <- Z %*% solve(crossprod(Z), crossprod(Z, X)); \
       b <- solve(crossprod(Xh, X), crossprod(Xh, y)); e <- as.vector(y - X %*% b); \
       list(k = k, N = nrow(k), K = ncol(X), b = drop(b), e = e, Xh = Xh, A = solve(crossprod(Xh))) }; \
     sw <- function(t, meat) sqrt(diag(t$A %*% meat %*% t$A))";

fn context() -> Context {
    Context {
        dataset: "d".into(),
        frames: Vec::new(),
        columns: [
            ("wage", VarKind::Number),
            ("educ", VarKind::Number),
            ("x", VarKind::Number),
            ("z1", VarKind::Number),
            ("z2", VarKind::Number),
            ("region", VarKind::Number),
            ("works", VarKind::Number),
            ("earn", VarKind::Number),
            ("kids", VarKind::Number),
            ("dist", VarKind::Number),
            ("high", VarKind::Number),
            ("hrs", VarKind::Number),
            ("top", VarKind::Number),
        ]
        .into_iter()
        .map(|(name, kind)| ColumnInfo {
            name: name.into(),
            kind,
        })
        .collect(),
    }
}

/// Makes the data, then runs the dialog's code on it. `flags` lists the
/// check boxes to turn on; the rest are off.
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

/// A report with spaces tidied, blank lines, notes and the lines that echo
/// the call left out.
fn report(text: &str) -> Vec<String> {
    text.lines()
        .filter(|l| {
            let l = l.trim_start();
            !(l.starts_with("Call:")
                || l.starts_with("sampleSelection::")
                || l.starts_with("selection(")
                || l.starts_with("NOTE:")
                || l.starts_with("warning:"))
        })
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|l| !l.is_empty())
        .collect()
}

fn holds(e: &mut Engine, exprs: &[String]) {
    let all = exprs.join(", ");
    let (outcome, text) = run(e, Source::Typed, &format!("stopifnot({all})"));
    if outcome != Outcome::Ok {
        // stopifnot names only the first failure; show each check's own
        // result, and for a comparison how far apart the two sides are, so a
        // platform-only difference can be read off the CI log
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

fn same(a: &str, b: &str) -> String {
    within(a, b, "1e-9")
}

#[test]
fn two_stage_least_squares_is_ivregress() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    let iv = |e: &mut Engine, set: &[(&str, &str)], flags: &[&str]| {
        let (outcome, text) = dialog(
            e,
            &IV_REGRESSION,
            &[[("y", "wage")].as_slice(), set].concat(),
            &[
                ("endog", &["educ"]),
                ("instruments", &["z1", "z2"]),
                ("x", &["x"]),
            ],
            flags,
        );
        assert_eq!(outcome, Outcome::Ok, "{text}");
        text
    };
    let text = iv(&mut e, &[], &["small"]);
    let (_, expected) = run(
        &mut e,
        Source::Typed,
        "m <- fixest::feols(wage ~ x | educ ~ z1 + z2, data = d, vcov = 'iid'); \
         print(summary(m)); print(stats::confint(m, level = 0.95))",
    );
    assert!(report(&expected).len() > 10, "{expected}");
    assert_eq!(report(&text), report(&expected), "{text}");
    iv(&mut e, &[("errors", "hetero"), ("save", "rs")], &["small"]);
    iv(&mut e, &[("cluster", "region"), ("save", "cs")], &["small"]);
    iv(&mut e, &[("save", "cl")], &[]);
    iv(&mut e, &[("errors", "hetero"), ("save", "rl")], &[]);
    iv(&mut e, &[("cluster", "region"), ("save", "ccl")], &[]);
    define(&mut e, ORACLE);
    define(
        &mut e,
        "t <- tsls(); N <- t$N; K <- t$K; G <- 6; \
         hc <- crossprod(t$Xh * t$e); cr <- crossprod(rowsum(t$Xh * t$e, t$k$region))",
    );
    holds(
        &mut e,
        &[
            same("coef(model)", "t$b"),
            // with small: s^2 on N - K, HC1, and G/(G-1) (N-1)/(N-K)
            same(
                "fixest::se(model)",
                "sqrt(diag(t$A) * sum(t$e^2) / (N - K))",
            ),
            same("fixest::se(rs)", "sw(t, hc) * sqrt(N / (N - K))"),
            same(
                "fixest::se(cs)",
                "sw(t, cr) * sqrt(G / (G - 1) * (N - 1) / (N - K))",
            ),
            // without: robust and clustered errors have no factor at all;
            // fixest's conventional errors divide by N - 1 where Stata's
            // divide by N
            same("fixest::se(rl)", "sw(t, hc)"),
            same("fixest::se(ccl)", "sw(t, cr)"),
            same("fixest::se(cl)", "sqrt(diag(t$A) * sum(t$e^2) / (N - 1))"),
            // Stata 19: ivregress 2sls wage x (educ = z1 z2), small, then
            // vce(robust) small and vce(cluster region) small (educ, x, _cons)
            same(
                "c(coef(model), fixest::se(model))[c(2, 3, 1, 5, 6, 4)]",
                "c(.515819113588581, .561279057392704, 1.69302370769631, \
                   .0814791321724118, .0867610528641146, .936593094938806)",
            ),
            same(
                "fixest::se(rs)[c(2, 3, 1)]",
                "c(.086556606123138, .0790260069509579, .981997380224778)",
            ),
            same(
                "fixest::se(cs)[c(2, 3, 1)]",
                "c(.0897916132958334, .0327084594809248, 1.08601266378396)",
            ),
            // Stata 19, no small: vce(robust), vce(cluster region), and the
            // conventional errors times sqrt(N / (N - 1))
            same(
                "fixest::se(rl)[c(2, 3, 1)]",
                "c(.0854492265393755, .0780149716227096, .969433996574752)",
            ),
            same(
                "fixest::se(ccl)[c(2, 3, 1)]",
                "c(.0812645511449996, .0296023000456003, .982879451886227)",
            ),
            same(
                "fixest::se(cl)[c(2, 3, 1)]",
                "c(.0804367122866083, .0856510576492978, .9246106002676) * sqrt(N / (N - 1))",
            ),
        ],
    );
    // the diagnostics the summary prints, and the robust first stage the
    // dialog adds under robust errors
    holds(
        &mut e,
        &[
            // the excluded instruments' F in the first stage
            same(
                "fixest::fitstat(model, 'ivf')[[1]]$stat",
                "local({ k <- t$k; anova(lm(educ ~ x, k), lm(educ ~ z1 + z2 + x, k))$F[2] })",
            ),
            // the first-stage residuals added to the structural equation
            same(
                "fixest::fitstat(model, 'wh')[[1]]$stat",
                "local({ k <- t$k; r <- resid(lm(educ ~ z1 + z2 + x, k)); \
                   anova(lm(wage ~ educ + x, k), lm(wage ~ educ + x + r, k))$F[2] })",
            ),
            same(
                "fixest::fitstat(model, 'sargan')[[1]]$stat",
                "N * summary(lm(t$e ~ z1 + z2 + x, t$k))$r.squared",
            ),
            // HC1 Wald / 2 on the two excluded instruments
            same(
                "fixest::fitstat(rs, 'ivwald')[[1]]$stat",
                "local({ f <- lm(educ ~ z1 + z2 + x, t$k); g <- coef(f)[2:3]; \
                   V <- sandwich::vcovHC(f, type = 'HC1')[2:3, 2:3]; drop(t(g) %*% solve(V, g)) / 2 })",
            ),
            // Stata 19: estat firststage (and after vce(robust)), estat
            // endogenous (Wu-Hausman), estat overid (Sargan)
            same("fixest::fitstat(model, 'ivf')[[1]]$stat", "79.346335626217"),
            same("fixest::fitstat(rs, 'ivwald')[[1]]$stat", "99.3151611007975"),
            same("fixest::fitstat(model, 'wh')[[1]]$stat", "24.9335356631255"),
            same(
                "fixest::fitstat(model, 'sargan')[[1]]$stat",
                "4.96008186572006",
            ),
        ],
    );
}

#[test]
fn two_stage_least_squares_with_fixed_effects_is_xtivreg() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    for (cluster, save) in [("", "fe"), ("region", "fc")] {
        let (outcome, text) = dialog(
            &mut e,
            &IV_REGRESSION,
            &[("y", "wage"), ("cluster", cluster), ("save", save)],
            &[
                ("endog", &["educ"]),
                ("instruments", &["z1", "z2"]),
                ("x", &["x"]),
                ("fe", &["region"]),
            ],
            &["small", "first"],
        );
        assert_eq!(outcome, Outcome::Ok, "{text}");
        assert!(text.contains("First stage"), "{text}");
    }
    define(&mut e, ORACLE);
    define(&mut e, "t <- tsls(fe = TRUE); N <- t$N; K <- t$K; G <- 6");
    holds(
        &mut e,
        &[
            same("coef(fe)", "t$b"),
            // s^2 on N - K - G (the region intercepts counted); clustered by
            // region, which the effects are nested in, so they count as one
            // intercept, as xtivreg's constant
            same(
                "fixest::se(fe)",
                "sqrt(diag(t$A) * sum(t$e^2) / (N - K - G))",
            ),
            same(
                "fixest::se(fc)",
                "sw(t, crossprod(rowsum(t$Xh * t$e, t$k$region))) * sqrt(G / (G - 1) * (N - 1) / (N - K - 1))",
            ),
            // Stata 19: xtset region; xtivreg wage x (educ = z1 z2), fe, and
            // with vce(cluster region)
            same(
                "c(coef(fe), fixest::se(fe))",
                "c(.510293283301664, .524232297908049, .0808634433640891, .0898561046606238)",
            ),
            same(
                "fixest::se(fc)",
                "c(.0855591630770197, .0655523114761484)",
            ),
        ],
    );
}

#[test]
fn heckman_is_statas_both_ways() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    let (outcome, text) = dialog(
        &mut e,
        &HECKMAN,
        &[("y", "earn"), ("selected", "works")],
        &[("x", &["x", "z1"]), ("z", &["x", "kids", "dist", "z1"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let (_, expected) = run(
        &mut e,
        Source::Typed,
        "m <- sampleSelection::selection(works ~ x + kids + dist + z1, earn ~ x + z1, data = d, \
         method = 'ml', gradtol = 1e-12, reltol = 1e-16, tol = 1e-14); print(summary(m))",
    );
    assert!(report(&expected).len() > 15, "{expected}");
    assert_eq!(report(&text), report(&expected), "{text}");
    // Heckman's log-likelihood written out, in (gamma, beta, log sigma,
    // atanh rho): it is the package's at its estimate, and flat there
    define(
        &mut e,
        "k <- d[!is.na(d$x), ]; W <- cbind(1, k$x, k$kids, k$dist, k$z1); Xo <- cbind(1, k$x, k$z1); \
         sel <- k$works == 1; \
         ll <- function(th) { a <- as.vector(W %*% th[1:5]); sg <- exp(th[9]); r <- tanh(th[10]); \
           u <- (k$earn[sel] - as.vector(Xo[sel, ] %*% th[6:8])) / sg; \
           sum(pnorm(-a[!sel], log.p = TRUE)) + \
           sum(pnorm((a[sel] + r * u) / sqrt(1 - r^2), log.p = TRUE) + dnorm(u, log = TRUE) - log(sg)) }; \
         th <- c(coef(model)[1:8], log(coef(model)['sigma']), atanh(coef(model)['rho'])); \
         grad <- vapply(1:10, function(j) { h <- 1e-5 * max(1, abs(th[j])); \
           up <- th; up[j] <- up[j] + h; dn <- th; dn[j] <- dn[j] - h; (ll(up) - ll(dn)) / (2 * h) }, 0); \
         sec <- sqrt(diag(vcov(model)))",
    );
    // The likelihood is flat at its maximum, so where the package's optimiser
    // stops moves with the last digits of the platform's libm and BLAS. On
    // Ubuntu 26.04 the estimates differ from Stata's by 3e-9 to 9e-9 and the
    // numeric-Hessian errors by up to 3e-8 (relative), while the likelihood,
    // the written-out likelihood (1e-12) and the gradient (1e-6) still agree;
    // Windows and macOS agree to 1e-9. 1e-7 still catches any real
    // difference from Stata's printed digits.
    let flat = |a: &str, b: &str| within(a, b, "1e-7");
    holds(
        &mut e,
        &[
            within("ll(th)", "logLik(model)", "1e-12"),
            "max(abs(grad)) < 1e-6".to_string(),
            // Stata 19: heckman earn x z1, select(works = x kids dist z1):
            // earn (x, z1, _cons), works (x, kids, dist, z1, _cons), its log
            // likelihood, rho and sigma; /athrho's and /lnsigma's errors
            // turned into rho's and sigma's by the delta method
            flat(
                "coef(model)[c(7, 8, 6, 2:5, 1)]",
                "c(.534451649172847, .274711127288714, 2.12564139551447, \
                   .69097480518434, -.673145313757891, -.168622265882451, .498160788138113, \
                   .647921484814399)",
            ),
            flat(
                "sec[c(7, 8, 6, 2:5, 1)]",
                "c(.130226678506693, .110536840824045, .418913261053247, \
                   .155492769454467, .148064116887622, .0687592963926237, .151384792914781, \
                   .497930046016517)",
            ),
            same("logLik(model)", "-122.553124733457"),
            flat(
                "coef(model)[c('rho', 'sigma')]",
                "c(.539245698283218, .80605015703993)",
            ),
            flat(
                "sec[c('rho', 'sigma')]",
                "c((1 - .539245698283218^2) * .426283215640952, .80605015703993 * .111238713683047)",
            ),
        ],
    );

    // two steps, selected where the earnings are not missing
    let (outcome, text) = dialog(
        &mut e,
        &HECKMAN,
        &[("y", "earn"), ("method", "2step"), ("save", "h2")],
        &[("x", &["x", "z1"]), ("z", &["x", "kids", "dist", "z1"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    // the probit's gamma (the package's own), the inverse Mills ratio, least
    // squares on the selected rows, sigma^2 = e'e/n + b_lambda^2 mean(delta),
    // and Heckman's corrected variance with the probit's observed information
    define(
        &mut e,
        "two <- local({ g <- coef(h2)[1:5]; a <- as.vector(W %*% g); s <- k$works; \
           lam <- dnorm(a) / pnorm(a); Xs <- cbind(Xo[sel, ], lam[sel]); ys <- k$earn[sel]; \
           b <- solve(crossprod(Xs), crossprod(Xs, ys)); es <- as.vector(ys - Xs %*% b); \
           dl <- lam[sel] * (lam[sel] + a[sel]); s2 <- sum(es^2) / sum(sel) + b[4]^2 * mean(dl); \
           rho <- b[4] / sqrt(s2); pd <- dnorm(a); P <- pnorm(a); \
           wo <- ifelse(s == 1, pd * (pd + a * P) / P^2, pd * (pd - a * (1 - P)) / (1 - P)^2); \
           Vg <- solve(crossprod(W, wo * W)); \
           Q <- rho^2 * crossprod(Xs, dl * W[sel, ]) %*% Vg %*% crossprod(W[sel, ], dl * Xs); \
           XX <- solve(crossprod(Xs)); V <- s2 * XX %*% (crossprod(Xs, (1 - rho^2 * dl) * Xs) + Q) %*% XX; \
           list(b = drop(b), se = sqrt(diag(V)), sg = sqrt(diag(Vg)), sigma = sqrt(s2), rho = rho) }); \
         se2 <- sqrt(diag(vcov(h2)))",
    );
    // the probit inside stops at glm's default tolerance, so the package's
    // estimates are within about 1e-8 of the exact two-step ones; Stata's
    // twostep reports selection-equation errors about 3e-6 from its own probit
    // command's (which are the package's to 1e-10), and outcome errors 1e-7
    // from the package's
    holds(
        &mut e,
        &[
            same("coef(h2)[6:9]", "two$b"),
            same("se2[6:9]", "two$se"),
            same("se2[1:5]", "two$sg"),
            same("coef(h2)[c('sigma', 'rho')]", "c(two$sigma, two$rho)"),
            // Stata 19: heckman earn x z1, select(x kids dist z1) twostep:
            // earn (x, z1, _cons), works (x, kids, dist, z1, _cons), lambda,
            // rho and sigma; probit works x kids dist z1 for x's error
            same(
                "coef(h2)[c(7, 8, 6, 2:5, 1, 9)]",
                "c(.523379664891441, .274000006433513, 2.16388341342385, \
                   .716938646117305, -.691491298394987, -.160938793717781, .494666862844898, \
                   .596554400109923, .409931115823279)",
            ),
            same(
                "coef(h2)[c('rho', 'sigma')]",
                "c(.510773783704692, .802568825772553)",
            ),
            within(
                "se2[c(7, 8, 6, 9)]",
                "c(.135106033507835, .111657346853728, .453805549047174, .331260086090165)",
                "1e-6",
            ),
            same("se2[2]", ".157479856987281"),
        ],
    );
}

/// Each value of `got` within `tol` (relative, mean) of the Stata numbers.
fn near(got: &str, stata: &str, tol: &str) -> String {
    within(got, &format!("c({stata})"), tol)
}

#[test]
fn liml_and_gmm_match_ivregress() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    let fit = |e: &mut Engine, set: &[(&str, &str)], x: &[&str], insts: &[&str], flags: &[&str]| {
        let (outcome, text) = dialog(
            e,
            &IV_REGRESSION,
            &[[("y", "wage")].as_slice(), set].concat(),
            &[("endog", &["educ"]), ("instruments", insts), ("x", x)],
            flags,
        );
        assert_eq!(outcome, Outcome::Ok, "{text}");
        text
    };
    let (x, z) = (["x"], ["z1", "z2"]);
    let liml = |e: &mut Engine, save: &str, errors: &str, cluster: &str, flags: &[&str]| {
        fit(
            e,
            &[
                ("estimator", "liml"),
                ("save", save),
                ("errors", errors),
                ("cluster", cluster),
            ],
            &x,
            &z,
            flags,
        )
    };
    let text = liml(&mut e, "l", "iid", "", &["first"]);
    assert!(
        text.contains("kappa = 1.043479293") && text.contains("Coefficients:"),
        "{text}"
    );
    liml(&mut e, "ls", "iid", "", &["small"]);
    liml(&mut e, "lr", "hetero", "", &[]);
    liml(&mut e, "lrs", "hetero", "", &["small"]);
    liml(&mut e, "lc", "iid", "region", &[]);
    liml(&mut e, "lcs", "iid", "region", &["small"]);
    fit(
        &mut e,
        &[("estimator", "liml"), ("save", "l0")],
        &[],
        &z,
        &[],
    );
    let gmm = |e: &mut Engine, save: &str, errors: &str, cluster: &str, flags: &[&str]| {
        fit(
            e,
            &[
                ("estimator", "gmm"),
                ("save", save),
                ("errors", errors),
                ("cluster", cluster),
            ],
            &x,
            &z,
            flags,
        )
    };
    let text = gmm(&mut e, "g", "hetero", "", &[]);
    assert!(text.contains("Hansen's J = 3.980823"), "{text}");
    gmm(&mut e, "gs", "hetero", "", &["small"]);
    gmm(&mut e, "gu", "iid", "", &[]);
    gmm(&mut e, "gus", "iid", "", &["small"]);
    gmm(&mut e, "gc", "iid", "region", &[]);
    gmm(&mut e, "gcs", "iid", "region", &["small"]);
    fit(
        &mut e,
        &[("estimator", "gmm"), ("save", "g0"), ("errors", "hetero")],
        &[],
        &["z1", "z2", "x"],
        &[],
    );
    let se = |m: &str| format!("sqrt(diag({m}$vcov))[c(2, 3, 1)]");
    let b = |m: &str| format!("coef({m})[c(2, 3, 1)]");
    // the LIML k written out: the least variance ratio of the outcome less
    // educ's part, after the exogenous predictor alone and after all the
    // instruments, is kappa, at the LIML coefficient
    define(
        &mut e,
        "k <- d[!is.na(d$x), ]; ratio <- function(g) { r <- k$wage - g * k$educ; \
           sum(resid(lm(r ~ x, k))^2) / sum(resid(lm(r ~ z1 + z2 + x, k))^2) }; \
         least <- optimize(ratio, c(0, 1), tol = 1e-12)",
    );
    holds(
        &mut e,
        &[
            within("least$objective", "l$kappa", "1e-12"),
            within("least$minimum", "coef(l)[2]", "1e-6"),
            // Stata 19: ivregress liml wage x (educ = z1 z2): e(b), its
            // errors and e(kappa); then small, vce(robust) with and without
            // small, vce(cluster region) with and without small
            same(
                &b("l"),
                "c(.498447193365507, .566211009068049, 1.88873382345548)",
            ),
            same("l$kappa", "1.04347929250276"),
            same(
                &se("l"),
                "c(.082467239678828, .0865299369077466, .947367126537199)",
            ),
            same(
                &se("ls"),
                "c(.0835359741922704, .0876513219617324, .959644534499127)",
            ),
            same(
                &se("lr"),
                "c(.0891813317268475, .0791658743292889, 1.01138875002576)",
            ),
            same(
                &se("lrs"),
                "c(.0903370775423062, .0801918247856405, 1.02449584646635)",
            ),
            same(
                &se("lc"),
                "c(.0859468051647588, .030262696253335, 1.03186648014669)",
            ),
            same(
                &se("lcs"),
                "c(.0949651746626453, .03343815084169, 1.14013988452289)",
            ),
            same(&b("lcs"), &b("l")),
            // ivregress liml wage (educ = z1 z2)
            same(
                "c(coef(l0)[c(2, 1)], sqrt(diag(l0$vcov))[c(2, 1)])",
                "c(.459945888070735, 3.44702201488394, .100187667706886, 1.18996236882436)",
            ),
            // ivregress gmm wage x (educ = z1 z2), estat overid; then small,
            // vce(unadjusted) with and without small, wmatrix(cluster region)
            // with and without small (and its J)
            same(
                &b("g"),
                "c(.533541755446612, .569129369158756, 1.44488754839693)",
            ),
            same(
                &se("g"),
                "c(.0839159460935805, .0770453545788644, .94926125117524)",
            ),
            same("g$J", "3.98082286609787"),
            same("pchisq(g$J, 1, lower.tail = FALSE)", ".0460210758055616"),
            same(
                &se("gs"),
                "c(.0850034551234408, .0780438241512609, .961563206052783)",
            ),
            same(
                &se("gu"),
                "c(.0849862873200696, .0779156898702126, .961423544513104)",
            ),
            same(
                &se("gus"),
                "c(.0860876674412186, .0789254385562032, .973883116678474)",
            ),
            same(&b("gu"), &b("g")),
            same(
                &b("gc"),
                "c(.522334538671452, .567134273562958, 1.50976405113003)",
            ),
            same(
                &se("gc"),
                "c(.0848662462003165, .0361900888513891, 1.01167758891253)",
            ),
            same("gc$J", "3.04842106963101"),
            same(
                &se("gcs"),
                "c(.0937712330077658, .0399875027610472, 1.11783257968936)",
            ),
            // ivregress gmm wage (educ = z1 z2 x)
            same(
                "c(coef(g0)[c(2, 1)], sqrt(diag(g0$vcov))[c(2, 1)], g0$J)",
                "c(.738464603000113, .24708644915647, .0838435431243328, .997931527006952, \
                   29.1150248245141)",
            ),
        ],
    );
}

#[test]
fn iv_probit_and_tobit_match_stata() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    let fit = |e: &mut Engine, set: &[(&str, &str)]| {
        let (outcome, text) = dialog(
            e,
            &IV_ML,
            &[[("endog", "educ")].as_slice(), set].concat(),
            &[("instruments", &["z1", "z2"]), ("x", &["x"])],
            &[],
        );
        assert_eq!(outcome, Outcome::Ok, "{text}");
        text
    };
    let text = fit(&mut e, &[("y", "high"), ("save", "p")]);
    assert!(
        text.contains("Wald test of exogeneity") && text.contains("corr(educ, high)"),
        "{text}"
    );
    fit(
        &mut e,
        &[("y", "high"), ("save", "pr"), ("errors", "hetero")],
    );
    fit(
        &mut e,
        &[("y", "high"), ("save", "pc"), ("cluster", "region")],
    );
    let tobit = |e: &mut Engine, set: &[(&str, &str)]| {
        fit(e, &[[("model", "tobit")].as_slice(), set].concat())
    };
    let text = tobit(&mut e, &[("y", "hrs"), ("save", "t")]);
    assert!(text.contains("34 censored below and 0 above"), "{text}");
    tobit(
        &mut e,
        &[("y", "hrs"), ("save", "tr"), ("errors", "hetero")],
    );
    tobit(
        &mut e,
        &[("y", "hrs"), ("save", "tc"), ("cluster", "region")],
    );
    let text = tobit(
        &mut e,
        &[("y", "top"), ("save", "tu"), ("lower", ""), ("upper", "10")],
    );
    assert!(text.contains("0 censored below and 30 above"), "{text}");
    tobit(&mut e, &[("y", "hrs"), ("save", "tb"), ("upper", "3")]);
    // Stata's order: the outcome's educ, x, _cons; educ's x, z1, z2, _cons;
    // /athrho2_1, (/lnsigma1,) /lnsigma2
    let b = |m: &str| format!("coef({m})[c(2, 3, 1, 7, 5, 6, 4, 8:length(coef({m})))]");
    let se = |m: &str| format!("sqrt(diag({m}$vcov))[c(2, 3, 1, 7, 5, 6, 4, 8:length(coef({m})))]");
    // Stata stops where its change in the log-likelihood is below 1e-7, which
    // on this flat likelihood leaves its probit estimates about 1e-7 from the
    // maximum the dialog reaches (the log-likelihoods agree to 1e-15)
    let ml = |a: &str, b: &str| near(a, b, "1e-6");
    holds(
        &mut e,
        &[
            // Stata 19: ivprobit high x (educ = z1 z2), then vce(robust) and
            // vce(cluster region): e(b), the errors, e(ll), e(chi2_exog)
            ml(
                &b("p"),
                ".445027211426933, .654367535925901, -6.69367627371841, .35494909486421, \
                 .850704114589881, .53469502190419, 11.0824051603976, .635720894873503, \
                 -.0708165020149279",
            ),
            ml(
                &se("p"),
                ".165789105027139, .150011343568814, 2.06490601874771, .0807294285935275, \
                 .0873995656453498, .0899633867388431, .179777723692365, .210850980303174, \
                 .0654972679194354",
            ),
            within("p$loglik", "-201.901474847615", "1e-12"),
            ml("coef(p)[8]^2 / p$vcov[8, 8]", "9.09037340510604"),
            ml(
                &se("pr"),
                ".166183448658647, .142504353842388, 2.04252461111504, .0728714178677016, \
                 .0840492211149925, .0939368956510701, .173842039584188, .200459221404438, \
                 .0559409043707051",
            ),
            ml(
                &se("pc"),
                ".171378786557404, .191125736064477, 2.27586078298853, .050735322521732, \
                 .0766708170397616, .134795092094013, .219368391366173, .146241278370201, \
                 .0657944003628893",
            ),
            // ivtobit hrs x (educ = z1 z2), ll(0), then vce(robust) and
            // vce(cluster region)
            ml(
                &b("t"),
                ".461867784483787, .529953459427294, -5.58505841070149, .351721046701458, \
                 .880332037400995, .489795035718691, 11.0899659490096, .736876624713189, \
                 -.0258646139482307, -.0656071714266214",
            ),
            ml(
                &se("t"),
                ".0903627429101763, .094798953548531, 1.055459287166, .0811121744676045, \
                 .082637638901454, .0821943491587294, .180624593839367, .139614921608568, \
                 .09794258398048, .065878290122782",
            ),
            within("t$loglik", "-271.771226716996", "1e-12"),
            ml("coef(t)[8]^2 / t$vcov[8, 8]", "27.8564574948848"),
            ml(
                &se("tr"),
                ".0903427826625507, .0911466807438586, 1.01616245478992, .0734456728777231, \
                 .0764866453331406, .081542282234108, .174941878332976, .132000401750469, \
                 .107015097347805, .0570012426644965",
            ),
            ml(
                &se("tc"),
                ".118741907408908, .0598794454307406, 1.37488250234631, .0503636322757906, \
                 .077415071417707, .133765085453119, .226058839431677, .204268078477806, \
                 .153263171598813, .0676054965194201",
            ),
            // ivtobit top x (educ = z1 z2), ul(10); ivtobit hrs ..., ll(0) ul(3)
            ml(
                &b("tu"),
                ".499501042877933, .601043041629748, 1.81820389150888, .354475211189553, \
                 .855400457987996, .527998865409236, 11.0835111802591, .563603416372363, \
                 -.00971325625829627, -.0701804246877386",
            ),
            ml(
                &se("tu"),
                ".0907493275231051, .0952312244757202, 1.04559658586063, .0807538440568585, \
                 .0845824516845294, .0848595744176109, .179826791553234, .12949570486903, \
                 .0868503102619269, .0655043033439294",
            ),
            within("tu$loglik", "-287.082280870356", "1e-12"),
            ml(
                &b("tb"),
                ".51720544070403, .54649443419374, -6.26721735041622, .352045221760911, \
                 .877594120077694, .494232437052534, 11.089203962001, .685057933388784, \
                 .00416676628101525, -.0662175545585323",
            ),
            ml(
                &se("tb"),
                ".0987512753033668, .10263443662479, 1.16688742258873, .081076261125825, \
                 .0836705409856743, .0847128437134688, .180547762468196, .144007578959286, \
                 .104407030850768, .0658836736904021",
            ),
            within("tb$loglik", "-269.160854752638", "1e-12"),
        ],
    );
    // the probit's log-likelihood written out on its own at the dialog's
    // estimate: educ's normal density times the probit given its error
    define(
        &mut e,
        "k <- d[!is.na(d$x), ]; th <- coef(p); rho <- tanh(th[8]); sg <- exp(th[9]); \
         u <- (k$educ - th[4] - th[5] * k$z1 - th[6] * k$z2 - th[7] * k$x) / sg; \
         w <- (th[1] + th[2] * k$educ + th[3] * k$x + rho * u) / sqrt(1 - rho^2); \
         own <- sum(dnorm(u, log = TRUE) - log(sg) + pnorm((2 * k$high - 1) * w, log.p = TRUE))",
    );
    holds(&mut e, &[within("own", "p$loglik", "1e-13")]);
}

#[test]
fn treatment_effects_match_teffects() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    let te = |e: &mut Engine, method: &str, stat: &str, save: &str, x: &[&str], tx: &[&str]| {
        let (outcome, text) = dialog(
            e,
            &TREATMENT_EFFECTS,
            &[
                ("y", "wage"),
                ("treat", "works"),
                ("method", method),
                ("stat", stat),
                ("save", save),
            ],
            &[("x", x), ("tx", tx)],
            &[],
        );
        assert_eq!(outcome, Outcome::Ok, "{text}");
        text
    };
    let xz = ["x", "z1"];
    let text = te(&mut e, "ra", "ate", "ra", &xz, &[]);
    assert!(
        text.contains("118 observations, 63 treated") && text.contains("ATE (1 vs 0)"),
        "{text}"
    );
    te(&mut e, "ra", "atet", "rat", &xz, &[]);
    te(&mut e, "ra", "pomeans", "rap", &xz, &[]);
    te(&mut e, "ipw", "ate", "ipw", &[], &xz);
    te(&mut e, "ipw", "atet", "ipt", &[], &xz);
    te(&mut e, "ipw", "pomeans", "ipp", &[], &xz);
    te(&mut e, "aipw", "ate", "aipw", &xz, &xz);
    te(&mut e, "aipw", "atet", "aipt", &xz, &xz);
    te(&mut e, "aipw", "pomeans", "aipp", &xz, &xz);
    te(&mut e, "aipw", "ate", "a2", &["x"], &["x", "z1", "kids"]);
    te(&mut e, "aipw", "atet", "a2t", &["x"], &["x", "z1", "kids"]);
    te(&mut e, "ra", "ate", "ra0", &[], &[]);
    te(&mut e, "ipw", "atet", "i2t", &[], &["x", "z1", "kids"]);
    let both = |m: &str| format!("c(coef({m}), sqrt(diag({m}$vcov)))");
    // the point estimates written out: the two groups' regressions predicted
    // for every row, and the logit's weights normalised within each group
    define(
        &mut e,
        "k <- d[!is.na(d$x), ]; on <- k$works == 1; \
         p1 <- predict(lm(wage ~ x + z1, k[on, ]), k); p0 <- predict(lm(wage ~ x + z1, k[!on, ]), k); \
         ps <- fitted(glm(works ~ x + z1, binomial, k)); \
         w1 <- weighted.mean(k$wage[on], 1 / ps[on]); w0 <- weighted.mean(k$wage[!on], 1 / (1 - ps[!on]))",
    );
    holds(
        &mut e,
        &[
            same("coef(ra)", "c(mean(p1 - p0), mean(p0))"),
            same("coef(rat)", "c(mean(p1[on] - p0[on]), mean(p0[on]))"),
            same("coef(ipw)", "c(w1 - w0, w0)"),
            // Stata 19: teffects ra (wage x z1) (works), then atet and
            // pomeans: the effect and the control mean, their robust errors
            same(
                &both("ra"),
                "c(-.0592069620531749, 8.97046608742692, .273167715955733, .222986635007254)",
            ),
            same(
                &both("rat"),
                "c(-.159285687561524, 9.41866663994248, .298609563451583, .275744386687767)",
            ),
            same(
                &both("rap"),
                "c(8.97046608742692, 8.91125912537374, .222986635007254, .187754581964885)",
            ),
            // teffects ipw (wage) (works x z1), atet, pomeans
            same(
                &both("ipw"),
                "c(.0569419121063197, 8.90853312432116, .272457488490695, .236927548463686)",
            ),
            same(
                &both("ipt"),
                "c(-.0674895878332434, 9.32687054021419, .33271157760977, .300674208761148)",
            ),
            same(
                &both("ipp"),
                "c(8.90853312432116, 8.96547503642748, .236927548463686, .175102912741137)",
            ),
            // teffects aipw (wage x z1) (works x z1), pomeans, atet (its
            // table's printed digits)
            same(
                &both("aipw"),
                "c(-.0182462465517867, 9.00024822219498, .26622142121603, .225521500701419)",
            ),
            same(
                &both("aipp"),
                "c(9.00024822219498, 8.98200197564319, .225521500701419, .172843123499455)",
            ),
            near(
                &both("aipt"),
                "-.2150681, 9.474449, .3055101, .2865445",
                "1e-6",
            ),
            // teffects aipw (wage x) (works x z1 kids), and atet; teffects ra
            // (wage) (works); teffects ipw (wage) (works x z1 kids), atet
            same(
                &both("a2"),
                "c(-.0403376749718972, 8.89546710255316, .251608291224233, .207734447524056)",
            ),
            same(
                &both("a2t"),
                "c(-.0188113984329089, 9.27819235081386, .303686894129314, .277527041830216)",
            ),
            same(
                &both("ra0"),
                "c(.802308225108225, 8.45707272727273, .286435922175046, .224914102765567)",
            ),
            "ra0$nobs == 118".to_string(),
            same(
                &both("i2t"),
                "c(.180320838772696, 9.07906011360826, .350157039524563, .322869328341323)",
            ),
        ],
    );
}

#[test]
fn iv_models_have_the_model_menu() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    // the menu: Model fit and Robust standard errors on a two-stage fit
    // (fixest), Model fit on a Heckman model (selection)
    let entries = |class: &str| -> Vec<&str> {
        postest::model_specs(class)
            .iter()
            .map(|s| s.entry)
            .collect()
    };
    assert!(postest::is_model_class("fixest") && postest::is_model_class("selection"));
    assert_eq!(entries("fixest"), ["Model fit", "Robust standard errors"]);
    assert_eq!(entries("selection"), ["Model fit"]);
    assert!(!postest::takes_model_table("selection"));
    // LIML, GMM, IV probit and tobit and treatment effects are plain lists
    assert!(!postest::is_model_class("list"));
    let (outcome, text) = dialog(
        &mut e,
        &IV_REGRESSION,
        &[("y", "wage"), ("save", "iv")],
        &[
            ("endog", &["educ"]),
            ("instruments", &["z1", "z2"]),
            ("x", &["x"]),
        ],
        &["small"],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    for (method, save) in [("ml", "hm"), ("2step", "h2")] {
        let (outcome, text) = dialog(
            &mut e,
            &HECKMAN,
            &[
                ("y", "earn"),
                ("selected", "works"),
                ("method", method),
                ("save", save),
            ],
            &[("x", &["x", "z1"]), ("z", &["x", "kids", "dist", "z1"])],
            &[],
        );
        assert_eq!(outcome, Outcome::Ok, "{text}");
    }
    let (outcome, text) = run(
        &mut e,
        Source::Typed,
        "cat(class(iv)[1], class(hm)[1], class(h2)[1])",
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert_eq!(
        text.split_whitespace().collect::<Vec<_>>(),
        ["fixest", "selection", "selection"]
    );
    // the dialogs of the Objects pane's Model menu, on an object by name
    let on_model = |e: &mut Engine, spec: &Spec, model: &str| {
        let ctx = Context {
            dataset: model.into(),
            frames: Vec::new(),
            columns: Vec::new(),
        };
        let values = spec.defaults(&ctx);
        let code = spec.generate(&ctx, &values, Style::Base);
        assert!(code.is_ok(), "{code:?}");
        let (outcome, text) = run(e, Source::Menu(spec.menu.into()), &code.unwrap_or_default());
        assert_eq!(outcome, Outcome::Ok, "{text}");
        text
    };
    let text = on_model(&mut e, &MODEL_FIT, "iv");
    assert!(
        text.contains("Observations: 118") && text.contains("BIC"),
        "{text}"
    );
    let text = on_model(&mut e, &ROBUST, "iv");
    assert!(text.contains("HC1"), "{text}");
    // Stata 19: heckman earn x z1, select(works = x kids dist z1): e(ll), and
    // estat ic's BIC on its 118 observations (-2 ll + 10 ln 118)
    let text = on_model(&mut e, &MODEL_FIT, "hm");
    assert!(
        text.contains("Observations: 118")
            && text.contains("-122.5531")
            && text.contains("292.8131"),
        "{text}"
    );
    let text = on_model(&mut e, &MODEL_FIT, "h2");
    assert!(
        text.contains("Observations: 118") && text.contains("two steps have no likelihood"),
        "{text}"
    );
}
