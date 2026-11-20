# sspace, ucm and dfactor: linear Gaussian state-space models fitted by maximum likelihood.
# States x_t = A x_{t-1} + C e_t, observations y_t = off_t + D x_t + v_t, with diagonal Var(e) = Q and
# Var(v) = R. A stationary model starts from the states' unconditional variance, a model with a unit root
# fixed by constraints (and every ucm trend) starts diffuse: the exact diffuse filter, one observation at a
# time, each diffuse step adding -ln(2 pi)/2 to the log likelihood.
# The three builders (sspace, ucm, dfactor) describe a model by its parameters and entries; stata_ss_fit
# fits it, prints it and keeps what predict needs.
stata_ss_unconditional <- function(A, CQC) {
  m <- nrow(A)
  p0 <- tryCatch(solve(diag(m * m) - kronecker(A, A), as.vector(CQC)), error = function(e) NULL)
  if (is.null(p0) || any(!is.finite(p0))) NULL else matrix(p0, m, m)
}
stata_ss_stationary <- function(A) {
  ev <- tryCatch(eigen(A, only.values = TRUE)$values, error = function(e) NULL)
  !is.null(ev) && max(Mod(ev)) < 1 - 1e-9
}
# the Kalman filter: the log likelihood, the one-step predictions of the observations and the states
# (NA where diffuse), a(t|t-1) and the filtered states a(t|t)
stata_ss_filter <- function(M, y, off, diffuse) {
  A <- M$A; D <- M$D; R <- M$R
  m <- nrow(A); p <- nrow(D); n <- nrow(y)
  CQC <- M$C %*% M$Q %*% t(M$C)
  Dt <- t(D)
  At <- t(A)
  a <- numeric(m)
  if (diffuse) {
    P <- matrix(0, m, m)
    Pinf <- diag(m)
  } else {
    P <- stata_ss_unconditional(A, CQC)
    if (is.null(P)) return(NULL)
    Pinf <- matrix(0, m, m)
  }
  ll <- 0
  l2pi <- log(2 * pi)
  pred <- matrix(NA_real_, n, p)
  one <- matrix(NA_real_, n, m)
  st <- matrix(NA_real_, n, m)
  for (t in seq_len(n)) {
    yhat <- drop(D %*% a) + off[t, ]
    v <- y[t, ] - yhat
    Finf <- D %*% Pinf %*% Dt
    dstep <- any(abs(Finf) > 1e-9)
    if (!dstep) {
      pred[t, ] <- yhat
      one[t, ] <- a
    }
    if (dstep) {
      if (p != 1) return(NULL)
      fi <- Finf[1, 1]
      fs <- (D %*% P %*% Dt)[1, 1] + R[1, 1]
      mi <- drop(Pinf %*% Dt)
      ms <- drop(P %*% Dt)
      kinf <- mi / fi
      a <- a + kinf * v
      P <- P + outer(kinf, kinf) * fs - (outer(kinf, ms) + outer(ms, kinf))
      Pinf <- Pinf - outer(kinf, mi)
      ll <- ll - 0.5 * l2pi
    } else {
      Fm <- D %*% P %*% Dt + R
      ch <- tryCatch(chol(Fm), error = function(e) NULL)
      if (is.null(ch)) return(NULL)
      Fi <- chol2inv(ch)
      quad <- sum(v * drop(Fi %*% v))
      ll <- ll - 0.5 * (p * l2pi + 2 * sum(log(diag(ch))) + quad)
      PDt <- P %*% Dt
      K <- PDt %*% Fi
      a <- a + drop(K %*% v)
      P <- P - K %*% t(PDt)
    }
    # the filtered states are missing at every diffuse step (Stata 19: two for a local linear trend)
    if (!dstep) st[t, ] <- a
    a <- drop(A %*% a)
    P <- A %*% P %*% At + CQC
    Pinf <- A %*% Pinf %*% At
  }
  if (!is.finite(ll)) NULL else list(ll = ll, pred = pred, one = one, st = st)
}
# the model's matrices at the parameter values th (an entry is a parameter k > 0 or the fixed value v)
stata_ss_model <- function(spec, th) {
  val <- function(k, v) ifelse(k > 0, th[pmax(k, 1)], v)
  m <- spec$m; p <- spec$p; q <- spec$nsh
  A <- matrix(0, m, m)
  if (nrow(spec$A)) for (r in seq_len(nrow(spec$A))) { e <- spec$A[r, ]; A[e[1], e[2]] <- val(e[3], e[4]) }
  C <- matrix(0, m, q)
  if (nrow(spec$C)) for (r in seq_len(nrow(spec$C))) { e <- spec$C[r, ]; C[e[1], e[2]] <- e[4] }
  Q <- matrix(0, q, q)
  if (nrow(spec$Q)) for (r in seq_len(nrow(spec$Q))) { e <- spec$Q[r, ]; Q[e[1], e[1]] <- val(e[3], e[4]) }
  D <- matrix(0, p, m)
  if (nrow(spec$D)) for (r in seq_len(nrow(spec$D))) { e <- spec$D[r, ]; D[e[1], e[2]] <- val(e[3], e[4]) }
  R <- matrix(0, p, p)
  if (nrow(spec$R)) for (r in seq_len(nrow(spec$R))) { e <- spec$R[r, ]; R[e[1], e[1]] <- val(e[3], e[4]) }
  off <- matrix(0, nrow(spec$y), p)
  for (x in spec$exog) off[, x$i] <- off[, x$i] + th[x$k] * x$data
  list(A = A, C = C, Q = Q, D = D, R = R, off = off)
}
stata_ss_run <- function(spec, th) {
  par <- spec$par
  bad <- par$variance & is.na(par$fixed) & th <= 0
  if (any(bad) || any(!is.finite(th))) return(NULL)
  M <- stata_ss_model(spec, th)
  if (!spec$diffuse && !stata_ss_stationary(M$A)) return(NULL)
  stata_ss_filter(M, spec$y, M$off, spec$diffuse)
}
stata_ss_loglik <- function(spec, th) {
  f <- stata_ss_run(spec, th)
  if (is.null(f)) -Inf else f$ll
}
# StatCraft's search: BFGS then Newton on the working scale (variances as logarithms)
stata_ss_fit_theta <- function(spec, start) {
  par <- spec$par
  free <- which(is.na(par$fixed))
  isvar <- par$variance[free]
  theta_of <- function(phi) {
    th <- ifelse(is.na(par$fixed), 0, par$fixed)
    th[free] <- ifelse(isvar, exp(phi), phi)
    th
  }
  f <- function(phi) stata_ss_loglik(spec, theta_of(phi))
  phi0 <- ifelse(isvar, log(pmax(start[free], 1e-8)), start[free])
  if (!is.finite(f(phi0))) stop("could not calculate the likelihood at the initial values (Stata: r(459))", call. = FALSE)
  grad <- function(fn, x, scale) {
    vapply(seq_along(x), function(i) {
      h <- scale * max(abs(x[i]), 1)
      up <- x; dn <- x
      up[i] <- up[i] + h
      dn[i] <- dn[i] - h
      (fn(up) - fn(dn)) / (2 * h)
    }, 0)
  }
  hess <- function(fn, x, scale) {
    n <- length(x)
    step <- function(i) scale * (abs(x[i]) + 0.01)
    f0 <- fn(x)
    sh <- function(i, di, j = 0, dj = 0) { y <- x; y[i] <- y[i] + di; if (j) y[j] <- y[j] + dj; fn(y) }
    H <- matrix(0, n, n)
    for (i in seq_len(n)) {
      hi <- step(i)
      H[i, i] <- (sh(i, hi) - 2 * f0 + sh(i, -hi)) / (hi * hi)
      for (j in seq_len(i - 1)) {
        hj <- step(j)
        v <- (sh(i, hi, j, hj) - sh(i, hi, j, -hj) - sh(i, -hi, j, hj) + sh(i, -hi, j, -hj)) / (4 * hi * hj)
        H[i, j] <- v
        H[j, i] <- v
      }
    }
    H
  }
  solve_ <- function(a, b) tryCatch(solve(a, b), error = function(e) NULL)
  n <- length(phi0)
  x <- phi0
  fx <- f(x)
  hinv <- diag(0.1, n)
  g <- grad(f, x, 1e-6)
  for (it in 1:300) {
    d <- drop(hinv %*% g)
    tt <- 1
    nxt <- x
    fnext <- -Inf
    for (k in 1:60) {
      nxt <- x + tt * d
      fnext <- f(nxt)
      if (is.finite(fnext) && fnext >= fx) break
      tt <- tt * 0.5
    }
    if (!is.finite(fnext) || fnext < fx) break
    gn <- grad(f, nxt, 1e-6)
    s <- nxt - x
    yv <- -(gn - g)
    sy <- sum(s * yv)
    change <- fnext - fx
    x <- nxt
    fx <- fnext
    g <- gn
    if (sy > 1e-14) {
      hy <- drop(hinv %*% yv)
      yhy <- sum(yv * hy)
      hinv <- hinv + (sy + yhy) * outer(s, s) / (sy * sy) - (outer(hy, s) + outer(s, hy)) / sy
    }
    # the gradient's differences keep it near 1e-6, so a step that gains nothing ends BFGS; Newton polishes
    if (change == 0 || (abs(change) < 1e-11 && max(abs(g)) < 1e-4)) break
  }
  converged <- FALSE
  for (it in 1:30) {
    h <- hess(f, x, 1e-4)
    gg <- grad(f, x, 1e-6)
    step <- solve_(-h, gg)
    if (is.null(step)) break
    tt <- 1
    moved <- FALSE
    for (k in 1:30) {
      nxt <- x + tt * step
      fnext <- f(nxt)
      if (is.finite(fnext) && fnext >= fx - 1e-12) {
        gain <- fnext - fx
        x <- nxt
        fx <- fnext
        moved <- TRUE
        if (abs(gain) < 1e-12) converged <- TRUE
        break
      }
      tt <- tt * 0.5
    }
    size <- max(abs(step))
    if (!moved || converged || size < 1e-10) {
      converged <- converged || size < 1e-7
      break
    }
  }
  theta <- theta_of(x)
  ll <- stata_ss_loglik(spec, theta)
  # the observed information on the natural scale, by central differences at two steps, extrapolated
  g2 <- function(xx) { th <- theta; th[free] <- xx; stata_ss_loglik(spec, th) }
  x0 <- theta[free]
  h1 <- hess(g2, x0, 1.2e-3)
  h2 <- hess(g2, x0, 6e-4)
  H <- (4 * h2 - h1) / 3
  inv <- tryCatch(solve(-H), error = function(e) NULL)
  if (is.null(inv) || any(!is.finite(inv))) stop("Hessian is not negative semidefinite (Stata: r(430))", call. = FALSE)
  V <- matrix(0, length(theta), length(theta))
  V[free, free] <- inv
  list(theta = theta, V = V, ll = ll, converged = converged)
}
# the number as Stata's %9.0g shows it, for the table
stata_ss_g9 <- function(v) trimws(stata_fmt(v, "%9.0g"))
# Stata's table, in R: sections of rows (heading or NULL, parameter indices), constrained rows, one-sided
# tests of the variances
stata_ss_table <- function(spec, est, sections, depvar, level = 0.95) {
  par <- spec$par
  crit <- stats::qnorm(1 - (1 - level) / 2)
  rule <- strrep("-", 78)
  mid <- paste0(strrep("-", 13), "+", strrep("-", 64))
  cat(rule, "\n", sep = "")
  cat(sprintf("%12s | Coefficient  Std. err.      z    P>|z|     [%d%% conf. interval]\n", depvar, round(level * 100)))
  num <- function(x, w) sprintf(paste0("%", w, "s"), if (is.finite(x)) stata_ss_g9(x) else ".")
  for (s in sections) {
    cat(mid, "\n", sep = "")
    if (!is.null(s$heading)) cat(sprintf("%-13s|\n", s$heading))
    last <- NULL
    for (i in s$rows) {
      b <- est$theta[i]
      se <- sqrt(max(est$V[i, i], 0))
      lab <- par$label[[i]]
      if (lab$kind == "lag") {
        if (!identical(last, lab$base)) { cat(sprintf("%12s |\n", substr(lab$base, 1, 12))); last <- lab$base }
        pre <- sprintf("%12s |", paste0("L", lab$k, "."))
      } else if (lab$kind == "var") {
        last <- NULL
        pre <- sprintf("%13s|", paste0("var(", lab$name, ")"))
      } else {
        last <- NULL
        pre <- sprintf("%12s |", substr(lab$name, 1, 12))
      }
      if (!is.na(par$fixed[i])) { cat(pre, sprintf("%11s", stata_ss_g9(b)), "  (constrained)\n", sep = ""); next }
      z <- b / se
      if (par$variance[i]) { pv <- stats::pnorm(-z); lo <- max(b - crit * se, 0) } else { pv <- 2 * stats::pnorm(-abs(z)); lo <- b - crit * se }
      hi <- b + crit * se
      ok <- is.finite(z) && se > 0
      cat(pre, num(b, 11), if (ok) num(se, 11) else sprintf("%11s", "."),
          if (ok) sprintf("%9.2f", z) else sprintf("%9s", "."), if (ok) sprintf("%8.3f", pv) else sprintf("%8s", "."),
          if (ok) num(lo, 13) else sprintf("%13s", "."), if (ok) num(hi, 12) else sprintf("%12s", "."), "\n", sep = "")
    }
  }
  cat(rule, "\n", sep = "")
}
# the Wald test of the coefficients that are neither constrained, constants nor variances
stata_ss_wald <- function(spec, est) {
  par <- spec$par
  idx <- which(!par$variance & is.na(par$fixed) & par$term != "_cons")
  if (!length(idx)) return(list(chi2 = NA_real_, df = 0, p = NA_real_))
  iv <- tryCatch(solve(est$V[idx, idx, drop = FALSE]), error = function(e) NULL)
  if (is.null(iv)) return(list(chi2 = NA_real_, df = length(idx), p = NA_real_))
  b <- est$theta[idx]
  chi2 <- drop(t(b) %*% iv %*% b)
  list(chi2 = chi2, df = length(idx), p = stats::pchisq(chi2, length(idx), lower.tail = FALSE))
}
# the data: the complete periods, which must follow each other
stata_ss_data <- function(d, vars, time, delta) {
  stop_ <- function(...) stop(..., call. = FALSE)
  miss <- setdiff(c(vars, time), names(d))
  if (length(miss)) stop_("variable ", miss[1], " not found (Stata: r(111))")
  num <- function(v) { v <- unclass(v); attributes(v) <- NULL; as.numeric(v) }
  tm <- num(d[[time]]) / delta
  rows <- which(!is.na(tm))
  rows <- rows[order(tm[rows], rows)]
  tt <- round(tm[rows])
  if (!length(rows)) stop_("no observations (Stata: r(2000))")
  if (anyDuplicated(tt)) stop_("repeated time values in sample (Stata: r(451))")
  Y <- vapply(vars, function(v) num(d[[v]])[rows], numeric(length(rows)))
  Y <- matrix(Y, length(rows), length(vars))
  use <- rowSums(is.na(Y)) == 0
  w <- which(use)
  if (!length(w)) stop_("no observations (Stata: r(2000))")
  if (!all(use[w[1]:w[length(w)]]) || any(diff(tt[w]) != 1)) {
    stop_("a state-space model with missing values or gaps inside the sample is not translated yet")
  }
  list(Y = Y[w, , drop = FALSE], rows = rows[w], tmu = tt[w] * delta, n = length(w))
}
stata_ss_var <- function(x) { n <- max(length(x), 2); mean_ <- sum(x) / n; sum((x - mean_)^2) / (n - 1) }
# an empty parameter set and entries
stata_ss_new <- function(y, diffuse = FALSE) {
  list(par = list(eq = character(0), term = character(0), variance = logical(0), fixed = numeric(0), start = numeric(0), label = list()),
       m = 0, p = ncol(y), nsh = 0, A = matrix(0, 0, 4), C = matrix(0, 0, 4), Q = matrix(0, 0, 4), D = matrix(0, 0, 4),
       R = matrix(0, 0, 4), exog = list(), y = y, diffuse = diffuse, state_names = character(0))
}
stata_ss_param <- function(spec, eq, term, variance, fixed, start, label) {
  k <- length(spec$par$eq) + 1
  spec$par$eq[k] <- eq
  spec$par$term[k] <- term
  spec$par$variance[k] <- variance
  spec$par$fixed[k] <- if (is.na(fixed)) NA_real_ else fixed
  spec$par$start[k] <- if (is.na(fixed)) start else fixed
  spec$par$label[[k]] <- label
  list(spec = spec, k = k)
}
stata_ss_lab <- function(kind, name = "", base = "", k = 1) list(kind = kind, name = name, base = base, k = k)
# fits the model and prints it; `sections` and the e() pieces come from the builder
stata_ss_fit <- function(spec, time, tmu, rows, nobs, sections, depvar, title, header, constraints = character(0),
                         level = 0.95, show = TRUE, tmfmt = NULL, log = TRUE) {
  est <- stata_ss_fit_theta(spec, spec$par$start)
  par <- spec$par
  w <- stata_ss_wald(spec, est)
  fmt_t <- function(v) if (is.null(tmfmt)) format(v, digits = 15) else trimws(stata_fmt(v, tmfmt))
  n <- nobs
  if (show) {
    if (log && !est$converged) cat("convergence not achieved\n")
    cat("\n", title, "\n", sep = "")
    for (h in header) cat(h, "\n", sep = "")
    cat("\n")
    sample <- paste0("Sample: ", fmt_t(tmu[1]), " thru ", fmt_t(tmu[n]))
    ll <- trimws(stata_fmt(est$ll, "%10.0g"))
    if (w$df > 0) {
      cat(sprintf("%-56sNumber of obs = %6d\n", sample, n))
      cat(sprintf("%-56s%-14s= %6s\n", "", paste0("Wald chi2(", w$df, ")"), sprintf("%.2f", w$chi2)))
      cat(sprintf("%-56sProb > chi2   = %s\n", paste0("Log likelihood = ", ll), sprintf("%.4f", w$p)))
    } else {
      right <- paste0("Number of obs = ", n)
      cat(sample, strrep(" ", max(78 - nchar(sample) - nchar(right), 0)), right, "\n", sep = "")
      cat("Log likelihood = ", ll, "\n", sep = "")
    }
    for (k in seq_along(constraints)) cat(sprintf(" (%2d)  %s\n", k, constraints[k]))
    stata_ss_table(spec, est, sections, depvar, level)
    if (spec$diffuse) cat("Note: Model is not stationary.\n")
    cat("Note: Tests of variances against zero are one sided, and the two-sided\n")
    cat("      confidence intervals are truncated at zero.\n")
    if (!est$converged) cat("Note: Convergence not achieved.\n")
  }
  cn <- paste0(par$eq, ":", par$term)
  b <- stats::setNames(est$theta, cn)
  V <- est$V
  dimnames(V) <- list(cn, cn)
  filtered <- stata_ss_run(spec, est$theta)
  structure(list(b = b, vcov = V, df = Inf, rows = rows, est = est, wald = w, spec = spec, y = par$eq[1],
                 tmins = fmt_t(tmu[1]), tmaxs = fmt_t(tmu[n]), tmin = tmu[1], tmax = tmu[n], n = n, timevar = time,
                 predict_ok = !is.null(filtered)), class = c("stata_ss", "stata_arima"))
}
# sspace (state equations) (observation equations), constraints()
stata_sspace <- function(d, time, eqs, cons, delta = 1, level = 0.95, show = TRUE, tmfmt = NULL, log = TRUE) {
  stop_ <- function(...) stop(..., call. = FALSE)
  states <- Filter(function(e) e$state, eqs)
  obs <- Filter(function(e) !e$state, eqs)
  if (!length(states) || !length(obs)) stop_("you must specify at least one state equation and one observation equation (Stata: r(198))")
  snames <- vapply(states, function(e) e$name, "")
  onames <- vapply(obs, function(e) e$name, "")
  if (anyDuplicated(c(snames, onames))) stop_("equation names must be different (Stata: r(198))")
  sidx <- function(t) match(t, snames)
  vars <- onames
  for (e in obs) for (t in e$terms) if (is.na(sidx(t)) && !t %in% vars) vars <- c(vars, t)
  for (e in states) {
    if (e$constant) stop_("state equations with a constant are not translated yet; specify noconstant")
    for (t in e$terms) if (is.na(sidx(sub("^.*\\.", "", t)))) stop_(t, ": exogenous variables in state equations are not translated yet")
  }
  dat <- stata_ss_data(d, vars, time, delta)
  col <- function(name) dat$Y[, match(name, vars)]
  # the constraints [eq]term = value
  parsed <- lapply(cons, function(tx) {
    m <- regmatches(tx, regexec("^\\s*\\[([^]]+)\\]\\s*([^=]+?)\\s*=\\s*([-+0-9.eE]+)\\s*$", tx))[[1]]
    if (length(m) != 4 || is.na(suppressWarnings(as.numeric(m[4])))) {
      stop_("the constraint '", tx, "' is not one sspace translates (only [equation]term = number)")
    }
    list(eq = m[2], term = m[3], value = as.numeric(m[4]))
  })
  used <- rep(FALSE, length(parsed))
  fixed_at <- function(eq, term) {
    for (i in seq_along(parsed)) {
      pc <- parsed[[i]]
      if (pc$eq == eq && (pc$term == term || sub("L1\\.", "L.", pc$term) == term)) { used[i] <<- TRUE; return(pc$value) }
    }
    NA_real_
  }
  spec <- stata_ss_new(matrix(0, dat$n, length(onames)))
  spec$y <- dat$Y[, seq_along(onames), drop = FALSE]
  spec$m <- length(snames)
  spec$state_names <- snames
  add <- function(eq, term, variance, fixed, start, label) {
    r <- stata_ss_param(spec, eq, term, variance, fixed, start, label)
    spec <<- r$spec
    r$k
  }
  ent <- function(M, ...) rbind(M, c(...))
  # state equations: the first lags
  for (i in seq_along(states)) {
    e <- states[[i]]
    for (t in e$terms) {
      ops <- sub("\\.[^.]*$", "", t)
      base <- sub("^.*\\.", "", t)
      lag <- if (ops == "L") 1 else if (grepl("^L[0-9]+$", ops)) as.numeric(substring(ops, 2)) else 0
      if (lag != 1) stop_(t, ": only first lags of the states are translated yet")
      j <- sidx(base)
      term <- paste0("L.", base)
      k <- add(e$name, term, FALSE, fixed_at(e$name, term), 0.5, stata_ss_lab("lag", base = base, k = 1))
      spec$A <- ent(spec$A, i, j, k, 0)
      if (!is.na(spec$par$fixed[k])) spec$A[nrow(spec$A), ] <- c(i, j, 0, spec$par$fixed[k])
    }
  }
  ysd <- vapply(onames, function(nm) stata_ss_var(col(nm)), 0)
  for (i in seq_along(obs)) {
    e <- obs[[i]]
    for (t in e$terms) {
      fx <- fixed_at(e$name, t)
      isstate <- !is.na(sidx(t))
      k <- add(e$name, t, FALSE, fx, if (isstate) 1 else 0, stata_ss_lab("plain", name = t))
      if (isstate) {
        spec$D <- ent(spec$D, i, sidx(t), if (is.na(fx)) k else 0, if (is.na(fx)) 0 else fx)
      } else {
        spec$exog[[length(spec$exog) + 1]] <- list(i = i, data = col(t), k = k)
      }
    }
    if (e$constant) {
      y <- col(e$name)
      k <- add(e$name, "_cons", FALSE, fixed_at(e$name, "_cons"), sum(y) / max(length(y), 1), stata_ss_lab("plain", name = "_cons"))
      spec$exog[[length(spec$exog) + 1]] <- list(i = i, data = rep(1, dat$n), k = k)
    }
  }
  if (any(!used)) stop_("the constraint '", cons[which(!used)[1]], "' does not apply to a parameter of this model (Stata: r(198))")
  scale <- max(max(ysd), 1e-6)
  nstate_err <- 0
  for (i in seq_along(states)) {
    e <- states[[i]]
    if (e$error) {
      nstate_err <- nstate_err + 1
      spec$nsh <- spec$nsh + 1
      spec$C <- ent(spec$C, i, spec$nsh, 0, 1)
      k <- add("/state", paste0("var(", e$name, ")"), TRUE, NA, scale / 2, stata_ss_lab("var", name = e$name))
      spec$Q <- ent(spec$Q, spec$nsh, spec$nsh, k, 0)
    }
  }
  nobs_err <- 0
  for (i in seq_along(obs)) {
    e <- obs[[i]]
    if (e$error) {
      nobs_err <- nobs_err + 1
      k <- add("/observable", paste0("var(", e$name, ")"), TRUE, NA, ysd[i] / 2, stata_ss_lab("var", name = e$name))
      spec$R <- ent(spec$R, i, i, k, 0)
    }
  }
  # a unit root fixed by the constraints makes the model diffuse
  th0 <- spec$par$start
  a_free <- nrow(spec$A) > 0 && any(spec$A[, 3] > 0)
  spec$diffuse <- !a_free && !stata_ss_stationary(stata_ss_model(spec, th0)$A)
  par <- spec$par
  sections <- list()
  eqrows <- function(eq) which(par$eq == eq & !par$variance)
  for (nm in c(snames, onames)) {
    r <- eqrows(nm)
    if (length(r)) sections[[length(sections) + 1]] <- list(heading = nm, rows = r)
  }
  for (v in c("/state", "/observable")) {
    r <- which(par$eq == v)
    if (length(r)) sections[[length(sections) + 1]] <- list(heading = v, rows = r)
  }
  eqn <- c(Filter(function(nm) length(eqrows(nm)) > 0, c(snames, onames)), "/state", "/observable")
  fit <- stata_ss_fit(spec, time, dat$tmu, dat$rows, dat$n, sections, onames[1], "State-space model", character(0),
                      constraints = vapply(cons, function(tx) paste(strsplit(trimws(tx), "[[:space:]]+")[[1]], collapse = " "), ""),
                      level = level, show = show, tmfmt = tmfmt, log = log)
  fit$cmd <- "sspace"
  fit$eqn <- eqn
  fit$sections <- length(sections)
  fit$obs <- onames
  fit$states <- snames
  fit$extra <- list(k_obser = length(obs), k_state = length(states), k_state_err = nstate_err, k_obser_err = nobs_err,
                    state_deps = snames, obser_deps = onames)
  fit
}
# ucm depvar [indepvars], model(llevel | rwalk | lltrend)
stata_ucm <- function(d, time, y, xs, model, delta = 1, level = 0.95, show = TRUE, tmfmt = NULL, log = TRUE) {
  stop_ <- function(...) stop(..., call. = FALSE)
  dat <- stata_ss_data(d, c(y, xs), time, delta)
  yy <- dat$Y[, 1]
  comps <- switch(model, llevel = "local level", rwalk = "random walk", lltrend = "local linear trend")
  slope <- model == "lltrend"
  noise <- model != "rwalk"
  vy <- stata_ss_var(yy)
  dy <- diff(yy)
  vdy <- max(stata_ss_var(dy), 1e-6)
  spec <- stata_ss_new(dat$Y[, 1, drop = FALSE], diffuse = TRUE)
  spec$state_names <- c("level", if (slope) "slope")
  add <- function(eq, term, variance, start, label) {
    r <- stata_ss_param(spec, eq, term, variance, NA_real_, start, label)
    spec <<- r$spec
    r$k
  }
  ent <- function(M, ...) rbind(M, c(...))
  for (k in seq_along(xs)) {
    kk <- add(y, xs[k], FALSE, 0, stata_ss_lab("plain", name = xs[k]))
    spec$exog[[length(spec$exog) + 1]] <- list(i = 1, data = dat$Y[, k + 1], k = kk)
  }
  spec$m <- if (slope) 2 else 1
  spec$A <- ent(spec$A, 1, 1, 0, 1)
  spec$D <- ent(spec$D, 1, 1, 0, 1)
  if (slope) { spec$A <- ent(spec$A, 1, 2, 0, 1); spec$A <- ent(spec$A, 2, 2, 0, 1) }
  for (k in seq_len(spec$m)) {
    comp <- c("level", "slope")[k]
    kk <- add("/", paste0("var(", comp, ")"), TRUE, if (k == 1) vdy / 2 else vdy / 100, stata_ss_lab("var", name = comp))
    spec$C <- ent(spec$C, k, k, 0, 1)
    spec$nsh <- spec$nsh + 1
    spec$Q <- ent(spec$Q, k, k, kk, 0)
  }
  if (noise) {
    kk <- add("/", paste0("var(", y, ")"), TRUE, max(vy, 1e-6) / 4, stata_ss_lab("var", name = y))
    spec$R <- ent(spec$R, 1, 1, kk, 0)
  }
  par <- spec$par
  sections <- list()
  co <- which(!par$variance)
  if (length(co)) sections[[1]] <- list(heading = NULL, rows = co)
  sections[[length(sections) + 1]] <- list(heading = NULL, rows = which(vapply(par$label, function(l) l$kind == "var" && l$name %in% c("level", "slope"), TRUE)))
  er <- which(vapply(par$label, function(l) l$kind == "var" && l$name == y, TRUE))
  if (length(er)) sections[[length(sections) + 1]] <- list(heading = NULL, rows = er)
  fit <- stata_ss_fit(spec, time, dat$tmu, dat$rows, dat$n, sections, y, "Unobserved-components model", paste0("Components: ", comps),
                      level = level, show = show, tmfmt = tmfmt, log = log)
  fit$cmd <- "ucm"
  fit$model <- model
  fit$obs <- y
  fit$eqn <- "/"
  fit$extra <- list(k_cycles = 0)
  fit
}
# dfactor (y1 y2 = , noconstant [ar(1)]) [(f = , ar(1))]
stata_dfactor <- function(d, time, ys, factor = NULL, f_ar = 0, o_ar = 0, delta = 1, level = 0.95, show = TRUE, tmfmt = NULL, log = TRUE) {
  dat <- stata_ss_data(d, ys, time, delta)
  k <- length(ys)
  spec <- stata_ss_new(dat$Y, diffuse = FALSE)
  add <- function(eq, term, variance, start, label) {
    r <- stata_ss_param(spec, eq, term, variance, NA_real_, start, label)
    spec <<- r$spec
    r$k
  }
  ent <- function(M, ...) rbind(M, c(...))
  sections <- list()
  if (!is.null(factor)) {
    spec$m <- 1
    spec$state_names <- factor
    spec$nsh <- 1
    spec$C <- ent(spec$C, 1, 1, 0, 1)
    spec$Q <- ent(spec$Q, 1, 1, 0, 1)
    if (f_ar == 1) {
      kk <- add(factor, paste0("L.", factor), FALSE, 0.5, stata_ss_lab("lag", base = factor, k = 1))
      spec$A <- ent(spec$A, 1, 1, kk, 0)
      sections[[length(sections) + 1]] <- list(heading = factor, rows = kk)
    }
    for (i in seq_len(k)) {
      kk <- add(ys[i], factor, FALSE, 0.5, stata_ss_lab("plain", name = factor))
      spec$D <- ent(spec$D, i, 1, kk, 0)
      sections[[length(sections) + 1]] <- list(heading = ys[i], rows = kk)
    }
  }
  if (o_ar == 1) {
    base <- spec$m
    for (i in seq_len(k)) {
      s <- base + i
      en <- paste0("e.", ys[i])
      spec$state_names <- c(spec$state_names, en)
      kk <- add(en, paste0("L.", en), FALSE, 0.3, stata_ss_lab("lag", base = en, k = 1))
      spec$A <- ent(spec$A, s, s, kk, 0)
      spec$D <- ent(spec$D, i, s, 0, 1)
      sections[[length(sections) + 1]] <- list(heading = en, rows = kk)
    }
    spec$m <- base + k
  }
  vrows <- integer(0)
  for (i in seq_len(k)) {
    v <- stata_ss_var(dat$Y[, i])
    kk <- add("/observable", paste0("var(e.", ys[i], ")"), TRUE, max(v, 1e-6) / 2, stata_ss_lab("var", name = paste0("e.", ys[i])))
    vrows <- c(vrows, kk)
    if (o_ar == 1) {
      spec$nsh <- spec$nsh + 1
      spec$C <- ent(spec$C, spec$m - k + i, spec$nsh, 0, 1)
      spec$Q <- ent(spec$Q, spec$nsh, spec$nsh, kk, 0)
    } else {
      spec$R <- ent(spec$R, i, i, kk, 0)
    }
  }
  sections[[length(sections) + 1]] <- list(heading = "/observable", rows = vrows)
  fit <- stata_ss_fit(spec, time, dat$tmu, dat$rows, dat$n, sections, "", "Dynamic-factor model", character(0),
                      level = level, show = show, tmfmt = tmfmt, log = log)
  fit$cmd <- "dfactor"
  fit$obs <- ys
  fit$factor <- factor
  fit$f_ar <- f_ar
  fit$o_ar <- o_ar
  fit$eqn <- unique(Filter(Negate(is.null), vapply(sections, function(s) s$heading, "")))
  fit
}
# e() after the three commands: the scalars in Stata's order, then the macros, b and V. Stata's own search is
# not reproduced, so e(ic), e(tech_steps), e(technique), e(ilog), e(gradient) and the datasignature are not kept.
stata_ss_e <- function(fit, cmdline) {
  est <- fit$est
  w <- fit$wald
  spec <- fit$spec
  rank <- sum(is.na(spec$par$fixed))
  base <- list(N = fit$n, stationary = as.numeric(!spec$diffuse), rc = 0, converged = as.numeric(est$converged), ll = est$ll,
               rank = rank, k_eq = length(fit$eqn))
  tail_ <- list(tmax = fit$tmax, tmin = fit$tmin, k = length(spec$par$eq), k_dv = length(fit$obs))
  wald <- list(p = if (w$df > 0) w$p else NA_real_, df_m = w$df, chi2 = if (w$df > 0) w$chi2 else NA_real_)
  tx <- function(v) paste(v, collapse = " ")
  time <- fit$timevar
  if (fit$cmd == "sspace") {
    sc <- c(base, wald, list(k_obser = fit$extra$k_obser, k_state = fit$extra$k_state, k_state_err = fit$extra$k_state_err,
                             k_obser_err = fit$extra$k_obser_err), tail_)
    mac <- list(cmd = "sspace", title = "State-space model", cmdline = cmdline, estat_cmd = "sspace_estat", predict = "sspace_p",
                covariates = "_NONE", marginsnotok = "_ALL", eqnames = tx(fit$eqn), state_deps = tx(fit$states),
                obser_deps = tx(fit$obs), tvar = time, depvar = tx(fit$obs), datasignaturevars = tx(c(fit$obs, time)),
                tmaxs = fit$tmaxs, tmins = fit$tmins, chi2type = "Wald", opt = "optimize", vce = "OIM", method = "hybrid",
                R_structure = "diagonal", Q_structure = "diagonal", initial_values = "NLS", properties = "b V")
  } else if (fit$cmd == "ucm") {
    sc <- c(base[c("N", "stationary", "rc", "converged", "ll", "rank")], list(k_eq = 1), wald, tail_[c("tmax", "tmin")],
            list(k_cycles = 0), tail_[c("k", "k_dv")])
    mac <- list(cmd = "ucm", title = "Unobserved-components model", cmdline = cmdline, estat_cmd = "ucm_estat", predict = "ucm_p",
                covariates = "_NONE", model = fit$model, eqnames = "/", tvar = time, depvar = tx(fit$obs), tmaxs = fit$tmaxs,
                tmins = fit$tmins, marginsnotok = "_ALL", chi2type = "Wald", opt = "optimize", vce = "OIM",
                initial_values = "fixed", properties = "b V")
  } else {
    ke <- length(fit$eqn)
    sc <- c(base[c("N", "stationary", "rc", "converged", "ll", "rank")], list(k_eq = ke, k_eq_model = ke), wald,
            list(k_factor = as.numeric(!is.null(fit$factor)), k_obser = length(fit$obs), k = tail_$k, o_ar_max = fit$o_ar,
                 f_ar_max = fit$f_ar, k_dv = length(fit$obs)), tail_[c("tmin", "tmax")])
    mac <- list(cmd = "dfactor", title = "Dynamic-factor model", cmdline = cmdline, estat_cmd = "dfactor_estat",
                predict = "dfactor_p", covariates = "_NONE", marginsnotok = "_ALL", observ_cov = "diagonal")
    if (!is.null(fit$factor)) {
      mac <- c(mac, list(factor_cov = "identity", f_ar = if (fit$f_ar == 1) "ar(1)", model = "DF", factor_deps = fit$factor))
    } else {
      mac <- c(mac, list(o_ar = "ar(1)", model = "SUR"))
    }
    mac <- c(mac, list(obser_deps = tx(fit$obs), tvar = time, depvar = tx(fit$obs), tmaxs = fit$tmaxs, tmins = fit$tmins,
                       eqnames = tx(fit$eqn), datasignaturevars = tx(c(fit$obs, time)), chi2type = "Wald", opt = "optimize",
                       vce = "OIM", method = "hybrid", initial_values = "NLS", properties = "b V"))
  }
  mac <- Filter(Negate(is.null), mac)
  b <- matrix(fit$b, 1, dimnames = list("y1", names(fit$b)))
  c(sc, mac, list(b = b, V = fit$vcov))
}
# predict after the three commands: xb (the one-step prediction of the first observation), states (equation()),
# trend (the level) and factors (the factor), each from the one-step or the filtered states (smethod())
stata_ss_predict <- function(fit, d, what = "xb", equation = NULL, filtered = FALSE) {
  stop_ <- function(...) stop(..., call. = FALSE)
  spec <- fit$spec
  f <- stata_ss_run(spec, fit$est$theta)
  if (is.null(f)) stop_("could not run the Kalman filter (Stata: r(459))")
  which_state <- 1
  if (what == "states" && !is.null(equation)) {
    which_state <- match(equation, spec$state_names)
    if (is.na(which_state)) stop_("equation(", equation, ") is not a state of the model (Stata: r(198))")
  }
  # dfactor's xb is the regression on the exogenous variables alone (none here: 0), the other two's is the
  # one-step prediction of the (first) observation
  val <- if (what == "xb" && fit$cmd == "dfactor") rep(0, nrow(f$pred))
         else if (what == "xb") f$pred[, 1]
         else if (filtered) f$st[, which_state]
         else f$one[, which_state]
  out <- rep(NA_real_, nrow(d))
  out[fit$rows] <- val
  out
}
# the texts of the constraints named in constraints(numlist)
stata_ss_cons <- function(ids) {
  cns <- get0("stata_cns", ifnotfound = list())
  vapply(ids, function(i) {
    t <- cns[[as.character(i)]]
    if (is.null(t)) stop("constraint ", i, " not found (Stata: r(111))", call. = FALSE)
    t
  }, "")
}
