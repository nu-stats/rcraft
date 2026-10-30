//! P2B.4, `ivregress` (`2sls`, `liml`, `gmm`) and its `estat endogenous` and
//! `estat overid`.
//!
//! Written out in base R, as Stata computes it (StatCraft's probe 219 decoded
//! most of it; all of it was checked against Stata 19 here):
//!
//! - X is the endogenous and exogenous variables with the constant; Z the
//!   exogenous variables and the instruments with the constant.
//! - 2SLS: b = (X'P_Z X)^-1 X'P_Z y. LIML: the k-class estimator with kappa the
//!   smallest root of |W1 - kappa W| = 0. GMM: two steps from 2SLS with the
//!   robust weight matrix, and its sandwich variance.
//! - Without `small` the variance divides the residual sum of squares by N and
//!   the tests are z and chi-squared; with `small`, by N - k, with t and F.
//! - `vce(robust)` is the plain sandwich, times N / (N - k) with `small`.
//!   `vce(cluster v)` is the clustered sandwich with **no** adjustment without
//!   `small`, and G / (G - 1) times (N - 1) / (N - k) with it: unlike
//!   `regress`, where the adjustment is always there.
//! - R-squared is 1 - RSS / TSS from the structural residuals, so it can be
//!   negative.
//! - `estat endogenous`: Durbin's chi2 = N (RSS0 - RSS1) / RSS0 and the
//!   Wu-Hausman F, RSS1 adding the first-stage residuals. `estat overid`:
//!   Sargan's N u'P_Z u / u'u and Basmann's.

use super::model::{level, vce, Estimates, Fit, Spec};
use super::stats::r_logical;
use super::*;

/// Stata's `ivregress` and the pieces its `estat` tests read.
pub(super) const IV_HELPER: &str = r#"stata_ivregress <- function(y, endog, exog, inst, data, estimator = "2sls", small = FALSE,
                            robust = FALSE, cluster = NULL, level = 0.95, first = FALSE) {
  vars <- unique(c(all.vars(y), all.vars(endog), all.vars(exog), all.vars(inst), cluster))
  ok <- stats::complete.cases(data[vars])
  plain <- function(v) if (inherits(v, "haven_labelled")) as.vector(unclass(v)) else v
  d <- as.data.frame(lapply(data[ok, vars, drop = FALSE], plain), check.names = FALSE)
  yv <- as.numeric(stats::model.frame(y, d)[[1]])
  X1 <- stats::model.matrix(stats::update(endog, ~ . - 1), d)
  ex_terms <- stats::terms(exog)
  ex_frame <- stats::model.frame(ex_terms, d)
  X2 <- stats::model.matrix(ex_terms, ex_frame)
  Zi <- stats::model.matrix(stats::update(inst, ~ . - 1), d)
  X <- cbind(X2[, 1, drop = FALSE], X1, X2[, -1, drop = FALSE])
  Z <- cbind(X2, Zi)
  N <- length(yv)
  k <- ncol(X)
  L <- ncol(Z)
  if (L < k) {
    stop("equation not identified; must have at least as many instruments not in the regression as there are instrumented variables")
  }
  proj <- function(A, B) A %*% solve(crossprod(A), crossprod(A, B))
  MZ <- function(A) A - proj(Z, A)
  PX <- proj(Z, X)
  kappa <- 1
  G <- NA
  if (estimator == "gmm") {
    b1 <- solve(crossprod(PX, X), crossprod(PX, yv))
    u1 <- drop(yv - X %*% b1)
    Wm <- solve(crossprod(Z * u1) / N)
    Q <- crossprod(Z, X)
    A <- t(Q) %*% Wm %*% Q
    b <- drop(solve(A, t(Q) %*% Wm %*% crossprod(Z, yv)))
    u <- drop(yv - X %*% b)
    V <- N * solve(A) %*% t(Q) %*% Wm %*% (crossprod(Z * u) / N) %*% Wm %*% Q %*% solve(A)
    if (small) V <- V * N / (N - k)
    kappa <- NA_real_
  } else {
    if (estimator == "liml") {
      Yall <- cbind(yv, X1)
      W <- crossprod(MZ(Yall))
      W1 <- crossprod(Yall - proj(X2, Yall))
      kappa <- min(Re(eigen(solve(W, W1), only.values = TRUE)$values))
      Xk <- X - kappa * MZ(X)
    } else {
      Xk <- PX
    }
    A <- crossprod(Xk, X)
    b <- drop(solve(A, crossprod(Xk, yv)))
    u <- drop(yv - X %*% b)
    Ai <- solve(A)
    if (!robust) {
      V <- sum(u^2) / (if (small) N - k else N) * Ai
    } else if (is.null(cluster)) {
      V <- Ai %*% crossprod(Xk * u) %*% Ai
      if (small) V <- V * N / (N - k)
    } else {
      s <- rowsum(Xk * u, d[[cluster]])
      G <- nrow(s)
      V <- Ai %*% crossprod(s) %*% Ai
      if (small) V <- V * G / (G - 1) * (N - 1) / (N - k)
    }
  }
  names(b) <- colnames(X)
  dimnames(V) <- list(colnames(X), colnames(X))
  df <- if (!small) Inf else if (is.na(G)) N - k else G - 1
  rss <- sum(u^2)
  tss <- sum((yv - mean(yv))^2)
  r2 <- 1 - rss / tss
  rmse <- sqrt(rss / (if (small) N - k else N))
  q <- k - 1
  Wald <- drop(t(b[-1]) %*% solve(V[-1, -1, drop = FALSE], b[-1]))
  if (first) {
    cat("First-stage regressions\n-----------------------\n\n")
    for (j in seq_len(ncol(X1))) {
      bj <- drop(solve(crossprod(Z), crossprod(Z, X1[, j])))
      ej <- drop(X1[, j] - Z %*% bj)
      Vj <- sum(ej^2) / (N - L) * solve(crossprod(Z))
      names(bj) <- colnames(Z)
      cat(colnames(X1)[j], "\n")
      stata_coefs(bj, Vj, N - L, level)
      cat("\n")
    }
  }
  title <- c("2sls" = "Instrumental variables 2SLS regression", liml = "Instrumental variables LIML regression",
             gmm = "Instrumental variables GMM regression")[[estimator]]
  cat(sprintf("%-48sNumber of obs = %10d\n", title, N))
  if (small) {
    cat(sprintf("%-48sF(%d, %g) = %10.2f\n%-48sProb > F = %10.4f\n", "", q, df, Wald / q, "",
                stats::pf(Wald / q, q, df, lower.tail = FALSE)))
  } else {
    cat(sprintf("%-48sWald chi2(%d) = %10.2f\n%-48sProb > chi2 = %10.4f\n", "", q, Wald, "",
                stats::pchisq(Wald, q, lower.tail = FALSE)))
  }
  cat(sprintf("%-48sR-squared = %10.4f\n%-48sRoot MSE = %10.4f\n", "", r2, "", rmse))
  if (!is.na(G)) cat(sprintf("(Std. err. adjusted for %d clusters in %s)\n", G, cluster))
  cat("\n")
  stata_coefs(b, V, df, level)
  cat("Endogenous:", colnames(X1), "\nExogenous: ", c(colnames(X2)[-1], colnames(Zi)), "\n")
  out <- list(coefficients = b, vcov = V, df = df, estimator = estimator, kappa = kappa, N = N,
              rss = rss, rmse = rmse, r2 = r2, r2_a = 1 - (1 - r2) * (N - 1) / (N - k),
              Wald = Wald, robust = robust, y = yv, X = X, Z = Z, X1 = X1, u = u,
              endog = endog, exog = ex_terms, xlevels = stats::.getXlevels(ex_terms, ex_frame))
  class(out) <- "stata_ivregress"
  invisible(out)
}
stata_iv_xb <- function(fit, newdata) {
  X1 <- stats::model.matrix(stats::update(fit$endog, ~ . - 1),
                            stats::model.frame(stats::update(fit$endog, ~ . - 1), newdata, na.action = stats::na.pass))
  tt <- stats::delete.response(fit$exog)
  X2 <- stats::model.matrix(tt, stats::model.frame(tt, newdata, xlev = fit$xlevels, na.action = stats::na.pass))
  X <- cbind(X2[, 1, drop = FALSE], X1, X2[, -1, drop = FALSE])
  drop(X[, names(fit$coefficients), drop = FALSE] %*% fit$coefficients)
}
stata_iv_endogenous <- function(fit) {
  N <- fit$N
  k <- ncol(fit$X)
  ls_rss <- function(A) sum((fit$y - A %*% solve(crossprod(A), crossprod(A, fit$y)))^2)
  v <- fit$X1 - fit$Z %*% solve(crossprod(fit$Z), crossprod(fit$Z, fit$X1))
  rss0 <- ls_rss(fit$X)
  rss1 <- ls_rss(cbind(fit$X, v))
  q <- ncol(fit$X1)
  durbin <- N * (rss0 - rss1) / rss0
  wu <- ((rss0 - rss1) / q) / (rss1 / (N - k - q))
  cat("Tests of endogeneity\nH0: Variables are exogenous\n\n")
  cat(sprintf("  Durbin (score) chi2(%d)          = %9.6g  (p = %.4f)\n", q, durbin, stats::pchisq(durbin, q, lower.tail = FALSE)))
  cat(sprintf("  Wu-Hausman F(%d,%d)              = %9.6g  (p = %.4f)\n", q, N - k - q, wu, stats::pf(wu, q, N - k - q, lower.tail = FALSE)))
  invisible(c(durbin = durbin, wu = wu, df = q, wudf_r = N - k - q))
}
stata_iv_overid <- function(fit) {
  u <- fit$u
  Pu <- fit$Z %*% solve(crossprod(fit$Z), crossprod(fit$Z, u))
  upu <- sum(u * Pu)
  N <- fit$N
  L <- ncol(fit$Z)
  l <- L - ncol(fit$X)
  if (l < 1) stop("model is exactly identified; no test of overidentifying restrictions")
  sargan <- N * upu / sum(u^2)
  basmann <- (N - L) * upu / (sum(u^2) - upu)
  cat("Tests of overidentifying restrictions:\n\n")
  cat(sprintf("  Sargan (score) chi2(%d) = %9.6g  (p = %.4f)\n", l, sargan, stats::pchisq(sargan, l, lower.tail = FALSE)))
  cat(sprintf("  Basmann chi2(%d)        = %9.6g  (p = %.4f)\n", l, basmann, stats::pchisq(basmann, l, lower.tail = FALSE)))
  invisible(c(sargan = sargan, basmann = basmann, df = l))
}"#;

impl Session {
    pub(super) fn ivregress(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "ivregress")?;
        if let Some(o) = opts.other(&[
            ("small", 5),
            ("robust", 1),
            ("vce", 3),
            ("level", 3),
            ("first", 5),
        ]) {
            return Err(no(format!(
                "ivregress with the option {o} is not translated yet"
            )));
        }
        let main = c.parts.main.trim();
        let (estimator, rest) = first_word(main);
        if !matches!(estimator, "2sls" | "liml" | "gmm") {
            return Err(no(format!(
                "ivregress {estimator}: only 2sls, liml and gmm are estimators"
            )));
        }
        let (open, close) = match (rest.find('('), rest.rfind(')')) {
            (Some(a), Some(b)) if a < b => (a, b),
            _ => return Err(no("ivregress needs (endogenous = instruments)")),
        };
        let inside = rest.get(open + 1..close).unwrap_or("");
        let outside = format!(
            "{} {}",
            rest.get(..open).unwrap_or(""),
            rest.get(close + 1..).unwrap_or("")
        );
        if outside.contains('(') || inside.contains('(') {
            return Err(no(
                "ivregress with more than one (endogenous = instruments) is not translated",
            ));
        }
        let (endog, inst) = inside
            .split_once('=')
            .ok_or_else(|| no("ivregress needs (endogenous = instruments)"))?;
        let plain = |list: &str, what: &str| -> Result<Vec<String>, Untranslated> {
            let v: Vec<String> = list.split_whitespace().map(str::to_string).collect();
            if v.is_empty() {
                return Err(no(format!("ivregress needs {what}")));
            }
            if let Some(bad) = v.iter().find(|w| !is_name(w)) {
                return Err(no(format!(
                    "{bad}: only plain variable names are translated inside the parentheses"
                )));
            }
            Ok(v)
        };
        let endog = plain(endog, "an endogenous variable")?;
        let inst = plain(inst, "instruments")?;
        let mut words = outside.split_whitespace();
        let Some(dep) = words.next() else {
            return Err(no("ivregress needs a variable to explain"));
        };
        if !is_name(dep) {
            return Err(no(format!("{dep} is not a variable name")));
        }
        let spec = Spec::from_terms(dep, words)?;
        self.helpers_for(&spec);
        let (robust, cluster) = vce(opts)?;
        if estimator == "gmm" && (robust || cluster.is_some()) {
            return Err(no(
                "ivregress gmm with a vce() of its own is not translated yet (its default is \
                 the robust weight matrix)",
            ));
        }
        if estimator == "liml" && robust {
            return Err(no(
                "ivregress liml with robust or clustered errors is not translated yet",
            ));
        }
        let level = level(opts)?;
        let src = self.rows_for(c, out)?;
        let names = |v: &[String]| v.iter().map(|n| r_name(n)).collect::<Vec<_>>().join(" + ");
        self.helpers.insert("stata_coefs");
        self.helpers.insert("stata_ivregress");
        let call = format!(
            "stata_ivregress(~{}, ~{}, ~{}, ~{}, data = {src}, estimator = \"{estimator}\", small = {}, robust = {}{}, level = {}, first = {})",
            r_name(dep),
            names(&endog),
            spec.right(false),
            names(&inst),
            r_logical(opts.has("small", 5)),
            r_logical(robust),
            cluster
                .as_ref()
                .map(|v| format!(", cluster = {}", r_text(v)))
                .unwrap_or_default(),
            r_number(level),
            r_logical(opts.has("first", 5))
        );
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
            fit: Fit::Ivregress,
            dep: dep.to_string(),
            factors: spec.factors.clone(),
        });
        if !opts.has("small", 5) {
            out.caveat("without small, as in Stata: the variance divides by N, with z tests");
        }
        if cluster.is_some() && !opts.has("small", 5) {
            out.caveat(
                "clustered errors without small carry no G / (G - 1), as Stata's ivregress \
                 (unlike its regress)",
            );
        }
        out.caveat("printed in R's layout, not Stata's table");
        Ok(())
    }

    /// `estat endogenous` and `estat overid` after `ivregress`.
    pub(super) fn estat_iv(&mut self, sub: &str, out: &mut Out) -> Result<(), Untranslated> {
        let is = |full: &str, min: usize| sub.len() >= min && full.starts_with(sub);
        if is("endogenous", 4) {
            self.helpers.insert("stata_ivregress");
            out.line(
                "if (stata_estimates$robust) stop(\"estat endogenous after robust errors is not translated yet\")",
            );
            out.line("stata_iv_endogenous(stata_estimates)");
            Ok(())
        } else if is("overid", 4) {
            self.helpers.insert("stata_ivregress");
            out.line(
                "if (stata_estimates$robust || stata_estimates$estimator != \"2sls\") stop(\"estat overid after liml, gmm or robust errors is not translated yet\")",
            );
            out.line("stata_iv_overid(stata_estimates)");
            Ok(())
        } else {
            Err(no(format!(
                "estat {sub} after ivregress is not translated yet"
            )))
        }
    }
}
