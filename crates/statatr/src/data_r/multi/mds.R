# mds varlist, id(var) [dimension(#) std]: classical metric scaling of L2 distances
# (plan/stata/multivariate.md). Stata keeps standardized variables as floats.
stata_mds <- function(d, vars, id, dims = 2, std = FALSE, cmdline = "", show = TRUE, idtype = "") {
  for (v in c(vars, id)) {
    if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
    if (is.character(d[[v]])) stop(v, ": string variable, type mismatch (Stata: r(109))", call. = FALSE)
  }
  X <- vapply(vars, function(v) stata_mn_num(d[[v]]), numeric(nrow(d)))
  X <- matrix(X, nrow(d), length(vars), dimnames = list(NULL, vars))
  ids <- stata_mn_num(d[[id]])
  ok <- stats::complete.cases(X) & !is.na(ids)
  rows <- which(ok)
  X <- X[ok, , drop = FALSE]; ids <- ids[ok]
  n <- nrow(X); k <- length(vars)
  if (n < 2) stop("insufficient observations (Stata: r(2001))", call. = FALSE)
  coding <- matrix(c(rep(0, k), rep(1, k)), k, 2, dimnames = list(vars, c("loc", "scale")))
  Z <- X
  if (std) {
    for (j in seq_len(k)) {
      m <- mean(X[, j]); s <- stats::sd(X[, j])
      if (!(s > 0)) s <- 1
      Z[, j] <- readBin(writeBin((X[, j] - m) / s, raw(), size = 4), "numeric", n = n, size = 4)
      coding[j, ] <- c(m, s)
    }
  }
  D2 <- as.matrix(stats::dist(Z))^2
  J <- diag(n) - 1 / n
  Bm <- -0.5 * J %*% D2 %*% J
  Bm <- (Bm + t(Bm)) / 2
  em <- eigen(Bm, symmetric = TRUE)
  ev <- em$values
  npos <- sum(ev > abs(ev[1]) * 1e-10)
  retained <- min(dims, npos)
  abst <- sum(abs(ev)); sqt <- sum(ev^2)
  m1 <- sum(abs(ev[seq_len(retained)])) / abst
  m2 <- sum(ev[seq_len(retained)]^2) / sqt
  dup <- anyDuplicated(ids) > 0
  Y <- em$vectors[, seq_len(retained), drop = FALSE] %*% diag(sqrt(pmax(ev[seq_len(retained)], 0)), nrow = retained)
  # each dimension is signed so that the first observation's coordinate is positive
  for (j in seq_len(retained)) if (Y[1, j] < 0) Y[, j] <- -Y[, j]
  if (show) {
    if (dup) cat("(id() has duplicate values)\n")
    cat("\nClassical metric multidimensional scaling\n")
    cat(sprintf("Dissimilarity: L2, computed on %d variable%s\n\n", k, if (k == 1) "" else "s"))
    cat(sprintf("%45sNumber of obs        = %10d\n", "", n))
    cat(sprintf("    Eigenvalues > 0      = %9d         Mardia fit measure 1 = %10.4f\n", npos, m1))
    cat(sprintf("    Retained dimensions  = %9d         Mardia fit measure 2 = %10.4f\n\n", retained, m2))
    cat("    ", strrep("-", 74), "\n", sep = "")
    cat("                 |                   abs(eigenvalue)          (eigenvalue)^2\n")
    cat("      Dimension  |  Eigenvalue      Percent    Cumul.       Percent    Cumul.\n")
    mid <- paste0("    -------------+", strrep("-", 60))
    cat(mid, "\n", sep = "")
    c1 <- 0; c2 <- 0
    for (i in seq_len(npos)) {
      if (i == retained + 1) cat(mid, "\n", sep = "")
      p1 <- 100 * abs(ev[i]) / abst; p2 <- 100 * ev[i]^2 / sqt
      c1 <- c1 + p1; c2 <- c2 + p2
      cat(sprintf("%15d  |%12s%13.2f%10.2f%14.2f%10.2f\n", i, stata_fmt(ev[i], "%10.0g", trim = TRUE), p1, c1, p2, c2))
    }
    cat("    ", strrep("-", 74), "\n", sep = "")
  }
  idl <- as.character(seq_len(n))
  D <- sqrt(D2); dimnames(D) <- list(idl, idl)
  Dc <- as.matrix(stats::dist(Y))
  lo <- lower.tri(D)
  lf <- if (retained >= 1 && n > 2) unname(stats::coef(stats::lm(Dc[lo] ~ D[lo]))) else c(NA_real_, NA_real_)
  E_ <- list(mardia2 = m2, mardia1 = m1, np = npos, p = retained, addcons = 0, rc = 0, N = n,
             cmdline = cmdline, estat_cmd = "mds_estat", cmd = "mds", marginsnotok = "_ALL", predict = "mds_p",
             dtype = "dissimilarity", dname = "L2", duplicates = if (dup) "1" else "0", idtype = idtype, id = id,
             varlist = paste(vars, collapse = " "), method = "classical", unique = "1",
             norm = "principal", properties = "nob noV eigen")
  E_$idcoding <- matrix(ids, n, 1, dimnames = list(idl, "c1"))
  E_$coding <- coding
  E_$D <- D
  E_$Y <- `dimnames<-`(Y, list(idl, paste0("dim", seq_len(retained))))
  E_$linearf <- matrix(lf, 1, dimnames = list("distance", c("_cons", "dissim")))
  E_$Ev <- matrix(ev, 1, dimnames = list("r1", paste0("e", seq_len(n))))
  attr(E_, "stata_rows") <- rows
  E_
}
