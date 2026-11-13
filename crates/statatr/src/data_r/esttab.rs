//! `esttab` (Ben Jann's estout package, SSC): tables of stored models, as
//! the package prints and writes them, decoded from Stata 19's output alone
//! (`plan/stata/esttab.md`), never its code.
//!
//! The rows are the models' coefficients by Stata's names in e(b), in the
//! order they first appear with `_cons` last (base levels as 0 over `(.)`
//! unless `nobaselevels`); under each, its t statistic (or `se`, `p`, none
//! with `not`) in parentheses; then `N` and the statistics asked for. The
//! numbers use esttab's automatic format (a3) unless a format is given.
//! The screen table and `.txt` are laid out in columns of fixed width;
//! `.csv` cells are `="..."`; `.tex` is a tabular with `\sym{}` stars,
//! `booktabs` rules or plain ones. Files end their lines with CR LF.
//!
//! Also as Stata 19 printed and wrote them (`plan/stata/esttab-options.md`):
//! `stats()` with `fmt()` and `labels()`, `varlabels()`, `mgroups()` (with
//! `prefix() suffix() span` in .tex), `starlevels()`, `compress`, `nogaps`,
//! `tex` on the screen, and `cells()` of `estpost summarize`'s results.

use super::*;

/// The table, its cells formatted, and its renderers.
pub(super) const ESTTAB_HELPER: &str = r##"stata_esttab_fmt <- function(x, fmt) {
  # esttab's a# (automatic), a fixed number of decimals (2) or a Stata %w.df, %w.de or %w.dg,
  # printed as Stata's string() does (12344.19 in %8.3f is 1.2e+04)
  vapply(x, function(v) {
    if (is.na(v)) return(".")
    if (grepl("^a[0-9]+$", fmt)) {
      k <- as.numeric(sub("a", "", fmt))
      if (v == round(v) && abs(v) < 1e15) return(formatC(v, format = "f", digits = 0))
      a <- abs(v)
      # Stata's log10 is ln(x) / ln(10), a little below at the powers of 10
      fl <- floor(log(a) / log(10))
      if (fl < -6) return(sprintf(paste0("%.", k - 1, "e"), v))
      d <- if (a < 1) k - 1 - fl else max(1, k - fl)
      return(formatC(v, format = "f", digits = d))
    }
    if (grepl("^[0-9]+$", fmt)) return(formatC(v, format = "f", digits = as.numeric(fmt)))
    if (grepl("^%-?0?[0-9]+\\.[0-9]+[feg]c?$", fmt)) return(stata_fmt(v, fmt, trim = TRUE))
    stop("esttab: the format ", fmt, " is not translated yet")
  }, "")
}
# padding to a width (R's formatC counts a backslash twice): w < 0 pads on the right
stata_pad <- function(s, w) {
  n <- pmax(0, abs(w) - nchar(s))
  if (w < 0) paste0(s, strrep(" ", n)) else paste0(strrep(" ", n), s)
}
stata_esttab <- function(models, d, aux = "t", bfmt = "a3", auxfmt = NULL, star = TRUE,
                         stars = c("*" = 0.05, "**" = 0.01, "***" = 0.001), star_text = NULL,
                         raw_stars = FALSE,
                         label = FALSE, nobase = FALSE, keep = NULL, drop = NULL, order = NULL,
                         stats = character(), statfmt = list(), sstats = NULL, noobs = FALSE, obslast = FALSE,
                         numbers = TRUE, mtitles = NULL, nomtitles = FALSE, title = NULL, notes = TRUE,
                         addnotes = character(), varlabels = NULL, nogaps = FALSE, mgroups = NULL,
                         cells = NULL, compress = FALSE, varwidth = NULL, modelwidth = NULL,
                         mlabels = NULL, prehead = NULL, posthead = NULL, prefoot = NULL, postfoot = NULL,
                         fragment = FALSE, alignment = NULL, format = "screen", booktabs = FALSE, file = NULL, replace = FALSE,
                         from_eststo = FALSE, refcat = NULL, refcat_label = TRUE, eqlabels = TRUE, elist = NULL) {
  # compress narrows the columns: 9 for the models, 10 (16 with label) for the names
  if (is.null(modelwidth)) modelwidth <- if (compress) 9 else 12
  if (is.null(varwidth)) varwidth <- if (label) (if (compress) 16 else 20) else (if (compress) 10 else 12)
  out <- if (!is.null(cells) && isTRUE(cells$side)) {
    stata_esttab_side(models, d, cells$items, star, stars, raw_stars, label, nobase, keep, drop, sstats,
                      noobs, numbers, mtitles, nomtitles, title, varlabels, mlabels, refcat, refcat_label,
                      eqlabels, varwidth, modelwidth, format, booktabs, prehead, posthead, prefoot,
                      postfoot, fragment, elist)
  } else if (!is.null(cells)) {
    stata_esttab_cells(models, d, cells, label, varlabels, numbers, nomtitles, noobs, varwidth, modelwidth, format)
  } else {
    stata_esttab_table(models, d, aux, bfmt, auxfmt, star, stars, star_text, raw_stars, label, nobase,
                       keep, drop, order, stats, statfmt, sstats, noobs, obslast, numbers, mtitles,
                       nomtitles, title, notes, addnotes, varlabels, nogaps, mgroups, varwidth,
                       modelwidth, format, booktabs, mlabels, prehead, posthead, prefoot, postfoot,
                       fragment, alignment)
  }
  if (is.null(file) && from_eststo) cat("(tabulating estimates stored by eststo; specify \".\" to tabulate the active results)\n")
  # on the screen a blank line comes first
  stata_esttab_emit(if (is.null(file) && format == "tex") c("", out) else out, file, replace)
}
# a table's lines on the screen, else in the file (Stata's \r\n line ends)
stata_esttab_emit <- function(out, file, replace) {
  if (is.null(file)) {
    cat(out, sep = "\n")
    cat("\n")
  } else {
    if (file.exists(file) && !replace) stop("file ", file, " already exists")
    if (!file.exists(file) && replace) cat("(file ", file, " not found)\n", sep = "")
    con <- file(file, "wb")
    writeBin(charToRaw(paste0(paste(out, collapse = "\r\n"), "\r\n")), con)
    close(con)
    cat("(output written to ", file, ")\n", sep = "")
  }
  invisible(out)
}
# esttab matrix(M, fmt()): the matrix's rows and columns by their names, a title row of
# mlabel (the matrix's name unless given, none leaving it out); on the screen and in a .txt
# 12 + 13-wide columns between rules, in a .tex `&` and 12-wide cells, the head and foot of
# a tabular unless fragment, posthead() prefoot() postfoot() in their places; in a .csv
# ="" cells
stata_esttab_matrix <- function(M, name, fmts = NULL, mlabel = NULL, title = NULL, format = "screen",
                                file = NULL, replace = FALSE, posthead = NULL, prefoot = NULL,
                                postfoot = NULL, fragment = FALSE) {
  M <- as.matrix(M)
  if (is.null(rownames(M)) || is.null(colnames(M))) stop("RCraft: esttab matrix() of a matrix without row and column names is not translated yet", call. = FALSE)
  if (any(grepl(":", c(rownames(M), colnames(M))))) stop("RCraft: esttab matrix() of a matrix with equation names is not translated yet", call. = FALSE)
  k <- ncol(M)
  if (!is.null(fmts) && length(fmts) != 1 && length(fmts) != k) stop("RCraft: esttab matrix(), fmt() with other than one format or one a column is not translated yet", call. = FALSE)
  cell <- function(v, j) {
    if (is.na(v)) return(".")
    if (is.null(fmts)) return(trimws(stata_fmt(v, "%9.0g")))
    trimws(stata_fmt(v, paste0("%30.", fmts[min(j, length(fmts))], "f")))
  }
  body <- lapply(seq_len(nrow(M)), function(i) vapply(seq_len(k), function(j) cell(M[i, j], j), ""))
  wide <- c(rownames(M)[nchar(rownames(M)) > 12], unlist(body)[nchar(unlist(body)) > 12], colnames(M)[nchar(colnames(M)) > 12])
  if (length(wide)) stop("RCraft: esttab matrix() with a name or a value wider than 12 characters is not translated yet", call. = FALSE)
  ml <- if (is.null(mlabel)) name else mlabel
  if (format == "csv") {
    q <- function(x) paste0("=\"", x, "\"", collapse = ",")
    out <- c(if (!is.na(ml)) q(c("", ml, rep("", k - 1))), q(c("", colnames(M))),
             vapply(seq_len(nrow(M)), function(i) q(c(rownames(M)[i], body[[i]])), ""))
  } else if (format == "tex") {
    if (any(grepl("[\\\\&%_#$^~{}]", c(rownames(M), colnames(M), ml)))) stop("RCraft: esttab matrix() in .tex with names holding TeX's special characters is not translated yet", call. = FALSE)
    row <- function(l, cells) paste0(stata_pad(l, -12), paste0("&", stata_pad(cells, 12), collapse = ""), "\\\\")
    head <- c(if (!is.na(ml)) row("", c(ml, rep("", k - 1))), row("", colnames(M)))
    out <- c(if (!fragment) c(sprintf("\\begin{tabular}{l*{%d}{c}}", k), "\\hline\\hline"), head,
             if (is.null(posthead)) "\\hline" else posthead,
             vapply(seq_len(nrow(M)), function(i) row(rownames(M)[i], body[[i]]), ""),
             prefoot,
             if (!is.null(postfoot)) postfoot else if (!fragment) c("\\hline\\hline", "\\end{tabular}"))
  } else {
    row <- function(l, cells) paste0(stata_pad(l, -12), paste0(stata_pad(cells, 13), collapse = ""))
    rule <- strrep("-", 12 + 13 * k)
    out <- c(if (!is.null(title)) title, rule, if (!is.na(ml)) row("", c(ml, rep("", k - 1))), row("", colnames(M)), rule,
             vapply(seq_len(nrow(M)), function(i) row(rownames(M)[i], body[[i]]), ""), rule)
  }
  stata_esttab_emit(if (is.null(file)) c("", out) else out, file, replace)
}
# the lines of a table of models' coefficients
stata_esttab_table <- function(models, d, aux, bfmt, auxfmt, star, stars, star_text, raw_stars, label,
                               nobase, keep, drop, order, stats, statfmt, sstats, noobs, obslast,
                               numbers, mtitles, nomtitles, title, notes, addnotes, varlabels, nogaps,
                               mgroups, varwidth, modelwidth, format, booktabs, mlabels, prehead,
                               posthead, prefoot, postfoot, fragment, alignment) {
  # varlabels() takes the blank lines between the coefficients away, as nogaps does
  gaps <- !nogaps && is.null(varlabels)
  stats <- c(stats, sstats$names)
  wild <- function(p) paste0("^", gsub("\\?", ".", gsub("\\*", ".*", gsub("([.+^$(){}|\\[\\]])", "\\\\\\1", p))), "$")
  hits <- function(n, pats) Reduce(`|`, lapply(pats, function(p) grepl(wild(p), n)), rep(FALSE, length(n)))
  getm <- function(name) {
    e <- if (name == ".") {
      if (!exists("stata_e")) stop("esttab: no estimation results in memory")
      stata_e
    } else {
      s <- if (exists("stata_stored")) stata_stored[[name]] else NULL
      if (is.null(s)) stop("esttab: estimation result ", name, " not found")
      s$stata_e
    }
    if (identical(e$cmd, "estpost")) stop("RCraft: esttab of estpost's results without cells() is not translated yet")
    raw <- colnames(e$b)
    if (any(grepl(":", raw))) stop("esttab of a model with several equations is not translated yet")
    if (any(grepl("#", raw))) stop("esttab of interactions is not translated yet")
    base <- grepl("^[0-9]+b?n?b\\.", raw) | grepl("^o\\.", raw) | grepl("^[0-9]+o\\.", raw)
    nm <- sub("^o\\.", "", sub("^([0-9]+)[bno]+\\.", "\\1.", raw))
    b <- as.numeric(e$b[1, ])
    se <- sqrt(pmax(diag(as.matrix(e$V)), 0))
    df <- if (!is.null(e$df_r) && is.finite(e$df_r)) e$df_r else Inf
    t <- ifelse(base, NA, b / se)
    p <- if (is.finite(df)) 2 * stats::pt(-abs(t), df) else 2 * stats::pnorm(-abs(t))
    b[base] <- 0
    se[base] <- NA
    k <- if (!is.null(e$rank)) e$rank else length(b) - sum(base)
    st <- list(N = e$N, r2 = e$r2, ar2 = e$r2_a, pr2 = e$r2_p,
               aic = if (!is.null(e$ll)) -2 * e$ll + 2 * k else NULL,
               bic = if (!is.null(e$ll)) -2 * e$ll + k * log(e$N) else NULL)
    for (s in setdiff(stats, names(st))) st[[s]] <- e[[s]]
    list(t = data.frame(name = nm, base = base, b = b, se = se, t = t, p = p, stringsAsFactors = FALSE)[!(base & nobase), , drop = FALSE],
         stats = st, depvar = if (is.null(e$depvar)) "" else e$depvar)
  }
  ms <- lapply(models, getm)
  all <- unique(unlist(lapply(ms, function(m) m$t$name)))
  all <- c(setdiff(all, "_cons"), intersect(all, "_cons"))
  if (length(keep)) all <- all[hits(all, keep)]
  if (length(drop)) all <- all[!hits(all, drop)]
  if (length(order)) {
    first <- character()
    for (o in order) first <- c(first, setdiff(all[hits(all, o)], first))
    all <- c(first, setdiff(all, first))
  }
  lab <- function(n) {
    if (!is.null(varlabels) && n %in% names(varlabels)) return(varlabels[[n]])
    if (!label) return(n)
    # varlabels() takes the place of label's Constant for _cons
    if (n == "_cons") return(if (is.null(varlabels)) "Constant" else n)
    m <- regmatches(n, regexec("^([0-9]+)\\.(.+)$", n))[[1]]
    if (length(m)) {
      v <- m[3]
      l <- if (v %in% names(d)) attr(d[[v]], "labels", exact = TRUE) else NULL
      k <- if (!is.null(l)) names(l)[match(as.numeric(m[2]), as.numeric(l))] else NA
      return(if (length(k) && !is.na(k)) k else paste0(v, "=", m[2]))
    }
    if (n %in% names(d)) stata_varlab(d, n) else n
  }
  afmt <- if (is.null(auxfmt)) switch(aux, t = "2", se = "a3", p = "3", "a3") else auxfmt
  cells <- lapply(ms, function(m) {
    r <- lapply(all, function(n) {
      i <- match(n, m$t$name)
      if (is.na(i)) return(list(main = "", st = "", aux = ""))
      x <- m$t[i, ]
      s <- ""
      if (star && !x$base && !is.na(x$p)) for (k in names(stars)) if (x$p < stars[[k]]) s <- k
      a <- switch(aux, t = x$t, se = x$se, p = x$p, NA)
      list(main = stata_esttab_fmt(x$b, bfmt), st = s,
           aux = if (aux == "none") "" else paste0("(", if (x$base) "." else stata_esttab_fmt(a, afmt), ")"))
    })
    r
  })
  if (!is.null(sstats)) {
    # stats(): the rows named, in order, with their labels and formats (the last format
    # for the rest; b()'s, N's too, without one)
    srows <- sstats$names
    slabel <- function(s) {
      i <- match(s, sstats$names)
      if (i <= length(sstats$labels)) sstats$labels[[i]] else s
    }
    sfmt <- function(s) {
      fm <- sstats$fmts
      if (length(fm)) fm[[min(match(s, sstats$names), length(fm))]] else bfmt
    }
  } else {
    srows <- c(if (!noobs && !obslast) "N", stats, if (!noobs && obslast) "N")
    slabel <- function(s) {
      if (label) switch(s, N = "Observations", r2 = "R-squared", ar2 = "Adjusted R-squared", pr2 = "Pseudo R-squared", aic = "AIC", bic = "BIC", s)
      else switch(s, N = "N", r2 = "R-sq", ar2 = "adj. R-sq", pr2 = "pseudo R-sq", aic = "AIC", bic = "BIC", s)
    }
    # the statistics take b()'s format unless given their own; N is a count
    sfmt <- function(s) if (!is.null(statfmt[[s]])) statfmt[[s]] else if (s == "N") "a3" else bfmt
  }
  scell <- function(m, s) {
    v <- m$stats[[s]]
    if (is.null(v) || !length(v) || is.na(v)) return("")
    stata_esttab_fmt(v, sfmt(s))
  }
  # the header rows: a label for each model, and mgroups' for each group of models.
  # mlabels() without span sets its labels, and the numbers above them, as the
  # coefficients are set (right-aligned, the stars' blank after); with span, from the left
  nmod <- length(ms)
  heads <- list()
  ml <- mlabels
  if (!is.null(ml)) {
    titles <- if (isTRUE(ml$none)) NULL else if (length(ml$labels)) {
      rest <- models[-seq_along(ml$labels)]
      if (any(rest == ".")) stop("RCraft: esttab, mlabels() with fewer labels than models of the active results is not translated yet")
      c(ml$labels, rest)[seq_len(nmod)]
    } else {
      if (label) stop("RCraft: esttab, mlabels(, depvars) with label is not translated yet")
      vapply(ms, function(m) m$depvar, "")
    }
    if (!is.null(titles)) {
      pre <- if (is.null(ml$prefix)) "" else gsub("@span", "1", ml$prefix, fixed = TRUE)
      suf <- if (is.null(ml$suffix)) "" else gsub("@span", "1", ml$suffix, fixed = TRUE)
      titles <- paste0(pre, titles, suf)
    }
    hspan <- isTRUE(ml$span)
    if (numbers) heads[[length(heads) + 1]] <- list(h = paste0("(", seq_len(nmod), ")"), span = hspan && !is.null(titles), mc = TRUE)
    if (!is.null(titles)) heads[[length(heads) + 1]] <- list(h = titles, span = hspan, mc = FALSE)
  } else {
    hspan <- FALSE
    if (numbers) heads[[length(heads) + 1]] <- list(h = paste0("(", seq_len(nmod), ")"), span = !nomtitles, mc = TRUE)
    if (!nomtitles) {
      heads[[length(heads) + 1]] <- list(h = if (!is.null(mtitles)) rep_len(mtitles, nmod) else
        vapply(ms, function(m) if (label && m$depvar %in% names(d)) stata_varlab(d, m$depvar) else m$depvar, ""),
        span = TRUE, mc = TRUE)
    }
  }
  groups <- NULL
  if (!is.null(mgroups)) {
    pat <- if (length(mgroups$pattern)) mgroups$pattern else 1
    # a pattern longer than the models: the rest is left out (ZNMIXH's TableF11, as Stata wrote it)
    pat <- c(pat, rep(0, nmod))[seq_len(nmod)]
    if (pat[1] != 1) stop("RCraft: esttab, mgroups() with a pattern that does not start with 1 is not translated yet")
    starts <- which(pat == 1)
    if (length(mgroups$labels) < length(starts)) stop("RCraft: esttab, mgroups() with fewer labels than groups is not translated yet")
    groups <- list(labels = mgroups$labels[seq_along(starts)], sizes = diff(c(starts, nmod + 1)),
                   prefix = mgroups$prefix, suffix = mgroups$suffix, span = isTRUE(mgroups$span))
    groups$per_model <- rep("", nmod)
    groups$per_model[starts] <- groups$labels
    # erepeat() after the spanned row, @span its columns (the first model is column 2)
    groups$after <- if (groups$span && !is.null(mgroups$erepeat)) {
      paste0(vapply(seq_along(starts), function(i) gsub("@span", paste0(starts[i] + 1, "-", starts[i] + groups$sizes[i]), mgroups$erepeat, fixed = TRUE), ""), collapse = "")
    } else ""
  }
  note1 <- switch(aux, t = "t statistics in parentheses", se = "Standard errors in parentheses",
                  p = "p-values in parentheses", NULL)
  note2 <- if (star) {
    if (!is.null(star_text)) star_text else paste(paste0(names(stars), " p<", format(unname(stars), scientific = FALSE, drop0trailing = TRUE)), collapse = ", ")
  }
  # addnotes() between the first note and the stars'
  nts <- if (notes) c(note1, addnotes, note2) else addnotes
  vw <- varwidth
  # starlevels() stars are as typed, in a column as wide as the longest; star()'s
  # are 3 wide on the screen and \sym{} in 9 in .tex
  maxs <- if (star) max(nchar(names(stars))) else 0
  sw <- if (!star) 0 else if (raw_stars) maxs else 3
  nm_cell <- function(s, w) {
    if (nchar(s) <= w) return(stata_pad(s, -w))
    paste0(substr(s, 1, w - 2), "~", substr(s, nchar(s), nchar(s)))
  }
  # the table's sections, each replaced by prehead() posthead() prefoot() postfoot() when given:
  # prehead, the header rows, posthead, the coefficients, prefoot, the statistics, postfoot
  sect <- function(given, default) {
    if (is.null(given)) default else gsub("@M", nmod, gsub("@title", if (is.null(title)) "" else title, given, fixed = TRUE), fixed = TRUE)
  }
  if (format %in% c("screen", "txt")) {
    width <- vw + nmod * (1 + modelwidth + sw)
    rule <- strrep("-", width)
    cell <- function(x, s = "") paste0(" ", stata_pad(x, modelwidth), stata_pad(s, -sw))
    row <- function(n, xs, ss = rep("", length(xs))) paste0(nm_cell(n, vw), paste0(mapply(cell, xs, ss), collapse = ""))
    # spanned: a label from the left of the models it spans
    srow <- function(labs, k) {
      w <- k * (1 + modelwidth + sw) - 1
      paste0(stata_pad("", -vw), paste0(" ", labs, strrep(" ", pmax(0, w - nchar(labs))), collapse = ""))
    }
    # on the screen a blank line comes first
    out <- c(if (format == "screen") "", sect(prehead, c(title, rule)))
    if (!is.null(groups)) {
      out <- c(out, paste0(if (groups$span) srow(groups$labels, groups$sizes) else row("", groups$per_model), groups$after))
    }
    for (h in heads) out <- c(out, if (!is.null(ml) && h$span) srow(h$h, 1) else row("", h$h))
    out <- c(out, sect(posthead, rule))
    for (j in seq_along(all)) {
      # one line a coefficient with not: no blank lines between them
      if (j > 1 && aux != "none" && gaps) out <- c(out, "")
      out <- c(out, row(lab(all[j]), vapply(cells, function(m) m[[j]]$main, ""), vapply(cells, function(m) m[[j]]$st, "")))
      if (aux != "none") out <- c(out, row("", vapply(cells, function(m) m[[j]]$aux, "")))
    }
    out <- c(out, sect(prefoot, if (length(srows)) rule))
    for (s in srows) out <- c(out, row(slabel(s), vapply(ms, scell, "", s = s)))
    out <- c(out, sect(postfoot, c(rule, nts)))
  } else if (format == "csv") {
    q <- function(x) paste0("=\"", x, "\"")
    line <- function(xs) paste(q(xs), collapse = ",")
    # the blank lines: none with nogaps; varlabels() keeps the first and those before the
    # coefficients, the statistics and the notes
    blank <- if (!nogaps) "" else character()
    out <- sect(prehead, blank)
    if (!is.null(groups)) out <- c(out, line(c("", groups$per_model)))
    for (h in heads) out <- c(out, line(c("", h$h)))
    out <- c(out, sect(posthead, blank))
    for (j in seq_along(all)) {
      if (j > 1 && gaps) out <- c(out, "")
      out <- c(out, line(c(lab(all[j]), vapply(cells, function(m) paste0(m[[j]]$main, m[[j]]$st), ""))))
      if (aux != "none") out <- c(out, line(c("", vapply(cells, function(m) m[[j]]$aux, ""))))
    }
    out <- c(out, sect(prefoot, if (length(srows)) blank))
    for (s in srows) out <- c(out, line(c(slabel(s), vapply(ms, scell, "", s = s))))
    out <- c(out, sect(postfoot, if (length(nts)) c(blank, vapply(nts, q, ""))))
  } else {
    esc <- function(s) gsub("_", "\\\\_", s)
    # a cell padded by its length as written, then its _ escaped
    padl <- function(s, w) paste0(esc(s), strrep(" ", pmax(0, w - nchar(s))))
    padr <- function(s, w) paste0(strrep(" ", pmax(0, w - nchar(s))), esc(s))
    tsw <- if (!star) 0 else if (raw_stars) maxs else 9
    texs <- function(s) {
      if (raw_stars) return(stata_pad(s, -maxs))
      if (nzchar(s)) stata_pad(paste0("\\sym{", s, "}"), -9) else strrep(" ", 9)
    }
    sl <- function(s) if (is.null(sstats) && s == "N" && !label) stata_pad("\\(N\\)", -vw) else padl(slabel(s), vw)
    nmc <- function(s) stata_pad(s, -vw)
    tname <- function(n) {
      s <- lab(n)
      if (identical(s, "_cons")) stata_pad("\\_cons", -vw) else padl(s, vw)
    }
    tcell <- function(x, s = "") paste0("&", stata_pad(x, modelwidth), if (star) texs(s) else "")
    n <- nmod
    mw1 <- modelwidth + tsw
    top <- if (booktabs) "\\toprule" else "\\hline\\hline"
    mid <- if (booktabs) "\\midrule" else "\\hline"
    bot <- if (booktabs) "\\bottomrule" else "\\hline\\hline"
    gap <- if (booktabs) "\\addlinespace" else "[1em]"
    # a title makes it a table with a caption; fragment leaves out the head and the foot
    open <- if (is.null(title)) "{" else "\\begin{table}[htbp]\\centering"
    colspec <- if (is.null(alignment)) "c" else alignment
    out <- sect(prehead, if (!fragment) c(open, if (star) "\\def\\sym#1{\\ifmmode^{#1}\\else\\(^{#1}\\)\\fi}",
             if (!is.null(title)) paste0("\\caption{", title, "}"), sprintf("\\begin{tabular}{l*{%d}{%s}}", n, colspec), top))
    if (!is.null(groups)) {
      g <- if (groups$span) {
        # a label spanning its models, from the left, padded to their width
        k <- groups$sizes
        pre <- if (is.null(groups$prefix)) rep("", length(k)) else vapply(k, function(x) gsub("@span", x, groups$prefix, fixed = TRUE), "")
        suf <- if (is.null(groups$suffix)) "" else groups$suffix
        raw <- paste0(pre, groups$labels, suf)
        paste0("&", pre, esc(groups$labels), suf, strrep(" ", pmax(0, k * mw1 + (k - 1) - nchar(raw))))
      } else {
        paste0("&", padr(groups$per_model, modelwidth), strrep(" ", tsw))
      }
      out <- c(out, paste0(nmc(""), paste0(g, collapse = ""), "\\\\", groups$after))
    }
    for (h in heads) {
      mc <- if (h$mc) paste0("\\multicolumn{1}{c}{", h$h, "}") else h$h
      # spanned, a model's label is padded to its width from the left; without the
      # titles the numbers sit in the coefficients' column, the stars' blank after
      g <- if (h$span) padl(mc, mw1) else paste0(padr(mc, modelwidth), strrep(" ", tsw))
      out <- c(out, paste0(nmc(""), paste0("&", g, collapse = ""), "\\\\"))
    }
    out <- c(out, sect(posthead, mid))
    for (j in seq_along(all)) {
      if (j > 1 && gaps) out <- c(out, gap)
      out <- c(out, paste0(tname(all[j]), paste0(mapply(tcell, vapply(cells, function(m) m[[j]]$main, ""), vapply(cells, function(m) m[[j]]$st, "")), collapse = ""), "\\\\"))
      if (aux != "none") out <- c(out, paste0(nmc(""), paste0(vapply(vapply(cells, function(m) m[[j]]$aux, ""), tcell, ""), collapse = ""), "\\\\"))
    }
    out <- c(out, sect(prefoot, if (length(srows)) mid))
    for (s in srows) out <- c(out, paste0(sl(s), paste0(vapply(vapply(ms, scell, "", s = s), tcell, ""), collapse = ""), "\\\\"))
    texnote <- function(s) {
      s <- sub("^t statistics", "\\\\textit{t} statistics", s)
      if (star && identical(s, note2) && is.null(star_text)) {
        s <- paste(paste0("\\sym{", names(stars), "} \\(p<", format(unname(stars), scientific = FALSE, drop0trailing = TRUE), "\\)"), collapse = ", ")
      }
      sprintf("\\multicolumn{%d}{l}{\\footnotesize %s}\\\\", n + 1, s)
    }
    out <- c(out, sect(postfoot, if (!fragment) c(bot, vapply(nts, texnote, ""), "\\end{tabular}", if (is.null(title)) "}" else "\\end{table}")))
  }
  out
}
# cells() of estpost summarize's results: a column for each statistic, a row for each
# variable, N below; a statistic without fmt() takes the first one's (%9.0g without)
# cells("b(star fmt() label()) se(label())"): each model's statistics side by side, a
# column each (the first with its stars), its numbers and titles spanning them; the
# statistics without fmt() take the first one's; refcat() heading rows (ref. in the
# first column unless nolabel); the one equation's name as a row unless eqlabels(none)
stata_esttab_side <- function(models, d, items, star, stars, raw_stars, label, nobase, keep, drop, sstats,
                              noobs, numbers, mtitles, nomtitles, title, varlabels, mlabels, refcat,
                              refcat_label, eqlabels, vw, mw, format, booktabs, prehead, posthead, prefoot,
                              postfoot, fragment, elist = NULL) {
  wild <- function(p) paste0("^", gsub("\\?", ".", gsub("\\*", ".*", gsub("([.+^$(){}|\\[\\]])", "\\\\\\1", p))), "$")
  hits <- function(n, pats) Reduce(`|`, lapply(pats, function(p) grepl(wild(p), n)), rep(FALSE, length(n)))
  getm <- function(name) {
    e <- if (name == ".") stata_e else {
      s <- if (exists("stata_stored")) stata_stored[[name]] else NULL
      if (is.null(s)) stop("esttab: estimation result ", name, " not found")
      s$stata_e
    }
    raw <- colnames(e$b)
    eq <- unique(ifelse(grepl(":", raw), sub(":.*$", "", raw), ""))
    # several equations: with eqlabels(none) their rows one after another (cmclogit's alt and 2), a name in one only
    if (length(eq) > 1 && (eqlabels || anyDuplicated(sub("^[^:]*:", "", raw)))) stop("RCraft: esttab of a model with several equations is not translated yet (with eqlabels(none) and no name twice it is)")
    raw <- sub("^[^:]*:", "", raw)
    if (any(grepl("#", raw))) stop("RCraft: esttab of interactions is not translated yet")
    base <- grepl("^[0-9]+b?n?b\\.", raw) | grepl("^o\\.", raw) | grepl("^[0-9]+o\\.", raw)
    nm <- sub("^o\\.", "", sub("^([0-9]+)[bno]+\\.", "\\1.", raw))
    b <- as.numeric(e$b[1, ])
    se <- sqrt(pmax(diag(as.matrix(e$V)), 0))
    df <- if (!is.null(e$df_r) && is.finite(e$df_r)) e$df_r else Inf
    t <- ifelse(base, NA, b / se)
    p <- if (is.finite(df)) 2 * stats::pt(-abs(t), df) else 2 * stats::pnorm(-abs(t))
    b[base] <- 0
    se[base] <- NA
    tab <- data.frame(name = nm, base = base, b = b, se = se, t = t, p = p, stringsAsFactors = FALSE)
    list(t = tab[!(base & nobase), , drop = FALSE], e = e, eq = eq,
         depvar = if (is.null(e$depvar)) "" else e$depvar)
  }
  ms <- lapply(models, getm)
  nmod <- length(ms)
  all <- unique(unlist(lapply(ms, function(m) m$t$name)))
  all <- c(setdiff(all, "_cons"), intersect(all, "_cons"))
  if (length(keep)) all <- all[hits(all, keep)]
  if (length(drop)) all <- all[!hits(all, drop)]
  lab <- function(n) {
    if (!is.null(varlabels) && n %in% names(varlabels)) return(varlabels[[n]])
    if (!label) return(n)
    if (n %in% names(d)) stata_varlab(d, n) else n
  }
  fmts <- vapply(items, function(x) if (is.null(x$fmt)) NA_character_ else x$fmt, "")
  fmts[is.na(fmts)] <- if (!is.na(fmts[1])) fmts[1] else "a3"
  k <- length(items)
  cellv <- function(m, n, i) {
    j <- match(n, m$t$name)
    if (is.na(j)) return(list(v = "", s = ""))
    x <- m$t[j, ]
    v <- switch(items[[i]]$stat, b = x$b, se = x$se, t = x$t, p = x$p)
    s <- ""
    if (isTRUE(items[[i]]$star) && star && !x$base && !is.na(x$p)) for (q in names(stars)) if (x$p < stars[[q]]) s <- q
    list(v = if (is.na(v)) "" else stata_esttab_fmt(v, fmts[i]), s = s)
  }
  # the statistics' rows: stats() names them, else N
  srows <- if (!is.null(sstats)) sstats$names else if (!noobs) "N" else character()
  slabel <- function(s) {
    if (!is.null(sstats)) {
      i <- match(s, sstats$names)
      return(if (i <= length(sstats$labels)) sstats$labels[[i]] else s)
    }
    if (label && s == "N") "Observations" else s
  }
  sfmtf <- function(s) {
    if (!is.null(sstats)) {
      fm <- sstats$fmts
      return(if (length(fm)) fm[[min(match(s, sstats$names), length(fm))]] else fmts[1])
    }
    "a3"
  }
  sval <- function(m, s) {
    v <- m$e[[s]]
    if (is.null(v) || !length(v) || is.na(v)) "" else stata_esttab_fmt(v, sfmtf(s))
  }
  # the header rows: the numbers, the titles (mlabels() or the dependent variables), the cells' labels
  titles <- NULL
  if (!is.null(mlabels)) {
    if (!isTRUE(mlabels$none)) {
      tl <- if (length(mlabels$labels)) c(mlabels$labels, models[-seq_along(mlabels$labels)])[seq_len(nmod)] else vapply(ms, function(m) m$depvar, "")
      pre <- if (is.null(mlabels$prefix)) "" else gsub("@span", k, mlabels$prefix, fixed = TRUE)
      suf <- if (is.null(mlabels$suffix)) "" else gsub("@span", k, mlabels$suffix, fixed = TRUE)
      titles <- list(h = paste0(pre, tl, suf), raw = !is.null(mlabels$prefix))
    }
  } else if (!nomtitles) {
    titles <- list(h = if (!is.null(mtitles)) rep_len(mtitles, nmod) else vapply(ms, function(m) m$depvar, ""), raw = FALSE)
  }
  clabels <- vapply(items, function(x) if (is.null(x$label)) x$stat else x$label, "")
  eqrow <- if (eqlabels && nzchar(ms[[1]]$eq[1])) ms[[1]]$eq[1] else NULL
  sect <- function(given, default) {
    if (is.null(given)) return(default)
    gsub("@M", nmod, gsub("@title", if (is.null(title)) "" else title, given, fixed = TRUE), fixed = TRUE)
  }
  nm_cell <- function(s, w) if (nchar(s) <= w) stata_pad(s, -w) else paste0(substr(s, 1, w - 2), "~", substr(s, nchar(s), nchar(s)))
  blankm <- lapply(seq_len(nmod), function(j) lapply(seq_len(k), function(i) list(v = "", s = "")))
  firstonly <- function(v) lapply(v, function(x) c(list(list(v = x, s = "")), lapply(seq_len(k - 1), function(i) list(v = "", s = ""))))
  rowsx <- function(emit) {
    out <- character()
    first <- TRUE
    for (n in all) {
      if (first && !is.null(eqrow)) out <- c(out, emit$blank(eqrow))
      first <- FALSE
      if (!is.null(refcat) && n %in% names(refcat)) out <- c(out, emit$ref(refcat[[n]]))
      out <- c(out, emit$coef(lab(n), lapply(ms, function(m) lapply(seq_len(k), function(i) cellv(m, n, i))), n))
    }
    out
  }
  if (format %in% c("screen", "txt")) {
    sw <- if (!star) 0 else if (raw_stars) max(nchar(names(stars))) else 3
    width <- vw + nmod * (k * (1 + mw) + sw)
    rule <- strrep("-", width)
    one <- function(i, v, s = "") paste0(" ", stata_pad(v, mw), if (i == 1) stata_pad(s, -sw) else "")
    line <- function(n, vals) paste0(nm_cell(n, vw), paste0(vapply(vals, function(m) paste0(vapply(seq_len(k), function(i) one(i, m[[i]]$v, m[[i]]$s), ""), collapse = ""), ""), collapse = ""))
    emit <- list(blank = function(n) line(n, blankm),
                 ref = function(txt) line(txt, if (refcat_label) firstonly(rep("ref.", nmod)) else blankm),
                 coef = function(n, vals, key = NULL) line(n, vals))
    out <- c(if (format == "screen") "", sect(prehead, c(title, rule)))
    if (numbers) out <- c(out, line("", firstonly(paste0("(", seq_len(nmod), ")"))))
    if (!is.null(titles)) out <- c(out, line("", firstonly(titles$h)))
    out <- c(out, line("", lapply(seq_len(nmod), function(j) lapply(seq_len(k), function(i) list(v = clabels[i], s = "")))))
    out <- c(out, sect(posthead, rule), rowsx(emit))
    out <- c(out, sect(prefoot, if (length(srows)) rule))
    for (s in srows) out <- c(out, line(slabel(s), firstonly(vapply(ms, sval, "", s = s))))
    out <- c(out, sect(postfoot, rule))
    return(out)
  }
  esc <- function(s) gsub("_", "\\\\_", s)
  padl <- function(s, w) paste0(esc(s), strrep(" ", pmax(0, w - nchar(s))))
  padr <- function(s, w) paste0(strrep(" ", pmax(0, w - nchar(s))), esc(s))
  maxs <- if (star) max(nchar(names(stars))) else 0
  tsw <- if (!star) 0 else if (raw_stars) maxs else 9
  texs <- function(s) {
    if (raw_stars) return(stata_pad(s, -maxs))
    if (nzchar(s)) stata_pad(paste0("\\sym{", s, "}"), -9) else strrep(" ", 9)
  }
  tcell <- function(i, v, s = "") paste0("&", stata_pad(v, mw), if (i == 1 && star) texs(s) else "")
  tline <- function(n, vals) paste0(n, paste0(vapply(vals, function(m) paste0(vapply(seq_len(k), function(i) tcell(i, m[[i]]$v, m[[i]]$s), ""), collapse = ""), ""), collapse = ""), "\\\\")
  tname <- function(s) if (identical(s, "_cons")) stata_pad("\\_cons", -vw) else padl(s, vw)
  span <- k * mw + tsw + (k - 1)
  hrow <- function(h, mc) paste0(stata_pad("", -vw), paste0("&", padl(if (mc) paste0("\\multicolumn{", k, "}{c}{", h, "}") else h, span), collapse = ""), "\\\\")
  top <- if (booktabs) "\\toprule" else "\\hline\\hline"
  mid <- if (booktabs) "\\midrule" else "\\hline"
  bot <- if (booktabs) "\\bottomrule" else "\\hline\\hline"
  open <- if (is.null(title)) "{" else "\\begin{table}[htbp]\\centering"
  emit <- list(blank = function(n) tline(tname(n), blankm),
               ref = function(txt) tline(padl(txt, vw), if (refcat_label) firstonly(rep("ref.", nmod)) else blankm),
               coef = function(n, vals, key = NULL) { l <- tline(tname(n), vals); if (!is.null(key) && !is.null(elist) && key %in% names(elist)) paste0(l, elist[[key]]) else l })
  out <- sect(prehead, if (!fragment) c(open, if (star) "\\def\\sym#1{\\ifmmode^{#1}\\else\\(^{#1}\\)\\fi}",
           if (!is.null(title)) paste0("\\caption{", title, "}"), sprintf("\\begin{tabular}{l*{%d}{%s}}", nmod, strrep("c", k)), top))
  if (numbers) out <- c(out, hrow(paste0("(", seq_len(nmod), ")"), TRUE))
  if (!is.null(titles)) out <- c(out, hrow(titles$h, !titles$raw))
  out <- c(out, paste0(stata_pad("", -vw), paste0(vapply(seq_len(nmod), function(j) paste0(vapply(seq_len(k), function(i) paste0("&", padr(clabels[i], mw), if (i == 1 && star) strrep(" ", tsw) else ""), ""), collapse = ""), ""), collapse = ""), "\\\\"))
  out <- c(out, sect(posthead, mid), rowsx(emit))
  out <- c(out, sect(prefoot, if (length(srows)) mid))
  for (s in srows) {
    l <- if (is.null(sstats) && s == "N" && !label) stata_pad("\\(N\\)", -vw) else padl(slabel(s), vw)
    out <- c(out, tline(l, firstonly(vapply(ms, sval, "", s = s))))
  }
  out <- c(out, sect(postfoot, if (!fragment) c(bot, "\\end{tabular}", if (is.null(title)) "}" else "\\end{table}")))
  out
}
stata_esttab_cells <- function(models, d, cells, label, varlabels, numbers, nomtitles, noobs, vw, mw, format) {
  if (length(models) != 1) stop("RCraft: esttab, cells() of more than one model is not translated yet")
  e <- if (models[1] == ".") {
    if (!exists("stata_e")) stop("esttab: no estimation results in memory")
    stata_e
  } else stata_stored[[models[1]]]$stata_e
  if (!identical(e$cmd, "estpost") || !identical(e$subcmd, "summarize")) {
    stop("RCraft: esttab, cells() is translated for the results of estpost summarize, not yet for other models")
  }
  nms <- vapply(cells, function(x) x$stat, "")
  fmts <- vapply(cells, function(x) if (is.null(x$fmt)) NA_character_ else x$fmt, "")
  fmts[is.na(fmts)] <- if (!is.na(fmts[1])) fmts[1] else "%9.0g"
  vars <- colnames(e$mean)
  k <- length(nms)
  val <- function(i, v) stata_esttab_fmt(e[[nms[i]]][1, v], fmts[i])
  lab <- function(n) {
    if (!is.null(varlabels) && n %in% names(varlabels)) return(varlabels[[n]])
    if (label && n %in% names(d)) stata_varlab(d, n) else n
  }
  depvar <- if (is.null(e$depvar)) "" else e$depvar
  ncell <- stata_esttab_fmt(e$N, "a3")
  rest <- rep("", k - 1)
  if (format == "tex") {
    esc <- function(s) gsub("_", "\\\\_", s)
    padl <- function(s, w) paste0(esc(s), strrep(" ", pmax(0, w - nchar(s))))
    padr <- function(s, w) paste0(strrep(" ", pmax(0, w - nchar(s))), esc(s))
    blank <- strrep(" ", vw)
    spanw <- k * mw + (k - 1)
    cellsr <- function(xs) paste0("&", padr(xs, mw), collapse = "")
    out <- c("{", "\\def\\sym#1{\\ifmmode^{#1}\\else\\(^{#1}\\)\\fi}",
             sprintf("\\begin{tabular}{l*{1}{%s}}", strrep("c", k)), "\\hline\\hline")
    if (numbers && !nomtitles) out <- c(out, paste0(blank, "&", padl(sprintf("\\multicolumn{%d}{c}{(1)}", k), spanw), "\\\\"))
    if (numbers && nomtitles) out <- c(out, paste0(blank, cellsr(c("\\multicolumn{1}{c}{(1)}", rest)), "\\\\"))
    if (!nomtitles) out <- c(out, paste0(blank, "&", padl(sprintf("\\multicolumn{%d}{c}{%s}", k, depvar), spanw), "\\\\"))
    out <- c(out, paste0(blank, cellsr(nms), "\\\\"), "\\hline")
    for (v in vars) out <- c(out, paste0(padl(lab(v), vw), cellsr(vapply(seq_len(k), val, "", v = v)), "\\\\"))
    if (!noobs) {
      out <- c(out, "\\hline", paste0(if (label) padl("Observations", vw) else stata_pad("\\(N\\)", -vw), cellsr(c(ncell, rest)), "\\\\"))
    }
    out <- c(out, "\\hline\\hline", "\\end{tabular}", "}")
  } else {
    nm_cell <- function(s, w) {
      if (nchar(s) <= w) return(stata_pad(s, -w))
      paste0(substr(s, 1, w - 2), "~", substr(s, nchar(s), nchar(s)))
    }
    row <- function(n, xs) paste0(nm_cell(n, vw), paste0(" ", stata_pad(xs, mw), collapse = ""))
    rule <- strrep("-", vw + k * (1 + mw))
    out <- if (format == "screen") "" else character()
    out <- c(out, rule, row("", c("(1)", rest)), row("", c(depvar, rest)), row("", nms), rule)
    for (v in vars) out <- c(out, row(lab(v), vapply(seq_len(k), val, "", v = v)))
    out <- c(out, rule, row(if (label) "Observations" else "N", c(ncell, rest)), rule)
  }
  out
}"##;

impl Session {
    /// `esttab [names] [using file] [, options]`.
    pub(super) fn esttab(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let known = [
            ("se", 2),
            ("t", 1),
            ("p", 1),
            ("not", 3),
            ("b", 1),
            ("star", 4),
            ("nostar", 6),
            ("label", 3),
            ("title", 5),
            ("mtitles", 7),
            ("nomtitles", 9),
            ("nonumbers", 8),
            ("nonotes", 7),
            ("addnotes", 7),
            ("scalars", 7),
            ("r2", 2),
            ("ar2", 3),
            ("pr2", 3),
            ("aic", 3),
            ("bic", 3),
            ("obslast", 7),
            ("noobs", 5),
            ("keep", 4),
            ("drop", 4),
            ("order", 5),
            ("nobaselevels", 12),
            ("noconstant", 10),
            ("modelwidth", 10),
            ("varwidth", 8),
            ("booktabs", 8),
            ("tex", 3),
            ("csv", 3),
            ("replace", 7),
            ("compress", 8),
            ("nogaps", 6),
            ("varlabels", 8),
            ("stats", 5),
            ("starlevels", 10),
            ("mgroups", 7),
            ("cells", 5),
            ("mlabels", 7),
            ("prehead", 7),
            ("posthead", 8),
            ("prefoot", 7),
            ("postfoot", 8),
            ("fragment", 8),
            ("alignment", 9),
            ("refcat", 6),
            ("eqlabels", 8),
            ("sfmt", 4),
        ];
        if let Some(o) = opts.other(&known) {
            return Err(no(format!(
                "esttab with the option {o} is not translated yet"
            )));
        }
        if opts.has("tex", 3) && opts.has("booktabs", 8) {
            return Err(no(
                "only one allowed of smcl, fixed, tab, csv, scsv, rtf, html, html5, tex, booktabs, \
                 md, or mmd (Stata: r(198))",
            ));
        }
        if c.parts.main.trim_start().starts_with("matrix(") {
            return self.esttab_matrix(c, opts, out);
        }
        // the models: names, else those eststo stored, else the active ones
        let mut names: Vec<String> = c
            .parts
            .main
            .split_whitespace()
            .map(str::to_string)
            .collect();
        let named = !names.is_empty();
        if names.is_empty() {
            names = self
                .eststo_names
                .iter()
                .filter(|n| self.stored_fits.contains_key(*n))
                .cloned()
                .collect();
        }
        // on the screen Stata says so when it takes eststo's models
        let from_eststo = !named && !names.is_empty() && c.using.is_none();
        if names.is_empty() {
            if matches!(self.e_ret, super::returned::Returned::Nothing) {
                return Err(no(
                    "esttab: no model is fitted or stored before it in this do-file",
                ));
            }
            names.push(".".into());
        }
        for n in &names {
            if n != "." && !self.stored_fits.contains_key(n) {
                return Err(no(format!(
                    "esttab: {n} is not stored before it in this do-file (eststo or estimates store)"
                )));
            }
        }
        let fmt_arg = |name: &str, m: usize| -> Option<String> {
            opts.arg(name, m).map(|a| {
                let a = a.trim().to_string();
                a.split(',').next().unwrap_or("").trim().to_string()
            })
        };
        let (aux, auxfmt) = if opts.has("not", 3) {
            ("none", None)
        } else if opts.has("se", 2) {
            ("se", fmt_arg("se", 2))
        } else if opts.has("p", 1) {
            ("p", fmt_arg("p", 1))
        } else {
            ("t", fmt_arg("t", 1))
        };
        let file = match &c.using {
            Some(f) => {
                let f = f.trim().trim_matches('"').replace('\\', "/");
                let ext = f.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
                let named = if f.contains('.') {
                    f.clone()
                } else {
                    format!("{f}.txt")
                };
                let kind = if opts.has("tex", 3) || ext == "tex" {
                    "tex"
                } else if opts.has("csv", 3) || ext == "csv" {
                    "csv"
                } else if ext == "txt" || !f.contains('.') {
                    "txt"
                } else {
                    return Err(no(format!(
                        "esttab using a .{ext} file is not translated yet (.txt, .csv and .tex are)"
                    )));
                };
                Some((named, kind))
            }
            None if opts.has("csv", 3) => {
                return Err(no("esttab with csv on the screen is not translated yet"))
            }
            None => None,
        };
        // tex (or booktabs) without a file is printed
        let kind = match &file {
            Some((_, k)) => *k,
            None if opts.has("tex", 3) || opts.has("booktabs", 8) => "tex",
            None => "screen",
        };
        if (file.is_some() || kind == "tex") && opts.has("not", 3) {
            return Err(no("esttab using a file with not is not translated yet"));
        }
        let extra = Extra::read(opts, kind)?;
        let words = |name: &str, m: usize| -> Vec<String> {
            opts.raw(name, m)
                .map(|a| quoted_words(&a))
                .unwrap_or_default()
        };
        let mut stats: Vec<String> = Vec::new();
        let mut statfmt: Vec<String> = Vec::new();
        for (o, m) in [("r2", 2), ("ar2", 3), ("pr2", 3), ("aic", 3), ("bic", 3)] {
            if opts.has(o, m) {
                stats.push(r_text(o));
                if let Some(f) = fmt_arg(o, m).filter(|f| !f.is_empty()) {
                    statfmt.push(format!("{o} = {}", r_text(&f)));
                }
            }
        }
        for s in words("scalars", 7) {
            if !is_name(&s) {
                return Err(no(format!(
                    "esttab scalars({s}): names are translated, not labels or formats"
                )));
            }
            stats.push(r_text(&s));
        }
        if opts.has("starlevels", 10) && (opts.has("star", 4) || opts.has("nostar", 6)) {
            return Err(no(
                "esttab with starlevels() and star() or nostar together is not translated yet",
            ));
        }
        // star() and starlevels() take the same pairs; starlevels() prints them as typed
        let star_arg = opts
            .arg("star", 4)
            .map(|s| ("star", s))
            .or_else(|| opts.arg("starlevels", 10).map(|s| ("starlevels", s)));
        let (star, stars, star_text) = if opts.has("nostar", 6) {
            (false, "NULL".to_string(), "NULL".to_string())
        } else if let Some((o, s)) = star_arg {
            let w: Vec<&str> = s.split_whitespace().collect();
            if w.len() % 2 != 0 || w.is_empty() {
                return Err(no(format!(
                    "{o}({s}): symbol and level pairs are translated"
                )));
            }
            let mut pairs = Vec::new();
            let mut text = Vec::new();
            for p in w.chunks(2) {
                let [sym, lev] = p else {
                    return Err(no(format!(
                        "{o}({s}): symbol and level pairs are translated"
                    )));
                };
                let (sym, lev) = (*sym, *lev);
                let v: f64 = lev
                    .parse()
                    .map_err(|_| no(format!("{o}({s}): {lev} is not a level")))?;
                pairs.push(format!("{} = {}", r_text(sym), r_number(v)));
                text.push(format!("{sym} p<{lev}"));
            }
            (
                true,
                format!("c({})", pairs.join(", ")),
                r_text(&text.join(", ")),
            )
        } else {
            (
                true,
                "c(\"*\" = 0.05, \"**\" = 0.01, \"***\" = 0.001)".to_string(),
                "NULL".to_string(),
            )
        };
        let mut drop = words("drop", 4);
        if opts.has("noconstant", 10) {
            drop.push("_cons".into());
        }
        let list = |v: Vec<String>| -> String {
            format!(
                "c({})",
                v.iter().map(|s| r_text(s)).collect::<Vec<_>>().join(", ")
            )
        };
        let num = |name: &str, m: usize| -> Result<String, Untranslated> {
            match opts.arg(name, m) {
                None => Ok("NULL".into()),
                Some(a) => a
                    .trim()
                    .parse::<f64>()
                    .map(r_number)
                    .map_err(|_| no(format!("{name}({a}) needs a number"))),
            }
        };
        let mtitles = words("mtitles", 7);
        let title = opts
            .arg("title", 5)
            .map(|t| t.trim().trim_matches('"').to_string());
        if extra.sstats != "NULL" {
            if opts.has("scalars", 7) {
                return Err(no(
                    "esttab with stats() and scalars() together is not translated yet",
                ));
            }
            // stats() names every row: Stata leaves r2, ar2, ... and noobs aside
            stats.clear();
            statfmt.clear();
        }
        if extra.mlabels != "NULL" && (opts.has("mtitles", 7) || opts.has("nomtitles", 9)) {
            return Err(no(
                "esttab with mlabels() and mtitles() or nomtitles together is not translated yet",
            ));
        }
        if kind == "csv" && opts.has("nogaps", 6) && !opts.has("nonotes", 7) {
            return Err(no(
                "esttab using a .csv file with nogaps and its notes is not translated yet",
            ));
        }
        if extra.cells != "NULL" && extra.side && kind == "csv" {
            return Err(no("esttab, cells() in a .csv file is not translated yet"));
        }
        if extra.cells != "NULL" && !extra.side {
            // what was checked against Stata with cells()
            let with: &[(&str, usize)] = if kind == "tex" {
                &[
                    ("tex", 3),
                    ("label", 3),
                    ("varlabels", 8),
                    ("nonumbers", 8),
                    ("nomtitles", 9),
                    ("noobs", 5),
                    ("nonotes", 7),
                    ("replace", 7),
                    ("cells", 5),
                ]
            } else {
                &[
                    ("label", 3),
                    ("varlabels", 8),
                    ("compress", 8),
                    ("nonotes", 7),
                    ("replace", 7),
                    ("cells", 5),
                ]
            };
            if let Some(o) = opts.other(with) {
                return Err(no(format!(
                    "esttab, cells() with the option {o} is not translated yet"
                )));
            }
            if kind == "csv" {
                return Err(no("esttab, cells() in a .csv file is not translated yet"));
            }
            if names.len() != 1 {
                return Err(no(
                    "esttab, cells() of more than one model is not translated yet",
                ));
            }
        }
        let d = self.data()?;
        self.helpers.insert("stata_esttab");
        self.helpers.insert("stata_graph");
        self.helpers.insert("stata_fmt");
        let call = format!(
            "stata_esttab(c({}), {d}, aux = {}, bfmt = {}, auxfmt = {}, star = {}, stars = {stars}, star_text = {star_text}, \
             raw_stars = {}, label = {}, nobase = {}, keep = {}, drop = {}, order = {}, stats = c({}), statfmt = list({}), \
             sstats = {}, noobs = {}, obslast = {}, numbers = {}, mtitles = {}, nomtitles = {}, title = {}, notes = {}, \
             addnotes = {}, varlabels = {}, nogaps = {}, mgroups = {}, cells = {}, compress = {}, \
             varwidth = {}, modelwidth = {}, mlabels = {}, prehead = {}, posthead = {}, prefoot = {}, postfoot = {}, \
             fragment = {}, alignment = {}, format = {}, booktabs = {}, file = {}, replace = {}, from_eststo = {}, \
             refcat = {}, refcat_label = {}, eqlabels = {}, elist = {})",
            names.iter().map(|n| r_text(n)).collect::<Vec<_>>().join(", "),
            r_text(aux),
            r_text(&fmt_arg("b", 1).filter(|f| !f.is_empty()).unwrap_or_else(|| "a3".into())),
            auxfmt.filter(|f| !f.is_empty()).map(|f| r_text(&f)).unwrap_or_else(|| "NULL".into()),
            super::stats::r_logical(star),
            super::stats::r_logical(star && opts.has("starlevels", 10)),
            super::stats::r_logical(opts.has("label", 3)),
            super::stats::r_logical(opts.has("nobaselevels", 12)),
            list(words("keep", 4)),
            list(drop),
            list(words("order", 5)),
            stats.join(", "),
            statfmt.join(", "),
            extra.sstats,
            super::stats::r_logical(opts.has("noobs", 5)),
            super::stats::r_logical(opts.has("obslast", 7)),
            super::stats::r_logical(!opts.has("nonumbers", 8)),
            if mtitles.is_empty() { "NULL".to_string() } else { list(mtitles) },
            super::stats::r_logical(opts.has("nomtitles", 9)),
            title.map(|t| r_text(&t)).unwrap_or_else(|| "NULL".into()),
            super::stats::r_logical(!opts.has("nonotes", 7)),
            list(words("addnotes", 7)),
            extra.varlabels,
            super::stats::r_logical(opts.has("nogaps", 6)),
            extra.mgroups,
            extra.cells,
            super::stats::r_logical(opts.has("compress", 8)),
            num("varwidth", 8)?,
            num("modelwidth", 10)?,
            extra.mlabels,
            extra.heads[0],
            extra.heads[1],
            extra.heads[2],
            extra.heads[3],
            super::stats::r_logical(extra.fragment),
            extra.alignment,
            r_text(kind),
            super::stats::r_logical(opts.has("booktabs", 8)),
            file.as_ref().map(|(f, _)| r_text(f)).unwrap_or_else(|| "NULL".into()),
            super::stats::r_logical(opts.has("replace", 7)),
            super::stats::r_logical(from_eststo),
            extra.refcat,
            super::stats::r_logical(extra.refcat_label),
            super::stats::r_logical(extra.eqlabels),
            extra.elist,
        );
        if file.is_none() && self.quiet(c) {
            return Ok(());
        }
        out.line(if file.is_some() {
            super::returned::quietly(c, call)
        } else {
            call
        });
        out.caveat("esttab as the estout package prints it, matched to its output (not its code)");
        Ok(())
    }
}

/// The options read into R values: `stats()`, `varlabels()`, `mgroups()`,
/// `cells()`, `mlabels()`, `prehead()` `posthead()` `prefoot()` `postfoot()` and
/// `alignment()`, each `NULL` when not given; `fragment`.
struct Extra {
    sstats: String,
    varlabels: String,
    mgroups: String,
    cells: String,
    mlabels: String,
    heads: [String; 4],
    fragment: bool,
    alignment: String,
    side: bool,
    refcat: String,
    refcat_label: bool,
    eqlabels: bool,
    elist: String,
}

/// The lines of prehead() and the like: each quoted string (empty ones kept) and
/// each word outside quotes (`posthead(\hline)`; the `}` ZNMIXH's postfoot has after
/// a closing quote is a line of its own in Stata); `None` for an unclosed quote.
pub(super) fn quoted_lines(text: &str) -> Option<Vec<String>> {
    let mut out = Vec::new();
    let mut cur: Option<String> = None;
    let mut word = String::new();
    for ch in text.chars() {
        match (&mut cur, ch) {
            (None, '"') => {
                if !word.is_empty() {
                    out.push(std::mem::take(&mut word));
                }
                cur = Some(String::new());
            }
            (Some(_), '"') => out.extend(cur.take()),
            (Some(s), c) => s.push(c),
            (None, c) if c.is_whitespace() => {
                if !word.is_empty() {
                    out.push(std::mem::take(&mut word));
                }
            }
            (None, c) => word.push(c),
        }
    }
    if !word.is_empty() {
        out.push(word);
    }
    if cur.is_some() || out.is_empty() {
        return None;
    }
    Some(out)
}

/// `"text"' compound quotes as plain ones, when the text holds no quote.
fn uncompound(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(i) = rest.find("`\"") {
        let before = rest.get(..i).unwrap_or("");
        let inner = rest.get(i + 2..).unwrap_or("");
        match inner.find("\"'") {
            Some(j) if !inner.get(..j).unwrap_or("").contains('"') => {
                out.push_str(before);
                out.push('"');
                out.push_str(inner.get(..j).unwrap_or(""));
                out.push('"');
                rest = inner.get(j + 2..).unwrap_or("");
            }
            _ => {
                out.push_str(rest.get(..i + 2).unwrap_or(""));
                rest = inner;
            }
        }
    }
    out.push_str(rest);
    out
}

/// A format esttab takes: a#, a number of decimals, or a Stata %w.d[feg][c].
fn esttab_format(f: &str) -> bool {
    let digits = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
    if let Some(k) = f.strip_prefix('a') {
        return digits(k);
    }
    if digits(f) {
        return true;
    }
    let Some(rest) = f.strip_prefix('%') else {
        return false;
    };
    let rest = rest.strip_suffix('c').unwrap_or(rest);
    let rest = rest.trim_start_matches('-');
    let Some((w, d)) = rest.split_once('.') else {
        return false;
    };
    let Some(d) = d.strip_suffix(['f', 'e', 'g']) else {
        return false;
    };
    digits(w) && digits(d)
}

impl Extra {
    fn read(opts: &Opts, kind: &str) -> Result<Extra, Untranslated> {
        let list = |v: &[String]| -> String {
            format!(
                "c({})",
                v.iter().map(|s| r_text(s)).collect::<Vec<_>>().join(", ")
            )
        };
        let plain = |what: &str, text: &str| -> Result<(), Untranslated> {
            if text.contains("`\"") || text.contains("\"'") {
                return Err(no(format!(
                    "esttab, {what} with compound quotes is not translated yet"
                )));
            }
            Ok(())
        };
        // stats(names [, fmt() labels()])
        let sstats = match opts.arg("stats", 5) {
            None => "NULL".to_string(),
            Some(a) => {
                let a = uncompound(&a);
                plain("stats()", &a)?;
                let p = crate::lang::syntax::parts(&a);
                let names: Vec<String> = p.main.split_whitespace().map(str::to_string).collect();
                if names.is_empty() {
                    return Err(no("esttab, stats() without names is not translated yet"));
                }
                for n in &names {
                    if !is_name(n) || ["ar2", "pr2", "aic", "bic"].contains(&n.as_str()) {
                        return Err(no(format!("esttab, stats({n}) is not translated yet")));
                    }
                }
                let mut fmts = Vec::new();
                let mut labels = Vec::new();
                for o in crate::lang::syntax::options(&p.options) {
                    let inner = o.raw.clone().unwrap_or_default();
                    if o.is("fmt", 3) {
                        fmts = inner.split_whitespace().map(str::to_string).collect();
                        if let Some(f) = fmts.iter().find(|f| !esttab_format(f)) {
                            return Err(no(format!(
                                "esttab, stats(..., fmt({f})) is not translated yet"
                            )));
                        }
                    } else if o.is("labels", 5) {
                        labels = quoted_words(&inner);
                    } else {
                        return Err(no(format!(
                            "esttab, stats(..., {}()) is not translated yet",
                            o.name
                        )));
                    }
                }
                format!(
                    "list(names = {}, labels = {}, fmts = {})",
                    list(&names),
                    list(&labels),
                    list(&fmts)
                )
            }
        };
        let mut elist = "NULL".to_string();
        // varlabels(name label ...): pairs in turn, an odd word last left out
        let varlabels = match opts.arg("varlabels", 8) {
            None => "NULL".to_string(),
            Some(a) => {
                plain("varlabels()", &a)?;
                let p = crate::lang::syntax::parts(&a);
                // elist(name text ...): text after the row's end (YFBPT6's \addlinespace)
                for o in crate::lang::syntax::options(&p.options) {
                    if o.is("elist", 5) && o.raw.is_some() {
                        let w = quoted_words(&o.raw.clone().unwrap_or_default());
                        if w.len() % 2 != 0 {
                            return Err(no(
                                "esttab, varlabels(..., elist()) takes name and text pairs",
                            ));
                        }
                        let pairs: Vec<String> = w
                            .chunks(2)
                            .filter_map(|p| match p {
                                [n, t] => Some(format!("{} = {}", r_text(n), r_text(t))),
                                _ => None,
                            })
                            .collect();
                        elist = format!("c({})", pairs.join(", "));
                    } else {
                        return Err(no(format!(
                            "esttab, varlabels(..., {}) is not translated yet",
                            o.name
                        )));
                    }
                }
                let w = quoted_words(&p.main);
                let mut pairs = Vec::new();
                for pair in w.chunks(2) {
                    if let [n, l] = pair {
                        if n.contains(['*', '?', '~']) {
                            return Err(no(format!(
                                "esttab, varlabels() of a pattern ({n}) is not translated yet"
                            )));
                        }
                        pairs.push(format!("{} = {}", r_text(n), r_text(l)));
                    }
                }
                format!("c({})", pairs.join(", "))
            }
        };
        // mgroups(labels [, pattern() prefix() suffix() span])
        let mgroups = match opts.arg("mgroups", 7) {
            None => "NULL".to_string(),
            Some(a) => {
                plain("mgroups()", &a)?;
                let p = crate::lang::syntax::parts(&a);
                let labels = quoted_words(&p.main);
                let mut pattern = Vec::new();
                let (mut prefix, mut suffix, mut span) = (None, None, false);
                let mut erepeat = None;
                for o in crate::lang::syntax::options(&p.options) {
                    let inner = o.raw.clone().unwrap_or_default();
                    if o.is("pattern", 3) {
                        for w in inner.split_whitespace() {
                            match w {
                                "0" | "1" => pattern.push(w.to_string()),
                                _ => {
                                    return Err(no(format!(
                                        "esttab, mgroups(..., pattern({inner})): 0s and 1s are translated"
                                    )))
                                }
                            }
                        }
                    } else if o.is("prefix", 6) && o.raw.is_some() {
                        prefix = Some(inner);
                    } else if o.is("suffix", 6) && o.raw.is_some() {
                        suffix = Some(inner);
                    } else if o.is("span", 4) && o.raw.is_none() {
                        span = true;
                    } else if o.is("erepeat", 7) && o.raw.is_some() {
                        erepeat = Some(inner);
                    } else {
                        return Err(no(format!(
                            "esttab, mgroups(..., {}) is not translated yet",
                            o.name
                        )));
                    }
                }
                if labels.is_empty() {
                    return Err(no("esttab, mgroups() without labels is not translated yet"));
                }
                // as checked against Stata: prefix() and suffix() with span in .tex,
                // span on the screen and in .txt
                if (prefix.is_some() || suffix.is_some()) && (kind != "tex" || !span) {
                    return Err(no(
                        "esttab, mgroups(..., prefix() suffix()) is translated with span in .tex",
                    ));
                }
                if span && kind == "csv" {
                    return Err(no(
                        "esttab, mgroups(..., span) in a .csv file is not translated yet",
                    ));
                }
                // erepeat() was checked on the screen, in .txt and in .tex
                if erepeat.is_some() && kind == "csv" {
                    return Err(no(
                        "esttab, mgroups(..., erepeat()) in a .csv file is not translated yet",
                    ));
                }
                let text =
                    |s: Option<String>| s.map(|s| r_text(&s)).unwrap_or_else(|| "NULL".into());
                format!(
                    "list(labels = {}, pattern = as.numeric({}), prefix = {}, suffix = {}, span = {}, erepeat = {})",
                    list(&labels),
                    list(&pattern),
                    text(prefix),
                    text(suffix),
                    super::stats::r_logical(span),
                    text(erepeat)
                )
            }
        };
        // cells("stat[(fmt(f))] ..."): estpost summarize's statistics; or a model's
        // b(star fmt() label()) se(fmt() label()) side by side (also t and p)
        let mut side = false;
        let cells = match opts.arg("cells", 5) {
            None => "NULL".to_string(),
            Some(a) => {
                plain("cells()", &a)?;
                let a = a.trim().trim_matches('"');
                if a.contains(['&', '[', '"']) {
                    return Err(no(format!("esttab, cells({a}) is not translated yet")));
                }
                const STATS: &[&str] =
                    &["count", "sum_w", "mean", "Var", "sd", "min", "max", "sum"];
                const MODEL: &[&str] = &["b", "se", "t", "p"];
                let parsed = crate::lang::syntax::options(a);
                side = parsed
                    .first()
                    .is_some_and(|o| MODEL.contains(&o.name.as_str()));
                let mut els = Vec::new();
                for o in &parsed {
                    let ok = if side {
                        MODEL.contains(&o.name.as_str())
                    } else {
                        STATS.contains(&o.name.as_str())
                    };
                    if !ok {
                        return Err(no(format!(
                            "esttab, cells({}): estpost summarize's statistics ({}), or b, se, t and p of models, are translated",
                            o.name,
                            STATS.join(" ")
                        )));
                    }
                    let mut fmt = "NULL".to_string();
                    let mut star = false;
                    let mut label = "NULL".to_string();
                    for s in crate::lang::syntax::options(&o.raw.clone().unwrap_or_default()) {
                        let f = s.raw.clone().unwrap_or_default();
                        let f = f.trim();
                        if s.is("fmt", 3) && esttab_format(f) {
                            fmt = r_text(f);
                        } else if side && s.is("star", 4) && s.raw.is_none() {
                            star = true;
                        } else if side && s.is("label", 3) && s.raw.is_some() {
                            label = r_text(f.trim_matches('"'));
                        } else {
                            return Err(no(format!(
                                "esttab, cells({}({})) is not translated yet",
                                o.name,
                                o.raw.clone().unwrap_or_default()
                            )));
                        }
                    }
                    els.push(if side {
                        format!(
                            "list(stat = {}, fmt = {fmt}, star = {}, label = {label})",
                            r_text(&o.name),
                            super::stats::r_logical(star)
                        )
                    } else {
                        format!("list(stat = {}, fmt = {fmt})", r_text(&o.name))
                    });
                }
                if els.is_empty() {
                    return Err(no(format!("esttab, cells({a}) is not translated yet")));
                }
                if side {
                    if els.len() > 2 {
                        return Err(no("esttab, cells() of more than two statistics of models is not translated yet"));
                    }
                    format!("list(side = TRUE, items = list({}))", els.join(", "))
                } else {
                    format!("list({})", els.join(", "))
                }
            }
        };
        // mlabels(labels | none [, span prefix() suffix() depvars]): notitles and nonumbers
        // there change nothing Stata prints
        let mlabels = match opts.raw("mlabels", 7) {
            None => "NULL".to_string(),
            Some(a) => {
                plain("mlabels()", &a)?;
                let p = crate::lang::syntax::parts(&a);
                let none = p.main.trim() == "none";
                let labels = if none {
                    Vec::new()
                } else {
                    quoted_words(&p.main)
                };
                let (mut prefix, mut suffix, mut span, mut depvars) = (None, None, false, false);
                for o in crate::lang::syntax::options(&p.options) {
                    let inner = o.raw.clone().unwrap_or_default();
                    if o.is("prefix", 6) && o.raw.is_some() {
                        prefix = Some(inner);
                    } else if o.is("suffix", 6) && o.raw.is_some() {
                        suffix = Some(inner);
                    } else if o.is("span", 4) && o.raw.is_none() {
                        span = true;
                    } else if o.is("depvars", 7) && o.raw.is_none() {
                        depvars = true;
                    } else if (o.is("notitles", 4) || o.is("nonumbers", 5)) && o.raw.is_none() {
                    } else {
                        return Err(no(format!(
                            "esttab, mlabels(..., {}) is not translated yet",
                            o.name
                        )));
                    }
                }
                if none && !p.options.trim().is_empty() {
                    return Err(no(
                        "esttab, mlabels(none, ...) with suboptions is not translated yet",
                    ));
                }
                if labels.is_empty() && !none && !depvars {
                    return Err(no("esttab, mlabels() without labels is not translated yet"));
                }
                // as checked against Stata: prefix() and suffix() in .tex, @span with span
                if prefix.is_some() || suffix.is_some() {
                    let at = [&prefix, &suffix]
                        .iter()
                        .any(|s| s.as_deref().is_some_and(|s| s.contains('@')));
                    if kind != "tex" || (at && !span) {
                        return Err(no(
                            "esttab, mlabels(..., prefix() suffix()) is translated in .tex (@span with span)",
                        ));
                    }
                }
                if span && kind == "csv" {
                    return Err(no(
                        "esttab, mlabels(..., span) in a .csv file is not translated yet",
                    ));
                }
                let text =
                    |s: Option<String>| s.map(|s| r_text(&s)).unwrap_or_else(|| "NULL".into());
                format!(
                    "list(labels = {}, none = {}, prefix = {}, suffix = {}, span = {})",
                    list(&labels),
                    super::stats::r_logical(none),
                    text(prefix),
                    text(suffix),
                    super::stats::r_logical(span)
                )
            }
        };
        // prehead() posthead() prefoot() postfoot(): quoted lines in place of a section;
        // @title in prehead() is the title
        let mut heads: [String; 4] = Default::default();
        for (slot, (name, m)) in heads.iter_mut().zip([
            ("prehead", 7),
            ("posthead", 8),
            ("prefoot", 7),
            ("postfoot", 8),
        ]) {
            *slot = match opts.raw(name, m) {
                None => "NULL".to_string(),
                Some(a) => {
                    plain(&format!("{name}()"), &a)?;
                    let Some(lines) = quoted_lines(&a) else {
                        return Err(no(format!(
                            "esttab, {name}() of text outside quotes is not translated yet"
                        )));
                    };
                    let mut all = lines.join("\n");
                    if name == "prehead" {
                        all = all.replace("@title", "").replace("@M", "");
                    }
                    if all.contains('@') {
                        return Err(no(format!(
                            "esttab, {name}() with @ substitutions (other than @title and @M in prehead()) is not translated yet"
                        )));
                    }
                    list(&lines)
                }
            };
        }
        let fragment = opts.has("fragment", 8);
        if fragment {
            if kind != "tex" {
                return Err(no("esttab, fragment outside .tex is not translated yet"));
            }
            if opts.has("prehead", 7) != opts.has("postfoot", 8) {
                return Err(no(
                    "esttab, fragment with one of prehead() and postfoot() is not translated yet",
                ));
            }
        }
        // alignment() sets the columns of .tex's tabular; elsewhere Stata leaves it aside
        let alignment = match opts.arg("alignment", 9) {
            Some(a) if kind == "tex" => {
                let a = a.trim();
                if a.is_empty() || a.contains(['"', '`', '\'']) {
                    return Err(no(format!("esttab, alignment({a}) is not translated yet")));
                }
                r_text(a)
            }
            _ => "NULL".to_string(),
        };
        // refcat(name "text" ... [, nolabel]): a heading row before each name, ref. in
        // the first column unless nolabel (checked with cells() side by side)
        let (refcat, refcat_label) = match opts.raw("refcat", 6) {
            None => ("NULL".to_string(), true),
            Some(a) => {
                plain("refcat()", &a)?;
                let p = crate::lang::syntax::parts(&a);
                let mut label = true;
                for o in crate::lang::syntax::options(&p.options) {
                    if o.is("nolabel", 5) && o.raw.is_none() {
                        label = false;
                    } else {
                        return Err(no(format!(
                            "esttab, refcat(..., {}) is not translated yet",
                            o.name
                        )));
                    }
                }
                let w = quoted_words(&p.main);
                if w.len() % 2 != 0 || w.is_empty() {
                    return Err(no("esttab, refcat() takes name and text pairs"));
                }
                let pairs: Vec<String> = w
                    .chunks(2)
                    .filter_map(|p| match p {
                        [n, t] => Some(format!("{} = {}", r_text(n), r_text(t))),
                        _ => None,
                    })
                    .collect();
                (format!("c({})", pairs.join(", ")), label)
            }
        };
        // eqlabels(none): no row for the equation's name
        let eqlabels = match opts.arg("eqlabels", 8) {
            None => true,
            Some(a) if a.trim() == "none" => false,
            Some(a) => {
                return Err(no(format!(
                    "esttab, eqlabels({}) is not translated yet (none is)",
                    a.trim()
                )))
            }
        };
        if elist != "NULL" && (!side || kind != "tex") {
            return Err(no(
                "esttab, varlabels(..., elist()) is translated in .tex with cells() side by side",
            ));
        }
        if !side && (refcat != "NULL" || !eqlabels || opts.has("sfmt", 4)) {
            return Err(no(
                "esttab, refcat(), eqlabels() and sfmt() are translated with cells() of a model's statistics side by side",
            ));
        }
        Ok(Extra {
            sstats,
            varlabels,
            mgroups,
            cells,
            mlabels,
            heads,
            fragment,
            alignment,
            side,
            refcat,
            refcat_label,
            eqlabels,
            elist,
        })
    }
}

/// Words split at spaces outside quotes, the quotes taken off.
fn quoted_words(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quote = false;
    for ch in text.chars() {
        match ch {
            '"' => quote = !quote,
            c if c.is_whitespace() && !quote => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}
