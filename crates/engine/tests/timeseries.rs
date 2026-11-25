//! Statistics menu time series (P6.5) end to end against a real R (skipped
//! with a note when none is found). Each dialog's code is built from its spec
//! and run on made-up monthly data (150 months, the rows shuffled out of time
//! order; an ARMA(1, 1) around a mean, a random walk with drift whose changes
//! are AR(1), a regression on another series with AR(1) errors, a seasonal
//! AR series, two series that move together, a local level and a local
//! trend, and three made-up curves for the edges of the unit-root p-value),
//! and its numbers are checked three ways:
//!
//! - against the formulas written out here in plain R: the exact Gaussian
//!   likelihood of an ARMA series from its autocovariances (no package, no
//!   Kalman filter), which must be stats::arima's at its estimate and flat
//!   there, with its Hessian for the errors; forecast errors from the psi
//!   weights; the autocorrelations, Ljung-Box Q, Durbin-Levinson's partial
//!   autocorrelations and the regression ones; the Dickey-Fuller and
//!   Phillips-Perron regressions, the smoothing recursions, the
//!   autoregression by least squares and its moving-average weights;
//! - against Stata 19's own results for the same data (`arima` with
//!   `vce(oim)` and tight tolerances, its forecasts with `predict, y` and
//!   `mse`, `corrgram` and `corrgram, yw`, `dfuller`, `pperron`, `tssmooth`
//!   and `var` with `irf`), recorded below;
//! - and against the report of the same code written out by hand.
//!
//! The test records the R version it ran with (AGENTS.md section 5), and
//! checks it is the one `ENVIRONMENT` pins, and the versions of urca, vars
//! and forecast where they are used (those tests skip with a note when the
//! packages are missing).

use std::path::PathBuf;
use std::time::{Duration, Instant};

use rcraft_catalog::timeseries::{
    ARIMA, AUTOCORRELATIONS, FORECAST, SMOOTHING, TS_PLOT, UNIT_ROOT, VAR,
};
use rcraft_engine::{Engine, Item, Outcome, Source};
use rcraft_rcodegen::dialog::{ColumnInfo, Context, Spec, Style, VarKind, Widget};
use rcraft_rhost::{find_installs, Env, SessionConfig};

fn engine() -> Option<Engine> {
    let install = find_installs(&Env::from_process())
        .into_iter()
        .find(|i| i.is_supported());
    let Some(install) = install else {
        eprintln!("skipped: no supported R on this machine");
        return None;
    };
    Engine::start(SessionConfig::new(install)).ok()
}

fn settle(e: &mut Engine) {
    let limit = Instant::now() + Duration::from_secs(120);
    loop {
        e.poll(Duration::from_millis(50));
        if !e.is_working() {
            break;
        }
        assert!(Instant::now() < limit, "the engine did not settle");
    }
}

/// What a run printed, and the plots it drew.
fn run(e: &mut Engine, source: Source, code: &str) -> (Outcome, String, Vec<PathBuf>) {
    let id = e.submit(source, code);
    assert!(id.is_ok(), "{id:?}");
    let id = id.unwrap_or_default();
    settle(e);
    let outcome = e
        .history()
        .entries()
        .last()
        .and_then(|h| h.outcome)
        .unwrap_or(Outcome::Error);
    let mut plots = Vec::new();
    let text = e
        .results()
        .blocks()
        .iter()
        .find(|b| b.command == id)
        .map(|b| {
            b.items
                .iter()
                .map(|i| match i {
                    Item::Text { text, .. } => text.clone(),
                    Item::Warning(w) => format!("warning: {w}\n"),
                    Item::Error { message, .. } => format!("error: {message}\n"),
                    Item::Notice(n) => n.clone(),
                    Item::Plot { path } => {
                        plots.push(path.clone());
                        String::new()
                    }
                })
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default();
    (outcome, text, plots)
}

/// The R version, recorded.
fn versions(e: &mut Engine) {
    let (outcome, text, _) = run(
        e,
        Source::Typed,
        "cat(paste(R.version$major, R.version$minor, sep = '.'))",
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let r = text.trim();
    let pinned = include_str!("../../../ENVIRONMENT")
        .lines()
        .find_map(|l| l.strip_prefix("R_VERSION="))
        .unwrap_or("")
        .trim();
    eprintln!("ran with R {r} and its stats package (ENVIRONMENT pins R {pinned})");
    if r != pinned {
        eprintln!("note: this R is not the pinned one, so the run is not a Verified one");
    }
}

/// 150 months, even months first and then odd ones.
const MAKE: &str = "d <- local({ \
     set.seed(6501); n <- 150; e <- rnorm(n + 50); \
     a <- stats::filter(e, 0.6, method = 'recursive'); arma <- a + 0.3 * c(0, head(a, -1)); \
     sales <- round(10 + tail(as.numeric(arma), n), 3); \
     g <- as.numeric(stats::filter(rnorm(n + 50, sd = 0.8), 0.4, method = 'recursive')); \
     price <- round(50 + cumsum(0.2 + tail(g, n)), 3); \
     temp <- round(rnorm(n, 2, 1), 3); \
     load <- round(5 + 1.5 * temp + tail(as.numeric(stats::filter(rnorm(n + 50), 0.5, method = 'recursive')), n), 3); \
     visits <- round(20 + tail(as.numeric(stats::filter(rnorm(n + 50), c(0.5, 0, 0, 0.4, -0.2), method = 'recursive')), n), 3); \
     v <- matrix(0, n + 50, 2); for (t in 2:(n + 50)) v[t, ] <- c(0.5 * v[t - 1, 1] + 0.2 * v[t - 1, 2], \
       -0.1 * v[t - 1, 1] + 0.6 * v[t - 1, 2]) + rnorm(2); \
     infl <- round(2 + tail(v[, 1], n), 3); growth <- round(1 + tail(v[, 2], n), 3); \
     stock <- round(30 + cumsum(rnorm(n, sd = 0.5)) + rnorm(n), 3); \
     output <- round(100 + cumsum(cumsum(rnorm(n, 0.05, 0.1)) + rnorm(n, sd = 0.6)) + rnorm(n, sd = 1.5), 3); \
     month <- seq_len(n); \
     ex <- exp(month / 25) + (month * 7) %% 5 / 10; ex2 <- month %% 2 * 2 + (month * 7) %% 5 / 100; \
     ex3 <- exp(month / 60) + (month * 7) %% 5 / 3; \
     out <- data.frame(month, sales, price, temp, load, visits, infl, growth, stock, output, ex, ex2, ex3); \
     out[c(seq(2, n, 2), seq(1, n, 2)), ] })";

/// The formulas in plain R. `ll_arma(th, y)`: the exact log-likelihood of
/// an ARMA(1, 1) (or AR(1) with theta = 0) with mean `th[1]`, phi, theta and
/// innovation variance, from the Toeplitz matrix of its autocovariances.
/// `fd(f, th)`: central-difference gradient and Hessian.
const ORACLE: &str = "o <- d[order(d$month), ]; \
     ll_arma <- function(th, y) { n <- length(y); mu <- th[1]; phi <- th[2]; theta <- th[3]; s2 <- th[4]; \
       g0 <- (1 + 2 * phi * theta + theta^2) / (1 - phi^2); \
       g1 <- (phi + theta) * (1 + phi * theta) / (1 - phi^2); \
       ch <- chol(toeplitz(c(g0, g1 * phi^(0:(n - 2))) * s2)); \
       z <- backsolve(ch, y - mu, transpose = TRUE); \
       -0.5 * (n * log(2 * pi) + 2 * sum(log(diag(ch))) + sum(z^2)) }; \
     fd <- function(f, th, h = 1e-4) { k <- length(th); g <- numeric(k); H <- matrix(0, k, k); \
       e <- function(i) { v <- numeric(k); v[i] <- h; v }; \
       for (i in 1:k) { g[i] <- (f(th + e(i)) - f(th - e(i))) / (2 * h); \
         for (j in 1:k) H[i, j] <- (f(th + e(i) + e(j)) - f(th + e(i) - e(j)) - f(th - e(i) + e(j)) + \
           f(th - e(i) - e(j))) / (4 * h^2) }; list(g = g, H = H) }";

fn context() -> Context {
    Context {
        dataset: "d".into(),
        frames: Vec::new(),
        columns: [
            "month", "sales", "price", "temp", "load", "visits", "infl", "growth", "stock",
            "output", "ex", "ex2", "ex3",
        ]
        .into_iter()
        .map(|name| ColumnInfo {
            name: name.into(),
            kind: VarKind::Number,
        })
        .collect(),
    }
}

/// Makes the data, then runs the dialog's code on it, its check boxes on
/// unless `off` names them.
fn dialog(
    e: &mut Engine,
    spec: &Spec,
    set: &[(&str, &str)],
    off: &[&str],
) -> (String, Vec<PathBuf>) {
    dialog_lists(e, spec, set, &[], off)
}

/// [`dialog`], with the multi-variable widgets in `lists` set too.
fn dialog_lists(
    e: &mut Engine,
    spec: &Spec,
    set: &[(&str, &str)],
    lists: &[(&str, &[&str])],
    off: &[&str],
) -> (String, Vec<PathBuf>) {
    let (made, text, _) = run(e, Source::Typed, MAKE);
    assert_eq!(made, Outcome::Ok, "{text}");
    let ctx = context();
    let mut values = spec.defaults(&ctx);
    for (id, text) in set {
        values.set_text(id, *text);
    }
    for (id, names) in lists {
        values.set_list(id, names.iter().map(|n| n.to_string()).collect());
    }
    for w in spec.widgets {
        if let Widget::Check { id, .. } = w {
            values.set_flag(id, !off.contains(id));
        }
    }
    let code = spec.generate(&ctx, &values, Style::Base);
    assert!(code.is_ok(), "{code:?}");
    let code = code.unwrap_or_default();
    let (outcome, text, plots) = run(e, Source::Menu(spec.menu.into()), &code);
    assert_eq!(outcome, Outcome::Ok, "{code}\n{text}");
    (text, plots)
}

/// A report with spaces tidied, blank lines left out, and the echoed call
/// (which wraps where its text does) skipped up to "Coefficients:".
fn report(text: &str) -> Vec<String> {
    let text = text
        .find("Coefficients:")
        .map_or(text, |start| &text[start..]);
    text.lines()
        .filter(|l| {
            let l = l.trim_start();
            !(l.starts_with("Call:") || l.starts_with("stats::arima(") || l.starts_with("warning:"))
        })
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|l| !l.is_empty())
        .collect()
}

fn holds(e: &mut Engine, exprs: &[String]) {
    let exprs = exprs.join(", ");
    let (outcome, text, _) = run(e, Source::Typed, &format!("stopifnot({exprs})"));
    assert_eq!(outcome, Outcome::Ok, "{exprs}\n{text}");
}

fn define(e: &mut Engine, code: &str) {
    let (outcome, text, _) = run(e, Source::Typed, code);
    assert_eq!(outcome, Outcome::Ok, "{code}\n{text}");
}

fn within(a: &str, b: &str, tol: &str) -> String {
    format!("isTRUE(all.equal(unname(as.vector({a})), unname(as.vector({b})), tolerance = {tol}))")
}

#[test]
fn arima_is_statas_and_the_exact_likelihood() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    versions(&mut e);
    let (text, _) = dialog(
        &mut e,
        &ARIMA,
        &[
            ("y", "sales"),
            ("time", "month"),
            ("q", "1"),
            ("horizon", "6"),
        ],
        &[],
    );
    let (_, expected, _) = run(
        &mut e,
        Source::Typed,
        "m <- stats::arima(d$sales[order(d$month)], order = c(1, 0, 1), include.mean = TRUE, \
         method = 'ML', transform.pars = FALSE, \
         optim.control = list(reltol = 1e-15, maxit = 1000, ndeps = rep(1e-5, 3))); print(m); \
         print(stats::confint(m, level = 0.95)); cat('\\nForecasts\\n'); \
         f <- stats::predict(m, n.ahead = 6); \
         print(data.frame(step = 1:6, forecast = as.numeric(f$pred), se = as.numeric(f$se), \
           lower = as.numeric(f$pred - qnorm(0.975) * f$se), upper = as.numeric(f$pred + qnorm(0.975) * f$se)))",
    );
    assert!(report(&expected).len() > 10, "{expected}");
    assert_eq!(report(&text), report(&expected), "{text}");
    define(&mut e, ORACLE);
    // in (mean, phi, theta, sigma^2): the likelihood written out is arima's
    // at its estimate and flat there; the errors are its inverse Hessian.
    // arima's errors come from optim's finite-difference Hessian, good to
    // about 1e-5, so they are held there
    define(
        &mut e,
        "cf <- coef(model); th <- c(cf['intercept'], cf['ar1'], cf['ma1'], model$sigma2); \
         f <- function(t) ll_arma(t, o$sales); D <- fd(f, th); V <- solve(-D$H)",
    );
    holds(
        &mut e,
        &[
            within("f(th)", "model$loglik", "1e-12"),
            "max(abs(D$g)) < 1e-4".to_string(),
            within(
                "sqrt(diag(model$var.coef))[c('intercept', 'ar1', 'ma1')]",
                "sqrt(diag(V))[1:3]",
                "1e-5",
            ),
            // Stata 19: arima sales, arima(1,0,1) vce(oim) with tolerance(1e-12)
            // ltolerance(1e-14) nrtolerance(1e-12) (_cons, ar, ma, sigma); its
            // default tolerances stop about 1e-5 short of this maximum
            within(
                "c(cf[c('intercept', 'ar1', 'ma1')], sqrt(model$sigma2))",
                "c(9.83142874325537, .65279298320395, .240039724289261, .961416797289784)",
                "1e-7",
            ),
            within(
                "sqrt(diag(model$var.coef))[c('intercept', 'ar1', 'ma1')]",
                "c(.276590183106972, .0778381450103159, .0963693010327902)",
                "1e-5",
            ),
            within("model$loglik", "-207.391612893552", "1e-12"),
            // tsappend, add(5); predict double f, y dynamic(151); predict
            // double m, mse dynamic(151)
            within(
                "stats::predict(model, n.ahead = 5)$pred",
                "c(11.1598944864624, 10.6986418588477, 10.3975393800565, 10.2009817946763, 10.0726703821445)",
                "1e-7",
            ),
            within(
                "stats::predict(model, n.ahead = 5)$se",
                "c(.961416797289784, 1.28885451905983, 1.40539498185986, 1.45221828537432, 1.47171890542269)",
                "1e-7",
            ),
        ],
    );

    // AR(2): Stata's arima sales, arima(2,0,0) vce(oim), tight tolerances
    // (ar, L2.ar, _cons, sigma). Without the constant this series sits next
    // to a unit root (phi1 + phi2 = 0.994), where neither program's
    // numerical Hessian can be trusted, so the constant is kept here and
    // left out in the differenced test instead
    dialog(
        &mut e,
        &ARIMA,
        &[
            ("y", "sales"),
            ("time", "month"),
            ("p", "2"),
            ("save", "ar2"),
        ],
        &[],
    );
    holds(
        &mut e,
        &[
            within(
                "c(coef(ar2), sqrt(ar2$sigma2))",
                "c(.892703448603408, -.186475184223885, 9.83092789462575, .96161791954373)",
                "1e-7",
            ),
            within(
                "sqrt(diag(ar2$var.coef))",
                "c(.080103069935971, .0800978433652346, .264203605587805)",
                "1e-5",
            ),
            within("ar2$loglik", "-207.42292489076", "1e-12"),
        ],
    );
}

#[test]
fn differenced_arima_forecasts_the_levels() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    versions(&mut e);
    let (text, _) = dialog(
        &mut e,
        &ARIMA,
        &[
            ("y", "price"),
            ("time", "month"),
            ("d", "1"),
            ("horizon", "6"),
            ("save", "ari"),
        ],
        &[],
    );
    assert!(text.contains("Forecasts"), "{text}");
    define(&mut e, ORACLE);
    // the changes are AR(1) around a drift: the likelihood written out on
    // diff(price), with theta = 0
    define(
        &mut e,
        "cf <- coef(ari); th <- c(cf['intercept'], cf['ar1'], 0, ari$sigma2); \
         f <- function(t) ll_arma(c(t[1:2], 0, t[3]), diff(o$price)); D <- fd(f, th[-3]); \
         fc <- local({ y <- o$price; p <- stats::predict(ari, n.ahead = 6); \
           psi <- cumsum(cf['ar1']^(0:5)); \
           list(level = y[150] + cumsum(as.numeric(p$pred)), se = sqrt(ari$sigma2 * cumsum(psi^2)), \
             dse = as.numeric(p$se)) })",
    );
    holds(
        &mut e,
        &[
            within("f(th[-3])", "ari$loglik", "1e-12"),
            "max(abs(D$g)) < 1e-4".to_string(),
            within(
                "sqrt(diag(ari$var.coef))[c('intercept', 'ar1')]",
                "sqrt(diag(solve(-D$H)))[1:2]",
                "1e-5",
            ),
            // Stata 19: arima price, arima(1,1,0) vce(oim), tight tolerances:
            // _cons is the drift of D.price, as the dialog's constant
            within(
                "c(cf[c('intercept', 'ar1')], sqrt(ari$sigma2))",
                "c(.277261122650884, .358514891485083, .738554799169654)",
                "1e-7",
            ),
            within(
                "sqrt(diag(ari$var.coef))[c('intercept', 'ar1')]",
                "c(.0939697459153037, .0761238973811186)",
                "1e-5",
            ),
            within("ari$loglik", "-166.334689746684", "1e-12"),
            // the dialog's forecasts are the levels, their errors those of the
            // summed changes (psi weights 1, 1 + phi, 1 + phi + phi^2, ...);
            // Stata's predict, y gives the same levels, and its mse is for
            // D.price, the changes
            "local({ x <- local({ y <- o$price; f <- stats::predict(ari, n.ahead = 6); \
               a <- c(1, -cf['ar1']); a <- c(a, 0) - c(0, a); \
               psi <- c(1, stats::ARMAtoMA(ar = -a[-1], ma = numeric(0), lag.max = 5)); \
               c(stats::diffinv(as.numeric(f$pred), xi = y[150])[-1], sqrt(ari$sigma2 * cumsum(psi^2))) }); \
               isTRUE(all.equal(x, c(fc$level, fc$se), tolerance = 1e-12)) })"
                .to_string(),
            within(
                "fc$level[1:5]",
                "c(91.5640036547059, 91.8727217817151, 92.1612607088699, 92.4425650923787, 92.7212757842574)",
                "1e-9",
            ),
            within(
                "fc$dse[1:5]",
                "c(.738554799169654, .784584713184072, .790306657718145, .79103911416237, .791133209750622)",
                "1e-7",
            ),
        ],
    );

    // no constant: Stata's arima price, arima(1,1,0) noconstant
    dialog(
        &mut e,
        &ARIMA,
        &[
            ("y", "price"),
            ("time", "month"),
            ("d", "1"),
            ("save", "nc"),
        ],
        &["constant"],
    );
    holds(
        &mut e,
        &[
            within(
                "c(coef(nc), sqrt(nc$sigma2))",
                "c(.426856585015156, .757490919600167)",
                "1e-7",
            ),
            within("sqrt(diag(nc$var.coef))", ".0738403110392111", "1e-5"),
            within("nc$loglik", "-170.138595832104", "1e-12"),
        ],
    );
}

#[test]
fn autocorrelations_are_corrgrams() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    versions(&mut e);
    let (text, plots) = dialog(
        &mut e,
        &AUTOCORRELATIONS,
        &[("y", "sales"), ("time", "month"), ("lags", "6")],
        &[],
    );
    assert_eq!(plots.len(), 1, "{text}");
    define(&mut e, ORACLE);
    // the table by hand: r_k = sum (y_t - m)(y_t+k - m) / sum (y_t - m)^2,
    // Q = n (n + 2) sum r_k^2 / (n - k), Durbin-Levinson for the partial
    // autocorrelations, and the last coefficient of y on its first k lags
    define(
        &mut e,
        "y <- o$sales; n <- length(y); m <- mean(y); \
         r <- vapply(1:6, function(k) sum((y[1:(n - k)] - m) * (y[(k + 1):n] - m)) / sum((y - m)^2), 0); \
         q <- n * (n + 2) * cumsum(r^2 / (n - 1:6)); \
         dl <- local({ out <- numeric(6); a <- numeric(0); \
           for (k in 1:6) { rr <- c(1, r); \
             kk <- (r[k] - sum(a * r[(k - 1):1][seq_along(a)])) / (1 - sum(a * r[seq_along(a)])); \
             a <- c(a - kk * rev(a), kk); out[k] <- kk }; out }); \
         ols <- vapply(1:6, function(k) { X <- cbind(1, sapply(1:k, function(j) y[(k + 1 - j):(n - j)])); \
           yy <- y[(k + 1):n]; solve(crossprod(X), crossprod(X, yy))[k + 1] }, 0); \
         tab <- data.frame(lag = 1:6, ac = r, pac = dl, pac_ols = ols, q = q, \
           p = stats::pchisq(q, 1:6, lower.tail = FALSE))",
    );
    let (_, expected, _) = run(&mut e, Source::Typed, "print(tab)");
    assert_eq!(report(&text), report(&expected), "{text}");
    holds(
        &mut e,
        &[
            // Stata 19: corrgram sales, lags(6) (AC, PAC by regression, Q) and
            // corrgram sales, lags(6) yw (PAC by Yule-Walker)
            within(
                "tab$ac",
                "c(.74867286860563, .480777711005256, .304166367474957, .209888522329758, \
                   .115430671739954, -.00354066635251112)",
                "1e-12",
            ),
            within(
                "tab$pac_ols",
                "c(.755664070977273, -.188406731513465, .0432354572769435, .0395122142759966, \
                   -.0969988904243968, -.119375966709723)",
                "1e-12",
            ),
            within(
                "tab$pac",
                "c(.74867286860563, -.181422890733943, .0346927460510926, .0351997699095548, \
                   -.0872492334019505, -.114530563280028)",
                "1e-12",
            ),
            within(
                "tab$q",
                "c(85.7694782781542, 121.378642661313, 135.728245950744, 142.607785493015, \
                   144.702907365471, 144.704892282522)",
                "1e-12",
            ),
        ],
    );
}

/// Whether these packages load; records their versions, or says the test is
/// skipped.
fn packages(e: &mut Engine, pkgs: &[&str]) -> bool {
    let list = pkgs
        .iter()
        .map(|p| format!("'{p}'"))
        .collect::<Vec<_>>()
        .join(", ");
    let (_, text, _) = run(
        e,
        Source::Typed,
        &format!(
            "local({{ p <- c({list}); ok <- vapply(p, requireNamespace, TRUE, quietly = TRUE); \
             if (all(ok)) cat(paste(p, vapply(p, function(x) format(utils::packageVersion(x)), ''), \
             collapse = ', ')) else cat('missing') }})"
        ),
    );
    if text.contains("missing") {
        eprintln!("skipped: {list} are not all installed");
        return false;
    }
    eprintln!("ran with {}", text.trim());
    true
}

/// Runs a model dialog (Objects > Model) on the model called `model`.
fn on_model(e: &mut Engine, spec: &Spec, model: &str, set: &[(&str, &str)]) -> (Outcome, String) {
    let ctx = Context {
        dataset: model.into(),
        frames: Vec::new(),
        columns: Vec::new(),
    };
    let mut values = spec.defaults(&ctx);
    for (id, text) in set {
        values.set_text(id, *text);
    }
    let code = spec.generate(&ctx, &values, Style::Base);
    assert!(code.is_ok(), "{code:?}");
    let (outcome, text, _) = run(e, Source::Menu(spec.menu.into()), &code.unwrap_or_default());
    (outcome, text)
}

/// The lines of a printout from the one starting with `marker` (tidied as
/// [`report`] tidies them), up to the next blank line.
fn table(text: &str, marker: &str) -> Vec<String> {
    let start = text
        .lines()
        .position(|l| l.trim_start().starts_with(marker))
        .unwrap_or(usize::MAX);
    text.lines()
        .skip(start)
        .take_while(|l| !l.trim().is_empty())
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect()
}

/// What R prints for `code`, tidied by [`table`] from `marker`.
fn printed(e: &mut Engine, code: &str, marker: &str) -> Vec<String> {
    let (outcome, text, _) = run(e, Source::Typed, code);
    assert_eq!(outcome, Outcome::Ok, "{code}\n{text}");
    let lines = table(&text, marker);
    assert!(lines.len() > 1, "{code}\n{text}");
    lines
}

/// The exact log-likelihood of an AR(p) series with mean `mu`, written out
/// from its autocovariances (their pattern from ARMAacf, its variance from
/// the Yule-Walker relation), for the seasonal model.
const AR_ORACLE: &str = "ll_ar <- function(mu, ar, s2, y) { n <- length(y); p <- length(ar); \
     r <- stats::ARMAacf(ar = ar, lag.max = n - 1); g0 <- s2 / (1 - sum(ar * r[2:(p + 1)])); \
     ch <- chol(toeplitz(as.numeric(r) * g0)); z <- backsolve(ch, y - mu, transpose = TRUE); \
     -0.5 * (n * log(2 * pi) + 2 * sum(log(diag(ch))) + sum(z^2)) }";

#[test]
fn seasonal_arima_matches_stata() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    versions(&mut e);
    // arima visits, arima(1,0,0) sarima(1,0,0,4): (1 - phi B)(1 - Phi B^4)
    let (text, _) = dialog(
        &mut e,
        &ARIMA,
        &[
            ("y", "visits"),
            ("time", "month"),
            ("sp", "1"),
            ("period", "4"),
            ("horizon", "6"),
            ("save", "sar"),
        ],
        &[],
    );
    assert!(text.contains("sar1"), "{text}");
    define(&mut e, ORACLE);
    define(&mut e, AR_ORACLE);
    // the likelihood written out is the multiplied-out AR(5)'s, at arima's
    // estimate, and flat there
    define(
        &mut e,
        "cf <- coef(sar); f <- function(t) ll_ar(t[3], c(t[1], 0, 0, t[2], -t[1] * t[2]), t[4], o$visits); \
         th <- c(cf['ar1'], cf['sar1'], cf['intercept'], sar$sigma2); D <- fd(f, th)",
    );
    holds(
        &mut e,
        &[
            within("f(th)", "sar$loglik", "1e-12"),
            "max(abs(D$g)) < 1e-4".to_string(),
            within(
                "sqrt(diag(sar$var.coef))",
                "sqrt(diag(solve(-D$H)))[1:3]",
                "1e-5",
            ),
            // Stata 19: arima visits, arima(1,0,0) sarima(1,0,0,4) vce(oim)
            // tolerance(1e-12) ltolerance(1e-14) nrtolerance(1e-12) (_cons,
            // ar, sar's L4.ar, sigma)
            within(
                "c(cf[c('intercept', 'ar1', 'sar1')], sqrt(sar$sigma2))",
                "c(20.190311635398846, .50131820676472405, .22142058806508003, 1.0796372896146389)",
                "1e-7",
            ),
            within(
                "sqrt(diag(sar$var.coef))[c('intercept', 'ar1', 'sar1')]",
                "c(.22388209920639252, .070624742279117567, .08075173454721539)",
                "1e-5",
            ),
            within("sar$loglik", "-224.5938940685692", "1e-12"),
            // tsappend, add(5); predict double f, y dynamic(151) and mse
            within(
                "stats::predict(sar, n.ahead = 5)$pred",
                "c(20.214470349217461, 20.120521274728056, 20.198770207784342, 20.158409760422689, \
                   20.194687766710363)",
                "1e-7",
            ),
            within(
                "stats::predict(sar, n.ahead = 5)$se",
                "c(1.0796372896146389, 1.2077083239147481, 1.2378132910077122, 1.2452648343782304, \
                   1.2826084386439041)",
                "1e-7",
            ),
        ],
    );

    // a seasonal difference: arima visits, arima(1,0,0) sarima(0,1,1,4), with
    // the dialog's level forecasts; Stata's predict, y gives the same levels
    // (its mse is for the differenced series)
    let (text, _) = dialog(
        &mut e,
        &ARIMA,
        &[
            ("y", "visits"),
            ("time", "month"),
            ("sd", "1"),
            ("sq", "1"),
            ("period", "4"),
            ("horizon", "6"),
            ("save", "sdm"),
        ],
        &[],
    );
    // the level errors from the psi weights of the differenced series' model,
    // summed over the seasons (another route than the dialog's polynomials):
    // y(n + h) less its forecast is the sum of the differenced series' errors
    // at h, h - 4, h - 8, ...
    define(
        &mut e,
        "cf <- coef(sdm); pw <- c(1, stats::ARMAtoMA(ar = cf['ar1'], ma = c(0, 0, 0, cf['sma1']), lag.max = 5)); \
         lw <- vapply(0:5, function(j) sum(pw[seq(j, 0, by = -4) + 1]), 0); \
         lse <- sqrt(sdm$sigma2 * cumsum(lw^2)); \
         st <- c(20.482309217205753, 20.100993435607737, 20.322904344356985, 20.197835608310051, \
           20.555165866686256, 20.136100026362513)",
    );
    let expected = printed(
        &mut e,
        "print(data.frame(step = 1:6, forecast = st, se = lse, lower = st - qnorm(0.975) * lse, \
         upper = st + qnorm(0.975) * lse))",
        "step",
    );
    assert_eq!(table(&text, "step"), expected, "{text}");
    holds(
        &mut e,
        &[
            within(
                "c(cf[c('intercept', 'ar1', 'sma1')], sqrt(sdm$sigma2))",
                "c(-.0091104606867900579, .53944870511257181, -.72721643461278407, 1.1406928759819148)",
                "1e-7",
            ),
            within(
                "sqrt(diag(sdm$var.coef))[c('intercept', 'ar1', 'sma1')]",
                "c(.060022248536958436, .069764100262185699, .097263593864275938)",
                "1e-5",
            ),
            within("sdm$loglik", "-227.99738319899853", "1e-12"),
            within(
                "stats::predict(sdm, n.ahead = 6)$se",
                "c(1.1406928759969892, 1.2960825858276404, 1.3379159270329279, 1.3498461228114338, \
                   1.5359929572224182, 1.5860640700549833)",
                "1e-7",
            ),
        ],
    );
}

#[test]
fn arimax_matches_stata() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    versions(&mut e);
    // arima load temp, ar(1): load = b0 + b1 temp + u, u AR(1)
    dialog_lists(
        &mut e,
        &ARIMA,
        &[("y", "load"), ("time", "month"), ("save", "ax")],
        &[("xreg", &["temp"])],
        &[],
    );
    define(&mut e, ORACLE);
    define(
        &mut e,
        "cf <- coef(ax); th <- c(cf['intercept'], cf['temp'], cf['ar1'], ax$sigma2); \
         f <- function(t) ll_arma(c(0, t[3], 0, t[4]), o$load - t[1] - t[2] * o$temp); D <- fd(f, th)",
    );
    holds(
        &mut e,
        &[
            within("f(th)", "ax$loglik", "1e-12"),
            "max(abs(D$g)) < 1e-4".to_string(),
            within(
                "sqrt(diag(ax$var.coef))[c('intercept', 'temp', 'ar1')]",
                "sqrt(diag(solve(-D$H)))[1:3]",
                "1e-5",
            ),
            // Stata 19: arima load temp, ar(1) vce(oim), tight tolerances
            // (temp, _cons, ar, sigma)
            within(
                "c(cf[c('temp', 'intercept', 'ar1')], sqrt(ax$sigma2))",
                "c(1.3895054111050593, 5.1984903071475728, .45936728514053571, .99722390266273764)",
                "1e-7",
            ),
            within(
                "sqrt(diag(ax$var.coef))[c('temp', 'intercept', 'ar1')]",
                "c(.074369098883555598, .20561546946729506, .072642250886588725)",
                "1e-5",
            ),
            within("ax$loglik", "-212.54228392138381", "1e-12"),
        ],
    );
    // with a difference, the exogenous variable is differenced too: Stata's
    // arima load temp, arima(1,1,0) is arima D.load D.temp, ar(1)
    dialog_lists(
        &mut e,
        &ARIMA,
        &[
            ("y", "load"),
            ("time", "month"),
            ("d", "1"),
            ("save", "axd"),
        ],
        &[("xreg", &["temp"])],
        &[],
    );
    holds(
        &mut e,
        &[within("axd$loglik", "-225.98206249383685", "1e-12")],
    );
}

#[test]
fn unit_root_tests_are_dfuller_and_pperron() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    versions(&mut e);
    if !packages(&mut e, &["urca"]) {
        return;
    }
    // each case: the dialog's settings, then Stata 19's Z(t) and MacKinnon
    // p-value (dfuller ..., r(Zt) and r(p)) and the observations, printed as
    // R prints them and set against the dialog's line
    let df: &[(&str, &str, &str, &str, &str, &str)] = &[
        (
            "price",
            "trend",
            "2",
            "-3.046247248544784",
            ".1196067240872162",
            "147",
        ),
        (
            "price",
            "drift",
            "2",
            "-.49121353585816407",
            ".89371974119184605",
            "147",
        ),
        ("sales", "none", "1", "-.50884101337137222", "NA", "148"),
        (
            "sales",
            "drift",
            "0",
            "-4.4837356881811878",
            ".00021038322461839999",
            "149",
        ),
        (
            "sales",
            "trend",
            "3",
            "-4.0551241555255606",
            ".0073308120565085998",
            "146",
        ),
        (
            "price",
            "drift",
            "4",
            "-.70386611281877043",
            ".84574558437388858",
            "145",
        ),
        // the made-up curves: near the top of MacKinnon's polynomials, and
        // past their bounds
        (
            "ex3",
            "trend",
            "2",
            "-.86965852917485564",
            ".95929023586417095",
            "147",
        ),
        (
            "ex3",
            "drift",
            "2",
            "2.235358380838707",
            ".9989103598872211",
            "147",
        ),
        ("ex", "trend", "0", "104.9107675227433", "1", "149"),
        ("ex", "drift", "0", "184.62471419413399", "1", "149"),
        ("ex2", "trend", "0", "-1714.8160425825549", "0", "149"),
        ("ex2", "drift", "0", "-1720.4413910857779", "0", "149"),
    ];
    for (y, model, lags, z, p, n) in df {
        let (text, _) = dialog(
            &mut e,
            &UNIT_ROOT,
            &[
                ("y", y),
                ("time", "month"),
                ("model", model),
                ("lags", lags),
            ],
            &[],
        );
        let expected = printed(
            &mut e,
            &format!(
                "print(data.frame(test = 'Z(t)', statistic = {z}, p.value = {p}, lags = {lags}, \
                 observations = {n}))"
            ),
            "test",
        );
        assert_eq!(table(&text, "test"), expected, "{y} {model} {lags}\n{text}");
        // the statistic by hand: the t of the last level in the regression of
        // the change on it, the lagged changes and the deterministic terms
        let (constant, trend) = match *model {
            "trend" => ("TRUE", "TRUE"),
            "none" => ("FALSE", "FALSE"),
            _ => ("TRUE", "FALSE"),
        };
        define(
            &mut e,
            &format!(
                "o <- d[order(d$month), ]; k <- {lags}; x <- o${y}; dx <- diff(x); m <- length(dx); \
                 rows <- (k + 1):m; X <- cbind(y1 = x[rows], if ({constant}) cbind(one = rep(1, length(rows))), \
                 if ({trend}) cbind(tt = rows), if (k > 0) sapply(seq_len(k), function(j) dx[rows - j])); \
                 b <- solve(crossprod(X), crossprod(X, dx[rows])); r <- dx[rows] - X %*% b; \
                 tau <- (b[1] - 0) / sqrt(sum(r^2) / (length(rows) - ncol(X)) * solve(crossprod(X))[1, 1])"
            ),
        );
        holds(&mut e, &[within("tau", z, "1e-10")]);
    }
    // pperron ...: r(Zrho), r(Zt) and r(pval), with Stata's default lags (4
    // for 149 observations) or those given
    let pp: &[(&str, &str, &str, &str, &str, &str)] = &[
        (
            "price",
            "trend",
            "auto",
            "-16.304797242453791",
            "-2.9090607538788511",
            ".1593034705196313",
        ),
        (
            "price",
            "drift",
            "auto",
            "-.27928534460252807",
            "-.30027044550531712",
            ".925466020299652",
        ),
        (
            "sales",
            "drift",
            "auto",
            "-37.888762781841344",
            "-4.5646862030270494",
            ".0001499773709118",
        ),
        (
            "sales",
            "drift",
            "3",
            "-38.56102702667079",
            "-4.600947308596246",
            ".0001286513182512",
        ),
        (
            "price",
            "trend",
            "6",
            "-15.82444764948709",
            "-2.867569305193753",
            ".1732341169997724",
        ),
        (
            "sales",
            "none",
            "auto",
            "-.33803423582680397",
            "-.30715224521261908",
            "NA",
        ),
    ];
    for (y, model, lags, zrho, zt, p) in pp {
        let (text, _) = dialog(
            &mut e,
            &UNIT_ROOT,
            &[
                ("y", y),
                ("time", "month"),
                ("test", "pp"),
                ("model", model),
                ("lags", lags),
            ],
            &[],
        );
        let k = if *lags == "auto" { "4" } else { lags };
        let expected = printed(
            &mut e,
            &format!(
                "print(data.frame(test = c('Z(rho)', 'Z(t)'), statistic = c({zrho}, {zt}), \
                 p.value = c(NA, {p}), lags = {k}, observations = 149))"
            ),
            "test",
        );
        assert_eq!(table(&text, "test"), expected, "{y} {model} {lags}\n{text}");
    }
}

#[test]
fn exponential_smoothing_is_tssmooth() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    versions(&mut e);
    // single, given: tssmooth exponential double a = load, parms(0.3)
    // forecast(3): the start is the mean of the first 75 rows, and Stata's
    // series is the one-step forecast
    let (text, _) = dialog(
        &mut e,
        &SMOOTHING,
        &[
            ("y", "load"),
            ("time", "month"),
            ("fit", "given"),
            ("alpha", "0.3"),
            ("save", "s1"),
        ],
        &[],
    );
    assert!(text.contains("Sum of squared errors"), "{text}");
    define(
        &mut e,
        "o <- d[order(d$month), ]; ses <- function(x, a, s) { f <- numeric(length(x)); \
           for (t in seq_along(x)) { f[t] <- s; s <- a * x[t] + (1 - a) * s }; list(f = f, next1 = s) }; \
         h1 <- ses(o$load, 0.3, mean(o$load[1:75]))",
    );
    holds(
        &mut e,
        &[
            within("s1$fitted[, 'xhat']", "h1$f", "1e-12"),
            within("s1$SSE", "sum((o$load - h1$f)^2)", "1e-12"),
            within("predict(s1, 1)", "h1$next1", "1e-12"),
            // Stata: r(rss), which it sums a little differently (2e-9 off its
            // own series), and its series at rows 1, 2 and 151
            within("s1$SSE", "480.89082538476214", "1e-8"),
            within(
                "c(s1$fitted[1:2, 'xhat'], predict(s1, 1))",
                "c(7.9040400000000002, 8.1356279999999988, 7.4596719870590906)",
                "1e-12",
            ),
        ],
    );
    // single, fitted: tssmooth exponential double e = stock, forecast(3).
    // Stata's search stops near the least-squares alpha (the sum of squares
    // is flat there), so its alpha is held to 1e-4, and ours is no worse
    dialog(
        &mut e,
        &SMOOTHING,
        &[("y", "stock"), ("time", "month"), ("save", "s2")],
        &[],
    );
    holds(
        &mut e,
        &[
            "abs(s2$alpha - .51107566528320303) < 1e-4".to_string(),
            within("s2$SSE", "300.52336611693431", "1e-8"),
            "s2$SSE <= sum((o$stock - ses(o$stock, .51107566528320303, mean(o$stock[1:75]))$f)^2) + 1e-9"
                .to_string(),
            within("predict(s2, 1)", "33.705377821927748", "1e-5"),
        ],
    );
    // double (Brown's), given: tssmooth dexponential double b = price,
    // parms(0.3); written out with the two smoothed series and Stata's start
    // values (from the line fitted to the first 75 rows)
    dialog(
        &mut e,
        &SMOOTHING,
        &[
            ("y", "price"),
            ("time", "month"),
            ("method", "double"),
            ("fit", "given"),
            ("alpha", "0.3"),
            ("save", "s3"),
        ],
        &[],
    );
    define(
        &mut e,
        "brown <- function(x, a) { b <- coef(lm(x[1:75] ~ seq_len(75))); \
           S1 <- b[[1]] - (1 - a) / a * b[[2]]; S2 <- b[[1]] - 2 * (1 - a) / a * b[[2]]; f <- numeric(length(x)); \
           for (t in seq_along(x)) { f[t] <- 2 * S1 - S2 + a / (1 - a) * (S1 - S2); \
             S1 <- a * x[t] + (1 - a) * S1; S2 <- a * S1 + (1 - a) * S2 }; \
           list(f = f, ahead = 2 * S1 - S2 + a / (1 - a) * (S1 - S2) * 1:3) }; \
         h3 <- brown(o$price, 0.3)",
    );
    holds(
        &mut e,
        &[
            within("s3$fitted[, 'xhat']", "h3$f", "1e-12"),
            within("predict(s3, 3)", "h3$ahead", "1e-12"),
            // Stata: r(rss), its series at rows 1 and 2
            within("s3$SSE", "188.81924488779623", "1e-8"),
            within(
                "s3$fitted[1:2, 'xhat']",
                "c(48.498629122807031, 49.774330141299217)",
                "1e-12",
            ),
        ],
    );
    // double, fitted: tssmooth dexponential double e = output, forecast(3)
    let (text, _) = dialog(
        &mut e,
        &SMOOTHING,
        &[
            ("y", "output"),
            ("time", "month"),
            ("method", "double"),
            ("save", "s4"),
        ],
        &[],
    );
    assert!(text.contains("Brown's parameter: 0.3991"), "{text}");
    holds(
        &mut e,
        &[
            "abs(1 - sqrt(1 - s4$alpha) - .39915957641601563) < 1e-4".to_string(),
            within("s4$SSE", "1199.692619535228", "1e-8"),
            within(
                "predict(s4, 3)[c(1, 3)]",
                "c(680.40123130854772, 694.27383927144854)",
                "1e-5",
            ),
        ],
    );
    // Holt-Winters, given: tssmooth hwinters double c = price, parms(0.5 0.2)
    // forecast(3)
    dialog(
        &mut e,
        &SMOOTHING,
        &[
            ("y", "price"),
            ("time", "month"),
            ("method", "holt"),
            ("fit", "given"),
            ("alpha", "0.5"),
            ("beta", "0.2"),
            ("save", "s5"),
        ],
        &[],
    );
    define(
        &mut e,
        "holt <- function(x, a, g) { b <- coef(lm(x[1:75] ~ seq_len(75))); L <- b[[1]]; B <- b[[2]]; \
           f <- numeric(length(x)); for (t in seq_along(x)) { f[t] <- L + B; Ln <- a * x[t] + (1 - a) * (L + B); \
             B <- g * (Ln - L) + (1 - g) * B; L <- Ln }; list(f = f, ahead = L + B * 1:3) }; \
         h5 <- holt(o$price, 0.5, 0.2)",
    );
    holds(
        &mut e,
        &[
            within("s5$fitted[, 'xhat']", "h5$f", "1e-12"),
            within("predict(s5, 3)", "h5$ahead", "1e-12"),
            within("s5$SSE", "193.12763031353825", "1e-8"),
            within(
                "c(s5$fitted[c(1, 2, 3, 10, 50, 149, 150), 'xhat'], predict(s5, 3))",
                "c(48.498629122807024, 49.774330141299203, 50.74564763641537, 50.724909338522522, \
                   63.120940328120106, 90.774679377853403, 91.096880837859288, 91.529193484076302, \
                   91.910446549222954, 92.29169961436962)",
                "1e-12",
            ),
        ],
    );
    // Holt-Winters, fitted: tssmooth hwinters double e = output, forecast(3)
    dialog(
        &mut e,
        &SMOOTHING,
        &[
            ("y", "output"),
            ("time", "month"),
            ("method", "holt"),
            ("save", "s6"),
        ],
        &[],
    );
    holds(
        &mut e,
        &[
            within(
                "c(s6$alpha, s6$beta)",
                "c(.76734864112522461, .1105645124190299)",
                "1e-5",
            ),
            within("s6$SSE", "1151.5842030927524", "1e-8"),
            within(
                "predict(s6, 3)[c(1, 3)]",
                "c(680.30594911668527, 693.93105973267484)",
                "1e-6",
            ),
        ],
    );
    // R's start values: HoltWinters' own
    dialog(
        &mut e,
        &SMOOTHING,
        &[
            ("y", "price"),
            ("time", "month"),
            ("method", "holt"),
            ("fit", "given"),
            ("alpha", "0.5"),
            ("beta", "0.2"),
            ("start", "r"),
            ("save", "s7"),
        ],
        &[],
    );
    holds(
        &mut e,
        &[within(
            "s7$SSE",
            "stats::HoltWinters(stats::ts(o$price), alpha = 0.5, beta = 0.2, gamma = FALSE)$SSE",
            "1e-12",
        )],
    );
}

#[test]
fn vector_autoregression_is_statas_var() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    versions(&mut e);
    if !packages(&mut e, &["vars"]) {
        return;
    }
    let (text, _) = dialog_lists(
        &mut e,
        &VAR,
        &[("time", "month"), ("irf", "8"), ("save", "vm")],
        &[("ys", &["infl", "growth"])],
        &[],
    );
    // Stata 19: var infl growth, lags(1/2): e(b) and the square roots of
    // e(V)'s diagonal, in Stata's order (per equation L1, L2 of infl, L1, L2
    // of growth, _cons); vars orders them infl.l1, growth.l1, infl.l2, ...
    define(
        &mut e,
        "sb <- c(.34741211668314659, .026927781223174293, .24421565679176355, .0087165130396686559, \
           1.0587879740361574, .015599998004161493, -.03629657418032789, .63228190210051272, \
           -.27841606051109702, .82629268790931865); \
         ss <- c(.081139036456232203, .078541820879642357, .082466934983274121, .085303142471072957, \
           .21952485356896742, .077821701397304321, .075330671876947253, .079095309356811208, \
           .081815559705548574, .21054967312739081); \
         idx <- c(1, 3, 2, 4, 5); sb <- c(sb[idx], sb[5 + idx]); ss <- c(ss[idx], ss[5 + idx]); \
         o <- d[order(d$month), ]",
    );
    let expected = printed(
        &mut e,
        "print(data.frame(equation = rep(c('infl', 'growth'), each = 5), \
           term = rep(c('infl.l1', 'growth.l1', 'infl.l2', 'growth.l2', 'const'), 2), \
           estimate = sb, std.error = ss, z = sb / ss, p.value = 2 * pnorm(-abs(sb / ss))))",
        "equation",
    );
    assert_eq!(table(&text, "equation"), expected, "{text}");
    // by hand: least squares on the lags, the ML covariance (divisor T) for
    // the errors and the log-likelihood
    define(
        &mut e,
        "Y <- as.matrix(o[3:150, c('infl', 'growth')]); \
         X <- cbind(o$infl[2:149], o$growth[2:149], o$infl[1:148], o$growth[1:148], 1); \
         B <- solve(crossprod(X), crossprod(X, Y)); R <- Y - X %*% B; S <- crossprod(R) / 148; \
         se <- sqrt(outer(diag(solve(crossprod(X))), diag(S))); \
         ll <- -148 / 2 * (2 * log(2 * pi) + log(det(S)) + 2)",
    );
    holds(
        &mut e,
        &[
            within("as.vector(B)", "sb", "1e-12"),
            within("as.vector(se)", "ss", "1e-12"),
            within("as.numeric(logLik(vm))", "ll", "1e-12"),
            // e(ll), e(aic), e(sbic), e(detsig_ml)
            within("ll", "-415.35501811011051", "1e-12"),
            within(
                "c((-2 * ll + 20) / 148, (-2 * ll + log(148) * 10) / 148, det(S))",
                "c(5.7480407852717637, 5.9505551280936633, .93908575066440303)",
                "1e-12",
            ),
        ],
    );
    assert!(text.contains("5.7480408"), "{text}");
    // the impulse responses: vars' own (made with the residual covariance
    // divided by the degrees of freedom) rescaled to Stata's (divided by
    // T), against irf create r1, step(6) and its oirf
    define(
        &mut e,
        "ir <- vars::irf(vm, n.ahead = 8, ortho = TRUE, boot = FALSE)$irf; sc <- sqrt(143 / 148); \
         oi <- do.call(rbind, lapply(c('infl', 'growth'), function(i) do.call(rbind, \
           lapply(c('infl', 'growth'), function(r) data.frame(impulse = i, response = r, step = 0:8, \
             oirf = ir[[i]][, r] * sc)))))",
    );
    let expected = printed(&mut e, "print(oi)", "impulse");
    assert_eq!(table(&text, "impulse"), expected, "{text}");
    holds(
        &mut e,
        &[within(
            "oi$oirf[oi$step <= 6]",
            "c(1.0056731668687273, .33888187861246033, .14162909219282013, .051753389245883914, \
               .015785670804247653, .0037636276011342785, .00099990361042569991, \
               -.042999556698371716, -.011499342102734397, -.026514992790352358, \
               -.023654186238839501, -.011907212112487981, -.0025752236747906632, \
               .0011726385763314996, \
               0, .23532564959446473, .23894634344837704, .12411700038148828, .02662856636290236, \
               -.013269041638891277, -.014919704428273126, \
               .9635977180411528, .60926539802277357, .12061748383536629, -.098178972233365885, \
               -.10239534071004801, -.041497734445408621, .001096718149787486)",
            "1e-10",
        )],
    );
    // var infl growth, lags(1/1) noconstant
    dialog_lists(
        &mut e,
        &VAR,
        &[
            ("time", "month"),
            ("lags", "1"),
            ("terms", "none"),
            ("save", "vn"),
        ],
        &[("ys", &["infl", "growth"])],
        &[],
    );
    holds(
        &mut e,
        &[within(
            "as.numeric(logLik(vn))",
            "-446.17956886345974",
            "1e-12",
        )],
    );
}

#[test]
fn forecasts_are_statas_predictions() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    versions(&mut e);
    if !packages(&mut e, &["forecast"]) {
        return;
    }
    // an ARMA(1, 1) from the ARIMA dialog, then Objects > Model > Forecast
    dialog(
        &mut e,
        &ARIMA,
        &[
            ("y", "sales"),
            ("time", "month"),
            ("q", "1"),
            ("save", "am"),
        ],
        &[],
    );
    let (outcome, text) = on_model(&mut e, &FORECAST, "am", &[("horizon", "6")]);
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let expected = printed(
        &mut e,
        "local({ p <- stats::predict(am, n.ahead = 6); z <- qnorm(0.975); \
           print(data.frame(step = 1:6, forecast = as.numeric(p$pred), se = as.numeric(p$se), \
             lower = as.numeric(p$pred - z * p$se), upper = as.numeric(p$pred + z * p$se))) })",
        "step",
    );
    assert_eq!(table(&text, "step"), expected, "{text}");
    holds(
        &mut e,
        &[
            // Stata 19, as in arima_is_statas_and_the_exact_likelihood:
            // predict, y dynamic(151) and mse after arima sales, arima(1,0,1)
            within(
                "forecast::forecast(am, h = 5)$mean",
                "c(11.1598944864624, 10.6986418588477, 10.3975393800565, 10.2009817946763, 10.0726703821445)",
                "1e-7",
            ),
            within(
                "with(forecast::forecast(am, h = 5, level = 95), (upper - mean) / qnorm(0.975))",
                "c(.961416797289784, 1.28885451905983, 1.40539498185986, 1.45221828537432, 1.47171890542269)",
                "1e-7",
            ),
        ],
    );
    // the seasonal model: arima visits, arima(1,0,0) sarima(1,0,0,4)
    dialog(
        &mut e,
        &ARIMA,
        &[
            ("y", "visits"),
            ("time", "month"),
            ("sp", "1"),
            ("save", "sm"),
        ],
        &[],
    );
    let (outcome, text) = on_model(
        &mut e,
        &FORECAST,
        "sm",
        &[("horizon", "6"), ("conf", "0.9")],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let expected = printed(
        &mut e,
        "local({ p <- stats::predict(sm, n.ahead = 6); z <- qnorm(0.95); \
           print(data.frame(step = 1:6, forecast = as.numeric(p$pred), se = as.numeric(p$se), \
             lower = as.numeric(p$pred - z * p$se), upper = as.numeric(p$pred + z * p$se))) })",
        "step",
    );
    assert_eq!(table(&text, "step"), expected, "{text}");
    holds(
        &mut e,
        &[within(
            "forecast::forecast(sm, h = 5)$mean",
            "c(20.214470349217461, 20.120521274728056, 20.198770207784342, 20.158409760422689, \
               20.194687766710363)",
            "1e-7",
        )],
    );
    // Holt-Winters from the smoothing dialog: tssmooth hwinters ..., parms(0.5
    // 0.2) forecast(3); Stata gives no errors for these
    dialog(
        &mut e,
        &SMOOTHING,
        &[
            ("y", "price"),
            ("time", "month"),
            ("method", "holt"),
            ("fit", "given"),
            ("alpha", "0.5"),
            ("beta", "0.2"),
            ("save", "hw"),
        ],
        &[],
    );
    let (outcome, text) = on_model(&mut e, &FORECAST, "hw", &[("horizon", "6")]);
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(text.contains("91.52919"), "{text}");
    holds(
        &mut e,
        &[within(
            "forecast::forecast(hw, h = 3)$mean",
            "c(91.529193484076302, 91.910446549222954, 92.29169961436962)",
            "1e-12",
        )],
    );
    // a model with an exogenous variable is refused with the reason
    dialog_lists(
        &mut e,
        &ARIMA,
        &[("y", "load"), ("save", "axm")],
        &[("xreg", &["temp"])],
        &[],
    );
    let (outcome, text) = on_model(&mut e, &FORECAST, "axm", &[]);
    assert_eq!(outcome, Outcome::Error, "{text}");
    assert!(text.contains("exogenous"), "{text}");
}

#[test]
fn time_series_plot_draws_one_picture() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    for ys in [&["sales"][..], &["sales", "price", "load"][..]] {
        let (text, plots) =
            dialog_lists(&mut e, &TS_PLOT, &[("time", "month")], &[("ys", ys)], &[]);
        assert_eq!(plots.len(), 1, "{text}");
        assert!(
            plots
                .first()
                .is_some_and(|p| std::fs::read(p).is_ok_and(|b| {
                    b.starts_with(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a])
                        && b.len() > 2000
                })),
            "{plots:?}"
        );
    }
}
