# mprobit and slogit: maximum likelihood by Newton's method on numerical derivatives of the log
# likelihood (plan/stata/choice-models.md). Stata stops where its own iterations stop; these find the
# optimum, which agrees with Stata's to about 1e-7.
stata_mx_num <- function(u) { u <- unclass(u); attributes(u) <- NULL; as.numeric(u) }
# the 4th-order central difference gradient of f at x
stata_mx_grad <- function(f, x, h0 = 2e-3) {
  vapply(seq_along(x), function(i) {
    h <- h0 * max(1, abs(x[i]))
    e <- numeric(length(x)); e[i] <- h
    (f(x - 2 * e) - 8 * f(x - e) + 8 * f(x + e) - f(x + 2 * e)) / (12 * h)
  }, 0)
}
stata_mx_hess <- function(f, x, h0 = 2e-3) {
  p <- length(x)
  H <- matrix(0, p, p)
  for (i in seq_len(p)) {
    h <- h0 * max(1, abs(x[i]))
    e <- numeric(p); e[i] <- h
    H[i, ] <- (stata_mx_grad(f, x - 2 * e) - 8 * stata_mx_grad(f, x - e) + 8 * stata_mx_grad(f, x + e) -
               stata_mx_grad(f, x + 2 * e)) / (12 * h)
  }
  (H + t(H)) / 2
}
# cheap derivatives for the iterations: central differences of order 2
stata_mx_grad2 <- function(f, x, h0 = 1e-5) {
  vapply(seq_along(x), function(i) {
    h <- h0 * max(1, abs(x[i])); e <- numeric(length(x)); e[i] <- h
    (f(x + e) - f(x - e)) / (2 * h)
  }, 0)
}
stata_mx_hess2 <- function(f, x, h0 = 1e-4) {
  p <- length(x); H <- matrix(0, p, p)
  for (i in seq_len(p)) {
    h <- h0 * max(1, abs(x[i])); e <- numeric(p); e[i] <- h
    H[i, ] <- (stata_mx_grad2(f, x + e) - stata_mx_grad2(f, x - e)) / (2 * h)
  }
  (H + t(H)) / 2
}
# maximize f from start by Newton's method; the last step and the Hessian use the accurate derivatives
stata_mx_max <- function(f, start, maxit = 80) {
  g <- function(x) { v <- f(x); if (is.finite(v)) v else -1e300 }
  x <- start
  conv <- FALSE
  for (it in seq_len(maxit)) {
    gr <- stata_mx_grad2(g, x)
    H <- stata_mx_hess2(g, x)
    ev <- try(max(eigen(H, symmetric = TRUE, only.values = TRUE)$values), silent = TRUE)
    if (!inherits(ev, "try-error") && is.finite(ev) && ev > -1e-8 * max(1, max(abs(H)))) H <- H - (ev + 1e-3 * max(1, max(abs(H)))) * diag(nrow(H))
    st <- try(solve(H, gr), silent = TRUE)
    if (inherits(st, "try-error") || any(!is.finite(st))) st <- -gr / max(1, sqrt(sum(gr^2)))
    big <- max(abs(st))
    if (big > 2) st <- st * 2 / big
    t <- 1; f0 <- g(x)
    while (t > 1e-8 && g(x - t * st) < f0) t <- t / 2
    xn <- x - t * st
    if (g(xn) >= f0) x <- xn
    if (isTRUE(getOption("mx.trace"))) cat("it", it, g(x), t, max(abs(t * st)), "
")
    if (max(abs(t * st) / (1 + abs(x))) < 1e-6) { conv <- TRUE; break }
  }
  # polish with the accurate gradient and Hessian; the Hessian of the last step is the one returned
  H <- NULL
  for (it in 1:3) {
    gr <- stata_mx_grad(g, x); H <- stata_mx_hess(g, x)
    st <- try(solve(H, gr), silent = TRUE)
    if (inherits(st, "try-error") || any(!is.finite(st))) break
    xn <- x - st
    if (g(xn) >= g(x) - 1e-13) x <- xn
    if (max(abs(st) / (1 + abs(x))) < 1e-9) break
  }
  if (is.null(H)) H <- stata_mx_hess(g, x)
  list(x = x, value = g(x), H = H, converged = conv)
}
# Gauss-Hermite nodes and weights for the standard normal
stata_mx_gh <- function(n = 64) {
  i <- seq_len(n - 1)
  J <- matrix(0, n, n)
  J[cbind(i, i + 1)] <- sqrt(i / 2); J[cbind(i + 1, i)] <- sqrt(i / 2)
  e <- eigen(J, symmetric = TRUE)
  list(z = sqrt(2) * e$values, w = e$vectors[1, ]^2)
}
stata_mx_prep <- function(d, y, xs, cmd) {
  for (v in c(y, xs)) {
    if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
    if (is.character(d[[v]])) stop(v, ": string variable, type mismatch (Stata: r(109))", call. = FALSE)
  }
  yy <- stata_mx_num(d[[y]])
  X <- vapply(xs, function(v) stata_mx_num(d[[v]]), numeric(nrow(d)))
  X <- matrix(X, nrow(d), length(xs), dimnames = list(NULL, xs))
  ok <- !is.na(yy) & stats::complete.cases(X)
  rows <- which(ok)
  list(y = yy[ok], X = X[ok, , drop = FALSE], rows = rows, lab = attr(d[[y]], "labels"))
}
stata_mx_names <- function(lv, lab) {
  if (is.null(lab)) return(format(lv, trim = TRUE))
  vapply(lv, function(v) { k <- names(lab)[lab == v]; if (length(k)) k[1] else format(v, trim = TRUE) }, "")
}
# ------------------------------------------------------------------ mprobit
stata_mprobit_prob <- function(eta, nodes) {
  # eta: n x J utilities (the base column 0); P(j) = E over z of prod over m of pnorm(z + eta_j - eta_m)
  n <- nrow(eta); J <- ncol(eta)
  P <- matrix(0, n, J)
  for (j in seq_len(J)) {
    acc <- matrix(1, n, length(nodes$z))
    for (m in seq_len(J)) {
      if (m == j) next
      acc <- acc * stats::pnorm(outer(eta[, j] - eta[, m], nodes$z, "+"))
    }
    P[, j] <- drop(acc %*% nodes$w)
  }
  P
}
stata_mprobit <- function(d, y, xs, base = NULL, probitparam = FALSE, nolog = FALSE, cmdline = "", show = TRUE) {
  pr <- stata_mx_prep(d, y, xs, "mprobit")
  N <- length(pr$y); p <- length(xs)
  lv <- sort(unique(pr$y)); J <- length(lv)
  if (J < 3) stop("mprobit needs three or more outcomes (Stata: r(148))", call. = FALSE)
  cnt <- vapply(lv, function(l) sum(pr$y == l), 1)
  ib <- if (is.null(base)) which.max(cnt) else match(base, lv)
  if (is.na(ib)) stop("baseoutcome(", base, ") is not an outcome of ", y, " (Stata: r(459))", call. = FALSE)
  yi <- match(pr$y, lv)
  X1 <- cbind(pr$X, `_cons` = 1)
  nodes <- stata_mx_gh(64)
  others <- setdiff(seq_len(J), ib)
  ll <- function(th) {
    B <- matrix(th, p + 1, J - 1)
    eta <- matrix(0, N, J); eta[, others] <- X1 %*% B
    P <- stata_mprobit_prob(eta, nodes)
    v <- sum(log(pmax(P[cbind(seq_len(N), yi)], 1e-300)))
    v
  }
  fit <- stata_mx_max(ll, rep(0, (p + 1) * (J - 1)))
  names_x <- c(xs, "_cons")
  vcv <- -solve(fit$H)
  B <- matrix(fit$x, p + 1, J - 1)
  lab <- stata_mx_names(lv, pr$lab)
  sc <- if (probitparam) 1 / sqrt(2) else 1
  bfull <- numeric(0); nm <- character(0)
  for (j in seq_len(J)) {
    if (j == ib) { bfull <- c(bfull, rep(0, p + 1)); nm <- c(nm, paste0(lab[j], ":o.", names_x)) }
    else { k <- match(j, others); bfull <- c(bfull, B[, k]); nm <- c(nm, paste0(lab[j], ":", names_x)) }
  }
  idx <- unlist(lapply(others, function(j) { k <- match(j, others); ((k - 1) * (p + 1) + 1):(k * (p + 1)) }))
  Vfull <- matrix(0, length(bfull), length(bfull), dimnames = list(nm, nm))
  pos <- unlist(lapply(others, function(j) { s <- (j - 1) * (p + 1); s + seq_len(p + 1) }))
  Vfull[pos, pos] <- vcv
  names(bfull) <- nm
  bb <- bfull * sc; Vv <- Vfull * sc  # probitparam: coefficients / sqrt(2), the variance by the same factor
  # Wald chi2 of every slope
  sl <- unlist(lapply(others, function(j) { s <- (j - 1) * (p + 1); s + seq_len(p) }))
  chi2 <- drop(t(bb[sl]) %*% solve(Vv[sl, sl, drop = FALSE], bb[sl]))
  dfm <- length(sl)
  pv <- stats::pchisq(chi2, dfm, lower.tail = FALSE)
  if (show) {
    cat(sprintf("\nMultinomial probit regression%26sNumber of obs = %5d\n", "", N))
    cat(sprintf("%55sWald chi2(%d)  = %6.2f\n", "", dfm, chi2))
    cat(sprintf("Log likelihood = %-37s Prob > chi2   = %6.4f\n\n", format(fit$value, digits = 8), pv))
    cat(strrep("-", 78), "\n", sep = "")
    cat(sprintf("%12s | Coefficient  Std. err.      z    P>|z|     [95%% conf. interval]\n", y))
    q95 <- stats::qnorm(0.975)
    for (j in seq_len(J)) {
      cat("-------------+", strrep("-", 64), "\n", sep = "")
      if (j == ib) { cat(sprintf("%12s |  (base outcome)\n", lab[j])); next }
      cat(sprintf("%12s |\n", lab[j]))
      for (i in seq_len(p + 1)) {
        q <- (j - 1) * (p + 1) + i; se <- sqrt(Vv[q, q]); z <- bb[q] / se
        cat(sprintf("%12s | %10s %10s %7.2f %7.3f %11s %11s\n", names_x[i], format(bb[q], digits = 7), format(se, digits = 7), z,
                    2 * stats::pnorm(-abs(z)), format(bb[q] - q95 * se, digits = 7), format(bb[q] + q95 * se, digits = 7)))
      }
    }
    cat(strrep("-", 78), "\n", sep = "")
  }
  E_ <- list(rank = length(sl) + length(others), N = N, ic = NA_real_, k = length(bfull) - (p + 1), converged = as.numeric(fit$converged),
             rc = 0, ll = fit$value, df_m = dfm, chi2 = chi2, p = pv, k_indvars = p, const = 1, k_out = J, k_dv = 1, k_eq = J,
             k_eq_model = J, k_eq_base = ib, probitparam = as.numeric(probitparam), k_points = 15, baseout = lv[ib], ibaseout = ib,
             cmdline = cmdline, cmd = "mprobit",
             marginsdefault = paste0("predict(pr outcome(", seq_len(J), "))", collapse = " "), marginsnotok = "stdp SCores",
             predict = "mprobit_p", title = "Multinomial probit regression", depvar = y, indvars = paste(xs, collapse = " "),
             outeqs = paste(lab, collapse = " "), chi2type = "Wald", deriv_useminbound = "off", opt = "moptimize", vce = "oim",
             user = "mprobit_lf", ml_method = "e1", technique = "nr", which = "max", properties = "b V")
  for (j in seq_len(J)) E_[[paste0("out", j)]] <- lab[j]
  E_$b <- matrix(bb, 1, dimnames = list("y1", nm)); E_$V <- Vv
  E_$outcomes <- matrix(lv, J, 1, dimnames = list(NULL, "outcomes"))
  attr(E_, "stata_rows") <- pr$rows
  attr(E_, "mprobit") <- list(lv = lv, ib = ib, others = others, p = p, xs = xs, B = B, scale = sc)
  E_
}
stata_mprobit_pr <- function(d, E_) {
  m <- attr(E_, "mprobit")
  X <- vapply(m$xs, function(v) stata_mx_num(d[[v]]), numeric(nrow(d)))
  X <- cbind(matrix(X, nrow(d), length(m$xs)), 1)
  ok <- stats::complete.cases(X)
  J <- length(m$lv)
  eta <- matrix(0, nrow(d), J); eta[, m$others] <- X %*% m$B
  P <- matrix(NA_real_, nrow(d), J)
  if (any(ok)) P[ok, ] <- stata_mprobit_prob(eta[ok, , drop = FALSE], stata_mx_gh(64))
  P
}
