//! Statistics menu, instrumental variables and selection models (P6.6):
//! two-stage least squares through `fixest` (with fixed effects if asked, its
//! first-stage F, Wu-Hausman and Sargan statistics), and Heckman's selection
//! model through `sampleSelection`, by maximum likelihood or in two steps.
//! Each fits its model, keeps it in the workspace under the name the dialog
//! asks for and prints the package's own summary. None changes the data.

use rcraft_rcodegen::dialog::{Args, Opt, Problem, Spec, TextDefault, TextRule};
use rcraft_rcodegen::dialog::{VarKind, Widget};
use rcraft_rcodegen::{r_name, r_string};

use crate::data::BASE;

const NUMBERS: &[VarKind] = &[VarKind::Number];
const SELECTED: &[VarKind] = &[VarKind::Number, VarKind::Logical];
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

const SAVE: Widget = Widget::Text {
    id: "save",
    label: "Save the model as",
    default: TextDefault::Fixed("model"),
    rule: TextRule::Name,
    hint: "The fitted model stays in R's workspace under this name; a name already in use is \
           replaced.",
};

fn joined(names: &[&str]) -> String {
    names
        .iter()
        .map(|n| r_name(n))
        .collect::<Vec<_>>()
        .join(" + ")
}

/// Each variable in one role only, and the model not saved over the data.
fn roles(
    a: &Args,
    lists: &[(&'static str, &str)],
    single: &[(&'static str, &str)],
) -> Vec<Problem> {
    let mut out = Vec::new();
    let mut seen: Vec<(String, &str)> = Vec::new();
    for (id, what) in single {
        let v = a.var_text(id);
        if v.is_empty() {
            continue;
        }
        if let Some((_, first)) = seen.iter().find(|(s, _)| s == v) {
            out.push(Problem::new(
                id,
                format!("{v} is already the {first}; choose another variable."),
            ));
        } else {
            seen.push((v.to_string(), what));
        }
    }
    for (id, what) in lists {
        for v in a.vars(id) {
            if let Some((_, first)) = seen.iter().find(|(s, _)| s == v) {
                out.push(Problem::new(
                    id,
                    format!("{v} is already the {first}; each variable takes one role."),
                ));
            } else {
                seen.push((v.to_string(), what));
            }
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

// ---- Two-stage least squares ----

const IV_ERRORS: &[Opt] = &[
    Opt {
        value: "iid",
        label: "Conventional",
    },
    Opt {
        value: "hetero",
        label: "Robust to heteroskedasticity",
    },
];

fn iv_check(a: &Args) -> Vec<Problem> {
    let mut out = roles(
        a,
        &[
            ("endog", "an endogenous predictor"),
            ("instruments", "an instrument"),
            ("x", "an exogenous predictor"),
            ("fe", "a fixed effect"),
        ],
        &[("y", "outcome")],
    );
    let (endog, instruments) = (a.vars("endog").len(), a.vars("instruments").len());
    if instruments < endog {
        out.push(Problem::new(
            "instruments",
            format!(
                "{endog} endogenous predictors need at least {endog} instruments; there are \
                 {instruments}."
            ),
        ));
    }
    out
}

pub const IV_REGRESSION: Spec = Spec {
    entry: "Two-stage least squares",
    menu: "Statistics > Instrumental variables > Two-stage least squares",
    about: "A linear model in which some predictors are endogenous (correlated with the error), \
            estimated by two-stage least squares with instruments that move them but not the \
            outcome otherwise, as Stata's ivregress 2sls. It fits with fixest::feols, optionally \
            with fixed effects absorbed (as xtivreg, fe), and prints the summary with the \
            first-stage F, the Wu-Hausman test of endogeneity and Sargan's test of the \
            over-identifying restrictions.",
    widgets: &[
        Widget::Variable {
            id: "y",
            label: "Outcome",
            kinds: NUMBERS,
            required: true,
            hint: "",
        },
        Widget::Variables {
            id: "endog",
            label: "Endogenous predictors",
            kinds: NUMBERS,
            min: 1,
            hint: "The predictors to instrument.",
        },
        Widget::Variables {
            id: "instruments",
            label: "Instruments",
            kinds: PREDICTORS,
            min: 1,
            hint: "Excluded instruments: variables that move the endogenous predictors but not \
                   the outcome except through them. At least one for each endogenous predictor.",
        },
        Widget::Variables {
            id: "x",
            label: "Exogenous predictors",
            kinds: PREDICTORS,
            min: 0,
            hint: "Optional: the other predictors. They are used as their own instruments.",
        },
        Widget::Variables {
            id: "fe",
            label: "Fixed effects",
            kinds: GROUPS,
            min: 0,
            hint: "Optional: groupings whose intercepts are absorbed, as xtivreg, fe.",
        },
        Widget::Choice {
            id: "errors",
            label: "Standard errors",
            options: IV_ERRORS,
            default: "iid",
            hint: "A variable under Cluster by takes the place of this choice.",
        },
        Widget::Variable {
            id: "cluster",
            label: "Cluster by",
            kinds: GROUPS,
            required: false,
            hint: "Optional: cluster the standard errors by this variable.",
        },
        Widget::Check {
            id: "small",
            label: "Small-sample statistics",
            default: true,
            hint:
                "On: Stata's ivregress with the small option (and xtivreg, fe): degrees-of-freedom \
                   adjustments and t tests. Off: ivregress's default large-sample robust and \
                   clustered errors (conventional errors still divide by N - 1, not N).",
        },
        Widget::Check {
            id: "first",
            label: "Show the first stage",
            default: false,
            hint: "",
        },
        Widget::Choice {
            id: "conf",
            label: "Confidence level",
            options: LEVELS,
            default: "0.95",
            hint: "For the confidence intervals printed after the summary.",
        },
        SAVE,
    ],
    needs: &["fixest"],
    styles: BASE,
    check: iv_check,
    code: |a, _| {
        let model = a.name("save");
        let exog = match a.vars("x") {
            x if x.is_empty() => "1".to_string(),
            x => joined(&x),
        };
        let fe = match a.vars("fe") {
            fe if fe.is_empty() => String::new(),
            fe => format!(" | {}", joined(&fe)),
        };
        let errors = match a.var("cluster") {
            Some(c) => format!("cluster = ~{c}"),
            None => format!("vcov = {}", r_string(a.choice("errors"))),
        };
        let ssc = if a.flag("small") {
            ""
        } else {
            ", ssc = fixest::ssc(adj = FALSE, cluster.adj = FALSE)"
        };
        let stage = if a.flag("first") { ", stage = 1:2" } else { "" };
        let mut code = format!(
            "{model} <- fixest::feols({} ~ {exog}{fe} | {} ~ {}, data = {}, {errors}{ssc})\n\
             print(summary({model}{stage}))",
            a.var("y").unwrap_or_default(),
            joined(&a.vars("endog")),
            joined(&a.vars("instruments")),
            a.data(),
        );
        // the summary's first-stage F is the conventional one; with robust or
        // clustered errors Stata's estat firststage reports the Wald form
        if a.var("cluster").is_some() || a.choice("errors") != "iid" {
            code.push_str(&format!("\nprint(fixest::fitstat({model}, ~ ivwald))"));
        }
        code.push_str(&format!(
            "\nprint(stats::confint({model}, level = {}))",
            a.choice("conf")
        ));
        code
    },
};

// ---- Heckman selection model ----

const METHODS: &[Opt] = &[
    Opt {
        value: "ml",
        label: "Maximum likelihood",
    },
    Opt {
        value: "2step",
        label: "Two steps (Heckman's)",
    },
];

fn heckman_check(a: &Args) -> Vec<Problem> {
    roles(
        a,
        &[],
        &[("y", "outcome"), ("selected", "selection indicator")],
    )
    .into_iter()
    .chain(a.vars("x").contains(&a.var_text("y")).then(|| {
        Problem::new(
            "x",
            format!(
                "{} is the outcome; it cannot also be a predictor.",
                a.var_text("y")
            ),
        )
    }))
    .chain(a.vars("z").contains(&a.var_text("y")).then(|| {
        Problem::new(
            "z",
            format!(
                "{} is the outcome; it cannot also be a predictor.",
                a.var_text("y")
            ),
        )
    }))
    .collect()
}

pub const HECKMAN: Spec = Spec {
    entry: "Heckman selection model",
    menu: "Statistics > Selection models > Heckman selection model",
    about: "A linear model of an outcome that is seen only for some rows (wages only for those in \
            work), with a probit model of which rows are seen, estimated jointly so that the \
            selection does not bias the coefficients: Heckman's model, as Stata's heckman. It \
            fits with sampleSelection::selection, by maximum likelihood (Stata's default, fitted \
            to Stata's precision) or in Heckman's two steps, and prints the summary with rho, \
            the correlation of the two errors.",
    widgets: &[
        Widget::Variable {
            id: "y",
            label: "Outcome",
            kinds: NUMBERS,
            required: true,
            hint: "Seen only for the selected rows; its value elsewhere is not used.",
        },
        Widget::Variables {
            id: "x",
            label: "Outcome predictors",
            kinds: PREDICTORS,
            min: 1,
            hint: "",
        },
        Widget::Variable {
            id: "selected",
            label: "Selected (1 or TRUE)",
            kinds: SELECTED,
            required: false,
            hint: "Optional: which rows are seen. Left empty, a row is selected when its outcome \
                   is not missing, as Stata's heckman does.",
        },
        Widget::Variables {
            id: "z",
            label: "Selection predictors",
            kinds: PREDICTORS,
            min: 1,
            hint: "What decides whether a row is seen. Include at least one variable that is not \
                   an outcome predictor, or the model rests on the normal curve's shape alone.",
        },
        Widget::Choice {
            id: "method",
            label: "Estimation",
            options: METHODS,
            default: "ml",
            hint: "",
        },
        SAVE,
    ],
    needs: &["sampleSelection"],
    styles: BASE,
    check: heckman_check,
    code: |a, _| {
        let model = a.name("save");
        let y = a.var("y").unwrap_or_default();
        let selected = a.var("selected").unwrap_or_else(|| format!("!is.na({y})"));
        // maxLik's default stopping rule (the gradient below 1e-6) leaves the
        // estimates about 1e-8 from the maximum; these reach it, as Stata does
        let method = if a.choice("method") == "ml" {
            "method = \"ml\", gradtol = 1e-12, reltol = 1e-16, tol = 1e-14"
        } else {
            "method = \"2step\""
        };
        format!(
            "{model} <- sampleSelection::selection({selected} ~ {}, {y} ~ {}, data = {}, {method})\n\
             print(summary({model}))",
            joined(&a.vars("z")),
            joined(&a.vars("x")),
            a.data(),
        )
    },
};

pub const SPECS: &[&Spec] = &[&IV_REGRESSION, &HECKMAN];

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
                col("educ", VarKind::Number),
                col("x", VarKind::Number),
                col("z1", VarKind::Number),
                col("z2", VarKind::Number),
                col("region", VarKind::Number),
                col("works", VarKind::Number),
                col("earn", VarKind::Number),
                col("kids", VarKind::Number),
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

    #[test]
    fn iv_writes_feols_with_its_instruments() {
        let v = values(
            &IV_REGRESSION,
            &[("y", "wage")],
            &[
                ("endog", &["educ"]),
                ("instruments", &["z1", "z2"]),
                ("x", &["x"]),
            ],
        );
        assert_eq!(
            code(&IV_REGRESSION, &v),
            "model <- fixest::feols(wage ~ x | educ ~ z1 + z2, data = d, vcov = \"iid\")\n\
             print(summary(model))\nprint(stats::confint(model, level = 0.95))"
        );
        // fixed effects, a cluster, large-sample errors, the first stage
        let mut v = values(
            &IV_REGRESSION,
            &[("y", "wage"), ("cluster", "region"), ("save", "iv")],
            &[
                ("endog", &["educ"]),
                ("instruments", &["z1", "home town"]),
                ("fe", &["region"]),
            ],
        );
        v.set_flag("small", false);
        v.set_flag("first", true);
        assert_eq!(
            code(&IV_REGRESSION, &v),
            "iv <- fixest::feols(wage ~ 1 | region | educ ~ z1 + `home town`, data = d, \
             cluster = ~region, ssc = fixest::ssc(adj = FALSE, cluster.adj = FALSE))\n\
             print(summary(iv, stage = 1:2))\nprint(fixest::fitstat(iv, ~ ivwald))\n\
             print(stats::confint(iv, level = 0.95))"
        );
    }

    #[test]
    fn iv_checks_roles_and_counts() {
        let v = values(
            &IV_REGRESSION,
            &[("y", "wage")],
            &[
                ("endog", &["educ", "x"]),
                ("instruments", &["z1"]),
                ("x", &["z1"]),
            ],
        );
        let problems = IV_REGRESSION.problems(&ctx(), &v);
        let widgets: Vec<&str> = problems.iter().map(|p| p.widget).collect();
        assert!(widgets.contains(&"instruments"), "{problems:?}");
        assert!(widgets.contains(&"x"), "{problems:?}");
        let v = values(
            &IV_REGRESSION,
            &[("y", "wage"), ("save", "d")],
            &[("endog", &["educ"]), ("instruments", &["z1"])],
        );
        assert!(IV_REGRESSION
            .problems(&ctx(), &v)
            .iter()
            .any(|p| p.widget == "save"));
    }

    #[test]
    fn heckman_writes_selection() {
        let v = values(
            &HECKMAN,
            &[("y", "earn"), ("selected", "works")],
            &[("x", &["x", "z1"]), ("z", &["x", "kids", "z1"])],
        );
        assert_eq!(
            code(&HECKMAN, &v),
            "model <- sampleSelection::selection(works ~ x + kids + z1, earn ~ x + z1, data = d, \
             method = \"ml\", gradtol = 1e-12, reltol = 1e-16, tol = 1e-14)\nprint(summary(model))"
        );
        // no indicator: selected where the outcome is not missing
        let v = values(
            &HECKMAN,
            &[("y", "earn"), ("method", "2step"), ("save", "h")],
            &[("x", &["x"]), ("z", &["kids", "home town"])],
        );
        assert_eq!(
            code(&HECKMAN, &v),
            "h <- sampleSelection::selection(!is.na(earn) ~ kids + `home town`, earn ~ x, data = d, \
             method = \"2step\")\nprint(summary(h))"
        );
        let v = values(
            &HECKMAN,
            &[("y", "earn"), ("selected", "earn")],
            &[("x", &["earn"]), ("z", &["kids"])],
        );
        let problems = HECKMAN.problems(&ctx(), &v);
        let widgets: Vec<&str> = problems.iter().map(|p| p.widget).collect();
        assert!(widgets.contains(&"selected"), "{problems:?}");
        assert!(widgets.contains(&"x"), "{problems:?}");
    }
}
