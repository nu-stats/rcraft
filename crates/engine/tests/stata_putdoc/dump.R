# Reads a .docx or .pdf back for crates/engine/tests/stata_putdoc.rs, the same way for the files Stata 19 wrote and the
# ones RCraft writes: structure only (paragraph styles, alignment, runs with bold/italic/underline/line break flags,
# tables with their cells' spans and alignment, the pieces of text on each PDF page). A number is kept as it is
# written; any other text only as its length and a checksum (#11:1a2b3c4d), so no text Stata wrote is kept
# (AGENTS.md, section 1).
stata_dump_mask <- function(s) {
  if (!nzchar(s) || grepl("^ *[-+]?([0-9]+[.]?[0-9]*|[.][0-9]+)(e[-+][0-9]+)? *$", s) || grepl("^ *[.] *$", s)) return(s)
  b <- as.integer(charToRaw(enc2utf8(s)))
  a <- 1; d <- 0
  for (x in b) { a <- (a + x) %% 65521; d <- (d + a) %% 65521 }
  sprintf("#%d:%04x%04x", length(b), d, a)
}
stata_dump_docx <- function(path) {
  td <- tempfile(); dir.create(td); on.exit(unlink(td, recursive = TRUE))
  utils::unzip(path, exdir = td)
  p <- file.path(td, "word", "document.xml")
  x <- readChar(p, file.info(p)$size, useBytes = TRUE)
  Encoding(x) <- "UTF-8"
  x <- gsub(">[[:space:]]+<", "><", x)
  body <- sub("^.*<w:body>", "", x); body <- sub("</w:body>.*$", "", body)
  body <- gsub("<w:sectPr.*?</w:sectPr>|<w:sectPr[^>]*/>", "", body, perl = TRUE)
  unesc <- function(s) { s <- gsub("&lt;", "<", s, fixed = TRUE); s <- gsub("&gt;", ">", s, fixed = TRUE); gsub("&amp;", "&", s, fixed = TRUE) }
  runs <- function(frag) {
    rs <- regmatches(frag, gregexpr("<w:r>.*?</w:r>", frag, perl = TRUE))[[1]]
    out <- character()
    for (r in rs) {
      t <- if (grepl("<w:t[ >]", r)) unesc(sub("</w:t>.*$", "", sub("^.*?<w:t[^>]*>", "", r, perl = TRUE))) else ""
      fl <- paste0(c(if (grepl("<w:b[ /]", r)) "b", if (grepl("<w:i[ /]", r)) "i", if (grepl("<w:u[ /]", r)) "u", if (grepl("<w:br[ />]", r)) "L"), collapse = "")
      if (nzchar(t) || nzchar(fl)) out <- c(out, paste0(if (nzchar(fl)) paste0("{", fl, "}"), stata_dump_mask(t)))
    }
    paste(out, collapse = "|")
  }
  attr1 <- function(frag, tag) { m <- regmatches(frag, regexpr(paste0("<w:", tag, " w:val=\"[^\"]*\""), frag)); if (length(m)) sub("\".*$", "", sub("^.*w:val=\"", "", m)) else "-" }
  para <- function(frag) {
    if (grepl("w:type=\"page\"", frag, fixed = TRUE)) return("PAGEBREAK")
    st <- attr1(frag, "pStyle"); if (st == "Normal") st <- "-"
    paste0("P style=", st, " jc=", attr1(frag, "jc"), " ", runs(frag))
  }
  tbls <- regmatches(body, gregexpr("<w:tbl>.*?</w:tbl>", body, perl = TRUE))[[1]]
  for (i in seq_along(tbls)) body <- sub("<w:tbl>.*?</w:tbl>", paste0("<TBL", i, ">"), body, perl = TRUE)
  items <- regmatches(body, gregexpr("<TBL[0-9]+>|<w:p/>|<w:p[ >].*?</w:p>", body, perl = TRUE))[[1]]
  out <- character()
  for (it in items) {
    if (startsWith(it, "<TBL")) {
      tb <- tbls[as.integer(sub("<TBL([0-9]+)>", "\\1", it))]
      out <- c(out, "TABLE")
      for (tr in regmatches(tb, gregexpr("<w:tr>.*?</w:tr>", tb, perl = TRUE))[[1]]) {
        cells <- vapply(regmatches(tr, gregexpr("<w:tc>.*?</w:tc>", tr, perl = TRUE))[[1]], function(tc) {
          sp <- attr1(tc, "gridSpan"); jc <- attr1(tc, "jc")
          paste0(runs(tc), if (sp != "-") paste0("{span", sp, "}"), if (jc != "-") paste0("{", jc, "}"))
        }, "")
        out <- c(out, paste0("  ROW ", paste(cells, collapse = " | ")))
      }
    } else if (it == "<w:p/>") out <- c(out, "P style=- jc=- ")
    else out <- c(out, para(it))
  }
  paste(out, collapse = "\n")
}
stata_dump_pdf <- function(path) {
  b <- readBin(path, "raw", file.info(path)$size)
  # one plain ASCII character per byte (a non-ASCII byte becomes ?), so that no locale (UTF-8 on Linux and macOS,
  # Windows code pages) translates the binary file and the byte offsets stay the characters' offsets
  ascii <- function(r) iconv(rawToChar(replace(r, r == as.raw(0), as.raw(1))), "latin1", "ASCII", sub = "?")
  txt <- ascii(b)
  st <- gregexpr("(?<!end)stream\r?\n", txt, perl = TRUE)[[1]]
  en <- gregexpr("endstream", txt, fixed = TRUE)[[1]]
  mt <- attr(st, "match.length")
  unesc <- function(s) {
    m <- gregexpr("\\\\[0-7]{3}", s)
    regmatches(s, m) <- lapply(regmatches(s, m), function(z) vapply(z, function(o) rawToChar(as.raw(strtoi(substring(o, 2), 8L))), ""))
    gsub("\\\\(.)", "\\1", s)
  }
  out <- character(); page <- 0L
  for (i in seq_along(st)) {
    raw <- b[(st[i] + mt[i]):(en[i] - 1)]
    s <- tryCatch(ascii(memDecompress(raw, "gzip")), error = function(e) tryCatch(ascii(raw), error = function(e) ""))
    if (!grepl("T[jJ]", s, useBytes = TRUE)) next
    page <- page + 1L
    out <- c(out, paste0("PAGE ", page))
    items <- regmatches(s, gregexpr("\\[.*?\\]\\s*TJ|\\((?:\\\\.|[^\\\\)])*\\)\\s*Tj", s, perl = TRUE))[[1]]
    for (it in items) {
      parts <- regmatches(it, gregexpr("\\((?:\\\\.|[^\\\\)])*\\)", it, perl = TRUE))[[1]]
      out <- c(out, paste0("  ", stata_dump_mask(unesc(paste0(substr(parts, 2, nchar(parts) - 1), collapse = "")))))
    }
  }
  paste(out, collapse = "\n")
}
