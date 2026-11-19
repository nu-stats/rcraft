//! `pwmean`, pairwise comparisons of means with equal variances, as Stata 19
//! prints and leaves them (`plan/stata/pwmean.md`, the last piece of batch C).
//! Every group's mean, the pooled within-group variance on N - G degrees of
//! freedom, each pair `j vs i` (i < j), and the eight ways to adjust: none,
//! Bonferroni, Sidak, Scheffe, Tukey, SNK, Duncan and Dunnett. The tables are Stata's to the column;
//! e() holds the means and the contrasts with their matrices, r() the tables
//! and the letters of the means that cannot be told apart.

use super::*;

fn stop(out: &mut Out, msg: &str) {
    out.line(format!("stop({}, call. = FALSE)", r_text(msg)));
}

/// R source of `stata_pwmean` (the data in, the groups out), `stata_pwmean_out` (the
/// tables, e() and r() of one set of groups) and `stata_pwmean_replay` (a bare
/// `pwmean`, which shows the groups kept in e() again).
pub(super) const PWMEAN_HELPER: &str = r##"stata_dpmax <- function(cc, kk, nu, rho = 0.5) {
  # P(max |T_j| <= cc) for kk equicorrelated t variates (correlation rho) on nu df: the
  # integral of the product form over the chi scale and the common normal term
  inner <- function(s) vapply(s, function(si) {
    f <- function(z) stats::dnorm(z) * (stats::pnorm((cc * si + sqrt(rho) * z) / sqrt(1 - rho)) -
                                        stats::pnorm((-cc * si + sqrt(rho) * z) / sqrt(1 - rho)))^kk
    stats::integrate(f, -Inf, Inf, rel.tol = 1e-13, abs.tol = 0, subdivisions = 400L)$value
  }, 0)
  g <- function(s) exp((nu / 2) * log(nu) + (nu - 1) * log(s) - nu * s^2 / 2 - ((nu / 2 - 1) * log(2) + lgamma(nu / 2))) * inner(s)
  lo <- max(1e-8, 1 - 12 / sqrt(nu))
  hi <- 1 + 12 / sqrt(nu)
  stats::integrate(g, lo, 1, rel.tol = 1e-13, abs.tol = 0, subdivisions = 400L)$value +
    stats::integrate(g, 1, hi, rel.tol = 1e-13, abs.tol = 0, subdivisions = 400L)$value
}
stata_pwmean <- function(d, y, over, method = "noadjust", level = 95, tables = "ci", sort = FALSE,
                         show = TRUE, cmdline = "") {
  num <- function(u) { u <- unclass(u); attributes(u) <- NULL; as.numeric(u) }
  for (v in c(y, over)) if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
  for (v in c(y, over)) if (is.character(d[[v]])) stop(v, " is a string variable, type mismatch (Stata: r(109))", call. = FALSE)
  yv <- num(d[[y]])
  om <- matrix(vapply(over, function(v) num(d[[v]]), numeric(nrow(d))), nrow(d))
  ok <- !is.na(yv) & rowSums(is.na(om)) == 0
  if (!any(ok)) stop("no observations (Stata: r(2000))", call. = FALSE)
  yv <- yv[ok]
  om <- om[ok, , drop = FALSE]
  if (any(om < 0 | om != floor(om))) stop("over() variables may not contain negative or noninteger values (Stata: r(452))", call. = FALSE)
  key <- do.call(paste, c(as.data.frame(om), sep = "\r"))
  ord <- do.call(order, as.data.frame(om))
  first <- ord[!duplicated(key[ord])]
  cells <- om[first, , drop = FALSE]
  gk <- key[first]
  k <- length(gk)
  if (k < 2) stop("pwmean needs two or more groups (Stata: r(322))", call. = FALSE)
  gi <- match(key, gk)
  n <- tabulate(gi, k)
  m <- vapply(seq_len(k), function(j) mean(yv[gi == j]), 0)
  ssw <- sum(vapply(seq_len(k), function(j) sum((yv[gi == j] - m[j])^2), 0))
  dfr <- length(yv) - k
  # the labels of the cells
  comp <- function(j) vapply(seq_along(over), function(i) {
    u <- cells[j, i]
    vl <- attr(d[[over[i]]], "labels", exact = TRUE)
    lab <- if (!is.null(vl)) names(vl)[match(u, num(vl))] else NA
    if (length(lab) && !is.na(lab)) c(lab, "1") else c(stata_fmt(u, "%9.0g", trim = TRUE), "0")
  }, c("", ""))
  st <- list(y = y, over = over, cells = cells, cp = lapply(seq_len(k), comp), k = k, n = n, m = m,
             s2 = if (dfr > 0) ssw / dfr else 0, dfr = dfr)
  stata_pwmean_out(st, method, level, tables, sort, show, cmdline)
}
stata_pwmean_replay <- function(e, method = "noadjust", level = 95, tables = "ci", sort = FALSE,
                                show = TRUE, err = "") {
  if (is.null(e) || !identical(e$cmd, "pwmean") || is.null(attr(e, "pw"))) stop("last estimates not found (Stata: r(301))", call. = FALSE)
  if (nzchar(err)) stop(err, call. = FALSE)
  stata_pwmean_out(attr(e, "pw"), method, level, tables, sort, show, e$cmdline)
}
stata_pwmean_out <- function(st, method, level, tables, sort, show, cmdline) {
  y <- st$y; over <- st$over; cells <- st$cells; cp <- st$cp; k <- st$k; n <- st$n; m <- st$m
  s2 <- st$s2; dfr <- st$dfr
  over_label <- paste(over, collapse = "#")
  if (method == "dunnett" && "groups" %in% tables) {
    if (show) cat("\nPairwise comparisons of means with equal variances\n\nOver: ", paste(over, collapse = " "), "\n\n", sep = "")
    stop("only one of options dunnett or groups is allowed (Stata: r(198))", call. = FALSE)
  }
  # the comparisons: every pair, or each group against the first (Dunnett)
  asked <- method
  M <- if (method == "dunnett") k - 1 else k * (k - 1) / 2
  ignored <- ""
  if (M == 1 && method != "noadjust") {
    ignored <- method
    method <- "noadjust"
  }
  alpha <- (100 - level) / 100
  # the matrices follow level() only for the tables shown: contrasts with intervals (the default),
  # means with cimeans; otherwise they hold the 95% ones (Stata 19)
  alpha_vs <- if (any(tables %in% c("pv", "ci", "wide"))) alpha else 0.05
  alpha_m <- if ("means" %in% tables) alpha else 0.05
  title <- c(noadjust = "Unadjusted", bonferroni = "Bonferroni", sidak = "Sidak", scheffe = "Scheffe", tukey = "Tukey",
             snk = "SNK", duncan = "Duncan", dunnett = "Dunnett")[[method]]
  if (method == "dunnett") {
    pi_ <- rep(1L, k - 1)
    pj_ <- 2:k
  } else {
    pi_ <- unlist(lapply(seq_len(k - 1), function(i) rep(i, k - i)))
    pj_ <- unlist(lapply(seq_len(k - 1), function(i) seq.int(i + 1, k)))
  }
  nc <- length(pi_)
  # a range method (Tukey, SNK, Duncan, Dunnett) has no value on 1 residual degree of freedom,
  # and nothing is known on none (Stata prints missing)
  range_m <- method %in% c("tukey", "snk", "duncan", "dunnett")
  no_se <- dfr < 1
  no_adj <- no_se || (range_m && dfr < 2)
  # the number of means a pair spans in the sorted means (SNK, Duncan)
  rk <- rank(m, ties.method = "first")
  span <- abs(rk[pj_] - rk[pi_]) + 1
  # a studentized range quantile: R's qtukey, then the root of ptukey near it
  qtk <- function(p, kk) {
    q0 <- stats::qtukey(p, kk, dfr)
    f <- function(q) stats::ptukey(q, kk, dfr) - p
    r <- tryCatch(stats::uniroot(f, c(q0 * 0.999, q0 * 1.001), tol = 1e-14), error = function(e) NULL)
    if (is.null(r)) q0 else r$root
  }
  dunnett_crit <- function(p) {
    lo <- stats::qt(1 - (1 - p) / 2, dfr) * 0.99
    hi <- stats::qt(1 - (1 - p) / (2 * (k - 1)), dfr) * 1.01
    stats::uniroot(function(cc) stata_dpmax(cc, k - 1, dfr) - p, c(lo, hi), tol = 1e-12)$root
  }
  crit <- if (no_adj) rep(NA_real_, nc) else switch(method,
    noadjust = rep(stats::qt(alpha_vs / 2, dfr, lower.tail = FALSE), nc),
    bonferroni = rep(stats::qt(alpha_vs / (2 * M), dfr, lower.tail = FALSE), nc),
    sidak = rep(stats::qt(-expm1(log1p(-alpha_vs) / M) / 2, dfr, lower.tail = FALSE), nc),
    scheffe = rep(sqrt((k - 1) * stats::qf(alpha_vs, k - 1, dfr, lower.tail = FALSE)), nc),
    tukey = rep(qtk(1 - alpha_vs, k) / sqrt(2), nc),
    snk = vapply(span, function(r) qtk(1 - alpha_vs, r) / sqrt(2), 0),
    duncan = vapply(span, function(r) qtk((1 - alpha_vs)^(r - 1), r) / sqrt(2), 0),
    dunnett = rep(dunnett_crit(1 - alpha_vs), nc))
  adj <- function(t) {
    if (no_adj) return(rep(NA_real_, length(t)))
    raw <- 2 * stats::pt(abs(t), dfr, lower.tail = FALSE)
    tk <- function(r) vapply(seq_along(t), function(i) stats::ptukey(abs(t[i]) * sqrt(2), r[i], dfr, lower.tail = FALSE), 0)
    switch(method,
      noadjust = raw,
      bonferroni = pmin(raw * M, 1),
      sidak = pmin(-expm1(M * log1p(-raw)), 1),
      scheffe = stats::pf(t^2 / (k - 1), k - 1, dfr, lower.tail = FALSE),
      tukey = pmin(pmax(tk(rep(k, length(t))), 0), 1),
      snk = pmin(pmax(tk(span), 0), 1),
      duncan = pmin(pmax(-expm1(log1p(-pmin(pmax(tk(span), 0), 1)) / (span - 1)), 0), 1),
      dunnett = pmin(pmax(vapply(abs(t), function(a) 1 - stata_dpmax(a, k - 1, dfr), 0), 0), 1))
  }
  dif <- m[pj_] - m[pi_]
  se <- if (no_se) rep(NA_real_, nc) else sqrt(s2 * (1 / n[pi_] + 1 / n[pj_]))
  tt <- dif / se
  pv <- adj(tt)
  # the means that cannot be told apart (not for Dunnett): runs of the sorted means, none of
  # whose pairs differ, of two or more; a letter for each run, the lowest run first
  # (plan/stata/pwmean.md)
  nl <- 0
  gl <- rep("", k)
  if (method != "dunnett") {
    P <- matrix(1, k, k)
    P[cbind(pi_, pj_)] <- pv
    P[cbind(pj_, pi_)] <- pv
    mo <- order(m)
    # the letters follow level() only when the table of them is shown, like the matrices
    alpha_g <- if ("groups" %in% tables) alpha else 0.05
    sig <- (P < alpha_g)[mo, mo, drop = FALSE]
    sig[is.na(sig)] <- FALSE
    ends <- integer(k)
    for (s in seq_len(k)) {
      e <- s
      while (e < k && !any(sig[s:(e + 1), s:(e + 1)])) e <- e + 1
      ends[s] <- e
    }
    runs <- list()
    last <- 0
    for (s in seq_len(k)) if (ends[s] > last && ends[s] > s) {
      runs[[length(runs) + 1]] <- mo[s:ends[s]]
      last <- ends[s]
    }
    if (k <= 24) {
      # groups of unequal size make the sets that cannot be told apart more than runs of
      # neighbours (Duncan: B held means 1, 5, 6 and 7): every maximal set with no significant
      # pair of two or more, the sets listed by their lowest means first
      ns <- !sig
      diag(ns) <- FALSE
      cl <- list()
      bk <- function(R, Pc, X) {
        if (!length(Pc) && !length(X)) { cl[[length(cl) + 1]] <<- sort(R); return(invisible()) }
        for (v in Pc) {
          nb <- which(ns[v, ])
          bk(c(R, v), intersect(Pc, nb), intersect(X, nb))
          Pc <- setdiff(Pc, v)
          X <- c(X, v)
        }
      }
      bk(integer(), seq_len(k), integer())
      cl <- cl[vapply(cl, length, 0L) >= 2]
      if (length(cl)) {
        keym <- matrix(vapply(cl, function(s) c(s, rep(Inf, k - length(s))), numeric(k)), nrow = k)
        cl <- cl[do.call(order, as.data.frame(t(keym)))]
      }
      runs <- lapply(cl, function(s) mo[s])
    }
    if (length(runs) > 26) stop("RCraft does not letter more than 26 groups of means", call. = FALSE)
    nl <- length(runs)
    gl <- vapply(seq_len(k), function(g) if (nl == 0) "" else paste(vapply(seq_len(nl), function(r) if (g %in% runs[[r]]) LETTERS[r] else " ", ""), collapse = ""), "")
  }
  txt <- function(j) cp[[j]][1, ]
  lab_any <- function(j) any(cp[[j]][2, ] == "1")
  one <- length(over) == 1
  cell <- function(j, sep) if (one) txt(j) else paste(txt(j), collapse = sep)
  mean_lab <- vapply(seq_len(k), function(j) cell(j, if (lab_any(j)) "#" else " "), "")
  vs_lab <- vapply(seq_along(pi_), function(c) {
    sep <- if (lab_any(pj_[c])) "#" else " "
    if (one) paste(txt(pj_[c]), "vs", txt(pi_[c]))
    else paste0("(", cell(pj_[c], sep), ") vs (", cell(pi_[c], sep), ")")
  }, "")
  lv <- function(j) vapply(seq_along(over), function(i) stata_fmt(cells[j, i], "%9.0g", trim = TRUE), "")
  # Stata flags the base term of each variable with bn: its lowest level in e(b), and in the
  # contrasts every term equal to the one the first contrast has
  base <- apply(cells, 2, min)
  cn <- vapply(seq_len(k), function(j) paste(paste0(lv(j), ifelse(cells[j, ] == base, "bn", ""), ".", over), collapse = "#"), "")
  term <- function(c) paste0(lv(pj_[c]), "vs", lv(pi_[c]))
  t1 <- term(1)
  vs <- vapply(seq_along(pi_), function(c) paste(paste0(term(c), ifelse(term(c) == t1, "bn", ""), ".", over), collapse = "#"), "")
  # the matrices
  rows9 <- c("b", "se", "t", "pvalue", "ll", "ul", "df", "crit", "eform")
  sev <- sqrt(s2 / n)
  semean <- if (no_se) rep(NA_real_, k) else sev
  tmean <- m / semean
  qu <- if (no_se) NA_real_ else stats::qt(alpha_m / 2, dfr, lower.tail = FALSE)
  tab <- rbind(b = m, se = semean, t = tmean, pvalue = 2 * stats::pt(abs(tmean), dfr, lower.tail = FALSE),
               ll = m - qu * semean, ul = m + qu * semean, df = dfr, crit = qu, eform = 0)
  colnames(tab) <- cn
  tabvs <- rbind(b = dif, se = se, t = tt, pvalue = pv, ll = dif - crit * se, ul = dif + crit * se, df = dfr, crit = crit, eform = 0)
  colnames(tabvs) <- vs
  A <- matrix(0, nc, k)
  A[cbind(seq_len(nc), pj_)] <- 1
  A[cbind(seq_len(nc), pi_)] <- -1
  Vm <- diag(s2 / n, k)
  Vvs <- A %*% Vm %*% t(A)
  dimnames(Vm) <- list(cn, cn)
  dimnames(Vvs) <- list(vs, vs)
  e <- list(balanced = as.numeric(all(n == n[1])), df_r = dfr, cmdline = cmdline, cmd = "pwmean",
            title = "Pairwise comparisons of means with equal variances", depvar = y,
            over = paste(over, collapse = " "), properties = "b V",
            b = matrix(m, 1, dimnames = list("y1", cn)), V = Vm,
            error = matrix(0, 1, k, dimnames = list("r1", paste0("c", seq_len(k)))),
            b_vs = matrix(dif, 1, dimnames = list("r1", vs)), V_vs = Vvs,
            error_vs = matrix(0, 1, nc, dimnames = list("r1", paste0("c", seq_len(nc)))))
  attr(e, "stata_sample") <- FALSE
  attr(e, "pw") <- st
  r <- list(level = level, mctitle_vs = title, mcmethod_vs = method)
  for (g in rev(seq_len(k))) if (nl > 0) r[[paste0("groups", g)]] <- gl[g]
  r$table_vs <- tabvs
  r$table <- tab
  if (asked != "dunnett") r$k_groups <- matrix(nl, 1, 1, dimnames = list("r1", "c1"))
  out <- list(e = e, r = r)
  if (!show) return(out)
  # the tables
  g9 <- function(v) if (is.na(v)) "." else stata_fmt(v, "%9.0g", trim = TRUE)
  rj <- function(s, w) formatC(s, width = w)
  centred <- function(s, at) paste0(strrep(" ", max(at - ceiling(nchar(s) / 2), 0)), s)
  cit <- paste0("[", stata_fmt(level, "%9.0g", trim = TRUE), "% conf. interval]")
  cat("\nPairwise comparisons of means with equal variances\n\nOver: ", paste(over, collapse = " "), "\n\n", sep = "")
  hasc <- any(tables %in% c("groups", "pv", "ci", "wide"))
  if (ignored != "" && hasc) cat("note: option ", ignored, " ignored since there is only one comparison\n", sep = "")
  gap <- FALSE
  sep_table <- function() { if (gap) cat("\n"); gap <<- TRUE }
  # an adjusted method says how many comparisons it adjusts for before the letters and before
  # the first of the tables of contrasts (several in a row share one), not before the means
  comparisons <- function() {
    wc <- max(13, nchar(over_label) + 1)
    rule <- strrep("-", wc + 14)
    cat(rule, "\n", strrep(" ", wc), "|    Number of\n", strrep(" ", wc), "|  comparisons\n", strrep("-", wc), "+", strrep("-", 13), "\n", sep = "")
    cat(rj(over_label, wc - 1), " |", sprintf("%13d", as.integer(M)), "\n", rule, "\n\n", sep = "")
  }
  prev_contrast <- FALSE
  show_table <- function(kind) {
    sep_table()
    if (method != "noadjust" && (kind == "groups" || (kind != "means" && !prev_contrast))) comparisons()
    prev_contrast <<- kind %in% c("pv", "ci", "wide")
    mean_rows <- kind %in% c("means", "groups")
    labs <- if (mean_rows) mean_lab else vs_lab
    w <- max(13, nchar(over_label) + 1, max(nchar(labs)) + 2)
    gcol <- max(10, nchar(title) + 3)
    right <- switch(kind, ci = 48, means = 48, pv = 39, wide = 64, groups = 22 + gcol)
    ttl <- if (kind == "means") "Unadjusted" else title
    cat(strrep("-", w + 1 + right), "\n", sep = "")
    if (kind == "wide") {
      first <- centred(ttl, 33)
      cat(strrep(" ", w), "|", first, strrep(" ", max(54 - (nchar(first) + ceiling(nchar(ttl) / 2)), 0)), ttl, "\n", sep = "")
    } else if (kind == "groups") {
      cat(strrep(" ", w), "|", rj(ttl, right), "\n", sep = "")
    } else {
      cat(strrep(" ", w), "|", centred(ttl, if (kind == "pv") 33 else 38), "\n", sep = "")
    }
    dep <- rj(stata_tab_abbrev(y, w - 1), w - 1)
    col <- if (mean_rows) "Mean" else "Contrast"
    cat(dep, " | ", rj(col, 10), switch(kind,
      ci = paste0("   Std. err.", rj(cit, 25)), means = paste0("   Std. err.", rj(cit, 25)),
      pv = "   Std. err.      t    P>|t|", wide = paste0("   Std. err.      t    P>|t|", rj(cit, 25)),
      groups = paste0("   Std. err.", rj("groups", gcol - 1))), "\n", sep = "")
    cat(strrep("-", w), "+", strrep("-", right), "\n", rj(over_label, w - 1), " |\n", sep = "")
    idx <- seq_along(labs)
    if (sort) idx <- order(if (mean_rows) m else dif)
    for (i in idx) {
      lab <- paste0(rj(labs[i], w - 2), "  |")
      if (mean_rows) {
        cells9 <- paste0(rj(g9(m[i]), 11), rj(g9(if (kind == "groups") sev[i] else semean[i]), 11))
        tail <- if (kind == "groups") rj(if (nl == 0) "" else gl[i], gcol) else paste0(rj(g9(tab["ll", i]), 14), rj(g9(tab["ul", i]), 12))
      } else {
        cells9 <- paste0(rj(g9(dif[i]), 11), rj(g9(se[i]), 11))
        tcol <- paste0(rj(if (is.na(tt[i])) "." else sprintf("%.2f", tt[i]), 9), rj(if (is.na(pv[i])) "." else sprintf("%.3f", pv[i]), 8))
        lims <- paste0(rj(g9(tabvs["ll", i]), 13), rj(g9(tabvs["ul", i]), 12))
        tail <- switch(kind, ci = paste0(rj(g9(tabvs["ll", i]), 14), rj(g9(tabvs["ul", i]), 12)), pv = tcol, wide = paste0(tcol, lims))
      }
      cat(lab, cells9, tail, "\n", sep = "")
    }
    cat(strrep("-", w + 1 + right), "\n", sep = "")
    if (kind == "groups") {
      width <- w + 1 + right
      words <- strsplit(paste0("Note: Means sharing a letter in the group label are not significantly different at the ",
                               stata_fmt(100 - level, "%9.0g", trim = TRUE), "% level."), " ", fixed = TRUE)[[1]]
      cur <- words[1]
      for (wd in words[-1]) {
        if (nchar(cur) + 1 + nchar(wd) <= width) cur <- paste(cur, wd)
        else { cat(cur, "\n", sep = ""); cur <- paste0("      ", wd) }
      }
      cat(cur, "\n", sep = "")
    }
  }
  for (kind in c("groups", "means", "pv", "ci", "wide")) if (kind %in% tables) show_table(kind)
  out
}"##;

/// The names `pwmean` keeps in r(); the letters `r(groups1)`, ... are left out
/// because their order can differ from Stata's (`plan/stata/pwmean.md`).
const R_PWMEAN: &[&str] = &[
    "level",
    "mctitle_vs",
    "mcmethod_vs",
    "table_vs",
    "table",
    "k_groups",
];

/// How Stata spells the method of `mcompare()`, and the least that may be typed.
const METHODS: &[(&str, usize)] = &[
    ("noadjust", 5),
    ("bonferroni", 3),
    ("sidak", 3),
    ("scheffe", 3),
    ("tukey", 3),
    ("snk", 3),
    ("duncan", 4),
    ("dunnett", 5),
];

/// The options `pwmean` knows, with the shortest abbreviation of each.
const OPTIONS: &[(&str, usize)] = &[
    ("over", 4),
    ("mcompare", 5),
    ("effects", 3),
    ("cimeans", 3),
    ("pveffects", 2),
    ("cieffects", 3),
    ("ci", 2),
    ("groups", 5),
    ("sort", 4),
    ("level", 1),
];

/// `i.g`, `ib2.g` and `c.g` in over() name the variable `g`, as in Stata.
fn factor_base(tok: &str) -> &str {
    if let Some(r) = tok.strip_prefix("i.").or_else(|| tok.strip_prefix("c.")) {
        return r;
    }
    if let Some(r) = tok.strip_prefix("ib") {
        let digits = r.chars().take_while(char::is_ascii_digit).count();
        if digits > 0 {
            if let Some(rest) = r.get(digits..).and_then(|t| t.strip_prefix('.')) {
                return rest;
            }
        }
    }
    tok
}

impl Session {
    /// `pwmean varname [if] [in], over(varlist) [mcompare(method) effects
    /// cimeans pveffects cieffects ci groups sort level(#)]`, and a bare
    /// `pwmean [, options]` that shows the groups of the last one again.
    ///
    /// Observed in Stata 19: no over() r(198), weights r(101), more than one
    /// variable r(103), a factor variable r(101), a string r(109), a name the
    /// data lack r(111), an over() value negative or not whole r(452), no rows
    /// r(2000), one group r(322), an unknown option or method r(198), an
    /// option twice r(198) (a second level() only when it says 95), level()
    /// outside 10 to 99.99 r(198), `by:` r(190); a bare `pwmean` after another
    /// estimation r(301), with an if or in r(102), with over() r(198).
    pub(super) fn pwmean(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "pwmean")?;
        if let Some(o) = opts.other(OPTIONS) {
            return Err(no(format!(
                "pwmean with the option {o} is not translated yet"
            )));
        }
        // an option given twice is Stata's r(198); ci is cieffects
        let same: [&[(&str, usize)]; 8] = [
            &[("over", 4)],
            &[("mcompare", 5)],
            &[("effects", 3)],
            &[("cimeans", 3)],
            &[("pveffects", 2)],
            &[("cieffects", 3), ("ci", 2)],
            &[("groups", 5)],
            &[("sort", 4)],
        ];
        let twice = same.iter().any(|grp| {
            c.options
                .iter()
                .filter(|o| grp.iter().any(|(f, m)| o.is(f, *m)))
                .count()
                > 1
        });
        if twice {
            stop(out, "duplicate option not allowed (Stata: r(198))");
            return Ok(());
        }
        let vars: Vec<&str> = c.parts.main.split_whitespace().collect();
        let bare = vars.is_empty();
        if c.parts.weight.is_some() {
            if bare {
                // after the check that there is something to show again
                return self.pwmean_replay(c, opts, out, "weights not allowed (Stata: r(101))");
            }
            stop(out, "weights not allowed (Stata: r(101))");
            return Ok(());
        }
        if bare {
            let err = if c.parts.if_exp.is_some() || c.parts.in_range.is_some() {
                "too few variables specified (Stata: r(102))"
            } else if opts.has("over", 4) {
                "option over() not allowed (Stata: r(198))"
            } else {
                ""
            };
            return self.pwmean_replay(c, opts, out, err);
        }
        if vars.iter().any(|v| v.contains('.') || v.contains('#')) {
            stop(
                out,
                "factor variables and time-series operators not allowed (Stata: r(101))",
            );
            return Ok(());
        }
        if let Some(v) = vars.iter().find(|v| !is_name(v)) {
            return Err(no(format!(
                "pwmean {v}: only plain variable names are translated"
            )));
        }
        let Some(over) = opts.arg("over", 4) else {
            stop(out, "option over() required (Stata: r(198))");
            return Ok(());
        };
        if over.split_whitespace().next().is_none() {
            stop(out, "option over() incorrectly specified (Stata: r(198))");
            return Ok(());
        }
        let mut names: Vec<String> = Vec::new();
        for tok in over.split_whitespace() {
            if tok.contains('#') {
                stop(
                    out,
                    "interactions may not be specified in the over() option (Stata: r(198))",
                );
                return Ok(());
            }
            let name = factor_base(tok);
            if !is_name(name) {
                return Err(no(format!(
                    "pwmean over({tok}): only plain variable names are translated"
                )));
            }
            // a variable named twice counts once
            if !names.iter().any(|v| v == name) {
                names.push(name.to_string());
            }
        }
        let over = names;
        if let [_, _, ..] = vars.as_slice() {
            stop(out, "too many variables specified (Stata: r(103))");
            return Ok(());
        }
        let Some((method, level, tables, sort)) = self.pwmean_options(c, opts, out)? else {
            return Ok(());
        };
        let rows = self.rows_for(c, out)?;
        for h in [
            "stata_pwmean",
            "stata_fmt",
            "stata_returned",
            "stata_tables",
            "stata_names",
        ] {
            self.helpers.insert(h);
        }
        let Some(y) = vars.first() else {
            return Ok(());
        };
        let q = |v: &[String]| v.iter().map(|s| r_text(s)).collect::<Vec<_>>().join(", ");
        out.line(format!(
            "stata_pw <- stata_pwmean({rows}, {}, c({}), method = {}, level = {}, tables = c({}), sort = {}{}, cmdline = {})",
            r_text(y),
            q(&over),
            r_text(&method),
            r_number(level),
            tables.iter().map(|t| r_text(t)).collect::<Vec<_>>().join(", "),
            if sort { "TRUE" } else { "FALSE" },
            if self.quiet(c) { ", show = FALSE" } else { "" },
            r_text(format!("{} {}", c.written, super::returned::after_name(c).trim()).trim())
        ));
        out.line("stata_e <- stata_pw$e");
        out.line("stata_r <- stata_pw$r");
        out.line("rm(stata_pw)");
        // e() holds means and contrasts, but not a model that predict, test or margins read
        out.line("if (exists(\"stata_b\")) rm(stata_b)");
        self.estimates = None;
        self.estimates_lost =
            Some("pwmean's results are not a model RCraft's predict, test or margins read".into());
        self.e_new = true;
        for t in ["cmdline", "cmd", "title", "depvar", "over", "properties"] {
            self.e_text.insert(t.to_string());
        }
        self.e_ret = super::returned::Returned::Kept {
            cmd: "pwmean".into(),
            model: String::new(),
            names: &[],
        };
        self.r_new = Some((R_PWMEAN, "pwmean".into()));
        Ok(())
    }

    /// The method, level, tables and sort of the options. `None` when Stata
    /// stops on one of them (the stop is written).
    #[allow(clippy::type_complexity)]
    fn pwmean_options(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<Option<(String, f64, Vec<&'static str>, bool)>, Untranslated> {
        let method = match opts.arg("mcompare", 5) {
            None => "noadjust".to_string(),
            Some(m) => {
                let m = m.trim().to_string();
                if m.is_empty() {
                    // an empty mcompare() is no adjustment
                    "noadjust".to_string()
                } else {
                    match METHODS
                        .iter()
                        .find(|(full, min)| m.len() >= *min && full.starts_with(m.as_str()))
                    {
                        Some((full, _)) => (*full).to_string(),
                        None => {
                            stop(
                                out,
                                &format!("invalid mcompare() option; method '{m}' not allowed (Stata: r(198))"),
                            );
                            return Ok(None);
                        }
                    }
                }
            }
        };
        // the first level() counts; each later one must say 95 (Stata's r(198) otherwise)
        let levels: Vec<String> = c
            .options
            .iter()
            .filter(|o| o.is("level", 1))
            .map(Opt::arg)
            .collect();
        let mut level = 95.0;
        for (i, l) in levels.iter().enumerate() {
            let v: f64 = l
                .trim()
                .parse()
                .map_err(|_| no(format!("level({l}) needs a number")))?;
            if i > 0 {
                if (v - 95.0).abs() > f64::EPSILON {
                    stop(out, "duplicate level() option not allowed (Stata: r(198))");
                    return Ok(None);
                }
                continue;
            }
            if !(10.0..=99.99).contains(&v) {
                stop(
                    out,
                    "level() must be between 10 and 99.99 inclusive (Stata: r(198))",
                );
                return Ok(None);
            }
            level = v;
        }
        let groups = opts.has("groups", 5);
        let cimeans = opts.has("cimeans", 3);
        let pv = opts.has("pveffects", 2);
        let ci = opts.has("cieffects", 3) || opts.has("ci", 2);
        let wide = opts.has("effects", 3);
        let mut tables: Vec<&'static str> = Vec::new();
        for (on, name) in [
            (groups, "groups"),
            (cimeans, "means"),
            (pv, "pv"),
            (ci, "ci"),
            (wide, "wide"),
        ] {
            if on {
                tables.push(name);
            }
        }
        if tables.is_empty() {
            tables.push("ci");
        }
        if groups {
            out.caveat(
                "pwmean groups: when the means form overlapping runs, the order of the letters can differ from Stata's; r(groups#) is not kept"
                    .to_string(),
            );
        }
        Ok(Some((method, level, tables, opts.has("sort", 4))))
    }

    /// A bare `pwmean`: the groups kept in e() again, with new options. `err`
    /// is what Stata says once it has found them (empty when it shows them).
    fn pwmean_replay(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
        err: &str,
    ) -> Result<(), Untranslated> {
        let Some((method, level, tables, sort)) = self.pwmean_options(c, opts, out)? else {
            return Ok(());
        };
        self.helpers.insert("stata_pwmean");
        for h in ["stata_fmt", "stata_returned", "stata_tables", "stata_names"] {
            self.helpers.insert(h);
        }
        out.line(format!(
            "stata_pw <- stata_pwmean_replay(if (exists(\"stata_e\")) stata_e else NULL, method = {}, level = {}, tables = c({}), sort = {}{}, err = {})",
            r_text(&method),
            r_number(level),
            tables.iter().map(|t| r_text(t)).collect::<Vec<_>>().join(", "),
            if sort { "TRUE" } else { "FALSE" },
            if self.quiet(c) { ", show = FALSE" } else { "" },
            r_text(err)
        ));
        out.line("stata_r <- stata_pw$r");
        out.line("rm(stata_pw)");
        self.r_new = Some((R_PWMEAN, "pwmean".into()));
        Ok(())
    }
}
