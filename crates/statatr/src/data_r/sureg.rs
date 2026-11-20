//! `sureg` and `mvreg`, the systems of least-squares equations, as Stata 19
//! prints and leaves them (`plan/stata/sureg.md`).
//!
//! - `mvreg`: each equation by least squares on the same regressors; e(Sigma)
//!   = E'E/(N - k) (weights: aweights scaled to sum to N, fweights count N),
//!   V = Sigma kron (X'X)^-1, t tests on N - k degrees of freedom.
//! - `sureg`: two-step feasible GLS. Sigma from the equations' least-squares
//!   residuals (cross products over N, or over sqrt((N - ki)(N - kj)) with
//!   `dfk`), then b = (X'(Sigma^-1 kron W)X)^-1 X'(Sigma^-1 kron W)y and V the
//!   inverse. `isure` goes round again with the residuals of the last fit
//!   until no coefficient moves by 1e-6 (relative to its size plus 1).
//!   `small` gives F and t on N*M - k degrees of freedom.

use super::model::{Estimates, Fit};
use super::weights::W;
use super::*;

/// The fits, the tables and what they leave in e().
pub(super) const SUREG_HELPER: &str = r##"# the rows, equations' columns and weights both commands use
stata_ls_prep <- function(d, eqs, noconst, wvar, wtype) {
  num <- function(u) { u <- unclass(u); attributes(u) <- NULL; as.numeric(u) }
  vars <- unique(unlist(lapply(eqs, function(q) c(q$dep, q$xs))))
  for (v in c(vars, wvar)) if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
  for (v in c(vars, wvar)) if (is.character(d[[v]])) stop(v, ": string variable, type mismatch (Stata: r(109))", call. = FALSE)
  ok <- stats::complete.cases(d[unique(c(vars, wvar))])
  if (!any(ok)) stop("no observations (Stata: r(2000))", call. = FALSE)
  rows <- which(ok)
  n <- length(rows)
  w <- if (is.null(wvar)) rep(1, n) else num(d[[wvar]])[rows]
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
  mk <- function(xs) {
    X <- vapply(xs, function(v) num(d[[v]])[rows], numeric(n))
    X <- matrix(X, n, length(xs), dimnames = list(NULL, xs))
    if (!noconst) X <- cbind(X, `_cons` = 1)
    X
  }
  list(rows = rows, n = n, w = w, wn = wn, N = Nn,
       X = lapply(eqs, function(q) mk(q$xs)),
       y = lapply(eqs, function(q) num(d[[q$dep]])[rows]))
}
# the equation table of a multi-equation fit, as Stata prints it
stata_ls_eqtable <- function(eqnames, nm, b, se, df, level) {
  pad <- stata_tab_pad
  q <- if (is.finite(df)) stats::qt((1 + level / 100) / 2, df) else stats::qnorm((1 + level / 100) / 2)
  cat(strrep("-", 78), "\n", sep = "")
  cat("             | Coefficient  Std. err.", if (is.finite(df)) "      t    P>|t|" else "      z    P>|z|",
      "     [", format(level), "% conf. interval]\n", sep = "")
  rule <- paste0(strrep("-", 13), "+", strrep("-", 64), "\n")
  cat(rule)
  eq <- rep(eqnames, vapply(nm, length, 1L))
  k <- 0L
  for (i in seq_along(eqnames)) {
    if (i > 1) cat(rule)
    cat(formatC(stata_tab_abbrev(eqnames[i]), width = -12), " |\n", sep = "")
    for (j in seq_along(nm[[i]])) {
      k <- k + 1L
      z <- b[k] / se[k]
      p <- if (is.finite(df)) 2 * stats::pt(-abs(z), df) else 2 * stats::pnorm(-abs(z))
      cat(pad(stata_tab_abbrev(nm[[i]][j]), 12), " |", pad(stata_tab_g(b[k]), 11), pad(stata_tab_g(se[k]), 11),
          pad(sprintf("%.2f", z), 9), pad(sprintf("%.3f", p), 8), pad(stata_tab_g(b[k] - q * se[k]), 13),
          pad(stata_tab_g(b[k] + q * se[k]), 12), "\n", sep = "")
    }
  }
  cat(strrep("-", 78), "\n", sep = "")
}
# the correlation matrix of residuals and Breusch-Pagan's test (colw: 8 for sureg, 9 for mvreg)
stata_ls_corr <- function(S, names, N, colw) {
  M <- length(names)
  R <- stats::cov2cor(S)
  lw <- max(nchar(names))
  colw <- max(colw, lw + 2L)
  cat("Correlation matrix of residuals:\n\n")
  cat(strrep(" ", lw), paste0(formatC(names, width = colw), collapse = ""), "\n", sep = "")
  for (i in seq_len(M)) {
    cat(formatC(names[i], width = lw), paste0(sprintf(paste0("%", colw, ".4f"), R[i, seq_len(i)]), collapse = ""), "\n", sep = "")
  }
  chi2 <- N * sum(R[lower.tri(R)]^2)
  q <- M * (M - 1) / 2
  cat(sprintf("\nBreusch–Pagan test of independence: chi2(%d) = %9.3f, Pr = %6.4f\n", as.integer(q), chi2,
              stats::pchisq(chi2, q, lower.tail = FALSE)))
}
stata_mvreg <- function(d, deps, xs, noconst = FALSE, wvar = NULL, wtype = NULL, level = 95, corr = FALSE,
                        noheader = FALSE, notable = FALSE, cmdline = "", show = TRUE) {
  eqs <- lapply(deps, function(v) list(dep = v, xs = xs))
  p <- stata_ls_prep(d, eqs, noconst, wvar, wtype)
  X <- p$X[[1]]
  k <- ncol(X)
  M <- length(deps)
  N <- p$N
  wn <- p$wn
  Y <- do.call(cbind, p$y)
  A <- crossprod(X * wn, X)
  Ainv <- solve(A)
  B <- Ainv %*% crossprod(X * wn, Y)
  E <- Y - X %*% B
  dfr <- N - k
  Sigma <- crossprod(E * wn, E) / dfr
  dimnames(Sigma) <- list(deps, deps)
  cons <- !noconst
  nmv <- colnames(X)
  b <- as.vector(B)
  names(b) <- paste0(rep(deps, each = k), ":", rep(nmv, M))
  V <- kronecker(Sigma, Ainv)
  dimnames(V) <- list(names(b), names(b))
  rmse <- sqrt(diag(Sigma))
  ybar <- if (cons) colSums(Y * wn) / sum(wn) else rep(0, M)
  tss <- vapply(seq_len(M), function(i) sum(wn * (Y[, i] - ybar[i])^2), 0)
  rss <- colSums(E^2 * wn)
  r2 <- 1 - rss / tss
  dfm <- k - cons
  Fv <- ((tss - rss) / dfm) / (rss / dfr)
  pF <- stats::pf(Fv, dfm, dfr, lower.tail = FALSE)
  if (show) {
    if (!noheader) {
      cat("\n")
      cat("Equation             Obs   Parms        RMSE    \"R-sq\"          F      P>F\n")
      cat(strrep("-", 74), "\n", sep = "")
      for (i in seq_len(M)) {
        cat(formatC(deps[i], width = -12), formatC(stata_tab_count(N), width = 12), formatC(as.character(k), width = 8),
            formatC(stata_fmt(rmse[i], "%9.0g", trim = TRUE), width = 12), formatC(sprintf("%.4f", r2[i]), width = 10),
            formatC(stata_fmt(Fv[i], "%9.0g", trim = TRUE), width = 11), formatC(sprintf("%.4f", pF[i]), width = 9), "\n", sep = "")
      }
    }
    if (!notable) {
      cat("\n")
      stata_ls_eqtable(deps, lapply(seq_len(M), function(i) nmv), b, sqrt(diag(V)), dfr, level)
    }
    if (corr) {
      cat("\n")
      stata_ls_corr(Sigma, deps, N, 9)
    }
  }
  e <- list(k = k, df_r = dfr, k_eq = M, N = N, rank = M * k)
  txt <- function(v) paste0(" ", paste(v, collapse = " "))
  e$cmd <- "mvreg"
  e$cmdline <- cmdline
  e$estat_cmd <- "mvreg_estat"
  if (!is.null(wtype)) {
    e$wexp <- paste0("= ", wvar)
    e$wtype <- c(f = "fweight", a = "aweight")[[wtype]]
  }
  e$marginsdefault <- paste0("predict(xb equation(", deps, "))", collapse = " ")
  e$marginsok <- "XB default"
  e$marginsnotok <- "stdp Residuals stddp"
  e$predict <- "reg3_p"
  e$depvar <- paste(deps, collapse = " ")
  e$eqnames <- paste(deps, collapse = " ")
  e$F <- txt(vapply(Fv, function(x) stata_fmt(x, "%9.0g", trim = TRUE), ""))
  e$rmse <- txt(vapply(rmse, function(x) stata_fmt(x, "%9.0g", trim = TRUE), ""))
  e$p_F <- txt(sprintf("%.4f", pF))
  e$r2 <- txt(sprintf("%.4f", r2))
  e$properties <- "b V"
  e$b <- matrix(b, 1, length(b), dimnames = list("y1", names(b)))
  e$V <- V
  e$Sigma <- Sigma
  attr(e, "stata_rows") <- p$rows
  attr(e, "stata_b") <- b
  attr(e, "stata_V") <- V
  attr(e, "stata_df") <- dfr
  e
}
stata_sureg <- function(d, eqs, noconst = FALSE, wvar = NULL, wtype = NULL, level = 95, corr = FALSE, small = FALSE,
                        dfk = FALSE, isure = FALSE, noheader = FALSE, notable = FALSE, cmdline = "", show = TRUE) {
  p <- stata_ls_prep(d, eqs, noconst, wvar, wtype)
  M <- length(eqs)
  N <- p$N
  wn <- p$wn
  X <- p$X
  y <- p$y
  ki <- vapply(X, ncol, 1L)
  K <- sum(ki)
  deps <- vapply(eqs, function(q) q$dep, "")
  # each equation by least squares first
  b1 <- lapply(seq_len(M), function(i) drop(solve(crossprod(X[[i]] * wn, X[[i]]), crossprod(X[[i]] * wn, y[[i]]))))
  resid <- function(bl) vapply(seq_len(M), function(i) y[[i]] - drop(X[[i]] %*% bl[[i]]), numeric(p$n))
  sigma <- function(E) {
    S <- crossprod(E * wn, E)
    if (dfk) S <- S / sqrt(outer(N - ki, N - ki)) else S <- S / N
    S
  }
  gls <- function(S) {
    Si <- solve(S)
    A <- matrix(0, K, K)
    r <- numeric(K)
    off <- c(0L, cumsum(ki))
    for (i in seq_len(M)) {
      for (j in seq_len(M)) {
        A[off[i] + seq_len(ki[i]), off[j] + seq_len(ki[j])] <- Si[i, j] * crossprod(X[[i]] * wn, X[[j]])
        r[off[i] + seq_len(ki[i])] <- r[off[i] + seq_len(ki[i])] + Si[i, j] * drop(crossprod(X[[i]] * wn, y[[j]]))
      }
    }
    Ainv <- solve(A)
    list(b = drop(Ainv %*% r), V = Ainv)
  }
  E1 <- resid(b1)
  S1 <- sigma(E1)
  Sig <- S1
  f <- gls(S1)
  ic <- 0L
  if (isure) {
    repeat {
      ic <- ic + 1L
      bl <- split(f$b, rep(seq_len(M), ki))
      E <- resid(bl)
      Sn <- sigma(E)
      fn <- gls(Sn)
      tol <- max(abs(fn$b - f$b) / (abs(f$b) + 1))
      f <- fn
      Sig <- Sn
      if (tol < 1e-6) break
      if (ic >= 1000) stop("RCraft: isure did not settle in 1000 iterations", call. = FALSE)
    }
    ic <- 0L
  }
  off <- c(0L, cumsum(ki))
  bl <- split(f$b, rep(seq_len(M), ki))
  E <- resid(bl)
  nm <- lapply(X, colnames)
  b <- f$b
  names(b) <- unlist(lapply(seq_len(M), function(i) paste0(deps[i], ":", nm[[i]])))
  V <- f$V
  dimnames(V) <- list(names(b), names(b))
  cons <- !noconst
  rss <- colSums(E^2 * wn)
  tss <- vapply(seq_len(M), function(i) {
    yb <- if (cons) sum(wn * y[[i]]) / sum(wn) else 0
    sum(wn * (y[[i]] - yb)^2)
  }, 0)
  mss <- tss - rss
  r2 <- mss / tss
  small_dof <- small || dfk
  rmse <- sqrt(rss / if (small_dof) N - ki else N)
  dfm <- ki - cons
  chi2 <- vapply(seq_len(M), function(i) {
    s <- which(nm[[i]] != "_cons")
    idx <- off[i] + s
    drop(t(b[idx]) %*% solve(V[idx, idx, drop = FALSE], b[idx]))
  }, 0)
  dfr <- N * M - K
  stat <- if (small) chi2 / dfm else chi2
  pv <- if (small) stats::pf(stat, dfm, dfr, lower.tail = FALSE) else stats::pchisq(stat, dfm, lower.tail = FALSE)
  df <- if (small) dfr else Inf
  Efin <- E
  ll <- -N / 2 * (M * (1 + log(2 * pi)) + log(det(crossprod(Efin * wn, Efin) / N)))
  title <- if (isure) "Seemingly unrelated regression, iterated " else "Seemingly unrelated regression"
  if (show) {
    if (!noheader) {
      cat("\n", title, "\n", sep = "")
      cat(strrep("-", 78), "\n", sep = "")
      cat(if (small) "Equation             Obs   Params         RMSE  \"R-squared\"         F      P>F\n"
          else "Equation             Obs   Params         RMSE  \"R-squared\"      chi2   P>chi2\n")
      cat(strrep("-", 78), "\n", sep = "")
      for (i in seq_len(M)) {
        cat(formatC(deps[i], width = -12), formatC(stata_tab_count(N), width = 12), formatC(as.character(ki[i]), width = 9),
            formatC(stata_fmt(rmse[i], "%9.0g", trim = TRUE), width = 13), formatC(sprintf("%.4f", r2[i]), width = 12),
            formatC(sprintf("%.2f", stat[i]), width = 11), formatC(sprintf("%.4f", pv[i]), width = 9), "\n", sep = "")
      }
      cat(strrep("-", 78), "\n", sep = "")
      if (identical(wtype, "f")) cat("\n")
    }
    if (!notable) {
      cat("\n")
      stata_ls_eqtable(deps, nm, b, sqrt(diag(V)), df, level)
    }
    if (dfk) cat("Note: Small-sample degrees-of-freedom adjustment applied when estimating\n      covariance matrix of residuals.\n")
    if (corr) {
      cat("\n")
      stata_ls_corr(Sig, deps, N, 8)
    }
  }
  e <- list()
  for (i in seq_len(M)) {
    e[[paste0("cons_", i)]] <- as.numeric(cons)
    e[[paste0(if (small) "F_" else "chi2_", i)]] <- stat[i]
    e[[paste0("p_", i)]] <- pv[i]
    e[[paste0("df_m", i)]] <- dfm[i]
    e[[paste0("rss_", i)]] <- rss[i]
    e[[paste0("rmse_", i)]] <- rmse[i]
    e[[paste0("mss_", i)]] <- mss[i]
    e[[paste0("r2_", i)]] <- r2[i]
  }
  e$k <- K
  e$N <- N
  if (small) e$df_r <- dfr
  e$rank <- K
  e$ll <- ll
  e$dfk2_adj <- N
  e$k_eq <- M
  e$ic <- ic
  e$cmd <- "sureg"
  e$cmdline <- cmdline
  e$method <- if (isure) "isure" else "sure"
  e$vce <- "unadjusted"
  e$marginsdefault <- paste0("predict(xb equation(", deps, "))", collapse = " ")
  e$marginsok <- "XB default"
  e$marginsnotok <- "stdp Residuals stddp"
  e$predict <- "reg3_p"
  if (small) e$small <- "small"
  e$corr <- "unstructured"
  e$exog <- paste(unique(unlist(lapply(eqs, function(q) q$xs))), collapse = " ")
  e$depvar <- paste(deps, collapse = " ")
  if (!is.null(wtype)) {
    e$wexp <- paste0("= ", wvar)
    e$wtype <- c(f = "fweight", a = "aweight")[[wtype]]
  }
  e$eqnames <- paste(deps, collapse = " ")
  e$properties <- "b V"
  e$b <- matrix(b, 1, length(b), dimnames = list("y1", names(b)))
  e$V <- V
  dimnames(Sig) <- list(deps, deps)
  e$Sigma <- Sig
  attr(e, "stata_rows") <- p$rows
  attr(e, "stata_b") <- b
  attr(e, "stata_V") <- V
  attr(e, "stata_df") <- df
  e
}
# the equation a predict asks for: a name or #k, the first when none is given
stata_ls_eq <- function(e, spec = "") {
  eqs <- strsplit(e$eqnames, " ", fixed = TRUE)[[1]]
  if (!nzchar(spec)) return(eqs[1])
  if (startsWith(spec, "#")) {
    k <- suppressWarnings(as.integer(substring(spec, 2)))
    if (is.na(k) || k < 1L || k > length(eqs)) stop("equation(", spec, ") not found (Stata: r(303))", call. = FALSE)
    return(eqs[k])
  }
  if (!spec %in% eqs) stop("equation(", spec, ") not found (Stata: r(303))", call. = FALSE)
  spec
}"##;

/// Equation names a model gives its coefficients in `e(b)`.
pub(super) fn eq_stop(out: &mut Out, msg: &str) {
    out.line(format!("stop({}, call. = FALSE)", r_text(msg)));
}

impl Session {
    /// The command whose results are active, when it is `sureg` or `mvreg`.
    pub(super) fn sureg_active(&self) -> Option<String> {
        match &self.e_ret {
            super::returned::Returned::Program { cmd, .. } if cmd == "sureg" || cmd == "mvreg" => {
                Some(cmd.clone())
            }
            _ => None,
        }
    }

    /// `mvreg depvars = indepvars [if] [in] [weight] [, noconstant corr level() noheader
    /// notable]`.
    pub(super) fn mvreg(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "mvreg")?;
        if let Some(o) = opts.other(&[
            ("noconstant", 3),
            ("corr", 4),
            ("level", 3),
            ("noheader", 3),
            ("notable", 4),
        ]) {
            return Err(no(format!(
                "mvreg with the option {o} is not translated yet"
            )));
        }
        let Some((left, right)) = c.parts.main.split_once('=') else {
            eq_stop(out, "= required (Stata: r(198))");
            return Ok(());
        };
        let deps: Vec<&str> = left.split_whitespace().collect();
        let xs: Vec<&str> = right.split_whitespace().collect();
        if deps.is_empty() || xs.is_empty() {
            return Err(no("mvreg needs dependent and independent variables"));
        }
        if deps.iter().chain(xs.iter()).any(|v| !is_name(v)) {
            return Err(no(
                "mvreg with factor variables, operators or abbreviations is not translated yet",
            ));
        }
        if deps.len() < 2 {
            return Err(no(
                "mvreg with one dependent variable is not translated yet",
            ));
        }
        let w = self.weight_of(c, &[W::F, W::A, W::P, W::I])?;
        if let Some((k @ (W::P | W::I), _)) = &w {
            eq_stop(
                out,
                &format!(
                    "{} not allowed (Stata: r(101))",
                    if *k == W::P { "pweights" } else { "iweights" }
                ),
            );
            return Ok(());
        }
        let level = super::model::level(opts)?;
        let rows = self.rows_for(c, out)?;
        let src = self.weighted_rows(rows, &w);
        self.sureg_helpers();
        let wargs = match &w {
            Some((k, v)) => format!(", wvar = {}, wtype = {}", r_text(v), r_text(k.code())),
            None => String::new(),
        };
        let q = |v: &[&str]| v.iter().map(|s| r_text(s)).collect::<Vec<_>>().join(", ");
        let show = !self.quiet(c);
        out.line(format!(
            "stata_e <- stata_mvreg({src}, c({}), c({}), noconst = {}{wargs}, level = {}, corr = {}, noheader = {}, notable = {}, cmdline = {}, show = {})",
            q(&deps),
            q(&xs),
            super::stats::r_logical(opts.has("noconstant", 3)),
            r_number(level * 100.0),
            super::stats::r_logical(opts.has("corr", 4)),
            super::stats::r_logical(opts.has("noheader", 3)),
            super::stats::r_logical(opts.has("notable", 4)),
            r_text(&super::returned::cmdline(c)),
            super::stats::r_logical(show)
        ));
        self.sureg_after(c, "mvreg", deps.first().copied().unwrap_or(""), &src, out)?;
        if w.is_some() {
            out.caveat(super::weights::weight_caveat(&w));
        }
        out.caveat(
            "mvreg as Stata 19: each equation by least squares, Sigma = E'E/(N - k), V = Sigma kron (X'X)^-1",
        );
        Ok(())
    }

    /// `sureg (depvar varlist) (depvar varlist) ... [if] [in] [weight] [, small dfk isure corr
    /// noconstant level() nolog noheader notable]`.
    pub(super) fn sureg(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "sureg")?;
        if opts.has("noconstant", 3) {
            eq_stop(out, "option noconstant not allowed (Stata: r(198))");
            return Ok(());
        }
        if let Some(o) = opts.other(&[
            ("corr", 4),
            ("level", 3),
            ("noheader", 3),
            ("notable", 4),
            ("small", 5),
            ("dfk", 3),
            ("isure", 3),
            ("nolog", 5),
        ]) {
            return Err(no(format!(
                "sureg with the option {o} is not translated yet"
            )));
        }
        let main = c.parts.main.trim();
        let mut eqs: Vec<(String, Vec<String>)> = Vec::new();
        let mut rest = main;
        while !rest.trim().is_empty() {
            let r = rest.trim_start();
            let Some(r) = r.strip_prefix('(') else {
                return Err(no(
                    "sureg takes each equation in parentheses, (y x1 x2) (z x1)",
                ));
            };
            let Some((inner, after)) = r.split_once(')') else {
                return Err(no("sureg: an equation's parenthesis is not closed"));
            };
            if inner.contains([':', '=']) {
                return Err(no(
                    "sureg equation names (name: y x) and the y1 y2 = x form are not translated yet",
                ));
            }
            let words: Vec<&str> = inner.split_whitespace().collect();
            let Some((dep, xs)) = words.split_first() else {
                return Err(no("sureg: an equation without variables"));
            };
            if words.iter().any(|v| !is_name(v)) {
                return Err(no(
                    "sureg with factor variables, operators or abbreviations is not translated yet",
                ));
            }
            eqs.push((dep.to_string(), xs.iter().map(|s| s.to_string()).collect()));
            rest = after;
        }
        if eqs.len() < 2 {
            return Err(no("sureg with one equation is not translated yet"));
        }
        if eqs.iter().any(|(_, xs)| xs.is_empty()) {
            return Err(no(
                "sureg with an equation that has no regressor is not translated yet",
            ));
        }
        let mut seen: Vec<&str> = Vec::new();
        for (dep, _) in &eqs {
            if seen.contains(&dep.as_str()) {
                return Err(no(
                    "sureg with the same dependent variable in two equations is not translated yet",
                ));
            }
            seen.push(dep);
        }
        let w = self.weight_of(c, &[W::F, W::A, W::P, W::I])?;
        if let Some((k @ (W::P | W::I), _)) = &w {
            eq_stop(
                out,
                &format!(
                    "{} not allowed (Stata: r(101))",
                    if *k == W::P { "pweights" } else { "iweights" }
                ),
            );
            return Ok(());
        }
        if opts.has("isure", 3) && !opts.has("nolog", 5) {
            return Err(no(
                "sureg, isure without nolog prints Stata's iteration log, which is not translated yet",
            ));
        }
        let level = super::model::level(opts)?;
        let rows = self.rows_for(c, out)?;
        let src = self.weighted_rows(rows, &w);
        self.sureg_helpers();
        let wargs = match &w {
            Some((k, v)) => format!(", wvar = {}, wtype = {}", r_text(v), r_text(k.code())),
            None => String::new(),
        };
        let lst = eqs
            .iter()
            .map(|(dep, xs)| {
                format!(
                    "list(dep = {}, xs = c({}))",
                    r_text(dep),
                    xs.iter().map(|s| r_text(s)).collect::<Vec<_>>().join(", ")
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        let show = !self.quiet(c);
        let flag = |n: &str, m: usize| super::stats::r_logical(opts.has(n, m));
        out.line(format!(
            "stata_e <- stata_sureg({src}, list({lst}), noconst = {}{wargs}, level = {}, corr = {}, small = {}, dfk = {}, isure = {}, noheader = {}, notable = {}, cmdline = {}, show = {})",
            flag("noconstant", 3),
            r_number(level * 100.0),
            flag("corr", 4),
            flag("small", 5),
            flag("dfk", 3),
            flag("isure", 3),
            flag("noheader", 3),
            flag("notable", 4),
            r_text(&super::returned::cmdline(c)),
            super::stats::r_logical(show)
        ));
        let first = eqs.first().map(|e| e.0.clone()).unwrap_or_default();
        self.sureg_after(c, "sureg", &first, &src, out)?;
        if w.is_some() {
            out.caveat(super::weights::weight_caveat(&w));
        }
        out.caveat(
            "sureg as Stata 19: two-step feasible GLS (Sigma from the equations' least-squares residuals), \
             V the inverse of X'(Sigma^-1 kron I)X",
        );
        Ok(())
    }

    fn sureg_helpers(&mut self) {
        for h in [
            "stata_lsfam",
            "stata_sureg",
            "stata_fmt",
            "stata_tables",
            "stata_returned",
            "stata_eclass",
        ] {
            self.helpers.insert(h);
        }
    }

    /// What stays in R after `sureg` or `mvreg`: the coefficients and their variance for
    /// `test`, the rows used and e().
    fn sureg_after(
        &mut self,
        c: &Command,
        cmd: &str,
        first: &str,
        src: &str,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        out.line("stata_b <- attr(stata_e, \"stata_b\")");
        out.line("stata_vcov <- attr(stata_e, \"stata_V\")");
        out.line("stata_df <- attr(stata_e, \"stata_df\")");
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
            fit: if cmd == "sureg" {
                Fit::Sureg
            } else {
                Fit::Mvreg
            },
            dep: first.to_string(),
            factors: BTreeMap::new(),
        });
        self.e_new = true;
        let mut texts: Vec<String> = [
            "cmd",
            "cmdline",
            "estat_cmd",
            "method",
            "vce",
            "marginsdefault",
            "marginsok",
            "marginsnotok",
            "predict",
            "small",
            "corr",
            "exog",
            "depvar",
            "wexp",
            "wtype",
            "eqnames",
            "F",
            "rmse",
            "p_F",
            "r2",
            "properties",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        texts.retain(|t| !(cmd == "sureg" && t == "estat_cmd"));
        self.e_ret = super::returned::Returned::Program {
            cmd: cmd.to_string(),
            texts,
        };
        let _ = c;
        Ok(())
    }

    /// `predict [type] newvar [if] [in] [, xb residuals equation(name | #k)]` after `sureg` or
    /// `mvreg`.
    pub(super) fn sureg_predict(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[("xb", 2), ("residuals", 1), ("equation", 2)]) {
            return Err(no(format!(
                "predict after {} with the option {o} is not translated yet",
                self.sureg_active().unwrap_or_default()
            )));
        }
        if opts.has("xb", 2) && opts.has("residuals", 1) {
            return Err(no("predict takes one statistic at a time"));
        }
        let eq = match opts.arg("equation", 2) {
            Some(e) => {
                let e = e.trim().to_string();
                if e.contains(',') || e.contains(' ') {
                    return Err(no("predict with several equations is not translated yet"));
                }
                e
            }
            None => String::new(),
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
        let quiet = self.quiet(c);
        let resid = opts.has("residuals", 1);
        self.helpers.insert("stata_lsfam");
        self.helpers.insert("stata_sureg");
        self.helpers.insert("stata_float");
        out.line(format!(
            "stata_eqn <- stata_ls_eq(stata_e, {})",
            r_text(&eq)
        ));
        if !resid && !opts.has("xb", 2) && !quiet {
            out.line("cat(\"(option xb assumed; fitted values)\\n\")");
        }
        let xb = format!("stata_ls_xb(stata_e, {base}, which = stata_eqn)");
        let v = if resid {
            format!("{base}[[stata_eqn]] - {xb}")
        } else {
            xb
        };
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
        out.line("rm(stata_eqn)");
        self.kinds.insert(w.to_string(), Kind::Number);
        self.storage.insert(w.to_string(), st);
        Ok(())
    }
}
