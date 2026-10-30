//! Post-estimation (P4.5): what to do with a model already fitted. These dialogs
//! open from the Objects pane on a model object (`lm`, `glm`, `aov` and the
//! models fitted by MASS and nnet), so the model is the dialog's "object" and
//! there are no columns to choose from. They print R's own output, or add
//! columns to a data frame named in the dialog. Base R, except robust standard
//! errors, which need the sandwich and lmtest packages.

use rcraft_rcodegen::dialog::{Args, Opt, Problem, Spec, TextDefault, TextRule, Widget};
use rcraft_rcodegen::{r_name, r_string};

use crate::data::BASE;

/// The first class names of R objects these dialogs are offered on.
pub const MODEL_CLASSES: &[&str] = &["lm", "glm", "aov", "negbin", "polr", "multinom"];

/// Whether an object whose first class is `class` is a model these dialogs take.
pub fn is_model_class(class: &str) -> bool {
    MODEL_CLASSES.contains(&class)
}

const LINEAR_ONLY: &str = "stopifnot(\"this works for linear and generalised linear models\" = \
                           inherits({m}, \"lm\"))";

/// The guard line for models that are `lm` or `glm` objects.
fn linear_guard(model: &str) -> String {
    LINEAR_ONLY.replace("{m}", model)
}

// ---- Model fit ----

pub const MODEL_FIT: Spec = Spec {
    entry: "Model fit",
    menu: "Objects > Model > Fit statistics",
    about: "Number of observations, log-likelihood, AIC and BIC, with R-squared for a linear \
            model and the deviance for a generalised linear model.",
    widgets: &[],
    needs: &[],
    styles: BASE,
    check: |_| Vec::new(),
    code: |a, _| {
        let m = a.data();
        format!(
            "(function(m) {{\n  ll <- stats::logLik(m)\n  \
             cat(\"Observations: \", stats::nobs(m), \"\\n\", sep = \"\")\n  \
             print(c(logLik = as.numeric(ll), df = attr(ll, \"df\"), AIC = stats::AIC(m), \
             BIC = stats::BIC(m)))\n  \
             if (inherits(m, \"glm\")) {{\n    cat(\"\\nDeviance\\n\")\n    \
             print(c(null = m$null.deviance, residual = m$deviance))\n  }} else if (inherits(m, \"lm\")) {{\n    \
             s <- stats::summary.lm(m)\n    cat(\"\\nR-squared\\n\")\n    \
             print(c(r.squared = s$r.squared, adj.r.squared = s$adj.r.squared))\n  }}\n}})({m})"
        )
    },
};

// ---- Compare models ----

/// The names typed in the Compare box, separated by commas.
fn other_models<'a>(a: &'a Args) -> Vec<&'a str> {
    a.values
        .text("others")
        .split(',')
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .collect()
}

fn compare_check(a: &Args) -> Vec<Problem> {
    let others = other_models(a);
    if others.is_empty() {
        return vec![Problem::new(
            "others",
            "Type the name of at least one other model.",
        )];
    }
    let mut seen = vec![a.ctx.dataset.as_str()];
    for name in others {
        if seen.contains(&name) {
            return vec![Problem::new(
                "others",
                format!("{name} is listed more than once, or is this model."),
            )];
        }
        seen.push(name);
    }
    Vec::new()
}

pub const COMPARE_MODELS: Spec = Spec {
    entry: "Compare models",
    menu: "Objects > Model > Compare models",
    about: "Put this model next to others in a table of log-likelihood, AIC and BIC, and test \
            whether the larger of nested models fits better. Models should be fitted to the same \
            rows; R refuses or warns when they are not.",
    widgets: &[
        Widget::Text {
            id: "others",
            label: "Other models",
            default: TextDefault::Empty,
            rule: TextRule::Any,
            hint: "The names of models in the workspace, separated by commas. This model is \
                   included first.",
        },
        Widget::Check {
            id: "nested",
            label: "Test whether the larger model fits better (nested models only)",
            default: true,
            hint: "An F test for linear models, a likelihood-ratio test for generalised ones.",
        },
    ],
    needs: &[],
    styles: BASE,
    check: compare_check,
    code: |a, _| {
        let mut names = vec![a.ctx.dataset.as_str()];
        names.extend(other_models(a));
        let list: Vec<String> = names
            .iter()
            .map(|n| format!("{} = {}", r_name(n), r_name(n)))
            .collect();
        let mut body = String::from(
            "  print(data.frame(model = names(models), \
             df = vapply(models, function(m) attr(stats::logLik(m), \"df\"), numeric(1)), \
             logLik = vapply(models, function(m) as.numeric(stats::logLik(m)), numeric(1)), \
             AIC = vapply(models, stats::AIC, numeric(1)), \
             BIC = vapply(models, stats::BIC, numeric(1)), row.names = NULL))\n",
        );
        if a.flag("nested") {
            body.push_str(
                "  test <- if (all(vapply(models, inherits, logical(1), \"glm\"))) \
                 list(test = \"Chisq\") else if (all(vapply(models, inherits, logical(1), \"lm\"))) \
                 list(test = \"F\") else list()\n  \
                 cat(\"\\nTest of nested models\\n\")\n  \
                 print(do.call(stats::anova, c(unname(models), test)))\n",
            );
        }
        format!("(function(models) {{\n{body}}})(list({}))", list.join(", "))
    },
};

// ---- Predicted values and residuals ----

const FITTED_TYPES: &[Opt] = &[
    Opt {
        value: "response",
        label: "The outcome's scale (probabilities, counts)",
    },
    Opt {
        value: "link",
        label: "The linear predictor (generalised models only)",
    },
];

const RESIDUAL_TYPES: &[Opt] = &[
    Opt {
        value: "deviance",
        label: "Deviance residuals",
    },
    Opt {
        value: "pearson",
        label: "Pearson residuals",
    },
    Opt {
        value: "response",
        label: "Raw residuals (observed minus predicted)",
    },
];

fn predicted_check(a: &Args) -> Vec<Problem> {
    let (fitted, resid) = (a.name_text("fitted"), a.name_text("resid"));
    if fitted.is_empty() && resid.is_empty() {
        vec![Problem::new("fitted", "Name at least one new variable.")]
    } else if fitted == resid {
        vec![Problem::new(
            "resid",
            "The two new variables need different names.",
        )]
    } else {
        Vec::new()
    }
}

pub const PREDICTED: Spec = Spec {
    entry: "Predicted values and residuals",
    menu: "Objects > Model > Predicted values and residuals",
    about: "Add the model's predicted values and residuals to a data frame as new variables. \
            Use the data frame the model was fitted to. Rows the model left out for missing \
            values get missing in the new variables. A name already in the data frame is replaced.",
    widgets: &[
        Widget::Text {
            id: "data",
            label: "Data frame",
            default: TextDefault::Empty,
            rule: TextRule::Name,
            hint: "The name of the data frame the model was fitted to; it is changed in place.",
        },
        Widget::Text {
            id: "fitted",
            label: "Predicted values as",
            default: TextDefault::Fixed("fitted"),
            rule: TextRule::Any,
            hint: "Leave empty to skip.",
        },
        Widget::Choice {
            id: "type",
            label: "Predict on",
            options: FITTED_TYPES,
            default: "response",
            hint: "",
        },
        Widget::Text {
            id: "resid",
            label: "Residuals as",
            default: TextDefault::Fixed("resid"),
            rule: TextRule::Any,
            hint: "Leave empty to skip.",
        },
        Widget::Choice {
            id: "rtype",
            label: "Kind of residual",
            options: RESIDUAL_TYPES,
            default: "deviance",
            hint: "For a linear model the deviance residual is the raw residual.",
        },
    ],
    needs: &[],
    styles: BASE,
    check: predicted_check,
    code: |a, _| {
        let (m, d) = (a.data(), a.name("data"));
        let mut code = linear_guard(&m);
        if !a.name_text("fitted").is_empty() {
            code.push_str(&format!(
                "\n{d}[[{}]] <- stats::predict({m}, newdata = {d}, type = {})",
                a.name_string("fitted"),
                r_string(a.choice("type")),
            ));
        }
        if !a.name_text("resid").is_empty() {
            code.push_str(&format!(
                "\nres <- stats::residuals({m}, type = {})\n\
                 stopifnot(\"the model was not fitted to this data frame\" = \
                 all(names(res) %in% rownames({d})))\n\
                 {d}[[{}]] <- res[match(rownames({d}), names(res))]\nrm(res)",
                r_string(a.choice("rtype")),
                a.name_string("resid"),
            ));
        }
        code
    },
};

// ---- Diagnostics ----

pub const DIAGNOSTICS: Spec = Spec {
    entry: "Model diagnostics",
    menu: "Objects > Model > Diagnostics",
    about: "Checks on a linear model: whether the residuals look normal, whether their spread \
            changes with the predicted value (a Koenker version of the Breusch-Pagan test), and \
            which rows pull the fit hardest (Cook's distance and leverage). For a generalised \
            model only the influence part is shown.",
    widgets: &[],
    needs: &[],
    styles: BASE,
    check: |_| Vec::new(),
    code: |a, _| {
        let m = a.data();
        format!(
            "{guard}\n(function(m) {{\n  r <- stats::residuals(m, type = \"deviance\")\n  n <- length(r)\n  \
             cat(\"Observations: \", n, \"\\n\", sep = \"\")\n  \
             if (!inherits(m, \"glm\")) {{\n    \
             if (n >= 3 && n <= 5000) {{\n      s <- stats::shapiro.test(r)\n      \
             cat(\"\\nShapiro-Wilk test of the residuals: W = \", signif(s$statistic, 5), \
             \", p = \", signif(s$p.value, 4), \"\\n\", sep = \"\")\n    }}\n    \
             bp <- n * stats::summary.lm(stats::lm(r^2 ~ stats::fitted(m)))$r.squared\n    \
             cat(\"\\nBreusch-Pagan test of constant spread (Koenker): chi-squared(1) = \", \
             signif(bp, 5), \", p = \", signif(stats::pchisq(bp, 1, lower.tail = FALSE), 4), \
             \"\\n\", sep = \"\")\n  }}\n  \
             cd <- stats::cooks.distance(m)\n  h <- stats::hatvalues(m)\n  \
             k <- length(stats::coef(m))\n  \
             cat(\"\\nRows with Cook's distance above 4/n: \", sum(cd > 4 / n, na.rm = TRUE), \"\\n\", sep = \"\")\n  \
             cat(\"Rows with leverage above 2k/n: \", sum(h > 2 * k / n, na.rm = TRUE), \"\\n\", sep = \"\")\n  \
             top <- utils::head(order(cd, decreasing = TRUE), 5)\n  \
             cat(\"\\nFive rows with the largest Cook's distance\\n\")\n  \
             print(data.frame(row = names(cd)[top], cooks.distance = unname(cd[top]), \
             leverage = unname(h[top])), row.names = FALSE)\n}})({m})",
            guard = linear_guard(&m),
        )
    },
};

// ---- Diagnostic plots ----

pub const DIAGNOSTIC_PLOTS: Spec = Spec {
    entry: "Diagnostic plots",
    menu: "Objects > Model > Diagnostic plots",
    about: "The four standard plots for a linear or generalised linear model on one page: \
            residuals against fitted values, a normal Q-Q plot of the residuals, spread \
            against fitted values, and residuals against leverage.",
    widgets: &[],
    needs: &[],
    styles: BASE,
    check: |_| Vec::new(),
    code: |a, _| {
        let m = a.data();
        format!(
            "{}\n(function(m) {{\n  op <- graphics::par(mfrow = c(2, 2))\n  \
             on.exit(graphics::par(op))\n  plot(m)\n}})({m})",
            linear_guard(&m)
        )
    },
};

// ---- Robust standard errors ----

const ROBUST_TYPES: &[Opt] = &[
    Opt {
        value: "HC1",
        label: "HC1 (the usual \"robust\" correction)",
    },
    Opt {
        value: "HC3",
        label: "HC3 (better in small samples)",
    },
    Opt {
        value: "HC2",
        label: "HC2",
    },
    Opt {
        value: "HC0",
        label: "HC0 (no small-sample correction)",
    },
];

const LEVELS: &[Opt] = &[
    Opt {
        value: "0.95",
        label: "95%",
    },
    Opt {
        value: "0.9",
        label: "90%",
    },
    Opt {
        value: "0.99",
        label: "99%",
    },
];

pub const ROBUST: Spec = Spec {
    entry: "Robust standard errors",
    menu: "Objects > Model > Robust standard errors",
    about: "Re-test the model's coefficients with standard errors that stay valid when the \
            spread of the errors is not constant (heteroskedasticity-robust). The model itself \
            is not changed.",
    widgets: &[
        Widget::Choice {
            id: "type",
            label: "Correction",
            options: ROBUST_TYPES,
            default: "HC1",
            hint: "",
        },
        Widget::Text {
            id: "cluster",
            label: "Cluster by",
            default: TextDefault::Empty,
            rule: TextRule::Any,
            hint: "Optional: a variable whose groups may be correlated inside. Needs the data \
                   frame below. Rows the model left out are matched by row name.",
        },
        Widget::Text {
            id: "data",
            label: "Data frame",
            default: TextDefault::Empty,
            rule: TextRule::Any,
            hint: "The data frame the model was fitted to; only needed to cluster.",
        },
        Widget::Choice {
            id: "conf",
            label: "Confidence level",
            options: LEVELS,
            default: "0.95",
            hint: "",
        },
    ],
    needs: &["sandwich", "lmtest"],
    styles: BASE,
    check: |a| {
        let (cluster, data) = (a.name_text("cluster"), a.name_text("data"));
        if !cluster.is_empty() && data.is_empty() {
            vec![Problem::new(
                "data",
                "Name the data frame that holds the cluster variable.",
            )]
        } else if cluster.is_empty() && !data.is_empty() {
            vec![Problem::new(
                "cluster",
                "Name the cluster variable, or empty the data frame.",
            )]
        } else {
            Vec::new()
        }
    },
    code: |a, _| {
        let m = a.data();
        let ty = r_string(a.choice("type"));
        let (cluster, data) = (a.name_text("cluster"), a.name("data"));
        let (vcov, label) = if cluster.is_empty() {
            (
                format!("sandwich::vcovHC({m}, type = {ty})"),
                format!("{} robust", a.choice("type")),
            )
        } else {
            (
                format!(
                    "sandwich::vcovCL({m}, cluster = {data}[[{}]][match(rownames(stats::model.frame({m})), \
                     rownames({data}))], type = {ty})",
                    a.name_string("cluster"),
                ),
                format!("{} cluster-robust (by {cluster})", a.choice("type")),
            )
        };
        format!(
            "{guard}\ncat(\"Coefficients with {label} standard errors\\n\")\n\
             print(lmtest::coeftest({m}, vcov. = {vcov}))\n\
             cat(\"\\nConfidence intervals\\n\")\n\
             print(lmtest::coefci({m}, vcov. = {vcov}, level = {}))",
            a.choice("conf"),
            guard = linear_guard(&m),
        )
    },
};

// ---- Tests of terms ----

const TERM_TESTS: &[Opt] = &[
    Opt {
        value: "each",
        label: "Each term on its own (drop it from the model)",
    },
    Opt {
        value: "together",
        label: "The terms listed below, together",
    },
];

/// The terms typed in the box, separated by commas.
fn typed_terms<'a>(a: &'a Args) -> Vec<&'a str> {
    a.values
        .text("terms")
        .split(',')
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .collect()
}

pub const TEST_TERMS: Spec = Spec {
    entry: "Test terms",
    menu: "Objects > Model > Test terms",
    about: "Test whether terms of the model matter: each term on its own, or several together \
            (does the model fit worse without them?). An F test for linear models, a \
            likelihood-ratio test for generalised ones.",
    widgets: &[
        Widget::Choice {
            id: "kind",
            label: "Test",
            options: TERM_TESTS,
            default: "each",
            hint: "",
        },
        Widget::Text {
            id: "terms",
            label: "Terms to test together",
            default: TextDefault::Empty,
            rule: TextRule::Any,
            hint: "Variable names in the model, separated by commas. Used by the second choice.",
        },
    ],
    needs: &[],
    styles: BASE,
    check: |a| {
        if a.choice("kind") == "together" && typed_terms(a).is_empty() {
            vec![Problem::new("terms", "Type the terms to test together.")]
        } else {
            Vec::new()
        }
    },
    code: |a, _| {
        let m = a.data();
        let test = "if (inherits(m, \"glm\")) \"Chisq\" else \"F\"";
        if a.choice("kind") == "each" {
            return format!(
                "{}\n(function(m) stats::drop1(m, test = {test}))({m})",
                linear_guard(&m)
            );
        }
        let terms = typed_terms(a);
        let quoted: Vec<String> = terms.iter().map(|t| r_string(&r_name(t))).collect();
        let minus: Vec<String> = terms.iter().map(|t| r_name(t)).collect();
        format!(
            "{guard}\n(function(m) {{\n  have <- attr(stats::terms(m), \"term.labels\")\n  \
             stopifnot(\"not terms of the model\" = all(c({}) %in% have))\n  \
             reduced <- stats::update(m, . ~ . - {})\n  \
             cat(\"Model without the terms against the full model\\n\")\n  \
             print(stats::anova(reduced, m, test = {test}))\n}})({m})",
            quoted.join(", "),
            minus.join(" - "),
            guard = linear_guard(&m),
        )
    },
};

// ---- Average marginal effects ----

pub const MARGINAL_EFFECTS: Spec = Spec {
    entry: "Average marginal effects",
    menu: "Objects > Model > Average marginal effects",
    about: "How much the outcome changes, on average over the rows, when each predictor goes \
            up by one unit (or, for a category, from the reference category). On the \
            outcome's own scale, so a logistic model gives changes in probability. Uses the \
            marginaleffects package.",
    widgets: &[Widget::Choice {
        id: "conf",
        label: "Confidence level",
        options: LEVELS,
        default: "0.95",
        hint: "",
    }],
    needs: &["marginaleffects"],
    styles: BASE,
    check: |_| Vec::new(),
    code: |a, _| {
        format!(
            "print(marginaleffects::avg_slopes({}, conf_level = {}))",
            a.data(),
            a.choice("conf")
        )
    },
};

// ---- Sums of squares, Type II and Type III ----

const SS_TYPES: &[Opt] = &[
    Opt {
        value: "2",
        label: "Type II: each term after the terms that do not contain it",
    },
    Opt {
        value: "3",
        label: "Type III: each term after all the others (factors coded to sum to zero)",
    },
];

/// Type II and III sums of squares of a linear model in base R, from the
/// model's own model matrix: the sum of squares of a term is the rise in the
/// residual sum of squares when its columns are taken out (Type III: all other
/// columns stay; Type II: only the terms that do not contain it stay, so a
/// main effect is judged without the interactions it is part of). Type III
/// recodes the factors to sum to zero first, which is what makes it mean the
/// same thing whatever coding the model was fitted with; the saved model is
/// not touched. The table is an `anova` object, printed like R's own.
const SS_R: &str = r#"(function(m, type) {
  if (!inherits(m, "lm") || inherits(m, "glm")) stop("This needs a model fitted with the Linear model dialog (lm).")
  if (anyNA(stats::coef(m))) stop("The model has terms that are redundant with each other (a coefficient is NA); take one of them out first.")
  tt <- stats::terms(m)
  mf <- stats::model.frame(m)
  y <- stats::model.response(mf)
  w <- stats::model.weights(mf)
  off <- stats::model.offset(mf)
  if (is.null(off)) off <- 0
  coding <- if (type == 3) lapply(m$xlevels, function(x) "contr.sum") else m$contrasts
  if (!length(coding)) coding <- NULL
  X <- stats::model.matrix(tt, mf, contrasts.arg = coding)
  asg <- attr(X, "assign")
  labels <- attr(tt, "term.labels")
  fac <- attr(tt, "factors")
  contains <- function(j, i) all(fac[fac[, i] > 0, j] > 0)
  rss <- function(cols) {
    if (!length(cols)) {
      r <- y - off
    } else {
      xs <- X[, cols, drop = FALSE]
      fit <- if (is.null(w)) stats::lm.fit(xs, y - off) else stats::lm.wfit(xs, y - off, w)
      r <- fit$residuals
    }
    sum(if (is.null(w)) r^2 else w * r^2)
  }
  full <- rss(seq_len(ncol(X)))
  df_res <- m$df.residual
  ids <- if (type == 3) sort(unique(asg)) else setdiff(sort(unique(asg)), 0)
  ss <- numeric()
  df <- numeric()
  for (i in ids) {
    if (type == 3) {
      ss <- c(ss, rss(which(asg != i)) - full)
    } else {
      outer_terms <- which(vapply(seq_along(labels), function(j) j != i && contains(j, i), TRUE))
      without <- which(!(asg %in% c(i, outer_terms)))
      ss <- c(ss, rss(without) - rss(which(!(asg %in% outer_terms))))
    }
    df <- c(df, sum(asg == i))
  }
  f <- (ss / df) / (full / df_res)
  tab <- data.frame(
    `Sum Sq` = c(ss, full), Df = c(df, df_res), `Mean Sq` = c(ss / df, full / df_res),
    `F value` = c(f, NA), `Pr(>F)` = c(stats::pf(f, df, df_res, lower.tail = FALSE), NA),
    check.names = FALSE, row.names = c(ifelse(ids == 0, "(Intercept)", labels[pmax(ids, 1)]), "Residuals"))
  print(structure(tab, heading = c(paste0("Anova Table (Type ", c("II", "III")[type - 1], " tests)\n"),
    paste("Response:", deparse(stats::formula(m)[[2L]]))), class = c("anova", "data.frame")))
  if (type == 3) cat("\nFactors were coded to sum to zero for this table; the saved model is unchanged.\n")
  invisible(tab)
})"#;

pub const SUMS_OF_SQUARES: Spec = Spec {
    entry: "Sums of squares (Type II/III)",
    menu: "Objects > Model > Sums of squares (Type II/III)",
    about: "The analysis-of-variance table of a linear model with Type II or Type III sums of \
            squares, which do not depend on the order the terms were typed in (the Linear \
            model dialog's own table does). The model itself is not changed.",
    widgets: &[Widget::Choice {
        id: "type",
        label: "Sums of squares",
        options: SS_TYPES,
        default: "2",
        hint: "With no interactions in the model the two give the same tests; they differ \
               when there are interactions, most when group sizes are unequal.",
    }],
    needs: &[],
    styles: BASE,
    check: |_| Vec::new(),
    code: |a, _| format!("{SS_R}({m}, {t}L)", m = a.data(), t = a.choice("type")),
};

// ---- Custom contrasts ----

const ADJUSTMENTS: &[Opt] = &[
    Opt {
        value: "none",
        label: "None: each comparison on its own",
    },
    Opt {
        value: "holm",
        label: "Holm: for several comparisons at once",
    },
    Opt {
        value: "bonferroni",
        label: "Bonferroni: for several comparisons at once",
    },
];

/// Planned comparisons between the levels of one factor of a linear model, in
/// base R. The estimated mean of each level is the model's prediction averaged
/// over every combination of the model's other factors (equal weights), with
/// number variables held at their means; a comparison is a sum of those means
/// with weights that add up to zero. Its standard error comes from the model's
/// covariance matrix, so it is the usual one for the model, with the model's
/// residual degrees of freedom. Models that bend a number variable (`poly()`,
/// `log()`, `I(x^2)`) are refused, because "held at its mean" would be unclear.
const CONTRASTS_R: &str = r#"(function(m, fac, spec, adjust, level) {
  if (!inherits(m, "lm") || inherits(m, "glm")) stop("This needs a model fitted with the Linear model dialog (lm).")
  if (anyNA(stats::coef(m))) stop("The model has terms that are redundant with each other (a coefficient is NA); take one of them out first.")
  lv <- m$xlevels[[fac]]
  if (is.null(lv)) stop(paste0("'", fac, "' is not a factor in the model. The factors are: ", paste(names(m$xlevels), collapse = ", "), "."))
  mf <- stats::model.frame(m)
  used <- names(mf)[-1L]
  used <- used[!startsWith(used, "(")]
  if (!all(used %in% all.vars(stats::formula(m)[[3L]]))) stop("Comparisons need a model whose number variables enter as they are (no poly(), log(), I(x^2) and the like).")
  tt <- stats::delete.response(stats::terms(m))
  others <- setdiff(names(m$xlevels), fac)
  grid <- if (length(others)) expand.grid(m$xlevels[others], stringsAsFactors = FALSE) else data.frame(row = 1)
  base <- mf[rep(1L, nrow(grid)), used, drop = FALSE]
  for (v in used) if (!(v %in% names(m$xlevels))) base[[v]] <- mean(mf[[v]])
  for (v in others) base[[v]] <- factor(grid[[v]], levels = m$xlevels[[v]])
  rows <- t(vapply(lv, function(l) {
    nd <- base
    nd[[fac]] <- factor(rep(l, nrow(nd)), levels = lv)
    colMeans(stats::model.matrix(tt, nd, contrasts.arg = m$contrasts))
  }, numeric(length(stats::coef(m)))))
  rownames(rows) <- lv
  b <- stats::coef(m)
  v <- stats::vcov(m)
  df <- m$df.residual
  parts <- trimws(strsplit(spec, ";", fixed = TRUE)[[1L]])
  parts <- parts[nzchar(parts)]
  if (!length(parts)) stop("Type at least one comparison.")
  est <- se <- numeric(length(parts))
  for (i in seq_along(parts)) {
    pieces <- trimws(strsplit(parts[i], ",", fixed = TRUE)[[1L]])
    w <- stats::setNames(numeric(length(lv)), lv)
    for (p in pieces[nzchar(pieces)]) {
      kv <- trimws(strsplit(p, "=", fixed = TRUE)[[1L]])
      weight <- suppressWarnings(as.numeric(kv[2L]))
      if (length(kv) != 2L || is.na(weight) || !is.finite(weight)) stop(paste0("'", p, "' should be a level, an equals sign and a weight, such as: Treated = 1."))
      if (!(kv[1L] %in% lv)) stop(paste0("'", kv[1L], "' is not a level of ", fac, ". The levels are: ", paste(lv, collapse = ", "), "."))
      w[kv[1L]] <- w[kv[1L]] + weight
    }
    if (abs(sum(w)) > 1e-8) stop(paste0("The weights of '", parts[i], "' add up to ", format(sum(w)), ", not 0, so it does not compare levels."))
    cvec <- as.vector(w %*% rows)
    est[i] <- sum(cvec * b)
    se[i] <- sqrt(drop(t(cvec) %*% v %*% cvec))
  }
  num <- function(x) formatC(x, digits = 4, format = "f")
  mean_est <- as.vector(rows %*% b)
  mean_se <- sqrt(rowSums((rows %*% v) * rows))
  cat("Estimated means of ", fac, " (other factors averaged, number variables at their means)\n", sep = "")
  print(data.frame(Level = lv, Mean = round(mean_est, 4), `Std. Error` = round(mean_se, 4), check.names = FALSE), row.names = FALSE)
  tval <- est / se
  p_raw <- 2 * stats::pt(-abs(tval), df)
  crit <- stats::qt(1 - (1 - level) / 2, df)
  tab <- data.frame(Comparison = parts, Estimate = est, `Std. Error` = se, t = tval, df = df,
                    p = stats::p.adjust(p_raw, adjust),
                    lower = est - crit * se, upper = est + crit * se, check.names = FALSE)
  names(tab)[6] <- if (adjust == "none") "p" else paste0("p (", adjust, ")")
  names(tab)[7:8] <- paste0(c("lower ", "upper "), format(100 * level), "%")
  cat("\nComparisons (intervals are not adjusted)\n")
  print(tab, digits = 4, row.names = FALSE)
  invisible(tab)
})"#;

pub const CONTRASTS: Spec = Spec {
    entry: "Custom contrasts",
    menu: "Objects > Model > Custom contrasts",
    about: "Compare the levels of one factor in a linear model with comparisons you choose \
            (a level against another, or against the mean of several), with an estimate, \
            standard error, test and interval for each. The model is not changed.",
    widgets: &[
        Widget::Text {
            id: "factor",
            label: "Factor",
            default: TextDefault::Empty,
            rule: TextRule::Any,
            hint: "The factor whose levels you compare, named as in the model.",
        },
        Widget::Text {
            id: "contrasts",
            label: "Comparisons",
            default: TextDefault::Empty,
            rule: TextRule::Any,
            hint: "Levels with weights that add up to zero, a comparison per group, groups \
                   separated by semicolons. For example: Treated = 1, Control = -1; \
                   Treated = 1, Low = -0.5, Control = -0.5. Level names must not contain \
                   commas, semicolons or equals signs.",
        },
        Widget::Choice {
            id: "adjust",
            label: "p-values",
            options: ADJUSTMENTS,
            default: "none",
            hint: "",
        },
        Widget::Choice {
            id: "conf",
            label: "Confidence level",
            options: LEVELS,
            default: "0.95",
            hint: "",
        },
    ],
    needs: &[],
    styles: BASE,
    check: |a| {
        let mut out = Vec::new();
        if a.name_text("factor").is_empty() {
            out.push(Problem::new("factor", "Type the factor to compare."));
        }
        if a.values.text("contrasts").trim().is_empty() {
            out.push(Problem::new(
                "contrasts",
                "Type at least one comparison, such as: A = 1, B = -1.",
            ));
        }
        out
    },
    code: |a, _| {
        format!(
            "{CONTRASTS_R}({m}, {fac}, {spec}, {adjust}, {conf})",
            m = a.data(),
            fac = r_string(a.name_text("factor")),
            spec = r_string(a.values.text("contrasts").trim()),
            adjust = r_string(a.choice("adjust")),
            conf = a.choice("conf"),
        )
    },
};

/// The dialogs offered on a model object, in menu order.
pub const MODEL_SPECS: &[&Spec] = &[
    &MODEL_FIT,
    &COMPARE_MODELS,
    &PREDICTED,
    &DIAGNOSTICS,
    &DIAGNOSTIC_PLOTS,
    &ROBUST,
    &TEST_TERMS,
    &SUMS_OF_SQUARES,
    &CONTRASTS,
    &MARGINAL_EFFECTS,
];

#[cfg(test)]
mod tests {
    use super::*;
    use rcraft_rcodegen::dialog::{Context, Style, Values};

    fn make(spec: &Spec, model: &str, set: &[(&str, &str)]) -> Result<String, Vec<Problem>> {
        let c = Context {
            dataset: model.into(),
            frames: Vec::new(),
            columns: Vec::new(),
        };
        let mut v: Values = spec.defaults(&c);
        for (id, text) in set {
            v.set_text(id, *text);
        }
        spec.generate(&c, &v, Style::Base)
    }

    fn message(spec: &Spec, model: &str, set: &[(&str, &str)]) -> String {
        match make(spec, model, set) {
            Ok(code) => format!("no problem: {code}"),
            Err(p) => p
                .iter()
                .map(|p| p.message.clone())
                .collect::<Vec<_>>()
                .join(" "),
        }
    }

    #[test]
    fn model_actions_are_offered_on_model_classes_only() {
        for class in ["lm", "glm", "aov", "negbin", "polr", "multinom"] {
            assert!(is_model_class(class), "{class}");
        }
        for class in ["data.frame", "tbl_df", "function", "numeric", ""] {
            assert!(!is_model_class(class), "{class}");
        }
        assert!(crate::data::is_object_menu(
            "Objects > Model > Fit statistics"
        ));
    }

    #[test]
    fn model_fit_and_diagnostics_name_the_model_safely() {
        let fit = make(&MODEL_FIT, "my model", &[]).unwrap_or_default();
        assert!(fit.ends_with("})(`my model`)"), "{fit}");
        assert!(fit.contains("stats::summary.lm(m)"), "{fit}");
        let diag = make(&DIAGNOSTICS, "my model", &[]).unwrap_or_default();
        assert!(
            diag.starts_with(
                "stopifnot(\"this works for linear and generalised linear models\" = \
                 inherits(`my model`, \"lm\"))\n"
            ),
            "{diag}"
        );
        assert!(diag.contains("stats::cooks.distance(m)"), "{diag}");
    }

    #[test]
    fn compare_models_needs_other_distinct_models() {
        assert!(message(&COMPARE_MODELS, "m1", &[]).contains("at least one other model"));
        assert!(message(&COMPARE_MODELS, "m1", &[("others", " , ")]).contains("at least one"));
        assert!(message(&COMPARE_MODELS, "m1", &[("others", "m2, m2")]).contains("more than once"));
        assert!(message(&COMPARE_MODELS, "m1", &[("others", "m1")]).contains("more than once"));
        let out = make(&COMPARE_MODELS, "m1", &[("others", "m2, big model")]).unwrap_or_default();
        assert!(
            out.ends_with("(list(m1 = m1, m2 = m2, `big model` = `big model`))"),
            "{out}"
        );
        assert!(out.contains("do.call(stats::anova"), "{out}");
    }

    #[test]
    fn predicted_values_write_named_columns_and_check_the_names() {
        let out = make(
            &PREDICTED,
            "m1",
            &[("data", "my data"), ("fitted", "yhat"), ("resid", "e")],
        )
        .unwrap_or_default();
        assert!(
            out.contains(
                "`my data`[[\"yhat\"]] <- stats::predict(m1, newdata = `my data`, type = \"response\")"
            ),
            "{out}"
        );
        assert!(
            out.contains("res <- stats::residuals(m1, type = \"deviance\")"),
            "{out}"
        );
        assert!(out.ends_with("rm(res)"), "{out}");
        // only predicted values: no residual lines
        let only = make(
            &PREDICTED,
            "m1",
            &[("data", "d"), ("fitted", "yhat"), ("resid", "")],
        )
        .unwrap_or_default();
        assert!(!only.contains("residuals"), "{only}");
        assert!(message(
            &PREDICTED,
            "m1",
            &[("data", "d"), ("fitted", ""), ("resid", "")]
        )
        .contains("at least one new variable"));
        assert!(message(
            &PREDICTED,
            "m1",
            &[("data", "d"), ("fitted", "x"), ("resid", "x")]
        )
        .contains("different names"));
        assert!(message(&PREDICTED, "m1", &[("data", "")]).contains("Type"));
    }

    #[test]
    fn robust_errors_need_two_packages_and_write_the_chosen_correction() {
        assert_eq!(ROBUST.packages(Style::Base), vec!["sandwich", "lmtest"]);
        let out = make(&ROBUST, "m1", &[("type", "HC1"), ("conf", "0.9")]).unwrap_or_default();
        assert!(
            out.contains(
                "print(lmtest::coeftest(m1, vcov. = sandwich::vcovHC(m1, type = \"HC1\")))"
            ),
            "{out}"
        );
        assert!(out.ends_with("level = 0.9))"), "{out}");
        assert!(message(&ROBUST, "m1", &[("type", "HC9")]).contains("Pick one"));
    }

    #[test]
    fn clustering_needs_both_the_variable_and_the_data_frame() {
        assert!(message(&ROBUST, "m1", &[("cluster", "school")]).contains("Name the data frame"));
        assert!(message(&ROBUST, "m1", &[("data", "d")]).contains("Name the cluster variable"));
        let out = make(
            &ROBUST,
            "m1",
            &[("cluster", "school"), ("data", "my data"), ("type", "HC3")],
        )
        .unwrap_or_default();
        assert!(
            out.contains(
                "sandwich::vcovCL(m1, cluster = `my data`[[\"school\"]]\
                 [match(rownames(stats::model.frame(m1)), rownames(`my data`))], type = \"HC3\")"
            ),
            "{out}"
        );
        assert!(
            out.contains("HC3 cluster-robust (by school) standard errors"),
            "{out}"
        );
    }

    #[test]
    fn test_terms_checks_the_terms_in_the_generated_r() {
        let each = make(&TEST_TERMS, "m1", &[]).unwrap_or_default();
        assert!(
            each.ends_with(
                "stats::drop1(m, test = if (inherits(m, \"glm\")) \"Chisq\" else \"F\"))(m1)"
            ),
            "{each}"
        );
        assert!(message(&TEST_TERMS, "m1", &[("kind", "together")]).contains("Type the terms"));
        let together = make(
            &TEST_TERMS,
            "m1",
            &[("kind", "together"), ("terms", "age, my var")],
        )
        .unwrap_or_default();
        assert!(
            together.contains("all(c(\"age\", \"`my var`\") %in% have)"),
            "{together}"
        );
        assert!(
            together.contains("stats::update(m, . ~ . - age - `my var`)"),
            "{together}"
        );
    }

    #[test]
    fn marginal_effects_need_their_package_and_use_the_level() {
        assert_eq!(
            MARGINAL_EFFECTS.packages(Style::Base),
            vec!["marginaleffects"]
        );
        assert_eq!(
            make(&MARGINAL_EFFECTS, "my fit", &[("conf", "0.9")]).unwrap_or_default(),
            "print(marginaleffects::avg_slopes(`my fit`, conf_level = 0.9))"
        );
    }

    #[test]
    fn every_dialog_is_a_catalog_entry_and_in_the_statistics_menu_list() {
        for spec in MODEL_SPECS {
            assert!(
                crate::ENTRIES
                    .iter()
                    .any(|e| e.name == spec.entry && e.area == crate::Area::Statistics),
                "{} has no Statistics catalog entry",
                spec.entry
            );
            assert!(spec.menu.starts_with("Objects > Model > "));
        }
    }
}
