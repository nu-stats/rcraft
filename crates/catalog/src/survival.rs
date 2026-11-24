//! Statistics menu, survival analysis (P6.3), through the `survival` package
//! that comes with R: Kaplan-Meier survival curves (and their plot), the
//! log-rank test, the Cox proportional hazards model (with its test of
//! proportional hazards) and parametric survival models. Each takes a time and
//! an event variable (1 or TRUE when the event happened, 0 or FALSE when the
//! row was censored). The models are kept in the workspace under the name the
//! dialog asks for; none changes the data. The Cox dialog also takes start, stop
//! rows and splits follow-up at times; a competing-risks (Fine-Gray) model and
//! survival curves from a saved Cox model are dialogs of their own.

use rcraft_rcodegen::dialog::{
    expression_problem, Args, Opt, Problem, Spec, TextDefault, TextRule,
};
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
    let roles: Vec<(&str, &str)> = [
        "time", "event", "status", "entry", "group", "strata", "cluster",
    ]
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

/// The numbers typed in a box, separated by commas, each as typed; `Err`
/// says which entry is not a finite number.
fn number_list(text: &str) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    for part in text.split(',').map(str::trim).filter(|p| !p.is_empty()) {
        if part.parse::<f64>().is_ok_and(f64::is_finite) {
            out.push(part.to_string());
        } else {
            return Err(format!(
                "{part} is not a number; separate times with commas."
            ));
        }
    }
    Ok(out)
}

/// The split times typed in the Cox dialog.
fn cut_times(text: &str) -> Result<Vec<String>, String> {
    number_list(text)
}

/// The roles check, and the options of the Cox dialog that depend on each
/// other: split times are numbers, the interval-specific predictors are among
/// the predictors and need split times, and the cut rows' columns are free.
fn cox_check(a: &Args) -> Vec<Problem> {
    let mut out = roles_check(a);
    let cuts = cut_times(a.expr("split"));
    if let Err(message) = &cuts {
        out.push(Problem::new("split", message.clone()));
    }
    let split = cuts.is_ok_and(|c| !c.is_empty());
    if split {
        // without split times the list is not used, and is left alone
        for v in a.vars("varying") {
            if !a.vars("x").contains(&v) {
                out.push(Problem::new(
                    "varying",
                    format!("{v} is not among the predictors; add it there first."),
                ));
            }
        }
        for taken in ["interval", "tstart"] {
            if a.ctx.columns.iter().any(|c| c.name == taken)
                && (taken == "interval" || a.var_text("entry").is_empty())
            {
                out.push(Problem::new(
                    "split",
                    format!("The data already has a column called {taken}, which the cut rows need; rename it first."),
                ));
            }
        }
        let name = a.name_text("savesplit");
        if name == a.ctx.dataset || name == a.name_text("save") {
            out.push(Problem::new(
                "savesplit",
                format!("{name} is already in use here; choose another name."),
            ));
        }
    }
    out
}

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
            label: "Start time (start, stop input)",
            kinds: NUMBERS,
            required: false,
            hint: "Optional: when each row came under observation, for late entry or for data \
                   with one row per interval of a subject's follow-up (start, stop, event, and \
                   covariates that change between rows); Time is then when the row ended. \
                   This is Surv(start, stop, event).",
        },
        Widget::Text {
            id: "split",
            label: "Split follow-up at times",
            default: TextDefault::Empty,
            rule: TextRule::Any,
            hint: "Optional, for time-varying effects: times separated by commas, such as 10, 20. \
                   Each row is cut at them (survival::survSplit) into one row per interval, with \
                   the interval number in a column called interval, and the model is fitted to \
                   the cut rows. The estimates are the same unless an effect is allowed to \
                   change by interval (below). Stata's stsplit, at().",
        },
        Widget::Variables {
            id: "varying",
            label: "Effect differs by interval",
            kinds: NUMBERS,
            min: 0,
            hint: "Predictors, from the list above, that get a separate coefficient in each \
                   interval of the split times (and none overall): predictor:interval in R, \
                   Stata's c.x#i.interval. Needs split times.",
        },
        Widget::Text {
            id: "savesplit",
            label: "Save the cut rows as",
            default: TextDefault::Fixed("split_data"),
            rule: TextRule::Name,
            hint: "Used with split times: the data frame of cut rows stays in R's workspace \
                   under this name; a name already in use is replaced.",
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
            hint: "Tests, for each term and overall, whether its effect changes over time (scaled Schoenfeld residuals); a small p-value says the hazards may not be proportional.",
        },
        save("cox"),
    ],
    needs: &[],
    styles: BASE,
    check: cox_check,
    code: |a, _| {
        let model = a.name("save");
        let cuts = cut_times(a.expr("split")).unwrap_or_default();
        let split = !cuts.is_empty();
        let strata = a
            .var("strata")
            .map_or_else(String::new, |s| format!(" + survival::strata({s})"));
        let cluster = a
            .var("cluster")
            .map_or_else(String::new, |c| format!(", cluster = {c}"));
        let mut code = String::new();
        let (data, surv_fit, terms) = if split {
            let cut_data = a.name("savesplit");
            let start = match a.var_string("entry") {
                Some(entry) => entry,
                None => "\"tstart\"".to_string(),
            };
            code.push_str(&format!(
                "{cut_data} <- survival::survSplit(data = {}, cut = c({}), end = {}, event = {}, \
                 start = {start}, episode = \"interval\")\n\
                 {cut_data}$interval <- factor({cut_data}$interval)\n",
                a.data(),
                cuts.join(", "),
                a.var_string("time").unwrap_or_default(),
                a.var_string("event").unwrap_or_default(),
            ));
            let surv_fit = if a.var("entry").is_some() {
                surv(a)
            } else {
                format!(
                    "survival::Surv(tstart, {}, {})",
                    a.var("time").unwrap_or_default(),
                    a.var("event").unwrap_or_default()
                )
            };
            let varying = a.vars("varying");
            let terms = a
                .vars("x")
                .iter()
                .map(|n| {
                    if varying.contains(n) {
                        format!("{}:interval", r_name(n))
                    } else {
                        r_name(n)
                    }
                })
                .collect::<Vec<_>>()
                .join(" + ");
            (cut_data, surv_fit, terms)
        } else {
            (a.data(), surv(a), rhs(a))
        };
        code.push_str(&format!(
            "{model} <- survival::coxph({surv_fit} ~ {terms}{strata}, data = {data}, ties = {}{cluster})\n\
             print(summary({model}, conf.int = {}))",
            r_string(a.choice("ties")),
            a.choice("conf")
        ));
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

// ---- Competing-risks model ----

pub const COMPETING_RISKS: Spec = Spec {
    entry: "Competing-risks model",
    menu: "Statistics > Survival > Competing-risks model",
    about: "How the predictors change the chance of one kind of event when another kind can \
            happen first and prevent it (Fine and Gray's subdistribution hazard model). \
            The weighted rows are made as survival::finegray makes them (subjects with a \
            competing event stay in the risk set, with weights that fall as censoring would \
            have removed them) but with the censoring distribution taken just before each \
            time, as Stata's stcrreg does, so the subhazard ratios equal stcrreg's even with \
            tied times; survival::coxph then fits them with each subject as a cluster. The \
            exp(coef) column is the subhazard ratio. The rows are kept in the workspace as \
            the model's name followed by _data. Rows missing a value are left out first, as \
            in Stata. The robust standard errors are stcrreg's too: Fine and Gray's sandwich, \
            with the term for estimating the censoring distribution, times n / (n - 1); they \
            replace the plain cluster sandwich in the summary's robust se column, and equal \
            stcrreg's, tied times included.",
    widgets: &[
        TIME,
        Widget::Variable {
            id: "status",
            label: "Outcome",
            kinds: GROUPS,
            required: true,
            hint: "What ended each row's follow-up: 0 (or the first level) when it was \
                   censored, otherwise a code for the kind of event.",
        },
        Widget::Text {
            id: "cause",
            label: "Event of interest",
            default: TextDefault::Fixed("1"),
            rule: TextRule::Any,
            hint: "The code (or level) of the outcome to model; every other non-censored code \
                   is the competing event.",
        },
        PREDICTOR_LIST,
        Widget::Choice {
            id: "ties",
            label: "Tied event times",
            options: TIES,
            default: "efron",
            hint: "Breslow is Stata's.",
        },
        CONFIDENCE,
        save("fg"),
    ],
    needs: &[],
    styles: BASE,
    check: |a| {
        let mut out = roles_check(a);
        let cause = a.expr("cause");
        if cause.is_empty() {
            out.push(Problem::new(
                "cause",
                "Type the code of the event of interest.",
            ));
        } else if cause == a.var_text("status") {
            out.push(Problem::new("cause", "Type a code, not the column's name."));
        }
        out
    },
    code: |a, _| {
        let model = a.name("save");
        let rows = r_name(&format!("{}_data", a.name_text("save")));
        let data = a.data();
        let cols = [a.var_text("time"), a.var_text("status")]
            .into_iter()
            .chain(a.vars("x"))
            .map(r_string)
            .collect::<Vec<_>>()
            .join(", ");
        let xcols = a
            .vars("x")
            .into_iter()
            .map(r_string)
            .collect::<Vec<_>>()
            .join(", ");
        let build = SUBDISTRIBUTION_ROWS
            .replace("{rows}", &rows)
            .replace("{data}", &data)
            .replace("{cols}", &cols)
            .replace("{xcols}", &xcols)
            .replace("{time}", &r_string(a.var_text("time")))
            .replace("{status}", &r_string(a.var_text("status")))
            .replace("{cause}", &r_string(a.expr("cause")))
            .replace("{x}", &rhs(a));
        let variance = STCRREG_VARIANCE
            .replace("{model}", &model)
            .replace("{rows}", &rows);
        format!(
            "{build}\n\
             {model} <- survival::coxph(survival::Surv(fgstart, fgstop, fgstatus) ~ {x}, \
             data = {rows}, weights = fgwt, ties = {ties}, cluster = .row)\n\
             {variance}\n\
             cat(\"Competing-risks regression: exp(coef) is the subhazard ratio\\n\")\n\
             print(summary({model}, conf.int = {conf}))",
            x = rhs(a),
            ties = r_string(a.choice("ties")),
            conf = a.choice("conf"),
        )
    },
};

/// The rows of Fine and Gray's weighted risk sets, as survival::finegray makes
/// them (start, stop, status, weight, one block per subject) but with the
/// censoring distribution G taken just before each time, as Stata's stcrreg
/// does: a subject with the competing event stays in every later risk set,
/// from the pieces between censoring times, with weight G(t-) / G(own time-).
/// Rows missing a value are left out first.
const SUBDISTRIBUTION_ROWS: &str = "{rows} <- local({
  dd <- {data}[stats::complete.cases({data}[, c({cols})]), , drop = FALSE]
  tm <- dd[[{time}]]
  code <- as.character(dd[[{status}]])
  st <- ifelse(code == {cause}, 1L, ifelse(code == levels(factor(dd[[{status}]]))[1], 0L, 2L))
  cu <- sort(unique(tm[st == 0]))
  keep <- 1 - vapply(cu, function(u) sum(tm == u & st == 0) / sum(tm >= u), 0)
  g_before <- function(t) prod(keep[cu < t])
  g_upto <- function(t) prod(keep[cu <= t])
  last <- max(tm)
  pieces <- lapply(seq_along(tm), function(i) {
    own <- data.frame(fgstart = 0, fgstop = tm[i], fgstatus = as.integer(st[i] == 1), fgwt = 1, .row = i)
    if (st[i] != 2) return(own)
    from <- unique(c(tm[i], cu[cu > tm[i]]))
    from <- from[from < last]
    if (!length(from)) return(own)
    rbind(own, data.frame(fgstart = from, fgstop = c(from[-1], last), fgstatus = 0L,
                          fgwt = vapply(from, g_upto, 0) / g_before(tm[i]), .row = i))
  })
  out <- do.call(rbind, pieces)
  res <- cbind(dd[out$.row, c({xcols}), drop = FALSE], out)
  rownames(res) <- NULL
  attr(res, \"subjects\") <- list(tm = tm, st = st,
    X = stats::model.matrix(~ {x}, dd)[, -1, drop = FALSE])
  res
})";

/// Stata's robust covariance for the model fitted to those rows: Fine and
/// Gray's sandwich, with each subject's score residual plus the term for
/// estimating the censoring distribution, times n / (n - 1). It replaces the
/// plain cluster sandwich coxph gives, which leaves that term out.
const STCRREG_VARIANCE: &str = "if (!anyNA(stats::coef({model}))) {model}$var <- local({
  sj <- attr({rows}, \"subjects\")
  tm <- sj$tm; st <- sj$st; X <- sj$X
  n <- length(tm); b <- stats::coef({model}); e <- as.vector(exp(X %*% b))
  cu <- sort(unique(tm[st == 0]))
  nr <- vapply(cu, function(u) sum(tm >= u), 0)
  dc <- vapply(cu, function(u) sum(tm == u & st == 0), 0)
  g_before <- function(t) prod((1 - dc / nr)[cu < t])
  gt <- vapply(tm, g_before, 0)
  uk <- sort(unique(tm[st == 1])); K <- length(uk)
  dk <- vapply(uk, function(t) sum(tm == t & st == 1), 0)
  W <- matrix(vapply(uk, function(t) ifelse(tm >= t, 1, ifelse(st == 2, g_before(t) / gt, 0)), numeric(n)), n, K)
  s0 <- colSums(W * e)
  Ebar <- crossprod(W * e, X) / s0
  q <- dk / s0
  info <- Reduce(`+`, lapply(seq_len(K), function(k)
    dk[k] * (crossprod(X, W[, k] * e * X) / s0[k] - tcrossprod(Ebar[k, ]))))
  ev <- outer(seq_len(n), seq_len(K), function(i, k) as.numeric(tm[i] == uk[k] & st[i] == 1))
  eta <- X * as.numeric(st == 1) - ev %*% Ebar
  eta <- eta - e * (X * as.vector(W %*% q) - W %*% (Ebar * q))
  Wc <- ifelse(outer(tm, uk, '<') & st == 2, W, 0)
  Ck <- (crossprod(Wc * e, X) - Ebar * colSums(Wc * e)) * q
  Cj <- e * (X * as.vector(Wc %*% q) - Wc %*% (Ebar * q))
  own <- ifelse(st == 0, 1 / nr[match(tm, cu)], 0)
  jump <- function(tt) own * outer(tm, tt, '<=') -
    t(outer(cu, tm, '<=')) %*% ((dc / nr^2) * t(outer(tt, cu, '>=')))
  cmp <- which(st == 2)
  psi <- jump(uk) %*% Ck - jump(tm[cmp]) %*% Cj[cmp, , drop = FALSE]
  ii <- solve(info)
  v <- ii %*% crossprod(eta + psi) %*% ii * n / (n - 1)
  dimnames(v) <- dimnames({model}$var)
  v
})";

// ---- Survival curves from a Cox model ----

pub const COX_CURVES: Spec = Spec {
    entry: "Survival curves from a Cox model",
    menu: "Statistics > Survival > Survival curves from a Cox model",
    about: "The survival curve a fitted Cox model predicts for given covariate values, with \
            survival::survfit: the baseline survival (the product of one minus the baseline \
            hazard at each event time) raised to the model's hazard ratio for those values, \
            so the curve at time t is S0(t)^exp(xb), as Stata's stcurve, survival and \
            predict, basesurv give it. Without values the curve is for the mean of every \
            predictor (a categorical predictor's 0 and 1 column too, as Stata does). Give \
            several values (age = c(40, 60)) for one curve each. The survival at chosen times \
            is printed, and the curves drawn in the Graph tab. For a model fitted with Efron \
            ties (one stratum, no weights) the baseline is moved to Stata's Efron baseline, \
            and the confidence limits with it.",
    widgets: &[
        Widget::Text {
            id: "model",
            label: "Cox model",
            default: TextDefault::Fixed("cox"),
            rule: TextRule::Name,
            hint: "The name of a model in the workspace fitted by the Cox dialog (or coxph).",
        },
        Widget::Text {
            id: "values",
            label: "Covariate values",
            default: TextDefault::Empty,
            rule: TextRule::Any,
            hint: "Optional: the values to predict at, written as in data.frame, such as \
                   arm = \"b\", age = 50. Every predictor of the model needs one, and a \
                   stratified model also the strata. Leave empty for the means.",
        },
        Widget::Text {
            id: "times",
            label: "Show survival at times",
            default: TextDefault::Empty,
            rule: TextRule::Any,
            hint: "Optional: times separated by commas, such as 10, 20, 30. Survival, its \
                   standard error and its interval are printed at each.",
        },
        CONFIDENCE,
        Widget::Check {
            id: "draw",
            label: "Draw the curves",
            default: true,
            hint: "With one curve its confidence band is drawn too.",
        },
        save("curves"),
    ],
    needs: &[],
    styles: BASE,
    check: |a| {
        let mut out = Vec::new();
        if a.name_text("model").is_empty() {
            out.push(Problem::new("model", "Type the name of the Cox model."));
        }
        if let Some(message) = expression_problem(a.expr("values")) {
            out.push(Problem::new("values", message));
        }
        if let Err(message) = number_list(a.expr("times")) {
            out.push(Problem::new("times", message));
        }
        let save = a.name_text("save");
        if save == a.name_text("model") || save == a.ctx.dataset {
            out.push(Problem::new(
                "save",
                format!("{save} is already in use here; choose another name."),
            ));
        }
        out
    },
    code: |a, _| {
        let (model, curves) = (a.name("model"), a.name("save"));
        let values = a.expr("values");
        // without values, the curve is for the mean of every column of the
        // design matrix (a factor's 0 and 1 column too, as Stata does), where
        // survfit would set a factor to its first level
        let (fitted, newdata) = if values.is_empty() {
            (
                format!(
                    "local({{ m <- {model}; m$means <- colMeans(stats::model.matrix(m)); m }})"
                ),
                String::new(),
            )
        } else {
            (model.clone(), format!(", newdata = data.frame({values})"))
        };
        let mut code = format!(
            "stopifnot(\"this needs a Cox model fitted with coxph\" = inherits({model}, \"coxph\"))\n\
             {curves} <- survival::survfit({fitted}{newdata}, conf.int = {}, stype = 1)\n{}",
            a.choice("conf"),
            EFRON_BASELINE
                .replace("{model}", &model)
                .replace("{curves}", &curves)
        );
        if !values.is_empty() {
            code.push_str(&format!(
                "\ncat(\"Covariate values\\n\")\nprint(data.frame({values}))"
            ));
        }
        let times = number_list(a.expr("times")).unwrap_or_default();
        if !times.is_empty() {
            code.push_str(&format!(
                "\nprint(summary({curves}, times = c({})))",
                times.join(", ")
            ));
        }
        if a.flag("draw") {
            code.push_str(&format!(
                "\nk <- NCOL({curves}$surv)\n\
                 graphics::plot({curves}, conf.int = (k == 1), col = seq_len(k), lwd = 2, \
                 xlab = \"Time\", ylab = \"Survival probability\", \
                 main = \"Survival curves from a Cox model\")\n\
                 if (k > 1) graphics::legend(\"topright\", legend = paste(\"Row\", seq_len(k)), \
                 col = seq_len(k), lwd = 2, bty = \"n\")\nrm(k)"
            ));
        }
        code
    },
};

/// With Efron ties R's baseline survival is Breslow's product limit, and
/// Stata's `predict, basesurv` is Efron's: the events at one time are taken in
/// turn, each against the risk set less the share of the tied events already
/// counted, a factor (1 - r / (sum of r over the risk set - (l - 1) / m times
/// the sum over the tied events))^(1 / r). Every curve is S0(t)^exp(xb), so
/// each is moved to Stata's baseline by one power at each time, which leaves
/// untied data (and Breslow and exact ties) alone. Only models of a single
/// stratum without case weights.
const EFRON_BASELINE: &str = "if (identical({model}$method, 'efron') && NCOL({model}$y) == 2 &&
    is.null({model}$strata) && is.null({model}$weights) && !anyNA(stats::coef({model}))) {
  k <- local({
    tm <- {model}$y[, 1]; ev <- {model}$y[, 2] == 1
    r <- as.vector(exp(stats::model.matrix({model}) %*% stats::coef({model})))
    tt <- sort(unique(tm[ev]))
    logs <- vapply(tt, function(t) {
      D <- which(tm == t & ev); m <- length(D)
      sr <- sum(r[tm >= t]); sd <- sum(r[D])
      efron <- sum(log1p(-r[D] / (sr - (seq_len(m) - 1) / m * sd)) / r[D])
      breslow <- if (m == 1) efron else log(stats::uniroot(
        function(a) sum(r[D] / (1 - a^r[D])) - sr, c(1e-12, 1 - 1e-12), tol = 1e-14)$root)
      c(efron, breslow)
    }, numeric(2))
    c(1, cumsum(logs[1, ]) / cumsum(logs[2, ]))[findInterval({curves}$time, tt) + 1]
  })
  {curves}$surv <- exp(log({curves}$surv) * k)
  {curves}$lower <- exp(log({curves}$lower) * k)
  {curves}$upper <- exp(log({curves}$upper) * k)
  rm(k)
}";

pub const SPECS: &[&Spec] = &[
    &KAPLAN_MEIER,
    &LOG_RANK,
    &COX,
    &COX_CURVES,
    &COMPETING_RISKS,
    &PARAMETRIC,
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
                col("time", VarKind::Number),
                col("died", VarKind::Number),
                col("arm", VarKind::Factor),
                col("age", VarKind::Number),
                col("start", VarKind::Number),
                col("centre", VarKind::Text),
                col("patient id", VarKind::Number),
                col("cause", VarKind::Number),
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

    #[test]
    fn cox_splits_follow_up_at_the_times() {
        let mut v = values(
            &COX,
            &[
                ("time", "time"),
                ("event", "died"),
                ("split", "10, 20"),
                ("ties", "breslow"),
            ],
            &[("x", &["arm", "age"]), ("varying", &["age"])],
        );
        v.set_flag("zph", false);
        assert_eq!(
            code(&COX, &v),
            "split_data <- survival::survSplit(data = d, cut = c(10, 20), end = \"time\", \
             event = \"died\", start = \"tstart\", episode = \"interval\")\n\
             split_data$interval <- factor(split_data$interval)\n\
             cox <- survival::coxph(survival::Surv(tstart, time, died) ~ arm + age:interval, \
             data = split_data, ties = \"breslow\")\nprint(summary(cox, conf.int = 0.95))"
        );
        // with start times the cut rows keep them
        v.set_text("entry", "start");
        let with_start = code(&COX, &v);
        assert!(
            with_start.contains("start = \"start\", episode"),
            "{with_start}"
        );
        assert!(with_start.contains("survival::Surv(start, time, died) ~ arm + age:interval"));
        // a bad time, a separate effect for a non-predictor, and a taken name
        let mut bad = values(
            &COX,
            &[("time", "time"), ("event", "died"), ("split", "10, soon")],
            &[("x", &["arm"])],
        );
        let problems = COX.problems(&ctx(), &bad);
        assert!(problems.iter().any(|p| p.widget == "split"), "{problems:?}");
        bad.set_text("split", "10");
        bad.set_list("varying", vec!["age".into()]);
        let problems = COX.problems(&ctx(), &bad);
        assert!(
            problems.iter().any(|p| p.widget == "varying"),
            "{problems:?}"
        );
        bad.set_text("savesplit", "d");
        let problems = COX.problems(&ctx(), &bad);
        assert!(
            problems.iter().any(|p| p.widget == "savesplit"),
            "{problems:?}"
        );
    }

    #[test]
    fn competing_risks_expand_the_rows_then_fit_a_weighted_cox() {
        let v = values(
            &COMPETING_RISKS,
            &[("time", "time"), ("status", "cause"), ("ties", "breslow")],
            &[("x", &["arm", "age"])],
        );
        let text = code(&COMPETING_RISKS, &v);
        assert!(
            text.starts_with(
                "fg_data <- local({\n  dd <- d[stats::complete.cases(d[, c(\"time\", \"cause\", \"arm\", \"age\")]), , drop = FALSE]\n  \
                 tm <- dd[[\"time\"]]\n  code <- as.character(dd[[\"cause\"]])\n  \
                 st <- ifelse(code == \"1\", 1L,"
            ),
            "{text}"
        );
        assert!(text.contains("res <- cbind(dd[out$.row, c(\"arm\", \"age\"), drop = FALSE], out)"));
        assert!(text.contains(
            "fg <- survival::coxph(survival::Surv(fgstart, fgstop, fgstatus) ~ arm + age, \
             data = fg_data, weights = fgwt, ties = \"breslow\", cluster = .row)\n\
             if (!anyNA(stats::coef(fg))) fg$var <- local({"
        ));
        assert!(text.contains("model.matrix(~ arm + age, dd)"), "{text}");
        assert!(text.ends_with(
            "cat(\"Competing-risks regression: exp(coef) is the subhazard ratio\\n\")\n\
             print(summary(fg, conf.int = 0.95))"
        ));
        for placeholder in [
            "{rows}", "{data}", "{cols}", "{xcols}", "{time}", "{status}", "{cause}", "{model}",
            "{x}",
        ] {
            assert!(
                !text.contains(placeholder),
                "{placeholder} unfilled: {text}"
            );
        }
        let mut empty = v.clone();
        empty.set_text("cause", "");
        assert!(COMPETING_RISKS
            .problems(&ctx(), &empty)
            .iter()
            .any(|p| p.widget == "cause"));
    }

    #[test]
    fn cox_curves_predict_at_values_or_means() {
        let v = values(&COX_CURVES, &[], &[]);
        let text = code(&COX_CURVES, &v);
        // no values: the means of the design matrix's columns, then the Efron
        // baseline step, then the plot
        assert!(
            text.contains(
                "curves <- survival::survfit(local({ m <- cox; m$means <- colMeans(stats::model.matrix(m)); m }), \
                 conf.int = 0.95, stype = 1)\nif (identical(cox$method, 'efron')"
            ),
            "{text}"
        );
        assert!(
            text.contains("curves$surv <- exp(log(curves$surv) * k)"),
            "{text}"
        );
        assert!(
            text.ends_with(
                "main = \"Survival curves from a Cox model\")\n\
                 if (k > 1) graphics::legend(\"topright\", legend = paste(\"Row\", seq_len(k)), \
                 col = seq_len(k), lwd = 2, bty = \"n\")\nrm(k)"
            ),
            "{text}"
        );
        for placeholder in ["{model}", "{curves}"] {
            assert!(!text.contains(placeholder), "{placeholder} unfilled");
        }
        let mut v = values(
            &COX_CURVES,
            &[
                ("values", "arm = \"b\", age = c(40, 60)"),
                ("times", "10, 20"),
            ],
            &[],
        );
        v.set_flag("draw", false);
        let text = code(&COX_CURVES, &v);
        assert!(
            text.contains(
                "curves <- survival::survfit(cox, newdata = data.frame(arm = \"b\", age = c(40, 60)), \
                 conf.int = 0.95, stype = 1)\nif (identical(cox$method, 'efron')"
            ),
            "{text}"
        );
        assert!(text.ends_with(
            "cat(\"Covariate values\\n\")\nprint(data.frame(arm = \"b\", age = c(40, 60)))\n\
             print(summary(curves, times = c(10, 20)))"
        ));
        v.set_text("times", "10, soon");
        assert!(COX_CURVES
            .problems(&ctx(), &v)
            .iter()
            .any(|p| p.widget == "times"));
    }
}
