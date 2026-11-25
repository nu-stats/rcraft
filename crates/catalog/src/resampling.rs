//! Statistics menu, resampling (P6.8): the bootstrap, with `boot::boot` and the
//! normal, percentile, BCa, basic and studentised intervals of `boot::boot.ci`
//! (and Stata's normal, percentile and bias-corrected ones worked out from the
//! replicates), of rows, of rows within strata or of whole clusters; and the
//! permutation test, written in base R: the labels of the data are shuffled
//! with `sample`, within strata when asked, or every arrangement is listed
//! when there are no more of them than the number of shuffles asked for.
//! Neither changes the data. Both keep their result in the workspace under the
//! name the dialog asks for.

use rcraft_rcodegen::dialog::{expression_problem, Args, Opt, Problem, Spec, TextDefault};
use rcraft_rcodegen::dialog::{TextRule, VarKind, Widget};
use rcraft_rcodegen::r_string;

use crate::data::BASE;

const NUMBERS: &[VarKind] = &[VarKind::Number];
/// What can name a group.
const GROUPS: &[VarKind] = &[
    VarKind::Number,
    VarKind::Factor,
    VarKind::Text,
    VarKind::Logical,
];

/// A whole number of at least `least`, or a problem on the widget.
fn whole(a: &Args, id: &'static str, what: &str, least: u32, out: &mut Vec<Problem>) {
    match a.number(id).trim().parse::<u32>() {
        Ok(n) if n >= least => {}
        _ => out.push(Problem::new(
            id,
            format!("{what} must be a whole number, {least} or more."),
        )),
    }
}

/// The result is not saved over the data frame.
fn not_the_data(a: &Args, out: &mut Vec<Problem>) {
    if a.name_text("save") == a.ctx.dataset {
        out.push(Problem::new(
            "save",
            format!("{} is the data frame; choose another name.", a.ctx.dataset),
        ));
    }
}

// ---- Bootstrap ----

const STATISTICS: &[Opt] = &[
    Opt {
        value: "mean",
        label: "Mean",
    },
    Opt {
        value: "median",
        label: "Median",
    },
    Opt {
        value: "correlation",
        label: "Correlation of two variables",
    },
    Opt {
        value: "coefficient",
        label: "Coefficients of a linear model",
    },
    Opt {
        value: "custom",
        label: "A statistic you write (R)",
    },
];

/// The interval checks, the `boot.ci` type each asks for, and its title.
const BOOT_TYPES: &[(&str, &str, &str)] = &[
    ("normal", "norm", "Normal"),
    ("percentile", "perc", "Percentile"),
    ("bca", "bca", "BCa"),
    ("basic", "basic", "Basic"),
    ("student", "stud", "Studentised"),
];

fn bootstrap_check(a: &Args) -> Vec<Problem> {
    let mut out = Vec::new();
    whole(a, "reps", "The number of replications", 100, &mut out);
    whole(a, "seed", "The seed", 0, &mut out);
    match a.number("level").trim().parse::<f64>() {
        Ok(l) if l > 0.0 && l < 100.0 => {}
        _ => out.push(Problem::new(
            "level",
            "The confidence level must be between 0 and 100.",
        )),
    }
    let v = a.var_text("v");
    let x = a.vars("x");
    let kind = a.choice("statistic");
    match kind {
        "correlation" if x.len() != 1 => out.push(Problem::new(
            "x",
            "A correlation needs exactly one second variable.",
        )),
        "coefficient" if x.is_empty() => {
            out.push(Problem::new("x", "Choose at least one predictor."));
        }
        _ => {}
    }
    if kind != "mean" && kind != "median" && x.contains(&v) {
        out.push(Problem::new(
            "x",
            format!("{v} is the first variable; it cannot also be in this list."),
        ));
    }
    if kind == "custom" {
        let e = a.expr("expr");
        if e.is_empty() {
            out.push(Problem::new("expr", "Type the statistic to bootstrap."));
        } else if let Some(m) = expression_problem(e) {
            out.push(Problem::new("expr", m));
        }
    }
    let (cluster, strata) = (a.var_text("cluster"), a.var_text("strata"));
    if !cluster.is_empty() && cluster == strata {
        out.push(Problem::new(
            "strata",
            "The strata must be a different variable from the clusters.",
        ));
    }
    if a.flag("student") {
        if kind != "mean" && kind != "coefficient" {
            out.push(Problem::new(
                "student",
                "A studentised interval needs each replicate's own variance; the dialog has one \
                 for a mean and for the coefficients of a linear model only.",
            ));
        } else if !cluster.is_empty() {
            out.push(Problem::new(
                "student",
                "A studentised interval with clusters would need each replicate's variance to \
                 allow for the clusters; it is not offered.",
            ));
        }
    }
    let any = BOOT_TYPES.iter().any(|(flag, _, _)| a.flag(flag)) || a.flag("stata");
    if !any {
        out.push(Problem::new(
            "normal",
            "Choose at least one kind of interval.",
        ));
    }
    not_the_data(a, &mut out);
    out
}

/// The statistic as an R function body of the resampled data frame `d`, and
/// the row label of a single estimate (empty when the function names its
/// estimates).
fn bootstrap_statistic(a: &Args, xs: &[&str]) -> (String, String) {
    let v = a.var_text("v");
    let student = a.flag("student");
    match a.choice("statistic") {
        "median" => ("stats::median(d[[1]])".into(), format!("median of {v}")),
        "correlation" => (
            "stats::cor(d[[1]], d[[2]])".into(),
            format!(
                "correlation of {v} and {}",
                xs.first().copied().unwrap_or_default()
            ),
        ),
        "coefficient" => {
            let nm = std::iter::once(r_string("(Intercept)"))
                .chain(xs.iter().map(|n| r_string(n)))
                .collect::<Vec<_>>()
                .join(", ");
            let last = xs.len() + 1;
            let body = if student {
                format!(
                    "{{\n    X <- cbind(1, as.matrix(d[, 2:{last}, drop = FALSE]))\n    \
                     fit <- stats::lm.fit(X, d[[1]])\n    \
                     vb <- sum(fit$residuals^2) / (nrow(X) - ncol(X)) * diag(solve(crossprod(X)))\n    \
                     nm <- c({nm})\n    \
                     stats::setNames(c(fit$coefficients, vb), c(nm, paste(nm, \"variance\")))\n  }}"
                )
            } else {
                format!(
                    "stats::setNames(stats::lm.fit(cbind(1, as.matrix(d[, 2:{last}, drop = FALSE])), \
                     d[[1]])$coefficients, c({nm}))"
                )
            };
            (body, String::new())
        }
        "custom" => (
            format!(
                "{{\n    r <- with(d, {{\n      {}\n    }})\n    \
                 if (!is.numeric(r) || length(r) == 0) \
                 stop(\"Your statistic must give one or more numbers.\")\n    r\n  }}",
                a.expr("expr")
            ),
            String::new(),
        ),
        _ => {
            let body = if student {
                "{\n    x <- d[[1]]\n    c(mean(x), stats::var(x) / length(x))\n  }".to_string()
            } else {
                "mean(d[[1]])".to_string()
            };
            (body, format!("mean of {v}"))
        }
    }
}

fn bootstrap_code(a: &Args) -> String {
    let m = a.name("save");
    let kind = a.choice("statistic");
    let v = a.var_text("v");
    let mut xs: Vec<&str> = Vec::new();
    if matches!(kind, "correlation" | "coefficient" | "custom") {
        xs = a.vars("x");
    }
    let (cluster, strata) = (a.var_text("cluster"), a.var_text("strata"));
    let mut cols: Vec<&str> = vec![v];
    cols.extend(xs.iter().copied());
    for extra in [cluster, strata] {
        if !extra.is_empty() && !cols.contains(&extra) {
            cols.push(extra);
        }
    }
    let names = cols.iter().map(|n| r_string(n)).collect::<Vec<_>>();
    let (stat, label) = bootstrap_statistic(a, &xs);
    let reps = a.number("reps");
    let seed = a.number("seed");
    let draw = if cluster.is_empty() {
        let st = if strata.is_empty() {
            String::new()
        } else {
            format!(", strata = as.integer(factor(d[[{}]]))", r_string(strata))
        };
        format!(
            "set.seed({seed})\n  boot::boot(d, function(d, i) f(d[i, , drop = FALSE]), R = {reps}{st})"
        )
    } else {
        let (setup, st) = if strata.is_empty() {
            (String::new(), "")
        } else {
            (
                format!(
                    "st <- as.integer(factor(d[[{}]]))\n  \
                     if (any(tapply(st, cl, function(z) length(unique(z))) > 1)) \
                     stop(\"Each cluster must lie within one stratum.\")\n  \
                     st <- st[match(levels(cl), as.character(cl))]\n  ",
                    r_string(strata)
                ),
                ", strata = st",
            )
        };
        format!(
            "cl <- factor(d[[{}]])\n  \
             if (nlevels(cl) < 2) stop(\"A cluster bootstrap needs at least two clusters.\")\n  \
             rows <- split(seq_len(nrow(d)), cl)\n  {setup}set.seed({seed})\n  \
             boot::boot(data.frame(cluster = levels(cl)), function(k, i) \
             f(d[unlist(rows[i], use.names = FALSE), , drop = FALSE]), R = {reps}{st})",
            r_string(cluster)
        )
    };
    let mut notes = String::new();
    if !cluster.is_empty() {
        notes.push_str(&format!(
            "cat(\"Clusters of \", {}, \" drawn, \", nrow({m}$data), \" in all\\n\", sep = \"\")\n",
            r_string(cluster)
        ));
    }
    if !strata.is_empty() {
        notes.push_str(&format!(
            "cat(\"Drawn within the \", length(unique({m}$strata)), \" strata of \", {}, \"\\n\", \
             sep = \"\")\n",
            r_string(strata)
        ));
    }
    let naming = if label.is_empty() {
        "  if (is.null(names(est)) || any(is.na(names(est)) | names(est) == \"\")) \
         names(est) <- if (k == 1) \"statistic\" else paste(\"statistic\", seq_len(k))\n"
            .to_string()
    } else {
        format!("  names(est) <- {}\n", r_string(&label))
    };
    let half = if a.flag("student") { " / 2" } else { "" };
    let level = a.number("level");
    let types = BOOT_TYPES
        .iter()
        .filter(|(flag, _, _)| a.flag(flag))
        .map(|(_, ty, _)| r_string(ty))
        .collect::<Vec<_>>();
    let mut intervals = String::new();
    if !types.is_empty() {
        intervals.push_str(&format!(
            "  for (ty in c({})) {{\n    \
             lim <- t(sapply(seq_len(k), function(j) {{\n      \
             r <- tryCatch(boot::boot.ci({m}, conf = {level} / 100, type = ty, \
             index = if (ty == \"stud\") c(j, k + j) else j), error = function(e) NULL)\n      \
             if (is.null(r)) return(c(NA_real_, NA_real_))\n      \
             switch(ty, norm = r$normal[2:3], perc = r$percent[4:5], bca = r$bca[4:5], \
             basic = r$basic[4:5], stud = r$student[4:5])\n    }}))\n    \
             title <- c(norm = \"Normal\", perc = \"Percentile\", bca = \"BCa\", basic = \"Basic\", \
             stud = \"Studentised\")[[ty]]\n    \
             out[[paste(title, \"lower\")]] <- lim[, 1]\n    \
             out[[paste(title, \"upper\")]] <- lim[, 2]\n  }}\n",
            types.join(", ")
        ));
    }
    if a.flag("stata") {
        intervals.push_str(&format!(
            "  z <- stats::qnorm((1 + {level} / 100) / 2)\n  \
             pct <- function(x, p) {{\n    x <- sort(x)\n    n <- length(x)\n    \
             vapply(p, function(q) {{\n      h <- q * n\n      \
             if (abs(h - round(h)) < 1e-9 * max(1, h)) {{\n        \
             h <- round(h)\n        (x[min(n, max(1, h))] + x[min(n, h + 1)]) / 2\n      \
             }} else x[min(n, max(1, ceiling(h)))]\n    }}, 0)\n  }}\n  \
             lim <- t(sapply(seq_len(k), function(j) {{\n    \
             tj <- tt[!is.na(tt[, j]), j]\n    th <- unname(est[j])\n    se <- stats::sd(tj)\n    \
             z0 <- stats::qnorm(mean(tj <= th))\n    \
             c(th - z * se, th + z * se, pct(tj, (1 + c(-1, 1) * {level} / 100) / 2), \
             pct(tj, stats::pnorm(2 * z0 + c(-z, z))))\n  }}))\n  \
             for (h in 1:3) {{\n    \
             title <- c(\"Stata normal\", \"Stata percentile\", \"Stata BC\")[h]\n    \
             out[[paste(title, \"lower\")]] <- lim[, 2 * h - 1]\n    \
             out[[paste(title, \"upper\")]] <- lim[, 2 * h]\n  }}\n"
        ));
    }
    format!(
        "{m} <- local({{\n  d <- stats::na.omit({}[, c({}), drop = FALSE])\n  \
         f <- function(d) {stat}\n  {draw}\n}})\n\
         cat(sprintf(\"Bootstrap: %d replications, seed {seed}\\n\", nrow({m}$t)))\n{notes}\
         {m}$intervals <- local({{\n  k <- length({m}$t0){half}\n  est <- {m}$t0[seq_len(k)]\n\
         {naming}  tt <- {m}$t[, seq_len(k), drop = FALSE]\n  \
         out <- data.frame(Estimate = unname(est), \
         Bias = unname(colMeans(tt, na.rm = TRUE) - est), \
         Std.Err = unname(apply(tt, 2, stats::sd, na.rm = TRUE)), \
         row.names = names(est))\n{intervals}  out\n}})\nprint({m}$intervals)",
        a.data(),
        names.join(", "),
    )
}

pub const BOOTSTRAP: Spec = Spec {
    entry: "Bootstrap",
    menu: "Statistics > Resampling > Bootstrap",
    about:
        "Estimates how much a statistic varies from sample to sample by drawing the rows again, \
            with replacement, many times (boot::boot) and recomputing the statistic each time. \
            With strata the rows are drawn within each stratum, as many as it has; with clusters \
            whole clusters are drawn (within the strata, when both are given) and every row of a \
            drawn cluster comes with it. It prints the estimate, its bias and bootstrap standard \
            error, and the confidence intervals asked for, from boot::boot.ci: the normal \
            interval (the estimate corrected for bias, plus or minus z standard errors), the \
            percentile interval, the BCa interval, the basic interval (twice the estimate less \
            the upper and lower percentile limits) and the studentised interval (each replicate \
            divided by its own standard error: the mean's sd / sqrt(n), or a coefficient's \
            ordinary least-squares error). Stata's intervals can be added: its normal interval is \
            the estimate (not corrected for bias) plus or minus z standard errors, its percentile \
            and bias-corrected limits are its own percentiles of the replicates (the average of \
            two when the position falls exactly on one), so they can differ from boot's by a \
            replicate. A statistic you write is an R expression in the columns listed under \
            Variable and the second list, for example `sd(v) / mean(v)`; it may give several \
            numbers. The same seed draws the same rows. An interval that cannot be worked out is \
            shown as NA. The fitted object is `name`, with the replicates in `name$t` and the \
            intervals in `name$intervals`. Rows with a missing value are left out.",
    widgets: &[
        Widget::Choice {
            id: "statistic",
            label: "Statistic",
            options: STATISTICS,
            default: "mean",
            hint: "",
        },
        Widget::Variable {
            id: "v",
            label: "Variable",
            kinds: NUMBERS,
            required: true,
            hint:
                "The variable; for a correlation, the first one; for a linear model, the outcome.",
        },
        Widget::Variables {
            id: "x",
            label: "Second variable, predictors or other columns",
            kinds: NUMBERS,
            min: 0,
            hint: "One variable for a correlation; the predictors for a linear model; any other \
                   columns your statistic uses. Not used for a mean or a median.",
        },
        Widget::Text {
            id: "expr",
            label: "Your statistic (R)",
            default: TextDefault::Empty,
            rule: TextRule::Any,
            hint: "Only for a statistic you write: an R expression of the columns chosen above, \
                   such as sd(v) / mean(v) or quantile(v, c(0.25, 0.75)).",
        },
        Widget::Variable {
            id: "cluster",
            label: "Draw whole clusters of",
            kinds: GROUPS,
            required: false,
            hint: "Optional: each cluster is drawn whole, as Stata's cluster() does.",
        },
        Widget::Variable {
            id: "strata",
            label: "Draw within strata of",
            kinds: GROUPS,
            required: false,
            hint: "Optional: each stratum keeps its number of rows (or of clusters).",
        },
        Widget::Text {
            id: "reps",
            label: "Replications",
            default: TextDefault::Fixed("999"),
            rule: TextRule::Number,
            hint: "100 or more. A number one less than a round number (999, 4999) makes the \
                   percentile limits fall exactly on a replicate.",
        },
        Widget::Text {
            id: "seed",
            label: "Seed",
            default: TextDefault::Fixed("1"),
            rule: TextRule::Number,
            hint: "The same seed draws the same rows, so the same intervals.",
        },
        Widget::Text {
            id: "level",
            label: "Confidence level (%)",
            default: TextDefault::Fixed("95"),
            rule: TextRule::Number,
            hint: "",
        },
        Widget::Check {
            id: "normal",
            label: "Normal interval",
            default: true,
            hint: "",
        },
        Widget::Check {
            id: "percentile",
            label: "Percentile interval",
            default: true,
            hint: "",
        },
        Widget::Check {
            id: "bca",
            label: "BCa interval (bias-corrected and accelerated)",
            default: true,
            hint: "",
        },
        Widget::Check {
            id: "basic",
            label: "Basic interval",
            default: false,
            hint: "",
        },
        Widget::Check {
            id: "student",
            label: "Studentised interval",
            default: false,
            hint: "For a mean or the coefficients of a linear model, without clusters.",
        },
        Widget::Check {
            id: "stata",
            label: "Stata's normal, percentile and bias-corrected intervals",
            default: false,
            hint: "As Stata's estat bootstrap prints them from the same replicates.",
        },
        Widget::Text {
            id: "save",
            label: "Save the bootstrap as",
            default: TextDefault::Fixed("boot_fit"),
            rule: TextRule::Name,
            hint: "The result stays in R's workspace under this name; a name already in use is \
                   replaced.",
        },
    ],
    needs: &["boot"],
    styles: BASE,
    check: bootstrap_check,
    code: |a, _| bootstrap_code(a),
};

// ---- Permutation test ----

const PERMUTATION_STATISTICS: &[Opt] = &[
    Opt {
        value: "means",
        label: "Difference in means between two groups",
    },
    Opt {
        value: "medians",
        label: "Difference in medians between two groups",
    },
    Opt {
        value: "correlation",
        label: "Correlation of two variables",
    },
    Opt {
        value: "coefficient",
        label: "Coefficient of a predictor in a linear model",
    },
];

const ALTERNATIVES: &[Opt] = &[
    Opt {
        value: "two",
        label: "Two-sided (as large in size)",
    },
    Opt {
        value: "greater",
        label: "One-sided: as large or larger",
    },
    Opt {
        value: "less",
        label: "One-sided: as small or smaller",
    },
    Opt {
        value: "stata",
        label: "Two-sided as Stata (twice the smaller one-sided)",
    },
];

fn permutation_check(a: &Args) -> Vec<Problem> {
    let mut out = Vec::new();
    whole(a, "reps", "The number of permutations", 1, &mut out);
    whole(a, "seed", "The seed", 0, &mut out);
    let (v, w) = (a.var_text("v"), a.var_text("w"));
    if v == w && !v.is_empty() {
        out.push(Problem::new(
            "w",
            "Choose a different variable from the first one.",
        ));
    }
    let z = a.vars("z");
    if a.choice("statistic") == "coefficient" && (z.contains(&v) || z.contains(&w)) {
        out.push(Problem::new(
            "z",
            "The other predictors cannot include the outcome or the permuted predictor.",
        ));
    }
    let s = a.var_text("strata");
    if !s.is_empty()
        && (s == v || s == w || (a.choice("statistic") == "coefficient" && z.contains(&s)))
    {
        out.push(Problem::new(
            "strata",
            "The strata must be a variable not otherwise used in the test.",
        ));
    }
    not_the_data(a, &mut out);
    out
}

/// The R that lists or shuffles the arrangements of the rows' positions `p`
/// (for a correlation or a coefficient), within strata when `strata`.
fn shuffle_positions(strata: bool, reps: &str, seed: &str) -> String {
    let perms = "perms <- function(k) {\n    if (k == 1) return(matrix(1L, 1, 1))\n    \
                 q <- perms(k - 1)\n    \
                 do.call(rbind, lapply(seq_len(k), function(i) \
                 cbind(i, matrix(ifelse(q >= i, q + 1L, q), nrow(q)))))\n  }\n  ";
    if strata {
        format!(
            "{perms}sizes <- lengths(units)\n  \
             arrangements <- prod(factorial(pmin(sizes, 171)))\n  \
             exact <- sum(lfactorial(sizes)) <= log({reps})\n  \
             if (exact) {{\n    \
             parts <- lapply(units, function(u) {{ q <- perms(length(u)); matrix(u[q], nrow(q)) }})\n    \
             grid <- as.matrix(expand.grid(lapply(parts, function(q) seq_len(nrow(q)))))\n    \
             t <- apply(grid, 1, function(r) {{\n      p <- seq_len(n)\n      \
             for (h in seq_along(parts)) p[units[[h]]] <- parts[[h]][r[[h]], ]\n      stat(p)\n    }})\n  \
             }} else {{\n    set.seed({seed})\n    \
             t <- replicate({reps}, {{\n      p <- seq_len(n)\n      \
             for (u in units) p[u] <- u[sample.int(length(u))]\n      stat(p)\n    }})\n  }}"
        )
    } else {
        format!(
            "arrangements <- if (n <= 170) factorial(n) else Inf\n  \
             exact <- lfactorial(n) <= log({reps})\n  \
             if (exact) {{\n    {}\n    t <- apply(perms(n), 1, stat)\n  }} else {{\n    \
             set.seed({seed})\n    t <- replicate({reps}, stat(sample.int(n)))\n  }}",
            perms.trim_end().replace("\n  ", "\n    ")
        )
    }
}

fn permutation_code(a: &Args) -> String {
    let m = a.name("save");
    let reps = a.number("reps");
    let seed = a.number("seed");
    let kind = a.choice("statistic");
    let strata = a.var_text("strata");
    let zs = if kind == "coefficient" {
        a.vars("z")
    } else {
        Vec::new()
    };
    let mut cols = vec![a.var_text("v"), a.var_text("w")];
    cols.extend(zs.iter().copied());
    if !strata.is_empty() {
        cols.push(strata);
    }
    let cols = cols
        .iter()
        .map(|n| r_string(n))
        .collect::<Vec<_>>()
        .join(", ");
    let take = format!(
        "d <- stats::na.omit({}[, c({cols}), drop = FALSE])\n  n <- nrow(d)\n  y <- d[[1]]\n  ",
        a.data()
    );
    let units = if strata.is_empty() {
        "nstrata <- 1L\n  ".to_string()
    } else {
        format!(
            "units <- split(seq_len(n), factor(d[[{}]]))\n  nstrata <- length(units)\n  ",
            r_string(strata)
        )
    };
    let body = if kind == "correlation" || kind == "coefficient" {
        let stat = if kind == "coefficient" {
            format!(
                "if (!is.numeric(d[[2]])) stop(\"The permuted predictor must be numeric.\")\n  \
                 Z <- as.matrix(d[, seq_len({}) + 2L, drop = FALSE])\n  \
                 if (n < ncol(Z) + 3) stop(\"A regression coefficient needs more rows than coefficients.\")\n  \
                 x <- d[[2]]\n  \
                 stat <- function(p) stats::lm.fit(cbind(1, x[p], Z), y)$coefficients[[2]]\n  ",
                zs.len()
            )
        } else {
            "if (!is.numeric(d[[2]])) stop(\"A correlation needs a numeric second variable.\")\n  \
             if (n < 3) stop(\"A correlation needs at least three rows.\")\n  \
             x <- d[[2]]\n  stat <- function(p) stats::cor(x, y[p])\n  "
                .to_string()
        };
        format!(
            "{take}{stat}obs <- stat(seq_len(n))\n  {units}{}",
            shuffle_positions(!strata.is_empty(), reps, seed)
        )
    } else {
        let f = if kind == "medians" {
            "stats::median"
        } else {
            "mean"
        };
        let shuffle = if strata.is_empty() {
            format!(
                "arrangements <- choose(n, sum(second))\n  \
                 exact <- arrangements <= {reps}\n  \
                 if (exact) {{\n    \
                 t <- utils::combn(n, sum(second), function(ix) {{\n      \
                 s <- logical(n)\n      s[ix] <- TRUE\n      stat(s)\n    }})\n  }} else {{\n    \
                 set.seed({seed})\n    t <- replicate({reps}, stat(sample(second)))\n  }}"
            )
        } else {
            format!(
                "arrangements <- prod(vapply(units, function(u) choose(length(u), sum(second[u])), 0))\n  \
                 exact <- arrangements <= {reps}\n  \
                 if (exact) {{\n    \
                 parts <- lapply(units, function(u) {{\n      k <- sum(second[u])\n      \
                 if (k == 0) list(integer(0)) else \
                 utils::combn(length(u), k, function(ix) u[ix], simplify = FALSE)\n    }})\n    \
                 grid <- as.matrix(expand.grid(lapply(parts, seq_along)))\n    \
                 t <- apply(grid, 1, function(r) {{\n      s <- logical(n)\n      \
                 for (h in seq_along(parts)) s[parts[[h]][[r[[h]]]]] <- TRUE\n      stat(s)\n    }})\n  \
                 }} else {{\n    set.seed({seed})\n    \
                 t <- replicate({reps}, {{\n      s <- second\n      \
                 for (u in units) s[u] <- s[u][sample.int(length(u))]\n      stat(s)\n    }})\n  }}"
            )
        };
        format!(
            "{take}g <- factor(d[[2]])\n  \
             if (nlevels(g) != 2) stop(\"The group variable must have exactly two groups among the rows used; it has \", nlevels(g), \".\")\n  \
             second <- g == levels(g)[2]\n  \
             stat <- function(s) {f}(y[s]) - {f}(y[!s])\n  \
             obs <- stat(second)\n  {units}{shuffle}"
        )
    };
    let (pexpr, title) = match a.choice("alternative") {
        "greater" => (
            "pp(sum(t >= obs - tol))",
            "One-sided p-value (as large or larger)",
        ),
        "less" => (
            "pp(sum(t <= obs + tol))",
            "One-sided p-value (as small or smaller)",
        ),
        "stata" => (
            "min(1, 2 * min(pp(sum(t >= obs - tol)), pp(sum(t <= obs + tol))))",
            "Two-sided p-value (twice the smaller one-sided)",
        ),
        _ => ("pp(sum(abs(t) >= abs(obs) - tol))", "Two-sided p-value"),
    };
    let within = if strata.is_empty() {
        String::new()
    } else {
        format!(
            "cat(\"Shuffled within the \", {m}$strata, \" strata of \", {}, \"\\n\", sep = \"\")\n",
            r_string(strata)
        )
    };
    format!(
        "{m} <- local({{\n  {body}\n  tol <- 1e-9 * max(1, abs(obs))\n  \
         pp <- function(count) if (exact) count / length(t) else (count + 1) / (length(t) + 1)\n  \
         p <- {pexpr}\n  \
         list(statistic = obs, p.value = p, alternative = {}, exact = exact, \
         arrangements = arrangements, permutations = length(t), strata = nstrata, replicates = t)\n}})\n\
         cat(sprintf(\"Observed statistic: %.6g\\n\", {m}$statistic))\n{within}\
         cat(sprintf(if ({m}$exact) \"All %.0f arrangements listed (exact)\\n\" \
         else \"%d random permutations of %.4g arrangements, seed {seed}\\n\", \
         if ({m}$exact) {m}$arrangements else {m}$permutations, {m}$arrangements))\n\
         cat(sprintf(\"{title}: %.6g\\n\", {m}$p.value))\n\
         if (!{m}$exact) cat(sprintf(\"Monte Carlo standard error: %.4g\\n\", \
         sqrt({m}$p.value * (1 - {m}$p.value) / {m}$permutations)))",
        r_string(a.choice("alternative")),
    )
}

pub const PERMUTATION_TEST: Spec = Spec {
    entry: "Permutation test",
    menu: "Statistics > Resampling > Permutation test",
    about: "Asks how often shuffling the data would give a statistic as far from zero as the one \
            observed. For two groups the rows' group labels are shuffled; for a correlation one \
            variable is shuffled against the other; for a regression coefficient the predictor \
            is shuffled against the outcome and the other predictors, and the model refitted (as \
            Stata's permute x _b[x]: regress y x z does). With strata the shuffles stay within \
            each stratum. The two-sided p-value counts the shuffles whose statistic is at least \
            as large in size as the observed one; a one-sided p-value counts those at least as \
            large (or at least as small). Over B random shuffles with a seed it is (count + 1) / \
            (B + 1), the observed arrangement counting as one; when the arrangements number no \
            more than B, every one is listed instead and the p-value is exact: count / \
            arrangements. Stata's permute gives count / B (the two agree within the Monte Carlo \
            error), and its two-sided p-value is twice the smaller one-sided one, which can be \
            chosen here. The difference is the second group's statistic minus the first's (groups in \
            alphabetical order). Rows with a missing value are left out.",
    widgets: &[
        Widget::Choice {
            id: "statistic",
            label: "Statistic",
            options: PERMUTATION_STATISTICS,
            default: "means",
            hint: "",
        },
        Widget::Variable {
            id: "v",
            label: "Variable",
            kinds: NUMBERS,
            required: true,
            hint: "The numeric variable compared between the groups, the first of the two \
                   correlated, or the outcome of the linear model.",
        },
        Widget::Variable {
            id: "w",
            label: "Group variable, second variable or permuted predictor",
            kinds: GROUPS,
            required: true,
            hint: "A variable with exactly two groups; for a correlation, a numeric variable; for \
                   a coefficient, the numeric predictor whose coefficient is tested.",
        },
        Widget::Variables {
            id: "z",
            label: "Other predictors (kept in place)",
            kinds: NUMBERS,
            min: 0,
            hint: "Only for a regression coefficient: the model's other predictors.",
        },
        Widget::Variable {
            id: "strata",
            label: "Shuffle within strata of",
            kinds: GROUPS,
            required: false,
            hint: "Optional: the shuffles stay inside each stratum, as Stata's strata() does.",
        },
        Widget::Choice {
            id: "alternative",
            label: "p-value",
            options: ALTERNATIVES,
            default: "two",
            hint: "",
        },
        Widget::Text {
            id: "reps",
            label: "Permutations (B)",
            default: TextDefault::Fixed("50000"),
            rule: TextRule::Number,
            hint: "Random shuffles. Used only when the arrangements are more than this many.",
        },
        Widget::Text {
            id: "seed",
            label: "Seed",
            default: TextDefault::Fixed("1"),
            rule: TextRule::Number,
            hint: "The same seed shuffles the same way.",
        },
        Widget::Text {
            id: "save",
            label: "Save the result as",
            default: TextDefault::Fixed("perm_test"),
            rule: TextRule::Name,
            hint: "A list with the observed statistic, the p-value and the permuted statistics; a \
                   name already in use is replaced.",
        },
    ],
    needs: &[],
    styles: BASE,
    check: permutation_check,
    code: |a, _| permutation_code(a),
};

pub const SPECS: &[&Spec] = &[&BOOTSTRAP, &PERMUTATION_TEST];

#[cfg(test)]
mod tests {
    use super::*;
    use rcraft_rcodegen::dialog::{ColumnInfo, Context, Style, Values};

    fn ctx() -> Context {
        let col = |name: &str, kind| ColumnInfo {
            name: name.into(),
            kind,
        };
        Context {
            dataset: "d".into(),
            frames: Vec::new(),
            columns: vec![
                col("height", VarKind::Number),
                col("arm span", VarKind::Number),
                col("age", VarKind::Number),
                col("species", VarKind::Factor),
            ],
        }
    }

    fn values(spec: &Spec, set: &[(&str, &str)], lists: &[(&str, &[&str])]) -> Values {
        let mut v = spec.defaults(&ctx());
        for (id, text) in set {
            v.set_text(id, *text);
        }
        for (id, names) in lists {
            v.set_list(id, names.iter().map(|n| n.to_string()).collect());
        }
        v
    }

    fn code(spec: &Spec, v: &Values) -> String {
        let out = spec.generate(&ctx(), v, Style::Base);
        assert!(out.is_ok(), "{out:?}");
        out.unwrap_or_default()
    }

    fn widgets(spec: &Spec, v: &Values) -> Vec<&'static str> {
        spec.problems(&ctx(), v).iter().map(|p| p.widget).collect()
    }

    #[test]
    fn bootstrap_writes_boot_and_boot_ci() {
        let v = values(&BOOTSTRAP, &[("v", "height")], &[]);
        let c = code(&BOOTSTRAP, &v);
        assert!(
            c.starts_with(
                "boot_fit <- local({\n  d <- stats::na.omit(d[, c(\"height\"), drop = FALSE])\n  \
                 f <- function(d) mean(d[[1]])\n  set.seed(1)\n  \
                 boot::boot(d, function(d, i) f(d[i, , drop = FALSE]), R = 999)\n})\n"
            ),
            "{c}"
        );
        assert!(c.contains("boot::boot.ci(boot_fit, conf = 95 / 100"), "{c}");
        assert!(c.ends_with("print(boot_fit$intervals)"), "{c}");
        // a correlation takes the second variable; a name needing backticks is quoted
        let mut v = values(
            &BOOTSTRAP,
            &[
                ("statistic", "correlation"),
                ("v", "height"),
                ("seed", "42"),
            ],
            &[("x", &["arm span"])],
        );
        v.set_flag("bca", false);
        let c = code(&BOOTSTRAP, &v);
        assert!(c.contains("c(\"height\", \"arm span\")"), "{c}");
        assert!(c.contains("stats::cor(d[[1]], d[[2]])"), "{c}");
        assert!(c.contains("set.seed(42)"), "{c}");
        assert!(!c.contains("\"bca\""), "{c}");
        // a linear model needs predictors
        let v = values(
            &BOOTSTRAP,
            &[("statistic", "coefficient"), ("v", "height")],
            &[],
        );
        assert_eq!(widgets(&BOOTSTRAP, &v), vec!["x"]);
        let v = values(
            &BOOTSTRAP,
            &[("statistic", "coefficient"), ("v", "height")],
            &[("x", &["age", "arm span"])],
        );
        assert!(code(&BOOTSTRAP, &v).contains("stats::lm.fit("));
        // too few replications, a bad level, no interval, the data's own name
        let mut v = values(
            &BOOTSTRAP,
            &[
                ("v", "height"),
                ("reps", "50"),
                ("level", "100"),
                ("save", "d"),
            ],
            &[],
        );
        v.set_flag("normal", false);
        v.set_flag("percentile", false);
        v.set_flag("bca", false);
        assert_eq!(
            widgets(&BOOTSTRAP, &v),
            vec!["reps", "level", "normal", "save"]
        );
        // clusters within strata; a statistic of one's own; the new intervals
        let mut v = values(
            &BOOTSTRAP,
            &[
                ("statistic", "custom"),
                ("v", "height"),
                ("expr", "sd(height) / mean(height)"),
                ("cluster", "species"),
                ("strata", "age"),
            ],
            &[],
        );
        v.set_flag("basic", true);
        v.set_flag("stata", true);
        let c = code(&BOOTSTRAP, &v);
        assert!(
            c.contains("c(\"height\", \"species\", \"age\")")
                && c.contains("r <- with(d, {\n      sd(height) / mean(height)\n    })")
                && c.contains("cl <- factor(d[[\"species\"]])")
                && c.contains("Each cluster must lie within one stratum.")
                && c.contains("boot::boot(data.frame(cluster = levels(cl))")
                && c.contains("R = 999, strata = st)")
                && c.contains("\"basic\"")
                && c.contains("Stata BC")
                && c.contains("pct <- function(x, p)"),
            "{c}"
        );
        // strata of rows only
        let v = values(&BOOTSTRAP, &[("v", "height"), ("strata", "species")], &[]);
        assert!(code(&BOOTSTRAP, &v)
            .contains("R = 999, strata = as.integer(factor(d[[\"species\"]])))"));
        // studentised: a mean carries its variance, a median is refused, and
        // so are clusters; an unbalanced statistic and the same strata and
        // clusters are refused
        let mut v = values(&BOOTSTRAP, &[("v", "height")], &[]);
        v.set_flag("student", true);
        let c = code(&BOOTSTRAP, &v);
        assert!(
            c.contains("c(mean(x), stats::var(x) / length(x))")
                && c.contains("k <- length(boot_fit$t0) / 2"),
            "{c}"
        );
        let mut v = values(&BOOTSTRAP, &[("statistic", "median"), ("v", "height")], &[]);
        v.set_flag("student", true);
        assert_eq!(widgets(&BOOTSTRAP, &v), vec!["student"]);
        let mut v = values(&BOOTSTRAP, &[("v", "height"), ("cluster", "species")], &[]);
        v.set_flag("student", true);
        assert_eq!(widgets(&BOOTSTRAP, &v), vec!["student"]);
        let v = values(
            &BOOTSTRAP,
            &[
                ("statistic", "custom"),
                ("v", "height"),
                ("expr", "mean(height"),
                ("cluster", "species"),
                ("strata", "species"),
            ],
            &[],
        );
        assert_eq!(widgets(&BOOTSTRAP, &v), vec!["expr", "strata"]);
        let v = values(&BOOTSTRAP, &[("statistic", "custom"), ("v", "height")], &[]);
        assert_eq!(widgets(&BOOTSTRAP, &v), vec!["expr"]);
    }

    #[test]
    fn permutation_test_writes_the_shuffles() {
        let v = values(&PERMUTATION_TEST, &[("v", "height"), ("w", "species")], &[]);
        let c = code(&PERMUTATION_TEST, &v);
        assert!(
            c.starts_with(
                "perm_test <- local({\n  d <- stats::na.omit(d[, c(\"height\", \"species\")"
            ),
            "{c}"
        );
        assert!(
            c.contains("stat <- function(s) mean(y[s]) - mean(y[!s])"),
            "{c}"
        );
        assert!(c.contains("exact <- arrangements <= 50000"), "{c}");
        assert!(c.contains("replicate(50000, stat(sample(second)))"), "{c}");
        assert!(c.contains("(count + 1) / (length(t) + 1)"), "{c}");
        let v = values(
            &PERMUTATION_TEST,
            &[
                ("statistic", "medians"),
                ("v", "height"),
                ("w", "species"),
                ("seed", "5"),
            ],
            &[],
        );
        let c = code(&PERMUTATION_TEST, &v);
        assert!(
            c.contains("stats::median(y[s])") && c.contains("set.seed(5)"),
            "{c}"
        );
        let v = values(
            &PERMUTATION_TEST,
            &[("statistic", "correlation"), ("v", "height"), ("w", "age")],
            &[],
        );
        let c = code(&PERMUTATION_TEST, &v);
        assert!(
            c.contains("stats::cor(x, y[p])") && c.contains("sample.int(n)"),
            "{c}"
        );
        // the same variable twice, no permutations, the data's own name
        let v = values(
            &PERMUTATION_TEST,
            &[
                ("v", "height"),
                ("w", "height"),
                ("reps", "0"),
                ("save", "d"),
            ],
            &[],
        );
        assert_eq!(widgets(&PERMUTATION_TEST, &v), vec!["reps", "w", "save"]);
        // within strata, one-sided
        let v = values(
            &PERMUTATION_TEST,
            &[
                ("v", "height"),
                ("w", "species"),
                ("strata", "age"),
                ("alternative", "greater"),
            ],
            &[],
        );
        let c = code(&PERMUTATION_TEST, &v);
        assert!(
            c.contains("units <- split(seq_len(n), factor(d[[\"age\"]]))")
                && c.contains("for (u in units) s[u] <- s[u][sample.int(length(u))]")
                && c.contains("p <- pp(sum(t >= obs - tol))")
                && c.contains("One-sided p-value (as large or larger)"),
            "{c}"
        );
        // a regression coefficient, with another predictor
        let v = values(
            &PERMUTATION_TEST,
            &[
                ("statistic", "coefficient"),
                ("v", "height"),
                ("w", "age"),
                ("alternative", "less"),
            ],
            &[("z", &["arm span"])],
        );
        let c = code(&PERMUTATION_TEST, &v);
        assert!(
            c.contains("c(\"height\", \"age\", \"arm span\")")
                && c.contains("stats::lm.fit(cbind(1, x[p], Z), y)$coefficients[[2]]")
                && c.contains("p <- pp(sum(t <= obs + tol))"),
            "{c}"
        );
        // the strata cannot be a variable of the test
        let v = values(
            &PERMUTATION_TEST,
            &[
                ("statistic", "coefficient"),
                ("v", "height"),
                ("w", "age"),
                ("strata", "arm span"),
            ],
            &[("z", &["arm span", "height"])],
        );
        assert_eq!(widgets(&PERMUTATION_TEST, &v), vec!["z", "strata"]);
    }
}
