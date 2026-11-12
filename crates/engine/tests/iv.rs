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

use rcraft_catalog::iv::{HECKMAN, IV_REGRESSION};
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
/// (kids and dist are excluded from the earnings equation).
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
     data.frame(wage, educ, x, z1, z2, region, works, earn, kids, dist) })";

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
    holds(
        &mut e,
        &[
            within("ll(th)", "logLik(model)", "1e-12"),
            "max(abs(grad)) < 1e-6".to_string(),
            // Stata 19: heckman earn x z1, select(works = x kids dist z1):
            // earn (x, z1, _cons), works (x, kids, dist, z1, _cons), its log
            // likelihood, rho and sigma; /athrho's and /lnsigma's errors
            // turned into rho's and sigma's by the delta method
            same(
                "coef(model)[c(7, 8, 6, 2:5, 1)]",
                "c(.534451649172847, .274711127288714, 2.12564139551447, \
                   .69097480518434, -.673145313757891, -.168622265882451, .498160788138113, \
                   .647921484814399)",
            ),
            same(
                "sec[c(7, 8, 6, 2:5, 1)]",
                "c(.130226678506693, .110536840824045, .418913261053247, \
                   .155492769454467, .148064116887622, .0687592963926237, .151384792914781, \
                   .497930046016517)",
            ),
            same("logLik(model)", "-122.553124733457"),
            same(
                "coef(model)[c('rho', 'sigma')]",
                "c(.539245698283218, .80605015703993)",
            ),
            same(
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
