//! `glm`, fitted by maximum likelihood as Stata 19 does by default, observed on
//! StatCraft's probes 220-222 and 272 and probes of our own (2026-11-15); Stata's code
//! was not read.
//!
//! - The start is one IRLS step from mu = (y + mean(y)) / 2 (Bernoulli: (y + 0.5) / 2),
//!   then Stata's Newton-Raphson on the log likelihood with the observed information
//!   (the iteration log is that path).
//! - The variance is the inverse of the observed information (OIM), times the scale:
//!   1 for poisson, binomial and nbinomial, Pearson chi2 / df for gaussian; scale(x2)
//!   uses Pearson chi2 / df for the discrete families too (with Stata's note).
//!   Robust: the sandwich with N / (N - 1); clustered: G / (G - 1).
//! - The header: deviance and Pearson chi2 with their ratios to df = N - k, AIC =
//!   (-2 ll + 2 k) / N, BIC = deviance - df ln N; gaussian's log likelihood takes the
//!   variance as deviance / N. Frequency weights count as copies (N their sum).
//! - Translated: gaussian (identity), poisson (log), binomial (logit, probit,
//!   cloglog; with n, logit), nbinomial k (log). Not yet: irls, gamma and inverse
//!   gaussian (Stata's default scale there is not Pearson / df exactly), other links.

use super::model::{level, vce, Estimates, Fit, Spec};
use super::stats::r_logical;
use super::weights::W;
use super::*;

pub(super) const GLM_HELPER: &str = r##"stata_glm <- function(formula, data, family, link, n = NULL, k = 1, weights = NULL,
                      wtype = NULL, vce = "oim", cluster = NULL, scale = NULL, offrow = NULL) {
  mf <- stats::model.frame(formula, data = data, na.action = stats::na.omit)
  if (!is.null(weights)) {
    w0 <- data[[weights]][match(rownames(mf), rownames(data))]
    keep <- !is.na(w0)
    mf <- mf[keep, , drop = FALSE]
    w0 <- w0[keep]
  } else w0 <- rep(1, nrow(mf))
  y <- as.numeric(stats::model.response(mf))
  X <- stats::model.matrix(stats::terms(mf), mf)
  off <- stats::model.offset(mf)
  if (is.null(off)) off <- rep(0, length(y))
  N <- length(y)
  nn <- if (is.null(n)) rep(1, N) else if (is.character(n)) data[[n]][match(rownames(mf), rownames(data))] else rep(n, N)
  # aweights are scaled to sum to N; fweights count as copies
  w <- if (identical(wtype, "aweight")) w0 * N / sum(w0) else w0
  Nobs <- if (identical(wtype, "fweight")) sum(w0) else N
  # the link: eta from mu, mu from eta, g'(mu), g''(mu)
  L <- switch(link,
    identity = list(f = function(m) m, inv = function(e) e, d1 = function(m) 1 + 0 * m, d2 = function(m) 0 * m),
    log = list(f = log, inv = exp, d1 = function(m) 1 / m, d2 = function(m) -1 / m^2),
    logit = list(f = function(m) log(m / (nn - m)), inv = function(e) nn * stats::plogis(e),
                 d1 = function(m) nn / (m * (nn - m)), d2 = function(m) -nn * (nn - 2 * m) / (m^2 * (nn - m)^2)),
    probit = list(f = function(m) stats::qnorm(m), inv = function(e) stats::pnorm(e),
                  d1 = function(m) 1 / stats::dnorm(stats::qnorm(m)),
                  d2 = function(m) stats::qnorm(m) / stats::dnorm(stats::qnorm(m))^2),
    cloglog = list(f = function(m) log(-log(1 - m)), inv = function(e) 1 - exp(-exp(e)),
                   d1 = function(m) 1 / ((1 - m) * -log(1 - m)),
                   d2 = function(m) { l <- -log(1 - m); -(1 - l) / ((1 - m) * l)^2 }))
  # the variance function V(mu), V'(mu), and the log likelihood at scale 1
  Fm <- switch(family,
    gaussian = list(v = function(m) 1 + 0 * m, v1 = function(m) 0 * m,
                    ll = function(m) -0.5 * sum(w * (y - m)^2)),
    poisson = list(v = function(m) m, v1 = function(m) 1 + 0 * m,
                   ll = function(m) sum(w * (y * log(m) - m - lgamma(y + 1)))),
    binomial = list(v = function(m) m * (1 - m / nn), v1 = function(m) 1 - 2 * m / nn,
                    ll = function(m) sum(w * (lchoose(nn, y) + ifelse(y > 0, y * log(m / nn), 0) +
                                              ifelse(nn - y > 0, (nn - y) * log(1 - m / nn), 0)))),
    nbinomial = list(v = function(m) m + k * m^2, v1 = function(m) 1 + 2 * k * m,
                     ll = function(m) sum(w * (lgamma(y + 1 / k) - lgamma(1 / k) - lgamma(y + 1) +
                                                 y * log(k * m / (1 + k * m)) - log(1 + k * m) / k))))
  mu_of <- function(b) L$inv(drop(X %*% b) + off)
  ll <- function(b) {
    m <- mu_of(b)
    if (any(!is.finite(m))) return(NaN)
    Fm$ll(m)
  }
  Nw <- sum(w)
  # gaussian: the likelihood with the variance concentrated out (deviance / N), as Stata's log shows
  if (family == "gaussian") Fm$ll <- function(m) -0.5 * (Nw * log(2 * pi * sum(w * (y - m)^2) / Nw) + Nw)
  deriv <- function(b) {
    m <- mu_of(b)
    v <- Fm$v(m)
    g1 <- L$d1(m)
    wo <- 1 / (v * g1^2) + (y - m) * (Fm$v1(m) * g1 + v * L$d2(m)) / (v^2 * g1^3)
    g <- colSums(X * (w * (y - m) / (v * g1)))
    nh <- crossprod(X * (w * wo), X)
    if (family == "gaussian") {
      D <- sum(w * (y - m)^2)
      nh <- (Nw / D) * nh - (2 * Nw / D^2) * tcrossprod(g)
      g <- (Nw / D) * g
    }
    list(ll = Fm$ll(m), g = g, nh = nh)
  }
  # Stata's start: one IRLS step from its mu: (y + mean(y)) / 2, binomial n (y + 0.5) / (n + 1)
  # (probes GL, 2026-11-15); a log link's start kept above 0
  m0 <- if (family == "binomial") nn * (y + 0.5) / (nn + 1) else (y + sum(w * y) / sum(w)) / 2
  if (link == "log") m0 <- pmax(m0, 1e-8)
  e0 <- L$f(m0) - off
  z <- e0 + (y - m0) * L$d1(m0)
  ww <- w / (Fm$v(m0) * L$d1(m0)^2)
  b0 <- stats::lm.wfit(X, z, ww)$coefficients
  if (anyNA(b0)) stop("glm with collinear regressors is not translated yet", call. = FALSE)
  # gaussian identity: the start is the estimate, and Stata's log is that one line
  path <- if (family == "gaussian" && link == "identity") {
    list(b = b0, converged = TRUE, lls = ll(b0), odd = FALSE)
  } else stata_ml_path(ll, deriv, b0)
  if (!path$converged) stop("glm: Stata's Newton-Raphson did not converge here (Stata: r(430))", call. = FALSE)
  b <- path$b
  names(b) <- colnames(X)
  m <- mu_of(b)
  v <- Fm$v(m)
  kk <- ncol(X)
  df <- Nobs - kk
  dev <- switch(family,
    gaussian = sum(w * (y - m)^2),
    poisson = 2 * sum(w * (ifelse(y > 0, y * log(y / m), 0) - (y - m))),
    binomial = 2 * sum(w * (ifelse(y > 0, y * log(y / m), 0) + ifelse(nn - y > 0, (nn - y) * log((nn - y) / (nn - m)), 0))),
    nbinomial = 2 * sum(w * (ifelse(y > 0, y * log(y / m), 0) - (y + 1 / k) * log((1 + k * y) / (1 + k * m)))))
  pearson <- sum(w * (y - m)^2 / v)
  # the scale: Pearson chi2 / df for gaussian, and for the others with scale(x2)
  phi <- if (family == "gaussian" || identical(scale, "x2")) pearson / df else 1
  shown <- if (family == "gaussian") phi else 1
  llv <- Fm$ll(m)
  # the observed information at scale 1 (gaussian's not concentrated)
  B <- solve(crossprod(X * (w * (1 / (v * L$d1(m)^2) + (y - m) * (Fm$v1(m) * L$d1(m) + v * L$d2(m)) / (v^2 * L$d1(m)^3))), X))
  if (vce == "oim") {
    V <- phi * B
  } else {
    s <- X * (w * (y - m) / (v * L$d1(m)))
    if (vce == "cluster") {
      cl <- data[[cluster]][match(rownames(mf), rownames(data))]
      if (anyNA(cl)) stop("the cluster variable is missing in rows the model uses; Stata would leave those rows out", call. = FALSE)
      S <- rowsum(s, cl)
      G <- nrow(S)
      V <- G / (G - 1) * B %*% crossprod(S) %*% B
    } else {
      V <- N / (N - 1) * B %*% crossprod(s) %*% B
    }
  }
  dimnames(V) <- list(names(b), names(b))
  fit <- list(coefficients = b, fitted.values = m, linear.predictors = drop(X %*% b) + off, y = y,
              prior.weights = w, model = mf, terms = stats::terms(mf), rank = kk, df.residual = df,
              stata_lls = path$lls, stata_odd = path$odd, G = if (vce == "cluster") G else NULL)
  fit$stata <- list(N = Nobs, k = kk, df = df, phi = shown, deviance = dev, pearson = pearson, ll = llv,
                    aic = (-2 * llv + 2 * kk) / Nobs, bic = dev - df * log(Nobs), ic = length(path$lls) - 1,
                    family = family, link = link, n = n, k_nb = k, scaled = identical(scale, "x2"), offrow = offrow,
                    sumw = if (identical(wtype, "aweight")) sum(w0) else NULL,
                    G = if (vce == "cluster") G else NULL)
  class(fit) <- c("stata_glm", "lm")
  list(fit = fit, b = b, V = V)
}
# the header Stata 19 prints above glm's table
stata_glm_header <- function(fit, robust) {
  s <- fit$stata
  pad <- function(x, w) formatC(x, width = w)
  g <- function(x, f = "%9.0g") trimws(stata_fmt(x, f))
  nn <- if (is.null(s$n)) "" else as.character(s$n)
  vf <- switch(s$family, gaussian = c("1", "Gaussian"), poisson = c("u", "Poisson"),
    binomial = if (is.null(s$n)) c("u*(1-u)", "Bernoulli") else c(paste0("u*(1-u/", nn, ")"), "Binomial"),
    nbinomial = c(paste0("u+(", g(s$k_nb), ")u^2"), "Neg. Binomial"))
  lf <- switch(s$link, identity = c("u", "Identity"), log = c("ln(u)", "Log"),
    logit = if (is.null(s$n)) c("ln(u/(1-u))", "Logit") else c(paste0("ln(u/(", nn, "-u))"), "Logit"),
    probit = c("invnorm(u)", "Probit"), cloglog = c("ln(-ln(1-u))", "Complementary log–log"))
  left <- function(a, b) cat(formatC(a, width = -50), b, "\n", sep = "")
  cat("\n")
  left("Generalized linear models", paste0("Number of obs   = ", pad(stata_tab_count(s$N), 10)))
  left("Optimization     : ML", paste0("Residual df     = ", pad(stata_tab_count(s$df), 10)))
  left("", paste0("Scale parameter = ", pad(g(s$phi), 10)))
  left(paste0("Deviance         = ", pad(g(s$deviance, "%12.0g"), 12)), paste0("(1/df) Deviance = ", pad(g(s$deviance / s$df), 10)))
  left(paste0("Pearson          = ", pad(g(s$pearson, "%12.0g"), 12)), paste0("(1/df) Pearson  = ", pad(g(s$pearson / s$df), 10)))
  cat("\n")
  left(paste0("Variance function: V(u) = ", vf[1]), paste0("[", vf[2], "]"))
  left(paste0("Link function    : g(u) = ", lf[1]), paste0("[", lf[2], "]"))
  cat("\n")
  left("", paste0("AIC             = ", pad(g(s$aic), 10)))
  lab <- if (robust) paste0("Log pseudolikelihood = ", pad(g(s$ll, "%12.0g"), 12)) else paste0("Log likelihood   = ", g(s$ll, "%12.0g"))
  left(lab, paste0("BIC             = ", pad(g(s$bic), 10)))
  cat("\n")
}
# mu for predict: the inverse link of the linear prediction (n times it for binomial n)
stata_glm_mu <- function(fit, d, xb = FALSE) {
  tt <- stats::delete.response(fit$terms)
  mf <- stats::model.frame(tt, d, na.action = stats::na.pass)
  X <- stats::model.matrix(tt, mf)
  off <- stats::model.offset(mf)
  eta <- drop(X %*% fit$coefficients) + if (is.null(off)) 0 else off
  if (xb) return(eta)
  s <- fit$stata
  nn <- if (is.null(s$n)) 1 else if (is.character(s$n)) d[[s$n]] else s$n
  switch(s$link, identity = eta, log = exp(eta), logit = nn * stats::plogis(eta),
         probit = stats::pnorm(eta), cloglog = 1 - exp(-exp(eta)))
}"##;

impl Session {
    /// `glm depvar indepvars, family() link() ...` by maximum likelihood.
    pub(super) fn glm(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "glm")?;
        if let Some(o) = opts.other(&[
            ("family", 1),
            ("link", 1),
            ("eform", 2),
            ("noconstant", 5),
            ("exposure", 3),
            ("offset", 6),
            ("robust", 1),
            ("vce", 3),
            ("scale", 3),
            ("level", 3),
            ("nolog", 5),
            ("ml", 2),
        ]) {
            return Err(no(format!("glm with the option {o} is not translated yet")));
        }
        // the family and its n or k
        let fam_text = opts
            .arg("family", 1)
            .map_or_else(|| "gaussian".to_string(), |s| s.trim().to_string());
        let mut fw = fam_text.split_whitespace();
        let fam_word = fw.next().unwrap_or("gaussian").to_lowercase();
        let fam_arg = fw.next().map(str::to_string);
        if fw.next().is_some() {
            return Err(no(format!("glm, family({fam_text}) is not translated")));
        }
        let starts =
            |full: &str, min: usize| fam_word.len() >= min && full.starts_with(fam_word.as_str());
        let family = if starts("gaussian", 3) {
            "gaussian"
        } else if starts("poisson", 1) {
            "poisson"
        } else if starts("binomial", 1) {
            "binomial"
        } else if starts("nbinomial", 2) {
            "nbinomial"
        } else {
            return Err(no(format!(
                "glm, family({fam_word}) is not translated yet (gaussian, poisson, binomial and \
                 nbinomial are)"
            )));
        };
        let mut n_arg = "NULL".to_string();
        let mut k = 1.0;
        match (family, &fam_arg) {
            ("binomial", Some(a)) => {
                n_arg = match a.parse::<f64>() {
                    Ok(v) if v >= 1.0 => r_number(v),
                    _ if is_name(a) => r_text(a),
                    _ => return Err(no(format!("glm, family(binomial {a}) is not translated"))),
                }
            }
            ("nbinomial", Some(a)) => match a.parse::<f64>() {
                Ok(v) if v > 0.0 => k = v,
                _ => {
                    return Err(no(format!(
                        "glm, family(nbinomial {a}) is not translated yet (a number k is)"
                    )))
                }
            },
            (_, Some(a)) => return Err(no(format!("glm, family({family} {a}) is not translated"))),
            _ => {}
        }
        let canonical = match family {
            "gaussian" => "identity",
            "binomial" => "logit",
            _ => "log",
        };
        let link_word = opts
            .arg("link", 1)
            .map(|l| l.trim().to_lowercase())
            .unwrap_or_else(|| canonical.to_string());
        let link = ["identity", "log", "logit", "probit", "cloglog"]
            .into_iter()
            .find(|l| !link_word.is_empty() && l.starts_with(link_word.as_str()))
            .ok_or_else(|| no(format!("glm, link({link_word}) is not translated yet")))?;
        let checked = matches!(
            (family, link, n_arg.as_str()),
            ("gaussian", "identity", _)
                | ("poisson", "log", _)
                | ("binomial", "logit", _)
                | ("binomial", "probit" | "cloglog", "NULL")
                | ("nbinomial", "log", _)
        );
        if !checked {
            return Err(no(format!(
                "glm, family({family}) link({link}) is not translated yet"
            )));
        }
        let scale = match opts.arg("scale", 3).map(|s| s.trim().to_lowercase()) {
            None => "NULL",
            Some(s) if s == "x2" && family != "gaussian" => "\"x2\"",
            Some(s) if s == "dev" => {
                out.line("stop(\"option scale(dev) allowed only with option irls (Stata: r(198))\", call. = FALSE)");
                return Ok(());
            }
            Some(s) => return Err(no(format!("glm, scale({s}) is not translated yet"))),
        };
        let spec = Spec::read(c, "glm")?;
        self.helpers_for(&spec);
        let level = level(opts)?;
        let (robust, cluster) = vce(opts)?;
        let mut right = spec.right(opts.has("noconstant", 5))?;
        // offset() or exposure(): in the formula, and Stata's row "ln(t) | 1 (exposure)"
        let offrow = match (opts.arg("exposure", 3), opts.arg("offset", 6)) {
            (Some(_), Some(_)) => {
                return Err(no(
                    "glm with both offset() and exposure() is not translated",
                ))
            }
            (Some(v), None) if is_name(v.trim()) => {
                right.push_str(&format!(" + offset(log({}))", r_name(v.trim())));
                format!(
                    ", offrow = c({}, \"exposure\")",
                    r_text(&format!("ln({})", v.trim()))
                )
            }
            (None, Some(v)) if is_name(v.trim()) => {
                right.push_str(&format!(" + offset({})", r_name(v.trim())));
                format!(", offrow = c({}, \"offset\")", r_text(v.trim()))
            }
            (None, None) => String::new(),
            _ => return Err(no("glm: offset() and exposure() take one variable")),
        };
        let w = self.weight_of(c, &[W::F, W::P, W::A, W::I])?;
        if matches!(w, Some((W::A, _))) && family != "gaussian" {
            return Err(no("glm with aweights is translated for gaussian only"));
        }
        if w.is_some() && cluster.is_some() {
            return Err(no("glm with weights and clusters is not translated yet"));
        }
        if matches!(w, Some((W::F | W::I, _))) && robust {
            return Err(no(
                "glm with fweights or iweights and robust errors is not translated yet",
            ));
        }
        let robust = robust || matches!(w, Some((W::P, _)));
        let vce_kind = if cluster.is_some() {
            "cluster"
        } else if robust {
            "robust"
        } else {
            "oim"
        };
        let rows = self.rows_for(c, out)?;
        let src = self.weighted_rows(rows, &w);
        let (weights, wtype) = match &w {
            Some((k, v)) => (
                r_text(v),
                r_text(match k {
                    W::F => "fweight",
                    W::A => "aweight",
                    W::P => "pweight",
                    W::I => "iweight",
                }),
            ),
            None => ("NULL".to_string(), "NULL".to_string()),
        };
        let clus = match &cluster {
            Some(v) if is_name(v) => format!(", cluster = {}", r_text(v)),
            Some(v) => return Err(no(format!("glm, vce(cluster {v}) takes one variable"))),
            None => String::new(),
        };
        self.helpers.insert("stata_glm");
        self.helpers.insert("stata_probit_newton");
        self.helpers.insert("stata_ml_header");
        self.helpers.insert("stata_fmt");
        self.helpers.insert("stata_tables");
        out.line(format!(
            "stata_m <- stata_glm({} ~ {right}, data = {src}, family = {}, link = {}, n = {n_arg}, k = {}, weights = {weights}, wtype = {wtype}, vce = {}{clus}, scale = {scale}{offrow})",
            r_name(&spec.dep),
            r_text(family),
            r_text(link),
            r_number(k),
            r_text(vce_kind)
        ));
        out.line("stata_estimates <- stata_m$fit");
        out.line("stata_b <- stata_m$b");
        out.line("stata_vcov <- stata_m$V");
        out.line("stata_df <- Inf");
        out.line(self.keep_sample(&src));
        let d = self.data()?;
        out.package("dplyr");
        out.line(self.esample_line(&d, &src));
        self.esample_new = true;
        self.fit_robust = robust || cluster.is_some();
        self.fit_cluster = cluster.is_some();
        self.estimates = Some(Estimates {
            fit: Fit::Glm,
            dep: spec.dep.clone(),
            factors: spec.factors.clone(),
        });
        let args = super::returned::e_args(&spec.dep, vce_kind, cluster.as_deref(), &w);
        self.keep_e(c, out, &args, "");
        if !self.quiet(c) {
            // Stata's note on aweights: their sum, with commas and up to 16 digits
            out.line("if (!is.null(stata_estimates$stata$sumw)) cat(\"(sum of wgt is \", format(stata_estimates$stata$sumw, digits = 16, big.mark = \",\"), \")\n\", sep = \"\")");
            let eform = opts.has("eform", 2);
            let (label, note) = match (eform, family, link) {
                (false, _, _) => ("Coefficient", ""),
                (true, "poisson" | "nbinomial", "log") => {
                    ("IRR", "Note: _cons estimates baseline incidence rate.")
                }
                (true, "binomial", "logit") => {
                    ("Odds ratio", "Note: _cons estimates baseline odds.")
                }
                (true, _, _) => ("exp(b)", ""),
            };
            let scaled = if scale == "\"x2\"" {
                "(Standard errors scaled using square root of Pearson X2-based dispersion.)"
            } else {
                ""
            };
            out.line(format!(
                "if (!stata_ml_print(stata_estimates, \"Generalized linear models\", robust = {}, level = {}, label = {}, eform = {}, note = {}, iterations = {}, glm = {})) stata_coefs(stata_b, stata_vcov, stata_df, level = {}, eform = {}, label = {})",
                r_logical(robust || cluster.is_some()),
                r_number(level),
                r_text(label),
                r_logical(eform),
                if note.is_empty() { "NULL".to_string() } else { r_text(note) },
                if opts.has("nolog", 5) { "NULL" } else { "\"glm\"" },
                if scaled.is_empty() { "TRUE".to_string() } else { r_text(scaled) },
                r_number(level),
                r_logical(eform),
                r_text(label)
            ));
        }
        out.line("rm(stata_m)");
        out.caveat(
            "glm by maximum likelihood as Stata 19 fits it: its start (one IRLS step), its \
             Newton-Raphson and the observed information (OIM), with its scale",
        );
        Ok(())
    }
}
