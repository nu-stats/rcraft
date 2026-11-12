//! Statistics menu, multilevel models (P6.2): linear mixed models with random
//! intercepts, random slopes and a nested level through `lme4::lmer`, and
//! logistic or Poisson models with a random intercept through `lme4::glmer`.
//! The defaults are Stata's (`mixed`, `melogit`, `mepoisson`): maximum
//! likelihood, independent random slopes, and 7-point adaptive quadrature.
//! Each fits its model, keeps it under the name the dialog asks for and
//! prints lme4's summary and Wald intervals for the coefficients. None
//! changes the data.

use rcraft_rcodegen::dialog::{Args, Opt, Problem, Spec, TextDefault, TextRule};
use rcraft_rcodegen::dialog::{VarKind, Widget};
use rcraft_rcodegen::r_name;

use crate::data::BASE;

const NUMBERS: &[VarKind] = &[VarKind::Number];
const PREDICTORS: &[VarKind] = &[
    VarKind::Number,
    VarKind::Factor,
    VarKind::Text,
    VarKind::Logical,
];
const GROUPS: &[VarKind] = &[VarKind::Number, VarKind::Factor, VarKind::Text];

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

const CONFIDENCE: Widget = Widget::Choice {
    id: "conf",
    label: "Confidence level",
    options: LEVELS,
    default: "0.95",
    hint: "For the Wald intervals of the coefficients printed after the summary.",
};

const SAVE: Widget = Widget::Text {
    id: "save",
    label: "Save the model as",
    default: TextDefault::Fixed("model"),
    rule: TextRule::Name,
    hint: "The fitted model stays in R's workspace under this name; a name already in use is \
           replaced.",
};

const GROUP: Widget = Widget::Variable {
    id: "group",
    label: "Groups",
    kinds: GROUPS,
    required: true,
    hint: "What the rows are nested in (pupils in schools): each group gets its own random \
           intercept.",
};

/// The fixed part's predictors, or 1 for none.
fn fixed(a: &Args) -> String {
    match a.vars("x") {
        x if x.is_empty() => "1".to_string(),
        x => x.iter().map(|n| r_name(n)).collect::<Vec<_>>().join(" + "),
    }
}

/// The outcome and groups each in one role, the model not saved over the data.
fn roles(a: &Args, singles: &[(&'static str, &str)]) -> Vec<Problem> {
    let mut out = Vec::new();
    let y = a.var_text("y");
    let x = a.vars("x");
    if x.contains(&y) {
        out.push(Problem::new(
            "x",
            format!("{y} is the outcome; it cannot also be a predictor."),
        ));
    }
    let mut seen: Vec<(&str, &str)> = vec![(y, "outcome")];
    for (id, what) in singles {
        let v = a.var_text(id);
        if v.is_empty() {
            continue;
        }
        if let Some((_, first)) = seen.iter().find(|(s, _)| *s == v) {
            out.push(Problem::new(
                id,
                format!("{v} is already the {first}; choose another variable."),
            ));
        } else if x.contains(&v) {
            out.push(Problem::new(
                id,
                format!("{v} is a predictor; choose another variable for the {what}."),
            ));
        } else {
            seen.push((v, what));
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

// ---- Linear mixed model ----

const METHODS: &[Opt] = &[
    Opt {
        value: "ml",
        label: "Maximum likelihood (Stata's default)",
    },
    Opt {
        value: "reml",
        label: "Restricted maximum likelihood (REML, lme4's default)",
    },
];

const COVARIANCES: &[Opt] = &[
    Opt {
        value: "independent",
        label: "Independent (Stata's default)",
    },
    Opt {
        value: "unstructured",
        label: "Correlated with the intercept (lme4's default)",
    },
];

fn linear_check(a: &Args) -> Vec<Problem> {
    let mut out = roles(a, &[("group", "groups"), ("inner", "nested groups")]);
    let y = a.var_text("y");
    let group = a.var_text("group");
    for s in a.vars("slopes") {
        if s == y || s == group || s == a.var_text("inner") {
            out.push(Problem::new(
                "slopes",
                format!("{s} cannot have a random slope: it is the outcome or a grouping."),
            ));
        }
    }
    out
}

pub const LINEAR_MIXED: Spec = Spec {
    entry: "Linear mixed model",
    menu: "Statistics > Multilevel models > Linear mixed model",
    about: "A linear model for rows nested in groups (pupils in schools, visits in patients): \
            each group gets a random intercept, and optionally random slopes and a nested level \
            (classes within schools), as Stata's mixed. It fits with lme4::lmer, by maximum \
            likelihood as Stata does unless REML is chosen, to the maximum's precision, and \
            prints the summary and Wald intervals for the coefficients.",
    widgets: &[
        Widget::Variable {
            id: "y",
            label: "Outcome",
            kinds: NUMBERS,
            required: true,
            hint: "",
        },
        Widget::Variables {
            id: "x",
            label: "Predictors",
            kinds: PREDICTORS,
            min: 0,
            hint: "The fixed part. Leave empty for the intercept alone.",
        },
        GROUP,
        Widget::Variable {
            id: "inner",
            label: "Nested groups (optional)",
            kinds: GROUPS,
            required: false,
            hint: "A level within the groups (classes within schools), with its own random \
                   intercept, as Stata's || school: || class:.",
        },
        Widget::Variables {
            id: "slopes",
            label: "Random slopes (optional)",
            kinds: NUMBERS,
            min: 0,
            hint: "Predictors whose effect varies from group to group. Usually also among the \
                   predictors.",
        },
        Widget::Choice {
            id: "covariance",
            label: "Random slopes are",
            options: COVARIANCES,
            default: "independent",
            hint: "Whether the slopes may be correlated with the intercept; only used with \
                   random slopes.",
        },
        Widget::Choice {
            id: "method",
            label: "Estimation",
            options: METHODS,
            default: "ml",
            hint: "",
        },
        CONFIDENCE,
        SAVE,
    ],
    needs: &["lme4"],
    styles: BASE,
    check: linear_check,
    code: |a, _| {
        let model = a.name("save");
        let group = a.var("group").unwrap_or_default();
        let slopes = a.vars("slopes");
        let mut random = if slopes.is_empty() {
            format!("(1 | {group})")
        } else {
            let bar = if a.choice("covariance") == "unstructured" {
                "|"
            } else {
                "||"
            };
            format!(
                "(1 + {} {bar} {group})",
                slopes
                    .iter()
                    .map(|n| r_name(n))
                    .collect::<Vec<_>>()
                    .join(" + ")
            )
        };
        if let Some(inner) = a.var("inner") {
            random.push_str(&format!(" + (1 | {group}:{inner})"));
        }
        let reml = if a.choice("method") == "reml" {
            "TRUE"
        } else {
            "FALSE"
        };
        // lmer's default stopping rule can leave a fit with random slopes
        // about 1e-6 from the maximum; these tolerances reach it
        format!(
            "{model} <- lme4::lmer({} ~ {} + {random}, data = {}, REML = {reml}, \
             control = lme4::lmerControl(optCtrl = list(ftol_abs = 1e-14, ftol_rel = 1e-14, \
             xtol_abs = 1e-14, xtol_rel = 1e-14, maxeval = 1e5)))\nprint(summary({model}))\n\
             print(stats::confint({model}, parm = \"beta_\", method = \"Wald\", level = {}))",
            a.var("y").unwrap_or_default(),
            fixed(a),
            a.data(),
            a.choice("conf"),
        )
    },
};

// ---- Logistic or Poisson mixed model ----

const FAMILIES: &[Opt] = &[
    Opt {
        value: "binomial",
        label: "Logistic (0 or 1)",
    },
    Opt {
        value: "poisson",
        label: "Poisson (counts)",
    },
];

fn generalized_check(a: &Args) -> Vec<Problem> {
    let mut out = roles(a, &[("group", "groups"), ("exposure", "exposure")]);
    if !a.var_text("exposure").is_empty() && a.choice("family") != "poisson" {
        out.push(Problem::new(
            "exposure",
            "An exposure is for the Poisson model only; leave it empty for the logistic.",
        ));
    }
    out
}

pub const GENERALIZED_MIXED: Spec = Spec {
    entry: "Logistic or Poisson mixed model",
    menu: "Statistics > Multilevel models > Logistic or Poisson mixed model",
    about: "A logistic model of a 0/1 outcome, or a Poisson model of counts, for rows nested in \
            groups, each group with a random intercept, as Stata's melogit and mepoisson. It \
            fits with lme4::glmer by 7-point adaptive Gauss-Hermite quadrature (Stata's default \
            number of points), to the maximum's precision, and prints the summary and the odds \
            or rate ratios with Wald intervals.",
    widgets: &[
        Widget::Variable {
            id: "y",
            label: "Outcome",
            kinds: NUMBERS,
            required: true,
            hint: "0 and 1 for the logistic model; counts of zero or more for Poisson.",
        },
        Widget::Choice {
            id: "family",
            label: "Model",
            options: FAMILIES,
            default: "binomial",
            hint: "",
        },
        Widget::Variables {
            id: "x",
            label: "Predictors",
            kinds: PREDICTORS,
            min: 0,
            hint: "Leave empty for the intercept alone.",
        },
        GROUP,
        Widget::Variable {
            id: "exposure",
            label: "Exposure (Poisson only)",
            kinds: NUMBERS,
            required: false,
            hint: "Optional: how long or how much each row was observed; its log is the offset, \
                   as Stata's exposure(). Must be above zero.",
        },
        Widget::Check {
            id: "ratios",
            label: "Show odds or rate ratios with confidence intervals",
            default: true,
            hint: "",
        },
        CONFIDENCE,
        SAVE,
    ],
    needs: &["lme4"],
    styles: BASE,
    check: generalized_check,
    code: |a, _| {
        let model = a.name("save");
        let poisson = a.choice("family") == "poisson";
        let offset = match (poisson, a.var("exposure")) {
            (true, Some(e)) => format!(", offset = log({e})"),
            _ => String::new(),
        };
        // glmer's default stopping rules leave the variance about 1e-5 from
        // the maximum; these reach it
        let mut code = format!(
            "{model} <- lme4::glmer({} ~ {} + (1 | {}), data = {}, family = stats::{}{offset}, \
             nAGQ = 7, control = lme4::glmerControl(optimizer = \"bobyqa\", \
             optCtrl = list(rhoend = 1e-12, maxfun = 1e5), tolPwrss = 1e-12))\n\
             print(summary({model}))",
            a.var("y").unwrap_or_default(),
            fixed(a),
            a.var("group").unwrap_or_default(),
            a.data(),
            a.choice("family"),
        );
        let ci = format!(
            "stats::confint({model}, parm = \"beta_\", method = \"Wald\", level = {})",
            a.choice("conf")
        );
        if a.flag("ratios") {
            let (title, column) = if poisson {
                ("Rate ratios", "rate_ratio")
            } else {
                ("Odds ratios", "odds_ratio")
            };
            code.push_str(&format!(
                "\ncat(\"\\n{title}\\n\")\nprint(exp(cbind({column} = lme4::fixef({model}), {ci})))"
            ));
        } else {
            code.push_str(&format!("\nprint({ci})"));
        }
        code
    },
};

pub const SPECS: &[&Spec] = &[&LINEAR_MIXED, &GENERALIZED_MIXED];

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
                col("score", VarKind::Number),
                col("hours", VarKind::Number),
                col("girl", VarKind::Number),
                col("school", VarKind::Number),
                col("class", VarKind::Text),
                col("passed", VarKind::Number),
                col("absences", VarKind::Number),
                col("days", VarKind::Number),
                col("home town", VarKind::Text),
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

    const LMER_CONTROL: &str = "control = lme4::lmerControl(optCtrl = list(ftol_abs = 1e-14, \
         ftol_rel = 1e-14, xtol_abs = 1e-14, xtol_rel = 1e-14, maxeval = 1e5)))";

    #[test]
    fn linear_mixed_writes_lmer() {
        let v = values(
            &LINEAR_MIXED,
            &[("y", "score"), ("group", "school")],
            &[("x", &["hours", "girl"])],
        );
        assert_eq!(
            code(&LINEAR_MIXED, &v),
            format!(
                "model <- lme4::lmer(score ~ hours + girl + (1 | school), data = d, REML = FALSE, \
                 {LMER_CONTROL}\nprint(summary(model))\n\
                 print(stats::confint(model, parm = \"beta_\", method = \"Wald\", level = 0.95))"
            )
        );
        // independent slopes (||), a nested level, REML
        let v = values(
            &LINEAR_MIXED,
            &[
                ("y", "score"),
                ("group", "school"),
                ("inner", "class"),
                ("method", "reml"),
            ],
            &[("x", &["hours"]), ("slopes", &["hours"])],
        );
        assert!(code(&LINEAR_MIXED, &v).starts_with(
            "model <- lme4::lmer(score ~ hours + (1 + hours || school) + (1 | school:class), \
             data = d, REML = TRUE,"
        ));
        let v = values(
            &LINEAR_MIXED,
            &[
                ("y", "score"),
                ("group", "school"),
                ("covariance", "unstructured"),
            ],
            &[("slopes", &["hours"])],
        );
        assert!(code(&LINEAR_MIXED, &v)
            .starts_with("model <- lme4::lmer(score ~ 1 + (1 + hours | school), data = d,"));
    }

    #[test]
    fn linear_mixed_checks_roles() {
        let v = values(
            &LINEAR_MIXED,
            &[("y", "score"), ("group", "hours"), ("inner", "score")],
            &[("x", &["hours"]), ("slopes", &["score"])],
        );
        let problems = LINEAR_MIXED.problems(&ctx(), &v);
        let widgets: Vec<&str> = problems.iter().map(|p| p.widget).collect();
        assert!(widgets.contains(&"group"), "{problems:?}");
        assert!(widgets.contains(&"inner"), "{problems:?}");
        assert!(widgets.contains(&"slopes"), "{problems:?}");
    }

    #[test]
    fn generalized_mixed_writes_glmer() {
        let v = values(
            &GENERALIZED_MIXED,
            &[("y", "passed"), ("group", "school")],
            &[("x", &["hours", "girl"])],
        );
        assert_eq!(
            code(&GENERALIZED_MIXED, &v),
            "model <- lme4::glmer(passed ~ hours + girl + (1 | school), data = d, \
             family = stats::binomial, nAGQ = 7, control = lme4::glmerControl(optimizer = \"bobyqa\", \
             optCtrl = list(rhoend = 1e-12, maxfun = 1e5), tolPwrss = 1e-12))\n\
             print(summary(model))\ncat(\"\\nOdds ratios\\n\")\n\
             print(exp(cbind(odds_ratio = lme4::fixef(model), stats::confint(model, parm = \"beta_\", \
             method = \"Wald\", level = 0.95))))"
        );
        let v = values(
            &GENERALIZED_MIXED,
            &[
                ("y", "absences"),
                ("family", "poisson"),
                ("group", "school"),
                ("exposure", "days"),
            ],
            &[("x", &["hours"])],
        );
        assert!(code(&GENERALIZED_MIXED, &v)
            .contains("family = stats::poisson, offset = log(days), nAGQ = 7"));
        let v = values(
            &GENERALIZED_MIXED,
            &[("y", "passed"), ("group", "school"), ("exposure", "days")],
            &[("x", &["hours"])],
        );
        assert!(GENERALIZED_MIXED
            .problems(&ctx(), &v)
            .iter()
            .any(|p| p.widget == "exposure"));
    }
}
