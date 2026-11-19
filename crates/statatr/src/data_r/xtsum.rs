//! Batch H of the StatCraft port, panel and time-series summaries as Stata 19
//! does them: `xtdescribe`, `xtsum`, `xttab`, `tsreport` and `tsappend`
//! (`plan/stata/xtdescribe.md` and the notes beside it). They read what
//! `xtset` and `tsset` left (`ts.rs`, which this file only reads).

use super::*;

fn stop(out: &mut Out, msg: &str) {
    out.line(format!("stop({}, call. = FALSE)", r_text(msg)));
}

/// The pieces every command here shares: a number as `display` writes it,
/// a time in its format, and the plain numbers of a column.
pub(super) const XT_COMMON_HELPER: &str = r##"stata_xt_num <- function(x) {
  u <- unclass(x)
  attributes(u) <- NULL
  if (is.numeric(u)) as.numeric(u) else rep(NA_real_, length(x))
}
stata_xt_g8 <- function(v) {
  if (is.na(v)) return(".")
  sub("^(-?)0\\.", "\\1.", sprintf("%.8g", v))
}
stata_xt_text <- function(v, fmt) {
  if (is.na(v)) return(".")
  if (!is.null(fmt) && grepl("^%-?(t|d)", fmt)) return(stata_fmt(v, fmt, trim = TRUE))
  stata_xt_g8(v)
}
stata_xt_fmt_of <- function(d, v, fmts) {
  f <- attr(d[[v]], "format.stata", exact = TRUE)
  if (is.null(f) && v %in% names(fmts)) f <- fmts[[v]]
  f
}
stata_xt_pad <- function(s, w) {
  g <- w - nchar(s, type = "chars")
  ifelse(g > 0, paste0(strrep(" ", pmax(g, 0)), s), s)
}"##;

/// `xtdescribe`: who is in the panel, when, and the patterns of presence.
///
/// The panel's values and the times are shown as the first, the second, `...`
/// and the last (a missing second shows `.`); `T` counts a missing time as one
/// more value, and the rows with a missing panel are left out. A pattern is a
/// character per period of the span (`.` absent, a digit the rows there, `X`
/// for 10 or more), the most frequent first and ties by the text of the
/// pattern (`.` before a digit before `X`). A span longer than the pattern
/// width allows (`width()`, 100) is shown with one column for each
/// `floor((span - 1) / (width - 1))` periods, a digit counting the periods
/// present in it, and a note under the table. r() is `summarize T_i, detail`.
pub(super) const XTDESCRIBE_HELPER: &str = r##"stata_xtdescribe <- function(d, panel, time, delta = 1, patterns = 9, width = 100,
                             fmts = character(0), show = TRUE) {
  id <- stata_xt_num(d[[panel]])
  tv <- stata_xt_num(d[[time]])
  keep <- !is.na(id)
  id <- id[keep]
  tv <- tv[keep]
  tu <- sort(unique(tv[!is.na(tv)]))
  if (length(tu) < 2) stop(time, " must have multiple distinct nonmissing values (Stata: r(459))", call. = FALSE)
  tf <- stata_xt_fmt_of(d, time, fmts)
  pf <- stata_xt_fmt_of(d, panel, fmts)
  if (!is.null(tf) && grepl("^%-?t[cC]", tf)) stop("xtdescribe with a %tc or %tC time is not translated yet", call. = FALSE)
  unit <- "unit"
  if (!is.null(tf) && grepl("^%-?(t?d|t[wmqhy])", tf)) {
    unit <- c(d = "day", w = "week", m = "month", q = "quarter", h = "halfyear", y = "year")[[substr(sub("^%-?t?", "", tf), 1, 1)]]
  }
  span <- as.integer(round((tu[length(tu)] - tu[1]) / delta)) + 1L
  ids <- sort(unique(id))
  n <- length(ids)
  if (as.numeric(n) * span > 1e8) stop("xtdescribe of a panel this large is not translated", call. = FALSE)
  gi <- match(id, ids)
  Ti <- tabulate(gi, n)
  uq <- !anyDuplicated(data.frame(gi, tv))
  nT <- length(tu) + as.integer(anyNA(tv))
  k <- as.integer(round((tv - tu[1]) / delta)) + 1L
  ok <- !is.na(k) & k >= 1L & k <= span
  cnt <- matrix(tabulate((gi[ok] - 1L) * span + k[ok], n * span), nrow = span)
  ch <- function(m) ifelse(m == 0, ".", ifelse(m > 9, "X", as.character(m)))
  f <- max(1L, as.integer(floor((span - 1) / (width - 1))))
  g <- ceiling(seq_len(span) / f)
  ncols <- max(g)
  # the patterns as shown (a column counts f periods) are what is counted; the most
  # frequent come first, a tie by the full pattern's text (the least of those merged)
  rawp <- apply(ch(cnt), 2, paste, collapse = "")
  comp <- if (f == 1) rawp else apply(ch(rowsum(cnt, g)), 2, paste, collapse = "")
  o2 <- order(comp, rawp, method = "radix")
  keep1 <- !duplicated(comp[o2])
  up <- comp[o2][keep1]
  tie <- rawp[o2][keep1]
  freq <- tabulate(match(comp, up), length(up))
  o <- order(-freq, tie, method = "radix")
  shown <- up[o]
  freq <- freq[o]
  xrow <- paste(ifelse(as.vector(rowsum(as.numeric(rowSums(cnt) > 0), g)) > 0, "X", "."), collapse = "")
  pct <- c(1, 5, 10, 25, 50, 75, 90, 95, 99)
  P <- unname(stats::quantile(Ti, pct / 100, type = 2))
  m <- mean(Ti)
  dv <- Ti - m
  m2 <- mean(dv^2)
  okm <- n > 1 && m2 > 0
  r <- c(list(N = n, sum_w = n, mean = m, Var = if (n > 1) stats::var(Ti) else NA_real_,
              sd = if (n > 1) stats::sd(Ti) else NA_real_,
              skewness = if (okm) mean(dv^3) / m2^1.5 else NA_real_,
              kurtosis = if (okm) mean(dv^4) / m2^2 else NA_real_,
              sum = sum(Ti), min = min(Ti), max = max(Ti)),
         stats::setNames(as.list(P), paste0("p", pct)))
  if (show) {
    series <- function(vals, fm) {
      s <- vapply(c(vals[1], if (length(vals) >= 2) vals[2] else NA_real_, vals[length(vals)]), fm, "")
      paste0(s[1], ", ", s[2], ", ..., ", s[3])
    }
    head <- function(label, series_text, key, value) {
      cat(formatC(sprintf("%8s:  %s", label, series_text), width = -61), sprintf("%s = %10d", key, value), "\n", sep = "")
    }
    cat("\n")
    head(panel, series(ids, function(v) stata_xt_text(v, pf)), "n", n)
    head(time, series(tu, function(v) stata_xt_text(v, tf)), "T", nT)
    dl <- paste0("Delta(", time, ")")
    cat("           ", dl, " = ", stata_fmt(delta, "%9.0g", trim = TRUE), " ", unit, if (delta != 1) "s", "\n", sep = "")
    cat("           ", formatC(paste0("Span(", time, ")"), width = -nchar(dl)), " = ", span, " periods\n", sep = "")
    cat("           (", panel, "*", time, if (uq) " uniquely identifies each observation)" else " does not uniquely identify observations)", "\n", sep = "")
    a0 <- function(v) sprintf("%.0f", v)
    cat("\nDistribution of T_i:   min      5%     25%       50%       75%     95%     max\n")
    cat(sprintf("%26s%8s%8s%10s%10s%8s%8s", a0(min(Ti)), a0(P[2]), a0(P[4]), a0(P[5]), a0(P[6]), a0(P[8]), a0(max(Ti))), "\n", sep = "")
    cat("\n")
    if (patterns > 0) {
      pw <- if (f > 1) max(floor(span / f + 0.25) + 3, 10) else max(span + 2, 9)
      rule <- paste0(" ", strrep("-", 27), "+", strrep("-", pw))
      cat("     Freq.  Percent    Cum. |  Pattern", if (f > 1) "*", "\n", sep = "")
      cat(rule, "\n", sep = "")
      np <- length(shown)
      cum <- 0
      for (j in seq_len(min(patterns, np))) {
        cum <- cum + freq[j]
        cat(sprintf("%9d%10.2f%8.2f |  %s", freq[j], 100 * freq[j] / n, 100 * cum / n, shown[j]), "\n", sep = "")
      }
      if (np > patterns) {
        rest <- sum(freq[-seq_len(patterns)])
        cat(sprintf("%9d%10.2f%8.2f | (other patterns)", rest, 100 * rest / n, 100), "\n", sep = "")
      }
      cat(rule, "\n", sep = "")
      cat(sprintf("%9d%10.2f%8s |  %s", n, 100, "", xrow), "\n", sep = "")
      if (f > 1) {
        cat(" ", strrep("-", 28 + pw), "\n", sep = "")
        cat(" *Each column represents ", f, " periods.\n", sep = "")
      }
      cat("\n")
    }
  }
  r
}"##;

/// `xtsum`: the overall, between and within variation of each variable.
///
/// Overall is the mean, standard deviation (n - 1), minimum and maximum of
/// the rows with the variable; between those of the panel means (a missing
/// panel value is a panel of its own here); within those of x - panel mean +
/// overall mean, its standard deviation about the overall mean. The count
/// column reads `N`, `n` and `T-bar` (`T` when every panel has as many
/// rows). A string variable shows as no rows. r() is the last variable's.
pub(super) const XTSUM_HELPER: &str = r##"stata_xtsum <- function(d, panel, vars = NULL, show = TRUE) {
  id <- stata_xt_num(d[[panel]])
  if (is.null(vars)) vars <- names(d)
  for (v in vars) if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
  g9 <- function(v) if (is.na(v)) "." else stata_fmt(v, "%9.0g", trim = TRUE)
  g8 <- function(v) if (is.na(v)) "." else stata_fmt(v, "%8.0g", trim = TRUE)
  r <- NULL
  if (show) {
    cat("\nVariable         |      Mean   Std. dev.       Min        Max |    Observations\n")
    cat(strrep("-", 17), "+", strrep("-", 44), "+", strrep("-", 16), "\n", sep = "")
  }
  first <- TRUE
  for (v in vars) {
    x <- if (is.character(d[[v]])) rep(NA_real_, nrow(d)) else stata_xt_num(d[[v]])
    use <- !is.na(x)
    x <- x[use]
    gi <- match(id[use], unique(id[use]))
    N <- length(x)
    n <- if (N) max(gi) else 0L
    sz <- tabulate(gi, n)
    gm <- if (N) as.vector(rowsum(x, gi)) / sz else numeric(0)
    mean_ <- if (N) mean(x) else NA_real_
    sd_ <- if (N > 1) stats::sd(x) else NA_real_
    w <- if (N) x - gm[gi] + mean_ else numeric(0)
    sd_w <- if (N > 1) sqrt(sum((w - mean_)^2) / (N - 1)) else NA_real_
    sd_b <- if (n > 1) stats::sd(gm) else NA_real_
    ext <- function(z, f) if (length(z)) f(z) else NA_real_
    Tbar <- if (n) N / n else NA_real_
    r <- list(max_w = ext(w, max), min_w = ext(w, min), sd_w = sd_w, max_b = ext(gm, max), min_b = ext(gm, min),
              sd_b = sd_b, max = ext(x, max), min = ext(x, min), sd = sd_, mean = mean_, Tbar = Tbar, n = n, N = N)
    if (show) {
      if (!first) cat(strrep(" ", 17), "|", strrep(" ", 44), "|\n", sep = "")
      first <- FALSE
      nm <- stata_tab_abbrev(v, 8)
      bal <- !n || all(sz == sz[1])
      cnt <- function(lab, val) paste0(" | ", stata_xt_pad(lab, 5), " =", stata_xt_pad(g8(val), 8))
      row <- function(label, a, b, cc, dd, right) {
        paste0(label, "|", stata_xt_pad(a, 10), stata_xt_pad(b, 11), stata_xt_pad(cc, 11), stata_xt_pad(dd, 11), right)
      }
      cat(row(sprintf("%-8s overall ", nm), g9(r$mean), g9(r$sd), g9(r$min), g9(r$max), cnt("N", N)), "\n", sep = "")
      cat(row("         between ", "", g9(r$sd_b), g9(r$min_b), g9(r$max_b), cnt("n", n)), "\n", sep = "")
      cat(row("         within  ", "", g9(r$sd_w), g9(r$min_w), g9(r$max_w), cnt(if (bal) "T" else "T-bar", Tbar)), "\n", sep = "")
    }
  }
  if (show) cat("\n")
  r
}"##;

/// `xttab`: a one-way table with overall, between and within counts.
///
/// Overall counts the rows; between the panels that have the value at least
/// once (its percent is of all panels, so the column can add to more than
/// 100); within the average, over those panels, of the share of their rows
/// with the value. The total row's within percent is n over the between
/// total. A value shows as its value label (cut to 8 characters; blank when
/// the variable has labels but not that value) or as `%9.0g` writes it, whatever the variable's format.
/// r(results) holds the table, r(n_panels) the number of panels.
pub(super) const XTTAB_HELPER: &str = r##"stata_xttab <- function(d, panel, var, show = TRUE) {
  if (!var %in% names(d)) stop("variable ", var, " not found (Stata: r(111))", call. = FALSE)
  x0 <- d[[var]]
  if (is.character(x0)) stop(var, " must be numeric (Stata: r(109))", call. = FALSE)
  x <- stata_xt_num(x0)
  id <- stata_xt_num(d[[panel]])
  use <- !is.na(x) & !is.na(id)
  if (!any(use)) stop("no observations (Stata: r(2000))", call. = FALSE)
  x <- x[use]
  id <- id[use]
  N <- length(x)
  vals <- sort(unique(x))
  nv <- length(vals)
  gi <- match(id, unique(id))
  n <- max(gi)
  size <- tabulate(gi, n)
  key <- (match(x, vals) - 1) * n + gi
  rl <- rle(sort(key))
  pv <- (rl$values - 1) %/% n + 1
  pg <- (rl$values - 1) %% n + 1
  pc <- rl$lengths
  overall <- as.vector(rowsum(pc, pv))
  between <- tabulate(pv, nv)
  within <- 100 * as.vector(rowsum(pc / size[pg], pv)) / between
  tb <- sum(between)
  # the values are kept as floats in r(results), though the groups are of the doubles
  res <- cbind(value = c(stata_float(vals), NA_real_), `Overall:Freq` = c(overall, N), `Overall:Percent` = c(100 * overall / N, 100),
               `Between:Freq` = c(between, tb), `Between:Percent` = c(100 * between / n, 100 * tb / n),
               `Within:Percent` = c(within, 100 * n / tb))
  rownames(res) <- paste0("r", seq_len(nv + 1))
  out <- list(n_panels = n, results = res)
  if (show) {
    labs <- attr(x0, "labels", exact = TRUE)
    label_of <- function(v) {
      if (length(labs)) {
        j <- match(v, as.numeric(unclass(labs)))
        return(if (is.na(j)) "" else substr(names(labs)[j], 1, 8))
      }
      stata_fmt(v, "%9.0g", trim = TRUE)
    }
    cat("\n                  Overall             Between            Within\n")
    cat(sprintf("%9s |    Freq.  Percent      Freq.  Percent        Percent", stata_tab_abbrev(var, 9)), "\n", sep = "")
    rule <- paste0(strrep("-", 10), "+", strrep("-", 53))
    cat(rule, "\n", sep = "")
    for (i in seq_len(nv)) {
      cat(sprintf("%9s |%8d%10.2f%10d%10.2f%15.2f", label_of(vals[i]), as.integer(overall[i]), 100 * overall[i] / N,
                  as.integer(between[i]), 100 * between[i] / n, within[i]), "\n", sep = "")
    }
    cat(rule, "\n", sep = "")
    cat(sprintf("%9s |%8d%10.2f%10d%10.2f%15.2f", "Total", as.integer(N), 100, as.integer(tb), 100 * tb / n, 100 * n / tb), "\n", sep = "")
    cat(strrep(" ", 33), "(n = ", n, ")\n", sep = "")
    cat("\n")
  }
  out
}"##;

/// `tsreport`: the span of the data and its gaps, in total, by variable, with
/// the gaps listed (`detail`).
///
/// `d` holds each row's position in the data as `stata_obs_`. A gap is a
/// period missing between two rows of a panel that have the variable; with a
/// panel the end of each panel but the data's last row also counts, unless
/// `panel` is given. The rows are expected in panel and time order.
pub(super) const TSREPORT_HELPER: &str = r##"stata_tsreport <- function(d, time, panel = NULL, delta = 1, vars = NULL, detail = FALSE,
                           casewise = FALSE, by_panel = FALSE, fmts = character(0), show = TRUE) {
  tv <- stata_xt_num(d[[time]])
  pv <- if (is.null(panel)) rep(1, nrow(d)) else stata_xt_num(d[[panel]])
  obs <- d$stata_obs_
  for (v in vars) if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
  for (v in vars) if (is.character(d[[v]])) stop("string variables not allowed in varlist;\n", v, " is a string variable (Stata: r(109))", call. = FALSE)
  tf <- stata_xt_fmt_of(d, time, fmts)
  if (is.null(tf)) tf <- "%12.0g"
  if (grepl("^%-?t[cC]", tf)) stop("tsreport with a %tc or %tC time is not translated yet", call. = FALSE)
  pf <- if (is.null(panel)) NULL else stata_xt_fmt_of(d, panel, fmts)
  if (is.null(pf) && !is.null(panel)) pf <- "%12.0g"
  fw <- function(f) { m <- regmatches(f, regexec("^%-?0?([0-9]+)", f))[[1]]; if (length(m) == 2) as.integer(m[2]) else if (grepl("^%-?t", f)) 9L else 10L }
  tw <- fw(tf)
  tshow <- function(v) if (is.na(v)) "." else stata_fmt(v, tf, trim = TRUE)
  num_of <- function(v) stata_xt_num(d[[v]])
  base <- !is.na(tv) & !is.na(pv)
  ob <- which(base)
  if (length(ob) > 1 && is.unsorted(order(pv[ob], tv[ob], method = "radix"), strictly = TRUE)) {
    stop("tsreport on data that are not sorted by the panel and time variables is not translated yet", call. = FALSE)
  }
  one <- function(use) {
    i <- which(base & use)
    n <- length(i)
    gaps <- list(before = integer(0), after = integer(0), pan = numeric(0), from = numeric(0), to = numeric(0), len = numeric(0))
    add <- function(b, a, p, f, t, l) {
      gaps$before <<- c(gaps$before, b); gaps$after <<- c(gaps$after, a); gaps$pan <<- c(gaps$pan, p)
      gaps$from <<- c(gaps$from, f); gaps$to <<- c(gaps$to, t); gaps$len <<- c(gaps$len, l)
    }
    if (n) {
      for (j in seq_len(n)) {
        a <- i[j]
        b <- if (j < n) i[j + 1] else NA_integer_
        if (!is.na(b) && pv[b] == pv[a]) {
          step <- (tv[b] - tv[a]) / delta
          if (step > 1 + 1e-9) add(obs[a], obs[b], pv[a], tv[a] + delta, tv[b] - delta, round(step) - 1)
        } else if (!is.null(panel) && !by_panel && (!is.na(b) || tv[a] < max(tv[i]))) {
          # a panel's end counts, and so does the last row's when it is before the sample's end
          add(obs[a], NA_integer_, pv[a], tv[a] + delta, NA_real_, NA_real_)
        }
      }
    }
    list(n = n, start = if (n) min(tv[i]) else NA_real_, end = if (n) max(tv[i]) else NA_real_, gaps = gaps)
  }
  tab <- function(g) {
    k <- length(g$before)
    if (!k) return(NULL)
    m <- cbind(ObsBefore = g$before, ObsAfter = g$after)
    if (!is.null(panel)) m <- cbind(m, g$pan)
    m <- cbind(m, StartTime = g$from, EndTime = g$to, Obs = g$len)
    if (!is.null(panel)) colnames(m)[3] <- panel
    rownames(m) <- ifelse(is.na(g$after), as.character(g$before), paste0(g$before, "--", g$after))
    m
  }
  report <- function(rep, label) {
    g <- rep$gaps
    k <- length(g$before)
    cells <- c(tshow(rep$start), tshow(rep$end), as.character(rep$n), as.character(k))
    w <- max(nchar(cells))
    if (nzchar(label)) cat(label, "\n", sep = "")
    if (is.null(panel)) cat("Time variable: ", time, "\n", sep = "")
    else cat("Panel variable: ", panel, "\nTime variable:  ", time, "\n", sep = "")
    cat(strrep("-", 18 + w), "\n", sep = "")
    cat("Starting period = ", stata_xt_pad(cells[1], w), "\nEnding period   = ", stata_xt_pad(cells[2], w),
        "\nNumber of obs   = ", stata_xt_pad(cells[3], w), "\nNumber of gaps  = ", stata_xt_pad(cells[4], w),
        if (!is.null(panel) && !by_panel) " (includes panel changes)", "\n", sep = "")
    if (detail && k > 0) {
      pw <- if (is.null(panel)) 0L else max(11L, fw(pf))
      sw <- if (is.null(panel)) max(11L, tw + 1L) else max(12L, tw + 2L)
      ew <- max(12L, tw + 2L)
      pnm <- if (is.null(panel)) "" else stata_xt_pad(stata_tab_abbrev(panel, 9), pw)
      full <- 14 + pw + sw + ew + 13
      cat("\nGap report\n", strrep("-", full), "\n", sep = "")
      cat("Observations |", pnm, stata_xt_pad("Start", sw), stata_xt_pad("End", ew), stata_xt_pad("N. obs", 13), "\n", sep = "")
      cat(strrep("-", 13), "+", strrep("-", full - 14), "\n", sep = "")
      for (j in seq_len(k)) {
        ob <- if (is.na(g$after[j])) as.character(g$before[j]) else paste0(g$before[j], "–", g$after[j])
        pcell <- if (is.null(panel)) "" else stata_xt_pad(if (is.na(g$pan[j])) "." else stata_fmt(g$pan[j], pf, trim = TRUE), pw)
        cat(stata_xt_pad(ob, 12), " |", pcell, stata_xt_pad(tshow(g$from[j]), sw), stata_xt_pad(tshow(g$to[j]), ew),
            stata_xt_pad(if (is.na(g$len[j])) "." else as.character(g$len[j]), 13), "\n", sep = "")
      }
      cat(strrep("-", full), "\n", sep = "")
    }
    cat("\n")
  }
  keep_text <- function(r) {
    r$panelvar <- if (is.null(panel)) NULL else panel
    r$timevar <- time
    r$tsfmt <- tf
    r
  }
  if (show) cat("\n")
  if (is.null(vars) || (casewise && length(vars) > 1)) {
    use <- rep(TRUE, nrow(d))
    for (v in vars) use <- use & !is.na(num_of(v))
    rep1 <- one(use)
    if (show) report(rep1, if (length(vars) > 1) paste0("Variables: ", paste(vars, collapse = " ")) else "")
    r <- list(N_gaps = length(rep1$gaps$before), N_obs = rep1$n, start = rep1$start, end = rep1$end)
    r <- keep_text(r)
    t1 <- tab(rep1$gaps)
    if (detail && !is.null(t1)) r$table <- t1
    return(r)
  }
  reps <- lapply(vars, function(v) one(!is.na(num_of(v))))
  if (show) {
    if (detail) {
      for (j in seq_along(vars)) report(reps[[j]], paste0("Variable: ", vars[j]))
    } else {
      sw <- max(11L, tw + 1L)
      ew <- max(12L, tw + 2L)
      tot <- sw + ew + 10 + 9
      cat("Gap summary report\n", strrep("-", 14 + tot), "\n", sep = "")
      cat(strrep(" ", 13), "|", strrep(" ", tot - 11), "Number of  \n", sep = "")
      cat("    Variable |", stata_xt_pad("Start", sw), stata_xt_pad("End", ew), "       obs     gaps\n", sep = "")
      cat(strrep("-", 13), "+", strrep("-", tot), "\n", sep = "")
      for (j in seq_along(vars)) {
        cat(stata_xt_pad(stata_tab_abbrev(vars[j], 12), 12), " |", stata_xt_pad(tshow(reps[[j]]$start), sw), stata_xt_pad(tshow(reps[[j]]$end), ew),
            stata_xt_pad(as.character(reps[[j]]$n), 10), stata_xt_pad(as.character(length(reps[[j]]$gaps$before)), 9), "\n", sep = "")
      }
      cat(strrep("-", 14 + tot), "\n\n", sep = "")
    }
  }
  r <- list()
  for (j in rev(seq_along(vars))) {
    r[[paste0("N_gaps", j)]] <- length(reps[[j]]$gaps$before)
    r[[paste0("N_obs", j)]] <- reps[[j]]$n
    r[[paste0("start", j)]] <- reps[[j]]$start
    r[[paste0("end", j)]] <- reps[[j]]$end
  }
  r <- keep_text(r)
  for (j in rev(seq_along(vars))) r[[paste0("var", j)]] <- vars[j]
  if (detail) for (j in rev(seq_along(vars))) {
    t1 <- tab(reps[[j]]$gaps)
    if (!is.null(t1)) r[[paste0("table", j)]] <- t1
  }
  r
}"##;

/// `tsappend`: new periods after each panel's last (`add(#)`, or up to a date
/// with `last()` and `tsfmt()`), then the periods missing inside each panel
/// are filled too, as `tsfill` does, so the data have no gaps. The other
/// variables are missing in the new rows; the data end in panel and time
/// order. r(add) is the `add()` number, or with `last()` the periods added
/// after the panels' ends.
pub(super) const TSAPPEND_HELPER: &str = r##"stata_tsappend <- function(d, time, panel = NULL, delta = 1, add = NULL, last = NULL, show = TRUE) {
  tv <- stata_xt_num(d[[time]])
  if (anyNA(tv)) stop("the time variable may not be missing (Stata: r(198))", call. = FALSE)
  pv <- if (is.null(panel)) rep(0, nrow(d)) else stata_xt_num(d[[panel]])
  if (anyNA(pv)) stop("tsappend with a missing panel is not translated yet", call. = FALSE)
  if (!nrow(d)) return(list(d = d, add = 0))
  if (length(tv) > 1 && is.unsorted(order(pv, tv, method = "radix"), strictly = TRUE)) {
    stop("tsappend on data that are not sorted by the panel and time variables is not translated yet", call. = FALSE)
  }
  ups <- unique(pv)
  lo <- vapply(ups, function(p) min(tv[pv == p]), 0)
  hi <- vapply(ups, function(p) max(tv[pv == p]), 0)
  add_p <- numeric(0)
  add_t <- numeric(0)
  appended <- 0
  new_end <- hi
  for (j in seq_along(ups)) {
    k <- if (!is.null(add)) add else if (last >= hi[j] + delta - 1e-9 * delta) floor((last - hi[j]) / delta + 1e-9) else 0
    new_end[j] <- hi[j] + k * delta
    appended <- appended + k
  }
  if (!is.null(last) && show) {
    txt <- if (last == round(last)) sprintf("%.0f", last) else stata_xt_g8(last)
    if (is.null(panel)) {
      if (max(hi) >= last) cat(txt, " is already in the dataset\n", sep = "")
    } else if (any(hi >= last)) cat("Some panels have observations later than ", txt, "\n", sep = "")
  }
  for (j in seq_along(ups)) {
    have <- round((tv[pv == ups[j]] - lo[j]) / delta)
    miss <- setdiff(seq(0, round((new_end[j] - lo[j]) / delta)), have)
    add_p <- c(add_p, rep(ups[j], length(miss)))
    add_t <- c(add_t, lo[j] + miss * delta)
  }
  if (!length(add_t)) return(list(d = d, add = if (!is.null(add)) add else appended))
  new <- d[rep(NA_integer_, length(add_t)), , drop = FALSE]
  for (v in names(new)) if (is.character(new[[v]])) new[[v]][] <- ""
  x <- new[[time]]
  x[] <- add_t
  new[[time]] <- x
  if (!is.null(panel)) {
    x <- new[[panel]]
    x[] <- add_p
    new[[panel]] <- x
  }
  out <- dplyr::bind_rows(d, new)
  out <- out[order(c(pv, add_p), c(tv, add_t), method = "radix"), , drop = FALSE]
  rownames(out) <- NULL
  list(d = out, add = if (!is.null(add)) add else appended)
}"##;

const R_XTSUM: &[&str] = &[
    "max_w", "min_w", "sd_w", "max_b", "min_b", "sd_b", "max", "min", "sd", "mean", "Tbar", "n",
    "N",
];
const R_XTTAB: &[&str] = &["n_panels", "results"];
const R_TSAPPEND: &[&str] = &["add"];
const R_TSREPORT: &[&str] = &[
    "N_gaps", "N_obs", "start", "end", "panelvar", "timevar", "tsfmt", "table",
];

/// What the names tsreport leaves with a varlist are: `Some(0)` a number
/// (`N_gaps2`), `Some(1)` text (`var2`), `Some(2)` a matrix (`table2`).
pub(super) fn tsreport_name(name: &str) -> Option<u8> {
    let number = |p: &str| {
        name.strip_prefix(p)
            .is_some_and(|k| !k.is_empty() && k.chars().all(|c| c.is_ascii_digit()))
    };
    if ["N_gaps", "N_obs", "start", "end"]
        .iter()
        .any(|p| number(p))
    {
        Some(0)
    } else if number("var") {
        Some(1)
    } else if number("table") {
        Some(2)
    } else {
        None
    }
}

/// The R of `c(a = "%8.0g", ...)`: the display format Stata gave each of these
/// variables when `generate` made them, for those whose storage is known.
fn formats(s: &Session, names: &[&str]) -> String {
    let list: Vec<String> = names
        .iter()
        .filter_map(|v| {
            let f = s.storage.get(*v)?.default_format()?;
            Some(format!("{} = {}", r_text(v), r_text(f)))
        })
        .collect();
    if list.is_empty() {
        String::new()
    } else {
        format!(", fmts = c({})", list.join(", "))
    }
}

impl Session {
    /// What `xtset` and `tsset` left, or Stata's own stop when they were not
    /// run: the panel variable and, when `time`, the time variable.
    fn panel_state(
        &self,
        need_time: bool,
        out: &mut Out,
        msg_none: &str,
        msg_no_time: &str,
        code: u16,
    ) -> Result<Option<(Option<String>, Option<String>)>, Untranslated> {
        if self.ts_unknown {
            return Err(no(
                "the data were read from a file whose xtset or tsset settings are not known here; \
                 set them again with xtset",
            ));
        }
        if need_time && self.panel_time.is_none() {
            let msg = if self.panel.is_none() {
                msg_none
            } else {
                msg_no_time
            };
            stop(out, &format!("{msg} (Stata: r({code}))"));
            return Ok(None);
        }
        if !need_time && self.panel.is_none() {
            stop(out, &format!("{msg_none} (Stata: r({code}))"));
            return Ok(None);
        }
        Ok(Some((self.panel.clone(), self.panel_time.clone())))
    }

    /// Whether a command that has no varlist, weights or `if`/`in` got one.
    fn refuse_extras(&self, c: &Command, out: &mut Out, what: &str, if_in: bool) -> bool {
        if c.parts.weight.is_some() {
            stop(out, "weights not allowed (Stata: r(101))");
            return true;
        }
        if if_in && c.parts.if_exp.is_some() {
            stop(out, "if not allowed (Stata: r(101))");
            return true;
        }
        if if_in && c.parts.in_range.is_some() {
            stop(out, "in range not allowed (Stata: r(101))");
            return true;
        }
        if !c.parts.main.trim().is_empty() && what == "varlist" {
            stop(out, "varlist not allowed (Stata: r(101))");
            return true;
        }
        false
    }

    /// `i(var)` or `t(var)` naming the variable xtset already holds is no
    /// change; naming another one would reset the panel as the old `iis` did.
    fn same_setting(
        &self,
        opts: &Opts,
        name: &str,
        current: &Option<String>,
    ) -> Result<(), Untranslated> {
        if let Some(v) = opts.arg(name, 1) {
            if current.as_deref() != Some(v.trim()) {
                return Err(no(format!(
                    "{name}({}) other than the variable xtset holds is not translated yet (Stata also resets the panel settings)",
                    v.trim()
                )));
            }
        }
        Ok(())
    }

    /// `xtdescribe [if] [in] [, patterns(#) width(#) i() t()]`.
    pub(super) fn xtdescribe(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "xtdescribe")?;
        if let Some(o) = opts.other(&[("patterns", 1), ("width", 1), ("i", 1), ("t", 1)]) {
            stop(out, &format!("option {o} not allowed (Stata: r(198))"));
            return Ok(());
        }
        if self.refuse_extras(c, out, "varlist", false) {
            return Ok(());
        }
        let patterns = match opts.arg("patterns", 1) {
            None => 9,
            Some(p) => match p.trim().parse::<i64>() {
                Ok(v) if v >= 0 => v,
                Ok(_) => {
                    stop(out, "invalid syntax (Stata: r(198))");
                    return Ok(());
                }
                Err(_) => {
                    stop(
                        out,
                        "option patterns() incorrectly specified (Stata: r(198))",
                    );
                    return Ok(());
                }
            },
        };
        let width = match opts.arg("width", 1) {
            None => 100,
            Some(w) => match w.trim().parse::<i64>() {
                Ok(v) if v >= 3 => v,
                _ => {
                    return Err(no(format!(
                        "xtdescribe, width({}): a width below 3 is not translated",
                        w.trim()
                    )))
                }
            },
        };
        let (panel, time) = (self.panel.clone(), self.panel_time.clone());
        self.same_setting(opts, "i", &panel)?;
        self.same_setting(opts, "t", &time)?;
        let Some((Some(panel), Some(time))) = self.panel_state(
            true,
            out,
            "must specify panelvar and timevar; use xtset",
            "must specify timevar; use xtset",
            459,
        )?
        else {
            return Ok(());
        };
        let rows = self.rows_for(c, out)?;
        for h in ["stata_xtdescribe", "stata_xt_common", "stata_fmt"] {
            self.helpers.insert(h);
        }
        out.line(format!(
            "stata_r <- stata_xtdescribe({rows}, {}, {}, delta = {}, patterns = {patterns}, width = {width}{}{})",
            r_text(&panel),
            r_text(&time),
            self.ts_delta.clone().unwrap_or_else(|| "1".to_string()),
            formats(self, &[&panel, &time]),
            if self.quiet(c) { ", show = FALSE" } else { "" }
        ));
        self.r_new = Some((returned::R_SUMMARIZE_DETAIL, "xtdescribe".into()));
        Ok(())
    }

    /// `xtsum [varlist] [if] [in] [, i(var)]`.
    pub(super) fn xtsum(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "xtsum")?;
        if let Some(o) = opts.other(&[("i", 1)]) {
            stop(out, &format!("option {o} not allowed (Stata: r(198))"));
            return Ok(());
        }
        if self.refuse_extras(c, out, "", false) {
            return Ok(());
        }
        let vars: Vec<&str> = c.parts.main.split_whitespace().collect();
        if let Some(v) = vars.iter().find(|v| !is_name(v)) {
            return Err(no(format!(
                "xtsum {v}: only plain variable names are translated"
            )));
        }
        let panel = self.panel.clone();
        self.same_setting(opts, "i", &panel)?;
        let Some((Some(panel), _)) =
            self.panel_state(false, out, "must specify panelvar; use xtset", "", 459)?
        else {
            return Ok(());
        };
        let rows = self.rows_for(c, out)?;
        for h in [
            "stata_xtsum",
            "stata_xt_common",
            "stata_fmt",
            "stata_tables",
        ] {
            self.helpers.insert(h);
        }
        let vpart = if vars.is_empty() {
            String::new()
        } else {
            format!(
                ", c({})",
                vars.iter()
                    .map(|v| r_text(v))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        out.line(format!(
            "stata_r <- stata_xtsum({rows}, {}{vpart}{})",
            r_text(&panel),
            if self.quiet(c) { ", show = FALSE" } else { "" }
        ));
        self.r_new = Some((R_XTSUM, "xtsum".into()));
        Ok(())
    }

    /// `xttab varname [if] [in] [, i(var)]`.
    pub(super) fn xttab(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "xttab")?;
        if let Some(o) = opts.other(&[("i", 1)]) {
            stop(out, &format!("option {o} not allowed (Stata: r(198))"));
            return Ok(());
        }
        if self.refuse_extras(c, out, "", false) {
            return Ok(());
        }
        let vars: Vec<&str> = c.parts.main.split_whitespace().collect();
        let [var] = vars.as_slice() else {
            if vars.is_empty() {
                stop(out, "varlist required (Stata: r(100))");
            } else {
                stop(out, "too many variables specified (Stata: r(103))");
            }
            return Ok(());
        };
        if !is_name(var) {
            return Err(no(format!(
                "xttab {var}: only a plain variable name is translated"
            )));
        }
        let panel = self.panel.clone();
        self.same_setting(opts, "i", &panel)?;
        let Some((Some(panel), _)) =
            self.panel_state(false, out, "must specify panelvar; use xtset", "", 459)?
        else {
            return Ok(());
        };
        let rows = self.rows_for(c, out)?;
        for h in [
            "stata_xttab",
            "stata_xt_common",
            "stata_fmt",
            "stata_tables",
            "stata_float",
        ] {
            self.helpers.insert(h);
        }
        out.line(format!(
            "stata_r <- stata_xttab({rows}, {}, {}{})",
            r_text(&panel),
            r_text(var),
            if self.quiet(c) { ", show = FALSE" } else { "" }
        ));
        self.r_new = Some((R_XTTAB, "xttab".into()));
        Ok(())
    }

    /// `tsreport [varlist] [if] [in] [, detail casewise panel]`.
    pub(super) fn tsreport(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "tsreport")?;
        if let Some(o) = opts.other(&[("detail", 1), ("casewise", 1), ("panel", 1)]) {
            stop(out, &format!("option {o} not allowed (Stata: r(198))"));
            return Ok(());
        }
        if self.refuse_extras(c, out, "", false) {
            return Ok(());
        }
        let vars: Vec<&str> = c.parts.main.split_whitespace().collect();
        if let Some(v) = vars.iter().find(|v| !is_name(v)) {
            return Err(no(format!(
                "tsreport {v}: only plain variable names are translated"
            )));
        }
        let casewise = opts.has("casewise", 1);
        if casewise && vars.len() < 2 {
            stop(
                out,
                "casewise can only be specified with two or more variables (Stata: r(198))",
            );
            return Ok(());
        }
        let Some((panel, Some(time))) = self.panel_state(
            true,
            out,
            "time variable not set, use tsset varname ...",
            "time variable not set, use tsset varname ...",
            111,
        )?
        else {
            return Ok(());
        };
        let d = self.data()?;
        out.package("dplyr");
        let numbered = format!("dplyr::mutate({d}, stata_obs_ = dplyr::row_number())");
        let rows = match self.condition(c, out)? {
            Some(cond) => format!("dplyr::filter({numbered}, {cond})"),
            None => numbered,
        };
        for h in [
            "stata_tsreport",
            "stata_xt_common",
            "stata_fmt",
            "stata_tables",
        ] {
            self.helpers.insert(h);
        }
        let mut names: Vec<&str> = vec![time.as_str()];
        if let Some(p) = &panel {
            names.push(p);
        }
        let vpart = if vars.is_empty() {
            String::new()
        } else {
            format!(
                ", vars = c({})",
                vars.iter()
                    .map(|v| r_text(v))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        let flag = |on: bool, name: &str| {
            if on {
                format!(", {name} = TRUE")
            } else {
                String::new()
            }
        };
        out.line(format!(
            "stata_r <- stata_tsreport({rows}, {}{}, delta = {}{vpart}{}{}{}{}{})",
            r_text(&time),
            panel
                .as_ref()
                .map(|p| format!(", panel = {}", r_text(p)))
                .unwrap_or_default(),
            self.ts_delta.clone().unwrap_or_else(|| "1".to_string()),
            flag(opts.has("detail", 1), "detail"),
            flag(casewise, "casewise"),
            flag(opts.has("panel", 1), "by_panel"),
            formats(self, &names),
            if self.quiet(c) { ", show = FALSE" } else { "" }
        ));
        self.r_new = Some((R_TSREPORT, "tsreport".into()));
        Ok(())
    }

    /// `tsappend, add(#)` or `tsappend, last(date) tsfmt(function)`.
    pub(super) fn tsappend(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "tsappend")?;
        if let Some(o) = opts.other(&[("add", 1), ("last", 1), ("tsfmt", 1)]) {
            stop(out, &format!("option {o} not allowed (Stata: r(198))"));
            return Ok(());
        }
        if self.refuse_extras(c, out, "varlist", true) {
            return Ok(());
        }
        let (add, last, fmt) = (
            opts.arg("add", 1),
            opts.arg("last", 1),
            opts.arg("tsfmt", 1),
        );
        if add.is_some() && (last.is_some() || fmt.is_some()) {
            stop(out, "specify add() or tsfmt(), not both (Stata: r(198))");
            return Ok(());
        }
        if add.is_none() && last.is_none() && fmt.is_none() {
            stop(
                out,
                "you must specify either add() or last() (Stata: r(198))",
            );
            return Ok(());
        }
        if add.is_none() && last.is_none() {
            stop(
                out,
                "you must specify either add() or last() (Stata: r(198))",
            );
            return Ok(());
        }
        if last.is_some() && fmt.is_none() {
            stop(out, "tsfmt() must be specified with last() (Stata: r(198))");
            return Ok(());
        }
        let how = if let Some(a) = &add {
            match a.trim().parse::<f64>() {
                Ok(v) if v >= 1.0 && v.fract() == 0.0 && v < 1e9 => {
                    format!("add = {}", r_number(v))
                }
                _ => {
                    stop(out, "add() must specify a positive integer (Stata: r(198))");
                    return Ok(());
                }
            }
        } else {
            let (Some(date), Some(f)) = (&last, &fmt) else {
                return Ok(());
            };
            let f = f.trim();
            if !matches!(f, "td" | "tw" | "tm" | "tq" | "th") {
                return Err(no(format!(
                    "tsappend, tsfmt({f}): only td, tw, tm, tq and th are translated"
                )));
            }
            let (value, _) = self.value(&format!("{f}({})", date.trim()), out)?;
            format!("last = {value}")
        };
        let Some((panel, Some(time))) = self.panel_state(
            true,
            out,
            "time variable not set, use tsset varname ...",
            "time variable not set, use tsset varname ...",
            111,
        )?
        else {
            return Ok(());
        };
        let d = self.data()?;
        for h in ["stata_tsappend", "stata_xt_common", "stata_fmt"] {
            self.helpers.insert(h);
        }
        out.package("dplyr");
        out.line(format!(
            "stata_ta <- stata_tsappend({d}, {}{}, delta = {}, {how}{})",
            r_text(&time),
            panel
                .as_ref()
                .map(|p| format!(", panel = {}", r_text(p)))
                .unwrap_or_default(),
            self.ts_delta.clone().unwrap_or_else(|| "1".to_string()),
            if self.quiet(c) { ", show = FALSE" } else { "" }
        ));
        out.line(format!("{d} <- stata_ta$d"));
        out.line("stata_r <- list(add = stata_ta$add)");
        out.line("rm(stata_ta)");
        self.rows_unknown();
        self.r_new = Some((R_TSAPPEND, "tsappend".into()));
        Ok(())
    }
}
