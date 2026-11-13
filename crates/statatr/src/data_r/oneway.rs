//! `oneway response factor [if] [in] [, tabulate noanova nolabel]`: the
//! one-way analysis of variance as Stata 19 prints it, with Bartlett's test
//! of equal variances and the r() it leaves (`plan/stata/oneway.md`).

use super::*;

pub(super) const ONEWAY_HELPER: &str = r##"stata_oneway <- function(d, y, g, tabulate = FALSE, anova = TRUE, nolabel = FALSE, show = TRUE,
                         w = NULL, wtype = "f", compare = character(0), cols = c(TRUE, TRUE, TRUE),
                         missing = FALSE, obs = TRUE, obs_always = FALSE) {
  for (v in c(y, g)) if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
  if (is.character(d[[y]])) stop(y, " is a string variable; oneway needs a number (Stata: type mismatch, r(109))", call. = FALSE)
  num <- function(v) { u <- unclass(v); attributes(u) <- NULL; as.numeric(u) }
  x <- num(d[[y]])
  gv <- d[[g]]
  gk <- if (is.character(gv)) ifelse(is.na(gv), "", gv) else num(gv)
  wv <- if (is.null(w)) rep(1, length(x)) else num(w)
  # with missing, the missing values of the group are a group of their own (Stata 19: ".")
  if (missing && !is.character(gv)) gk[is.na(gk)] <- Inf
  keep <- !is.na(x) & !is.na(wv) & (missing | (if (is.character(gv)) gk != "" else !is.na(gk)))
  x <- x[keep]
  gk <- gk[keep]
  wv <- wv[keep]
  if (!length(x)) stop("no observations (Stata: r(2000))", call. = FALSE)
  # analytic weights are scaled to sum to the rows, frequency weights count rows
  wraw <- wv
  if (!is.null(w) && wtype == "a") wv <- wv * length(x) / sum(wv)
  lv <- sort(unique(gk), method = "radix")
  vl <- if (nolabel || is.character(gv)) NULL else attr(gv, "labels", exact = TRUE)
  name <- function(v) {
    if (is.character(v)) return(v)
    if (is.infinite(v)) return(".")
    k <- if (!is.null(vl)) names(vl)[match(v, num(vl))] else NA
    if (length(k) && !is.na(k)) k else stata_fmt(v, "%9.0g", trim = TRUE)
  }
  n <- vapply(lv, function(v) sum(wv[gk == v]), 0)
  m <- vapply(lv, function(v) sum(wv[gk == v] * x[gk == v]) / sum(wv[gk == v]), 0)
  ssg <- vapply(seq_along(lv), function(i) sum(wv[gk == lv[i]] * (x[gk == lv[i]] - m[i])^2), 0)
  s2 <- ifelse(n > 1, ssg / (n - 1), 0)
  N <- sum(wv)
  k <- length(lv)
  M <- sum(wv * x) / N
  mss <- sum(n * (m - M)^2)
  rss <- sum(ssg)
  df_m <- k - 1
  df_r <- N - k
  ms <- function(ss, df) if (df > 0) ss / df else NA_real_
  F <- if (df_m > 0 && df_r > 0) ms(mss, df_m) / ms(rss, df_r) else NA_real_
  p <- if (is.finite(F)) stats::pf(F, df_m, df_r, lower.tail = FALSE) else NA_real_
  # Bartlett's test on the cells with a positive variance
  use <- n > 1 & s2 > 0
  single <- sum(n == 1)
  # groups of several equal values (with frequency weights, one row counted several times)
  multi <- sum(n > 1 & s2 == 0)
  chi2 <- NA_real_
  dfb <- NA_real_
  if (sum(use) >= 2) {
    # with analytic weights Stata 19 counts each group's rows, the weighted sum of squares
    # over the rows less one; frequency weights count as rows
    if (!is.null(w) && wtype == "a") {
      rows_g <- vapply(lv, function(v) sum(gk == v), 0)
      nu <- rows_g[use] - 1
      vb <- ssg[use] / nu
    } else {
      nu <- n[use] - 1
      vb <- s2[use]
    }
    sp <- sum(nu * vb) / sum(nu)
    C <- 1 + (sum(1 / nu) - 1 / sum(nu)) / (3 * (sum(use) - 1))
    chi2 <- (sum(nu) * log(sp) - sum(nu * log(vb))) / C
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
  if (tabulate && any(cols)) {
    weighted <- !is.null(w)
    weighted <- (weighted && obs) || obs_always
    width <- 12 * (sum(cols) + weighted)
    rows_n <- vapply(lv, function(v) sum(gk == v), 0)
    # with weights: the weight sums as Freq., the rows (frequency weights: their sums) as Obs.
    freq <- vapply(lv, function(v) sum(wraw[gk == v]), 0)
    obs <- if (weighted && wtype == "a") rows_n else n
    # the sd: with analytic weights summarize's (the weighted variance times rows/(rows - 1)), as Stata 19 shows it
    sd_g <- if (weighted && wtype == "a") sqrt(ifelse(rows_n > 1, ssg / n * rows_n / (rows_n - 1), 0)) else sqrt(s2)
    tot_ss <- sum(wv * (x - M)^2)
    sd_t <- if (weighted && wtype == "a") sqrt(tot_ss / N * length(x) / (length(x) - 1)) else if (N > 1) sqrt(tot_ss / (N - 1)) else 0
    title <- paste("Summary of", y)
    head1 <- if (nchar(g) > 11) substr(g, 1, 11) else ""
    head2 <- if (nchar(g) > 11) substring(g, 12) else g
    centre <- function(s) paste0(strrep(" ", max(0, ceiling((width - nchar(s)) / 2))), s)
    # a title as wide as its columns goes on two lines (Stata 19: "Summary of" over "y")
    if (nchar(title) >= width) {
      cat("\n", sprintf("%11s", head1), " |", centre("Summary of"), "\n", sep = "")
      cat(sprintf("%11s", ""), " |", centre(y), "\n", sep = "")
    } else {
      cat("\n", sprintf("%11s", head1), " |", centre(title), "\n", sep = "")
    }
    line <- function(lab, a, b, f, o = NULL) {
      v <- c(c(a, b, f)[cols], if (weighted) o)
      paste0(sprintf("%11s |", lab), paste0(sprintf("%12s", v), collapse = ""), "\n")
    }
    cat(line(head2, "Mean", "Std. dev.", "Freq.", "Obs"))
    rule <- paste0(strrep("-", 12), "+", strrep("-", width), "\n")
    cat(rule)
    for (i in seq_along(lv)) {
      cat(line(substr(name(lv[i]), 1, 9), g9(m[i]), g9(sd_g[i]), format(if (weighted) freq[i] else n[i], scientific = FALSE),
               format(obs[i], scientific = FALSE)))
    }
    cat(rule)
    cat(line("Total", g9(M), g9(sd_t), format(if (weighted) sum(wraw) else N, scientific = FALSE),
             format(if (weighted && wtype == "a") length(x) else N, scientific = FALSE)))
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
    if ((single > 0 || multi > 0) && is.finite(chi2)) {
      cat("\nnote: Bartlett's test performed on cells with positive variance:\n")
      if (single > 0) cat(sprintf("      %d single-observation cells not used\n", single))
      if (multi > 0) cat(sprintf("      %d multiple-observation cells not used\n", multi))
    }
  }
  # the multiple-comparison tests: each later group's mean less each earlier one's, with the
  # pairwise t test's p-value adjusted (Bonferroni, Scheffe, Sidak)
  if (length(compare) && k >= 2) {
    ls <- if (exists("stata_linesize")) stata_linesize else 79
    if (9 + 11 * (k - 1) > ls) stop("oneway: comparisons of this many groups (a table wider than the line) are not translated yet", call. = FALSE)
    msw <- ms(rss, df_r)
    pairs <- k * (k - 1) / 2
    labs <- vapply(lv, function(v) substr(name(v), 1, 8), "")
    for (how in compare) {
      title <- paste("Comparison of", y, "by", g)
      sub <- paste0("(", c(bonferroni = "Bonferroni", scheffe = "Scheffe", sidak = "Sidak")[[how]], ")")
      cat("\n", strrep(" ", ceiling((77 - nchar(title)) / 2)), title, "\n", sep = "")
      cat(strrep(" ", floor((77 - nchar(sub)) / 2)), sub, "\n", sep = "")
      cat("Row Mean-|\n")
      cat("Col Mean |", paste0(sprintf("%11s", labs[-k]), collapse = ""), "\n", sep = "")
      cat(strrep("-", 9), "+", strrep("-", 11 * (k - 1)), "\n", sep = "")
      for (i in 2:k) {
        dif <- m[i] - m[seq_len(i - 1)]
        t <- dif / sqrt(msw * (1 / n[i] + 1 / n[seq_len(i - 1)]))
        pt2 <- 2 * stats::pt(-abs(t), df_r)
        p <- switch(how,
          bonferroni = pmin(1, pt2 * pairs),
          sidak = pmin(1, 1 - (1 - pt2)^pairs),
          scheffe = stats::pf(t^2 / (k - 1), k - 1, df_r, lower.tail = FALSE))
        cat(sprintf("%8s |", labs[i]), paste0(sprintf("%11s", vapply(dif, stata_fmt, "", fmt = "%8.0g", trim = TRUE)), collapse = ""), "\n", sep = "")
        cat("         |", paste0(sprintf("%11.3f", p), collapse = ""), "\n", sep = "")
        if (i < k) cat("         |\n")
      }
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
        if let Some(o) = opts.other(&[
            ("tabulate", 8),
            ("noanova", 7),
            ("nolabel", 7),
            ("bonferroni", 10),
            ("scheffe", 7),
            ("sidak", 5),
            ("nomeans", 7),
            ("nostandard", 10),
            ("nofreq", 6),
            ("missing", 1),
            ("noobs", 5),
            ("wrap", 4),
        ]) {
            return Err(no(format!(
                "oneway with the option {o} is not translated yet"
            )));
        }
        let w = self.weight_of(c, &[super::weights::W::F, super::weights::W::A])?;
        let compare: Vec<String> = [("bonferroni", 10), ("scheffe", 7), ("sidak", 5)]
            .iter()
            .filter(|(o, m)| opts.has(o, *m))
            .map(|(o, _)| r_text(o))
            .collect();
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
        let mut rows = self.rows_for(c, out)?;
        if w.is_some() {
            rows = self.weighted_rows(rows, &w);
            out.line(format!("stata_rows <- {rows}"));
            rows = "stata_rows".into();
            out.caveat(super::weights::weight_caveat(&w));
        }
        let wargs = super::weights::w_args(&rows, &w);
        for h in ["stata_oneway", "stata_fmt", "stata_returned"] {
            self.helpers.insert(h);
        }
        let mut args = format!("{rows}, {}, {}{wargs}", r_text(y), r_text(g));
        if !compare.is_empty() {
            args.push_str(&format!(", compare = c({})", compare.join(", ")));
        }
        if opts.has("tabulate", 8) {
            args.push_str(", tabulate = TRUE");
        }
        if opts.has("noanova", 7) {
            args.push_str(", anova = FALSE");
        }
        let cols =
            [("nomeans", 7), ("nostandard", 10), ("nofreq", 6)].map(|(o, m)| !opts.has(o, m));
        if cols.iter().any(|c| !c) {
            let v: Vec<&str> = cols.iter().map(|c| super::stats::r_logical(*c)).collect();
            args.push_str(&format!(", cols = c({})", v.join(", ")));
        }
        if opts.has("missing", 1) {
            args.push_str(", missing = TRUE");
        }
        if opts.has("noobs", 5) {
            args.push_str(", obs = FALSE");
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
