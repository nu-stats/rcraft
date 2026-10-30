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

use super::model::{cluster_rows, keep_sample, level, vce, Estimates, Fit, Spec};
use super::stats::r_logical;
use super::*;

/// The observed information of a binary or Poisson glm, and its robust or
/// clustered sandwich the way Stata scales them.
pub(super) const GLM_VCOV_HELPER: &str = r#"stata_glm_vcov <- function(fit, robust = FALSE, cluster = NULL) {
  X <- stats::model.matrix(fit)
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
  V <- solve(crossprod(X * w, X))
  if (robust) {
    s <- X * ((y - mu) * me / fam$variance(mu))
    if (is.null(cluster)) {
      n <- nrow(X)
      V <- V %*% crossprod(s) %*% V * n / (n - 1)
    } else {
      sg <- rowsum(s, cluster)
      G <- nrow(sg)
      V <- V %*% crossprod(sg) %*% V * G / (G - 1)
    }
  }
  dimnames(V) <- list(colnames(X), colnames(X))
  V
}"#;

/// The variance of nbreg's coefficients and `ln(alpha)` together, from the
/// observed information of the whole likelihood (the analytic gradient,
/// differenced once).
pub(super) const NBREG_VCOV_HELPER: &str = r#"stata_nbreg_vcov <- function(fit) {
  X <- stats::model.matrix(fit)
  y <- fit$y
  off <- stats::model.offset(stats::model.frame(fit))
  if (is.null(off)) off <- 0
  k <- ncol(X)
  nll <- function(p) {
    mu <- exp(drop(X %*% p[seq_len(k)]) + off)
    th <- exp(-p[k + 1])
    -sum(lgamma(y + th) - lgamma(th) - lgamma(y + 1) + th * log(th / (th + mu)) + y * log(mu / (th + mu)))
  }
  grad <- function(p) {
    mu <- exp(drop(X %*% p[seq_len(k)]) + off)
    th <- exp(-p[k + 1])
    gb <- crossprod(X, (y - mu) / (1 + mu / th))
    gt <- sum(digamma(y + th) - digamma(th) + log(th / (th + mu)) + (mu - y) / (th + mu))
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
  X <- stats::model.matrix(fit)
  y <- fit$y
  off <- if (is.null(fit$offset)) 0 else fit$offset
  b <- stats::coef(fit)
  for (i in 1:25) {
    eta <- drop(X %*% b) + off
    P <- stats::pnorm(eta)
    f <- stats::dnorm(eta)
    s <- colSums(X * (y * f / P - (1 - y) * f / (1 - P)))
    w <- y * (eta * f * P + f^2) / P^2 + (1 - y) * (f^2 - eta * f * (1 - P)) / (1 - P)^2
    step <- solve(crossprod(X * w, X), s)
    b <- b + step
    if (max(abs(step)) < 1e-15 * (1 + max(abs(b)))) break
  }
  stats::update(fit, start = unname(b), control = stats::glm.control(epsilon = 1e-15, maxit = 1))
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
pub(super) const ML_HEADER_HELPER: &str = r#"stata_ml_header <- function(fit, robust = FALSE) {
  ll <- as.numeric(stats::logLik(fit))
  n <- NROW(stats::fitted(fit))
  if (inherits(fit, "negbin")) {
    ll0 <- NA_real_
  } else if (inherits(fit, "glm")) {
    ll0 <- ll + (fit$deviance - fit$null.deviance) / 2
  } else {
    nj <- table(fit$model[[1]])
    nj <- nj[nj > 0]
    ll0 <- sum(nj * log(nj / sum(nj)))
  }
  keep <- !grepl("(Intercept)", names(stata_b), fixed = TRUE) & !grepl("^/", names(stata_b))
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
}"#;

impl Session {
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
            ]),
            Fit::Ologit => known.push(("or", 2)),
            Fit::Oprobit => {}
            Fit::Mlogit => known.extend([("noconstant", 5), ("rrr", 3), ("baseoutcome", 4)]),
            Fit::Regress => return self.regress(c, opts, out),
            Fit::Xtreg => return self.xtreg(c, opts, out),
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
        let mut right = spec.right(opts.has("noconstant", 5));
        match (opts.arg("exposure", 3), opts.arg("offset", 6)) {
            (Some(_), Some(_)) => {
                return Err(no(format!(
                    "{name}: Stata takes exposure() or offset(), not both"
                )))
            }
            (Some(e), None) => {
                let e = e.trim();
                if !is_name(e) {
                    return Err(no(format!("exposure({e}) needs one variable name")));
                }
                right.push_str(&format!(" + offset(log({}))", r_name(e)));
            }
            (None, Some(o)) => {
                let o = o.trim();
                if !is_name(o) {
                    return Err(no(format!("offset({o}) needs one variable name")));
                }
                right.push_str(&format!(" + offset({})", r_name(o)));
            }
            (None, None) => {}
        }
        let src = self.rows_for(c, out)?;
        let dep = r_name(&spec.dep);
        let control = "control = stats::glm.control(epsilon = 1e-12, maxit = 100)";
        match fit {
            Fit::Logit | Fit::Probit => {
                let link = if fit == Fit::Logit { "logit" } else { "probit" };
                out.line(format!(
                    "stata_estimates <- stats::glm(as.numeric({dep} != 0) ~ {right}, family = stats::binomial(link = \"{link}\"), data = {src}, {control})"
                ));
                if fit == Fit::Probit {
                    self.helpers.insert("stata_probit_newton");
                    out.line("stata_estimates <- stata_probit_newton(stata_estimates)");
                }
                out.caveat("the outcome is 1 where it is not 0 (and not missing), as Stata takes it");
                out.caveat(
                    "a variable that predicts the outcome perfectly is not dropped as Stata drops \
                     it: R warns and gives a very large coefficient",
                );
            }
            Fit::Poisson => out.line(format!(
                "stata_estimates <- stats::glm({dep} ~ {right}, family = stats::poisson(), data = {src}, {control})"
            )),
            Fit::Nbreg => {
                out.package("MASS");
                out.line(format!(
                    "stata_estimates <- MASS::glm.nb({dep} ~ {right}, data = {src}, {control})"
                ));
            }
            Fit::Ologit | Fit::Oprobit => {
                out.package("MASS");
                let method = if fit == Fit::Ologit { "logistic" } else { "probit" };
                out.line(format!(
                    "stata_estimates <- MASS::polr(factor({dep}) ~ {right}, data = {src}, method = \"{method}\", Hess = TRUE, model = TRUE, control = list(reltol = 1e-14, maxit = 1000))"
                ));
                out.line("stata_levels <- stata_estimates$lev");
            }
            Fit::Mlogit => {
                out.package("nnet");
                let fit_with = |response: &str| {
                    format!(
                        "stata_estimates <- nnet::multinom({response} ~ {right}, data = {src}, trace = FALSE, model = TRUE, maxit = 1000, reltol = 1e-14)"
                    )
                };
                out.line(fit_with(&format!("factor({dep})")));
                out.line("stata_levels <- stata_estimates$lev");
                match opts.arg("baseoutcome", 4) {
                    Some(b) => {
                        let b = b.trim();
                        if b.parse::<f64>().is_err() {
                            return Err(no(format!("baseoutcome({b}) needs a value of {}", spec.dep)));
                        }
                        out.line(format!("stata_base <- {}", r_text(b)));
                    }
                    None => {
                        out.line("stata_base <- names(which.max(table(stata_estimates$model[[1]])))");
                        out.caveat("the base outcome is the most frequent one, as in Stata");
                    }
                }
                out.line(fit_with(&format!(
                    "stats::relevel(factor({dep}), ref = stata_base)"
                )));
            }
            Fit::Regress | Fit::Xtreg => {}
        }
        match fit {
            Fit::Logit | Fit::Probit | Fit::Poisson => {
                self.helpers.insert("stata_glm_vcov");
                out.line("stata_b <- stats::coef(stata_estimates)");
                match &cluster {
                    Some(v) => {
                        out.line(format!("stata_cluster <- {}", cluster_rows(&src, v)));
                        out.line(no_missing_cluster());
                        out.line(
                            "stata_vcov <- stata_glm_vcov(stata_estimates, robust = TRUE, cluster = stata_cluster)",
                        );
                        out.caveat("clustered standard errors carry G / (G - 1), as Stata's do for this model");
                    }
                    None if robust => {
                        out.line("stata_vcov <- stata_glm_vcov(stata_estimates, robust = TRUE)");
                        out.caveat(
                            "robust standard errors carry N / (N - 1), as Stata's do for a \
                             maximum-likelihood model (not regress's HC1)",
                        );
                    }
                    None => out.line("stata_vcov <- stata_glm_vcov(stata_estimates)"),
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
                out.line(
                    "stata_b <- c(stats::coef(stata_estimates), \"/lnalpha\" = log(1 / stata_estimates$theta))",
                );
                out.line("stata_vcov <- stata_nbreg_vcov(stata_estimates)");
                out.caveat(
                    "alpha is 1 / theta of MASS::glm.nb; the standard errors come from the whole \
                     likelihood, alpha included, as Stata's do",
                );
                out.caveat("the likelihood-ratio test of alpha = 0 is not shown");
            }
            Fit::Ologit | Fit::Oprobit => {
                out.line(
                    "stata_b <- c(stats::coef(stata_estimates), stats::setNames(stata_estimates$zeta, paste0(\"/cut\", seq_along(stata_estimates$zeta))))",
                );
                out.line("stata_vcov <- stats::vcov(stata_estimates)");
                out.line("dimnames(stata_vcov) <- list(names(stata_b), names(stata_b))");
                out.caveat("the cut points are R's zeta, which are Stata's /cut1, /cut2, ... with the same sign");
                out.caveat(
                    "the standard errors come from R's numerical Hessian, so the last digit or \
                     two can differ from Stata's",
                );
            }
            Fit::Mlogit => {
                self.helpers.insert("stata_multinom_b");
                out.line("stata_b <- stata_multinom_b(stata_estimates)");
                out.line("stata_vcov <- stats::vcov(stata_estimates)");
                out.line("dimnames(stata_vcov) <- list(names(stata_b), names(stata_b))");
                out.caveat("each coefficient is named outcome:variable; the base outcome has none");
            }
            Fit::Regress | Fit::Xtreg => {}
        }
        if fit.one_outcome() {
            out.line(keep_sample(&src));
        }
        out.line("stata_df <- Inf");
        self.estimates = Some(Estimates {
            fit,
            dep: spec.dep.clone(),
            factors: spec.factors.clone(),
        });
        if Self::quiet(c) {
            return Ok(());
        }
        self.helpers.insert("stata_ml_header");
        self.helpers.insert("stata_coefs");
        out.line(format!(
            "stata_ml_header(stata_estimates, robust = {})",
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
        }
        if fit == Fit::Mlogit {
            out.line("cat(\"base outcome:\", stata_base, \"\\n\")");
        }
        out.caveat("a row with a missing value anywhere in the model is dropped, as in Stata");
        out.caveat("printed in R's layout, not Stata's table");
        Ok(())
    }
}

/// Stata leaves out a row whose cluster is missing; rather than fit the rows
/// differently, the R stops and says so.
pub(super) fn no_missing_cluster() -> &'static str {
    "if (anyNA(stata_cluster)) stop(\"the cluster variable is missing in rows the model uses; Stata would leave those rows out\")"
}
