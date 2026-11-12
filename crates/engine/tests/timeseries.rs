//! Statistics menu time series (P6.5) end to end against a real R (skipped
//! with a note when none is found). Each dialog's code is built from its spec
//! and run on made-up monthly data (150 months, the rows shuffled out of time
//! order; an ARMA(1, 1) around a mean, and a random walk with drift whose
//! changes are AR(1)), and its numbers are checked three ways:
//!
//! - against the formulas written out here in plain R: the exact Gaussian
//!   likelihood of an ARMA series from its autocovariances (no package, no
//!   Kalman filter), which must be stats::arima's at its estimate and flat
//!   there, with its Hessian for the errors; forecast errors from the psi
//!   weights; the autocorrelations, Ljung-Box Q, Durbin-Levinson's partial
//!   autocorrelations and the regression ones;
//! - against Stata 19's own results for the same data (`arima` with
//!   `vce(oim)` and tight tolerances, its forecasts with `predict, y` and
//!   `mse`, `corrgram` and `corrgram, yw`), recorded below;
//! - and against the report of the same code written out by hand.
//!
//! The test records the R version it ran with (AGENTS.md section 5), and
//! checks it is the one `ENVIRONMENT` pins.

use std::time::{Duration, Instant};

use rcraft_catalog::timeseries::{ARIMA, AUTOCORRELATIONS};
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
fn run(e: &mut Engine, source: Source, code: &str) -> (Outcome, String, usize) {
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
    let mut plots = 0;
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
                    Item::Plot { .. } => {
                        plots += 1;
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
     month <- seq_len(n); out <- data.frame(month, sales, price); \
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
            ("month", VarKind::Number),
            ("sales", VarKind::Number),
            ("price", VarKind::Number),
        ]
        .into_iter()
        .map(|(name, kind)| ColumnInfo {
            name: name.into(),
            kind,
        })
        .collect(),
    }
}

/// Makes the data, then runs the dialog's code on it, its check boxes on
/// unless `off` names them.
fn dialog(e: &mut Engine, spec: &Spec, set: &[(&str, &str)], off: &[&str]) -> (String, usize) {
    let (made, text, _) = run(e, Source::Typed, MAKE);
    assert_eq!(made, Outcome::Ok, "{text}");
    let ctx = context();
    let mut values = spec.defaults(&ctx);
    for (id, text) in set {
        values.set_text(id, *text);
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
    assert_eq!(plots, 1, "{text}");
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
