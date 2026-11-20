//! `areg depvar indepvars [if] [in] [weight], absorb(var)`: linear regression absorbing
//! the indicators of one variable, as Stata 19 does it (`plan/stata/areg.md`).
//!
//! - The slopes are the within fit: each group's (weighted) means removed and the
//!   grand means put back, so the constant is ybar - xbar b. The residual degrees of
//!   freedom are N - k - G (k the slopes, G the groups); s^2 = rss/df_r; V =
//!   s^2 (X*'WX*)^-1 on those columns.
//! - R-squared, rss and mss are the full model's (the indicators included),
//!   R-squared adjusted divides by df_r, F is the Wald test of the slopes.
//! - Robust: the sandwich times N/(N - k - G); clustered: G_c/(G_c - 1) (N - 1)/(N - k - G)
//!   with t and F on G_c - 1; sampling weights imply robust.
//! - `F test of absorbed indicators` compares the fit with the pooled one without them
//!   (not printed after robust errors); e(ll_0) is the within data's constant-only likelihood.

use super::model::{level, vce, Estimates, Fit, Spec};
use super::weights::W;
use super::*;

/// The fit, its table and the predictions.
pub(super) const AREG_HELPER: &str = r##"stata_areg <- function(d, dep, terms, absorb, wvar = NULL, wtype = NULL, robust = FALSE, cluster = NULL,
                       level = 95, cmdline = "", show = TRUE) {
  num <- function(u) { u <- unclass(u); attributes(u) <- NULL; as.numeric(u) }
  tvars <- unique(unlist(regmatches(terms, gregexpr("[A-Za-z_][A-Za-z0-9_]*", terms))))
  tvars <- setdiff(tvars, "factor")
  for (v in unique(c(dep, tvars, absorb, wvar, cluster))) if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
  ok <- stats::complete.cases(d[unique(c(dep, tvars, absorb, wvar, cluster))])
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
  y <- num(mf[[dep]])
  g <- as.character(mf[[absorb]])
  w <- if (is.null(wvar)) rep(1, n) else num(mf[[wvar]])
  if (identical(wtype, "f")) {
    Nn <- sum(w)
    wn <- w
  } else if (!is.null(wtype)) {
    Nn <- n
    wn <- w * n / sum(w)
  } else {
    Nn <- n
    wn <- w
  }
  if (identical(wtype, "p")) robust <- TRUE
  gi <- match(g, unique(g))
  G <- max(gi)
  k <- ncol(X)
  gm <- function(v) {
    s <- rowsum(wn * v, gi)[, 1] / rowsum(wn, gi)[, 1]
    s[gi]
  }
  wm <- function(v) sum(wn * v) / sum(wn)
  within <- function(M) matrix(vapply(seq_len(ncol(M)), function(j) M[, j] - gm(M[, j]) + wm(M[, j]), numeric(n)), n, ncol(M))
  # collinear regressors, as Stata 19 omits them (probe 2026-11-20): one the absorbed groups make
  # collinear (constant within each group, or constant), wherever it stands, then the later of a
  # collinear set, in the order written; each with a note, an o. column of zeros in e(b) and e(V)
  Xall <- X
  nmall <- nmx
  kept <- rep(TRUE, k)
  if (k > 0) {
    Xd0 <- within(X)
    Xd0 <- Xd0 - matrix(colSums(wn * Xd0) / sum(wn), n, k, byrow = TRUE)
    sw <- sqrt(wn)
    Q <- matrix(0, n, 0)
    for (j in seq_len(k)) {
      ref <- sqrt(sum(wn * (X[, j] - wm(X[, j]))^2))
      v <- Xd0[, j] * sw
      if (ref > 0 && sqrt(sum(v^2)) <= 1e-7 * ref) { kept[j] <- FALSE; next }
      if (ncol(Q)) v <- v - drop(Q %*% crossprod(Q, v))
      nv <- sqrt(sum(v^2))
      if (!(ref > 0) || nv <= 1e-7 * ref) kept[j] <- FALSE else Q <- cbind(Q, v / nv)
    }
  }
  if (any(!kept & grepl("^factor\\(", colnames(X)))) {
    stop("RCraft: areg with a factor level omitted for collinearity is not translated yet", call. = FALSE)
  }
  if (k > 0 && !any(kept)) stop("RCraft: areg with every regressor omitted for collinearity is not translated yet", call. = FALSE)
  if (show) for (v in nmx[!kept]) cat("note: ", v, " omitted because of collinearity.\n", sep = "")
  X <- X[, kept, drop = FALSE]
  nmx <- nmx[kept]
  k <- ncol(X)
  Xs <- within(X)
  ys <- y - gm(y) + wm(y)
  Z <- cbind(Xs, `_cons` = 1)
  A <- crossprod(Z * wn, Z)
  Ainv <- solve(A)
  b <- drop(Ainv %*% crossprod(Z * wn, ys))
  res <- ys - drop(Z %*% b)
  rss <- sum(wn * res^2)
  dfr <- Nn - k - G
  s2 <- rss / dfr
  V <- Ainv * s2
  df <- dfr
  if (robust || !is.null(cluster)) {
    if (is.null(cluster)) {
      sc <- Z * (if (identical(wtype, "f")) sqrt(w) * res else wn * res)
      V <- Ainv %*% crossprod(sc) %*% Ainv * Nn / dfr
    } else {
      sg <- rowsum(Z * (wn * res), mf[[cluster]])
      Gc <- nrow(sg)
      V <- Ainv %*% crossprod(sg) %*% Ainv * Gc / (Gc - 1) * (Nn - 1) / dfr
      df <- Gc - 1
    }
  }
  nm <- c(nmx, "_cons")
  dimnames(V) <- list(nm, nm)
  names(b) <- nm
  yb <- wm(y)
  tss <- sum(wn * (y - yb)^2)
  mss <- tss - rss
  r2 <- 1 - rss / tss
  df_m <- k
  W <- drop(t(b[seq_len(k)]) %*% solve(V[seq_len(k), seq_len(k), drop = FALSE], b[seq_len(k)]))
  Fv <- W / df_m
  pF <- stats::pf(Fv, df_m, df, lower.tail = FALSE)
  # the absorbed indicators: the pooled fit against this one
  Zp <- cbind(X, 1)
  bp <- solve(crossprod(Zp * wn, Zp), crossprod(Zp * wn, y))
  rss_p <- sum(wn * (y - drop(Zp %*% bp))^2)
  df_a <- G - 1
  F_abs <- ((rss_p - rss) / df_a) / (rss / dfr)
  p_abs <- stats::pf(F_abs, df_a, dfr, lower.tail = FALSE)
  ll <- -0.5 * Nn * (log(2 * pi) + log(rss / Nn) + 1) + if (identical(wtype, "a")) 0.5 * sum(log(wn)) else 0
  tss_w <- sum(wn * (ys - wm(ys))^2)
  ll0 <- -0.5 * Nn * (log(2 * pi) + log(tss_w / Nn) + 1) + if (identical(wtype, "a")) 0.5 * sum(log(wn)) else 0
  # the absorbed effect of each group: ybar_g - xbar_g b - _cons
  xbg <- if (k) rowsum(wn * X, gi) / rowsum(wn, gi)[, 1] else matrix(0, G, 0)
  ybg <- rowsum(wn * y, gi)[, 1] / rowsum(wn, gi)[, 1]
  dg <- stats::setNames(ybg - drop(xbg %*% b[seq_len(k)]) - b[["_cons"]], unique(g))
  e <- list()
  put <- function(...) { a <- list(...); for (nme in names(a)) e[[nme]] <<- a[[nme]] }
  if (!(robust || !is.null(cluster))) put(F_absorb = F_abs, p_absorb = p_abs)
  put(rank = if (is.null(cluster)) k + 1 else k, p = pF, rc = 0, k_absorb = G, ll_0 = ll0, ll = ll, tss = tss, df_a = df_a,
      r2_a = 1 - (1 - r2) * (Nn - 1) / dfr, rss = rss, mss = mss, rmse = sqrt(rss / dfr), r2 = r2, F = Fv, df_r = df,
      df_m = df_m)
  if (!is.null(cluster)) put(N_clust = Gc)
  # Stata keeps no e(sum_w) when the clusters are not the absorbed variable
  put(sum_w = if (!is.null(cluster) && cluster != absorb) NA_real_ else if (is.null(wvar)) Nn else sum(w), N = Nn)
  put(cmdline = cmdline)
  if (robust || !is.null(cluster)) put(vcetype = "Robust")
  put(cmd = "areg", footnote = "areg_footnote", title = "Linear regression, absorbing indicators", predict = "areg_p",
      marginsnotok = "Residuals SCore D XBD", vce = if (!is.null(cluster)) "cluster" else if (robust) "robust" else "ols")
  if (!is.null(cluster)) put(clustvar = cluster)
  put(depvar = dep, properties = "b V", model = "ols", absvar = absorb)
  # a factor's base level is a column of its own in e(b) and e(V), zero
  full <- character()
  for (j in seq_along(nmall)) {
    cj <- colnames(Xall)[j]
    fv <- if (grepl("^factor\\(", cj)) sub("^(factor\\([^)]*\\)).*$", "\\1", cj) else NA_character_
    if (!is.na(fv) && (j == 1 || !startsWith(colnames(Xall)[j - 1], fv))) {
      v <- sub("^factor\\((.*)\\)$", "\\1", fv)
      full <- c(full, paste0(xlev[[fv]][1], "b.", v))
    }
    full <- c(full, if (kept[j]) nmall[j] else paste0("o.", nmall[j]))
  }
  full <- c(full, "_cons")
  bf <- stats::setNames(numeric(length(full)), full)
  bf[nm] <- b
  Vf <- matrix(0, length(full), length(full), dimnames = list(full, full))
  Vf[nm, nm] <- V
  put(b = matrix(bf, 1, length(bf), dimnames = list("y1", full)), V = Vf)
  attr(e, "stata_rows") <- rows
  rname <- sub("^_cons$", "(Intercept)", nm)
  rname[seq_len(k)] <- colnames(X)
  attr(e, "stata_b") <- stats::setNames(b, rname)
  attr(e, "stata_V") <- { Vn <- V; dimnames(Vn) <- list(rname, rname); Vn }
  attr(e, "stata_areg") <- list(fo = fo, xlev = xlev, dg = dg, absorb = absorb, nm = nm, k = k, nmx = nmx, cols = colnames(X))
  attr(e, "stata_robust") <- robust || !is.null(cluster)
  attr(e, "stata_rows_used") <- rows
  if (show && !is.null(wtype) && wtype %in% c("a", "p")) cat("(sum of wgt is ", stata_fmt(sum(w), "%18.0g", trim = TRUE), ")\n", sep = "")
  if (show) stata_areg_show(e, level, G, if (is.null(cluster)) NULL else Gc, cluster, Nn, mf)
  e
}
stata_areg_show <- function(e, level = 95, G = e$k_absorb, Gc = NULL, cluster = NULL, Nn = e$N, data = NULL) {
  robust <- !is.null(e[["vcetype"]])
  pad <- stata_tab_pad
  left <- c(e$title, paste0("Absorbed variable: ", e$absvar))
  fmt <- function(label, v) sprintf("%-18s=%7s", label, v)
  right <- c(fmt("Number of obs", stata_tab_count(e$N)), fmt("No. of categories", stata_tab_count(e$k_absorb)),
             fmt(sprintf("F(%d, %d)", as.integer(e$df_m), as.integer(e$df_r)), sprintf("%.2f", e$F)),
             fmt("Prob > F", sprintf("%.4f", e$p)), fmt("R-squared", sprintf("%.4f", e$r2)),
             fmt("Adj R-squared", sprintf("%.4f", e$r2_a)), fmt("Root MSE", sprintf("%.4f", e$rmse)))
  cat("\n")
  for (i in seq_along(right)) {
    l <- if (i <= length(left)) left[i] else ""
    cat(formatC(l, width = -(78 - nchar(right[i]))), right[i], "\n", sep = "")
  }
  cat("\n")
  if (!is.null(e[["N_clust"]])) {
    cat(pad(sprintf("(Std. err. adjusted for %s clusters in %s)", stata_tab_count(e$N_clust), e$clustvar), 78), "\n", sep = "")
  }
  nm <- colnames(e$b)
  b <- stats::setNames(as.vector(e$b), nm)
  se <- stats::setNames(sqrt(diag(e$V)), nm)
  df <- e$df_r
  q <- stats::qt((1 + level / 100) / 2, df)
  rows <- stata_coef_rows(nm, data)
  if (is.null(rows)) stop("RCraft: areg with a term whose table layout was not checked is not translated yet", call. = FALSE)
  cat(strrep("-", 78), "\n", sep = "")
  if (robust) cat("             |               Robust\n")
  cat(pad(stata_tab_abbrev(e$depvar), 12), " | Coefficient  ", if (robust) "std. err." else "Std. err.",
      "      t    P>|t|     [", format(level), "% conf. interval]\n", sep = "")
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
        pad(sprintf("%.3f", p), 8), pad(stata_tab_g(bi - q * si), 13), pad(stata_tab_g(bi + q * si), 12), "\n", sep = "")
  }
  cat(strrep("-", 78), "\n", sep = "")
  if (!robust) {
    lf <- sprintf("F test of absorbed indicators: F(%d, %d) = %s", as.integer(e$df_a), as.integer(e$df_r),
                  sprintf("%5.3f", e$F_absorb))
    pr <- sprintf("Prob > F = %5.3f", e$p_absorb)
    cat(lf, strrep(" ", max(1, 78 - nchar(lf) - nchar(pr))), pr, "\n", sep = "")
  }
  invisible(NULL)
}
# predict after areg: xb, residuals, d (the absorbed effect), xbd
stata_areg_predict <- function(e, d, what) {
  a <- attr(e, "stata_areg")
  b <- e$b
  nm <- colnames(b)
  mf <- stats::model.frame(a$fo, d, xlev = a$xlev, na.action = stats::na.pass)
  X <- stats::model.matrix(a$fo, mf)
  # the columns kept (one omitted for collinearity is not in the prediction)
  X <- X[, a$cols, drop = FALSE]
  xb <- drop(X %*% b[1, a$nmx]) + b[1, "_cons"]
  g <- as.character(d[[a$absorb]])
  dd <- unname(a$dg[g])
  switch(what, xb = xb, d = dd, xbd = xb + dd,
         residuals = { yv <- unclass(d[[e$depvar]]); attributes(yv) <- NULL; as.numeric(yv) - xb - dd })
}"##;

impl Session {
    /// The command whose results are active, when it is `areg`.
    pub(super) fn areg_active(&self) -> bool {
        matches!(
            &self.e_ret,
            super::returned::Returned::Program { cmd, .. } if cmd == "areg"
        ) && self.estimates.as_ref().is_some_and(|e| e.fit == Fit::Areg)
    }

    /// `areg depvar indepvars [if] [in] [weight], absorb(var) [vce() robust cluster() level()]`.
    pub(super) fn areg(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "areg")?;
        if let Some(o) = opts.other(&[
            ("absorb", 2),
            ("robust", 1),
            ("vce", 3),
            ("cluster", 2),
            ("level", 3),
        ]) {
            return Err(no(format!(
                "areg with the option {o} is not translated yet"
            )));
        }
        let Some(absorb) = opts.arg("absorb", 2) else {
            out.line("stop(\"option absorb() required (Stata: r(198))\", call. = FALSE)");
            return Ok(());
        };
        let absorb = absorb.trim().to_string();
        if !is_name(&absorb) {
            return Err(no("areg, absorb() takes one plain variable name"));
        }
        let spec = Spec::read(c, "areg")?;
        if !is_name(&spec.dep) || spec.terms.is_empty() {
            return Err(no(
                "areg with operators or no independent variable is not translated yet",
            ));
        }
        // plain variables and factor variables (i.g), as R's factor(g)
        let plain = |t: &str| {
            is_name(t)
                || (t.starts_with("factor(") && t.ends_with(')') && is_name(&t[7..t.len() - 1]))
        };
        if spec.terms.iter().any(|t| !plain(t)) {
            return Err(no(
                "areg with interactions, ib. bases or operators is not translated yet",
            ));
        }
        let w = self.weight_of(c, &[W::F, W::A, W::P, W::I])?;
        if let Some((W::I, _)) = &w {
            out.line("stop(\"iweights not allowed (Stata: r(101))\", call. = FALSE)");
            return Ok(());
        }
        let level = level(opts)?;
        let (mut robust, mut cluster) = vce(opts)?;
        if let Some(cl) = opts.arg("cluster", 2) {
            if cluster.is_some() {
                return Err(no(
                    "areg with both cluster() and vce(cluster) is not translated yet",
                ));
            }
            let cl = cl.trim().to_string();
            if !is_name(&cl) {
                return Err(no("areg, cluster() takes one plain variable name"));
            }
            cluster = Some(cl);
            robust = true;
        }
        let rows = self.rows_for(c, out)?;
        let src = self.weighted_rows(rows, &w);
        for h in [
            "stata_lsfam",
            "stata_areg",
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
        let cl = match &cluster {
            Some(v) => format!(", cluster = {}", r_text(v)),
            None => String::new(),
        };
        let show = !self.quiet(c);
        out.line(format!(
            "stata_e <- stata_areg({src}, {}, c({}), {}{wargs}, robust = {}{cl}, level = {}, cmdline = {}, show = {})",
            r_text(&spec.dep),
            spec.terms
                .iter()
                .map(|t| r_text(t))
                .collect::<Vec<_>>()
                .join(", "),
            r_text(&absorb),
            super::stats::r_logical(robust),
            r_number(level * 100.0),
            r_text(&super::returned::cmdline(c)),
            super::stats::r_logical(show)
        ));
        self.areg_after(&spec, &src, out)?;
        if w.is_some() {
            out.caveat(super::weights::weight_caveat(&w));
        }
        out.caveat(
            "areg as Stata 19: the within fit with the grand means put back, N - k - G residual degrees of \
             freedom, R-squared of the model with the indicators",
        );
        Ok(())
    }

    fn areg_after(&mut self, spec: &Spec, src: &str, out: &mut Out) -> Result<(), Untranslated> {
        out.line("stata_b <- attr(stata_e, \"stata_b\")");
        out.line("stata_vcov <- attr(stata_e, \"stata_V\")");
        out.line("stata_df <- stata_e$df_r");
        out.line("stata_estimates <- list(rows = attr(stata_e, \"stata_rows\"))");
        out.line(self.keep_rows(src, "stata_estimates$rows"));
        let d = self.data()?;
        out.package("dplyr");
        out.line(super::model::esample_rows_line(
            &d,
            src,
            "stata_estimates$rows",
        ));
        self.esample_new = true;
        self.fit_robust = false;
        self.fit_cluster = false;
        self.estimates = Some(Estimates {
            fit: Fit::Areg,
            dep: spec.dep.clone(),
            factors: spec.factors.clone(),
        });
        self.e_new = true;
        self.e_ret = super::returned::Returned::Program {
            cmd: "areg".into(),
            texts: [
                "cmdline",
                "vcetype",
                "cmd",
                "footnote",
                "title",
                "predict",
                "marginsnotok",
                "vce",
                "clustvar",
                "depvar",
                "properties",
                "model",
                "absvar",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
        };
        Ok(())
    }

    /// `predict [type] newvar [if] [in] [, xb residuals d xbd]` after `areg`.
    pub(super) fn areg_predict(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[("xb", 2), ("residuals", 1), ("d", 1), ("xbd", 3)]) {
            return Err(no(format!(
                "predict after areg with the option {o} is not translated yet"
            )));
        }
        if c.options.len() > 1 {
            return Err(no("predict takes one statistic at a time"));
        }
        let what = if opts.has("residuals", 1) {
            "residuals"
        } else if opts.has("xbd", 3) {
            "xbd"
        } else if opts.has("d", 1) {
            "d"
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
        if !c.options.iter().any(|_| true) && !self.quiet(c) {
            out.line("cat(\"(option xb assumed; fitted values)\\n\")");
        }
        self.helpers.insert("stata_lsfam");
        self.helpers.insert("stata_areg");
        let v = format!("stata_areg_predict(stata_e, {base}, {})", r_text(what));
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
