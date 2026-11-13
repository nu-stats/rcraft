//! `tabulate x y, row column cell nofreq`: Stata 19's two-way table with its
//! percentages (`plan/stata/tabulate-percentages.md`). Each cell is a stack
//! of lines, the frequency first and then the row, column and cell
//! percentages asked for (`%.2f`); with more than one line a cell, a Key box
//! names them and a rule follows each row. The rest of the layout is the
//! counts table's (`stata_tab2_print`): its widths, blocks and chi2 line.

pub(super) const TABPCT_HELPER: &str = r##"stata_tab2_pct_print <- function(rname, cname, rlabs, clabs, counts, freq = TRUE,
                                 row = FALSE, column = FALSE, cell = FALSE, chi2 = NULL) {
  shown <- c(frequency = freq, "row percentage" = row, "column percentage" = column, "cell percentage" = cell)
  shown <- names(shown)[shown]
  if (!length(shown)) return(invisible(NULL))
  clabs <- stata_tab2_clabs(clabs)
  rlabs <- substr(rlabs, 1, 21)
  L <- max(10, nchar(rlabs))
  per <- max(1, (65 - L) %/% 11)
  k <- ncol(counts)
  blocks <- split(seq_len(k), (seq_len(k) - 1) %/% per)
  rt <- rowSums(counts)
  ct <- colSums(counts)
  N <- sum(counts)
  pc <- function(a, b) if (is.finite(b) && b > 0) sprintf("%.2f", 100 * a / b) else "."
  # a row's lines (with its total last): the counts n, its row's total and the column totals
  lines_of <- function(n, rtot, ctot) {
    out <- list()
    for (s in shown) out[[length(out) + 1]] <- switch(s,
      "frequency" = vapply(c(n, rtot), stata_tab_count, ""),
      "row percentage" = vapply(c(n, rtot), function(v) pc(v, rtot), ""),
      "column percentage" = vapply(seq_along(c(n, rtot)), function(j) pc(c(n, rtot)[j], c(ctot, N)[j]), ""),
      "cell percentage" = vapply(c(n, rtot), function(v) pc(v, N), ""))
    out
  }
  many <- length(shown) > 1
  if (many) {
    W <- max(nchar(shown)) + 2
    centre <- function(s) {
      l <- (W - nchar(s)) %/% 2
      paste0(strrep(" ", l), s, strrep(" ", W - nchar(s) - l))
    }
    cat("\n+", strrep("-", W), "+\n", sep = "")
    cat("| ", formatC("Key", width = -(W - 1)), "|\n", sep = "")
    cat("|", strrep("-", W), "|\n", sep = "")
    for (s in shown) cat("|", centre(s), "|\n", sep = "")
    cat("+", strrep("-", W), "+\n", sep = "")
  }
  for (b in seq_along(blocks)) {
    j <- blocks[[b]]
    if (b > 1) cat("\n")
    off <- max(0, ceiling((11 * length(j) - nchar(cname)) / 2))
    cat("\n", strrep(" ", L), " |", strrep(" ", off), cname, "\n", sep = "")
    cat(stata_tab_pad(rname, L), " |", paste(stata_tab_pad(clabs[j], 10), collapse = " "), " |     Total\n", sep = "")
    rule <- paste0(strrep("-", L + 1), "+", strrep("-", 11 * length(j)), "+", strrep("-", 10), "\n")
    cat(rule)
    show_row <- function(label, ls) {
      for (i in seq_along(ls)) {
        v <- ls[[i]]
        cat(stata_tab_pad(if (i == 1) label else "", L), " |",
            paste(stata_tab_pad(v[j], 10), collapse = " "), " |",
            stata_tab_pad(v[length(v)], 10), " \n", sep = "")
      }
    }
    for (i in seq_len(nrow(counts))) {
      show_row(rlabs[i], lines_of(counts[i, ], rt[i], ct))
      if (many || i == nrow(counts)) cat(rule)
    }
    show_row("Total", lines_of(ct, N, ct))
  }
  if (!is.null(chi2)) {
    cat("\n", strrep(" ", 10), sprintf("Pearson chi2(%d) = %8.4f   Pr = %5.3f", as.integer(chi2[["df"]]),
        chi2[["chi2"]], chi2[["p"]]), "\n", sep = "")
  }
  invisible(NULL)
}"##;
