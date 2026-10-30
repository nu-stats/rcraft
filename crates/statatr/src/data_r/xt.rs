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
pub(super) const XTREG_HELPER: &str = r#"stata_xtreg <- function(formula, data, panel, type = "re", robust = FALSE, cluster = NULL, level = 0.95) {
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
    for (v in colnames(X)[!varies]) cat("note: ", v, " omitted because of collinearity.\n", sep = "")
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
  out <- list(coefficients = b, vcov = V, df = df, type = type, N = N, n = n, r2 = r2,
              sigma_e = sqrt(s2e), terms = tt, xlevels = stats::.getXlevels(tt, mf))
  if (type != "be") {
    out$sigma_u <- sigma_u
    out$rho <- sigma_u^2 / (sigma_u^2 + s2e)
    cat(sprintf("sigma_u = %.7g   sigma_e = %.7g   rho = %.7g (fraction of variance due to u_i)\n",
                sigma_u, sqrt(s2e), out$rho))
  }
  if (type == "fe") {
    out$F_u <- F_u
    out$corr <- corr_u
    if (!robust) cat(sprintf("F test that all u_i=0: F(%d, %d) = %.2f    Prob > F = %.4f\n",
                             n - 1, N - n - (kf - 1), F_u, stats::pf(F_u, n - 1, N - n - (kf - 1), lower.tail = FALSE)))
  }
  if (type == "re") out$theta <- theta
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
        if let Some(o) = opts.other(&[]) {
            return Err(no(format!(
                "xtset with the option {o} is not translated yet"
            )));
        }
        let words: Vec<&str> = c.parts.main.split_whitespace().collect();
        let (panel, time) = match words.as_slice() {
            [] => {
                return Err(no(
                    "xtset alone (showing the settings) is not translated yet",
                ))
            }
            [p] => (*p, None),
            [p, t] => (*p, Some(*t)),
            _ => return Err(no("xtset takes a panel variable and a time variable")),
        };
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
        ]) {
            return Err(no(format!(
                "xtreg with the option {o} is not translated yet"
            )));
        }
        let kind = match (opts.has("fe", 2), opts.has("be", 2)) {
            (true, false) => "fe",
            (false, true) => "be",
            (false, false) => "re",
            (true, true) => return Err(no("xtreg takes one of fe, re and be")),
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
        let level = level(opts)?;
        let spec = Spec::read(c, "xtreg")?;
        self.helpers_for(&spec);
        let src = self.rows_for(c, out)?;
        self.helpers.insert("stata_coefs");
        self.helpers.insert("stata_xtreg");
        let call = format!(
            "stata_xtreg({} ~ {}, data = {src}, panel = {}, type = \"{kind}\", robust = {}{}, level = {})",
            r_name(&spec.dep),
            spec.right(false),
            r_text(&panel),
            r_logical(robust),
            cluster
                .as_ref()
                .map(|v| format!(", cluster = {}", r_text(v)))
                .unwrap_or_default(),
            r_number(level)
        );
        // the table is printed as the fit is made; quietly keeps it to itself
        if Self::quiet(c) {
            out.line(format!(
                "invisible(utils::capture.output(stata_estimates <- {call}))"
            ));
        } else {
            out.line(format!("stata_estimates <- {call}"));
        }
        out.line("stata_b <- stata_estimates$coefficients");
        out.line("stata_vcov <- stata_estimates$vcov");
        out.line("stata_df <- stata_estimates$df");
        self.estimates = Some(Estimates {
            fit: Fit::Xtreg,
            dep: spec.dep.clone(),
            factors: spec.factors.clone(),
        });
        match kind {
            "fe" => {
                out.caveat("within regression, as Stata's fe: the constant is the grand mean's")
            }
            "re" => {
                out.caveat("Swamy-Arora random effects, as Stata's re (the default), with z tests")
            }
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
