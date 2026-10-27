//! Statistics menu, linear models and ANOVA (P4.3): linear regression, one-way
//! and factorial analysis of variance. Base R only (the stats package ships with
//! R). Each fits a model, keeps it in the workspace under a name the dialog
//! asks for (so later commands, and the post-estimation menu of P4.5, can use
//! it), and prints R's own summary in the Results window. None changes the data.
//!
//! Categorical predictors use R's default treatment contrasts: the first level
//! is the reference and each coefficient is a difference from it.

use rcraft_rcodegen::dialog::{Args, Opt, Problem, Spec, TextDefault, TextRule};
use rcraft_rcodegen::dialog::{VarKind, Widget};
use rcraft_rcodegen::r_name;

use crate::data::BASE;

const NUMBERS: &[VarKind] = &[VarKind::Number];
const GROUPS: &[VarKind] = &[VarKind::Factor, VarKind::Text, VarKind::Logical];
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

const fn save(default: &'static str) -> Widget {
    Widget::Text {
        id: "save",
        label: "Save the model as",
        default: TextDefault::Fixed(default),
        rule: TextRule::Name,
        hint: "The fitted model stays in R's workspace under this name; a name already in \
               use is replaced.",
    }
}

const OUTCOME: Widget = Widget::Variable {
    id: "y",
    label: "Outcome",
    kinds: NUMBERS,
    required: true,
    hint: "",
};

/// A model must not be saved over the data frame it is fitted to.
fn not_the_data(a: &Args) -> Option<Problem> {
    (a.name_text("save") == a.ctx.dataset).then(|| {
        Problem::new(
            "save",
            format!("{} is the data frame; choose another name.", a.ctx.dataset),
        )
    })
}

/// `a + b + c`, or `(a + b + c)^2` for every two-way interaction.
fn terms(names: &[&str], interactions: bool) -> String {
    let joined = names
        .iter()
        .map(|n| r_name(n))
        .collect::<Vec<_>>()
        .join(" + ");
    if interactions && names.len() > 1 {
        format!("({joined})^2")
    } else {
        joined
    }
}

// ---- Linear model ----

fn linear_check(a: &Args) -> Vec<Problem> {
    let y = a.var_text("y");
    let xs = a.vars("x");
    let mut out = Vec::new();
    if xs.contains(&y) {
        out.push(Problem::new(
            "x",
            format!("{y} is the outcome; it cannot also be a predictor."),
        ));
    }
    if a.var_text("weights") == y {
        out.push(Problem::new(
            "weights",
            format!("{y} is the outcome; choose another variable for the weights."),
        ));
    }
    out.extend(not_the_data(a));
    out
}

pub const LINEAR: Spec = Spec {
    entry: "Linear model",
    menu: "Statistics > Linear models > Linear model",
    about: "Fit a straight-line (least squares) regression of a number variable on one or more \
            predictors, number or categorical. Rows missing any variable used are left out.",
    widgets: &[
        OUTCOME,
        Widget::Variables {
            id: "x",
            label: "Predictors",
            kinds: PREDICTORS,
            min: 1,
            hint: "A categorical predictor gets one coefficient per level after the first.",
        },
        Widget::Choice {
            id: "interactions",
            label: "Interactions",
            options: &[
                Opt {
                    value: "none",
                    label: "None (each predictor on its own)",
                },
                Opt {
                    value: "two",
                    label: "All two-way interactions",
                },
            ],
            default: "none",
            hint: "",
        },
        Widget::Variable {
            id: "weights",
            label: "Weights",
            kinds: NUMBERS,
            required: false,
            hint: "Optional: weighted least squares, each row counting this much.",
        },
        Widget::Choice {
            id: "conf",
            label: "Confidence level",
            options: LEVELS,
            default: "0.95",
            hint: "For the coefficient intervals.",
        },
        save("model"),
    ],
    styles: BASE,
    check: linear_check,
    code: |a, _| {
        let xs = a.vars("x");
        let weights = a
            .var("weights")
            .map_or_else(String::new, |w| format!(", weights = {w}"));
        let model = a.name("save");
        format!(
            "{model} <- stats::lm({} ~ {}, data = {}{weights})\nprint(summary({model}))\n\
             cat(\"\\nConfidence intervals\\n\")\nprint(stats::confint({model}, level = {}))",
            a.var("y").unwrap_or_default(),
            terms(&xs, a.choice("interactions") == "two"),
            a.data(),
            a.choice("conf"),
        )
    },
};

// ---- ANOVA ----

fn anova_check(a: &Args) -> Vec<Problem> {
    let mut out = Vec::new();
    out.extend(not_the_data(a));
    let factors = a.vars("g");
    if factors.len() > 1 {
        let mut seen: Vec<&str> = Vec::new();
        for f in &factors {
            if seen.contains(f) {
                out.push(Problem::new("g", format!("{f} is chosen twice.")));
            }
            seen.push(f);
        }
    }
    out
}

pub const ONE_WAY: Spec = Spec {
    entry: "One-way ANOVA",
    menu: "Statistics > Linear models > One-way ANOVA",
    about: "Compare the mean of a number variable across the groups of one variable, with the \
            group means and, if you want them, Tukey comparisons of every pair of groups.",
    widgets: &[
        OUTCOME,
        Widget::Variable {
            id: "g",
            label: "Groups",
            kinds: GROUPS,
            required: true,
            hint: "",
        },
        Widget::Check {
            id: "tukey",
            label: "Tukey comparisons of every pair of groups",
            default: true,
            hint: "",
        },
        save("anova"),
    ],
    styles: BASE,
    check: |a| {
        let mut out = Vec::new();
        out.extend(not_the_data(a));
        out
    },
    code: |a, _| {
        let (y, g, model) = (
            a.var("y").unwrap_or_default(),
            a.var("g").unwrap_or_default(),
            a.name("save"),
        );
        let mut code = format!(
            "{model} <- stats::aov({y} ~ {g}, data = {d})\nprint(summary({model}))\n\
             cat(\"\\nGroup means\\n\")\nprint(tapply({d}[[{ys}]], {d}[[{gs}]], mean, na.rm = TRUE))",
            d = a.data(),
            ys = a.var_string("y").unwrap_or_default(),
            gs = a.var_string("g").unwrap_or_default(),
        );
        if a.flag("tukey") {
            code.push_str(&format!("\nprint(stats::TukeyHSD({model}))"));
        }
        code
    },
};

pub const FACTORIAL: Spec = Spec {
    entry: "Factorial ANOVA",
    menu: "Statistics > Linear models > Factorial ANOVA",
    about: "Compare the mean of a number variable across the groups of two or more variables, \
            with their interactions if you want them. R's sequential (type I) sums of squares: \
            with unequal group sizes the order of the variables matters.",
    widgets: &[
        OUTCOME,
        Widget::Variables {
            id: "g",
            label: "Grouping variables",
            kinds: GROUPS,
            min: 1,
            hint: "One or more; each is a factor of the design.",
        },
        Widget::Choice {
            id: "interactions",
            label: "Interactions",
            options: &[
                Opt {
                    value: "all",
                    label: "All (full factorial)",
                },
                Opt {
                    value: "none",
                    label: "None (main effects only)",
                },
            ],
            default: "all",
            hint: "",
        },
        Widget::Check {
            id: "tukey",
            label: "Tukey comparisons within each variable",
            default: false,
            hint: "",
        },
        save("anova"),
    ],
    styles: BASE,
    check: anova_check,
    code: |a, _| {
        let factors = a.vars("g");
        let rhs = if a.choice("interactions") == "all" {
            factors
                .iter()
                .map(|n| r_name(n))
                .collect::<Vec<_>>()
                .join(" * ")
        } else {
            terms(&factors, false)
        };
        let model = a.name("save");
        let mut code = format!(
            "{model} <- stats::aov({} ~ {rhs}, data = {})\nprint(summary({model}))",
            a.var("y").unwrap_or_default(),
            a.data(),
        );
        if a.flag("tukey") {
            let which: Vec<String> = factors
                .iter()
                .map(|f| rcraft_rcodegen::r_string(f))
                .collect();
            code.push_str(&format!(
                "\nprint(stats::TukeyHSD({model}, which = c({})))",
                which.join(", ")
            ));
        }
        code
    },
};

/// The dialogs of this module, in menu order.
pub const SPECS: &[&Spec] = &[&LINEAR, &ONE_WAY, &FACTORIAL];

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
                col("blood pressure", VarKind::Number),
                col("age", VarKind::Number),
                col("arm", VarKind::Factor),
                col("smoker", VarKind::Text),
                col("born", VarKind::Date),
                col("w", VarKind::Number),
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
    fn linear_model_writes_the_fit_the_summary_and_the_intervals() {
        assert_eq!(
            code(
                &LINEAR,
                &[("y", "blood pressure")],
                &[("x", &["age", "arm"])]
            ),
            "model <- stats::lm(`blood pressure` ~ age + arm, data = `my data`)\n\
             print(summary(model))\ncat(\"\\nConfidence intervals\\n\")\n\
             print(stats::confint(model, level = 0.95))"
        );
        let full = code(
            &LINEAR,
            &[
                ("y", "blood pressure"),
                ("interactions", "two"),
                ("weights", "w"),
                ("conf", "0.99"),
                ("save", "fit 2"),
            ],
            &[("x", &["age", "smoker"])],
        );
        assert!(
            full.starts_with(
                "`fit 2` <- stats::lm(`blood pressure` ~ (age + smoker)^2, data = `my data`, \
                 weights = w)\n"
            ),
            "{full}"
        );
        assert!(full.ends_with("level = 0.99))"), "{full}");
        // a single predictor has no interaction to add
        assert!(code(
            &LINEAR,
            &[("y", "blood pressure"), ("interactions", "two")],
            &[("x", &["age"])]
        )
        .contains("~ age, data"));
    }

    #[test]
    fn linear_model_refuses_a_confused_set_of_choices() {
        let x = |names: &'static [&'static str]| [("x", names)];
        assert!(message(&LINEAR, &[("y", "age")], &[]).contains("at least 1 variable"));
        assert!(message(&LINEAR, &[("y", "age")], &x(&["age", "arm"])).contains("cannot also"));
        assert!(message(&LINEAR, &[("y", "age")], &x(&["born"])).contains("date"));
        assert!(message(&LINEAR, &[("y", "arm")], &x(&["age"])).contains("needs number"));
        assert!(
            message(&LINEAR, &[("y", "age"), ("weights", "age")], &x(&["arm"]))
                .contains("choose another")
        );
        assert!(
            message(&LINEAR, &[("y", "age"), ("save", "my data")], &x(&["arm"]))
                .contains("is the data frame")
        );
        assert!(message(&LINEAR, &[("y", "age"), ("save", " ")], &x(&["arm"])).contains("Type"));
    }

    #[test]
    fn anova_dialogs_write_their_models_and_comparisons() {
        let one = code(&ONE_WAY, &[("y", "blood pressure"), ("g", "arm")], &[]);
        assert!(
            one.starts_with("anova <- stats::aov(`blood pressure` ~ arm, data = `my data`)\n"),
            "{one}"
        );
        assert!(one.contains("tapply(`my data`[[\"blood pressure\"]], `my data`[[\"arm\"]], mean"));
        assert!(one.ends_with("print(stats::TukeyHSD(anova))"), "{one}");
        assert!(message(&ONE_WAY, &[("y", "age"), ("g", "age")], &[]).contains("needs"));
        assert!(message(
            &ONE_WAY,
            &[("y", "age"), ("g", "arm"), ("save", "my data")],
            &[]
        )
        .contains("is the data frame"));

        let factorial = code(&FACTORIAL, &[("y", "age")], &[("g", &["arm", "smoker"])]);
        assert!(
            factorial.contains("~ arm * smoker, data = `my data`)"),
            "{factorial}"
        );
        assert!(!factorial.contains("TukeyHSD"), "{factorial}");
        let main = code(
            &FACTORIAL,
            &[("y", "age"), ("interactions", "none")],
            &[("g", &["arm", "smoker"])],
        );
        assert!(main.contains("~ arm + smoker, data"), "{main}");
        assert!(
            message(&FACTORIAL, &[("y", "age")], &[("g", &["arm", "arm"])])
                .contains("chosen twice")
        );
        assert!(message(&FACTORIAL, &[("y", "age")], &[("g", &["age"])]).contains("needs"));
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
            assert!(spec.menu.starts_with("Statistics > Linear models > "));
            assert!(
                crate::stats::SPECS.iter().any(|s| s.menu == spec.menu),
                "{} is not in the menu",
                spec.entry
            );
        }
    }
}
