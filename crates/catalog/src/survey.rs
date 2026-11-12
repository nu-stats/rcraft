//! Statistics menu, survey data (P6.4), through Thomas Lumley's `survey`
//! package: means and totals, proportions and two-way tables with the
//! Rao-Scott test, and regression, each with design-based standard errors.
//! Every dialog declares the design itself (sampling weights, primary
//! sampling units, strata, a finite-population correction) with
//! `survey::svydesign` and keeps it under a name, so its R runs on its own.
//! PSUs are taken as nested in their strata (`nest = TRUE`), as Stata's
//! `svyset` takes them. Confidence intervals use the design's degrees of
//! freedom (PSUs less strata), as Stata's do. None changes the data.

use rcraft_rcodegen::dialog::{Args, Opt, Problem, Spec, TextDefault, TextRule};
use rcraft_rcodegen::dialog::{VarKind, Widget};
use rcraft_rcodegen::r_name;

use crate::data::BASE;

const NUMBERS: &[VarKind] = &[VarKind::Number];
const CATEGORIES: &[VarKind] = &[
    VarKind::Factor,
    VarKind::Text,
    VarKind::Logical,
    VarKind::Number,
];
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
    hint: "Intervals use t with the design's degrees of freedom (PSUs less strata).",
};

const WEIGHTS: Widget = Widget::Variable {
    id: "weights",
    label: "Sampling weights",
    kinds: NUMBERS,
    required: true,
    hint: "How many population members each row stands for (Stata's pweight).",
};

const PSU: Widget = Widget::Variable {
    id: "psu",
    label: "Primary sampling units",
    kinds: GROUPS,
    required: false,
    hint: "Optional: the clusters sampled first (schools, villages). Without them each row is \
           its own unit.",
};

const STRATA: Widget = Widget::Variable {
    id: "strata",
    label: "Strata",
    kinds: GROUPS,
    required: false,
    hint: "Optional: the strata sampled separately. Units are read as numbered within their \
           stratum.",
};

const FPC: Widget = Widget::Variable {
    id: "fpc",
    label: "Finite-population correction",
    kinds: NUMBERS,
    required: false,
    hint: "Optional: the number of units in each stratum's population (or the sampling \
           fraction), for sampling without replacement.",
};

const SAVE_DESIGN: Widget = Widget::Text {
    id: "design",
    label: "Save the design as",
    default: TextDefault::Fixed("design"),
    rule: TextRule::Name,
    hint: "The survey design stays in R's workspace under this name, for further survey:: \
           calls; a name already in use is replaced.",
};

/// `design <- survey::svydesign(...)`, the design every dialog starts from.
fn design_line(a: &Args) -> String {
    let design = a.name("design");
    let ids = a.var("psu").unwrap_or_else(|| "1".into());
    let strata = a
        .var("strata")
        .map_or_else(String::new, |s| format!(", strata = ~{s}"));
    let fpc = a
        .var("fpc")
        .map_or_else(String::new, |f| format!(", fpc = ~{f}"));
    let nest = if a.var("strata").is_some() && a.var("psu").is_some() {
        ", nest = TRUE"
    } else {
        ""
    };
    format!(
        "{design} <- survey::svydesign(ids = ~{ids}{strata}, weights = ~{}{fpc}{nest}, data = {})",
        a.var("weights").unwrap_or_default(),
        a.data()
    )
}

/// `~a + b` of a list widget.
fn formula_of(a: &Args, id: &str) -> String {
    format!(
        "~{}",
        a.vars(id)
            .iter()
            .map(|n| r_name(n))
            .collect::<Vec<_>>()
            .join(" + ")
    )
}

/// The design's variables are four different ones, none of them analysed,
/// and the design is not saved over the data frame.
fn design_check(a: &Args, analysed: &[&str]) -> Vec<Problem> {
    let mut out = Vec::new();
    let roles: Vec<(&str, &str)> = ["weights", "psu", "strata", "fpc"]
        .into_iter()
        .map(|id| (id, a.var_text(id)))
        .filter(|(_, v)| !v.is_empty())
        .collect();
    for (k, (id, v)) in roles.iter().enumerate() {
        if roles
            .get(..k)
            .is_some_and(|r| r.iter().any(|(_, w)| w == v))
        {
            out.push(Problem::new(
                id,
                format!("{v} already has a part in the design; choose another variable."),
            ));
        }
        if analysed.contains(v) {
            out.push(Problem::new(
                id,
                format!("{v} is analysed; it cannot also be part of the design."),
            ));
        }
    }
    if a.name_text("design") == a.ctx.dataset {
        out.push(Problem::new(
            "design",
            format!("{} is the data frame; choose another name.", a.ctx.dataset),
        ));
    }
    out
}

// ---- Means and totals ----

const STATISTICS: &[Opt] = &[
    Opt {
        value: "svymean",
        label: "Means",
    },
    Opt {
        value: "svytotal",
        label: "Totals",
    },
];

fn means_check(a: &Args) -> Vec<Problem> {
    let mut analysed = a.vars("x");
    let by = a.var_text("by");
    if !by.is_empty() {
        analysed.push(by);
    }
    let mut out = design_check(a, &analysed);
    if a.vars("x").contains(&by) {
        out.push(Problem::new(
            "by",
            format!("{by} is analysed; choose another variable for the groups."),
        ));
    }
    out
}

pub const SURVEY_MEANS: Spec = Spec {
    entry: "Survey means and totals",
    menu: "Statistics > Survey data > Means and totals",
    about: "Population means or totals estimated from a survey sample, with standard errors \
            from the design (Taylor linearization), overall or for each group, with \
            survey::svymean, survey::svytotal and survey::svyby. Rows with a missing value in \
            any variable chosen are left out of all of them, as Stata's svy: mean does.",
    widgets: &[
        Widget::Variables {
            id: "x",
            label: "Variables",
            kinds: NUMBERS,
            min: 1,
            hint: "",
        },
        Widget::Choice {
            id: "stat",
            label: "Estimate",
            options: STATISTICS,
            default: "svymean",
            hint: "",
        },
        Widget::Variable {
            id: "by",
            label: "For each group of",
            kinds: GROUPS,
            required: false,
            hint: "Optional: estimates for each group (domains), with the whole design kept.",
        },
        WEIGHTS,
        PSU,
        STRATA,
        FPC,
        CONFIDENCE,
        SAVE_DESIGN,
    ],
    needs: &["survey"],
    styles: BASE,
    check: means_check,
    code: |a, _| {
        let design = a.name("design");
        let stat = a.choice("stat");
        let vars = formula_of(a, "x");
        let estimate = match a.var("by") {
            Some(by) => {
                format!("survey::svyby({vars}, ~{by}, {design}, survey::{stat}, na.rm = TRUE)")
            }
            None => format!("survey::{stat}({vars}, {design}, na.rm = TRUE)"),
        };
        format!(
            "{}\nprint(local({{\n  est <- {estimate}\n  \
             cbind(est = stats::coef(est), se = survey::SE(est), \
             stats::confint(est, level = {}, df = survey::degf({design})))\n}}))",
            design_line(a),
            a.choice("conf")
        )
    },
};

// ---- Proportions and tables ----

fn tables_check(a: &Args) -> Vec<Problem> {
    let row = a.var_text("row");
    let col = a.var_text("col");
    let mut analysed = vec![row];
    if !col.is_empty() {
        analysed.push(col);
    }
    let mut out = design_check(a, &analysed);
    if !col.is_empty() && col == row {
        out.push(Problem::new(
            "col",
            "Choose two different variables for a table.",
        ));
    }
    out
}

pub const SURVEY_TABLES: Spec = Spec {
    entry: "Survey proportions and tables",
    menu: "Statistics > Survey data > Proportions and tables",
    about: "The population share in each category of a variable, with design-based standard \
            errors and intervals (survey::svymean of its levels); with a second variable, the \
            estimated population table (survey::svytable), its proportions, and the Rao-Scott \
            test of independence (survey::svychisq), as Stata's svy: tabulate reports it.",
    widgets: &[
        Widget::Variable {
            id: "row",
            label: "Variable",
            kinds: CATEGORIES,
            required: true,
            hint: "",
        },
        Widget::Variable {
            id: "col",
            label: "Cross with",
            kinds: CATEGORIES,
            required: false,
            hint: "Optional: a second variable for a two-way table and its test.",
        },
        WEIGHTS,
        PSU,
        STRATA,
        FPC,
        CONFIDENCE,
        SAVE_DESIGN,
    ],
    needs: &["survey"],
    styles: BASE,
    check: tables_check,
    code: |a, _| {
        let design = a.name("design");
        let row = a.var("row").unwrap_or_default();
        let mut code = format!(
            "{}\nprint(local({{\n  est <- survey::svymean(~factor({row}), {design}, na.rm = TRUE)\n  \
             cbind(proportion = stats::coef(est), se = survey::SE(est), \
             stats::confint(est, level = {}, df = survey::degf({design})))\n}}))",
            design_line(a),
            a.choice("conf")
        );
        if let Some(col) = a.var("col") {
            code.push_str(&format!(
                "\ntab <- survey::svytable(~{row} + {col}, {design})\nprint(tab)\n\
                 print(prop.table(tab))\nprint(survey::svychisq(~{row} + {col}, {design}))"
            ));
        }
        code
    },
};

// ---- Regression ----

const FAMILIES: &[Opt] = &[
    Opt {
        value: "gaussian",
        label: "Linear",
    },
    Opt {
        value: "quasibinomial",
        label: "Logistic (0 or 1)",
    },
    Opt {
        value: "quasipoisson",
        label: "Poisson (counts)",
    },
];

fn regression_check(a: &Args) -> Vec<Problem> {
    let y = a.var_text("y");
    let mut analysed = a.vars("x");
    analysed.push(y);
    let mut out = design_check(a, &analysed);
    if a.vars("x").contains(&y) {
        out.push(Problem::new(
            "x",
            format!("{y} is the outcome; it cannot also be a predictor."),
        ));
    }
    let save = a.name_text("save");
    if save == a.ctx.dataset {
        out.push(Problem::new(
            "save",
            format!("{} is the data frame; choose another name.", a.ctx.dataset),
        ));
    }
    if save == a.name_text("design") {
        out.push(Problem::new(
            "save",
            "The model and the design need different names.",
        ));
    }
    out
}

pub const SURVEY_REGRESSION: Spec = Spec {
    entry: "Survey regression",
    menu: "Statistics > Survey data > Regression",
    about: "A linear, logistic or Poisson model fitted with the sampling weights, with standard \
            errors from the design, with survey::svyglm. Logistic and Poisson models use the \
            quasi families, as survey recommends, so the weights need not be whole numbers; \
            their estimates are the usual ones.",
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
            hint: "A categorical predictor gets one coefficient per level after the first.",
        },
        Widget::Choice {
            id: "family",
            label: "Model",
            options: FAMILIES,
            default: "gaussian",
            hint: "",
        },
        Widget::Check {
            id: "ratios",
            label: "Show odds or rate ratios (logistic or Poisson)",
            default: true,
            hint: "",
        },
        WEIGHTS,
        PSU,
        STRATA,
        FPC,
        CONFIDENCE,
        SAVE_DESIGN,
        Widget::Text {
            id: "save",
            label: "Save the model as",
            default: TextDefault::Fixed("model"),
            rule: TextRule::Name,
            hint: "The fitted model stays in R's workspace under this name.",
        },
    ],
    needs: &["survey"],
    styles: BASE,
    check: regression_check,
    code: |a, _| {
        let design = a.name("design");
        let model = a.name("save");
        let family = a.choice("family");
        let rhs = a
            .vars("x")
            .iter()
            .map(|n| r_name(n))
            .collect::<Vec<_>>()
            .join(" + ");
        // glm's default convergence (epsilon = 1e-8) stops a few digits short of the
        // maximum; Stata's svy: logit and svy: poisson iterate to it. The tighter
        // tolerance costs an iteration or two and brings the estimates and standard
        // errors to Stata's within 1e-10.
        let control = if family == "gaussian" {
            ""
        } else {
            ", control = stats::glm.control(epsilon = 1e-14, maxit = 100)"
        };
        let mut code = format!(
            "{}\n{model} <- survey::svyglm({} ~ {rhs}, design = {design}, family = stats::{family}(){control})\n\
             print(summary({model}))",
            design_line(a),
            a.var("y").unwrap_or_default(),
        );
        let ci = format!(
            "stats::confint({model}, level = {}, df = survey::degf({design}))",
            a.choice("conf")
        );
        if family != "gaussian" && a.flag("ratios") {
            let (title, column) = if family == "quasibinomial" {
                ("Odds ratios", "odds_ratio")
            } else {
                ("Rate ratios", "rate_ratio")
            };
            code.push_str(&format!(
                "\ncat(\"\\n{title}\\n\")\nprint(exp(cbind({column} = stats::coef({model}), {ci})))"
            ));
        } else {
            code.push_str(&format!("\nprint({ci})"));
        }
        code
    },
};

pub const SPECS: &[&Spec] = &[&SURVEY_MEANS, &SURVEY_TABLES, &SURVEY_REGRESSION];

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
                col("income", VarKind::Number),
                col("hours", VarKind::Number),
                col("region", VarKind::Factor),
                col("smokes", VarKind::Number),
                col("wt", VarKind::Number),
                col("school id", VarKind::Number),
                col("stratum", VarKind::Number),
                col("pop", VarKind::Number),
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
    fn means_declare_the_design_and_estimate() {
        let v = values(
            &SURVEY_MEANS,
            &[
                ("weights", "wt"),
                ("psu", "school id"),
                ("strata", "stratum"),
            ],
            &[("x", &["income", "hours"])],
        );
        assert_eq!(
            code(&SURVEY_MEANS, &v),
            "design <- survey::svydesign(ids = ~`school id`, strata = ~stratum, weights = ~wt, nest = TRUE, data = d)\n\
             print(local({\n  est <- survey::svymean(~income + hours, design, na.rm = TRUE)\n  \
             cbind(est = stats::coef(est), se = survey::SE(est), stats::confint(est, level = 0.95, \
             df = survey::degf(design)))\n}))"
        );
        // weights alone; totals by group with a correction
        let v = values(
            &SURVEY_MEANS,
            &[
                ("weights", "wt"),
                ("stat", "svytotal"),
                ("by", "region"),
                ("fpc", "pop"),
            ],
            &[("x", &["income"])],
        );
        let c = code(&SURVEY_MEANS, &v);
        assert!(
            c.starts_with(
                "design <- survey::svydesign(ids = ~1, weights = ~wt, fpc = ~pop, data = d)\n"
            ),
            "{c}"
        );
        assert!(
            c.contains(
                "est <- survey::svyby(~income, ~region, design, survey::svytotal, na.rm = TRUE)"
            ),
            "{c}"
        );
    }

    #[test]
    fn the_design_takes_each_variable_once() {
        let v = values(
            &SURVEY_MEANS,
            &[("weights", "wt"), ("psu", "wt"), ("strata", "income")],
            &[("x", &["income"])],
        );
        let problems = SURVEY_MEANS.problems(&ctx(), &v);
        let widgets: Vec<&str> = problems.iter().map(|p| p.widget).collect();
        assert!(widgets.contains(&"psu"), "{problems:?}");
        assert!(widgets.contains(&"strata"), "{problems:?}");
    }

    #[test]
    fn tables_give_proportions_and_the_rao_scott_test() {
        let v = values(
            &SURVEY_TABLES,
            &[
                ("row", "region"),
                ("col", "smokes"),
                ("weights", "wt"),
                ("psu", "school id"),
            ],
            &[],
        );
        assert_eq!(
            code(&SURVEY_TABLES, &v),
            "design <- survey::svydesign(ids = ~`school id`, weights = ~wt, data = d)\n\
             print(local({\n  est <- survey::svymean(~factor(region), design, na.rm = TRUE)\n  \
             cbind(proportion = stats::coef(est), se = survey::SE(est), stats::confint(est, \
             level = 0.95, df = survey::degf(design)))\n}))\n\
             tab <- survey::svytable(~region + smokes, design)\nprint(tab)\nprint(prop.table(tab))\n\
             print(survey::svychisq(~region + smokes, design))"
        );
    }

    #[test]
    fn regression_writes_svyglm() {
        let v = values(
            &SURVEY_REGRESSION,
            &[
                ("y", "smokes"),
                ("family", "quasibinomial"),
                ("weights", "wt"),
                ("strata", "stratum"),
            ],
            &[("x", &["income", "region"])],
        );
        assert_eq!(
            code(&SURVEY_REGRESSION, &v),
            "design <- survey::svydesign(ids = ~1, strata = ~stratum, weights = ~wt, data = d)\n\
             model <- survey::svyglm(smokes ~ income + region, design = design, family = stats::quasibinomial(), \
             control = stats::glm.control(epsilon = 1e-14, maxit = 100))\n\
             print(summary(model))\ncat(\"\\nOdds ratios\\n\")\n\
             print(exp(cbind(odds_ratio = stats::coef(model), stats::confint(model, level = 0.95, \
             df = survey::degf(design)))))"
        );
        let v = values(
            &SURVEY_REGRESSION,
            &[("y", "income"), ("weights", "wt"), ("save", "design")],
            &[("x", &["hours"])],
        );
        assert!(SURVEY_REGRESSION
            .problems(&ctx(), &v)
            .iter()
            .any(|p| p.widget == "save"));
    }
}
