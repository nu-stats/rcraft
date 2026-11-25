//! Statistics menu, multilevel models (P6.2): linear mixed models with random
//! intercepts, random slopes, a nested level and crossed effects through
//! `lme4::lmer`, and logistic or Poisson models with random intercepts or
//! slopes and crossed effects through `lme4::glmer`; mixed models of
//! Gaussian, logistic, Poisson and negative binomial outcomes, with
//! zero-inflation if wanted, through `glmmTMB::glmmTMB`; and the intraclass
//! correlation of a fitted lme4 model. The defaults are Stata's (`mixed`,
//! `melogit`, `mepoisson`, `menbreg`): maximum likelihood, independent random
//! slopes, 7-point adaptive quadrature for a single random intercept and the
//! Laplace approximation otherwise (Stata's `intmethod(laplace)`). Each fit
//! keeps its model under the name the dialog asks for and prints the summary
//! and Wald intervals for the coefficients. A fit changes the data only when
//! "Save random-effect predictions" is on.

use rcraft_rcodegen::dialog::{Args, Opt, Problem, Spec, TextDefault, TextRule};
use rcraft_rcodegen::dialog::{VarKind, Widget};
use rcraft_rcodegen::r_name;

use crate::data::BASE;

const NUMBERS: &[VarKind] = &[VarKind::Number];
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
    hint: "For the Wald intervals of the coefficients printed after the summary.",
};

const SAVE: Widget = Widget::Text {
    id: "save",
    label: "Save the model as",
    default: TextDefault::Fixed("model"),
    rule: TextRule::Name,
    hint: "The fitted model stays in R's workspace under this name; a name already in use is \
           replaced.",
};

const GROUP: Widget = Widget::Variable {
    id: "group",
    label: "Groups",
    kinds: GROUPS,
    required: true,
    hint: "What the rows are nested in (pupils in schools): each group gets its own random \
           intercept.",
};

const CROSSED: Widget = Widget::Variable {
    id: "crossed",
    label: "Crossed groups (optional)",
    kinds: GROUPS,
    required: false,
    hint: "A second grouping that cuts across the first (pupils in schools and in \
           neighbourhoods, subjects rated by the same raters), with its own random intercept, \
           as Stata's || _all: R.a || b:.",
};

const RANEF: Widget = Widget::Check {
    id: "ranef",
    label: "Save random-effect predictions in the data",
    default: false,
    hint: "Adds a column u_<group> (and u_<slope>_<group> for a random slope) with each row's \
           group's conditional mode, as Stata's predict, reffects (predict, remodes after \
           melogit and mepoisson). A row left out of the fit for a missing predictor gets its \
           group's value too. The columns are added to the data frame; a name already used is \
           replaced.",
};

/// The fixed part's predictors, or 1 for none.
fn fixed(a: &Args) -> String {
    match a.vars("x") {
        x if x.is_empty() => "1".to_string(),
        x => x.iter().map(|n| r_name(n)).collect::<Vec<_>>().join(" + "),
    }
}

/// The outcome and groups each in one role, the model not saved over the data.
fn roles(a: &Args, singles: &[(&'static str, &str)]) -> Vec<Problem> {
    let mut out = Vec::new();
    let y = a.var_text("y");
    let x = a.vars("x");
    if x.contains(&y) {
        out.push(Problem::new(
            "x",
            format!("{y} is the outcome; it cannot also be a predictor."),
        ));
    }
    let mut seen: Vec<(&str, &str)> = vec![(y, "outcome")];
    for (id, what) in singles {
        let v = a.var_text(id);
        if v.is_empty() {
            continue;
        }
        if let Some((_, first)) = seen.iter().find(|(s, _)| *s == v) {
            out.push(Problem::new(
                id,
                format!("{v} is already the {first}; choose another variable."),
            ));
        } else if x.contains(&v) {
            out.push(Problem::new(
                id,
                format!("{v} is a predictor; choose another variable for the {what}."),
            ));
        } else {
            seen.push((v, what));
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

// ---- Linear mixed model ----

const METHODS: &[Opt] = &[
    Opt {
        value: "ml",
        label: "Maximum likelihood (Stata's default)",
    },
    Opt {
        value: "reml",
        label: "Restricted maximum likelihood (REML, lme4's default)",
    },
];

const COVARIANCES: &[Opt] = &[
    Opt {
        value: "independent",
        label: "Independent (Stata's default)",
    },
    Opt {
        value: "unstructured",
        label: "Correlated with the intercept (lme4's default)",
    },
];

/// A random slope on the outcome or on a grouping is nonsense.
fn slope_problems(a: &Args) -> Vec<Problem> {
    let y = a.var_text("y");
    let others = [
        a.var_text("group"),
        a.var_text("inner"),
        a.var_text("crossed"),
        a.var_text("exposure"),
    ];
    a.vars("slopes")
        .into_iter()
        .filter(|s| *s == y || others.contains(s))
        .map(|s| {
            Problem::new(
                "slopes",
                format!("{s} cannot have a random slope: it is the outcome or a grouping."),
            )
        })
        .collect()
}

/// The random part of lme4's (and glmmTMB's) formula: a random intercept for
/// each group, with the slopes in the same term, a nested level inside it and
/// a crossed group beside it.
fn random_part(a: &Args, nested: bool) -> String {
    let group = a.var("group").unwrap_or_default();
    let slopes = a.vars("slopes");
    let mut random = if slopes.is_empty() {
        format!("(1 | {group})")
    } else {
        let bar = if a.choice("covariance") == "unstructured" {
            "|"
        } else {
            "||"
        };
        format!(
            "(1 + {} {bar} {group})",
            slopes
                .iter()
                .map(|n| r_name(n))
                .collect::<Vec<_>>()
                .join(" + ")
        )
    };
    if let (true, Some(inner)) = (nested, a.var("inner")) {
        random.push_str(&format!(" + (1 | {group}:{inner})"));
    }
    if let Some(crossed) = a.var("crossed") {
        random.push_str(&format!(" + (1 | {crossed})"));
    }
    random
}

/// The code that adds each random effect's conditional modes to the data
/// frame as new columns, `u_<group>` and `u_<slope>_<group>`. Every row gets
/// its group's value, also a row the fit left out for a missing predictor, as
/// Stata's predict does. `modes` is the R expression for the list of
/// per-group data frames.
fn save_ranef(data: &str, modes: &str) -> String {
    format!(
        "\n{data} <- (function(d) {{\n  re <- {modes}\n  \
         for (g in names(re)) {{\n    \
         key <- do.call(paste, c(lapply(strsplit(g, \":\", fixed = TRUE)[[1]], \
         function(v) as.character(d[[v]])), sep = \":\"))\n    \
         for (t in names(re[[g]])) {{\n      \
         col <- paste(c(\"u\", if (t != \"(Intercept)\") t, gsub(\":\", \"_\", g)), collapse = \"_\")\n      \
         d[[col]] <- re[[g]][[t]][match(key, rownames(re[[g]]))]\n    }}\n  }}\n  \
         d\n}})({data})"
    )
}

fn linear_check(a: &Args) -> Vec<Problem> {
    let mut out = roles(
        a,
        &[
            ("group", "groups"),
            ("inner", "nested groups"),
            ("crossed", "crossed groups"),
        ],
    );
    out.extend(slope_problems(a));
    out
}

pub const LINEAR_MIXED: Spec = Spec {
    entry: "Linear mixed model",
    menu: "Statistics > Multilevel models > Linear mixed model",
    about: "A linear model for rows nested in groups (pupils in schools, visits in patients): \
            each group gets a random intercept, and optionally random slopes and a nested level \
            (classes within schools), as Stata's mixed. It fits with lme4::lmer, by maximum \
            likelihood as Stata does unless REML is chosen, with tight convergence settings to \
            match Stata's results, and \
            prints the summary and Wald intervals for the coefficients.",
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
            min: 0,
            hint: "The fixed part. Leave empty for the intercept alone.",
        },
        GROUP,
        Widget::Variable {
            id: "inner",
            label: "Nested groups (optional)",
            kinds: GROUPS,
            required: false,
            hint: "A level within the groups (classes within schools), with its own random \
                   intercept, as Stata's || school: || class:.",
        },
        CROSSED,
        Widget::Variables {
            id: "slopes",
            label: "Random slopes (optional)",
            kinds: NUMBERS,
            min: 0,
            hint: "Predictors whose effect varies from group to group. Usually also among the \
                   predictors.",
        },
        Widget::Choice {
            id: "covariance",
            label: "Random slopes are",
            options: COVARIANCES,
            default: "independent",
            hint: "Whether the slopes may be correlated with the intercept; only used with \
                   random slopes.",
        },
        Widget::Choice {
            id: "method",
            label: "Estimation",
            options: METHODS,
            default: "ml",
            hint: "",
        },
        CONFIDENCE,
        RANEF,
        SAVE,
    ],
    needs: &["lme4"],
    styles: BASE,
    check: linear_check,
    code: |a, _| {
        let model = a.name("save");
        let random = random_part(a, true);
        let reml = if a.choice("method") == "reml" {
            "TRUE"
        } else {
            "FALSE"
        };
        // lmer's default stopping rule can leave a fit with random slopes
        // about 1e-6 from the maximum; these tolerances reach it
        let mut code = format!(
            "{model} <- lme4::lmer({} ~ {} + {random}, data = {}, REML = {reml}, \
             control = lme4::lmerControl(optCtrl = list(ftol_abs = 1e-14, ftol_rel = 1e-14, \
             xtol_abs = 1e-14, xtol_rel = 1e-14, maxeval = 1e5)))\nprint(summary({model}))\n\
             print(stats::confint({model}, parm = \"beta_\", method = \"Wald\", level = {}))",
            a.var("y").unwrap_or_default(),
            fixed(a),
            a.data(),
            a.choice("conf"),
        );
        if a.flag("ranef") {
            code.push_str(&save_ranef(&a.data(), &format!("lme4::ranef({model})")));
        }
        code
    },
};

// ---- Logistic or Poisson mixed model ----

const FAMILIES: &[Opt] = &[
    Opt {
        value: "binomial",
        label: "Logistic (0 or 1)",
    },
    Opt {
        value: "poisson",
        label: "Poisson (counts)",
    },
];

fn generalized_check(a: &Args) -> Vec<Problem> {
    let mut out = roles(
        a,
        &[
            ("group", "groups"),
            ("crossed", "crossed groups"),
            ("exposure", "exposure"),
        ],
    );
    out.extend(slope_problems(a));
    if !a.var_text("exposure").is_empty() && a.choice("family") != "poisson" {
        out.push(Problem::new(
            "exposure",
            "An exposure is for the Poisson model only; leave it empty for the logistic.",
        ));
    }
    out
}

pub const GENERALIZED_MIXED: Spec = Spec {
    entry: "Logistic or Poisson mixed model",
    menu: "Statistics > Multilevel models > Logistic or Poisson mixed model",
    about: "A logistic model of a 0/1 outcome, or a Poisson model of counts, for rows nested in \
            groups, each group with a random intercept, as Stata's melogit and mepoisson. It \
            fits with lme4::glmer by 7-point adaptive Gauss-Hermite quadrature (Stata's default \
            number of points), with tight convergence settings to match Stata's results, and prints \
            the summary and the odds \
            or rate ratios with Wald intervals.",
    widgets: &[
        Widget::Variable {
            id: "y",
            label: "Outcome",
            kinds: NUMBERS,
            required: true,
            hint: "0 and 1 for the logistic model; counts of zero or more for Poisson.",
        },
        Widget::Choice {
            id: "family",
            label: "Model",
            options: FAMILIES,
            default: "binomial",
            hint: "",
        },
        Widget::Variables {
            id: "x",
            label: "Predictors",
            kinds: PREDICTORS,
            min: 0,
            hint: "Leave empty for the intercept alone.",
        },
        GROUP,
        Widget::Variable {
            id: "exposure",
            label: "Exposure (Poisson only)",
            kinds: NUMBERS,
            required: false,
            hint: "Optional: how long or how much each row was observed; its log is the offset, \
                   as Stata's exposure(). Must be above zero.",
        },
        Widget::Variables {
            id: "slopes",
            label: "Random slopes (optional)",
            kinds: NUMBERS,
            min: 0,
            hint: "Predictors whose effect varies from group to group. Usually also among the \
                   predictors. With slopes (or crossed groups) the fit uses the Laplace \
                   approximation, as Stata's intmethod(laplace).",
        },
        Widget::Choice {
            id: "covariance",
            label: "Random slopes are",
            options: COVARIANCES,
            default: "independent",
            hint: "Whether the slopes may be correlated with the intercept; only used with \
                   random slopes.",
        },
        CROSSED,
        Widget::Check {
            id: "ratios",
            label: "Show odds or rate ratios with confidence intervals",
            default: true,
            hint: "",
        },
        CONFIDENCE,
        RANEF,
        SAVE,
    ],
    needs: &["lme4"],
    styles: BASE,
    check: generalized_check,
    code: |a, _| {
        let model = a.name("save");
        let poisson = a.choice("family") == "poisson";
        let offset = match (poisson, a.var("exposure")) {
            (true, Some(e)) => format!(", offset = log({e})"),
            _ => String::new(),
        };
        // glmer's default stopping rules leave the variance about 1e-5 from
        // the maximum; these reach it
        // 7-point quadrature for one random intercept; glmer allows nothing
        // but the Laplace approximation (nAGQ = 1) with random slopes or more
        // than one random effect, as Stata's intmethod(laplace)
        let points = if a.vars("slopes").is_empty() && a.var("crossed").is_none() {
            7
        } else {
            1
        };
        let mut code = format!(
            "{model} <- lme4::glmer({} ~ {} + {}, data = {}, family = stats::{}{offset}, \
             nAGQ = {points}, control = lme4::glmerControl(optimizer = \"bobyqa\", \
             optCtrl = list(rhoend = 1e-12, maxfun = 1e5), tolPwrss = 1e-12))\n\
             print(summary({model}))",
            a.var("y").unwrap_or_default(),
            fixed(a),
            random_part(a, false),
            a.data(),
            a.choice("family"),
        );
        let ci = format!(
            "stats::confint({model}, parm = \"beta_\", method = \"Wald\", level = {})",
            a.choice("conf")
        );
        if a.flag("ratios") {
            let (title, column) = if poisson {
                ("Rate ratios", "rate_ratio")
            } else {
                ("Odds ratios", "odds_ratio")
            };
            code.push_str(&format!(
                "\ncat(\"\\n{title}\\n\")\nprint(exp(cbind({column} = lme4::fixef({model}), {ci})))"
            ));
        } else {
            code.push_str(&format!("\nprint({ci})"));
        }
        if a.flag("ranef") {
            code.push_str(&save_ranef(
                &a.data(),
                &format!("lme4::ranef({model})"),
            ));
        }
        code
    },
};

// ---- Mixed model by glmmTMB ----

const TMB_FAMILIES: &[Opt] = &[
    Opt {
        value: "gaussian",
        label: "Linear (continuous outcome)",
    },
    Opt {
        value: "binomial",
        label: "Logistic (0 or 1)",
    },
    Opt {
        value: "poisson",
        label: "Poisson (counts)",
    },
    Opt {
        value: "nbinom2",
        label: "Negative binomial (overdispersed counts)",
    },
];

fn tmb_check(a: &Args) -> Vec<Problem> {
    let mut out = roles(
        a,
        &[
            ("group", "groups"),
            ("crossed", "crossed groups"),
            ("exposure", "exposure"),
        ],
    );
    out.extend(slope_problems(a));
    let counts = matches!(a.choice("family"), "poisson" | "nbinom2");
    if !a.var_text("exposure").is_empty() && !counts {
        out.push(Problem::new(
            "exposure",
            "An exposure is for the Poisson and negative binomial models; leave it empty here.",
        ));
    }
    if a.flag("zi") && !counts {
        out.push(Problem::new(
            "zi",
            "Zero-inflation is for counts: choose the Poisson or negative binomial model.",
        ));
    }
    out
}

pub const TMB_MIXED: Spec = Spec {
    entry: "Mixed model (glmmTMB)",
    menu: "Statistics > Multilevel models > Mixed model (glmmTMB)",
    about: "A mixed model for rows nested in groups with a linear, logistic, Poisson or negative \
            binomial outcome, with random intercepts, random slopes and a crossed group, fitted \
            with glmmTMB::glmmTMB by maximum likelihood. Non-linear outcomes use the Laplace \
            approximation, as Stata's melogit, mepoisson and menbreg with intmethod(laplace); \
            the negative binomial is the one whose variance is mu + mu^2/theta, as menbreg's \
            default. Zero-inflation (excess zeros, one probability for every row) can be added \
            for counts. It prints the summary and Wald intervals for every parameter.",
    widgets: &[
        Widget::Variable {
            id: "y",
            label: "Outcome",
            kinds: NUMBERS,
            required: true,
            hint: "0 and 1 for the logistic model; counts of zero or more for Poisson and \
                   negative binomial.",
        },
        Widget::Choice {
            id: "family",
            label: "Model",
            options: TMB_FAMILIES,
            default: "gaussian",
            hint: "",
        },
        Widget::Variables {
            id: "x",
            label: "Predictors",
            kinds: PREDICTORS,
            min: 0,
            hint: "The fixed part. Leave empty for the intercept alone.",
        },
        GROUP,
        Widget::Variables {
            id: "slopes",
            label: "Random slopes (optional)",
            kinds: NUMBERS,
            min: 0,
            hint: "Predictors whose effect varies from group to group. Usually also among the \
                   predictors.",
        },
        Widget::Choice {
            id: "covariance",
            label: "Random slopes are",
            options: COVARIANCES,
            default: "independent",
            hint: "Whether the slopes may be correlated with the intercept; only used with \
                   random slopes.",
        },
        CROSSED,
        Widget::Variable {
            id: "exposure",
            label: "Exposure (counts only)",
            kinds: NUMBERS,
            required: false,
            hint: "Optional: how long or how much each row was observed; its log is the offset. \
                   Must be above zero.",
        },
        Widget::Check {
            id: "zi",
            label: "Add zero-inflation (counts only)",
            default: false,
            hint: "Allows extra zeros beyond what the count model gives, with one probability \
                   for every row.",
        },
        CONFIDENCE,
        SAVE,
    ],
    needs: &["glmmTMB"],
    styles: BASE,
    check: tmb_check,
    code: |a, _| {
        let model = a.name("save");
        let family = match a.choice("family") {
            "nbinom2" => "glmmTMB::nbinom2".to_string(),
            f => format!("stats::{f}"),
        };
        let offset = match a.var("exposure") {
            Some(e) if matches!(a.choice("family"), "poisson" | "nbinom2") => {
                format!(" + offset(log({e}))")
            }
            _ => String::new(),
        };
        let zi = if a.flag("zi") {
            ", ziformula = ~ 1"
        } else {
            ""
        };
        format!(
            "{model} <- glmmTMB::glmmTMB({} ~ {}{offset} + {}, data = {}, family = {family}{zi}, \
             REML = FALSE)\nprint(summary({model}))\nprint(stats::confint({model}, level = {}))",
            a.var("y").unwrap_or_default(),
            fixed(a),
            random_part(a, false),
            a.data(),
            a.choice("conf"),
        )
    },
};

// ---- Intraclass correlation ----

pub const ICC: Spec = Spec {
    entry: "Intraclass correlation",
    menu: "Objects > Model > Intraclass correlation",
    about: "How alike two rows of the same group are: the variance of the group intercepts over \
            their sum with the residual variance, from a model fitted by lmer or glmer, as \
            Stata's estat icc. For a logistic model the residual variance is pi^2/3 (the \
            variance of the logistic latent variable), for a probit model 1. With nested levels \
            each level's correlation adds the levels above it; random slopes are left out of it \
            (the correlation where the slopes' variables are zero). Crossed groups and models \
            for counts have none.",
    widgets: &[],
    needs: &[],
    styles: BASE,
    check: |_| Vec::new(),
    code: |a, _| {
        let m = a.data();
        format!(
            "(function(m) {{\n  \
             stopifnot(\"this works for models fitted by lmer or glmer\" = inherits(m, \"merMod\"))\n  \
             resid <- if (!inherits(m, \"glmerMod\")) stats::sigma(m)^2 else {{\n    \
             f <- stats::family(m)\n    \
             if (f$family == \"binomial\" && f$link == \"logit\") pi^2 / 3 else \
             if (f$family == \"binomial\" && f$link == \"probit\") 1 else \
             stop(\"the intraclass correlation is defined for linear, logistic and probit models\")\n  }}\n  \
             v <- as.data.frame(lme4::VarCorr(m))\n  \
             v <- v[v$grp != \"Residual\" & v$var1 == \"(Intercept)\" & is.na(v$var2), ]\n  \
             parts <- strsplit(v$grp, \":\", fixed = TRUE)\n  \
             v <- v[order(lengths(parts)), ]\n  parts <- parts[order(lengths(parts))]\n  \
             nested <- all(vapply(seq_along(parts), function(i) length(parts[[i]]) == i && \
             (i == 1 || identical(parts[[i]][seq_len(i - 1)], parts[[i - 1]])), logical(1)))\n  \
             stopifnot(\"crossed groups have no intraclass correlation\" = nested)\n  \
             inner <- vapply(parts, function(p) p[length(p)], \"\")\n  \
             level <- vapply(seq_along(inner), function(i) paste(inner[seq_len(i)], collapse = \">\"), \"\")\n  \
             cat(\"Residual variance: \", format(resid, digits = 10), \"\\n\\n\", sep = \"\")\n  \
             print(data.frame(level = level, variance = v$vcov, icc = cumsum(v$vcov) / (sum(v$vcov) + resid)))\n  \
             invisible(NULL)\n}})({m})"
        )
    },
};

pub const SPECS: &[&Spec] = &[&LINEAR_MIXED, &GENERALIZED_MIXED, &TMB_MIXED];

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
                col("score", VarKind::Number),
                col("hours", VarKind::Number),
                col("girl", VarKind::Number),
                col("school", VarKind::Number),
                col("class", VarKind::Text),
                col("passed", VarKind::Number),
                col("absences", VarKind::Number),
                col("days", VarKind::Number),
                col("home town", VarKind::Text),
                col("rater", VarKind::Number),
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

    const LMER_CONTROL: &str = "control = lme4::lmerControl(optCtrl = list(ftol_abs = 1e-14, \
         ftol_rel = 1e-14, xtol_abs = 1e-14, xtol_rel = 1e-14, maxeval = 1e5)))";

    #[test]
    fn linear_mixed_writes_lmer() {
        let v = values(
            &LINEAR_MIXED,
            &[("y", "score"), ("group", "school")],
            &[("x", &["hours", "girl"])],
        );
        assert_eq!(
            code(&LINEAR_MIXED, &v),
            format!(
                "model <- lme4::lmer(score ~ hours + girl + (1 | school), data = d, REML = FALSE, \
                 {LMER_CONTROL}\nprint(summary(model))\n\
                 print(stats::confint(model, parm = \"beta_\", method = \"Wald\", level = 0.95))"
            )
        );
        // independent slopes (||), a nested level, REML
        let v = values(
            &LINEAR_MIXED,
            &[
                ("y", "score"),
                ("group", "school"),
                ("inner", "class"),
                ("method", "reml"),
            ],
            &[("x", &["hours"]), ("slopes", &["hours"])],
        );
        assert!(code(&LINEAR_MIXED, &v).starts_with(
            "model <- lme4::lmer(score ~ hours + (1 + hours || school) + (1 | school:class), \
             data = d, REML = TRUE,"
        ));
        let v = values(
            &LINEAR_MIXED,
            &[
                ("y", "score"),
                ("group", "school"),
                ("covariance", "unstructured"),
            ],
            &[("slopes", &["hours"])],
        );
        assert!(code(&LINEAR_MIXED, &v)
            .starts_with("model <- lme4::lmer(score ~ 1 + (1 + hours | school), data = d,"));
    }

    #[test]
    fn linear_mixed_checks_roles() {
        let v = values(
            &LINEAR_MIXED,
            &[("y", "score"), ("group", "hours"), ("inner", "score")],
            &[("x", &["hours"]), ("slopes", &["score"])],
        );
        let problems = LINEAR_MIXED.problems(&ctx(), &v);
        let widgets: Vec<&str> = problems.iter().map(|p| p.widget).collect();
        assert!(widgets.contains(&"group"), "{problems:?}");
        assert!(widgets.contains(&"inner"), "{problems:?}");
        assert!(widgets.contains(&"slopes"), "{problems:?}");
    }

    #[test]
    fn generalized_mixed_writes_glmer() {
        let v = values(
            &GENERALIZED_MIXED,
            &[("y", "passed"), ("group", "school")],
            &[("x", &["hours", "girl"])],
        );
        assert_eq!(
            code(&GENERALIZED_MIXED, &v),
            "model <- lme4::glmer(passed ~ hours + girl + (1 | school), data = d, \
             family = stats::binomial, nAGQ = 7, control = lme4::glmerControl(optimizer = \"bobyqa\", \
             optCtrl = list(rhoend = 1e-12, maxfun = 1e5), tolPwrss = 1e-12))\n\
             print(summary(model))\ncat(\"\\nOdds ratios\\n\")\n\
             print(exp(cbind(odds_ratio = lme4::fixef(model), stats::confint(model, parm = \"beta_\", \
             method = \"Wald\", level = 0.95))))"
        );
        let v = values(
            &GENERALIZED_MIXED,
            &[
                ("y", "absences"),
                ("family", "poisson"),
                ("group", "school"),
                ("exposure", "days"),
            ],
            &[("x", &["hours"])],
        );
        assert!(code(&GENERALIZED_MIXED, &v)
            .contains("family = stats::poisson, offset = log(days), nAGQ = 7"));
        let v = values(
            &GENERALIZED_MIXED,
            &[("y", "passed"), ("group", "school"), ("exposure", "days")],
            &[("x", &["hours"])],
        );
        assert!(GENERALIZED_MIXED
            .problems(&ctx(), &v)
            .iter()
            .any(|p| p.widget == "exposure"));
    }

    #[test]
    fn the_new_options_are_in_the_code() {
        // crossed groups beside the group, and the predictions saved
        let v = values(
            &LINEAR_MIXED,
            &[("y", "score"), ("group", "school"), ("crossed", "rater")],
            &[("x", &["hours"])],
        );
        assert!(code(&LINEAR_MIXED, &v)
            .contains("score ~ hours + (1 | school) + (1 | rater), data = d, REML = FALSE"));
        let mut v = values(
            &LINEAR_MIXED,
            &[("y", "score"), ("group", "school")],
            &[("x", &["hours"])],
        );
        assert!(!code(&LINEAR_MIXED, &v).contains("ranef"));
        v.set_flag("ranef", true);
        let text = code(&LINEAR_MIXED, &v);
        assert!(
            text.contains("d <- (function(d) {\n  re <- lme4::ranef(model)"),
            "{text}"
        );
        assert!(text.ends_with("d\n})(d)"), "{text}");
        // slopes and crossed groups in the logistic model need the Laplace
        // approximation; a lone random intercept keeps 7 points
        let v = values(
            &GENERALIZED_MIXED,
            &[("y", "passed"), ("group", "school")],
            &[("x", &["hours"]), ("slopes", &["hours"])],
        );
        let text = code(&GENERALIZED_MIXED, &v);
        assert!(
            text.contains("passed ~ hours + (1 + hours || school), data = d"),
            "{text}"
        );
        assert!(text.contains("nAGQ = 1,"), "{text}");
        let v = values(
            &GENERALIZED_MIXED,
            &[
                ("y", "passed"),
                ("group", "school"),
                ("covariance", "unstructured"),
            ],
            &[("slopes", &["hours", "girl"])],
        );
        assert!(code(&GENERALIZED_MIXED, &v)
            .contains("passed ~ 1 + (1 + hours + girl | school), data = d"));
        let v = values(
            &GENERALIZED_MIXED,
            &[("y", "passed"), ("group", "school"), ("crossed", "rater")],
            &[],
        );
        let text = code(&GENERALIZED_MIXED, &v);
        assert!(text.contains("(1 | school) + (1 | rater)"), "{text}");
        assert!(text.contains("nAGQ = 1,"), "{text}");
        let v = values(
            &GENERALIZED_MIXED,
            &[("y", "passed"), ("group", "school")],
            &[],
        );
        assert!(code(&GENERALIZED_MIXED, &v).contains("nAGQ = 7,"));
        // a slope on a grouping, or a crossed group that is the group, is refused
        let v = values(
            &GENERALIZED_MIXED,
            &[("y", "passed"), ("group", "school"), ("crossed", "school")],
            &[("slopes", &["school", "passed"])],
        );
        let problems = GENERALIZED_MIXED.problems(&ctx(), &v);
        let widgets: Vec<&str> = problems.iter().map(|p| p.widget).collect();
        assert!(widgets.contains(&"crossed"), "{problems:?}");
        assert_eq!(widgets.iter().filter(|w| **w == "slopes").count(), 2);
    }

    #[test]
    fn tmb_mixed_writes_glmmtmb() {
        let v = values(
            &TMB_MIXED,
            &[("y", "score"), ("group", "school")],
            &[("x", &["hours", "girl"])],
        );
        assert_eq!(
            code(&TMB_MIXED, &v),
            "model <- glmmTMB::glmmTMB(score ~ hours + girl + (1 | school), data = d, \
             family = stats::gaussian, REML = FALSE)\nprint(summary(model))\n\
             print(stats::confint(model, level = 0.95))"
        );
        let mut v = values(
            &TMB_MIXED,
            &[
                ("y", "absences"),
                ("family", "nbinom2"),
                ("group", "school"),
                ("exposure", "days"),
                ("crossed", "rater"),
                ("covariance", "unstructured"),
            ],
            &[("x", &["hours"]), ("slopes", &["hours"])],
        );
        v.set_flag("zi", true);
        let text = code(&TMB_MIXED, &v);
        assert!(
            text.contains(
                "glmmTMB::glmmTMB(absences ~ hours + offset(log(days)) + (1 + hours | school) \
                 + (1 | rater), data = d, family = glmmTMB::nbinom2, ziformula = ~ 1, REML = FALSE)"
            ),
            "{text}"
        );
        // zero-inflation and an exposure are for counts
        let mut v = values(
            &TMB_MIXED,
            &[
                ("y", "passed"),
                ("family", "binomial"),
                ("group", "school"),
                ("exposure", "days"),
            ],
            &[],
        );
        v.set_flag("zi", true);
        let problems = TMB_MIXED.problems(&ctx(), &v);
        let widgets: Vec<&str> = problems.iter().map(|p| p.widget).collect();
        assert!(
            widgets.contains(&"zi") && widgets.contains(&"exposure"),
            "{problems:?}"
        );
    }

    #[test]
    fn intraclass_correlation_reads_the_model_named() {
        let mut c = ctx();
        c.dataset = "my fit".into();
        let v = ICC.defaults(&c);
        let out = ICC.generate(&c, &v, Style::Base);
        assert!(out.is_ok(), "{out:?}");
        let text = out.unwrap_or_default();
        assert!(text.ends_with("})(`my fit`)"), "{text}");
        assert!(text.contains("pi^2 / 3"), "{text}");
        assert!(text.contains("stopifnot(\"crossed groups have no intraclass correlation\""));
    }
}
