//! `cluster2` (Mitchell Petersen's add-on, not on SSC), as it behaves on
//! Stata 19 with made-up data (`plan/stata/cluster2.md`); only its results
//! were read, never its code.
//!
//! - The coefficients are least squares on the rows with no missing value in
//!   the model's variables.
//! - The variance is the one clustered by `fcluster()` plus the one clustered
//!   by `tcluster()` less the one clustered by their cells (firm-year), each
//!   with Stata's small-sample factor G/(G-1) (N-1)/(N-K). With one row per
//!   cell the last is the robust one, N/(N-K), which is the same thing.
//! - `e(F)` and `e(df_r)` are those of the last fit: robust (N-K) with one row
//!   per cell, else clustered by cell (cells - 1). The t tests use that df.
//! - A variance below 0 is kept in `e(V)`; its standard error shows as `.`.
//! - No `e(sample)`. A missing value in a cluster variable makes cluster2 fit
//!   its parts on different rows; RCraft stops there instead.

use super::model::{level, Estimates, Fit, Spec};
use super::*;

/// The fit, its variance, its e() and Stata's header and footer.
pub(super) const CLUSTER2_HELPER: &str = r#"stata_cluster2 <- function(formula, data, fcluster, tcluster) {
  vars <- unique(all.vars(formula))
  rows <- which(stats::complete.cases(data[vars]))
  d <- data[rows, , drop = FALSE]
  if (anyNA(d[[fcluster]]) || anyNA(d[[tcluster]]))
    stop("cluster2 with a missing value in fcluster() or tcluster() is not translated: its parts are then fitted on different rows", call. = FALSE)
  mf <- stats::model.frame(formula, d, na.action = stats::na.pass)
  y <- as.numeric(stats::model.response(mf))
  X <- stats::model.matrix(attr(mf, "terms"), mf)
  # the constant last, as Stata has it
  X <- cbind(X[, colnames(X) != "(Intercept)", drop = FALSE], "(Intercept)" = 1)
  N <- length(y)
  K <- ncol(X)
  if (qr(X)$rank < K) stop("cluster2 with collinear variables is not translated yet", call. = FALSE)
  B <- solve(crossprod(X))
  b <- drop(B %*% crossprod(X, y))
  names(b) <- colnames(X)
  e <- drop(y - X %*% b)
  clustered <- function(cl) {
    G <- length(unique(cl))
    B %*% crossprod(rowsum(X * e, cl)) %*% B * G / (G - 1) * (N - 1) / (N - K)
  }
  f <- as.character(d[[fcluster]])
  t <- as.character(d[[tcluster]])
  cell <- paste(f, t, sep = "\r")
  multiple <- anyDuplicated(cell) > 0
  if (multiple) {
    Vc <- clustered(cell)
    df_r <- length(unique(cell)) - 1
  } else {
    Vc <- B %*% crossprod(X * e) %*% B * N / (N - K)
    df_r <- N - K
  }
  V <- clustered(f) + clustered(t) - Vc
  dimnames(V) <- list(names(b), names(b))
  s <- seq_len(K - 1)
  Fs <- if (K > 1) drop(t(b[s]) %*% solve(Vc[s, s, drop = FALSE], b[s])) / (K - 1) else 0
  rss <- sum(e^2)
  tss <- sum((y - mean(y))^2)
  ll <- function(ss) -N / 2 * (1 + log(2 * pi * ss / N))
  fit <- stats::lm(formula, data = d)
  fit$stata <- list(N = N, df_m = K - 1, df_r = df_r, F = Fs, r2 = 1 - rss / tss, rmse = sqrt(rss / (N - K)),
                    mss = tss - rss, rss = rss, r2_a = 1 - (rss / (N - K)) / (tss / (N - 1)), ll = ll(rss),
                    ll_0 = ll(tss), G = c(length(unique(f)), length(unique(t))), names = c(fcluster, tcluster),
                    multiple = multiple)
  list(fit = fit, b = b, V = V, df = df_r, rows = rows)
}
stata_cluster2_header <- function(fit) {
  e <- fit$stata
  row <- function(left, right) cat(formatC(left, width = -55, flag = "-"), right, "\n", sep = "")
  p <- if (e$df_m > 0) sprintf("%7.4f", stats::pf(e$F, e$df_m, e$df_r, lower.tail = FALSE)) else "      ."
  cat(" \n")
  row("Linear regression with 2D clustered SEs", sprintf("Number of obs = %7d", e$N))
  row("", sprintf("F(%3d,%6d) = %7.2f", e$df_m, e$df_r, e$F))
  row("", paste0("Prob > F      = ", p))
  row(sprintf("Number of clusters (%s) = %7d", e$names[1], e$G[1]), sprintf("R-squared     = %7.4f", e$r2))
  row(sprintf("Number of clusters (%s) = %7d", e$names[2], e$G[2]), sprintf("Root MSE      = %7.4f", e$rmse))
  invisible(NULL)
}
stata_cluster2_footer <- function(fit) {
  e <- fit$stata
  cat(" \n     SE clustered by ", e$names[1], " and ", e$names[2],
      if (e$multiple) paste0(" (multiple obs per ", e$names[1], "-", e$names[2], ")"), "\n \n", sep = "")
  invisible(NULL)
}"#;

impl Session {
    /// `cluster2 depvar indepvars [if] [in], fcluster(v) tcluster(v)`.
    pub(super) fn cluster2(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "cluster2")?;
        if let Some(o) = opts.other(&[("fcluster", 8), ("tcluster", 8)]) {
            return Err(no(format!(
                "cluster2 with the option {o} is not translated yet (Stata: option {o} not allowed, r(198))"
            )));
        }
        if c.parts.weight.is_some() {
            return Err(no(
                "cluster2 does not take weights (Stata: weights not allowed, r(101))",
            ));
        }
        let mut cl = Vec::new();
        for o in ["fcluster", "tcluster"] {
            let v = opts
                .arg(o, 8)
                .map(|v| v.trim().to_string())
                .ok_or_else(|| {
                    no(format!(
                        "cluster2 needs {o}() (Stata: option {o}() required, r(198))"
                    ))
                })?;
            if !is_name(&v) {
                return Err(no(format!(
                    "cluster2, {o}({v}): one variable name is translated"
                )));
            }
            cl.push(v);
        }
        let [fcl, tcl] = cl.as_slice() else {
            return Err(no("cluster2 needs fcluster() and tcluster()"));
        };
        let spec = Spec::read(c, "cluster2")?;
        if !spec.factors.is_empty() {
            return Err(no("cluster2 with factor variables is not translated yet"));
        }
        let level = level(opts)?;
        let right = spec.right(false)?;
        let src = self.rows_for(c, out)?;
        self.helpers.insert("stata_cluster2");
        self.helpers.insert("stata_coefs");
        out.line(format!(
            "stata_m <- stata_cluster2({} ~ {right}, data = {src}, fcluster = {}, tcluster = {})",
            r_name(&spec.dep),
            r_text(fcl),
            r_text(tcl)
        ));
        out.line("stata_estimates <- stata_m$fit");
        out.line("stata_b <- stata_m$b");
        out.line("stata_vcov <- stata_m$V");
        out.line("stata_df <- stata_m$df");
        out.line(self.keep_rows(&src, "stata_m$rows"));
        // cluster2 leaves no e(sample): lines that read it are not translated
        self.fit_robust = true;
        self.fit_cluster = true;
        self.estimates = Some(Estimates {
            fit: Fit::Cluster2,
            dep: spec.dep.clone(),
            factors: spec.factors.clone(),
        });
        let args = super::returned::e_args(&spec.dep, "robust", None, &None);
        self.keep_e(c, out, &args, "");
        if !self.quiet(c) {
            out.line("stata_cluster2_header(stata_estimates)");
            out.line(format!(
                "stata_coefs(stata_b, stata_vcov, stata_df, level = {}, label = \"Coefficient\")",
                r_number(level)
            ));
            out.line("stata_cluster2_footer(stata_estimates)");
        }
        out.line("rm(stata_m)");
        out.caveat(
            "cluster2 (Petersen's add-on) as it behaves in Stata 19, matched to its results (not its code): \
             the variance clustered by each dimension less the one clustered by their cells",
        );
        out.caveat("the coefficient table is printed in R's layout, not Stata's");
        Ok(())
    }
}
