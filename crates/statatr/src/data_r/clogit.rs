//! P2C.9 continued, `clogit`: conditional (fixed-effects) logistic
//! regression, as Stata 19 does it (observed with made-up data,
//! `plan/stata/clogit.md`).
//!
//! - The outcome is positive where it is not 0. A group whose rows are all
//!   positive or all negative tells nothing and is dropped with Stata's
//!   note; so is a variable that does not vary within any group.
//! - The likelihood is the exact conditional one, also with several
//!   positives in a group: R's `survival::coxph` with one stratum per group
//!   and `method = "exact"` maximises the same function.
//! - `coxph` gives no robust variance with that method, so the scores are
//!   worked out here: each row's x times its outcome less its probability of
//!   being among the group's positives. `vce(robust)` clusters on the group,
//!   and both it and `vce(cluster v)` carry G / (G - 1); the groups must be
//!   nested in the clusters (r(459)).
//! - `predict` gives pc1 (the probability of being the positive one, within
//!   the group, on every row), `pu0` (invlogit of xb: the fixed effect taken as zero) or `xb`.

use super::model::{esample_line, keep_sample, level, vce, Estimates, Fit, Spec};
use super::stats::r_logical;
use super::*;

/// The fit, its variance and what Stata prints before the table.
pub(super) const CLOGIT_HELPER: &str = r#"stata_clogit <- function(formula, data, group, robust = FALSE, cluster = NULL, notes = TRUE) {
  tt <- stats::terms(formula)
  mf <- stats::model.frame(formula, data, na.action = stats::na.pass)
  ok <- stats::complete.cases(mf) & !is.na(data[[group]])
  if (!is.null(cluster)) ok <- ok & !is.na(data[[cluster]])
  rn <- rownames(data)[ok]
  d <- data[ok, , drop = FALSE]
  mf <- stats::model.frame(formula, d, na.action = stats::na.pass)
  y <- as.numeric(stats::model.response(mf) != 0)
  g <- d[[group]]
  pos <- tapply(y, g, sum)
  size <- tapply(y, g, length)
  bad <- names(pos)[pos == 0 | pos == size]
  out <- as.character(g) %in% bad
  multiple <- any(pos > 1 & pos < size)
  if (notes && multiple) cat("note: multiple positive outcomes within groups encountered.\n")
  if (notes && length(bad) > 0) {
    cat(sprintf("note: %d group%s (%d obs) omitted because of all positive or\n      all negative outcomes.\n",
                length(bad), if (length(bad) == 1) "" else "s", sum(out)))
  }
  rn <- rn[!out]
  d <- d[!out, , drop = FALSE]
  mf <- mf[!out, , drop = FALSE]
  y <- y[!out]
  g <- g[!out]
  if (length(y) == 0) stop("no observations")
  X <- stats::model.matrix(tt, mf)
  X <- X[, colnames(X) != "(Intercept)", drop = FALSE]
  gi <- as.integer(factor(g))
  Wd <- X - (rowsum(X, gi) / tabulate(gi))[gi, , drop = FALSE]
  shown <- sub("^factor\\((.+)\\)(.+)$", "\\2.\\1", colnames(X))
  flat <- colSums(Wd^2) <= 1e-12 * pmax(colSums(X^2), 1)
  keep <- !flat
  if (any(keep)) {
    q <- qr(Wd[, keep, drop = FALSE], tol = 1e-9)
    ind <- which(keep)
    if (q$rank < length(ind)) keep[ind[q$pivot[-seq_len(q$rank)]]] <- FALSE
  }
  if (notes) {
    for (j in which(flat)) cat("note: ", shown[j], " omitted because of no within-group variance.\n", sep = "")
    for (j in which(!flat & !keep)) cat("note: ", shown[j], " omitted because of collinearity.\n", sep = "")
  }
  if (!any(keep)) stop("no covariates left to fit (Stata leaves a model with none)")
  Xk <- X[, keep, drop = FALSE]
  fd <- as.data.frame(Xk)
  xs <- paste0("stata_x", seq_len(ncol(Xk)))
  names(fd) <- xs
  fd$stata_one <- 1
  fd$stata_y <- y
  fd$stata_g <- gi
  rownames(fd) <- rn
  f2 <- stats::as.formula(paste("survival::Surv(stata_one, stata_y) ~", paste(xs, collapse = " + "),
                                "+ survival::strata(stata_g)"))
  ctl <- survival::coxph.control(eps = 1e-12, toler.chol = 1e-15, iter.max = 100)
  fit <- survival::coxph(f2, data = fd, method = "exact", control = ctl, model = TRUE)
  b <- stats::coef(fit)
  V <- stats::vcov(fit)
  G <- NA
  if (robust || !is.null(cluster)) {
    # each row's score: its x times its outcome less its chance of being among
    # the group's positives (exact: from the elementary symmetric sums)
    esym <- function(w, k) {
      a <- c(1, numeric(k))
      if (k > 0) for (v in w) a[2:(k + 1)] <- a[2:(k + 1)] + v * a[1:k]
      a[k + 1]
    }
    eta <- drop(Xk %*% b)
    S <- matrix(0, nrow(Xk), ncol(Xk))
    for (h in split(seq_along(gi), gi)) {
      w <- exp(eta[h] - max(eta[h]))
      k <- sum(y[h])
      all <- esym(w, k)
      p <- vapply(seq_along(h), function(j) w[j] * esym(w[-j], k - 1) / all, 0)
      S[h, ] <- (y[h] - p) * Xk[h, , drop = FALSE]
    }
    cl <- if (is.null(cluster)) gi else d[[cluster]]
    if (any(tapply(cl, gi, function(v) length(unique(v))) > 1)) stop("groups (strata) are not nested within clusters")
    U <- rowsum(S, cl)
    G <- nrow(U)
    V <- V %*% crossprod(U) %*% V * G / (G - 1)
  }
  bf <- stats::setNames(rep(NA_real_, ncol(X)), colnames(X))
  bf[keep] <- b
  Vf <- matrix(0, ncol(X), ncol(X), dimnames = list(colnames(X), colnames(X)))
  Vf[keep, keep] <- V
  fit$stata_terms <- stats::delete.response(tt)
  fit$stata_xlevels <- stats::.getXlevels(tt, mf)
  fit$xlevels <- fit$stata_xlevels
  fit$stata_b <- stats::setNames(b, colnames(X)[keep])
  fit$stata_group <- group
  # for stata_eb: the terms in the order written (the omitted ones are NA in b)
  fit$stata_written <- vapply(attr(tt, "term.labels"), function(t) paste(sort(trimws(strsplit(t, ":", fixed = TRUE)[[1]])), collapse = ":"), "")
  list(fit = fit, b = bf, V = Vf, N = length(y), G = if (is.null(cluster)) NA else G,
       ll = fit$loglik[2], ll_0 = fit$loglik[1], N_drop = sum(out), N_group_drop = length(bad),
       multiple = multiple, group = group)
}
stata_clogit_header <- function(m, robust, clustvar = NULL, group = NULL) {
  k <- sum(!is.na(m$b))
  pad <- function(s) formatC(s, width = -56, flag = "-")
  cat(pad("Conditional (fixed-effects) logistic regression"),
      sprintf("Number of obs = %6s\n", format(m$N, big.mark = ",")), sep = "")
  if (robust) {
    keep <- !is.na(m$b)
    bk <- m$b[keep]
    chi2 <- drop(t(bk) %*% solve(m$V[keep, keep, drop = FALSE], bk))
    test <- sprintf("Wald chi2(%d)", k)
  } else {
    chi2 <- 2 * (m$ll - m$ll_0)
    test <- sprintf("LR chi2(%d)", k)
  }
  cat(pad(""), sprintf("%-13s = %6.2f\n", test, chi2), sep = "")
  cat(pad(""), sprintf("Prob > chi2   = %6.4f\n", stats::pchisq(chi2, k, lower.tail = FALSE)), sep = "")
  cat(pad(sprintf("Log %s = %.8g", if (robust) "pseudolikelihood" else "likelihood", m$ll)),
      sprintf("Pseudo R2     = %6.4f\n", 1 - m$ll / m$ll_0), sep = "")
  if (!is.null(clustvar)) {
    cat("\n", formatC(sprintf("(Std. err. adjusted for %s clusters in %s)", format(m$G, big.mark = ","), clustvar), width = 78), "\n", sep = "")
  } else if (robust) {
    cat("\n", formatC(sprintf("(Std. err. adjusted for clustering on %s)", group), width = 78), "\n", sep = "")
  }
  cat("\n")
  invisible(NULL)
}
stata_clogit_xb <- function(fit, newdata) {
  mf <- stats::model.frame(fit$stata_terms, newdata, xlev = fit$stata_xlevels, na.action = stats::na.pass)
  X <- stats::model.matrix(fit$stata_terms, mf)
  drop(X[, names(fit$stata_b), drop = FALSE] %*% fit$stata_b)
}
stata_clogit_pc1 <- function(fit, newdata) {
  e <- exp(stata_clogit_xb(fit, newdata))
  g <- newdata[[fit$stata_group]]
  e[is.na(g)] <- NA
  tot <- stats::ave(ifelse(is.na(e), 0, e), ifelse(is.na(g), -Inf, g), FUN = sum)
  e / tot
}"#;

impl Session {
    pub(super) fn clogit(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "clogit")?;
        if let Some(o) = opts.other(&[
            ("group", 2),
            ("strata", 2),
            ("robust", 1),
            ("vce", 3),
            ("or", 2),
            ("level", 3),
            ("baselevels", 5),
            ("allbaselevels", 4),
            ("nolog", 5),
            ("log", 3),
        ]) {
            return Err(no(format!(
                "clogit with the option {o} is not translated yet"
            )));
        }
        if c.parts.weight.is_some() {
            return Err(no("clogit with weights is not translated yet"));
        }
        let group = opts
            .arg("group", 2)
            .or_else(|| opts.arg("strata", 2))
            .map(|g| g.trim().to_string())
            .ok_or_else(|| no("group() required (Stata: r(198))"))?;
        if !is_name(&group) {
            return Err(no(format!(
                "group({group}): only a variable name is translated"
            )));
        }
        let level = level(opts)?;
        let (robust, cluster) = vce(opts)?;
        let spec = Spec::read(c, "clogit")?;
        if spec.terms.is_empty() {
            return Err(no("clogit needs at least one covariate here"));
        }
        self.helpers_for(&spec);
        let right = spec.right(false)?;
        let src = self.rows_for(c, out)?;
        out.package("survival");
        self.helpers.insert("stata_clogit");
        self.helpers.insert("stata_coefs");
        let quiet = self.quiet(c);
        out.line(format!(
            "stata_m <- stata_clogit({} ~ {right}, data = {src}, group = {}, robust = {}{}{})",
            r_name(&spec.dep),
            r_text(&group),
            r_logical(robust || cluster.is_some()),
            cluster
                .as_deref()
                .map(|v| format!(", cluster = {}", r_text(v)))
                .unwrap_or_default(),
            if quiet { ", notes = FALSE" } else { "" }
        ));
        out.line("stata_estimates <- stata_m$fit");
        out.line("stata_b <- stata_m$b");
        out.line("stata_vcov <- stata_m$V");
        out.line("stata_df <- Inf");
        out.line(keep_sample(&src));
        let d = self.data()?;
        out.package("dplyr");
        out.line(esample_line(&d, &src));
        self.esample_new = true;
        self.fit_robust = robust || cluster.is_some();
        self.fit_cluster = cluster.is_some();
        self.estimates = Some(Estimates {
            fit: Fit::Clogit,
            dep: spec.dep.clone(),
            factors: spec.factors.clone(),
        });
        let vce = if cluster.is_some() {
            "cluster"
        } else if robust {
            "robust"
        } else {
            "oim"
        };
        let args = super::returned::e_args(&spec.dep, vce, cluster.as_deref(), &None);
        self.keep_e(c, out, &args, "");
        if !quiet {
            out.line(format!(
                "stata_clogit_header(stata_m, robust = {}{}, group = {})",
                r_logical(robust || cluster.is_some()),
                cluster
                    .as_deref()
                    .map(|v| format!(", clustvar = {}", r_text(v)))
                    .unwrap_or_default(),
                r_text(&group)
            ));
            let or = opts.has("or", 2);
            out.line(format!(
                "stata_coefs(stata_b, stata_vcov, stata_df, level = {}, eform = {}, label = {})",
                r_number(level),
                r_logical(or),
                r_text(if or { "Odds ratio" } else { "Coef." })
            ));
        }
        out.line("rm(stata_m)");
        out.caveat(
            "the exact conditional likelihood, as Stata's, fitted by R's survival::coxph with a \
             stratum per group",
        );
        if robust || cluster.is_some() {
            out.caveat(
                "robust errors cluster on the group, as Stata's do for clogit; with clusters the \
                 groups must be nested in them; both carry G / (G - 1)",
            );
        }
        out.caveat("printed in R's layout, not Stata's table");
        Ok(())
    }
}
