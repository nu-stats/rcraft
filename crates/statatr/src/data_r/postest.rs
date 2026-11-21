//! Batch L, post-estimation: `contrast` and `pwcompare` here (`lrtest`,
//! `hausman` and `predictnl` follow in this file), after the models whose fit R
//! holds as an `lm` or `glm` (`plan/stata/postest.md`).
//!
//! A contrast is a row of weights on the coefficients. The margins it compares
//! are Stata's "asbalanced" ones: the term's variables at their levels, every
//! other factor of the model averaged over its levels with equal weights, and
//! the continuous covariates at zero. Each margin is the mean of the model
//! matrix over those cells, so the weights come from R's own model matrix and
//! need no coefficient names to be read. Tests are F tests with the model's
//! residual degrees of freedom after the linear models and chi-squared tests
//! after the others, the rank taken from a generalized inverse of the
//! contrasts' variance. A contrast the data cannot estimate (a weight outside
//! the row space of the design) is `(not testable)` or `(not estimable)`.

use super::model::Fit;
use super::*;

pub(super) const CONTRAST_HELPER: &str = r##"stata_contrast <- function(kind, fit, data, b, V, df, terms, factors, effects = FALSE, cimargins = FALSE,
                           mcomp = "noadjust", level = 95, eq = "", cmdline = "", est_cmd = "", est_cmdline = "",
                           show = TRUE) {
  plain <- function(v) if (inherits(v, "haven_labelled")) as.vector(unclass(v)) else v
  data <- as.data.frame(lapply(data, plain), check.names = FALSE)
  fvars <- names(factors)
  tt <- stats::delete.response(stats::terms(fit))
  xl <- fit$xlevels
  ttf <- tt
  # regress leaves out whole terms (named in $omitted); xtreg keeps them in its
  # terms and names the columns it left out, so its design is complete already
  omitted <- if (is.null(fit$panel)) fit$omitted else NULL
  if (length(omitted)) {
    # the terms Stata omits stay in the design: a margin that averages over them
    # is not estimable
    ttf <- stats::delete.response(stats::terms(stats::update(stats::formula(fit),
      stats::as.formula(paste(". ~ . +", paste(omitted, collapse = " + "))))))
    for (o in omitted) for (part in strsplit(o, ":", fixed = TRUE)[[1]]) {
      m <- regmatches(part, regexec("^(factor|stata_ib)\\(([^,()]+)", part))[[1]]
      if (length(m) && is.null(xl[[part]])) xl[[part]] <- as.character(sort(unique(data[[trimws(m[3])]])))
    }
  }
  mm <- function(nd) {
    mf <- stats::model.frame(ttf, nd, xlev = xl, na.action = stats::na.pass)
    suppressWarnings(stats::model.matrix(ttf, mf, contrasts.arg = fit$contrasts))
  }
  X <- mm(data)
  X <- X[stats::complete.cases(X), , drop = FALSE]
  fn <- colnames(X)
  bb <- stats::setNames(numeric(length(fn)), fn)
  have <- intersect(fn, names(b))
  if (length(have) < sum(!is.na(b))) stop("contrast: the coefficients and the model matrix do not match", call. = FALSE)
  bb[have] <- ifelse(is.na(b[have]), 0, b[have])
  VV <- matrix(0, length(fn), length(fn), dimnames = list(fn, fn))
  VV[have, have] <- V[have, have]
  VV[is.na(VV)] <- 0
  small <- is.finite(df)
  alpha <- 1 - level / 100
  cont <- setdiff(all.vars(ttf), fvars)
  lev <- lapply(stats::setNames(fvars, fvars), function(v) sort(unique(data[[v]][!is.na(data[[v]])])))
  lab <- function(x) as.character(x)
  # the balanced margin of a cell: the model matrix averaged over the levels of
  # the other factors, the continuous variables at zero
  wrow <- function(vals) {
    others <- setdiff(fvars, names(vals))
    g <- if (length(others)) expand.grid(rev(lev[others]), KEEP.OUT.ATTRS = FALSE) else data.frame(.n = 1)
    nd <- data[rep(1L, nrow(g)), , drop = FALSE]
    for (v in cont) nd[[v]] <- 0
    for (o in others) nd[[o]] <- g[[o]]
    for (v in names(vals)) nd[[v]] <- vals[[v]]
    colMeans(mm(nd))
  }
  # a weight vector is estimable when it lies in the design's row space (1e-8
  # of the largest weight)
  # after xtreg, fe the design holds an indicator for each panel as well: a
  # factor constant within the panels is then not estimable
  Xe <- X
  if (identical(fit$type, "fe") && !is.null(fit$panel) && fit$panel %in% names(data)) {
    okr <- stats::complete.cases(mm(data))
    gd <- stats::model.matrix(~ g - 1, data.frame(g = factor(data[[fit$panel]][okr])))
    Xe <- cbind(X, gd)
  }
  sv <- svd(crossprod(Xe))
  keep <- sv$d > max(sv$d) * 1e-10
  H <- sv$v[, keep, drop = FALSE] %*% t(sv$v[, keep, drop = FALSE])
  # a margin carries the constant, which after xtreg, fe and areg is the panels' mean effect:
  # the panel indicators carry it too, by their share of the rows (the intercept equals their sum)
  pshare <- if (ncol(Xe) > ncol(X)) colMeans(Xe[, ncol(X) + seq_len(ncol(Xe) - ncol(X)), drop = FALSE]) else NULL
  estimable <- function(L, margin = FALSE) {
    L <- matrix(L, ncol = length(fn))
    L <- cbind(L, matrix(0, nrow(L), ncol(Xe) - ncol(X)))
    ic <- match("(Intercept)", fn)
    if (margin && !is.null(pshare) && !is.na(ic)) {
      L[, ncol(X) + seq_along(pshare)] <- outer(L[, ic], pshare)
    }
    all(vapply(seq_len(nrow(L)), function(i) {
      l <- L[i, ]
      max(abs(l - drop(l %*% H))) <= 1e-8 * max(abs(l), 1e-300)
    }, TRUE))
  }
  wald <- function(L) {
    L <- matrix(L, ncol = length(fn))
    est <- drop(L %*% bb)
    Vc <- L %*% VV %*% t(L)
    # a generalized inverse by sweeping: the largest remaining variance first,
    # stopping when what is left of a row is gone (the contrasts' variance has
    # a rank below their number after a cluster-robust fit with few clusters)
    k <- nrow(Vc)
    S <- integer()
    R <- Vc
    d0 <- diag(Vc)
    repeat {
      cand <- setdiff(seq_len(k), S)
      if (!length(cand)) break
      j <- cand[which.max(diag(R)[cand])]
      if (!(R[j, j] > 1e-9 * d0[j])) break
      S <- c(S, j)
      R <- R - outer(R[, j], R[j, ]) / R[j, j]
    }
    r <- length(S)
    W <- if (r) drop(t(est[S]) %*% solve(Vc[S, S, drop = FALSE], est[S])) else 0
    p <- if (small) stats::pf(W / r, r, df, lower.tail = FALSE) else stats::pchisq(W, r, lower.tail = FALSE)
    list(df = r, stat = if (small) W / r else W, p = p, W = W)
  }
  cellgrid <- function(v) expand.grid(rev(lev[v]), KEEP.OUT.ATTRS = FALSE)[v]
  cache <- new.env()
  mrow <- function(vals) {
    key <- paste(names(vals), unlist(vals), collapse = "|")
    if (is.null(cache[[key]])) cache[[key]] <- wrow(vals)
    cache[[key]]
  }
  # one variable's contrasts of its levels: a unit is one contrast (a row) or,
  # for a plain variable, all of them tested together
  cunits <- function(v, op, for_effects = FALSE) {
    lv <- lev[[v]]
    k <- length(lv)
    I <- diag(k)
    bi <- if (is.na(factors[[v]])) 1L else match(as.numeric(factors[[v]]), lv)
    plain_op <- op == ""
    if (plain_op && !for_effects) {
      js <- setdiff(seq_len(k), bi)
      return(list(list(label = "(joint)", coef = t(vapply(js, function(j) I[j, ] - I[bi, ], numeric(k))))))
    }
    rows <- switch(if (plain_op) "r" else op,
      r = lapply(setdiff(seq_len(k), bi), function(j) list(I[j, ] - I[bi, ], sprintf("(%s vs %s)", lab(lv[j]), if (plain_op) "base" else lab(lv[bi])))),
      ar = lapply(seq_len(k)[-1], function(j) list(I[j, ] - I[j - 1, ], sprintf("(%s vs %s)", lab(lv[j]), lab(lv[j - 1])))),
      a = lapply(seq_len(k - 1), function(j) list(I[j, ] - I[j + 1, ], sprintf("(%s vs %s)", lab(lv[j]), lab(lv[j + 1])))),
      gw = {
        n <- vapply(lv, function(l) sum(data[[v]] == l, na.rm = TRUE), 0)
        lapply(seq_len(k), function(j) list(I[j, ] - n / sum(n), sprintf("(%s vs mean)", lab(lv[j]))))
      },
      g = lapply(seq_len(k), function(j) list(I[j, ] - 1 / k, sprintf("(%s vs mean)", lab(lv[j])))))
    lapply(rows, function(r) list(label = r[[2]], coef = matrix(r[[1]], 1)))
  }
  # a term's contrast rows: the combinations of its variables' units, the first
  # variable changing slowest; with @, each combination at each level of the
  # variables after it
  term_rows <- function(tm, for_effects) {
    vars <- tm$vars
    at <- tm$at
    us <- lapply(seq_along(vars), function(i) cunits(vars[i], tm$ops[i], for_effects))
    allv <- c(vars, at)
    cg <- cellgrid(allv)
    Mc <- t(vapply(seq_len(nrow(cg)), function(i) mrow(as.list(cg[i, , drop = FALSE])), numeric(length(fn))))
    combos <- expand.grid(rev(lapply(us, seq_along)), KEEP.OUT.ATTRS = FALSE)
    combos <- combos[rev(seq_along(combos))]
    atg <- if (length(at)) cellgrid(at) else data.frame(.n = 1)
    out <- list()
    for (ci in seq_len(nrow(combos))) {
      units <- lapply(seq_along(vars), function(i) us[[i]][[combos[ci, i]]])
      for (ai in seq_len(nrow(atg))) {
        # every row of the tensor product of the units' contrast rows
        idx <- expand.grid(rev(lapply(units, function(u) seq_len(nrow(u$coef)))), KEEP.OUT.ATTRS = FALSE)
        idx <- idx[rev(seq_along(idx))]
        L <- t(vapply(seq_len(nrow(idx)), function(ri) {
          w <- numeric(nrow(cg))
          for (cj in seq_len(nrow(cg))) {
            if (length(at) && !all(vapply(at, function(a) cg[[a]][cj] == atg[[a]][ai], TRUE))) next
            w[cj] <- prod(vapply(seq_along(vars), function(i) units[[i]]$coef[idx[ri, i], match(cg[[vars[i]]][cj], lev[[vars[i]]])], 0))
          }
          drop(w %*% Mc)
        }, numeric(length(fn))))
        ul <- vapply(units, function(u) u$label, "")
        atl <- if (length(at)) paste(vapply(at, function(a) lab(atg[[a]][ai]), ""), collapse = " ") else ""
        label <- if (all(tm$ops == "") && length(at)) atl else paste(c(ul, if (nzchar(atl)) atl), collapse = " ")
        out[[length(out) + 1]] <- list(label = label, L = matrix(L, ncol = length(fn)))
      }
    }
    out
  }
  g <- function(v) stata_fmt(v, "%9.0g", trim = TRUE)
  fnum <- function(x, f) ifelse(is.na(x), sprintf("%*s", as.integer(sub("^%([0-9]+).*$", "\\1", f)), "."), sprintf(f, x))
  # an estimate's table row: value, standard error, statistic, p, interval, after
  # the adjustment for m comparisons
  est_row <- function(l, m, rk) {
    l <- drop(l)
    if (!estimable(l)) return(list(ok = FALSE))
    est <- sum(l * bb)
    se <- sqrt(max(drop(l %*% VV %*% l), 0))
    st <- est / se
    p0 <- if (small) 2 * stats::pt(-abs(st), df) else 2 * stats::pnorm(-abs(st))
    q <- function(a) if (small) stats::qt(1 - a / 2, df) else stats::qnorm(1 - a / 2)
    adj <- switch(mcomp,
      bonferroni = list(p = min(1, p0 * m), crit = q(alpha / m)),
      sidak = list(p = 1 - (1 - p0)^m, crit = q(1 - (1 - alpha)^(1 / m))),
      scheffe = list(
        p = if (small) stats::pf(st^2 / rk, rk, df, lower.tail = FALSE) else stats::pchisq(st^2, rk, lower.tail = FALSE),
        crit = if (small) sqrt(rk * stats::qf(1 - alpha, rk, df)) else sqrt(stats::qchisq(1 - alpha, rk))),
      list(p = p0, crit = q(alpha)))
    list(ok = TRUE, est = est, se = se, st = st, p = adj$p, p0 = p0, crit = adj$crit, lo = est - adj$crit * se, hi = est + adj$crit * se)
  }
  method <- c(noadjust = "Unadjusted", bonferroni = "Bonferroni", sidak = "Sidak", scheffe = "Scheffe")[[mcomp]]
  title <- if (kind == "contrast") "Contrasts of marginal linear predictions" else "Pairwise comparisons of marginal linear predictions"
  termlab <- function(tm) paste0(paste(tm$vars, collapse = "#"), if (length(tm$at)) paste0("@", paste(tm$at, collapse = "#")) else "")
  Wof <- function(entry_labels, term_labels) max(13L, nchar(entry_labels) + 2L, nchar(term_labels) + 1L)
  bar <- function(W) paste0(strrep(" ", W), "|")
  dashes <- function(W, body) paste0(strrep("-", W + 1L + body))
  sepline <- function(W, body) paste0(strrep("-", W), "+", strrep("-", body))
  cilab <- sprintf("[%s%% conf. interval]", format(level))
  center <- function(txt, start) paste0(strrep(" ", start - floor(nchar(txt) / 2)), txt)
  wrap <- function(txt, width) {
    words <- strsplit(txt, " ", fixed = TRUE)[[1]]
    lines <- character()
    cur <- ""
    for (w in words) {
      if (nzchar(cur) && nchar(cur) + 1 + nchar(w) > width) {
        lines <- c(lines, cur)
        cur <- paste0("      ", w)
      } else cur <- if (nzchar(cur)) paste(cur, w) else w
    }
    c(lines, cur)
  }
  res <- list()
  # ---- the terms' rows and tests
  if (kind == "contrast") {
    for (tm in terms) {
      tests <- term_rows(tm, FALSE)
      allop <- all(tm$ops != "")
      ests <- if (allop) tests else term_rows(tm, TRUE)
      Lall <- do.call(rbind, lapply(tests, function(e) e$L))
      res[[length(res) + 1]] <- list(tm = tm, label = termlab(tm), tests = tests, ests = ests, Lall = Lall, show_est = allop || effects || mcomp != "noadjust")
    }
  } else {
    for (tm in terms) {
      cg <- cellgrid(tm$vars)
      Mc <- t(vapply(seq_len(nrow(cg)), function(i) mrow(as.list(cg[i, , drop = FALSE])), numeric(length(fn))))
      cl <- if (length(tm$vars) == 1) lab(cg[[1]]) else apply(cg, 1, function(r) paste0("(", paste(lab(r), collapse = " "), ")"))
      k <- nrow(cg)
      ests <- list()
      if (k >= 2) for (i in seq_len(k - 1)) for (j in (i + 1):k) ests[[length(ests) + 1]] <- list(label = paste(cl[j], "vs", cl[i]), L = matrix(Mc[j, ] - Mc[i, ], 1), i = i, j = j)
      res[[length(res) + 1]] <- list(tm = tm, label = termlab(tm), ests = ests, Mc = Mc, cl = cl, show_est = TRUE, Lall = do.call(rbind, lapply(ests, function(e) e$L)))
    }
  }
  # rank of each term's contrasts, for Scheffe
  for (i in seq_along(res)) res[[i]]$rank <- if (is.null(res[[i]]$Lall) || !length(res[[i]]$Lall)) 1L else qr(res[[i]]$Lall)$rank
  # ---- values
  rows_b <- numeric()
  rows_n <- character()
  tabcols <- list()
  for (i in seq_along(res)) {
    r <- res[[i]]
    r$er <- lapply(r$ests, function(e) est_row(e$L, length(r$ests), r$rank))
    if (kind == "contrast") {
      r$tr <- lapply(r$tests, function(e) if (estimable(e$L)) {
        w <- wald(e$L)
        w$ok <- TRUE
        if (nrow(e$L) == 1 && mcomp != "noadjust") {
          m <- length(r$ests)
          w$padj <- switch(mcomp, bonferroni = min(1, w$p * m), sidak = 1 - (1 - w$p)^m,
            scheffe = if (small) stats::pf(w$W / r$rank, r$rank, df, lower.tail = FALSE) else stats::pchisq(w$W, r$rank, lower.tail = FALSE))
        }
        w
      } else list(ok = FALSE))
      r$joint <- if (estimable(r$Lall)) { w <- wald(r$Lall); w$ok <- TRUE; w } else list(ok = FALSE)
    }
    res[[i]] <- r
    for (j in seq_along(r$ests)) {
      e <- r$er[[j]]
      rows_n <- c(rows_n, paste0(if (kind == "contrast") "c" else "p", length(rows_n) + 1))
      tabcols[[length(tabcols) + 1]] <- if (e$ok) c(b = e$est, se = e$se, t = e$st, pvalue = e$p, ll = e$lo, ul = e$hi, df = if (small) df else NA_real_, crit = e$crit, eform = 0) else rep(NA_real_, 9)
    }
  }
  tab <- if (length(tabcols)) do.call(cbind, tabcols) else matrix(NA_real_, 9, 0)
  rownames(tab) <- c("b", "se", if (small) "t" else "z", "pvalue", "ll", "ul", "df", "crit", "eform")
  colnames(tab) <- rows_n
  bvec <- matrix(tab["b", ], 1, dimnames = list("r1", rows_n))
  Lrows <- do.call(rbind, lapply(res, function(r) do.call(rbind, lapply(r$ests, function(e) e$L))))
  Vb <- Lrows %*% VV %*% t(Lrows)
  dimnames(Vb) <- list(rows_n, rows_n)
  out <- list(level = level, k_terms = length(res))
  if (small) out$df_r <- df
  if (kind == "pwcompare") out$balanced <- 0
  out$cmd <- kind
  out$title <- title
  out$margin_method <- "asbalanced"
  out$est_cmdline <- est_cmdline
  out$est_cmd <- est_cmd
  out$cmdline <- cmdline
  out$emptycells <- "strict"
  out$mcmethod <- if (kind == "contrast") mcomp else NULL
  if (kind == "contrast") {
    out$mcmethod <- mcomp
    # r(F), r(df), r(p): each term's rows, then its joint test when it has several
    ts <- unlist(lapply(res, function(r) c(r$tr, if (length(r$tests) > 1) list(r$joint))), recursive = FALSE)
    pick <- function(f) matrix(vapply(ts, function(w) if (isTRUE(w$ok)) f(w) else NA_real_, 0), 1, dimnames = list("r1", paste0("c", seq_along(ts))))
    out[[if (small) "F" else "chi2"]] <- pick(function(w) w$stat)
    out$df <- pick(function(w) w$df)
    out$p <- pick(function(w) w$p)
    if (small) out$df2 <- pick(function(w) df)
    out$table <- tab
    out$b <- bvec
    out$V <- Vb
  } else {
    out$mcmethod_vs <- mcomp
    out$mctitle_vs <- method
    out$table_vs <- tab
    out$b_vs <- bvec
    out$V_vs <- Vb
    # the margins themselves
    Mall <- do.call(rbind, lapply(res, function(r) r$Mc))
    mn <- paste0("m", seq_len(nrow(Mall)))
    crit0 <- if (small) stats::qt(1 - alpha / 2, df) else stats::qnorm(1 - alpha / 2)
    mt <- vapply(seq_len(nrow(Mall)), function(i) {
      l <- Mall[i, ]
      if (!estimable(l, margin = TRUE)) return(rep(NA_real_, 9))
      m <- sum(l * bb)
      se <- sqrt(max(drop(l %*% VV %*% l), 0))
      st <- m / se
      c(m, se, st, if (small) 2 * stats::pt(-abs(st), df) else 2 * stats::pnorm(-abs(st)), m - crit0 * se, m + crit0 * se, if (small) df else NA_real_, crit0, 0)
    }, numeric(9))
    mt <- matrix(mt, 9, dimnames = list(rownames(tab), mn))
    out$table <- mt
    out$b <- matrix(mt["b", ], 1, dimnames = list("r1", mn))
    out$V <- Mall %*% VV %*% t(Mall)
    dimnames(out$V) <- list(mn, mn)
  }
  if (!show) return(invisible(out))
  # ---- print
  printer <- function() {
  cat("\n", title, "\n\nMargins: asbalanced\n\n", sep = "")
  if (kind == "contrast") {
    adjcol <- mcomp != "noadjust" && any(vapply(res, function(r) length(r$tests) > 1, TRUE))
    labs <- unlist(lapply(res, function(r) if (length(r$tests) > 1) vapply(r$tests, function(e) e$label, "") else character()))
    W <- Wof(c(labs, "Joint"), vapply(res, function(r) r$label, ""))
    cols <- 34L + if (adjcol) 11L else 0L
    T <- W + 1L + cols
    sname <- if (small) "F" else "chi2"
    cat(strrep("-", T), "\n", sep = "")
    if (adjcol) cat(bar(W), sprintf("%*s", cols, method), "\n", sep = "")
    cat(bar(W), sprintf("%11s%12s%11s", "df", sname, paste0("P>", sname)), if (adjcol) sprintf("%11s", paste0("P>", sname)), "\n", sep = "")
    cat(sepline(W, cols), "\n", sep = "")
    for (i in seq_along(res)) {
      r <- res[[i]]
      if (i > 1) cat(bar(W), "\n", sep = "")
      body <- function(w, padj = NULL) {
        if (!isTRUE(w$ok)) return("  (not testable)")
        paste0(sprintf("%11d%12.2f%11.4f", as.integer(w$df), w$stat, w$p), if (adjcol) if (is.null(padj)) strrep(" ", 11) else sprintf("%11.4f", padj) else "")
      }
      if (length(r$tests) == 1) {
        cat(sprintf("%*s |", W - 1L, r$label), body(r$tr[[1]]), "\n", sep = "")
      } else {
        cat(sprintf("%*s |", W - 1L, r$label), "\n", sep = "")
        for (j in seq_along(r$tests)) cat(sprintf("%*s  |", W - 2L, r$tests[[j]]$label), body(r$tr[[j]], r$tr[[j]]$padj), "\n", sep = "")
        cat(sprintf("%*s  |", W - 2L, "Joint"), sub("\\s+$", "", body(r$joint)), "\n", sep = "")
      }
    }
    if (small) {
      cat(bar(W), "\n", sep = "")
      cat(sprintf("%*s |%11d", W - 1L, "Denominator", as.integer(df)), "\n", sep = "")
    }
    cat(strrep("-", T), "\n", sep = "")
    if (mcomp != "noadjust") {
      cat(paste(wrap(sprintf("Note: %s-adjusted p-values are reported for tests on individual contrasts only.", method), T), collapse = "\n"), "\n", sep = "")
    }
    cat("\n")
  }
  anyest <- any(vapply(res, function(r) isTRUE(r$show_est) && length(r$ests) > 0, TRUE))
  if (kind == "pwcompare" && cimargins) {
    # the margins themselves
    W <- Wof(unlist(lapply(res, function(r) r$cl)), vapply(res, function(r) r$label, ""))
    T <- W + 1L + 48L
    cat(strrep("-", T), "\n", sep = "")
    cat(bar(W), center(method, 38), "\n", sep = "")
    cat(bar(W), sprintf("%11s%12s%25s", "Margin", "Std. err.", cilab), "\n", sep = "")
    cat(sepline(W, 48), "\n", sep = "")
    if (nzchar(eq)) cat(sprintf("%-*s|", W, eq), "\n", sep = "")
    for (r in res) {
      cat(sprintf("%*s |", W - 1L, r$label), "\n", sep = "")
      for (j in seq_len(nrow(r$Mc))) {
        l <- r$Mc[j, ]
        m <- sum(l * bb)
        se <- sqrt(max(drop(l %*% VV %*% l), 0))
        crit <- if (small) stats::qt(1 - alpha / 2, df) else stats::qnorm(1 - alpha / 2)
        cat(sprintf("%*s  |", W - 2L, r$cl[j]), sprintf("%11s%11s%14s%12s", g(m), g(se), g(m - crit * se), g(m + crit * se)), "\n", sep = "")
      }
    }
    cat(strrep("-", T), "\n\n", sep = "")
    return(invisible(out))
  }
  if (anyest && mcomp != "noadjust") {
    W <- Wof(character(), c(vapply(res, function(r) if (isTRUE(r$show_est) && length(r$ests)) r$label else "", ""), if (nzchar(eq)) eq))
    cat(strrep("-", W + 14L), "\n", sep = "")
    cat(bar(W), sprintf("%13s", "Number of"), "\n", sep = "")
    cat(bar(W), sprintf("%13s", "comparisons"), "\n", sep = "")
    cat(sepline(W, 13), "\n", sep = "")
    if (nzchar(eq)) cat(sprintf("%-*s|", W, eq), "\n", sep = "")
    for (r in res) if (isTRUE(r$show_est) && length(r$ests)) cat(sprintf("%*s |%13d", W - 1L, r$label, length(r$ests)), "\n", sep = "")
    cat(strrep("-", W + 14L), "\n\n", sep = "")
  }
  if (anyest) {
    labs <- unlist(lapply(res, function(r) vapply(r$ests, function(e) e$label, "")))
    W <- Wof(labs, vapply(res, function(r) r$label, ""))
    cols <- if (effects) 64L else 48L
    T <- W + 1L + cols
    cat(strrep("-", T), "\n", sep = "")
    if (kind == "pwcompare" || mcomp != "noadjust") {
      if (effects) cat(bar(W), center(method, 33), strrep(" ", 21 - nchar(method)), method, "\n", sep = "") else cat(bar(W), center(method, 38), "\n", sep = "")
    }
    what <- if (kind == "pwcompare" && FALSE) "Margin" else "Contrast"
    stn <- if (small) "t" else "z"
    cat(bar(W), if (effects) sprintf("%11s%12s%7s%9s%25s", what, "Std. err.", stn, paste0("P>|", stn, "|"), cilab) else sprintf("%11s%12s%25s", what, "Std. err.", cilab), "\n", sep = "")
    cat(sepline(W, cols), "\n", sep = "")
    if (nzchar(eq) && kind == "pwcompare") cat(sprintf("%-*s|", W, eq), "\n", sep = "")
    first <- TRUE
    for (r in res) {
      if (!isTRUE(r$show_est) || !length(r$ests)) next
      if (!first && kind == "pwcompare") cat(bar(W), "\n", sep = "")
      first <- FALSE
      cat(sprintf("%*s |", W - 1L, r$label), "\n", sep = "")
      for (j in seq_along(r$ests)) {
        e <- r$er[[j]]
        body <- if (!e$ok) sprintf("%11s  (not estimable)", ".") else if (effects) {
          sprintf("%11s%11s%9.2f%8.3f%13s%12s", g(e$est), g(e$se), e$st, e$p, g(e$lo), g(e$hi))
        } else sprintf("%11s%11s%14s%12s", g(e$est), g(e$se), g(e$lo), g(e$hi))
        cat(sprintf("%*s  |", W - 2L, r$ests[[j]]$label), body, "\n", sep = "")
      }
    }
    cat(strrep("-", T), "\n\n", sep = "")
  }
  }
  txt <- utils::capture.output(printer())
  while (length(txt) && !nzchar(txt[length(txt)])) txt <- txt[-length(txt)]
  cat(txt, sep = "\n")
  cat("\n")
  invisible(out)
}
"##;

/// The r() `contrast` and `pwcompare` leave (the scalars and texts; the matrices
/// are read from the list as they are).
const R_CONTRAST: &[&str] = &[
    "level",
    "k_terms",
    "df_r",
    "balanced",
    "mcmethod",
    "mcmethod_vs",
    "mctitle_vs",
    "cmd",
    "title",
    "margin_method",
    "est_cmdline",
    "est_cmd",
    "cmdline",
    "emptycells",
];

/// One term of `contrast` or `pwcompare`: `r.g#h@k` as R's `list(vars, ops, at)`.
struct Term {
    vars: Vec<String>,
    ops: Vec<&'static str>,
    at: Vec<String>,
}

fn parse_term(tok: &str, pw: bool) -> Result<Term, Untranslated> {
    let (main, at) = match tok.split_once('@') {
        Some((m, a)) => (m, Some(a)),
        None => (tok, None),
    };
    if pw && at.is_some() {
        return Err(no("pwcompare with @ is not translated yet"));
    }
    let mut vars = Vec::new();
    let mut ops = Vec::new();
    for part in main.split('#') {
        let (op, v): (&'static str, &str) = if let Some(v) = part.strip_prefix("ar.") {
            ("ar", v)
        } else if let Some(v) = part.strip_prefix("gw.") {
            ("gw", v)
        } else if let Some(v) = part.strip_prefix("r.") {
            ("r", v)
        } else if let Some(v) = part.strip_prefix("g.") {
            ("g", v)
        } else if let Some(v) = part.strip_prefix("a.") {
            ("a", v)
        } else {
            ("", part.strip_prefix("i.").unwrap_or(part))
        };
        if pw && !op.is_empty() {
            return Err(no(format!(
                "pwcompare {tok}: contrast operators are not translated"
            )));
        }
        if !is_name(v) {
            return Err(no(format!(
                "{tok}: only factor variables, joined by # and @, are translated"
            )));
        }
        if vars.iter().any(|x| x == v) {
            return Err(no(format!("{tok}: a variable twice in one term")));
        }
        vars.push(v.to_string());
        ops.push(op);
    }
    let mut ats = Vec::new();
    if let Some(a) = at {
        for part in a.split('#') {
            if !is_name(part) || vars.iter().any(|x| x == part) {
                return Err(no(format!(
                    "{tok}: only a factor variable after @ is translated"
                )));
            }
            ats.push(part.to_string());
        }
        if ats.len() > 1 {
            return Err(no(format!(
                "{tok}: more than one variable after @ is not translated yet"
            )));
        }
    }
    Ok(Term { vars, ops, at: ats })
}

impl Session {
    /// `contrast` and `pwcompare`.
    pub(super) fn contrast_cmd(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let name = c.name.as_str();
        let pw = name == "pwcompare";
        self.no_by(c, name)?;
        let est = self.estimates_for(name)?;
        if !matches!(
            est.fit,
            Fit::Regress | Fit::Xtreg | Fit::Areg | Fit::Logit | Fit::Probit | Fit::Poisson
        ) {
            return Err(no(format!(
                "{name} after {} is not translated yet",
                est.fit.name()
            )));
        }
        if self.svy_fit {
            return Err(no(format!(
                "{name} after a svy: model is not translated yet"
            )));
        }
        if let Some(o) = opts.other(&[
            ("effects", 7),
            ("level", 3),
            ("mcompare", 5),
            ("cimargins", 9),
        ]) {
            return Err(no(format!(
                "{name} with the option {o} is not translated yet"
            )));
        }
        if !pw && opts.has("cimargins", 9) {
            return Err(no("contrast, cimargins is not an option (Stata: r(198))"));
        }
        let effects = opts.has("effects", 7);
        let cimargins = opts.has("cimargins", 9);
        if cimargins && effects {
            return Err(no(
                "pwcompare with cimargins and effects together is not translated yet",
            ));
        }
        let mcomp = match opts.arg("mcompare", 5).as_deref().map(str::trim) {
            None | Some("noadjust") => "noadjust",
            Some(m) if m.len() >= 3 && "bonferroni".starts_with(m) => "bonferroni",
            Some(m) if m.len() >= 3 && "sidak".starts_with(m) => "sidak",
            Some(m) if m.len() >= 3 && "scheffe".starts_with(m) => "scheffe",
            Some(m) => {
                return Err(no(format!(
                    "{name}, mcompare({m}) is not translated yet: noadjust, bonferroni, sidak and scheffe are"
                )))
            }
        };
        let level = super::model::level(opts)? * 100.0;
        let toks: Vec<&str> = c.parts.main.split_whitespace().collect();
        if toks.is_empty() {
            return Err(no(format!("{name} needs a term")));
        }
        let mut terms = Vec::new();
        for t in &toks {
            terms.push(parse_term(t, pw)?);
        }
        // a variable set twice: Stata merges them, not translated
        for (i, a) in terms.iter().enumerate() {
            for b in terms.iter().skip(i + 1) {
                if a.vars == b.vars && a.at == b.at {
                    return Err(no(format!(
                        "{name}: one term given twice is not translated yet"
                    )));
                }
            }
        }
        for t in &terms {
            for v in t.vars.iter().chain(t.at.iter()) {
                if !est.factors.contains_key(v) {
                    out.line(format!(
                        "stop({}, call. = FALSE)",
                        r_text(&format!(
                            "factor {v} not found in list of covariates (Stata: r(111))"
                        ))
                    ));
                    return Ok(());
                }
            }
            if t.ops.contains(&"gw") && self.weighted_fit {
                return Err(no(
                    "contrast gw. after a weighted model is not translated yet",
                ));
            }
            if t.vars.len() + t.at.len() > 3 {
                return Err(no(format!(
                    "{name} with more than three variables in a term is not translated yet"
                )));
            }
            if effects && !t.at.is_empty() && t.ops.contains(&"") {
                return Err(no(
                    "contrast, effects of a term with @ is not translated yet",
                ));
            }
        }
        let rows = if !self.changed_since_fit {
            "stata_data".to_string()
        } else if self.esample {
            format!("{}[stata_esample, , drop = FALSE]", self.data()?)
        } else {
            return Err(no(format!(
                "{name} after a command that may have changed the rows since {} is not translated yet",
                est.fit.name()
            )));
        };
        let factors: Vec<String> = est
            .factors
            .iter()
            .map(|(v, expr)| {
                let base = super::margins::factor_base(expr)
                    .map_or("NA_character_".to_string(), |b| r_text(&b));
                format!("{} = {base}", r_text(v))
            })
            .collect();
        let terms_r: Vec<String> = terms
            .iter()
            .map(|t| {
                let q = |v: &Vec<String>| {
                    if v.is_empty() {
                        "character()".to_string()
                    } else {
                        format!(
                            "c({})",
                            v.iter().map(|x| r_text(x)).collect::<Vec<_>>().join(", ")
                        )
                    }
                };
                format!(
                    "list(vars = {}, ops = c({}), at = {})",
                    q(&t.vars),
                    t.ops
                        .iter()
                        .map(|o| r_text(o))
                        .collect::<Vec<_>>()
                        .join(", "),
                    q(&t.at)
                )
            })
            .collect();
        for h in ["stata_contrast", "stata_fmt"] {
            self.helpers.insert(h);
        }
        let eq = if matches!(est.fit, Fit::Regress | Fit::Xtreg | Fit::Areg) {
            String::new()
        } else {
            est.dep.clone()
        };
        // areg keeps no lm: its formula, factor levels and absorbed variable stand in, as for xtreg, fe
        let fit_r = if est.fit == Fit::Areg {
            "list(terms = stats::terms(attr(stata_e, \"stata_areg\")$fo), xlevels = attr(stata_e, \"stata_areg\")$xlev, panel = attr(stata_e, \"stata_areg\")$absorb, type = \"fe\")"
        } else {
            "stata_estimates"
        };
        out.line(format!(
            "stata_r <- stata_contrast({}, {fit_r}, {rows}, stata_b, stata_vcov, stata_df, list({}), c({}), effects = {}, cimargins = {}, mcomp = {}, level = {}, eq = {}, cmdline = {}, est_cmd = {}, est_cmdline = if (exists(\"stata_e\")) stata_e$cmdline else \"\"{})",
            r_text(name),
            terms_r.join(", "),
            factors.join(", "),
            super::stats::r_logical(effects),
            super::stats::r_logical(cimargins),
            r_text(mcomp),
            r_number(level),
            r_text(&eq),
            r_text(c.text.trim()),
            r_text(est.fit.name()),
            if self.quiet(c) { ", show = FALSE" } else { "" }
        ));
        self.r_new = Some((R_CONTRAST, name.to_string()));
        out.caveat(
            "margins asbalanced: the other factors averaged with equal weights, the covariates at zero",
        );
        Ok(())
    }
}

/// `hausman`: the Hausman specification test of two stored sets of results
/// (`plan/stata/postest.md`).
pub(super) const HAUSMAN_HELPER: &str = r##"stata_hausman <- function(s1, s2, n1, n2, cmd1, cmd2, constant = FALSE, sig = "", show = TRUE) {
  part <- function(p) {
    if (p == "(Intercept)") return("_cons")
    m <- regmatches(p, regexec("^factor\\(([^()]+)\\)(.+)$", p))[[1]]
    if (length(m)) return(paste0(m[3], ".", m[2]))
    m <- regmatches(p, regexec("^stata_ib\\(([^,]+), \"[^\"]*\"\\)(.+)$", p))[[1]]
    if (length(m)) return(paste0(m[3], ".", m[2]))
    p
  }
  prep <- function(s) {
    b <- s$stata_b
    V <- as.matrix(s$stata_vcov)
    if (!is.null(rownames(V)) && all(names(b) %in% rownames(V))) V <- V[names(b), names(b), drop = FALSE]
    nm <- vapply(names(b), function(n) paste(vapply(strsplit(n, ":", fixed = TRUE)[[1]], part, ""), collapse = "#"), "")
    list(b = unname(b), V = V, nm = unname(nm), rmse = if (!is.null(s$stata_estimates$rmse_gls)) s$stata_estimates$rmse_gls else s$stata_e[["rmse"]])
  }
  a <- prep(s1)
  z <- prep(s2)
  common <- list()
  for (i in seq_along(a$nm)) {
    nm <- a$nm[i]
    if (nm == "_cons" && !constant) next
    j <- match(nm, z$nm)
    if (!is.na(j) && a$V[i, i] != 0 && z$V[j, j] != 0) common[[length(common) + 1]] <- c(i, j)
  }
  if (!length(common)) stop("no coefficients in common; specify equations (Stata: r(498))", call. = FALSE)
  ratio <- if (nzchar(sig)) {
    if (is.null(a$rmse) || is.null(z$rmse)) stop("sigmamore and sigmaless need e(rmse), which is not kept here", call. = FALSE)
    (z$rmse / a$rmse)^2
  } else 1
  w1 <- if (sig == "sigmamore") ratio else 1
  w2 <- if (sig == "sigmaless") 1 / ratio else 1
  ia <- vapply(common, function(x) x[1], 0)
  ib <- vapply(common, function(x) x[2], 0)
  d <- a$b[ia] - z$b[ib]
  Vd <- w1 * a$V[ia, ia, drop = FALSE] - w2 * z$V[ib, ib, drop = FALSE]
  # the inverse by Gauss-Jordan, a column with no usable pivot left out
  n <- length(d)
  scale <- max(abs(diag(Vd)))
  m <- cbind(Vd, diag(n))
  used <- rep(FALSE, n)
  pivot_of <- rep(NA_integer_, n)
  rank <- 0L
  for (cc in seq_len(n)) {
    cand <- which(!used)
    if (!length(cand)) next
    piv <- cand[which.max(abs(m[cand, cc]))]
    pivot <- m[piv, cc]
    if (abs(pivot) <= 1e-13 * max(scale, 1e-300)) next
    rank <- rank + 1L
    used[piv] <- TRUE
    pivot_of[cc] <- piv
    row <- m[piv, ] / pivot
    for (i in seq_len(n)) if (i != piv) m[i, ] <- m[i, ] - m[i, cc] * row
    m[piv, ] <- row
  }
  inv <- t(vapply(seq_len(n), function(cc) if (is.na(pivot_of[cc])) numeric(n) else m[pivot_of[cc], n + seq_len(n)], numeric(n)))
  chi2 <- drop(t(d) %*% inv %*% d)
  p <- if (chi2 < 0) 1 else stats::pchisq(chi2, rank, lower.tail = FALSE)
  r <- list(rank = rank, chi2 = chi2, df = rank, p = p)
  if (!show) return(invisible(r))
  g9 <- function(v) stata_fmt(v, "%9.0g", trim = TRUE)
  centred <- function(s) {
    len <- min(nchar(s), 10L)
    left <- (10L - len) %/% 2L
    paste0(strrep(" ", left), s, strrep(" ", 10L - len - left))
  }
  cat("\n                 ---- Coefficients ----\n")
  cat("             |      (b)          (B)            (b-B)     sqrt(diag(V_b-V_B))\n")
  cat(sprintf("             |   %s   %s     Difference       Std. err.\n", centred(n1), centred(n2)))
  cat("-------------+----------------------------------------------------------------\n")
  for (k in seq_len(n)) {
    var <- Vd[k, k]
    cat(sprintf("%12s |%12s%13s%16s%16s\n", substr(a$nm[ia[k]], 1, 12), g9(a$b[ia[k]]), g9(z$b[ib[k]]), g9(d[k]), if (var > 0) g9(sqrt(var)) else "."))
  }
  cat(strrep("-", 78), "\n", sep = "")
  cat(sprintf("%78s\n", sprintf("b = Consistent under H0 and Ha; obtained from %s.", cmd1)))
  cat(sprintf("%78s\n", sprintf("B = Inconsistent under Ha, efficient under H0; obtained from %s.", cmd2)))
  cat("\nTest of H0: Difference in coefficients not systematic\n\n")
  label <- sprintf("chi2(%d)", rank)
  wd <- if (chi2 < 0) nchar(label) else 11L
  cat(sprintf("%*s = (b-B)'[(V_b-V_B)^(-1)](b-B)\n", wd, label))
  cat(sprintf("%*s = %6.2f\n", wd, "", chi2))
  if (chi2 < 0) {
    cat("\nWarning: chi2 < 0 ==> model fitted on these data\n         fails to meet the asymptotic assumptions\n         of the Hausman test; see suest for a\n         generalized test.\n")
  } else {
    cat(sprintf("Prob > chi2 = %6.4f\n", p))
  }
  invisible(r)
}"##;

/// `lrtest`: the likelihood-ratio test of two stored sets of results.
pub(super) const LRTEST_HELPER: &str = r##"stata_lrtest <- function(s1, s2, n1, n2, show = TRUE) {
  e1 <- s1$stata_e
  e2 <- s2$stata_e
  ll1 <- e1[["ll"]]
  ll2 <- e2[["ll"]]
  k1 <- e1[["rank"]]
  k2 <- e2[["rank"]]
  if (is.null(ll1) || is.null(ll2) || is.null(k1) || is.null(k2) || !is.finite(ll1) || !is.finite(ll2)) {
    stop("likelihood information not found (Stata: r(498))", call. = FALSE)
  }
  if (k1 == k2) stop(sprintf("df(unrestricted) = df(restricted) = %s (Stata: r(498))", format(k1)), call. = FALSE)
  small <- if (k1 >= k2) n2 else n1
  big <- if (k1 >= k2) n1 else n2
  df <- abs(k1 - k2)
  chi2 <- 2 * abs(ll1 - ll2)
  p <- stats::pchisq(chi2, df, lower.tail = FALSE)
  r <- list(p = p, chi2 = chi2, df = df)
  if (!show) return(invisible(r))
  cat("\nLikelihood-ratio test\n")
  cat(sprintf("Assumption: %s nested within %s\n\n", small, big))
  cat(sprintf("%11s = %6.2f\n", sprintf("LR chi2(%s)", format(df)), chi2))
  cat(sprintf("Prob > chi2 = %6.4f\n", p))
  invisible(r)
}"##;

const R_HAUSMAN: &[&str] = &["rank", "chi2", "df", "p"];
const R_LRTEST: &[&str] = &["p", "chi2", "df"];

impl Session {
    /// The results `hausman` and `lrtest` read: stored under a name, or the
    /// active ones (`.`), as an R expression and the model kind.
    fn stored_or_active(
        &self,
        what: &str,
        name: &str,
        out: &mut Out,
    ) -> Result<Option<(String, Fit)>, Untranslated> {
        if name == "." {
            let est = self.estimates_for(what)?;
            return Ok(Some((
                "list(stata_b = stata_b, stata_vcov = stata_vcov, stata_e = if (exists(\"stata_e\")) stata_e else list())"
                    .to_string(),
                est.fit,
            )));
        }
        match self.stored_fit(name) {
            Some(f) => {
                let est = f.estimates().ok_or_else(|| {
                    no(format!(
                        "{what}: the results stored as {name} are not held here"
                    ))
                })?;
                Ok(Some((format!("stata_stored[[{}]]", r_text(name)), est.fit)))
            }
            None if self.stores_lost => Err(no(format!(
                "{what} {name}: no line translated before it stored {name}, and one that was not may have"
            ))),
            None => {
                out.line(format!(
                    "stop({})",
                    r_text(&format!("estimation result {name} not found"))
                ));
                Ok(None)
            }
        }
    }

    /// `hausman consistent [efficient] [, constant sigmamore sigmaless]`.
    pub(super) fn hausman_cmd(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "hausman")?;
        if let Some(o) = opts.other(&[("constant", 4), ("sigmamore", 6), ("sigmaless", 6)]) {
            return Err(no(format!(
                "hausman with the option {o} is not translated yet"
            )));
        }
        if opts.has("sigmamore", 6) && opts.has("sigmaless", 6) {
            return Err(no(
                "hausman: sigmamore and sigmaless together (Stata stops)",
            ));
        }
        let words: Vec<&str> = c.parts.main.split_whitespace().collect();
        let (a, b) = match words.as_slice() {
            [a] => (*a, "."),
            [a, b] => (*a, *b),
            _ => return Err(no("hausman needs one or two names of stored results")),
        };
        for n in [a, b] {
            if n != "." && !is_name(n) {
                return Err(no(format!("hausman {n}: not a name of stored results")));
            }
        }
        let Some((s1, f1)) = self.stored_or_active("hausman", a, out)? else {
            return Ok(());
        };
        let Some((s2, f2)) = self.stored_or_active("hausman", b, out)? else {
            return Ok(());
        };
        let ok = |f: Fit| {
            matches!(
                f,
                Fit::Xtreg
                    | Fit::Regress
                    | Fit::Ivregress
                    | Fit::Logit
                    | Fit::Probit
                    | Fit::Poisson
            )
        };
        if !ok(f1) || !ok(f2) {
            return Err(no(format!(
                "hausman after {} and {} is not translated yet",
                f1.name(),
                f2.name()
            )));
        }
        let sig = if opts.has("sigmamore", 6) {
            "sigmamore"
        } else if opts.has("sigmaless", 6) {
            "sigmaless"
        } else {
            ""
        };
        self.helpers.insert("stata_hausman");
        self.helpers.insert("stata_fmt");
        out.line(format!(
            "stata_r <- stata_hausman({s1}, {s2}, {}, {}, {}, {}, constant = {}, sig = {}{})",
            r_text(a),
            r_text(b),
            r_text(f1.name()),
            r_text(f2.name()),
            super::stats::r_logical(opts.has("constant", 4)),
            r_text(sig),
            if self.quiet(c) { ", show = FALSE" } else { "" }
        ));
        self.r_new = Some((R_HAUSMAN, "hausman".into()));
        Ok(())
    }

    /// `lrtest name1 [name2]`.
    pub(super) fn lrtest_cmd(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "lrtest")?;
        if let Some(o) = opts.other(&[]) {
            return Err(no(format!(
                "lrtest with the option {o} is not translated yet"
            )));
        }
        let words: Vec<&str> = c.parts.main.split_whitespace().collect();
        let (a, b) = match words.as_slice() {
            [a] => (*a, "."),
            [a, b] => (*a, *b),
            _ => return Err(no("lrtest needs one or two names of stored results")),
        };
        for n in [a, b] {
            if n != "." && !is_name(n) {
                return Err(no(format!("lrtest {n}: not a name of stored results")));
            }
        }
        if a == b {
            out.line(format!(
                "stop({})",
                r_text(&format!(
                    "models {a} specified more than once (Stata: r(198))"
                ))
            ));
            return Ok(());
        }
        let Some((s1, f1)) = self.stored_or_active("lrtest", a, out)? else {
            return Ok(());
        };
        let Some((s2, f2)) = self.stored_or_active("lrtest", b, out)? else {
            return Ok(());
        };
        let ok = |f: Fit| matches!(f, Fit::Regress | Fit::Logit | Fit::Probit | Fit::Poisson);
        if !ok(f1) || !ok(f2) {
            return Err(no(format!(
                "lrtest after {} and {} is not translated yet",
                f1.name(),
                f2.name()
            )));
        }
        // a robust model: Stata stops ("LR test likely invalid for models with robust VCE")
        let robust = |n: &str, s: &Session| {
            if n == "." {
                s.fit_robust || s.fit_cluster || s.weighted_fit
            } else {
                s.stored_fit(n).is_some_and(|f| f.robust_or_weighted())
            }
        };
        if robust(a, self) || robust(b, self) {
            return Err(no(
                "lrtest after a robust, clustered or weighted model is not translated yet",
            ));
        }
        self.helpers.insert("stata_lrtest");
        out.line(format!(
            "stata_r <- stata_lrtest({s1}, {s2}, {}, {}{})",
            r_text(a),
            r_text(b),
            if self.quiet(c) { ", show = FALSE" } else { "" }
        ));
        self.r_new = Some((R_LRTEST, "lrtest".into()));
        Ok(())
    }
}

/// `predictnl`: a nonlinear prediction at each row with its delta-method
/// standard error, the derivatives taken numerically at the coefficients
/// (central differences, extrapolated, as `nlcom` takes them).
pub(super) const PREDICTNL_HELPER: &str = r##"stata_predictnl <- function(d, fit, b, V, valf, condf, preds, link, level = 95, want = character()) {
  env <- environment(stata_bget)
  b0 <- stata_b
  on.exit(assign("stata_b", b0, envir = env), add = TRUE)
  tt <- stats::delete.response(stats::terms(fit))
  mf <- stats::model.frame(tt, d, xlev = fit$xlevels, na.action = stats::na.pass)
  X <- suppressWarnings(stats::model.matrix(tt, mf, contrasts.arg = fit$contrasts))
  X <- X[, names(stats::coef(fit)), drop = FALSE]
  linkinv <- switch(link, logit = stats::plogis, probit = stats::pnorm, log = exp, identity = function(z) z)
  at <- function(bv) {
    assign("stata_b", bv, envir = env)
    dd <- d
    xb <- drop(X %*% bv[colnames(X)])
    for (nm in names(preds)) dd[[nm]] <- if (preds[[nm]] == "xb") xb else linkinv(xb)
    v <- as.numeric(valf(dd))
    if (!is.null(condf)) {
      cnd <- condf(dd)
      v[is.na(cnd) | !cnd] <- NA_real_
    }
    v
  }
  est <- at(b0)
  k <- length(b0)
  J <- matrix(0, length(est), k)
  for (j in seq_len(k)) {
    if (is.na(b0[j])) next
    dif <- function(h) {
      up <- b0
      dn <- b0
      up[j] <- up[j] + h
      dn[j] <- dn[j] - h
      (at(up) - at(dn)) / (2 * h)
    }
    h <- 1e-3 * (abs(b0[j]) + 1e-3)
    J[, j] <- (4 * dif(h / 2) - dif(h)) / 3
  }
  Vb <- as.matrix(V)
  if (!is.null(rownames(Vb)) && all(names(b0) %in% rownames(Vb))) Vb <- Vb[names(b0), names(b0), drop = FALSE]
  Vb[is.na(Vb)] <- 0
  var <- rowSums((J %*% Vb) * J)
  var[is.na(est) | !is.finite(var)] <- NA_real_
  se <- sqrt(var)
  crit <- stats::qnorm((100 + level) / 200)
  wald <- (est / se)^2
  list(est = est, se = se, var = var, wald = wald, p = stats::pchisq(wald, 1, lower.tail = FALSE),
       lo = est - crit * se, hi = est + crit * se)
}"##;

const PREDICTNL_NAMES: &[(&str, &str, usize)] = &[
    ("se", "se", 2),
    ("variance", "var", 3),
    ("wald", "wald", 4),
    ("p", "p", 1),
];

/// `xb()` and `predict(type)` in the text as names of columns (`_pnl_xb`,
/// `_pnl_pr`), with the prediction each stands for.
fn pnl_substitute(
    exp: &str,
    fit: Fit,
) -> Result<(String, BTreeMap<String, &'static str>), Untranslated> {
    let default = match fit {
        Fit::Regress => "xb",
        Fit::Logit | Fit::Probit => "pr",
        _ => "n",
    };
    let mut preds: BTreeMap<String, &'static str> = BTreeMap::new();
    let mut out = String::new();
    let mut rest = exp;
    while let Some(i) = rest.find("predict(").or_else(|| rest.find("xb()")) {
        let (head, tail) = rest.split_at(i);
        out.push_str(head);
        // not part of a longer name (`mypredict(`)
        let glued = head
            .chars()
            .last()
            .is_some_and(|ch| ch.is_alphanumeric() || ch == '_');
        if glued {
            let n = if tail.starts_with("xb()") { 4 } else { 8 };
            out.push_str(&tail[..n]);
            rest = &tail[n..];
            continue;
        }
        let (kind, after): (String, &str) = if let Some(t) = tail.strip_prefix("xb()") {
            ("xb".into(), t)
        } else {
            let inner = tail.get(8..).unwrap_or("");
            let close = inner
                .find(')')
                .ok_or_else(|| no("predictnl: a predict( without its )"))?;
            let opt = inner.get(..close).unwrap_or("").trim();
            if opt.contains(',') || opt.contains(' ') {
                return Err(no(format!(
                    "predictnl: predict({opt}) is not translated yet"
                )));
            }
            (
                if opt.is_empty() {
                    default.into()
                } else {
                    opt.to_string()
                },
                inner.get(close + 1..).unwrap_or(""),
            )
        };
        let ok = match fit {
            Fit::Regress => kind == "xb",
            Fit::Logit | Fit::Probit => kind == "xb" || kind == "pr",
            _ => kind == "xb" || kind == "n",
        };
        if !ok {
            return Err(no(format!(
                "predictnl: predict({kind}) after this model is not translated yet"
            )));
        }
        let name = format!("_pnl_{kind}");
        preds.insert(name.clone(), if kind == "xb" { "xb" } else { "mean" });
        out.push_str(&name);
        rest = after;
    }
    out.push_str(rest);
    Ok((out, preds))
}

impl Session {
    /// `predictnl [type] newvar = exp [if] [in] [, se() variance() wald() p() ci() level()]`.
    pub(super) fn predictnl_cmd(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "predictnl")?;
        let est = self.estimates_for("predictnl")?;
        if !matches!(
            est.fit,
            Fit::Regress | Fit::Logit | Fit::Probit | Fit::Poisson
        ) {
            return Err(no(format!(
                "predictnl after {} is not translated yet",
                est.fit.name()
            )));
        }
        if self.svy_fit {
            return Err(no("predictnl after a svy: model is not translated yet"));
        }
        if let Some(o) = opts.other(&[
            ("se", 2),
            ("variance", 3),
            ("wald", 4),
            ("p", 1),
            ("ci", 2),
            ("level", 3),
        ]) {
            return Err(no(format!(
                "predictnl with the option {o} is not translated yet"
            )));
        }
        let (lhs, rhs) = c
            .parts
            .main
            .split_once('=')
            .ok_or_else(|| no("predictnl needs `newvar = exp`"))?;
        let words: Vec<&str> = lhs.split_whitespace().collect();
        let (ty, var) = match words.as_slice() {
            [v] => ("float", *v),
            [t, v] if matches!(*t, "float" | "double") => (*t, *v),
            _ => return Err(no("predictnl needs one new variable name")),
        };
        if !is_name(var) {
            return Err(no(format!("{var} is not a variable name")));
        }
        let level = super::model::level(opts)? * 100.0;
        // the new variables, in the order Stata makes them
        let mut news: Vec<(String, &str, bool)> = vec![(var.to_string(), "est", ty == "double")];
        for (opt, key, min) in PREDICTNL_NAMES {
            if let Some(n) = opts.arg(opt, *min) {
                let n = n.trim();
                if !is_name(n) {
                    return Err(no(format!("{opt}({n}) needs a new variable name")));
                }
                news.push((n.to_string(), key, false));
            }
        }
        let mut ci = false;
        if let Some(t) = opts.arg("ci", 2) {
            let w: Vec<&str> = t.split_whitespace().collect();
            let [lo, hi] = w.as_slice() else {
                return Err(no("ci() requires two new variable names (Stata: r(198))"));
            };
            for (n, key) in [(*lo, "lo"), (*hi, "hi")] {
                if !is_name(n) {
                    return Err(no(format!("ci({t}) needs new variable names")));
                }
                news.push((n.to_string(), key, false));
            }
            ci = true;
        }
        let d = self.data()?;
        let (text, preds) = pnl_substitute(rhs.trim(), est.fit)?;
        for (n, _, _) in &news {
            out.line(format!(
                "if ({} %in% names({d})) stop(\"variable {n} already defined\")",
                r_text(n)
            ));
        }
        // the predictions are columns while the expression is read
        let added: Vec<String> = preds
            .keys()
            .filter(|n| !self.kinds.contains_key(*n))
            .cloned()
            .collect();
        for n in &added {
            self.kinds.insert(n.clone(), Kind::Number);
        }
        let read = self.drawing_value(c, &text, out);
        for n in &added {
            self.kinds.remove(n);
        }
        let (cond, value, kind) = read?;
        if kind == Kind::Text {
            return Err(no(
                "predictnl of a string expression (Stata: type mismatch)",
            ));
        }
        if value.contains("stata_v = ") {
            return Err(no("predictnl: this expression is not translated yet"));
        }
        let link = match est.fit {
            Fit::Logit => "logit",
            Fit::Probit => "probit",
            Fit::Poisson => "log",
            _ => "identity",
        };
        let pred_r = if preds.is_empty() {
            "character()".to_string()
        } else {
            format!(
                "c({})",
                preds
                    .iter()
                    .map(|(n, k)| format!("{} = {}", r_text(n), r_text(k)))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        out.package("dplyr");
        self.helpers.insert("stata_predictnl");
        self.helpers.insert("stata_float");
        self.helpers.insert("stata_chg");
        self.helpers.insert("stata_mv");
        out.line(format!(
            "stata_pn <- stata_predictnl({d}, stata_estimates, stata_b, stata_vcov, function(.d) dplyr::mutate(.d, .stata_pnl = {value})[[\".stata_pnl\"]], {}, {pred_r}, {}, level = {})",
            match &cond {
                Some(cnd) => format!("function(.d) dplyr::mutate(.d, .stata_pnc = {cnd})[[\".stata_pnc\"]]"),
                None => "NULL".to_string(),
            },
            r_text(link),
            r_number(level)
        ));
        for (n, key, double) in &news {
            let col = format!("stata_pn[[{}]]", r_text(key));
            let col = if *double {
                col
            } else {
                format!("stata_float({col})")
            };
            out.line(format!("{d}[[{}]] <- {col}", r_text(n)));
            self.kinds.insert(n.clone(), Kind::Number);
            self.storage.insert(
                n.clone(),
                if *double {
                    Storage::Double
                } else {
                    Storage::Float
                },
            );
            self.lost_vars.remove(n);
        }
        if !self.quiet(c) {
            out.line(format!("stata_gen_note({d}[[{}]])", r_text(var)));
            out.line(format!(
                "if (sum(!is.na({d}[[{v}]])) > 1 && all({d}[[{v}]][!is.na({d}[[{v}]])] == {d}[[{v}]][!is.na({d}[[{v}]])][1])) cat(\"warning: prediction constant over observations; perhaps you meant to run nlcom.\\n\")",
                v = r_text(var)
            ));
            if ci {
                out.line(
                    "cat(\"note: confidence intervals calculated using Z critical values.\\n\")",
                );
            }
            if opts.has("p", 1) {
                out.line("cat(\"note: p-values are with respect to the chi-squared(1) distribution.\\n\")");
            }
        }
        out.caveat("delta-method standard errors from numerical derivatives of the coefficients");
        Ok(())
    }
}
