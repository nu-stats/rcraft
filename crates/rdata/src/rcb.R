function(df, path, max_cells = 2e8) {
  # RCraft columnar, version 1. Base R only: no package is needed, so it works
  # wherever R does. The Rust reader is crates/rdata/src/lib.rs; the layout is
  # described there.
  if (!is.data.frame(df)) stop("not a data frame")
  n <- nrow(df)
  p <- length(df)
  if (as.numeric(n) * p > max_cells) {
    stop("the table is too large to bring over (", n, " rows by ", p, " columns)")
  }
  con <- file(path, "wb")
  on.exit(close(con), add = TRUE)
  i4 <- function(v) writeBin(as.integer(v), con, size = 4L, endian = "little")
  d8 <- function(v) writeBin(as.double(v), con, size = 8L, endian = "little")
  txt <- function(x) {
    x <- enc2utf8(as.character(x))
    na <- is.na(x)
    len <- nchar(x, "bytes")
    len[na] <- -1L
    blob <- charToRaw(paste(x[!na], collapse = ""))
    if (length(blob) >= 2147483647) stop("a text column is too large to bring over")
    i4(len)
    i4(length(blob))
    writeBin(blob, con)
  }
  str1 <- function(s) {
    r <- charToRaw(enc2utf8(as.character(s)[[1L]]))
    i4(length(r))
    writeBin(r, con)
  }
  as_text <- function(v) {
    if (is.list(v) && !is.data.frame(v)) {
      vapply(v, function(e) paste(format(e), collapse = ", "), "", USE.NAMES = FALSE)
    } else if (length(dim(v)) == 2L || is.data.frame(v)) {
      apply(as.matrix(format(v)), 1L, paste, collapse = ", ")
    } else {
      as.character(format(v))
    }
  }
  skip <- c("class", "levels", "names", "dim", "dimnames", "tzone", "units", "tsp", "comment")
  attr_block <- function(v) {
    at <- attributes(v)
    at <- at[setdiff(names(at), skip)]
    keep <- list()
    dropped <- 0L
    for (k in names(at)) {
      a <- at[[k]]
      ok <- nzchar(k) && is.atomic(a) && !is.object(a) && length(a) <= 10000L &&
        (is.character(a) || is.numeric(a) || is.logical(a))
      if (ok) keep[[k]] <- a else dropped <- dropped + 1L
    }
    i4(length(keep))
    i4(dropped)
    for (k in names(keep)) {
      a <- keep[[k]]
      nm <- names(a)
      str1(k)
      if (is.character(a)) {
        i4(6L)
        i4(length(a))
        txt(a)
      } else {
        i4(2L)
        i4(length(a))
        d8(a)
      }
      i4(!is.null(nm))
      if (!is.null(nm)) txt(nm)
    }
  }

  writeBin(charToRaw("RCB1"), con)
  i4(p)
  i4(n)
  nms <- names(df)
  if (is.null(nms)) nms <- rep("", p)
  for (j in seq_len(p)) {
    v <- df[[j]]
    str1(nms[[j]])
    str1(paste(class(v), collapse = " "))
    if (inherits(v, "POSIXlt")) v <- as.POSIXct(v)
    ok <- is.null(dim(v)) && !is.data.frame(v) && !is.list(v) && length(v) == n
    if (ok && is.factor(v)) {
      i4(4L)
      attr_block(v)
      i4(is.ordered(v))
      i4(nlevels(v))
      txt(levels(v))
      codes <- as.integer(v) - 1L
      codes[is.na(codes)] <- -1L
      i4(codes)
    } else if (ok && inherits(v, "Date")) {
      i4(5L)
      attr_block(v)
      d8(v)
    } else if (ok && inherits(v, "POSIXct")) {
      i4(7L)
      attr_block(v)
      tz <- attr(v, "tzone")
      str1(if (length(tz)) tz[[1L]] else "")
      d8(v)
    } else if (ok && inherits(v, "difftime")) {
      i4(8L)
      attr_block(v)
      str1(attr(v, "units"))
      d8(v)
    } else if (ok && is.logical(v)) {
      i4(3L)
      attr_block(v)
      r <- as.integer(v)
      r[is.na(r)] <- -1L
      i4(r)
    } else if (ok && is.integer(v)) {
      i4(1L)
      attr_block(v)
      i4(v)
    } else if (ok && is.double(v)) {
      i4(2L)
      attr_block(v)
      d8(v)
    } else if (ok && is.character(v)) {
      i4(6L)
      attr_block(v)
      txt(v)
    } else {
      i4(9L)
      attr_block(v)
      txt(as_text(v))
    }
  }
  pos <- seek(con, rw = "write")
  writeBin(charToRaw("RCBE"), con)
  d8(pos)
  invisible(c(rows = n, cols = p))
}
