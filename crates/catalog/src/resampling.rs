//! Statistics menu, resampling (P6.8): the bootstrap, with `boot::boot` and the
//! normal, percentile and BCa intervals of `boot::boot.ci`, and the permutation
//! test, written in base R: the labels of the data are shuffled with `sample`,
//! or every arrangement is listed when there are no more of them than the
//! number of shuffles asked for. Neither changes the data. Both keep their
//! result in the workspace under the name the dialog asks for.

use rcraft_rcodegen::dialog::{Args, Opt, Problem, Spec, TextDefault, TextRule};
use rcraft_rcodegen::dialog::{VarKind, Widget};
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
    match a.choice("statistic") {
        "correlation" if x.len() != 1 => out.push(Problem::new(
            "x",
            "A correlation needs exactly one second variable.",
        )),
        "coefficient" if x.is_empty() => {
            out.push(Problem::new("x", "Choose at least one predictor."));
        }
        _ => {}
    }
    if a.choice("statistic") != "mean" && a.choice("statistic") != "median" && x.contains(&v) {
        out.push(Problem::new(
            "x",
            format!("{v} is the first variable; it cannot also be in this list."),
        ));
    }
    if !(a.flag("normal") || a.flag("percentile") || a.flag("bca")) {
        out.push(Problem::new(
            "normal",
            "Choose at least one kind of interval.",
        ));
    }
    not_the_data(a, &mut out);
    out
}

pub const BOOTSTRAP: Spec = Spec {
    entry: "Bootstrap",
    menu: "Statistics > Resampling > Bootstrap",
    about: "Estimates how much a statistic varies from sample to sample by drawing the rows again, \
            with replacement, many times (boot::boot) and recomputing the statistic each time. It \
            prints the estimate, its bias and bootstrap standard error, and the confidence \
            intervals asked for: the normal interval (the estimate corrected for bias, plus or \
            minus z standard errors), the percentile interval and the BCa interval \
            (boot::boot.ci). The same seed draws the same rows. With 999 replications the \
            percentile limits are the 25th smallest and the 25th largest replicate at 95%. An \
            interval that cannot be worked out (a BCa interval needs more variation than a very \
            small sample gives) is shown as NA. The fitted object is `name`, with the replicates in \
            `name$t` and the intervals in `name$intervals`. Rows with a missing value are left out.",
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
            hint: "The variable; for a correlation, the first one; for a linear model, the outcome.",
        },
        Widget::Variables {
            id: "x",
            label: "Second variable or predictors",
            kinds: NUMBERS,
            min: 0,
            hint: "One variable for a correlation; the predictors for a linear model. Not used for \
                   a mean or a median.",
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
    code: |a, _| {
        let m = a.name("save");
        let xs = a.vars("x");
        let kind = a.choice("statistic");
        let mut names = vec![r_string(a.var_text("v"))];
        if kind == "correlation" || kind == "coefficient" {
            names.extend(xs.iter().map(|n| r_string(n)));
        }
        let (stat, label) = match kind {
            "median" => (
                "stats::median(d[[1]][i])".to_string(),
                format!("median of {}", a.var_text("v")),
            ),
            "correlation" => (
                "stats::cor(d[[1]][i], d[[2]][i])".to_string(),
                format!(
                    "correlation of {} and {}",
                    a.var_text("v"),
                    xs.first().cloned().unwrap_or_default()
                ),
            ),
            "coefficient" => (
                "stats::setNames(stats::lm.fit(cbind(1, as.matrix(d[i, -1, drop = FALSE])), \
                 d[[1]][i])$coefficients, c(\"(Intercept)\", names(d)[-1]))"
                    .to_string(),
                String::new(),
            ),
            _ => (
                "mean(d[[1]][i])".to_string(),
                format!("mean of {}", a.var_text("v")),
            ),
        };
        let mut columns = Vec::new();
        let mut types = Vec::new();
        for (flag, key, title) in [
            ("normal", "norm", "Normal"),
            ("percentile", "perc", "Percentile"),
            ("bca", "bca", "BCa"),
        ] {
            if a.flag(flag) {
                types.push(r_string(key));
                columns.push(r_string(title));
            }
        }
        let level = a.number("level");
        let naming = if label.is_empty() {
            String::new()
        } else {
            format!("  names(est) <- {}\n", r_string(&label))
        };
        format!(
            "{m} <- local({{\n  d <- stats::na.omit({}[, c({}), drop = FALSE])\n  \
             set.seed({})\n  boot::boot(d, function(d, i) {stat}, R = {})\n}})\n\
             cat(sprintf(\"Bootstrap: %d replications, seed {}\\n\", nrow({m}$t)))\n\
             {m}$intervals <- local({{\n  est <- {m}$t0\n{naming}  \
             out <- data.frame(Estimate = unname(est), \
             Bias = unname(colMeans({m}$t, na.rm = TRUE) - est), \
             Std.Err = unname(apply({m}$t, 2, stats::sd, na.rm = TRUE)), \
             row.names = names(est))\n  \
             for (k in seq_along(c({}))) {{\n    ty <- c({})[k]\n    \
             lim <- t(sapply(seq_along(est), function(j) {{\n      \
             r <- tryCatch(boot::boot.ci({m}, conf = {level} / 100, type = ty, index = j), \
             error = function(e) NULL)\n      \
             if (is.null(r)) return(c(NA_real_, NA_real_))\n      \
             if (ty == \"norm\") r$normal[2:3] else if (ty == \"perc\") r$percent[4:5] \
             else r$bca[4:5]\n    }}))\n    \
             out[[paste(c({})[k], \"lower\")]] <- lim[, 1]\n    \
             out[[paste(c({})[k], \"upper\")]] <- lim[, 2]\n  }}\n  out\n}})\nprint({m}$intervals)",
            a.data(),
            names.join(", "),
            a.number("seed"),
            a.number("reps"),
            a.number("seed"),
            types.join(", "),
            types.join(", "),
            columns.join(", "),
            columns.join(", "),
        )
    },
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
];

fn permutation_check(a: &Args) -> Vec<Problem> {
    let mut out = Vec::new();
    whole(a, "reps", "The number of permutations", 1, &mut out);
    whole(a, "seed", "The seed", 0, &mut out);
    if a.var_text("v") == a.var_text("w") && !a.var_text("v").is_empty() {
        out.push(Problem::new(
            "w",
            "Choose a different variable from the first one.",
        ));
    }
    not_the_data(a, &mut out);
    out
}

pub const PERMUTATION_TEST: Spec = Spec {
    entry: "Permutation test",
    menu: "Statistics > Resampling > Permutation test",
    about: "Asks how often shuffling the data would give a statistic as far from zero as the one \
            observed. For two groups the rows' group labels are shuffled; for a correlation one \
            variable is shuffled against the other. The two-sided p-value counts the shuffles \
            whose statistic is at least as large in size as the observed one: (count + 1) / \
            (B + 1) over B random shuffles with a seed, the observed arrangement counting as one. \
            When the arrangements number no more than B, every one is listed instead and the \
            p-value is exact: count / arrangements. The difference is the second group's \
            statistic minus the first's (groups in alphabetical order). Rows with a missing \
            value are left out.",
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
            hint: "The numeric variable compared between the groups, or the first of the two \
                   correlated.",
        },
        Widget::Variable {
            id: "w",
            label: "Group variable or second variable",
            kinds: GROUPS,
            required: true,
            hint: "A variable with exactly two groups; for a correlation, a numeric variable.",
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
    code: |a, _| {
        let m = a.name("save");
        let v = r_string(a.var_text("v"));
        let w = r_string(a.var_text("w"));
        let reps = a.number("reps");
        let seed = a.number("seed");
        let kind = a.choice("statistic");
        let take = format!(
            "d <- stats::na.omit({}[, c({v}, {w}), drop = FALSE])\n  n <- nrow(d)\n  y <- d[[1]]\n  ",
            a.data()
        );
        let body = if kind == "correlation" {
            format!(
                "{take}if (!is.numeric(d[[2]])) stop(\"A correlation needs a numeric second variable.\")\n  \
                 if (n < 3) stop(\"A correlation needs at least three rows.\")\n  \
                 x <- d[[2]]\n  stat <- function(p) stats::cor(x, y[p])\n  \
                 obs <- stat(seq_len(n))\n  arrangements <- if (n <= 170) factorial(n) else Inf\n  \
                 exact <- lfactorial(n) <= log({reps})\n  \
                 if (exact) {{\n    \
                 perms <- function(k) {{\n      if (k == 1) return(matrix(1L, 1, 1))\n      \
                 q <- perms(k - 1)\n      \
                 do.call(rbind, lapply(seq_len(k), function(i) \
                 cbind(i, matrix(ifelse(q >= i, q + 1L, q), nrow(q)))))\n    }}\n    \
                 t <- apply(perms(n), 1, stat)\n  }} else {{\n    \
                 set.seed({seed})\n    t <- replicate({reps}, stat(sample.int(n)))\n  }}"
            )
        } else {
            let f = if kind == "medians" {
                "stats::median"
            } else {
                "mean"
            };
            format!(
                "{take}g <- factor(d[[2]])\n  \
                 if (nlevels(g) != 2) stop(\"The group variable must have exactly two groups among the rows used; it has \", nlevels(g), \".\")\n  \
                 second <- g == levels(g)[2]\n  \
                 stat <- function(s) {f}(y[s]) - {f}(y[!s])\n  \
                 obs <- stat(second)\n  arrangements <- choose(n, sum(second))\n  \
                 exact <- arrangements <= {reps}\n  \
                 if (exact) {{\n    \
                 t <- utils::combn(n, sum(second), function(ix) {{\n      \
                 s <- logical(n)\n      s[ix] <- TRUE\n      stat(s)\n    }})\n  }} else {{\n    \
                 set.seed({seed})\n    t <- replicate({reps}, stat(sample(second)))\n  }}"
            )
        };
        format!(
            "{m} <- local({{\n  {body}\n  tol <- 1e-9 * max(1, abs(obs))\n  \
             count <- sum(abs(t) >= abs(obs) - tol)\n  \
             p <- if (exact) count / length(t) else (count + 1) / (length(t) + 1)\n  \
             list(statistic = obs, p.value = p, exact = exact, arrangements = arrangements, \
             permutations = length(t), replicates = t)\n}})\n\
             cat(sprintf(\"Observed statistic: %.6g\\n\", {m}$statistic))\n\
             cat(sprintf(if ({m}$exact) \"All %.0f arrangements listed (exact)\\n\" \
             else \"%d random permutations of %.4g arrangements, seed {seed}\\n\", \
             if ({m}$exact) {m}$arrangements else {m}$permutations, {m}$arrangements))\n\
             cat(sprintf(\"Two-sided p-value: %.6g\\n\", {m}$p.value))\n\
             if (!{m}$exact) cat(sprintf(\"Monte Carlo standard error: %.4g\\n\", \
             sqrt({m}$p.value * (1 - {m}$p.value) / {m}$permutations)))"
        )
    },
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
                 set.seed(1)\n  boot::boot(d, function(d, i) mean(d[[1]][i]), R = 999)\n})\n"
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
        assert!(c.contains("stats::cor(d[[1]][i], d[[2]][i])"), "{c}");
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
    }
}
