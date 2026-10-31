//! P2B.4, survival data: `stset` and `stcox`.
//!
//! `stset` makes Stata's four variables, `_st` (the row is used), `_d` (it
//! ends in a failure), `_t` (the time it ends) and `_t0` (the time it enters),
//! and `stcox` fits the Cox model on them with R's `survival::coxph`. What
//! Stata 19 does, as checked against it:
//!
//! - ties are Breslow's unless `efron` or `exactp` (R's `exact`, the exact
//!   partial likelihood) is asked for; R's own default is Efron's. `exactm`,
//!   the exact marginal likelihood, has no counterpart in R and is refused;
//! - the estimates, standard errors and log likelihoods are coxph's, to 15
//!   digits; robust errors are coxph's times N / (N - 1) and clustered ones
//!   times G / (G - 1);
//! - a row that enters at or after the time it ends (`enter()`) is not used;
//! - the table shows hazard ratios unless `nohr`, while e(b) holds the
//!   coefficients; `predict, xb` is x b with nothing subtracted (R's
//!   `predict()` would centre it) and `hr` its exponential.

use super::model::{level, vce, Estimates, Fit, Spec};
use super::stats::r_logical;
use super::*;

/// `stset`: Stata's `_st`, `_d`, `_t` and `_t0`, and its summary.
pub(super) const STSET_HELPER: &str = r#"stata_stset <- function(d, time, fail = NULL, values = NULL, id = NULL,
                        origin = 0, enter = 0, exit = Inf, scale = 1) {
  num <- function(v) as.vector(unclass(v))
  t <- num(time)
  dd <- if (is.null(fail)) rep(1, length(t)) else {
    f <- num(fail)
    if (is.null(values)) as.numeric(!is.na(f) & f != 0) else as.numeric(f %in% values)
  }
  t1 <- (t - origin) / scale
  t0 <- rep_len((pmax(origin, enter) - origin) / scale, length(t))
  if (!is.null(id)) {
    # several records of one subject: each starts where the one before ended
    iv <- num(id)
    o <- order(iv, t1)
    n <- length(o)
    prev <- c(NA, t1[o][-n])
    same <- c(FALSE, iv[o][-1] == iv[o][-n])
    t0[o] <- ifelse(!is.na(same) & same, pmax(t0[o], prev), t0[o])
  }
  ex <- (exit - origin) / scale
  dd[t1 > ex] <- 0
  t1 <- pmin(t1, ex)
  st <- as.numeric(!is.na(t1) & t1 > t0)
  d[["_st"]] <- st
  d[["_d"]] <- ifelse(st == 1, dd, NA_real_)
  d[["_t"]] <- ifelse(st == 1, t1, NA_real_)
  d[["_t0"]] <- ifelse(st == 1, t0, NA_real_)
  u <- st == 1
  cat(sprintf("%d total observations\n%d observations excluded\n%d observations remaining\n%d failures in single-record/single-failure data\n%g total analysis time at risk and under observation\n",
              length(st), sum(!u), sum(u), as.integer(sum(dd[u])), sum(t1[u] - t0[u])))
  d
}"#;

/// `stcox` on the stset data, Stata's way.
pub(super) const STCOX_HELPER: &str = r#"stata_stcox <- function(formula, data, ties = "breslow", robust = FALSE, cluster = NULL) {
  ctl <- survival::coxph.control(eps = 1e-12, toler.chol = 1e-15, iter.max = 100)
  if (ties == "exact") {
    # R's exact partial likelihood takes no entry times
    if (any(data[["_t0"]] != 0)) stop("stcox, exactp with late entry (enter() or id()) is not translated yet")
    formula <- stats::update(formula, survival::Surv(`_t`, `_d`) ~ .)
  }
  fit <- if (is.null(cluster) && !robust) {
    survival::coxph(formula, data = data, ties = ties, control = ctl, model = TRUE)
  } else {
    # robust without clusters: each row its own (coxph wants one or the other);
    # in a column of its own, since coxph looks for it in the data first
    data[["stata_coxph_cluster"]] <- if (is.null(cluster)) seq_len(nrow(data)) else cluster
    survival::coxph(formula, data = data, ties = ties, cluster = stata_coxph_cluster, control = ctl, model = TRUE)
  }
  V <- stats::vcov(fit)
  N <- fit$n
  G <- if (is.null(cluster)) NA else length(unique(cluster))
  # Stata's small-sample factors on coxph's sandwich
  if (!is.null(cluster)) V <- V * G / (G - 1) else if (robust) V <- V * N / (N - 1)
  b <- stats::coef(fit)
  dimnames(V) <- list(names(b), names(b))
  list(fit = fit, b = b, V = V, N = N, G = G, ll = fit$loglik[2], ll_0 = fit$loglik[1],
       N_fail = fit$nevent, risk = sum(data[["_t"]] - data[["_t0"]]))
}
stata_stcox_header <- function(m, robust) {
  k <- length(m$b)
  cat(sprintf("No. of subjects = %10d                 Number of obs = %10d\n", m$N, m$N))
  cat(sprintf("No. of failures = %10d\nTime at risk    = %10g\n", as.integer(m$N_fail), m$risk))
  if (robust) {
    w <- drop(t(m$b) %*% solve(m$V, m$b))
    cat(sprintf("Wald chi2(%d)    = %10.2f\nProb > chi2     = %10.4f\n", k, w, stats::pchisq(w, k, lower.tail = FALSE)))
  } else {
    lr <- 2 * (m$ll - m$ll_0)
    cat(sprintf("LR chi2(%d)      = %10.2f\nProb > chi2     = %10.4f\n", k, lr, stats::pchisq(lr, k, lower.tail = FALSE)))
  }
  cat(sprintf("Log likelihood  = %.7g\n", m$ll))
  if (!is.na(m$G)) cat(sprintf("(Std. err. adjusted for %d clusters)\n", m$G))
  cat("\n")
  invisible(NULL)
}
stata_st_xb <- function(fit, newdata) {
  tt <- stats::delete.response(stats::terms(fit))
  mf <- stats::model.frame(tt, newdata, xlev = fit$xlevels, na.action = stats::na.pass)
  X <- stats::model.matrix(tt, mf)
  b <- stats::coef(fit)
  drop(X[, names(b), drop = FALSE] %*% b)
}"#;

/// `estat phtest`: the Grambsch-Therneau test of proportional hazards on the
/// analysis time, as Stata's (R's `cox.zph` now gives an exact score test,
/// which is a different number): with the Schoenfeld residuals s, the model
/// variance V and g the event times less their mean, the global chi2 is
/// `D u' V u / sum(g^2)` with `u = sum g s`, and each covariate's
/// `D (V u)_j^2 / (V_jj sum(g^2))`; rho is the correlation of the scaled
/// residuals with time.
pub(super) const PHTEST_HELPER: &str = r#"stata_phtest <- function(fit, detail = FALSE) {
  sr <- stats::residuals(fit, type = "schoenfeld")
  if (is.null(dim(sr))) sr <- matrix(sr, ncol = 1, dimnames = list(names(sr), names(stats::coef(fit))))
  tt <- as.numeric(rownames(sr))
  V <- if (is.null(fit$naive.var)) fit$var else fit$naive.var
  D <- nrow(sr)
  g <- tt - mean(tt)
  u <- colSums(sr * g)
  q <- ncol(sr)
  global <- drop(t(u) %*% V %*% u) * D / sum(g^2)
  Vu <- drop(V %*% u)
  chi <- D * Vu^2 / (diag(V) * sum(g^2))
  ss <- sr %*% V * D + matrix(stats::coef(fit), D, q, byrow = TRUE)
  rho <- apply(ss, 2, function(r) stats::cor(tt, r))
  cat("Test of proportional-hazards assumption\nTime function: Analysis time\n\n")
  tab <- data.frame(rho = rho, chi2 = chi, df = 1, p = stats::pchisq(chi, 1, lower.tail = FALSE))
  rownames(tab) <- colnames(sr)
  if (detail) print(tab, digits = 6)
  cat(sprintf("Global test: chi2(%d) = %.2f   Prob>chi2 = %.4f\n", q, global, stats::pchisq(global, q, lower.tail = FALSE)))
  invisible(list(table = tab, chi2 = global, df = q))
}"#;

/// What `stset` declared, for `stcox` to read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StSet {
    /// The subject variable of `id()`, if any.
    pub(super) id: Option<String>,
}

impl Session {
    pub(super) fn stset(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[
            ("failure", 1),
            ("id", 2),
            ("origin", 1),
            ("enter", 2),
            ("exit", 2),
            ("scale", 2),
            ("noshow", 4),
        ]) {
            return Err(no(format!(
                "stset with the option {o} is not translated yet"
            )));
        }
        if c.parts.weight.is_some() || c.parts.if_exp.is_some() || c.parts.in_range.is_some() {
            return Err(no("stset with if, in or weights is not translated yet"));
        }
        let words: Vec<&str> = c.parts.main.split_whitespace().collect();
        let [time] = words.as_slice() else {
            return Err(no(
                "stset takes one time variable here (stset alone, clear and the rest are not translated yet)",
            ));
        };
        if !is_name(time) {
            return Err(no(format!("{time} is not a variable name")));
        }
        let d = self.data()?;
        let col = |v: &str| format!("{d}${}", r_name(v));
        let mut args = vec![d.clone(), col(time)];
        if let Some(f) = opts.arg("failure", 1) {
            let f = f.trim();
            let (var, values) = match f.split_once("==") {
                Some((v, vals)) => (v.trim(), Some(vals.trim())),
                None => (f, None),
            };
            if !is_name(var) {
                return Err(no(format!("failure({f}): one variable, or var == numbers")));
            }
            args.push(format!("fail = {}", col(var)));
            if let Some(vals) = values {
                let nums: Result<Vec<f64>, _> = vals
                    .split([' ', ','])
                    .filter(|s| !s.is_empty())
                    .map(str::parse::<f64>)
                    .collect();
                let nums =
                    nums.map_err(|_| no(format!("failure({f}): the values must be numbers")))?;
                let nums: Vec<String> = nums.into_iter().map(r_number).collect();
                args.push(format!("values = c({})", nums.join(", ")));
            }
        }
        let id = match opts.arg("id", 2) {
            Some(v) if is_name(v.trim()) => {
                args.push(format!("id = {}", col(v.trim())));
                Some(v.trim().to_string())
            }
            Some(v) => return Err(no(format!("id({v}) needs one variable name"))),
            None => None,
        };
        // origin(), enter() and exit() take `time exp` (or a plain variable or number)
        for (o, m) in [("origin", 1), ("enter", 2), ("exit", 2)] {
            if let Some(a) = opts.arg(o, m) {
                let a = a.trim();
                let a = a.strip_prefix("time ").map(str::trim).unwrap_or(a);
                let v = if a == "." && o == "exit" {
                    "Inf".to_string()
                } else if let Ok(n) = a.parse::<f64>() {
                    r_number(n)
                } else if is_name(a) {
                    col(a)
                } else if a.contains("==") && !a.contains('(') {
                    return Err(no(format!(
                        "stset, {o}({a}): var==value (the time the variable first takes the \
                         value) is not translated yet"
                    )));
                } else {
                    // `time exp`: an expression of the data, row by row
                    let (code, _) = self.value(a, out)?;
                    format!("with({d}, {code})")
                };
                args.push(format!("{o} = {v}"));
            }
        }
        if let Some(s) = opts.arg("scale", 2) {
            let n: f64 = s
                .trim()
                .parse()
                .map_err(|_| no(format!("scale({s}) needs a number")))?;
            args.push(format!("scale = {}", r_number(n)));
        }
        self.helpers.insert("stata_stset");
        let call = format!("stata_stset({})", args.join(", "));
        if Self::quiet(c) {
            out.line(format!("invisible(utils::capture.output({d} <- {call}))"));
        } else {
            out.line(format!("{d} <- {call}"));
        }
        for v in ["_st", "_d", "_t", "_t0"] {
            self.kinds.insert(v.to_string(), Kind::Number);
        }
        self.stset = Some(StSet { id });
        out.caveat("_st, _d, _t and _t0 are made as Stata makes them; a row that enters at or after its end is not used");
        Ok(())
    }

    pub(super) fn stcox(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "stcox")?;
        let st = self
            .stset
            .clone()
            .ok_or_else(|| no("data not st; use stset"))?;
        if let Some(o) = opts.other(&[
            ("breslow", 3),
            ("efron", 3),
            ("exactp", 6),
            ("exactm", 6),
            ("robust", 1),
            ("vce", 3),
            ("nohr", 4),
            ("hr", 2),
            ("level", 3),
            ("strata", 3),
            ("noshow", 4),
        ]) {
            return Err(no(format!(
                "stcox with the option {o} is not translated yet"
            )));
        }
        if opts.has("exactm", 6) {
            return Err(no(
                "stcox, exactm (Stata's exact marginal likelihood) has no counterpart in R's \
                 survival; breslow, efron and exactp are translated",
            ));
        }
        let ties = if opts.has("efron", 3) {
            "efron"
        } else if opts.has("exactp", 6) {
            "exact"
        } else {
            "breslow"
        };
        let (robust, cluster) = vce(opts)?;
        // robust errors with several records per subject cluster on it, as Stata's
        let cluster = cluster.or_else(|| if robust { st.id.clone() } else { None });
        let level = level(opts)?;
        let spec = Spec::from_terms("_t", c.parts.main.split_whitespace())?;
        if spec.terms.is_empty() {
            return Err(no("stcox needs at least one covariate here"));
        }
        self.helpers_for(&spec);
        let mut right = spec.right(false);
        if let Some(s) = opts.arg("strata", 3) {
            let vs: Vec<&str> = s.split_whitespace().collect();
            if vs.is_empty() || vs.iter().any(|v| !is_name(v)) {
                return Err(no(format!(
                    "strata({s}): only variable names are translated"
                )));
            }
            let vs: Vec<String> = vs.iter().map(|v| r_name(v)).collect();
            right.push_str(&format!(" + survival::strata({})", vs.join(", ")));
        }
        let rows = self.rows_for(c, out)?;
        out.package("dplyr");
        let src = format!("dplyr::filter({rows}, `_st` == 1)");
        out.package("survival");
        self.helpers.insert("stata_stcox");
        self.helpers.insert("stata_coefs");
        let cl = match &cluster {
            Some(v) => format!(", cluster = {src}${}", r_name(v)),
            None => String::new(),
        };
        out.line(format!(
            "stata_m <- stata_stcox(survival::Surv(`_t0`, `_t`, `_d`) ~ {right}, data = {src}, ties = \"{ties}\", robust = {}{cl})",
            r_logical(robust || cluster.is_some())
        ));
        out.line("stata_estimates <- stata_m$fit");
        out.line("stata_b <- stata_m$b");
        out.line("stata_vcov <- stata_m$V");
        out.line("stata_df <- Inf");
        out.line(super::model::keep_sample(&src));
        let d = self.data()?;
        out.line(super::model::esample_line(&d, &src));
        self.esample_new = true;
        self.estimates = Some(Estimates {
            fit: Fit::Stcox,
            dep: "_t".into(),
            factors: spec.factors.clone(),
        });
        // e(), as Stata leaves it; its ties are named breslow, efron, exactp
        let vce = if cluster.is_some() {
            "cluster"
        } else if robust {
            "robust"
        } else {
            "oim"
        };
        let args = format!(
            "{}, extra = list(ties = {})",
            super::returned::e_args("_t", vce, cluster.as_deref(), &None),
            r_text(if ties == "exact" { "exactp" } else { ties })
        );
        self.keep_e(c, out, &args, "");
        if !Self::quiet(c) {
            out.line(format!(
                "stata_stcox_header(stata_m, robust = {})",
                r_logical(robust || cluster.is_some())
            ));
            let hr = !opts.has("nohr", 4);
            out.line(format!(
                "stata_coefs(stata_b, stata_vcov, level = {}, eform = {}, label = {})",
                r_number(level),
                r_logical(hr),
                r_text(if hr { "Haz. ratio" } else { "Coefficient" })
            ));
        }
        out.line("rm(stata_m)");
        out.caveat(match ties {
            "efron" => "Efron's method for tied times, as asked",
            "exact" => "the exact partial likelihood for tied times (Stata's exactp, R's exact)",
            _ => "Breslow's method for tied times, Stata's default (R's own default is Efron's)",
        });
        if robust || cluster.is_some() {
            out.caveat("robust errors carry N / (N - 1), clustered ones G / (G - 1), as Stata's");
        }
        out.caveat("printed in R's layout, not Stata's table");
        Ok(())
    }
}
