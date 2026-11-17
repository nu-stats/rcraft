//! `ttest` and `ttesti` as Stata 19 prints them (`plan/stata/ttest.md`): the
//! one-sample, paired and two-sample tables (by(), unpaired, unequal with
//! Satterthwaite's degrees of freedom, welch with Welch's), the three
//! alternatives below, and the r() `stata_ttest_r` leaves. `ttesti` takes the
//! counts, means and standard deviations typed in. Checked against Stata 19
//! on made-up data.

pub(super) const TTEST_SHOW_HELPER: &str = r##"# ttesti's r(): the same as ttest's, from the counts, means and standard deviations typed in
stata_ttesti_r <- function(n1, m1, s1, n2 = NULL, m2 = NULL, s2 = NULL, mu = NULL, level = 95, equal = TRUE, welch = FALSE) {
  q <- (1 + level / 100) / 2
  one <- function(n, m, s) { se <- s / sqrt(n); h <- stats::qt(q, n - 1) * se; list(N = n, mu = m, sd = s, se = se, lb = m - h, ub = m + h) }
  tests <- function(t, df) list(p_u = stats::pt(t, df, lower.tail = FALSE), p_l = stats::pt(t, df), p = 2 * stats::pt(-abs(t), df), t = t, df_t = df)
  a <- one(n1, m1, s1)
  if (!is.null(mu)) {
    return(c(list(ub_1 = a$ub, lb_1 = a$lb, level = level, sd_1 = a$sd, se = a$se), tests((a$mu - mu) / a$se, a$N - 1),
             list(mu_1 = a$mu, N_1 = a$N)))
  }
  b <- one(n2, m2, s2)
  N <- n1 + n2
  # the two groups together: the pooled mean, and the spread within and between them
  sdc <- sqrt(((n1 - 1) * s1^2 + (n2 - 1) * s2^2 + n1 * n2 / N * (m1 - m2)^2) / (N - 1))
  all <- one(N, (n1 * m1 + n2 * m2) / N, sdc)
  diff <- m1 - m2
  v1 <- s1^2 / n1; v2 <- s2^2 / n2
  if (equal && !welch) {
    df <- N - 2
    se <- sqrt(((n1 - 1) * s1^2 + (n2 - 1) * s2^2) / df * (1 / n1 + 1 / n2))
  } else {
    se <- sqrt(v1 + v2)
    df <- if (welch) (v1 + v2)^2 / (v1^2 / (n1 + 1) + v2^2 / (n2 + 1)) - 2 else (v1 + v2)^2 / (v1^2 / (n1 - 1) + v2^2 / (n2 - 1))
  }
  h <- stats::qt(q, df) * se
  c(list(ub_diff = diff + h, lb_diff = diff - h, mu_diff = diff, ub_combined = all$ub, lb_combined = all$lb,
         se_combined = all$se, mu_combined = all$mu, N_combined = all$N, ub_2 = b$ub, lb_2 = b$lb, se_2 = b$se,
         ub_1 = a$ub, lb_1 = a$lb, se_1 = a$se, level = level, sd = all$sd, sd_2 = b$sd, sd_1 = a$sd, se = se),
    tests(diff / se, df), list(mu_2 = b$mu, N_2 = b$N, mu_1 = a$mu, N_1 = a$N))
}
# ttest's table as Stata 19 prints it, from its r(): kind "one", "paired" or "two"; rows the names of the
# rows (two of them for paired and two), head the first column's title, nm the names in the lines below
stata_ttest_show <- function(R, kind, rows, head = "Variable", nm = rows, mu = 0, equal = TRUE, welch = FALSE) {
  g <- function(v) stata_fmt(v, "%9.0g", trim = TRUE)
  row <- function(name, n, m, se, sd, lb, ub)
    cat(sprintf("%8s |%8s%12s%12s%12s%12s%12s", name, n, m, se, sd, lb, ub), "\n", sep = "")
  bar <- function() cat(strrep("-", 9), "+", strrep("-", 68), "\n", sep = "")
  title <- switch(kind, one = "One-sample t test", paired = "Paired t test",
                  two = if (equal && !welch) "Two-sample t test with equal variances" else "Two-sample t test with unequal variances")
  cat("\n", title, "\n", strrep("-", 78), "\n", sep = "")
  cat(sprintf("%8s |%8s%12s%13s%12s%23s", head, "Obs", "Mean", "Std. err.", "Std. dev.", paste0("[", R$level, "% conf. interval]")), "\n", sep = "")
  bar()
  if (kind == "one") {
    row(rows[1], R$N_1, g(R$mu_1), g(R$se), g(R$sd_1), g(R$lb_1), g(R$ub_1))
  } else {
    row(rows[1], R$N_1, g(R$mu_1), g(R$se_1), g(R$sd_1), g(R$lb_1), g(R$ub_1))
    row(rows[2], R$N_2, g(R$mu_2), g(R$se_2), g(R$sd_2), g(R$lb_2), g(R$ub_2))
    bar()
    if (kind == "paired") {
      row("diff", R$N_1, g(R$mu_diff), g(R$se), g(R$sd_diff), g(R$lb_diff), g(R$ub_diff))
    } else {
      row("Combined", R$N_combined, g(R$mu_combined), g(R$se_combined), g(R$sd), g(R$lb_combined), g(R$ub_combined))
      bar()
      row("diff", "", g(R$mu_diff), g(R$se), "", g(R$lb_diff), g(R$ub_diff))
    }
  }
  cat(strrep("-", 78), "\n", sep = "")
  # a line's two parts, the right one ending at column 78
  two <- function(l, r) cat(l, strrep(" ", max(1, 78 - nchar(l) - nchar(r))), r, "\n", sep = "")
  m0 <- g(mu)
  lhs <- switch(kind, one = "mean", paired = "mean(diff)", two = "diff")
  rhs <- switch(kind, one = paste0("mean(", nm[1], ")"), paired = paste0("mean(", nm[1], " - ", nm[2], ")"),
                two = paste0("mean(", nm[1], ") - mean(", nm[2], ")"))
  ind <- if (kind == "paired") 5 else 4
  two(paste0(strrep(" ", ind), lhs, " = ", rhs), sprintf("t = %8s", stata_fmt(R$t, "%8.4f")))
  dfl <- if (kind == "two" && welch) "Welch's degrees of freedom" else if (kind == "two" && !equal) "Satterthwaite's degrees of freedom" else "Degrees of freedom"
  two(paste0(strrep(" ", ind - 4), "H0: ", lhs, " = ", m0), sprintf("%s = %8s", dfl, stata_fmt(R$df_t, "%8.0g", trim = TRUE)))
  # each alternative centred over its p-value
  ha <- paste0("Ha: ", lhs, c(" < ", " != ", " > "), m0)
  line <- rep(" ", 78)
  for (k in 1:3) {
    s <- ceiling(c(10.5, 39.5, 69.5)[k] + 0.5 - nchar(ha[k]) / 2)
    line[s:(s + nchar(ha[k]) - 1)] <- strsplit(ha[k], "")[[1]]
  }
  cat(sub(" +$", "", paste(line, collapse = "")), "\n", sep = "")
  p4 <- function(v) stata_fmt(v, "%6.4f")
  cat(" Pr(T < t) = ", p4(R$p_l), "         Pr(|T| > |t|) = ", p4(R$p), "          Pr(T > t) = ", p4(R$p_u), "\n", sep = "")
  invisible(R)
}
# by()'s groups as ttest names them: a value's label (or the value), cut to 8
stata_ttest_groups <- function(g) {
  lv <- sort(unique(g[!is.na(g)]))
  vl <- attr(g, "labels", exact = TRUE)
  vapply(lv, function(v) {
    s <- if (is.character(v)) v else {
      k <- if (length(vl)) names(vl)[match(as.numeric(v), as.numeric(unclass(vl)))] else NA
      if (length(k) && !is.na(k)) k else stata_fmt(as.numeric(v), "%9.0g", trim = TRUE)
    }
    trimws(substr(s, 1, 8), "right")
  }, "")
}"##;
