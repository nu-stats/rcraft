//! `qreg depvar indepvars [if] [in] [fweight], [quantile(#)] nolog`: quantile regression as
//! Stata 19 reports it (`plan/stata/qreg.md`).
//!
//! - The estimates minimise the sum of q (r > 0) and (1 - q) (-r) (r < 0) over the residuals,
//!   found by the Barrodale-Roberts simplex in R's `quantreg` (a separate R process, never
//!   linked); where the minimum is not unique Stata's own path may end at another vertex, which
//!   is why Stata's iteration log is not reproduced and only `nolog` is translated.
//! - Header: `Raw sum of deviations` about the sample quantile (the smallest y with at least
//!   the share q at or below it), `Min sum of deviations`, `Pseudo R2` = 1 - min/raw.
//! - V = q (1 - q) s^2 (X'WX)^-1, with the sparsity s = xbar'(b(q + h) - b(q - h))/(2h) and
//!   Hall and Sheather's bandwidth h = n^(-1/3) z^(2/3) [1.5 phi(Phi^-1(q))^2 /
//!   (2 Phi^-1(q)^2 + 1)]^(1/3); t tests on N - k degrees of freedom (k with the constant).

use super::model::{level, Estimates, Fit, Spec};
use super::weights::W;
use super::*;

/// The fit, its table and the predictions.
pub(super) const QREG_HELPER: &str = r##"# the minimum of sum w rho_q(y - X b) by descending the edges of the linear programme's vertices (each
# fits k rows exactly): from the k rows least squares fits best, along the steepest edge to the first
# residual that turns the slope up. Where the optimum is not unique this ends where Stata's does.
stata_qreg_solve <- function(X, y, w, q) {
  n <- nrow(X)
  k <- ncol(X)
  ls <- solve(crossprod(X * w, X), crossprod(X * w, y))
  ord <- order(abs(y - drop(X %*% ls)))
  h <- integer()
  for (r in ord) {
    trial <- c(h, r)
    if (qr(X[trial, , drop = FALSE], tol = 1e-10)$rank == length(trial)) h <- trial
    if (length(h) == k) break
  }
  if (length(h) < k) stop("RCraft: qreg with collinear regressors is not translated yet", call. = FALSE)
  for (iter in seq_len(50 * n + 100)) {
    inv <- solve(X[h, , drop = FALSE])
    b <- drop(inv %*% y[h])
    resid <- y - drop(X %*% b)
    Cm <- X %*% inv
    nb <- setdiff(seq_len(n), h)
    rr <- resid[nb]
    wr <- w[nb]
    gs <- numeric(2 * k)
    for (j in seq_len(k)) {
      for (s in 1:2) {
        sg <- if (s == 1) 1 else -1
        cj <- sg * Cm[nb, j]
        term <- ifelse(rr > 1e-12, -q * cj, ifelse(rr < -1e-12, (1 - q) * cj, pmax(-q * cj, (1 - q) * cj)))
        gs[2 * (j - 1) + s] <- 2 * (w[h[j]] * (if (sg > 0) 1 - q else q) + sum(wr * term))
      }
    }
    cand <- which(gs < -1e-10)
    if (!length(cand)) {
      # an edge along which the objective stays flat: the optimum is not unique
      if (any(abs(gs) < 1e-10)) {
        stop("RCraft: qreg where the minimum is not unique is not translated: Stata ends at one of several optimal vertices by its own path, which is not reproduced", call. = FALSE)
      }
      return(b)
    }
    m <- cand[which.min(gs[cand])]
    j <- (m + 1) %/% 2
    sg <- if (m %% 2 == 1) 1 else -1
    g <- gs[m]
    d <- sg * inv[, j]
    cd <- drop(X[nb, , drop = FALSE] %*% d)
    t <- rr / cd
    keep <- which(abs(cd) > 1e-14 & t > -1e-12)
    t <- pmax(t[keep], 0)
    add <- 2 * wr[keep] * abs(cd[keep])
    o <- order(t)
    slope <- g
    enter <- NA_integer_
    for (i in o) {
      slope <- slope + add[i]
      if (slope >= -1e-12) {
        enter <- nb[keep[i]]
        break
      }
    }
    if (is.na(enter)) stop("RCraft: qreg did not find an optimum", call. = FALSE)
    h[j] <- enter
  }
  stop("RCraft: qreg did not settle", call. = FALSE)
}
stata_qreg <- function(d, dep, terms, q = 0.5, wvar = NULL, wtype = NULL, level = 95, cmdline = "", show = TRUE) {
  num <- function(u) { u <- unclass(u); attributes(u) <- NULL; as.numeric(u) }
  tvars <- setdiff(unique(unlist(regmatches(terms, gregexpr("[A-Za-z_][A-Za-z0-9_]*", terms)))), "factor")
  for (v in unique(c(dep, tvars, wvar))) if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
  ok <- stats::complete.cases(d[unique(c(dep, tvars, wvar))])
  if (!any(ok)) stop("no observations (Stata: r(2000))", call. = FALSE)
  rows <- which(ok)
  mf <- d[rows, , drop = FALSE]
  n <- nrow(mf)
  fo <- stats::as.formula(paste("~", paste(terms, collapse = " + ")))
  X <- stats::model.matrix(fo, mf)
  X <- X[, colnames(X) != "(Intercept)", drop = FALSE]
  xlev <- stats::.getXlevels(stats::terms(fo), stats::model.frame(fo, mf))
  rn <- function(s) ifelse(grepl("^factor\\(", s), sub("^factor\\(([^)]*)\\)(.*)$", "\\2.\\1", s), s)
  nmx <- rn(colnames(X))
  k <- ncol(X)
  X1 <- cbind(X, 1)
  nm <- c(nmx, "_cons")
  y <- num(mf[[dep]])
  w <- if (is.null(wvar)) rep(1, n) else num(mf[[wvar]])
  N <- sum(w)
  if (qr(X1 * sqrt(w))$rank < k + 1) stop("RCraft: qreg with collinear regressors is not translated yet", call. = FALSE)
  fit <- function(tau) stats::setNames(stata_qreg_solve(X1, y, w, tau), nm)
  b <- fit(q)
  res <- y - drop(X1 %*% b)
  rho <- function(r) ifelse(r > 0, q * r, (1 - q) * (-r))
  mins <- sum(w * rho(res))
  # the sample quantile: the smallest y with at least the share q at or below it
  o <- order(y)
  cw <- cumsum(w[o]) / N
  qv <- y[o][which(cw >= q - 1e-12)[1]]
  raw <- sum(w * rho(y - qv))
  z <- stats::qnorm(1 - (100 - level) / 200)
  qn <- stats::qnorm(q)
  h <- N^(-1/3) * z^(2/3) * (1.5 * stats::dnorm(qn)^2 / (2 * qn^2 + 1))^(1/3)
  xbar <- colSums(w * X1) / N
  hi <- min(q + h, 1 - 1e-9)
  lo <- max(q - h, 1e-9)
  spars <- sum(xbar * (fit(hi) - fit(lo))) / (hi - lo)
  A <- crossprod(X1 * w, X1)
  V <- q * (1 - q) * spars^2 * solve(A)
  dimnames(V) <- list(nm, nm)
  dfr <- N - k - 1
  e <- list(rank = k + 1, sparsity = spars, bwidth = h, df_m = k, df_r = dfr, f_r = 1 / spars, N = N, sum_w = N,
            q_v = qv, q = q, sum_rdev = raw, sum_adev = mins, convcode = 0, r2_p = 1 - mins / raw)
  e$cmd <- "qreg"
  e$cmdline <- cmdline
  e$predict <- "qreg_p"
  e$properties <- "b V"
  e$marginsnotok <- "stdp stddp residuals"
  e$vce <- "iid"
  e$denmethod <- "fitted"
  e$bwmethod <- "hsheather"
  e$depvar <- dep
  # a factor's base level is a column of its own in e(b) and e(V), zero
  full <- character()
  for (j in seq_along(nmx)) {
    cj <- colnames(X)[j]
    fv <- if (grepl("^factor\\(", cj)) sub("^(factor\\([^)]*\\)).*$", "\\1", cj) else NA_character_
    if (!is.na(fv) && (j == 1 || !startsWith(colnames(X)[j - 1], fv))) {
      v <- sub("^factor\\((.*)\\)$", "\\1", fv)
      full <- c(full, paste0(xlev[[fv]][1], "b.", v))
    }
    full <- c(full, nmx[j])
  }
  full <- c(full, "_cons")
  bf <- stats::setNames(numeric(length(full)), full)
  bf[nm] <- b
  Vf <- matrix(0, length(full), length(full), dimnames = list(full, full))
  Vf[nm, nm] <- V
  e$b <- matrix(bf, 1, length(bf), dimnames = list("y1", full))
  e$V <- Vf
  attr(e, "stata_rows") <- rows
  rname <- sub("^_cons$", "(Intercept)", nm)
  rname[seq_len(k)] <- colnames(X)
  attr(e, "stata_b") <- stats::setNames(b, rname)
  attr(e, "stata_V") <- { Vn <- V; dimnames(Vn) <- list(rname, rname); Vn }
  attr(e, "stata_areg") <- list(fo = fo, xlev = xlev, nmx = nmx)
  if (show) stata_qreg_show(e, level, mf)
  e
}
stata_qreg_show <- function(e, level = 95, data = NULL) {
  pad <- stata_tab_pad
  qq <- e$q
  title <- if (qq == 0.5) "Median regression" else paste0(sub("^0", "", format(qq)), " Quantile regression")
  fmt <- function(label, v) sprintf("%-14s=%11s", label, v)
  cat("\n")
  r1 <- fmt("Number of obs", stata_tab_count(e$N))
  cat(formatC(title, width = -(78 - nchar(r1))), r1, "\n", sep = "")
  cat("  Raw sum of deviations", formatC(stata_fmt(e$sum_rdev, "%9.0g", trim = TRUE), width = 9), " (about ",
      stata_fmt(e$q_v, "%10.0g", trim = TRUE), ")\n", sep = "")
  l3 <- paste0("  Min sum of deviations", formatC(stata_fmt(e$sum_adev, "%9.0g", trim = TRUE), width = 9))
  r3 <- fmt("Pseudo R2", sprintf("%.4f", e$r2_p))
  cat(formatC(l3, width = -(78 - nchar(r3))), r3, "\n", sep = "")
  cat("\n")
  nm <- colnames(e$b)
  b <- stats::setNames(as.vector(e$b), nm)
  se <- stats::setNames(sqrt(diag(e$V)), nm)
  df <- e$df_r
  qt <- stats::qt((1 + level / 100) / 2, df)
  rows <- stata_coef_rows(nm, data)
  if (is.null(rows)) stop("RCraft: qreg with a term whose table layout was not checked is not translated yet", call. = FALSE)
  cat(strrep("-", 78), "\n", sep = "")
  cat(pad(stata_tab_abbrev(e$depvar), 12), " | Coefficient  Std. err.      t    P>|t|     [", format(level), "% conf. interval]\n", sep = "")
  cat(strrep("-", 13), "+", strrep("-", 64), "\n", sep = "")
  for (r in rows) {
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
    label <- if (r$kind == "level") paste0(pad(r$label, 11), " ") else pad(stata_tab_abbrev(r$label), 12)
    if (!is.null(r$note)) {
      cat(label, " |", pad("0", 11), "  ", r$note, "\n", sep = "")
      next
    }
    bi <- b[[r$col]]
    si <- se[[r$col]]
    t <- bi / si
    p <- 2 * stats::pt(-abs(t), df)
    cat(label, " |", pad(stata_tab_g(bi), 11), pad(stata_tab_g(si), 11), pad(sprintf("%.2f", t), 9),
        pad(sprintf("%.3f", p), 8), pad(stata_tab_g(bi - qt * si), 13), pad(stata_tab_g(bi + qt * si), 12), "\n", sep = "")
  }
  cat(strrep("-", 78), "\n", sep = "")
  invisible(NULL)
}
stata_qreg_predict <- function(e, d, what) {
  a <- attr(e, "stata_areg")
  b <- e$b
  mf <- stats::model.frame(a$fo, d, xlev = a$xlev, na.action = stats::na.pass)
  X <- stats::model.matrix(a$fo, mf)
  X <- X[, colnames(X) != "(Intercept)", drop = FALSE]
  xb <- drop(X %*% b[1, a$nmx]) + b[1, "_cons"]
  switch(what, xb = xb, residuals = { yv <- unclass(d[[e$depvar]]); attributes(yv) <- NULL; as.numeric(yv) - xb })
}"##;

impl Session {
    /// Whether `qreg`'s results are the active ones.
    pub(super) fn qreg_active(&self) -> bool {
        matches!(
            &self.e_ret,
            super::returned::Returned::Program { cmd, .. } if cmd == "qreg"
        ) && self.estimates.as_ref().is_some_and(|e| e.fit == Fit::Qreg)
    }

    /// `qreg depvar indepvars [if] [in] [fweight] [, quantile(#) level() nolog]`.
    pub(super) fn qreg(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "qreg")?;
        if let Some(o) = opts.other(&[("quantile", 1), ("level", 3), ("nolog", 3)]) {
            return Err(no(format!(
                "qreg with the option {o} is not translated yet"
            )));
        }
        if !opts.has("nolog", 3) && !self.quiet(c) {
            return Err(no(
                "qreg without nolog prints Stata's own simplex path (its iteration log), which is not translated; use nolog",
            ));
        }
        let q = match opts.arg("quantile", 1) {
            None => 0.5,
            Some(t) => {
                let v: f64 = t
                    .trim()
                    .parse()
                    .map_err(|_| no(format!("quantile({t}) is not a number")))?;
                // quantile(75) means .75
                let v = if v > 1.0 { v / 100.0 } else { v };
                if !(v > 0.0 && v < 1.0) {
                    return Err(no(format!("quantile({t}) is outside (0, 1)")));
                }
                v
            }
        };
        let spec = Spec::read(c, "qreg")?;
        if !is_name(&spec.dep) || spec.terms.is_empty() {
            return Err(no(
                "qreg with operators or no independent variable is not translated yet",
            ));
        }
        let plain = |t: &str| {
            is_name(t)
                || (t.starts_with("factor(") && t.ends_with(')') && is_name(&t[7..t.len() - 1]))
        };
        if spec.terms.iter().any(|t| !plain(t)) {
            return Err(no(
                "qreg with interactions, ib. bases or operators is not translated yet",
            ));
        }
        let w = self.weight_of(c, &[W::F, W::A, W::P, W::I])?;
        match &w {
            Some((W::F, _)) | None => {}
            Some((k, _)) => {
                let word = match k {
                    W::A => "aweights",
                    W::P => "pweights",
                    _ => "iweights",
                };
                out.line(format!(
                    "stop({}, call. = FALSE)",
                    r_text(&format!("{word} not allowed (Stata: r(101))"))
                ));
                return Ok(());
            }
        }
        let level = level(opts)?;
        let rows = self.rows_for(c, out)?;
        let src = self.weighted_rows(rows, &w);
        for h in [
            "stata_lsfam",
            "stata_qreg",
            "stata_fmt",
            "stata_tables",
            "stata_returned",
            "stata_eclass",
        ] {
            self.helpers.insert(h);
        }
        let wargs = match &w {
            Some((k, v)) => format!(", wvar = {}, wtype = {}", r_text(v), r_text(k.code())),
            None => String::new(),
        };
        let show = !self.quiet(c);
        out.line(format!(
            "stata_e <- stata_qreg({src}, {}, c({}), q = {}{wargs}, level = {}, cmdline = {}, show = {})",
            r_text(&spec.dep),
            spec.terms
                .iter()
                .map(|t| r_text(t))
                .collect::<Vec<_>>()
                .join(", "),
            r_number(q),
            r_number(level * 100.0),
            r_text(&super::returned::cmdline(c)),
            super::stats::r_logical(show)
        ));
        out.line("stata_b <- attr(stata_e, \"stata_b\")");
        out.line("stata_vcov <- attr(stata_e, \"stata_V\")");
        out.line("stata_df <- stata_e$df_r");
        out.line("stata_estimates <- list(rows = attr(stata_e, \"stata_rows\"))");
        out.line(self.keep_rows(&src, "stata_estimates$rows"));
        let d = self.data()?;
        out.package("dplyr");
        out.line(super::model::esample_rows_line(
            &d,
            &src,
            "stata_estimates$rows",
        ));
        self.esample_new = true;
        self.fit_robust = false;
        self.fit_cluster = false;
        self.estimates = Some(Estimates {
            fit: Fit::Qreg,
            dep: spec.dep.clone(),
            factors: spec.factors.clone(),
        });
        self.e_new = true;
        self.e_ret = super::returned::Returned::Program {
            cmd: "qreg".into(),
            texts: [
                "cmd",
                "cmdline",
                "predict",
                "properties",
                "marginsnotok",
                "vce",
                "denmethod",
                "bwmethod",
                "depvar",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
        };
        out.caveat(
            "qreg as Stata 19 reports it: the estimates from R's quantreg (simplex), the sparsity from fits at q +/- h \
             (Hall and Sheather's bandwidth), V = q(1 - q) s^2 (X'WX)^-1; where the minimum is not unique Stata's vertex can differ",
        );
        Ok(())
    }

    /// `predict [type] newvar [if] [in] [, xb residuals]` after `qreg`.
    pub(super) fn qreg_predict(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[("xb", 2), ("residuals", 1)]) {
            return Err(no(format!(
                "predict after qreg with the option {o} is not translated yet"
            )));
        }
        if c.options.len() > 1 {
            return Err(no("predict takes one statistic at a time"));
        }
        let what = if opts.has("residuals", 1) {
            "residuals"
        } else {
            "xb"
        };
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
        if c.options.is_empty() && !self.quiet(c) {
            out.line("cat(\"(option xb assumed; fitted values)\\n\")");
        }
        self.helpers.insert("stata_qreg");
        let v = format!("stata_qreg_predict(stata_e, {base}, {})", r_text(what));
        let v = stored(&v, st, &mut self.helpers, out);
        let v = match self.condition(c, out)? {
            Some(cond) => format!("dplyr::if_else({cond}, {v}, NA_real_)"),
            None => v,
        };
        out.package("dplyr");
        out.line(format!(
            "{base} <- dplyr::mutate({base}, {} = {v})",
            r_name(w)
        ));
        self.kinds.insert(w.to_string(), Kind::Number);
        self.storage.insert(w.to_string(), st);
        Ok(())
    }
}
