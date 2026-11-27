//! Statistics menu, instrumental variables and selection models (P6.6):
//! two-stage least squares through `fixest` (with fixed effects if asked, its
//! first-stage F, Wu-Hausman and Sargan statistics), or LIML and two-step GMM
//! written as matrix algebra in plain R, as Stata's `ivregress liml` and
//! `ivregress gmm`; IV probit and tobit by maximum likelihood with `optim`, as
//! `ivprobit` and `ivtobit`; treatment effects by regression adjustment,
//! inverse-probability weighting or both, as `teffects ra`, `ipw` and `aipw`;
//! and Heckman's selection model through `sampleSelection`, by maximum
//! likelihood or in two steps. Each fits its model, keeps it in the workspace
//! under the name the dialog asks for and prints its results. None changes
//! the data.

use rcraft_rcodegen::dialog::{Args, Opt, Problem, Spec, TextDefault, TextRule};
use rcraft_rcodegen::dialog::{VarKind, Widget};
use rcraft_rcodegen::{r_name, r_string};

use crate::data::BASE;

const NUMBERS: &[VarKind] = &[VarKind::Number];
const SELECTED: &[VarKind] = &[VarKind::Number, VarKind::Logical];
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

const SAVE: Widget = Widget::Text {
    id: "save",
    label: "Save the model as",
    default: TextDefault::Fixed("model"),
    rule: TextRule::Name,
    hint: "The fitted model stays in R's workspace under this name; a name already in use is \
           replaced.",
};

fn joined(names: &[&str]) -> String {
    names
        .iter()
        .map(|n| r_name(n))
        .collect::<Vec<_>>()
        .join(" + ")
}

/// A formula's right side: the terms, or `1` when there are none.
fn rhs(names: &[&str]) -> String {
    if names.is_empty() {
        "1".to_string()
    } else {
        joined(names)
    }
}

/// The columns as an R character vector.
fn strings(names: &[&str]) -> String {
    let quoted: Vec<String> = names.iter().map(|n| r_string(n)).collect();
    format!("c({})", quoted.join(", "))
}

/// The lines (inside a `local` block) that keep the rows complete in every
/// column the model uses, as `k`.
fn complete_rows(data: &str, columns: &[&str]) -> String {
    format!(
        "  k <- {data}[stats::complete.cases({data}[{}]), ]\n",
        strings(columns)
    )
}

/// The coefficient table and confidence intervals of the estimates `b` with
/// covariance `V`: t tests on `N - K` degrees of freedom when `small`, else
/// z tests. Leaves `se` and `ci` defined.
fn coef_table(small: bool, conf: &str) -> String {
    let (stat, q) = if small {
        (
            "`t value` = s, `Pr(>|t|)` = 2 * stats::pt(-abs(s), N - K)",
            format!("stats::qt((1 + {conf}) / 2, N - K)"),
        )
    } else {
        (
            "`z value` = s, `Pr(>|z|)` = 2 * stats::pnorm(-abs(s))",
            format!("stats::qnorm((1 + {conf}) / 2)"),
        )
    };
    format!(
        "  se <- sqrt(diag(V)); s <- b / se\n  \
         stats::printCoefmat(cbind(Estimate = b, `Std. Error` = se, {stat}))\n  \
         q <- {q}\n  \
         ci <- cbind(b - q * se, b + q * se)\n  \
         colnames(ci) <- paste(100 * c(1 - {conf}, 1 + {conf}) / 2, \"%\")\n  \
         cat(\"\\n\")\n  print(ci)\n"
    )
}

/// Each variable in one role only, and the model not saved over the data.
fn roles(
    a: &Args,
    lists: &[(&'static str, &str)],
    single: &[(&'static str, &str)],
) -> Vec<Problem> {
    let mut out = Vec::new();
    let mut seen: Vec<(String, &str)> = Vec::new();
    for (id, what) in single {
        let v = a.var_text(id);
        if v.is_empty() {
            continue;
        }
        if let Some((_, first)) = seen.iter().find(|(s, _)| s == v) {
            out.push(Problem::new(
                id,
                format!("{v} is already the {first}; choose another variable."),
            ));
        } else {
            seen.push((v.to_string(), what));
        }
    }
    for (id, what) in lists {
        for v in a.vars(id) {
            if let Some((_, first)) = seen.iter().find(|(s, _)| s == v) {
                out.push(Problem::new(
                    id,
                    format!("{v} is already the {first}; each variable takes one role."),
                ));
            } else {
                seen.push((v.to_string(), what));
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

// ---- Two-stage least squares ----

const IV_ERRORS: &[Opt] = &[
    Opt {
        value: "iid",
        label: "Conventional",
    },
    Opt {
        value: "hetero",
        label: "Robust (the spread of the errors may vary)",
    },
];

/// The two-stage dialog's choice: the estimator's own default first (conventional
/// for two-stage least squares and LIML, robust for GMM, as Stata's ivregress).
const IV_ERRORS_BY_ESTIMATOR: &[Opt] = &[
    Opt {
        value: "default",
        label: "Stata's default for the estimator (robust for GMM, else conventional)",
    },
    Opt {
        value: "iid",
        label: "Conventional",
    },
    Opt {
        value: "hetero",
        label: "Robust (the spread of the errors may vary)",
    },
];

/// The errors in effect: the choice, with "default" resolved by the estimator.
fn iv_errors<'a>(a: &'a Args) -> &'a str {
    match a.choice("errors") {
        "default" if a.choice("estimator") == "gmm" => "hetero",
        "default" => "iid",
        other => other,
    }
}

fn iv_check(a: &Args) -> Vec<Problem> {
    let mut out = roles(
        a,
        &[
            ("endog", "an endogenous predictor"),
            ("instruments", "an instrument"),
            ("x", "an exogenous predictor"),
            ("fe", "a fixed effect"),
        ],
        &[("y", "outcome")],
    );
    let (endog, instruments) = (a.vars("endog").len(), a.vars("instruments").len());
    if instruments < endog {
        out.push(Problem::new(
            "instruments",
            format!(
                "{endog} endogenous predictors need at least {endog} instruments; there are \
                 {instruments}."
            ),
        ));
    }
    if a.choice("estimator") != "2sls" && !a.vars("fe").is_empty() {
        out.push(Problem::new(
            "fe",
            "LIML and GMM take no fixed effects here; choose two-stage least squares, or add \
             the groups as exogenous predictors.",
        ));
    }
    out
}

const ESTIMATORS: &[Opt] = &[
    Opt {
        value: "2sls",
        label: "Two-stage least squares",
    },
    Opt {
        value: "liml",
        label: "Limited-information maximum likelihood (LIML)",
    },
    Opt {
        value: "gmm",
        label: "Two-step GMM",
    },
];

/// LIML or two-step GMM, as matrix algebra in plain R: Stata's `ivregress
/// liml` and `ivregress gmm` (whose weights are robust, or clustered when a
/// cluster is chosen), with its conventional, robust or clustered variance.
fn iv_matrix(a: &Args) -> String {
    let model = a.name("save");
    let data = a.data();
    let y = a.var_text("y");
    let (endog, insts, x) = (a.vars("endog"), a.vars("instruments"), a.vars("x"));
    let cluster = a.var_text("cluster");
    let mut columns = vec![y];
    columns.extend(&endog);
    columns.extend(&insts);
    columns.extend(&x);
    if !cluster.is_empty() {
        columns.push(cluster);
    }
    let small = a.flag("small");
    let instruments = joined(&[insts.as_slice(), x.as_slice()].concat());
    let mut code = format!(
        "{model} <- local({{\n{}  y <- k[[{}]]\n  \
         X <- stats::model.matrix(~ {}, k)\n  \
         Z <- stats::model.matrix(~ {instruments}, k)\n  \
         N <- nrow(X); K <- ncol(X); L <- ncol(Z)\n  \
         # the regressors projected on the instruments\n  \
         Xh <- X - qr.resid(qr(Z), X)\n",
        complete_rows(&data, &columns),
        r_string(y),
        joined(&[endog.as_slice(), x.as_slice()].concat()),
    );
    if !cluster.is_empty() {
        code.push_str(&format!(
            "  cl <- k[[{}]]; G <- length(unique(cl))\n",
            r_string(cluster)
        ));
    }
    if a.flag("first") {
        for en in &endog {
            code.push_str(&format!(
                "  print(summary(stats::lm({} ~ {instruments}, data = k)))\n",
                r_name(en)
            ));
        }
    }
    let errors = if cluster.is_empty() {
        iv_errors(a)
    } else {
        "cluster"
    };
    let factor = match (small, errors) {
        (false, _) => "",
        (true, "cluster") => " * G / (G - 1) * (N - 1) / (N - K)",
        (true, _) => " * N / (N - K)",
    };
    if a.choice("estimator") == "liml" {
        let variance = match errors {
            "cluster" => format!("A %*% crossprod(rowsum(Xh * e, cl)) %*% A{factor}"),
            "hetero" => format!("A %*% crossprod(Xh * e) %*% A{factor}"),
            _ if small => "A * sum(e^2) / (N - K)".to_string(),
            _ => "A * sum(e^2) / N".to_string(),
        };
        code.push_str(&format!(
            "  # kappa is the smallest eigenvalue of (Y'M Y)^-1 Y'M1 Y: Y the outcome and the\n  \
             # endogenous predictors, M and M1 what is left after all the instruments and\n  \
             # after the exogenous predictors alone\n  \
             Y <- cbind(y, as.matrix(k[{}]))\n  \
             X1 <- stats::model.matrix(~ {}, k)\n  \
             kappa <- min(Re(eigen(solve(crossprod(qr.resid(qr(Z), Y)), \
             crossprod(qr.resid(qr(X1), Y))), only.values = TRUE)$values))\n  \
             Xk <- X - kappa * qr.resid(qr(Z), X)\n  \
             A <- solve(crossprod(Xk, X))\n  \
             b <- drop(A %*% crossprod(Xk, y))\n  \
             e <- drop(y - X %*% b)\n  \
             V <- {variance}\n  \
             cat(\"Limited-information maximum likelihood, kappa = \", format(kappa, digits = 10), \
             \", \", N, \" observations\\n\\n\", sep = \"\")\n",
            strings(&endog),
            rhs(&x),
        ));
        code.push_str(&coef_table(small, a.choice("conf")));
        code.push_str(
            "  list(coefficients = b, vcov = V, residuals = e, nobs = N, kappa = kappa)\n})",
        );
    } else {
        let (meat, weights) = if errors == "cluster" {
            ("crossprod(rowsum(Z * e, cl)) / N", "clustered")
        } else {
            ("crossprod(Z * e) / N", "robust")
        };
        let variance = if errors == "iid" {
            format!("N * B{factor}")
        } else {
            format!("N * B %*% t(ZX) %*% W %*% S(e) %*% W %*% ZX %*% B{factor}")
        };
        code.push_str(&format!(
            "  # step one: two-stage least squares, whose residuals weight the moments\n  \
             e <- drop(y - X %*% solve(crossprod(Xh, X), crossprod(Xh, y)))\n  \
             S <- function(e) {meat}\n  \
             W <- solve(S(e))\n  \
             # step two: GMM with those weights\n  \
             ZX <- crossprod(Z, X)\n  \
             B <- solve(t(ZX) %*% W %*% ZX)\n  \
             b <- drop(B %*% t(ZX) %*% W %*% crossprod(Z, y))\n  \
             e <- drop(y - X %*% b)\n  \
             V <- {variance}\n  \
             cat(\"Two-step GMM with {weights} weights, \", N, \" observations\\n\\n\", sep = \"\")\n"
        ));
        code.push_str(&coef_table(small, a.choice("conf")));
        code.push_str(
            "  # Hansen's J test of the over-identifying restrictions, with the first step's weights\n  \
             m <- colMeans(Z * e)\n  \
             J <- if (L > K) N * drop(t(m) %*% W %*% m) else NA\n  \
             if (L > K) cat(\"\\nHansen's J = \", format(J, digits = 7), \", chi2(\", L - K, \"), p = \", \
             format(stats::pchisq(J, L - K, lower.tail = FALSE), digits = 4), \"\\n\", sep = \"\")\n  \
             list(coefficients = b, vcov = V, residuals = e, nobs = N, J = J)\n})",
        );
    }
    code
}

pub const IV_REGRESSION: Spec = Spec {
    entry: "Two-stage least squares",
    menu: "Statistics > Instrumental variables > Two-stage least squares",
    about: "A linear model in which some predictors are endogenous (correlated with the error), \
            estimated by two-stage least squares with instruments that move them but not the \
            outcome otherwise, as Stata's ivregress 2sls. It fits with fixest::feols, optionally \
            with fixed effects absorbed (as xtivreg, fe), and prints the summary with the \
            first-stage F, the Wu-Hausman test of endogeneity and Sargan's test of the \
            over-identifying restrictions. It can instead estimate by limited-information \
            maximum likelihood or by two-step GMM, written out as matrix algebra in plain R \
            (Stata's ivregress liml and ivregress gmm), with Hansen's J test after GMM.",
    widgets: &[
        Widget::Choice {
            id: "estimator",
            label: "Estimator",
            options: ESTIMATORS,
            default: "2sls",
            hint: "LIML: k-class with k the smallest eigenvalue, less biased with weak \
                   instruments. GMM: two steps, the moments weighted by the first step's robust \
                   (or, with a cluster, clustered) residuals. Standard errors are robust unless you choose \
                   Conventional (Stata's vce(unadjusted)).",
        },
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
            hint: "The predictors to instrument.",
        },
        Widget::Variables {
            id: "instruments",
            label: "Instruments",
            kinds: PREDICTORS,
            min: 1,
            hint: "Excluded instruments: variables that move the endogenous predictors but not \
                   the outcome except through them. At least one for each endogenous predictor.",
        },
        Widget::Variables {
            id: "x",
            label: "Exogenous predictors",
            kinds: PREDICTORS,
            min: 0,
            hint: "Optional: the other predictors. They are used as their own instruments.",
        },
        Widget::Variables {
            id: "fe",
            label: "Fixed effects",
            kinds: GROUPS,
            min: 0,
            hint: "Optional: groupings whose intercepts are absorbed, as xtivreg, fe.",
        },
        Widget::Choice {
            id: "errors",
            label: "Standard errors",
            options: IV_ERRORS_BY_ESTIMATOR,
            default: "default",
            hint: "A variable under Cluster by takes the place of this choice. Left at Stata's \
                   default, GMM uses robust errors (as ivregress gmm) and the other estimators \
                   conventional ones.",
        },
        Widget::Variable {
            id: "cluster",
            label: "Cluster by",
            kinds: GROUPS,
            required: false,
            hint: "Optional: cluster the standard errors by this variable.",
        },
        Widget::Check {
            id: "small",
            label: "Small-sample statistics",
            default: true,
            hint:
                "On: Stata's ivregress with the small option (and xtivreg, fe): degrees-of-freedom \
                   adjustments and t tests. Off: ivregress's default, z tests and robust or \
                   clustered errors without the small-sample factors; the conventional errors \
                   of two-stage least squares then differ a little from Stata's, as fixest \
                   divides by N - 1 where Stata divides by N (LIML and GMM divide by N, as \
                   Stata).",
        },
        Widget::Check {
            id: "first",
            label: "Show the first stage",
            default: false,
            hint: "Also prints the regression of each endogenous variable on the instruments and the other predictors, to judge how strong the instruments are.",
        },
        Widget::Choice {
            id: "conf",
            label: "Confidence level",
            options: LEVELS,
            default: "0.95",
            hint: "For the confidence intervals printed after the summary.",
        },
        SAVE,
    ],
    needs: &["fixest"],
    styles: BASE,
    check: iv_check,
    code: |a, _| {
        if a.choice("estimator") != "2sls" {
            return iv_matrix(a);
        }
        let model = a.name("save");
        let exog = match a.vars("x") {
            x if x.is_empty() => "1".to_string(),
            x => joined(&x),
        };
        let fe = match a.vars("fe") {
            fe if fe.is_empty() => String::new(),
            fe => format!(" | {}", joined(&fe)),
        };
        let errors = match a.var("cluster") {
            Some(c) => format!("cluster = ~{c}"),
            None => format!("vcov = {}", r_string(iv_errors(a))),
        };
        let ssc = if a.flag("small") {
            ""
        } else {
            ", ssc = fixest::ssc(adj = FALSE, cluster.adj = FALSE)"
        };
        let stage = if a.flag("first") { ", stage = 1:2" } else { "" };
        let mut code = format!(
            "{model} <- fixest::feols({} ~ {exog}{fe} | {} ~ {}, data = {}, {errors}{ssc})\n\
             print(summary({model}{stage}))",
            a.var("y").unwrap_or_default(),
            joined(&a.vars("endog")),
            joined(&a.vars("instruments")),
            a.data(),
        );
        // the summary's first-stage F is the conventional one; with robust or
        // clustered errors Stata's estat firststage reports the Wald form
        if a.var("cluster").is_some() || iv_errors(a) != "iid" {
            code.push_str(&format!("\nprint(fixest::fitstat({model}, ~ ivwald))"));
        }
        code.push_str(&format!(
            "\nprint(stats::confint({model}, level = {}))",
            a.choice("conf")
        ));
        code
    },
};

// ---- Heckman selection model ----

const METHODS: &[Opt] = &[
    Opt {
        value: "ml",
        label: "Maximum likelihood",
    },
    Opt {
        value: "2step",
        label: "Two steps (Heckman's)",
    },
];

fn heckman_check(a: &Args) -> Vec<Problem> {
    roles(
        a,
        &[],
        &[("y", "outcome"), ("selected", "selection indicator")],
    )
    .into_iter()
    .chain(a.vars("x").contains(&a.var_text("y")).then(|| {
        Problem::new(
            "x",
            format!(
                "{} is the outcome; it cannot also be a predictor.",
                a.var_text("y")
            ),
        )
    }))
    .chain(a.vars("z").contains(&a.var_text("y")).then(|| {
        Problem::new(
            "z",
            format!(
                "{} is the outcome; it cannot also be a predictor.",
                a.var_text("y")
            ),
        )
    }))
    .collect()
}

pub const HECKMAN: Spec = Spec {
    entry: "Heckman selection model",
    menu: "Statistics > Selection models > Heckman selection model",
    about: "A linear model of an outcome that is seen only for some rows (wages only for those in \
            work), with a probit model of which rows are seen, estimated jointly so that the \
            selection does not bias the coefficients: Heckman's model, as Stata's heckman. It \
            fits with sampleSelection::selection, by maximum likelihood (Stata's default, fitted \
            to Stata's precision) or in Heckman's two steps, and prints the summary with rho, \
            the correlation of the two errors.",
    widgets: &[
        Widget::Variable {
            id: "y",
            label: "Outcome",
            kinds: NUMBERS,
            required: true,
            hint: "Seen only for the selected rows; its value elsewhere is not used.",
        },
        Widget::Variables {
            id: "x",
            label: "Outcome predictors",
            kinds: PREDICTORS,
            min: 1,
            hint: "",
        },
        Widget::Variable {
            id: "selected",
            label: "Selected (1 or TRUE)",
            kinds: SELECTED,
            required: false,
            hint: "Optional: which rows are seen. Left empty, a row is selected when its outcome \
                   is not missing, as Stata's heckman does.",
        },
        Widget::Variables {
            id: "z",
            label: "Selection predictors",
            kinds: PREDICTORS,
            min: 1,
            hint: "What decides whether a row is seen. Include at least one variable that is not \
                   an outcome predictor, or the model rests on the normal curve's shape alone.",
        },
        Widget::Choice {
            id: "method",
            label: "Estimation",
            options: METHODS,
            default: "ml",
            hint: "",
        },
        SAVE,
    ],
    needs: &["sampleSelection"],
    styles: BASE,
    check: heckman_check,
    code: |a, _| {
        let model = a.name("save");
        let y = a.var("y").unwrap_or_default();
        let selected = a.var("selected").unwrap_or_else(|| format!("!is.na({y})"));
        // maxLik's default stopping rule (the gradient below 1e-6) leaves the
        // estimates about 1e-8 from the maximum; these reach it, as Stata does
        let method = if a.choice("method") == "ml" {
            "method = \"ml\", gradtol = 1e-12, reltol = 1e-16, tol = 1e-14"
        } else {
            "method = \"2step\""
        };
        format!(
            "suppressPackageStartupMessages(requireNamespace(\"sampleSelection\", quietly = TRUE))\n\
             {model} <- sampleSelection::selection({selected} ~ {}, {y} ~ {}, data = {}, {method})\n\
             print(summary({model}))",
            joined(&a.vars("z")),
            joined(&a.vars("x")),
            a.data(),
        )
    },
};

// ---- IV probit or tobit ----

const IV_ML_MODELS: &[Opt] = &[
    Opt {
        value: "probit",
        label: "Probit (a 0/1 outcome)",
    },
    Opt {
        value: "tobit",
        label: "Tobit (an outcome censored at a limit)",
    },
];

/// A limit box: `None` when empty, else the number or why it is not one.
fn limit(text: &str) -> Option<Result<f64, String>> {
    let t = text.trim();
    if t.is_empty() {
        return None;
    }
    Some(
        t.parse::<f64>()
            .ok()
            .filter(|v| v.is_finite())
            .ok_or_else(|| format!("{t} is not a number.")),
    )
}

fn iv_ml_check(a: &Args) -> Vec<Problem> {
    let mut out = roles(
        a,
        &[
            ("instruments", "an instrument"),
            ("x", "an exogenous predictor"),
        ],
        &[("y", "outcome"), ("endog", "endogenous predictor")],
    );
    if a.choice("model") == "tobit" {
        let (lower, upper) = (limit(a.values.text("lower")), limit(a.values.text("upper")));
        for (id, l) in [("lower", &lower), ("upper", &upper)] {
            if let Some(Err(why)) = l {
                out.push(Problem::new(id, why.clone()));
            }
        }
        match (lower, upper) {
            (None, None) => out.push(Problem::new(
                "lower",
                "A tobit model needs a lower limit, an upper limit or both.",
            )),
            (Some(Ok(l)), Some(Ok(u))) if l >= u => out.push(Problem::new(
                "upper",
                "The upper limit must be above the lower one.",
            )),
            _ => {}
        }
    }
    out
}

pub const IV_ML: Spec = Spec {
    entry: "IV probit or tobit",
    menu: "Statistics > Instrumental variables > IV probit or tobit",
    about: "A probit model of a 0/1 outcome, or a tobit model of an outcome censored at a limit, \
            with one endogenous predictor, estimated with its instruments by maximum likelihood \
            on the joint density of the outcome and the endogenous predictor, as Stata's \
            ivprobit and ivtobit. The likelihood and its derivatives are written out in plain R \
            and maximised by optim; it prints the outcome equation, the endogenous predictor's \
            regression on the instruments, the correlation of the two errors and their standard \
            deviations, and the Wald test of exogeneity (that the correlation is zero).",
    widgets: &[
        Widget::Choice {
            id: "model",
            label: "Model",
            options: IV_ML_MODELS,
            default: "probit",
            hint: "",
        },
        Widget::Variable {
            id: "y",
            label: "Outcome",
            kinds: SELECTED,
            required: true,
            hint: "Probit: any value other than 0 (or TRUE) counts as 1, as in Stata.",
        },
        Widget::Variable {
            id: "endog",
            label: "Endogenous predictor",
            kinds: NUMBERS,
            required: true,
            hint: "The continuous predictor to instrument.",
        },
        Widget::Variables {
            id: "instruments",
            label: "Instruments",
            kinds: PREDICTORS,
            min: 1,
            hint: "Excluded instruments: they move the endogenous predictor but not the outcome \
                   except through it.",
        },
        Widget::Variables {
            id: "x",
            label: "Exogenous predictors",
            kinds: PREDICTORS,
            min: 0,
            hint: "Optional: the other predictors, in both equations.",
        },
        Widget::Text {
            id: "lower",
            label: "Tobit: lower limit",
            default: TextDefault::Fixed("0"),
            rule: TextRule::Any,
            hint: "Outcomes at or below it are censored there, as ivtobit's ll(). Empty for none.",
        },
        Widget::Text {
            id: "upper",
            label: "Tobit: upper limit",
            default: TextDefault::Empty,
            rule: TextRule::Any,
            hint: "Outcomes at or above it are censored there, as ul(). Empty for none.",
        },
        Widget::Choice {
            id: "errors",
            label: "Standard errors",
            options: IV_ERRORS,
            default: "iid",
            hint: "Conventional: from the observed information. Robust: the sandwich, times \
                   N / (N - 1) as Stata. A variable under Cluster by takes the place of this \
                   choice.",
        },
        Widget::Variable {
            id: "cluster",
            label: "Cluster by",
            kinds: GROUPS,
            required: false,
            hint: "Optional: cluster the standard errors by this variable.",
        },
        Widget::Choice {
            id: "conf",
            label: "Confidence level",
            options: LEVELS,
            default: "0.95",
            hint: "",
        },
        SAVE,
    ],
    needs: &[],
    styles: BASE,
    check: iv_ml_check,
    code: |a, _| {
        let model = a.name("save");
        let data = a.data();
        let tobit = a.choice("model") == "tobit";
        let (y, endog) = (a.var_text("y"), a.var_text("endog"));
        let (insts, x) = (a.vars("instruments"), a.vars("x"));
        let cluster = a.var_text("cluster");
        let mut columns = vec![y, endog];
        columns.extend(&insts);
        columns.extend(&x);
        if !cluster.is_empty() {
            columns.push(cluster);
        }
        let bound = |id: &str, none: &str| match limit(a.values.text(id)) {
            Some(Ok(_)) => a.values.text(id).trim().to_string(),
            _ => none.to_string(),
        };
        let (lower, upper) = (bound("lower", "-Inf"), bound("upper", "Inf"));
        let mut code = format!(
            "{model} <- local({{\n{}  y <- k[[{}]]; v <- k[[{}]]; N <- nrow(k)\n  \
             X <- stats::model.matrix(~ {}, k)\n  \
             Z <- stats::model.matrix(~ {}, k)\n  \
             px <- ncol(X); pz <- ncol(Z)\n  \
             # start: the endogenous predictor's regression on the instruments, then the\n  \
             # outcome's on the predictors and that regression's residuals\n  \
             gam <- stats::lm.fit(Z, v)$coefficients; r <- v - drop(Z %*% gam)\n",
            complete_rows(&data, &columns),
            r_string(y),
            r_string(endog),
            joined(&[&[endog][..], x.as_slice()].concat()),
            joined(&[insts.as_slice(), x.as_slice()].concat()),
        );
        if tobit {
            code.push_str(&format!(
                "  lo <- y <= {lower}; hi <- y >= {upper}; mid <- !lo & !hi\n  \
                 b <- stats::lm.fit(cbind(X, r), y)$coefficients[seq_len(px)]\n  \
                 start <- c(b, gam, 0, log(stats::sd(y - drop(X %*% b))), log(stats::sd(r)))\n  \
                 # each row's log-likelihood and its derivatives in (beta, gamma, atanh(rho),\n  \
                 # log sigma, log sigma of the endogenous predictor): the predictor's normal\n  \
                 # density times the outcome's censored normal given the predictor's error\n  \
                 obs <- function(th) {{\n    \
                 b <- th[seq_len(px)]; g <- th[px + seq_len(pz)]; rho <- tanh(th[px + pz + 1])\n    \
                 s1 <- exp(th[px + pz + 2]); ls <- th[px + pz + 3]; sr <- sqrt(1 - rho^2)\n    \
                 u <- (v - drop(Z %*% g)) / exp(ls); m <- drop(X %*% b) + rho * s1 * u; s <- s1 * sr\n    \
                 z <- (y - m) / s; zl <- ({lower} - m) / s; zu <- ({upper} - m) / s\n    \
                 ly <- ifelse(mid, stats::dnorm(z, log = TRUE) - log(s), ifelse(lo, \
                 stats::pnorm(zl, log.p = TRUE), stats::pnorm(zu, lower.tail = FALSE, log.p = TRUE)))\n    \
                 dm <- ifelse(mid, z / s, ifelse(lo, -exp(stats::dnorm(zl, log = TRUE) - \
                 stats::pnorm(zl, log.p = TRUE)) / s, exp(stats::dnorm(zu, log = TRUE) - \
                 stats::pnorm(zu, lower.tail = FALSE, log.p = TRUE)) / s))\n    \
                 ds <- ifelse(mid, (z^2 - 1) / s, ifelse(lo, dm * zl, dm * zu))\n    \
                 list(ll = stats::dnorm(u, log = TRUE) - ls + ly, sc = cbind(X * dm, \
                 Z * ((u - dm * rho * s1) / exp(ls)), (dm * u * sr - ds * rho) * s1 * sr, \
                 dm * rho * s1 * u + ds * s, -1 + u^2 - dm * rho * s1 * u))\n  }}\n"
            ));
        } else {
            code.push_str(
                "  sgn <- ifelse(y != 0, 1, -1)\n  \
                 b <- stats::glm.fit(cbind(X, r), as.numeric(y != 0), \
                 family = stats::binomial(link = \"probit\"))$coefficients[seq_len(px)]\n  \
                 start <- c(b, gam, 0, log(stats::sd(r)))\n  \
                 # each row's log-likelihood and its derivatives in (beta, gamma, atanh(rho),\n  \
                 # log sigma of the endogenous predictor): the predictor's normal density times\n  \
                 # the outcome's probit given the predictor's error\n  \
                 obs <- function(th) {\n    \
                 b <- th[seq_len(px)]; g <- th[px + seq_len(pz)]; rho <- tanh(th[px + pz + 1])\n    \
                 ls <- th[px + pz + 2]; sr <- sqrt(1 - rho^2); u <- (v - drop(Z %*% g)) / exp(ls)\n    \
                 w <- (drop(X %*% b) + rho * u) / sr\n    \
                 lp <- stats::pnorm(sgn * w, log.p = TRUE); lam <- sgn * exp(stats::dnorm(w, log = TRUE) - lp)\n    \
                 list(ll = stats::dnorm(u, log = TRUE) - ls + lp, sc = cbind(X * (lam / sr), \
                 Z * ((u - lam * rho / sr) / exp(ls)), lam * (sr * u + w * rho), \
                 -1 + u^2 - lam * rho * u / sr))\n  }\n",
            );
        }
        let variance = if !cluster.is_empty() {
            format!(
                "  cl <- k[[{}]]; G <- length(unique(cl))\n  \
                 V <- Vi %*% crossprod(rowsum(obs(th)$sc, cl)) %*% Vi * G / (G - 1)\n",
                r_string(cluster)
            )
        } else if a.choice("errors") == "hetero" {
            "  V <- Vi %*% crossprod(obs(th)$sc) %*% Vi * N / (N - 1)\n".to_string()
        } else {
            "  V <- Vi\n".to_string()
        };
        let sigmas = if tobit {
            format!(
                "paste0(\"lnsigma:\", {}), paste0(\"lnsigma:\", {})",
                r_string(y),
                r_string(endog)
            )
        } else {
            format!("paste0(\"lnsigma:\", {})", r_string(endog))
        };
        let title = if tobit {
            "IV tobit by maximum likelihood, \", N, \" observations, \", sum(lo), \
             \" censored below and \", sum(hi), \" above\\n\\n"
        } else {
            "IV probit by maximum likelihood, \", N, \" observations\\n\\n"
        };
        code.push_str(&format!(
            "  f <- function(th) sum(obs(th)$ll)\n  \
             grad <- function(th) colSums(obs(th)$sc)\n  \
             hess <- function(th) stats::optimHess(th, f, grad, control = list(ndeps = rep(1e-5, length(th))))\n  \
             th <- stats::optim(start, f, grad, method = \"BFGS\", \
             control = list(fnscale = -1, maxit = 10000, reltol = 1e-16))$par\n  \
             # Newton steps on from where BFGS stops, to the maximum\n  \
             for (i in 1:10) {{ s <- solve(hess(th), grad(th)); if (!isTRUE(f(th - s) >= f(th))) break; th <- th - s }}\n  \
             names(th) <- c(paste0({ys}, \":\", colnames(X)), paste0({es}, \":\", colnames(Z)), \
             \"athrho\", {sigmas})\n  \
             Vi <- solve(-hess(th))\n{variance}  \
             dimnames(V) <- list(names(th), names(th))\n  \
             cat(\"{title}\", sep = \"\")\n  \
             b <- th\n",
            ys = r_string(y),
            es = r_string(endog),
        ));
        code.push_str(&coef_table(false, a.choice("conf")));
        code.push_str(&format!(
            "  # the correlation of the errors and their standard deviations, with the delta\n  \
             # method's errors and the intervals of atanh(rho) and log sigma transformed\n  \
             a <- px + pz + 1; sg <- (a + 1):length(th)\n  \
             anc <- rbind(c(tanh(th[a]), (1 - tanh(th[a])^2) * se[a], tanh(ci[a, ])), \
             cbind(exp(th[sg]), exp(th[sg]) * se[sg], exp(ci[sg, , drop = FALSE])))\n  \
             dimnames(anc) <- list(c(paste0(\"corr(\", {es}, \", \", {ys}, \")\"), \
             sub(\"^lnsigma:\", \"sd:\", names(th)[sg])), c(\"Estimate\", \"Std. Error\", colnames(ci)))\n  \
             cat(\"\\n\")\n  print(anc)\n  \
             cat(\"\\nWald test of exogeneity (corr = 0): chi2(1) = \", format(th[a]^2 / V[a, a], digits = 7), \
             \", p = \", format(stats::pchisq(th[a]^2 / V[a, a], 1, lower.tail = FALSE), digits = 4), \
             \"\\nLog-likelihood: \", format(f(th), digits = 12), \"\\n\", sep = \"\")\n  \
             list(coefficients = th, vcov = V, loglik = f(th), nobs = N)\n}})",
            ys = r_string(y),
            es = r_string(endog),
        ));
        code
    },
};

// ---- Treatment effects ----

const TE_METHODS: &[Opt] = &[
    Opt {
        value: "ra",
        label: "Regression adjustment",
    },
    Opt {
        value: "ipw",
        label: "Inverse-probability weighting",
    },
    Opt {
        value: "aipw",
        label: "Augmented IPW (both models; doubly robust)",
    },
];

const TE_STATS: &[Opt] = &[
    Opt {
        value: "ate",
        label: "Average treatment effect (ATE)",
    },
    Opt {
        value: "atet",
        label: "Average effect on the treated (ATET)",
    },
    Opt {
        value: "pomeans",
        label: "Potential-outcome means",
    },
];

fn te_check(a: &Args) -> Vec<Problem> {
    let mut out = roles(
        a,
        &[("x", "an outcome-model predictor")],
        &[("y", "outcome"), ("treat", "treatment")],
    );
    for v in a.vars("tx") {
        if v == a.var_text("y") || v == a.var_text("treat") {
            out.push(Problem::new(
                "tx",
                format!("{v} is the outcome or the treatment; it cannot predict the treatment."),
            ));
        }
    }
    out
}

pub const TREATMENT_EFFECTS: Spec = Spec {
    entry: "Treatment effects",
    menu: "Statistics > Treatment effects > Treatment effects",
    about: "The effect of a 0/1 treatment on an outcome from observational data, adjusting for \
            the predictors of the outcome, of the treatment or both, as Stata's teffects ra, ipw \
            and aipw: regression adjustment (a linear model of the outcome in each group, by \
            glm), inverse-probability weighting (a logit model of the treatment, by glm) or \
            augmented IPW. It gives the average treatment effect, the average effect on the \
            treated or the two potential-outcome means, with Stata's robust standard errors from \
            the stacked estimating equations of the models and the effects.",
    widgets: &[
        Widget::Choice {
            id: "method",
            label: "Method",
            options: TE_METHODS,
            default: "ra",
            hint: "",
        },
        Widget::Variable {
            id: "y",
            label: "Outcome",
            kinds: NUMBERS,
            required: true,
            hint: "",
        },
        Widget::Variable {
            id: "treat",
            label: "Treatment (0/1)",
            kinds: SELECTED,
            required: true,
            hint: "1 or TRUE for the treated, 0 or FALSE for the controls.",
        },
        Widget::Variables {
            id: "x",
            label: "Outcome-model predictors",
            kinds: PREDICTORS,
            min: 0,
            hint: "Used by regression adjustment and augmented IPW.",
        },
        Widget::Variables {
            id: "tx",
            label: "Treatment-model predictors",
            kinds: PREDICTORS,
            min: 0,
            hint: "Used by inverse-probability weighting and augmented IPW.",
        },
        Widget::Choice {
            id: "stat",
            label: "Effect",
            options: TE_STATS,
            default: "ate",
            hint: "ATE and ATET come with the control group's potential-outcome mean (over all \
                   rows, or over the treated), as Stata.",
        },
        Widget::Choice {
            id: "conf",
            label: "Confidence level",
            options: LEVELS,
            default: "0.95",
            hint: "",
        },
        SAVE,
    ],
    needs: &[],
    styles: BASE,
    check: te_check,
    code: |a, _| {
        let model = a.name("save");
        let data = a.data();
        let method = a.choice("method");
        let (outcome, treatment) = (method != "ipw", method != "ra");
        let (y, treat) = (a.var_text("y"), a.var_text("treat"));
        let (x, tx) = (a.vars("x"), a.vars("tx"));
        let mut columns = vec![y, treat];
        if outcome {
            columns.extend(&x);
        }
        if treatment {
            for v in &tx {
                if !columns.contains(v) {
                    columns.push(v);
                }
            }
        }
        let mut code = format!(
            "{model} <- local({{\n{}  y <- k[[{}]]; tr <- as.numeric(k[[{}]]); N <- nrow(k)\n  \
             stopifnot(\"the treatment must be 0 or 1 (FALSE or TRUE)\" = all(tr %in% c(0, 1)))\n",
            complete_rows(&data, &columns),
            r_string(y),
            r_string(treat),
        );
        let mut start = Vec::new();
        let mut models = Vec::new();
        if outcome {
            code.push_str(&format!(
                "  # the outcome's linear model in each group\n  \
                 Xo <- stats::model.matrix(~ {}, k); po <- ncol(Xo)\n  \
                 b0 <- stats::glm.fit(Xo[tr == 0, , drop = FALSE], y[tr == 0])$coefficients\n  \
                 b1 <- stats::glm.fit(Xo[tr == 1, , drop = FALSE], y[tr == 1])$coefficients\n",
                rhs(&x)
            ));
            start.push("b0, b1");
            models.push("Xo * ((1 - tr) * (y - m0)), Xo * (tr * (y - m1))");
        }
        if treatment {
            code.push_str(&format!(
                "  # the treatment's logit model\n  \
                 Xt <- stats::model.matrix(~ {}, k); pt <- ncol(Xt)\n  \
                 g <- stats::glm.fit(Xt, tr, family = stats::binomial())$coefficients\n",
                rhs(&tx)
            ));
            start.push("g");
            models.push("Xt * (tr - p)");
        }
        let effects = match (method, a.choice("stat")) {
            ("ra", "atet") => "tr * (m1 - m0 - e[1]), tr * (m0 - e[2])",
            ("ra", "pomeans") => "m0 - e[1], m1 - e[2]",
            ("ra", _) => "m1 - m0 - e[1], m0 - e[2]",
            ("ipw", "atet") => "tr * (y - e[2] - e[1]), (1 - tr) * p / (1 - p) * (y - e[2])",
            ("ipw", "pomeans") => "(1 - tr) / (1 - p) * (y - e[1]), tr / p * (y - e[2])",
            ("ipw", _) => "tr / p * (y - e[2] - e[1]), (1 - tr) / (1 - p) * (y - e[2])",
            (_, "atet") => {
                "tr * (y - m0) - (1 - tr) * p / (1 - p) * (y - m0) - tr * e[1], \
                 tr * m0 + (1 - tr) * p / (1 - p) * (y - m0) - tr * e[2]"
            }
            (_, "pomeans") => {
                "m0 + (1 - tr) / (1 - p) * (y - m0) - e[1], m1 + tr / p * (y - m1) - e[2]"
            }
            _ => {
                "m1 + tr / p * (y - m1) - m0 - (1 - tr) / (1 - p) * (y - m0) - e[1], \
                 m0 + (1 - tr) / (1 - p) * (y - m0) - e[2]"
            }
        };
        let names = match a.choice("stat") {
            "atet" => "\"ATET (1 vs 0)\", \"POmean 0 (treated)\"",
            "pomeans" => "\"POmean 0\", \"POmean 1\"",
            _ => "\"ATE (1 vs 0)\", \"POmean 0\"",
        };
        let mut parts = String::new();
        if outcome {
            parts.push_str(
                "    m0 <- drop(Xo %*% th[seq_len(po)]); m1 <- drop(Xo %*% th[po + seq_len(po)])\n",
            );
        }
        if treatment {
            let from = if outcome { "2 * po + " } else { "" };
            parts.push_str(&format!(
                "    p <- stats::plogis(drop(Xt %*% th[{from}seq_len(pt)]))\n"
            ));
        }
        let title = match method {
            "ra" => "regression adjustment",
            "ipw" => "inverse-probability weighting",
            _ => "augmented inverse-probability weighting",
        };
        code.push_str(&format!(
            "  # the estimating equations, one row per observation: the models' and then the\n  \
             # two effects' (e)\n  \
             psi <- function(th) {{\n{parts}    e <- th[length(th) - 1:0]\n    \
             cbind({}, {effects})\n  }}\n  \
             jac <- function(th) sapply(seq_along(th), function(j) {{ h <- 1e-6 * max(abs(th[j]), 1); \
             u <- th; u[j] <- u[j] + h; l <- th; l[j] <- l[j] - h; \
             (colMeans(psi(u)) - colMeans(psi(l))) / (2 * h) }})\n  \
             th <- c({}, 0, 0)\n  \
             for (i in 1:3) th <- th - solve(jac(th), colMeans(psi(th)))\n  \
             # Stata's robust errors: the sandwich of the stacked equations\n  \
             Gi <- solve(jac(th)); Va <- Gi %*% crossprod(psi(th)) %*% t(Gi) / N^2\n  \
             j <- length(th) - 1:0\n  \
             b <- th[j]; V <- Va[j, j]\n  \
             names(b) <- c({names}); dimnames(V) <- list(names(b), names(b))\n  \
             cat(\"Treatment effects by {title}, \", N, \" observations, \", sum(tr), \" treated\\n\\n\", sep = \"\")\n",
            models.join(", "),
            start.join(", "),
        ));
        code.push_str(&coef_table(false, a.choice("conf")));
        code.push_str("  list(coefficients = b, vcov = V, nobs = N, estimates = th)\n})");
        code
    },
};

pub const SPECS: &[&Spec] = &[&IV_REGRESSION, &IV_ML, &HECKMAN, &TREATMENT_EFFECTS];

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
                col("educ", VarKind::Number),
                col("x", VarKind::Number),
                col("z1", VarKind::Number),
                col("z2", VarKind::Number),
                col("region", VarKind::Number),
                col("works", VarKind::Number),
                col("earn", VarKind::Number),
                col("kids", VarKind::Number),
                col("home town", VarKind::Text),
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
    fn iv_writes_feols_with_its_instruments() {
        let v = values(
            &IV_REGRESSION,
            &[("y", "wage")],
            &[
                ("endog", &["educ"]),
                ("instruments", &["z1", "z2"]),
                ("x", &["x"]),
            ],
        );
        assert_eq!(
            code(&IV_REGRESSION, &v),
            "model <- fixest::feols(wage ~ x | educ ~ z1 + z2, data = d, vcov = \"iid\")\n\
             print(summary(model))\nprint(stats::confint(model, level = 0.95))"
        );
        // fixed effects, a cluster, large-sample errors, the first stage
        let mut v = values(
            &IV_REGRESSION,
            &[("y", "wage"), ("cluster", "region"), ("save", "iv")],
            &[
                ("endog", &["educ"]),
                ("instruments", &["z1", "home town"]),
                ("fe", &["region"]),
            ],
        );
        v.set_flag("small", false);
        v.set_flag("first", true);
        assert_eq!(
            code(&IV_REGRESSION, &v),
            "iv <- fixest::feols(wage ~ 1 | region | educ ~ z1 + `home town`, data = d, \
             cluster = ~region, ssc = fixest::ssc(adj = FALSE, cluster.adj = FALSE))\n\
             print(summary(iv, stage = 1:2))\nprint(fixest::fitstat(iv, ~ ivwald))\n\
             print(stats::confint(iv, level = 0.95))"
        );
    }

    #[test]
    fn iv_checks_roles_and_counts() {
        let v = values(
            &IV_REGRESSION,
            &[("y", "wage")],
            &[
                ("endog", &["educ", "x"]),
                ("instruments", &["z1"]),
                ("x", &["z1"]),
            ],
        );
        let problems = IV_REGRESSION.problems(&ctx(), &v);
        let widgets: Vec<&str> = problems.iter().map(|p| p.widget).collect();
        assert!(widgets.contains(&"instruments"), "{problems:?}");
        assert!(widgets.contains(&"x"), "{problems:?}");
        let v = values(
            &IV_REGRESSION,
            &[("y", "wage"), ("save", "d")],
            &[("endog", &["educ"]), ("instruments", &["z1"])],
        );
        assert!(IV_REGRESSION
            .problems(&ctx(), &v)
            .iter()
            .any(|p| p.widget == "save"));
    }

    #[test]
    fn heckman_writes_selection() {
        let v = values(
            &HECKMAN,
            &[("y", "earn"), ("selected", "works")],
            &[("x", &["x", "z1"]), ("z", &["x", "kids", "z1"])],
        );
        assert_eq!(
            code(&HECKMAN, &v),
            "suppressPackageStartupMessages(requireNamespace(\"sampleSelection\", quietly = TRUE))\n\
             model <- sampleSelection::selection(works ~ x + kids + z1, earn ~ x + z1, data = d, \
             method = \"ml\", gradtol = 1e-12, reltol = 1e-16, tol = 1e-14)\nprint(summary(model))"
        );
        // no indicator: selected where the outcome is not missing
        let v = values(
            &HECKMAN,
            &[("y", "earn"), ("method", "2step"), ("save", "h")],
            &[("x", &["x"]), ("z", &["kids", "home town"])],
        );
        assert_eq!(
            code(&HECKMAN, &v),
            "suppressPackageStartupMessages(requireNamespace(\"sampleSelection\", quietly = TRUE))\n\
             h <- sampleSelection::selection(!is.na(earn) ~ kids + `home town`, earn ~ x, data = d, \
             method = \"2step\")\nprint(summary(h))"
        );
        let v = values(
            &HECKMAN,
            &[("y", "earn"), ("selected", "earn")],
            &[("x", &["earn"]), ("z", &["kids"])],
        );
        let problems = HECKMAN.problems(&ctx(), &v);
        let widgets: Vec<&str> = problems.iter().map(|p| p.widget).collect();
        assert!(widgets.contains(&"selected"), "{problems:?}");
        assert!(widgets.contains(&"x"), "{problems:?}");
    }

    fn widgets_with_problems(spec: &Spec, v: &Values) -> Vec<&'static str> {
        spec.problems(&ctx(), v).iter().map(|p| p.widget).collect()
    }

    #[test]
    fn liml_and_gmm_are_written_out() {
        let lists: &[(&str, &[&str])] = &[
            ("endog", &["educ"]),
            ("instruments", &["z1", "z2"]),
            ("x", &["x"]),
        ];
        let v = values(
            &IV_REGRESSION,
            &[("y", "wage"), ("estimator", "liml")],
            lists,
        );
        let liml = code(&IV_REGRESSION, &v);
        assert!(
            liml.starts_with(
                "model <- local({\n  k <- d[stats::complete.cases(d[c(\"wage\", \"educ\", \
                 \"z1\", \"z2\", \"x\")]), ]\n"
            ),
            "{liml}"
        );
        assert!(
            liml.contains("X <- stats::model.matrix(~ educ + x, k)"),
            "{liml}"
        );
        assert!(
            liml.contains("Z <- stats::model.matrix(~ z1 + z2 + x, k)"),
            "{liml}"
        );
        // small-sample statistics are on by default: s^2 on N - K
        assert!(liml.contains("V <- A * sum(e^2) / (N - K)\n"), "{liml}");
        assert!(!liml.contains("fixest"), "{liml}");
        // GMM, clustered: the clustered weights and the small-sample factor
        let mut v = values(
            &IV_REGRESSION,
            &[("y", "wage"), ("estimator", "gmm"), ("cluster", "region")],
            lists,
        );
        v.set_flag("small", true);
        let gmm = code(&IV_REGRESSION, &v);
        assert!(
            gmm.contains("S <- function(e) crossprod(rowsum(Z * e, cl)) / N"),
            "{gmm}"
        );
        assert!(gmm.contains("* G / (G - 1) * (N - 1) / (N - K)"), "{gmm}");
        assert!(gmm.contains("Hansen's J"), "{gmm}");
        // with no error choice made GMM is robust, as ivregress gmm; Conventional
        // stays reachable, and LIML defaults to conventional
        let by = |estimator: &str, errors: Option<&str>| {
            let mut v = values(
                &IV_REGRESSION,
                &[("y", "wage"), ("estimator", estimator)],
                lists,
            );
            if let Some(errors) = errors {
                v.set_text("errors", errors);
            }
            code(&IV_REGRESSION, &v)
        };
        assert!(by("gmm", None).contains("N * B %*% t(ZX) %*% W %*% S(e)"));
        assert!(by("gmm", Some("hetero")).contains("N * B %*% t(ZX) %*% W %*% S(e)"));
        assert!(by("gmm", Some("iid")).contains("V <- N * B"));
        assert!(by("liml", None).contains("A * sum(e^2)"));
        assert!(by("liml", Some("hetero")).contains("crossprod(Xh * e)"));
        // fixed effects are refused with LIML and GMM
        let v = values(
            &IV_REGRESSION,
            &[("y", "wage"), ("estimator", "gmm")],
            &[
                ("endog", &["educ"]),
                ("instruments", &["z1"]),
                ("fe", &["region"]),
            ],
        );
        assert!(widgets_with_problems(&IV_REGRESSION, &v).contains(&"fe"));
    }

    #[test]
    fn iv_probit_and_tobit_check_their_limits() {
        let lists: &[(&str, &[&str])] = &[("instruments", &["z1", "home town"]), ("x", &["x"])];
        let v = values(&IV_ML, &[("y", "works"), ("endog", "educ")], lists);
        let probit = code(&IV_ML, &v);
        assert!(probit.contains("binomial(link = \"probit\")"), "{probit}");
        assert!(
            probit.contains("Z <- stats::model.matrix(~ z1 + `home town` + x, k)"),
            "{probit}"
        );
        assert!(
            probit.contains("stats::optim(start, f, grad, method = \"BFGS\""),
            "{probit}"
        );
        let v = values(
            &IV_ML,
            &[
                ("y", "earn"),
                ("endog", "educ"),
                ("model", "tobit"),
                ("lower", ""),
                ("upper", "10"),
            ],
            lists,
        );
        let tobit = code(&IV_ML, &v);
        assert!(tobit.contains("lo <- y <= -Inf; hi <- y >= 10;"), "{tobit}");
        for (lower, upper, widget) in [
            ("", "", "lower"),
            ("zero", "", "lower"),
            ("5", "2", "upper"),
        ] {
            let v = values(
                &IV_ML,
                &[
                    ("y", "earn"),
                    ("endog", "educ"),
                    ("model", "tobit"),
                    ("lower", lower),
                    ("upper", upper),
                ],
                lists,
            );
            assert!(
                widgets_with_problems(&IV_ML, &v).contains(&widget),
                "{lower} {upper}"
            );
        }
        // the outcome cannot be the endogenous predictor
        let v = values(&IV_ML, &[("y", "educ"), ("endog", "educ")], lists);
        assert!(widgets_with_problems(&IV_ML, &v).contains(&"endog"));
    }

    #[test]
    fn treatment_effects_write_their_equations() {
        let v = values(
            &TREATMENT_EFFECTS,
            &[
                ("y", "earn"),
                ("treat", "works"),
                ("method", "aipw"),
                ("stat", "atet"),
            ],
            &[("x", &["x"]), ("tx", &["x", "kids"])],
        );
        let aipw = code(&TREATMENT_EFFECTS, &v);
        assert!(
            aipw.contains("d[c(\"earn\", \"works\", \"x\", \"kids\")]"),
            "{aipw}"
        );
        assert!(
            aipw.contains("Xt * (tr - p)") && aipw.contains("ATET (1 vs 0)"),
            "{aipw}"
        );
        assert!(aipw.contains("th[2 * po + seq_len(pt)]"), "{aipw}");
        let v = values(
            &TREATMENT_EFFECTS,
            &[("y", "earn"), ("treat", "works"), ("method", "ipw")],
            &[("x", &["x"])],
        );
        let ipw = code(&TREATMENT_EFFECTS, &v);
        assert!(!ipw.contains("Xo") && ipw.contains("~ 1, k"), "{ipw}");
        let v = values(
            &TREATMENT_EFFECTS,
            &[("y", "earn"), ("treat", "works")],
            &[("tx", &["works"])],
        );
        assert!(widgets_with_problems(&TREATMENT_EFFECTS, &v).contains(&"tx"));
    }
}
