//! Statistics menu, structural equation models (P6.10): a model in lavaan's
//! syntax, typed or built from variable pickers (a latent variable measured by
//! some columns, outcomes regressed on it and on other columns), fitted by
//! maximum likelihood with `lavaan::sem`, then its summary with the fit
//! indices and, if asked, the standardised estimates. By default the fit
//! follows Stata's `sem`: intercepts estimated, standard errors from the
//! observed information, and observed exogenous variables held at their
//! sample means, variances and covariances while they covary freely with the
//! latent exogenous ones (lavaan's own default keeps those covariances at
//! zero). The fitted model stays in the workspace; the data are not changed.

use rcraft_rcodegen::dialog::{Args, Problem, Spec, TextDefault, TextRule, VarKind, Widget};
use rcraft_rcodegen::r_string;

use crate::data::BASE;

const NUMBERS: &[VarKind] = &[VarKind::Number];

/// Whether lavaan's model syntax takes `name` as it is: a letter or a dot
/// first, then letters, digits, dots and underscores.
fn plain(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '.')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_')
}

/// The lines the helper builds from the pickers: the latent variable's
/// measurement line, then one regression line per outcome.
fn helper_lines(a: &Args) -> Vec<String> {
    let mut out = Vec::new();
    let latent = a.name_text("latent");
    let indicators = a.vars("indicators");
    if !indicators.is_empty() {
        out.push(format!("{latent} =~ {}", indicators.join(" + ")));
    }
    let mut rhs: Vec<&str> = Vec::new();
    if !indicators.is_empty() && a.flag("by_latent") {
        rhs.push(latent);
    }
    rhs.extend(a.vars("predictors"));
    if !rhs.is_empty() {
        for y in a.vars("outcomes") {
            out.push(format!("{y} ~ {}", rhs.join(" + ")));
        }
    }
    out
}

/// The typed lines (blank ones dropped, spaces trimmed), then the helper's.
fn model_text(a: &Args) -> String {
    let mut lines: Vec<String> = a
        .values
        .text("model")
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect();
    lines.extend(helper_lines(a));
    lines.join("\n")
}

fn sem_check(a: &Args) -> Vec<Problem> {
    let mut out = Vec::new();
    if a.values.text("model").contains('\0') {
        out.push(Problem::new(
            "model",
            "The model cannot hold a null character.",
        ));
    }
    let latent = a.name_text("latent");
    let indicators = a.vars("indicators");
    let outcomes = a.vars("outcomes");
    let predictors = a.vars("predictors");
    if !indicators.is_empty() {
        if !plain(latent) {
            out.push(Problem::new(
                "latent",
                format!(
                    "{latent} cannot be a name in lavaan's model syntax: use letters, digits, dots \
                     and underscores, starting with a letter."
                ),
            ));
        } else if a.ctx.column(latent).is_some() {
            out.push(Problem::new(
                "latent",
                format!("{latent} is a column of the data; a latent variable needs a new name."),
            ));
        }
    }
    let mut seen: Vec<(&str, &str)> = Vec::new();
    for (id, what, names) in [
        ("indicators", "an indicator", &indicators),
        ("outcomes", "an outcome", &outcomes),
        ("predictors", "a predictor", &predictors),
    ] {
        for v in names.iter() {
            if !plain(v) {
                out.push(Problem::new(
                    id,
                    format!(
                        "{v} cannot be a name in lavaan's model syntax; rename the column (letters, \
                         digits, dots and underscores, starting with a letter) or type the model."
                    ),
                ));
            } else if let Some((_, first)) = seen.iter().find(|(s, _)| s == v) {
                out.push(Problem::new(
                    id,
                    format!("{v} is already {first}; in the helper each variable takes one role."),
                ));
            } else {
                seen.push((v, what));
            }
        }
    }
    if !predictors.is_empty() && outcomes.is_empty() {
        out.push(Problem::new(
            "outcomes",
            "Choose the outcomes the predictors predict.",
        ));
    }
    if !outcomes.is_empty()
        && predictors.is_empty()
        && (indicators.is_empty() || !a.flag("by_latent"))
    {
        out.push(Problem::new(
            "outcomes",
            "Nothing predicts the outcomes: choose predictors, or a latent variable with its \
             indicators and the box that lets it predict them.",
        ));
    }
    if out.is_empty() && model_text(a).is_empty() {
        out.push(Problem::new(
            "model",
            "Type the model, or build it with the helper below.",
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

/// The fit as Stata's `sem` makes it: with observed exogenous variables, their
/// means, variances and covariances are fixed at the sample values (on the
/// rows the model uses, divided by N) and counted as lavaan counts its own
/// fixed exogenous moments, so the degrees of freedom stay Stata's; the
/// latent exogenous variables covary with them freely.
const STATA_FIT: &str = "(function(model, data) {\n  \
     f0 <- lavaan::sem(model, data = data, do.fit = FALSE)\n  \
     ox <- lavaan::lavNames(f0, \"ov.x\")\n  \
     if (length(ox) == 0) return(lavaan::sem(model, data = data, estimator = \"ML\", \
     meanstructure = TRUE, information = \"observed\"))\n  \
     lx <- lavaan::lavNames(f0, \"lv.x\")\n  \
     k <- stats::na.omit(data[lavaan::lavNames(f0, \"ov\")])\n  \
     S <- stats::cov(k[ox]) * (nrow(k) - 1) / nrow(k)\n  \
     i <- which(upper.tri(S, diag = TRUE), arr.ind = TRUE)\n  \
     have <- with(lavaan::parTable(f0), c(paste(lhs, op, rhs), paste(rhs, op, lhs)))\n  \
     free <- c(outer(lx, ox, paste, sep = \" ~~ \"))\n  \
     extra <- c(free[!free %in% have], sprintf(\"%s ~~ %.17g*%s\", ox[i[, 1]], S[i], ox[i[, 2]]), \
     sprintf(\"%s ~ %.17g*1\", ox, colMeans(k[ox])))\n  \
     pt <- lavaan::parTable(lavaan::sem(paste(c(model, extra), collapse = \"\\n\"), data = data, \
     meanstructure = TRUE, fixed.x = FALSE, do.fit = FALSE))\n  \
     pt$exo[pt$free == 0L & pt$lhs %in% ox & (pt$op == \"~1\" | (pt$op == \"~~\" & pt$rhs %in% ox))] <- 1L\n  \
     lavaan::sem(pt, data = data, estimator = \"ML\", meanstructure = TRUE, fixed.x = FALSE, \
     information = \"observed\")\n})";

pub const SEM: Spec = Spec {
    entry: "Structural equation model",
    menu: "Statistics > Structural equation model",
    about: "Latent variables measured by observed ones, and regressions among them, fitted \
            together by maximum likelihood with lavaan::sem, as Stata's sem. Type the model in \
            lavaan's syntax or build it with the helper; the summary shows the estimates, the \
            chi-squared test against the saturated model and the fit indices (RMSEA, CFI, TLI, \
            SRMR), with standardised estimates if asked. Rows missing a value the model uses are \
            left out.",
    widgets: &[
        Widget::Text {
            id: "model",
            label: "Model (lavaan syntax)",
            default: TextDefault::Empty,
            rule: TextRule::Lines,
            hint: "One relation per line: L =~ x1 + x2 + x3 (L is measured by x1, x2 and x3; the \
                   first loading is fixed at 1), y ~ L + z (a regression), x1 ~~ x2 (a \
                   covariance). Lines built by the helper below are added after these.",
        },
        Widget::Text {
            id: "latent",
            label: "Helper: latent variable",
            default: TextDefault::Fixed("L"),
            rule: TextRule::Name,
            hint: "A new name, not a column, for the variable the indicators measure.",
        },
        Widget::Variables {
            id: "indicators",
            label: "Measured by (indicators)",
            kinds: NUMBERS,
            min: 0,
            hint: "Adds the line latent =~ indicators. Three or more identify it on its own.",
        },
        Widget::Variables {
            id: "outcomes",
            label: "Outcomes",
            kinds: NUMBERS,
            min: 0,
            hint: "Adds a line outcome ~ predictors for each.",
        },
        Widget::Variables {
            id: "predictors",
            label: "Predicted by (columns)",
            kinds: NUMBERS,
            min: 0,
            hint: "",
        },
        Widget::Check {
            id: "by_latent",
            label: "The latent variable also predicts the outcomes",
            default: true,
            hint: "",
        },
        Widget::Check {
            id: "stata",
            label: "Fit as Stata's sem",
            default: true,
            hint: "Intercepts, observed-information standard errors, observed exogenous \
                   variables held at their sample moments and covarying with the latent ones, \
                   RMSEA on N - 1 and SRMR without the means, as Stata's sem and estat gof. \
                   Unticked: lavaan's own defaults.",
        },
        Widget::Check {
            id: "standardized",
            label: "Standardised estimates",
            default: true,
            hint: "Adds the columns Std.lv (latent variables with variance 1) and Std.all \
                   (every variable standardised, as Stata's standardized option).",
        },
        Widget::Text {
            id: "save",
            label: "Save the model as",
            default: TextDefault::Fixed("fit"),
            rule: TextRule::Name,
            hint: "The fitted model stays in R's workspace under this name; a name already in \
                   use is replaced.",
        },
    ],
    needs: &["lavaan"],
    styles: BASE,
    check: sem_check,
    code: |a, _| {
        let fit = a.name("save");
        let model = r_string(&model_text(a));
        let data = a.data();
        let std = if a.flag("standardized") {
            "TRUE"
        } else {
            "FALSE"
        };
        if a.flag("stata") {
            format!(
                "{fit} <- {STATA_FIT}({model}, {data})\n\
                 lavaan::summary({fit}, fit.measures = TRUE, standardized = {std})\n\
                 local({{\n  \
                 m <- lavaan::fitMeasures({fit}, c(\"chisq\", \"df\", \"pvalue\", \"cfi\", \"tli\", \
                 \"srmr_bentler_nomean\"))\n  \
                 n <- lavaan::lavInspect({fit}, \"ntotal\")\n  \
                 rmsea <- if (m[[\"df\"]] > 0) sqrt(max(m[[\"chisq\"]] - m[[\"df\"]], 0) / \
                 (m[[\"df\"]] * (n - 1))) else 0\n  \
                 cat(\"\\nFit indices as Stata's estat gof (RMSEA on N - 1, SRMR without the means)\\n\")\n  \
                 print(c(chisq = m[[\"chisq\"]], df = m[[\"df\"]], p = m[[\"pvalue\"]], RMSEA = rmsea, \
                 CFI = m[[\"cfi\"]], TLI = m[[\"tli\"]], SRMR = m[[\"srmr_bentler_nomean\"]]))\n}})"
            )
        } else {
            format!(
                "{fit} <- lavaan::sem({model}, data = {data}, estimator = \"ML\")\n\
                 lavaan::summary({fit}, fit.measures = TRUE, standardized = {std})\n\
                 cat(\"\\nFit indices\\n\")\n\
                 print(lavaan::fitMeasures({fit}, c(\"chisq\", \"df\", \"pvalue\", \"rmsea\", \"cfi\", \
                 \"tli\", \"srmr\")))"
            )
        }
    },
};

pub const SPECS: &[&Spec] = &[&SEM];

#[cfg(test)]
mod tests {
    use super::*;
    use rcraft_rcodegen::dialog::{ColumnInfo, Context, Style, Values};

    fn ctx() -> Context {
        let col = |name: &str| ColumnInfo {
            name: name.into(),
            kind: VarKind::Number,
        };
        Context {
            dataset: "d".into(),
            frames: Vec::new(),
            columns: vec![
                col("x1"),
                col("x2"),
                col("x3"),
                col("y"),
                col("z"),
                col("odd name"),
            ],
        }
    }

    fn values(set: &[(&str, &str)], lists: &[(&str, &[&str])]) -> Values {
        let mut v = SEM.defaults(&ctx());
        for (id, t) in set {
            v.set_text(id, *t);
        }
        for (id, names) in lists {
            v.set_list(id, names.iter().map(|n| n.to_string()).collect());
        }
        v
    }

    fn code(set: &[(&str, &str)], lists: &[(&str, &[&str])]) -> Result<String, Vec<Problem>> {
        SEM.generate(&ctx(), &values(set, lists), Style::Base)
    }

    #[test]
    fn the_helper_builds_the_measurement_and_regression_lines() {
        let c = code(
            &[],
            &[
                ("indicators", &["x1", "x2", "x3"]),
                ("outcomes", &["y"]),
                ("predictors", &["z"]),
            ],
        )
        .unwrap_or_default();
        assert!(c.contains("(\"L =~ x1 + x2 + x3\\ny ~ L + z\", d)"), "{c}");
        assert!(c.starts_with("fit <- (function(model, data)"), "{c}");
        assert!(c.contains("standardized = TRUE"), "{c}");
        assert!(c.contains("srmr_bentler_nomean"), "{c}");
    }

    #[test]
    fn typed_lines_come_first_and_lavaan_defaults_when_asked() {
        let mut v = values(
            &[("model", "  F =~ x1 + x2 + x3 \n\n"), ("latent", "G")],
            &[("outcomes", &["y"]), ("predictors", &["z"])],
        );
        v.set_flag("stata", false);
        v.set_flag("standardized", false);
        let c = SEM.generate(&ctx(), &v, Style::Base).unwrap_or_default();
        assert!(
            c.starts_with(
                "fit <- lavaan::sem(\"F =~ x1 + x2 + x3\\ny ~ z\", data = d, estimator = \"ML\")"
            ),
            "{c}"
        );
        assert!(c.contains("standardized = FALSE"), "{c}");
    }

    #[test]
    fn bad_models_are_refused() {
        let problem = |set: &[(&str, &str)], lists: &[(&str, &[&str])]| -> String {
            code(set, lists)
                .err()
                .and_then(|p| p.first().map(|p| p.message.clone()))
                .unwrap_or_default()
        };
        assert!(problem(&[], &[]).contains("Type the model"));
        assert!(problem(&[], &[("indicators", &["odd name", "x2"])]).contains("rename"));
        assert!(
            problem(&[("latent", "y")], &[("indicators", &["x1", "x2"])]).contains("is a column")
        );
        assert!(
            problem(&[("latent", "2L")], &[("indicators", &["x1", "x2"])])
                .contains("cannot be a name")
        );
        assert!(
            problem(&[], &[("indicators", &["x1", "x2"]), ("outcomes", &["x1"])])
                .contains("one role")
        );
        assert!(problem(&[], &[("predictors", &["z"])]).contains("Choose the outcomes"));
        assert!(problem(&[], &[("outcomes", &["y"])]).contains("Nothing predicts"));
        assert!(problem(&[("model", "a =~ b\0")], &[]).contains("null"));
        assert!(problem(&[("model", "L =~ x1 + x2"), ("save", "d")], &[]).contains("data frame"));
    }

    #[test]
    fn quotes_in_the_model_stay_inside_the_string() {
        let c = code(&[("model", "y ~ x1\")\nsystem(\"echo")], &[]).unwrap_or_default();
        assert!(c.contains("(\"y ~ x1\\\")\\nsystem(\\\"echo\", d)"), "{c}");
    }
}
