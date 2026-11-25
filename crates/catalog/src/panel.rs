//! Statistics menu, panel and fixed-effects models (P6.1): fixed-effects
//! regression and count or binary models with any number of absorbed fixed
//! effects through `fixest`, the classic panel estimators (within, random,
//! between, pooled, first differences) through `plm`, the Hausman test
//! between fixed and random effects, the Arellano-Bond dynamic panel model
//! (`plm::pgmm`), panel instrumental variables (`plm::plm` with instruments),
//! the Levin-Lin-Chu and Im-Pesaran-Shin panel unit-root tests
//! (`plm::purtest`) and Wooldridge's test for serial correlation in base R.
//! The models keep their fit in the workspace under the name the dialog asks
//! for (the tests keep none) and print the package's own summary. None changes
//! the data.

use rcraft_rcodegen::dialog::{Args, Opt, Problem, Spec, TextDefault, TextRule};
use rcraft_rcodegen::dialog::{VarKind, Widget};
use rcraft_rcodegen::{r_name, r_string};

use crate::data::BASE;

const NUMBERS: &[VarKind] = &[VarKind::Number];
const PREDICTORS: &[VarKind] = &[
    VarKind::Number,
    VarKind::Factor,
    VarKind::Text,
    VarKind::Logical,
];
/// What can name a group (a panel, a year, a firm).
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
    hint: "For the confidence intervals printed after the summary.",
};

const PREDICTOR_LIST: Widget = Widget::Variables {
    id: "x",
    label: "Predictors",
    kinds: PREDICTORS,
    min: 1,
    hint: "A variable that does not change within a fixed-effect group is absorbed by it and \
           dropped.",
};

const FIXED_EFFECTS: Widget = Widget::Variables {
    id: "fe",
    label: "Fixed effects",
    kinds: GROUPS,
    min: 1,
    hint: "Each is a grouping (a person, a firm, a year): every group gets its own intercept, \
           which is absorbed rather than shown.",
};

const SAVE: Widget = Widget::Text {
    id: "save",
    label: "Save the model as",
    default: TextDefault::Fixed("model"),
    rule: TextRule::Name,
    hint: "The fitted model stays in R's workspace under this name; a name already in use is \
           replaced.",
};

const ERRORS: &[Opt] = &[
    Opt {
        value: "cluster",
        label: "Clustered by the first fixed effect",
    },
    Opt {
        value: "iid",
        label: "Conventional",
    },
    Opt {
        value: "hetero",
        label: "Robust (the spread of the errors may vary)",
    },
    Opt {
        value: "twoway",
        label: "Clustered by the first two fixed effects",
    },
];

const ERROR_CHOICE: Widget = Widget::Choice {
    id: "errors",
    label: "Standard errors",
    options: ERRORS,
    default: "cluster",
    hint: "The first choice clusters by the first fixed effect. A variable under Cluster by \
           takes the place of this choice.",
};

const CLUSTER: Widget = Widget::Variable {
    id: "cluster",
    label: "Cluster by",
    kinds: GROUPS,
    required: false,
    hint: "Optional: cluster the standard errors by this variable instead.",
};

/// The predictors, joined for a formula.
fn rhs(a: &Args) -> String {
    a.vars("x")
        .iter()
        .map(|n| r_name(n))
        .collect::<Vec<_>>()
        .join(" + ")
}

/// The fixed effects, joined for the part after `|`.
fn fe_part(a: &Args) -> String {
    a.vars("fe")
        .iter()
        .map(|n| r_name(n))
        .collect::<Vec<_>>()
        .join(" + ")
}

/// The outcome is neither a predictor nor a fixed effect, a predictor is not
/// also a fixed effect, and the model is not saved over the data frame.
fn fixest_check(a: &Args) -> Vec<Problem> {
    let y = a.var_text("y");
    let x = a.vars("x");
    let fe = a.vars("fe");
    let mut out = Vec::new();
    if x.contains(&y) {
        out.push(Problem::new(
            "x",
            format!("{y} is the outcome; it cannot also be a predictor."),
        ));
    }
    if fe.contains(&y) {
        out.push(Problem::new(
            "fe",
            format!("{y} is the outcome; it cannot also be a fixed effect."),
        ));
    }
    if let Some(both) = x.iter().find(|v| fe.contains(v)) {
        out.push(Problem::new(
            "fe",
            format!("{both} is a predictor; it cannot also be a fixed effect."),
        ));
    }
    if a.choice("errors") == "twoway" && fe.len() < 2 && a.var("cluster").is_none() {
        out.push(Problem::new(
            "errors",
            "Two-way clustering needs two fixed effects.",
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

/// fixest's `vcov =` argument: a cluster variable, or the choice.
fn vcov_arg(a: &Args) -> String {
    match a.var("cluster") {
        Some(c) => format!("cluster = ~{c}"),
        None => format!("vcov = {}", r_string(a.choice("errors"))),
    }
}

// ---- Fixed-effects regression ----

pub const FIXED_EFFECTS_REGRESSION: Spec = Spec {
    entry: "Fixed-effects regression",
    menu: "Statistics > Panel models > Fixed-effects regression",
    about: "A linear model with one or more fixed effects (each person, firm or year its own \
            intercept), absorbed rather than estimated one by one, so it stays fast with \
            thousands of groups. It fits with fixest::feols and prints its summary and \
            confidence intervals.",
    widgets: &[
        Widget::Variable {
            id: "y",
            label: "Outcome",
            kinds: NUMBERS,
            required: true,
            hint: "",
        },
        PREDICTOR_LIST,
        FIXED_EFFECTS,
        ERROR_CHOICE,
        CLUSTER,
        CONFIDENCE,
        SAVE,
    ],
    needs: &["fixest"],
    styles: BASE,
    check: fixest_check,
    code: |a, _| {
        let model = a.name("save");
        format!(
            "{model} <- fixest::feols({} ~ {} | {}, data = {}, {})\nprint(summary({model}))\n\
             print(stats::confint({model}, level = {}))",
            a.var("y").unwrap_or_default(),
            rhs(a),
            fe_part(a),
            a.data(),
            vcov_arg(a),
            a.choice("conf")
        )
    },
};

// ---- Fixed-effects count or binary model ----

const FAMILIES: &[Opt] = &[
    Opt {
        value: "poisson",
        label: "Poisson (counts)",
    },
    Opt {
        value: "logit",
        label: "Logit (0 or 1)",
    },
];

fn glm_check(a: &Args) -> Vec<Problem> {
    let mut out = fixest_check(a);
    let exposure = a.var_text("exposure");
    if !exposure.is_empty() {
        if a.choice("family") != "poisson" {
            out.push(Problem::new(
                "exposure",
                "An exposure is for the Poisson model only; leave it empty for logit.",
            ));
        } else if exposure == a.var_text("y") {
            out.push(Problem::new(
                "exposure",
                "The exposure cannot be the outcome.",
            ));
        }
    }
    out
}

pub const FIXED_EFFECTS_GLM: Spec = Spec {
    entry: "Fixed-effects count or binary model",
    menu: "Statistics > Panel models > Fixed-effects count or binary model",
    about: "A Poisson model of counts, or a logit model of a 0/1 outcome, with one or more \
            fixed effects absorbed. It fits with fixest::fepois or fixest::feglm. Groups whose \
            outcome never varies (all zero counts, all 0 or all 1) tell nothing about the \
            coefficients and are dropped, with a note. The standard errors follow Stata's \
            conventions for these models (and glm's): no degrees-of-freedom adjustment, and \
            G/(G - 1) for G clusters.",
    widgets: &[
        Widget::Variable {
            id: "y",
            label: "Outcome",
            kinds: NUMBERS,
            required: true,
            hint: "Counts of zero or more for Poisson; 0 and 1 for logit.",
        },
        Widget::Choice {
            id: "family",
            label: "Model",
            options: FAMILIES,
            default: "poisson",
            hint: "",
        },
        PREDICTOR_LIST,
        FIXED_EFFECTS,
        Widget::Variable {
            id: "exposure",
            label: "Exposure (Poisson only)",
            kinds: NUMBERS,
            required: false,
            hint: "Optional: how long or how much each row was observed. The model then \
                   describes rates; its log is added as an offset. Must be above zero.",
        },
        Widget::Check {
            id: "ratios",
            label: "Show rate or odds ratios with confidence intervals",
            default: true,
            hint: "",
        },
        ERROR_CHOICE,
        CLUSTER,
        CONFIDENCE,
        SAVE,
    ],
    needs: &["fixest"],
    styles: BASE,
    check: glm_check,
    code: |a, _| {
        let model = a.name("save");
        let poisson = a.choice("family") == "poisson";
        let offset = match (poisson, a.var("exposure")) {
            (true, Some(e)) => format!(", offset = ~log({e})"),
            _ => String::new(),
        };
        let call = if poisson {
            "fixest::fepois("
        } else {
            "fixest::feglm("
        };
        let family = if poisson { "" } else { ", family = \"logit\"" };
        let mut code = format!(
            "{model} <- {call}{} ~ {} | {}, data = {}{family}{offset}, {}, \
             ssc = fixest::ssc(adj = FALSE))\nprint(summary({model}))",
            a.var("y").unwrap_or_default(),
            rhs(a),
            fe_part(a),
            a.data(),
            vcov_arg(a),
        );
        if a.flag("ratios") {
            let (title, column) = if poisson {
                ("Rate ratios", "rate_ratio")
            } else {
                ("Odds ratios", "odds_ratio")
            };
            code.push_str(&format!(
                "\ncat(\"\\n{title}\\n\")\nprint(exp(cbind({column} = stats::coef({model}), \
                 stats::confint({model}, level = {}))))",
                a.choice("conf")
            ));
        } else {
            code.push_str(&format!(
                "\nprint(stats::confint({model}, level = {}))",
                a.choice("conf")
            ));
        }
        code
    },
};

// ---- Panel model (plm) ----

const PANEL_MODELS: &[Opt] = &[
    Opt {
        value: "within",
        label: "Fixed effects (within)",
    },
    Opt {
        value: "random",
        label: "Random effects",
    },
    Opt {
        value: "between",
        label: "Between (group means)",
    },
    Opt {
        value: "pooling",
        label: "Pooled (ordinary least squares)",
    },
    Opt {
        value: "fd",
        label: "First differences",
    },
];

const EFFECTS: &[Opt] = &[
    Opt {
        value: "individual",
        label: "Panel units",
    },
    Opt {
        value: "time",
        label: "Time periods",
    },
    Opt {
        value: "twoways",
        label: "Both",
    },
];

const PANEL_ERRORS: &[Opt] = &[
    Opt {
        value: "conventional",
        label: "Conventional",
    },
    Opt {
        value: "group",
        label: "Clustered by panel unit",
    },
    Opt {
        value: "time",
        label: "Clustered by time period",
    },
];

const PANEL_ID: Widget = Widget::Variable {
    id: "id",
    label: "Panel unit",
    kinds: GROUPS,
    required: true,
    hint: "What the rows are observations of, followed over time: a person, firm or country.",
};

const PANEL_TIME: Widget = Widget::Variable {
    id: "time",
    label: "Time",
    kinds: GROUPS,
    required: true,
    hint: "The period each row belongs to. Each unit may have each period once.",
};

/// The outcome, predictors, unit and time are four different variables.
fn plm_check(a: &Args) -> Vec<Problem> {
    let y = a.var_text("y");
    let id = a.var_text("id");
    let time = a.var_text("time");
    let x = a.vars("x");
    let mut out = Vec::new();
    if x.contains(&y) {
        out.push(Problem::new(
            "x",
            format!("{y} is the outcome; it cannot also be a predictor."),
        ));
    }
    for (w, v, what) in [("id", id, "panel unit"), ("time", time, "time")] {
        if v == y || x.contains(&v) {
            out.push(Problem::new(
                w,
                format!("{v} is in the model; choose another variable for the {what}."),
            ));
        }
    }
    if !id.is_empty() && id == time {
        out.push(Problem::new(
            "time",
            "The panel unit and the time must be different variables.",
        ));
    }
    out
}

fn panel_check(a: &Args) -> Vec<Problem> {
    let mut out = plm_check(a);
    let model = a.choice("model");
    if a.choice("effect") == "twoways" && model == "between" {
        out.push(Problem::new(
            "effect",
            "Between models take panel-unit or time effects, not both.",
        ));
    }
    if model == "fd" {
        let categorical = a.vars("x").into_iter().find(|v| {
            a.ctx
                .column(v)
                .is_some_and(|c| matches!(c.kind, VarKind::Factor | VarKind::Text))
        });
        if let Some(v) = categorical {
            out.push(Problem::new(
                "x",
                format!(
                    "{v} is categorical; first differences need numbers. Make a 0/1 variable for \
                     each level you need first."
                ),
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

/// `index = c("id", "time")` for plm.
fn index_arg(a: &Args) -> String {
    format!(
        "index = c({}, {})",
        a.var_string("id").unwrap_or_default(),
        a.var_string("time").unwrap_or_default()
    )
}

pub const PANEL_MODEL: Spec = Spec {
    entry: "Panel model",
    menu: "Statistics > Panel models > Panel model",
    about: "The classic panel estimators of a linear model: fixed effects (within), random \
            effects, between, pooled and first differences, with effects for the panel units, \
            the time periods or both. It fits with plm::plm and prints its summary.",
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
            hint: "With fixed effects, a variable that does not change within a unit is dropped.",
        },
        PANEL_ID,
        PANEL_TIME,
        Widget::Choice {
            id: "model",
            label: "Estimator",
            options: PANEL_MODELS,
            default: "within",
            hint: "",
        },
        Widget::Choice {
            id: "effect",
            label: "Effects for",
            options: EFFECTS,
            default: "individual",
            hint: "Pooled and first-difference models have no effects, so this is ignored for \
                   them.",
        },
        Widget::Choice {
            id: "errors",
            label: "Standard errors",
            options: PANEL_ERRORS,
            default: "conventional",
            hint: "Clustered errors are plm::vcovHC's (Arellano's) with Stata's small-sample \
                   correction, shown in the summary.",
        },
        SAVE,
    ],
    needs: &["plm"],
    styles: BASE,
    check: panel_check,
    code: |a, _| {
        let model = a.name("save");
        let estimator = a.choice("model");
        let y = a.var("y").unwrap_or_default();
        // plm's own model = "fd" differences row by row (its NEWS says so), so a
        // missing value or year inside a unit is differenced across; diff() of a
        // pseries goes by the time index, as Stata's D. does, and the pooled fit
        // of the differences is then Stata's regress D.y D.x.
        let (formula, estimator) = if estimator == "fd" {
            let x = a
                .vars("x")
                .iter()
                .map(|n| format!("diff({})", r_name(n)))
                .collect::<Vec<_>>()
                .join(" + ");
            (format!("diff({y}) ~ {x}"), "pooling")
        } else {
            (format!("{y} ~ {}", rhs(a)), estimator)
        };
        let effect = if estimator == "pooling" {
            String::new()
        } else {
            format!(", effect = {}", r_string(a.choice("effect")))
        };
        let summary = match a.choice("errors") {
            "conventional" => format!("summary({model})"),
            cluster => format!(
                "summary({model}, vcov = plm::vcovHC({model}, cluster = {}, type = \"sss\"))",
                r_string(cluster)
            ),
        };
        format!(
            "{model} <- plm::plm({formula}, data = {}, {}, model = {}{effect})\nprint({summary})",
            a.data(),
            index_arg(a),
            r_string(estimator),
        )
    },
};

// ---- Hausman test ----

pub const HAUSMAN: Spec = Spec {
    entry: "Hausman test (fixed or random effects)",
    menu: "Statistics > Panel models > Hausman test (fixed or random effects)",
    about: "Tests whether random effects can stand in for fixed effects: it fits the model both \
            ways with plm::plm and compares the coefficients with plm::phtest. A small p-value \
            says the two differ, so the fixed-effects estimates are the ones to use.",
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
            hint: "Only predictors that change within a unit can be compared.",
        },
        PANEL_ID,
        PANEL_TIME,
    ],
    needs: &["plm"],
    styles: BASE,
    check: plm_check,
    code: |a, _| {
        let formula = format!("{} ~ {}", a.var("y").unwrap_or_default(), rhs(a));
        let index = index_arg(a);
        let d = a.data();
        format!(
            "print(local({{\n  fixed <- plm::plm({formula}, data = {d}, {index}, model = \"within\")\n  \
             random <- plm::plm({formula}, data = {d}, {index}, model = \"random\")\n  \
             plm::phtest(fixed, random)\n}}))"
        )
    },
};

// ---- Dynamic panel model (Arellano-Bond) ----

const STEPS: &[Opt] = &[
    Opt {
        value: "twosteps",
        label: "Two steps",
    },
    Opt {
        value: "onestep",
        label: "One step",
    },
];

const DYNAMIC_ERRORS: &[Opt] = &[
    Opt {
        value: "robust",
        label: "Robust (Windmeijer's correction after two steps)",
    },
    Opt {
        value: "conventional",
        label: "Conventional",
    },
];

const DYNAMIC_METHODS: &[Opt] = &[
    Opt {
        value: "difference",
        label: "Difference GMM (Arellano-Bond)",
    },
    Opt {
        value: "system",
        label: "System GMM (Blundell-Bond)",
    },
];

/// Blundell-Bond system GMM written out, as Stata's xtdpdsys has it (without a constant).
/// Each unit stacks the equation in differences (periods lags+2 to T; the outcome's earlier
/// levels from the first period to two back as instruments, a block for each period, and the
/// differences of the other predictors as their own) and the equation in levels (periods
/// max(3, lags+1) to T; the lagged difference of the outcome as the instrument for each
/// period). The first-step weighting matrix is block diagonal: the first-difference matrix
/// (2 on the diagonal, -1 beside it) for the difference equation, the identity for levels.
/// Two steps weight by the inverse of the first step's squared scores; the robust errors
/// after two steps are Windmeijer's. After one step the conventional errors and Sargan's
/// statistic divide by sigma^2, the difference equation's residual sum of squares over
/// twice its observations less the coefficients.
const SYSTEM_GMM: &str = "MODEL <- local({
  d <- @DATA@
  id <- @ID@
  time <- @TIME@
  y <- @Y@
  x <- @X@
  lags <- @LAGS@
  limit <- @LIMIT@
  steps <- @STEPS@
  robust <- @ROBUST@
  cons <- @CONS@
  times <- @TIMES@
  d <- d[order(d[[id]], d[[time]]), ]
  units <- split(d, d[[id]])
  # every unit is laid out on the whole grid of periods; what it does not have is a row of
  # zeros, which contributes nothing, so a unit may miss periods
  stamps <- sort(unique(d[[time]]))
  periods <- length(stamps)
  stopifnot('system GMM needs the periods one step apart: no missing step in the time variable' =
    length(stamps) > 1 && all(diff(stamps) == diff(stamps)[1]) && diff(stamps)[1] > 0,
    'a unit is in the data twice for one period' =
      !any(vapply(units, function(u) anyDuplicated(u[[time]]) > 0, TRUE)),
    'too few periods for this many lags' = periods >= lags + 3)
  ts <- (lags + 2):periods
  ls <- max(if (cons) 2 else 3, lags + 1):periods
  nd <- length(ts)
  nl <- length(ls)
  # the level equation's instrument is the lagged difference, so a period-2 row carries
  # only the constant: Stata's xtdpdsys keeps that row when there is one
  instrumented <- ls >= 3
  # time effects are a dummy for each period from the third on, among the exogenous
  # predictors: they are differenced in the one equation and kept in the other, and
  # instrument themselves, which is what year dummies do in Stata's xtdpdsys
  dummies <- matrix(0, periods, 0)
  if (times && periods > 2) {
    dummies <- outer(stamps, stamps[-(1:2)], `==`) * 1
    colnames(dummies) <- paste(time, stamps[-(1:2)])
  }
  stack <- function(u) {
    at <- match(stamps, u[[time]])
    v <- u[[y]][at]
    xs <- cbind(as.matrix(u[x])[at, , drop = FALSE], dummies)
    dv <- c(NA, diff(v))
    dx <- if (ncol(xs)) rbind(NA, apply(xs, 2, diff)) else matrix(0, periods, 0)
    gmm <- matrix(0, nd, sum(pmin(ts - 2, limit)))
    off <- 0
    for (r in seq_len(nd)) {
      keep <- seq(max(1, ts[r] - 1 - limit), ts[r] - 2)
      levels <- v[keep]
      levels[is.na(levels)] <- 0
      gmm[r, off + seq_along(keep)] <- levels
      off <- off + length(keep)
    }
    zd <- cbind(gmm, dx[ts, , drop = FALSE])
    xd <- cbind(vapply(seq_len(lags), function(j) dv[ts - j], numeric(nd)), dx[ts, , drop = FALSE],
      if (cons) 0)
    xl <- cbind(vapply(seq_len(lags), function(j) v[ls - j], numeric(nl)), xs[ls, , drop = FALSE],
      if (cons) 1)
    zl <- matrix(0, nl, sum(instrumented) + cons)
    zl[cbind(which(instrumented), seq_len(sum(instrumented)))] <- dv[ls[instrumented] - 1]
    if (cons) zl[, ncol(zl)] <- 1
    z <- matrix(0, nd + nl, ncol(zd) + ncol(zl))
    z[seq_len(nd), seq_len(ncol(zd))] <- zd
    z[nd + seq_len(nl), ncol(zd) + seq_len(ncol(zl))] <- zl
    # a row the unit cannot fill, because it misses a period or a predictor, is set to zero
    # and so adds nothing to any of the sums
    xx <- rbind(xd, xl)
    yy <- c(dv[ts], v[ls])
    empty <- is.na(yy) | apply(is.na(xx), 1, any)
    z[empty, ] <- 0
    xx[empty, ] <- 0
    yy[empty] <- 0
    z[is.na(z)] <- 0
    xx[is.na(xx)] <- 0
    list(z = z, x = xx, y = yy, filled = !empty)
  }
  rows <- lapply(units, stack)
  filled <- sum(vapply(rows, function(r) sum(r$filled[seq_len(nd)]), 1))
  stopifnot('no unit has enough periods in a row for this model' = filled > 0)
  tri <- diag(2, nd)
  if (nd > 1) for (i in seq_len(nd - 1)) tri[i, i + 1] <- tri[i + 1, i] <- -1
  h <- matrix(0, nd + nl, nd + nl)
  h[seq_len(nd), seq_len(nd)] <- tri
  h[nd + seq_len(nl), nd + seq_len(nl)] <- diag(nl)
  total <- function(f) Reduce(`+`, lapply(rows, f))
  zx <- total(function(r) crossprod(r$z, r$x))
  zy <- total(function(r) crossprod(r$z, r$y))
  one <- solve(total(function(r) t(r$z) %*% h %*% r$z))
  bread1 <- solve(t(zx) %*% one %*% zx)
  b1 <- bread1 %*% t(zx) %*% one %*% zy
  resid <- function(b) lapply(rows, function(r) as.numeric(r$y - r$x %*% b))
  scores <- function(u) Map(function(r, e) as.numeric(crossprod(r$z, e)), rows, u)
  meat <- function(u) Reduce(`+`, lapply(scores(u), tcrossprod))
  u1 <- resid(b1)
  k <- length(b1)
  rob1 <- bread1 %*% t(zx) %*% one %*% meat(u1) %*% one %*% zx %*% bread1
  if (steps == 'onestep') {
    b <- b1
    used <- u1
    bread <- bread1
    weight <- one
    sigma2 <- sum(vapply(u1, function(e) sum(e[seq_len(nd)]^2), 1)) / (2 * (filled - k))
    s <- Reduce(`+`, scores(u1))
    sargan <- sum(s * (one %*% s)) / sigma2
    v <- if (robust) rob1 else bread1 * sigma2
  } else {
    two <- solve(meat(u1))
    bread2 <- solve(t(zx) %*% two %*% zx)
    b <- bread2 %*% t(zx) %*% two %*% zy
    used <- resid(b)
    bread <- bread2
    weight <- two
    g <- Reduce(`+`, scores(used))
    sargan <- sum(g * (two %*% g))
    v <- bread2
    if (robust) {
      shift <- vapply(seq_len(k), function(j) {
        dw <- Reduce(`+`, Map(function(r, e) {
          xj <- r$x[, j]
          t(r$z) %*% (tcrossprod(xj, e) + tcrossprod(e, xj)) %*% r$z
        }, rows, u1))
        as.numeric(bread2 %*% t(zx) %*% two %*% dw %*% two %*% g)
      }, numeric(k))
      v <- bread2 + shift %*% bread2 + bread2 %*% t(shift) + shift %*% rob1 %*% t(shift)
    }
  }
  # Arellano and Bond's test of autocorrelation of order j in the differenced errors: the
  # lagged residual is the difference equation's, padded with zeros over the level rows,
  # while the residual it is weighted by is the whole system's, as Stata's estat abond has it
  mtest <- function(order) {
    if (order >= nd) return(NA_real_)
    lagged <- lapply(used, function(e) {
      out <- numeric(length(e))
      out[(order + 1):nd] <- e[seq_len(nd - order)]
      out
    })
    ex <- Reduce(`+`, Map(function(e, r) crossprod(e, r$x), lagged, rows))
    eve <- Reduce(`+`, Map(function(e, l) sum(e * l)^2, used, lagged))
    zve <- Reduce(`+`, Map(function(r, e, l) crossprod(r$z, e) * sum(e * l), rows, used, lagged))
    top <- Reduce(`+`, Map(function(e, l) sum(e * l), used, lagged))
    bottom <- eve - 2 * ex %*% bread %*% t(zx) %*% weight %*% zve + ex %*% v %*% t(ex)
    as.numeric(top / sqrt(bottom))
  }
  # with one step and conventional errors Stata leaves them out, as they need a robust one
  m <- if (robust || steps != 'onestep') vapply(1:2, mtest, numeric(1)) else c(NA_real_, NA_real_)
  list(coefficients = setNames(as.numeric(b),
    c(paste0('L', seq_len(lags), '.', y), x, colnames(dummies), if (cons) '(Intercept)')), vcov = v,
    sargan = sargan, df = ncol(rows[[1]]$z) - k, units = length(rows), steps = steps,
    robust = robust, difference = filled,
    level = sum(vapply(rows, function(r) sum(r$filled[nd + seq_len(nl)]), 1)), m = m)
})
local({
  fit <- MODEL
  se <- sqrt(diag(fit$vcov))
  table <- cbind(Estimate = fit$coefficients, 'Std. Error' = se, 'z-value' = fit$coefficients / se,
    'Pr(>|z|)' = 2 * stats::pnorm(-abs(fit$coefficients / se)))
  cat(sprintf('%s system GMM (Blundell-Bond): %d units, %d difference and %d level equations\\n',
    if (fit$steps == 'onestep') 'One-step' else 'Two-step', fit$units, fit$difference, fit$level))
  cat(if (fit$robust) 'Standard errors robust to heteroskedasticity' else 'Conventional standard errors')
  cat(if (fit$robust && fit$steps != 'onestep') ' (Windmeijer-corrected)\\n\\n' else '\\n\\n')
  stats::printCoefmat(table, digits = 7)
  if (fit$robust && fit$steps == 'onestep') {
    cat('\\nSargan test: not available with robust errors\\n')
  } else {
    cat(sprintf('\\nSargan test: chisq(%d) = %s (p-value = %s)\\n', fit$df, format(fit$sargan, digits = 7),
      format.pval(stats::pchisq(fit$sargan, fit$df, lower.tail = FALSE), digits = 5)))
  }
  if (all(is.na(fit$m))) {
    cat('Autocorrelation tests: not available after one step with conventional errors\\n')
  } else {
    for (j in seq_along(fit$m)) {
      if (is.na(fit$m[j])) {
        cat(sprintf('Autocorrelation test (%d): too few periods\\n', j))
      } else {
        cat(sprintf('Autocorrelation test (%d): normal = %s (p-value = %s)\\n', j,
          format(fit$m[j], digits = 7),
          format.pval(2 * stats::pnorm(-abs(fit$m[j])), digits = 5)))
      }
    }
  }
})";

const INSTRUMENT_LIMITS: &[Opt] = &[
    Opt {
        value: "all",
        label: "All earlier",
    },
    Opt {
        value: "1",
        label: "1",
    },
    Opt {
        value: "2",
        label: "2",
    },
    Opt {
        value: "3",
        label: "3",
    },
    Opt {
        value: "4",
        label: "4",
    },
];

const OUTCOME_LAGS: &[Opt] = &[
    Opt {
        value: "1",
        label: "1",
    },
    Opt {
        value: "2",
        label: "2",
    },
    Opt {
        value: "3",
        label: "3",
    },
];

/// After one step plm leaves the standard errors and the Sargan statistic unscaled. Stata's
/// scale for both is the residual variance of the differenced equation, the sum of squared
/// residuals over twice the used rows less the coefficients; the covariance also takes the
/// number of units (plm's weighting matrix is divided by it). Rows plm pads for gaps in the
/// panel are all zeros in the instruments and are not counted. The covariance in the saved
/// model is replaced, so `summary(model, robust = FALSE)` shows the scaled errors.
const ONE_STEP_CONVENTIONAL: &str = "local({
  w <- MODEL$W
  e <- MODEL$residuals
  used <- vapply(w, function(a) sum(rowSums(abs(a)) > 0), 1)
  groups <- length(w)
  k <- length(stats::coef(MODEL))
  sigma2 <- sum(unlist(e)^2) / (2 * (sum(used) - k))
  scores <- Reduce(`+`, Map(function(a, r) crossprod(a, r), w, e))
  sargan <- sum(scores * (MODEL$A1 %*% scores)) / groups / sigma2
  df <- ncol(w[[1]]) - k
  MODEL$vcov <<- MODEL$vcov * sigma2 * groups
  out <- utils::capture.output(print(summary(MODEL, robust = FALSE TIMES)))
  line <- sprintf('Sargan test: chisq(%d) = %s (p-value = %s)', df, format(sargan, digits = 7),
    format.pval(stats::pchisq(sargan, df, lower.tail = FALSE), digits = 5))
  out[grepl('^Sargan test', out)] <- line
  cat(out, sep = '\\n')
})";

/// The panel check, plus the choices a dynamic model cannot take.
fn dynamic_check(a: &Args) -> Vec<Problem> {
    let mut out = plm_check(a);
    if a.choice("method") == "difference" && a.flag("constant") {
        out.push(Problem::new(
            "constant",
            "Difference GMM has no constant: differencing takes it away with the unit effects; \
             untick it, or take system GMM.",
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

pub const DYNAMIC_PANEL: Spec = Spec {
    entry: "Dynamic panel model (GMM)",
    menu: "Statistics > Panel models > Dynamic panel model (GMM)",
    about: "The Arellano-Bond difference GMM estimator of a panel model with lags of the outcome \
            among the predictors and a fixed effect for each unit: the model is differenced, and \
            every lag of the outcome from the second back is an instrument; the other \
            predictors are taken as exogenous and instrument themselves. It fits with \
            plm::pgmm and prints the summary with Arellano and Bond's tests of first- and \
            second-order autocorrelation in the differenced errors (a significant first order \
            is expected; a significant second order says the instruments are not valid) and \
            the Sargan test of the overidentifying restrictions. Time effects add a dummy for \
            each period. After one step with conventional errors, the standard errors and the \
            Sargan test are scaled as Stata's xtabond does (plm leaves them unscaled); after \
            one step with robust errors there is no Sargan test, as in Stata. The number of \
            lags of the outcome used as instruments can be limited (Stata's maxldep). System \
            GMM (Blundell-Bond, Stata's xtdpdsys) adds the equation in levels, with the lagged \
            differences of the outcome as its instruments; it is written in base R, needs a \
            time variable whose periods step evenly, and can take a constant in the level \
            equation (instrumented by itself, which keeps the second period's row) and time \
            effects (a dummy for each period from the third on, among the exogenous \
            predictors). A unit may miss periods: the rows it cannot fill drop out. \
            Difference GMM has no constant.",
    widgets: &[
        Widget::Variable {
            id: "y",
            label: "Outcome",
            kinds: NUMBERS,
            required: true,
            hint: "",
        },
        Widget::Choice {
            id: "lags",
            label: "Lags of the outcome as predictors",
            options: OUTCOME_LAGS,
            default: "1",
            hint: "The outcome one, two or three periods back.",
        },
        Widget::Choice {
            id: "maxlag",
            label: "Lags of the outcome used as instruments",
            options: INSTRUMENT_LIMITS,
            default: "all",
            hint: "Every earlier level is an instrument by default; fewer instruments make a \
                   smaller, less efficient but sturdier fit (Stata's maxldep).",
        },
        Widget::Variables {
            id: "x",
            label: "Other predictors",
            kinds: NUMBERS,
            min: 0,
            hint: "Taken as exogenous: not correlated with the errors in any period.",
        },
        PANEL_ID,
        PANEL_TIME,
        Widget::Choice {
            id: "method",
            label: "Estimator",
            options: DYNAMIC_METHODS,
            default: "difference",
            hint: "System GMM adds the equation in levels, with the lagged differences of the \
                   outcome as its instruments: it is more efficient when the outcome is close \
                   to a random walk, and it needs a time variable whose periods step evenly.",
        },
        Widget::Choice {
            id: "steps",
            label: "Estimation",
            options: STEPS,
            default: "twosteps",
            hint: "",
        },
        Widget::Choice {
            id: "errors",
            label: "Standard errors",
            options: DYNAMIC_ERRORS,
            default: "robust",
            hint: "After two steps, conventional errors are biased downwards; robust ones are \
                   recommended.",
        },
        Widget::Check {
            id: "constant",
            label: "Constant (system GMM only)",
            default: false,
            hint: "The level equation gets a constant, instrumented by itself, as Stata's \
                   xtdpdsys does unless told otherwise. Difference GMM has none: it is \
                   differenced away.",
        },
        Widget::Check {
            id: "twoways",
            label: "Time effects (a dummy for each period)",
            default: false,
            hint: "Adds a dummy for each period to the predictors, as a variable for each year \
                   would in Stata.",
        },
        SAVE,
    ],
    needs: &["plm"],
    styles: BASE,
    check: dynamic_check,
    code: |a, _| {
        let model = a.name("save");
        let y = a.var("y").unwrap_or_default();
        let lags: usize = a.choice("lags").parse().unwrap_or(1);
        if a.choice("method") == "system" {
            let others: Vec<String> = a.vars("x").iter().map(|n| r_string(n)).collect();
            let others = if others.is_empty() {
                "character(0)".to_string()
            } else {
                format!("c({})", others.join(", "))
            };
            let limit = match a.choice("maxlag").parse::<usize>() {
                Ok(q) => q.to_string(),
                Err(_) => "Inf".to_string(),
            };
            return SYSTEM_GMM
                .replace("@LIMIT@", &limit)
                .replace("@DATA@", &a.data())
                .replace("@ID@", &a.var_string("id").unwrap_or_default())
                .replace("@TIME@", &a.var_string("time").unwrap_or_default())
                .replace("@Y@", &a.var_string("y").unwrap_or_default())
                .replace("@X@", &others)
                .replace("@LAGS@", &lags.to_string())
                .replace("@STEPS@", &r_string(a.choice("steps")))
                .replace(
                    "@ROBUST@",
                    if a.choice("errors") == "robust" {
                        "TRUE"
                    } else {
                        "FALSE"
                    },
                )
                .replace("@CONS@", if a.flag("constant") { "TRUE" } else { "FALSE" })
                .replace("@TIMES@", if a.flag("twoways") { "TRUE" } else { "FALSE" })
                .replace("MODEL", &model);
        }
        let lag_term = if lags == 1 {
            format!("stats::lag({y}, 1)")
        } else {
            format!("stats::lag({y}, 1:{lags})")
        };
        let mut terms = vec![lag_term];
        terms.extend(a.vars("x").iter().map(|n| r_name(n)));
        let steps = a.choice("steps");
        let effect = if a.flag("twoways") {
            "twoways"
        } else {
            "individual"
        };
        // plm leaves the period dummies out of the table unless asked
        let times = if a.flag("twoways") {
            ", time.dummies = TRUE"
        } else {
            ""
        };
        // the last lag of the outcome used as an instrument: Stata's maxldep(q) is lags 2 to q + 1
        let last = match a.choice("maxlag").parse::<usize>() {
            Ok(q) => (q + 1).to_string(),
            Err(_) => "99".to_string(),
        };
        let fit = format!(
            "{model} <- local({{\n  # pgmm looks for plm() in the frame that calls it\n  plm <- plm::plm\n  \
             plm::pgmm({y} ~ {} | stats::lag({y}, 2:{last}), data = {}, {}, \
             effect = \"{effect}\", model = {}, transformation = \"d\")\n}})",
            terms.join(" + "),
            a.data(),
            index_arg(a),
            r_string(steps),
        );
        let robust = a.choice("errors") == "robust";
        let show = if steps == "onestep" && robust {
            format!(
                "out <- utils::capture.output(print(summary({model}, robust = TRUE{times})))\n\
                 cat(out[!grepl(\"^Sargan test\", out)], sep = \"\\n\")\n\
                 cat(\"Sargan test: not available with robust errors\\n\")"
            )
        } else if steps == "onestep" {
            ONE_STEP_CONVENTIONAL
                .replace("MODEL", &model)
                .replace(" TIMES", times)
        } else {
            format!(
                "print(summary({model}, robust = {}{times}))",
                if a.choice("errors") == "robust" {
                    "TRUE"
                } else {
                    "FALSE"
                }
            )
        };
        format!("{fit}\n{show}")
    },
};

// ---- Panel instrumental variables ----

const IV_MODELS: &[Opt] = &[
    Opt {
        value: "within",
        label: "Fixed effects (within)",
    },
    Opt {
        value: "random",
        label: "Random effects (Balestra and Varadharajan-Krishnakumar)",
    },
    Opt {
        value: "ec2sls",
        label: "Random effects, EC2SLS (Baltagi)",
    },
    Opt {
        value: "between",
        label: "Between",
    },
    Opt {
        value: "fd",
        label: "First differences",
    },
];

const IV_ERRORS: &[Opt] = &[
    Opt {
        value: "conventional",
        label: "Conventional",
    },
    Opt {
        value: "robust",
        label: "Robust to clustering on the panel unit",
    },
];

/// Coefficients with the constant and with standard errors written out: the fixed-effects
/// constant that plm leaves out, and errors clustered on the panel unit (plm's own
/// covariance functions do not use the instrumented predictors). Two-stage least squares is
/// run on the transformed data the fit used; the within fit adds the grand means back, so the
/// constant is the one Stata reports. The clustered errors take the factor G/(G-1) and
/// (N-1)/(N-K), K counting the constant; conventional errors on the within fit divide by
/// N-G-K without the constant. Estimates equal the fit's; only the errors are new.
const IV_TABLE: &str = "local({
  m <- MODEL
  robust <- ROBUST
  small <- SMALL
  within <- identical(m$args$model, 'within')
  res <- stats::residuals(m)
  index <- plm::index(m)
  id <- as.character(index[[1]])[match(names(res), rownames(index))]
  y <- as.numeric(plm::pmodel.response(m))
  x <- stats::model.matrix(m)
  z <- stats::model.matrix(m, rhs = 2)
  if (within) {
    raw <- function(rhs) stats::model.matrix(m$model, rhs = rhs, model = 'pooling')
    x <- cbind(sweep(x, 2, colMeans(raw(1)[, colnames(x), drop = FALSE]), '+'), '(Intercept)' = 1)
    z <- cbind(sweep(z, 2, colMeans(raw(2)[, colnames(z), drop = FALSE]), '+'), '(Intercept)' = 1)
    y <- y + mean(as.numeric(m$model[[1]]))
  }
  fitted <- z %*% solve(crossprod(z), crossprod(z, x))
  bread <- solve(crossprod(fitted))
  b <- as.numeric(bread %*% crossprod(fitted, y))
  e <- as.numeric(y - x %*% b)
  n <- length(e)
  g <- length(unique(id))
  labels <- colnames(x)
  if (robust) {
    meat <- Reduce(`+`, lapply(split(seq_along(e), id), function(i) {
      s <- crossprod(fitted[i, , drop = FALSE], e[i])
      tcrossprod(s)
    }))
    v <- bread %*% meat %*% bread * g / (g - 1) * (n - 1) / (n - ncol(x))
  } else if (within) {
    v <- bread * sum(e^2) / (n - g - ncol(x) + 1)
  } else {
    b <- as.numeric(stats::coef(m))
    labels <- names(stats::coef(m))
    v <- stats::vcov(m)
  }
  se <- sqrt(diag(v))
  stat <- b / se
  if (small) {
    # Stata's degrees of freedom: the within fit takes the unit effects off, the clustered
    # differences count the units less one (random effects are computed directly)
    df <- if (within) n - g - ncol(x) + 1
      else if (robust) g - identical(m$args$model, 'fd')
      else n - ncol(x)
    table <- cbind(Estimate = b, 'Std. Error' = se, 't-value' = stat,
      'Pr(>|t|)' = 2 * stats::pt(-abs(stat), df))
  } else {
    table <- cbind(Estimate = b, 'Std. Error' = se, 'z-value' = stat,
      'Pr(>|z|)' = 2 * stats::pnorm(-abs(stat)))
  }
  rownames(table) <- labels
  cat(if (robust) '\nCoefficients, standard errors robust to clustering on the panel unit'
      else '\nCoefficients with the constant', if (small) sprintf(' (t statistics, %d degrees of freedom)', df), ':\n', sep = '')
  stats::printCoefmat(table, digits = 7)
})";

/// Whether the fit is random effects, computed directly ([`RE_IV_STATA`])
/// rather than by plm: plm's Swamy-Arora components and EC2SLS instruments are
/// not Stata's on an unbalanced panel.
fn direct_re_iv(a: &Args) -> bool {
    matches!(a.choice("model"), "random" | "ec2sls")
}

fn panel_iv_check(a: &Args) -> Vec<Problem> {
    let mut out = plm_check(a);
    let model = a.choice("model");
    let random = matches!(model, "random" | "ec2sls");
    if a.choice("errors") == "robust" && model == "between" {
        out.push(Problem::new(
            "errors",
            "Robust errors are not offered for the between estimator (Stata's xtivreg, be has \
             none either); take conventional errors.",
        ));
    }
    if a.flag("nosa") && !random {
        out.push(Problem::new(
            "nosa",
            "Baltagi-Chang components are for the random-effects estimators.",
        ));
    }
    if a.flag("regress") {
        if !matches!(model, "within" | "random" | "ec2sls") {
            out.push(Problem::new(
                "regress",
                "Ignoring the instruments is offered for fixed and random effects.",
            ));
        }
        if a.choice("errors") == "robust" {
            out.push(Problem::new(
                "regress",
                "With the instruments ignored, take conventional errors.",
            ));
        }
    }
    let endogenous = a.vars("endog");
    let instruments = a.vars("inst");
    let exogenous = a.vars("x");
    let y = a.var_text("y");
    if endogenous.contains(&y) {
        out.push(Problem::new(
            "endog",
            format!("{y} is the outcome; it cannot also be an endogenous predictor."),
        ));
    }
    if let Some(v) = endogenous.iter().find(|v| exogenous.contains(v)) {
        out.push(Problem::new(
            "endog",
            format!("{v} is already an exogenous predictor; it cannot also be endogenous."),
        ));
    }
    if let Some(v) = instruments
        .iter()
        .find(|v| exogenous.contains(v) || endogenous.contains(v) || **v == y)
    {
        out.push(Problem::new(
            "inst",
            format!("{v} is in the model; an instrument must be a variable left out of it."),
        ));
    }
    if instruments.len() < endogenous.len() {
        out.push(Problem::new(
            "inst",
            format!(
                "The model needs at least as many instruments as endogenous predictors: {} \
                 instruments for {}.",
                instruments.len(),
                endogenous.len()
            ),
        ));
    }
    for (w, list) in [("endog", &endogenous), ("inst", &instruments)] {
        for v in list {
            if [a.var_text("id"), a.var_text("time")].contains(v) {
                out.push(Problem::new(
                    w,
                    format!("{v} is the panel unit or the time; choose another variable."),
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

pub const PANEL_IV: Spec = Spec {
    entry: "Panel instrumental variables",
    menu: "Statistics > Panel models > Panel instrumental variables",
    about: "A panel regression in which some predictors are correlated with the errors, \
            replaced by instruments: two-stage least squares on the within-transformed data \
            (fixed effects), or the random-effects version of Balestra and Varadharajan-\
            Krishnakumar (Stata's xtivreg, re default), the same with Baltagi's EC2SLS, the \
            between estimator, or first differences. Fixed effects, between and differences \
            fit with plm::plm and print its summary, then the coefficients with the constant \
            (the fixed-effects constant is the average of the unit effects, as in Stata). \
            Random effects are computed directly with xtivreg's variance components and \
            instruments, unbalanced panels too, and saved as a list of coefficients, \
            covariance and components. Standard errors robust to clustering on the panel \
            unit (Stata's vce(robust)) are offered for all but the between estimator.",
    widgets: &[
        Widget::Variable {
            id: "y",
            label: "Outcome",
            kinds: NUMBERS,
            required: true,
            hint: "",
        },
        Widget::Variables {
            id: "endog",
            label: "Endogenous predictors",
            kinds: NUMBERS,
            min: 1,
            hint: "Predictors that may be correlated with the errors.",
        },
        Widget::Variables {
            id: "inst",
            label: "Instruments",
            kinds: NUMBERS,
            min: 1,
            hint: "Variables left out of the model that move the endogenous predictors and are \
                   not correlated with the errors: at least as many as endogenous predictors.",
        },
        Widget::Variables {
            id: "x",
            label: "Exogenous predictors",
            kinds: PREDICTORS,
            min: 0,
            hint: "Predictors that are not correlated with the errors.",
        },
        PANEL_ID,
        PANEL_TIME,
        Widget::Choice {
            id: "model",
            label: "Estimator",
            options: IV_MODELS,
            default: "within",
            hint: "",
        },
        Widget::Choice {
            id: "errors",
            label: "Standard errors",
            options: IV_ERRORS,
            default: "conventional",
            hint: "",
        },
        Widget::Check {
            id: "small",
            label: "Small-sample statistics (t instead of z)",
            default: false,
            hint:
                "Judges each coefficient by a t distribution with the degrees of freedom Stata's \
                   small option uses; the errors themselves do not change.",
        },
        Widget::Check {
            id: "nosa",
            label: "Baltagi-Chang variance components (Stata's nosa)",
            default: false,
            hint: "Random effects only: the variance components of Baltagi and Chang instead of \
                   the adapted Swamy-Arora ones.",
        },
        Widget::Check {
            id: "regress",
            label: "Ignore the instruments (Stata's regress)",
            default: false,
            hint: "Fixed or random effects, conventional errors: the endogenous predictors are \
                   treated as exogenous (fixed effects: Stata's xtreg, fe; random effects: \
                   xtivreg's own variance components, computed directly).",
        },
        SAVE,
    ],
    needs: &["plm"],
    styles: BASE,
    check: panel_iv_check,
    code: |a, _| {
        let model = a.name("save");
        let join = |names: &[&str]| {
            names
                .iter()
                .map(|n| r_name(n))
                .collect::<Vec<_>>()
                .join(" + ")
        };
        let exogenous = a.vars("x");
        let endogenous = a.vars("endog");
        let instruments = a.vars("inst");
        let predictors: Vec<&str> = exogenous.iter().chain(&endogenous).copied().collect();
        let all_instruments: Vec<&str> = exogenous.iter().chain(&instruments).copied().collect();
        let robust = a.choice("errors") == "robust";
        if direct_re_iv(a) {
            let regress = a.flag("regress");
            let mut cols: Vec<&str> = vec![a.var_text("y"), a.var_text("id")];
            for v in predictors.iter().chain(&instruments) {
                if !cols.contains(v) {
                    cols.push(v);
                }
            }
            let ec2sls = a.choice("model") == "ec2sls";
            let mut title = match (regress, ec2sls) {
                (true, _) => "Random-effects GLS regression, the instruments ignored",
                (false, true) => "EC2SLS random-effects IV",
                (false, false) => "G2SLS random-effects IV",
            }
            .to_string();
            if a.flag("nosa") {
                title.push_str(", Baltagi-Chang components");
            }
            if robust {
                title.push_str(", errors robust to clustering on the panel unit");
            }
            return RE_IV_STATA
                .replace("{model}", &model)
                .replace("{data}", &a.data())
                .replace(
                    "{cols}",
                    &cols
                        .iter()
                        .map(|c| r_string(c))
                        .collect::<Vec<_>>()
                        .join(", "),
                )
                .replace("{id}", &r_string(a.var_text("id")))
                .replace("{y}", &r_string(a.var_text("y")))
                .replace("{xterms}", &join(&predictors))
                .replace(
                    "{zterms}",
                    &join(if regress {
                        &predictors
                    } else {
                        &all_instruments
                    }),
                )
                .replace(
                    "{ec2sls}",
                    if ec2sls && !regress { "TRUE" } else { "FALSE" },
                )
                .replace("{nosa}", if a.flag("nosa") { "TRUE" } else { "FALSE" })
                .replace("{robust}", if robust { "TRUE" } else { "FALSE" })
                .replace("{small}", if a.flag("small") { "TRUE" } else { "FALSE" })
                .replace("{title}", &r_string(&title));
        }
        if a.flag("regress") {
            // fixed effects with the instruments ignored: the within regression
            return format!(
                "{model} <- plm::plm({} ~ {}, data = {}, {}, model = \"within\")\nprint(summary({model}))",
                a.var("y").unwrap_or_default(),
                join(&predictors),
                a.data(),
                index_arg(a),
            );
        }
        let plm_model = a.choice("model");
        let mut out = format!(
            "{model} <- plm::plm({} ~ {} | {}, data = {}, {}, model = {})\nprint(summary({model}))",
            a.var("y").unwrap_or_default(),
            join(&predictors),
            join(&all_instruments),
            a.data(),
            index_arg(a),
            r_string(plm_model),
        );
        // the table with the constant: for the within fit always, for the rest when robust
        let small = a.flag("small");
        if robust || small || plm_model == "within" {
            out.push('\n');
            out.push_str(
                &IV_TABLE
                    .replace("MODEL", &model)
                    .replace("ROBUST", if robust { "TRUE" } else { "FALSE" })
                    .replace("SMALL", if small { "TRUE" } else { "FALSE" }),
            );
        }
        out
    },
};

/// Stata's random-effects IV (`xtivreg, re`) computed directly: plm's
/// components and EC2SLS instruments are not Stata's on unbalanced panels, and
/// plm has no Baltagi-Chang components (`nosa`), no fit with the instruments
/// ignored (`regress`) and no robust errors after EC2SLS. The components are
/// Stata's: sigma_e^2 from the within 2SLS residuals over N - n -
/// K (N - n with nosa); sigma_u^2 from the between 2SLS residuals, the adapted
/// Swamy-Arora (sum of T_i e_i^2 - (n - K - 1) sigma_e^2) / (N - tr((X'PX)^-1
/// X'PPX)) or, with nosa, (sum of T_i e_i^2 - n sigma_e^2) / N. The data are
/// quasi-demeaned with theta_i = 1 - sqrt(sigma_e^2 / (T_i sigma_u^2 +
/// sigma_e^2)); G2SLS transforms the instruments the same way. EC2SLS takes
/// each instrument's within deviation with the grand mean added back, its unit
/// mean, and 1 - theta_i for the constant (no plain constant), as black-box
/// runs of Stata 19 on unbalanced panels show; on a balanced panel these span
/// Baltagi's within and between parts. The conventional covariance is the
/// transformed residuals' variance over N - p times (X^'X^)^-1, the robust one
/// the cluster sandwich times n / (n - 1) (N - 1) / (N - p); the small option
/// gives t statistics on N - p + 1 degrees of freedom (n when robust).
const RE_IV_STATA: &str = "{model} <- local({
  d <- {data}[stats::complete.cases({data}[, c({cols})]), , drop = FALSE]
  id <- as.character(d[[{id}]])
  ec2sls <- {ec2sls}; nosa <- {nosa}; robust <- {robust}; small <- {small}
  y <- d[[{y}]]
  X0 <- stats::model.matrix(~ {xterms}, d)[, -1, drop = FALSE]
  Z0 <- stats::model.matrix(~ {zterms}, d)[, -1, drop = FALSE]
  N <- length(y); n <- length(unique(id)); K <- ncol(X0)
  tsls <- function(y, X, Z) {
    Xh <- Z %*% solve(crossprod(Z), crossprod(Z, X))
    b <- solve(crossprod(Xh, X), crossprod(Xh, y))
    list(b = drop(b), e = drop(y - X %*% b), Xh = Xh)
  }
  dm <- function(v) v - stats::ave(v, id)
  mb <- function(v) stats::ave(v, id)
  w <- tsls(dm(y), apply(X0, 2, dm), apply(Z0, 2, dm))
  se2 <- sum(w$e^2) / (N - n - if (nosa) 0 else K)
  first <- !duplicated(id)
  Tu <- as.vector(table(id)[id[first]])
  Xb <- cbind(1, apply(X0, 2, mb))
  Zb <- cbind(1, apply(Z0, 2, mb))
  b1 <- tsls(mb(y)[first], Xb[first, , drop = FALSE], Zb[first, , drop = FALSE])
  uPu <- sum(Tu * b1$e^2)
  su2 <- if (nosa) (uPu - n * se2) / N else
    (uPu - (n - K - 1) * se2) / (N - sum(diag(solve(crossprod(Xb)) %*% crossprod(rowsum(Xb, id)))))
  su2 <- max(su2, 0)
  Ti <- as.vector(table(id)[id])
  th <- 1 - sqrt(se2 / (Ti * su2 + se2))
  X <- cbind('(Intercept)' = 1, X0)
  Xs <- X - th * apply(X, 2, mb)
  ys <- y - th * mb(y)
  Z <- if (ec2sls) cbind(sweep(apply(Z0, 2, dm), 2, colMeans(Z0), '+'), 1 - th, apply(Z0, 2, mb)) else {
    Zc <- cbind(1, Z0); Zc - th * apply(Zc, 2, mb)
  }
  r <- tsls(ys, Xs, Z)
  A <- solve(crossprod(r$Xh))
  p <- ncol(X)
  V <- if (robust) {
    meat <- Reduce(`+`, lapply(split(seq_len(N), id), function(i)
      tcrossprod(colSums(r$Xh[i, , drop = FALSE] * r$e[i]))))
    A %*% meat %*% A * n / (n - 1) * (N - 1) / (N - p)
  } else sum(r$e^2) / (N - p) * A
  dimnames(V) <- list(colnames(X), colnames(X))
  list(coefficients = stats::setNames(r$b, colnames(X)), vcov = V, sigma_u = sqrt(su2),
       sigma_e = sqrt(se2), nobs = N, groups = n,
       df = if (small) (if (robust) n else N - p + 1) else NA)
})
local({
  m <- {model}
  se <- sqrt(diag(m$vcov))
  z <- m$coefficients / se
  cat({title}, sprintf(' (%d rows, %d units)\\n', m$nobs, m$groups), sep = '')
  table <- if (is.na(m$df)) cbind(Estimate = m$coefficients, 'Std. Error' = se,
      'z value' = z, 'Pr(>|z|)' = 2 * stats::pnorm(-abs(z))) else {
    cat(sprintf('(t statistics, %d degrees of freedom)\\n', m$df))
    cbind(Estimate = m$coefficients, 'Std. Error' = se, 't value' = z,
      'Pr(>|t|)' = 2 * stats::pt(-abs(z), m$df))
  }
  stats::printCoefmat(table, digits = 7)
  cat(sprintf('sigma_u = %.7g, sigma_e = %.7g\\n', m$sigma_u, m$sigma_e))
})";
// ---- Panel unit-root test ----

const UNIT_ROOT_TESTS: &[Opt] = &[
    Opt {
        value: "levinlin",
        label: "Levin-Lin-Chu (one common root)",
    },
    Opt {
        value: "ips",
        label: "Im-Pesaran-Shin (a root for each unit)",
    },
];

const UNIT_ROOT_TERMS: &[Opt] = &[
    Opt {
        value: "intercept",
        label: "A mean for each unit",
    },
    Opt {
        value: "trend",
        label: "A mean and a trend for each unit",
    },
    Opt {
        value: "none",
        label: "Neither (Levin-Lin-Chu only)",
    },
];

const UNIT_ROOT_LAGS: &[Opt] = &[
    Opt {
        value: "0",
        label: "0",
    },
    Opt {
        value: "1",
        label: "1",
    },
    Opt {
        value: "2",
        label: "2",
    },
    Opt {
        value: "3",
        label: "3",
    },
    Opt {
        value: "4",
        label: "4",
    },
];

/// The periods at which Im, Pesaran and Shin tabulate the mean and variance of the
/// Dickey-Fuller t statistic (Table 3 of the 2003 paper); between them Stata, and so the
/// dialog, interpolates linearly, and beyond them takes the end rows. The numbers were read
/// off Stata 19's xtunitroot ips on made-up panels by solving for them, and checked at
/// periods between the rows.
const IPS_PERIODS: [f64; 10] = [10., 15., 20., 25., 30., 40., 50., 60., 70., 100.];

const NA: f64 = f64::NAN;

/// (lags, with a trend, mean at each of the periods, variance at each).
#[allow(clippy::type_complexity)]
const IPS_TABLE: &[(usize, bool, [f64; 10], [f64; 10])] = &[
    (
        0,
        false,
        [
            -1.504, -1.514, -1.522, -1.520, -1.526, -1.523, -1.527, -1.519, -1.524, -1.532,
        ],
        [
            1.069, 0.923, 0.851, 0.809, 0.789, 0.770, 0.760, 0.749, 0.736, 0.735,
        ],
    ),
    (
        1,
        false,
        [
            -1.488, -1.503, -1.516, -1.514, -1.519, -1.520, -1.524, -1.519, -1.522, -1.530,
        ],
        [
            1.255, 1.011, 0.915, 0.861, 0.831, 0.803, 0.781, 0.770, 0.753, 0.745,
        ],
    ),
    (
        2,
        false,
        [
            -1.319, -1.387, -1.428, -1.443, -1.460, -1.476, -1.493, -1.490, -1.498, -1.514,
        ],
        [
            1.421, 1.078, 0.969, 0.905, 0.865, 0.830, 0.798, 0.789, 0.766, 0.754,
        ],
    ),
    (
        3,
        false,
        [
            -1.306, -1.366, -1.413, -1.433, -1.453, -1.471, -1.489, -1.486, -1.495, -1.512,
        ],
        [
            1.759, 1.181, 1.037, 0.952, 0.907, 0.858, 0.819, 0.802, 0.782, 0.761,
        ],
    ),
    (
        4,
        false,
        [
            NA, -1.260, -1.329, -1.363, -1.394, -1.428, -1.454, -1.458, -1.470, -1.495,
        ],
        [
            NA, 1.279, 1.097, 1.005, 0.946, 0.886, 0.842, 0.819, 0.801, 0.771,
        ],
    ),
    (
        0,
        true,
        [
            -2.166, -2.167, -2.168, -2.167, -2.172, -2.173, -2.176, -2.174, -2.174, -2.177,
        ],
        [
            1.132, 0.869, 0.763, 0.713, 0.690, 0.655, 0.633, 0.621, 0.610, 0.597,
        ],
    ),
    (
        1,
        true,
        [
            -2.173, -2.169, -2.172, -2.172, -2.173, -2.177, -2.180, -2.178, -2.176, -2.179,
        ],
        [
            1.453, 0.975, 0.845, 0.769, 0.734, 0.687, 0.654, 0.641, 0.627, 0.605,
        ],
    ),
    (
        2,
        true,
        [
            -1.914, -1.999, -2.047, -2.074, -2.095, -2.120, -2.137, -2.143, -2.146, -2.158,
        ],
        [
            1.627, 1.036, 0.882, 0.796, 0.756, 0.702, 0.661, 0.653, 0.634, 0.613,
        ],
    ),
    (
        3,
        true,
        [
            NA, -1.977, -2.032, -2.065, -2.091, -2.117, -2.137, -2.142, -2.146, -2.158,
        ],
        [
            NA, 1.214, 0.983, 0.861, 0.808, 0.735, 0.688, 0.674, 0.650, 0.625,
        ],
    ),
    (
        4,
        true,
        [
            NA, -1.823, -1.911, -1.968, -2.009, -2.057, -2.091, -2.103, -2.114, -2.135,
        ],
        [
            NA, 1.332, 1.052, 0.913, 0.845, 0.759, 0.705, 0.685, 0.662, 0.629,
        ],
    ),
];

fn ips_grid() -> String {
    let items: Vec<String> = IPS_PERIODS.iter().map(|p| format!("{p}")).collect();
    format!("c({})", items.join(", "))
}

/// The table row for these lags and this trend, as the text of two R vectors.
fn ips_moments(lags: usize, trend: bool) -> (String, String) {
    let row = IPS_TABLE
        .iter()
        .find(|(l, t, _, _)| *l == lags && *t == trend);
    let text = |values: Option<&[f64; 10]>| {
        let items: Vec<String> = values
            .map(|v| {
                v.iter()
                    .map(|x| {
                        if x.is_nan() {
                            "NA".to_string()
                        } else {
                            format!("{x:.3}")
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();
        format!("c({})", items.join(", "))
    };
    (text(row.map(|r| &r.2)), text(row.map(|r| &r.3)))
}

/// Im-Pesaran-Shin W-t-bar as Stata's xtunitroot ips has it: each unit's augmented
/// Dickey-Fuller t (lags of the difference, a mean, perhaps a trend; the sample drops the
/// first lags + 1 periods), averaged over the units, then standardised with the tabulated
/// mean and variance of that t at the number of periods.
const IPS_CODE: &str = "print(local({
  panel <- plm::pdata.frame(@DATA@, @INDEX@)
  size <- plm::pdim(panel)
  stopifnot(\"the panel must be balanced: every unit observed in every period\" = size$balanced)
  periods <- size$nT$T
  lags <- @LAGS@
  trend <- @TREND@
  # mean and variance of the Dickey-Fuller t: Im, Pesaran and Shin (2003), Table 3
  grid <- @GRID@
  at <- min(max(periods, 10), 100)
  mean_t <- stats::approx(grid, @MEANS@, xout = at)$y
  var_t <- stats::approx(grid, @VARIANCES@, xout = at)$y
  stopifnot(\"too few periods for this many lags: no tabulated moments\" =
    !is.na(mean_t) && periods - lags - 1 > 2 + lags + trend)
  dickey_fuller <- function(y) {
    change <- diff(y)
    rows <- (lags + 1):(length(y) - 1)
    x <- cbind(1, if (trend) rows + 1, y[rows])
    if (lags > 0) x <- cbind(x, vapply(1:lags, function(j) change[rows - j], numeric(length(rows))))
    fit <- stats::lm.fit(x, change[rows])
    spot <- 2 + trend
    s2 <- sum(fit$residuals^2) / (length(rows) - ncol(x))
    unname(fit$coefficients[spot] / sqrt(s2 * solve(crossprod(x))[spot, spot]))
  }
  groups <- split(panel[[@VARIABLE@]], plm::index(panel)[[1]])
  t_bar <- mean(vapply(groups, dickey_fuller, numeric(1)))
  w <- sqrt(length(groups)) * (t_bar - mean_t) / sqrt(var_t)
  structure(list(
    statistic = c(`W-t-bar` = w),
    p.value = stats::pnorm(w),
    parameter = c(units = length(groups), periods = periods, lags = lags),
    estimate = c(`t-bar` = t_bar, `mean of t` = mean_t, `variance of t` = var_t),
    method = paste('Im-Pesaran-Shin unit-root test, a mean', if (trend) 'and a trend', 'for each unit'),
    data.name = @VARIABLE@,
    alternative = 'some units are stationary'
  ), class = 'htest')
}))";

fn unit_root_check(a: &Args) -> Vec<Problem> {
    let v = a.var_text("var");
    let id = a.var_text("id");
    let time = a.var_text("time");
    let mut out = Vec::new();
    for (w, other, what) in [("id", id, "panel unit"), ("time", time, "time")] {
        if !v.is_empty() && v == other {
            out.push(Problem::new(
                w,
                format!("{v} is the variable tested; choose another variable for the {what}."),
            ));
        }
    }
    if !id.is_empty() && id == time {
        out.push(Problem::new(
            "time",
            "The panel unit and the time must be different variables.",
        ));
    }
    if a.choice("test") == "ips" && a.choice("terms") == "none" {
        out.push(Problem::new(
            "terms",
            "The Im-Pesaran-Shin test needs a mean for each unit, with or without a trend.",
        ));
    }
    out
}

pub const PANEL_UNIT_ROOT: Spec = Spec {
    entry: "Panel unit-root test",
    menu: "Statistics > Panel models > Panel unit-root test",
    about: "Tests whether a variable has a unit root in every unit (the null) against being \
            stationary. Levin-Lin-Chu assumes one autoregressive coefficient for all units; \
            Im-Pesaran-Shin lets each unit have its own and tests whether some are stationary. \
            Each unit's regression has the lags of the difference chosen. It fits with \
            plm::purtest. The panel must be balanced. Levin-Lin-Chu's long-run variance uses a \
            Bartlett kernel with floor(3.21 T^(1/3)) lags, and its statistic and p-value are \
            Stata's xtunitroot llc. Im-Pesaran-Shin's W-t-bar is each unit's augmented \
            Dickey-Fuller t averaged and standardised with Im, Pesaran and Shin's tabulated \
            mean and variance (interpolated between their periods), and is Stata's xtunitroot \
            ips; it takes up to four lags and is written in base R.",
    widgets: &[
        Widget::Variable {
            id: "var",
            label: "Variable",
            kinds: NUMBERS,
            required: true,
            hint: "",
        },
        PANEL_ID,
        PANEL_TIME,
        Widget::Choice {
            id: "test",
            label: "Test",
            options: UNIT_ROOT_TESTS,
            default: "levinlin",
            hint: "",
        },
        Widget::Choice {
            id: "terms",
            label: "Include",
            options: UNIT_ROOT_TERMS,
            default: "intercept",
            hint: "",
        },
        Widget::Choice {
            id: "lags",
            label: "Lags of the difference",
            options: UNIT_ROOT_LAGS,
            default: "1",
            hint: "Added to each unit's regression to remove autocorrelation.",
        },
    ],
    needs: &["plm"],
    styles: BASE,
    check: unit_root_check,
    code: |a, _| {
        let test = a.choice("test");
        let variable = a.var_string("var").unwrap_or_default();
        if test == "ips" {
            let lags: usize = a.choice("lags").parse().unwrap_or(1);
            let trend = a.choice("terms") == "trend";
            let (means, variances) = ips_moments(lags, trend);
            return IPS_CODE
                .replace("@DATA@", &a.data())
                .replace("@INDEX@", &index_arg(a))
                .replace("@VARIABLE@", &variable)
                .replace("@LAGS@", &lags.to_string())
                .replace("@TREND@", if trend { "TRUE" } else { "FALSE" })
                .replace("@GRID@", &ips_grid())
                .replace("@MEANS@", &means)
                .replace("@VARIANCES@", &variances);
        }
        format!(
            "print(local({{\n  panel <- plm::pdata.frame({}, {})\n  \
             size <- plm::pdim(panel)\n  \
             stopifnot(\"the panel must be balanced: every unit observed in every period\" = \
             size$balanced)\n  periods <- size$nT$T\n  \
             plm::purtest(panel[[{variable}]], test = {}, exo = {}, lags = {}, \
             q = floor(3.21 * periods^(1/3)))\n}}))",
            a.data(),
            index_arg(a),
            r_string(test),
            r_string(a.choice("terms")),
            a.choice("lags"),
        )
    },
};

// ---- Panel serial-correlation test (Wooldridge) ----

/// A text for inside an R string.
fn in_string(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

pub const PANEL_SERIAL: Spec = Spec {
    entry: "Panel serial-correlation test",
    menu: "Statistics > Panel models > Panel serial-correlation test",
    about: "Wooldridge's test for first-order autocorrelation in the errors of a panel model. \
            If the errors are not autocorrelated, the residuals of the model in first \
            differences have a correlation of -0.5 with their own lag. The dialog regresses \
            the first differences of the outcome on those of the predictors (no constant), \
            regresses the residuals on their lag, and tests that coefficient against -0.5 with \
            standard errors clustered by panel unit: an F test with 1 and (units - 1) degrees \
            of freedom. Differences and lags go by the time variable, so a missing period is \
            not differenced across. Base R; no package.",
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
            kinds: NUMBERS,
            min: 1,
            hint: "Numbers only: the model is fitted in first differences.",
        },
        PANEL_ID,
        Widget::Variable {
            id: "time",
            label: "Time",
            kinds: NUMBERS,
            required: true,
            hint: "A whole number for each period (a year, say): differences go by this.",
        },
    ],
    needs: &[],
    styles: BASE,
    check: plm_check,
    code: |a, _| {
        let dif_x: Vec<String> = a
            .vars("x")
            .iter()
            .map(|n| format!("dif(d0[[{}]])", r_string(n)))
            .collect();
        let (id, time) = (
            a.var_string("id").unwrap_or_default(),
            a.var_string("time").unwrap_or_default(),
        );
        let y = a.var_string("y").unwrap_or_default();
        let label = format!(
            "{} on {}, clustered by {}",
            a.var_text("y"),
            a.vars("x").join(" and "),
            a.var_text("id")
        );
        format!(
            "print(local({{\n  d0 <- {d}\n  d0 <- d0[order(d0[[{id}]], d0[[{time}]]), ]\n  \
             stopifnot(\"each unit may have each period only once\" = \
             !anyDuplicated(d0[c({id}, {time})]))\n  \
             key <- function(i, t) paste(i, t)\n  \
             prev <- match(key(d0[[{id}]], d0[[{time}]] - 1), key(d0[[{id}]], d0[[{time}]]))\n  \
             dif <- function(v) v - v[prev]\n  dy <- dif(d0[[{y}]])\n  dx <- cbind({x})\n  \
             ok <- stats::complete.cases(dy, dx)\n  \
             e <- rep(NA_real_, length(dy))\n  \
             e[ok] <- stats::lm.fit(dx[ok, , drop = FALSE], dy[ok])$residuals\n  \
             le <- e[prev]\n  keep <- !is.na(e) & !is.na(le)\n  \
             ee <- e[keep]; ll <- le[keep]; g <- d0[[{id}]][keep]\n  \
             b <- sum(ll * ee) / sum(ll^2)\n  u <- ee - b * ll\n  n <- length(unique(g))\n  \
             stopifnot(\"the test needs at least two units with consecutive periods\" = n > 1)\n  \
             v <- n / (n - 1) * sum(tapply(ll * u, g, sum)^2) / sum(ll^2)^2\n  \
             f <- (b + 0.5)^2 / v\n  \
             structure(list(statistic = c(F = f), parameter = c(`num df` = 1, `denom df` = n - 1), \
             p.value = stats::pf(f, 1, n - 1, lower.tail = FALSE), \
             estimate = c(`residual on its lag` = b), \
             null.value = c(`residual on its lag` = -0.5), alternative = \"two.sided\", \
             method = \"Wooldridge test for serial correlation in panel data\", \
             data.name = \"{label}\"), class = \"htest\")\n}}))",
            d = a.data(),
            x = dif_x.join(", "),
            label = in_string(&label),
        )
    },
};

pub const SPECS: &[&Spec] = &[
    &FIXED_EFFECTS_REGRESSION,
    &FIXED_EFFECTS_GLM,
    &PANEL_MODEL,
    &HAUSMAN,
    &DYNAMIC_PANEL,
    &PANEL_IV,
    &PANEL_UNIT_ROOT,
    &PANEL_SERIAL,
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
                col("wage", VarKind::Number),
                col("hours", VarKind::Number),
                col("age group", VarKind::Factor),
                col("id", VarKind::Number),
                col("year", VarKind::Number),
                col("firm", VarKind::Text),
                col("visits", VarKind::Number),
                col("days", VarKind::Number),
                col("left", VarKind::Number),
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
    fn fixed_effects_regression_writes_feols() {
        let v = values(
            &FIXED_EFFECTS_REGRESSION,
            &[("y", "wage")],
            &[("x", &["hours", "age group"]), ("fe", &["id", "year"])],
        );
        assert_eq!(
            code(&FIXED_EFFECTS_REGRESSION, &v),
            "model <- fixest::feols(wage ~ hours + `age group` | id + year, data = d, vcov = \"cluster\")\n\
             print(summary(model))\nprint(stats::confint(model, level = 0.95))"
        );
        // a cluster variable takes the place of the choice
        let v = values(
            &FIXED_EFFECTS_REGRESSION,
            &[
                ("y", "wage"),
                ("cluster", "firm"),
                ("save", "fe1"),
                ("conf", "0.9"),
            ],
            &[("x", &["hours"]), ("fe", &["id"])],
        );
        assert_eq!(
            code(&FIXED_EFFECTS_REGRESSION, &v),
            "fe1 <- fixest::feols(wage ~ hours | id, data = d, cluster = ~firm)\n\
             print(summary(fe1))\nprint(stats::confint(fe1, level = 0.9))"
        );
    }

    #[test]
    fn fixed_effects_regression_checks_its_variables() {
        let v = values(
            &FIXED_EFFECTS_REGRESSION,
            &[("y", "wage"), ("errors", "twoway")],
            &[("x", &["wage", "id"]), ("fe", &["id"])],
        );
        let problems = FIXED_EFFECTS_REGRESSION.problems(&ctx(), &v);
        let widgets: Vec<&str> = problems.iter().map(|p| p.widget).collect();
        assert!(widgets.contains(&"x"), "{problems:?}");
        assert!(widgets.contains(&"fe"), "{problems:?}");
        assert!(widgets.contains(&"errors"), "{problems:?}");
        let v = values(
            &FIXED_EFFECTS_REGRESSION,
            &[("y", "wage"), ("save", "d")],
            &[("x", &["hours"]), ("fe", &["id"])],
        );
        assert!(FIXED_EFFECTS_REGRESSION
            .problems(&ctx(), &v)
            .iter()
            .any(|p| p.widget == "save"));
    }

    #[test]
    fn fixed_effects_glm_writes_fepois_and_feglm() {
        let v = values(
            &FIXED_EFFECTS_GLM,
            &[("y", "visits"), ("exposure", "days")],
            &[("x", &["hours"]), ("fe", &["id"])],
        );
        assert_eq!(
            code(&FIXED_EFFECTS_GLM, &v),
            "model <- fixest::fepois(visits ~ hours | id, data = d, offset = ~log(days), vcov = \"cluster\", \
             ssc = fixest::ssc(adj = FALSE))\n\
             print(summary(model))\ncat(\"\\nRate ratios\\n\")\n\
             print(exp(cbind(rate_ratio = stats::coef(model), stats::confint(model, level = 0.95))))"
        );
        // the exposure is for Poisson only
        let v = values(
            &FIXED_EFFECTS_GLM,
            &[("y", "left"), ("family", "logit"), ("exposure", "days")],
            &[("x", &["hours"]), ("fe", &["firm"])],
        );
        assert!(FIXED_EFFECTS_GLM
            .problems(&ctx(), &v)
            .iter()
            .any(|p| p.widget == "exposure"));
        let mut v = values(
            &FIXED_EFFECTS_GLM,
            &[("y", "left"), ("family", "logit"), ("errors", "iid")],
            &[("x", &["hours"]), ("fe", &["firm"])],
        );
        v.set_flag("ratios", false);
        assert_eq!(
            code(&FIXED_EFFECTS_GLM, &v),
            "model <- fixest::feglm(left ~ hours | firm, data = d, family = \"logit\", vcov = \"iid\", \
             ssc = fixest::ssc(adj = FALSE))\n\
             print(summary(model))\nprint(stats::confint(model, level = 0.95))"
        );
    }

    #[test]
    fn panel_model_writes_plm() {
        let v = values(
            &PANEL_MODEL,
            &[("y", "wage"), ("id", "id"), ("time", "year")],
            &[("x", &["hours"])],
        );
        assert_eq!(
            code(&PANEL_MODEL, &v),
            "model <- plm::plm(wage ~ hours, data = d, index = c(\"id\", \"year\"), model = \"within\", effect = \"individual\")\n\
             print(summary(model))"
        );
        let v = values(
            &PANEL_MODEL,
            &[
                ("y", "wage"),
                ("id", "id"),
                ("time", "year"),
                ("model", "random"),
                ("effect", "twoways"),
                ("errors", "group"),
                ("save", "re"),
            ],
            &[("x", &["hours", "age group"])],
        );
        assert_eq!(
            code(&PANEL_MODEL, &v),
            "re <- plm::plm(wage ~ hours + `age group`, data = d, index = c(\"id\", \"year\"), model = \"random\", effect = \"twoways\")\n\
             print(summary(re, vcov = plm::vcovHC(re, cluster = \"group\", type = \"sss\")))"
        );
        // first differences: the pooled fit of diff(), which goes by the time index
        let v = values(
            &PANEL_MODEL,
            &[
                ("y", "wage"),
                ("id", "id"),
                ("time", "year"),
                ("model", "fd"),
                ("errors", "group"),
            ],
            &[("x", &["hours", "days"])],
        );
        assert_eq!(
            code(&PANEL_MODEL, &v),
            "model <- plm::plm(diff(wage) ~ diff(hours) + diff(days), data = d, index = c(\"id\", \"year\"), model = \"pooling\")\n\
             print(summary(model, vcov = plm::vcovHC(model, cluster = \"group\", type = \"sss\")))"
        );
        // pooled: no effect argument
        let v = values(
            &PANEL_MODEL,
            &[
                ("y", "wage"),
                ("id", "id"),
                ("time", "year"),
                ("model", "pooling"),
            ],
            &[("x", &["hours"])],
        );
        assert!(!code(&PANEL_MODEL, &v).contains("effect ="));
    }

    #[test]
    fn panel_model_checks_its_variables() {
        let v = values(
            &PANEL_MODEL,
            &[
                ("y", "wage"),
                ("id", "year"),
                ("time", "year"),
                ("model", "between"),
                ("effect", "twoways"),
            ],
            &[("x", &["hours"])],
        );
        let problems = PANEL_MODEL.problems(&ctx(), &v);
        let widgets: Vec<&str> = problems.iter().map(|p| p.widget).collect();
        assert!(widgets.contains(&"time"), "{problems:?}");
        assert!(widgets.contains(&"effect"), "{problems:?}");
        // first differences of a categorical predictor
        let v = values(
            &PANEL_MODEL,
            &[
                ("y", "wage"),
                ("id", "id"),
                ("time", "year"),
                ("model", "fd"),
            ],
            &[("x", &["hours", "firm"])],
        );
        assert!(PANEL_MODEL
            .problems(&ctx(), &v)
            .iter()
            .any(|p| p.widget == "x" && p.message.contains("firm")));
        let v = values(
            &PANEL_MODEL,
            &[("y", "wage"), ("id", "hours"), ("time", "year")],
            &[("x", &["hours"])],
        );
        assert!(PANEL_MODEL
            .problems(&ctx(), &v)
            .iter()
            .any(|p| p.widget == "id"));
    }

    #[test]
    fn dynamic_panel_writes_pgmm() {
        let v = values(
            &DYNAMIC_PANEL,
            &[("y", "wage"), ("id", "id"), ("time", "year")],
            &[("x", &["hours"])],
        );
        assert_eq!(
            code(&DYNAMIC_PANEL, &v),
            "model <- local({\n  # pgmm looks for plm() in the frame that calls it\n  plm <- plm::plm\n  \
             plm::pgmm(wage ~ stats::lag(wage, 1) + hours | stats::lag(wage, 2:99), data = d, \
             index = c(\"id\", \"year\"), effect = \"individual\", model = \"twosteps\", transformation = \"d\")\n})\n\
             print(summary(model, robust = TRUE))"
        );
        // two lags, no other predictor, conventional errors
        let v = values(
            &DYNAMIC_PANEL,
            &[
                ("y", "wage"),
                ("id", "id"),
                ("time", "year"),
                ("lags", "2"),
                ("errors", "conventional"),
                ("save", "ab"),
            ],
            &[],
        );
        let out = code(&DYNAMIC_PANEL, &v);
        assert!(
            out.contains("plm::pgmm(wage ~ stats::lag(wage, 1:2) | "),
            "{out}"
        );
        assert!(out.ends_with("print(summary(ab, robust = FALSE))"), "{out}");
        // one step with robust errors prints no Sargan test
        let v = values(
            &DYNAMIC_PANEL,
            &[
                ("y", "wage"),
                ("id", "id"),
                ("time", "year"),
                ("steps", "onestep"),
            ],
            &[("x", &["hours"])],
        );
        let out = code(&DYNAMIC_PANEL, &v);
        assert!(out.contains("model = \"onestep\""), "{out}");
        assert!(out.contains("grepl(\"^Sargan test\""), "{out}");
        let v = values(
            &DYNAMIC_PANEL,
            &[
                ("y", "wage"),
                ("id", "id"),
                ("time", "year"),
                ("steps", "onestep"),
                ("errors", "conventional"),
            ],
            &[("x", &["hours"])],
        );
        // one step with conventional errors scales them as Stata does
        let out = code(&DYNAMIC_PANEL, &v);
        assert!(
            out.contains("ab$vcov <<-") || out.contains("model$vcov <<-"),
            "{out}"
        );
        assert!(DYNAMIC_PANEL.problems(&ctx(), &v).is_empty());
        // a limit on the instrument lags is Stata's maxldep: lags 2 to q + 1
        let mut v = values(
            &DYNAMIC_PANEL,
            &[("y", "wage"), ("id", "id"), ("time", "year")],
            &[],
        );
        v.set_text("maxlag", "2");
        assert!(code(&DYNAMIC_PANEL, &v).contains("stats::lag(wage, 2:3)"));
        // system GMM is base R and takes the limit
        v.set_text("method", "system");
        let out = code(&DYNAMIC_PANEL, &v);
        assert!(
            out.contains("limit <- 2") && out.contains("y <- \"wage\""),
            "{out}"
        );
        assert!(!out.contains("pgmm"), "{out}");
        assert!(DYNAMIC_PANEL.problems(&ctx(), &v).is_empty());
        // the constant belongs to system GMM only
        v.set_flag("constant", true);
        assert!(code(&DYNAMIC_PANEL, &v).contains("cons <- TRUE"));
        assert!(DYNAMIC_PANEL.problems(&ctx(), &v).is_empty());
        v.set_text("method", "difference");
        assert!(DYNAMIC_PANEL
            .problems(&ctx(), &v)
            .iter()
            .any(|p| p.widget == "constant"));
        v.set_flag("constant", false);
        v.set_text("method", "system");
        // system GMM takes time effects as period dummies of its own
        v.set_flag("twoways", true);
        let out = code(&DYNAMIC_PANEL, &v);
        assert!(out.contains("times <- TRUE"), "{out}");
        assert!(DYNAMIC_PANEL.problems(&ctx(), &v).is_empty());
        // time effects use plm's two-way effect
        let mut v = values(
            &DYNAMIC_PANEL,
            &[("y", "wage"), ("id", "id"), ("time", "year")],
            &[],
        );
        v.set_flag("twoways", true);
        let out = code(&DYNAMIC_PANEL, &v);
        assert!(out.contains("effect = \"twoways\""), "{out}");
        // the outcome cannot be a predictor
        let v = values(
            &DYNAMIC_PANEL,
            &[("y", "wage"), ("id", "id"), ("time", "year")],
            &[("x", &["wage"])],
        );
        assert!(DYNAMIC_PANEL
            .problems(&ctx(), &v)
            .iter()
            .any(|p| p.widget == "x"));
    }

    #[test]
    fn panel_iv_writes_the_instrument_formula() {
        let v = values(
            &PANEL_IV,
            &[("y", "wage"), ("id", "id"), ("time", "year")],
            &[
                ("x", &["hours"]),
                ("endog", &["days"]),
                ("inst", &["visits"]),
            ],
        );
        let out = code(&PANEL_IV, &v);
        assert!(
            out.starts_with(
                "model <- plm::plm(wage ~ hours + days | hours + visits, data = d, index = c(\"id\", \"year\"), model = \"within\")\n\
                 print(summary(model))\n"
            ),
            "{out}"
        );
        // the within fit gets the constant, with conventional errors
        assert!(out.contains("m <- model\n  robust <- FALSE"), "{out}");
        // the others print the summary only, unless robust errors are asked for
        let mut v = values(
            &PANEL_IV,
            &[
                ("y", "wage"),
                ("id", "id"),
                ("time", "year"),
                ("model", "fd"),
            ],
            &[("endog", &["days"]), ("inst", &["visits"])],
        );
        assert!(code(&PANEL_IV, &v).ends_with("model = \"fd\")\nprint(summary(model))"));
        v.set_text("errors", "robust");
        assert!(code(&PANEL_IV, &v).contains("robust <- TRUE"));
        // EC2SLS is Baltagi's random-effects estimator; random effects are
        // computed directly (Stata's components, instruments and cluster sandwich)
        let mut v = values(
            &PANEL_IV,
            &[
                ("y", "wage"),
                ("id", "id"),
                ("time", "year"),
                ("model", "ec2sls"),
            ],
            &[("endog", &["days"]), ("inst", &["visits"])],
        );
        let c = code(&PANEL_IV, &v);
        assert!(
            c.contains("ec2sls <- TRUE; nosa <- FALSE; robust <- FALSE; small <- FALSE")
                && !c.contains("plm::plm"),
            "{c}"
        );
        assert!(PANEL_IV.problems(&ctx(), &v).is_empty());
        v.set_text("errors", "robust");
        v.set_flag("small", true);
        assert!(PANEL_IV.problems(&ctx(), &v).is_empty());
        let c = code(&PANEL_IV, &v);
        assert!(
            c.contains("ec2sls <- TRUE; nosa <- FALSE; robust <- TRUE; small <- TRUE")
                && !c.contains("plm::plm"),
            "{c}"
        );
        v.set_flag("small", false);
        // Baltagi-Chang components and the instruments ignored, random effects
        v.set_text("errors", "conventional");
        v.set_text("model", "random");
        v.set_flag("nosa", true);
        let c = code(&PANEL_IV, &v);
        assert!(
            c.contains("ec2sls <- FALSE; nosa <- TRUE; robust <- FALSE"),
            "{c}"
        );
        assert!(c.contains("stats::model.matrix(~ visits, d)"), "{c}");
        v.set_flag("nosa", false);
        v.set_flag("regress", true);
        let c = code(&PANEL_IV, &v);
        assert!(c.contains("stats::model.matrix(~ days, d)[, -1, drop = FALSE]\n  Z0 <- stats::model.matrix(~ days, d)"), "{c}");
        v.set_text("model", "within");
        assert!(code(&PANEL_IV, &v).starts_with("model <- plm::plm(wage ~ days, data = d"));
        v.set_text("model", "fd");
        assert!(PANEL_IV
            .problems(&ctx(), &v)
            .iter()
            .any(|p| p.widget == "regress"));
        v.set_flag("regress", false);
        v.set_flag("nosa", true);
        assert!(PANEL_IV
            .problems(&ctx(), &v)
            .iter()
            .any(|p| p.widget == "nosa"));
        v.set_flag("nosa", false);
        v.set_text("errors", "robust");
        v.set_text("model", "between");
        assert!(PANEL_IV
            .problems(&ctx(), &v)
            .iter()
            .any(|p| p.widget == "errors"));
        let v = values(
            &PANEL_IV,
            &[
                ("y", "wage"),
                ("id", "id"),
                ("time", "year"),
                ("model", "random"),
                ("save", "iv"),
            ],
            &[("endog", &["days", "hours"]), ("inst", &["visits", "left"])],
        );
        let c = code(&PANEL_IV, &v);
        assert!(
            c.starts_with("iv <- local({")
                && c.contains("X0 <- stats::model.matrix(~ days + hours, d)")
                && c.contains("Z0 <- stats::model.matrix(~ visits + left, d)"),
            "{c}"
        );
        let v = values(
            &PANEL_IV,
            &[
                ("y", "wage"),
                ("id", "id"),
                ("time", "year"),
                ("model", "between"),
                ("save", "iv"),
            ],
            &[("endog", &["days", "hours"]), ("inst", &["visits", "left"])],
        );
        assert!(code(&PANEL_IV, &v)
            .starts_with("iv <- plm::plm(wage ~ days + hours | visits + left, data = d, "));
        // fewer instruments than endogenous predictors, an instrument already in the model
        let v = values(
            &PANEL_IV,
            &[("y", "wage"), ("id", "id"), ("time", "year")],
            &[
                ("x", &["hours"]),
                ("endog", &["days", "left"]),
                ("inst", &["hours"]),
            ],
        );
        let problems = PANEL_IV.problems(&ctx(), &v);
        let messages: Vec<&str> = problems.iter().map(|p| p.message.as_str()).collect();
        assert!(
            messages.iter().any(|m| m.contains("at least as many")),
            "{messages:?}"
        );
        assert!(
            messages.iter().any(|m| m.contains("left out of it")),
            "{messages:?}"
        );
    }

    #[test]
    fn panel_unit_root_writes_purtest() {
        let v = values(
            &PANEL_UNIT_ROOT,
            &[("var", "wage"), ("id", "id"), ("time", "year")],
            &[],
        );
        let out = code(&PANEL_UNIT_ROOT, &v);
        assert!(
            out.contains(
                "plm::purtest(panel[[\"wage\"]], test = \"levinlin\", exo = \"intercept\", lags = 1, \
                 q = floor(3.21 * periods^(1/3)))"
            ),
            "{out}"
        );
        assert!(
            out.contains("plm::pdata.frame(d, index = c(\"id\", \"year\"))"),
            "{out}"
        );
        let v = values(
            &PANEL_UNIT_ROOT,
            &[
                ("var", "wage"),
                ("id", "id"),
                ("time", "year"),
                ("test", "ips"),
                ("terms", "trend"),
                ("lags", "2"),
            ],
            &[],
        );
        // Im-Pesaran-Shin is base R with the tabulated moments for these lags and the trend
        let out = code(&PANEL_UNIT_ROOT, &v);
        assert!(out.contains("lags <- 2\n  trend <- TRUE"), "{out}");
        assert!(
            out.contains("c(NA, -1.823, -1.911") || out.contains("c(-1.914, -1.999, -2.047"),
            "{out}"
        );
        assert!(out.contains("panel[[\"wage\"]]"), "{out}");
        assert!(!out.contains("purtest"), "{out}");
        // Im-Pesaran-Shin needs a mean; the variable is not the unit or the time
        let v = values(
            &PANEL_UNIT_ROOT,
            &[
                ("var", "id"),
                ("id", "id"),
                ("time", "year"),
                ("test", "ips"),
                ("terms", "none"),
            ],
            &[],
        );
        let widgets: Vec<&str> = PANEL_UNIT_ROOT
            .problems(&ctx(), &v)
            .iter()
            .map(|p| p.widget)
            .collect();
        assert!(
            widgets.contains(&"terms") && widgets.contains(&"id"),
            "{widgets:?}"
        );
    }

    #[test]
    fn panel_serial_correlation_is_base_r() {
        let v = values(
            &PANEL_SERIAL,
            &[("y", "wage"), ("id", "firm"), ("time", "year")],
            &[("x", &["hours", "days"])],
        );
        let out = code(&PANEL_SERIAL, &v);
        assert!(
            out.contains("dx <- cbind(dif(d0[[\"hours\"]]), dif(d0[[\"days\"]]))"),
            "{out}"
        );
        assert!(
            out.contains("stats::pf(f, 1, n - 1, lower.tail = FALSE)"),
            "{out}"
        );
        assert!(
            out.contains("data.name = \"wage on hours and days, clustered by firm\""),
            "{out}"
        );
        assert!(PANEL_SERIAL.packages(Style::Base).is_empty());
        // the unit and the time must differ from the model's variables
        let v = values(
            &PANEL_SERIAL,
            &[("y", "wage"), ("id", "hours"), ("time", "hours")],
            &[("x", &["hours"])],
        );
        let widgets: Vec<&str> = PANEL_SERIAL
            .problems(&ctx(), &v)
            .iter()
            .map(|p| p.widget)
            .collect();
        assert!(
            widgets.contains(&"id") && widgets.contains(&"time"),
            "{widgets:?}"
        );
    }

    #[test]
    fn hausman_fits_both_and_tests() {
        let v = values(
            &HAUSMAN,
            &[("y", "wage"), ("id", "id"), ("time", "year")],
            &[("x", &["hours"])],
        );
        assert_eq!(
            code(&HAUSMAN, &v),
            "print(local({\n  fixed <- plm::plm(wage ~ hours, data = d, index = c(\"id\", \"year\"), model = \"within\")\n  \
             random <- plm::plm(wage ~ hours, data = d, index = c(\"id\", \"year\"), model = \"random\")\n  \
             plm::phtest(fixed, random)\n}))"
        );
    }
}
