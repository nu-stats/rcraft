# ---- the XPORT version 5 pieces (IBM floats, 80-byte records)
stata_xpt_pad <- function(text, n) {
  b <- charToRaw(enc2utf8(text))
  b <- b[seq_len(min(length(b), n))]
  c(b, rep(as.raw(0x20), n - length(b)))
}
stata_xpt_ibm <- function(x) {
  tags <- rep(NA_character_, length(x))
  miss <- is.na(x)
  if (any(miss)) tags[miss] <- haven::na_tag(x[miss])
  out <- raw(8 * length(x))
  for (i in seq_along(x)) {
    v <- x[i]
    at <- 8 * (i - 1)
    if (miss[i]) {
      out[at + 1] <- if (is.na(tags[i])) as.raw(0x2e) else charToRaw(toupper(tags[i]))
      next
    }
    if (v == 0 || !is.finite(v)) next
    b <- as.integer(writeBin(abs(v), raw(), endian = "big"))
    e11 <- (b[1] %% 128) * 16 + b[2] %/% 16
    if (e11 == 0) next
    m <- (b[2] %% 16) * 2^48 + b[3] * 2^40 + b[4] * 2^32 + b[5] * 2^24 + b[6] * 2^16 + b[7] * 2^8 + b[8] + 2^52
    e2 <- e11 - 1023
    k <- (e2 + 4) %/% 4
    sh <- (e2 + 4) %% 4
    ex <- k + 64
    if (ex < 0 || ex > 127) next
    fr <- m * 2^sh
    bytes <- vapply(6:0, function(j) (fr %/% 2^(8 * j)) %% 256, 0)
    out[at + 1] <- as.raw((if (v < 0) 128 else 0) + ex)
    out[at + 2:8] <- as.raw(bytes)
  }
  out
}
stata_xpt_i16 <- function(n) as.raw(c((n %/% 256) %% 256, n %% 256))
stata_xpt_i32 <- function(n) as.raw(c((n %/% 16777216) %% 256, (n %/% 65536) %% 256, (n %/% 256) %% 256, n %% 256))
stata_xpt_header <- function(kind, numbers) c(charToRaw("HEADER RECORD*******"), stata_xpt_pad(kind, 8), charToRaw("HEADER RECORD!!!!!!!"), stata_xpt_pad(numbers, 30), charToRaw("  "))
stata_xpt_real <- function(name, kind, stamp) c(stata_xpt_pad("SAS", 8), stata_xpt_pad(name, 8), stata_xpt_pad(kind, 8), stata_xpt_pad("6.06", 8), stata_xpt_pad("bsd4.2", 8), stata_xpt_pad("", 24), stata_xpt_pad(stamp, 16), stata_xpt_pad(stamp, 16), stata_xpt_pad("", 64))
stata_xpt_record <- function(b) { r <- length(b) %% 80; if (r) c(b, rep(as.raw(0x20), 80 - r)) else b }
# vars: list of list(name, label, format, fwidth, numeric = TRUE/FALSE, values = numeric or character, width)
stata_xpt_write <- function(member, stamp, vars) {
  out <- c(stata_xpt_header("LIBRARY", strrep("0", 30)), stata_xpt_real("SAS", "SASLIB", stamp),
           stata_xpt_header("MEMBER", "000000000000000001600000000140"), stata_xpt_header("DSCRPTR", strrep("0", 30)),
           stata_xpt_real(member, "SASDATA", stamp),
           stata_xpt_header("NAMESTR", sprintf("000000%04d00000000000000000000", min(length(vars), 9999))))
  pos <- 0
  for (k in seq_along(vars)) {
    v <- vars[[k]]
    w <- if (v$numeric) 8 else v$width
    out <- c(out, stata_xpt_i16(if (v$numeric) 1 else 2), as.raw(c(0, 0)), stata_xpt_i16(w), stata_xpt_i16(k),
             stata_xpt_pad(v$name, 8), stata_xpt_pad(v$label, 40), stata_xpt_pad(v$format, 8), stata_xpt_i16(v$fwidth),
             as.raw(c(0, 0, 0, 0)), charToRaw("  "), stata_xpt_pad("", 8), as.raw(c(0, 0, 0, 0)), stata_xpt_i32(pos), raw(52))
    pos <- pos + w
  }
  out <- stata_xpt_record(out)
  out <- c(out, stata_xpt_header("OBS", strrep("0", 30)))
  n <- if (length(vars)) length(vars[[1]]$values) else 0
  cells <- lapply(vars, function(v) if (v$numeric) matrix(stata_xpt_ibm(v$values), nrow = 8) else matrix(unlist(lapply(v$values, function(s) stata_xpt_pad(s, v$width))), nrow = v$width))
  if (n > 0) {
    rows <- vector("list", n)
    for (i in seq_len(n)) rows[[i]] <- unlist(lapply(cells, function(m) if (is.matrix(m)) m[, i] else m[, i]), use.names = FALSE)
    out <- c(out, unlist(rows, use.names = FALSE))
  }
  stata_xpt_record(out)
}
stata_xpt_from_ibm <- function(b) {
  b <- as.integer(b)
  frac <- sum(b[2:8] * 2^(8 * (6:0)))
  if (frac == 0) {
    if (b[1] %in% c(0x2e, 0x5f)) return(NA_real_)
    if (b[1] >= 0x41 && b[1] <= 0x5a) return(haven::tagged_na(tolower(rawToChar(as.raw(b[1])))))
    return(0)
  }
  v <- frac * 2^(4 * ((b[1] %% 128) - 64) - 56)
  if (b[1] >= 128) -v else v
}
stata_xpt_read <- function(bytes) {
  rec <- function(at) bytes[(at + 1):(at + 80)]
  fld <- function(r, from, len) sub(" +$", "", rawToChar(r[(from + 1):(from + len)]))
  if (length(bytes) < 80 || !identical(bytes[1:48], charToRaw("HEADER RECORD*******LIBRARY HEADER RECORD!!!!!!!"))) return(NULL)
  at <- 240
  size <- suppressWarnings(as.integer(fld(rec(at), 74, 4)))
  if (is.na(size)) size <- 140
  size <- max(1, min(size, 140))
  at <- at + 80 * 4
  count <- as.integer(fld(rec(at), 54, 4))
  at <- at + 80
  layout <- lapply(seq_len(count), function(k) {
    r <- bytes[(at + (k - 1) * size + 1):(at + k * size)]
    i16 <- function(p) as.integer(r[p + 1]) * 256 + as.integer(r[p + 2])
    list(name = fld(r, 8, 8), label = fld(r, 16, 40), format = fld(r, 56, 8), fwidth = i16(64), numeric = i16(0) == 1,
         width = max(i16(4), 1), pos = sum(as.integer(r[85:88]) * 256^(3:0)))
  })
  at <- at + count * size
  at <- ceiling(at / 80) * 80 + 80
  end <- length(bytes)
  look <- at
  while (look + 80 <= length(bytes)) {
    if (identical(bytes[(look + 1):(look + 28)], charToRaw("HEADER RECORD*******MEMBER  "))) { end <- look; break }
    look <- look + 80
  }
  data <- if (end > at) bytes[(at + 1):end] else raw(0)
  row <- max(vapply(layout, function(l) l$pos + l$width, 0), 0)
  rows <- if (row > 0) length(data) %/% row else 0
  while (rows > 0 && all(data[((rows - 1) * row + 1):(rows * row)] == as.raw(0x20))) rows <- rows - 1
  for (j in seq_along(layout)) {
    l <- layout[[j]]
    if (l$numeric) {
      layout[[j]]$values <- vapply(seq_len(rows), function(r) {
        cell <- data[(r - 1) * row + l$pos + seq_len(min(l$width, 8))]
        cell <- c(cell, raw(8 - length(cell)))
        stata_xpt_from_ibm(cell)
      }, 0)
    } else {
      layout[[j]]$values <- vapply(seq_len(rows), function(r) sub(" +$", "", rawToChar(data[(r - 1) * row + l$pos + seq_len(l$width)])), "")
    }
  }
  layout
}
