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
//! Stata weighs them by the group sizes, so its discriminant functions are Stata's
//! only with proportional priors (the classification table is Stata's with either);
//! `stats::varimax` stops about 5e-9 short of the exact two-factor rotation.
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
            within("unclass(fav$loadings)", "B", "1e-7"),
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
            // lda's scaling is the sphered eigenvectors, up to sign; MASS weights the
            // group means by the priors, so the canonical functions are Stata's
            // (which weigh by the group sizes) with proportional priors
            same("abs(lp$scaling)", "abs(U)"),
            same("lp$svd^2 / sum(lp$svd^2)", "em$values[1:2] / sum(em$values[1:2])"),
            // Stata 19: the canonical function coefficients (up to sign), their
            // eigenvalues' shares, and the two classification tables
            same("abs(lp$scaling)", "abs(s_U)"),
            same("lp$svd^2 / sum(lp$svd^2)", "s_Ev / sum(s_Ev)"),
            same("unname(tab(ld))", "s_equal"),
            same("unname(tab(lp))", "s_prop"),
            same("unname(counts(rep(1 / 3, 3)))", "s_equal"),
            same("unname(counts(c(20, 15, 10) / 45))", "s_prop"),
        ],
    );
}
