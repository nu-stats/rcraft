//! Reports (Phase 7): a table of models side by side (P7.1), in base R so it
//! needs no package. Each coefficient with its stars, and below it what the
//! user chose in parentheses; then the rows that describe each fit. The value
//! is a data frame, so Results shows it as a grid that copies into Word or
//! Excel, and it can be kept under a name.

use rcraft_rcodegen::dialog::{Args, Opt, Problem, Spec, TextDefault, TextRule, Widget};
use rcraft_rcodegen::r_name;

use crate::data::BASE;

/// The names typed in the Other models box, separated by commas.
fn other_models<'a>(a: &'a Args) -> Vec<&'a str> {
    a.values
        .text("others")
        .split(',')
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .collect()
}

const PARENS: &[Opt] = &[
    Opt {
        value: "se",
        label: "Standard errors",
    },
    Opt {
        value: "statistic",
        label: "t or z statistics",
    },
    Opt {
        value: "p",
        label: "p-values",
    },
];

/// Most decimal places offered.
const MAX_DIGITS: u32 = 10;

fn table_check(a: &Args) -> Vec<Problem> {
    let mut out = Vec::new();
    let mut seen = vec![a.ctx.dataset.as_str()];
    for name in other_models(a) {
        if seen.contains(&name) {
            out.push(Problem::new(
                "others",
                format!("{name} is listed more than once, or is this model."),
            ));
            break;
        }
        seen.push(name);
    }
    if !a
        .number("digits")
        .parse::<u32>()
        .is_ok_and(|d| d <= MAX_DIGITS)
    {
        out.push(Problem::new(
            "digits",
            format!("Type a whole number of decimal places, 0 to {MAX_DIGITS}."),
        ));
    }
    out
}

fn table_code(a: &Args, _: rcraft_rcodegen::dialog::Style) -> String {
    let mut names = vec![a.ctx.dataset.as_str()];
    names.extend(other_models(a));
    let list: Vec<String> = names
        .iter()
        .map(|n| format!("{} = {}", r_name(n), r_name(n)))
        .collect();
    let (column, words) = match a.choice("paren") {
        "statistic" => (3, "t or z statistics"),
        "p" => (4, "p-values"),
        _ => (2, "standard errors"),
    };
    let stars = a.flag("stars");
    let mut note = format!("{} in parentheses", capitalised(words));
    if stars {
        note.push_str("; * p < 0.05, ** p < 0.01, *** p < 0.001");
    }
    let star = if stars {
        "ifelse(p < 0.001, \"***\", ifelse(p < 0.01, \"**\", ifelse(p < 0.05, \"*\", \"\")))"
    } else {
        "rep(\"\", length(p))"
    };
    let mut stats: Vec<&str> = Vec::new();
    if a.flag("n") {
        stats.push(
            "    Observations = vapply(models, function(m) as.character(stats::nobs(m)), \"\")",
        );
    }
    if a.flag("r2") {
        stats.push(
            "    `R-squared` = vapply(models, function(m) \
             if (inherits(m, \"glm\")) \"\" else fmt(summary(m)$r.squared), \"\")",
        );
        stats.push(
            "    `Adjusted R-squared` = vapply(models, function(m) \
             if (inherits(m, \"glm\")) \"\" else fmt(summary(m)$adj.r.squared), \"\")",
        );
    }
    if a.flag("ic") {
        stats.push("    AIC = vapply(models, function(m) fmt(stats::AIC(m)), \"\")");
        stats.push("    BIC = vapply(models, function(m) fmt(stats::BIC(m)), \"\")");
    }
    if a.flag("ll") {
        stats.push(
            "    `Log-likelihood` = vapply(models, function(m) fmt(as.numeric(stats::logLik(m))), \"\")",
        );
    }
    let foot = if stats.is_empty() {
        String::new()
    } else {
        format!(
            "  extra <- list(\n{}\n  )\n  \
             fit <- data.frame(term = names(extra), do.call(rbind, extra), check.names = FALSE, \
             row.names = NULL)\n  \
             names(fit) <- names(out)\n  \
             out <- rbind(out, fit)\n",
            stats.join(",\n")
        )
    };
    let call = format!(
        "(function(models, digits) {{\n  \
         ok <- vapply(models, inherits, logical(1), \"lm\")\n  \
         if (!all(ok)) stop(\"the table takes linear and generalised linear models; not: \", \
         paste(names(models)[!ok], collapse = \", \"))\n  \
         fmt <- function(x) formatC(x, format = \"f\", digits = digits)\n  \
         terms <- unique(unlist(lapply(models, function(m) names(stats::coef(m)))))\n  \
         cells <- vapply(models, function(m) {{\n    \
         ct <- stats::coef(summary(m))\n    \
         p <- ct[, 4]\n    \
         out <- rep(\"\", 2 * length(terms))\n    \
         i <- match(rownames(ct), terms)\n    \
         out[2 * i - 1] <- paste0(fmt(ct[, 1]), {star})\n    \
         out[2 * i] <- paste0(\"(\", fmt(ct[, {column}]), \")\")\n    \
         out\n  \
         }}, character(2 * length(terms)))\n  \
         out <- data.frame(term = as.vector(rbind(terms, \"\")), \
         matrix(cells, ncol = length(models), dimnames = list(NULL, names(models))), \
         check.names = FALSE, row.names = NULL)\n\
         {foot}  \
         cat(\"{note}\\n\")\n  \
         out\n}})(list({}), {})",
        list.join(", "),
        a.number("digits"),
    );
    if a.name_text("save").is_empty() {
        call
    } else {
        let save = a.name("save");
        format!("{save} <- {call}\n{save}")
    }
}

fn capitalised(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().chain(c).collect(),
        None => String::new(),
    }
}

pub const MODEL_TABLE: Spec = Spec {
    entry: "Model table",
    menu: "Objects > Model > Model table",
    about: "Put this model and others side by side in one table: each coefficient with its \
            stars and, below it, its standard error (or statistic or p-value), then the number \
            of rows, R-squared, AIC and BIC. The table copies into Word or Excel from Results. \
            For linear and generalised linear models.",
    widgets: &[
        Widget::Text {
            id: "others",
            label: "Other models",
            default: TextDefault::Empty,
            rule: TextRule::Any,
            hint: "Optional: names of models in the workspace, separated by commas. This model \
                   is the first column.",
        },
        Widget::Choice {
            id: "paren",
            label: "In parentheses",
            options: PARENS,
            default: "se",
            hint: "",
        },
        Widget::Check {
            id: "stars",
            label: "Stars for p < 0.05, 0.01 and 0.001",
            default: true,
            hint: "",
        },
        Widget::Text {
            id: "digits",
            label: "Decimal places",
            default: TextDefault::Fixed("3"),
            rule: TextRule::Number,
            hint: "",
        },
        Widget::Check {
            id: "n",
            label: "Number of observations",
            default: true,
            hint: "",
        },
        Widget::Check {
            id: "r2",
            label: "R-squared and adjusted R-squared (linear models)",
            default: true,
            hint: "",
        },
        Widget::Check {
            id: "ic",
            label: "AIC and BIC",
            default: true,
            hint: "",
        },
        Widget::Check {
            id: "ll",
            label: "Log-likelihood",
            default: false,
            hint: "",
        },
        Widget::Text {
            id: "save",
            label: "Also save the table as",
            default: TextDefault::Empty,
            rule: TextRule::Any,
            hint: "Optional: a name for the table, so later commands can use it.",
        },
    ],
    needs: &[],
    styles: BASE,
    check: table_check,
    code: table_code,
};

/// The Reports dialogs that work on a fitted model; they are offered in the
/// Objects pane's Model menu with the post-estimation ones.
pub const MODEL_REPORTS: &[&Spec] = &[&MODEL_TABLE];

#[cfg(test)]
mod tests {
    use super::*;
    use rcraft_rcodegen::dialog::{Context, Style, Values};

    fn make(
        model: &str,
        set: &[(&str, &str)],
        flags: &[(&str, bool)],
    ) -> Result<String, Vec<Problem>> {
        let c = Context {
            dataset: model.into(),
            frames: Vec::new(),
            columns: Vec::new(),
        };
        let mut v: Values = MODEL_TABLE.defaults(&c);
        for (id, text) in set {
            v.set_text(id, *text);
        }
        for (id, on) in flags {
            v.set_flag(id, *on);
        }
        MODEL_TABLE.generate(&c, &v, Style::Base)
    }

    #[test]
    fn one_model_or_several_with_the_choices_made() {
        let one = make("fit", &[], &[]).unwrap_or_default();
        assert!(one.ends_with("})(list(fit = fit), 3)"), "{one}");
        assert!(one.contains("fmt(ct[, 2])"), "{one}");
        assert!(
            one.contains("Standard errors in parentheses; * p < 0.05"),
            "{one}"
        );
        assert!(
            one.contains("Observations = ") && one.contains("AIC = "),
            "{one}"
        );
        assert!(!one.contains("Log-likelihood"), "{one}");

        let two = make(
            "my fit",
            &[
                ("others", "m2, m3"),
                ("paren", "p"),
                ("digits", "2"),
                ("save", "tab"),
            ],
            &[
                ("stars", false),
                ("n", false),
                ("r2", false),
                ("ic", false),
                ("ll", true),
            ],
        )
        .unwrap_or_default();
        assert!(two.starts_with("tab <- (function(models, digits)"), "{two}");
        assert!(
            two.contains("})(list(`my fit` = `my fit`, m2 = m2, m3 = m3), 2)\ntab"),
            "{two}"
        );
        assert!(
            two.contains("fmt(ct[, 4])") && two.contains("P-values in parentheses\\n"),
            "{two}"
        );
        assert!(!two.contains("***") && !two.contains("AIC ="), "{two}");
        assert!(two.contains("`Log-likelihood` = "), "{two}");

        // nothing below the coefficients: no rows to add
        let bare =
            make("fit", &[], &[("n", false), ("r2", false), ("ic", false)]).unwrap_or_default();
        assert!(!bare.contains("extra <- list("), "{bare}");
    }

    #[test]
    fn names_listed_twice_and_odd_decimal_places_are_refused() {
        let problems = |set: &[(&str, &str)]| match make("fit", set, &[]) {
            Ok(_) => Vec::new(),
            Err(p) => p.into_iter().map(|p| p.message).collect(),
        };
        assert!(problems(&[("others", "fit")])[0].contains("listed more than once"));
        assert!(problems(&[("others", "a, a")])[0].contains("listed more than once"));
        assert!(problems(&[("digits", "11")])[0].contains("decimal places"));
        assert!(problems(&[("digits", "1.5")])[0].contains("decimal places"));
        assert!(problems(&[("others", "a, b"), ("digits", "0")]).is_empty());
    }
}
