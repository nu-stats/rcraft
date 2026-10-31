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
use super::weights::W;
use super::*;

/// The observed information of a binary or Poisson glm, and its robust or
/// clustered sandwich the way Stata scales them.
pub(super) const GLM_VCOV_HELPER: &str = r#"stata_glm_vcov <- function(fit, robust = FALSE, cluster = NULL, kind = "") {
  # the weights the model was given (1 without): frequency ones count as copies
  pw <- fit$prior.weights
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
  X <- stats::model.matrix(fit)
  y <- fit$y
  off <- if (is.null(fit$offset)) 0 else fit$offset
  b <- stats::coef(fit)
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
  # weights that are not whole numbers make binomial() warn; Stata takes them
  withCallingHandlers(
    stats::update(fit, start = unname(b), control = stats::glm.control(epsilon = 1e-15, maxit = 1)),
    warning = function(w) if (grepl("non-integer", conditionMessage(w))) invokeRestart("muffleWarning")
  )
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
pub(super) const ML_HEADER_HELPER: &str = r#"stata_ml_header <- function(fit, robust = FALSE, nobs = NULL) {
  ll <- as.numeric(stats::logLik(fit))
  if (inherits(fit, "glm") && fit$family$family == "binomial") {
    # by hand: R's binomial logLik rounds weights that are not whole numbers
    mu <- fit$fitted.values
    ll <- sum(fit$prior.weights * (fit$y * log(mu) + (1 - fit$y) * log(1 - mu)))
  }
  n <- if (is.null(nobs)) NROW(stats::fitted(fit)) else nobs
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
            Fit::Stcox => return self.stcox(c, opts, out),
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
                // the first fit only finds the rows and terms; stata_perfect
                // applies Stata's check for perfect prediction and fits again
                out.line(format!(
                    "stata_estimates <- suppressWarnings(stats::glm(as.numeric({dep} != 0) ~ {right}, family = stats::binomial(link = \"{link}\"), data = {src}{weights}, {control}))"
                ));
                self.helpers.insert("stata_perfect");
                out.line(format!(
                    "stata_estimates <- stata_perfect(stata_estimates{})",
                    if Self::quiet(c) { ", notes = FALSE" } else { "" }
                ));
                if fit == Fit::Probit {
                    self.helpers.insert("stata_probit_newton");
                    out.line("stata_estimates <- stata_probit_newton(stata_estimates)");
                }
                out.caveat("the outcome is 1 where it is not 0 (and not missing), as Stata takes it");
                out.caveat(
                    "a variable that predicts the outcome perfectly is dealt with as Stata does: \
                     omitted with the rows it decides, or the fit stops with Stata's message",
                );
            }
            Fit::Poisson => out.line(format!(
                "stata_estimates <- stats::glm({dep} ~ {right}, family = stats::poisson(), data = {src}{weights}, {control})"
            )),
            Fit::Nbreg => {
                out.package("MASS");
                out.line(format!(
                    "stata_estimates <- MASS::glm.nb({dep} ~ {right}, data = {src}{weights}, {control})"
                ));
            }
            Fit::Ologit | Fit::Oprobit => {
                out.package("MASS");
                let method = if fit == Fit::Ologit { "logistic" } else { "probit" };
                out.line(format!(
                    "stata_estimates <- MASS::polr(factor({dep}) ~ {right}, data = {src}{weights}, method = \"{method}\", Hess = TRUE, model = TRUE, control = list(reltol = 1e-14, maxit = 1000))"
                ));
                out.line("stata_levels <- stata_estimates$lev");
            }
            Fit::Mlogit => {
                out.package("nnet");
                let fit_with = |response: &str| {
                    format!(
                        "stata_estimates <- nnet::multinom({response} ~ {right}, data = {src}{weights}, trace = FALSE, model = TRUE, maxit = 1000, reltol = 1e-14)"
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
                        out.line("stata_base <- local({ w <- stats::model.weights(stata_estimates$model); y <- stata_estimates$model[[1]]; names(which.max(if (is.null(w)) table(y) else tapply(w, y, sum))) })");
                        out.caveat("the base outcome is the most frequent one, as in Stata");
                    }
                }
                out.line(fit_with(&format!(
                    "stats::relevel(factor({dep}), ref = stata_base)"
                )));
            }
            Fit::Regress | Fit::Xtreg | Fit::Ivregress | Fit::Margins | Fit::Stcox => {}
        }
        match fit {
            Fit::Logit | Fit::Probit | Fit::Poisson => {
                self.helpers.insert("stata_glm_vcov");
                self.fit_robust = (robust || cluster.is_some()) && w.is_none();
                self.fit_cluster = cluster.is_some() && w.is_none();
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
            Fit::Regress | Fit::Xtreg | Fit::Ivregress | Fit::Margins | Fit::Stcox => {}
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
            out.line(keep_sample(&src));
        }
        let d = self.data()?;
        out.package("dplyr");
        out.line(super::model::esample_line(&d, &src));
        self.esample_new = true;
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
            "stata_ml_header(stata_estimates, robust = {}{})",
            r_logical(robust),
            if matches!(w, Some((W::F, _))) {
                ", nobs = sum(stats::model.weights(stats::model.frame(stata_estimates)))"
            } else {
                ""
            }
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
        out.caveat("printed in R's layout, not Stata's table");
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
  for (t in labs) {
    v <- mf[[t]]
    if (is.null(v)) next
    if (is.factor(v)) {
      name <- all.vars(stats::as.formula(paste("~", t)))[1]
      lv <- levels(v)
      o <- suppressWarnings(order(as.numeric(lv), lv))
      cols <- lapply(lv[o], function(l) list(name = paste0(l, ".", name), x = as.numeric(v == l)))
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
  refit(call)
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
