# truncreg (plan/stata/truncreg.md): normal regression observed only between ll and ul; the parameters are
# the coefficients and sigma itself, from least squares on the rows kept with sigma at the root mean squared error
stata_cx_truncmodel <- function(y, X1, w, lo, hi) {
  k1 <- ncol(X1)
  obs <- function(par) {
    s <- par[k1 + 1]
    if (is.na(s) || s <= 0) return(NULL)
    eta <- drop(X1 %*% par[seq_len(k1)])
    z <- (y - eta) / s
    n <- length(y)
    if (is.null(lo)) { a <- rep(-Inf, n); Pa <- 0; fa <- 0; afa <- 0; aafa <- 0; a3 <- 0 }
    else { a <- (lo - eta) / s; Pa <- stats::pnorm(a); fa <- stats::dnorm(a); afa <- a * fa; aafa <- a * a * fa; a3 <- a^3 * fa }
    if (is.null(hi)) { cc <- rep(Inf, n); Pc <- 1; fc <- 0; cfc <- 0; ccfc <- 0; c3 <- 0 }
    else { cc <- (hi - eta) / s; Pc <- stats::pnorm(cc); fc <- stats::dnorm(cc); cfc <- cc * fc; ccfc <- cc * cc * fc; c3 <- cc^3 * fc }
    prob <- Pc - Pa
    ll <- -0.5 * z^2 - 0.5 * log(2 * pi) - log(s) - log(prob)
    p_e <- -(fc - fa) / s
    p_s <- -(cfc - afa) / s
    p_ee <- -(cfc - afa) / s^2
    p_es <- ((fc - fa) - (ccfc - aafa)) / s^2
    p_ss <- ((cfc - afa) + (cfc - c3) - (afa - a3)) / s^2
    lp <- function(d2, d1, d1b) d2 / prob - d1 * d1b / prob^2
    g_e <- z / s - p_e / prob
    g_s <- (z^2 - 1) / s - p_s / prob
    h_ee <- -1 / s^2 - lp(p_ee, p_e, p_e)
    h_es <- -2 * z / s^2 - lp(p_es, p_e, p_s)
    h_ss <- (1 - 3 * z^2) / s^2 - lp(p_ss, p_s, p_s)
    cbind(ll, g_e, g_s, h_ee, h_es, h_ss) * w
  }
  llf <- function(par) { o <- obs(par); if (is.null(o)) NaN else sum(o[, 1]) }
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
stata_truncreg <- function(d, dep, terms, ll = NULL, ul = NULL, wvar = NULL, wtype = NULL, robust = FALSE, cluster = NULL,
                           level = 95, nolog = FALSE, maxit = 300, cmdline = "", show = TRUE, offset = NULL, nocons = FALSE) {
  s <- stata_cx_setup(d, dep, terms, wvar = wvar, cluster = cluster, offset = offset, nocons = nocons)
  mf <- s$mf; X <- s$X; des <- s$des
  y <- stata_cx_num(mf[[dep]])
  w0 <- if (is.null(wvar)) rep(1, nrow(mf)) else stata_cx_num(mf[[wvar]])
  fwk <- identical(wtype, "f")
  lim <- function(v, extreme) if (is.null(v)) NULL else if (isTRUE(v)) extreme else v
  lo <- lim(ll, min(y)); hi <- lim(ul, max(y))
  if (!is.null(lo) && !is.null(hi) && lo > hi) stop("ll() > ul() not allowed (Stata: r(198))", call. = FALSE)
  keep <- rep(TRUE, length(y))
  if (!is.null(lo)) keep <- keep & y > lo
  if (!is.null(hi)) keep <- keep & y < hi
  ntrunc <- if (fwk) sum(w0[!keep]) else sum(!keep)
  if (!any(keep)) stop("no observations (Stata: r(2000))", call. = FALSE)
  rows_all <- s$rows
  rows <- rows_all[keep]
  mf <- mf[keep, , drop = FALSE]; X <- X[keep, , drop = FALSE]; y <- y[keep]; w0 <- w0[keep]
  # an offset: y - offset is what the regressors explain, the limits move with it
  offv <- if (is.null(s$off)) rep(0, length(y)) else s$off[keep]
  lov <- if (is.null(lo)) NULL else lo - offv
  hiv <- if (is.null(hi)) NULL else hi - offv
  y <- y - offv
  cl <- if (is.null(cluster)) NULL else mf[[cluster]]
  w <- if (identical(wtype, "a")) w0 * length(y) / sum(w0) else w0
  if (identical(wtype, "a") && show) cat("(sum of wgt is ", stata_fmt(sum(w0), "%18.0g", trim = TRUE), ")\n", sep = "")
  if (show) cat("(", format(ntrunc, scientific = FALSE), " obs truncated)\n", sep = "")
  k <- ncol(X); nc <- if (nocons) 0 else 1; X1 <- if (nocons) X else cbind(X, 1)
  # the start: least squares on the rows kept, sigma at the root mean squared error
  wn <- sum(w)
  b0 <- tryCatch(as.numeric(solve(crossprod(X1 * w, X1), crossprod(X1 * w, y))), error = function(e) NULL)
  if (is.null(b0)) stop("RCraft: a model with collinear regressors is not translated yet", call. = FALSE)
  rss <- sum(w * (y - drop(X1 %*% b0))^2)
  start <- c(b0, sqrt(rss / (wn - ncol(X1))))
  model <- stata_cx_truncmodel(y, X1, w, lov, hiv)
  f <- stata_cx_max(model$llf, model$derivs, start, maxit)
  # Stata's numerical derivatives and our exact ones agree where the search ends, not on the way there
  if (!f$converged) stop("truncreg stopped before it converged: Stata's unconverged estimates come from numerical derivatives and are not reproduced (try iterate() larger)", call. = FALSE)
  rob <- robust
  V <- stata_cx_vcov(f, rob, w, fwk, cl)
  Vm <- f$inv
  b <- f$b
  N <- if (fwk) sum(w) else length(y)
  nbf <- N + ntrunc
  G <- if (is.null(cl)) NULL else length(unique(cl))
  chi2 <- sum(b[seq_len(k)] * solve(V[seq_len(k), seq_len(k), drop = FALSE], b[seq_len(k)]))
  e <- list(rank = k + nc + 1, N = N, ic = length(f$log) - 1, k_eq = 2, converged = as.numeric(f$converged), rc = 0)
  if (!is.null(G)) e$N_clust <- G
  e <- c(e, list(ll = f$ll, k_eq_model = 1, df_m = k, chi2 = chi2, p = stats::pchisq(chi2, k, lower.tail = FALSE), N_bf = nbf,
                 sigma = b[k + nc + 1], k_aux = 1))
  e$cmdline <- cmdline; e$cmd <- "truncreg"
  if (!is.null(lo)) e$llopt <- stata_fmt(lo, "%9.0g", trim = TRUE)
  if (!is.null(hi)) e$ulopt <- stata_fmt(hi, "%9.0g", trim = TRUE)
  e$marginsok <- "default XB Pr(passthru) E(passthru) YStar(passthru)"; e$predict <- "truncr_p"; e$chi2type <- "Wald"
  e$deriv_useminbound <- "off"; e$opt <- "moptimize"
  if (!is.null(wvar)) { e$wtype <- c(f = "fweight", p = "pweight", i = "iweight", a = "aweight")[[wtype]]; e$wexp <- paste("=", wvar) }
  if (rob) { e$vcetype <- "Robust"; if (!is.null(cluster)) e$clustvar <- cluster }
  e$vce <- if (!is.null(cluster)) "cluster" else if (rob) "robust" else "oim"
  e$title <- "Truncated regression"; e$user <- "trunc_ll"
  if (!is.null(offset)) e$offset1 <- offset
  e$ml_method <- "lf"; e$technique <- "nr"; e$which <- "max"
  e$depvar <- dep; e$properties <- "b V"
  cn <- c(paste0(dep, ":", c(des$nmx, if (!nocons) "_cons")), "sigma:_cons")
  e$b <- matrix(b, 1, k + nc + 1, dimnames = list("y1", cn)); e$V <- matrix(V, k + nc + 1, k + nc + 1, dimnames = list(cn, cn))
  if (rob) e$V_modelbased <- matrix(Vm, k + nc + 1, k + nc + 1, dimnames = list(cn, cn))
  e$means <- matrix(colSums(X * w) / sum(w), 1, k, dimnames = list("r1", des$nmx))
  e$dummy <- matrix(0, 1, k, dimnames = list("r1", des$nmx))
  ilog <- numeric(20); mm <- min(20, length(f$log)); ilog[seq_len(mm)] <- f$log[seq_len(mm)]
  e$ilog <- matrix(ilog, 1, 20, dimnames = list("r1", paste0("c", 1:20)))
  e$gradient <- matrix(f$g, 1, k + nc + 1, dimnames = list("r1", cn))
  attr(e, "stata_rows") <- rows
  rname <- sub("^_cons$", "(Intercept)", c(des$nmx, if (!nocons) "_cons"))
  keep1 <- seq_len(k + nc)
  attr(e, "stata_b") <- stats::setNames(b[keep1], rname)
  attr(e, "stata_V") <- { Vn <- V[keep1, keep1, drop = FALSE]; dimnames(Vn) <- list(rname, rname); Vn }
  attr(e, "stata_areg") <- list(fo = des$fo, xlev = des$xlev, nmx = des$nmx, nocons = nocons)
  attr(e, "stata_disp") <- list(lo = lo, hi = hi, N = N, robust = rob, G = G, cluster = cluster, k = k, nmx = des$nmx, chi2 = chi2, offset = offset, nocons = nocons)
  if (show) {
    cat("\n")
    if (!nolog) { cat("Fitting full model:\n\n"); stata_cx_log(f, if (rob) "Log pseudolikelihood" else "Log likelihood"); cat("\n") }
    stata_truncreg_show(e, level, mf)
  }
  e
}
stata_truncreg_show <- function(e, level = 95, data = NULL) {
  a <- attr(e, "stata_disp"); dep <- e$depvar; k <- a$k
  shown <- function(v, inf) if (is.null(v)) inf else stata_fmt(v, "%9.0g", trim = TRUE)
  lt <- shown(a$lo, "-inf"); ht <- shown(a$hi, "+inf"); w <- max(nchar(lt), nchar(ht))
  r10 <- function(label, value) sprintf("%-18s= %10s", label, value)
  cat("Truncated regression\n")
  cat(sprintf("%-48s%s\n", paste0("Limit: Lower = ", formatC(lt, width = w)), r10("Number of obs", format(a$N, scientific = FALSE))))
  cat(sprintf("%-48s%s\n", paste0("       Upper = ", formatC(ht, width = w)), r10(sprintf("Wald chi2(%d)", k), sprintf("%.2f", a$chi2))))
  lab <- if (a$robust) "Log pseudolikelihood" else "Log likelihood"
  cat(sprintf("%-48s%s\n", paste0(lab, " = ", stata_fmt(e$ll, "%10.0g", trim = TRUE)), r10("Prob > chi2", sprintf("%.4f", e$p))))
  cat("\n")
  stata_cx_note_clusters(a$G, a$cluster)
  stata_cx_rule()
  if (a$robust) cat(strrep(" ", 13), "|               Robust\n", sep = "")
  cat(stata_tab_pad(stata_tab_abbrev(dep, 12), 12), " | Coefficient  ", if (a$robust) "std. err." else "Std. err.", "      z    P>|z|     [",
      stata_fmt(level, "%9.0g", trim = TRUE), "% conf. interval]\n", sep = "")
  stata_cx_rule2()
  b <- attr(e, "stata_b"); V <- attr(e, "stata_V")
  stata_cx_rows(c(a$nmx, if (!isTRUE(a$nocons)) "_cons"), as.numeric(b), sqrt(diag(V)), "z", Inf, level, data, offset = a$offset)
  stata_cx_rule2()
  nb <- length(e$b)
  stata_cx_aux("/sigma", e$b[1, nb], sqrt(e$V[nb, nb]), "z", Inf, level)
  stata_cx_rule()
  stata_cx_warn(e)
  invisible(NULL)
}
stata_truncreg_replay <- function(level = 95, data = NULL) {
  if (!identical(stata_e$cmd, "truncreg")) stop("last estimates not found (Stata: r(301))", call. = FALSE)
  stata_truncreg_show(stata_e, level, if (exists("stata_data")) stata_data else data)
}
