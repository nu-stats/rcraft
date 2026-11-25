//! Statistics menu resampling dialogs (P6.8) end to end against a real R
//! (skipped with a note when none is found, or when boot is not installed).
//! Each dialog's code is built from its spec and run on made-up data, and its
//! results are checked against the formula written out here in plain R:
//!
//! - the bootstrap: the row draws of `set.seed(s); matrix(sample.int(n, n * R,
//!   replace = TRUE), R, n)` are `boot`'s own (`boot.array`), each replicate is
//!   recomputed by hand from them, the percentile limits are the order
//!   statistics `quantile(type = 6)` gives (exact at R = 999), the normal
//!   limits are the bias-corrected estimate plus or minus z standard errors,
//!   and the BCa limits come from the jackknife acceleration and the
//!   bias-correction constant, interpolated on the normal scale;
//! - the permutation test: every arrangement listed by an independent
//!   enumeration (bit masks, or all index tuples) for the exact p-value, the
//!   shuffles redone with `sample` for the Monte Carlo one, and the two-sided
//!   p-value as (count + 1)/(B + 1);
//! - and against Stata 19's `permute` on the same made-up data, recorded below
//!   (`permute ... : ttest, by()` and `permute ... : correlate`); only numbers
//!   are kept.
//!
//! Stata's two-sided p is 2 * min(lower, upper); the dialog's counts the
//! shuffles at least as large in size as the observed statistic. The two agree
//! on these data to within the Monte Carlo error, which the test allows for.
//!
//! P6.8b adds: whole clusters and strata drawn (the draws are `boot.array`'s,
//! each replicate redone by hand, the standard error of a mean of equal
//! clusters against its exact bootstrap value and Stata's `bootstrap,
//! cluster() strata()`); basic and studentised intervals by their formulas;
//! Stata's normal, percentile and bias-corrected intervals against `bstat` and
//! `estat bootstrap` run on the dialog's own replicates (to 1e-9); a statistic
//! the user writes; permutations within strata listed whole (252^2 crossed
//! sums) and shuffled; a regression coefficient's predictor permuted (all 5040
//! orders of seven rows, and Monte Carlo against `permute x _b[x]: regress`);
//! one-sided p-values and Stata's two-sided rule against `permute`'s lower,
//! upper and two-sided p. Stata keeps `permute`'s statistic in single
//! precision, so it is compared to 1e-6.
//!
//! The test records the R and package versions it ran with (AGENTS.md
//! section 5), and checks R is the one `ENVIRONMENT` pins.

use std::time::{Duration, Instant};

use rcraft_catalog::resampling::{BOOTSTRAP, PERMUTATION_TEST};
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
    let limit = Instant::now() + Duration::from_secs(300);
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

/// The R and boot versions, recorded; false when boot is missing.
fn versions(e: &mut Engine) -> bool {
    let (outcome, text) = run(
        e,
        Source::Typed,
        "cat(paste(R.version$major, R.version$minor, sep = '.'), \
         if (requireNamespace('boot', quietly = TRUE)) as.character(packageVersion('boot')) \
         else 'none')",
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let mut words = text.split_whitespace();
    let (r, boot) = (words.next().unwrap_or(""), words.next().unwrap_or("none"));
    if boot == "none" {
        eprintln!("skipped: boot is not installed");
        return false;
    }
    let pinned = include_str!("../../../ENVIRONMENT")
        .lines()
        .find_map(|l| l.strip_prefix("R_VERSION="))
        .unwrap_or("")
        .trim();
    eprintln!("ran with R {r} and boot {boot} (ENVIRONMENT pins R {pinned})");
    if r != pinned {
        eprintln!("note: this R is not the pinned one, so the run is not a Verified one");
    }
    true
}

/// The made-up data sets: `bd`, thirty rows of an outcome `v` and two
/// predictors; `pt`, twenty rows in two groups of ten with `y` and `x` (the
/// data Stata's `permute` ran on); `e8` and `e6`, the small ones listed whole.
const MAKE: &str = r#"
bd <- local({
  r <- 1:30
  v <- round(sin(r * 1.9) * 2 + cos(r * 4.1) + 5, 3)
  data.frame(v = v,
             x1 = round(0.6 * v + sin(r * 2.3), 3),
             x2 = round(cos(r * 3.7) + 0.2 * v, 3))
})
pt <- local({
  r <- 1:20
  y <- round(c(0, 0.6)[rep(1:2, each = 10)] + sin(r * 2.7) + 0.4 * cos(r * 5.3), 3)
  data.frame(g = rep(1:2, each = 10), y = y, x = round(0.5 * y + 0.8 * cos(r * 3.1), 3))
})
e8 <- data.frame(g = rep(c("a", "b"), each = 4),
                 y = c(3.1, 4.7, 2.2, 5.9, 6.4, 7.3, 4.1, 8.8))
e6 <- data.frame(a = c(1.5, 2.1, 3.7, 3.9, 5.2, 6.8), b = c(2.2, 1.1, 4.9, 3.0, 6.1, 5.5))
"#;

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

fn bd_context() -> Context {
    context(
        "bd",
        &[
            ("v", VarKind::Number),
            ("x1", VarKind::Number),
            ("x2", VarKind::Number),
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

const ALL3: &[&str] = &["normal", "percentile", "bca"];

/// The bootstrap dialog's code, run with its widgets set.
fn bootstrap(
    e: &mut Engine,
    statistic: &str,
    v: &str,
    x: &[&str],
    extra: &[(&str, &str)],
    flags: &[&str],
) -> String {
    let mut set = vec![("statistic", statistic), ("v", v)];
    set.extend_from_slice(extra);
    let (outcome, text) = dialog(e, &bd_context(), &BOOTSTRAP, &set, &[("x", x)], flags);
    assert_eq!(outcome, Outcome::Ok, "{text}");
    text
}

/// R that interpolates on the normal scale, as the BCa limits do, and builds
/// the row draws of seed `s`.
const ORACLE: &str = r#"
draws <- function(s, n, R) { set.seed(s); matrix(sample.int(n, n * R, replace = TRUE), R, n) }
interp <- function(t, alpha) {
  t <- sort(t); R <- length(t); rk <- (R + 1) * alpha; k <- trunc(rk)
  vapply(seq_along(alpha), function(j) {
    if (k[j] < 1) return(t[1])
    if (k[j] >= R) return(t[R])
    if (k[j] == rk[j]) return(t[k[j]])
    q <- qnorm(alpha[j]); lo <- qnorm(k[j] / (R + 1)); hi <- qnorm((k[j] + 1) / (R + 1))
    t[k[j]] + (q - lo) / (hi - lo) * (t[k[j] + 1] - t[k[j]])
  }, 0)
}
bca <- function(t, th, jk, level) {
  n <- length(jk); L <- (n - 1) * (mean(jk) - jk)
  acc <- sum(L^3) / (6 * sum(L^2)^1.5)
  z0 <- qnorm(mean(t < th)); zz <- qnorm(c((1 - level) / 2, (1 + level) / 2))
  interp(t, pnorm(z0 + (z0 + zz) / (1 - acc * (z0 + zz))))
}
"#;

#[test]
fn bootstrap_matches_the_formulas() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    define(&mut e, MAKE);
    define(&mut e, ORACLE);

    // the mean: every replicate, every limit, by hand
    let text = bootstrap(&mut e, "mean", "v", &[], &[("save", "bm")], ALL3);
    assert!(
        text.contains("Bootstrap: 999 replications, seed 1") && text.contains("Percentile lower"),
        "{text}"
    );
    define(
        &mut e,
        "idx <- draws(1, 30, 999); th <- mean(bd$v); \
         t_hand <- apply(idx, 1, function(i) mean(bd$v[i])); \
         jk <- sapply(1:30, function(i) mean(bd$v[-i])); \
         iv <- bm$intervals; bias <- mean(t_hand) - th; se <- sd(t_hand)",
    );
    holds(
        &mut e,
        &[
            // the draws are boot's own, the replicates are the hand ones
            "identical(boot::boot.array(bm, indices = TRUE), idx)".into(),
            same("bm$t[, 1]", "t_hand"),
            same("bm$t0", "th"),
            same("iv$Estimate", "th"),
            same("iv$Bias", "bias"),
            same("iv$Std.Err", "se"),
            // percentile: the 25th smallest and 25th largest of 999
            same(
                "c(iv$`Percentile lower`, iv$`Percentile upper`)",
                "quantile(t_hand, c(0.025, 0.975), type = 6)",
            ),
            same(
                "c(iv$`Percentile lower`, iv$`Percentile upper`)",
                "sort(t_hand)[c(25, 975)]",
            ),
            // normal: bias-corrected estimate plus or minus z standard errors
            same(
                "c(iv$`Normal lower`, iv$`Normal upper`)",
                "th - bias + qnorm(c(0.025, 0.975)) * se",
            ),
            // BCa: jackknife acceleration and bias-correction constant
            same(
                "c(iv$`BCa lower`, iv$`BCa upper`)",
                "bca(t_hand, th, jk, 0.95)",
            ),
            "iv$`BCa lower` < th && th < iv$`BCa upper`".into(),
        ],
    );

    // the same seed gives the same replicates; another seed does not
    define(&mut e, "t_first <- bm$t");
    bootstrap(&mut e, "mean", "v", &[], &[("save", "bm")], ALL3);
    holds(&mut e, &["identical(bm$t, t_first)".into()]);
    bootstrap(
        &mut e,
        "mean",
        "v",
        &[],
        &[("save", "bm2"), ("seed", "2")],
        &["percentile"],
    );
    holds(
        &mut e,
        &[
            "!identical(bm2$t, t_first)".into(),
            "identical(bm2$t, matrix(apply(draws(2, 30, 999), 1, function(i) mean(bd$v[i])), ncol = 1))".into(),
        ],
    );

    // a 90% level and a different number of replications: the limits move to
    // the 5th and 95th percentile of 499 (the 25th smallest and largest)
    bootstrap(
        &mut e,
        "mean",
        "v",
        &[],
        &[("save", "b90"), ("reps", "499"), ("level", "90")],
        &["percentile", "normal"],
    );
    define(
        &mut e,
        "t499 <- apply(draws(1, 30, 499), 1, function(i) mean(bd$v[i])); iv <- b90$intervals",
    );
    holds(
        &mut e,
        &[
            same(
                "c(iv$`Percentile lower`, iv$`Percentile upper`)",
                "quantile(t499, c(0.05, 0.95), type = 6)",
            ),
            same(
                "c(iv$`Normal lower`, iv$`Normal upper`)",
                "mean(bd$v) - (mean(t499) - mean(bd$v)) + qnorm(c(0.05, 0.95)) * sd(t499)",
            ),
            "!('BCa lower' %in% names(iv))".into(),
        ],
    );

    // the median
    bootstrap(
        &mut e,
        "median",
        "v",
        &[],
        &[("save", "bmed")],
        &["percentile", "bca"],
    );
    define(
        &mut e,
        "tm <- apply(idx, 1, function(i) median(bd$v[i])); \
         jkm <- sapply(1:30, function(i) median(bd$v[-i])); iv <- bmed$intervals",
    );
    holds(
        &mut e,
        &[
            same("bmed$t[, 1]", "tm"),
            same("iv$Estimate", "median(bd$v)"),
            same(
                "c(iv$`Percentile lower`, iv$`Percentile upper`)",
                "quantile(tm, c(0.025, 0.975), type = 6)",
            ),
            // the median's limits come from boot's own empirical influence, so
            // the BCa limits are only checked to bracket the estimate and lie
            // inside the replicates' range
            "iv$`BCa lower` <= iv$Estimate && iv$Estimate <= iv$`BCa upper`".into(),
            "iv$`BCa lower` >= min(tm) && iv$`BCa upper` <= max(tm)".into(),
        ],
    );

    // a correlation
    bootstrap(
        &mut e,
        "correlation",
        "v",
        &["x1"],
        &[("save", "bcor")],
        ALL3,
    );
    define(
        &mut e,
        "tc <- apply(idx, 1, function(i) cor(bd$v[i], bd$x1[i])); iv <- bcor$intervals",
    );
    holds(
        &mut e,
        &[
            same("bcor$t[, 1]", "tc"),
            same("iv$Estimate", "cor(bd$v, bd$x1)"),
            same(
                "c(iv$`Percentile lower`, iv$`Percentile upper`)",
                "quantile(tc, c(0.025, 0.975), type = 6)",
            ),
            same(
                "c(iv$`Normal lower`, iv$`Normal upper`)",
                "cor(bd$v, bd$x1) - (mean(tc) - cor(bd$v, bd$x1)) + qnorm(c(0.025, 0.975)) * sd(tc)",
            ),
            "all(is.finite(c(iv$`BCa lower`, iv$`BCa upper`)))".into(),
            "row.names(iv) == 'correlation of v and x1'".into(),
        ],
    );

    // the coefficients of a linear model
    let text = bootstrap(
        &mut e,
        "coefficient",
        "v",
        &["x1", "x2"],
        &[("save", "bcoef")],
        &["percentile", "normal"],
    );
    assert!(
        text.contains("(Intercept)") && text.contains("x2"),
        "{text}"
    );
    define(
        &mut e,
        "tb <- t(apply(idx, 1, function(i) coef(lm(v ~ x1 + x2, data = bd[i, ])))); \
         iv <- bcoef$intervals; fit0 <- coef(lm(v ~ x1 + x2, data = bd))",
    );
    holds(
        &mut e,
        &[
            same("bcoef$t", "tb"),
            same("iv$Estimate", "fit0"),
            same(
                "iv$`Percentile lower`",
                "apply(tb, 2, quantile, probs = 0.025, type = 6)",
            ),
            same(
                "iv$`Percentile upper`",
                "apply(tb, 2, quantile, probs = 0.975, type = 6)",
            ),
            "identical(row.names(iv), c('(Intercept)', 'x1', 'x2'))".into(),
        ],
    );
}

/// What Stata 19's `permute` reported on `pt` (two-sided p, and its 95%
/// Monte Carlo interval), for the difference in means of the two groups and
/// for the correlation of x and y.
const STATA: &str = "\
stata_diff <- 0.5951; stata_r <- 0.3300888; \
stata_p_diff <- 0.1288; stata_ci_diff <- c(0.1258, 0.1317); \
stata_p_r <- 0.1523; stata_ci_r <- c(0.1492, 0.1555)";

fn pt_context() -> Context {
    context(
        "pt",
        &[
            ("g", VarKind::Number),
            ("y", VarKind::Number),
            ("x", VarKind::Number),
        ],
    )
}

fn permutation(e: &mut Engine, ctx: &Context, set: &[(&str, &str)]) -> String {
    let (outcome, text) = dialog(e, ctx, &PERMUTATION_TEST, set, &[], &[]);
    assert_eq!(outcome, Outcome::Ok, "{text}");
    text
}

#[test]
fn permutation_test_matches_exact_listing_and_stata() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    define(&mut e, MAKE);
    define(&mut e, STATA);

    // ---- exact, on eight rows: 70 arrangements, listed by bit masks ----
    let e8_ctx = context("e8", &[("g", VarKind::Text), ("y", VarKind::Number)]);
    let text = permutation(&mut e, &e8_ctx, &[("v", "y"), ("w", "g"), ("save", "pe")]);
    assert!(
        text.contains("All 70 arrangements listed (exact)") && text.contains("Two-sided p-value"),
        "{text}"
    );
    define(
        &mut e,
        "masks <- Filter(function(m) sum(bitwAnd(m, 2^(0:7)) > 0) == 4, 0:255); \
         dm <- sapply(masks, function(m) { s <- bitwAnd(m, 2^(0:7)) > 0; \
           mean(e8$y[s]) - mean(e8$y[!s]) }); \
         obs8 <- mean(e8$y[5:8]) - mean(e8$y[1:4]); \
         count8 <- sum(abs(dm) >= abs(obs8) - 1e-9); \
         dmed <- sapply(masks, function(m) { s <- bitwAnd(m, 2^(0:7)) > 0; \
           median(e8$y[s]) - median(e8$y[!s]) }); \
         obsmed <- median(e8$y[5:8]) - median(e8$y[1:4]); \
         countmed <- sum(abs(dmed) >= abs(obsmed) - 1e-9)",
    );
    holds(
        &mut e,
        &[
            "length(masks) == 70".into(),
            "pe$exact && pe$arrangements == 70 && pe$permutations == 70".into(),
            same("pe$statistic", "obs8"),
            same("pe$p.value", "count8 / 70"),
            same("sort(pe$replicates)", "sort(dm)"),
            // by hand, the observed arrangement is one of the 70
            "count8 >= 2".into(),
        ],
    );
    permutation(
        &mut e,
        &e8_ctx,
        &[
            ("statistic", "medians"),
            ("v", "y"),
            ("w", "g"),
            ("save", "pm"),
        ],
    );
    holds(
        &mut e,
        &[
            "pm$exact".into(),
            same("pm$statistic", "obsmed"),
            same("pm$p.value", "countmed / 70"),
            same("sort(pm$replicates)", "sort(dmed)"),
        ],
    );
    // a small B turns the same data into Monte Carlo: 10 random shuffles
    let text = permutation(
        &mut e,
        &e8_ctx,
        &[("v", "y"), ("w", "g"), ("reps", "10"), ("save", "pmc")],
    );
    assert!(text.contains("10 random permutations of 70"), "{text}");
    define(
        &mut e,
        "set.seed(1); sec <- rep(c(FALSE, TRUE), each = 4); \
         tmc <- replicate(10, { s <- sample(sec); mean(e8$y[s]) - mean(e8$y[!s]) }); \
         pmc_hand <- (sum(abs(tmc) >= abs(obs8) - 1e-9) + 1) / 11",
    );
    holds(
        &mut e,
        &[
            "!pmc$exact".into(),
            same("pmc$replicates", "tmc"),
            same("pmc$p.value", "pmc_hand"),
        ],
    );

    // ---- exact correlation on six rows: 720 arrangements, listed as tuples ----
    let e6_ctx = context("e6", &[("a", VarKind::Number), ("b", VarKind::Number)]);
    permutation(
        &mut e,
        &e6_ctx,
        &[
            ("statistic", "correlation"),
            ("v", "a"),
            ("w", "b"),
            ("save", "pc6"),
        ],
    );
    define(
        &mut e,
        "tup <- as.matrix(expand.grid(rep(list(1:6), 6))); \
         tup <- tup[apply(tup, 1, function(r) length(unique(r)) == 6), ]; \
         rr <- apply(tup, 1, function(p) cor(e6$b, e6$a[p])); \
         obs6 <- cor(e6$a, e6$b)",
    );
    holds(
        &mut e,
        &[
            "nrow(tup) == 720".into(),
            "pc6$exact && pc6$arrangements == 720 && pc6$permutations == 720".into(),
            same("pc6$statistic", "obs6"),
            same("pc6$p.value", "sum(abs(rr) >= abs(obs6) - 1e-9) / 720"),
            same("sort(pc6$replicates)", "sort(rr)"),
        ],
    );

    // ---- Monte Carlo on Stata's data: 20 rows, 184756 arrangements ----
    let text = permutation(
        &mut e,
        &pt_context(),
        &[
            ("v", "y"),
            ("w", "g"),
            ("reps", "50000"),
            ("seed", "7"),
            ("save", "ppt"),
        ],
    );
    assert!(
        text.contains("50000 random permutations of 1.848e+05 arrangements, seed 7"),
        "{text}"
    );
    define(
        &mut e,
        "second <- pt$g == 2; \
         st <- function(s) mean(pt$y[s]) - mean(pt$y[!s]); \
         obs <- st(second); \
         all_t <- utils::combn(20, 10, function(ix) { s <- logical(20); s[ix] <- TRUE; st(s) }); \
         p_exact <- sum(abs(all_t) >= abs(obs) - 1e-9) / length(all_t); \
         set.seed(7); tmc <- replicate(50000, st(sample(second))); \
         p_hand <- (sum(abs(tmc) >= abs(obs) - 1e-9) + 1) / 50001; \
         se <- sqrt(ppt$p.value * (1 - ppt$p.value) / 50000)",
    );
    holds(
        &mut e,
        &[
            "length(all_t) == 184756".into(),
            // the difference is the second group's mean minus the first's
            within("ppt$statistic", "stata_diff", "1e-4"),
            same("ppt$statistic", "obs"),
            // the same shuffles, the same counts
            same("ppt$replicates", "tmc"),
            same("ppt$p.value", "p_hand"),
            // against the exact p-value, within four Monte Carlo errors
            "abs(ppt$p.value - p_exact) < 4 * se".into(),
            // against Stata's p-value and its Monte Carlo interval: the
            // dialog's p lies within it, or within four combined errors
            "(ppt$p.value >= stata_ci_diff[1] && ppt$p.value <= stata_ci_diff[2]) || \
             abs(ppt$p.value - stata_p_diff) < 4 * sqrt(se^2 + 0.0015^2)"
                .into(),
            "abs(p_exact - stata_p_diff) < 4 * 0.0015".into(),
        ],
    );

    // ---- Monte Carlo correlation on the same data ----
    permutation(
        &mut e,
        &pt_context(),
        &[
            ("statistic", "correlation"),
            ("v", "y"),
            ("w", "x"),
            ("reps", "50000"),
            ("seed", "7"),
            ("save", "pcor"),
        ],
    );
    define(
        &mut e,
        "set.seed(7); tr <- replicate(50000, cor(pt$x, pt$y[sample.int(20)])); \
         obsr <- cor(pt$x, pt$y); \
         pr_hand <- (sum(abs(tr) >= abs(obsr) - 1e-9) + 1) / 50001; \
         ser <- sqrt(pcor$p.value * (1 - pcor$p.value) / 50000)",
    );
    holds(
        &mut e,
        &[
            within("pcor$statistic", "stata_r", "1e-6"),
            same("pcor$replicates", "tr"),
            same("pcor$p.value", "pr_hand"),
            "(pcor$p.value >= stata_ci_r[1] && pcor$p.value <= stata_ci_r[2]) || \
             abs(pcor$p.value - stata_p_r) < 4 * sqrt(ser^2 + 0.0016^2)"
                .into(),
        ],
    );

    // ---- refusals say why, and R does not crash ----
    let ctx = pt_context();
    let mut same_var = PERMUTATION_TEST.defaults(&ctx);
    same_var.set_text("v", "y");
    same_var.set_text("w", "y");
    assert!(PERMUTATION_TEST
        .generate(&ctx, &same_var, Style::Base)
        .is_err());
    // twenty different values are not two groups: R stops with a plain message
    let (outcome, text) = dialog(
        &mut e,
        &ctx,
        &PERMUTATION_TEST,
        &[("v", "x"), ("w", "y"), ("save", "pbad")],
        &[],
        &[],
    );
    assert_eq!(outcome, Outcome::Error, "{text}");
    assert!(text.contains("exactly two groups"), "{text}");
    // a bootstrap too small for its replications is refused before R runs
    let mut small = BOOTSTRAP.defaults(&bd_context());
    small.set_text("v", "v");
    small.set_text("reps", "20");
    assert!(BOOTSTRAP
        .generate(&bd_context(), &small, Style::Base)
        .is_err());
}

// ---- P6.8b: clusters and strata, the new intervals, one's own statistic,
// permutations within strata and of a regression coefficient, one-sided p ----

/// `bc`: forty rows in eight clusters of five (`cl`), four clusters in each of
/// two strata (`st`), with `st2` crossing the clusters; `pts`: `pt` with strata
/// `s` (five rows of each group in each) and another predictor `z`; `c7`: seven
/// rows for an exact coefficient listing. `mc_sd` is the Monte Carlo error of a
/// standard error worked out from the replicates (their kurtosis).
const MAKE_B: &str = r#"
bc <- local({
  r <- 1:40
  cl <- rep(1:8, each = 5); st <- rep(1:2, each = 20)
  v <- round(sin(r * 1.9) * 2 + cos(r * 4.1) + 5 + 0.8 * sin(cl * 2.2), 3)
  data.frame(cl = cl, st = st, st2 = rep(1:2, 20), v = v, x1 = round(0.6 * v + sin(r * 2.3), 3))
})
pts <- local({
  r <- 1:20
  y <- round(c(0, 0.6)[rep(1:2, each = 10)] + sin(r * 2.7) + 0.4 * cos(r * 5.3), 3)
  data.frame(g = rep(1:2, each = 10), y = y, x = round(0.5 * y + 0.8 * cos(r * 3.1), 3),
             s = rep(rep(1:2, each = 5), 2), z = round(cos(r * 1.3), 3))
})
c7 <- data.frame(y = c(2.1, 3.4, 2.9, 4.8, 5.1, 4.2, 6.3), x = c(1.0, 1.7, 2.2, 2.4, 3.1, 3.5, 3.9),
                 z = c(0.5, -0.2, 0.9, 0.1, -0.7, 0.4, 0.3))
mc_sd <- function(t) {
  k <- mean((t - mean(t))^4) / mean((t - mean(t))^2)^2
  sd(t) * sqrt((k - 1) / (4 * length(t)))
}
"#;

/// What Stata 19 reported on the same data. `bstat` on the dialog's own 999
/// replicates of the mean of `bc$v` (seed 1) and 1000 (seed 2): the standard
/// error and `estat bootstrap`'s normal, percentile and bias-corrected limits;
/// `bootstrap ..., reps(20000)` standard errors with `cluster(cl)`, `cluster(cl)
/// strata(st)` and `strata(st)` for the mean, for `r(sd)/r(mean)` and for
/// `regress v x1` with `cluster(cl)` (constant, then x1); `permute ...,
/// reps(50000) seed(1)` p-values (lower, upper, two-sided) on `pts` and `c7`.
const STATA_B: &str = "\
s_se <- 0.265642397125646; s_norm <- c(4.46025046886685, 5.50154953113315); \
s_perc <- c(4.470075, 5.497575); s_bc <- c(4.480375, 5.500875); \
s_perc2 <- c(4.4412, 5.5004875); s_bc2 <- c(4.41995, 5.4894); \
s_cl <- 0.246349076524278; s_clst <- 0.236487176238673; s_st <- 0.269007372156452; \
s_cv <- 0.038085977725217; s_cv_b <- 0.344588848186307; \
s_reg <- c(0.38413537665302, 0.107883507527712); \
sp_strata <- c(0.93036, 0.06968, 0.13936); sp_diff <- 0.595099985; \
sp_coef <- c(0.91282, 0.08718, 0.17436); sp_coef_b <- 0.382758051; \
sp_coef_strata <- c(0.91046, 0.08954, 0.17908); \
sp_cor_strata <- c(0.9235, 0.0765, 0.153); sp_rho <- 0.330088764; \
sp_c7 <- c(0.96714, 0.03308, 0.06616); sp_c7_b <- 1.14418196";

fn bc_context() -> Context {
    context(
        "bc",
        &[
            ("cl", VarKind::Number),
            ("st", VarKind::Number),
            ("st2", VarKind::Number),
            ("v", VarKind::Number),
            ("x1", VarKind::Number),
        ],
    )
}

fn bc_boot(e: &mut Engine, set: &[(&str, &str)], x: &[&str], flags: &[&str]) -> (Outcome, String) {
    dialog(e, &bc_context(), &BOOTSTRAP, set, &[("x", x)], flags)
}

fn bc_ok(e: &mut Engine, set: &[(&str, &str)], x: &[&str], flags: &[&str]) -> String {
    let (outcome, text) = bc_boot(e, set, x, flags);
    assert_eq!(outcome, Outcome::Ok, "{text}");
    text
}

#[test]
fn bootstrap_clusters_strata_intervals_and_own_statistic() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    define(&mut e, MAKE_B);
    define(&mut e, STATA_B);
    define(&mut e, ORACLE);

    // ---- Stata's intervals, basic and studentised, on the same 999 replicates ----
    let text = bc_ok(
        &mut e,
        &[("v", "v"), ("save", "b1")],
        &[],
        &["percentile", "basic", "student", "stata"],
    );
    assert!(
        text.contains("Stata BC lower") && text.contains("Studentised upper"),
        "{text}"
    );
    define(
        &mut e,
        "idx <- draws(1, 40, 999); th <- mean(bc$v); \
         t1 <- apply(idx, 1, function(i) mean(bc$v[i])); \
         v1 <- apply(idx, 1, function(i) var(bc$v[i]) / 40); v0 <- var(bc$v) / 40; \
         zt <- (t1 - th) / sqrt(v1); iv <- b1$intervals",
    );
    holds(
        &mut e,
        &[
            same("b1$t[, 1]", "t1"),
            same("b1$t[, 2]", "v1"),
            same("iv$Estimate", "th"),
            // Stata's standard error and its three intervals, to 1e-9
            same("iv$Std.Err", "s_se"),
            same(
                "c(iv$`Stata normal lower`, iv$`Stata normal upper`)",
                "s_norm",
            ),
            same(
                "c(iv$`Stata percentile lower`, iv$`Stata percentile upper`)",
                "s_perc",
            ),
            same("c(iv$`Stata BC lower`, iv$`Stata BC upper`)", "s_bc"),
            // at 999 boot's percentile limits are Stata's
            same("c(iv$`Percentile lower`, iv$`Percentile upper`)", "s_perc"),
            // basic: twice the estimate less the upper and lower limits
            same(
                "c(iv$`Basic lower`, iv$`Basic upper`)",
                "2 * th - interp(t1, c(0.975, 0.025))",
            ),
            // studentised: each replicate over its own standard error
            same(
                "c(iv$`Studentised lower`, iv$`Studentised upper`)",
                "th - sqrt(v0) * interp(zt, c(0.975, 0.025))",
            ),
            "nrow(iv) == 1".into(),
        ],
    );
    // at 1000 replicates Stata's percentile limits average two replicates
    bc_ok(
        &mut e,
        &[("v", "v"), ("reps", "1000"), ("seed", "2"), ("save", "b2")],
        &[],
        &["stata", "percentile"],
    );
    define(&mut e, "iv <- b2$intervals");
    holds(
        &mut e,
        &[
            same(
                "c(iv$`Stata percentile lower`, iv$`Stata percentile upper`)",
                "s_perc2",
            ),
            same("c(iv$`Stata BC lower`, iv$`Stata BC upper`)", "s_bc2"),
            "abs(iv$`Percentile lower` - s_perc2[1]) > 1e-6".into(),
        ],
    );

    // ---- whole clusters: 20000 draws of eight clusters ----
    let text = bc_ok(
        &mut e,
        &[
            ("v", "v"),
            ("cluster", "cl"),
            ("reps", "20000"),
            ("save", "bcl"),
        ],
        &[],
        &["normal"],
    );
    assert!(text.contains("Clusters of cl drawn, 8 in all"), "{text}");
    define(
        &mut e,
        "jdx <- draws(1, 8, 20000); \
         tcl <- apply(jdx, 1, function(j) mean(unlist(lapply(j, function(g) bc$v[bc$cl == g])))); \
         cm <- tapply(bc$v, bc$cl, mean); \
         ideal_cl <- sqrt(mean((cm - mean(cm))^2) / 8); se_cl <- bcl$intervals$Std.Err",
    );
    holds(
        &mut e,
        &[
            "identical(boot::boot.array(bcl, indices = TRUE), jdx)".into(),
            same("bcl$t[, 1]", "tcl"),
            // the exact bootstrap error of a mean of equal clusters, and
            // Stata's, within four Monte Carlo errors
            "abs(se_cl - ideal_cl) < 4 * mc_sd(tcl)".into(),
            "abs(s_cl - ideal_cl) < 4 * mc_sd(tcl)".into(),
            "abs(se_cl - s_cl) < 4 * sqrt(2) * mc_sd(tcl)".into(),
        ],
    );

    // ---- clusters within strata ----
    let text = bc_ok(
        &mut e,
        &[
            ("v", "v"),
            ("cluster", "cl"),
            ("strata", "st"),
            ("reps", "20000"),
            ("save", "bcs"),
        ],
        &[],
        &["percentile"],
    );
    assert!(text.contains("Drawn within the 2 strata of st"), "{text}");
    define(
        &mut e,
        "kdx <- boot::boot.array(bcs, indices = TRUE); cst <- rep(1:2, each = 4); \
         tcs <- apply(kdx, 1, function(j) mean(unlist(lapply(j, function(g) bc$v[bc$cl == g])))); \
         ideal_cs <- sqrt(sum(sapply(1:2, function(h) { c <- cm[cst == h]; \
           (4 / 8)^2 * mean((c - mean(c))^2) / 4 }))); se_cs <- bcs$intervals$Std.Err",
    );
    holds(
        &mut e,
        &[
            // every drawn cluster comes from the stratum of its place
            "all(cst[kdx] == cst[col(kdx)])".into(),
            same("bcs$t[, 1]", "tcs"),
            "abs(se_cs - ideal_cs) < 4 * mc_sd(tcs)".into(),
            "abs(s_clst - ideal_cs) < 4 * mc_sd(tcs)".into(),
        ],
    );

    // ---- rows within strata ----
    bc_ok(
        &mut e,
        &[
            ("v", "v"),
            ("strata", "st"),
            ("reps", "20000"),
            ("save", "bst"),
        ],
        &[],
        &["normal"],
    );
    define(
        &mut e,
        "ldx <- boot::boot.array(bst, indices = TRUE); \
         tst <- apply(ldx, 1, function(i) mean(bc$v[i])); \
         ideal_st <- sqrt(sum(sapply(1:2, function(h) { y <- bc$v[bc$st == h]; \
           (20 / 40)^2 * mean((y - mean(y))^2) / 20 }))); se_st <- bst$intervals$Std.Err",
    );
    holds(
        &mut e,
        &[
            "all(bc$st[ldx] == bc$st[col(ldx)])".into(),
            same("bst$t[, 1]", "tst"),
            "abs(se_st - ideal_st) < 4 * mc_sd(tst)".into(),
            "abs(s_st - ideal_st) < 4 * mc_sd(tst)".into(),
        ],
    );

    // ---- a statistic of one's own: the coefficient of variation ----
    bc_ok(
        &mut e,
        &[
            ("statistic", "custom"),
            ("v", "v"),
            ("expr", "sd(v) / mean(v)"),
            ("reps", "20000"),
            ("save", "bcv"),
        ],
        &[],
        &["normal", "percentile", "bca"],
    );
    define(
        &mut e,
        "tcv <- apply(draws(1, 40, 20000), 1, function(i) sd(bc$v[i]) / mean(bc$v[i])); \
         iv <- bcv$intervals",
    );
    holds(
        &mut e,
        &[
            same("bcv$t[, 1]", "tcv"),
            same("iv$Estimate", "s_cv_b"),
            "row.names(iv) == 'statistic'".into(),
            "abs(iv$Std.Err - s_cv) < 4 * sqrt(2) * mc_sd(tcv)".into(),
            "all(is.finite(c(iv$`BCa lower`, iv$`BCa upper`)))".into(),
        ],
    );
    // several numbers keep their names
    bc_ok(
        &mut e,
        &[
            ("statistic", "custom"),
            ("v", "v"),
            ("expr", "quantile(v, c(0.25, 0.75))"),
            ("save", "bq"),
        ],
        &[],
        &["percentile"],
    );
    holds(
        &mut e,
        &[
            "identical(row.names(bq$intervals), c('25%', '75%'))".into(),
            same("bq$intervals$Estimate", "quantile(bc$v, c(0.25, 0.75))"),
        ],
    );

    // ---- coefficients with clusters, against Stata's errors ----
    bc_ok(
        &mut e,
        &[
            ("statistic", "coefficient"),
            ("v", "v"),
            ("cluster", "cl"),
            ("reps", "20000"),
            ("save", "breg"),
        ],
        &["x1"],
        &["normal"],
    );
    define(
        &mut e,
        "iv <- breg$intervals; mcs <- apply(breg$t, 2, mc_sd)",
    );
    holds(
        &mut e,
        &[
            same("iv$Estimate", "coef(lm(v ~ x1, data = bc))"),
            "all(abs(iv$Std.Err - s_reg) < 4 * sqrt(2) * mcs)".into(),
        ],
    );

    // ---- studentised coefficients: each replicate's least-squares error ----
    bc_ok(
        &mut e,
        &[("statistic", "coefficient"), ("v", "v"), ("save", "bstu")],
        &["x1"],
        &["student"],
    );
    define(
        &mut e,
        "fits <- lapply(seq_len(999), function(r) lm(v ~ x1, data = bc[idx[r, ], ])); \
         b_r <- sapply(fits, function(f) coef(f)[2]); \
         s_r <- sapply(fits, function(f) sqrt(vcov(f)[2, 2])); \
         f0 <- lm(v ~ x1, data = bc); b0 <- coef(f0)[2]; s0 <- sqrt(vcov(f0)[2, 2]); \
         iv <- bstu$intervals",
    );
    holds(
        &mut e,
        &[
            "identical(row.names(iv), c('(Intercept)', 'x1'))".into(),
            same("bstu$t[, 2]", "b_r"),
            same("sqrt(bstu$t[, 4])", "s_r"),
            same(
                "c(iv$`Studentised lower`[2], iv$`Studentised upper`[2])",
                "b0 - s0 * interp((b_r - b0) / s_r, c(0.975, 0.025))",
            ),
        ],
    );

    // ---- refusals: R says why and stays up ----
    let (outcome, text) = bc_boot(
        &mut e,
        &[
            ("v", "v"),
            ("cluster", "cl"),
            ("strata", "st2"),
            ("save", "bbad"),
        ],
        &[],
        &["normal"],
    );
    assert_eq!(outcome, Outcome::Error, "{text}");
    assert!(
        text.contains("Each cluster must lie within one stratum"),
        "{text}"
    );
    let (outcome, text) = bc_boot(
        &mut e,
        &[
            ("statistic", "custom"),
            ("v", "v"),
            ("expr", "'a'"),
            ("save", "bbad"),
        ],
        &[],
        &["normal"],
    );
    assert_eq!(outcome, Outcome::Error, "{text}");
    assert!(text.contains("must give one or more numbers"), "{text}");
    let mut stud = BOOTSTRAP.defaults(&bc_context());
    stud.set_text("v", "v");
    stud.set_text("cluster", "cl");
    stud.set_flag("student", true);
    assert!(BOOTSTRAP
        .generate(&bc_context(), &stud, Style::Base)
        .is_err());
    holds(&mut e, &["exists('b1')".into()]);
}

fn pts_context() -> Context {
    context(
        "pts",
        &[
            ("g", VarKind::Number),
            ("y", VarKind::Number),
            ("x", VarKind::Number),
            ("s", VarKind::Number),
            ("z", VarKind::Number),
        ],
    )
}

fn perm(e: &mut Engine, ctx: &Context, set: &[(&str, &str)], z: &[&str]) -> String {
    let (outcome, text) = dialog(e, ctx, &PERMUTATION_TEST, set, &[("z", z)], &[]);
    assert_eq!(outcome, Outcome::Ok, "{text}");
    text
}

/// Four Monte Carlo errors of Stata's `permute` p-value `p` from 50000
/// shuffles, combined with the dialog's own when `both`.
fn mc(p: &str, both: bool) -> String {
    let k = if both { "2 * " } else { "" };
    format!("4 * sqrt({k}{p} * (1 - {p}) / 50000)")
}

#[test]
fn permutation_within_strata_coefficients_and_one_sided() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if !versions(&mut e) {
        return;
    }
    define(&mut e, MAKE_B);
    define(&mut e, STATA_B);
    let ctx = pts_context();

    // ---- the difference in means within strata, every one of 252^2 listed ----
    // the hand listing: the second group's sum in each stratum, crossed
    define(
        &mut e,
        "u1 <- which(pts$s == 1); u2 <- which(pts$s == 2); \
         sec <- outer(combn(pts$y[u1], 5, sum), combn(pts$y[u2], 5, sum), '+'); \
         dd <- as.vector(sec / 10 - (sum(pts$y) - sec) / 10); \
         obs <- mean(pts$y[pts$g == 2]) - mean(pts$y[pts$g == 1]); tol <- 1e-9; \
         ex <- c(less = mean(dd <= obs + tol), greater = mean(dd >= obs - tol), \
                 two = mean(abs(dd) >= abs(obs) - tol))",
    );
    for alt in ["two", "greater", "less", "stata"] {
        let name = format!("ps_{alt}");
        let text = perm(
            &mut e,
            &ctx,
            &[
                ("v", "y"),
                ("w", "g"),
                ("strata", "s"),
                ("alternative", alt),
                ("reps", "100000"),
                ("save", &name),
            ],
            &[],
        );
        assert!(
            text.contains("Shuffled within the 2 strata of s")
                && text.contains("All 63504 arrangements listed (exact)"),
            "{text}"
        );
    }
    holds(
        &mut e,
        &[
            "length(dd) == 63504".into(),
            same("sort(ps_two$replicates)", "sort(dd)"),
            within("ps_two$statistic", "sp_diff", "1e-6"),
            same("ps_two$p.value", "ex[['two']]"),
            same("ps_greater$p.value", "ex[['greater']]"),
            same("ps_less$p.value", "ex[['less']]"),
            same(
                "ps_stata$p.value",
                "min(1, 2 * min(ex[['greater']], ex[['less']]))",
            ),
            // Stata's Monte Carlo p-values lie within four errors of the exact ones
            format!(
                "abs(sp_strata[1] - ex[['less']]) < {}",
                mc("ex[['less']]", false)
            ),
            format!(
                "abs(sp_strata[2] - ex[['greater']]) < {}",
                mc("ex[['greater']]", false)
            ),
            format!(
                "abs(sp_strata[3] - ps_stata$p.value) < 2 * {}",
                mc("ex[['greater']]", false)
            ),
        ],
    );
    // the same within strata by random shuffles
    perm(
        &mut e,
        &ctx,
        &[
            ("v", "y"),
            ("w", "g"),
            ("strata", "s"),
            ("alternative", "greater"),
            ("reps", "20000"),
            ("seed", "3"),
            ("save", "psm"),
        ],
        &[],
    );
    define(
        &mut e,
        "set.seed(3); second <- pts$g == 2; units <- list(u1, u2); \
         tm <- replicate(20000, { s <- second; \
           for (u in units) s[u] <- s[u][sample.int(length(u))]; \
           mean(pts$y[s]) - mean(pts$y[!s]) })",
    );
    holds(
        &mut e,
        &[
            "!psm$exact".into(),
            same("psm$replicates", "tm"),
            same("psm$p.value", "(sum(tm >= obs - tol) + 1) / 20001"),
            "abs(psm$p.value - ex[['greater']]) < \
             4 * sqrt(ex[['greater']] * (1 - ex[['greater']]) / 20000)"
                .into(),
        ],
    );

    // ---- a regression coefficient: x shuffled in y ~ x + z ----
    let text = perm(
        &mut e,
        &ctx,
        &[
            ("statistic", "coefficient"),
            ("v", "y"),
            ("w", "x"),
            ("alternative", "greater"),
            ("save", "pcg"),
        ],
        &["z"],
    );
    assert!(text.contains("50000 random permutations"), "{text}");
    define(
        &mut e,
        "set.seed(1); \
         tc <- replicate(50000, { p <- sample.int(20); \
           qr.coef(qr(cbind(1, pts$x[p], pts$z)), pts$y)[2] }); \
         bc0 <- coef(lm(y ~ x + z, data = pts))[['x']]; tc0 <- 1e-9 * max(1, abs(bc0))",
    );
    holds(
        &mut e,
        &[
            same("pcg$statistic", "bc0"),
            within("pcg$statistic", "sp_coef_b", "1e-6"),
            same("pcg$replicates", "tc"),
            same("pcg$p.value", "(sum(tc >= bc0 - tc0) + 1) / 50001"),
            // against Stata's upper p, within four combined Monte Carlo errors
            format!("abs(pcg$p.value - sp_coef[2]) < {}", mc("sp_coef[2]", true)),
        ],
    );
    perm(
        &mut e,
        &ctx,
        &[
            ("statistic", "coefficient"),
            ("v", "y"),
            ("w", "x"),
            ("strata", "s"),
            ("alternative", "less"),
            ("save", "pcs"),
        ],
        &["z"],
    );
    define(
        &mut e,
        "set.seed(1); units <- split(1:20, factor(pts$s)); \
         tcs <- replicate(50000, { p <- 1:20; \
           for (u in units) p[u] <- u[sample.int(length(u))]; \
           qr.coef(qr(cbind(1, pts$x[p], pts$z)), pts$y)[2] })",
    );
    holds(
        &mut e,
        &[
            same("pcs$replicates", "tcs"),
            same("pcs$p.value", "(sum(tcs <= bc0 + tc0) + 1) / 50001"),
            format!(
                "abs(pcs$p.value - sp_coef_strata[1]) < {}",
                mc("sp_coef_strata[1]", true)
            ),
        ],
    );

    // ---- a correlation within strata, one-sided ----
    perm(
        &mut e,
        &ctx,
        &[
            ("statistic", "correlation"),
            ("v", "y"),
            ("w", "x"),
            ("strata", "s"),
            ("alternative", "greater"),
            ("save", "prs"),
        ],
        &[],
    );
    holds(
        &mut e,
        &[
            within("prs$statistic", "sp_rho", "1e-6"),
            format!(
                "abs(prs$p.value - sp_cor_strata[2]) < {}",
                mc("sp_cor_strata[2]", true)
            ),
        ],
    );

    // ---- seven rows: all 5040 orders of x listed, against Stata ----
    let c7_ctx = context(
        "c7",
        &[
            ("y", VarKind::Number),
            ("x", VarKind::Number),
            ("z", VarKind::Number),
        ],
    );
    for alt in ["less", "stata"] {
        let name = format!("p7_{alt}");
        let text = perm(
            &mut e,
            &c7_ctx,
            &[
                ("statistic", "coefficient"),
                ("v", "y"),
                ("w", "x"),
                ("alternative", alt),
                ("save", &name),
            ],
            &["z"],
        );
        assert!(
            text.contains("All 5040 arrangements listed (exact)"),
            "{text}"
        );
    }
    define(
        &mut e,
        "gr <- as.matrix(expand.grid(rep(list(1:7), 7))); \
         gr <- gr[rowSums(sapply(1:7, function(k) rowSums(gr == k) > 0)) == 7, ]; \
         t7 <- apply(gr, 1, function(p) qr.coef(qr(cbind(1, c7$x[p], c7$z)), c7$y)[2]); \
         b7 <- coef(lm(y ~ x + z, data = c7))[['x']]; tol7 <- 1e-9 * max(1, abs(b7)); \
         e7 <- c(less = mean(t7 <= b7 + tol7), greater = mean(t7 >= b7 - tol7))",
    );
    holds(
        &mut e,
        &[
            "nrow(gr) == 5040".into(),
            within("p7_less$statistic", "sp_c7_b", "1e-6"),
            same("sort(p7_less$replicates)", "sort(t7)"),
            same("p7_less$p.value", "e7[['less']]"),
            same("p7_stata$p.value", "min(1, 2 * min(e7))"),
            format!(
                "abs(sp_c7[1] - e7[['less']]) < {}",
                mc("e7[['less']]", false)
            ),
            format!(
                "abs(sp_c7[2] - e7[['greater']]) < {}",
                mc("e7[['greater']]", false)
            ),
            format!(
                "abs(sp_c7[3] - p7_stata$p.value) < 2 * {}",
                mc("e7[['greater']]", false)
            ),
        ],
    );

    // ---- refusals: a non-numeric predictor stops R with a reason ----
    let mut pg = pts_context();
    pg.dataset = "pg".into();
    define(
        &mut e,
        "pg <- transform(pts, x = ifelse(pts$x > 0, 'up', 'down'))",
    );
    let (outcome, text) = dialog(
        &mut e,
        &pg,
        &PERMUTATION_TEST,
        &[
            ("statistic", "coefficient"),
            ("v", "y"),
            ("w", "x"),
            ("save", "pbad"),
        ],
        &[("z", &["z"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Error, "{text}");
    assert!(text.contains("must be numeric"), "{text}");
    let mut bad = PERMUTATION_TEST.defaults(&ctx);
    bad.set_text("statistic", "coefficient");
    bad.set_text("v", "y");
    bad.set_text("w", "x");
    bad.set_text("strata", "z");
    bad.set_list("z", vec!["z".into()]);
    assert!(PERMUTATION_TEST.generate(&ctx, &bad, Style::Base).is_err());
}
