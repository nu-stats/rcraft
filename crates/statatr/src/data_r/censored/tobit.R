# tobit and intreg (plan/stata/censored.md): normal regression where each outcome is known to lie in
# [lo, hi] (equal for an exact value, -Inf or Inf for censoring). The parameters are the coefficients,
# the constant and sigma^2 (tobit) or, in Olsen's form, beta/sigma, (constant - m)/sigma and 1/sigma (intreg).
stata_cx_nterms <- function(lo, hi, eta, theta) {
  sg <- exp(theta); n <- length(eta)
  ex <- lo == hi
  za <- (lo - eta) / sg; zb <- (hi - eta) / sg
  fa <- ifelse(is.finite(lo), stats::dnorm(za), 0); fb <- ifelse(is.finite(hi), stats::dnorm(zb), 0)
  zfa <- ifelse(fa == 0, 0, za * fa); zfb <- ifelse(fb == 0, 0, zb * fb)
  p <- ifelse(is.infinite(lo), stats::pnorm(zb), ifelse(is.infinite(hi), stats::pnorm(-za), stats::pnorm(zb) - stats::pnorm(za)))
  sq <- function(z, f) ifelse(f == 0, 0, z^2 * f)
  cb <- function(z, zf) ifelse(zf == 0, 0, z^2 * zf)
  le <- (fa - fb) / (sg * p)
  lt <- (zfa - zfb) / p
  lee <- (zfa - zfb) / (sg^2 * p) - le^2
  let <- ((sq(za, fa) - sq(zb, fb)) / sg - (fa - fb) / sg) / p - le * lt
  ltt <- ((-zfa + cb(za, zfa)) - (-zfb + cb(zb, zfb))) / p - lt^2
  out <- cbind(log(p), le, lt, lee, let, ltt)
  z <- za
  e <- cbind(-0.5 * z^2 - 0.5 * log(2 * pi) - theta, z / sg, z^2 - 1, -1 / sg^2, -2 * z / sg, -2 * z^2)
  out[ex, ] <- e[ex, ]
  out
}
stata_cx_oterms <- function(lo, hi, eta, tau, m) {
  ex <- lo == hi
  za <- tau * (lo - m) - eta; zb <- tau * (hi - m) - eta
  ya <- ifelse(is.finite(lo), lo - m, 0); yb <- ifelse(is.finite(hi), hi - m, 0)
  fa <- ifelse(is.finite(lo), stats::dnorm(za), 0); fb <- ifelse(is.finite(hi), stats::dnorm(zb), 0)
  p <- ifelse(is.infinite(lo), stats::pnorm(zb), ifelse(is.infinite(hi), stats::pnorm(-za), stats::pnorm(zb) - stats::pnorm(za)))
  zf <- function(z, f) ifelse(f == 0, 0, z * f)
  le <- (fa - fb) / p
  lt <- (fb * yb - fa * ya) / p
  lee <- (zf(za, fa) - zf(zb, fb)) / p - le^2
  let <- (-zf(za, fa) * ya + zf(zb, fb) * yb) / p - le * lt
  ltt <- (-zf(zb, fb) * yb^2 + zf(za, fa) * ya^2) / p - lt^2
  out <- cbind(log(p), le, lt, lee, let, ltt)
  yc <- lo - m; z <- tau * yc - eta
  e <- cbind(-0.5 * z^2 - 0.5 * log(2 * pi) + log(tau), z, -z * yc + 1 / tau, -1, yc, -yc^2 - 1 / tau^2)
  out[ex, ] <- e[ex, ]
  out
}
# the model over (beta, constant, last) for the given columns X1 (constant last)
stata_cx_nmodel <- function(lo, hi, X1, w, kind, m = 0) {
  k1 <- ncol(X1)
  obs <- function(par) {
    last <- par[k1 + 1]
    if (is.na(last) || last <= 0) return(NULL)
    eta <- drop(X1 %*% par[seq_len(k1)])
    if (kind == "tobit") {
      t <- stata_cx_nterms(lo, hi, eta, 0.5 * log(last))
      v <- last
      # theta = (1/2) ln v: dtheta/dv = 1/(2v), d2theta/dv2 = -1/(2 v^2)
      t <- cbind(t[, 1], t[, 2], t[, 3] / (2 * v), t[, 4], t[, 5] / (2 * v), t[, 6] / (4 * v^2) - t[, 3] / (2 * v^2))
    } else t <- stata_cx_oterms(lo, hi, eta, last, m)
    t * w
  }
  llf <- function(par) { o <- obs(par); if (is.null(o)) -Inf else sum(o[, 1]) }
  derivs <- function(par) {
    o <- obs(par)
    sc <- cbind(X1 * o[, 2], o[, 3])
    H <- matrix(0, k1 + 1, k1 + 1)
    H[1:k1, 1:k1] <- -crossprod(X1 * o[, 4], X1)
    H[1:k1, k1 + 1] <- -colSums(X1 * o[, 5]); H[k1 + 1, 1:k1] <- H[1:k1, k1 + 1]
    H[k1 + 1, k1 + 1] <- -sum(o[, 6])
    list(ll = sum(o[, 1]), g = colSums(sc), negH = H, scores = sc)
  }
  list(llf = llf, derivs = derivs)
}
# the fit: starting values as Stata's (tobit: least squares with the censored values at their limit,
# sigma^2 = RSS/N; intreg: Olsen's, from the midpoints), the constant-only model, the full model
stata_cx_nfit <- function(lo, hi, mid, X, w, fwk, kind, maxit, cmax = maxit, nocons = FALSE, from = NULL) {
  rows <- length(lo); k <- ncol(X)
  w0 <- if (is.null(w)) rep(1, rows) else w
  ws <- if (fwk || is.null(w)) w0 else w0 * rows / sum(w0)
  n <- sum(ws)
  mu <- sum(ws * mid) / n
  ss <- sum(ws * (mid - mu)^2)
  ones <- matrix(1, rows, 1)
  mc <- if (kind == "tobit" || nocons) 0 else mu
  cfit <- NULL
  if (!nocons) {
    cmod <- stata_cx_nmodel(lo, hi, ones, w0, kind, mc)
    cstart <- if (kind == "tobit") c(mu, ss / n) else c(0, 1 / sqrt(ss / (n - 1)))
    cfit <- stata_cx_max(cmod$llf, cmod$derivs, cstart, cmax)
  }
  X1 <- if (nocons) X else cbind(X, 1)
  b0 <- tryCatch(as.numeric(solve(crossprod(X1 * ws, X1), crossprod(X1 * ws, mid))), error = function(e) NULL)
  if (is.null(b0)) stop("RCraft: a model with collinear regressors is not translated yet", call. = FALSE)
  rss <- sum(ws * (mid - drop(X1 %*% b0))^2)
  df <- if (kind == "tobit") n else n - ncol(X1)
  sigma <- sqrt(rss / df)
  start <- if (kind == "tobit") c(b0, rss / df) else if (nocons) c(b0 / sigma, 1 / sigma) else c(b0[seq_len(k)] / sigma, (b0[k + 1] - mu) / sigma, 1 / sigma)
  model <- stata_cx_nmodel(lo, hi, X1, w0, kind, mc)
  if (!is.null(from)) start <- from(X1, ws, mid, n)
  if (!is.finite(model$llf(start))) stop("initial values not feasible (Stata: r(1400))", call. = FALSE)
  ll_start <- model$llf(start)
  m <- stata_cx_max(model$llf, model$derivs, start, maxit)
  m$ll_start <- ll_start; m$ll_0 <- if (is.null(cfit)) NULL else cfit$ll; m$cfit <- cfit; m$mu <- if (nocons) 0 else mu
  m
}
# tobit y x, ll(#) ul(#)
stata_tobit <- function(d, dep, terms, ll = NULL, ul = NULL, wvar = NULL, wtype = NULL, robust = FALSE, cluster = NULL,
                        level = 95, nolog = FALSE, maxit = 300, cmdline = "", show = TRUE, offset = NULL, nocons = FALSE,
                        from = NULL, from_skip = FALSE, from_copy = FALSE, stepwise = FALSE) {
  s <- stata_cx_setup(d, dep, terms, wvar = wvar, cluster = cluster, offset = offset, nocons = nocons)
  mf <- s$mf; n <- s$n; X <- s$X; des <- s$des
  y <- stata_cx_num(mf[[dep]])
  w <- if (is.null(wvar)) NULL else stata_cx_num(mf[[wvar]])
  fwk <- identical(wtype, "f")
  lim <- function(v, extreme) if (is.null(v)) NULL else if (isTRUE(v)) extreme else v
  lo_l <- lim(ll, min(y)); hi_l <- lim(ul, max(y))
  if (!is.null(lo_l) && !is.null(hi_l) && lo_l > hi_l) stop("observations with ll() > ul() not allowed (Stata: r(198))", call. = FALSE)
  lo <- y; hi <- y
  for (i in seq_len(n)) {
    if (!is.null(lo_l) && y[i] <= lo_l) { lo[i] <- -Inf; hi[i] <- lo_l }
    else if (!is.null(hi_l) && y[i] >= hi_l) { lo[i] <- hi_l; hi[i] <- Inf }
  }
  mid <- ifelse(is.infinite(lo), hi, ifelse(is.infinite(hi), lo, y))
  # an offset: y - offset is what the regressors explain, the limits move with it
  if (!is.null(s$off)) { lo <- lo - s$off; hi <- hi - s$off; mid <- mid - s$off }
  if (!is.null(from) && from_copy) stop("invalid from() option;\noption copy not allowed (Stata: r(198))", call. = FALSE)
  fromf <- NULL
  if (!is.null(from)) fromf <- function(X1, ws, mid, n) stata_cx_from(from, des, dep, from_skip, from_copy, X1, ws, mid, n)
  # the command has begun its output with a blank line when from() makes it stop
  f <- tryCatch(stata_cx_nfit(lo, hi, mid, X, w, fwk, "tobit", maxit, nocons = nocons, from = fromf),
                error = function(err) { if (!is.null(from) && show) cat("\n"); stop(err) })
  k <- ncol(X)
  cl <- if (is.null(cluster)) NULL else mf[[cluster]]
  V <- stata_cx_vcov(f, robust, w, fwk, cl)
  Vm <- f$inv
  cnt <- function(sel) if (fwk) sum(w[sel]) else sum(sel)
  N <- cnt(rep(TRUE, n)); unc <- cnt(is.finite(lo) & is.finite(hi)); lc <- cnt(is.infinite(lo)); rc <- cnt(is.infinite(hi))
  G <- if (is.null(cl)) NULL else length(unique(cl))
  df_r <- N - k
  nb <- length(des$full) + 1
  b <- f$b
  lr <- 2 * (f$ll - f$ll_0)
  wald <- sum(b[seq_len(k)] * solve(V[seq_len(k), seq_len(k), drop = FALSE], b[seq_len(k)]))
  fstat <- wald / k
  e <- list()
  if (!robust) {
    e <- list(p = stats::pchisq(lr, k, lower.tail = FALSE), chi2 = lr, k_aux = 1, N_unc = unc, N_lc = lc, N_rc = rc)
    if (fwk) e$N <- N
    e <- c(e, list(k_dv = 1, k_eq = 2))
    if (any(grepl("b\\.", des$full))) e$k_autoCns <- sum(grepl("[0-9]b\\.", des$full))
    e <- c(e, list(k = nb + 0, ll = f$ll, rc = 0, converged = as.numeric(f$converged), ic = length(f$log) - 1))
    if (!fwk) e$N <- N
    e <- c(e, list(rank = nb - (if (is.null(e$k_autoCns)) 0 else e$k_autoCns), ll_0 = f$ll_0, r2_p = 1 - f$ll / f$ll_0, df_r = df_r, df_m = k))
  } else {
    e <- list(rank = nb - sum(grepl("[0-9]b\\.", des$full)), N = N, ic = length(f$log) - 1, converged = as.numeric(f$converged), rc = 0)
    if (!is.null(G)) e$N_clust <- G
    e <- c(e, list(ll = f$ll, k = nb, k_eq = 2, k_dv = 1, N_rc = rc, N_lc = lc, N_unc = unc, k_aux = 1,
                   p = stats::pf(fstat, k, df_r, lower.tail = FALSE), F = fstat, df_m = k, ll_0 = f$ll_0, r2_p = 1 - f$ll / f$ll_0, df_r = df_r))
  }
  e$cmdline <- cmdline
  if (stepwise) e$stepwise <- "stepwise"
  if (!is.null(offset)) { e$offset <- offset; e$offset1 <- offset }
  e$marginsok <- "default XB Pr(passthru) E(passthru) YStar(passthru)"
  if (!is.null(lo_l)) e$llopt <- stata_fmt(lo_l, "%9.0g", trim = TRUE)
  if (!is.null(hi_l)) e$ulopt <- stata_fmt(hi_l, "%9.0g", trim = TRUE)
  e$predict <- "tobit_p"; e$cmd <- "tobit"; e$title <- "Tobit regression"
  if (!robust) e$chi2type <- "LR"
  e$method <- "ml"
  if (!is.null(wvar)) { e$wtype <- c(f = "fweight", p = "pweight", i = "iweight", a = "aweight")[[wtype]]; e$wexp <- paste("=", wvar) }
  e$depvar <- dep; e$footnote <- "gsem_footnote"; e$deriv_useminbound <- "off"; e$opt <- "moptimize"
  if (robust) { e$vcetype <- "Robust"; if (!is.null(cluster)) e$clustvar <- cluster }
  e$vce <- if (!is.null(cluster)) "cluster" else if (robust) "robust" else "oim"
  e$user <- "_gsem_eval_iid()"; e$ml_method <- "gf2"; e$technique <- "nr"; e$which <- "max"; e$properties <- "b V"
  # e(b), e(V) and e(gradient) hold a zero column for each factor's base level
  cn <- c(paste0(dep, ":", des$full), paste0("/:var(e.", dep, ")"))
  nfull <- length(cn)
  pos <- c(which(!grepl("[0-9]b\\.", des$full)), nfull)
  bF <- numeric(nfull); bF[pos] <- b
  VF <- matrix(0, nfull, nfull, dimnames = list(cn, cn)); VF[pos, pos] <- V
  gF <- numeric(nfull); gF[pos] <- f$g
  ilog <- numeric(20); mm <- min(20, length(f$log)); ilog[seq_len(mm)] <- f$log[seq_len(mm)]
  e$b <- matrix(bF, 1, nfull, dimnames = list("y1", cn)); e$V <- VF
  if (robust) e$V_modelbased <- { M <- matrix(0, nfull, nfull, dimnames = list(cn, cn)); M[pos, pos] <- Vm; M }
  e$gradient <- matrix(gF, 1, nfull, dimnames = list("r1", cn))
  e$ilog <- matrix(ilog, 1, 20, dimnames = list("r1", paste0("c", 1:20)))
  attr(e, "stata_rows") <- s$rows
  rname <- sub("^_cons$", "(Intercept)", c(des$nmx, "_cons"))
  keep <- seq_len(k + 1)
  attr(e, "stata_b") <- stats::setNames(b[keep], rname)
  attr(e, "stata_V") <- { Vn <- V[keep, keep, drop = FALSE]; dimnames(Vn) <- list(rname, rname); Vn }
  attr(e, "stata_areg") <- list(fo = des$fo, xlev = des$xlev, nmx = des$nmx)
  attr(e, "stata_disp") <- list(lo = lo_l, hi = hi_l, N = N, unc = unc, lc = lc, rc = rc, robust = robust, G = G, cluster = cluster,
                                df_r = df_r, k = k, lr = lr, fstat = fstat, wvar = wvar, full = des$full, nmx = des$nmx, offset = offset)
  if (show) {
    if (!nolog) {
      cat("\n")
      if (any(is.infinite(lo) | is.infinite(hi))) {
        cat("Refining starting values:\n\nGrid node 0:  Log likelihood = ", stata_cx_g10(f$ll_start), "\n\nFitting full model:\n\n", sep = "")
      }
      stata_cx_log(f, if (robust) "Log pseudolikelihood" else "Log likelihood")
    }
    stata_tobit_show(e, level, mf)
  }
  e
}
stata_tobit_show <- function(e, level = 95, data = NULL) {
  a <- attr(e, "stata_disp"); dep <- e$depvar
  cat("\n")
  shown <- function(v, inf) if (is.null(v)) inf else stata_fmt(v, "%9.0g", trim = TRUE)
  lt <- shown(a$lo, "-inf"); ht <- shown(a$hi, "+inf"); w <- max(nchar(lt), nchar(ht))
  cnt <- function(v) format(v, big.mark = "", scientific = FALSE)
  cat(sprintf("%-52s%s\n", "Tobit regression", stata_cx_head_right("Number of obs", cnt(a$N))))
  cat(strrep(" ", 52), stata_cx_head_count("Uncensored", cnt(a$unc)), "\n", sep = "")
  cat(sprintf("%-52s%s\n", paste0("Limits: Lower = ", formatC(lt, width = w)), stata_cx_head_count("Left-censored", cnt(a$lc))))
  cat(sprintf("%-52s%s\n", paste0("        Upper = ", formatC(ht, width = w)), stata_cx_head_count("Right-censored", cnt(a$rc))))
  cat("\n")
  k <- a$k
  if (a$robust || isTRUE(a$ftest)) {
    cat(strrep(" ", 52), stata_cx_head_right(sprintf("F(%d, %s)", k, stata_fmt(a$df_r, "%9.0g", trim = TRUE)), sprintf("%.2f", a$fstat)), "\n", sep = "")
    cat(strrep(" ", 52), stata_cx_head_right("Prob > F", sprintf("%.4f", e$p)), "\n", sep = "")
  } else {
    cat(strrep(" ", 52), stata_cx_head_right(sprintf("LR chi2(%d)", k), sprintf("%.2f", a$lr)), "\n", sep = "")
    cat(strrep(" ", 52), stata_cx_head_right("Prob > chi2", sprintf("%.4f", e$p)), "\n", sep = "")
  }
  lab <- if (a$robust) "Log pseudolikelihood" else "Log likelihood"
  cat(sprintf("%-52s%s\n", paste0(lab, " = ", stata_fmt(e$ll, "%10.0g", trim = TRUE)), stata_cx_head_right("Pseudo R2", sprintf("%.4f", e$r2_p))))
  cat("\n")
  stata_cx_note_clusters(a$G, a$cluster)
  if (isTRUE(a$nocons)) cat(" ( 1)  [", dep, "]_cons = 0\n", sep = "")
  stata_cx_rule()
  if (a$robust) cat(strrep(" ", 13), "|               Robust\n", sep = "")
  cat(stata_tab_pad(stata_tab_abbrev(dep, 12), 12), " | Coefficient  ", if (a$robust) "std. err." else "Std. err.", "      t    P>|t|     [",
      stata_fmt(level, "%9.0g", trim = TRUE), "% conf. interval]\n", sep = "")
  stata_cx_rule2()
  nm <- if (isTRUE(a$nocons)) a$nmx else c(a$nmx, "_cons")
  b <- attr(e, "stata_b"); V <- attr(e, "stata_V")
  stata_cx_rows(nm, as.numeric(b), sqrt(diag(V)), "t", a$df_r, level, data, offset = a$offset)
  if (isTRUE(a$nocons)) cat(stata_tab_pad("_cons", 12), " |", stata_tab_pad("0", 11), "  (omitted)\n", sep = "")
  stata_cx_rule2()
  nb <- length(e$b); v <- e$b[1, nb]; se <- sqrt(e$V[nb, nb])
  crit <- stats::qt((1 + level / 100) / 2, a$df_r); half <- crit * se / v
  stata_cx_plain(stata_tab_abbrev(paste0("var(e.", dep, ")"), 13), v, se, v * exp(-half), v * exp(half))
  stata_cx_rule()
  stata_cx_warn(e)
  invisible(NULL)
}
stata_tobit_replay <- function(level = 95, data = NULL) {
  if (!identical(stata_e$cmd, "tobit")) stop("last estimates not found (Stata: r(301))", call. = FALSE)
  stata_tobit_show(stata_e, level, if (exists("stata_data")) stata_data else data)
}
# predict after tobit: xb, stdp, pr(a,b), e(a,b), ystar(a,b); a and b are numbers or columns, NULL for .
stata_tobit_predict <- function(e, d, what, a = NULL, b = NULL) {
  ar <- attr(e, "stata_areg"); bb <- attr(e, "stata_b")
  mf <- stats::model.frame(ar$fo, d, xlev = ar$xlev, na.action = stats::na.pass)
  X <- stats::model.matrix(ar$fo, mf); X <- X[, colnames(X) != "(Intercept)", drop = FALSE]
  X1 <- if (isTRUE(ar$nocons)) X else cbind(X, 1)
  xb <- drop(X1 %*% bb)
  of <- if (!is.null(e$offset)) e$offset else e$offset1
  if (!is.null(of)) xb <- xb + stata_cx_num(d[[of]])
  if (what == "xb") return(xb)
  stdp <- sqrt(rowSums((X1 %*% attr(e, "stata_V")) * X1))
  if (what == "stdp") return(stdp)
  nb <- length(e$b); sg <- if (identical(e$cmd, "tobit")) sqrt(e$b[1, nb]) else e$sigma
  if (what == "stdf") return(sqrt(stdp^2 + sg^2))
  lo <- if (is.null(a)) rep(-Inf, nrow(d)) else rep_len(a, nrow(d)); hi <- if (is.null(b)) rep(Inf, nrow(d)) else rep_len(b, nrow(d))
  al <- (lo - xb) / sg; be <- (hi - xb) / sg
  Pa <- ifelse(is.infinite(lo), 0, stats::pnorm(al)); Pb <- ifelse(is.infinite(hi), 1, stats::pnorm(be))
  fa <- ifelse(is.infinite(lo), 0, stats::dnorm(al)); fb <- ifelse(is.infinite(hi), 0, stats::dnorm(be))
  switch(what,
    pr = Pb - Pa,
    e = xb + sg * (fa - fb) / (Pb - Pa),
    ystar = ifelse(is.infinite(lo), 0, lo * Pa) + ifelse(is.infinite(hi), 0, hi * (1 - Pb)) + xb * (Pb - Pa) + sg * (fa - fb))
}
# intreg lo hi x: lo missing is left-censored at hi, hi missing is right-censored at lo, equal is exact
stata_intreg <- function(d, lo, hi, terms, wvar = NULL, wtype = NULL, robust = FALSE, cluster = NULL,
                         level = 95, nolog = FALSE, maxit = 300, cmdline = "", show = TRUE, offset = NULL, nocons = FALSE) {
  tv <- stata_cx_vars(terms)
  stata_cx_check(d, c(lo, hi, tv, wvar, cluster, offset))
  vars <- unique(c(tv, wvar, cluster, offset))
  ok <- stats::complete.cases(d[vars]) & !(is.na(d[[lo]]) & is.na(d[[hi]]))
  if (!any(ok)) stop("no observations (Stata: r(2000))", call. = FALSE)
  rows <- which(ok); mf <- d[rows, , drop = FALSE]; n <- nrow(mf)
  des <- stata_cx_design(mf, terms, nocons)
  if (qr(if (nocons) des$X else cbind(des$X, 1))$rank < ncol(des$X) + !nocons) stop("RCraft: a model with collinear regressors is not translated yet", call. = FALSE)
  X <- des$X; k <- ncol(X); nc <- if (nocons) 0 else 1
  l <- stata_cx_num(mf[[lo]]); h <- stata_cx_num(mf[[hi]])
  if (any(!is.na(l) & !is.na(h) & l > h)) stop(sprintf("observations with %s > %s not allowed (Stata: r(498))", lo, hi), call. = FALSE)
  w <- if (is.null(wvar)) NULL else stata_cx_num(mf[[wvar]])
  fwk <- identical(wtype, "f")
  blo <- ifelse(is.na(l), -Inf, l); bhi <- ifelse(is.na(h), Inf, h)
  mid <- ifelse(is.na(l), h, ifelse(is.na(h), l, (l + h) / 2))
  off <- if (is.null(offset)) NULL else stata_cx_num(mf[[offset]])
  if (!is.null(off)) { blo <- blo - off; bhi <- bhi - off; mid <- mid - off }
  f <- stata_cx_nfit(blo, bhi, mid, X, w, fwk, "intreg", maxit, nocons = nocons)
  cl <- if (is.null(cluster)) NULL else mf[[cluster]]
  Vo <- stata_cx_vcov(f, robust, w, fwk, cl)
  m <- f$mu; p <- f$b; ns <- k + nc + 1; tau <- p[ns]
  # Olsen's (delta, delta0, tau) back to (beta, constant, ln sigma), the variance by the delta method
  b <- c(p[seq_len(k)] / tau, if (!nocons) p[k + 1] / tau + m, -log(tau))
  J <- matrix(0, ns, ns)
  for (j in seq_len(k + nc)) { J[j, j] <- 1 / tau; J[j, ns] <- -p[j] / tau^2 }
  J[ns, ns] <- -1 / tau
  V <- J %*% Vo %*% t(J); Vm <- J %*% f$inv %*% t(J)
  cnt <- function(sel) if (fwk) sum(w[sel]) else sum(sel)
  N <- cnt(rep(TRUE, n)); unc <- cnt(is.finite(blo) & is.finite(bhi) & blo == bhi)
  lc <- cnt(is.infinite(blo)); rc <- cnt(is.infinite(bhi)); nint <- N - unc - lc - rc
  G <- if (is.null(cl)) NULL else length(unique(cl))
  kk <- length(des$full) + 1
  nbase <- sum(grepl("[0-9]b\\.", des$full))
  wald <- robust || nocons
  stat <- if (wald) sum(b[seq_len(k)] * solve(V[seq_len(k), seq_len(k), drop = FALSE], b[seq_len(k)])) else 2 * (f$ll - f$ll_0)
  pv <- stats::pchisq(stat, k, lower.tail = FALSE)
  sigma <- exp(b[ns]); sesig <- sigma * sqrt(V[ns, ns])
  e <- list(rank = kk - nbase, N = N, ic = length(f$log) - 1, k = kk, k_eq = 2, k_dv = 2, converged = as.numeric(f$converged), rc = 0)
  if (!is.null(G)) e$N_clust <- G
  e <- c(e, list(ll = f$ll, k_eq_model = 1))
  if (!nocons) e <- c(e, list(ll_0 = f$ll_0, rank0 = 2))
  e <- c(e, list(df_m = k, chi2 = stat, p = pv, N_unc = unc))
  if (is.null(G)) e$N_lc <- lc
  e <- c(e, list(k_aux = 1, se_sigma = sesig, sigma = sigma, N_int = nint, N_rc = rc))
  if (!is.null(G)) e$N_lc <- lc
  e$cmdline <- cmdline; e$cmd <- "intreg"; e$ml_score <- "intrg_ll2"; e$depvar <- paste(lo, hi); e$title <- "Interval regression"
  e$marginsok <- "default XB Pr(passthru) E(passthru) YStar(passthru)"; e$predict <- "intreg_p"
  if (!is.null(offset)) e$offset <- offset
  if (!is.null(wvar)) { e$wexp <- paste("=", wvar); e$wtype <- c(f = "fweight", p = "pweight", i = "iweight")[[wtype]] }
  e$chi2type <- if (wald) "Wald" else "LR"
  e$deriv_useminbound <- "off"; e$opt <- "moptimize"
  if (robust) { e$vcetype <- "Robust"; if (!is.null(cluster)) e$clustvar <- cluster }
  e$vce <- if (!is.null(cluster)) "cluster" else if (robust) "robust" else "oim"
  e$user <- "intrg_ll"; e$ml_method <- "e2"; e$technique <- "nr"; e$which <- "max"; e$properties <- "b V"
  cn <- c(paste0("model:", des$full), "lnsigma:_cons")
  nfull <- length(cn)
  pos <- c(which(!grepl("[0-9]b\\.", des$full)), nfull)
  bF <- numeric(nfull); bF[pos] <- b
  VF <- matrix(0, nfull, nfull, dimnames = list(cn, cn)); VF[pos, pos] <- V
  gF <- numeric(nfull)
  # the gradient is in Olsen's scale, which Stata does not show: at the optimum it is zero to within rounding
  gF[pos] <- f$g
  ilog <- numeric(20); mm <- min(20, length(f$log)); ilog[seq_len(mm)] <- f$log[seq_len(mm)]
  e$b <- matrix(bF, 1, nfull, dimnames = list("y1", cn)); e$V <- VF
  if (robust) e$V_modelbased <- { M <- matrix(0, nfull, nfull, dimnames = list(cn, cn)); M[pos, pos] <- Vm; M }
  e$ilog <- matrix(ilog, 1, 20, dimnames = list("r1", paste0("c", 1:20)))
  e$gradient <- matrix(gF, 1, nfull, dimnames = list("r1", cn))
  attr(e, "stata_rows") <- rows
  rname <- sub("^_cons$", "(Intercept)", c(des$nmx, if (!nocons) "_cons"))
  keep <- seq_len(k + nc)
  attr(e, "stata_b") <- stats::setNames(b[keep], rname)
  attr(e, "stata_V") <- { Vn <- V[keep, keep, drop = FALSE]; dimnames(Vn) <- list(rname, rname); Vn }
  attr(e, "stata_areg") <- list(fo = des$fo, xlev = des$xlev, nmx = des$nmx, nocons = nocons)
  attr(e, "stata_disp") <- list(N = N, unc = unc, lc = lc, rc = rc, nint = nint, robust = robust, wald = wald, nocons = nocons, G = G, cluster = cluster,
                                k = k, stat = stat, nmx = des$nmx, offset = offset)
  if (show) {
    if (!nolog) {
      cat("\nFitting constant-only model:\n\n")
      stata_cx_log(f$cfit, if (robust) "Log likelihood" else "Log likelihood")
      cat("\nFitting full model:\n\n")
      stata_cx_log(f, if (robust) "Log likelihood" else "Log likelihood")
    }
    stata_intreg_show(e, level, mf)
  }
  e
}
stata_intreg_show <- function(e, level = 95, data = NULL) {
  a <- attr(e, "stata_disp"); k <- a$k
  cat("\n")
  cnt <- function(v) format(v, big.mark = "", scientific = FALSE)
  cat(sprintf("%-52s%s\n", "Interval regression", stata_cx_head_right("Number of obs", cnt(a$N))))
  cat(strrep(" ", 52), stata_cx_head_count("Uncensored", cnt(a$unc)), "\n", sep = "")
  cat(strrep(" ", 52), stata_cx_head_count("Left-censored", cnt(a$lc)), "\n", sep = "")
  cat(strrep(" ", 52), stata_cx_head_count("Right-censored", cnt(a$rc)), "\n", sep = "")
  cat(strrep(" ", 52), stata_cx_head_count("Interval-cens.", cnt(a$nint)), "\n", sep = "")
  cat("\n")
  cat(strrep(" ", 52), stata_cx_head_right(sprintf("%s chi2(%d)", if (isTRUE(a$wald)) "Wald" else "LR", k), sprintf("%.2f", a$stat)), "\n", sep = "")
  lab <- if (a$robust) "Log pseudolikelihood" else "Log likelihood"
  cat(sprintf("%-52s%s\n", paste0(lab, " = ", stata_fmt(e$ll, "%10.0g", trim = TRUE)), stata_cx_head_right("Prob > chi2", sprintf("%.4f", e$p))))
  cat("\n")
  stata_cx_note_clusters(a$G, a$cluster)
  stata_cx_rule()
  if (a$robust) cat(strrep(" ", 13), "|               Robust\n", sep = "")
  cat(strrep(" ", 13), "| Coefficient  ", if (a$robust) "std. err." else "Std. err.", "      z    P>|z|     [",
      stata_fmt(level, "%9.0g", trim = TRUE), "% conf. interval]\n", sep = "")
  stata_cx_rule2()
  b <- attr(e, "stata_b"); V <- attr(e, "stata_V")
  stata_cx_rows(c(a$nmx, if (!a$nocons) "_cons"), as.numeric(b), sqrt(diag(V)), "z", Inf, level, data, offset = a$offset)
  stata_cx_rule2()
  nb <- length(e$b); ls <- e$b[1, nb]; se <- sqrt(e$V[nb, nb])
  stata_cx_aux("/lnsigma", ls, se, "z", Inf, level)
  stata_cx_rule2()
  crit <- stats::qnorm((1 + level / 100) / 2)
  stata_cx_plain("sigma", exp(ls), exp(ls) * se, exp(ls - crit * se), exp(ls + crit * se), wide = FALSE)
  stata_cx_rule()
  stata_cx_warn(e)
  invisible(NULL)
}
stata_intreg_replay <- function(level = 95, data = NULL) {
  if (!identical(stata_e$cmd, "intreg")) stop("last estimates not found (Stata: r(301))", call. = FALSE)
  stata_intreg_show(stata_e, level, if (exists("stata_data")) stata_data else data)
}
# tobit's from(matrix [, skip copy]): the vector's columns matched to the model's parameters by name (a column with no equation
# belongs to the first one); parameters it does not name start at 0, except sigma^2, which is the mean squared residual at
# the given coefficients
stata_cx_from <- function(m, des, dep, skip, copy, X1, ws, mid, n) {
  m <- as.matrix(m)
  vals <- as.numeric(m[1, ])
  pn <- c(paste0(dep, ":", des$full), paste0("/:var(e.", dep, ")"))
  isbase <- grepl("[0-9]b\\.", c(des$full, ""))
  out <- rep(NA_real_, length(pn))
    out <- rep(NA_real_, length(pn))
    cn <- colnames(m); if (is.null(cn)) cn <- paste0("c", seq_along(vals))
    for (j in seq_along(vals)) {
      nm <- cn[j]
      full <- if (grepl(":", nm, fixed = TRUE)) nm else paste0(dep, ":", nm)
      i <- match(full, pn)
      if (is.na(i)) { if (!skip) stop(sprintf("initial vector: extra parameter %s found\nspecify skip option if necessary (Stata: r(111))", nm), call. = FALSE) }
      else out[i] <- vals[j]
    }
  if (all(is.na(out))) stop("tobit from() naming none of the model's parameters starts at zeros, whose path Stata takes differently (one iteration more here); not translated", call. = FALSE)
  keep <- which(!isbase)
  out <- out[keep]
  nk <- length(out)
  b <- ifelse(is.na(out[-nk]), 0, out[-nk])
  v <- out[nk]
  if (is.na(v)) v <- sum(ws * (mid - drop(X1 %*% b))^2) / n
  c(b, v)
}
