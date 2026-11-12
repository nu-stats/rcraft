//! Statistics menu, survival analysis (P6.3), through the `survival` package
//! that comes with R: Kaplan-Meier survival curves (and their plot), the
//! log-rank test, the Cox proportional hazards model (with its test of
//! proportional hazards) and parametric survival models. Each takes a time and
//! an event variable (1 or TRUE when the event happened, 0 or FALSE when the
//! row was censored). The models are kept in the workspace under the name the
//! dialog asks for; none changes the data.

use rcraft_rcodegen::dialog::{Args, Opt, Problem, Spec, TextDefault, TextRule};
use rcraft_rcodegen::dialog::{VarKind, Widget};
use rcraft_rcodegen::{r_name, r_string};

use crate::data::BASE;

const NUMBERS: &[VarKind] = &[VarKind::Number];
const EVENTS: &[VarKind] = &[VarKind::Number, VarKind::Logical];
const PREDICTORS: &[VarKind] = &[
    VarKind::Number,
    VarKind::Factor,
    VarKind::Text,
    VarKind::Logical,
];
const GROUPS: &[VarKind] = &[
    VarKind::Factor,
    VarKind::Text,
    VarKind::Logical,
    VarKind::Number,
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

const CONFIDENCE: Widget = Widget::Choice {
    id: "conf",
    label: "Confidence level",
    options: LEVELS,
    default: "0.95",
    hint: "",
};

const TIME: Widget = Widget::Variable {
    id: "time",
    label: "Time",
    kinds: NUMBERS,
    required: true,
    hint: "How long each row was followed: to the event, or to the end of follow-up.",
};

const EVENT: Widget = Widget::Variable {
    id: "event",
    label: "Event",
    kinds: EVENTS,
    required: true,
    hint: "1 (or TRUE) when the event happened, 0 (or FALSE) when the row was censored.",
};

const PREDICTOR_LIST: Widget = Widget::Variables {
    id: "x",
    label: "Predictors",
    kinds: PREDICTORS,
    min: 1,
    hint: "A categorical predictor gets one coefficient per level after the first.",
};

const fn save(default: &'static str) -> Widget {
    Widget::Text {
        id: "save",
        label: "Save the result as",
        default: TextDefault::Fixed(default),
        rule: TextRule::Name,
        hint: "It stays in R's workspace under this name; a name already in use is replaced.",
    }
}

/// `survival::Surv(time, event)`, or with an entry time `Surv(entry, time, event)`.
fn surv(a: &Args) -> String {
    let time = a.var("time").unwrap_or_default();
    let event = a.var("event").unwrap_or_default();
    match a.var("entry") {
        Some(entry) => format!("survival::Surv({entry}, {time}, {event})"),
        None => format!("survival::Surv({time}, {event})"),
    }
}

fn rhs(a: &Args) -> String {
    a.vars("x")
        .iter()
        .map(|n| r_name(n))
        .collect::<Vec<_>>()
        .join(" + ")
}

/// Each variable in one role only, and the result not saved over the data.
fn roles_check(a: &Args) -> Vec<Problem> {
    let mut out = Vec::new();
    let roles: Vec<(&str, &str)> = ["time", "event", "entry", "group", "strata", "cluster"]
        .into_iter()
        .map(|id| (id, a.var_text(id)))
        .filter(|(_, v)| !v.is_empty())
        .collect();
    for (k, (id, v)) in roles.iter().enumerate() {
        if let Some((first, _)) = roles.get(..k).and_then(|r| r.iter().find(|(_, w)| w == v)) {
            out.push(Problem::new(
                id,
                format!("{v} is already the {first} variable; choose another."),
            ));
        }
    }
    for (id, v) in &roles {
        if a.vars("x").contains(v) {
            out.push(Problem::new(
                "x",
                format!("{v} is the {id} variable; it cannot also be a predictor."),
            ));
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

// ---- Kaplan-Meier survival curves ----

pub const KAPLAN_MEIER: Spec = Spec {
    entry: "Kaplan-Meier survival curves",
    menu: "Statistics > Survival > Kaplan-Meier survival curves",
    about: "The share still without the event over time, estimated by the Kaplan-Meier method \
            with survival::survfit, for all rows or for each group: the number at risk, the \
            events and the median time with its confidence interval, and a plot of the curves \
            (censored rows marked) in the Graph tab. Its intervals are on the log-log scale, \
            as Stata's are, so they stay between 0 and 1.",
    widgets: &[
        TIME,
        EVENT,
        Widget::Variable {
            id: "group",
            label: "Groups",
            kinds: GROUPS,
            required: false,
            hint: "Optional: a curve for each group.",
        },
        CONFIDENCE,
        Widget::Check {
            id: "draw",
            label: "Draw the curves",
            default: true,
            hint: "With one curve its confidence band is drawn too.",
        },
        save("km"),
    ],
    needs: &[],
    styles: BASE,
    check: roles_check,
    code: |a, _| {
        let km = a.name("save");
        let group = a.var("group");
        let rhs = group.clone().unwrap_or_else(|| "1".into());
        let mut code = format!(
            "{km} <- survival::survfit({} ~ {rhs}, data = {}, conf.int = {}, \
             conf.type = \"log-log\")\nprint({km})",
            surv(a),
            a.data(),
            a.choice("conf")
        );
        if a.flag("draw") {
            let title = match a.var_text("group") {
                "" => "Kaplan-Meier survival curve".to_string(),
                g => format!("Kaplan-Meier survival curves by {g}"),
            };
            if group.is_some() {
                code.push_str(&format!(
                    "\ngraphics::plot({km}, col = seq_along({km}$strata), lwd = 2, mark.time = TRUE, \
                     xlab = {}, ylab = \"Survival probability\", main = {})\n\
                     graphics::legend(\"topright\", legend = names({km}$strata), \
                     col = seq_along({km}$strata), lwd = 2, bty = \"n\")",
                    r_string(a.var_text("time")),
                    r_string(&title)
                ));
            } else {
                code.push_str(&format!(
                    "\ngraphics::plot({km}, conf.int = TRUE, lwd = 2, mark.time = TRUE, \
                     xlab = {}, ylab = \"Survival probability\", main = {})",
                    r_string(a.var_text("time")),
                    r_string(&title)
                ));
            }
        }
        code
    },
};

// ---- Log-rank test ----

const LOG_RANK_TESTS: &[Opt] = &[
    Opt {
        value: "0",
        label: "Log-rank",
    },
    Opt {
        value: "1",
        label: "Peto-Peto (weighs early times more)",
    },
];

pub const LOG_RANK: Spec = Spec {
    entry: "Log-rank test",
    menu: "Statistics > Survival > Log-rank test",
    about: "Tests whether the groups' survival curves differ, comparing each group's events \
            with those expected if all shared one curve, with survival::survdiff.",
    widgets: &[
        TIME,
        EVENT,
        Widget::Variable {
            id: "group",
            label: "Groups",
            kinds: GROUPS,
            required: true,
            hint: "",
        },
        Widget::Choice {
            id: "rho",
            label: "Test",
            options: LOG_RANK_TESTS,
            default: "0",
            hint: "",
        },
    ],
    needs: &[],
    styles: BASE,
    check: roles_check,
    code: |a, _| {
        format!(
            "print(survival::survdiff({} ~ {}, data = {}, rho = {}))",
            surv(a),
            a.var("group").unwrap_or_default(),
            a.data(),
            a.choice("rho")
        )
    },
};

// ---- Cox model ----

const TIES: &[Opt] = &[
    Opt {
        value: "efron",
        label: "Efron",
    },
    Opt {
        value: "breslow",
        label: "Breslow",
    },
    Opt {
        value: "exact",
        label: "Exact",
    },
];

pub const COX: Spec = Spec {
    entry: "Cox proportional hazards model",
    menu: "Statistics > Survival > Cox proportional hazards model",
    about: "How the predictors raise or lower the hazard of the event, without assuming a shape \
            for it over time, with survival::coxph. The summary shows the hazard ratios with \
            their confidence intervals; the test of proportional hazards (survival::cox.zph) \
            checks the model's main assumption.",
    widgets: &[
        TIME,
        EVENT,
        PREDICTOR_LIST,
        Widget::Variable {
            id: "entry",
            label: "Entry time",
            kinds: NUMBERS,
            required: false,
            hint: "Optional: when each row came under observation, for late entry or rows that \
                   split one subject's time; Time is then when it left.",
        },
        Widget::Variable {
            id: "strata",
            label: "Strata",
            kinds: GROUPS,
            required: false,
            hint: "Optional: each stratum gets its own baseline hazard, with no coefficient.",
        },
        Widget::Variable {
            id: "cluster",
            label: "Cluster",
            kinds: GROUPS,
            required: false,
            hint: "Optional: robust standard errors for rows that are not independent (several \
                   rows per subject, say).",
        },
        Widget::Choice {
            id: "ties",
            label: "Tied event times",
            options: TIES,
            default: "efron",
            hint: "Efron is R's default; Breslow is Stata's.",
        },
        CONFIDENCE,
        Widget::Check {
            id: "zph",
            label: "Test proportional hazards",
            default: true,
            hint: "",
        },
        save("cox"),
    ],
    needs: &[],
    styles: BASE,
    check: roles_check,
    code: |a, _| {
        let model = a.name("save");
        let strata = a
            .var("strata")
            .map_or_else(String::new, |s| format!(" + survival::strata({s})"));
        let cluster = a
            .var("cluster")
            .map_or_else(String::new, |c| format!(", cluster = {c}"));
        let mut code = format!(
            "{model} <- survival::coxph({} ~ {}{strata}, data = {}, ties = {}{cluster})\n\
             print(summary({model}, conf.int = {}))",
            surv(a),
            rhs(a),
            a.data(),
            r_string(a.choice("ties")),
            a.choice("conf")
        );
        if a.flag("zph") {
            code.push_str(&format!("\nprint(survival::cox.zph({model}))"));
        }
        code
    },
};

// ---- Parametric survival model ----

const DISTRIBUTIONS: &[Opt] = &[
    Opt {
        value: "weibull",
        label: "Weibull",
    },
    Opt {
        value: "exponential",
        label: "Exponential",
    },
    Opt {
        value: "lognormal",
        label: "Log-normal",
    },
    Opt {
        value: "loglogistic",
        label: "Log-logistic",
    },
];

pub const PARAMETRIC: Spec = Spec {
    entry: "Parametric survival model",
    menu: "Statistics > Survival > Parametric survival model",
    about: "A survival model with a chosen distribution for the times, written as accelerated \
            failure time (each coefficient stretches or shrinks the time to the event), with \
            survival::survreg. The time ratios are the exponentiated coefficients.",
    widgets: &[
        TIME,
        EVENT,
        PREDICTOR_LIST,
        Widget::Choice {
            id: "dist",
            label: "Distribution",
            options: DISTRIBUTIONS,
            default: "weibull",
            hint: "",
        },
        Widget::Check {
            id: "ratios",
            label: "Show time ratios with confidence intervals",
            default: true,
            hint: "",
        },
        CONFIDENCE,
        save("aft"),
    ],
    needs: &[],
    styles: BASE,
    check: roles_check,
    code: |a, _| {
        let model = a.name("save");
        let mut code = format!(
            "{model} <- survival::survreg({} ~ {}, data = {}, dist = {})\nprint(summary({model}))",
            surv(a),
            rhs(a),
            a.data(),
            r_string(a.choice("dist"))
        );
        if a.flag("ratios") {
            code.push_str(&format!(
                "\ncat(\"\\nTime ratios\\n\")\nprint(exp(cbind(time_ratio = stats::coef({model}), \
                 stats::confint({model}, level = {}))))",
                a.choice("conf")
            ));
        }
        code
    },
};

pub const SPECS: &[&Spec] = &[&KAPLAN_MEIER, &LOG_RANK, &COX, &PARAMETRIC];

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
                col("time", VarKind::Number),
                col("died", VarKind::Number),
                col("arm", VarKind::Factor),
                col("age", VarKind::Number),
                col("start", VarKind::Number),
                col("centre", VarKind::Text),
                col("patient id", VarKind::Number),
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

    #[test]
    fn kaplan_meier_fits_and_draws() {
        let v = values(&KAPLAN_MEIER, &[("time", "time"), ("event", "died")], &[]);
        assert_eq!(
            code(&KAPLAN_MEIER, &v),
            "km <- survival::survfit(survival::Surv(time, died) ~ 1, data = d, conf.int = 0.95, \
             conf.type = \"log-log\")\n\
             print(km)\n\
             graphics::plot(km, conf.int = TRUE, lwd = 2, mark.time = TRUE, xlab = \"time\", \
             ylab = \"Survival probability\", main = \"Kaplan-Meier survival curve\")"
        );
        let mut v = values(
            &KAPLAN_MEIER,
            &[("time", "time"), ("event", "died"), ("group", "arm")],
            &[],
        );
        assert!(code(&KAPLAN_MEIER, &v).contains(
            "graphics::legend(\"topright\", legend = names(km$strata), col = seq_along(km$strata)"
        ));
        v.set_flag("draw", false);
        assert_eq!(
            code(&KAPLAN_MEIER, &v),
            "km <- survival::survfit(survival::Surv(time, died) ~ arm, data = d, conf.int = 0.95, \
             conf.type = \"log-log\")\n\
             print(km)"
        );
    }

    #[test]
    fn log_rank_writes_survdiff() {
        let v = values(
            &LOG_RANK,
            &[
                ("time", "time"),
                ("event", "died"),
                ("group", "arm"),
                ("rho", "1"),
            ],
            &[],
        );
        assert_eq!(
            code(&LOG_RANK, &v),
            "print(survival::survdiff(survival::Surv(time, died) ~ arm, data = d, rho = 1))"
        );
    }

    #[test]
    fn cox_writes_coxph_with_its_options() {
        let v = values(
            &COX,
            &[("time", "time"), ("event", "died")],
            &[("x", &["arm", "age"])],
        );
        assert_eq!(
            code(&COX, &v),
            "cox <- survival::coxph(survival::Surv(time, died) ~ arm + age, data = d, ties = \"efron\")\n\
             print(summary(cox, conf.int = 0.95))\nprint(survival::cox.zph(cox))"
        );
        let mut v = values(
            &COX,
            &[
                ("time", "time"),
                ("event", "died"),
                ("entry", "start"),
                ("strata", "centre"),
                ("cluster", "patient id"),
                ("ties", "breslow"),
                ("conf", "0.9"),
            ],
            &[("x", &["age"])],
        );
        v.set_flag("zph", false);
        assert_eq!(
            code(&COX, &v),
            "cox <- survival::coxph(survival::Surv(start, time, died) ~ age + survival::strata(centre), \
             data = d, ties = \"breslow\", cluster = `patient id`)\nprint(summary(cox, conf.int = 0.9))"
        );
    }

    #[test]
    fn each_variable_has_one_role() {
        let v = values(
            &COX,
            &[("time", "time"), ("event", "time"), ("strata", "age")],
            &[("x", &["age"])],
        );
        let problems = COX.problems(&ctx(), &v);
        let widgets: Vec<&str> = problems.iter().map(|p| p.widget).collect();
        assert!(widgets.contains(&"event"), "{problems:?}");
        assert!(widgets.contains(&"x"), "{problems:?}");
    }

    #[test]
    fn parametric_writes_survreg_and_time_ratios() {
        let v = values(
            &PARAMETRIC,
            &[("time", "time"), ("event", "died"), ("dist", "lognormal")],
            &[("x", &["arm"])],
        );
        assert_eq!(
            code(&PARAMETRIC, &v),
            "aft <- survival::survreg(survival::Surv(time, died) ~ arm, data = d, dist = \"lognormal\")\n\
             print(summary(aft))\ncat(\"\\nTime ratios\\n\")\n\
             print(exp(cbind(time_ratio = stats::coef(aft), stats::confint(aft, level = 0.95))))"
        );
    }
}
