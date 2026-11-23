//! Statistics menu, Bayesian regression (P6.9): a Gaussian, binomial (logit) or
//! Poisson (log) regression fitted by Hamiltonian Monte Carlo with
//! `rstanarm::stan_glm`, with either rstanarm's default weakly informative
//! priors or a normal(m, s) prior on every coefficient. It keeps the fit in the
//! workspace under the name the dialog asks for and prints each parameter's
//! posterior mean, standard deviation, credible interval, Monte Carlo standard
//! error, effective sample size and R-hat. It does not change the data.

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

const FAMILIES: &[Opt] = &[
    Opt {
        value: "gaussian",
        label: "Linear (Gaussian outcome)",
    },
    Opt {
        value: "binomial",
        label: "Logit (0/1 outcome)",
    },
    Opt {
        value: "poisson",
        label: "Poisson (count outcome)",
    },
];

const PRIORS: &[Opt] = &[
    Opt {
        value: "default",
        label: "rstanarm's default weakly informative priors",
    },
    Opt {
        value: "normal",
        label: "Normal(mean, standard deviation) on every coefficient",
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

/// A whole number in `least..=most`, or a problem on the widget.
fn whole(a: &Args, id: &'static str, what: &str, least: u32, most: u32, out: &mut Vec<Problem>) {
    match a.number(id).trim().parse::<u32>() {
        Ok(n) if (least..=most).contains(&n) => {}
        _ => out.push(Problem::new(
            id,
            format!("{what} must be a whole number from {least} to {most}."),
        )),
    }
}

fn bayes_check(a: &Args) -> Vec<Problem> {
    let y = a.var_text("y");
    let mut out = Vec::new();
    if a.vars("x").contains(&y) {
        out.push(Problem::new(
            "x",
            format!("{y} is the outcome; it cannot also be a predictor."),
        ));
    }
    if a.choice("prior") == "normal" {
        match a.number("prior_sd").trim().parse::<f64>() {
            Ok(s) if s > 0.0 && s.is_finite() => {}
            _ => out.push(Problem::new(
                "prior_sd",
                "The prior's standard deviation must be a number above 0.",
            )),
        }
    }
    whole(a, "chains", "The number of chains", 1, 64, &mut out);
    whole(
        a,
        "iter",
        "The number of iterations",
        100,
        1_000_000,
        &mut out,
    );
    whole(a, "seed", "The seed", 0, 2_147_483_647, &mut out);
    if a.name_text("save") == a.ctx.dataset {
        out.push(Problem::new(
            "save",
            format!("{} is the data frame; choose another name.", a.ctx.dataset),
        ));
    }
    out
}

pub const BAYESIAN_REGRESSION: Spec = Spec {
    entry: "Bayesian regression",
    menu: "Statistics > Bayesian regression",
    about: "Fits a regression by Bayes' rule, sampling the posterior with Hamiltonian Monte \
            Carlo (rstanarm::stan_glm): a linear model with a Gaussian outcome, a logit model \
            of a 0/1 outcome or a Poisson model of counts. The priors are rstanarm's default \
            weakly informative ones, or a normal(mean, standard deviation) on every coefficient \
            as typed (the intercept and, in the linear model, the error scale keep their default \
            priors). The table gives each parameter's posterior mean, standard deviation and \
            equal-tailed credible interval, its Monte Carlo standard error (mcse), its effective \
            sample size (n_eff) and R-hat, which is close to 1 once the chains agree. The \
            iterations are per chain, the first half of them warm-up. The same seed gives the \
            same draws. Rows with a missing value are left out.",
    widgets: &[
        Widget::Variable {
            id: "y",
            label: "Outcome",
            kinds: NUMBERS,
            required: true,
            hint: "A 0/1 variable for the logit model, a count for the Poisson model.",
        },
        Widget::Variables {
            id: "x",
            label: "Predictors",
            kinds: PREDICTORS,
            min: 1,
            hint: "A text or factor variable becomes dummy variables for its levels.",
        },
        Widget::Choice {
            id: "family",
            label: "Model",
            options: FAMILIES,
            default: "gaussian",
            hint: "",
        },
        Widget::Choice {
            id: "prior",
            label: "Prior on the coefficients",
            options: PRIORS,
            default: "default",
            hint: "The default priors scale with the data; a typed normal prior is used as it is, \
                   on the scale of the data.",
        },
        Widget::Text {
            id: "prior_mean",
            label: "Prior mean",
            default: TextDefault::Fixed("0"),
            rule: TextRule::Number,
            hint: "Used with the normal prior.",
        },
        Widget::Text {
            id: "prior_sd",
            label: "Prior standard deviation",
            default: TextDefault::Fixed("10"),
            rule: TextRule::Number,
            hint: "Used with the normal prior; above 0.",
        },
        Widget::Text {
            id: "chains",
            label: "Chains",
            default: TextDefault::Fixed("4"),
            rule: TextRule::Number,
            hint: "Independent runs of the sampler; R-hat compares them.",
        },
        Widget::Text {
            id: "iter",
            label: "Iterations per chain",
            default: TextDefault::Fixed("2000"),
            rule: TextRule::Number,
            hint: "Warm-up included: the first half is thrown away.",
        },
        Widget::Text {
            id: "seed",
            label: "Seed",
            default: TextDefault::Fixed("1"),
            rule: TextRule::Number,
            hint: "The same seed gives the same draws.",
        },
        Widget::Choice {
            id: "level",
            label: "Credible interval",
            options: LEVELS,
            default: "0.95",
            hint: "Equal-tailed: the same probability left in each tail.",
        },
        Widget::Check {
            id: "show_priors",
            label: "Show the priors used",
            default: false,
            hint: "Prints rstanarm's prior summary, with the scales it adjusted to the data.",
        },
        Widget::Text {
            id: "save",
            label: "Save the model as",
            default: TextDefault::Fixed("bayes"),
            rule: TextRule::Name,
            hint: "The fitted model stays in R's workspace under this name; a name already in use \
                   is replaced.",
        },
    ],
    needs: &["rstanarm"],
    styles: BASE,
    check: bayes_check,
    code: |a, _| {
        let m = a.name("save");
        let y = a.var("y").unwrap_or_default();
        let x = a
            .vars("x")
            .iter()
            .map(|n| r_name(n))
            .collect::<Vec<_>>()
            .join(" + ");
        let prior = if a.choice("prior") == "normal" {
            format!(
                ", prior = rstanarm::normal(location = {}, scale = {}, autoscale = FALSE)",
                a.number("prior_mean").trim(),
                a.number("prior_sd").trim()
            )
        } else {
            String::new()
        };
        let level = a.choice("level");
        let percent = match level {
            "0.9" => "90",
            "0.99" => "99",
            _ => "95",
        };
        let mut code = format!(
            "{m} <- rstanarm::stan_glm({y} ~ {x}, data = {}, family = stats::{}(){prior}, \
             chains = {}, iter = {}, seed = {}, cores = 1, refresh = 0)\n\
             cat(\"Posterior summary: {} chains of {} iterations (the first half warm-up), \
             {percent}% credible interval\\n\")\n\
             print(local({{\n  s <- unclass(summary({m}, probs = c((1 - {level}) / 2, \
             (1 + {level}) / 2)))\n  \
             round(s[!rownames(s) %in% c(\"mean_PPD\", \"log-posterior\"), , drop = FALSE], 4)\n}}))",
            a.data(),
            a.choice("family"),
            a.number("chains").trim(),
            a.number("iter").trim(),
            a.number("seed").trim(),
            a.number("chains").trim(),
            a.number("iter").trim(),
        );
        if a.flag("show_priors") {
            code.push_str(&format!("\nprint(rstanarm::prior_summary({m}))"));
        }
        code
    },
};

pub const SPECS: &[&Spec] = &[&BAYESIAN_REGRESSION];

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
                col("y", VarKind::Number),
                col("arm span", VarKind::Number),
                col("age", VarKind::Number),
                col("group", VarKind::Factor),
            ],
        }
    }

    fn values(set: &[(&str, &str)], x: &[&str]) -> Values {
        let mut v = BAYESIAN_REGRESSION.defaults(&ctx());
        v.set_text("y", "y");
        v.set_list("x", x.iter().map(|n| n.to_string()).collect());
        for (id, text) in set {
            v.set_text(id, *text);
        }
        v
    }

    fn widgets(v: &Values) -> Vec<&'static str> {
        BAYESIAN_REGRESSION
            .problems(&ctx(), v)
            .iter()
            .map(|p| p.widget)
            .collect()
    }

    #[test]
    fn writes_stan_glm() {
        let v = values(&[], &["arm span", "group"]);
        let c = BAYESIAN_REGRESSION
            .generate(&ctx(), &v, Style::Base)
            .unwrap_or_default();
        assert_eq!(
            c,
            "bayes <- rstanarm::stan_glm(y ~ `arm span` + group, data = d, family = stats::gaussian(), \
             chains = 4, iter = 2000, seed = 1, cores = 1, refresh = 0)\n\
             cat(\"Posterior summary: 4 chains of 2000 iterations (the first half warm-up), 95% credible interval\\n\")\n\
             print(local({\n  s <- unclass(summary(bayes, probs = c((1 - 0.95) / 2, (1 + 0.95) / 2)))\n  \
             round(s[!rownames(s) %in% c(\"mean_PPD\", \"log-posterior\"), , drop = FALSE], 4)\n}))"
        );
        let mut v = values(
            &[
                ("family", "poisson"),
                ("prior", "normal"),
                ("prior_mean", "0.5"),
                ("prior_sd", "3"),
                ("level", "0.9"),
                ("save", "post"),
            ],
            &["age"],
        );
        v.set_flag("show_priors", true);
        let c = BAYESIAN_REGRESSION
            .generate(&ctx(), &v, Style::Base)
            .unwrap_or_default();
        assert!(
            c.starts_with(
                "post <- rstanarm::stan_glm(y ~ age, data = d, family = stats::poisson(), \
                 prior = rstanarm::normal(location = 0.5, scale = 3, autoscale = FALSE), "
            ),
            "{c}"
        );
        assert!(c.contains("90% credible interval"), "{c}");
        assert!(c.ends_with("\nprint(rstanarm::prior_summary(post))"), "{c}");
    }

    #[test]
    fn refuses_what_cannot_run() {
        assert!(widgets(&values(&[], &["age"])).is_empty());
        assert_eq!(widgets(&values(&[], &["y"])), vec!["x"]);
        assert_eq!(
            widgets(&values(&[("prior", "normal"), ("prior_sd", "0")], &["age"])),
            vec!["prior_sd"]
        );
        assert_eq!(
            widgets(&values(&[("chains", "0")], &["age"])),
            vec!["chains"]
        );
        assert_eq!(widgets(&values(&[("iter", "10")], &["age"])), vec!["iter"]);
        assert_eq!(widgets(&values(&[("seed", "1.5")], &["age"])), vec!["seed"]);
        assert_eq!(widgets(&values(&[("save", "d")], &["age"])), vec!["save"]);
        // a typed prior is not checked while the default is chosen
        assert!(widgets(&values(&[("prior_sd", "0")], &["age"])).is_empty());
    }
}
