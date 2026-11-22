
# ------------------------------------------------------------------ nlogitgen, nlogittree, nlogit
# the groups of a nested logit tree from a variable's value labels: nlogitgen newvar = altvar(g1: a | b, g2: c | d)
stata_nlogitgen <- function(d, newvar, altvar, groups) {
  if (newvar %in% names(d)) stop("variable ", newvar, " already defined (Stata: r(110))", call. = FALSE)
  if (!altvar %in% names(d)) stop("variable ", altvar, " not found (Stata: r(111))", call. = FALSE)
  lab <- attr(d[[altvar]], "labels")
  a <- stata_mx_num(d[[altvar]])
  lv <- sort(unique(a[!is.na(a)]))
  txt <- stata_mx_names(lv, lab)
  g <- rep(NA_real_, nrow(d))
  for (k in seq_along(groups)) {
    for (member in groups[[k]]$members) {
      v <- if (member %in% txt) lv[match(member, txt)] else suppressWarnings(as.numeric(member))
      if (is.na(v) || !v %in% lv) stop("no observations for ", altvar, " == ", member, " (Stata: r(198))", call. = FALSE)
      g[!is.na(a) & a == v] <- k
    }
  }
  if (anyNA(g[!is.na(a)])) stop("every alternative needs a group (Stata: r(198))", call. = FALSE)
  d[[newvar]] <- g
  gl <- vapply(groups, function(x) x$name, "")
  attr(d[[newvar]], "labels") <- stats::setNames(seq_along(gl), gl)
  cat(sprintf("New variable %s is generated with %d groups\n", newvar, length(gl)))
  cat(sprintf("label list lb_%s\nlb_%s:\n", newvar, newvar))
  for (k in seq_along(gl)) cat(sprintf("%12d %s\n", k, gl[k]))
  d
}
# the tree's table: groups, alternatives, observations and the number of times each is chosen
stata_nlogit_tree_text <- function(tr) {
  cat("\nTree structure specified for the nested logit model\n\n")
  hasn <- !is.null(tr$k)
  cat(sprintf(" %-6s %s      %s   %s %s\n", "type", "N", "alt", "N", if (hasn) "  k " else ""))
  invisible(NULL)
}
stata_nlogit_setup <- function(d, depvar, altvar, nestvar, case) {
  for (v in c(depvar, altvar, nestvar, case)) if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
  list(y = stata_mx_num(d[[depvar]]), alt = stata_mx_num(d[[altvar]]), nest = stata_mx_num(d[[nestvar]]), case = stata_mx_num(d[[case]]))
}
stata_nlogit_print_tree <- function(d, depvar, altvar, nestvar, case) {
  s <- stata_nlogit_setup(d, depvar, altvar, nestvar, case)
  alts <- sort(unique(s$alt)); nests <- sort(unique(s$nest))
  an <- stata_mx_names(alts, attr(d[[altvar]], "labels")); nn <- stata_mx_names(nests, attr(d[[nestvar]], "labels"))
  cat("\nTree structure specified for the nested logit model\n\n")
  cat(sprintf(" %-6s%5s      %-5s%4s %3s \n", nestvar, "N", altvar, "N", if (!is.na(depvar) && nzchar(depvar)) "k" else ""))
  cat(strrep("-", 29), "\n", sep = "")
  for (g in seq_along(nests)) {
    inn <- alts[vapply(alts, function(a) all(s$nest[s$alt == a] == nests[g]), TRUE)]
    for (i in seq_along(inn)) {
      a <- inn[i]
      cat(sprintf(" %-6s %3s %s %-5s %3d %3d\n", if (i == 1) nn[g] else "", if (i == 1) format(sum(s$nest == nests[g])) else "",
                  if (i == 1) "---" else "   +-", an[match(a, alts)], sum(s$alt == a), sum(s$alt == a & s$y == 1)))
    }
  }
  cat(strrep("-", 29), "\n", sep = "")
  cat(sprintf("%14s %5s %5d %3d\n", "Total", "", length(s$y), sum(s$y == 1)))
  cat("\nk = number of times alternative is chosen\nN = number of observations at each level\n\n")
}
# nlogit depvar indepvars || nestvar: nestvars [, base(group)] || altvar: [altvars], [noconstant] case(var)
stata_nlogit <- function(d, depvar, indep, nestvar, nestx, altvar, case, base = NULL, noconst = FALSE, technique = "bfgs",
                         cmdline = "", show = TRUE) {
  s <- stata_nlogit_setup(d, depvar, altvar, nestvar, case)
  for (v in c(indep, nestx)) if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
  X <- vapply(indep, function(v) stata_mx_num(d[[v]]), numeric(nrow(d)))
  X <- matrix(X, nrow(d), length(indep), dimnames = list(NULL, indep))
  Z <- vapply(nestx, function(v) stata_mx_num(d[[v]]), numeric(nrow(d)))
  Z <- matrix(Z, nrow(d), length(nestx), dimnames = list(NULL, nestx))
  ok <- !is.na(s$y) & !is.na(s$alt) & !is.na(s$nest) & !is.na(s$case) & stats::complete.cases(X) & stats::complete.cases(Z)
  rows <- which(ok)
  y <- s$y[ok]; alt <- s$alt[ok]; nest <- s$nest[ok]; cs <- s$case[ok]; X <- X[ok, , drop = FALSE]; Z <- Z[ok, , drop = FALSE]
  alts <- sort(unique(alt)); nests <- sort(unique(nest))
  G <- length(nests); A <- length(alts); K <- length(indep); L <- length(nestx)
  ai <- match(alt, alts); gi <- match(nest, nests)
  anest <- vapply(seq_len(A), function(a) gi[ai == a][1], 1L)
  cases <- unique(cs); ci <- match(cs, cases); C <- length(cases)
  if (any(tapply(y, ci, sum) != 1)) stop("each case must have exactly one chosen alternative (Stata: r(459))", call. = FALSE)
  an <- stata_mx_names(alts, attr(d[[altvar]], "labels")); nn <- stata_mx_names(nests, attr(d[[nestvar]], "labels"))
  ib <- if (is.null(base)) 1L else { m <- match(base, nn); if (is.na(m)) match(suppressWarnings(as.numeric(base)), nests) else m }
  if (is.na(ib)) stop("base(", base, ") is not a group of ", nestvar, " (Stata: r(198))", call. = FALSE)
  nonb <- setdiff(seq_len(G), ib)
  # parameters: beta (K), gamma (L per non-base group), ASC (A - 1 unless noconstant), tau (G)
  nasc <- if (noconst) 0L else A - 1L
  idx <- function(th) {
    o <- 0
    list(beta = th[seq_len(K)], gamma = matrix(th[K + seq_len(L * length(nonb))], L, length(nonb)),
         asc = if (nasc) th[K + L * length(nonb) + seq_len(nasc)] else numeric(0),
         tau = th[K + L * length(nonb) + nasc + seq_len(G)])
  }
  # case-level data: the row of each case's first line gives z
  first <- match(seq_len(C), ci)
  Zc <- Z[first, , drop = FALSE]
  dat <- list(X = X, Z = Z, ai = ai, gi = gi, ci = ci, C = C, G = G, K = K, L = L, nonb = nonb, nasc = nasc)
  ll <- function(th, tau1 = FALSE) sum(stata_nlogit_lp(th, dat, tau1)$lp[y == 1])
  start <- c(rep(0, K), rep(0, L * length(nonb)), rep(0, nasc), rep(1, G))
  fit <- stata_mx_max(ll, start)
  th <- fit$x
  V <- -solve(fit$H)
  # tau = 1: the conditional logit
  th1 <- c(rep(0, K), rep(0, L * length(nonb)), rep(0, nasc))
  ll1 <- function(t) ll(c(t, rep(1, G)), TRUE)
  fit1 <- stata_mx_max(ll1, th1)
  p <- idx(th)
  nm <- c(paste0("alt:", indep))
  full <- p$beta
  if (L > 0) for (k in seq_along(nonb)) for (l in seq_len(L)) { nm <- c(nm, paste0(nn[nonb[k]], ":", nestx[l])); full <- c(full, p$gamma[l, k]) }
  if (nasc) { nm <- c(nm, paste0(an[-1], ":_cons")); full <- c(full, p$asc) }
  nm <- c(nm, paste0("/", nestvar, ":", nn, "_tau")); full <- c(full, p$tau)
  names(full) <- nm
  wald_idx <- seq_len(K + L * length(nonb))
  chi2 <- drop(t(th[wald_idx]) %*% solve(V[wald_idx, wald_idx, drop = FALSE], th[wald_idx]))
  dfm <- length(wald_idx)
  pv <- stats::pchisq(chi2, dfm, lower.tail = FALSE)
  chi2c <- 2 * (fit$value - fit1$value)
  pc <- stats::pchisq(chi2c, G, lower.tail = FALSE)
  npar <- length(th)
  cnt <- as.vector(table(factor(ci, levels = seq_len(C))))
  if (show) {
    stata_nlogit_print_tree(d, depvar, altvar, nestvar, case)
    cat(sprintf("\nRUM-consistent nested logit regression         Number of obs      = %10s\n", format(length(y), big.mark = ",")))
    cat(sprintf("Case variable: %-30s Number of cases    = %10s\n\n", case, format(C, big.mark = ",")))
    cat(sprintf("Alternative variable: %-23s Alts per case: min = %10d\n%62s = %10.1f\n%62s = %10d\n\n", altvar, min(cnt), "avg", mean(cnt), "max", max(cnt)))
    cat(sprintf("%50sWald chi2(%d)    = %10.2f\n", "", dfm, chi2))
    cat(sprintf("Log likelihood = %-33s Prob > chi2     = %10.4f\n\n", format(fit$value, digits = 8), pv))
    q95 <- stats::qnorm(0.975)
    sd <- sqrt(diag(V))
    line <- function(l, b, se, z = TRUE) {
      if (z) {
        zz <- b / se
        cat(sprintf("%12s | %10s %10s %7.2f %7.3f %11s %11s\n", l, format(b, digits = 7), format(se, digits = 7), zz, 2 * stats::pnorm(-abs(zz)),
                    format(b - q95 * se, digits = 7), format(b + q95 * se, digits = 7)))
      } else {
        cat(sprintf("%12s | %10s %10s %7s %7s %11s %11s\n", l, format(b, digits = 7), format(se, digits = 7), "", "", format(b - q95 * se, digits = 7), format(b + q95 * se, digits = 7)))
      }
    }
    rule <- paste0(strrep("-", 78), "\n")
    cat(rule, sprintf("%12s | Coefficient  Std. err.      z    P>|z|     [95%% conf. interval]\n", depvar), "-------------+", strrep("-", 64), "\n", sep = "")
    cat(sprintf("%12s |\n", "alt"))
    for (i in seq_len(K)) line(indep[i], p$beta[i], sd[i])
    cat(rule)
    pos <- K
    if (L > 0) {
      cat(sprintf("%s equations\n", nestvar), rule, sep = "")
      for (g in seq_len(G)) {
        cat(sprintf("%12s |\n", nn[g]))
        for (l in seq_len(L)) {
          if (g == ib) cat(sprintf("%12s | %10s  (base)\n", nestx[l], "0"))
          else { pos <- pos + 1; line(nestx[l], th[pos], sd[pos]) }
        }
        if (g < G) cat("-------------+", strrep("-", 64), "\n", sep = "")
      }
      cat(rule)
    }
    if (nasc) {
      cat(sprintf("%s equations\n", altvar), rule, sep = "")
      for (a in seq_len(A)) {
        cat(sprintf("%12s |\n", an[a]))
        if (a == 1) cat(sprintf("%12s | %10s  (base)\n", "_cons", "0")) else { pos <- pos + 1; line("_cons", th[pos], sd[pos]) }
        if (a < A) cat("-------------+", strrep("-", 64), "\n", sep = "")
      }
      cat(rule)
    }
    cat("dissimilarity parameters\n", rule, sprintf("%12s |\n", paste0("/", nestvar)), sep = "")
    for (g in seq_len(G)) { pos <- pos + 1; line(paste0(nn[g], "_tau"), th[pos], sd[pos], z = FALSE) }
    cat(rule)
    cat(sprintf("LR test for IIA (tau=1): chi2(%d) = %-8.2f %24s Prob > chi2 = %6.4f\n", G, chi2c, "", pc))
  }
  # V on the full coefficient vector (the same order)
  E_ <- list(rank = npar, N = length(y), ic = NA_real_, k_eq = L * length(nonb) / max(L, 1) + 1 + nasc + G, converged = as.numeric(fit$converged), rc = 0,
             ll = fit$value, k_eq_model = 1 + (L > 0), df_m = dfm, chi2 = chi2, p = pv, ll_c = fit1$value, df_c = G, chi2_c = chi2c, p_c = pc,
             k_alt = A, alt_min = min(cnt), alt_avg = mean(cnt), alt_max = max(cnt), N_ic = C, N_case = C, const = as.numeric(!noconst),
             k_indvars = K, rum = 1, levels = 2, i_base = 1, i_base1 = ib, k_ind2vars = 0, k_ind2vars1 = L, k_alt1 = G, const1 = 0)
  E_$cmdline <- cmdline; E_$cmd <- "nlogit"; E_$marginsnotok <- "_ALL"; E_$predict <- "nlogit_p"; E_$estat_cmd <- "nlogit_estat"
  E_$marktype <- "casewise"; E_$ind2vars1 <- paste(nestx, collapse = " "); E_$alteqs1 <- paste(nn, collapse = " ")
  for (g in seq_len(G)) E_[[paste0("alt1_", g)]] <- nn[g]
  E_$altvar1 <- nestvar; E_$alteqs <- paste(an, collapse = " ")
  for (a in seq_len(A)) E_[[paste0("alt", a)]] <- an[a]
  E_$altvar <- altvar; E_$key_N_ic <- "cases"; E_$case <- case; E_$depvar <- depvar; E_$indvars <- paste(indep, collapse = " ")
  E_$chi2type <- "Wald"; E_$deriv_useminbound <- "off"; E_$opt <- "moptimize"; E_$vce <- "oim"; E_$title <- "RUM-consistent nested logit regression"
  E_$user <- "nlogit_lf"; E_$ml_method <- "d1"; E_$technique <- technique; E_$which <- "max"; E_$properties <- "b V"
  E_$b <- matrix(full, 1, dimnames = list("y1", nm)); E_$V <- `dimnames<-`(V, list(nm, nm))
  attr(E_, "stata_rows") <- rows
  attr(E_, "nlogit") <- list(th = th, indep = indep, nestx = nestx, nestvar = nestvar, altvar = altvar, case = case, base = ib,
                             noconst = noconst, alts = alts, nests = nests, depvar = depvar)
  E_
}
# the log probabilities of every alternative (lp), of its group (lpg) and the group's utility terms
stata_nlogit_lp <- function(th, dat, tau1 = FALSE) {
  with(dat, {
    p <- list(beta = th[seq_len(K)], gamma = matrix(th[K + seq_len(L * length(nonb))], L, length(nonb)),
              asc = if (nasc) th[K + L * length(nonb) + seq_len(nasc)] else numeric(0),
              tau = th[K + L * length(nonb) + nasc + seq_len(G)])
    tau <- if (tau1) rep(1, G) else p$tau
    V <- drop(X %*% p$beta) + (if (nasc) c(0, p$asc)[ai] else 0)
    ex <- exp(V / tau[gi])
    key <- ci * (G + 1) + gi
    S <- rowsum(ex, key)
    Ig <- log(S[match(key, as.numeric(rownames(S))), 1])
    first <- match(seq_len(C), ci)
    Zc <- Z[first, , drop = FALSE]
    Yg <- matrix(0, C, G)
    if (L > 0) for (k in seq_along(nonb)) Yg[, nonb[k]] <- drop(Zc %*% p$gamma[, k])
    IV <- matrix(-Inf, C, G)
    IV[cbind(ci, gi)] <- Ig
    top <- Yg + sweep(IV, 2, tau, `*`)
    top[!is.finite(top)] <- -Inf
    mt <- apply(top, 1, max)
    lPg <- top - mt - log(rowSums(exp(top - mt)))
    lPj <- V / tau[gi] - Ig
    list(lp = lPg[cbind(ci, gi)] + lPj, lpg = lPg[cbind(ci, gi)])
  })
}
# predict stub*: the probability of the group (stub1) and of the alternative (stub2) for the rows of d
stata_nlogit_pr <- function(d, E_) {
  m <- attr(E_, "nlogit")
  num <- function(v) stata_mx_num(d[[v]])
  X <- vapply(m$indep, num, numeric(nrow(d))); X <- matrix(X, nrow(d), length(m$indep))
  Z <- vapply(m$nestx, num, numeric(nrow(d))); Z <- matrix(Z, nrow(d), length(m$nestx))
  alt <- num(m$altvar); nest <- num(m$nestvar); cs <- num(m$case)
  ok <- !is.na(alt) & !is.na(nest) & !is.na(cs) & stats::complete.cases(X) & stats::complete.cases(Z)
  out <- matrix(NA_real_, nrow(d), 2)
  if (!any(ok)) return(out)
  cases <- unique(cs[ok]); ci <- match(cs[ok], cases)
  ai <- match(alt[ok], m$alts); gi <- match(nest[ok], m$nests)
  G <- length(m$nests); A <- length(m$alts); K <- length(m$indep); L <- length(m$nestx)
  nonb <- setdiff(seq_len(G), m$base)
  dat <- list(X = X[ok, , drop = FALSE], Z = Z[ok, , drop = FALSE], ai = ai, gi = gi, ci = ci, C = length(cases), G = G, K = K, L = L,
              nonb = nonb, nasc = if (m$noconst) 0L else A - 1L)
  lp <- stata_nlogit_lp(m$th, dat)
  out[ok, 1] <- exp(lp$lpg); out[ok, 2] <- exp(lp$lp)
  out
}
