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
pub(super) const CLOGIT_HELPER: &str = r#"stata_clogit <- function(formula, data, group, robust = FALSE, cluster = NULL, notes = TRUE, weights = NULL, psu = NULL) {
  tt <- stats::terms(formula)
  mf <- stats::model.frame(formula, data, na.action = stats::na.pass)
  ok <- stats::complete.cases(mf) & !is.na(data[[group]])
  for (v in c(weights, psu)) ok <- ok & !is.na(data[[v]])
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
  ll_at <- fit$loglik[2]
  # Stata reports where its Newton-Raphson stops, which can be short of the
  # maximum by a few parts in 1e7: with one positive outcome a group, its path
  # is followed, from an ordinary logit's slopes (StatCraft's decoding of
  # clogit's start, probe 243; the steps and stopping rule are Stata's ml's)
  if (all(tapply(y, gi, sum) == 1)) {
    lg <- suppressWarnings(stats::glm.fit(cbind(1, Xk), y, family = stats::binomial(),
                                          control = stats::glm.control(epsilon = 1e-14, maxit = 100)))
    b0 <- stats::coef(lg)[-1]
    if (all(is.finite(b0))) {
      cll <- function(bb) {
        eta <- drop(Xk %*% bb)
        m <- as.numeric(tapply(eta, gi, max))[gi]
        sum(y * eta) - sum(log(as.numeric(tapply(exp(eta - m), gi, sum)))) - sum(as.numeric(tapply(m, gi, function(v) v[1])))
      }
      cder <- function(bb) {
        eta <- drop(Xk %*% bb)
        w <- exp(eta - as.numeric(tapply(eta, gi, max))[gi])
        p <- w / as.numeric(tapply(w, gi, sum))[gi]
        px <- rowsum(p * Xk, gi)
        list(ll = cll(bb), g = colSums((y - p) * Xk),
             nh = crossprod(Xk * p, Xk) - crossprod(px))
      }
      path <- stata_ml_path(cll, cder, b0)
      if (path$converged) {
        b <- stats::setNames(path$b, names(b))
        V <- solve(cder(b)$nh)
        dimnames(V) <- list(names(b), names(b))
        ll_at <- cll(b)
      }
    }
  }
  S <- NULL
  cl <- NULL
  G <- NA
  wrow <- NULL
  N_pop <- NA
  df_r <- NA
  # svy: sampling weights (one a group) and the design's PSUs: the weighted
  # likelihood on Stata's path from a weighted logit's slopes, and its
  # linearized variance, G / (G - 1) times the PSUs' score sums
  if (!is.null(weights)) {
    wrow <- as.numeric(d[[weights]][!out])
    if (anyNA(wrow) || any(wrow < 0)) stop("svy: sampling weights missing or below 0 are not translated", call. = FALSE)
    if (any(tapply(wrow, gi, function(v) length(unique(v))) > 1)) stop("weights must be the same within each group (Stata: r(407))", call. = FALSE)
    if (!all(tapply(y, gi, sum) == 1)) stop("svy: clogit with a group of more than one positive outcome is not translated yet", call. = FALSE)
    wg <- as.numeric(tapply(wrow, gi, function(v) v[1]))
    lg <- suppressWarnings(stats::glm.fit(cbind(1, Xk), y, weights = wrow, family = stats::binomial(),
                                          control = stats::glm.control(epsilon = 1e-14, maxit = 100)))
    wll <- function(bb) {
      eta <- drop(Xk %*% bb)
      m <- as.numeric(tapply(eta, gi, max))
      lse <- m + log(as.numeric(tapply(exp(eta - m[gi]), gi, sum)))
      sum(wrow * y * eta) - sum(wg * lse)
    }
    wder <- function(bb) {
      eta <- drop(Xk %*% bb)
      e1 <- exp(eta - as.numeric(tapply(eta, gi, max))[gi])
      p <- e1 / as.numeric(tapply(e1, gi, sum))[gi]
      px <- rowsum(p * Xk, gi)
      list(ll = wll(bb), g = colSums(wrow * (y - p) * Xk),
           nh = crossprod(Xk * (wrow * p), Xk) - crossprod(px * sqrt(wg)), p = p)
    }
    path <- stata_ml_path(wll, wder, stats::coef(lg)[-1])
    if (!path$converged) stop("svy: clogit did not converge as Stata's would", call. = FALSE)
    b <- stats::setNames(path$b, names(b))
    dd <- wder(b)
    V <- solve(dd$nh)
    dimnames(V) <- list(names(b), names(b))
    ll_at <- wll(b)
    S <- wrow * (y - dd$p) * Xk
    cl <- if (is.null(psu)) seq_along(y) else d[[psu]][!out]
    if (anyNA(cl)) stop("svy: missing values in the PSU variable are not translated", call. = FALSE)
    Vinfo <- V
    U <- rowsum(S, cl)
    G <- nrow(U)
    V <- V %*% crossprod(U) %*% V * G / (G - 1)
    N_pop <- sum(wg)
    df_r <- G - 1
  } else if (robust || !is.null(cluster)) {
    Vinfo <- V
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
  } else {
    Vinfo <- V
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
  # for margins: the model as written, pu0 = invlogit(xb), and each row's
  # influence on b for vce(unconditional), clustered as the model was
  fit$stata_mfit <- structure(list(terms = stats::delete.response(tt), coefficients = stats::setNames(b, colnames(X)[keep]),
                                    xlevels = stats::.getXlevels(tt, mf), contrasts = attr(X, "contrasts")),
                               class = "stata_clogit_mfit")
  if (!is.null(S)) {
    colnames(S) <- colnames(X)[keep]
    fit$stata_influence <- S %*% Vinfo
    colnames(fit$stata_influence) <- colnames(X)[keep]
    fit$stata_cluster <- cl
  }
  fit$stata_weights <- wrow
  # for suest: the rows (the data's row names), the design, the outcome, the
  # groups, the coefficients and the model's own variance
  fit$stata_suest <- list(rows = rn, X = Xk, y = y, gi = gi, b = b, Vinfo = Vinfo, cols = colnames(X)[keep])
  fit$stata_written <-vapply(attr(tt, "term.labels"), function(t) paste(sort(trimws(strsplit(t, ":", fixed = TRUE)[[1]])), collapse = ":"), "")
  list(fit = fit, b = bf, V = Vf, N = length(y), G = if (is.null(cluster)) NA else G,
       ll = ll_at, ll_0 = fit$loglik[1], N_drop = sum(out), N_group_drop = length(bad),
       multiple = multiple, group = group, N_pop = N_pop, df_r = df_r)
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
        self.helpers.insert("stata_probit_newton");
        self.helpers.insert("stata_coefs");
        let quiet = self.quiet(c);
        // under svy: the design's weights and PSUs, and no vce() of its own
        let svy = self.svy_call.clone();
        if svy.is_some() && (robust || cluster.is_some()) {
            return Err(no("svy: clogit with vce() is not allowed (Stata: r(198))"));
        }
        let design = match &svy {
            Some(((psu, w), _, _)) => {
                self.helpers.insert("stata_svy");
                let p = if psu.is_empty() {
                    String::new()
                } else {
                    format!(", psu = {}", r_text(psu))
                };
                format!(", weights = {}{p}", r_text(w))
            }
            None => String::new(),
        };
        out.line(format!(
            "stata_m <- stata_clogit({} ~ {right}, data = {src}, group = {}, robust = {}{}{}{design})",
            r_name(&spec.dep),
            r_text(&group),
            r_logical(robust || cluster.is_some()),
            cluster
                .as_deref()
                .map(|v| format!(", cluster = {}", r_text(v)))
                .unwrap_or_default(),
            if quiet || svy.is_some() { ", notes = FALSE" } else { "" }
        ));
        out.line("stata_estimates <- stata_m$fit");
        out.line("stata_b <- stata_m$b");
        out.line("stata_vcov <- stata_m$V");
        out.line(if svy.is_some() {
            "stata_df <- stata_m$df_r"
        } else {
            "stata_df <- Inf"
        });
        out.line(keep_sample(&src));
        let d = self.data()?;
        out.package("dplyr");
        out.line(esample_line(&d, &src));
        self.esample_new = true;
        self.fit_robust = robust || cluster.is_some() || svy.is_some();
        self.fit_cluster = cluster.is_some();
        self.svy_fit = svy.is_some();
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
        let vce = if svy.is_some() { "linearized" } else { vce };
        let mut args = super::returned::e_args(&spec.dep, vce, cluster.as_deref(), &None);
        if let Some(((psu, w), line, command)) = &svy {
            args.push_str(&format!(
                ", extra = list(svy = TRUE, psu = {}, wvar = {}, svy_cmdline = {}, command = {})",
                r_text(psu),
                r_text(w),
                r_text(line),
                r_text(command)
            ));
        }
        self.keep_e(c, out, &args, "");
        if !quiet && svy.is_some() {
            out.line(
                "stata_svy_header(\"Survey: Conditional (fixed-effects) logistic regression\", stata_m)",
            );
            out.line(format!(
                "stata_coefs(stata_b, stata_vcov, stata_df, level = {}, label = \"Coefficient\")",
                r_number(level)
            ));
        } else if !quiet {
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

/// `asclogit` with alternative-specific variables only and `noconstant`:
/// clogit's fit on the cases (Stata 19's estimates and variance are
/// clogit's, to the last digit), with asclogit's notes, header, e() and
/// `estat summarize` (`plan/stata/asclogit.md`).
pub(super) const ASCLOGIT_HELPER: &str = r#"stata_asclogit_check <- function(formula, data, case, alts, vars, cluster = NULL, notes = TRUE) {
  ok <- stats::complete.cases(stats::model.frame(formula, data, na.action = stats::na.pass)) &
    !is.na(data[[case]]) & !is.na(data[[alts]])
  if (!is.null(cluster)) ok <- ok & !is.na(data[[cluster]])
  d <- data[ok, , drop = FALSE]
  y <- as.numeric(d[[all.vars(formula)[1]]] != 0)
  pos <- tapply(y, d[[case]], sum)
  if (any(pos != 1)) stop("asclogit with cases that do not choose exactly one alternative is not translated yet", call. = FALSE)
  # a variable that does not vary within a case, case by case (Stata's notes)
  if (notes) for (v in vars) {
    n <- sum(tapply(d[[v]], d[[case]], function(x) length(unique(x)) == 1))
    if (n > 0) cat("note: variable ", v, " has ", n, " cases that are not alternative-specific; there is no within-case variability.\n", sep = "")
  }
  sz <- as.numeric(table(d[[case]]))
  list(rows = which(ok), alt_min = min(sz), alt_avg = mean(sz), alt_max = max(sz),
       N_case = length(sz), alts = sort(unique(d[[alts]])))
}
stata_asclogit_header <- function(m, a, case, alts, clustvar = NULL) {
  e <- stata_e
  row <- function(left, right) cat(formatC(left, width = -47, flag = "-"), right, "\n", sep = "")
  row("Alternative-specific conditional logit", sprintf("Number of obs      = %10s", format(e$N, big.mark = ",")))
  row(paste("Case ID variable:", case), sprintf("Number of cases    = %10s", format(e$N_case, big.mark = ",")))
  row(paste("Alternatives variable:", alts), sprintf("Alts per case: min = %10d", as.integer(e$alt_min)))
  row("", sprintf("               avg = %10.1f", e$alt_avg))
  row("", sprintf("               max = %10d", as.integer(e$alt_max)))
  cat(formatC("", width = 50), sprintf("Wald chi2(%d)    = %10.2f\n", as.integer(e$df_m), e$chi2), sep = "")
  lab <- if (is.null(clustvar)) "Log likelihood" else "Log pseudolikelihood"
  row(sprintf("%s = %.8g", lab, e$ll), sprintf("   Prob > chi2     = %10.4f", e$p))
  if (!is.null(clustvar)) cat(formatC(sprintf("(Std. err. adjusted for %d clusters in %s)", as.integer(e$N_clust), clustvar), width = 78), "\n", sep = "")
  cat("\n")
  invisible(NULL)
}
stata_asclogit_summarize <- function(data, depvar, vars, eq, clustvar = NULL) {
  d <- data[stata_esample, , drop = FALSE]
  cat("Estimation sample asclogit", formatC("", width = 19), sprintf("Number of obs        = %10s\n\n", format(nrow(d), big.mark = ",")), sep = "")
  if (!is.null(clustvar)) {
    sz <- as.numeric(table(d[[clustvar]]))
    cat(formatC("", width = 45), sprintf("Number of clusters   = %10d\n", length(sz)), sep = "")
    cat(formatC("", width = 45), sprintf("Obs per cluster: min = %10d\n", as.integer(min(sz))), sep = "")
    cat(formatC("", width = 45), sprintf("                 avg = %10.1f\n", mean(sz)), sep = "")
    cat(formatC("", width = 45), sprintf("                 max = %10d\n\n", as.integer(max(sz))), sep = "")
  }
  line <- function(ch) cat(strrep("-", 13), ch, strrep("-", 64), "\n", sep = "")
  # Stata 19: the mean right in 10 after the bar, the std. dev. in 11, min and
  # max in 9, each %9.0g; the label after three spaces when there is one
  num <- function(x, w) formatC(stata_fmt(x, "%9.0g", trim = TRUE), width = w)
  line("-")
  cat("    Variable |      Mean   Std. dev.     Min      Max   Label\n")
  one <- function(v) {
    x <- as.numeric(d[[v]])
    l <- attr(d[[v]], "label", exact = TRUE)
    lab <- if (is.null(l) || !nzchar(l)) "" else paste0("   ", l)
    cat(formatC(v, width = 12), " |", num(mean(x), 10), num(stats::sd(x), 11), num(min(x), 9), num(max(x), 9), lab, "\n", sep = "")
  }
  line("+")
  cat(formatC("depvar", width = -12, flag = "-"), " |\n", sep = "")
  one(depvar)
  line("+")
  cat(formatC(eq, width = -12, flag = "-"), " |\n", sep = "")
  for (v in vars) one(v)
  line("-")
  if (!is.null(clustvar)) cat("Std. dev. not adjusted for clustering\n")
  invisible(NULL)
}"#;

impl Session {
    /// `asclogit depvar indepvars [if] [in], case(v) alternatives(v)
    /// noconstant [vce(cluster v) | vce(robust) | level(#) nolog]`.
    pub(super) fn asclogit(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "asclogit")?;
        if let Some(o) = opts.other(&[
            ("case", 4),
            ("alternatives", 3),
            ("noconstant", 5),
            ("robust", 1),
            ("vce", 3),
            ("level", 3),
            ("nolog", 5),
            ("log", 3),
        ]) {
            return Err(no(format!(
                "asclogit with the option {o} is not translated yet"
            )));
        }
        if c.parts.weight.is_some() {
            return Err(no("asclogit with weights is not translated yet"));
        }
        if !opts.has("noconstant", 5) {
            return Err(no(
                "asclogit with the alternatives' constants is not translated yet (noconstant is)",
            ));
        }
        let name_of = |o: &str, min: usize| -> Result<String, Untranslated> {
            let v = opts
                .arg(o, min)
                .map(|v| v.trim().to_string())
                .ok_or_else(|| no(format!("asclogit needs {o}() (Stata: r(198))")))?;
            if is_name(&v) {
                Ok(v)
            } else {
                Err(no(format!(
                    "asclogit, {o}({v}): one variable name is translated"
                )))
            }
        };
        let case = name_of("case", 4)?;
        let alts = name_of("alternatives", 3)?;
        let level = level(opts)?;
        let (robust, cluster) = vce(opts)?;
        let spec = Spec::read(c, "asclogit")?;
        if spec.terms.is_empty() || !spec.factors.is_empty() {
            return Err(no(
                "asclogit with factor variables or no variables is not translated yet",
            ));
        }
        let vars: Vec<String> = spec.terms.clone();
        if vars.iter().any(|v| !is_name(v)) {
            return Err(no(
                "asclogit with interactions or operators is not translated yet",
            ));
        }
        let right = spec.right(false)?;
        let src = self.rows_for(c, out)?;
        out.package("survival");
        self.helpers.insert("stata_clogit");
        self.helpers.insert("stata_probit_newton");
        self.helpers.insert("stata_asclogit");
        self.helpers.insert("stata_coefs");
        let quiet = self.quiet(c);
        let vlist = format!(
            "c({})",
            vars.iter()
                .map(|v| r_text(v))
                .collect::<Vec<_>>()
                .join(", ")
        );
        let cl = cluster
            .as_deref()
            .map(|v| format!(", cluster = {}", r_text(v)))
            .unwrap_or_default();
        out.line(format!(
            "stata_a <- stata_asclogit_check({} ~ {right}, data = {src}, case = {}, alts = {}, vars = {vlist}{cl}, notes = {})",
            r_name(&spec.dep),
            r_text(&case),
            r_text(&alts),
            r_logical(!quiet)
        ));
        out.line(format!(
            "stata_m <- stata_clogit({} ~ {right}, data = {src}, group = {}, robust = {}{cl}, notes = FALSE)",
            r_name(&spec.dep),
            r_text(&case),
            r_logical(robust || cluster.is_some()),
        ));
        out.line("stata_estimates <- stata_m$fit");
        out.line(format!("stata_estimates$stata_altvar <- {}", r_text(&alts)));
        out.line("stata_estimates$stata_asc <- stata_a");
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
            fit: Fit::Asclogit,
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
        let mut args = super::returned::e_args(&spec.dep, vce, cluster.as_deref(), &None);
        args.push_str(&format!(
            ", extra = list(case = {}, altvar = {}, indvars = {})",
            r_text(&case),
            r_text(&alts),
            r_text(&vars.join(" "))
        ));
        self.keep_e(c, out, &args, "");
        self.asclogit_vars = Some((vars.clone(), alts.clone(), cluster.clone()));
        if !quiet {
            out.line(format!(
                "stata_asclogit_header(stata_m, stata_a, {}, {}{})",
                r_text(&case),
                r_text(&alts),
                cluster
                    .as_deref()
                    .map(|v| format!(", clustvar = {}", r_text(v)))
                    .unwrap_or_default()
            ));
            out.line(format!(
                "stata_coefs(stata_b, stata_vcov, stata_df, level = {}, label = \"Coefficient\")",
                r_number(level)
            ));
        }
        out.line("rm(stata_m, stata_a)");
        out.caveat(
            "asclogit with alternative-specific variables only and noconstant is clogit on the \
             cases: the same estimates and variance, as in Stata 19",
        );
        out.caveat("printed in R's layout, not Stata's table");
        Ok(())
    }

    /// `estat summarize` after asclogit: the estimation sample's means.
    pub(super) fn asclogit_summarize(&mut self, out: &mut Out) -> Result<(), Untranslated> {
        let Some((vars, alts, cluster)) = self.asclogit_vars.clone() else {
            return Err(no(
                "estat summarize: the asclogit before it is not known here",
            ));
        };
        let est = self.estimates_for("estat")?;
        let d = self.data()?;
        self.helpers.insert("stata_asclogit");
        self.helpers.insert("stata_fmt");
        out.line(format!(
            "stata_asclogit_summarize({d}, {}, c({}), {}{})",
            r_text(&est.dep),
            vars.iter()
                .map(|v| r_text(v))
                .collect::<Vec<_>>()
                .join(", "),
            r_text(&alts),
            cluster
                .as_deref()
                .map(|v| format!(", clustvar = {}", r_text(v)))
                .unwrap_or_default()
        ));
        Ok(())
    }
}
