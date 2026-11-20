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
stata_cx_nfit <- function(lo, hi, mid, X, w, fwk, kind, maxit, from = NULL) {
  rows <- length(lo); k <- ncol(X)
  w0 <- if (is.null(w)) rep(1, rows) else w
  ws <- if (fwk || is.null(w)) w0 else w0 * rows / sum(w0)
  n <- sum(ws)
  mu <- sum(ws * mid) / n
  ss <- sum(ws * (mid - mu)^2)
  ones <- matrix(1, rows, 1)
  mc <- if (kind == "tobit") 0 else mu
  cmod <- stata_cx_nmodel(lo, hi, ones, w0, kind, mc)
  cstart <- if (kind == "tobit") c(mu, ss / n) else c(0, 1 / sqrt(ss / (n - 1)))
  cfit <- stata_cx_max(cmod$llf, cmod$derivs, cstart, maxit)
  X1 <- cbind(X, 1)
  b0 <- tryCatch(as.numeric(solve(crossprod(X1 * ws, X1), crossprod(X1 * ws, mid))), error = function(e) NULL)
  if (is.null(b0)) stop("RCraft: a model with collinear regressors is not translated yet", call. = FALSE)
  rss <- sum(ws * (mid - drop(X1 %*% b0))^2)
  df <- if (kind == "tobit") n else n - k - 1
  sigma <- sqrt(rss / df)
  start <- if (kind == "tobit") c(b0, rss / df) else c(b0[seq_len(k)] / sigma, (b0[k + 1] - mu) / sigma, 1 / sigma)
  model <- stata_cx_nmodel(lo, hi, X1, w0, kind, mc)
  if (!is.finite(model$llf(start))) stop("initial values not feasible (Stata: r(1400))", call. = FALSE)
  ll_start <- model$llf(start)
  m <- stata_cx_max(model$llf, model$derivs, start, maxit)
  m$ll_start <- ll_start; m$ll_0 <- cfit$ll; m$cfit <- cfit; m$mu <- mu
  m
}
# the design columns of a command's terms for the rows kept
stata_cx_setup <- function(d, dep, terms, extra = character(), wvar = NULL, cluster = NULL) {
  tv <- stata_cx_vars(terms)
  vars <- unique(c(dep, tv, extra, wvar, cluster))
  stata_cx_check(d, vars)
  ok <- stats::complete.cases(d[vars])
  if (!any(ok)) stop("no observations (Stata: r(2000))", call. = FALSE)
  rows <- which(ok)
  mf <- d[rows, , drop = FALSE]
  des <- stata_cx_design(mf, terms)
  X1 <- cbind(des$X, 1)
  if (qr(X1)$rank < ncol(X1)) stop("RCraft: a model with collinear regressors is not translated yet", call. = FALSE)
  list(rows = rows, mf = mf, des = des, X = des$X, n = nrow(mf))
}
# tobit y x, ll(#) ul(#)
stata_tobit <- function(d, dep, terms, ll = NULL, ul = NULL, wvar = NULL, wtype = NULL, robust = FALSE, cluster = NULL,
                        level = 95, nolog = FALSE, maxit = 300, cmdline = "", show = TRUE) {
  s <- stata_cx_setup(d, dep, terms, wvar = wvar, cluster = cluster)
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
  f <- stata_cx_nfit(lo, hi, mid, X, w, fwk, "tobit", maxit)
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
                                df_r = df_r, k = k, lr = lr, fstat = fstat, wvar = wvar, full = des$full, nmx = des$nmx)
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
  if (a$robust) {
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
  stata_cx_rule()
  if (a$robust) cat(strrep(" ", 13), "|               Robust\n", sep = "")
  cat(stata_tab_pad(stata_tab_abbrev(dep, 12), 12), " | Coefficient  ", if (a$robust) "std. err." else "Std. err.", "      t    P>|t|     [",
      stata_fmt(level, "%9.0g", trim = TRUE), "% conf. interval]\n", sep = "")
  stata_cx_rule2()
  keep <- seq_len(k + 1)
  nm <- c(a$nmx, "_cons")
  b <- attr(e, "stata_b"); V <- attr(e, "stata_V")
  stata_cx_rows(nm, as.numeric(b), sqrt(diag(V)), "t", a$df_r, level, data)
  stata_cx_rule2()
  nb <- length(e$b); v <- e$b[1, nb]; se <- sqrt(e$V[nb, nb])
  crit <- stats::qt((1 + level / 100) / 2, a$df_r); half <- crit * se / v
  stata_cx_plain(stata_tab_abbrev(paste0("var(e.", dep, ")"), 13), v, se, v * exp(-half), v * exp(half))
  stata_cx_rule()
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
  X1 <- cbind(X, 1)
  xb <- drop(X1 %*% bb)
  if (what == "xb") return(xb)
  if (what == "stdp") return(sqrt(rowSums((X1 %*% attr(e, "stata_V")) * X1)))
  nb <- length(e$b); sg <- sqrt(e$b[1, nb])
  lo <- if (is.null(a)) rep(-Inf, nrow(d)) else rep_len(a, nrow(d)); hi <- if (is.null(b)) rep(Inf, nrow(d)) else rep_len(b, nrow(d))
  al <- (lo - xb) / sg; be <- (hi - xb) / sg
  Pa <- ifelse(is.infinite(lo), 0, stats::pnorm(al)); Pb <- ifelse(is.infinite(hi), 1, stats::pnorm(be))
  fa <- ifelse(is.infinite(lo), 0, stats::dnorm(al)); fb <- ifelse(is.infinite(hi), 0, stats::dnorm(be))
  switch(what,
    pr = Pb - Pa,
    e = xb + sg * (fa - fb) / (Pb - Pa),
    ystar = ifelse(is.infinite(lo), 0, lo * Pa) + ifelse(is.infinite(hi), 0, hi * (1 - Pb)) + xb * (Pb - Pa) + sg * (fa - fb))
}
