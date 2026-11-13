//! `pdslasso depvar d (controls) [if] [in], robust` (Ahrens, Hansen and
//! Schaffer's pdslasso on SSC, with lassopack's rlasso), as Stata 19 runs it,
//! matched to its output (made-up data and ZNMIXH's own; its code and help
//! were not read, see `plan/stata/pdslasso.md`).
//!
//! The rigorous lasso, heteroskedastic loadings: the first residuals from OLS
//! on the five controls most correlated with the variable; then twice
//! sigma = sqrt(mean(e^2)), psi_j = sqrt(mean(x_j^2 e^2)) / sigma (x_j
//! centred), lambda = 2 c sqrt(n) qnorm(1 - gamma / (2p)) sigma (c = 1.1,
//! gamma = 0.1 / log(n)), the lasso of (1/n) RSS + (lambda/n) sum psi_j |b_j|
//! with the constant unpenalized, and the next residuals from OLS on what it
//! selected. Controls are selected for the dependent variable and for d; e(b)
//! is OLS of y on d, their union and the constant, its e(V) the
//! heteroskedasticity-robust HC0; the two CHS estimates are also shown.

use super::*;

pub(super) const PDSLASSO_HELPER: &str = r##"stata_rlasso_cd <- function(Xc, yc, pen, tol = 1e-12, maxit = 100000) {
  b <- numeric(ncol(Xc)); r <- yc; ss <- colSums(Xc^2)
  for (it in seq_len(maxit)) {
    bmax <- 0
    for (j in seq_along(b)) {
      if (ss[j] == 0) next
      rj <- r + Xc[, j] * b[j]
      z <- sum(Xc[, j] * rj)
      nb <- sign(z) * max(abs(z) - pen[j] / 2, 0) / ss[j]
      bmax <- max(bmax, abs(nb - b[j]))
      r <- rj - Xc[, j] * nb
      b[j] <- nb
    }
    if (bmax < tol) break
  }
  b
}
# the rigorous lasso of y on X with heteroskedastic loadings: what it selects and its fit
stata_rlasso <- function(X, y, npsi = 2) {
  n <- nrow(X); p <- ncol(X)
  Xc <- sweep(X, 2, colMeans(X)); yc <- y - mean(y)
  lam0 <- 2 * 1.1 * sqrt(n) * qnorm(1 - (0.1 / log(n)) / (2 * p))
  cr <- suppressWarnings(abs(stats::cor(X, y)))
  top <- utils::head(order(-cr, na.last = NA), 5)
  e <- if (length(top)) stats::residuals(stats::lm(y ~ X[, top, drop = FALSE])) else yc
  b <- numeric(p)
  for (k in seq_len(npsi)) {
    s <- sqrt(mean(e^2))
    psi <- sqrt(colMeans(Xc^2 * e^2)) / s
    b <- stata_rlasso_cd(Xc, yc, lam0 * s * psi)
    sel <- which(b != 0)
    e <- if (length(sel)) stats::residuals(stats::lm(y ~ X[, sel, drop = FALSE])) else yc
  }
  sel <- which(b != 0)
  list(sel = sel, b = b, cons = mean(y) - sum(colMeans(X) * b), post = e)
}
# a name as Stata's coefficient tables abbreviate it: the first characters, ~ and as
# many of the last as make it match one variable of the data alone
stata_abbrev_unique <- function(s, vars, n = 12) {
  if (nchar(s) <= n) return(s)
  for (k in 1:(n - 2)) {
    pre <- substr(s, 1, n - 1 - k); suf <- substr(s, nchar(s) - k + 1, nchar(s))
    hit <- vars[startsWith(vars, pre) & endsWith(vars, suf) & nchar(vars) >= nchar(pre) + nchar(suf)]
    if (length(hit) <= 1) return(paste0(pre, "~", suf))
  }
  # no short form matches one variable alone: the first n - 2 and .. (Stata 19)
  paste0(substr(s, 1, n - 2), "..")
}
stata_pdslasso <- function(d, y, dv, ctl, show = TRUE) {
  for (v in c(y, dv, ctl)) if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
  num <- function(v) { u <- unclass(d[[v]]); attributes(u) <- NULL; as.numeric(u) }
  ok <- stats::complete.cases(as.data.frame(lapply(c(y, dv, ctl), num)))
  yv <- num(y)[ok]; Zv <- num(dv)[ok]
  X <- vapply(ctl, function(v) num(v)[ok], numeric(sum(ok)))
  if (!is.matrix(X)) X <- matrix(X, ncol = length(ctl), dimnames = list(NULL, ctl))
  n <- length(yv)
  live <- apply(X, 2, function(v) length(unique(v)) > 1)
  Xl <- X[, live, drop = FALSE]
  ry <- stata_rlasso(Xl, yv); rz <- stata_rlasso(Xl, Zv)
  sy <- colnames(Xl)[ry$sel]; sz <- colnames(Xl)[rz$sel]
  sel <- ctl[ctl %in% union(sy, sz)]
  hc0 <- function(M, e) { B <- solve(crossprod(M)); B %*% crossprod(M * e) %*% B }
  M <- cbind(Z = Zv, X[, sel, drop = FALSE], `_cons` = 1)
  colnames(M)[1] <- dv
  b <- solve(crossprod(M), crossprod(M, yv))
  e <- as.numeric(yv - M %*% b)
  V <- hc0(M, e)
  bn <- colnames(M)
  bm <- matrix(as.numeric(b), 1, dimnames = list("y1", bn))
  dimnames(V) <- list(bn, bn)
  # the CHS estimates: y and d less their lasso fits, then less their post-lasso fits
  one <- function(u, w) { bb <- sum(w * u) / sum(w^2); ee <- u - w * bb; c(bb, sqrt(sum(w^2 * ee^2)) / sum(w^2)) }
  chs_l <- one(yv - (ry$cons + Xl %*% ry$b), Zv - (rz$cons + Xl %*% rz$b))
  chs_pl <- one(ry$post, rz$post)
  if (show) {
    vars <- names(d)
    ab <- function(s) stata_abbrev_unique(s, vars)
    g9 <- function(v) stata_fmt(v, "%9.0g")
    rule <- strrep("-", 78)
    wrap <- function(w) {
      lines <- character(); cur <- ""
      for (x in w) {
        if (!nzchar(cur)) cur <- x else if (nchar(cur) + 1 + nchar(x) <= 40) cur <- paste(cur, x) else { lines <- c(lines, cur); cur <- x }
      }
      c(lines, cur)
    }
    item <- function(label, w) {
      l <- wrap(w)
      cat(sprintf("%-39s%s\n", label, l[1]))
      for (x in l[-1]) cat(strrep(" ", 39), x, "\n", sep = "")
    }
    table <- function(title, nm, bb, se) {
      cat(title, "\n", rule, "\n", sep = "")
      cat("             |               Robust\n")
      cat(sprintf("%12s | Coefficient  std. err.      z    P>|z|     [95%% conf. interval]\n", ab(y)))
      cat(strrep("-", 13), "+", strrep("-", 64), "\n", sep = "")
      for (i in seq_along(nm)) {
        z <- bb[i] / se[i]; q <- stats::qnorm(0.975)
        cat(sprintf("%12s | %10s %10s %8.2f %7.3f %12s %11s\n", if (nm[i] == "_cons") nm[i] else ab(nm[i]),
                    g9(bb[i]), g9(se[i]), z, 2 * stats::pnorm(-abs(z)), g9(bb[i] - q * se[i]), g9(bb[i] + q * se[i])))
      }
      cat(rule, "\n", sep = "")
    }
    cat("1.  (PDS/CHS) Selecting HD controls for dep var ", y, "...\n", sep = "")
    cat("Selected: ", paste(ctl[ctl %in% sy], collapse = " "), "\n", sep = "")
    cat("2.  (PDS/CHS) Selecting HD controls for exog regressor ", dv, "...\n", sep = "")
    cat("Selected: ", paste(ctl[ctl %in% sz], collapse = " "), "\n", sep = "")
    cat("\n\nEstimation results:\n\nSpecification:\n")
    cat(sprintf("%-39s%s\n", "Regularization method:", "lasso"))
    cat(sprintf("%-39s%s\n", "Penalty loadings:", "heteroskedastic"))
    cat(sprintf("%-39s%s\n", "Number of observations:", n))
    item("Exogenous (1):", dv)
    item(sprintf("High-dim controls (%d):", length(ctl)), ctl)
    item(sprintf("Selected controls (%d):", length(sel)), sel)
    item("Unpenalized controls (1):", "_cons")
    cat("\nStructural equation:\n\n")
    table("OLS using CHS lasso-orthogonalized vars", dv, chs_l[1], chs_l[2])
    cat("\n")
    table("OLS using CHS post-lasso-orthogonalized vars", dv, chs_pl[1], chs_pl[2])
    cat("\n")
    table("OLS with PDS-selected variables and full regressor set", bn, as.numeric(b), sqrt(diag(V)))
    cat("Standard errors and test statistics valid for the following variables only:\n    ", dv, "\n", rule, "\n", sep = "")
  }
  e <- list(N = n, cmd = "pdslasso", depvar = y, dexog = dv, vcetype = "Robust", vce = "robust",
            method = "lasso", post = "pds", pload = "heteroskedastic",
            xselected = paste(sel, collapse = " "), xselected_y = paste(c(ctl[ctl %in% sy], "_cons"), collapse = " "),
            xselected_dex = paste(c(ctl[ctl %in% sz], "_cons"), collapse = " "), b = bm, V = V)
  attr(e, "stata_sample") <- FALSE
  e
}"##;

impl Session {
    /// `pdslasso depvar d (controls) [if] [in], robust`.
    pub(super) fn pdslasso(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[("robust", 1)]) {
            return Err(no(format!(
                "pdslasso with the option {o} is not translated yet"
            )));
        }
        if !opts.has("robust", 1) {
            return Err(no(
                "pdslasso without robust (homoskedastic loadings) is not translated yet",
            ));
        }
        if c.parts.weight.is_some() {
            return Err(no("pdslasso with weights is not translated yet"));
        }
        let main = c.parts.main.trim();
        let (head, rest) = main
            .split_once('(')
            .ok_or_else(|| no("pdslasso needs its high-dimensional controls in parentheses"))?;
        let (inside, after) = rest
            .split_once(')')
            .ok_or_else(|| no("pdslasso: a ( without its )"))?;
        if !after.trim().is_empty() {
            return Err(no(
                "pdslasso with words after the controls is not translated yet",
            ));
        }
        let words: Vec<&str> = head.split_whitespace().collect();
        let [dep, d] = words.as_slice() else {
            return Err(no(
                "pdslasso is translated with one dependent variable and one regressor of interest",
            ));
        };
        let ctl: Vec<&str> = inside.split_whitespace().collect();
        if ctl.is_empty() {
            return Err(no("pdslasso without controls is not translated yet"));
        }
        if let Some(v) = [*dep, *d].iter().chain(ctl.iter()).find(|v| !is_name(v)) {
            return Err(no(format!(
                "pdslasso {v}: variable names written out are translated (not ranges or patterns)"
            )));
        }
        let rows = self.rows_for(c, out)?;
        for h in [
            "stata_pdslasso",
            "stata_fmt",
            "stata_returned",
            "stata_eclass",
        ] {
            self.helpers.insert(h);
        }
        out.line(format!(
            "stata_e <- stata_pdslasso({rows}, {}, {}, c({}), show = {})",
            r_text(dep),
            r_text(d),
            ctl.iter().map(|v| r_text(v)).collect::<Vec<_>>().join(", "),
            super::stats::r_logical(!self.quiet(c))
        ));
        self.estimates = None;
        self.estimates_lost =
            Some("predict, test, lincom and margins are not translated after pdslasso yet".into());
        self.active_store = None;
        self.e_text.clear();
        self.esample = false;
        self.e_ret = super::returned::Returned::Program {
            cmd: "pdslasso".into(),
            texts: [
                "cmd",
                "depvar",
                "dexog",
                "vcetype",
                "vce",
                "method",
                "post",
                "pload",
                "xselected",
                "xselected_y",
                "xselected_dex",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
        };
        out.caveat("pdslasso as the SSC package runs it, matched to its output (not its code)");
        Ok(())
    }
}
