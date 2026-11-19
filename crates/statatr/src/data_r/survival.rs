//! Batch K of the StatCraft port: survival analysis. `stdescribe`, `stsum`,
//! `sts list | generate | test`, `stptime` and `stir` on the data `stset`
//! made (`_st`, `_d`, `_t`, `_t0`), and `ltable` on a time and a failure
//! variable. `streg` and `stcurve` are in `streg.rs`. What Stata 19 does is
//! in `plan/stata/survival.md`.

use super::st::StSet;
use super::*;

/// The R side of the descriptive survival commands.
pub(super) const SURVIVAL_HELPER: &str = r##"stata_sv_num <- function(v) {
  v <- unclass(v)
  attributes(v) <- NULL
  as.numeric(v)
}
stata_sv_head <- function(info, id = TRUE) {
  cat("\n        Failure _d: ", info$fail, "\n  Analysis time _t: ", info$time, "\n", sep = "")
  if (id && !is.null(info$id)) cat("       ID variable: ", info$id, "\n", sep = "")
  cat("\n")
}
stata_sv_g <- function(v, w = 9) sprintf("%*s", w, ifelse(is.na(v), ".", stata_fmt(v, "%9.0g", trim = TRUE)))
stata_sv_f <- function(v, w, dig = 4) sprintf("%*s", w, ifelse(is.na(v), ".", sprintf("%.*f", dig, v)))
stata_sv_rate <- function(v, w, fmt = "%9.0g") sprintf("%*s", w, ifelse(is.na(v), ".", stata_fmt(v, fmt, trim = TRUE)))
stata_sv_comma <- function(v, w = 10) sprintf("%*s", w, ifelse(is.na(v), ".", formatC(v, digits = 7, format = "g", big.mark = ",")))
stata_sv_median <- function(v) if (!length(v)) NA_real_ else stats::median(v)
# the usable rows, as plain vectors
stata_sv_rows <- function(d) list(t = stata_sv_num(d[["_t"]]), d = stata_sv_num(d[["_d"]]), t0 = stata_sv_num(d[["_t0"]]))
stata_sv_groups <- function(d, by) {
  if (is.null(by)) return(list(list(label = NULL, idx = seq_len(nrow(d)))))
  v <- d[[by]]
  key <- if (is.character(v) || is.factor(v)) as.character(v) else stata_sv_num(v)
  ok <- !is.na(key)
  lv <- sort(unique(key[ok]))
  lapply(lv, function(l) list(label = if (is.numeric(l)) format(l, digits = 8) else l, idx = which(ok & key == l)))
}
stata_sv_needs_zero_entry <- function(r, what) {
  if (any(r$t0 > 0)) stop(what, " with late entry (enter() or several records per subject) is not translated yet", call. = FALSE)
}
# the Kaplan-Meier table: one row per distinct time
stata_sv_km <- function(t, d, level = 95, t0 = NULL) {
  ut <- sort(unique(t))
  m <- length(ut)
  ix <- match(t, ut)
  nfail <- tabulate(ix[d != 0], m)
  nlost <- tabulate(ix[d == 0], m)
  n <- if (is.null(t0) || !any(t0 > 0)) rev(cumsum(rev(nfail + nlost))) else vapply(ut, function(u) sum(t >= u & t0 < u), 0)
  S <- cumprod(1 - nfail / n)
  gw <- cumsum(ifelse(nfail > 0, nfail / (n * (n - nfail)), 0))
  na <- cumsum(nfail / n)
  nav <- cumsum(nfail / n^2)
  z <- stats::qnorm(1 - (100 - level) / 200)
  edge <- !is.finite(S) | S <= 0 | S >= 1
  se <- ifelse(edge, NA_real_, S * sqrt(gw))
  sell <- ifelse(edge, NA_real_, sqrt(gw) / abs(log(S)))
  lb <- ifelse(edge, NA_real_, S^exp(z * sell))
  ub <- ifelse(edge, NA_real_, S^exp(-z * sell))
  nase <- sqrt(nav)
  nalb <- ifelse(na > 0, na * exp(-z * nase / na), NA_real_)
  naub <- ifelse(na > 0, na * exp(z * nase / na), NA_real_)
  data.frame(time = ut, n = n, fail = nfail, lost = nlost, S = S, se = se, lb = lb, ub = ub,
             na = na, nase = nase, nalb = nalb, naub = naub, h = ifelse(nfail > 0, nfail / n, NA_real_))
}
# the p-th survival time (p as a fraction of failures): the first time S falls to 1 - p or below
stata_sv_pct <- function(km, p) {
  hit <- which(km$S <= 1 - p + 1e-10)
  if (length(hit)) km$time[hit[1]] else NA_real_
}
stata_stdescribe <- function(d, info, id = NULL, show = TRUE) {
  r <- stata_sv_rows(d)
  n <- length(r$t)
  sid <- if (is.null(id)) seq_len(n) else {
    v <- d[[id]]
    if (is.character(v) || is.factor(v)) as.character(v) else stata_sv_num(v)
  }
  subj <- unique(sid)
  ns <- length(subj)
  rec <- as.numeric(table(match(sid, subj)))
  per <- function(f) vapply(seq_len(ns), function(i) f(which(match(sid, subj) == i)), 0)
  ix <- match(sid, subj)
  idx <- split(seq_len(n), ix)
  entry <- vapply(idx, function(k) { o <- k[order(r$t[k])]; r$t0[o[1]] }, 0)
  exit <- vapply(idx, function(k) max(r$t[k]), 0)
  risk <- vapply(idx, function(k) sum(r$t[k] - r$t0[k]), 0)
  fail <- vapply(idx, function(k) sum(r$d[k] != 0), 0)
  gap <- vapply(idx, function(k) {
    o <- k[order(r$t[k])]
    if (length(o) < 2) return(0)
    sum(pmax(0, r$t0[o][-1] - r$t[o][-length(o)]))
  }, 0)
  ng <- sum(gap > 0)
  st <- function(v) c(mean = mean(v), min = min(v), med = stata_sv_median(v), max = max(v))
  sr <- st(rec); se <- st(entry); sx <- st(exit); sk <- st(risk); sf <- st(fail)
  sg <- if (ng) st(gap[gap > 0]) else c(mean = NA_real_, min = NA_real_, med = NA_real_, max = NA_real_)
  if (show) {
    stata_sv_head(info)
    cat("                                   |-------------- Per subject --------------|\n")
    cat("Category                   Total        Mean         Min     Median        Max\n")
    cat(strrep("-", 78), "\n", sep = "")
    g <- function(v) stata_sv_g(v, 11)
    row <- function(lab, tot, s) cat(sprintf("%-22s", lab), stata_sv_g(tot, 7), g(s[["mean"]]), g(s[["min"]]), g(s[["med"]]), g(s[["max"]]), "\n", sep = "")
    cat(sprintf("%-22s%8s   \n", "Number of subjects", stata_sv_g(ns, 3)))
    row("Number of records", n, sr)
    cat("\n")
    cat(sprintf("%-22s%8s%12s%12s%11s%11s\n", "Entry time (first)", "", g(se[["mean"]]), g(se[["min"]]), g(se[["med"]]), g(se[["max"]])))
    cat(sprintf("%-22s%8s%12s%12s%11s%11s\n", "Exit time (final)", "", g(sx[["mean"]]), g(sx[["min"]]), g(sx[["med"]]), g(sx[["max"]])))
    cat("\n")
    cat(sprintf("%-22s%8s   \n", "Subjects with gap", stata_sv_g(ng, 3)))
    if (ng) row("Time on gap", sum(gap), sg) else cat(sprintf("%-22s%8s   \n", "Time on gap", stata_sv_g(0, 3)))
    row("Time at risk", sum(risk), sk)
    cat("\n")
    row("Failures", sum(fail), sf)
    cat(strrep("-", 78), "\n", sep = "")
  }
  invisible(list(f_max = sf[["max"]], f_med = sf[["med"]], f_min = sf[["min"]], f_mean = sf[["mean"]], N_fail = sum(fail),
                 tr_max = sk[["max"]], tr_med = sk[["med"]], tr_min = sk[["min"]], tr_mean = sk[["mean"]], tr = sum(risk),
                 gap_max = sg[["max"]], gap_med = sg[["med"]], gap_min = sg[["min"]], gap_mean = sg[["mean"]], gap = sum(gap), N_gap = ng,
                 t1_max = sx[["max"]], t1_med = sx[["med"]], t1_min = sx[["min"]], t1_mean = sx[["mean"]],
                 t0_max = se[["max"]], t0_med = se[["med"]], t0_min = se[["min"]], t0_mean = se[["mean"]],
                 N_max = sr[["max"]], N_med = sr[["med"]], N_min = sr[["min"]], N_mean = sr[["mean"]], N_total = n, N_sub = ns))
}
stata_stsum <- function(d, info, by = NULL, id = NULL, show = TRUE) {
  r <- stata_sv_rows(d)
  nsub <- function(idx) if (is.null(id)) length(idx) else length(unique(d[[id]][idx]))
  row <- function(idx) {
    risk <- sum(r$t[idx] - r$t0[idx])
    fails <- sum(r$d[idx] != 0)
    pc <- c(NA_real_, NA_real_, NA_real_)
    if (length(idx)) {
      km <- stata_sv_km(r$t[idx], r$d[idx], t0 = r$t0[idx])
      pc <- vapply(c(0.25, 0.5, 0.75), function(p) stata_sv_pct(km, p), 0)
    }
    list(risk = risk, ir = fails / risk, n = nsub(idx), pc = pc)
  }
  groups <- stata_sv_groups(d, by)
  tot <- row(seq_along(r$t))
  if (show) {
    stata_sv_head(info, id = FALSE)
    lab <- if (is.null(by)) "" else sprintf("%8s", by)
    first <- if (is.null(by)) "         " else sprintf("%8s ", by)
    cat("         |               Incidence     Number of   |------ Survival time -----|\n")
    cat(if (is.null(by)) "         " else sprintf("%8s ", by), "| Time at risk       rate      subjects        25%       50%       75%\n", sep = "")
    rule <- paste0(strrep("-", 9), "+", strrep("-", 69), "\n")
    cat(rule)
    line <- function(l, x) cat(sprintf("%8s |", l), stata_sv_comma(x$risk, 13), stata_sv_rate(x$ir, 12), stata_sv_g(x$n, 14),
                              stata_sv_g(x$pc[1], 11), stata_sv_g(x$pc[2], 10), stata_sv_g(x$pc[3], 10), "\n", sep = "")
    if (!is.null(by)) {
      for (g in groups) line(g$label, row(g$idx))
      cat(rule)
    }
    line("Total", tot)
  }
  invisible(list(p75 = tot$pc[3], p50 = tot$pc[2], p25 = tot$pc[1], N_sub = tot$n, ir = tot$ir, risk = tot$risk))
}
stata_stslist <- function(d, info, by = NULL, at = NULL, mode = "s", level = 95, show = TRUE) {
  r <- stata_sv_rows(d)
  stata_sv_needs_zero_entry(r, "sts")
  word <- c(s = "Kaplan–Meier survivor function", f = "Kaplan–Meier failure function", na = "Nelson–Aalen cumulative hazard function")[[mode]]
  colw <- c(s = "Survivor", f = "  Failure", na = "Cumulative")[[mode]]
  col2 <- c(s = "function", f = "function", na = "   hazard")[[mode]]
  vals <- function(km) {
    if (mode == "s") list(v = km$S, se = km$se, lb = km$lb, ub = km$ub)
    else if (mode == "f") list(v = 1 - km$S, se = km$se, lb = 1 - km$ub, ub = 1 - km$lb)
    else list(v = km$na, se = km$nase, lb = km$nalb, ub = km$naub)
  }
  groups <- stata_sv_groups(d, by)
  if (show) {
    stata_sv_head(info, id = FALSE)
    cat(word, "\n\n", sep = "")
    if (!is.null(by)) cat("By variable: ", by, "\n\n", sep = "")
  }
  tab <- function(km, at) {
    v <- vals(km)
    if (is.null(at)) {
      list(time = km$time, n = km$n, fail = km$fail, lost = km$lost, v = v$v, se = v$se, lb = v$lb, ub = v$ub)
    } else {
      pos <- vapply(at, function(a) sum(km$time <= a), 0)
      if (any(pos == 0)) stop("sts list, at() before the first time is not translated yet", call. = FALSE)
      cf <- c(0, cumsum(km$fail))
      list(time = at, n = km$n[pos], fail = diff(c(0, cf[pos + 1])), v = v$v[pos], se = v$se[pos], lb = v$lb[pos], ub = v$ub[pos])
    }
  }
  one <- function(km, grp = NULL) {
    t <- tab(km, at)
    f <- function(x, w) stata_sv_f(x, w)
    if (is.null(at)) {
      for (i in seq_along(t$time)) {
        cat(sprintf("%6s", format(t$time[i], digits = 8)), sprintf("%9d%7d%7d", as.integer(t$n[i]), as.integer(t$fail[i]), as.integer(t$lost[i])),
            f(t$v[i], 12), f(t$se[i], 10), f(t$lb[i], 11), f(t$ub[i], 10), "\n", sep = "")
      }
    } else {
      for (i in seq_along(t$time)) {
        cat(sprintf("%8s", format(t$time[i], digits = 8)), sprintf("%9d%9d", as.integer(t$n[i]), as.integer(t$fail[i])),
            f(t$v[i], 12), f(t$se[i], 10), f(t$lb[i], 11), f(t$ub[i], 10), "\n", sep = "")
      }
    }
    invisible(t)
  }
  if (show) {
    lvl <- format(level)
    if (is.null(at)) {
      cat("             At", if (is.null(by)) "                " else "           Net  ", sprintf("%10s      Std.\n", colw), sep = "")
      cat(if (is.null(by)) "  Time     risk   Fail   Lost" else "  Time     risk   Fail   lost", sprintf("%11s     error     [%s%% conf. int.]\n", col2, lvl), sep = "")
      cat(strrep("-", 72), "\n", sep = "")
    } else {
      cat("              Beg.", sprintf("%19s      Std.\n", colw), sep = "")
      cat("    Time     total     Fail", sprintf("%11s     error     [%s%% conf. int.]\n", col2, lvl), sep = "")
      cat(strrep("-", 70), "\n", sep = "")
    }
  }
  for (g in groups) {
    km <- stata_sv_km(r$t[g$idx], r$d[g$idx], level)
    if (show) {
      if (!is.null(g$label)) cat(by, "=", g$label, " \n", sep = "")
      one(km)
    }
  }
  if (show) {
    cat(strrep("-", if (is.null(at)) 72 else 70), "\n", sep = "")
    if (!is.null(by) && is.null(at)) cat("Note: Net lost equals the number lost minus the number who entered.\n")
    if (!is.null(at)) cat("Note: Survivor function is calculated over full data and evaluated at\n      indicated times; it is not calculated from aggregates shown at left.\n")
  }
  invisible(NULL)
}
# sts generate: one new column per request (name, what), by group, on the rows `use` picks
stata_stsgen <- function(d, use, reqs, by = NULL, level = 95) {
  sub <- d[use, , drop = FALSE]
  r <- stata_sv_rows(sub)
  stata_sv_needs_zero_entry(r, "sts generate")
  groups <- stata_sv_groups(sub, by)
  pos <- which(use)
  for (rq in reqs) d[[rq$name]] <- rep(NA_real_, nrow(d))
  for (g in groups) {
    km <- stata_sv_km(r$t[g$idx], r$d[g$idx], level)
    m <- match(r$t[g$idx], km$time)
    for (rq in reqs) {
      col <- switch(rq$what,
        s = km$S, "se(s)" = km$se, "lb(s)" = km$lb, "ub(s)" = km$ub,
        na = km$na, "se(na)" = ifelse(km$na > 0, km$nase, NA_real_), "lb(na)" = km$nalb, "ub(na)" = km$naub,
        h = km$h, n = ifelse(km$fail > 0, km$n, NA_real_), d = ifelse(km$fail > 0, km$fail, NA_real_))
      d[[rq$name]][pos[g$idx]] <- col[m]
    }
  }
  lab <- c(s = "S(t)", "se(s)" = "Standard error of S(t)", "lb(s)" = "Lower bound of CI for S(t)", "ub(s)" = "Upper bound of CI for S(t)",
           na = "Nelson–Aalen cumulative hazard", "se(na)" = "Standard error of Nelson–Aalen cumulative hazard",
           "lb(na)" = "Lower bound of CI for Nelson–Aalen cumulative hazard", "ub(na)" = "Upper bound of CI for Nelson–Aalen cumulative hazard",
           h = "Hazard function", n = "N, entering population", d = "d, number of failures")
  for (rq in reqs) attr(d[[rq$name]], "label") <- unname(lab[rq$what])
  d
}
# stir: the incidence-rate comparison of failures and time at risk by exposure
stata_stir <- function(d, info, expo, strata = NULL, level = 95, show = TRUE) {
  r <- stata_sv_rows(d)
  x <- stata_sv_num(d[[expo]])
  vals <- sort(unique(x[!is.na(x)]))
  if (length(vals) != 2) stop("stir with an exposure variable that does not have exactly two values is not translated yet", call. = FALSE)
  df <- data.frame(Failures = as.numeric(r$d != 0), Time = r$t - r$t0, e = as.numeric(x == vals[2]))
  names(df)[3] <- expo
  if (!is.null(strata)) df[[strata]] <- d[[strata]]
  df <- df[!is.na(x), , drop = FALSE]
  if (show) {
    stata_sv_head(info, id = TRUE)
    cat("Exposed:   ", expo, " = ", format(vals[2], digits = 8), "\nUnexposed: ", expo, " = ", format(vals[1], digits = 8), "\n", sep = "")
  }
  if (is.null(strata)) {
    n <- stata_epi_counts(df, "Failures", expo, "Time")
    stata_epi("ir", n$a, n$b, t1 = n$t1, t0 = n$t0, level = level, expo = n$expo, case = "Failures", time = "Time", show = show)
  } else {
    stata_epi_by("ir", df, "Failures", expo, strata, time = "Time", level = level, show = show)
  }
}
# sts test: the weighted log-rank family
stata_ststest <- function(d, info, by, test = "logrank", p = 0, q = 0, strata = NULL, show = TRUE) {
  r <- stata_sv_rows(d)
  stata_sv_needs_zero_entry(r, "sts test")
  gv <- d[[by]]
  gk <- if (is.character(gv) || is.factor(gv)) as.character(gv) else stata_sv_num(gv)
  ok <- !is.na(gk)
  lv <- sort(unique(gk[ok]))
  K <- length(lv)
  gi <- match(gk, lv)
  st <- if (is.null(strata)) rep(1, length(r$t)) else {
    v <- d[[strata]]
    if (is.character(v) || is.factor(v)) as.character(v) else stata_sv_num(v)
  }
  U <- rep(0, K)
  V <- matrix(0, K, K)
  E <- rep(0, K)
  O <- rep(0, K)
  for (s in unique(st[ok])) {
    sel <- ok & st == s
    t <- r$t[sel]; dd <- r$d[sel] != 0; g <- gi[sel]
    ut <- sort(unique(t[dd]))
    allt <- sort(unique(t))
    # the Kaplan-Meier (and Peto-Peto) estimates over all times of the stratum, all groups together
    ix <- match(t, allt)
    nf <- tabulate(ix[dd], length(allt)); nl <- tabulate(ix[!dd], length(allt))
    nn <- rev(cumsum(rev(nf + nl)))
    Skm <- cumprod(1 - nf / nn)
    Spp <- cumprod(1 - nf / (nn + 1))
    for (j in match(ut, allt)) {
      tj <- allt[j]
      nk <- vapply(seq_len(K), function(k) sum(t >= tj & g == k), 0)
      dk <- vapply(seq_len(K), function(k) sum(t == tj & dd & g == k), 0)
      n <- sum(nk); dj <- sum(dk)
      w <- switch(test, logrank = 1, wilcoxon = n, tware = sqrt(n), peto = Spp[j],
                  fh = (if (j > 1) Skm[j - 1] else 1)^p * (1 - (if (j > 1) Skm[j - 1] else 1))^q)
      ek <- dj * nk / n
      U <- U + w * (dk - ek)
      E <- E + ek
      O <- O + dk
      if (n > 1) {
        v <- dj * (n - dj) / (n - 1) * (diag(nk / n, K) - outer(nk, nk) / n^2)
        V <- V + w^2 * v
      }
    }
  }
  chi2 <- if (K > 1) drop(t(U[-K]) %*% solve(V[-K, -K, drop = FALSE], U[-K])) else NA_real_
  df <- K - 1
  if (show) {
    stata_sv_head(info, id = FALSE)
    cat("Equality of survivor functions\n")
    cat(c(logrank = if (is.null(strata)) "Log-rank test" else "Stratified log-rank test", wilcoxon = "Wilcoxon–Breslow–Gehan test",
          tware = "Tarone–Ware test", peto = "Peto–Peto–Prentice test", fh = "Fleming–Harrington test")[[test]], "\n", sep = "")
    if (test == "fh") cat("Powers: p = ", format(p), ", q = ", format(q), "\n", sep = "")
    if (!is.null(strata)) cat("Strata: ", strata, "\n", sep = "")
    cat("\n")
    ranks <- test != "logrank"
    cat(sprintf("%5s |  Observed       Expected%s\n", "", if (ranks) "       Sum of" else ""))
    cat(sprintf("%5s |    events         events%s%s\n", by, if (!is.null(strata) && !ranks) "*" else "", if (ranks) "        ranks" else ""))
    cat(strrep("-", 6), "+", strrep("-", if (ranks) 38 else 25), "\n", sep = "")
    for (k in seq_len(K)) cat(sprintf("%5s |%10d%15.2f", format(lv[k], digits = 8), as.integer(O[k]), E[k]), if (ranks) sprintf("%13s", stata_fmt(U[k], "%10.0g", trim = TRUE)), "\n", sep = "")
    cat(strrep("-", 6), "+", strrep("-", if (ranks) 38 else 25), "\n", sep = "")
    cat(sprintf("%5s |%10d%15.2f", "Total", as.integer(sum(O)), sum(E)), if (ranks) sprintf("%13s", stata_fmt(sum(U), "%10.0g", trim = TRUE)), "\n", sep = "")
    if (!is.null(strata) && !ranks) cat("\n* aggregated over stratum-specific results\n")
    cat(sprintf("\n%s chi2(%d) = %7.2f\n%s Pr>chi2 = %6.4f\n", strrep(" ", if (ranks) 25 else 16), as.integer(df), chi2, strrep(" ", if (ranks) 25 else 16),
                stats::pchisq(chi2, df, lower.tail = FALSE)))
  }
  invisible(list(chi2 = chi2, df = df))
}
"##;

/// `stptime` and `ltable`.
pub(super) const STPTIME_HELPER: &str = r##"stata_stptime <- function(d, info, by = NULL, at = NULL, trim = FALSE, per = 1, dd = NULL, level = 95, show = TRUE) {
  r <- stata_sv_rows(d)
  z <- stats::qnorm(1 - (100 - level) / 200)
  piece <- function(idx, a, b) {
    lo <- pmax(r$t0[idx], a)
    hi <- pmin(r$t[idx], b)
    c(sum(pmax(0, hi - lo)), sum(r$d[idx] != 0 & r$t[idx] > a & r$t[idx] <= b))
  }
  est <- function(pt, fl) {
    rt <- fl / pt * per
    c(rt, if (fl > 0) rt * exp(-z / sqrt(fl)) else NA_real_, if (fl > 0) rt * exp(z / sqrt(fl)) else NA_real_)
  }
  num <- function(v, w, fmt = "%9.0g") if (is.null(dd)) stata_sv_rate(v, w, fmt) else sprintf("%*s", w, ifelse(is.na(v), ".", sprintf("%.*f", dd, v)))
  cuts <- at
  pieces <- function(idx) {
    if (is.null(cuts)) return(list(list(label = "Total", v = piece(idx, -Inf, Inf))))
    k <- length(cuts)
    out <- lapply(seq_len(k - 1), function(i) list(label = sprintf("(%-4s–%4s]", format(cuts[i]), format(cuts[i + 1])), v = piece(idx, cuts[i], cuts[i + 1])))
    if (!trim) out[[k]] <- list(label = sprintf("     >%4s ", format(cuts[k])), v = piece(idx, cuts[k], Inf))
    out
  }
  groups <- stata_sv_groups(d, by)
  all <- seq_along(r$t)
  tot <- if (is.null(cuts)) piece(all, -Inf, Inf) else Reduce(`+`, lapply(pieces(all), function(p) p$v))
  te <- est(tot[1], tot[2])
  if (show) {
    stata_sv_head(info)
    cat("Estimated person-time and incidence rate", if (!is.null(by) || !is.null(cuts)) "s", "\n\n", sep = "")
    head1 <- if (!is.null(by)) sprintf("%10s |", by) else "    Cohort |"
    cat(head1, "  Person-time   Failures        Rate   [", format(level), "% conf. interval]\n", sep = "")
    rule <- paste0(strrep("-", 11), "+", strrep("-", 59), "\n")
    cat(rule)
    line <- function(lab, v, bar = "|") {
      e <- est(v[1], v[2])
      cat(sprintf("%11s", lab), bar, num(v[1], 13), sprintf("%11d", as.integer(v[2])), num(e[1], 12, "%10.0g"), num(e[2], 12), num(e[3], 12), "\n", sep = "")
    }
    if (!is.null(by)) {
      for (g in groups) {
        if (!is.null(cuts)) {
          cat(sprintf("%-11s|\n", g$label))
          for (p in pieces(g$idx)) line(p$label, p$v)
          cat(rule)
        } else {
          line(sprintf("%10s ", g$label), piece(g$idx, -Inf, Inf))
        }
      }
      if (is.null(cuts)) cat(rule)
      line("     Total ", tot)
    } else if (!is.null(cuts)) {
      for (p in pieces(all)) line(p$label, p$v)
      cat(rule)
      line("     Total ", tot)
    } else line("     Total ", tot)
  }
  invisible(list(ptime = tot[1], failures = tot[2], rate = te[1], lb = te[2], ub = te[3]))
}
stata_ltable <- function(d, time, fail = NULL, w = NULL, by = NULL, width = 1, cuts = NULL, mode = "s", noadjust = FALSE, level = 95, test = FALSE, show = TRUE) {
  t <- stata_sv_num(d[[time]])
  f <- if (is.null(fail)) rep(1, length(t)) else as.numeric(stata_sv_num(d[[fail]]) != 0)
  wt <- if (is.null(w)) rep(1, length(t)) else stata_sv_num(d[[w]])
  ok <- !is.na(t) & !is.na(f) & !is.na(wt)
  z <- stats::qnorm(1 - (100 - level) / 200)
  start <- if (is.null(cuts)) floor(t / width) * width else vapply(t, function(x) if (is.na(x) || x < cuts[1]) NA_real_ else max(cuts[cuts <= x]), 0)
  ok <- ok & !is.na(start)
  tab <- function(idx) {
    idx <- idx[ok[idx]]
    st <- sort(unique(start[idx]))
    rows <- lapply(st, function(s) {
      inn <- idx[start[idx] == s]
      beg <- sum(wt[idx[start[idx] >= s]])
      dk <- sum(wt[inn][f[inn] == 1])
      lk <- sum(wt[inn][f[inn] == 0])
      last <- !is.null(cuts) && s == max(cuts)
      wd <- if (is.null(cuts)) width else if (last) NA_real_ else cuts[match(s, cuts) + 1] - s
      end <- if (is.null(cuts)) s + width else if (last) NA_real_ else cuts[match(s, cuts) + 1]
      c(start = s, end = end, wd = wd, beg = beg, d = dk, l = lk)
    })
    m <- do.call(rbind, rows)
    np <- if (noadjust) m[, "beg"] else m[, "beg"] - m[, "l"] / 2
    S <- cumprod(1 - m[, "d"] / np)
    gw <- cumsum(ifelse(m[, "d"] > 0, m[, "d"] / (np * (np - m[, "d"])), 0))
    edge <- !is.finite(S) | S <= 0 | S >= 1
    se <- ifelse(!is.finite(S) | S <= 0, NA_real_, S * sqrt(gw))
    sell <- ifelse(edge, NA_real_, sqrt(gw) / abs(log(S)))
    lb <- ifelse(edge, NA_real_, S^exp(z * sell))
    ub <- ifelse(edge, NA_real_, S^exp(-z * sell))
    wd <- ifelse(is.na(m[, "wd"]), 1, m[, "wd"])
    h <- m[, "d"] / (wd * (np - m[, "d"] / 2))
    hse <- ifelse(m[, "d"] > 0, h * sqrt((1 - (h * wd / 2)^2) / m[, "d"]), NA_real_)
    hlb <- ifelse(m[, "d"] > 0, pmax(0, h - z * hse), NA_real_)
    hub <- ifelse(m[, "d"] > 0, h + z * hse, NA_real_)
    list(m = m, S = S, se = se, lb = lb, ub = ub, h = h, hse = hse, hlb = hlb, hub = hub)
  }
  fm <- function(x, wd, dig = 4) stata_sv_f(x, wd, dig)
  show_tab <- function(t) {
    m <- t$m
    for (i in seq_len(nrow(m))) {
      end <- if (is.na(m[i, "end"])) "." else format(m[i, "end"], digits = 8)
      pre <- sprintf("%5s%6s", format(m[i, "start"], digits = 8), end)
      if (mode == "h") {
        cat(pre, sprintf("%10d", as.integer(m[i, "beg"])), fm(1 - t$S[i], 10), fm(t$se[i], 8), fm(t$h[i], 10), fm(t$hse[i], 10), fm(t$hlb[i], 10), fm(t$hub[i], 10), "\n", sep = "")
      } else if (mode == "f") {
        cat(pre, sprintf("%10d%9d%7d", as.integer(m[i, "beg"]), as.integer(m[i, "d"]), as.integer(m[i, "l"])), fm(1 - t$S[i], 11), fm(t$se[i], 10), fm(1 - t$ub[i], 11), fm(1 - t$lb[i], 10), "\n", sep = "")
      } else {
        cat(pre, sprintf("%10d%9d%7d", as.integer(m[i, "beg"]), as.integer(m[i, "d"]), as.integer(m[i, "l"])), fm(t$S[i], 11), fm(t$se[i], 10), fm(t$lb[i], 11), fm(t$ub[i], 10), "\n", sep = "")
      }
    }
  }
  head <- function() {
    if (mode == "h") {
      cat("                 Beg.     Cum.     Std.                Std.\n   Interval     Total   Failure   Error    Hazard     Error    [", format(level), "% Conf. Int.]\n", sep = "")
    } else if (mode == "f") {
      cat("                 Beg.                      Cum.       Std.\n   Interval     Total   Deaths   Lost    Failure     Error     [", format(level), "% Conf. Int.]\n", sep = "")
    } else {
      cat("                 Beg.                                 Std.\n   Interval     total   Deaths   Lost   Survival     error     [", format(level), "% conf. int.]\n", sep = "")
    }
    cat(strrep("-", 79), "\n", sep = "")
  }
  groups <- stata_sv_groups(d, by)
  if (show) {
    cat("\n")
    head()
  }
  for (g in groups) {
    tt <- tab(g$idx)
    if (show) {
      if (!is.null(g$label)) cat(by, " = ", g$label, "\n", sep = "")
      show_tab(tt)
    }
  }
  if (show) cat(strrep("-", 79), "\n", sep = "")
  out <- list()
  if (test && !is.null(by)) {
    {
      D <- vapply(groups, function(g) { i <- g$idx[ok[g$idx]]; sum(wt[i][f[i] == 1]) }, 0)
      Tm <- vapply(groups, function(g) { i <- g$idx[ok[g$idx]]; sum(wt[i] * t[i]) }, 0)
      lr <- 2 * (sum(ifelse(D > 0, D * log(D / Tm), 0)) - sum(D) * log(sum(D) / sum(Tm)))
      df <- length(groups) - 1
      if (show) cat("\n\nLikelihood-ratio test statistic of homogeneity (", by, "=", by, "):\nchi2( ", df, " ) = ", format(lr, digits = 7), ",   P = ", format(stats::pchisq(lr, df, lower.tail = FALSE), digits = 7), "\n \nLogrank test of homogeneity (", by, "=", by, "):\n", sep = "")
    }
    dd <- d[ok, , drop = FALSE]
    dd[["_t"]] <- t[ok]
    dd[["_d"]] <- f[ok]
    dd[["_t0"]] <- rep(0, sum(ok))
    if (!is.null(w)) dd <- dd[rep(seq_len(nrow(dd)), times = wt[ok]), , drop = FALSE]
    out <- stata_ststest(dd, list(fail = "", time = ""), by, "logrank", show = show)
  }
  invisible(out)
}"##;

const R_STDESCRIBE: &[&str] = &[
    "f_max", "f_med", "f_min", "f_mean", "N_fail", "tr_max", "tr_med", "tr_min", "tr_mean", "tr",
    "gap_max", "gap_med", "gap_min", "gap_mean", "gap", "N_gap", "t1_max", "t1_med", "t1_min",
    "t1_mean", "t0_max", "t0_med", "t0_min", "t0_mean", "N_max", "N_med", "N_min", "N_mean",
    "N_total", "N_sub",
];
const R_STSUM: &[&str] = &["p75", "p50", "p25", "N_sub", "ir", "risk"];
const R_STPTIME: &[&str] = &["ptime", "failures", "rate", "lb", "ub"];
const R_STTEST: &[&str] = &["chi2", "df"];

/// The numbers of an `at()` list: plain, ascending, without a missing value.
fn at_list(text: &str, what: &str) -> Result<Vec<f64>, Untranslated> {
    let mut v: Vec<f64> = Vec::new();
    for w in text.split([' ', ',']).filter(|w| !w.is_empty()) {
        let x: f64 = w.parse().map_err(|_| {
            no(format!(
                "{what}, at({text}): only plain numbers are translated"
            ))
        })?;
        if !x.is_finite() {
            return Err(no(format!("{what}, at({text}): only plain numbers")));
        }
        v.push(x);
    }
    if v.len() < 2 || !increasing(&v) {
        return Err(no(format!(
            "{what}, at({text}): two or more increasing numbers are translated"
        )));
    }
    Ok(v)
}

/// Whether the numbers rise strictly.
fn increasing(v: &[f64]) -> bool {
    v.windows(2).all(|p| matches!(p, [a, b] if a < b))
}

fn stop_line(out: &mut Out, msg: &str) {
    out.line(format!("stop({}, call. = FALSE)", r_text(msg)));
}

impl Session {
    /// What `stset` declared, or Stata's error for data that are not st.
    fn st_declared(&mut self, out: &mut Out) -> Option<StSet> {
        match self.stset.clone() {
            Some(s) => Some(s),
            None => {
                stop_line(out, "data not st (Stata: r(119))");
                None
            }
        }
    }

    pub(super) fn st_info(st: &StSet) -> String {
        format!(
            "list(fail = {}, time = {}, id = {})",
            r_text(&st.fail),
            r_text(&st.time),
            st.id.as_deref().map_or("NULL".to_string(), r_text)
        )
    }

    /// The rows of the data `stset` uses (and `if` and `in` pick).
    fn st_rows(&mut self, c: &Command, out: &mut Out) -> Result<String, Untranslated> {
        let rows = self.rows_for(c, out)?;
        out.package("dplyr");
        Ok(format!("dplyr::filter({rows}, `_st` == 1)"))
    }

    fn st_by(opts: &Opts, what: &str) -> Result<Option<String>, Untranslated> {
        match opts.arg("by", 2) {
            None => Ok(None),
            Some(v) if is_name(v.trim()) => Ok(Some(v.trim().to_string())),
            Some(v) => Err(no(format!(
                "{what}, by({v}): one variable name is translated"
            ))),
        }
    }

    fn st_level(opts: &Opts) -> Result<f64, Untranslated> {
        Ok(super::returned::percent(super::model::level(opts)?))
    }

    pub(super) fn stdescribe(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "stdescribe")?;
        if let Some(o) = opts.other(&[("noshow", 4)]) {
            return Err(no(format!(
                "stdescribe with the option {o} is not translated yet"
            )));
        }
        if !c.parts.main.trim().is_empty() {
            return Err(no("stdescribe takes no varlist here"));
        }
        let Some(st) = self.st_declared(out) else {
            return Ok(());
        };
        let src = self.st_rows(c, out)?;
        let id = st.id.as_deref().map_or("NULL".to_string(), r_text);
        let show = !self.quiet(c);
        for h in ["stata_survival", "stata_fmt", "stata_returned"] {
            self.helpers.insert(h);
        }
        out.line(format!(
            "stata_r <- stata_stdescribe({src}, {}, id = {id}, show = {})",
            Self::st_info(&st),
            super::stats::r_logical(show)
        ));
        self.r_new = Some((R_STDESCRIBE, "stdescribe".into()));
        Ok(())
    }

    pub(super) fn stsum(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "stsum")?;
        if let Some(o) = opts.other(&[("by", 2), ("noshow", 4)]) {
            return Err(no(format!(
                "stsum with the option {o} is not translated yet"
            )));
        }
        if !c.parts.main.trim().is_empty() {
            return Err(no("stsum takes no varlist"));
        }
        let by = Self::st_by(opts, "stsum")?;
        let Some(st) = self.st_declared(out) else {
            return Ok(());
        };
        let src = self.st_rows(c, out)?;
        let id = st.id.as_deref().map_or("NULL".to_string(), r_text);
        for h in ["stata_survival", "stata_fmt", "stata_returned"] {
            self.helpers.insert(h);
        }
        out.line(format!(
            "stata_r <- stata_stsum({src}, {}, by = {}, id = {id}, show = {})",
            Self::st_info(&st),
            by.as_deref().map_or("NULL".to_string(), r_text),
            super::stats::r_logical(!self.quiet(c))
        ));
        self.r_new = Some((R_STSUM, "stsum".into()));
        Ok(())
    }

    pub(super) fn sts(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "sts")?;
        let main = c.parts.main.trim();
        let (sub, rest) = first_word(main);
        let by = Self::st_by(opts, "sts")?;
        match sub {
            "list" | "l" | "li" | "lis" => {
                if let Some(o) = opts.other(&[
                    ("by", 2),
                    ("at", 2),
                    ("failure", 1),
                    ("survival", 1),
                    ("na", 2),
                    ("level", 3),
                    ("noshow", 4),
                ]) {
                    return Err(no(format!(
                        "sts list with the option {o} is not translated yet"
                    )));
                }
                if !rest.is_empty() {
                    return Err(no("sts list takes no varlist"));
                }
                let modes = [
                    opts.has("failure", 1),
                    opts.has("survival", 1),
                    opts.has("na", 2),
                ];
                if modes.iter().filter(|m| **m).count() > 1 {
                    return Err(no("sts list: one of survival, failure and na"));
                }
                let mode = if modes[0] {
                    "f"
                } else if modes[2] {
                    "na"
                } else {
                    "s"
                };
                let at = match opts.arg("at", 2) {
                    Some(a) => {
                        let v = at_list(&a, "sts list")?;
                        format!(
                            "c({})",
                            v.iter()
                                .map(|x| r_number(*x))
                                .collect::<Vec<_>>()
                                .join(", ")
                        )
                    }
                    None => "NULL".to_string(),
                };
                let level = Self::st_level(opts)?;
                let Some(st) = self.st_declared(out) else {
                    return Ok(());
                };
                let src = self.st_rows(c, out)?;
                for h in ["stata_survival", "stata_fmt"] {
                    self.helpers.insert(h);
                }
                out.line(format!(
                    "stata_stslist({src}, {}, by = {}, at = {at}, mode = {}, level = {}, show = {})",
                    Self::st_info(&st),
                    by.as_deref().map_or("NULL".to_string(), r_text),
                    r_text(mode),
                    r_number(level),
                    super::stats::r_logical(!self.quiet(c))
                ));
                Ok(())
            }
            "generate" | "gen" | "g" | "ge" | "gene" | "gener" | "genera" | "generat" => {
                if let Some(o) = opts.other(&[("by", 2), ("level", 3)]) {
                    return Err(no(format!(
                        "sts generate with the option {o} is not translated yet"
                    )));
                }
                // name = function, one or more
                let words: Vec<&str> = rest.split_whitespace().collect();
                let flat = words.join(" ").replace(" = ", "=").replace("= ", "=");
                let mut reqs: Vec<(String, String)> = Vec::new();
                for item in flat.split_whitespace() {
                    let Some((name, what)) = item.split_once('=') else {
                        return Err(no(format!("sts generate: {item} is not name = function")));
                    };
                    let what = what.trim().to_lowercase();
                    let ok = matches!(
                        what.as_str(),
                        "s" | "se(s)"
                            | "lb(s)"
                            | "ub(s)"
                            | "na"
                            | "se(na)"
                            | "lb(na)"
                            | "ub(na)"
                            | "h"
                            | "n"
                            | "d"
                    );
                    if !is_name(name) || !ok {
                        return Err(no(format!(
                            "sts generate {item}: s, se(s), lb(s), ub(s), na, se(na), lb(na), ub(na), h, n and d are translated"
                        )));
                    }
                    reqs.push((name.to_string(), what));
                }
                if reqs.is_empty() {
                    return Err(no("sts generate needs name = function"));
                }
                let level = Self::st_level(opts)?;
                let Some(_st) = self.st_declared(out) else {
                    return Ok(());
                };
                for (name, _) in &reqs {
                    if self.kinds.contains_key(name) {
                        stop_line(out, "variable already defined (Stata: r(110))");
                        return Ok(());
                    }
                }
                let d = self.data()?;
                let cond = self.condition(c, out)?;
                let used = match cond {
                    Some(cond) => {
                        out.package("dplyr");
                        format!("local({{ s <- dplyr::transmute({d}, s = {cond})$s; !is.na(s) & s }}) & !is.na({d}[[\"_st\"]]) & {d}[[\"_st\"]] == 1")
                    }
                    None => format!("!is.na({d}[[\"_st\"]]) & {d}[[\"_st\"]] == 1"),
                };
                let list: Vec<String> = reqs
                    .iter()
                    .map(|(n, w)| format!("list(name = {}, what = {})", r_text(n), r_text(w)))
                    .collect();
                self.helpers.insert("stata_survival");
                self.helpers.insert("stata_fmt");
                out.line(format!(
                    "{d} <- stata_stsgen({d}, {used}, list({}), by = {}, level = {})",
                    list.join(", "),
                    by.as_deref().map_or("NULL".to_string(), r_text),
                    r_number(level)
                ));
                for (name, _) in &reqs {
                    self.kinds.insert(name.clone(), Kind::Number);
                }
                Ok(())
            }
            "test" | "t" | "te" | "tes" => {
                if let Some(o) = opts.other(&[
                    ("logrank", 3),
                    ("wilcoxon", 3),
                    ("tware", 2),
                    ("peto", 4),
                    ("fh", 2),
                    ("strata", 3),
                    ("noshow", 4),
                ]) {
                    return Err(no(format!(
                        "sts test with the option {o} is not translated yet"
                    )));
                }
                let var = rest.trim();
                if !is_name(var) {
                    return Err(no("sts test takes one variable name"));
                }
                let kinds = [
                    opts.has("logrank", 3),
                    opts.has("wilcoxon", 3),
                    opts.has("tware", 2),
                    opts.has("peto", 4),
                    opts.has("fh", 2),
                ];
                if kinds.iter().filter(|k| **k).count() > 1 {
                    return Err(no("sts test: one test at a time"));
                }
                let (test, extra) = if opts.has("wilcoxon", 3) {
                    ("wilcoxon", String::new())
                } else if opts.has("tware", 2) {
                    ("tware", String::new())
                } else if opts.has("peto", 4) {
                    ("peto", String::new())
                } else if let Some(a) = opts.arg("fh", 2) {
                    let nums: Vec<f64> = a
                        .split([' ', ','])
                        .filter(|w| !w.is_empty())
                        .filter_map(|w| w.parse().ok())
                        .collect();
                    let [p, q] = nums.as_slice() else {
                        return Err(no("sts test, fh(p q) takes two numbers"));
                    };
                    (
                        "fh",
                        format!(", p = {}, q = {}", r_number(*p), r_number(*q)),
                    )
                } else {
                    ("logrank", String::new())
                };
                let strata = match opts.arg("strata", 3) {
                    None => String::new(),
                    Some(s) if is_name(s.trim()) => format!(", strata = {}", r_text(s.trim())),
                    Some(s) => return Err(no(format!("sts test, strata({s}): one variable name"))),
                };
                let Some(st) = self.st_declared(out) else {
                    return Ok(());
                };
                let src = self.st_rows(c, out)?;
                for h in ["stata_survival", "stata_fmt", "stata_returned"] {
                    self.helpers.insert(h);
                }
                out.line(format!(
                    "stata_r <- stata_ststest({src}, {}, {}, test = {}{extra}{strata}, show = {})",
                    Self::st_info(&st),
                    r_text(var),
                    r_text(test),
                    super::stats::r_logical(!self.quiet(c))
                ));
                self.r_new = Some((R_STTEST, "sts".into()));
                Ok(())
            }
            "graph" | "gr" | "gra" | "grap" => {
                if !rest.is_empty() {
                    return Err(no("sts graph takes no varlist"));
                }
                self.sts_graph(c, opts, out)
            }
            other => Err(no(format!("sts {other} is not translated yet"))),
        }
    }

    pub(super) fn stptime(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "stptime")?;
        if let Some(o) = opts.other(&[
            ("by", 2),
            ("at", 2),
            ("trim", 4),
            ("per", 3),
            ("dd", 2),
            ("level", 3),
            ("title", 5),
            ("noshow", 4),
        ]) {
            return Err(no(format!(
                "stptime with the option {o} is not translated yet"
            )));
        }
        if !c.parts.main.trim().is_empty() {
            return Err(no("stptime takes no varlist"));
        }
        let by = Self::st_by(opts, "stptime")?;
        let Some(st) = self.st_declared(out) else {
            return Ok(());
        };
        if st.id.is_none() {
            stop_line(
                out,
                "stptime requires that you have previously stset an id() variable (Stata: r(198))",
            );
            return Ok(());
        }
        let at = match opts.arg("at", 2) {
            Some(a) => {
                if a.split([' ', ',']).any(|w| w == ".") {
                    stop_line(out, "missing values not allowed in at() (Stata: r(127))");
                    return Ok(());
                }
                let v = at_list(&a, "stptime")?;
                format!(
                    "c({})",
                    v.iter()
                        .map(|x| r_number(*x))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            None => "NULL".to_string(),
        };
        let per = match opts.arg("per", 3) {
            Some(p) => p
                .trim()
                .parse::<f64>()
                .map_err(|_| no(format!("stptime, per({p}) needs a number")))?,
            None => 1.0,
        };
        let dd = match opts.arg("dd", 2) {
            Some(p) => p
                .trim()
                .parse::<u32>()
                .map_err(|_| no(format!("stptime, dd({p}) needs a whole number")))?
                .to_string(),
            None => "NULL".to_string(),
        };
        let level = Self::st_level(opts)?;
        let src = self.st_rows(c, out)?;
        for h in [
            "stata_survival",
            "stata_stptime",
            "stata_fmt",
            "stata_returned",
        ] {
            self.helpers.insert(h);
        }
        out.line(format!(
            "stata_r <- stata_stptime({src}, {}, by = {}, at = {at}, trim = {}, per = {}, dd = {dd}, level = {}, show = {})",
            Self::st_info(&st),
            by.as_deref().map_or("NULL".to_string(), r_text),
            super::stats::r_logical(opts.has("trim", 4)),
            r_number(per),
            r_number(level),
            super::stats::r_logical(!self.quiet(c))
        ));
        self.r_new = Some((R_STPTIME, "stptime".into()));
        Ok(())
    }

    pub(super) fn stir(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "stir")?;
        if let Some(o) = opts.other(&[("strata", 3), ("level", 3), ("noshow", 4)]) {
            return Err(no(format!(
                "stir with the option {o} is not translated yet"
            )));
        }
        let expo = c.parts.main.trim();
        if !is_name(expo) {
            return Err(no("stir takes one exposure variable name"));
        }
        let strata = match opts.arg("strata", 3) {
            None => None,
            Some(s) if is_name(s.trim()) && s.trim() != expo => Some(s.trim().to_string()),
            Some(s) => {
                return Err(no(format!(
                    "stir, strata({s}): one variable name other than the exposure"
                )))
            }
        };
        let level = Self::st_level(opts)?;
        let Some(st) = self.st_declared(out) else {
            return Ok(());
        };
        if st.id.is_none() {
            stop_line(
                out,
                "stir requires that you have previously stset an id() variable (Stata: r(198))",
            );
            return Ok(());
        }
        let src = self.st_rows(c, out)?;
        for h in [
            "stata_survival",
            "stata_epitab",
            "stata_fmt",
            "stata_returned",
            "stata_tables",
        ] {
            self.helpers.insert(h);
        }
        out.line(format!(
            "stata_r <- stata_stir({src}, {}, {}, strata = {}, level = {}, show = {})",
            Self::st_info(&st),
            r_text(expo),
            strata.as_deref().map_or("NULL".to_string(), r_text),
            r_number(level),
            super::stats::r_logical(!self.quiet(c))
        ));
        let names = if strata.is_some() {
            super::epitab::R_IR_BY
        } else {
            super::epitab::R_IR
        };
        self.r_new = Some((names, "stir".into()));
        Ok(())
    }

    pub(super) fn ltable(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        use crate::lang::syntax::WeightKind;
        self.no_by(c, "ltable")?;
        if let Some(o) = opts.other(&[
            ("by", 2),
            ("intervals", 2),
            ("survival", 1),
            ("failure", 1),
            ("hazard", 3),
            ("noadjust", 3),
            ("level", 3),
            ("test", 2),
        ]) {
            return Err(no(format!(
                "ltable with the option {o} is not translated yet"
            )));
        }
        let words: Vec<&str> = c.parts.main.split_whitespace().collect();
        let (time, fail) = match words.as_slice() {
            [t] => (*t, None),
            [t, f] => (*t, Some(*f)),
            _ => return Err(no("ltable takes a time variable and a failure variable")),
        };
        if !is_name(time) || fail.is_some_and(|f| !is_name(f)) {
            return Err(no("ltable: variable names written out are translated"));
        }
        let by = Self::st_by(opts, "ltable")?;
        let wt = match &c.parts.weight {
            None => None,
            Some(w) if w.kind == WeightKind::Frequency && is_name(w.exp.trim()) => {
                Some(w.exp.trim().to_string())
            }
            Some(_) => return Err(no("ltable with weights other than fweight=var")),
        };
        let modes = [
            opts.has("survival", 1),
            opts.has("failure", 1),
            opts.has("hazard", 3),
        ];
        if modes.iter().filter(|m| **m).count() > 1 {
            return Err(no("ltable: one of survival, failure and hazard"));
        }
        let mode = if modes[1] {
            "f"
        } else if modes[2] {
            "h"
        } else {
            "s"
        };
        let (width, cuts) = match opts.arg("intervals", 2) {
            None => ("1".to_string(), "NULL".to_string()),
            Some(a) => {
                let v: Result<Vec<f64>, _> = a
                    .split([' ', ','])
                    .filter(|w| !w.is_empty())
                    .map(str::parse::<f64>)
                    .collect();
                let v = v.map_err(|_| no(format!("ltable, intervals({a}): plain numbers")))?;
                match v.as_slice() {
                    [w] if *w > 0.0 => (r_number(*w), "NULL".to_string()),
                    vs if vs.len() >= 2 && increasing(vs) => (
                        "1".to_string(),
                        format!(
                            "c({})",
                            vs.iter()
                                .map(|x| r_number(*x))
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                    ),
                    _ => return Err(no(format!("ltable, intervals({a}): a width or cut points"))),
                }
            }
        };
        let level = Self::st_level(opts)?;
        let rows = self.rows_for(c, out)?;
        for h in [
            "stata_survival",
            "stata_stptime",
            "stata_fmt",
            "stata_returned",
        ] {
            self.helpers.insert(h);
        }
        out.line(format!(
            "stata_r <- stata_ltable({rows}, {}, fail = {}, w = {}, by = {}, width = {width}, cuts = {cuts}, mode = {}, noadjust = {}, level = {}, test = {}, show = {})",
            r_text(time),
            fail.map_or("NULL".to_string(), r_text),
            wt.as_deref().map_or("NULL".to_string(), r_text),
            by.as_deref().map_or("NULL".to_string(), r_text),
            r_text(mode),
            super::stats::r_logical(opts.has("noadjust", 3)),
            r_number(level),
            super::stats::r_logical(opts.has("test", 2)),
            super::stats::r_logical(!self.quiet(c))
        ));
        if opts.has("test", 2) && by.is_some() {
            self.r_new = Some((R_STTEST, "ltable".into()));
        }
        Ok(())
    }
}

/// The graphs: `sts graph` and `stcurve` (after `streg`), drawn by ggplot2.
pub(super) const STGRAPH_HELPER: &str = r##"stata_stsgraph <- function(d, by = NULL, mode = "s", ci = FALSE, level = 95) {
  r <- stata_sv_rows(d)
  stata_sv_needs_zero_entry(r, "sts graph")
  groups <- stata_sv_groups(d, by)
  rows <- lapply(groups, function(g) {
    km <- stata_sv_km(r$t[g$idx], r$d[g$idx], level)
    v <- switch(mode, s = km$S, f = 1 - km$S, na = km$na)
    lo <- switch(mode, s = km$lb, f = 1 - km$ub, na = km$nalb)
    hi <- switch(mode, s = km$ub, f = 1 - km$lb, na = km$naub)
    data.frame(time = c(0, km$time), v = c(if (mode == "s") 1 else 0, v), lo = c(NA, lo), hi = c(NA, hi),
               group = if (is.null(g$label)) "all" else paste0(by, "=", g$label))
  })
  all <- do.call(rbind, rows)
  ylab <- c(s = "Kaplan–Meier survival estimate", f = "Kaplan–Meier failure estimate", na = "Nelson–Aalen cumulative hazard estimate")[[mode]]
  p <- ggplot2::ggplot(all, ggplot2::aes(x = time, y = v, colour = group)) +
    ggplot2::geom_step() +
    ggplot2::labs(x = "analysis time", title = ylab, y = NULL, colour = NULL)
  if (ci) p <- p + ggplot2::geom_step(ggplot2::aes(y = lo), linetype = "dashed", na.rm = TRUE) + ggplot2::geom_step(ggplot2::aes(y = hi), linetype = "dashed", na.rm = TRUE)
  if (mode != "na") p <- p + ggplot2::coord_cartesian(ylim = c(0, 1))
  if (is.null(by)) p <- p + ggplot2::theme(legend.position = "none")
  p
}
stata_stcurve <- function(d, mode = "s", ats = list()) {
  e <- if (exists("stata_e")) stata_e else NULL
  if (is.null(e) || !identical(e$cmd2, "streg")) stop("last estimates not found (Stata: r(301))", call. = FALSE)
  m <- e$stata_streg
  num <- stata_sv_num
  sub <- d[stata_esample, , drop = FALSE]
  t <- num(sub[["_t"]])
  grid <- seq(min(t), max(t), length.out = 101)
  xs <- m$xs
  k <- m$k
  th <- m$theta
  means <- vapply(xs, function(v) mean(num(sub[[v]])), 0)
  if (!length(ats)) ats <- list(list(label = "", vals = numeric(0)))
  rows <- lapply(ats, function(a) {
    x <- means
    for (nm in names(a$vals)) {
      if (!(nm %in% xs)) stop(nm, " is not a covariate of the model (Stata: r(111))", call. = FALSE)
      x[[nm]] <- a$vals[[nm]]
    }
    xb <- sum(th[seq_along(xs)] * x) + th[k]
    if (m$dist == "gompertz") {
      gam <- th[k + 1]
      H <- exp(xb) * expm1(gam * grid) / gam
      S <- exp(-H)
      h <- exp(xb + gam * grid)
    } else {
      sg <- if (m$dist == "exponential") 1 else exp(th[k + 1])
      fam <- stata_sl_fam(m$dist)
      z <- (log(grid) - xb) / sg
      lS <- fam$ls(z)
      H <- -lS
      S <- exp(lS)
      h <- exp(fam$lf(z) - lS) / (sg * grid)
    }
    v <- switch(mode, s = S, f = 1 - S, h = h, c = H)
    data.frame(time = grid, v = v, group = a$label)
  })
  all <- do.call(rbind, rows)
  ylab <- c(s = "Survival", f = "Failure", h = "Hazard function", c = "Cumulative hazard")[[mode]]
  ttl <- paste0(c(exponential = "Exponential", weibull = "Weibull", lognormal = "Lognormal", loglogistic = "Loglogistic", gompertz = "Gompertz")[[m$dist]], " regression")
  p <- ggplot2::ggplot(all, ggplot2::aes(x = time, y = v, colour = group)) + ggplot2::geom_line() +
    ggplot2::labs(x = "Analysis time", y = ylab, title = ttl, colour = NULL)
  if (length(ats) < 2) p <- p + ggplot2::theme(legend.position = "none")
  p
}"##;

impl Session {
    /// `sts graph`: the Kaplan-Meier (or failure, or Nelson-Aalen) step plot.
    pub(super) fn sts_graph(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[
            ("by", 2),
            ("failure", 1),
            ("survival", 1),
            ("na", 2),
            ("ci", 2),
            ("level", 3),
            ("noshow", 4),
            ("name", 4),
            ("saving", 4),
            ("nodraw", 6),
        ]) {
            return Err(no(format!(
                "sts graph with the option {o} is not translated yet"
            )));
        }
        let by = Self::st_by(opts, "sts graph")?;
        let mode = if opts.has("failure", 1) {
            "f"
        } else if opts.has("na", 2) {
            "na"
        } else {
            "s"
        };
        let level = Self::st_level(opts)?;
        let Some(_st) = self.st_declared(out) else {
            return Ok(());
        };
        let src = self.st_rows(c, out)?;
        for h in ["stata_survival", "stata_stgraph", "stata_fmt"] {
            self.helpers.insert(h);
        }
        out.package("ggplot2");
        out.line(format!(
            "stata_graph <- stata_stsgraph({src}, by = {}, mode = {}, ci = {}, level = {})",
            by.as_deref().map_or("NULL".to_string(), r_text),
            r_text(mode),
            super::stats::r_logical(opts.has("ci", 2)),
            r_number(level)
        ));
        self.keep_graph(opts, out)?;
        if !self.quiet(c) && !opts.has("nodraw", 6) {
            out.line("print(stata_graph)");
        }
        out.caveat("drawn by ggplot2, so its look is R's");
        self.graph_drawn = true;
        Ok(())
    }

    /// `stcurve` after `streg`: survivor, failure, hazard or cumulative hazard
    /// at the covariates' means, or at the values `at()` gives.
    pub(super) fn stcurve(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "stcurve")?;
        let is_at = |n: &str| {
            n.len() >= 2
                && n.starts_with("at")
                && n.get(2..)
                    .is_some_and(|d| d.chars().all(|c| c.is_ascii_digit()))
        };
        for o in opts.0 {
            let kind = ["survival", "failure", "hazard", "cumhaz"]
                .iter()
                .any(|k| !o.name.is_empty() && k.starts_with(o.name.as_str()));
            let known =
                kind || o.is("name", 4) || o.is("saving", 4) || o.is("nodraw", 6) || is_at(&o.name);
            if !known {
                return Err(no(format!(
                    "stcurve with the option {} is not translated yet",
                    o.name
                )));
            }
        }
        if !self.streg_active() {
            return Err(no(
                "stcurve is translated after streg (after stcox, the baseline curves are not)",
            ));
        }
        let mode = if opts.0.iter().any(|o| o.is("failure", 1)) {
            "f"
        } else if opts.0.iter().any(|o| o.is("hazard", 1)) {
            "h"
        } else if opts.0.iter().any(|o| o.is("cumhaz", 1)) {
            "c"
        } else {
            "s"
        };
        let mut ats: Vec<String> = Vec::new();
        for o in opts.0.iter().filter(|o| is_at(&o.name)) {
            let a = o.arg();
            let mut vals: Vec<String> = Vec::new();
            for w in a.split_whitespace() {
                let Some((v, x)) = w.split_once('=') else {
                    return Err(no(format!("stcurve, {}({a}): var=value pairs", o.name)));
                };
                let x: f64 = x
                    .parse()
                    .map_err(|_| no(format!("stcurve, {}({a}): values must be numbers", o.name)))?;
                if !is_name(v) {
                    return Err(no(format!(
                        "stcurve, {}({a}): {v} is not a variable name",
                        o.name
                    )));
                }
                vals.push(format!("{v} = {}", r_number(x)));
            }
            ats.push(format!(
                "list(label = {}, vals = c({}))",
                r_text(a.trim()),
                vals.join(", ")
            ));
        }
        let d = self.data()?;
        for h in [
            "stata_survival",
            "stata_streg",
            "stata_stgraph",
            "stata_fmt",
        ] {
            self.helpers.insert(h);
        }
        out.package("ggplot2");
        out.line(format!(
            "stata_graph <- stata_stcurve({d}, mode = {}, ats = list({}))",
            r_text(mode),
            ats.join(", ")
        ));
        self.keep_graph(opts, out)?;
        if !self.quiet(c) && !opts.0.iter().any(|o| o.is("nodraw", 6)) {
            out.line("print(stata_graph)");
        }
        out.caveat("drawn by ggplot2, so its look is R's");
        self.graph_drawn = true;
        Ok(())
    }
}
