//! Stata's own tables for `summarize`, `tabulate` and `regress` (P2B.7).
//!
//! Each layout was read off Stata 19's output (never its program files):
//! the widths and formats of every column, where the separators fall, how a
//! long name is shortened or split, how two-way tables wrap at a linesize of
//! 80, and how `regress` groups a factor's levels, two factors' cells and
//! value labels (shortened past 11 characters). A term omitted for
//! collinearity is a row `0  (omitted)` (probe 288), an empty cell
//! `0  (empty)`. A table whose shape was not checked against Stata (an
//! omitted term in an interaction with a continuous variable, a long second
//! label in a cell) is printed as before, in R's layout, and says so.

/// The printers, and the small pieces they share.
pub(super) const TABLES_HELPER: &str = r##"stata_tab_pad <- function(s, w) formatC(as.character(s), width = w)
stata_tab_abbrev <- function(s, n = 12) {
  ifelse(nchar(s) > n, paste0(substr(s, 1, n - 2), "~", substr(s, nchar(s), nchar(s))), s)
}
stata_tab_count <- function(v) stata_fmt(v, "%20.0fc", trim = TRUE)
stata_tab_g <- function(v, fmt = "%9.0g") stata_fmt(v, fmt)
# summarize: Obs with commas, the rest at %9.0g, a rule after every five
stata_sum_print <- function(names, N, mean, sd, min, max, string) {
  rule <- paste0(strrep("-", 13), "+", strrep("-", 57), "\n")
  cat("\n    Variable |        Obs        Mean    Std. dev.       Min        Max\n", rule, sep = "")
  for (i in seq_along(names)) {
    if (i > 1 && (i - 1) %% 5 == 0) cat(rule)
    lab <- stata_tab_pad(stata_tab_abbrev(names[i]), 12)
    if (string[i] || N[i] == 0) {
      cat(lab, " |", stata_tab_pad(stata_tab_count(N[i]), 11), "\n", sep = "")
      next
    }
    cat(lab, " |", stata_tab_pad(stata_tab_count(N[i]), 11), stata_tab_pad(stata_tab_g(mean[i]), 12),
        stata_tab_pad(stata_tab_g(sd[i]), 12), stata_tab_pad(stata_tab_g(min[i]), 11),
        stata_tab_pad(stata_tab_g(max[i]), 11), "\n", sep = "")
  }
  invisible(NULL)
}
# a one-way table: the labels' column as wide as the widest (11 at least),
# a name longer than that split over two lines
stata_tab1_print <- function(name, labs, freq) {
  total <- sum(freq)
  pct <- 100 * freq / total
  cum <- cumsum(pct)
  W <- max(11, nchar(labs))
  cat("\n")
  if (nchar(name) > W) {
    cat(stata_tab_pad(substr(name, 1, W), W), " |\n", sep = "")
    name <- substr(name, W + 1, 2 * W)
  }
  cat(stata_tab_pad(name, W), " |      Freq.     Percent        Cum.\n", sep = "")
  rule <- paste0(strrep("-", W + 1), "+", strrep("-", 35), "\n")
  cat(rule)
  for (i in seq_along(labs)) {
    cat(stata_tab_pad(labs[i], W), " |", stata_tab_pad(stata_tab_count(freq[i]), 11),
        stata_tab_pad(sprintf("%.2f", pct[i]), 12), stata_tab_pad(sprintf("%.2f", cum[i]), 12), "\n", sep = "")
  }
  cat(rule, stata_tab_pad("Total", W), " |", stata_tab_pad(stata_tab_count(total), 11),
      stata_tab_pad("100.00", 12), "\n", sep = "")
  invisible(NULL)
}
# a column's label is cut at 9 characters (Stata 19: "Secondary - complete" is "Secondary"); a
# number, not a label, is as it is
stata_tab2_clabs <- function(clabs) {
  num <- grepl("^-?[0-9.]+(e[-+]?[0-9]+)?$", clabs)
  ifelse(num, clabs, substr(clabs, 1, 9))
}
# whether a two-way table has the shape Stata's was seen in
stata_tab2_fits <- function(rname, cname, clabs) {
  nchar(rname) <= 10 && all(nchar(stata_tab2_clabs(clabs)) <= 10) && nchar(cname) <= 33
}
# a two-way table: rows' labels cut at 21, columns of 10 a space apart in blocks that fit
# a line of 80, the column variable's name centred over its columns
stata_tab2_print <- function(rname, cname, rlabs, clabs, counts, chi2 = NULL) {
  clabs <- stata_tab2_clabs(clabs)
  rlabs <- substr(rlabs, 1, 21)
  L <- max(10, nchar(rlabs))
  per <- max(1, (65 - L) %/% 11)
  k <- ncol(counts)
  blocks <- split(seq_len(k), (seq_len(k) - 1) %/% per)
  rt <- rowSums(counts)
  ct <- colSums(counts)
  cells <- function(v) paste(stata_tab_pad(vapply(v, stata_tab_count, ""), 10), collapse = " ")
  for (b in seq_along(blocks)) {
    j <- blocks[[b]]
    if (b > 1) cat("\n")
    off <- max(0, ceiling((11 * length(j) - nchar(cname)) / 2))
    cat("\n", strrep(" ", L), " |", strrep(" ", off), cname, "\n", sep = "")
    cat(stata_tab_pad(rname, L), " |", paste(stata_tab_pad(clabs[j], 10), collapse = " "), " |     Total\n", sep = "")
    rule <- paste0(strrep("-", L + 1), "+", strrep("-", 11 * length(j)), "+", strrep("-", 10), "\n")
    cat(rule)
    for (i in seq_len(nrow(counts))) {
      cat(stata_tab_pad(rlabs[i], L), " |", cells(counts[i, j]), " |", stata_tab_pad(stata_tab_count(rt[i]), 10),
          " \n", sep = "")
    }
    cat(rule, stata_tab_pad("Total", L), " |", cells(ct[j]), " |", stata_tab_pad(stata_tab_count(sum(counts)), 10),
        " \n", sep = "")
  }
  if (!is.null(chi2)) {
    cat("\n", strrep(" ", 10), sprintf("Pearson chi2(%d) = %8.4f   Pr = %5.3f", as.integer(chi2[["df"]]),
        chi2[["chi2"]], chi2[["p"]]), "\n", sep = "")
  }
  invisible(NULL)
}
# regress's coefficient rows from e(b)'s names: a factor with one level
# shown inline (1.g), one with more or with value labels, two factors
# interacted (cells 2 1, or 2#yes with labels) and any interaction with a
# continuous variable, as a group (its name, then its levels); NULL for a
# shape not checked against Stata (read off Stata 19, probes 286 and P2C.15's)
stata_coef_rows <- function(nm) {
  one_f <- "^([0-9]+)([bo]?)\\.([A-Za-z_][A-Za-z0-9_]*)$"
  two_f <- "^([0-9]+)([bo]?)\\.([A-Za-z_][A-Za-z0-9_]*)#([0-9]+)([bo]?)\\.([A-Za-z_][A-Za-z0-9_]*)$"
  f_c <- "^[0-9]+\\.[A-Za-z_][A-Za-z0-9_]*(#c\\.[A-Za-z_][A-Za-z0-9_]*)+$"
  parts <- function(n, re) regmatches(n, regexec(re, n))[[1]]
  # a value's label (the data's value labels), or the value itself
  labs <- function(v) attr(stata_data[[v]], "labels")
  lab <- function(v, l) {
    s <- labs(v)
    i <- if (is.null(s)) NA else match(as.numeric(l), as.numeric(s))
    if (is.na(i)) l else names(s)[i]
  }
  short <- function(s) if (nchar(s) > 11) paste0(substr(s, 1, 9), "..") else s
  key <- function(n) {
    if (grepl(one_f, n)) return(parts(n, one_f)[4])
    if (grepl(two_f, n)) return(paste0(parts(n, two_f)[4], "#", parts(n, two_f)[7]))
    if (grepl(f_c, n)) return(sub("^[0-9]+\\.", "", n))
    NA_character_
  }
  # which cells of two factors are shown: those estimated (a base mark
  # stays on one), and those left out with no factor at its base (empty
  # cells, or collinear ones)
  shows <- function(n) {
    if (!grepl(two_f, n)) return(!grepl("(^|#)[0-9]+b\\.", n))
    p <- parts(n, two_f)
    (p[3] != "o" && p[6] != "o" && !(p[3] == "b" && p[6] == "b")) || (p[3] != "b" && p[6] != "b")
  }
  shown <- nm[vapply(nm, shows, TRUE)]
  # a term omitted for collinearity is a row of its own, 0 (omitted); one in
  # an interaction with a continuous variable was not checked
  if (any(!grepl(two_f, shown) & grepl("#", shown, fixed = TRUE) & grepl("(^|#)([0-9]+)?o\\.", shown))) return(NULL)
  keys <- vapply(shown, key, "")
  terms <- list()
  seen <- character(0)
  for (n in shown) {
    k <- keys[[n]]
    if (is.na(k)) {
      plain <- sub("^o\\.", "", n)
      # a plain variable or the constant; an interaction of continuous
      # variables was not checked
      if (grepl("#", plain, fixed = TRUE) || grepl(".", plain, fixed = TRUE)) return(NULL)
      terms[[length(terms) + 1]] <- list(big = FALSE, rows = list(
        list(kind = "row", label = plain, col = n, note = if (plain != n) "(omitted)")))
      next
    }
    if (k %in% seen) next
    seen <- c(seen, k)
    levels <- shown[!is.na(keys) & keys == k]
    if (grepl(two_f, n)) {
      p <- parts(n, two_f)
      v1 <- p[4]
      v2 <- p[7]
      sep <- if (is.null(labs(v1)) && is.null(labs(v2))) " " else "#"
      rows <- list(list(kind = "head", label = k))
      for (l in levels) {
        q <- parts(l, two_f)
        a <- lab(v1, q[2])
        b2 <- lab(v2, q[5])
        note <- NULL
        if (q[3] == "o" || q[6] == "o") {
          cnt <- sum(stata_data[[v1]] == as.numeric(q[2]) & stata_data[[v2]] == as.numeric(q[5]), na.rm = TRUE)
          note <- if (cnt == 0) "(empty)" else "(omitted)"
        }
        text <- paste0(a, sep, b2)
        if (nchar(text) <= 11) {
          rows[[length(rows) + 1]] <- list(kind = "level", label = text, col = l, note = note)
        } else if (sep == "#" && nchar(a) > 11 && nchar(b2) <= 11) {
          # a long first label: shortened on a line of its own, the second below
          rows[[length(rows) + 1]] <- list(kind = "wrap", label = short(a))
          rows[[length(rows) + 1]] <- list(kind = "level", label = b2, col = l, note = note)
        } else {
          return(NULL)
        }
      }
      terms[[length(terms) + 1]] <- list(big = TRUE, rows = rows)
      next
    }
    v <- sub("#.*$", "", k)
    labelled <- !grepl("#", k, fixed = TRUE) && !is.null(labs(v))
    if (length(levels) == 1 && !grepl("#", k, fixed = TRUE) && !labelled) {
      terms[[length(terms) + 1]] <- list(big = FALSE, rows = list(
        list(kind = "row", label = sub("o\\.", ".", n), col = n, note = if (grepl("^[0-9]+o\\.", n)) "(omitted)")))
      next
    }
    rows <- list(list(kind = "head", label = k))
    for (l in levels) {
      lv <- sub("o?\\..*$", "", l)
      if (labelled) lv <- short(lab(v, lv))
      rows[[length(rows) + 1]] <- list(kind = "level", label = lv, col = l,
                                       note = if (grepl("^[0-9]+o\\.", l)) "(omitted)")
    }
    # a separator goes next to an interaction or a group of more than one row
    terms[[length(terms) + 1]] <- list(big = length(levels) > 1 || grepl("#", k, fixed = TRUE), rows = rows)
  }
  out <- list()
  for (i in seq_along(terms)) {
    if (i > 1 && (terms[[i]]$big || terms[[i - 1]]$big)) out[[length(out) + 1]] <- list(kind = "blank")
    out <- c(out, terms[[i]]$rows)
  }
  out
}
# regress as Stata prints it, from e(); FALSE when its shape was not checked
stata_regress_print <- function(level = 0.95) {
  e <- stata_e
  nm <- colnames(e$b)
  rows <- stata_coef_rows(nm)
  if (is.null(rows)) return(FALSE)
  robust <- !is.null(e[["vcetype"]])
  count <- stata_tab_count
  pad <- stata_tab_pad
  if (!robust) {
    tss <- e$mss + e$rss
    dft <- e$df_m + e$df_r
    left <- function(name, ss, df) {
      paste0(pad(name, 12), " |", pad(stata_tab_g(ss, "%11.0g"), 12), pad(count(df), 10),
             pad(stata_tab_g(if (df > 0) ss / df else NA_real_, "%11.0g"), 12))
    }
    line <- function(l, label, value) cat(l, "   ", formatC(label, width = -16), "=", pad(value, 10), "\n", sep = "")
    rule <- paste0(strrep("-", 13), "+", strrep("-", 34))
    cat("\n")
    line("      Source |       SS           df       MS   ", "Number of obs", count(e$N))
    line(rule, sprintf("F(%d, %d)", as.integer(e$df_m), as.integer(e$df_r)), sprintf("%.2f", e$F))
    line(left("Model", e$mss, e$df_m), "Prob > F", sprintf("%.4f", stats::pf(e$F, e$df_m, e$df_r, lower.tail = FALSE)))
    line(left("Residual", e$rss, e$df_r), "R-squared", sprintf("%.4f", e$r2))
    line(rule, "Adj R-squared", sprintf("%.4f", e$r2_a))
    line(left("Total", tss, dft), "Root MSE", stata_tab_g(e$rmse, "%10.5g"))
  } else {
    line <- function(l, label, value) cat(formatC(l, width = -48), formatC(label, width = -18), "=", pad(value, 11), "\n", sep = "")
    cat("\n")
    line("Linear regression", "Number of obs", count(e$N))
    line("", sprintf("F(%d, %d)", as.integer(e$df_m), as.integer(e$df_r)), sprintf("%.2f", e$F))
    line("", "Prob > F", sprintf("%.4f", stats::pf(e$F, e$df_m, e$df_r, lower.tail = FALSE)))
    line("", "R-squared", sprintf("%.4f", e$r2))
    line("", "Root MSE", stata_tab_g(e$rmse, "%10.5g"))
  }
  cat("\n")
  if (!is.null(e[["N_clust"]])) {
    cat(pad(sprintf("(Std. err. adjusted for %s clusters in %s)", count(e$N_clust), e$clustvar), 78), "\n", sep = "")
  }
  # by name, which a 1 x 1 matrix would lose
  b <- stats::setNames(as.vector(e$b), colnames(e$b))
  se <- stats::setNames(sqrt(diag(as.matrix(e$V))), colnames(e$b))
  df <- e$df_r
  q <- stats::qt((1 + level) / 2, df)
  cat(strrep("-", 78), "\n", sep = "")
  if (robust) cat("             |               Robust\n")
  cat(pad(stata_tab_abbrev(e$depvar), 12), " | Coefficient  ", if (robust) "std. err." else "Std. err.",
      "      t    P>|t|     [", format(100 * level), "% conf. interval]\n", sep = "")
  cat(strrep("-", 13), "+", strrep("-", 64), "\n", sep = "")
  for (r in rows) {
    if (r$kind == "blank") {
      cat(strrep(" ", 13), "|\n", sep = "")
      next
    }
    if (r$kind == "head") {
      cat(pad(stata_tab_abbrev(r$label), 12), " |\n", sep = "")
      next
    }
    # a cell's long first label, on a line of its own
    if (r$kind == "wrap") {
      cat(pad(r$label, 11), " #|\n", sep = "")
      next
    }
    label <- if (r$kind == "level") paste0(pad(r$label, 11), " ") else pad(stata_tab_abbrev(r$label), 12)
    if (!is.null(r$note)) {
      cat(label, " |", pad("0", 11), "  ", r$note, "\n", sep = "")
      next
    }
    bi <- b[[r$col]]
    si <- se[[r$col]]
    t <- bi / si
    p <- 2 * stats::pt(-abs(t), df)
    cat(label, " |", pad(stata_tab_g(bi), 11), pad(stata_tab_g(si), 11), pad(sprintf("%.2f", t), 9),
        pad(sprintf("%.3f", p), 8), pad(stata_tab_g(bi - q * si), 13), pad(stata_tab_g(bi + q * si), 12), "\n", sep = "")
  }
  cat(strrep("-", 78), "\n", sep = "")
  TRUE
}"##;
