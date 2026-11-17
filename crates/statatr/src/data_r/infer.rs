//! The lasso inference commands `dsregress`, `poregress`, `xporegress` and
//! `dslogit`, as Stata 19 runs them.
//!
//! The rules are StatCraft's decoding (our own project, its
//! `plan/stata/lasso.md`, probes 475 and 480), checked again here on those
//! probes' Stata 19 logs (`plan/stata/lasso.md` in this repository): each
//! variable's lasso is the lasso RCraft already runs as Stata does (the outcome
//! and each variable of interest on the controls; for `dslogit` the outcome by
//! a logit lasso on the variables of interest and the controls); double
//! selection fits the outcome on the variables of interest and the union of
//! the selected controls, partialing out regresses the outcome's residual on
//! the variables' residuals, cross-fit does that fold by fold (one uniform per
//! observation, rank r to fold r mod K) and averages over resamples.

use super::returned::Returned;
use super::*;

pub(super) const INFER_HELPER: &str = r###"# least squares of y on the columns of X (a constant last when cons) and the
# variance Stata's inference commands give: robust (HC1), hc0, hc2, hc3, ols, cluster
stata_infer_ols <- function(y, X, vce = "robust", cl = NULL, cons = TRUE) {
  if (cons) X <- cbind(X, 1)
  n <- length(y); m <- ncol(X)
  inv <- tryCatch(solve(crossprod(X)), error = function(e) NULL)
  if (is.null(inv)) stata_lasso_stop("collinear variables", 459)
  b <- as.numeric(inv %*% crossprod(X, y)); e <- as.numeric(y - X %*% b)
  if (vce == "ols") return(list(b = b, V = inv * sum(e^2) / (n - m), e = e))
  if (vce == "cluster") {
    g <- split(seq_len(n), cl); G <- length(g)
    S <- t(vapply(g, function(i) colSums(X[i, , drop = FALSE] * e[i]), numeric(m)))
    if (m == 1) S <- matrix(S, ncol = 1)
    mid <- crossprod(S) * G / (G - 1) * (n - 1) / (n - m)
  } else {
    h <- rowSums((X %*% inv) * X)
    w <- switch(vce, hc2 = 1 / (1 - h), hc3 = 1 / (1 - h)^2, hc0 = rep(1, n), rep(n / (n - m), n))
    mid <- crossprod(X * (w * e^2), X)
  }
  list(b = b, V = inv %*% mid %*% inv, e = e)
}
stata_infer_resid <- function(y, C, sel) {
  X <- C[, sel, drop = FALSE]
  if (!ncol(X)) return(y - mean(y))
  as.numeric(stats::lm.fit(cbind(X, 1), y)$residuals)
}
# the logit model of y on X with a constant (Newton), its robust (n / (n - 1)),
# cluster or model-based variance
stata_infer_logit <- function(y, X, vce = "robust", cl = NULL) {
  X <- cbind(X, 1); n <- length(y); k <- ncol(X)
  b <- numeric(k)
  ll <- function(b) { eta <- as.numeric(X %*% b); sum(y * eta - ifelse(eta > 0, eta + log1p(exp(-eta)), log1p(exp(eta)))) }
  cur <- ll(b)
  for (it in 1:200) {
    mu <- stats::plogis(as.numeric(X %*% b)); w <- mu * (1 - mu)
    H <- crossprod(X * w, X); g <- as.numeric(crossprod(X, y - mu))
    st <- tryCatch(solve(H, g), error = function(e) NULL)
    if (is.null(st)) stata_lasso_stop("collinear variables", 459)
    t <- 1; ok <- FALSE
    for (h in 1:40) { nb <- b + t * st; l2 <- ll(nb); if (is.finite(l2) && l2 >= cur - 1e-12) { ok <- TRUE; break }; t <- t / 2 }
    if (!ok) break
    d <- max(abs(nb - b)); b <- nb; cur <- l2
    if (d < 1e-13) break
  }
  mu <- stats::plogis(as.numeric(X %*% b)); w <- mu * (1 - mu)
  A <- solve(crossprod(X * w, X))
  if (vce == "ols") return(list(b = b, V = A))
  sc <- X * (y - mu)
  if (vce == "cluster") { g <- split(seq_len(n), cl); S <- t(vapply(g, function(i) colSums(sc[i, , drop = FALSE]), numeric(k))); G <- length(g) }
  else { S <- sc; G <- n }
  list(b = b, V = A %*% crossprod(S) %*% A * G / (G - 1))
}
# the folds of a cross-fit: one uniform per observation, rank r to fold r mod K
stata_infer_xfolds <- function(n, k) {
  u <- stata_runiform(n)
  f <- integer(n); f[order(u, seq_len(n))] <- (seq_len(n) - 1) %% k
  f
}
# the cross-fit's robust variance: moment and score matrices averaged over the
# folds with equal weights
stata_infer_xvce <- function(ytil, D, b, fold, K) {
  k <- ncol(D); J <- matrix(0, k, k); O <- matrix(0, k, k)
  e <- as.numeric(ytil - D %*% b)
  for (f in 0:(K - 1)) {
    i <- which(fold == f); nk <- max(length(i), 1)
    Di <- D[i, , drop = FALSE]
    J <- J + crossprod(Di) / nk / K
    O <- O + crossprod(Di * e[i]) / nk / K
  }
  Ji <- solve(J)
  Ji %*% O %*% Ji / length(ytil)
}
# one variable's lasso as the lasso command runs it: the selected controls,
# the lambda and the count of selected variables
stata_infer_lasso <- function(src, v, always, others, args) {
  f <- do.call(stata_lasso_run, c(list(src, nrow(src), v, always, others), args, list(inner = "fit", show = FALSE, rseed = NULL)))
  if (is.na(f$sel)) stata_lasso_stop("lambda not selected", 430)
  r <- f$recs[[f$sel]]
  list(sel = f$vars[r$b != 0], lambda = r$lambda, k = sum(r$b != 0))
}
stata_infer_run <- function(src, nall, cmd, y, dvars, always, others, largs = list(), over = list(), label = "plugin",
                            vce = "robust", cluster = NULL, level = 95, nolog = FALSE, legend = FALSE, coef = FALSE,
                            xfolds = 10, resample = 1, rseed = NULL, cmdline = "", show = TRUE) {
  say <- show && !nolog
  xpo <- startsWith(cmd, "xpo"); po <- xpo || startsWith(cmd, "po")
  model <- if (endsWith(cmd, "logit")) "logit" else "linear"
  ctl <- c(always, others)
  for (v in c(y, dvars, ctl, cluster)) if (!v %in% names(src)) stata_lasso_stop(paste0("variable ", v, " not found"), 111)
  num <- function(v) { u <- unclass(src[[v]]); attributes(u) <- NULL; as.numeric(u) }
  all <- c(y, dvars, ctl)
  M <- matrix(unlist(lapply(all, num)), nrow = nrow(src), dimnames = list(NULL, all))
  used <- stats::complete.cases(M)
  if (!is.null(cluster)) used <- used & !is.na(src[[cluster]])
  n <- sum(used)
  if (n == 0) stata_lasso_stop("no observations", 2000)
  M <- M[used, , drop = FALSE]
  d <- as.data.frame(M); d$stata_row <- seq_len(n)
  yv <- M[, y]; D <- M[, dvars, drop = FALSE]; C <- M[, ctl, drop = FALSE]
  if (model == "logit" && any(yv != 0 & yv != 1)) stata_lasso_stop("outcome must be 0 or 1", 2000)
  cl <- if (is.null(cluster)) NULL else src[[cluster]][used]
  if (!is.null(rseed)) stata_set_seed(rseed)
  rng <- stata_lasso_rngstate()
  argsof <- function(v) { a <- largs; for (o in over) if (v %in% o$vars) a <- utils::modifyList(a, o$args); a }
  labof <- function(v) { l <- label; for (o in over) if (v %in% o$vars) l <- o$label; l }
  sq <- isTRUE(largs$sqrtl)
  est <- function(v) paste0("Estimating ", if (sq) "square-root " else "", "lasso for ", v, " using ", labof(v), "\n")
  # one variable's lasso on the rows given: the outcome of dslogit by a logit
  # lasso on the variables of interest and the controls
  fit1 <- function(dd, v) {
    if (model == "logit" && v == y) {
      a <- argsof(v); a$fam <- "logit"
      f <- stata_infer_lasso(dd, v, always, c(dvars, others), a)
      f$sel <- intersect(f$sel, ctl)
      f
    } else stata_infer_lasso(dd, v, always, others, argsof(v))
  }
  vars <- c(y, dvars); k <- length(dvars)
  infos <- list(); counts <- lapply(vars, function(v) integer(0)); names(counts) <- vars
  union <- ctl %in% always
  if (!xpo) {
    if (say) cat("\n")
    for (v in vars) {
      if (say) cat(est(v))
      infos[[v]] <- fit1(d, v)
      union <- union | ctl %in% infos[[v]]$sel
    }
    if (model == "logit") {
      f <- stata_infer_logit(yv, cbind(D, C[, union, drop = FALSE]), vce, cl)
      b <- f$b[seq_len(k)]; V <- f$V[seq_len(k), seq_len(k), drop = FALSE]
    } else if (po) {
      ytil <- stata_infer_resid(yv, C, ctl %in% infos[[y]]$sel)
      dtil <- sapply(dvars, function(v) stata_infer_resid(D[, v], C, ctl %in% infos[[v]]$sel))
      f <- stata_infer_ols(ytil, matrix(dtil, nrow = n), vce, cl, cons = FALSE)
      b <- f$b; V <- f$V
    } else {
      f <- stata_infer_ols(yv, cbind(D, C[, union, drop = FALSE]), vce, cl)
      b <- f$b[seq_len(k)]; V <- f$V[seq_len(k), seq_len(k), drop = FALSE]
    }
  } else {
    bs <- list(); Vs <- list()
    for (r in seq_len(resample)) {
      fold <- stata_infer_xfolds(n, xfolds)
      til <- matrix(0, n, length(vars), dimnames = list(NULL, vars))
      for (kf in 0:(xfolds - 1)) {
        if (say) {
          cat("\n")
          if (resample > 1) cat("Resample ", r, " of ", resample, " ...\n", sep = "")
          cat("Cross-fit fold ", kf + 1, " of ", xfolds, " ...\n", sep = "")
          if (resample > 1) cat("Estimating lassos: 1.\n")
        }
        tr <- which(fold != kf); te <- which(fold == kf)
        dd <- d[tr, , drop = FALSE]; dd$stata_row <- seq_along(tr)
        for (v in vars) {
          if (say && resample == 1) cat(est(v))
          f <- fit1(dd, v)
          counts[[v]] <- c(counts[[v]], f$k)
          s <- ctl %in% f$sel
          union <- union | s
          Xs <- C[, s, drop = FALSE]
          bt <- stats::lm.fit(cbind(Xs[tr, , drop = FALSE], 1), M[tr, v])$coefficients
          bt[is.na(bt)] <- 0
          til[te, v] <- M[te, v] - as.numeric(cbind(Xs[te, , drop = FALSE], 1) %*% bt)
        }
        # a fold with nobody held out: Stata fits the first fold's lassos, then stops
        if (kf == 0 && any(tabulate(fold + 1, xfolds) == 0)) stata_lasso_stop("no observations", 2000)
      }
      Dt <- til[, dvars, drop = FALSE]
      f <- stata_infer_ols(til[, y], Dt, if (vce == "robust") "hc0" else vce, cl, cons = FALSE)
      if (vce == "robust") f$V <- stata_infer_xvce(til[, y], Dt, f$b, fold, xfolds)
      bs[[r]] <- f$b; Vs[[r]] <- f$V
    }
    b <- Reduce(`+`, bs) / resample
    V <- Reduce(`+`, lapply(seq_len(resample), function(r) Vs[[r]] + tcrossprod(bs[[r]] - b))) / resample
  }
  V <- (V + t(V)) / 2
  chi2 <- as.numeric(t(b) %*% solve(V) %*% b)
  pv <- stats::pchisq(chi2, k, lower.tail = FALSE)
  se <- sqrt(diag(V))
  ksel <- sum(union)
  mname <- if (model == "logit") "logit" else "linear"
  title <- if (xpo) "Cross-fit partialing-out" else if (po) paste("Partialing-out", mname, "model") else paste("Double-selection", mname, "model")
  if (show) {
    cat("\n")
    lw <- if (po) 37 else 38; lab <- if (po) 29 else 28
    right <- function(l, v) paste0(formatC(l, width = -lab), "= ", formatC(v, width = 10))
    lines <- c(right("Number of obs", n), right("Number of controls", length(ctl)), right("Number of selected controls", ksel))
    if (xpo) lines <- c(lines, right("Number of folds in cross-fit", xfolds), right("Number of resamples", resample))
    lines <- c(lines, right(paste0("Wald chi2(", k, ")"), stata_fmt(chi2, "%9.2f", trim = TRUE)), right("Prob > chi2", stata_fmt(pv, "%6.4f", trim = TRUE)))
    for (i in seq_along(lines)) {
      head <- if (i == 1) title else if (i == 2 && xpo) paste(mname, "model") else ""
      cat(formatC(head, width = -lw), lines[i], "\n", sep = "")
    }
    cat("\n")
    if (vce == "cluster") cat(sprintf("%78s\n", paste0("(Std. err. adjusted for ", length(unique(cl)), " clusters in ", cluster, ")")))
    cat(strrep("-", 78), "\n", sep = "")
    vl <- switch(vce, robust = , cluster = "               Robust", hc3 = "             Robust HC3", hc2 = "             Robust HC2", "")
    if (nzchar(vl) && !legend) cat("             |", vl, "\n", sep = "")
    expo <- model != "linear" && !coef
    cl_lab <- if (expo) "Odds ratio" else "Coefficient"
    if (legend) cat(sprintf("%12s | Coefficient  Legend\n", y))
    else cat(sprintf("%12s |%-14s%s      z    P>|z|     [%s%% conf. interval]\n", y, sprintf(" %10s", cl_lab),
                     if (vce == "ols") "Std. err." else "std. err.", stata_fmt(level, "%9.0g", trim = TRUE)))
    cat("-------------+", strrep("-", 64), "\n", sep = "")
    zq <- stats::qnorm(0.5 + level / 200)
    for (i in seq_len(k)) {
      bi <- b[i]; si <- se[i]; zi <- bi / si; pi <- 2 * stats::pnorm(-abs(zi))
      if (legend) { cat(sprintf("%12s |%11s  _b[%s]\n", dvars[i], stata_fmt(bi, "%9.0g", trim = TRUE), dvars[i])); next }
      v4 <- if (expo) c(exp(bi), exp(bi) * si, exp(bi - zq * si), exp(bi + zq * si)) else c(bi, si, bi - zq * si, bi + zq * si)
      cat(sprintf("%12s |%11s%11s%9s%8s%13s%12s\n", dvars[i], stata_fmt(v4[1], "%9.0g", trim = TRUE), stata_fmt(v4[2], "%9.0g", trim = TRUE),
                  stata_fmt(zi, "%7.2f", trim = TRUE), stata_fmt(pi, "%5.3f", trim = TRUE), stata_fmt(v4[3], "%9.0g", trim = TRUE), stata_fmt(v4[4], "%9.0g", trim = TRUE)))
    }
    cat(strrep("-", 78), "\n", sep = "")
    cat("Note: Chi-squared test is a Wald test of the coefficients of the variables\n",
        "      of interest jointly equal to zero. Lassos select controls for model\n",
        "      estimation. Type lassoinfo to see number of selected variables in each\n",
        "      lasso.\n", sep = "")
    if (vce == "cluster") cat("Note: Lassos are performed accounting for clusters in ", cluster, ".\n", sep = "")
  }
  # e(), in Stata 19's order (probes 475, 480)
  bn <- if (model != "linear") paste0(y, ":", dvars) else dvars
  s <- list(N = n, k_varsofinterest = k, k_controls = length(ctl))
  if (xpo) { s$n_xfolds <- xfolds; s$n_resample <- resample }
  s <- c(s, list(chi2 = chi2, p = pv, df = k, rank = k, k_controls_sel = ksel))
  runs <- if (xpo) xfolds * resample else 1
  m <- list(controls_sel = paste(ctl[union], collapse = " "), lasso_depvars_uniq = paste(vars, collapse = " "),
            lasso_depvars = paste(rep(vars, runs), collapse = " "), chi2type = "Wald", cmd = cmd, cmdline = cmdline,
            rngstate = rng, select_cmd = paste0(cmd, "_select"), predict = paste0(cmd, "_p"), marginsnotok = "_ALL")
  if (xpo) { m$title2 <- paste(mname, "model"); m$title <- title } else m$title <- title
  vt <- switch(vce, ols = "", hc2 = "Robust HC2", hc3 = "Robust HC3", hc0 = "Robust HC0", "Robust")
  if (nzchar(vt)) m$vcetype <- vt
  m$vce <- vce; m$model <- mname; m$controls <- paste(ctl, collapse = " "); m$varsofinterest <- paste(dvars, collapse = " ")
  if (po) { m$depvar <- y; m$properties <- "b V" } else { m$properties <- "b V"; m$depvar <- y }
  e <- c(lapply(s, as.numeric), m, list(b = matrix(b, 1, k, dimnames = list("y1", bn)), V = matrix(V, k, k, dimnames = list(bn, bn))))
  attr(e, "stata_sample") <- TRUE
  stata_e <<- e
  stata_esample <<- seq_len(nall) %in% src$stata_row[used]
  stata_infer <<- list(cmd = cmd, xpo = xpo, vars = vars, infos = infos, counts = counts, label = vapply(vars, labof, ""),
                       model = vapply(vars, function(v) if (v == y) mname else "linear", ""))
  invisible(NULL)
}
# lassoinfo after an inference command (probes 475, 480)
stata_infer_info <- function() {
  x <- stata_infer
  cat("\n    Estimate: active\n     Command: ", x$cmd, "\n", sep = "")
  if (x$xpo) {
    cat(strrep("-", 60), "\n            |                      No. of selected variables\n",
        "            |           Selection  -------------------------\n",
        "   Variable |    Model     method      min   median      max\n",
        "------------+", strrep("-", 47), "\n", sep = "")
    for (v in c(x$vars[-1], x$vars[1])) {
      cs <- x$counts[[v]]
      cat(sprintf("%11s |%9s%11s%9s%9s%9s\n", v, x$model[[v]], x$label[[v]], min(cs), stata_fmt(stats::median(cs), "%9.0g", trim = TRUE), max(cs)))
    }
    cat(strrep("-", 60), "\n", sep = "")
  } else {
    cat(strrep("-", 54), "\n            |                                   No. of\n",
        "            |           Selection             selected\n",
        "   Variable |    Model     method    lambda  variables\n",
        "------------+", strrep("-", 41), "\n", sep = "")
    for (v in x$vars) cat(sprintf("%11s |%9s%11s%10s%11s\n", v, x$model[[v]], x$label[[v]], stata_lasso_g8(x$infos[[v]]$lambda), x$infos[[v]]$k))
    cat(strrep("-", 54), "\n", sep = "")
  }
  invisible(NULL)
}
"###;

/// The e() names an inference command leaves that are text.
const INFER_TEXTS: &[&str] = &[
    "controls_sel",
    "lasso_depvars_uniq",
    "lasso_depvars",
    "chi2type",
    "cmd",
    "cmdline",
    "rngstate",
    "select_cmd",
    "predict",
    "marginsnotok",
    "title2",
    "title",
    "vcetype",
    "vce",
    "model",
    "controls",
    "varsofinterest",
    "depvar",
    "properties",
];

/// The commands translated here.
pub(super) const INFER_TRANSLATED: &[&str] = &["dsregress", "poregress", "xporegress", "dslogit"];

/// A list of plain names, or why it is not one (a range the run makes names).
fn names_of(text: &str, what: &str) -> Result<Vec<String>, Untranslated> {
    let mut out = Vec::new();
    for w in text.split_whitespace() {
        if !is_name(w) {
            return Err(no(if w.contains('.') {
                format!("{what} with factor variables ({w}) is not translated yet")
            } else {
                format!(
                    "{what} {w}: the data's variable names are {}",
                    crate::dofile::KNOWN_WHEN_RUN
                )
            }));
        }
        out.push(w.to_string());
    }
    Ok(out)
}

/// The R vector of names.
fn r_names(v: &[String]) -> String {
    if v.is_empty() {
        return "character(0)".into();
    }
    format!(
        "c({})",
        v.iter().map(|x| r_text(x)).collect::<Vec<_>>().join(", ")
    )
}

/// `selection(kind)` of the inference commands: the R argument of the lasso and the
/// word its log uses.
fn selection_of(text: &str) -> Option<(&'static str, &'static str)> {
    let t = text.trim();
    if t.contains(',') {
        return None;
    }
    let abbr = |full: &str, min: usize| t.len() >= min && full.starts_with(t);
    if abbr("plugin", 1) {
        Some(("plugin", "plugin"))
    } else if abbr("cv", 2) {
        Some(("cv", "cv"))
    } else if abbr("bic", 1) {
        Some(("bic", "BIC"))
    } else if abbr("adaptive", 1) {
        Some(("adaptive", "adaptive"))
    } else {
        None
    }
}

/// The selection() text inside lasso(varlist, selection(...)).
fn sub_selection(subs: &str) -> Option<String> {
    let t = subs.trim();
    let rest = t
        .strip_prefix("selection(")
        .or_else(|| t.strip_prefix("sel("))?;
    rest.strip_suffix(')').map(|s| s.to_string())
}

impl Session {
    fn infer_error(&mut self, out: &mut Out, msg: &str, rc: u32) -> Result<(), Untranslated> {
        out.line(format!(
            "stop({}, call. = FALSE)",
            r_text(&format!("{msg} (Stata: r({rc}))"))
        ));
        Ok(())
    }

    /// The inference command whose results are active, if this do-file translated it.
    pub(super) fn infer_active(&self) -> Option<String> {
        match &self.e_ret {
            Returned::Program { cmd, .. } if INFER_TRANSLATED.contains(&cmd.as_str()) => {
                Some(cmd.clone())
            }
            _ => None,
        }
    }

    /// `dsregress|poregress|xporegress|dslogit depvar varsofinterest,
    /// controls([(always)] others) [options]`.
    pub(super) fn infer(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let cmd = c.name.as_str();
        if !INFER_TRANSLATED.contains(&cmd) {
            return Err(no(format!(
                "{cmd} is not translated yet (dsregress, poregress, xporegress and dslogit are)"
            )));
        }
        self.no_by(c, cmd)?;
        if c.parts.weight.is_some() {
            return Err(no(format!("{cmd} with weights is not translated yet")));
        }
        let xpo = cmd.starts_with("xpo");
        let logit = cmd.ends_with("logit");
        let mut largs: Vec<String> = Vec::new();
        let mut label = "plugin";
        let mut vce = "robust".to_string();
        let mut cluster: Option<String> = None;
        let mut level = 95.0;
        let mut xfolds = 10.0;
        let mut resample = 1.0;
        let mut rseed: Option<f64> = None;
        let mut overs: Vec<String> = Vec::new();
        let mut controls: Option<String> = None;
        let mut sqrt = false;
        let mut coef = false;
        for o in opts.0 {
            let a = o.arg();
            if o.is("controls", 3) {
                controls = Some(a);
            } else if o.is("selection", 3) {
                match selection_of(&a) {
                    Some((k, l)) => {
                        largs.push(format!("sel = {}", r_text(k)));
                        label = l;
                    }
                    None => {
                        return Err(no(format!(
                            "{cmd}, selection({a}) is not translated yet (plugin, cv, bic and adaptive without suboptions are)"
                        )))
                    }
                }
            } else if o.is("rseed", 3) {
                match a.trim().parse::<f64>() {
                    Ok(v) if v >= 0.0 && v.fract() == 0.0 => rseed = Some(v),
                    _ => return Err(no(format!("{cmd}, rseed({a}) is not translated"))),
                }
            } else if o.is("vce", 3) {
                let words: Vec<&str> = a.split_whitespace().collect();
                match words.as_slice() {
                    ["robust" | "r"] => vce = "robust".into(),
                    // xporegress gives the robust variance for vce(ols) (probe 480)
                    ["ols"] => vce = if xpo { "robust".into() } else { "ols".into() },
                    [w @ ("hc2" | "hc3")] if !xpo => vce = (*w).to_string(),
                    // poregress's cluster variance is not Stata's (1.8% off in lasso_infer.do)
                    ["cluster" | "cl", v] if !xpo && !cmd.starts_with("po") && is_name(v) => {
                        vce = "cluster".into();
                        cluster = Some((*v).to_string());
                    }
                    _ => return Err(no(format!("{cmd}, vce({a}) is not translated yet"))),
                }
            } else if o.is("level", 1) {
                match a.trim().parse::<f64>() {
                    Ok(v) if (10.0..=99.99).contains(&v) => level = v,
                    _ => return Err(no(format!("{cmd}, level({a}) is not translated"))),
                }
            } else if o.is("nolog", 3) || o.is("coeflegend", 5) || (logit && o.is("or", 2)) {
            } else if logit && o.is("coef", 4) {
                coef = true;
            } else if !logit && o.is("sqrtlasso", 4) {
                sqrt = true;
            } else if xpo && o.is("xfolds", 3) {
                match a.trim().parse::<f64>() {
                    Ok(v) if v >= 2.0 && v.fract() == 0.0 => xfolds = v,
                    Ok(v) if v < 1.0 => {
                        return self.infer_error(
                            out,
                            "option xfolds(#) must be an integer >= 2",
                            198,
                        )
                    }
                    _ => {
                        return Err(no(format!(
                            "{cmd}, xfolds({a}) is not translated (xfolds(1) shows partialing-out)"
                        )))
                    }
                }
            } else if xpo && o.is("resample", 3) {
                match a.trim().parse::<f64>() {
                    Ok(v) if v >= 1.0 && v.fract() == 0.0 => resample = v,
                    Ok(v) if v < 1.0 => {
                        return self.infer_error(
                            out,
                            "option resample(#) must be a positive integer",
                            198,
                        )
                    }
                    _ => return Err(no(format!("{cmd}, resample({a}) is not translated"))),
                }
            } else if o.is("lasso", 3) {
                let (vars, subs) = a.split_once(',').unwrap_or((a.as_str(), ""));
                let vs = names_of(vars, cmd)?;
                match sub_selection(subs).as_deref().and_then(selection_of) {
                    Some((k, l)) => overs.push(format!(
                        "list(vars = {}, args = list(sel = {}), label = {})",
                        r_names(&vs),
                        r_text(k),
                        r_text(l)
                    )),
                    None => {
                        return Err(no(format!(
                        "{cmd}, lasso({a}): only selection(plugin|cv|bic|adaptive) is translated"
                    )))
                    }
                }
            } else {
                return Err(no(format!(
                    "{cmd} with the option {} is not translated yet",
                    o.name
                )));
            }
        }
        if xpo && (label == "cv" || label == "adaptive" || !overs.is_empty()) {
            return Err(no(format!(
                "{cmd} with selection(cv) or lasso() is not translated (Stata's cross-validation folds inside a cross-fit are not decoded)"
            )));
        }
        let Some(controls) = controls else {
            return self.infer_error(out, "option controls() required", 198);
        };
        let words: Vec<String> = names_of(&c.parts.main, cmd)?;
        let Some((dep, dvars)) = words.split_first() else {
            return Err(no(format!("{cmd} without variables")));
        };
        if dvars.is_empty() {
            return self.infer_error(out, "varsofinterest required", 198);
        }
        let (always, others) = match (controls.find('('), controls.find(')')) {
            (Some(a), Some(b)) if a < b => (
                names_of(controls.get(a + 1..b).unwrap_or(""), cmd)?,
                names_of(
                    &format!(
                        "{} {}",
                        controls.get(..a).unwrap_or(""),
                        controls.get(b + 1..).unwrap_or("")
                    ),
                    cmd,
                )?,
            ),
            (None, None) => (Vec::new(), names_of(&controls, cmd)?),
            _ => return Err(no(format!("{cmd}: controls() with a ( and no )"))),
        };
        if let Some(v) = dvars
            .iter()
            .find(|v| always.contains(v) || others.contains(v))
        {
            return self.infer_error(
                out,
                &format!("{v} cannot be specified both in varsofinterest and controls"),
                198,
            );
        }
        if others.is_empty() {
            return Err(no(format!(
                "{cmd} with always-in controls only is not translated"
            )));
        }
        if label == "plugin" && !largs.iter().any(|a| a.starts_with("sel ")) {
            largs.push("sel = \"plugin\"".into());
        }
        if sqrt {
            largs.push("sqrtl = TRUE".into());
        }
        let rows = self.rows_for(c, out)?;
        let d = self.data()?;
        out.package("dplyr");
        for h in [
            "stata_lasso",
            "stata_infer",
            "stata_fmt",
            "stata_rng",
            "stata_returned",
        ] {
            self.helpers.insert(h);
        }
        out.line(format!(
            "stata_infer_run(local({{ {d} <- dplyr::mutate({d}, stata_row = seq_len(nrow({d}))); {rows} }}), nrow({d}), {}, {}, {}, {}, {}, largs = list({}), over = list({}), label = {}, vce = {}, cluster = {}, level = {}, nolog = {}, legend = {}, coef = {}, xfolds = {}, resample = {}, rseed = {}, cmdline = {}, show = {})",
            r_text(cmd),
            r_text(dep),
            r_names(dvars),
            r_names(&always),
            r_names(&others),
            largs.join(", "),
            overs.join(", "),
            r_text(label),
            r_text(&vce),
            cluster.as_deref().map(r_text).unwrap_or_else(|| "NULL".into()),
            r_number(level),
            super::stats::r_logical(opts.has("nolog", 3)),
            super::stats::r_logical(opts.has("coeflegend", 5)),
            super::stats::r_logical(coef),
            r_number(xfolds),
            r_number(resample),
            rseed.map(r_number).unwrap_or_else(|| "NULL".into()),
            r_text(&super::returned::cmdline(c)),
            super::stats::r_logical(!self.quiet(c))
        ));
        self.esample_new = true;
        self.e_new = true;
        self.estimates = None;
        self.estimates_lost = Some(format!(
            "test, lincom, margins and the other post-estimation commands are not translated after {cmd} yet"
        ));
        self.e_text.clear();
        self.fit_ts = None;
        let mut texts: Vec<String> = INFER_TEXTS.iter().map(|s| s.to_string()).collect();
        if label != "plugin" || !overs.is_empty() {
            // lassoinfo's table for the other selections is not known
            texts.push("=notplugin".into());
        }
        self.e_ret = Returned::Program {
            cmd: cmd.into(),
            texts,
        };
        Ok(())
    }

    /// `lassoinfo` after an inference command.
    pub(super) fn infer_info(&mut self, c: &Command, out: &mut Out) -> Result<(), Untranslated> {
        let plugin = !matches!(&self.e_ret, Returned::Program { texts, .. } if texts.iter().any(|t| t == "=notplugin"));
        if !c.parts.main.trim().is_empty() || !c.options.is_empty() || !plugin {
            return Err(no(
                "lassoinfo after an inference command is translated for the active results with plug-in lassos only",
            ));
        }
        self.helpers.insert("stata_infer");
        self.helpers.insert("stata_lasso");
        self.helpers.insert("stata_fmt");
        if !self.quiet(c) {
            out.line("stata_infer_info()");
        }
        Ok(())
    }

    /// `predict [type] newvar [if] [in] [, xb]` after an inference command: the
    /// variables of interest times their coefficients (probe 480).
    pub(super) fn infer_predict(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[("xb", 2)]) {
            return self.infer_error(out, &format!("option {o} not allowed"), 198);
        }
        let mut words: Vec<&str> = c.parts.main.split_whitespace().collect();
        let ty = match words.first() {
            Some(t) if storage(t).is_some() => {
                let t = *t;
                words.remove(0);
                t
            }
            _ => "float",
        };
        let st = storage(ty).ok_or_else(|| no(format!("{ty} is not a storage type")))?;
        if st == Storage::Str {
            return Err(no("predict makes a numeric variable, not a string"));
        }
        let [w] = words.as_slice() else {
            return Err(no("predict makes one new variable"));
        };
        if !is_name(w) {
            return Err(no(format!(
                "{w}: only a plain new variable name is translated"
            )));
        }
        let base = self.data()?;
        out.line(format!(
            "if ({} %in% names({base})) stop(\"variable {w} already defined (Stata: r(110))\", call. = FALSE)",
            r_text(w)
        ));
        if !opts.has("xb", 2) && !self.quiet(c) {
            out.line("cat(\"(option xb assumed); partial linear prediction\\n\")");
        }
        out.line(format!(
            "stata_infer_p <- local({{ b <- stata_e$b; v <- 0; for (j in seq_len(ncol(b))) {{ x <- unclass({base}[[sub(\"^.*:\", \"\", colnames(b)[j])]]); attributes(x) <- NULL; v <- v + as.numeric(x) * b[1, j] }}; v }})"
        ));
        let v = stored("stata_infer_p", st, &mut self.helpers, out);
        let v = match self.condition(c, out)? {
            Some(cond) => format!("dplyr::if_else({cond}, {v}, NA_real_)"),
            None => v,
        };
        out.package("dplyr");
        out.line(format!(
            "{base} <- dplyr::mutate({base}, {} = {v})",
            r_name(w)
        ));
        out.line("rm(stata_infer_p)");
        self.kinds.insert(w.to_string(), Kind::Number);
        self.storage.insert(w.to_string(), st);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::data_r::translate;
    use crate::reader::read;

    fn r(source: &str) -> Vec<String> {
        translate(&read(source))
            .lines
            .into_iter()
            .map(|l| match l.result {
                Ok(r) => r.code,
                Err(e) => format!("NOT: {}", e.reason),
            })
            .collect()
    }

    #[test]
    fn the_inference_commands_run_their_lassos_then_the_estimator() {
        let out = r("use d
dsregress y d1, controls((x7) x1 x2) vce(hc3) coefl
lassoinfo
predict p
xporegress y d1 d2, controls(x1 x2) rseed(1) xfolds(5) resample(2)
dslogit yb d1, controls(x1 x2) selection(cv) rseed(3)
lassoinfo");
        assert!(
            out[1].contains("stata_infer_run(")
                && out[1].contains("\"dsregress\"")
                && out[1].contains("c(\"x7\"), c(\"x1\", \"x2\")")
                && out[1].contains("sel = \"plugin\"")
                && out[1].contains("vce = \"hc3\"")
                && out[1].contains("legend = TRUE"),
            "{out:?}"
        );
        assert!(out[2].contains("stata_infer_info()"), "{out:?}");
        assert!(out[3].contains("partial linear prediction"), "{out:?}");
        assert!(
            out[4].contains("xfolds = 5") && out[4].contains("resample = 2"),
            "{out:?}"
        );
        assert!(out[5].contains("sel = \"cv\""), "{out:?}");
        // lassoinfo's table after cross-validated lassos is not known
        assert!(
            out[6].contains("NOT: lassoinfo after an inference command"),
            "{out:?}"
        );
    }

    #[test]
    fn what_is_not_statas_is_refused_and_statas_errors_are_kept() {
        let out = r("use d
dsregress y d1
dsregress y, controls(x1 x2)
dsregress y d1, controls(d1 x1)
xporegress y d1, controls(x1 x2) resample(0)
xporegress y d1, controls(x1 x2) selection(cv)
poregress y d1, controls(x1 x2) vce(cluster g)
pologit yb d1, controls(x1 x2)
dsregress y d1, controls(x1 x2)
predict r, residuals");
        assert!(
            out[1].contains("option controls() required (Stata: r(198))"),
            "{out:?}"
        );
        assert!(
            out[2].contains("varsofinterest required (Stata: r(198))"),
            "{out:?}"
        );
        assert!(
            out[3].contains("d1 cannot be specified both in varsofinterest and controls"),
            "{out:?}"
        );
        assert!(
            out[4].contains("resample(#) must be a positive integer"),
            "{out:?}"
        );
        assert!(
            out[5].contains("NOT: xporegress with selection(cv)"),
            "{out:?}"
        );
        assert!(out[6].contains("NOT: poregress, vce(cluster g)"), "{out:?}");
        assert!(
            out[7].contains("NOT: pologit is not translated yet"),
            "{out:?}"
        );
        assert!(
            out[9].contains("option residuals not allowed (Stata: r(198))"),
            "{out:?}"
        );
    }
}
