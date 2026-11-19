//! Batch G of the StatCraft port, `xtunitroot`: the panel unit-root tests of
//! Im, Pesaran and Shin (`ips`), Harris and Tzavalis (`ht`), Breitung
//! (`breitung`), the Fisher-type tests (`fisher`) and Hadri (`hadri`)
//! (`plan/stata/xtunitroot.md`). Strongly balanced panels only; Levin, Lin
//! and Chu (`llc`) and the forms noted in the plan are refused.

use super::stats::r_logical;
use super::*;

fn stop(out: &mut Out, msg: &str) {
    out.line(format!("stop({}, call. = FALSE)", r_text(msg)));
}

/// The published tables the tests read: Im, Pesaran and Shin (2003), Table 1
/// (T, then the mean and variance of t-tilde and of t), Table 2 (the exact
/// critical values of t-bar by trend, level, N and T) and Table 3 (the mean
/// and variance of t(p, 0) by trend, p, and T), as the notes carry them.
pub(super) const XTUNITROOT_HELPER: &str = r##"
stata_xtur_t1 <- matrix(c(6.0, -1.125, 0.497, -1.520, 1.745, 7.0, -1.178, 0.506, -1.514, 1.414, 8.0, -1.214, 0.506, -1.501, 1.228, 9.0, -1.244, 0.527, -1.501, 1.132, 10.0, -1.274, 0.521, -1.504, 1.069, 15.0, -1.349, 0.565, -1.514, 0.923, 20.0, -1.395, 0.592, -1.522, 0.851, 25.0, -1.423, 0.609, -1.520, 0.809, 30.0, -1.439, 0.623, -1.526, 0.789, 40.0, -1.463, 0.639, -1.523, 0.770, 50.0, -1.477, 0.656, -1.527, 0.760, 100.0, -1.504, 0.683, -1.532, 0.735, 500.0, -1.526, 0.704, -1.531, 0.715, 1000.0, -1.526, 0.702, -1.529, 0.707, Inf, -1.533, 0.706, -1.533, 0.706), 15, 5, byrow = TRUE)
stata_xtur_t2 <- array(c(-3.79, -2.66, -2.54, -2.50, -2.46, -2.44, -2.43, -2.42, -2.42, -2.40, -2.40, -3.45, -2.47, -2.38, -2.33, -2.32, -2.31, -2.29, -2.28, -2.28, -2.28, -2.27, -3.06, -2.32, -2.24, -2.21, -2.19, -2.18, -2.16, -2.16, -2.16, -2.16, -2.15, -2.79, -2.14, -2.10, -2.08, -2.07, -2.05, -2.04, -2.05, -2.04, -2.04, -2.04, -2.61, -2.06, -2.02, -2.00, -1.99, -1.99, -1.98, -1.98, -1.98, -1.97, -1.97, -2.51, -2.01, -1.97, -1.95, -1.94, -1.94, -1.93, -1.93, -1.93, -1.93, -1.92, -2.20, -1.85, -1.83, -1.82, -1.82, -1.82, -1.81, -1.81, -1.81, -1.81, -1.81, -2.00, -1.75, -1.74, -1.73, -1.73, -1.73, -1.73, -1.73, -1.73, -1.73, -1.73, -2.76, -2.28, -2.21, -2.19, -2.18, -2.16, -2.16, -2.15, -2.16, -2.15, -2.15, -2.57, -2.17, -2.11, -2.09, -2.08, -2.07, -2.07, -2.06, -2.06, -2.06, -2.05, -2.42, -2.06, -2.02, -1.99, -1.99, -1.99, -1.98, -1.98, -1.97, -1.98, -1.97, -2.28, -1.95, -1.92, -1.91, -1.90, -1.90, -1.90, -1.89, -1.89, -1.89, -1.89, -2.18, -1.89, -1.87, -1.86, -1.85, -1.85, -1.85, -1.85, -1.84, -1.84, -1.84, -2.11, -1.85, -1.83, -1.82, -1.82, -1.82, -1.81, -1.81, -1.81, -1.81, -1.81, -1.95, -1.75, -1.74, -1.73, -1.73, -1.73, -1.73, -1.73, -1.73, -1.73, -1.73, -1.84, -1.68, -1.67, -1.67, -1.67, -1.67, -1.67, -1.67, -1.67, -1.67, -1.67, -2.38, -2.10, -2.06, -2.04, -2.04, -2.02, -2.02, -2.02, -2.02, -2.02, -2.01, -2.27, -2.01, -1.98, -1.96, -1.95, -1.95, -1.95, -1.95, -1.94, -1.95, -1.94, -2.17, -1.93, -1.90, -1.89, -1.88, -1.88, -1.88, -1.88, -1.88, -1.88, -1.88, -2.06, -1.85, -1.83, -1.82, -1.82, -1.82, -1.81, -1.81, -1.81, -1.81, -1.81, -2.00, -1.80, -1.79, -1.78, -1.78, -1.78, -1.78, -1.78, -1.78, -1.77, -1.77, -1.96, -1.77, -1.76, -1.75, -1.75, -1.75, -1.75, -1.75, -1.75, -1.75, -1.75, -1.85, -1.70, -1.69, -1.69, -1.69, -1.69, -1.68, -1.68, -1.68, -1.68, -1.69, -1.77, -1.64, -1.64, -1.64, -1.64, -1.64, -1.64, -1.64, -1.64, -1.64, -1.64, -8.12, -3.42, -3.21, -3.13, -3.09, -3.05, -3.03, -3.02, -3.00, -3.00, -2.99, -7.36, -3.20, -3.03, -2.97, -2.94, -2.93, -2.90, -2.88, -2.88, -2.87, -2.86, -6.44, -3.03, -2.88, -2.84, -2.82, -2.79, -2.78, -2.77, -2.76, -2.75, -2.75, -5.72, -2.86, -2.74, -2.71, -2.69, -2.68, -2.67, -2.65, -2.66, -2.65, -2.64, -5.54, -2.75, -2.67, -2.63, -2.62, -2.61, -2.59, -2.60, -2.59, -2.58, -2.58, -5.16, -2.69, -2.61, -2.58, -2.58, -2.56, -2.55, -2.55, -2.55, -2.54, -2.54, -4.50, -2.53, -2.48, -2.46, -2.45, -2.45, -2.44, -2.44, -2.44, -2.44, -2.43, -4.00, -2.42, -2.39, -2.38, -2.37, -2.37, -2.36, -2.36, -2.36, -2.36, -2.36, -4.66, -2.98, -2.87, -2.82, -2.80, -2.79, -2.77, -2.76, -2.75, -2.75, -2.75, -4.38, -2.85, -2.76, -2.72, -2.70, -2.69, -2.68, -2.67, -2.67, -2.66, -2.66, -4.11, -2.74, -2.66, -2.63, -2.62, -2.60, -2.60, -2.59, -2.59, -2.58, -2.58, -3.88, -2.63, -2.57, -2.55, -2.53, -2.53, -2.52, -2.52, -2.52, -2.51, -2.51, -3.73, -2.56, -2.52, -2.49, -2.48, -2.48, -2.48, -2.47, -2.47, -2.46, -2.46, -3.62, -2.52, -2.48, -2.46, -2.45, -2.45, -2.44, -2.44, -2.44, -2.44, -2.43, -3.35, -2.42, -2.38, -2.38, -2.37, -2.37, -2.36, -2.36, -2.36, -2.36, -2.36, -3.13, -2.34, -2.32, -2.32, -2.31, -2.31, -2.31, -2.31, -2.31, -2.31, -2.31, -3.73, -2.77, -2.70, -2.67, -2.65, -2.64, -2.63, -2.62, -2.63, -2.62, -2.62, -3.60, -2.68, -2.62, -2.59, -2.58, -2.57, -2.57, -2.56, -2.56, -2.55, -2.55, -3.45, -2.59, -2.54, -2.52, -2.51, -2.51, -2.50, -2.50, -2.50, -2.49, -2.49, -3.33, -2.52, -2.47, -2.46, -2.45, -2.45, -2.44, -2.44, -2.44, -2.44, -2.44, -3.26, -2.47, -2.44, -2.42, -2.41, -2.41, -2.41, -2.40, -2.40, -2.40, -2.40, -3.18, -2.44, -2.40, -2.39, -2.39, -2.38, -2.38, -2.38, -2.38, -2.38, -2.38, -3.02, -2.36, -2.33, -2.33, -2.33, -2.32, -2.32, -2.32, -2.32, -2.32, -2.32, -2.90, -2.30, -2.29, -2.28, -2.28, -2.28, -2.28, -2.28, -2.28, -2.28, -2.28), dim = c(11, 8, 3, 2))
stata_xtur_t3 <- array(c(-1.504, -1.514, -1.522, -1.520, -1.526, -1.523, -1.527, -1.519, -1.524, -1.532, 1.069, 0.923, 0.851, 0.809, 0.789, 0.770, 0.760, 0.749, 0.736, 0.735, -1.488, -1.503, -1.516, -1.514, -1.519, -1.520, -1.524, -1.519, -1.522, -1.530, 1.255, 1.011, 0.915, 0.861, 0.831, 0.803, 0.781, 0.770, 0.753, 0.745, -1.319, -1.387, -1.428, -1.443, -1.460, -1.476, -1.493, -1.490, -1.498, -1.514, 1.421, 1.078, 0.969, 0.905, 0.865, 0.830, 0.798, 0.789, 0.766, 0.754, -1.306, -1.366, -1.413, -1.433, -1.453, -1.471, -1.489, -1.486, -1.495, -1.512, 1.759, 1.181, 1.037, 0.952, 0.907, 0.858, 0.819, 0.802, 0.782, 0.761, -1.171, -1.260, -1.329, -1.363, -1.394, -1.428, -1.454, -1.458, -1.470, -1.495, 2.080, 1.279, 1.097, 1.005, 0.946, 0.886, 0.842, 0.819, 0.801, 0.771, NA, NA, -1.313, -1.351, -1.384, -1.421, -1.451, -1.454, -1.467, -1.494, NA, NA, 1.171, 1.055, 0.980, 0.912, 0.863, 0.839, 0.814, 0.781, NA, NA, NA, -1.289, -1.331, -1.380, -1.418, -1.427, -1.444, -1.476, NA, NA, NA, 1.114, 1.023, 0.942, 0.886, 0.858, 0.834, 0.795, NA, NA, NA, -1.273, -1.319, -1.371, -1.411, -1.423, -1.441, -1.474, NA, NA, NA, 1.164, 1.062, 0.968, 0.910, 0.875, 0.851, 0.806, NA, NA, NA, -1.212, -1.266, -1.329, -1.377, -1.393, -1.415, -1.456, NA, NA, NA, 1.217, 1.105, 0.996, 0.929, 0.896, 0.871, 0.818, -2.166, -2.167, -2.168, -2.167, -2.172, -2.173, -2.176, -2.174, -2.174, -2.177, 1.132, 0.869, 0.763, 0.713, 0.690, 0.655, 0.633, 0.621, 0.610, 0.597, -2.173, -2.169, -2.172, -2.172, -2.173, -2.177, -2.180, -2.178, -2.176, -2.179, 1.453, 0.975, 0.845, 0.769, 0.734, 0.687, 0.654, 0.641, 0.627, 0.605, -1.914, -1.999, -2.047, -2.074, -2.095, -2.120, -2.137, -2.143, -2.146, -2.158, 1.627, 1.036, 0.882, 0.796, 0.756, 0.702, 0.661, 0.653, 0.634, 0.613, -1.922, -1.977, -2.032, -2.065, -2.091, -2.117, -2.137, -2.142, -2.146, -2.158, 2.482, 1.214, 0.983, 0.861, 0.808, 0.735, 0.688, 0.674, 0.650, 0.625, -1.750, -1.823, -1.911, -1.968, -2.009, -2.057, -2.091, -2.103, -2.114, -2.135, 3.947, 1.332, 1.052, 0.913, 0.845, 0.759, 0.705, 0.685, 0.662, 0.629, NA, NA, -1.888, -1.955, -1.998, -2.051, -2.087, -2.101, -2.111, -2.135, NA, NA, 1.165, 0.991, 0.899, 0.792, 0.730, 0.705, 0.673, 0.638, NA, NA, NA, -1.868, -1.923, -1.995, -2.042, -2.065, -2.081, -2.113, NA, NA, NA, 1.055, 0.945, 0.828, 0.753, 0.725, 0.689, 0.650, NA, NA, NA, -1.851, -1.912, -1.986, -2.036, -2.063, -2.079, -2.112, NA, NA, NA, 1.145, 1.009, 0.872, 0.786, 0.747, 0.713, 0.661, NA, NA, NA, -1.761, -1.835, -1.925, -1.987, -2.024, -2.046, -2.088, NA, NA, NA, 1.208, 1.063, 0.902, 0.808, 0.766, 0.728, 0.670), dim = c(10, 2, 9, 2))

stata_xtur_interp <- function(xs, ys, x) {
  ok <- !is.na(ys)
  xs <- xs[ok]
  ys <- ys[ok]
  if (!length(xs)) return(NA_real_)
  if (x <= xs[1]) return(ys[1])
  for (i in seq_len(length(xs) - 1)) {
    a <- xs[i]
    b <- xs[i + 1]
    if (x <= b) {
      if (is.infinite(b)) return(ys[i])
      return(ys[i] + (x - a) / (b - a) * (ys[i + 1] - ys[i]))
    }
  }
  ys[length(ys)]
}
stata_xtur_above <- function(knots, x) {
  i <- which(knots >= x)
  if (length(i)) i[1] else length(knots)
}
stata_xtur_resid <- function(X, y) {
  if (is.null(X) || ncol(X) == 0) return(y)
  as.vector(qr.resid(qr(X), y))
}
stata_xtur_det <- function(n, det, start) {
  switch(det, n = matrix(0, n, 0), c = matrix(1, n, 1), t = cbind(1, start + seq_len(n) - 1))
}
# the Dickey-Fuller regression of one panel with p lags: tau, and the IPS t-tilde
stata_xtur_adf <- function(full, win, p, trend) {
  tt <- win[win - p - 1 >= 1]
  ylag <- full[tt - 1]
  dy <- full[tt] - full[tt - 1]
  X <- cbind(ylag)
  for (j in seq_len(p)) X <- cbind(X, full[tt - j] - full[tt - j - 1])
  if (trend) X <- cbind(X, tt)
  X <- cbind(X, 1)
  ok <- stats::complete.cases(X, dy)
  X <- X[ok, , drop = FALSE]
  dy <- dy[ok]
  ylag <- ylag[ok]
  if (length(dy) <= ncol(X)) return(NULL)
  det <- X[, -1, drop = FALSE]
  my <- stata_xtur_resid(det, ylag)
  mdy <- stata_xtur_resid(det, dy)
  n <- length(dy)
  k <- ncol(X)
  syy <- sum(my^2)
  b <- sum(mdy * my) / syy
  ssr <- sum((mdy - b * my)^2)
  tau <- b / sqrt(ssr / (n - k) / syy)
  s2 <- sum((dy - mean(dy))^2) / (n - 1)
  tilde <- b * sqrt(syy) / sqrt(s2)
  if (!is.finite(tau) || !is.finite(tilde)) return(NULL)
  c(tau, tilde, n)
}
stata_xtur_head <- function(title, rule, h, n, tt) {
  cat("\n")
  for (l in title) cat(l, "\n", sep = "")
  cat(strrep("-", rule), "\n", sep = "")
  cat(stata_tst_left(h[1], 44), "Number of panels  = ", sprintf("%6d", n), "\n", sep = "")
  cat(stata_tst_left(h[2], 44), "Number of periods = ", sprintf("%6d", tt), "\n", sep = "")
  cat("\n")
}
stata_xtur_two <- function(left, right) {
  if (!nzchar(right)) cat(left, "\n", sep = "") else cat(stata_tst_left(left, 44), right, "\n", sep = "")
}
stata_xtur_lags <- function(p, none) if (p == 0) none else if (p == 1) "1 lag" else paste0(p, " lags")
stata_xtur_inc <- function(on) if (on) "Included" else "Not included"
stata_xtur_name <- c(n = "noconstant", c = "constant", t = "trend")
stata_xtunitroot <- function(d, test, y, panel, time, delta = 1, keep = rep(TRUE, nrow(d)), det = "c", lags = NULL,
                             demean = FALSE, altt = FALSE, drift = FALSE, robust = FALSE, kernel = NULL,
                             ur = "dfuller", show = TRUE) {
  if (!(y %in% names(d))) stop("variable ", y, " not found (Stata: r(111))", call. = FALSE)
  if (is.character(d[[y]]) || is.factor(d[[y]])) stop("type mismatch (Stata: r(109))", call. = FALSE)
  pa <- d[[panel]]
  ta <- stata_xt_num(d[[time]])
  ya <- stata_xt_num(d[[y]])
  sel <- keep & !is.na(pa) & !is.na(ta)
  if (!any(sel)) stop("no observations (Stata: r(2000))", call. = FALSE)
  pv <- pa[sel]
  tv <- ta[sel]
  yv <- ya[sel]
  o <- order(pv, tv)
  pv <- pv[o]
  tv <- tv[o]
  yv <- yv[o]
  ids <- unique(pv)
  g <- match(pv, ids)
  nt <- tabulate(g, length(ids))
  bad <- function() stop("xtunitroot ", test, " with an unbalanced panel or a gap in the series is not translated (Stata: strongly balanced data only)", call. = FALSE)
  if (length(unique(nt)) != 1 || anyNA(yv)) bad()
  tt <- nt[1]
  Y <- matrix(yv, nrow = tt)
  Tm <- matrix(tv, nrow = tt)
  if (any(Tm != Tm[, 1]) || any(abs(diff(Tm[, 1]) - delta) > 1e-9)) bad()
  n <- ncol(Y)
  if (tt < 4) stop("fewer than 4 periods: not translated (Stata: r(2001))", call. = FALSE)
  # the series on the grid of times, with the rows before the sample that the lags read; a
  # demeaned series exists only inside the sample, where its cross-sectional means are worked out
  tmin <- min(ta[!is.na(ta) & !is.na(pa) & pa %in% ids])
  s1 <- as.integer(round((Tm[1, 1] - tmin) / delta)) + 1L
  span <- s1 + tt - 1L
  win <- s1:span
  full <- matrix(NA_real_, span, n)
  if (!demean) {
    for (i in seq_len(n)) {
      rows <- which(!is.na(pa) & !is.na(ta) & pa == ids[i] & ta <= Tm[tt, 1])
      full[as.integer(round((ta[rows] - tmin) / delta)) + 1L, i] <- ya[rows]
    }
  }
  if (demean) Y <- Y - rowMeans(Y)
  full[win, ] <- Y
  pre <- s1 > 1 && any(!is.na(full[seq_len(s1 - 1L), ]))
  refuse <- function(what) stop("xtunitroot ", test, ": ", what, " is not translated yet (the result in Stata is not decoded)", call. = FALSE)
  if (test == "ht" && pre && det == "t") refuse("a sample that starts after the data do, with a trend,")
  if (test == "fisher" && ur == "pperron" && (det == "t" || demean || pre)) refuse("the Phillips-Perron form with a trend, demean, or a sample that starts after the data do,")
  cs <- if (demean) "Cross-sectional means removed" else ""
  r <- list()
  m <- list()
  rule <- function() cat(strrep("-", 78), "\n", sep = "")
  dn <- stata_xtur_name[[det]]
  if (test == "ips") {
    p <- if (is.null(lags)) 0 else lags
    lagged <- !is.null(lags)
    trend <- det == "t"
    fits <- lapply(seq_len(n), function(i) stata_xtur_adf(full[, i], win, p, trend))
    if (any(vapply(fits, is.null, TRUE))) stop("insufficient observations (Stata: r(2001))", call. = FALSE)
    tau <- vapply(fits, function(f) f[1], 0)
    til <- vapply(fits, function(f) f[2], 0)
    nobs <- sum(vapply(fits, function(f) f[3], 0))
    tbar <- mean(tau)
    ttbar <- mean(til)
    t1 <- stata_xtur_t1
    zt_tilde <- sqrt(n) * (ttbar - stata_xtur_interp(t1[, 1], t1[, 2], tt)) / sqrt(stata_xtur_interp(t1[, 1], t1[, 3], tt))
    zt <- sqrt(n) * (tbar - stata_xtur_interp(t1[, 1], t1[, 4], tt)) / sqrt(stata_xtur_interp(t1[, 1], t1[, 5], tt))
    if (show) {
      stata_xtur_head(paste0("Im–Pesaran–Shin unit-root test for ", y), 34 + nchar(y) + 1,
                      c("H0: All panels contain unit roots", "Ha: Some panels are stationary"), n, tt)
      stata_xtur_two("AR parameter: Panel-specific", "Asymptotics: T,N -> Infinity")
      cat(stata_tst_left("Panel means:  Included", 62), "sequentially\n", sep = "")
      stata_xtur_two(paste0("Time trend:   ", stata_xtur_inc(trend)), cs)
      cat("\nADF regressions: ", stata_xtur_lags(p, "No lags included"), "\n", sep = "")
      rule()
    }
    r$N <- nobs
    r$N_t <- tt
    r$N_g <- n
    if (!lagged) {
      cv <- vapply(1:3, function(l) stata_xtur_t2[stata_xtur_above(c(5, 10, 15, 20, 25, 30, 40, 50, 60, 70, 100), tt),
                                                  stata_xtur_above(c(5, 7, 10, 15, 20, 25, 50, 100), n), l, as.integer(trend) + 1], 0)
      pz <- stats::pnorm(zt_tilde)
      if (show) {
        cat(stata_tst_pad("Fixed-N exact critical values", 75), "\n", sep = "")
        cat("                    Statistic      p-value         1%      5%      10%\n")
        rule()
        cat(" ", stata_tst_left("t-bar", 15), stata_tst_pad(sprintf("%.4f", tbar), 12), stata_tst_pad(sprintf("%.3f", cv[1]), 27),
            stata_tst_pad(sprintf("%.3f", cv[2]), 8), stata_tst_pad(sprintf("%.3f", cv[3]), 8), "\n", sep = "")
        cat(" ", stata_tst_left("t-tilde-bar", 15), stata_tst_pad(sprintf("%.4f", ttbar), 12), "\n", sep = "")
        cat(" ", stata_tst_left("Z-t-tilde-bar", 15), stata_tst_pad(sprintf("%.4f", zt_tilde), 12), stata_tst_pad(sprintf("%.4f", pz), 14), "\n", sep = "")
        rule()
      }
      r$p_zttildebar <- pz
      r$cv_10 <- cv[3]
      r$cv_5 <- cv[2]
      r$cv_1 <- cv[1]
      r$zttildebar <- zt_tilde
      r$zt <- zt
      r$ttildebar <- ttbar
      r$tbar <- tbar
    } else {
      tk <- c(10, 15, 20, 25, 30, 40, 50, 60, 70, 100)
      mm <- stata_xtur_interp(tk, stata_xtur_t3[, 1, p + 1, as.integer(trend) + 1], tt)
      vv <- stata_xtur_interp(tk, stata_xtur_t3[, 2, p + 1, as.integer(trend) + 1], tt)
      w <- sqrt(n) * (tbar - mm) / sqrt(vv)
      pw <- stats::pnorm(w)
      if (show) {
        cat("                    Statistic      p-value\n")
        rule()
        cat(" ", stata_tst_left("W-t-bar", 15), stata_tst_pad(sprintf("%.4f", w), 12), stata_tst_pad(sprintf("%.4f", pw), 14), "\n", sep = "")
        rule()
      }
      r$p_wtbar <- pw
      r$wtbar <- w
      r$lags <- p
    }
    m$test <- "ips"
  } else if (test == "ht") {
    num <- 0
    den <- 0
    for (i in seq_len(n)) {
      rows <- win[win >= 2]
      cur <- full[rows, i]
      lag <- full[rows - 1, i]
      okc <- !is.na(cur) & !is.na(lag)
      cur <- cur[okc]
      lag <- lag[okc]
      dm <- stata_xtur_det(length(cur), det, 2)
      a <- stata_xtur_resid(dm, cur)
      cc <- stata_xtur_resid(dm, lag)
      num <- num + sum(a * cc)
      den <- den + sum(cc^2)
    }
    rho <- num / den
    ta <- tt - if (altt) 1 else 0
    mv <- switch(det,
      n = c(1, 2 / (ta * (ta - 1))),
      c = c(1 - 3 / (ta + 1), 3 * (17 * ta^2 - 20 * ta + 17) / (5 * (ta - 1) * (ta + 1)^3)),
      t = c(1 - 15 / (2 * (ta + 2)), 15 * (193 * ta^2 - 728 * ta + 1147) / (112 * (ta + 2)^3 * (ta - 2))))
    z <- sqrt(n) * (rho - mv[1]) / sqrt(mv[2])
    pz <- stats::pnorm(z)
    if (show) {
      stata_xtur_head(paste0("Harris-Tzavalis unit-root test for ", y), 34 + nchar(y) + 1,
                      c("H0: Panels contain unit roots", "Ha: Panels are stationary"), n, tt)
      stata_xtur_two("AR parameter: Common", "Asymptotics: N -> Infinity")
      cat(stata_tst_left(paste0("Panel means:  ", stata_xtur_inc(det != "n")), 57), "T Fixed\n", sep = "")
      stata_xtur_two(paste0("Time trend:   ", stata_xtur_inc(det == "t")), cs)
      if (altt) cat("Small-sample adjustment to T applied\n")
      rule()
      cat("                    Statistic         z         p-value\n")
      rule()
      cat(" ", stata_tst_left("rho", 15), stata_tst_pad(sprintf("%.4f", rho), 12), stata_tst_pad(sprintf("%.4f", z), 14),
          stata_tst_pad(sprintf("%.4f", pz), 13), "\n", sep = "")
      rule()
    }
    r$N <- n * tt
    r$N_t <- tt
    r$N_g <- n
    r$p <- pz
    r$z <- z
    r$rho <- rho
    m$test <- "ht"
  } else if (test == "breitung") {
    sxy <- 0
    sxx <- 0
    for (i in seq_len(n)) {
      s <- Y[, i]
      tn <- length(s)
      dy <- diff(s)
      nd <- length(dy)
      first <- s[1]
      last <- s[tn]
      mu <- if (det == "t") mean(dy) else 0
      s2 <- sum((dy - mu)^2) / (nd - 1)
      if (det == "t") {
        for (j in seq_len(nd - 1) - 1) {
          rest <- dy[(j + 2):nd]
          mm <- length(rest)
          v <- sqrt(mm / (mm + 1)) * (dy[j + 1] - mean(rest))
          x <- s[j + 1] - first - j / (tn - 1) * (last - first)
          sxy <- sxy + v * x / s2
          sxx <- sxx + x * x / s2
        }
      } else {
        base <- if (det == "c") first else 0
        for (j in seq_len(nd)) {
          x <- s[j] - base
          sxy <- sxy + dy[j] * x / s2
          sxx <- sxx + x * x / s2
        }
      }
    }
    lambda <- sxy / sqrt(sxx)
    pl <- stats::pnorm(lambda)
    if (show) {
      stata_xtur_head(paste0("Breitung unit-root test for ", y), 28 + nchar(y) + 2,
                      c("H0: Panels contain unit roots", "Ha: Panels are stationary"), n, tt)
      stata_xtur_two("AR parameter: Common", "Asymptotics:  T,N -> Infinity")
      cat(stata_tst_left(paste0("Panel means:  ", stata_xtur_inc(det != "n")), 62), "sequentially\n", sep = "")
      stata_xtur_two(paste0("Time trend:   ", stata_xtur_inc(det == "t")), "Prewhitening: Not performed")
      if (demean) stata_xtur_two("", "Cross-sectional means removed")
      rule()
      cat("                    Statistic      p-value\n")
      rule()
      cat(" ", stata_tst_left("lambda", 15), stata_tst_pad(sprintf("%.4f", lambda), 12), stata_tst_pad(sprintf("%.4f", pl), 14), "\n", sep = "")
      rule()
    }
    r$lags <- 0
    r$p <- pl
    r$lambda <- lambda
    r$N_t <- tt
    r$N_g <- n
    r$N <- n * tt
    m$test <- "breitung"
  } else if (test == "fisher") {
    p <- lags
    pp <- ur == "pperron"
    pvals <- vapply(seq_len(n), function(i) {
      di <- data.frame(t = seq_len(span), y = full[, i])
      kp <- seq_len(span) %in% win
      if (pp) {
        di$y[!kp] <- NA_real_
        stata_pperron(di, "y", "t", NULL, 1, kp, lags = p, det = det, show = FALSE)$pval
      } else {
        stata_dfuller(di, "y", "t", NULL, 1, kp, lags = p, det = det, drift = drift, show = FALSE)$p
      }
    }, 0)
    if (any(pvals <= 0 | pvals >= 1)) stop("xtunitroot fisher with a panel whose p-value is 0 or 1 is not translated yet (Stata drops the panel from some of the statistics)", call. = FALSE)
    pstat <- -2 * sum(log(pvals))
    z <- sum(stats::qnorm(pvals)) / sqrt(n)
    kk <- 3 * (5 * n + 3) / (pi^2 * n * (5 * n + 2))
    lstat <- sqrt(kk) * sum(log(pvals / (1 - pvals)))
    ldf <- 5 * n + 4
    pm <- -sum(log(pvals) + 1) / sqrt(n)
    p_p <- stats::pchisq(pstat, 2 * n, lower.tail = FALSE)
    p_z <- stats::pnorm(z)
    p_l <- stats::pt(lstat, ldf)
    p_pm <- stats::pnorm(pm, lower.tail = FALSE)
    opts <- paste(c(if (det == "t") "trend", if (drift) "drift", paste0("lags(", p, ")")), collapse = " ")
    if (show) {
      stata_xtur_head(c(paste0("Fisher-type unit-root test for ", y),
                        if (pp) "Based on Phillips–Perron tests" else "Based on augmented Dickey–Fuller tests"),
                      if (pp) 32 else 38, c("H0: All panels contain unit roots", "Ha: At least one panel is stationary"), n, tt)
      if (pp) {
        stata_xtur_two("AR parameter:    Panel-specific", "Asymptotics: T -> Infinity")
        cat("Panel means:     Included\n")
        stata_xtur_two(paste0("Time trend:      ", stata_xtur_inc(det == "t")), cs)
        cat("Newey–West lags: ", stata_xtur_lags(p, "0 lags"), "\n", sep = "")
      } else {
        stata_xtur_two("AR parameter: Panel-specific", "Asymptotics: T -> Infinity")
        cat("Panel means:  Included\n")
        stata_xtur_two(paste0("Time trend:   ", stata_xtur_inc(det == "t")), cs)
        stata_xtur_two(paste0("Drift term:   ", stata_xtur_inc(drift)), paste0("ADF regressions: ", stata_xtur_lags(p, "0 lags")))
      }
      rule()
      cat("                                  Statistic      p-value\n")
      rule()
      line <- function(name, sym, v, pr) {
        cat(" ", stata_tst_left(name, 26), stata_tst_left(sym, 2), stata_tst_pad(sprintf("%.4f", v), 14), stata_tst_pad(sprintf("%.4f", pr), 13), "\n", sep = "")
      }
      line(paste0("Inverse chi-squared(", 2 * n, ")"), "P", pstat, p_p)
      line("Inverse normal", "Z", z, p_z)
      line(paste0("Inverse logit t(", ldf, ")"), "L*", lstat, p_l)
      line("Modified inv. chi-squared", "Pm", pm, p_pm)
      rule()
      cat(" P statistic requires number of panels to be finite.\n")
      cat(" Other statistics are suitable for finite or infinite number of panels.\n")
      rule()
    }
    r$N <- n * tt
    r$N_t <- tt
    r$N_g <- n
    r$p_Pm <- p_pm
    r$Pm <- pm
    r$df_L <- ldf
    r$p_L <- p_l
    r$L <- lstat
    r$p_Z <- p_z
    r$Z <- z
    r$df_P <- 2 * n
    r$p_P <- p_p
    r$P <- pstat
    m$options <- opts
    m$urtest <- ur
    m$test <- "fisher"
  } else {
    ks <- if (is.null(kernel)) NULL else kernel
    parts <- vapply(seq_len(n), function(i) {
      s <- Y[, i]
      e <- stata_xtur_resid(stata_xtur_det(length(s), det, 1), s)
      ss <- sum(cumsum(e)^2)
      sse <- sum(e^2)
      s2 <- sse / (tt - if (det == "t") 2 else 1)
      lr <- sse / tt
      if (!is.null(ks)) {
        l <- ks$lags
        for (j in seq_len(l)) {
          gm <- sum(e[(j + 1):tt] * e[1:(tt - j)]) / tt
          x <- j / (l + 1)
          w <- if (ks$name == "bartlett") 1 - x else if (x <= 0.5) 1 - 6 * x^2 + 6 * x^3 else 2 * (1 - x)^3
          lr <- lr + 2 * w * gm
        }
      }
      c(ss / tt^2, s2, lr)
    }, numeric(3))
    lm <- if (!is.null(ks)) sum(parts[1, ]) / sum(parts[3, ]) else if (robust) mean(parts[1, ] / parts[2, ]) else sum(parts[1, ]) / sum(parts[2, ])
    xi <- if (det == "t") 1 / 15 else 1 / 6
    zeta2 <- if (det == "t") 11 / 6300 else 1 / 45
    z <- sqrt(n) * (lm - xi) / sqrt(zeta2)
    pz <- stats::pnorm(z, lower.tail = FALSE)
    if (show) {
      stata_xtur_head(paste0("Hadri LM test for ", y), 18 + nchar(y) + 2,
                      c("H0: All panels are stationary", "Ha: Some panels contain unit roots"), n, tt)
      stata_xtur_two(paste0("Time trend:         ", stata_xtur_inc(det == "t")), "Asymptotics: T, N -> Infinity")
      cat(stata_tst_left(paste0("Heteroskedasticity: ", if (robust || !is.null(ks)) "Robust" else "Not robust"), 62), "sequentially\n", sep = "")
      lrv <- if (is.null(ks)) "(not used)" else paste0(if (ks$name == "bartlett") "Bartlett" else "Parzen", " kernel, ", stata_xtur_lags(ks$lags, "0 lags"))
      stata_xtur_two(paste0("LR variance:        ", lrv), cs)
      rule()
      cat("                    Statistic      p-value\n")
      rule()
      cat(" ", stata_tst_left("z", 15), stata_tst_pad(sprintf("%.4f", z), 12), stata_tst_pad(sprintf("%.4f", pz), 14), "\n", sep = "")
      rule()
    }
    r$N <- n * tt
    r$N_t <- tt
    r$N_g <- n
    r$p <- pz
    r$z <- z
    m$test <- "hadri"
  }
  if (demean) m$demean <- "demean"
  if (test %in% c("ips", "ht", "breitung", "hadri")) m$deterministics <- dn
  invisible(c(r, m))
}"##;

const R_XTUNITROOT: &[&str] = &[
    "N",
    "N_t",
    "N_g",
    "p_zttildebar",
    "cv_10",
    "cv_5",
    "cv_1",
    "zttildebar",
    "zt",
    "ttildebar",
    "tbar",
    "p_wtbar",
    "wtbar",
    "lags",
    "p",
    "z",
    "rho",
    "mean",
    "Var",
    "lambda",
    "p_Pm",
    "Pm",
    "df_L",
    "p_L",
    "L",
    "p_Z",
    "Z",
    "df_P",
    "p_P",
    "P",
    "options",
    "urtest",
    "test",
    "demean",
    "deterministics",
];

/// The names `xtunitroot` leaves as text.
pub(super) fn text_name(name: &str) -> bool {
    matches!(
        name,
        "options" | "urtest" | "test" | "demean" | "deterministics"
    )
}

impl Session {
    /// `xtunitroot test varname [if] [in] [, options]`.
    pub(super) fn xtunitroot(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "xtunitroot")?;
        let words: Vec<&str> = c.parts.main.split_whitespace().collect();
        let Some((test, rest)) = words.split_first() else {
            stop(out, "invalid syntax (Stata: r(198))");
            return Ok(());
        };
        let allowed: &[(&str, usize)] = match *test {
            "ips" => &[("trend", 5), ("demean", 6), ("lags", 1), ("noconstant", 4)],
            "ht" => &[
                ("noconstant", 4),
                ("trend", 5),
                ("demean", 6),
                ("altt", 4),
            ],
            "breitung" => &[
                ("noconstant", 4),
                ("trend", 5),
                ("demean", 6),
                ("robust", 6),
                ("lags", 1),
            ],
            "fisher" => &[
                ("dfuller", 2),
                ("pperron", 2),
                ("lags", 1),
                ("trend", 5),
                ("drift", 5),
                ("demean", 6),
                ("noconstant", 4),
            ],
            "hadri" => &[
                ("noconstant", 4),
                ("trend", 5),
                ("demean", 6),
                ("robust", 6),
                ("kernel", 6),
            ],
            "llc" => {
                return Err(no(
                    "xtunitroot llc is not translated yet: Levin, Lin and Chu's adjustment table (their Table 2) and its bandwidth rule are not decoded here",
                ))
            }
            _ => {
                stop(out, "invalid syntax (Stata: r(198))");
                return Ok(());
            }
        };
        if let Some(o) = opts.other(allowed) {
            stop(out, &format!("option {o} not allowed (Stata: r(198))"));
            return Ok(());
        }
        let var = match rest {
            [] => {
                stop(out, "varlist required (Stata: r(100))");
                return Ok(());
            }
            [v] => v.to_string(),
            _ => {
                stop(out, "too many variables specified (Stata: r(103))");
                return Ok(());
            }
        };
        if !is_name(&var) {
            return Err(no(format!(
                "xtunitroot {test} {var}: only a plain variable name is translated"
            )));
        }
        let noconstant = opts.has("noconstant", 4);
        let trend = opts.has("trend", 5);
        if noconstant && trend {
            stop(
                out,
                "options noconstant and trend may not be combined (Stata: r(198))",
            );
            return Ok(());
        }
        if noconstant && matches!(*test, "ips" | "hadri" | "fisher") {
            if *test == "fisher" {
                return Err(no(
                    "xtunitroot fisher, noconstant is not translated yet (Stata's result with no constant is not decoded)",
                ));
            }
            stop(out, "option noconstant not allowed (Stata: r(198))");
            return Ok(());
        }
        let det = if noconstant {
            "n"
        } else if trend {
            "t"
        } else {
            "c"
        };
        let lags = match opts.arg("lags", 1) {
            None => None,
            Some(t) => match t.trim().parse::<i64>() {
                Ok(v) if v >= 0 => Some(v),
                Ok(_) => {
                    stop(out, "option lags() incorrectly specified (Stata: r(198))");
                    return Ok(());
                }
                Err(_) => {
                    return Err(no(format!(
                        "xtunitroot {test}, lags({}): only a whole number of lags is translated",
                        t.trim()
                    )))
                }
            },
        };
        if *test == "breitung" && (opts.has("robust", 6) || lags.is_some_and(|l| l > 0)) {
            return Err(no(
                "xtunitroot breitung with robust or lags() is not translated yet (the cross-section robust statistic and the prewhitening are not decoded)",
            ));
        }
        if *test == "ips" && lags.is_some_and(|l| l > 8) {
            return Err(no(
                "xtunitroot ips with more than 8 lags is not translated (Stata's table stops at 8)",
            ));
        }
        let ur = if opts.has("pperron", 2) {
            "pperron"
        } else {
            "dfuller"
        };
        if *test == "fisher" {
            if opts.has("pperron", 2) && opts.has("dfuller", 2) {
                stop(
                    out,
                    "options dfuller and pperron may not be combined (Stata: r(198))",
                );
                return Ok(());
            }
            if !opts.has("pperron", 2) && !opts.has("dfuller", 2) {
                stop(
                    out,
                    "must specify either dfuller or pperron (Stata: r(198))",
                );
                return Ok(());
            }
            if lags.is_none() {
                stop(out, "option lags() required (Stata: r(198))");
                return Ok(());
            }
            if opts.has("drift", 5) && (trend || ur == "pperron") {
                stop(
                    out,
                    "options trend and drift may not be combined (Stata: r(198))",
                );
                return Ok(());
            }
        }
        let kernel = match opts.arg("kernel", 6) {
            None => "NULL".to_string(),
            Some(k) => {
                let w: Vec<&str> = k.split_whitespace().collect();
                match w.as_slice() {
                    [name @ ("bartlett" | "parzen"), n] if n.parse::<u32>().is_ok() => {
                        format!("list(name = {}, lags = {n})", r_text(name))
                    }
                    _ => return Err(no(format!(
                        "xtunitroot hadri, kernel({}): only bartlett # and parzen # are translated",
                        k.trim()
                    ))),
                }
            }
        };
        let Some((panel, Some(time))) = self.panel_state(
            true,
            out,
            "must specify panelvar and timevar; use xtset (Stata: r(459))",
            "must specify timevar; use xtset (Stata: r(459))",
            459,
        )?
        else {
            return Ok(());
        };
        let Some(panel) = panel else {
            stop(out, "must specify panelvar; use xtset (Stata: r(459))");
            return Ok(());
        };
        let d = self.data()?;
        let cond = self.condition(c, out)?;
        let keep = match cond {
            Some(cond) => {
                out.package("dplyr");
                format!(
                    "seq_len(nrow(stata_d)) %in% dplyr::filter(dplyr::mutate(stata_d, stata_obs_ = dplyr::row_number()), {cond})$stata_obs_"
                )
            }
            None => "rep(TRUE, nrow(stata_d))".to_string(),
        };
        for h in [
            "stata_xtunitroot",
            "stata_tst_common",
            "stata_dfuller",
            "stata_pperron",
            "stata_xt_common",
            "stata_fmt",
            "stata_tables",
        ] {
            self.helpers.insert(h);
        }
        let show = if self.quiet(c) { ", show = FALSE" } else { "" };
        out.line(format!(
            "stata_r <- local({{ stata_d <- {d}; stata_xtunitroot(stata_d, {}, {}, {}, {}, delta = {}, keep = {keep}, det = {}, lags = {}, demean = {}, altt = {}, drift = {}, robust = {}, kernel = {kernel}, ur = {}{show}) }})",
            r_text(test),
            r_text(&var),
            r_text(&panel),
            r_text(&time),
            self.ts_delta.clone().unwrap_or_else(|| "1".to_string()),
            r_text(det),
            lags.map_or("NULL".to_string(), |l| l.to_string()),
            r_logical(opts.has("demean", 6)),
            r_logical(opts.has("altt", 4)),
            r_logical(opts.has("drift", 5)),
            r_logical(opts.has("robust", 6)),
            r_text(ur),
        ));
        self.r_new = Some((R_XTUNITROOT, "xtunitroot".into()));
        Ok(())
    }
}
