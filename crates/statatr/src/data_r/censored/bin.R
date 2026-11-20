# cloglog and hetprobit (plan/stata/cloglog.md, plan/stata/truncreg.md): binary-outcome models Stata fits by
# ml's Newton steps. The outcome is 1 where it is not 0.
stata_cx_lnr <- function(u) {
  lp <- stats::pnorm(u, log.p = TRUE)
  list(lp = lp, ratio = exp(stats::dnorm(u, log = TRUE) - lp))
}
# a weighted probit or logit from the constant-only model, Stata's start; the iterations and the one before the last
stata_cx_binfit <- function(y, X1, w, link, maxit = 300) {
  q <- ifelse(y != 0, 1, -1)
  k1 <- ncol(X1)
  if (link == "probit") {
    llf <- function(b) sum(w * stata_cx_lnr(q * drop(X1 %*% b))$lp)
    derivs <- function(b) {
      xb <- drop(X1 %*% b); r <- stata_cx_lnr(q * xb); lam <- q * r$ratio
      sc <- X1 * (w * lam)
      list(ll = sum(w * r$lp), g = colSums(sc), negH = crossprod(X1 * (w * lam * (lam + xb)), X1), scores = sc)
    }
    b0 <- c(rep(0, k1 - 1), stats::qnorm(sum(w * (y != 0)) / sum(w)))
  } else {
    llf <- function(b) { xb <- drop(X1 %*% b); sum(w * stats::plogis(q * xb, log.p = TRUE)) }
    derivs <- function(b) {
      xb <- drop(X1 %*% b); p <- stats::plogis(xb)
      sc <- X1 * (w * ((y != 0) - p))
      list(ll = sum(w * stats::plogis(q * xb, log.p = TRUE)), g = colSums(sc), negH = crossprod(X1 * (w * p * (1 - p)), X1), scores = sc)
    }
    pm <- sum(w * (y != 0)) / sum(w)
    b0 <- c(rep(0, k1 - 1), log(pm / (1 - pm)))
  }
  m <- stata_cx_max(llf, derivs, b0, maxit)
  m$start <- b0
  m
}
# the iterate before the last one of a run: stata_cx_max keeps it
stata_cx_cloglog_model <- function(y, X1, w) {
  yy <- y != 0
  obs <- function(b) {
    eta <- drop(X1 %*% b)
    mu <- exp(eta)
    u <- exp(-mu); om <- -expm1(-mu)
    ll <- ifelse(yy, log(om), -mu)
    g <- ifelse(yy, mu * u / om, -mu)
    h <- ifelse(yy, mu * u * (1 - mu - u) / om^2, -mu)
    cbind(w * ll, w * g, w * h)
  }
  llf <- function(b) { o <- obs(b); if (anyNA(o[, 1])) NaN else sum(o[, 1]) }
  derivs <- function(b) {
    o <- obs(b)
    sc <- X1 * o[, 2]
    list(ll = sum(o[, 1]), g = colSums(sc), negH = crossprod(X1 * (-o[, 3]), X1), scores = sc)
  }
  list(llf = llf, derivs = derivs)
}
stata_cloglog <- function(d, dep, terms, wvar = NULL, wtype = NULL, robust = FALSE, cluster = NULL, level = 95, nolog = FALSE,
                          eform = FALSE, maxit = 300, cmdline = "", show = TRUE) {
  s <- stata_cx_setup(d, dep, terms, wvar = wvar, cluster = cluster)
  mf <- s$mf; X <- s$X; des <- s$des
  y <- stata_cx_num(mf[[dep]])
  w <- if (is.null(wvar)) rep(1, nrow(mf)) else stata_cx_num(mf[[wvar]])
  fwk <- identical(wtype, "f")
  X1 <- cbind(X, 1); k <- ncol(X)
  cl <- if (is.null(cluster)) NULL else mf[[cluster]]
  # the start: the logit fit one iteration before it ends
  lg <- stata_cx_binfit(y, X1, w, "logit", maxit)
  start <- if (is.null(lg$prev)) lg$b else lg$prev
  model <- stata_cx_cloglog_model(y, X1, w)
  f <- stata_cx_max(model$llf, model$derivs, start, maxit)
  # the constant-only likelihood at ln(-ln(1 - ybar)) rounded to float
  ybar <- sum(w * (y != 0)) / sum(w)
  c0 <- log(-log(1 - ybar))
  c0f <- readBin(writeBin(c0, raw(), size = 4), "numeric", size = 4)
  mu0 <- exp(c0f)
  ll0 <- sum(w * ifelse(y != 0, log(-expm1(-mu0)), -mu0))
  V <- stata_cx_vcov(f, robust, w, fwk, cl)
  b <- f$b
  cnt <- function(sel) if (fwk) sum(w[sel]) else sum(sel)
  N <- cnt(rep(TRUE, length(y))); n1 <- cnt(y != 0); n0 <- N - n1
  G <- if (is.null(cl)) NULL else length(unique(cl))
  kk <- length(des$full)
  nbase <- sum(grepl("[0-9]b\\.", des$full))
  stat <- if (robust) sum(b[seq_len(k)] * solve(V[seq_len(k), seq_len(k), drop = FALSE], b[seq_len(k)])) else 2 * (f$ll - ll0)
  e <- list(rank = kk - nbase, N = N, ic = length(f$log) - 1, k = kk, k_eq = 1, k_dv = 1, converged = as.numeric(f$converged), rc = 0)
  if (!is.null(G)) e$N_clust <- G
  e <- c(e, list(ll = f$ll, k_eq_model = 1, ll_0 = ll0, df_m = k, chi2 = stat, p = stats::pchisq(stat, k, lower.tail = FALSE), N_f = n0, N_s = n1))
  e$cmdline <- cmdline; e$cmd <- "cloglog"; e$predict <- "clog_p"; e$marginsok <- "default Pr"; e$marginsnotok <- "stdp SCore"
  e$chi2type <- if (robust) "Wald" else "LR"
  if (!is.null(wvar)) { e$wtype <- c(f = "fweight", p = "pweight", i = "iweight")[[wtype]]; e$wexp <- paste("=", wvar) }
  e$deriv_useminbound <- "off"; e$opt <- "moptimize"
  if (robust) { e$vcetype <- "Robust"; if (!is.null(cluster)) e$clustvar <- cluster }
  e$vce <- if (!is.null(cluster)) "cluster" else if (robust) "robust" else "oim"
  e$title <- "Complementary log–log regression"; e$user <- "clog_lf"; e$ml_method <- "e2"; e$technique <- "nr"; e$which <- "max"
  e$depvar <- dep; e$properties <- "b V"
  cn <- paste0(dep, ":", des$full)
  nfull <- length(cn)
  pos <- which(!grepl("[0-9]b\\.", des$full))
  bF <- numeric(nfull); bF[pos] <- b
  VF <- matrix(0, nfull, nfull, dimnames = list(cn, cn)); VF[pos, pos] <- V
  gF <- numeric(nfull); gF[pos] <- f$g
  ilog <- numeric(20); mm <- min(20, length(f$log)); ilog[seq_len(mm)] <- f$log[seq_len(mm)]
  e$b <- matrix(bF, 1, nfull, dimnames = list("y1", cn)); e$V <- VF
  if (robust) e$V_modelbased <- { M <- matrix(0, nfull, nfull, dimnames = list(cn, cn)); M[pos, pos] <- f$inv; M }
  e$ilog <- matrix(ilog, 1, 20, dimnames = list("r1", paste0("c", 1:20)))
  e$gradient <- matrix(gF, 1, nfull, dimnames = list("r1", cn))
  attr(e, "stata_rows") <- s$rows
  rname <- sub("^_cons$", "(Intercept)", c(des$nmx, "_cons"))
  attr(e, "stata_b") <- stats::setNames(b, rname)
  attr(e, "stata_V") <- { Vn <- V; dimnames(Vn) <- list(rname, rname); Vn }
  attr(e, "stata_areg") <- list(fo = des$fo, xlev = des$xlev, nmx = des$nmx)
  attr(e, "stata_disp") <- list(N = N, n0 = n0, n1 = n1, robust = robust, G = G, cluster = cluster, k = k, stat = stat, nmx = des$nmx)
  if (show) {
    cat("\n")
    if (!nolog) { stata_cx_log(f, if (robust) "Log pseudolikelihood" else "Log likelihood"); cat("\n") }
    stata_cloglog_show(e, level, mf, eform)
  }
  e
}
stata_cloglog_show <- function(e, level = 95, data = NULL, eform = FALSE) {
  a <- attr(e, "stata_disp"); dep <- e$depvar; k <- a$k
  r10 <- function(label, value) sprintf("%-18s= %10s", label, value)
  cnt <- function(v) format(v, scientific = FALSE)
  cat(e$title, strrep(" ", 48 - nchar(e$title)), r10("Number of obs", cnt(a$N)), "
", sep = "")
  cat(strrep(" ", 48), r10("Zero outcomes", cnt(a$n0)), "\n", sep = "")
  cat(strrep(" ", 48), r10("Nonzero outcomes", cnt(a$n1)), "\n", sep = "")
  cat("\n")
  cat(strrep(" ", 48), r10(sprintf("%s chi2(%d)", if (a$robust) "Wald" else "LR", k), sprintf("%.2f", a$stat)), "\n", sep = "")
  lab <- if (a$robust) "Log pseudolikelihood" else "Log likelihood"
  cat(sprintf("%-48s%s\n", paste0(lab, " = ", stata_fmt(e$ll, "%10.0g", trim = TRUE)), r10("Prob > chi2", sprintf("%.4f", e$p))))
  cat("\n")
  stata_cx_note_clusters(a$G, a$cluster)
  stata_cx_rule()
  if (a$robust) cat(strrep(" ", 13), "|               Robust\n", sep = "")
  cat(stata_tab_pad(stata_tab_abbrev(dep, 12), 12), " |", if (eform) "      exp(b)" else " Coefficient", "  ", if (a$robust) "std. err." else "Std. err.", "      z    P>|z|     [",
      stata_fmt(level, "%9.0g", trim = TRUE), "% conf. interval]\n", sep = "")
  stata_cx_rule2()
  b <- attr(e, "stata_b"); V <- attr(e, "stata_V")
  stata_cx_rows(c(a$nmx, "_cons"), as.numeric(b), sqrt(diag(V)), "z", Inf, level, data, eform = eform)
  stata_cx_rule()
  invisible(NULL)
}
stata_cloglog_replay <- function(level = 95, data = NULL, eform = FALSE) {
  if (!identical(stata_e$cmd, "cloglog")) stop("last estimates not found (Stata: r(301))", call. = FALSE)
  stata_cloglog_show(stata_e, level, if (exists("stata_data")) stata_data else data, eform)
}
# predict after cloglog: pr (the default) and xb
stata_cloglog_predict <- function(e, d, what) {
  ar <- attr(e, "stata_areg"); bb <- attr(e, "stata_b")
  mf <- stats::model.frame(ar$fo, d, xlev = ar$xlev, na.action = stats::na.pass)
  X <- stats::model.matrix(ar$fo, mf); X <- X[, colnames(X) != "(Intercept)", drop = FALSE]
  xb <- drop(cbind(X, 1) %*% bb)
  if (what == "xb") xb else -expm1(-exp(xb))
}
# hetprobit y x, het(z): Pr(y) = Phi(xb / exp(zg))
stata_cx_hetmodel <- function(y, X1, Z, w) {
  q <- ifelse(y != 0, 1, -1); k1 <- ncol(X1); m <- ncol(Z)
  parts <- function(par) {
    s <- exp(drop(Z %*% par[k1 + seq_len(m)]))
    t <- drop(X1 %*% par[seq_len(k1)]) / s
    list(s = s, t = t)
  }
  llf <- function(par) { p <- parts(par); sum(w * stata_cx_lnr(q * p$t)$lp) }
  derivs <- function(par) {
    p <- parts(par); u <- q * p$t
    r <- stata_cx_lnr(u)
    lam <- q * r$ratio; kap <- -r$ratio * (u + r$ratio)
    dt <- cbind(X1 / p$s, -p$t * Z)
    sc <- dt * (w * lam)
    P <- k1 + m
    H <- crossprod(dt * (w * kap), dt)
    # second derivatives of t: zero in beta-beta, -x z / s in beta-gamma, t z z in gamma-gamma
    Hbg <- matrix(0, k1, m)
    for (j in seq_len(m)) Hbg[, j] <- colSums(X1 * (w * lam * (-Z[, j] / p$s)))
    Hgg <- matrix(0, m, m)
    for (i in seq_len(m)) for (j in seq_len(m)) Hgg[i, j] <- sum(w * lam * p$t * Z[, i] * Z[, j])
    H[seq_len(k1), k1 + seq_len(m)] <- H[seq_len(k1), k1 + seq_len(m)] + Hbg
    H[k1 + seq_len(m), seq_len(k1)] <- H[k1 + seq_len(m), seq_len(k1)] + t(Hbg)
    H[k1 + seq_len(m), k1 + seq_len(m)] <- H[k1 + seq_len(m), k1 + seq_len(m)] + Hgg
    list(ll = sum(w * r$lp), g = colSums(sc), negH = -H, scores = sc)
  }
  list(llf = llf, derivs = derivs)
}
stata_hetprobit <- function(d, dep, terms, het, wvar = NULL, wtype = NULL, robust = FALSE, cluster = NULL, level = 95, nolog = FALSE,
                            maxit = 300, cmdline = "", show = TRUE) {
  s <- stata_cx_setup(d, dep, terms, extra = het, wvar = wvar, cluster = cluster)
  mf <- s$mf; X <- s$X; des <- s$des
  y <- stata_cx_num(mf[[dep]])
  w <- if (is.null(wvar)) rep(1, nrow(mf)) else stata_cx_num(mf[[wvar]])
  fwk <- identical(wtype, "f")
  Z <- sapply(het, function(v) stata_cx_num(mf[[v]])); if (!is.matrix(Z)) Z <- matrix(Z, ncol = length(het))
  colnames(Z) <- het
  X1 <- cbind(X, 1); k <- ncol(X); m <- ncol(Z)
  cl <- if (is.null(cluster)) NULL else mf[[cluster]]
  pr <- stata_cx_binfit(y, X1, w, "probit", maxit)
  model <- stata_cx_hetmodel(y, X1, Z, w)
  f <- stata_cx_max(model$llf, model$derivs, c(pr$b, rep(0, m)), maxit)
  V <- stata_cx_vcov(f, robust, w, fwk, cl)
  b <- f$b
  np <- k + 1 + m
  chi2 <- sum(b[seq_len(k)] * solve(V[seq_len(k), seq_len(k), drop = FALSE], b[seq_len(k)]))
  gi <- k + 1 + seq_len(m)
  chi2c <- if (robust) sum(b[gi] * solve(V[gi, gi, drop = FALSE], b[gi])) else 2 * (f$ll - pr$ll)
  N <- if (fwk) sum(w) else length(y)
  n1 <- sum(y != 0); n0 <- length(y) - n1
  G <- if (is.null(cl)) NULL else length(unique(cl))
  e <- list(rank = np, N = N, ic = length(f$log) - 1, k = np, k_eq = 2, k_dv = 1, converged = as.numeric(f$converged), rc = 0)
  if (!is.null(G)) e$N_clust <- G
  e <- c(e, list(ll = f$ll, k_eq_model = 1, df_m = k, chi2 = chi2, p = stats::pchisq(chi2, k, lower.tail = FALSE),
                 chi2_c = chi2c, df_m_c = m, p_c = stats::pchisq(chi2c, m, lower.tail = FALSE), N_f = n0, N_s = n1))
  e$cmdline <- cmdline; e$cmd <- "hetprobit"; e$predict <- "hetprobit_p"; e$method <- "ml"
  if (!robust) e <- append(e, list(ll_c = pr$ll), after = match("p", names(e)))
  e$chi2_ct <- if (robust) "Wald" else "LR"; e$chi2type <- "Wald"
  if (!is.null(wvar)) { e$wtype <- c(f = "fweight", p = "pweight", i = "iweight")[[wtype]]; e$wexp <- paste("=", wvar) }
  e$deriv_useminbound <- "off"; e$opt <- "moptimize"
  if (robust) { e$vcetype <- "Robust"; if (!is.null(cluster)) e$clustvar <- cluster }
  e$vce <- if (!is.null(cluster)) "cluster" else if (robust) "robust" else "oim"
  e$title <- "Heteroskedastic probit model"; e$user <- "hetpr_lf"; e$ml_method <- "e2"; e$technique <- "nr"; e$which <- "max"
  e$depvar <- dep; e$properties <- "b V"
  cn <- c(paste0(dep, ":", c(des$nmx, "_cons")), paste0("lnsigma:", het))
  e$b <- matrix(b, 1, np, dimnames = list("y1", cn)); e$V <- matrix(V, np, np, dimnames = list(cn, cn))
  if (robust) e$V_modelbased <- matrix(f$inv, np, np, dimnames = list(cn, cn))
  ilog <- numeric(20); mm <- min(20, length(f$log)); ilog[seq_len(mm)] <- f$log[seq_len(mm)]
  e$ilog <- matrix(ilog, 1, 20, dimnames = list("r1", paste0("c", 1:20)))
  e$gradient <- matrix(f$g, 1, np, dimnames = list("r1", cn))
  attr(e, "stata_rows") <- s$rows
  rname <- sub("^_cons$", "(Intercept)", c(des$nmx, "_cons"))
  keep <- seq_len(k + 1)
  attr(e, "stata_b") <- stats::setNames(b[keep], rname)
  attr(e, "stata_V") <- { Vn <- V[keep, keep, drop = FALSE]; dimnames(Vn) <- list(rname, rname); Vn }
  attr(e, "stata_areg") <- list(fo = des$fo, xlev = des$xlev, nmx = des$nmx, het = het)
  attr(e, "stata_disp") <- list(N = N, n0 = n0, n1 = n1, robust = robust, G = G, cluster = cluster, k = k, nmx = des$nmx, het = het)
  if (show) {
    cat("\n")
    if (!nolog) {
      cat("Fitting probit model:\n\n"); stata_cx_log(pr, "Log likelihood")
      cat("\nFitting full model:\n\n"); stata_cx_log(f, if (robust) "Log pseudolikelihood" else "Log likelihood"); cat("\n")
    }
    stata_hetprobit_show(e, level, mf)
  }
  e
}
stata_hetprobit_show <- function(e, level = 95, data = NULL) {
  a <- attr(e, "stata_disp"); dep <- e$depvar; k <- a$k
  r10 <- function(label, value) sprintf("%-18s= %10s", label, value)
  cnt <- function(v) format(v, scientific = FALSE)
  cat(e$title, strrep(" ", 48 - nchar(e$title)), r10("Number of obs", cnt(a$N)), "
", sep = "")
  cat(strrep(" ", 48), r10("Zero outcomes", cnt(a$n0)), "\n", sep = "")
  cat(strrep(" ", 48), r10("Nonzero outcomes", cnt(a$n1)), "\n", sep = "")
  cat("\n")
  cat(strrep(" ", 48), r10(sprintf("Wald chi2(%d)", k), sprintf("%.2f", e$chi2)), "\n", sep = "")
  lab <- if (a$robust) "Log pseudolikelihood" else "Log likelihood"
  cat(sprintf("%-48s%s\n", paste0(lab, " = ", stata_fmt(e$ll, "%9.0g", trim = TRUE)), r10("Prob > chi2", sprintf("%.4f", e$p))))
  cat("\n")
  stata_cx_note_clusters(a$G, a$cluster)
  stata_cx_rule()
  if (a$robust) cat(strrep(" ", 13), "|               Robust\n", sep = "")
  cat(stata_tab_pad(stata_tab_abbrev(dep, 12), 12), " | Coefficient  ", if (a$robust) "std. err." else "Std. err.", "      z    P>|z|     [",
      stata_fmt(level, "%9.0g", trim = TRUE), "% conf. interval]\n", sep = "")
  stata_cx_rule2()
  cat(stata_tab_pad(dep, 12), " |\n", sep = "")
  b <- e$b[1, ]; se <- sqrt(diag(e$V))
  crit <- stats::qnorm((1 + level / 100) / 2)
  row <- function(label, j) {
    st <- b[j] / se[j]; p <- 2 * stats::pnorm(-abs(st))
    cat(stata_tab_pad(stata_tab_abbrev(label, 12), 12), " |", stata_tab_pad(stata_tab_g(b[j]), 11), stata_tab_pad(stata_tab_g(se[j]), 11),
        stata_tab_pad(sprintf("%.2f", st), 9), stata_tab_pad(sprintf("%.3f", p), 8), stata_tab_pad(stata_tab_g(b[j] - crit * se[j]), 13),
        stata_tab_pad(stata_tab_g(b[j] + crit * se[j]), 12), "\n", sep = "")
  }
  nm <- c(a$nmx, "_cons")
  for (j in seq_along(nm)) row(nm[j], j)
  stata_cx_rule2()
  cat(stata_tab_pad("lnsigma", 12), " |\n", sep = "")
  for (j in seq_along(a$het)) row(a$het[j], k + 1 + j)
  stata_cx_rule()
  m <- length(a$het)
  txt <- sprintf("%s test of lnsigma=0: chi2(%d) = %.2f", if (a$robust) "Wald" else "LR", m, e$chi2_c)
  right <- sprintf("Prob > chi2 = %.4f", e$p_c)
  cat(formatC(txt, width = -(78 - nchar(right))), right, "\n", sep = "")
  invisible(NULL)
}
stata_hetprobit_replay <- function(level = 95, data = NULL) {
  if (!identical(stata_e$cmd, "hetprobit")) stop("last estimates not found (Stata: r(301))", call. = FALSE)
  stata_hetprobit_show(stata_e, level, if (exists("stata_data")) stata_data else data)
}
stata_hetprobit_predict <- function(e, d, what) {
  ar <- attr(e, "stata_areg"); k <- length(ar$nmx)
  mf <- stats::model.frame(ar$fo, d, xlev = ar$xlev, na.action = stats::na.pass)
  X <- stats::model.matrix(ar$fo, mf); X <- X[, colnames(X) != "(Intercept)", drop = FALSE]
  X1 <- cbind(X, 1)
  b <- e$b[1, ]
  xb <- drop(X1 %*% b[seq_len(k + 1)])
  Z <- sapply(ar$het, function(v) stata_cx_num(d[[v]])); if (!is.matrix(Z)) Z <- matrix(Z, ncol = length(ar$het))
  sg <- exp(drop(Z %*% b[k + 1 + seq_along(ar$het)]))
  switch(what, xb = xb, pr = stats::pnorm(xb / sg), sigma = sg,
         stdp = sqrt(rowSums((X1 %*% attr(e, "stata_V")) * X1)))
}
