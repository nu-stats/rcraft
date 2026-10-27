//! Statistics menu, tests and intervals (P4.2): t-tests, a test of proportions,
//! rank tests, variance tests and exact tests. Base R only (the stats package
//! ships with R). Each prints R's own test report in the Results window and
//! changes no data. Where a test needs a variable that must have exactly two
//! groups, R is the one that checks: it refuses a grouping variable with other
//! than two levels, and its message is shown.

use rcraft_rcodegen::dialog::{no_check, Args, Opt, Problem, Spec, TextDefault, TextRule};
use rcraft_rcodegen::dialog::{VarKind, Widget};
use rcraft_rcodegen::r_string;

use crate::data::BASE;

const NUMBERS: &[VarKind] = &[VarKind::Number];
const GROUPS: &[VarKind] = &[VarKind::Factor, VarKind::Text, VarKind::Logical];

const ALTERNATIVES: &[Opt] = &[
    Opt {
        value: "two.sided",
        label: "Different (two-sided)",
    },
    Opt {
        value: "less",
        label: "Smaller (one-sided)",
    },
    Opt {
        value: "greater",
        label: "Larger (one-sided)",
    },
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

const ALTERNATIVE: Widget = Widget::Choice {
    id: "alternative",
    label: "Alternative",
    options: ALTERNATIVES,
    default: "two.sided",
    hint: "",
};

const CONFIDENCE: Widget = Widget::Choice {
    id: "conf",
    label: "Confidence level",
    options: LEVELS,
    default: "0.95",
    hint: "",
};

const fn outcome(label: &'static str) -> Widget {
    Widget::Variable {
        id: "y",
        label,
        kinds: NUMBERS,
        required: true,
        hint: "",
    }
}

const GROUP: Widget = Widget::Variable {
    id: "g",
    label: "Groups",
    kinds: GROUPS,
    required: true,
    hint: "Needs exactly two groups here; R says so if there are more.",
};

const GROUPS_ANY: Widget = Widget::Variable {
    id: "g",
    label: "Groups",
    kinds: GROUPS,
    required: true,
    hint: "",
};

/// `alternative = "less", conf.level = 0.95`
fn tail(a: &Args) -> String {
    format!(
        "alternative = {}, conf.level = {}",
        r_string(a.choice("alternative")),
        a.choice("conf"),
    )
}

// ---- t-tests ----

pub const ONE_T: Spec = Spec {
    entry: "One-sample t-test",
    menu: "Statistics > Tests and intervals > One-sample t-test",
    about: "Test whether the mean of a number variable differs from a value, with a confidence \
            interval for the mean.",
    widgets: &[
        outcome("Variable"),
        Widget::Text {
            id: "mu",
            label: "Mean under the null",
            default: TextDefault::Fixed("0"),
            rule: TextRule::Number,
            hint: "The value the mean is compared with.",
        },
        ALTERNATIVE,
        CONFIDENCE,
    ],
    styles: BASE,
    check: no_check,
    code: |a, _| {
        format!(
            "stats::t.test({}[[{}]], mu = {}, {})",
            a.data(),
            a.var_string("y").unwrap_or_default(),
            a.number("mu"),
            tail(a),
        )
    },
};

const VARIANCES: &[Opt] = &[
    Opt {
        value: "welch",
        label: "Not equal (Welch, the safer default)",
    },
    Opt {
        value: "pooled",
        label: "Equal (pooled)",
    },
];

pub const TWO_T: Spec = Spec {
    entry: "Two-sample t-test",
    menu: "Statistics > Tests and intervals > Two-sample t-test",
    about: "Compare the means of a number variable between two groups.",
    widgets: &[
        outcome("Variable"),
        GROUP,
        Widget::Choice {
            id: "variance",
            label: "Group variances",
            options: VARIANCES,
            default: "welch",
            hint: "",
        },
        ALTERNATIVE,
        CONFIDENCE,
    ],
    styles: BASE,
    check: no_check,
    code: |a, _| {
        format!(
            "stats::t.test({} ~ {}, data = {}, var.equal = {}, {})",
            a.var("y").unwrap_or_default(),
            a.var("g").unwrap_or_default(),
            a.data(),
            if a.choice("variance") == "pooled" {
                "TRUE"
            } else {
                "FALSE"
            },
            tail(a),
        )
    },
};

fn pair_check(a: &Args) -> Vec<Problem> {
    if a.var_text("x") == a.var_text("y") {
        vec![Problem::new("y", "Choose two different variables.")]
    } else {
        Vec::new()
    }
}

const fn pair_widgets(second: &'static str) -> [Widget; 2] {
    [
        Widget::Variable {
            id: "x",
            label: "First variable",
            kinds: NUMBERS,
            required: true,
            hint: "",
        },
        Widget::Variable {
            id: "y",
            label: second,
            kinds: NUMBERS,
            required: true,
            hint: "Row i of the first is paired with row i of the second.",
        },
    ]
}

const PAIR: [Widget; 2] = pair_widgets("Second variable");

pub const PAIRED_T: Spec = Spec {
    entry: "Paired t-test",
    menu: "Statistics > Tests and intervals > Paired t-test",
    about: "Test whether the mean difference between two measurements on the same rows is zero. \
            Rows missing either value are left out.",
    widgets: &[PAIR[0], PAIR[1], ALTERNATIVE, CONFIDENCE],
    styles: BASE,
    check: pair_check,
    code: |a, _| {
        format!(
            "stats::t.test({d}[[{x}]], {d}[[{y}]], paired = TRUE, {})",
            tail(a),
            d = a.data(),
            x = a.var_string("x").unwrap_or_default(),
            y = a.var_string("y").unwrap_or_default(),
        )
    },
};

// ---- proportions and exact tests ----

/// The outcome, the value that counts as a success, and a hint.
const SUCCESS: Widget = Widget::Text {
    id: "success",
    label: "Counts as a success",
    default: TextDefault::Empty,
    rule: TextRule::Name,
    hint: "The value of the variable to count, exactly as it appears, for example: yes or 1.",
};

const OUTCOME_ANY: Widget = Widget::Variable {
    id: "y",
    label: "Variable",
    kinds: &[],
    required: true,
    hint: "",
};

const P0: Widget = Widget::Text {
    id: "p0",
    label: "Proportion under the null",
    default: TextDefault::Fixed("0.5"),
    rule: TextRule::Number,
    hint: "",
};

fn p0_check(a: &Args) -> Vec<Problem> {
    match a.number("p0").parse::<f64>() {
        Ok(p) if (0.0..=1.0).contains(&p) => Vec::new(),
        _ => vec![Problem::new("p0", "Type a proportion from 0 to 1.")],
    }
}

/// `sum(x == "yes", na.rm = TRUE)` and `sum(!is.na(x))` for the chosen outcome.
fn counts(a: &Args) -> (String, String) {
    let col = format!("{}[[{}]]", a.data(), a.var_string("y").unwrap_or_default());
    (
        format!("sum({col} == {}, na.rm = TRUE)", a.name_string("success")),
        format!("sum(!is.na({col}))"),
    )
}

fn proportions_check(a: &Args) -> Vec<Problem> {
    if a.var_text("g").is_empty() {
        return p0_check(a);
    }
    if a.var_text("g") == a.var_text("y") {
        return vec![Problem::new(
            "g",
            "Choose a different variable for the groups.",
        )];
    }
    Vec::new()
}

pub const PROPORTIONS: Spec = Spec {
    entry: "Test of proportions",
    menu: "Statistics > Tests and intervals > Test of proportions",
    about: "Test one proportion against a value, or compare the proportions between groups \
            (R's prop.test, with continuity correction). Missing values are left out.",
    widgets: &[
        OUTCOME_ANY,
        SUCCESS,
        Widget::Variable {
            id: "g",
            label: "Compare groups",
            kinds: GROUPS,
            required: false,
            hint: "Optional. Leave empty to test one proportion against the value below.",
        },
        P0,
        CONFIDENCE,
    ],
    styles: BASE,
    check: proportions_check,
    code: |a, _| {
        let conf = a.choice("conf");
        if a.var_text("g").is_empty() {
            let (hits, n) = counts(a);
            return format!(
                "stats::prop.test({hits}, {n}, p = {}, conf.level = {conf})",
                a.number("p0")
            );
        }
        format!(
            "(function(x, g, s) {{\n  keep <- !is.na(x) & !is.na(g)\n  g <- droplevels(as.factor(g[keep]))\n  \
             hits <- tapply(x[keep] == s, g, sum)\n  n <- tapply(x[keep], g, length)\n  \
             stats::prop.test(as.vector(hits), as.vector(n), conf.level = {conf})\n}})\
             ({d}[[{y}]], {d}[[{g}]], {s})",
            d = a.data(),
            y = a.var_string("y").unwrap_or_default(),
            g = a.var_string("g").unwrap_or_default(),
            s = a.name_string("success"),
        )
    },
};

pub const BINOMIAL: Spec = Spec {
    entry: "Exact binomial test",
    menu: "Statistics > Tests and intervals > Exact binomial test",
    about: "Exact test of one proportion against a value, with an exact confidence interval. \
            Missing values are left out.",
    widgets: &[OUTCOME_ANY, SUCCESS, P0, ALTERNATIVE, CONFIDENCE],
    styles: BASE,
    check: p0_check,
    code: |a, _| {
        let (hits, n) = counts(a);
        format!(
            "stats::binom.test({hits}, {n}, p = {}, {})",
            a.number("p0"),
            tail(a)
        )
    },
};

pub const FISHER: Spec = Spec {
    entry: "Fisher's exact test",
    menu: "Statistics > Tests and intervals > Fisher's exact test",
    about: "Exact test of whether two categorical variables are independent, for small counts. \
            Missing values are left out.",
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
        format!(
            "stats::fisher.test(table({d}[[{r}]], {d}[[{c}]]))",
            d = a.data(),
            r = a.var_string("row").unwrap_or_default(),
            c = a.var_string("col").unwrap_or_default(),
        )
    },
};

// ---- rank tests ----

pub const MANN_WHITNEY: Spec = Spec {
    entry: "Mann-Whitney test",
    menu: "Statistics > Tests and intervals > Mann-Whitney test",
    about: "Compare a number variable between two groups by ranks, without assuming it is \
            normal (the Wilcoxon rank-sum test).",
    widgets: &[outcome("Variable"), GROUP, ALTERNATIVE],
    styles: BASE,
    check: no_check,
    code: |a, _| {
        format!(
            "stats::wilcox.test({} ~ {}, data = {}, alternative = {})",
            a.var("y").unwrap_or_default(),
            a.var("g").unwrap_or_default(),
            a.data(),
            r_string(a.choice("alternative")),
        )
    },
};

pub const SIGNED_RANK: Spec = Spec {
    entry: "Wilcoxon signed-rank test",
    menu: "Statistics > Tests and intervals > Wilcoxon signed-rank test",
    about: "Test whether paired measurements differ, by ranks (the paired non-parametric test). \
            Rows missing either value are left out.",
    widgets: &[PAIR[0], PAIR[1], ALTERNATIVE],
    styles: BASE,
    check: pair_check,
    code: |a, _| {
        format!(
            "stats::wilcox.test({d}[[{x}]], {d}[[{y}]], paired = TRUE, alternative = {})",
            r_string(a.choice("alternative")),
            d = a.data(),
            x = a.var_string("x").unwrap_or_default(),
            y = a.var_string("y").unwrap_or_default(),
        )
    },
};

pub const KRUSKAL: Spec = Spec {
    entry: "Kruskal-Wallis test",
    menu: "Statistics > Tests and intervals > Kruskal-Wallis test",
    about: "Compare a number variable between two or more groups by ranks.",
    widgets: &[outcome("Variable"), GROUPS_ANY],
    styles: BASE,
    check: no_check,
    code: |a, _| {
        format!(
            "stats::kruskal.test({} ~ {}, data = {})",
            a.var("y").unwrap_or_default(),
            a.var("g").unwrap_or_default(),
            a.data(),
        )
    },
};

// ---- variance tests ----

const VARIANCE_TESTS: &[Opt] = &[
    Opt {
        value: "levene",
        label: "Brown-Forsythe (Levene, centred on the median)",
    },
    Opt {
        value: "bartlett",
        label: "Bartlett (assumes normal data)",
    },
    Opt {
        value: "f",
        label: "F test (exactly two groups, assumes normal data)",
    },
];

pub const VARIANCE: Spec = Spec {
    entry: "Variance test",
    menu: "Statistics > Tests and intervals > Variance test",
    about: "Test whether a number variable has the same spread in every group.",
    widgets: &[
        outcome("Variable"),
        GROUPS_ANY,
        Widget::Choice {
            id: "test",
            label: "Test",
            options: VARIANCE_TESTS,
            default: "levene",
            hint: "Brown-Forsythe holds up best when the data are not normal.",
        },
    ],
    styles: BASE,
    check: no_check,
    code: |a, _| {
        let (y, g, d) = (
            a.var("y").unwrap_or_default(),
            a.var("g").unwrap_or_default(),
            a.data(),
        );
        match a.choice("test") {
            "bartlett" => format!("stats::bartlett.test({y} ~ {g}, data = {d})"),
            "f" => format!("stats::var.test({y} ~ {g}, data = {d})"),
            _ => format!(
                "(function(y, g) {{\n  g <- factor(g)\n  \
                 cat(\"Brown-Forsythe test of equal variances (Levene's test centred on the median)\\n\")\n  \
                 spread <- abs(y - stats::ave(y, g, FUN = function(v) stats::median(v, na.rm = TRUE)))\n  \
                 stats::anova(stats::lm(spread ~ g))\n}})({d}[[{ys}]], {d}[[{gs}]])",
                ys = a.var_string("y").unwrap_or_default(),
                gs = a.var_string("g").unwrap_or_default(),
            ),
        }
    },
};

/// The dialogs of this module, in menu order.
pub const SPECS: &[&Spec] = &[
    &ONE_T,
    &TWO_T,
    &PAIRED_T,
    &PROPORTIONS,
    &BINOMIAL,
    &FISHER,
    &MANN_WHITNEY,
    &SIGNED_RANK,
    &KRUSKAL,
    &VARIANCE,
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
                col("blood pressure", VarKind::Number),
                col("after", VarKind::Number),
                col("arm", VarKind::Factor),
                col("smoker", VarKind::Text),
                col("born", VarKind::Date),
            ],
        }
    }

    fn make(spec: &Spec, set: &[(&str, &str)]) -> Result<String, Vec<Problem>> {
        let c = ctx();
        let mut v: Values = spec.defaults(&c);
        for (id, text) in set {
            v.set_text(id, *text);
        }
        spec.generate(&c, &v, Style::Base)
    }

    fn code(spec: &Spec, set: &[(&str, &str)]) -> String {
        match make(spec, set) {
            Ok(code) => code,
            Err(p) => format!("problems: {p:?}"),
        }
    }

    fn message(spec: &Spec, set: &[(&str, &str)]) -> String {
        match make(spec, set) {
            Ok(code) => format!("no problem: {code}"),
            Err(p) => p
                .iter()
                .map(|p| p.message.clone())
                .collect::<Vec<_>>()
                .join(" "),
        }
    }

    #[test]
    fn t_tests_write_the_chosen_options() {
        assert_eq!(
            code(&ONE_T, &[("y", "blood pressure"), ("mu", " 120 ")]),
            "stats::t.test(`my data`[[\"blood pressure\"]], mu = 120, \
             alternative = \"two.sided\", conf.level = 0.95)"
        );
        assert_eq!(
            code(
                &TWO_T,
                &[
                    ("y", "blood pressure"),
                    ("g", "arm"),
                    ("variance", "pooled"),
                    ("alternative", "less"),
                    ("conf", "0.99")
                ]
            ),
            "stats::t.test(`blood pressure` ~ arm, data = `my data`, var.equal = TRUE, \
             alternative = \"less\", conf.level = 0.99)"
        );
        assert_eq!(
            code(&PAIRED_T, &[("x", "blood pressure"), ("y", "after")]),
            "stats::t.test(`my data`[[\"blood pressure\"]], `my data`[[\"after\"]], paired = TRUE, \
             alternative = \"two.sided\", conf.level = 0.95)"
        );
    }

    #[test]
    fn numbers_and_pairs_are_checked() {
        assert!(message(&ONE_T, &[("y", "after"), ("mu", "")]).contains("Type"));
        assert!(message(&ONE_T, &[("y", "after"), ("mu", "inf")]).contains("not a number"));
        assert!(message(&ONE_T, &[("y", "after"), ("mu", "abc")]).contains("not a number"));
        assert!(message(&ONE_T, &[("y", "after"), ("mu", "1e2")]).starts_with("no problem"));
        assert!(message(&ONE_T, &[("y", "arm")]).contains("needs number"));
        assert!(message(&PAIRED_T, &[("x", "after"), ("y", "after")]).contains("two different"));
        assert!(message(&TWO_T, &[("y", "after"), ("g", "born")]).contains("needs"));
        assert!(message(&TWO_T, &[("y", "after"), ("g", "after")]).contains("needs"));
        assert!(message(
            &TWO_T,
            &[("y", "after"), ("g", "arm"), ("alternative", "x")]
        )
        .contains("Pick one"));
    }

    #[test]
    fn proportions_count_a_success_value_with_or_without_groups() {
        let one = code(
            &PROPORTIONS,
            &[("y", "smoker"), ("success", "yes"), ("p0", "0.3")],
        );
        assert_eq!(
            one,
            "stats::prop.test(sum(`my data`[[\"smoker\"]] == \"yes\", na.rm = TRUE), \
             sum(!is.na(`my data`[[\"smoker\"]])), p = 0.3, conf.level = 0.95)"
        );
        let grouped = code(
            &PROPORTIONS,
            &[("y", "smoker"), ("success", "yes"), ("g", "arm")],
        );
        assert!(
            grouped.contains("tapply(x[keep] == s, g, sum)"),
            "{grouped}"
        );
        assert!(
            grouped.ends_with("(`my data`[[\"smoker\"]], `my data`[[\"arm\"]], \"yes\")"),
            "{grouped}"
        );
        assert!(
            !grouped.contains("p ="),
            "p0 is for the one-sample test only"
        );
        // p0 is checked only when it is used
        assert!(message(
            &PROPORTIONS,
            &[("y", "smoker"), ("success", "yes"), ("p0", "1.5")]
        )
        .contains("from 0 to 1"));
        assert!(message(
            &PROPORTIONS,
            &[
                ("y", "smoker"),
                ("success", "yes"),
                ("g", "arm"),
                ("p0", "1.5")
            ]
        )
        .starts_with("no problem"));
        assert!(message(&PROPORTIONS, &[("y", "smoker"), ("success", "")]).contains("success"));
        assert!(message(
            &PROPORTIONS,
            &[("y", "smoker"), ("success", "yes"), ("g", "smoker")]
        )
        .contains("different"));
        assert!(message(
            &BINOMIAL,
            &[("y", "smoker"), ("success", "yes"), ("p0", "-1")]
        )
        .contains("from 0 to 1"));
    }

    #[test]
    fn rank_variance_and_exact_tests_write_their_calls() {
        assert_eq!(
            code(&MANN_WHITNEY, &[("y", "after"), ("g", "arm")]),
            "stats::wilcox.test(after ~ arm, data = `my data`, alternative = \"two.sided\")"
        );
        assert_eq!(
            code(&KRUSKAL, &[("y", "after"), ("g", "smoker")]),
            "stats::kruskal.test(after ~ smoker, data = `my data`)"
        );
        assert!(
            code(&SIGNED_RANK, &[("x", "after"), ("y", "blood pressure")])
                .contains("paired = TRUE")
        );
        assert!(
            code(&VARIANCE, &[("y", "after"), ("g", "arm"), ("test", "f")])
                .starts_with("stats::var.test(after ~ arm")
        );
        assert!(
            code(&VARIANCE, &[("y", "after"), ("g", "arm")]).contains("Brown-Forsythe"),
            "the default is Brown-Forsythe"
        );
        assert_eq!(
            code(&FISHER, &[("row", "smoker"), ("col", "arm")]),
            "stats::fisher.test(table(`my data`[[\"smoker\"]], `my data`[[\"arm\"]]))"
        );
        assert!(message(&FISHER, &[("row", "arm"), ("col", "arm")]).contains("different"));
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
            assert!(spec.menu.starts_with("Statistics > Tests and intervals > "));
            assert!(
                crate::stats::SPECS.iter().any(|s| s.menu == spec.menu),
                "{} is not in the menu",
                spec.entry
            );
        }
    }
}
