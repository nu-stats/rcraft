//! Statistics menu, meta-analysis (P6.11): one row per study, its effect size
//! computed from the data (a standardised mean difference, Hedges's g,
//! Cohen's d or Glass's delta, from two groups' sizes, means and standard
//! deviations; a log odds ratio, log Peto odds ratio, log risk ratio or risk
//! difference from a 2×2 table; Fisher's z from a correlation) or given with
//! its standard error, pooled by `metafor::rma` with random effects (REML, ML,
//! empirical Bayes, Paule-Mandel, Hedges, Sidik-Jonkman or
//! DerSimonian-Laird; Knapp-Hartung errors and a prediction interval if
//! asked) or a fixed effect (inverse variance, or Mantel-Haenszel with
//! `metafor::rma.mh`). Moderators make it a meta-regression (Stata's `meta
//! regress`). Then, if asked: subgroups and the test of their differences,
//! a cumulative and a leave-one-out analysis, forest, funnel, Galbraith,
//! L'Abbé and bubble plots, a test for small-study effects (Egger's,
//! Harbord's, Peters's or Begg's) and trim-and-fill.
//!
//! The effect sizes and the fit are Stata's `meta esize`, `meta summarize`,
//! `meta regress`, `meta bias` and `meta trimfill`, found by running both:
//! Hedges's g with Stata's approximate bias correction by default (or the
//! exact one), each with Stata's variances; half a count added to every cell
//! of a table with an empty cell for odds and risk ratios (also before
//! Mantel-Haenszel pooling), but not for risk differences and Peto's odds
//! ratio; the Mantel-Haenszel risk difference's standard error as Greenland
//! and Robins give it; Stata's prediction interval (t on k - 2 degrees of
//! freedom); Begg's test with Stata's continuity correction. The fitted
//! model stays in the workspace (with the effect sizes, when the dialog
//! computes them itself); the data are not changed.

use rcraft_rcodegen::dialog::{Args, Opt, Problem, Spec, TextDefault, TextRule, VarKind, Widget};
use rcraft_rcodegen::{r_name, r_string};

use crate::data::BASE;

const NUMBERS: &[VarKind] = &[VarKind::Number];

/// A column picker for one input of the effect size.
const fn input(id: &'static str, label: &'static str, hint: &'static str) -> Widget {
    Widget::Variable {
        id,
        label,
        kinds: NUMBERS,
        required: false,
        hint,
    }
}

const GROUPS: &[(&str, &str)] = &[
    ("n1", "n1i"),
    ("m1", "m1i"),
    ("sd1", "sd1i"),
    ("n2", "n2i"),
    ("m2", "m2i"),
    ("sd2", "sd2i"),
];

const TABLE: &[(&str, &str)] = &[("ev1", "ai"), ("ne1", "bi"), ("ev2", "ci"), ("ne2", "di")];

/// The pickers each effect size reads, with the argument of
/// `metafor::escalc` each one fills, in the order the dialog asks for them.
fn inputs(measure: &str) -> &'static [(&'static str, &'static str)] {
    match measure {
        "SMDA" | "SMD" | "COHEN" => GROUPS,
        // standardised by group 2's standard deviation
        "GLASS" => &[
            ("n1", "n1i"),
            ("m1", "m1i"),
            ("n2", "n2i"),
            ("m2", "m2i"),
            ("sd2", "sd2i"),
        ],
        // by group 1's: metafor's SMD1 divides by the column given as sd2i
        "GLASS1" => &[
            ("n1", "n1i"),
            ("m1", "m1i"),
            ("n2", "n2i"),
            ("m2", "m2i"),
            ("sd1", "sd2i"),
        ],
        "OR" | "RR" | "RD" | "PETO" => TABLE,
        "ZCOR" => &[("r", "ri"), ("nr", "ni")],
        _ => &[("est", "yi"), ("se", "sei")],
    }
}

fn is_table(measure: &str) -> bool {
    matches!(measure, "OR" | "RR" | "RD" | "PETO")
}

/// The scale the pooled estimate is shown on as well (Stata's eform for
/// ratios; a correlation for Fisher's z), and the function that gets there.
fn back(measure: &str) -> Option<(&'static str, &'static str)> {
    match measure {
        "OR" | "PETO" => Some(("Odds ratio", "exp")),
        "RR" => Some(("Risk ratio", "exp")),
        "ZCOR" => Some(("Correlation", "tanh")),
        _ => None,
    }
}

/// Whether the dialog computes the effect sizes itself (with `escalc`, then
/// on Stata's footing) before fitting.
fn is_adjusted(a: &Args) -> bool {
    match a.choice("measure") {
        "SMDA" | "GLASS" | "GLASS1" => true,
        "SMD" | "COHEN" => a.flag("holkinse"),
        _ => false,
    }
}

fn is_random(method: &str) -> bool {
    !matches!(method, "FE" | "MH")
}

/// The arguments naming the effect size and its columns and the data: what
/// `escalc`, `rma` and `rma.mh` share; with `zeros`, also `to = "none"` for
/// the measures Stata adds no half counts to (`rma.mh` takes two of them).
fn effect_args(a: &Args, zeros: bool) -> String {
    let measure = a.choice("measure");
    let mut parts: Vec<String> = Vec::new();
    match measure {
        "given" => {}
        "SMDA" | "COHEN" => parts.push("measure = \"SMD\"".into()),
        "GLASS" | "GLASS1" => parts.push("measure = \"SMD1\"".into()),
        m => parts.push(format!("measure = \"{m}\"")),
    }
    for (id, arg) in inputs(measure) {
        parts.push(format!("{arg} = {}", r_name(a.var_text(id))));
    }
    parts.push(format!("data = {}", a.data()));
    match measure {
        // Stata's variance of the exact Hedges's g: the large-sample variance
        // of d, times the squared correction
        "SMD" if !a.flag("holkinse") => parts.push("vtype = \"LS2\"".into()),
        "SMDA" | "COHEN" | "GLASS" | "GLASS1" => parts.push("correct = FALSE".into()),
        // Stata adds no half counts to empty cells for these two
        "RD" | "PETO" if zeros => parts.push("to = \"none\"".into()),
        _ => {}
    }
    parts.join(", ")
}

/// A column of the data as R code (`d[["n1"]]`).
fn column(a: &Args, id: &str) -> String {
    format!("{}[[{}]]", a.data(), r_string(a.var_text(id)))
}

/// `rma`'s own options after the data: labels, moderators, method, errors.
fn fit_options(a: &Args, method: &str) -> String {
    let mut out = String::new();
    if let Some(label) = a.var("label") {
        out.push_str(&format!(", slab = {label}"));
    }
    let mods = a.vars("mods");
    if !mods.is_empty() {
        let terms: Vec<String> = mods.iter().map(|m| r_name(m)).collect();
        out.push_str(&format!(", mods = ~ {}", terms.join(" + ")));
    }
    out.push_str(&format!(", method = \"{method}\""));
    if a.var("cluster").is_some() {
        // rma.mv maximises its likelihood with optim; no tau² iteration
        return out;
    }
    if is_random(method) && a.flag("knha") {
        // the truncated adjustment never makes the errors smaller
        let test = if a.flag("truncated") { "adhoc" } else { "knha" };
        out.push_str(&format!(", test = \"{test}\""));
    }
    out.push_str(control(method));
    out
}

/// The iterated estimators of tau², stopped where Stata stops: its
/// iterations follow the same path as metafor's (each iterate equal to 1e-15)
/// and stop once tau² changes by less than its default tolerance, 1e-6, one
/// step after metafor's own default (1e-5). Paule-Mandel's root is taken to
/// Stata's precision (metafor's own tolerance stops about 3e-5 away).
fn control(method: &str) -> &'static str {
    match method {
        "REML" | "ML" | "EB" => ", control = list(threshold = 1e-6)",
        "PM" => ", control = list(tol = 1e-10)",
        _ => "",
    }
}

/// The effect sizes put on Stata's footing, as R lines inside a `local`
/// that starts from `es <- escalc(...)`.
fn adjust_lines(a: &Args) -> String {
    let n1 = column(a, "n1");
    let n2 = column(a, "n2");
    let holkinse = a.flag("holkinse");
    match a.choice("measure") {
        // Hedges's g as Stata's default: d times 1 - 3/(4N - 9), its
        // variance times the square (or Hedges and Olkin's)
        "SMDA" => {
            let var = if holkinse {
                "  es$vi <- N / (n1 * n2) + es$yi^2 / (2 * (N - 3.94))\n"
            } else {
                "  es$vi <- J^2 * es$vi\n"
            };
            format!(
                "  n1 <- {n1}\n  n2 <- {n2}\n  N <- n1 + n2\n  \
                 J <- 1 - 3 / (4 * N - 9)\n  \
                 es$yi <- J * es$yi\n{var}"
            )
        }
        // Hedges and Olkin's variances (Stata's holkinse)
        "SMD" => format!(
            "  n1 <- {n1}\n  n2 <- {n2}\n  N <- n1 + n2\n  \
             es$vi <- N / (n1 * n2) + es$yi^2 / (2 * (N - 3.94))\n"
        ),
        "COHEN" => format!(
            "  n1 <- {n1}\n  n2 <- {n2}\n  N <- n1 + n2\n  \
             es$vi <- N / (n1 * n2) + es$yi^2 / (2 * (N - 2))\n"
        ),
        // Glass's delta with Stata's variance: the group whose standard
        // deviation divides has n - 1 in its term
        "GLASS1" => format!(
            "  n1 <- {n1}\n  n2 <- {n2}\n  \
             es$vi <- 1 / n1 + 1 / n2 + es$yi^2 / (2 * (n1 - 1))\n"
        ),
        _ => format!(
            "  n1 <- {n1}\n  n2 <- {n2}\n  \
             es$vi <- 1 / n1 + 1 / n2 + es$yi^2 / (2 * (n2 - 1))\n"
        ),
    }
}

/// The four cells of the table, without rows that miss one, as `t`.
fn cells(a: &Args) -> String {
    let cells: Vec<String> = TABLE
        .iter()
        .zip(["a", "b", "c", "d"])
        .map(|((id, _), cell)| format!("{cell} = {}", column(a, id)))
        .collect();
    format!("  t <- stats::na.omit(data.frame({}))\n", cells.join(", "))
}

fn is_mh_rd(a: &Args) -> bool {
    a.choice("method") == "MH" && a.choice("measure") == "RD"
}

/// An R function `gr(m, rows)`: a Mantel-Haenszel risk-difference fit `m`
/// on the data's `rows`, carrying Greenland and Robins' standard error (with
/// its z, p and interval) as Stata does, in place of metafor's Sato one.
fn gr_function(a: &Args) -> String {
    let cells: Vec<String> = TABLE
        .iter()
        .zip(["a", "b", "c", "d"])
        .map(|((id, _), cell)| format!("{cell} = {}[rows]", column(a, id)))
        .collect();
    format!(
        "  gr <- function(m, rows = seq_len(nrow({}))) {{\n    \
         t <- stats::na.omit(data.frame({}))\n    \
         n1 <- t$a + t$b; n2 <- t$c + t$d; N <- n1 + n2\n    \
         se <- sqrt(sum((t$a * t$b * n2^3 + t$c * t$d * n1^3) / (n1 * n2 * N^2)) / \
         sum(n1 * n2 / N)^2)\n    \
         z <- stats::qnorm(0.975)\n    \
         m$se <- se; m$vb[] <- se^2; m$zval <- m$beta[1] / se\n    \
         m$pval <- 2 * stats::pnorm(-abs(m$zval))\n    \
         m$ci.lb <- m$beta[1] - z * se; m$ci.ub <- m$beta[1] + z * se\n    \
         m\n  }}\n",
        a.data(),
        cells.join(", ")
    )
}

/// The study labels of the data's rows `rows` (an R expression), for tables.
fn row_labels(a: &Args, rows: &str) -> String {
    match a.var_text("label") {
        "" => format!("paste(\"Study\", {rows})"),
        _ => format!("as.character({}[{rows}])", column(a, "label")),
    }
}

/// A cumulative (`cumulative`) or leave-one-out table for a Mantel-Haenszel
/// risk difference, each refit carrying the Greenland-Robins error, as
/// Stata's: `o` holds the data's rows in the order of the steps.
fn mh_rd_table(a: &Args, fit: &str, cumulative: bool) -> String {
    let (title, order, rows) = match a.var("cumul") {
        Some(_) if cumulative => (
            format!(
                "\nCumulative meta-analysis, in the order of {}\n",
                a.var_text("cumul")
            ),
            format!("order({})", column(a, "cumul")),
            "o[seq_len(s)]",
        ),
        _ => (
            "\nLeave-one-out meta-analysis (the study left out)\n".to_string(),
            format!("seq_len(nrow({}))", a.data()),
            "o[-s]",
        ),
    };
    format!(
        "local({{\n\
         {}  \
         o <- {order}\n  \
         tab <- t(vapply(seq_along(o), function(s) {{\n    \
         .rows <- {rows}\n    \
         f <- gr(stats::update({fit}, subset = .rows), .rows)\n    \
         c(estimate = f$b[[1]], se = f$se, zval = f$zval, pval = f$pval, ci.lb = f$ci.lb, \
         ci.ub = f$ci.ub, Q = f$QE, I2 = f$I2, H2 = f$H2)\n  }}, numeric(9)))\n  \
         rownames(tab) <- {}\n  \
         cat({})\n  \
         print(tab)\n}})\n",
        gr_function(a),
        row_labels(a, "o"),
        r_string(&title)
    )
}

/// The fit: `rma` on the effect sizes (computed first, as `<save>_es`, when
/// the dialog puts them on Stata's footing itself), or `rma.mh`.
fn fit_code(a: &Args, fit: &str) -> String {
    let measure = a.choice("measure");
    let method = a.choice("method");
    if method == "MH" {
        let args = effect_args(a, false);
        let zeros = if measure == "RD" {
            "to = c(\"none\", \"none\")"
        } else {
            "add = c(1/2, 1/2), to = c(\"only0\", \"only0\")"
        };
        let label = a
            .var("label")
            .map(|l| format!(", slab = {l}"))
            .unwrap_or_default();
        let mut out = format!("{fit} <- metafor::rma.mh({args}, {zeros}{label})\n");
        if measure == "RD" {
            // metafor's summary gives Sato's standard error; Stata's is
            // Greenland and Robins', so the fit carries that one instead
            out.push_str(&format!(
                "{fit} <- local({{\n{}  gr({fit})\n}})\n\
                 cat(\"The standard error is Greenland and Robins' (Stata's), not Sato's.\\n\")\n",
                gr_function(a)
            ));
        }
        return out;
    }
    let options = fit_options(a, method);
    let es = r_name(&format!("{}_es", a.name_text("save")));
    let multilevel = a.var("cluster");
    if !is_adjusted(a) && multilevel.is_none() {
        return format!(
            "{fit} <- metafor::rma({}{options})\n{}",
            effect_args(a, true),
            stata_stop(a, fit)
        );
    }
    let mut out = if is_adjusted(a) {
        format!(
            "{es} <- local({{\n  \
             es <- metafor::escalc({})\n\
             {}  es\n}})\n",
            effect_args(a, true),
            adjust_lines(a)
        )
    } else {
        format!("{es} <- metafor::escalc({})\n", effect_args(a, true))
    };
    match multilevel {
        // studies within clusters, as Stata's meta multilevel: a random
        // effect for each level
        Some(cluster) => {
            out.push_str(&format!(
                "{es}$.study <- seq_len(nrow({es}))\n\
                 {fit} <- metafor::rma.mv(yi, vi, random = ~ 1 | {cluster}/.study, data = {es}{options})\n"
            ));
        }
        None => out.push_str(&format!(
            "{fit} <- metafor::rma(yi, vi, data = {es}{options})\n{}",
            stata_stop(a, fit)
        )),
    }
    out
}

/// For REML, ML and empirical Bayes: the fit again, stopped as Stata stops,
/// once tau² moves by less than 1e-6 relative to 1 + tau² (the same
/// iterations, which the first fit's tau² lets metafor's absolute threshold
/// follow).
fn stata_stop(a: &Args, fit: &str) -> String {
    if matches!(a.choice("method"), "REML" | "ML" | "EB") {
        format!(
            "{fit} <- stats::update({fit}, control = list(threshold = 1e-6 * (1 + {fit}$tau2)))\n"
        )
    } else {
        String::new()
    }
}

/// The model the tests for small-study effects, trim-and-fill and the
/// Galbraith plot use: the fit, or after Mantel-Haenszel the inverse-variance
/// one, as Stata does.
fn alt_model(a: &Args, fit: &str) -> String {
    if a.choice("method") == "MH" {
        format!("metafor::rma({}, method = \"FE\")", effect_args(a, true))
    } else {
        fit.to_string()
    }
}

/// Each subgroup fitted alone (the same model on its rows), then the test of
/// their differences: Cochran's Q among the subgroups' estimates, as Stata's
/// Q_b.
fn subgroup_code(a: &Args, fit: &str, axis: &str) -> String {
    let title = r_string(&format!("\nSubgroups of {}\n", a.var_text("subgroup")));
    let (helper, refit) = if is_mh_rd(a) {
        (
            gr_function(a),
            format!("gr(stats::update({fit}, subset = .rows), .rows)"),
        )
    } else {
        (
            String::new(),
            format!("stats::update({fit}, subset = .rows)"),
        )
    };
    // Stata's Q_b uses the groups' unadjusted errors, also when the table
    // shows Knapp-Hartung ones
    let plain = if is_random(a.choice("method")) && a.flag("knha") {
        format!(
            "  se <- vapply(split(seq_along(g), g), function(.rows) \
             stats::update({fit}, subset = .rows, test = \"z\")$se, numeric(1))\n"
        )
    } else {
        "  se <- tab[, \"se\"]\n".to_string()
    };
    // the forest plot with the studies in their groups, each group's
    // diamond under it, and the overall one at the bottom
    let forest = if a.flag("forest") {
        format!(
            "  gi <- as.character(g)[{fit}$not.na]; lv <- names(fits)\n  \
             rows <- numeric(length(gi)); polys <- numeric(length(lv))\n  \
             top <- length(gi) + 3 * length(lv); r <- top\n  \
             for (j in seq_along(lv)) {{\n    \
             i <- which(gi == lv[j]); rows[i] <- r - seq_along(i) + 1\n    \
             r <- r - length(i); polys[j] <- r; r <- r - 3\n  }}\n  \
             metafor::forest({fit}, rows = rows, ylim = c(-1.5, top + 3), header = TRUE, \
             showweights = TRUE{axis})\n  \
             for (j in seq_along(lv)) metafor::addpoly(fits[[j]], row = polys[j], \
             mlab = paste(\"Subgroup\", lv[j]){axis})\n"
        )
    } else {
        String::new()
    };
    format!(
        "local({{\n\
         {helper}  \
         g <- {}\n  \
         fits <- lapply(split(seq_along(g), g), function(.rows) {refit})\n  \
         tab <- t(vapply(fits, function(f) c(k = f$k, estimate = f$b[[1]], se = f$se, \
         ci.lb = f$ci.lb, ci.ub = f$ci.ub, tau2 = f$tau2, I2 = f$I2, H2 = f$H2, Q = f$QE), \
         numeric(9)))\n  \
         cat({title})\n  \
         print(tab)\n\
         {plain}  \
         w <- 1 / se^2; est <- tab[, \"estimate\"]\n  \
         qb <- sum(w * (est - sum(w * est) / sum(w))^2)\n  \
         cat(\"Test of group differences: Q_b =\", format(qb, digits = 7), \"on\", nrow(tab) - 1, \
         \"df, p =\", format(stats::pchisq(qb, nrow(tab) - 1, lower.tail = FALSE), digits = 4), \
         \"\\n\")\n\
         {forest}}})\n",
        column(a, "subgroup")
    )
}

/// The test for small-study effects chosen, on `model`.
fn bias_code(a: &Args, model: &str) -> String {
    let method = a.choice("method");
    match a.choice("egger") {
        "model" => format!("metafor::regtest({model})\n"),
        "lm" => format!("metafor::regtest({model}, model = \"lm\")\n"),
        "peters" => format!("metafor::regtest({model}, predictor = \"ninv\")\n"),
        // Harbord's score test as a meta-regression of Z/V on 1/sqrt(V),
        // with variance 1/V, in the model chosen (inverse variance after
        // Mantel-Haenszel)
        "harbord" => {
            let m = if method == "MH" { "FE" } else { method };
            let ctl = control(m);
            // the efficient score and its variance: for the log odds ratio
            // the hypergeometric ones, for the log risk ratio the binomial
            let score = if a.choice("measure") == "RR" {
                "  Z <- (t$a * N - m1 * n1) / (N - m1)\n  \
                 V <- n1 * n2 * m1 / (N * (N - m1))\n"
            } else {
                "  Z <- t$a - n1 * m1 / N\n  \
                 V <- n1 * n2 * m1 * (t$b + t$d) / (N^2 * (N - 1))\n"
            };
            format!(
                "local({{\n\
                 {}  \
                 n1 <- t$a + t$b; n2 <- t$c + t$d; N <- n1 + n2; m1 <- t$a + t$c\n\
                 {score}  \
                 h <- metafor::rma(yi = Z / V, vi = 1 / V, mods = ~ I(1 / sqrt(V)), method = \"{m}\"{ctl})\n  \
                 cat(\"\\nHarbord's test for small-study effects, as Stata's meta bias, harbord\\n\")\n  \
                 print(c(beta1 = h$b[2], se = h$se[2], z = h$zval[2], pval = h$pval[2]))\n}})\n",
                cells(a)
            )
        }
        // Begg's rank correlation: Kendall's score between the standardised
        // effects and their variances, with Stata's continuity correction
        "begg" => format!(
            "local({{\n  \
             m <- {model}\n  \
             y <- m$yi; v <- m$vi; w <- 1 / v\n  \
             b <- sum(w * y) / sum(w)\n  \
             t <- (y - b) / sqrt(v - 1 / sum(w))\n  \
             k <- length(y)\n  \
             S <- sum(sign(outer(t, t, \"-\")) * sign(outer(v, v, \"-\"))) / 2\n  \
             tv <- table(v)\n  \
             se <- sqrt((k * (k - 1) * (2 * k + 5) - sum(tv * (tv - 1) * (2 * tv + 5))) / 18)\n  \
             cat(\"\\nBegg's test for small-study effects, as Stata's meta bias, begg\\n\")\n  \
             print(c(score = S, se = se, z = (S - 1) / se, pval = 2 * stats::pnorm(-(abs(S) - 1) / se)))\n}})\n"
        ),
        _ => String::new(),
    }
}

/// The moderator values typed for predictions: rows separated by `;`, each
/// `count` finite numbers separated by commas or spaces; `None` when they do
/// not parse. Each number is kept as typed (and checked to be finite).
fn at_rows(text: &str, count: usize) -> Option<Vec<Vec<String>>> {
    let mut rows = Vec::new();
    for row in text.split(';').map(str::trim).filter(|r| !r.is_empty()) {
        let values: Vec<String> = row
            .split(|c: char| c == ',' || c.is_whitespace())
            .filter(|v| !v.is_empty())
            .map(str::to_string)
            .collect();
        let finite = values
            .iter()
            .all(|v| v.parse::<f64>().is_ok_and(f64::is_finite));
        if values.len() != count || !finite {
            return None;
        }
        rows.push(values);
    }
    (!rows.is_empty()).then_some(rows)
}

fn meta_check(a: &Args) -> Vec<Problem> {
    let mut out = Vec::new();
    let measure = a.choice("measure");
    let method = a.choice("method");
    for (id, _) in inputs(measure) {
        if a.var_text(id).is_empty() {
            let what = META
                .widgets
                .iter()
                .find_map(|w| match w {
                    Widget::Variable { id: wid, label, .. } if wid == id => Some(*label),
                    _ => None,
                })
                .unwrap_or("this input");
            out.push(Problem::new(
                id,
                format!("Choose the column for {what}; this effect size needs it."),
            ));
        }
    }
    if method == "MH" && !matches!(measure, "OR" | "RR" | "RD") {
        out.push(Problem::new(
            "method",
            "Mantel-Haenszel pools 2×2 tables: choose the log odds ratio, log risk ratio or risk \
             difference, or another method.",
        ));
    }
    let mods = a.vars("mods");
    if !mods.is_empty() {
        if method == "MH" {
            out.push(Problem::new(
                "mods",
                "A meta-regression is fitted by inverse variance or random effects, not \
                 Mantel-Haenszel.",
            ));
        }
        for (id, set, what) in [
            ("subgroup", !a.var_text("subgroup").is_empty(), "Subgroups"),
            (
                "cumul",
                !a.var_text("cumul").is_empty(),
                "A cumulative analysis",
            ),
            ("leave1out", a.flag("leave1out"), "A leave-one-out analysis"),
            ("trimfill", a.choice("trimfill") != "none", "Trim-and-fill"),
            ("galbraith", a.flag("galbraith"), "The Galbraith plot"),
            ("labbe", a.flag("labbe"), "The L'Abbé plot"),
            ("pi", a.flag("pi"), "The prediction interval"),
            (
                "egger",
                matches!(a.choice("egger"), "harbord" | "peters" | "begg"),
                "This test for small-study effects",
            ),
        ] {
            if set {
                out.push(Problem::new(
                    id,
                    format!("{what} works without moderators: clear the moderators, or this."),
                ));
            }
        }
    }
    if !is_random(method) {
        for (id, what) in [
            ("knha", "Knapp-Hartung errors"),
            ("pi", "A prediction interval"),
        ] {
            if a.flag(id) {
                out.push(Problem::new(
                    id,
                    format!("{what} need a random-effects model, as in Stata."),
                ));
            }
        }
    }
    if a.flag("truncated") && !a.flag("knha") {
        out.push(Problem::new(
            "truncated",
            "The truncation applies to Knapp-Hartung errors: tick those too.",
        ));
    }
    if a.flag("holkinse") && !matches!(measure, "SMDA" | "SMD" | "COHEN") {
        out.push(Problem::new(
            "holkinse",
            "Hedges and Olkin's variance is for Hedges's g and Cohen's d.",
        ));
    }
    match a.choice("egger") {
        "harbord" if !matches!(measure, "OR" | "RR") => out.push(Problem::new(
            "egger",
            "Harbord's test is for log odds and risk ratios, as in Stata.",
        )),
        "peters" if measure != "OR" => out.push(Problem::new(
            "egger",
            "Peters's test is for log odds ratios, as in Stata.",
        )),
        _ => {}
    }
    if a.var("cluster").is_some() {
        if !matches!(method, "REML" | "ML") {
            out.push(Problem::new(
                "method",
                "A multilevel meta-analysis is fitted by REML or maximum likelihood, as Stata's \
                 meta multilevel.",
            ));
        }
        for (id, set, what) in [
            ("subgroup", !a.var_text("subgroup").is_empty(), "Subgroups"),
            (
                "cumul",
                !a.var_text("cumul").is_empty(),
                "A cumulative analysis",
            ),
            ("leave1out", a.flag("leave1out"), "A leave-one-out analysis"),
            ("trimfill", a.choice("trimfill") != "none", "Trim-and-fill"),
            (
                "egger",
                a.choice("egger") != "none",
                "A test for small-study effects",
            ),
            ("galbraith", a.flag("galbraith"), "The Galbraith plot"),
            ("labbe", a.flag("labbe"), "The L'Abbé plot"),
            ("knha", a.flag("knha"), "Knapp-Hartung errors"),
            ("pi", a.flag("pi"), "The prediction interval"),
            ("studypred", a.flag("studypred"), "The study predictions"),
        ] {
            if set {
                out.push(Problem::new(
                    id,
                    format!("{what} works without clusters: clear the clusters, or this."),
                ));
            }
        }
    }
    if a.flag("studypred") && (mods.is_empty() || !is_random(method)) {
        out.push(Problem::new(
            "studypred",
            "The study predictions need moderators and a random-effects model, as Stata's \
             predict after meta regress.",
        ));
    }
    if !a.values.text("at").trim().is_empty() {
        if mods.is_empty() {
            out.push(Problem::new(
                "at",
                "Predictions at moderator values need moderators.",
            ));
        } else if mods
            .iter()
            .any(|m| a.ctx.column(m).is_some_and(|c| c.kind != VarKind::Number))
        {
            out.push(Problem::new(
                "at",
                "Predictions at moderator values need numeric moderators only.",
            ));
        } else if at_rows(a.values.text("at"), mods.len()).is_none() {
            out.push(Problem::new(
                "at",
                format!(
                    "Type one row of {} number(s) per prediction, rows separated by ';' (for \
                     example 2005; 2010).",
                    mods.len()
                ),
            ));
        }
    }
    if a.flag("labbe") && !is_table(measure) {
        out.push(Problem::new(
            "labbe",
            "The L'Abbé plot shows the two groups' event rates: it needs a 2×2 table.",
        ));
    }
    if a.name_text("save") == a.ctx.dataset {
        out.push(Problem::new(
            "save",
            format!("{} is the data frame; choose another name.", a.ctx.dataset),
        ));
    }
    out
}

pub const META: Spec = Spec {
    entry: "Meta-analysis",
    menu: "Statistics > Meta-analysis",
    about: "One row per study: an effect size computed from the data, or given with its \
            standard error, pooled by metafor::rma with random effects or a fixed effect \
            (inverse variance or Mantel-Haenszel), as Stata's meta esize and meta summarize; \
            with moderators, a meta-regression, as meta regress. The summary shows the pooled \
            estimate, its standard error and confidence interval, tau², I², H² and the test of \
            homogeneity. Subgroups, cumulative and leave-one-out analyses, plots, tests for \
            small-study effects (meta bias) and trim-and-fill (meta trimfill) follow if asked. \
            Studies missing an input are left out.",
    widgets: &[
        Widget::Choice {
            id: "measure",
            label: "Effect size",
            options: &[
                Opt {
                    value: "SMDA",
                    label: "Hedges's g (Stata's default)",
                },
                Opt {
                    value: "SMD",
                    label: "Hedges's g, exact correction",
                },
                Opt {
                    value: "COHEN",
                    label: "Cohen's d",
                },
                Opt {
                    value: "GLASS",
                    label: "Glass's delta (group 2's standard deviation)",
                },
                Opt {
                    value: "GLASS1",
                    label: "Glass's delta (group 1's standard deviation)",
                },
                Opt {
                    value: "OR",
                    label: "Log odds ratio, from a 2×2 table",
                },
                Opt {
                    value: "PETO",
                    label: "Log Peto odds ratio, from a 2×2 table",
                },
                Opt {
                    value: "RR",
                    label: "Log risk ratio, from a 2×2 table",
                },
                Opt {
                    value: "RD",
                    label: "Risk difference, from a 2×2 table",
                },
                Opt {
                    value: "ZCOR",
                    label: "Fisher's z, from a correlation and its sample size",
                },
                Opt {
                    value: "given",
                    label: "Given: an estimate and its standard error",
                },
            ],
            default: "SMDA",
            hint: "Group 1 is compared with group 2 (treated with control). Stata's default \
                   Hedges's g uses the approximate bias correction 1 - 3/(4N - 9); the exact one \
                   is esize(hedgesg, exact). Glass's delta is esize(glassdelta2) or \
                   esize(glassdelta1). A table with an empty cell gets half a count added to \
                   each of its cells for odds and risk ratios, but not for the risk difference or \
                   Peto's odds ratio, as Stata does. Fisher's z is atanh(r), with standard error \
                   1/sqrt(n - 3).",
        },
        input("n1", "Group 1 size (mean difference)", ""),
        input("m1", "Group 1 mean (mean difference)", ""),
        input("sd1", "Group 1 standard deviation (mean difference)", ""),
        input("n2", "Group 2 size (mean difference)", ""),
        input("m2", "Group 2 mean (mean difference)", ""),
        input("sd2", "Group 2 standard deviation (mean difference)", ""),
        input("ev1", "Group 1 with the event (2×2 table)", ""),
        input("ne1", "Group 1 without it (2×2 table)", ""),
        input("ev2", "Group 2 with the event (2×2 table)", ""),
        input("ne2", "Group 2 without it (2×2 table)", ""),
        input("r", "Correlation (Fisher's z)", ""),
        input("nr", "Sample size (Fisher's z)", ""),
        input("est", "Estimate (given)", ""),
        input("se", "Its standard error (given)", ""),
        Widget::Variable {
            id: "label",
            label: "Study labels",
            kinds: &[],
            required: false,
            hint: "Optional: names the studies in the forest plot and the tables.",
        },
        Widget::Choice {
            id: "method",
            label: "Model",
            options: &[
                Opt {
                    value: "REML",
                    label: "Random effects, REML",
                },
                Opt {
                    value: "ML",
                    label: "Random effects, maximum likelihood",
                },
                Opt {
                    value: "EB",
                    label: "Random effects, empirical Bayes",
                },
                Opt {
                    value: "PM",
                    label: "Random effects, Paule-Mandel",
                },
                Opt {
                    value: "HE",
                    label: "Random effects, Hedges",
                },
                Opt {
                    value: "SJ",
                    label: "Random effects, Sidik-Jonkman",
                },
                Opt {
                    value: "DL",
                    label: "Random effects, DerSimonian-Laird",
                },
                Opt {
                    value: "FE",
                    label: "Fixed effect, inverse variance",
                },
                Opt {
                    value: "MH",
                    label: "Fixed effect, Mantel-Haenszel (2×2 tables)",
                },
            ],
            default: "REML",
            hint: "REML is Stata's default, random(reml); the others are random(mle), \
                   random(ebayes), random(pmandel), random(hedges), random(sjonkman) and \
                   random(dlaird). For odds ratios, risk ratios and risk differences Stata's \
                   fixed means Mantel-Haenszel; fixed(ivariance) is inverse variance (and \
                   Stata's fixed for Peto's odds ratio). A Mantel-Haenszel risk difference gets \
                   Stata's Greenland-Robins standard error.",
        },
        Widget::Variables {
            id: "mods",
            label: "Moderators (meta-regression)",
            kinds: &[],
            min: 0,
            hint: "Optional: study-level columns the effect is regressed on, as Stata's meta \
                   regress. The summary then shows their coefficients, the residual tau², I² and \
                   H², R² and the tests of the moderators and of residual homogeneity.",
        },
        Widget::Check {
            id: "studypred",
            label: "Study predictions (meta-regression)",
            default: false,
            hint: "Each study's linear prediction and its standard error, random effect and \
                   fitted value, as Stata's predict xb, stdp, reffects and fitted after meta \
                   regress.",
        },
        Widget::Text {
            id: "at",
            label: "Predict at moderator values",
            default: TextDefault::Empty,
            rule: TextRule::Any,
            hint: "Optional, numeric moderators only: one row per prediction, the moderators' \
                   values in their order, rows separated by ';' (2005; 2010), as Stata's margins, \
                   at().",
        },
        Widget::Variable {
            id: "cluster",
            label: "Clusters (multilevel)",
            kinds: &[],
            required: false,
            hint: "Optional: studies are nested in these clusters (a lab, a paper), with a \
                   random effect for each level, as Stata's meta multilevel. REML or maximum \
                   likelihood.",
        },
        Widget::Variable {
            id: "subgroup",
            label: "Subgroups by",
            kinds: &[],
            required: false,
            hint: "Optional: each group pooled alone (its own tau²), then the test of group \
                   differences, as Stata's subgroup() option.",
        },
        Widget::Variable {
            id: "cumul",
            label: "Cumulative, in the order of",
            kinds: &[],
            required: false,
            hint: "Optional: the pooled estimate as studies are added in this column's order, as \
                   Stata's cumulative() option.",
        },
        Widget::Check {
            id: "leave1out",
            label: "Leave-one-out analysis",
            default: false,
            hint: "The pooled estimate without each study in turn, as Stata's leaveoneout.",
        },
        Widget::Check {
            id: "knha",
            label: "Knapp-Hartung standard errors",
            default: false,
            hint: "Random effects only: the pooled estimate's error and interval adjusted, with t \
                   on k - 1 degrees of freedom (k - p in a meta-regression), as Stata's \
                   se(khartung).",
        },
        Widget::Check {
            id: "truncated",
            label: "Truncated Knapp-Hartung (never smaller)",
            default: false,
            hint: "As Stata's se(khartung, truncated): the adjustment is at least 1.",
        },
        Widget::Check {
            id: "pi",
            label: "Prediction interval",
            default: false,
            hint: "Random effects only: where a new study's true effect may lie, with t on k - 2 \
                   degrees of freedom, as Stata's predinterval.",
        },
        Widget::Check {
            id: "holkinse",
            label: "Hedges and Olkin's variance",
            default: false,
            hint: "For Hedges's g and Cohen's d: Stata's holkinse option.",
        },
        Widget::Check {
            id: "eform",
            label: "Ratios or correlation too",
            default: true,
            hint: "For a log odds or risk ratio, the pooled ratio and its interval on the ratio \
                   scale (Stata's eform); for Fisher's z, on the correlation scale. The plots' \
                   axes follow.",
        },
        Widget::Check {
            id: "forest",
            label: "Forest plot",
            default: true,
            hint: "",
        },
        Widget::Check {
            id: "funnel",
            label: "Funnel plot",
            default: false,
            hint: "",
        },
        Widget::Check {
            id: "contours",
            label: "Contour-enhanced funnel plot",
            default: false,
            hint: "The funnel plot around zero, shaded where a study's effect would be \
                   significant at 10%, 5% and 1%, as Stata's meta funnelplot, contours(1 5 10).",
        },
        Widget::Check {
            id: "galbraith",
            label: "Galbraith plot",
            default: false,
            hint: "Each study's z against its precision, as Stata's meta galbraithplot.",
        },
        Widget::Check {
            id: "labbe",
            label: "L'Abbé plot",
            default: false,
            hint: "2×2 tables: group 1's event rate against group 2's, as Stata's meta \
                   labbeplot.",
        },
        Widget::Check {
            id: "bubble",
            label: "Bubble plot",
            default: true,
            hint: "With one numeric moderator: the effects against it, sized by weight, with the \
                   fitted line, as Stata's estat bubbleplot.",
        },
        Widget::Choice {
            id: "egger",
            label: "Test for small-study effects",
            options: &[
                Opt {
                    value: "none",
                    label: "None",
                },
                Opt {
                    value: "model",
                    label: "Egger: meta-regression on the standard error (meta bias, egger)",
                },
                Opt {
                    value: "lm",
                    label: "Egger, traditional (meta bias, egger traditional)",
                },
                Opt {
                    value: "harbord",
                    label: "Harbord (meta bias, harbord; log odds or risk ratios)",
                },
                Opt {
                    value: "peters",
                    label: "Peters (meta bias, peters; log odds ratios)",
                },
                Opt {
                    value: "begg",
                    label: "Begg's rank correlation (meta bias, begg)",
                },
            ],
            default: "none",
            hint: "The regression tests use the model chosen above (after Mantel-Haenszel, the \
                   inverse variance one, as Stata does); with moderators, Egger's test adjusts \
                   for them. The traditional test is Egger's original regression with a \
                   multiplicative dispersion.",
        },
        Widget::Choice {
            id: "trimfill",
            label: "Trim-and-fill",
            options: &[
                Opt {
                    value: "none",
                    label: "None",
                },
                Opt {
                    value: "L0",
                    label: "Linear estimator (Stata's default)",
                },
                Opt {
                    value: "R0",
                    label: "Run estimator (estimator(run))",
                },
            ],
            default: "none",
            hint: "Studies imputed on the side the funnel plot lacks, then the pooled estimate \
                   with them, as Stata's meta trimfill.",
        },
        Widget::Text {
            id: "save",
            label: "Save the model as",
            default: TextDefault::Fixed("ma"),
            rule: TextRule::Name,
            hint: "The fitted model stays in R's workspace under this name (and the effect sizes \
                   under this name with _es, when the dialog computes them on Stata's footing); \
                   a name already in use is replaced.",
        },
    ],
    needs: &["metafor"],
    styles: BASE,
    check: meta_check,
    code: |a, _| {
        let fit = a.name("save");
        let measure = a.choice("measure");
        let method = a.choice("method");
        let mods = a.vars("mods");
        let mut out = fit_code(a, &fit);
        out.push_str(&format!("summary({fit})\n"));
        if a.var("cluster").is_some() {
            out.push_str(&format!(
                "cat(\"\\nStandard deviation of each level (Stata's sd(_cons))\\n\")\n\
                 print(stats::setNames(sqrt({fit}$sigma2), {fit}$s.names))\n"
            ));
        }
        let shown = back(measure).filter(|_| a.flag("eform"));
        let pi = is_random(method) && a.flag("pi");
        if mods.is_empty() && (shown.is_some() || pi) {
            let what = shown.map_or("Estimate", |(name, _)| name);
            let with_pi = if pi {
                " and Stata's prediction interval"
            } else {
                ""
            };
            let mut args = String::new();
            if let Some((_, f)) = shown {
                args.push_str(&format!(", transf = {f}"));
            }
            if pi {
                args.push_str(", pi.type = \"Riley\"");
            }
            out.push_str(&format!(
                "cat(\"\\n{what}, pooled, with its 95% confidence interval{with_pi}\\n\")\n\
                 print(predict({fit}{args}))\n"
            ));
        }
        if !mods.is_empty() {
            if a.flag("studypred") {
                // predict xb and stdp, reffects, fitted (xb plus the random effect)
                out.push_str(&format!(
                    "local({{\n  \
                     p <- predict({fit}); re <- metafor::ranef({fit})\n  \
                     cat(\"\\nStudy predictions, as Stata's predict after meta regress\\n\")\n  \
                     print(data.frame(xb = p$pred, stdp = p$se, reffects = re$pred, \
                     fitted = p$pred + re$pred, row.names = {fit}$slab))\n}})\n"
                ));
            }
            if let Some(rows) = at_rows(a.values.text("at"), mods.len()) {
                let rows: Vec<String> = rows
                    .iter()
                    .map(|r| format!("c({})", r.join(", ")))
                    .collect();
                let transf = shown
                    .map(|(_, f)| format!(", transf = {f}"))
                    .unwrap_or_default();
                out.push_str(&format!(
                    "cat(\"\\nPredictions at the moderator values, as Stata's margins, at()\\n\")\n\
                     print(predict({fit}, newmods = rbind({}){transf}))\n",
                    rows.join(", ")
                ));
            }
        }
        let axis = shown
            .map(|(_, f)| format!(", atransf = {f}"))
            .unwrap_or_default();
        let subgroups = !a.var_text("subgroup").is_empty();
        if subgroups {
            out.push_str(&subgroup_code(a, &fit, &axis));
        }
        if a.var("cumul").is_some() && is_mh_rd(a) {
            out.push_str(&mh_rd_table(a, &fit, true));
        } else if let Some(order) = a.var("cumul") {
            out.push_str(&format!("print(metafor::cumul({fit}, order = {order}))\n"));
        }
        if a.flag("leave1out") && is_mh_rd(a) {
            out.push_str(&mh_rd_table(a, &fit, false));
        } else if a.flag("leave1out") {
            out.push_str(&format!("print(metafor::leave1out({fit}))\n"));
        }
        // with subgroups the forest plot is drawn with their diamonds
        if a.flag("forest") && !subgroups {
            out.push_str(&format!(
                "metafor::forest({fit}, header = TRUE, showweights = TRUE{axis})\n"
            ));
        }
        if a.flag("funnel") {
            out.push_str(&format!("metafor::funnel({fit}{axis})\n"));
        }
        if a.flag("contours") {
            out.push_str(&format!(
                "metafor::funnel({fit}, level = c(90, 95, 99), shade = c(\"white\", \"gray55\", \
                 \"gray75\"), refline = 0, legend = TRUE{axis})\n"
            ));
        }
        let alt = alt_model(a, &fit);
        if a.flag("galbraith") {
            out.push_str(&format!("metafor::galbraith({alt})\n"));
        }
        if a.flag("labbe") {
            out.push_str(&format!("metafor::labbe({fit})\n"));
        }
        let numeric_mod = matches!(mods.as_slice(), [m] if a
            .ctx
            .column(m)
            .is_some_and(|c| c.kind == VarKind::Number));
        if a.flag("bubble") && numeric_mod && a.var("cluster").is_none() {
            out.push_str(&format!("metafor::regplot({fit})\n"));
        }
        out.push_str(&bias_code(a, &alt));
        match a.choice("trimfill") {
            "L0" => out.push_str(&format!("metafor::trimfill({alt})\n")),
            "R0" => out.push_str(&format!("metafor::trimfill({alt}, estimator = \"R0\")\n")),
            _ => {}
        }
        out.trim_end().to_string()
    },
};

// ---- Multivariate meta-analysis (P6.11d) ----

/// Names the multivariate dialog's own long table uses.
const MV_RESERVED: &[&str] = &["study", "outcome", "yi"];

fn mv_check(a: &Args) -> Vec<Problem> {
    let mut out = Vec::new();
    let p = a.vars("outcomes").len();
    let need = p * (p + 1) / 2;
    if a.vars("wcov").len() != need {
        out.push(Problem::new(
            "wcov",
            format!(
                "{p} outcomes need {need} columns of within-study variances and covariances, in \
                 Stata's order: v11 v12 ... v1{p} v22 ... v{p}{p}."
            ),
        ));
    }
    let outcomes = a.vars("outcomes");
    for (i, y) in outcomes.iter().enumerate() {
        if outcomes.iter().take(i).any(|x| x == y) {
            out.push(Problem::new("outcomes", format!("{y} is chosen twice.")));
        }
    }
    for m in a.vars("mods") {
        if MV_RESERVED.contains(&m) || outcomes.contains(&m) {
            out.push(Problem::new(
                "mods",
                format!("{m} cannot be a moderator here: rename the column or choose another."),
            ));
        }
    }
    if a.name_text("save") == a.ctx.dataset {
        out.push(Problem::new(
            "save",
            format!("{} is the data frame; choose another name.", a.ctx.dataset),
        ));
    }
    out
}

/// R's `c("a", "b")` of column names, as strings.
fn names_vector(names: &[&str]) -> String {
    let quoted: Vec<String> = names.iter().map(|n| r_string(n)).collect();
    format!("c({})", quoted.join(", "))
}

pub const MV_META: Spec = Spec {
    entry: "Multivariate meta-analysis",
    menu: "Statistics > Multivariate meta-analysis",
    about: "Several outcomes per study, with their within-study variances and covariances, \
            pooled together by metafor::rma.mv, as Stata's meta mvregress: each outcome's \
            estimate, the between-study standard deviations and correlations (estat sd), and the \
            test of homogeneity. With moderators, each outcome has its own coefficients. Studies \
            missing a value are left out.",
    widgets: &[
        Widget::Variables {
            id: "outcomes",
            label: "Effect sizes (one column per outcome)",
            kinds: NUMBERS,
            min: 2,
            hint: "",
        },
        Widget::Variables {
            id: "wcov",
            label: "Within-study variances and covariances",
            kinds: NUMBERS,
            min: 3,
            hint: "In Stata's wcovvariables() order: v11 v12 ... v1p v22 ... vpp (two outcomes: \
                   the first one's variance, the covariance, the second one's variance).",
        },
        Widget::Variables {
            id: "mods",
            label: "Moderators",
            kinds: &[],
            min: 0,
            hint: "Optional: each outcome gets its own intercept and coefficients, as \
                   meta mvregress y1 y2 = x.",
        },
        Widget::Choice {
            id: "method",
            label: "Model",
            options: &[
                Opt {
                    value: "REML",
                    label: "Random effects, REML",
                },
                Opt {
                    value: "ML",
                    label: "Random effects, maximum likelihood",
                },
                Opt {
                    value: "FE",
                    label: "Fixed effects",
                },
            ],
            default: "REML",
            hint: "REML is Stata's default; random(mle) and fixed.",
        },
        Widget::Choice {
            id: "struct",
            label: "Between-study covariance",
            options: &[
                Opt {
                    value: "UN",
                    label: "Unstructured (Stata's default)",
                },
                Opt {
                    value: "DIAG",
                    label: "Independent",
                },
                Opt {
                    value: "CS",
                    label: "Exchangeable",
                },
                Opt {
                    value: "ID",
                    label: "Identity",
                },
            ],
            default: "UN",
            hint: "As Stata's covariance() option: unstructured, independent (own variances, no \
                   correlation), exchangeable (one variance, one correlation) or identity (one \
                   variance).",
        },
        Widget::Text {
            id: "save",
            label: "Save the model as",
            default: TextDefault::Fixed("mv"),
            rule: TextRule::Name,
            hint: "The fitted model stays in R's workspace under this name; a name already in \
                   use is replaced.",
        },
    ],
    needs: &["metafor"],
    styles: BASE,
    check: mv_check,
    code: |a, _| {
        let fit = a.name("save");
        let data = a.data();
        let outcomes = names_vector(&a.vars("outcomes"));
        let wcov = names_vector(&a.vars("wcov"));
        let mods = a.vars("mods");
        let (x, keep, extra, terms) = if mods.is_empty() {
            (String::new(), "y, w", String::new(), String::new())
        } else {
            let terms: Vec<String> = mods
                .iter()
                .map(|m| format!(" + outcome:{}", r_name(m)))
                .collect();
            (
                format!("  x <- {data}[{}]\n", names_vector(&mods)),
                "y, w, x",
                ", x[rep(seq_len(k), each = p), , drop = FALSE]".to_string(),
                terms.join(""),
            )
        };
        let subset_x = if mods.is_empty() {
            ""
        } else {
            "; x <- x[keep, , drop = FALSE]"
        };
        let method = a.choice("method");
        let random = if method == "FE" {
            String::new()
        } else {
            format!(
                ", random = ~ outcome | study, struct = \"{}\", method = \"{method}\"",
                a.choice("struct")
            )
        };
        let mut out = format!(
            "{fit} <- local({{\n  \
             y <- as.matrix({data}[{outcomes}])\n  \
             w <- as.matrix({data}[{wcov}])\n\
             {x}  \
             keep <- stats::complete.cases({keep})\n  \
             y <- y[keep, , drop = FALSE]; w <- w[keep, , drop = FALSE]{subset_x}\n  \
             k <- nrow(y); p <- ncol(y)\n  \
             long <- data.frame(study = rep(seq_len(k), each = p), outcome = factor(rep(colnames(y), \
             k), levels = colnames(y)), yi = c(t(y)){extra})\n  \
             # each study's within-study covariance matrix, from Stata's v11 v12 ... vpp\n  \
             V <- metafor::bldiag(lapply(seq_len(k), function(i) {{\n    \
             S <- matrix(0, p, p); S[lower.tri(S, diag = TRUE)] <- w[i, ]\n    \
             S + t(S) - diag(diag(S), p)\n  }}))\n  \
             metafor::rma.mv(yi, V, mods = ~ outcome - 1{terms}{random}, data = long)\n}})\n\
             summary({fit})\n"
        );
        if method != "FE" {
            out.push_str(&format!(
                "cat(\"\\nBetween-study standard deviations and correlations, as Stata's estat sd\\n\")\n\
                 print(c(sd = sqrt({fit}$tau2), corr = {fit}$rho))\n"
            ));
        }
        out.trim_end().to_string()
    },
};

pub const SPECS: &[&Spec] = &[&META, &MV_META];

#[cfg(test)]
mod tests {
    use super::*;
    use rcraft_rcodegen::dialog::{ColumnInfo, Context, Style, Values};

    fn ctx() -> Context {
        let col = |name: &str| ColumnInfo {
            name: name.into(),
            kind: VarKind::Number,
        };
        let mut columns: Vec<ColumnInfo> = [
            "n1", "m1", "sd1", "n2", "m2", "sd2", "a", "b", "c", "d", "est", "se", "r", "n", "year",
        ]
        .into_iter()
        .map(col)
        .collect();
        for name in ["study", "grp"] {
            columns.push(ColumnInfo {
                name: name.into(),
                kind: VarKind::Text,
            });
        }
        Context {
            dataset: "dat".into(),
            frames: Vec::new(),
            columns,
        }
    }

    fn values(set: &[(&str, &str)]) -> Values {
        let mut v = META.defaults(&ctx());
        for (id, t) in [
            ("n1", "n1"),
            ("m1", "m1"),
            ("sd1", "sd1"),
            ("n2", "n2"),
            ("m2", "m2"),
            ("sd2", "sd2"),
            ("ev1", "a"),
            ("ne1", "b"),
            ("ev2", "c"),
            ("ne2", "d"),
            ("r", "r"),
            ("nr", "n"),
            ("est", "est"),
            ("se", "se"),
        ] {
            v.set_text(id, t);
        }
        for (id, t) in set {
            v.set_text(id, *t);
        }
        v
    }

    fn with(set: &[(&str, &str)], flags: &[(&str, bool)]) -> Result<String, Vec<Problem>> {
        let mut v = values(set);
        for (id, on) in flags {
            v.set_flag(id, *on);
        }
        META.generate(&ctx(), &v, Style::Base)
    }

    fn code(set: &[(&str, &str)]) -> Result<String, Vec<Problem>> {
        with(set, &[])
    }

    fn problem(set: &[(&str, &str)], flags: &[(&str, bool)]) -> String {
        with(set, flags)
            .err()
            .map(|p| {
                p.iter()
                    .map(|p| p.message.clone())
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default()
    }

    #[test]
    fn stata_default_hedges_g_with_reml_and_a_forest_plot_by_default() {
        let c = code(&[("label", "study")]).unwrap_or_default();
        assert_eq!(
            c,
            "ma_es <- local({\n  \
             es <- metafor::escalc(measure = \"SMD\", n1i = n1, m1i = m1, sd1i = sd1, n2i = n2, \
             m2i = m2, sd2i = sd2, data = dat, correct = FALSE)\n  \
             n1 <- dat[[\"n1\"]]\n  n2 <- dat[[\"n2\"]]\n  N <- n1 + n2\n  \
             J <- 1 - 3 / (4 * N - 9)\n  \
             es$yi <- J * es$yi\n  \
             es$vi <- J^2 * es$vi\n  \
             es\n})\n\
             ma <- metafor::rma(yi, vi, data = ma_es, slab = study, method = \"REML\", \
             control = list(threshold = 1e-6))\n\
             ma <- stats::update(ma, control = list(threshold = 1e-6 * (1 + ma$tau2)))\n\
             summary(ma)\n\
             metafor::forest(ma, header = TRUE, showweights = TRUE)"
        );
    }

    #[test]
    fn exact_g_glass_cohen_and_hedges_olkin_write_their_own_variances() {
        let c = code(&[("measure", "SMD")]).unwrap_or_default();
        assert!(
            c.starts_with(
                "ma <- metafor::rma(measure = \"SMD\", n1i = n1, m1i = m1, sd1i = sd1, n2i = n2, \
                 m2i = m2, sd2i = sd2, data = dat, vtype = \"LS2\", method = \"REML\""
            ),
            "{c}"
        );
        let c = code(&[("measure", "COHEN")]).unwrap_or_default();
        assert!(
            c.contains("sd2i = sd2, data = dat, correct = FALSE, method"),
            "{c}"
        );
        let c = code(&[("measure", "GLASS"), ("sd1", "")]).unwrap_or_default();
        assert!(
            c.contains(
                "escalc(measure = \"SMD1\", n1i = n1, m1i = m1, n2i = n2, m2i = m2, sd2i = sd2"
            ),
            "{c}"
        );
        assert!(
            c.contains("es$vi <- 1 / n1 + 1 / n2 + es$yi^2 / (2 * (n2 - 1))"),
            "{c}"
        );
        let c = code(&[("measure", "GLASS1"), ("sd2", "")]).unwrap_or_default();
        assert!(c.contains("m2i = m2, sd2i = sd1, data"), "{c}");
        assert!(c.contains("(2 * (n1 - 1))"), "{c}");
        let c = with(&[("measure", "COHEN")], &[("holkinse", true)]).unwrap_or_default();
        assert!(
            c.contains("es$vi <- N / (n1 * n2) + es$yi^2 / (2 * (N - 2))"),
            "{c}"
        );
        let c = with(&[], &[("holkinse", true)]).unwrap_or_default();
        assert!(
            c.contains("es$yi <- J * es$yi\n  es$vi <- N / (n1 * n2) + es$yi^2 / (2 * (N - 3.94))"),
            "{c}"
        );
    }

    #[test]
    fn tables_show_ratios_and_mantel_haenszel_adds_half_counts() {
        let c = with(
            &[("measure", "OR"), ("method", "MH"), ("egger", "model")],
            &[("funnel", true)],
        )
        .unwrap_or_default();
        assert!(
            c.starts_with(
                "ma <- metafor::rma.mh(measure = \"OR\", ai = a, bi = b, ci = c, di = d, data = dat, \
                 add = c(1/2, 1/2), to = c(\"only0\", \"only0\"))\n"
            ),
            "{c}"
        );
        assert!(c.contains("print(predict(ma, transf = exp))"), "{c}");
        assert!(
            c.contains("metafor::forest(ma, header = TRUE, showweights = TRUE, atransf = exp)"),
            "{c}"
        );
        assert!(c.contains("metafor::funnel(ma, atransf = exp)"), "{c}");
        // Egger's test after Mantel-Haenszel uses the inverse-variance model
        assert!(
            c.ends_with(
                "metafor::regtest(metafor::rma(measure = \"OR\", ai = a, bi = b, ci = c, di = d, \
                 data = dat, method = \"FE\"))"
            ),
            "{c}"
        );
    }

    #[test]
    fn risk_differences_carry_greenland_robins_and_peto_adds_no_half_counts() {
        let c = code(&[("measure", "RD"), ("method", "MH")]).unwrap_or_default();
        assert!(
            c.starts_with(
                "ma <- metafor::rma.mh(measure = \"RD\", ai = a, bi = b, ci = c, di = d, data = dat, \
                 to = c(\"none\", \"none\"))\nma <- local({\n"
            ),
            "{c}"
        );
        assert!(c.contains("m$se <- se; m$vb[] <- se^2"), "{c}");
        assert!(c.contains("Greenland and Robins'"), "{c}");
        assert!(!c.contains("predict("), "{c}");
        let c = code(&[("measure", "PETO"), ("method", "FE")]).unwrap_or_default();
        assert!(
            c.starts_with(
                "ma <- metafor::rma(measure = \"PETO\", ai = a, bi = b, ci = c, di = d, data = dat, \
                 to = \"none\", method = \"FE\")\n"
            ),
            "{c}"
        );
        assert!(c.contains("Odds ratio, pooled"), "{c}");
        let c = code(&[("measure", "ZCOR"), ("method", "DL")]).unwrap_or_default();
        assert!(
            c.starts_with("ma <- metafor::rma(measure = \"ZCOR\", ri = r, ni = n, data = dat"),
            "{c}"
        );
        assert!(c.contains("print(predict(ma, transf = tanh))"), "{c}");
    }

    #[test]
    fn knapp_hartung_paule_mandel_and_the_prediction_interval() {
        let c = with(
            &[("measure", "given"), ("method", "PM")],
            &[("knha", true), ("pi", true), ("forest", false)],
        )
        .unwrap_or_default();
        assert_eq!(
            c,
            "ma <- metafor::rma(yi = est, sei = se, data = dat, method = \"PM\", test = \"knha\", \
             control = list(tol = 1e-10))\n\
             summary(ma)\n\
             cat(\"\\nEstimate, pooled, with its 95% confidence interval and Stata's prediction \
             interval\\n\")\n\
             print(predict(ma, pi.type = \"Riley\"))"
        );
        let c = with(
            &[("measure", "given"), ("method", "DL")],
            &[("knha", true), ("truncated", true)],
        )
        .unwrap_or_default();
        assert!(c.contains("method = \"DL\", test = \"adhoc\")"), "{c}");
    }

    #[test]
    fn meta_regression_subgroups_and_the_other_analyses() {
        let mut v = values(&[("measure", "given"), ("method", "DL")]);
        v.set_list("mods", vec!["year".into()]);
        v.set_flag("forest", false);
        let c = META.generate(&ctx(), &v, Style::Base).unwrap_or_default();
        assert_eq!(
            c,
            "ma <- metafor::rma(yi = est, sei = se, data = dat, mods = ~ year, method = \"DL\")\n\
             summary(ma)\n\
             metafor::regplot(ma)"
        );
        let c = with(
            &[
                ("measure", "given"),
                ("method", "DL"),
                ("subgroup", "grp"),
                ("cumul", "year"),
                ("trimfill", "R0"),
                ("egger", "begg"),
            ],
            &[("leave1out", true), ("galbraith", true), ("forest", false)],
        )
        .unwrap_or_default();
        assert!(c.contains("g <- dat[[\"grp\"]]"), "{c}");
        assert!(c.contains("stats::update(ma, subset = .rows)"), "{c}");
        assert!(c.contains("cat(\"\\nSubgroups of grp\\n\")"), "{c}");
        assert!(
            c.contains("print(metafor::cumul(ma, order = year))\nprint(metafor::leave1out(ma))\nmetafor::galbraith(ma)\n"),
            "{c}"
        );
        assert!(c.contains("z = (S - 1) / se"), "{c}");
        assert!(
            c.ends_with("metafor::trimfill(ma, estimator = \"R0\")"),
            "{c}"
        );
        let c = with(
            &[("measure", "OR"), ("method", "DL"), ("egger", "harbord")],
            &[("labbe", true), ("forest", false)],
        )
        .unwrap_or_default();
        assert!(c.contains("metafor::labbe(ma)"), "{c}");
        assert!(
            c.contains("mods = ~ I(1 / sqrt(V)), method = \"DL\")"),
            "{c}"
        );
        let c = code(&[("measure", "OR"), ("egger", "peters")]).unwrap_or_default();
        assert!(
            c.ends_with("metafor::regtest(ma, predictor = \"ninv\")"),
            "{c}"
        );
    }

    #[test]
    fn missing_inputs_and_bad_choices_are_refused() {
        assert!(problem(&[("sd2", "")], &[]).contains("Group 2 standard deviation"));
        assert!(problem(&[("measure", "RR"), ("ne2", "")], &[]).contains("Group 2 without it"));
        assert!(problem(&[("measure", "given"), ("method", "MH")], &[]).contains("Mantel-Haenszel"));
        assert!(problem(&[("measure", "PETO"), ("method", "MH")], &[]).contains("Mantel-Haenszel"));
        assert!(problem(&[("save", "dat")], &[]).contains("data frame"));
        assert!(problem(&[("method", "FE")], &[("pi", true)]).contains("random-effects"));
        assert!(problem(&[], &[("truncated", true)]).contains("tick those too"));
        assert!(problem(&[("measure", "OR")], &[("holkinse", true)]).contains("Olkin"));
        assert!(problem(&[("egger", "harbord")], &[]).contains("log odds and risk ratios"));
        assert!(code(&[("measure", "RR"), ("egger", "harbord")]).is_ok());
        assert!(problem(&[("measure", "RR"), ("egger", "peters")], &[]).contains("Peters"));
        assert!(problem(&[("cluster", "grp"), ("method", "DL")], &[]).contains("multilevel"));
        assert!(problem(&[("cluster", "grp"), ("subgroup", "grp")], &[]).contains("clusters"));
        assert!(problem(&[], &[("studypred", true)]).contains("moderators"));
        assert!(problem(&[("at", "2005")], &[]).contains("need moderators"));
        let mut v = values(&[("at", "2005, 1; 2010")]);
        v.set_list("mods", vec!["year".into()]);
        let p = META
            .generate(&ctx(), &v, Style::Base)
            .err()
            .unwrap_or_default();
        assert!(p.iter().any(|p| p.message.contains("1 number(s)")), "{p:?}");
        let mut v = values(&[("at", "2005")]);
        v.set_list("mods", vec!["grp".into()]);
        let p = META
            .generate(&ctx(), &v, Style::Base)
            .err()
            .unwrap_or_default();
        assert!(
            p.iter().any(|p| p.message.contains("numeric moderators")),
            "{p:?}"
        );
        assert!(problem(&[], &[("labbe", true)]).contains("2×2 table"));
        let mut v = values(&[("cumul", "year")]);
        v.set_list("mods", vec!["year".into()]);
        let p = META
            .generate(&ctx(), &v, Style::Base)
            .err()
            .unwrap_or_default();
        assert!(
            p.iter().any(|p| p.message.contains("without moderators")),
            "{p:?}"
        );
        // the pickers of another effect size are not needed
        assert!(code(&[("measure", "given"), ("n1", "")]).is_ok());
        assert!(code(&[("measure", "GLASS"), ("sd1", "")]).is_ok());
    }

    #[test]
    fn multilevel_predictions_and_risk_difference_tables() {
        let c = with(
            &[("measure", "given"), ("cluster", "grp")],
            &[("forest", false)],
        )
        .unwrap_or_default();
        assert_eq!(
            c,
            "ma_es <- metafor::escalc(yi = est, sei = se, data = dat)\n\
             ma_es$.study <- seq_len(nrow(ma_es))\n\
             ma <- metafor::rma.mv(yi, vi, random = ~ 1 | grp/.study, data = ma_es, method = \"REML\")\n\
             summary(ma)\n\
             cat(\"\\nStandard deviation of each level (Stata's sd(_cons))\\n\")\n\
             print(stats::setNames(sqrt(ma$sigma2), ma$s.names))"
        );
        let mut v = values(&[("measure", "given"), ("at", "2005; 2010")]);
        v.set_list("mods", vec!["year".into()]);
        v.set_flag("studypred", true);
        v.set_flag("forest", false);
        v.set_flag("bubble", false);
        let c = META.generate(&ctx(), &v, Style::Base).unwrap_or_default();
        assert!(c.contains("re <- metafor::ranef(ma)"), "{c}");
        assert!(
            c.ends_with("print(predict(ma, newmods = rbind(c(2005), c(2010))))"),
            "{c}"
        );
        let c = with(
            &[
                ("measure", "RD"),
                ("method", "MH"),
                ("cumul", "year"),
                ("subgroup", "grp"),
            ],
            &[("leave1out", true), ("contours", true)],
        )
        .unwrap_or_default();
        assert!(
            c.contains("gr(stats::update(ma, subset = .rows), .rows)"),
            "{c}"
        );
        assert!(c.contains("o <- order(dat[[\"year\"]])"), "{c}");
        assert!(c.contains(".rows <- o[-s]"), "{c}");
        assert!(c.contains("metafor::addpoly(fits[[j]]"), "{c}");
        assert!(
            !c.contains("metafor::forest(ma, header = TRUE, showweights = TRUE)\n"),
            "{c}"
        );
        assert!(c.contains("level = c(90, 95, 99)"), "{c}");
        let c = with(
            &[("measure", "given"), ("subgroup", "grp")],
            &[("knha", true), ("forest", false)],
        )
        .unwrap_or_default();
        assert!(c.contains("subset = .rows, test = \"z\")$se"), "{c}");
        let c = code(&[("measure", "RR"), ("egger", "harbord")]).unwrap_or_default();
        assert!(c.contains("Z <- (t$a * N - m1 * n1) / (N - m1)"), "{c}");
    }

    #[test]
    fn the_multivariate_dialog_builds_stata_s_covariance_blocks() {
        let mut ctx = ctx();
        for name in ["est2", "v11", "v12", "v22"] {
            ctx.columns.push(ColumnInfo {
                name: name.into(),
                kind: VarKind::Number,
            });
        }
        let mut v = MV_META.defaults(&ctx);
        v.set_list("outcomes", vec!["est".into(), "est2".into()]);
        v.set_list("wcov", vec!["v11".into(), "v12".into(), "v22".into()]);
        v.set_list("mods", vec!["year".into()]);
        let c = MV_META.generate(&ctx, &v, Style::Base).unwrap_or_default();
        assert!(
            c.starts_with("mv <- local({\n  y <- as.matrix(dat[c(\"est\", \"est2\")])"),
            "{c}"
        );
        assert!(c.contains("x <- dat[c(\"year\")]"), "{c}");
        assert!(
            c.contains(
                "metafor::rma.mv(yi, V, mods = ~ outcome - 1 + outcome:year, random = ~ outcome | \
                 study, struct = \"UN\", method = \"REML\", data = long)"
            ),
            "{c}"
        );
        assert!(
            c.ends_with("print(c(sd = sqrt(mv$tau2), corr = mv$rho))"),
            "{c}"
        );
        v.set_text("method", "FE");
        let c = MV_META.generate(&ctx, &v, Style::Base).unwrap_or_default();
        assert!(c.contains("outcome:year, data = long)"), "{c}");
        assert!(!c.contains("estat sd"), "{c}");
        v.set_list(
            "wcov",
            vec!["v11".into(), "v12".into(), "v22".into(), "se".into()],
        );
        let p = MV_META
            .generate(&ctx, &v, Style::Base)
            .err()
            .unwrap_or_default();
        assert!(
            p.iter().any(|p| p.message.contains("need 3 columns")),
            "{p:?}"
        );
        v.set_list("wcov", vec!["v11".into(), "v12".into(), "v22".into()]);
        v.set_list("mods", vec!["study".into()]);
        let p = MV_META
            .generate(&ctx, &v, Style::Base)
            .err()
            .unwrap_or_default();
        assert!(
            p.iter()
                .any(|p| p.message.contains("cannot be a moderator")),
            "{p:?}"
        );
    }
}
