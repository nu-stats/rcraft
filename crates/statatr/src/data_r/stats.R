# RCraft: Stata's statistics commands in R (P2B.4).
# Written for RCraft from Stata's documented rules; each function stores
# what the Stata command returns under Stata's names, in `stata_r` (r-class)
# or `stata_e` (e-class), and prints a short table.

stata_g <- function(x, w = 10, d = 7) formatC(signif(as.numeric(x), d), width = w, format = "g", digits = d)

stata_missing <- function(x) if (is.character(x)) x == "" else is.na(x)

stata_num <- function(d, v) {
  x <- d[[v]]
  if (is.null(x)) stop("variable ", v, " not found", call. = FALSE)
  if (is.character(x)) stop(v, ": string variables are not allowed here", call. = FALSE)
  as.numeric(x)
}

# ---- factor variables -------------------------------------------------------
# A term is written as Stata names it: `x`, `i.g`, `ib2.g`, `ibn.g`,
# `ibfirst.g`, `iblast.g`, `ibfreq.g`, `i.g#c.x`; `##` is expanded before.

stata_fv_parse <- function(term) {
  lapply(strsplit(term, "#", fixed = TRUE)[[1]], function(p) {
    dot <- regexpr(".", p, fixed = TRUE)
    if (dot < 0) return(list(var = p, factor = FALSE, base = NA_character_))
    pre <- substr(p, 1, dot - 1)
    list(var = substr(p, dot + 1, nchar(p)), factor = pre != "c",
         base = if (pre == "c") NA_character_ else sub("^ib?", "", pre))
  })
}

stata_fv_name <- function(cells) {
  if (length(cells) == 1 && is.na(cells[[1]]$level)) {
    return(paste0(if (cells[[1]]$mark == "o") "o." else "", cells[[1]]$var))
  }
  paste(vapply(cells, function(c) {
    if (is.na(c$level)) paste0("c", c$mark, ".", c$var) else paste0(c$level, c$mark, ".", c$var)
  }, ""), collapse = "#")
}

stata_fv_column <- function(d, cells) {
  v <- rep(1, nrow(d))
  for (c in cells) {
    x <- as.numeric(d[[c$var]])
    v <- v * (if (is.na(c$level)) x else as.numeric(x == c$level))
  }
  v
}

# Every coefficient of `terms` over the sample `rows`: levels and cells of the
# factors, with base levels and cells a lower-order term covers marked
# "base" and interaction cells without observations marked "empty".
stata_fv_expand <- function(d, terms, rows) {
  parsed <- lapply(terms, stata_fv_parse)
  levels <- list()
  for (t in parsed) for (c in t) if (c$factor && is.null(levels[[c$var]])) {
    if (is.character(d[[c$var]])) stop(c$var, ":  string variables may not be used as factor variables", call. = FALSE)
    x <- as.numeric(d[[c$var]])[rows]
    if (any(x != trunc(x))) stop(c$var, ":  factor variables may not contain noninteger values", call. = FALSE)
    if (any(x < 0)) stop(c$var, ":  factor variables may not contain negative values", call. = FALSE)
    levels[[c$var]] <- sort(unique(x))
  }
  base_of <- function(c) {
    lv <- levels[[c$var]]
    b <- c$base
    if (b == "" || b == "first") return(lv[1])
    if (b == "last") return(lv[length(lv)])
    if (b == "n") return(NA_real_)
    if (b == "freq") {
      x <- as.numeric(d[[c$var]])[rows]
      return(lv[which.max(vapply(lv, function(l) sum(x == l), 0))])
    }
    b <- as.numeric(b)
    if (b %in% lv) b else NA_real_
  }
  sig <- function(cs) sort(vapply(cs, function(c) paste0(c$var, if (c$factor) ":f" else ":c"), ""))
  sigs <- lapply(parsed, sig)
  out <- list()
  for (t in parsed) {
    lists <- lapply(t, function(c) if (c$factor) levels[[c$var]] else NA_real_)
    grid <- as.matrix(rev(expand.grid(rev(lists), KEEP.OUT.ATTRS = FALSE)))
    for (k in seq_len(nrow(grid))) {
      pick <- grid[k, ]
      isb <- vapply(seq_along(t), function(j) t[[j]]$factor && isTRUE(base_of(t[[j]]) == pick[j]), TRUE)
      kind <- "fit"
      if (any(isb)) {
        rest <- sig(t[!isb])
        if (length(rest) == 0 || any(vapply(sigs, identical, TRUE, rest))) kind <- "base"
      }
      if (kind == "fit" && sum(vapply(t, function(c) c$factor, TRUE)) > 1) {
        present <- rep(TRUE, length(rows))
        for (j in seq_along(t)) if (t[[j]]$factor) present <- present & as.numeric(d[[t[[j]]$var]])[rows] == pick[j]
        if (!any(present)) kind <- "empty"
      }
      cells <- lapply(seq_along(t), function(j) {
        mark <- if (isb[j]) "b" else if (kind != "fit") "o" else ""
        list(var = t[[j]]$var, level = if (t[[j]]$factor) unname(pick[j]) else NA_real_, mark = mark)
      })
      out[[length(out) + 1]] <- list(cells = cells, kind = kind)
    }
  }
  out
}

# Which columns are collinear with the constant (when there is one) and the
# columns kept before them. regress takes the columns largest first (so the
# smallest of a collinear set goes); the ML commands take them in order.
stata_collinear <- function(X, constant, how) {
  k <- ncol(X)
  if (k == 0) return(logical(0))
  raw <- colSums(X^2)
  Xc <- if (constant) sweep(X, 2, colMeans(X)) else X
  A <- crossprod(Xc)
  dropped <- rep(FALSE, k)
  swept <- rep(FALSE, k)
  repeat {
    open <- which(!swept & !dropped)
    for (j in open) if (A[j, j] <= 1e-13 * max(raw[j], .Machine$double.xmin)) dropped[j] <- TRUE
    open <- which(!swept & !dropped)
    if (!length(open)) break
    p <- if (how == "order") open[1] else open[which.max(diag(A)[open])]
    swept[p] <- TRUE
    rest <- which(!swept & !dropped)
    if (length(rest)) A[rest, rest] <- A[rest, rest] - outer(A[rest, p], A[p, rest]) / A[p, p]
  }
  dropped
}

# The estimation sample and design: rows where `cond` holds and every
# variable is present, the coefficients (with Stata's names), and X for the
# fitted ones (the constant last).
stata_model <- function(d, y, terms, cond = TRUE, cons = TRUE, how = "order", extra = character(),
                        add_cons = cons) {
  vars <- unique(c(y, unlist(lapply(terms, function(t) vapply(stata_fv_parse(t), function(c) c$var, ""))), extra))
  ok <- rep(TRUE, nrow(d)) & cond
  ok[is.na(ok)] <- FALSE
  for (v in vars) {
    if (is.null(d[[v]])) stop("variable ", v, " not found", call. = FALSE)
    ok <- ok & !stata_missing(d[[v]])
  }
  rows <- which(ok)
  if (!length(rows)) stop("no observations", call. = FALSE)
  coefs <- stata_fv_expand(d, terms, rows)
  fit <- which(vapply(coefs, function(c) c$kind == "fit", TRUE))
  X <- vapply(coefs[fit], function(c) stata_fv_column(d, c$cells)[rows], numeric(length(rows)))
  X <- matrix(X, nrow = length(rows))
  gone <- stata_collinear(X, cons, how)
  for (j in fit[gone]) {
    coefs[[j]]$kind <- "omitted"
    coefs[[j]]$cells <- lapply(coefs[[j]]$cells, function(c) { if (c$mark != "b") c$mark <- "o"; c })
    message("note: ", stata_fv_name(coefs[[j]]$cells), " omitted because of collinearity.")
  }
  X <- X[, !gone, drop = FALSE]
  names <- vapply(coefs, function(c) stata_fv_name(c$cells), "")
  colnames(X) <- names[fit[!gone]]
  if (add_cons) {
    coefs[[length(coefs) + 1]] <- list(cells = list(), kind = "fit", cons = TRUE)
    names <- c(names, "_cons")
    X <- cbind(X, `_cons` = 1)
  }
  list(rows = rows, y = if (length(y)) stata_num(d, y[1])[rows] else NULL, X = X, coefs = coefs,
       names = names, fitted = colnames(X), n = length(rows), cons = cons)
}

# The model's coefficients as Stata's e(b) and e(V): every name, with zeros
# for the base, empty and omitted ones.
stata_full <- function(m, b, V) {
  B <- setNames(rep(0, length(m$names)), m$names)
  B[m$fitted] <- b
  VV <- matrix(0, length(m$names), length(m$names), dimnames = list(m$names, m$names))
  VV[m$fitted, m$fitted] <- V
  list(b = B, V = VV)
}

# The columns of a model's coefficients over all rows of `d` (for predict).
stata_design <- function(d, e) {
  X <- vapply(e$coefs, function(c) {
    if (isTRUE(c$cons)) rep(1, nrow(d)) else if (isTRUE(c$aux)) rep(0, nrow(d)) else stata_fv_column(d, c$cells)
  }, numeric(nrow(d)))
  X <- matrix(X, nrow = nrow(d))
  colnames(X) <- names(e$b)
  X
}

stata_sandwich <- function(bread, scores, cluster = NULL, n) {
  if (is.null(cluster)) {
    return(list(V = bread %*% crossprod(scores) %*% bread, G = NA))
  }
  S <- rowsum(scores, cluster, reorder = FALSE)
  list(V = bread %*% crossprod(S) %*% bread, G = nrow(S))
}

stata_cluster_values <- function(d, v, rows) {
  x <- d[[v]]
  if (is.null(x)) stop("variable ", v, " not found", call. = FALSE)
  x[rows]
}

stata_wald <- function(b, V, idx) {
  if (!length(idx)) return(NA_real_)
  Vi <- tryCatch(solve(V[idx, idx, drop = FALSE]), error = function(e) NULL)
  if (is.null(Vi)) return(NA_real_)
  drop(t(b[idx]) %*% Vi %*% b[idx])
}

# ---- the coefficient table --------------------------------------------------

stata_coef_table <- function(e, eform = NULL) {
  b <- e$b
  se <- sqrt(pmax(diag(e$V), 0))
  small <- !is.null(e$df_r) && !is.na(e$df_r)
  stat <- b / se
  p <- if (small) 2 * stats::pt(-abs(stat), e$df_r) else 2 * stats::pnorm(-abs(stat))
  crit <- if (small) stats::qt(0.975, e$df_r) else stats::qnorm(0.975)
  lo <- b - crit * se
  hi <- b + crit * se
  head <- if (is.null(eform)) "Coefficient" else eform
  if (!is.null(eform)) {
    se <- exp(b) * se
    lo <- exp(lo)
    hi <- exp(hi)
    b <- exp(b)
  }
  cat(sprintf("%13s | %11s %10s %8s %7s  %21s\n", e$depvar, head, "Std. err.", if (small) "t" else "z",
              if (small) "P>|t|" else "P>|z|", "[95% conf. interval]"))
  cat(strrep("-", 14), "+", strrep("-", 64), "\n", sep = "")
  kinds <- vapply(e$coefs, function(c) c$kind, "")
  for (j in seq_along(b)) {
    nm <- names(b)[j]
    if (kinds[j] == "fit") {
      cat(sprintf("%13s | %11s %10s %8s %7s %10s %10s\n", substr(nm, 1, 13), stata_g(b[j], 11), stata_g(se[j]),
                  formatC(stat[j], format = "f", digits = 2), formatC(p[j], format = "f", digits = 3),
                  stata_g(lo[j]), stata_g(hi[j])))
    } else {
      cat(sprintf("%13s | %11s  (%s)\n", substr(nm, 1, 13), if (is.null(eform)) "0" else "1", kinds[j]))
    }
  }
  cat(strrep("-", 79), "\n", sep = "")
  invisible(NULL)
}

stata_header <- function(title, lines) {
  cat("\n", title, "\n", sep = "")
  for (l in names(lines)) cat(sprintf("%48s%-17s = %10s\n", "", l, lines[[l]]))
  cat("\n")
}

# ---- regress ----------------------------------------------------------------

stata_regress <- function(d, y, terms, cond = TRUE, vce = "ols", cluster = NULL, cons = TRUE, show = TRUE) {
  m <- stata_model(d, y, terms, cond, cons, how = "pivot", extra = cluster)
  X <- m$X
  yv <- m$y
  n <- m$n
  k <- ncol(X)
  XtXi <- solve(crossprod(X))
  b <- drop(XtXi %*% crossprod(X, yv))
  names(b) <- colnames(X)
  res <- drop(yv - X %*% b)
  rss <- sum(res^2)
  tss <- if (cons) sum((yv - mean(yv))^2) else sum(yv^2)
  mss <- tss - rss
  df_m <- k - cons
  df_r <- n - k
  s2 <- rss / df_r
  V <- s2 * XtXi
  G <- NA
  if (vce == "robust") V <- stata_sandwich(XtXi, X * res)$V * n / (n - k)
  if (vce == "cluster") {
    s <- stata_sandwich(XtXi, X * res, stata_cluster_values(d, cluster, m$rows))
    G <- s$G
    V <- s$V * (G / (G - 1)) * ((n - 1) / (n - k))
    df_r <- G - 1
  }
  dimnames(V) <- list(names(b), names(b))
  idx <- setdiff(names(b), "_cons")
  F <- if (df_m > 0) stata_wald(b, V, idx) / length(idx) else NA_real_
  if (vce == "cluster" && length(idx) > df_r) F <- NA_real_
  full <- stata_full(m, b, V)
  r2 <- mss / tss
  e <- list(cmd = "regress", depvar = y, b = full$b, V = full$V, coefs = m$coefs, N = n,
            df_m = df_m, df_r = df_r, F = F, r2 = r2,
            r2_a = 1 - (1 - r2) * (n - cons) / (n - k),
            rmse = sqrt(s2), mss = mss, rss = rss,
            ll = -0.5 * n * (log(2 * pi) + log(rss / n) + 1),
            ll_0 = if (cons) -0.5 * n * (log(2 * pi) + log(tss / n) + 1) else NA_real_,
            rank = k, sample = seq_len(nrow(d)) %in% m$rows, vce = vce,
            N_clust = G, clustvar = cluster, family = "gaussian")
  e$p <- if (is.na(F)) NA_real_ else stats::pf(F, length(idx), df_r, lower.tail = FALSE)
  if (show) {
    stata_header(if (vce == "ols") "      Source |       SS           df       MS" else "Linear regression",
                 list("Number of obs" = n, "F" = stata_g(F, 10, 6), "Prob > F" = formatC(e$p, format = "f", digits = 4),
                      "R-squared" = formatC(r2, format = "f", digits = 4), "Root MSE" = stata_g(e$rmse, 10, 5)))
    stata_coef_table(e)
  }
  invisible(e)
}

# ---- maximum likelihood -----------------------------------------------------
# Newton-Raphson as Stata runs it ([R] maximize): the full step, halved while
# it lowers the likelihood, otherwise stretched by 1/8, 1/4, ... while that
# raises it; converged when g H^-1 g' <= 1e-5 and either the log likelihood
# (1e-7) or the coefficients (1e-6) have stopped changing. Iteration 0 is
# never the last. Where -H is not positive definite, -H + lambda I is used.
# `f(b, TRUE)` gives the log likelihood, gradient, -H and scores.

stata_newton <- function(f, start, maxit = 300) {
  reldif <- function(a, b) abs(a - b) / (abs(b) + 1)
  inv_of <- function(A) {
    ch <- tryCatch(chol(A), error = function(e) NULL)
    if (!is.null(ch)) return(list(inv = chol2inv(ch), concave = TRUE))
    lambda <- 1e-4 * max(abs(diag(A)), 1)
    for (i in seq_len(80)) {
      ch <- tryCatch(chol(A + diag(lambda, nrow(A))), error = function(e) NULL)
      if (!is.null(ch)) return(list(inv = chol2inv(ch), concave = FALSE))
      lambda <- lambda * 2
    }
    stop("the likelihood could not be maximized", call. = FALSE)
  }
  bad <- function(v) is.na(v) || is.nan(v)
  b <- start
  prev <- NULL
  d <- f(b, TRUE)
  iv <- inv_of(d$H)
  iter <- 0
  converged <- FALSE
  repeat {
    iter <- iter + 1
    step <- drop(iv$inv %*% d$g)
    if (!is.null(prev) && iv$concave) {
      nr <- sum(d$g * step)
      if (abs(nr) <= 1e-5 && (reldif(d$ll, prev$ll) <= 1e-7 || max(reldif(b, prev$b)) <= 1e-6)) {
        converged <- TRUE
        break
      }
    }
    if (iter > maxit) break
    t <- 1
    nxt <- f(b + t * step)$ll
    halved <- FALSE
    while ((bad(nxt) || nxt < d$ll) && t > 1e-15) {
      halved <- TRUE
      t <- t / 2
      nxt <- f(b + t * step)$ll
    }
    if (!halved) {
      extra <- 0.125
      repeat {
        further <- f(b + (t + extra) * step)$ll
        if (!bad(further) && further > nxt) {
          t <- t + extra
          nxt <- further
          extra <- extra * 2
        } else {
          break
        }
      }
    }
    prev <- list(b = b, ll = d$ll)
    b <- b + t * step
    d <- f(b, TRUE)
    iv <- inv_of(d$H)
  }
  if (!converged) warning("convergence not achieved", call. = FALSE)
  V <- if (iv$concave) iv$inv else solve(d$H)
  list(b = b, ll = d$ll, V = V, scores = d$scores, converged = converged, iterations = iter - 1)
}

# ---- logit, probit, poisson -------------------------------------------------

stata_ml <- function(model, d, y, terms, cond = TRUE, vce = "oim", cluster = NULL, offset = NULL,
                     exposure = NULL, eform = NULL, show = TRUE) {
  m <- stata_model(d, y, terms, cond, TRUE, how = "order", extra = c(cluster, offset, exposure))
  X <- m$X
  yv <- m$y
  n <- m$n
  off <- rep(0, n)
  if (!is.null(offset)) off <- stata_num(d, offset)[m$rows]
  if (!is.null(exposure)) {
    ex <- stata_num(d, exposure)[m$rows]
    if (any(ex <= 0)) stop(exposure, " must be greater than zero", call. = FALSE)
    off <- log(ex)
  }
  if (model == "poisson") {
    if (any(yv < 0)) stop(y, " must be greater than or equal to zero", call. = FALSE)
  } else {
    yv <- as.numeric(yv != 0)
    if (all(yv == 1) || all(yv == 0)) stop("outcome does not vary", call. = FALSE)
  }
  q <- 2 * yv - 1
  f <- function(b, full = FALSE) {
    eta <- drop(X %*% b) + off
    if (model == "poisson") {
      mu <- exp(eta)
      ll <- sum(-mu + yv * eta - lgamma(yv + 1))
      if (full) {
        w <- mu
        score <- yv - mu
      }
    } else if (model == "logit") {
      ll <- sum(stats::plogis(q * eta, log.p = TRUE))
      if (full) {
        pr <- stats::plogis(eta)
        w <- pr * (1 - pr)
        score <- yv - pr
      }
    } else {
      ll <- sum(stats::pnorm(q * eta, log.p = TRUE))
      if (full) {
        lam <- q * exp(stats::dnorm(eta, log = TRUE) - stats::pnorm(q * eta, log.p = TRUE))
        w <- lam * (lam + eta)
        score <- lam
      }
    }
    if (!full) return(list(ll = ll))
    list(ll = ll, g = colSums(X * score), H = crossprod(X * w, X), scores = X * score)
  }
  # Stata starts from the constant-only fit: slopes 0, the constant at the mean
  start <- rep(0, ncol(X))
  if (model == "poisson") {
    start[ncol(X)] <- log(sum(yv) / sum(exp(off)))
  } else {
    p1 <- mean(yv)
    start[ncol(X)] <- if (model == "logit") stats::qlogis(p1) else stats::qnorm(p1)
  }
  ll_0 <- f(start)$ll
  fit <- stata_newton(f, start)
  b <- setNames(fit$b, colnames(X))
  ll <- fit$ll
  if (model != "poisson") {
    eta <- drop(X %*% b) + off
    pr <- if (model == "logit") stats::plogis(eta) else stats::pnorm(eta)
    if (any(pr < 1e-10 | pr > 1 - 1e-10)) warning("some outcomes are predicted perfectly; Stata drops those observations, so its results will differ", call. = FALSE)
  }
  V <- fit$V
  G <- NA
  if (vce == "robust") V <- stata_sandwich(V, fit$scores)$V * n / (n - 1)
  if (vce == "cluster") {
    s <- stata_sandwich(V, fit$scores, stata_cluster_values(d, cluster, m$rows))
    G <- s$G
    V <- s$V * G / (G - 1)
  }
  dimnames(V) <- list(names(b), names(b))
  idx <- setdiff(names(b), "_cons")
  chi2 <- if (vce == "oim") 2 * (ll - ll_0) else stata_wald(b, V, idx)
  full <- stata_full(m, b, V)
  e <- list(cmd = model, depvar = y, b = full$b, V = full$V, coefs = m$coefs, N = n,
            ll = ll, ll_0 = ll_0, df_m = length(idx), chi2 = chi2,
            chi2type = if (vce == "oim") "LR" else "Wald",
            p = stats::pchisq(chi2, length(idx), lower.tail = FALSE),
            r2_p = 1 - ll / ll_0, rank = ncol(X), sample = seq_len(nrow(d)) %in% m$rows,
            vce = vce, N_clust = G, clustvar = cluster, family = model,
            offset = offset, exposure = exposure, converged = fit$converged)
  if (show) {
    title <- c(logit = "Logistic regression", probit = "Probit regression", poisson = "Poisson regression")[[model]]
    lines <- list("Number of obs" = n)
    lines[[paste0(e$chi2type, " chi2(", e$df_m, ")")]] <- formatC(chi2, format = "f", digits = 2)
    lines[["Prob > chi2"]] <- formatC(e$p, format = "f", digits = 4)
    lines[["Pseudo R2"]] <- formatC(e$r2_p, format = "f", digits = 4)
    stata_header(paste0(title, "    Log likelihood = ", formatC(ll, format = "f", digits = 4)), lines)
    stata_coef_table(e, eform)
  }
  invisible(e)
}

# Adds a parameter that is not a regressor (`/lnalpha`, `/cut1`) to a model.
stata_aux <- function(m, names) {
  for (n in names) m$coefs[[length(m$coefs) + 1]] <- list(cells = list(), kind = "fit", aux = TRUE)
  m$names <- c(m$names, names)
  m$fitted <- c(m$fitted, names)
  m
}

stata_ml_vce <- function(fit, vce, d, cluster, rows) {
  V <- fit$V
  G <- NA
  n <- nrow(fit$scores)
  if (vce == "robust") V <- stata_sandwich(V, fit$scores)$V * n / (n - 1)
  if (vce == "cluster") {
    s <- stata_sandwich(V, fit$scores, stata_cluster_values(d, cluster, rows))
    G <- s$G
    V <- s$V * G / (G - 1)
  }
  list(V = V, G = G)
}

stata_ml_show <- function(e, title, eform = NULL) {
  lines <- list("Number of obs" = e$N)
  lines[[paste0(e$chi2type, " chi2(", e$df_m, ")")]] <- formatC(e$chi2, format = "f", digits = 2)
  lines[["Prob > chi2"]] <- formatC(e$p, format = "f", digits = 4)
  if (!is.null(e$r2_p)) lines[["Pseudo R2"]] <- formatC(e$r2_p, format = "f", digits = 4)
  stata_header(paste0(title, "    Log likelihood = ", formatC(e$ll, format = "f", digits = 4)), lines)
  stata_coef_table(e, eform)
}

# ---- nbreg ------------------------------------------------------------------
# Mean dispersion; the parameters are the coefficients, the constant and
# ln alpha. Stata fits the Poisson model, then the constant-only negative
# binomial from (ln ybar, 0), then the full model from the Poisson slopes.

stata_nb_f <- function(X, y) {
  small <- y == floor(y) & y < 1e5
  function(b, full = FALSE) {
    k <- ncol(X)
    theta <- b[k + 1]
    eta <- drop(X %*% b[seq_len(k)])
    mu <- exp(eta)
    m <- exp(-theta)
    lg <- lgamma(y + m) - lgamma(m)
    for (i in which(small)) lg[i] <- sum(log(m + seq_len(y[i]) - 1))
    mm <- m + mu
    ll <- sum(lg - lgamma(y + 1) + y * eta - y * log(mm) - m * log1p(mu / m))
    if (!full) return(list(ll = ll))
    dg <- digamma(y + m) - digamma(m)
    tg <- trigamma(y + m) - trigamma(m)
    for (i in which(small)) {
      j <- m + seq_len(y[i]) - 1
      dg[i] <- sum(1 / j)
      tg[i] <- -sum(1 / j^2)
    }
    s_eta <- (y - mu) * m / mm
    h_eta <- (y + m) * mu * m / mm^2
    d_m <- dg - log1p(mu / m) + (mu - y) / mm
    d_mm <- tg + mu / (m * mm) - (mu - y) / mm^2
    h_et <- m * (-mu * (mu - y) / mm^2)
    h_theta <- -(m * d_m + m^2 * d_mm)
    S <- cbind(X * s_eta, -m * d_m)
    cross <- colSums(X * h_et)
    H <- rbind(cbind(crossprod(X * h_eta, X), cross), c(cross, sum(h_theta)))
    list(ll = ll, g = colSums(S), H = H, scores = S)
  }
}

stata_nbreg <- function(d, y, terms, cond = TRUE, vce = "oim", cluster = NULL, eform = NULL, show = TRUE) {
  m <- stata_model(d, y, terms, cond, TRUE, how = "order", extra = cluster)
  X <- m$X
  yv <- m$y
  n <- m$n
  if (any(yv < 0)) stop(y, " must be greater than or equal to zero", call. = FALSE)
  k <- ncol(X)
  ybar <- mean(yv)
  pf <- function(b, full = FALSE) {
    eta <- drop(X %*% b)
    mu <- exp(eta)
    ll <- sum(-mu + yv * eta - lgamma(yv + 1))
    if (!full) return(list(ll = ll))
    list(ll = ll, g = colSums(X * (yv - mu)), H = crossprod(X * mu, X), scores = X * (yv - mu))
  }
  pois <- stata_newton(pf, c(rep(0, k - 1), log(ybar)))
  constant <- stata_newton(stata_nb_f(matrix(1, n, 1), yv), c(log(ybar), 0))
  slopes <- pois$b[seq_len(k - 1)]
  shift <- if (k > 1) sum(slopes * colMeans(X[, seq_len(k - 1), drop = FALSE])) else 0
  fit <- stata_newton(stata_nb_f(X, yv), c(slopes, log(ybar) - shift, constant$b[2]))
  m <- stata_aux(m, "/lnalpha")
  b <- setNames(fit$b, m$fitted)
  v <- stata_ml_vce(fit, vce, d, cluster, m$rows)
  dimnames(v$V) <- list(names(b), names(b))
  idx <- setdiff(names(b), c("_cons", "/lnalpha"))
  chi2 <- if (vce == "oim") 2 * (fit$ll - constant$ll) else stata_wald(b, v$V, idx)
  full <- stata_full(m, b, v$V)
  raw <- 2 * (fit$ll - pois$ll)
  e <- list(cmd = "nbreg", depvar = y, b = full$b, V = full$V, coefs = m$coefs, N = n,
            ll = fit$ll, ll_0 = constant$ll, df_m = length(idx), chi2 = chi2,
            chi2type = if (vce == "oim") "LR" else "Wald",
            p = stats::pchisq(chi2, length(idx), lower.tail = FALSE),
            r2_p = 1 - fit$ll / constant$ll, alpha = exp(fit$b[k + 1]), ll_c = pois$ll,
            chi2_c = max(raw, 0), rank = k + 1, sample = seq_len(nrow(d)) %in% m$rows, vce = vce,
            N_clust = v$G, clustvar = cluster, family = "nbreg", converged = fit$converged)
  if (show) {
    stata_ml_show(e, "Negative binomial regression", eform)
    cat(sprintf("%13s | %11s\n", "alpha", stata_g(e$alpha, 11)))
  }
  invisible(e)
}

# ---- ologit, oprobit --------------------------------------------------------
# P(y = j) = F(cut_j - xb) - F(cut_j-1 - xb), no constant; Stata starts with
# the slopes at zero and the cutpoints at the cumulative shares.

stata_ordered <- function(link, d, y, terms, cond = TRUE, vce = "oim", cluster = NULL, eform = NULL, show = TRUE) {
  m <- stata_model(d, y, terms, cond, TRUE, how = "order", extra = cluster, add_cons = FALSE)
  X <- m$X
  n <- m$n
  k <- ncol(X)
  out <- sort(unique(m$y))
  if (length(out) < 2) stop("outcome does not vary", call. = FALSE)
  cat_ <- match(m$y, out)
  nc <- length(out) - 1
  Fc <- if (link == "logit") stats::plogis else stats::pnorm
  dens <- if (link == "logit") function(z) { p <- stats::plogis(z); p * (1 - p) } else stats::dnorm
  ddens <- if (link == "logit") function(z) { p <- stats::plogis(z); p * (1 - p) * (1 - 2 * p) } else function(z) -z * stats::dnorm(z)
  hi <- cat_ <= nc
  lo <- cat_ > 1
  onehot <- function(j, ok) {
    M <- matrix(0, n, nc)
    M[cbind(which(ok), j[ok])] <- 1
    M
  }
  Ahi <- cbind(-X * hi, onehot(pmin(cat_, nc), hi))
  Alo <- cbind(-X * lo, onehot(pmax(cat_ - 1, 1), lo))
  f <- function(b, full = FALSE) {
    cuts <- b[k + seq_len(nc)]
    if (nc > 1 && any(diff(cuts) <= 0)) return(list(ll = -Inf))
    xb <- if (k) drop(X %*% b[seq_len(k)]) else rep(0, n)
    u <- cuts[pmin(cat_, nc)] - xb
    l <- cuts[pmax(cat_ - 1, 1)] - xb
    P <- ifelse(hi & lo, Fc(u) - Fc(l), ifelse(hi, Fc(u), Fc(-l)))
    ll <- sum(log(P))
    if (!full) return(list(ll = ll))
    fu <- ifelse(hi, dens(u), 0)
    fl <- ifelse(lo, dens(l), 0)
    S <- (Ahi * fu - Alo * fl) / P
    H <- crossprod(S) - crossprod(Ahi * (ifelse(hi, ddens(u), 0) / P), Ahi) +
      crossprod(Alo * (ifelse(lo, ddens(l), 0) / P), Alo)
    list(ll = ll, g = colSums(S), H = H, scores = S)
  }
  shares <- cumsum(tabulate(cat_, length(out)))[seq_len(nc)] / n
  start <- c(rep(0, k), if (link == "logit") stats::qlogis(shares) else stats::qnorm(shares))
  ll_0 <- f(start)$ll
  fit <- stata_newton(f, start)
  cuts <- paste0("/cut", seq_len(nc))
  m <- stata_aux(m, cuts)
  b <- setNames(fit$b, m$fitted)
  v <- stata_ml_vce(fit, vce, d, cluster, m$rows)
  dimnames(v$V) <- list(names(b), names(b))
  idx <- setdiff(names(b), cuts)
  chi2 <- if (vce == "oim") 2 * (fit$ll - ll_0) else stata_wald(b, v$V, idx)
  full <- stata_full(m, b, v$V)
  e <- list(cmd = paste0("o", link), depvar = y, b = full$b, V = full$V, coefs = m$coefs, N = n,
            ll = fit$ll, ll_0 = ll_0, df_m = length(idx), chi2 = chi2,
            chi2type = if (vce == "oim") "LR" else "Wald",
            p = stats::pchisq(chi2, length(idx), lower.tail = FALSE),
            r2_p = 1 - fit$ll / ll_0, k_cat = length(out), cat = out, rank = length(b),
            sample = seq_len(nrow(d)) %in% m$rows, vce = vce, N_clust = v$G, clustvar = cluster,
            family = paste0("o", link), converged = fit$converged)
  if (show) stata_ml_show(e, if (link == "logit") "Ordered logistic regression" else "Ordered probit regression", eform)
  invisible(e)
}

# ---- mlogit -----------------------------------------------------------------
# Each outcome but the base has its own coefficients; the base is the most
# frequent outcome (the first of ties) unless baseoutcome() names one.
# Stata starts at the constant-only model (constants ln(n_j / n_base)).

stata_mlogit <- function(d, y, terms, cond = TRUE, base = NULL, vce = "oim", cluster = NULL, eform = NULL, show = TRUE) {
  m <- stata_model(d, y, terms, cond, TRUE, how = "order", extra = cluster)
  X <- m$X
  n <- m$n
  p <- ncol(X)
  out <- sort(unique(m$y))
  if (length(out) < 2) stop("outcome does not vary", call. = FALSE)
  counts <- tabulate(match(m$y, out), length(out))
  bi <- if (is.null(base)) which.max(counts) else match(base, out)
  if (is.na(bi)) stop("outcome ", base, " not found", call. = FALSE)
  Y <- outer(m$y, out, "==") * 1
  others <- setdiff(seq_along(out), bi)
  J <- length(others)
  f <- function(b, full = FALSE) {
    B <- matrix(b, p, J)
    eta <- matrix(0, n, length(out))
    eta[, others] <- X %*% B
    mx <- apply(eta, 1, max)
    ex <- exp(eta - mx)
    P <- ex / rowSums(ex)
    ll <- sum(log(P[cbind(seq_len(n), match(m$y, out))]))
    if (!full) return(list(ll = ll))
    S <- do.call(cbind, lapply(others, function(j) X * (Y[, j] - P[, j])))
    H <- matrix(0, p * J, p * J)
    for (a in seq_len(J)) for (c in seq_len(J)) {
      w <- P[, others[a]] * ((a == c) - P[, others[c]])
      H[(a - 1) * p + seq_len(p), (c - 1) * p + seq_len(p)] <- crossprod(X * w, X)
    }
    list(ll = ll, g = colSums(S), H = H, scores = S)
  }
  start <- unlist(lapply(others, function(j) c(rep(0, p - 1), log(counts[j] / counts[bi]))))
  ll_0 <- f(start)$ll
  fit <- stata_newton(f, start)
  label <- function(v) format(v)
  eqn <- unlist(lapply(others, function(j) paste0(label(out[j]), ":", colnames(X))))
  b <- setNames(fit$b, eqn)
  v <- stata_ml_vce(fit, vce, d, cluster, m$rows)
  dimnames(v$V) <- list(eqn, eqn)
  idx <- eqn[!grepl(":_cons$", eqn)]
  chi2 <- if (vce == "oim") 2 * (fit$ll - ll_0) else stata_wald(b, v$V, idx)
  # e(b): every outcome's equation, the base's as zeros under o. names
  all <- unlist(lapply(seq_along(out), function(j) {
    nm <- m$names
    if (j == bi) nm <- ifelse(grepl("^o\\.", nm), nm, paste0("o.", nm))
    paste0(label(out[j]), ":", nm)
  }))
  B <- setNames(rep(0, length(all)), all)
  VV <- matrix(0, length(all), length(all), dimnames = list(all, all))
  B[eqn] <- b
  VV[eqn, eqn] <- v$V
  coefs <- rep(list(list(cells = list(), kind = "fit")), length(all))
  for (j in seq_along(all)) if (!(all[j] %in% eqn)) coefs[[j]]$kind <- if (startsWith(sub("^[^:]*:", "", all[j]), "o.")) "base" else "omitted"
  e <- list(cmd = "mlogit", depvar = y, b = B, V = VV, coefs = coefs, N = n, ll = fit$ll, ll_0 = ll_0,
            df_m = length(idx), chi2 = chi2, chi2type = if (vce == "oim") "LR" else "Wald",
            p = stats::pchisq(chi2, length(idx), lower.tail = FALSE), r2_p = 1 - fit$ll / ll_0,
            k_out = length(out), baseout = out[bi], out = out, rank = length(b),
            sample = seq_len(nrow(d)) %in% m$rows, vce = vce, N_clust = v$G, clustvar = cluster,
            family = "mlogit", converged = fit$converged)
  if (show) stata_ml_show(e, "Multinomial logistic regression", eform)
  invisible(e)
}

# ---- xtreg ------------------------------------------------------------------
# fe: least squares on y - ybar_i + ybar, df N - n - k, regressors without
# within variation omitted; vce(robust) clusters on the panel. re
# (Swamy-Arora): sigma_e from fe, sigma_u from the between regression less
# sigma_e^2 / T-bar (the harmonic mean of the group sizes), then least
# squares on the quasi-demeaned data.

stata_ols <- function(X, y) {
  XtXi <- solve(crossprod(X))
  b <- drop(XtXi %*% crossprod(X, y))
  r <- drop(y - X %*% b)
  list(b = b, XtXi = XtXi, res = r, rss = sum(r^2))
}

stata_xtreg <- function(d, y, terms, ivar, model = "re", cond = TRUE, vce = "ols", cluster = NULL, show = TRUE) {
  if (vce == "robust") cluster <- ivar
  m <- stata_model(d, y, terms, cond, TRUE, how = "order", extra = c(ivar, cluster))
  g <- as.numeric(stata_num(d, ivar)[m$rows])
  yv <- m$y
  N <- m$n
  ng <- length(unique(g))
  gi <- match(g, sort(unique(g)))
  T_i <- tabulate(gi)
  gmean <- function(v) (rowsum(v, gi, reorder = TRUE) / T_i)[gi, , drop = FALSE]
  slopes <- setdiff(colnames(m$X), "_cons")
  Xs <- m$X[, slopes, drop = FALSE]
  # the within transform, and the regressors it leaves no variation in
  Xw <- if (length(slopes)) Xs - gmean(Xs) + matrix(colMeans(Xs), N, length(slopes), byrow = TRUE) else Xs
  yw <- drop(yv - gmean(matrix(yv)) + mean(yv))
  gone <- stata_collinear(Xw, TRUE, "order")
  if (model == "fe" && any(gone)) {
    for (nm in slopes[gone]) {
      j <- match(nm, m$names)
      m$coefs[[j]]$kind <- "omitted"
      m$coefs[[j]]$cells <- lapply(m$coefs[[j]]$cells, function(c) { if (c$mark != "b") c$mark <- "o"; c })
      message("note: ", nm, " omitted because of collinearity.")
    }
    m$names <- vapply(seq_along(m$coefs), function(j) if (isTRUE(m$coefs[[j]]$cons)) "_cons" else stata_fv_name(m$coefs[[j]]$cells), "")
    m$fitted <- c(slopes[!gone], "_cons")
  }
  kf <- sum(!gone)
  Xf <- cbind(Xw[, !gone, drop = FALSE], `_cons` = 1)
  fe <- stata_ols(Xf, yw)
  df_fe <- N - ng - kf
  sigma_e2 <- fe$rss / df_fe
  xb_of <- function(b, cols) drop(cbind(Xs[, cols, drop = FALSE], 1) %*% b)
  r2s <- function(xb) {
    c(w = stats::cor(yv - gmean(matrix(yv)), xb - gmean(matrix(xb)))^2,
      b = stats::cor(rowsum(yv, gi) / T_i, rowsum(xb, gi) / T_i)[1]^2,
      o = stats::cor(yv, xb)^2)
  }
  cl <- if (!is.null(cluster)) stata_cluster_values(d, cluster, m$rows) else NULL
  if (model == "fe") {
    b <- setNames(fe$b, c(slopes[!gone], "_cons"))
    if (is.null(cl)) {
      V <- sigma_e2 * fe$XtXi
      df_t <- df_fe
      G <- NA
    } else {
      s <- stata_sandwich(fe$XtXi, Xf * fe$res, cl)
      G <- s$G
      V <- s$V * (G / (G - 1)) * ((N - 1) / (N - (kf + 1)))
      df_t <- G - 1
    }
    dimnames(V) <- list(names(b), names(b))
    xb <- xb_of(b, !gone)
    u_i <- drop(rowsum(yv - xb, gi) / T_i)
    sigma_u <- stats::sd(u_i)
    idx <- setdiff(names(b), "_cons")
    F <- if (kf) stata_wald(b, V, idx) / kf else NA_real_
    pooled <- stata_ols(cbind(Xs[, !gone, drop = FALSE], 1), yv)
    r2 <- r2s(xb)
    full <- stata_full(m, b, V)
    e <- list(cmd = "xtreg", model = "fe", depvar = y, b = full$b, V = full$V, coefs = m$coefs, N = N,
              N_g = ng, df_m = kf + ng - 1, df_b = kf, df_r = df_t, F = F, p = stats::pf(F, kf, df_t, lower.tail = FALSE),
              r2_w = r2[["w"]], r2_b = r2[["b"]], r2_o = r2[["o"]], sigma_u = sigma_u,
              sigma_e = sqrt(sigma_e2), rho = sigma_u^2 / (sigma_u^2 + sigma_e2),
              corr = stats::cor(u_i[gi], xb), F_f = ((pooled$rss - fe$rss) / (ng - 1)) / (fe$rss / df_fe),
              df_a = ng - 1, rss = fe$rss, N_clust = G, ivar = ivar, rank = kf + 1,
              sample = seq_len(nrow(d)) %in% m$rows, vce = vce, family = "gaussian")
  } else {
    # the between regression on the group means
    Xb <- cbind(rowsum(Xs, gi) / T_i, `_cons` = 1)
    be <- stata_ols(Xb, drop(rowsum(yv, gi) / T_i))
    s_be2 <- be$rss / (ng - ncol(Xb))
    tbar <- ng / sum(1 / T_i)
    su2 <- max(s_be2 - sigma_e2 / tbar, 0)
    theta <- (1 - sqrt(sigma_e2 / (T_i * su2 + sigma_e2)))[gi]
    Xq <- cbind(Xs - theta * gmean(Xs), `_cons` = 1 - theta)
    yq <- drop(yv - theta * gmean(matrix(yv)))
    gls <- stata_ols(Xq, yq)
    K <- ncol(Xq)
    b <- setNames(gls$b, colnames(m$X))
    if (is.null(cl)) {
      V <- gls$rss / (N - K) * gls$XtXi
      G <- NA
    } else {
      s <- stata_sandwich(gls$XtXi, Xq * gls$res, cl)
      G <- s$G
      V <- s$V * (G / (G - 1)) * ((N - 1) / (N - K))
    }
    dimnames(V) <- list(names(b), names(b))
    idx <- setdiff(names(b), "_cons")
    chi2 <- stata_wald(b, V, idx)
    r2 <- r2s(xb_of(b, rep(TRUE, length(slopes))))
    full <- stata_full(m, b, V)
    e <- list(cmd = "xtreg", model = "re", depvar = y, b = full$b, V = full$V, coefs = m$coefs, N = N,
              N_g = ng, df_m = length(idx), chi2 = chi2, chi2type = "Wald",
              p = stats::pchisq(chi2, length(idx), lower.tail = FALSE),
              r2_w = r2[["w"]], r2_b = r2[["b"]], r2_o = r2[["o"]], sigma_u = sqrt(su2),
              sigma_e = sqrt(sigma_e2), rho = su2 / (su2 + sigma_e2), Tbar = tbar,
              rmse = sqrt(gls$rss / (N - K)), N_clust = G, ivar = ivar, rank = K,
              sample = seq_len(nrow(d)) %in% m$rows, vce = vce, family = "gaussian")
  }
  if (show) {
    stata_header(paste(if (model == "fe") "Fixed-effects (within)" else "Random-effects GLS", "regression"),
                 list("Number of obs" = N, "Number of groups" = ng,
                      "R-sq within" = formatC(e$r2_w, format = "f", digits = 4),
                      "R-sq between" = formatC(e$r2_b, format = "f", digits = 4),
                      "R-sq overall" = formatC(e$r2_o, format = "f", digits = 4)))
    stata_coef_table(e)
    cat(sprintf("%13s | %11s\n%13s | %11s\n%13s | %11s\n", "sigma_u", stata_g(e$sigma_u, 11), "sigma_e",
                stata_g(e$sigma_e, 11), "rho", stata_g(e$rho, 11)))
  }
  invisible(e)
}

# ---- ivregress 2sls ---------------------------------------------------------
# X = endogenous, exogenous, constant; Z = exogenous, instruments, constant.
# sigma^2 = RSS/N (RSS/(N - k) with small); no finite-sample factor for
# vce(robust) or vce(cluster) without small.

stata_ivregress <- function(d, y, exog, endog, inst, cond = TRUE, small = FALSE, vce = "unadjusted",
                            cluster = NULL, show = TRUE) {
  vars <- unique(c(y, exog, endog, inst, cluster))
  ok <- rep(TRUE, nrow(d)) & cond
  ok[is.na(ok)] <- FALSE
  for (v in vars) ok <- ok & !stata_missing(d[[v]])
  rows <- which(ok)
  num <- function(vs) matrix(vapply(vs, function(v) stata_num(d, v)[rows], numeric(length(rows))),
                             length(rows), dimnames = list(NULL, vs))
  X <- cbind(num(c(endog, exog)), `_cons` = 1)
  Z <- cbind(num(c(exog, inst)), `_cons` = 1)
  yv <- stata_num(d, y)[rows]
  n <- length(rows)
  k <- ncol(X)
  Xh <- Z %*% solve(crossprod(Z), crossprod(Z, X))
  ainv <- solve(crossprod(Xh, X))
  b <- setNames(drop(ainv %*% crossprod(Xh, yv)), colnames(X))
  u <- drop(yv - X %*% b)
  rss <- sum(u^2)
  tss <- sum((yv - mean(yv))^2)
  df_r <- n - k
  sigma2 <- if (small) rss / df_r else rss / n
  G <- NA
  if (vce == "cluster") {
    s <- stata_sandwich(ainv, Xh * u, stata_cluster_values(d, cluster, rows))
    G <- s$G
    V <- s$V * (if (small) G / (G - 1) * (n - 1) / df_r else 1)
  } else if (vce == "robust") {
    V <- stata_sandwich(ainv, Xh * u)$V * (if (small) n / df_r else 1)
  } else {
    V <- sigma2 * ainv
  }
  dimnames(V) <- list(names(b), names(b))
  idx <- setdiff(names(b), "_cons")
  W <- stata_wald(b, V, idx)
  r2 <- 1 - rss / tss
  coefs <- lapply(names(b), function(nm) if (nm == "_cons") list(cells = list(), kind = "fit", cons = TRUE) else list(cells = list(list(var = nm, level = NA_real_, mark = "")), kind = "fit"))
  e <- list(cmd = "ivregress", estimator = "2sls", depvar = y, b = b, V = V, coefs = coefs, N = n,
            mss = tss - rss, rss = rss, rmse = sqrt(sigma2), r2 = r2, r2_a = 1 - (1 - r2) * (n - 1) / df_r,
            df_m = k - 1, rank = k, k_endog = length(endog), N_clust = G, endog = endog,
            exog = c(exog, inst), sample = seq_len(nrow(d)) %in% rows, vce = vce, family = "gaussian")
  if (small) {
    e$F <- W / length(idx)
    e$df_r <- df_r
    e$p <- stats::pf(e$F, length(idx), df_r, lower.tail = FALSE)
  } else {
    e$chi2 <- W
    e$p <- stats::pchisq(W, length(idx), lower.tail = FALSE)
  }
  if (show) {
    stata_header("Instrumental variables 2SLS regression",
                 list("Number of obs" = n, "R-squared" = formatC(r2, format = "f", digits = 4),
                      "Root MSE" = formatC(e$rmse, format = "f", digits = 4)))
    stata_coef_table(e)
    cat("Endogenous: ", paste(endog, collapse = " "), "\nExogenous:  ", paste(c(exog, inst), collapse = " "), "\n", sep = "")
  }
  invisible(e)
}

# ---- stcox ------------------------------------------------------------------
# Breslow's method for ties (Stata's default), Newton-Raphson from zero.

stata_stcox <- function(d, st, terms, cond = TRUE, eform = "Haz. ratio", show = TRUE) {
  if (is.null(st)) stop("data not st; use stset", call. = FALSE)
  m <- stata_model(d, st$time, terms, cond, FALSE, how = "order")
  tm <- m$y
  fail <- if (is.null(st$failure)) rep(1, m$n) else {
    fv <- as.numeric(d[[st$failure]])[m$rows]
    as.numeric(!is.na(fv) & fv != 0)
  }
  keep <- tm > 0
  X <- m$X[keep, , drop = FALSE]
  tm <- tm[keep]
  fail <- fail[keep]
  rows <- m$rows[keep]
  n <- length(tm)
  p <- ncol(X)
  o <- order(-tm)
  X <- X[o, , drop = FALSE]
  tm <- tm[o]
  fail <- fail[o]
  last <- stats::ave(seq_len(n), tm, FUN = max)
  ii <- rep(seq_len(p), p)
  jj <- rep(seq_len(p), each = p)
  f <- function(b, full = FALSE) {
    xb <- drop(X %*% b)
    w <- exp(xb)
    S0 <- cumsum(w)[last]
    ll <- sum((xb - log(S0))[fail == 1])
    if (!full) return(list(ll = ll))
    S1 <- apply(X * w, 2, cumsum)[last, , drop = FALSE]
    S2 <- apply(X[, ii, drop = FALSE] * X[, jj, drop = FALSE] * w, 2, cumsum)[last, , drop = FALSE]
    F1 <- fail == 1
    E1 <- S1[F1, , drop = FALSE] / S0[F1]
    g <- colSums(X[F1, , drop = FALSE] - E1)
    H <- matrix(colSums(S2[F1, , drop = FALSE] / S0[F1]), p, p) - crossprod(E1)
    list(ll = ll, g = g, H = H, scores = NULL)
  }
  ll_0 <- f(rep(0, p))$ll
  fit <- stata_newton(f, rep(0, p))
  b <- setNames(fit$b, colnames(X))
  V <- fit$V
  dimnames(V) <- list(names(b), names(b))
  full <- stata_full(m, b, V)
  chi2 <- 2 * (fit$ll - ll_0)
  e <- list(cmd = "cox", depvar = st$time, b = full$b, V = full$V, coefs = m$coefs, N = n,
            N_fail = sum(fail), ll = fit$ll, ll_0 = ll_0, df_m = p, chi2 = chi2, chi2type = "LR",
            p = stats::pchisq(chi2, p, lower.tail = FALSE), rank = p,
            sample = seq_len(nrow(d)) %in% rows, vce = "oim", family = "cox", converged = fit$converged)
  if (show) stata_ml_show(e, "Cox regression with Breslow method for ties", eform)
  invisible(e)
}

# ---- anova ------------------------------------------------------------------
# regress with the terms as factors, plus Stata's partial (Type III) sums
# of squares: each term's columns, coded to sum to zero (effects coding),
# dropped from the full model.

stata_anova <- function(d, y, terms, cond = TRUE, show = TRUE) {
  e <- stata_regress(d, y, terms, cond, show = FALSE)
  rows <- which(e$sample)
  sub <- d[rows, , drop = FALSE]
  effect_cols <- function(t) {
    cols <- matrix(1, length(rows), 1)
    for (c in stata_fv_parse(t)) {
      x <- as.numeric(sub[[c$var]])
      if (c$factor) {
        lv <- sort(unique(x))
        E <- vapply(lv[-1], function(l) (x == l) - (x == lv[1]), numeric(length(x)))
        E <- matrix(E, length(x))
      } else {
        E <- matrix(x, length(x))
      }
      cols <- do.call(cbind, lapply(seq_len(ncol(cols)), function(i) cols[, i] * E))
    }
    cols
  }
  blocks <- lapply(terms, effect_cols)
  yv <- stata_num(sub, y)
  fit_rss <- function(keep) {
    X <- do.call(cbind, c(list(rep(1, length(rows))), blocks[keep]))
    q <- qr(X)
    list(rss = sum(qr.resid(q, yv)^2), rank = q$rank)
  }
  full <- fit_rss(seq_along(terms))
  for (i in seq_along(terms)) {
    less <- fit_rss(setdiff(seq_along(terms), i))
    e[[paste0("ss_", i)]] <- less$rss - full$rss
    e[[paste0("df_", i)]] <- full$rank - less$rank
    e[[paste0("F_", i)]] <- (e[[paste0("ss_", i)]] / e[[paste0("df_", i)]]) / (e$rss / e$df_r)
  }
  e$cmd <- "anova"
  e$sstype <- "partial"
  e$terms <- terms
  if (show) {
    cat(sprintf("\n%27s %10s %10s %10s %10s %8s\n", "Source", "Partial SS", "df", "MS", "F", "Prob>F"))
    row <- function(nm, ss, df, F = NA) cat(sprintf("%27s %10s %10d %10s %10s %8s\n", nm, stata_g(ss), df, stata_g(ss / df),
                                                     if (is.na(F)) "" else formatC(F, format = "f", digits = 2),
                                                     if (is.na(F)) "" else formatC(stats::pf(F, df, e$df_r, lower.tail = FALSE), format = "f", digits = 4)))
    row("Model", e$mss, e$df_m, e$F)
    for (i in seq_along(terms)) row(terms[i], e[[paste0("ss_", i)]], e[[paste0("df_", i)]], e[[paste0("F_", i)]])
    row("Residual", e$rss, e$df_r)
    cat(sprintf("\nNumber of obs = %d   R-squared = %.4f   Root MSE = %s\n", e$N, e$r2, stata_g(e$rmse, 8, 6)))
  }
  invisible(e)
}

# ---- margins ----------------------------------------------------------------
# Predictions (or dydx() derivatives) averaged over the estimation sample,
# or at chosen values (at(), atmeans), with delta-method standard errors
# whose Jacobian is a five-point difference (step 1e-3 of the coefficient),
# as Stata takes it.

stata_margins <- function(d, e, cond = TRUE, over = NULL, at = NULL, atmeans = FALSE, dydx = NULL,
                          predict = NULL, show = TRUE) {
  if (is.null(e)) stop("last estimates not found", call. = FALSE)
  kind <- if (!is.null(predict) && predict == "xb") "xb" else switch(e$family,
    gaussian = "xb", logit = "logit", probit = "probit", poisson = "count", nbreg = "count",
    stop("margins after ", e$cmd, " is not translated yet", call. = FALSE))
  ok <- e$sample & (rep(TRUE, nrow(d)) & cond)
  ok[is.na(ok)] <- FALSE
  sub <- d[ok, , drop = FALSE]
  n <- nrow(sub)
  coefs <- e$coefs
  vars <- unique(unlist(lapply(coefs, function(c) vapply(c$cells, function(x) x$var, ""))))
  factor_vars <- unique(unlist(lapply(coefs, function(c) vapply(Filter(function(x) !is.na(x$level), c$cells), function(x) x$var, ""))))
  off <- rep(0, n)
  if (!is.null(e$offset)) off <- stata_num(sub, e$offset)
  if (!is.null(e$exposure)) off <- log(stata_num(sub, e$exposure))
  levels_of <- function(v) sort(unique(as.numeric(sub[[v]])))
  # a setting: for each covariate, values per observation or "at its means"
  observed <- lapply(setNames(vars, vars), function(v) list(v = as.numeric(sub[[v]])))
  base_setting <- if (atmeans) lapply(setNames(vars, vars), function(v) list(mean = TRUE)) else observed
  settings <- list()
  if (length(at)) {
    for (a in at) {
      grid <- expand.grid(a, KEEP.OUT.ATTRS = FALSE)
      for (r in seq_len(nrow(grid))) {
        s <- base_setting
        for (v in names(grid)) s[[v]] <- list(v = rep(grid[r, v], n))
        settings[[length(settings) + 1]] <- s
      }
    }
  } else {
    settings <- list(base_setting)
  }
  m_rows <- if (any(vapply(settings[[1]], function(x) isTRUE(x$mean), TRUE)) &&
                all(vapply(settings[[1]], function(x) isTRUE(x$mean) || length(unique(x$v)) == 1, TRUE))) 1 else n
  value_of <- function(s, cell) {
    x <- s[[cell$var]]
    if (isTRUE(x$mean)) {
      v <- as.numeric(sub[[cell$var]])
      return(rep(if (is.na(cell$level)) mean(v) else mean(v == cell$level), m_rows))
    }
    v <- x$v[seq_len(m_rows)]
    if (is.na(cell$level)) v else as.numeric(v == cell$level)
  }
  design <- function(s) {
    X <- vapply(coefs, function(c) {
      if (isTRUE(c$cons)) return(rep(1, m_rows))
      if (isTRUE(c$aux)) return(rep(0, m_rows))
      Reduce(`*`, lapply(c$cells, function(cell) value_of(s, cell)), rep(1, m_rows))
    }, numeric(m_rows))
    matrix(X, m_rows)
  }
  # d design / d covariate w (continuous)
  ddesign <- function(s, w) {
    X <- vapply(coefs, function(c) {
      hits <- which(vapply(c$cells, function(cell) cell$var == w && is.na(cell$level), TRUE))
      if (!length(hits)) return(rep(0, m_rows))
      rest <- lapply(c$cells[-hits[1]], function(cell) value_of(s, cell))
      length(hits) * Reduce(`*`, rest, rep(1, m_rows))
    }, numeric(m_rows))
    matrix(X, m_rows)
  }
  offm <- off[seq_len(m_rows)]
  if (m_rows == 1) offm <- mean(off)
  pred <- function(eta) switch(kind, xb = eta, logit = stats::plogis(eta), probit = stats::pnorm(eta), count = exp(eta))
  dpred <- function(eta) switch(kind, xb = rep(1, length(eta)), logit = { p <- stats::plogis(eta); p * (1 - p) },
                                probit = stats::dnorm(eta), count = exp(eta))
  # each margin as a function of the coefficients
  items <- list()
  labels <- character()
  add <- function(label, fun) {
    items[[length(items) + 1]] <<- fun
    labels <<- c(labels, label)
  }
  at_label <- function(i) if (length(settings) > 1) paste0(i, "._at") else ""
  for (i in seq_along(settings)) {
    s <- settings[[i]]
    if (!is.null(over)) {
      for (l in levels_of(over)) {
        s2 <- s
        s2[[over]] <- list(v = rep(l, n))
        X <- design(s2)
        add(paste0(l, ".", over, if (length(settings) > 1) paste0("#", at_label(i)) else ""),
            local({ X <- X; function(b) mean(pred(drop(X %*% b) + offm)) }))
      }
    } else if (length(dydx)) {
      for (w in dydx) {
        if (w %in% factor_vars) {
          lv <- levels_of(w)
          base_cells <- Filter(function(c) length(c$cells) == 1 && c$cells[[1]]$var == w && c$kind == "base", coefs)
          base <- if (length(base_cells)) base_cells[[1]]$cells[[1]]$level else lv[1]
          s0 <- s
          s0[[w]] <- list(v = rep(base, n))
          X0 <- design(s0)
          for (l in setdiff(lv, base)) {
            s1 <- s
            s1[[w]] <- list(v = rep(l, n))
            X1 <- design(s1)
            add(paste0(l, ".", w), local({ X0 <- X0; X1 <- X1; function(b) mean(pred(drop(X1 %*% b) + offm) - pred(drop(X0 %*% b) + offm)) }))
          }
        } else {
          X <- design(s)
          D <- ddesign(s, w)
          add(if (length(settings) > 1) paste0(w, "#", at_label(i)) else w,
              local({ X <- X; D <- D; function(b) mean(dpred(drop(X %*% b) + offm) * drop(D %*% b)) }))
        }
      }
    } else {
      X <- design(s)
      add(if (length(settings) > 1) at_label(i) else "_cons", local({ X <- X; function(b) mean(pred(drop(X %*% b) + offm)) }))
    }
  }
  b0 <- e$b
  est <- vapply(items, function(f) f(b0), 0)
  J <- matrix(0, length(items), length(b0))
  for (j in seq_along(b0)) {
    if (e$V[j, j] == 0) next
    h <- 1e-3 * max(abs(b0[j]), 1e-2)
    at_k <- function(k) { bb <- b0; bb[j] <- bb[j] + k * h; vapply(items, function(f) f(bb), 0) }
    J[, j] <- (at_k(-2) - 8 * at_k(-1) + 8 * at_k(1) - at_k(2)) / (12 * h)
  }
  V <- J %*% e$V %*% t(J)
  dimnames(V) <- list(labels, labels)
  out <- list(b = matrix(est, 1, dimnames = list("r1", labels)), V = V, N = n, Jacobian = J)
  if (show) {
    se <- sqrt(pmax(diag(V), 0))
    z <- est / se
    cat(sprintf("\n%13s | %11s %10s %8s %7s  %21s\n", "", if (length(dydx)) "dy/dx" else "Margin", "Std. err.", "z", "P>|z|", "[95% conf. interval]"))
    cat(strrep("-", 14), "+", strrep("-", 64), "\n", sep = "")
    for (k in seq_along(est)) cat(sprintf("%13s | %11s %10s %8.2f %7.3f %10s %10s\n", substr(labels[k], 1, 13), stata_g(est[k], 11),
                                          stata_g(se[k]), z[k], 2 * stats::pnorm(-abs(z[k])), stata_g(est[k] - 1.959964 * se[k]),
                                          stata_g(est[k] + 1.959964 * se[k])))
  }
  invisible(out)
}

# ---- predict ----------------------------------------------------------------

stata_predict <- function(d, e, what, cond = TRUE) {
  if (is.null(e)) stop("last estimates not found", call. = FALSE)
  X <- stata_design(d, e)
  used <- names(e$b)[e$b != 0 | diag(e$V) != 0]
  xb <- drop(X[, used, drop = FALSE] %*% e$b[used])
  if (!is.null(e$offset)) xb <- xb + stata_num(d, e$offset)
  if (!is.null(e$exposure)) xb <- xb + log(stata_num(d, e$exposure))
  out <- switch(what,
    xb = xb,
    stdp = sqrt(rowSums((X[, used, drop = FALSE] %*% e$V[used, used, drop = FALSE]) * X[, used, drop = FALSE])),
    residuals = stata_num(d, e$depvar) - xb,
    pr = if (e$family == "logit") stats::plogis(xb) else stats::pnorm(xb),
    ue = stata_num(d, e$depvar) - xb,
    n = exp(xb),
    stop("predict: ", what, " is not available after ", e$cmd, call. = FALSE))
  ok <- rep(TRUE, nrow(d)) & cond
  out[is.na(ok) | !ok] <- NA
  out
}

# ---- test and lincom --------------------------------------------------------
# Each restriction is a named vector of weights on coefficients and a value.

stata_restrictions <- function(e, R, r) {
  M <- matrix(0, length(R), length(e$b), dimnames = list(NULL, names(e$b)))
  for (i in seq_along(R)) {
    w <- R[[i]]
    bad <- setdiff(names(w), names(e$b))
    if (length(bad)) stop(bad[1], " not found", call. = FALSE)
    M[i, names(w)] <- w
  }
  list(R = M, r = r)
}

stata_test <- function(e, R, r, show = TRUE) {
  if (is.null(e)) stop("last estimates not found", call. = FALSE)
  rs <- stata_restrictions(e, R, r)
  M <- rs$R
  # restrictions on base or omitted coefficients are dropped, as Stata does
  keep <- rowSums(abs(M %*% e$V)) > 0
  if (!all(keep)) message("note: ", sum(!keep), " constraint(s) dropped (on a base or omitted coefficient)")
  M <- M[keep, , drop = FALSE]
  rr <- rs$r[keep]
  q <- nrow(M)
  if (q == 0) stop("no constraints left to test", call. = FALSE)
  # dependent restrictions are dropped too
  qrM <- qr(t(M))
  if (qrM$rank < q) {
    M <- M[qrM$pivot[seq_len(qrM$rank)], , drop = FALSE]
    rr <- rr[qrM$pivot[seq_len(qrM$rank)]]
    q <- nrow(M)
  }
  dv <- M %*% e$b - rr
  W <- drop(t(dv) %*% solve(M %*% e$V %*% t(M)) %*% dv)
  if (!is.null(e$df_r) && !is.na(e$df_r)) {
    out <- list(F = W / q, df = q, df_r = e$df_r, p = stats::pf(W / q, q, e$df_r, lower.tail = FALSE), drop = 0)
    if (show) cat(sprintf("\n       F(%3d, %6g) = %10.2f\n            Prob > F = %10.4f\n", q, e$df_r, out$F, out$p))
  } else {
    out <- list(chi2 = W, df = q, p = stats::pchisq(W, q, lower.tail = FALSE), drop = 0)
    if (show) cat(sprintf("\n           chi2(%3d) = %10.2f\n         Prob > chi2 = %10.4f\n", q, W, out$p))
  }
  invisible(out)
}

stata_lincom <- function(e, w, const = 0, eform = NULL, show = TRUE) {
  if (is.null(e)) stop("last estimates not found", call. = FALSE)
  M <- stata_restrictions(e, list(w), 0)$R
  est <- drop(M %*% e$b) + const
  se <- sqrt(drop(M %*% e$V %*% t(M)))
  stat <- est / se
  small <- !is.null(e$df_r) && !is.na(e$df_r)
  if (small) {
    p <- 2 * stats::pt(-abs(stat), e$df_r)
    crit <- stats::qt(0.975, e$df_r)
  } else {
    p <- 2 * stats::pnorm(-abs(stat))
    crit <- stats::qnorm(0.975)
  }
  out <- list(estimate = est, se = se, p = p, lb = est - crit * se, ub = est + crit * se, level = 95)
  if (small) {
    out$t <- stat
    out$df <- e$df_r
  } else {
    out$z <- stat
  }
  if (show) {
    cat(sprintf("\n %11s %10s %8s %7s  %21s\n", if (is.null(eform)) "Coefficient" else eform, "Std. err.",
                if (small) "t" else "z", "P>|t|", "[95% conf. interval]"))
    v <- if (is.null(eform)) c(est, se, out$lb, out$ub) else c(exp(est), exp(est) * se, exp(out$lb), exp(out$ub))
    cat(sprintf(" %11s %10s %8.2f %7.3f %10s %10s\n", stata_g(v[1], 11), stata_g(v[2]), stat, p, stata_g(v[3]), stata_g(v[4])))
  }
  invisible(out)
}

stata_estat_ic <- function(e, show = TRUE) {
  if (is.null(e)) stop("last estimates not found", call. = FALSE)
  df <- e$rank
  S <- matrix(c(e$N, e$ll_0, e$ll, df, -2 * e$ll + 2 * df, -2 * e$ll + log(e$N) * df), 1,
              dimnames = list(".", c("N", "ll0", "ll", "df", "AIC", "BIC")))
  if (show) {
    cat("\nAkaike's information criterion and Bayesian information criterion\n\n")
    print(S)
  }
  invisible(list(S = S))
}

# ---- summarize, tabulate, correlate, ttest ----------------------------------

stata_pct <- function(x, p) {
  # Stata's percentiles: the average of two order statistics when n*p/100 is whole
  x <- sort(x)
  n <- length(x)
  vapply(p, function(q) {
    h <- n * q / 100
    i <- floor(h)
    if (i < 1) return(x[1])
    if (i >= n) return(x[n])
    if (h == i) (x[i] + x[i + 1]) / 2 else x[i + 1]
  }, 0)
}

stata_summarize <- function(d, vars, cond = TRUE, detail = FALSE, meanonly = FALSE, show = TRUE) {
  ok <- rep(TRUE, nrow(d)) & cond
  ok[is.na(ok)] <- FALSE
  out <- list()
  if (show && !detail && !meanonly) {
    cat(sprintf("\n%12s | %10s %11s %11s %10s %10s\n", "Variable", "Obs", "Mean", "Std. dev.", "Min", "Max"))
    cat(strrep("-", 13), "+", strrep("-", 57), "\n", sep = "")
  }
  for (v in vars) {
    raw <- d[[v]]
    if (is.null(raw)) stop("variable ", v, " not found", call. = FALSE)
    x <- if (is.character(raw)) numeric(0) else as.numeric(raw)[ok]
    x <- x[!is.na(x)]
    n <- length(x)
    out <- list(N = n, sum_w = n, mean = if (n) mean(x) else NA_real_, sum = sum(x),
                min = if (n) min(x) else NA_real_, max = if (n) max(x) else NA_real_)
    if (!meanonly) {
      out$Var <- if (n > 1) stats::var(x) else NA_real_
      out$sd <- sqrt(out$Var)
    }
    if (detail && n > 0) {
      ps <- stata_pct(x, c(1, 5, 10, 25, 50, 75, 90, 95, 99))
      for (i in seq_along(ps)) out[[paste0("p", c(1, 5, 10, 25, 50, 75, 90, 95, 99)[i])]] <- ps[i]
      m2 <- mean((x - out$mean)^2)
      out$skewness <- mean((x - out$mean)^3) / m2^1.5
      out$kurtosis <- mean((x - out$mean)^4) / m2^2
    }
    if (show && !meanonly) {
      if (detail) {
        cat("\n", v, "\n", strrep("-", 61), "\n", sep = "")
        if (n > 0) {
          cat(sprintf("%12s %12s\n", c("1%", "5%", "10%", "25%", "50%", "75%", "90%", "95%", "99%"),
                      stata_g(unlist(out[c("p1", "p5", "p10", "p25", "p50", "p75", "p90", "p95", "p99")]))), sep = "")
          cat(sprintf("\n%12s %12s\n", c("Obs", "Mean", "Std. dev.", "Variance", "Skewness", "Kurtosis"),
                      stata_g(c(n, out$mean, out$sd, out$Var, out$skewness, out$kurtosis))), sep = "")
        }
      } else {
        cat(sprintf("%12s | %10d %11s %11s %10s %10s\n", substr(v, 1, 12), n, stata_g(out$mean),
                    stata_g(out$sd), stata_g(out$min), stata_g(out$max)))
      }
    }
  }
  invisible(out)
}

stata_levels <- function(x, missing) {
  if (is.character(x)) {
    keep <- missing | x != ""
  } else {
    keep <- missing | !is.na(x)
  }
  keep
}

stata_tabulate <- function(d, vars, cond = TRUE, missing = FALSE, chi2 = FALSE, lrchi2 = FALSE,
                           exact = FALSE, V = FALSE, show = TRUE) {
  ok <- rep(TRUE, nrow(d)) & cond
  ok[is.na(ok)] <- FALSE
  cols <- lapply(vars, function(v) {
    x <- d[[v]]
    if (is.null(x)) stop("variable ", v, " not found", call. = FALSE)
    x
  })
  for (x in cols) ok <- ok & stata_levels(x, missing)
  shown <- lapply(cols, function(x) {
    x <- x[ok]
    if (inherits(x, "haven_labelled")) {
      lab <- attr(x, "labels")
      v <- as.numeric(x)
      factor(ifelse(v %in% lab, names(lab)[match(v, lab)], ifelse(is.na(v), ".", format(v))),
             levels = unique(c(ifelse(sort(unique(v)) %in% lab, names(lab)[match(sort(unique(v)), lab)], format(sort(unique(v)))), ".")))
    } else {
      v <- if (is.character(x)) x else as.numeric(x)
      lv <- sort(unique(v[!is.na(v)]))
      factor(ifelse(is.na(v), ".", as.character(v)), levels = c(as.character(lv), if (anyNA(v)) "."))
    }
  })
  shown <- lapply(shown, droplevels)
  n <- sum(ok)
  if (length(vars) == 1) {
    tab <- table(shown[[1]])
    out <- list(N = n, r = length(tab))
    if (show) {
      cat(sprintf("\n%12s | %10s %10s %10s\n", substr(vars[1], 1, 12), "Freq.", "Percent", "Cum."))
      cat(strrep("-", 13), "+", strrep("-", 35), "\n", sep = "")
      cum <- cumsum(tab) / n * 100
      for (i in seq_along(tab)) cat(sprintf("%12s | %10d %10.2f %10.2f\n", substr(names(tab)[i], 1, 12), tab[[i]], tab[[i]] / n * 100, cum[[i]]))
      cat(strrep("-", 13), "+", strrep("-", 35), "\n", sep = "")
      cat(sprintf("%12s | %10d %10.2f\n", "Total", n, 100))
    }
    return(invisible(out))
  }
  tab <- table(shown[[1]], shown[[2]], dnn = vars)
  out <- list(N = n, r = nrow(tab), c = ncol(tab))
  ex <- outer(rowSums(tab), colSums(tab)) / n
  if (chi2 || V) {
    out$chi2 <- sum((tab - ex)^2 / ex)
    out$p <- stats::pchisq(out$chi2, (nrow(tab) - 1) * (ncol(tab) - 1), lower.tail = FALSE)
  }
  if (lrchi2) {
    pos <- tab > 0
    out$chi2_lr <- 2 * sum(tab[pos] * log(tab[pos] / ex[pos]))
    out$p_lr <- stats::pchisq(out$chi2_lr, (nrow(tab) - 1) * (ncol(tab) - 1), lower.tail = FALSE)
  }
  if (V) {
    out$CramersV <- if (min(dim(tab)) == 2 && length(tab) == 4) {
      (tab[1, 1] * tab[2, 2] - tab[1, 2] * tab[2, 1]) / sqrt(prod(rowSums(tab)) * prod(colSums(tab)))
    } else {
      sqrt(out$chi2 / n / (min(dim(tab)) - 1))
    }
  }
  if (exact) {
    out$p_exact <- stats::fisher.test(tab, workspace = 2e7)$p.value
    if (all(dim(tab) == 2)) out$p1_exact <- min(stats::fisher.test(tab, alternative = "less")$p.value, stats::fisher.test(tab, alternative = "greater")$p.value)
  }
  if (!chi2 && V) {
    out$chi2 <- NULL
    out$p <- NULL
  }
  if (show) {
    print(stats::addmargins(tab))
    if (!is.null(out$chi2)) cat(sprintf("\n          Pearson chi2(%d) = %8.4f   Pr = %5.3f\n", (nrow(tab) - 1) * (ncol(tab) - 1), out$chi2, out$p))
    if (!is.null(out$chi2_lr)) cat(sprintf(" likelihood-ratio chi2(%d) = %8.4f   Pr = %5.3f\n", (nrow(tab) - 1) * (ncol(tab) - 1), out$chi2_lr, out$p_lr))
    if (!is.null(out$CramersV)) cat(sprintf("               Cramer's V = %8.4f\n", out$CramersV))
    if (!is.null(out$p_exact)) cat(sprintf("           Fisher's exact =                 %5.3f\n", out$p_exact))
  }
  invisible(out)
}

stata_correlate <- function(d, vars, cond = TRUE, covariance = FALSE, show = TRUE) {
  ok <- rep(TRUE, nrow(d)) & cond
  ok[is.na(ok)] <- FALSE
  X <- vapply(vars, function(v) stata_num(d, v), numeric(nrow(d)))
  X <- matrix(X, nrow = nrow(d), dimnames = list(NULL, vars))
  ok <- ok & stats::complete.cases(X)
  X <- X[ok, , drop = FALSE]
  n <- nrow(X)
  C <- if (covariance) stats::cov(X) else stats::cor(X)
  out <- list(N = n, C = C)
  if (length(vars) >= 2) {
    if (covariance) {
      out$cov_12 <- C[1, 2]
      out$Var_1 <- C[1, 1]
      out$Var_2 <- C[2, 2]
    } else {
      out$rho <- C[1, 2]
    }
  }
  if (show) {
    cat("(obs=", n, ")\n\n", sep = "")
    print(round(C, 4))
  }
  invisible(out)
}

stata_ttest_print <- function(out, h0) {
  cat(sprintf("\n t = %9.4f   degrees of freedom = %g\n", out$t, out$df_t))
  cat(sprintf(" Ha: diff < %s   Pr(T < t) = %.4f\n Ha: diff != %s  Pr(|T| > |t|) = %.4f\n Ha: diff > %s   Pr(T > t) = %.4f\n",
              h0, out$p_l, h0, out$p, h0, out$p_u))
}

stata_ttest <- function(d, x, cond = TRUE, mu = NULL, y = NULL, by = NULL, unequal = FALSE,
                        welch = FALSE, unpaired = FALSE, show = TRUE) {
  ok <- rep(TRUE, nrow(d)) & cond
  ok[is.na(ok)] <- FALSE
  xv <- stata_num(d, x)
  finish <- function(out, t, df) {
    out$t <- t
    out$df_t <- df
    out$p_l <- stats::pt(t, df)
    out$p_u <- stats::pt(t, df, lower.tail = FALSE)
    out$p <- 2 * stats::pt(-abs(t), df)
    out$level <- 95
    out
  }
  if (!is.null(mu)) {
    v <- xv[ok & !is.na(xv)]
    n <- length(v)
    se <- stats::sd(v) / sqrt(n)
    out <- finish(list(N_1 = n, mu_1 = mean(v), sd_1 = stats::sd(v), se = se, sd = stats::sd(v)),
                  (mean(v) - mu) / se, n - 1)
    if (show) {
      cat(sprintf("\nOne-sample t test\n%8s | %7d %10s %10s %10s\n", x, n, stata_g(mean(v)), stata_g(se), stata_g(stats::sd(v))))
      stata_ttest_print(out, format(mu))
    }
    return(invisible(out))
  }
  if (!is.null(y) && !unpaired) {
    yv <- stata_num(d, y)
    keep <- ok & !is.na(xv) & !is.na(yv)
    dif <- xv[keep] - yv[keep]
    n <- length(dif)
    se <- stats::sd(dif) / sqrt(n)
    out <- finish(list(N_1 = n, N_2 = n, mu_1 = mean(xv[keep]), mu_2 = mean(yv[keep]),
                       sd_1 = stats::sd(xv[keep]), sd_2 = stats::sd(yv[keep]), se = se, mu_diff = mean(dif),
                       sd_diff = stats::sd(dif)),
                  mean(dif) / se, n - 1)
    if (show) {
      cat("\nPaired t test\n")
      stata_ttest_print(out, "0")
    }
    return(invisible(out))
  }
  if (!is.null(by)) {
    g <- d[[by]]
    if (is.null(g)) stop("variable ", by, " not found", call. = FALSE)
    keep <- ok & !is.na(xv) & !stata_missing(g)
    lv <- sort(unique(g[keep]))
    if (length(lv) != 2) stop("more than 2 groups found, only 2 allowed", call. = FALSE)
    a <- xv[keep & g == lv[1]]
    b <- xv[keep & g == lv[2]]
  } else {
    yv <- stata_num(d, y)
    a <- xv[ok & !is.na(xv)]
    b <- yv[ok & !is.na(yv)]
  }
  n1 <- length(a)
  n2 <- length(b)
  v1 <- stats::var(a)
  v2 <- stats::var(b)
  if (unequal || welch) {
    se <- sqrt(v1 / n1 + v2 / n2)
    df <- if (welch) -2 + (v1 / n1 + v2 / n2)^2 / ((v1 / n1)^2 / (n1 + 1) + (v2 / n2)^2 / (n2 + 1))
          else (v1 / n1 + v2 / n2)^2 / ((v1 / n1)^2 / (n1 - 1) + (v2 / n2)^2 / (n2 - 1))
  } else {
    sp <- ((n1 - 1) * v1 + (n2 - 1) * v2) / (n1 + n2 - 2)
    se <- sqrt(sp * (1 / n1 + 1 / n2))
    df <- n1 + n2 - 2
  }
  out <- finish(list(N_1 = n1, N_2 = n2, mu_1 = mean(a), mu_2 = mean(b), sd_1 = sqrt(v1), sd_2 = sqrt(v2), se = se),
                (mean(a) - mean(b)) / se, df)
  if (show) {
    cat(if (unequal || welch) "\nTwo-sample t test with unequal variances\n" else "\nTwo-sample t test with equal variances\n")
    stata_ttest_print(out, "0")
  }
  invisible(out)
}
