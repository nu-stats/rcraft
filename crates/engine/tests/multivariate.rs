//! Statistics menu multivariate dialogs (P6.7) end to end against a real R
//! (skipped with a note when none is found, or when MASS is not installed).
//! Each dialog's code is built from its spec and run on made-up data, and its
//! results are checked two ways:
//!
//! - against the formula written out here in plain R: the eigenproblem of the
//!   correlation (or covariance) matrix for principal components; the
//!   maximum-likelihood fit by its own criterion and Kaiser's two-factor
//!   varimax angle for factor analysis; Lance and Williams's update, merge by
//!   merge, and Lloyd's iterations for clusters; the pooled covariance
//!   matrix, the linear scores and Cholesky-sphered eigenproblem of the
//!   [MV] manual for discriminant analysis;
//! - and against Stata 19's own results for the same data, recorded below
//!   (`pca`, `factor, ml` with `rotate, varimax`, `cluster ...linkage` with
//!   `cluster generate`, `cluster kmeans`, `discrim lda` with `estat
//!   classtable`). Only numbers are kept.
//!
//! Where Stata and R differ by convention, the test says so: Stata signs each
//! principal component so its elements sum to more than 0 where the dialog
//! makes the largest loading positive (compared by absolute value, with the
//! sign rule checked); `factor, ml` stops at its default tolerance about 3e-5
//! short of the maximum (checked at 1e-4, and to 1e-6 against a run with the
//! tolerance tightened); the likelihood-ratio statistic of `factanal` uses N-1 (the
//! dialog recomputes it with N, as Stata has it); `MASS::lda` weighs the group means by the priors where
//! Stata weighs them by the group sizes, so the dialog takes the discriminant
//! functions from the fit with the sizes as priors (Stata's with any priors);
//! a two-factor varimax takes Kaiser's exact angle, as Stata does.
//!
//! P6.7c adds principal factors (by hand from the reduced correlation matrix,
//! and Stata's `factor, pf | ipf | pcf`: ipf stops when the communalities'
//! squared changes sum below 1e-8, a uniqueness may go below 0, and only
//! eigenvalues above 0, or 1 for pcf, give factors); varimax as Stata turns
//! it (Kaiser's exact angle for two factors, the SVD iteration stopped at a
//! relative change of 1e-8 for more) or to the exact maximum; promax (the
//! least-squares fit to the varimax loadings raised to the power) and oblimin
//! (a minimum of its criterion, checked by turning each axis a little), both
//! against `rotate` (oblimin to 1e-7 against a run at tolerance 1e-13);
//! oblique factors ordered by the structure matrix's sums of squares; other
//! dissimilarities by Lance and Williams on the matrices built here (Canberra
//! as Stata's |a - b| / (|a| + |b|)), k-means in L1 and L-infinity, and
//! `cluster stop`'s Calinski-Harabasz and Duda-Hart numbers; and the quadratic
//! discriminant's posteriors from each group's own normal density, against
//! `discrim qda` with `predict double, pr` to 1e-12.
//!
//! The test records the R and package versions it ran with (AGENTS.md
//! section 5), and checks R is the one `ENVIRONMENT` pins.

use std::time::{Duration, Instant};

use rcraft_catalog::multivariate::{
    CLUSTER_ANALYSIS, DISCRIMINANT_ANALYSIS, FACTOR_ANALYSIS, PRINCIPAL_COMPONENTS,
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

/// The R and MASS versions, recorded; false when MASS is missing.
fn versions(e: &mut Engine) -> bool {
    let (outcome, text) = run(
        e,
        Source::Typed,
        "cat(paste(R.version$major, R.version$minor, sep = '.'), \
         if (requireNamespace('MASS', quietly = TRUE)) as.character(packageVersion('MASS')) \
         else 'none')",
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let mut words = text.split_whitespace();
    let (r, mass) = (words.next().unwrap_or(""), words.next().unwrap_or("none"));
    if mass == "none" {
        eprintln!("skipped: MASS is not installed");
        return false;
    }
    let pinned = include_str!("../../../ENVIRONMENT")
        .lines()
        .find_map(|l| l.strip_prefix("R_VERSION="))
        .unwrap_or("")
        .trim();
    eprintln!("ran with R {r} and MASS {mass} (ENVIRONMENT pins R {pinned})");
    if r != pinned {
        eprintln!("note: this R is not the pinned one, so the run is not a Verified one");
    }
    true
}

/// The three made-up data sets: `fa`, forty rows of six variables built from
/// two factors; `cl`, thirty points in the plane in three overlapping lumps of
/// 14, 9 and 7; `di`, forty-five rows in three groups of 20, 15 and 10.
const MAKE: &str = r#"
fa <- local({
  r <- 1:40
  f1 <- sin(r * 1.3) + 0.3 * cos(r * 0.7); f2 <- cos(r * 2.1) + 0.2 * sin(r)
  nz <- function(j, c) sin(r * c + j)
  data.frame(
    v1 = round(0.9 * f1 + 0.5 * nz(1, 3.7), 3),
    v2 = round(0.8 * f1 + 0.6 * nz(2, 4.3), 3),
    v3 = round(0.7 * f1 + 0.3 * f2 + 0.6 * nz(3, 5.9), 3),
    v4 = round(0.8 * f2 + 0.5 * nz(4, 6.7), 3),
    v5 = round(0.7 * f2 + 0.6 * nz(5, 7.9), 3),
    v6 = round(0.4 * f1 + 0.5 * f2 + 0.6 * nz(6, 8.3), 3))
})
cl <- local({
  r <- 1:30; g <- rep(1:3, times = c(14, 9, 7))
  cx <- c(0, 3, 1.5)[g]; cy <- c(0, 1, 4)[g]
  data.frame(x = round(cx + 1.3 * sin(r * 2.9) + 0.4 * cos(r * 7.1), 3),
             y = round(cy + 1.1 * cos(r * 3.3) + 0.5 * sin(r * 5.3), 3))
})
di <- local({
  r <- 1:45; g <- rep(1:3, times = c(20, 15, 10))
  data.frame(g = g,
    y1 = round(c(0, 1.2, 2.4)[g] + 1.1 * sin(r * 2.3) + 0.3 * cos(r * 5.9), 3),
    y2 = round(c(0, 1.5, 0.8)[g] + 1.0 * cos(r * 3.1) + 0.4 * sin(r * 6.7), 3),
    y3 = round(c(1, 0, 1.5)[g] + 0.9 * sin(r * 4.1 + 1) + 0.3 * cos(r * 8.3), 3))
})
"#;

/// The context of a dialog working on the data frame `dataset`.
fn context(dataset: &str, columns: &[(&str, VarKind)]) -> Context {
    Context {
        dataset: dataset.into(),
        frames: Vec::new(),
        columns: columns
            .iter()
            .map(|(name, kind)| ColumnInfo {
                name: (*name).into(),
                kind: *kind,
            })
            .collect(),
    }
}

fn fa_context() -> Context {
    context(
        "fa",
        &[
            ("v1", VarKind::Number),
            ("v2", VarKind::Number),
            ("v3", VarKind::Number),
            ("v4", VarKind::Number),
            ("v5", VarKind::Number),
            ("v6", VarKind::Number),
        ],
    )
}

/// Runs the dialog's code on the data frame of `ctx`.
fn dialog(
    e: &mut Engine,
    ctx: &Context,
    spec: &Spec,
    set: &[(&str, &str)],
    lists: &[(&str, &[&str])],
    flags: &[&str],
) -> (Outcome, String) {
    let mut values = spec.defaults(ctx);
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
    let code = spec.generate(ctx, &values, Style::Base);
    assert!(code.is_ok(), "{code:?}");
    let code = code.unwrap_or_default();
    run(e, Source::Menu(spec.menu.into()), &code)
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
fn pca_matches_stata() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    define(&mut e, MAKE);
    let ctx = fa_context();
    let vars: &[&str] = &["v1", "v2", "v3", "v4", "v5", "v6"];
    let (outcome, text) = dialog(
        &mut e,
        &ctx,
        &PRINCIPAL_COMPONENTS,
        &[],
        &[("x", vars)],
        &["scale"],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(
        text.contains("Eigenvalue") && text.contains("Loadings (eigenvectors)"),
        "{text}"
    );
    // the covariance matrix: scaling off, its scores shown
    let (outcome, text) = dialog(
        &mut e,
        &ctx,
        &PRINCIPAL_COMPONENTS,
        &[("save", "pcov")],
        &[("x", vars)],
        &["scores"],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(text.contains("Component scores"), "{text}");
    // signed as Stata signs them (small-fixes row 4), for both matrices
    for (save, flags) in [
        ("pstata", &["scale", "stata_sign"][..]),
        ("pcstata", &["stata_sign"][..]),
    ] {
        let (outcome, text) = dialog(
            &mut e,
            &ctx,
            &PRINCIPAL_COMPONENTS,
            &[("save", save)],
            &[("x", vars)],
            flags,
        );
        assert_eq!(outcome, Outcome::Ok, "{text}");
    }
    // the eigenproblems, written out
    define(
        &mut e,
        "S <- stats::cor(fa); ev <- eigen(S, symmetric = TRUE); \
         Sc <- stats::cov(fa); evc <- eigen(Sc, symmetric = TRUE); \
         big <- function(m) all(apply(m, 2, function(v) v[which.max(abs(v))] > 0))",
    );
    define(
        &mut e,
        // Stata 19: pca v1-v6 (e(Ev), e(L)), and with covariance
        "stata_L <- matrix(c(0.502720276678799, 0.507574826059692, 0.484077169754609, 0.0876212139390659, \
           0.0729873710962595, 0.492244633914007, -0.114510208535459, -0.270119377422029, \
           0.125667460538489, 0.667510607039092, 0.670583488485705, 0.053646974795921, \
           -0.402072865515679, 0.246822560622486, -0.527422423617336, 0.219233269250459, \
           -0.141161293859698, 0.656697711446444, -0.208781792306897, 0.138808144523378, \
           -0.0572749940870249, -0.667447978754781, 0.683878409831664, 0.143824731842968, \
           0.658083528551194, 0.116965889508953, -0.684361456427385, 0.0741519113016593, \
           0.227255312643128, -0.166586205981655, -0.309569667480356, 0.758639058554676, \
           0.00845825979201844, 0.218411482569879, 0.0756932043729436, -0.524527235466425), 6);
         stata_ev <- c(2.83937184202909, 1.75846065659262, 0.569981520609631, 0.354086552789133, \
           0.314188296514895, 0.163911131464638);
         stata_Lc <- matrix(c(0.589631739268124, 0.542120467961187, 0.504127238065609, 0.0422427905314714, \
           0.033926983627875, 0.318370922880151, -0.0670642711634121, -0.237988240972586, \
           0.190892365801435, 0.692878971315289, 0.646482781919909, 0.0663538179835039, \
           -0.303983186816655, 0.491058700331067, -0.514046645477482, 0.356145897429923, \
           -0.132787990344299, 0.507680086865202, -0.677676822776569, 0.203273123670095, \
           0.452279975895831, -0.407824019247627, 0.285860505309468, 0.216426258336298, \
           0.269910109321625, 0.151898747735198, -0.490375059911666, -0.431458335176932, \
           0.690976733036516, 0.00156950553676934, 0.152753639796898, -0.586459850147785, \
           -0.0121244716468984, -0.19702195774095, -0.0641190142604408, 0.767890139967244), 6);
         stata_evc <- c(1.33099732241673, 0.81384910337236, 0.223703803921598, 0.166500666255201, \
           0.158325085753631, 0.0614426112291954)",
    );
    holds(
        &mut e,
        &[
            // the eigenproblem of the correlation matrix, and of the covariance matrix
            same("pca$sdev^2", "ev$values"),
            same("abs(unclass(pca$rotation))", "abs(ev$vectors)"),
            same("pcov$sdev^2", "evc$values"),
            same("abs(unclass(pcov$rotation))", "abs(evc$vectors)"),
            // each component signed so that its largest loading is positive
            "big(unclass(pca$rotation))".to_string(),
            // the scores are the data, centred and scaled, times the loadings
            same(
                "abs(pca$x)",
                "abs(scale(fa) %*% pca$rotation)",
            ),
            // the dialog's table: eigenvalues, shares, running shares
            // Stata 19: eigenvalues and eigenvectors, up to Stata's sign rule
            // (the elements of each component sum to more than 0)
            same("pca$sdev^2", "stata_ev"),
            same("abs(unclass(pca$rotation))", "abs(stata_L)"),
            same("pcov$sdev^2", "stata_evc"),
            same("abs(unclass(pcov$rotation))", "abs(stata_Lc)"),
            // each component is Stata's up to its sign, and Stata's sum to more than 0
            "all(sapply(1:6, function(j) { a <- unname(unclass(pca$rotation)[, j]); b <- stata_L[, j]; \n               isTRUE(all.equal(a, b, tolerance = 1e-9)) || isTRUE(all.equal(a, -b, tolerance = 1e-9)) }))"
                .to_string(),
            "all(colSums(stata_L) > 0) && all(colSums(stata_Lc) > 0)".to_string(),
            // with "sign as Stata", the loadings and scores are Stata's exactly
            same("unclass(pstata$rotation)", "stata_L"),
            same("unclass(pcstata$rotation)", "stata_Lc"),
            same("pstata$x", "scale(fa) %*% stata_L"),
        ],
    );
}

#[test]
fn factor_analysis_matches_stata() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    define(&mut e, MAKE);
    let ctx = fa_context();
    let vars: &[&str] = &["v1", "v2", "v3", "v4", "v5", "v6"];
    let (outcome, text) = dialog(
        &mut e,
        &ctx,
        &FACTOR_ANALYSIS,
        &[("factors", "2"), ("save", "fnone")],
        &[("x", vars)],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(
        text.contains("Uniquenesses") && text.contains("Loadings"),
        "{text}"
    );
    let (outcome, text) = dialog(
        &mut e,
        &ctx,
        &FACTOR_ANALYSIS,
        &[("factors", "2"), ("rotation", "varimax"), ("save", "fav")],
        &[("x", vars)],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(text.contains("SS loadings"), "{text}");
    // one factor, and too many factors, which the dialog does not let through
    let (outcome, text) = dialog(
        &mut e,
        &ctx,
        &FACTOR_ANALYSIS,
        &[("factors", "1"), ("save", "fa1")],
        &[("x", vars)],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    // when the tightest stop fails (it does on some platforms: "unable to optimize
    // from this starting value"), the dialog asks again looser: make the first
    // attempt fail (an NA stop) and the fit must still come back, from the next one
    let mut values = FACTOR_ANALYSIS.defaults(&ctx);
    values.set_text("factors", "2");
    values.set_text("save", "fretry");
    values.set_list("x", vars.iter().map(|n| n.to_string()).collect());
    let code = FACTOR_ANALYSIS.generate(&ctx, &values, Style::Base);
    assert!(code.is_ok(), "{code:?}");
    let code = code.unwrap_or_default();
    assert!(code.contains("c(10, 1e3,"), "{code}");
    let (outcome, text) = run(
        &mut e,
        Source::Typed,
        &code.replace("c(10, 1e3,", "c(NA, 1e3,"),
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    holds(
        &mut e,
        &[within("fretry$uniquenesses", "fnone$uniquenesses", "1e-4")],
    );
    // the criterion factanal minimises, written out: the eigenvalues of the
    // correlation matrix scaled by the uniquenesses, beyond the factors kept
    define(&mut e, "fa_ml <- fnone");
    define(
        &mut e,
        "fa_raw <- stats::factanal(~ v1 + v2 + v3 + v4 + v5 + v6, factors = 2, data = fa, \
         rotation = 'none', control = list(opt = list(factr = 10)))",
    );
    define(
        &mut e,
        "R <- stats::cor(fa); k <- 2; p <- 6; \
         crit <- function(psi) { s <- diag(1 / sqrt(psi)); \
           ev <- eigen(s %*% R %*% s, symmetric = TRUE, only.values = TRUE)$values[-(1:k)]; \
           -sum(log(ev) - ev) - k + p }; \
         lam <- function(psi) { s <- diag(sqrt(psi)); si <- diag(1 / sqrt(psi)); \
           ee <- eigen(si %*% R %*% si, symmetric = TRUE); \
           s %*% ee$vectors[, 1:k] %*% diag(sqrt(pmax(ee$values[1:k] - 1, 0))) }; \
         L0 <- lam(fa_ml$uniquenesses)",
    );
    // Stata 19: factor v1-v6, ml factors(2), at its default tolerance and with
    // tolerance(1e-14); rotate, varimax after the tight one
    define(
        &mut e,
        "stata_psi <- c(0.449154980428328, 0.0547760460145088, 0.516192139083965, 0.329482805469436, \
           0.397065363818655, 0.338267279601188);
         stata_L <- matrix(c(0.728591397584142, 0.971218327947429, 0.581779755540873, -0.130929198278157, \
           -0.152903147204768, 0.739739385597846, 0.141419921323179, -0.0442596151219379, \
           0.381235067923731, 0.808315990511328, 0.761285270655421, 0.338405616416211), 6);
         loose_psi <- c(0.449146429550742, 0.0547864652027622, 0.516183561025383, 0.329495392725672, \
           0.397054500453951, 0.338267096755076);
         loose_L <- matrix(c(0.728602135574486, 0.971250967294222, 0.581800171912483, -0.130925709707194, \
           -0.152900901787428, 0.739707673268102, 0.14142372926644, -0.0442655159392015, \
           0.381250784130655, 0.808304184115967, 0.761297873047539, 0.338379206982125), 6);
         stata_rL <- matrix(c(0.741633202551716, 0.953127186117463, 0.633085763830666, -0.00620785724453685, \
           -0.0350928494893736, 0.782672670656257, 0.0287264974118355, -0.191761625868029, \
           0.28811504057746, 0.818827611885739, 0.775695254372964, 0.221711998851024), 6);
         stata_rEv <- c(2.47311537937742, 1.44194600549381);
         stata_chi2 <- 6.933830637194603",
    );
    define(
        &mut e,
        "A <- unclass(fa_ml$loadings); u <- A[, 1]^2 - A[, 2]^2; v <- 2 * A[, 1] * A[, 2]; \
         phi <- atan2(2 * sum(u * v) - 2 * sum(u) * sum(v) / 6, \
                      sum(u^2 - v^2) - (sum(u)^2 - sum(v)^2) / 6) / 4; \
         Tm <- matrix(c(cos(phi), sin(phi), -sin(phi), cos(phi)), 2); B <- A %*% Tm; \
         B <- B[, order(colSums(B^2), decreasing = TRUE)]; B <- sweep(B, 2, ifelse(colSums(B) < 0, -1, 1), '*')",
    );
    holds(
        &mut e,
        &[
            // the dialog's unrotated fit is factanal's, and is a stationary
            // point of the criterion: no step in any uniqueness lowers it
            "ncol(fa1$loadings) == 1 && length(fa1$uniquenesses) == 6".to_string(),
            "all(sapply(1:6, function(j) { psi <- fa_ml$uniquenesses; h <- 1e-5; \
               psi1 <- psi; psi1[j] <- psi[j] + h; psi2 <- psi; psi2[j] <- psi[j] - h; \
               abs(crit(psi1) - crit(psi2)) / (2 * h) < 1e-5 }))"
                .to_string(),
            // the loadings are the eigenvectors of the scaled matrix, at that point
            within("abs(L0)", "abs(unclass(fa_ml$loadings))", "1e-6"),
            // Stata 19, tolerance(1e-14): uniquenesses and loadings
            within("fa_ml$uniquenesses", "stata_psi", "1e-6"),
            within("unclass(fa_ml$loadings)", "stata_L", "1e-6"),
            // Stata at its default tolerance stops about 3e-5 short
            within("fa_ml$uniquenesses", "loose_psi", "1e-4"),
            within("unclass(fa_ml$loadings)", "loose_L", "1e-4"),
            // the test of the number of factors: factanal's own statistic uses
            // N - 1 where Stata's uses N, N - (2p+5)/6 - 2k/3 times the same
            // criterion; the dialog's is Stata's (and its p-value goes with it),
            // with or without the rotation
            // (stats::factanal itself, run here on the data, gives the N - 1 figure)
            within(
                "fa_raw$STATISTIC * (40 - (2 * 6 + 5) / 6 - 2 * 2 / 3) / (40 - 1 - (2 * 6 + 5) / 6 - 2 * 2 / 3)",
                "stata_chi2",
                "1e-6",
            ),
            "fa_raw$STATISTIC < stata_chi2".to_string(),
            within("fnone$STATISTIC", "stata_chi2", "1e-6"),
            within("fav$STATISTIC", "stata_chi2", "1e-6"),
            within(
                "fnone$PVAL",
                "pchisq(stata_chi2, fnone$dof, lower.tail = FALSE)",
                "1e-6",
            ),
            "fa1$STATISTIC > 0 && fa1$PVAL >= 0 && fa1$PVAL <= 1".to_string(),
            // the varimax dialog: Kaiser's angle (two factors), normalisation off
            within("unclass(fav$loadings)", "B", "1e-12"),
            // Stata 19: rotate, varimax (after the tight fit): rotated loadings
            // and variances; Stata orders by variance and signs by the sum
            within("unclass(fav$loadings)", "stata_rL", "1e-6"),
            within("colSums(unclass(fav$loadings)^2)", "stata_rEv", "1e-6"),
            // the uniquenesses are what the rotation leaves alone
            same("fav$uniquenesses", "fa_ml$uniquenesses"),
        ],
    );
}

/// Single-linkage heights etc. by Lance and Williams's update, merge by merge.
const LANCE_WILLIAMS: &str = r#"
lw <- function(P, linkage) {
  D <- as.matrix(dist(P)); if (linkage == "ward") D <- D^2
  n <- nrow(P); size <- rep(1, n); active <- rep(TRUE, n); heights <- numeric(0)
  for (s in seq_len(n - 1)) {
    M <- D; M[!active, ] <- Inf; M[, !active] <- Inf; diag(M) <- Inf
    k <- which(M == min(M), arr.ind = TRUE)[1, ]; i <- min(k); j <- max(k); v <- D[i, j]
    heights <- c(heights, v)
    for (m in which(active)) {
      if (m == i || m == j) next
      a <- D[m, i]; b <- D[m, j]; si <- size[i]; sj <- size[j]; sm <- size[m]
      D[m, i] <- D[i, m] <- switch(linkage, single = min(a, b), complete = max(a, b),
        average = (si * a + sj * b) / (si + sj),
        ward = ((si + sm) * a + (sj + sm) * b - sm * v) / (si + sj + sm))
    }
    size[i] <- size[i] + size[j]; active[j] <- FALSE
  }
  sort(heights)
}
same_partition <- function(a, b) { t <- table(a, b); all(rowSums(t > 0) == 1) && all(colSums(t > 0) == 1) }
"#;

#[test]
fn cluster_analysis_matches_stata() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    define(&mut e, MAKE);
    define(&mut e, LANCE_WILLIAMS);
    let ctx = context("cl", &[("x", VarKind::Number), ("y", VarKind::Number)]);
    let vars: &[&str] = &["x", "y"];
    // the four linkages, each cut into three groups
    for (method, save) in [
        ("ward", "cw"),
        ("average", "ca"),
        ("complete", "cc"),
        ("single", "cs"),
    ] {
        let (outcome, text) = dialog(
            &mut e,
            &ctx,
            &CLUSTER_ANALYSIS,
            &[("method", method), ("save", save)],
            &[("x", vars)],
            &[],
        );
        assert_eq!(outcome, Outcome::Ok, "{method}: {text}");
        assert!(
            text.contains("Group sizes") && text.contains("Group means"),
            "{text}"
        );
    }
    // k-means: from the first rows (Stata's start(firstk)), then from random rows
    let (outcome, text) = dialog(
        &mut e,
        &ctx,
        &CLUSTER_ANALYSIS,
        &[("method", "kmeans"), ("save", "ck")],
        &[("x", vars)],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let (outcome, text) = dialog(
        &mut e,
        &ctx,
        &CLUSTER_ANALYSIS,
        &[
            ("method", "kmeans"),
            ("start", "random"),
            ("seed", "7"),
            ("save", "cr"),
        ],
        &[("x", vars)],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    // Lloyd's iterations from the first three rows, written out
    define(
        &mut e,
        "P <- as.matrix(cl); ctr <- P[1:3, ]; \
         repeat { dd <- sapply(1:3, function(k) rowSums(sweep(P, 2, ctr[k, ])^2)); \
           g <- max.col(-dd, ties.method = 'first'); \
           new <- t(sapply(1:3, function(k) colMeans(P[g == k, , drop = FALSE]))); \
           if (isTRUE(all.equal(new, ctr, tolerance = 0))) break; ctr <- new }",
    );
    // Stata 19: cluster <linkage>linkage x y; cluster generate g3 = groups(3);
    // the six largest heights; cluster kmeans x y, k(3) start(firstk)
    define(
        &mut e,
        "s_ward <- c(1,1,1,1,1,1,1,1,1,1,1,1,1,1,2,3,2,3,2,3,2,3,2,2,2,2,2,2,2,2); \
         s_avg <- s_ward; \
         s_comp <- c(1,1,1,1,1,1,1,1,1,1,1,1,1,1,2,2,2,2,2,2,2,2,2,3,3,3,3,3,3,3); \
         s_single <- c(1,1,1,1,1,1,1,1,1,1,1,1,1,1,3,2,3,2,3,2,3,2,3,3,3,3,3,3,3,3); \
         s_km <- c(1,1,1,1,1,1,1,1,1,1,1,1,1,1,3,3,2,3,2,3,2,3,2,2,2,2,2,2,2,2); \
         h_single <- c(1.74885019369871, 1.56860989414194, 1.2126574124624, 1.03792292584758, \
           1.00659674150079, .904690554830766); \
         h_comp <- c(6.5437496895893, 5.97268490714185, 3.84081371586803, 3.69259001244384, \
           1.97758261521485, 1.83815668537804); \
         h_avg <- c(4.15626798637414, 3.96922799803874, 2.29117241748626, 2.21749025744897, \
           1.55382417023969, 1.34521837241599); \
         h_ward <- c(170.333408595238, 96.1988368333333, 30.3911881547619, 26.3235798095238, \
           5.31929465714286, 3.8551992); \
         top6 <- function(m) rev(tail(sort(m$height), 6))",
    );
    holds(
        &mut e,
        &[
            // the heights are the merge-by-merge ones, all of them
            same("sort(cw$height)", "lw(cl, 'ward')"),
            same("sort(ca$height)", "lw(cl, 'average')"),
            same("sort(cc$height)", "lw(cl, 'complete')"),
            same("sort(cs$height)", "lw(cl, 'single')"),
            // Stata 19: the heights of the last six joins
            same("top6(cw)", "h_ward"),
            same("top6(ca)", "h_avg"),
            same("top6(cc)", "h_comp"),
            same("top6(cs)", "h_single"),
            // Stata 19: the three groups, as a partition of the rows
            "same_partition(cw$groups, s_ward)".to_string(),
            "same_partition(ca$groups, s_avg)".to_string(),
            "same_partition(cc$groups, s_comp)".to_string(),
            "same_partition(cs$groups, s_single)".to_string(),
            // the group sizes the dialog prints, sorted: 14, 12, 4 and 14, 9, 7
            "identical(sort(as.vector(table(cw$groups))), c(4L, 12L, 14L))".to_string(),
            "identical(sort(as.vector(table(cc$groups))), c(7L, 9L, 14L))".to_string(),
            // k-means from the first rows: Lloyd's centres, Stata's groups
            same("ck$centers[order(ck$centers[, 1]), ]", "ctr[order(ctr[, 1]), ]"),
            "same_partition(ck$groups, s_km)".to_string(),
            "identical(sort(as.vector(table(ck$groups))), c(5L, 11L, 14L))".to_string(),
            // a random start with a seed is repeatable, and every row has a group
            "identical(unname(cr$groups), unname(local({ set.seed(7); \
               stats::kmeans(as.matrix(cl), centers = 3, algorithm = 'Lloyd', iter.max = 1000)$cluster })))"
                .to_string(),
            "length(cr$groups) == 30 && !anyNA(cr$groups)".to_string(),
        ],
    );
}

#[test]
fn discriminant_analysis_matches_stata() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    define(&mut e, MAKE);
    let ctx = context(
        "di",
        &[
            ("g", VarKind::Number),
            ("y1", VarKind::Number),
            ("y2", VarKind::Number),
            ("y3", VarKind::Number),
        ],
    );
    let vars: &[&str] = &["y1", "y2", "y3"];
    let (outcome, text) = dialog(
        &mut e,
        &ctx,
        &DISCRIMINANT_ANALYSIS,
        &[("group", "g"), ("save", "ld")],
        &[("x", vars)],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(
        text.contains("Prior probabilities of groups")
            && text.contains("Coefficients of linear discriminants")
            && text.contains("Correctly classified"),
        "{text}"
    );
    let (outcome, text) = dialog(
        &mut e,
        &ctx,
        &DISCRIMINANT_ANALYSIS,
        &[("group", "g"), ("priors", "proportional"), ("save", "lp")],
        &[("x", vars)],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    // the pooled covariance, the linear scores and the sphered eigenproblem
    define(
        &mut e,
        "X <- as.matrix(di[, 2:4]); g <- di$g; N <- nrow(X); G <- 3; \
         mu <- t(sapply(1:3, function(k) colMeans(X[g == k, ]))); gm <- colMeans(X); \
         W <- Reduce(`+`, lapply(1:3, function(k) crossprod(sweep(X[g == k, ], 2, mu[k, ])))); \
         Tm <- crossprod(sweep(X, 2, gm)); B <- Tm - W; S <- W / (N - G); Si <- solve(S); \
         cls <- function(pri) { sc <- X %*% t(mu %*% Si) + \
             rep(-0.5 * rowSums((mu %*% Si) * mu) + log(pri), each = N); max.col(sc, ties.method = 'first') }; \
         counts <- function(pri) table(factor(g, 1:3), factor(cls(pri), 1:3)); \
         Q <- solve(chol(S)); M <- t(Q) %*% B %*% Q / (N - G); em <- eigen((M + t(M)) / 2, symmetric = TRUE); \
         U <- Q %*% em$vectors[, 1:2]",
    );
    // Stata 19: discrim lda y1 y2 y3, group(g): e(L_unstd), e(Ev), then
    // estat classtable with equal and with proportional priors
    define(
        &mut e,
        "s_U <- matrix(c(-0.862181485061325, -0.874769396308118, 0.591076775081071, 0.733379542286395, \
           -0.428385672455923, 0.929909911262688), 3);
         s_Ev <- c(1.70687214853526, 0.972322714320031);
         s_equal <- matrix(c(18, 0, 0, 1, 13, 0, 1, 2, 10), 3); \
         s_prop <- matrix(c(19, 0, 1, 1, 14, 0, 0, 1, 9), 3); \
         tab <- function(m) { cls <- stats::predict(m, stats::model.frame(g ~ y1 + y2 + y3, data = di))$class; \
           unclass(table(factor(di$g, 1:3), cls)) }",
    );
    holds(
        &mut e,
        &[
            // the model is lda's, with the priors asked for
            same("ld$prior", "rep(1 / 3, 3)"),
            same("lp$prior", "c(20, 15, 10) / 45"),
            same("ld$means", "mu"),
            // the linear scores written out classify every row as predict() does
            "identical(as.integer(stats::predict(ld, di)$class), as.integer(cls(rep(1 / 3, 3))))"
                .to_string(),
            "identical(as.integer(stats::predict(lp, di)$class), as.integer(cls(c(20, 15, 10) / 45)))"
                .to_string(),
            // lda's scaling is the sphered eigenvectors, up to sign. MASS weighs the
            // group means by the priors; the dialog takes the canonical functions
            // from the fit with the group sizes as priors, as Stata weighs them, so
            // they are Stata's with equal priors too
            same("abs(lp$scaling)", "abs(U)"),
            same("abs(ld$scaling)", "abs(U)"),
            same("lp$svd^2 / sum(lp$svd^2)", "em$values[1:2] / sum(em$values[1:2])"),
            // Stata 19: the canonical function coefficients (up to sign), their
            // eigenvalues' shares, and the two classification tables
            same("abs(lp$scaling)", "abs(s_U)"),
            same("abs(ld$scaling)", "abs(s_U)"),
            same("lp$svd^2 / sum(lp$svd^2)", "s_Ev / sum(s_Ev)"),
            same("ld$svd^2 / sum(ld$svd^2)", "s_Ev / sum(s_Ev)"),
            same("unname(tab(ld))", "s_equal"),
            same("unname(tab(lp))", "s_prop"),
            same("unname(counts(rep(1 / 3, 3)))", "s_equal"),
            same("unname(counts(c(20, 15, 10) / 45))", "s_prop"),
        ],
    );
}

// ---- P6.7c: principal factors, oblique rotations, other dissimilarities,
// stopping rules, quadratic discriminant analysis ----

/// What Stata 19 reported on `fa`, `cl` and `di` (`factor ..., pf | ipf | pcf
/// factors()` with `e(L)`, `e(Psi)`, `e(Ev)`; `rotate, varimax | promax[()] |
/// oblimin[()] oblique` with `e(r_L)` and `e(r_Phi)`, `_t` at
/// `tolerance(1e-13) gtolerance(1e-13) ltolerance(1e-13)`; the six largest
/// heights and the three groups of `cluster ...linkage, measure()`; `cluster
/// kmeans, measure() start(firstk)`; `predict double, pr` after `discrim
/// qda`, rows 1, 6, ..., 41). Only numbers are kept.
const STATA_C: &str = r#"
s_pf_l <- matrix(c(0.781711060488896, -0.103428116204682, 0.842061236515291, -0.321120840978291, 0.723687792540141, 0.176110733541267, 0.11173771145422, 0.744610644484511, 0.0883884554264514, 0.732534271889572, 0.773783061371424, 0.0980332452816684), 6, byrow = TRUE)
s_pf_psi <- c(0.378230442687676, 0.187814279447735, 0.445260988459935, 0.433069671959335, 0.455581021454541, 0.391649256754211)
s_pf_ev <- c(2.46290139048299, 1.24549294875358, 0.163050669081259, -0.128691761806244, -0.145017596393854, -0.194560744018675)
s_ipf_l <- matrix(c(0.783094559280537, -0.0904632731712465, 0.888490515107777, -0.329895061741565, 0.727100102471638, 0.180723275514864, 0.112530483490098, 0.793362993444015, 0.0892119330529319, 0.790772091556373, 0.750424121697087, 0.0969081190188785), 6, byrow = TRUE)
s_ipf_psi <- c(0.378579307432367, 0.101753852802045, 0.438664538672912, 0.357912050919037, 0.366720730216519, 0.427472454043378)
s_ipf1_psi <- c(0.357610781992704, 0.30447336406525, 0.472565769930967, 0.992909645740815, 0.995700337282762, 0.428230935536863)
s_ipf3_psi <- c(0.233878569706957, 0.262759515751732, 0.374768636344839, 0.408117456009877, 0.312833527335365, -0.755403748677332)
s_ipf5_psi <- c(0.238231649769926, 0.317102470547971, 0.371801326708059, 0.438907659419383, 0.283599757267977)
s_pcf_l <- matrix(c(0.847105571103192, -0.151848509560925, 0.855285698336012, -0.358197102159311, 0.8156911236047, 0.166643802566886, 0.147645563383613, 0.885165541931635, 0.122986900556868, 0.88924039668645, 0.82945365659207, 0.0711396238763342), 6, byrow = TRUE)
s_pcf_psi <- c(0.25935418155006, 0.140181210226552, 0.306877833938551, 0.194682750990011, 0.194125739192341, 0.306945785480778)
s_pf2_vl <- matrix(c(0.788505329498955, 0.00537611979671697, 0.878303526235322, -0.201912447250941, 0.692481812327637, 0.274240681036743, 0.00797105346327366, 0.7529055653582, -0.0134894463166567, 0.737724212279582, 0.752866984998819, 0.203818659951872), 6, byrow = TRUE)
s_pf2_pl <- matrix(c(0.793011773853345, -0.0412650608485141, 0.899291597502606, -0.255409605702744, 0.67573827128086, 0.235281417162209, -0.0498082724611041, 0.758026256189016, -0.0702366141862328, 0.74400264090891, 0.741909142340079, 0.160761375613515), 6, byrow = TRUE)
s_pf2_pphi <- matrix(c(1, 0.134473287328104, 0.134473287328104, 0.999999999999999), 2, byrow = TRUE)
s_pf2_ol <- matrix(c(0.788821503440121, -0.0154899250167358, 0.879194827126321, -0.225169761516942, 0.692060576720436, 0.255935078677675, 0.00602198718110285, 0.752748799882009, -0.0154081295819688, 0.73813426950225, 0.752653645664257, 0.183910001137405), 6, byrow = TRUE)
s_pf2_olt <- matrix(c(0.788821517166108, -0.0154895494412599, 0.879195173346013, -0.225169352523822, 0.692060159761897, 0.255935420642804, 0.00602078895300009, 0.752748837552989, -0.0154093042513155, 0.738134296274815, 0.752653342507975, 0.18391036867765), 6, byrow = TRUE)
s_pf2_ophit <- matrix(c(1, 0.0290442918958042, 0.0290442918958042, 1), 2, byrow = TRUE)
s_pf3_l <- matrix(c(0.781711060488896, -0.103428116204682, -0.203068127647817, 0.842061236515291, -0.321120840978291, 0.108380177637406, 0.723687792540141, 0.176110733541267, -0.192506983977009, 0.11173771145422, 0.744610644484511, 0.0584444020548693, 0.0883884554264514, 0.732534271889572, -0.0293726179863796, 0.773783061371424, 0.0980332452816684, 0.262164650575366), 6, byrow = TRUE)
s_pf3_vl <- matrix(c(0.74755891762213, -0.00549304864105699, 0.322694457465546, 0.885886745261965, -0.196919082649416, 0.0189613531863122, 0.65244812601153, 0.263640579589486, 0.310810292747462, 0.0116540208566016, 0.754885978844531, -0.0189055260182153, -0.0230222733682987, 0.735322137436129, 0.0636636345031511, 0.782910362009932, 0.216139092797594, -0.131970848596226), 6, byrow = TRUE)
s_pf3_pl <- matrix(c(0.594457830949965, -0.070038609541064, 0.398069820815445, 0.86648049515796, -0.236918322040192, 0.0844274443449111, 0.493430247848867, 0.205158010107853, 0.378179222651777, -0.017628544941469, 0.758647160734842, -0.0203063670738518, -0.0852144827992034, 0.732066096447294, 0.0652616927767794, 0.80915237392895, 0.197163585577307, -0.0844899516665766), 6, byrow = TRUE)
s_pf3_pphi <- matrix(c(0.999999999999997, 0.0843006766332891, 0.338827913948806, 0.0843006766332891, 0.999999999999999, 0.126913088987163, 0.338827913948806, 0.126913088987163, 0.999999999999897), 3, byrow = TRUE)
s_pf3_p4l <- matrix(c(0.532460916174464, -0.0796551975372057, 0.430435049674437, 0.854532642413326, -0.241262624507593, 0.100451044277548, 0.428439953532606, 0.196787053675407, 0.40827817349812, -0.0325620513019452, 0.760744783174726, -0.0210931409991806, -0.11211764303316, 0.732552556030215, 0.0689498898551884, 0.812779659183234, 0.197297450006136, -0.0793576188040252), 6, byrow = TRUE)
s_pf3_ol <- matrix(c(0.766003790087811, 0.0540106110605119, -0.0815458851366723, 0.274469423780375, 0.623461309461445, -0.27946862499652, 0.721310498176153, 0.0247828619155561, 0.195606644060518, -0.023900252026236, 0.106050767073284, 0.751018634539425, 0.109978226419488, -0.0529130031148309, 0.733243289961446, -0.0130060682122077, 0.821622724975175, 0.144345639380877), 6, byrow = TRUE)
s_pf3_olt <- matrix(c(0.76600384504211, 0.0540104563592195, -0.0815461191899224, 0.274469292415937, 0.623461148014731, -0.279468969091365, 0.721310679003401, 0.024782859150764, 0.195606434267761, -0.0238999091022124, 0.10605117239872, 0.751018593527572, 0.109978586722695, -0.0529126340276772, 0.733243280761723, -0.0130060482021681, 0.821622839039402, 0.144345287869298), 6, byrow = TRUE)
s_pf3_ophit <- matrix(c(1, 0.822730050235168, 0.0116020269855565, 0.822730050235168, 1, -0.0057366515484292, 0.0116020269855565, -0.0057366515484292, 1), 3, byrow = TRUE)
s_pf3_o25lt <- matrix(c(0.858478419971595, -0.0569734563925124, -0.0832222156407854, 0.20422285112632, 0.665451245774638, -0.301215913475613, 0.820957260328971, -0.0814409254819432, 0.194961902033641, -0.0206489790274724, 0.129593337958172, 0.747363291152444, 0.155940144772692, -0.0720667173162547, 0.735183999908972, -0.140351138475232, 0.93817941589312, 0.115620146736004), 6, byrow = TRUE)
s_pf3_o25phi <- matrix(c(1, 0.893857525973026, -0.0234695056647486, 0.893857525973026, 1, -0.00705490070531406, -0.0234695056647486, -0.00705490070531406, 1), 3, byrow = TRUE)
s_ml_pl <- matrix(c(0.745193271423763, -0.0215789574915459, 0.977709946933441, -0.258625847099875, 0.613062753430658, 0.247723282458164, -0.0778987415242375, 0.827175017756918, -0.103246585492575, 0.785595372572328, 0.769684325147412, 0.170474995457478), 6, byrow = TRUE)
s_pcf3_vl <- matrix(c(0.86009845809143, -0.0296050812983931, 0.897595350600241, -0.23268299541281, 0.783626224938579, 0.281162062963625, 0.0200222897747679, 0.897171308570534, -0.00496538198227248, 0.897691264182419, 0.810855634986871, 0.188593090353969), 6, byrow = TRUE)
s_pcf3_pl <- matrix(c(0.864075047677567, -0.0670796255313511, 0.911310829531607, -0.27243101423959, 0.772642419286994, 0.247996308845668, -0.0224473307246188, 0.899145452152155, -0.0475344070507176, 0.900753043864437, 0.80434143356061, 0.153950446122782), 6, byrow = TRUE)
s_pcf3_pphi <- matrix(c(1, 0.090400403306075, 0.090400403306075, 1), 2, byrow = TRUE)
s_pq <- matrix(c(0.997359240910283, 0.00264075896257451, 1.2714233020811e-10, 0.871374888834512, 0.119499309051568, 0.00912580211391917, 0.99999998220834, 1.7791132054649e-08, 5.2854044823073e-13, 0.975935769810808, 0.0240642301891921, 1.0066752388515e-16, 0.00510998031519929, 0.994890019684801, 1.1070076299228e-23, 0.0612141034612402, 0.938785896385649, 1.5311089117153e-10, 0.00451621020480413, 0.8514930629431, 0.143990726852095, 1.4853166493981e-06, 0.0181018292893514, 0.981896685393999, 0.00970645510650662, 0.0231582993326621, 0.967135245560831), 9, byrow = TRUE)
s_pp <- matrix(c(0.998018122299047, 0.00198187763734026, 6.3613161864932e-11, 0.902453331485557, 0.0928210271023983, 0.00472564141204462, 0.999999986656387, 1.3343349100339e-08, 2.6427022529085e-13, 0.981842591342714, 0.0181574086572861, 5.0638405504190e-17, 0.00680172153255212, 0.993198278467448, 7.3675016034303e-24, 0.0799866999111086, 0.920013299988859, 1.0003278830192e-10, 0.00631521757954885, 0.893010463879978, 0.100674318540473, 2.9439831855215e-06, 0.0269091515317419, 0.973087904485073, 0.0190083069048361, 0.0340134521006195, 0.946978240994544), 9, byrow = TRUE)
s_h_a1_hgt <- c(5.15513888888889, 4.85142857142857, 2.96254166666667, 2.92005714285714, 2.0229, 1.5985)
s_h_ci_hgt <- c(6.516, 5.477, 3.095, 3.009, 1.838, 1.71)
s_h_sc_hgt <- c(1.07594936708861, 1.04188880426504, 1.00861008610086, 1.00426706827309, 0.995238095238095, 0.798276108262478)
s_h_a2_hgt <- c(18.3858037307692, 16.745410952381, 5.76168022857143, 5.24230804166667, 2.5186041, 1.93625633333333)
s_h_w2_hgt <- c(30.7675095926516, 18.2553553547628, 10.0120116419578, 7.77635749456534, 2.72696316865943, 2.49588823740807)
s_g_ga1 <- c(1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 3, 2, 3, 2, 3, 2, 3, 2, 3, 3, 3, 3, 3, 3, 3, 3)
s_g_gci <- c(1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 3, 3, 3, 3, 3, 3, 3)
s_g_gsc <- c(3, 3, 3, 3, 3, 3, 3, 2, 3, 3, 1, 2, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3)
s_g_k1 <- c(1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 3, 3, 2, 3, 2, 3, 2, 3, 2, 2, 2, 2, 2, 2, 2, 2)
s_g_k2 <- c(3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 1, 1, 2, 1, 2, 1, 2, 1, 2, 2, 2, 2, 2, 2, 2, 2)
s_g_cq <- c(1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3)
s_ch_ward <- c(26.31788617634851, 42.3209015427328, 47.10457788519061, 71.37156927682965, 68.60791944580156, 66.6040401663316, 68.96362358292205)
s_duda_ward <- c(0.5154839772132365, 0.309295046812442, 0.2754341696105296, 0.3739568204013217, 0.5299545909252241, 0.3135851204481027, 0.4795204223276368, 0.2130864368786516)
s_t2_ward <- c(26.31788617634849, 31.26422309145373, 31.56757920401918, 16.74105526212421, 4.434770611706038, 6.56678045091266, 6.512501492377406, 14.77172502667506)
s_ch_a1 <- c(23.31713211316867, 42.32090154273278, 47.10457788519063, 71.37156927682965, 68.607919445802)
s_duda_a1 <- c(0.5456267497227268, 0.2869336272663078, 0.2754341696105295, 0.3739568204013216, 0.5299545909252241, 0.5322767646863963)
s_t2_a1 <- c(23.31713211316869, 39.76202466206632, 31.56757920401919, 16.74105526212422, 4.434770611706038, 2.636165617275295)
s_ch_k1 <- 44.45324879575385
"#;

/// Principal factors by hand: the eigenvectors of the correlation matrix with
/// `h` on its diagonal, each signed so its loadings sum above 0; Stata's ipf
/// stop; the oblimin criterion; Kaiser's angle for a pair of factors.
const FACTOR_HAND: &str = r#"
Rf <- cor(fa); smc <- 1 - 1 / diag(solve(Rf))
pf_hand <- function(R, k, h) { Rr <- R; diag(Rr) <- h; e <- eigen(Rr, symmetric = TRUE)
  L <- e$vectors[, 1:k, drop = FALSE] %*% diag(sqrt(e$values[1:k]), k)
  sweep(L, 2, ifelse(colSums(L) < 0, -1, 1), "*") }
ipf_hand <- function(R, k) { h <- 1 - 1 / diag(solve(R))
  repeat { L <- pf_hand(R, k, h); hn <- rowSums(L^2); if (sum((hn - h)^2) < 1e-8) return(L); h <- hn } }
angle <- function(x, y) { p <- length(x); u <- x^2 - y^2; v <- 2 * x * y
  atan2(2 * sum(u * v) - 2 * sum(u) * sum(v) / p, sum(u^2 - v^2) - (sum(u)^2 - sum(v)^2) / p) / 4 }
vcrit <- function(A) sum(apply(A^2, 2, function(c) mean(c^2) - mean(c)^2))
ob_f <- function(L, gam) { p <- nrow(L); k <- ncol(L); L2 <- L^2
  sum(L2 * ((diag(p) - gam / p) %*% L2 %*% (matrix(1, k, k) - diag(k)))) / 4 }
# no small turn of one oblique axis towards another lowers the criterion
ob_min <- function(A, L, gam, h = 1e-4) { Tm <- t(qr.solve(L, A)); f0 <- ob_f(L, gam); k <- ncol(A)
  all(sapply(seq_len(k), function(j) all(sapply(setdiff(seq_len(k), j), function(i) all(sapply(c(-h, h), function(s) {
    T2 <- Tm; T2[, j] <- T2[, j] + s * T2[, i]; T2[, j] <- T2[, j] / sqrt(sum(T2[, j]^2))
    ob_f(A %*% t(solve(T2)), gam) >= f0 - 1e-12 })))))) }
"#;

fn fa_dialog(e: &mut Engine, set: &[(&str, &str)], vars: &[&str], flags: &[&str]) -> String {
    let (outcome, text) = dialog(
        e,
        &fa_context(),
        &FACTOR_ANALYSIS,
        set,
        &[("x", vars)],
        flags,
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    text
}

#[test]
fn principal_factors_and_oblique_rotations_match_stata() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    define(&mut e, MAKE);
    define(&mut e, STATA_C);
    define(&mut e, FACTOR_HAND);
    let all: &[&str] = &["v1", "v2", "v3", "v4", "v5", "v6"];
    let text = fa_dialog(
        &mut e,
        &[("method", "pf"), ("factors", "2"), ("save", "fpf")],
        all,
        &[],
    );
    assert!(
        text.contains("Principal factors, 40 rows, 2 factors") && text.contains("Uniquenesses"),
        "{text}"
    );
    for (method, k, save) in [
        ("ipf", "2", "fipf"),
        ("ipf", "1", "fipf1"),
        ("ipf", "3", "fipf3"),
        ("pcf", "2", "fpcf"),
        ("pf", "3", "fpf3"),
        ("pf", "4", "fpf4"),
    ] {
        fa_dialog(
            &mut e,
            &[("method", method), ("factors", k), ("save", save)],
            all,
            &[],
        );
    }
    fa_dialog(
        &mut e,
        &[("method", "ipf"), ("factors", "2"), ("save", "fipf5")],
        &["v1", "v2", "v3", "v4", "v5"],
        &[],
    );
    holds(
        &mut e,
        &[
            // by hand: the reduced correlation matrix and Stata's stop
            same("unclass(fpf$loadings)", "pf_hand(Rf, 2, smc)"),
            same("unclass(fpcf$loadings)", "pf_hand(Rf, 2, rep(1, 6))"),
            same("unclass(fipf$loadings)", "ipf_hand(Rf, 2)"),
            same("fpf$uniquenesses", "1 - rowSums(pf_hand(Rf, 2, smc)^2)"),
            // Stata: loadings, uniquenesses and eigenvalues
            same("unclass(fpf$loadings)", "s_pf_l"),
            same("fpf$uniquenesses", "s_pf_psi"),
            same("fpf$eigenvalues", "s_pf_ev"),
            same("unclass(fpcf$loadings)", "s_pcf_l"),
            same("fpcf$uniquenesses", "s_pcf_psi"),
            same("unclass(fipf$loadings)", "s_ipf_l"),
            same("fipf$uniquenesses", "s_ipf_psi"),
            same("fipf1$uniquenesses", "s_ipf1_psi"),
            // three factors: a uniqueness below 0 after 3642 rounds, as in Stata
            same("fipf3$uniquenesses", "s_ipf3_psi"),
            "fipf3$uniquenesses[6] < 0".into(),
            same("fipf5$uniquenesses", "s_ipf5_psi"),
            same("unclass(fpf3$loadings)", "s_pf3_l"),
            // four asked for, three eigenvalues above 0: three kept, as Stata
            "fpf4$factors == 3".into(),
            same("unclass(fpf4$loadings)", "s_pf3_l"),
        ],
    );

    // ---- rotations of the principal factors ----
    for (k, rotation, extra, save) in [
        ("2", "varimax", None, "fv2"),
        ("2", "promax", None, "fp2"),
        ("2", "oblimin", None, "fo2"),
        ("3", "varimax", None, "fv3"),
        ("3", "promax", None, "fp3"),
        ("3", "promax", Some(("power", "4")), "fp34"),
        ("3", "oblimin", None, "fo3"),
        ("3", "oblimin", Some(("gamma", "0.25")), "fo325"),
    ] {
        let mut set = vec![
            ("method", "pf"),
            ("factors", k),
            ("rotation", rotation),
            ("save", save),
        ];
        if let Some(x) = extra {
            set.push(x);
        }
        let text = fa_dialog(&mut e, &set, all, &[]);
        if rotation != "varimax" {
            assert!(text.contains("Factor correlations"), "{text}");
        }
    }
    fa_dialog(
        &mut e,
        &[
            ("method", "pf"),
            ("factors", "3"),
            ("rotation", "varimax"),
            ("save", "fx3"),
        ],
        all,
        &["exact"],
    );
    fa_dialog(
        &mut e,
        &[
            ("method", "pcf"),
            ("factors", "3"),
            ("rotation", "promax"),
            ("save", "fpcfp"),
        ],
        all,
        &[],
    );
    let text = fa_dialog(
        &mut e,
        &[("factors", "2"), ("rotation", "promax"), ("save", "fmlp")],
        all,
        &[],
    );
    assert!(text.contains("Factor Correlations"), "{text}");
    define(
        &mut e,
        "A2 <- pf_hand(Rf, 2, smc); A3 <- pf_hand(Rf, 3, smc); \
         phi <- angle(A2[, 1], A2[, 2]); \
         K2 <- A2 %*% matrix(c(cos(phi), sin(phi), -sin(phi), cos(phi)), 2); \
         K2 <- K2[, order(colSums(K2^2), decreasing = TRUE)]; K2 <- sweep(K2, 2, ifelse(colSums(K2) < 0, -1, 1), '*'); \
         V <- unclass(fv2$loadings); Q <- V * abs(V)^2; U <- qr.coef(qr(V), Q); \
         U <- U %*% diag(sqrt(diag(solve(crossprod(U))))); P2 <- V %*% U",
    );
    holds(
        &mut e,
        &[
            // two factors: Kaiser's angle, exactly, and Stata's
            same("unclass(fv2$loadings)", "K2"),
            within("unclass(fv2$loadings)", "s_pf2_vl", "1e-12"),
            // promax: the varimax loadings cubed as the target, least squares
            same("unclass(fp2$loadings)", "P2"),
            same("unclass(fp2$loadings)", "s_pf2_pl"),
            same("fp2$Phi", "s_pf2_pphi"),
            // oblimin: a minimum of the criterion; Stata's tight run, and its
            // default stop within 1e-5
            "ob_min(A2, unclass(fo2$loadings), 0)".into(),
            within("unclass(fo2$loadings)", "s_pf2_olt", "1e-7"),
            within("fo2$Phi", "s_pf2_ophit", "1e-7"),
            within("unclass(fo2$loadings)", "s_pf2_ol", "1e-5"),
            // three factors: Stata's varimax stops at a relative change of
            // 1e-8; the exact maximum is a little higher, every pair at angle 0
            within("unclass(fv3$loadings)", "s_pf3_vl", "1e-10"),
            "vcrit(unclass(fx3$loadings)) > vcrit(s_pf3_vl)".into(),
            "max(abs(unclass(fx3$loadings) - s_pf3_vl)) > 1e-5".into(),
            "all(abs(c(angle(unclass(fx3$loadings)[, 1], unclass(fx3$loadings)[, 2]), \
               angle(unclass(fx3$loadings)[, 1], unclass(fx3$loadings)[, 3]), \
               angle(unclass(fx3$loadings)[, 2], unclass(fx3$loadings)[, 3]))) < 1e-12)"
                .into(),
            within("unclass(fp3$loadings)", "s_pf3_pl", "1e-10"),
            within("fp3$Phi", "s_pf3_pphi", "1e-10"),
            within("unclass(fp34$loadings)", "s_pf3_p4l", "1e-10"),
            // oblique factors ordered by the structure's sums of squares
            "ob_min(A3, unclass(fo3$loadings), 0)".into(),
            within("unclass(fo3$loadings)", "s_pf3_olt", "1e-7"),
            within("fo3$Phi", "s_pf3_ophit", "1e-7"),
            "all(diff(colSums((unclass(fo3$loadings) %*% fo3$Phi)^2)) <= 0)".into(),
            "ob_min(A3, unclass(fo325$loadings), 0.25)".into(),
            within("unclass(fo325$loadings)", "s_pf3_o25lt", "1e-7"),
            within("fo325$Phi", "s_pf3_o25phi", "1e-5"),
            // principal-component factors keep eigenvalues above 1: two
            "ncol(unclass(fpcfp$loadings)) == 2".into(),
            within("unclass(fpcfp$loadings)", "s_pcf3_pl", "1e-10"),
            within("fpcfp$Phi", "s_pcf3_pphi", "1e-10"),
            // maximum likelihood with promax: Stata's to its fit's agreement
            within("unclass(fmlp$loadings)", "s_ml_pl", "1e-6"),
        ],
    );
}

fn cl_dialog(e: &mut Engine, set: &[(&str, &str)], flags: &[&str]) -> String {
    let ctx = context("cl", &[("x", VarKind::Number), ("y", VarKind::Number)]);
    let (outcome, text) = dialog(
        e,
        &ctx,
        &CLUSTER_ANALYSIS,
        set,
        &[("x", &["x", "y"])],
        flags,
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    text
}

/// Lance and Williams's update on a given dissimilarity matrix.
const LANCE_WILLIAMS_D: &str = r#"
lwd <- function(D, linkage) {
  n <- nrow(D); size <- rep(1, n); active <- rep(TRUE, n); heights <- numeric(0)
  for (s in seq_len(n - 1)) {
    M <- D; M[!active, ] <- Inf; M[, !active] <- Inf; diag(M) <- Inf
    k <- which(M == min(M), arr.ind = TRUE)[1, ]; i <- min(k); j <- max(k); v <- D[i, j]
    heights <- c(heights, v)
    for (m in which(active)) {
      if (m == i || m == j) next
      a <- D[m, i]; b <- D[m, j]; si <- size[i]; sj <- size[j]; sm <- size[m]
      D[m, i] <- D[i, m] <- switch(linkage, single = min(a, b), complete = max(a, b),
        average = (si * a + sj * b) / (si + sj),
        ward = ((si + sm) * a + (sj + sm) * b - sm * v) / (si + sj + sm))
    }
    size[i] <- size[i] + size[j]; active[j] <- FALSE
  }
  sort(heights)
}
P <- as.matrix(cl); n <- nrow(P)
canb <- matrix(0, n, n)
for (i in 1:n) for (j in 1:n) { a <- abs(P[i, ] - P[j, ]); b <- abs(P[i, ]) + abs(P[j, ])
  canb[i, j] <- sum(ifelse(b > 0, a / b, 0)) }
L1 <- as.matrix(dist(P, 'manhattan')); Linf <- as.matrix(dist(P, 'maximum')); L2 <- as.matrix(dist(P))
"#;

#[test]
fn cluster_dissimilarities_and_stopping_rules_match_stata() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    define(&mut e, MAKE);
    define(&mut e, STATA_C);
    define(&mut e, LANCE_WILLIAMS);
    define(&mut e, LANCE_WILLIAMS_D);
    let text = cl_dialog(
        &mut e,
        &[
            ("method", "average"),
            ("distance", "L1"),
            ("most", "6"),
            ("save", "ca1"),
        ],
        &["rules"],
    );
    assert!(
        text.contains("Stopping rules") && text.contains("Duda.Hart"),
        "{text}"
    );
    for (method, distance, save) in [
        ("complete", "Linf", "cci"),
        ("single", "canberra", "csc"),
        ("average", "L2squared", "ca2"),
        ("ward", "L2", "cw2"),
    ] {
        cl_dialog(
            &mut e,
            &[("method", method), ("distance", distance), ("save", save)],
            &[],
        );
    }
    cl_dialog(&mut e, &[("most", "8"), ("save", "cwr")], &["rules"]);
    let text = cl_dialog(
        &mut e,
        &[("method", "kmeans"), ("distance", "L1"), ("save", "ck1")],
        &["rules"],
    );
    assert!(
        text.contains("Calinski.Harabasz") && !text.contains("Duda"),
        "{text}"
    );
    cl_dialog(
        &mut e,
        &[("method", "kmeans"), ("distance", "Linf"), ("save", "ck2")],
        &[],
    );
    define(
        &mut e,
        "top6 <- function(m) rev(tail(sort(m$height), 6)); \
         lloyd <- function(far) { ctr <- P[1:3, ]; g <- 0; \
           repeat { D <- sapply(1:3, function(j) far(ctr[j, ])); gn <- max.col(-D, ties.method = 'first'); \
             if (identical(gn, g)) return(gn); g <- gn; \
             ctr <- t(sapply(1:3, function(j) colMeans(P[g == j, , drop = FALSE]))) } }; \
         g_l1 <- lloyd(function(c) rowSums(abs(sweep(P, 2, c)))); \
         g_linf <- lloyd(function(c) apply(abs(sweep(P, 2, c)), 1, max))",
    );
    holds(
        &mut e,
        &[
            // every merge height by Lance and Williams on the dissimilarities
            same("sort(ca1$height)", "lwd(L1, 'average')"),
            same("sort(cci$height)", "lwd(Linf, 'complete')"),
            same("sort(csc$height)", "lwd(canb, 'single')"),
            same("sort(ca2$height)", "lwd(L2^2, 'average')"),
            same("sort(cw2$height)", "lwd(L2, 'ward')"),
            // Stata: the six largest heights and the three groups
            same("top6(ca1)", "s_h_a1_hgt"),
            same("top6(cci)", "s_h_ci_hgt"),
            same("top6(csc)", "s_h_sc_hgt"),
            same("top6(ca2)", "s_h_a2_hgt"),
            same("top6(cw2)", "s_h_w2_hgt"),
            "same_partition(ca1$groups, s_g_ga1)".into(),
            "same_partition(cci$groups, s_g_gci)".into(),
            "same_partition(csc$groups, s_g_gsc)".into(),
            // k-means by the nearest centre in L1 and L-infinity
            "identical(unname(ck1$groups), g_l1)".into(),
            "identical(unname(ck2$groups), g_linf)".into(),
            "same_partition(ck1$groups, s_g_k1)".into(),
            "same_partition(ck2$groups, s_g_k2)".into(),
            // the stopping rules: Calinski-Harabasz from 2 groups, Duda-Hart from 1
            same("cwr$rules$Calinski.Harabasz[2:8]", "s_ch_ward"),
            same("cwr$rules$Duda.Hart[1:8]", "s_duda_ward"),
            same("cwr$rules$pseudo.T2[1:8]", "s_t2_ward"),
            same("ca1$rules$Calinski.Harabasz[2:6]", "s_ch_a1"),
            same("ca1$rules$Duda.Hart[1:6]", "s_duda_a1"),
            same("ca1$rules$pseudo.T2[1:6]", "s_t2_a1"),
            same("ck1$rules$Calinski.Harabasz", "s_ch_k1"),
            // by hand: one group's pseudo-T2 is two groups' pseudo-F
            same("cwr$rules$pseudo.T2[1]", "cwr$rules$Calinski.Harabasz[2]"),
        ],
    );
}

#[test]
fn quadratic_discriminant_matches_stata() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    define(&mut e, MAKE);
    define(&mut e, STATA_C);
    let ctx = context(
        "di",
        &[
            ("g", VarKind::Number),
            ("y1", VarKind::Number),
            ("y2", VarKind::Number),
            ("y3", VarKind::Number),
        ],
    );
    let vars: &[&str] = &["y1", "y2", "y3"];
    for (priors, save) in [("equal", "qe"), ("proportional", "qp")] {
        let (outcome, text) = dialog(
            &mut e,
            &ctx,
            &DISCRIMINANT_ANALYSIS,
            &[
                ("group", "g"),
                ("method", "quadratic"),
                ("priors", priors),
                ("save", save),
            ],
            &[("x", vars)],
            &[],
        );
        assert_eq!(outcome, Outcome::Ok, "{text}");
        assert!(
            text.contains("Correctly classified: 97.78%")
                && !text.contains("Coefficients of linear"),
            "{text}"
        );
    }
    // the posterior by hand: each group's own mean and covariance (n - 1)
    define(
        &mut e,
        "post <- function(prior) { X <- as.matrix(di[, 2:4]); \
           dens <- sapply(1:3, function(k) { Y <- X[di$g == k, ]; m <- colMeans(Y); S <- cov(Y); \
             d <- sweep(X, 2, m); exp(-0.5 * rowSums((d %*% solve(S)) * d)) / sqrt(det(S)) * prior[k] }); \
           dens / rowSums(dens) }; \
         rows <- seq(1, 45, 5); pe <- predict(qe, di)$posterior; pq <- predict(qp, di)$posterior",
    );
    holds(
        &mut e,
        &[
            same("pe", "post(rep(1 / 3, 3))"),
            same("pq", "post(c(20, 15, 10) / 45)"),
            // Stata's predict double, pr: rows 1, 6, ..., 41
            within("pe[rows, ]", "s_pq", "1e-9"),
            within("pq[rows, ]", "s_pp", "1e-9"),
            "all(abs(pe[rows, ] - s_pq) < 1e-12)".into(),
            // and its classification of every row
            "identical(as.integer(predict(qe, di)$class), as.integer(s_g_cq))".into(),
        ],
    );
}
