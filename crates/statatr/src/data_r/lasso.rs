//! `lasso linear` and what follows it (`lassoselect`, `lassoknots`,
//! `lassocoef`, `lassoinfo`, `predict`), as Stata 19 runs them.
//!
//! The rules are StatCraft's decoding of Stata's (our own project, its
//! `plan/stata/lasso.md`, probes 380-385 and 470-504), checked again here on
//! the probes' Stata 19 logs (`plan/stata/lasso.md` in this repository has
//! what was found): predictors standardised with divisor N, the outcome
//! centred, a geometric grid from lambda_max down to lambda_max times 1e-4
//! (1e-2 when p >= N), the deviance stop, BIC and cross-validation with
//! Stata's "minimum found" rule, the folds from one `runiform()` per
//! observation, the adaptive steps and the plug-in penalty. One rule is this
//! repository's own finding: Stata's solver is cyclic coordinate descent over
//! every predictor, warm started along the grid, that stops when no
//! coefficient moves by more than `tolerance()` (1e-7) relative to
//! max(|b|, 1); with it e(L1_max), e(L2_max) and the R-squared agree with
//! Stata's to about 1e-15 where the exact optimum is 1e-8 away.

use super::returned::Returned;
use super::*;

pub(super) const LASSO_HELPER: &str = r###"# the standardised problem of rows `rows` of y and the columns of X: each
# predictor to mean 0 and variance 1 (divisor N), the outcome centred
stata_lasso_design <- function(y, X, rows = seq_along(y), tol = 1e-7) {
  y <- y[rows]; X <- X[rows, , drop = FALSE]
  n <- length(y); p <- ncol(X)
  ybar <- sum(y) / n; yc <- y - ybar; yvar <- sum(yc^2) / n
  means <- colSums(X) / n
  Xc <- sweep(X, 2, means)
  sds <- sqrt(colSums(Xc^2) / n)
  sds[!is.finite(sds) | sds <= 0] <- 0
  Z <- Xc
  for (j in seq_len(p)) Z[, j] <- if (sds[j] > 0) Xc[, j] / sds[j] else 0
  list(n = n, p = p, means = means, sds = sds, ybar = ybar, yvar = yvar, z = Z, yc = yc,
       xtx = crossprod(Z) / n, xty = as.numeric(crossprod(Z, yc)) / n, tol = tol)
}
# the residual sum of squares over N at standardised coefficients b
stata_lasso_mse <- function(D, b) {
  a <- which(b != 0)
  if (!length(a)) return(max(D$yvar, 0))
  ba <- b[a]
  max(D$yvar - 2 * sum(ba * D$xty[a]) + sum(ba * (D$xtx[a, a, drop = FALSE] %*% ba)), 0)
}
stata_lasso_devratio <- function(D, b) if (D$yvar > 0) 1 - stata_lasso_mse(D, b) / D$yvar else 0
stata_lasso_unstd <- function(D, b) {
  v <- ifelse(D$sds > 0, b / ifelse(D$sds > 0, D$sds, 1), 0)
  c(v, D$ybar - sum(v * D$means))
}
# the lasso at lambda from the start b, as Stata 19 solves it: cyclic
# coordinate descent over every predictor in order, warm started, until in a
# sweep no coefficient in the variable's own units moves by more than tol
# relative to max(|b|, 1) (tolerance(), 1e-7 by default). Stata stops there,
# about 1e-8 short of the exact optimum; this rule gives its L1 and L2 norms,
# R-squared and log likelihood to about 1e-15 (probes 380, 381, 470), also
# with tolerance(1e-3)
stata_lasso_solve <- function(D, lam, w, b, tol = D$tol) {
  ok <- which(D$sds > 0 & is.finite(w))
  xtx <- D$xtx; xty <- D$xty; sds <- D$sds
  if (!length(ok)) return(b)
  for (it in 1:100000) {
    mx <- 0
    for (j in ok) {
      old <- b[j]
      r <- xty[j] - sum(xtx[j, ] * b) + old
      t <- lam * w[j]
      nb <- if (r > t) r - t else if (r < -t) r + t else 0
      if (nb != old) {
        b[j] <- nb
        ch <- abs(nb - old) / max(abs(nb), sds[j])
        if (ch > mx) mx <- ch
      }
    }
    if (mx < tol) break
  }
  b
}
stata_lasso_grid <- function(lmax, n, ratio, lmin = NULL) {
  last <- if (is.null(lmin)) lmax * ratio else lmin
  if (n <= 1) return(lmax)
  st <- log(last / lmax) / (n - 1)
  g <- lmax * exp(st * (0:(n - 1)))
  g[n] <- last
  g
}
stata_lasso_lmax <- function(D, w) {
  b <- numeric(D$p)
  if (any(D$sds > 0 & w == 0)) b <- stata_lasso_solve(D, 1e300, w, b)
  a <- which(b != 0)
  g <- D$xty - (if (length(a)) as.numeric(D$xtx[, a, drop = FALSE] %*% b[a]) else 0)
  pen <- D$sds > 0 & is.finite(w) & w > 0
  if (!any(pen)) return(0)
  max(abs(g[pen]) / w[pen])
}
# least squares on the selected predictors: the mean squared residual and the
# standardised coefficients
stata_lasso_post <- function(D, b) {
  a <- which(b != 0)
  out <- numeric(D$p)
  if (!length(a)) return(list(mse = D$yvar, b = out))
  ba <- tryCatch(solve(D$xtx[a, a, drop = FALSE], D$xty[a]), error = function(e) qr.solve(D$xtx[a, a, drop = FALSE], D$xty[a]))
  out[a] <- ba
  list(mse = stata_lasso_mse(D, out), b = out)
}
stata_lasso_rec <- function(D, id, lam, b, dfc, postsel = FALSE, cvm = NA_real_, se = NA_real_) {
  nz <- sum(b != 0)
  mse <- if (postsel) { m <- stata_lasso_post(D, b)$mse; if (nz == 0) D$yvar + D$ybar^2 else m } else stata_lasso_mse(D, b)
  n <- D$n
  ll <- -0.5 * n * (log(2 * pi) + 1 + log(mse))
  o <- stata_lasso_unstd(D, b)[seq_len(D$p)]
  list(id = id, lambda = lam, b = b, devratio = stata_lasso_devratio(D, b), nonzero = nz, cvm = cvm, se = se,
       ll = ll, bic = -2 * ll + (nz + dfc) * log(n), l1 = sum(abs(o)), l2 = sum(o^2))
}
# Stata's "minimum found": the criterion more than rel above its lowest value
# so far at need lambdas in a row (CV 0.1% three times, BIC 1% twice)
stata_lasso_finder <- function(rel, need) {
  s <- new.env(); s$min <- Inf; s$above <- 0
  function(v) {
    if (v < s$min) s$min <- v
    if (v > s$min + rel * abs(s$min)) s$above <- s$above + 1 else s$above <- 0
    s$above >= need
  }
}
stata_lasso_g8 <- function(v) sprintf("%8s", stata_fmt(v, "%9.0g", trim = TRUE))
stata_lasso_gridline <- function(k, lam, nz, w) {
  cat(sprintf("%-18slambda = %s   no. of nonzero coef. = %*d\n", paste0("Grid value ", k, ":"), stata_lasso_g8(lam), w, as.integer(nz)))
}
stata_lasso_foldsline <- function(k) {
  paste0("Folds: ", paste(vapply(seq_len(k), function(f) if (f == 1 || f %% 5 == 0 || f == k) as.character(f) else ".", ""), collapse = ""))
}
# the generator's state as c(rngstate) shows it: XAA, the 312 words in hex,
# then 0001, 0000, the position and four digits StatCraft found it does not
# reproduce (0000)
stata_lasso_rngstate <- function() {
  if (!exists("stata_rng_state")) stata_set_seed(123456789)
  s <- stata_rng_state
  # words drawn ahead and handed back are not used yet: the state is where
  # they came from, in the array before the last twist when they reach into it
  w <- s$w
  ahead <- if (is.null(s$buf)) 0L else length(s$buf[[1]])
  i <- s$i - ahead
  if (i < 0 && !is.null(s$prev)) { w <- s$prev; i <- 312L + i }
  if (i < 0) i <- 0L
  paste0("XAA", paste(sprintf("%04x%04x%04x%04x", w[[4]], w[[3]], w[[2]], w[[1]]), collapse = ""),
         sprintf("00010000%04x0000", as.integer(i)))
}
# the folds: one uniform per observation, the observations ranked by it and
# the ranks cut into K consecutive blocks, the first N mod K a row larger
stata_lasso_folds <- function(n, k) {
  u <- stata_runiform(n)
  ord <- order(u, seq_len(n))
  base <- max(n %/% k, 1); extra <- n %% k; big <- extra * (base + 1)
  rk <- seq_len(n) - 1
  f <- ifelse(rk < big, rk %/% (base + 1), extra + (rk - big) %/% base)
  fold <- integer(n)
  fold[ord] <- pmin(f, k - 1)
  fold
}
stata_lasso_cv_setup <- function(y, X, fold, k, noconstant, tol = 1e-7) {
  ds <- list(); tests <- list()
  for (f in 0:(k - 1)) {
    tr <- which(fold != f); te <- which(fold == f)
    ds[[f + 1]] <- stata_lasso_design(y, X, tr, tol = tol)
    tests[[f + 1]] <- te
  }
  s <- new.env()
  s$ds <- ds; s$tests <- tests; s$starts <- lapply(ds, function(d) numeric(d$p))
  s$y <- y; s$X <- X; s$noconstant <- noconstant
  s
}
stata_lasso_cv_eval <- function(s, lam, w) {
  k <- length(s$ds); mses <- numeric(k); sse <- 0; nt <- 0
  for (f in seq_len(k)) {
    d <- s$ds[[f]]
    b <- stata_lasso_solve(d, lam, w, s$starts[[f]])
    s$starts[[f]] <- b
    o <- stata_lasso_unstd(d, b)
    cons <- if (s$noconstant) 0 else o[d$p + 1]
    te <- s$tests[[f]]
    a <- which(o[seq_len(d$p)] != 0)
    fit <- cons + (if (length(a)) as.numeric(s$X[te, a, drop = FALSE] %*% o[a]) else 0)
    e2 <- sum((s$y[te] - fit)^2)
    sse <- sse + e2; nt <- nt + length(te)
    mses[f] <- e2 / max(length(te), 1)
  }
  m <- sum(mses) / k
  eq <- length(unique(lengths(s$tests))) == 1
  list(cvm = if (eq) m else sse / max(nt, 1), se = sqrt(sum((mses - m)^2) / max(k - 1, 1)))
}
# the plug-in penalty's rounds: residuals first from least squares on the five
# predictors most correlated with y, then from the post-lasso fit scaled by
# sqrt(N / (N - k)); the loadings or the scale from them; until the selected
# set repeats (at most 15 rounds)
stata_lasso_plugin <- function(D, y, X, base, hetero, bw) {
  p <- D$p; n <- D$n
  pen <- which(bw > 0)
  top <- utils::head(pen[order(-abs(D$xty[pen]))], 5)
  b0 <- numeric(p); b0[bw == 0] <- 1; b0[top] <- 1
  resid <- function(bs) { o <- stata_lasso_unstd(D, bs); as.numeric(y - o[p + 1] - X %*% o[seq_len(p)]) }
  e <- resid(stata_lasso_post(D, b0)$b)
  log <- list(); prev <- NULL; last <- NULL
  for (it in 1:15) {
    if (hetero) {
      lam <- base
      w <- sqrt(colSums(D$z^2 * e^2) / n) * bw
    } else {
      lam <- base * sqrt(sum(e^2) / n)
      w <- bw
    }
    w <- ifelse(bw == 0, 0, ifelse(D$sds > 0 & is.finite(w) & w > 0, w, Inf))
    # each round starts from the last one's solution: closest to Stata's (to
    # about 2e-9, probes 381, 385, 471; not its exact start)
    b <- stata_lasso_solve(D, lam, w, if (is.null(last)) numeric(p) else last$b)
    nz <- sum(b != 0)
    log[[it]] <- c(lam, nz)
    set <- b != 0
    same <- !is.null(prev) && identical(prev, set)
    last <- list(lambda = lam, b = b)
    prev <- set
    e <- resid(stata_lasso_post(D, b)$b) * sqrt(n / max(n - nz, 1))
    if (same) break
  }
  list(lambda = last$lambda, b = last$b, log = log)
}
stata_lasso_stop <- function(msg, rc = 198) stop(msg, " (Stata: r(", rc, "))", call. = FALSE)
# lasso linear: the fit, its log and table, e(), e(sample)
stata_lasso_run <- function(src, nall, y, always, others, sel = "cv", folds = 10, serule = FALSE,
                            alllambdas = FALSE, gridminok = FALSE, stopok = FALSE, strict = FALSE,
                            steps = 2, power = NULL, hetero = TRUE, cc = 1.1, gamma = NULL,
                            postsel = FALSE, grid = 100, ratio = NULL, lmin = NULL, stop = 1e-5,
                            nolog = FALSE, rseed = NULL, noconstant = FALSE, penaltywt = NULL, tolerance = 1e-7,
                            cmdline = "", show = TRUE) {
  say <- show && !nolog
  vars <- c(always, others)
  for (v in c(y, vars)) if (!v %in% names(src)) stata_lasso_stop(paste0("variable ", v, " not found"), 111)
  num <- function(v) { u <- unclass(src[[v]]); attributes(u) <- NULL; as.numeric(u) }
  yall <- num(y)
  Xall <- matrix(unlist(lapply(vars, num)), nrow = nrow(src), ncol = length(vars), dimnames = list(NULL, vars))
  used <- !is.na(yall) & stats::complete.cases(Xall)
  n <- sum(used)
  if (n == 0) stata_lasso_stop("no observations", 2000)
  if (sel %in% c("cv", "adaptive") && n < folds) stata_lasso_stop("the number of observations is less than the cross-validation folds")
  yv <- yall[used]; X <- Xall[used, , drop = FALSE]
  if (!is.null(penaltywt)) {
    if (length(penaltywt) != length(others)) stata_lasso_stop("option penaltywt() is not conformable with indepvars dimension")
    if (any(penaltywt < 0)) stata_lasso_stop("option penaltywt() contains at least one negative value")
  }
  # predictors that are constant, or equal to an earlier one up to scale, are dropped
  D0 <- stata_lasso_design(yv, X)
  kept <- integer(0); notes <- 0
  for (j in seq_along(vars)) {
    if (D0$sds[j] <= 0) {
      if (show) cat("note: ", vars[j], " omitted because it is constant.\n", sep = "")
      notes <- notes + 1; next
    }
    tw <- which(vapply(kept, function(k) abs(sum(D0$z[, j] * D0$z[, k]) / D0$n) > 1 - 1e-10, TRUE))
    if (length(tw)) {
      if (show) cat("note: ", vars[kept[tw[1]]], " omitted because of collinearity with another variable.\n", sep = "")
      notes <- notes + 1
      kept <- kept[-tw[1]]
    }
    kept <- c(kept, j)
  }
  skip <- notes > 0
  blank <- function() { if (skip) skip <<- FALSE else if (say) cat("\n") }
  isalways <- kept <= length(always)
  bw <- ifelse(isalways, 0, if (is.null(penaltywt)) 1 else penaltywt[pmax(kept - length(always), 1)])
  X <- X[, kept, drop = FALSE]; vars <- vars[kept]
  D <- stata_lasso_design(yv, X, tol = tolerance)
  p <- D$p
  if (!is.null(rseed)) stata_set_seed(rseed)
  rng <- stata_lasso_rngstate()
  if (is.null(ratio)) ratio <- if (n > p) 1e-4 else 1e-2
  fin <- bw[is.finite(bw)]
  w <- if (length(fin)) bw / (sum(fin) / length(fin)) else bw
  npen <- sum(bw > 0)
  dfc <- if (noconstant) 0 else 1
  nzw <- function(w) nchar(as.character(max(sum(is.finite(w)), 1)))
  recs <- list(); sel_i <- NA; id_cv <- NA; id_se <- NA; gmax <- 0; gmin <- 0; ptype <- "heteroskedastic"
  stepsdone <- 1; gridmin <- FALSE; stopsel <- FALSE; failed <- FALSE
  fold <- if (sel %in% c("cv", "adaptive")) stata_lasso_folds(n, folds) else NULL
  if (sel %in% c("none", "bic")) {
    lmax <- stata_lasso_lmax(D, w)
    lams <- stata_lasso_grid(lmax, grid, ratio, lmin)
    gmax <- lams[1]; gmin <- lams[length(lams)]
    if (say) { blank(); cat("Evaluating up to ", length(lams), " lambdas in grid ...\n", sep = "") }
    find <- stata_lasso_finder(0.01, 2)
    b <- numeric(p); hit <- FALSE; found <- FALSE
    for (k in seq_along(lams)) {
      b <- stata_lasso_solve(D, lams[k], w, b)
      r <- stata_lasso_rec(D, k, lams[k], b, dfc, postsel)
      if (say) {
        stata_lasso_gridline(k, lams[k], r$nonzero, nzw(w))
        if (sel == "bic") cat(sprintf("%21s = %s\n", "BIC", stata_lasso_g8(r$bic)))
      }
      prev <- if (length(recs)) recs[[length(recs)]]$devratio else NULL
      recs[[k]] <- r
      if (sel == "bic" && find(r$bic)) { found <- TRUE; break }
      if (!is.null(prev) && stop > 0 && r$devratio - prev < stop * r$devratio) { hit <- k < length(lams); break }
    }
    if (sel == "bic") {
      sel_i <- which.min(vapply(recs, function(r) r$bic, 0))
      bystop <- hit && sel_i == length(recs)
      stopsel <- bystop
      if (say) {
        if (bystop) cat("... change in deviance stopping tolerance reached ... last lambda selected\nMinimum of BIC not found; lambda selected based on stop() stopping criterion.\n")
        else if (hit) cat("... change in deviance stopping tolerance reached\n")
        else if (found) cat("... selection BIC complete ... minimum found\n")
        else cat("... selection BIC complete\n")
      }
    } else if (hit && say) cat("... change in deviance stopping tolerance reached\n")
  } else if (sel == "plugin") {
    if (say) { blank(); cat("Computing plugin lambda ...\n") }
    ptype <- if (hetero) "heteroskedastic" else "homoskedastic"
    g <- if (is.null(gamma)) 0.1 / log(max(n, max(npen, 1))) else gamma
    base <- cc / sqrt(n) * stats::qnorm(1 - g / (2 * max(npen, 1)))
    pw <- ifelse(isalways, 0, 1)
    pf <- stata_lasso_plugin(D, yv, X, base, hetero, pw)
    if (say) for (i in seq_along(pf$log)) cat(sprintf("Iteration %d:     lambda = %s   no. of nonzero coef. = %*d\n", i, stata_lasso_g8(pf$log[[i]][1]), nzw(pw), as.integer(pf$log[[i]][2])))
    recs[[1]] <- stata_lasso_rec(D, 1, pf$lambda, pf$b, dfc)
    gmax <- gmin <- pf$lambda; sel_i <- 1
  } else {
    nsteps <- if (sel == "adaptive") max(steps, 1) else 1
    stepsdone <- nsteps
    pw <- if (is.null(power)) 1 else power
    prevb <- NULL; nextid <- 1; ntotal <- 0; allrecs <- list()
    for (step in seq_len(nsteps)) {
      if (sel == "adaptive") {
        if (say) { blank(); cat("Lasso step ", step, " of ", nsteps, ":\n", sep = "") }
        if (!is.null(prevb)) {
          raw <- ifelse(isalways, 0, ifelse(abs(prevb) > 0, 1 / abs(prevb)^pw, Inf))
          f <- raw[is.finite(raw)]
          w <- raw / (if (length(f)) sum(f) / length(f) else 1)
        }
      }
      lmax <- stata_lasso_lmax(D, w)
      lams <- stata_lasso_grid(lmax, grid, ratio, lmin)
      gmax <- lams[1]; gmin <- lams[length(lams)]
      cvs <- stata_lasso_cv_setup(yv, X, fold, folds, noconstant, tolerance)
      lamuse <- lams; tolhit <- FALSE
      full <- alllambdas || (sel == "adaptive" && step > 1)
      if (full) {
        if (say) { blank(); cat("Evaluating up to ", length(lams), " lambdas in grid ...\n", sep = "") }
        b <- numeric(p); dv <- NULL; lamuse <- numeric(0)
        for (k in seq_along(lams)) {
          b <- stata_lasso_solve(D, lams[k], w, b)
          d1 <- stata_lasso_devratio(D, b)
          if (say) stata_lasso_gridline(k, lams[k], sum(b != 0), nzw(w))
          lamuse <- c(lamuse, lams[k])
          if (!is.null(dv) && stop > 0 && d1 - dv < stop * d1) { tolhit <- k < length(lams); break }
          dv <- d1
        }
        if (say && tolhit) cat("... change in deviance stopping tolerance reached\n")
      }
      base <- nextid
      if (say) { blank(); cat(folds, "-fold cross-validation with ", length(lamuse), " lambdas ...\n", sep = "") }
      srecs <- list(); b <- numeric(p); find <- stata_lasso_finder(0.001, 3); found <- FALSE; dv <- NULL
      for (k in seq_along(lamuse)) {
        lam <- lamuse[k]
        b <- stata_lasso_solve(D, lam, w, b)
        if (say && !full) stata_lasso_gridline(k, lam, sum(b != 0), nzw(w))
        pt <- stata_lasso_cv_eval(cvs, lam, w)
        if (say && !full) cat(stata_lasso_foldsline(folds), "   CVF = ", stata_lasso_g8(pt$cvm), "\n", sep = "")
        r <- stata_lasso_rec(D, base + k - 1, lam, b, dfc, cvm = pt$cvm, se = pt$se)
        srecs[[k]] <- r
        mf <- find(pt$cvm)
        if (!full) {
          if (!is.null(dv) && stop > 0 && r$devratio - dv < stop * r$devratio) {
            tolhit <- k < length(lamuse)
            if (tolhit) break
          }
          dv <- r$devratio
          if (mf) { found <- TRUE; break }
        }
      }
      if (full && say) {
        bar <- "  10"
        if (length(lamuse) >= 11) for (m in 11:length(lamuse)) bar <- paste0(bar, if (m %% 10 == 0) m else if (m %% 2 == 0) "." else "")
        for (f in seq_len(folds)) cat(sprintf("Fold %2d of %2d:%s\n", f, folds, bar))
      }
      ntotal <- ntotal + length(srecs)
      # e()'s lambda statistics are over every step's lambdas (probe 383)
      allrecs <- c(allrecs, srecs)
      nextid <- base + length(srecs)
      cvm <- vapply(srecs, function(r) r$cvm, 0)
      best <- which.min(cvm)
      atend <- best == length(srecs)
      nomin <- atend && !found
      laststep <- !(sel == "adaptive" && step > 1)
      ser <- function() { lim <- cvm[best] + srecs[[best]]$se; srecs[[which(cvm <= lim)[1]]]$id }
      gm <- FALSE; ss <- FALSE
      if (nomin) {
        bystop <- tolhit && (serule || !strict || !laststep)
        if (bystop) ss <- !serule && laststep
        else if (serule) {}
        else if (gridminok) gm <- TRUE
        else {
          recs <- srecs; id_cv <- srecs[[best]]$id; id_se <- ser()
          if (full && say) cat("... cross-validation complete\n")
          if (show) cat("lambda not selected\n", if (tolhit) "    No minimum of cross-validation function found.\n" else "    No minimum of cross-validation function found. Change in deviance stopping tolerance not reached.\n", sep = "")
          failed <- TRUE
          break
        }
      }
      if (say) {
        if (ss) cat("... cross-validation complete\nMinimum of CV function not found; lambda selected based on stop() stopping criterion.\n")
        else if (gm) cat("Minimum of CV function not found; minimum grid value is selected lambda.\n")
        else if (nomin && (gridminok || serule)) {}
        else if (found) cat("... cross-validation complete ... minimum found\n")
        else if (full) cat("... cross-validation complete\n")
      }
      gridmin <- gm; stopsel <- ss
      id_cv <- srecs[[best]]$id
      id_se <- ser()
      pick <- if (serule) id_se else id_cv
      sel_i <- which(vapply(srecs, function(r) r$id, 0) == pick)
      prevb <- srecs[[sel_i]]$b
      recs <- srecs
    }
  }
  if (sel %in% c("cv", "adaptive")) ntl <- ntotal else ntl <- length(recs)
  fit <- list(depvar = y, vars = vars, always = isalways, n = n, kind = sel, recs = recs, n_lambda = ntl,
              gmax = gmax, gmin = gmin, sel = if (failed) NA else sel_i, user = FALSE, orig_sel = NA,
              id_cv = id_cv, id_se = id_se, serule = serule, n_fold = folds, steps = stepsdone,
              power = if (sel == "adaptive") power else NULL, ptype = ptype, stop = stop, cmdline = cmdline,
              rngstate = rng, D = D, noconstant = noconstant, gridmin = gridmin, stopsel = stopsel,
              allrecs = if (sel %in% c("cv", "adaptive")) allrecs else recs)
  stata_lasso <<- fit
  stata_e <<- stata_lasso_e(fit)
  stata_esample <<- seq_len(nall) %in% src$stata_row[used]
  if (show) stata_lasso_table(fit)
  # no lambda selected: e() is posted all the same (Stata 19), then r(430)
  if (failed) { stata_fit_failed <<- FALSE; stop(" (Stata: r(430))", call. = FALSE) }
  invisible(NULL)
}
stata_lasso_names <- function(fit, b) c(fit$vars[b != 0], if (!fit$noconstant) "_cons")
# e() as Stata 19 posts it, in the order ereturn list shows (probes 380, 381, 382)
stata_lasso_e <- function(fit) {
  D <- fit$D; r <- if (is.na(fit$sel)) NULL else fit$recs[[fit$sel]]
  recs <- fit$recs
  user <- fit$user
  s <- list()
  if (user) {
    s$ID_sel <- r$id; s$lambda_sel <- r$lambda; s$alpha_sel <- 1; s$ll_sel <- r$ll; s$k_nonzero_sel <- r$nonzero
    s$stop <- fit$stop
  }
  s$N <- fit$n; s$k_allvars <- length(fit$vars); s$n_lambda <- fit$n_lambda
  s$lambda_last <- recs[[length(recs)]]$lambda; s$lambda_gmax <- fit$gmax; s$lambda_gmin <- fit$gmin
  all <- if (is.null(fit$allrecs)) recs else fit$allrecs
  col <- function(f) vapply(all, f, 0)
  s$k_nonzero_min <- min(col(function(r) r$nonzero)); s$k_nonzero_max <- max(col(function(r) r$nonzero))
  s$devratio_min <- min(col(function(r) r$devratio)); s$devratio_max <- max(col(function(r) r$devratio))
  s$L1_min <- min(col(function(r) r$l1)); s$L1_max <- max(col(function(r) r$l1))
  s$L2_min <- min(col(function(r) r$l2)); s$L2_max <- max(col(function(r) r$l2))
  if (!user) {
    if (!is.null(r)) { s$ll_sel <- r$ll; s$k_nonzero_sel <- r$nonzero; s$lambda_sel <- r$lambda }
    byid <- function(id) if (is.na(id)) NULL else all[[which(col(function(r) r$id) == id)]]
    if (fit$kind %in% c("cv", "adaptive")) {
      a <- byid(fit$id_se); cvr <- byid(fit$id_cv)
      if (!is.null(a) && !is.null(cvr)) {
        s$k_nonzero_serule <- a$nonzero; s$k_nonzero_cv <- cvr$nonzero; s$ID_serule <- a$id; s$ID_cv <- cvr$id
        s$lambda_serule <- a$lambda; s$lambda_cv <- cvr$lambda; s$cvm_serule <- a$cvm; s$cvm_min <- cvr$cvm
        s$n_fold <- fit$n_fold
      }
    } else if (fit$kind == "bic" && !is.null(r)) {
      s$bic_min <- r$bic; s$k_nonzero_bic <- r$nonzero; s$ID_bic <- r$id; s$lambda_bic <- r$lambda
    }
    s$stop <- fit$stop
    s$ID_sel <- if (is.null(r)) NA_real_ else r$id
  }
  s <- lapply(s, as.numeric)
  m <- list()
  mats <- list()
  if (!is.null(r)) {
    nm <- stata_lasso_names(fit, r$b)
    selv <- setdiff(nm, "_cons")
    m$othervars_sel <- paste(fit$vars[r$b != 0 & !fit$always], collapse = " ")
    m$post_sel_vars <- paste(c(fit$depvar, selv), collapse = " ")
    m$allvars_sel <- paste(selv, collapse = " ")
    m$sel_criterion <- if (user) "user" else if (fit$kind %in% c("cv", "adaptive") && fit$gridmin) "grid min."
      else if (fit$kind %in% c("cv", "adaptive", "bic") && fit$stopsel) "stop"
      else if (fit$kind == "cv" && fit$serule) "serule" else switch(fit$kind, cv = , adaptive = "CV min.", bic = "BIC min.", plugin = "plugin", none = "not sel.")
    o <- stata_lasso_unstd(D, r$b)
    po <- stata_lasso_unstd(D, stata_lasso_post(D, r$b)$b)
    a <- which(r$b != 0)
    pick <- function(v, cons) c(v[a], if (!fit$noconstant) cons)
    mk <- function(v, row) matrix(v, 1, length(nm), dimnames = list(row, nm))
    mats$b <- mk(pick(o, o[D$p + 1]), "y1")
    mats$b_standardized <- mk(pick(r$b, 0), "r1")
    mats$b_postselection <- mk(pick(po, po[D$p + 1]), "r1")
  } else {
    m$sel_criterion <- "not sel."
  }
  alw <- fit$vars[fit$always]
  if (user) {
    m$rngstate <- fit$rngstate; m$depvar <- fit$depvar; m$allvars <- paste(fit$vars, collapse = " ")
    if (length(alw)) m$alwaysvars <- paste(alw, collapse = " ")
    m$model <- "linear"; m$title <- "Lasso"; m$marginsnotok <- "_ALL"; m$predict <- "lasso_p"; m$cmd <- "lasso"
    m$cmdline <- fit$cmdline
  } else {
    m$cmdline <- fit$cmdline; m$cmd <- "lasso"; m$predict <- "lasso_p"; m$marginsnotok <- "_ALL"
    if (fit$kind == "plugin") m$plugin_type <- fit$ptype
    m$lasso_selection <- fit$kind; m$title <- "Lasso"; m$model <- "linear"
    if (length(alw)) m$alwaysvars <- paste(alw, collapse = " ")
    m$allvars <- paste(fit$vars, collapse = " "); m$depvar <- fit$depvar; m$rngstate <- fit$rngstate
    if (!is.null(r)) m$properties <- "b"
  }
  e <- c(s, m, mats)
  attr(e, "stata_sample") <- TRUE
  e
}
stata_lasso_header <- function(fit) {
  cat("\n")
  ad <- fit$kind == "adaptive"
  lw <- if (ad) 43 else 44; lab <- if (ad) 19 else 18
  right <- function(l, v) paste0(formatC(l, width = -lab), "= ", formatC(v, width = 10))
  method <- switch(fit$kind, none = "None", cv = if (fit$serule) "Cross-validation one s.e. rule" else "Cross-validation",
                   adaptive = if (!is.null(fit$power)) "Adaptive power" else "Adaptive",
                   plugin = paste("Plugin", fit$ptype), bic = "Bayesian information criterion")
  cat(formatC("Lasso linear model", width = -lw), right("No. of obs", fit$n), "\n", sep = "")
  cat(strrep(" ", lw), right("No. of covariates", length(fit$vars)), "\n", sep = "")
  third <- switch(fit$kind, cv = right("No. of CV folds", fit$n_fold), adaptive = right("No. of lasso steps", fit$steps), NULL)
  sl <- paste0("Selection: ", method)
  if (is.null(third)) cat(sl, "\n", sep = "") else cat(formatC(sl, width = -lw), third, "\n", sep = "")
  if (ad && !is.null(fit$power)) cat(strrep(" ", lw), right("Power", stata_fmt(fit$power, "%9.3f", trim = TRUE)), "\n", sep = "")
}
stata_lasso_note <- function(fit, knots = FALSE) {
  if (is.na(fit$sel)) return("Note: No lambda selected. lassoselect can be used to select lambda.")
  by <- switch(fit$kind, cv = if (fit$serule) "cross-validation one s.e. rule" else "cross-validation",
               adaptive = "cross-validation in final adaptive step", bic = "Bayesian information criterion",
               plugin = if (knots) paste0("plugin assuming ", fit$ptype, " errors") else paste0("plugin formula assuming ", fit$ptype, " errors"),
               none = "lassoselect")
  paste0("* lambda selected by ", by, ".")
}
stata_lasso_table <- function(fit) {
  stata_lasso_header(fit)
  cat("\n")
  if (fit$kind == "adaptive") cat("Final adaptive step results\n")
  cv <- fit$kind %in% c("cv", "adaptive")
  rule <- strrep("-", 74)
  cat(rule, "\n", sep = "")
  cat("         |                                No. of", if (cv) "      Out-of-      CV mean", "\n", sep = "")
  cat("         |                               nonzero", if (cv) "       sample   prediction" else "    In-sample", "\n", sep = "")
  cat("      ID |     Description      lambda     coef.", if (cv) "    R-squared        error" else "    R-squared          BIC", "\n", sep = "")
  cat("---------+", strrep("-", 64), "\n", sep = "")
  n <- length(fit$recs); s <- fit$sel
  shown <- integer(0); lab <- character(0)
  add <- function(i, l) if (!i %in% shown) { shown <<- c(shown, i); lab <<- c(lab, l) }
  if (fit$kind == "plugin") add(1, "selected lambda") else {
    if (!is.na(s)) add(s, "selected lambda")
    add(1, "first lambda")
    if (n > 0) add(n, "last lambda")
    if (!is.na(s)) { if (s > 1) add(s - 1, "lambda before"); if (s < n) add(s + 1, "lambda after") }
  }
  o <- order(shown)
  for (k in o) {
    r <- fit$recs[[shown[k]]]
    id <- if (!is.na(s) && s == shown[k]) paste0("* ", r$id) else as.character(r$id)
    if (cv) { r2 <- 1 - r$cvm / fit$D$yvar; last <- r$cvm } else { r2 <- r$devratio; last <- r$bic }
    cat(sprintf("%8s |%16s%12s%10d%13s%13s\n", id, lab[k], stata_lasso_g8(r$lambda), as.integer(r$nonzero),
                stata_fmt(r2, "%9.4f", trim = TRUE), stata_lasso_g8(last)))
  }
  cat(rule, "\n", sep = "")
  cat(stata_lasso_note(fit), "\n", sep = "")
  stata_lasso_stopnotes(fit, 74)
}
stata_lasso_stopnotes <- function(fit, width) {
  note <- function(a, b) { w <- paste(a, b); if (nchar(w) <= width) cat(w, "\n", sep = "") else cat(a, "\n      ", b, "\n", sep = "") }
  if (isTRUE(fit$gridmin)) note("Note: Minimum of CV function not found; minimum grid value is selected", "lambda.")
  if (isTRUE(fit$stopsel)) {
    if (fit$kind == "bic") note("Note: Minimum of BIC not found; lambda selected based on stop() stopping", "criterion.")
    else note("Note: Minimum of CV function not found; lambda selected based on stop()", "stopping criterion.")
  }
}
stata_lasso_active <- function() {
  if (!exists("stata_lasso") || !identical(stata_e$cmd, "lasso")) stata_lasso_stop("last estimates not found", 301)
  stata_lasso
}
# lassoselect id = # | lambda = #
stata_lasso_select <- function(what, value) {
  fit <- stata_lasso_active()
  if (fit$kind == "plugin") stata_lasso_stop("lassoselect not allowed with selection(plugin)")
  ids <- vapply(fit$recs, function(r) r$id, 0); lams <- vapply(fit$recs, function(r) r$lambda, 0)
  if (what == "id") {
    if (value != round(value) || value < ids[1] || value > ids[length(ids)]) stata_lasso_stop("id out of range")
    i <- which(ids == value)
  } else {
    if (value > lams[1] || value < lams[length(lams)]) stata_lasso_stop("lambda out of range")
    i <- which.min(abs(lams - value))
  }
  if (!fit$user) fit$orig_sel <- fit$sel
  fit$sel <- i; fit$user <- TRUE
  cat("ID = ", ids[i], "  lambda = ", stata_fmt(lams[i], "%9.0g", trim = TRUE), " selected\n", sep = "")
  nm <- stata_e[["_estimates_name"]]
  stata_lasso <<- fit
  e <- stata_lasso_e(fit)
  # results stored under a name keep it
  if (!is.null(nm)) e[["_estimates_name"]] <- nm
  stata_e <<- e
  invisible(NULL)
}
# lassoknots [, display(...) alllambdas]
stata_lasso_knots <- function(cols = NULL, all = FALSE) {
  fit <- stata_lasso_active()
  cv <- fit$kind %in% c("cv", "adaptive")
  if (is.null(cols)) cols <- c("nonzero", if (cv) "cvm" else "r2", "vars")
  for (k in cols) if (k %in% c("cvm", "osr2") && !cv) stata_lasso_stop(paste0("option ", c(cvm = "cvmpe", osr2 = "osr2")[[k]], " not allowed when no cross-validation result is found"))
  rank <- c(nonzero = 0, cvm = 1, osr2 = 2, r2 = 3, bic = 4, l1 = 5, l2 = 6, vars = 7)
  cols <- cols[order(rank[cols])]
  wv <- "vars" %in% cols
  numc <- setdiff(cols, "vars")
  width <- function(k) switch(k, nonzero = , bic = , l1 = , l2 = 11, cvm = , r2 = if (wv) 10 else 11, osr2 = 12)
  title <- function(k) switch(k, nonzero = c("No. of", "nonzero", "coef."), cvm = c("CV mean", "pred.", "error"),
                              osr2 = c("Out-of-", "sample", "R-squared"), bic = c("", "", "BIC"), r2 = c("", "In-sample", "R-squared"),
                              l1 = c("Relative", "L1", "length"), l2 = c("Relative", "L2", "length"))
  l1m <- max(vapply(fit$recs, function(r) r$l1, 0)); l2m <- max(vapply(fit$recs, function(r) r$l2, 0))
  value <- function(k, r) switch(k, nonzero = as.character(r$nonzero), cvm = stata_lasso_g8(r$cvm),
                                 osr2 = stata_fmt(1 - r$cvm / fit$D$yvar, "%9.4f", trim = TRUE), bic = stata_lasso_g8(r$bic),
                                 r2 = stata_fmt(r$devratio, "%9.4f", trim = TRUE), l1 = stata_fmt(r$l1 / l1m, "%9.4f", trim = TRUE),
                                 l2 = stata_fmt(r$l2 / l2m, "%9.4f", trim = TRUE))
  ls <- max(if (exists("stata_linesize")) stata_linesize else 80, 40)
  numw <- 9 + sum(vapply(numc, width, 0))
  idw <- 6
  left <- idw + 2 + numw
  total <- if (wv) ls else left
  area <- max(ls - (left + 2), 0)
  vw <- max(area - 3, 0) %/% 4
  center <- function(t, w) { n <- nchar(t); if (n >= w) return(t); l <- (w - n + 1) %/% 2; paste0(strrep(" ", l), t, strrep(" ", w - n - l)) }
  padr <- function(s, w) formatC(s, width = -w)
  cat("\n", strrep("-", total), "\n", sep = "")
  top <- any(vapply(numc, function(k) nzchar(title(k)[1]), TRUE))
  for (line in 1:3) {
    if (line == 1 && !wv && !top) next
    # with one column besides lambda, ID meets the bar (probe 501); else a space between
    s <- if (line < 3) paste0(strrep(" ", idw), " |", strrep(" ", 9)) else if (wv || length(numc) > 1) paste0(formatC("ID", width = idw), " |   lambda") else paste0(formatC("ID", width = idw + 1), "|   lambda")
    for (k in numc) s <- paste0(s, formatC(title(k)[line], width = width(k)))
    if (wv) {
      s <- paste0(s, " |")
      if (line == 2) s <- paste0(s, center("Variables (A)dded, (R)emoved,", area))
      if (line == 3) s <- paste0(s, center("or left (U)nchanged", area))
    }
    if (wv && line > 1) cat(padr(s, ls), "\n", sep = "") else cat(sub(" +$", "", s), "\n", sep = "")
  }
  sep <- paste0(strrep("-", idw + 1), "+", strrep("-", numw))
  if (wv) sep <- paste0(sep, "-+", strrep("-", area))
  cat(sep, "\n", sep = "")
  n <- length(fit$recs); prev <- rep(FALSE, length(fit$vars))
  for (i in seq_len(n)) {
    r <- fit$recs[[i]]
    now <- r$b != 0
    added <- fit$vars[now & !prev]; removed <- fit$vars[!now & prev]
    prev <- now
    changed <- length(added) || length(removed)
    marked <- (!is.na(fit$sel) && fit$sel == i) || (!is.na(fit$orig_sel) && fit$orig_sel == i)
    islast <- i == n && n > 1
    if (!(all || changed || marked || islast)) next
    mark <- if (fit$user && !is.na(fit$sel) && fit$sel == i) "+" else if (marked) "*" else ""
    id <- formatC(if (nzchar(mark)) paste(mark, r$id) else as.character(r$id), width = idw)
    lead <- paste0(id, " |", formatC(stata_lasso_g8(r$lambda), width = 9))
    for (k in numc) lead <- paste0(lead, formatC(value(k, r), width = width(k)))
    if (!wv) { cat(lead, "\n", sep = ""); next }
    blankl <- paste0(strrep(" ", idw), " |", strrep(" ", numw))
    groups <- list()
    if (length(added)) groups[[length(groups) + 1]] <- list("A", added)
    if (length(removed)) groups[[length(groups) + 1]] <- list("R", removed)
    if (!length(groups)) groups[[1]] <- list("U", character(0))
    for (g in groups) {
      if (!length(g[[2]])) { cat(padr(paste0(lead, " | ", g[[1]]), ls), "\n", sep = ""); next }
      ch <- split(g[[2]], (seq_along(g[[2]]) - 1) %/% 3)
      for (k in seq_along(ch)) {
        l <- if (k == 1) lead else blankl
        pre <- if (k == 1) paste0(g[[1]], " ") else "  "
        cat(padr(paste0(l, " | ", pre, paste(formatC(ch[[k]], width = -vw), collapse = "")), ls), "\n", sep = "")
      }
    }
  }
  cat(strrep("-", total), "\n", sep = "")
  if (fit$user) {
    cat("+ lambda selected by lassoselect.\n")
    if (!is.na(fit$orig_sel)) { f2 <- fit; f2$sel <- f2$orig_sel; cat(stata_lasso_note(f2, TRUE), "\n", sep = "") }
  } else cat(stata_lasso_note(fit, TRUE), "\n", sep = "")
  invisible(NULL)
}
# the e() of estimates named in lassocoef or lassoinfo: stored ones, or the active ones
stata_lasso_named <- function(names) {
  act <- function() {
    if (!exists("stata_e") || !identical(stata_e$cmd, "lasso")) stata_lasso_stop("last estimates not found", 301)
    nm <- stata_e[["_estimates_name"]]
    list(name = if (is.null(nm)) "active" else nm, e = stata_e)
  }
  if (!length(names)) return(list(act()))
  lapply(names, function(w) {
    if (w == ".") return(act())
    st <- if (exists("stata_stored")) stata_stored[[w]] else NULL
    if (is.null(st) || is.null(st$stata_e)) stata_lasso_stop(paste0("estimates ", w, " not found"), 111)
    list(name = w, e = st$stata_e)
  })
}
# lassocoef [names] [, display(coef[, ...]) sort(coef[, ...]) nolegend]
stata_lasso_coef <- function(names = character(0), coef = NULL, sortby = NULL, legend = TRUE) {
  lst <- stata_lasso_named(names)
  vars <- character(0)
  for (x in lst) for (v in setdiff(colnames(x$e$b), "_cons")) if (!v %in% vars) vars <- c(vars, v)
  val <- function(e, which, v) { m <- e[[which]]; j <- match(v, colnames(m)); if (is.na(j)) NA_real_ else m[1, j] }
  if (!is.null(sortby)) {
    key <- vapply(vars, function(v) { k <- vapply(lst, function(x) val(x$e, sortby, v), 0); k <- k[!is.na(k)]; if (length(k)) abs(k[1]) else 0 }, 0)
    vars <- vars[order(-key)]
  }
  if (any(vapply(lst, function(x) "_cons" %in% colnames(x$e$b), TRUE))) vars <- c(vars, "_cons")
  colw <- function(k) if (k == 1) 10 else 11
  width <- 14 + sum(vapply(seq_along(lst), colw, 0))
  center <- function(t, w) { n <- nchar(t); if (n >= w) return(t); l <- (w - n + 1) %/% 2; paste0(strrep(" ", l), t, strrep(" ", w - n - l)) }
  cat("\n", strrep("-", width), "\n", sep = "")
  h <- paste0(strrep(" ", 12), " |")
  for (k in seq_along(lst)) h <- paste0(h, if (!is.null(coef)) formatC(lst[[k]]$name, width = colw(k)) else center(lst[[k]]$name, colw(k)))
  cat(h, "\n", "-------------+", strrep("-", width - 14), "\n", sep = "")
  for (v in vars) {
    cells <- vapply(seq_along(lst), function(k) {
      e <- lst[[k]]$e; w <- colw(k)
      if (!is.null(coef)) { x <- val(e, coef, v); if (is.na(x)) "" else formatC(stata_fmt(x, "%9.0g", trim = TRUE), width = w) }
      else if (v %in% colnames(e$b)) center("x", w) else ""
    }, "")
    last <- max(c(0, which(nzchar(cells))))
    s <- paste0(formatC(v, width = 12), " |")
    for (k in seq_len(last)) s <- paste0(s, formatC(cells[k], width = -colw(k)))
    cat(s, "\n", sep = "")
  }
  cat(strrep("-", width), "\n", sep = "")
  if (legend) cat("Legend:\n  b - base level\n  e - empty cell\n  o - omitted\n", if (is.null(coef)) "  x - estimated\n", sep = "")
  invisible(NULL)
}
# lassoinfo [names]
stata_lasso_info <- function(names = character(0)) {
  lst <- stata_lasso_named(names)
  for (x in lst) {
    e <- x$e
    nm <- if (!length(names)) "active" else x$name
    method <- e$lasso_selection; crit <- e$sel_criterion
    lam <- if (is.null(e$lambda_sel) || is.na(e$lambda_sel)) "" else stata_lasso_g8(e$lambda_sel)
    nsel <- if (is.null(e$k_nonzero_sel) || is.na(e$k_nonzero_sel)) "" else as.character(e$k_nonzero_sel)
    cat("\n    Estimate: ", nm, "\n     Command: lasso\n", sep = "")
    if (!method %in% c("plugin", "none")) {
      cat(strrep("-", 65), "\n            |                                              No. of\n",
          "  Dependent |           Selection  Selection             selected\n",
          "   variable |    Model     method  criterion    lambda  variables\n",
          "------------+", strrep("-", 52), "\n", sep = "")
      cat(sprintf("%11s |%9s%11s%11s%10s%11s\n", e$depvar, "linear", method, crit, lam, nsel))
      cat(strrep("-", 65), "\n", sep = "")
    } else {
      cat(strrep("-", 54), "\n            |                                   No. of\n",
          "  Dependent |           Selection             selected\n",
          "   variable |    Model     method    lambda  variables\n",
          "------------+", strrep("-", 41), "\n", sep = "")
      cat(sprintf("%11s |%9s%11s%10s%11s\n", e$depvar, "linear", method, lam, nsel))
      cat(strrep("-", 54), "\n", sep = "")
    }
  }
  invisible(NULL)
}
# predict after lasso: the linear prediction with the penalized or the
# postselection coefficients, and Stata's note on what it assumed
stata_lasso_predict <- function(d, post = FALSE, xb = FALSE, penalized = FALSE, show = TRUE) {
  if (!exists("stata_e") || !identical(stata_e$cmd, "lasso")) stata_lasso_stop("last estimates not found", 301)
  kind <- if (post) "postselection" else "penalized"
  if (show) {
    if (!xb && !(post || penalized)) cat("(options xb penalized assumed; linear prediction with ", kind, " coefficients)\n", sep = "")
    else if (!xb) cat("(option xb assumed; linear prediction with ", kind, " coefficients)\n", sep = "")
    else if (!(post || penalized)) cat("(option penalized assumed; linear prediction with ", kind, " coefficients)\n", sep = "")
  }
  b <- stata_e[[if (post) "b_postselection" else "b"]]
  if (is.null(b)) stata_lasso_stop("last estimates not found", 301)
  nm <- colnames(b)
  eta <- if ("_cons" %in% nm) rep(b[1, "_cons"], nrow(d)) else rep(0, nrow(d))
  for (v in setdiff(nm, "_cons")) { x <- unclass(d[[v]]); attributes(x) <- NULL; eta <- eta + as.numeric(x) * b[1, v] }
  eta
}
"###;

/// The e() names a lasso leaves that are text.
const LASSO_TEXTS: &[&str] = &[
    "othervars_sel",
    "post_sel_vars",
    "allvars_sel",
    "sel_criterion",
    "cmdline",
    "cmd",
    "predict",
    "marginsnotok",
    "plugin_type",
    "lasso_selection",
    "title",
    "model",
    "alwaysvars",
    "allvars",
    "depvar",
    "rngstate",
    "properties",
];

/// Stata's message for `lassoselect` without `id = #` or `lambda = #`.
const SELECT_SYNTAX: &str =
    "invalid expression\n    Expression must be one of\n        id = #\n        lambda = #";

/// `name(arg)` and `name` words of an option's text, split at the top level.
fn sub_options(text: &str) -> Vec<(String, Option<String>)> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut cur = String::new();
    let mut push = |cur: &mut String| {
        let t = cur.trim();
        if !t.is_empty() {
            match t.split_once('(') {
                Some((n, a)) => out.push((
                    n.trim().to_string(),
                    Some(a.strip_suffix(')').unwrap_or(a).to_string()),
                )),
                None => out.push((t.to_string(), None)),
            }
        }
        cur.clear();
    };
    for ch in text.chars() {
        match ch {
            '(' => {
                depth += 1;
                cur.push(ch);
            }
            ')' => {
                depth = depth.saturating_sub(1);
                cur.push(ch);
                if depth == 0 {
                    push(&mut cur);
                }
            }
            c if c.is_whitespace() && depth == 0 => push(&mut cur),
            _ => cur.push(ch),
        }
    }
    push(&mut cur);
    out
}

/// Whether `word` is `full` or an abbreviation of it at least `min` long.
fn abbr(word: &str, full: &str, min: usize) -> bool {
    word.len() >= min && full.starts_with(word)
}

/// A whole number written out.
fn whole(t: &str) -> Option<f64> {
    t.trim()
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite() && v.fract() == 0.0)
}

/// A number written out.
fn number(t: &str) -> Option<f64> {
    t.trim().parse::<f64>().ok().filter(|v| v.is_finite())
}

/// The R arguments of `stata_lasso_run` beyond the variables, or Stata's
/// error (message, code) for options it refuses, or a reason RCraft does not
/// translate them.
enum Parsed {
    Args(Vec<String>),
    Stata(String, u32),
}

impl Session {
    /// A line Stata stops with an error: the same message and code in R
    /// (capture takes it, `_rc` holds the code).
    fn lasso_error(&mut self, out: &mut Out, msg: &str, rc: u32) -> Result<(), Untranslated> {
        out.line(format!(
            "stop({}, call. = FALSE)",
            r_text(&format!("{msg} (Stata: r({rc}))"))
        ));
        Ok(())
    }

    fn lasso_options(&self, opts: &Opts) -> Result<Parsed, Untranslated> {
        let mut args: Vec<String> = Vec::new();
        let stata = |m: &str| Ok(Parsed::Stata(m.to_string(), 198));
        // alpha() is elasticnet's (Stata: alphas(#) not allowed)
        for o in opts.0 {
            if o.is("alphas", 5) {
                return stata(&format!("option alphas({}) not allowed", o.arg().trim()));
            }
        }
        if opts.0.iter().filter(|o| o.is("selection", 3)).count() > 1 {
            return stata("option selection() not allowed");
        }
        if let Some(o) = opts.other(&[
            ("selection", 3),
            ("grid", 3),
            ("stop", 3),
            ("nolog", 3),
            ("log", 3),
            ("rseed", 3),
            ("noconstant", 3),
            ("tolerance", 3),
            ("penaltywt", 4),
        ]) {
            // lasso's other options in its manual entry: not translated yet;
            // any other word is Stata's r(198) (probe 384)
            let known = [
                ("offset", 3),
                ("exposure", 3),
                ("cluster", 2),
                ("cvtolerance", 5),
                ("bictolerance", 6),
                ("dtolerance", 4),
                ("toltype", 7),
                ("verbose", 4),
                ("level", 1),
                // cluster() reaches here as vce(cluster ...)
                ("vce", 3),
            ];
            if opts
                .0
                .iter()
                .any(|x| x.name == o && known.iter().any(|(f, m)| x.is(f, *m)))
            {
                return Err(no(format!(
                    "lasso with the option {o} is not translated yet"
                )));
            }
            return stata(&format!("option {o} not allowed"));
        }
        let mut kind = "cv";
        let mut exclusive = 0;
        if let Some(sel) = opts.arg("selection", 3) {
            let (head, tail) = match sel.split_once(',') {
                Some((h, t)) => (h.trim().to_string(), t.to_string()),
                None => (sel.trim().to_string(), String::new()),
            };
            let h = head.as_str();
            kind = if abbr(h, "cv", 2) {
                "cv"
            } else if abbr(h, "adaptive", 1) {
                "adaptive"
            } else if abbr(h, "none", 1) {
                "none"
            } else if abbr(h, "plugin", 1) {
                "plugin"
            } else if abbr(h, "bic", 1) {
                "bic"
            } else {
                return stata(
                    "option selection() invalid\n    Only one of cv, adaptive, none, plugin, or bic is allowed.",
                );
            };
            for (name, arg) in sub_options(&tail) {
                let n = name.as_str();
                let a = arg.clone().unwrap_or_default();
                let cvlike = matches!(kind, "cv" | "adaptive");
                if cvlike && abbr(n, "folds", 3) {
                    match whole(&a) {
                        Some(f) if f >= 3.0 => args.push(format!("folds = {}", r_number(f))),
                        _ => return stata("option folds() must be an integer >=3"),
                    }
                } else if kind == "cv" && abbr(n, "serule", 3) {
                    args.push("serule = TRUE".into());
                } else if cvlike && abbr(n, "alllambdas", 3) {
                    args.push("alllambdas = TRUE".into());
                } else if cvlike && abbr(n, "gridminok", 5) {
                    exclusive += 1;
                    args.push("gridminok = TRUE".into());
                } else if cvlike && abbr(n, "stopok", 4) {
                    exclusive += 1;
                    args.push("stopok = TRUE".into());
                } else if cvlike && abbr(n, "strict", 3) {
                    exclusive += 1;
                    args.push("strict = TRUE".into());
                } else if kind == "adaptive" && abbr(n, "steps", 3) {
                    match whole(&a) {
                        Some(s) if s >= 1.0 => args.push(format!("steps = {}", r_number(s))),
                        _ => {
                            return Err(no(format!(
                                "lasso, selection(adaptive, steps({a})) is not translated"
                            )))
                        }
                    }
                } else if kind == "adaptive" && abbr(n, "power", 3) {
                    match number(&a) {
                        Some(p) if p > 0.0 => args.push(format!("power = {}", r_number(p))),
                        _ => {
                            return Err(no(format!(
                                "lasso, selection(adaptive, power({a})) is not translated"
                            )))
                        }
                    }
                } else if kind == "plugin" && abbr(n, "heteroskedastic", 6) {
                    args.push("hetero = TRUE".into());
                } else if kind == "plugin" && abbr(n, "homoskedastic", 5) {
                    args.push("hetero = FALSE".into());
                } else if kind == "plugin" && n == "c" {
                    match number(&a) {
                        Some(v) if v > 0.0 => args.push(format!("cc = {}", r_number(v))),
                        _ => {
                            return Err(no(format!(
                                "lasso, selection(plugin, c({a})) is not translated"
                            )))
                        }
                    }
                } else if kind == "plugin" && abbr(n, "gamma", 3) {
                    match number(&a) {
                        Some(v) if v > 0.0 && v < 1.0 => {
                            args.push(format!("gamma = {}", r_number(v)))
                        }
                        _ => {
                            return Err(no(format!(
                                "lasso, selection(plugin, gamma({a})) is not translated"
                            )))
                        }
                    }
                } else if kind == "bic" && abbr(n, "postselection", 4) {
                    args.push("postsel = TRUE".into());
                } else {
                    return Err(no(format!(
                        "lasso, selection({kind}) with {n} is not translated yet"
                    )));
                }
            }
        }
        if exclusive > 1 {
            return stata("only one of stopok, strict, or gridminok is allowed");
        }
        args.insert(0, format!("sel = {}", r_text(kind)));
        if let Some(g) = opts.arg("grid", 3) {
            let (head, tail) = match g.split_once(',') {
                Some((h, t)) => (h.to_string(), t.to_string()),
                None => (g.clone(), String::new()),
            };
            match whole(&head) {
                Some(n) if n >= 1.0 => args.push(format!("grid = {}", r_number(n))),
                _ => return stata("option grid() must be a positive integer"),
            }
            for (name, arg) in sub_options(&tail) {
                let v = arg.as_deref().and_then(number);
                match (name.as_str(), v) {
                    (n, Some(r)) if abbr(n, "ratio", 3) && r > 0.0 && r < 1.0 => {
                        args.push(format!("ratio = {}", r_number(r)))
                    }
                    (n, Some(m)) if abbr(n, "min", 3) && m > 0.0 => {
                        args.push(format!("lmin = {}", r_number(m)))
                    }
                    (n, _) => {
                        return Err(no(format!("lasso, grid() with {n} is not translated yet")))
                    }
                }
            }
        }
        if let Some(s) = opts.arg("stop", 3) {
            match number(&s) {
                Some(v) if v >= 0.0 => args.push(format!("stop = {}", r_number(v))),
                _ => return Err(no(format!("lasso, stop({s}) is not translated"))),
            }
        }
        if opts.has("nolog", 3) {
            args.push("nolog = TRUE".into());
        }
        if let Some(s) = opts.arg("rseed", 3) {
            match whole(&s) {
                Some(v) if v >= 0.0 => args.push(format!("rseed = {}", r_number(v))),
                _ => {
                    return Err(no(format!(
                        "lasso, rseed({s}): a whole number is translated"
                    )))
                }
            }
        }
        if opts.has("noconstant", 3) {
            args.push("noconstant = TRUE".into());
        }
        if let Some(t) = opts.arg("tolerance", 3) {
            match number(&t) {
                // the default, written out
                Some(1e-7) => {}
                Some(v) if v > 0.0 && v < 1.0 => {
                    return Err(no(format!(
                        "lasso, tolerance({t}): Stata's convergence test at other tolerances is not decoded (its coefficients differ from RCraft's in the 6th digit at tolerance(1e-3))"
                    )))
                }
                Some(_) => {
                    return stata("tolerance() invalid -- invalid number, outside of allowed range")
                }
                None => return Err(no(format!("lasso, tolerance({t}) is not translated"))),
            }
        }
        if let Some(p) = opts.arg("penaltywt", 4) {
            let p = p.trim();
            let words: Vec<&str> = p.split_whitespace().collect();
            let nums: Option<Vec<f64>> = words.iter().map(|w| number(w)).collect();
            match nums {
                Some(v) if !v.is_empty() => args.push(format!(
                    "penaltywt = c({})",
                    v.iter().map(|x| r_number(*x)).collect::<Vec<_>>().join(", ")
                )),
                _ if is_name(p) && self.user_mats.contains(p) => args.push(format!(
                    "penaltywt = as.numeric(stata_mat[[{}]])",
                    r_text(p)
                )),
                _ => {
                    return Err(no(format!(
                        "lasso, penaltywt({p}): a number list or a matrix this do-file made is translated"
                    )))
                }
            }
        }
        Ok(Parsed::Args(args))
    }

    /// `lasso linear depvar [(alwaysvars)] othervars [if] [in] [, options]`.
    pub(super) fn lasso(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "lasso")?;
        if c.parts.weight.is_some() {
            return Err(no("lasso with weights is not translated yet"));
        }
        let main = c.parts.main.trim().to_string();
        let (model, rest) = first_word(&main);
        match model {
            "linear" => {}
            "logit" | "probit" | "poisson" | "cox" => {
                return Err(no(format!(
                    "lasso {model} is not translated yet (lasso linear is)"
                )))
            }
            "" => return Err(no("lasso without a model and variables")),
            _ => {
                return self.lasso_error(
                    out,
                    "invalid model\n    Only one of linear, logit, probit, or poisson is allowed.",
                    198,
                )
            }
        }
        // depvar [(alwaysvars)] othervars
        let (always, others_text) = match (rest.find('('), rest.find(')')) {
            (Some(a), Some(b)) if a < b => (
                rest.get(a + 1..b).unwrap_or("").to_string(),
                format!(
                    "{} {}",
                    rest.get(..a).unwrap_or(""),
                    rest.get(b + 1..).unwrap_or("")
                ),
            ),
            (None, None) => (String::new(), rest.to_string()),
            _ => {
                return Err(no(
                    "lasso: a ( without its ), or more than one group of always-in variables",
                ))
            }
        };
        let others: Vec<&str> = others_text.split_whitespace().collect();
        let always: Vec<&str> = always.split_whitespace().collect();
        // Stata's words with r(100) when nothing follows the model, r(198)
        // with a dependent variable alone (probes 383 and 384, lasso.do)
        let required = "dependent variable depvar and independent variables othervars required\n    The full syntax is\n        lasso linear depvar [(alwaysvars)] othervars [, ...]\n    Also see lasso.";
        if others.is_empty() && always.is_empty() {
            return self.lasso_error(out, required, 100);
        }
        if others.len() + always.len() < 2 || others.is_empty() {
            return self.lasso_error(out, required, 198);
        }
        if let Some(v) = others.iter().chain(always.iter()).find(|v| !is_name(v)) {
            return Err(no(if v.contains('.') {
                format!("lasso with factor variables ({v}) is not translated yet")
            } else {
                // a range or pattern the run makes the data's names (a dry run cannot)
                format!(
                    "lasso {v}: the data's variable names are {}",
                    crate::dofile::KNOWN_WHEN_RUN
                )
            }));
        }
        let (dep, others) = others
            .split_first()
            .map(|(d, o)| (*d, o.to_vec()))
            .unwrap_or(("", Vec::new()));
        if others.is_empty() && always.is_empty() {
            return self.lasso_error(out, required, 198);
        }
        if others.is_empty() {
            return Err(no(
                "lasso with always-in variables only is not translated yet",
            ));
        }
        let args = match self.lasso_options(opts)? {
            Parsed::Stata(msg, rc) => return self.lasso_error(out, &msg, rc),
            Parsed::Args(a) => a,
        };
        let rows = self.rows_for(c, out)?;
        let d = self.data()?;
        out.package("dplyr");
        for h in ["stata_lasso", "stata_fmt", "stata_rng", "stata_returned"] {
            self.helpers.insert(h);
        }
        let list = |v: &[&str]| {
            format!(
                "c({})",
                v.iter().map(|x| r_text(x)).collect::<Vec<_>>().join(", ")
            )
        };
        out.line(format!(
            "stata_lasso_run(local({{ {d} <- dplyr::mutate({d}, stata_row = seq_len(nrow({d}))); {rows} }}), nrow({d}), {}, {}, {}, {}, cmdline = {}, show = {})",
            r_text(dep),
            if always.is_empty() { "character(0)".to_string() } else { list(&always) },
            list(&others),
            args.join(", "),
            r_text(&super::returned::cmdline(c)),
            super::stats::r_logical(!self.quiet(c))
        ));
        self.esample_new = true;
        self.e_new = true;
        self.estimates = None;
        self.estimates_lost = Some(
            "test, lincom, margins and the other post-estimation commands are not translated after lasso yet"
                .into(),
        );
        self.e_text.clear();
        self.fit_ts = None;
        self.e_ret = Returned::Program {
            cmd: "lasso".into(),
            texts: LASSO_TEXTS.iter().map(|s| s.to_string()).collect(),
        };
        Ok(())
    }

    /// Whether the active results are a lasso's this do-file translated.
    fn lasso_active(&self) -> bool {
        matches!(&self.e_ret, Returned::Program { cmd, .. } if cmd == "lasso")
    }

    fn lasso_need(&self, what: &str) -> Result<(), Untranslated> {
        if self.lasso_active() {
            return Ok(());
        }
        Err(no(match &self.e_ret {
            Returned::Lost(why) => format!("{what}: {why}"),
            Returned::Nothing => format!("{what}: no lasso is fitted before it in this do-file"),
            _ => format!("{what} after a model other than lasso is not translated yet"),
        }))
    }

    /// The names given to lassocoef or lassoinfo: `.` or results stored from a lasso.
    fn lasso_names(&self, what: &str, main: &str) -> Result<String, Untranslated> {
        let words: Vec<&str> = main.split_whitespace().collect();
        if words.is_empty() || words.contains(&".") {
            self.lasso_need(what)?;
        }
        for w in &words {
            if *w == "." {
                continue;
            }
            match self.stored_fits.get(*w) {
                Some(f) if f.is_lasso() => {}
                Some(f) if f.later().is_some() => {
                    return Err(no(format!("{what} {w}: {}", f.later().unwrap_or_default())))
                }
                Some(_) => {
                    return Err(no(format!(
                        "{what} {w}: those results are not a lasso's"
                    )))
                }
                None if self.stores_lost => {
                    return Err(no(format!(
                        "{what} {w}: a line not translated may have stored it"
                    )))
                }
                None => {
                    return Err(no(format!(
                        "{what} {w}: no results are stored under that name before it (Stata: estimation result {w} not found, r(111))"
                    )))
                }
            }
        }
        Ok(format!(
            "c({})",
            words
                .iter()
                .map(|w| r_text(w))
                .collect::<Vec<_>>()
                .join(", ")
        ))
    }

    /// `lassoselect`, `lassoknots`, `lassocoef` and `lassoinfo` after lasso.
    pub(super) fn lasso_after(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let name = c.name.as_str();
        if c.parts.if_exp.is_some() || c.parts.in_range.is_some() || c.parts.weight.is_some() {
            return Err(no(format!(
                "{name} with if, in or weights is not translated"
            )));
        }
        self.helpers.insert("stata_lasso");
        self.helpers.insert("stata_fmt");
        self.helpers.insert("stata_rng");
        let show = !self.quiet(c);
        match name {
            "lassoselect" => {
                self.lasso_need(name)?;
                if !c.options.is_empty() {
                    return Err(no("lassoselect with options is not translated yet"));
                }
                let text = c.parts.main.trim();
                let parsed = text.split_once('=').and_then(|(l, r)| {
                    let l = l.trim();
                    let v = number(r)?;
                    match l {
                        "id" | "lambda" => Some((l.to_string(), v)),
                        _ => None,
                    }
                });
                let Some((what, v)) = parsed else {
                    return self.lasso_error(out, SELECT_SYNTAX, 198);
                };
                let call = format!("stata_lasso_select({}, {})", r_text(&what), r_number(v));
                out.line(if show {
                    call
                } else {
                    format!("invisible(utils::capture.output({call}))")
                });
                self.e_ret = Returned::Program {
                    cmd: "lasso".into(),
                    texts: LASSO_TEXTS.iter().map(|s| s.to_string()).collect(),
                };
                self.e_text.clear();
                Ok(())
            }
            "lassoknots" => {
                self.lasso_need(name)?;
                if !c.parts.main.trim().is_empty() {
                    return Err(no("lassoknots with a name or for() is not translated yet"));
                }
                if let Some(o) = opts.other(&[("display", 3), ("alllambdas", 3)]) {
                    return Err(no(format!(
                        "lassoknots with the option {o} is not translated yet"
                    )));
                }
                let mut cols: Vec<&str> = Vec::new();
                if let Some(d) = opts.arg("display", 3) {
                    for w in d.split_whitespace() {
                        let k = match w {
                            "nonzero" => "nonzero",
                            "cvmpe" => "cvm",
                            "osr2" => "osr2",
                            "bic" => "bic",
                            "r2" => "r2",
                            "l1" => "l1",
                            "l2" => "l2",
                            "variables" => "vars",
                            "cvmd" | "cvdevratio" | "devratio" => {
                                return Err(no(format!(
                                    "lassoknots, display({w}) is for lasso logit, probit and poisson, not translated yet"
                                )))
                            }
                            _ => {
                                return self.lasso_error(
                                    out,
                                    &format!("option {w} not allowed"),
                                    198,
                                )
                            }
                        };
                        cols.push(k);
                    }
                    if cols.len() > 3 {
                        return self.lasso_error(out, "too many options specified\n    only 3 options of nonzero, {cvmpe | cvmd}, {cvdevratio | osr2}, bic, {r2 | devratio}, l1, l2, or variables can be specified in display()", 198);
                    }
                    if cols.is_empty() {
                        return Err(no(
                            "lassoknots, display() with nothing in it is not translated",
                        ));
                    }
                }
                let cols_r = if cols.is_empty() {
                    "NULL".to_string()
                } else {
                    format!(
                        "c({})",
                        cols.iter()
                            .map(|k| r_text(k))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                };
                let call = format!(
                    "stata_lasso_knots({cols_r}, all = {})",
                    super::stats::r_logical(opts.has("alllambdas", 3))
                );
                if show {
                    out.line(call);
                }
                Ok(())
            }
            "lassocoef" => {
                let names = self.lasso_names(name, &c.parts.main)?;
                if let Some(o) = opts.other(&[("display", 3), ("sort", 3), ("nolegend", 5)]) {
                    return Err(no(format!(
                        "lassocoef with the option {o} is not translated yet"
                    )));
                }
                // display(coef[, standardized | penalized | postselection]), sort() the same
                let which = |arg: &str| -> Option<&'static str> {
                    let (head, tail) = match arg.split_once(',') {
                        Some((h, t)) => (h.trim(), t.trim()),
                        None => (arg.trim(), ""),
                    };
                    if head != "coef" {
                        return None;
                    }
                    Some(match tail {
                        "" | "standardized" => "b_standardized",
                        "penalized" => "b",
                        "postselection" => "b_postselection",
                        _ => return None,
                    })
                };
                let coef = match opts.arg("display", 3) {
                    None => "NULL".to_string(),
                    Some(a) => match which(&a) {
                        Some(w) => r_text(w),
                        None => {
                            return Err(no(format!(
                                "lassocoef, display({a}) is not translated yet"
                            )))
                        }
                    },
                };
                let sortby = match opts.arg("sort", 3) {
                    None => "NULL".to_string(),
                    Some(a) => match which(&a) {
                        Some(w) => r_text(w),
                        None => {
                            return Err(no(format!("lassocoef, sort({a}) is not translated yet")))
                        }
                    },
                };
                if show {
                    out.line(format!(
                        "stata_lasso_coef({names}, coef = {coef}, sortby = {sortby}, legend = {})",
                        super::stats::r_logical(!opts.has("nolegend", 5))
                    ));
                }
                Ok(())
            }
            "lassoinfo" => {
                let names = self.lasso_names(name, &c.parts.main)?;
                if !c.options.is_empty() {
                    return Err(no("lassoinfo with options is not translated yet"));
                }
                if show {
                    out.line(format!("stata_lasso_info({names})"));
                }
                Ok(())
            }
            _ => Err(no(format!("{name} is not translated yet"))),
        }
    }

    /// `predict [type] newvar [if] [in] [, xb penalized postselection]` after lasso.
    pub(super) fn lasso_predict(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if opts.has("residuals", 1) {
            return self.lasso_error(out, "option residuals not allowed", 198);
        }
        if let Some(o) = opts.other(&[("xb", 2), ("penalized", 3), ("postselection", 4)]) {
            return Err(no(format!(
                "predict after lasso with the option {o} is not translated yet"
            )));
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
            return Err(no("predict after lasso makes one new variable"));
        };
        if !is_name(w) {
            return Err(no(format!(
                "{w}: only a plain new variable name is translated"
            )));
        }
        self.helpers.insert("stata_lasso");
        self.helpers.insert("stata_fmt");
        self.helpers.insert("stata_rng");
        let base = self.data()?;
        out.line(format!(
            "if ({} %in% names({base})) stop(\"variable {w} already defined (Stata: r(110))\", call. = FALSE)",
            r_text(w)
        ));
        out.line(format!(
            "stata_lasso_p <- stata_lasso_predict({base}, post = {}, xb = {}, penalized = {}, show = {})",
            super::stats::r_logical(opts.has("postselection", 4)),
            super::stats::r_logical(opts.has("xb", 2)),
            super::stats::r_logical(opts.has("penalized", 3)),
            super::stats::r_logical(!self.quiet(c))
        ));
        let v = stored("stata_lasso_p", st, &mut self.helpers, out);
        let v = match self.condition(c, out)? {
            Some(cond) => format!("dplyr::if_else({cond}, {v}, NA_real_)"),
            None => v,
        };
        out.package("dplyr");
        out.line(format!(
            "{base} <- dplyr::mutate({base}, {} = {v})",
            r_name(w)
        ));
        out.line("rm(stata_lasso_p)");
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
    fn lasso_linear_becomes_one_call_with_statas_options() {
        let out = r("use d\nlasso linear y (x7) x1 x2 x3 if x1 > 0, selection(cv, folds(5) serule) rseed(12) grid(30, ratio(.01)) stop(0) nolog");
        let l = &out[1];
        assert!(
            l.contains(
                "stata_lasso_run(local({ d <- dplyr::mutate(d, stata_row = seq_len(nrow(d)))"
            ),
            "{l}"
        );
        assert!(
            l.contains("\"y\", c(\"x7\"), c(\"x1\", \"x2\", \"x3\")"),
            "{l}"
        );
        for a in [
            "sel = \"cv\"",
            "folds = 5",
            "serule = TRUE",
            "grid = 30",
            "ratio = 0.01",
            "stop = 0",
            "nolog = TRUE",
            "rseed = 12",
        ] {
            assert!(l.contains(a), "{a}: {l}");
        }
        assert!(
            l.contains("cmdline = \"lasso linear y (x7) x1 x2 x3 if x1 > 0, selection(cv, folds(5) serule)"),
            "{l}"
        );
        // the last model's results go first
        assert!(l.starts_with("rm(list = intersect("), "{l}");
        let out = r("use d\nlasso linear y x1 x2, selection(plugin, homoskedastic c(1.5) gamma(.05)) penaltywt(1 2) noconstant");
        for a in [
            "sel = \"plugin\"",
            "hetero = FALSE",
            "cc = 1.5",
            "gamma = 0.05",
            "penaltywt = c(1, 2)",
            "noconstant = TRUE",
        ] {
            assert!(out[1].contains(a), "{a}: {}", out[1]);
        }
        let out = r("use d\nquietly lasso linear y x1 x2, selection(bic, postselection)");
        assert!(
            out[1].contains("postsel = TRUE") && out[1].contains("show = FALSE"),
            "{}",
            out[1]
        );
    }

    #[test]
    fn lasso_stops_where_stata_does_with_its_words() {
        let out = r("use d
lasso linear y x1 x2, selection(foo)
lasso y x1 x2
lasso linear
lasso linear y, selection(bic)
lasso linear y x1 x2, selection(cv, folds(2))
lasso linear y x1 x2, grid(0)
lasso linear y x1 x2, bogus
lasso linear y x1 x2, selection(plugin) selection(bic)
lasso linear y x1 x2, alpha(1)
lasso linear y x1 x2, tolerance(2)");
        let want = [
            ("option selection() invalid\\n    Only one of cv, adaptive, none, plugin, or bic is allowed.", 198),
            ("invalid model\\n    Only one of linear, logit, probit, or poisson is allowed.", 198),
            ("dependent variable depvar and independent variables othervars required", 100),
            ("dependent variable depvar and independent variables othervars required", 198),
            ("option folds() must be an integer >=3", 198),
            ("option grid() must be a positive integer", 198),
            ("option bogus not allowed", 198),
            ("option selection() not allowed", 198),
            ("option alphas(1) not allowed", 198),
            ("tolerance() invalid -- invalid number, outside of allowed range", 198),
        ];
        assert_eq!(out.len(), want.len() + 1, "{out:?}");
        for (line, (msg, rc)) in out.iter().skip(1).zip(want) {
            assert!(
                line.contains("stop(\"") && line.contains(msg),
                "{msg}: {line}"
            );
            assert!(line.contains(&format!("(Stata: r({rc}))")), "{rc}: {line}");
        }
    }

    #[test]
    fn what_lasso_does_not_match_yet_is_refused() {
        let out = r("use d
lasso logit y x1 x2
lasso linear y i.g x1
lasso linear y x1 x2, tolerance(1e-3)
lasso linear y x1 x2, cluster(g)
lasso linear y x1 x2 [pw = w]");
        assert!(
            out[1].contains("NOT: lasso logit is not translated yet"),
            "{out:?}"
        );
        assert!(
            out[2].contains("NOT: lasso with factor variables (i.g)"),
            "{out:?}"
        );
        assert!(out[3].contains("NOT: lasso, tolerance(1e-3)"), "{out:?}");
        // cluster(g) reaches the translator as vce(cluster g)
        assert!(out[4].contains("NOT: lasso with the option vce"), "{out:?}");
        assert!(out[5].contains("NOT: lasso with weights"), "{out:?}");
        // the default tolerance written out is the default
        let out = r("use d\nlasso linear y x1 x2, tolerance(1e-7)");
        assert!(out[1].contains("stata_lasso_run("), "{out:?}");
    }

    #[test]
    fn what_follows_a_lasso_reads_it() {
        let out = r("use d
lasso linear y x1 x2, selection(none)
lassoselect id = 4
lassoselect lambda = .1
lassoselect id
lassoknots, display(nonzero cvmpe bic) alllambdas
lassoknots, display(nonzero l1-norm)
lassocoef, display(coef, postselection) sort(coef, standardized) nolegend
lassoinfo
predict double p
predict q, postselection
predict r, residuals
ereturn list
display e(lambda_sel) e(sel_criterion)");
        assert!(out[2].contains("stata_lasso_select(\"id\", 4)"), "{out:?}");
        assert!(
            out[3].contains("stata_lasso_select(\"lambda\", 0.1)"),
            "{out:?}"
        );
        assert!(
            out[4].contains("invalid expression") && out[4].contains("(Stata: r(198))"),
            "{out:?}"
        );
        assert!(
            out[5].contains("stata_lasso_knots(c(\"nonzero\", \"cvm\", \"bic\"), all = TRUE)"),
            "{out:?}"
        );
        assert!(
            out[6].contains("option l1-norm not allowed (Stata: r(198))"),
            "{out:?}"
        );
        assert!(
            out[7].contains("stata_lasso_coef(c(), coef = \"b_postselection\", sortby = \"b_standardized\", legend = FALSE)"),
            "{out:?}"
        );
        assert!(out[8].contains("stata_lasso_info(c())"), "{out:?}");
        assert!(
            out[9].contains("stata_lasso_predict(d, post = FALSE")
                && out[9].contains("dplyr::mutate(d, p = stata_lasso_p)"),
            "{out:?}"
        );
        assert!(
            out[10].contains("post = TRUE") && out[10].contains("stata_float(stata_lasso_p)"),
            "{out:?}"
        );
        assert!(
            out[11].contains("option residuals not allowed (Stata: r(198))"),
            "{out:?}"
        );
        assert!(
            out[12].contains("stata_list_returned(stata_e, \"e\")"),
            "{out:?}"
        );
        assert!(!out[13].starts_with("NOT"), "{out:?}");
        // without a lasso before them they say why
        let out = r("use d\nregress y x1\nlassoknots\nlassocoef");
        assert!(
            out[2].contains("NOT: lassoknots after a model other than lasso"),
            "{out:?}"
        );
        assert!(
            out[3].contains("NOT: lassocoef after a model other than lasso"),
            "{out:?}"
        );
    }

    #[test]
    fn stored_lasso_results_are_named_in_lassocoef() {
        let out = r("use d
lasso linear y x1 x2, selection(bic)
estimates store b1
lasso linear y x1 x2, selection(plugin)
lassocoef b1 ., display(coef)
lassoinfo b1 b2");
        assert!(
            out[4].contains("stata_lasso_coef(c(\"b1\", \".\"), coef = \"b_standardized\""),
            "{out:?}"
        );
        assert!(
            out[5].contains("NOT: lassoinfo b2: no results are stored under that name"),
            "{out:?}"
        );
    }

    #[test]
    fn a_range_of_predictors_waits_for_the_datas_names() {
        // the run makes x2-x4 the data's names (the probes, run on R); a dry
        // run cannot, and says the line waits for the run, not that it is refused
        use crate::dofile::{dry_run, status, NoFiles, Options, Status};
        let d = dry_run(
            "use d\nlasso linear y (x1) x2-x4, selection(bic)\nlassoknots\n",
            &NoFiles,
            Options::default(),
        );
        let st: Vec<Status> = d
            .translation
            .lines
            .iter()
            .zip(&d.origins)
            .filter(|(l, _)| l.stata.contains("lasso"))
            .map(|(l, o)| status(l, o))
            .collect();
        assert_eq!(st, vec![Status::Later, Status::Later]);
        let out = r("use d\nlasso linear y (x1) x2-x4, selection(bic)");
        assert!(
            out[1]
                .contains("x2-x4: the data's variable names are known only when the do-file runs"),
            "{out:?}"
        );
    }
}
