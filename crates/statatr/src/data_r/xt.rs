//! P2B.4, panel data: `xtset` and `xtreg` (`fe`, `re`, `be`).
//!
//! Written out in base R rather than left to a package, because Stata's
//! estimators are specific and an R package's defaults differ. What Stata 19
//! does, as checked against it (and as StatCraft's probe 174 decoded):
//!
//! - `fe`: least squares on y - ybar_i + ybar (the grand mean added back, so
//!   there is a constant), df = N - n - k; a regressor with no variation
//!   within a panel is dropped with Stata's note. sigma_u is the standard
//!   deviation over the panels of ybar_i - xbar_i b - _cons; the F test that
//!   all u_i = 0 compares with the pooled fit.
//! - `be`: least squares on the panel means.
//! - `re` (the default; Swamy-Arora): sigma_e^2 from `fe`, sigma_u^2 =
//!   s_be^2 - sigma_e^2 / T-bar with T-bar the harmonic mean of the T_i,
//!   theta_i = 1 - sqrt(sigma_e^2 / (T_i sigma_u^2 + sigma_e^2)), least
//!   squares on the quasi-demeaned data with that fit's own variance, and z
//!   tests.
//! - `vce(robust)` is clustered on the panel; `vce(cluster v)` on v. Both are
//!   scaled by G / (G - 1) and (N - 1) / (N - k), with k the regressors and the
//!   constant (not the panel effects), and `fe` tests on G - 1 degrees of
//!   freedom.
//! - R-squared within, between and overall are the squared correlations of
//!   the matching transforms of y and xb.

use super::model::{level, vce, Estimates, Fit, Spec};
use super::stats::r_logical;
use super::*;

/// `xtset`: checks that panel and time identify the rows, and says what the
/// panel is.
pub(super) const XTSET_HELPER: &str = r#"stata_xtset <- function(d, panel, time = NULL) {
  if (!is.null(time)) {
    key <- d[c(panel, time)]
    key <- key[stats::complete.cases(key), , drop = FALSE]
    if (anyDuplicated(key)) stop("repeated time values within panel")
  }
  Ti <- table(d[[panel]])
  cat("Panel variable: ", panel, if (length(unique(Ti)) == 1) " (balanced)" else " (unbalanced)", "\n", sep = "")
  if (!is.null(time)) {
    tv <- d[[time]]
    cat(" Time variable: ", time, ", ", min(tv, na.rm = TRUE), " to ", max(tv, na.rm = TRUE), "\n", sep = "")
  }
  invisible(NULL)
}"#;

/// Stata's `xtreg`, fe, be and re, with its robust and clustered variances.
pub(super) const XTREG_HELPER: &str = r#"stata_xtreg <- function(formula, data, panel, type = "re", robust = FALSE, cluster = NULL, level = 0.95, corr = "exchangeable") {
  mf <- stats::model.frame(formula, data, na.action = stats::na.omit)
  rows <- match(rownames(mf), rownames(data))
  g <- data[[panel]][rows]
  cl <- if (is.null(cluster)) g else data[[cluster]][rows]
  keep <- !is.na(g) & !is.na(cl)
  mf <- mf[keep, , drop = FALSE]
  g <- g[keep]
  cl <- cl[keep]
  tt <- attr(mf, "terms")
  y <- as.numeric(stats::model.response(mf))
  X <- stats::model.matrix(tt, mf)
  X <- X[, colnames(X) != "(Intercept)", drop = FALSE]
  N <- length(y)
  gi <- as.integer(factor(g))
  n <- max(gi)
  Ti <- tabulate(gi, n)
  mean_i <- function(v) rowsum(v, gi) / Ti
  ybar <- drop(mean_i(y))
  Xbar <- mean_i(X)
  Xd <- X - Xbar[gi, , drop = FALSE]
  varies <- colSums(Xd^2) > 1e-12 * pmax(colSums(X^2), 1)
  if (type %in% c("mle", "pa")) {
    r <- stata_xt_other(y, X, gi, type, corr = corr, robust = robust, level = level)
    kb <- ncol(X) + 1
    bb <- r$b[seq_len(kb)]
    slope <- bb[-1]
    W <- drop(t(slope) %*% solve(r$V[2:kb, 2:kb, drop = FALSE], slope))
    title <- if (type == "mle") "Random-effects ML regression" else "GEE population-averaged model"
    cat(sprintf("%-48sNumber of obs     = %10d\n", title, N))
    cat(sprintf("%-48sNumber of groups  = %10d\n", paste("Group variable:", panel), n))
    if (type == "mle") {
      cat(sprintf("LR chi2(%d) = %.2f    Prob > chi2 = %.4f\nLog likelihood = %.7g\n",
                  kb - 1, r$chi2, stats::pchisq(r$chi2, kb - 1, lower.tail = FALSE), r$ll))
    } else {
      cat(sprintf("Correlation: %s    Wald chi2(%d) = %.2f    Prob > chi2 = %.4f\nScale parameter = %.7g\n",
                  r$corr, kb - 1, W, stats::pchisq(W, kb - 1, lower.tail = FALSE), r$phi))
    }
    cat("\n")
    stata_coefs(r$b, r$V, Inf, level)
    out <- list(coefficients = bb, b = r$b, vcov = r$V, df = Inf, type = type, N = N, n = n,
                terms = tt, xlevels = stats::.getXlevels(tt, mf), rows = rows[keep], panel = panel,
                response = stats::formula(tt)[[2]])
    if (type == "mle") {
      out$sigma_u <- r$sigma_u
      out$sigma_e <- r$sigma_e
      out$rho <- r$sigma_u^2 / (r$sigma_u^2 + r$sigma_e^2)
      out$ll <- r$ll
      out$ll_0 <- r$ll_0
      out$chi2 <- r$chi2
      out$chi2_c <- r$chi2_c
      xbf <- drop(cbind(1, X) %*% bb)
      su2 <- r$sigma_u^2
      out$u <- stats::setNames(drop(Ti * su2 / (Ti * su2 + r$sigma_e^2) * mean_i(y - xbf)), levels(factor(g)))
      cat(sprintf("rho = %.7g\nLR test of sigma_u=0: chibar2(01) = %.2f    Prob >= chibar2 = %.4f\n",
                  out$rho, r$chi2_c, 0.5 * stats::pchisq(r$chi2_c, 1, lower.tail = FALSE)))
    } else {
      out$phi <- r$phi
      out$alpha <- r$alpha
    }
    class(out) <- "stata_xtreg"
    return(invisible(out))
  }
  ls <- function(Z, v) {
    b <- solve(crossprod(Z), crossprod(Z, v))
    list(b = drop(b), e = drop(v - Z %*% b), bread = solve(crossprod(Z)))
  }
  sandwich <- function(Z, e, bread, k) {
    s <- rowsum(Z * e, cl)
    G <- nrow(s)
    list(V = bread %*% crossprod(s) %*% bread * G / (G - 1) * (N - 1) / (N - k), G = G)
  }
  # fe, which re needs too for sigma_e
  Xf <- Xd[, varies, drop = FALSE] + matrix(colMeans(X[, varies, drop = FALSE]), N, sum(varies), byrow = TRUE)
  Zf <- cbind("(Intercept)" = 1, Xf)
  ff <- ls(Zf, y - ybar[gi] + mean(y))
  kf <- ncol(Zf)
  s2e <- sum(ff$e^2) / (N - n - (kf - 1))
  # be, which re needs too for sigma_u
  Zb <- cbind("(Intercept)" = 1, Xbar)
  fb <- ls(Zb, ybar)
  s2b <- sum(fb$e^2) / (n - ncol(Zb))
  G <- NA
  if (type == "fe") {
    # Stata's name for the column: 1.g for factor(g)1
    stata_nm <- function(v) sub("^(factor|stata_ib)\\(([^,()]+)(, \"[^\"]*\")?\\)(.+)$", "\\4.\\2", v)
    for (v in colnames(X)[!varies]) cat("note: ", stata_nm(v), " omitted because of collinearity.\n", sep = "")
    b <- ff$b
    if (robust) {
      s <- sandwich(Zf, ff$e, ff$bread, kf)
      V <- s$V
      G <- s$G
      df <- G - 1
    } else {
      V <- s2e * ff$bread
      df <- N - n - (kf - 1)
    }
    slope <- b[-1]
    u <- ybar - drop(Xbar[, varies, drop = FALSE] %*% slope) - b[1]
    sigma_u <- stats::sd(u)
    pooled <- ls(cbind(1, X[, varies, drop = FALSE]), y)
    F_u <- ((sum(pooled$e^2) - sum(ff$e^2)) / (n - 1)) / s2e
    corr_u <- stats::cor(u[gi], drop(X[, varies, drop = FALSE] %*% slope))
    Xs <- X[, varies, drop = FALSE]
  } else if (type == "be") {
    b <- fb$b
    V <- s2b * fb$bread
    df <- n - ncol(Zb)
    slope <- b[-1]
    Xs <- X
  } else {
    Tbar <- n / sum(1 / Ti)
    s2u <- max(0, s2b - s2e / Tbar)
    theta <- 1 - sqrt(s2e / (Ti * s2u + s2e))
    th <- theta[gi]
    Zr <- cbind("(Intercept)" = 1 - th, X - th * Xbar[gi, , drop = FALSE])
    fr <- ls(Zr, y - th * ybar[gi])
    b <- fr$b
    if (robust) {
      s <- sandwich(Zr, fr$e, fr$bread, ncol(Zr))
      V <- s$V
      G <- s$G
    } else {
      V <- sum(fr$e^2) / (N - ncol(Zr)) * fr$bread
    }
    df <- Inf
    slope <- b[-1]
    sigma_u <- sqrt(s2u)
    Xs <- X
  }
  names(b) <- c("(Intercept)", colnames(Xs))
  dimnames(V) <- list(names(b), names(b))
  xb <- drop(Xs %*% slope)
  r2 <- c(within = stats::cor(y - ybar[gi], xb - drop(mean_i(xb))[gi])^2,
          between = stats::cor(ybar, drop(mean_i(xb)))^2,
          overall = stats::cor(y, xb)^2)
  k <- length(slope)
  W <- drop(t(slope) %*% solve(V[-1, -1, drop = FALSE], slope))
  title <- c(fe = "Fixed-effects (within) regression", be = "Between regression (regression on group means)",
             re = "Random-effects GLS regression")[[type]]
  cat(sprintf("%-48sNumber of obs     = %10d\n", title, N))
  cat(sprintf("%-48sNumber of groups  = %10d\n", paste("Group variable:", panel), n))
  cat(sprintf("R-squared: Within = %.4f  Between = %.4f  Overall = %.4f\n", r2[1], r2[2], r2[3]))
  cat(sprintf("Obs per group: min = %d  avg = %.1f  max = %d\n", min(Ti), N / n, max(Ti)))
  if (is.finite(df)) {
    cat(sprintf("F(%d,%g) = %.2f    Prob > F = %.4f\n", k, df, W / k, stats::pf(W / k, k, df, lower.tail = FALSE)))
  } else {
    cat(sprintf("%s chi2(%d) = %.2f    Prob > chi2 = %.4f\n", "Wald", k, W, stats::pchisq(W, k, lower.tail = FALSE)))
  }
  if (type == "fe") cat(sprintf("corr(u_i, Xb) = %.4f\n", corr_u))
  if (!is.na(G)) cat(sprintf("(Std. err. adjusted for %d clusters)\n", G))
  cat("\n")
  stata_coefs(b, V, df, level)
  out <- list(coefficients = b, b = b, vcov = V, df = df, type = type, N = N, n = n, r2 = r2, Ti = Ti, G = G,
              sigma_e = sqrt(s2e), terms = tt, xlevels = stats::.getXlevels(tt, mf))
  if (type != "be") {
    out$sigma_u <- sigma_u
    out$rho <- sigma_u^2 / (sigma_u^2 + s2e)
    cat(sprintf("sigma_u = %.7g   sigma_e = %.7g   rho = %.7g (fraction of variance due to u_i)\n",
                sigma_u, sqrt(s2e), out$rho))
  }
  if (type == "fe") {
    out$F_u <- F_u
    out$df_f <- c(n - 1, N - n - (kf - 1))
    out$corr <- corr_u
    if (!robust) cat(sprintf("F test that all u_i=0: F(%d, %d) = %.2f    Prob > F = %.4f\n",
                             n - 1, N - n - (kf - 1), F_u, stats::pf(F_u, n - 1, N - n - (kf - 1), lower.tail = FALSE)))
  }
  if (type == "re") out$theta <- theta
  # what predict and estat need afterwards: the rows used, each panel's u_i
  # (fe: ybar_i - xbar_i b - _cons; re: the BLUP), and fe's log likelihood
  out$rows <- rows[keep]
  out$panel <- panel
  out$response <- stats::formula(tt)[[2]]
  xbf <- drop(cbind(1, Xs) %*% b)
  if (type == "fe") out$u <- stats::setNames(drop(u), levels(factor(g)))
  if (type == "re") out$u <- stats::setNames(drop(Ti * s2u / (Ti * s2u + s2e) * mean_i(y - xbf)), levels(factor(g)))
  if (type == "fe") {
    out$ll <- -N / 2 * (1 + log(2 * pi) + log(sum(ff$e^2) / N))
    out$ll_0 <- -N / 2 * (1 + log(2 * pi) + log(sum((y - ybar[gi])^2) / N))
  }
  class(out) <- "stata_xtreg"
  invisible(out)
}
stata_xt_xb <- function(fit, newdata) {
  tt <- stats::delete.response(fit$terms)
  mf <- stats::model.frame(tt, newdata, xlev = fit$xlevels, na.action = stats::na.pass)
  X <- stats::model.matrix(tt, mf)
  drop(X[, names(fit$coefficients), drop = FALSE] %*% fit$coefficients)
}"#;

impl Session {
    pub(super) fn xtset(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        // the units only label the time variable and delta() only matters for
        // the time-series operators, so none of them changes what is fitted
        let units = [
            ("clocktime", 5),
            ("daily", 1),
            ("weekly", 1),
            ("monthly", 1),
            ("quarterly", 1),
            ("halfyearly", 1),
            ("yearly", 1),
            ("generic", 1),
            ("delta", 1),
            ("format", 1),
            ("noquery", 5),
        ];
        if let Some(o) = opts.other(&units) {
            return Err(no(format!(
                "xtset with the option {o} is not translated yet"
            )));
        }
        if c.options
            .iter()
            .any(|o| units.iter().any(|(u, m)| o.is(u, *m)))
        {
            out.caveat(
                "the time unit and delta() only label the time and matter for lags, which are \
                 not translated; nothing fitted here changes",
            );
        }
        let words: Vec<&str> = c.parts.main.split_whitespace().collect();
        let (panel, time) = match words.as_slice() {
            // xtset alone: the settings again
            [] => match &self.panel {
                Some(p) => (p.clone(), self.panel_time.clone()),
                None => return Err(no("panel variable not set; use xtset varname ...")),
            },
            [p] => (p.to_string(), None),
            [p, t] => (p.to_string(), Some(t.to_string())),
            _ => return Err(no("xtset takes a panel variable and a time variable")),
        };
        let (panel, time) = (panel.as_str(), time.as_deref());
        for v in [Some(panel), time].into_iter().flatten() {
            if !is_name(v) {
                return Err(no(format!("{v} is not a variable name")));
            }
        }
        let d = self.data()?;
        self.helpers.insert("stata_xtset");
        out.line(format!(
            "stata_xtset({d}, {}{})",
            r_text(panel),
            time.map(|t| format!(", {}", r_text(t))).unwrap_or_default()
        ));
        self.panel = Some(panel.to_string());
        self.panel_time = time.map(str::to_string);
        Ok(())
    }

    pub(super) fn xtreg(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "xtreg")?;
        if let Some(o) = opts.other(&[
            ("fe", 2),
            ("re", 2),
            ("be", 2),
            ("robust", 1),
            ("vce", 3),
            ("level", 3),
            ("i", 1),
            ("mle", 3),
            ("pa", 2),
            ("corr", 1),
        ]) {
            return Err(no(format!(
                "xtreg with the option {o} is not translated yet"
            )));
        }
        let picked: Vec<&str> = [("fe", 2), ("be", 2), ("mle", 3), ("pa", 2), ("re", 2)]
            .into_iter()
            .filter(|(o, m)| opts.has(o, *m))
            .map(|(o, _)| o)
            .collect();
        let kind = match picked.as_slice() {
            [] => "re",
            [one] => *one,
            _ => return Err(no("xtreg takes one of fe, re, be, mle and pa")),
        };
        let corr = match opts.arg("corr", 1).as_deref().map(str::trim) {
            None => "exchangeable",
            Some(_) if kind != "pa" => return Err(no("corr() goes with xtreg, pa")),
            Some(c) if c.len() >= 3 && "exchangeable".starts_with(c) => "exchangeable",
            Some(c) if c.len() >= 3 && "independent".starts_with(c) => "independent",
            Some(c) => return Err(no(format!("xtreg, pa corr({c}) is not translated yet"))),
        };
        let panel = match opts.arg("i", 1) {
            Some(p) if is_name(p.trim()) => p.trim().to_string(),
            Some(p) => return Err(no(format!("i({p}) needs one variable name"))),
            None => self
                .panel
                .clone()
                .ok_or_else(|| no("must specify panelvar; use xtset"))?,
        };
        let (robust, cluster) = vce(opts)?;
        if kind == "be" && robust {
            return Err(no(
                "xtreg, be with robust or clustered errors: Stata does not allow it",
            ));
        }
        if kind == "mle" && robust {
            return Err(no(
                "xtreg, mle with robust or clustered errors is not translated yet",
            ));
        }
        if kind == "pa" && cluster.is_some() {
            return Err(no(
                "xtreg, pa with vce(cluster) is not translated yet (vce(robust) is)",
            ));
        }
        let level = level(opts)?;
        let spec = Spec::read(c, "xtreg")?;
        self.helpers_for(&spec);
        let src = self.rows_for(c, out)?;
        self.helpers.insert("stata_coefs");
        self.helpers.insert("stata_xtreg");
        if matches!(kind, "mle" | "pa") {
            self.helpers.insert("stata_xt_other");
        }
        let call = format!(
            "stata_xtreg({} ~ {}, data = {src}, panel = {}, type = \"{kind}\", robust = {}{}, level = {}{})",
            r_name(&spec.dep),
            spec.right(false),
            r_text(&panel),
            r_logical(robust),
            cluster
                .as_ref()
                .map(|v| format!(", cluster = {}", r_text(v)))
                .unwrap_or_default(),
            r_number(level),
            if kind == "pa" {
                format!(", corr = {}", r_text(corr))
            } else {
                String::new()
            }
        );
        // the table is printed as the fit is made; quietly keeps it to itself
        if Self::quiet(c) {
            out.line(format!(
                "invisible(utils::capture.output(stata_estimates <- {call}))"
            ));
        } else {
            out.line(format!("stata_estimates <- {call}"));
        }
        out.line("stata_b <- stata_estimates$b");
        out.line("stata_vcov <- stata_estimates$vcov");
        out.line("stata_df <- stata_estimates$df");
        // the estimation sample, for margins and e(sample)
        out.line(format!(
            "stata_data <- {src}[stata_estimates$rows, , drop = FALSE]"
        ));
        let d = self.data()?;
        out.package("dplyr");
        out.line(super::model::esample_rows_line(
            &d,
            &src,
            "stata_estimates$rows",
        ));
        self.esample_new = true;
        self.estimates = Some(Estimates {
            fit: Fit::Xtreg,
            dep: spec.dep.clone(),
            factors: spec.factors.clone(),
        });
        if matches!(kind, "fe" | "re" | "be") {
            // Stata's robust here clusters on the panel
            let (vce, cl) = match (&cluster, robust) {
                (Some(v), _) => ("cluster", Some(v.as_str())),
                (None, true) => ("cluster", Some(panel.as_str())),
                _ => ("oim", None),
            };
            let args = super::returned::e_args(&spec.dep, vce, cl, &None);
            self.keep_e(c, out, &args, kind);
        } else {
            self.e_ret = super::returned::Returned::Lost(format!(
                "e() after xtreg, {kind} is not kept in R yet"
            ));
            self.e_new = true;
        }
        match kind {
            "fe" => {
                out.caveat("within regression, as Stata's fe: the constant is the grand mean's")
            }
            "re" => {
                out.caveat("Swamy-Arora random effects, as Stata's re (the default), with z tests")
            }
            "mle" => out.caveat(
                "maximum-likelihood random effects, as Stata's mle; the standard errors are from \
                 the exact Hessian, Stata's from a numerical one, so they can differ in the fifth \
                 digit",
            ),
            "pa" => out.caveat(
                "GEE population-averaged, as Stata's pa, iterated until b changes by less than \
                 1e-6, where Stata stops",
            ),
            _ => out.caveat("regression on the panel means, as Stata's be"),
        }
        if robust {
            out.caveat(
                "clustered on the panel (or the cluster variable), scaled by G / (G - 1) and \
                 (N - 1) / (N - k), as Stata's",
            );
        }
        out.caveat("printed in R's layout, not Stata's table");
        Ok(())
    }
}

/// `predict` after `xtreg`: `xb`; `u`, each panel's u_i (for every row of a
/// panel that was in the estimation sample, as Stata fills it in); `e`, the
/// rest of the residual; `ue`, both; `xbu`, xb and u.
pub(super) const XT_PREDICT_HELPER: &str = r#"stata_xt_predict <- function(fit, newdata, stat) {
  xb <- stata_xt_xb(fit, newdata)
  if (stat == "xb") return(xb)
  if (is.null(fit$u)) stop("predict, ", stat, " after xtreg, be is not available")
  u <- unname(fit$u[as.character(newdata[[fit$panel]])])
  if (stat == "u") return(u)
  if (stat == "xbu") return(xb + u)
  y <- eval(fit$response, newdata)
  if (stat == "ue") return(y - xb)
  y - xb - u
}"#;

/// `estat ic` after `xtreg, fe`: the within regression's normal log likelihood,
/// with the coefficients (the constant included) as its degrees of freedom.
pub(super) const XT_IC_HELPER: &str = r#"stata_xt_ic <- function(fit) {
  if (is.null(fit$ll)) stop("estat ic is not available after xtreg, ", fit$type, ", as in Stata 19")
  k <- length(fit$coefficients)
  tab <- data.frame(N = fit$N, "ll(null)" = fit$ll_0, "ll(model)" = fit$ll, df = k,
                    AIC = -2 * fit$ll + 2 * k, BIC = -2 * fit$ll + log(fit$N) * k, check.names = FALSE)
  rownames(tab) <- "."
  print(tab, digits = 8)
  invisible(tab)
}"#;

/// `xtreg, mle` and `xtreg, pa`, decoded in Stata 19 (see the P2B.4 notes).
///
/// - `mle`: the random-effects likelihood maximised in b, ln sigma_u and
///   ln sigma_e (analytic gradient, Newton steps on its numerical Jacobian);
///   e(b) has `/sigma_u` and `/sigma_e` with delta-method errors; the model
///   test is the LR against the constant-only model, and the test of
///   sigma_u = 0 the chi-bar-squared against the pooled fit.
/// - `pa` (GEE, `corr(exchangeable)` by default, or `independent`): alpha is
///   the within-panel cross products over phi sum T_i (T_i - 1), phi is
///   sum e^2 / N, b the GLS fit with that correlation, iterated from least
///   squares until the largest relative change in b is below 1e-6, where
///   Stata stops; the variance is phi (sum X_i' R^-1 X_i)^-1, or with
///   `robust` the sandwich clustered on the panel with G / (G - 1).
pub(super) const XT_OTHER_HELPER: &str = r#"stata_xt_other <- function(y, X, gi, type, corr = "exchangeable", robust = FALSE, level = 0.95) {
  N <- length(y)
  n <- max(gi)
  Ti <- tabulate(gi, n)
  Z <- cbind("(Intercept)" = 1, X)
  k <- ncol(Z)
  if (type == "pa") {
    gls <- function(a) {
      A <- matrix(0, k, k)
      B <- numeric(k)
      for (i in seq_len(n)) {
        r <- gi == i
        R <- matrix(a, Ti[i], Ti[i])
        diag(R) <- 1
        Ri <- solve(R)
        Zi <- Z[r, , drop = FALSE]
        A <- A + crossprod(Zi, Ri %*% Zi)
        B <- B + drop(crossprod(Zi, Ri %*% y[r]))
      }
      list(b = solve(A, B), A = A)
    }
    a <- 0
    f <- gls(0)
    phi <- NA
    for (it in seq_len(100)) {
      e <- drop(y - Z %*% f$b)
      phi <- sum(e^2) / N
      if (corr == "independent") break
      S1 <- rowsum(e, gi)[, 1]
      S2 <- rowsum(e^2, gi)[, 1]
      a <- sum(S1^2 - S2) / (phi * sum(Ti * (Ti - 1)))
      g <- gls(a)
      done <- max(abs(g$b - f$b) / (abs(f$b) + 1)) < 1e-6
      f <- g
      if (done) break
    }
    b <- f$b
    # the variance and Stata's e(phi) take phi from the final b's residuals
    phi <- sum(drop(y - Z %*% b)^2) / N
    names(b) <- colnames(Z)
    Ai <- solve(f$A)
    if (robust) {
      e <- drop(y - Z %*% b)
      M <- matrix(0, k, k)
      for (i in seq_len(n)) {
        r <- gi == i
        R <- matrix(a, Ti[i], Ti[i])
        diag(R) <- 1
        s <- crossprod(Z[r, , drop = FALSE], solve(R, e[r]))
        M <- M + tcrossprod(s)
      }
      V <- Ai %*% M %*% Ai * n / (n - 1)
    } else {
      V <- phi * Ai
    }
    dimnames(V) <- list(names(b), names(b))
    return(list(b = b, V = V, phi = phi, alpha = a, corr = corr))
  }
  # mle
  ll <- function(p, Z) {
    kk <- ncol(Z)
    su <- exp(2 * p[kk + 1])
    se <- exp(2 * p[kk + 2])
    e <- drop(y - Z %*% p[seq_len(kk)])
    S1 <- rowsum(e, gi)[, 1]
    S2 <- rowsum(e^2, gi)[, 1]
    a <- se + Ti * su
    -0.5 * sum(Ti * log(2 * pi) + (Ti - 1) * log(se) + log(a) + (S2 - su * S1^2 / a) / se)
  }
  gr <- function(p, Z) {
    kk <- ncol(Z)
    su <- exp(2 * p[kk + 1])
    se <- exp(2 * p[kk + 2])
    e <- drop(y - Z %*% p[seq_len(kk)])
    S1 <- rowsum(e, gi)[, 1]
    S2 <- rowsum(e^2, gi)[, 1]
    a <- se + Ti * su
    gb <- drop(crossprod(Z, e - (su / a)[gi] * S1[gi])) / se
    gsu <- -0.5 * sum(Ti / a - S1^2 / a^2)
    gse <- -0.5 * sum((Ti - 1) / se + 1 / a - S2 / se^2 + su * S1^2 * (a + se) / (a^2 * se^2))
    c(gb, gsu * 2 * su, gse * 2 * se)
  }
  fit_ml <- function(Z) {
    kk <- ncol(Z)
    b0 <- solve(crossprod(Z), crossprod(Z, y))
    s0 <- sum((y - Z %*% b0)^2) / N
    p <- c(drop(b0), log(sqrt(s0 / 2)), log(sqrt(s0 / 2)))
    o <- stats::optim(p, function(q) -ll(q, Z), function(q) -gr(q, Z), method = "BFGS",
                      control = list(reltol = 1e-15, maxit = 2000))
    p <- o$par
    hess <- function(p) stats::optimHess(p, function(q) -ll(q, Z), function(q) -gr(q, Z),
                                         control = list(ndeps = rep(1e-5, kk + 2)))
    for (i in seq_len(25)) {
      step <- solve(hess(p), gr(p, Z))
      p <- p + step
      if (max(abs(step)) < 1e-13) break
    }
    list(p = p, ll = ll(p, Z), H = hess(p))
  }
  m <- fit_ml(Z)
  m0 <- fit_ml(Z[, 1, drop = FALSE])
  bp <- solve(crossprod(Z), crossprod(Z, y))
  ll_p <- -N / 2 * (log(2 * pi * sum((y - Z %*% bp)^2) / N) + 1)
  p <- m$p
  Vp <- solve(m$H)
  su <- exp(p[k + 1])
  sg <- exp(p[k + 2])
  J <- diag(c(rep(1, k), su, sg))
  V <- J %*% Vp %*% J
  b <- c(p[seq_len(k)], su, sg)
  names(b) <- c(colnames(Z), "/sigma_u", "/sigma_e")
  dimnames(V) <- list(names(b), names(b))
  list(b = b, V = V, ll = m$ll, ll_0 = m0$ll, ll_c = ll_p, sigma_u = su, sigma_e = sg,
       chi2 = 2 * (m$ll - m0$ll), chi2_c = max(0, 2 * (m$ll - ll_p)))
}"#;
