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
  list(b = b, ll = d$ll, log = lg, flags = fl, converged = conv, g = d$g, negH = d$negH, inv = inv, scores = d$scores,
       prev = if (is.null(prev)) NULL else prev$b)
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
  if (!isTRUE(m$converged)) cat("convergence not achieved\n")
}
stata_cx_warn <- function(e) if (isTRUE(e$converged == 0)) cat("Warning: Convergence not achieved.\n")
# the offset's row: coefficient fixed at 1
stata_cx_offrow <- function(offset) if (!is.null(offset)) cat(stata_tab_pad(stata_tab_abbrev(offset, 12), 12), " |", stata_tab_pad("1", 11), "  (offset)\n", sep = "")
# the explanatory columns: model.matrix of the terms (factor(g) for i.g), Stata's names (1b.g, 2.g), the
# base-level columns Stata keeps as zeros in e(b)
stata_cx_design <- function(mf, terms, nocons = FALSE) {
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
  list(X = X, fo = fo, xlev = xlev, nmx = nmx, full = if (nocons) full else c(full, "_cons"))
}
stata_cx_num <- function(u) { u <- unclass(u); attributes(u) <- NULL; as.numeric(u) }
# the data's variables the terms use
stata_cx_vars <- function(terms) setdiff(unique(unlist(regmatches(terms, gregexpr("[A-Za-z_][A-Za-z0-9_]*", terms)))), "factor")
stata_cx_check <- function(d, vars) for (v in unique(vars)) if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
# the rows of coefficients, as stata_coef_rows lays them out; z or t tests, exponentiated with eform
stata_cx_rows <- function(nm, b, se, test, df, level, data, eform = FALSE, offset = NULL) {
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
  stata_cx_offrow(offset)
}
# a row with no test: a name right-aligned in 13, estimate, standard error, interval (var(e.y), sigma, rho)
stata_cx_plain <- function(name, est, se, lo, hi, wide = TRUE) {
  g9 <- stata_cx_g9
  cat(sprintf(if (wide) "%13s|%11s%11s%17s%13s%12s\n" else "%12s |%11s%11s%17s%13s%12s\n", name, g9(est), g9(se), "", g9(lo), g9(hi)))
}
# a row with a test, name right-aligned in 13 (/lnsigma, /athrho)
stata_cx_aux <- function(name, est, se, test, df, level) {
  g9 <- stata_cx_g9
  crit <- if (test == "t") stats::qt((1 + level / 100) / 2, df) else stats::qnorm((1 + level / 100) / 2)
  st <- est / se
  p <- if (test == "t") 2 * stats::pt(-abs(st), df) else 2 * stats::pnorm(-abs(st))
  cat(sprintf("%12s |%11s%11s%9.2f%8.3f%13s%12s\n", name, g9(est), g9(se), st, p, g9(est - crit * se), g9(est + crit * se)))
}
stata_cx_rule <- function() cat(strrep("-", 78), "\n", sep = "")
stata_cx_rule2 <- function() cat(strrep("-", 13), "+", strrep("-", 64), "\n", sep = "")
stata_cx_head_right <- function(label, value) sprintf("%-18s= %6s", label, value)
stata_cx_head_count <- function(label, n) sprintf("%17s = %6s", label, n)
stata_cx_note_clusters <- function(G, v) if (!is.null(G)) cat(formatC(sprintf("(Std. err. adjusted for %s clusters in %s)", format(G, big.mark = ","), v), width = 78), "\n", sep = "")
# the design columns of a command's terms for the rows kept
stata_cx_setup <- function(d, dep, terms, extra = character(), wvar = NULL, cluster = NULL, offset = NULL, nocons = FALSE) {
  tv <- stata_cx_vars(terms)
  vars <- unique(c(dep, tv, extra, wvar, cluster, offset))
  stata_cx_check(d, vars)
  ok <- stats::complete.cases(d[vars])
  if (!any(ok)) stop("no observations (Stata: r(2000))", call. = FALSE)
  rows <- which(ok)
  mf <- d[rows, , drop = FALSE]
  des <- stata_cx_design(mf, terms, nocons)
  X1 <- if (nocons) des$X else cbind(des$X, 1)
  if (qr(X1)$rank < ncol(X1)) stop("RCraft: a model with collinear regressors is not translated yet", call. = FALSE)
  off <- if (is.null(offset)) NULL else stata_cx_num(mf[[offset]])
  list(rows = rows, mf = mf, des = des, X = des$X, n = nrow(mf), off = off)
}
# estat ic after the six models: the estimates stats table with the one row "." and r(S); n() and df() as given
# (heckman twostep keeps no likelihood: r(321))
stata_cx_estat_ic <- function(e, n = NULL, df = NULL) {
  if (is.null(e$ll)) stop("likelihood information not found in last estimation results (Stata: r(321))", call. = FALSE)
  stata_est_stats(NULL, n = n, df = df, dot = TRUE)
}
# e(V)'s column names as Stata keeps them (truncreg's equation is eq1, its sigma sits in /)
stata_cx_vnames <- function(e) {
  nm <- colnames(e$V)
  if (identical(e$cmd, "truncreg")) {
    k <- length(nm)
    nm <- c(sub("^[^:]*:", "eq1:", nm[-k]), "/:sigma")
  }
  nm
}
# estat vce after the six: the lower triangle in equation blocks, r(V)
stata_cx_estat_vce <- function(e, correlation = FALSE, format = NULL) {
  V <- as.matrix(e$V)
  nm <- stata_cx_vnames(e)
  eq <- sub(":.*$", "", nm)
  short <- sub("^[^:]*:", "", nm)
  if (any(grepl("[.#]", short[eq != "/"]))) stop("RCraft: estat vce with factor variables after ", e$cmd, " is not translated yet", call. = FALSE)
  k <- length(nm)
  M <- if (correlation) { s <- sqrt(diag(V)); V / outer(s, s) } else V
  f <- if (!is.null(format)) format else if (correlation) "%8.4f" else "%10.0g"
  fw <- as.integer(sub("^%-?0?([0-9]+).*$", "\\1", f))
  first <- c(TRUE, eq[-1] != eq[-k])
  W <- ifelse(first, fw + 1L, fw + 2L)
  grp <- cumsum(first)
  G <- max(grp)
  Wg <- vapply(seq_len(G), function(g) sum(W[grp == g]), 0)
  geq <- eq[first]
  ls <- if (exists("stata_linesize")) stata_linesize else 79
  if (14 + sum(Wg + 1) + G > ls) stop("RCraft: estat vce wider than the line after ", e$cmd, " is not translated yet", call. = FALSE)
  paren <- grepl("(", short, fixed = TRUE)
  spec <- paren[k]
  pad <- function(s, n) formatC(s, width = n)
  blank <- function(n) strrep(" ", n)
  cat("\n", if (correlation) "Correlation" else "Covariance", " matrix of coefficients of ", e$cmd, " model\n\n", sep = "")
  cat(blank(13), "|", paste(vapply(seq_len(G), function(g) formatC(paste0(" ", geq[g]), width = -(Wg[g] + if (g < G || spec) 1L else 0L)), ""), collapse = "|"), "\n", sep = "")
  hcell <- function(j) if (j == k && spec) paste0(" ", pad(short[j], W[j])) else pad(stata_tab_abbrev(short[j], 12), W[j])
  cat(pad("e(V)", 12), " |", paste(vapply(seq_len(G), function(g) paste0(paste(vapply(which(grp == g), hcell, ""), collapse = ""), if (g == G && spec) "" else " "), ""), collapse = "|"), "\n", sep = "")
  dash <- paste0(strrep("-", 13), "+", paste(vapply(seq_len(G), function(g) strrep("-", Wg[g] + if (g < G) 1 else 0), ""), collapse = "+"), "\n")
  for (g in seq_len(G)) {
    cat(dash)
    cat(formatC(geq[g], width = -13), "|", paste(vapply(seq_len(G), function(h) blank(Wg[h] + if (h < G) 1 else 0), ""), collapse = "|"), "\n", sep = "")
    for (i in which(grp == g)) {
      cells <- vapply(seq_len(k), function(j) if (j <= i) pad(stata_fmt(M[i, j], f, trim = TRUE), W[j]) else blank(W[j]), "")
      lab <- if (paren[i]) paste0(pad(short[i], 13), "|") else paste0(pad(stata_tab_abbrev(short[i], 12), 12), " |")
      cat(lab, paste(vapply(seq_len(G), function(h) paste0(paste(cells[grp == h], collapse = ""), " "), ""), collapse = "|"), "\n", sep = "")
    }
  }
  dimnames(M) <- list(nm, nm)
  invisible(list(V = M))
}
# estat summarize after the six: mean, sd, min and max over e(sample) of the dependent variables and each
# equation's regressors (or of the varlist), weights as Stata weights them, r(stats)
stata_cx_estat_summ <- function(e, d, vars = NULL, equation = FALSE, labels = FALSE, noheader = FALSE) {
  nm <- stata_cx_vnames(e)
  eqv <- sub(":.*$", "", nm)
  sh <- sub("^[^:]*:", "", nm)
  deps <- strsplit(e$depvar, " ", fixed = TRUE)[[1]]
  eqn <- unique(eqv[!startsWith(eqv, "/")])
  grp <- lapply(eqn, function(g) setdiff(unique(sh[eqv == g]), "_cons"))
  names(grp) <- eqn
  grp <- grp[lengths(grp) > 0]
  if (any(grepl("[.#]", unlist(grp)))) stop("RCraft: estat summarize with factor variables after ", e$cmd, " is not translated yet", call. = FALSE)
  if (!is.null(vars)) vars <- unique(vars)
  secs <- if (!is.null(vars)) list(vars) else if (equation) c(list(depvar = deps), grp) else list(unique(c(deps, unlist(grp))))
  shown <- unique(unlist(secs))
  miss <- setdiff(shown, names(d))
  if (length(miss)) stop("variable ", miss[1], " not found (Stata: r(111))", call. = FALSE)
  x <- d[which(stata_esample), , drop = FALSE]
  num <- function(u) { u <- unclass(u); attributes(u) <- NULL; as.numeric(u) }
  wv <- if (is.null(e$wexp)) NULL else sub("^=\\s*", "", e$wexp)
  wt <- e$wtype
  w <- if (is.null(wv)) NULL else num(x[[wv]])
  cl <- e$clustvar
  if (!is.null(cl) && identical(wt, "fweight")) stop("RCraft: estat summarize after a clustered fit with frequency weights is not translated yet", call. = FALSE)
  st <- function(v) {
    u <- num(x[[v]])
    ok <- !is.na(u)
    uu <- u[ok]
    if (is.null(w)) return(c(mean(uu), stats::sd(uu), min(uu), max(uu)))
    ww <- w[ok]
    sw <- sum(ww)
    m <- sum(ww * uu) / sw
    ss <- sum(ww * (uu - m)^2)
    den <- if (wt %in% c("fweight", "iweight")) sw - 1 else (length(uu) - 1) * sw / length(uu)
    c(m, sqrt(ss / den), min(uu), max(uu))
  }
  stw <- if (is.null(w)) NULL else c(mean(w), stats::sd(w), min(w), max(w))
  nobs <- if (!is.null(e$N)) e$N else nrow(x)
  g <- function(v, wd) formatC(stata_fmt(v, "%9.0g", trim = TRUE), width = wd)
  g8 <- function(v, wd) formatC(stata_fmt(v, "%8.0g", trim = TRUE), width = wd)
  nmiss <- vapply(shown, function(v) sum(is.na(num(x[[v]]))), 0)
  cat("\n")
  if (labels) {
    if (!noheader) {
      if (is.null(cl)) {
        cat("Estimation sample ", formatC(e$cmd, width = -34), "Number of obs = ", formatC(stata_tab_count(nobs), width = 10), "\n", sep = "")
      } else {
        sz <- as.numeric(table(x[[cl]]))
        ind <- strrep(" ", 45)
        cat("Estimation sample ", formatC(e$cmd, width = -27), sprintf("Number of obs        = %10s\n\n", stata_tab_count(nobs)), sep = "")
        cat(ind, sprintf("Number of clusters   = %10d\n", length(sz)), ind, sprintf("Obs per cluster: min = %10d\n", as.integer(min(sz))),
            ind, sprintf("                 avg = %10.1f\n", mean(sz)), ind, sprintf("                 max = %10d\n\n", as.integer(max(sz))), sep = "")
      }
    }
    line <- function(ch) cat(strrep("-", 13), ch, strrep("-", 64), "\n", sep = "")
    row <- function(v, s, lab) cat(formatC(stata_tab_abbrev(v), width = 12), " |", g(s[1], 10), g(s[2], 11), g8(s[3], 9), g8(s[4], 9),
                                   if (is.null(lab) || !nzchar(lab)) "" else paste0("   ", lab), "\n", sep = "")
    cat(strrep("-", 78), "\n", sep = "")
    cat("    Variable |      Mean   Std. dev.     Min      Max   Label\n")
    lab <- function(v) { l <- attr(d[[v]], "label", exact = TRUE); if (is.null(l)) "" else l }
  } else {
    if (!noheader) {
      if (is.null(cl)) {
        cat("  Estimation sample ", formatC(e$cmd, width = -23), "Number of obs = ", formatC(stata_tab_count(nobs), width = 10), "\n\n", sep = "")
      } else {
        sz <- as.numeric(table(x[[cl]]))
        ind <- strrep(" ", 36)
        cat("  Estimation sample ", e$cmd, "\n", ind, sprintf("Number of obs        = %10s\n\n", stata_tab_count(nobs)), sep = "")
        cat(ind, sprintf("Number of clusters   = %10d\n", length(sz)), ind, sprintf("Obs per cluster: min = %10d\n", as.integer(min(sz))),
            strrep(" ", 53), sprintf("avg = %10.1f\n", mean(sz)), strrep(" ", 53), sprintf("max = %10d\n\n", as.integer(max(sz))), sep = "")
      }
    }
    line <- function(ch) cat("  ", strrep("-", 13), ch, strrep("-", 53), "\n", sep = "")
    row <- function(v, s, lab) cat("  ", formatC(stata_tab_abbrev(v), width = 12), " |", g(s[1], 13), g(s[2], 14), g(s[3], 13), g(s[4], 13), "\n", sep = "")
    cat("  ", strrep("-", 67), "\n", sep = "")
    cat("      Variable |         Mean      Std. dev.         Min          Max\n")
    lab <- function(v) ""
  }
  line("+")
  rn <- character(0)
  mat <- NULL
  add <- function(name, s) { rn <<- c(rn, name); mat <<- rbind(mat, s) }
  for (si in seq_along(secs)) {
    if (length(secs) > 1) {
      if (si > 1) line("+")
      cat(if (labels) "" else "  ", formatC(names(secs)[si], width = -12), " |\n", sep = "")
    }
    for (v in secs[[si]]) {
      s <- st(v)
      row(v, s, lab(v))
      add(if (length(secs) > 1) paste0(names(secs)[si], ":", v) else v, s)
    }
  }
  if (!is.null(stw)) {
    line("+")
    row("_weight", stw, paste("=", wv))
    add(if (length(secs) > 1) "weight:_weight" else "_weight", stw)
  }
  if (labels) cat(strrep("-", 78), "\n", sep = "") else cat("  ", strrep("-", 67), "\n", sep = "")
  if (!is.null(cl)) cat(if (labels) "" else "  ", "Std. dev. not adjusted for clustering\n", sep = "")
  for (v in shown[nmiss > 0]) cat("  ", v, " statistics based on ", sum(!is.na(num(x[[v]]))), " nonmissing observations\n", sep = "")
  dimnames(mat) <- list(rn, c("mean", "sd", "min", "max"))
  invisible(list(stats = mat))
}
