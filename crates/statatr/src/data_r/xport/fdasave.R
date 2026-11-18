stata_fdasave <- function(d, file, shown, rename = FALSE, replace = FALSE, vallabfile = "xpf") {
  vars <- names(d)
  if (!length(vars)) stop("no variables defined (Stata: r(111))", call. = FALSE)
  if (replace && file.exists(file)) file.remove(file)
  if (!nrow(d)) stop("no observations (Stata: r(2000))", call. = FALSE)
  if (file.exists(file)) stop("file ", shown, " already exists (Stata: r(602))", call. = FALSE)
  formats <- file.path(dirname(file), "formats.xpf")
  if (file.exists(formats)) file.remove(formats)
  too_big <- 9.046256971665327e74
  strwidth <- function(x) {
    fmt <- attr(x, "format.stata", exact = TRUE)
    w <- suppressWarnings(as.integer(sub("^%-?([0-9]+)s$", "\\1", fmt)))
    if (is.null(fmt) || is.na(w)) w <- max(c(nchar(x, "bytes"), 1L), na.rm = TRUE)
    w
  }
  for (v in vars) {
    x <- d[[v]]
    if (is.character(x)) {
      if (strwidth(x) > 200) stop(v, ":  string variable too long for .xpt format (Stata: r(459))", call. = FALSE)
    } else {
      y <- as.numeric(unclass(x))
      if (any(!is.na(y) & abs(y) >= too_big)) stop(v, ":  variable contains |value| >= 9.046e+74; too large for .xpt format (Stata: r(459))", call. = FALSE)
    }
  }
  used <- character()
  for (v in vars) {
    x <- d[[v]]
    if (is.character(x)) next
    labs <- attr(x, "labels", exact = TRUE)
    nm <- attr(x, "vallab", exact = TRUE)
    if (!is.null(nm) && !length(labs)) stop("value label ", nm, " not found (Stata: r(111))", call. = FALSE)
    if (!length(labs)) next
    if (is.null(nm)) stop("RCraft: the name of the value label of ", v, " is not known here, so the .xpt file cannot be written like Stata's (not translated yet)", call. = FALSE)
    if (!(nm %in% used)) used <- c(used, nm)
  }
  up <- function(s, n) toupper(substr(s, 1, n))
  sas <- character()
  listed <- logical()
  for (v in vars) {
    cand <- up(v, 8)
    n <- 2
    while (cand %in% sas) {
      dg <- as.character(n)
      cand <- paste0(up(v, 8 - nchar(dg)), dg)
      n <- n + 1
    }
    sas <- c(sas, cand)
    listed <- c(listed, nchar(v, "bytes") > 8 || v != tolower(v) || cand != up(v, 8))
  }
  ch <- which(listed)
  show <- function() {
    cat("\n")
    cat(sprintf("%32s -> %s", vars[ch], sas[ch]), sep = "\n")
    cat("\n")
  }
  if (length(ch)) {
    if (!rename) {
      cat("the following variable(s) have names that must be changed to fit into .xpt format:  (suggested renamings shown):\n")
      show()
      cat("specify option rename to save .xpt file with suggested names\n")
      stop("the names must be changed first (Stata: r(110))", call. = FALSE)
    }
    cat("the following variable(s) were renamed in the output file:\n")
    show()
  }
  keytext <- function(k) if (is.na(k)) paste0(".", haven::na_tag(k)) else format(k, scientific = FALSE, trim = TRUE)
  xv <- lapply(seq_along(vars), function(i) {
    x <- d[[vars[i]]]
    lab <- attr(x, "label", exact = TRUE)
    lab <- if (is.null(lab)) "" else substr(lab, 1, 40)
    if (is.character(x)) {
      return(list(name = sas[i], label = lab, format = "", fwidth = 0, numeric = FALSE, values = ifelse(is.na(x), "", x), width = max(strwidth(x), 1)))
    }
    fmt <- attr(x, "format.stata", exact = TRUE)
    nm <- attr(x, "vallab", exact = TRUE)
    haslab <- length(attr(x, "labels", exact = TRUE)) > 0
    if (inherits(x, "Date")) {
      x <- as.numeric(x - as.Date("1960-01-01"))
      if (is.null(fmt)) fmt <- "%td"
    }
    if (haslab) {
      f <- toupper(substr(nm, 1, 8))
      fw <- 0
    } else if (!is.null(fmt) && grepl("^%(td|d)", fmt)) {
      f <- "MMDDYY"
      fw <- 10
    } else {
      f <- "BEST"
      fw <- 12
    }
    list(name = sas[i], label = lab, format = f, fwidth = fw, numeric = TRUE, values = as.numeric(unclass(x)), width = 8)
  })
  stem <- toupper(substr(sub("\\.[^.]*$", "", basename(file)), 1, 8))
  now <- Sys.time()
  stamp <- sprintf("%02d%s%s:%s", as.integer(format(now, "%d")), toupper(month.abb[as.integer(format(now, "%m"))]), format(now, "%y"), format(now, "%H:%M:%S"))
  writeBin(stata_xpt_write(stem, stamp, xv), file)
  cat("file ", shown, " saved\n", sep = "")
  entries <- function(nm) {
    for (v in vars) {
      x <- d[[v]]
      if (identical(attr(x, "vallab", exact = TRUE), nm)) {
        labs <- attr(x, "labels", exact = TRUE)
        key <- as.numeric(unclass(labs))
        o <- order(is.na(key), key)
        return(list(key = vapply(o, function(j) keytext(labs[j]), ""), text = names(labs)[o]))
      }
    }
  }
  if (vallabfile %in% c("sascode", "both") && length(used)) {
    win <- .Platform$OS.type == "windows"
    sep <- if (win) "\\" else "/"
    full <- normalizePath(file, winslash = sep, mustWork = FALSE)
    txt <- paste0("\nlibname datapath '", sub("[\\/][^\\/]*$", "", full), sep, "' ;\nlibname xptfile xport '", full, "' ;\n\nproc copy in = xptfile out = datapath ;\n\nproc format library = work ;\n")
    for (nm in used) {
      e <- entries(nm)
      txt <- paste0(txt, "    value ", up(nm, 8), "\n")
      for (j in seq_along(e$key)) txt <- paste0(txt, sprintf("%12s", e$key[j]), " = '", e$text[j], "'", if (j == length(e$key)) " ;" else "", "\n")
      txt <- paste0(txt, "\n")
    }
    txt <- paste0(txt, "quit ;\n")
    writeBin(charToRaw(enc2utf8(txt)), sub("\\.[^.]*$", ".sas", file))
    cat("file ", sub("\\.[^.]*$", ".sas", shown), " saved\n", sep = "")
  }
  if (vallabfile %in% c("xpf", "both") && length(used)) {
    k <- character()
    s <- character()
    t <- character()
    for (nm in used) {
      e <- entries(nm)
      k <- c(k, rep(up(nm, 8), length(e$key)))
      s <- c(s, e$key)
      t <- c(t, e$text)
    }
    col <- function(name, w, vals) list(name = name, label = "", format = "", fwidth = 0, numeric = FALSE, values = vals, width = w)
    writeBin(stata_xpt_write("FORMATS", stamp, list(col("FMTNAME", 8, k), col("START", 12, s), col("LABEL", 40, t))), formats)
    folder <- dirname(shown)
    cat("file ", if (folder == ".") "" else paste0(folder, "/"), "formats.xpf saved\n", sep = "")
  }
  invisible(NULL)
}
