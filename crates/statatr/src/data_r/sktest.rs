//! `sktest varlist [if] [in] [fw aw] [, noadjust]`: the skewness and
//! kurtosis tests for normality of D'Agostino, Belanger and D'Agostino
//! (1990), with Royston's (1991c) adjustment to the joint test, as Stata's
//! manual ([R] sktest, Methods and formulas) gives them; the table and r()
//! as Stata 19 prints and leaves them (`plan/stata/sktest.md`).

use super::weights::{w_args, W};
use super::*;

pub(super) const SKTEST_HELPER: &str = r##"stata_sktest <- function(d, vars, adjust = TRUE, w = NULL, wtype = "f", show = TRUE) {
  num <- function(v) { u <- unclass(v); attributes(u) <- NULL; as.numeric(u) }
  for (v in vars) if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
  weighted <- !is.null(w)
  wt <- if (weighted) num(w) else rep(1, nrow(d))
  one <- function(x, v) {
    ok <- !is.na(x)
    x <- x[ok]
    v <- v[ok]
    k <- length(x)
    n <- if (weighted && wtype == "f") sum(v) else k
    out <- c(N = n, p_skew = NA_real_, p_kurt = NA_real_, chi2 = NA_real_, p_chi2 = NA_real_)
    if (n < 8) return(out)
    W <- sum(v)
    m <- sum(v * x) / W
    m2 <- sum(v * (x - m)^2) / W
    if (!is.finite(m2) || m2 <= 0) return(out)
    sk <- (sum(v * (x - m)^3) / W) / m2^1.5
    b2 <- (sum(v * (x - m)^4) / W) / m2^2
    # the skewness test
    Y <- sk * sqrt((n + 1) * (n + 3) / (6 * (n - 2)))
    beta2 <- 3 * (n^2 + 27 * n - 70) * (n + 1) * (n + 3) / ((n - 2) * (n + 5) * (n + 7) * (n + 9))
    W2 <- -1 + sqrt(2 * (beta2 - 1))
    delta <- 1 / sqrt(log(sqrt(W2)))
    alpha <- sqrt(2 / (W2 - 1))
    Z1 <- delta * log(Y / alpha + sqrt((Y / alpha)^2 + 1))
    # the kurtosis test
    E <- 3 * (n - 1) / (n + 1)
    V <- 24 * n * (n - 2) * (n - 3) / ((n + 1)^2 * (n + 3) * (n + 5))
    xk <- (b2 - E) / sqrt(V)
    sb1 <- 6 * (n^2 - 5 * n + 2) / ((n + 7) * (n + 9)) * sqrt(6 * (n + 3) * (n + 5) / (n * (n - 2) * (n - 3)))
    A <- 6 + 8 / sb1 * (2 / sb1 + sqrt(1 + 4 / sb1^2))
    t <- (1 - 2 / A) / (1 + xk * sqrt(2 / (A - 4)))
    Z2 <- (1 - 2 / (9 * A) - sign(t) * abs(t)^(1 / 3)) / sqrt(2 / (9 * A))
    K2 <- Z1^2 + Z2^2
    chi2 <- K2
    if (adjust) {
      Zc <- -stats::qnorm(exp(-K2 / 2))
      om <- 0.55 * n^0.2 - 0.21
      a1 <- (-5 + 3.46 * log(n)) * exp(-1.37 * log(n))
      b1 <- 1 + (0.854 - 0.148 * log(n)) * exp(-0.55 * log(n))
      a2 <- a1 - (2.13 / (1 - 2.37 * log(n))) * om
      b2c <- 2.13 / (1 - 2.37 * log(n)) + b1
      Z <- if (Zc < -1) Zc else if (Zc < om) a1 + b1 * Zc else a2 + b2c * Zc
      P <- stats::pnorm(-Z)
      chi2 <- -2 * log(P)
    }
    p_chi2 <- stats::pchisq(chi2, 2, lower.tail = FALSE)
    fin <- function(v) if (is.finite(v)) v else NA_real_
    c(N = n, p_skew = fin(2 * stats::pnorm(-abs(Z1))), p_kurt = fin(2 * stats::pnorm(-abs(Z2))),
      chi2 = fin(chi2), p_chi2 = if (is.finite(chi2)) p_chi2 else NA_real_)
  }
  tab <- t(vapply(vars, function(v) one(num(d[[v]]), wt), numeric(5)))
  dimnames(tab) <- list(vars, c("N", "p_skew", "p_kurt", "chi2", "p_chi2"))
  last <- tab[nrow(tab), ]
  r <- list(N = last[["N"]], p_skew = last[["p_skew"]], p_kurt = last[["p_kurt"]], chi2 = last[["chi2"]],
            p_chi2 = last[["p_chi2"]], table = tab)
  if (!show) return(r)
  # the names' column: as wide as the longest name where the line has room, else 12 (Stata 19)
  body <- if (adjust) 65 else 61
  ls <- if (exists("stata_linesize")) stata_linesize else 79
  sw <- max(12, min(max(nchar(vars)), ls - 2 - body))
  ab <- function(s) if (nchar(s) <= sw) s else paste0(substr(s, 1, sw - 2), "~", substr(s, nchar(s), nchar(s)))
  f <- function(v, d) if (is.na(v)) "." else sprintf(paste0("%.", d, "f"), v)
  cat("\nSkewness and kurtosis tests for normality\n")
  cat(strrep(" ", sw + 45), if (adjust) "----- Joint test -----" else "--- Joint test ---", "\n", sep = "")
  cat(sprintf("%*s", sw, "Variable"), " |       Obs   Pr(skewness)   Pr(kurtosis)   ",
      if (adjust) "Adj chi2(2)" else "chi2(2)", "  Prob>chi2\n", sep = "")
  cat(strrep("-", sw + 1), "+", strrep("-", body), "\n", sep = "")
  for (i in seq_len(nrow(tab))) {
    o <- tab[i, ]
    obs <- format(o[["N"]], big.mark = ",", scientific = FALSE)
    if (o[["N"]] < 8) {
      # too few observations: every test missing, the columns as Stata 19 sets them
      cat(sprintf("%*s |%10s%14s%15s%*s%11s", sw, ab(vars[i]), obs, ".", ".", if (adjust) 15 else 11, ".", "."), "\n", sep = "")
      next
    }
    cat(sprintf("%*s |%10s%15s%15s%*s%11s", sw, ab(vars[i]), obs, f(o[["p_skew"]], 4), f(o[["p_kurt"]], 4),
                if (adjust) 14 else 10, f(o[["chi2"]], 2), f(o[["p_chi2"]], 4)), "\n", sep = "")
  }
  invisible(r)
}"##;

/// The r() sktest leaves (Stata 19), the last variable's, and r(table).
const R_SKTEST: &[&str] = &["N", "p_skew", "p_kurt", "chi2", "p_chi2"];

impl Session {
    pub(super) fn sktest(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "sktest")?;
        if let Some(o) = opts.other(&[("noadjust", 8)]) {
            return Err(no(format!(
                "sktest with the option {o} is not translated yet"
            )));
        }
        let vars: Vec<&str> = c.parts.main.split_whitespace().collect();
        if vars.is_empty() {
            return Err(no("sktest needs a varlist"));
        }
        if let Some(v) = vars.iter().find(|v| !is_name(v)) {
            return Err(no(format!(
                "sktest {v}: patterns and abbreviations are taken once the data's variables are known"
            )));
        }
        let w = self.weight_of(c, &[W::F, W::A])?;
        let mut rows = self.rows_for(c, out)?;
        if w.is_some() {
            rows = self.weighted_rows(rows, &w);
            out.line(format!("stata_rows <- {rows}"));
            rows = "stata_rows".into();
            out.caveat(super::weights::weight_caveat(&w));
        }
        let wargs = w_args(&rows, &w);
        for h in ["stata_sktest", "stata_fmt", "stata_returned"] {
            self.helpers.insert(h);
        }
        let list: Vec<String> = vars.iter().map(|v| r_text(v)).collect();
        out.line(format!(
            "stata_r <- stata_sktest({rows}, c({}){}{wargs}{})",
            list.join(", "),
            if opts.has("noadjust", 8) {
                ", adjust = FALSE"
            } else {
                ""
            },
            if self.quiet(c) { ", show = FALSE" } else { "" }
        ));
        self.r_new = Some((R_SKTEST, "sktest".into()));
        Ok(())
    }
}
