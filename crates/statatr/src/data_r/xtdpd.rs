//! `xtabond`, `xtdpdsys` and `xtdpd`, the dynamic panel-data GMM estimators,
//! with `estat sargan`, `estat abond` and `predict`, as Stata 19 fits them.
//! The method is StatCraft's decoding of Stata's (its `plan/stata/panel.md`,
//! probes 326 and 420 to 430), our own work, written here in R:
//!
//! - per panel, the differenced equations (t = p+2..T) with GMM-type
//!   instruments, lags of the outcome two or more periods back (one column
//!   per period and lag) and standard ones (D.x); with a constant, a level
//!   equation per period (t = p+1..T) with the constant as instrument, and
//!   for xtdpdsys (or xtdpd's lgmmiv()/liv()) level instruments too, LD.y;
//!   instruments that are zero in every panel are dropped;
//! - one step: W = (sum Z_i' H Z_i)^-1, H from the differencing (block
//!   diagonal when the level equations have instruments of their own), the
//!   Moore-Penrose inverse when it is rank deficient; sigma^2 = SSR of the
//!   differenced equations / (2(N - k)); two step: W = S^-1 from the
//!   one-step residuals; vce(robust): the sandwich, and Windmeijer's
//!   correction after two steps;
//! - Sargan's and Arellano and Bond's tests from the same fit.
//!
//! Weights, `if` with operators, factor variables, abbreviated variable
//! names and the display options are not translated.

use super::returned::Returned;
use super::*;

pub(super) const XTDPD_HELPER: &str = r##"stata_xtdpd_pad <- function(s, w) paste0(s, strrep(" ", pmax(0, w - nchar(s))))
stata_xtdpd_lpad <- function(s, w) paste0(strrep(" ", pmax(0, w - nchar(s))), s)
stata_xtdpd_short <- function(s, n = 12) if (nchar(s) > n) paste0(substr(s, 1, n - 2), "~", substr(s, nchar(s), nchar(s))) else s
stata_xtdpd_ginv <- function(a) {
  e <- eigen((a + t(a)) / 2, symmetric = TRUE)
  tol <- max(abs(e$values)) * length(e$values) * .Machine$double.eps
  if (all(abs(e$values) > tol)) return(solve(a))
  keep <- abs(e$values) > tol
  v <- e$vectors[, keep, drop = FALSE]
  v %*% (t(v) / e$values[keep])
}
# Arellano and Bond's m statistics, orders 1 to `orders` (StatCraft's decoding, probes 326, 420, 427)
stata_xtdpd_arm <- function(f, orders) {
  k <- nrow(f$V); l <- ncol(f$infl)
  out <- matrix(0, 0, 2)
  for (j in seq_len(orders)) {
    num <- 0; sq <- 0; wx <- numeric(k); zew <- numeric(l); whw <- 0; zhw <- numeric(l)
    for (u in f$units) {
      ed <- u$ed; n <- length(ed)
      # the residual j periods before each one (by period: panels may have gaps)
      at <- match(u$pd - j, u$pd)
      w <- ifelse(is.na(at), 0, ed[at])
      si <- sum(w * ed)
      if (n > 0) wx <- wx + colSums(w * u$xd)
      num <- num + si; sq <- sq + si^2
      zew <- zew + as.numeric(crossprod(u$z, u$eall)) * si
      if (!f$robust && !f$twostep && n > 0) {
        nb <- function(p) { k <- match(p, u$pd); ifelse(is.na(k), 0, w[k]) }
        hw <- 2 * w - nb(u$pd - 1) - nb(u$pd + 1)
        whw <- whw + sum(w * hw)
        zhw <- zhw + as.numeric(crossprod(u$z[seq_len(n), , drop = FALSE], hw))
      }
    }
    t2 <- sum(wx * (f$infl %*% zew)); t3 <- sum(wx * (f$V %*% wx))
    if (!f$robust && !f$twostep) { sq <- f$sig2 * whw; t2 <- f$sig2 * sum(wx * (f$infl %*% zhw)) }
    z <- num / sqrt(sq - 2 * t2 + t3)
    out <- rbind(out, c(z, 2 * stats::pnorm(-abs(z))))
  }
  out
}
stata_xtdpd_setarm <- function(e, arm) {
  e <- e[!(names(e) == "artests" | grepl("^arm[0-9]+$", names(e)))]
  am <- list()
  for (i in seq_len(nrow(arm))) am[[paste0("arm", i)]] <- arm[i, 1]
  add <- c(list(artests = nrow(arm)), am)
  at <- match("sargan", names(e))
  if (is.na(at)) at <- match("cmdline", names(e))
  out <- c(e[seq_len(at - 1)], add, e[at:length(e)])
  attr(out, "stata_sample") <- TRUE
  out
}
# xtabond, xtdpdsys and xtdpd as Stata 19 fits them (StatCraft's decoding, plan/stata/panel.md)
stata_xtdpd <- function(d, sel, cmd, ivar, tvar, delta, ser, lags, nreg, srcs, lsrcs, dstd, lstd, constant,
                        twostep, robust, level, artests, names, rows, tail, notes, sigvars, depvar,
                        cmdline, show = TRUE) {
  title <- switch(cmd, xtabond = paste0("Arellano", intToUtf8(8211L), "Bond dynamic panel-data estimation"), xtdpdsys = "System dynamic panel-data estimation",
                  "Dynamic panel-data estimation")
  vars <- unique(c(ivar, tvar, vapply(ser, function(s) s$v, "")))
  for (v in vars) if (!v %in% names(d)) {
    if (sum(startsWith(names(d), v)) == 1) stop("RCraft does not translate the abbreviated variable name ", v, " here; write it in full", call. = FALSE)
    stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
  }
  for (v in vars) if (is.character(d[[v]])) stop("type mismatch (Stata: r(109))", call. = FALSE)
  num <- function(v) { x <- unclass(d[[v]]); attributes(x) <- NULL; as.numeric(x) }
  id <- num(ivar); tt <- num(tvar)
  ok <- !is.na(id) & !is.na(tt)
  if (!any(ok)) stop("no observations (Stata: r(2000))", call. = FALSE)
  tmin <- min(tt[ok]); per <- round((tt - tmin) / delta) + 1
  tmax <- max(per[ok])
  if (tmax > 2001) stop("time variable spans too many periods", call. = FALSE)
  ids <- sort(unique(id[ok])); G <- length(ids); gi <- match(id, ids)
  at <- cbind(gi[ok], per[ok])
  raw <- function(v) { m <- matrix(NA_real_, G, tmax); m[at] <- num(v)[ok]; m }
  lagm <- function(m, k) {
    if (k == 0) return(m)
    r <- matrix(NA_real_, nrow(m), ncol(m))
    if (k < ncol(m)) r[, (k + 1):ncol(m)] <- m[, 1:(ncol(m) - k)]
    r
  }
  tsm <- function(m, s, n) { m <- lagm(m, s); for (i in seq_len(n)) m <- m - lagm(m, 1); m }
  P <- lapply(ser, function(s) tsm(raw(s$v), s$s, s$n))
  S <- matrix(FALSE, G, tmax); S[at] <- sel[ok]
  # the instrument columns
  ncols <- 0; dcol <- list()
  for (si in seq_along(srcs)) {
    s <- srcs[[si]]; st <- integer(tmax); nl <- integer(tmax)
    for (t in seq_len(tmax)) if (t >= lags + 2) {
      n <- max(0, min(s$hi, t - 1) + 1 - s$lo); st[t] <- ncols; nl[t] <- n; ncols <- ncols + n
    }
    dcol[[si]] <- list(st = st, nl = nl)
  }
  dstd0 <- ncols; ncols <- ncols + length(dstd)
  ccol <- ncols; if (constant) ncols <- ncols + 1
  lstd0 <- ncols; ncols <- ncols + length(lstd)
  lcol <- list()
  for (li in seq_along(lsrcs)) {
    v <- integer(tmax)
    for (t in seq_len(tmax)) if (t >= lags + 1) { v[t] <- ncols; ncols <- ncols + 1 }
    lcol[[li]] <- v
  }
  has_level <- constant || length(lsrcs) > 0 || length(lstd) > 0
  blockdiag <- length(lsrcs) > 0 || length(lstd) > 0
  k <- lags + nreg + as.integer(constant)
  Y <- numeric(); X <- matrix(0, 0, k); Z <- matrix(0, 0, ncols); U <- integer(); D <- logical(); Hs <- list(); kept <- list()
  nu <- 0
  for (g in seq_len(G)) {
    pr <- function(s, t) t >= 1 && t <= tmax && !is.na(P[[s]][g, t])
    va <- function(s, t) if (pr(s, t)) P[[s]][g, t] else 0
    dp <- integer(); lp <- integer()
    if (tmax >= lags + 2) for (t in (lags + 2):tmax) {
      if (S[g, t] && all(vapply(0:(lags + 1), function(j) pr(1, t - j), TRUE)) &&
          all(vapply(seq_len(nreg), function(s) pr(s + 1, t) && pr(s + 1, t - 1), TRUE)) &&
          all(vapply(dstd, function(q) pr(q$var, t) && (!q$diff || pr(q$var, t - 1)), TRUE))) dp <- c(dp, t)
    }
    if (has_level && tmax >= lags + 1) for (t in (lags + 1):tmax) {
      if (S[g, t] && all(vapply(lstd, function(q) pr(q, t), TRUE)) && all(vapply(0:lags, function(j) pr(1, t - j), TRUE)) &&
          all(vapply(seq_len(nreg), function(s) pr(s + 1, t), TRUE))) lp <- c(lp, t)
    }
    if (length(dp) + length(lp) == 0 || (length(dp) == 0 && !blockdiag)) next
    nu <- nu + 1
    for (t in dp) {
      Y <- c(Y, va(1, t) - va(1, t - 1))
      X <- rbind(X, c(vapply(seq_len(lags), function(j) va(1, t - j) - va(1, t - j - 1), 0),
                      vapply(seq_len(nreg), function(s) va(s + 1, t) - va(s + 1, t - 1), 0), if (constant) 0))
      z <- numeric(ncols)
      for (si in seq_along(srcs)) {
        s <- srcs[[si]]; st <- dcol[[si]]$st[t]; nl <- dcol[[si]]$nl[t]
        for (m in seq_len(nl) - 1) z[st + m + 1] <- va(s$var, t - s$lo - (nl - 1) + m)
      }
      for (di in seq_along(dstd)) { q <- dstd[[di]]; z[dstd0 + di] <- if (q$diff) va(q$var, t) - va(q$var, t - 1) else va(q$var, t) }
      Z <- rbind(Z, z)
    }
    for (t in lp) {
      Y <- c(Y, va(1, t))
      X <- rbind(X, c(vapply(seq_len(lags), function(j) va(1, t - j), 0), vapply(seq_len(nreg), function(s) va(s + 1, t), 0), if (constant) 1))
      z <- numeric(ncols)
      if (constant) z[ccol + 1] <- 1
      for (li in seq_along(lstd)) z[lstd0 + li] <- va(lstd[[li]], t)
      for (li in seq_along(lsrcs)) {
        q <- lsrcs[[li]]
        z[lcol[[li]][t] + 1] <- if (pr(q$var, t - q$j) && pr(q$var, t - q$j - 1)) va(q$var, t - q$j) - va(q$var, t - q$j - 1) else 0
      }
      Z <- rbind(Z, z)
    }
    U <- c(U, rep(nu, length(dp) + length(lp))); D <- c(D, rep(TRUE, length(dp)), rep(FALSE, length(lp)))
    ne <- tmax - lags
    A <- matrix(0, length(dp) + length(lp), ne)
    for (i in seq_along(dp)) { A[i, dp[i] - lags] <- 1; A[i, dp[i] - lags - 1] <- -1 }
    for (i in seq_along(lp)) A[length(dp) + i, lp[i] - lags] <- 1
    H <- tcrossprod(A)
    if (blockdiag && length(dp) > 0 && length(lp) > 0) {
      H[seq_along(dp), length(dp) + seq_along(lp)] <- 0; H[length(dp) + seq_along(lp), seq_along(dp)] <- 0
    }
    Hs[[nu]] <- H
    kept[[nu]] <- list(g = g, periods = c(dp, lp), nd = length(dp), nl = length(lp))
  }
  if (nu == 0) stop("no observations (Stata: r(2000))", call. = FALSE)
  # instruments that are zero everywhere are dropped
  Z <- Z[, colSums(Z != 0) > 0, drop = FALSE]
  L <- ncol(Z)
  rows_of <- split(seq_along(U), U)
  ZX <- crossprod(Z, X); Zy <- crossprod(Z, Y)
  ZHZ <- matrix(0, L, L)
  for (i in seq_len(nu)) { r <- rows_of[[i]]; Zi <- Z[r, , drop = FALSE]; ZHZ <- ZHZ + crossprod(Zi, Hs[[i]] %*% Zi) }
  ev <- eigen(crossprod(Z), symmetric = TRUE, only.values = TRUE)$values
  zrank <- sum(abs(ev) > max(abs(ev)) * length(ev) * .Machine$double.eps)
  gmm <- function(W) {
    XZ <- t(ZX); mi <- solve(XZ %*% W %*% ZX); infl <- mi %*% XZ %*% W
    list(b = as.numeric(infl %*% Zy), mi = mi, infl = infl)
  }
  ze <- function(e) t(rowsum(Z * e, U, reorder = TRUE))
  W1 <- stata_xtdpd_ginv(ZHZ)
  f1 <- gmm(W1)
  r1 <- as.numeric(Y - X %*% f1$b)
  ZE1 <- ze(r1)
  Sm <- tcrossprod(ZE1)
  nd_total <- sum(D)
  XZ <- t(ZX)
  v1r <- f1$mi %*% (XZ %*% W1 %*% Sm %*% W1 %*% ZX) %*% f1$mi
  if (twostep) {
    W2 <- stata_xtdpd_ginv(Sm)
    f2 <- gmm(W2)
    res <- as.numeric(Y - X %*% f2$b)
    if (robust) {
      # Windmeijer's finite-sample correction
      wg <- W2 %*% rowSums(ze(res))
      Dm <- matrix(0, k, k)
      for (p in seq_len(k)) {
        Ap <- t(rowsum(Z * X[, p], U, reorder = TRUE))
        ds <- Ap %*% t(ZE1) + ZE1 %*% t(Ap)
        Dm[, p] <- f2$infl %*% ds %*% wg
      }
      dv <- Dm %*% f2$mi
      V <- f2$mi + dv + t(dv) + Dm %*% v1r %*% t(Dm)
    } else V <- f2$mi
    b <- f2$b; infl <- f2$infl
  } else {
    ssd1 <- sum(r1[D]^2)
    V <- if (robust) v1r else f1$mi * (ssd1 / (2 * (nd_total - k)))
    b <- f1$b; infl <- f1$infl; res <- r1
  }
  rss <- sum(res[D]^2); s2 <- rss / (2 * (nd_total - k))
  sargan <- NULL
  if (!robust) {
    Wsg <- if (twostep) tryCatch(solve(Sm), error = function(e) NULL) else W1
    if (!is.null(Wsg)) { g <- rowSums(ze(res)); sargan <- sum(g * (Wsg %*% g)) / (if (twostep) 1 else s2) }
  }
  ks <- k - as.integer(constant)
  chi2 <- if (ks > 0) tryCatch(sum(b[1:ks] * solve(V[1:ks, 1:ks, drop = FALSE], b[1:ks])), error = function(e) NaN) else NaN
  pval <- stats::pchisq(chi2, ks, lower.tail = FALSE)
  count_levels <- cmd != "xtabond" && has_level
  cnt <- vapply(kept, function(u) if (count_levels) u$nl else u$nd, 0)
  head <- function(ttl, levels) {
    cn <- vapply(kept, function(u) if (levels) u$nl else u$nd, 0)
    line <- function(l, lab, v) paste0(stata_xtdpd_pad(l, 48), stata_xtdpd_pad(lab, 18), "= ", stata_xtdpd_lpad(v, 10))
    c("", line(ttl, "Number of obs", format(sum(cn))), line(paste0("Group variable: ", ivar), "Number of groups", format(nu)),
      paste0("Time variable: ", tvar), paste0(strrep(" ", 48), "Obs per group:"),
      paste0(strrep(" ", 62), "min = ", stata_xtdpd_lpad(format(min(cn)), 10)),
      paste0(strrep(" ", 62), "avg = ", stata_xtdpd_lpad(stata_fmt(sum(cn) / nu, "%9.0g", trim = TRUE), 10)),
      paste0(strrep(" ", 62), "max = ", stata_xtdpd_lpad(format(max(cn)), 10)), "")
  }
  line <- function(l, lab, v) paste0(stata_xtdpd_pad(l, 48), stata_xtdpd_pad(lab, 18), "= ", stata_xtdpd_lpad(v, 10))
  out <- c(head(title, count_levels),
           line(sprintf("Number of instruments = %6d", zrank), sprintf("Wald chi2(%d)", ks), sprintf("%.2f", chi2)),
           line("", "Prob > chi2", sprintf("%.4f", pval)),
           if (twostep) "Two-step results" else "One-step results")
  if (robust && cmd != "xtdpdsys") out <- c(out, sprintf("%78s", paste0("(Std. err. adjusted for clustering on ", ivar, ")")))
  out <- c(out, strrep("-", 78))
  if (robust) out <- c(out, if (twostep) "             |              WC-robust" else "             |               Robust")
  out <- c(out, sprintf("%12s | Coefficient  %s      z    P>|z|     [%s%% conf. interval]", stata_xtdpd_short(depvar),
                        if (robust) "std. err." else "Std. err.", stata_fmt(level, "%9.0g", trim = TRUE)),
           paste0("-------------+", strrep("-", 64)))
  se <- sqrt(diag(V)); crit <- stats::qnorm((100 + level) / 200)
  g9 <- function(x) stata_fmt(x, "%9.0g", trim = TRUE)
  body <- character()
  for (r in rows) {
    p <- strsplit(r, "\t", fixed = TRUE)[[1]]
    body <- c(body, switch(p[1], G = "             |", H = sprintf("%12s |", stata_xtdpd_short(p[2])), {
      i <- as.integer(p[3]); z <- b[i] / se[i]
      sprintf("%12s |%11s%11s%9.2f%8.3f%13s%12s", stata_xtdpd_short(p[2]), g9(b[i]), g9(se[i]), z, 2 * stats::pnorm(-abs(z)),
              g9(b[i] - crit * se[i]), g9(b[i] + crit * se[i]))
    }))
  }
  tl <- c(if (twostep && !robust) c("Warning: gmm two-step standard errors are biased; robust standard ", "         errors are recommended."), tail)
  report <- c(out, body, strrep("-", 78), tl)
  if (show) { for (n in notes) cat(n, "\n", sep = ""); cat(report, sep = "\n") }
  # what estat needs
  units <- lapply(seq_len(nu), function(i) {
    r <- rows_of[[i]]; nd <- kept[[i]]$nd
    list(ed = res[r][seq_len(nd)], pd = kept[[i]]$periods[seq_len(nd)], eall = res[r], xd = X[r, , drop = FALSE][seq_len(nd), , drop = FALSE], z = Z[r, , drop = FALSE])
  })
  fit <- list(units = units, infl = infl, V = V, twostep = twostep, robust = robust, system = cmd == "xtdpdsys", ivar = ivar,
              zrank = zrank, k = k, has_level = has_level, artests = artests, sig2 = s2, sargan = sargan, report = report,
              refit_head = head("Dynamic panel-data estimation", has_level), nhead = length(head(title, count_levels)), cmd = cmd, delta = delta)
  # a one-step fit with vce(gmm) and level equations has no AR tests (xtabond, xtdpdsys; probes 420, 430);
  # without level equations they assume homoskedastic errors (xtdpd, probe 427)
  arm <- if (robust || twostep || !has_level) stata_xtdpd_arm(fit, artests) else matrix(0, 0, 2)
  stata_xtdpd_fit <<- fit
  Vs <- (V + t(V)) / 2
  e <- list(rank = k, sig2 = s2, rss = rss, N = sum(cnt), N_g = nu, g_min = min(cnt), g_max = max(cnt), g_avg = sum(cnt) / nu,
            t_min = 1, t_max = tmax, chi2 = chi2, df_m = ks, zrank = zrank)
  if (!is.null(sargan)) e$sargan <- sargan
  m <- list(cmdline = cmdline, cmd = cmd, estat_cmd = "xtdpd_estat", marginsok = "XB default", predict = "xtdpd_p", depvar = depvar,
            transform = "difference", tvar = tvar, ivar = ivar, datasignaturevars = paste(sigvars, collapse = " "))
  if (cmd != "xtdpd") m$system <- "system"
  m$vce <- if (robust) "robust" else "gmm"
  if (robust) m$vcetype <- if (twostep) "WC-robust" else "Robust"
  if (twostep) m$twostep <- "twostep"
  m$properties <- "b V"
  e <- c(e, m, list(b = matrix(b, 1, k, dimnames = list("y1", names)), V = matrix(Vs, k, k, dimnames = list(names, names))))
  e <- stata_xtdpd_setarm(e, arm)
  stata_e <<- e
  smp <- logical(nrow(d))
  for (u in kept) smp[ok & gi == u$g & per %in% u$periods] <- TRUE
  stata_esample <<- smp
  invisible(NULL)
}
stata_xtdpd_replay <- function(cmd) {
  if (!exists("stata_xtdpd_fit") || !identical(stata_e$cmd, cmd)) stop("last estimates not found (Stata: r(301))", call. = FALSE)
  cat(stata_xtdpd_fit$report, sep = "\n")
}
stata_xtdpd_sargan <- function() {
  f <- stata_xtdpd_fit
  cat("Sargan test of overidentifying restrictions\nH0: Overidentifying restrictions are valid\n")
  df <- max(0, f$zrank - f$k)
  if (is.null(f$sargan)) {
    cat("        cannot calculate Sargan test with vce(robust)\n\n")
    cat(sprintf("        %-13s= %9s\n", paste0("chi2(", df, ")"), "."), sprintf("        %-13s= %9s\n", "Prob > chi2", "."), sep = "")
    return(invisible(NULL))
  }
  cat("\n", sprintf("        %-13s= %9s\n", paste0("chi2(", df, ")"), stata_fmt(f$sargan, "%9.0g", trim = TRUE)),
      sprintf("        %-13s= %9.4f\n", "Prob > chi2", stats::pchisq(f$sargan, df, lower.tail = FALSE)), sep = "")
  invisible(NULL)
}
stata_xtdpd_abond <- function(orders = NULL) {
  f <- stata_xtdpd_fit
  if (!f$robust && !f$twostep && f$has_level) stop("artests not computed for one-step system estimator with vce(gmm) (Stata: r(198))", call. = FALSE)
  if (is.null(orders)) orders <- f$artests
  refit <- orders > f$artests
  if (refit) {
    noted <- any(grepl("adjusted for clustering", f$report, fixed = TRUE))
    for (l in c(f$refit_head, f$report[-seq_len(f$nhead)])) {
      cat(l, "\n", sep = "")
      if (f$robust && !noted && endsWith(l, "results")) cat(sprintf("%78s\n", paste0("(Std. err. adjusted for clustering on ", f$ivar, ")")))
    }
  }
  arm <- stata_xtdpd_arm(f, orders)
  cat("\nArellano", intToUtf8(8211L), "Bond test for zero autocorrelation in first-differenced errors\nH0: No autocorrelation \n\nOrder         z   Prob > z\n",
      strrep("-", 26), "\n", sep = "")
  for (i in seq_len(nrow(arm))) cat(sprintf("%5d   %s%.4f\n", i, stata_xtdpd_pad(stata_fmt(arm[i, 1], "%7.0g", trim = TRUE), 12), arm[i, 2]))
  cat(strrep("-", 26), "\n", sep = "")
  if (f$system) stata_r <<- list(arm = matrix(cbind(seq_len(nrow(arm)), arm), nrow(arm), 3,
                                              dimnames = list(paste0("r", seq_len(nrow(arm))), c("order", "arstat", "p-values"))))
  # the refit for more orders than the fit's leaves e() as it was (xtdpd.do in RCraft's corpus)
  invisible(NULL)
}
# predict after xtabond, xtdpdsys and xtdpd: xb, e and their first differences, for every row
# whose terms all exist (probe 429)
stata_xtdpd_predict <- function(d, what = "xb", difference = FALSE) {
  e <- stata_e; b <- e$b; nm <- colnames(b)
  num <- function(v) { x <- unclass(d[[v]]); attributes(x) <- NULL; as.numeric(x) }
  id <- num(e$ivar); tt <- num(e$tvar)
  ok <- !is.na(id) & !is.na(tt)
  out <- rep(NA_real_, nrow(d))
  if (!any(ok)) return(out)
  delta <- stata_xtdpd_fit$delta
  tmin <- min(tt[ok]); per <- round((tt - tmin) / delta) + 1; tmax <- max(per[ok])
  ids <- sort(unique(id[ok])); gi <- match(id, ids); at <- cbind(gi[ok], per[ok])
  lagm <- function(m, k) {
    if (k == 0) return(m)
    r <- matrix(NA_real_, nrow(m), ncol(m))
    if (k < ncol(m)) r[, (k + 1):ncol(m)] <- m[, 1:(ncol(m) - k)]
    r
  }
  term <- function(name) {
    s <- 0; n <- 0; base <- name; op <- substring(sub("[.].*$", "", name), 2)
    if (grepl("^L[0-9]*[.]", name)) { s <- if (nzchar(op)) as.integer(op) else 1L; base <- sub("^[^.]*[.]", "", name) }
    else if (grepl("^D[0-9]*[.]", name)) { n <- if (nzchar(op)) as.integer(op) else 1L; base <- sub("^[^.]*[.]", "", name) }
    m <- matrix(NA_real_, length(ids), tmax); m[at] <- num(base)[ok]
    m <- lagm(m, s); for (i in seq_len(n)) m <- m - lagm(m, 1)
    m
  }
  f <- matrix(0, length(ids), tmax)
  for (j in seq_along(nm)) f <- f + b[1, j] * (if (nm[j] == "_cons") 1 else term(nm[j]))
  if (what == "e") f <- term(e$depvar) - f
  if (difference) f <- f - lagm(f, 1)
  out[ok] <- f[at]
  out
}
"##;

/// The commands.
pub(super) const XTDPD_CMDS: &[&str] = &["xtabond", "xtdpdsys", "xtdpd"];

/// The e() macros they leave, read back as text.
const XTDPD_TEXTS: &[&str] = &[
    "cmdline",
    "cmd",
    "estat_cmd",
    "marginsok",
    "predict",
    "depvar",
    "transform",
    "tvar",
    "ivar",
    "datasignaturevars",
    "system",
    "vce",
    "vcetype",
    "twostep",
    "properties",
];

/// Marks results after which estat abond is not translated (a one-step fit
/// with vce(gmm) and no level equation).
const NO_ABOND: &str = "=no_abond";

/// Display options Stata takes and that are not translated here.
const DISPLAY: &[&str] = &[
    "noci",
    "nopvalues",
    "noomitted",
    "vsquish",
    "noemptycells",
    "baselevels",
    "allbaselevels",
    "nofvlabel",
    "fvwrap",
    "fvwrapon",
    "cformat",
    "pformat",
    "sformat",
    "nolstretch",
    "coeflegend",
];

/// Most lags of a series in the model or its instruments that are translated.
const MAX_LAG: usize = 100;

/// A Stata error the line raises, or a line that is not translated.
enum Stop {
    Stata(String, u32),
    No(Untranslated),
}

impl From<Untranslated> for Stop {
    fn from(u: Untranslated) -> Self {
        Stop::No(u)
    }
}

fn stata(msg: impl Into<String>, rc: u32) -> Stop {
    Stop::Stata(msg.into(), rc)
}

/// Where a regressor's instruments come from.
#[derive(Clone, Copy, PartialEq)]
enum Role {
    Pre,
    Endog,
    Exog,
}

/// A variable lagged `.1` times and differenced `.2` times.
type Series = (String, usize, usize);

/// `x` lagged `a` times as a prefix: ``, `L.`, `L2.`.
fn lag_prefix(a: usize) -> String {
    match a {
        0 => String::new(),
        1 => "L.".into(),
        n => format!("L{n}."),
    }
}

/// The first difference of a series, as Stata names it: `D.x`, `LD.x`,
/// `L2D.x`, `D2.w`.
fn diff_label(base: &str, shift: usize, dn: usize) -> String {
    let l = match shift {
        0 => String::new(),
        1 => "L".into(),
        n => format!("L{n}"),
    };
    let d = if dn == 0 {
        "D".to_string()
    } else {
        format!("D{}", dn + 1)
    };
    format!("{l}{d}.{base}")
}

/// How Stata names a series: `x`, `L.x`, `L2.x`, `D.x`, `D2.x`.
fn series_name(s: &Series) -> String {
    match (s.1, s.2) {
        (a, 0) => format!("{}{}", lag_prefix(a), s.0),
        (_, 1) => format!("D.{}", s.0),
        (_, n) => format!("D{n}.{}", s.0),
    }
}

/// `x`, `L.x`, `L2.x`, `D.x`, `D2.x` as a series; `None` for any other form.
fn parse_ts(token: &str) -> Option<Series> {
    let Some((op, base)) = token.split_once('.') else {
        return is_name(token).then(|| (token.to_string(), 0, 0));
    };
    if !is_name(base) {
        return None;
    }
    let num = |s: &str| -> Option<usize> {
        if s.is_empty() {
            Some(1)
        } else {
            s.parse::<usize>()
                .ok()
                .filter(|n| (1..=MAX_LAG).contains(n))
        }
    };
    if let Some(n) = op.strip_prefix('L').and_then(num) {
        return Some((base.to_string(), n, 0));
    }
    op.strip_prefix('D')
        .and_then(num)
        .map(|n| (base.to_string(), 0, n))
}

fn series_of(token: &str) -> Result<Series, Stop> {
    parse_ts(token).ok_or_else(|| {
        Stop::No(no(format!(
            "{token}: only plain names and the L#. and D#. operators are translated in the dynamic panel-data commands"
        )))
    })
}

/// A variable of pre() or endogenous() with its lag structure: its lags in
/// the model and the most instrument lags (`None` for all).
struct Lagged {
    name: String,
    a: usize,
    b: Option<usize>,
}

/// `varlist [, lagstruct(#, #)]`.
fn name_list(text: &str, structure: bool) -> Result<Vec<Lagged>, Stop> {
    let (names, rest) = match text.split_once(',') {
        Some((n, r)) => (n, Some(r.trim())),
        None => (text, None),
    };
    if names.contains('(') || (rest.is_some() && !structure) {
        return Err(stata("invalid syntax", 198));
    }
    let (mut a, mut b) = (0, None);
    if let Some(r) = rest {
        let bad = || stata(format!("{r} invalid\ninvalid syntax"), 198);
        let inner = r
            .strip_prefix("lag")
            .and_then(|x| x.split_once('('))
            .and_then(|(_, x)| x.strip_suffix(')'))
            .ok_or_else(bad)?;
        let (x, y) = inner.split_once(',').ok_or_else(bad)?;
        a = x.trim().parse::<usize>().map_err(|_| bad())?;
        b = match y.trim() {
            "." => None,
            v => match v.parse::<usize>() {
                Ok(n) if n >= 1 => Some(n),
                _ => return Err(bad()),
            },
        };
        if a > MAX_LAG || b.is_some_and(|n| n > MAX_LAG) {
            return Err(Stop::No(no(format!(
                "{r}: so many lags are not translated"
            ))));
        }
    }
    names
        .split_whitespace()
        .map(|n| {
            if is_name(n) {
                Ok(Lagged {
                    name: n.to_string(),
                    a,
                    b,
                })
            } else {
                Err(Stop::No(no(format!(
                    "{n}: only plain variable names are translated in pre(), endogenous() and inst()"
                ))))
            }
        })
        .collect()
}

/// `lagrange(# #)` or `lagrange(# .)`: the lowest and highest lag.
fn lag_range(tail: &str, option: &str, raw: &str) -> Result<(usize, Option<usize>), Stop> {
    let bad = || stata(format!("{option}({raw}) invalid\ninvalid syntax"), 198);
    let inner = tail
        .strip_prefix("lag")
        .and_then(|x| x.split_once('('))
        .and_then(|(_, x)| x.strip_suffix(')'))
        .ok_or_else(bad)?;
    let mut it = inner.split_whitespace();
    let lo = it
        .next()
        .and_then(|x| x.parse::<usize>().ok())
        .ok_or_else(bad)?;
    let hi = match it.next().ok_or_else(bad)? {
        "." => None,
        v => Some(v.parse::<usize>().map_err(|_| bad())?),
    };
    if hi.is_some_and(|h| h < lo) {
        return Err(stata(
            format!("last lag cannot be less than first lag\n{option}({raw}) invalid"),
            198,
        ));
    }
    if lo > MAX_LAG || hi.is_some_and(|h| h > MAX_LAG) {
        return Err(Stop::No(no(format!(
            "{option}({raw}): so many lags are not translated"
        ))));
    }
    Ok((lo, hi))
}

/// A variable list with an optional `, option(...)` tail.
fn iv_list(text: &str) -> (Vec<String>, Option<String>) {
    let (n, t) = match text.split_once(',') {
        Some((n, t)) => (n, Some(t.trim().to_string())),
        None => (text, None),
    };
    (n.split_whitespace().map(str::to_string).collect(), t)
}

/// The position of a series among the model's (1 is the outcome), added
/// when new.
fn series_idx(series: &mut Vec<Series>, s: Series) -> usize {
    if let Some(i) = series.iter().position(|x| *x == s) {
        return i + 1;
    }
    series.push(s);
    series.len()
}

/// `maxldep(#)`, `maxlags(#)`: a positive whole number.
fn limit(opts: &Opts, name: &str, min: usize) -> Result<Option<usize>, Stop> {
    match opts.arg(name, min) {
        None => Ok(None),
        Some(a) => match a.trim().parse::<usize>() {
            Ok(n) if n >= 1 => Ok(Some(n.min(MAX_LAG))),
            _ => Err(stata(
                format!("{name}() invalid -- invalid number, outside of allowed range"),
                125,
            )),
        },
    }
}

fn r_list(items: &[String]) -> String {
    format!("list({})", items.join(", "))
}

fn r_chars(items: &[String]) -> String {
    if items.is_empty() {
        "character()".into()
    } else {
        format!(
            "c({})",
            items
                .iter()
                .map(|s| r_text(s))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

impl Session {
    /// The command whose results are active, when it is one of these.
    pub(super) fn xtdpd_active(&self) -> Option<String> {
        match &self.e_ret {
            Returned::Program { cmd, .. } if XTDPD_CMDS.contains(&cmd.as_str()) => {
                Some(cmd.clone())
            }
            _ => None,
        }
    }

    fn xtdpd_fail(&mut self, out: &mut Out, msg: &str, rc: u32) -> Result<(), Untranslated> {
        out.line(format!(
            "stop({}, call. = FALSE)",
            r_text(&format!("{msg} (Stata: r({rc}))"))
        ));
        // Stata stops before it fits: the results it had stay
        self.e_kept = true;
        Ok(())
    }

    /// `xtabond depvar [indepvars] [if] [in] [, options]`, `xtdpdsys` the
    /// same, `xtdpd L(0/#).depvar [indepvars] [if] [in], dgmmiv(...) [...]`;
    /// `xtdpdsys` and `xtdpd` alone replay the last fit.
    pub(super) fn xtdpd(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let cmd = c.name.clone();
        self.no_by(c, &cmd)?;
        if c.parts.weight.is_some() {
            return Err(no(format!("{cmd} with weights is not translated yet")));
        }
        if c.parts.main.trim().is_empty()
            && opts.0.is_empty()
            && c.parts.if_exp.is_none()
            && c.parts.in_range.is_none()
            && cmd != "xtabond"
        {
            return match (&self.e_ret, self.xtdpd_active()) {
                (_, Some(_))
                | (Returned::Nothing | Returned::Kept { .. } | Returned::Program { .. }, None) => {
                    self.helpers.insert("stata_xtdpd");
                    out.line(format!("stata_xtdpd_replay({})", r_text(&cmd)));
                    self.e_kept = true;
                    Ok(())
                }
                (Returned::Lost(why), None) => Err(no(format!(
                    "{cmd} alone replays the last fit, which is not known here: {why}"
                ))),
            };
        }
        let (call, no_ar) = match self.xtdpd_call(c, opts, &cmd) {
            Ok(x) => x,
            Err(Stop::Stata(m, rc)) => return self.xtdpd_fail(out, &m, rc),
            Err(Stop::No(u)) => return Err(u),
        };
        let d = self.data()?;
        let sel = match self.condition(c, out)? {
            Some(cond) => {
                out.package("dplyr");
                format!("local({{ s <- dplyr::transmute({d}, s = {cond})$s; !is.na(s) & s }})")
            }
            None => format!("rep(TRUE, nrow({d}))"),
        };
        for h in ["stata_xtdpd", "stata_fmt", "stata_returned"] {
            self.helpers.insert(h);
        }
        out.line(format!(
            "stata_xtdpd({d}, {sel}, {call}, cmdline = {}, show = {})",
            r_text(&super::returned::cmdline(c)),
            super::stats::r_logical(!self.quiet(c))
        ));
        self.esample_new = true;
        self.e_new = true;
        self.estimates = None;
        self.estimates_lost = Some(format!(
            "test, lincom, margins and the other post-estimation commands are not translated after {cmd} yet (estat abond, estat sargan and predict are)"
        ));
        self.e_text.clear();
        self.fit_ts = None;
        let mut texts: Vec<String> = XTDPD_TEXTS.iter().map(|s| s.to_string()).collect();
        if no_ar {
            texts.push(NO_ABOND.into());
        }
        self.e_ret = Returned::Program {
            cmd: cmd.clone(),
            texts,
        };
        out.caveat(format!(
            "{cmd}'s instruments, weights and variances as StatCraft decoded Stata 19's; Stata's e(datasignature) is not kept"
        ));
        Ok(())
    }

    /// The arguments of `stata_xtdpd()` after the data and the rows, or why not.
    #[allow(clippy::too_many_lines)]
    fn xtdpd_call(&self, c: &Command, opts: &Opts, cmd: &str) -> Result<(String, bool), Stop> {
        let dpd = cmd == "xtdpd";
        let system = cmd == "xtdpdsys";
        const KNOWN: &[(&str, usize)] = &[
            ("lags", 3),
            ("maxldep", 4),
            ("maxlags", 4),
            ("twostep", 3),
            ("vce", 3),
            ("level", 1),
            ("noconstant", 3),
            ("pre", 3),
            ("endogenous", 3),
            ("inst", 3),
            ("diffvars", 4),
            ("artests", 3),
        ];
        const DPD: &[(&str, usize)] = &[
            ("dgmmiv", 4),
            ("lgmmiv", 4),
            ("div", 3),
            ("liv", 3),
            ("twostep", 3),
            ("vce", 3),
            ("level", 1),
            ("noconstant", 6),
            ("hascons", 4),
            ("artests", 3),
            ("diffvars", 4),
        ];
        let known = if dpd { DPD } else { KNOWN };
        for o in opts.0 {
            if known.iter().any(|(n, m)| o.is(n, *m)) || (cmd == "xtabond" && o.is("robust", 1)) {
                continue;
            }
            if o.name.len() >= 3 && DISPLAY.iter().any(|d| d.starts_with(o.name.as_str())) {
                return Err(Stop::No(no(format!(
                    "{cmd} with the display option {} is not translated yet",
                    o.name
                ))));
            }
            let shown = match &o.raw {
                Some(r) => format!("{}({r})", o.name),
                None => o.name.clone(),
            };
            return Err(stata(format!("option {shown} not allowed"), 198));
        }
        let level = match opts.arg("level", 1) {
            None => 95.0,
            Some(l) => match l.trim().parse::<f64>() {
                Ok(v) if (10.0..=99.99).contains(&v) => v,
                _ => return Err(Stop::No(no(format!("{cmd}, level({l}) is not translated")))),
            },
        };
        let twostep = opts.has("twostep", 3);
        let robust = match opts.arg("vce", 3).map(|a| a.trim().to_string()) {
            Some(a) if !a.is_empty() && "robust".starts_with(a.as_str()) => true,
            Some(a) if !a.is_empty() && "gmm".starts_with(a.as_str()) => false,
            Some(a) => {
                return Err(Stop::No(no(format!(
                    "{cmd}, vce({a}) is not translated (gmm and robust are)"
                ))))
            }
            None => false,
        } || (cmd == "xtabond" && opts.has("robust", 1));
        let constant = !opts.has("noconstant", if dpd { 6 } else { 3 });
        let artests = match opts.arg("artests", 3) {
            None => 2,
            Some(a) => match a.trim().parse::<usize>() {
                Ok(n) if (1..=MAX_LAG).contains(&n) => n,
                _ => {
                    return Err(Stop::No(no(format!(
                        "{cmd}, artests({a}) is not translated"
                    ))))
                }
            },
        };
        let tokens: Vec<String> = c
            .parts
            .main
            .split_whitespace()
            .map(str::to_string)
            .collect();
        let Some((dep_token, exog_in)) = tokens.split_first() else {
            return Err(Stop::No(no(format!(
                "{cmd} without a varlist is not translated"
            ))));
        };
        // xtdpd writes the outcome with its lags: L(0/#).y, or L(a/b).y with
        // a >= 1 for the lag L^a.y on L^(a+1)..L^b.y (probe 427)
        let (outcome, lags): (Series, usize) = if dpd {
            match dep_token
                .strip_prefix("L(")
                .and_then(|x| x.split_once(")."))
            {
                Some((range, v)) => {
                    let (a, b) = range
                        .split_once('/')
                        .and_then(|(a, b)| {
                            Some((a.parse::<usize>().ok()?, b.parse::<usize>().ok()?))
                        })
                        .filter(|(a, b)| a <= b && *b <= MAX_LAG && is_name(v))
                        .ok_or_else(|| {
                            Stop::No(no(format!(
                                "xtdpd {dep_token}: only L(#/#).y is translated for the outcome"
                            )))
                        })?;
                    if a > 0 && a == b {
                        return Err(Stop::No(no(format!("xtdpd {dep_token} is not translated"))));
                    }
                    ((v.to_string(), a, 0), b - a)
                }
                None if is_name(dep_token) => ((dep_token.clone(), 0, 0), 0),
                None => {
                    return Err(Stop::No(no(format!(
                        "xtdpd {dep_token}: only y and L(#/#).y are translated for the outcome"
                    ))))
                }
            }
        } else {
            if !is_name(dep_token) {
                return Err(Stop::No(no(format!(
                    "{cmd} {dep_token}: only a plain outcome name is translated"
                ))));
            }
            let lags = match opts.arg("lags", 3) {
                None => 1,
                Some(a) => match a.trim().parse::<usize>() {
                    Ok(n) if n >= 1 => n.min(MAX_LAG),
                    _ => return Err(stata("lags of dependent variable must >=1", 198)),
                },
            };
            ((dep_token.clone(), 0, 0), lags)
        };
        let depvar = series_name(&outcome);
        let (dep_base, dep_shift) = (outcome.0.clone(), outcome.1);
        if let Some(a) = opts.arg("diffvars", 4) {
            let in_model = a.split_whitespace().all(|v| exog_in.iter().any(|m| m == v));
            // with a constant and the variables in the model Stata fits (xtdpd.do in
            // RCraft's corpus), in a way not decoded
            if constant && in_model {
                return Err(Stop::No(no(format!(
                    "{cmd} with diffvars() and a constant is not translated"
                ))));
            }
            if !in_model {
                return Err(stata(
                    "option diffvars() specified incorrectly\n    diffvars() may not be specified if \
                     the model has a constant or you specify instruments for level equations.",
                    498,
                ));
            }
        }
        let maxldep = limit(opts, "maxldep", 4)?;
        let maxlags = limit(opts, "maxlags", 4)?;
        let list = |name: &str, structure: bool| -> Result<Vec<Lagged>, Stop> {
            match opts.0.iter().find(|o| o.is(name, 3)) {
                None => Ok(Vec::new()),
                Some(o) => name_list(&o.raw.clone().unwrap_or_else(|| o.arg()), structure),
            }
        };
        let (pre, endog, inst) = if dpd {
            (Vec::new(), Vec::new(), Vec::new())
        } else {
            (
                list("pre", true)?,
                list("endogenous", true)?,
                list("inst", false)?
                    .into_iter()
                    .map(|l| l.name)
                    .collect::<Vec<_>>(),
            )
        };
        // the regressors: predetermined, endogenous, then the strictly exogenous;
        // (label, kind, series, row label under a heading, heading group)
        let mut regs: Vec<(String, Role, Series, Option<String>, usize)> = Vec::new();
        let mut lagged_src: Vec<(usize, Role, usize, Option<usize>, String)> = Vec::new();
        for (kind, vars) in [(Role::Pre, &pre), (Role::Endog, &endog)] {
            for v in vars {
                if kind == Role::Endog && pre.iter().any(|q| q.name == v.name) {
                    continue;
                }
                let group = regs.len();
                for j in 0..=v.a {
                    let sub = (v.a > 0).then(|| {
                        if j == 0 {
                            "--.".to_string()
                        } else {
                            format!("L{j}.")
                        }
                    });
                    regs.push((
                        format!("{}{}", lag_prefix(j), v.name),
                        kind,
                        (v.name.clone(), j, 0),
                        sub,
                        group,
                    ));
                }
                lagged_src.push((regs.len() - 1, kind, v.a, v.b, v.name.clone()));
            }
        }
        let mut dup_exog: Vec<String> = Vec::new();
        let mut notes: Vec<String> = Vec::new();
        for n in exog_in {
            let s = series_of(n)?;
            let label = series_name(&s);
            if regs.iter().any(|r| r.0 == label) {
                notes.push(format!("note: {n} omitted because of collinearity."));
                // a regressor already; as an exogenous variable it still
                // instruments (`Standard: D.x`, probes 420, 422)
                if !dup_exog.contains(&label) {
                    dup_exog.push(label);
                }
            } else {
                let sub = if s.1 > 0 {
                    Some(format!("L{}.", s.1))
                } else if s.2 > 0 {
                    Some(format!("D{}.", s.2))
                } else {
                    None
                };
                let group = regs.len();
                regs.push((label, Role::Exog, s, sub, group));
            }
        }
        let mut inst = inst;
        if let Some(a) = opts.arg("diffvars", 4) {
            for v in a.split_whitespace() {
                notes.push(format!(
                    "note: {v} omitted from diffvars() because of collinearity."
                ));
                inst.push(v.to_string());
            }
        }
        // the series: the outcome, the regressors, then what the instruments need
        let mut series: Vec<Series> = vec![outcome.clone()];
        series.extend(regs.iter().map(|r| r.2.clone()));
        let nreg = regs.len();
        // (series, lowest lag, highest lag or all, label)
        let mut srcs: Vec<(usize, usize, Option<usize>, String)> = Vec::new();
        // (series, lag of the difference, label)
        let mut lsrcs: Vec<(usize, usize, String)> = Vec::new();
        // (series, differenced, label)
        let mut dstd: Vec<(usize, bool, String)> = Vec::new();
        let mut lstd: Vec<(usize, String)> = Vec::new();
        let at = |series: &mut Vec<Series>, tok: &str| -> Result<usize, Stop> {
            if tok == depvar {
                return Ok(1);
            }
            let s = series_of(tok)?;
            Ok(series_idx(series, s))
        };
        if dpd {
            for o in opts.0.iter().filter(|o| o.is("dgmmiv", 4)) {
                let raw = o.raw.clone().unwrap_or_else(|| o.arg());
                let (names, tail) = iv_list(&raw);
                let (lo, hi) = match &tail {
                    Some(t) => lag_range(t, "dgmmiv", &raw)?,
                    None => (2, None),
                };
                for n in &names {
                    let v = at(&mut series, n)?;
                    srcs.push((v, lo, hi, n.clone()));
                }
            }
            if srcs.is_empty() {
                return Err(stata("dgmmiv() is required", 198));
            }
            for o in opts.0.iter().filter(|o| o.is("lgmmiv", 4)) {
                let raw = o.raw.clone().unwrap_or_else(|| o.arg());
                let (names, tail) = iv_list(&raw);
                if let Some(t) = tail {
                    return Err(stata(format!("{t} not allowed in lgmmiv()"), 198));
                }
                for n in &names {
                    let v = at(&mut series, n)?;
                    let base = series_of(n)?.0;
                    lsrcs.push((v, 1, diff_label(&base, 1, 0)));
                }
            }
            for o in opts.0.iter().filter(|o| o.is("div", 3)) {
                let raw = o.raw.clone().unwrap_or_else(|| o.arg());
                let (names, tail) = iv_list(&raw);
                let nodiff = match tail.as_deref() {
                    None => false,
                    Some(t) if t.len() >= 3 && "nodifference".starts_with(t) => true,
                    Some(t) => return Err(stata(format!("{t} not allowed in div()"), 198)),
                };
                for n in &names {
                    let v = at(&mut series, n)?;
                    let s = series_of(n)?;
                    dstd.push((
                        v,
                        !nodiff,
                        if nodiff {
                            n.clone()
                        } else {
                            diff_label(&s.0, s.1, s.2)
                        },
                    ));
                }
            }
            for o in opts.0.iter().filter(|o| o.is("liv", 3)) {
                for n in o.arg().split_whitespace() {
                    let v = at(&mut series, n)?;
                    lstd.push((v, n.to_string()));
                }
            }
        } else {
            srcs.push((1, 2, maxldep.map(|q| q + 1), depvar.clone()));
            lsrcs.push((1, 1, diff_label(&depvar, 1, 0)));
            for (pos, kind, a, b, base) in &lagged_src {
                let cap = match (b, maxlags) {
                    (Some(b), Some(m)) => Some((*b).min(m)),
                    (Some(b), None) => Some(*b),
                    (None, m) => m,
                };
                let lo = if *kind == Role::Pre { 1 } else { 2 };
                srcs.push((
                    pos + 2,
                    lo,
                    cap.map(|c| c + lo - 1),
                    format!("{}{base}", lag_prefix(*a)),
                ));
                let lab = if *kind == Role::Pre { *a } else { a + 1 };
                lsrcs.push((
                    pos + 2,
                    usize::from(*kind != Role::Pre),
                    diff_label(base, lab, 0),
                ));
            }
            if !system {
                lsrcs.clear();
            }
            for (j, r) in regs.iter().enumerate() {
                if r.1 == Role::Exog {
                    dstd.push((j + 2, true, diff_label(&r.2 .0, r.2 .1, r.2 .2)));
                }
            }
            for n in &dup_exog {
                let v = at(&mut series, n)?;
                let s = series_of(n)?;
                dstd.push((v, true, diff_label(&s.0, s.1, s.2)));
            }
            for n in &inst {
                let v = at(&mut series, n)?;
                dstd.push((v, false, n.clone()));
            }
        }
        let has_level = constant || !lsrcs.is_empty() || !lstd.is_empty();
        let panel = self.panel.clone().ok_or_else(|| {
            if self.ts_unknown {
                Stop::No(no("the data were read from a file whose xtset or tsset settings are not known here; set them again with xtset"))
            } else {
                stata("must specify panelvar and timevar; use xtset", 459)
            }
        })?;
        let time = self
            .panel_time
            .clone()
            .ok_or_else(|| stata("must specify timevar; use xtset", 459))?;
        if !is_name(&panel) || !is_name(&time) {
            return Err(Stop::No(no(format!(
                "{cmd}'s panel and time variables must be plain names"
            ))));
        }
        let delta = self.ts_delta.clone().unwrap_or_else(|| "1".into());
        // b's names and the table's rows
        let mut names: Vec<String> = (1..=lags)
            .map(|j| format!("{}{dep_base}", lag_prefix(j + dep_shift)))
            .collect();
        names.extend(regs.iter().map(|r| r.0.clone()));
        if constant {
            names.push("_cons".into());
        }
        let mut rows: Vec<String> = Vec::new();
        if lags > 0 {
            rows.push(format!("H\t{dep_base}"));
            for j in 0..lags {
                rows.push(format!("R\tL{}.\t{}", j + 1 + dep_shift, j + 1));
            }
            if nreg > 0 || constant {
                rows.push("G".into());
            }
        }
        let mut j = 0;
        while let Some(r) = regs.get(j) {
            match &r.3 {
                None => {
                    rows.push(format!("R\t{}\t{}", r.0, lags + j + 1));
                    j += 1;
                }
                Some(_) => {
                    rows.push(format!("H\t{}", r.2 .0));
                    let group = r.4;
                    while let Some(q) = regs.get(j).filter(|q| q.4 == group && q.3.is_some()) {
                        rows.push(format!(
                            "R\t{}\t{}",
                            q.3.as_deref().unwrap_or(""),
                            lags + j + 1
                        ));
                        j += 1;
                    }
                    rows.push("G".into());
                }
            }
        }
        if constant {
            rows.push(format!("R\t_cons\t{}", names.len()));
        }
        // the instruments under the table
        let gmm_diff: Vec<String> = srcs
            .iter()
            .map(|s| {
                format!(
                    "L({}/{}).{}",
                    s.1,
                    s.2.map_or(".".to_string(), |h| h.to_string()),
                    s.3
                )
            })
            .collect();
        let mut tail = vec![
            "Instruments for differenced equation".to_string(),
            format!("        GMM-type: {}", gmm_diff.join(" ")),
        ];
        if !dstd.is_empty() {
            tail.push(format!(
                "        Standard: {}",
                dstd.iter()
                    .map(|d| d.2.clone())
                    .collect::<Vec<_>>()
                    .join(" ")
            ));
        }
        if has_level {
            tail.push("Instruments for level equation".into());
        }
        if !lsrcs.is_empty() {
            tail.push(format!(
                "        GMM-type: {}",
                lsrcs
                    .iter()
                    .map(|l| l.2.clone())
                    .collect::<Vec<_>>()
                    .join(" ")
            ));
        }
        let mut std_level: Vec<String> = lstd.iter().map(|l| l.1.clone()).collect();
        if constant {
            std_level.push("_cons".into());
        }
        if !std_level.is_empty() {
            tail.push(format!("        Standard: {}", std_level.join(" ")));
        }
        // the variables of Stata's checksum: the outcome, its lags, the regressors
        let mut sigvars: Vec<String> = vec![depvar.clone()];
        sigvars.extend(names.iter().take(lags).cloned());
        for r in &regs {
            if !sigvars.contains(&r.2 .0) {
                sigvars.push(r.2 .0.clone());
            }
        }
        let ser: Vec<String> = series
            .iter()
            .map(|s| format!("list(v = {}, s = {}, n = {})", r_text(&s.0), s.1, s.2))
            .collect();
        let srcs_r: Vec<String> = srcs
            .iter()
            .map(|s| {
                format!(
                    "list(var = {}, lo = {}, hi = {})",
                    s.0,
                    s.1,
                    s.2.map_or("Inf".to_string(), |h| h.to_string())
                )
            })
            .collect();
        let lsrcs_r: Vec<String> = lsrcs
            .iter()
            .map(|l| format!("list(var = {}, j = {})", l.0, l.1))
            .collect();
        let dstd_r: Vec<String> = dstd
            .iter()
            .map(|d| {
                format!(
                    "list(var = {}, diff = {})",
                    d.0,
                    super::stats::r_logical(d.1)
                )
            })
            .collect();
        let lstd_r: Vec<String> = lstd.iter().map(|l| l.0.to_string()).collect();
        let call = format!(
            "{}, {}, {}, {delta}, {}, {lags}, {nreg}, {}, {}, {}, {}, {}, twostep = {}, robust = {}, level = {}, artests = {artests}, names = {}, rows = {}, tail = {}, notes = {}, sigvars = {}, depvar = {}",
            r_text(cmd),
            r_text(&panel),
            r_text(&time),
            r_list(&ser),
            r_list(&srcs_r),
            r_list(&lsrcs_r),
            r_list(&dstd_r),
            r_list(&lstd_r),
            super::stats::r_logical(constant),
            super::stats::r_logical(twostep),
            super::stats::r_logical(robust),
            r_number(level),
            r_chars(&names),
            r_chars(&rows),
            r_chars(&tail),
            r_chars(&notes),
            r_chars(&sigvars),
            r_text(&depvar),
        );
        // estat abond after a one-step fit with vce(gmm) and no level equation
        // is not decoded (its AR tests at the fit are, probe 427)
        Ok((call, !has_level && !twostep && !robust))
    }

    /// `estat sargan` and `estat abond [, artests(#)]` after these commands.
    pub(super) fn xtdpd_estat(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let (sub, rest) = first_word(&c.parts.main);
        if !rest.is_empty() {
            return Err(no(format!("estat {sub} with a varlist is not translated")));
        }
        self.helpers.insert("stata_xtdpd");
        self.helpers.insert("stata_fmt");
        // the r() before stays (Stata 19, probe 424), except r(arm) after xtdpdsys
        let keep = match &self.r_ret {
            Returned::Kept { names, cmd, .. } => Some((*names, cmd.clone())),
            _ => None,
        };
        let quiet = self.quiet(c);
        match sub {
            "sargan" => {
                if !opts.0.is_empty() {
                    return Err(no("estat sargan with options is not translated"));
                }
                if !quiet {
                    out.line("stata_xtdpd_sargan()");
                }
                self.r_new = keep;
            }
            "abond" => {
                if matches!(&self.e_ret, Returned::Program { texts, .. } if texts.iter().any(|t| t == NO_ABOND))
                {
                    return Err(no(
                        "estat abond after a one-step fit with vce(gmm) and no level equation is not translated",
                    ));
                }
                if let Some(o) = opts.other(&[("artests", 3)]) {
                    return Err(no(format!(
                        "estat abond with the option {o} is not translated"
                    )));
                }
                let orders = match opts.arg("artests", 3) {
                    None => "NULL".to_string(),
                    Some(a) => match a.trim().parse::<usize>() {
                        Ok(n) if (1..=MAX_LAG).contains(&n) => n.to_string(),
                        _ => {
                            return Err(no(format!("estat abond, artests({a}) is not translated")))
                        }
                    },
                };
                let call = format!("stata_xtdpd_abond({orders})");
                out.line(if quiet {
                    format!("invisible(utils::capture.output({call}))")
                } else {
                    call
                });
                if self.xtdpd_active().as_deref() == Some("xtdpdsys") {
                    self.r_new = Some((&["arm"], "estat".into()));
                } else {
                    self.r_new = keep;
                }
            }
            _ => {
                return Err(no(format!(
                    "estat {sub} after {} is not translated yet (abond and sargan are)",
                    self.xtdpd_active().unwrap_or_default()
                )))
            }
        }
        Ok(())
    }

    /// `predict [type] newvar [if] [in] [, xb e difference]` (probe 429).
    pub(super) fn xtdpd_predict(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        for o in opts.0 {
            if !(o.is("xb", 2) || o.is("e", 1) || o.is("difference", 3)) {
                return self.xtdpd_fail(out, &format!("option {} not allowed", o.name), 198);
            }
        }
        let want_e = opts.has("e", 1);
        if want_e && opts.has("xb", 2) {
            return self.xtdpd_fail(out, "options xb and e may not be combined", 198);
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
        let base = self.data()?;
        out.line(format!(
            "if ({} %in% names({base})) stop(\"variable {w} already defined (Stata: r(110))\", call. = FALSE)",
            r_text(w)
        ));
        let quiet = self.quiet(c);
        if !want_e && !opts.has("xb", 2) && !quiet {
            out.line("cat(\"(option xb assumed; fitted values)\\n\")");
        }
        self.helpers.insert("stata_xtdpd");
        out.line(format!(
            "stata_xtdpd_p <- stata_xtdpd_predict({base}, {}, difference = {})",
            r_text(if want_e { "e" } else { "xb" }),
            super::stats::r_logical(opts.has("difference", 3))
        ));
        let v = stored("stata_xtdpd_p", st, &mut self.helpers, out);
        let v = match self.condition(c, out)? {
            Some(cond) => format!("dplyr::if_else({cond}, {v}, NA_real_)"),
            None => v,
        };
        out.package("dplyr");
        out.line(format!(
            "{base} <- dplyr::mutate({base}, {} = structure({v}, label = {}))",
            r_name(w),
            r_text(if want_e {
                "Residuals"
            } else {
                "Linear prediction"
            })
        ));
        out.line("rm(stata_xtdpd_p)");
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
    fn the_three_commands_build_their_instruments() {
        let out = r("use d
xtset id t
xtdpdsys y x, twostep vce(robust)
estat abond, artests(3)
estat sargan
predict p, e difference
xtabond y w, pre(x, lagstruct(1, 2)) lags(2)
xtdpd L(0/1).y x w, dgmmiv(y, lagrange(2 4)) lgmmiv(y) div(x) liv(w)
xtdpd L(1/2).y x, dgmmiv(y) div(x)
xtdpdsys");
        assert!(
            out[2].contains("stata_xtdpd(")
                && out[2].contains("\"xtdpdsys\"")
                && out[2].contains("list(list(var = 1, lo = 2, hi = Inf))")
                && out[2].contains("list(list(var = 1, j = 1))")
                && out[2].contains("twostep = TRUE, robust = TRUE")
                && out[2].contains("names = c(\"L.y\", \"x\", \"_cons\")"),
            "{out:?}"
        );
        assert!(out[3].contains("stata_xtdpd_abond(3)"), "{out:?}");
        assert!(out[4].contains("stata_xtdpd_sargan()"), "{out:?}");
        assert!(
            out[5].contains("stata_xtdpd_predict(") && out[5].contains("\"e\", difference = TRUE"),
            "{out:?}"
        );
        // x and its lag are regressors, instrumented by L(1/2).L.x
        assert!(
            out[6].contains("L(1/2).L.x\"")
                && out[6]
                    .contains("names = c(\"L.y\", \"L2.y\", \"x\", \"L.x\", \"w\", \"_cons\")"),
            "{out:?}"
        );
        assert!(
            out[7].contains("L(2/4).y\"")
                && out[7].contains("\"        Standard: w _cons\"")
                && out[7].contains("list(3)"),
            "{out:?}"
        );
        // the outcome is L.y, its lag L2.y
        assert!(
            out[8].contains("list(v = \"y\", s = 1, n = 0)")
                && out[8].contains("names = c(\"L2.y\", \"x\", \"_cons\")")
                && out[8].contains("depvar = \"L.y\""),
            "{out:?}"
        );
        assert!(
            out[9].contains("stata_xtdpd_replay(\"xtdpdsys\")"),
            "{out:?}"
        );
    }

    #[test]
    fn statas_errors_are_kept_and_the_rest_refused() {
        let out = r("use d
xtset id t
xtdpdsys y x, lags(0)
xtdpdsys y x, maxldep(0)
xtdpdsys y, diffvars(x)
xtdpdsys y x, diffvars(x)
xtdpd L(0/1).y x
xtdpd L(0/1).y x, dgmmiv(y, lagrange(3 2))
xtdpdsys y x, bogus
xtdpdsys y x, coeflegend
xtdpd L(0/1).y x, dgmmiv(y) div(x) noconstant
estat abond
predict p, scores");
        assert!(
            out[2].contains("lags of dependent variable must >=1 (Stata: r(198))"),
            "{out:?}"
        );
        assert!(
            out[3].contains("outside of allowed range (Stata: r(125))"),
            "{out:?}"
        );
        assert!(out[4].contains("(Stata: r(498))"), "{out:?}");
        assert!(
            out[5].starts_with("NOT: xtdpdsys with diffvars() and a constant"),
            "{out:?}"
        );
        assert!(
            out[6].contains("dgmmiv() is required (Stata: r(198))"),
            "{out:?}"
        );
        assert!(
            out[7].contains("last lag cannot be less than first lag"),
            "{out:?}"
        );
        assert!(
            out[8].contains("option bogus not allowed (Stata: r(198))"),
            "{out:?}"
        );
        assert!(
            out[9].starts_with("NOT: xtdpdsys with the display option"),
            "{out:?}"
        );
        assert!(
            out[11].starts_with("NOT: estat abond after a one-step fit"),
            "{out:?}"
        );
        assert!(
            out[12].contains("option scores not allowed (Stata: r(198))"),
            "{out:?}"
        );
    }
}
