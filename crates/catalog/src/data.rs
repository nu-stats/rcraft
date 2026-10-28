//! Data menu dialogs (P3.5): each is one [`Spec`], giving the dialog and the
//! R in base and tidyverse style. Results go into a data frame named in the
//! dialog; by default the one being worked on, which is then replaced.

use rcraft_rcodegen::dialog::{
    no_check, Args, Opt, Problem, Spec, Style, TextDefault, TextRule, VarKind, Widget,
};
use rcraft_rcodegen::r_string;

use crate::data_tools;

const BOTH: &[Style] = &[Style::Base, Style::Tidyverse];

pub(crate) const RESULT: Widget = Widget::Text {
    id: "result",
    label: "Put the result in",
    default: TextDefault::DatasetName,
    rule: TextRule::Name,
    hint: "The data frame's own name replaces it; a new name keeps the original.",
};

/// `result <- data` when the result is a new data frame, so the lines after it
/// can change `result` in place.
pub(crate) fn copy_line(a: &Args) -> String {
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
    needs: &[],
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
    needs: &[],
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
    needs: &[],
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
    needs: &[],
    styles: BOTH,
    check: rename_check,
    code: rename_code,
};

// ---- Labels ----
//
// RCraft keeps labels as the plain attributes `label` (the variable's) and
// `labels` (a named vector: names are the labels, values the codes), which is
// what haven and the Variables pane use. Base R only: no package is needed.

pub(crate) const BASE: &[Style] = &[Style::Base];

const fn labelled_var(hint: &'static str) -> Widget {
    Widget::Variable {
        id: "var",
        label: "Variable",
        kinds: &[VarKind::Number, VarKind::Text],
        required: true,
        hint,
    }
}

/// `attr(res[["x"]], "key") <- value`, after the copy line.
fn set_attr(a: &Args, key: &str, value: &str) -> String {
    format!(
        "{}attr({}[[{}]], {}) <- {value}",
        copy_line(a),
        a.name("result"),
        a.var_string("var").unwrap_or_default(),
        r_string(key),
    )
}

fn variable_label_code(a: &Args, _: Style) -> String {
    let text = a.name_text("label");
    let value = if text.is_empty() {
        "NULL".to_string()
    } else {
        a.name_string("label")
    };
    set_attr(a, "label", &value)
}

pub const VARIABLE_LABEL: Spec = Spec {
    entry: "Variable label",
    menu: "Data > Variable label",
    about:
        "Give a variable a label: a longer description shown in the Variables pane and in output.",
    widgets: &[
        Widget::Variable {
            id: "var",
            label: "Variable",
            kinds: &[],
            required: true,
            hint: "",
        },
        Widget::Text {
            id: "label",
            label: "Label",
            default: TextDefault::Empty,
            rule: TextRule::Any,
            hint: "Leave empty to remove the label.",
        },
        RESULT,
    ],
    needs: &[],
    styles: BASE,
    check: no_check,
    code: variable_label_code,
};

/// `code = label` pairs separated by `;`, as typed in the Value labels dialog.
pub fn parse_value_labels(text: &str) -> Result<Vec<(String, String)>, String> {
    let mut out = Vec::new();
    for part in text.split(';').map(str::trim).filter(|p| !p.is_empty()) {
        let Some((code, label)) = part.split_once('=') else {
            return Err(format!("\"{part}\" needs an = (for example 1 = Low)."));
        };
        let (code, label) = (code.trim(), label.trim());
        if code.is_empty() || label.is_empty() {
            return Err(format!(
                "\"{part}\" needs a value before and a label after the =."
            ));
        }
        out.push((code.to_string(), label.to_string()));
    }
    Ok(out)
}

fn value_labels_check(a: &Args) -> Vec<Problem> {
    let pairs = match parse_value_labels(a.expr("labels")) {
        Ok(p) => p,
        Err(why) => return vec![Problem::new("labels", why)],
    };
    let numeric = a
        .ctx
        .column(a.var_text("var"))
        .is_some_and(|c| c.kind == VarKind::Number);
    let mut seen: Vec<&str> = Vec::new();
    for (code, _) in &pairs {
        if numeric && !code.parse::<f64>().is_ok_and(f64::is_finite) {
            return vec![Problem::new(
                "labels",
                format!(
                    "{code} is not a number, and {} holds numbers.",
                    a.var_text("var")
                ),
            )];
        }
        if seen.contains(&code.as_str()) {
            return vec![Problem::new("labels", format!("{code} has two labels."))];
        }
        seen.push(code);
    }
    Vec::new()
}

fn value_labels_code(a: &Args, _: Style) -> String {
    let pairs = parse_value_labels(a.expr("labels")).unwrap_or_default();
    if pairs.is_empty() {
        return set_attr(a, "labels", "NULL");
    }
    let numeric = a
        .ctx
        .column(a.var_text("var"))
        .is_some_and(|c| c.kind == VarKind::Number);
    let codes: Vec<String> = pairs
        .iter()
        .map(|(c, _)| if numeric { c.clone() } else { r_string(c) })
        .collect();
    let names: Vec<String> = pairs.iter().map(|(_, l)| r_string(l)).collect();
    set_attr(
        a,
        "labels",
        &format!(
            "stats::setNames(c({}), c({}))",
            codes.join(", "),
            names.join(", ")
        ),
    )
}

pub const VALUE_LABELS: Spec = Spec {
    entry: "Value labels",
    menu: "Data > Value labels",
    about: "Say what the codes of a variable mean. The labels are kept with the variable; \
            the values themselves do not change.",
    widgets: &[
        labelled_var(""),
        Widget::Text {
            id: "labels",
            label: "Labels",
            default: TextDefault::Empty,
            rule: TextRule::Any,
            hint: "Value = label, separated by ; for example: 1 = Low; 2 = Medium; 3 = High. \
                   These replace the labels the variable has now; leave empty to remove them.",
        },
        RESULT,
    ],
    needs: &[],
    styles: BASE,
    check: value_labels_check,
    code: value_labels_code,
};

fn labelled_to_factor_code(a: &Args, _: Style) -> String {
    let (res, col) = (a.name("result"), a.var_string("var").unwrap_or_default());
    let none = r_string(&format!("{} has no value labels", a.var_text("var")));
    // a function, so its names (x, l, f) cannot clash with the data frame's
    format!(
        "{copy}{res}[[{col}]] <- (function(x) {{\n  l <- attr(x, \"labels\")\n  \
         if (is.null(l)) stop({none})\n  \
         f <- factor(x, levels = l, labels = names(l))\n  \
         attr(f, \"label\") <- attr(x, \"label\")\n  f\n}})({res}[[{col}]])",
        copy = copy_line(a),
    )
}

pub const LABELLED_TO_FACTOR: Spec = Spec {
    entry: "Labelled to factor",
    menu: "Data > Labelled to factor",
    about: "Turn a variable with value labels into a factor whose levels are the labels. \
            Values that have no label become missing.",
    widgets: &[
        labelled_var("The variable must have value labels (see Value labels)."),
        RESULT,
    ],
    needs: &[],
    styles: BASE,
    check: no_check,
    code: labelled_to_factor_code,
};

// ---- Objects pane: rename, remove, save ----
//
// These work on an object in R's workspace (not only a data frame), named in
// the dialog's context as its "data frame"; they have no columns to choose.

fn rename_object_check(a: &Args) -> Vec<Problem> {
    if a.name_text("result") == a.ctx.dataset {
        vec![Problem::new(
            "result",
            "The new name is the same as the old one.",
        )]
    } else {
        Vec::new()
    }
}

fn rename_object_code(a: &Args, _: Style) -> String {
    let new = a.name("result");
    let taken = r_string(&format!("{} already exists", a.name_text("result")));
    format!(
        "if (exists({}, envir = globalenv(), inherits = FALSE)) stop({taken})\n\
         {new} <- {old}\nrm({old})",
        a.name_string("result"),
        old = a.data(),
    )
}

pub const RENAME_OBJECT: Spec = Spec {
    entry: "Rename object",
    menu: "Objects > Rename",
    about: "Give an object in R's workspace a new name. A name that is taken is not replaced.",
    widgets: &[Widget::Text {
        id: "result",
        label: "New name",
        default: TextDefault::Empty,
        rule: TextRule::Name,
        hint: "",
    }],
    needs: &[],
    styles: BASE,
    check: rename_object_check,
    code: rename_object_code,
};

pub const REMOVE_OBJECT: Spec = Spec {
    entry: "Remove object",
    menu: "Objects > Remove",
    about: "Remove this object from R's workspace. This cannot be undone; files on disk \
            are not touched.",
    widgets: &[],
    needs: &[],
    styles: BASE,
    check: no_check,
    code: |a, _| format!("rm({})", a.data()),
};

fn save_object_check(a: &Args) -> Vec<Problem> {
    let file = a.name_text("file");
    if file.is_empty() {
        vec![Problem::new("file", "Type where to save it.")]
    } else if !file.to_ascii_lowercase().ends_with(".rds") {
        vec![Problem::new("file", "The file name should end in .rds.")]
    } else {
        Vec::new()
    }
}

fn save_object_code(a: &Args, _: Style) -> String {
    let path = r_string(&a.name_text("file").replace('\\', "/"));
    let guard = if a.flag("replace") {
        String::new()
    } else {
        format!(
            "if (file.exists({path})) stop({})\n",
            r_string("the file already exists")
        )
    };
    format!("{guard}saveRDS({}, file = {path})", a.data())
}

pub const SAVE_OBJECT: Spec = Spec {
    entry: "Save object",
    menu: "Objects > Save",
    about: "Save this object in an .rds file, which keeps it exactly as it is in R \
            (types, labels, everything). Open it again with Use my file.",
    widgets: &[
        Widget::Text {
            id: "file",
            label: "Save to",
            default: TextDefault::Empty,
            rule: TextRule::Any,
            hint: "The full path of the file, ending in .rds",
        },
        Widget::Check {
            id: "replace",
            label: "Replace the file if it is already there",
            default: false,
            hint: "",
        },
    ],
    needs: &[],
    styles: BASE,
    check: save_object_check,
    code: save_object_code,
};

/// The dialogs of the Objects pane's menu, in order. They are not in the Data
/// menu: each needs an object to work on.
pub const OBJECT_SPECS: &[&Spec] = &[&RENAME_OBJECT, &REMOVE_OBJECT, &SAVE_OBJECT];

/// Whether `menu` (as in [`Spec::menu`]) is one of the Objects pane's.
pub fn is_object_menu(menu: &str) -> bool {
    OBJECT_SPECS.iter().any(|s| s.menu == menu)
}

/// The Data menu's dialogs, in menu order.
pub const SPECS: &[&Spec] = &[
    &COMPUTE,
    &FILTER,
    &SORT,
    &RENAME,
    &VARIABLE_LABEL,
    &VALUE_LABELS,
    &LABELLED_TO_FACTOR,
    &data_tools::RECODE,
    &data_tools::CONVERT_TYPE,
    &data_tools::REFERENCE_LEVEL,
    &data_tools::COMBINE_LEVELS,
    &data_tools::REORDER_LEVELS,
    &data_tools::TEXT_CASE,
    &data_tools::FIND_REPLACE,
    &data_tools::EXTRACT_TEXT,
    &data_tools::TEXT_TO_DATE,
    &data_tools::DATE_PART,
];

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
    fn variable_label_sets_or_removes() {
        assert_eq!(
            code(
                &VARIABLE_LABEL,
                &[("var", "home town"), ("label", " Where \"home\" is ")],
                Style::Base
            ),
            "attr(people[[\"home town\"]], \"label\") <- \"Where \\\"home\\\" is\""
        );
        assert_eq!(
            code(
                &VARIABLE_LABEL,
                &[("var", "age"), ("result", "p")],
                Style::Base
            ),
            "p <- people\nattr(p[[\"age\"]], \"label\") <- NULL"
        );
    }

    #[test]
    fn value_labels_for_numbers_and_text() {
        let nums = [("var", "age"), ("labels", "1 = Low; 3 = High")];
        assert_eq!(
            code(&VALUE_LABELS, &nums, Style::Base),
            "attr(people[[\"age\"]], \"labels\") <- stats::setNames(c(1, 3), c(\"Low\", \"High\"))"
        );
        let text = [("var", "home town"), ("labels", "a = Alpha")];
        assert_eq!(
            code(&VALUE_LABELS, &text, Style::Base),
            "attr(people[[\"home town\"]], \"labels\") <- stats::setNames(c(\"a\"), c(\"Alpha\"))"
        );
        assert_eq!(
            code(&VALUE_LABELS, &[("var", "age")], Style::Base),
            "attr(people[[\"age\"]], \"labels\") <- NULL"
        );
    }

    #[test]
    fn value_labels_refuse_bad_input() {
        let why = |labels: &str, var: &str| {
            run(
                &VALUE_LABELS,
                &[("var", var), ("labels", labels)],
                Style::Base,
            )
            .err()
            .and_then(|p| p.first().map(|p| p.message.clone()))
        };
        assert!(why("1 Low", "age").is_some_and(|m| m.contains("needs an =")));
        assert!(why("= Low", "age").is_some());
        assert!(why("1 =", "age").is_some());
        assert!(why("x = Low", "age").is_some_and(|m| m.contains("not a number")));
        assert!(why("inf = Low", "age").is_some());
        assert!(why("1 = a; 1 = b", "age").is_some_and(|m| m.contains("two labels")));
        assert!(why("x = Low", "home town").is_none());
        // a factor is not labelled this way
        assert!(why("1 = a", "sex").is_some());
    }

    #[test]
    fn labelled_to_factor_checks_for_labels_when_it_runs() {
        let out = code(&LABELLED_TO_FACTOR, &[("var", "age")], Style::Base);
        assert!(
            out.starts_with("people[[\"age\"]] <- (function(x) {\n"),
            "{out}"
        );
        assert!(out.contains("stop(\"age has no value labels\")"), "{out}");
        assert!(out.contains("factor(x, levels = l, labels = names(l))"));
        assert!(out.ends_with("})(people[[\"age\"]])"), "{out}");
        let copy = code(
            &LABELLED_TO_FACTOR,
            &[("var", "age"), ("result", "p")],
            Style::Base,
        );
        assert!(
            copy.starts_with("p <- people\np[[\"age\"]] <- (function(x) {"),
            "{copy}"
        );
    }

    #[test]
    fn object_dialogs_rename_remove_and_save() {
        assert_eq!(
            code(&RENAME_OBJECT, &[("result", "my people")], Style::Base),
            "if (exists(\"my people\", envir = globalenv(), inherits = FALSE)) \
             stop(\"my people already exists\")\n`my people` <- people\nrm(people)"
        );
        assert!(run(&RENAME_OBJECT, &[("result", "people")], Style::Base).is_err());
        assert!(run(&RENAME_OBJECT, &[], Style::Base).is_err());
        assert_eq!(code(&REMOVE_OBJECT, &[], Style::Base), "rm(people)");
        assert_eq!(
            code(
                &SAVE_OBJECT,
                &[("file", "C:\\data\\people.RDS")],
                Style::Base
            ),
            "if (file.exists(\"C:/data/people.RDS\")) stop(\"the file already exists\")\n\
             saveRDS(people, file = \"C:/data/people.RDS\")"
        );
        let mut c = ctx();
        let mut v = SAVE_OBJECT.defaults(&c);
        v.set_text("file", "/tmp/p.rds");
        v.set_flag("replace", true);
        assert_eq!(
            SAVE_OBJECT.generate(&c, &v, Style::Base),
            Ok("saveRDS(people, file = \"/tmp/p.rds\")".to_string())
        );
        v.set_text("file", "/tmp/p.csv");
        assert!(SAVE_OBJECT.generate(&c, &v, Style::Base).is_err());
        v.set_text("file", " ");
        assert!(SAVE_OBJECT.generate(&c, &v, Style::Base).is_err());
        c.columns.clear();
        assert!(is_object_menu("Objects > Rename"));
        assert!(!is_object_menu("Data > Sort rows"));
    }

    #[test]
    fn every_spec_is_a_catalog_entry_and_widget_ids_are_unique() {
        for spec in SPECS.iter().chain(OBJECT_SPECS) {
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
