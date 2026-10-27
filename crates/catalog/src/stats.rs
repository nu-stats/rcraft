//! Statistics menu, descriptives (P4.1): summary statistics, frequency tables,
//! cross-tabulation, correlation, missing-data patterns and a normality test.
//! Base R only (the stats package ships with R), so each runs against any R.
//! They print their tables in the Results window; none changes the data. Each
//! is one call of an anonymous function on the chosen columns, so no helper
//! name lands in the user's workspace.

use rcraft_rcodegen::dialog::{no_check, Args, Opt, Problem, Spec, TextDefault, TextRule, VarKind};
use rcraft_rcodegen::dialog::{Style, Widget};
use rcraft_rcodegen::r_string;

use crate::data::BASE;

/// `c("a", "b")` for column names.
fn names_vector(names: &[&str]) -> String {
    let quoted: Vec<String> = names.iter().map(|n| r_string(n)).collect();
    format!("c({})", quoted.join(", "))
}

/// The chosen columns as a data frame subset: `people[c("a", "b")]`.
fn subset(a: &Args, names: &[&str]) -> String {
    format!("{}[{}]", a.data(), names_vector(names))
}

const NUMBERS: &[VarKind] = &[VarKind::Number];
const GROUPS: &[VarKind] = &[VarKind::Factor, VarKind::Text, VarKind::Logical];

// ---- Summary statistics ----

fn summary_code(a: &Args, _: Style) -> String {
    let vars = a.vars_or_all("vars", NUMBERS);
    let by = a
        .var_string("by")
        .map_or_else(|| "NULL".to_string(), |s| format!("{}[[{s}]]", a.data()));
    let call = format!(
        "(function(d, g) {{\n  s <- function(d) t(sapply(d, function(x) {{\n    x <- as.numeric(x)\n    \
         q <- stats::quantile(x, c(0, 0.25, 0.5, 0.75, 1), na.rm = TRUE, names = FALSE)\n    \
         c(n = sum(!is.na(x)), missing = sum(is.na(x)), mean = mean(x, na.rm = TRUE),\n      \
         sd = stats::sd(x, na.rm = TRUE), min = q[1], q25 = q[2], median = q[3], q75 = q[4], max = q[5])\n  \
         }}))\n  if (is.null(g)) s(d) else lapply(split(d, g, drop = TRUE), s)\n}})({}, {by})",
        subset(a, &vars),
    );
    let save = a.name_text("save");
    if save.is_empty() {
        call
    } else {
        format!("{} <- {call}\n{}", a.name("save"), a.name("save"))
    }
}

fn summary_check(a: &Args) -> Vec<Problem> {
    if a.vars_or_all("vars", NUMBERS).is_empty() {
        vec![Problem::new(
            "vars",
            format!("{} has no number variables.", a.ctx.dataset),
        )]
    } else {
        Vec::new()
    }
}

pub const SUMMARY: Spec = Spec {
    entry: "Summary statistics",
    menu: "Statistics > Descriptives > Summary statistics",
    about: "Count, missing values, mean, standard deviation, minimum, quartiles and maximum of \
            number variables, all together or split by a group.",
    widgets: &[
        Widget::Variables {
            id: "vars",
            label: "Variables",
            kinds: NUMBERS,
            min: 0,
            hint: "Choose none to use every number variable.",
        },
        Widget::Variable {
            id: "by",
            label: "Split by",
            kinds: GROUPS,
            required: false,
            hint: "Optional. Rows with a missing group value are left out.",
        },
        Widget::Text {
            id: "save",
            label: "Also save the table as",
            default: TextDefault::Empty,
            rule: TextRule::Any,
            hint: "Optional: a name for the table, so later commands can use it.",
        },
    ],
    styles: BASE,
    check: summary_check,
    code: summary_code,
};

// ---- Frequency table ----

pub const FREQUENCY: Spec = Spec {
    entry: "Frequency table",
    menu: "Statistics > Descriptives > Frequency table",
    about: "How many rows have each value of a variable, and what share of the rows that is.",
    widgets: &[
        Widget::Variables {
            id: "vars",
            label: "Variables",
            kinds: &[],
            min: 1,
            hint: "One table is shown for each.",
        },
        Widget::Check {
            id: "percent",
            label: "Show percentages",
            default: true,
            hint: "",
        },
        Widget::Check {
            id: "missing",
            label: "Count missing values as a category",
            default: true,
            hint: "",
        },
    ],
    styles: BASE,
    check: no_check,
    code: |a, _| {
        let vars = a.vars("vars");
        let use_na = if a.flag("missing") { "ifany" } else { "no" };
        let table = if a.flag("percent") {
            "cbind(count = t, percent = round(100 * t / sum(t), 1))"
        } else {
            "cbind(count = t)"
        };
        format!(
            "invisible(lapply({}, function(v) {{\n  cat(\"\\n\", v, \"\\n\", sep = \"\")\n  \
             t <- table({}[[v]], useNA = \"{use_na}\")\n  print({table})\n}}))",
            names_vector(&vars),
            a.data(),
        )
    },
};

// ---- Cross-tabulation ----

const SHARES: &[Opt] = &[
    Opt {
        value: "none",
        label: "None (counts only)",
    },
    Opt {
        value: "row",
        label: "Of each row",
    },
    Opt {
        value: "column",
        label: "Of each column",
    },
    Opt {
        value: "total",
        label: "Of the whole table",
    },
];

pub const CROSSTAB: Spec = Spec {
    entry: "Cross-tabulation",
    menu: "Statistics > Descriptives > Cross-tabulation",
    about: "Count the rows for every pair of values of two variables, with shares and a \
            chi-squared test if you want them.",
    widgets: &[
        Widget::Variable {
            id: "row",
            label: "Rows",
            kinds: &[],
            required: true,
            hint: "",
        },
        Widget::Variable {
            id: "col",
            label: "Columns",
            kinds: &[],
            required: true,
            hint: "",
        },
        Widget::Choice {
            id: "share",
            label: "Percentages",
            options: SHARES,
            default: "none",
            hint: "",
        },
        Widget::Check {
            id: "test",
            label: "Chi-squared test of independence",
            default: false,
            hint: "R warns when some expected counts are small.",
        },
    ],
    styles: BASE,
    check: |a| {
        if a.var_text("row") == a.var_text("col") {
            vec![Problem::new(
                "col",
                "Choose a different variable for the columns.",
            )]
        } else {
            Vec::new()
        }
    },
    code: |a, _| {
        let (row, col) = (
            a.var_string("row").unwrap_or_default(),
            a.var_string("col").unwrap_or_default(),
        );
        let share = match a.choice("share") {
            "row" => Some(("Row percentages", "prop.table(t, 1)")),
            "column" => Some(("Column percentages", "prop.table(t, 2)")),
            "total" => Some(("Percentages of the whole table", "prop.table(t)")),
            _ => None,
        };
        let mut body = String::from("  print(stats::addmargins(t))\n");
        if let Some((title, shares)) = share {
            body.push_str(&format!(
                "  cat(\"\\n{title}\\n\")\n  print(round(100 * {shares}, 1))\n"
            ));
        }
        if a.flag("test") {
            body.push_str("  print(stats::chisq.test(t))\n");
        }
        format!(
            "(function(t) {{\n{body}}})(table({d}[[{row}]], {d}[[{col}]], useNA = \"no\", \
             dnn = c({row}, {col})))",
            d = a.data(),
        )
    },
};

// ---- Correlation ----

const METHODS: &[Opt] = &[
    Opt {
        value: "pearson",
        label: "Pearson (straight-line)",
    },
    Opt {
        value: "spearman",
        label: "Spearman (ranks)",
    },
    Opt {
        value: "kendall",
        label: "Kendall (rank pairs)",
    },
];

const USES: &[Opt] = &[
    Opt {
        value: "pairwise",
        label: "Each pair uses all rows it can",
    },
    Opt {
        value: "complete",
        label: "Only rows complete in every variable",
    },
];

pub const CORRELATION: Spec = Spec {
    entry: "Correlation",
    menu: "Statistics > Descriptives > Correlation",
    about: "How strongly pairs of number variables move together, with the number of rows \
            each pair used.",
    widgets: &[
        Widget::Variables {
            id: "vars",
            label: "Variables",
            kinds: NUMBERS,
            min: 2,
            hint: "",
        },
        Widget::Choice {
            id: "method",
            label: "Kind",
            options: METHODS,
            default: "pearson",
            hint: "",
        },
        Widget::Choice {
            id: "use",
            label: "Missing values",
            options: USES,
            default: "pairwise",
            hint: "",
        },
        Widget::Check {
            id: "p",
            label: "Show p-values",
            default: false,
            hint: "Test of zero correlation for each pair. Not adjusted for the number of pairs.",
        },
    ],
    styles: BASE,
    check: no_check,
    code: |a, _| {
        let vars = a.vars("vars");
        let rows = if a.choice("use") == "complete" {
            "  d <- d[stats::complete.cases(d), , drop = FALSE]\n"
        } else {
            ""
        };
        let p = if a.flag("p") {
            "  cat(\"\\nP-values\\n\")\n  \
             print(round(sapply(names(d), function(a) sapply(names(d), function(b) {\n    \
             if (a == b) return(NA_real_)\n    \
             tryCatch(stats::cor.test(d[[a]], d[[b]], method = m)$p.value, error = function(e) NA_real_)\n  \
             })), 4))\n"
        } else {
            ""
        };
        format!(
            "(function(d, m) {{\n  d[] <- lapply(d, as.numeric)\n{rows}  cat(\"Correlations\\n\")\n  \
             print(round(stats::cor(d, use = \"pairwise.complete.obs\", method = m), 3))\n  \
             cat(\"\\nRows used by each pair\\n\")\n  print(crossprod(!is.na(d)))\n{p}}})({}, {})",
            subset(a, &vars),
            r_string(a.choice("method")),
        )
    },
};

// ---- Missing-data patterns ----

pub const MISSING: Spec = Spec {
    entry: "Missing-data patterns",
    menu: "Statistics > Descriptives > Missing-data patterns",
    about: "How many values are missing in each variable, and which variables tend to be \
            missing together.",
    widgets: &[Widget::Variables {
        id: "vars",
        label: "Variables",
        kinds: &[],
        min: 0,
        hint: "Choose none to use every variable.",
    }],
    styles: BASE,
    check: |a| {
        if a.vars_or_all("vars", &[]).is_empty() {
            vec![Problem::new(
                "vars",
                format!("{} has no variables.", a.ctx.dataset),
            )]
        } else {
            Vec::new()
        }
    },
    code: |a, _| {
        let vars = a.vars_or_all("vars", &[]);
        format!(
            "(function(d) {{\n  m <- is.na(d)\n  cat(\"Missing values by variable\\n\")\n  \
             print(cbind(missing = colSums(m), percent = round(100 * colMeans(m), 1)))\n  \
             if (nrow(d) == 0) return(invisible(NULL))\n  \
             pat <- apply(m, 1, function(r) paste(ifelse(r, \"X\", \".\"), collapse = \"\"))\n  \
             cat(\"\\nPatterns (X = missing), one letter per variable in the order above\\n\")\n  \
             tab <- sort(table(pat), decreasing = TRUE)\n  \
             print(data.frame(pattern = names(tab), rows = as.vector(tab)), row.names = FALSE)\n}})({})",
            subset(a, &vars),
        )
    },
};

// ---- Normality test ----

pub const NORMALITY: Spec = Spec {
    entry: "Normality test",
    menu: "Statistics > Descriptives > Normality test",
    about: "Shapiro-Wilk test of whether a number variable looks normally distributed. It needs \
            3 to 5000 values that are not all equal.",
    widgets: &[Widget::Variables {
        id: "vars",
        label: "Variables",
        kinds: NUMBERS,
        min: 1,
        hint: "A small p-value says the values are unlikely to be normal.",
    }],
    styles: BASE,
    check: no_check,
    code: |a, _| {
        let vars = a.vars("vars");
        format!(
            "(function(d) t(sapply(d, function(x) {{\n  x <- as.numeric(x)\n  x <- x[!is.na(x)]\n  \
             if (length(x) < 3 || length(x) > 5000 || stats::var(x) == 0) {{\n    \
             return(c(n = length(x), W = NA_real_, p.value = NA_real_))\n  }}\n  \
             s <- stats::shapiro.test(x)\n  \
             c(n = length(x), W = unname(s$statistic), p.value = s$p.value)\n}})))({})",
            subset(a, &vars),
        )
    },
};

/// The Statistics menu's dialogs, in menu order.
pub const SPECS: &[&Spec] = &[
    &SUMMARY,
    &FREQUENCY,
    &CROSSTAB,
    &CORRELATION,
    &MISSING,
    &NORMALITY,
];

#[cfg(test)]
mod tests {
    use super::*;
    use rcraft_rcodegen::dialog::{ColumnInfo, Context, Values};

    fn ctx() -> Context {
        let col = |name: &str, kind| ColumnInfo {
            name: name.into(),
            kind,
        };
        Context {
            dataset: "my data".into(),
            columns: vec![
                col("age", VarKind::Number),
                col("home town", VarKind::Number),
                col("sex", VarKind::Factor),
                col("born", VarKind::Date),
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
    fn summary_uses_the_chosen_or_every_number_variable() {
        let chosen = code(&SUMMARY, &[], &[("vars", &["home town", "age"])]);
        assert!(
            chosen.ends_with("})(`my data`[c(\"home town\", \"age\")], NULL)"),
            "{chosen}"
        );
        let all = code(&SUMMARY, &[], &[]);
        assert!(
            all.ends_with("})(`my data`[c(\"age\", \"home town\")], NULL)"),
            "dates and factors are not number variables: {all}"
        );
        let split = code(&SUMMARY, &[("by", "sex"), ("save", "my table")], &[]);
        assert!(
            split.starts_with("`my table` <- (function(d, g) {"),
            "{split}"
        );
        assert!(
            split.contains("`my data`[[\"sex\"]])\n`my table`"),
            "{split}"
        );
        assert!(split.contains("split(d, g, drop = TRUE)"));
    }

    #[test]
    fn summary_refuses_a_data_frame_with_no_numbers_and_a_wrong_split() {
        let mut c = ctx();
        c.columns.retain(|c| c.kind != VarKind::Number);
        let v = SUMMARY.defaults(&c);
        let problems = SUMMARY.problems(&c, &v);
        assert_eq!(problems.len(), 1);
        assert!(problems[0].message.contains("no number variables"));
        assert!(message(&SUMMARY, &[("by", "age")], &[]).contains("needs"));
        assert!(message(&SUMMARY, &[], &[("vars", &["sex"])]).contains("factor"));
    }

    #[test]
    fn frequency_needs_a_variable_and_follows_its_boxes() {
        assert!(message(&FREQUENCY, &[], &[]).contains("at least 1 variable"));
        let out = code(&FREQUENCY, &[], &[("vars", &["sex", "age"])]);
        assert!(
            out.contains("lapply(c(\"sex\", \"age\"), function(v) {"),
            "{out}"
        );
        assert!(
            out.contains("table(`my data`[[v]], useNA = \"ifany\")"),
            "{out}"
        );
        assert!(
            out.contains("percent = round(100 * t / sum(t), 1)"),
            "{out}"
        );
        let c = ctx();
        let mut v = FREQUENCY.defaults(&c);
        v.set_list("vars", vec!["sex".into()]);
        v.set_flag("percent", false);
        v.set_flag("missing", false);
        let plain = FREQUENCY.generate(&c, &v, Style::Base).unwrap_or_default();
        assert!(
            plain.contains("cbind(count = t)") && plain.contains("useNA = \"no\""),
            "{plain}"
        );
    }

    #[test]
    fn cross_tabulation_needs_two_different_variables() {
        assert!(message(&CROSSTAB, &[("row", "sex")], &[]).contains("columns"));
        assert!(message(&CROSSTAB, &[("row", "sex"), ("col", "sex")], &[]).contains("different"));
        let out = code(
            &CROSSTAB,
            &[("row", "sex"), ("col", "age"), ("share", "column")],
            &[],
        );
        assert!(out.contains("round(100 * prop.table(t, 2), 1)"), "{out}");
        assert!(out.contains("dnn = c(\"sex\", \"age\")"), "{out}");
        assert!(!out.contains("chisq.test"), "{out}");
        assert!(message(
            &CROSSTAB,
            &[("row", "sex"), ("col", "age"), ("share", "x")],
            &[]
        )
        .contains("Pick one"));
    }

    #[test]
    fn correlation_needs_two_numbers_and_a_known_method() {
        assert!(message(&CORRELATION, &[], &[("vars", &["age"])]).contains("at least 2 variables"));
        let out = code(
            &CORRELATION,
            &[("method", "kendall"), ("use", "complete")],
            &[("vars", &["age", "home town"])],
        );
        assert!(
            out.ends_with("`my data`[c(\"age\", \"home town\")], \"kendall\")"),
            "{out}"
        );
        assert!(out.contains("complete.cases(d)"), "{out}");
        assert!(!out.contains("cor.test"), "p-values only when asked: {out}");
        assert!(message(
            &CORRELATION,
            &[("method", "x")],
            &[("vars", &["age", "home town"])]
        )
        .contains("Pick one"));
    }

    #[test]
    fn missing_patterns_and_normality_take_their_variables() {
        let all = code(&MISSING, &[], &[]);
        assert!(
            all.contains("c(\"age\", \"home town\", \"sex\", \"born\")"),
            "{all}"
        );
        assert!(message(&NORMALITY, &[], &[]).contains("at least 1 variable"));
        assert!(message(&NORMALITY, &[], &[("vars", &["sex"])]).contains("factor"));
        let out = code(&NORMALITY, &[], &[("vars", &["age"])]);
        assert!(out.contains("stats::shapiro.test(x)"), "{out}");
    }

    #[test]
    fn every_dialog_is_a_catalog_entry_in_the_statistics_menu() {
        for spec in SPECS {
            assert!(
                crate::ENTRIES
                    .iter()
                    .any(|e| e.name == spec.entry && e.area == crate::Area::Statistics),
                "{} has no Statistics catalog entry",
                spec.entry
            );
            assert!(spec.menu.starts_with("Statistics > Descriptives > "));
            let mut ids: Vec<&str> = spec.widgets.iter().map(|w| w.id()).collect();
            ids.sort_unstable();
            let n = ids.len();
            ids.dedup();
            assert_eq!(n, ids.len(), "{} repeats a widget id", spec.entry);
        }
    }
}
