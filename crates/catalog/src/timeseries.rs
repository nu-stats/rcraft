//! Statistics menu, time series (P6.5): ARIMA models by exact maximum
//! likelihood (with seasonal orders and exogenous variables) with forecasts,
//! the autocorrelations with Ljung-Box tests, unit-root tests, exponential
//! smoothing, vector autoregressions and a time-series plot, set up as
//! Stata's `arima`, `corrgram`, `dfuller`, `pperron`, `tssmooth` and `var`.
//! Base R's `stats`, with urca for the Dickey-Fuller test and vars for the
//! autoregression; forecasts of a fitted model (Objects > Model) use the
//! forecast package. The rows are put in the order of a time variable if one
//! is chosen. The model dialogs keep their model under the name they ask
//! for; none changes the data.

use rcraft_rcodegen::dialog::{Args, Opt, Problem, Spec, TextDefault, TextRule};
use rcraft_rcodegen::dialog::{VarKind, Widget};
use rcraft_rcodegen::r_string;

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

/// `d[order(d$t), c("a", "b"), drop = FALSE]`: these columns, in time order.
fn columns(a: &Args, names: &[&str]) -> String {
    let cols = names.iter().map(|n| r_string(n)).collect::<Vec<_>>();
    let rows = match a.var("time") {
        Some(t) => format!("order({}${t})", a.data()),
        None => String::new(),
    };
    format!("{}[{rows}, c({}), drop = FALSE]", a.data(), cols.join(", "))
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

/// The time variable is not one of the series chosen in `id`.
fn time_not_in(a: &Args, id: &str) -> Vec<Problem> {
    let t = a.var_text("time");
    if !t.is_empty() && a.vars(id).contains(&t) {
        vec![Problem::new(
            "time",
            "The time variable cannot also be one of the series.",
        )]
    } else {
        Vec::new()
    }
}

/// A model name that is not the data frame's.
fn save_check(a: &Args) -> Vec<Problem> {
    if a.name_text("save") == a.ctx.dataset {
        vec![Problem::new(
            "save",
            format!("{} is the data frame; choose another name.", a.ctx.dataset),
        )]
    } else {
        Vec::new()
    }
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

const SEASONAL_ORDERS: &[Opt] = &[
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
];

const SEASONAL_DIFFERENCES: &[Opt] = &[
    Opt {
        value: "0",
        label: "0",
    },
    Opt {
        value: "1",
        label: "1 (the change from the same season one cycle before)",
    },
];

const PERIODS: &[Opt] = &[
    Opt {
        value: "4",
        label: "4 (quarters)",
    },
    Opt {
        value: "12",
        label: "12 (months)",
    },
    Opt {
        value: "7",
        label: "7 (days of the week)",
    },
    Opt {
        value: "2",
        label: "2",
    },
    Opt {
        value: "24",
        label: "24 (hours)",
    },
    Opt {
        value: "52",
        label: "52 (weeks)",
    },
];

/// The ARIMA dialog's choices, read once.
struct Orders<'a> {
    p: &'a str,
    d: &'a str,
    q: &'a str,
    sp: &'a str,
    sd: &'a str,
    sq: &'a str,
    period: &'a str,
}

impl<'a> Orders<'a> {
    fn of(a: &'a Args) -> Orders<'a> {
        Orders {
            p: a.choice("p"),
            d: a.choice("d"),
            q: a.choice("q"),
            sp: a.choice("sp"),
            sd: a.choice("sd"),
            sq: a.choice("sq"),
            period: a.choice("period"),
        }
    }

    fn seasonal(&self) -> bool {
        self.sp != "0" || self.sd != "0" || self.sq != "0"
    }

    fn differenced(&self) -> bool {
        self.d != "0" || self.sd != "0"
    }

    /// The AR and MA coefficients to estimate.
    fn terms(&self) -> usize {
        let n = |s: &str| s.parse::<usize>().unwrap_or(0);
        n(self.p) + n(self.q) + n(self.sp) + n(self.sq)
    }

    /// `v` with the differences applied, first the ordinary ones, then the
    /// seasonal ones (as Stata's D.S4.y; the order does not matter).
    fn diffed(&self, v: &str) -> String {
        let mut e = v.to_string();
        if self.d != "0" {
            e = format!("diff({e}, differences = {})", self.d);
        }
        if self.sd != "0" {
            e = format!(
                "diff({e}, lag = {}, differences = {})",
                self.period, self.sd
            );
        }
        e
    }
}

fn arima_check(a: &Args) -> Vec<Problem> {
    let mut out = time_check(a);
    out.extend(time_not_in(a, "xreg"));
    let o = Orders::of(a);
    let xs = a.vars("xreg");
    if o.terms() == 0 && !a.flag("constant") && xs.is_empty() {
        out.push(Problem::new(
            "constant",
            "With no AR or MA terms, no constant and no exogenous variables there is nothing to \
             estimate.",
        ));
    }
    if xs.contains(&a.var_text("y")) {
        out.push(Problem::new(
            "xreg",
            "The series cannot also be one of its exogenous variables.",
        ));
    }
    if !xs.is_empty() && a.choice("horizon") != "0" {
        out.push(Problem::new(
            "horizon",
            "Forecasts of a model with exogenous variables need those variables' future values, \
             which the data do not have; choose no forecast.",
        ));
    }
    out.extend(save_check(a));
    out
}

pub const ARIMA: Spec = Spec {
    entry: "ARIMA model",
    menu: "Statistics > Time series > ARIMA model",
    about: "An autoregressive integrated moving-average model of one series, fitted by exact \
            maximum likelihood (the Kalman filter) as Stata's arima: AR and MA orders, the number \
            of differences, and a constant (the mean, or with differences the mean change, as \
            Stata's _cons); seasonal AR and MA orders and seasonal differences, multiplied in as \
            Stata's sarima(P, D, Q, s); and exogenous variables, a regression with ARMA errors as \
            Stata's arima y x (with differences they are differenced too). It fits with \
            stats::arima on the differenced series, with tight convergence settings to match \
            Stata's results, prints the estimates with their intervals, and forecasts the \
            series' levels with standard errors if asked.",
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
        Widget::Choice {
            id: "sp",
            label: "Seasonal AR order (P)",
            options: SEASONAL_ORDERS,
            default: "0",
            hint: "",
        },
        Widget::Choice {
            id: "sd",
            label: "Seasonal differences (D)",
            options: SEASONAL_DIFFERENCES,
            default: "0",
            hint: "",
        },
        Widget::Choice {
            id: "sq",
            label: "Seasonal MA order (Q)",
            options: SEASONAL_ORDERS,
            default: "0",
            hint: "",
        },
        Widget::Choice {
            id: "period",
            label: "Season length (s)",
            options: PERIODS,
            default: "4",
            hint: "Rows per cycle; used only with seasonal terms.",
        },
        Widget::Variables {
            id: "xreg",
            label: "Exogenous variables (optional)",
            kinds: NUMBERS,
            min: 0,
            hint: "Predictors of the series, whose errors follow the ARMA model.",
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
                   intervals (not with exogenous variables).",
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
        let o = Orders::of(a);
        let (p, q, s) = (o.p, o.q, o.period);
        let constant = a.flag("constant");
        let xs = a.vars("xreg");
        let free = o.terms() + usize::from(constant) + xs.len();
        let y = series(a);
        let mut fitted = format!("{}, order = c({p}, 0, {q})", o.diffed("y"));
        if o.seasonal() {
            fitted.push_str(&format!(
                ", seasonal = list(order = c({}, 0, {}), period = {s})",
                o.sp, o.sq
            ));
        }
        let mut x = String::new();
        if !xs.is_empty() {
            x = format!("\n  x <- as.matrix({})", columns(a, &xs));
            fitted.push_str(&format!(", xreg = {}", o.diffed("x")));
        }
        let mean = if constant { "TRUE" } else { "FALSE" };
        // Stata's arima models the differenced series with its own constant;
        // stats::arima with d > 0 would drop the constant, so the differences
        // are fitted directly. Untransformed parameters and a fine step for
        // the Hessian give the estimates and errors at the maximum (the
        // defaults leave the errors up to 1e-3 off). The model keeps the
        // series it was fitted to as x, where forecast::forecast looks for it
        // (it would otherwise look up the name y, gone with local())
        let mut code = format!(
            "{model} <- local({{\n  y <- {y}{x}\n  m <- stats::arima({fitted}, \
             include.mean = {mean}, method = \"ML\", transform.pars = FALSE,\n    \
             optim.control = list(reltol = 1e-15, maxit = 1000, ndeps = rep(1e-5, {free})))\n  \
             m$x <- stats::ts({})\n  m\n}})\n\
             print({model})\nprint(stats::confint({model}, level = {}))",
            o.diffed("y"),
            a.choice("conf")
        );
        let h = a.choice("horizon");
        if h != "0" {
            let z = format!("stats::qnorm(1 - (1 - {}) / 2)", a.choice("conf"));
            let body = if !o.differenced() {
                format!(
                    "  y <- {y}\n  f <- stats::predict({model}, n.ahead = {h})\n  \
                     data.frame(step = 1:{h}, forecast = as.numeric(f$pred), se = as.numeric(f$se),\n    \
                     lower = as.numeric(f$pred - {z} * f$se), upper = as.numeric(f$pred + {z} * f$se))"
                )
            } else {
                // the level forecasts undo the differences, one step at a
                // time; their errors come from the psi weights of the AR
                // polynomial times the differences, phi(B) Phi(B^s) (1 - B)^d
                // (1 - B^s)^D, and of theta(B) Theta(B^s)
                let (season, ar, ma) = if o.seasonal() {
                    (
                        format!(
                            "\n  sp <- function(a) {{ out <- numeric((length(a) - 1) * {s} + 1); \
                             out[(seq_along(a) - 1) * {s} + 1] <- a; out }}"
                        ),
                        "pm(pm(c(1, -pick(\"ar\")), sp(c(1, -pick(\"sar\")))), dl)",
                        "pm(c(1, pick(\"ma\")), sp(c(1, pick(\"sma\"))))",
                    )
                } else {
                    (
                        String::new(),
                        "pm(c(1, -pick(\"ar\")), dl)",
                        "c(1, pick(\"ma\"))",
                    )
                };
                let mut dl = String::new();
                if o.d != "0" {
                    dl.push_str(&format!(
                        "\n  for (i in seq_len({})) dl <- pm(dl, c(1, -1))",
                        o.d
                    ));
                }
                if o.sd != "0" {
                    dl.push_str(&format!(
                        "\n  for (i in seq_len({})) dl <- pm(dl, sp(c(1, -1)))",
                        o.sd
                    ));
                }
                format!(
                    "  y <- {y}\n  f <- stats::predict({model}, n.ahead = {h})\n  \
                     cf <- stats::coef({model})\n  \
                     pick <- function(k) cf[grepl(paste0(\"^\", k, \"[0-9]+$\"), names(cf))]\n  \
                     pm <- function(a, b) {{ out <- numeric(length(a) + length(b) - 1); \
                     for (i in seq_along(a)) out[i - 1 + seq_along(b)] <- out[i - 1 + seq_along(b)] + a[i] * b; \
                     out }}{season}\n  \
                     dl <- 1{dl}\n  \
                     ar <- {ar}\n  ma <- {ma}\n  \
                     psi <- c(1, stats::ARMAtoMA(ar = -ar[-1], ma = ma[-1], lag.max = {h} - 1))\n  \
                     se <- sqrt({model}$sigma2 * cumsum(psi^2))\n  \
                     lv <- as.numeric(stats::na.omit(y))\n  \
                     for (i in 1:{h}) lv <- c(lv, f$pred[[i]] - sum(dl[-1] * rev(utils::tail(lv, length(dl) - 1))))\n  \
                     level <- utils::tail(lv, {h})\n  \
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
            hint: "Also draws the autocorrelations and partial autocorrelations, one spike per lag, with dashed bounds beyond which a correlation is unlikely to be chance.",
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

// ---- Unit-root test ----

const UNIT_TESTS: &[Opt] = &[
    Opt {
        value: "df",
        label: "Augmented Dickey-Fuller",
    },
    Opt {
        value: "pp",
        label: "Phillips-Perron",
    },
];

const DETERMINISTIC: &[Opt] = &[
    Opt {
        value: "drift",
        label: "Constant",
    },
    Opt {
        value: "trend",
        label: "Constant and trend",
    },
    Opt {
        value: "none",
        label: "Neither",
    },
];

const UNIT_LAGS: &[Opt] = &[
    Opt {
        value: "auto",
        label: "Stata's default",
    },
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
    Opt {
        value: "6",
        label: "6",
    },
    Opt {
        value: "8",
        label: "8",
    },
    Opt {
        value: "12",
        label: "12",
    },
];

/// MacKinnon's (1994) approximate asymptotic p-value of a Dickey-Fuller
/// Z(t), from his published response-surface coefficients for one variable
/// (as Stata's dfuller and pperron report it): a normal probability of a
/// polynomial in the statistic, quadratic below the switch point and cubic
/// above it, and 1 or 0 past the bounds of his simulations.
const MACKINNON: &str = "  mackinnon <- function(tau, trend) {\n    \
     small <- if (trend) c(3.2512, 1.6047, 0.049588) else c(2.1659, 1.4412, 0.038269)\n    \
     large <- if (trend) c(2.5261, 0.61654, -0.37956, -0.060285) else \
     c(1.7339, 0.93202, -0.12745, -0.010368)\n    \
     if (tau > if (trend) 0.7 else 2.74) return(1)\n    \
     if (tau < if (trend) -16.18 else -18.83) return(0)\n    \
     stats::pnorm(if (tau <= if (trend) -2.89 else -1.61) sum(small * tau^(0:2)) else \
     sum(large * tau^(0:3)))\n  }\n";

const UNBROKEN: &str = "  stopifnot(\"the series has missing values; it needs to be unbroken\" = \
                        !anyNA(y))\n";

pub const UNIT_ROOT: Spec = Spec {
    entry: "Unit-root test",
    menu: "Statistics > Time series > Unit-root test",
    about: "Whether a series has a unit root (wanders like a random walk) against being \
            stationary, around a constant or a trend: the augmented Dickey-Fuller test \
            (urca::ur.df, as Stata's dfuller) or the Phillips-Perron test (as Stata's pperron: \
            the regression of the series on its last value, with the Newey-West long-run \
            variance of its residuals). Small (very negative) statistics speak against a unit \
            root. The p-value of Z(t) is MacKinnon's approximation, as Stata reports it; there is \
            none without a constant.",
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
            id: "test",
            label: "Test",
            options: UNIT_TESTS,
            default: "df",
            hint: "",
        },
        Widget::Choice {
            id: "model",
            label: "Deterministic terms",
            options: DETERMINISTIC,
            default: "drift",
            hint: "Under the alternative the series is stationary around a constant, a line, or \
                   zero.",
        },
        Widget::Choice {
            id: "lags",
            label: "Lags",
            options: UNIT_LAGS,
            default: "auto",
            hint: "Dickey-Fuller: the lagged differences added to the regression (Stata's \
                   default 0). Phillips-Perron: the Newey-West lags (Stata's default \
                   int(4 (T/100)^(2/9)) for T observations).",
        },
    ],
    needs: &["urca"],
    styles: BASE,
    check: time_check,
    code: |a, _| {
        let y = series(a);
        let model = a.choice("model");
        let trend = if model == "trend" { "TRUE" } else { "FALSE" };
        let p = |tau: &str| {
            if model == "none" {
                "NA".to_string()
            } else {
                format!("mackinnon({tau}, {trend})")
            }
        };
        let helper = if model == "none" { "" } else { MACKINNON };
        let note = if model == "none" {
            "There is no MacKinnon p-value without a constant (Stata gives none either)."
        } else {
            "The p-value is MacKinnon's (1994) approximation, as Stata's."
        };
        if a.choice("test") == "df" {
            let k = match a.choice("lags") {
                "auto" => "0",
                k => k,
            };
            format!(
                "local({{\n  y <- {y}\n{UNBROKEN}{helper}  \
                 u <- urca::ur.df(y, type = \"{model}\", lags = {k})\n  \
                 tau <- u@teststat[[1]]\n  \
                 cat(\"Augmented Dickey-Fuller test\\n\")\n  \
                 print(data.frame(test = \"Z(t)\", statistic = tau, p.value = {}, lags = {k}, \
                 observations = length(y) - {k} - 1))\n  \
                 cat(\"\\nCritical values of Z(t) (urca's, from Dickey and Fuller's tables)\\n\")\n  \
                 print(u@cval[1, ])\n  cat({})\n}})",
                p("tau"),
                r_string(&format!("\n{note}\n"))
            )
        } else {
            let regression = match model {
                "trend" => "y0 ~ y1 + trend",
                "none" => "y0 ~ 0 + y1",
                _ => "y0 ~ y1",
            };
            let k = match a.choice("lags") {
                "auto" => "floor(4 * (nt / 100)^(2 / 9))",
                k => k,
            };
            // Phillips and Perron's Z statistics (as in Hamilton's Time
            // Series Analysis, 17.6): the OLS t and the coefficient corrected
            // by the Bartlett-weighted autocovariances of the residuals
            format!(
                "local({{\n  y <- {y}\n{UNBROKEN}{helper}  \
                 n <- length(y); y0 <- y[-1]; y1 <- y[-n]; trend <- seq_along(y0)\n  \
                 f <- stats::lm({regression})\n  \
                 e <- stats::residuals(f); nt <- length(e); k <- {k}\n  \
                 g <- function(j) sum(e[(j + 1):nt] * e[seq_len(nt - j)]) / nt\n  \
                 g0 <- g(0)\n  \
                 lam2 <- g0 + 2 * sum(vapply(seq_len(k), function(j) (1 - j / (k + 1)) * g(j), 0))\n  \
                 s2 <- sum(e^2) / f$df.residual\n  \
                 rho <- stats::coef(f)[[\"y1\"]]; se <- sqrt(stats::vcov(f)[[\"y1\", \"y1\"]])\n  \
                 zt <- sqrt(g0 / lam2) * (rho - 1) / se - 0.5 * (lam2 - g0) / sqrt(lam2) * nt * se / sqrt(s2)\n  \
                 zrho <- nt * (rho - 1) - 0.5 * nt^2 * se^2 / s2 * (lam2 - g0)\n  \
                 cat(\"Phillips-Perron test\\n\")\n  \
                 print(data.frame(test = c(\"Z(rho)\", \"Z(t)\"), statistic = c(zrho, zt), \
                 p.value = c(NA, {}), lags = k, observations = nt))\n  cat({})\n}})",
                p("zt"),
                r_string(&format!("\nNewey-West lags as shown. {note}\n"))
            )
        }
    },
};

// ---- Exponential smoothing ----

const SMOOTHERS: &[Opt] = &[
    Opt {
        value: "single",
        label: "Single (a level)",
    },
    Opt {
        value: "double",
        label: "Double (Brown's: a level and a trend, one parameter)",
    },
    Opt {
        value: "holt",
        label: "Holt-Winters (a level and a trend, two parameters)",
    },
];

const PARAMETERS: &[Opt] = &[
    Opt {
        value: "fitted",
        label: "Fitted (the smallest sum of squared errors)",
    },
    Opt {
        value: "given",
        label: "As given below",
    },
];

const STARTS: &[Opt] = &[
    Opt {
        value: "stata",
        label: "From the first half of the series, as Stata's",
    },
    Opt {
        value: "r",
        label: "From the first rows, as R's HoltWinters",
    },
];

fn parameter_check(a: &Args, id: &'static str, out: &mut Vec<Problem>) {
    let ok = a
        .number(id)
        .parse::<f64>()
        .is_ok_and(|v| (0.0..=1.0).contains(&v));
    if !ok {
        out.push(Problem::new(id, "A smoothing parameter is from 0 to 1."));
    }
}

fn smoothing_check(a: &Args) -> Vec<Problem> {
    let mut out = time_check(a);
    if a.choice("fit") == "given" {
        parameter_check(a, "alpha", &mut out);
        if a.choice("method") == "holt" {
            parameter_check(a, "beta", &mut out);
        }
    }
    out.extend(save_check(a));
    out
}

pub const SMOOTHING: Spec = Spec {
    entry: "Exponential smoothing",
    menu: "Statistics > Time series > Exponential smoothing",
    about: "Exponential smoothing of one series with stats::HoltWinters, as Stata's tssmooth: \
            single smoothing (a level; exponential), Brown's double smoothing (a level and a \
            trend from one parameter; dexponential, fitted as Holt's with alpha (2 - alpha) and \
            alpha / (2 - alpha)), or Holt-Winters (a level and a trend, two parameters; \
            hwinters). The parameters are fitted by least squares on the one-step forecast \
            errors, or given. Stata's start values come from the first half of the series (its \
            mean, or the line fitted to it); R's from the first rows. The model is kept for \
            Forecast (Objects > Model).",
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
            id: "method",
            label: "Smoothing",
            options: SMOOTHERS,
            default: "single",
            hint: "",
        },
        Widget::Choice {
            id: "fit",
            label: "Parameters",
            options: PARAMETERS,
            default: "fitted",
            hint: "",
        },
        Widget::Text {
            id: "alpha",
            label: "Alpha (level)",
            default: TextDefault::Fixed("0.5"),
            rule: TextRule::Number,
            hint: "From 0 to 1; used when the parameters are given. Brown's one parameter.",
        },
        Widget::Text {
            id: "beta",
            label: "Beta (trend)",
            default: TextDefault::Fixed("0.3"),
            rule: TextRule::Number,
            hint: "From 0 to 1; used for Holt-Winters when the parameters are given.",
        },
        Widget::Choice {
            id: "start",
            label: "Start values",
            options: STARTS,
            default: "stata",
            hint: "",
        },
        Widget::Text {
            id: "save",
            label: "Save the model as",
            default: TextDefault::Fixed("smooth"),
            rule: TextRule::Name,
            hint: "The fitted model stays in R's workspace under this name; a name already in use \
                   is replaced.",
        },
    ],
    needs: &[],
    styles: BASE,
    check: smoothing_check,
    code: |a, _| {
        let model = a.name("save");
        let y = series(a);
        let method = a.choice("method");
        let given = a.choice("fit") == "given";
        let (alpha, beta) = (a.number("alpha"), a.number("beta"));
        let stata = a.choice("start") == "stata";
        // Stata's first forecast (of row 1) is its start value, where
        // HoltWinters' first is of row 2 (row 3 with a trend): the start
        // values are put in front of the series, at times before 1
        let start = match (method, stata) {
            ("single", true) => {
                "  s0 <- mean(y[seq_len(length(y) %/% 2)])\n  \
                 fit <- function(a) stats::HoltWinters(stats::ts(c(s0, y), start = 0), alpha = a, \
                 beta = FALSE, gamma = FALSE, l.start = s0)\n"
            }
            ("single", false) => {
                "  fit <- function(a) stats::HoltWinters(stats::ts(y), alpha = a, beta = FALSE, \
                 gamma = FALSE)\n"
            }
            (_, true) => {
                "  m <- length(y) %/% 2\n  \
                 b <- stats::coef(stats::lm(y[seq_len(m)] ~ seq_len(m)))\n  \
                 fit <- function(p) stats::HoltWinters(stats::ts(c(b[[1]] - b[[2]], b[[1]], y), \
                 start = -1), alpha = @ALPHA@, beta = @BETA@, gamma = FALSE, l.start = b[[1]], \
                 b.start = b[[2]])\n"
            }
            (_, false) => {
                "  fit <- function(p) stats::HoltWinters(stats::ts(y), alpha = @ALPHA@, \
                 beta = @BETA@, gamma = FALSE)\n"
            }
        };
        let start = match method {
            "double" => start
                .replace("@ALPHA@", "p * (2 - p)")
                .replace("@BETA@", "p / (2 - p)"),
            _ => start.replace("@ALPHA@", "p[1]").replace("@BETA@", "p[2]"),
        };
        let call = match (method, given) {
            ("holt", true) => format!("fit(c({alpha}, {beta}))"),
            ("holt", false) => "fit(stats::nlminb(c(0.5, 0.1), function(p) fit(p)$SSE, lower = 0, \
                                upper = 1, control = list(rel.tol = 1e-15, x.tol = 1e-12))$par)"
                .to_string(),
            (_, true) => format!("fit({alpha})"),
            (_, false) => {
                "fit(stats::optimize(function(a) fit(a)$SSE, c(0, 1), tol = 1e-10)$minimum)"
                    .to_string()
            }
        };
        let mut code = format!(
            "{model} <- local({{\n  y <- {y}\n{UNBROKEN}{start}  {call}\n}})\nprint({model})\n\
             cat(\"\\nSum of squared errors: \", format({model}$SSE, digits = 10), \
             \"\\nRoot mean squared error: \", format(sqrt({model}$SSE / NROW({model}$fitted)), \
             digits = 10), \"\\n\", sep = \"\")"
        );
        if method == "double" {
            code.push_str(&format!(
                "\ncat(\"Brown's parameter: \", format(1 - sqrt(1 - {model}$alpha[[1]]), \
                 digits = 10), \"\\n\", sep = \"\")"
            ));
        }
        code
    },
};

// ---- Vector autoregression ----

const VAR_LAGS: &[Opt] = &[
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

const VAR_TERMS: &[Opt] = &[
    Opt {
        value: "const",
        label: "A constant in each equation",
    },
    Opt {
        value: "none",
        label: "No constant",
    },
];

const STEPS: &[Opt] = &[
    Opt {
        value: "0",
        label: "None",
    },
    Opt {
        value: "8",
        label: "8 steps",
    },
    Opt {
        value: "12",
        label: "12 steps",
    },
    Opt {
        value: "20",
        label: "20 steps",
    },
];

pub const VAR: Spec = Spec {
    entry: "Vector autoregression",
    menu: "Statistics > Time series > Vector autoregression",
    about: "Two or more series, each regressed on the lags of all of them (vars::VAR, by least \
            squares equation by equation), as Stata's var: the coefficients with the large-sample \
            standard errors Stata reports (the residual variance divided by the observations, \
            not the degrees of freedom), the log-likelihood and Stata's AIC and SBIC per \
            observation. Orthogonalised impulse responses (Cholesky, in the order the series are \
            listed) if asked, as Stata's irf table oirf. The model is kept under the name asked \
            for.",
    widgets: &[
        Widget::Variables {
            id: "ys",
            label: "Series",
            kinds: NUMBERS,
            min: 2,
            hint: "Two or more; the impulse responses take them in this order.",
        },
        TIME,
        Widget::Choice {
            id: "lags",
            label: "Lags",
            options: VAR_LAGS,
            default: "2",
            hint: "Lags 1 to this one of every series, as Stata's lags(1/p).",
        },
        Widget::Choice {
            id: "terms",
            label: "Deterministic terms",
            options: VAR_TERMS,
            default: "const",
            hint: "",
        },
        Widget::Choice {
            id: "irf",
            label: "Impulse responses",
            options: STEPS,
            default: "0",
            hint: "",
        },
        Widget::Text {
            id: "save",
            label: "Save the model as",
            default: TextDefault::Fixed("var_model"),
            rule: TextRule::Name,
            hint: "The fitted model stays in R's workspace under this name; a name already in use \
                   is replaced.",
        },
    ],
    needs: &["vars"],
    styles: BASE,
    check: |a| {
        let mut out = time_not_in(a, "ys");
        out.extend(save_check(a));
        out
    },
    code: |a, _| {
        let model = a.name("save");
        let y = columns(a, &a.vars("ys"));
        let mut code = format!(
            "{model} <- local({{\n  y <- {y}\n  vars::VAR(y, p = {}, type = \"{}\")\n}})\n\
             print(local({{\n  m <- {model}\n  \
             do.call(rbind, lapply(names(m$varresult), function(eq) {{\n    \
             f <- m$varresult[[eq]]; s <- summary(f)$coefficients\n    \
             se <- s[, 2] * sqrt(f$df.residual / m$obs)\n    \
             data.frame(equation = eq, term = rownames(s), estimate = s[, 1], std.error = se, \
             z = s[, 1] / se,\n      p.value = 2 * stats::pnorm(-abs(s[, 1] / se)), row.names = NULL)\n  \
             }}))\n}}))\n\
             local({{\n  m <- {model}\n  ll <- as.numeric(stats::logLik(m))\n  \
             k <- sum(vapply(m$varresult, function(f) length(stats::coef(f)), 0))\n  \
             cat(\"\\n\")\n  \
             print(c(observations = m$obs, logLik = ll, AIC = (-2 * ll + 2 * k) / m$obs, \
             SBIC = (-2 * ll + log(m$obs) * k) / m$obs,\n    \
             det.Sigma = det(crossprod(stats::residuals(m)) / m$obs)))\n}})",
            a.choice("lags"),
            a.choice("terms")
        );
        let h = a.choice("irf");
        if h != "0" {
            code.push_str(&format!(
                "\ncat(\"\\nOrthogonalised impulse responses\\n\")\nprint(local({{\n  m <- {model}\n  \
                 chol_l <- t(chol(crossprod(stats::residuals(m)) / m$obs))\n  \
                 ph <- vars::Phi(m, nstep = {h})\n  nm <- colnames(m$y)\n  \
                 out <- do.call(rbind, lapply(0:{h}, function(i) data.frame(impulse = rep(nm, each = length(nm)),\n    \
                 response = rep(nm, length(nm)), step = i, oirf = as.vector(ph[, , i + 1] %*% chol_l))))\n  \
                 out <- out[order(match(out$impulse, nm), match(out$response, nm), out$step), ]\n  \
                 rownames(out) <- NULL\n  out\n}}))"
            ));
        }
        code
    },
};

// ---- Forecast (Objects > Model) ----

const FORECAST_HORIZONS: &[Opt] = &[
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

pub const FORECAST: Spec = Spec {
    entry: "Forecast",
    menu: "Objects > Model > Forecast",
    about: "Forecasts of a fitted ARIMA model (stats::arima, also from the ARIMA dialog) or \
            exponential smoothing (HoltWinters, also from the Exponential smoothing dialog) past \
            the series' end, with forecast::forecast: each step's forecast, its standard error \
            and its interval, as Stata's predict, dynamic() and mse after arima and \
            tssmooth's forecast(). An ARIMA model fitted on a differenced series forecasts that \
            series (the changes, as Stata's predict without the y option); the ARIMA dialog's own \
            forecasts give the levels. A model with exogenous variables needs their future \
            values and is refused.",
    widgets: &[
        Widget::Choice {
            id: "horizon",
            label: "Forecast",
            options: FORECAST_HORIZONS,
            default: "12",
            hint: "",
        },
        Widget::Choice {
            id: "conf",
            label: "Confidence level",
            options: LEVELS,
            default: "0.95",
            hint: "",
        },
    ],
    needs: &["forecast"],
    styles: BASE,
    check: |_| Vec::new(),
    code: |a, _| {
        let conf = a.choice("conf");
        let percent = match conf {
            "0.9" => "90",
            "0.99" => "99",
            _ => "95",
        };
        let h = a.choice("horizon");
        format!(
            "print((function(m) {{\n  \
             stopifnot(\"this works for models fitted by arima or HoltWinters\" = \
             inherits(m, c(\"Arima\", \"HoltWinters\")))\n  \
             if (inherits(m, \"Arima\")) stopifnot(\"a model with exogenous variables needs their \
             future values\" =\n    \
             all(grepl(\"^(s?ar|s?ma)[0-9]+$|^intercept$\", names(stats::coef(m)))))\n  \
             f <- forecast::forecast(m, h = {h}, level = {percent})\n  \
             z <- stats::qnorm(1 - (1 - {conf}) / 2)\n  \
             data.frame(step = 1:{h}, forecast = as.numeric(f$mean), \
             se = (as.numeric(f$upper) - as.numeric(f$mean)) / z,\n    \
             lower = as.numeric(f$lower), upper = as.numeric(f$upper))\n}})({}))",
            a.data()
        )
    },
};

// ---- Time-series plot ----

pub const TS_PLOT: Spec = Spec {
    entry: "Time-series plot",
    menu: "Statistics > Time series > Time-series plot",
    about: "One or more series drawn against time on one set of axes (stats::ts.plot), each in \
            its own colour with a legend, as Stata's tsline. The rows are put in the time \
            variable's order and drawn one per step.",
    widgets: &[
        Widget::Variables {
            id: "ys",
            label: "Series",
            kinds: NUMBERS,
            min: 1,
            hint: "",
        },
        TIME,
        Widget::Text {
            id: "title",
            label: "Title",
            default: TextDefault::Empty,
            rule: TextRule::Any,
            hint: "Optional. If left empty, the title names the series.",
        },
    ],
    needs: &[],
    styles: BASE,
    check: |a| time_not_in(a, "ys"),
    code: |a, _| {
        let ys = a.vars("ys");
        let title = match a.expr("title") {
            "" => ys.join(", "),
            t => t.to_string(),
        };
        let xlab = match a.var("time") {
            Some(_) => format!("Row, in order of {}", a.var_text("time")),
            None => "Row".to_string(),
        };
        format!(
            "local({{\n  y <- {}\n  \
             stats::ts.plot(stats::ts(y), col = seq_len(ncol(y)), xlab = {}, ylab = \"\", main = {})\n  \
             if (ncol(y) > 1) graphics::legend(\"topleft\", legend = names(y), col = seq_len(ncol(y)), \
             lty = 1, bty = \"n\")\n}})",
            columns(a, &ys),
            r_string(&xlab),
            r_string(&title)
        )
    },
};

pub const SPECS: &[&Spec] = &[
    &ARIMA,
    &AUTOCORRELATIONS,
    &UNIT_ROOT,
    &SMOOTHING,
    &VAR,
    &TS_PLOT,
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
            "model <- local({\n  y <- d$sales[order(d$month)]\n  m <- stats::arima(y, order = c(1, 0, 1), \
             include.mean = TRUE, method = \"ML\", transform.pars = FALSE,\n    \
             optim.control = list(reltol = 1e-15, maxit = 1000, ndeps = rep(1e-5, 3)))\n  \
             m$x <- stats::ts(y)\n  m\n})\n\
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
            out.contains("for (i in seq_len(1)) dl <- pm(dl, c(1, -1))"),
            "{out}"
        );
        assert!(!out.contains("sp <- function"), "{out}");
        let mut v = values(&ARIMA, &[("y", "sales"), ("p", "0")]);
        v.set_flag("constant", false);
        assert!(ARIMA
            .problems(&ctx(), &v)
            .iter()
            .any(|p| p.widget == "constant"));
    }

    #[test]
    fn arima_takes_seasons_and_exogenous_variables() {
        let v = values(
            &ARIMA,
            &[
                ("y", "sales"),
                ("sp", "1"),
                ("sd", "1"),
                ("period", "12"),
                ("horizon", "6"),
            ],
        );
        let out = code(&ARIMA, &v);
        assert!(
            out.contains(
                "stats::arima(diff(y, lag = 12, differences = 1), order = c(1, 0, 0), \
                 seasonal = list(order = c(1, 0, 0), period = 12), include.mean = TRUE"
            ),
            "{out}"
        );
        assert!(out.contains("ndeps = rep(1e-5, 3)"), "{out}");
        assert!(
            out.contains("for (i in seq_len(1)) dl <- pm(dl, sp(c(1, -1)))"),
            "{out}"
        );
        let mut v = values(&ARIMA, &[("y", "sales"), ("time", "month"), ("d", "1")]);
        v.set_list("xreg", vec!["price".into()]);
        let out = code(&ARIMA, &v);
        assert!(
            out.contains("  x <- as.matrix(d[order(d$month), c(\"price\"), drop = FALSE])\n"),
            "{out}"
        );
        assert!(
            out.contains(", xreg = diff(x, differences = 1), include.mean = TRUE"),
            "{out}"
        );
        v.set_text("horizon", "6");
        v.set_list("xreg", vec!["price".into(), "sales".into()]);
        let problems = ARIMA.problems(&ctx(), &v);
        assert!(
            problems.iter().any(|p| p.widget == "horizon"),
            "{problems:?}"
        );
        assert!(problems.iter().any(|p| p.widget == "xreg"), "{problems:?}");
    }

    #[test]
    fn unit_root_tests_and_smoothing() {
        let v = values(
            &UNIT_ROOT,
            &[("y", "price"), ("model", "trend"), ("lags", "2")],
        );
        let out = code(&UNIT_ROOT, &v);
        assert!(
            out.contains("u <- urca::ur.df(y, type = \"trend\", lags = 2)"),
            "{out}"
        );
        assert!(out.contains("p.value = mackinnon(tau, TRUE)"), "{out}");
        let v = values(
            &UNIT_ROOT,
            &[("y", "price"), ("test", "pp"), ("model", "none")],
        );
        let out = code(&UNIT_ROOT, &v);
        assert!(out.contains("f <- stats::lm(y0 ~ 0 + y1)"), "{out}");
        assert!(out.contains("k <- floor(4 * (nt / 100)^(2 / 9))"), "{out}");
        assert!(!out.contains("mackinnon"), "{out}");

        let v = values(&SMOOTHING, &[("y", "sales"), ("method", "double")]);
        let out = code(&SMOOTHING, &v);
        assert!(
            out.contains("alpha = p * (2 - p), beta = p / (2 - p), gamma = FALSE"),
            "{out}"
        );
        assert!(out.contains("Brown's parameter"), "{out}");
        let v = values(
            &SMOOTHING,
            &[
                ("y", "sales"),
                ("method", "holt"),
                ("fit", "given"),
                ("beta", "1.5"),
            ],
        );
        assert!(SMOOTHING
            .problems(&ctx(), &v)
            .iter()
            .any(|p| p.widget == "beta"));
        let v = values(
            &SMOOTHING,
            &[
                ("y", "sales"),
                ("method", "holt"),
                ("fit", "given"),
                ("start", "r"),
            ],
        );
        assert!(
            code(&SMOOTHING, &v).contains("  fit(c(0.5, 0.3))\n"),
            "{}",
            code(&SMOOTHING, &v)
        );
    }

    #[test]
    fn var_forecast_and_plot() {
        let mut v = values(&VAR, &[("irf", "8")]);
        v.set_list("ys", vec!["sales".into(), "price".into()]);
        let out = code(&VAR, &v);
        assert!(
            out.starts_with(
                "var_model <- local({\n  y <- d[, c(\"sales\", \"price\"), drop = FALSE]\n  \
                 vars::VAR(y, p = 2, type = \"const\")\n})"
            ),
            "{out}"
        );
        assert!(out.contains("vars::Phi(m, nstep = 8)"), "{out}");
        v.set_text("time", "sales");
        assert!(VAR.problems(&ctx(), &v).iter().any(|p| p.widget == "time"));

        let c = Context {
            dataset: "fit".into(),
            frames: Vec::new(),
            columns: Vec::new(),
        };
        let mut v = FORECAST.defaults(&c);
        v.set_text("conf", "0.9");
        let out = FORECAST.generate(&c, &v, Style::Base).unwrap_or_default();
        assert!(
            out.contains("f <- forecast::forecast(m, h = 12, level = 90)"),
            "{out}"
        );
        assert!(out.ends_with("})(fit))"), "{out}");

        let mut v = values(&TS_PLOT, &[("time", "month")]);
        v.set_list("ys", vec!["sales".into(), "price".into()]);
        let out = code(&TS_PLOT, &v);
        assert!(out.contains("main = \"sales, price\""), "{out}");
        assert!(out.contains("xlab = \"Row, in order of month\""), "{out}");
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
