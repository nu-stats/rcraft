//! P2B.4, the maximum-likelihood models: `logit`, `probit`, `poisson`,
//! `nbreg`, `ologit`, `oprobit` and `mlogit`.
//!
//! R fits each of them (`stats::glm`, `MASS::glm.nb`, `MASS::polr`,
//! `nnet::multinom`), but its defaults are not Stata's in several places,
//! and each place is handled here rather than left to R:
//!
//! - `logit` and `probit` take the outcome as 1 where it is not 0 (and not
//!   missing), so a 0/2 variable works as it does in Stata.
//! - Stata's standard errors come from the observed information. For logit
//!   and poisson that is the same matrix as R's (their links are canonical),
//!   but for probit R's `summary()` uses the expected information, which is
//!   a different number, so the observed one is worked out.
//! - `robust` after a maximum-likelihood model is the sandwich scaled by
//!   `N / (N - 1)`, and `vce(cluster v)` by `G / (G - 1)`: not regress's HC1.
//! - `nbreg` is Stata's NB2 with `alpha = 1 / theta`; its standard errors are
//!   from the whole likelihood, alpha included, where `glm.nb` holds theta
//!   fixed.
//! - `mlogit`'s base outcome is the most frequent one, where R takes the
//!   first level.
//! - intervals are Wald intervals, as Stata's are; R's `confint()` on a glm
//!   would give profile-likelihood ones.
//! - the iterations are run to a tighter tolerance than R's default, so the
//!   estimates agree with Stata's to the digits shown.

use super::model::{cluster_rows, level, vce, Estimates, Fit, Spec};
use super::stats::r_logical;
use super::weights::W;
use super::*;

/// nbreg, ologit, oprobit and mlogit: the terms Stata omits for
/// collinearity are found before the fit (`stata_collinear_rhs`, in the
/// order written) and the model is fitted without them; R's fits there
/// would not leave the column out themselves.
fn collinear_first(dep: &str, right: &str, src: &str) -> String {
    format!(
        "stata_cl <- stata_collinear_rhs({}, {src})",
        r_text(&format!("{dep} ~ {right}"))
    )
}

/// The formula with the right-hand side `collinear_first` left.
fn with_rhs(response: &str) -> String {
    format!(
        "stats::as.formula(paste({}, stata_cl$rhs))",
        r_text(&format!("{response} ~"))
    )
}

/// The omitted terms kept on the fit, for `e(b)`, `_b[]` and the notes.
const KEEP_OMITTED: &str =
    "stata_estimates$omitted <- stata_cl$omitted\nstata_estimates$stata_written <- stata_cl$written";

/// The observed information of a binary or Poisson glm, and its robust or
/// clustered sandwich the way Stata scales them.
pub(super) const GLM_VCOV_HELPER: &str = r#"stata_glm_vcov <- function(fit, robust = FALSE, cluster = NULL, kind = "") {
  # the weights the model was given (1 without): frequency ones count as copies
  pw <- fit$prior.weights
  X <- stats::model.matrix(fit)
  # a column omitted for collinearity has zero rows, as in Stata's e(V)
  full <- colnames(X)
  X <- X[, !is.na(stats::coef(fit))[full], drop = FALSE]
  eta <- fit$linear.predictors
  y <- fit$y
  fam <- fit$family
  # worked out at the final estimates: glm's own $weights are a step behind
  mu <- fam$linkinv(eta)
  me <- fam$mu.eta(eta)
  if (fam$link == "probit") {
    P <- stats::pnorm(eta)
    f <- stats::dnorm(eta)
    w <- y * (eta * f * P + f^2) / P^2 + (1 - y) * (f^2 - eta * f * (1 - P)) / (1 - P)^2
  } else {
    w <- me^2 / fam$variance(mu)
  }
  V <- solve(crossprod(X * (w * pw), X))
  if (robust) {
    r <- (y - mu) * me / fam$variance(mu)
    s <- X * (pw * r)
    if (is.null(cluster)) {
      n <- if (kind == "f") sum(pw) else nrow(X)
      if (kind == "f") s <- X * (sqrt(pw) * r)
      V <- V %*% crossprod(s) %*% V * n / (n - 1)
    } else {
      sg <- rowsum(s, cluster)
      G <- nrow(sg)
      V <- V %*% crossprod(sg) %*% V * G / (G - 1)
    }
  }
  dimnames(V) <- list(colnames(X), colnames(X))
  if (length(full) == ncol(X)) return(V)
  out <- matrix(0, length(full), length(full), dimnames = list(full, full))
  out[colnames(X), colnames(X)] <- V
  out
}"#;

/// The variance of nbreg's coefficients and `ln(alpha)` together, from the
/// observed information of the whole likelihood (the analytic gradient,
/// differenced once).
pub(super) const NBREG_VCOV_HELPER: &str = r#"stata_nbreg_vcov <- function(fit) {
  X <- stats::model.matrix(fit)
  # at the end of Stata's own path: the exact inverse of -H there
  V <- fit$stata_count$V
  if (!is.null(V) && all(dim(V) == ncol(X) + 1)) {
    nm <- c(colnames(X), "/lnalpha")
    dimnames(V) <- list(nm, nm)
    return(V)
  }
  y <- fit$y
  off <- stats::model.offset(stats::model.frame(fit))
  if (is.null(off)) off <- 0
  k <- ncol(X)
  # the weights the model was given (1 without)
  pw <- fit$prior.weights
  if (is.null(pw)) pw <- rep(1, length(y))
  nll <- function(p) {
    mu <- exp(drop(X %*% p[seq_len(k)]) + off)
    th <- exp(-p[k + 1])
    -sum(pw * (lgamma(y + th) - lgamma(th) - lgamma(y + 1) + th * log(th / (th + mu)) + y * log(mu / (th + mu))))
  }
  grad <- function(p) {
    mu <- exp(drop(X %*% p[seq_len(k)]) + off)
    th <- exp(-p[k + 1])
    gb <- crossprod(X, pw * (y - mu) / (1 + mu / th))
    gt <- sum(pw * (digamma(y + th) - digamma(th) + log(th / (th + mu)) + (mu - y) / (th + mu)))
    -c(gb, -th * gt)
  }
  p <- c(stats::coef(fit), log(1 / fit$theta))
  H <- stats::optimHess(p, nll, grad, control = list(ndeps = rep(1e-5, k + 1)))
  V <- solve(H)
  nm <- c(colnames(X), "/lnalpha")
  dimnames(V) <- list(nm, nm)
  V
}"#;

/// Newton steps on the observed information after `glm`'s probit fit.
/// `glm` scores with the expected information and stops on the change in
/// deviance, which for probit leaves the score near 1e-7; Stata's estimates
/// sit at about 1e-11. The fit is then refreshed from the polished estimates
/// (one `glm` step from there moves them by no more than the score).
pub(super) const PROBIT_NEWTON_HELPER: &str = r#"stata_probit_newton <- function(fit) {
  # a column omitted for collinearity stays out (0 to start glm again)
  full <- stats::coef(fit)
  est <- !is.na(full)
  X <- stats::model.matrix(fit)[, est, drop = FALSE]
  y <- fit$y
  off <- if (is.null(fit$offset)) 0 else fit$offset
  b <- full[est]
  pw <- fit$prior.weights
  for (i in 1:25) {
    eta <- drop(X %*% b) + off
    P <- stats::pnorm(eta)
    f <- stats::dnorm(eta)
    s <- colSums(X * (pw * (y * f / P - (1 - y) * f / (1 - P))))
    w <- y * (eta * f * P + f^2) / P^2 + (1 - y) * (f^2 - eta * f * (1 - P)) / (1 - P)^2
    step <- solve(crossprod(X * (w * pw), X), s)
    b <- b + step
    if (max(abs(step)) < 1e-15 * (1 + max(abs(b)))) break
  }
  start <- rep(0, length(full))
  start[est] <- b
  # glm's check for collinear columns follows epsilon / 1000: with one left
  # out, the first fit's epsilon leaves it out again
  eps <- if (all(est)) 1e-15 else 1e-12
  # weights that are not whole numbers make binomial() warn; Stata takes them
  out <- withCallingHandlers(
    stats::update(fit, start = start, control = stats::glm.control(epsilon = eps, maxit = 1)),
    warning = function(w) if (grepl("non-integer", conditionMessage(w))) invokeRestart("muffleWarning")
  )
  # the terms stata_collinear fitted it without
  out$omitted <- fit$omitted
  out$stata_written <- fit$stata_written
  # Stata reports where its own Newton-Raphson stops, which can be short of
  # the maximum (probe 596): its path from the constant-only fit is followed
  # when the model has a constant and no offset (the start known there)
  cons <- colnames(X) == "(Intercept)"
  if (sum(cons) == 1 && all(off == 0)) {
    pos <- y != 0
    pbar <- sum(pw * pos) / sum(pw)
    b0 <- rep(0, ncol(X))
    b0[cons] <- stats::qnorm(pbar)
    ll <- function(b) {
      z <- drop(X %*% b)
      sum(pw * ifelse(pos, stats::pnorm(z, log.p = TRUE), stats::pnorm(z, lower.tail = FALSE, log.p = TRUE)))
    }
    deriv <- function(b) {
      z <- drop(X %*% b)
      lam <- ifelse(pos, exp(stats::dnorm(z, log = TRUE) - stats::pnorm(z, log.p = TRUE)),
                    -exp(stats::dnorm(z, log = TRUE) - stats::pnorm(z, lower.tail = FALSE, log.p = TRUE)))
      list(ll = ll(b), g = colSums(X * (pw * lam)), nh = crossprod(X * (pw * lam * (lam + z)), X))
    }
    path <- stata_ml_path(ll, deriv, b0)
    if (path$converged) {
      bs <- path$b
      out$coefficients[est] <- bs
      eta <- drop(X %*% bs)
      out$linear.predictors <- eta
      out$fitted.values <- stats::pnorm(eta)
      lv <- ll(bs)
      out$deviance <- -2 * lv
      out$aic <- -2 * lv + 2 * out$rank
    }
  }
  out
}
# Stata's Newton-Raphson (StatCraft's decoding, plan/stata/logit.md, probes
# 061-063): the full step, halved while it lowers the likelihood, else
# stepped forward by 1/8, 1/4, ... while it rises; where -H is not positive
# definite, -H + lambda I; stops when g'(-H)^-1 g <= 1e-5 and the likelihood
# moved by at most 1e-7 or the coefficients by at most 1e-6 (relative)
stata_ml_path <- function(ll, deriv, b, max_iter = 300) {
  concave_inv <- function(a) {
    ok <- tryCatch({ chol(a); TRUE }, error = function(e) FALSE)
    if (ok) return(list(inv = solve(a), nc = FALSE))
    lambda <- 1e-4 * max(1, max(abs(diag(a))))
    for (i in 1:80) {
      m <- a + diag(lambda, nrow(a))
      if (!inherits(try(chol(m), silent = TRUE), "try-error")) return(list(inv = solve(m), nc = TRUE))
      lambda <- 2 * lambda
    }
    stop("the likelihood is not concave and RCraft could not step as Stata does", call. = FALSE)
  }
  reldif <- function(a, b) abs(a - b) / (abs(b) + 1)
  d <- deriv(b)
  ci <- concave_inv(d$nh)
  prev <- NULL
  converged <- FALSE
  iter <- 0
  # each point's likelihood (Stata's iteration log), and whether a step was halved or
  # -H was not concave (which Stata marks in its log)
  lls <- numeric()
  odd <- ci$nc
  repeat {
    iter <- iter + 1
    lls <- c(lls, d$ll)
    step <- drop(ci$inv %*% d$g)
    if (!is.null(prev) && !ci$nc) {
      nr <- sum(d$g * step)
      if (abs(nr) <= 1e-5 && (reldif(d$ll, prev$ll) <= 1e-7 || max(reldif(b, prev$b)) <= 1e-6)) {
        converged <- TRUE
        break
      }
    }
    if (iter > max_iter) break
    t <- 1
    nxt <- ll(b + t * step)
    halved <- FALSE
    while ((is.nan(nxt) || nxt < d$ll) && t > 1e-15) {
      halved <- TRUE
      odd <- TRUE
      t <- t / 2
      nxt <- ll(b + t * step)
    }
    if (!halved) {
      ex <- 0.125
      repeat {
        fu <- ll(b + (t + ex) * step)
        if (!is.nan(fu) && fu > nxt) {
          t <- t + ex
          nxt <- fu
          ex <- 2 * ex
        } else break
      }
    }
    prev <- list(b = b, ll = d$ll)
    b <- b + t * step
    d <- deriv(b)
    ci <- concave_inv(d$nh)
    if (ci$nc) odd <- TRUE
  }
  list(b = b, converged = converged, lls = lls, odd = odd)
}"#;

/// mlogit's coefficients as one named vector, in the order of `vcov()`.
pub(super) const MULTINOM_B_HELPER: &str = r#"stata_multinom_b <- function(fit) {
  cf <- stats::coef(fit)
  if (is.null(dim(cf))) cf <- matrix(cf, 1, dimnames = list(fit$lev[2], names(cf)))
  stats::setNames(as.vector(t(cf)), paste(rep(rownames(cf), each = ncol(cf)), colnames(cf), sep = ":"))
}"#;

/// The lines above Stata's table: observations, the test of the model
/// (likelihood ratio, or Wald when robust), pseudo R-squared and the log
/// likelihood.
pub(super) const ML_HEADER_HELPER: &str = r#"stata_ml_ll <- function(fit) {
  ll <- as.numeric(stats::logLik(fit))
  if (inherits(fit, "glm") && fit$family$family == "binomial") {
    # by hand: R's binomial logLik rounds weights that are not whole numbers
    mu <- fit$fitted.values
    ll <- sum(fit$prior.weights * (fit$y * log(mu) + (1 - fit$y) * log(1 - mu)))
  }
  if (inherits(fit, "negbin")) {
    ll0 <- NA_real_
  } else if (inherits(fit, "glm")) {
    ll0 <- ll + (fit$deviance - fit$null.deviance) / 2
  } else {
    wt <- stats::model.weights(fit$model)
    nj <- if (is.null(wt)) table(fit$model[[1]]) else tapply(wt, fit$model[[1]], sum)
    nj <- nj[nj > 0]
    ll0 <- sum(nj * log(nj / sum(nj)))
  }
  c(ll = ll, ll_0 = ll0)
}
stata_ml_header <- function(fit, robust = FALSE, nobs = NULL) {
  l <- stata_ml_ll(fit)
  ll <- l[["ll"]]
  ll0 <- l[["ll_0"]]
  n <- if (is.null(nobs)) NROW(stats::fitted(fit)) else nobs
  keep <- !grepl("(Intercept)", names(stata_b), fixed = TRUE) & !grepl("^/", names(stata_b)) & !is.na(stata_b)
  q <- sum(keep)
  cat(sprintf("Number of obs   = %10d\n", n))
  if (q > 0 && robust) {
    bk <- stata_b[keep]
    w <- drop(t(bk) %*% solve(stata_vcov[keep, keep, drop = FALSE], bk))
    cat(sprintf("Wald chi2(%d)    = %10.2f\nProb > chi2     = %10.4f\n", q, w, stats::pchisq(w, q, lower.tail = FALSE)))
  } else if (q > 0 && !is.na(ll0)) {
    lr <- 2 * (ll - ll0)
    cat(sprintf("LR chi2(%d)      = %10.2f\nProb > chi2     = %10.4f\n", q, lr, stats::pchisq(lr, q, lower.tail = FALSE)))
  }
  if (!is.na(ll0)) cat(sprintf("Pseudo R2       = %10.4f\n", 1 - ll / ll0))
  cat(sprintf("Log likelihood  = %.4f\n\n", ll))
  invisible(c(N = n, ll = ll, ll_0 = ll0))
}
# Stata's iteration log: its own Newton-Raphson (stata_ml_path) from the constant-only fit,
# each point's likelihood. logit and probit start at the constant; ologit and oprobit at the
# cut points of the outcome's shares, the slopes 0; mlogit at each outcome's log odds against
# the base. NULL when the model has no constant (logit, probit, mlogit) or an offset, or a
# step was halved or -H not concave (marked in Stata's log, not checked)
stata_ml_iterations <- function(fit, kind) {
  if (kind %in% c("ologit", "oprobit", "mlogit")) {
    m <- stata_ml_exact(fit, kind)
    if (is.null(m) || m$odd || !m$converged) return(NULL)
    return(m$lls)
  }
  if (kind == "glm") return(if (isTRUE(fit$stata_odd)) NULL else fit$stata_lls)
  if (kind %in% c("poisson", "nbreg")) return(stata_ml_count(fit, kind)$lls)
  X <- stats::model.matrix(fit)
  if (!"(Intercept)" %in% colnames(X)) return(NULL)
  off <- fit$offset
  if (!is.null(off) && any(off != 0)) return(NULL)
  pw <- fit$prior.weights
  pos <- fit$y != 0
  pbar <- sum(pw * pos) / sum(pw)
  b0 <- rep(0, ncol(X))
  b0[colnames(X) == "(Intercept)"] <- if (kind == "logit") stats::qlogis(pbar) else stats::qnorm(pbar)
  ll <- function(b) {
    z <- drop(X %*% b)
    if (kind == "logit") sum(pw * ifelse(pos, stats::plogis(z, log.p = TRUE), stats::plogis(z, lower.tail = FALSE, log.p = TRUE)))
    else sum(pw * ifelse(pos, stats::pnorm(z, log.p = TRUE), stats::pnorm(z, lower.tail = FALSE, log.p = TRUE)))
  }
  deriv <- function(b) {
    z <- drop(X %*% b)
    if (kind == "logit") {
      p <- stats::plogis(z)
      list(ll = ll(b), g = colSums(X * (pw * (pos - p))), nh = crossprod(X * (pw * p * (1 - p)), X))
    } else {
      lam <- ifelse(pos, exp(stats::dnorm(z, log = TRUE) - stats::pnorm(z, log.p = TRUE)),
                    -exp(stats::dnorm(z, log = TRUE) - stats::pnorm(z, lower.tail = FALSE, log.p = TRUE)))
      list(ll = ll(b), g = colSums(X * (pw * lam)), nh = crossprod(X * (pw * lam * (lam + z)), X))
    }
  }
  path <- tryCatch(stata_ml_path(ll, deriv, b0), error = function(e) NULL)
  if (is.null(path) || path$odd || !path$converged) return(NULL)
  path$lls
}
# poisson's start (found on probes, 2026-11-15, to 1e-16): the slopes of the least-squares
# fit of y on the centred regressors over mean(y), the constant then exactly
# ln(sum y / sum exp(x'b)), and one full Newton step from there; its log is the path from
# that start. nbreg's log has three stages: that Poisson fit; the constant-only model from
# (ln mean(y), lnalpha 0); the full model from Poisson's slopes, the constant
# ln mean(y) - b'mean(x) and the constant-only lnalpha. NULL without a constant, with an
# offset or weights, or where a step was halved. With the path's end (nbreg's lnalpha last)
stata_ml_count <- function(fit, kind) {
  X <- stats::model.matrix(fit)
  ic <- colnames(X) == "(Intercept)"
  if (sum(ic) != 1) return(NULL)
  off <- stats::model.offset(stats::model.frame(fit))
  if (!is.null(off) && any(off != 0)) return(NULL)
  y <- fit$y
  pw <- fit$prior.weights
  if (is.null(pw)) pw <- rep(1, length(y))
  if (any(pw != 1) || length(y) != nrow(X) || !(mean(y) > 0)) return(NULL)
  ybar <- mean(y)
  Xs <- X[, !ic, drop = FALSE]
  pois_ll <- function(b) {
    e <- drop(X %*% b)
    sum(y * e - exp(e) - lgamma(y + 1))
  }
  pois_deriv <- function(b) {
    mu <- exp(drop(X %*% b))
    list(ll = pois_ll(b), g = colSums(X * (y - mu)), nh = crossprod(X * mu, X))
  }
  b <- rep(0, ncol(Xs))
  if (ncol(Xs) > 0) {
    b <- qr.coef(qr(sweep(Xs, 2, colMeans(Xs))), y - ybar) / ybar
    if (anyNA(b)) return(NULL)
  }
  b0 <- numeric(ncol(X))
  b0[!ic] <- b
  b0[ic] <- log(sum(y) / sum(exp(drop(Xs %*% b))))
  d0 <- pois_deriv(b0)
  b0 <- b0 + drop(solve(d0$nh, d0$g))
  pois <- tryCatch(stata_ml_path(pois_ll, pois_deriv, b0), error = function(e) NULL)
  if (is.null(pois) || pois$odd || !pois$converged) return(NULL)
  if (kind == "poisson") return(list(lls = pois$lls, b = pois$b))
  nb <- function(X) {
    k <- ncol(X)
    ll <- function(p) {
      mu <- exp(drop(X %*% p[seq_len(k)]))
      a <- exp(p[k + 1])
      m <- 1 / a
      sum(lgamma(y + m) - lgamma(m) - lgamma(y + 1) - (m + y) * log1p(a * mu) + y * (log(a) + log(mu)))
    }
    deriv <- function(p) {
      mu <- exp(drop(X %*% p[seq_len(k)]))
      a <- exp(p[k + 1])
      m <- 1 / a
      q <- 1 + a * mu
      D <- digamma(y + m) - digamma(m)
      H <- matrix(0, k + 1, k + 1)
      H[seq_len(k), seq_len(k)] <- crossprod(X * (-mu * (1 + a * y) / q^2), X)
      H[seq_len(k), k + 1] <- H[k + 1, seq_len(k)] <- colSums(X * (a * mu * (mu - y) / q^2))
      H[k + 1, k + 1] <- sum(m * D + m^2 * (trigamma(y + m) - trigamma(m)) - m * log(q) + mu / q - (y - mu) * a * mu / q^2)
      list(ll = ll(p), g = c(colSums(X * ((y - mu) / q)), sum(-m * D + m * log(q) + (y - mu) / q)), nh = -H)
    }
    list(ll = ll, deriv = deriv)
  }
  f0 <- nb(X[, ic, drop = FALSE])
  cons <- tryCatch(stata_ml_path(f0$ll, f0$deriv, c(log(ybar), 0)), error = function(e) NULL)
  if (is.null(cons) || cons$odd || !cons$converged) return(NULL)
  bs <- pois$b[!ic]
  start <- numeric(ncol(X) + 1)
  start[which(!ic)] <- bs
  start[which(ic)] <- log(ybar) - sum(bs * colMeans(Xs))
  start[ncol(X) + 1] <- cons$b[2]
  f1 <- nb(X)
  full <- tryCatch(stata_ml_path(f1$ll, f1$deriv, start), error = function(e) NULL)
  if (is.null(full) || full$odd || !full$converged) return(NULL)
  V <- tryCatch(solve(f1$deriv(full$b)$nh), error = function(e) NULL)
  list(lls = list(`Fitting Poisson model:` = pois$lls, `Fitting constant-only model:` = cons$lls,
                  `Fitting full model:` = full$lls), b = full$b, V = V,
       ll_0 = cons$lls[length(cons$lls)], ll_c = pois$lls[length(pois$lls)])
}
# poisson's and nbreg's fit moved to where Stata's path ends: its estimates, likelihood and
# fitted values (the variance, robust or not, is then worked out there)
stata_ml_count_fit <- function(fit, kind) {
  r <- tryCatch(stata_ml_count(fit, kind), error = function(e) NULL)
  if (is.null(r) || anyNA(stats::coef(fit))) return(fit)
  X <- stats::model.matrix(fit)
  k <- ncol(X)
  b <- r$b
  y <- fit$y
  eta <- drop(X %*% b[seq_len(k)])
  mu <- exp(eta)
  fit$coefficients[] <- b[seq_len(k)]
  fit$linear.predictors[] <- eta
  fit$fitted.values[] <- mu
  if (kind == "poisson") {
    fit$deviance <- 2 * sum(ifelse(y > 0, y * log(y / mu), 0) - (y - mu))
    fit$aic <- -2 * sum(stats::dpois(y, mu, log = TRUE)) + 2 * fit$rank
  } else {
    a <- exp(b[k + 1])
    m <- 1 / a
    fit$theta <- m
    # the exact variance (the inverse of -H) and the constant-only and Poisson likelihoods
    # where Stata's paths end, for e(ll_0) and e(ll_c)
    fit$stata_count <- list(V = r$V, ll_0 = r$ll_0, ll_c = r$ll_c)
    fit$twologlik <- 2 * sum(lgamma(y + m) - lgamma(m) - lgamma(y + 1) - (m + y) * log1p(a * mu) + y * (log(a) + log(mu)))
  }
  fit
}
# where Stata's Newton-Raphson stops, each point's likelihood, and the variance there
# (the inverse of -H, exact); NULL when R cannot follow it
stata_ml_end <- function(ll, deriv, b0) {
  path <- tryCatch(stata_ml_path(ll, deriv, b0), error = function(e) NULL)
  if (is.null(path)) return(NULL)
  V <- tryCatch(solve(deriv(path$b)$nh), error = function(e) NULL)
  if (is.null(V)) return(NULL)
  list(lls = path$lls, b = path$b, V = V, odd = path$odd, converged = path$converged)
}
# ologit, oprobit and mlogit as Stata estimates them: its path's end and exact variance, in
# the order of stata_b (polr's slopes then cut points; multinom's equations in turn)
stata_ml_exact <- function(fit, kind) {
  if (kind == "mlogit") stata_ml_mlogit(fit) else stata_ml_ordered(fit, kind == "ologit")
}
# the fit moved to Stata's estimates (its probabilities too, which predict and the scores
# read); stata_b and stata_vcov take them where the path ends converged
stata_ml_exact_fit <- function(fit, kind) {
  m <- tryCatch(stata_ml_exact(fit, kind), error = function(e) NULL)
  if (is.null(m) || !m$converged || length(m$b) != length(stata_b)) return(fit)
  b <- m$b
  if (kind == "mlogit") {
    X <- stats::model.matrix(fit)
    E <- exp(X %*% matrix(b, ncol(X)))
    P <- cbind(1, E) / (1 + rowSums(E))
    dimnames(P) <- list(rownames(X), fit$lev)
    fit$fitted.values <- if (ncol(P) == 2) P[, 2] else P
  } else {
    k <- length(fit$coefficients)
    fit$coefficients[] <- b[seq_len(k)]
    fit$zeta[] <- b[k + seq_along(fit$zeta)]
    mf <- stats::model.frame(fit)
    X <- stats::model.matrix(stats::delete.response(stats::terms(fit)), mf)
    X <- X[, colnames(X) != "(Intercept)", drop = FALSE]
    eta <- if (k > 0) drop(X %*% fit$coefficients) else rep(0, nrow(X))
    cdf <- if (kind == "ologit") stats::plogis else stats::pnorm
    cum <- cbind(0, sapply(fit$zeta, function(z) cdf(z - eta)), 1)
    P <- cum[, -1, drop = FALSE] - cum[, -ncol(cum), drop = FALSE]
    dimnames(P) <- list(rownames(X), fit$lev)
    fit$fitted.values <- P
    fit$lp <- eta
  }
  fit$stata_exact <- list(b = b, V = m$V)
  fit
}
# ologit and oprobit (polr): the likelihood in the slopes and cut points, its exact
# derivatives; each row's probability is F(upper cut - x'b) - F(lower cut - x'b)
stata_ml_ordered <- function(fit, logistic) {
  mf <- stats::model.frame(fit)
  if (!is.null(stats::model.offset(mf))) return(NULL)
  X <- stats::model.matrix(stats::delete.response(stats::terms(fit)), mf)
  X <- X[, colnames(X) != "(Intercept)", drop = FALSE]
  j <- as.integer(stats::model.response(mf))
  w <- stats::model.weights(mf)
  if (is.null(w)) w <- rep(1, length(j))
  J <- length(fit$lev)
  if (J < 2 || any(tabulate(j, J) == 0)) return(NULL)
  k <- ncol(X)
  n <- length(j)
  cdf <- function(z) if (logistic) stats::plogis(z) else stats::pnorm(z)
  pdf <- function(z) {
    r <- if (logistic) stats::dlogis(z) else stats::dnorm(z)
    r[!is.finite(z)] <- 0
    r
  }
  slope <- function(z) {
    r <- if (logistic) stats::dlogis(z) * (1 - 2 * stats::plogis(z)) else -z * stats::dnorm(z)
    r[!is.finite(z)] <- 0
    r
  }
  # each row's upper and lower bound minus x'b, as functions of (b, cuts)
  Du <- cbind(-X, matrix(0, n, J - 1))
  Dl <- Du
  up <- which(j <= J - 1)
  lo <- which(j >= 2)
  Du[cbind(up, k + j[up])] <- 1
  Dl[cbind(lo, k + j[lo] - 1)] <- 1
  bounds <- function(b) {
    eta <- if (k > 0) drop(X %*% b[seq_len(k)]) else rep(0, n)
    cu <- c(-Inf, b[k + seq_len(J - 1)], Inf)
    u <- cu[j + 1] - eta
    l <- cu[j] - eta
    list(u = u, l = l, p = cdf(u) - cdf(l))
  }
  ll <- function(b) {
    q <- bounds(b)
    if (any(!(q$p > 0))) return(NaN)
    sum(w * log(q$p))
  }
  deriv <- function(b) {
    q <- bounds(b)
    G <- Du * (pdf(q$u) / q$p) - Dl * (pdf(q$l) / q$p)
    H <- crossprod(Du * (w * slope(q$u) / q$p), Du) - crossprod(Dl * (w * slope(q$l) / q$p), Dl) - crossprod(G * w, G)
    list(ll = sum(w * log(q$p)), g = colSums(G * w), nh = -H)
  }
  cp <- cumsum(tapply(w, factor(j, levels = seq_len(J)), sum)) / sum(w)
  b0 <- c(rep(0, k), if (logistic) stats::qlogis(cp[-J]) else stats::qnorm(cp[-J]))
  stata_ml_end(ll, deriv, unname(b0))
}
# mlogit (nnet::multinom, the base outcome its first level): each other outcome's
# equation; the path does not depend on which outcome is the base
stata_ml_mlogit <- function(fit) {
  mf <- fit$model
  X <- stats::model.matrix(fit)
  ic <- which(colnames(X) == "(Intercept)")
  if (length(ic) != 1 || !is.null(stats::model.offset(mf))) return(NULL)
  y <- stats::model.response(mf)
  w <- stats::model.weights(mf)
  if (is.null(w)) w <- rep(1, length(y))
  lev <- levels(y)
  if (length(lev) < 2) return(NULL)
  Y <- matrix(vapply(lev[-1], function(l) as.numeric(y == l), numeric(length(y))), nrow = length(y))
  k <- ncol(X)
  m <- ncol(Y)
  ll <- function(b) {
    E <- X %*% matrix(b, k)
    sum(w * (rowSums(Y * E) - log(1 + rowSums(exp(E)))))
  }
  deriv <- function(b) {
    E <- exp(X %*% matrix(b, k))
    P <- E / (1 + rowSums(E))
    H <- matrix(0, k * m, k * m)
    for (a in seq_len(m)) for (c in seq_len(m)) {
      H[(a - 1) * k + seq_len(k), (c - 1) * k + seq_len(k)] <- crossprod(X * (w * P[, a] * ((a == c) - P[, c])), X)
    }
    list(ll = ll(b), g = as.vector(crossprod(X, w * (Y - P))), nh = H)
  }
  pj <- colSums(Y * w) / sum(w)
  pb <- 1 - sum(pj)
  if (any(pj <= 0) || pb <= 0) return(NULL)
  b0 <- as.vector(vapply(seq_len(m), function(a) replace(rep(0, k), ic, log(pj[a] / pb)), numeric(k)))
  stata_ml_end(ll, deriv, b0)
}
# prints the whole table or nothing: a model the layout cannot place (an
# omitted column, a missing variance) falls back to R's layout untouched
stata_ml_print <- function(...) {
  ok <- FALSE
  out <- tryCatch(utils::capture.output(ok <- stata_ml_print_body(...)), error = function(e) NULL)
  if (is.null(out) || !isTRUE(ok)) return(FALSE)
  cat(paste0(out, "\n"), sep = "")
  TRUE
}
# The table in blocks, as Stata 19 lays them out: logit, probit and poisson one block;
# ologit and oprobit the slopes, then the cut points (/cut1, ... with no z or P); nbreg the
# slopes, /lnalpha, then alpha; mlogit each outcome's equation under its value, the base
# outcome marked. Odds ratios, IRR and RRR transform the first block only (each equation of
# mlogit). The header from e(): the model test is e(chi2) on e(df_m), Pseudo R2 e(r2_p)
# (nbreg's against its constant-only fit, with "Dispersion: mean")
stata_ml_print_body <- function(fit, title, robust = FALSE, nobs = NULL, level = 0.95, label = "Coefficient",
                                eform = FALSE, note = NULL, iterations = NULL, data = NULL, glm = NULL) {
  e <- stata_e
  kind <- switch(if (is.null(e$cmd)) "" else e$cmd, ologit = , oprobit = "ordered", nbreg = "nbreg",
                 mlogit = "mlogit", "single")
  full <- colnames(e$b)
  eq <- ifelse(grepl(":", full, fixed = TRUE), sub(":.*$", "", full), "")
  nm <- sub("^[^:]*:", "", full)
  bf <- stats::setNames(as.vector(e$b), full)
  sf <- stats::setNames(sqrt(pmax(diag(as.matrix(e$V)), 0)), full)
  block <- function(i, ...) list(b = stats::setNames(bf[i], nm[i]), se = stats::setNames(sf[i], nm[i]), ...)
  if (kind == "single") {
    if (length(unique(eq[eq != ""])) > 1) return(FALSE)
    blocks <- list(block(seq_along(full)))
  } else if (kind == "mlogit") {
    base <- as.character(e$baseout)
    blocks <- lapply(unique(eq), function(q) block(which(eq == q), head = q, base = identical(q, base)))
  } else {
    if (!any(eq == "/") || !any(eq != "/")) return(FALSE)
    blocks <- list(block(which(eq != "/")), block(which(eq == "/"), aux = TRUE))
    if (kind == "nbreg") {
      if (!identical(nm[eq == "/"], "lnalpha")) return(FALSE)
      blocks[[3]] <- list(alpha = TRUE)
    }
  }
  for (k in seq_along(blocks)) {
    bl <- blocks[[k]]
    if (isTRUE(bl$aux) || isTRUE(bl$base) || isTRUE(bl$alpha)) next
    blocks[[k]]$rows <- stata_coef_rows(names(bl$b), data)
    if (is.null(blocks[[k]]$rows)) return(FALSE)
  }
  n <- if (is.null(nobs)) NROW(stats::fitted(fit)) else nobs
  if (!is.null(glm)) {
    # glm: its own header (stata_glm_header)
  } else if (kind == "single") {
    l <- stata_ml_ll(fit)
    ll <- l[["ll"]]
    b <- blocks[[1]]$b
    se <- blocks[[1]]$se
    keep <- names(b) != "_cons" & !grepl("^[0-9]+[bo]+\\.|^o\\.", names(b)) & se > 0
    q <- sum(keep)
    stat <- if (!robust) 2 * (ll - l[["ll_0"]]) else if (q > 0) {
      drop(t(b[keep]) %*% solve(as.matrix(e$V)[keep, keep, drop = FALSE], b[keep]))
    } else NA_real_
    r2p <- 1 - ll / l[["ll_0"]]
  } else {
    ll <- e$ll
    q <- e$df_m
    stat <- e$chi2
    r2p <- e$r2_p
    if (is.null(ll) || is.null(q) || is.null(stat) || is.null(r2p)) return(FALSE)
  }
  pad <- stata_tab_pad
  # the iteration log, as Stata prints it before the header
  if (!is.null(iterations)) {
    lls <- stata_ml_iterations(fit, iterations)
    # one stage, or nbreg's three, each under its title
    stages <- if (is.null(lls)) list() else if (is.list(lls)) lls else list(lls)
    for (s in seq_along(stages)) {
      if (!is.null(names(stages))) cat("\n", names(stages)[s], "\n", sep = "")
      cat("\n")
      for (i in seq_along(stages[[s]])) {
        cat("Iteration ", i - 1, ":  ", if (robust) "Log pseudolikelihood" else "Log likelihood", " = ",
            stata_fmt(stages[[s]][i], "%10.0g"), "  \n", sep = "")
      }
    }
  }
  if (!is.null(glm)) {
    stata_glm_header(fit, robust)
  } else {
  cat("\n")
  # the values right-aligned after "= " in 6 columns, or as wide as the widest (the left
  # column gives way: probe PN); Prob > chi2 is "." with no test (chi2(0))
  vals <- c(stata_tab_count(n), if (is.na(stat)) "." else sprintf("%.2f", stat),
            if (q == 0 || is.na(stat)) "." else sprintf("%.4f", stats::pchisq(stat, q, lower.tail = FALSE)),
            sprintf("%.4f", r2p))
  wv <- max(6, nchar(vals))
  row <- function(left, label, value) cat(formatC(left, width = -(56 - (wv - 6)), flag = "-"), formatC(label, width = -13, flag = "-"), " = ", pad(value, wv), "\n", sep = "")
  row(title, "Number of obs", vals[1])
  row("", sprintf(if (robust) "Wald chi2(%d)" else "LR chi2(%d)", q), vals[2])
  row(if (kind == "nbreg") "Dispersion: mean" else "", "Prob > chi2", vals[3])
  row(paste0(if (robust) "Log pseudolikelihood = " else "Log likelihood = ", trimws(stata_fmt(ll, "%10.0g"))),
      "Pseudo R2", vals[4])
  cat("\n")
  }
  if (!is.null(e[["N_clust"]])) {
    cat(pad(sprintf("(Std. err. adjusted for %s clusters in %s)", stata_tab_count(e$N_clust), e$clustvar), 78), "\n", sep = "")
  }
  qz <- stats::qnorm((1 + level) / 2)
  sep_line <- function() cat(strrep("-", 13), "+", strrep("-", 64), "\n", sep = "")
  cat(strrep("-", 78), "\n", sep = "")
  if (robust) cat("             |               Robust\n") else if (!is.null(glm)) cat("             |                 OIM\n")
  cat(pad(stata_tab_abbrev(e$depvar), 12), " | ", formatC(formatC(label, width = 10), width = -13, flag = "-"),
      if (robust || !is.null(glm)) "std. err." else "Std. err.", "      z    P>|z|     [", format(100 * level), "% conf. interval]\n", sep = "")
  sep_line()
  # a row with no test (a cut point, /lnalpha, alpha): the estimate, its error and interval
  aux_row <- function(lab, shown) {
    cat(pad(lab, 12), " |", pad(stata_tab_g(shown[1]), 11), pad(stata_tab_g(shown[2]), 11), strrep(" ", 17),
        pad(stata_tab_g(shown[3]), 13), pad(stata_tab_g(shown[4]), 12), "\n", sep = "")
  }
  # an outcome's value, or its value label as Stata names the equation (spaces as _)
  outcome <- function(v) {
    s <- attr((if (is.null(data)) stata_data else data)[[e$depvar]], "labels")
    i <- if (is.null(s)) NA else match(suppressWarnings(as.numeric(v)), as.numeric(s))
    if (is.na(i)) v else gsub(" ", "_", names(s)[i], fixed = TRUE)
  }
  for (k in seq_along(blocks)) {
    bl <- blocks[[k]]
    if (k > 1) sep_line()
    if (!is.null(bl$head)) {
      cat(formatC(stata_tab_abbrev(outcome(bl$head)), width = -13, flag = "-"), "|",
          if (isTRUE(bl$base)) "  (base outcome)", "\n", sep = "")
      if (isTRUE(bl$base)) next
    }
    if (isTRUE(bl$alpha)) {
      lna <- bf[["/:lnalpha"]]
      s <- sf[["/:lnalpha"]]
      aux_row("alpha", c(exp(lna), exp(lna) * s, exp(lna - qz * s), exp(lna + qz * s)))
      next
    }
    if (isTRUE(bl$aux)) {
      for (i in seq_along(bl$b)) {
        aux_row(paste0("/", names(bl$b)[i]), c(bl$b[[i]], bl$se[[i]], bl$b[[i]] - qz * bl$se[[i]], bl$b[[i]] + qz * bl$se[[i]]))
      }
      next
    }
    for (r in bl$rows) {
      if (r$kind == "blank") {
        cat(strrep(" ", 13), "|\n", sep = "")
        next
      }
      if (r$kind == "head") {
        cat(pad(stata_tab_abbrev(r$label), 12), " |\n", sep = "")
        next
      }
      if (r$kind == "wrap") {
        cat(pad(r$label, 11), " #|\n", sep = "")
        next
      }
      lab <- if (r$kind == "level") paste0(pad(r$label, 11), " ") else pad(stata_tab_abbrev(r$label), 12)
      if (!is.null(r$note)) {
        cat(lab, " |", pad(if (eform) "1" else "0", 11), "  ", r$note, "\n", sep = "")
        next
      }
      bi <- bl$b[[r$col]]
      si <- bl$se[[r$col]]
      z <- bi / si
      p <- 2 * stats::pnorm(-abs(z))
      lo <- bi - qz * si
      hi <- bi + qz * si
      shown <- if (eform) c(exp(bi), exp(bi) * si, exp(lo), exp(hi)) else c(bi, si, lo, hi)
      cat(lab, " |", pad(stata_tab_g(shown[1]), 11), pad(stata_tab_g(shown[2]), 11), pad(sprintf("%.2f", z), 9),
          pad(sprintf("%.3f", p), 8), pad(stata_tab_g(shown[3]), 13), pad(stata_tab_g(shown[4]), 12), "\n", sep = "")
    }
  }
  # glm's offset or exposure: a row of its own, as Stata prints it
  off <- if (!is.null(glm)) fit$stata$offrow else NULL
  if (!is.null(off)) cat(pad(off[1], 12), " |", pad("1", 11), "  (", off[2], ")\n", sep = "")
  cat(strrep("-", 78), "\n", sep = "")
  if (eform && kind == "ordered") cat("Note: Estimates are transformed only in the first equation to odds ratios.\n")
  if (eform && kind == "nbreg") cat("Note: Estimates are transformed only in the first equation to incidence-rate\n      ratios.\n")
  if (!is.null(note) && "_cons" %in% nm) cat(note, "\n", sep = "")
  # glm's note on scaled standard errors
  if (is.character(glm)) cat(glm, "\n", sep = "")
  # nbreg: the test of alpha = 0 against the Poisson fit, on the boundary (half chi2(1))
  if (kind == "nbreg" && !robust && !is.null(e$ll_c)) {
    chi <- max(0, 2 * (e$ll - e$ll_c))
    # %8.2f, which Stata writes as 5.3e+05 when it does not fit (probe PN)
    cat(formatC(paste0("LR test of alpha=0: chibar2(01) = ", trimws(stata_fmt(chi, "%8.2f"))), width = -55, flag = "-"),
        "Prob >= chibar2 = ", sprintf("%5.3f", 0.5 * stats::pchisq(chi, 1, lower.tail = FALSE)), "\n", sep = "")
  }
  TRUE
}"#;

impl Session {
    /// ologit, oprobit and mlogit at Stata's estimates: its Newton-Raphson's end from the
    /// constant-only fit and the exact variance there (R's optimisers stop a hair away, and
    /// polr's variance is a numerical Hessian's), where the path ends converged.
    fn exact_ml(&mut self, fit: Fit, out: &mut Out) {
        self.helpers.insert("stata_ml_header");
        self.helpers.insert("stata_probit_newton");
        out.line(format!(
            "stata_estimates <- stata_ml_exact_fit(stata_estimates, {})",
            r_text(fit.name())
        ));
        out.line("if (!is.null(stata_estimates$stata_exact)) { stata_b[] <- stata_estimates$stata_exact$b; stata_vcov[] <- stata_estimates$stata_exact$V }");
    }

    /// poisson and nbreg: the fit moved to where Stata's own path ends
    /// (`stata_ml_count_fit`), before the estimates and variance are read.
    fn count_ml(&mut self, fit: Fit, out: &mut Out) {
        self.helpers.insert("stata_ml_header");
        self.helpers.insert("stata_probit_newton");
        out.line(format!(
            "stata_estimates <- stata_ml_count_fit(stata_estimates, {})",
            r_text(fit.name())
        ));
    }

    pub(super) fn ml(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
        fit: Fit,
    ) -> Result<(), Untranslated> {
        let name = fit.name();
        self.no_by(c, name)?;
        let mut known: Vec<(&str, usize)> = vec![("level", 3), ("nolog", 5)];
        match fit {
            Fit::Logit => known.extend([
                ("noconstant", 5),
                ("or", 2),
                ("robust", 1),
                ("vce", 3),
                ("offset", 6),
            ]),
            Fit::Probit => {
                known.extend([("noconstant", 5), ("robust", 1), ("vce", 3), ("offset", 6)])
            }
            Fit::Poisson => known.extend([
                ("noconstant", 5),
                ("irr", 3),
                ("exposure", 3),
                ("offset", 6),
                ("robust", 1),
                ("vce", 3),
            ]),
            Fit::Nbreg => known.extend([
                ("noconstant", 5),
                ("irr", 3),
                ("exposure", 3),
                ("offset", 6),
                ("robust", 1),
                ("vce", 3),
            ]),
            Fit::Ologit => known.extend([("or", 2), ("robust", 1), ("vce", 3)]),
            Fit::Oprobit => known.extend([("robust", 1), ("vce", 3)]),
            Fit::Mlogit => known.extend([
                ("noconstant", 5),
                ("rrr", 3),
                ("baseoutcome", 4),
                ("robust", 1),
                ("vce", 3),
            ]),
            Fit::Regress => return self.regress(c, opts, out),
            Fit::Xtreg => return self.xtreg(c, opts, out),
            Fit::Ivregress => return self.ivregress(c, opts, out),
            Fit::Margins => return Err(no("margins is not a model to fit")),
            Fit::Suest => return Err(no("suest is not a model to fit")),
            Fit::Stcox => return self.stcox(c, opts, out),
            Fit::Clogit => return self.clogit(c, opts, out),
            Fit::Robreg => return self.robreg(c, opts, out),
            Fit::Glm => return self.glm(c, opts, out),
            Fit::Cnsreg
            | Fit::Sureg
            | Fit::Mvreg
            | Fit::Areg
            | Fit::Qreg
            | Fit::Tobit
            | Fit::Intreg
            | Fit::Truncreg
            | Fit::Cloglog
            | Fit::Hetprobit
            | Fit::Heckman => {
                return Err(no("this model is not fitted here"))
            }
            Fit::Reghdfe => return self.reghdfe(c, opts, out),
            Fit::Cluster2 => return self.cluster2(c, opts, out),
            Fit::Asclogit => return self.asclogit(c, opts, out),
        }
        if let Some(o) = opts.other(&known) {
            return Err(no(format!(
                "{name} with the option {o} is not translated yet"
            )));
        }
        let spec = Spec::read(c, name)?;
        self.helpers_for(&spec);
        let level = level(opts)?;
        let (robust, cluster) = vce(opts)?;
        let mut right = spec.right(opts.has("noconstant", 5))?;
        // exposure() and offset(): Stata's errors as Stata gives them (probe 510);
        // the checks on the data run once the rows are known, below
        let stop = |out: &mut Out, msg: &str| {
            out.line(format!("stop({}, call. = FALSE)", r_text(msg)));
        };
        let mut offset_var: Option<(String, bool)> = None;
        match (opts.arg("exposure", 3), opts.arg("offset", 6)) {
            (Some(_), Some(_)) => {
                stop(
                    out,
                    "only one of offset() or exposure() can be specified (Stata: r(198))",
                );
                return Ok(());
            }
            (Some(v), None) | (None, Some(v)) => {
                let exposure = opts.arg("exposure", 3).is_some();
                let opt = if exposure { "exposure" } else { "offset" };
                let words: Vec<&str> = v.split_whitespace().collect();
                if words.len() > 1 {
                    stop(
                        out,
                        &format!("{opt}():  too many variables specified (Stata: r(103))"),
                    );
                    return Ok(());
                }
                let e = v.trim();
                if !is_name(e) {
                    return Err(no(format!("{opt}({e}) needs one variable name")));
                }
                if exposure {
                    right.push_str(&format!(" + offset(log({}))", r_name(e)));
                } else {
                    right.push_str(&format!(" + offset({})", r_name(e)));
                }
                offset_var = Some((e.to_string(), exposure));
            }
            (None, None) => {}
        }
        let w = match fit {
            Fit::Logit
            | Fit::Probit
            | Fit::Poisson
            | Fit::Nbreg
            | Fit::Ologit
            | Fit::Oprobit
            | Fit::Mlogit => self.weight_of(c, &[W::F, W::P, W::I])?,
            _ => None,
        };
        if matches!(w, Some((W::I, _))) && (robust || cluster.is_some()) {
            return Err(no(
                "robust or clustered errors with iweights are not translated yet",
            ));
        }
        // sampling weights mean robust errors
        let robust = robust || matches!(w, Some((W::P, _)));
        let rows = self.rows_for(c, out)?;
        let src = self.weighted_rows(rows, &w);
        if let Some((v, exposure)) = &offset_var {
            let opt = if *exposure { "exposure" } else { "offset" };
            let positive = if *exposure {
                format!(
                    "\n  ok <- stats::complete.cases(d[intersect(all.vars(stats::as.formula({})), names(d))])\n  if (any(x[ok] <= 0, na.rm = TRUE)) stop(\"exposure() must be greater than zero (Stata: r(459))\", call. = FALSE)",
                    r_text(&format!("{} ~ {right}", spec.dep))
                )
            } else {
                String::new()
            };
            out.line(format!(
                "local({{\n  d <- {src}\n  x <- d[[{q}]]\n  if (is.null(x)) stop(\"variable {v} not found\\n(error in option {opt}()) (Stata: r(111))\", call. = FALSE)\n  if (is.character(x)) stop(\"string variables not allowed in option {opt}();\\n{v} is a string variable (Stata: r(109))\", call. = FALSE){positive}\n}})",
                q = r_text(v)
            ));
        }
        let weights = match &w {
            Some((_, v)) => format!(", weights = {}", r_name(v)),
            None => String::new(),
        };
        let kind = match &w {
            Some((k, _)) => format!(", kind = {}", r_text(k.code())),
            None => String::new(),
        };
        if w.is_some() {
            out.caveat(super::weights::weight_caveat(&w));
        }
        let dep = r_name(&spec.dep);
        let control = "control = stats::glm.control(epsilon = 1e-12, maxit = 100)";
        match fit {
            Fit::Logit | Fit::Probit => {
                let link = if fit == Fit::Logit { "logit" } else { "probit" };
                let formula = self.model_formula(&format!("as.numeric({dep} != 0)"), &right);
                // the first fit only finds the rows and terms; stata_perfect
                // applies Stata's check for perfect prediction and fits again
                out.line(format!(
                    "stata_estimates <- suppressWarnings(stats::glm({formula}, family = stats::binomial(link = \"{link}\"), data = {src}{weights}, {control}))"
                ));
                self.helpers.insert("stata_perfect");
                out.line(format!(
                    "stata_estimates <- stata_perfect(stata_estimates{})",
                    if self.quiet(c) { ", notes = FALSE" } else { "" }
                ));
                self.helpers.insert("stata_collinear");
                // the rows a perfect predictor excluded, kept through the refits
                out.line("stata_excluded <- stata_estimates$stata_excluded");
                out.line("stata_estimates <- stata_collinear(stata_estimates)");
                if fit == Fit::Probit {
                    self.helpers.insert("stata_probit_newton");
                    out.line("stata_estimates <- stata_probit_newton(stata_estimates)");
                }
                out.line("stata_estimates$stata_excluded <- stata_excluded");
                out.line("stata_estimates$omitted <- unique(c(stata_estimates$omitted, attr(stata_excluded, \"omitted\")))");
                out.line("if (length(attr(stata_excluded, \"omitted\"))) stata_estimates$stata_written <- attr(stata_excluded, \"written\")");
                out.line("rm(stata_excluded)");
                out.caveat(
                    "the outcome is 1 where it is not 0 (and not missing), as Stata takes it",
                );
                out.caveat(
                    "a variable that predicts the outcome perfectly is dealt with as Stata does: \
                     omitted with the rows it decides, or the fit stops with Stata's message",
                );
            }
            Fit::Poisson => {
                let formula = self.model_formula(&dep, &right);
                out.line(format!(
                    "stata_estimates <- stats::glm({formula}, family = stats::poisson(), data = {src}{weights}, {control})"
                ));
                self.helpers.insert("stata_collinear");
                out.line("stata_estimates <- stata_collinear(stata_estimates)");
            }
            Fit::Nbreg => {
                out.package("MASS");
                self.helpers.insert("stata_collinear");
                out.line(collinear_first(&dep, &right, &src));
                out.line(format!(
                    "stata_estimates <- tryCatch(MASS::glm.nb({}, data = {src}{weights}, {control}), error = function(e) stop(\"nbreg: R's glm.nb could not estimate alpha; when the counts show no overdispersion Stata's alpha goes to 0, which is not reproduced (\", conditionMessage(e), \")\", call. = FALSE))",
                    with_rhs(&dep)
                ));
                // glm.nb may also stop at a theta that only means alpha is 0;
                // Stata's numbers there come from its own path to the boundary
                out.line("if (1 / stata_estimates$theta < 1e-6) stop(\"nbreg: alpha goes to 0 here (the counts show no overdispersion), where Stata's estimates come from its own path to the boundary, which is not reproduced\", call. = FALSE)");
                out.line(KEEP_OMITTED);
            }
            Fit::Ologit | Fit::Oprobit => {
                out.package("MASS");
                let method = if fit == Fit::Ologit {
                    "logistic"
                } else {
                    "probit"
                };
                self.helpers.insert("stata_collinear");
                out.line(collinear_first(&dep, &right, &src));
                out.line(format!(
                    "stata_estimates <- MASS::polr({}, data = {src}{weights}, method = \"{method}\", Hess = TRUE, model = TRUE, control = list(reltol = 1e-14, maxit = 1000))",
                    with_rhs(&format!("factor({dep})"))
                ));
                out.line(KEEP_OMITTED);
                out.line("stata_levels <- stata_estimates$lev");
            }
            Fit::Mlogit => {
                out.package("nnet");
                self.helpers.insert("stata_collinear");
                out.line(collinear_first(&dep, &right, &src));
                let fit_with = |response: &str| {
                    format!(
                        "stata_estimates <- nnet::multinom({}, data = {src}{weights}, trace = FALSE, model = TRUE, maxit = 1000, reltol = 1e-14)\n{KEEP_OMITTED}",
                        with_rhs(response)
                    )
                };
                out.line(fit_with(&format!("factor({dep})")));
                out.line("stata_levels <- stata_estimates$lev");
                match opts.arg("baseoutcome", 4) {
                    Some(b) => {
                        let b = b.trim();
                        if b.parse::<f64>().is_err() {
                            return Err(no(format!(
                                "baseoutcome({b}) needs a value of {}",
                                spec.dep
                            )));
                        }
                        out.line(format!("stata_base <- {}", r_text(b)));
                    }
                    None => {
                        out.line("stata_base <- local({ w <- stats::model.weights(stata_estimates$model); y <- stata_estimates$model[[1]]; names(which.max(if (is.null(w)) table(y) else tapply(w, y, sum))) })");
                        out.caveat("the base outcome is the most frequent one, as in Stata");
                    }
                }
                out.line(fit_with(&format!(
                    "stats::relevel(factor({dep}), ref = stata_base)"
                )));
            }
            Fit::Regress
            | Fit::Xtreg
            | Fit::Ivregress
            | Fit::Margins
            | Fit::Stcox
            | Fit::Clogit
            | Fit::Robreg
            | Fit::Glm
            | Fit::Cnsreg
            | Fit::Sureg
            | Fit::Mvreg
            | Fit::Areg
            | Fit::Qreg
            | Fit::Tobit
            | Fit::Intreg
            | Fit::Truncreg
            | Fit::Cloglog
            | Fit::Hetprobit
            | Fit::Heckman
            | Fit::Reghdfe
            | Fit::Cluster2
            | Fit::Asclogit
            | Fit::Suest => {}
        }
        match fit {
            Fit::Logit | Fit::Probit | Fit::Poisson => {
                self.helpers.insert("stata_glm_vcov");
                self.fit_robust = (robust || cluster.is_some()) && w.is_none();
                self.fit_cluster = cluster.is_some() && w.is_none();
                if fit == Fit::Poisson {
                    self.count_ml(fit, out);
                }
                out.line("stata_b <- stats::coef(stata_estimates)");
                match &cluster {
                    Some(v) => {
                        out.line(format!("stata_cluster <- {}", cluster_rows(&src, v)));
                        out.line(no_missing_cluster());
                        out.line(
                            format!("stata_vcov <- stata_glm_vcov(stata_estimates, robust = TRUE, cluster = stata_cluster{kind})"),
                        );
                        out.caveat("clustered standard errors carry G / (G - 1), as Stata's do for this model");
                    }
                    None if robust => {
                        out.line(format!(
                            "stata_vcov <- stata_glm_vcov(stata_estimates, robust = TRUE{kind})"
                        ));
                        out.caveat(
                            "robust standard errors carry N / (N - 1), as Stata's do for a \
                             maximum-likelihood model (not regress's HC1)",
                        );
                    }
                    None => out.line(format!(
                        "stata_vcov <- stata_glm_vcov(stata_estimates{kind})"
                    )),
                }
                if fit == Fit::Probit {
                    out.caveat(
                        "standard errors from the observed information, as Stata's; R's own \
                         summary() uses the expected information, which differs for probit",
                    );
                }
            }
            Fit::Nbreg => {
                self.helpers.insert("stata_nbreg_vcov");
                self.count_ml(fit, out);
                out.line(
                    "stata_b <- c(stats::coef(stata_estimates), \"/lnalpha\" = log(1 / stata_estimates$theta))",
                );
                out.line("stata_vcov <- stata_nbreg_vcov(stata_estimates)");
                out.caveat(
                    "alpha is 1 / theta of MASS::glm.nb; the standard errors come from the whole \
                     likelihood, alpha included, as Stata's do",
                );
            }
            Fit::Ologit | Fit::Oprobit => {
                out.line(
                    "stata_b <- c(stats::coef(stata_estimates), stats::setNames(stata_estimates$zeta, paste0(\"/cut\", seq_along(stata_estimates$zeta))))",
                );
                out.line("stata_vcov <- stats::vcov(stata_estimates)");
                out.line("dimnames(stata_vcov) <- list(names(stata_b), names(stata_b))");
                self.exact_ml(fit, out);
                out.caveat("the cut points are R's zeta, which are Stata's /cut1, /cut2, ... with the same sign");
            }
            Fit::Mlogit => {
                self.helpers.insert("stata_multinom_b");
                out.line("stata_b <- stata_multinom_b(stata_estimates)");
                out.line("stata_vcov <- stats::vcov(stata_estimates)");
                out.line("dimnames(stata_vcov) <- list(names(stata_b), names(stata_b))");
                self.exact_ml(fit, out);
                out.caveat("each coefficient is named outcome:variable; the base outcome has none");
            }
            Fit::Regress
            | Fit::Xtreg
            | Fit::Ivregress
            | Fit::Margins
            | Fit::Stcox
            | Fit::Clogit
            | Fit::Robreg
            | Fit::Glm
            | Fit::Cnsreg
            | Fit::Sureg
            | Fit::Mvreg
            | Fit::Areg
            | Fit::Qreg
            | Fit::Tobit
            | Fit::Intreg
            | Fit::Truncreg
            | Fit::Cloglog
            | Fit::Hetprobit
            | Fit::Heckman
            | Fit::Reghdfe
            | Fit::Cluster2
            | Fit::Asclogit
            | Fit::Suest => {}
        }
        if matches!(fit, Fit::Nbreg | Fit::Ologit | Fit::Oprobit | Fit::Mlogit)
            && (robust || cluster.is_some())
        {
            // the sandwich of each row's score around the model's own variance
            self.helpers.insert("stata_ml_sandwich");
            let cl = match &cluster {
                Some(v) => {
                    out.line(format!("stata_cluster <- {}", cluster_rows(&src, v)));
                    out.line(no_missing_cluster());
                    out.caveat(
                        "clustered standard errors carry G / (G - 1), as Stata's do for this model",
                    );
                    ", cluster = stata_cluster"
                }
                None => {
                    out.caveat(
                        "robust standard errors carry N / (N - 1), as Stata's do for a \
                         maximum-likelihood model (not regress's HC1)",
                    );
                    ""
                }
            };
            out.line(format!(
                "stata_vcov <- stata_ml_sandwich(stata_estimates, stata_vcov{cl}{kind})"
            ));
        }
        if fit.one_outcome() {
            out.line(self.keep_sample(&src));
        }
        let d = self.data()?;
        out.package("dplyr");
        out.line(self.esample_line(&d, &src));
        self.esample_new = true;
        out.line("stata_df <- Inf");
        self.estimates = Some(Estimates {
            fit,
            dep: spec.dep.clone(),
            factors: spec.factors.clone(),
        });
        // e(), as Stata leaves it
        let vce = if cluster.is_some() {
            "cluster"
        } else if robust {
            "robust"
        } else {
            "oim"
        };
        let mut args = super::returned::e_args(&spec.dep, vce, cluster.as_deref(), &w);
        // e(offset): ln(t) for exposure(t), the variable for offset()
        if let Some((v, exposure)) = &offset_var {
            let shown = if *exposure {
                format!("ln({v})")
            } else {
                v.clone()
            };
            args.push_str(&format!(", extra = list(offset = {})", r_text(&shown)));
        }
        self.keep_e(c, out, &args, "");
        if self.quiet(c) {
            return Ok(());
        }
        self.helpers.insert("stata_ml_header");
        self.helpers.insert("stata_coefs");
        if matches!(
            fit,
            Fit::Logit
                | Fit::Probit
                | Fit::Poisson
                | Fit::Nbreg
                | Fit::Ologit
                | Fit::Oprobit
                | Fit::Mlogit
        ) {
            out.line("stata_omitted_notes()");
        }
        let nobs = if matches!(w, Some((W::F, _))) {
            ", nobs = sum(stats::model.weights(stats::model.frame(stata_estimates)))"
        } else {
            ""
        };
        // Stata 19's layout (not with an offset, whose row was not checked), else R's
        let stata_title = match fit {
            Fit::Logit => Some("Logistic regression"),
            Fit::Probit => Some("Probit regression"),
            Fit::Poisson => Some("Poisson regression"),
            Fit::Nbreg => Some("Negative binomial regression"),
            Fit::Ologit => Some("Ordered logistic regression"),
            Fit::Oprobit => Some("Ordered probit regression"),
            Fit::Mlogit => Some("Multinomial logistic regression"),
            _ => None,
        }
        .filter(|_| offset_var.is_none());
        let stata_open = if let Some(title) = stata_title {
            let (label, eform, note) = match fit {
                Fit::Logit if opts.has("or", 2) => {
                    ("Odds ratio", true, "Note: _cons estimates baseline odds.")
                }
                Fit::Poisson | Fit::Nbreg if opts.has("irr", 3) => (
                    "IRR",
                    true,
                    "Note: _cons estimates baseline incidence rate.",
                ),
                Fit::Ologit if opts.has("or", 2) => ("Odds ratio", true, ""),
                Fit::Mlogit if opts.has("rrr", 3) => (
                    "RRR",
                    true,
                    "Note: _cons estimates baseline relative risk for each outcome.",
                ),
                _ => ("Coefficient", false, ""),
            };
            self.helpers.insert("stata_tables");
            self.helpers.insert("stata_fmt");
            // the iteration log: Stata's own Newton steps from its starting values
            // (stata_ml_iterations, line for line on probes); nbreg's in three stages
            let iterations = match fit {
                // stepwise runs its final model without the log
                _ if opts.has("nolog", 5) || self.sw.as_ref().is_some_and(|s| s.mark) => "NULL",
                Fit::Logit => "\"logit\"",
                Fit::Probit => "\"probit\"",
                Fit::Ologit => "\"ologit\"",
                Fit::Oprobit => "\"oprobit\"",
                Fit::Mlogit => "\"mlogit\"",
                Fit::Poisson => "\"poisson\"",
                Fit::Nbreg => "\"nbreg\"",
                _ => "NULL",
            };
            if iterations != "NULL" {
                self.helpers.insert("stata_probit_newton");
            }
            // the model's rows, for value labels: a one-outcome fit keeps them as stata_data
            let data = if fit.one_outcome() {
                String::new()
            } else {
                self.helpers.insert("stata_slice");
                self.helpers.insert("stata_fit_rows");
                format!(", data = stata_slice({src}, stata_fit_rows(stata_estimates, {src}))")
            };
            out.line(format!(
                "if (!stata_ml_print(stata_estimates, {}, robust = {}{nobs}, level = {}, label = {}, eform = {}, note = {}, iterations = {iterations}{data})) {{",
                r_text(title),
                r_logical(robust),
                r_number(level),
                r_text(label),
                r_logical(eform),
                if note.is_empty() {
                    "NULL".to_string()
                } else {
                    r_text(note)
                }
            ));
            true
        } else {
            false
        };
        out.line(format!(
            "stata_ml_header(stata_estimates, robust = {}{nobs})",
            r_logical(robust)
        ));
        let (eform, label) = match fit {
            Fit::Logit if opts.has("or", 2) => ("TRUE", "Odds ratio"),
            Fit::Ologit if opts.has("or", 2) => (
                "names(stata_b) %in% names(stats::coef(stata_estimates))",
                "Odds ratio",
            ),
            Fit::Poisson if opts.has("irr", 3) => ("TRUE", "IRR"),
            Fit::Nbreg if opts.has("irr", 3) => ("names(stata_b) != \"/lnalpha\"", "IRR"),
            Fit::Mlogit if opts.has("rrr", 3) => ("TRUE", "RRR"),
            _ => ("FALSE", "Coef."),
        };
        out.line(format!(
            "stata_coefs(stata_b, stata_vcov, stata_df, level = {}, eform = {eform}, label = {})",
            r_number(level),
            r_text(label)
        ));
        if fit == Fit::Nbreg {
            out.line(format!(
                "stata_coefs(c(alpha = stata_b[[\"/lnalpha\"]]), stata_vcov[\"/lnalpha\", \"/lnalpha\", drop = FALSE], level = {}, eform = TRUE, label = \"alpha\")",
                r_number(level)
            ));
            if !robust && cluster.is_none() {
                // as Stata: not shown with robust or clustered errors
                self.helpers.insert("stata_nbreg_lr");
                out.line("stata_nbreg_lr(stata_estimates)");
            }
        }
        if fit == Fit::Mlogit {
            out.line("cat(\"base outcome:\", stata_base, \"\\n\")");
        }
        out.caveat("a row with a missing value anywhere in the model is dropped, as in Stata");
        if stata_open {
            out.line("cat(\"(printed in R's layout: Stata's table for this model is not matched yet)\\n\")");
            out.line("}");
            out.caveat("the header and table as Stata 19 prints them, with its iteration log");
        } else {
            out.caveat("printed in R's layout, not Stata's table");
        }
        Ok(())
    }
}

/// Stata leaves out a row whose cluster is missing; rather than fit the rows
/// differently, the R stops and says so.
pub(super) fn no_missing_cluster() -> &'static str {
    "if (anyNA(stata_cluster)) stop(\"the cluster variable is missing in rows the model uses; Stata would leave those rows out\")"
}

/// Stata's check for perfect prediction before `logit` and `probit`, decoded
/// in Stata 19 (see the P2B.4 notes). Each regressor in turn, on the rows still
/// left (a factor variable as one indicator per level, the base included, in
/// the order of the values), is tested for a cut c:
///
/// - the outcome is 1 below some value and 0 above it, or the other way round,
///   with no value in common: Stata stops, `outcome = x <= 4 predicts data
///   perfectly` (r(2000));
/// - the outcome is determined on both sides of c and mixed at c: the
///   variable is omitted and only the rows at c are kept (`... except for
///   x == 4 subsample`);
/// - determined on one side of c only: the same, worded `x != c predicts
///   success|failure perfectly`.
///
/// A factor left with one level, and the interactions it is in, are then
/// omitted as Stata omits them for collinearity. The stop is an R error of
/// class `stata_r2000`, which a `by` loop passes over as Stata's `by` does.
pub(super) const PERFECT_HELPER: &str = r#"stata_perfect <- function(fit, notes = TRUE) {
  env <- parent.frame()
  r2000 <- function(m) stop(structure(class = c("stata_r2000", "error", "condition"),
                                      list(message = m, call = NULL)))
  say <- function(...) if (notes) cat(..., "\n", sep = "")
  num <- function(v) format(v, digits = 15)
  mf <- stats::model.frame(fit)
  y <- stats::model.response(mf)
  if (length(unique(y)) < 2) {
    r2000("outcome does not vary; remember:\n                                  0 = negative outcome,\n        all other nonmissing values = positive outcome")
  }
  keep <- rep(TRUE, nrow(mf))
  labs <- attr(stats::terms(fit), "term.labels")
  gone <- character(0)
  # the rows each omitted variable excluded (x != cut), for predict
  rules <- list()
  for (t in labs) {
    v <- mf[[t]]
    if (is.null(v)) next
    if (is.factor(v)) {
      name <- all.vars(stats::as.formula(paste("~", t)))[1]
      lv <- levels(v)
      o <- suppressWarnings(order(as.numeric(lv), lv))
      cols <- lapply(lv[o], function(l) list(name = paste0(l, ".", name), x = as.numeric(v == l), level = l))
    } else if (is.numeric(v) && is.null(dim(v))) {
      cols <- list(list(name = t, x = as.numeric(v)))
    } else {
      next
    }
    for (cl in cols) {
      x <- cl$x[keep]
      yy <- y[keep]
      if (length(unique(x)) < 2) next
      lo1 <- min(x[yy == 1]); hi1 <- max(x[yy == 1])
      lo0 <- min(x[yy == 0]); hi0 <- max(x[yy == 0])
      n <- cl$name
      if (hi1 < lo0) r2000(paste0("outcome = ", n, " <= ", num(hi1), " predicts data perfectly"))
      if (hi0 < lo1) r2000(paste0("outcome = ", n, " > ", num(hi0), " predicts data perfectly"))
      if (hi1 == lo0) {
        cut <- hi1; below <- "success"; above <- "failure"; sign <- " < "
      } else if (hi0 == lo1) {
        cut <- hi0; below <- "failure"; above <- "success"; sign <- " > "
      } else {
        next
      }
      nb <- sum(x < cut)
      na <- sum(x > cut)
      if (nb > 0 && na > 0) {
        say("note: outcome = ", n, sign, num(cut), " predicts data perfectly except for\n      ",
            n, " == ", num(cut), " subsample:\n      ", n, " omitted and ", nb + na, " obs not used.")
      } else {
        say("note: ", n, " != ", num(cut), " predicts ", if (nb > 0) below else above,
            " perfectly;\n      ", n, " omitted and ", nb + na, " obs not used.")
      }
      keep[which(keep)[x != cut]] <- FALSE
      rules[[length(rules) + 1]] <- list(term = t, level = cl$level, cut = cut)
      if (!is.factor(v)) gone <- c(gone, t)
    }
  }
  # the first fit was only for its rows and terms: fit again where the warnings show
  refit <- function(call) withCallingHandlers(eval(call, env), warning = function(w) {
    # weights that are not whole numbers make binomial() warn; Stata takes them
    if (grepl("non-integer", conditionMessage(w))) invokeRestart("muffleWarning")
  })
  if (all(keep) && length(gone) == 0) return(refit(fit$call))
  # a factor left with one level goes, with every term it is in
  for (t in labs) {
    v <- mf[[t]]
    if (is.factor(v) && length(unique(v[keep])) < 2) {
      fac <- all.vars(stats::as.formula(paste("~", t)))[1]
      for (l in levels(v)[-1]) say("note: ", l, ".", fac, " omitted because of collinearity.")
      gone <- c(gone, labs[vapply(labs, function(l) fac %in% all.vars(stats::as.formula(paste("~", l))), TRUE)])
    }
  }
  gone <- unique(gone)
  tt <- stats::terms(fit)
  f <- if (length(gone) == length(labs)) {
    stats::update(stats::formula(fit), . ~ 1)
  } else if (length(gone) > 0) {
    stats::formula(stats::drop.terms(tt, dropx = which(labs %in% gone), keep.response = TRUE))
  } else {
    stats::formula(fit)
  }
  d <- as.data.frame(fit$data)
  d <- d[match(rownames(mf)[keep], rownames(d)), , drop = FALSE]
  call <- stats::update(fit, formula. = f, evaluate = FALSE)
  call$data <- d
  out <- refit(call)
  out$stata_excluded <- rules
  # e(b) keeps an omitted variable as a column of 0 (o.x), as Stata does
  attr(out$stata_excluded, "omitted") <- setdiff(gone, character(0))
  attr(out$stata_excluded, "written") <- unname(vapply(labs, function(t) paste(sort(trimws(strsplit(t, ":", fixed = TRUE)[[1]])), collapse = ":"), ""))
  out
}
# predict's default after a perfect predictor was omitted: missing on the rows
# it excluded (x != cut), as Stata's "missing for excluded observations"
stata_excluded_na <- function(fit, data, v) {
  for (r in fit$stata_excluded) {
    x <- tryCatch(eval(str2lang(r$term), data, globalenv()), error = function(e) NULL)
    if (is.null(x)) stop("predict: the rows the omitted ", r$term, " excluded could not be found", call. = FALSE)
    if (!is.null(r$level)) x <- as.numeric(as.character(x) == r$level) else x <- as.numeric(x)
    v[!is.na(x) & x != r$cut] <- NA
  }
  v
}"#;

/// Each row's score (the gradient of its log likelihood) after `nbreg`,
/// `ologit`/`oprobit` or `mlogit`, one column per element of `stata_b`, and
/// the robust or clustered sandwich Stata builds from them around the model's
/// own variance: `N / (N - 1)`, or `G / (G - 1)` with clusters.
pub(super) const ML_SANDWICH_HELPER: &str = r#"stata_ml_scores <- function(fit) {
  if (inherits(fit, "negbin")) {
    X <- stats::model.matrix(fit)
    y <- fit$y
    mu <- fit$fitted.values
    th <- fit$theta
    gt <- digamma(y + th) - digamma(th) + log(th / (th + mu)) + (mu - y) / (th + mu)
    return(cbind(X * ((y - mu) / (1 + mu / th)), "/lnalpha" = -th * gt))
  }
  if (inherits(fit, "polr")) {
    mf <- stats::model.frame(fit)
    X <- stats::model.matrix(stats::delete.response(stats::terms(fit)), mf)
    X <- X[, colnames(X) != "(Intercept)", drop = FALSE]
    j <- as.integer(stats::model.response(mf))
    z <- c(-Inf, fit$zeta, Inf)
    eta <- drop(X %*% fit$coefficients)
    F <- if (fit$method == "probit") stats::pnorm else stats::plogis
    f <- if (fit$method == "probit") stats::dnorm else stats::dlogis
    up <- z[j + 1] - eta
    lo <- z[j] - eta
    P <- F(up) - F(lo)
    fu <- ifelse(is.finite(up), f(up), 0)
    fl <- ifelse(is.finite(lo), f(lo), 0)
    K <- length(fit$zeta)
    C <- matrix(0, length(j), K)
    for (k in seq_len(K)) C[, k] <- (j == k) * fu / P - (j == k + 1) * fl / P
    return(cbind(X * (-(fu - fl) / P), C))
  }
  # multinom: for each outcome but the base, x times (1 if that outcome, less its probability)
  X <- stats::model.matrix(fit)
  y <- stats::model.response(stats::model.frame(fit))
  P <- stats::fitted(fit)
  if (is.null(dim(P))) P <- cbind(1 - P, P)
  colnames(P) <- fit$lev
  do.call(cbind, lapply(fit$lev[-1], function(k) X * ((y == k) - P[, k])))
}
stata_ml_sandwich <- function(fit, V, cluster = NULL, kind = "") {
  S <- stata_ml_scores(fit)
  # weighted scores: sampling weights as they are, frequency ones as copies
  w <- if (inherits(fit, "negbin")) fit$prior.weights else stats::model.weights(stats::model.frame(fit))
  if (!is.null(w)) S <- S * (if (kind == "f" && is.null(cluster)) sqrt(w) else w)
  if (is.null(cluster)) {
    n <- if (kind == "f") sum(w) else nrow(S)
    out <- V %*% crossprod(S) %*% V * n / (n - 1)
  } else {
    sg <- rowsum(S, cluster)
    G <- nrow(sg)
    out <- V %*% crossprod(sg) %*% V * G / (G - 1)
  }
  dimnames(out) <- dimnames(V)
  out
}"#;

/// The likelihood-ratio test of alpha = 0 after `nbreg`: the Poisson model on
/// the same rows against it, a chi-bar-squared with half its mass at zero.
pub(super) const NBREG_LR_HELPER: &str = r#"stata_nbreg_lr <- function(fit) {
  p <- stats::glm(fit$y ~ stats::model.matrix(fit) - 1, family = stats::poisson(),
                  offset = fit$offset, control = stats::glm.control(epsilon = 1e-14, maxit = 100))
  chi2 <- max(0, 2 * (as.numeric(stats::logLik(fit)) - as.numeric(stats::logLik(p))))
  pr <- 0.5 * stats::pchisq(chi2, 1, lower.tail = FALSE)
  cat(sprintf("LR test of alpha=0: chibar2(01) = %.2f  Prob >= chibar2 = %.4f\n", chi2, pr))
  invisible(c(chi2_c = chi2, p = pr))
}"#;
