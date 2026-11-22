# heckman (plan/stata/heckman.md): Heckman's selection model by maximum likelihood (Newton from the two-step
# estimates, the parameters the outcome and selection coefficients, atanh rho and ln sigma) and by the two-step method
stata_cx_millsr <- function(t) exp(stats::dnorm(t, log = TRUE) - stats::pnorm(t, log.p = TRUE))
# Heckman's two-step estimates (weighted for the ML start): probit for selection, then least squares of y on x and the
# inverse Mills ratio among the selected
stata_cx_heck2 <- function(y, sel, X1, Z1, w, pr) {
  gamma <- pr$b
  zg <- drop(Z1 %*% gamma)
  rows <- which(sel)
  lam <- stata_cx_millsr(zg[rows]); delta <- lam * (lam + zg[rows])
  Xs <- cbind(X1[rows, , drop = FALSE], lam); ys <- y[rows]; ws <- w[rows]
  p <- ncol(Xs)
  xtx <- crossprod(Xs * ws, Xs)
  xtxi <- solve(xtx)
  beta <- drop(xtxi %*% crossprod(Xs * ws, ys))
  n1 <- sum(ws)
  rss <- sum(ws * (ys - drop(Xs %*% beta))^2)
  bl <- beta[p]
  # delta's mean is unweighted even with weights (Stata's ML start matches only so)
  sigma <- sqrt(rss / n1 + bl^2 * mean(delta))
  rho <- bl / sigma
  if (abs(rho) > 1) { rho <- sign(rho); sigma <- abs(bl) }
  list(beta = beta, gamma = gamma, rho = rho, sigma = sigma, Xs = Xs, rows = rows, delta = delta, xtxi = xtxi, Vg = solve(pr$negH))
}
stata_cx_heckmodel <- function(y, sel, X1, Z1, w) {
  kx <- ncol(X1); kz <- ncol(Z1)
  n <- length(y)
  y0 <- ifelse(sel, y, 0)
  # each parameter's index (1 a = z'g, 2 m = x'b, 3 alpha, 4 theta) and the column that multiplies it
  idx <- c(rep(2, kx), rep(1, kz), 3, 4)
  D <- cbind(X1, Z1, 1, 1)
  obs <- function(par) {
    beta <- par[seq_len(kx)]; gamma <- par[kx + seq_len(kz)]; alpha <- par[kx + kz + 1]; theta <- par[kx + kz + 2]
    a <- drop(Z1 %*% gamma); m <- ifelse(sel, drop(X1 %*% beta), 0)
    sigma <- exp(theta); cc <- cosh(alpha); ss <- sinh(alpha)
    e <- (y0 - m) / sigma
    wv <- ifelse(sel, a * cc + e * ss, -a)
    r <- stata_cx_lnr(wv)
    mm <- r$ratio; dm <- -mm * (wv + mm)
    wp <- cbind(cc, -ss / sigma, a * ss + e * cc, -ss * e)
    wpp <- list(
      c(0, 0, 0, 0)[1] + 0 * a, 0 * a, ss + 0 * a, 0 * a,
      0 * a, 0 * a, -cc / sigma + 0 * a, ss / sigma + 0 * a,
      ss + 0 * a, -cc / sigma + 0 * a, wv, -cc * e,
      0 * a, ss / sigma + 0 * a, -cc * e, ss * e)
    g <- matrix(0, n, 4); H <- array(0, c(n, 4, 4))
    s1 <- sel
    for (i in 1:4) {
      g[, i] <- ifelse(s1, mm * wp[, i], 0)
      for (j in 1:4) H[, i, j] <- ifelse(s1, mm * wpp[[(i - 1) * 4 + j]] + dm * wp[, i] * wp[, j], 0)
    }
    g[, 1] <- ifelse(s1, g[, 1], -mm); H[, 1, 1] <- ifelse(s1, H[, 1, 1], -mm * (wv + mm))
    g[, 2] <- g[, 2] + ifelse(s1, e / sigma, 0)
    g[, 4] <- g[, 4] + ifelse(s1, e^2 - 1, 0)
    H[, 2, 2] <- H[, 2, 2] - ifelse(s1, 1 / sigma^2, 0)
    H[, 2, 4] <- H[, 2, 4] - ifelse(s1, 2 * e / sigma, 0); H[, 4, 2] <- H[, 4, 2] - ifelse(s1, 2 * e / sigma, 0)
    H[, 4, 4] <- H[, 4, 4] - ifelse(s1, 2 * e^2, 0)
    ll <- r$lp - ifelse(s1, 0.5 * e^2 + theta + 0.5 * log(2 * pi), 0)
    list(ll = ll, g = g, H = H)
  }
  llf <- function(par) { o <- obs(par); v <- sum(w * o$ll); if (is.finite(v)) v else NaN }
  derivs <- function(par) {
    o <- obs(par)
    P <- length(idx)
    sc <- matrix(0, n, P)
    for (p in seq_len(P)) sc[, p] <- w * o$g[, idx[p]] * D[, p]
    nh <- matrix(0, P, P)
    for (p in seq_len(P)) for (q in p:P) {
      v <- -sum(w * o$H[, idx[p], idx[q]] * D[, p] * D[, q])
      nh[p, q] <- v; nh[q, p] <- v
    }
    list(ll = sum(w * o$ll), g = colSums(sc), negH = nh, scores = sc)
  }
  list(llf = llf, derivs = derivs)
}
stata_heckman <- function(d, dep, terms, sel, seldep = NULL, twostep = FALSE, wvar = NULL, wtype = NULL, robust = FALSE,
                          cluster = NULL, level = 95, nolog = FALSE, maxit = 300, cmdline = "", show = TRUE, offset = NULL, nocons = FALSE) {
  stata_cx_check(d, c(dep, terms, sel, seldep, wvar, cluster, offset))
  num <- stata_cx_num
  y <- num(d[[dep]])
  cols <- function(v) { m <- sapply(v, function(u) num(d[[u]])); if (!is.matrix(m)) m <- matrix(m, nrow = nrow(d)); colnames(m) <- v; m }
  Xc <- cols(terms); Zc <- cols(sel)
  okz <- stats::complete.cases(Zc)
  if (!is.null(seldep)) { sv <- num(d[[seldep]]); keep <- okz & !is.na(sv); s <- !is.na(sv) & sv != 0 }
  else { s <- !is.na(y); keep <- okz }
  need <- !is.na(y) & stats::complete.cases(Xc)
  keep <- keep & (!s | need)
  if (!is.null(wvar)) keep <- keep & !is.na(num(d[[wvar]]))
  if (!is.null(cluster)) keep <- keep & !is.na(d[[cluster]])
  if (!is.null(offset)) keep <- keep & !is.na(num(d[[offset]]))
  if (!any(keep)) stop("no observations (Stata: r(2000))", call. = FALSE)
  rows <- which(keep)
  y <- y[rows]; sel_ <- s[rows]
  # an offset in the outcome equation: y - offset is what x explains
  if (!is.null(offset)) y <- y - num(d[[offset]])[rows]
  Xc <- Xc[rows, , drop = FALSE]; Zc <- Zc[rows, , drop = FALSE]
  w <- if (is.null(wvar)) rep(1, length(rows)) else num(d[[wvar]])[rows]
  fwk <- identical(wtype, "f")
  cl <- if (is.null(cluster)) NULL else d[[cluster]][rows]
  X1 <- if (nocons) ifelse(is.na(Xc), 0, Xc) else cbind(ifelse(is.na(Xc), 0, Xc), 1); Z1 <- cbind(Zc, 1)
  cx <- if (nocons) character() else "_cons"
  kx <- ncol(X1); kz <- ncol(Z1); nslope <- length(terms)
  if (qr(Z1)$rank < kz || qr(X1[sel_, , drop = FALSE])$rank < kx) stop("RCraft: heckman with collinear regressors is not translated yet", call. = FALSE)
  cnt <- function(v) if (fwk) sum(w[v]) else sum(v)
  N <- cnt(rep(TRUE, length(y))); nsel <- cnt(sel_); nns <- N - nsel
  seleq <- if (is.null(seldep)) "select" else seldep
  # probit for selection, from the constant-only model, as Stata's probit
  pr <- stata_cx_binfit(as.numeric(sel_), Z1, w, "probit", 300)
  t2 <- stata_cx_heck2(y, sel_, X1, Z1, w, pr)
  disp <- list(N = N, nsel = nsel, nns = nns, kx = kx, kz = kz, terms = terms, sel = sel, seldep = seldep, dep = dep, seleq = seleq, offset = offset, nocons = nocons,
               G = if (is.null(cl)) NULL else length(unique(cl)), cluster = cluster, robust = robust)
  if (twostep) {
    # V = sigma^2 (X'X)^-1 [X'(I - rho^2 Delta)X + rho^2 (X'Delta Z) Vg (Z'Delta X)] (X'X)^-1
    Xs <- t2$Xs; rho <- t2$rho; sigma <- t2$sigma; dl <- t2$delta
    Zs <- Z1[t2$rows, , drop = FALSE]
    mid <- crossprod(Xs * (1 - rho^2 * dl), Xs)
    xdz <- crossprod(Xs * dl, Zs)
    mid <- mid + rho^2 * xdz %*% t2$Vg %*% t(xdz)
    Vb <- sigma^2 * t2$xtxi %*% mid %*% t2$xtxi
    Vb <- (Vb + t(Vb)) / 2
    pb <- kx + 1
    chi2 <- sum(t2$beta[seq_len(nslope)] * solve(Vb[seq_len(nslope), seq_len(nslope), drop = FALSE], t2$beta[seq_len(nslope)]))
    Vg <- (t2$Vg + t(t2$Vg)) / 2
    size <- kx + kz + 1
    cn <- c(paste0(dep, ":", c(terms, cx)), paste0(seleq, ":", c(sel, "_cons")), "/mills:lambda")
    bv <- c(t2$beta[seq_len(kx)], t2$gamma, t2$beta[kx + 1])
    VV <- matrix(0, size, size, dimnames = list(cn, cn))
    oi <- c(seq_len(kx), size); gi <- kx + seq_len(kz)
    bi <- c(seq_len(kx), kx + 1)
    VV[oi, oi] <- Vb[bi, bi]; VV[gi, gi] <- Vg
    e <- list(N = N, chi2 = chi2, df_m = nslope, p = stats::pchisq(chi2, nslope, lower.tail = FALSE), N_nonselected = nns, N_selected = nsel,
              rho = rho, sigma = sigma, lambda = t2$beta[kx + 1], selambda = sqrt(Vb[kx + 1, kx + 1]), fconverged = 1, rank = size)
    e$cmdline <- cmdline; e$cmd <- "heckman"; e$vce <- "conventional"; e$method <- "two-step"; e$rhometh <- "rhosigma"
    e$depvar <- if (is.null(seldep)) dep else paste(dep, seldep)
    e$marginsnotok <- "SCores SCORESEL stdp stdf stdpsel YCond YExpected"
    e$marginsok <- "XB XBSel Mills NShazard PSel E(passthru) Pr(passthru) YStar(passthru) default"
    e$predict <- "heckma_p"; e$title <- "Heckman selection model -- two-step estimates"; e$title2 <- "(regression model with sample selection)"
    e$chi2type <- "Wald"; e$properties <- "b V"
    e$b <- matrix(bv, 1, size, dimnames = list("y1", cn)); e$V <- VV
    disp$twostep <- TRUE; disp$chi2 <- chi2
    nm <- c(terms, cx)
  } else {
    model <- stata_cx_heckmodel(y, sel_, X1, Z1, w)
    rho0 <- max(min(t2$rho, 0.99), -0.99)
    start <- c(t2$beta[seq_len(kx)], t2$gamma, atanh(rho0), log(t2$sigma))
    f <- stata_cx_max(model$llf, model$derivs, start, maxit)
    V <- stata_cx_vcov(f, robust, w, fwk, cl)
    b <- f$b
    P <- length(b)
    ia <- kx + kz + 1; it <- ia + 1
    chi2 <- sum(b[seq_len(nslope)] * solve(V[seq_len(nslope), seq_len(nslope), drop = FALSE], b[seq_len(nslope)]))
    rho <- tanh(b[ia]); sigma <- exp(b[it]); lambda <- rho * sigma
    ga <- (1 - rho^2) * sigma; gt <- rho * sigma
    selam <- sqrt(ga^2 * V[ia, ia] + 2 * ga * gt * V[ia, it] + gt^2 * V[it, it])
    # the independent equations: the probit and the regression on the selected
    Xsel <- X1[sel_, , drop = FALSE]; ysel <- y[sel_]; wsel <- w[sel_]
    bo <- solve(crossprod(Xsel * wsel, Xsel), crossprod(Xsel * wsel, ysel))
    rss <- sum(wsel * (ysel - drop(Xsel %*% bo))^2)
    nl <- floor(sum(wsel))
    llreg <- -0.5 * nl * (log(2 * pi) + log(rss / nl) + 1)
    llc <- pr$ll + llreg
    chi2c <- if (robust) (b[ia] / sqrt(V[ia, ia]))^2 else 2 * (f$ll - llc)
    e <- list(rank = P, N = N, ic = length(f$log) - 1, k = P, k_eq = 4, k_dv = 2, converged = as.numeric(f$converged), rc = 0)
    if (!is.null(disp$G)) e$N_clust <- disp$G
    e <- c(e, list(ll = f$ll, k_eq_model = 1, df_m = nslope, chi2 = chi2, p = stats::pchisq(chi2, nslope, lower.tail = FALSE), rho = rho, sigma = sigma,
                   lambda = lambda, selambda = selam, k_aux = 2, N_selected = nsel, N_nonselected = nns,
                   p_c = stats::pchisq(chi2c, 1, lower.tail = FALSE), chi2_c = chi2c))
    e$cmdline <- cmdline; e$marginsok <- "default XB PSel YCond YExpected Mills NShazard"
    e$marginsnotok <- "SCores SCORESEL stdp stdf stdpsel"; e$cmd <- "heckman"; e$predict <- "heckma_p"; e$method <- "ml"
    e$title2 <- "(regression model with sample selection)"; e$chi2_ct <- if (robust) "Wald" else "LR"
    e$depvar <- if (is.null(seldep)) dep else paste(dep, seldep)
    if (!is.null(wvar)) { e$wtype <- c(f = "fweight", p = "pweight", i = "iweight")[[wtype]]; e$wexp <- paste("=", wvar) }
    e$chi2type <- "Wald"; e$deriv_useminbound <- "off"; e$opt <- "moptimize"
    if (robust) { e$vcetype <- "Robust"; if (!is.null(cluster)) e$clustvar <- cluster }
    e$vce <- if (!is.null(cluster)) "cluster" else if (robust) "robust" else "oim"
    e$title <- "Heckman selection model"; e$user <- "heck_d2"
    if (!is.null(offset)) e$offset1 <- offset
    e$ml_method <- "e2"; e$technique <- "nr"; e$which <- "max"; e$properties <- "b V"
    cn <- c(paste0(dep, ":", c(terms, cx)), paste0(seleq, ":", c(sel, "_cons")), "/:athrho", "/:lnsigma")
    e$b <- matrix(b, 1, P, dimnames = list("y1", cn)); e$V <- matrix(V, P, P, dimnames = list(cn, cn))
    if (robust) e$V_modelbased <- matrix(f$inv, P, P, dimnames = list(cn, cn))
    ilog <- numeric(20); mm <- min(20, length(f$log)); ilog[seq_len(mm)] <- f$log[seq_len(mm)]
    e$ilog <- matrix(ilog, 1, 20, dimnames = list("r1", paste0("c", 1:20)))
    e$gradient <- matrix(f$g, 1, P, dimnames = list("r1", cn))
    disp$twostep <- FALSE
    nm <- c(terms, cx)
  }
  attr(e, "stata_rows") <- rows
  rname <- sub("^_cons$", "(Intercept)", nm)
  keep1 <- seq_len(kx)
  attr(e, "stata_b") <- stats::setNames(e$b[1, keep1], rname)
  attr(e, "stata_V") <- { Vn <- e$V[keep1, keep1, drop = FALSE]; dimnames(Vn) <- list(rname, rname); Vn }
  attr(e, "stata_areg") <- list(terms = terms, sel = sel, nocons = nocons)
  attr(e, "stata_disp") <- disp
  if (show) {
    cat("\n")
    if (!twostep && !nolog) { stata_cx_log(f, if (robust) "Log pseudolikelihood" else "Log likelihood"); cat("\n") }
    stata_heckman_show(e, level)
  }
  e
}
stata_heckman_show <- function(e, level = 95) {
  a <- attr(e, "stata_disp"); b <- e$b[1, ]; V <- e$V; se <- sqrt(diag(V))
  r10 <- function(label, value) sprintf("%-18s= %10s", label, value)
  r12 <- function(label, value) sprintf("%-12s= %10s", label, value)
  cnt <- function(v) format(v, scientific = FALSE)
  kx <- a$kx; kz <- a$kz; nslope <- length(a$terms)
  title <- if (a$twostep) "Heckman selection model -- two-step estimates" else "Heckman selection model"
  cat(sprintf("%-48s%s\n", title, r10("Number of obs", cnt(a$N))))
  cat(sprintf("%-48s%6s%s\n", "(regression model with sample selection)", "", r12("Selected", cnt(a$nsel))))
  cat(sprintf("%-48s%6s%s\n", "", "", r12("Nonselected", cnt(a$nns))))
  cat("\n")
  cat(strrep(" ", 48), r10(sprintf("Wald chi2(%d)", nslope), sprintf("%.2f", e$chi2)), "\n", sep = "")
  if (a$twostep) cat(strrep(" ", 48), r10("Prob > chi2", sprintf("%.4f", e$p)), "\n", sep = "")
  else {
    lab <- if (a$robust) "Log pseudolikelihood" else "Log likelihood"
    cat(sprintf("%-48s%s\n", paste0(lab, " = ", stata_fmt(e$ll, "%9.0g", trim = TRUE)), r10("Prob > chi2", sprintf("%.4f", e$p))))
  }
  cat("\n")
  stata_cx_note_clusters(a$G, a$cluster)
  stata_cx_rule()
  if (a$robust) cat(strrep(" ", 13), "|               Robust\n", sep = "")
  cat(strrep(" ", 13), "| Coefficient  ", if (a$robust) "std. err." else "Std. err.", "      z    P>|z|     [",
      stata_fmt(level, "%9.0g", trim = TRUE), "% conf. interval]\n", sep = "")
  stata_cx_rule2()
  crit <- stats::qnorm((1 + level / 100) / 2)
  row <- function(label, j) {
    st <- b[j] / se[j]; p <- 2 * stats::pnorm(-abs(st))
    cat(stata_tab_pad(stata_tab_abbrev(label, 12), 12), " |", stata_tab_pad(stata_tab_g(b[j]), 11), stata_tab_pad(stata_tab_g(se[j]), 11),
        stata_tab_pad(sprintf("%.2f", st), 9), stata_tab_pad(sprintf("%.3f", p), 8), stata_tab_pad(stata_tab_g(b[j] - crit * se[j]), 13),
        stata_tab_pad(stata_tab_g(b[j] + crit * se[j]), 12), "\n", sep = "")
  }
  eq <- function(name) cat(formatC(stata_tab_abbrev(name, 12), width = -13), "|\n", sep = "")
  eq(a$dep)
  nm <- c(a$terms, if (!isTRUE(a$nocons)) "_cons")
  for (j in seq_along(nm)) row(nm[j], j)
  stata_cx_offrow(a$offset)
  stata_cx_rule2()
  eq(a$seleq)
  nz <- c(a$sel, "_cons")
  for (j in seq_along(nz)) row(nz[j], kx + j)
  stata_cx_rule2()
  if (a$twostep) {
    eq("/mills")
    row("lambda", kx + kz + 1)
    stata_cx_rule2()
    cat(sprintf("%12s | %10.5f\n", "rho", e$rho))
    cat(sprintf("%12s | %10.7f\n", "sigma", e$sigma))
    stata_cx_rule()
    return(invisible(NULL))
  }
  ia <- kx + kz + 1; it <- ia + 1
  stata_cx_aux("/athrho", b[ia], se[ia], "z", Inf, level)
  stata_cx_aux("/lnsigma", b[it], se[it], "z", Inf, level)
  stata_cx_rule2()
  rho <- tanh(b[ia]); sigma <- exp(b[it])
  stata_cx_plain("rho", rho, (1 - rho^2) * se[ia], tanh(b[ia] - crit * se[ia]), tanh(b[ia] + crit * se[ia]), wide = FALSE)
  stata_cx_plain("sigma", sigma, sigma * se[it], exp(b[it] - crit * se[it]), exp(b[it] + crit * se[it]), wide = FALSE)
  stata_cx_plain("lambda", e$lambda, e$selambda, e$lambda - crit * e$selambda, e$lambda + crit * e$selambda, wide = FALSE)
  stata_cx_rule()
  txt <- sprintf("%s test of indep. eqns. (rho = 0): chi2(1) = %.2f", if (a$robust) "Wald" else "LR", e$chi2_c)
  cat(sprintf("%-58sProb > chi2 = %.4f\n", txt, e$p_c))
  stata_cx_warn(e)
  invisible(NULL)
}
stata_heckman_replay <- function(level = 95, data = NULL) {
  if (!identical(stata_e$cmd, "heckman")) stop("last estimates not found (Stata: r(301))", call. = FALSE)
  stata_heckman_show(stata_e, level)
}
# predict after heckman: xb (the default), xbsel, psel, mills, ycond, yexp
stata_heckman_predict <- function(e, d, what) {
  ar <- attr(e, "stata_areg"); a <- attr(e, "stata_disp")
  num <- stata_cx_num
  col <- function(v) if (identical(v, "_cons")) rep(1, nrow(d)) else num(d[[v]])
  b <- e$b[1, ]
  kx <- a$kx; kz <- a$kz
  X1 <- sapply(c(ar$terms, if (!isTRUE(ar$nocons)) "_cons"), col); Z1 <- sapply(c(ar$sel, "_cons"), col)
  if (!is.matrix(X1)) X1 <- matrix(X1, nrow = nrow(d)); if (!is.matrix(Z1)) Z1 <- matrix(Z1, nrow = nrow(d))
  xb <- drop(X1 %*% b[seq_len(kx)]); if (!is.null(e$offset1)) xb <- xb + num(d[[e$offset1]]); zg <- drop(Z1 %*% b[kx + seq_len(kz)])
  if (what == "xb") return(xb)
  if (what == "xbsel") return(zg)
  if (what == "psel") return(stats::pnorm(zg))
  mills <- stata_cx_millsr(zg)
  if (what == "mills") return(mills)
  lam <- e$lambda
  if (what == "ycond") return(xb + lam * mills)
  if (what == "yexp") return(stats::pnorm(zg) * (xb + lam * mills))
  stop("predict statistic not translated", call. = FALSE)
}
