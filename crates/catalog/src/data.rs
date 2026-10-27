//! Data menu dialogs (P3.5): each is one [`Spec`], giving the dialog and the
//! R in base and tidyverse style. Results go into a data frame named in the
//! dialog; by default the one being worked on, which is then replaced.

use rcraft_rcodegen::dialog::{
    no_check, Args, Opt, Problem, Spec, Style, TextDefault, TextRule, Widget,
};

const BOTH: &[Style] = &[Style::Base, Style::Tidyverse];

const RESULT: Widget = Widget::Text {
    id: "result",
    label: "Put the result in",
    default: TextDefault::DatasetName,
    rule: TextRule::Name,
    hint: "The data frame's own name replaces it; a new name keeps the original.",
};

/// `result <- data` when the result is a new data frame, so the lines after it
/// can change `result` in place.
fn copy_line(a: &Args) -> String {
    if a.replaces_data("result") {
        String::new()
    } else {
        format!("{} <- {}\n", a.name("result"), a.data())
    }
}

// ---- Sort rows ----

const ORDER: &[Opt] = &[
    Opt {
        value: "up",
        label: "Smallest first (A to Z)",
    },
    Opt {
        value: "down",
        label: "Largest first (Z to A)",
    },
];

const fn key(id: &'static str, label: &'static str, required: bool) -> Widget {
    Widget::Variable {
        id,
        label,
        kinds: &[],
        required,
        hint: "",
    }
}

const fn order(id: &'static str) -> Widget {
    Widget::Choice {
        id,
        label: "Order",
        options: ORDER,
        default: "up",
        hint: "",
    }
}

/// The chosen sort keys, in order, with whether each runs largest first.
fn sort_keys<'a>(a: &'a Args) -> Vec<(&'a str, bool)> {
    [("by1", "order1"), ("by2", "order2"), ("by3", "order3")]
        .into_iter()
        .filter(|(v, _)| !a.var_text(v).is_empty())
        .map(|(v, o)| (v, a.choice(o) == "down"))
        .collect()
}

fn sort_check(a: &Args) -> Vec<Problem> {
    let mut seen: Vec<&str> = Vec::new();
    let mut out = Vec::new();
    for id in ["by1", "by2", "by3"] {
        let name = a.var_text(id);
        if name.is_empty() {
            continue;
        }
        if seen.contains(&name) {
            out.push(Problem::new(id, format!("{name} is already a sort key.")));
        }
        seen.push(name);
    }
    out
}

fn sort_code(a: &Args, style: Style) -> String {
    let keys = sort_keys(a);
    match style {
        Style::Base => {
            let terms: Vec<String> = keys
                .iter()
                .filter_map(|(id, down)| {
                    a.var_string(id).map(|s| {
                        let col = format!("{}[[{s}]]", a.data());
                        if *down {
                            format!("-xtfrm({col})")
                        } else {
                            col
                        }
                    })
                })
                .collect();
            format!(
                "{} <- {}[order({}), , drop = FALSE]",
                a.name("result"),
                a.data(),
                terms.join(", ")
            )
        }
        Style::Tidyverse => {
            let terms: Vec<String> = keys
                .iter()
                .filter_map(|(id, down)| {
                    a.var(id).map(|v| {
                        if *down {
                            format!("dplyr::desc({v})")
                        } else {
                            v
                        }
                    })
                })
                .collect();
            format!(
                "{} <- dplyr::arrange({}, {})",
                a.name("result"),
                a.data(),
                terms.join(", ")
            )
        }
    }
}

pub const SORT: Spec = Spec {
    entry: "Sort rows",
    menu: "Data > Sort rows",
    about: "Put the rows in order by up to three variables. Missing values go last.",
    widgets: &[
        key("by1", "Sort by", true),
        order("order1"),
        key("by2", "Then by", false),
        order("order2"),
        key("by3", "Then by", false),
        order("order3"),
        RESULT,
    ],
    styles: BOTH,
    check: sort_check,
    code: sort_code,
};

// ---- Filter rows ----

fn filter_code(a: &Args, style: Style) -> String {
    let cond = if a.choice("keep") == "drop" {
        format!("!({})", a.expr("condition"))
    } else {
        a.expr("condition").to_string()
    };
    let call = match style {
        Style::Base => "subset",
        Style::Tidyverse => "dplyr::filter",
    };
    format!("{} <- {call}({}, {cond})", a.name("result"), a.data())
}

pub const FILTER: Spec = Spec {
    entry: "Filter rows",
    menu: "Data > Filter rows",
    about: "Keep or drop the rows where a condition holds.",
    widgets: &[
        Widget::Choice {
            id: "keep",
            label: "Rows to",
            options: &[
                Opt {
                    value: "keep",
                    label: "Keep",
                },
                Opt {
                    value: "drop",
                    label: "Drop",
                },
            ],
            default: "keep",
            hint: "",
        },
        Widget::Text {
            id: "condition",
            label: "Where",
            default: TextDefault::Empty,
            rule: TextRule::Expression,
            hint: "An R condition, for example: age >= 18 & sex == \"female\". \
                   Rows where it is NA are left out either way, as in R.",
        },
        RESULT,
    ],
    styles: BOTH,
    check: no_check,
    code: filter_code,
};

// ---- New variable ----

fn compute_code(a: &Args, style: Style) -> String {
    match style {
        Style::Base => format!(
            "{}{res}[[{}]] <- with({res}, {})",
            copy_line(a),
            a.name_string("name"),
            a.expr("value"),
            res = a.name("result"),
        ),
        Style::Tidyverse => format!(
            "{} <- dplyr::mutate({}, {} = {})",
            a.name("result"),
            a.data(),
            a.name("name"),
            a.expr("value")
        ),
    }
}

pub const COMPUTE: Spec = Spec {
    entry: "New variable",
    menu: "Data > New variable",
    about: "Add a variable worked out from the others. A name that is taken is replaced.",
    widgets: &[
        Widget::Text {
            id: "name",
            label: "New variable",
            default: TextDefault::Empty,
            rule: TextRule::Name,
            hint: "",
        },
        Widget::Text {
            id: "value",
            label: "Value",
            default: TextDefault::Empty,
            rule: TextRule::Expression,
            hint: "An R expression using the other variables, for example: weight / height^2",
        },
        RESULT,
    ],
    styles: BOTH,
    check: no_check,
    code: compute_code,
};

// ---- Rename variable ----

fn rename_check(a: &Args) -> Vec<Problem> {
    let old = a.var_text("old");
    let new = a.name_text("new");
    if new == old {
        vec![Problem::new(
            "new",
            "The new name is the same as the old one.",
        )]
    } else if a.ctx.column(new).is_some() {
        vec![Problem::new(
            "new",
            format!("{new} is already a variable in {}.", a.ctx.dataset),
        )]
    } else {
        Vec::new()
    }
}

fn rename_code(a: &Args, style: Style) -> String {
    match style {
        Style::Base => {
            let res = a.name("result");
            format!(
                "{}names({res})[names({res}) == {}] <- {}",
                copy_line(a),
                a.var_string("old").unwrap_or_default(),
                a.name_string("new"),
            )
        }
        Style::Tidyverse => format!(
            "{} <- dplyr::rename({}, {} = {})",
            a.name("result"),
            a.data(),
            a.name("new"),
            a.var("old").unwrap_or_default()
        ),
    }
}

pub const RENAME: Spec = Spec {
    entry: "Rename variable",
    menu: "Data > Rename variable",
    about: "Give a variable a new name.",
    widgets: &[
        Widget::Variable {
            id: "old",
            label: "Variable",
            kinds: &[],
            required: true,
            hint: "",
        },
        Widget::Text {
            id: "new",
            label: "New name",
            default: TextDefault::Empty,
            rule: TextRule::Name,
            hint: "",
        },
        RESULT,
    ],
    styles: BOTH,
    check: rename_check,
    code: rename_code,
};

/// The Data menu's dialogs, in menu order.
pub const SPECS: &[&Spec] = &[&COMPUTE, &FILTER, &SORT, &RENAME];

#[cfg(test)]
mod tests {
    use super::*;
    use rcraft_rcodegen::dialog::{ColumnInfo, Context, Values, VarKind};

    fn ctx() -> Context {
        let col = |name: &str, kind| ColumnInfo {
            name: name.into(),
            kind,
        };
        Context {
            dataset: "people".into(),
            columns: vec![
                col("age", VarKind::Number),
                col("sex", VarKind::Factor),
                col("home town", VarKind::Text),
            ],
        }
    }

    fn run(spec: &Spec, set: &[(&str, &str)], style: Style) -> Result<String, Vec<Problem>> {
        let c = ctx();
        let mut v: Values = spec.defaults(&c);
        for (id, text) in set {
            v.set_text(id, *text);
        }
        spec.generate(&c, &v, style)
    }

    fn code(spec: &Spec, set: &[(&str, &str)], style: Style) -> String {
        match run(spec, set, style) {
            Ok(code) => code,
            Err(p) => format!("problems: {p:?}"),
        }
    }

    #[test]
    fn sort_in_both_styles() {
        let set = [("by1", "sex"), ("by2", "age"), ("order2", "down")];
        assert_eq!(
            code(&SORT, &set, Style::Base),
            "people <- people[order(people[[\"sex\"]], -xtfrm(people[[\"age\"]])), , drop = FALSE]"
        );
        assert_eq!(
            code(&SORT, &set, Style::Tidyverse),
            "people <- dplyr::arrange(people, sex, dplyr::desc(age))"
        );
        assert_eq!(
            code(
                &SORT,
                &[("by1", "home town"), ("result", "s")],
                Style::Tidyverse
            ),
            "s <- dplyr::arrange(people, `home town`)"
        );
    }

    #[test]
    fn sort_needs_a_key_and_no_repeats() {
        assert!(run(&SORT, &[], Style::Base).is_err());
        let twice = run(&SORT, &[("by1", "age"), ("by3", "age")], Style::Base);
        assert_eq!(
            twice.err().and_then(|p| p.first().map(|p| p.widget)),
            Some("by3")
        );
    }

    #[test]
    fn filter_keeps_or_drops() {
        let set = [("condition", " age >= 18 & sex == \"f\" ")];
        assert_eq!(
            code(&FILTER, &set, Style::Base),
            "people <- subset(people, age >= 18 & sex == \"f\")"
        );
        let drop = [
            ("condition", "age < 18"),
            ("keep", "drop"),
            ("result", "adults"),
        ];
        assert_eq!(
            code(&FILTER, &drop, Style::Tidyverse),
            "adults <- dplyr::filter(people, !(age < 18))"
        );
        assert!(run(&FILTER, &[("condition", "age), q(")], Style::Base).is_err());
        assert!(run(&FILTER, &[], Style::Base).is_err());
    }

    #[test]
    fn new_variable_in_place_and_as_a_copy() {
        let set = [("name", "age2"), ("value", "age^2")];
        assert_eq!(
            code(&COMPUTE, &set, Style::Base),
            "people[[\"age2\"]] <- with(people, age^2)"
        );
        assert_eq!(
            code(
                &COMPUTE,
                &[("name", "age 2"), ("value", "age * 2"), ("result", "p2")],
                Style::Base
            ),
            "p2 <- people\np2[[\"age 2\"]] <- with(p2, age * 2)"
        );
        assert_eq!(
            code(
                &COMPUTE,
                &[("name", "age 2"), ("value", "age * 2")],
                Style::Tidyverse
            ),
            "people <- dplyr::mutate(people, `age 2` = age * 2)"
        );
    }

    #[test]
    fn rename_in_both_styles_and_refuses_a_taken_name() {
        let set = [("old", "home town"), ("new", "town")];
        assert_eq!(
            code(&RENAME, &set, Style::Base),
            "names(people)[names(people) == \"home town\"] <- \"town\""
        );
        assert_eq!(
            code(&RENAME, &set, Style::Tidyverse),
            "people <- dplyr::rename(people, town = `home town`)"
        );
        let copy = [("old", "age"), ("new", "years"), ("result", "p")];
        assert_eq!(
            code(&RENAME, &copy, Style::Base),
            "p <- people\nnames(p)[names(p) == \"age\"] <- \"years\""
        );
        let taken = run(&RENAME, &[("old", "age"), ("new", "sex")], Style::Base);
        assert_eq!(
            taken
                .err()
                .and_then(|p| p.first().map(|p| p.message.clone())),
            Some("sex is already a variable in people.".to_string())
        );
    }

    #[test]
    fn every_spec_is_a_catalog_entry_and_widget_ids_are_unique() {
        for spec in SPECS {
            assert!(
                crate::ENTRIES.iter().any(|e| e.name == spec.entry),
                "{} has no catalog entry",
                spec.entry
            );
            let mut ids: Vec<&str> = spec.widgets.iter().map(|w| w.id()).collect();
            ids.sort_unstable();
            let n = ids.len();
            ids.dedup();
            assert_eq!(n, ids.len(), "{} repeats a widget id", spec.entry);
            assert!(!spec.styles.is_empty());
        }
    }
}
