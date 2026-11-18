//! `putdocx` and `putpdf`, as Stata 19 does them (`plan/stata/putdocx.md`):
//! a document built in memory by `begin`, `paragraph`, `text`, `table`,
//! `pagebreak` and `describe`, written by `save`. The model lives in R
//! (`stata_pd`, one document per command), so the messages and the counts
//! `describe` leaves in r() are Stata's; the file is written from it.
//!
//! - Word: the package is written here (zip of the parts), not with officer:
//!   officer's template has no `Title` or `TableGrid` style, French style ids
//!   and A4 pages, and Stata's file has Letter pages and those styles.
//! - PDF: R's own `pdf()` device draws the text (Helvetica 11, A4 pages with 36 points of margin, as the file Stata 19 wrote here).
//! - Not translated (refused): `image`, `sectionbreak`, `pagenumber`,
//!   `textblock`, `textfile`, `append` of documents, `collect`, begin's options,
//!   fonts, cell formats and table options other than `nformat()`.

use super::*;

pub(super) const PUTDOC_HELPER: &str = r##"if (!exists("stata_pd")) stata_pd <- list()
stata_pd_doc <- function(k) {
  d <- stata_pd[[k]]
  if (is.null(d)) stop("document not opened (Stata: r(198))", call. = FALSE)
  d
}
stata_pd_put <- function(k, d) {
  stata_pd[[k]] <<- d
  invisible(NULL)
}
stata_pd_begin <- function(k) {
  if (!is.null(stata_pd[[k]])) stop("document already open in memory (Stata: r(110))", call. = FALSE)
  stata_pd_put(k, list(blocks = list(), cur = 0L, names = list(), npara = 0L))
}
stata_pd_clear <- function(k) {
  stata_pd[[k]] <<- NULL
  invisible(NULL)
}
stata_pd_add <- function(d, blk) {
  d$blocks[[length(d$blocks) + 1L]] <- blk
  d
}
stata_pd_paragraph <- function(k, style = "", halign = "") {
  d <- stata_pd_doc(k)
  d <- stata_pd_add(d, list(t = "p", style = style, jc = halign, runs = list()))
  d$cur <- length(d$blocks)
  d$npara <- d$npara + 1L
  stata_pd_put(k, d)
}
# a number as text: the default is %11.0g trimmed; nformat() is used as given (padded to its width)
stata_pd_num <- function(x, nformat = NULL, pad = TRUE, default = "%11.0g") {
  x <- x[1]
  if (is.na(x)) return(".")
  if (is.null(nformat)) stata_fmt(x, default, trim = TRUE) else stata_fmt(x, nformat, trim = !pad)
}
stata_pd_text <- function(k, x, bold = FALSE, italic = FALSE, underline = FALSE, linebreak = FALSE, nformat = NULL) {
  d <- stata_pd_doc(k)
  if (d$cur == 0L) stop("no active paragraph (Stata: r(198))", call. = FALSE)
  s <- if (is.character(x)) x[1] else stata_pd_num(x, nformat)
  if (is.na(s)) s <- ""
  b <- d$blocks[[d$cur]]
  b$runs[[length(b$runs) + 1L]] <- list(text = s, b = bold, i = italic, u = underline, br = linebreak)
  d$blocks[[d$cur]] <- b
  stata_pd_put(k, d)
}
stata_pd_pagebreak <- function(k) {
  d <- stata_pd_doc(k)
  d <- stata_pd_add(d, list(t = "pb"))
  d$cur <- 0L
  stata_pd_put(k, d)
}
stata_pd_cell <- function(text = "", span = 1L, jc = "") list(text = text, span = span, jc = jc)
stata_pd_new_table <- function(k, d, name, cells, etable = FALSE) {
  if (!is.null(d$names[[name]])) cat("(note: table ", name, " has been redefined)\n", sep = "")
  d <- stata_pd_add(d, list(t = "tbl", name = name, cells = cells, etable = etable))
  d$names[[name]] <- length(d$blocks)
  d <- stata_pd_add(d, list(t = "empty"))
  d$cur <- 0L
  stata_pd_put(k, d)
}
stata_pd_table_size <- function(k, name, nr, nc) {
  d <- stata_pd_doc(k)
  if (!is.na(nr) && !is.na(nc) && (nr < 1 || nc < 1) && k == "pdf") stop("RCraft: putpdf table with no rows or columns leaves Stata's own document unusable (r(3301) at the next command); not reproduced", call. = FALSE)
  if (is.na(nr) || is.na(nc) || nr < 1 || nc < 1) stop("failed to add table (Stata: r(198))", call. = FALSE)
  if (nr * nc > 1e6) stop("RCraft: a table of more than a million cells is not made here", call. = FALSE)
  cells <- lapply(seq_len(nr), function(i) lapply(seq_len(nc), function(j) stata_pd_cell()))
  stata_pd_new_table(k, d, name, cells)
}
stata_pd_set_cell <- function(k, name, i, j, x, nformat = NULL) {
  d <- stata_pd_doc(k)
  idx <- d$names[[name]]
  if (is.null(idx)) stop("table ", name, " does not exist (Stata: r(198))", call. = FALSE)
  tb <- d$blocks[[idx]]
  if (is.na(i) || i < 1 || i > length(tb$cells)) stop("invalid cell specification; row out of range (Stata: r(198))", call. = FALSE)
  if (is.na(j) || j < 1 || j > length(tb$cells[[i]])) stop("invalid cell specification; column out of range (Stata: r(198))", call. = FALSE)
  tb$cells[[i]][[j]]$text <- if (is.character(x)) x[1] else stata_pd_num(x, nformat)
  d$blocks[[idx]] <- tb
  stata_pd_put(k, d)
}
# the values of the variables as Stata shows them in the cells: each variable's display format
# (value labels used), a string as it is, a missing value as . (or .a ...)
stata_pd_table_data <- function(k, name, dat, varnames = FALSE, obsno = FALSE, defaults = character(0)) {
  d <- stata_pd_doc(k)
  obs <- dat$stata_obs_
  dat$stata_obs_ <- NULL
  miss <- function(z) { t <- tryCatch(haven::na_tag(z), error = function(e) NA_character_); if (is.na(t)) "." else paste0(".", t) }
  cols <- lapply(names(dat), function(v) {
    x <- dat[[v]]
    if (is.character(x)) return(ifelse(is.na(x), "", x))
    if (!is.numeric(x)) stop("RCraft: putdocx table of the variable ", v, " (not a number or a string) is not translated yet", call. = FALSE)
    u <- unclass(x); attributes(u) <- NULL
    f <- attr(x, "format.stata", exact = TRUE)
    if (is.null(f) && v %in% names(defaults)) f <- defaults[[v]]
    if (is.null(f)) {
      kk <- u[!is.na(u)]
      f <- if (all(kk == round(kk))) "%12.0g" else if (all(kk == stata_float(kk))) "%9.0g" else "%10.0g"
    }
    if (!grepl("^%-?0?[0-9]+(\\.[0-9]+)?[gfe]c?$", f)) stop("RCraft: putdocx table of a variable with the display format ", f, " is not translated yet", call. = FALSE)
    s <- vapply(u, function(z) if (is.na(z)) miss(z) else stata_fmt(z, f, trim = TRUE), "")
    vl <- attr(x, "labels", exact = TRUE)
    if (length(vl)) {
      m <- match(u, as.numeric(unclass(vl)))
      s[!is.na(m)] <- names(vl)[m[!is.na(m)]]
    }
    s
  })
  n <- length(obs)
  cells <- list()
  if (varnames) cells[[1]] <- c(if (obsno) list(stata_pd_cell("")), lapply(names(dat), function(v) stata_pd_cell(v)))
  for (i in seq_len(n)) {
    cells[[length(cells) + 1L]] <- c(if (obsno) list(stata_pd_cell(as.character(obs[i]))), lapply(cols, function(cl) stata_pd_cell(cl[i])))
  }
  if (!length(cells) || !length(cells[[1]])) stop("RCraft: putdocx table from data with no observations or variables is not translated yet", call. = FALSE)
  stata_pd_new_table(k, d, name, cells)
}
stata_pd_table_matrix <- function(k, name, M, rows = FALSE, cols = FALSE, nformat = NULL) {
  d <- stata_pd_doc(k)
  why <- attr(M, "stata_names")
  if (!is.null(why)) stop("RCraft: putdocx table of a matrix not laid out here as Stata keeps it: ", why, call. = FALSE)
  M <- as.matrix(M)
  rn <- if (is.null(rownames(M))) paste0("r", seq_len(nrow(M))) else rownames(M)
  cn <- if (is.null(colnames(M))) paste0("c", seq_len(ncol(M))) else colnames(M)
  num <- function(z) stata_pd_num(z, nformat, pad = FALSE, default = "%12.0g")
  cells <- list()
  if (cols) cells[[1]] <- c(if (rows) list(stata_pd_cell("")), lapply(cn, function(v) stata_pd_cell(v)))
  for (i in seq_len(nrow(M))) {
    cells[[length(cells) + 1L]] <- c(if (rows) list(stata_pd_cell(rn[i])), lapply(seq_len(ncol(M)), function(j) stata_pd_cell(num(unname(M[i, j])))))
  }
  if (!length(cells)) stop("failed to add table (Stata: r(198))", call. = FALSE)
  stata_pd_new_table(k, d, name, cells)
}
# the coefficient table of the last estimation command's output, as Stata keeps it for etable
stata_pd_table_etable <- function(k, name) {
  d <- stata_pd_doc(k)
  txt <- utils::capture.output(stata_regress_print(level = 0.95))
  h <- grep("\\| *Coefficient", txt)
  if (!length(h)) stop("information for the estimation table not found;\n    Please replay estimation results and try again (Stata: r(111))", call. = FALSE)
  h <- h[1]
  hl <- txt[h]
  dep <- trimws(sub("\\|.*$", "", hl))
  rh <- sub("^[^|]*\\|", "", hl)
  titles <- regmatches(rh, gregexpr("Coefficient|Std\\. err\\.|std\\. err\\.|P>\\|[tz]\\||\\[[0-9.]+% conf\\. interval\\]|\\b[tz]\\b", rh))[[1]]
  ci <- grep("conf\\. interval", titles)
  right <- function(x, span = 1L) stata_pd_cell(x, span, "right")
  head <- c(list(right(dep)), lapply(titles, function(t) right(t, if (length(ci) && t == titles[ci]) 2L else 1L)))
  cells <- list(head)
  i <- h + 2L
  while (i <= length(txt) && grepl("\\|", txt[i])) {
    nm <- trimws(sub("\\|.*$", "", txt[i]))
    val <- strsplit(trimws(sub("^[^|]*\\|", "", txt[i])), " +")[[1]]
    if (length(val) != length(titles) + length(ci)) stop("RCraft: putdocx table = etable of this coefficient table (base levels, equations) is not translated yet", call. = FALSE)
    cells[[length(cells) + 1L]] <- c(list(right(nm)), lapply(val, right))
    i <- i + 1L
  }
  stata_pd_new_table(k, d, name, cells, etable = TRUE)
}
stata_pd_describe <- function(k, name = NULL) {
  d <- stata_pd_doc(k)
  rule <- function() cat("\n  ", strrep("-", 20), "+", strrep("-", 23), "\n", sep = "")
  row <- function(l, v) cat("  ", formatC(l, width = -20), "|  ", v, "\n", sep = "")
  if (is.null(name)) {
    rule()
    row("No. of tables", sum(vapply(d$blocks, function(b) b$t == "tbl", NA)))
    row("No. of paragraphs", d$npara)
    cat("\n")
    return(invisible(NULL))
  }
  idx <- d$names[[name]]
  if (is.null(idx)) stop("table ", name, " does not exist (Stata: r(198))", call. = FALSE)
  tb <- d$blocks[[idx]]
  nr <- length(tb$cells)
  nc <- sum(vapply(tb$cells[[1]], function(z) z$span, 1L))
  rule()
  row("Table name", name)
  row("No. of rows", nr)
  row("No. of cols", nc)
  cat("\n")
  list(nrows = nr, ncols = nc)
}
stata_pd_esc <- function(s) {
  s <- gsub("&", "&amp;", s, fixed = TRUE)
  s <- gsub("<", "&lt;", s, fixed = TRUE)
  gsub(">", "&gt;", s, fixed = TRUE)
}
stata_pd_run_xml <- function(r) {
  pr <- paste0(c(if (isTRUE(r$b)) "<w:b/>", if (isTRUE(r$i)) "<w:i/>", if (isTRUE(r$u)) "<w:u w:val=\"single\"/>"), collapse = "")
  paste0("<w:r>", if (nzchar(pr)) paste0("<w:rPr>", pr, "</w:rPr>"), "<w:t xml:space=\"preserve\">", stata_pd_esc(r$text), "</w:t>", if (isTRUE(r$br)) "<w:br/>", "</w:r>")
}
stata_pd_cell_xml <- function(cell, w, edges) {
  bd <- paste0(vapply(c("top", "left", "bottom", "right")[c("top", "left", "bottom", "right") %in% edges], function(e)
    paste0("<w:", e, " w:val=\"single\" w:color=\"000000\"/>"), ""), collapse = "")
  paste0("<w:tc><w:tcPr><w:tcW w:w=\"", w * cell$span, "\" w:type=\"dxa\"/>",
         if (cell$span > 1L) paste0("<w:gridSpan w:val=\"", cell$span, "\"/>"),
         if (nzchar(bd)) paste0("<w:tcBorders>", bd, "</w:tcBorders>"), "</w:tcPr>",
         "<w:p><w:pPr><w:spacing w:after=\"0\"/>", if (nzchar(cell$jc)) paste0("<w:jc w:val=\"", cell$jc, "\"/>"), "</w:pPr>",
         if (nzchar(cell$text)) paste0("<w:r><w:t xml:space=\"preserve\">", stata_pd_esc(cell$text), "</w:t></w:r>"), "</w:p></w:tc>")
}
stata_pd_xml <- function(b) {
  switch(b$t,
    p = paste0("<w:p>", if (nzchar(b$style) || nzchar(b$jc)) paste0("<w:pPr>", if (nzchar(b$style)) paste0("<w:pStyle w:val=\"", b$style, "\"/>"),
        if (nzchar(b$jc)) paste0("<w:jc w:val=\"", b$jc, "\"/>"), "</w:pPr>"), paste0(vapply(b$runs, stata_pd_run_xml, ""), collapse = ""), "</w:p>"),
    empty = "<w:p/>",
    pb = "<w:p><w:r><w:br w:type=\"page\"/></w:r></w:p>",
    tbl = {
      nc <- sum(vapply(b$cells[[1]], function(z) z$span, 1L))
      w <- 9360L %/% nc
      nr <- length(b$cells)
      rows <- vapply(seq_len(nr), function(i) {
        col <- 0L
        paste0("<w:tr>", paste0(vapply(b$cells[[i]], function(cell) {
          col <<- col + cell$span
          edges <- if (b$etable) c(if (i <= 2L) "top", if (i == 1L || i == nr) "bottom", if (col == 1L) "right", if (col == 2L) "left") else character(0)
          stata_pd_cell_xml(cell, w, edges)
        }, ""), collapse = ""), "</w:tr>")
      }, "")
      paste0("<w:tbl><w:tblPr><w:tblStyle w:val=\"TableGrid\"/>",
             if (b$etable) "<w:tblBorders><w:top w:val=\"nil\"/><w:left w:val=\"nil\"/><w:bottom w:val=\"nil\"/><w:right w:val=\"nil\"/><w:insideH w:val=\"nil\"/><w:insideV w:val=\"nil\"/></w:tblBorders>",
             "</w:tblPr><w:tblGrid>", paste0(rep(paste0("<w:gridCol w:w=\"", w, "\"/>"), nc), collapse = ""), "</w:tblGrid>",
             paste0(rows, collapse = ""), "</w:tbl>")
    })
}
stata_pd_styles <- function() {
  ns <- "xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\""
  head <- vapply(1:9, function(n) paste0("<w:style w:type=\"paragraph\" w:styleId=\"Heading", n, "\"><w:name w:val=\"heading ", n,
    "\"/><w:basedOn w:val=\"Normal\"/><w:next w:val=\"Normal\"/><w:pPr><w:keepNext/><w:spacing w:before=\"240\" w:after=\"60\"/><w:outlineLvl w:val=\"", n - 1,
    "\"/></w:pPr><w:rPr><w:b/><w:sz w:val=\"", max(22L, 36L - 4L * (n - 1L)), "\"/></w:rPr></w:style>"), "")
  paste0("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><w:styles ", ns, ">",
    "<w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:ascii=\"Calibri\" w:hAnsi=\"Calibri\" w:cs=\"Calibri\"/><w:sz w:val=\"22\"/></w:rPr></w:rPrDefault>",
    "<w:pPrDefault><w:pPr><w:spacing w:after=\"160\" w:line=\"259\" w:lineRule=\"auto\"/></w:pPr></w:pPrDefault></w:docDefaults>",
    "<w:style w:type=\"paragraph\" w:default=\"1\" w:styleId=\"Normal\"><w:name w:val=\"Normal\"/></w:style>",
    "<w:style w:type=\"character\" w:default=\"1\" w:styleId=\"DefaultParagraphFont\"><w:name w:val=\"Default Paragraph Font\"/></w:style>",
    "<w:style w:type=\"paragraph\" w:styleId=\"Title\"><w:name w:val=\"Title\"/><w:basedOn w:val=\"Normal\"/><w:next w:val=\"Normal\"/><w:pPr><w:spacing w:after=\"80\"/></w:pPr><w:rPr><w:sz w:val=\"56\"/></w:rPr></w:style>",
    "<w:style w:type=\"paragraph\" w:styleId=\"Subtitle\"><w:name w:val=\"Subtitle\"/><w:basedOn w:val=\"Normal\"/><w:next w:val=\"Normal\"/><w:rPr><w:color w:val=\"5A5A5A\"/><w:sz w:val=\"30\"/></w:rPr></w:style>",
    paste0(head, collapse = ""),
    "<w:style w:type=\"table\" w:default=\"1\" w:styleId=\"TableNormal\"><w:name w:val=\"Normal Table\"/><w:tblPr><w:tblInd w:w=\"0\" w:type=\"dxa\"/><w:tblCellMar><w:top w:w=\"0\" w:type=\"dxa\"/><w:left w:w=\"108\" w:type=\"dxa\"/><w:bottom w:w=\"0\" w:type=\"dxa\"/><w:right w:w=\"108\" w:type=\"dxa\"/></w:tblCellMar></w:tblPr></w:style>",
    "<w:style w:type=\"table\" w:styleId=\"TableGrid\"><w:name w:val=\"Table Grid\"/><w:basedOn w:val=\"TableNormal\"/><w:tblPr><w:tblBorders>",
    paste0(vapply(c("top", "left", "bottom", "right", "insideH", "insideV"), function(e) paste0("<w:", e, " w:val=\"single\" w:sz=\"4\" w:space=\"0\" w:color=\"auto\"/>"), ""), collapse = ""),
    "</w:tblBorders></w:tblPr></w:style></w:styles>")
}
stata_pd_abs <- function(f) if (grepl("^([A-Za-z]:)?[/\\]|^~", f)) f else file.path(getwd(), f)
stata_pd_write_docx <- function(d, file, append) {
  file <- stata_pd_abs(file)
  ns <- "xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\""
  body <- paste0(vapply(d$blocks, stata_pd_xml, ""), collapse = "")
  td <- tempfile("pd")
  dir.create(td)
  on.exit(unlink(td, recursive = TRUE), add = TRUE)
  put <- function(path, x) {
    p <- file.path(td, path)
    dir.create(dirname(p), showWarnings = FALSE, recursive = TRUE)
    writeBin(charToRaw(enc2utf8(x)), p)
  }
  rd <- function(path) {
    p <- file.path(td, path)
    x <- readChar(p, file.info(p)$size, useBytes = TRUE)
    Encoding(x) <- "UTF-8"
    x
  }
  if (append) {
    utils::unzip(file, exdir = td)
    doc <- rd("word/document.xml")
    hits <- gregexpr("<w:sectPr", doc, fixed = TRUE)[[1]]
    pos <- if (hits[1] > 0) max(hits) else regexpr("</w:body>", doc, fixed = TRUE)[[1]]
    put("word/document.xml", paste0(substr(doc, 1, pos - 1), "<w:p><w:pPr><w:pStyle w:val=\"Normal\"/></w:pPr></w:p>", body, substring(doc, pos)))
  } else {
    put("[Content_Types].xml", paste0("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">",
      "<Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/>",
      "<Override PartName=\"/word/document.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml\"/>",
      "<Override PartName=\"/word/styles.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml\"/></Types>"))
    put("_rels/.rels", paste0("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">",
      "<Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"word/document.xml\"/></Relationships>"))
    put("word/_rels/document.xml.rels", paste0("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">",
      "<Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles\" Target=\"styles.xml\"/></Relationships>"))
    put("word/styles.xml", stata_pd_styles())
    put("word/document.xml", paste0("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><w:document ", ns, "><w:body>", body,
      "<w:sectPr><w:pgSz w:w=\"12240\" w:h=\"15840\"/><w:pgMar w:top=\"1440\" w:right=\"1440\" w:bottom=\"1440\" w:left=\"1440\" w:header=\"720\" w:footer=\"720\" w:gutter=\"0\"/></w:sectPr></w:body></w:document>"))
  }
  files <- list.files(td, recursive = TRUE, all.files = TRUE, include.dirs = FALSE)
  files <- c("[Content_Types].xml", setdiff(files, "[Content_Types].xml"))
  unlink(file)
  zip::zip(file, files, root = td, mode = "mirror")
}
stata_pd_save <- function(k, file, replace = FALSE, append = FALSE) {
  d <- stata_pd_doc(k)
  ext <- if (k == "docx") "docx" else "pdf"
  if (!nzchar(file)) stop("invalid file specification (Stata: r(198))", call. = FALSE)
  ex <- sub("^.*\\.([^./\\\\]+)$", "\\1", basename(file))
  has <- grepl("\\.[^./\\\\]+$", basename(file))
  if (!has) file <- paste0(file, ".", ext)
  else if (tolower(ex) != ext) stop(file, " is not a ", ext, " file (Stata: r(198))", call. = FALSE)
  if (file.exists(file) && !replace && !append) {
    if (k == "docx") stop("file ", file, " already exists\nyou must specify one of the replace , append, and append() options (Stata: r(198))", call. = FALSE)
    stop(file, " already exists\nyou must specify the replace option (Stata: r(198))", call. = FALSE)
  }
  if (append && !file.exists(file)) stop("file ", file, " not found (Stata: r(601))", call. = FALSE)
  if (k == "docx") stata_pd_write_docx(d, file, append && file.exists(file)) else stata_pd_write_pdf(d, file)
  stata_pd[[k]] <<- NULL
  cat("successfully ", if (append) "appended to" else "created", " \"", normalizePath(stata_pd_abs(file), winslash = "/", mustWork = FALSE), "\"\n", sep = "")
  invisible(NULL)
}
# the document drawn by R's pdf() device: A4 pages, 36 points of margin, Helvetica 11 (as the file Stata 19 made here)
stata_pd_write_pdf <- function(d, file) {
  file <- stata_pd_abs(file)
  W <- 595.276; H <- 841.89; M <- 36; TOP <- 85.89; CW <- W - 2 * M; LH <- 15.09; GAP <- 8; RH <- 18.716
  grDevices::pdf(file, width = W / 72, height = H / 72, paper = "special", family = "Helvetica", compress = FALSE, title = "")
  dev <- grDevices::dev.cur()
  on.exit(grDevices::dev.off(dev), add = TRUE)
  y <- TOP
  newpage <- function() {
    graphics::par(mar = c(0, 0, 0, 0), xaxs = "i", yaxs = "i")
    graphics::plot.new()
    graphics::plot.window(xlim = c(0, W), ylim = c(H, 0))
    y <<- TOP
  }
  newpage()
  sw <- function(s, f) if (nzchar(s)) graphics::strwidth(s, units = "inches", cex = 11 / 12, font = f) * 72 else 0
  put <- function(x, yy, s, f) graphics::text(x, yy, s, adj = c(0, 0), cex = 11 / 12, font = f, xpd = NA)
  for (b in d$blocks) {
    if (b$t == "pb") { newpage(); next }
    if (b$t == "p") {
      toks <- list()
      for (ri in seq_along(b$runs)) {
        r <- b$runs[[ri]]
        f <- 1L + isTRUE(r$b) + 2L * isTRUE(r$i)
        for (t in regmatches(r$text, gregexpr("[^ ]+ *| +", r$text))[[1]]) toks[[length(toks) + 1L]] <- list(t = t, f = f, run = ri, u = isTRUE(r$u), w = sw(t, f), wt = sw(sub(" +$", "", t), f))
        if (isTRUE(r$br)) toks[[length(toks) + 1L]] <- list(t = "\n", f = f, run = ri, u = FALSE, w = 0, wt = 0)
      }
      lines <- list(); cur <- list(); cw <- 0
      for (t in toks) {
        if (t$t == "\n") { lines[[length(lines) + 1L]] <- cur; cur <- list(); cw <- 0; next }
        if (length(cur) && cw + t$wt > CW) { lines[[length(lines) + 1L]] <- cur; cur <- list(); cw <- 0 }
        cur[[length(cur) + 1L]] <- t
        cw <- cw + t$w
      }
      if (length(cur) || !length(lines)) lines[[length(lines) + 1L]] <- cur
      for (ln in lines) {
        if (y + LH > H - M) newpage()
        tot <- sum(vapply(ln, function(t) t$w, 0)) - if (length(ln)) (ln[[length(ln)]]$w - ln[[length(ln)]]$wt) else 0
        x <- M + switch(b$jc, center = (CW - tot) / 2, right = CW - tot, 0)
        i <- 1L
        while (i <= length(ln)) {
          j <- i
          while (j < length(ln) && ln[[j + 1L]]$run == ln[[i]]$run) j <- j + 1L
          s <- paste0(vapply(ln[i:j], function(t) t$t, ""), collapse = "")
          wd <- sum(vapply(ln[i:j], function(t) t$w, 0))
          put(x, y + 10.8, s, ln[[i]]$f)
          if (ln[[i]]$u) graphics::segments(x, y + 12.3, x + sw(sub(" +$", "", s), ln[[i]]$f), y + 12.3, lwd = 0.5)
          x <- x + wd
          i <- j + 1L
        }
        y <- y + LH
      }
      y <- y + GAP
    } else if (b$t == "tbl") {
      nc <- sum(vapply(b$cells[[1]], function(z) z$span, 1L))
      cw <- CW / nc
      nr <- length(b$cells)
      for (i in seq_len(nr)) {
        if (y + RH > H - M) newpage()
        x <- M; col <- 0L
        for (cell in b$cells[[i]]) {
          w <- cw * cell$span
          col <- col + cell$span
          if (!b$etable) graphics::rect(x, y + RH, x + w, y, lwd = 0.75)
          else {
            if (i <= 2L) graphics::segments(x, y, x + w, y, lwd = 0.75)
            if (i == 1L || i == nr) graphics::segments(x, y + RH, x + w, y + RH, lwd = 0.75)
            if (col == 1L) graphics::segments(x + w, y, x + w, y + RH, lwd = 0.75)
          }
          if (nzchar(cell$text)) put(if (identical(cell$jc, "right")) x + w - 2 - sw(cell$text, 1L) else x + 2, y + 13.44, cell$text, 1L)
          x <- x + w
        }
        y <- y + RH
      }
      y <- y + GAP
    }
  }
  invisible(NULL)
}"##;

/// The text inside the first balanced parentheses of `s` (which must start
/// with one), and what follows the closing one. Quotes keep a `)` inside text.
fn paren(s: &str) -> Option<(&str, &str)> {
    let s = s.trim_start();
    if !s.starts_with('(') {
        return None;
    }
    let mut depth = 0usize;
    let mut quoted = false;
    for (i, ch) in s.char_indices() {
        match ch {
            '"' => quoted = !quoted,
            '(' if !quoted => depth += 1,
            ')' if !quoted => {
                depth -= 1;
                if depth == 0 {
                    return Some((s.get(1..i)?, s.get(i + 1..)?));
                }
            }
            _ => {}
        }
    }
    None
}

/// `%w.df` and its kin, as Stata's `nformat()` takes them.
fn nformat_ok(f: &str) -> bool {
    let Some(rest) = f.strip_prefix('%') else {
        return false;
    };
    let rest = rest.strip_prefix('-').unwrap_or(rest);
    let rest = rest.strip_prefix('0').unwrap_or(rest);
    let Some((w, d)) = rest.split_once('.') else {
        return false;
    };
    let d = d.strip_suffix('c').unwrap_or(d);
    let Some(d) = d
        .strip_suffix('f')
        .or_else(|| d.strip_suffix('g'))
        .or_else(|| d.strip_suffix('e'))
    else {
        return false;
    };
    !w.is_empty()
        && w.bytes().all(|b| b.is_ascii_digit())
        && !d.is_empty()
        && d.bytes().all(|b| b.is_ascii_digit())
}

impl Session {
    pub(super) fn putdoc(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let cmd = c.name.as_str();
        let docx = cmd == "putdocx";
        let k = if docx { "docx" } else { "pdf" };
        let stop = |out: &mut Out, msg: &str| {
            out.line(format!("stop({}, call. = FALSE)", r_text(msg)));
        };
        let main = c.parts.main.trim();
        let (sub, rest) = first_word(main);
        let rest = rest.trim();
        if c.parts.if_exp.is_some() && sub != "table"
            || c.parts.in_range.is_some() && sub != "table"
        {
            return Err(no(format!(
                "{cmd} {sub} with if or in is not translated yet"
            )));
        }
        self.helpers.insert("stata_putdoc");
        self.helpers.insert("stata_fmt");
        let refuse_opts = |known: &[(&str, usize)]| -> Result<(), Untranslated> {
            match opts.other(known) {
                Some(o) => Err(no(format!(
                    "{cmd} {sub} with the option {o} is not translated yet"
                ))),
                None => Ok(()),
            }
        };
        let quoted = |s: &str| r_text(k) + ", " + &r_text(s);
        match sub {
            "begin" | "clear" | "pagebreak" => {
                refuse_opts(&[])?;
                if !rest.is_empty() {
                    return Err(no(format!(
                        "{cmd} {sub} with more than its name is not translated yet"
                    )));
                }
                out.line(format!("stata_pd_{sub}({})", r_text(k)));
                Ok(())
            }
            "paragraph" => {
                refuse_opts(&[("style", 1), ("halign", 1)])?;
                if !rest.is_empty() {
                    return Err(no(format!(
                        "{cmd} paragraph with more than its name is not translated yet"
                    )));
                }
                let mut args = String::new();
                if let Some(s) = opts.arg("style", 1) {
                    let s = crate::dofile::macros::unquote(s.trim());
                    if !docx {
                        // putpdf has no style(): Stata 19 says so, with the option as typed
                        stop(
                            out,
                            &format!("option style({s}) not allowed (Stata: r(198))"),
                        );
                        return Ok(());
                    }
                    let ok = s == "Title"
                        || s == "Subtitle"
                        || s.strip_prefix("Heading")
                            .and_then(|n| n.parse::<u32>().ok())
                            .is_some_and(|n| {
                                (1..=9).contains(&n) && n.to_string() == s["Heading".len()..]
                            });
                    if !ok {
                        // Stata 19 refused Normal, title, heading1, Emphasis, Caption and Heading10
                        if [
                            "Normal",
                            "title",
                            "heading1",
                            "Emphasis",
                            "Caption",
                            "Heading10",
                        ]
                        .contains(&s.as_str())
                        {
                            stop(out, "option style() specified incorrectly (Stata: r(198))");
                            return Ok(());
                        }
                        return Err(no(format!("putdocx paragraph, style({s}): only Title, Subtitle and Heading1 to Heading9 are translated")));
                    }
                    args.push_str(&format!(", style = {}", r_text(&s)));
                }
                if let Some(h) = opts.arg("halign", 1) {
                    let h = crate::dofile::macros::unquote(h.trim());
                    if !["left", "center", "right"].contains(&h.as_str()) {
                        if h == "justified" || h == "both" {
                            return Err(no(format!(
                                "putdoc paragraph, halign({h}) is not translated yet"
                            )));
                        }
                        stop(out, "option halign() specified incorrectly (Stata: r(198))");
                        return Ok(());
                    }
                    args.push_str(&format!(", halign = {}", r_text(&h)));
                }
                out.line(format!("stata_pd_paragraph({}{args})", r_text(k)));
                Ok(())
            }
            "text" => {
                refuse_opts(&[
                    ("bold", 1),
                    ("italic", 1),
                    ("underline", 1),
                    ("linebreak", 2),
                    ("nformat", 2),
                ])?;
                let Some((inner, after)) = paren(rest) else {
                    return Err(no(format!(
                        "{cmd} text is translated for text (expression) only"
                    )));
                };
                if !after.trim().is_empty() {
                    return Err(no(format!(
                        "{cmd} text with more than one expression is not translated yet"
                    )));
                }
                let (code, _) = self.value(inner, out)?;
                let mut args = String::new();
                for (flag, name, min) in [
                    ("bold", "bold", 1),
                    ("italic", "italic", 1),
                    ("underline", "underline", 1),
                    ("linebreak", "linebreak", 2),
                ] {
                    if opts.has(name, min) {
                        args.push_str(&format!(", {flag} = TRUE"));
                    }
                }
                if let Some(f) = opts.arg("nformat", 2) {
                    let f = crate::dofile::macros::unquote(f.trim());
                    if !nformat_ok(&f) {
                        stop(
                            out,
                            "option nformat() specified incorrectly (Stata: r(198))",
                        );
                        return Ok(());
                    }
                    args.push_str(&format!(", nformat = {}", r_text(&f)));
                }
                out.line(format!("stata_pd_text({}, {code}{args})", r_text(k)));
                Ok(())
            }
            "describe" => {
                refuse_opts(&[])?;
                if rest.is_empty() {
                    out.line(format!("stata_pd_describe({})", r_text(k)));
                } else if is_name(rest) {
                    out.line(format!(
                        "stata_r <- stata_pd_describe({}, {})",
                        r_text(k),
                        r_text(rest)
                    ));
                    self.r_new = Some((returned::R_PUTDOC_DESCRIBE, format!("{cmd} describe")));
                } else {
                    return Err(no(format!(
                        "{cmd} describe with more than a table name is not translated yet"
                    )));
                }
                Ok(())
            }
            "save" => {
                refuse_opts(&[("replace", 1), ("append", 1)])?;
                if rest.is_empty() {
                    stop(out, "invalid file specification (Stata: r(198))");
                    return Ok(());
                }
                if !docx && opts.has("append", 1) {
                    stop(out, "option append not allowed (Stata: r(198))");
                    return Ok(());
                }
                if opts.raw("append", 1).is_some() {
                    return Err(no("putdocx save, append(docname) is not translated yet"));
                }
                let file = file_name(rest, "");
                out.package("zip");
                out.line(format!(
                    "stata_pd_save({}, replace = {}, append = {})",
                    quoted(&file),
                    super::stats::r_logical(opts.has("replace", 1)),
                    super::stats::r_logical(opts.has("append", 1))
                ));
                Ok(())
            }
            "table" => self.putdoc_table(c, opts, k, rest, out),
            "image" | "sectionbreak" | "pagenumber" | "textblock" | "textfile" | "append"
            | "collect" => Err(no(format!("{cmd} {sub} is not translated yet"))),
            _ => Err(no(format!("{cmd} {sub} is not translated yet"))),
        }
    }

    fn putdoc_table(
        &mut self,
        c: &Command,
        opts: &Opts,
        k: &str,
        rest: &str,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let cmd = c.name.as_str();
        let stop = |out: &mut Out, msg: &str| {
            out.line(format!("stop({}, call. = FALSE)", r_text(msg)));
        };
        // name = ... or name(i,j) = ...
        let Some((lhs, rhs)) = rest.split_once('=') else {
            if rest.is_empty() {
                stop(out, "table or cell specification required (Stata: r(198))");
                return Ok(());
            }
            return Err(no(format!(
                "{cmd} table is translated for name = ... and name(i,j) = ..."
            )));
        };
        let (lhs, rhs) = (lhs.trim(), rhs.trim());
        // a cell
        if let Some((name, cell)) = lhs.split_once('(') {
            let name = name.trim();
            let Some(cell) = cell.trim().strip_suffix(')') else {
                return Err(no(format!("{cmd} table {lhs}: a cell is name(row,column)")));
            };
            let ij: Vec<&str> = cell.split(',').map(str::trim).collect();
            let [i, j] = ij.as_slice() else {
                return Err(no(format!("{cmd} table {lhs}: a cell is name(row,column)")));
            };
            if !is_name(name) {
                return Err(no(format!("{cmd} table {lhs}: not a table name")));
            }
            let (i, _) = self.value(i, out)?;
            let (j, _) = self.value(j, out)?;
            if let Some(o) = opts.other(&[("nformat", 2)]) {
                return Err(no(format!(
                    "{cmd} table cell with the option {o} is not translated yet"
                )));
            }
            let Some((inner, after)) = paren(rhs) else {
                return Err(no(format!("{cmd} table cell = needs (expression)")));
            };
            if !after.trim().is_empty() {
                return Err(no(format!(
                    "{cmd} table cell with more than one expression is not translated yet"
                )));
            }
            let (code, _) = self.value(inner, out)?;
            let mut nf = String::new();
            if let Some(f) = opts.arg("nformat", 2) {
                let f = crate::dofile::macros::unquote(f.trim());
                if !nformat_ok(&f) {
                    stop(
                        out,
                        "option nformat() specified incorrectly (Stata: r(198))",
                    );
                    return Ok(());
                }
                nf = format!(", nformat = {}", r_text(&f));
            }
            out.line(format!(
                "stata_pd_set_cell({}, {}, {i}, {j}, {code}{nf})",
                r_text(k),
                r_text(name)
            ));
            return Ok(());
        }
        let name = lhs;
        if !is_name(name) {
            return Err(no(format!("{cmd} table {name}: not a table name")));
        }
        // name = (rows, columns)
        if rhs.starts_with('(') {
            if let Some(o) = opts.other(&[]) {
                return Err(no(format!(
                    "{cmd} table with the option {o} is not translated yet"
                )));
            }
            let Some((inner, after)) = paren(rhs) else {
                return Err(no(format!("{cmd} table size: a ( without its )")));
            };
            let sz: Vec<&str> = inner.split(',').map(str::trim).collect();
            let [r, cc] = sz.as_slice() else {
                return Err(no(format!("{cmd} table {name} = (rows, columns)")));
            };
            if !after.trim().is_empty() {
                return Err(no(format!(
                    "{cmd} table size with more than (rows, columns) is not translated yet"
                )));
            }
            let (r, _) = self.value(r, out)?;
            let (cc, _) = self.value(cc, out)?;
            out.line(format!(
                "stata_pd_table_size({}, {}, {r}, {cc})",
                r_text(k),
                r_text(name)
            ));
            return Ok(());
        }
        // name = etable
        if rhs == "etable" {
            if let Some(o) = opts.other(&[]) {
                return Err(no(format!(
                    "{cmd} table = etable with the option {o} is not translated yet"
                )));
            }
            if self.estimates_lost.is_some() {
                return Err(no(format!(
                    "{cmd} table = etable: the model Stata holds here is not known"
                )));
            }
            let fit = self.estimates.as_ref().map(|e| e.fit);
            // Stata takes the table from r(), which the last printed estimation command
            // left: none after quietly, or once a command has replaced r()
            if self.est_shown == Some(false) || fit.is_none() {
                stop(
                    out,
                    "information for the estimation table not found;\n    Please replay estimation results and try again (Stata: r(111))",
                );
                return Ok(());
            }
            if self.est_shown.is_none() {
                return Err(no(format!(
                    "{cmd} table = etable: an earlier line may have replaced the r() the table is kept in"
                )));
            }
            if fit != Some(super::model::Fit::Regress) {
                return Err(no(format!(
                    "{cmd} table = etable is translated after regress only, so far"
                )));
            }
            if self.weighted_fit {
                return Err(no(format!(
                    "{cmd} table = etable after a fit with weights is not translated yet"
                )));
            }
            for h in [
                "stata_tables",
                "stata_fmt",
                "stata_coefs",
                "stata_ml_header",
            ] {
                self.helpers.insert(h);
            }
            out.line(format!(
                "stata_pd_table_etable({}, {})",
                r_text(k),
                r_text(name)
            ));
            return Ok(());
        }
        // name = matrix(M)
        if let Some(arg) = rhs.strip_prefix("matrix") {
            if let Some((inner, after)) = paren(arg) {
                if !after.trim().is_empty() {
                    return Err(no(format!(
                        "{cmd} table matrix() with more than one matrix is not translated yet"
                    )));
                }
                if let Some(o) = opts.other(&[("rownames", 4), ("colnames", 4), ("nformat", 2)]) {
                    return Err(no(format!(
                        "{cmd} table = matrix() with the option {o} is not translated yet"
                    )));
                }
                let mname = inner.trim();
                if is_name(mname)
                    && !self.user_mats.contains(mname)
                    && !self.lost_mats.contains_key(mname)
                {
                    // never made: Stata stops with the name and "not found" (r(111))
                    stop(out, &format!("{mname} not found (Stata: r(111))"));
                    return Ok(());
                }
                let (m, kind) = self.matrix_expr(mname, out)?;
                if !matches!(kind, super::mexpr::MKind::Matrix) {
                    return Err(no(format!(
                        "{cmd} table = matrix({inner}): a matrix is translated here"
                    )));
                }
                let mut args = String::new();
                if opts.has("rownames", 4) {
                    args.push_str(", rows = TRUE");
                }
                if opts.has("colnames", 4) {
                    args.push_str(", cols = TRUE");
                }
                if let Some(f) = opts.arg("nformat", 2) {
                    let f = crate::dofile::macros::unquote(f.trim());
                    if !nformat_ok(&f) {
                        stop(
                            out,
                            "option nformat() specified incorrectly (Stata: r(198))",
                        );
                        return Ok(());
                    }
                    args.push_str(&format!(", nformat = {}", r_text(&f)));
                }
                self.helpers.insert("stata_matrix");
                out.line(format!(
                    "stata_pd_table_matrix({}, {}, {m}{args})",
                    r_text(k),
                    r_text(name)
                ));
                return Ok(());
            }
        }
        // name = data(varlist) [if] [in] [, varnames obsno]
        if let Some(arg) = rhs.strip_prefix("data") {
            if let Some((inner, after)) = paren(arg) {
                if !after.trim().is_empty() {
                    return Err(no(format!(
                        "{cmd} table data() with more than a varlist is not translated yet"
                    )));
                }
                if let Some(o) = opts.other(&[("varnames", 4), ("obsno", 3)]) {
                    return Err(no(format!(
                        "{cmd} table = data() with the option {o} is not translated yet"
                    )));
                }
                let d = self.data()?;
                let vars = inner.trim();
                if vars.is_empty() {
                    return Err(no(format!("{cmd} table = data() needs a varlist")));
                }
                // a variable that is not there stops as Stata does (r(111))
                for v in vars.split_whitespace().filter(|v| is_name(v)) {
                    out.line(format!(
                        "if (!{} %in% names({d})) stop(\"variable {v} not found (Stata: r(111))\", call. = FALSE)",
                        r_text(v)
                    ));
                }
                let mut steps = vec![format!(
                    "dplyr::mutate({d}, stata_obs_ = dplyr::row_number())"
                )];
                if let Some(cond) = self.condition(c, out)? {
                    steps.push(format!("dplyr::filter({cond})"));
                }
                steps.push(format!("dplyr::select({}, stata_obs_)", select(vars)?));
                let defaults: Vec<String> = self
                    .storage
                    .iter()
                    .filter_map(|(v, t)| {
                        Some(format!("{} = {}", r_text(v), r_text(t.default_format()?)))
                    })
                    .collect();
                let mut args = String::new();
                if opts.has("varnames", 4) {
                    args.push_str(", varnames = TRUE");
                }
                if opts.has("obsno", 3) {
                    args.push_str(", obsno = TRUE");
                }
                if !defaults.is_empty() {
                    args.push_str(&format!(", defaults = c({})", defaults.join(", ")));
                }
                for h in ["stata_float", "stata_fmt"] {
                    self.helpers.insert(h);
                }
                out.package("dplyr");
                out.package("haven");
                out.line(format!(
                    "stata_pd_table_data({}, {}, {}{args})",
                    r_text(k),
                    r_text(name),
                    steps.join(" |> ")
                ));
                return Ok(());
            }
        }
        Err(no(format!(
            "{cmd} table {name} = {rhs}: only (rows, columns), data(), matrix() and etable are translated"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parentheses_and_formats_are_read_as_stata_does() {
        assert_eq!(paren(r#"("a)b") , bold"#), Some(("\"a)b\"", " , bold")));
        assert_eq!(paren("(1/(2+1))"), Some(("1/(2+1)", "")));
        assert!(paren("x(1)").is_none() && paren("(1").is_none());
        assert!(nformat_ok("%5.3f") && nformat_ok("%-9.2f") && nformat_ok("%9.0g"));
        assert!(!nformat_ok("bogus") && !nformat_ok("%5f") && !nformat_ok("%.3f"));
    }
}
