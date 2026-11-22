
# ------------------------------------------------------------------ slogit
# eta_j = theta_j - sum_d phi_dj (x'beta_d); the base outcome has eta 0; the first D other outcomes have
# phi_dj = 1 for d = their place and 0 otherwise (Stata's constraints), the rest are free
stata_slogit_eta <- function(X, th, p, D, k, others) {
  nb <- p * D
  beta <- matrix(th[seq_len(nb)], p, D)
  nf <- k - 1 - D
  phi <- matrix(0, D, k - 1)
  phi[, seq_len(D)] <- diag(D)
  if (nf > 0) phi[, D + seq_len(nf)] <- matrix(th[nb + seq_len(D * nf)], D, nf)
  theta <- th[nb + D * nf + seq_len(k - 1)]
  Z <- X %*% beta
  eta <- matrix(0, nrow(X), k)
  eta[, others] <- matrix(theta, nrow(X), k - 1, byrow = TRUE) - Z %*% phi
  list(eta = eta, beta = beta, phi = phi, theta = theta)
}
stata_slogit_pr <- function(eta) {
  m <- apply(eta, 1, max)
  e <- exp(eta - m)
  e / rowSums(e)
}
stata_slogit <- function(d, y, xs, dim = 1, base = NULL, nolog = FALSE, cmdline = "", show = TRUE) {
  pr <- stata_mx_prep(d, y, xs, "slogit")
  N <- length(pr$y); p <- length(xs)
  lv <- sort(unique(pr$y)); k <- length(lv)
  if (k < 3) stop("slogit needs three or more outcomes (Stata: r(148))", call. = FALSE)
  D <- dim
  lim <- min(k - 1, p)
  if (D < 1 || D > lim) {
    stop("dimension() must be a positive integer and cannot exceed ", lim, "; see slogit for a definition of the dimension limits (Stata: r(198))", call. = FALSE)
  }
  ib <- if (is.null(base)) k else match(base, lv)
  if (is.na(ib)) stop("baseoutcome(", base, ") is not an outcome of ", y, " (Stata: r(459))", call. = FALSE)
  yi <- match(pr$y, lv)
  others <- setdiff(seq_len(k), ib)
  X <- pr$X
  cnt <- vapply(seq_len(k), function(j) sum(yi == j), 1)
  ll <- function(th) {
    e <- stata_slogit_eta(X, th, p, D, k, others)$eta
    m <- apply(e, 1, max)
    sum(e[cbind(seq_len(N), yi)] - m - log(rowSums(exp(e - m))))
  }
  # start: the multinomial logit, reduced to D dimensions by its singular value decomposition
  nf <- k - 1 - D
  X1 <- cbind(X, 1)
  llm <- function(th) {
    B <- matrix(th, p + 1, k - 1)
    e <- matrix(0, N, k); e[, others] <- X1 %*% B
    m <- apply(e, 1, max)
    sum(e[cbind(seq_len(N), yi)] - m - log(rowSums(exp(e - m))))
  }
  mm <- stata_mx_max(llm, rep(0, (p + 1) * (k - 1)))
  Bm <- matrix(mm$x, p + 1, k - 1)
  sv <- svd(Bm[seq_len(p), , drop = FALSE])
  beta0 <- sv$u[, seq_len(D), drop = FALSE] %*% diag(sv$d[seq_len(D)], nrow = D)
  phi0 <- t(sv$v[, seq_len(D), drop = FALSE])
  P0 <- phi0[, seq_len(D), drop = FALSE]
  P0 <- if (abs(det(P0)) > 1e-8) P0 else diag(D)
  beta0 <- beta0 %*% P0
  phi0 <- solve(P0, phi0)
  start <- c(as.vector(beta0), if (nf > 0) as.vector(phi0[, D + seq_len(nf), drop = FALSE]), Bm[p + 1, ])
  fit <- stata_mx_max(ll, start)
  th <- fit$x
  V <- -solve(fit$H)
  e <- stata_slogit_eta(X, th, p, D, k, others)
  lab <- stata_mx_names(lv, pr$lab)
  # the full coefficient vector as Stata's e(b): beta by dimension, phi by dimension and outcome, theta
  nb <- p * D
  nm <- c(as.vector(outer(xs, seq_len(D), function(x, dd) paste0("dim", dd, ":", x))))
  full <- th[seq_len(nb)]
  for (dd in seq_len(D)) for (jj in seq_along(others)) {
    nm <- c(nm, paste0("phi", dd, "_", others[jj], ":_cons")); full <- c(full, e$phi[dd, jj])
  }
  for (jj in seq_along(others)) { nm <- c(nm, paste0("theta", others[jj], ":_cons")); full <- c(full, e$theta[jj]) }
  # map parameters (th order: beta, free phi by column, theta) to e(b) positions
  pos <- seq_len(nb)
  fp <- 0
  for (dd in seq_len(D)) for (jj in seq_along(others)) {
    nm_i <- nb + (dd - 1) * length(others) + jj
    if (jj > D) { fp <- fp + 1 }
  }
  free_phi_index <- function(dd, jj) nb + (jj - D - 1) * D + dd
  for (dd in seq_len(D)) for (jj in seq_along(others)) {
    pos <- c(pos, if (jj > D) free_phi_index(dd, jj) else NA_integer_)
  }
  pos <- c(pos, nb + D * nf + seq_len(k - 1))
  Vf <- matrix(0, length(full), length(full), dimnames = list(nm, nm))
  ok <- which(!is.na(pos))
  Vf[ok, ok] <- V[pos[ok], pos[ok]]
  names(full) <- nm
  bidx <- seq_len(nb)
  chi2 <- drop(t(th[bidx]) %*% solve(V[bidx, bidx, drop = FALSE], th[bidx]))
  pv <- stats::pchisq(chi2, nb, lower.tail = FALSE)
  ll0 <- sum(cnt * log(cnt / N))
  if (show) {
    cat(sprintf("\nStereotype logistic regression%25sNumber of obs = %5d\n", "", N))
    cat(sprintf("%55sWald chi2(%d)  = %6.2f\n", "", nb, chi2))
    cat(sprintf("Log likelihood = %-37s Prob > chi2   = %6.4f\n\n", format(fit$value, digits = 8), pv))
    cons <- character(0)
    cn <- 0
    for (dd in seq_len(D)) for (jj in seq_len(D)) {
      cn <- cn + 1; cons <- c(cons, sprintf(" (%2d)  [phi%d_%d]_cons = %d", cn, dd, others[jj], as.integer(dd == jj)))
    }
    cat(paste(cons, collapse = "\n"), "\n", sep = "")
    q95 <- stats::qnorm(0.975)
    line <- function(l, b, se) {
      z <- b / se
      cat(sprintf("%12s | %10s %10s %7.2f %7.3f %11s %11s\n", l, format(b, digits = 7), format(se, digits = 7), z,
                  2 * stats::pnorm(-abs(z)), format(b - q95 * se, digits = 7), format(b + q95 * se, digits = 7)))
    }
    spec <- function(l, b, what) cat(sprintf("%12s | %10s  (%s)\n", l, format(b), what))
    rule <- paste0("-------------+", strrep("-", 64), "\n")
    cat(strrep("-", 78), "\n", sprintf("%12s | Coefficient  Std. err.      z    P>|z|     [95%% conf. interval]\n", y), rule, sep = "")
    sdv <- sqrt(diag(V))
    for (dd in seq_len(D)) {
      if (D > 1) cat(sprintf("%12s |\n", paste0("dim", dd)))
      for (i in seq_len(p)) line(xs[i], th[(dd - 1) * p + i], sdv[(dd - 1) * p + i])
    }
    cat(rule)
    for (dd in seq_len(D)) {
      for (j in seq_len(k)) {
        l <- paste0("/phi", dd, "_", j)
        if (j == ib) spec(l, 0, "base outcome")
        else {
          jj <- match(j, others)
          if (jj <= D) spec(l, as.numeric(dd == jj), if (dd == jj) "constrained" else "omitted")
          else line(l, e$phi[dd, jj], sdv[free_phi_index(dd, jj)])
        }
      }
      cat(rule)
    }
    for (j in seq_len(k)) {
      l <- paste0("/theta", j)
      if (j == ib) spec(l, 0, "base outcome") else { jj <- match(j, others); line(l, e$theta[jj], sdv[nb + D * nf + jj]) }
    }
    cat(strrep("-", 78), "\n", sep = "")
    cat(sprintf("(%s=%s is the base outcome)\n", y, lab[ib]))
  }
  E_ <- list(rank = length(th), N = N, k = length(full), k_eq = D + D * (k - 1) + (k - 1), ll = fit$value, k_eq_model = D,
             df_m = nb, chi2 = chi2, p = pv, k_dim = D, k_indvars = p, k_out = k, i_base = ib, ic = NA_real_,
             converged = as.numeric(fit$converged), rc = 0, df_0 = k - 1, ll_0 = ll0,
             cmdline = cmdline, cmd = "slogit", predict = "slogit_p", marginsnotok = "SCores stdp", footnote = "slogit_footnote",
             marginsdefault = paste0("predict(pr outcome(#", seq_len(k), "))", collapse = " "),
             title = "Stereotype logistic regression", indvars = paste(xs, collapse = " "), chi2type = "Wald",
             deriv_useminbound = "off", opt = "moptimize", vce = "oim", user = "slogit_d2", ml_method = "e2",
             technique = "nr", which = "max", depvar = y, properties = "b V")
  for (j in seq_len(k)) E_[[paste0("out", j)]] <- lab[j]
  E_$b <- matrix(full, 1, dimnames = list("y1", nm)); E_$V <- Vf
  E_$outcomes <- matrix(lv, k, 1, dimnames = list(NULL, "outcomes"))
  attr(E_, "stata_rows") <- pr$rows
  attr(E_, "slogit") <- list(lv = lv, ib = ib, others = others, p = p, D = D, k = k, xs = xs, th = th)
  E_
}
# the outcomes' probabilities (or the linear index of one) for the rows of d
stata_slogit_pred <- function(d, E_, what = "pr", outcome = 1L) {
  m <- attr(E_, "slogit")
  X <- vapply(m$xs, function(v) stata_mx_num(d[[v]]), numeric(nrow(d)))
  X <- matrix(X, nrow(d), length(m$xs))
  ok <- stats::complete.cases(X)
  e <- matrix(NA_real_, nrow(d), m$k)
  if (any(ok)) e[ok, ] <- stata_slogit_eta(X[ok, , drop = FALSE], m$th, m$p, m$D, m$k, m$others)$eta
  if (what == "xb") return(e[, outcome])
  P <- matrix(NA_real_, nrow(d), m$k)
  if (any(ok)) P[ok, ] <- stata_slogit_pr(e[ok, , drop = FALSE])
  P
}
