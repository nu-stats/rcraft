//! Statistics menu, meta-analysis (P6.11): one row per study, its effect size
//! computed from the data (a standardised mean difference from two groups'
//! sizes, means and standard deviations, a log odds ratio or log risk ratio
//! from a 2×2 table) or given with its standard error, pooled by
//! `metafor::rma` with random effects (REML or DerSimonian-Laird) or a fixed
//! effect (inverse variance, or Mantel-Haenszel with `metafor::rma.mh`), then
//! a forest plot, a funnel plot and Egger's test if asked. By default the
//! effect sizes and the fit are Stata's `meta esize` and `meta summarize`:
//! Hedges's g with the exact bias correction and Stata's variance, and half
//! a count added to every cell of a table with an empty cell, also for the
//! Mantel-Haenszel estimate. The fitted model stays in the workspace; the
//! data are not changed.

use rcraft_rcodegen::dialog::{Args, Opt, Problem, Spec, TextDefault, TextRule, VarKind, Widget};
use rcraft_rcodegen::r_name;

use crate::data::BASE;

const NUMBERS: &[VarKind] = &[VarKind::Number];

/// A column picker for one input of the effect size.
const fn input(id: &'static str, label: &'static str, hint: &'static str) -> Widget {
    Widget::Variable {
        id,
        label,
        kinds: NUMBERS,
        required: false,
        hint,
    }
}

/// The pickers each effect size reads, with the argument of `metafor::rma`
/// each one fills, in the order the dialog asks for them.
fn inputs(measure: &str) -> &'static [(&'static str, &'static str)] {
    match measure {
        "SMD" => &[
            ("n1", "n1i"),
            ("m1", "m1i"),
            ("sd1", "sd1i"),
            ("n2", "n2i"),
            ("m2", "m2i"),
            ("sd2", "sd2i"),
        ],
        "OR" | "RR" => &[("ev1", "ai"), ("ne1", "bi"), ("ev2", "ci"), ("ne2", "di")],
        _ => &[("est", "yi"), ("se", "sei")],
    }
}

fn is_table(measure: &str) -> bool {
    matches!(measure, "OR" | "RR")
}

/// The arguments naming the effect size and its columns, the data and the
/// study labels: everything `rma` and `rma.mh` share.
fn effect_args(a: &Args) -> String {
    let measure = a.choice("measure");
    let mut parts: Vec<String> = Vec::new();
    if measure != "given" {
        parts.push(format!("measure = \"{measure}\""));
    }
    for (id, arg) in inputs(measure) {
        parts.push(format!("{arg} = {}", r_name(a.var_text(id))));
    }
    if measure == "SMD" {
        // Stata's variance of Hedges's g: the large-sample variance of d,
        // times the squared correction
        parts.push("vtype = \"LS2\"".into());
    }
    parts.push(format!("data = {}", a.data()));
    if let Some(label) = a.var("label") {
        parts.push(format!("slab = {label}"));
    }
    parts.join(", ")
}

fn meta_check(a: &Args) -> Vec<Problem> {
    let mut out = Vec::new();
    let measure = a.choice("measure");
    for (id, _) in inputs(measure) {
        if a.var_text(id).is_empty() {
            let what = META
                .widgets
                .iter()
                .find_map(|w| match w {
                    Widget::Variable { id: wid, label, .. } if wid == id => Some(*label),
                    _ => None,
                })
                .unwrap_or("this input");
            out.push(Problem::new(
                id,
                format!("Choose the column for {what}; this effect size needs it."),
            ));
        }
    }
    if a.choice("method") == "MH" && !is_table(measure) {
        out.push(Problem::new(
            "method",
            "Mantel-Haenszel pools 2×2 tables: choose the log odds ratio or log risk ratio, or \
             another method.",
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

pub const META: Spec = Spec {
    entry: "Meta-analysis",
    menu: "Statistics > Meta-analysis",
    about: "One row per study: an effect size computed from the data, or given with its \
            standard error, pooled by metafor::rma with random effects (REML or \
            DerSimonian-Laird) or a fixed effect (inverse variance or Mantel-Haenszel), as \
            Stata's meta esize and meta summarize. The summary shows the pooled estimate, its \
            standard error and confidence interval, tau², I², H² and the test of homogeneity; a \
            forest plot, a funnel plot and Egger's test for small-study effects (meta bias, \
            egger) follow if asked. Studies missing an input are left out.",
    widgets: &[
        Widget::Choice {
            id: "measure",
            label: "Effect size",
            options: &[
                Opt {
                    value: "SMD",
                    label: "Standardised mean difference (Hedges's g)",
                },
                Opt {
                    value: "OR",
                    label: "Log odds ratio, from a 2×2 table",
                },
                Opt {
                    value: "RR",
                    label: "Log risk ratio, from a 2×2 table",
                },
                Opt {
                    value: "given",
                    label: "Given: an estimate and its standard error",
                },
            ],
            default: "SMD",
            hint: "Group 1 is compared with group 2 (treated with control). Hedges's g uses the \
                   exact bias correction, as Stata's esize(hedgesg, exact). A table with an \
                   empty cell gets half a count added to each of its cells, as Stata does.",
        },
        input("n1", "Group 1 size (mean difference)", ""),
        input("m1", "Group 1 mean (mean difference)", ""),
        input("sd1", "Group 1 standard deviation (mean difference)", ""),
        input("n2", "Group 2 size (mean difference)", ""),
        input("m2", "Group 2 mean (mean difference)", ""),
        input("sd2", "Group 2 standard deviation (mean difference)", ""),
        input("ev1", "Group 1 with the event (2×2 table)", ""),
        input("ne1", "Group 1 without it (2×2 table)", ""),
        input("ev2", "Group 2 with the event (2×2 table)", ""),
        input("ne2", "Group 2 without it (2×2 table)", ""),
        input("est", "Estimate (given)", ""),
        input("se", "Its standard error (given)", ""),
        Widget::Variable {
            id: "label",
            label: "Study labels",
            kinds: &[],
            required: false,
            hint: "Optional: names the studies in the forest plot.",
        },
        Widget::Choice {
            id: "method",
            label: "Model",
            options: &[
                Opt {
                    value: "REML",
                    label: "Random effects, REML",
                },
                Opt {
                    value: "DL",
                    label: "Random effects, DerSimonian-Laird",
                },
                Opt {
                    value: "FE",
                    label: "Fixed effect, inverse variance",
                },
                Opt {
                    value: "MH",
                    label: "Fixed effect, Mantel-Haenszel (2×2 tables)",
                },
            ],
            default: "REML",
            hint: "REML is Stata's default, random(reml); DerSimonian-Laird is random(dlaird). \
                   For 2×2 tables Stata's fixed means Mantel-Haenszel; fixed(ivariance) is \
                   inverse variance.",
        },
        Widget::Check {
            id: "eform",
            label: "Odds or risk ratios too",
            default: true,
            hint: "For a log odds or risk ratio: the pooled ratio and its interval on the ratio \
                   scale, as Stata's eform option, and the plots' axes in ratios.",
        },
        Widget::Check {
            id: "forest",
            label: "Forest plot",
            default: true,
            hint: "",
        },
        Widget::Check {
            id: "funnel",
            label: "Funnel plot",
            default: false,
            hint: "",
        },
        Widget::Choice {
            id: "egger",
            label: "Egger's test for small-study effects",
            options: &[
                Opt {
                    value: "none",
                    label: "None",
                },
                Opt {
                    value: "model",
                    label: "Meta-regression on the standard error (meta bias, egger)",
                },
                Opt {
                    value: "lm",
                    label: "Traditional: weighted regression (meta bias, egger traditional)",
                },
            ],
            default: "none",
            hint: "The first uses the model chosen above (after Mantel-Haenszel, the inverse \
                   variance one, as Stata does); the traditional test is Egger's original \
                   regression with a multiplicative dispersion.",
        },
        Widget::Text {
            id: "save",
            label: "Save the model as",
            default: TextDefault::Fixed("ma"),
            rule: TextRule::Name,
            hint: "The fitted model stays in R's workspace under this name; a name already in \
                   use is replaced.",
        },
    ],
    needs: &["metafor"],
    styles: BASE,
    check: meta_check,
    code: |a, _| {
        let fit = a.name("save");
        let measure = a.choice("measure");
        let method = a.choice("method");
        let args = effect_args(a);
        let ratio = is_table(measure) && a.flag("eform");
        let mut out = if method == "MH" {
            format!(
                "{fit} <- metafor::rma.mh({args}, add = c(1/2, 1/2), to = c(\"only0\", \"only0\"))\n"
            )
        } else {
            format!("{fit} <- metafor::rma({args}, method = \"{method}\")\n")
        };
        out.push_str(&format!("summary({fit})\n"));
        if ratio {
            let name = if measure == "OR" {
                "Odds ratio"
            } else {
                "Risk ratio"
            };
            out.push_str(&format!(
                "cat(\"\\n{name}, pooled, with its 95% confidence interval\\n\")\n\
                 print(predict({fit}, transf = exp))\n"
            ));
        }
        let axis = if ratio { ", atransf = exp" } else { "" };
        if a.flag("forest") {
            out.push_str(&format!("metafor::forest({fit}, header = TRUE{axis})\n"));
        }
        if a.flag("funnel") {
            out.push_str(&format!("metafor::funnel({fit}{axis})\n"));
        }
        let egger = a.choice("egger");
        if egger != "none" {
            let model = if method == "MH" {
                format!("metafor::rma({args}, method = \"FE\")")
            } else {
                fit.clone()
            };
            if egger == "lm" {
                out.push_str(&format!("metafor::regtest({model}, model = \"lm\")\n"));
            } else {
                out.push_str(&format!("metafor::regtest({model})\n"));
            }
        }
        out.trim_end().to_string()
    },
};

pub const SPECS: &[&Spec] = &[&META];

#[cfg(test)]
mod tests {
    use super::*;
    use rcraft_rcodegen::dialog::{ColumnInfo, Context, Style, Values};

    fn ctx() -> Context {
        let col = |name: &str| ColumnInfo {
            name: name.into(),
            kind: VarKind::Number,
        };
        let mut columns: Vec<ColumnInfo> = [
            "n1", "m1", "sd1", "n2", "m2", "sd2", "a", "b", "c", "d", "est", "se",
        ]
        .into_iter()
        .map(col)
        .collect();
        columns.push(ColumnInfo {
            name: "study".into(),
            kind: VarKind::Text,
        });
        Context {
            dataset: "dat".into(),
            frames: Vec::new(),
            columns,
        }
    }

    fn values(set: &[(&str, &str)]) -> Values {
        let mut v = META.defaults(&ctx());
        for (id, t) in [
            ("n1", "n1"),
            ("m1", "m1"),
            ("sd1", "sd1"),
            ("n2", "n2"),
            ("m2", "m2"),
            ("sd2", "sd2"),
            ("ev1", "a"),
            ("ne1", "b"),
            ("ev2", "c"),
            ("ne2", "d"),
            ("est", "est"),
            ("se", "se"),
        ] {
            v.set_text(id, t);
        }
        for (id, t) in set {
            v.set_text(id, *t);
        }
        v
    }

    fn code(set: &[(&str, &str)]) -> Result<String, Vec<Problem>> {
        META.generate(&ctx(), &values(set), Style::Base)
    }

    #[test]
    fn hedges_g_with_reml_and_a_forest_plot_by_default() {
        let c = code(&[("label", "study")]).unwrap_or_default();
        assert_eq!(
            c,
            "ma <- metafor::rma(measure = \"SMD\", n1i = n1, m1i = m1, sd1i = sd1, n2i = n2, \
             m2i = m2, sd2i = sd2, vtype = \"LS2\", data = dat, slab = study, method = \"REML\")\n\
             summary(ma)\n\
             metafor::forest(ma, header = TRUE)"
        );
    }

    #[test]
    fn tables_show_ratios_and_mantel_haenszel_adds_half_counts() {
        let mut v = values(&[("measure", "OR"), ("method", "MH"), ("egger", "model")]);
        v.set_flag("funnel", true);
        let c = META.generate(&ctx(), &v, Style::Base).unwrap_or_default();
        assert!(
            c.starts_with(
                "ma <- metafor::rma.mh(measure = \"OR\", ai = a, bi = b, ci = c, di = d, data = dat, \
                 add = c(1/2, 1/2), to = c(\"only0\", \"only0\"))\n"
            ),
            "{c}"
        );
        assert!(c.contains("print(predict(ma, transf = exp))"), "{c}");
        assert!(
            c.contains("metafor::forest(ma, header = TRUE, atransf = exp)"),
            "{c}"
        );
        assert!(c.contains("metafor::funnel(ma, atransf = exp)"), "{c}");
        // Egger's test after Mantel-Haenszel uses the inverse-variance model
        assert!(
            c.ends_with(
                "metafor::regtest(metafor::rma(measure = \"OR\", ai = a, bi = b, ci = c, di = d, \
                 data = dat, method = \"FE\"))"
            ),
            "{c}"
        );
    }

    #[test]
    fn given_estimates_dersimonian_laird_and_the_traditional_test() {
        let mut v = values(&[("measure", "given"), ("method", "DL"), ("egger", "lm")]);
        v.set_flag("forest", false);
        v.set_flag("funnel", true);
        let c = META.generate(&ctx(), &v, Style::Base).unwrap_or_default();
        assert_eq!(
            c,
            "ma <- metafor::rma(yi = est, sei = se, data = dat, method = \"DL\")\n\
             summary(ma)\n\
             metafor::funnel(ma)\n\
             metafor::regtest(ma, model = \"lm\")"
        );
    }

    #[test]
    fn missing_inputs_and_bad_choices_are_refused() {
        let problem = |set: &[(&str, &str)]| -> String {
            code(set)
                .err()
                .and_then(|p| p.first().map(|p| p.message.clone()))
                .unwrap_or_default()
        };
        assert!(problem(&[("sd2", "")]).contains("Group 2 standard deviation"));
        assert!(problem(&[("measure", "RR"), ("ne2", "")]).contains("Group 2 without it"));
        assert!(problem(&[("measure", "given"), ("method", "MH")]).contains("Mantel-Haenszel"));
        assert!(problem(&[("save", "dat")]).contains("data frame"));
        // the pickers of another effect size are not needed
        assert!(code(&[("measure", "given"), ("n1", "")]).is_ok());
    }
}
