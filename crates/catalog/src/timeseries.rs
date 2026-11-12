//! Statistics menu, time series (P6.5): ARIMA models by exact maximum
//! likelihood with forecasts, and the autocorrelations with Ljung-Box tests,
//! both with base R's `stats` and set up as Stata's `arima` and `corrgram`.
//! The rows are put in the order of a time variable if one is chosen. The
//! ARIMA dialog keeps its model under the name it asks for; neither changes
//! the data.

use rcraft_rcodegen::dialog::{Args, Opt, Problem, Spec, TextDefault, TextRule};
use rcraft_rcodegen::dialog::{VarKind, Widget};

use crate::data::BASE;

const NUMBERS: &[VarKind] = &[VarKind::Number];
const TIMES: &[VarKind] = &[VarKind::Number, VarKind::Date];

const TIME: Widget = Widget::Variable {
    id: "time",
    label: "Time (optional)",
    kinds: TIMES,
    required: false,
    hint: "The rows are put in this variable's order first, as Stata's tsset does. Leave empty \
           if the rows are already in time order.",
};

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

/// `d$y`, in the order of the time variable if there is one.
fn series(a: &Args) -> String {
    let y = format!("{}${}", a.data(), a.var("y").unwrap_or_default());
    match a.var("time") {
        Some(t) => format!("{y}[order({}${t})]", a.data()),
        None => y,
    }
}

fn time_check(a: &Args) -> Vec<Problem> {
    let mut out = Vec::new();
    if !a.var_text("time").is_empty() && a.var_text("time") == a.var_text("y") {
        out.push(Problem::new(
            "time",
            "The time variable cannot be the series itself.",
        ));
    }
    out
}

// ---- ARIMA ----

const ORDERS: &[Opt] = &[
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
];

const DIFFERENCES: &[Opt] = &[
    Opt {
        value: "0",
        label: "0 (the series itself)",
    },
    Opt {
        value: "1",
        label: "1 (its changes)",
    },
    Opt {
        value: "2",
        label: "2 (the changes of its changes)",
    },
];

const HORIZONS: &[Opt] = &[
    Opt {
        value: "0",
        label: "None",
    },
    Opt {
        value: "6",
        label: "6 periods",
    },
    Opt {
        value: "12",
        label: "12 periods",
    },
    Opt {
        value: "24",
        label: "24 periods",
    },
];

fn arima_check(a: &Args) -> Vec<Problem> {
    let mut out = time_check(a);
    if a.choice("p") == "0" && a.choice("q") == "0" && !a.flag("constant") {
        out.push(Problem::new(
            "constant",
            "With no AR or MA terms and no constant there is nothing to estimate.",
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

pub const ARIMA: Spec = Spec {
    entry: "ARIMA model",
    menu: "Statistics > Time series > ARIMA model",
    about: "An autoregressive integrated moving-average model of one series, fitted by exact \
            maximum likelihood (the Kalman filter) as Stata's arima: AR and MA orders, the number \
            of differences, and a constant (the mean, or with differences the mean change, as \
            Stata's _cons). It fits with stats::arima on the differenced series, to the maximum's \
            precision, prints the estimates with their intervals, and forecasts the series' \
            levels with standard errors if asked.",
    widgets: &[
        Widget::Variable {
            id: "y",
            label: "Series",
            kinds: NUMBERS,
            required: true,
            hint: "",
        },
        TIME,
        Widget::Choice {
            id: "p",
            label: "AR order (p)",
            options: ORDERS,
            default: "1",
            hint: "",
        },
        Widget::Choice {
            id: "d",
            label: "Differences (d)",
            options: DIFFERENCES,
            default: "0",
            hint: "",
        },
        Widget::Choice {
            id: "q",
            label: "MA order (q)",
            options: ORDERS,
            default: "0",
            hint: "",
        },
        Widget::Check {
            id: "constant",
            label: "Constant",
            default: true,
            hint: "The mean of the series, or with differences the mean of the differenced \
                   series (a drift), as Stata's _cons.",
        },
        Widget::Choice {
            id: "horizon",
            label: "Forecast",
            options: HORIZONS,
            default: "0",
            hint: "Forecasts of the series itself, past its last row, with standard errors and \
                   intervals.",
        },
        Widget::Choice {
            id: "conf",
            label: "Confidence level",
            options: LEVELS,
            default: "0.95",
            hint: "For the estimates' and the forecasts' intervals.",
        },
        Widget::Text {
            id: "save",
            label: "Save the model as",
            default: TextDefault::Fixed("model"),
            rule: TextRule::Name,
            hint: "The fitted model stays in R's workspace under this name; a name already in use \
                   is replaced.",
        },
    ],
    needs: &[],
    styles: BASE,
    check: arima_check,
    code: |a, _| {
        let model = a.name("save");
        let (p, d, q) = (a.choice("p"), a.choice("d"), a.choice("q"));
        let constant = a.flag("constant");
        let free = p.parse::<usize>().unwrap_or(0)
            + q.parse::<usize>().unwrap_or(0)
            + usize::from(constant);
        let y = series(a);
        let fitted = if d == "0" {
            "y".to_string()
        } else {
            format!("diff(y, differences = {d})")
        };
        let mean = if constant { "TRUE" } else { "FALSE" };
        // Stata's arima models the differenced series with its own constant;
        // stats::arima with d > 0 would drop the constant, so the differences
        // are fitted directly. Untransformed parameters and a fine step for
        // the Hessian give the estimates and errors at the maximum (the
        // defaults leave the errors up to 1e-3 off)
        let mut code = format!(
            "{model} <- local({{\n  y <- {y}\n  stats::arima({fitted}, order = c({p}, 0, {q}), \
             include.mean = {mean}, method = \"ML\", transform.pars = FALSE,\n    \
             optim.control = list(reltol = 1e-15, maxit = 1000, ndeps = rep(1e-5, {free})))\n}})\n\
             print({model})\nprint(stats::confint({model}, level = {}))",
            a.choice("conf")
        );
        let h = a.choice("horizon");
        if h != "0" {
            let z = format!("stats::qnorm(1 - (1 - {}) / 2)", a.choice("conf"));
            let body = if d == "0" {
                format!(
                    "  y <- {y}\n  f <- stats::predict({model}, n.ahead = {h})\n  \
                     data.frame(step = 1:{h}, forecast = as.numeric(f$pred), se = as.numeric(f$se),\n    \
                     lower = as.numeric(f$pred - {z} * f$se), upper = as.numeric(f$pred + {z} * f$se))"
                )
            } else {
                // the level forecasts undo the differences; their errors come
                // from the psi weights of phi(B) (1 - B)^d and theta(B)
                format!(
                    "  y <- {y}\n  f <- stats::predict({model}, n.ahead = {h})\n  \
                     cf <- stats::coef({model})\n  \
                     a <- c(1, -cf[startsWith(names(cf), \"ar\")])\n  \
                     for (i in seq_len({d})) a <- c(a, 0) - c(0, a)\n  \
                     psi <- c(1, stats::ARMAtoMA(ar = -a[-1], ma = cf[startsWith(names(cf), \"ma\")], \
                     lag.max = {h} - 1))\n  \
                     se <- sqrt({model}$sigma2 * cumsum(psi^2))\n  \
                     level <- stats::diffinv(as.numeric(f$pred), differences = {d}, \
                     xi = utils::tail(stats::na.omit(y), {d}))[-(1:{d})]\n  \
                     data.frame(step = 1:{h}, forecast = level, se = se, lower = level - {z} * se, \
                     upper = level + {z} * se)"
                )
            };
            code.push_str(&format!(
                "\ncat(\"\\nForecasts\\n\")\nprint(local({{\n{body}\n}}))"
            ));
        }
        code
    },
};

// ---- Autocorrelations ----

const LAGS: &[Opt] = &[
    Opt {
        value: "6",
        label: "6",
    },
    Opt {
        value: "12",
        label: "12",
    },
    Opt {
        value: "24",
        label: "24",
    },
    Opt {
        value: "36",
        label: "36",
    },
];

pub const AUTOCORRELATIONS: Spec = Spec {
    entry: "Autocorrelations",
    menu: "Statistics > Time series > Autocorrelations",
    about: "The autocorrelations and partial autocorrelations of one series at each lag, with \
            the Ljung-Box Q test that all autocorrelations up to the lag are zero, as Stata's \
            corrgram: pac is the Yule-Walker partial autocorrelation (R's pacf, corrgram's yw \
            option) and pac_ols the regression one (corrgram's default). The correlograms are \
            drawn in the Graph tab if asked.",
    widgets: &[
        Widget::Variable {
            id: "y",
            label: "Series",
            kinds: NUMBERS,
            required: true,
            hint: "",
        },
        TIME,
        Widget::Choice {
            id: "lags",
            label: "Lags",
            options: LAGS,
            default: "12",
            hint: "",
        },
        Widget::Check {
            id: "draw",
            label: "Draw the correlograms",
            default: true,
            hint: "",
        },
    ],
    needs: &[],
    styles: BASE,
    check: time_check,
    code: |a, _| {
        let y = series(a);
        let l = a.choice("lags");
        let mut code = format!(
            "print(local({{\n  y <- {y}\n  \
             ac <- stats::acf(y, lag.max = {l}, plot = FALSE, na.action = stats::na.pass)$acf[-1]\n  \
             pac <- stats::pacf(y, lag.max = {l}, plot = FALSE, na.action = stats::na.pass)$acf[, 1, 1]\n  \
             pac_ols <- vapply(1:{l}, function(k) {{\n    x <- stats::embed(y, k + 1)\n    \
             stats::coef(stats::lm(x[, 1] ~ x[, -1]))[[k + 1]]\n  }}, 0)\n  \
             q <- vapply(1:{l}, function(k) stats::Box.test(y, lag = k, type = \"Ljung-Box\")$statistic, 0)\n  \
             data.frame(lag = 1:{l}, ac = ac, pac = pac, pac_ols = pac_ols, q = q,\n    \
             p = stats::pchisq(q, 1:{l}, lower.tail = FALSE))\n}}))"
        );
        if a.flag("draw") {
            let name = a.var_text("y");
            code.push_str(&format!(
                "\nlocal({{\n  y <- {y}\n  op <- graphics::par(mfrow = c(2, 1))\n  \
                 stats::acf(y, lag.max = {l}, na.action = stats::na.pass, main = {})\n  \
                 stats::pacf(y, lag.max = {l}, na.action = stats::na.pass, main = \"\")\n  \
                 graphics::par(op)\n}})",
                rcraft_rcodegen::r_string(&format!("Autocorrelations of {name}"))
            ));
        }
        code
    },
};

pub const SPECS: &[&Spec] = &[&ARIMA, &AUTOCORRELATIONS];

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
                col("month", VarKind::Number),
                col("sales", VarKind::Number),
                col("price", VarKind::Number),
            ],
        }
    }

    fn values(spec: &Spec, set: &[(&str, &str)]) -> Values {
        let mut v = spec.defaults(&ctx());
        for (id, text) in set {
            v.set_text(id, *text);
        }
        v
    }

    fn code(spec: &Spec, v: &Values) -> String {
        let out = spec.generate(&ctx(), v, Style::Base);
        assert!(out.is_ok(), "{out:?}");
        out.unwrap_or_default()
    }

    #[test]
    fn arima_fits_the_differenced_series() {
        let v = values(&ARIMA, &[("y", "sales"), ("time", "month"), ("q", "1")]);
        assert_eq!(
            code(&ARIMA, &v),
            "model <- local({\n  y <- d$sales[order(d$month)]\n  stats::arima(y, order = c(1, 0, 1), \
             include.mean = TRUE, method = \"ML\", transform.pars = FALSE,\n    \
             optim.control = list(reltol = 1e-15, maxit = 1000, ndeps = rep(1e-5, 3)))\n})\n\
             print(model)\nprint(stats::confint(model, level = 0.95))"
        );
        let v = values(
            &ARIMA,
            &[
                ("y", "price"),
                ("d", "1"),
                ("horizon", "6"),
                ("save", "fit"),
            ],
        );
        let out = code(&ARIMA, &v);
        assert!(
            out.contains("stats::arima(diff(y, differences = 1), order = c(1, 0, 0)"),
            "{out}"
        );
        assert!(
            out.contains("level <- stats::diffinv(as.numeric(f$pred), differences = 1"),
            "{out}"
        );
        let mut v = values(&ARIMA, &[("y", "sales"), ("p", "0")]);
        v.set_flag("constant", false);
        assert!(ARIMA
            .problems(&ctx(), &v)
            .iter()
            .any(|p| p.widget == "constant"));
    }

    #[test]
    fn autocorrelations_tabulate_and_draw() {
        let v = values(&AUTOCORRELATIONS, &[("y", "sales"), ("lags", "6")]);
        let out = code(&AUTOCORRELATIONS, &v);
        assert!(out.starts_with("print(local({\n  y <- d$sales\n"), "{out}");
        assert!(out.contains("type = \"Ljung-Box\""), "{out}");
        assert!(
            out.contains("main = \"Autocorrelations of sales\""),
            "{out}"
        );
        let v = values(&AUTOCORRELATIONS, &[("y", "sales"), ("time", "sales")]);
        assert!(AUTOCORRELATIONS
            .problems(&ctx(), &v)
            .iter()
            .any(|p| p.widget == "time"));
    }
}
