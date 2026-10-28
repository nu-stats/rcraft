//! Statistics menu, generalised linear models (P4.4, first part): logistic and
//! Poisson regression through `stats::glm`. Base R only. Each fits a model,
//! keeps it in the workspace under a name the dialog asks for, prints R's own
//! summary and, if wanted, the exponentiated coefficients (odds ratios or rate
//! ratios) with their confidence intervals. None changes the data.
//!
//! Negative binomial, ordinal and multinomial models need the MASS and nnet
//! packages; they come with the packages panel of P1A.

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
    hint: "For the intervals around the ratios.",
};

const PREDICTOR_LIST: Widget = Widget::Variables {
    id: "x",
    label: "Predictors",
    kinds: PREDICTORS,
    min: 1,
    hint: "A categorical predictor gets one coefficient per level after the first.",
};

const SAVE: Widget = Widget::Text {
    id: "save",
    label: "Save the model as",
    default: TextDefault::Fixed("model"),
    rule: TextRule::Name,
    hint: "The fitted model stays in R's workspace under this name; a name already in use is \
           replaced.",
};

/// Checks shared by the dialogs: the outcome is not a predictor, and the model
/// is not saved over the data frame.
fn common_check(a: &Args) -> Vec<Problem> {
    let y = a.var_text("y");
    let mut out = Vec::new();
    if a.vars("x").contains(&y) {
        out.push(Problem::new(
            "x",
            format!("{y} is the outcome; it cannot also be a predictor."),
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

fn rhs(a: &Args) -> String {
    a.vars("x")
        .iter()
        .map(|n| r_name(n))
        .collect::<Vec<_>>()
        .join(" + ")
}

// ---- Logistic model ----

const LINKS: &[Opt] = &[
    Opt {
        value: "logit",
        label: "Logit (logistic regression)",
    },
    Opt {
        value: "probit",
        label: "Probit",
    },
    Opt {
        value: "cloglog",
        label: "Complementary log-log",
    },
];

pub const LOGISTIC: Spec = Spec {
    entry: "Logistic model",
    menu: "Statistics > Generalised models > Logistic model",
    about: "Model a yes/no outcome from one or more predictors. The outcome must have exactly \
            two different values; for a factor or text, the first level counts as no and the \
            other as yes. Rows missing any variable used are left out.",
    widgets: &[
        Widget::Variable {
            id: "y",
            label: "Outcome",
            kinds: &[
                VarKind::Number,
                VarKind::Logical,
                VarKind::Factor,
                VarKind::Text,
            ],
            required: true,
            hint: "",
        },
        PREDICTOR_LIST,
        Widget::Choice {
            id: "link",
            label: "Link",
            options: LINKS,
            default: "logit",
            hint: "",
        },
        Widget::Check {
            id: "ratios",
            label: "Show odds ratios with confidence intervals",
            default: true,
            hint: "Only for the logit link.",
        },
        CONFIDENCE,
        SAVE,
    ],
    needs: &[],
    styles: BASE,
    check: common_check,
    code: |a, _| {
        let (data, model) = (a.data(), a.name("save"));
        let outcome = a.var_string("y").unwrap_or_default();
        let link = a.choice("link");
        let mut code = format!(
            "stopifnot(\"the outcome must have exactly two different values\" = \
             length(unique(stats::na.omit({data}[[{outcome}]]))) == 2)\n\
             {model} <- stats::glm({} ~ {}, family = stats::binomial(link = {}), data = {data})\n\
             print(summary({model}))",
            a.var("y").unwrap_or_default(),
            rhs(a),
            r_string(link),
        );
        if link == "logit" && a.flag("ratios") {
            code.push_str(&format!(
                "\ncat(\"\\nOdds ratios\\n\")\nprint(exp(cbind(odds_ratio = stats::coef({model}), \
                 stats::confint({model}, level = {}))))",
                a.choice("conf")
            ));
        }
        code
    },
};

// ---- Poisson model ----

const POISSON_FAMILIES: &[Opt] = &[
    Opt {
        value: "poisson",
        label: "Poisson",
    },
    Opt {
        value: "quasipoisson",
        label: "Quasi-Poisson (counts more spread out than Poisson)",
    },
];

fn poisson_check(a: &Args) -> Vec<Problem> {
    let mut out = common_check(a);
    let y = a.var_text("y");
    if a.var_text("exposure") == y {
        out.push(Problem::new(
            "exposure",
            format!("{y} is the outcome; choose another variable for the exposure."),
        ));
    }
    out
}

pub const POISSON: Spec = Spec {
    entry: "Poisson model",
    menu: "Statistics > Generalised models > Poisson model",
    about: "Model a count outcome from one or more predictors. The counts must be whole numbers \
            of zero or more. Rows missing any variable used are left out.",
    widgets: &[
        Widget::Variable {
            id: "y",
            label: "Outcome (counts)",
            kinds: NUMBERS,
            required: true,
            hint: "",
        },
        PREDICTOR_LIST,
        Widget::Variable {
            id: "exposure",
            label: "Exposure",
            kinds: NUMBERS,
            required: false,
            hint: "Optional: how long or how much each row was observed. The model then \
                   describes rates; its log is added as an offset. Must be above zero.",
        },
        Widget::Choice {
            id: "family",
            label: "Family",
            options: POISSON_FAMILIES,
            default: "poisson",
            hint: "",
        },
        Widget::Check {
            id: "ratios",
            label: "Show rate ratios with confidence intervals",
            default: true,
            hint: "",
        },
        CONFIDENCE,
        SAVE,
    ],
    needs: &[],
    styles: BASE,
    check: poisson_check,
    code: |a, _| {
        let model = a.name("save");
        let offset = a
            .var("exposure")
            .map_or_else(String::new, |e| format!(", offset = log({e})"));
        let mut code = format!(
            "{model} <- stats::glm({} ~ {}, family = stats::{}(), data = {}{offset})\n\
             print(summary({model}))",
            a.var("y").unwrap_or_default(),
            rhs(a),
            a.choice("family"),
            a.data(),
        );
        if a.flag("ratios") {
            code.push_str(&format!(
                "\ncat(\"\\nRate ratios\\n\")\nprint(exp(cbind(rate_ratio = stats::coef({model}), \
                 stats::confint({model}, level = {}))))",
                a.choice("conf")
            ));
        }
        code
    },
};

// ---- Negative binomial model ----

pub const NEGATIVE_BINOMIAL: Spec = Spec {
    entry: "Negative binomial model",
    menu: "Statistics > Generalised models > Negative binomial model",
    about: "Model a count outcome that varies more than a Poisson model allows. It fits with \
            MASS::glm.nb, which estimates the extra spread (theta) along with the \
            coefficients. The counts must be whole numbers of zero or more.",
    widgets: &[
        Widget::Variable {
            id: "y",
            label: "Outcome (counts)",
            kinds: NUMBERS,
            required: true,
            hint: "",
        },
        PREDICTOR_LIST,
        Widget::Variable {
            id: "exposure",
            label: "Exposure",
            kinds: NUMBERS,
            required: false,
            hint: "Optional: how long or how much each row was observed. The model then \
                   describes rates; its log is added as an offset. Must be above zero.",
        },
        Widget::Check {
            id: "ratios",
            label: "Show rate ratios with confidence intervals",
            default: true,
            hint: "",
        },
        CONFIDENCE,
        SAVE,
    ],
    needs: &["MASS"],
    styles: BASE,
    check: poisson_check,
    code: |a, _| {
        let model = a.name("save");
        let offset = a
            .var("exposure")
            .map_or_else(String::new, |e| format!(" + offset(log({e}))"));
        let mut code = format!(
            "{model} <- MASS::glm.nb({} ~ {}{offset}, data = {})\nprint(summary({model}))",
            a.var("y").unwrap_or_default(),
            rhs(a),
            a.data(),
        );
        if a.flag("ratios") {
            code.push_str(&format!(
                "\ncat(\"\\nRate ratios\\n\")\nprint(exp(cbind(rate_ratio = stats::coef({model}), \
                 stats::confint({model}, level = {}))))",
                a.choice("conf")
            ));
        }
        code
    },
};

// ---- Ordinal model ----

const ORDINAL_LINKS: &[Opt] = &[
    Opt {
        value: "logistic",
        label: "Logit (proportional odds)",
    },
    Opt {
        value: "probit",
        label: "Probit",
    },
    Opt {
        value: "cloglog",
        label: "Complementary log-log",
    },
];

pub const ORDINAL: Spec = Spec {
    entry: "Ordinal model",
    menu: "Statistics > Generalised models > Ordinal model",
    about: "Model an outcome with ordered categories (for example low, middle, high) from one or \
            more predictors, with MASS::polr. The categories are put in the order of the \
            variable's levels, lowest first; text is ordered alphabetically and numbers by size, \
            and the order used is printed. Rows missing any variable used are left out.",
    widgets: &[
        Widget::Variable {
            id: "y",
            label: "Outcome (ordered categories)",
            kinds: &[VarKind::Factor, VarKind::Text, VarKind::Number],
            required: true,
            hint:
                "For a factor, set the order of its levels first (Data > Factor > Reorder levels).",
        },
        PREDICTOR_LIST,
        Widget::Choice {
            id: "link",
            label: "Link",
            options: ORDINAL_LINKS,
            default: "logistic",
            hint: "",
        },
        Widget::Check {
            id: "ratios",
            label: "Show odds ratios with confidence intervals",
            default: true,
            hint: "Only for the logit link.",
        },
        Widget::Check {
            id: "pvalues",
            label: "Show p-values",
            default: true,
            hint: "From the normal approximation to each t value.",
        },
        CONFIDENCE,
        SAVE,
    ],
    needs: &["MASS"],
    styles: BASE,
    check: common_check,
    code: |a, _| {
        let (data, model) = (a.data(), a.name("save"));
        let (y, link) = (a.var("y").unwrap_or_default(), a.choice("link"));
        let ys = a.var_string("y").unwrap_or_default();
        let mut code = format!(
            "{model} <- MASS::polr(ordered({y}) ~ {}, data = {data}, method = {}, Hess = TRUE)\n\
             cat(\"Outcome order, lowest first: \", \
             paste(levels(ordered({data}[[{ys}]])), collapse = \" < \"), \"\\n\", sep = \"\")\n\
             print(summary({model}))",
            rhs(a),
            r_string(link),
        );
        if a.flag("pvalues") {
            code.push_str(&format!(
                "\ncat(\"\\nCoefficients with p-values\\n\")\n\
                 print(cbind(stats::coef(summary({model})), \"p value\" = 2 * stats::pnorm(\
                 abs(stats::coef(summary({model}))[, \"t value\"]), lower.tail = FALSE)))"
            ));
        }
        if link == "logistic" && a.flag("ratios") {
            code.push_str(&format!(
                "\ncat(\"\\nOdds ratios\\n\")\nprint(exp(cbind(odds_ratio = stats::coef({model}), \
                 stats::confint({model}, level = {}))))",
                a.choice("conf")
            ));
        }
        code
    },
};

// ---- Multinomial model ----

pub const MULTINOMIAL: Spec = Spec {
    entry: "Multinomial model",
    menu: "Statistics > Generalised models > Multinomial model",
    about: "Model an outcome with three or more unordered categories from one or more \
            predictors, with nnet::multinom. Each category other than the reference gets its own \
            set of coefficients, compared with the reference. Rows missing any variable used \
            are left out.",
    widgets: &[
        Widget::Variable {
            id: "y",
            label: "Outcome (categories)",
            kinds: &[VarKind::Factor, VarKind::Text],
            required: true,
            hint: "",
        },
        PREDICTOR_LIST,
        Widget::Text {
            id: "reference",
            label: "Reference category",
            default: TextDefault::Empty,
            rule: TextRule::Any,
            hint: "Optional: the category the others are compared with. Leave empty to use the \
                   first level. R refuses a category that is not there.",
        },
        Widget::Check {
            id: "ratios",
            label: "Show relative risk ratios with confidence intervals",
            default: true,
            hint: "",
        },
        Widget::Check {
            id: "pvalues",
            label: "Show p-values",
            default: true,
            hint: "From the normal approximation to each z value.",
        },
        CONFIDENCE,
        SAVE,
    ],
    needs: &["nnet"],
    styles: BASE,
    check: common_check,
    code: |a, _| {
        let (data, model) = (a.data(), a.name("save"));
        let outcome = a.var_string("y").unwrap_or_default();
        let y = a.var("y").unwrap_or_default();
        let reference = a.values.text("reference").trim();
        let response = if reference.is_empty() {
            format!("factor({y})")
        } else {
            format!("stats::relevel(factor({y}), ref = {})", r_string(reference))
        };
        let mut code = format!(
            "stopifnot(\"the outcome must have at least three different values\" = \
             length(unique(stats::na.omit({data}[[{outcome}]]))) >= 3)\n\
             {model} <- nnet::multinom({response} ~ {}, data = {data}, trace = FALSE)\n\
             print(summary({model}))",
            rhs(a),
        );
        if a.flag("pvalues") {
            code.push_str(&format!(
                "\ncat(\"\\np-values\\n\")\nprint(2 * stats::pnorm(abs(summary({model})$coefficients / \
                 summary({model})$standard.errors), lower.tail = FALSE))"
            ));
        }
        if a.flag("ratios") {
            code.push_str(&format!(
                "\ncat(\"\\nRelative risk ratios\\n\")\nprint(exp(stats::coef({model})))\n\
                 cat(\"\\nConfidence intervals for the ratios\\n\")\n\
                 print(exp(stats::confint({model}, level = {})))",
                a.choice("conf")
            ));
        }
        code
    },
};

/// The dialogs of this module, in menu order.
pub const SPECS: &[&Spec] = &[
    &LOGISTIC,
    &POISSON,
    &NEGATIVE_BINOMIAL,
    &ORDINAL,
    &MULTINOMIAL,
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
            dataset: "my data".into(),
            columns: vec![
                col("won game", VarKind::Logical),
                col("age", VarKind::Number),
                col("arm", VarKind::Factor),
                col("born", VarKind::Date),
                col("visits", VarKind::Number),
                col("years", VarKind::Number),
            ],
        }
    }

    fn make(
        spec: &Spec,
        set: &[(&str, &str)],
        lists: &[(&str, &[&str])],
    ) -> Result<String, Vec<Problem>> {
        let c = ctx();
        let mut v: Values = spec.defaults(&c);
        for (id, text) in set {
            v.set_text(id, *text);
        }
        for (id, names) in lists {
            v.set_list(id, names.iter().map(|n| n.to_string()).collect());
        }
        spec.generate(&c, &v, Style::Base)
    }

    fn code(spec: &Spec, set: &[(&str, &str)], lists: &[(&str, &[&str])]) -> String {
        match make(spec, set, lists) {
            Ok(code) => code,
            Err(p) => format!("problems: {p:?}"),
        }
    }

    fn message(spec: &Spec, set: &[(&str, &str)], lists: &[(&str, &[&str])]) -> String {
        match make(spec, set, lists) {
            Ok(code) => format!("no problem: {code}"),
            Err(p) => p
                .iter()
                .map(|p| p.message.clone())
                .collect::<Vec<_>>()
                .join(" "),
        }
    }

    #[test]
    fn logistic_checks_the_outcome_then_fits_and_shows_odds_ratios() {
        let out = code(&LOGISTIC, &[("y", "won game")], &[("x", &["age", "arm"])]);
        assert!(
            out.starts_with(
                "stopifnot(\"the outcome must have exactly two different values\" = \
                 length(unique(stats::na.omit(`my data`[[\"won game\"]]))) == 2)\n\
                 model <- stats::glm(`won game` ~ age + arm, \
                 family = stats::binomial(link = \"logit\"), data = `my data`)\n\
                 print(summary(model))"
            ),
            "{out}"
        );
        assert!(
            out.ends_with(
                "print(exp(cbind(odds_ratio = stats::coef(model), \
                 stats::confint(model, level = 0.95))))"
            ),
            "{out}"
        );
        // odds ratios belong to the logit link only
        let probit = code(
            &LOGISTIC,
            &[("y", "won game"), ("link", "probit")],
            &[("x", &["age"])],
        );
        assert!(probit.contains("binomial(link = \"probit\")"), "{probit}");
        assert!(!probit.contains("Odds ratios"), "{probit}");
    }

    #[test]
    fn logistic_refuses_a_confused_set_of_choices() {
        let x = |names: &'static [&'static str]| [("x", names)];
        assert!(message(&LOGISTIC, &[("y", "age")], &[]).contains("at least 1 variable"));
        assert!(message(&LOGISTIC, &[("y", "age")], &x(&["age"])).contains("cannot also"));
        assert!(message(&LOGISTIC, &[("y", "born")], &x(&["age"])).contains("date"));
        assert!(message(&LOGISTIC, &[("y", "arm")], &x(&["born"])).contains("date"));
        assert!(message(
            &LOGISTIC,
            &[("y", "age"), ("save", "my data")],
            &x(&["arm"])
        )
        .contains("is the data frame"));
        assert!(message(&LOGISTIC, &[("y", "age"), ("link", "x")], &x(&["arm"])).contains("Pick"));
    }

    #[test]
    fn poisson_writes_an_offset_for_an_exposure() {
        let plain = code(&POISSON, &[("y", "visits")], &[("x", &["age"])]);
        assert!(
            plain.starts_with(
                "model <- stats::glm(visits ~ age, family = stats::poisson(), data = `my data`)\n"
            ),
            "{plain}"
        );
        assert!(plain.contains("Rate ratios"), "{plain}");
        let rate = code(
            &POISSON,
            &[
                ("y", "visits"),
                ("exposure", "years"),
                ("family", "quasipoisson"),
                ("save", "visit rate"),
            ],
            &[("x", &["age", "arm"])],
        );
        assert!(
            rate.starts_with(
                "`visit rate` <- stats::glm(visits ~ age + arm, family = stats::quasipoisson(), \
                 data = `my data`, offset = log(years))"
            ),
            "{rate}"
        );
        assert!(message(
            &POISSON,
            &[("y", "visits"), ("exposure", "visits")],
            &[("x", &["age"])]
        )
        .contains("choose another"));
        assert!(message(&POISSON, &[("y", "arm")], &[("x", &["age"])]).contains("needs number"));
    }

    #[test]
    fn negative_binomial_uses_mass_and_an_offset_for_an_exposure() {
        assert_eq!(NEGATIVE_BINOMIAL.packages(Style::Base), vec!["MASS"]);
        let plain = code(&NEGATIVE_BINOMIAL, &[("y", "visits")], &[("x", &["age"])]);
        assert!(
            plain.starts_with("model <- MASS::glm.nb(visits ~ age, data = `my data`)\n"),
            "{plain}"
        );
        assert!(plain.contains("Rate ratios"), "{plain}");
        let rate = code(
            &NEGATIVE_BINOMIAL,
            &[
                ("y", "visits"),
                ("exposure", "years"),
                ("save", "visit nb"),
                ("conf", "0.9"),
            ],
            &[("x", &["age", "arm"])],
        );
        assert!(
            rate.starts_with(
                "`visit nb` <- MASS::glm.nb(visits ~ age + arm + offset(log(years)), \
                 data = `my data`)\n"
            ),
            "{rate}"
        );
        assert!(rate.ends_with("level = 0.9))))"), "{rate}");
        assert!(message(
            &NEGATIVE_BINOMIAL,
            &[("y", "visits"), ("exposure", "visits")],
            &[("x", &["age"])]
        )
        .contains("choose another"));
        assert!(
            message(&NEGATIVE_BINOMIAL, &[("y", "arm")], &[("x", &["age"])])
                .contains("needs number")
        );
        assert!(message(
            &NEGATIVE_BINOMIAL,
            &[("y", "visits"), ("save", "my data")],
            &[("x", &["age"])]
        )
        .contains("is the data frame"));
    }

    #[test]
    fn ordinal_orders_the_outcome_explicitly_and_shows_ratios_for_the_logit_link() {
        assert_eq!(ORDINAL.packages(Style::Base), vec!["MASS"]);
        let out = code(&ORDINAL, &[("y", "visits")], &[("x", &["age", "arm"])]);
        assert!(
            out.starts_with(
                "model <- MASS::polr(ordered(visits) ~ age + arm, data = `my data`, \
                 method = \"logistic\", Hess = TRUE)\n"
            ),
            "{out}"
        );
        assert!(
            out.contains("paste(levels(ordered(`my data`[[\"visits\"]])), collapse = \" < \")"),
            "{out}"
        );
        assert!(out.contains("Coefficients with p-values"), "{out}");
        assert!(out.ends_with("level = 0.95))))"), "{out}");
        let probit = code(
            &ORDINAL,
            &[("y", "visits"), ("link", "probit")],
            &[("x", &["age"])],
        );
        assert!(probit.contains("method = \"probit\""), "{probit}");
        assert!(!probit.contains("Odds ratios"), "{probit}");
        assert!(
            message(&ORDINAL, &[("y", "visits")], &[("x", &["visits"])]).contains("cannot also")
        );
        assert!(message(&ORDINAL, &[("y", "born")], &[("x", &["age"])]).contains("date"));
    }

    #[test]
    fn multinomial_needs_nnet_three_categories_and_a_reference() {
        assert_eq!(MULTINOMIAL.packages(Style::Base), vec!["nnet"]);
        let out = code(&MULTINOMIAL, &[("y", "arm")], &[("x", &["age"])]);
        assert!(
            out.contains("length(unique(stats::na.omit(`my data`[[\"arm\"]]))) >= 3)"),
            "{out}"
        );
        assert!(
            out.contains(
                "model <- nnet::multinom(factor(arm) ~ age, data = `my data`, trace = FALSE)"
            ),
            "{out}"
        );
        assert!(out.contains("Relative risk ratios"), "{out}");
        let with_reference = code(
            &MULTINOMIAL,
            &[("y", "arm"), ("reference", "control")],
            &[("x", &["age"])],
        );
        assert!(
            with_reference.contains("stats::relevel(factor(arm), ref = \"control\") ~ age"),
            "{with_reference}"
        );
        assert!(message(&MULTINOMIAL, &[("y", "visits")], &[("x", &["age"])]).contains("needs"));
        assert!(message(
            &MULTINOMIAL,
            &[("y", "arm"), ("save", "my data")],
            &[("x", &["age"])]
        )
        .contains("is the data frame"));
    }

    #[test]
    fn every_dialog_is_a_catalog_entry_and_in_the_statistics_menu() {
        for spec in SPECS {
            assert!(
                crate::ENTRIES
                    .iter()
                    .any(|e| e.name == spec.entry && e.area == crate::Area::Statistics),
                "{} has no Statistics catalog entry",
                spec.entry
            );
            assert!(spec.menu.starts_with("Statistics > Generalised models > "));
            assert!(
                crate::stats::SPECS.iter().any(|s| s.menu == spec.menu),
                "{} is not in the menu",
                spec.entry
            );
        }
    }
}
