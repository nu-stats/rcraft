//! Statistics menu, panel and fixed-effects models (P6.1): fixed-effects
//! regression and count or binary models with any number of absorbed fixed
//! effects through `fixest`, the classic panel estimators (within, random,
//! between, pooled, first differences) through `plm`, and the Hausman test
//! between fixed and random effects. Each fits its model, keeps it in the
//! workspace under the name the dialog asks for (the Hausman test keeps
//! none) and prints the package's own summary. None changes the data.

use rcraft_rcodegen::dialog::{Args, Opt, Problem, Spec, TextDefault, TextRule};
use rcraft_rcodegen::dialog::{VarKind, Widget};
use rcraft_rcodegen::{r_name, r_string};

use crate::data::BASE;

const NUMBERS: &[VarKind] = &[VarKind::Number];
const PREDICTORS: &[VarKind] = &[
    VarKind::Number,
    VarKind::Factor,
    VarKind::Text,
    VarKind::Logical,
];
/// What can name a group (a panel, a year, a firm).
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
    hint: "For the confidence intervals printed after the summary.",
};

const PREDICTOR_LIST: Widget = Widget::Variables {
    id: "x",
    label: "Predictors",
    kinds: PREDICTORS,
    min: 1,
    hint: "A variable that does not change within a fixed-effect group is absorbed by it and \
           dropped.",
};

const FIXED_EFFECTS: Widget = Widget::Variables {
    id: "fe",
    label: "Fixed effects",
    kinds: GROUPS,
    min: 1,
    hint: "Each is a grouping (a person, a firm, a year): every group gets its own intercept, \
           which is absorbed rather than shown.",
};

const SAVE: Widget = Widget::Text {
    id: "save",
    label: "Save the model as",
    default: TextDefault::Fixed("model"),
    rule: TextRule::Name,
    hint: "The fitted model stays in R's workspace under this name; a name already in use is \
           replaced.",
};

const ERRORS: &[Opt] = &[
    Opt {
        value: "cluster",
        label: "Clustered by the first fixed effect",
    },
    Opt {
        value: "iid",
        label: "Conventional",
    },
    Opt {
        value: "hetero",
        label: "Robust to heteroskedasticity",
    },
    Opt {
        value: "twoway",
        label: "Clustered by the first two fixed effects",
    },
];

const ERROR_CHOICE: Widget = Widget::Choice {
    id: "errors",
    label: "Standard errors",
    options: ERRORS,
    default: "cluster",
    hint: "fixest's own default is clustered by the first fixed effect. A variable under \
           Cluster by takes the place of this choice.",
};

const CLUSTER: Widget = Widget::Variable {
    id: "cluster",
    label: "Cluster by",
    kinds: GROUPS,
    required: false,
    hint: "Optional: cluster the standard errors by this variable instead.",
};

/// The predictors, joined for a formula.
fn rhs(a: &Args) -> String {
    a.vars("x")
        .iter()
        .map(|n| r_name(n))
        .collect::<Vec<_>>()
        .join(" + ")
}

/// The fixed effects, joined for the part after `|`.
fn fe_part(a: &Args) -> String {
    a.vars("fe")
        .iter()
        .map(|n| r_name(n))
        .collect::<Vec<_>>()
        .join(" + ")
}

/// The outcome is neither a predictor nor a fixed effect, a predictor is not
/// also a fixed effect, and the model is not saved over the data frame.
fn fixest_check(a: &Args) -> Vec<Problem> {
    let y = a.var_text("y");
    let x = a.vars("x");
    let fe = a.vars("fe");
    let mut out = Vec::new();
    if x.contains(&y) {
        out.push(Problem::new(
            "x",
            format!("{y} is the outcome; it cannot also be a predictor."),
        ));
    }
    if fe.contains(&y) {
        out.push(Problem::new(
            "fe",
            format!("{y} is the outcome; it cannot also be a fixed effect."),
        ));
    }
    if let Some(both) = x.iter().find(|v| fe.contains(v)) {
        out.push(Problem::new(
            "fe",
            format!("{both} is a predictor; it cannot also be a fixed effect."),
        ));
    }
    if a.choice("errors") == "twoway" && fe.len() < 2 && a.var("cluster").is_none() {
        out.push(Problem::new(
            "errors",
            "Two-way clustering needs two fixed effects.",
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

/// fixest's `vcov =` argument: a cluster variable, or the choice.
fn vcov_arg(a: &Args) -> String {
    match a.var("cluster") {
        Some(c) => format!("cluster = ~{c}"),
        None => format!("vcov = {}", r_string(a.choice("errors"))),
    }
}

// ---- Fixed-effects regression ----

pub const FIXED_EFFECTS_REGRESSION: Spec = Spec {
    entry: "Fixed-effects regression",
    menu: "Statistics > Panel models > Fixed-effects regression",
    about: "A linear model with one or more fixed effects (each person, firm or year its own \
            intercept), absorbed rather than estimated one by one, so it stays fast with \
            thousands of groups. It fits with fixest::feols and prints its summary and \
            confidence intervals.",
    widgets: &[
        Widget::Variable {
            id: "y",
            label: "Outcome",
            kinds: NUMBERS,
            required: true,
            hint: "",
        },
        PREDICTOR_LIST,
        FIXED_EFFECTS,
        ERROR_CHOICE,
        CLUSTER,
        CONFIDENCE,
        SAVE,
    ],
    needs: &["fixest"],
    styles: BASE,
    check: fixest_check,
    code: |a, _| {
        let model = a.name("save");
        format!(
            "{model} <- fixest::feols({} ~ {} | {}, data = {}, {})\nprint(summary({model}))\n\
             print(stats::confint({model}, level = {}))",
            a.var("y").unwrap_or_default(),
            rhs(a),
            fe_part(a),
            a.data(),
            vcov_arg(a),
            a.choice("conf")
        )
    },
};

// ---- Fixed-effects count or binary model ----

const FAMILIES: &[Opt] = &[
    Opt {
        value: "poisson",
        label: "Poisson (counts)",
    },
    Opt {
        value: "logit",
        label: "Logit (0 or 1)",
    },
];

fn glm_check(a: &Args) -> Vec<Problem> {
    let mut out = fixest_check(a);
    let exposure = a.var_text("exposure");
    if !exposure.is_empty() {
        if a.choice("family") != "poisson" {
            out.push(Problem::new(
                "exposure",
                "An exposure is for the Poisson model only; leave it empty for logit.",
            ));
        } else if exposure == a.var_text("y") {
            out.push(Problem::new(
                "exposure",
                "The exposure cannot be the outcome.",
            ));
        }
    }
    out
}

pub const FIXED_EFFECTS_GLM: Spec = Spec {
    entry: "Fixed-effects count or binary model",
    menu: "Statistics > Panel models > Fixed-effects count or binary model",
    about: "A Poisson model of counts, or a logit model of a 0/1 outcome, with one or more \
            fixed effects absorbed. It fits with fixest::fepois or fixest::feglm. Groups whose \
            outcome never varies (all zero counts, all 0 or all 1) tell nothing about the \
            coefficients and are dropped, with a note. The standard errors follow Stata's \
            conventions for these models (and glm's): no degrees-of-freedom adjustment, and \
            G/(G - 1) for G clusters.",
    widgets: &[
        Widget::Variable {
            id: "y",
            label: "Outcome",
            kinds: NUMBERS,
            required: true,
            hint: "Counts of zero or more for Poisson; 0 and 1 for logit.",
        },
        Widget::Choice {
            id: "family",
            label: "Model",
            options: FAMILIES,
            default: "poisson",
            hint: "",
        },
        PREDICTOR_LIST,
        FIXED_EFFECTS,
        Widget::Variable {
            id: "exposure",
            label: "Exposure (Poisson only)",
            kinds: NUMBERS,
            required: false,
            hint: "Optional: how long or how much each row was observed. The model then \
                   describes rates; its log is added as an offset. Must be above zero.",
        },
        Widget::Check {
            id: "ratios",
            label: "Show rate or odds ratios with confidence intervals",
            default: true,
            hint: "",
        },
        ERROR_CHOICE,
        CLUSTER,
        CONFIDENCE,
        SAVE,
    ],
    needs: &["fixest"],
    styles: BASE,
    check: glm_check,
    code: |a, _| {
        let model = a.name("save");
        let poisson = a.choice("family") == "poisson";
        let offset = match (poisson, a.var("exposure")) {
            (true, Some(e)) => format!(", offset = ~log({e})"),
            _ => String::new(),
        };
        let call = if poisson {
            "fixest::fepois("
        } else {
            "fixest::feglm("
        };
        let family = if poisson { "" } else { ", family = \"logit\"" };
        let mut code = format!(
            "{model} <- {call}{} ~ {} | {}, data = {}{family}{offset}, {}, \
             ssc = fixest::ssc(adj = FALSE))\nprint(summary({model}))",
            a.var("y").unwrap_or_default(),
            rhs(a),
            fe_part(a),
            a.data(),
            vcov_arg(a),
        );
        if a.flag("ratios") {
            let (title, column) = if poisson {
                ("Rate ratios", "rate_ratio")
            } else {
                ("Odds ratios", "odds_ratio")
            };
            code.push_str(&format!(
                "\ncat(\"\\n{title}\\n\")\nprint(exp(cbind({column} = stats::coef({model}), \
                 stats::confint({model}, level = {}))))",
                a.choice("conf")
            ));
        } else {
            code.push_str(&format!(
                "\nprint(stats::confint({model}, level = {}))",
                a.choice("conf")
            ));
        }
        code
    },
};

// ---- Panel model (plm) ----

const PANEL_MODELS: &[Opt] = &[
    Opt {
        value: "within",
        label: "Fixed effects (within)",
    },
    Opt {
        value: "random",
        label: "Random effects",
    },
    Opt {
        value: "between",
        label: "Between (group means)",
    },
    Opt {
        value: "pooling",
        label: "Pooled (ordinary least squares)",
    },
    Opt {
        value: "fd",
        label: "First differences",
    },
];

const EFFECTS: &[Opt] = &[
    Opt {
        value: "individual",
        label: "Panel units",
    },
    Opt {
        value: "time",
        label: "Time periods",
    },
    Opt {
        value: "twoways",
        label: "Both",
    },
];

const PANEL_ERRORS: &[Opt] = &[
    Opt {
        value: "conventional",
        label: "Conventional",
    },
    Opt {
        value: "group",
        label: "Clustered by panel unit",
    },
    Opt {
        value: "time",
        label: "Clustered by time period",
    },
];

const PANEL_ID: Widget = Widget::Variable {
    id: "id",
    label: "Panel unit",
    kinds: GROUPS,
    required: true,
    hint: "What the rows are observations of, followed over time: a person, firm or country.",
};

const PANEL_TIME: Widget = Widget::Variable {
    id: "time",
    label: "Time",
    kinds: GROUPS,
    required: true,
    hint: "The period each row belongs to. Each unit may have each period once.",
};

/// The outcome, predictors, unit and time are four different variables.
fn plm_check(a: &Args) -> Vec<Problem> {
    let y = a.var_text("y");
    let id = a.var_text("id");
    let time = a.var_text("time");
    let x = a.vars("x");
    let mut out = Vec::new();
    if x.contains(&y) {
        out.push(Problem::new(
            "x",
            format!("{y} is the outcome; it cannot also be a predictor."),
        ));
    }
    for (w, v, what) in [("id", id, "panel unit"), ("time", time, "time")] {
        if v == y || x.contains(&v) {
            out.push(Problem::new(
                w,
                format!("{v} is in the model; choose another variable for the {what}."),
            ));
        }
    }
    if !id.is_empty() && id == time {
        out.push(Problem::new(
            "time",
            "The panel unit and the time must be different variables.",
        ));
    }
    out
}

fn panel_check(a: &Args) -> Vec<Problem> {
    let mut out = plm_check(a);
    let model = a.choice("model");
    if a.choice("effect") == "twoways" && model == "between" {
        out.push(Problem::new(
            "effect",
            "Between models take panel-unit or time effects, not both.",
        ));
    }
    if model == "fd" {
        let categorical = a.vars("x").into_iter().find(|v| {
            a.ctx
                .column(v)
                .is_some_and(|c| matches!(c.kind, VarKind::Factor | VarKind::Text))
        });
        if let Some(v) = categorical {
            out.push(Problem::new(
                "x",
                format!(
                    "{v} is categorical; first differences need numbers. Make a 0/1 variable for \
                     each level you need first."
                ),
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

/// `index = c("id", "time")` for plm.
fn index_arg(a: &Args) -> String {
    format!(
        "index = c({}, {})",
        a.var_string("id").unwrap_or_default(),
        a.var_string("time").unwrap_or_default()
    )
}

pub const PANEL_MODEL: Spec = Spec {
    entry: "Panel model",
    menu: "Statistics > Panel models > Panel model",
    about: "The classic panel estimators of a linear model: fixed effects (within), random \
            effects, between, pooled and first differences, with effects for the panel units, \
            the time periods or both. It fits with plm::plm and prints its summary.",
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
            min: 1,
            hint: "With fixed effects, a variable that does not change within a unit is dropped.",
        },
        PANEL_ID,
        PANEL_TIME,
        Widget::Choice {
            id: "model",
            label: "Estimator",
            options: PANEL_MODELS,
            default: "within",
            hint: "",
        },
        Widget::Choice {
            id: "effect",
            label: "Effects for",
            options: EFFECTS,
            default: "individual",
            hint: "Pooled and first-difference models have no effects, so this is ignored for \
                   them.",
        },
        Widget::Choice {
            id: "errors",
            label: "Standard errors",
            options: PANEL_ERRORS,
            default: "conventional",
            hint: "Clustered errors are plm::vcovHC's (Arellano's) with Stata's small-sample \
                   correction, shown in the summary.",
        },
        SAVE,
    ],
    needs: &["plm"],
    styles: BASE,
    check: panel_check,
    code: |a, _| {
        let model = a.name("save");
        let estimator = a.choice("model");
        let y = a.var("y").unwrap_or_default();
        // plm's own model = "fd" differences row by row (its NEWS says so), so a
        // missing value or year inside a unit is differenced across; diff() of a
        // pseries goes by the time index, as Stata's D. does, and the pooled fit
        // of the differences is then Stata's regress D.y D.x.
        let (formula, estimator) = if estimator == "fd" {
            let x = a
                .vars("x")
                .iter()
                .map(|n| format!("diff({})", r_name(n)))
                .collect::<Vec<_>>()
                .join(" + ");
            (format!("diff({y}) ~ {x}"), "pooling")
        } else {
            (format!("{y} ~ {}", rhs(a)), estimator)
        };
        let effect = if estimator == "pooling" {
            String::new()
        } else {
            format!(", effect = {}", r_string(a.choice("effect")))
        };
        let summary = match a.choice("errors") {
            "conventional" => format!("summary({model})"),
            cluster => format!(
                "summary({model}, vcov = plm::vcovHC({model}, cluster = {}, type = \"sss\"))",
                r_string(cluster)
            ),
        };
        format!(
            "{model} <- plm::plm({formula}, data = {}, {}, model = {}{effect})\nprint({summary})",
            a.data(),
            index_arg(a),
            r_string(estimator),
        )
    },
};

// ---- Hausman test ----

pub const HAUSMAN: Spec = Spec {
    entry: "Hausman test (fixed or random effects)",
    menu: "Statistics > Panel models > Hausman test (fixed or random effects)",
    about: "Tests whether random effects can stand in for fixed effects: it fits the model both \
            ways with plm::plm and compares the coefficients with plm::phtest. A small p-value \
            says the two differ, so the fixed-effects estimates are the ones to use.",
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
            min: 1,
            hint: "Only predictors that change within a unit can be compared.",
        },
        PANEL_ID,
        PANEL_TIME,
    ],
    needs: &["plm"],
    styles: BASE,
    check: plm_check,
    code: |a, _| {
        let formula = format!("{} ~ {}", a.var("y").unwrap_or_default(), rhs(a));
        let index = index_arg(a);
        let d = a.data();
        format!(
            "print(local({{\n  fixed <- plm::plm({formula}, data = {d}, {index}, model = \"within\")\n  \
             random <- plm::plm({formula}, data = {d}, {index}, model = \"random\")\n  \
             plm::phtest(fixed, random)\n}}))"
        )
    },
};

pub const SPECS: &[&Spec] = &[
    &FIXED_EFFECTS_REGRESSION,
    &FIXED_EFFECTS_GLM,
    &PANEL_MODEL,
    &HAUSMAN,
];

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
                col("wage", VarKind::Number),
                col("hours", VarKind::Number),
                col("age group", VarKind::Factor),
                col("id", VarKind::Number),
                col("year", VarKind::Number),
                col("firm", VarKind::Text),
                col("visits", VarKind::Number),
                col("days", VarKind::Number),
                col("left", VarKind::Number),
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

    #[test]
    fn fixed_effects_regression_writes_feols() {
        let v = values(
            &FIXED_EFFECTS_REGRESSION,
            &[("y", "wage")],
            &[("x", &["hours", "age group"]), ("fe", &["id", "year"])],
        );
        assert_eq!(
            code(&FIXED_EFFECTS_REGRESSION, &v),
            "model <- fixest::feols(wage ~ hours + `age group` | id + year, data = d, vcov = \"cluster\")\n\
             print(summary(model))\nprint(stats::confint(model, level = 0.95))"
        );
        // a cluster variable takes the place of the choice
        let v = values(
            &FIXED_EFFECTS_REGRESSION,
            &[
                ("y", "wage"),
                ("cluster", "firm"),
                ("save", "fe1"),
                ("conf", "0.9"),
            ],
            &[("x", &["hours"]), ("fe", &["id"])],
        );
        assert_eq!(
            code(&FIXED_EFFECTS_REGRESSION, &v),
            "fe1 <- fixest::feols(wage ~ hours | id, data = d, cluster = ~firm)\n\
             print(summary(fe1))\nprint(stats::confint(fe1, level = 0.9))"
        );
    }

    #[test]
    fn fixed_effects_regression_checks_its_variables() {
        let v = values(
            &FIXED_EFFECTS_REGRESSION,
            &[("y", "wage"), ("errors", "twoway")],
            &[("x", &["wage", "id"]), ("fe", &["id"])],
        );
        let problems = FIXED_EFFECTS_REGRESSION.problems(&ctx(), &v);
        let widgets: Vec<&str> = problems.iter().map(|p| p.widget).collect();
        assert!(widgets.contains(&"x"), "{problems:?}");
        assert!(widgets.contains(&"fe"), "{problems:?}");
        assert!(widgets.contains(&"errors"), "{problems:?}");
        let v = values(
            &FIXED_EFFECTS_REGRESSION,
            &[("y", "wage"), ("save", "d")],
            &[("x", &["hours"]), ("fe", &["id"])],
        );
        assert!(FIXED_EFFECTS_REGRESSION
            .problems(&ctx(), &v)
            .iter()
            .any(|p| p.widget == "save"));
    }

    #[test]
    fn fixed_effects_glm_writes_fepois_and_feglm() {
        let v = values(
            &FIXED_EFFECTS_GLM,
            &[("y", "visits"), ("exposure", "days")],
            &[("x", &["hours"]), ("fe", &["id"])],
        );
        assert_eq!(
            code(&FIXED_EFFECTS_GLM, &v),
            "model <- fixest::fepois(visits ~ hours | id, data = d, offset = ~log(days), vcov = \"cluster\", \
             ssc = fixest::ssc(adj = FALSE))\n\
             print(summary(model))\ncat(\"\\nRate ratios\\n\")\n\
             print(exp(cbind(rate_ratio = stats::coef(model), stats::confint(model, level = 0.95))))"
        );
        // the exposure is for Poisson only
        let v = values(
            &FIXED_EFFECTS_GLM,
            &[("y", "left"), ("family", "logit"), ("exposure", "days")],
            &[("x", &["hours"]), ("fe", &["firm"])],
        );
        assert!(FIXED_EFFECTS_GLM
            .problems(&ctx(), &v)
            .iter()
            .any(|p| p.widget == "exposure"));
        let mut v = values(
            &FIXED_EFFECTS_GLM,
            &[("y", "left"), ("family", "logit"), ("errors", "iid")],
            &[("x", &["hours"]), ("fe", &["firm"])],
        );
        v.set_flag("ratios", false);
        assert_eq!(
            code(&FIXED_EFFECTS_GLM, &v),
            "model <- fixest::feglm(left ~ hours | firm, data = d, family = \"logit\", vcov = \"iid\", \
             ssc = fixest::ssc(adj = FALSE))\n\
             print(summary(model))\nprint(stats::confint(model, level = 0.95))"
        );
    }

    #[test]
    fn panel_model_writes_plm() {
        let v = values(
            &PANEL_MODEL,
            &[("y", "wage"), ("id", "id"), ("time", "year")],
            &[("x", &["hours"])],
        );
        assert_eq!(
            code(&PANEL_MODEL, &v),
            "model <- plm::plm(wage ~ hours, data = d, index = c(\"id\", \"year\"), model = \"within\", effect = \"individual\")\n\
             print(summary(model))"
        );
        let v = values(
            &PANEL_MODEL,
            &[
                ("y", "wage"),
                ("id", "id"),
                ("time", "year"),
                ("model", "random"),
                ("effect", "twoways"),
                ("errors", "group"),
                ("save", "re"),
            ],
            &[("x", &["hours", "age group"])],
        );
        assert_eq!(
            code(&PANEL_MODEL, &v),
            "re <- plm::plm(wage ~ hours + `age group`, data = d, index = c(\"id\", \"year\"), model = \"random\", effect = \"twoways\")\n\
             print(summary(re, vcov = plm::vcovHC(re, cluster = \"group\", type = \"sss\")))"
        );
        // first differences: the pooled fit of diff(), which goes by the time index
        let v = values(
            &PANEL_MODEL,
            &[
                ("y", "wage"),
                ("id", "id"),
                ("time", "year"),
                ("model", "fd"),
                ("errors", "group"),
            ],
            &[("x", &["hours", "days"])],
        );
        assert_eq!(
            code(&PANEL_MODEL, &v),
            "model <- plm::plm(diff(wage) ~ diff(hours) + diff(days), data = d, index = c(\"id\", \"year\"), model = \"pooling\")\n\
             print(summary(model, vcov = plm::vcovHC(model, cluster = \"group\", type = \"sss\")))"
        );
        // pooled: no effect argument
        let v = values(
            &PANEL_MODEL,
            &[
                ("y", "wage"),
                ("id", "id"),
                ("time", "year"),
                ("model", "pooling"),
            ],
            &[("x", &["hours"])],
        );
        assert!(!code(&PANEL_MODEL, &v).contains("effect ="));
    }

    #[test]
    fn panel_model_checks_its_variables() {
        let v = values(
            &PANEL_MODEL,
            &[
                ("y", "wage"),
                ("id", "year"),
                ("time", "year"),
                ("model", "between"),
                ("effect", "twoways"),
            ],
            &[("x", &["hours"])],
        );
        let problems = PANEL_MODEL.problems(&ctx(), &v);
        let widgets: Vec<&str> = problems.iter().map(|p| p.widget).collect();
        assert!(widgets.contains(&"time"), "{problems:?}");
        assert!(widgets.contains(&"effect"), "{problems:?}");
        // first differences of a categorical predictor
        let v = values(
            &PANEL_MODEL,
            &[
                ("y", "wage"),
                ("id", "id"),
                ("time", "year"),
                ("model", "fd"),
            ],
            &[("x", &["hours", "firm"])],
        );
        assert!(PANEL_MODEL
            .problems(&ctx(), &v)
            .iter()
            .any(|p| p.widget == "x" && p.message.contains("firm")));
        let v = values(
            &PANEL_MODEL,
            &[("y", "wage"), ("id", "hours"), ("time", "year")],
            &[("x", &["hours"])],
        );
        assert!(PANEL_MODEL
            .problems(&ctx(), &v)
            .iter()
            .any(|p| p.widget == "id"));
    }

    #[test]
    fn hausman_fits_both_and_tests() {
        let v = values(
            &HAUSMAN,
            &[("y", "wage"), ("id", "id"), ("time", "year")],
            &[("x", &["hours"])],
        );
        assert_eq!(
            code(&HAUSMAN, &v),
            "print(local({\n  fixed <- plm::plm(wage ~ hours, data = d, index = c(\"id\", \"year\"), model = \"within\")\n  \
             random <- plm::plm(wage ~ hours, data = d, index = c(\"id\", \"year\"), model = \"random\")\n  \
             plm::phtest(fixed, random)\n}))"
        );
    }
}
