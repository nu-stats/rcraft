//! `mvtest` as Stata 19 prints it and the r() it leaves
//! (`plan/stata/mvtest.md`): `means` (Hotelling's T² that the means are equal
//! or zero; by() the four MANOVA statistics, the LR test allowing for
//! heterogeneity), `covariances` (Box's M by(),
//! else the adjusted LR tests that the matrix is diagonal, spherical or
//! compound symmetric), `correlations` (Lawley's test of compound symmetry,
//! Jennrich's of equal matrices by()) and `normality` (Doornik–Hansen,
//! Henze–Zirkler, Mardia's skewness and kurtosis, the univariate and
//! bivariate tables). Rows with any of the variables (or the by() variable)
//! missing are left out.

use super::*;

pub(super) const MVTEST_HELPER: &str = r##"stata_mv_num <- function(u) { u <- unclass(u); attributes(u) <- NULL; as.numeric(u) }
# a number of degrees of freedom as mvtest writes it in a label: %9.1f, a trailing .0 dropped
stata_mv_df <- function(v) sub("\\.0$", "", trimws(sprintf("%9.1f", v)))
stata_mv_line <- function(label, v, f = "%9.2f") cat(sprintf("%22s = ", label), sprintf(f, v), "\n", sep = "")
# Stata's stop when a matrix is singular, r(506)
stata_mv_singular <- function(S, msg) {
  e <- eigen(S, symmetric = TRUE, only.values = TRUE)$values
  if (!all(is.finite(e)) || min(e) <= 1e-12 * max(abs(e), 1e-300)) stop(msg, " (Stata: r(506))", call. = FALSE)
}
# a generalized inverse of a symmetric matrix as Stata's invsym() makes it: the columns taken in
# order, one that depends on those before it dropped (zero in the inverse); and the rank
stata_mv_ginv <- function(A) {
  keep <- integer(0)
  for (j in seq_len(ncol(A))) {
    r <- A[j, j]
    if (length(keep)) r <- r - drop(A[j, keep] %*% solve(A[keep, keep, drop = FALSE], A[keep, j]))
    if (A[j, j] > 0 && r > 1e-13 * A[j, j]) keep <- c(keep, j)
  }
  inv <- matrix(0, nrow(A), ncol(A))
  if (length(keep)) inv[keep, keep] <- solve(A[keep, keep, drop = FALSE])
  list(inv = inv, rank = length(keep))
}
stata_mv_groups <- function(X, g) {
  lv <- sort(unique(g))
  list(lv = lv, X = lapply(lv, function(l) X[g == l, , drop = FALSE]), n = vapply(lv, function(l) sum(g == l), 1))
}
stata_mv_manova <- function(X, gs) {
  N <- nrow(X); p <- ncol(X); k <- length(gs$lv); m <- colMeans(X)
  W <- Reduce(`+`, lapply(gs$X, function(Y) crossprod(sweep(Y, 2, colMeans(Y)))))
  stata_mv_singular(W, "collinearity among the dependent variables")
  B <- Reduce(`+`, lapply(gs$X, function(Y) { dd <- colMeans(Y) - m; nrow(Y) * tcrossprod(dd) }))
  lam <- Re(eigen(solve(W, B), only.values = TRUE)$values)
  q <- k - 1; ve <- N - k; s <- min(p, q); mm <- (abs(p - q) - 1) / 2; nn <- (ve - p - 1) / 2
  L <- det(W) / det(W + B)
  t <- if (p^2 + q^2 - 5 > 0) sqrt((p^2 * q^2 - 4) / (p^2 + q^2 - 5)) else 1
  d1 <- p * q; d2 <- t * (ve - (p - q + 1) / 2) - (p * q - 2) / 2
  Fw <- (1 - L^(1 / t)) / L^(1 / t) * d2 / d1
  V <- sum(lam / (1 + lam)); pd1 <- s * (2 * mm + s + 1); pd2 <- s * (2 * nn + s + 1)
  Fp <- pd2 / pd1 * V / (s - V)
  U <- sum(lam); ld2 <- 2 * (s * nn + 1); Fl <- ld2 * U / (s * pd1)
  th <- max(lam); r <- max(p, q); rd2 <- ve - r + q; Fr <- th * rd2 / r
  st <- rbind(c(L, Fw, d1, d2), c(V, Fp, pd1, pd2), c(U, Fl, pd1, ld2), c(th, Fr, r, rd2))
  st <- cbind(st, stats::pf(st[, 2], st[, 3], st[, 4], lower.tail = FALSE), c(min(p, q) <= 2, s == 1, s == 1, s == 1) + 0)
  dimnames(st) <- list(c("Wilks", "Pillai", "Lawley", "Roy"), c("statistic", "F", "df1", "df2", "pvalue", "exact"))
  st
}
# the common mean of groups whose covariances differ, by maximum likelihood, and the LR test
stata_mv_lr <- function(X, gs) {
  mi <- lapply(gs$X, colMeans)
  Si <- lapply(gs$X, function(Y) crossprod(sweep(Y, 2, colMeans(Y))) / nrow(Y))
  for (S in Si) stata_mv_singular(S, "singular covariance matrix")
  mu <- colMeans(X)
  for (it in 1:1000) {
    Ii <- Map(function(S, m) solve(S + tcrossprod(m - mu)), Si, mi)
    A <- Reduce(`+`, Map(`*`, Ii, gs$n))
    b <- Reduce(`+`, Map(function(I, m, n) n * I %*% m, Ii, mi, gs$n))
    new <- drop(solve(A, b))
    dd <- max(abs(new - mu))
    mu <- new
    if (dd < 1e-15 * max(1, max(abs(mu)))) break
  }
  lr <- sum(mapply(function(S, m, n) n * log(det(S + tcrossprod(m - mu)) / det(S)), Si, mi, gs$n))
  list(chi2 = lr, mu = mu)
}
# Doornik and Hansen's z1 (skewness) and z2 (kurtosis) of one variable
stata_mv_dhz <- function(y) {
  n <- length(y); dv <- y - mean(y); m2 <- mean(dv^2)
  sk <- mean(dv^3) / m2^1.5; b2 <- mean(dv^4) / m2^2; b1 <- sk^2
  beta <- 3 * (n^2 + 27 * n - 70) * (n + 1) * (n + 3) / ((n - 2) * (n + 5) * (n + 7) * (n + 9))
  w2 <- -1 + sqrt(2 * (beta - 1)); del <- 1 / sqrt(log(sqrt(w2)))
  yy <- sk * sqrt((w2 - 1) * (n + 1) * (n + 3) / (12 * (n - 2)))
  z1 <- del * log(yy + sqrt(yy^2 + 1))
  dd <- (n - 3) * (n + 1) * (n^2 + 15 * n - 4)
  a <- (n - 2) * (n + 5) * (n + 7) * (n^2 + 27 * n - 70) / (6 * dd)
  cc <- (n - 7) * (n + 5) * (n + 7) * (n^2 + 2 * n - 5) / (6 * dd)
  kk <- (n + 5) * (n + 7) * (n^3 + 37 * n^2 + 11 * n - 313) / (12 * dd)
  al <- a + b1 * cc; chi <- (b2 - 1 - b1) * 2 * kk
  z2 <- ((chi / (2 * al))^(1 / 3) - 1 + 1 / (9 * al)) * sqrt(9 * al)
  c(b1 = sk, b2 = b2, z1 = z1, z2 = z2)
}
stata_mv_dh <- function(X) {
  Xc <- sweep(X, 2, colMeans(X)); S <- stats::cov(X); V <- diag(1 / sqrt(diag(S)), ncol(X))
  e <- eigen(V %*% S %*% V, symmetric = TRUE)
  Tm <- e$vectors %*% diag(1 / sqrt(e$values), ncol(X)) %*% t(e$vectors)
  U <- t(apply(Xc %*% V %*% Tm, 2, stata_mv_dhz))
  dimnames(U) <- list(colnames(X), c("b1", "b2", "z1", "z2"))
  list(U = U, chi2 = sum(U[, 3:4]^2), df = 2 * ncol(X))
}
stata_mvtest <- function(d, sub, vars, by = NULL, zero = FALSE, lr = FALSE, hyp = "diagonal", missing = FALSE,
                         stats = "dh", univariate = FALSE, bivariate = FALSE, show = TRUE) {
  for (v in c(vars, by)) if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
  X <- vapply(vars, function(v) stata_mv_num(d[[v]]), numeric(nrow(d)))
  if (!is.matrix(X)) X <- matrix(X, ncol = length(vars))
  colnames(X) <- vars
  ok <- stats::complete.cases(X)
  if (!is.null(by)) {
    g <- d[[by]]
    if (is.character(g)) {
      gok <- g != ""
    } else {
      g <- stata_mv_num(g)
      gok <- !is.na(g)
      # missing: the missing values of by() a group of their own (after the others)
      if (missing) { g[!gok] <- Inf; gok[] <- TRUE }
    }
    ok <- ok & (gok | missing)
    g <- g[ok]
  }
  X <- X[ok, , drop = FALSE]
  N <- nrow(X); p <- ncol(X)
  if (N == 0) stop("no observations (Stata: r(2000))", call. = FALSE)
  pchi <- function(x, df) stats::pchisq(x, df, lower.tail = FALSE)
  gs <- if (!is.null(by)) stata_mv_groups(X, g) else NULL
  if (sub == "means") {
    if (is.null(by)) {
      if (!zero && p < 2) stop("too few variables specified (Stata: r(102))", call. = FALSE)
      m <- colMeans(X); S <- stats::cov(X)
      # collinear: the test on as many contrasts as are free, as in Stata (where the rows are too
      # few for the contrasts, which ones Stata keeps was not worked out)
      if (N - 1 < (if (zero) p else p - 1))
        stop("mvtest means with fewer rows than contrasts of the variables is not translated yet", call. = FALSE)
      if (zero) {
        A <- stata_mv_ginv(S); cm <- m
      } else {
        C <- cbind(-1, diag(p - 1)); cm <- drop(C %*% m); A <- stata_mv_ginv(C %*% S %*% t(C))
      }
      T2 <- N * drop(t(cm) %*% A$inv %*% cm); q <- A$rank
      F <- (N - q) / (q * (N - 1)) * T2
      rr <- list(p_F = stats::pf(F, q, N - q, lower.tail = FALSE), df_r = N - q, df_m = q, F = F, T2 = T2, Ftype = "Hotelling")
      if (!show) return(rr)
      cat("\nTest that all means ", if (zero) "are 0" else "are the same", "\n\n", sep = "")
      stata_mv_line("Hotelling T2", T2)
      stata_mv_line(paste0("Hotelling F(", q, ",", N - q, ")"), F)
      stata_mv_line("Prob > F", rr$p_F, "%9.4f")
      return(invisible(rr))
    }
    k <- length(gs$lv)
    if (lr) {
      o <- stata_mv_lr(X, gs); df <- p * (k - 1)
      M <- matrix(o$mu, ncol = 1, dimnames = list(vars, "means"))
      rr <- list(p_chi2 = pchi(o$chi2, df), df = df, chi2 = o$chi2, rc = 0, chi2type = "LR", M = M)
      if (!show) return(rr)
      cat("\nTest for equality of ", k, " group means, allowing for heterogeneity\n\n", sep = "")
      stata_mv_line(paste0("LR chi2(", df, ")"), o$chi2)
      stata_mv_line("Prob > chi2", rr$p_chi2, "%9.4f")
      return(invisible(rr))
    }
    st <- stata_mv_manova(X, gs)
    rr <- list(p_F = st[1, 5], df_r = st[1, 4], df_m = st[1, 3], F = st[1, 2], Ftype = "Wilks/LR", stat_m = st)
    if (!show) return(rr)
    cat("\nTest for equality of ", k, " group means, assuming homogeneity\n\n", sep = "")
    cat("                            | Statistic   F(df1,     df2)   =  F      Prob>F\n")
    cat("     -----------------------+-----------------------------------------------\n")
    lab <- c("Wilks' lambda", "Pillai's trace", "Lawley-Hotelling trace", "Roy's largest root")
    for (i in 1:4) {
      flag <- if (st[i, 6] == 1) "e" else if (i == 4) "u" else "a"
      cat(sprintf("%27s |%10.4f%8.1f%10.1f%10.2f%9.4f %s", lab[i], st[i, 1], st[i, 3], st[i, 4], st[i, 2], st[i, 5], flag), "\n", sep = "")
    }
    cat("     -----------------------------------------------------------------------\n")
    cat("                            e = exact, a = approximate, u = upper bound on F\n")
    return(invisible(rr))
  }
  if (sub == "covariances") {
    if (!is.null(by)) {
      k <- length(gs$lv)
      Si <- lapply(gs$X, stats::cov); ni <- gs$n
      for (S in Si) stata_mv_singular(S, "sample covariance matrix is singular")
      Sp <- Reduce(`+`, Map(`*`, Si, ni - 1)) / (N - k)
      M <- (N - k) * log(det(Sp)) - sum((ni - 1) * vapply(Si, function(S) log(det(S)), 1))
      c1 <- (sum(1 / (ni - 1)) - 1 / (N - k)) * (2 * p^2 + 3 * p - 1) / (6 * (p + 1) * (k - 1))
      c2 <- (sum(1 / (ni - 1)^2) - 1 / (N - k)^2) * (p - 1) * (p + 2) / (6 * (k - 1))
      d1 <- p * (p + 1) * (k - 1) / 2; d2 <- (d1 + 2) / (c2 - c1^2)
      F <- M * (1 - c1 - d1 / d2) / d1; chi2 <- M * (1 - c1)
      rr <- list(p_chi2 = pchi(chi2, d1), df = d1, chi2 = chi2, p_F_Box = stats::pf(F, d1, d2, lower.tail = FALSE),
                 df_r_Box = d2, df_m_Box = d1, F_Box = F, chi2type = "Box")
      if (!show) return(rr)
      cat("\nTest of equality of covariance matrices across ", k, " samples\n\n", sep = "")
      cat(sprintf("%22s = %9s", "Modified LR chi2", stata_fmt(M, "%9.0g")), "\n", sep = "")
      cat(sprintf("%22s = %9.2f%14s = %7.4f", paste0("Box F(", stata_mv_df(d1), ",", stata_mv_df(d2), ")"), F, "Prob > F", rr$p_F_Box), "\n", sep = "")
      cat(sprintf("%22s = %9.2f%14s = %7.4f", paste0("Box chi2(", stata_mv_df(d1), ")"), chi2, "Prob > chi2", rr$p_chi2), "\n", sep = "")
      return(invisible(rr))
    }
    S <- stats::cov(X)
    stata_mv_singular(S, "sample covariance matrix is singular")
    if (hyp == "diagonal") {
      chi2 <- -(N - 1 - (2 * p + 5) / 6) * log(det(stats::cov2cor(S))); df <- p * (p - 1) / 2
    } else if (hyp == "spherical") {
      chi2 <- -(N - 1 - (2 * p^2 + p + 2) / (6 * p)) * (log(det(S)) - p * log(sum(diag(S)) / p)); df <- p * (p + 1) / 2 - 1
    } else {
      s2 <- mean(diag(S)); r <- (sum(S) - sum(diag(S))) / (p * (p - 1)) / s2
      chi2 <- -(N - 1 - p * (p + 1)^2 * (2 * p - 3) / (6 * (p - 1) * (p^2 + p - 4))) *
        log(det(S) / (s2^p * (1 - r)^(p - 1) * (1 + (p - 1) * r)))
      df <- p * (p + 1) / 2 - 2
    }
    rr <- list(p_chi2 = pchi(chi2, df), df = df, chi2 = chi2, chi2type = "Adjusted LR")
    if (!show) return(rr)
    cat("\nTest that covariance matrix is ", if (hyp == "compound") "compound symmetric" else hyp, "\n\n", sep = "")
    stata_mv_line(paste0("Adjusted LR chi2(", df, ")"), chi2)
    stata_mv_line("Prob > chi2", rr$p_chi2, "%9.4f")
    return(invisible(rr))
  }
  if (sub == "correlations") {
    if (!is.null(by)) {
      k <- length(gs$lv); ni <- gs$n
      Ri <- lapply(gs$X, stats::cor)
      for (R in Ri) stata_mv_singular(R, "sample correlation matrix is singular")
      Rb <- Reduce(`+`, Map(`*`, Ri, ni)) / sum(ni); Ri1 <- solve(Rb)
      Si <- solve(diag(p) + Rb * Ri1)
      chi2 <- sum(mapply(function(R, n) {
        Z <- sqrt(n) * Ri1 %*% (R - Rb); dz <- diag(Z)
        sum(diag(Z %*% Z)) / 2 - drop(t(dz) %*% Si %*% dz)
      }, Ri, ni))
      df <- (k - 1) * p * (p - 1) / 2
      rr <- list(p_chi2 = pchi(chi2, df), df = df, chi2 = chi2, chi2type = "Jennrich")
      if (!show) return(rr)
      cat("\nTest of equality of correlation matrices across samples\n\n")
      stata_mv_line(paste0("Jennrich chi2(", df, ")"), chi2)
      stata_mv_line("Prob > chi2", rr$p_chi2, "%9.4f")
      return(invisible(rr))
    }
    if (p < 3) stop("compound symmetry test requires at least three variables (Stata: r(102))", call. = FALSE)
    R <- stats::cor(X); lo <- R[lower.tri(R)]
    stata_mv_singular(R, "sample correlation matrix is singular")
    rb <- mean(lo); rk <- (colSums(R) - 1) / (p - 1)
    mu <- (p - 1)^2 * (1 - (1 - rb)^2) / (p - (p - 2) * (1 - rb)^2)
    chi2 <- (N - 1) / (1 - rb)^2 * (sum((lo - rb)^2) - mu * sum((rk - rb)^2))
    df <- (p + 1) * (p - 2) / 2
    rr <- list(p_chi2 = pchi(chi2, df), df = df, chi2 = chi2, chi2type = "Lawley")
    if (!show) return(rr)
    cat("\nTest that correlation matrix is compound symmetric (all correlations equal)\n\n")
    stata_mv_line(paste0("Lawley chi2(", df, ")"), chi2)
    stata_mv_line("Prob > chi2", rr$p_chi2, "%9.4f")
    return(invisible(rr))
  }
  # normality
  stata_mv_singular(stats::cov(X), "sample covariance matrix is singular")
  all <- "all" %in% stats
  rr <- list()
  dh <- NULL
  if (all || "dh" %in% stats) {
    dh <- stata_mv_dh(X)
    rr <- c(rr, list(p_dh = pchi(dh$chi2, dh$df), df_dh = dh$df, chi2_dh = dh$chi2))
  }
  Xc <- sweep(X, 2, colMeans(X)); D <- Xc %*% solve(crossprod(Xc) / N) %*% t(Xc)
  if (all || "hz" %in% stats) {
    Dj <- diag(D); Djk <- outer(Dj, Dj, `+`) - 2 * D
    b <- 1 / sqrt(2) * ((2 * p + 1) / 4)^(1 / (p + 4)) * N^(1 / (p + 4))
    hz <- N * (sum(exp(-b^2 / 2 * Djk)) / N^2 - 2 * (1 + b^2)^(-p / 2) / N * sum(exp(-b^2 / (2 * (1 + b^2)) * Dj)) + (1 + 2 * b^2)^(-p / 2))
    wb <- (1 + b^2) * (1 + 3 * b^2); a <- 1 + 2 * b^2
    E <- 1 - a^(-p / 2) * (1 + p * b^2 / a + p * (p + 2) * b^4 / (2 * a^2))
    V <- 2 * (1 + 4 * b^2)^(-p / 2) + 2 * a^(-p) * (1 + 2 * p * b^4 / a^2 + 3 * p * (p + 2) * b^8 / (4 * a^4)) -
      4 * wb^(-p / 2) * (1 + 3 * p * b^4 / (2 * wb) + p * (p + 2) * b^8 / (2 * wb^2))
    z <- (log(hz) - log(sqrt(E^4 / (V + E^2)))) / sqrt(log((V + E^2) / E^2))
    rr <- c(rr, list(rank_hz = p, p_hz = pchi(z^2, 1), z_hz = z, V_hz = V, E_hz = E, hz = hz))
  }
  if (all || "kurtosis" %in% stats) {
    b2 <- mean(diag(D)^2); zk <- (b2 - p * (p + 2)) / sqrt(8 * p * (p + 2) / N)
    rr <- c(rr, list(rank_mkurt = p, p_mkurt = pchi(zk^2, 1), chi2_mkurt = zk^2, z_mkurt = zk, mkurt = b2))
  }
  if (all || "skewness" %in% stats) {
    # Mardia's skewness with the small-sample factor
    b1 <- sum(D^3) / N^2
    ck <- (p + 1) * (N + 1) * (N + 3) / (N * ((N + 1) * (p + 1) - 6))
    dfs <- p * (p + 1) * (p + 2) / 6; cs <- N * b1 / 6 * ck
    rr <- c(rr, list(rank_mskew = p, p_mskew = pchi(cs, dfs), df_mskew = dfs, chi2_mskew = cs, mskew = b1))
  }
  if (!is.null(dh)) rr$U_dh <- dh$U
  pairs <- if (bivariate && p >= 2) utils::combn(p, 2) else NULL
  if (!is.null(pairs)) {
    Bt <- t(apply(pairs, 2, function(ij) { o <- stata_mv_dh(X[, ij, drop = FALSE]); c(o$chi2, o$df, pchi(o$chi2, o$df)) }))
    dimnames(Bt) <- list(apply(pairs, 2, function(ij) paste(vars[ij], collapse = ":")), c("chi2", "df", "p"))
    rr$Btest <- Bt
  }
  if (univariate) {
    Ut <- stata_sktest(as.data.frame(X), vars, show = FALSE)$table[, -1, drop = FALSE]
    dimnames(Ut) <- list(vars, c("P_skew", "P_kurt", "chi2", "P_chi2"))
    rr$Utest <- Ut
  }
  if (!show) return(rr)
  if (univariate) {
    cat("\nTest for univariate normality\n\n")
    cat("    ---------------------------------------------------------------------\n")
    cat("                 |                                   ------- joint ------\n")
    cat("        Variable |  Pr(Skewness)   Pr(Kurtosis)  adj chi2(2)    Prob>chi2\n")
    cat("    -------------+-------------------------------------------------------\n")
    for (i in seq_len(p)) cat(sprintf("    %12s |%11.4f%15.4f%15.2f%13.4f", stata_tab_abbrev(vars[i], 12), Ut[i, 1], Ut[i, 2], Ut[i, 3], Ut[i, 4]), "\n", sep = "")
    cat("    ---------------------------------------------------------------------\n")
  }
  if (!is.null(pairs)) {
    cat("\nDoornik-Hansen test for bivariate normality\n\n")
    cat("    --------------------------------------------------------\n")
    cat("    Pair of variables          |      chi2    df   Prob>chi2\n")
    cat("    ---------------------------+----------------------------\n")
    for (j in seq_len(ncol(pairs))) {
      i1 <- pairs[1, j]
      first <- if (j == 1 || pairs[1, j - 1] != i1) stata_tab_abbrev(vars[i1], 12) else ""
      cat(sprintf("    %-14s%-13s|%10.2f%6.0f%11.4f", first, stata_tab_abbrev(vars[pairs[2, j]], 12), Bt[j, 1], Bt[j, 2], Bt[j, 3]), "\n", sep = "")
    }
    cat("    --------------------------------------------------------\n")
  }
  cat("\nTest for multivariate normality\n\n")
  row <- function(label, v, df, chi2, pv) {
    cat(sprintf("    %-16s %s %9s%12s = %8.3f   Prob>chi2 = %7.4f", label, if (is.na(v)) " " else "=",
                if (is.na(v)) "" else stata_fmt(v, "%9.0g"), paste0("chi2(", df, ")"), chi2, pv), "\n", sep = "")
  }
  if (!is.null(rr$mskew)) row("Mardia mSkewness", rr$mskew, rr$df_mskew, rr$chi2_mskew, rr$p_mskew)
  if (!is.null(rr$mkurt)) row("Mardia mKurtosis", rr$mkurt, 1, rr$chi2_mkurt, rr$p_mkurt)
  if (!is.null(rr$hz)) row("Henze-Zirkler", rr$hz, 1, rr$z_hz^2, rr$p_hz)
  if (!is.null(dh)) row("Doornik-Hansen", NA, rr$df_dh, rr$chi2_dh, rr$p_dh)
  invisible(rr)
}"##;

/// The r() mvtest leaves (Stata 19), each form's in Stata's order; a name a
/// form does not leave is missing, as in Stata.
const R_MVTEST: &[&str] = &[
    "p_F",
    "df_r",
    "df_m",
    "F",
    "T2",
    "Ftype",
    "stat_m",
    "p_chi2",
    "df",
    "chi2",
    "rc",
    "chi2type",
    "M",
    "p_F_Box",
    "df_r_Box",
    "df_m_Box",
    "F_Box",
    "p_dh",
    "df_dh",
    "chi2_dh",
    "rank_hz",
    "p_hz",
    "z_hz",
    "V_hz",
    "E_hz",
    "hz",
    "rank_mkurt",
    "p_mkurt",
    "chi2_mkurt",
    "z_mkurt",
    "mkurt",
    "rank_mskew",
    "p_mskew",
    "df_mskew",
    "chi2_mskew",
    "mskew",
    "U_dh",
    "Btest",
    "Utest",
];

/// Whether `word` is `full` or a short form of it at least `min` long.
fn short(word: &str, full: &str, min: usize) -> bool {
    word.len() >= min && full.starts_with(word)
}

/// The R that stops where Stata stops with `message`, r(`rc`).
fn stops(message: &str, rc: u32) -> String {
    format!(
        "stop({}, call. = FALSE)",
        r_text(&format!("{message} (Stata: r({rc}))"))
    )
}

impl Session {
    /// `mvtest means|covariances|correlations|normality varlist [if] [in] [, options]`.
    pub(super) fn mvtest(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "mvtest")?;
        if c.parts.weight.is_some() {
            return Err(no("mvtest with weights is not translated yet"));
        }
        let mut words = c.parts.main.split_whitespace();
        let Some(first) = words.next() else {
            out.line(stops("subcommand and varlist required", 198));
            return Ok(());
        };
        let sub = if short(first, "means", 1) {
            "means"
        } else if short(first, "covariances", 3) {
            "covariances"
        } else if short(first, "correlations", 4) {
            "correlations"
        } else if short(first, "normality", 4) {
            "normality"
        } else {
            out.line(stops(
                &format!("unknown subcommand of mvtest: {first}"),
                198,
            ));
            return Ok(());
        };
        let vars: Vec<&str> = words.collect();
        if vars.is_empty() {
            out.line(stops("varlist required", 100));
            return Ok(());
        }
        if let Some(v) = vars.iter().find(|v| !is_name(v)) {
            return Err(no(format!(
                "mvtest {sub} {v}: the variables written out are translated"
            )));
        }
        let by = opts.arg("by", 2).map(|g| g.trim().to_string());
        if let Some(g) = &by {
            if !is_name(g) {
                return Err(no("mvtest's by(): one variable is translated"));
            }
        }
        // the options each subcommand takes in Stata 19, at their shortest; any
        // other is Stata's error, r(198)
        let valid: &[(&str, usize)] = match sub {
            "means" => &[
                ("equal", 1),
                ("zero", 1),
                ("by", 2),
                ("heterogeneous", 3),
                ("lr", 2),
                ("protect", 4),
                ("missing", 4),
            ],
            "covariances" => &[
                ("by", 2),
                ("diagonal", 4),
                ("spherical", 3),
                ("compound", 4),
                ("missing", 4),
            ],
            "correlations" => &[("by", 2), ("compound", 4), ("missing", 4)],
            _ => &[("stats", 2), ("univariate", 3), ("bivariate", 3)],
        };
        // and those it takes only without by() (means: lr only with it)
        let wrong: &[(&str, usize)] = match (sub, by.is_some()) {
            ("means", true) => &[("equal", 1), ("zero", 1)],
            ("means", false) => &[("lr", 2)],
            ("covariances", true) => &[("diagonal", 4), ("spherical", 3), ("compound", 4)],
            ("correlations", true) => &[("compound", 4)],
            _ => &[],
        };
        if let Some(o) = opts.0.iter().find(|o| {
            !valid.iter().any(|(f, m)| o.is(f, *m)) || wrong.iter().any(|(f, m)| o.is(f, *m))
        }) {
            let paren = if o.arg.is_some() { "()" } else { "" };
            out.line(stops(&format!("option {}{paren} not allowed", o.name), 198));
            return Ok(());
        }
        // heterogeneous: for more than two groups Stata 19 takes the p-value of
        // James' (1954) approximation from its own search, a few 1e-6 off the root
        // (two groups get the MNV F, which the group count picks only once the data
        // are known); protect(): Stata's runs from random starts
        if opts.has("heterogeneous", 3) {
            return Err(no(
                "mvtest means, heterogeneous is not translated yet: for more than two groups Stata 19 takes the p-value of James' approximation from its own search, a few 1e-6 off the exact root",
            ));
        }
        if opts.has("protect", 4) {
            return Err(no("mvtest means with protect() is not translated yet"));
        }
        let missing = opts.has("missing", 4);
        if missing && by.is_none() {
            return Err(no(format!(
                "mvtest {sub}, missing without by() is not translated yet"
            )));
        }
        let mut args = String::new();
        if missing {
            args.push_str(", missing = TRUE");
        }
        match sub {
            "means" => {
                if opts.has("equal", 1) && opts.has("zero", 1) {
                    out.line(stops("exactly one hypothesis should be specified", 198));
                    return Ok(());
                }
                if opts.has("zero", 1) {
                    args.push_str(", zero = TRUE");
                }
                if opts.has("lr", 2) {
                    args.push_str(", lr = TRUE");
                }
            }
            "covariances" => {
                let hyps: Vec<&str> = [("diagonal", 4), ("spherical", 3), ("compound", 4)]
                    .into_iter()
                    .filter(|(o, m)| opts.has(o, *m))
                    .map(|(o, _)| o)
                    .collect();
                if hyps.len() > 1 {
                    out.line(stops("only one hypothesis may be specified", 198));
                    return Ok(());
                }
                if let Some(h) = hyps.first() {
                    args.push_str(&format!(", hyp = {}", r_text(h)));
                }
            }
            "correlations" => {}
            _ => {
                // stats(): dhansen, hzirkler, skewness and kurtosis from two letters, all
                let mut stats: Vec<&str> = Vec::new();
                for w in opts.arg("stats", 2).unwrap_or_default().split_whitespace() {
                    let s = if short(w, "dhansen", 2) {
                        "dh"
                    } else if short(w, "hzirkler", 2) {
                        "hz"
                    } else if short(w, "skewness", 2) {
                        "skewness"
                    } else if short(w, "kurtosis", 2) {
                        "kurtosis"
                    } else if w == "all" {
                        "all"
                    } else {
                        out.line(stops(&format!("option {w} not allowed"), 198));
                        return Ok(());
                    };
                    stats.push(s);
                }
                if stats.is_empty() {
                    stats.push("dh");
                }
                let q: Vec<String> = stats.iter().map(|s| r_text(s)).collect();
                args.push_str(&format!(", stats = c({})", q.join(", ")));
                if opts.has("univariate", 3) {
                    args.push_str(", univariate = TRUE");
                    self.helpers.insert("stata_sktest");
                }
                if opts.has("bivariate", 3) {
                    args.push_str(", bivariate = TRUE");
                }
            }
        }
        let rows = self.rows_for(c, out)?;
        let list: Vec<String> = vars.iter().map(|v| r_text(v)).collect();
        if let Some(g) = &by {
            args.push_str(&format!(", by = {}", r_text(g)));
        }
        for h in [
            "stata_mvtest",
            "stata_fmt",
            "stata_tables",
            "stata_returned",
        ] {
            self.helpers.insert(h);
        }
        out.line(format!(
            "stata_r <- stata_mvtest({rows}, {}, c({}){args}{})",
            r_text(sub),
            list.join(", "),
            if self.quiet(c) { ", show = FALSE" } else { "" }
        ));
        self.r_new = Some((R_MVTEST, "mvtest".into()));
        Ok(())
    }
}
