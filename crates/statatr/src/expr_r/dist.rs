//! Stata's distribution functions in R (`ttail()`, `invFtail()`,
//! `chi2tail()`, `normalden()`, `ibeta()`, `binomial()`, `comb()`, ...), with
//! Stata 19's missing values where R gives a number, observed on 3,700
//! arguments (`plan/stata/distributions.md`).
//!
//! R's `pt`, `pf`, `pchisq`, `pbeta`, `pgamma`, `pbinom`, `ppois` and their
//! densities give Stata's doubles to 1e-12 or closer. The rules R does not
//! share are written out here: a degree of freedom, shape or standard
//! deviation must be above 0 (R takes 0 as a point mass), the t, F and chi2
//! degrees of freedom at most 2e17 and a beta or gamma shape at most 1e17
//! (Stata gives missing above), the counts of the discrete distributions are
//! cut to whole numbers, and `comb()` and `lnfactorial()` take whole numbers
//! only. R's `qf` is not used: it gives 0 for small p and swaps to chi2 for
//! df2 above 4e5, where Stata's `invF()` is right; the F quantile comes from
//! `qbeta` instead, on whichever side of one half keeps its digits.
//!
//! Batch F (`plan/stata/functions-f.md`) adds the noncentral `nt()`, `nttail()`, `nchi2()`,
//! `nchi2tail()`, `nF()` and `nFtail()` (R's `pt`, `pchisq` and `pf` with `ncp`, and the limits past
//! which Stata gives missing) and the old names `chiprob()`, `fprob()` and `invfprob()`. Its third
//! round adds `invnF()`, `invnFtail()`, `npnF()`, `npnchi2()` and `npnt()`: root-finding on a
//! noncentral F summed as a Poisson mixture of betas (R's `pf` with `ncp` is good only to 1e-9),
//! with the edges past which Stata gives missing, found by bisection on Stata 19.

use super::*;

/// Each distribution function, with the numbers of arguments it takes.
pub(super) const FUNCTIONS: &[(&str, &[usize])] = &[
    ("F", &[3]),
    ("Fden", &[3]),
    ("Ftail", &[3]),
    ("betaden", &[3]),
    ("binomial", &[3]),
    ("binomialp", &[3]),
    ("binomialtail", &[3]),
    ("chi2", &[2]),
    ("chi2den", &[2]),
    ("chi2tail", &[2]),
    ("chiprob", &[2]),
    ("comb", &[2]),
    ("digamma", &[1]),
    ("fprob", &[3]),
    ("gammaden", &[4]),
    ("gammap", &[2]),
    ("gammaptail", &[2]),
    ("ibeta", &[3]),
    ("ibetatail", &[3]),
    ("invF", &[3]),
    ("invFtail", &[3]),
    ("invchi2", &[2]),
    ("invchi2tail", &[2]),
    ("invfprob", &[3]),
    ("invgammap", &[2]),
    ("invgammaptail", &[2]),
    ("invibeta", &[3]),
    ("invibetatail", &[3]),
    ("invnF", &[4]),
    ("invnFtail", &[4]),
    ("invnchi2", &[3]),
    ("invnchi2tail", &[3]),
    ("invnt", &[3]),
    ("invnttail", &[3]),
    ("invt", &[2]),
    ("invttail", &[2]),
    ("lnfactorial", &[1]),
    ("lngamma", &[1]),
    ("lnnormal", &[1]),
    ("lnnormalden", &[1, 2, 3]),
    ("nF", &[4]),
    ("nFtail", &[4]),
    ("nchi2", &[3]),
    ("nchi2tail", &[3]),
    ("normalden", &[1, 2, 3]),
    ("npnF", &[4]),
    ("npnchi2", &[3]),
    ("npnt", &[3]),
    ("nt", &[3]),
    ("nttail", &[3]),
    ("poisson", &[2]),
    ("poissonp", &[2]),
    ("poissontail", &[2]),
    ("t", &[2]),
    ("tden", &[2]),
    ("tprob", &[2]),
    ("trigamma", &[1]),
    ("ttail", &[2]),
];

pub(super) fn is_distribution(name: &str) -> bool {
    FUNCTIONS.iter().any(|(f, _)| *f == name)
}

/// The R helpers, one a function (`stata_d_ttail()` for `ttail()`).
pub(crate) const DIST_HELPER: &str = r##"stata_d_keep <- function(r, ok) {
  ok <- rep_len(ok, length(r))
  r[is.na(ok) | !ok | !is.finite(r)] <- NA_real_
  r
}
stata_d_pos <- function(d, hi = Inf) !is.na(d) & d > 0 & d <= hi
stata_d_whole <- function(v) !is.na(v) & is.finite(v) & v == floor(v)
stata_d_prob <- function(p) !is.na(p) & p >= 0 & p <= 1
stata_d_run <- function(e) suppressWarnings(e)
stata_d_t <- function(d, x) stata_d_keep(stata_d_run(stats::pt(x, d)), stata_d_pos(d, 2e17))
stata_d_ttail <- function(d, x) stata_d_keep(stata_d_run(stats::pt(x, d, lower.tail = FALSE)), stata_d_pos(d, 2e17))
stata_d_tden <- function(d, x) stata_d_keep(stata_d_run(stats::dt(x, d)), stata_d_pos(d))
stata_d_tprob <- function(d, x) stata_d_keep(stata_d_run(2 * stats::pt(-abs(x), d)), stata_d_pos(d, 2e17))
# Stata 19 gives missing where the quantile passes 9.3675e153 in size (bracketed to 9.365e153 ..
# 9.370e153 with 1 and 0.9 degrees of freedom)
stata_d_tbig <- function(q) { q[!is.na(q) & abs(q) > 9.3675e153] <- NA_real_; q }
stata_d_invt <- function(d, p) stata_d_keep(stata_d_tbig(stata_d_run(stats::qt(p, d))), stata_d_pos(d, 2e17))
# by symmetry: R's upper tail fails for tiny p below 1 degree of freedom, its lower tail does not
stata_d_invttail <- function(d, p) stata_d_keep(stata_d_tbig(-stata_d_run(stats::qt(p, d))), stata_d_pos(d, 2e17))
stata_d_F <- function(a, b, f) stata_d_keep(stata_d_run(stats::pf(f, a, b)), stata_d_pos(a, 2e17) & stata_d_pos(b, 2e17))
stata_d_Ftail <- function(a, b, f) stata_d_keep(stata_d_run(stats::pf(f, a, b, lower.tail = FALSE)), stata_d_pos(a, 2e17) & stata_d_pos(b, 2e17))
stata_d_Fden <- function(a, b, f) stata_d_keep(stata_d_run(stats::df(f, a, b)), stata_d_pos(a) & stata_d_pos(b))
stata_d_invF_side <- function(a, b, p, upper) {
  x <- stata_d_run(stats::qbeta(p, a / 2, b / 2, lower.tail = !upper))
  y <- stata_d_run(stats::qbeta(p, b / 2, a / 2, lower.tail = upper))
  r <- ifelse(!is.na(x) & x < 0.5, (b / a) * x / (1 - x), (b / a) * (1 - y) / y)
  stata_d_keep(r, stata_d_pos(a, 2e17) & stata_d_pos(b, 2e17))
}
stata_d_invF <- function(a, b, p) stata_d_invF_side(a, b, p, FALSE)
stata_d_invFtail <- function(a, b, p) stata_d_invF_side(a, b, p, TRUE)
stata_d_chi2 <- function(d, x) stata_d_keep(stata_d_run(stats::pchisq(x, d)), stata_d_pos(d, 2e17))
stata_d_chi2tail <- function(d, x) stata_d_keep(stata_d_run(stats::pchisq(x, d, lower.tail = FALSE)), stata_d_pos(d, 2e17))
stata_d_chi2den <- function(d, x) stata_d_keep(stata_d_run(stats::dchisq(x, d)), stata_d_pos(d, 2e17))
stata_d_invchi2 <- function(d, p) stata_d_keep(stata_d_run(stats::qchisq(p, d)), stata_d_pos(d, 2e17))
stata_d_invchi2tail <- function(d, p) stata_d_keep(stata_d_run(stats::qchisq(p, d, lower.tail = FALSE)), stata_d_pos(d, 2e17))
stata_d_normalden <- function(z) stata_d_keep(stats::dnorm(z), TRUE)
stata_d_normalden_s <- function(z, s) {
  r <- stata_d_run(stats::dnorm(z, 0, s))
  n <- length(r)
  ok <- rep_len(stata_d_pos(s), n)
  # Stata 19: normalden(., s) is 0, not missing, for a valid s
  r[is.na(rep_len(z, n)) & ok] <- 0
  stata_d_keep(r, ok)
}
stata_d_normalden_ms <- function(z, m, s) stata_d_keep(stata_d_run(stats::dnorm(z, m, s)), stata_d_pos(s))
stata_d_lnnormal <- function(z) stata_d_keep(stats::pnorm(z, log.p = TRUE), TRUE)
stata_d_lnnormalden <- function(z) stata_d_keep(stats::dnorm(z, log = TRUE), TRUE)
stata_d_lnnormalden_s <- function(z, s) stata_d_keep(stata_d_run(stats::dnorm(z, 0, s, log = TRUE)), stata_d_pos(s))
stata_d_lnnormalden_ms <- function(z, m, s) stata_d_keep(stata_d_run(stats::dnorm(z, m, s, log = TRUE)), stata_d_pos(s))
stata_d_ibeta <- function(a, b, x) stata_d_keep(stata_d_run(stats::pbeta(x, a, b)), stata_d_pos(a, 1e17) & stata_d_pos(b, 1e17))
stata_d_ibetatail <- function(a, b, x) stata_d_keep(stata_d_run(stats::pbeta(x, a, b, lower.tail = FALSE)), stata_d_pos(a, 1e17) & stata_d_pos(b, 1e17))
stata_d_betaden <- function(a, b, x) stata_d_keep(stata_d_run(stats::dbeta(x, a, b)), stata_d_pos(a) & stata_d_pos(b))
stata_d_invibeta <- function(a, b, p) stata_d_keep(stata_d_run(stats::qbeta(p, a, b)), stata_d_pos(a, 1e17) & stata_d_pos(b, 1e17))
stata_d_invibetatail <- function(a, b, p) stata_d_keep(stata_d_run(stats::qbeta(p, a, b, lower.tail = FALSE)), stata_d_pos(a, 1e17) & stata_d_pos(b, 1e17))
stata_d_gammap <- function(a, x) stata_d_keep(stata_d_run(stats::pgamma(x, a)), stata_d_pos(a, 1e17))
stata_d_gammaptail <- function(a, x) stata_d_keep(stata_d_run(stats::pgamma(x, a, lower.tail = FALSE)), stata_d_pos(a, 1e17))
stata_d_gammaden <- function(a, b, g, x) stata_d_keep(stata_d_run(stats::dgamma(x - g, shape = a, scale = b)), stata_d_pos(a) & stata_d_pos(b))
stata_d_invgammap <- function(a, p) stata_d_keep(stata_d_run(stats::qgamma(p, a)), stata_d_pos(a, 1e17))
stata_d_invgammaptail <- function(a, p) stata_d_keep(stata_d_run(stats::qgamma(p, a, lower.tail = FALSE)), stata_d_pos(a, 1e17))
stata_d_lngamma <- function(x) stata_d_keep(stata_d_run(lgamma(x)), TRUE)
stata_d_digamma <- function(x) stata_d_keep(stata_d_run(digamma(x)), TRUE)
stata_d_trigamma <- function(x) stata_d_keep(stata_d_run(trigamma(x)), TRUE)
stata_d_lnfactorial <- function(n) stata_d_keep(stata_d_run(lgamma(n + 1)), stata_d_whole(n) & n >= 0)
stata_d_comb <- function(n, k) {
  ok <- stata_d_whole(n) & stata_d_whole(k) & k >= 0 & k <= n
  stata_d_keep(stata_d_run(choose(n, k)), ok)
}
stata_d_binok <- function(n, p) !is.na(n) & n >= 0 & n <= 1e17 & stata_d_prob(p)
stata_d_binomial <- function(n, k, p) stata_d_keep(stata_d_run(stats::pbinom(floor(k), floor(n), p)), stata_d_binok(n, p))
stata_d_binomialtail <- function(n, k, p) stata_d_keep(stata_d_run(stats::pbinom(floor(k) - 1, floor(n), p, lower.tail = FALSE)), stata_d_binok(n, p))
stata_d_binomialp <- function(n, k, p) {
  # Stata 19 gives missing for n (cut to a whole number) above 1e6
  ok <- stata_d_binok(n, p) & floor(n) >= 1 & floor(n) <= 1e6 & !is.na(k) & floor(k) >= 0 & floor(k) <= floor(n)
  stata_d_keep(stata_d_run(stats::dbinom(floor(k), floor(n), p)), ok)
}
# Stata 19 gives missing for a mean above 2^53 (the cdf and tail), and above 1e8 or a count
# above 1e9 (the probability)
stata_d_poisok <- function(m, k) stata_d_pos(m, 2^53) & !is.na(k) & k >= 0
stata_d_poisson <- function(m, k) stata_d_keep(stata_d_run(stats::ppois(floor(k), m)), stata_d_poisok(m, k))
stata_d_poissontail <- function(m, k) stata_d_keep(stata_d_run(stats::ppois(floor(k) - 1, m, lower.tail = FALSE)), stata_d_poisok(m, k))
stata_d_poissonp <- function(m, k) stata_d_keep(stata_d_run(stats::dpois(floor(k), m)), stata_d_poisok(m, k) & m <= 1e8 & k <= 1e9)
# the old names: chiprob() is chi2tail() except that it gives 1, not missing, above 2e17 degrees of freedom
stata_d_chiprob <- function(d, x) {
  r <- stata_d_chi2tail(d, x)
  big <- !is.na(d) & d > 2e17 & !is.na(x)
  r[rep_len(big, length(r))] <- 1
  r
}
stata_d_fprob <- stata_d_Ftail
stata_d_invfprob <- stata_d_invFtail
# the noncentral distributions are R's (AS 243 and the series of R's pnchisq and pnbeta), to about 1e-9 apart
# in the cdf; Stata gives missing for nt() and nttail() with df above 1e10 or |np| above 1000, for nchi2()
# df above 1e6 or np outside 0 .. 1e4, for nF() df above 1e8 or np outside 0 .. 1e4 (nFtail(): no limit on df,
# np 0 .. 1000),
# and R's pt() turns to a normal approximation at |np| above 37.62 that Stata does not use: there the cdf is
# integrated, nt = E[pnorm(t w - np)] for w = sqrt(chisq(df) / df), which gives Stata's values to about 1e-13
# R gives wrong values for |t| above 1e154 (Stata: 0 and 1)
stata_d_ntbig <- function(t) { big <- !is.na(t) & abs(t) > 1e100; t[big] <- sign(t[big]) * Inf; t }
stata_d_ntint <- function(d, np, t, upper) {
  f <- function(w) {
    z <- t * w - np
    (if (upper) stats::pnorm(-z) else stats::pnorm(z)) * 2 * d * w * stats::dchisq(d * w^2, d)
  }
  q <- sqrt(stats::qchisq(c(1e-12, .001, .5, .999, 1 - 1e-12), d) / d)
  # the integrand turns over where t w = np, within 8 / |t| of it
  w0 <- if (t != 0 && np / t > 0) np / t
  near <- if (!is.null(w0)) w0 + c(-8, -4, -1, 0, 1, 4, 8) / abs(t)
  brk <- sort(unique(c(0, q, w0, near[!is.na(near) & near > 0], Inf)))
  brk <- brk[!is.na(brk)]
  sum(vapply(seq_len(length(brk) - 1L), function(i) {
    stats::integrate(f, brk[i], brk[i + 1L], rel.tol = 1e-13, abs.tol = 1e-300, subdivisions = 2000L, stop.on.error = FALSE)$value
  }, 0))
}
stata_d_ntcdf <- function(d, np, t, upper) {
  if (length(d) == 0L || length(np) == 0L || length(t) == 0L) return(numeric(0))
  n <- max(length(d), length(np), length(t))
  d <- rep_len(as.numeric(d), n)
  np <- rep_len(as.numeric(np), n)
  t <- stata_d_ntbig(rep_len(as.numeric(t), n))
  r <- stata_d_run(stats::pt(t, d, np, lower.tail = !upper))
  ok <- stata_d_pos(d, 1e10) & !is.na(np) & abs(np) <= 1000
  for (i in which(ok & !is.na(t) & abs(np) > 37.62)) {
    r[i] <- if (is.infinite(t[i])) as.numeric((t[i] > 0) != upper) else stata_d_ntint(d[i], np[i], t[i], upper)
  }
  stata_d_keep(r, ok)
}
stata_d_nt <- function(d, np, t) stata_d_ntcdf(d, np, t, FALSE)
stata_d_nttail <- function(d, np, t) stata_d_ntcdf(d, np, t, TRUE)
stata_d_nchi2ok <- function(d, np) stata_d_pos(d, 1e6) & !is.na(np) & np >= 0 & np <= 1e4
stata_d_nchi2 <- function(d, np, x) stata_d_keep(stata_d_run(stats::pchisq(x, d, np)), stata_d_nchi2ok(d, np))
stata_d_nchi2tail <- function(d, np, x) stata_d_keep(stata_d_run(stats::pchisq(x, d, np, lower.tail = FALSE)), stata_d_nchi2ok(d, np))
# the inverses: invnchi2() is R's qchisq() (to 2e-10), on the domain of nchi2(); p of 0 is 0 and of 1 missing
# (the tail the other way round). invnt() and invnttail() find t by root-finding on nt() (Stata: df 1 to 1e6,
# |np| to 1000, p from 1e-8 to 1 - 1e-8, and for np of 0 the central t, invnt() from p of 7.07e-151)
stata_d_invnchi2 <- function(d, np, p) stata_d_keep(stata_d_run(stats::qchisq(p, d, np)), stata_d_nchi2ok(d, np))
stata_d_invnchi2tail <- function(d, np, p) stata_d_keep(stata_d_run(stats::qchisq(p, d, np, lower.tail = FALSE)), stata_d_nchi2ok(d, np))
stata_d_invnt1 <- function(d, np, p, upper) {
  if (anyNA(c(d, np, p)) || d < 1 || d > 1e6 || abs(np) > 1000 || p <= 0 || p >= 1) return(NA_real_)
  if (np == 0 && !upper) return(if (p < 7.07e-151) NA_real_ else stats::qt(p, d))
  if (p < 1e-8 || p > 1 - 1e-8) return(NA_real_)
  if (np == 0) return(stats::qt(p, d, lower.tail = FALSE))
  x0 <- tryCatch(stata_d_run(stats::qt(p, d, np, lower.tail = !upper)), error = function(e) NA_real_)
  y0 <- if (is.finite(x0)) asinh(x0) else asinh(np)
  g <- function(y) (if (upper) -1 else 1) * (stata_d_ntcdf(d, np, sinh(y), upper) - p)
  lo <- y0 - 1
  hi <- y0 + 1
  for (k in 1:60) if (is.finite(g(lo)) && g(lo) > 0) lo <- lo - 2^(k / 3) else break
  for (k in 1:60) if (is.finite(g(hi)) && g(hi) < 0) hi <- hi + 2^(k / 3) else break
  if (!is.finite(g(lo)) || !is.finite(g(hi)) || g(lo) > 0 || g(hi) < 0) return(NA_real_)
  sinh(stats::uniroot(g, c(lo, hi), tol = 1e-15, maxiter = 1000)$root)
}
stata_d_invnt_ <- function(d, np, p, upper) {
  if (length(d) == 0L || length(np) == 0L || length(p) == 0L) return(numeric(0))
  n <- max(length(d), length(np), length(p))
  d <- rep_len(as.numeric(d), n)
  np <- rep_len(as.numeric(np), n)
  p <- rep_len(as.numeric(p), n)
  vapply(seq_len(n), function(i) stata_d_invnt1(d[i], np[i], p[i], upper), 0)
}
stata_d_invnt <- function(d, np, p) stata_d_invnt_(d, np, p, FALSE)
stata_d_invnttail <- function(d, np, p) stata_d_invnt_(d, np, p, TRUE)

stata_d_nFok <- function(a, b, np, top, hi) stata_d_pos(a, hi) & stata_d_pos(b, hi) & !is.na(np) & np >= 0 & np <= top
# nF() and nFtail() as Stata 19 computes them (fourth round): np of 0 is the central F (R's pf() without ncp,
# since given ncp = 0 it takes its noncentral series, good only to 1e-9); else the Poisson mixture of betas at
# y = a x / (a x + b) taken in double, as Stata takes it (so x near 1e15 loses what Stata loses), and the
# tail as 1 - nF() (Stata's tail below about 1e-15 is that subtraction's rounding). Stata's own nF() below
# about 1e-20 with np of 100 or more sums only part of the series (0 where it should not); not matched
stata_d_nF1 <- function(a, b, np, x, upper) {
  if (anyNA(c(a, b, np, x)) || a <= 0 || b <= 0 || np < 0 || np > 1e4) return(NA_real_)
  if (np == 0) return(stats::pf(x, a, b, lower.tail = !upper))
  if (x <= 0) return(as.numeric(upper))
  y <- if (is.finite(a * x)) a * x / (a * x + b) else 1
  m <- np / 2
  j <- 0:ceiling(m + 12 * sqrt(m) + 40)
  w <- stats::dpois(j, m)
  j <- j[w > 0]
  w <- w[w > 0]
  s <- sum(w * stats::pbeta(y, a / 2 + j, b / 2))
  if (upper) 1 - s else s
}
stata_d_nF <- function(a, b, np, x) {
  stata_d_keep(stata_d_vec(function(a, b, np, x) stata_d_nF1(a, b, np, x, FALSE), a, b, np, x), stata_d_nFok(a, b, np, 1e4, 1e8))
}
stata_d_nFtail <- function(a, b, np, x) {
  stata_d_keep(stata_d_vec(function(a, b, np, x) stata_d_nF1(a, b, np, x, TRUE), a, b, np, x), stata_d_nFok(a, b, np, 1e3, Inf))
}
# the noncentral F's inverses and the noncentrality solvers (third round). Stata's values are good to about
# 1e-14 in the cdf, R's pf() with a noncentrality only to about 1e-9 (its pnbeta series), so the noncentral F
# is summed here as the Poisson mixture of betas, which keeps its digits in both tails
stata_d_root <- function(g, lo, hi) {
  glo <- g(lo)
  ghi <- g(hi)
  if (!is.finite(glo) || !is.finite(ghi) || sign(glo) == sign(ghi)) return(NA_real_)
  if (glo == 0) return(lo)
  if (ghi == 0) return(hi)
  tryCatch(stats::uniroot(g, c(lo, hi), f.lower = glo, f.upper = ghi, tol = 1e-15, maxiter = 2000L)$root,
    error = function(e) NA_real_)
}
stata_d_nFs <- function(a, b, np, x, upper) {
  if (x <= 0) return(as.numeric(upper))
  r <- b / a / x
  if (r == 0) return(as.numeric(!upper))
  y <- if (is.finite(r)) 1 / (1 + r) else 0
  y1 <- if (is.finite(r)) r / (1 + r) else 1
  m <- np / 2
  j <- 0:ceiling(m + 12 * sqrt(m) + 40)
  w <- stats::dpois(j, m)
  j <- j[w > 0]
  w <- w[w > 0]
  sum(w * (if (upper) stats::pbeta(y1, b / 2, a / 2 + j) else stats::pbeta(y, a / 2 + j, b / 2)))
}
# x where the noncentral F's cdf (upper: its tail) is q, found on log x
stata_d_Froot <- function(a, b, np, q, upper) {
  g <- function(lx) {
    v <- stata_d_nFs(a, b, np, exp(lx), upper) - q
    if (upper) -v else v
  }
  x0 <- tryCatch(suppressWarnings(stats::qf(q, a, b, np, lower.tail = !upper)), error = function(e) NA_real_)
  y0 <- if (is.finite(x0) && x0 > 0) log(x0) else 0
  lo <- y0 - 1
  hi <- y0 + 1
  for (k in 1:80) if (lo > -745 && is.finite(g(lo)) && g(lo) > 0) lo <- max(lo - 2^(k / 2), -745) else break
  for (k in 1:80) if (hi < 709 && is.finite(g(hi)) && g(hi) < 0) hi <- min(hi + 2^(k / 2), 709) else break
  exp(stata_d_root(g, lo, hi))
}
# the central F's quantile through the beta, on the side of one half that keeps its digits
stata_d_qF <- function(a, b, p, upper) {
  y <- stats::qbeta(p, a / 2, b / 2, lower.tail = !upper)
  if (y < .5) return(b / a * y / (1 - y))
  z <- stats::qbeta(p, b / 2, a / 2, lower.tail = upper)
  b / a * (1 - z) / z
}
# invnF(): df 1e-6 to 1e6, np 0 to 1e4, p 0 to 1 - 2^-12 (p of 0 is 0), and for np above 0 p from 1e-8; up to
# np 1000 missing past x = b / a 99999972.96 (Stata's search on y = a x / (a x + b) stops there), above it
# b / a 2^52 at most
stata_d_invnF1 <- function(a, b, np, p) {
  if (anyNA(c(a, b, np, p)) || a < 1e-6 || a > 1e6 || b < 1e-6 || b > 1e6 || np < 0 || np > 1e4 ||
    p < 0 || p > 1 - 2^-12) return(NA_real_)
  if (p == 0) return(0)
  if (np == 0) return(stata_d_qF(a, b, p, FALSE))
  if (p < 1e-8) return(NA_real_)
  if (np <= 1000 && p > stata_d_nFs(a, b, np, b / a * 99999972.962401286, FALSE)) return(NA_real_)
  if (np > 1000 && stata_d_nFs(a, b, np, b / a * 2^52, FALSE) < p) return(b / a * 2^52)
  stata_d_Froot(a, b, np, p, FALSE)
}
# invnFtail(): any df, np 0 to 1000, p from 1e-14; p of 1 is 0 whatever a and b are; Stata solves nF() = 1 - p
# (below one half the tail it reaches is 1 - (1 - p)) and stops at x = b / a 2^53
stata_d_invnFtail1 <- function(a, b, np, p) {
  if (is.na(np) || is.na(p) || np > 1000) return(NA_real_)
  if (p == 1) return(0)
  if (is.na(a) || is.na(b) || a <= 0 || b <= 0 || np < 0 || p < 1e-14 || p > 1) return(NA_real_)
  if (np == 0) return(stata_d_qF(a, b, p, TRUE))
  if (stata_d_nFs(a, b, np, b / a * 2^53, TRUE) > p) return(b / a * 2^53)
  if (p <= .5) stata_d_Froot(a, b, np, 1 - (1 - p), TRUE) else stata_d_Froot(a, b, np, 1 - p, FALSE)
}
# np for a cdf that falls as np grows, on 0 .. 1e4: missing above the central cdf, 0 at it (within 1e-14,
# as Stata takes npnF(3, 5, 1, F(3, 5, 1)) and not 1e-12 above it), 1e4 just past it
stata_d_np0 <- function(cdf, c0, p) {
  if (p > c0 + 1e-14) return(NA_real_)
  if (p >= c0 - 1e-14) return(0)
  min(stata_d_root(function(np) cdf(np) - p, 0, 1e4 * (1 + 1e-13)), 1e4)
}
# the noncentral chi-square's cdf as the Poisson mixture of central ones. R's pchisq() with a noncentrality is
# good only to about 1e-12 there, as large as the step the search in npnchi2() looks for at np 1e4 (x = 1e4, df 1,
# p one half), where its sign came out differently from one platform to another
stata_d_ncs <- function(d, x, np) {
  m <- np / 2
  j <- 0:ceiling(m + 12 * sqrt(m) + 40)
  w <- stats::dpois(j, m)
  j <- j[w > 0]
  w <- w[w > 0]
  sum(w * stats::pchisq(x, d + 2 * j))
}
# npnchi2(): df 2e-10 to 1e6, x above 0, p from 2^-30 below 1
stata_d_npnchi21 <- function(d, x, p) {
  if (anyNA(c(d, x, p)) || d < 2e-10 || d > 1e6 || x <= 0 || p < 2^-30 || p >= 1) return(NA_real_)
  stata_d_np0(function(np) stata_d_ncs(d, x, np), stats::pchisq(x, d), p)
}
# npnF(): df1 2e-10 to 1e6, df2 to 1e6, x above 0, p from 1e-8 below 1
stata_d_npnF1 <- function(a, b, x, p) {
  if (anyNA(c(a, b, x, p)) || a < 2e-10 || a > 1e6 || b <= 0 || b > 1e6 || x <= 0 || p < 1e-8 || p >= 1) {
    return(NA_real_)
  }
  stata_d_np0(function(np) stata_d_nFs(a, b, np, x, FALSE), stata_d_nFs(a, b, 0, x, FALSE), p)
}
# npnt(): df to 1e8, p from 1e-8 to 1 - 1e-8, np between -1000 and 1000
stata_d_npnt1 <- function(d, t, p) {
  if (anyNA(c(d, t, p)) || d <= 0 || d > 1e8 || p < 1e-8 || p > 1 - 1e-8) return(NA_real_)
  if (t == 0) return(-stats::qnorm(p))
  stata_d_root(function(np) stata_d_nt(d, np, t) - p, -1000, 1000)
}
stata_d_vec <- function(f, ...) {
  args <- lapply(list(...), as.numeric)
  if (any(lengths(args) == 0L)) return(numeric(0))
  n <- max(lengths(args))
  args <- lapply(args, rep_len, n)
  vapply(seq_len(n), function(i) do.call(f, lapply(args, `[`, i)), 0)
}
stata_d_invnF <- function(a, b, np, p) stata_d_keep(stata_d_vec(stata_d_invnF1, a, b, np, p), TRUE)
stata_d_invnFtail <- function(a, b, np, p) stata_d_keep(stata_d_vec(stata_d_invnFtail1, a, b, np, p), TRUE)
stata_d_npnchi2 <- function(d, x, p) stata_d_keep(stata_d_vec(stata_d_npnchi21, d, x, p), TRUE)
stata_d_npnF <- function(a, b, x, p) stata_d_keep(stata_d_vec(stata_d_npnF1, a, b, x, p), TRUE)
stata_d_npnt <- function(d, t, p) stata_d_keep(stata_d_vec(stata_d_npnt1, d, t, p), TRUE)"##;

impl Translator<'_> {
    /// A distribution function: its R helper on the arguments' numbers.
    pub(super) fn distribution(
        &mut self,
        name: &str,
        args: &[Expr],
    ) -> Result<(String, Kind), Untranslated> {
        let takes = FUNCTIONS
            .iter()
            .find(|(f, _)| *f == name)
            .map_or(&[][..], |(_, n)| *n);
        if !takes.contains(&args.len()) {
            let n = match (takes.first(), takes.last()) {
                (Some(a), Some(b)) if a != b => format!("{a} to {b}"),
                (Some(a), _) => a.to_string(),
                _ => "no".to_string(),
            };
            return Err(no(format!(
                "{name}() takes {n} argument{} (Stata: invalid syntax)",
                if takes == [1] { "" } else { "s" }
            )));
        }
        let mut vals = Vec::new();
        for a in args {
            let (c, k) = self.expr(a)?;
            if k == Kind::Text {
                return Err(no(format!("{name}() of a string (Stata: type mismatch)")));
            }
            vals.push(number(&c, k)?);
        }
        self.helpers.push("stata_dist");
        let suffix = match (name, args.len()) {
            ("normalden" | "lnnormalden", 2) => "_s",
            ("normalden" | "lnnormalden", 3) => "_ms",
            _ => "",
        };
        Ok((
            format!("stata_d_{name}{suffix}({})", vals.join(", ")),
            Kind::Number,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::super::{translate, Use};

    fn none(_: &str) -> Option<super::Kind> {
        None
    }

    #[test]
    fn distribution_functions_call_their_helpers() {
        let code = |t: &str| translate(t, &none, Use::Value).map(|r| (r.code, r.helpers));
        assert_eq!(
            code("2*ttail(df, abs(t))"),
            Ok((
                "(2 * stata_d_ttail(df, abs(t)))".to_string(),
                vec!["stata_dist"]
            ))
        );
        assert_eq!(
            code("normalden(x, 2)").map(|r| r.0),
            Ok("stata_d_normalden_s(x, 2)".to_string())
        );
        assert_eq!(
            code("invFtail(3, 20, .05)").map(|r| r.0),
            Ok("stata_d_invFtail(3, 20, 0.05)".to_string())
        );
        assert_eq!(
            code("nttail(df, 1.5, t)").map(|r| r.0),
            Ok("stata_d_nttail(df, 1.5, t)".to_string())
        );
        assert_eq!(
            code("nF(3, 10, np, x)").map(|r| r.0),
            Ok("stata_d_nF(3, 10, np, x)".to_string())
        );
        assert_eq!(
            code("invnFtail(3, 10, np, .05) + npnt(df, t, .5)").map(|r| r.0),
            Ok("(stata_d_invnFtail(3, 10, np, 0.05) + stata_d_npnt(df, t, 0.5))".to_string())
        );
        assert_eq!(
            code("chiprob(3, 7.81)").map(|r| r.0),
            Ok("stata_d_chiprob(3, 7.81)".to_string())
        );
        let refused = |t: &str| translate(t, &none, Use::Value).err().map(|e| e.reason);
        assert_eq!(
            refused("nt(5, 1)"),
            Some("nt() takes 3 arguments (Stata: invalid syntax)".to_string())
        );
        assert_eq!(
            refused("ttail(5)"),
            Some("ttail() takes 2 arguments (Stata: invalid syntax)".to_string())
        );
        assert_eq!(
            refused("normalden(1, 2, 3, 4)"),
            Some("normalden() takes 1 to 3 arguments (Stata: invalid syntax)".to_string())
        );
        assert_eq!(
            refused("chi2tail(\"a\", 1)"),
            Some("chi2tail() of a string (Stata: type mismatch)".to_string())
        );
    }

    #[test]
    fn the_table_is_sorted_and_has_no_twins() {
        let names: Vec<&str> = super::FUNCTIONS.iter().map(|(f, _)| *f).collect();
        let mut sorted = names.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(names, sorted);
    }
}
