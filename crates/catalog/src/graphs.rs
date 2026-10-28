//! Graphs menu (P5.3): the common plots, in base R graphics (no package needed).
//! Each draws on the live graphics device, so the Graph tab shows it and a later
//! command can add to it. None changes the data. A title is optional: left empty,
//! the plot gets a plain one naming its variables.

use rcraft_rcodegen::dialog::{no_check, Args, Opt, Problem, Spec, TextDefault, TextRule};
use rcraft_rcodegen::dialog::{VarKind, Widget};
use rcraft_rcodegen::r_string;

use crate::data::BASE;

const NUMBERS: &[VarKind] = &[VarKind::Number];
const GROUPS: &[VarKind] = &[VarKind::Factor, VarKind::Text, VarKind::Logical];
const CATEGORIES: &[VarKind] = &[
    VarKind::Factor,
    VarKind::Text,
    VarKind::Logical,
    VarKind::Number,
];
const BLUE: &str = "#8fb4e6";

const TITLE: Widget = Widget::Text {
    id: "title",
    label: "Title",
    default: TextDefault::Empty,
    rule: TextRule::Any,
    hint: "Optional. Left empty, the plot is titled with its variables.",
};

const fn number_var(label: &'static str, required: bool, hint: &'static str) -> Widget {
    Widget::Variable {
        id: "x",
        label,
        kinds: NUMBERS,
        required,
        hint,
    }
}

const GROUP: Widget = Widget::Variable {
    id: "g",
    label: "Colour or split by",
    kinds: GROUPS,
    required: false,
    hint: "Optional: one group per colour or box.",
};

/// The title in R, or `fallback` when none was typed.
fn title(a: &Args, fallback: &str) -> String {
    let typed = a.values.text("title").trim();
    r_string(if typed.is_empty() { fallback } else { typed })
}

/// `data[["name"]]` for a chosen column.
fn col(a: &Args, id: &str) -> String {
    format!("{}[[{}]]", a.data(), a.var_string(id).unwrap_or_default())
}

// ---- Histogram ----

const BREAKS: &[Opt] = &[
    Opt {
        value: "Sturges",
        label: "R's default (Sturges)",
    },
    Opt {
        value: "Scott",
        label: "Scott",
    },
    Opt {
        value: "FD",
        label: "Freedman-Diaconis",
    },
];

fn histogram_check(a: &Args) -> Vec<Problem> {
    let bins = a.values.text("bins").trim();
    if bins.is_empty() || bins.parse::<u32>().is_ok_and(|n| (1..=1000).contains(&n)) {
        Vec::new()
    } else {
        vec![Problem::new(
            "bins",
            "Type a whole number of bars from 1 to 1000, or leave it empty.",
        )]
    }
}

pub const HISTOGRAM: Spec = Spec {
    entry: "Histogram",
    menu: "Graphs > One variable > Histogram",
    about: "Bars showing how many rows fall in each range of a number variable.",
    widgets: &[
        number_var("Variable", true, ""),
        Widget::Choice {
            id: "breaks",
            label: "How to choose the bars",
            options: BREAKS,
            default: "Sturges",
            hint: "",
        },
        Widget::Text {
            id: "bins",
            label: "Number of bars",
            default: TextDefault::Empty,
            rule: TextRule::Any,
            hint: "Optional. Used instead of the choice above; R may adjust it to round limits.",
        },
        Widget::Check {
            id: "density",
            label: "Draw the density curve over the bars",
            default: false,
            hint: "The bars then show density, not counts.",
        },
        TITLE,
    ],
    needs: &[],
    styles: BASE,
    check: histogram_check,
    code: |a, _| {
        let name = a.var_text("x");
        let breaks = a.values.text("bins").trim();
        let breaks = if breaks.is_empty() {
            r_string(a.choice("breaks"))
        } else {
            breaks.to_string()
        };
        let x = col(a, "x");
        let density = a.flag("density");
        let mut code = format!(
            "graphics::hist({x}, breaks = {breaks}{}, main = {}, xlab = {}, col = \"{BLUE}\", border = \"white\")",
            if density { ", freq = FALSE" } else { "" },
            title(a, &format!("Histogram of {name}")),
            r_string(name),
        );
        if density {
            code.push_str(&format!(
                "\ngraphics::lines(stats::density({x}, na.rm = TRUE), lwd = 2)"
            ));
        }
        code
    },
};

// ---- Density ----

pub const DENSITY: Spec = Spec {
    entry: "Density plot",
    menu: "Graphs > One variable > Density plot",
    about: "A smooth curve of how the values of a number variable are spread, one curve per \
            group if you choose one.",
    widgets: &[number_var("Variable", true, ""), GROUP, TITLE],
    needs: &[],
    styles: BASE,
    check: no_check,
    code: |a, _| {
        let name = a.var_text("x");
        let (x, ttl) = (col(a, "x"), title(a, &format!("Density of {name}")));
        let Some(g) = a.var_string("g") else {
            return format!(
                "(function(d) {{\n  plot(d, main = {ttl}, xlab = {})\n  \
                 graphics::polygon(d, col = \"{BLUE}\", border = \"#2465b8\")\n}})\
                 (stats::density({x}, na.rm = TRUE))",
                r_string(name)
            );
        };
        format!(
            "(function(x, g) {{\n  g <- factor(g)\n  \
             cols <- grDevices::hcl.colors(nlevels(g), \"Dark 3\")\n  \
             dens <- lapply(levels(g), function(l) stats::density(x[!is.na(g) & g == l], na.rm = TRUE))\n  \
             plot(range(sapply(dens, function(d) range(d$x))), range(sapply(dens, function(d) range(d$y))), \
             type = \"n\", main = {ttl}, xlab = {}, ylab = \"Density\")\n  \
             for (i in seq_along(dens)) graphics::lines(dens[[i]], col = cols[i], lwd = 2)\n  \
             graphics::legend(\"topright\", legend = levels(g), col = cols, lwd = 2, bty = \"n\")\n}})\
             ({x}, {}[[{g}]])",
            r_string(name),
            a.data(),
        )
    },
};

// ---- Box plot ----

pub const BOX_PLOT: Spec = Spec {
    entry: "Box plot",
    menu: "Graphs > One variable > Box plot",
    about: "The median, quartiles and outliers of a number variable, with one box per group if \
            you choose one.",
    widgets: &[number_var("Variable", true, ""), GROUP, TITLE],
    needs: &[],
    styles: BASE,
    check: |a| {
        if !a.var_text("g").is_empty() && a.var_text("g") == a.var_text("x") {
            vec![Problem::new(
                "g",
                "Choose a different variable for the groups.",
            )]
        } else {
            Vec::new()
        }
    },
    code: |a, _| {
        let name = a.var_text("x");
        let ttl = title(a, &format!("Box plot of {name}"));
        match a.var("g") {
            Some(g) => format!(
                "graphics::boxplot({} ~ {g}, data = {}, main = {ttl}, xlab = {}, ylab = {}, col = \"{BLUE}\")",
                a.var("x").unwrap_or_default(),
                a.data(),
                r_string(a.var_text("g")),
                r_string(name),
            ),
            None => format!(
                "graphics::boxplot({}, main = {ttl}, ylab = {}, col = \"{BLUE}\")",
                col(a, "x"),
                r_string(name),
            ),
        }
    },
};

// ---- Bar chart ----

pub const BAR_CHART: Spec = Spec {
    entry: "Bar chart",
    menu: "Graphs > One variable > Bar chart",
    about: "A bar for each value of a categorical variable: how many rows have it, or what \
            percent of the rows. Missing values are left out.",
    widgets: &[
        Widget::Variable {
            id: "x",
            label: "Variable",
            kinds: CATEGORIES,
            required: true,
            hint: "A number variable is treated as categories, one bar per distinct value.",
        },
        Widget::Check {
            id: "percent",
            label: "Show percentages instead of counts",
            default: false,
            hint: "",
        },
        TITLE,
    ],
    needs: &[],
    styles: BASE,
    check: no_check,
    code: |a, _| {
        let name = a.var_text("x");
        let t = format!("table({})", col(a, "x"));
        let (height, ylab) = if a.flag("percent") {
            (format!("100 * prop.table({t})"), "Percent")
        } else {
            (t, "Count")
        };
        format!(
            "graphics::barplot({height}, main = {}, xlab = {}, ylab = \"{ylab}\", col = \"{BLUE}\", border = \"white\")",
            title(a, &format!("Bar chart of {name}")),
            r_string(name),
        )
    },
};

// ---- Normal Q-Q plot ----

pub const QQ_PLOT: Spec = Spec {
    entry: "Normal Q-Q plot",
    menu: "Graphs > One variable > Normal Q-Q plot",
    about: "Compare a number variable with a normal distribution: points near the line mean the \
            values look normal. The Shapiro-Wilk test is under Statistics.",
    widgets: &[number_var("Variable", true, ""), TITLE],
    needs: &[],
    styles: BASE,
    check: no_check,
    code: |a, _| {
        let name = a.var_text("x");
        let x = col(a, "x");
        format!(
            "stats::qqnorm({x}, main = {}, pch = 19, col = \"#2465b8\")\nstats::qqline({x}, lwd = 2)",
            title(a, &format!("Normal Q-Q plot of {name}")),
        )
    },
};

// ---- Scatter plot ----

pub const SCATTER: Spec = Spec {
    entry: "Scatter plot",
    menu: "Graphs > Two variables > Scatter plot",
    about: "One point per row, a number variable against another, with a straight-line fit or \
            a smooth curve if you want them. Rows missing either value are left out.",
    widgets: &[
        Widget::Variable {
            id: "x",
            label: "Horizontal axis",
            kinds: NUMBERS,
            required: true,
            hint: "",
        },
        Widget::Variable {
            id: "y",
            label: "Vertical axis",
            kinds: NUMBERS,
            required: true,
            hint: "",
        },
        GROUP,
        Widget::Check {
            id: "fit",
            label: "Add the straight-line (least squares) fit",
            default: false,
            hint: "",
        },
        Widget::Check {
            id: "smooth",
            label: "Add a smooth curve (lowess)",
            default: false,
            hint: "",
        },
        TITLE,
    ],
    needs: &[],
    styles: BASE,
    check: |a| {
        if a.var_text("x") == a.var_text("y") {
            vec![Problem::new(
                "y",
                "Choose a different variable for the vertical axis.",
            )]
        } else {
            Vec::new()
        }
    },
    code: |a, _| {
        let (xn, yn) = (a.var_text("x"), a.var_text("y"));
        let group = a
            .var_string("g")
            .map_or_else(|| "NULL".to_string(), |g| format!("{}[[{g}]]", a.data()));
        let mut body = format!(
            "  ok <- stats::complete.cases(x, y)\n  \
             cols <- if (is.null(g)) \"#2465b8\" else grDevices::hcl.colors(nlevels(factor(g)), \"Dark 3\")[factor(g)]\n  \
             plot(x, y, xlab = {}, ylab = {}, main = {}, pch = 19, col = cols)\n",
            r_string(xn),
            r_string(yn),
            title(a, &format!("{yn} against {xn}")),
        );
        if a.flag("fit") {
            body.push_str("  graphics::abline(stats::lm(y ~ x), lwd = 2)\n");
        }
        if a.flag("smooth") {
            body.push_str("  graphics::lines(stats::lowess(x[ok], y[ok]), lwd = 2, lty = 2)\n");
        }
        body.push_str(
            "  if (!is.null(g)) graphics::legend(\"topright\", legend = levels(factor(g)), \
             col = grDevices::hcl.colors(nlevels(factor(g)), \"Dark 3\"), pch = 19, bty = \"n\")\n",
        );
        format!(
            "(function(x, y, g) {{\n{body}}})({}, {}, {group})",
            col(a, "x"),
            col(a, "y"),
        )
    },
};

// ---- Line plot ----

pub const LINE_PLOT: Spec = Spec {
    entry: "Line plot",
    menu: "Graphs > Two variables > Line plot",
    about: "A number variable joined by a line, in the order of the horizontal variable (a \
            time, say). Rows missing either value are left out.",
    widgets: &[
        Widget::Variable {
            id: "x",
            label: "Horizontal axis",
            kinds: &[VarKind::Number, VarKind::Date],
            required: true,
            hint: "Usually time or an order.",
        },
        Widget::Variable {
            id: "y",
            label: "Vertical axis",
            kinds: NUMBERS,
            required: true,
            hint: "",
        },
        Widget::Check {
            id: "points",
            label: "Mark each row with a point",
            default: false,
            hint: "",
        },
        TITLE,
    ],
    needs: &[],
    styles: BASE,
    check: |a| {
        if a.var_text("x") == a.var_text("y") {
            vec![Problem::new(
                "y",
                "Choose a different variable for the vertical axis.",
            )]
        } else {
            Vec::new()
        }
    },
    code: |a, _| {
        let (xn, yn) = (a.var_text("x"), a.var_text("y"));
        format!(
            "(function(x, y) {{\n  ok <- !is.na(x) & !is.na(y)\n  o <- order(x[ok])\n  \
             plot(x[ok][o], y[ok][o], type = \"{}\", xlab = {}, ylab = {}, main = {}, \
             col = \"#2465b8\", lwd = 2)\n}})({}, {})",
            if a.flag("points") { "b" } else { "l" },
            r_string(xn),
            r_string(yn),
            title(a, &format!("{yn} by {xn}")),
            col(a, "x"),
            col(a, "y"),
        )
    },
};

// ---- Scatter plot matrix ----

pub const SCATTER_MATRIX: Spec = Spec {
    entry: "Scatter plot matrix",
    menu: "Graphs > Several variables > Scatter plot matrix",
    about: "Every pair of the chosen number variables plotted against each other, in one grid.",
    widgets: &[
        Widget::Variables {
            id: "vars",
            label: "Variables",
            kinds: NUMBERS,
            min: 2,
            hint: "",
        },
        TITLE,
    ],
    needs: &[],
    styles: BASE,
    check: no_check,
    code: |a, _| {
        let vars = a.vars("vars");
        let names: Vec<String> = vars.iter().map(|v| r_string(v)).collect();
        let main = a.values.text("title").trim();
        let main = if main.is_empty() {
            String::new()
        } else {
            format!(", main = {}", r_string(main))
        };
        format!(
            "graphics::pairs({}[c({})], pch = 19, col = \"#2465b8\"{main})",
            a.data(),
            names.join(", ")
        )
    },
};

/// The Graphs menu's dialogs, in menu order.
pub const SPECS: &[&Spec] = &[
    &HISTOGRAM,
    &DENSITY,
    &BOX_PLOT,
    &BAR_CHART,
    &QQ_PLOT,
    &SCATTER,
    &LINE_PLOT,
    &SCATTER_MATRIX,
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
            frames: Vec::new(),
            columns: vec![
                col("blood pressure", VarKind::Number),
                col("age", VarKind::Number),
                col("arm", VarKind::Factor),
                col("born", VarKind::Date),
                col("smoker", VarKind::Text),
            ],
        }
    }

    fn make(
        spec: &Spec,
        set: &[(&str, &str)],
        lists: &[(&str, &[&str])],
        flags: &[&str],
    ) -> Result<String, Vec<Problem>> {
        let c = ctx();
        let mut v: Values = spec.defaults(&c);
        for (id, text) in set {
            v.set_text(id, *text);
        }
        for (id, names) in lists {
            v.set_list(id, names.iter().map(|n| n.to_string()).collect());
        }
        for id in flags {
            v.set_flag(id, true);
        }
        spec.generate(&c, &v, Style::Base)
    }

    fn code(spec: &Spec, set: &[(&str, &str)], flags: &[&str]) -> String {
        match make(spec, set, &[], flags) {
            Ok(code) => code,
            Err(p) => format!("problems: {p:?}"),
        }
    }

    fn message(spec: &Spec, set: &[(&str, &str)]) -> String {
        match make(spec, set, &[], &[]) {
            Ok(code) => format!("no problem: {code}"),
            Err(p) => p
                .iter()
                .map(|p| p.message.clone())
                .collect::<Vec<_>>()
                .join(" "),
        }
    }

    #[test]
    fn histogram_titles_itself_and_takes_bars_or_a_rule() {
        let plain = code(&HISTOGRAM, &[("x", "blood pressure")], &[]);
        assert_eq!(
            plain,
            "graphics::hist(`my data`[[\"blood pressure\"]], breaks = \"Sturges\", \
             main = \"Histogram of blood pressure\", xlab = \"blood pressure\", \
             col = \"#8fb4e6\", border = \"white\")"
        );
        let full = code(
            &HISTOGRAM,
            &[("x", "age"), ("bins", "12"), ("title", "Ages \"so far\"")],
            &["density"],
        );
        assert!(full.contains("breaks = 12, freq = FALSE"), "{full}");
        assert!(full.contains("main = \"Ages \\\"so far\\\"\""), "{full}");
        assert!(
            full.ends_with(
                "graphics::lines(stats::density(`my data`[[\"age\"]], na.rm = TRUE), lwd = 2)"
            ),
            "{full}"
        );
        for bad in ["0", "1001", "abc", "2.5", "-3"] {
            assert!(
                message(&HISTOGRAM, &[("x", "age"), ("bins", bad)]).contains("whole number"),
                "{bad}"
            );
        }
        assert!(message(&HISTOGRAM, &[("x", "arm")]).contains("needs number"));
    }

    #[test]
    fn grouped_plots_use_the_group_and_refuse_the_same_variable() {
        let boxes = code(&BOX_PLOT, &[("x", "age"), ("g", "arm")], &[]);
        assert!(
            boxes.starts_with("graphics::boxplot(age ~ arm, data = `my data`"),
            "{boxes}"
        );
        assert!(code(&BOX_PLOT, &[("x", "age")], &[]).contains("boxplot(`my data`[[\"age\"]]"));
        assert!(message(&BOX_PLOT, &[("x", "age"), ("g", "age")]).contains("needs"));
        let dens = code(&DENSITY, &[("x", "age"), ("g", "smoker")], &[]);
        assert!(dens.contains("hcl.colors(nlevels(g)"), "{dens}");
        assert!(
            dens.ends_with("(`my data`[[\"age\"]], `my data`[[\"smoker\"]])"),
            "{dens}"
        );
        let one = code(&DENSITY, &[("x", "age")], &[]);
        assert!(one.contains("graphics::polygon(d"), "{one}");
    }

    #[test]
    fn bar_chart_counts_or_shows_percent_and_takes_any_categories() {
        let counts = code(&BAR_CHART, &[("x", "arm")], &[]);
        assert!(
            counts.starts_with("graphics::barplot(table(`my data`[[\"arm\"]])"),
            "{counts}"
        );
        assert!(counts.contains("ylab = \"Count\""), "{counts}");
        let pct = code(&BAR_CHART, &[("x", "smoker")], &["percent"]);
        assert!(pct.contains("100 * prop.table(table("), "{pct}");
        assert!(pct.contains("ylab = \"Percent\""), "{pct}");
        assert!(message(&BAR_CHART, &[("x", "born")]).contains("needs"));
        assert!(message(&BAR_CHART, &[("x", "age")]).starts_with("no problem"));
    }

    #[test]
    fn scatter_and_line_plots_check_their_axes_and_write_their_extras() {
        let plain = code(&SCATTER, &[("x", "age"), ("y", "blood pressure")], &[]);
        assert!(plain.starts_with("(function(x, y, g) {"), "{plain}");
        assert!(
            plain.ends_with("(`my data`[[\"age\"]], `my data`[[\"blood pressure\"]], NULL)"),
            "{plain}"
        );
        assert!(
            !plain.contains("abline") && !plain.contains("lowess"),
            "{plain}"
        );
        let rich = code(
            &SCATTER,
            &[("x", "age"), ("y", "blood pressure"), ("g", "arm")],
            &["fit", "smooth"],
        );
        assert!(
            rich.contains("graphics::abline(stats::lm(y ~ x), lwd = 2)"),
            "{rich}"
        );
        assert!(rich.contains("stats::lowess(x[ok], y[ok])"), "{rich}");
        assert!(rich.ends_with("`my data`[[\"arm\"]])"), "{rich}");
        assert!(message(&SCATTER, &[("x", "age"), ("y", "age")]).contains("different variable"));
        assert!(message(&SCATTER, &[("x", "age")]).contains("Choose vertical axis"));
        let line = code(&LINE_PLOT, &[("x", "born"), ("y", "age")], &["points"]);
        assert!(line.contains("type = \"b\""), "{line}");
        assert!(code(&LINE_PLOT, &[("x", "born"), ("y", "age")], &[]).contains("type = \"l\""));
        assert!(message(&LINE_PLOT, &[("x", "smoker"), ("y", "age")]).contains("needs"));
    }

    #[test]
    fn q_q_plot_and_scatter_matrix() {
        let qq = code(&QQ_PLOT, &[("x", "age")], &[]);
        assert!(qq.contains("stats::qqnorm("), "{qq}");
        assert!(
            qq.ends_with("stats::qqline(`my data`[[\"age\"]], lwd = 2)"),
            "{qq}"
        );
        let matrix = make(
            &SCATTER_MATRIX,
            &[("title", "All pairs")],
            &[("vars", &["age", "blood pressure"])],
            &[],
        )
        .unwrap_or_default();
        assert_eq!(
            matrix,
            "graphics::pairs(`my data`[c(\"age\", \"blood pressure\")], pch = 19, \
             col = \"#2465b8\", main = \"All pairs\")"
        );
        assert!(message(&SCATTER_MATRIX, &[]).contains("at least 2 variables"));
    }

    #[test]
    fn every_graph_dialog_is_a_catalog_entry_in_the_graphs_menu() {
        for spec in SPECS {
            assert!(
                crate::ENTRIES
                    .iter()
                    .any(|e| e.name == spec.entry && e.area == crate::Area::Graphs),
                "{} has no Graphs catalog entry",
                spec.entry
            );
            assert!(spec.menu.starts_with("Graphs > "));
            assert_eq!(spec.menu.split(" > ").count(), 3, "{}", spec.menu);
            let mut ids: Vec<&str> = spec.widgets.iter().map(|w| w.id()).collect();
            ids.sort_unstable();
            let n = ids.len();
            ids.dedup();
            assert_eq!(n, ids.len(), "{} repeats a widget id", spec.entry);
        }
    }
}
