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
                        effect = "diff", scale = "variance", prdiscordant = NULL, show = TRUE) {
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
      cut <- if (is.null(df)) zc else stats::qt(1 - alpha / side, df)
      if (!onesided) up(cut, df, l) + dn(cut, df, l) else if (l >= 0) up(cut, df, l) else dn(cut, df, l)
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
    c(a, floor(nratio * a + 1e-9))
  }
  tol <- 1e-12
  iterated <- TRUE
  closed <- onesided && (known || props) && !(find == "effect" && props)
  if (find == "power") {
    g <- sizes()
    pv <- pw(g[1], g[2], v2)
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
      tail <- if (!iterated) c(NN, list(N1 = g[1], N2 = g[2], nratio = nratio, m1 = v1, m2 = x2, diff = diff), sdl, ks)
        else if (find == "n") c(list(N = N), ksr, sdr, list(diff = diff, m2 = x2, m1 = v1, nratio = nratio, N2 = g[2], N1 = g[1]))
        else c(NN, list(N1 = g[1]), ksr, sdr, list(diff = diff, m2 = x2, m1 = v1, nratio = nratio, N2 = g[2]))
      c(head, list(delta = delta), nf, os, it, tail)
    },
    twoproportions = {
      na <- g[2] / g[1]
      tail <- if (!iterated) c(NN, list(N1 = g[1], N2 = g[2], nratio = nratio, p1 = v1, p2 = x2, continuity = as.numeric(continuity)))
        else if (find == "n") c(list(N = N, continuity = as.numeric(continuity), p2 = x2, p1 = v1),
                                if (abs(na - nratio) > 1e-12) list(nratio_a = na), list(nratio = nratio, N2 = g[2], N1 = g[1]))
        else c(NN, list(N1 = g[1], continuity = as.numeric(continuity), p2 = x2, p1 = v1, nratio = nratio, N2 = g[2]))
      c(head, list(delta = delta), nf, os, it, tail)
    })
  r <- c(r, list(type = "test", method = method), if (find == "effect") list(direction = if (direction == "lower") "lower" else "upper"),
         if (method == "oneproportion") list(test = test), if (method == "twoproportions") list(test = "chi2", effect = effect))
  if (!show) return(r)
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
  num <- function(k, v, extra = "") sprintf("%13s = %9.4f%s", k, v, extra)
  int <- function(k, v) sprintf("%13s = %9.0f", k, v)
  eff <- if (method == "twoproportions") paste0("  (", switch(effect, diff = "difference", ratio = "ratio", oratio = "odds ratio"), ")") else ""
  sizes_lines <- function(tot = N) {
    if (!two) return(int("N", tot))
    c(int("N", tot), if (g[1] == g[2]) int("N per group", g[1]) else c(int("N1", g[1]), int("N2", g[2])))
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
    c(a, floor(nratio * a + 1e-9))
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
    tail <- if (!iterated) c(NN, list(N1 = g[1], N2 = g[2], nratio = nratio), if (find == "n" && abs(na - nratio) > 1e-12) list(nratio_a = na), P)
      else if (find == "n") c(list(N = N), rev(P), if (abs(na - nratio) > 1e-12) list(nratio_a = na), list(nratio = nratio, N2 = g[2], N1 = g[1]))
      else c(NN, list(N1 = g[1]), rev(P), list(nratio = nratio, N2 = g[2]))
    r <- c(head, list(delta = delta, nfractional = 0), os, it, tail)
  }
  dl <- if (find == "effect") list(direction = if (direction == "lower") "lower" else "upper")
  r <- c(r, switch(method,
    onevariance = c(list(type = "test", method = method), dl, list(scale = scale)),
    twovariances = c(list(scale = scale, type = "test", method = method), dl),
    pairedproportions = c(list(effect = effect, type = "test", method = method), dl),
    c(list(type = "test", method = method), dl)))
  if (!show) return(invisible(r))
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
  num <- function(k, v, extra = "") sprintf("%13s = %9.4f%s", k, v, extra)
  int <- function(k, v) sprintf("%13s = %9.0f", k, v)
  eff <- if (pp) paste0("  (", if (effect == "ratio") "ratio" else "difference", ")") else ""
  sizes_lines <- function(tot = N) {
    if (!two) return(int("N", tot))
    c(int("N", tot), if (g[1] == g[2]) int("N per group", g[1]) else c(int("N1", g[1]), int("N2", g[2])))
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

/// One number of the line: a value, or an option's.
fn number(what: &str, text: &str) -> Result<f64, Untranslated> {
    let t = text.trim();
    if t.split_whitespace().count() > 1 || t.contains('(') {
        return Err(no(format!(
            "power: {what} {t}: a list of values (Stata's table of results) is not translated yet"
        )));
    }
    t.parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
        .ok_or_else(|| no(format!("power: {what} {t} needs a number")))
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
        let Some((method, extra)) = METHODS.iter().find(|(m, _)| *m == method) else {
            return Err(no(format!(
                "power {method} is not translated yet (onemean, twomeans, pairedmeans, oneproportion, twoproportions, \
                 onevariance, twovariances, onecorrelation, twocorrelations and pairedproportions are)"
            )));
        };
        let later = LATER.contains(method);
        let mut known: Vec<(&str, usize)> = vec![
            ("alpha", 5),
            ("power", 5),
            ("n", 1),
            ("onesided", 8),
            ("direction", 9),
        ];
        known.extend_from_slice(extra);
        if let Some(o) = opts.other(&known) {
            return Err(no(format!(
                "power {method} with the option {o} is not translated yet"
            )));
        }
        if rest.contains('(') {
            return Err(no(format!(
                "power {method} {rest}: a list of values (Stata's table of results) is not translated yet"
            )));
        }
        let values: Vec<&str> = rest.split_whitespace().collect();
        // pairedproportions with prdiscordant(): the discordant proportions are worked out
        let paired_sum = *method == "pairedproportions" && opts.has("prdiscordant", 12);
        if paired_sum && !values.is_empty() {
            return Err(no(
                "power pairedproportions with prdiscordant() and values is not translated yet",
            ));
        }
        if !paired_sum && (values.is_empty() || values.len() > 2) {
            return Err(no(format!(
                "power {method} takes one or two values (Stata: r(198))"
            )));
        }
        if *method == "pairedproportions" && values.len() == 1 {
            return Err(no(
                "power pairedproportions: the discordant proportions are worked out from prdiscordant(), n() and power() (Stata: r(198))",
            ));
        }
        let props = method.contains("proportion");
        let corrs = method.contains("correlation");
        let mut args = vec![r_text(method)];
        let mut given = Vec::new();
        for (i, v) in values.iter().enumerate() {
            let x = number("the value", v)?;
            if props && !(x > 0.0 && x < 1.0) {
                return Err(no(format!(
                    "power {method}: {v} must be between 0 and 1 (Stata: r(198))"
                )));
            }
            if corrs && !(x > -1.0 && x < 1.0) {
                return Err(no(format!(
                    "power {method}: {v} must be between -1 and 1 (Stata: r(198))"
                )));
            }
            if method.contains("variance") && x <= 0.0 {
                return Err(no(format!(
                    "power {method}: {v} must be above 0 (Stata: r(198))"
                )));
            }
            given.push(x);
            args.push(if i == 0 {
                r_number(x)
            } else {
                format!("v2 = {}", r_number(x))
            });
        }
        if later {
            if let [a, b] = given[..] {
                if a == b {
                    return Err(no(format!(
                        "power {method}: the two values must differ (Stata: r(198))"
                    )));
                }
                if *method == "pairedproportions" && a + b > 1.0 {
                    return Err(no(
                        "power pairedproportions: p12 + p21 must not pass 1 (Stata: r(198))",
                    ));
                }
            }
            // Stata's sd is a flag here: the values are standard deviations
            if let Some(s) = opts.arg("sd", 2) {
                if !s.trim().is_empty() {
                    return Err(no(format!(
                        "power {method}: sd takes no value here (Stata: r(198))"
                    )));
                }
                args.push("scale = \"standard deviation\"".into());
            }
        }
        for &(name, min, range) in NUMBERS {
            if !known.iter().any(|(k, _)| *k == name) || (later && name == "sd") {
                continue;
            }
            if let Some(t) = opts.arg(name, min) {
                let x = number(&format!("{name}()"), &t)?;
                if !range.holds(x) {
                    return Err(no(format!(
                        "power {method}: {name}({}) {} (Stata: r(198))",
                        t.trim(),
                        range.why()
                    )));
                }
                args.push(format!("{name} = {}", r_number(x)));
            }
        }
        let has_n = opts.has("n", 1);
        let (n1, n2) = (opts.has("n1", 2), opts.has("n2", 2));
        if n1 != n2 {
            return Err(no(format!(
                "power {method}: n1() or n2() alone is not translated yet; give both"
            )));
        }
        if n1 && (has_n || opts.has("nratio", 6)) {
            return Err(no(format!(
                "power {method}: n() or nratio() with n1() and n2() is not translated yet"
            )));
        }
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
        if values.len() < 2 && !(opts.has("power", 5) && (has_n || n1)) {
            return Err(no(format!(
                "power {method} {rest}: the target value needs n() and power() (Stata: r(198))"
            )));
        }
        // the smallest group each later test is defined for (Stata prints a missing power below it)
        let least = match *method {
            "onevariance" | "twovariances" => 2.0,
            "onecorrelation" | "twocorrelations" => 4.0,
            _ => 1.0,
        };
        let two = method.starts_with("two");
        for (name, min, per) in [
            ("n", 1, if two { 2.0 } else { 1.0 }),
            ("n1", 2, 1.0),
            ("n2", 2, 1.0),
        ] {
            if let Some(x) = opts
                .arg(name, min)
                .and_then(|t| t.trim().parse::<f64>().ok())
            {
                if later && x < least * per {
                    return Err(no(format!(
                        "power {method}: {name}({x}) is too small for the test (Stata prints a missing power)"
                    )));
                }
            }
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
        if self.quiet(c) {
            args.push("show = FALSE".into());
        }
        for h in ["stata_power", "stata_returned"] {
            self.helpers.insert(h);
        }
        out.line(format!("stata_r <- stata_power({})", args.join(", ")));
        self.r_new = Some((R_POWER, "power".into()));
        Ok(())
    }
}
