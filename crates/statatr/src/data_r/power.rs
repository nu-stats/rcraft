//! `power onemean | twomeans | pairedmeans | oneproportion | twoproportions |
//! onevariance | twovariances | onecorrelation | twocorrelations |
//! pairedproportions`: the sample size, the power or the target value, as
//! Stata 19 works them out and prints them (`plan/stata/power.md`). The power
//! functions are the [PSS] manual's: t (noncentral t) or z tests of means,
//! Satterthwaite's t for unequal standard deviations, the score or Wald test
//! of one proportion, Pearson's chi-squared (the pooled z) test of two with
//! Fleiss's continuity correction, the chi-squared test of a variance, the F
//! test of two, Fisher's z test of correlations and the large-sample McNemar
//! test. Sample sizes are the smallest whole numbers reaching the power (the
//! second group's from the first's, rounded up, times nratio()), the target
//! values R's root of the power function or Stata's closed forms.

use super::*;

pub(super) const POWER_HELPER: &str = r##"
stata_power <- function(method, v1 = NULL, v2 = NULL, alpha = 0.05, power = NULL, n = NULL, n1 = NULL, n2 = NULL,
                        nratio = NULL, onesided = FALSE, direction = "upper", sd = NULL, sd1 = NULL, sd2 = NULL,
                        known = FALSE, sddiff = NULL, corr = NULL, test = "score", continuity = FALSE,
                        effect = "diff", scale = "variance", prdiscordant = NULL, show = TRUE, table_row = FALSE) {
  if (method %in% c("onevariance", "twovariances", "onecorrelation", "twocorrelations", "pairedproportions"))
    return(stata_power_g(method, v1, v2, alpha, power, n, n1, n2, nratio, onesided, direction, scale, effect,
                         prdiscordant, show))
  two <- method %in% c("twomeans", "twoproportions")
  props <- method %in% c("oneproportion", "twoproportions")
  stop_r <- function(m) stop(m, call. = FALSE)
  # what is worked out: the sample size, the power, or the effect (the second value)
  find <- if (is.null(v2)) "effect" else if (is.null(power) && (!is.null(n) || !is.null(n1) || !is.null(n2))) "power" else "n"
  if (find == "effect" && (is.null(power) || (is.null(n) && is.null(n1)))) stop_r("power: the target needs n() and power() (Stata: r(198))")
  target <- if (is.null(power)) 0.8 else power
  if (is.null(nratio)) nratio <- if (!is.null(n1) && !is.null(n2)) n2 / n1 else 1
  # the standard deviations
  unequal <- FALSE
  if (method == "onemean") s <- if (is.null(sd)) 1 else sd
  if (method == "pairedmeans") {
    if (!is.null(corr)) {
      sdc <- if (is.null(sd)) 1 else sd
      s <- sdc * sqrt(2 * (1 - corr))
    } else s <- if (is.null(sddiff)) 1 else sddiff
  }
  if (method == "twomeans") {
    unequal <- !is.null(sd1) || !is.null(sd2)
    if (unequal) {
      sd1 <- if (is.null(sd1)) 1 else sd1
      sd2 <- if (is.null(sd2)) 1 else sd2
      unequal <- sd1 != sd2
    }
    if (!unequal) {
      s <- if (!is.null(sd)) sd else if (!is.null(sd1)) sd1 else 1
      sd1 <- s
      sd2 <- s
    }
  }
  side <- if (onesided) 1 else 2
  zc <- stats::qnorm(1 - alpha / side)
  # the power at group sizes a (and b) for the second value x
  pw <- function(a, b, x) suppressWarnings(pw0(a, b, x))
  pw0 <- function(a, b, x) {
    up <- function(cut, df, l) if (is.null(df)) 1 - stats::pnorm(cut - l) else 1 - stats::pt(cut, df, l)
    dn <- function(cut, df, l) if (is.null(df)) stats::pnorm(-cut - l) else stats::pt(-cut, df, l)
    tails <- function(l, df) {
      # Stata's noncentral t is defined for noncentralities up to 1000, past them the power is missing
      if (!is.null(df) && abs(l) > 1000) return(NA_real_)
      cut <- if (is.null(df)) zc else stats::qt(1 - alpha / side, df)
      # R's noncentral t can pass 1 by 1e-11 at large n, where Stata's power is 1
      min(1, if (!onesided) up(cut, df, l) + dn(cut, df, l) else if (l >= 0) up(cut, df, l) else dn(cut, df, l))
    }
    if (method %in% c("onemean", "pairedmeans")) return(tails(sqrt(a) * (x - v1) / s, if (known) NULL else a - 1))
    if (method == "twomeans") {
      if (!unequal) return(tails((x - v1) / (s * sqrt(1 / a + 1 / b)), if (known) NULL else a + b - 2))
      va <- sd1^2 / a
      vb <- sd2^2 / b
      return(tails((x - v1) / sqrt(va + vb), if (known) NULL else (va + vb)^2 / (va^2 / (a - 1) + vb^2 / (b - 1))))
    }
    if (method == "oneproportion") {
      s0 <- sqrt(v1 * (1 - v1))
      sa <- sqrt(x * (1 - x))
      d <- x - v1
      if (test == "wald") s0 <- sa
      upp <- stats::pnorm((sqrt(a) * d - zc * s0) / sa)
      low <- stats::pnorm((-sqrt(a) * d - zc * s0) / sa)
      return(if (!onesided) upp + low else if (d >= 0) upp else low)
    }
    pb <- (a * v1 + b * x) / (a + b)
    d <- x - v1
    s0 <- sqrt(pb * (1 - pb) * (1 / a + 1 / b))
    sa <- sqrt(v1 * (1 - v1) / a + x * (1 - x) / b)
    # Fleiss's continuity correction, taken off in both tails (Stata 19)
    cc <- if (continuity) (1 / a + 1 / b) / 2 else 0
    upp <- stats::pnorm((abs(d) - cc - zc * s0) / sa)
    low <- stats::pnorm((-abs(d) - cc - zc * s0) / sa)
    if (!onesided) upp + low else upp
  }
  # the group sizes from n(), n1(), n2(); a total that does not split into whole groups at
  # nratio() is cut to the groups rounded down, as Stata 19's "Actual sample sizes"
  actual <- FALSE
  sizes <- function() {
    if (!two) return(c(n, NA))
    if (!is.null(n1) && !is.null(n2)) return(c(n1, n2))
    a <- n / (1 + nratio)
    if (abs(a - round(a)) < 1e-8 && abs(nratio * round(a) - (n - round(a))) < 1e-8) return(c(round(a), n - round(a)))
    actual <<- TRUE
    a <- floor(a + 1e-9)
    # the second group from the total (Stata 19: n(50) nratio(2) is 16 and 33)
    c(a, floor(n * nratio / (1 + nratio) + 1e-9))
  }
  tol <- 1e-12
  iterated <- TRUE
  closed <- onesided && (known || props) && !(find == "effect" && props)
  if (find == "power") {
    g <- sizes()
    pv <- pw(g[1], g[2], v2)
    # Stata 19 stops there for two means; one sample (or paired) shows a missing power
    if (is.na(pv) && two && !table_row) stop_r("failure to compute power; the evaluation of the power function resulted in a missing value: Stata's noncentral t takes a noncentrality up to 1000 (Stata: r(498))")
    N <- if (!two) g[1] else if (actual) n else sum(g)
    x2 <- v2
    iterated <- FALSE
  } else if (find == "n") {
    f <- function(a) pw(a, nratio * a, v2) - target
    # the smallest group where the test is defined (a t test needs 2; with continuity the corrected
    # difference must stay above 0)
    lo <- if (known || props) 1 else 2 + 1e-9
    if (continuity) lo <- max(lo, (1 + 1 / nratio) / (2 * abs(v2 - v1)) + 1e-6)
    if (f(lo) >= 0) a <- lo else a <- stats::uniroot(f, c(lo, 10), extendInt = "upX", tol = tol)$root
    if (two) {
      A <- ceiling(a - 1e-9)
      g <- c(A, ceiling(nratio * A - 1e-9))
      N <- sum(g)
    } else {
      g <- c(ceiling(a - 1e-9), NA)
      N <- g[1]
    }
    pv <- target
    x2 <- v2
    iterated <- !closed
  } else {
    g <- sizes()
    N <- if (!two) g[1] else if (actual) n else sum(g)
    upper <- direction != "lower"
    f <- function(x) pw(g[1], g[2], x) - target
    if (props) {
      eps <- 1e-10
      x2 <- if (upper) stats::uniroot(f, c(v1 + eps, 1 - eps), tol = tol)$root else stats::uniroot(f, c(eps, v1 - eps), tol = tol)$root
    } else {
      sc <- if (method == "twomeans" && unequal) max(sd1, sd2) else s
      x2 <- if (upper) stats::uniroot(f, c(v1 + 1e-9 * sc, v1 + sc), extendInt = "upX", tol = tol)$root else
        stats::uniroot(f, c(v1 - sc, v1 - 1e-9 * sc), extendInt = "downX", tol = tol)$root
    }
    pv <- target
    iterated <- !closed
  }
  diff <- x2 - v1
  delta <- switch(method, onemean = diff / s, pairedmeans = diff / s, twomeans = diff, oneproportion = diff,
                  twoproportions = switch(effect, diff = diff, ratio = x2 / v1, oratio = (x2 / (1 - x2)) / (v1 / (1 - v1))))
  # r() in the order of Stata 19's return list for this form
  it <- if (iterated) list(converged = 1, maxiter = 500, tolerance = 1e-12, ftolerance = 1e-12) else list()
  head <- list(power = pv, beta = 1 - pv, alpha = alpha)
  nf <- if (find != "power" && method %in% c("onemean", "pairedmeans", "oneproportion") || two) list(nfractional = 0) else list()
  if (method %in% c("onemean", "pairedmeans", "oneproportion") && find != "n") nf <- list()
  os <- list(onesided = as.numeric(onesided))
  # the total given, then (when it did not split) the total of the groups
  NN <- c(list(N = N), if (actual) list(N_a = sum(g)))
  # the groups' own ratio, when it misses nratio() (a rounded sample size, a total that does not split)
  nra <- if (two && abs(g[2] / g[1] - nratio) > 1e-12) list(nratio_a = g[2] / g[1])
  ks <- if (method == "twomeans") (if (known) list(knownsds = 1) else list(knownsds = 0, unequal = as.numeric(unequal))) else list()
  r <- switch(method,
    onemean = {
      tail <- if (iterated && find == "n") list(m0 = v1, delta = delta, knownsd = as.numeric(known), sd = s, diff = diff, ma = x2)
        else if (iterated) list(m0 = v1, ma = x2, delta = delta, knownsd = as.numeric(known), sd = s, diff = diff)
        else list(m0 = v1, ma = x2, diff = diff, sd = s, delta = delta, knownsd = as.numeric(known))
      c(head, list(N = N), nf, os, it, tail)
    },
    pairedmeans = {
      sds <- if (is.null(corr)) list(sd_d = s) else list(corr = corr, sd2 = sdc, sd1 = sdc, sd = sdc, sd_d = s)
      tail <- if (find == "power") c(list(d0 = 0, da = diff, ma1 = v1, ma2 = x2, sd_d = s),
                                     if (!is.null(corr)) list(sd = sdc, sd1 = sdc, sd2 = sdc, corr = corr),
                                     list(delta = delta, knownsd = as.numeric(known)))
        else if (find == "n") c(list(d0 = 0, delta = delta, knownsd = as.numeric(known)), sds, list(ma2 = x2, ma1 = v1, da = diff))
        else c(list(d0 = 0, da = diff, delta = delta, knownsd = as.numeric(known)), sds, list(ma2 = x2, ma1 = v1))
      c(head, list(N = N), nf, os, it, tail)
    },
    oneproportion = {
      tail <- if (!iterated) list(p0 = v1, pa = x2, delta = delta, diff = diff, continuity = 0)
        else if (find == "n") list(p0 = v1, delta = delta, continuity = 0, diff = diff, pa = x2)
        else list(p0 = v1, pa = x2, delta = delta, continuity = 0, diff = diff)
      c(head, list(N = N), nf, os, it, tail)
    },
    twomeans = {
      sdl <- if (unequal) list(sd1 = sd1, sd2 = sd2) else list(sd = s, sd1 = sd1, sd2 = sd2)
      sdr <- if (unequal) list(sd2 = sd2, sd1 = sd1) else list(sd2 = sd2, sd1 = sd1, sd = s)
      ksr <- rev(ks)
      tail <- if (!iterated) c(NN, list(N1 = g[1], N2 = g[2], nratio = nratio), nra, list(m1 = v1, m2 = x2, diff = diff), sdl, ks)
        else if (find == "n") c(list(N = N), ksr, sdr, list(diff = diff, m2 = x2, m1 = v1), nra, list(nratio = nratio, N2 = g[2], N1 = g[1]))
        else c(NN, list(N1 = g[1]), ksr, sdr, list(diff = diff, m2 = x2, m1 = v1), nra, list(nratio = nratio, N2 = g[2]))
      c(head, list(delta = delta), nf, os, it, tail)
    },
    twoproportions = {
      tail <- if (!iterated) c(NN, list(N1 = g[1], N2 = g[2], nratio = nratio), nra, list(p1 = v1, p2 = x2, continuity = as.numeric(continuity)))
        else if (find == "n") c(list(N = N, continuity = as.numeric(continuity), p2 = x2, p1 = v1), nra,
                                list(nratio = nratio, N2 = g[2], N1 = g[1]))
        else c(NN, list(N1 = g[1], continuity = as.numeric(continuity), p2 = x2, p1 = v1), nra, list(nratio = nratio, N2 = g[2]))
      c(head, list(delta = delta), nf, os, it, tail)
    })
  r <- c(r, list(type = "test", method = method), if (find == "effect") list(direction = if (direction == "lower") "lower" else "upper"),
         if (method == "oneproportion") list(test = test), if (method == "twoproportions") list(test = "chi2", effect = effect))
  # the printout
  title <- switch(method, onemean = "one-sample mean test", pairedmeans = "two-sample paired-means test",
                  twomeans = "two-sample means test", oneproportion = "one-sample proportion test",
                  twoproportions = "two-sample proportions test")
  what <- if (find == "power") "power" else if (find == "n") (if (two) "sample sizes" else "sample size") else
    switch(method, onemean = "target mean", pairedmeans = "target parameters", twomeans = "experimental-group mean",
           oneproportion = "target proportion", twoproportions = "experimental-group proportion")
  testline <- switch(method,
    onemean = if (known) "z test" else "t test",
    pairedmeans = paste0(if (known) "Paired z test" else "Paired t test", if (!is.null(corr)) " assuming sd1 = sd2 = sd" else ""),
    twomeans = if (unequal) (if (known) "z test assuming unequal variances" else "Satterthwaite's t test assuming unequal variances")
               else if (known) "z test assuming sd1 = sd2 = sd" else "t test assuming sd1 = sd2 = sd",
    oneproportion = if (test == "wald") "Wald z test" else "Score z test",
    twoproportions = "Pearson's chi-squared test ")
  sym <- switch(method, onemean = c("m", "m0", "ma"), pairedmeans = c("d", "d0", "da"), twomeans = c("m2", "m1", "m2"),
                oneproportion = c("p", "p0", "pa"), twoproportions = c("p2", "p1", "p2"))
  rel <- if (!onesided) "!=" else if (diff >= 0) ">" else "<"
  h0 <- sprintf("H0: %s = %s  versus  Ha: %s %s %s", sym[1], sym[2], sym[1], rel, sym[2])
  # the target's side: always after one sample, after two only when two-sided (Stata 19)
  if (find == "effect" && (!onesided || !two)) h0 <- paste0(h0, "; ", sym[3], if (direction == "lower") " < " else " > ", sym[2])
  # the head of the printout, for a table of several (stata_power_table)
  attr(r, "stata_head") <- c(paste0("Estimated ", what, " for a ", title), testline, h0)
  attr(r, "stata_find") <- find
  if (!show) return(invisible(r))
  num <- function(k, v, extra = "") if (is.na(v)) sprintf("%13s = %9s%s", k, ".", extra) else sprintf("%13s = %9.4f%s", k, v, extra)
  int <- function(k, v) sprintf("%13s = %9s", k, formatC(v, format = "f", digits = 0, big.mark = ","))
  eff <- if (method == "twoproportions") paste0("  (", switch(effect, diff = "difference", ratio = "ratio", oratio = "odds ratio"), ")") else ""
  sizes_lines <- function(tot = N) {
    if (!two) return(int("N", tot))
    c(int("N", tot), if (g[1] == g[2] && is.null(n1)) int("N per group", g[1]) else c(int("N1", g[1]), int("N2", g[2])))
  }
  given_n <- if (!two) int("N", N) else if (actual) c(int("N", N), if (nratio != 1) num("N2/N1", nratio)) else
    c(sizes_lines(), if (g[1] != g[2]) num("N2/N1", g[2] / g[1]))
  given <- c(num("alpha", alpha), if (find != "power") num("power", target), if (find != "n") given_n,
             if (find != "effect") num("delta", delta, eff))
  right <- character()
  vals <- switch(method,
    onemean = c(num("m0", v1), if (find != "effect") num("ma", x2), num("sd", s)),
    pairedmeans = {
      right <- c(num("ma1", v1), if (find != "effect") num("ma2", x2), if (!is.null(corr)) c(num("sd", sdc), num("corr", corr)))
      c(num("d0", 0), if (find != "effect") num("da", diff), num("sd_d", s))
    },
    twomeans = c(num("m1", v1), if (find != "effect") num("m2", x2), if (unequal) c(num("sd1", sd1), num("sd2", sd2)) else num("sd", s),
                 if (find == "n" && nratio != 1) num("N2/N1", nratio)),
    oneproportion = c(num("p0", v1), if (find != "effect") num("pa", x2)),
    twoproportions = c(num("p1", v1), if (find != "effect") num("p2", x2), if (find == "n" && nratio != 1) num("N2/N1", nratio)))
  left <- c(given, vals)
  if (length(right)) left <- paste0(left, c(right, rep("", length(left) - length(right))))
  cat("\n")
  if (iterated) cat("Performing iteration ...\n\n")
  cat("Estimated ", what, " for a ", title, "\n", testline, "\n", h0, "\n\nStudy parameters:\n\n", sep = "")
  cat(left, sep = "\n")
  if (actual) {
    cat("\nActual sample sizes:\n\n")
    cat(sizes_lines(sum(g)), sep = "\n")
    if (g[1] != g[2] && abs(g[2] / g[1] - nratio) > 1e-12) cat(num("N2/N1", g[2] / g[1]), "\n", sep = "")
  }
  if (find == "power") {
    cat("\nEstimated power:\n\n", num("power", pv), "\n", sep = "")
  } else if (find == "n") {
    cat("\nEstimated ", what, ":\n\n", sep = "")
    cat(sizes_lines(), sep = "\n")
    if (two && g[1] != g[2] && abs(g[2] / g[1] - nratio) > 1e-12) cat(num("N2/N1", g[2] / g[1]), "\n", sep = "")
  } else {
    cat("\nEstimated effect size and ", what, ":\n\n", sep = "")
    cat(num("delta", delta, eff), "\n", sep = "")
    cat(switch(method, onemean = num("ma", x2), pairedmeans = c(num("da", diff), num("ma2", x2)), twomeans = num("m2", x2),
               oneproportion = num("pa", x2), twoproportions = num("p2", x2)), sep = "\n")
  }
  cat("\n")
  invisible(r)
}

# power onevariance | twovariances | onecorrelation | twocorrelations | pairedproportions (Stata 19): the
# chi-squared test of a variance, the F test of two (on n1 - 1 and n2 - 1 df, as Stata), Fisher's z test of
# one or two correlations, the large-sample McNemar test of discordant proportions
stata_power_g <- function(method, v1, v2, alpha, power, n, n1, n2, nratio, onesided, direction, scale, effect,
                          prdisc, show) {
  stop_r <- function(m) stop(m, call. = FALSE)
  two <- method %in% c("twovariances", "twocorrelations")
  pp <- method == "pairedproportions"
  sdsc <- scale == "standard deviation"
  # p12 + p21 given: the discordant proportions are what is worked out
  pdm <- pp && !is.null(prdisc)
  find <- if (is.null(v2)) "effect" else if (is.null(power) && (!is.null(n) || !is.null(n1))) "power" else "n"
  if (find == "effect" && (is.null(power) || (is.null(n) && is.null(n1)))) stop_r("power: the target needs n() and power() (Stata: r(198))")
  target <- if (is.null(power)) 0.8 else power
  if (is.null(nratio)) nratio <- if (!is.null(n1) && !is.null(n2)) n2 / n1 else 1
  side <- if (onesided) 1 else 2
  a2 <- alpha / side
  zc <- stats::qnorm(1 - a2)
  # the power at group sizes a (and b) for the alternative's value x
  pw <- function(a, b, x) suppressWarnings(pw0(a, b, x))
  pw0 <- function(a, b, x) {
    tails <- function(up, lo, ge) if (!onesided) up + lo else if (ge) up else lo
    if (method %in% c("onevariance", "twovariances")) {
      r <- if (sdsc) (x / v1)^2 else x / v1
      if (method == "onevariance") {
        d <- a - 1
        return(tails(stats::pchisq(stats::qchisq(1 - a2, d) / r, d, lower.tail = FALSE),
                     stats::pchisq(stats::qchisq(a2, d) / r, d), r >= 1))
      }
      d1 <- a - 1
      d2 <- b - 1
      return(tails(stats::pf(stats::qf(1 - a2, d1, d2) / r, d1, d2, lower.tail = FALSE),
                   stats::pf(stats::qf(a2, d1, d2) / r, d1, d2), r >= 1))
    }
    if (!pp) {
      se <- if (two) sqrt(1 / (a - 3) + 1 / (b - 3)) else 1 / sqrt(a - 3)
      l <- (atanh(x) - atanh(v1)) / se
      return(tails(stats::pnorm(l - zc), stats::pnorm(-l - zc), l >= 0))
    }
    p12 <- if (pdm) prdisc - x else v1
    pd <- p12 + x
    d <- x - p12
    s <- sqrt(pd - d^2)
    # one-sided, the power alone takes the upper tail whatever the sign of p21 - p12 (Stata 19)
    tails(stats::pnorm((d * sqrt(a) - zc * sqrt(pd)) / s), stats::pnorm((-d * sqrt(a) - zc * sqrt(pd)) / s), d >= 0 || find == "power")
  }
  # the smallest group each test is defined for
  least <- switch(method, onevariance = 2, twovariances = 2, onecorrelation = 4, twocorrelations = 4, pairedproportions = 1)
  actual <- FALSE
  sizes <- function() {
    if (!two) return(c(n, NA))
    if (!is.null(n1) && !is.null(n2)) return(c(n1, n2))
    a <- n / (1 + nratio)
    if (abs(a - round(a)) < 1e-8 && abs(nratio * round(a) - (n - round(a))) < 1e-8) return(c(round(a), n - round(a)))
    actual <<- TRUE
    a <- floor(a + 1e-9)
    # the second group from the total (Stata 19: n(50) nratio(2) is 16 and 33)
    c(a, floor(n * nratio / (1 + nratio) + 1e-9))
  }
  # Stata works the answer out without a search for these one-sided forms
  closed <- onesided && (if (find == "n") method %in% c("onecorrelation", "twocorrelations", "pairedproportions") else !pp)
  iterated <- FALSE
  tol <- 1e-13
  if (find == "power") {
    g <- sizes()
    if (min(g, na.rm = TRUE) < least) stop_r(sprintf("power %s: the groups need at least %d observations (Stata prints a missing power)", method, least))
    x2 <- v2
    pv <- pw(g[1], g[2], x2)
    N <- if (!two) g[1] else if (actual) n else sum(g)
  } else if (find == "n") {
    x2 <- v2
    f <- function(a) pw(a, nratio * a, x2) - target
    lo <- if (two) max(least, least / nratio) else least
    a <- if (f(lo) >= 0) lo else stats::uniroot(f, c(lo, lo + 10), extendInt = "upX", tol = tol)$root
    A <- ceiling(a - 1e-9)
    g <- if (two) c(A, ceiling(nratio * A - 1e-9)) else c(A, NA)
    N <- sum(g, na.rm = TRUE)
    pv <- target
    iterated <- !closed
  } else {
    g <- sizes()
    if (min(g, na.rm = TRUE) < least) stop_r(sprintf("power %s: the groups need at least %d observations", method, least))
    N <- if (!two) g[1] else if (actual) n else sum(g)
    a <- g[1]
    b <- g[2]
    up <- direction != "lower"
    f <- function(x) pw(a, b, x) - target
    zb <- stats::qnorm(target)
    if (closed && method == "onevariance") {
      d <- a - 1
      r <- if (up) stats::qchisq(1 - alpha, d) / stats::qchisq(1 - target, d) else stats::qchisq(alpha, d) / stats::qchisq(target, d)
      x2 <- if (sdsc) v1 * sqrt(r) else v1 * r
    } else if (closed && method == "twovariances") {
      r <- if (up) stats::qf(1 - alpha, a - 1, b - 1) / stats::qf(1 - target, a - 1, b - 1) else
        stats::qf(alpha, a - 1, b - 1) / stats::qf(target, a - 1, b - 1)
      x2 <- if (sdsc) v1 * sqrt(r) else v1 * r
    } else if (closed) {
      se <- if (two) sqrt(1 / (a - 3) + 1 / (b - 3)) else 1 / sqrt(a - 3)
      x2 <- tanh(atanh(v1) + (if (up) 1 else -1) * (stats::qnorm(1 - alpha) + zb) * se)
    } else if (pp) {
      e <- 1e-12
      ends <- if (up) c(prdisc / 2 + e, prdisc - e) else c(e, prdisc / 2 - e)
      if (f(ends[if (up) 2 else 1]) < 0) stop_r("power pairedproportions: no discordant proportions with that sum reach the power at this n()")
      x2 <- stats::uniroot(f, ends, tol = 1e-15)$root
    } else {
      # searched on the log scale (variances) or Fisher's z (correlations)
      to <- if (method %in% c("onevariance", "twovariances")) function(t) v1 * exp(t) else function(t) tanh(atanh(v1) + t)
      h <- function(t) f(to(t))
      t <- if (up) stats::uniroot(h, c(1e-10, 1), extendInt = "upX", tol = tol)$root else
        stats::uniroot(h, c(-1, -1e-10), extendInt = "downX", tol = tol)$root
      x2 <- to(t)
    }
    if (pdm) v1 <- prdisc - x2
    pv <- target
    iterated <- !closed
  }
  delta <- if (method %in% c("onevariance", "twovariances")) x2 / v1 else if (pp && effect == "ratio") x2 / v1 else x2 - v1
  # r() in the order of Stata 19's return list for this form
  nm <- switch(method, onevariance = if (sdsc) c("s0", "sa") else c("v0", "va"),
               twovariances = if (sdsc) c("s1", "s2") else c("v1", "v2"), onecorrelation = c("r0", "ra"),
               twocorrelations = c("r1", "r2"), pairedproportions = c("p12", "p21"))
  P <- stats::setNames(list(v1, x2), nm)
  it <- if (iterated) list(converged = 1, maxiter = 500, tolerance = 1e-12, ftolerance = 1e-12) else list()
  head <- list(power = pv, beta = 1 - pv, alpha = alpha)
  os <- list(onesided = as.numeric(onesided))
  if (!two) {
    tail <- switch(method,
      onevariance = if (find == "n") c(P[1], list(delta = delta, ratio = delta), P[2]) else c(P, list(delta = delta, ratio = delta)),
      onecorrelation = if (find == "n" && iterated) c(P[1], list(delta = delta), P[2]) else c(P, list(delta = delta)),
      pairedproportions = c(list(delta = delta), P, if (pdm) list(prdiscordant = prdisc)))
    r <- c(head, list(N = N), if (find == "n") list(nfractional = 0), os, it, tail)
  } else {
    NN <- c(list(N = N), if (actual) list(N_a = sum(g)))
    na <- g[2] / g[1]
    tail <- if (!iterated) c(NN, list(N1 = g[1], N2 = g[2], nratio = nratio), if (abs(na - nratio) > 1e-12) list(nratio_a = na), P)
      else if (find == "n") c(list(N = N), rev(P), if (abs(na - nratio) > 1e-12) list(nratio_a = na), list(nratio = nratio, N2 = g[2], N1 = g[1]))
      else c(NN, list(N1 = g[1]), rev(P), if (abs(na - nratio) > 1e-12) list(nratio_a = na), list(nratio = nratio, N2 = g[2]))
    r <- c(head, list(delta = delta, nfractional = 0), os, it, tail)
  }
  dl <- if (find == "effect") list(direction = if (direction == "lower") "lower" else "upper")
  r <- c(r, switch(method,
    onevariance = c(list(type = "test", method = method), dl, list(scale = scale)),
    twovariances = c(list(scale = scale, type = "test", method = method), dl),
    pairedproportions = c(list(effect = effect, type = "test", method = method), dl),
    c(list(type = "test", method = method), dl)))
  # the printout
  unit <- if (sdsc) "standard deviation" else "variance"
  title <- switch(method, onevariance = if (sdsc) "one-sample standard-deviation test" else "one-sample variance test",
                  twovariances = if (sdsc) "two-sample standard-deviations test" else "two-sample variances test",
                  onecorrelation = "one-sample correlation test", twocorrelations = "two-sample correlations test",
                  pairedproportions = "two-sample paired-proportions test")
  what <- if (find == "power") "power" else if (find == "n") (if (two) "sample sizes" else "sample size") else
    switch(method, onevariance = paste("target", unit), twovariances = paste("experimental-group", unit),
           onecorrelation = "target correlation", twocorrelations = "experimental-group correlation",
           pairedproportions = "discordant proportions")
  testline <- switch(method, onevariance = "Chi-squared test", twovariances = "F test",
                     pairedproportions = "Large-sample McNemar's test", "Fisher's z test")
  sym <- switch(method, onevariance = c(substr(nm[1], 1, 1), nm), onecorrelation = c("r", nm), c(nm[2], nm))
  rel <- if (!onesided) "!=" else if (x2 >= v1) ">" else "<"
  h0 <- sprintf("H0: %s = %s  versus  Ha: %s %s %s", sym[1], sym[2], sym[1], rel, sym[2])
  # the target's side: always after one sample, after two (and paired proportions) only when two-sided
  if (find == "effect" && (!onesided || !(two || pp))) h0 <- paste0(h0, "; ", sym[3], if (direction == "lower") " < " else " > ", sym[2])
  # the head of the printout, for a table of several (stata_power_table)
  attr(r, "stata_head") <- c(paste0("Estimated ", what, " for a ", title), testline, h0)
  attr(r, "stata_find") <- find
  if (!show) return(invisible(r))
  num <- function(k, v, extra = "") sprintf("%13s = %9.4f%s", k, v, extra)
  int <- function(k, v) sprintf("%13s = %9s", k, formatC(v, format = "f", digits = 0, big.mark = ","))
  eff <- if (pp) paste0("  (", if (effect == "ratio") "ratio" else "difference", ")") else ""
  sizes_lines <- function(tot = N) {
    if (!two) return(int("N", tot))
    c(int("N", tot), if (g[1] == g[2] && is.null(n1)) int("N per group", g[1]) else c(int("N1", g[1]), int("N2", g[2])))
  }
  given_n <- if (!two) int("N", N) else if (actual) c(int("N", N), if (nratio != 1) num("N2/N1", nratio)) else
    c(sizes_lines(), if (g[1] != g[2]) num("N2/N1", g[2] / g[1]))
  given <- c(num("alpha", alpha), if (find != "power") num("power", target), if (find != "n") given_n,
             if (find != "effect") num("delta", delta, eff))
  vals <- c(if (pdm) num("p12 + p21", prdisc) else c(num(nm[1], v1), if (find != "effect") num(nm[2], x2)),
            if (two && find == "n" && nratio != 1) num("N2/N1", nratio))
  cat("\n")
  if (iterated) cat("Performing iteration ...\n\n")
  cat("Estimated ", what, " for a ", title, "\n", testline, "\n", h0, "\n\nStudy parameters:\n\n", sep = "")
  cat(c(given, vals), sep = "\n")
  if (actual) {
    cat("\nActual sample sizes:\n\n")
    cat(sizes_lines(sum(g)), sep = "\n")
    if (g[1] != g[2] && abs(g[2] / g[1] - nratio) > 1e-12) cat(num("N2/N1", g[2] / g[1]), "\n", sep = "")
  }
  if (find == "power") {
    cat("\nEstimated power:\n\n", num("power", pv), "\n", sep = "")
  } else if (find == "n") {
    cat("\nEstimated ", what, ":\n\n", sep = "")
    cat(sizes_lines(), sep = "\n")
    if (two && g[1] != g[2] && abs(g[2] / g[1] - nratio) > 1e-12) cat(num("N2/N1", g[2] / g[1]), "\n", sep = "")
  } else {
    cat("\nEstimated effect size and ", what, ":\n\n", sep = "")
    cat(num("delta", delta, eff), "\n", sep = "")
    cat(if (pp) c(num("p12", v1), num("p21", x2)) else num(nm[2], x2), sep = "\n")
  }
  cat("\n")
  invisible(r)
}

# power oneway (Stata 19): the F test of a group effect, noncentrality N * Var_m / Var_e on k - 1 and
# N - k df. The noncentral F is summed here (Poisson weights of regularized betas): R's own pf() with a
# noncentrality agrees with Stata's only to about 1e-9
stata_power_oneway <- function(means = NULL, ngroups = NULL, varmeans = NULL, varerror = 1, alpha = 0.05,
                               power = NULL, n = NULL, npergroup = NULL, ns = NULL, grweights = NULL, show = TRUE) {
  stop_r <- function(m) stop(m, call. = FALSE)
  k <- if (is.null(means)) ngroups else length(means)
  w <- if (!is.null(ns)) ns else if (!is.null(grweights)) grweights else rep(1, k)
  w <- w / sum(w)
  vm <- if (!is.null(means)) sum(w * (means - sum(w * means))^2) else varmeans
  find <- if (is.null(power) && (!is.null(n) || !is.null(npergroup) || !is.null(ns))) "power" else
    if (!is.null(vm)) "n" else "effect"
  target <- if (is.null(power)) 0.8 else power
  tails <- function(N, v) {
    d1 <- k - 1
    d2 <- N - k
    f <- stats::qf(1 - alpha, d1, d2)
    x <- d1 * f / (d1 * f + d2)
    h <- N * v / varerror / 2
    j <- 0:ceiling(h + 40 * sqrt(h) + 60)
    p <- stats::dpois(j, h)
    c(up = sum(p * stats::pbeta(x, d1 / 2 + j, d2 / 2, lower.tail = FALSE)),
      lo = sum(p * stats::pbeta(x, d1 / 2 + j, d2 / 2)))
  }
  pw <- function(N, v) tails(N, v)[["up"]]
  balanced <- is.null(ns) && is.null(grweights)
  actual <- FALSE
  iterated <- FALSE
  if (find == "n") {
    lo <- k + 1
    f <- function(N) pw(N, vm) - target
    Na <- if (f(lo) >= 0) lo else stats::uniroot(f, c(lo, lo + 10), extendInt = "upX", tol = 1e-13)$root
    g <- if (balanced) rep(ceiling(Na / k - 1e-9), k) else ceiling(Na * w - 1e-9)
    N <- sum(g)
    # Stata says it searches when it has the means or weights (not with varmeans() alone)
    iterated <- !is.null(means) || !is.null(grweights)
    pv <- target
    bv <- 1 - target
  } else {
    g <- if (!is.null(ns)) ns else if (!is.null(npergroup)) rep(npergroup, k) else rep(floor(n / k + 1e-9), k)
    N <- if (!is.null(n)) n else sum(g)
    actual <- !is.null(n) && sum(g) != n
    Na <- sum(g)
    if (find == "power") {
      t2 <- tails(Na, vm)
      pv <- t2[["up"]]
      bv <- 1 - pv
    } else {
      vm <- stats::uniroot(function(v) pw(Na, v) - target, c(1e-10, 1) * varerror, extendInt = "upX", tol = 1e-15)$root
      iterated <- TRUE
      pv <- target
      bv <- 1 - target
    }
  }
  if (min(g) < 1 || sum(g) <= k) stop_r("power oneway: too few observations for the F test")
  delta <- sqrt(vm / varerror)
  sz <- if (balanced) list(balanced = 1, N_per_group = g[1]) else
    c(if (!is.null(grweights)) stats::setNames(as.list(grweights), paste0("grwgt", seq_len(k))), list(balanced = 0),
      stats::setNames(as.list(g), paste0("N", seq_len(k))))
  r <- c(list(Var_m = vm, Var_e = varerror), sz, list(N_avg = sum(g) / k, N_g = k, N_a = Na),
         if (!is.null(means)) stats::setNames(as.list(means), paste0("m", seq_len(k))),
         list(power = pv, beta = bv, alpha = alpha, delta = delta),
         if (iterated) list(converged = 1, maxiter = 500, tolerance = 1e-12, ftolerance = 1e-12),
         list(onesided = 0, nfractional = 0, N = N, type = "test", method = "oneway"))
  if (!show) return(invisible(r))
  num <- function(k, v) sprintf("%13s = %9.4f", k, v)
  int <- function(k, v) sprintf("%13s = %9s", k, formatC(v, format = "f", digits = 0, big.mark = ","))
  sizes <- function(tot) c(int("N", tot), if (balanced) int("N per group", g[1]) else
    c(num("Average N", sum(g) / k), int(paste0("N", seq_len(k)), g)))
  what <- switch(find, power = "power", n = "sample size", effect = "between-group variance")
  cat("\n")
  if (iterated) cat("Performing iteration ...\n\n")
  cat("Estimated ", what, " for one-way ANOVA\nF test for group effect\nH0: delta = 0  versus  Ha: delta != 0\n\n",
      "Study parameters:\n\n", sep = "")
  given <- c(num("alpha", alpha), if (find != "power") num("power", target),
             if (find != "n") (if (actual) int("N", N) else sizes(N)), if (find != "effect") num("delta", delta),
             int("N_g", k), if (!is.null(means)) num(paste0("m", seq_len(k)), means),
             if (find != "effect") num("Var_m", vm), num("Var_e", varerror))
  cat(given, sep = "\n")
  if (actual) {
    cat("\nActual sample sizes:\n\n")
    cat(sizes(sum(g)), sep = "\n")
  }
  if (find == "power") cat("\nEstimated power:\n\n", num("power", pv), "\n", sep = "")
  else if (find == "n") {
    cat("\nEstimated sample sizes:\n\n")
    cat(sizes(N), sep = "\n")
  } else cat("\nEstimated effect size and between-group variance:\n\n", num("delta", delta), "\n", num("Var_m", vm), "\n", sep = "")
  cat("\n")
  invisible(r)
}


# power logrank and power cox (Stata 19): the number of events E for the log-rank test (Freedman's
# method, or Schoenfeld's) and the Wald test of a Cox coefficient (Hsieh and Lavori), the sample size
# from the events and the probability of an event. Stata's power is the one tail on the effect's side,
# with E = N * Pr_E as it is; r(E) and the sizes are rounded up
stata_power_surv <- function(method, s1 = NULL, s2 = NULL, hratio = NULL, b1 = NULL, sd = 0.5, r2 = NULL,
                             eventprob = 1, schoenfeld = FALSE, alpha = 0.05, power = NULL, n = NULL, n1 = NULL,
                             n2 = NULL, nratio = NULL, onesided = FALSE, direction = "lower", show = TRUE) {
  stop_r <- function(m) stop(m, call. = FALSE)
  lr <- method == "logrank"
  up <- direction == "upper"
  if (lr && !is.null(s1) && !is.null(s2)) hratio <- log(s2) / log(s1)
  if (!lr && !is.null(hratio)) b1 <- log(hratio)
  find <- if (lr && is.null(hratio) || !lr && is.null(b1)) "effect" else
    if (is.null(power) && (!is.null(n) || !is.null(n1))) "power" else "n"
  target <- if (is.null(power)) 0.8 else power
  if (is.null(nratio)) nratio <- if (!is.null(n1) && !is.null(n2)) n2 / n1 else 1
  side <- if (onesided) 1 else 2
  zc <- stats::qnorm(1 - alpha / side)
  zz <- zc + stats::qnorm(target)
  r2v <- if (is.null(r2)) 0 else r2
  # the probability of an event: no censoring without s1 (logrank), eventprob() (cox)
  s2_given <- s2
  pr_e <- function(hr, a, b) {
    if (!lr) return(eventprob)
    if (is.null(s1)) return(1)
    # in the ratio of the groups, as Stata 19 works it (its doubles: n1(100) n2(100) with 0.5 and 0.4 gives
    # 0.55000000000000004, so E is 111)
    R <- b / a
    1 - (s1 + R * (if (!is.null(s2_given)) s2_given else s1^hr)) / (1 + R)
  }
  # the standardized effect for E events at hazard ratio hr (cox: coefficient b)
  eff <- function(E, x, R) {
    if (!lr) return(sqrt(E * sd^2 * (1 - r2v)) * abs(x))
    if (schoenfeld) sqrt(E * R) * abs(log(x)) / (1 + R) else sqrt(E * R) * abs(1 - x) / (1 + R * x)
  }
  pw <- function(E, x, R) stats::pnorm(eff(E, x, R) - zc)
  actual <- FALSE
  if (find == "n") {
    x <- if (lr) hratio else b1
    R <- nratio
    Ef <- if (!lr) zz^2 / (sd^2 * (1 - r2v) * b1^2) else if (schoenfeld) zz^2 * (1 + R)^2 / (R * log(hratio)^2) else
      zz^2 * (1 + R * hratio)^2 / (R * (1 - hratio)^2)
    P <- pr_e(hratio, 1, R)
    Nf <- Ef / P
    g <- if (lr) c(ceiling(Nf / (1 + R) - 1e-9), ceiling(R * Nf / (1 + R) - 1e-9)) else c(ceiling(Nf - 1e-9), NA)
    N <- sum(g, na.rm = TRUE)
    # with no censoring every subject is an event: E is the rounded-up N (Stata 19)
    E <- if (P == 1) N else ceiling(Ef - 1e-9)
    pv <- target
    iterated <- FALSE
  } else {
    g <- if (!lr) c(n, NA) else if (!is.null(n1)) c(n1, n2) else {
      a <- n / (1 + nratio)
      if (abs(a - round(a)) < 1e-8) c(round(a), n - round(a)) else {
        actual <- TRUE
        c(floor(a + 1e-9), floor(n * nratio / (1 + nratio) + 1e-9))
      }
    }
    Ng <- sum(g, na.rm = TRUE)
    N <- if (lr && !is.null(n1)) Ng else n
    R <- if (lr) g[2] / g[1] else 1
    # the target is worked out from the total given (Stata 19: n(401) nratio(1.5), groups 160 and 240)
    Nt <- if (is.null(n)) Ng else n
    iterated <- FALSE
    if (find == "power") {
      x <- if (lr) hratio else b1
      P <- pr_e(hratio, g[1], g[2])
      pv <- pw(Ng * P, x, R)
    } else {
      pv <- target
      if (lr && !is.null(s1)) {
        f <- function(h) pw(Nt * pr_e(h, g[1], g[2]), h, R) - target
        e <- 1e-10
        hratio <- if (up) stats::uniroot(f, c(1 + e, 2), extendInt = "upX", tol = 1e-14)$root else
          stats::uniroot(f, c(e, 1 - e), tol = 1e-14)$root
        iterated <- TRUE
        x <- hratio
      } else {
        P <- pr_e(1, g[1], g[2])
        c0 <- zz / sqrt(Nt * P * (if (lr) R else 1))
        if (!lr) {
          b1 <- (if (up) 1 else -1) * zz / sqrt(Nt * P * sd^2 * (1 - r2v))
          x <- b1
        } else if (schoenfeld) {
          hratio <- exp((if (up) 1 else -1) * c0 * (1 + R))
          x <- hratio
        } else {
          hratio <- if (up) (1 + c0) / (1 - c0 * R) else (1 - c0) / (1 + c0 * R)
          x <- hratio
        }
      }
      P <- pr_e(hratio, g[1], g[2])
    }
    # rounded up as the double it is (Stata 19: 200 * 0.55 is 110.00000000000001, so 111)
    E <- ceiling(Ng * P)
  }
  delta <- if (!lr) b1 else if (schoenfeld) log(hratio) else hratio
  s2v <- if (lr && !is.null(s2_given)) s2_given else if (lr && !is.null(s1)) s1^hratio
  # r() in the order of Stata 19's return list for this form
  head <- list(power = pv, beta = 1 - pv, alpha = alpha, delta = delta)
  os <- list(onesided = as.numeric(onesided))
  it <- if (iterated) list(converged = 1, maxiter = 500, tolerance = 1e-12, ftolerance = 1e-12) else list()
  if (lr) {
    nra <- if (abs(g[2] / g[1] - nratio) > 1e-12) list(nratio_a = g[2] / g[1])
    NN <- c(list(N = N), if (actual) list(N_a = sum(g)))
    cens <- if (is.null(s1)) list(Pr_E = P, Pr_w = 0) else if (find == "n") list(s1 = s1, Pr_w = 0, Pr_E = P, s2 = s2v) else
      list(s1 = s1, s2 = s2v, Pr_w = 0, Pr_E = P)
    tail <- if (iterated) c(NN, list(N1 = g[1], Pr_w = 0, Pr_E = P, s2 = s2v, s1 = s1, E = E, hratio = hratio, nratio = R, N2 = g[2]))
      else c(NN, list(N1 = g[1], N2 = g[2], nratio = R), if (find == "n") nra, list(hratio = hratio, E = E), cens)
    r <- c(head, list(nfractional = 0), os, it, tail)
  } else {
    r <- c(head, list(N = N), if (find == "n") list(nfractional = 0), os, list(E = E),
           if (!is.null(hratio) && find != "effect") list(hratio = hratio), list(b1 = b1, sd = sd, R2 = r2v, Pr_E = P, Pr_w = 0))
  }
  dl <- if (find == "effect") list(direction = if (up) "upper" else "lower")
  r <- c(r, list(type = "test", method = method), dl,
         if (lr) list(test = if (schoenfeld) "Schoenfeld" else "Freedman", effect = if (schoenfeld) "lnhratio" else "hratio")
         else list(effect = "coefficient"))
  # the printout
  title <- if (lr) "two-sample comparison of survivor functions" else "Cox PH regression"
  what <- if (find == "power") "power" else if (find == "n") (if (lr) "sample sizes" else "sample size") else
    if (lr) (if (schoenfeld) "log hazard-ratio" else "hazard ratio") else "target coefficient"
  testline <- if (!lr) "Wald test" else if (schoenfeld) "Log-rank test, Schoenfeld method" else "Log-rank test, Freedman method"
  sym <- if (!lr) c("beta1", "0") else if (schoenfeld) c("ln(HR)", "0") else c("HR", "1")
  above <- if (find == "effect") up else if (lr) hratio > 1 else b1 > 0
  rel <- if (!onesided) "!=" else if (above) ">" else "<"
  h0 <- sprintf("H0: %s = %s  versus  Ha: %s %s %s", sym[1], sym[2], sym[1], rel, sym[2])
  # the target's side, only when two-sided (Stata 19)
  if (find == "effect" && !onesided) h0 <- paste0(h0, "; ", sym[1], if (up) " > " else " < ", sym[2])
  # after cox with b1 given or worked out, Stata keeps r(hratio) out of return list but readable
  if (!lr && is.null(r$hratio)) attr(r, "stata_hidden") <- list(hratio = exp(b1))
  attr(r, "stata_head") <- c(paste0("Estimated ", what, " for ", if (lr) "" else "", title), testline, h0)
  attr(r, "stata_find") <- find
  if (!show) return(invisible(r))
  num <- function(k, v, extra = "") sprintf("%13s = %9.4f%s", k, v, extra)
  int <- function(k, v) sprintf("%13s = %9s", k, formatC(v, format = "f", digits = 0, big.mark = ","))
  ex <- if (!lr) "  (coefficient)" else if (schoenfeld) "  (log hazard-ratio)" else "  (hazard ratio)"
  sizes_lines <- function(tot) {
    if (!lr) return(int("N", tot))
    c(int("N", tot), if (g[1] == g[2] && is.null(n1)) int("N per group", g[1]) else c(int("N1", g[1]), int("N2", g[2])))
  }
  given_n <- if (!lr) int("N", N) else if (actual) c(int("N", N), if (nratio != 1) num("N2/N1", nratio)) else
    c(sizes_lines(N), if (g[1] != g[2]) num("N2/N1", g[2] / g[1]))
  study <- c(num("alpha", alpha), if (find != "power") num("power", target), if (find != "n") given_n,
             if (find != "effect") c(num("delta", delta, ex), if (lr || !is.null(hratio)) num("hratio", hratio) else num("b1", b1)),
             if (!lr) c(num("sd", sd), if (!is.null(r2)) num("R2", r2)),
             if (lr && find == "n" && nratio != 1) num("N2/N1", nratio))
  cens_lines <- c(if (lr && !is.null(s1)) c(num("s1", s1), num("s2", s2v)), num("Pr_E", P))
  cat("\n")
  if (iterated) cat("Performing iteration ...\n\n")
  cat("Estimated ", what, " for ", title, "\n", testline, "\n", h0, "\n\nStudy parameters:\n\n", sep = "")
  cat(study, sep = "\n")
  if (actual) {
    cat("\nActual sample sizes:\n\n")
    cat(sizes_lines(sum(g)), sep = "\n")
  }
  if (find == "n") {
    cat("\nCensoring:\n\n")
    cat(cens_lines, sep = "\n")
    cat("\nEstimated number of events and ", what, ":\n\n", int("E", E), "\n", sep = "")
    cat(sizes_lines(N), sep = "\n")
    if (lr && g[1] != g[2] && abs(g[2] / g[1] - nratio) > 1e-12) cat(num("N2/N1", g[2] / g[1]), "\n", sep = "")
  } else {
    cat("\nNumber of events and censoring:\n\n", int("E", E), "\n", sep = "")
    cat(cens_lines, sep = "\n")
    if (find == "power") cat("\nEstimated power:\n\n", num("power", pv), "\n", sep = "") else {
      cat("\nEstimated effect size and ", what, ":\n\n", num("delta", delta, ex), "\n", sep = "")
      cat(if (!lr) num("b1", b1) else if (schoenfeld) num("ln(hratio)", log(hratio)) else num("hratio", hratio), "
", sep = "")
    }
  }
  cat("\n")
  invisible(r)
}

# power oneslope, rsquared and pcorr (Stata 19): the t test of a regression slope (df N - 2, noncentrality
# sqrt(N) * delta with delta = (ba - b0) * sdx / sderror), and the F test of R2 (all coefficients, a subset
# of them) or a partial correlation on ntested and N - ncontrol - ntested - 1 df with noncentrality
# N * f2 (summed to full precision, as oneway's)
stata_power_reg <- function(method, v1 = NULL, v2 = NULL, alpha = 0.05, power = NULL, n = NULL,
                            onesided = FALSE, direction = "upper", sdx = 1, sderror = NULL, sdy = NULL,
                            ncontrol = 0, ntested = 1, show = TRUE) {
  stop_r <- function(m) stop(m, call. = FALSE)
  slope <- method == "oneslope"
  # rsquared with two values (or one, ncontrol() and the R2 searched): a subset of the coefficients
  subset <- method == "rsquared" && (!is.null(v2) || ncontrol > 0)
  given <- if (slope || subset) v2 else v1
  find <- if (is.null(given)) "effect" else if (is.null(power) && !is.null(n)) "power" else "n"
  target <- if (is.null(power)) 0.8 else power
  up <- direction != "lower"
  # the F tests' noncentral F, summed (R's pf() with ncp agrees with Stata's only to about 1e-9)
  ncf <- function(N, f2) {
    d2 <- N - ncontrol - ntested - 1
    f <- stats::qf(1 - alpha, ntested, d2)
    x <- ntested * f / (ntested * f + d2)
    h <- N * f2 / 2
    j <- 0:ceiling(h + 40 * sqrt(h) + 60)
    sum(stats::dpois(j, h) * stats::pbeta(x, ntested / 2 + j, d2 / 2, lower.tail = FALSE))
  }
  side <- if (onesided) 1 else 2
  tpow <- function(N, d) {
    l <- sqrt(N) * d
    df <- N - 2
    if (abs(l) > 1000) return(NA_real_)
    cut <- stats::qt(1 - alpha / side, df)
    up_t <- 1 - suppressWarnings(stats::pt(cut, df, l))
    lo_t <- suppressWarnings(stats::pt(-cut, df, l))
    min(1, if (!onesided) up_t + lo_t else if (d >= 0) up_t else lo_t)
  }
  # the effect size: oneslope's delta, the F tests' f2
  b0 <- if (slope) v1 else 0
  sde <- function(ba) if (!is.null(sdy)) sqrt(sdy^2 - ba^2 * sdx^2) else if (is.null(sderror)) 1 else sderror
  eff <- function(x) if (slope) (x - b0) * sdx / sde(x) else if (subset) (x - v1) / (1 - x) else x / (1 - x)
  pw <- function(N, d) if (slope) tpow(N, d) else ncf(N, d)
  lo <- if (slope) 3 else ncontrol + ntested + 2
  if (find == "power") {
    if (n < lo) stop_r(sprintf("power %s: n() is too small for the test", method))
    x <- given
    delta <- eff(x)
    pv <- pw(n, delta)
    N <- n
  } else if (find == "n") {
    x <- given
    delta <- eff(x)
    f <- function(N) pw(N, delta) - target
    a <- if (f(lo) >= 0) lo else stats::uniroot(f, c(lo, lo + 10), extendInt = "upX", tol = 1e-12)$root
    N <- ceiling(a - 1e-9)
    pv <- target
  } else {
    if (n < lo) stop_r(sprintf("power %s: n() is too small for the test", method))
    N <- n
    f <- function(d) pw(N, d) - target
    delta <- if (slope && !up) stats::uniroot(f, c(-1, -1e-10), extendInt = "downX", tol = 1e-14)$root else
      stats::uniroot(f, c(1e-10, 1), extendInt = "upX", tol = 1e-14)$root
    # back from the effect size to the slope or the R2
    if (slope && !is.null(sdy) && b0 != 0) stop_r("power oneslope: the target slope with sdy() and b0 other than 0 is not translated yet")
    x <- if (slope) {
      if (is.null(sdy)) b0 + delta * sde(0) / sdx else b0 + sign(delta) * sqrt(delta^2 * sdy^2 / (sdx^2 * (1 + delta^2)))
    } else if (subset) (delta + v1) / (1 + delta) else delta / (1 + delta)
    pv <- target
  }
  iterated <- find != "power"
  it <- if (iterated) list(converged = 1, maxiter = 500, tolerance = 1e-12, ftolerance = 1e-12) else list()
  head <- c(list(power = pv, beta = 1 - pv, alpha = alpha, N = N), if (find == "n") list(nfractional = 0))
  if (slope) {
    se <- sde(x)
    sy <- sqrt(x^2 * sdx^2 + se^2)
    co <- x * sdx / sy
    tail <- switch(find,
      n = list(b0 = b0, delta = delta, corr = co, sdy = sy, sderror = se, sdx = sdx, diff = x - b0, ba = x),
      power = list(b0 = b0, ba = x, diff = x - b0, sdx = sdx, sderror = se, sdy = sy, corr = co, delta = delta),
      effect = list(b0 = b0, ba = x, delta = delta, corr = co, sdy = sy, sderror = se, sdx = sdx, diff = x - b0))
    r <- c(head, list(onesided = as.numeric(onesided)), it, tail)
  } else if (method == "pcorr") {
    r <- c(head, it, list(ncontrol = ncontrol, rho2_p = x, ntested = ntested, delta = delta))
  } else if (subset) {
    tail <- switch(find,
      n = list(ncontrol = ncontrol, R2_R = v1, R2_F = x, delta = delta, ntested = ntested, R2_diff = x - v1),
      power = list(ncontrol = ncontrol, R2_R = v1, R2_F = x, R2_diff = x - v1, ntested = ntested, delta = delta),
      effect = list(ncontrol = ncontrol, R2_R = v1, R2_F = x, R2_diff = x - v1, delta = delta, ntested = ntested))
    r <- c(head, it, tail)
  } else {
    r <- c(head, it, list(R2_T = x, ntested = ntested, delta = delta))
  }
  r <- c(r, list(type = "test", method = method), if (slope && find == "effect") list(direction = if (up) "upper" else "lower"))
  title <- if (slope) "a linear regression slope test" else "multiple linear regression"
  what <- if (find == "power") "power" else if (find == "n") "sample size" else
    if (slope) "target slope" else if (method == "pcorr") "squared partial correlation" else "R-squared"
  testline <- if (slope) "t test" else if (method == "pcorr") "F test for partial correlation" else
    if (subset) "F test for R2 testing subset of coefficients" else "F test for R2 testing all coefficients"
  # oneslope one-sided says "b > b0" whatever the slope's sign (Stata 19)
  h0 <- if (slope) paste0("H0: b = b0  versus  Ha: b ", if (onesided) ">" else "!=", " b0",
                          if (find == "effect") paste0("; ba ", if (up) ">" else "<", " b0") else "")
    else if (method == "pcorr") "H0: rho2_p = 0  versus  Ha: rho2_p != 0"
    else if (subset) "H0: R2_F = R2_R  versus  Ha: R2_F != R2_R" else "H0: R2_T = 0  versus  Ha: R2_T != 0"
  attr(r, "stata_head") <- c(paste0("Estimated ", what, " for ", title), testline, h0)
  attr(r, "stata_find") <- find
  if (!show) return(invisible(r))
  num <- function(k, v) if (is.na(v)) sprintf("%13s = %9s", k, ".") else sprintf("%13s = %9.4f", k, v)
  int <- function(k, v) sprintf("%13s = %9s", k, formatC(v, format = "f", digits = 0, big.mark = ","))
  study <- c(num("alpha", alpha), if (find != "power") num("power", target), if (find != "n") int("N", N),
             if (find != "effect") num("delta", delta))
  study <- c(study, if (slope) c(num("b0", b0), if (find != "effect") num("ba", x), num("sdx", sdx),
                                 num("sderror", if (find == "effect" && !is.null(sdy)) sde(x) else sde(x)),
                                 if (!is.null(sdy)) num("sdy", sdy))
    else if (method == "pcorr") c(if (find != "effect") num("rho2_p", x), int("ncontrol", ncontrol), int("ntested", ntested))
    else if (subset) c(num("R2_R", v1), if (find != "effect") c(num("R2_F", x), num("R2_diff", x - v1)),
                       int("ncontrol", ncontrol), int("ntested", ntested))
    else c(if (find != "effect") num("R2_T", x), int("ntested", ntested)))
  cat("\n")
  if (iterated) cat("Performing iteration ...\n\n")
  cat(attr(r, "stata_head"), sep = "\n")
  cat("\nStudy parameters:\n\n")
  cat(study, sep = "\n")
  if (find == "power") cat("\nEstimated power:\n\n", num("power", pv), "\n", sep = "")
  else if (find == "n") cat("\nEstimated sample size:\n\n", int("N", N), "\n", sep = "")
  else {
    cat("\nEstimated effect size and ", what, ":\n\n", num("delta", delta), "\n", sep = "")
    cat(if (slope) num("ba", x) else if (method == "pcorr") num("rho2_p", x) else if (subset) c(num("R2_diff", x - v1), num("R2_F", x))
        else num("R2_T", x), sep = "\n")
  }
  cat("\n")
  invisible(r)
}

# power exponential (Stata 19): Lachin's test of two exponential hazards, the hazard difference (or the
# log hazard-ratio with loghazard), conditional (H0's variance at the groups' mean hazard) or
# unconditional (the alternative's); with studytime(T) the probability of an event is 1 - exp(-h T)
stata_power_exp <- function(h1, h2 = NULL, hratio = NULL, loghazard = FALSE, unconditional = FALSE,
                            studytime = NULL, alpha = 0.05, power = NULL, n = NULL, n1 = NULL, n2 = NULL,
                            nratio = NULL, onesided = FALSE, show = TRUE) {
  stop_r <- function(m) stop(m, call. = FALSE)
  given_hr <- !is.null(hratio)
  if (is.null(h2)) h2 <- h1 * hratio
  find <- if (is.null(power) && (!is.null(n) || !is.null(n1))) "power" else "n"
  actual <- FALSE
  target <- if (is.null(power)) 0.8 else power
  if (is.null(nratio)) nratio <- if (!is.null(n1)) n2 / n1 else 1
  side <- if (onesided) 1 else 2
  zc <- stats::qnorm(1 - alpha / side)
  pe <- function(h) if (is.null(studytime)) 1 else 1 - exp(-h * studytime)
  phi <- function(h) if (loghazard) 1 / pe(h) else h^2 / pe(h)
  D <- if (loghazard) log(h2 / h1) else h2 - h1
  sds <- function(R) {
    Q1 <- 1 / (1 + R)
    Q2 <- R / (1 + R)
    s1 <- sqrt(phi(h1) / Q1 + phi(h2) / Q2)
    s0 <- if (unconditional) s1 else sqrt(phi(Q1 * h1 + Q2 * h2) * (1 / Q1 + 1 / Q2))
    c(s0, s1)
  }
  pw <- function(N, R) {
    s <- sds(R)
    stats::pnorm((sqrt(N) * abs(D) - zc * s[1]) / s[2])
  }
  if (find == "n") {
    s <- sds(nratio)
    Nf <- ((zc * s[1] + stats::qnorm(target) * s[2]) / D)^2
    g <- c(ceiling(Nf / (1 + nratio) - 1e-9), ceiling(nratio * Nf / (1 + nratio) - 1e-9))
    pv <- target
  } else {
    g <- if (!is.null(n1)) c(n1, n2) else {
      a <- n / (1 + nratio)
      if (abs(a - round(a)) < 1e-8 && abs(nratio * round(a) - (n - round(a))) < 1e-8) c(round(a), n - round(a)) else {
        actual <- TRUE
        c(floor(a + 1e-9), floor(n * nratio / (1 + nratio) + 1e-9))
      }
    }
    pv <- pw(sum(g), g[2] / g[1])
  }
  N <- if (actual) n else sum(g)
  R <- g[2] / g[1]
  # the expected events (each group's rounded, the total their sum) and the probabilities of an event:
  # under H0 at the groups' mean hazard (unconditional: the first group's), then each group's
  # (the sample size works H0 out at the ratio asked for, the power at the groups' own)
  Rh <- if (find == "n") nratio else R
  h0 <- if (unconditional) h1 else (h1 + Rh * h2) / (1 + Rh)
  e0 <- floor(g * pe(h0) + 0.5)
  ea <- floor(g * c(pe(h1), pe(h2)) + 0.5)
  m <- function(v) matrix(v, 1, dimnames = list("r1", paste0("c", seq_along(v))))
  r <- c(list(power = pv, beta = 1 - pv, alpha = alpha, delta = D, nfractional = 0, onesided = as.numeric(onesided),
              N = N),
         if (actual) list(N_a = sum(g)),
         list(N1 = g[1], N2 = g[2], nratio = nratio),
         if (abs(R - nratio) > 1e-12) list(nratio_a = R),
         list(hratio = h2 / h1, lnhratio = log(h2 / h1), h1 = h1, h2 = h2, hdiff = h2 - h1),
         if (!is.null(studytime)) list(studytime = studytime, fperiod = studytime),
         list(unconditional = as.numeric(unconditional),
              test = if (loghazard) "log-hazard difference" else "hazard difference",
              effect = if (loghazard) "lnhratio" else "difference", type = "test", method = "exponential",
              E0_vec = m(c(sum(e0), e0)), Ea_vec = m(c(sum(ea), ea)), Pr_vec = m(c(pe(h0), pe(h0), pe(h1), pe(h2)))))
  testline <- paste0("Exponential test, ", if (loghazard) "log hazard-ratio" else "hazard difference", ", ",
                     if (unconditional) "unconditional" else "conditional")
  rel <- if (!onesided) "!=" else if (D >= 0) ">" else "<"
  h0line <- if (loghazard) sprintf("H0: ln(HR) = 0  versus  Ha: ln(HR) %s 0", rel) else sprintf("H0: h2 = h1  versus  Ha: h2 %s h1", rel)
  what <- if (find == "n") "sample sizes" else "power"
  attr(r, "stata_head") <- c(paste0("Estimated ", what, " for two-sample comparison of survivor functions"), testline, h0line)
  attr(r, "stata_find") <- find
  if (!show) return(invisible(r))
  num <- function(k, v, extra = "") sprintf("%13s = %9.4f%s", k, v, extra)
  int <- function(k, v) sprintf("%13s = %9s", k, formatC(v, format = "f", digits = 0, big.mark = ","))
  sizes_lines <- function(tot) c(int("N", tot), if (g[1] == g[2] && is.null(n1)) int("N per group", g[1]) else
    c(int("N1", g[1]), int("N2", g[2])))
  study <- c(num("alpha", alpha), if (find == "n") num("power", target),
             if (find == "power") (if (actual) c(int("N", N), if (nratio != 1) num("N2/N1", nratio)) else
               c(sizes_lines(N), if (g[1] != g[2]) num("N2/N1", R))),
             num("delta", D, if (loghazard) "  (log hazard-ratio)" else "  (hazard difference)"),
             if (find == "n" && nratio != 1) num("N2/N1", nratio))
  cat("note: input parameters are hazard rates.\n\n")
  cat(attr(r, "stata_head"), sep = "\n")
  cat("\nStudy parameters:\n\n")
  cat(study, sep = "\n")
  if (actual) {
    cat("\nActual sample sizes:\n\n")
    cat(sizes_lines(sum(g)), sep = "\n")
    if (g[1] != g[2] && abs(R - nratio) > 1e-12) cat(num("N2/N1", R), "\n", sep = "")
  }
  if (!is.null(studytime)) {
    cat("\nAccrual and follow-up information:\n\n")
    cat(num("duration", studytime), num("follow-up", studytime), sep = "\n")
  }
  cat("\nSurvival information:\n\n")
  cat(num("h1", h1), num("h2", h2), if (given_hr) num("hratio", hratio), sep = "\n")
  if (find == "n") {
    cat("\nEstimated sample sizes:\n\n")
    cat(sizes_lines(N), sep = "\n")
    if (g[1] != g[2] && abs(R - nratio) > 1e-12) cat(num("N2/N1", R), "\n", sep = "")
  } else cat("\nEstimated power:\n\n", num("power", pv), "\n", sep = "")
  cat("\n")
  invisible(r)
}
# power with lists of values (Stata 19): every combination, nested values > alpha > power > n > n1 > n2 >
# nratio > the method's own numbers (the last varies fastest), each worked out as one power line and
# printed as Stata's table; r() is the last row's with the table's own (columns, labels, widths,
# formats and r(pss_table))
stata_power_table <- function(method, lists, fixed = list(), notable = FALSE, show = TRUE) {
  stop_r <- function(m) stop(m, call. = FALSE)
  order <- c("v1", "v2", "alpha", "power", "n", "n1", "n2", "nratio", "sd", "sd1", "sd2", "sddiff", "corr",
             "prdiscordant")
  lists <- lists[intersect(order, names(lists))]
  if (prod(lengths(lists)) > 10000) stop_r("power: the lists make more than 10,000 rows")
  rows <- if (length(lists)) rev(expand.grid(rev(lists), KEEP.OUT.ATTRS = FALSE)) else data.frame(row.names = 1L)
  rs <- lapply(seq_len(nrow(rows)), function(i)
    do.call(stata_power, c(list(method), fixed, as.list(rows[i, , drop = FALSE]), list(show = FALSE, table_row = TRUE))))
  r <- rs[[length(rs)]]
  two <- method %in% c("twomeans", "twoproportions", "twovariances", "twocorrelations")
  sdsc <- identical(r$scale, "standard deviation")
  params <- switch(method,
    onemean = c("m0", "ma", "sd"),
    twomeans = if (is.null(r$sd)) c("m1", "m2", "sd1", "sd2") else c("m1", "m2", "sd"),
    pairedmeans = c("d0", "da", "ma1", "ma2", "sd_d", if (!is.null(r$corr)) c("sd", "corr")),
    oneproportion = c("p0", "pa"), twoproportions = c("p1", "p2"),
    onevariance = if (sdsc) c("s0", "sa") else c("v0", "va"),
    twovariances = if (sdsc) c("s1", "s2") else c("v1", "v2"),
    onecorrelation = c("r0", "ra"), twocorrelations = c("r1", "r2"),
    pairedproportions = c("p12", "p21", if (!is.null(r$prdiscordant)) "prdiscordant"))
  cols <- c("alpha", "power", "N", if (two) c("N1", "N2"),
            if ("nratio" %in% names(lists) || !is.null(fixed$nratio)) "nratio", "delta", params)
  labs <- ifelse(cols == "nratio", "N2/N1", cols)
  widths <- pmax(8L, nchar(labs) + 1L)
  fmts <- ifelse(cols %in% c("N", "N1", "N2"), "%7.0gc", "%7.4g")
  val <- function(x, k) if (is.null(x[[k]])) NA_real_ else as.numeric(x[[k]])
  tab <- matrix(unlist(lapply(rs, function(x) vapply(cols, function(k) val(x, k), 0))), ncol = length(cols),
                byrow = TRUE, dimnames = list(paste0("r", seq_along(rs)), cols))
  # the table's own scalars (where Stata's return list puts them is not followed: RCraft refuses it)
  sc <- which(vapply(r, function(v) is.numeric(v) && !is.matrix(v), TRUE))
  r <- append(r, list(separator = 0, divider = 0), after = max(sc))
  q <- function(s) paste0("\"", s, "\"", collapse = " ")
  at <- max(match(c("method", "direction"), names(r), nomatch = 0))
  r <- append(r, list(columns = paste(cols, collapse = " "), labels = q(labs), widths = paste(widths, collapse = " "),
                      formats = q(fmts)), after = at)
  r$pss_table <- tab
  if (!show) return(invisible(r))
  cat("\n")
  if (any(vapply(rs, function(x) !is.null(x$converged), TRUE))) cat("Performing iteration ...\n\n")
  cat(paste0(attr(rs[[1]], "stata_head"), "\n"), sep = "")
  if (!notable) {
    rule <- paste0("  +", strrep("-", sum(widths) + 1), "+")
    line <- function(cells) paste0("  |", paste0(" ", cells, collapse = ""), " |")
    cell <- function(x, f, w) sprintf("%*s", w - 1L, stata_fmt(x, f, trim = TRUE))
    cat("\n", rule, "\n", line(sprintf("%*s", widths - 1L, labs)), "\n",
        paste0("  |", strrep("-", sum(widths) + 1), "|"), "\n", sep = "")
    for (i in seq_len(nrow(tab))) cat(line(mapply(cell, tab[i, ], fmts, widths)), "\n", sep = "")
    cat(rule, "\n", sep = "")
    if (any(vapply(rs, function(x) !is.null(x$N_a), TRUE))) cat("   Note: N1 and N2 have been rounded down.\n")
    if (two && method == "twomeans" && anyNA(tab[, "power"])) cat("   Warning: Some of the computations failed; execute each computation separately to see the specific error.\n")
  }
  cat("\n")
  invisible(r)
}
"##;

/// The r() power leaves (Stata 19), but its search's own r(init), r(iter),
/// r(deltax) and r(function).
const R_POWER: &[&str] = &[
    "power",
    "beta",
    "alpha",
    "N",
    "N_a",
    "nfractional",
    "onesided",
    "converged",
    "maxiter",
    "tolerance",
    "ftolerance",
    "delta",
    "diff",
    "m0",
    "ma",
    "sd",
    "knownsd",
    "m1",
    "m2",
    "sd1",
    "sd2",
    "knownsds",
    "unequal",
    "N1",
    "N2",
    "nratio",
    "nratio_a",
    "d0",
    "da",
    "ma1",
    "ma2",
    "sd_d",
    "corr",
    "p0",
    "pa",
    "p1",
    "p2",
    "continuity",
    "v0",
    "va",
    "s0",
    "sa",
    "v1",
    "v2",
    "s1",
    "s2",
    "ratio",
    "r0",
    "ra",
    "r1",
    "r2",
    "p12",
    "p21",
    "prdiscordant",
    "type",
    "method",
    "direction",
    "test",
    "effect",
    "scale",
    "separator",
    "divider",
    "columns",
    "labels",
    "widths",
    "formats",
    "pss_table",
    "hratio",
    "E",
    "Pr_E",
    "Pr_w",
    "b1",
    "R2",
    "b0",
    "ba",
    "sdx",
    "sderror",
    "sdy",
    "R2_T",
    "R2_R",
    "R2_F",
    "R2_diff",
    "ntested",
    "ncontrol",
    "rho2_p",
    "hdiff",
    "lnhratio",
    "h1",
    "h2",
    "unconditional",
    "studytime",
    "fperiod",
    "E0_vec",
    "Ea_vec",
    "Pr_vec",
];

/// The methods from onevariance on: Stata's sd flag (not sd()), and
/// values that are not means.
const LATER: &[&str] = &[
    "onevariance",
    "twovariances",
    "onecorrelation",
    "twocorrelations",
    "pairedproportions",
];

/// The methods translated, and the options each takes beyond alpha(),
/// power(), n(), onesided and direction(), at their full names (no
/// abbreviation was checked).
const METHODS: &[(&str, &[(&str, usize)])] = &[
    ("onemean", &[("sd", 2), ("knownsd", 7)]),
    (
        "twomeans",
        &[
            ("sd", 2),
            ("sd1", 3),
            ("sd2", 3),
            ("knownsds", 8),
            ("n1", 2),
            ("n2", 2),
            ("nratio", 6),
        ],
    ),
    (
        "pairedmeans",
        &[("sddiff", 6), ("sd", 2), ("corr", 4), ("knownsd", 7)],
    ),
    ("oneproportion", &[("test", 4)]),
    (
        "twoproportions",
        &[
            ("n1", 2),
            ("n2", 2),
            ("nratio", 6),
            ("continuity", 10),
            ("effect", 6),
            ("test", 4),
        ],
    ),
    ("onevariance", &[("sd", 2)]),
    (
        "twovariances",
        &[("sd", 2), ("n1", 2), ("n2", 2), ("nratio", 6)],
    ),
    ("onecorrelation", &[]),
    ("twocorrelations", &[("n1", 2), ("n2", 2), ("nratio", 6)]),
    ("pairedproportions", &[("effect", 6), ("prdiscordant", 12)]),
];

/// The r() owner after a power table: `return list` is refused after it.
pub(super) const POWER_TABLE: &str = "power, table";
/// The r() owner after power oneway: `return list` is refused after it.
pub(super) const POWER_ONEWAY: &str = "power oneway";

/// The r() power oneway leaves (Stata 19), for up to 20 groups.
const R_ONEWAY: &[&str] = &[
    "Var_m",
    "Var_e",
    "balanced",
    "N_per_group",
    "N_avg",
    "N_g",
    "N_a",
    "power",
    "beta",
    "alpha",
    "delta",
    "converged",
    "maxiter",
    "tolerance",
    "ftolerance",
    "onesided",
    "nfractional",
    "N",
    "type",
    "method",
    "m1",
    "m2",
    "m3",
    "m4",
    "m5",
    "m6",
    "m7",
    "m8",
    "m9",
    "m10",
    "m11",
    "m12",
    "m13",
    "m14",
    "m15",
    "m16",
    "m17",
    "m18",
    "m19",
    "m20",
    "N1",
    "N2",
    "N3",
    "N4",
    "N5",
    "N6",
    "N7",
    "N8",
    "N9",
    "N10",
    "N11",
    "N12",
    "N13",
    "N14",
    "N15",
    "N16",
    "N17",
    "N18",
    "N19",
    "N20",
    "grwgt1",
    "grwgt2",
    "grwgt3",
    "grwgt4",
    "grwgt5",
    "grwgt6",
    "grwgt7",
    "grwgt8",
    "grwgt9",
    "grwgt10",
    "grwgt11",
    "grwgt12",
    "grwgt13",
    "grwgt14",
    "grwgt15",
    "grwgt16",
    "grwgt17",
    "grwgt18",
    "grwgt19",
    "grwgt20",
];

/// A Stata numlist as power reads one: numbers, `a(d)b` and `a/b`.
fn numlist(what: &str, text: &str) -> Result<Vec<f64>, Untranslated> {
    let bad = || no(format!("power: {what} {} needs numbers", text.trim()));
    let mut out = Vec::new();
    for tok in text.split_whitespace() {
        let mut range = |a: f64, d: f64, b: f64| -> Result<(), Untranslated> {
            let steps = (b - a) / d;
            if d == 0.0 || !steps.is_finite() || steps < -1e-9 || steps > 10_000.0 {
                return Err(bad());
            }
            let n = (steps + 1e-9).floor() as usize;
            out.extend((0..=n).map(|i| a + d * i as f64));
            Ok(())
        };
        let num = |s: &str| s.trim().parse::<f64>().ok().filter(|v| v.is_finite());
        if let Some((a, rest)) = tok.split_once('(') {
            let (d, b) = rest.split_once(')').ok_or_else(bad)?;
            let (Some(a), Some(d), Some(b)) = (num(a), num(d), num(b)) else {
                return Err(bad());
            };
            range(a, d, b)?;
        } else if let Some((a, b)) = tok.split_once('/') {
            let (Some(a), Some(b)) = (num(a), num(b)) else {
                return Err(bad());
            };
            range(a, if b >= a { 1.0 } else { -1.0 }, b)?;
        } else {
            out.push(num(tok).ok_or_else(bad)?);
        }
    }
    if out.is_empty() || out.len() > 10_000 {
        return Err(bad());
    }
    Ok(out)
}

/// The values after the method: numbers or lists in parentheses.
fn value_words(rest: &str) -> Result<Vec<String>, Untranslated> {
    let mut words = Vec::new();
    let mut cur = String::new();
    let mut depth = 0usize;
    for ch in rest.chars() {
        match ch {
            '(' => {
                depth += 1;
                cur.push(ch);
            }
            ')' => {
                depth = depth
                    .checked_sub(1)
                    .ok_or_else(|| no(format!("power: unbalanced parentheses in {rest}")))?;
                cur.push(ch);
            }
            c if c.is_whitespace() && depth == 0 => {
                if !cur.is_empty() {
                    words.push(std::mem::take(&mut cur));
                }
            }
            c => cur.push(c),
        }
    }
    if depth != 0 {
        return Err(no(format!("power: unbalanced parentheses in {rest}")));
    }
    if !cur.is_empty() {
        words.push(cur);
    }
    Ok(words)
}

fn r_vector(xs: &[f64]) -> String {
    if let [x] = xs {
        r_number(*x)
    } else {
        format!(
            "c({})",
            xs.iter()
                .map(|x| r_number(*x))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

/// What an option's number must be.
#[derive(Clone, Copy)]
enum Range {
    /// Between 0 and 1.
    Prob,
    /// Above 0.
    Positive,
    /// A whole number, 1 or more.
    Whole,
    /// Between -1 and 1.
    Corr,
}

impl Range {
    fn holds(self, x: f64) -> bool {
        match self {
            Range::Prob => x > 0.0 && x < 1.0,
            Range::Positive => x > 0.0,
            Range::Whole => x >= 1.0 && x.fract() == 0.0,
            Range::Corr => x > -1.0 && x < 1.0,
        }
    }

    fn why(self) -> &'static str {
        match self {
            Range::Prob => "must be between 0 and 1",
            Range::Positive => "must be above 0",
            Range::Whole => "must be a whole number above 0",
            Range::Corr => "must be between -1 and 1",
        }
    }
}

/// The number options, with their shortest forms and ranges.
const NUMBERS: &[(&str, usize, Range)] = &[
    ("alpha", 5, Range::Prob),
    ("power", 5, Range::Prob),
    ("n", 1, Range::Whole),
    ("n1", 2, Range::Whole),
    ("n2", 2, Range::Whole),
    ("nratio", 6, Range::Positive),
    ("sd", 2, Range::Positive),
    ("sd1", 3, Range::Positive),
    ("sd2", 3, Range::Positive),
    ("sddiff", 6, Range::Positive),
    ("corr", 4, Range::Corr),
    ("prdiscordant", 12, Range::Prob),
];

impl Session {
    pub(super) fn power(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "power")?;
        if c.parts.weight.is_some() {
            return Err(no("power takes no weights (Stata: r(101))"));
        }
        let main = c.parts.main.trim();
        let (method, rest) = first_word(main);
        if method == "exponential" {
            return self.power_exponential(c, rest, opts, out);
        }
        if matches!(method, "oneslope" | "rsquared" | "pcorr") {
            return self.power_regression(c, method, rest, opts, out);
        }
        if matches!(method, "logrank" | "cox") {
            return self.power_survival(c, method, rest, opts, out);
        }
        if method == "oneway" {
            return self.power_oneway(c, rest, opts, out);
        }
        let Some((method, extra)) = METHODS.iter().find(|(m, _)| *m == method) else {
            return Err(no(format!(
                "power {method} is not translated yet (onemean, twomeans, pairedmeans, oneproportion, twoproportions, \
                 onevariance, twovariances, onecorrelation, twocorrelations, pairedproportions, oneway, logrank, cox, exponential, \
                 oneslope, rsquared and pcorr are)"
            )));
        };
        let later = LATER.contains(method);
        let mut known: Vec<(&str, usize)> = vec![
            ("alpha", 5),
            ("power", 5),
            ("n", 1),
            ("onesided", 8),
            ("direction", 9),
            ("table", 5),
            ("notable", 7),
        ];
        known.extend_from_slice(extra);
        if let Some(o) = opts.other(&known) {
            return Err(no(format!(
                "power {method} with the option {o} is not translated yet"
            )));
        }
        let words = value_words(rest)?;
        // pairedproportions with prdiscordant(): the discordant proportions are worked out
        let paired_sum = *method == "pairedproportions" && opts.has("prdiscordant", 12);
        if paired_sum && !words.is_empty() {
            return Err(no(
                "power pairedproportions with prdiscordant() and values is not translated yet",
            ));
        }
        if !paired_sum && (words.is_empty() || words.len() > 2) {
            return Err(no(format!(
                "power {method} takes one or two values (Stata: r(198))"
            )));
        }
        if *method == "pairedproportions" && words.len() == 1 {
            return Err(no(
                "power pairedproportions: the discordant proportions are worked out from prdiscordant(), n() and power() (Stata: r(198))",
            ));
        }
        let props = method.contains("proportion");
        let corrs = method.contains("correlation");
        // what stays fixed, and the lists of several values (Stata's table)
        let mut args: Vec<String> = Vec::new();
        let mut lists: Vec<String> = Vec::new();
        let mut put = |name: &str, xs: &[f64]| {
            if xs.len() > 1 {
                lists.push(format!("{name} = {}", r_vector(xs)));
            } else {
                args.push(format!("{name} = {}", r_vector(xs)));
            }
        };
        let mut given = Vec::new();
        let mut scale_sd = false;
        for (i, w) in words.iter().enumerate() {
            let inner = w
                .strip_prefix('(')
                .and_then(|s| s.strip_suffix(')'))
                .unwrap_or(w);
            let xs = numlist("the value", inner)?;
            for &x in &xs {
                if props && !(x > 0.0 && x < 1.0) {
                    return Err(no(format!(
                        "power {method}: {x} must be between 0 and 1 (Stata: r(198))"
                    )));
                }
                if corrs && !(x > -1.0 && x < 1.0) {
                    return Err(no(format!(
                        "power {method}: {x} must be between -1 and 1 (Stata: r(198))"
                    )));
                }
                if method.contains("variance") && x <= 0.0 {
                    return Err(no(format!(
                        "power {method}: {x} must be above 0 (Stata: r(198))"
                    )));
                }
            }
            put(if i == 0 { "v1" } else { "v2" }, &xs);
            given.push(xs);
        }
        if later {
            if let [a, b] = &given[..] {
                for x in a {
                    for y in b {
                        if x == y {
                            return Err(no(format!(
                                "power {method}: the two values must differ (Stata: r(198))"
                            )));
                        }
                        if *method == "pairedproportions" && x + y > 1.0 {
                            return Err(no(
                                "power pairedproportions: p12 + p21 must not pass 1 (Stata: r(198))",
                            ));
                        }
                    }
                }
            }
            // Stata's sd is a flag here: the values are standard deviations
            if let Some(s) = opts.arg("sd", 2) {
                if !s.trim().is_empty() {
                    return Err(no(format!(
                        "power {method}: sd takes no value here (Stata: r(198))"
                    )));
                }
                scale_sd = true;
            }
        }
        // the smallest group each later test is defined for (Stata prints a missing power below it)
        let least = match *method {
            "onevariance" | "twovariances" => 2.0,
            "onecorrelation" | "twocorrelations" => 4.0,
            _ => 1.0,
        };
        let two = method.starts_with("two");
        for &(name, min, range) in NUMBERS {
            if !known.iter().any(|(k, _)| *k == name) || (later && name == "sd") {
                continue;
            }
            if let Some(t) = opts.arg(name, min) {
                let xs = numlist(&format!("{name}()"), &t)?;
                for &x in &xs {
                    if !range.holds(x) {
                        return Err(no(format!(
                            "power {method}: {name}({}) {} (Stata: r(198))",
                            t.trim(),
                            range.why()
                        )));
                    }
                    let per = if two && name == "n" { 2.0 } else { 1.0 };
                    if later && matches!(name, "n" | "n1" | "n2") && x < least * per {
                        return Err(no(format!(
                            "power {method}: {name}({x}) is too small for the test (Stata prints a missing power)"
                        )));
                    }
                }
                put(name, &xs);
            }
        }
        if scale_sd {
            args.push("scale = \"standard deviation\"".into());
        }
        let has_n = opts.has("n", 1);
        let n1 = split_groups(method, opts, &mut args, &lists)?;
        if opts.has("sddiff", 6) && opts.has("corr", 4) {
            return Err(no(
                "power pairedmeans: sddiff() with corr() is not translated yet",
            ));
        }
        if opts.has("sd", 2) && (opts.has("sd1", 3) || opts.has("sd2", 3)) {
            return Err(no(
                "power twomeans: sd() with sd1() or sd2() is not translated yet",
            ));
        }
        if words.len() == 2 && opts.has("power", 5) && (has_n || n1) {
            return Err(no(format!(
                "power {method}: with two values, power() and n() are not both given (Stata: r(198))"
            )));
        }
        if words.len() < 2 && !(opts.has("power", 5) && (has_n || n1)) {
            return Err(no(format!(
                "power {method} {rest}: the target value needs n() and power() (Stata: r(198))"
            )));
        }
        if opts.has("onesided", 8) {
            args.push("onesided = TRUE".into());
        }
        if let Some(d) = opts.arg("direction", 9) {
            let d = d.trim().to_lowercase();
            if !matches!(d.as_str(), "upper" | "lower") {
                return Err(no(format!(
                    "power {method}: direction({d}) is upper or lower (Stata: r(198))"
                )));
            }
            args.push(format!("direction = {}", r_text(&d)));
        }
        if opts.has("knownsd", 7) || opts.has("knownsds", 8) {
            args.push("known = TRUE".into());
        }
        if let Some(t) = opts.arg("test", 4) {
            let t = t.trim().to_lowercase();
            match (*method, t.as_str()) {
                ("oneproportion", "score" | "wald") | ("twoproportions", "chi2") => {
                    args.push(format!("test = {}", r_text(&t)))
                }
                _ => {
                    return Err(no(format!(
                        "power {method}, test({t}) is not translated yet"
                    )))
                }
            }
        }
        if opts.has("continuity", 10) {
            args.push("continuity = TRUE".into());
        }
        if let Some(e) = opts.arg("effect", 6) {
            let e = e.trim().to_lowercase();
            let fits = match *method {
                "pairedproportions" => matches!(e.as_str(), "diff" | "ratio"),
                _ => matches!(e.as_str(), "diff" | "ratio" | "oratio"),
            };
            if !fits {
                return Err(no(format!(
                    "power {method}, effect({e}) is not translated yet"
                )));
            }
            args.push(format!("effect = {}", r_text(&e)));
        }
        // Stata's table: lists of values, or table asked for
        let table = match opts.arg("table", 5) {
            Some(t) if !t.trim().is_empty() => {
                return Err(no(format!(
                    "power {method}, table() with columns is not translated yet"
                )))
            }
            Some(_) => true,
            None => !lists.is_empty(),
        };
        let notable = opts.has("notable", 7);
        if notable && !table {
            return Err(no(format!(
                "power {method}, notable without lists of values is not translated yet"
            )));
        }
        let quiet = self.quiet(c);
        for h in ["stata_power", "stata_returned", "stata_fmt"] {
            self.helpers.insert(h);
        }
        let method_r = r_text(method);
        if table {
            out.line(format!(
                "stata_r <- stata_power_table({method_r}, lists = list({}), fixed = list({}){}{})",
                lists.join(", "),
                args.join(", "),
                if notable { ", notable = TRUE" } else { "" },
                if quiet { ", show = FALSE" } else { "" }
            ));
            // where Stata's return list puts a table's own r() is not followed
            self.r_new = Some((R_POWER, POWER_TABLE.into()));
        } else {
            if quiet {
                args.push("show = FALSE".into());
            }
            out.line(format!(
                "stata_r <- stata_power({method_r}, {})",
                args.join(", ")
            ));
            self.r_new = Some((R_POWER, "power".into()));
        }
        Ok(())
    }

    /// `power oneway m1 m2 ... [, ngroups() varmeans() varerror() n() npergroup() n#() grweights()
    /// alpha() power()]`: the F test of a group effect, as Stata 19 prints it.
    fn power_oneway(
        &mut self,
        c: &Command,
        rest: &str,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let groups: Vec<String> = (1..=20).map(|i| format!("n{i}")).collect();
        let mut known: Vec<(&str, usize)> = vec![
            ("alpha", 5),
            ("power", 5),
            ("n", 1),
            ("npergroup", 9),
            ("ngroups", 7),
            ("varmeans", 8),
            ("varerror", 8),
            ("grweights", 9),
        ];
        known.extend(groups.iter().map(|g| (g.as_str(), g.len())));
        if let Some(o) = opts.other(&known) {
            return Err(no(format!(
                "power oneway with the option {o} is not translated yet"
            )));
        }
        if rest.contains('(') {
            return Err(no(
                "power oneway: lists of values (Stata's table) are not translated yet for oneway",
            ));
        }
        let one = |name: &str, min: usize, range: Range| -> Result<Option<f64>, Untranslated> {
            let Some(t) = opts.arg(name, min) else {
                return Ok(None);
            };
            match numlist(&format!("{name}()"), &t)?[..] {
                [x] if range.holds(x) => Ok(Some(x)),
                [x] => Err(no(format!(
                    "power oneway: {name}({x}) {} (Stata: r(198))",
                    range.why()
                ))),
                _ => Err(no(format!(
                    "power oneway: a list in {name}() (Stata's table) is not translated yet for oneway"
                ))),
            }
        };
        let means = if rest.trim().is_empty() {
            Vec::new()
        } else {
            numlist("the means", rest)?
        };
        let ngroups = one("ngroups", 7, Range::Whole)?;
        let k = if means.is_empty() {
            ngroups.ok_or_else(|| {
                no("power oneway needs the group means or ngroups() (Stata: r(198))")
            })?
        } else {
            if ngroups.is_some() || opts.has("varmeans", 8) {
                return Err(no(
                    "power oneway: the means with ngroups() or varmeans() is not translated yet",
                ));
            }
            means.len() as f64
        };
        if !(2.0..=20.0).contains(&k) {
            return Err(no(format!(
                "power oneway with {k} groups is not translated (2 to 20 are)"
            )));
        }
        let k = k as usize;
        let mut args: Vec<String> = Vec::new();
        if !means.is_empty() {
            args.push(format!("means = {}", r_vector(&means)));
        } else {
            args.push(format!("ngroups = {k}"));
        }
        let varmeans = one("varmeans", 8, Range::Positive)?;
        for (name, min, range) in [
            ("varmeans", 8, Range::Positive),
            ("varerror", 8, Range::Positive),
            ("alpha", 5, Range::Prob),
            ("power", 5, Range::Prob),
            ("n", 1, Range::Whole),
            ("npergroup", 9, Range::Whole),
        ] {
            if let Some(x) = one(name, min, range)? {
                args.push(format!("{name} = {}", r_number(x)));
            }
        }
        let given: Vec<&String> = groups.iter().filter(|g| opts.has(g, g.len())).collect();
        let sizes_given = !given.is_empty();
        if sizes_given {
            if given.len() != k || groups.iter().take(k).any(|g| !opts.has(g, g.len())) {
                return Err(no(format!(
                    "power oneway: give n1() to n{k}(), one for each group (Stata: r(198))"
                )));
            }
            let mut ns = Vec::new();
            for g in groups.iter().take(k) {
                ns.push(one(g, g.len(), Range::Whole)?.unwrap_or(1.0));
            }
            args.push(format!("ns = {}", r_vector(&ns)));
        }
        let has_n = opts.has("n", 1) || opts.has("npergroup", 9);
        if [opts.has("n", 1), opts.has("npergroup", 9), sizes_given]
            .iter()
            .filter(|b| **b)
            .count()
            > 1
        {
            return Err(no(
                "power oneway: give one of n(), npergroup() and n#() (Stata: r(198))",
            ));
        }
        if let Some(t) = opts.arg("grweights", 9) {
            if has_n || sizes_given {
                return Err(no(
                    "power oneway: grweights() with n() or npergroup() is not translated yet",
                ));
            }
            let w = numlist("grweights()", &t)?;
            if w.len() != k || w.iter().any(|x| *x <= 0.0) {
                return Err(no(format!(
                    "power oneway: grweights() needs {k} weights above 0 (Stata: r(198))"
                )));
            }
            args.push(format!("grweights = {}", r_vector(&w)));
        }
        let effect = means.is_empty() && varmeans.is_none();
        if effect && !(has_n && opts.has("power", 5)) {
            return Err(no(
                "power oneway: the between-group variance needs n() (or npergroup()) and power() (Stata: r(198))",
            ));
        }
        let n_given = opts
            .arg("n", 1)
            .and_then(|t| t.trim().parse::<f64>().ok())
            .map(|n| n / k as f64)
            .or(opts
                .arg("npergroup", 9)
                .and_then(|t| t.trim().parse::<f64>().ok()));
        if n_given.is_some_and(|per| per < 1.0)
            || (has_n && n_given.is_some_and(|per| per * (k as f64) <= k as f64))
        {
            return Err(no(
                "power oneway: too few observations for the F test (Stata: r(198))",
            ));
        }
        if self.quiet(c) {
            args.push("show = FALSE".into());
        }
        for h in ["stata_power", "stata_returned"] {
            self.helpers.insert(h);
        }
        out.line(format!(
            "stata_r <- stata_power_oneway({})",
            args.join(", ")
        ));
        self.r_new = Some((R_ONEWAY, POWER_ONEWAY.into()));
        Ok(())
    }

    /// `power logrank [s1 [s2]]` and `power cox [b1]`: the log-rank test of two survivor functions
    /// and the Wald test of a Cox coefficient, as Stata 19 prints them.
    fn power_survival(
        &mut self,
        c: &Command,
        method: &str,
        rest: &str,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let lr = method == "logrank";
        let mut known: Vec<(&str, usize)> = vec![
            ("alpha", 5),
            ("power", 5),
            ("n", 1),
            ("onesided", 8),
            ("direction", 9),
            ("hratio", 6),
        ];
        if lr {
            known.extend_from_slice(&[("n1", 2), ("n2", 2), ("nratio", 6), ("schoenfeld", 10)]);
        } else {
            known.extend_from_slice(&[("sd", 2), ("r2", 2), ("eventprob", 9)]);
        }
        if let Some(o) = opts.other(&known) {
            return Err(no(format!(
                "power {method} with the option {o} is not translated yet"
            )));
        }
        if rest.contains('(') {
            return Err(no(format!(
                "power {method}: lists of values (Stata's table) are not translated yet for {method}"
            )));
        }
        let values = if rest.trim().is_empty() {
            Vec::new()
        } else {
            numlist("the value", rest)?
        };
        if values.len() > if lr { 2 } else { 1 } {
            return Err(no(format!(
                "power {method}: too many values (Stata: r(198))"
            )));
        }
        let one = |name: &str, min: usize| -> Result<Option<f64>, Untranslated> {
            let Some(t) = opts.arg(name, min) else {
                return Ok(None);
            };
            match numlist(&format!("{name}()"), &t)?[..] {
                [x] => Ok(Some(x)),
                _ => Err(no(format!(
                    "power {method}: a list in {name}() (Stata's table) is not translated yet for {method}"
                ))),
            }
        };
        let bad = |what: String| no(format!("power {method}: {what} (Stata: r(198))"));
        let mut args = vec![r_text(method)];
        if lr {
            for (i, x) in values.iter().enumerate() {
                if !(*x > 0.0 && *x < 1.0) {
                    return Err(bad(format!("{x} must be between 0 and 1")));
                }
                args.push(format!(
                    "{} = {}",
                    if i == 0 { "s1" } else { "s2" },
                    r_number(*x)
                ));
            }
            if let [a, b] = values[..] {
                if a == b {
                    return Err(bad("the two survival probabilities must differ".into()));
                }
            }
        } else if let Some(b) = values.first() {
            if *b == 0.0 {
                return Err(bad("the coefficient must not be 0".into()));
            }
            args.push(format!("b1 = {}", r_number(*b)));
        }
        let hratio = one("hratio", 6)?;
        if let Some(h) = hratio {
            if h <= 0.0 || h.is_nan() || h == 1.0 {
                return Err(bad(format!("hratio({h}) must be above 0 and not 1")));
            }
            if values.len() == if lr { 2 } else { 1 } {
                return Err(bad("hratio() with the values given".into()));
            }
            args.push(format!("hratio = {}", r_number(h)));
        }
        let effect = hratio.is_none() && values.len() < if lr { 2 } else { 1 };
        let ranges: &[(&str, usize, Range)] = &[
            ("alpha", 5, Range::Prob),
            ("power", 5, Range::Prob),
            ("n", 1, Range::Whole),
            ("n1", 2, Range::Whole),
            ("n2", 2, Range::Whole),
            ("nratio", 6, Range::Positive),
            ("sd", 2, Range::Positive),
        ];
        for &(name, min, range) in ranges {
            if !known.iter().any(|(k, _)| *k == name) {
                continue;
            }
            if let Some(x) = one(name, min)? {
                if !range.holds(x) {
                    return Err(bad(format!("{name}({x}) {}", range.why())));
                }
                args.push(format!("{name} = {}", r_number(x)));
            }
        }
        if !lr {
            if let Some(x) = one("r2", 2)? {
                if !(0.0..1.0).contains(&x) {
                    return Err(bad(format!("r2({x}) must be at least 0 and below 1")));
                }
                args.push(format!("r2 = {}", r_number(x)));
            }
            if let Some(x) = one("eventprob", 9)? {
                if !(x > 0.0 && x <= 1.0) {
                    return Err(bad(format!("eventprob({x}) must be above 0 and at most 1")));
                }
                args.push(format!("eventprob = {}", r_number(x)));
            }
        }
        let has_n = opts.has("n", 1);
        let n1 = split_groups(method, opts, &mut args, &[])?;
        if effect && !(opts.has("power", 5) && (has_n || n1)) {
            return Err(bad(format!(
                "the {} needs n() and power()",
                if lr { "hazard ratio" } else { "coefficient" }
            )));
        }
        if !effect && opts.has("power", 5) && (has_n || n1) {
            return Err(bad(
                "power() and n() are not both given with the effect".into()
            ));
        }
        if opts.has("schoenfeld", 10) {
            args.push("schoenfeld = TRUE".into());
        }
        if opts.has("onesided", 8) {
            args.push("onesided = TRUE".into());
        }
        if let Some(d) = opts.arg("direction", 9) {
            let d = d.trim().to_lowercase();
            if !matches!(d.as_str(), "upper" | "lower") {
                return Err(bad(format!("direction({d}) is upper or lower")));
            }
            args.push(format!("direction = {}", r_text(&d)));
        }
        if self.quiet(c) {
            args.push("show = FALSE".into());
        }
        for h in ["stata_power", "stata_returned"] {
            self.helpers.insert(h);
        }
        out.line(format!("stata_r <- stata_power_surv({})", args.join(", ")));
        self.r_new = Some((R_POWER, "power".into()));
        Ok(())
    }

    /// `power exponential h1 [h2]` (or `hratio()`), as Stata 19 prints it.
    fn power_exponential(
        &mut self,
        c: &Command,
        rest: &str,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let known: &[(&str, usize)] = &[
            ("alpha", 5),
            ("power", 5),
            ("n", 1),
            ("n1", 2),
            ("n2", 2),
            ("nratio", 6),
            ("onesided", 8),
            ("hratio", 6),
            ("loghazard", 9),
            ("unconditional", 13),
            ("studytime", 9),
        ];
        if let Some(o) = opts.other(known) {
            return Err(no(format!(
                "power exponential with the option {o} is not translated yet"
            )));
        }
        if rest.contains('(') {
            return Err(no(
                "power exponential: lists of values (Stata's table) are not translated yet",
            ));
        }
        let bad = |what: String| no(format!("power exponential: {what} (Stata: r(198))"));
        let values = numlist("the hazard rate", rest)?;
        let hr = match opts.arg("hratio", 6) {
            Some(t) => match numlist("hratio()", &t)?[..] {
                [x] if x > 0.0 && x != 1.0 => Some(x),
                [x] => return Err(bad(format!("hratio({x}) must be above 0 and not 1"))),
                _ => return Err(no(
                    "power exponential: a list in hratio() (Stata's table) is not translated yet",
                )),
            },
            None => None,
        };
        if values.iter().any(|x| *x <= 0.0) {
            return Err(bad("hazard rates must be above 0".into()));
        }
        let mut args = Vec::new();
        match (&values[..], hr) {
            ([a, b], None) => {
                if a == b {
                    return Err(bad(
                        "the control and experimental hazard rates must be different".into(),
                    ));
                }
                args.push(format!("h1 = {}", r_number(*a)));
                args.push(format!("h2 = {}", r_number(*b)));
            }
            ([a], Some(x)) => {
                args.push(format!("h1 = {}", r_number(*a)));
                args.push(format!("hratio = {}", r_number(x)));
            }
            ([_, _], Some(_)) => {
                return Err(bad(
                    "only one of the experimental-group hazard rate and hratio() is allowed".into(),
                ))
            }
            _ => return Err(bad("give the two hazard rates, or one and hratio()".into())),
        }
        let ranges: &[(&str, usize, Range)] = &[
            ("alpha", 5, Range::Prob),
            ("power", 5, Range::Prob),
            ("n", 1, Range::Whole),
            ("n1", 2, Range::Whole),
            ("n2", 2, Range::Whole),
            ("nratio", 6, Range::Positive),
            ("studytime", 9, Range::Positive),
        ];
        for &(name, min, range) in ranges {
            let Some(t) = opts.arg(name, min) else {
                continue;
            };
            let [x] = numlist(&format!("{name}()"), &t)?[..] else {
                return Err(no(format!(
                    "power exponential: a list in {name}() (Stata's table) is not translated yet"
                )));
            };
            if !range.holds(x) || (name == "studytime" && x <= 1.0e-4) {
                return Err(bad(format!("{name}({x}) {}", range.why())));
            }
            args.push(format!("{name} = {}", r_number(x)));
        }
        let (has_n, has_power) = (opts.has("n", 1), opts.has("power", 5));
        let groups = opts.has("n1", 2) || opts.has("n2", 2);
        if has_power && (has_n || groups) {
            return Err(bad("effect-size determination is not available".into()));
        }
        split_groups("exponential", opts, &mut args, &[])?;
        for (flag, min, r) in [
            ("onesided", 8, "onesided"),
            ("loghazard", 9, "loghazard"),
            ("unconditional", 13, "unconditional"),
        ] {
            if opts.has(flag, min) {
                args.push(format!("{r} = TRUE"));
            }
        }
        if self.quiet(c) {
            args.push("show = FALSE".into());
        }
        for h in ["stata_power", "stata_returned"] {
            self.helpers.insert(h);
        }
        out.line(format!("stata_r <- stata_power_exp({})", args.join(", ")));
        self.r_new = Some((R_POWER, "power".into()));
        Ok(())
    }

    /// `power oneslope b0 [ba]`, `power rsquared [R2_R] [R2_F]` and `power pcorr [rho2_p]`, as
    /// Stata 19 prints them.
    fn power_regression(
        &mut self,
        c: &Command,
        method: &str,
        rest: &str,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let slope = method == "oneslope";
        let mut known: Vec<(&str, usize)> = vec![("alpha", 5), ("power", 5), ("n", 1)];
        if slope {
            known.extend_from_slice(&[
                ("onesided", 8),
                ("direction", 9),
                ("sdx", 3),
                ("sderror", 7),
                ("sdy", 3),
            ]);
        } else {
            known.extend_from_slice(&[("ncontrol", 8), ("ntested", 7)]);
        }
        if let Some(o) = opts.other(&known) {
            return Err(no(format!(
                "power {method} with the option {o} is not translated yet"
            )));
        }
        if rest.contains('(') {
            return Err(no(format!(
                "power {method}: lists of values (Stata's table) are not translated yet for {method}"
            )));
        }
        let bad = |what: String| no(format!("power {method}: {what} (Stata: r(198))"));
        let values = if rest.trim().is_empty() {
            Vec::new()
        } else {
            numlist("the value", rest)?
        };
        let one = |name: &str, min: usize| -> Result<Option<f64>, Untranslated> {
            let Some(t) = opts.arg(name, min) else {
                return Ok(None);
            };
            match numlist(&format!("{name}()"), &t)?[..] {
                [x] => Ok(Some(x)),
                _ => Err(no(format!(
                    "power {method}: a list in {name}() (Stata's table) is not translated yet for {method}"
                ))),
            }
        };
        let mut args = vec![r_text(method)];
        let most = if method == "pcorr" { 1 } else { 2 };
        if values.len() > most || (slope && values.is_empty()) {
            return Err(bad("the wrong number of values".into()));
        }
        if !slope {
            for x in &values {
                if !(*x > 0.0 && *x < 1.0) {
                    return Err(bad(format!("{x} must be between 0 and 1")));
                }
            }
        }
        if let [a, b] = values[..] {
            if a == b || (!slope && b <= a) {
                return Err(bad(format!(
                    "{b} must {} {a}",
                    if slope { "differ from" } else { "be above" }
                )));
            }
        }
        for (i, x) in values.iter().enumerate() {
            args.push(format!(
                "{} = {}",
                if i == 0 { "v1" } else { "v2" },
                r_number(*x)
            ));
        }
        let ranges: &[(&str, usize, Range)] = &[
            ("alpha", 5, Range::Prob),
            ("power", 5, Range::Prob),
            ("n", 1, Range::Whole),
            ("sdx", 3, Range::Positive),
            ("sderror", 7, Range::Positive),
            ("sdy", 3, Range::Positive),
            ("ncontrol", 8, Range::Whole),
            ("ntested", 7, Range::Whole),
        ];
        for &(name, min, range) in ranges {
            if !known.iter().any(|(k, _)| *k == name) {
                continue;
            }
            if let Some(x) = one(name, min)? {
                if !range.holds(x) {
                    return Err(bad(format!("{name}({x}) {}", range.why())));
                }
                args.push(format!("{name} = {}", r_number(x)));
            }
        }
        if opts.has("sderror", 7) && opts.has("sdy", 3) {
            return Err(bad("sderror() and sdy() are not both given".into()));
        }
        let ncontrol = opts.has("ncontrol", 8);
        if method == "pcorr" && !ncontrol {
            return Err(no("power pcorr without ncontrol() is not translated yet"));
        }
        // what is worked out: the effect needs n() and power(); rsquared with one value and
        // ncontrol() is the reduced model's R2, so it can only be the target
        let effect = if slope {
            values.len() == 1
        } else if method == "rsquared" && ncontrol {
            values.len() < 2
        } else {
            values.is_empty()
        };
        let (has_n, has_power) = (opts.has("n", 1), opts.has("power", 5));
        if effect && !(has_n && has_power) {
            return Err(bad(format!(
                "the effect needs n() and power(){}",
                if method == "rsquared" && ncontrol && values.len() == 1 {
                    ", or give the full model's R2 too"
                } else {
                    ""
                }
            )));
        }
        if method == "rsquared" && ncontrol && values.is_empty() {
            return Err(bad("the reduced model's R2 is needed".into()));
        }
        if !effect && has_n && has_power {
            return Err(bad(
                "power() and n() are not both given with the effect".into()
            ));
        }
        if opts.has("onesided", 8) {
            args.push("onesided = TRUE".into());
        }
        if let Some(d) = opts.arg("direction", 9) {
            let d = d.trim().to_lowercase();
            if !matches!(d.as_str(), "upper" | "lower") {
                return Err(bad(format!("direction({d}) is upper or lower")));
            }
            args.push(format!("direction = {}", r_text(&d)));
        }
        if self.quiet(c) {
            args.push("show = FALSE".into());
        }
        for h in ["stata_power", "stata_returned"] {
            self.helpers.insert(h);
        }
        out.line(format!("stata_r <- stata_power_reg({})", args.join(", ")));
        self.r_new = Some((R_POWER, "power".into()));
        Ok(())
    }
}

/// n1() or n2() alone (with nratio()), or n() with one of them: both groups,
/// as Stata 19 works them out (n1(40) nratio(2): 40 and 80; n(100) n2(70): 30
/// and 70), passed to R as n1 and n2. Whether the groups were given.
fn split_groups(
    method: &str,
    opts: &Opts,
    args: &mut Vec<String>,
    lists: &[String],
) -> Result<bool, Untranslated> {
    let (has_n, n1, n2, ratio) = (
        opts.has("n", 1),
        opts.has("n1", 2),
        opts.has("n2", 2),
        opts.has("nratio", 6),
    );
    if !n1 && !n2 {
        return Ok(false);
    }
    if n1 && n2 && (has_n || ratio) {
        return Err(no(format!(
            "power {method}: n() or nratio() with n1() and n2() cannot be given together (Stata: r(198))"
        )));
    }
    if n1 && n2 {
        return Ok(true);
    }
    if has_n && ratio {
        return Err(no(format!(
            "power {method}: n() and nratio() with n1() or n2() is not translated yet"
        )));
    }
    let sizes = ["n = ", "n1 = ", "n2 = ", "nratio = "];
    if lists.iter().any(|l| sizes.iter().any(|s| l.starts_with(s))) {
        return Err(no(format!(
            "power {method}: lists in n(), n1(), n2() or nratio() with one group's size are not translated yet"
        )));
    }
    let get = |name: &str, min: usize| {
        opts.arg(name, min)
            .and_then(|t| t.trim().parse::<f64>().ok())
    };
    let r = get("nratio", 6).unwrap_or(1.0);
    let (a, b) = match (get("n", 1), get("n1", 2), get("n2", 2)) {
        (Some(n), Some(a), _) if has_n => (a, n - a),
        (Some(n), _, Some(b)) if has_n => (n - b, b),
        (_, Some(a), _) => (a, a * r),
        (_, _, Some(b)) => (b / r, b),
        _ => return Err(no(format!("power {method}: n1() or n2() needs a number"))),
    };
    let whole = |x: f64| x >= 1.0 && (x - x.round()).abs() < 1e-9;
    if !whole(a) || !whole(b) {
        return Err(no(format!(
            "power {method}: groups of {a} and {b} are not whole numbers (Stata's actual sample sizes from one group's size are not translated yet)"
        )));
    }
    args.retain(|x| !sizes.iter().any(|s| x.starts_with(s)));
    args.push(format!("n1 = {}", r_number(a.round())));
    args.push(format!("n2 = {}", r_number(b.round())));
    Ok(true)
}
