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
