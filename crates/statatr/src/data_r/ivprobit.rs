//! `ivprobit` with one endogenous regressor, as Stata 19 fits it. The method
//! is StatCraft's decoding of Stata's (its `plan/stata/ivprobit.md`, probes
//! 450, 451 and 596), our own work, written here in R:
//!
//! - maximum likelihood on the joint density of the endogenous variable and
//!   the outcome, from the exogenous probit Stata fits first, by ml's Newton
//!   steps (halving, the forward search, modified Marquardt where -H is not
//!   positive definite, ml's convergence rule) with the Hessian from central
//!   differences of the analytic gradient; oim, robust and clustered variances;
//! - Newey's two-step estimator with `twostep`;
//! - Stata's table, the exogeneity test, e() and predict's xb, pr and stdp.
//!
//! Weights, several endogenous variables, factor variables, twostep with
//! first or vce(), and test, margins or estat afterwards are not translated.

use super::returned::Returned;
use super::*;

pub(super) const IVPROBIT_HELPER: &str = r##"# Stata's ml: Newton-Raphson, halving while the likelihood falls, else stepping on by
# 1/8, 1/4, ... of the step while it rises (each added to the point reached); modified
# Marquardt where -H is not positive definite; done when g'H^-1g <= 1e-5 and (reldif(ll) <= 1e-7
# or mreldif(b) <= 1e-6), checked before each step after the first
stata_ml_max <- function(llf, derivs, b, maxit = 300) {
  rd <- function(a, b) abs(a - b) / (abs(b) + 1)
  lg <- numeric(0); prev <- NULL; conv <- FALSE
  d <- derivs(b)
  repeat {
    lg <- c(lg, d$ll)
    A <- d$negH
    concave <- !inherits(tryCatch(chol((A + t(A)) / 2), error = function(e) e), "error")
    if (concave) {
      inv <- solve(A); step <- as.numeric(inv %*% d$g)
      if (!is.null(prev)) {
        nr <- sum(d$g * step)
        if (abs(nr) <= 1e-5 && (rd(d$ll, prev$ll) <= 1e-7 || max(rd(b, prev$b)) <= 1e-6)) { conv <- TRUE; break }
      }
    }
    if (length(lg) > maxit) break
    if (!concave) {
      m <- A; diag(m) <- diag(A) + 1.1 * abs(diag(A))
      dir <- as.numeric(solve(m, d$g)); u <- dir / sqrt(sum(dir^2)); gu <- sum(d$g * u)
      len <- 0.01 * abs(d$ll) / abs(gu); v <- llf(b + len * u); tries <- 0
      while (!(isTRUE(v > d$ll)) && tries < 60) { len <- len / 2; v <- llf(b + len * u); tries <- tries + 1 }
      for (i in 1:200) { f2 <- llf(b + 2 * len * u); if (isTRUE(f2 > v)) { len <- 2 * len; v <- f2 } else break }
      prev <- list(b = b, ll = d$ll); b <- b + len * u; d <- derivs(b); next
    }
    t <- 1; nx <- llf(b + t * step); halved <- FALSE
    while ((is.nan(nx) || nx < d$ll) && t > 1e-15) { halved <- TRUE; t <- t / 2; nx <- llf(b + t * step) }
    pt <- b + t * step
    if (!halved) {
      ex <- 0.125
      repeat { ah <- pt + ex * step; f <- llf(ah); if (isTRUE(f > nx)) { pt <- ah; nx <- f; ex <- ex * 2 } else break }
    }
    prev <- list(b = b, ll = d$ll); b <- pt; d <- derivs(b)
  }
  list(b = b, ll = d$ll, log = lg, converged = conv, negH = d$negH, g = d$g, scores = d$scores)
}
# the numerical -H from an analytic gradient: five points, step 1e-3 max(|b_j|, 1)
stata_ml_numhess <- function(b, grf) {
  p <- length(b); h <- matrix(0, p, p)
  for (j in seq_len(p)) {
    st <- 1e-3 * max(abs(b[j]), 1)
    sh <- function(k) { c <- b; c[j] <- c[j] + k * st; grf(c) }
    m2 <- sh(-2); m1 <- sh(-1); p1 <- sh(1); p2 <- sh(2)
    h[, j] <- -(m2 - 8 * m1 + 8 * p1 - p2) / (12 * st)
  }
  (h + t(h)) / 2
}
stata_iv_lnr <- function(z) { lp <- stats::pnorm(z, log.p = TRUE); list(lp = lp, ratio = exp(stats::dnorm(z, log = TRUE) - lp)) }
# probit of y on X (constant last), Stata's start: the constant at invnorm(mean y)
stata_probit <- function(y, X, maxit = 300) {
  q <- ifelse(y != 0, 1, -1)
  llf <- function(b) sum(stata_iv_lnr(q * as.numeric(X %*% b))$lp)
  derivs <- function(b) {
    xb <- as.numeric(X %*% b); r <- stata_iv_lnr(q * xb); lam <- q * r$ratio
    g <- as.numeric(crossprod(X, lam)); w <- lam * (lam + xb)
    list(ll = sum(r$lp), g = g, negH = crossprod(X * w, X), scores = X * lam)
  }
  b <- c(rep(0, ncol(X) - 1), stats::qnorm(mean(y != 0)))
  stata_ml_max(llf, derivs, b, maxit)
}
stata_ivprobit_fit <- function(y, endog, X, Z, maxit = 300) {
  # X: exogenous columns then the constant; Z: the reduced form's, constant last
  n <- length(y); kx <- ncol(X); kz <- ncol(Z)
  pi <- as.numeric(solve(crossprod(Z), crossprod(Z, endog)))
  res <- as.numeric(endog - Z %*% pi); mse <- sum(res^2) / n
  pr <- stata_probit(y, cbind(endog, X[, -kx, drop = FALSE], res, 1), maxit)
  mse <- mse * n / (n - kz); sigma <- sqrt(mse)
  c <- pr$b[kx + 1]; scale <- 1 / sqrt(1 + c^2 * mse)
  start <- c(pr$b[seq_len(kx)] * scale, pr$b[length(pr$b)] * scale, pi, asinh(c * sigma), 0.5 * log(mse))
  q <- ifelse(y != 0, 1, -1); A <- cbind(endog, X)
  parts <- function(b) list(beta = b[seq_len(kx + 1)], pi = b[kx + 1 + seq_len(kz)], athrho = b[kx + kz + 2], lnsig = b[kx + kz + 3])
  obs <- function(b) {
    p <- parts(b); sigma <- exp(p$lnsig); rho <- tanh(p$athrho); s <- sqrt(1 - rho^2)
    e <- as.numeric(endog - Z %*% p$pi) / sigma
    w <- (as.numeric(A %*% p$beta) + rho * e) / s
    r <- stata_iv_lnr(q * w); lam <- q * r$ratio
    ll <- -0.5 * log(2 * base::pi) - p$lnsig - 0.5 * e^2 + r$lp
    sc <- cbind(A * (lam / s), Z * ((e - lam * rho / s) / sigma), lam * (s * e + w * rho), -1 + e^2 - lam * rho * e / s)
    list(ll = ll, sc = sc)
  }
  llf <- function(b) sum(obs(b)$ll)
  grf <- function(b) colSums(obs(b)$sc)
  derivs <- function(b) { o <- obs(b); list(ll = sum(o$ll), g = colSums(o$sc), negH = stata_ml_numhess(b, grf), scores = o$sc) }
  m <- stata_ml_max(llf, derivs, start, maxit)
  m$probit <- pr
  m
}
stata_iv_twostep <- function(y, endog, X, Z) {
  n <- length(y); kz <- ncol(Z); kx <- ncol(X)
  pi <- as.numeric(solve(crossprod(Z), crossprod(Z, endog)))
  v <- as.numeric(endog - Z %*% pi); s2 <- sum(v^2) / (n - kz)
  pa <- stata_probit(y, cbind(Z[, -kz, drop = FALSE], v, 1))
  Va <- solve(pa$negH); pk <- length(pa$b); pos <- c(seq_len(kz - 1), pk)
  alpha <- pa$b[pos]; lambda <- pa$b[kz]
  pc <- stata_probit(y, cbind(endog, X[, -kx, drop = FALSE], v, 1))
  Vc <- solve(pc$negH); beta <- pc$b[1]; rho <- pc$b[kx + 1]
  chi2_exog <- rho^2 / Vc[kx + 1, kx + 1]
  omega <- Va[pos, pos] + (lambda - beta)^2 * s2 * solve(crossprod(Z))
  D <- matrix(0, kz, kx + 1); D[, 1] <- pi
  for (cc in 2:(kx + 1)) if (cc == kx + 1) D[kz, cc] <- 1 else D[cc - 1, cc] <- 1
  oi <- solve(omega); V <- solve(t(D) %*% oi %*% D); b <- as.numeric(V %*% (t(D) %*% oi %*% alpha))
  list(b = b, V = V, chi2 = sum(b[1:kx] * solve(V[1:kx, 1:kx, drop = FALSE], b[1:kx])), chi2_exog = chi2_exog)
}
stata_iv_coef <- function(w, name, b, se, level) {
  g9 <- function(v) stata_fmt(v, "%9.0g", trim = TRUE)
  crit <- stats::qnorm((100 + level) / 200); z <- b / se
  short <- function(s, n) if (nchar(s) > n) paste0(substr(s, 1, n - 2), "~", substr(s, nchar(s), nchar(s))) else s
  cat(sprintf("%*s |%11s%11s%9.2f%8.3f%13s%12s\n", w - 1, short(name, w - 1), g9(b), g9(se), z, 2 * stats::pnorm(-abs(z)),
              g9(b - crit * se), g9(b + crit * se)))
}
stata_iv_wrapped <- function(head, words) {
  lines <- head; ind <- nchar(head)
  for (w in words) {
    k <- length(lines); first <- nchar(lines[k]) == ind
    if (!first && nchar(lines[k]) + 1 + nchar(w) > 79) lines <- c(lines, paste0(strrep(" ", ind), w))
    else lines[k] <- paste0(lines[k], if (first) "" else " ", w)
  }
  cat(lines, sep = "\n")
}
stata_iv_show <- function(e, first = FALSE, level = 95) {
  b <- e$b; V <- e$V; nm <- colnames(b); np <- length(b)
  rule <- function(w, m) paste0(strrep("-", w), m, strrep("-", 64))
  stat <- function(l, v) paste0(formatC(l, width = -13), " = ", formatC(v, width = 6))
  tail_lines <- function(left) {
    right <- sprintf("Prob > chi2 = %.4f", e$p_exog)
    cat(left, formatC(right, width = max(78 - nchar(left), 0)), "\n", sep = "")
    stata_iv_wrapped("Endogenous: ", e$endog)
    stata_iv_wrapped("Exogenous:  ", strsplit(e$exog, " ", fixed = TRUE)[[1]])
  }
  lvl <- stata_fmt(level, "%9.0g", trim = TRUE)
  if (identical(e$method, "twostep")) {
    row <- function(l, v) paste0(formatC(l, width = -16), "=", formatC(v, width = 11))
    cat("\n", formatC("Two-step probit with endogenous regressors", width = -50), row("Number of obs", format(e$N, big.mark = ",")), "\n", sep = "")
    cat(strrep(" ", 50), row(paste0("Wald chi2(", e$df_m, ")"), sprintf("%.2f", e$chi2)), "\n", sep = "")
    cat(strrep(" ", 50), row("Prob > chi2", sprintf("%.4f", e$p)), "\n\n", sep = "")
    w <- max(nchar(nm) + 1, 13)
    cat(rule(w, "-"), "\n", strrep(" ", w), "|", formatC(" Coefficient", width = -12), "  Std. err.      z    P>|z|     [", lvl, "% conf. interval]\n", rule(w, "+"), "\n", sep = "")
    for (j in seq_len(np)) stata_iv_coef(w, nm[j], b[1, j], sqrt(V[j, j]), level)
    cat(rule(w, "-"), "\n", sep = "")
    tail_lines(sprintf("Wald test of exogeneity: chi2(1) = %.2f", e$chi2_exog))
    return(invisible(NULL))
  }
  dep <- strsplit(e$depvar, " ")[[1]][1]; endog <- e$endog
  dn <- sub(paste0("^", dep, ":"), "", nm[startsWith(nm, paste0(dep, ":"))])
  rn <- sub(paste0("^", endog, ":"), "", nm[startsWith(nm, paste0(endog, ":"))])
  robust <- !identical(e$vce, "oim")
  ll_label <- if (robust) "Log pseudolikelihood" else "Log likelihood"
  cat(formatC("Probit model with endogenous regressors", width = -56), stat("Number of obs", format(e$N, big.mark = ",")), "\n", sep = "")
  cat(strrep(" ", 56), stat(paste0("Wald chi2(", e$df_m, ")"), sprintf("%.2f", e$chi2)), "\n", sep = "")
  cat(formatC(paste0(ll_label, " = ", stata_fmt(e$ll, "%10.0g")), width = -56), stat("Prob > chi2", sprintf("%.4f", e$p)), "\n\n", sep = "")
  a <- np - 1; l <- np; crit <- stats::qnorm((100 + level) / 200)
  sa <- sqrt(V[a, a]); sl <- sqrt(V[l, l]); rho <- tanh(b[1, a]); sigma <- exp(b[1, l])
  cn <- paste0("corr(e.", endog, ",e.", dep, ")"); sn <- paste0("sd(e.", endog, ")")
  w <- max(nchar(c(cn, sn, "/athrho2_1", dn, rn)) + 1, 13)
  if (!is.null(e$N_clust)) cat(formatC(paste0("(Std. err. adjusted for ", e$N_clust, " clusters in ", e$clustvar, ")"), width = w + 65), "\n", sep = "")
  cat(rule(w, "-"), "\n", sep = "")
  if (robust) cat(strrep(" ", w), "|", formatC("Robust", width = 21), "\n", sep = "")
  cat(strrep(" ", w), "|", formatC(" Coefficient", width = -12), "  ", if (robust) "std. err." else "Std. err.", "      z    P>|z|     [", lvl, "% conf. interval]\n", rule(w, "+"), "\n", sep = "")
  show <- function(names, from) for (j in seq_along(names)) stata_iv_coef(w, names[j], b[1, from + j], sqrt(V[from + j, from + j]), level)
  if (first) {
    cat(formatC(dep, width = -w), "|\n", sep = ""); show(dn, 0); cat(rule(w, "+"), "\n", sep = "")
    cat(formatC(endog, width = -w), "|\n", sep = ""); show(rn, length(dn)); cat(rule(w, "+"), "\n", sep = "")
    stata_iv_coef(w, "/athrho2_1", b[1, a], sa, level); stata_iv_coef(w, "/lnsigma2", b[1, l], sl, level); cat(rule(w, "+"), "\n", sep = "")
  } else { show(dn, 0); cat(rule(w, "+"), "\n", sep = "") }
  g9 <- function(v) stata_fmt(v, "%9.0g", trim = TRUE)
  plain <- function(name, est, se, lo, hi) cat(sprintf("%*s|%11s%11s%17s%13s%12s\n", w, name, g9(est), g9(se), "", g9(lo), g9(hi)))
  plain(cn, rho, (1 - rho^2) * sa, tanh(b[1, a] - crit * sa), tanh(b[1, a] + crit * sa))
  plain(sn, sigma, sigma * sl, exp(b[1, l] - crit * sl), exp(b[1, l] + crit * sl))
  cat(rule(w, "-"), "\n", sep = "")
  tail_lines(sprintf("Wald test of exogeneity (corr = 0): chi2(1) = %.2f", e$chi2_exog))
}
stata_iv_replay <- function(first = FALSE, level = 95) {
  if (!identical(stata_e$cmd, "ivprobit")) stop("last estimates not found (Stata: r(301))", call. = FALSE)
  if (!identical(stata_e$method, "twostep")) cat("\n")
  stata_iv_show(stata_e, first, level)
}
# ivprobit depvar [exog] (endog = instruments): the ML fit (or Newey's two-step), Stata's table and e()
stata_ivprobit <- function(d, sel, y, endog, exog, insts, vce = "oim", cluster = NULL, level = 95, first = FALSE,
                           nolog = FALSE, maxit = 300, twostep = FALSE, cmdline = "", show = TRUE) {
  red <- c(exog, setdiff(insts, exog))
  vars <- unique(c(y, endog, red, cluster))
  for (v in vars) if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
  num <- function(v) { u <- unclass(d[[v]]); attributes(u) <- NULL; as.numeric(u) }
  M <- sapply(c(y, endog, red), num); if (!is.matrix(M)) M <- matrix(M, nrow = 1)
  ok <- sel & stats::complete.cases(M)
  if (!is.null(cluster)) ok <- ok & !is.na(d[[cluster]])
  rows <- which(ok); n <- length(rows)
  if (!n) stop("no observations (Stata: r(2000))", call. = FALSE)
  yv <- M[rows, 1]; ev <- M[rows, 2]
  X <- cbind(M[rows, exog, drop = FALSE], 1); Z <- cbind(M[rows, red, drop = FALSE], 1)
  kx <- ncol(X)
  # an instrument collinear with the other instruments: Stata drops the later one of the set, says
  # so, and fits without it (a 0 column o.name stays in e(b) and e(V)); any other collinearity
  # (a regressor dropped, an instrument collinear with a regressor) is refused
  zn <- c(red, "_cons"); omit <- character(0)
  qz <- qr(Z)
  if (qz$rank < ncol(Z)) {
    gone <- zn[setdiff(seq_len(ncol(Z)), qz$pivot[seq_len(qz$rank)])]
    extra <- setdiff(insts, exog)
    qi <- qr(M[rows, extra, drop = FALSE])
    alone <- extra[setdiff(seq_along(extra), qi$pivot[seq_len(qi$rank)])]
    if (!all(gone %in% extra) || !setequal(gone, alone))
      stop("ivprobit with collinear regressors, or an instrument collinear with a regressor, is not translated (Stata's omitted-variable rows are not matched)", call. = FALSE)
    if (first) stop("ivprobit, first with an omitted instrument is not translated", call. = FALSE)
    omit <- gone
  }
  Zf <- Z[, !(zn %in% omit), drop = FALSE]
  redn <- ifelse(red %in% omit, paste0("o.", red), red)
  dn <- c(endog, exog, "_cons"); rn <- c(redn, "_cons")
  say_omitted <- function() if (show && length(omit)) cat(sprintf("note: %s omitted because of collinearity.\n", omit), sep = "")
  if (twostep) {
    if (show) cat("Checking reduced-form model...\n")
    say_omitted()
    f <- stata_iv_twostep(yv, ev, X, Zf)
    e <- list(N = n, chi2_exog = f$chi2_exog, df_exog = 1, p_exog = stats::pchisq(f$chi2_exog, 1, lower.tail = FALSE),
              chi2 = f$chi2, df_m = kx, p = stats::pchisq(f$chi2, kx, lower.tail = FALSE), rank = kx + 1, N_cds = 0, N_cdf = 0,
              cmdline = cmdline, cmd = "ivprobit", marginsprop = "nochainrule",
              marginsok = "asf fixedasf Pr XB default STRuctural fix(passthru) base(passthru) TARGet(passthru)",
              predict = "ivprobit_p", estat_cmd = "ivprobit_estat", footnote = "ivprobit_footnote", chi2type = "Wald",
              vce = "twostep", depvar = paste(y, endog), exog = paste(redn, collapse = " "), insts = paste(redn, collapse = " "), endog = endog, method = "twostep",
              properties = "b V", b = matrix(f$b, 1, kx + 1, dimnames = list(endog, dn)),
              V = matrix((f$V + t(f$V)) / 2, kx + 1, kx + 1, dimnames = list(dn, dn)), rules = matrix(0, 2, 4))
  } else {
    say_omitted()
    f <- stata_ivprobit_fit(yv, ev, X, Zf, maxit)
    A <- solve(f$negH); G <- NULL
    V <- if (vce == "oim") A else {
      S <- f$scores
      if (vce == "cluster") { S <- rowsum(S, d[[cluster]][rows]); G <- nrow(S); A %*% crossprod(S) %*% A * G / (G - 1) }
      else A %*% crossprod(S) %*% A * n / (n - 1)
    }
    V <- (V + t(V)) / 2; np <- length(f$b)
    chi2 <- sum(f$b[1:kx] * solve(V[1:kx, 1:kx, drop = FALSE], f$b[1:kx])); chi2x <- f$b[np - 1]^2 / V[np - 1, np - 1]
    robust <- vce != "oim"
    if (show && !nolog) {
      lab <- if (robust) "Log pseudolikelihood" else "Log likelihood"
      lg <- function(v, lab) for (i in seq_along(v)) cat(formatC(paste0("Iteration ", i - 1, ":"), width = -14), lab, " = ",
                                                        formatC(stata_fmt(v[i], "%10.0g", trim = TRUE), width = 10), "  \n", sep = "")
      cat("\nFitting exogenous probit model:\n"); lg(f$probit$log, "Log likelihood")
      cat("\nFitting full model:\n"); lg(f$log, lab); cat("\n")
    } else if (show) cat("\n")
    cn <- c(paste0(y, ":", dn), paste0(endog, ":", rn), "/:athrho2_1", "/:lnsigma2")
    # e(b), e(V) and e(gradient) carry a 0 for each omitted instrument; e(rank) counts the fitted ones
    nf <- length(cn); pos <- seq_len(np)
    if (length(omit)) pos <- c(seq_len(kx + 1), kx + 1 + which(!(zn %in% omit)), nf - 1, nf)
    spread <- function(m) { out <- matrix(0, nf, nf); out[pos, pos] <- m; out }
    bF <- numeric(nf); bF[pos] <- f$b; gF <- numeric(nf); gF[pos] <- f$g
    e <- list(rank = np, N = n, ic = length(f$log) - 1, k = nf, k_eq = 4, k_dv = 2, converged = as.numeric(f$converged), rc = 0)
    if (!is.null(G)) e$N_clust <- G
    e <- c(e, list(ll = f$ll, chi2_exog = chi2x, p_exog = stats::pchisq(chi2x, 1, lower.tail = FALSE), endog_ct = 1, chi2 = chi2,
                   df_m = kx, p = stats::pchisq(chi2, kx, lower.tail = FALSE), k_eq_model = 1, N_cds = 0, N_cdf = 0,
                   cmdline = cmdline, cmd = "ivprobit", marginsprop = "nochainrule",
                   marginsok = "asf fixedasf Pr XB default STRuctural fix(passthru) base(passthru) TARGet(passthru)",
                   predict = "ivprobit_p", estat_cmd = "ivprobit_estat", footnote = "ivprobit_footnote", chi2type = "Wald",
                   method = "ml", title = "Probit model with endogenous regressors", depvar = paste(y, endog), endog = endog,
                   exog = paste(redn, collapse = " "), insts = paste(redn, collapse = " "), deriv_useminbound = "off", opt = "moptimize",
                   vce = if (!is.null(cluster)) "cluster" else if (robust) "robust" else "oim"))
    if (robust) e$vcetype <- "Robust"
    if (!is.null(cluster)) e$clustvar <- cluster
    e <- c(e, list(user = "ivfprobit_eval()", ml_method = "lf1", technique = "nr", which = "max", properties = "b V"))
    rho <- tanh(f$b[np - 1]); sg <- exp(f$b[np])
    ilog <- numeric(20); m <- min(20, length(f$log)); ilog[1:m] <- f$log[1:m]
    e <- c(e, list(b = matrix(bF, 1, nf, dimnames = list("y1", cn)), V = matrix(spread(V), nf, nf, dimnames = list(cn, cn)),
                   rules = matrix(0, 1, 4), Sigma = matrix(c(1, rho * sg, rho * sg, sg^2), 2, 2, dimnames = list(paste0("e.", c(y, endog)), paste0("e.", c(y, endog)))),
                   ilog = matrix(ilog, 1, 20, dimnames = list("r1", paste0("c", 1:20))), gradient = matrix(gF, 1, nf, dimnames = list("r1", cn))))
    if (robust) e$V_modelbased <- matrix(spread((A + t(A)) / 2), nf, nf, dimnames = list(cn, cn))
  }
  attr(e, "stata_sample") <- TRUE
  stata_e <<- e
  smp <- logical(nrow(d)); smp[rows] <- TRUE; stata_esample <<- smp
  if (show) stata_iv_show(e, first, level)
  invisible(NULL)
}
# predict after ivprobit: xb (the default), pr (average structural function) and stdp
stata_iv_predict <- function(d, stat = "xb") {
  e <- stata_e; b <- e$b; nm <- colnames(b); V <- e$V
  num <- function(v) { u <- unclass(d[[v]]); attributes(u) <- NULL; as.numeric(u) }
  dep <- strsplit(e$depvar, " ")[[1]][1]
  # the 0 columns of omitted instruments (o.name) add nothing
  pick <- function(eq) { j <- which(startsWith(nm, paste0(eq, ":")) & !grepl(":o\\.", nm)); list(j = j, v = sub("^[^:]*:", "", nm[j])) }
  design <- function(p) sapply(p$v, function(v) if (v == "_cons") rep(1, nrow(d)) else num(v))
  if (stat == "pr" && identical(e$method, "twostep"))
    stop("probabilities and average structural function not available with two-step estimator (Stata: r(198))", call. = FALSE)
  if (identical(e$method, "twostep")) { p <- list(j = seq_along(nm), v = nm) } else p <- pick(dep)
  Xd <- matrix(design(p), nrow(d)); xb <- as.numeric(Xd %*% b[1, p$j])
  if (stat == "xb") return(xb)
  if (stat == "stdp") return(sqrt(rowSums((Xd %*% V[p$j, p$j]) * Xd)))
  np <- length(b); rho <- tanh(b[1, np - 1]); sigma <- exp(b[1, np])
  r <- pick(e$endog); Zr <- matrix(design(r), nrow(d))
  stats::pnorm((xb + rho * (num(e$endog) - as.numeric(Zr %*% b[1, r$j])) / sigma) / sqrt(1 - rho^2))
}
"##;

impl Session {
    /// Whether the active results are ivprobit's.
    pub(super) fn ivprobit_active(&self) -> bool {
        matches!(&self.e_ret, Returned::Program { cmd, .. } if cmd == "ivprobit")
    }

    fn iv_fail(&mut self, out: &mut Out, msg: &str, rc: u32) -> Result<(), Untranslated> {
        out.line(format!(
            "stop({}, call. = FALSE)",
            r_text(&format!("{msg} (Stata: r({rc}))"))
        ));
        // after some of these Stata has no results left (predict then says r(301),
        // corpus ivprobit.do): what e() holds is not known, so what reads it is refused
        Ok(())
    }

    /// `ivprobit depvar [exog] (endog = instruments) [if] [in] [, twostep first
    /// vce() level() nolog iterate()]`, or a replay.
    #[allow(clippy::too_many_lines)]
    pub(super) fn ivprobit(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "ivprobit")?;
        if c.parts.weight.is_some() {
            return Err(no("ivprobit with weights is not translated yet"));
        }
        if let Some(o) = opts.other(&[
            ("twostep", 3),
            ("first", 5),
            ("vce", 3),
            ("robust", 1),
            ("cluster", 2),
            ("level", 1),
            ("nolog", 5),
            ("iterate", 4),
        ]) {
            return Err(no(format!(
                "ivprobit with the option {o} is not translated yet"
            )));
        }
        let level = match opts.arg("level", 1) {
            None => 95.0,
            Some(l) => match l.trim().parse::<f64>() {
                Ok(v) if (10.0..=99.99).contains(&v) => v,
                _ => return Err(no(format!("ivprobit, level({l}) is not translated"))),
            },
        };
        let first = opts.has("first", 5);
        let main = c.parts.main.trim();
        if main.is_empty() {
            if opts
                .0
                .iter()
                .any(|o| !(o.is("first", 5) || o.is("level", 1)))
            {
                return Err(no(
                    "ivprobit's replay is translated with first and level() only",
                ));
            }
            self.helpers.insert("stata_ivprobit");
            self.helpers.insert("stata_fmt");
            out.line(format!(
                "stata_iv_replay(first = {}, level = {})",
                super::stats::r_logical(first),
                r_number(level)
            ));
            self.e_kept = true;
            return Ok(());
        }
        let twostep = opts.has("twostep", 3);
        let (open, close) = match (main.find('('), main.rfind(')')) {
            (Some(a), Some(b)) if a < b => (a, b),
            _ => return self.iv_fail(out, "equation not found", 198),
        };
        let inside = main.get(open + 1..close).unwrap_or("");
        let outside = format!(
            "{} {}",
            main.get(..open).unwrap_or(""),
            main.get(close + 1..).unwrap_or("")
        );
        if outside.contains('(') || inside.contains('(') {
            return Err(no(
                "ivprobit with more than one parenthesised group is not translated",
            ));
        }
        let Some((endog, inst)) = inside.split_once('=') else {
            return Err(no("ivprobit needs (endogenous = instruments)"));
        };
        let words: Vec<&str> = outside.split_whitespace().collect();
        let plain = |w: &&str| is_name(w);
        if !words.iter().all(plain)
            || !endog.split_whitespace().all(is_name)
            || !inst.split_whitespace().all(is_name)
        {
            return Err(no(
                "ivprobit is translated with plain variable names (no factor variables or operators)",
            ));
        }
        let Some((dep, exog)) = words.split_first() else {
            return self.iv_fail(out, "varlist required", 100);
        };
        let endog: Vec<&str> = endog.split_whitespace().collect();
        let inst: Vec<&str> = inst.split_whitespace().collect();
        let repeats = |v: &[&str]| {
            let mut seen = std::collections::HashSet::new();
            v.iter().any(|w| !seen.insert(*w))
        };
        if repeats(exog) || repeats(&inst) {
            return Err(no(
                "ivprobit with a variable listed twice is not translated (Stata's omitted-variable notes differ)",
            ));
        }
        if inst.is_empty() {
            return self.iv_fail(
                out,
                "no instrument variables specified\n    The syntax is: (all instrumented variables = instrument variables).",
                481,
            );
        }
        let Some(en) = endog.first().copied() else {
            return self.iv_fail(out, "varlist required", 100);
        };
        if exog.contains(&en) {
            out.line(format!(
                "cat(\"note: {en} omitted because of collinearity.\\n\")"
            ));
            return self.iv_fail(out, "dropping an endogenous variable is not allowed", 498);
        }
        let extra = inst.iter().filter(|z| !exog.contains(z)).count();
        if extra == 0 {
            return self.iv_fail(
                out,
                "no instrument variables specified\n    The syntax is: (all instrumented variables = instrument variables).",
                481,
            );
        }
        if extra < endog.len() {
            return self.iv_fail(
                out,
                "equation not identified; must have at least as many instruments\nnot in the regression as there are instrumented variables",
                481,
            );
        }
        if endog.len() > 1 {
            return Err(no(
                "ivprobit with more than one endogenous variable is not translated",
            ));
        }
        let (vce, cluster) = match opts.arg("vce", 3).as_deref().map(str::trim) {
            None if opts.has("cluster", 2) => match opts.arg("cluster", 2) {
                Some(v) if is_name(v.trim()) => ("cluster", Some(v.trim().to_string())),
                _ => {
                    return Err(no(
                        "ivprobit, cluster() with that argument is not translated",
                    ))
                }
            },
            None if opts.has("robust", 1) => ("robust", None),
            None => ("oim", None),
            Some("oim") => ("oim", None),
            Some(v) if !v.is_empty() && "robust".starts_with(v) => ("robust", None),
            Some(v) => match v.split_once(char::is_whitespace) {
                Some((w, cv)) if w.len() >= 2 && "cluster".starts_with(w) && is_name(cv.trim()) => {
                    ("cluster", Some(cv.trim().to_string()))
                }
                _ => return Err(no(format!("ivprobit, vce({v}) is not translated yet"))),
            },
        };
        if twostep && vce != "oim" {
            return Err(no("ivprobit, twostep with vce() is not translated"));
        }
        if twostep && first {
            return Err(no(
                "ivprobit, twostep first is not translated (its first-stage regression)",
            ));
        }
        let maxit = match opts.arg("iterate", 4) {
            None => 300,
            Some(a) => match a.trim().parse::<u32>() {
                Ok(n) => n,
                Err(_) => return Err(no(format!("ivprobit, iterate({a}) is not translated"))),
            },
        };
        if maxit < 300 && !twostep {
            return Err(no(
                "ivprobit with iterate() is not translated (Stata's message when it stops is not decoded)",
            ));
        }
        let d = self.data()?;
        let sel = match self.condition(c, out)? {
            Some(cond) => {
                out.package("dplyr");
                format!("local({{ s <- dplyr::transmute({d}, s = {cond})$s; !is.na(s) & s }})")
            }
            None => format!("rep(TRUE, nrow({d}))"),
        };
        for h in ["stata_ivprobit", "stata_fmt", "stata_returned"] {
            self.helpers.insert(h);
        }
        let names = |v: &[&str]| -> String {
            if v.is_empty() {
                "character()".into()
            } else {
                format!(
                    "c({})",
                    v.iter().map(|s| r_text(s)).collect::<Vec<_>>().join(", ")
                )
            }
        };
        out.line(format!(
            "stata_ivprobit({d}, {sel}, {}, {}, {}, {}, vce = {}, cluster = {}, level = {}, first = {}, nolog = {}, maxit = {maxit}, twostep = {}, cmdline = {}, show = {})",
            r_text(dep),
            r_text(en),
            names(exog),
            names(&inst),
            r_text(vce),
            cluster.as_deref().map_or("NULL".to_string(), r_text),
            r_number(level),
            super::stats::r_logical(first),
            super::stats::r_logical(opts.has("nolog", 5)),
            super::stats::r_logical(twostep),
            r_text(&super::returned::cmdline(c)),
            super::stats::r_logical(!self.quiet(c)),
        ));
        self.esample_new = true;
        self.e_new = true;
        self.estimates = None;
        self.estimates_lost = Some(
            "test, lincom, margins and estat are not translated after ivprobit yet (predict is)"
                .into(),
        );
        self.e_text.clear();
        self.fit_ts = None;
        self.e_ret = Returned::Program {
            cmd: "ivprobit".into(),
            texts: IV_TEXTS.iter().map(|s| s.to_string()).collect(),
        };
        out.caveat("ivprobit's fit as StatCraft decoded Stata 19's (its variance to about 1e-9)");
        Ok(())
    }

    /// `predict [type] newvar [if] [in] [, xb pr stdp]` after ivprobit.
    pub(super) fn ivprobit_predict(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let mut chosen: Vec<&str> = Vec::new();
        for o in opts.0 {
            if o.is("scores", 6) {
                return self.iv_fail(
                    out,
                    "too few variables specified\n    The current estimation results for ivprobit have 4 equations so you must specify 4 new variables, or you can use the equation() option and specify one variable at a time.",
                    102,
                );
            } else if o.is("xb", 2) {
                chosen.push("xb");
            } else if o.is("pr", 2) {
                chosen.push("pr");
            } else if o.is("stdp", 3) {
                chosen.push("stdp");
            } else if o.is("asf", 3) || o.is("fix", 3) {
                return Err(no(
                    "predict's asf and fix() after ivprobit are not translated",
                ));
            } else {
                return self.iv_fail(out, &format!("option {} not allowed", o.name), 198);
            }
        }
        if chosen.len() > 1 {
            return self.iv_fail(out, "you may specify at most one of {pr, xb, or stdp}", 198);
        }
        let mut words: Vec<&str> = c.parts.main.split_whitespace().collect();
        let ty = match words.first() {
            Some(t) if storage(t).is_some() => {
                let t = *t;
                words.remove(0);
                t
            }
            _ => "float",
        };
        let st = storage(ty).ok_or_else(|| no(format!("{ty} is not a storage type")))?;
        if st == Storage::Str {
            return Err(no("predict makes a numeric variable, not a string"));
        }
        let [w] = words.as_slice() else {
            return Err(no("predict makes one new variable"));
        };
        if !is_name(w) {
            return Err(no(format!(
                "{w}: only a plain new variable name is translated"
            )));
        }
        let stat = chosen.first().copied().unwrap_or("xb");
        let base = self.data()?;
        out.line(format!(
            "if ({} %in% names({base})) stop(\"variable {w} already defined (Stata: r(110))\", call. = FALSE)",
            r_text(w)
        ));
        let quiet = self.quiet(c);
        if chosen.is_empty() && !quiet {
            out.line("cat(\"(option xb assumed; fitted values)\\n\")");
        }
        self.helpers.insert("stata_ivprobit");
        out.line(format!(
            "stata_iv_p <- stata_iv_predict({base}, {})",
            r_text(stat)
        ));
        let v = stored("stata_iv_p", st, &mut self.helpers, out);
        let v = match self.condition(c, out)? {
            Some(cond) => format!("dplyr::if_else({cond}, {v}, NA_real_)"),
            None => v,
        };
        let label = match stat {
            "pr" => "Average structural function probabilities",
            "stdp" => "S.E. of the prediction",
            _ if chosen.is_empty() => "Fitted values",
            _ => "Linear prediction",
        };
        out.package("dplyr");
        out.line(format!(
            "{base} <- dplyr::mutate({base}, {} = structure({v}, label = {}))",
            r_name(w),
            r_text(label)
        ));
        out.line("rm(stata_iv_p)");
        if !quiet {
            self.helpers.insert("stata_chg");
            self.helpers.insert("stata_mv");
            out.line(format!("stata_gen_note({base}[[{}]])", r_text(w)));
        }
        self.kinds.insert(w.to_string(), Kind::Number);
        self.storage.insert(w.to_string(), st);
        Ok(())
    }
}

/// The e() macros ivprobit leaves, read back as text.
const IV_TEXTS: &[&str] = &[
    "cmdline",
    "cmd",
    "marginsprop",
    "marginsok",
    "predict",
    "estat_cmd",
    "footnote",
    "chi2type",
    "method",
    "title",
    "depvar",
    "endog",
    "exog",
    "insts",
    "deriv_useminbound",
    "opt",
    "vce",
    "vcetype",
    "clustvar",
    "user",
    "ml_method",
    "technique",
    "which",
    "properties",
];

#[cfg(test)]
mod tests {
    use crate::data_r::translate;
    use crate::reader::read;

    fn r(source: &str) -> Vec<String> {
        translate(&read(source))
            .lines
            .into_iter()
            .map(|l| match l.result {
                Ok(r) => r.code,
                Err(e) => format!("NOT: {}", e.reason),
            })
            .collect()
    }

    #[test]
    fn ivprobit_fits_replays_and_predicts() {
        let out = r("use d
ivprobit y x (y1 = z1 z2), vce(cluster g) first
ivprobit
predict p, pr
ivprobit y x (y1 = z1 z2), twostep");
        assert!(
            out[1].contains("stata_ivprobit(")
                && out[1].contains("\"y\", \"y1\", c(\"x\"), c(\"z1\", \"z2\")")
                && out[1].contains("vce = \"cluster\", cluster = \"g\"")
                && out[1].contains("first = TRUE"),
            "{out:?}"
        );
        assert!(out[2].contains("stata_iv_replay("), "{out:?}");
        assert!(
            out[3].contains("stata_iv_predict(") && out[3].contains("\"pr\""),
            "{out:?}"
        );
        assert!(out[4].contains("twostep = TRUE"), "{out:?}");
    }

    #[test]
    fn repeats_and_predict_options_stata_answers_differently_are_refused() {
        let out = r("use d
ivprobit y x (y1 = z1 z2 z2)
ivprobit y x x (y1 = z1 z2)
ivprobit y x (y1 = z1 z2)
predict p, asf
predict q, pr fix(x)");
        assert!(
            out[1].starts_with("NOT: ivprobit with a variable listed twice"),
            "{out:?}"
        );
        assert!(
            out[2].starts_with("NOT: ivprobit with a variable listed twice"),
            "{out:?}"
        );
        assert!(
            out[4].starts_with("NOT: predict's asf and fix()"),
            "{out:?}"
        );
        assert!(
            out[5].starts_with("NOT: predict's asf and fix()"),
            "{out:?}"
        );
    }

    #[test]
    fn statas_errors_are_kept_and_the_rest_refused() {
        let out = r("use d
ivprobit y x (y1 = )
ivprobit y x (y1 = x)
ivprobit y x (x = z1)
ivprobit y x (y1 y3 = z1 z2)
ivprobit y x (y1 = z1 z2) [pw = w]
ivprobit y x (y1 = z1 z2)
predict p, scores
predict p, pr xb");
        assert!(
            out[1].contains("no instrument variables specified"),
            "{out:?}"
        );
        assert!(out[2].contains("(Stata: r(481))"), "{out:?}");
        assert!(
            out[3].contains("omitted because of collinearity")
                && out[3].contains("(Stata: r(498))"),
            "{out:?}"
        );
        assert!(
            out[4].starts_with("NOT: ivprobit with more than one endogenous"),
            "{out:?}"
        );
        assert!(out[5].starts_with("NOT: ivprobit with weights"), "{out:?}");
        assert!(out[7].contains("(Stata: r(102))"), "{out:?}");
        assert!(
            out[8].contains("at most one of {pr, xb, or stdp}"),
            "{out:?}"
        );
    }
}
