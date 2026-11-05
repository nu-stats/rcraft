//! `prais` and `newey`, as Stata 19 fits them. The method is StatCraft's
//! decoding (its `plan/stata/time-series.md`, probes 154 and 155), our own
//! work, written here in R.
//!
//! - `prais`: least squares first (iteration 0), then rho from the residuals
//!   on the original data (e on e's previous value without a constant, or
//!   1 - DW/2 with `rhotype(dw)`), the data transformed (y - rho y_prev, the
//!   constant 1 - rho, the first observation scaled by sqrt(1 - rho^2), or
//!   dropped with `corc`), until rho moves by less than 1e-6; `twostep` stops
//!   after one round. The sums of squares treat the transformed constant as a
//!   constant. `vce(robust)` is HC1 on the transformed data ("Semirobust").
//! - `newey`: least squares with V = n/(n-k) (X'X)^-1 S (X'X)^-1, S the White
//!   sum plus the Bartlett-weighted (1 - l/(L+1)) cross products of the
//!   observations l time units apart.
//!
//! Plain variable names only for now (no factor variables or time-series
//! operators), and data without gaps for `prais`.

use super::{is_name, names, no, Opts, Out, Session};
use crate::expr_r::{r_number, r_text, Untranslated};
use crate::reader::Command;

pub(super) const TSREG_HELPER: &str = r##"stata_tsreg_data <- function(d, y, x, time, delta) {
  vars <- c(y, x, time)
  miss <- setdiff(vars, names(d))
  if (length(miss)) stop("variable ", miss[1], " not found (Stata: r(111))", call. = FALSE)
  num <- function(v) { v <- unclass(v); attributes(v) <- NULL; as.numeric(v) }
  rows <- which(stats::complete.cases(as.data.frame(lapply(d[vars], num))))
  rows <- rows[order(num(d[[time]])[rows])]
  X <- vapply(x, function(v) num(d[[v]])[rows], numeric(length(rows)))
  if (!is.matrix(X)) X <- matrix(X, length(rows), dimnames = list(NULL, x))
  list(rows = rows, y = num(d[[y]])[rows], X = X, t = num(d[[time]])[rows] / delta)
}
stata_tsreg_ols <- function(yv, Xm) {
  if (qr(Xm)$rank < ncol(Xm)) stop("collinear regressors in prais or newey are not translated yet", call. = FALSE)
  inv <- solve(crossprod(Xm))
  b <- drop(inv %*% crossprod(Xm, yv))
  list(b = b, inv = inv, e = drop(yv - Xm %*% b))
}
stata_prais <- function(d, y, x, time, delta = 1, corc = FALSE, twostep = FALSE, rhotype = "regress",
                        robust = FALSE, constant = TRUE, level = 0.95, show = TRUE, log = TRUE) {
  dd <- stata_tsreg_data(d, y, x, time, delta)
  n <- length(dd$y)
  if (n < 3) stop("insufficient observations (Stata: r(2001))", call. = FALSE)
  if (any(abs(diff(dd$t) - 1) > 1e-9)) stop("prais with gaps in the time variable is not translated yet", call. = FALSE)
  Y <- dd$y
  X0 <- if (constant) cbind(dd$X, `(Intercept)` = 1) else dd$X
  k <- ncol(dd$X)
  dwf <- function(e) sum(diff(e)^2) / sum(e^2)
  resid <- function(b) drop(Y - X0 %*% b)
  est <- function(e) if (rhotype == "dw") 1 - dwf(e) / 2 else sum(e[-1] * e[-n]) / sum(e[-n]^2)
  tr <- function(z, rho) {
    s <- c(sqrt(1 - rho^2) * z[1], z[-1] - rho * z[-n])
    if (corc) s[-1] else s
  }
  fit_on <- function(rho) {
    Xs <- apply(X0, 2, tr, rho = rho)
    if (!is.matrix(Xs)) Xs <- matrix(Xs, ncol = ncol(X0))
    colnames(Xs) <- colnames(X0)
    list(y = tr(Y, rho), X = Xs, f = stata_tsreg_ols(tr(Y, rho), Xs))
  }
  ols <- stata_tsreg_ols(Y, X0)
  dw_0 <- dwf(ols$e)
  b <- ols$b
  rho <- 0
  ic <- 0
  rhos <- 0
  repeat {
    nr <- est(resid(b))
    ic <- ic + 1
    rhos <- c(rhos, nr)
    b <- fit_on(nr)$f$b
    done <- twostep || abs(nr - rho) < 1e-6 || ic >= 100
    rho <- nr
    if (done) break
  }
  ft <- fit_on(rho)
  f <- ft$f
  N <- length(ft$y)
  df_r <- N - ncol(X0)
  rss <- sum(f$e^2)
  V <- if (robust) {
    meat <- crossprod(ft$X * f$e)
    f$inv %*% meat %*% f$inv * N / df_r
  } else f$inv * rss / df_r
  dimnames(V) <- list(colnames(X0), colnames(X0))
  b <- stats::setNames(f$b, colnames(X0))
  if (constant) {
    tss <- sum((ft$y - mean(ft$y))^2)
    mss <- tss - rss
    df_m <- k
  } else {
    tss <- sum(ft$y^2)
    mss <- tss - rss
    df_m <- ncol(X0)
  }
  r2 <- mss / tss
  q <- if (constant) seq_len(k) else seq_len(ncol(X0))
  F <- if (robust) drop(t(b[q]) %*% solve(V[q, q, drop = FALSE], b[q])) / length(q) else (mss / df_m) / (rss / df_r)
  ll <- -0.5 * N * (log(2 * pi) + log(rss / N) + 1)
  dw <- dwf(f$e)
  method <- if (corc) "Cochrane–Orcutt" else "Prais–Winsten"
  how <- if (twostep) "twostep" else "iterated"
  if (show) {
    if (log) for (i in seq_along(rhos)) cat(sprintf("Iteration %d:  rho = %.4f\n", i - 1, rhos[i]))
    cat("\n", method, " AR(1) regression with ", how, " estimates\n", sep = "")
    cat(sprintf("Number of obs = %d   F(%d, %d) = %.2f   R-squared = %.4f   Root MSE = %.5g\n", N, df_m, df_r, F, r2,
                sqrt(rss / df_r)))
    stata_coefs(b, V, df_r, level)
    cat(sprintf("rho = %.7f\nDurbin-Watson statistic (original)    = %.6f\nDurbin-Watson statistic (transformed) = %.6f\n",
                rho, dw_0, dw))
  }
  e <- list(N = N, sum_w = N, df_m = df_m, df_r = df_r, F = F, r2 = r2, rmse = sqrt(rss / df_r), mss = mss,
            rss = rss, r2_a = 1 - (1 - r2) * (N - 1) / df_r, ll = ll, N_gaps = 0, tol = 1e-6, max_ic = 100,
            ic = ic, dw = dw, dw_0 = dw_0, rho = rho, rank = ncol(X0),
            cmd = "prais", title = paste(method, "AR(1) regression"), cmdline = NA_character_,
            tranmeth = if (corc) "corc" else "prais", method = how, marginsok = "XB default", depvar = y,
            predict = "prais_p", rhotype = rhotype, vce = if (robust) "robust" else "ols")
  if (robust) e$vcetype <- "Semirobust"
  e$properties <- "b V"
  invisible(list(b = b, vcov = V, df = df_r, rows = dd$rows, e = e, depvar = y))
}
stata_newey <- function(d, y, x, time, delta = 1, lag = 0, constant = TRUE, level = 0.95, show = TRUE) {
  dd <- stata_tsreg_data(d, y, x, time, delta)
  X0 <- if (constant) cbind(dd$X, `(Intercept)` = 1) else dd$X
  n <- length(dd$y)
  p <- ncol(X0)
  if (n <= p) stop("insufficient observations (Stata: r(2001))", call. = FALSE)
  f <- stata_tsreg_ols(dd$y, X0)
  xe <- X0 * f$e
  S <- crossprod(xe)
  for (l in seq_len(lag)) {
    w <- 1 - l / (lag + 1)
    i <- match(dd$t - l, dd$t)
    ok <- which(!is.na(i))
    G <- crossprod(xe[ok, , drop = FALSE], xe[i[ok], , drop = FALSE])
    S <- S + w * (G + t(G))
  }
  V <- f$inv %*% S %*% f$inv * n / (n - p)
  dimnames(V) <- list(colnames(X0), colnames(X0))
  b <- stats::setNames(f$b, colnames(X0))
  q <- if (constant) seq_len(ncol(dd$X)) else seq_len(p)
  df_m <- length(q)
  F <- if (df_m > 0) drop(t(b[q]) %*% solve(V[q, q, drop = FALSE], b[q])) / df_m else 0
  df_r <- n - p
  if (show) {
    cat("\nRegression with Newey–West standard errors\n")
    cat(sprintf("Maximum lag = %d   Number of obs = %d   F(%d, %d) = %.2f   Prob > F = %.4f\n", lag, n, df_m, df_r, F,
                stats::pf(F, df_m, df_r, lower.tail = FALSE)))
    stata_coefs(b, V, df_r, level)
  }
  e <- list(df_m = df_m, df_r = df_r, F = F, p = stats::pf(F, df_m, df_r, lower.tail = FALSE), N = n, lag = lag,
            rank = p, title = "Regression with Newey–West standard errors", cmd = "newey",
            cmdline = NA_character_, estat_cmd = "newey_estat", predict = "newey_p", vcetype = "Newey–West",
            depvar = y, properties = "b V")
  invisible(list(b = b, vcov = V, df = df_r, rows = dd$rows, e = e, depvar = y))
}
# e() after prais or newey: its own values, the command line, b and V
stata_tsreg_e <- function(fit, cmd, cmdline) {
  e <- fit$e
  e$cmdline <- cmdline
  eb <- stata_eb(fit$b, fit$vcov, cmd, fit$depvar)
  c(e, list(b = eb$b, V = eb$V))
}"##;

impl Session {
    /// `prais` and `newey`.
    pub(super) fn tsreg(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let cmd = c.name.as_str();
        self.no_by(c, cmd)?;
        let known: &[(&str, usize)] = if cmd == "prais" {
            &[
                ("corc", 4),
                ("twostep", 3),
                ("rhotype", 4),
                ("vce", 3),
                ("robust", 1),
                ("noconstant", 3),
                ("nolog", 5),
                ("level", 3),
            ]
        } else {
            &[("lag", 3), ("noconstant", 3), ("level", 3)]
        };
        if let Some(o) = opts.other(known) {
            return Err(no(format!(
                "{cmd} with the option {o} is not translated yet"
            )));
        }
        if c.parts.main.trim().is_empty() {
            return Err(no(format!(
                "{cmd} alone (replaying the last fit) is not translated yet"
            )));
        }
        let vars = names(&c.parts.main).map_err(|e| {
            no(format!(
                "{} (factor variables and time-series operators with {cmd} are not translated yet)",
                e.reason
            ))
        })?;
        let Some((dep, regs)) = vars.split_first() else {
            return Err(no("varlist required (Stata: r(100))"));
        };
        let time = self
            .panel_time
            .clone()
            .ok_or_else(|| no("time variable not set, use tsset varname ... (Stata: r(111))"))?;
        if self.panel.is_some() {
            return Err(no(format!("{cmd} on panel data is not translated yet")));
        }
        if !is_name(&time) {
            return Err(no("the time variable must be a plain name"));
        }
        let level = match opts.arg("level", 3) {
            None => 0.95,
            Some(l) => match l.trim().parse::<f64>() {
                Ok(v) if (10.0..=99.99).contains(&v) => v / 100.0,
                _ => {
                    return Err(no(format!(
                        "level({l}) is not a valid confidence level (Stata: r(198))"
                    )))
                }
            },
        };
        let constant = !opts.has("noconstant", 3);
        let src = self.rows_for(c, out)?;
        for h in [
            "stata_tsreg",
            "stata_coefs",
            "stata_eb",
            "stata_returned",
            "stata_names",
        ] {
            self.helpers.insert(h);
        }
        let xs = regs
            .iter()
            .map(|v| r_text(v))
            .collect::<Vec<_>>()
            .join(", ");
        let delta = self.ts_delta.clone().unwrap_or_else(|| "1".into());
        let show = if self.quiet(c) { "FALSE" } else { "TRUE" };
        let common = format!(
            "{src}, {}, c({xs}), {}, delta = {delta}, constant = {}, level = {}, show = {show}",
            r_text(dep),
            r_text(&time),
            if constant { "TRUE" } else { "FALSE" },
            r_number(level)
        );
        let call = if cmd == "prais" {
            let rhotype = match opts.arg("rhotype", 4).as_deref().map(str::trim) {
                None => "regress",
                Some(t) if t.len() >= 2 && "regress".starts_with(t) => "regress",
                Some("dw") => "dw",
                Some(t) => {
                    return Err(no(format!(
                        "prais, rhotype({t}) is not translated yet (regress and dw are)"
                    )))
                }
            };
            let robust = opts.has("robust", 1)
                || opts.arg("vce", 3).is_some_and(|v| {
                    let v = v.trim();
                    !v.is_empty() && "robust".starts_with(v)
                });
            if opts.arg("vce", 3).is_some() && !robust {
                return Err(no(
                    "prais with this vce() is not translated yet (vce(robust) is)",
                ));
            }
            format!(
                "stata_prais({common}, corc = {}, twostep = {}, rhotype = {}, robust = {}, log = {})",
                if opts.has("corc", 4) { "TRUE" } else { "FALSE" },
                if opts.has("twostep", 3) { "TRUE" } else { "FALSE" },
                r_text(rhotype),
                if robust { "TRUE" } else { "FALSE" },
                if opts.has("nolog", 5) { "FALSE" } else { "TRUE" },
            )
        } else {
            let lag = opts
                .arg("lag", 3)
                .ok_or_else(|| no("option lag() required (Stata: r(198))"))?
                .trim()
                .parse::<u32>()
                .map_err(|_| no("lag() invalid (Stata: r(198))"))?;
            format!("stata_newey({common}, lag = {lag})")
        };
        out.line(format!("stata_estimates <- {call}"));
        out.line("stata_b <- stata_estimates$b");
        out.line("stata_vcov <- stata_estimates$vcov");
        out.line("stata_df <- stata_estimates$df");
        out.line(format!(
            "stata_e <- stata_tsreg_e(stata_estimates, {}, {})",
            r_text(cmd),
            r_text(&super::returned::cmdline(c))
        ));
        let d = self.data()?;
        out.package("dplyr");
        out.line(super::model::esample_rows_line(
            &d,
            &src,
            "stata_estimates$rows",
        ));
        self.esample_new = true;
        self.e_new = true;
        self.e_ret = super::returned::Returned::Kept {
            cmd: cmd.to_string(),
            model: String::new(),
            names: &[],
        };
        self.estimates = None;
        self.estimates_lost = Some(format!(
            "predict, margins and estat after {cmd} are not translated yet"
        ));
        out.caveat(format!(
            "{cmd} as Stata 19 fits it (StatCraft's decoding); printed in R's layout, not Stata's table"
        ));
        Ok(())
    }
}
