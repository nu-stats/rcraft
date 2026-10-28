//! Data menu dialogs for combining and reshaping data frames (P3.6): join,
//! append, aggregate, reshape wide and long, duplicates, random sample and
//! dropping missing values. Base R only (`merge`, `aggregate`, `reshape`), so
//! each runs against any R. A second data frame is named in a text box; its
//! variables are matched by name, as `merge` does.

use rcraft_rcodegen::dialog::{
    no_check, Args, Opt, Problem, Spec, Style, TextDefault, TextRule, VarKind, Widget,
};
use rcraft_rcodegen::r_string;

use crate::data::{copy_line, RESULT};

const BASE: &[Style] = &[Style::Base];

/// `c("a", "b")` for the chosen names.
fn cvec(names: &[&str]) -> String {
    let items: Vec<String> = names.iter().map(|n| r_string(n)).collect();
    format!("c({})", items.join(", "))
}

/// `data[c("a", "b")]`, or the whole data frame when no name is chosen.
fn columns(a: &Args, names: &[&str]) -> String {
    if names.is_empty() {
        a.data()
    } else {
        format!("{}[{}]", a.data(), cvec(names))
    }
}

const OTHER: Widget = Widget::Text {
    id: "other",
    label: "Other data frame",
    default: TextDefault::Empty,
    rule: TextRule::Name,
    hint: "The name of a data frame already in the workspace (see the Objects pane).",
};

fn other_check(a: &Args) -> Vec<Problem> {
    if a.name_text("other") == a.name_text("result") && !a.name_text("other").is_empty() {
        return vec![Problem::new(
            "result",
            "The result cannot replace the other data frame; choose another name.",
        )];
    }
    if a.name_text("other") == a.ctx.dataset {
        return vec![Problem::new(
            "other",
            "That is the data frame being worked on; name a different one.",
        )];
    }
    Vec::new()
}

// ---- Join ----

pub const JOIN: Spec = Spec {
    entry: "Join",
    menu: "Data > Combine > Join",
    about: "Add the columns of another data frame to this one, matching rows on variables \
            with the same names in both. The rows come out ordered by the match variables.",
    widgets: &[
        OTHER,
        Widget::Variables {
            id: "keys",
            label: "Match on",
            kinds: &[],
            min: 1,
            hint: "Each must also be a variable of the other data frame.",
        },
        Widget::Choice {
            id: "keep",
            label: "Keep",
            options: &[
                Opt {
                    value: "inner",
                    label: "Only rows found in both",
                },
                Opt {
                    value: "left",
                    label: "All rows of this data frame",
                },
                Opt {
                    value: "right",
                    label: "All rows of the other data frame",
                },
                Opt {
                    value: "full",
                    label: "All rows of both",
                },
            ],
            default: "inner",
            hint: "",
        },
        RESULT,
    ],
    needs: &[],
    styles: BASE,
    check: other_check,
    code: join_code,
};

fn join_code(a: &Args, _: Style) -> String {
    let keep = a.choice("keep");
    let x = matches!(keep, "left" | "full");
    let y = matches!(keep, "right" | "full");
    format!(
        "{} <- merge({}, {}, by = {}, all.x = {}, all.y = {})",
        a.name("result"),
        a.data(),
        a.name("other"),
        cvec(&a.vars("keys")),
        if x { "TRUE" } else { "FALSE" },
        if y { "TRUE" } else { "FALSE" },
    )
}

// ---- Append ----

pub const APPEND: Spec = Spec {
    entry: "Append",
    menu: "Data > Combine > Append",
    about: "Put the rows of another data frame under this one. A variable missing from \
            one of them is filled with missing values.",
    widgets: &[OTHER, RESULT],
    needs: &[],
    styles: BASE,
    check: other_check,
    code: append_code,
};

fn append_code(a: &Args, _: Style) -> String {
    format!(
        "{} <- (function(x, y) {{\n  for (n in setdiff(names(y), names(x))) x[[n]] <- NA\n  \
         for (n in setdiff(names(x), names(y))) y[[n]] <- NA\n  rbind(x, y[names(x)])\n}})({}, {})",
        a.name("result"),
        a.data(),
        a.name("other"),
    )
}

// ---- Aggregate ----

pub const AGGREGATE: Spec = Spec {
    entry: "Aggregate",
    menu: "Data > Combine > Aggregate",
    about: "One row for each group, with a summary of the chosen variables. Missing \
            values are left out of each summary.",
    widgets: &[
        Widget::Variables {
            id: "groups",
            label: "Group by",
            kinds: &[],
            min: 1,
            hint: "",
        },
        Widget::Variables {
            id: "values",
            label: "Summarise",
            kinds: &[VarKind::Number],
            min: 1,
            hint: "",
        },
        Widget::Choice {
            id: "how",
            label: "Summary",
            options: &[
                Opt {
                    value: "mean",
                    label: "Mean",
                },
                Opt {
                    value: "sum",
                    label: "Sum",
                },
                Opt {
                    value: "median",
                    label: "Median",
                },
                Opt {
                    value: "min",
                    label: "Smallest",
                },
                Opt {
                    value: "max",
                    label: "Largest",
                },
                Opt {
                    value: "sd",
                    label: "Standard deviation",
                },
                Opt {
                    value: "count",
                    label: "Count of values present",
                },
            ],
            default: "mean",
            hint: "",
        },
        RESULT,
    ],
    needs: &[],
    styles: BASE,
    check: aggregate_check,
    code: aggregate_code,
};

fn aggregate_check(a: &Args) -> Vec<Problem> {
    let groups = a.vars("groups");
    a.vars("values")
        .into_iter()
        .filter(|v| groups.contains(v))
        .map(|v| Problem::new("values", format!("{v} is also a grouping variable.")))
        .take(1)
        .collect()
}

fn aggregate_code(a: &Args, _: Style) -> String {
    let f = match a.choice("how") {
        "count" => "sum(!is.na(x))".to_string(),
        other => format!("{other}(x, na.rm = TRUE)"),
    };
    format!(
        "{} <- aggregate({}, by = {}, FUN = function(x) {f})",
        a.name("result"),
        columns(a, &a.vars("values")),
        columns(a, &a.vars("groups")),
    )
}

// ---- Wide to long ----

pub const WIDE_TO_LONG: Spec = Spec {
    entry: "Wide to long",
    menu: "Data > Reshape > Wide to long",
    about: "Stack several variables (income2024, income2025, ...) into one, with a new \
            variable saying which one each row came from.",
    widgets: &[
        Widget::Variables {
            id: "stack",
            label: "Variables to stack",
            kinds: &[],
            min: 2,
            hint: "They should hold the same kind of value.",
        },
        Widget::Variables {
            id: "ids",
            label: "Identify each case by",
            kinds: &[],
            min: 0,
            hint: "Leave empty to number the cases in a new variable called id.",
        },
        Widget::Text {
            id: "values",
            label: "New variable for the values",
            default: TextDefault::Fixed("value"),
            rule: TextRule::Name,
            hint: "",
        },
        Widget::Text {
            id: "which",
            label: "New variable for where each came from",
            default: TextDefault::Fixed("which"),
            rule: TextRule::Name,
            hint: "It holds the stacked variables' names.",
        },
        RESULT,
    ],
    needs: &[],
    styles: BASE,
    check: long_check,
    code: long_code,
};

fn long_check(a: &Args) -> Vec<Problem> {
    let stack = a.vars("stack");
    let ids = a.vars("ids");
    if let Some(v) = ids.iter().find(|v| stack.contains(v)) {
        return vec![Problem::new("ids", format!("{v} is also being stacked."))];
    }
    if ids.is_empty() && a.ctx.column("id").is_some() {
        return vec![Problem::new(
            "ids",
            "The data already has a variable called id; choose what identifies each case.",
        )];
    }
    if a.name_text("values") == a.name_text("which") {
        return vec![Problem::new("which", "The two new names must differ.")];
    }
    Vec::new()
}

fn long_code(a: &Args, _: Style) -> String {
    let stack = a.vars("stack");
    let ids = a.vars("ids");
    let idvar = if ids.is_empty() {
        String::new()
    } else {
        format!(", idvar = {}", cvec(&ids))
    };
    format!(
        "{res} <- (function(d) {{\n  long <- stats::reshape(d, direction = \"long\", \
         varying = list({stack}), v.names = {values}, timevar = {which}, times = {stack}{idvar})\n  \
         rownames(long) <- NULL\n  attr(long, \"reshapeLong\") <- NULL\n  long\n}})({data})",
        res = a.name("result"),
        stack = cvec(&stack),
        values = a.name_string("values"),
        which = a.name_string("which"),
        data = a.data(),
    )
}

// ---- Long to wide ----

pub const LONG_TO_WIDE: Spec = Spec {
    entry: "Long to wide",
    menu: "Data > Reshape > Long to wide",
    about: "Spread one variable over several, one for each value of another (income \
            becomes income_2024, income_2025, ...). Other variables that change within a \
            case keep their first value.",
    widgets: &[
        Widget::Variables {
            id: "ids",
            label: "Identify each case by",
            kinds: &[],
            min: 1,
            hint: "One row is made for each combination.",
        },
        Widget::Variable {
            id: "when",
            label: "Spread over",
            kinds: &[],
            required: true,
            hint: "Each of its values becomes a column.",
        },
        Widget::Variables {
            id: "values",
            label: "Variables to spread",
            kinds: &[],
            min: 1,
            hint: "",
        },
        Widget::Text {
            id: "sep",
            label: "Join names with",
            default: TextDefault::Fixed("_"),
            rule: TextRule::Any,
            hint: "income and 2024 become income_2024.",
        },
        RESULT,
    ],
    needs: &[],
    styles: BASE,
    check: wide_check,
    code: wide_code,
};

fn wide_check(a: &Args) -> Vec<Problem> {
    let when = a.var_text("when");
    let ids = a.vars("ids");
    let values = a.vars("values");
    if ids.contains(&when) || values.contains(&when) {
        return vec![Problem::new("when", format!("{when} is used twice."))];
    }
    if let Some(v) = values.iter().find(|v| ids.contains(v)) {
        return vec![Problem::new(
            "values",
            format!("{v} is also an identifier."),
        )];
    }
    Vec::new()
}

fn wide_code(a: &Args, _: Style) -> String {
    format!(
        "{res} <- (function(d) {{\n  wide <- stats::reshape(d, direction = \"wide\", idvar = {ids}, \
         timevar = {when}, v.names = {values}, sep = {sep})\n  rownames(wide) <- NULL\n  \
         attr(wide, \"reshapeWide\") <- NULL\n  wide\n}})({data})",
        res = a.name("result"),
        ids = cvec(&a.vars("ids")),
        when = a.var_string("when").unwrap_or_default(),
        values = cvec(&a.vars("values")),
        sep = r_string(a.values.text("sep")),
        data = a.data(),
    )
}

// ---- Duplicates ----

pub const DUPLICATES: Spec = Spec {
    entry: "Duplicates",
    menu: "Data > Rows > Duplicates",
    about: "Find rows that repeat the same values, and drop them or mark them.",
    widgets: &[
        Widget::Variables {
            id: "vars",
            label: "Rows repeat when these match",
            kinds: &[],
            min: 0,
            hint: "Leave empty to compare whole rows.",
        },
        Widget::Choice {
            id: "action",
            label: "Then",
            options: &[
                Opt {
                    value: "drop",
                    label: "Drop the repeats, keeping the first of each",
                },
                Opt {
                    value: "only",
                    label: "Keep only rows that have a repeat",
                },
                Opt {
                    value: "mark",
                    label: "Add a variable marking them",
                },
            ],
            default: "drop",
            hint: "",
        },
        Widget::Text {
            id: "into",
            label: "Marking variable",
            default: TextDefault::Fixed("duplicated"),
            rule: TextRule::Name,
            hint: "Used only when marking: TRUE for a row that has a repeat.",
        },
        RESULT,
    ],
    needs: &[],
    styles: BASE,
    check: no_check,
    code: duplicates_code,
};

fn duplicates_code(a: &Args, _: Style) -> String {
    let keys = columns(a, &a.vars("vars"));
    let res = a.name("result");
    let data = a.data();
    match a.choice("action") {
        "only" => format!(
            "{res} <- {data}[duplicated({keys}) | duplicated({keys}, fromLast = TRUE), , drop = FALSE]"
        ),
        "mark" => format!(
            "{}{res}[[{}]] <- duplicated({keys}) | duplicated({keys}, fromLast = TRUE)",
            copy_line(a),
            a.name_string("into"),
        ),
        _ => format!("{res} <- {data}[!duplicated({keys}), , drop = FALSE]"),
    }
}

// ---- Random sample ----

pub const SAMPLE: Spec = Spec {
    entry: "Random sample",
    menu: "Data > Rows > Random sample",
    about: "Keep a random subset of the rows.",
    widgets: &[
        Widget::Choice {
            id: "how",
            label: "Size is a",
            options: &[
                Opt {
                    value: "count",
                    label: "Number of rows",
                },
                Opt {
                    value: "percent",
                    label: "Percentage of rows",
                },
            ],
            default: "count",
            hint: "",
        },
        Widget::Text {
            id: "amount",
            label: "Size",
            default: TextDefault::Fixed("100"),
            rule: TextRule::Number,
            hint: "More rows than there are keeps them all.",
        },
        Widget::Text {
            id: "seed",
            label: "Random seed",
            default: TextDefault::Empty,
            rule: TextRule::Any,
            hint: "A whole number makes the same sample every time; empty gives a new one.",
        },
        RESULT,
    ],
    needs: &[],
    styles: BASE,
    check: sample_check,
    code: sample_code,
};

fn sample_check(a: &Args) -> Vec<Problem> {
    let n: f64 = a.number("amount").parse().unwrap_or(0.0);
    let mut out = Vec::new();
    if a.choice("how") == "percent" {
        if !(n > 0.0 && n <= 100.0) {
            out.push(Problem::new(
                "amount",
                "A percentage is above 0 and at most 100.",
            ));
        }
    } else if n < 1.0 || n.fract() != 0.0 {
        out.push(Problem::new(
            "amount",
            "A number of rows is a whole number, 1 or more.",
        ));
    }
    let seed = a.values.text("seed").trim();
    if !seed.is_empty() && seed.parse::<i32>().is_err() {
        out.push(Problem::new("seed", "A seed is a whole number."));
    }
    out
}

fn sample_code(a: &Args, _: Style) -> String {
    let data = a.data();
    let size = if a.choice("how") == "percent" {
        format!("round(nrow({data}) * {} / 100)", a.number("amount"))
    } else {
        format!("min({}, nrow({data}))", a.number("amount"))
    };
    let seed = a.values.text("seed").trim();
    let set = if seed.is_empty() {
        String::new()
    } else {
        format!("set.seed({seed})\n")
    };
    format!(
        "{set}{} <- {data}[sample(nrow({data}), {size}), , drop = FALSE]",
        a.name("result")
    )
}

// ---- Drop missing values ----

pub const DROP_MISSING: Spec = Spec {
    entry: "Drop missing values",
    menu: "Data > Rows > Drop missing values",
    about: "Drop the rows that have a missing value in any of the chosen variables, or \
            keep only those rows.",
    widgets: &[
        Widget::Variables {
            id: "vars",
            label: "Look at",
            kinds: &[],
            min: 0,
            hint: "Leave empty to look at every variable.",
        },
        Widget::Choice {
            id: "which",
            label: "Rows to",
            options: &[
                Opt {
                    value: "drop",
                    label: "Drop when something is missing",
                },
                Opt {
                    value: "keep",
                    label: "Keep only when something is missing",
                },
            ],
            default: "drop",
            hint: "",
        },
        RESULT,
    ],
    needs: &[],
    styles: BASE,
    check: no_check,
    code: missing_code,
};

fn missing_code(a: &Args, _: Style) -> String {
    let not = if a.choice("which") == "keep" { "!" } else { "" };
    format!(
        "{} <- {}[{not}complete.cases({}), , drop = FALSE]",
        a.name("result"),
        a.data(),
        columns(a, &a.vars("vars")),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use rcraft_rcodegen::dialog::{ColumnInfo, Context};

    fn ctx() -> Context {
        let col = |name: &str, kind| ColumnInfo {
            name: name.into(),
            kind,
        };
        Context {
            dataset: "d".into(),
            columns: vec![
                col("k", VarKind::Number),
                col("g", VarKind::Factor),
                col("x", VarKind::Number),
            ],
        }
    }

    fn code(spec: &Spec, set: &[(&str, &str)], lists: &[(&str, &[&str])]) -> String {
        let c = ctx();
        let mut v = spec.defaults(&c);
        for (id, t) in set {
            v.set_text(id, *t);
        }
        for (id, l) in lists {
            v.set_list(id, l.iter().map(|s| s.to_string()).collect());
        }
        spec.generate(&c, &v, Style::Base).unwrap_or_default()
    }

    fn refused(spec: &Spec, set: &[(&str, &str)], lists: &[(&str, &[&str])]) -> bool {
        let c = ctx();
        let mut v = spec.defaults(&c);
        for (id, t) in set {
            v.set_text(id, *t);
        }
        for (id, l) in lists {
            v.set_list(id, l.iter().map(|s| s.to_string()).collect());
        }
        spec.generate(&c, &v, Style::Base).is_err()
    }

    #[test]
    fn join_flags_follow_the_choice() {
        let c = code(
            &JOIN,
            &[("other", "e"), ("keep", "left"), ("result", "j")],
            &[("keys", &["k"])],
        );
        assert_eq!(
            c,
            "j <- merge(d, e, by = c(\"k\"), all.x = TRUE, all.y = FALSE)"
        );
    }

    #[test]
    fn join_refuses_itself_and_no_keys() {
        assert!(refused(&JOIN, &[("other", "d")], &[("keys", &["k"])]));
        assert!(refused(&JOIN, &[("other", "e")], &[]));
        assert!(refused(&JOIN, &[], &[("keys", &["k"])]));
    }

    #[test]
    fn aggregate_counts_and_refuses_overlap() {
        let c = code(
            &AGGREGATE,
            &[("how", "count")],
            &[("groups", &["g"]), ("values", &["x"])],
        );
        assert!(c.contains("FUN = function(x) sum(!is.na(x))"), "{c}");
        assert!(refused(
            &AGGREGATE,
            &[],
            &[("groups", &["x"]), ("values", &["x"])]
        ));
    }

    #[test]
    fn long_needs_ids_when_the_data_has_an_id_variable() {
        let mut c = ctx();
        c.columns.push(ColumnInfo {
            name: "id".into(),
            kind: VarKind::Number,
        });
        let mut v = WIDE_TO_LONG.defaults(&c);
        v.set_list("stack", vec!["k".into(), "x".into()]);
        assert!(WIDE_TO_LONG.generate(&c, &v, Style::Base).is_err());
        v.set_list("ids", vec!["g".into()]);
        assert!(WIDE_TO_LONG.generate(&c, &v, Style::Base).is_ok());
    }

    #[test]
    fn sample_checks_its_numbers() {
        let one = |set: &[(&str, &str)]| refused(&SAMPLE, set, &[]);
        assert!(one(&[("amount", "0")]));
        assert!(one(&[("amount", "2.5")]));
        assert!(one(&[("how", "percent"), ("amount", "120")]));
        assert!(one(&[("seed", "abc")]));
        assert!(!one(&[("how", "percent"), ("amount", "40"), ("seed", "7")]));
    }
}
