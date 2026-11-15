//! The epidemiological tables `cs`, `csi`, `cc`, `cci`, `ir` and `iri` as
//! Stata 19 prints them, and the r() they leave (`plan/stata/epitab.md`):
//! risks, odds or rates of the exposed and the unexposed, their difference
//! and ratio with Wald (log) intervals, Woolf's or the test-based interval
//! of the odds ratio, the exact Poisson interval of the rate ratio, the
//! attributable (or, below 1, prevented) fractions, Pearson's chi2 and
//! Fisher's exact p; by() the strata with the Mantel-Haenszel (or internally
//! standardized) combined estimate and the test of homogeneity; tabodds the
//! odds by level with the tests of equal odds and of trend. A nonzero
//! exposure or case value counts as one, as in
//! Stata.

use super::*;

pub(super) const EPITAB_HELPER: &str = r##"stata_epi_counts <- function(d, case, expo, time = NULL) {
  y <- stata_epi_num(d[[case]])
  x <- stata_epi_num(d[[expo]])
  t <- if (is.null(time)) NULL else stata_epi_num(d[[time]])
  ok <- !is.na(y) & !is.na(x) & (if (is.null(t)) TRUE else !is.na(t))
  y <- y[ok] != 0
  x <- x[ok] != 0
  lab <- function(v, name) {
    l <- attr(d[[v]], "label", exact = TRUE)
    if (is.character(l) && length(l) == 1 && nzchar(l)) l else name
  }
  r <- list(expo = lab(expo, expo), case = lab(case, case))
  if (is.null(t)) {
    c(r, list(a = sum(y & x), b = sum(y & !x), c = sum(!y & x), d = sum(!y & !x)))
  } else {
    t <- t[ok]
    c(r, list(a = sum(y & x), b = sum(y & !x), t1 = sum(t[x]), t0 = sum(t[!x]), time = lab(time, time)))
  }
}
stata_epi_num <- function(v) {
  if (inherits(v, "haven_labelled")) v <- unclass(v)
  as.numeric(v)
}
stata_epi <- function(kind, a, b, c = NA, d = NA, t1 = NA, t0 = NA, level = 95, ci = "woolf", exact = FALSE,
                      expo = NULL, case = NULL, time = NULL, show = TRUE) {
  z <- stats::qnorm(1 - (100 - level) / 200)
  g <- function(v) stata_fmt(v, "%9.0g", trim = TRUE)
  rowl <- function(lab, x, y, w) sprintf("%16s |%10s%12s  |%11s", lab, x, y, w)
  est <- function(lab, v, lo = NULL, hi = NULL, how = "") {
    if (is.null(lo)) return(sprintf("%16s |%17s       |", lab, g(v)))
    sprintf("%16s |%17s       |%12s%12s %s", lab, g(v), g(lo), g(hi), how)
  }
  center <- function(s, w) {
    left <- max(0, (w - nchar(s)) %/% 2)
    paste0(strrep(" ", left), s, strrep(" ", max(0, w - left - nchar(s))))
  }
  top <- sprintf("%16s |      Point estimate    |    [%s%% conf. interval]", "", g(level))
  bar <- paste0(strrep(" ", 17), "|", strrep("-", 24), "+", strrep("-", 24))
  foot <- paste0(strrep(" ", 17), "+", strrep("-", 49))
  gap <- paste0(strrep(" ", 17), "|", strrep(" ", 24), "|")
  frac <- function(ratio, lo, hi) {
    # attributable above 1, prevented below (Stata 19)
    if (ratio >= 1) list(f = (ratio - 1) / ratio, lo = (lo - 1) / lo, hi = (hi - 1) / hi, word = "Attr.")
    else list(f = 1 - ratio, lo = 1 - hi, hi = 1 - lo, word = "Prev.")
  }
  if (kind == "ir") {
    n <- a + b
    tt <- t1 + t0
    ird <- a / t1 - b / t0
    sd <- sqrt(a / t1^2 + b / t0^2)
    ci_r <- stats::poisson.test(c(a, b), c(t1, t0), conf.level = level / 100)$conf.int
    irr <- (a / t1) / (b / t0)
    fr <- frac(irr, ci_r[1], ci_r[2])
    r0 <- b / t0
    rt <- n / tt
    afp <- if (irr >= 1) (rt - r0) / rt else (r0 - rt) / r0
    pr <- t1 / tt
    lower_e <- stats::pbinom(a, n, pr)
    upper_e <- stats::pbinom(a - 1, n, pr, lower.tail = FALSE)
    at <- stats::dbinom(a, n, pr)
    lower_m <- lower_e - at / 2
    upper_m <- upper_e - at / 2
    r <- list(ird = ird, lb_ird = ird - z * sd, ub_ird = ird + z * sd, irr = irr, lb_irr = ci_r[1], ub_irr = ci_r[2],
              afe = fr$f, lb_afe = fr$lo, ub_afe = fr$hi, afp = afp, p_lower_midp = lower_m, p_upper_midp = upper_m,
              p_twosided_midp = min(1, 2 * min(lower_m, upper_m)), p_lower_exact = lower_e, p_upper_exact = upper_e,
              p_twosided_exact = min(1, 2 * min(lower_e, upper_e)))
    if (!show) return(r)
    cn <- if (is.null(case)) "Cases" else case
    tn <- if (is.null(time)) "Person-time" else time
    cat("\nIncidence-rate comparison\n\n")
    if (!is.null(expo)) cat(sprintf("%16s |%s|", "", center(expo, 24)), "\n", sep = "")
    cat(rowl("", "Exposed", "Unexposed", "Total"), "\n", sep = "")
    cat(strrep("-", 17), "+", strrep("-", 24), "+", strrep("-", 11), "\n", sep = "")
    cat(rowl(cn, g(a), g(b), g(n)), "\n", rowl(tn, g(t1), g(t0), g(tt)), "\n", sep = "")
    cat(strrep("-", 17), "+", strrep("-", 24), "+", strrep("-", 11), "\n", sep = "")
    cat(gap, "\n", rowl("Incidence rate", g(a / t1), g(r0), g(rt)), "\n", gap, "\n", top, "\n", bar, "\n", sep = "")
    cat(est("Inc. rate diff.", ird, r$lb_ird, r$ub_ird), "\n", sep = "")
    cat(est("Inc. rate ratio", irr, ci_r[1], ci_r[2], "(exact)"), "\n", sep = "")
    cat(est(paste(fr$word, "frac. ex."), fr$f, fr$lo, fr$hi, "(exact)"), "\n", sep = "")
    cat(est(paste(fr$word, "frac. pop"), afp), "\n", foot, "\n", sep = "")
    cat("Mid-p-values for tests of incidence-rate difference:\n")
    cw <- if (is.null(case)) "cases" else case
    l1 <- sprintf("  Adj Pr(Exposed %s <= %s)", cw, g(a))
    l2 <- sprintf("  Adj Pr(Exposed %s >= %s)", cw, g(a))
    cat(sprintf("%s = %6.4f (lower one-sided)", l1, lower_m), "\n", sep = "")
    cat(sprintf("%s = %6.4f (upper one-sided)", l2, upper_m), "\n", sep = "")
    cat(sprintf("%*s = %6.4f", nchar(l1), "Two-sided p-value", r$p_twosided_midp), "\n\n", sep = "")
    return(invisible(r))
  }
  n1 <- a + c
  n0 <- b + d
  m1 <- a + b
  m0 <- c + d
  N <- n1 + n0
  e <- (m1 * n1 / N)
  chi2 <- N * (a * d - b * c)^2 / (m1 * m0 * n1 * n0)
  p <- stats::pchisq(chi2, 1, lower.tail = FALSE)
  if (kind == "cs") {
    p1 <- a / n1
    p0 <- b / n0
    pt <- m1 / N
    rd <- p1 - p0
    sd <- sqrt(p1 * (1 - p1) / n1 + p0 * (1 - p0) / n0)
    rr <- p1 / p0
    sl <- sqrt(1 / a - 1 / n1 + 1 / b - 1 / n0)
    lr <- exp(log(rr) - z * sl)
    ur <- exp(log(rr) + z * sl)
    fr <- frac(rr, lr, ur)
    afp <- if (rr >= 1) (pt - p0) / pt else (p0 - pt) / p0
    r <- list(afp = afp, ub_afe = fr$hi, lb_afe = fr$lo, afe = fr$f, ub_rr = ur, lb_rr = lr, rr = rr,
              ub_rd = rd + z * sd, lb_rd = rd - z * sd, rd = rd)
    if (exact) {
      m <- matrix(c(a, c, b, d), 2)
      ft <- stats::fisher.test(m)$p.value
      one <- min(stats::fisher.test(m, alternative = "greater")$p.value, stats::fisher.test(m, alternative = "less")$p.value)
      r <- c(r, list(p_exact = ft, p1_exact = one))
    } else {
      r <- c(r, list(p = p, chi2 = chi2))
    }
    if (!show) return(r)
    cat("\n")
    if (!is.null(expo)) cat(sprintf("%16s |%s|", "", center(expo, 24)), "\n", sep = "")
    cat(rowl("", "Exposed", "Unexposed", "Total"), "\n", sep = "")
    cat(strrep("-", 17), "+", strrep("-", 24), "+", strrep("-", 11), "\n", sep = "")
    cat(rowl("Cases", g(a), g(b), g(m1)), "\n", rowl("Noncases", g(c), g(d), g(m0)), "\n", sep = "")
    cat(strrep("-", 17), "+", strrep("-", 24), "+", strrep("-", 11), "\n", sep = "")
    cat(rowl("Total", g(n1), g(n0), g(N)), "\n", gap, "\n", rowl("Risk", g(p1), g(p0), g(pt)), "\n", gap, "\n", sep = "")
    cat(top, "\n", bar, "\n", sep = "")
    cat(est("Risk difference", rd, r$lb_rd, r$ub_rd), "\n", est("Risk ratio", rr, lr, ur), "\n", sep = "")
    cat(est(paste(fr$word, "frac. ex."), fr$f, fr$lo, fr$hi), "\n", est(paste(fr$word, "frac. pop"), afp), "\n", sep = "")
    cat(foot, "\n", sep = "")
  } else {
    or <- (a * d) / (b * c)
    if (ci == "tb") {
      # the test-based interval on the Mantel-Haenszel chi2, (N - 1)/N Pearson's (Stata 19)
      ends <- or^(1 + c(-1, 1) * z / sqrt(chi2 * (N - 1) / N))
      lo <- min(ends)
      hi <- max(ends)
      how <- "(tb)"
    } else {
      sw <- sqrt(1 / a + 1 / b + 1 / c + 1 / d)
      lo <- exp(log(or) - z * sw)
      hi <- exp(log(or) + z * sw)
      how <- "(Woolf)"
    }
    fr <- frac(or, lo, hi)
    afp <- if (or >= 1) fr$f * a / m1 else fr$f * c / m0
    r <- list(afp = afp, ub_afe = fr$hi, lb_afe = fr$lo, chi2 = chi2, p = p, afe = fr$f, ub_or = hi, lb_or = lo, or = or)
    if (!show) return(r)
    cat("\n")
    if (!is.null(expo)) {
      cat(sprintf("%16s |%s|%24s", "", center(expo, 24), "Proportion"), "\n", sep = "")
    } else {
      cat(strrep(" ", 57), "Proportion\n", sep = "")
    }
    pr <- function(lab, x, y, w, q) paste0(rowl(lab, x, y, w), sprintf("%13.4f", q))
    cat(rowl("", "Exposed", "Unexposed", "Total"), "      exposed\n", sep = "")
    cat(strrep("-", 17), "+", strrep("-", 24), "+", strrep("-", 24), "\n", sep = "")
    cat(pr("Cases", g(a), g(b), g(m1), a / m1), "\n", pr("Controls", g(c), g(d), g(m0), c / m0), "\n", sep = "")
    cat(strrep("-", 17), "+", strrep("-", 24), "+", strrep("-", 24), "\n", sep = "")
    cat(pr("Total", g(n1), g(n0), g(N), n1 / N), "\n", gap, "\n", top, "\n", bar, "\n", sep = "")
    cat(est("Odds ratio", or, lo, hi, how), "\n", est(paste(fr$word, "frac. ex."), fr$f, fr$lo, fr$hi, how), "\n", sep = "")
    cat(est(paste(fr$word, "frac. pop"), afp), "\n", foot, "\n", sep = "")
  }
  if (kind == "cs" && exact) {
    cat(sprintf("%34s1-sided Fisher's exact P = %6.4f", "", r$p1_exact), "\n", sep = "")
    cat(sprintf("%34s2-sided Fisher's exact P = %6.4f", "", r$p_exact), "\n\n", sep = "")
  } else {
    cat(sprintf("%31schi2(1) = %8.2f  Pr>chi2 = %6.4f", "", chi2, p), "\n\n", sep = "")
  }
  invisible(r)
}
stata_epi_vlab <- function(s, v, cut = 16) {
  vl <- attr(s, "labels", exact = TRUE)
  vapply(v, function(x) {
    if (is.character(x)) return(substr(x, 1, cut))
    k <- if (!is.null(vl)) names(vl)[match(x, stata_epi_num(vl))] else NA
    if (length(k) && !is.na(k)) substr(k, 1, cut) else stata_fmt(x, "%9.0g", trim = TRUE)
  }, "")
}
# a double rounded to a float, as Stata keeps the homogeneity chi2
stata_epi_float <- function(x) readBin(writeBin(as.numeric(x), raw(), size = 4), "numeric", size = 4, n = length(x))
# by(): the strata's estimates, the crude one, the Mantel-Haenszel (or internally standardized) combined
# one, the test of homogeneity and, for odds ratios, the Mantel-Haenszel test (Stata 19)
stata_epi_by <- function(kind, d, case, expo, by, time = NULL, level = 95, ci = "woolf", or = FALSE, istd = FALSE,
                         show = TRUE) {
  y <- stata_epi_num(d[[case]])
  x <- stata_epi_num(d[[expo]])
  s <- d[[by]]
  key <- if (is.character(s)) s else stata_epi_num(s)
  tm <- if (is.null(time)) NULL else stata_epi_num(d[[time]])
  ok <- !is.na(y) & !is.na(x) & !is.na(key) & (if (is.character(key)) key != "" else TRUE) &
    (if (is.null(tm)) TRUE else !is.na(tm))
  y <- y[ok] != 0
  x <- x[ok] != 0
  key <- key[ok]
  lv <- sort(unique(key))
  lab <- stata_epi_vlab(s, lv)
  z <- stats::qnorm(1 - (100 - level) / 200)
  g <- function(v) if (is.na(v) || !is.finite(v)) "." else stata_fmt(v, "%9.0g", trim = TRUE)
  fin <- function(v) ifelse(is.finite(v), v, NA_real_)
  cnt <- function(f) vapply(lv, function(l) sum(f & key == l), 1)
  a <- cnt(y & x)
  b <- cnt(y & !x)
  k <- length(lv)
  hom <- function(th, mh, v) stata_epi_float(sum((log(stata_epi_float(th)) - log(mh))^2 / stata_epi_float(v)))
  dash <- strrep("-", 67)
  sep <- paste0(strrep("-", 17), "+", strrep("-", 49))
  lvl <- g(level)
  bylab <- stata_tab_abbrev(by, 16)
  mhrow <- function(...) paste0("    M–H combined |", sprintf(...))
  if (kind == "ir") {
    t1 <- vapply(lv, function(l) sum(tm[ok][x & key == l]), 1)
    t0 <- vapply(lv, function(l) sum(tm[ok][!x & key == l]), 1)
    tt <- t1 + t0
    pci <- function(a, b, t1, t0) {
      r <- try(stats::poisson.test(c(a, b), c(t1, t0), conf.level = level / 100)$conf.int, silent = TRUE)
      if (inherits(r, "try-error")) c(NA_real_, NA_real_) else as.numeric(r)
    }
    irr <- (a / t1) / (b / t0)
    cis <- t(mapply(pci, a, b, t1, t0))
    crude <- (sum(a) / sum(t1)) / (sum(b) / sum(t0))
    cci <- pci(sum(a), sum(b), sum(t1), sum(t0))
    w <- b * t1 / tt
    mh <- sum(a * t0 / tt) / sum(w)
    v <- sum((a + b) * t1 * t0 / tt^2) / (sum(a * t0 / tt) * sum(w))
    vi <- 1 / a + 1 / b
    lp <- sum(log(irr) / vi) / sum(1 / vi)
    sp <- 1 / sqrt(sum(1 / vi))
    chi2 <- hom(irr, mh, vi)
    r <- list(irr = mh, lb_irr = exp(log(mh) - z * sqrt(v)), ub_irr = exp(log(mh) + z * sqrt(v)), crude = crude,
              lb_crude = cci[1], ub_crude = cci[2], pooled = exp(lp), lb_pooled = exp(lp - z * sp),
              ub_pooled = exp(lp + z * sp), df = k - 1, chi2_mh = chi2)
    r <- lapply(r, fin)
    if (!show) return(r)
    cat("\nStratified incidence-rate analysis\n\n", dash, "\n", sep = "")
    cat(sprintf("%16s |        IRR     [%s%% conf. interval]   M–H weight", bylab, lvl), "\n", sep, "\n", sep = "")
    for (i in seq_len(k)) cat(sprintf("%16s |%11s%14s%11s%13s (exact)", lab[i], g(irr[i]), g(cis[i, 1]), g(cis[i, 2]), g(w[i])), "\n", sep = "")
    cat(sep, "\n", sprintf("%16s |%11s%14s%11s%13s (exact)", "Crude", g(crude), g(cci[1]), g(cci[2]), ""), "\n", sep = "")
    cat(mhrow("%11s%14s%11s", g(mh), g(r$lb_irr), g(r$ub_irr)), "\n", dash, "\n", sep = "")
    cat("Test of homogeneity (M–H): chi2(", k - 1, ") = ", sprintf("%-12.2f  Pr>chi2 = %6.4f", chi2,
        stats::pchisq(chi2, k - 1, lower.tail = FALSE)), "\n\n", sep = "")
    return(invisible(r))
  }
  c3 <- cnt(!y & x)
  d4 <- cnt(!y & !x)
  n1 <- a + c3
  n0 <- b + d4
  N <- n1 + n0
  A <- sum(a); B <- sum(b); C <- sum(c3); D <- sum(d4)
  # the Mantel-Haenszel test that the combined odds ratio is 1
  E <- n1 * (a + b) / N
  V <- n1 * n0 * (a + b) * (c3 + d4) / (N^2 * (N - 1))
  mhchi <- sum(a - E)^2 / sum(V)
  mhtest <- function() {
    cat("                 Test that combined odds ratio = 1:\n")
    cat("                                Mantel–Haenszel chi2(1) =", sprintf("%10.2f", mhchi), "\n", sep = "")
    cat("                                                Pr>chi2 =", sprintf("%10.4f", stats::pchisq(mhchi, 1, lower.tail = FALSE)), "\n\n", sep = "")
  }
  # the odds ratios: Woolf's or the test-based interval (on each table's (N - 1)/N Pearson chi2)
  oddsci <- function(o, a, b, c3, d4, chi) {
    if (ci == "tb") sort(o^(1 + c(-1, 1) * z / sqrt(chi)))
    else exp(log(o) + c(-1, 1) * z * sqrt(1 / a + 1 / b + 1 / c3 + 1 / d4))
  }
  pear <- function(a, b, c3, d4) { n <- a + b + c3 + d4; (n - 1) * (a * d4 - b * c3)^2 / ((a + b) * (c3 + d4) * (a + c3) * (b + d4)) }
  if (kind == "cs" && !or) {
    p1 <- A / (A + C); p0 <- B / (B + D); pt <- (A + B) / sum(N)
    rr <- (a / n1) / (b / n0)
    cis <- t(mapply(function(a, b, n1, n0) exp(log((a / n1) / (b / n0)) + c(-1, 1) * z * sqrt(1 / a - 1 / n1 + 1 / b - 1 / n0)), a, b, n1, n0))
    crude <- p1 / p0
    cci <- exp(log(crude) + c(-1, 1) * z * sqrt(1 / A - 1 / (A + C) + 1 / B - 1 / (B + D)))
    afp <- if (crude >= 1) (pt - p0) / pt else (p0 - pt) / p0
    if (istd) {
      # internally standardized: the exposed as the standard
      O <- A; Ex <- sum(n1 * b / n0)
      est <- O / Ex
      q1 <- a / n1; q0 <- b / n0
      se <- sqrt(sum(n1 * q1 * (1 - q1)) / O^2 + sum(n1^2 * q0 * (1 - q0) / n0) / Ex^2)
      w <- n1
      r <- list(ub_rr = exp(log(est) + z * se), lb_rr = exp(log(est) - z * se), rr = est, afp = afp,
                ub_crude = cci[2], lb_crude = cci[1], crude = crude)
    } else {
      w <- b * n1 / N
      est <- sum(a * n0 / N) / sum(w)
      v <- sum((n1 * n0 * (a + b) - a * b * N) / N^2) / (sum(a * n0 / N) * sum(w))
      chi2 <- hom(rr, est, 1 / a - 1 / n1 + 1 / b - 1 / n0)
      r <- list(chi2_mh = chi2, df = k - 1, ub_rr = exp(log(est) + z * sqrt(v)), lb_rr = exp(log(est) - z * sqrt(v)),
                rr = est, afp = afp, ub_crude = cci[2], lb_crude = cci[1], crude = crude)
    }
    r <- lapply(r, fin)
    if (!show) return(r)
    cat("\n", sprintf("%16s |  Risk ratio    [%s%% conf. interval]   %s", bylab, lvl, if (istd) "    Weight" else "M–H weight"), "\n", sep, "\n", sep = "")
    for (i in seq_len(k)) cat(sprintf("%16s |%12s%13s%11s%13s ", lab[i], g(rr[i]), g(cis[i, 1]), g(cis[i, 2]), g(w[i])), "\n", sep = "")
    cat(sep, "\n", sprintf("%16s |%12s%13s%11s%13s ", "Crude", g(crude), g(cci[1]), g(cci[2]), ""), "\n", sep = "")
    if (istd) {
      cat(sprintf("%16s |%12s%13s%11s", "I. Standardized", g(est), g(r$lb_rr), g(r$ub_rr)), "\n\n", sep = "")
      return(invisible(r))
    }
    cat(mhrow("%12s%13s%11s", g(est), g(r$lb_rr), g(r$ub_rr)), "\n", dash, "\n", sep = "")
    cat("Test of homogeneity (M–H)      chi2(", k - 1, ") =", sprintf("%9.3f  Pr>chi2 = %6.4f", chi2,
        stats::pchisq(chi2, k - 1, lower.tail = FALSE)), "\n\n", sep = "")
    return(invisible(r))
  }
  # odds ratios: cs, or woolf and cc, woolf or tb
  o <- a * d4 / (b * c3)
  cis <- t(mapply(function(o, a, b, c3, d4) oddsci(o, a, b, c3, d4, pear(a, b, c3, d4)), o, a, b, c3, d4))
  crude <- A * D / (B * C)
  cci <- oddsci(crude, A, B, C, D, pear(A, B, C, D))
  w <- b * c3 / N
  est <- sum(a * d4 / N) / sum(w)
  P <- (a + d4) / N; Q <- (b + c3) / N; R <- a * d4 / N; S <- b * c3 / N
  v <- sum(P * R) / (2 * sum(R)^2) + sum(P * S + Q * R) / (2 * sum(R) * sum(S)) + sum(Q * S) / (2 * sum(S)^2)
  eci <- if (ci == "tb") sort(est^(1 + c(-1, 1) * z / sqrt(mhchi))) else exp(log(est) + c(-1, 1) * z * sqrt(v))
  chi2 <- hom(o, est, 1 / a + 1 / b + 1 / c3 + 1 / d4)
  tag <- if (ci == "tb") "(tb)" else "(Woolf)"
  if (kind == "cs") {
    p0 <- B / (B + D); pt <- (A + B) / sum(N)
    crr <- (A / (A + C)) / p0
    afp <- if (crr >= 1) (pt - p0) / pt else (p0 - pt) / p0
    r <- list(chi2_mh = chi2, df = k - 1, ub_or = eci[2], lb_or = eci[1], or = est, chi2 = mhchi, afp = afp,
              ub_crude = cci[2], lb_crude = cci[1], crude = crude)
  } else {
    fr <- if (crude >= 1) (crude - 1) / crude * A / (A + B) else (1 - crude) * C / (C + D)
    r <- list(ub_or = eci[2], lb_or = eci[1], or = est, chi2_p = chi2, df = k - 1, chi2 = mhchi, afp = fr,
              ub_crude = cci[2], lb_crude = cci[1], crude = crude)
  }
  r <- lapply(r, fin)
  if (!show) return(r)
  if (kind == "cs") {
    cat("\n", sprintf("%16s |  Odds ratio    [%s%% conf. interval]   M–H weight", bylab, lvl), "\n", sep, "\n", sep = "")
    for (i in seq_len(k)) cat(sprintf("%16s |%12s%13s%11s%13s %s", lab[i], g(o[i]), g(cis[i, 1]), g(cis[i, 2]), g(w[i]), tag), "\n", sep = "")
    cat(sep, "\n", sprintf("%16s |%12s%13s%11s%13s ", "Crude", g(crude), g(cci[1]), g(cci[2]), ""), "\n", sep = "")
    cat(mhrow("%12s%13s%11s", g(est), g(eci[1]), g(eci[2])), "\n", dash, "\n", sep = "")
    hf <- "%9.3f  Pr>chi2 = %6.4f"
  } else {
    cat("\n", sprintf("%16s | Odds ratio     [%s%% conf. interval]   M–H weight", bylab, lvl), "\n", sep, "\n", sep = "")
    for (i in seq_len(k)) cat(sprintf("%16s |%11s%14s%11s%13s %s", lab[i], g(o[i]), g(cis[i, 1]), g(cis[i, 2]), g(w[i]), tag), "\n", sep = "")
    cat(sep, "\n", sprintf("%16s |%11s%14s%11s%13s %s", "Crude", g(crude), g(cci[1]), g(cci[2]), "", tag), "\n", sep = "")
    cat(mhrow("%11s%14s%11s%13s %s", g(est), g(eci[1]), g(eci[2]), "", if (ci == "tb") tag else ""), "\n", dash, "\n", sep = "")
    hf <- "%9.2f  Pr>chi2 = %6.4f"
  }
  cat("Test of homogeneity (M–H)      chi2(", k - 1, ") =", sprintf(hf, chi2, stats::pchisq(chi2, k - 1, lower.tail = FALSE)), "\n\n", sep = "")
  mhtest()
  invisible(r)
}
# tabodds: the odds of each level of the exposure with Woolf's interval, the test of equal odds ((N - 1)/N
# Pearson's) and the score test for trend
stata_tabodds <- function(d, case, expo, level = 95, show = TRUE) {
  y <- stata_epi_num(d[[case]])
  s <- d[[expo]]
  x <- stata_epi_num(s)
  ok <- !is.na(y) & !is.na(x)
  y <- y[ok] != 0
  x <- x[ok]
  lv <- sort(unique(x))
  k <- length(lv)
  a <- vapply(lv, function(l) sum(y & x == l), 1)
  n <- vapply(lv, function(l) sum(x == l), 1)
  cc <- n - a
  N <- sum(n); A <- sum(a)
  z <- stats::qnorm(1 - (100 - level) / 200)
  E <- n * A / N
  hom <- (N - 1) / N * sum((a - E)^2 / E + (cc - (n - E))^2 / (n - E))
  V <- A * (N - A) / (N^2 * (N - 1))
  U <- sum(lv * (a - E))
  tr <- U^2 / (V * (N * sum(n * lv^2) - sum(n * lv)^2))
  r <- list(chi2_tr = tr, p_trend = stats::pchisq(tr, 1, lower.tail = FALSE), chi2_hom = hom, df_hom = k - 1,
            p_hom = stats::pchisq(hom, k - 1, lower.tail = FALSE))
  if (!show) return(r)
  lab <- vapply(stata_epi_vlab(s, lv, 1e6), function(v) stata_tab_abbrev(v, 9), "")
  lvl <- stata_fmt(level, "%9.0g", trim = TRUE)
  cat("\n", strrep("-", 74), "\n", sprintf("%10s  |      Cases     Controls       Odds      [%s%% conf. interval]", stata_tab_abbrev(expo, 9), lvl), "\n", sep = "")
  cat(strrep("-", 12), "+", strrep("-", 61), "\n", sep = "")
  for (i in seq_len(k)) {
    se <- sqrt(1 / a[i] + 1 / cc[i])
    od <- a[i] / cc[i]
    cat(sprintf("%11s |%11.0f%13.0f%11.5f%15.5f%10.5f", lab[i], a[i], cc[i], od, exp(log(od) - z * se), exp(log(od) + z * se)), "\n", sep = "")
  }
  cat(strrep("-", 74), "\n", sep = "")
  # rounded from the double's exact digits: R's printf on Windows rounds 0.66500000000000004 (the double
  # of .665, a little above it) down at two places, as if from 17 digits; Stata rounds it up
  fx <- function(v, d) {
    if (!is.finite(v)) return(sprintf("%9s", "."))
    s <- sprintf("%.45f", abs(v))
    cut <- regexpr(".", s, fixed = TRUE) + d
    rest <- substring(s, cut + 1)
    up <- substr(rest, 1, 1) > "5" || (substr(rest, 1, 1) == "5" && grepl("[1-9]", substring(rest, 2)))
    q <- as.numeric(substr(s, 1, cut)) + if (up) 10^-d else 0
    sprintf(paste0("%9.", d, "f"), sign(v) * q)
  }
  cat("Test of homogeneity (equal odds): chi2(", k - 1, ")  =", fx(hom, 2), "\n", sep = "")
  cat(strrep(" ", 34), "Pr>chi2  =", fx(r$p_hom, 4), "\n\n", sep = "")
  cat("Score test for trend of odds:     chi2(1)  =", fx(tr, 2), "\n", sep = "")
  cat(strrep(" ", 34), "Pr>chi2  =", fx(r$p_trend, 4), "\n\n", sep = "")
  invisible(r)
}"##;

/// The r() each leaves (Stata 19).
const R_CS: &[&str] = &[
    "afp", "ub_afe", "lb_afe", "afe", "ub_rr", "lb_rr", "rr", "ub_rd", "lb_rd", "rd", "p", "chi2",
];
const R_CS_EXACT: &[&str] = &[
    "afp", "ub_afe", "lb_afe", "afe", "ub_rr", "lb_rr", "rr", "ub_rd", "lb_rd", "rd", "p_exact",
    "p1_exact",
];
const R_CC: &[&str] = &[
    "afp", "ub_afe", "lb_afe", "chi2", "p", "afe", "ub_or", "lb_or", "or",
];
const R_IR: &[&str] = &[
    "ird",
    "lb_ird",
    "ub_ird",
    "irr",
    "lb_irr",
    "ub_irr",
    "afe",
    "lb_afe",
    "ub_afe",
    "afp",
    "p_lower_midp",
    "p_upper_midp",
    "p_twosided_midp",
    "p_lower_exact",
    "p_upper_exact",
    "p_twosided_exact",
];

/// by(): the r() each stratified form leaves (Stata 19).
const R_CS_BY: &[&str] = &[
    "chi2_mh", "df", "ub_rr", "lb_rr", "rr", "afp", "ub_crude", "lb_crude", "crude",
];
const R_CS_ISTD: &[&str] = &[
    "ub_rr", "lb_rr", "rr", "afp", "ub_crude", "lb_crude", "crude",
];
const R_CS_BY_OR: &[&str] = &[
    "chi2_mh", "df", "ub_or", "lb_or", "or", "chi2", "afp", "ub_crude", "lb_crude", "crude",
];
const R_CC_BY: &[&str] = &[
    "ub_or", "lb_or", "or", "chi2_p", "df", "chi2", "afp", "ub_crude", "lb_crude", "crude",
];
const R_TABODDS: &[&str] = &["chi2_tr", "p_trend", "chi2_hom", "df_hom", "p_hom"];
const R_IR_BY: &[&str] = &[
    "irr",
    "lb_irr",
    "ub_irr",
    "crude",
    "lb_crude",
    "ub_crude",
    "pooled",
    "lb_pooled",
    "ub_pooled",
    "df",
    "chi2_mh",
];

impl Session {
    pub(super) fn epitab(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let cmd = c.name.as_str();
        self.no_by(c, cmd)?;
        if c.parts.weight.is_some() {
            return Err(no(format!("{cmd} with weights is not translated yet")));
        }
        let base = cmd.trim_end_matches('i');
        let immediate = cmd.ends_with('i');
        let by = opts.arg("by", 2).map(|g| g.trim().to_string());
        let known: &[(&str, usize)] = match (base, by.is_some()) {
            ("cs", false) => &[("level", 1), ("exact", 5)],
            ("cs", true) => &[
                ("level", 1),
                ("by", 2),
                ("or", 2),
                ("woolf", 5),
                ("istandard", 9),
            ],
            ("cc", false) => &[("level", 1), ("woolf", 5), ("tb", 2)],
            ("cc", true) => &[("level", 1), ("woolf", 5), ("tb", 2), ("by", 2)],
            (_, false) => &[("level", 1)],
            (_, true) => &[("level", 1), ("by", 2)],
        };
        if let Some(o) = opts.other(known) {
            return Err(no(format!(
                "{cmd} with the option {o} is not translated yet"
            )));
        }
        if let Some(g) = &by {
            if immediate || !is_name(g) {
                return Err(no(format!(
                    "{cmd}, by(): one stratifying variable is translated"
                )));
            }
            // cs, or: each stratum's interval is Cornfield's (Stata's own search)
            // unless woolf
            if base == "cs" && opts.has("or", 2) && !opts.has("woolf", 5) {
                return Err(no(format!(
                    "{cmd}, by() or: the strata's Cornfield intervals (Stata 19's own search) are not translated yet; or woolf is"
                )));
            }
            if base == "cs" && opts.has("or", 2) && opts.has("istandard", 9) {
                return Err(no(format!(
                    "{cmd}, by() with or and istandard is not translated yet"
                )));
            }
            if base == "cs" && opts.has("woolf", 5) && !opts.has("or", 2) {
                return Err(no(format!(
                    "{cmd}, by() woolf without or is not translated yet"
                )));
            }
        }
        if base == "cc" && !opts.has("woolf", 5) && !opts.has("tb", 2) {
            return Err(no(format!(
                "{cmd}'s exact interval of the odds ratio (its default) is not translated yet: Stata 19's search for it stops short of the exact bound; woolf and tb are translated"
            )));
        }
        if opts.has("woolf", 5) && opts.has("tb", 2) {
            return Err(no(format!("{cmd}: woolf and tb together (Stata: r(198))")));
        }
        let level = match opts.arg("level", 1) {
            None => "95".to_string(),
            Some(l) => {
                let v: f64 = l
                    .trim()
                    .parse()
                    .map_err(|_| no(format!("level({l}) needs a number")))?;
                if !(10.0..=99.99).contains(&v) {
                    return Err(no(format!(
                        "level({l}): levels from 10 to 99.99 are translated"
                    )));
                }
                r_number(v)
            }
        };
        let main = c.parts.main.trim();
        let words: Vec<&str> = main.split_whitespace().collect();
        let mut extra = format!(", level = {level}");
        if base == "cc" && opts.has("tb", 2) {
            extra.push_str(", ci = \"tb\"");
        }
        if base == "cs" && opts.has("exact", 5) {
            extra.push_str(", exact = TRUE");
        }
        if self.quiet(c) {
            extra.push_str(", show = FALSE");
        }
        let kind = r_text(base);
        let call = if immediate {
            if words.len() != 4 {
                return Err(no(format!("{cmd} takes four numbers (Stata: r(198))")));
            }
            let mut v = Vec::new();
            for w in &words {
                let x: f64 = w
                    .parse()
                    .ok()
                    .filter(|x: &f64| x.is_finite() && *x >= 0.0)
                    .ok_or_else(|| no(format!("{cmd} {main}: four counts are translated")))?;
                v.push(r_number(x));
            }
            let [a, b, c3, d] = v.as_slice() else {
                return Err(no(format!("{cmd} takes four numbers (Stata: r(198))")));
            };
            if base == "ir" {
                format!("stata_epi({kind}, {a}, {b}, t1 = {c3}, t0 = {d}{extra})")
            } else {
                format!("stata_epi({kind}, {a}, {b}, {c3}, {d}{extra})")
            }
        } else {
            let need = if base == "ir" { 3 } else { 2 };
            if words.len() != need || words.iter().any(|w| !is_name(w)) {
                return Err(no(format!(
                    "{cmd}: {} variable names written out are translated",
                    if need == 3 {
                        "the case, exposure and time"
                    } else {
                        "the case and exposure"
                    }
                )));
            }
            let rows = self.rows_for(c, out)?;
            let q: Vec<String> = words.iter().map(|w| r_text(w)).collect();
            if let Some(g) = &by {
                let time = match q.get(2) {
                    Some(t) => format!(", time = {t}"),
                    None => String::new(),
                };
                let mut more = String::new();
                if opts.has("or", 2) {
                    more.push_str(", or = TRUE");
                }
                if opts.has("istandard", 9) {
                    more.push_str(", istd = TRUE");
                }
                let (case, expo) = (
                    q.first().cloned().unwrap_or_default(),
                    q.get(1).cloned().unwrap_or_default(),
                );
                for h in [
                    "stata_epitab",
                    "stata_fmt",
                    "stata_returned",
                    "stata_tables",
                ] {
                    self.helpers.insert(h);
                }
                out.line(format!(
                    "stata_r <- stata_epi_by({kind}, {rows}, {case}, {expo}, {}{time}{more}{extra})",
                    r_text(g)
                ));
                let names = match base {
                    "cs" if opts.has("istandard", 9) => R_CS_ISTD,
                    "cs" if opts.has("or", 2) => R_CS_BY_OR,
                    "cs" => R_CS_BY,
                    "cc" => R_CC_BY,
                    _ => R_IR_BY,
                };
                self.r_new = Some((names, cmd.into()));
                return Ok(());
            }
            out.line(format!(
                "stata_epi_n <- stata_epi_counts({rows}, {})",
                q.join(", ")
            ));
            if base == "ir" {
                format!("stata_epi({kind}, stata_epi_n$a, stata_epi_n$b, t1 = stata_epi_n$t1, t0 = stata_epi_n$t0, expo = stata_epi_n$expo, case = stata_epi_n$case, time = stata_epi_n$time{extra})")
            } else {
                format!("stata_epi({kind}, stata_epi_n$a, stata_epi_n$b, stata_epi_n$c, stata_epi_n$d, expo = stata_epi_n$expo{extra})")
            }
        };
        for h in ["stata_epitab", "stata_fmt", "stata_returned"] {
            self.helpers.insert(h);
        }
        out.line(format!("stata_r <- {call}"));
        let names = match base {
            "cs" if opts.has("exact", 5) => R_CS_EXACT,
            "cs" => R_CS,
            "cc" => R_CC,
            _ => R_IR,
        };
        self.r_new = Some((names, cmd.into()));
        Ok(())
    }

    /// `tabodds case exposure [if] [in] [, level(#)]`.
    pub(super) fn tabodds(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "tabodds")?;
        if c.parts.weight.is_some() {
            return Err(no("tabodds with weights is not translated yet"));
        }
        // or: Stata 19's intervals of the odds ratios are wider than Woolf's (as
        // mhodds's are), by a factor not worked out yet
        if opts.has("or", 2) {
            return Err(no(
                "tabodds, or is not translated yet: Stata 19's intervals of the odds ratios are wider than Woolf's by a factor not worked out yet",
            ));
        }
        if let Some(o) = opts.other(&[("level", 1)]) {
            return Err(no(format!(
                "tabodds with the option {o} is not translated yet"
            )));
        }
        let words: Vec<&str> = c.parts.main.split_whitespace().collect();
        let [case, expo] = words.as_slice() else {
            return Err(no(
                "tabodds: the case and exposure variable names written out are translated",
            ));
        };
        if !is_name(case) || !is_name(expo) {
            return Err(no(
                "tabodds: the case and exposure variable names written out are translated",
            ));
        }
        let mut extra = String::new();
        if let Some(l) = opts.arg("level", 1) {
            let v: f64 = l
                .trim()
                .parse()
                .map_err(|_| no(format!("level({l}) needs a number")))?;
            if !(10.0..=99.99).contains(&v) {
                return Err(no(format!(
                    "level({l}): levels from 10 to 99.99 are translated"
                )));
            }
            extra.push_str(&format!(", level = {}", r_number(v)));
        }
        if self.quiet(c) {
            extra.push_str(", show = FALSE");
        }
        let rows = self.rows_for(c, out)?;
        for h in [
            "stata_epitab",
            "stata_fmt",
            "stata_returned",
            "stata_tables",
        ] {
            self.helpers.insert(h);
        }
        out.line(format!(
            "stata_r <- stata_tabodds({rows}, {}, {}{extra})",
            r_text(case),
            r_text(expo)
        ));
        self.r_new = Some((R_TABODDS, "tabodds".into()));
        Ok(())
    }
}
