# canon (varlist1) (varlist2): canonical correlations, coefficients, loadings and the four tests
# (plan/stata/multivariate.md). Needs stata_manova's stata_mn_four.
stata_canon <- function(d, v1, v2, lc = NULL, first = NULL, stdcoef = FALSE, cmdline = "", show = TRUE) {
  vars <- c(v1, v2)
  for (v in vars) {
    if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
    if (is.character(d[[v]])) stop(v, ": string variable, type mismatch (Stata: r(109))", call. = FALSE)
  }
  X <- vapply(vars, function(v) stata_mn_num(d[[v]]), numeric(nrow(d)))
  X <- matrix(X, nrow(d), length(vars), dimnames = list(NULL, vars))
  X <- X[stats::complete.cases(X), , drop = FALSE]
  n <- nrow(X)
  if (n < 3) stop("insufficient observations (Stata: r(2001))", call. = FALSE)
  pa <- length(v1); pb <- length(v2); k <- pa + pb
  S <- stats::cov(X)
  dimnames(S) <- list(vars, vars)
  S11 <- S[v1, v1, drop = FALSE]; S22 <- S[v2, v2, drop = FALSE]; S12 <- S[v1, v2, drop = FALSE]; S21 <- t(S12)
  for (M in list(S11, S22)) {
    ev <- eigen(M, symmetric = TRUE, only.values = TRUE)$values
    if (min(ev) <= 1e-12 * max(ev)) stop("matrix not positive definite (Stata: r(506))", call. = FALSE)
  }
  I22 <- solve(S22)
  e11 <- eigen(S11, symmetric = TRUE)
  Hi <- e11$vectors %*% diag(1 / sqrt(e11$values), nrow = pa) %*% t(e11$vectors)
  M <- Hi %*% S12 %*% I22 %*% S21 %*% Hi
  em <- eigen((M + t(M)) / 2, symmetric = TRUE)
  ncc <- min(pa, pb)
  cc <- sqrt(pmax(em$values[seq_len(ncc)], 0))
  raw1 <- matrix(0, pa, ncc); raw2 <- matrix(0, pb, ncc)
  for (j in seq_len(ncc)) {
    a <- drop(Hi %*% em$vectors[, j])
    sg <- if (a[1] < 0) -1 else 1
    raw1[, j] <- sg * a
    raw2[, j] <- sg * drop(I22 %*% S21 %*% a) / cc[j]
  }
  cn <- as.character(seq_len(ncc))
  dimnames(raw1) <- list(v1, cn); dimnames(raw2) <- list(v2, cn)
  sd1 <- sqrt(diag(S11)); sd2 <- sqrt(diag(S22))
  std1 <- raw1 * sd1; std2 <- raw2 * sd2
  Bm <- I22 %*% S21
  H <- (S12 %*% Bm) * (n - 1)
  E <- S11 * (n - 1) - H
  four <- stata_mn_four(E, H, pb, n - 1 - pb)
  pick <- if (!is.null(lc) && lc >= 1 && lc <= ncc) lc else if (!is.null(first)) seq_len(min(first, ncc)) else seq_len(ncc)
  if (show) {
    tab <- function(m) {
      t <- formatC(m[, pick, drop = FALSE], format = "f", digits = 4, width = 9)
      dimnames(t) <- list(rownames(m), as.character(pick))
      print(t, quote = FALSE)
    }
    cat(sprintf("\nCanonical correlation analysis%22sNumber of obs = %10d\n\n", "", n))
    cat(if (stdcoef) "Standardized" else "Raw", "coefficients for the first variable set\n")
    tab(if (stdcoef) std1 else raw1)
    cat(if (stdcoef) "\nStandardized" else "\nRaw", "coefficients for the second variable set\n")
    tab(if (stdcoef) std2 else raw2)
    cat("\nCanonical correlations:\n", paste(sprintf("%8.4f", cc), collapse = ""), "\n", sep = "")
    cat("\nTests of significance of all canonical correlations\n\n")
    cat("                         Statistic      df1      df2            F     Prob>F\n")
    lab <- c("Wilks' lambda", "Pillai's trace", "Lawley-Hotelling trace", "Roy's largest root")
    for (i in 1:4) {
      st <- four$st[i, ]
      cat(sprintf("%22s%12s%9s%9s%13.4f%11.4f %s\n", lab[i], stata_fmt(st[1], "%8.0g", trim = TRUE),
                  stata_fmt(st[3], "%9.0g", trim = TRUE), stata_fmt(st[4], "%9.0g", trim = TRUE), st[2], st[5],
                  if (st[6] == 1) "e" else if (i == 4) "u" else "a"))
    }
    cat("                            e = exact, a = approximate, u = upper bound on F\n")
  }
  total <- length(pick) * k
  b <- numeric(0); nm <- character(0)
  for (j in pick) {
    b <- c(b, raw1[, j], raw2[, j])
    nm <- c(nm, paste0("u", j, ":", v1), paste0("v", j, ":", v2))
  }
  E_ <- list(df_r = n - 1, N = n, df = n - 1, n_cc = length(pick), df1 = four$st[1, 3], df2 = four$st[1, 4], rank = total,
             cmdline = cmdline, marginsnotok = "_ALL", cmd = "canon", predict = "canon_p", estat_cmd = "canon_estat",
             properties = "b V")
  E_$b <- matrix(b, 1, dimnames = list("y1", nm))
  E_$V <- matrix(0, total, total, dimnames = list(nm, nm))
  for (j in rev(seq_len(ncc))) {
    lam <- prod(1 - cc[j:ncc]^2)
    pp <- pa - j + 1; qq <- pb - j + 1
    t <- if (pp^2 + qq^2 - 5 > 0) sqrt((pp^2 * qq^2 - 4) / (pp^2 + qq^2 - 5)) else 1
    d1 <- pp * qq
    d2 <- (n - 1 - (pp + qq + 1) / 2 - (j - 1)) * t - (d1 - 2) / 2
    root <- lam^(1 / t)
    f <- (1 - root) / root * d2 / d1
    E_[[paste0("stat_", j)]] <- matrix(c(lam, d1, d2, f, stats::pf(f, d1, d2, lower.tail = FALSE), as.numeric(min(pp, qq) <= 2)), 1,
                                       dimnames = list("r1", c("stat", "df1", "df2", "F", "p", "exact")))
  }
  sm <- four$st[, c(1, 3, 4, 2, 5, 6)]
  colnames(sm) <- c("stat", "df1", "df2", "F", "p", "exact")
  E_$stat_m <- sm
  load <- function(Sx, raw, sds, rn) { m <- (Sx %*% raw / sds)[, pick, drop = FALSE]; dimnames(m) <- list(rn, cn[pick]); m }
  E_$canload11 <- load(S11, raw1, sd1, v1)
  E_$canload22 <- load(S22, raw2, sd2, v2)
  E_$canload12 <- load(S12, raw2, sd1, v1)
  E_$canload21 <- load(S21, raw1, sd2, v2)
  E_$rawcoef_var1 <- raw1[, pick, drop = FALSE]; E_$rawcoef_var2 <- raw2[, pick, drop = FALSE]
  E_$stdcoef_var1 <- std1[, pick, drop = FALSE]; E_$stdcoef_var2 <- std2[, pick, drop = FALSE]
  E_$ccorr <- matrix(cc, 1, dimnames = list("r1", cn))
  E_$corr_var1 <- stats::cov2cor(S11); E_$corr_var2 <- stats::cov2cor(S22)
  E_$corr_mixed <- S21 / outer(sd2, sd1)
  attr(E_, "stata_rows") <- NULL
  E_
}
