//! `char`, `notes`, `labelbook`, `compare`, `ds`, `lookfor` and `misstable`, as
//! Stata 19 does them on made-up data (batch A2 of the StatCraft port;
//! `plan/stata/char.md` and the notes beside it).
//!
//! Characteristics live in R as the attribute `stata_char` (a named text
//! vector, in the order they were set) on a column, or on the data frame for
//! `_dta`; notes are characteristics (`note0`, `note1`, ...). A .dta file
//! keeps them in its characteristics section, which `use` reads and `save`
//! writes (formats 117 to 119, from Stata's published description).

use super::*;

/// Characteristics and notes in R.
pub(super) const CHAR_HELPER: &str = r##"stata_char_var <- function(d, v, quiet = FALSE) {
  if (isTRUE(attr(d, "stata_char_unknown"))) stop("the characteristics of this data file were not read (a .dta file of a format before Stata 13)", call. = FALSE)
  if (identical(v, "_dta")) return(v)
  nm <- names(d)
  if (v %in% nm) return(v)
  hit <- nm[startsWith(nm, v)]
  if (length(hit) == 1) return(hit)
  if (quiet) return(NA_character_)
  if (length(hit) > 1) stop(v, " ambiguous abbreviation (Stata: r(111))", call. = FALSE)
  stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
}
stata_char_own <- function(d, v) {
  if (identical(v, "_dta")) attr(d, "stata_char", exact = TRUE) else attr(d[[v]], "stata_char", exact = TRUE)
}
stata_char_all <- function(d, v, ch) {
  if (!length(ch)) ch <- NULL
  if (identical(v, "_dta")) attr(d, "stata_char") <- ch else attr(d[[v]], "stata_char") <- ch
  d
}
stata_char_put <- function(d, v, name, value) {
  v <- stata_char_var(d, v)
  ch <- stata_char_own(d, v)
  if (is.null(ch)) ch <- character()
  ch <- ch[names(ch) != name]
  value <- sub("[ \t]+$", "", sub("^[ \t]+", "", value))
  if (nzchar(value)) ch[[name]] <- value
  stata_char_all(d, v, ch)
}
stata_char_get <- function(d, v, name) {
  v <- stata_char_var(d, v, quiet = TRUE)
  if (is.na(v)) return("")
  ch <- stata_char_own(d, v)
  if (name %in% names(ch)) ch[[name]] else ""
}
stata_char_list <- function(d, owners = NULL, name = NULL) {
  if (is.null(owners)) owners <- c("_dta", names(d))
  for (v in owners) {
    v <- stata_char_var(d, v)
    ch <- stata_char_own(d, v)
    for (n in rev(names(ch))) {
      if (!is.null(name) && n != name) next
      cat(sprintf("  %-27s %s\n", paste0(v, "[", n, "]:"), ch[[n]]), sep = "")
    }
  }
  invisible(NULL)
}
stata_note_nums <- function(ch) {
  k <- grep("^note[0-9]+$", names(ch), value = TRUE)
  k <- as.integer(sub("^note", "", k))
  sort(k[k > 0L])
}
stata_note_add <- function(d, v, text) {
  v <- stata_char_var(d, v)
  text <- sub("[ \t]+$", "", sub("^[ \t]+", "", text))
  if (!nzchar(text)) {
    cat("  (no note added)\n")
    return(d)
  }
  ch <- stata_char_own(d, v)
  n0 <- if ("note0" %in% names(ch)) suppressWarnings(as.integer(ch[["note0"]])) else 0L
  if (is.na(n0)) n0 <- 0L
  n <- n0 + 1L
  d <- stata_char_put(d, v, "note0", as.character(n))
  stata_char_put(d, v, paste0("note", n), text)
}
stata_note_show <- function(d, owners, nums = NULL, pattern = NULL) {
  hit <- FALSE
  for (v in owners) {
    v <- stata_char_var(d, v)
    ch <- stata_char_own(d, v)
    ks <- stata_note_nums(ch)
    if (!is.null(nums)) ks <- ks[ks %in% nums]
    if (!is.null(pattern)) ks <- ks[vapply(ks, function(k) grepl(pattern, ch[[paste0("note", k)]], fixed = TRUE), NA)]
    if (!length(ks)) next
    hit <- TRUE
    cat("\n", v, ":\n", sep = "")
    for (k in ks) cat(sprintf("%3d.  %s\n", k, ch[[paste0("note", k)]]), sep = "")
  }
  if (!is.null(pattern) && !hit) cat(pattern, ": not found\n", sep = "")
  invisible(NULL)
}
stata_note_drop <- function(d, owners, nums = NULL) {
  total <- 0L
  for (v in unique(owners)) {
    v <- stata_char_var(d, v)
    ch <- stata_char_own(d, v)
    have <- stata_note_nums(ch)
    kill <- if (is.null(nums)) have else have[have %in% nums]
    total <- total + length(kill)
    if (!length(kill)) next
    ch <- ch[!names(ch) %in% paste0("note", kill)]
    left <- setdiff(have, kill)
    if (length(left)) ch[["note0"]] <- as.character(max(left)) else ch <- ch[names(ch) != "note0"]
    d <- stata_char_all(d, v, ch)
  }
  cat("  (", total, if (total == 1L) " note" else " notes", " dropped)\n", sep = "")
  d
}
stata_note_replace <- function(d, v, k, text) {
  v <- stata_char_var(d, v)
  ch <- stata_char_own(d, v)
  text <- sub("[ \t]+$", "", sub("^[ \t]+", "", text))
  if (k %in% stata_note_nums(ch) && nzchar(text)) {
    ch[[paste0("note", k)]] <- text
    d <- stata_char_all(d, v, ch)
    cat("  (note ", k, " for ", v, " replaced)\n", sep = "")
  } else {
    cat("  (no note replaced)\n")
  }
  d
}
stata_note_renumber <- function(d, owners) {
  for (v in unique(owners)) {
    v <- stata_char_var(d, v)
    ch <- stata_char_own(d, v)
    ks <- stata_note_nums(ch)
    if (!length(ks)) next
    texts <- vapply(ks, function(k) ch[[paste0("note", k)]], "")
    ch <- ch[!grepl("^note[0-9]+$", names(ch))]
    for (i in seq_along(ks)) ch[[paste0("note", i)]] <- texts[i]
    ch[["note0"]] <- as.character(length(ks))
    d <- stata_char_all(d, v, ch)
  }
  d
}"##;

/// The characteristics section of a .dta file: read by `use`, written by `save`.
pub(super) const DTA_CHARS_HELPER: &str = r##"stata_dta_chars_layout <- function(path, whole = FALSE) {
  size <- file.size(path)
  if (is.na(size)) return(NULL)
  if (whole && size > 2e9) return(NULL)
  con <- file(path, "rb")
  on.exit(close(con))
  head <- readBin(con, "raw", min(8192, size))
  if (length(head) < 40 || rawToChar(head[1:11]) != "<stata_dta>") return(NULL)
  rel <- suppressWarnings(as.integer(rawToChar(head[29:31])))
  if (is.na(rel) || !rel %in% 117:119) return(NULL)
  endian <- if (length(grepRaw("<byteorder>LSF", head, fixed = TRUE))) "little" else "big"
  m <- grepRaw("<map>", head, fixed = TRUE)
  if (!length(m)) return(NULL)
  w <- readBin(head[(m + 5):(m + 116)], "integer", 28, size = 4, endian = endian) %% 2^32
  lo <- if (endian == "little") seq(1, 28, 2) else seq(2, 28, 2)
  off <- w[lo] + w[-lo] * 2^32
  info <- list(rel = rel, endian = endian, map = m + 5, off = off, nm = if (rel == 117) 33 else 129)
  seek(con, off[9])
  info$chars <- readBin(con, "raw", off[10] - off[9])
  if (whole) {
    seek(con, 0)
    info$raw <- readBin(con, "raw", size)
  }
  info
}
stata_chars_read <- function(d, path) {
  info <- tryCatch(stata_dta_chars_layout(path), error = function(e) NULL)
  if (is.null(info)) {
    attr(d, "stata_char_unknown") <- TRUE
    return(d)
  }
  raw <- info$chars
  nm <- info$nm
  tag <- "<characteristics>"
  if (length(raw) < nchar(tag) || rawToChar(raw[seq_len(nchar(tag))]) != tag) return(d)
  at <- nchar(tag) + 1
  text <- function(r) {
    i <- match(as.raw(0), r)
    rawToChar(if (is.na(i)) r else r[seq_len(i - 1)])
  }
  repeat {
    if (at + 7 > length(raw) || rawToChar(raw[at:(at + 3)]) != "<ch>") break
    len <- readBin(raw[(at + 4):(at + 7)], "integer", 1, size = 4, endian = info$endian)
    if (is.na(len) || len < 2 * nm || at + 7 + len > length(raw)) break
    body <- raw[(at + 8):(at + 7 + len)]
    owner <- text(body[seq_len(nm)])
    name <- text(body[nm + seq_len(nm)])
    value <- text(body[-seq_len(2 * nm)])
    Encoding(value) <- "UTF-8"
    if (owner == "_dta") {
      ch <- attr(d, "stata_char", exact = TRUE)
      ch[[name]] <- value
      attr(d, "stata_char") <- ch
    } else if (owner %in% names(d)) {
      ch <- attr(d[[owner]], "stata_char", exact = TRUE)
      ch[[name]] <- value
      attr(d[[owner]], "stata_char") <- ch
    }
    at <- at + 8 + len + 5
  }
  d
}
stata_chars_write <- function(path, d) {
  items <- list()
  for (v in c("_dta", names(d))) {
    ch <- if (identical(v, "_dta")) attr(d, "stata_char", exact = TRUE) else attr(d[[v]], "stata_char", exact = TRUE)
    for (n in names(ch)) items[[length(items) + 1]] <- list(v, n, ch[[n]])
  }
  if (!length(items)) return(invisible(FALSE))
  info <- tryCatch(stata_dta_chars_layout(path, whole = TRUE), error = function(e) NULL)
  if (is.null(info)) {
    warning("the characteristics could not be written into ", path, call. = FALSE)
    return(invisible(FALSE))
  }
  nm <- info$nm
  be <- info$endian
  num <- function(x, size) {
    b <- as.raw((x %/% 256^(0:(size - 1))) %% 256)
    if (be == "big") rev(b) else b
  }
  pad <- function(s) {
    b <- charToRaw(enc2utf8(s))
    c(b, raw(nm - length(b)))
  }
  parts <- list(charToRaw("<characteristics>"))
  for (it in items) {
    val <- c(charToRaw(enc2utf8(it[[3]])), as.raw(0))
    parts[[length(parts) + 1]] <- c(charToRaw("<ch>"), num(2 * nm + length(val), 4), pad(it[[1]]), pad(it[[2]]), val, charToRaw("</ch>"))
  }
  parts[[length(parts) + 1]] <- charToRaw("</characteristics>")
  new <- unlist(parts)
  raw <- info$raw
  off <- info$off
  delta <- length(new) - (off[10] - off[9])
  out <- c(raw[seq_len(off[9])], new, raw[(off[10] + 1):length(raw)])
  for (i in 10:14) {
    p <- info$map + 8 * (i - 1)
    out[p:(p + 7)] <- num(off[i] + delta, 8)
  }
  writeBin(out, path)
  invisible(TRUE)
}"##;

/// `ds`, `lookfor` and `compare`.
pub(super) const DS_HELPER: &str = r##"stata_ds <- function(d, vars = NULL, complement = FALSE, test = NULL, negate = FALSE, alpha = FALSE) {
  all <- names(d)
  sel <- if (is.null(vars)) all else vars
  if (complement) sel <- all[!all %in% sel]
  if (!is.null(test)) {
    keep <- vapply(sel, function(v) isTRUE(test(d[[v]])), NA)
    sel <- sel[if (negate) !keep else keep]
  }
  if (alpha) sel <- sort(sel, method = "radix")
  sel
}
stata_ds_show <- function(names, all) {
  if (length(names)) {
    v <- stata_tab_abbrev(names, 12, vars = all)
    w <- max(nchar(v)) + 2
    ls <- if (exists("stata_linesize")) stata_linesize else 79
    nr <- ceiling(length(v) / max(1, floor(ls / w)))
    for (i in seq_len(nr)) {
      s <- v[seq(i, length(v), by = nr)]
      cat(sub(" +$", "", paste0(s, strrep(" ", w - nchar(s)), collapse = "")), "\n", sep = "")
    }
  }
  invisible(NULL)
}
stata_lookfor <- function(d, words) {
  hit <- vapply(names(d), function(v) {
    lab <- attr(d[[v]], "label", exact = TRUE)
    if (is.null(lab)) lab <- ""
    any(vapply(words, function(w) grepl(tolower(w), tolower(v), fixed = TRUE) || grepl(tolower(w), tolower(lab), fixed = TRUE), NA))
  }, NA)
  names(d)[hit]
}
stata_lookfor_show <- function(d, names) {
  if (length(names)) {
    print(data.frame(variable = names, type = vapply(d[names], function(v) class(v)[1], ""), label = vapply(d[names], function(v) { l <- attr(v, "label", exact = TRUE); if (is.null(l)) "" else l }, "")), row.names = FALSE)
  }
  invisible(NULL)
}
stata_compare_vars <- function(d, a, b, show = TRUE) {
  for (v in c(a, b)) if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
  x <- d[[a]]
  y <- d[[b]]
  n <- nrow(d)
  if (is.character(x) != is.character(y)) stop("compare of a number with a string is not translated (R does not give Stata's counts)", call. = FALSE)
  chr <- is.character(x)
  if (chr) {
    xm <- is.na(x) | x == ""
    ym <- is.na(y) | y == ""
    lev <- sort(unique(c(x[!xm], y[!ym])), method = "radix")
    xi <- match(x, lev)
    yi <- match(y, lev)
  } else {
    x <- as.numeric(unclass(x))
    y <- as.numeric(unclass(y))
    xm <- is.na(x)
    ym <- is.na(y)
    xi <- x
    yi <- y
  }
  both <- !xm & !ym
  lt <- both & xi < yi
  eq <- both & xi == yi
  gt <- both & xi > yi
  cnt <- c(lt = sum(lt), eq = sum(eq), gt = sum(gt), joint = sum(both), xo = sum(xm & !ym), yo = sum(!xm & ym), neither = sum(xm & ym), total = n)
  if (show) {
    g <- function(v, w) formatC(trimws(stata_fmt(v, "%9.0g")), width = w)
    line <- function(label, count, stats = NULL) {
      cat(sprintf("%-24s", label), formatC(count, width = 9), sep = "")
      if (!chr && !is.null(stats)) cat(g(stats[1], 14), g(stats[2], 13), g(stats[3], 12), sep = "")
      cat("\n")
    }
    dash <- paste0(strrep(" ", 23), strrep("-", 10), "\n")
    st <- function(m) if (any(m)) { dd <- (x - y)[m]; c(min(dd), mean(dd), max(dd)) } else NULL
    cat("\n")
    if (chr) {
      cat(strrep(" ", 28), "count\n", strrep("-", 33), "\n", sep = "")
    } else {
      cat(strrep(" ", 40), "---------- Difference ----------\n", strrep(" ", 28), "Count       Minimum      Average     Maximum\n", strrep("-", 72), "\n", sep = "")
    }
    if (cnt[["lt"]] > 0) line(paste0(a, "<", b), cnt[["lt"]], st(lt))
    if (cnt[["eq"]] > 0) line(paste0(a, "=", b), cnt[["eq"]])
    if (cnt[["gt"]] > 0) line(paste0(a, ">", b), cnt[["gt"]], st(gt))
    if (cnt[["joint"]] > 0) {
      cat(dash)
      line("Jointly defined", cnt[["joint"]], st(both))
    }
    if (cnt[["xo"]] > 0) line(paste(a, "missing only"), cnt[["xo"]])
    if (cnt[["yo"]] > 0) line(paste(b, "missing only"), cnt[["yo"]])
    if (cnt[["neither"]] > 0) line("Jointly missing", cnt[["neither"]])
    cat(dash)
    line("Total", cnt[["total"]])
  }
  list(N = n)
}"##;

/// `misstable summarize` and `labelbook`.
pub(super) const MISSTABLE_HELPER: &str = r##"stata_misstable <- function(d, vars = NULL, all = FALSE, show = TRUE) {
  if (is.null(vars)) vars <- names(d)
  for (v in vars) if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
  rows <- list()
  for (v in vars) {
    x <- d[[v]]
    if (is.character(x)) next
    x <- as.numeric(unclass(x))
    miss <- is.na(x)
    ext <- miss & !is.na(haven::na_tag(x))
    n_gt <- sum(ext)
    n_eq <- sum(miss & !ext)
    if (!all && n_gt + n_eq == 0) next
    ok <- x[!miss]
    rows[[length(rows) + 1]] <- list(name = v, eq = n_eq, gt = n_gt, lt = length(ok), uniq = length(unique(ok)),
      min = if (length(ok)) min(ok) else NA_real_, max = if (length(ok)) max(ok) else NA_real_)
  }
  if (!length(rows)) {
    if (show) cat("(variables nonmissing or string)\n")
    return(list(K_uniq = NA_real_, N_gt_dot = NA_real_, N_eq_dot = NA_real_, max = NA_real_, min = NA_real_, N_lt_dot = NA_real_, vartype = "none"))
  }
  if (show) {
    num <- function(n) if (n == 0) strrep(" ", 10) else formatC(n, width = 10)
    fm <- function(v, w) formatC(trimws(stata_fmt(v, "%9.0g")), width = w)
    cat(strrep(" ", 63), "Obs<.\n", strrep(" ", 48), "+", strrep("-", 30), "\n", strrep(" ", 15), "|", strrep(" ", 32), "| Unique\n", sep = "")
    cat("      Variable |     Obs=.     Obs>.     Obs<.  | values        Min         Max\n")
    cat("  ", strrep("-", 13), "+", strrep("-", 32), "+", strrep("-", 30), "\n", sep = "")
    for (r in rows) {
      nm <- stata_tab_abbrev(r$name, 12, vars = names(d))
      u <- if (r$uniq > 500) ">500" else as.character(r$uniq)
      cat(sprintf("%14s |", nm), num(r$eq), num(r$gt), formatC(r$lt, width = 10), "  |", formatC(u, width = 7), fm(r$min, 11), fm(r$max, 12), "\n", sep = "")
    }
    cat("  ", strrep("-", 77), "\n", sep = "")
  }
  r <- rows[[length(rows)]]
  list(K_uniq = r$uniq, N_gt_dot = r$gt, N_eq_dot = r$eq, max = r$max, min = r$min, N_lt_dot = r$lt, vartype = "numeric")
}
stata_labelbook <- function(d, sets, problems = FALSE, len = 12, show = TRUE) {
  nms <- sort(names(sets), method = "radix")
  vars <- names(d)
  using <- lapply(nms, function(s) {
    hit <- vapply(vars, function(v) identical(attr(d[[v]], "vallab", exact = TRUE), s), NA)
    vars[hit]
  })
  names(using) <- nms
  for (v in vars) {
    col <- d[[v]]
    if (!is.null(attr(col, "labels", exact = TRUE)) && is.null(attr(col, "vallab", exact = TRUE))) {
      stop("labelbook: the variable ", v, " has value labels whose set name R does not keep (they came with a data file), so which variables use each set is not known", call. = FALSE)
    }
  }
  info <- lapply(nms, function(s) {
    x <- sets[[s]]
    vals <- as.numeric(unclass(x))
    labs <- names(x)
    if (any(labs == "")) stop("labelbook: the value label ", s, " has a null string label, which is not translated yet", call. = FALSE)
    list(vals = vals, labs = labs,
      gaps = length(vals) > 0 && any(diff(sort(vals)) > 1),
      numeric = any(grepl("^[+-]?([0-9]+[.]?[0-9]*|[.][0-9]+)([eE][+-]?[0-9]+)?$", labs)),
      blanks = any(grepl("^ | $", labs)),
      nuniq = anyDuplicated(labs) > 0,
      nuniq_sh = anyDuplicated(substr(labs, 1, len)) > 0,
      notused = length(using[[s]]) == 0)
  })
  names(info) <- nms
  flag <- function(k) nms[vapply(info, function(i) isTRUE(i[[k]]), NA)]
  r <- list()
  for (k in c("notused", "nuniq_sh", "nuniq", "blanks", "numeric", "gaps")) {
    f <- flag(k)
    if (length(f)) r[[k]] <- paste(f, collapse = " ")
  }
  r[["names"]] <- paste(nms, collapse = " ")
  if (show && problems) {
    rows <- list(c("numeric", "Numeric -> numeric"), c("gaps", "Gaps in mapped values"), c("blanks", "Leading or trailing blanks"),
      c("nuniq", "Duplicate labels"), c("nuniq_sh", paste("Duplicate lab. at length", len)), c("notused", "Not used by variables"))
    any_problem <- any(vapply(rows, function(p) length(flag(p[1])) > 0, NA))
    if (!any_problem) {
      cat("\nno potential problems in dataset [unnamed]\n\n")
    } else {
      cat("\n   Potential problems in dataset   [unnamed]\n\n", sprintf("%32s   Value labels\n", "Potential problem"), strrep("-", 50), "\n", sep = "")
      for (p in rows) {
        f <- flag(p[1])
        if (length(f)) cat(sprintf("%32s   %s\n", p[2], paste(f, collapse = " ")))
      }
      cat(strrep("-", 50), "\n\n", sep = "")
    }
  } else if (show) {
    yn <- function(b) if (b) "yes" else "no"
    for (s in nms) {
      i <- info[[s]]
      lens <- nchar(i$labs, "bytes")
      rows <- list(
        c("Range:", paste0("[", format(min(i$vals), trim = TRUE), ",", format(max(i$vals), trim = TRUE), "]"), "String length:", paste0("[", min(lens), ",", max(lens), "]")),
        c("N:", length(i$vals), "Unique at full length:", yn(!i$nuniq)),
        c("Gaps:", yn(i$gaps), paste0("Unique at length ", len, ":"), yn(!i$nuniq_sh)),
        c("Missing .*:", "0", "Null string:", "no"),
        c("", "", "Leading/trailing blanks:", yn(i$blanks)),
        c("", "", "Numeric -> numeric:", yn(i$numeric)))
      cat("\n", strrep("-", 79), "\nValue label ", s, " \n", strrep("-", 79), "\n\n", sep = "")
      cat("      Values                                    Labels\n")
      for (p in rows) cat(sprintf("%-29s%26s  %s\n", if (nzchar(p[1])) sprintf("%13s  %s", p[1], p[2]) else "", p[3], p[4]))
      cat("  Definition\n")
      o <- order(i$vals)
      for (k in o) cat(sprintf("%12s   %s\n", format(i$vals[k], trim = TRUE), sub("^ +", "", i$labs[k])))
      cat("\n   Variables:  ", paste(using[[s]], collapse = " "), "\n\n", sep = "")
    }
    cat("\n")
  }
  r
}"##;

/// What follows the command word, as typed (a prefix such as `quietly` comes
/// before it; the colon of `note: text` is kept).
fn text_after(c: &Command) -> String {
    let t = c.text.as_str();
    let w = c.written.as_str();
    let mut from = 0;
    while let Some(i) = t.get(from..).and_then(|s| s.find(w)) {
        let s = from + i;
        let e = s + w.len();
        let before_ok = s == 0
            || t.get(..s)
                .is_some_and(|b| b.ends_with(|ch: char| ch.is_whitespace() || ch == ':'));
        let after_ok = t
            .get(e..)
            .and_then(|a| a.chars().next())
            .is_none_or(|ch| !(ch.is_alphanumeric() || ch == '_'));
        if before_ok && after_ok {
            return t.get(e..).unwrap_or("").trim().to_string();
        }
        from = e;
    }
    String::new()
}

/// A characteristic's name: letters, digits and underscores, up to 32.
fn is_char_name(s: &str) -> bool {
    !s.is_empty() && s.len() <= 32 && s.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

/// `x[name]` or `_dta[name]`: the owner, the characteristic's name (empty for
/// `x[]`) and what follows the bracket.
fn owner_and_char(text: &str) -> Option<(&str, &str, &str)> {
    let text = text.trim_start();
    let open = text.find('[')?;
    let close = text.find(']')?;
    if close < open {
        return None;
    }
    let owner = text.get(..open)?;
    let name = text.get(open + 1..close)?;
    let after = text.get(close + 1..)?;
    (owner == "_dta" || is_name(owner)).then_some((owner, name, after))
}

/// Text for a characteristic or note as Stata stores it: a pair of quotes
/// around a characteristic's text is taken off; other quotes, and the
/// backtick, are refused.
fn char_text(text: &str, quotes_kept: bool) -> Result<String, Untranslated> {
    let t = text.trim();
    let t = if !quotes_kept && t.len() >= 2 && t.starts_with('"') && t.ends_with('"') {
        t.get(1..t.len() - 1).unwrap_or("")
    } else {
        t
    };
    if t.contains(['"', '`']) {
        return Err(no(
            "a characteristic or note with quotes inside is not translated yet",
        ));
    }
    Ok(t.to_string())
}

/// `1`, `2/4`, `1 3`: the numbers of a note list, as R.
fn note_numbers(text: &str) -> Result<String, Untranslated> {
    let bad = || {
        no(format!(
            "the note numbers {text} are not translated (numbers or ranges like 1/3)"
        ))
    };
    let mut out: Vec<i64> = Vec::new();
    for tok in text.split([' ', ',']).filter(|t| !t.is_empty()) {
        if let Some((a, b)) = tok.split_once('/') {
            let a: i64 = a.parse().map_err(|_| bad())?;
            let b: i64 = b.parse().map_err(|_| bad())?;
            if a < 0 || b < a || b - a > 10_000 {
                return Err(bad());
            }
            out.extend(a..=b);
        } else {
            out.push(tok.parse::<i64>().map_err(|_| bad())?);
        }
    }
    if out.is_empty() {
        return Err(no("a note list needs numbers after in"));
    }
    Ok(format!(
        "c({})",
        out.iter()
            .map(|n| format!("{n}L"))
            .collect::<Vec<_>>()
            .join(", ")
    ))
}

fn stop_line(out: &mut Out, msg: &str) {
    out.line(format!("stop({}, call. = FALSE)", r_text(msg)));
}

const TS_NOTE: &str = "a note with TS (Stata puts the date and time there) is not translated: the stamp is the time the line runs";

impl Session {
    /// Why the characteristics are not known here, if they are not.
    fn chars_check(&self, what: &str) -> Result<(), Untranslated> {
        match &self.chars_lost {
            Some(why) => Err(no(format!(
                "{what}: the characteristics are not known here ({why})"
            ))),
            None => Ok(()),
        }
    }

    /// `char`: set, remove and list characteristics.
    pub(super) fn char_cmd(&mut self, c: &Command, out: &mut Out) -> Result<(), Untranslated> {
        let r = self.char_inner(c, out);
        if let Err(e) = &r {
            self.chars_lost = Some(format!(
                "{} was not translated: {}",
                c.text.trim(),
                e.reason
            ));
        }
        r
    }

    fn char_inner(&mut self, c: &Command, out: &mut Out) -> Result<(), Untranslated> {
        let d = self.data()?;
        self.chars_check("char")?;
        let rest = text_after(c);
        let (w1, tail) = first_word(&rest);
        let syntax = |out: &mut Out| stop_line(out, "invalid syntax (Stata: r(198))");
        let define = match w1 {
            "list" => {
                if self.chars_reserved_now() {
                    return Err(no(
                        "char list after tsset, xtset, stset, svyset or fvset: Stata also lists the characteristics those commands set, which R does not hold",
                    ));
                }
                let (owners, name) = if tail.is_empty() {
                    ("NULL".to_string(), "NULL".to_string())
                } else if let Some((o, n, after)) = owner_and_char(tail) {
                    if !after.trim().is_empty() || !(n.is_empty() || is_char_name(n)) {
                        self.helpers.insert("stata_char_var");
                        out.line(format!("invisible(stata_char_var({d}, {}))", r_text(o)));
                        syntax(out);
                        return Ok(());
                    }
                    (
                        r_text(o),
                        if n.is_empty() {
                            "NULL".into()
                        } else {
                            r_text(n)
                        },
                    )
                } else {
                    syntax(out);
                    return Ok(());
                };
                self.helpers.insert("stata_char_var");
                // quietly or under capture it is still run, for the error it may give
                let call = format!("stata_char_list({d}, {owners}, {name})");
                if self.quiet(c) {
                    out.line(format!("invisible(utils::capture.output({call}))"));
                } else {
                    out.line(call);
                }
                return Ok(());
            }
            "rename" | "dir" | "drop" => {
                return Err(no(format!("char {w1} is not translated yet")));
            }
            "define" => tail,
            _ => rest.as_str(),
        };
        let Some((owner, name, after)) = owner_and_char(define) else {
            return Err(no("char: the form is not translated (char var[name] text)"));
        };
        if !is_char_name(name) {
            // the variable is looked for first (r(111)), then the syntax is wrong (r(198))
            self.helpers.insert("stata_char_var");
            out.line(format!("invisible(stata_char_var({d}, {}))", r_text(owner)));
            syntax(out);
            return Ok(());
        }
        let text = char_text(after, false)?;
        self.helpers.insert("stata_char_var");
        out.line(format!(
            "{d} <- stata_char_put({d}, {}, {}, {})",
            r_text(owner),
            r_text(name),
            r_text(&text)
        ));
        Ok(())
    }

    /// Whether a command since set characteristics of its own (`_TS...` after
    /// `tsset`, `_fv_...` after `fvset`, ... after `stset`), which a listing
    /// would show and R does not hold.
    pub(super) fn chars_reserved_now(&self) -> bool {
        self.panel.is_some()
            || self.panel_time.is_some()
            || self.ts_delta.is_some()
            || self.stset.is_some()
            || !self.fv_base.is_empty()
            || self.svy_design.is_some()
    }

    /// The owners an evarlist names, as R: `_dta`, `_all`, variables.
    fn note_owners(&self, d: &str, words: &[&str]) -> Result<String, Untranslated> {
        if words.is_empty() {
            return Ok(format!("c(\"_dta\", names({d}))"));
        }
        let mut parts = Vec::new();
        for w in words {
            match *w {
                "_dta" => parts.push("\"_dta\"".to_string()),
                "_all" => parts.push(format!("c(\"_dta\", names({d}))")),
                w if is_name(w) => parts.push(r_text(w)),
                w => parts.push(format!("names(dplyr::select({d}, {}))", select(w)?)),
            }
        }
        Ok(format!("c({})", parts.join(", ")))
    }

    /// `note` and `notes`: add, list, drop, replace, search and renumber.
    pub(super) fn notes_cmd(&mut self, c: &Command, out: &mut Out) -> Result<(), Untranslated> {
        let r = self.notes_inner(c, out);
        if let Err(e) = &r {
            self.chars_lost = Some(format!(
                "{} was not translated: {}",
                c.text.trim(),
                e.reason
            ));
        }
        r
    }

    fn notes_inner(&mut self, c: &Command, out: &mut Out) -> Result<(), Untranslated> {
        let d = self.data()?;
        self.chars_check("notes")?;
        let text = text_after(c);
        let rest = text.trim();
        let quiet = self.quiet(c);
        self.helpers.insert("stata_char_var");
        out.package("dplyr");
        // a call that prints, quietly or not; the data it returns are kept
        let call = |out: &mut Out, f: &str, args: &str, keeps: bool| {
            let line = if keeps {
                format!("{d} <- {f}({d}, {args})")
            } else {
                format!("{f}({d}, {args})")
            };
            if quiet {
                out.line(format!("invisible(utils::capture.output({line}))"));
            } else {
                out.line(line);
            }
        };
        let (w1, tail) = first_word(rest);
        let colon = |s: &str| s.trim_start().starts_with(':');
        // `x y in 1/2`: the words before `in` and the numbers after it
        let split_in = |s: &str| -> (String, Option<String>) {
            let words: Vec<&str> = s.split_whitespace().collect();
            match words.iter().position(|w| *w == "in") {
                Some(i) => (
                    words.get(..i).unwrap_or(&[]).join(" "),
                    Some(words.get(i + 1..).unwrap_or(&[]).join(" ")),
                ),
                None => (words.join(" "), None),
            }
        };
        match w1 {
            "drop" if !colon(tail) => {
                let (head, nums) = split_in(tail);
                let words: Vec<&str> = head.split_whitespace().collect();
                if words.is_empty() {
                    return Err(no("notes drop needs variables, _dta or _all"));
                }
                let owners = self.note_owners(&d, &words)?;
                let nums = match nums {
                    Some(n) => note_numbers(&n)?,
                    None => "NULL".into(),
                };
                call(out, "stata_note_drop", &format!("{owners}, {nums}"), true);
            }
            "renumber" if !colon(tail) => {
                let words: Vec<&str> = tail.split_whitespace().collect();
                if words.is_empty() {
                    return Err(no("notes renumber needs variables, _dta or _all"));
                }
                let owners = self.note_owners(&d, &words)?;
                call(out, "stata_note_renumber", &owners, true);
            }
            "replace" if !colon(tail) => {
                let Some((head, text)) = tail.split_once(':') else {
                    return Err(no("notes replace needs : and the new text"));
                };
                let (who, nums) = split_in(head);
                let who: Vec<&str> = who.split_whitespace().collect();
                let (Some(owner), Some(nums)) = (who.first().filter(|_| who.len() == 1), nums)
                else {
                    return Err(no(
                        "notes replace: the form is notes replace var in # : text",
                    ));
                };
                let k: i64 = nums
                    .trim()
                    .parse()
                    .map_err(|_| no("notes replace: a single note number is translated"))?;
                let text = char_text(text, true)?;
                if text.split_whitespace().any(|w| w == "TS") {
                    return Err(no(TS_NOTE));
                }
                call(
                    out,
                    "stata_note_replace",
                    &format!("{}, {k}L, {}", r_text(owner), r_text(&text)),
                    true,
                );
            }
            "search" if !colon(tail) => {
                let t = tail.trim();
                let t = t
                    .strip_prefix('"')
                    .and_then(|s| s.strip_suffix('"'))
                    .unwrap_or(t);
                if t.is_empty() || t.contains(['"', '`']) || t.split_whitespace().count() > 1 {
                    return Err(no(
                        "notes search with several words or quotes is not translated",
                    ));
                }
                call(
                    out,
                    "stata_note_show",
                    &format!("c(\"_dta\", names({d})), NULL, {}", r_text(t)),
                    false,
                );
            }
            _ => {
                // `note: text`, `note x: text`, or a listing
                if let Some(i) = rest.find(':') {
                    let head = rest.get(..i).unwrap_or("").trim();
                    let text = char_text(rest.get(i + 1..).unwrap_or(""), true)?;
                    if text.split_whitespace().any(|w| w == "TS") {
                        return Err(no(TS_NOTE));
                    }
                    let owner = if head.is_empty() {
                        "_dta"
                    } else if head.split_whitespace().count() == 1
                        && (head == "_dta" || is_name(head))
                    {
                        head
                    } else {
                        return Err(no("note: the form is note [varname]: text"));
                    };
                    call(
                        out,
                        "stata_note_add",
                        &format!("{}, {}", r_text(owner), r_text(&text)),
                        true,
                    );
                } else {
                    // notes [list] [evarlist] [in numlist]
                    let body = if w1 == "list" { tail } else { rest };
                    let (head, nums) = split_in(body);
                    let words: Vec<&str> = head.split_whitespace().collect();
                    let owners = self.note_owners(&d, &words)?;
                    let nums = match nums {
                        Some(n) => note_numbers(&n)?,
                        None => "NULL".into(),
                    };
                    call(out, "stata_note_show", &format!("{owners}, {nums}"), false);
                }
            }
        }
        Ok(())
    }
}

/// What `labelbook` leaves in r() (text, each only when it has a name).
pub(super) const R_LABELBOOK: &[&str] = &[
    "names", "gaps", "notused", "numeric", "blanks", "nuniq", "nuniq_sh",
];
/// What `ds` and `lookfor` leave in r().
pub(super) const R_VARLIST: &[&str] = &["varlist"];
/// What `misstable summarize` leaves in r() (the last variable it showed).
pub(super) const R_MISSTABLE: &[&str] = &["vartype"];

/// The words of a `lookfor`, quoted phrases as one.
fn lookfor_words(text: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut rest = text.trim();
    while !rest.is_empty() {
        if let Some(q) = rest.strip_prefix('"') {
            let end = q.find('"').unwrap_or(q.len());
            words.push(q.get(..end).unwrap_or("").to_string());
            rest = q.get(end + 1..).unwrap_or("").trim_start();
        } else {
            let (w, next) = first_word(rest);
            words.push(w.to_string());
            rest = next;
        }
    }
    words.retain(|w| !w.is_empty());
    words
}

impl Session {
    /// The data restricted by `if` and `in`, as R.
    fn rows_expr(&self, c: &Command, out: &mut Out) -> Result<String, Untranslated> {
        let d = self.data()?;
        Ok(match self.condition(c, out)? {
            Some(cond) => {
                out.package("dplyr");
                format!("dplyr::filter({d}, {cond})")
            }
            None => d,
        })
    }

    /// `labelbook [names] [, problems length(#)]`.
    pub(super) fn labelbook(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let d = self.data()?;
        if let Some(o) = opts.other(&[("problems", 4), ("length", 3)]) {
            return Err(no(format!(
                "labelbook with the option {o} is not translated yet"
            )));
        }
        if c.parts.if_exp.is_some() || c.parts.in_range.is_some() || c.parts.weight.is_some() {
            return Err(no("labelbook takes label names and options only"));
        }
        if self.ts_unknown {
            return Err(no(
                "labelbook after a data file was read: its value label sets are not known here (R does not keep their names)",
            ));
        }
        let given: Vec<&str> = c.parts.main.split_whitespace().collect();
        for n in &given {
            if !self.label_sets.contains_key(*n) {
                stop_line(out, &format!("value label {n} not found (Stata: r(111))"));
                return Ok(());
            }
        }
        let mut chosen: Vec<String> = if given.is_empty() {
            self.label_sets.keys().cloned().collect()
        } else {
            given.iter().map(|g| g.to_string()).collect()
        };
        chosen.sort();
        chosen.dedup();
        if chosen.is_empty() {
            return Err(no(
                "labelbook with no value label defined in this do-file is not translated",
            ));
        }
        let len: usize = match opts.arg("length", 3) {
            Some(a) => a
                .trim()
                .parse()
                .map_err(|_| no("labelbook length() needs a number"))?,
            None => 12,
        };
        let problems = opts.has("problems", 4);
        if problems && len != 12 {
            return Err(no(
                "labelbook, problems with length() is not translated yet",
            ));
        }
        let sets: Vec<String> = chosen
            .iter()
            .map(|n| format!("{} = {}", r_text(n), label_object(n)))
            .collect();
        for h in ["stata_misstable", "stata_fmt"] {
            self.helpers.insert(h);
        }
        let show = if self.quiet(c) { "FALSE" } else { "TRUE" };
        out.line(format!(
            "stata_r <- stata_labelbook({d}, list({}), problems = {}, len = {len}L, show = {show})",
            sets.join(", "),
            if problems { "TRUE" } else { "FALSE" }
        ));
        self.r_new = Some((R_LABELBOOK, "labelbook".into()));
        Ok(())
    }

    /// `compare var1 var2 [if] [in]`.
    pub(super) fn compare(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[]) {
            return Err(no(format!(
                "compare with the option {o} is not translated yet"
            )));
        }
        if c.parts.weight.is_some() {
            stop_line(out, "weights not allowed (Stata: r(101))");
            return Ok(());
        }
        let words: Vec<&str> = c.parts.main.split_whitespace().collect();
        match words.as_slice() {
            [] => return Err(no("compare needs two variables (Stata: r(100))")),
            [_] => {
                stop_line(out, "too few variables specified (Stata: r(102))");
                return Ok(());
            }
            [a, b] if is_name(a) && is_name(b) => {
                let rows = self.rows_expr(c, out)?;
                for h in ["stata_ds", "stata_fmt"] {
                    self.helpers.insert(h);
                }
                let show = if self.quiet(c) { "FALSE" } else { "TRUE" };
                out.line(format!(
                    "stata_r <- stata_compare_vars({rows}, {}, {}, show = {show})",
                    r_text(a),
                    r_text(b)
                ));
                self.r_new = Some((returned::R_COUNT, "compare".into()));
            }
            [_, _] => {
                return Err(no(
                    "compare of names that are not plain variable names is not translated",
                ))
            }
            _ => {
                stop_line(out, "too many variables specified (Stata: r(103))");
            }
        }
        Ok(())
    }

    /// `ds [varlist] [, not has(type string|numeric|varlabel|vallab) not(...) alpha]`.
    pub(super) fn ds(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let d = self.data()?;
        if let Some(o) = opts.other(&[("has", 3), ("not", 3), ("alpha", 1)]) {
            return Err(no(format!("ds with the option {o} is not translated yet")));
        }
        if c.parts.if_exp.is_some() || c.parts.in_range.is_some() || c.parts.weight.is_some() {
            return Err(no("ds takes a varlist and options only"));
        }
        let find = |full: &str, min: usize| c.options.iter().find(|o| o.is(full, min));
        let has = find("has", 3);
        let not = find("not", 3);
        // `not` alone leaves out the varlist; has() and not() choose by a property
        let complement = not.is_some_and(|o| o.arg.is_none());
        let (spec, negate) = match (
            has.and_then(|o| o.arg.as_deref()),
            not.and_then(|o| o.arg.as_deref()),
        ) {
            (Some(_), Some(_)) => {
                stop_line(
                    out,
                    "may not combine options has() and not() (Stata: r(198))",
                );
                return Ok(());
            }
            (Some(s), None) => (Some(s.trim().to_string()), false),
            (None, Some(s)) => (Some(s.trim().to_string()), true),
            (None, None) => (None, false),
        };
        // the property: type string|numeric, varlabel [pattern], vallab (words may be shortened)
        let (kind, arg) = match spec.as_deref() {
            Some(s) => {
                let (w, a) = first_word(s);
                let kind = if w == "type" {
                    "type"
                } else if w.len() >= 4 && "varlabel".starts_with(w) {
                    "varlabel"
                } else if w.len() >= 4 && "vallabel".starts_with(w) {
                    "vallab"
                } else {
                    ""
                };
                (kind, a.to_string())
            }
            None => ("", String::new()),
        };
        let test = match (spec.as_deref(), kind, arg.as_str()) {
            (None, _, _) => "NULL".to_string(),
            (_, "type", "string") => "is.character".to_string(),
            (_, "type", "numeric") => "function(x) !is.character(x)".to_string(),
            (_, "varlabel", "") => "function(x) { l <- attr(x, \"label\", exact = TRUE); !is.null(l) && nzchar(l) }".to_string(),
            (_, "vallab", "") => "function(x) !is.null(attr(x, \"labels\", exact = TRUE))".to_string(),
            // a variable label that matches the pattern as a whole: * any text, ? one
            // character, the rest as it stands (capitals matter)
            (_, "varlabel", pat) => {
                let pat = pat
                    .strip_prefix('"')
                    .and_then(|p| p.strip_suffix('"'))
                    .unwrap_or(pat);
                if pat.contains(['"', '`']) {
                    return Err(no("ds has(varlabel) with a quote inside its pattern is not translated"));
                }
                let mut re = String::from("^");
                for ch in pat.chars() {
                    match ch {
                        '*' => re.push_str(".*"),
                        '?' => re.push('.'),
                        c if c.is_alphanumeric() || c == ' ' || c == '_' => re.push(c),
                        c => {
                            re.push('\\');
                            re.push(c);
                        }
                    }
                }
                re.push('$');
                format!(
                    "function(x) {{ l <- attr(x, \"label\", exact = TRUE); !is.null(l) && grepl({}, l) }}",
                    r_text(&re)
                )
            }
            (Some(other), _, _) => {
                return Err(no(format!(
                    "ds with has({other}) or not({other}) is not translated: only type string, type numeric, varlabel and vallab are (R does not keep Stata's storage types, display formats or the names of value labels)"
                )))
            }
        };
        let main = c.parts.main.trim();
        check_names(&d, main, out);
        let vars = if main.is_empty() {
            "NULL".to_string()
        } else {
            out.package("dplyr");
            format!("names(dplyr::select({d}, {}))", select(main)?)
        };
        for h in ["stata_ds", "stata_tables", "stata_fmt"] {
            self.helpers.insert(h);
        }
        out.line(format!(
            "stata_ds_names <- stata_ds({d}, {vars}, complement = {}, test = {test}, negate = {}, alpha = {})",
            super::stats::r_logical(complement),
            super::stats::r_logical(negate),
            super::stats::r_logical(opts.has("alpha", 1))
        ));
        out.line("stata_r <- if (length(stata_ds_names)) list(varlist = paste(stata_ds_names, collapse = \" \")) else list()");
        if !self.quiet(c) {
            out.line(format!("stata_ds_show(stata_ds_names, names({d}))"));
        }
        out.line("rm(stata_ds_names)");
        self.r_new = Some((R_VARLIST, "ds".into()));
        Ok(())
    }

    /// `lookfor word ...`: the variables whose name or label holds one of the words.
    pub(super) fn lookfor(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let d = self.data()?;
        if let Some(o) = opts.other(&[]) {
            return Err(no(format!(
                "lookfor with the option {o} is not translated yet"
            )));
        }
        if c.parts.if_exp.is_some() || c.parts.in_range.is_some() || c.parts.weight.is_some() {
            return Err(no("lookfor takes words only"));
        }
        let words = lookfor_words(&c.parts.main);
        if words.is_empty() {
            stop_line(out, "nothing to look for (Stata: r(198))");
            return Ok(());
        }
        let list: Vec<String> = words.iter().map(|w| r_text(w)).collect();
        self.helpers.insert("stata_ds");
        out.line(format!(
            "stata_look_names <- stata_lookfor({d}, c({}))",
            list.join(", ")
        ));
        out.line("stata_r <- if (length(stata_look_names)) list(varlist = paste(stata_look_names, collapse = \" \")) else list()");
        if !self.quiet(c) {
            out.line(format!("stata_lookfor_show({d}, stata_look_names)"));
        }
        out.line("rm(stata_look_names)");
        self.r_new = Some((R_VARLIST, "lookfor".into()));
        Ok(())
    }

    /// `misstable summarize [varlist] [if] [in] [, all]`.
    pub(super) fn misstable(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let d = self.data()?;
        let (sub, rest) = first_word(&c.parts.main);
        if sub.len() < 3 || !"summarize".starts_with(sub) {
            return Err(no(format!(
                "misstable {sub} is not translated yet (only misstable summarize)"
            )));
        }
        if let Some(o) = opts.other(&[("all", 3)]) {
            return Err(no(format!(
                "misstable summarize with the option {o} is not translated yet"
            )));
        }
        if c.parts.weight.is_some() {
            return Err(no("misstable summarize with weights is not translated"));
        }
        let rows = self.rows_expr(c, out)?;
        check_names(&d, rest, out);
        let vars = if rest.is_empty() {
            "NULL".to_string()
        } else {
            out.package("dplyr");
            format!("names(dplyr::select({d}, {}))", select(rest)?)
        };
        out.package("haven");
        for h in ["stata_misstable", "stata_fmt", "stata_tables"] {
            self.helpers.insert(h);
        }
        let show = if self.quiet(c) { "FALSE" } else { "TRUE" };
        out.line(format!(
            "stata_r <- stata_misstable({rows}, {vars}, all = {}, show = {show})",
            super::stats::r_logical(opts.has("all", 3))
        ));
        self.r_new = Some((R_MISSTABLE, "misstable".into()));
        Ok(())
    }
}

/// Stata's r(111) for a variable written out that is not there (a pattern or
/// range is looked up by R).
fn check_names(d: &str, varlist: &str, out: &mut Out) {
    for v in varlist.split_whitespace().filter(|v| is_name(v)) {
        out.line(format!(
            "if (!{} %in% names({d})) stop({}, call. = FALSE)",
            r_text(v),
            r_text(&format!("variable {v} not found (Stata: r(111))"))
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn first(text: &str) -> Command {
        let program = crate::reader::read(text);
        program
            .commands()
            .first()
            .map(|c| (*c).clone())
            .expect("one command")
    }

    #[test]
    fn the_text_after_the_command_word_keeps_its_colon_and_commas() {
        assert_eq!(
            text_after(&first("note: Made up, if in.")),
            ": Made up, if in."
        );
        assert_eq!(text_after(&first("note x: a, b")), "x: a, b");
        assert_eq!(text_after(&first("capture noisily char x[a] 1")), "x[a] 1");
        assert_eq!(text_after(&first("notes")), "");
    }

    #[test]
    fn owners_and_characteristic_names() {
        assert_eq!(owner_and_char("x[a] hi"), Some(("x", "a", " hi")));
        assert_eq!(owner_and_char("_dta[note0]"), Some(("_dta", "note0", "")));
        assert_eq!(owner_and_char("x[]"), Some(("x", "", "")));
        assert_eq!(owner_and_char("1x[a]"), None);
        assert_eq!(owner_and_char("list"), None);
        assert!(is_char_name("note1"));
        assert!(!is_char_name("a b"));
        assert!(!is_char_name(""));
    }

    #[test]
    fn characteristic_text_loses_its_outer_quotes_only() {
        assert_eq!(
            char_text(" \"two words\" ", false).ok().as_deref(),
            Some("two words")
        );
        assert_eq!(char_text("a, b", false).ok().as_deref(), Some("a, b"));
        assert!(char_text("say \"hi\" now", false).is_err());
        assert!(char_text("`x'", false).is_err());
    }

    #[test]
    fn note_numbers_are_a_list_of_numbers_and_ranges() {
        assert_eq!(note_numbers("1/3").ok().as_deref(), Some("c(1L, 2L, 3L)"));
        assert_eq!(note_numbers("2 5").ok().as_deref(), Some("c(2L, 5L)"));
        assert!(note_numbers("a").is_err());
        assert!(note_numbers("3/1").is_err());
        assert!(note_numbers("").is_err());
    }

    #[test]
    fn lookfor_takes_quoted_phrases_as_one_word() {
        assert_eq!(lookfor_words("the x"), vec!["the", "x"]);
        assert_eq!(lookfor_words("\"the x\" y"), vec!["the x", "y"]);
        assert!(lookfor_words("  ").is_empty());
    }
}
