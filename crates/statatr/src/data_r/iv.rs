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
                            robust = FALSE, cluster = NULL, level = 0.95, first = FALSE,
                            wmatrix = "robust", wcluster = NULL, vce_type = NULL, igmm = FALSE) {
  vars <- unique(c(all.vars(y), all.vars(endog), all.vars(exog), all.vars(inst), cluster, wcluster))
  ok <- stats::complete.cases(data[vars])
  plain <- function(v) if (inherits(v, "haven_labelled")) as.vector(unclass(v)) else v
  d <- as.data.frame(lapply(data[ok, vars, drop = FALSE], plain), check.names = FALSE)
  yv <- as.numeric(stats::model.frame(y, d)[[1]])
  # the endogenous variables and the instruments without a constant of their
  # own (a factor variable keeps its base level out)
  bare <- function(f) {
    M <- stats::model.matrix(f, stats::model.frame(f, d))
    M[, colnames(M) != "(Intercept)", drop = FALSE]
  }
  X1 <- bare(endog)
  Zi <- bare(inst)
  ex_terms <- stats::terms(exog)
  ex_frame <- stats::model.frame(ex_terms, d)
  X2 <- stats::model.matrix(ex_terms, ex_frame)
  cons <- "(Intercept)" %in% colnames(X2)
  X <- if (cons) cbind(X2[, 1, drop = FALSE], X1, X2[, -1, drop = FALSE]) else cbind(X1, X2)
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
  J <- NA_real_
  if (estimator == "gmm") {
    Q <- crossprod(Z, X)
    zy <- crossprod(Z, yv)
    b1 <- solve(crossprod(PX, X), crossprod(PX, yv))
    weight <- function(u) {
      if (wmatrix == "unadjusted") return(solve(sum(u^2) / N * crossprod(Z) / N))
      if (wmatrix == "cluster") return(solve(crossprod(rowsum(Z * u, d[[wcluster]])) / N))
      solve(crossprod(Z * u) / N)
    }
    step <- function(W) drop(solve(t(Q) %*% W %*% Q, t(Q) %*% W %*% zy))
    u1 <- drop(yv - X %*% b1)
    Wm <- weight(u1)
    b <- step(Wm)
    if (igmm) {
      # Stata's igmm: until both b and W change by less than 1e-6
      rel <- function(a, o) max(abs(a - o) / (abs(o) + 1))
      for (it in seq_len(100)) {
        Wn <- weight(drop(yv - X %*% b))
        bn <- step(Wn)
        done <- rel(bn, b) < 1e-6 && rel(Wn, Wm) < 1e-6
        b <- bn
        Wm <- Wn
        if (done) break
      }
    }
    u <- drop(yv - X %*% b)
    A <- t(Q) %*% Wm %*% Q
    Ai <- solve(A)
    gbar <- crossprod(Z, u) / N
    J <- if (wmatrix == "unadjusted") {
      drop(N * t(gbar) %*% weight(u) %*% gbar)
    } else {
      drop(N * t(gbar) %*% Wm %*% gbar)
    }
    vt <- if (is.null(vce_type)) wmatrix else vce_type
    if (vt == "unadjusted") {
      # with an unadjusted weight the variance is s^2 N (Q' (Z'Z / N)^-1 Q)^-1, s^2 from the final residuals
      V <- if (wmatrix == "unadjusted") {
        sum(u^2) / (if (small) N - k else N) * N * solve(t(Q) %*% solve(crossprod(Z) / N) %*% Q)
      } else {
        N * Ai
      }
    } else {
      S <- if (vt == "cluster") {
        s <- rowsum(Z * u, d[[if (is.null(cluster)) wcluster else cluster]])
        G <- nrow(s)
        crossprod(s) / N
      } else {
        crossprod(Z * u) / N
      }
      V <- N * Ai %*% t(Q) %*% Wm %*% S %*% Wm %*% Q %*% Ai
      if (small) V <- V * if (is.na(G)) N / (N - k) else G / (G - 1) * (N - 1) / (N - k)
    }
    robust <- vt != "unadjusted"
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
    # robust scores are the instruments' fitted X times u, for liml too
    if (!robust) {
      V <- sum(u^2) / (if (small) N - k else N) * Ai
    } else if (is.null(cluster)) {
      V <- Ai %*% crossprod(PX * u) %*% Ai
      if (small) V <- V * N / (N - k)
    } else {
      s <- rowsum(PX * u, d[[cluster]])
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
  r2 <- if (cons) 1 - rss / tss else NA_real_
  rmse <- sqrt(rss / (if (small) N - k else N))
  slopes <- names(b) != "(Intercept)"
  q <- sum(slopes)
  Wald <- drop(t(b[slopes]) %*% solve(V[slopes, slopes, drop = FALSE], b[slopes]))
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
  cat(sprintf("%-48sR-squared = %10s\n%-48sRoot MSE = %10.4f\n", "", if (cons) sprintf("%.4f", r2) else ".", "", rmse))
  if (!is.na(G)) cat(sprintf("(Std. err. adjusted for %d clusters)\n", G))
  cat("\n")
  stata_coefs(b, V, df, level)
  cat("Endogenous:", colnames(X1), "\nExogenous: ", c(colnames(X2)[colnames(X2) != "(Intercept)"], colnames(Zi)), "\n")
  if (estimator == "gmm") cat(sprintf("Hansen's J = %.6g\n", J))
  # for margins and predict: one set of terms for all the regressors
  rhs <- c(attr(stats::terms(endog), "term.labels"), attr(ex_terms, "term.labels"))
  allt <- stats::terms(stats::reformulate(if (length(rhs)) rhs else "1", intercept = cons))
  out <- list(coefficients = b, vcov = V, df = df, estimator = estimator, kappa = kappa, N = N,
              rss = rss, rmse = rmse, r2 = r2, r2_a = 1 - (1 - r2) * (N - 1) / (N - k),
              Wald = Wald, robust = robust, y = yv, X = X, Z = Z, X1 = X1, X2 = X2, Zi = Zi, u = u, J = J,
              endog = endog, exog = ex_terms, terms = allt, rows = which(ok), cluster = cluster,
              xlevels = stats::.getXlevels(allt, stats::model.frame(allt, d)))
  class(out) <- "stata_ivregress"
  invisible(out)
}
stata_iv_xb <- function(fit, newdata) {
  tt <- stats::delete.response(fit$terms)
  X <- stats::model.matrix(tt, stats::model.frame(tt, newdata, xlev = fit$xlevels, na.action = stats::na.pass))
  drop(X[, names(fit$coefficients), drop = FALSE] %*% fit$coefficients)
}
stata_iv_endogenous <- function(fit) {
  if (fit$estimator != "2sls") stop("estat endogenous is not available after ivregress ", fit$estimator, ", as in Stata 19")
  N <- fit$N
  k <- ncol(fit$X)
  q <- ncol(fit$X1)
  v <- fit$X1 - fit$Z %*% solve(crossprod(fit$Z), crossprod(fit$Z, fit$X1))
  ls_res <- function(A, b) b - A %*% solve(crossprod(A), crossprod(A, b))
  if (fit$robust && !is.null(fit$cluster)) stop("this test after clustered errors is not translated yet")
  if (fit$robust) {
    # Wooldridge's robust score test: N less the residual sum of squares of 1
    # on the OLS residuals times v, v made orthogonal to every regressor
    m <- drop(ls_res(fit$X, fit$y)) * ls_res(fit$X, v)
    score <- N - sum(ls_res(m, rep(1, N))^2)
    cat("Tests of endogeneity\nH0: Variables are exogenous\n\n")
    cat(sprintf("  Robust score chi2(%d)            = %9.6g  (p = %.4f)\n", q, score, stats::pchisq(score, q, lower.tail = FALSE)))
    return(invisible(c(r_score = score, df = q)))
  }
  rss0 <- sum(ls_res(fit$X, fit$y)^2)
  rss1 <- sum(ls_res(cbind(fit$X, v), fit$y)^2)
  durbin <- N * (rss0 - rss1) / rss0
  wu <- ((rss0 - rss1) / q) / (rss1 / (N - k - q))
  cat("Tests of endogeneity\nH0: Variables are exogenous\n\n")
  cat(sprintf("  Durbin (score) chi2(%d)          = %9.6g  (p = %.4f)\n", q, durbin, stats::pchisq(durbin, q, lower.tail = FALSE)))
  cat(sprintf("  Wu-Hausman F(%d,%d)              = %9.6g  (p = %.4f)\n", q, N - k - q, wu, stats::pf(wu, q, N - k - q, lower.tail = FALSE)))
  invisible(c(durbin = durbin, wu = wu, df = q, wudf_r = N - k - q))
}
stata_iv_overid <- function(fit) {
  u <- fit$u
  N <- fit$N
  L <- ncol(fit$Z)
  l <- L - ncol(fit$X)
  if (l < 1) stop("model is exactly identified; no test of overidentifying restrictions")
  cat("Tests of overidentifying restrictions:\n\n")
  if (fit$estimator == "gmm") {
    cat(sprintf("  Hansen's J chi2(%d) = %9.6g  (p = %.4f)\n", l, fit$J, stats::pchisq(fit$J, l, lower.tail = FALSE)))
    return(invisible(c(HansenJ = fit$J, df = l)))
  }
  if (fit$estimator == "liml") {
    ar <- N * (fit$kappa - 1)
    bf <- (fit$kappa - 1) * (N - L) / l
    cat(sprintf("  Anderson-Rubin chi2(%d) = %9.6g  (p = %.4f)\n", l, ar, stats::pchisq(ar, l, lower.tail = FALSE)))
    cat(sprintf("  Basmann F(%d, %d)        = %9.6g  (p = %.4f)\n", l, N - L, bf, stats::pf(bf, l, N - L, lower.tail = FALSE)))
    return(invisible(c(ar = ar, basmann = bf, df = l, basmann_df_d = N - L)))
  }
  if (fit$robust && !is.null(fit$cluster)) stop("this test after clustered errors is not translated yet")
  if (fit$robust) {
    # Wooldridge's robust score test, which is two-step GMM's J
    # the weight from the 2SLS residuals, evaluated at the two-step estimates
    Z <- fit$Z
    X <- fit$X
    W <- solve(crossprod(Z * u) / N)
    Q <- crossprod(Z, X)
    b2 <- solve(t(Q) %*% W %*% Q, t(Q) %*% W %*% crossprod(Z, fit$y))
    g <- crossprod(Z, drop(fit$y - X %*% b2)) / N
    score <- drop(N * t(g) %*% W %*% g)
    cat(sprintf("  Score chi2(%d) = %9.6g  (p = %.4f)\n", l, score, stats::pchisq(score, l, lower.tail = FALSE)))
    return(invisible(c(score = score, df = l)))
  }
  Pu <- fit$Z %*% solve(crossprod(fit$Z), crossprod(fit$Z, u))
  upu <- sum(u * Pu)
  sargan <- N * upu / sum(u^2)
  basmann <- (N - L) * upu / (sum(u^2) - upu)
  cat(sprintf("  Sargan (score) chi2(%d) = %9.6g  (p = %.4f)\n", l, sargan, stats::pchisq(sargan, l, lower.tail = FALSE)))
  cat(sprintf("  Basmann chi2(%d)        = %9.6g  (p = %.4f)\n", l, basmann, stats::pchisq(basmann, l, lower.tail = FALSE)))
  invisible(c(sargan = sargan, basmann = basmann, df = l))
}
stata_iv_firststage <- function(fit) {
  if (!is.null(fit$cluster)) stop("estat firststage after clustered errors is not translated yet")
  N <- fit$N
  Z <- fit$Z
  X2 <- fit$X2
  L <- ncol(Z)
  l <- ncol(fit$Zi)
  res <- function(A, b) b - A %*% solve(crossprod(A), crossprod(A, b))
  rows <- lapply(seq_len(ncol(fit$X1)), function(j) {
    x <- fit$X1[, j]
    e <- drop(res(Z, x))
    r2 <- 1 - sum(e^2) / sum((x - mean(x))^2)
    mx <- res(X2, x)
    mz <- res(X2, fit$Zi)
    pr2 <- sum(drop(mz %*% solve(crossprod(mz), crossprod(mz, mx)))^2) / sum(mx^2)
    rr <- sum(mx^2)
    if (fit$robust) {
      # the robust Wald test of the excluded instruments, HC1
      bj <- solve(crossprod(Z), crossprod(Z, x))
      B <- solve(crossprod(Z))
      Vj <- B %*% crossprod(Z * e) %*% B * N / (N - L)
      sel <- (L - l + 1):L
      F <- drop(t(bj[sel]) %*% solve(Vj[sel, sel, drop = FALSE], bj[sel])) / l
    } else {
      F <- ((rr - sum(e^2)) / l) / (sum(e^2) / (N - L))
    }
    c(r2 = r2, r2_a = 1 - (1 - r2) * (N - 1) / (N - L), partial_r2 = pr2, F = F, df1 = l, df2 = N - L,
      p = stats::pf(F, l, N - L, lower.tail = FALSE))
  })
  tab <- do.call(rbind, rows)
  rownames(tab) <- colnames(fit$X1)
  cat("First-stage regression summary statistics\n\n")
  print(tab, digits = 6)
  # the Cragg-Donald minimum eigenvalue statistic
  V1 <- res(Z, fit$X1)
  Svv <- crossprod(V1) / (N - L)
  mz <- res(X2, fit$Zi)
  mx <- res(X2, fit$X1)
  P <- t(mx) %*% mz %*% solve(crossprod(mz), t(mz) %*% mx)
  ev <- eigen(Svv, symmetric = TRUE)
  Sih <- ev$vectors %*% diag(1 / sqrt(ev$values), length(ev$values)) %*% t(ev$vectors)
  mineig <- min(eigen(Sih %*% P %*% Sih, symmetric = TRUE, only.values = TRUE)$values) / l
  cat(sprintf("\nMinimum eigenvalue statistic = %.6g\n", mineig))
  invisible(list(singleresults = tab, mineig = mineig))
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
            ("noconstant", 5),
            ("wmatrix", 2),
            ("igmm", 4),
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
            // Stata's own words: one group only
            return Err(no(
                "invalid syntax: the syntax is (all instrumented variables = instrument variables)",
            ));
        }
        let (endog, inst) = inside
            .split_once('=')
            .ok_or_else(|| no("ivregress needs (endogenous = instruments)"))?;
        let mut words = outside.split_whitespace();
        let Some(dep) = words.next() else {
            return Err(no("ivregress needs a variable to explain"));
        };
        if !is_name(dep) {
            return Err(no(format!("{dep} is not a variable name")));
        }
        // factor variables are allowed inside the parentheses too
        let endog_spec = Spec::from_terms(dep, endog.split_whitespace())?;
        let inst_spec = Spec::from_terms(dep, inst.split_whitespace())?;
        if endog_spec.right(false) == "1" {
            return Err(no("ivregress needs an endogenous variable"));
        }
        if inst_spec.right(false) == "1" {
            return Err(no("ivregress needs instruments"));
        }
        let mut spec = Spec::from_terms(dep, words)?;
        spec.factors.extend(endog_spec.factors.clone());
        self.helpers_for(&spec);
        self.helpers_for(&inst_spec);
        // gmm reads vce() itself: unadjusted, robust or cluster v
        let gmm = estimator == "gmm";
        let (robust, cluster, vce_type) = if gmm {
            match opts.arg("vce", 3).as_deref().map(str::trim) {
                None if opts.has("robust", 1) => (true, None, Some("robust")),
                None => (false, None, None),
                Some(v) if v.len() >= 2 && "unadjusted".starts_with(v) => {
                    (false, None, Some("unadjusted"))
                }
                Some(v) if !v.is_empty() && "robust".starts_with(v) => (true, None, Some("robust")),
                Some(v) => match v.split_once(char::is_whitespace) {
                    Some((w, cv))
                        if w.len() >= 2 && "cluster".starts_with(w) && is_name(cv.trim()) =>
                    {
                        (true, Some(cv.trim().to_string()), Some("cluster"))
                    }
                    _ => return Err(no(format!("ivregress gmm, vce({v}) is not translated yet"))),
                },
            }
        } else {
            let (r, cl) = vce(opts)?;
            (r, cl, None)
        };
        let (wmatrix, wcluster) = match opts.arg("wmatrix", 2).as_deref().map(str::trim) {
            None => ("robust", None),
            Some(_) if !gmm => return Err(no("wmatrix() goes with ivregress gmm")),
            Some(w) if !w.is_empty() && "robust".starts_with(w) => ("robust", None),
            Some(w) if w.len() >= 2 && "unadjusted".starts_with(w) => ("unadjusted", None),
            Some(w) => match w.split_once(char::is_whitespace) {
                Some((k, cv)) if k.len() >= 2 && "cluster".starts_with(k) && is_name(cv.trim()) => {
                    ("cluster", Some(cv.trim().to_string()))
                }
                _ => {
                    return Err(no(format!(
                        "ivregress gmm, wmatrix({w}) is not translated yet"
                    )))
                }
            },
        };
        if opts.has("igmm", 4) && !gmm {
            return Err(no("igmm goes with ivregress gmm"));
        }
        let level = level(opts)?;
        let rows = self.rows_for(c, out)?;
        let src = rows;
        self.helpers.insert("stata_coefs");
        self.helpers.insert("stata_ivregress");
        let mut extra = String::new();
        if gmm {
            extra.push_str(&format!(", wmatrix = {}", r_text(wmatrix)));
            if let Some(w) = &wcluster {
                extra.push_str(&format!(", wcluster = {}", r_text(w)));
            }
            if let Some(v) = vce_type {
                extra.push_str(&format!(", vce_type = {}", r_text(v)));
            }
            if opts.has("igmm", 4) {
                extra.push_str(", igmm = TRUE");
            }
        }
        let call = format!(
            "stata_ivregress(~{}, ~{}, ~{}, ~{}, data = {src}, estimator = \"{estimator}\", small = {}, robust = {}{}, level = {}, first = {}{extra})",
            r_name(dep),
            endog_spec.right(false),
            spec.right(opts.has("noconstant", 5)),
            inst_spec.right(false),
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
            fit: Fit::Ivregress,
            dep: dep.to_string(),
            factors: spec.factors.clone(),
        });
        // e(): gmm only with its default weight and variance, as checked against Stata
        if gmm && (vce_type.is_some() || wmatrix != "robust" || opts.has("igmm", 4)) {
            self.e_ret = super::returned::Returned::Lost(
                "e() after ivregress gmm with vce(), wmatrix() or igmm is not kept in R yet".into(),
            );
            self.e_new = true;
        } else {
            let vce = if cluster.is_some() {
                "cluster"
            } else if robust || gmm {
                "robust"
            } else {
                "oim"
            };
            let args = super::returned::e_args(dep, vce, cluster.as_deref(), &None);
            self.keep_e(c, out, &args, estimator);
        }
        if !opts.has("small", 5) {
            out.caveat("without small, as in Stata: the variance divides by N, with z tests");
        }
        if cluster.is_some() && !opts.has("small", 5) {
            out.caveat(
                "clustered errors without small carry no G / (G - 1), as Stata's ivregress \
                 (unlike its regress)",
            );
        }
        if gmm {
            out.caveat(
                "gmm as Stata's: the weight matrix from the 2SLS residuals (robust unless \
                 wmatrix() says otherwise), and the variance of the same kind unless vce() says",
            );
        }
        out.caveat("printed in R's layout, not Stata's table");
        Ok(())
    }

    /// `estat endogenous` and `estat overid` after `ivregress`.
    pub(super) fn estat_iv(&mut self, sub: &str, out: &mut Out) -> Result<(), Untranslated> {
        let is = |full: &str, min: usize| sub.len() >= min && full.starts_with(sub);
        self.helpers.insert("stata_ivregress");
        if is("endogenous", 4) {
            out.line("stata_iv_endogenous(stata_estimates)");
        } else if is("overid", 4) {
            out.line("stata_iv_overid(stata_estimates)");
        } else if is("firststage", 4) {
            out.line("stata_iv_firststage(stata_estimates)");
        } else if sub == "ic" {
            return Err(no(
                "estat ic is not available after ivregress, as in Stata 19 (r(321))",
            ));
        } else {
            return Err(no(format!(
                "estat {sub} after ivregress is not translated yet"
            )));
        }
        Ok(())
    }
}
