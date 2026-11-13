//! `oneway response factor [if] [in] [, tabulate noanova nolabel]`: the
//! one-way analysis of variance as Stata 19 prints it, with Bartlett's test
//! of equal variances and the r() it leaves (`plan/stata/oneway.md`).

use super::*;

pub(super) const ONEWAY_HELPER: &str = r##"stata_oneway <- function(d, y, g, tabulate = FALSE, anova = TRUE, nolabel = FALSE, show = TRUE) {
  for (v in c(y, g)) if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
  if (is.character(d[[y]])) stop(y, " is a string variable; oneway needs a number (Stata: type mismatch, r(109))", call. = FALSE)
  num <- function(v) { u <- unclass(v); attributes(u) <- NULL; as.numeric(u) }
  x <- num(d[[y]])
  gv <- d[[g]]
  gk <- if (is.character(gv)) ifelse(is.na(gv), "", gv) else num(gv)
  keep <- !is.na(x) & (if (is.character(gv)) gk != "" else !is.na(gk))
  x <- x[keep]
  gk <- gk[keep]
  if (!length(x)) stop("no observations (Stata: r(2000))", call. = FALSE)
  lv <- sort(unique(gk), method = "radix")
  vl <- if (nolabel || is.character(gv)) NULL else attr(gv, "labels", exact = TRUE)
  name <- function(v) {
    if (is.character(v)) return(v)
    k <- if (!is.null(vl)) names(vl)[match(v, num(vl))] else NA
    if (length(k) && !is.na(k)) k else stata_fmt(v, "%9.0g", trim = TRUE)
  }
  n <- vapply(lv, function(v) sum(gk == v), 0)
  m <- vapply(lv, function(v) mean(x[gk == v]), 0)
  s2 <- vapply(lv, function(v) if (sum(gk == v) > 1) stats::var(x[gk == v]) else 0, 0)
  N <- length(x)
  k <- length(lv)
  M <- mean(x)
  mss <- sum(n * (m - M)^2)
  rss <- sum(vapply(seq_along(lv), function(i) sum((x[gk == lv[i]] - m[i])^2), 0))
  df_m <- k - 1
  df_r <- N - k
  ms <- function(ss, df) if (df > 0) ss / df else NA_real_
  F <- if (df_m > 0 && df_r > 0) ms(mss, df_m) / ms(rss, df_r) else NA_real_
  p <- if (is.finite(F)) stats::pf(F, df_m, df_r, lower.tail = FALSE) else NA_real_
  # Bartlett's test on the cells with a positive variance
  use <- n > 1 & s2 > 0
  single <- sum(n == 1)
  if (any(n > 1 & s2 == 0)) stop("oneway: a group of several equal values (zero variance) in Bartlett's test is not translated yet", call. = FALSE)
  chi2 <- NA_real_
  dfb <- NA_real_
  if (sum(use) >= 2) {
    nu <- n[use] - 1
    sp <- sum(nu * s2[use]) / sum(nu)
    C <- 1 + (sum(1 / nu) - 1 / sum(nu)) / (3 * (sum(use) - 1))
    chi2 <- (sum(nu) * log(sp) - sum(nu * log(s2[use]))) / C
    dfb <- sum(use) - 1
  }
  A <- matrix(c(mss, rss, mss + rss, df_m, df_r, N - 1, ms(mss, df_m), ms(rss, df_r), ms(mss + rss, N - 1),
                F, NA, NA, p, NA, NA), 3, 5,
              dimnames = list(c("Between groups", "Within groups", "Total"), c("SS", "df", "MS", "F", "p")))
  r <- list(N = N, mss = mss, df_m = df_m, rss = rss, df_r = df_r, F = F, chi2bart = chi2, df_bart = dfb, ANOVA = A)
  # Stata 19: with noanova, r() is left empty
  if (!anova) r <- list()
  if (!show) return(r)
  g9 <- function(v) stata_fmt(v, "%10.0g", trim = TRUE)
  if (tabulate) {
    title <- paste("Summary of", y)
    off <- max(0, ceiling((36 - nchar(title)) / 2))
    head1 <- if (nchar(g) > 11) substr(g, 1, 11) else ""
    head2 <- if (nchar(g) > 11) substring(g, 12) else g
    cat("\n", sprintf("%11s", head1), " |", strrep(" ", off), title, "\n", sep = "")
    cat(sprintf("%11s |%12s%12s%12s", head2, "Mean", "Std. dev.", "Freq."), "\n", sep = "")
    rule <- paste0(strrep("-", 12), "+", strrep("-", 36), "\n")
    cat(rule)
    for (i in seq_along(lv)) {
      cat(sprintf("%11s |%12s%12s%12s", substr(name(lv[i]), 1, 9), g9(m[i]), g9(sqrt(s2[i])), format(n[i], scientific = FALSE)), "\n", sep = "")
    }
    cat(rule)
    cat(sprintf("%11s |%12s%12s%12s", "Total", g9(M), g9(if (N > 1) stats::sd(x) else 0), format(N, scientific = FALSE)), "\n", sep = "")
  }
  if (anova) {
    g10 <- function(v) stata_fmt(v, "%11.0g", trim = TRUE)
    rule <- paste0(strrep("-", 72), "\n")
    cat("\n                        Analysis of variance\n")
    cat("    Source              SS         df      MS            F     Prob > F\n")
    cat(rule)
    tail <- if (is.finite(F)) sprintf("%10.2f%11.4f", F, p) else ""
    cat(sprintf("%-14s%16s%7d%13s", "Between groups", g10(mss), as.integer(df_m), g10(ms(mss, df_m))), tail, "\n", sep = "")
    cat(sprintf("%-14s%16s%7d%13s", " Within groups", g10(rss), as.integer(df_r), g10(ms(rss, df_r))), "\n", sep = "")
    cat(rule)
    cat(sprintf("%-14s%16s%7d%13s", "    Total", g10(mss + rss), as.integer(N - 1), g10(ms(mss + rss, N - 1))), "\n", sep = "")
    if (is.finite(chi2)) {
      cat(sprintf("\nBartlett's equal-variances test: chi2(%d) = %8.4f    Prob>chi2 = %5.3f\n", as.integer(dfb), chi2,
                  stats::pchisq(chi2, dfb, lower.tail = FALSE)))
    }
    if (single > 0 && is.finite(chi2)) {
      cat("\nnote: Bartlett's test performed on cells with positive variance:\n")
      cat(sprintf("      %d single-observation cells not used\n", single))
    }
  }
  invisible(r)
}"##;

/// The r() oneway leaves (Stata 19), and its matrix r(ANOVA).
pub(super) const R_ONEWAY: &[&str] = &[
    "N", "mss", "df_m", "rss", "df_r", "F", "chi2bart", "df_bart",
];

impl Session {
    pub(super) fn oneway(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "oneway")?;
        if let Some(o) = opts.other(&[("tabulate", 8), ("noanova", 7), ("nolabel", 7)]) {
            return Err(no(format!(
                "oneway with the option {o} is not translated yet"
            )));
        }
        if c.parts.weight.is_some() {
            return Err(no("oneway with weights is not translated yet"));
        }
        let vars: Vec<&str> = c.parts.main.split_whitespace().collect();
        let [y, g] = vars.as_slice() else {
            return Err(no(
                "oneway takes a response and a factor variable (oneway y g)",
            ));
        };
        for v in [y, g] {
            if !is_name(v) {
                return Err(no(format!("oneway {v}: one variable name is translated")));
            }
        }
        let rows = self.rows_for(c, out)?;
        for h in ["stata_oneway", "stata_fmt", "stata_returned"] {
            self.helpers.insert(h);
        }
        let mut args = format!("{rows}, {}, {}", r_text(y), r_text(g));
        if opts.has("tabulate", 8) {
            args.push_str(", tabulate = TRUE");
        }
        if opts.has("noanova", 7) {
            args.push_str(", anova = FALSE");
        }
        if opts.has("nolabel", 7) {
            args.push_str(", nolabel = TRUE");
        }
        if self.quiet(c) {
            args.push_str(", show = FALSE");
        }
        out.line(format!("stata_r <- stata_oneway({args})"));
        self.r_new = Some((R_ONEWAY, "oneway".into()));
        Ok(())
    }
}
