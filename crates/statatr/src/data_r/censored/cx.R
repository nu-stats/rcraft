# Shared pieces of the censored and selection models (tobit, intreg, truncreg, cloglog, hetprobit,
# heckman), our own R after StatCraft's decoding of Stata 19 (plan/stata/censored.md): Newton-Raphson as
# Stata's ml steps, the design matrix, the robust and clustered variances, the coefficient rows.
stata_cx_g9 <- function(v) stata_fmt(v, "%9.0g", trim = TRUE)
stata_cx_g10 <- function(v) formatC(stata_fmt(v, "%10.0g", trim = TRUE), width = 10)
# Newton-Raphson with ml's step rules: halve while the likelihood falls, else step on by 1/8, 1/4, ... of
# the step while it rises (each added to the point reached); where -H is not positive definite, the modified
# Marquardt step; done when g'(-H)^-1 g <= 1e-5 and (reldif(ll) <= 1e-7 or mreldif(b) <= 1e-6), tested
# only where -H is positive definite and after the first iteration.
stata_cx_max <- function(llf, derivs, b, maxit = 300) {
  rd <- function(a, b) abs(a - b) / (abs(b) + 1)
  pd <- function(A) !inherits(tryCatch(chol(A), error = function(e) e), "error")
  cinv <- function(A) {
    if (pd(A)) return(list(inv = solve(A), nc = FALSE))
    sc <- max(max(abs(diag(A))), 1); lam <- 1e-4 * sc
    for (i in 1:80) {
      m <- A; diag(m) <- diag(A) + lam
      if (pd(m)) return(list(inv = solve(m), nc = TRUE))
      lam <- lam * 2
    }
    stop("the likelihood is not concave and could not be made so (Stata: r(430))", call. = FALSE)
  }
  marq <- function(b, d) {
    A <- d$negH; m <- A; diag(m) <- diag(A) + 1.1 * abs(diag(A))
    inv <- tryCatch(solve(m), error = function(e) NULL)
    if (is.null(inv)) return(NULL)
    dir <- as.numeric(inv %*% d$g); nr <- sqrt(sum(dir^2))
    if (!(is.finite(nr) && nr > 0)) return(NULL)
    u <- dir / nr; gu <- sum(d$g * u)
    len <- 0.01 * abs(d$ll) / abs(gu)
    if (!(is.finite(len) && len > 0)) return(NULL)
    rises <- function(v) isTRUE(v > d$ll)
    v <- llf(b + len * u); tries <- 0
    while (!rises(v) && tries < 60) { len <- len / 2; v <- llf(b + len * u); tries <- tries + 1 }
    if (!rises(v)) return(NULL)
    for (i in 1:200) { f2 <- llf(b + 2 * len * u); if (isTRUE(f2 > v)) { len <- 2 * len; v <- f2 } else break }
    b + len * u
  }
  d <- derivs(b); ci <- cinv(d$negH)
  lg <- numeric(0); fl <- character(0); backed <- FALSE; prev <- NULL; conv <- FALSE
  repeat {
    lg <- c(lg, d$ll); fl <- c(fl, if (ci$nc) "(not concave)" else if (backed) "(backed up)" else "")
    step <- as.numeric(ci$inv %*% d$g)
    if (!is.null(prev) && !ci$nc) {
      nr <- sum(d$g * step)
      if (abs(nr) <= 1e-5 && (rd(d$ll, prev$ll) <= 1e-7 || max(rd(b, prev$b)) <= 1e-6)) { conv <- TRUE; break }
    }
    if (length(lg) > maxit) break
    if (ci$nc) {
      nb <- marq(b, d)
      if (!is.null(nb)) { backed <- FALSE; prev <- list(b = b, ll = d$ll); b <- nb; d <- derivs(b); ci <- cinv(d$negH); next }
    }
    t <- 1; nx <- llf(b + t * step); halved <- FALSE
    while ((is.nan(nx) || isTRUE(nx < d$ll)) && t > 1e-15) { halved <- TRUE; t <- t / 2; nx <- llf(b + t * step) }
    backed <- halved && (is.nan(nx) || isTRUE(nx < d$ll))
    pt <- b + t * step
    if (!halved) {
      ex <- 0.125
      repeat { ah <- pt + ex * step; f <- llf(ah); if (isTRUE(f > nx)) { pt <- ah; nx <- f; ex <- ex * 2 } else break }
    }
    prev <- list(b = b, ll = d$ll); b <- pt; d <- derivs(b); ci <- cinv(d$negH)
  }
  inv <- if (ci$nc) solve(d$negH) else ci$inv
  list(b = b, ll = d$ll, log = lg, flags = fl, converged = conv, g = d$g, negH = d$negH, inv = inv, scores = d$scores)
}
# the variance: (-H)^-1, or the sandwich over the weighted scores (a frequency-weighted row once with its
# score over w, N/(N - 1); with clusters the scores summed within them, G/(G - 1))
stata_cx_vcov <- function(m, robust, w = NULL, fw = FALSE, cluster = NULL) {
  if (!robust) return(m$inv)
  S <- m$scores
  if (!is.null(cluster)) {
    sg <- rowsum(S, cluster); G <- nrow(sg)
    meat <- crossprod(sg); sc <- G / (G - 1)
  } else {
    wv <- if (fw && !is.null(w)) w else rep(1, nrow(S))
    meat <- crossprod(S / sqrt(wv)); N <- if (fw) sum(w) else nrow(S); sc <- N / (N - 1)
  }
  v <- m$inv %*% meat %*% m$inv * sc
  (v + t(v)) / 2
}
stata_cx_log <- function(m, label = "Log likelihood") {
  for (i in seq_along(m$log)) cat(formatC(paste0("Iteration ", i - 1, ":"), width = -13), " ", label, " = ", stata_cx_g10(m$log[i]),
                                  "  ", m$flags[i], "\n", sep = "")
}
# the explanatory columns: model.matrix of the terms (factor(g) for i.g), Stata's names (1b.g, 2.g), the
# base-level columns Stata keeps as zeros in e(b)
stata_cx_design <- function(mf, terms) {
  fo <- stats::as.formula(paste("~", paste(terms, collapse = " + ")))
  X <- stats::model.matrix(fo, mf)
  X <- X[, colnames(X) != "(Intercept)", drop = FALSE]
  xlev <- stats::.getXlevels(stats::terms(fo), stats::model.frame(fo, mf))
  rn <- function(s) ifelse(grepl("^factor\\(", s), sub("^factor\\(([^)]*)\\)(.*)$", "\\2.\\1", s), s)
  nmx <- rn(colnames(X))
  full <- character()
  for (j in seq_along(nmx)) {
    cj <- colnames(X)[j]
    fv <- if (grepl("^factor\\(", cj)) sub("^(factor\\([^)]*\\)).*$", "\\1", cj) else NA_character_
    if (!is.na(fv) && (j == 1 || !startsWith(colnames(X)[j - 1], fv))) {
      v <- sub("^factor\\((.*)\\)$", "\\1", fv)
      full <- c(full, paste0(xlev[[fv]][1], "b.", v))
    }
    full <- c(full, nmx[j])
  }
  list(X = X, fo = fo, xlev = xlev, nmx = nmx, full = c(full, "_cons"))
}
stata_cx_num <- function(u) { u <- unclass(u); attributes(u) <- NULL; as.numeric(u) }
# the data's variables the terms use
stata_cx_vars <- function(terms) setdiff(unique(unlist(regmatches(terms, gregexpr("[A-Za-z_][A-Za-z0-9_]*", terms)))), "factor")
stata_cx_check <- function(d, vars) for (v in unique(vars)) if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
# the rows of coefficients, as stata_coef_rows lays them out; z or t tests, exponentiated with eform
stata_cx_rows <- function(nm, b, se, test, df, level, data, eform = FALSE) {
  pad <- stata_tab_pad
  rows <- stata_coef_rows(nm, data)
  if (is.null(rows)) stop("RCraft: a model with a term whose table layout was not checked is not translated yet", call. = FALSE)
  names(b) <- nm; names(se) <- nm
  crit <- if (test == "t") stats::qt((1 + level / 100) / 2, df) else stats::qnorm((1 + level / 100) / 2)
  for (r in rows) {
    if (r$kind == "blank") { cat(strrep(" ", 13), "|\n", sep = ""); next }
    if (r$kind == "head") { cat(pad(stata_tab_abbrev(r$label), 12), " |\n", sep = ""); next }
    if (r$kind == "wrap") { cat(pad(r$label, 11), " #|\n", sep = ""); next }
    label <- if (r$kind == "level") paste0(pad(r$label, 11), " ") else pad(stata_tab_abbrev(r$label), 12)
    if (!is.null(r$note)) { cat(label, " |", pad("0", 11), "  ", r$note, "\n", sep = ""); next }
    bi <- b[[r$col]]; si <- se[[r$col]]
    st <- bi / si
    p <- if (test == "t") 2 * stats::pt(-abs(st), df) else 2 * stats::pnorm(-abs(st))
    lo <- bi - crit * si; hi <- bi + crit * si
    if (eform) { lo <- exp(lo); hi <- exp(hi); si <- exp(bi) * si; bi <- exp(bi) }
    cat(label, " |", pad(stata_tab_g(bi), 11), pad(stata_tab_g(si), 11), pad(sprintf("%.2f", st), 9),
        pad(sprintf("%.3f", p), 8), pad(stata_tab_g(lo), 13), pad(stata_tab_g(hi), 12), "\n", sep = "")
  }
}
# a row with no test: a name right-aligned in 13, estimate, standard error, interval (var(e.y), sigma, rho)
stata_cx_plain <- function(name, est, se, lo, hi) {
  g9 <- stata_cx_g9
  cat(sprintf("%13s|%11s%11s%17s%13s%12s\n", name, g9(est), g9(se), "", g9(lo), g9(hi)))
}
# a row with a test, name right-aligned in 13 (/lnsigma, /athrho)
stata_cx_aux <- function(name, est, se, test, df, level) {
  g9 <- stata_cx_g9
  crit <- if (test == "t") stats::qt((1 + level / 100) / 2, df) else stats::qnorm((1 + level / 100) / 2)
  st <- est / se
  p <- if (test == "t") 2 * stats::pt(-abs(st), df) else 2 * stats::pnorm(-abs(st))
  cat(sprintf("%13s|%11s%11s%9.2f%8.3f%13s%12s\n", name, g9(est), g9(se), st, p, g9(est - crit * se), g9(est + crit * se)))
}
stata_cx_rule <- function() cat(strrep("-", 78), "\n", sep = "")
stata_cx_rule2 <- function() cat(strrep("-", 13), "+", strrep("-", 64), "\n", sep = "")
stata_cx_head_right <- function(label, value) sprintf("%-18s= %6s", label, value)
stata_cx_head_count <- function(label, n) sprintf("%17s = %6s", label, n)
stata_cx_note_clusters <- function(G, v) if (!is.null(G)) cat(formatC(sprintf("(Std. err. adjusted for %s clusters in %s)", format(G, big.mark = ","), v), width = 78), "\n", sep = "")
