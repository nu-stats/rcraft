//! `nlsur (y1 = <expression with {b0 = #} parameters>) (y2 = ...) [if] [in] [weight] [, nls fgnls
//! ifgnls nolog level() iterate() eps() delta() noconstant initial() variables() vce()]`: systems
//! of nonlinear equations as Stata 19 runs them (`plan/stata/nlsur.md`).
//!
//! The iteration log is the Gauss-Newton path itself (ported from StatCraft's probe 537, then
//! checked against Stata on made-up data here):
//!
//! - Each step works on the residuals weighted (the square root of each row's weight) and
//!   whitened by Sigma (the identity for NLS), with forward differences d = delta (|b| + delta),
//!   delta = 4e-7 by default, the step halved while it raises the criterion and never stepped
//!   further, stopping once no parameter moved by more than eps() (1e-5, relative to its old size
//!   |b|), at most iterate() (300) steps a stage. A parameter whose derivative column is zero or a
//!   combination of earlier ones' does not move in that step; those still dependent at the end
//!   show as `(constrained)`.
//! - NLS first (the log says "Residual SS", "Scaled RSS" with weights), then FGNLS with
//!   Sigma = R'WR/N from the NLS residuals; `ifgnls` goes round again with Sigma from the last
//!   residuals until the parameters and Sigma each change by at most 1e-5. `nls` ends with a
//!   pass not logged, weighted by the diagonal of the NLS Sigma.
//! - Variances: the inverse of the last step's J'(Sigma^-1 kron W)J; robust and clustered, that
//!   around the scores at the final estimates (clusters times G/(G-1) (N-1)/(N-k)).

use super::model::{Estimates, Fit};
use super::weights::W;
use super::*;

/// The fit, its log, Stata's tables and e().
pub(super) const NLSUR_HELPER: &str = r##"stata_nlsur <- function(d, eqs, init, level = 95, method = "fgnls", nolog = FALSE, noconst = FALSE,
                        iterate = 300L, eps = 1e-5, delta = 4e-7, cmdline = "", show = TRUE,
                        wvar = NULL, wtype = NULL, wexp = NULL, robust = FALSE, cluster = NULL, markout = NULL,
                        evaluator = NULL, touse = NULL, rhs = NULL, funcprog = NULL, hascons = NULL) {
  M <- length(eqs)
  pn <- names(init)
  k <- length(pn)
  clash <- intersect(pn, names(d))
  if (length(clash)) stop("RCraft: nlsur with a parameter named as a variable (", clash[1], ") is not translated yet", call. = FALSE)
  deps <- vapply(eqs, function(q) q$dep, "")
  syms <- if (is.null(evaluator)) unique(unlist(lapply(eqs, function(q) all.vars(body(q$f))))) else character()
  miss <- Filter(function(s) !s %in% c(names(d), pn, ".v") && !exists(s, envir = globalenv()), syms)
  if (length(miss)) stop("variable ", miss[1], " not found (Stata: r(111))", call. = FALSE)
  for (v in deps) if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
  vars <- unique(c(deps, intersect(syms, names(d))))
  for (v in vars) if (is.character(d[[v]])) stop(v, ": string variable, type mismatch (Stata: r(109))", call. = FALSE)
  num <- function(u) { u <- unclass(u); attributes(u) <- NULL; as.numeric(u) }
  for (v in markout) if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
  rows <- which(stats::complete.cases(d[unique(c(deps, markout))]))
  # the evaluator form: the rows its begin line marked (every variable of the varlist, if, in)
  if (!is.null(touse)) {
    rows <- intersect(rows, which(d[[touse]] == 1))
    # its weights checked as Stata checks them (the begin line left out the missing and 0 ones)
    if (!is.null(wtype)) {
      wv <- num(d[[wvar]][rows])
      if (any(wv < 0) && wtype != "i") stop("negative weights encountered (Stata: r(402))", call. = FALSE)
      if (wtype == "f" && any(wv != round(wv))) stop("may not use noninteger frequency weights (Stata: r(401))", call. = FALSE)
    }
  }
  if (!is.null(cluster)) {
    if (!cluster %in% names(d)) stop("variable ", cluster, " not found (Stata: r(111))", call. = FALSE)
    # a row whose cluster is missing is left out, as Stata leaves it
    cv <- d[[cluster]][rows]
    rows <- rows[!(is.na(cv) | (is.character(cv) & cv == ""))]
  }
  n <- length(rows)
  # the weights: frequency and importance weights are taken as they are (N is their sum);
  # analytic and sampling weights are scaled to sum to the number of rows
  w <- rep(1, n)
  if (!is.null(wtype)) {
    w <- num(d[[wvar]][rows])
    if (wtype %in% c("a", "p")) w <- w * n / sum(w)
  }
  N <- if (!is.null(wtype) && wtype %in% c("f", "i")) sum(w) else n
  if (!is.null(wtype) && wtype == "p") robust <- TRUE
  if (identical(wtype, "i") && N != round(N)) stop("RCraft: nlsur with importance weights whose sum is not a whole number is not translated yet", call. = FALSE)
  if (show) {
    if (is.null(wtype) || wtype == "i") {
      cat("(obs = ", stata_tab_count(N), ")\n\n", sep = "")
    } else {
      cat("(sum of wgt is ", stata_fmt(sum(num(d[[wvar]][rows])), "%12.0gc", trim = TRUE), ")\n\n", sep = "")
    }
  }
  if (N * M < k) stop("cannot have fewer observations than parameters (Stata: r(2001))", call. = FALSE)
  sw <- rep(sqrt(w), M)
  base <- lapply(d[rows, vars, drop = FALSE], num)
  evq <- function(q, b) {
    r <- num(eqs[[q]]$f(c(base, as.list(b))))
    if (length(r) == 1L) rep(r, n) else r
  }
  ev <- function(b) unlist(lapply(seq_len(M), function(q) evq(q, b)), use.names = FALSE)
  if (!is.null(evaluator)) {
    # the program, called on all the rows, filled one column per equation
    ev <- function(b) {
      m <- as.matrix(evaluator(stats::setNames(as.numeric(b), pn)))
      if (nrow(m) != nrow(d) || ncol(m) != M) stop("RCraft: nlsur's evaluator did not give one value per row and equation", call. = FALSE)
      as.vector(m[rows, , drop = FALSE])
    }
    evq <- function(q, b) ev(b)[(q - 1L) * n + seq_len(n)]
  }
  y <- unlist(lapply(deps, function(v) base[[v]]), use.names = FALSE)
  b0 <- init
  # a stacked column whitened: each row times the square root of its weight, then by Sigma^-1/2
  wh <- function(v, Linv) {
    v <- v * sw
    if (is.null(Linv)) v else as.vector(matrix(v, n, M) %*% t(Linv))
  }
  jac <- function(b) {
    f0 <- ev(b)
    matrix(vapply(seq_len(k), function(j) {
      bj <- b[[j]]
      dj <- delta * (abs(bj) + delta)
      up <- b
      up[j] <- bj + dj
      dj <- up[[j]] - bj
      (ev(up) - f0) / dj
    }, numeric(length(f0))), length(f0), k)
  }
  # the parameters whose columns in J'J are neither zero nor a combination of earlier ones'
  independent <- function(A) {
    s <- A
    free <- integer()
    for (p in seq_len(k)) {
      dg <- A[p, p]
      pv <- s[p, p]
      if (!is.finite(dg) || dg <= 0 || pv <= 1e-13 * dg) next
      free <- c(free, p)
      rp <- s[p, ]
      for (i in seq_len(k)) {
        if (i == p) next
        s[i, ] <- s[i, ] - (s[i, p] / pv) * rp
      }
    }
    free
  }
  mreldif <- function(a, b) max(abs(a - b) / (abs(b) + 1))
  whj <- function(J, Linv) matrix(vapply(seq_len(k), function(j) wh(J[, j], Linv), numeric(nrow(J))), nrow(J), k)
  gn <- function(b, Linv) {
    crit <- function(b) {
      s <- sum(wh(y - ev(b), Linv)^2)
      if (is.finite(s)) s else Inf
    }
    log <- numeric()
    conv <- FALSE
    for (it in seq_len(max(iterate, 1L))) {
      r <- wh(y - ev(b), Linv)
      cur <- sum(r^2)
      if (!is.finite(cur)) cur <- Inf
      J <- jac(b)
      Jl <- J
      Jw <- whj(J, Linv)
      A <- crossprod(Jw)
      g <- as.vector(crossprod(Jw, r))
      free <- independent(A)
      st <- numeric(k)
      if (length(free)) {
        sol <- tryCatch(solve(A[free, free, drop = FALSE], g[free]), error = function(e) NULL)
        if (is.null(sol) || !all(is.finite(sol))) stop("could not calculate numerical derivatives (Stata: r(481))", call. = FALSE)
        st[free] <- sol
      }
      t <- 1
      nxt <- crit(b + st)
      while (nxt > cur && t > 1e-15) {
        t <- t / 2
        nxt <- crit(b + t * st)
      }
      nb <- b + t * st
      log <- c(log, nxt)
      rel <- abs(nb - b) / abs(b)
      rel[nb == b] <- 0
      ch <- max(rel)
      b <- nb
      if (ch <= eps) {
        conv <- TRUE
        break
      }
    }
    list(b = b, log = log, converged = conv, J = Jl)
  }
  sigma_of <- function(r) crossprod(matrix(r, n, M) * sqrt(w)) / N
  linv <- function(S) {
    L <- tryCatch(chol(S), error = function(e) NULL)
    if (is.null(L) || any(diag(L) <= 0)) stop("RCraft: nlsur where the residuals' covariance matrix is singular is not translated yet", call. = FALSE)
    solve(t(L))
  }
  logs <- function(label, v) {
    for (i in seq_along(v)) {
      cat(sprintf("%-13s %s = %8s\n", paste0("Iteration ", i - 1L, ":"), label, trimws(stata_fmt(v[i], "%9.0g"))))
    }
  }
  calc <- function(what) if (show) cat("Calculating ", what, " estimates", if (nolog) " ..." else ":", "\n", sep = "")
  sci <- function(v) sub("e([+-])0([0-9]{2})$", "e\\1\\2", sprintf("%.2e", v))
  calc("NLS")
  for (q in seq_len(M)) {
    if (any(!is.finite(evq(q, b0)))) {
      stop("could not evaluate equation ", q, ": starting values invalid or some RHS variables have missing values (Stata: r(480))", call. = FALSE)
    }
  }
  nls <- gn(b0, NULL)
  if (show && !nolog) logs(if (is.null(wtype)) "Residual SS" else "Scaled RSS", nls$log)
  b <- nls$b
  lastgn <- nls
  conv <- nls$converged
  Sigma <- sigma_of(y - ev(b))
  Linv <- NULL
  if (method == "nls") {
    # after the stacked fit (the one the log shows), a pass not logged that weights each equation
    # by 1/variance of the stacked fit's residuals; with no parameter shared by two equations it
    # leaves the estimates where they are
    Sigma <- diag(diag(Sigma), nrow = M)
    Linv <- linv(Sigma)
    hid <- gn(b, Linv)
    b <- hid$b
    lastgn <- hid
    conv <- hid$converged
  } else {
    if (show && !nolog) cat("\n")
    calc("FGNLS")
    Linv <- linv(Sigma)
    fg <- gn(b, Linv)
    if (show && !nolog) logs("Scaled RSS", fg$log)
    b <- fg$b
    lastgn <- fg
    conv <- fg$converged
    if (method == "ifgnls") {
      conv <- FALSE
      for (round in 2:100) {
        Sigma <- sigma_of(y - ev(b))
        Linv <- linv(Sigma)
        if (show) {
          if (nolog) cat("FGNLS iteration ", round, " ...\n", sep = "") else cat("\nFGNLS iteration ", round, ":\n", sep = "")
        }
        nx <- gn(b, Linv)
        after <- sigma_of(y - ev(nx$b))
        pch <- max(abs(nx$b - b) / (abs(nx$b) + 1))
        # relative to the new Sigma, as the parameter change is to the new estimates
        cch <- mreldif(as.vector(Sigma), as.vector(after))
        if (show && !nolog) {
          logs("Scaled RSS", nx$log)
          cat("Parameter change         = ", sci(pch), "\n", sep = "")
          cat("Covariance matrix change = ", sci(cch), "\n", sep = "")
        }
        b <- nx$b
        lastgn <- nx
        if (pch <= 1e-5 && cch <= 1e-5) {
          conv <- TRUE
          break
        }
      }
    }
  }
  # the variance: the inverse of the last Gauss-Newton step's J'(Sigma^-1 kron W)J, with J taken
  # at the start of that step and the Sigma it used (diagonal for nls); robust and clustered, that
  # matrix around the sum of the scores w_i J_i' Sigma^-1 r_i, with J and r at the final estimates
  Jw <- whj(lastgn$J, Linv)
  A <- crossprod(Jw)
  free <- independent(A)
  fr <- function(m) m[free, free, drop = FALSE]
  V <- matrix(0, k, k)
  fit <- ev(b)
  res <- y - fit
  Sf <- sigma_of(res)
  nclust <- NULL
  if (length(free)) {
    Ai <- solve(fr(A))
    small <- if (robust) {
      Jf <- jac(b)
      Si <- crossprod(Linv)
      sc <- matrix(0, n, k)
      for (q in seq_len(M)) {
        ix <- (q - 1L) * n + seq_len(n)
        rq <- 0
        for (l in seq_len(M)) rq <- rq + Si[q, l] * res[(l - 1L) * n + seq_len(n)]
        sc <- sc + Jf[ix, , drop = FALSE] * (w * rq)
      }
      sc <- sc[, free, drop = FALSE]
      if (is.null(cluster)) {
        # frequency weights are copies of the row: each copy's score counts once
        if (identical(wtype, "f")) sc <- sc / sqrt(w)
        Ai %*% crossprod(sc) %*% Ai
      } else {
        g <- d[[cluster]][rows]
        G <- length(unique(g))
        if (G < 2L) stop("RCraft: nlsur, vce(cluster) with one cluster is not translated yet", call. = FALSE)
        nclust <- G
        Ai %*% crossprod(rowsum(sc, match(g, unique(g)))) %*% Ai * (G / (G - 1)) * ((N - 1) / (N - k))
      }
    } else {
      Ai
    }
    V[free, free] <- small
  }
  ll <- -N / 2 * (M * (1 + log(2 * pi)) + as.numeric(determinant(Sf, logarithm = TRUE)$modulus))
  # each equation's constant: the first parameter whose derivative is the same, and not zero, on
  # every row (at the final estimates)
  const <- rep(NA_character_, M)
  # the evaluator form has constants only where hasconstants() names them: each equation's is the
  # named parameter whose derivative is the same, and not zero, on its rows
  if (!noconst && (is.null(evaluator) || !is.null(hascons))) {
    Jf <- jac(b)
    for (q in seq_len(M)) {
      ix <- (q - 1L) * n + seq_len(n)
      for (p in if (is.null(evaluator)) eqs[[q]]$params else hascons) {
        col <- Jf[ix, match(p, pn)]
        if (is.finite(col[1]) && col[1] != 0 && max(abs(col - col[1])) <= 1e-6 * abs(col[1])) {
          const[q] <- p
          break
        }
      }
    }
  }
  if (!is.null(hascons) && anyNA(const)) stop("RCraft: nlsur, hasconstants() where an equation has none of the named constants is not translated yet", call. = FALSE)
  stat <- lapply(seq_len(M), function(q) {
    ix <- (q - 1L) * n + seq_len(n)
    rss <- sum(w * res[ix]^2)
    tss <- if (is.na(const[q])) sum(w * y[ix]^2) else sum(w * (y[ix] - sum(w * y[ix]) / sum(w))^2)
    mss <- tss - rss
    if (abs(mss) <= 1e-12 * tss) mss <- 0
    list(rss = rss, mss = mss, r2 = if (tss > 0) mss / tss else 0, rmse = sqrt(rss / N), k = length(eqs[[q]]$params))
  })
  nm <- paste0("/", pn)
  se <- sqrt(pmax(diag(V), 0))
  if (show) {
    pad <- stata_tab_pad
    cat("\n", if (method == "nls") "Nonlinear least-squares regression " else "FGNLS regression ", "\n", sep = "")
    cat(strrep("-", 71), "\n", sep = "")
    cat("       Equation |        Obs   Parms       RMSE      R-sq     Constant\n")
    cat("----------------+------------------------------------------------------\n")
    for (q in seq_len(M)) {
      s <- stat[[q]]
      txt <- paste0(sprintf("%.4f", s$r2), if (is.na(const[q])) "*" else "")
      cw <- max(12L, 19L - nchar(txt))
      cat(sprintf("%2d%13s |%11s%8s%11s    %s%s\n", q, stata_tab_abbrev(deps[q]), stata_tab_count(N),
                  if (is.null(evaluator)) as.character(s$k) else ".",
                  trimws(stata_fmt(s$rmse, "%9.0g")), txt,
                  formatC(if (is.na(const[q])) "(none)" else const[q], width = cw)))
    }
    cat(strrep("-", 71), "\n", sep = "")
    if (any(is.na(const))) cat("* Uncentered R-sq\n")
    cat("\n")
    stata_ls_table(nm, b, se, Inf, level, if (length(unique(deps)) == 1L) deps[1] else "",
                   constr = !(seq_len(k) %in% free), robust = robust, clusters = nclust, clustvar = cluster)
  }
  # a run that ran out of steps: Stata prints the table, then ends with r(430) and no message
  # a run that ran out of steps keeps its results; the caller stops after posting them
  e <- list(N = N)
  for (q in seq_len(M)) {
    s <- stat[[q]]
    e[[paste0("rss_", q)]] <- s$rss
    e[[paste0("mss_", q)]] <- s$mss
    e[[paste0("r2_", q)]] <- s$r2
    e[[paste0("rmse_", q)]] <- s$rmse
    if (is.null(evaluator)) e[[paste0("k_", q)]] <- s$k
  }
  e$ll <- ll
  e$n_eq <- M
  e$k <- k
  e$k_eq_model <- 0
  if (is.null(nclust)) {
    e$converged <- as.numeric(conv)
    e$rank <- length(free)
    e$k_eq <- M
  } else {
    # with clusters Stata keeps k_eq before rank
    e$N_clust <- nclust
    e$converged <- as.numeric(conv)
    e$k_eq <- M
    e$rank <- length(free)
  }
  e$cmdline <- cmdline
  e$cmd <- "nlsur"
  e$predict <- "nlsur_p"
  e$estat_cmd <- "nlsur_estat"
  if (!is.null(evaluator)) {
    e$rhs <- rhs
    e$funcprog <- funcprog
  }
  if (!is.null(wtype)) {
    e$wexp <- wexp
    e$wtype <- c(f = "fweight", a = "aweight", p = "pweight", i = "iweight")[[wtype]]
  }
  if (!is.null(nclust)) e$clustvar <- cluster
  if (robust) e$vcetype <- "Robust"
  e$vce <- if (!is.null(nclust)) "cluster" else if (robust) "robust" else "gnr"
  e$method <- method
  e$constants <- if (noconst) " . ." else paste(ifelse(is.na(const), ".", const), collapse = " ")
  e$type <- if (is.null(evaluator)) "1" else "3"
  e$params <- paste(pn, collapse = " ")
  e$depvar <- paste(deps, collapse = " ")
  for (q in rev(seq_len(M))) {
    if (is.null(evaluator)) e[[paste0("params_", q)]] <- paste(eqs[[q]]$params, collapse = " ")
    e[[paste0("depvar_", q)]] <- deps[q]
  }
  if (is.null(evaluator)) for (q in rev(seq_len(M))) e[[paste0("sexp_", q)]] <- eqs[[q]]$sexp
  e$properties <- "b V"
  cn <- paste0(pn, ":_cons")
  e$b <- matrix(b, 1, k, dimnames = list("y1", cn))
  dimnames(V) <- list(cn, cn)
  e$V <- V
  e$init <- matrix(b0, 1, k, dimnames = list("r1", pn))
  SigmaOut <- if (method == "nls") diag(diag(Sigma), nrow = M) else if (method == "ifgnls") after else Sigma
  dimnames(SigmaOut) <- list(deps, deps)
  e$Sigma <- SigmaOut
  attr(e, "stata_rows") <- rows
  attr(e, "stata_b") <- stats::setNames(as.vector(b), cn)
  attr(e, "stata_V") <- V
  attr(e, "stata_df") <- Inf
  attr(e, "stata_live_b") <- TRUE
  attr(e, "stata_evaluator") <- evaluator
  attr(e, "stata_wvar") <- wvar
  attr(e, "stata_wtype") <- wtype
  attr(e, "stata_unconverged") <- !conv
  attr(e, "stata_eqs") <- eqs
  attr(e, "stata_pars") <- b
  e
}
# estat ic: N, the log likelihood and the free parameters, as Stata counts them
stata_nlsur_ic <- function(e) {
  n <- e$N
  ll <- e$ll
  k <- e$rank
  tab <- data.frame(N = n, "ll(model)" = ll, df = k, AIC = -2 * ll + 2 * k,
                    BIC = -2 * ll + log(n) * k, check.names = FALSE)
  rownames(tab) <- "."
  print(tab, digits = 8)
  invisible(tab)
}
# margins after nlsur: the mean fitted value of equation k over the rows of e(sample) that sel keeps
# (weighted as the fit was), its delta-method variance from e(V); Stata's layout
stata_nlsur_margins <- function(e, d, k, sel, level = 95, ptext = "predict()", popts = "", over = NULL,
                                post = FALSE, cmdline = "margins", dname = NULL, show = TRUE) {
  eqs <- attr(e, "stata_eqs")
  if (is.na(k) || k < 1L || k > length(eqs)) stop("equation number out of range (Stata: r(303))", call. = FALSE)
  num <- function(u) { u <- unclass(u); attributes(u) <- NULL; as.numeric(u) }
  keep <- which(stata_esample & !is.na(sel) & sel)
  if (!length(keep)) stop("no observations (Stata: r(2000))", call. = FALSE)
  # the groups of over(): whole numbers only (Stata 19's words), each combination present, in order
  grp <- rep(1L, length(keep))
  labs <- "_cons"
  if (length(over)) {
    vals <- lapply(over, function(v) num(d[[v]][keep]))
    for (i in seq_along(over)) {
      lab <- attr(d[[over[i]]], "labels", exact = TRUE)
      if (!is.null(lab)) stop("RCraft: margins, over() with a labelled variable after nlsur is not translated yet", call. = FALSE)
      if (anyNA(vals[[i]])) stop("RCraft: margins, over() with missing values after nlsur is not translated yet", call. = FALSE)
      if (any(vals[[i]] != round(vals[[i]]))) stop("only integer values are allowed in over() and within() variables (Stata: r(198))", call. = FALSE)
      if (any(vals[[i]] < 0)) stop("RCraft: margins, over() with negative values after nlsur is not translated yet", call. = FALSE)
    }
    key <- do.call(paste, c(lapply(vals, function(v) formatC(v, width = 20, format = "d", flag = "0")), sep = "\r"))
    combos <- sort(unique(key))
    grp <- match(key, combos)
    first <- match(seq_along(combos), grp)
    lv <- lapply(vals, function(v) v[first])
    labs <- vapply(seq_along(combos), function(j) paste(vapply(seq_along(over), function(i) paste0(format(lv[[i]][j], scientific = FALSE), ".", over[i]), ""), collapse = "#"), "")
    shown <- vapply(seq_along(combos), function(j) paste(vapply(seq_along(over), function(i) format(lv[[i]][j], scientific = FALSE), ""), collapse = " "), "")
    if (any(nchar(shown) > 11L)) stop("RCraft: margins, over() with levels wider than the table's column after nlsur is not translated yet", call. = FALSE)
  }
  G <- length(labs)
  wv <- attr(e, "stata_wvar")
  w <- if (is.null(wv)) rep(1, length(keep)) else num(d[[wv]][keep])
  b0 <- attr(e, "stata_pars")
  fitted <- if (is.null(attr(e, "stata_evaluator"))) {
    dd <- lapply(as.list(d[keep, , drop = FALSE]), function(u) if (is.character(u)) u else num(u))
    function(bv) {
      r <- num(eqs[[k]]$f(c(dd, as.list(bv))))
      if (length(r) == 1L) rep(r, length(keep)) else r
    }
  } else {
    function(bv) stata_nlsur_evalfit(e, dname, d, bv)[keep, k]
  }
  at <- function(bv) {
    r <- fitted(bv)
    vapply(seq_len(G), function(j) { s <- grp == j; sum(w[s] * r[s]) / sum(w[s]) }, 0)
  }
  m <- at(b0)
  if (!all(is.finite(m))) stop("prediction is not a number on these rows (Stata: r(498))", call. = FALSE)
  J <- matrix(vapply(seq_along(b0), function(j) {
    h <- 1e-5 * (abs(b0[[j]]) + 1e-3)
    up <- b0
    dn <- b0
    up[j] <- up[j] + h
    dn[j] <- dn[j] - h
    (at(up) - at(dn)) / (2 * h)
  }, numeric(G)), G, length(b0))
  V <- J %*% attr(e, "stata_V") %*% t(J)
  dimnames(V) <- list(labs, labs)
  fw <- identical(attr(e, "stata_wtype"), "f") || identical(attr(e, "stata_wtype"), "i")
  Ns <- vapply(seq_len(G), function(j) if (fw) sum(w[grp == j]) else sum(grp == j), 0)
  N <- sum(Ns)
  label <- paste0("Equation ", k, " fitted values")
  if (show) {
    pad <- stata_tab_pad
    nobs <- stata_tab_count(N)
    cat("\n", formatC("Predictive margins", width = -(62 - nchar(nobs)), flag = "-"), "Number of obs = ", nobs, "\n", sep = "")
    cat("Model VCE: ", if (identical(e$vcetype, "Robust")) "Robust" else "GNR", "\n\n", sep = "")
    cat("Expression: ", label, ", ", ptext, "\n", sep = "")
    if (length(over)) cat("Over:       ", paste(over, collapse = " "), "\n", sep = "")
    cat("\n")
    q <- stats::qnorm((1 + level / 100) / 2)
    cat(strrep("-", 78), "\n", sep = "")
    cat("             |            Delta-method\n")
    cat("             |     Margin   std. err.      z    P>|z|     [", format(level), "% conf. interval]\n", sep = "")
    cat(strrep("-", 13), "+", strrep("-", 64), "\n", sep = "")
    if (length(over)) cat(pad(stata_tab_abbrev(paste(over, collapse = "#"), 12), 12), " |\n", sep = "")
    for (j in seq_len(G)) {
      se <- sqrt(V[j, j])
      z <- m[j] / se
      lab <- if (length(over)) paste0(pad(shown[j], 11), "  |") else paste0(pad("_cons", 12), " |")
      cat(lab, pad(stata_tab_g(m[j]), 11), pad(stata_tab_g(se), 11), pad(sprintf("%.2f", z), 9),
          pad(sprintf("%.3f", 2 * stats::pnorm(-abs(z))), 8), pad(stata_tab_g(m[j] - q * se), 13), pad(stata_tab_g(m[j] + q * se), 12), "\n", sep = "")
    }
    cat(strrep("-", 78), "\n", sep = "")
  }
  bm <- matrix(m, 1, G, dimnames = list("r1", labs))
  if (!post) return(list(N = N, level = level, cmd = "margins", est_cmd = "nlsur", est_cmdline = e$cmdline, b = bm, V = V))
  # margins, post: Stata 19's e() after it
  pn <- colnames(e$b)
  out <- list(k_at = 0, k_by = G, k_predict = 1, k_margins = 0, N = N, cmd = "margins", title = "Predictive margins",
              model_vce = e$vce)
  if (!is.null(e$vcetype)) out$model_vcetype <- e$vcetype
  out$vce <- "delta"
  out$vcetype <- "Delta-method"
  out$expression <- ptext
  out$predict1_label <- label
  if (nzchar(popts)) out$predict1_opts <- popts
  if (length(over)) {
    out$by <- paste(over, collapse = " ")
    out$over <- paste(over, collapse = " ")
    for (j in seq_len(G)) out[[paste0("by", j)]] <- labs[j]
  }
  out$emptycells <- "strict"
  out$est_cmd <- "nlsur"
  out$est_cmdline <- e$cmdline
  out$cmdline <- cmdline
  out$properties <- "b V"
  out$b <- matrix(m, 1, G, dimnames = list("y1", labs))
  out$V <- V
  out$`_N` <- matrix(Ns, 1, G, dimnames = list("r1", labs))
  out$error <- matrix(0, 1, G, dimnames = list("r1", labs))
  out$Jacobian <- matrix(J, G, length(b0), dimnames = list(labs, pn))
  attr(out, "stata_b") <- stats::setNames(m, labs)
  out
}
# the evaluator form's fitted values (one column per equation) at b on the data d: the program's
# function run on d, with the copies of the dependent variables and every row marked
stata_nlsur_evalfit <- function(e, dname, d, b) {
  ev <- attr(e, "stata_evaluator")
  deps <- vapply(attr(e, "stata_eqs"), function(q) q$dep, "")
  num <- function(u) { u <- unclass(u); attributes(u) <- NULL; as.numeric(u) }
  for (q in seq_along(deps)) d[[paste0("__nlsur_y", q)]] <- num(d[[deps[q]]])
  d[["__nlsur_touse"]] <- 1
  old <- get(dname, envir = globalenv())
  on.exit(assign(dname, old, envir = globalenv()))
  assign(dname, d, envir = globalenv())
  m <- as.matrix(ev(stats::setNames(as.numeric(b), names(attr(e, "stata_pars")))))
  m[!is.finite(m)] <- NA_real_
  m
}
# estat vce after nlsur: Stata's table with each parameter its own equation (a:_cons)
stata_nlsur_vce <- function(e, correlation = FALSE) {
  V <- as.matrix(e$V)
  eq <- sub(":.*$", "", colnames(V))
  k <- length(eq)
  M <- if (correlation) {
    s <- sqrt(diag(V))
    V / outer(s, s)
  } else V
  f <- if (correlation) "%8.4f" else "%10.0g"
  fw <- if (correlation) 8L else 10L
  ls <- if (exists("stata_linesize")) stata_linesize else 79
  if (13L + k * (fw + 3L) > ls) stop("RCraft: estat vce after nlsur with more parameters than fit on one line is not translated yet", call. = FALSE)
  cell <- function(s, left = FALSE) paste0(" ", formatC(s, width = if (left) -fw else fw), " ")
  last_short <- function(cells) { cells[k] <- substring(cells[k], 1L, nchar(cells[k]) - 1L); cells }
  cat("\n", if (correlation) "Correlation" else "Covariance", " matrix of coefficients of nlsur model\n\n", sep = "")
  cat(strrep(" ", 13), "|", paste(cell(stata_tab_abbrev(eq, fw), TRUE), collapse = "|"), "\n", sep = "")
  cat(formatC("e(V)", width = 12), " |", paste(rep(cell("_cons"), k), collapse = "|"), "\n", sep = "")
  sep_line <- paste0(strrep("-", 13), "+", paste(last_short(rep(strrep("-", fw + 2L), k)), collapse = "+"), "\n")
  for (i in seq_len(k)) {
    cat(sep_line)
    cat(formatC(stata_tab_abbrev(eq[i], 12), width = -13), "|", paste(last_short(rep(strrep(" ", fw + 2L), k)), collapse = "|"), "\n", sep = "")
    cells <- vapply(seq_len(k), function(j) if (j <= i) cell(stata_fmt(M[i, j], f, trim = TRUE)) else strrep(" ", fw + 2L), "")
    cat(formatC("_cons", width = 12), " |", paste(cells, collapse = "|"), "\n", sep = "")
  }
  invisible(NULL)
}
# estat summarize after nlsur: the dependent variables over e(sample)
stata_nlsur_summarize <- function(e, d) {
  deps <- unique(vapply(attr(e, "stata_eqs"), function(q) q$dep, ""))
  x <- d[stata_esample, , drop = FALSE]
  for (v in deps) {
    l <- attr(x[[v]], "label", exact = TRUE)
    if (!is.null(l) && nzchar(l)) stop("RCraft: estat summarize after nlsur with labelled variables is not translated yet", call. = FALSE)
  }
  num <- function(u) { u <- unclass(u); attributes(u) <- NULL; as.numeric(u) }
  g <- function(v, wd) formatC(stata_fmt(v, "%9.0g", trim = TRUE), width = wd)
  cat("\n  Estimation sample nlsur", strrep(" ", 18), "Number of obs = ", formatC(stata_tab_count(nrow(x)), width = 10), "\n\n", sep = "")
  cat("  ", strrep("-", 67), "\n", sep = "")
  cat("      Variable |         Mean      Std. dev.         Min          Max\n")
  cat("  ", strrep("-", 13), "+", strrep("-", 53), "\n", sep = "")
  for (v in deps) {
    u <- num(x[[v]])
    cat("  ", formatC(stata_tab_abbrev(v), width = 12), " |", g(mean(u), 13), g(stats::sd(u), 14), g(min(u), 13), g(max(u), 13), "\n", sep = "")
  }
  cat("  ", strrep("-", 67), "\n", sep = "")
  invisible(NULL)
}
# the fitted values (or residuals) of equation k on every row of d
stata_nlsur_pred <- function(e, d, k, resid = FALSE, dname = NULL) {
  eqs <- attr(e, "stata_eqs")
  if (is.na(k) || k < 1L || k > length(eqs)) stop("equation number out of range (Stata: r(303))", call. = FALSE)
  num <- function(u) { u <- unclass(u); attributes(u) <- NULL; as.numeric(u) }
  r <- if (is.null(attr(e, "stata_evaluator"))) {
    v <- c(lapply(as.list(d), function(u) if (is.character(u)) u else num(u)), as.list(attr(e, "stata_pars")))
    r <- num(eqs[[k]]$f(v))
    if (length(r) == 1L) rep(r, nrow(d)) else r
  } else {
    # the evaluator's program, run on every row
    stata_nlsur_evalfit(e, dname, d, attr(e, "stata_pars"))[, k]
  }
  r[!is.finite(r)] <- NA_real_
  if (resid) num(d[[eqs[[k]]$dep]]) - r else r
}"##;

/// The `(...)` groups of an nlsur command, each without its parentheses.
fn equations(main: &str) -> Result<Vec<String>, Untranslated> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut cur = String::new();
    let mut seen_close = true;
    for ch in main.chars() {
        match ch {
            '(' => {
                if depth == 0 {
                    if !seen_close {
                        return Err(no("nlsur: expected an equation in parentheses"));
                    }
                    cur.clear();
                } else {
                    cur.push(ch);
                }
                depth += 1;
            }
            ')' => {
                if depth == 0 {
                    return Err(no("nlsur: an unmatched )"));
                }
                depth -= 1;
                if depth == 0 {
                    out.push(cur.clone());
                    seen_close = true;
                } else {
                    cur.push(ch);
                }
            }
            c if depth > 0 => cur.push(c),
            c if c.is_whitespace() => {}
            _ => return Err(no(
                "nlsur takes each equation in parentheses, (y1 = <expression>) (y2 = <expression>)",
            )),
        }
    }
    if depth != 0 {
        return Err(no("nlsur: an equation's parenthesis is not closed"));
    }
    Ok(out)
}

/// One parameter of an equation's expression.
struct Eq {
    dep: String,
    /// The expression as R, with the parameters as plain names.
    expr: String,
    /// The text `e(sexp_#)` holds.
    stored: String,
    params: Vec<String>,
}

/// nlsur's function-evaluator form (`nlsur name @ varlist, parameters() neq()`), from its begin line
/// to its fit (`dofile::nlsur_eval` writes the lines).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct NlEval {
    /// The program, `nlsur<name>`.
    pub(super) prog: String,
    pub(super) params: Vec<String>,
    pub(super) deps: Vec<String>,
    pub(super) rhs: Vec<String>,
}

impl Session {
    /// `_rcraft_nlsur_begin name @ varlist [if] [in], parameters() neq() ...`: the rows the fit
    /// uses (every variable of the varlist present, and if/in) as `__nlsur_touse`, and the matrix
    /// `__nlsur_at` the program reads.
    pub(super) fn nlsur_eval_begin(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.nlsur_eval = None;
        let main = c.parts.main.trim();
        let (name, vars) = main
            .split_once('@')
            .ok_or_else(|| no("nlsur: the evaluator form is nlsur name @ varlist"))?;
        let weight = self.weight_of(c, &[W::F, W::A, W::P, W::I])?;
        // parameters(names), or nparameters(#): the parameters b1, b2, ... (Stata 19)
        let params: Vec<String> = match (opts.arg("parameters", 5), opts.arg("nparameters", 4)) {
            (Some(p), None) => p.split_whitespace().map(str::to_string).collect(),
            (None, Some(n)) => {
                let n: usize = n
                    .trim()
                    .parse()
                    .map_err(|_| no(format!("nlsur, nparameters({n}) needs a whole number")))?;
                (1..=n).map(|i| format!("b{i}")).collect()
            }
            _ => {
                return Err(no(
                    "nlsur's evaluator form needs parameters() or nparameters() (one of them)",
                ))
            }
        };
        if params.is_empty() || params.iter().any(|p| !is_name(p)) {
            return Err(no(
                "nlsur, parameters(): plain parameter names are translated",
            ));
        }
        if let Some(h) = opts.arg("hasconstants", 4) {
            if h.split_whitespace().any(|p| !params.iter().any(|q| q == p)) {
                return Err(no(format!(
                    "nlsur, hasconstants({}) names something that is not a parameter",
                    h.trim()
                )));
            }
        }
        let neq: usize = opts
            .arg("neq", 3)
            .and_then(|n| n.trim().parse().ok())
            .ok_or_else(|| no("nlsur's evaluator form needs neq(#)"))?;
        let words: Vec<String> = vars.split_whitespace().map(str::to_string).collect();
        if words.len() <= neq || neq == 0 {
            return Err(no(
                "nlsur name @ varlist: the dependent variables then the others",
            ));
        }
        for w in &words {
            if !is_name(w) || !self.kinds.contains_key(w) {
                return Err(no(format!(
                    "nlsur name @ varlist: only variables named in full are translated ({w})"
                )));
            }
        }
        let d = self.data()?;
        let mut cond: Vec<String> = words
            .iter()
            .map(|w| format!("!is.na({})", r_name(w)))
            .collect();
        // a weight missing or 0 leaves the row out, as for every Stata weight
        if let Some((_, v)) = &weight {
            cond.push(format!("!is.na({0}) & {0} != 0", r_name(v)));
        }
        if let Some(e) = self.condition(c, out)? {
            cond.push(format!("({e})"));
        }
        out.package("dplyr");
        out.line(format!(
            "{d} <- dplyr::mutate({d}, {} = as.numeric({}))",
            r_name("__nlsur_touse"),
            cond.join(" & ")
        ));
        self.kinds.insert("__nlsur_touse".into(), Kind::Number);
        self.storage.insert("__nlsur_touse".into(), Storage::Double);
        // the starting values: the program's lines run once at them before they become the
        // function (Stata calls it there first too)
        let mut start: Vec<f64> = vec![0.0; params.len()];
        if let Some(a) = opts.arg("initial", 4) {
            let words: Vec<&str> = a.split_whitespace().collect();
            if words.len() % 2 != 0 {
                return Err(no(
                    "nlsur: initial() takes pairs of a parameter and a number",
                ));
            }
            for pair in words.chunks(2) {
                let (Some(n), Some(v)) = (pair.first(), pair.get(1)) else {
                    continue;
                };
                let v: f64 = v
                    .parse()
                    .map_err(|_| no(format!("nlsur: the starting value {v} is not a number")))?;
                let i = params.iter().position(|p| p == n).ok_or_else(|| {
                    no(format!(
                        "nlsur: initial() names {n}, which is not a parameter"
                    ))
                })?;
                if let Some(s) = start.get_mut(i) {
                    *s = v;
                }
            }
        }
        out.line(format!(
            "stata_nlsur_b <- c({})",
            start
                .iter()
                .map(|v| r_number(*v))
                .collect::<Vec<_>>()
                .join(", ")
        ));
        self.helpers.insert("stata_matrix");
        self.user_mats.insert("__nlsur_at".into());
        self.nlsur_eval = Some(NlEval {
            prog: format!("nlsur{}", name.trim()),
            params,
            deps: words.get(..neq).unwrap_or_default().to_vec(),
            rhs: words.get(neq..).unwrap_or_default().to_vec(),
        });
        Ok(())
    }

    /// `_rcraft_nlsur_at`: inside the evaluator's function, at() from its argument.
    pub(super) fn nlsur_eval_at(&mut self, out: &mut Out) -> Result<(), Untranslated> {
        let ev = self
            .nlsur_eval
            .as_ref()
            .ok_or_else(|| no("nlsur: the evaluator's begin line is not known"))?;
        out.line(format!(
            "stata_mat[[\"__nlsur_at\"]] <- matrix(stata_nlsur_b, 1, {}, dimnames = list(\"r1\", c({})))",
            ev.params.len(),
            ev.params
                .iter()
                .map(|p| r_text(p))
                .collect::<Vec<_>>()
                .join(", ")
        ));
        Ok(())
    }

    /// `_rcraft_nlsur_collect y1 y2`: what the program left in the copies, the function's value.
    pub(super) fn nlsur_eval_collect(
        &mut self,
        c: &Command,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let d = self.data()?;
        let cols: Vec<String> = c
            .parts
            .main
            .split_whitespace()
            .map(|v| format!("as.numeric(unclass({d}[[{}]]))", r_text(v)))
            .collect();
        out.line(format!("stata_nlsur_out <- cbind({})", cols.join(", ")));
        Ok(())
    }

    /// The command whose results are active, when it is `nlsur`.
    pub(super) fn nlsur_active(&self) -> bool {
        matches!(
            &self.e_ret,
            super::returned::Returned::Program { cmd, .. } if cmd == "nlsur"
        )
    }

    /// `nlsur (y1 = <sexp>) (y2 = <sexp>) ... [if] [in] [, options]`.
    pub(super) fn nlsur(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "nlsur")?;
        // the function-evaluator form: its begin line left what the program is
        let eval = if c.name == "_rcraft_nlsur_fit" {
            Some(
                self.nlsur_eval
                    .take()
                    .ok_or_else(|| no("nlsur: the evaluator's begin line is not known"))?,
            )
        } else {
            None
        };
        if let Some(o) = opts.other(&[
            ("nls", 3),
            ("fgnls", 5),
            ("ifgnls", 6),
            ("nolog", 5),
            ("level", 3),
            ("iterate", 4),
            ("eps", 3),
            ("noconstant", 3),
            ("initial", 4),
            ("vce", 3),
            ("delta", 5),
            ("variables", 9),
            ("parameters", 5),
            ("neq", 3),
            ("nparameters", 4),
            ("hasconstants", 4),
        ]) {
            return Err(no(format!(
                "nlsur with the option {o} is not translated yet"
            )));
        }
        // parameters() belongs to the function-evaluator form (Stata's own words)
        if let (Some(p), None) = (opts.arg("parameters", 5), &eval) {
            super::sureg::eq_stop(
                out,
                &format!("parameters({}) not allowed (Stata: r(198))", p.trim()),
            );
            return Ok(());
        }
        if eval.is_none()
            && (opts.has("neq", 3) || opts.has("nparameters", 4) || opts.has("hasconstants", 4))
        {
            return Err(no("nlsur with neq(), nparameters() or hasconstants() outside the evaluator form is not translated yet"));
        }
        let weight = self.weight_of(c, &[W::F, W::A, W::P, W::I])?;
        // vce(gnr) is the default; vce(robust) and vce(cluster v) as Stata takes them
        let (robust, cluster) = match opts.arg("vce", 3).map(|v| v.trim().to_string()) {
            Some(v) if v == "gnr" => (false, None),
            Some(v) if v == "ols" || v == "oim" => {
                return Err(no(format!("nlsur, vce({v}) is not translated yet")))
            }
            Some(_) => super::model::vce(opts)?,
            None => (false, None),
        };
        if matches!(weight, Some((W::I, _))) && robust {
            super::sureg::eq_stop(
                out,
                "robust variances not available with iweights (Stata: r(101))",
            );
            return Ok(());
        }
        if let Some(v) = &cluster {
            if !self.kinds.contains_key(v) {
                return Err(no(format!(
                    "nlsur, vce(cluster {v}): only a variable named in full is translated"
                )));
            }
        }
        let kinds_given = [
            opts.has("nls", 3),
            opts.has("fgnls", 5),
            opts.has("ifgnls", 6),
        ];
        if kinds_given.iter().filter(|b| **b).count() > 1 {
            super::sureg::eq_stop(
                out,
                "cannot specify more than one estimator (Stata: r(198))",
            );
            return Ok(());
        }
        let method = if opts.has("ifgnls", 6) {
            "ifgnls"
        } else if opts.has("nls", 3) {
            "nls"
        } else {
            "fgnls"
        };
        let texts = match &eval {
            Some(_) => Vec::new(),
            None => {
                let t = equations(c.parts.main.trim())?;
                if t.is_empty() {
                    return Err(no("nlsur needs at least one equation in parentheses"));
                }
                t
            }
        };
        // the parameters in order of first appearance, with their starting values (the
        // evaluator form's are parameters()'s, starting at 0)
        let mut pars: Vec<(String, f64)> = eval
            .as_ref()
            .map(|e| e.params.iter().map(|p| (p.clone(), 0.0)).collect())
            .unwrap_or_default();
        let mut eqs: Vec<Eq> = Vec::new();
        let mut parsed: Vec<(String, String, String, Vec<String>)> = Vec::new();
        // the linear combinations {name: varlist} met so far, with their variables
        let mut lcs: Vec<(String, Vec<String>)> = Vec::new();
        for t in &texts {
            let Some((lhs, rhs)) = t.split_once('=') else {
                return Err(no("nlsur needs (depvar = <expression>)"));
            };
            let dep = lhs.trim();
            if !is_name(dep) {
                return Err(no(format!(
                    "nlsur with {dep} on the left is not translated yet (one variable name is)"
                )));
            }
            let mut text = String::new();
            let mut stored = String::new();
            let mut used: Vec<String> = Vec::new();
            let mut rest = rhs;
            while let Some(open) = rest.find('{') {
                let before = rest.get(..open).unwrap_or("");
                text.push_str(before);
                stored.push_str(before);
                let tail = rest.get(open..).unwrap_or("");
                let close = tail
                    .find('}')
                    .ok_or_else(|| no("nlsur: a { without its }"))?;
                let body = tail.get(1..close).unwrap_or("").trim();
                // {xb: x1 x2}: the parameters xb_x1 and xb_x2 (starting at 0), as
                // ({xb_x1} *x1 + {xb_x2} *x2)
                if let Some((lc, vars)) = body.split_once(':') {
                    let lc = lc.trim();
                    let vars: Vec<&str> = vars.split_whitespace().collect();
                    if !is_name(lc) || vars.is_empty() {
                        return Err(no(format!(
                            "nlsur with {{{body}}} is not translated yet (a linear combination {{name: varlist}} is)"
                        )));
                    }
                    // written again with the same variables, it is the same parameters (Stata 19)
                    let again = match lcs.iter().find(|(n, _)| n == lc) {
                        Some((_, vs)) if vs.iter().map(String::as_str).eq(vars.iter().copied()) => {
                            true
                        }
                        Some(_) => {
                            return Err(no(format!(
                                "nlsur with the linear combination {lc} over other variables is not translated yet"
                            )))
                        }
                        None => false,
                    };
                    if !again {
                        lcs.push((lc.to_string(), vars.iter().map(|v| v.to_string()).collect()));
                    }
                    let mut r_terms = Vec::new();
                    let mut s_terms = Vec::new();
                    for v in &vars {
                        if !is_name(v) || !self.kinds.contains_key(*v) {
                            return Err(no(format!(
                                "nlsur, {{{body}}}: only variables named in full are translated"
                            )));
                        }
                        let p = format!("{lc}_{v}");
                        if !again {
                            if pars.iter().any(|(n, _)| *n == p) {
                                return Err(no(format!(
                                    "nlsur: the linear combination {lc} makes {p}, which is already a parameter"
                                )));
                            }
                            pars.push((p.clone(), 0.0));
                        }
                        if !used.contains(&p) {
                            used.push(p.clone());
                        }
                        r_terms.push(format!("{p}*{v}"));
                        s_terms.push(format!("{{{p}}} *{v}"));
                    }
                    text.push_str(&format!("({})", r_terms.join(" + ")));
                    stored.push_str(&format!(" ({}) ", s_terms.join(" + ")));
                    rest = tail.get(close + 1..).unwrap_or("");
                    continue;
                }
                let (name, init) = match body.split_once('=') {
                    Some((n, v)) => (n.trim(), Some(v.trim())),
                    None => (body, None),
                };
                if !is_name(name) {
                    return Err(no(format!("nlsur: {{{body}}} is not a parameter")));
                }
                let start = match init {
                    Some(v) => Some(v.parse::<f64>().map_err(|_| {
                        no(format!("nlsur: the starting value {v} is not a number"))
                    })?),
                    None => None,
                };
                match pars.iter_mut().find(|(n, _)| n == name) {
                    Some(p) => {
                        if let Some(s) = start {
                            p.1 = s;
                        }
                    }
                    None => pars.push((name.to_string(), start.unwrap_or(0.0))),
                }
                if !used.iter().any(|u| u == name) {
                    used.push(name.to_string());
                }
                text.push_str(name);
                stored.push_str(&format!(" {{{name}}} "));
                rest = tail.get(close + 1..).unwrap_or("");
            }
            text.push_str(rest);
            stored.push_str(rest);
            let stored = stored.split_whitespace().collect::<Vec<_>>().join(" ");
            if used.is_empty() {
                return Err(no("nlsur needs a {parameter} in every equation"));
            }
            parsed.push((dep.to_string(), text, stored, used));
        }
        // initial(a 1 b 2): starting values beside the {name = #} ones
        if let Some(a) = opts.arg("initial", 4) {
            let words: Vec<&str> = a.split_whitespace().collect();
            if words.len() % 2 != 0 {
                return Err(no(
                    "nlsur: initial() takes pairs of a parameter and a number",
                ));
            }
            for pair in words.chunks(2) {
                let (Some(name), Some(v)) = (pair.first(), pair.get(1)) else {
                    continue;
                };
                let v: f64 = v
                    .parse()
                    .map_err(|_| no(format!("nlsur: the starting value {v} is not a number")))?;
                match pars.iter_mut().find(|(n, _)| n == name) {
                    Some(p) => p.1 = v,
                    None => {
                        return Err(no(format!(
                            "nlsur: initial() names {name}, which is not a parameter"
                        )))
                    }
                }
            }
        }
        // the parameters are numbers the expression reads, as the data's are
        let saved: Vec<(String, Option<Kind>)> = pars
            .iter()
            .map(|(n, _)| (n.clone(), self.kinds.get(n).cloned()))
            .collect();
        for (n, _) in &pars {
            self.kinds.insert(n.clone(), Kind::Number);
        }
        let mut failed = None;
        for (dep, text, stored, used) in parsed {
            match self.expr(&text, Use::Value, out) {
                Ok(expr) => eqs.push(Eq {
                    dep,
                    expr,
                    stored,
                    params: used,
                }),
                Err(e) => {
                    failed = Some(e);
                    break;
                }
            }
        }
        for (n, k) in saved {
            match k {
                Some(k) => {
                    self.kinds.insert(n, k);
                }
                None => {
                    self.kinds.remove(&n);
                }
            }
        }
        if let Some(e) = failed {
            return Err(e);
        }
        if let Some(ev) = &eval {
            for d in &ev.deps {
                eqs.push(Eq {
                    dep: d.clone(),
                    expr: String::new(),
                    stored: String::new(),
                    params: Vec::new(),
                });
            }
        }
        let level = super::model::level(opts)?;
        let num_opt = |name: &str, min: usize, default: f64| -> Result<f64, Untranslated> {
            match opts.arg(name, min) {
                Some(v) => v
                    .trim()
                    .parse::<f64>()
                    .map_err(|_| no(format!("nlsur, {name}({v}) needs a number"))),
                None => Ok(default),
            }
        };
        let iterate = match opts.arg("iterate", 4) {
            Some(i) => i
                .trim()
                .parse::<u32>()
                .map_err(|_| no(format!("nlsur, iterate({i}) needs a whole number")))?,
            None => 300,
        };
        let eps = num_opt("eps", 3, 1e-5)?;
        let delta = num_opt("delta", 5, 4e-7)?;
        if !(delta > 0.0 && delta.is_finite()) {
            return Err(no("nlsur, delta() needs a positive number"));
        }
        // variables(): rows with one of them missing are left out as well
        let mut markout = Vec::new();
        if let Some(vs) = opts.arg("variables", 9) {
            for v in vs.split_whitespace() {
                if !is_name(v) || !self.kinds.contains_key(v) {
                    return Err(no(format!(
                        "nlsur, variables(): only variables named in full are translated ({v})"
                    )));
                }
                markout.push(r_text(v));
            }
        }
        // the evaluator form: all the data's rows go in (the program is called on them), the
        // rows used are those the begin line marked
        let src = match &eval {
            Some(_) => self.data()?,
            None => {
                let rows = self.rows_for(c, out)?;
                self.weighted_rows(rows, &weight)
            }
        };
        let wargs = match &weight {
            Some((k, v)) => format!(
                ", wvar = {}, wtype = {}, wexp = {}",
                r_text(v),
                r_text(k.code()),
                r_text(&format!("= {v}"))
            ),
            None => String::new(),
        };
        let vargs = match &cluster {
            Some(v) => format!(", robust = TRUE, cluster = {}", r_text(v)),
            None if robust => ", robust = TRUE".to_string(),
            None => String::new(),
        };
        let margs = if markout.is_empty() {
            String::new()
        } else {
            format!(", markout = c({})", markout.join(", "))
        };
        for h in [
            "stata_fmt",
            "stata_tables",
            "stata_lsfam",
            "stata_eclass",
            "stata_returned",
            "stata_nlsur",
        ] {
            self.helpers.insert(h);
        }
        let eq_list = eqs
            .iter()
            .map(|q| {
                if eval.is_some() {
                    return format!("list(dep = {})", r_text(&q.dep));
                }
                format!(
                    "list(dep = {}, f = function(.v) with(.v, {}), sexp = {}, params = c({}))",
                    r_text(&q.dep),
                    q.expr,
                    r_text(&q.stored),
                    q.params
                        .iter()
                        .map(|p| r_text(p))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        let init = pars
            .iter()
            .map(|(n, v)| format!("{} = {}", r_name(n), r_number(*v)))
            .collect::<Vec<_>>()
            .join(", ");
        let show = !self.quiet(c);
        // the evaluator form: the function its program became, the rows it marked, e(rhs), e(funcprog)
        let eargs = match &eval {
            Some(ev) => format!(
                ", evaluator = stata_nlsur_fn, touse = \"__nlsur_touse\", rhs = {}, funcprog = {}{}",
                r_text(&ev.rhs.join(" ")),
                r_text(&ev.prog),
                match opts.arg("hasconstants", 4) {
                    Some(h) => format!(
                        ", hascons = c({})",
                        h.split_whitespace().map(r_text).collect::<Vec<_>>().join(", ")
                    ),
                    None => String::new(),
                }
            ),
            None => String::new(),
        };
        let cmdline = super::returned::cmdline(c);
        let cmdline = match cmdline.strip_prefix("_rcraft_nlsur_fit") {
            Some(r) => format!("nlsur{r}"),
            None => cmdline,
        };
        out.line(format!(
            "stata_e <- stata_nlsur({src}, list({eq_list}), c({init}), level = {}, method = {}, nolog = {}, noconst = {}, iterate = {iterate}L, eps = {}, delta = {}{wargs}{vargs}{margs}{eargs}, cmdline = {}, show = {})",
            r_number(level * 100.0),
            r_text(method),
            super::stats::r_logical(opts.has("nolog", 5)),
            super::stats::r_logical(opts.has("noconstant", 3)),
            r_number(eps),
            r_number(delta),
            r_text(&cmdline),
            super::stats::r_logical(show)
        ));
        out.line("stata_b <- attr(stata_e, \"stata_b\")");
        out.line("stata_vcov <- attr(stata_e, \"stata_V\")");
        out.line("stata_df <- attr(stata_e, \"stata_df\")");
        out.line("stata_estimates <- list(rows = attr(stata_e, \"stata_rows\"))");
        out.line(self.keep_rows(&src, "stata_estimates$rows"));
        let d = self.data()?;
        out.package("dplyr");
        out.line(super::model::esample_rows_line(
            &d,
            &src,
            "stata_estimates$rows",
        ));
        self.esample_new = true;
        self.fit_robust = false;
        self.fit_cluster = false;
        // test, lincom and nlcom read e(b) and e(V) (z, as Stata's); margins is refused
        self.estimates = Some(Estimates {
            fit: Fit::Nlsur,
            dep: eqs.first().map(|q| q.dep.clone()).unwrap_or_default(),
            factors: std::collections::BTreeMap::new(),
        });
        self.estimates_lost = None;
        self.active_store = None;
        self.e_text.clear();
        self.e_new = true;
        let mut texts: Vec<String> = ["cmdline", "cmd", "predict", "estat_cmd"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        if eval.is_some() {
            texts.push("rhs".into());
            texts.push("funcprog".into());
        }
        if weight.is_some() {
            texts.push("wexp".into());
            texts.push("wtype".into());
        }
        if cluster.is_some() {
            texts.push("clustvar".into());
        }
        if robust || cluster.is_some() || matches!(weight, Some((W::P, _))) {
            texts.push("vcetype".into());
        }
        texts.extend(
            ["vce", "method", "constants", "type", "params", "depvar"]
                .iter()
                .map(|s| s.to_string()),
        );
        for q in (1..=eqs.len()).rev() {
            if eval.is_none() {
                texts.push(format!("params_{q}"));
            }
            texts.push(format!("depvar_{q}"));
        }
        if eval.is_none() {
            for q in (1..=eqs.len()).rev() {
                texts.push(format!("sexp_{q}"));
            }
        }
        texts.push("properties".into());
        self.e_ret = super::returned::Returned::Program {
            cmd: "nlsur".into(),
            texts,
        };
        // a run that ran out of steps: its table and e() are Stata's, then r(430)
        out.line("if (isTRUE(attr(stata_e, \"stata_unconverged\"))) { stata_fit_failed <- FALSE; stop(\"convergence not achieved (Stata: r(430))\", call. = FALSE) }");
        if weight.is_some() {
            out.caveat(super::weights::weight_caveat(&weight));
        }
        out.caveat(
            "nlsur as Stata 19 runs it: Gauss-Newton on the stacked equations with forward-difference derivatives, \
             NLS then FGNLS (Sigma from the NLS residuals), its log and stopping rule matched to its output",
        );
        Ok(())
    }

    /// `margins [if] [in] [, predict(yhat equation(#k)) level()]` after `nlsur`: Stata's predictive
    /// margin of one equation's fitted values (the model has no covariates, so dydx() and at() are
    /// Stata's r(322)).
    pub(super) fn nlsur_margins(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let base = self.data()?;
        // terms: the model has no covariates, so Stata stops on the first one (a variable with a
        // fraction r(452), else r(322)); seen with one term, and with the fractional one first
        let terms: Vec<&str> = c.parts.main.split_whitespace().collect();
        if let Some(first) = terms.first() {
            if !is_name(first) || !self.kinds.contains_key(*first) {
                return Err(no(format!(
                    "margins {} after nlsur is not translated yet (a plain variable is)",
                    c.parts.main.trim()
                )));
            }
            let m452 = format!(
                "{first}:  factor variables may not contain noninteger values (Stata: r(452))"
            );
            let m322 = format!("factor {first} not found in list of covariates (Stata: r(322))");
            let multi = "RCraft: margins with several terms after nlsur, the first a whole-number variable, is not translated yet";
            out.line(format!(
                "local({{ v <- as.numeric(unclass({base}[[{}]])); v <- v[!is.na(v)]; if (any(v != round(v))) stop({}, call. = FALSE) else if ({}) stop({}, call. = FALSE) else stop({}, call. = FALSE) }})",
                r_text(first),
                r_text(&m452),
                super::stats::r_logical(terms.len() == 1),
                r_text(&m322),
                r_text(multi)
            ));
            return Ok(());
        }
        for o in ["dydx", "eyex", "dyex", "eydx"] {
            if let Some(v) = opts.arg(o, 4) {
                let first = v.split_whitespace().next().unwrap_or("").to_string();
                super::sureg::eq_stop(
                    out,
                    &format!(
                        "invalid {o}() option;\n{first} not found in list of covariates (Stata: r(322))"
                    ),
                );
                return Ok(());
            }
        }
        if opts.has("at", 2) {
            super::sureg::eq_stop(out, "model contains no indepvars (Stata: r(322))");
            return Ok(());
        }
        if let Some(o) = opts.other(&[("predict", 7), ("level", 3), ("over", 4), ("post", 4)]) {
            return Err(no(format!(
                "margins after nlsur with the option {o} is not translated yet"
            )));
        }
        let level = super::model::level(opts)?;
        let popts_text = opts
            .arg("predict", 7)
            .map(|p| p.trim().to_string())
            .unwrap_or_default();
        let (k, ptext) = match opts.arg("predict", 7) {
            None => (1, "predict()".to_string()),
            Some(p) => {
                let p = p.trim().to_string();
                let parsed = crate::lang::syntax::options(&p);
                let popts = Opts(&parsed);
                if popts.has("residuals", 1) {
                    super::sureg::eq_stop(
                        out,
                        "prediction is a function of possibly stochastic quantities other than e(b) (Stata: r(498))",
                    );
                    return Ok(());
                }
                if let Some(o) = popts.other(&[("yhat", 1), ("equation", 2)]) {
                    return Err(no(format!(
                        "margins after nlsur, predict({o}) is not translated yet"
                    )));
                }
                let k = match popts.arg("equation", 2) {
                    Some(e) => match e
                        .trim()
                        .strip_prefix('#')
                        .and_then(|n| n.parse::<u32>().ok())
                    {
                        Some(k) => k,
                        None => {
                            return Err(no(format!(
                            "margins after nlsur, equation({}) is not translated (equation(#k) is)",
                            e.trim()
                        )))
                        }
                    },
                    None => 1,
                };
                (k, format!("predict({p})"))
            }
        };
        let over: Vec<String> = match opts.arg("over", 4) {
            Some(v) => {
                let vs: Vec<String> = v.split_whitespace().map(str::to_string).collect();
                if vs.is_empty()
                    || vs
                        .iter()
                        .any(|w| !is_name(w) || !self.kinds.contains_key(w))
                {
                    return Err(no(format!(
                        "margins, over({}) after nlsur: only variables named in full are translated",
                        v.trim()
                    )));
                }
                vs
            }
            None => Vec::new(),
        };
        let post = opts.has("post", 4);
        if post && self.fit_cluster_nlsur() {
            return Err(no(
                "margins, post after nlsur with vce(cluster) is not translated yet",
            ));
        }
        let sel = match self.condition(c, out)? {
            Some(cond) => {
                out.package("dplyr");
                format!("dplyr::pull(dplyr::mutate({base}, stata_sel = {cond}), stata_sel)")
            }
            None => "TRUE".to_string(),
        };
        for h in ["stata_nlsur", "stata_tables", "stata_fmt", "stata_returned"] {
            self.helpers.insert(h);
        }
        // e(cmdline) after post: margins and what follows it, as typed
        let rest = c
            .text
            .trim()
            .get(c.written.len()..)
            .unwrap_or("")
            .trim_start()
            .to_string();
        let over_r = if over.is_empty() {
            "NULL".to_string()
        } else {
            format!(
                "c({})",
                over.iter()
                    .map(|v| r_text(v))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        let call = format!(
            "stata_nlsur_margins(stata_e, {base}, {k}L, {sel}, level = {}, ptext = {}, popts = {}, over = {over_r}, post = {}, cmdline = {}, dname = {}, show = {})",
            r_number(level * 100.0),
            r_text(&ptext),
            r_text(&popts_text),
            super::stats::r_logical(post),
            r_text(&format!("margins {rest}")),
            r_text(&base),
            super::stats::r_logical(!self.quiet(c))
        );
        if post {
            // the margins become e(b) and e(V) (Stata 19's e() after margins, post)
            out.line(format!("stata_e <- {call}"));
            out.line("stata_b <- attr(stata_e, \"stata_b\")");
            out.line("stata_vcov <- stata_e$V");
            out.line("stata_df <- Inf");
            let dep = self
                .estimates
                .as_ref()
                .map(|e| e.dep.clone())
                .unwrap_or_default();
            self.estimates = Some(Estimates {
                fit: Fit::Margins,
                dep,
                factors: std::collections::BTreeMap::new(),
            });
            self.e_new = true;
            let mut texts: Vec<String> = [
                "cmd",
                "title",
                "model_vce",
                "model_vcetype",
                "vce",
                "vcetype",
                "expression",
                "predict1_label",
                "predict1_opts",
                "by",
                "over",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect();
            texts.extend((1..=64).map(|j| format!("by{j}")));
            texts.extend(
                [
                    "emptycells",
                    "est_cmd",
                    "est_cmdline",
                    "cmdline",
                    "properties",
                ]
                .iter()
                .map(|s| s.to_string()),
            );
            self.e_ret = super::returned::Returned::Program {
                cmd: "margins".into(),
                texts,
            };
            out.caveat(
                "margins, post after nlsur: the margins are now e(b) and e(V), with Stata 19's e()",
            );
        } else {
            out.line(format!("stata_r <- {call}"));
            self.r_new = Some((super::returned::R_MARGINS, "margins".into()));
        }
        out.caveat("margins after nlsur as Stata 19's: the mean fitted value of one equation over e(sample), with delta-method errors");
        Ok(())
    }

    /// The nlsur whose results are active was clustered.
    fn fit_cluster_nlsur(&self) -> bool {
        matches!(
            &self.e_ret,
            super::returned::Returned::Program { cmd, texts } if cmd == "nlsur" && texts.iter().any(|t| t == "clustvar")
        )
    }

    /// `estat summarize` after `nlsur`: the dependent variables over e(sample).
    pub(super) fn nlsur_summarize(&mut self, out: &mut Out) -> Result<(), Untranslated> {
        let base = self.data()?;
        for h in ["stata_nlsur", "stata_tables", "stata_fmt"] {
            self.helpers.insert(h);
        }
        out.line(format!("stata_nlsur_summarize(stata_e, {base})"));
        Ok(())
    }

    /// `predict [type] newvar [if] [in] [, yhat residuals equation(#k)]` after `nlsur`.
    pub(super) fn nlsur_predict(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[("yhat", 1), ("residuals", 1), ("equation", 2)]) {
            return Err(no(format!(
                "predict after nlsur with the option {o} is not translated yet"
            )));
        }
        if opts.has("yhat", 1) && opts.has("residuals", 1) {
            return Err(no("predict takes one statistic at a time"));
        }
        let k = match opts.arg("equation", 2) {
            Some(e) => {
                let e = e.trim().to_string();
                match e.strip_prefix('#').and_then(|n| n.parse::<u32>().ok()) {
                    Some(k) => k,
                    None => {
                        super::sureg::eq_stop(
                            out,
                            "option equation() misspecified (Stata: r(198))",
                        );
                        return Ok(());
                    }
                }
            }
            None => 1,
        };
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
        let resid = opts.has("residuals", 1);
        self.helpers.insert("stata_nlsur");
        self.helpers.insert("stata_float");
        if !resid && !opts.has("yhat", 1) && !quiet {
            out.line("cat(\"(option yhat assumed; fitted values)\\n\")");
        }
        let v = format!(
            "stata_nlsur_pred(stata_e, {base}, {k}L, resid = {}, dname = {})",
            super::stats::r_logical(resid),
            r_text(&base)
        );
        let v = stored(&v, st, &mut self.helpers, out);
        let v = match self.condition(c, out)? {
            Some(cond) => format!("dplyr::if_else({cond}, {v}, NA_real_)"),
            None => v,
        };
        out.package("dplyr");
        out.line(format!(
            "{base} <- dplyr::mutate({base}, {} = {v})",
            r_name(w)
        ));
        self.kinds.insert(w.to_string(), Kind::Number);
        self.storage.insert(w.to_string(), st);
        Ok(())
    }
}
