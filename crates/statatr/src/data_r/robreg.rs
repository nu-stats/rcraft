//! P2C.9 continued, `robreg m` (the SSC package's M regression), as it
//! behaves on Stata 19 with made-up data (`plan/stata/robreg.md`); only its
//! results were read, never its code.
//!
//! - Starting values from least absolute deviations (LAD). The scale is the
//!   normalized median absolute LAD residual, the p zero residuals of the
//!   LAD fit left out: median(|r| without the p smallest) / qnorm(0.75).
//! - With that scale fixed, Huber's M estimate by reweighted least squares,
//!   k set for the efficiency asked (95% by default: k = 1.3449975).
//! - The variance is the sandwich, s^2 A^-1 B A^-1 with A = X' diag(psi') X
//!   and B = X' diag(psi^2) X, times n / (n - p); t tests on n - p degrees of
//!   freedom, and a Wald chi2 in the header.
//! - The "empty model" (a constant alone, the same way) gives the pseudo R2,
//!   1 - sum rho(r / s) / sum rho(r0 / s); the weighted R2 is the R2 of the
//!   final weighted fit. Its start is the median.
//! - When the LAD fit is not unique (indicator variables often make it so),
//!   robreg starts from a point inside the LAD solutions; an interior-point
//!   solver (quantreg's Frisch-Newton) lands on the same one. When it is
//!   unique, the exact vertex. Checked in the corpus and on 200 fits of a
//!   replication package.

use super::model::{level, Estimates, Fit, Spec};
use super::*;

/// The fit, its variance and Stata's header.
pub(super) const ROBREG_HELPER: &str = r#"stata_robreg <- function(formula, data, efficiency = 95, level = 0.95) {
  mf <- stats::model.frame(formula, data, na.action = stats::na.omit)
  tt <- attr(mf, "terms")
  y <- as.numeric(stats::model.response(mf))
  X <- stats::model.matrix(tt, mf)
  n <- nrow(X)
  p <- ncol(X)
  if (qr(X)$rank < p) stop("robreg m with collinear variables is not translated yet")
  # least absolute deviations by an interior-point solver (quantreg's
  # Frisch-Newton), which lands where robreg does when the LAD fit is not
  # unique; when it is unique (the vertex through the p nearest rows, its
  # rows' dual weights inside (-1, 1)) it is that vertex exactly
  lad <- function(X, y) {
    b <- drop(quantreg::rq.fit(X, y, tau = 0.5, method = "fn")$coefficients)
    r <- abs(drop(y - X %*% b))
    o <- order(r)
    p <- ncol(X)
    if (length(r) > p) {
      near <- o[seq_len(p)]
      ex <- tryCatch(drop(solve(X[near, , drop = FALSE], y[near])), error = function(e) NULL)
      if (!is.null(ex) && sum(abs(y - X %*% ex)) <= sum(r) + 1e-9) {
        rest <- setdiff(seq_along(y), near)
        g <- drop(crossprod(X[rest, , drop = FALSE], sign(drop(y - X %*% ex))[rest]))
        u <- tryCatch(drop(solve(t(X[near, , drop = FALSE]), -g)), error = function(e) NULL)
        if (!is.null(u) && all(abs(u) < 1 - 1e-8)) b <- ex
      }
    }
    b
  }
  k_for <- function(eff) {
    if (eff >= 100) return(Inf)
    e <- function(k) (2 * stats::pnorm(k) - 1)^2 / ((2 * stats::pnorm(k) - 1) - 2 * k * stats::dnorm(k) + 2 * k^2 * stats::pnorm(-k))
    stats::uniroot(function(k) e(k) - eff / 100, c(0.01, 10), tol = 1e-12)$root
  }
  k <- k_for(efficiency)
  mest <- function(X, y) {
    # a constant alone starts at the median (the middle two averaged)
    b <- if (ncol(X) == 1 && all(X == 1)) stats::median(y) else lad(X, y)
    r <- drop(y - X %*% b)
    s <- stats::median(sort(abs(r))[-seq_len(ncol(X))]) / stats::qnorm(0.75)
    for (i in 1:1000) {
      w <- pmin(1, k / abs(drop(y - X %*% b) / s))
      bn <- drop(solve(crossprod(X, w * X), crossprod(X, w * y)))
      if (max(abs(bn - b)) < 1e-13 * max(1, max(abs(b)))) { b <- bn; break }
      b <- bn
    }
    list(b = b, s = s, iter = i)
  }
  m <- mest(X, y)
  b <- m$b
  s <- m$s
  r <- drop(y - X %*% b)
  u <- r / s
  psi <- pmax(-k, pmin(k, u))
  dpsi <- as.numeric(abs(u) <= k)
  A <- solve(crossprod(X, dpsi * X))
  V <- s^2 * A %*% crossprod(X, psi^2 * X) %*% A * n / (n - p)
  names(b) <- colnames(X)
  dimnames(V) <- list(colnames(X), colnames(X))
  rho <- function(u) ifelse(abs(u) <= k, u^2 / 2, k * abs(u) - k^2 / 2)
  m0 <- mest(matrix(1, n, 1), y)
  r0 <- y - m0$b
  w <- pmin(1, k / abs(u))
  yb <- sum(w * y) / sum(w)
  keep <- names(b) != "(Intercept)"
  bk <- b[keep]
  chi2 <- drop(t(bk) %*% solve(V[keep, keep, drop = FALSE], bk))
  fit <- stats::lm(formula, data = data, na.action = stats::na.omit)
  fit$coefficients <- b
  fit$fitted.values <- stats::setNames(drop(X %*% b), rownames(mf))
  fit$residuals <- stats::setNames(r, rownames(mf))
  fit$stata_weights <- stats::setNames(w, rownames(mf))
  fit$stata <- list(N = n, df_m = p - sum(!keep), df_r = n - p, scale = s, scale0 = m0$s, k = k,
                    efficiency = efficiency, r2_w = 1 - sum(w * r^2) / sum(w * (y - yb)^2),
                    r2_p = 1 - sum(rho(u)) / sum(rho(r0 / s)), chi2 = chi2, iterations = m$iter,
                    title = sprintf("M regression (%g%% efficiency)", efficiency))
  list(fit = fit, b = b, V = V, df = n - p)
}
stata_robreg_header <- function(fit) {
  e <- fit$stata
  pad <- function(s) formatC(s, width = -48, flag = "-")
  row <- function(left, name, value) cat(pad(left), sprintf("%-17s = %10s\n", name, value), sep = "")
  row(e$title, "Number of obs", format(e$N, big.mark = ","))
  row("", sprintf("Wald chi2(%d)", e$df_m), sprintf("%.2f", e$chi2))
  row("", "Prob > chi2", sprintf("%.4f", stats::pchisq(e$chi2, e$df_m, lower.tail = FALSE)))
  row("", "Pseudo R2", sprintf("%.4f", e$r2_p))
  row("", "Huber k", sprintf("%.8g", e$k))
  row("", "Scale", sprintf("%.8g", e$scale))
  cat("\n")
  invisible(NULL)
}"#;

impl Session {
    /// `robreg m depvar indepvars [if] [in] [, efficiency(#) huber level(#)]`.
    pub(super) fn robreg(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "robreg")?;
        let (sub, rest) = first_word(&c.parts.main);
        if sub != "m" {
            return Err(no(format!(
                "robreg {sub} is not translated yet (robreg m is)"
            )));
        }
        if let Some(o) = opts.other(&[("efficiency", 3), ("huber", 3), ("level", 3), ("nolog", 5)])
        {
            return Err(no(format!(
                "robreg m with the option {o} is not translated yet"
            )));
        }
        if c.parts.weight.is_some() {
            return Err(no("robreg m with weights is not translated yet"));
        }
        let efficiency = match opts.arg("efficiency", 3) {
            Some(e) => match e.trim().parse::<f64>() {
                Ok(v) if (63.7..=99.9).contains(&v) => v,
                _ => {
                    return Err(no(format!(
                        "efficiency({}): only a number from 63.7 to 99.9 is translated",
                        e.trim()
                    )))
                }
            },
            None => 95.0,
        };
        let level = level(opts)?;
        let mut words = rest.split_whitespace();
        let dep = words
            .next()
            .ok_or_else(|| no("robreg m needs a variable to explain"))?;
        let spec = Spec::from_terms(dep, words)?;
        if !is_name(&spec.dep) && super::ts::ts_name(&spec.dep).is_none() {
            return Err(no(format!("{} is not a variable name", spec.dep)));
        }
        self.helpers_for(&spec);
        let right = spec.right(false)?;
        let src = self.rows_for(c, out)?;
        self.helpers.insert("stata_robreg");
        self.helpers.insert("stata_coefs");
        out.line(format!(
            "stata_m <- stata_robreg({} ~ {right}, data = {src}, efficiency = {})",
            r_name(&spec.dep),
            r_number(efficiency)
        ));
        out.line("stata_estimates <- stata_m$fit");
        out.line("stata_b <- stata_m$b");
        out.line("stata_vcov <- stata_m$V");
        out.line("stata_df <- stata_m$df");
        out.line(self.keep_sample(&src));
        let d = self.data()?;
        out.package("dplyr");
        out.package("quantreg");
        out.line(self.esample_line(&d, &src));
        self.esample_new = true;
        self.fit_robust = true;
        self.fit_cluster = false;
        self.estimates = Some(Estimates {
            fit: Fit::Robreg,
            dep: spec.dep.clone(),
            factors: spec.factors.clone(),
        });
        let args = super::returned::e_args(&spec.dep, "robust", None, &None);
        self.keep_e(c, out, &args, "");
        if !self.quiet(c) {
            out.line("stata_robreg_header(stata_estimates)");
            out.line(format!(
                "stata_coefs(stata_b, stata_vcov, stata_df, level = {}, label = \"Coefficient\")",
                r_number(level)
            ));
        }
        out.line("rm(stata_m)");
        out.caveat(
            "robreg m as the SSC package fits it: LAD starting values, the scale from them, \
             Huber's M estimate with that scale fixed, robust standard errors; matched to its \
             results, not its code",
        );
        out.caveat("printed in R's layout, not Stata's table");
        Ok(())
    }
}
