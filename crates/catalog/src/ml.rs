//! Statistics menu, machine learning basics (P6.12): one dialog with four
//! steps, written with tidymodels.
//!
//! 1. **Split** the rows (those with no missing value in the columns used)
//!    into a training and a test set with `rsample::initial_split`, with a
//!    proportion and a seed, stratified by a column if asked. The split is
//!    what `sample` draws with the same seed: the training rows are
//!    `sample(n, floor(n * proportion))`.
//! 2. **Fit** on the training rows with `parsnip`: a linear regression (`lm`),
//!    a logistic regression (`glm`; the outcome has two levels, the first of
//!    which is the event) or a random forest (`ranger`, when installed;
//!    otherwise the dialog stops and names the package).
//! 3. **Tune**, if asked: the penalty of a lasso or ridge (`glmnet`, when
//!    installed) over a regular grid with v-fold cross-validation
//!    (`tune::tune_grid`), then the fit at the best penalty.
//! 4. **Evaluate** on the test rows with `yardstick`: RMSE and R² for a
//!    numeric outcome; accuracy, ROC AUC (two classes) and the confusion
//!    matrix for a class outcome.
//!
//! The split, the fitted workflow and the test predictions stay in the
//! workspace (`<name>_split`, `<name>_fit`, `<name>_pred`, with `<name>_train`,
//! `<name>_test` and, when tuning, `<name>_tuned`); the data are not changed.

use rcraft_rcodegen::dialog::{Args, Opt, Problem, Spec, TextDefault, TextRule, VarKind, Widget};
use rcraft_rcodegen::{r_name, r_string};

use crate::data::BASE;

const OUTCOMES: &[VarKind] = &[
    VarKind::Number,
    VarKind::Factor,
    VarKind::Text,
    VarKind::Logical,
];
const PREDICTORS: &[VarKind] = OUTCOMES;

/// Whether the model is a classifier: logistic always, the forest when the
/// outcome is not a number.
fn is_class(a: &Args) -> bool {
    match a.choice("model") {
        "logistic" => true,
        "forest" => !matches!(
            a.ctx.column(a.var_text("outcome")).map(|c| c.kind),
            Some(VarKind::Number)
        ),
        _ => false,
    }
}

fn is_tuned(a: &Args) -> bool {
    a.flag("tune")
}

/// A whole number typed in a number box, within the given bounds.
fn whole(a: &Args, id: &str, low: i64, high: i64) -> Option<i64> {
    a.number(id)
        .parse::<i64>()
        .ok()
        .filter(|n| (low..=high).contains(n))
}

/// A number typed in a number box, strictly between or up to the bounds.
fn fraction(a: &Args, id: &str) -> Option<f64> {
    a.number(id).parse::<f64>().ok().filter(|x| x.is_finite())
}

fn ml_check(a: &Args) -> Vec<Problem> {
    let mut out = Vec::new();
    let outcome = a.var_text("outcome");
    let predictors = a.vars("predictors");
    if predictors.contains(&outcome) {
        out.push(Problem::new(
            "predictors",
            format!("{outcome} is the outcome; it cannot also be a predictor."),
        ));
    }
    let outcome_kind = a.ctx.column(outcome).map(|c| c.kind);
    if a.choice("model") == "linear" && outcome_kind != Some(VarKind::Number) {
        out.push(Problem::new(
            "outcome",
            "A linear regression needs a numeric outcome; for classes choose the logistic \
             regression or the random forest.",
        ));
    }
    if a.choice("model") == "forest" && is_tuned(a) {
        out.push(Problem::new(
            "tune",
            "The penalty is tuned for the linear and the logistic model; a random forest has \
             none. Untick the box.",
        ));
    }
    match fraction(a, "proportion") {
        Some(p) if p > 0.0 && p < 1.0 => {}
        _ => out.push(Problem::new(
            "proportion",
            "The share for training is a number above 0 and below 1 (0.75 is three quarters).",
        )),
    }
    if whole(a, "seed", 0, i32::MAX as i64).is_none() {
        out.push(Problem::new(
            "seed",
            "The seed is a whole number from 0 to 2147483647.",
        ));
    }
    if whole(a, "folds", 2, 100).is_none() {
        out.push(Problem::new(
            "folds",
            "The number of folds is a whole number from 2 to 100.",
        ));
    }
    if whole(a, "levels", 2, 200).is_none() {
        out.push(Problem::new(
            "levels",
            "The number of penalties tried is a whole number from 2 to 200.",
        ));
    }
    match fraction(a, "mixture") {
        Some(m) if (0.0..=1.0).contains(&m) => {}
        _ => out.push(Problem::new(
            "mixture",
            "The lasso share is a number from 0 (ridge) to 1 (lasso).",
        )),
    }
    if whole(a, "trees", 1, 10_000).is_none() {
        out.push(Problem::new(
            "trees",
            "The number of trees is a whole number from 1 to 10000.",
        ));
    }
    let base = a.name_text("save");
    let taken = [
        "split", "train", "test", "fit", "pred", "tuned", "spec", "wf", "folds", "grid", "best",
    ];
    if taken.iter().any(|s| format!("{base}_{s}") == a.ctx.dataset) {
        out.push(Problem::new(
            "save",
            format!("{} is the data frame; choose another name.", a.ctx.dataset),
        ));
    }
    out
}

/// The name of one of the results, `<base>_<part>`.
fn object(a: &Args, part: &str) -> String {
    r_name(&format!("{}_{part}", a.name_text("save")))
}

/// The columns the model uses, each once, as R strings: the outcome, the
/// predictors, then the stratifying column.
fn used(a: &Args) -> Vec<String> {
    let mut names: Vec<&str> = vec![a.var_text("outcome")];
    names.extend(a.vars("predictors"));
    if a.var("strata").is_some() {
        names.push(a.var_text("strata"));
    }
    let mut seen: Vec<&str> = Vec::new();
    for n in names {
        if !seen.contains(&n) {
            seen.push(n);
        }
    }
    seen.iter().map(|n| r_string(n)).collect()
}

fn formula(a: &Args) -> String {
    let rhs: Vec<String> = a.vars("predictors").iter().map(|p| r_name(p)).collect();
    format!("{} ~ {}", r_name(a.var_text("outcome")), rhs.join(" + "))
}

/// The stop that names a package the step needs and the machine lacks.
fn needs_package(package: &str, what: &str) -> String {
    format!(
        "if (!requireNamespace({q}, quietly = TRUE)) stop({msg}, call. = FALSE)\n",
        q = r_string(package),
        msg = r_string(&format!(
            "{what} needs the {package} package, which is not installed. Install it and run \
             this dialog again."
        ))
    )
}

/// The model specification and the packages it needs.
fn model_lines(a: &Args, class: bool) -> String {
    let mixture = fraction(a, "mixture").unwrap_or(1.0);
    match (a.choice("model"), is_tuned(a)) {
        ("forest", _) => {
            let trees = whole(a, "trees", 1, 10_000).unwrap_or(500);
            let mode = if class {
                "classification"
            } else {
                "regression"
            };
            format!(
                "{}{spec} <- parsnip::set_mode(parsnip::set_engine(parsnip::rand_forest(trees = \
                 {trees}), \"ranger\"), \"{mode}\")\n",
                needs_package("ranger", "A random forest"),
                spec = object(a, "spec")
            )
        }
        ("logistic", true) => format!(
            "{}{spec} <- parsnip::set_engine(parsnip::logistic_reg(penalty = tune::tune(), \
             mixture = {mixture}), \"glmnet\")\n",
            needs_package("glmnet", "Tuning the penalty"),
            spec = object(a, "spec")
        ),
        ("linear", true) => format!(
            "{}{spec} <- parsnip::set_engine(parsnip::linear_reg(penalty = tune::tune(), \
             mixture = {mixture}), \"glmnet\")\n",
            needs_package("glmnet", "Tuning the penalty"),
            spec = object(a, "spec")
        ),
        ("logistic", false) => format!(
            "{spec} <- parsnip::set_engine(parsnip::logistic_reg(), \"glm\")\n",
            spec = object(a, "spec")
        ),
        _ => format!(
            "{spec} <- parsnip::set_engine(parsnip::linear_reg(), \"lm\")\n",
            spec = object(a, "spec")
        ),
    }
}

fn ml_code(a: &Args) -> String {
    let class = is_class(a);
    let data = a.data();
    let seed = whole(a, "seed", 0, i32::MAX as i64).unwrap_or(1);
    let prop = fraction(a, "proportion").unwrap_or(0.75);
    let y = r_string(a.var_text("outcome"));
    let (split, train, test, fit, pred, spec, tuned) = (
        object(a, "split"),
        object(a, "train"),
        object(a, "test"),
        object(a, "fit"),
        object(a, "pred"),
        object(a, "spec"),
        object(a, "tuned"),
    );
    let (wf, folds, grid, best) = (
        object(a, "wf"),
        object(a, "folds"),
        object(a, "grid"),
        object(a, "best"),
    );
    let cols = used(a).join(", ");
    let mut out = String::new();

    // 1. split (the model first, so a missing package stops the run early)
    out.push_str(&model_lines(a, class));
    let strata = a
        .var("strata")
        .map(|s| format!(", strata = {s}"))
        .unwrap_or_default();
    let to_class = if !class {
        String::new()
    } else if a.choice("model") == "logistic" {
        format!(
            "  k[[{y}]] <- factor(k[[{y}]])\n  \
             if (nlevels(k[[{y}]]) != 2) stop(\"The outcome needs exactly two classes.\", \
             call. = FALSE)\n"
        )
    } else {
        format!(
            "  k[[{y}]] <- factor(k[[{y}]])\n  \
             if (nlevels(k[[{y}]]) < 2) stop(\"The outcome needs at least two classes.\", \
             call. = FALSE)\n"
        )
    };
    out.push_str(&format!(
        "set.seed({seed})\n\
         {split} <- local({{\n  k <- stats::na.omit({data}[c({cols})])\n  \
         cat(\"Rows used:\", nrow(k), \"of\", nrow({data}), \
         \"(rows missing a value are left out)\\n\")\n{to_class}  \
         rsample::initial_split(k, prop = {prop}{strata})\n}})\n\
         {train} <- rsample::training({split})\n\
         {test} <- rsample::testing({split})\n\
         cat(\"Training rows:\", nrow({train}), \"  Test rows:\", nrow({test}), \
         \"  Seed: {seed}\\n\")\n"
    ));
    if class {
        out.push_str(&format!(
            "cat(\"The event (the first level of the outcome) is\", levels({train}[[{y}]])[1], \
             \"\\n\")\n"
        ));
    }

    // 2. fit (3. tune)
    out.push_str(&format!(
        "{wf} <- workflows::workflow({}, {spec})\n",
        formula(a)
    ));
    if is_tuned(a) {
        let v = whole(a, "folds", 2, 100).unwrap_or(5);
        let levels = whole(a, "levels", 2, 200).unwrap_or(10);
        let (metrics, main) = if class {
            ("yardstick::accuracy, yardstick::roc_auc", "roc_auc")
        } else {
            ("yardstick::rmse, yardstick::rsq", "rmse")
        };
        let fold_strata = if class {
            format!(", strata = {}", r_name(a.var_text("outcome")))
        } else {
            String::new()
        };
        out.push_str(&format!(
            "set.seed({seed})\n\
             {folds} <- rsample::vfold_cv({train}, v = {v}{fold_strata})\n\
             {grid} <- dials::grid_regular(dials::penalty(), levels = {levels})\n\
             {tuned} <- tune::tune_grid({wf}, resamples = {folds}, grid = {grid}, \
             metrics = yardstick::metric_set({metrics}))\n\
             cat(\"\\nCross-validation, {v} folds: the best penalties\\n\")\n\
             print(as.data.frame(tune::show_best({tuned}, metric = \"{main}\", n = 5)))\n\
             {best} <- tune::select_best({tuned}, metric = \"{main}\")\n\
             cat(\"Penalty chosen:\", format({best}$penalty), \"\\n\")\n\
             {fit} <- parsnip::fit(tune::finalize_workflow({wf}, {best}), data = {train})\n\
             local({{\n  lo <- min(workflows::extract_fit_engine({fit})$lambda)\n  \
             if ({best}$penalty < lo) cat(\"glmnet's path of penalties stops at\", format(lo), \
             \"; a smaller penalty is predicted at that end.\\n\")\n}})\n"
        ));
    } else {
        out.push_str(&format!("{fit} <- parsnip::fit({wf}, data = {train})\n"));
    }
    out.push_str("cat(\"\\nFitted model\\n\")\n");
    match (a.choice("model"), is_tuned(a)) {
        ("forest", _) => out.push_str(&format!("print(workflows::extract_fit_engine({fit}))\n")),
        (_, true) => out.push_str(&format!(
            "print(stats::coef(workflows::extract_fit_engine({fit}), s = {best}$penalty))\n"
        )),
        _ => out.push_str(&format!(
            "print(as.data.frame(broom::tidy(workflows::extract_fit_parsnip({fit}))))\n"
        )),
    }

    // 4. evaluate
    if class {
        out.push_str(&format!(
            "{pred} <- data.frame(truth = {test}[[{y}]], \
             .pred_class = stats::predict({fit}, {test}, type = \"class\")[[\".pred_class\"]], \
             .prob = stats::predict({fit}, {test}, type = \"prob\")[[1]])\n\
             cat(\"\\nTest-set metrics\\n\")\n\
             print(as.data.frame(rbind(yardstick::accuracy({pred}, truth, .pred_class), \
             if (nlevels({pred}$truth) == 2) yardstick::roc_auc({pred}, truth, .prob))))\n\
             cat(\"\\nConfusion matrix (rows: predicted, columns: truth)\\n\")\n\
             print(yardstick::conf_mat({pred}, truth, .pred_class))\n"
        ));
    } else {
        out.push_str(&format!(
            "{pred} <- data.frame(truth = {test}[[{y}]], \
             .pred = stats::predict({fit}, {test})[[\".pred\"]])\n\
             cat(\"\\nTest-set metrics\\n\")\n\
             print(as.data.frame(yardstick::metric_set(yardstick::rmse, yardstick::rsq)\
             ({pred}, truth = truth, estimate = .pred)))\n"
        ));
    }
    out
}

const MODELS: &[Opt] = &[
    Opt {
        value: "linear",
        label: "Linear regression (numeric outcome)",
    },
    Opt {
        value: "logistic",
        label: "Logistic regression (two classes)",
    },
    Opt {
        value: "forest",
        label: "Random forest (needs ranger)",
    },
];

pub const ML: Spec = Spec {
    entry: "Machine learning model",
    menu: "Statistics > Machine learning model",
    about: "Splits the rows into a training and a test set, fits a model on the training rows \
            with tidymodels, optionally chooses its penalty by cross-validation, and scores it \
            on the test rows: RMSE and R-squared for a numeric outcome; accuracy, ROC AUC and \
            the confusion matrix for classes. Rows missing a value the model uses are left out.",
    widgets: &[
        Widget::Variable {
            id: "outcome",
            label: "Outcome",
            kinds: OUTCOMES,
            required: true,
            hint: "A number for a linear regression; two classes for a logistic regression (the \
                   first level is the event).",
        },
        Widget::Variables {
            id: "predictors",
            label: "Predictors",
            kinds: PREDICTORS,
            min: 1,
            hint: "",
        },
        Widget::Choice {
            id: "model",
            label: "Model",
            options: MODELS,
            default: "linear",
            hint: "The random forest uses the ranger package; when it is missing the dialog \
                   stops and says so.",
        },
        Widget::Text {
            id: "proportion",
            label: "Share of rows for training",
            default: TextDefault::Fixed("0.75"),
            rule: TextRule::Number,
            hint: "The training rows are those sample() draws with the seed, floor(n * share) of \
                   them (as rsample::initial_split).",
        },
        Widget::Text {
            id: "seed",
            label: "Seed",
            default: TextDefault::Fixed("123"),
            rule: TextRule::Number,
            hint: "Set before the split and before the folds, so a run can be repeated.",
        },
        Widget::Variable {
            id: "strata",
            label: "Stratify the split by (optional)",
            kinds: &[],
            required: false,
            hint: "Each stratum is split in the same proportion (a number is cut into four \
                   groups first).",
        },
        Widget::Text {
            id: "trees",
            label: "Trees (random forest)",
            default: TextDefault::Fixed("500"),
            rule: TextRule::Number,
            hint: "",
        },
        Widget::Check {
            id: "tune",
            label: "Choose the penalty by cross-validation",
            default: false,
            hint: "Fits a lasso or ridge with glmnet (when installed) over a grid of penalties \
                   with v-fold cross-validation on the training rows, and keeps the best. \
                   Linear and logistic models only.",
        },
        Widget::Text {
            id: "folds",
            label: "Folds (v)",
            default: TextDefault::Fixed("5"),
            rule: TextRule::Number,
            hint: "",
        },
        Widget::Text {
            id: "levels",
            label: "Penalties tried",
            default: TextDefault::Fixed("10"),
            rule: TextRule::Number,
            hint: "A regular grid on the log scale, from 1e-10 to 1.",
        },
        Widget::Text {
            id: "mixture",
            label: "Lasso share (1 lasso, 0 ridge)",
            default: TextDefault::Fixed("1"),
            rule: TextRule::Number,
            hint: "",
        },
        Widget::Text {
            id: "save",
            label: "Save the results as",
            default: TextDefault::Fixed("ml"),
            rule: TextRule::Name,
            hint: "Kept in R's workspace as name_split, name_train, name_test, name_fit (the \
                   fitted workflow) and name_pred (the test predictions), with name_tuned when \
                   tuning; names already in use are replaced.",
        },
    ],
    needs: &["tidymodels"],
    styles: BASE,
    check: ml_check,
    code: |a, _| ml_code(a),
};

pub const SPECS: &[&Spec] = &[&ML];

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
                col("y", VarKind::Number),
                col("x1", VarKind::Number),
                col("g", VarKind::Factor),
                col("hit", VarKind::Logical),
                col("odd name", VarKind::Number),
            ],
        }
    }

    fn values(set: &[(&str, &str)], lists: &[(&str, &[&str])]) -> Values {
        let mut v = ML.defaults(&ctx());
        v.set_text("outcome", "y");
        v.set_list("predictors", vec!["x1".into(), "g".into()]);
        for (id, t) in set {
            v.set_text(id, *t);
        }
        for (id, names) in lists {
            v.set_list(id, names.iter().map(|n| n.to_string()).collect());
        }
        v
    }

    fn code(set: &[(&str, &str)]) -> Result<String, Vec<Problem>> {
        ML.generate(&ctx(), &values(set, &[]), Style::Base)
    }

    fn problem(set: &[(&str, &str)]) -> String {
        code(set)
            .err()
            .and_then(|p| p.first().map(|p| p.message.clone()))
            .unwrap_or_default()
    }

    #[test]
    fn a_linear_fit_splits_fits_and_scores() {
        let c = code(&[]).unwrap_or_default();
        assert!(c.contains("set.seed(123)\nml_split <- local({"), "{c}");
        assert!(c.contains("rsample::initial_split(k, prop = 0.75)"), "{c}");
        assert!(
            c.contains("ml_spec <- parsnip::set_engine(parsnip::linear_reg(), \"lm\")"),
            "{c}"
        );
        assert!(
            c.contains("workflows::workflow(y ~ x1 + g, ml_spec)"),
            "{c}"
        );
        assert!(
            c.contains("ml_fit <- parsnip::fit(ml_wf, data = ml_train)"),
            "{c}"
        );
        assert!(
            c.contains("yardstick::metric_set(yardstick::rmse, yardstick::rsq)"),
            "{c}"
        );
        assert!(!c.contains("vfold_cv"), "{c}");
    }

    #[test]
    fn a_logistic_fit_makes_the_outcome_a_factor_and_scores_classes() {
        let mut v = values(&[("model", "logistic"), ("strata", "g")], &[]);
        v.set_text("outcome", "hit");
        v.set_list("predictors", vec!["x1".into(), "g".into()]);
        let c = ML.generate(&ctx(), &v, Style::Base).unwrap_or_default();
        assert!(c.contains("k[[\"hit\"]] <- factor(k[[\"hit\"]])"), "{c}");
        assert!(
            c.contains("initial_split(k, prop = 0.75, strata = g)"),
            "{c}"
        );
        assert!(
            c.contains("yardstick::roc_auc(ml_pred, truth, .prob)"),
            "{c}"
        );
        assert!(c.contains("yardstick::conf_mat"), "{c}");
        assert!(c.contains("\"glm\""), "{c}");
    }

    #[test]
    fn tuning_names_glmnet_and_uses_the_folds_and_grid_asked() {
        let mut v = values(&[("folds", "4"), ("levels", "7"), ("mixture", "0.5")], &[]);
        v.set_flag("tune", true);
        let c = ML.generate(&ctx(), &v, Style::Base).unwrap_or_default();
        assert!(c.contains("requireNamespace(\"glmnet\""), "{c}");
        assert!(
            c.contains("Tuning the penalty needs the glmnet package"),
            "{c}"
        );
        assert!(c.contains("mixture = 0.5"), "{c}");
        assert!(c.contains("rsample::vfold_cv(ml_train, v = 4)"), "{c}");
        assert!(
            c.contains("dials::grid_regular(dials::penalty(), levels = 7)"),
            "{c}"
        );
        assert!(
            c.contains("tune::select_best(ml_tuned, metric = \"rmse\")"),
            "{c}"
        );
    }

    #[test]
    fn a_forest_names_ranger_and_takes_its_mode_from_the_outcome() {
        let c = code(&[("model", "forest"), ("trees", "200")]).unwrap_or_default();
        assert!(c.contains("requireNamespace(\"ranger\""), "{c}");
        assert!(c.contains("rand_forest(trees = 200)"), "{c}");
        assert!(c.contains("\"regression\")"), "{c}");
        let mut v = values(&[("model", "forest")], &[]);
        v.set_text("outcome", "g");
        v.set_list("predictors", vec!["x1".into()]);
        let c = ML.generate(&ctx(), &v, Style::Base).unwrap_or_default();
        assert!(c.contains("\"classification\")"), "{c}");
    }

    #[test]
    fn bad_choices_are_refused() {
        assert!(problem(&[("proportion", "1")]).contains("above 0 and below 1"));
        assert!(problem(&[("proportion", "0")]).contains("above 0 and below 1"));
        assert!(problem(&[("seed", "-1")]).contains("whole number"));
        assert!(problem(&[("seed", "1.5")]).contains("whole number"));
        assert!(problem(&[("folds", "1")]).contains("folds"));
        assert!(problem(&[("levels", "1")]).contains("penalties"));
        assert!(problem(&[("mixture", "2")]).contains("lasso share"));
        assert!(problem(&[("trees", "0")]).contains("trees"));
        assert!(problem(&[("save", "")]).contains("Type"));
        assert!(problem(&[("model", "logistic"), ("outcome", "y")]).is_empty());
        let mut v = values(&[], &[("predictors", &["x1"])]);
        v.set_text("outcome", "g");
        let p = ML
            .generate(&ctx(), &v, Style::Base)
            .err()
            .unwrap_or_default();
        assert!(
            p.first()
                .is_some_and(|p| p.message.contains("numeric outcome")),
            "{p:?}"
        );
        let mut v = values(&[("model", "forest")], &[]);
        v.set_flag("tune", true);
        let p = ML
            .generate(&ctx(), &v, Style::Base)
            .err()
            .unwrap_or_default();
        assert!(
            p.first()
                .is_some_and(|p| p.message.contains("random forest has")),
            "{p:?}"
        );
        let mut v = values(&[], &[("predictors", &["y", "x1"])]);
        v.set_text("outcome", "y");
        let p = ML
            .generate(&ctx(), &v, Style::Base)
            .err()
            .unwrap_or_default();
        assert!(
            p.first()
                .is_some_and(|p| p.message.contains("cannot also be")),
            "{p:?}"
        );
    }

    #[test]
    fn odd_column_names_are_quoted() {
        let mut v = values(&[], &[("predictors", &["odd name", "x1"])]);
        v.set_text("outcome", "y");
        let c = ML.generate(&ctx(), &v, Style::Base).unwrap_or_default();
        assert!(c.contains("y ~ `odd name` + x1"), "{c}");
        assert!(c.contains("c(\"y\", \"odd name\", \"x1\")"), "{c}");
    }
}
