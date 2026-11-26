//! Statistics menu, meta-analysis (P6.11): one row per study, its effect size
//! computed from the data (a standardised mean difference, Hedges's g,
//! Cohen's d or Glass's delta, from two groups' sizes, means and standard
//! deviations; a log odds ratio, log Peto odds ratio, log risk ratio or risk
//! difference from a 2×2 table) or given with its standard error, pooled by
//! `metafor::rma` with random effects (REML, ML, empirical Bayes,
//! Paule-Mandel, Hedges, Sidik-Jonkman or DerSimonian-Laird; Knapp-Hartung
//! errors and a prediction interval if asked) or a fixed effect (inverse
//! variance, or Mantel-Haenszel with `metafor::rma.mh`), then a forest plot,
//! a funnel plot and Egger's test if asked.
//!
//! The effect sizes and the fit are Stata's `meta esize` and `meta
//! summarize`, found by running both: Hedges's g with Stata's approximate
//! bias correction by default (or the exact one), each with Stata's
//! variances; half a count added to every cell of a table with an empty cell
//! for odds and risk ratios (also before Mantel-Haenszel pooling), but not
//! for risk differences and Peto's odds ratio; the Mantel-Haenszel risk
//! difference's standard error as Greenland and Robins give it; Stata's
//! prediction interval (t on k - 2 degrees of freedom). The fitted model
//! stays in the workspace; the data are not changed.

use rcraft_rcodegen::dialog::{Args, Opt, Problem, Spec, TextDefault, TextRule, VarKind, Widget};
use rcraft_rcodegen::{r_name, r_string};

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

const GROUPS: &[(&str, &str)] = &[
    ("n1", "n1i"),
    ("m1", "m1i"),
    ("sd1", "sd1i"),
    ("n2", "n2i"),
    ("m2", "m2i"),
    ("sd2", "sd2i"),
];

const TABLE: &[(&str, &str)] = &[("ev1", "ai"), ("ne1", "bi"), ("ev2", "ci"), ("ne2", "di")];

/// The pickers each effect size reads, with the argument of
/// `metafor::escalc` each one fills, in the order the dialog asks for them.
fn inputs(measure: &str) -> &'static [(&'static str, &'static str)] {
    match measure {
        "SMDA" | "SMD" | "COHEN" => GROUPS,
        // standardised by group 2's standard deviation only
        "GLASS" => &[
            ("n1", "n1i"),
            ("m1", "m1i"),
            ("n2", "n2i"),
            ("m2", "m2i"),
            ("sd2", "sd2i"),
        ],
        "OR" | "RR" | "RD" | "PETO" => TABLE,
        _ => &[("est", "yi"), ("se", "sei")],
    }
}

/// A ratio, pooled on the log scale.
fn is_ratio(measure: &str) -> bool {
    matches!(measure, "OR" | "RR" | "PETO")
}

/// Computed by `escalc` and then put on Stata's footing in the dialog's code.
fn is_adjusted(measure: &str) -> bool {
    matches!(measure, "SMDA" | "GLASS")
}

fn is_random(method: &str) -> bool {
    !matches!(method, "FE" | "MH")
}

/// The arguments naming the effect size and its columns and the data: what
/// `escalc`, `rma` and `rma.mh` share; with `zeros`, also `to = "none"` for
/// the measures Stata adds no half counts to (`rma.mh` takes two of them).
fn effect_args(a: &Args, zeros: bool) -> String {
    let measure = a.choice("measure");
    let mut parts: Vec<String> = Vec::new();
    match measure {
        "given" => {}
        "SMDA" | "COHEN" => parts.push("measure = \"SMD\"".into()),
        "GLASS" => parts.push("measure = \"SMD1\"".into()),
        m => parts.push(format!("measure = \"{m}\"")),
    }
    for (id, arg) in inputs(measure) {
        parts.push(format!("{arg} = {}", r_name(a.var_text(id))));
    }
    parts.push(format!("data = {}", a.data()));
    match measure {
        // Stata's variance of the exact Hedges's g: the large-sample variance
        // of d, times the squared correction
        "SMD" => parts.push("vtype = \"LS2\"".into()),
        "SMDA" | "COHEN" | "GLASS" => parts.push("correct = FALSE".into()),
        // Stata adds no half counts to empty cells for these two
        "RD" | "PETO" if zeros => parts.push("to = \"none\"".into()),
        _ => {}
    }
    parts.join(", ")
}

/// A column of the data as R code (`d[["n1"]]`).
fn column(a: &Args, id: &str) -> String {
    format!("{}[[{}]]", a.data(), r_string(a.var_text(id)))
}

/// `rma`'s own options after the data: labels, method, errors.
fn fit_options(a: &Args, method: &str) -> String {
    let mut out = String::new();
    if let Some(label) = a.var("label") {
        out.push_str(&format!(", slab = {label}"));
    }
    out.push_str(&format!(", method = \"{method}\""));
    if is_random(method) && a.flag("knha") {
        out.push_str(", test = \"knha\"");
    }
    if method == "PM" {
        // Paule-Mandel's root to Stata's precision (metafor's own tolerance
        // stops about 3e-5 away)
        out.push_str(", control = list(tol = 1e-10)");
    }
    out
}

/// The fit: `rma` on the effect sizes (computed first and adjusted for
/// Stata's approximate Hedges's g and Glass's delta), or `rma.mh`.
fn fit_code(a: &Args, fit: &str) -> String {
    let measure = a.choice("measure");
    let method = a.choice("method");
    let args = effect_args(a, method != "MH");
    if method == "MH" {
        let zeros = if measure == "RD" {
            "to = c(\"none\", \"none\")"
        } else {
            "add = c(1/2, 1/2), to = c(\"only0\", \"only0\")"
        };
        let label = a
            .var("label")
            .map(|l| format!(", slab = {l}"))
            .unwrap_or_default();
        return format!("{fit} <- metafor::rma.mh({args}, {zeros}{label})\n");
    }
    let options = fit_options(a, method);
    if !is_adjusted(measure) {
        return format!("{fit} <- metafor::rma({args}{options})\n");
    }
    let adjust = if measure == "SMDA" {
        // Hedges's g as Stata's default: d times 1 - 3/(4N - 9), its
        // variance times the square
        format!(
            "  J <- 1 - 3 / (4 * ({} + {}) - 9)\n  \
             es$yi <- J * es$yi\n  \
             es$vi <- J^2 * es$vi\n",
            column(a, "n1"),
            column(a, "n2")
        )
    } else {
        // Glass's delta with Stata's variance
        format!(
            "  n1 <- {}\n  n2 <- {}\n  \
             es$vi <- 1 / n1 + 1 / n2 + es$yi^2 / (2 * (n2 - 1))\n",
            column(a, "n1"),
            column(a, "n2")
        )
    };
    format!(
        "{fit} <- local({{\n  \
         es <- metafor::escalc({args})\n\
         {adjust}  \
         metafor::rma(yi, vi, data = es{options})\n}})\n"
    )
}

/// The Mantel-Haenszel risk difference with the Greenland-Robins standard
/// error, as Stata gives it (metafor's summary shows Sato's).
fn greenland_robins(a: &Args) -> String {
    let cells: Vec<String> = TABLE
        .iter()
        .zip(["a", "b", "c", "d"])
        .map(|((id, _), cell)| format!("{cell} = {}", column(a, id)))
        .collect();
    format!(
        "local({{\n  \
         t <- stats::na.omit(data.frame({}))\n  \
         n1 <- t$a + t$b; n2 <- t$c + t$d; N <- n1 + n2; w <- n1 * n2 / N\n  \
         rd <- sum((t$a * n2 - t$c * n1) / N) / sum(w)\n  \
         se <- sqrt(sum((t$a * t$b * n2^3 + t$c * t$d * n1^3) / (n1 * n2 * N^2)) / sum(w)^2)\n  \
         z <- stats::qnorm(0.975)\n  \
         cat(\"\\nRisk difference with the Greenland-Robins standard error, as Stata's\\n\")\n  \
         print(c(estimate = rd, se = se, zval = rd / se, pval = 2 * stats::pnorm(-abs(rd / se)), \
         ci.lb = rd - z * se, ci.ub = rd + z * se), digits = 7)\n}})\n",
        cells.join(", ")
    )
}

fn meta_check(a: &Args) -> Vec<Problem> {
    let mut out = Vec::new();
    let measure = a.choice("measure");
    let method = a.choice("method");
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
    if method == "MH" && !matches!(measure, "OR" | "RR" | "RD") {
        out.push(Problem::new(
            "method",
            "Mantel-Haenszel pools 2×2 tables: choose the log odds ratio, log risk ratio or risk \
             difference, or another method.",
        ));
    }
    if !is_random(method) {
        for (id, what) in [
            ("knha", "Knapp-Hartung errors"),
            ("pi", "A prediction interval"),
        ] {
            if a.flag(id) {
                out.push(Problem::new(
                    id,
                    format!("{what} need a random-effects model, as in Stata."),
                ));
            }
        }
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
            standard error, pooled by metafor::rma with random effects or a fixed effect \
            (inverse variance or Mantel-Haenszel), as Stata's meta esize and meta summarize. \
            The summary shows the pooled estimate, its standard error and confidence interval, \
            tau², I², H² and the test of homogeneity; a prediction interval, a forest plot, a \
            funnel plot and Egger's test for small-study effects (meta bias, egger) follow if \
            asked. Studies missing an input are left out.",
    widgets: &[
        Widget::Choice {
            id: "measure",
            label: "Effect size",
            options: &[
                Opt {
                    value: "SMDA",
                    label: "Hedges's g (Stata's default)",
                },
                Opt {
                    value: "SMD",
                    label: "Hedges's g, exact correction",
                },
                Opt {
                    value: "COHEN",
                    label: "Cohen's d",
                },
                Opt {
                    value: "GLASS",
                    label: "Glass's delta (group 2's standard deviation)",
                },
                Opt {
                    value: "OR",
                    label: "Log odds ratio, from a 2×2 table",
                },
                Opt {
                    value: "PETO",
                    label: "Log Peto odds ratio, from a 2×2 table",
                },
                Opt {
                    value: "RR",
                    label: "Log risk ratio, from a 2×2 table",
                },
                Opt {
                    value: "RD",
                    label: "Risk difference, from a 2×2 table",
                },
                Opt {
                    value: "given",
                    label: "Given: an estimate and its standard error",
                },
            ],
            default: "SMDA",
            hint: "Group 1 is compared with group 2 (treated with control). Stata's default \
                   Hedges's g uses the approximate bias correction 1 - 3/(4N - 9); the exact one \
                   is esize(hedgesg, exact). Glass's delta is esize(glassdelta2). A table with \
                   an empty cell gets half a count added to each of its cells for odds and risk \
                   ratios, but not for the risk difference or Peto's odds ratio, as Stata does.",
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
                    value: "ML",
                    label: "Random effects, maximum likelihood",
                },
                Opt {
                    value: "EB",
                    label: "Random effects, empirical Bayes",
                },
                Opt {
                    value: "PM",
                    label: "Random effects, Paule-Mandel",
                },
                Opt {
                    value: "HE",
                    label: "Random effects, Hedges",
                },
                Opt {
                    value: "SJ",
                    label: "Random effects, Sidik-Jonkman",
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
            hint: "REML is Stata's default, random(reml); the others are random(mle), \
                   random(ebayes), random(pmandel), random(hedges), random(sjonkman) and \
                   random(dlaird). For odds ratios, risk ratios and risk differences Stata's \
                   fixed means Mantel-Haenszel; fixed(ivariance) is inverse variance (and \
                   Stata's fixed for Peto's odds ratio). A Mantel-Haenszel risk difference also \
                   shows Stata's Greenland-Robins standard error.",
        },
        Widget::Check {
            id: "knha",
            label: "Knapp-Hartung standard errors",
            default: false,
            hint: "Random effects only: the pooled estimate's error and interval adjusted, with t \
                   on k - 1 degrees of freedom, as Stata's se(khartung).",
        },
        Widget::Check {
            id: "pi",
            label: "Prediction interval",
            default: false,
            hint: "Random effects only: where a new study's true effect may lie, with t on k - 2 \
                   degrees of freedom, as Stata's predinterval.",
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
        let mut out = fit_code(a, &fit);
        out.push_str(&format!("summary({fit})\n"));
        if method == "MH" && measure == "RD" {
            out.push_str(&greenland_robins(a));
        }
        let ratio = is_ratio(measure) && a.flag("eform");
        let pi = is_random(method) && a.flag("pi");
        if ratio || pi {
            let what = match measure {
                "OR" | "PETO" if ratio => "Odds ratio",
                "RR" if ratio => "Risk ratio",
                _ => "Estimate",
            };
            let with_pi = if pi {
                " and Stata's prediction interval"
            } else {
                ""
            };
            let mut args = Vec::new();
            if ratio {
                args.push(", transf = exp");
            }
            if pi {
                args.push(", pi.type = \"Riley\"");
            }
            out.push_str(&format!(
                "cat(\"\\n{what}, pooled, with its 95% confidence interval{with_pi}\\n\")\n\
                 print(predict({fit}{}))\n",
                args.join("")
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
                format!("metafor::rma({}, method = \"FE\")", effect_args(a, true))
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
    fn stata_default_hedges_g_with_reml_and_a_forest_plot_by_default() {
        let c = code(&[("label", "study")]).unwrap_or_default();
        assert_eq!(
            c,
            "ma <- local({\n  \
             es <- metafor::escalc(measure = \"SMD\", n1i = n1, m1i = m1, sd1i = sd1, n2i = n2, \
             m2i = m2, sd2i = sd2, data = dat, correct = FALSE)\n  \
             J <- 1 - 3 / (4 * (dat[[\"n1\"]] + dat[[\"n2\"]]) - 9)\n  \
             es$yi <- J * es$yi\n  \
             es$vi <- J^2 * es$vi\n  \
             metafor::rma(yi, vi, data = es, slab = study, method = \"REML\")\n})\n\
             summary(ma)\n\
             metafor::forest(ma, header = TRUE)"
        );
    }

    #[test]
    fn exact_g_glass_and_cohen_write_their_own_escalc_options() {
        let c = code(&[("measure", "SMD")]).unwrap_or_default();
        assert!(
            c.starts_with(
                "ma <- metafor::rma(measure = \"SMD\", n1i = n1, m1i = m1, sd1i = sd1, n2i = n2, \
                 m2i = m2, sd2i = sd2, data = dat, vtype = \"LS2\", method = \"REML\")\n"
            ),
            "{c}"
        );
        let c = code(&[("measure", "COHEN")]).unwrap_or_default();
        assert!(
            c.contains("sd2i = sd2, data = dat, correct = FALSE, method"),
            "{c}"
        );
        let c = code(&[("measure", "GLASS"), ("sd1", "")]).unwrap_or_default();
        assert!(
            c.contains(
                "escalc(measure = \"SMD1\", n1i = n1, m1i = m1, n2i = n2, m2i = m2, sd2i = sd2"
            ),
            "{c}"
        );
        assert!(
            c.contains("es$vi <- 1 / n1 + 1 / n2 + es$yi^2 / (2 * (n2 - 1))"),
            "{c}"
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
    fn risk_differences_and_peto_add_no_half_counts() {
        let c = code(&[("measure", "RD"), ("method", "MH")]).unwrap_or_default();
        assert!(
            c.starts_with(
                "ma <- metafor::rma.mh(measure = \"RD\", ai = a, bi = b, ci = c, di = d, data = dat, \
                 to = c(\"none\", \"none\"))\n"
            ),
            "{c}"
        );
        assert!(c.contains("Greenland-Robins"), "{c}");
        assert!(!c.contains("predict("), "{c}");
        let c = code(&[("measure", "PETO"), ("method", "FE")]).unwrap_or_default();
        assert!(
            c.starts_with(
                "ma <- metafor::rma(measure = \"PETO\", ai = a, bi = b, ci = c, di = d, data = dat, \
                 to = \"none\", method = \"FE\")\n"
            ),
            "{c}"
        );
        assert!(c.contains("Odds ratio, pooled"), "{c}");
    }

    #[test]
    fn knapp_hartung_paule_mandel_and_the_prediction_interval() {
        let mut v = values(&[("measure", "given"), ("method", "PM")]);
        v.set_flag("knha", true);
        v.set_flag("pi", true);
        v.set_flag("forest", false);
        let c = META.generate(&ctx(), &v, Style::Base).unwrap_or_default();
        assert_eq!(
            c,
            "ma <- metafor::rma(yi = est, sei = se, data = dat, method = \"PM\", test = \"knha\", \
             control = list(tol = 1e-10))\n\
             summary(ma)\n\
             cat(\"\\nEstimate, pooled, with its 95% confidence interval and Stata's prediction \
             interval\\n\")\n\
             print(predict(ma, pi.type = \"Riley\"))"
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
        assert!(problem(&[("measure", "PETO"), ("method", "MH")]).contains("Mantel-Haenszel"));
        assert!(problem(&[("save", "dat")]).contains("data frame"));
        let mut v = values(&[("method", "FE")]);
        v.set_flag("pi", true);
        let p = META
            .generate(&ctx(), &v, Style::Base)
            .err()
            .unwrap_or_default();
        assert!(
            p.iter().any(|p| p.message.contains("random-effects")),
            "{p:?}"
        );
        // the pickers of another effect size are not needed
        assert!(code(&[("measure", "given"), ("n1", "")]).is_ok());
        assert!(code(&[("measure", "GLASS"), ("sd1", "")]).is_ok());
    }
}
