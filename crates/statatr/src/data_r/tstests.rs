//! Batch G of the StatCraft port, the time-series tests as Stata 19 does them:
//! `dfuller`, `pperron`, `dfgls`, `corrgram` and `wntestq` (`plan/stata/tstests.md`).
//! They read what `tsset` left (`ts.rs`, which this file only reads); each
//! one works on one series on the time grid, so a gap in time is a missing
//! value, as `L.` and `D.` see it.

use super::stats::r_logical;
use super::xtsum::formats;
use super::*;

fn stop(out: &mut Out, msg: &str) {
    out.line(format!("stop({}, call. = FALSE)", r_text(msg)));
}

/// What the five commands share: the series on the time grid, a least-squares
/// fit, Fuller's tables, the critical-value block and the gap count.
pub(super) const TSTEST_COMMON_HELPER: &str = r##"stata_tst_pad <- function(s, w) {
  g <- w - nchar(s, type = "chars")
  paste0(strrep(" ", pmax(g, 0)), s)
}
stata_tst_left <- function(s, w) {
  g <- w - nchar(s, type = "chars")
  paste0(s, strrep(" ", pmax(g, 0)))
}
# the value s_{t - j} on a grid of consecutive times
stata_tst_lag <- function(x, j) {
  n <- length(x)
  if (j <= 0) return(x)
  if (j >= n) return(rep(NA_real_, n))
  c(rep(NA_real_, j), x[seq_len(n - j)])
}
# the sample of a series command: the rows `keep` keeps, in one panel at most, the
# series on the grid of times (a time with no row is missing)
stata_tst_prep <- function(d, y, time, panel, delta, keep, string_msg) {
  if (!(y %in% names(d))) stop("variable ", y, " not found (Stata: r(111))", call. = FALSE)
  if (is.character(d[[y]]) || is.factor(d[[y]])) stop(string_msg, call. = FALSE)
  tv0 <- stata_xt_num(d[[time]])
  keep <- keep & !is.na(tv0)
  if (!any(keep)) stop("no observations (Stata: r(2000))", call. = FALSE)
  if (!is.null(panel)) {
    pv <- d[[panel]]
    one <- unique(pv[keep])
    if (length(one) > 1) stop("sample may not include multiple panels (Stata: r(459))", call. = FALSE)
    inp <- !is.na(pv) & pv == one
    d <- d[inp, , drop = FALSE]
    keep <- keep[inp]
  }
  tv <- stata_xt_num(d[[time]])
  ok <- !is.na(tv)
  k <- rep(NA_integer_, length(tv))
  k[ok] <- as.integer(round((tv[ok] - min(tv[ok])) / delta)) + 1L
  span <- max(k[ok])
  if (span > 1e7) stop("a series spread over this many times is not translated", call. = FALSE)
  yv <- stata_xt_num(d[[y]])
  full <- rep(NA_real_, span)
  full[k[ok]] <- yv[ok]
  list(keep = keep & ok, tv = tv, k = k, span = span, full = full, y = yv)
}
stata_tst_ols <- function(X, y) {
  q <- qr(X)
  if (q$rank < ncol(X)) stop("the unit-root regression has collinear terms (Stata drops one): not translated", call. = FALSE)
  b <- qr.coef(q, y)
  e <- as.vector(y - X %*% b)
  df <- nrow(X) - ncol(X)
  rss <- sum(e^2)
  XtXi <- chol2inv(qr.R(q))
  list(b = as.vector(b), V = rss / df * XtXi, XtXi = XtXi, e = e, df = df, rss = rss, s2 = rss / df)
}
# Fuller's (1976) critical values at 1%, 5%, 10% for N = 25, 50, 100, 250, 500, Inf
stata_tst_tau_n <- matrix(c(-2.66, -1.95, -1.60, -2.62, -1.95, -1.61, -2.60, -1.95, -1.61, -2.58, -1.95, -1.62, -2.58, -1.95, -1.62, -2.58, -1.95, -1.62), 6, 3, byrow = TRUE)
stata_tst_tau_c <- matrix(c(-3.75, -3.00, -2.63, -3.58, -2.93, -2.60, -3.51, -2.89, -2.58, -3.46, -2.88, -2.57, -3.44, -2.87, -2.57, -3.43, -2.86, -2.57), 6, 3, byrow = TRUE)
stata_tst_tau_t <- matrix(c(-4.38, -3.60, -3.24, -4.15, -3.50, -3.18, -4.04, -3.45, -3.15, -3.99, -3.43, -3.13, -3.98, -3.42, -3.13, -3.96, -3.41, -3.12), 6, 3, byrow = TRUE)
stata_tst_rho_n <- matrix(c(-11.9, -7.3, -5.3, -12.9, -7.7, -5.5, -13.3, -7.9, -5.6, -13.6, -8.0, -5.7, -13.7, -8.0, -5.7, -13.8, -8.1, -5.7), 6, 3, byrow = TRUE)
stata_tst_rho_c <- matrix(c(-17.2, -12.5, -10.2, -18.9, -13.3, -10.7, -19.8, -13.7, -11.0, -20.3, -14.0, -11.2, -20.5, -14.0, -11.2, -20.7, -14.1, -11.3), 6, 3, byrow = TRUE)
stata_tst_rho_t <- matrix(c(-22.5, -17.9, -15.6, -25.7, -19.8, -16.8, -27.4, -20.7, -17.5, -28.4, -21.3, -18.0, -28.9, -21.5, -18.1, -29.5, -21.8, -18.3), 6, 3, byrow = TRUE)
stata_tst_crit <- function(tab, n) {
  sizes <- c(25, 50, 100, 250, 500, Inf)
  if (n <= 25) return(tab[1, ])
  for (i in 1:4) {
    if (n <= sizes[i + 1]) {
      w <- (n - sizes[i]) / (sizes[i + 1] - sizes[i])
      return(tab[i, ] + w * (tab[i + 1, ] - tab[i, ]))
    }
  }
  tab[5, ]
}
# MacKinnon's (1994) approximate p-value of the tau statistic (one series)
stata_tst_mackinnon <- function(tau, det) {
  p <- switch(det,
    n = list(-1.04, -19.04, Inf, c(0.6344, 1.2378, 3.2496e-2), c(0.4797, 9.3557e-1, -0.6999e-1, 3.3066e-2)),
    c = list(-1.61, -18.83, 2.74, c(2.1659, 1.4412, 3.8269e-2), c(1.7339, 9.3202e-1, -1.2745e-1, -1.0368e-2)),
    t = list(-2.89, -16.18, 0.7, c(3.2512, 1.6047, 4.9588e-2), c(2.5261, 6.1654e-1, -3.7956e-1, -6.0285e-2)))
  if (is.na(tau)) return(NA_real_)
  if (tau > p[[3]]) return(1)
  if (tau < p[[2]]) return(0)
  x <- if (tau <= p[[1]]) sum(p[[4]] * tau^(0:2)) else sum(p[[5]] * tau^(0:3))
  stats::pnorm(x)
}
stata_tst_h0 <- function(det, drift) {
  if (drift) return("H0: Random walk with drift, d = 0")
  switch(det, n = "H0: Random walk without drift, a = 0, d = 0", c = "H0: Random walk without drift, d = 0",
         t = "H0: Random walk with or without drift")
}
stata_tst_cv <- function(dist, rows) {
  cat(stata_tst_pad(dist, 52), "\n", sep = "")
  cat("                   Test      -------- critical value ---------\n")
  cat("              statistic           1%           5%          10%\n")
  cat(strrep("-", 62), "\n", sep = "")
  for (r in rows) {
    lead <- paste0(" ", r$label)
    f <- function(v) if (is.na(v)) "." else sprintf("%.3f", v)
    cat(lead, stata_tst_pad(f(r$stat), 23 - nchar(lead)), stata_tst_pad(f(r$cv[1]), 13), stata_tst_pad(f(r$cv[2]), 13),
        stata_tst_pad(f(r$cv[3]), 13), "\n", sep = "")
  }
  cat(strrep("-", 62), "\n", sep = "")
}
# the number of runs of missing values between the first and the last value
stata_tst_gaps <- function(x) {
  p <- which(!is.na(x))
  if (length(p) < 2) return(0L)
  r <- rle(is.na(x[p[1]:p[length(p)]]))
  sum(r$values)
}
stata_tst_gapnote <- function(g) {
  if (g > 0) cat("(note: time series has ", g, " gap", if (g > 1) "s", ")\n", sep = "")
}
# the regression table `dfuller, regress` shows (no ANOVA header)
stata_tst_regtable <- function(y, labels, fit, dep = paste0("D.", y)) {
  g9 <- function(v) stata_fmt(v, "%9.0g", trim = TRUE)
  q <- stats::qt(0.975, fit$df)
  cat(strrep("-", 78), "\n", sep = "")
  cat(stata_tst_pad(stata_tab_abbrev(dep), 12), " | Coefficient  Std. err.      t    P>|t|     [95% conf. interval]\n", sep = "")
  cat("-------------+----------------------------------------------------------------\n")
  ops <- grepl("\\.$", labels)
  se <- sqrt(diag(fit$V))
  if (any(ops)) cat(stata_tst_pad(stata_tab_abbrev(y), 12), " |\n", sep = "")
  for (i in seq_along(labels)) {
    if (!ops[i] && i > 1 && ops[i - 1]) cat("             |\n")
    t <- fit$b[i] / se[i]
    p <- 2 * stats::pt(-abs(t), fit$df)
    cat(stata_tst_pad(labels[i], 12), " |", stata_tst_pad(g9(fit$b[i]), 11), stata_tst_pad(g9(se[i]), 11),
        stata_tst_pad(sprintf("%.2f", t), 9), stata_tst_pad(sprintf("%.3f", p), 8),
        stata_tst_pad(g9(fit$b[i] - q * se[i]), 13), stata_tst_pad(g9(fit$b[i] + q * se[i]), 12), "\n", sep = "")
  }
  cat(strrep("-", 78), "\n", sep = "")
  invisible(NULL)
}
# the lines at the top of a test's report: the title with the number of observations and the
# variable with the lag count; the numbers are right-aligned to column 62
stata_tst_head <- function(title, nobs, var, label2, val2, aug = FALSE, pp = FALSE) {
  nobs <- as.character(nobs)
  val2 <- as.character(val2)
  w <- max(nchar(nobs), nchar(val2))
  nl <- if (pp) "Number of obs   = " else "Number of obs  = "
  room <- 62 - nchar(nl) - w
  if (aug) {
    cat("Augmented Dickey–Fuller test for unit root\n\n")
    cat(stata_tst_left(paste0("Variable: ", var), room), nl, stata_tst_pad(nobs, w), "\n", sep = "")
    cat(strrep(" ", room), label2, stata_tst_pad(val2, w), "\n", sep = "")
  } else {
    cat(stata_tst_left(title, room), nl, stata_tst_pad(nobs, w), "\n", sep = "")
    cat(stata_tst_left(paste0("Variable: ", var), room), label2, stata_tst_pad(val2, w), "\n", sep = "")
  }
}"##;

/// `dfuller`: the (augmented) Dickey-Fuller regression of D.y on L.y, lags of D.y
/// and the deterministic terms, with Fuller's critical values and MacKinnon's p.
pub(super) const DFULLER_HELPER: &str = r##"stata_dfuller <- function(d, y, time, panel = NULL, delta = 1, keep = rep(TRUE, nrow(d)), lags = 0,
                          det = "c", drift = FALSE, regress = FALSE, show = TRUE) {
  s <- stata_tst_prep(d, y, time, panel, delta, keep,
                      paste0(y, ":  may not use time-series operators on string variables (Stata: r(109))"))
  k <- s$k
  ly <- stata_tst_lag(s$full, 1)
  fd <- s$full - ly
  cols <- list(ly[k])
  labels <- "L1."
  for (j in seq_len(lags)) {
    cols[[length(cols) + 1]] <- stata_tst_lag(fd, j)[k]
    labels <- c(labels, if (j == 1) "LD." else paste0("L", j, "D."))
  }
  if (det == "t") {
    cols[[length(cols) + 1]] <- s$tv
    labels <- c(labels, "_trend")
  }
  if (det != "n") {
    cols[[length(cols) + 1]] <- rep(1, length(k))
    labels <- c(labels, "_cons")
  }
  X <- do.call(cbind, cols)
  dy <- fd[k]
  use <- s$keep & stats::complete.cases(X, dy)
  n <- sum(use)
  if (n == 0) stop("no observations (Stata: r(2000))", call. = FALSE)
  if (n <= ncol(X)) stop("too few observations for the unit-root regression: not translated (Stata shows a missing statistic)", call. = FALSE)
  fit <- stata_tst_ols(X[use, , drop = FALSE], dy[use])
  zt <- fit$b[1] / sqrt(fit$V[1, 1])
  cv <- if (drift) stats::qt(c(0.01, 0.05, 0.10), fit$df) else stata_tst_crit(switch(det, n = stata_tst_tau_n, c = stata_tst_tau_c, t = stata_tst_tau_t), n)
  pval <- if (drift) stats::pt(zt, fit$df) else stata_tst_mackinnon(zt, det)
  if (show) {
    cat("\n")
    if (lags == 0) stata_tst_head("Dickey–Fuller test for unit root", n, y, "Number of lags = ", lags)
    else stata_tst_head(NULL, n, y, "Number of lags = ", lags, aug = TRUE)
    cat("\n", stata_tst_h0(det, drift), "\n\n", sep = "")
    stata_tst_cv(if (drift) "t-distribution" else "Dickey–Fuller", list(list(label = "Z(t)", stat = zt, cv = cv)))
    if (drift) cat(sprintf("p-value for Z(t) = %.4f\n", pval))
    else if (det != "n") cat(sprintf("MacKinnon approximate p-value for Z(t) = %.4f.\n", pval))
    if (regress) {
      cat("\nRegression table\n")
      stata_tst_regtable(y, labels, fit)
    }
  }
  r <- list(lags = lags, N = n, Zt = zt)
  if (det != "n" || drift) r$p <- pval
  r$cv_1 <- cv[1]
  r$cv_5 <- cv[2]
  r$cv_10 <- cv[3]
  invisible(r)
}"##;

/// `pperron`: Phillips and Perron's Z(rho) and Z(t) with Newey-West weights.
pub(super) const PPERRON_HELPER: &str = r##"stata_pperron <- function(d, y, time, panel = NULL, delta = 1, keep = rep(TRUE, nrow(d)), lags = NULL,
                          det = "c", regress = FALSE, show = TRUE) {
  s <- stata_tst_prep(d, y, time, panel, delta, keep,
                      paste0(y, ":  may not use time-series operators on string variables (Stata: r(109))"))
  k <- s$k
  cols <- list(stata_tst_lag(s$full, 1)[k])
  if (det == "t") cols[[length(cols) + 1]] <- s$tv
  if (det != "n") cols[[length(cols) + 1]] <- rep(1, length(k))
  X <- do.call(cbind, cols)
  use <- s$keep & stats::complete.cases(X, s$y)
  n <- sum(use)
  if (n == 0) stop("no observations (Stata: r(2000))", call. = FALSE)
  if (n <= ncol(X) + 1) stop("too few observations for the unit-root regression: not translated", call. = FALSE)
  fit <- stata_tst_ols(X[use, , drop = FALSE], s$y[use])
  tt <- n
  eg <- rep(NA_real_, s$span)
  eg[k[use]] <- fit$e
  L <- length(eg)
  q <- if (is.null(lags) || lags < 0) floor(4 * (tt / 100)^(2 / 9)) else lags
  gam <- function(j) if (j >= L) 0 else sum(eg[(j + 1):L] * eg[1:(L - j)], na.rm = TRUE) / tt
  g0 <- gam(0)
  lam2 <- g0 + 2 * sum(vapply(seq_len(q), function(j) (1 - j / (q + 1)) * gam(j), 0))
  rho <- fit$b[1]
  se <- sqrt(fit$V[1, 1])
  s2 <- fit$rss / (tt - ncol(X))
  zrho <- tt * (rho - 1) - 0.5 * (tt^2 * se^2 / s2) * (lam2 - g0)
  zt <- sqrt(g0 / lam2) * (rho - 1) / se - 0.5 * (lam2 - g0) / sqrt(lam2) * tt * se / sqrt(s2)
  cv_rho <- stata_tst_crit(switch(det, n = stata_tst_rho_n, c = stata_tst_rho_c, t = stata_tst_rho_t), tt)
  cv_t <- stata_tst_crit(switch(det, n = stata_tst_tau_n, c = stata_tst_tau_c, t = stata_tst_tau_t), tt)
  pval <- stata_tst_mackinnon(zt, det)
  labels <- c("L1.", if (det == "t") "_trend", if (det != "n") "_cons")
  if (show) {
    cat("\n")
    stata_tst_head("Phillips–Perron test for unit root", n, y, "Newey–West lags = ", q, pp = TRUE)
    cat("\n", stata_tst_h0(det, FALSE), "\n\n", sep = "")
    stata_tst_cv("Dickey–Fuller", list(list(label = "Z(rho)", stat = zrho, cv = cv_rho), list(label = "Z(t)", stat = zt, cv = cv_t)))
    if (det != "n") cat(sprintf("MacKinnon approximate p-value for Z(t) = %.4f.\n", pval))
    if (regress) {
      cat("\nRegression table\n")
      stata_tst_regtable(y, labels, fit, dep = y)
    }
  }
  cvm <- matrix(c(zrho, cv_rho, zt, cv_t), 2, 4, byrow = TRUE,
                dimnames = list(c("Z(rho)", "Z(t)"), c("Test", "1%", "5%", "10%")))
  r <- list(N = n, lags = q, Zrho = zrho, Zt = zt)
  if (det != "n") r$pval <- pval
  r$cvalues <- cvm
  invisible(r)
}"##;

/// `corrgram` and `wntestq`: autocorrelations from the sum of squares about the mean,
/// partial autocorrelations by regression (or Yule-Walker), Ljung and Box's Q.
pub(super) const CORRGRAM_HELPER: &str = r##"stata_tst_acf <- function(d, y, time, panel, delta, keep, lags, strmsg = "type mismatch (Stata: r(109))") {
  s <- stata_tst_prep(d, y, time, panel, delta, keep, strmsg)
  x <- rep(NA_real_, s$span)
  x[s$k[s$keep]] <- s$y[s$keep]
  p <- which(!is.na(x))
  if (!length(p)) stop("no observations (Stata: r(2000))", call. = FALSE)
  x <- x[p[1]:p[length(p)]]
  n <- sum(!is.na(x))
  m <- if (is.null(lags)) min(floor(n / 2) - 2, 40) else lags
  if (m < 1) stop("lags() must be greater than zero (Stata: r(498))", call. = FALSE)
  if (m >= n) stop("lags() too large; must be less than ", n, " (Stata: r(498))", call. = FALSE)
  list(x = x, n = n, m = m, gaps = stata_tst_gaps(x))
}
stata_tst_autocorr <- function(x, m) {
  dd <- x - mean(x, na.rm = TRUE)
  c0 <- sum(dd^2, na.rm = TRUE)
  L <- length(x)
  vapply(seq_len(m), function(j) if (j >= L) 0 else sum(dd[(j + 1):L] * dd[1:(L - j)], na.rm = TRUE) / c0, 0)
}
stata_corrgram <- function(d, y, time, panel = NULL, delta = 1, keep = rep(TRUE, nrow(d)), lags = NULL,
                           noplot = FALSE, yw = FALSE, show = TRUE) {
  a <- stata_tst_acf(d, y, time, panel, delta, keep, lags)
  x <- a$x
  n <- a$n
  m <- a$m
  ac <- stata_tst_autocorr(x, m)
  pac <- if (yw) {
    # Durbin-Levinson from the autocorrelations
    phi <- numeric(0)
    out <- numeric(m)
    for (k in seq_len(m)) {
      if (k == 1) {
        nk <- ac[1]
        phi <- nk
      } else {
        nk <- (ac[k] - sum(phi * ac[(k - 1):1])) / (1 - sum(phi * ac[1:(k - 1)]))
        phi <- c(phi - nk * rev(phi), nk)
      }
      out[k] <- nk
    }
    out
  } else {
    vapply(seq_len(m), function(k) {
      cols <- c(list(x), lapply(seq_len(k), function(j) stata_tst_lag(x, j)))
      M <- do.call(cbind, cols)
      M <- M[stats::complete.cases(M), , drop = FALSE]
      X <- cbind(M[, -1, drop = FALSE], 1)
      unname(qr.coef(qr(X), M[, 1])[k])
    }, 0)
  }
  q <- n * (n + 2) * cumsum(ac^2 / (n - seq_len(m)))
  pv <- stats::pchisq(q, seq_len(m), lower.tail = FALSE)
  if (show) {
    stata_tst_gapnote(a$gaps)
    cat("\n")
    bar <- function(v) {
      b <- trunc(v * 8)
      dashes <- strrep("-", min(abs(b), 8))
      if (b < 0) paste0(stata_tst_pad(dashes, 9), "|", strrep(" ", 8)) else paste0(strrep(" ", 9), "|", stata_tst_left(dashes, 8))
    }
    if (noplot) {
      cat(" LAG       AC       PAC      Q     Prob>Q\n", strrep("-", 41), "\n", sep = "")
    } else {
      cat("                                          -1       0       1 -1       0       1\n")
      cat(" LAG       AC       PAC      Q     Prob>Q  [Autocorrelation]  [Partial autocor]\n")
      cat(strrep("-", 79), "\n", sep = "")
    }
    for (j in seq_len(m)) {
      row <- paste0(sprintf("%-3d", j), stata_tst_pad(sprintf("%.4f", ac[j]), 12), stata_tst_pad(sprintf("%.4f", pac[j]), 9),
                    stata_tst_pad(stata_fmt(q[j], "%9.5g", trim = TRUE), 9), stata_tst_pad(sprintf("%.4f", pv[j]), 8))
      if (!noplot) row <- paste0(row, " ", bar(ac[j]), " ", bar(pac[j]))
      cat(row, "\n", sep = "")
    }
  }
  r <- list(lags = m)
  for (j in rev(seq_len(min(m, 10)))) {
    r[[paste0("p", j)]] <- pv[j]
    r[[paste0("q", j)]] <- q[j]
    r[[paste0("pac", j)]] <- pac[j]
    r[[paste0("ac", j)]] <- ac[j]
  }
  r$P <- matrix(pv, 1, m)
  r$Q <- matrix(q, 1, m)
  r$PAC <- matrix(pac, 1, m)
  r$AC <- matrix(ac, 1, m)
  invisible(r)
}
stata_wntestq <- function(d, y, time, panel = NULL, delta = 1, keep = rep(TRUE, nrow(d)), lags = NULL, show = TRUE) {
  a <- stata_tst_acf(d, y, time, panel, delta, keep, lags, "no observations (Stata: r(2000))")
  n <- a$n
  m <- a$m
  ac <- stata_tst_autocorr(a$x, m)
  q <- n * (n + 2) * sum(ac^2 / (n - seq_len(m)))
  pv <- stats::pchisq(q, m, lower.tail = FALSE)
  if (show) {
    stata_tst_gapnote(a$gaps)
    cat("\nPortmanteau test for white noise\n", strrep("-", 39), "\n", sep = "")
    cat(" Portmanteau (Q) statistic = ", stata_tst_pad(sprintf("%.4f", q), 10), "\n", sep = "")
    cat(" ", stata_tst_left(paste0("Prob > chi2(", m, ")"), 26), "= ", stata_tst_pad(sprintf("%.4f", pv), 10), "\n", sep = "")
  }
  invisible(list(df = m, p = pv, stat = q))
}"##;

/// `dfgls`: Elliott, Rothenberg and Stock's GLS-detrended Dickey-Fuller test.
pub(super) const DFGLS_HELPER: &str = r##"stata_tst_ers <- function(t) {
  rows <- rbind(c(-3.77, -3.19, -2.89), c(-3.58, -3.03, -2.74), c(-3.46, -2.93, -2.64), c(-3.48, -2.89, -2.57))
  sizes <- c(50, 100, 200)
  if (t <= 50) return(rows[1, ])
  if (t > 200) return(rows[4, ])
  i <- if (t > 100) 2 else 1
  w <- (t - sizes[i]) / (sizes[i + 1] - sizes[i])
  rows[i, ] + w * (rows[i + 1, ] - rows[i, ])
}
# Cheung and Lai's (1995) response surface for the 5% and 10% critical values
stata_tst_cheunglai <- function(trend, level5, t, p) {
  cf <- if (trend) {
    if (level5) c(-2.838, -20.328, 124.191, 1.267, 10.530, -24.600) else c(-2.550, -20.166, 155.215, 1.133, 9.808, -20.313)
  } else {
    if (level5) c(-1.948, -17.839, 104.086, 0.802, 5.558, -18.332) else c(-1.624, -19.888, 155.231, 0.709, 5.480, -16.055)
  }
  a <- 1 / t
  b <- p / t
  cf[1] + cf[2] * a + cf[3] * a^2 + cf[4] * b + cf[5] * b^2 + cf[6] * b^3
}
stata_dfgls <- function(d, y, time, panel = NULL, delta = 1, keep = rep(TRUE, nrow(d)), maxlag = NULL,
                        trend = TRUE, ers = FALSE, show = TRUE) {
  s <- stata_tst_prep(d, y, time, panel, delta, keep, "no observations (Stata: r(2000))")
  x <- rep(NA_real_, s$span)
  x[s$k[s$keep]] <- s$y[s$keep]
  p <- which(!is.na(x))
  if (!length(p)) stop("no observations (Stata: r(2000))", call. = FALSE)
  x <- x[p[1]:p[length(p)]]
  g <- stata_tst_gaps(x)
  if (g > 0) {
    cat("\nNumber of gaps in sample = ", g, "\n", sep = "")
    stop("sample may not contain gaps (Stata: r(498))", call. = FALSE)
  }
  total <- length(x)
  tf <- total
  schwert <- is.null(maxlag)
  if (schwert) maxlag <- floor(12 * (tf / 100)^0.25)
  if (total - maxlag - 1 <= 0) stop("no observations (Stata: r(2000))", call. = FALSE)
  if (total - maxlag - 1 == 1) stop("insufficient observations (Stata: r(2001))", call. = FALSE)
  if (total - 2 * maxlag - 2 < 1) stop("too few observations for this many lags: not translated (Stata shows missing statistics)", call. = FALSE)
  cbar <- if (trend) -13.5 else -7
  abar <- 1 + cbar / tf
  z <- if (trend) cbind(1, seq_len(total)) else matrix(1, total, 1)
  zt <- rbind(z[1, ], z[-1, , drop = FALSE] - abar * z[-total, , drop = FALSE])
  yt <- c(x[1], x[-1] - abar * x[-total])
  beta <- qr.coef(qr(zt), yt)
  yd <- x - as.vector(z %*% beta)
  dy <- c(NA_real_, diff(yd))
  tt <- (maxlag + 2):total
  n <- length(tt)
  nf <- n
  fit <- function(l) {
    X <- cbind(yd[tt - 1], if (l > 0) vapply(seq_len(l), function(j) dy[tt - j], numeric(n)))
    f <- stata_tst_ols(X, dy[tt])
    list(l = l, tau = f$b[1] / sqrt(f$V[1, 1]), rss = f$rss, last_t = f$b[l + 1] / sqrt(f$V[l + 1, l + 1]),
         maic_term = f$b[1]^2 * sum(yd[tt - 1]^2) / (f$rss / nf))
  }
  seq_l <- if (maxlag == 0) 0 else maxlag:1
  fits <- lapply(seq_l, fit)
  sig2 <- function(f) f$rss / nf
  rmse <- function(f) sqrt(sig2(f))
  sic <- function(f) log(sig2(f)) + (f$l + 1) * log(nf) / nf
  maic <- function(f) log(sig2(f)) + 2 * (f$maic_term + f$l) / nf
  opt <- NULL
  for (f in fits) if (abs(f$last_t) > 1.6448536269514722) { opt <- f; break }
  best <- function(crit) {
    v <- vapply(fits, crit, 0)
    i <- which.min(v)
    list(l = fits[[i]]$l, v = v[i], rmse = rmse(fits[[i]]))
  }
  bs <- best(sic)
  bm <- best(maic)
  cvf <- function(l) {
    if (ers) {
      if (trend) stata_tst_ers(tf) else stata_tst_crit(stata_tst_tau_n, tf)
    } else {
      one <- if (trend) stata_tst_ers(tf)[1] else stata_tst_crit(stata_tst_tau_n, tf)[1]
      c(one, stata_tst_cheunglai(trend, TRUE, nf, l), stata_tst_cheunglai(trend, FALSE, nf, l))
    }
  }
  if (show) {
    w <- nchar(as.character(n))
    gg <- function(v) stata_fmt(v, "%9.0g", trim = TRUE)
    cat("\n", "DF-GLS test for unit root", stata_tst_pad(paste0("Number of obs = ", n), 62 - 25), "\n", sep = "")
    cat("Variable: ", y, "\n", sep = "")
    left <- paste0("Lag selection: ", if (schwert) "Schwert criterion" else "User specified")
    cat(left, stata_tst_pad(paste0("Maximum lag   = ", stata_tst_pad(as.character(maxlag), w)), 62 - nchar(left)), "\n\n", sep = "")
    if (ers) {
      cat("                                  Elliott–Rothenberg–Stock\n")
      cat("                             -------- critical value ---------\n")
    } else {
      cat("                             -------- Critical value ---------\n")
    }
    cat(" [lags]", stata_tst_pad(if (trend) "DF-GLS tau" else "DF-GLS mu", 16), "           1%           5%          10%\n", sep = "")
    cat(strrep("-", 62), "\n", sep = "")
    fv <- function(v) if (is.na(v)) "." else sprintf("%.3f", v)
    for (f in fits) {
      cvv <- cvf(f$l)
      cat(stata_tst_pad(as.character(f$l), 7), stata_tst_pad(fv(f$tau), 16), stata_tst_pad(fv(cvv[1]), 13),
          stata_tst_pad(fv(cvv[2]), 13), stata_tst_pad(fv(cvv[3]), 13), "\n", sep = "")
    }
    cat(strrep("-", 62), "\n", sep = "")
    lag_w <- if (!is.null(opt)) 1 else 2
    rmse_w <- if (!is.null(opt)) 8 else 7
    if (maxlag > 1) {
      if (!is.null(opt)) {
        cat("Opt lag (Ng–Perron seq t) = ", opt$l, " with RMSE = ", gg(rmse(opt)), "\n", sep = "")
      } else {
        cat("Opt lag (Ng–Perron seq t) = 0, use option maxlag(0)\n")
      }
      cat("Min SIC  = ", stata_tst_pad(gg(bs$v), 9), " at lag ", stata_tst_pad(as.character(bs$l), lag_w), " with RMSE = ",
          stata_tst_pad(gg(bs$rmse), rmse_w), "\n", sep = "")
      cat("Min MAIC = ", stata_tst_pad(gg(bm$v), 9), " at lag ", stata_tst_pad(as.character(bm$l), lag_w), " with RMSE = ",
          stata_tst_pad(gg(bm$rmse), rmse_w), "\n", sep = "")
    }
  }
  rows <- length(fits)
  # what Stata leaves: with no lag to choose among, its r() is shorter
  if (maxlag == 0) {
    cv0 <- cvf(0)
    return(invisible(list(N = n, cv_1 = cv0[1], cv_5 = cv0[2], cv_10 = cv0[3], rmse0 = rmse(fits[[1]]), dft0 = fits[[1]]$tau)))
  }
  cvm <- matrix(0, rows, 5, dimnames = list(rep("r1", rows), c("k", "DFGLS", "1% cvalue", "5% cvalue", "10% cvalue")))
  res <- matrix(0, rows, 5, dimnames = list(rep("r1", rows), c("k", "MAIC", "SIC", "RMSE", "DFGLS")))
  for (i in seq_len(rows)) {
    f <- fits[[i]]
    cvm[i, ] <- c(f$l, f$tau, cvf(f$l))
    res[i, ] <- c(f$l, maic(f), sic(f), rmse(f), f$tau)
  }
  r <- list(maxlag = maxlag, N = n)
  if (maxlag > 1) {
    r <- c(r, list(siclag = bs$l, maiclag = bm$l, maicrmse = bm$rmse, maicmin = bm$v,
                   sicrmse = bs$rmse, sicmin = bs$v, optrmse = if (is.null(opt)) NA_real_ else rmse(opt),
                   optlag = if (is.null(opt)) 0 else opt$l))
  }
  r$cvalues <- cvm
  r$results <- res
  invisible(r)
}"##;

const R_DFULLER: &[&str] = &["lags", "N", "Zt", "p", "cv_1", "cv_5", "cv_10"];
const R_PPERRON: &[&str] = &["N", "lags", "Zrho", "Zt", "pval", "cvalues"];
const R_DFGLS: &[&str] = &[
    "maxlag", "N", "siclag", "maiclag", "maicrmse", "maicmin", "sicrmse", "sicmin", "optrmse",
    "optlag", "cvalues", "results", "cv_1", "cv_5", "cv_10", "rmse0", "dft0",
];
const R_WNTESTQ: &[&str] = &["df", "p", "stat"];
const R_CORRGRAM: &[&str] = &["lags", "P", "Q", "PAC", "AC"];

/// Whether `name` is something corrgram leaves: `Some(false)` a number
/// (`ac3`), `Some(true)` a matrix (`AC`).
pub(super) fn corrgram_name(name: &str) -> Option<bool> {
    if matches!(name, "P" | "Q" | "PAC" | "AC") {
        return Some(true);
    }
    for p in ["pac", "ac", "p", "q"] {
        if let Some(k) = name.strip_prefix(p) {
            if !k.is_empty() && k.chars().all(|c| c.is_ascii_digit()) {
                return Some(false);
            }
        }
    }
    None
}

/// What the matrices are called among the r() of these commands.
pub(super) fn tstest_matrix(cmd: &str, name: &str) -> bool {
    match cmd {
        "corrgram" => corrgram_name(name) == Some(true),
        "pperron" | "dfgls" => name == "cvalues" || (cmd == "dfgls" && name == "results"),
        "varsoc" => name == "stats",
        _ => false,
    }
}

impl Session {
    /// `dfuller`, `pperron`, `dfgls`, `corrgram` and `wntestq`: one variable
    /// (a time-series operator is allowed), `if` and `in`, the data `tsset`.
    pub(super) fn tstest(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let name = c.name.as_str();
        self.no_by(c, name)?;
        if self.refuse_extras(c, out, "", false) {
            return Ok(());
        }
        let allowed: &[(&str, usize)] = match name {
            "dfuller" => &[
                ("noconstant", 3),
                ("trend", 1),
                ("drift", 2),
                ("regress", 1),
                ("lags", 1),
            ],
            "pperron" => &[("noconstant", 3), ("trend", 1), ("regress", 1), ("lags", 1)],
            "dfgls" => &[("maxlag", 1), ("notrend", 3), ("ers", 3)],
            "corrgram" => &[("lags", 1), ("noplot", 3), ("yw", 2)],
            _ => &[("lags", 1)],
        };
        if let Some(o) = opts.other(allowed) {
            stop(out, &format!("option {o} not allowed (Stata: r(198))"));
            return Ok(());
        }
        let words: Vec<&str> = c.parts.main.split_whitespace().collect();
        let var = match words.as_slice() {
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
        let cols = self.ts_columns(&var)?;
        if cols.is_none() && !is_name(&var) {
            return Err(no(format!(
                "{name} {var}: only a variable name or a time-series-operator term is translated"
            )));
        }
        let Some((panel, Some(time))) = self.panel_state(
            true,
            out,
            "time variable not set, use tsset varname ... (Stata: r(111))",
            "time variable not set, use tsset varname ... (Stata: r(111))",
            111,
        )?
        else {
            return Ok(());
        };
        // lags() and maxlag(): a whole number
        let number = |key: &str, out: &mut Out| -> Result<Option<Option<i64>>, Untranslated> {
            match opts.arg(key, 1) {
                None => Ok(Some(None)),
                Some(t) => match t.trim().parse::<i64>() {
                    Ok(v) => Ok(Some(Some(v))),
                    Err(_) => {
                        let rc = if key == "maxlag" { 126 } else { 198 };
                        stop(
                            out,
                            &format!("option {key}() incorrectly specified (Stata: r({rc}))"),
                        );
                        Ok(None)
                    }
                },
            }
        };
        let det = if opts.has("noconstant", 3) {
            "n"
        } else if opts.has("trend", 1) {
            "t"
        } else {
            "c"
        };
        let drift = opts.has("drift", 2);
        if matches!(name, "dfuller" | "pperron") {
            if opts.has("noconstant", 3) && opts.has("trend", 1) {
                stop(
                    out,
                    "cannot choose trend if constant is excluded (Stata: r(198))",
                );
                return Ok(());
            }
            if drift && opts.has("trend", 1) {
                stop(
                    out,
                    "cannot specify drift if time trend is included (Stata: r(198))",
                );
                return Ok(());
            }
            if drift && opts.has("noconstant", 3) {
                stop(
                    out,
                    "cannot specify drift if constant is excluded (Stata: r(198))",
                );
                return Ok(());
            }
        }
        let d = self.data()?;
        let frame = match cols {
            Some(cols) => {
                out.package("dplyr");
                format!("dplyr::mutate({d}, {cols})")
            }
            None => d,
        };
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
            "stata_tst_common",
            "stata_xt_common",
            "stata_fmt",
            "stata_tables",
        ] {
            self.helpers.insert(h);
        }
        let show = if self.quiet(c) { ", show = FALSE" } else { "" };
        let ts = format!(
            "{}, {}, panel = {}, delta = {}, keep = {keep}",
            r_text(&var),
            r_text(&time),
            panel
                .as_ref()
                .map(|p| r_text(p))
                .unwrap_or_else(|| "NULL".into()),
            self.ts_delta.clone().unwrap_or_else(|| "1".to_string())
        );
        let (helper, call, names): (&str, String, &'static [&'static str]) = match name {
            "dfuller" => {
                let Some(lags) = number("lags", out)? else {
                    return Ok(());
                };
                (
                    "stata_dfuller",
                    format!(
                        "lags = {}, det = {}, drift = {}, regress = {}",
                        lags.unwrap_or(0).max(0),
                        r_text(det),
                        r_logical(drift),
                        r_logical(opts.has("regress", 1))
                    ),
                    R_DFULLER,
                )
            }
            "pperron" => {
                let Some(lags) = number("lags", out)? else {
                    return Ok(());
                };
                (
                    "stata_pperron",
                    format!(
                        "lags = {}, det = {}, regress = {}",
                        lags.filter(|l| *l >= 0)
                            .map_or("NULL".to_string(), |l| l.to_string()),
                        r_text(det),
                        r_logical(opts.has("regress", 1))
                    ),
                    R_PPERRON,
                )
            }
            "dfgls" => {
                let Some(maxlag) = number("maxlag", out)? else {
                    return Ok(());
                };
                if maxlag.is_some_and(|m| m < 0) {
                    return Err(no("dfgls, maxlag(): a negative number is not translated"));
                }
                (
                    "stata_dfgls",
                    format!(
                        "maxlag = {}, trend = {}, ers = {}",
                        maxlag.map_or("NULL".to_string(), |m| m.to_string()),
                        r_logical(!opts.has("notrend", 3)),
                        r_logical(opts.has("ers", 3))
                    ),
                    R_DFGLS,
                )
            }
            "corrgram" => {
                let Some(lags) = number("lags", out)? else {
                    return Ok(());
                };
                (
                    "stata_corrgram",
                    format!(
                        "lags = {}, noplot = {}, yw = {}",
                        lags.map_or("NULL".to_string(), |l| l.to_string()),
                        r_logical(opts.has("noplot", 3)),
                        r_logical(opts.has("yw", 2))
                    ),
                    R_CORRGRAM,
                )
            }
            _ => {
                let Some(lags) = number("lags", out)? else {
                    return Ok(());
                };
                (
                    "stata_wntestq",
                    format!(
                        "lags = {}",
                        lags.map_or("NULL".to_string(), |l| l.to_string())
                    ),
                    R_WNTESTQ,
                )
            }
        };
        self.helpers.insert(helper);
        out.line(format!(
            "stata_r <- local({{ stata_d <- {frame}; {helper}(stata_d, {ts}, {call}{show}) }})"
        ));
        self.r_new = Some((names, name.into()));
        Ok(())
    }
}

/// `varsoc`: the lag-order selection criteria of a vector autoregression, each order
/// fitted on the sample the longest one leaves.
pub(super) const VARSOC_HELPER: &str = r##"stata_varsoc <- function(d, endog, time, delta = 1, keep = rep(TRUE, nrow(d)), maxlag = 4, exog = character(0),
                         constant = TRUE, fmts = character(0), show = TRUE) {
  for (v in c(endog, exog)) {
    if (!(v %in% names(d))) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
    if (is.character(d[[v]]) || is.factor(d[[v]])) stop("type mismatch (Stata: r(109))", call. = FALSE)
  }
  tv <- stata_xt_num(d[[time]])
  ok <- !is.na(tv)
  if (!any(ok)) stop("no observations (Stata: r(2000))", call. = FALSE)
  k <- rep(NA_integer_, length(tv))
  k[ok] <- as.integer(round((tv[ok] - min(tv[ok])) / delta)) + 1L
  span <- max(k[ok])
  grid <- function(v) {
    x <- rep(NA_real_, span)
    x[k[ok]] <- stata_xt_num(d[[v]])[ok]
    x
  }
  G <- lapply(endog, grid)
  ys <- do.call(cbind, lapply(G, function(g) g[k]))
  lagm <- function(l) do.call(cbind, lapply(G, function(g) stata_tst_lag(g, l)[k]))
  lags <- lapply(seq_len(maxlag), lagm)
  ex <- if (length(exog)) do.call(cbind, lapply(exog, function(v) stata_xt_num(d[[v]]))) else NULL
  allm <- cbind(ys, do.call(cbind, lags), ex)
  used <- keep & ok & stats::complete.cases(allm)
  n <- sum(used)
  if (n == 0) stop("no observations (Stata: r(2000))", call. = FALSE)
  neq <- length(endog)
  Ys <- ys[used, , drop = FALSE]
  rows <- matrix(NA_real_, maxlag + 1, 9, dimnames = list(rep("r1", maxlag + 1), c("lag", "LL", "LR", "df", "p", "FPE", "AIC", "HQIC", "SBIC")))
  for (ord in 0:maxlag) {
    X <- cbind(if (ord > 0) do.call(cbind, lags[seq_len(ord)])[used, , drop = FALSE], if (!is.null(ex)) ex[used, , drop = FALSE], if (constant) rep(1, n))
    if (is.null(X)) {
      E <- Ys
      kk <- 0
    } else {
      q <- qr(X)
      if (q$rank < ncol(X)) stop("the exogenous variables may not be collinear with the dependent variables, or their lags (Stata: r(198))", call. = FALSE)
      E <- qr.resid(q, Ys)
      kk <- ncol(X)
    }
    sigma <- crossprod(E) / n
    detsig <- det(sigma)
    ll <- -0.5 * n * (neq * log(2 * pi) + log(detsig) + neq)
    tp <- neq * kk
    rows[ord + 1, c("lag", "LL")] <- c(ord, ll)
    rows[ord + 1, "FPE"] <- detsig * ((n + kk) / (n - kk))^neq
    rows[ord + 1, "AIC"] <- -2 * ll / n + 2 * tp / n
    rows[ord + 1, "HQIC"] <- -2 * ll / n + 2 * log(log(n)) * tp / n
    rows[ord + 1, "SBIC"] <- -2 * ll / n + log(n) * tp / n
  }
  for (i in seq_len(maxlag)) {
    lr <- 2 * (rows[i + 1, "LL"] - rows[i, "LL"])
    rows[i + 1, c("LR", "df", "p")] <- c(lr, neq^2, stats::pchisq(lr, neq^2, lower.tail = FALSE))
  }
  tused <- tv[used]
  tmin <- min(tused)
  tmax <- max(tused)
  present <- rep(NA_real_, span)
  present[k[used]] <- 1
  gaps <- stata_tst_gaps(present)
  if (show) {
    lrstar <- which(!is.na(rows[, "p"]) & rows[, "p"] < 0.05)
    lrstar <- if (length(lrstar)) max(lrstar) - 1 else NA_integer_
    star <- function(i, s) if (!is.na(s) && s == i) "*" else " "
    sf <- function(col) which.min(rows[, col]) - 1
    stars <- c(sf("FPE"), sf("AIC"), sf("HQIC"), sf("SBIC"))
    tf <- stata_xt_fmt_of(d, time, fmts)
    left <- paste0("   Sample: ", stata_xt_text(tmin, tf), " thru ", stata_xt_text(tmax, tf))
    cat("\nLag-order selection criteria\n\n")
    cat(left, stata_tst_pad(paste0("Number of obs = ", n), 78 - nchar(left)), "\n", sep = "")
    rule <- paste0("  +", strrep("-", 75), "+")
    cat(rule, "\n  | Lag |    LL      LR      df    p     FPE       AIC      HQIC      SBIC    |\n", sep = "")
    cat("  |-----+", strrep("-", 69), "|\n", sep = "")
    g6 <- function(v, w) stata_tst_pad(stata_fmt(v, paste0("%", w, ".6g"), trim = TRUE), w)
    for (i in 0:maxlag) {
      r <- rows[i + 1, ]
      tests <- if (i == 0) strrep(" ", 20) else paste0(stata_tst_pad(stata_fmt(r[["LR"]], "%8.5g", trim = TRUE), 8), star(i, lrstar),
                                                         stata_tst_pad(as.character(r[["df"]]), 4), "  ", sprintf("%5.3f", r[["p"]]))
      cat("  | ", stata_tst_pad(as.character(i), 3), " | ", stata_tst_pad(stata_fmt(r[["LL"]], "%9.6g", trim = TRUE), 8), " ", tests, " ",
          stata_tst_pad(stata_fmt(r[["FPE"]], "%7.6g", trim = TRUE), 7), star(i, stars[1]), g6(r[["AIC"]], 9), star(i, stars[2]),
          g6(r[["HQIC"]], 9), star(i, stars[3]), g6(r[["SBIC"]], 9), star(i, stars[4]), " |\n", sep = "")
    }
    cat(rule, "\n   * optimal lag\n   Endogenous: ", paste(endog, collapse = " "), "\n    Exogenous: ", paste(c(exog, if (constant) "_cons"), collapse = " "), "\n", sep = "")
  }
  invisible(list(N_gaps = gaps, mlag = maxlag, N = n, tmax = tmax, tmin = tmin,
                 exog = paste(c(exog, if (constant) "_cons"), collapse = " "), endog = paste(endog, collapse = " "), stats = rows))
}"##;

const R_VARSOC: &[&str] = &[
    "N_gaps", "mlag", "N", "tmax", "tmin", "exog", "endog", "stats",
];

/// The names `varsoc` leaves as text.
pub(super) fn varsoc_text(name: &str) -> bool {
    matches!(name, "exog" | "endog")
}

impl Session {
    /// `varsoc depvarlist [if] [in] [, maxlag(#) exog(varlist) noconstant]`.
    pub(super) fn varsoc(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "varsoc")?;
        if self.refuse_extras(c, out, "", false) {
            return Ok(());
        }
        if let Some(o) = opts.other(&[("maxlag", 4), ("exog", 4), ("noconstant", 3)]) {
            stop(out, &format!("option {o} not allowed (Stata: r(198))"));
            return Ok(());
        }
        let endog: Vec<String> = c
            .parts
            .main
            .split_whitespace()
            .map(str::to_string)
            .collect();
        if opts.has("noconstant", 3) {
            return Err(no(
                "varsoc, noconstant is not translated yet (Stata's statistics without a constant, even at lag 0, are not decoded)",
            ));
        }
        if endog.is_empty() {
            stop(out, "varlist required (Stata: r(198))");
            return Ok(());
        }
        if let Some(v) = endog.iter().find(|v| !is_name(v)) {
            return Err(no(format!(
                "varsoc {v}: only plain variable names are translated"
            )));
        }
        let exog: Vec<String> = opts
            .arg("exog", 4)
            .map(|e| e.split_whitespace().map(str::to_string).collect())
            .unwrap_or_default();
        if let Some(v) = exog.iter().find(|v| !is_name(v)) {
            return Err(no(format!(
                "varsoc, exog({v}): only plain variable names are translated"
            )));
        }
        let maxlag = match opts.arg("maxlag", 4) {
            None => 4,
            Some(t) => match t.trim().parse::<i64>() {
                Ok(v) if (1..=100).contains(&v) => v,
                Ok(v) if v < 1 => {
                    stop(out, "maxlag() must be positive (Stata: r(198))");
                    return Ok(());
                }
                Ok(_) => {
                    return Err(no("varsoc, maxlag(): only 1 to 100 lags are translated"));
                }
                Err(_) => {
                    stop(out, "option maxlag() incorrectly specified (Stata: r(198))");
                    return Ok(());
                }
            },
        };
        if self.panel.is_some() {
            return Err(no(
                "varsoc after xtset (a panel) is not translated: Stata's lags and sample would be within panels",
            ));
        }
        let Some((_, Some(time))) = self.panel_state(
            true,
            out,
            "time variable not set, use tsset varname ... (Stata: r(111))",
            "time variable not set, use tsset varname ... (Stata: r(111))",
            111,
        )?
        else {
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
            "stata_varsoc",
            "stata_tst_common",
            "stata_xt_common",
            "stata_fmt",
            "stata_tables",
        ] {
            self.helpers.insert(h);
        }
        let names = |v: &[String]| v.iter().map(|s| r_text(s)).collect::<Vec<_>>().join(", ");
        let show = if self.quiet(c) { ", show = FALSE" } else { "" };
        let fmts = formats(self, &[time.as_str()]);
        out.line(format!(
            "stata_r <- local({{ stata_d <- {d}; stata_varsoc(stata_d, c({}), {}, delta = {}, keep = {keep}, maxlag = {maxlag}, exog = c({}), constant = {}{fmts}{show}) }})",
            names(&endog),
            r_text(&time),
            self.ts_delta.clone().unwrap_or_else(|| "1".to_string()),
            names(&exog),
            r_logical(!opts.has("noconstant", 3)),
        ));
        self.r_new = Some((R_VARSOC, "varsoc".into()));
        Ok(())
    }
}
