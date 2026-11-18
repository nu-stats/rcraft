stata_fdause <- function(file, shown, novallabels = FALSE) {
  if (!file.exists(file)) stop("file ", shown, " not found (Stata: r(601))", call. = FALSE)
  bytes <- readBin(file, "raw", file.size(file))
  vars <- stata_xpt_read(bytes)
  if (is.null(vars)) stop("file ", shown, " not SAS XPORT format (Stata: r(610))", call. = FALSE)
  folder <- dirname(file)
  found <- NULL
  for (f in c("formats.xpf", "FORMATS.xpf")) if (file.exists(file.path(folder, f))) { found <- file.path(folder, f); break }
  sets <- list()
  if (!is.null(found) && !novallabels) {
    fv <- stata_xpt_read(readBin(found, "raw", file.size(found)))
    col <- function(n) for (v in fv) if (v$name == n && !v$numeric) return(v$values)
    nm <- col("FMTNAME")
    st <- col("START")
    lb <- col("LABEL")
    if (!is.null(nm) && !is.null(st) && !is.null(lb)) {
      for (i in seq_along(nm)) {
        s <- trimws(st[i])
        key <- suppressWarnings(as.numeric(s))
        if (is.na(key) || grepl("[^0-9+-]", s)) {
          if (grepl("^\\.[a-z]$", s)) key <- haven::tagged_na(substr(s, 2, 2)) else next
        }
        k <- tolower(trimws(nm[i]))
        sets[[k]] <- c(sets[[k]], stats::setNames(key, lb[i]))
      }
    }
  }
  number <- c("BEST", "F", "COMMA", "COMMAX", "DOLLAR", "E", "PERCENT", "Z", "W", "D")
  dates <- c("MMDDYY", "DDMMYY", "YYMMDD", "DATE", "WEEKDATE", "WORDDATE", "MONYY", "JULIAN")
  labelled <- FALSE
  out <- list()
  for (v in vars) {
    name <- tolower(v$name)
    sasf <- toupper(trimws(v$format))
    x <- v$values
    if (v$numeric) {
      x <- as.numeric(x)
      if (sasf %in% dates) {
        attr(x, "format.stata") <- "%d"
      } else if (nzchar(sasf) && !startsWith(sasf, "$") && !(sasf %in% number)) {
        lab <- tolower(sasf)
        labs <- sets[[lab]]
        longest <- if (length(labs)) max(nchar(names(labs))) else 0
        x <- if (length(labs)) haven::labelled(x, labels = labs) else x
        attr(x, "vallab") <- lab
        attr(x, "format.stata") <- sprintf("%%%d.0g", max(longest, 10))
        labelled <- TRUE
      }
    } else {
      x <- as.character(x)
      attr(x, "format.stata") <- sprintf("%%%ds", min(max(v$width, 1), 2045))
    }
    if (nzchar(v$label)) attr(x, "label") <- v$label
    out[[name]] <- x
  }
  if (labelled && !novallabels && is.null(found))
    cat("Warning:  file ", shown, " contains variables with value labels, but value label file formats.xpf or FORMATS.xpf could not be opened\n", sep = "")
  as.data.frame(out, stringsAsFactors = FALSE, check.names = FALSE)
}
