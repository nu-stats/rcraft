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
    check: |_| Vec::new(),
    code: |a, _| {
        let m = a.data();
        let vcov = format!(
            "sandwich::vcovHC({m}, type = {})",
            r_string(a.choice("type"))
        );
        format!(
            "{guard}\ncat(\"Coefficients with {} robust standard errors\\n\")\n\
             print(lmtest::coeftest({m}, vcov. = {vcov}))\n\
             cat(\"\\nConfidence intervals\\n\")\n\
             print(lmtest::coefci({m}, vcov. = {vcov}, level = {}))",
            a.choice("type"),
            a.choice("conf"),
            guard = linear_guard(&m),
        )
    },
};

/// The dialogs offered on a model object, in menu order.
pub const MODEL_SPECS: &[&Spec] = &[
    &MODEL_FIT,
    &COMPARE_MODELS,
    &PREDICTED,
    &DIAGNOSTICS,
    &ROBUST,
];

#[cfg(test)]
mod tests {
    use super::*;
    use rcraft_rcodegen::dialog::{Context, Style, Values};

    fn make(spec: &Spec, model: &str, set: &[(&str, &str)]) -> Result<String, Vec<Problem>> {
        let c = Context {
            dataset: model.into(),
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
