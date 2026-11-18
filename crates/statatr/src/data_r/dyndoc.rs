//! The R for `markdown`, `dyntext` and `dyndoc` (P2C.12x, batch D of the
//! StatCraft port; `plan/stata/dyndoc.md`). `markdown src, saving(file)
//! [replace]` is translated here; `dyntext` and `dyndoc` are run by the do-file
//! runner (`dofile/dyndoc.rs`), whose lines call the helpers below.
//!
//! Markdown becomes HTML as Stata 19 writes it (StatCraft's rules, checked on
//! probes 546 and 547): only the body, one block a line; headings with an
//! anchor named after their text; the typographer's quotes, dashes, ellipses
//! and angle quotes as entities; tight and loose lists, indented and fenced
//! code, quotes, rules, HTML blocks, pipe tables (cells keep their spaces),
//! code spans, emphasis, links, images, autolinks and backslash escapes.

use super::*;

pub(super) const DYNDOC_HELPER: &str = r##"stata_dd_cr <- function() if (.Platform$OS.type == "windows") "\r" else ""
stata_dd_full <- function(f) normalizePath(if (grepl("^([A-Za-z]:)?[/\\]|^~", f)) f else file.path(getwd(), f), winslash = "/", mustWork = FALSE)
stata_dd_text <- function(f) {
  x <- readChar(f, file.info(f)$size, useBytes = TRUE)
  Encoding(x) <- "UTF-8"
  x
}
# what dyntext and dyndoc read, before any tag runs: the source not there (r(601)) or
# the target there without replace (r(602), its full path), else the source's text
stata_dd_check <- function(src, target, replace) {
  if (!file.exists(src) || dir.exists(src)) return(cat("601\n"))
  if (file.exists(target) && !replace) return(cat("602 ", stata_dd_full(target), "\n", sep = ""))
  cat("ok\n", stata_dd_text(src), sep = "")
}
stata_dd_read <- function(f) {
  if (!file.exists(f) || dir.exists(f)) return(cat("601\n"))
  cat("ok\n", stata_dd_text(f), sep = "")
}
stata_dd_begin <- function() {
  stata_dd <<- character()
  invisible(NULL)
}
# a line of the document; what a dd_do command prints ends in a carriage return on
# Windows, as in the files Stata 19 writes there
stata_dd_put <- function(x, cr = FALSE) {
  stata_dd <<- c(stata_dd, paste0(x, if (cr) stata_dd_cr() else ""))
  invisible(NULL)
}
# what a command printed: a blank line at its end is the one between commands in a log,
# which the closing blank line of the document already is
stata_dd_out <- function(lines) {
  while (length(lines) && !nzchar(lines[length(lines)])) lines <- lines[-length(lines)]
  if (length(lines)) stata_dd <<- c(stata_dd, paste0(lines, stata_dd_cr()))
  invisible(NULL)
}
stata_dd_unremove <- function(x) gsub("<<dd_remove>>.*?(<</dd_remove>>|$)", "", x, perl = TRUE)
# <<dd_graph>>: the graph in memory as SVG (Graph.svg, or Graph1.svg ... when it is there)
# and the <img> tag for it
stata_dd_graph <- function(saving = "", replace = FALSE, alt = "", height = "", width = "") {
  if (!exists("stata_graph")) stop("no graph in memory (Stata: r(111))", call. = FALSE)
  f <- saving
  if (!nzchar(f)) {
    k <- 0
    repeat {
      f <- if (k == 0) "Graph.svg" else paste0("Graph", k, ".svg")
      if (replace || !file.exists(f)) break
      k <- k + 1
    }
  } else if (file.exists(f) && !replace) {
    stop("RCraft: dd_graph to a file that is there, without replace, is not translated yet", call. = FALSE)
  }
  grDevices::svg(f)
  dev <- grDevices::dev.cur()
  tryCatch(print(stata_graph), finally = grDevices::dev.off(dev))
  paste0("<img src=\"", f, "\"", if (nzchar(width)) paste0(" width=\"", width, "\""),
         if (nzchar(height)) paste0(" height=\"", height, "\""), if (nzchar(alt)) paste0(" alt=\"", alt, "\""), " >")
}
# the document printed as it was made (dd_remove's text in it), then written without it:
# the text (dyntext) or HTML through Markdown (dyndoc)
stata_dd_save <- function(kind, target) {
  lines <- stata_dd
  rm(stata_dd, envir = globalenv())
  cat(paste0(sub("\r$", "", lines), "\n"), sep = "")
  saved <- stata_dd_unremove(lines)
  if (kind == "dyntext") {
    writeBin(charToRaw(enc2utf8(paste0(paste(saved, collapse = "\n"), "\n"))), target)
  } else {
    writeBin(charToRaw(enc2utf8(stata_md_to_html(paste(saved, collapse = "\n")))), target)
    cat("successfully converted \"", stata_dd_full(target), "\"\n", sep = "")
  }
  invisible(NULL)
}
stata_markdown <- function(src, target, replace) {
  if (!file.exists(src) || dir.exists(src)) stop("file ", src, " not found (Stata: r(601))", call. = FALSE)
  if (file.exists(target) && !replace) stop("file ", stata_dd_full(target), " already exists (Stata: r(602))", call. = FALSE)
  writeBin(charToRaw(enc2utf8(stata_md_to_html(stata_dd_text(src)))), target)
  cat("successfully converted \"", stata_dd_full(target), "\"\n", sep = "")
  invisible(NULL)
}
# Markdown into HTML as Stata 19's markdown and dyndoc write it
stata_md_indent <- function(l) { m <- regexpr("^ *", l); attr(m, "match.length") }
stata_md_blank <- function(l) !nzchar(trimws(l))
stata_md_atx <- function(l) {
  if (stata_md_indent(l) > 3) return(NULL)
  t <- sub("^ +", "", l)
  lev <- attr(regexpr("^#*", t), "match.length")
  if (lev < 1 || lev > 6) return(NULL)
  rest <- substring(t, lev + 1)
  if (nzchar(rest) && !startsWith(rest, " ")) return(NULL)
  body <- trimws(rest)
  tr <- sub("#+$", "", body)
  if (!nzchar(tr) || endsWith(tr, " ")) body <- sub(" +$", "", tr)
  list(level = lev, body = body)
}
stata_md_rule <- function(l) {
  if (stata_md_indent(l) > 3) return(FALSE)
  t <- gsub(" ", "", l, fixed = TRUE)
  nchar(t) >= 3 && grepl("^(-+|\\*+|_+)$", t)
}
stata_md_setext <- function(l) {
  if (stata_md_indent(l) > 3) return(0L)
  t <- trimws(l)
  if (grepl("^=+$", t)) 1L else if (grepl("^-+$", t)) 2L else 0L
}
stata_md_fence <- function(l) {
  ind <- stata_md_indent(l)
  if (ind > 3) return(NULL)
  t <- sub("^ +", "", l)
  ch <- substr(t, 1, 1)
  if (!ch %in% c("`", "~")) return(NULL)
  n <- attr(regexpr(paste0("^\\", ch, "*"), t), "match.length")
  if (n < 3) return(NULL)
  info <- trimws(substring(t, n + 1))
  if (ch == "`" && grepl("`", info, fixed = TRUE)) return(NULL)
  list(ch = ch, n = n, ind = ind, info = info)
}
stata_md_marker <- function(l) {
  ind <- stata_md_indent(l)
  if (ind > 3) return(NULL)
  t <- substring(l, ind + 1)
  first <- substr(t, 1, 1)
  if (first %in% c("-", "*", "+")) {
    ordered <- FALSE; mark <- first; width <- 1L
  } else if (grepl("^[0-9]", t)) {
    digits <- attr(regexpr("^[0-9]+", t), "match.length")
    if (digits > 9) return(NULL)
    d <- substr(t, digits + 1, digits + 1)
    if (!d %in% c(".", ")")) return(NULL)
    ordered <- TRUE; mark <- d; width <- digits + 1L
  } else return(NULL)
  after <- substring(t, width + 1)
  if (!nzchar(after)) return(list(ordered = ordered, mark = mark, content = ind + width + 1L))
  if (!startsWith(after, " ")) return(NULL)
  sp <- stata_md_indent(after)
  list(ordered = ordered, mark = mark, content = ind + width + (if (sp > 4) 1L else sp))
}
stata_md_block_tags <- c("address", "article", "aside", "blockquote", "body", "center", "dd", "details", "div", "dl",
  "dt", "fieldset", "figcaption", "figure", "footer", "form", "h1", "h2", "h3", "h4", "h5", "h6", "head", "header", "hr",
  "html", "iframe", "li", "main", "nav", "ol", "p", "pre", "section", "script", "style", "summary", "table", "tbody", "td",
  "tfoot", "th", "thead", "title", "tr", "ul")
stata_md_html_block <- function(l) {
  if (stata_md_indent(l) > 3) return(FALSE)
  t <- sub("^ +", "", l)
  if (startsWith(t, "<!--")) return(TRUE)
  if (!startsWith(t, "<")) return(FALSE)
  r <- sub("^/", "", substring(t, 2))
  nm <- tolower(regmatches(r, regexpr("^[A-Za-z0-9]*", r)))
  nx <- substr(r, nchar(nm) + 1, nchar(nm) + 1)
  nm %in% stata_md_block_tags && nx %in% c("", " ", ">", "/")
}
stata_md_cells <- function(l) {
  t <- trimws(l)
  t <- sub("^\\|", "", t)
  if (endsWith(t, "|")) t <- substr(t, 1, nchar(t) - 1)
  ch <- strsplit(t, "")[[1]]
  cells <- character(); cur <- ""; esc <- FALSE
  for (c in ch) {
    if (esc) { cur <- paste0(cur, c); esc <- FALSE }
    else if (c == "\\") { cur <- paste0(cur, c); esc <- TRUE }
    else if (c == "|") { cells <- c(cells, cur); cur <- "" }
    else cur <- paste0(cur, c)
  }
  c(cells, cur)
}
stata_md_delim <- function(l) {
  if (!grepl("-", l, fixed = TRUE)) return(NULL)
  out <- character()
  for (c in stata_md_cells(l)) {
    c <- trimws(c)
    left <- startsWith(c, ":"); right <- endsWith(c, ":")
    core <- gsub("^:+|:+$", "", c)
    if (!nzchar(core) || !grepl("^-+$", core)) return(NULL)
    out <- c(out, if (left && right) "center" else if (left) "left" else if (right) "right" else "")
  }
  out
}
stata_md_interrupts <- function(l) {
  if (!is.null(stata_md_atx(l)) || !is.null(stata_md_fence(l)) || stata_md_rule(l)) return(TRUE)
  if (stata_md_indent(l) <= 3 && startsWith(sub("^ +", "", l), ">")) return(TRUE)
  if (stata_md_html_block(l)) return(TRUE)
  m <- stata_md_marker(l)
  if (!is.null(m)) {
    t <- sub("^ +", "", l)
    body <- if (grepl(" ", t, fixed = TRUE)) trimws(sub("^[^ ]* ", "", t)) else ""
    return(nzchar(body) && (!m$ordered || startsWith(t, "1.") || startsWith(t, "1)")))
  }
  FALSE
}
stata_md_escape <- function(s) {
  s <- gsub("&", "&amp;", s, fixed = TRUE)
  s <- gsub("<", "&lt;", s, fixed = TRUE)
  s <- gsub(">", "&gt;", s, fixed = TRUE)
  gsub("\"", "&quot;", s, fixed = TRUE)
}
stata_md_blocks <- function(lines, tight = FALSE) {
  out <- character()
  i <- 1L
  blank <- FALSE
  n <- length(lines)
  get <- function(k) if (k <= n) lines[[k]] else NULL
  while (i <= n) {
    l <- lines[[i]]
    if (stata_md_blank(l)) { i <- i + 1L; blank <- TRUE; next }
    was <- blank
    blank <- FALSE
    a <- stata_md_atx(l)
    if (!is.null(a)) { out <- c(out, stata_md_heading(a$level, a$body)); i <- i + 1L; next }
    f <- stata_md_fence(l)
    if (!is.null(f)) {
      j <- i + 1L; code <- ""
      while (j <= n) {
        f2 <- stata_md_fence(lines[[j]])
        if (!is.null(f2) && f2$ch == f$ch && f2$n >= f$n && !nzchar(f2$info)) break
        drop <- min(stata_md_indent(lines[[j]]), f$ind)
        code <- paste0(code, substring(lines[[j]], drop + 1), "\n")
        j <- j + 1L
      }
      lang <- strsplit(f$info, " +")[[1]][1]
      open <- if (is.na(lang) || !nzchar(lang)) "<pre><code>" else paste0("<pre><code class=\"language-", stata_md_escape(lang), "\">")
      out <- c(out, paste0(open, stata_md_escape(code), "</code></pre>"))
      i <- j + 1L
      next
    }
    if (stata_md_indent(l) >= 4) {
      j <- i; code <- character()
      while (j <= n) {
        m <- lines[[j]]
        if (stata_md_blank(m)) code <- c(code, "")
        else if (stata_md_indent(m) >= 4) code <- c(code, substring(m, 5))
        else break
        j <- j + 1L
      }
      while (length(code) && !nzchar(code[length(code)])) code <- code[-length(code)]
      out <- c(out, paste0("<pre><code>", paste0(stata_md_escape(code), "\n", collapse = ""), "</code></pre>"))
      i <- j
      next
    }
    if (stata_md_rule(l)) { out <- c(out, "<hr />"); i <- i + 1L; next }
    if (stata_md_indent(l) <= 3 && startsWith(sub("^ +", "", l), ">")) {
      inner <- character(); j <- i
      while (j <= n && !stata_md_blank(lines[[j]])) {
        t <- sub("^ +", "", lines[[j]])
        inner <- c(inner, if (startsWith(t, ">")) sub("^ ", "", substring(t, 2)) else lines[[j]])
        j <- j + 1L
      }
      out <- c(out, "<blockquote>", stata_md_blocks(inner), "</blockquote>")
      i <- j
      next
    }
    if (stata_md_html_block(l)) {
      j <- i
      while (j <= n && !stata_md_blank(lines[[j]])) { out <- c(out, lines[[j]]); j <- j + 1L }
      i <- j
      next
    }
    m <- stata_md_marker(l)
    if (!is.null(m)) {
      r <- stata_md_list(lines, i, m$ordered, m$mark)
      out <- c(out, r$html)
      i <- r$nxt
      next
    }
    if (grepl("|", l, fixed = TRUE) && i < n) {
      al <- stata_md_delim(lines[[i + 1L]])
      head <- stata_md_cells(l)
      if (!is.null(al) && length(head) == length(al)) {
        r <- stata_md_table(lines, i, head, al)
        out <- c(out, r$html)
        i <- r$nxt
        next
      }
    }
    j <- i; para <- character(); level <- 0L
    while (j <= n) {
      m <- lines[[j]]
      if (stata_md_blank(m)) break
      if (length(para)) {
        s <- stata_md_setext(m)
        if (s > 0L) { level <- s; j <- j + 1L; break }
        if (stata_md_interrupts(m)) break
      }
      para <- c(para, m)
      j <- j + 1L
    }
    if (level > 0L) out <- c(out, stata_md_heading(level, paste(trimws(para), collapse = " ")))
    else out <- c(out, stata_md_paragraph(para, bare = tight && !was))
    i <- j
  }
  out
}
stata_md_heading <- function(level, body) {
  id <- stata_md_anchor(body)
  paste0("<h", level, "><a href=\"#", id, "\" id=\"", id, "\">", stata_md_inline(body), "</a></h", level, ">")
}
stata_md_anchor <- function(text) {
  plain <- trimws(gsub("[*`]", "", text))
  ch <- strsplit(plain, "")[[1]]
  keep <- vapply(ch, function(c) if (grepl("^[[:alnum:]_-]$", c)) tolower(c) else if (c == " ") "-" else "", "")
  paste(keep, collapse = "")
}
# in a tight list a paragraph has no <p>, unless a blank line comes before it in its item
# (corpus dyndoc.do: "b", a blank line, then more of the item)
stata_md_paragraph <- function(lines, bare = FALSE) {
  n <- length(lines); txt <- ""
  for (k in seq_len(n)) {
    body <- sub("^ +", "", lines[[k]])
    if (k == n) txt <- paste0(txt, sub(" +$", "", body))
    else if (endsWith(body, "  ")) txt <- paste0(txt, sub(" +$", "", body), "\001")
    else if (endsWith(body, "\\")) txt <- paste0(txt, substr(body, 1, nchar(body) - 1), "\001")
    else txt <- paste0(txt, sub(" +$", "", body), " ")
  }
  body <- gsub("\001", "<br />\n", stata_md_inline(txt), fixed = TRUE)
  if (bare) body else paste0("<p>", body, "</p>")
}
stata_md_table <- function(lines, start, head, al) {
  cell <- function(tag, k, text) {
    a <- if (k <= length(al)) al[[k]] else ""
    if (nzchar(a)) paste0("<", tag, " align=\"", a, "\">", stata_md_inline(text), "</", tag, ">")
    else paste0("<", tag, ">", stata_md_inline(text), "</", tag, ">")
  }
  out <- c("<table>", "<thead>", paste0("<tr>", paste0(vapply(seq_along(head), function(k) cell("th", k, head[[k]]), ""), collapse = ""), "</tr>"), "</thead>")
  j <- start + 2L; body <- character(); n <- length(lines)
  while (j <= n && !stata_md_blank(lines[[j]]) && !stata_md_interrupts(lines[[j]])) {
    cs <- stata_md_cells(lines[[j]])
    body <- c(body, paste0("<tr>", paste0(vapply(seq_along(head), function(k) cell("td", k, if (k <= length(cs)) cs[[k]] else ""), ""), collapse = ""), "</tr>"))
    j <- j + 1L
  }
  if (length(body)) out <- c(out, "<tbody>", body, "</tbody>")
  list(html = c(out, "</table>"), nxt = j)
}
stata_md_list <- function(lines, start, ordered, mark) {
  items <- list(); loose <- FALSE; j <- start; blank_before <- FALSE; n <- length(lines)
  while (j <= n) {
    l <- lines[[j]]
    m <- stata_md_marker(l)
    if (is.null(m) || m$ordered != ordered || m$mark != mark || stata_md_indent(l) >= 4) break
    if (blank_before && length(items)) loose <- TRUE
    content <- m$content
    item <- substring(l, min(content, nchar(l)) + 1)
    j <- j + 1L
    blank_before <- FALSE
    while (j <= n) {
      x <- lines[[j]]
      if (stata_md_blank(x)) {
        nx <- if (j + 1L <= n) lines[[j + 1L]] else NULL
        more <- !is.null(nx) && !stata_md_blank(nx) && (stata_md_indent(nx) >= content || {
          m2 <- stata_md_marker(nx); !is.null(m2) && m2$ordered == ordered && m2$mark == mark })
        if (!more) break
        if (stata_md_indent(nx) >= content) { item <- c(item, ""); j <- j + 1L; next }
        blank_before <- TRUE
        j <- j + 1L
        break
      }
      if (stata_md_indent(x) >= content) item <- c(item, substring(x, content + 1))
      else if (!is.null(stata_md_marker(x)) || stata_md_interrupts(x)) break
      else item <- c(item, sub("^ +", "", x))
      j <- j + 1L
    }
    if (length(item) > 1 && any(!nzchar(item[-1]))) {
      for (k in seq_along(item)[-1]) if (!nzchar(item[[k]]) && k < length(item) && stata_md_indent(item[[k + 1]]) == 0) loose <- TRUE
    }
    items[[length(items) + 1L]] <- item
  }
  tag <- if (ordered) "ol" else "ul"
  out <- paste0("<", tag, ">")
  for (item in items) {
    inner <- stata_md_blocks(item, tight = !loose)
    if (loose) out <- c(out, "<li>", inner, "</li>")
    else {
      txt <- paste0(paste0(inner, "\n"), collapse = "")
      nl <- lengths(regmatches(txt, gregexpr("\n", txt, fixed = TRUE)))
      if (endsWith(txt, "\n") && nl == 1) txt <- substr(txt, 1, nchar(txt) - 1)
      out <- c(out, paste0("<li>", txt, "</li>"))
    }
  }
  list(html = c(out, paste0("</", tag, ">")), nxt = j)
}
stata_md_punct <- function(c) !is.na(c) && grepl("^[^[:alnum:][:space:]]$", c)
stata_md_html_tag <- function(s, i) {
  n <- length(s)
  if (s[i] != "<") return(0L)
  k <- i + 1L
  if (k <= n && s[k] == "/") k <- k + 1L
  if (k > n || !grepl("^[A-Za-z]$", s[k])) return(0L)
  while (k <= n && grepl("^[A-Za-z0-9-]$", s[k])) k <- k + 1L
  q <- ""
  while (k <= n) {
    c <- s[k]
    if (nzchar(q)) { if (c == q) q <- "" }
    else if (c %in% c("\"", "'")) q <- c
    else if (c == ">") return(k - i + 1L)
    else if (c == "<") return(0L)
    k <- k + 1L
  }
  0L
}
stata_md_link <- function(s, open) {
  n <- length(s); depth <- 0L; k <- open; label_end <- 0L
  repeat {
    if (k > n) return(NULL)
    c <- s[k]
    if (c == "[") depth <- depth + 1L
    else if (c == "]") { depth <- depth - 1L; if (depth == 0L) { label_end <- k; break } }
    else if (c == "\\") k <- k + 1L
    k <- k + 1L
  }
  if (label_end + 1L > n || s[label_end + 1L] != "(") return(NULL)
  k <- label_end + 2L
  while (k <= n && s[k] == " ") k <- k + 1L
  href <- ""
  if (k <= n && s[k] == "<") {
    k <- k + 1L
    while (k <= n) { c <- s[k]; k <- k + 1L; if (c == ">") break; href <- paste0(href, c) }
  } else {
    par <- 0L
    while (k <= n) {
      c <- s[k]
      if (c == " " || (c == ")" && par == 0L)) break
      if (c == "(") par <- par + 1L
      if (c == ")") par <- par - 1L
      href <- paste0(href, c); k <- k + 1L
    }
  }
  while (k <= n && s[k] == " ") k <- k + 1L
  title <- NULL
  if (k <= n && s[k] %in% c("\"", "'")) {
    q <- s[k]; t <- ""; k <- k + 1L
    while (k <= n) { c <- s[k]; k <- k + 1L; if (c == q) break; t <- paste0(t, c) }
    title <- t
    while (k <= n && s[k] == " ") k <- k + 1L
  }
  if (k > n || s[k] != ")") return(NULL)
  list(label_end = label_end, href = href, title = title, end = k + 1L)
}
stata_md_inline <- function(text) {
  s <- strsplit(text, "")[[1]]
  pieces <- stata_md_emphasis(stata_md_scan(s))
  out <- ""; plain <- ""
  for (p in pieces) {
    if (p$k == "text") plain <- paste0(plain, p$v)
    else if (p$k == "html") { out <- paste0(out, stata_md_typeset(plain), p$v); plain <- "" }
    else plain <- paste0(plain, strrep(p$c, p$n))
  }
  paste0(out, stata_md_typeset(plain))
}
stata_md_scan <- function(s) {
  pieces <- list(); text <- ""; n <- length(s); i <- 1L
  add <- function(p) pieces[[length(pieces) + 1L]] <<- p
  flush <- function() if (nzchar(text)) { add(list(k = "text", v = text)); text <<- "" }
  while (i <= n) {
    c <- s[i]
    if (c == "\\") {
      if (i < n && grepl("^[[:punct:]]$", s[i + 1L])) {
        flush(); add(list(k = "html", v = gsub("&quot;", "\"", stata_md_escape(s[i + 1L]), fixed = TRUE))); i <- i + 2L; next
      }
      text <- paste0(text, c); i <- i + 1L; next
    }
    if (c == "`") {
      nn <- 0L; while (i + nn <= n && s[i + nn] == "`") nn <- nn + 1L
      k <- i + nn; found <- 0L
      while (k <= n) {
        if (s[k] == "`") {
          m <- 0L; while (k + m <= n && s[k + m] == "`") m <- m + 1L
          if (m == nn) { found <- k; break }
          k <- k + m
        } else k <- k + 1L
      }
      if (found > 0L) {
        code <- if (found > i + nn) paste(s[(i + nn):(found - 1L)], collapse = "") else ""
        code <- gsub("\n", " ", code, fixed = TRUE)
        if (startsWith(code, " ") && endsWith(code, " ") && nzchar(trimws(code))) code <- substr(code, 2, nchar(code) - 1)
        flush(); add(list(k = "html", v = paste0("<code>", stata_md_escape(code), "</code>")))
        i <- found + nn; next
      }
      text <- paste0(text, strrep("`", nn)); i <- i + nn; next
    }
    if (c == "<" && (i == n || s[i + 1L] != "<") && (i == 1L || s[i - 1L] != "<")) {
      rest <- if (i < n) paste(s[(i + 1L):n], collapse = "") else ""
      e <- regexpr(">", rest, fixed = TRUE)
      if (e > 0) {
        target <- substr(rest, 1, e - 1)
        uri <- grepl("^[A-Za-z0-9+.-]{2,}:", target) && !grepl("[ <]", target)
        mail <- !grepl("[ </]", target) && grepl("@", target, fixed = TRUE) && grepl(".", target, fixed = TRUE)
        if (uri || mail) {
          flush()
          href <- if (uri) target else paste0("mailto:", target)
          add(list(k = "html", v = paste0("<a href=\"", stata_md_escape(href), "\">", stata_md_escape(target), "</a>")))
          i <- i + 2L + nchar(target); next
        }
      }
      tl <- stata_md_html_tag(s, i)
      if (tl > 0L) { flush(); add(list(k = "html", v = paste(s[i:(i + tl - 1L)], collapse = ""))); i <- i + tl; next }
    }
    image <- c == "!" && i < n && s[i + 1L] == "["
    if (c == "[" || image) {
      open <- if (image) i + 1L else i
      lk <- stata_md_link(s, open)
      if (!is.null(lk)) {
        label <- if (lk$label_end > open + 1L) paste(s[(open + 1L):(lk$label_end - 1L)], collapse = "") else ""
        flush()
        ta <- if (is.null(lk$title)) "" else paste0(" title=\"", stata_md_escape(lk$title), "\"")
        if (image) add(list(k = "html", v = paste0("<img src=\"", stata_md_escape(lk$href), "\" alt=\"", stata_md_escape(gsub("[*`]", "", label)), "\"", ta, " />")))
        else add(list(k = "html", v = paste0("<a href=\"", stata_md_escape(lk$href), "\"", ta, ">", stata_md_inline(label), "</a>")))
        i <- lk$end; next
      }
    }
    if (c == "*" || c == "_") {
      nn <- 0L; while (i + nn <= n && s[i + nn] == c) nn <- nn + 1L
      before <- if (i == 1L) NA else s[i - 1L]
      after <- if (i + nn <= n) s[i + nn] else NA
      sb <- is.na(before) || grepl("^[[:space:]]$", before)
      sa <- is.na(after) || grepl("^[[:space:]]$", after)
      pb <- stata_md_punct(before); pa <- stata_md_punct(after)
      left <- !sa && (!pa || sb || pb)
      right <- !sb && (!pb || sa || pa)
      if (c == "*") { op <- left; cl <- right } else { op <- left && (!right || pb); cl <- right && (!left || pa) }
      flush(); add(list(k = "delim", c = c, n = nn, open = op, close = cl))
      i <- i + nn; next
    }
    text <- paste0(text, c); i <- i + 1L
  }
  flush()
  pieces
}
stata_md_emphasis <- function(p) {
  i <- 1L
  while (i <= length(p)) {
    x <- p[[i]]
    if (x$k != "delim" || !isTRUE(x$close)) { i <- i + 1L; next }
    o <- 0L
    if (i > 1L) for (k in (i - 1L):1L) { y <- p[[k]]; if (y$k == "delim" && isTRUE(y$open) && y$c == x$c && y$n > 0) { o <- k; break } }
    if (o == 0L) { i <- i + 1L; next }
    on <- p[[o]]$n; cn <- p[[i]]$n
    used <- if (on >= 2 && cn >= 2 && !(on %% 2 == 1 && cn %% 2 == 1)) 2L else 1L
    tags <- if (used == 2L) c("<strong>", "</strong>") else c("<em>", "</em>")
    p[[o]]$n <- on - used
    p[[i]]$n <- cn - used
    if (i - o > 1L) for (k in (o + 1L):(i - 1L)) if (p[[k]]$k == "delim") { p[[k]]$open <- FALSE; p[[k]]$close <- FALSE }
    p <- append(p, list(list(k = "html", v = tags[2])), after = i - 1L)
    p <- append(p, list(list(k = "html", v = tags[1])), after = o)
    i <- i + 2L
    if (i <= length(p) && p[[i]]$k == "delim" && p[[i]]$n == 0) i <- i + 1L
  }
  p
}
stata_md_typeset <- function(text) {
  if (!nzchar(text)) return("")
  s <- strsplit(text, "")[[1]]; n <- length(s); out <- character(); i <- 1L
  while (i <= n) {
    c <- s[i]
    nx <- if (i < n) s[i + 1L] else NA
    pv <- if (i > 1L) s[i - 1L] else NA
    if (c == "-" && identical(nx, "-")) {
      if (i + 2L <= n && s[i + 2L] == "-") { out <- c(out, "&mdash;"); i <- i + 3L } else { out <- c(out, "&ndash;"); i <- i + 2L }
      next
    }
    if (c == "." && identical(nx, ".") && i + 2L <= n && s[i + 2L] == ".") { out <- c(out, "&hellip;"); i <- i + 3L; next }
    if (c == "<" && identical(nx, "<")) { out <- c(out, "&laquo;"); i <- i + 2L; next }
    if (c == ">" && identical(nx, ">")) { out <- c(out, "&raquo;"); i <- i + 2L; next }
    if (c == "\"" || c == "'") {
      opening <- (is.na(pv) || grepl("^[[:space:]]$", pv) || pv %in% c("(", "[", "{", "-")) && !is.na(nx) && !grepl("^[[:space:]]$", nx)
      out <- c(out, if (c == "\"") { if (opening) "&ldquo;" else "&rdquo;" } else { if (opening) "&lsquo;" else "&rsquo;" })
    } else if (c == "&") {
      rest <- paste(s[(i + 1L):min(n, i + 32L)], collapse = "")
      if (i == n) rest <- ""
      e <- regexpr(";", rest, fixed = TRUE)
      nm <- if (e > 0) substr(rest, 1, e - 1) else ""
      ent <- e > 0 && nzchar(nm) && (grepl("^[A-Za-z0-9]+$", nm) || grepl("^#[0-9A-Fa-fx]+$", nm))
      out <- c(out, if (ent) "&" else "&amp;")
    } else if (c == "<") out <- c(out, "&lt;")
    else if (c == ">") out <- c(out, "&gt;")
    else out <- c(out, c)
    i <- i + 1L
  }
  paste(out, collapse = "")
}
stata_md_to_html <- function(text) {
  text <- gsub("\r\n?", "\n", text)
  lines <- strsplit(text, "\n", fixed = TRUE)[[1]]
  if (endsWith(text, "\n")) lines <- c(lines, "")
  lines <- vapply(lines, function(l) {
    if (!grepl("\t", l, fixed = TRUE)) return(l)
    o <- ""
    for (ch in strsplit(l, "")[[1]]) o <- paste0(o, if (ch == "\t") strrep(" ", 4 - nchar(o) %% 4) else ch)
    o
  }, "", USE.NAMES = FALSE)
  html <- stata_md_blocks(lines)
  if (!length(html)) return("")
  paste0(paste(html, collapse = "\n"), "\n")
}"##;

impl Session {
    /// `markdown src, saving(file) [replace]`.
    pub(super) fn markdown(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[("saving", 6), ("replace", 7)]) {
            return Err(no(format!(
                "markdown with the option {o} is not translated yet"
            )));
        }
        if c.using.is_some() || c.parts.if_exp.is_some() || c.parts.in_range.is_some() {
            return Err(no("markdown is translated for a file name and its options"));
        }
        let toks = crate::dofile::macros::tokens(c.parts.main.trim());
        let [src] = toks.as_slice() else {
            return Err(no("markdown is translated with one file name"));
        };
        let Some(target) = opts.arg("saving", 6) else {
            // Stata 19, before it reads anything
            out.line("stop(\"option saving() required (Stata: r(198))\", call. = FALSE)");
            return Ok(());
        };
        if target.contains(',') || target.trim().is_empty() {
            return Err(no("markdown, saving(file, ...) is not translated yet"));
        }
        self.helpers.insert("stata_dyndoc");
        out.line(format!(
            "stata_markdown({}, {}, {})",
            r_text(&file_name(src, "")),
            r_text(&file_name(&target, "")),
            super::stats::r_logical(opts.has("replace", 7))
        ));
        Ok(())
    }
}
