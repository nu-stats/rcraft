stata_arima <- function(d, y, x, time, delta = 1, ar = integer(0), ma = integer(0), sar = integer(0),
                        sma = integer(0), period = 0, nd = 0, constant = TRUE, level = 0.95, tol = 1e-6,
                        ltol = 1e-7, nrtol = 1e-5, iterate = 300, log = TRUE, show = TRUE, tmfmt = NULL) {
  stop_ <- function(...) stop(..., call. = FALSE)
  miss <- setdiff(c(y, x, time), names(d))
  if (length(miss)) stop_("variable ", miss[1], " not found (Stata: r(111))")
  num <- function(v) { v <- unclass(v); attributes(v) <- NULL; as.numeric(v) }
  tm <- num(d[[time]]) / delta
  rows <- which(!is.na(tm))
  rows <- rows[order(tm[rows], rows)]
  tt <- round(tm[rows])
  if (!length(rows)) stop_("no observations (Stata: r(2000))")
  if (anyDuplicated(tt)) stop_("repeated time values in sample (Stata: r(451))")
  grid <- seq(tt[1], tt[length(tt)])
  pos <- match(tt, grid)
  Yf <- rep(NA_real_, length(grid))
  Yf[pos] <- num(d[[y]])[rows]
  Xf <- matrix(NA_real_, length(grid), length(x))
  for (j in seq_along(x)) Xf[pos, j] <- num(d[[x[j]]])[rows]
  rowof <- rep(NA_integer_, length(grid))
  rowof[pos] <- rows
  dif <- function(v) { for (i in seq_len(nd)) v <- c(NA_real_, base::diff(v)); v }
  Yd <- dif(Yf)
  Xd <- if (length(x)) apply(Xf, 2, dif) else Xf
  if (!is.matrix(Xd)) Xd <- matrix(Xd, length(grid))
  use <- !is.na(Yd) & (if (length(x)) rowSums(is.na(Xd)) == 0 else TRUE)
  w <- which(use)
  if (!length(w)) stop_("no observations (Stata: r(2000))")
  if (!all(use[w[1]:w[length(w)]])) {
    stop_("arima with missing values or gaps inside the sample is not translated yet")
  }
  Y <- Yd[w]
  X <- Xd[w, , drop = FALSE]
  n <- length(Y)
  nb <- length(x) + constant
  Xc <- if (constant) cbind(X, 1) else X
  na <- length(ar); nm <- length(ma); nsa <- length(sar); nsm <- length(sma)
  np <- nb + na + nm + nsa + nsm + 1
  seg <- function(th, from, len) if (len) th[from + seq_len(len)] else numeric(0)
  lagpoly <- function(lags, coefs, scale, sign) {
    top <- if (length(lags)) max(lags) * scale else 0
    p <- numeric(top + 1)
    p[1] <- 1
    for (i in seq_along(lags)) p[lags[i] * scale + 1] <- sign * coefs[i]
    p
  }
  pmul <- function(a, b) {
    out <- numeric(length(a) + length(b) - 1)
    for (i in seq_along(a)) { j <- seq_along(b); out[i + j - 1] <- out[i + j - 1] + a[i] * b[j] }
    out
  }
  inv <- function(m) tryCatch(solve(m), error = function(e) NULL)
  # the Kalman filter: each observation's log likelihood, the one-step prediction of its ARMA
  # disturbance and that prediction's mean squared error
  filt <- function(th, dynamic = NULL, yy = Y, xx = Xc) {
    bb <- th[seq_len(nb)]
    arp <- pmul(lagpoly(ar, seg(th, nb, na), 1, -1), lagpoly(sar, seg(th, nb + na + nm, nsa), period, -1))
    map <- pmul(lagpoly(ma, seg(th, nb + na, nm), 1, 1), lagpoly(sma, seg(th, nb + na + nm + nsa, nsm), period, 1))
    if (nsm == 1 && abs(th[nb + na + nm + nsa + 1]) >= 1) return(NULL)
    pmax <- length(arp) - 1
    qmax <- length(map) - 1
    r <- max(pmax, qmax + 1, 1)
    phi <- numeric(r)
    if (pmax >= 1) phi[seq_len(pmax)] <- -arp[-1]
    Rv <- numeric(r)
    Rv[seq_along(map)] <- map
    sigma <- th[np]
    if (is.na(sigma) || sigma <= 0) return(NULL)
    s2 <- sigma * sigma
    Tm <- matrix(0, r, r)
    Tm[, 1] <- phi
    if (r > 1) Tm[cbind(1:(r - 1), 2:r)] <- 1
    RR <- s2 * outer(Rv, Rv)
    p0 <- tryCatch(solve(diag(r * r) - kronecker(Tm, Tm), as.vector(RR)), error = function(e) NULL)
    if (is.null(p0)) return(NULL)
    P <- matrix(p0, r, r)
    if (!is.finite(P[1, 1]) || P[1, 1] <= 0) return(NULL)
    xb <- if (nb) drop(xx %*% bb) else rep(0, length(yy))
    wv <- yy - xb
    nn <- length(yy)
    ll <- pred <- mse <- numeric(nn)
    a <- numeric(r)
    tT <- t(Tm)
    for (t in seq_len(nn)) {
      v <- wv[t] - a[1]
      f <- P[1, 1]
      if (!is.finite(f) || f <= 0) return(NULL)
      ll[t] <- -0.5 * (log(2 * pi) + log(f) + v * v / f)
      pred[t] <- a[1]
      mse[t] <- f
      k <- as.vector(Tm %*% P[, 1]) / f
      if (!is.null(dynamic) && t >= dynamic) { v <- 0; k <- 0 * k }
      a <- as.vector(Tm %*% a) + k * v
      P <- Tm %*% P %*% tT + RR - outer(k, k) * f
    }
    if (any(!is.finite(ll))) return(NULL)
    list(ll = ll, pred = pred, mse = mse)
  }
  contrib <- function(th) { f <- filt(th); if (is.null(f)) NULL else f$ll }
  loglik <- function(th) { cc <- contrib(th); if (is.null(cc)) -Inf else sum(cc) }
  # each observation's score by central differences
  central <- function(th, i, h) {
    up <- th; dn <- th
    up[i] <- up[i] + h
    dn[i] <- dn[i] - h
    cu <- contrib(up); cd <- contrib(dn)
    if (is.null(cu)) cu <- numeric(n)
    if (is.null(cd)) cd <- numeric(n)
    (cu - cd) / (2 * h)
  }
  scores <- function(th) {
    out <- matrix(0, n, np)
    for (i in seq_len(np)) {
      h <- 1e-3 * max(abs(th[i]), 1)
      wide <- central(th, i, h)
      narrow <- central(th, i, h / 2)
      out[, i] <- (4 * narrow - wide) / 3
    }
    out
  }
  reldif <- function(a, b) abs(a - b) / (abs(b) + 1)
  # least squares; the columns kept are those not collinear with the earlier ones
  lsq <- function(yv, Xm, cons) {
    Z <- if (cons) cbind(Xm, 1) else Xm
    if (!ncol(Z)) return(list(b = numeric(0), rss = sum(yv^2), e = yv, rank = 0L))
    f <- stats::lm.fit(Z, yv)
    b <- f$coefficients
    b[is.na(b)] <- 0
    list(b = unname(b), rss = sum(f$residuals^2), e = unname(f$residuals), rank = f$rank, ncol = ncol(Z))
  }
  lagmat <- function(v, lags, at) {
    if (!length(lags)) return(matrix(0, length(at), 0))
    matrix(vapply(lags, function(l) v[at - l], numeric(length(at))), length(at), length(lags))
  }
  # Hannan and Rissanen's two regressions for the MA terms' starting values
  hr <- function(u, arl, mal) {
    nu <- length(u)
    k <- min(max((nu - 2) %/% 15, 1), 6)
    p <- 5 * k + 1
    shock <- numeric(nu)
    if (nu > 2 * p + 1) {
      at <- (p + 1):nu
      f <- lsq(u[at], lagmat(u, seq_len(p), at), TRUE)
      shock[at] <- f$e
    }
    from <- max(if (length(arl)) max(arl) else 0, p + max(mal))
    if (nu - from <= length(arl) + length(mal)) return(NULL)
    at <- (from + 1):nu
    f <- lsq(u[at], cbind(lagmat(u, arl, at), lagmat(shock, mal, at)), FALSE)
    cf <- f$b
    if (length(mal)) {
      idx <- length(arl) + seq_along(mal)
      cf[idx] <- pmin(pmax(cf[idx], -0.95), 0.95)
    }
    list(b = cf, sigma = sqrt(f$rss / length(at)))
  }
  # Stata's starting values
  f0 <- lsq(Y, X, constant)
  if (nb && f0$rank < f0$ncol) stop_("arima with collinear regressors is not translated yet")
  resid <- as.vector(Y - (if (nb) drop(Xc %*% f0$b) else 0))
  start <- f0$b
  arma <- numeric(na + nm + nsa + nsm)
  sig <- sqrt(f0$rss / n)
  seasonal <- nsa > 0 || nsm > 0
  if (nm > 0 && !seasonal) {
    h <- hr(resid, ar, ma)
    if (!is.null(h)) {
      arma[seq_along(h$b)] <- h$b
      sig <- h$sigma
    } else if (na > 0) {
      lm_ <- max(ar)
      at <- (lm_ + 1):n
      if (length(at) > 0) {
        f <- lsq(resid[at], lagmat(resid, ar, at), FALSE)
        arma[seq_len(na)] <- f$b
        sig <- sqrt(f$rss / length(at))
      }
    }
  }
  if (na > 0 && nm == 0 && nsm == 0 && nsa == 0) {
    lm_ <- max(ar)
    at <- (lm_ + 1):n
    if (length(at) > 0) {
      f <- lsq(resid[at], lagmat(resid, ar, at), FALSE)
      if (f$rank == na) {
        arma[seq_len(na)] <- f$b
        sig <- sqrt(f$rss / length(at))
      }
    }
  }
  th <- c(start, arma, sig)
  # the path: BHHH for 5 iterations, then BFGS for 10, and so on
  bhhh_at <- function(k) k %% 15 < 5
  ll <- loglik(th)
  logl <- list(list(note = "(setting optimization to BHHH)"), list(ll = ll))
  converged <- FALSE
  if (is.finite(ll)) {
    S <- scores(th)
    g <- colSums(S)
    V <- inv(crossprod(S))
    for (k in seq_len(iterate) - 1) {
      bh <- bhhh_at(k + 1)
      if (is.null(V)) break
      vm <- V
      search <- function(vmat) {
        dd <- drop(vmat %*% g)
        tt_ <- 1
        for (i in 1:60) {
          nxt <- th + tt_ * dd
          nl <- loglik(nxt)
          if (is.finite(nl) && nl >= ll) return(list(th = nxt, ll = nl))
          tt_ <- tt_ * 0.5
        }
        NULL
      }
      found <- search(vm)
      if (is.null(found) && !bhhh_at(k)) {
        fresh <- inv(crossprod(S))
        if (!is.null(fresh)) { found <- search(fresh); vm <- fresh }
      }
      if (is.null(found)) break
      nxt <- found$th
      Sn <- scores(nxt)
      gn <- colSums(Sn)
      Vn <- if (bh) inv(crossprod(Sn)) else {
        step <- nxt - th
        yv <- -(gn - g)
        sy <- sum(step * yv)
        if (sy > 0) {
          hy <- drop(vm %*% yv)
          yhy <- sum(yv * hy)
          vm + (sy + yhy) * outer(step, step) / (sy * sy) - (outer(hy, step) + outer(step, hy)) / sy
        } else vm
      }
      cc <- max(reldif(nxt, th))
      lc <- reldif(found$ll, ll)
      th <- nxt
      ll <- found$ll
      S <- Sn
      g <- gn
      V <- Vn
      if (bh != bhhh_at(k)) logl[[length(logl) + 1]] <- list(note = if (bh) "(switching optimization to BHHH)" else "(switching optimization to BFGS)")
      logl[[length(logl) + 1]] <- list(ll = ll)
      nr <- if (is.null(V)) Inf else sum(g * drop(V %*% g))
      if ((cc <= tol || lc <= ltol) && abs(nr) <= nrtol) { converged <- TRUE; break }
    }
  }
  ll <- loglik(th)
  iters <- sum(vapply(logl, function(z) !is.null(z$ll), TRUE)) - 1
  if (log && show) {
    if (iterate > 0) cat("\n")
    i <- 0
    for (z in logl) {
      if (!is.null(z$note)) cat(z$note, "\n", sep = "")
      else {
        cat(sprintf("Iteration %d:  Log likelihood = %10s%s\n", i, trimws(stata_fmt(z$ll, "%10.0g")), if (iterate > 0) "  " else ""))
        i <- i + 1
      }
    }
    if (!converged) cat("convergence not achieved\n")
  }
  # OPG variance from each observation's score
  Sf <- matrix(0, n, np)
  for (i in seq_len(np)) Sf[, i] <- central(th, i, 1e-6 * max(abs(th[i]), 1))
  Vc <- inv(crossprod(Sf))
  if (is.null(Vc)) stop_("Hessian is not negative semidefinite (Stata: r(430))")
  tested <- setdiff(seq_len(np - 1), if (constant) nb else integer(0))
  df_m <- length(tested)
  chi2 <- NA_real_
  if (df_m > 0) {
    vq <- Vc[tested, tested, drop = FALSE]
    iq <- inv(vq)
    if (!is.null(iq)) chi2 <- drop(t(th[tested]) %*% iq %*% th[tested])
  }
  pval <- if (is.na(chi2)) NA_real_ else stats::pchisq(chi2, df_m, lower.tail = FALSE)
  lagn <- function(l, kind) if (l == 1) paste0("L.", kind) else paste0("L", l, ".", kind)
  nm_x <- vapply(x, function(v) if (nd == 0) v else if (nd == 1) paste0("D.", v) else paste0("D", nd, ".", v), "")
  cn <- c(if (length(x)) paste0(y, ":", nm_x), if (constant) paste0(y, ":_cons"),
          if (na) paste0("ARMA:", vapply(ar, lagn, "", "ar")), if (nm) paste0("ARMA:", vapply(ma, lagn, "", "ma")),
          if (nsa) paste0("ARMA", period, ":", vapply(sar, lagn, "", "ar")),
          if (nsm) paste0("ARMA", period, ":", vapply(sma, lagn, "", "ma")), "sigma:_cons")
  b <- stats::setNames(th, cn)
  dimnames(Vc) <- list(cn, cn)
  filtered <- filt(th)
  dep <- if (nd == 0) y else if (nd == 1) paste0("D.", y) else paste0("D", nd, ".", y)
  tmu <- grid[w] * delta
  fmt_t <- function(v) if (is.null(tmfmt)) format(v, digits = 15) else trimws(stata_fmt(v, tmfmt))
  if (show) {
    cat("\nARIMA regression\n\n")
    cat(sprintf("Sample: %s thru %s   Number of obs = %d\n", fmt_t(tmu[1]), fmt_t(tmu[n]), n))
    if (!is.na(chi2)) cat(sprintf("Wald chi2(%d) = %.2f   Prob > chi2 = %.4f\n", df_m, chi2, pval))
    cat(sprintf("Log likelihood = %s\n\n", trimws(stata_fmt(ll, "%9.0g"))))
    stata_coefs(b, Vc, Inf, level)
    if (!converged) cat("Warning: Convergence not achieved.\n")
    cat("Note: The test of the variance against zero is one sided, and the two-sided\n")
    cat("      confidence interval is truncated at zero.\n")
  }
  eqn <- c(y, if (na) paste0("AR", ar, "_terms"), if (nm) paste0("MA", ma, "_terms"), "SIGMA")
  e <- list(rc = 0, ll = ll, converged = as.numeric(converged), rank = np, k = np, k_dv = 1, ic = iters, N = n,
            k_eq_model = 1, sigma = th[np], p = NA_real_, df_m = df_m, chi2 = chi2, k1 = nb, tmin = tmu[1], tmax = tmu[n],
            k_eq = length(eqn), N_gaps = 0, ar_max = if (na) max(ar) else 0, ma_max = if (nm) max(ma) else 0)
  structure(list(b = b, vcov = Vc, df = Inf, rows = rowof[w], e = e, depvar = dep, y = y, x = x, nd = nd, constant = constant,
       ar = ar, ma = ma, eqnames = eqn, tmins = fmt_t(tmu[1]), tmaxs = fmt_t(tmu[n]),
       model = list(th = th, Y = Y, Xc = Xc, nb = nb, grid = grid[w], w = w, Yf = Yf, delta = delta, filt = filt),
       predict_ok = !is.null(filtered), cmdline = NA_character_), class = "stata_arima")
}
# e() after arima: the scalars in Stata's order, then the macros, b and V
stata_arima_e <- function(fit, cmdline) {
  e <- fit$e
  tx <- function(v) paste(v, collapse = " ")
  mac <- list(cmd = "arima", cmdline = cmdline, predict = "arima_p", estat_cmd = "arima_estat",
              covariates = if (length(fit$x)) tx(ifelse(fit$nd == 0, fit$x, paste0(if (fit$nd == 1) "D." else paste0("D", fit$nd, "."), fit$x))) else "_NONE",
              marginsnotok = "stdp mse Residuals YResiduals", marginsok = "xb y default",
              ar = if (length(fit$ar)) tx(fit$ar), ma = if (length(fit$ma)) tx(fit$ma),
              eqnames = tx(fit$eqnames), seasons = "1", tmaxs = fit$tmaxs, tmins = fit$tmins, chi2type = "Wald",
              vcetype = "OPG", vce = "opg", opt = "ml", title = "ARIMA regression", depvar = fit$depvar,
              properties = "b V")
  mac <- Filter(Negate(is.null), mac)
  b <- matrix(fit$b, 1, dimnames = list("y1", names(fit$b)))
  c(e, mac, list(b = b, V = fit$vcov))
}
# predict after arima: xb (the default), y, residuals, yresiduals, mse; structural and dynamic(t) modify xb
stata_arima_predict <- function(fit, d, what = "xb", structural = FALSE, dynamic = NULL, time = NULL) {
  stop_ <- function(...) stop(..., call. = FALSE)
  m <- fit$model
  n <- length(m$Y)
  from <- NULL
  if (!is.null(dynamic)) {
    p <- which(m$grid * m$delta >= dynamic)[1]
    from <- if (is.na(p)) n + 1 else p
  }
  f <- m$filt(m$th, from)
  if (is.null(f)) stop_("predictions could not be computed (Stata: r(498))")
  sb <- if (m$nb) drop(m$Xc %*% m$th[seq_len(m$nb)]) else rep(0, n)
  xb <- if (structural) sb else sb + f$pred
  lag <- m$Yf[m$w] - m$Y
  val <- switch(what,
    xb = xb,
    y = lag + xb,
    residuals = m$Y - xb,
    yresiduals = (lag + m$Y) - (lag + xb),
    mse = f$mse)
  if (what %in% c("y", "yresiduals")) val[is.na(lag)] <- NA_real_
  val[!is.finite(val)] <- NA_real_
  out <- rep(NA_real_, nrow(d))
  out[fit$rows] <- val
  # before the sample (lost to differencing), xb is the constant alone: nothing is seen yet
  tn <- { v <- unclass(d[[time]]); attributes(v) <- NULL; as.numeric(v) }
  first <- m$grid[1] * m$delta
  pre <- which(!is.na(tn) & tn < first)
  if (length(pre) && what == "xb") {
    if (length(fit$x)) stop_("predict before the sample with regressors is not translated yet")
    out[pre] <- if (fit$constant) m$th[m$nb] else 0
  }
  rest <- setdiff(which(!is.na(tn) & tn >= first), fit$rows)
  if (length(rest)) stop_("predict outside the estimation sample after arima is not translated yet")
  out
}
