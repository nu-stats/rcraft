# manova depvars = terms: the four multivariate statistics and what Stata leaves in e()
# (plan/stata/multivariate.md). Terms come from the translator as lists of parts, each part
# list(v = name, c = TRUE when continuous).
stata_mn_num <- function(u) { u <- unclass(u); attributes(u) <- NULL; as.numeric(u) }
stata_mn_g <- function(v) stata_fmt(v, "%9.0g", trim = TRUE)
# the four statistics for H against E: Wilks (Rao's F), Pillai, Lawley-Hotelling, Roy (an upper
# bound unless s = 1); returns the eigenvalues of E^-1 H and the 4 x 6 table of e(stat_#)
stata_mn_four <- function(E, H, q, ve) {
  p <- nrow(E)
  lam <- sort(Re(eigen(solve(E, H), only.values = TRUE)$values), decreasing = TRUE)
  lam[lam < 0] <- 0
  s <- min(p, q); mm <- (abs(p - q) - 1) / 2; nn <- (ve - p - 1) / 2
  L <- det(E) / det(E + H)
  t <- if (p^2 + q^2 - 5 > 0) sqrt((p^2 * q^2 - 4) / (p^2 + q^2 - 5)) else 1
  d1 <- p * q; d2 <- t * (ve - (p - q + 1) / 2) - (p * q - 2) / 2
  Fw <- (1 - L^(1 / t)) / L^(1 / t) * d2 / d1
  V <- sum(lam / (1 + lam)); pd1 <- s * (2 * mm + s + 1); pd2 <- s * (2 * nn + s + 1)
  Fp <- pd2 / pd1 * V / (s - V)
  U <- sum(lam); ld2 <- 2 * (s * nn + 1); Fl <- ld2 * U / (s * pd1)
  th <- max(lam); r <- max(p, q); rd2 <- ve - r + q; Fr <- th * rd2 / r
  st <- rbind(c(L, Fw, d1, d2), c(V, Fp, pd1, pd2), c(U, Fl, pd1, ld2), c(th, Fr, r, rd2))
  st <- cbind(st[, c(1, 2, 3, 4)], stats::pf(st[, 2], st[, 3], st[, 4], lower.tail = FALSE),
              c(min(p, q) <= 2, s == 1, s == 1, s == 1) + 0)
  # the columns as e(stat_#) holds them: statistic, F, df1, df2, pvalue, exact
  st <- st[, c(1, 2, 3, 4, 5, 6)]
  dimnames(st) <- list(c("Wilks", "Pillai", "Lawley", "Roy"), c("statistic", "F", "df1", "df2", "pvalue", "exact"))
  list(lam = lam, st = st)
}
# a source's four rows, as the table shows them
stata_mn_block <- function(label, df, st) {
  flag <- function(i) if (st[i, 6] == 1) "e" else if (i == 4) "u" else "a"
  for (i in 1:4) {
    cat(sprintf("%20s |%s%9.4f%10s%9.1f%9.1f%9.2f%7.4f %s\n", if (i == 1) label else "", c("W", "P", "L", "R")[i],
                st[i, 1], if (i == 1) format(df) else "", st[i, 3], st[i, 4], st[i, 2], st[i, 5], flag(i)))
  }
}
stata_manova <- function(d, deps, terms, cmdline = "", show = TRUE) {
  vars <- unique(c(deps, unlist(lapply(terms, function(t) vapply(t, function(p) p$v, "")))))
  for (v in vars) {
    if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
    if (is.character(d[[v]])) stop(v, ": string variable, type mismatch (Stata: r(109))", call. = FALSE)
  }
  num <- lapply(stats::setNames(vars, vars), function(v) stata_mn_num(d[[v]]))
  ok <- Reduce(`&`, lapply(num, function(u) !is.na(u)))
  rows <- which(ok)
  n <- length(rows)
  if (n == 0) stop("no observations (Stata: r(2000))", call. = FALSE)
  Y <- vapply(deps, function(v) num[[v]][rows], numeric(n))
  Y <- matrix(Y, n, length(deps), dimnames = list(NULL, deps))
  q <- length(deps)
  # one part of a term: effects columns, reference columns, their labels and base flags
  part <- function(p, alone) {
    x <- num[[p$v]][rows]
    if (isTRUE(p$c)) {
      m <- matrix(x, n, 1)
      return(list(eff = m, ref = m, lab = if (alone) p$v else paste0("c.", p$v), base = FALSE, fac = FALSE))
    }
    lv <- sort(unique(x)); k <- length(lv)
    if (k < 2) stop(p$v, " has one level (Stata: r(459))", call. = FALSE)
    ind <- vapply(lv, function(l) as.numeric(x == l), numeric(n))
    list(eff = ind[, -k, drop = FALSE] - ind[, k], ref = ind, lab = paste0(format(lv), ".", p$v),
         base = c(TRUE, rep(FALSE, k - 1)), fac = TRUE, lv = format(lv), v = p$v)
  }
  # a term's columns, the earlier parts varying slowest
  build <- function(t) {
    ps <- lapply(t, part, alone = length(t) == 1)
    eff <- matrix(1, n, 1); ref <- matrix(1, n, 1)
    cells <- list(list())
    for (pp in ps) {
      ne <- NULL; nr <- NULL; nc <- list()
      for (i in seq_len(ncol(eff))) for (j in seq_len(ncol(pp$eff))) ne <- cbind(ne, eff[, i] * pp$eff[, j])
      for (i in seq_len(ncol(ref))) for (j in seq_len(ncol(pp$ref))) {
        nr <- cbind(nr, ref[, i] * pp$ref[, j])
        nc <- c(nc, list(c(cells[[i]], list(list(lab = pp$lab[j], base = pp$base[j], fac = pp$fac, lv = if (pp$fac) pp$lv[j] else "", v = if (pp$fac) pp$v else "")))))
      }
      eff <- ne; ref <- nr; cells <- nc
    }
    labs <- vapply(seq_along(cells), function(i) {
      cl <- cells[[i]]
      anyb <- any(vapply(cl, function(c) c$fac && c$base, TRUE))
      paste(vapply(seq_along(cl), function(j) {
        c <- cl[[j]]
        if (!c$fac) return(c$lab)
        flag <- if (c$base) "b" else if (anyb) "o" else ""
        paste0(c$lv, flag, ".", c$v)
      }, ""), collapse = "#")
    }, "")
    keep <- vapply(cells, function(cl) !any(vapply(cl, function(c) c$fac && c$base, TRUE)), TRUE)
    list(eff = eff, ref = ref, names = labs, keep = keep,
         label = paste(vapply(t, function(p) p$v, ""), collapse = "#"))
  }
  bt <- lapply(terms, build)
  one <- matrix(1, n, 1)
  X <- do.call(cbind, c(list(one), lapply(bt, function(b) b$eff)))
  if (qr(X)$rank < ncol(X)) stop("collinear terms (Stata: r(459))", call. = FALSE)
  XtXi <- solve(crossprod(X))
  B <- XtXi %*% crossprod(X, Y)
  Rs <- Y - X %*% B
  E <- crossprod(Rs); dimnames(E) <- list(deps, deps)
  dfm <- ncol(X) - 1; dfr <- n - 1 - dfm
  Yc <- sweep(Y, 2, colMeans(Y))
  Hm <- crossprod(Yc) - E; dimnames(Hm) <- list(deps, deps)
  cnt <- 1; asg <- list()
  for (i in seq_along(bt)) { k <- ncol(bt[[i]]$eff); asg[[i]] <- cnt + seq_len(k); cnt <- cnt + k }
  H <- lapply(seq_along(bt), function(i) {
    k <- asg[[i]]
    h <- t(B[k, , drop = FALSE]) %*% solve(XtXi[k, k, drop = FALSE], B[k, , drop = FALSE])
    dimnames(h) <- list(deps, deps); h
  })
  fm <- stata_mn_four(E, Hm, dfm, dfr)
  ft <- lapply(seq_along(bt), function(i) stata_mn_four(E, H[[i]], length(asg[[i]]), dfr))
  # e(): reference coding
  nm <- unlist(lapply(bt, function(b) b$names)); keep <- unlist(lapply(bt, function(b) b$keep))
  Xr <- cbind(do.call(cbind, lapply(bt, function(b) b$ref)), one)
  nm <- c(nm, "_cons"); keep <- c(keep, TRUE)
  Xk <- Xr[, keep, drop = FALSE]
  if (qr(Xk)$rank < ncol(Xk)) stop("collinear terms (Stata: r(459))", call. = FALSE)
  Ki <- solve(crossprod(Xk))
  Bk <- Ki %*% crossprod(Xk, Y)
  kk <- length(nm); pos <- which(keep)
  Bf <- matrix(0, kk, q, dimnames = list(nm, deps)); Bf[pos, ] <- Bk
  Xf <- matrix(0, kk, kk, dimnames = list(nm, nm)); Xf[pos, pos] <- Ki
  Sg <- E / dfr
  bvec <- as.vector(Bf)
  names(bvec) <- as.vector(outer(nm, deps, function(a, b) paste0(b, ":", a)))
  Vf <- kronecker(Sg, Xf); dimnames(Vf) <- list(names(bvec), names(bvec))
  bmat <- matrix(bvec, 1, dimnames = list("y1", names(bvec)))
  fits <- t(vapply(seq_len(q), function(i) {
    rss <- E[i, i]; mss <- Hm[i, i]
    f <- (mss / dfm) / (rss / dfr)
    c(f, stats::pf(f, dfm, dfr, lower.tail = FALSE), mss / (mss + rss), sqrt(rss / dfr))
  }, numeric(4)))
  labels <- vapply(bt, function(b) b$label, "")
  if (show) {
    cat(sprintf("\n%23sNumber of obs = %10d\n\n", "", n))
    cat("                       W = Wilks' lambda      L = Lawley-Hotelling trace\n")
    cat("                       P = Pillai's trace     R = Roy's largest root\n\n")
    cat("              Source | Statistic        df    F(df1,     df2) =   F   Prob>F\n")
    rule <- "          -----------+-------------------------------------------------------"
    gap <- "                     |-------------------------------------------------------"
    cat(rule, "\n", sep = "")
    if (length(bt) > 1) {
      stata_mn_block("Model", dfm, fm$st); cat(gap, "\n", sep = "")
      cat(sprintf("%20s |%20s\n", "Residual", format(dfr)), rule, "\n", sep = "")
    }
    for (i in seq_along(bt)) { stata_mn_block(labels[i], length(asg[[i]]), ft[[i]]$st); cat(gap, "\n", sep = "") }
    cat(sprintf("%20s |%20s\n", "Residual", format(dfr)), rule, "\n", sep = "")
    cat(sprintf("%20s |%20s\n", "Total", format(n - 1)))
    cat("          -------------------------------------------------------------------\n")
    cat("                       e = exact, a = approximate, u = upper bound on F\n")
  }
  aux <- function(nh) matrix(c(min(q, nh), (abs(q - nh) - 1) / 2, (dfr - q - 1) / 2), 3, 1,
                             dimnames = list(c("s", "m", "n"), "value"))
  ev <- function(l, nh) { l <- l[seq_len(min(nh, q))]; matrix(l, 1, length(l), dimnames = list("r1", paste0("c", seq_along(l)))) }
  E_ <- list(N = n, k = q, rank = q * length(pos), df_m = dfm, df_r = dfr, k_eq = q)
  for (i in seq_along(bt)) E_[[paste0("df_", i)]] <- length(asg[[i]])
  E_$cmdline <- cmdline
  E_$marginsnotok <- "stdp Residuals STDDp"
  E_$marginsdefault <- paste0("predict(xb equation(", deps, "))", collapse = " ")
  E_$p_F <- paste(sprintf("%.4f", fits[, 2]), collapse = " ")
  E_$F <- paste(vapply(fits[, 1], stata_mn_g, ""), collapse = " ")
  E_$r2 <- paste(sprintf("%.4f", fits[, 3]), collapse = " ")
  E_$rmse <- paste(vapply(fits[, 4], stata_mn_g, ""), collapse = " ")
  E_$cmd <- "manova"; E_$predict <- "manova_p"; E_$estat_cmd <- "manova_estat"; E_$properties <- "b V"
  E_$depvar <- paste(deps, collapse = " "); E_$indepvars <- paste(labels, collapse = " ")
  for (i in seq_along(bt)) E_[[paste0("term_", i)]] <- labels[i]
  E_$b <- bmat; E_$V <- Vf
  for (i in seq_along(bt)) {
    nh <- length(asg[[i]])
    E_[[paste0("eigvals_", i)]] <- ev(ft[[i]]$lam, nh)
    E_[[paste0("aux_", i)]] <- aux(nh)
    E_[[paste0("stat_", i)]] <- ft[[i]]$st
    E_[[paste0("H_", i)]] <- H[[i]]
  }
  E_$eigvals_m <- ev(fm$lam, dfm); E_$aux_m <- aux(dfm); E_$stat_m <- fm$st; E_$H_m <- Hm
  E_$E <- E; E_$B <- Bf; E_$xpxinv <- Xf
  attr(E_, "stata_rows") <- rows
  E_
}
