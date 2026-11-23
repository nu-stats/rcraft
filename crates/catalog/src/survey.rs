//! Statistics menu, survey data (P6.4), through Thomas Lumley's `survey`
//! package: means and totals, proportions and two-way tables with the
//! Rao-Scott test, ratios, quantiles and regression, each with design-based
//! standard errors.
//! Every dialog declares the design itself (sampling weights, primary
//! sampling units, strata, a finite-population correction) with
//! `survey::svydesign` and keeps it under a name, so its R runs on its own.
//! PSUs are taken as nested in their strata (`nest = TRUE`), as Stata's
//! `svyset` takes them. Confidence intervals use the design's degrees of
//! freedom (PSUs less strata), as Stata's do. None changes the data.
//!
//! The design widgets also take a second stage (secondary sampling units,
//! each stage with its own correction), replicate weights (`svrepdesign`: BRR,
//! JK1 or bootstrap) in place of strata and clusters, and a subpopulation
//! (`subset` on the design, never a filter on the data).

use rcraft_rcodegen::dialog::{
    expression_problem, Args, Opt, Problem, Spec, TextDefault, TextRule,
};
use rcraft_rcodegen::dialog::{VarKind, Widget};
use rcraft_rcodegen::{r_name, r_string};

use crate::data::BASE;

const NUMBERS: &[VarKind] = &[VarKind::Number];
const CATEGORIES: &[VarKind] = &[
    VarKind::Factor,
    VarKind::Text,
    VarKind::Logical,
    VarKind::Number,
];
const PREDICTORS: &[VarKind] = &[
    VarKind::Number,
    VarKind::Factor,
    VarKind::Text,
    VarKind::Logical,
];
const GROUPS: &[VarKind] = &[VarKind::Number, VarKind::Factor, VarKind::Text];

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

const CONFIDENCE: Widget = Widget::Choice {
    id: "conf",
    label: "Confidence level",
    options: LEVELS,
    default: "0.95",
    hint: "Intervals use t with the design's degrees of freedom (PSUs less strata).",
};

const WEIGHTS: Widget = Widget::Variable {
    id: "weights",
    label: "Sampling weights",
    kinds: NUMBERS,
    required: true,
    hint: "How many population members each row stands for (Stata's pweight).",
};

const PSU: Widget = Widget::Variable {
    id: "psu",
    label: "Primary sampling units",
    kinds: GROUPS,
    required: false,
    hint: "Optional: the clusters sampled first (schools, villages). Without them each row is \
           its own unit.",
};

const STRATA: Widget = Widget::Variable {
    id: "strata",
    label: "Strata",
    kinds: GROUPS,
    required: false,
    hint: "Optional: the strata sampled separately. Units are read as numbered within their \
           stratum.",
};

const FPC: Widget = Widget::Variable {
    id: "fpc",
    label: "Finite-population correction",
    kinds: NUMBERS,
    required: false,
    hint: "Optional: the number of units in each stratum's population (or the sampling \
           fraction), for sampling without replacement.",
};

const SSU: Widget = Widget::Variable {
    id: "ssu",
    label: "Secondary sampling units",
    kinds: GROUPS,
    required: false,
    hint: "Optional, for a second stage: the units sampled within each primary unit. Needs \
           primary units, and is Stata's svyset psu || ssu.",
};

const FPC2: Widget = Widget::Variable {
    id: "fpc2",
    label: "Second-stage finite-population correction",
    kinds: NUMBERS,
    required: false,
    hint: "Optional: the number of secondary units in each primary unit's population (or the \
           sampling fraction). Give both corrections or neither.",
};

const METHODS: &[Opt] = &[
    Opt {
        value: "taylor",
        label: "Strata and clusters (linearisation)",
    },
    Opt {
        value: "BRR",
        label: "Replicate weights: balanced repeated replication (BRR)",
    },
    Opt {
        value: "JK1",
        label: "Replicate weights: delete-one jackknife (JK1)",
    },
    Opt {
        value: "bootstrap",
        label: "Replicate weights: bootstrap",
    },
];

const METHOD: Widget = Widget::Choice {
    id: "method",
    label: "Standard errors from",
    options: METHODS,
    default: "taylor",
    hint: "Replicate weights replace the strata, clusters and corrections: choose their columns \
           below. The bootstrap's variance divides by the number of replicates, as Stata's \
           bsrweight does, and its intervals use the normal distribution.",
};

const REPWEIGHTS: Widget = Widget::Variables {
    id: "repweights",
    label: "Replicate weights",
    kinds: NUMBERS,
    min: 0,
    hint: "With BRR, JK1 or bootstrap: the columns holding one full weight per replicate (each \
           already includes the sampling weight, as Stata's brrweight, jkrweight and bsrweight \
           take them). Leave empty for linearisation.",
};

const FAY: Widget = Widget::Text {
    id: "fay",
    label: "Fay's coefficient (BRR)",
    default: TextDefault::Fixed("0"),
    rule: TextRule::Number,
    hint: "0 for plain BRR; between 0 and 1 for Fay's method (Stata's fay()).",
};

const SUBPOP: Widget = Widget::Text {
    id: "subpop",
    label: "Subpopulation",
    default: TextDefault::Empty,
    rule: TextRule::Any,
    hint: "Optional: an R condition, such as region == \"north\". The estimates are for those \
           rows, with the whole design kept (Stata's subpop()); the data is not filtered. Rows \
           where it is missing are left out.",
};

const SAVE_DESIGN: Widget = Widget::Text {
    id: "design",
    label: "Save the design as",
    default: TextDefault::Fixed("design"),
    rule: TextRule::Name,
    hint: "The survey design stays in R's workspace under this name, for further survey:: \
           calls; a name already in use is replaced.",
};

/// Whether the standard errors come from replicate weights.
fn is_replicate(a: &Args) -> bool {
    a.choice("method") != "taylor"
}

/// The data the design is built on: the data frame, or, with a second stage,
/// its rows complete in every variable analysed and in the design (Stata's
/// estimation sample; the second stage's counts need it).
fn design_data(a: &Args, analysed: &[&str]) -> String {
    if a.var("ssu").is_none() || is_replicate(a) {
        return a.data();
    }
    let mut cols: Vec<&str> = analysed.to_vec();
    for id in ["weights", "psu", "ssu", "strata", "fpc", "fpc2"] {
        let v = a.var_text(id);
        if !v.is_empty() && !cols.contains(&v) {
            cols.push(v);
        }
    }
    let list: Vec<String> = cols.iter().map(|c| r_string(c)).collect();
    let d = a.data();
    format!(
        "{d}[stats::complete.cases({d}[, c({})]), , drop = FALSE]",
        list.join(", ")
    )
}

/// `design <- survey::svydesign(...)` (or `svrepdesign`), then the
/// subpopulation: the design every dialog starts from.
fn design_line(a: &Args, analysed: &[&str]) -> String {
    let design = a.name("design");
    let weights = a.var("weights").unwrap_or_default();
    let data = design_data(a, analysed);
    let mut line = if is_replicate(a) {
        let k = a.vars("repweights").len();
        let kind = match a.choice("method") {
            "BRR" => {
                let fay = a.number("fay");
                if fay.parse::<f64>().is_ok_and(|f| f != 0.0) {
                    format!(", type = \"BRR\", fay.rho = {fay}")
                } else {
                    ", type = \"BRR\"".to_string()
                }
            }
            "JK1" => format!(", type = \"JK1\", scale = ({}/{k})", k.saturating_sub(1)),
            _ => format!(", type = \"other\", scale = 1/{k}, rscales = rep(1, {k})"),
        };
        format!(
            "{design} <- survey::svrepdesign(weights = ~{weights}, repweights = {}{kind}, \
             combined.weights = TRUE, data = {data})",
            formula_of(a, "repweights")
        )
    } else {
        let mut ids = a.var("psu").unwrap_or_else(|| "1".into());
        if let Some(ssu) = a.var("ssu") {
            ids = format!("{ids} + {ssu}");
        }
        let strata = a
            .var("strata")
            .map_or_else(String::new, |s| format!(", strata = ~{s}"));
        let fpc = match (a.var("fpc"), a.var("fpc2")) {
            (Some(f), Some(g)) => format!(", fpc = ~{f} + {g}"),
            (Some(f), None) => format!(", fpc = ~{f}"),
            _ => String::new(),
        };
        let nest = if a.var("strata").is_some() && a.var("psu").is_some() {
            ", nest = TRUE"
        } else {
            ""
        };
        format!(
            "{design} <- survey::svydesign(ids = ~{ids}{strata}, weights = ~{weights}{fpc}{nest}, \
             data = {data})"
        )
    };
    let cond = a.expr("subpop");
    if !cond.is_empty() {
        line.push_str(&format!("\n{design} <- base::subset({design}, {cond})"));
    }
    line
}

/// The degrees of freedom for intervals: the design's (PSUs less strata); with
/// BRR or JK1 replicate weights the number of replicates less one, as Stata
/// takes it (survey's own count, the rank of the weights, can be one less); a
/// bootstrap design has none, as in Stata, so its intervals are normal.
fn degf_text(a: &Args) -> String {
    match a.choice("method") {
        "bootstrap" => "Inf".into(),
        "BRR" | "JK1" => a.vars("repweights").len().saturating_sub(1).to_string(),
        _ => format!("survey::degf({})", a.name("design")),
    }
}

/// `~a + b` of a list widget.
fn formula_of(a: &Args, id: &str) -> String {
    format!(
        "~{}",
        a.vars(id)
            .iter()
            .map(|n| r_name(n))
            .collect::<Vec<_>>()
            .join(" + ")
    )
}

/// The design's variables are different ones, none of them analysed, the
/// design is not saved over the data frame, and the options agree: a second
/// stage needs the first, corrections come in pairs, replicate weights take
/// the place of strata and clusters.
fn design_check(a: &Args, analysed: &[&str]) -> Vec<Problem> {
    let mut out = Vec::new();
    let mut roles: Vec<(&'static str, &str)> = ["weights", "psu", "ssu", "strata", "fpc", "fpc2"]
        .into_iter()
        .map(|id| (id, a.var_text(id)))
        .filter(|(_, v)| !v.is_empty())
        .collect();
    if is_replicate(a) {
        roles.extend(a.vars("repweights").into_iter().map(|v| ("repweights", v)));
    }
    for (k, (id, v)) in roles.iter().enumerate() {
        if roles
            .get(..k)
            .is_some_and(|r| r.iter().any(|(_, w)| w == v))
        {
            out.push(Problem::new(
                id,
                format!("{v} already has a part in the design; choose another variable."),
            ));
        }
        if analysed.contains(v) {
            out.push(Problem::new(
                id,
                format!("{v} is analysed; it cannot also be part of the design."),
            ));
        }
    }
    if a.name_text("design") == a.ctx.dataset {
        out.push(Problem::new(
            "design",
            format!("{} is the data frame; choose another name.", a.ctx.dataset),
        ));
    }
    let has = |id: &str| !a.var_text(id).is_empty();
    if is_replicate(a) {
        for id in ["psu", "ssu", "strata", "fpc", "fpc2"] {
            if has(id) {
                out.push(Problem::new(
                    id,
                    "Replicate weights stand in for strata, clusters and corrections; clear \
                     this, or choose linearisation.",
                ));
            }
        }
        if a.vars("repweights").len() < 2 {
            out.push(Problem::new(
                "repweights",
                "Choose at least two replicate-weight columns.",
            ));
        }
        if a.choice("method") == "BRR" {
            let fay = a.number("fay").parse::<f64>().unwrap_or(0.0);
            if !(0.0..1.0).contains(&fay) {
                out.push(Problem::new(
                    "fay",
                    "Fay's coefficient is 0 or between 0 and 1.",
                ));
            }
        }
    } else {
        if has("ssu") && !has("psu") {
            out.push(Problem::new(
                "ssu",
                "Secondary units sit inside primary units; choose those first.",
            ));
        }
        if has("fpc2") && !has("ssu") {
            out.push(Problem::new(
                "fpc2",
                "The second-stage correction needs secondary sampling units.",
            ));
        }
        if has("ssu") && has("fpc") != has("fpc2") {
            out.push(Problem::new(
                if has("fpc") { "fpc2" } else { "fpc" },
                "With a second stage, give the correction for both stages or for neither.",
            ));
        }
    }
    if let Some(message) = expression_problem(a.expr("subpop")) {
        out.push(Problem::new("subpop", message));
    }
    out
}

// ---- Means and totals ----

const STATISTICS: &[Opt] = &[
    Opt {
        value: "svymean",
        label: "Means",
    },
    Opt {
        value: "svytotal",
        label: "Totals",
    },
];

fn means_check(a: &Args) -> Vec<Problem> {
    let mut analysed = a.vars("x");
    let by = a.var_text("by");
    if !by.is_empty() {
        analysed.push(by);
    }
    let mut out = design_check(a, &analysed);
    if a.vars("x").contains(&by) {
        out.push(Problem::new(
            "by",
            format!("{by} is analysed; choose another variable for the groups."),
        ));
    }
    out
}

pub const SURVEY_MEANS: Spec = Spec {
    entry: "Survey means and totals",
    menu: "Statistics > Survey data > Means and totals",
    about: "Population means or totals estimated from a survey sample, with standard errors \
            from the design (Taylor linearisation, or replicate weights), overall or for each \
            group, with survey::svymean, survey::svytotal and survey::svyby. Rows with a \
            missing value in any variable chosen are left out of all of them, as Stata's svy: \
            mean does.",
    widgets: &[
        Widget::Variables {
            id: "x",
            label: "Variables",
            kinds: NUMBERS,
            min: 1,
            hint: "",
        },
        Widget::Choice {
            id: "stat",
            label: "Estimate",
            options: STATISTICS,
            default: "svymean",
            hint: "",
        },
        Widget::Variable {
            id: "by",
            label: "For each group of",
            kinds: GROUPS,
            required: false,
            hint: "Optional: estimates for each group (domains), with the whole design kept.",
        },
        WEIGHTS,
        METHOD,
        PSU,
        SSU,
        STRATA,
        FPC,
        FPC2,
        REPWEIGHTS,
        FAY,
        SUBPOP,
        CONFIDENCE,
        SAVE_DESIGN,
    ],
    needs: &["survey"],
    styles: BASE,
    check: means_check,
    code: |a, _| {
        let design = a.name("design");
        let stat = a.choice("stat");
        let vars = formula_of(a, "x");
        let estimate = match a.var("by") {
            Some(by) => {
                format!("survey::svyby({vars}, ~{by}, {design}, survey::{stat}, na.rm = TRUE)")
            }
            None => format!("survey::{stat}({vars}, {design}, na.rm = TRUE)"),
        };
        let mut analysed = a.vars("x");
        if !a.var_text("by").is_empty() {
            analysed.push(a.var_text("by"));
        }
        format!(
            "{}\nprint(local({{\n  est <- {estimate}\n  \
             cbind(est = stats::coef(est), se = survey::SE(est), \
             stats::confint(est, level = {}, df = {}))\n}}))",
            design_line(a, &analysed),
            a.choice("conf"),
            degf_text(a)
        )
    },
};

// ---- Proportions and tables ----

fn tables_check(a: &Args) -> Vec<Problem> {
    let row = a.var_text("row");
    let col = a.var_text("col");
    let mut analysed = vec![row];
    if !col.is_empty() {
        analysed.push(col);
    }
    let mut out = design_check(a, &analysed);
    if !col.is_empty() && col == row {
        out.push(Problem::new(
            "col",
            "Choose two different variables for a table.",
        ));
    }
    out
}

pub const SURVEY_TABLES: Spec = Spec {
    entry: "Survey proportions and tables",
    menu: "Statistics > Survey data > Proportions and tables",
    about: "The population share in each category of a variable, with design-based standard \
            errors and intervals (survey::svymean of its levels); with a second variable, the \
            estimated population table (survey::svytable), its proportions, and the Rao-Scott \
            test of independence (survey::svychisq), as Stata's svy: tabulate reports it.",
    widgets: &[
        Widget::Variable {
            id: "row",
            label: "Variable",
            kinds: CATEGORIES,
            required: true,
            hint: "",
        },
        Widget::Variable {
            id: "col",
            label: "Cross with",
            kinds: CATEGORIES,
            required: false,
            hint: "Optional: a second variable for a two-way table and its test.",
        },
        WEIGHTS,
        METHOD,
        PSU,
        SSU,
        STRATA,
        FPC,
        FPC2,
        REPWEIGHTS,
        FAY,
        SUBPOP,
        CONFIDENCE,
        SAVE_DESIGN,
    ],
    needs: &["survey"],
    styles: BASE,
    check: tables_check,
    code: |a, _| {
        let design = a.name("design");
        let row = a.var("row").unwrap_or_default();
        let mut analysed = vec![a.var_text("row")];
        if !a.var_text("col").is_empty() {
            analysed.push(a.var_text("col"));
        }
        let mut code = format!(
            "{}\nprint(local({{\n  est <- survey::svymean(~factor({row}), {design}, na.rm = TRUE)\n  \
             cbind(proportion = stats::coef(est), se = survey::SE(est), \
             stats::confint(est, level = {}, df = {}))\n}}))",
            design_line(a, &analysed),
            a.choice("conf"),
            degf_text(a)
        );
        if let Some(col) = a.var("col") {
            code.push_str(&format!(
                "\ntab <- survey::svytable(~{row} + {col}, {design})\nprint(tab)\n\
                 print(prop.table(tab))\nprint(survey::svychisq(~{row} + {col}, {design}))"
            ));
        }
        code
    },
};

// ---- Ratios ----

fn ratios_check(a: &Args) -> Vec<Problem> {
    let num = a.vars("num");
    let den = a.var_text("den");
    let by = a.var_text("by");
    let mut analysed = num.clone();
    analysed.push(den);
    if !by.is_empty() {
        analysed.push(by);
    }
    let mut out = design_check(a, &analysed);
    if num.contains(&den) {
        out.push(Problem::new(
            "den",
            format!("{den} is a numerator; choose a different denominator."),
        ));
    }
    if num.contains(&by) {
        out.push(Problem::new(
            "by",
            format!("{by} is analysed; choose another variable for the groups."),
        ));
    }
    if !by.is_empty() && by == den {
        out.push(Problem::new(
            "by",
            format!("{by} is the denominator; choose another variable for the groups."),
        ));
    }
    out
}

pub const SURVEY_RATIOS: Spec = Spec {
    entry: "Survey ratios",
    menu: "Statistics > Survey data > Ratios",
    about: "The population ratio of one or more variables to another (total of the first over \
            total of the second), with design-based standard errors and intervals, overall or \
            for each group, with survey::svyratio, as Stata's svy: ratio. Rows with a missing \
            value in any variable chosen are left out of all the ratios.",
    widgets: &[
        Widget::Variables {
            id: "num",
            label: "Numerators",
            kinds: NUMBERS,
            min: 1,
            hint: "Each is divided by the denominator.",
        },
        Widget::Variable {
            id: "den",
            label: "Denominator",
            kinds: NUMBERS,
            required: true,
            hint: "",
        },
        Widget::Variable {
            id: "by",
            label: "For each group of",
            kinds: GROUPS,
            required: false,
            hint: "Optional: ratios for each group (domains), with the whole design kept.",
        },
        WEIGHTS,
        METHOD,
        PSU,
        SSU,
        STRATA,
        FPC,
        FPC2,
        REPWEIGHTS,
        FAY,
        SUBPOP,
        CONFIDENCE,
        SAVE_DESIGN,
    ],
    needs: &["survey"],
    styles: BASE,
    check: ratios_check,
    code: |a, _| {
        let design = a.name("design");
        let num = formula_of(a, "num");
        let den = a.var("den").unwrap_or_default();
        let estimate = match a.var("by") {
            Some(by) => format!(
                "survey::svyby({num}, ~{by}, {design}, survey::svyratio, denominator = ~{den}, \
                 na.rm = TRUE)"
            ),
            None => format!("survey::svyratio({num}, ~{den}, {design}, na.rm = TRUE)"),
        };
        let mut analysed = a.vars("num");
        analysed.push(a.var_text("den"));
        if !a.var_text("by").is_empty() {
            analysed.push(a.var_text("by"));
        }
        format!(
            "{}\nprint(local({{\n  est <- {estimate}\n  \
             cbind(ratio = stats::coef(est), se = survey::SE(est), \
             stats::confint(est, level = {}, df = {}))\n}}))",
            design_line(a, &analysed),
            a.choice("conf"),
            degf_text(a)
        )
    },
};

// ---- Quantiles ----

/// The probabilities typed in the box, as typed, when each is a number
/// strictly between 0 and 1.
fn typed_quantiles<'a>(a: &'a Args) -> Option<Vec<&'a str>> {
    let items: Vec<&str> = a
        .values
        .text("quantiles")
        .split(',')
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .collect();
    let ok = !items.is_empty()
        && items
            .iter()
            .all(|t| t.parse::<f64>().is_ok_and(|p| p > 0.0 && p < 1.0));
    ok.then_some(items)
}

fn quantiles_check(a: &Args) -> Vec<Problem> {
    let mut out = design_check(a, &[a.var_text("y")]);
    if typed_quantiles(a).is_none() {
        out.push(Problem::new(
            "quantiles",
            "Type probabilities between 0 and 1, separated by commas (0.5 is the median).",
        ));
    }
    out
}

/// `alpha` for a confidence level of the dialog.
fn alpha_of(level: &str) -> &'static str {
    match level {
        "0.9" => "0.1",
        "0.99" => "0.01",
        _ => "0.05",
    }
}

pub const SURVEY_QUANTILES: Spec = Spec {
    entry: "Survey quantiles",
    menu: "Statistics > Survey data > Quantiles",
    about: "Population quantiles (the median, quartiles, any share) of a variable, with \
            design-based standard errors and intervals, with survey::svyquantile. The rule: a \
            quantile is the smallest value at which the weighted cumulative share of the sample \
            reaches the probability asked for (the inverse of the weighted distribution \
            function). The interval is Woodruff's, read off the design-based error of that \
            distribution function, with t on the design's degrees of freedom. Stata has no \
            command that gives the same numbers, and these are not Stata's centile or \
            summarize, detail.",
    widgets: &[
        Widget::Variable {
            id: "y",
            label: "Variable",
            kinds: NUMBERS,
            required: true,
            hint: "",
        },
        Widget::Text {
            id: "quantiles",
            label: "Probabilities",
            default: TextDefault::Fixed("0.25, 0.5, 0.75"),
            rule: TextRule::Any,
            hint: "Numbers between 0 and 1, separated by commas.",
        },
        WEIGHTS,
        METHOD,
        PSU,
        SSU,
        STRATA,
        FPC,
        FPC2,
        REPWEIGHTS,
        FAY,
        SUBPOP,
        CONFIDENCE,
        SAVE_DESIGN,
    ],
    needs: &["survey"],
    styles: BASE,
    check: quantiles_check,
    code: |a, _| {
        let design = a.name("design");
        let probs = typed_quantiles(a).unwrap_or_default().join(", ");
        let df = if a.choice("method") == "bootstrap" {
            ", df = Inf"
        } else {
            ""
        };
        format!(
            "{}\nprint(survey::svyquantile(~{}, {design}, quantiles = c({probs}), ci = TRUE, \
             alpha = {}{df}, na.rm = TRUE))",
            design_line(a, &[a.var_text("y")]),
            a.var("y").unwrap_or_default(),
            alpha_of(a.choice("conf"))
        )
    },
};

// ---- Regression ----

const FAMILIES: &[Opt] = &[
    Opt {
        value: "gaussian",
        label: "Linear",
    },
    Opt {
        value: "quasibinomial",
        label: "Logistic (0 or 1)",
    },
    Opt {
        value: "quasipoisson",
        label: "Poisson (counts)",
    },
];

fn regression_check(a: &Args) -> Vec<Problem> {
    let y = a.var_text("y");
    let mut analysed = a.vars("x");
    analysed.push(y);
    let mut out = design_check(a, &analysed);
    if a.vars("x").contains(&y) {
        out.push(Problem::new(
            "x",
            format!("{y} is the outcome; it cannot also be a predictor."),
        ));
    }
    let save = a.name_text("save");
    if save == a.ctx.dataset {
        out.push(Problem::new(
            "save",
            format!("{} is the data frame; choose another name.", a.ctx.dataset),
        ));
    }
    if save == a.name_text("design") {
        out.push(Problem::new(
            "save",
            "The model and the design need different names.",
        ));
    }
    out
}

pub const SURVEY_REGRESSION: Spec = Spec {
    entry: "Survey regression",
    menu: "Statistics > Survey data > Regression",
    about: "A linear, logistic or Poisson model fitted with the sampling weights, with standard \
            errors from the design, with survey::svyglm. Logistic and Poisson models use the \
            quasi families, as survey recommends, so the weights need not be whole numbers; \
            their estimates are the usual ones. The fitted model is a svyglm object, which the \
            Objects pane's Model menu takes for fit statistics, tests of terms and predicted \
            values.",
    widgets: &[
        Widget::Variable {
            id: "y",
            label: "Outcome",
            kinds: NUMBERS,
            required: true,
            hint: "",
        },
        Widget::Variables {
            id: "x",
            label: "Predictors",
            kinds: PREDICTORS,
            min: 1,
            hint: "A categorical predictor gets one coefficient per level after the first.",
        },
        Widget::Choice {
            id: "family",
            label: "Model",
            options: FAMILIES,
            default: "gaussian",
            hint: "",
        },
        Widget::Check {
            id: "ratios",
            label: "Show odds or rate ratios (logistic or Poisson)",
            default: true,
            hint: "",
        },
        WEIGHTS,
        METHOD,
        PSU,
        SSU,
        STRATA,
        FPC,
        FPC2,
        REPWEIGHTS,
        FAY,
        SUBPOP,
        CONFIDENCE,
        SAVE_DESIGN,
        Widget::Text {
            id: "save",
            label: "Save the model as",
            default: TextDefault::Fixed("model"),
            rule: TextRule::Name,
            hint: "The fitted model stays in R's workspace under this name; a name already in \
                   use is replaced.",
        },
    ],
    needs: &["survey"],
    styles: BASE,
    check: regression_check,
    code: |a, _| {
        let design = a.name("design");
        let model = a.name("save");
        let family = a.choice("family");
        let rhs = a
            .vars("x")
            .iter()
            .map(|n| r_name(n))
            .collect::<Vec<_>>()
            .join(" + ");
        // glm's default convergence (epsilon = 1e-8) stops a few digits short of the
        // maximum; Stata's svy: logit and svy: poisson iterate to it. The tighter
        // tolerance costs an iteration or two and brings the estimates and standard
        // errors to Stata's within 1e-10.
        let control = if family == "gaussian" {
            ""
        } else {
            ", control = stats::glm.control(epsilon = 1e-14, maxit = 100)"
        };
        let mut analysed = a.vars("x");
        analysed.push(a.var_text("y"));
        let mut code = format!(
            "{}\n{model} <- survey::svyglm({} ~ {rhs}, design = {design}, family = stats::{family}(){control})\n\
             print(summary({model}))",
            design_line(a, &analysed),
            a.var("y").unwrap_or_default(),
        );
        let ci = format!(
            "stats::confint({model}, level = {}, df = {})",
            a.choice("conf"),
            degf_text(a)
        );
        if family != "gaussian" && a.flag("ratios") {
            let (title, column) = if family == "quasibinomial" {
                ("Odds ratios", "odds_ratio")
            } else {
                ("Rate ratios", "rate_ratio")
            };
            code.push_str(&format!(
                "\ncat(\"\\n{title}\\n\")\nprint(exp(cbind({column} = stats::coef({model}), {ci})))"
            ));
        } else {
            code.push_str(&format!("\nprint({ci})"));
        }
        code
    },
};

pub const SPECS: &[&Spec] = &[
    &SURVEY_MEANS,
    &SURVEY_TABLES,
    &SURVEY_RATIOS,
    &SURVEY_QUANTILES,
    &SURVEY_REGRESSION,
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
            dataset: "d".into(),
            frames: Vec::new(),
            columns: vec![
                col("income", VarKind::Number),
                col("hours", VarKind::Number),
                col("region", VarKind::Factor),
                col("smokes", VarKind::Number),
                col("wt", VarKind::Number),
                col("school id", VarKind::Number),
                col("stratum", VarKind::Number),
                col("pop", VarKind::Number),
                col("pupil", VarKind::Number),
                col("pop2", VarKind::Number),
                col("r1", VarKind::Number),
                col("r2", VarKind::Number),
                col("r3", VarKind::Number),
            ],
        }
    }

    fn values(spec: &Spec, set: &[(&str, &str)], lists: &[(&str, &[&str])]) -> Values {
        let mut v = spec.defaults(&ctx());
        for (id, text) in set {
            v.set_text(id, *text);
        }
        for (id, names) in lists {
            v.set_list(id, names.iter().map(|n| n.to_string()).collect());
        }
        v
    }

    fn code(spec: &Spec, v: &Values) -> String {
        let out = spec.generate(&ctx(), v, Style::Base);
        assert!(out.is_ok(), "{out:?}");
        out.unwrap_or_default()
    }

    fn widgets_of(spec: &Spec, v: &Values) -> Vec<&'static str> {
        spec.problems(&ctx(), v).iter().map(|p| p.widget).collect()
    }

    #[test]
    fn means_declare_the_design_and_estimate() {
        let v = values(
            &SURVEY_MEANS,
            &[
                ("weights", "wt"),
                ("psu", "school id"),
                ("strata", "stratum"),
            ],
            &[("x", &["income", "hours"])],
        );
        assert_eq!(
            code(&SURVEY_MEANS, &v),
            "design <- survey::svydesign(ids = ~`school id`, strata = ~stratum, weights = ~wt, nest = TRUE, data = d)\n\
             print(local({\n  est <- survey::svymean(~income + hours, design, na.rm = TRUE)\n  \
             cbind(est = stats::coef(est), se = survey::SE(est), stats::confint(est, level = 0.95, \
             df = survey::degf(design)))\n}))"
        );
        // weights alone; totals by group with a correction
        let v = values(
            &SURVEY_MEANS,
            &[
                ("weights", "wt"),
                ("stat", "svytotal"),
                ("by", "region"),
                ("fpc", "pop"),
            ],
            &[("x", &["income"])],
        );
        let c = code(&SURVEY_MEANS, &v);
        assert!(
            c.starts_with(
                "design <- survey::svydesign(ids = ~1, weights = ~wt, fpc = ~pop, data = d)\n"
            ),
            "{c}"
        );
        assert!(
            c.contains(
                "est <- survey::svyby(~income, ~region, design, survey::svytotal, na.rm = TRUE)"
            ),
            "{c}"
        );
    }

    #[test]
    fn the_design_takes_each_variable_once() {
        let v = values(
            &SURVEY_MEANS,
            &[("weights", "wt"), ("psu", "wt"), ("strata", "income")],
            &[("x", &["income"])],
        );
        let widgets = widgets_of(&SURVEY_MEANS, &v);
        assert!(widgets.contains(&"psu"), "{widgets:?}");
        assert!(widgets.contains(&"strata"), "{widgets:?}");
    }

    #[test]
    fn a_second_stage_has_secondary_units_and_two_corrections() {
        let v = values(
            &SURVEY_MEANS,
            &[
                ("weights", "wt"),
                ("psu", "school id"),
                ("ssu", "pupil"),
                ("strata", "stratum"),
                ("fpc", "pop"),
                ("fpc2", "pop2"),
            ],
            &[("x", &["income"])],
        );
        assert!(SURVEY_MEANS.problems(&ctx(), &v).is_empty());
        let c = code(&SURVEY_MEANS, &v);
        // the design is built on the complete rows, the estimation sample
        assert!(
            c.starts_with(
                "design <- survey::svydesign(ids = ~`school id` + pupil, strata = ~stratum, \
                 weights = ~wt, fpc = ~pop + pop2, nest = TRUE, data = d[stats::complete.cases(\
                 d[, c(\"income\", \"wt\", \"school id\", \"pupil\", \"stratum\", \"pop\", \"pop2\")]), , drop = FALSE])\n"
            ),
            "{c}"
        );
        // one correction without the other, a second stage without a first
        let v = values(
            &SURVEY_MEANS,
            &[
                ("weights", "wt"),
                ("psu", "school id"),
                ("ssu", "pupil"),
                ("fpc", "pop"),
            ],
            &[("x", &["income"])],
        );
        assert!(widgets_of(&SURVEY_MEANS, &v).contains(&"fpc2"));
        let v = values(
            &SURVEY_MEANS,
            &[("weights", "wt"), ("ssu", "pupil")],
            &[("x", &["income"])],
        );
        assert!(widgets_of(&SURVEY_MEANS, &v).contains(&"ssu"));
        let v = values(
            &SURVEY_MEANS,
            &[("weights", "wt"), ("fpc2", "pop2")],
            &[("x", &["income"])],
        );
        assert!(widgets_of(&SURVEY_MEANS, &v).contains(&"fpc2"));
    }

    #[test]
    fn replicate_weights_make_a_replicate_design() {
        let reps: &[(&str, &[&str])] = &[("x", &["income"]), ("repweights", &["r1", "r2", "r3"])];
        let v = values(
            &SURVEY_MEANS,
            &[("weights", "wt"), ("method", "BRR"), ("fay", "0.3")],
            reps,
        );
        assert!(SURVEY_MEANS.problems(&ctx(), &v).is_empty());
        assert!(
            code(&SURVEY_MEANS, &v).starts_with(
                "design <- survey::svrepdesign(weights = ~wt, repweights = ~r1 + r2 + r3, \
                 type = \"BRR\", fay.rho = 0.3, combined.weights = TRUE, data = d)\n"
            ),
            "{}",
            code(&SURVEY_MEANS, &v)
        );
        let v = values(&SURVEY_MEANS, &[("weights", "wt"), ("method", "JK1")], reps);
        assert!(code(&SURVEY_MEANS, &v).contains("type = \"JK1\", scale = (2/3), combined"));
        let v = values(
            &SURVEY_MEANS,
            &[("weights", "wt"), ("method", "bootstrap")],
            reps,
        );
        let c = code(&SURVEY_MEANS, &v);
        assert!(
            c.contains("type = \"other\", scale = 1/3, rscales = rep(1, 3), combined"),
            "{c}"
        );
        assert!(c.contains("df = Inf"), "{c}");
        // strata with replicate weights, replicate weights with none chosen
        let v = values(
            &SURVEY_MEANS,
            &[("weights", "wt"), ("method", "JK1"), ("strata", "stratum")],
            reps,
        );
        assert!(widgets_of(&SURVEY_MEANS, &v).contains(&"strata"));
        let v = values(
            &SURVEY_MEANS,
            &[("weights", "wt"), ("method", "JK1")],
            &[("x", &["income"])],
        );
        assert!(widgets_of(&SURVEY_MEANS, &v).contains(&"repweights"));
        let v = values(&SURVEY_MEANS, &[("weights", "wt")], reps);
        assert!(SURVEY_MEANS.problems(&ctx(), &v).is_empty());
        assert!(!code(&SURVEY_MEANS, &v).contains("r1"));
        let v = values(
            &SURVEY_MEANS,
            &[("weights", "wt"), ("method", "BRR"), ("fay", "1.5")],
            reps,
        );
        assert!(widgets_of(&SURVEY_MEANS, &v).contains(&"fay"));
    }

    #[test]
    fn a_subpopulation_subsets_the_design() {
        let v = values(
            &SURVEY_MEANS,
            &[("weights", "wt"), ("subpop", "region == \"north\"")],
            &[("x", &["income"])],
        );
        assert!(SURVEY_MEANS.problems(&ctx(), &v).is_empty());
        assert!(
            code(&SURVEY_MEANS, &v).starts_with(
                "design <- survey::svydesign(ids = ~1, weights = ~wt, data = d)\n\
                 design <- base::subset(design, region == \"north\")\nprint("
            ),
            "{}",
            code(&SURVEY_MEANS, &v)
        );
        let v = values(
            &SURVEY_MEANS,
            &[("weights", "wt"), ("subpop", "region == (\"north\"")],
            &[("x", &["income"])],
        );
        assert!(widgets_of(&SURVEY_MEANS, &v).contains(&"subpop"));
    }

    #[test]
    fn tables_give_proportions_and_the_rao_scott_test() {
        let v = values(
            &SURVEY_TABLES,
            &[
                ("row", "region"),
                ("col", "smokes"),
                ("weights", "wt"),
                ("psu", "school id"),
            ],
            &[],
        );
        assert_eq!(
            code(&SURVEY_TABLES, &v),
            "design <- survey::svydesign(ids = ~`school id`, weights = ~wt, data = d)\n\
             print(local({\n  est <- survey::svymean(~factor(region), design, na.rm = TRUE)\n  \
             cbind(proportion = stats::coef(est), se = survey::SE(est), stats::confint(est, \
             level = 0.95, df = survey::degf(design)))\n}))\n\
             tab <- survey::svytable(~region + smokes, design)\nprint(tab)\nprint(prop.table(tab))\n\
             print(survey::svychisq(~region + smokes, design))"
        );
    }

    #[test]
    fn ratios_divide_numerators_by_a_denominator() {
        let v = values(
            &SURVEY_RATIOS,
            &[("den", "hours"), ("weights", "wt"), ("strata", "stratum")],
            &[("num", &["income", "smokes"])],
        );
        assert!(SURVEY_RATIOS.problems(&ctx(), &v).is_empty());
        assert_eq!(
            code(&SURVEY_RATIOS, &v),
            "design <- survey::svydesign(ids = ~1, strata = ~stratum, weights = ~wt, data = d)\n\
             print(local({\n  est <- survey::svyratio(~income + smokes, ~hours, design, na.rm = TRUE)\n  \
             cbind(ratio = stats::coef(est), se = survey::SE(est), stats::confint(est, level = 0.95, \
             df = survey::degf(design)))\n}))"
        );
        let v = values(
            &SURVEY_RATIOS,
            &[("den", "hours"), ("weights", "wt"), ("by", "region")],
            &[("num", &["income"])],
        );
        assert!(
            code(&SURVEY_RATIOS, &v).contains(
                "est <- survey::svyby(~income, ~region, design, survey::svyratio, \
                 denominator = ~hours, na.rm = TRUE)"
            ),
            "{}",
            code(&SURVEY_RATIOS, &v)
        );
        let v = values(
            &SURVEY_RATIOS,
            &[("den", "income"), ("weights", "wt")],
            &[("num", &["income"])],
        );
        assert!(widgets_of(&SURVEY_RATIOS, &v).contains(&"den"));
    }

    #[test]
    fn quantiles_name_the_probabilities() {
        let v = values(
            &SURVEY_QUANTILES,
            &[("y", "income"), ("weights", "wt"), ("conf", "0.9")],
            &[],
        );
        assert!(SURVEY_QUANTILES.problems(&ctx(), &v).is_empty());
        assert_eq!(
            code(&SURVEY_QUANTILES, &v),
            "design <- survey::svydesign(ids = ~1, weights = ~wt, data = d)\n\
             print(survey::svyquantile(~income, design, quantiles = c(0.25, 0.5, 0.75), \
             ci = TRUE, alpha = 0.1, na.rm = TRUE))"
        );
        for bad in ["", "0.5, 1.5", "median", "0"] {
            let v = values(
                &SURVEY_QUANTILES,
                &[("y", "income"), ("weights", "wt"), ("quantiles", bad)],
                &[],
            );
            assert!(
                widgets_of(&SURVEY_QUANTILES, &v).contains(&"quantiles"),
                "{bad}"
            );
        }
    }

    #[test]
    fn regression_writes_svyglm() {
        let v = values(
            &SURVEY_REGRESSION,
            &[
                ("y", "smokes"),
                ("family", "quasibinomial"),
                ("weights", "wt"),
                ("strata", "stratum"),
            ],
            &[("x", &["income", "region"])],
        );
        assert_eq!(
            code(&SURVEY_REGRESSION, &v),
            "design <- survey::svydesign(ids = ~1, strata = ~stratum, weights = ~wt, data = d)\n\
             model <- survey::svyglm(smokes ~ income + region, design = design, family = stats::quasibinomial(), \
             control = stats::glm.control(epsilon = 1e-14, maxit = 100))\n\
             print(summary(model))\ncat(\"\\nOdds ratios\\n\")\n\
             print(exp(cbind(odds_ratio = stats::coef(model), stats::confint(model, level = 0.95, \
             df = survey::degf(design)))))"
        );
        let v = values(
            &SURVEY_REGRESSION,
            &[("y", "income"), ("weights", "wt"), ("save", "design")],
            &[("x", &["hours"])],
        );
        assert!(SURVEY_REGRESSION
            .problems(&ctx(), &v)
            .iter()
            .any(|p| p.widget == "save"));
    }
}
