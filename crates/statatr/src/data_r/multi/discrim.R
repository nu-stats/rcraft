# discrim lda varlist, group(var): linear discriminant analysis, the resubstitution table and the
# canonical discriminant results Stata leaves in e() (plan/stata/multivariate.md).
stata_discrim <- function(d, vars, group, priors = "equal", notable = FALSE, cmdline = "", show = TRUE) {
  for (v in c(vars, group)) {
    if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
    if (is.character(d[[v]])) stop(v, ": string variable, type mismatch (Stata: r(109))", call. = FALSE)
  }
  if (!is.null(attr(d[[group]], "labels"))) {
    stop("discrim with a labelled group variable is not translated: Stata names the groups by their labels", call. = FALSE)
  }
  X <- vapply(vars, function(v) stata_mn_num(d[[v]]), numeric(nrow(d)))
  X <- matrix(X, nrow(d), length(vars), dimnames = list(NULL, vars))
  g <- stata_mn_num(d[[group]])
  ok <- stats::complete.cases(X) & !is.na(g)
  rows <- which(ok)
  X <- X[ok, , drop = FALSE]; g <- g[ok]
  N <- nrow(X); p <- length(vars)
  lv <- sort(unique(g)); G <- length(lv)
  if (G < 2) stop("discrim needs two or more groups (Stata: r(2001))", call. = FALSE)
  cnt <- vapply(lv, function(l) sum(g == l), 1)
  if (any(cnt < 2)) stop("a group has fewer than two observations (Stata: r(2001))", call. = FALSE)
  gn <- paste0("group", seq_len(G))
  mu <- vapply(seq_len(G), function(k) colMeans(X[g == lv[k], , drop = FALSE]), numeric(p))
  dimnames(mu) <- list(vars, gn)
  gm <- colMeans(X)
  Wk <- lapply(seq_len(G), function(k) { Y <- X[g == lv[k], , drop = FALSE]; crossprod(sweep(Y, 2, colMeans(Y))) })
  W <- Reduce(`+`, Wk); dimnames(W) <- list(vars, vars)
  Tm <- crossprod(sweep(X, 2, gm)); dimnames(Tm) <- list(vars, vars)
  B <- Tm - W
  S <- W / (N - G)
  if (min(eigen(S, symmetric = TRUE, only.values = TRUE)$values) <= 1e-12 * max(diag(S))) {
    stop("the pooled within-group covariance matrix is singular (Stata: r(506))", call. = FALSE)
  }
  Si <- solve(S)
  pri <- if (identical(priors, "equal")) rep(1 / G, G) else if (identical(priors, "proportional")) cnt / N else priors
  Cm <- rbind(Si %*% mu, `_cons` = -0.5 * colSums(mu * (Si %*% mu)))
  dimnames(Cm) <- list(c(vars, "_cons"), gn)
  scores <- X %*% Cm[vars, , drop = FALSE] + rep(Cm["_cons", ] + log(pri), each = N)
  best <- apply(scores, 1, max)
  nwin <- rowSums(scores == best)
  cls <- ifelse(nwin == 1, lv[max.col(scores, ties.method = "first")], NA_real_)
  counts <- matrix(0, G, G)
  for (i in seq_len(G)) for (j in seq_len(G)) counts[i, j] <- sum(g == lv[i] & !is.na(cls) & cls == lv[j])
  # canonical discriminant functions: the eigenproblem of the within covariance and the between
  # cross products, through the inverse of the Cholesky factor of S
  q <- min(p, G - 1)
  Q <- solve(chol(S))
  M <- t(Q) %*% B %*% Q / (N - G)
  em <- eigen((M + t(M)) / 2, symmetric = TRUE)
  ev <- pmax(em$values[seq_len(q)], 0)
  U <- Q %*% em$vectors[, seq_len(q), drop = FALSE]
  # LAPACK builds eigenvectors up to a sign that differs by platform: each function is signed so that
  # its first coefficient is positive, as Stata's are
  for (j in seq_len(q)) if (U[1, j] < 0) U[, j] <- -U[, j]
  fn <- paste0("function", seq_len(q))
  dimnames(U) <- list(vars, fn)
  ew <- eigen(W, symmetric = TRUE)
  # the eigenvectors of W: the largest coefficient of each is positive, as Stata's
  for (j in seq_len(p)) if (ew$vectors[which.max(abs(ew$vectors[, j])), j] < 0) ew$vectors[, j] <- -ew$vectors[, j]
  cum <- cumsum(ev) / sum(ev)
  stat <- t(vapply(seq_len(q), function(j) {
    pp <- p - j + 1; qq <- (G - 1) - j + 1
    lam <- prod(1 / (1 + ev[j:q]))
    t <- if (pp^2 + qq^2 - 5 > 0) sqrt((pp^2 * qq^2 - 4) / (pp^2 + qq^2 - 5)) else 1
    d1 <- pp * qq
    d2 <- t * (N - G - (pp - qq + 1) / 2) - (d1 - 2) / 2
    root <- lam^(1 / t)
    f <- (1 - root) / root * d2 / d1
    c(sqrt(ev[j] / (1 + ev[j])), ev[j], ev[j] / sum(ev), cum[j], lam, f, d1, d2,
      stats::pf(f, d1, d2, lower.tail = FALSE), as.numeric(min(pp, qq) <= 2))
  }, numeric(10)))
  dimnames(stat) <- list(fn, c("Canonical_corr", "Eigenvalue", "Proportion", "Cumulative", "Wilks_Lambda", "F", "df1", "df2", "pvalue", "exact"))
  Lraw <- U / sqrt(N - G)
  Lstd <- U * sqrt(diag(S))
  Ltot <- U * sqrt(diag(Tm) / (N - 1))
  cons <- -colSums(gm * U)
  E_ <- list(f = q, N = N, N_groups = G, k = p,
             cmd = "discrim", marginsnotok = "_ALL", cmdline = cmdline, subcmd = "lda", title = "Linear discriminant analysis",
             predict = "discrim_lda_p", estat_cmd = "discrim_lda_estat",
             grouplabels = paste0("`\"", format(lv, trim = TRUE), "\"'", collapse = " "), ties = "missing",
             varlist = paste(vars, collapse = " "), groupvar = group, properties = "nob noV eigen")
  E_$grouppriors <- matrix(pri, 1, dimnames = list("Priors", gn))
  E_$groupcounts <- matrix(cnt, 1, dimnames = list("Counts", gn))
  E_$groupvalues <- matrix(lv, 1, dimnames = list("Values", gn))
  E_$C <- Cm
  E_$W_eigvecs <- `dimnames<-`(ew$vectors, list(vars, paste0("function", seq_len(p))))
  E_$W_eigvals <- matrix(ew$values, 1, dimnames = list("Eigenvalues", paste0("function", seq_len(p))))
  E_$candisc_stat <- stat
  E_$cmeans <- `dimnames<-`(t(mu - gm) %*% U, list(gn, fn))
  E_$canstruct <- `dimnames<-`(diag(1 / sqrt(diag(S)), nrow = p) %*% S %*% U, list(vars, fn))
  E_$L_totalstd <- Ltot; E_$L_std <- Lstd
  E_$L_unstd <- rbind(U, `_cons` = cons)
  E_$L_raw <- Lraw
  E_$Ev <- matrix(ev, 1, dimnames = list("Eigenvalues", fn))
  E_$sqrtSinv <- `dimnames<-`(t(Q), list(vars, vars))
  E_$Sinv <- Si; E_$S <- S; E_$means <- mu
  E_$SSCP_B <- B; E_$SSCP_W <- W
  for (k in rev(seq_len(G))) E_[[paste0("SSCP_W", k)]] <- `dimnames<-`(Wk[[k]], list(vars, vars))
  E_$SSCP_T <- Tm
  attr(E_, "stata_rows") <- rows
  attr(E_, "class_counts") <- counts
  attr(E_, "classification") <- cls
  if (show && !notable) stata_discrim_table(E_, counts, "Linear discriminant analysis\nResubstitution classification summary")
  E_
}
# the resubstitution table
stata_discrim_table <- function(E_, counts, title) {
  G <- ncol(counts)
  vals <- E_$groupvalues[1, ]
  pri <- E_$grouppriors[1, ]
  cat("\n", title, "\n\n", sep = "")
  cat("    +---------+\n    | Key     |\n    |---------|\n    | Number  |\n    | Percent |\n    +---------+\n")
  cell <- function(k) if (k == 1) 7 else 8
  inner <- 7 + 8 * (G - 1) + 1
  cat(sprintf("%17s|%s\n", "", formatC(" Classified", width = -(inner + 8))))
  head <- sprintf("    True %-8s|", E_$groupvar)
  for (k in seq_len(G)) head <- paste0(head, formatC(format(vals[k], trim = TRUE), width = cell(k)))
  cat(head, " |  Total\n", sep = "")
  rule <- paste0("    -------------+", strrep("-", inner), "+-------")
  cat(rule, "\n", sep = "")
  tot <- rowSums(counts); coltot <- colSums(counts); grand <- sum(counts)
  for (i in seq_len(G)) {
    line <- sprintf("%16s |", format(vals[i], trim = TRUE)); pct <- sprintf("%17s|", "")
    for (k in seq_len(G)) {
      line <- paste0(line, formatC(counts[i, k], width = cell(k), format = "d"))
      pct <- paste0(pct, formatC(100 * counts[i, k] / tot[i], width = cell(k), format = "f", digits = 2))
    }
    cat(line, " |", formatC(tot[i], width = 7, format = "d"), "\n", sep = "")
    cat(pct, " |", formatC(100, width = 7, format = "f", digits = 2), "\n", sep = "")
    if (i < G) cat(sprintf("%17s|%s|%7s\n", "", strrep(" ", inner), ""))
  }
  cat(rule, "\n", sep = "")
  line <- sprintf("%16s |", "Total"); pct <- sprintf("%17s|", ""); pr <- sprintf("%16s |", "Priors")
  for (k in seq_len(G)) {
    line <- paste0(line, formatC(coltot[k], width = cell(k), format = "d"))
    pct <- paste0(pct, formatC(100 * coltot[k] / grand, width = cell(k), format = "f", digits = 2))
    pr <- paste0(pr, formatC(pri[k], width = cell(k), format = "f", digits = 4))
  }
  cat(line, " |", formatC(grand, width = 7, format = "d"), "\n", sep = "")
  cat(pct, " |", formatC(100, width = 7, format = "f", digits = 2), "\n", sep = "")
  cat(sprintf("%17s|%s|%7s\n", "", strrep(" ", inner), ""))
  cat(pr, " |\n", sep = "")
}
# the group each row of d is classified into, NA where a variable is missing or two groups tie
stata_discrim_classify <- function(d, E_) {
  vars <- strsplit(E_$varlist, " ")[[1]]
  X <- vapply(vars, function(v) stata_mn_num(d[[v]]), numeric(nrow(d)))
  X <- matrix(X, nrow(d), length(vars))
  sc <- X %*% E_$C[vars, , drop = FALSE] + rep(E_$C["_cons", ] + log(E_$grouppriors[1, ]), each = nrow(d))
  best <- apply(sc, 1, max)
  ok <- stats::complete.cases(X) & rowSums(sc == best) == 1
  out <- rep(NA_real_, nrow(d))
  out[ok] <- E_$groupvalues[1, max.col(sc, ties.method = "first")][ok]
  out
}
