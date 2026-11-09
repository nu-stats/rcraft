//! Stata's returned values (P2B.6): `e()` after a model, `r()` after the
//! commands that leave results, `_b[]` and `_se[]`, and `display`,
//! `ereturn list` and `return list`.
//!
//! In R each translated model leaves `stata_e`, and each r-class command
//! `stata_r`: a named list holding Stata's names, numbers as numbers and
//! macros as text, with `b` and `V` as matrices. What each list holds, and
//! when, was read off Stata 19's `ereturn list` and `return list` (see the
//! P2B.6 notes). The translator keeps track of which command left them, so
//! `e(name)` and `r(name)` are refused where Stata would hold a value RCraft
//! does not keep, rather than coming out missing.

use super::{no, Out, Session};
use crate::expr_r::{r_text, Kind, Untranslated, Use};
use crate::lang::expr::{parse_prefix, Expr};
use crate::reader::Command;

/// Reading `stata_e`, `stata_r`, `_b[]` and `_se[]` in R, and the lists.
pub(super) const RETURNED_HELPER: &str = r##"stata_eget <- function(name, text = FALSE) {
  if (isTRUE(get0("stata_fit_failed"))) stop("e() is not known here: R stopped in the model before it", call. = FALSE)
  v <- if (exists("stata_e")) stata_e[[name]] else NULL
  if (is.null(v) || is.matrix(v)) return(if (text) "" else NA_real_)
  v
}
stata_rget <- function(name, text = FALSE) {
  v <- if (exists("stata_r")) stata_r[[name]] else NULL
  if (is.null(v) || is.matrix(v)) return(if (text) "" else NA_real_)
  v
}
stata_bget <- function(key, se = FALSE) {
  if (isTRUE(get0("stata_fit_failed"))) stop("_b[] is not known here: R stopped in the model before it", call. = FALSE)
  st <- stata_bnames()
  nm <- names(stata_b)
  eq <- ""
  if (grepl(":", key, fixed = TRUE) && !startsWith(key, "/")) {
    eq <- sub(":.*$", "", key)
    key <- sub("^[^:]*:", "", key)
  }
  shown <- if (eq == "") key else paste0(eq, ":", key)
  if (inherits(stata_estimates, "multinom")) {
    # the equations are the outcomes in order, the base one all zeros
    if (eq == "") eq <- stata_levels[1]
    if (!eq %in% stata_levels) stop("equation [", eq, "] not found")
    k <- stata_canon(key)
    there <- k %in% sub("^[^:]*:", "", st) || (k == "_cons" && any(endsWith(nm, ":(Intercept)"))) ||
      k %in% stata_estimates[["omitted"]]
    if (eq == as.character(stata_base)) {
      if (!there) stop("[", shown, "] not found")
      return(0)
    }
    key <- paste0(eq, ":", key)
  } else if (eq != "") {
    stop("equation [", eq, "] not found")
  }
  i <- match(stata_canon(key), st)
  if (is.na(i)) {
    # a base or omitted level of a factor variable, or a term the model was
    # fitted again without (collinear): Stata holds a zero
    if (stata_base_level(stata_canon(key))) return(0)
    if (sub("^[^:]*:", "", stata_canon(key)) %in% stata_estimates[["omitted"]]) return(0)
    stop("[", shown, "] not found")
  }
  # a term R left out for collinearity: 0 in Stata's e(b) and e(V)
  if (is.na(stata_b[i])) return(0)
  if (se) sqrt(stata_vcov[nm[i], nm[i]]) else unname(stata_b[i])
}
stata_base_level <- function(k) {
  xl <- stata_estimates$xlevels
  if (is.null(xl)) return(FALSE)
  # a cell of two factors interacted alone (stata_cells): the base cell and
  # an empty one (a level R dropped) have no coefficient; Stata holds a zero
  for (n in names(xl)[startsWith(names(xl), "stata_cells(")]) {
    m <- regmatches(n, regexec("^stata_cells\\(([^,]+), ([^,]+),", n))[[1]]
    ab <- strsplit(xl[[n]], "#", fixed = TRUE)
    l1 <- unique(vapply(ab, function(x) x[1], ""))
    l2 <- unique(vapply(ab, function(x) x[2], ""))
    cells <- vapply(paste0(rep(l1, each = length(l2)), ".", m[2], "#", rep(l2, times = length(l1)), ".", m[3]),
                    stata_canon, "")
    if (stata_canon(k) %in% cells) return(TRUE)
  }
  vars <- sub("^stata_ib\\(([^,]+),.*$", "\\1", sub("^factor\\(([^()]+)\\)$", "\\1", names(xl)))
  parts <- strsplit(k, "#", fixed = TRUE)[[1]]
  all(vapply(parts, function(p) {
    m <- regmatches(p, regexec("^([0-9]+)\\.(.+)$", p))[[1]]
    if (length(m) == 0) return(p %in% vars || p %in% all.vars(stats::formula(stata_estimates)))
    j <- match(m[3], vars)
    !is.na(j) && m[2] %in% xl[[j]]
  }, TRUE)) && any(grepl("^[0-9]+\\.", parts))
}
stata_list_returned <- function(x, p) {
  if (is.null(x)) return(invisible(NULL))
  num <- names(x)[vapply(x, function(v) is.numeric(v) && !is.matrix(v), TRUE)]
  txt <- names(x)[vapply(x, is.character, TRUE)]
  mat <- names(x)[vapply(x, is.matrix, TRUE)]
  key <- function(n) sprintf("%22s", paste0(p, "(", n, ")"))
  if (length(num)) {
    cat("\nscalars:\n")
    for (n in num) cat(key(n), " =  ", stata_fmt(x[[n]], "%18.0g", trim = TRUE), "\n", sep = "")
  }
  if (length(txt)) {
    cat("\nmacros:\n")
    for (n in txt) cat(key(n), " : \"", x[[n]], "\"\n", sep = "")
  }
  if (length(mat)) {
    cat("\nmatrices:\n")
    for (n in mat) cat(key(n), " :  ", nrow(x[[n]]), " x ", ncol(x[[n]]), "\n", sep = "")
  }
  # results ereturn posted without esample() have no e(sample), nor does cluster2's
  if (p == "e" && !identical(x$cmd, "cluster2") && !identical(attr(x, "stata_sample"), FALSE)) cat("\nfunctions:\n", key("sample"), "   \n", sep = "")
  invisible(NULL)
}"##;

/// Stata's display formats, `%w.dg`, `%w.df`, `%w.de` (each with `c` for
/// commas, `-` to justify left, `0` to fill with zeros), `%ws` and `%21x`,
/// decoded in Stata 19 (the P2B.6 notes have the rules), and `display` itself.
/// `%21x` is the double's own bits: `+1.921fb54442d18X+001` is
/// (1 + 0x921fb54442d18 / 2^52) * 2^1, zero `+0.0000000000000X-3ff` and `.`
/// `+1.0000000000000X+3ff` (StatCraft's probes 000, 031 and 034).
pub(super) const FORMAT_HELPER: &str = r##"# Stata's date and time display formats (%td, %tcHH:MM, %tmCCYY!mnn, ...),
# StatCraft's decoding of Stata 19 (its plan/stata/dates.md, probe 033): each
# code's widest text sets the width, shorter text is padded to it, and a
# value outside 01jan0100 to 31dec9999 is shown as a number in %w.0g
stata_tfmt <- function(x, fmt) {
  if (is.character(x)) stop("type mismatch")
  m <- regmatches(fmt, regexec("^%(-?)(t([cCdwmqhy])|d)(.*)$", fmt))[[1]]
  if (length(m) == 0) stop("invalid %format")
  left <- m[2] == "-"
  unit <- if (m[3] == "d") "d" else m[4]
  det <- m[5]
  if (!nzchar(det)) det <- switch(unit, c = , C = "DDmonCCYY HH:MM:SS", d = "DDmonCCYY", w = "CCYY!www", m = "CCYY!mnn",
                                  q = "CCYY!qq", h = "CCYY!hh", y = "CCYY")
  known <- c("DAYNAME", "Dayname", "Month", "month", "a.m.", "A.M.", ".sss", ".ss", ".s", "JJJ", "jjj", "Mon", "mon",
             "Day", "day", "Da", "da", "CC", "cc", "YY", "yy", "NN", "nn", "DD", "dd", "WW", "ww", "HH", "hH", "Hh", "hh",
             "MM", "mm", "SS", "ss", "am", "AM", "h", "q")
  codes <- character(0)
  rest <- det
  while (nzchar(rest)) {
    hit <- known[startsWith(rest, known)][1]
    if (!is.na(hit)) {
      codes <- c(codes, hit)
      rest <- substring(rest, nchar(hit) + 1)
      next
    }
    ch <- substr(rest, 1, 1)
    rest <- substring(rest, 2)
    if (ch == "+") next
    if (ch == "_") { codes <- c(codes, "! "); next }
    if (ch %in% c(".", ",", ":", "-", "/", "\\", " ")) { codes <- c(codes, paste0("!", ch)); next }
    if (ch == "!" && nzchar(rest)) { codes <- c(codes, paste0("!", substr(rest, 1, 1))); rest <- substring(rest, 2); next }
    stop("invalid %format")
  }
  wid <- function(cd) {
    if (startsWith(cd, "!") || cd %in% c("h", "q")) return(1L)
    if (cd %in% c("Month", "month", "DAYNAME", "Dayname")) return(9L)
    if (cd %in% c("JJJ", "jjj", "Mon", "mon", "Day", "day", "Da", "da")) return(3L)
    if (cd %in% c(".s", ".ss", ".sss")) return(nchar(cd))
    if (cd %in% c("a.m.", "A.M.")) return(4L)
    2L
  }
  width <- sum(vapply(codes, wid, 1L))
  pad <- function(s) {
    gap <- strrep(" ", max(0, width - nchar(s)))
    if (left) paste0(s, gap) else paste0(gap, s)
  }
  months <- c("January", "February", "March", "April", "May", "June", "July", "August", "September", "October",
              "November", "December")
  days <- c("Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday")
  dnum <- function(y, mo, d) as.numeric(as.Date(sprintf("%04d-%02d-%02d", y, mo, d)) - as.Date("1960-01-01"))
  one <- function(v) {
    if (is.na(v)) return(pad("."))
    ms <- 0
    day <- NA_real_
    inleap <- 0
    if (is.finite(v)) {
      f <- floor(v)
      # datetime/C: the clock's time, 23:59:60 inside a leap second
      if (unit == "C") { w <- stata_dt_wall(v); inleap <- as.numeric(w$leap); v <- w$wall }
      if (unit %in% c("c", "C")) { day <- floor(v / 86400000); ms <- floor(v - day * 86400000) }
      else if (unit == "d") day <- f
      else if (unit == "y") { if (f >= 100 && f <= 9999) day <- dnum(f, 1, 1) }
      else {
        per <- switch(unit, w = 52, m = 12, q = 4, h = 2)
        y <- 1960 + floor(f / per); k <- f - floor(f / per) * per
        if (y >= 100 && y <= 9999) day <- if (unit == "w") dnum(y, 1, 1) + 7 * k else dnum(y, k * 12 / per + 1, 1)
      }
    }
    if (is.na(day) || day < -679350 || day > 2936549) {
      s <- trimws(stata_fmt(v, sprintf("%%%d.0g", min(max(width, 1), 12))), "left")
      return(pad(s))
    }
    lt <- as.POSIXlt(as.Date(day, origin = "1960-01-01"))
    y <- lt$year + 1900; mo <- lt$mon + 1; d <- lt$mday
    doy <- day - dnum(y, 1, 1) + 1
    dow <- (day + 5) %% 7
    hour <- ms %/% 3600000; minute <- ms %/% 60000 %% 60; second <- ms %/% 1000 %% 60 + inleap; milli <- ms %% 1000
    h12 <- if (hour %% 12 == 0) 12 else hour %% 12
    num <- function(v, p, n) if (p) formatC(v, width = n, flag = "0", format = "d") else as.character(v)
    pm <- hour >= 12
    show <- function(cd) {
      if (startsWith(cd, "!")) return(substring(cd, 2))
      switch(cd,
        CC = num(y %/% 100, TRUE, 2), cc = num(y %/% 100, FALSE, 2),
        YY = num(y %% 100, TRUE, 2), yy = num(y %% 100, FALSE, 2),
        JJJ = num(doy, TRUE, 3), jjj = num(doy, FALSE, 3),
        Month = months[mo], month = tolower(months[mo]),
        Mon = substr(months[mo], 1, 3), mon = tolower(substr(months[mo], 1, 3)),
        NN = num(mo, TRUE, 2), nn = num(mo, FALSE, 2), DD = num(d, TRUE, 2), dd = num(d, FALSE, 2),
        DAYNAME = formatC(days[dow + 1], width = -9), Dayname = days[dow + 1],
        Day = substr(days[dow + 1], 1, 3), Da = substr(days[dow + 1], 1, 2),
        day = tolower(substr(days[dow + 1], 1, 3)), da = tolower(substr(days[dow + 1], 1, 2)),
        h = as.character((mo - 1) %/% 6 + 1), q = as.character((mo - 1) %/% 3 + 1),
        WW = num(min((doy - 1) %/% 7 + 1, 52), TRUE, 2), ww = num(min((doy - 1) %/% 7 + 1, 52), FALSE, 2),
        HH = num(hour, TRUE, 2), hH = num(hour, FALSE, 2), Hh = num(h12, TRUE, 2), hh = num(h12, FALSE, 2),
        MM = num(minute, TRUE, 2), mm = num(minute, FALSE, 2), SS = num(second, TRUE, 2), ss = num(second, FALSE, 2),
        .s = paste0(".", substr(formatC(milli, width = 3, flag = "0", format = "d"), 1, 1)),
        .ss = paste0(".", substr(formatC(milli, width = 3, flag = "0", format = "d"), 1, 2)),
        .sss = paste0(".", formatC(milli, width = 3, flag = "0", format = "d")),
        am = if (pm) "pm" else "am", AM = if (pm) "PM" else "AM",
        a.m. = if (pm) "p.m." else "a.m.", A.M. = if (pm) "P.M." else "A.M.")
    }
    pad(paste(vapply(codes, show, ""), collapse = ""))
  }
  vapply(as.numeric(x), one, "")
}
stata_fmt <- function(x, fmt = "%10.0g", trim = FALSE) {
  if (fmt %in% c("%21x", "%-21x")) {
    if (is.character(x)) stop("type mismatch")
    return(stata_hex(x))
  }
  if (grepl("^%-?(t|d)", fmt)) {
    s <- stata_tfmt(x, fmt)
    return(if (trim) trimws(s) else s)
  }
  m <- regmatches(fmt, regexec("^%(-?)(0?)([0-9]+)(\\.([0-9]+))?([gfes])(c?)$", fmt))[[1]]
  if (length(m) == 0) stop("invalid %format")
  left <- m[2] == "-"
  zero <- m[3] == "0"
  w <- as.integer(m[4])
  d <- if (m[6] == "") 0L else as.integer(m[6])
  type <- m[7]
  comma <- m[8] == "c"
  if (is.character(x) != (type == "s")) stop("type mismatch")
  commas <- function(s) {
    int <- sub("\\..*$", "", s)
    rest <- substring(s, nchar(int) + 1)
    while (grepl("[0-9]{4}", int)) int <- sub("([0-9])([0-9]{3})(,|$)", "\\1,\\2\\3", int)
    paste0(int, rest)
  }
  # g and e round an exact half away from zero (f, like C, to even). Whether
  # a value is an exact half is worked out from the double itself, ax being
  # m * 2^k with m whole, not from printed digits: a 17-digit form ending in
  # 5 is most often no half (.14761666666666665 is not), and R's printf past
  # 17 digits is not exact on every platform
  rnd <- function(kind, digits, ax) {
    out <- sprintf(paste0("%.", digits, kind), ax)
    if (ax == 0 || !is.finite(ax) || ax < 2^-1000) return(out)
    k <- floor(log2(ax)) - 52
    while (ax / 2^k >= 2^53) k <- k + 1
    while (ax / 2^k < 2^52) k <- k - 1
    m <- ax / 2^k
    # rounded at 10^-q: a half when ax * 2 * 10^q is whole and odd
    q <- if (kind == "e") digits - as.integer(sub("^.*e", "", sprintf("%.16e", ax))) else digits
    half <- if (q >= 0) {
      # ax * 2 * 10^q is m * 5^q * 2^(k + 1 + q): whole and odd when m holds
      # exactly -(k + 1 + q) factors of 2
      t <- -(k + 1 + q)
      v <- 0
      while (t >= 0 && v <= t && m %% 2 == 0) {
        m <- m / 2
        v <- v + 1
      }
      t >= 0 && v == t
    } else {
      p <- -q
      p <= 22 && ax == floor(ax) && (2 * ax) %% 10^p == 0 && ((2 * ax) / 10^p) %% 2 == 1
    }
    # the next double up is past the half, by as little as can be
    if (half) out <- sprintf(paste0("%.", digits, kind), ax + 2^k)
    out
  }
  e_str <- function(ax, digits) rnd("e", max(digits, 1) - 1, ax)
  exp_width <- function(ax) if (ax != 0 && abs(floor(log10(ax))) >= 100) 3 else 2
  # the sign's place: a minus, or a space before a positive number in e
  # notation even past the width, as Stata prints it
  sgn <- if (!is.character(x) && !is.na(x) && x < 0) "-" else " "
  s <- if (type == "s") {
    x
  } else if (is.na(x)) {
    "."
  } else if (x == 0 && type == "g") {
    "0"
  } else if (type == "g") {
    ax <- abs(x)
    budget <- w - 1
    ed <- max(2, w - 4 - exp_width(ax))
    if (d > 0) ed <- min(max(d, 2), ed)
    fixed <- function(ax) {
      intdig <- if (ax >= 1) nchar(sprintf("%.0f", floor(ax))) else 0
      ints <- intdig + if (comma && intdig > 3) (intdig - 1) %/% 3 else 0
      if (ints + 1 > budget) return(NULL)
      dec <- budget - ints - 1
      z <- if (ax < 1) -floor(log10(ax)) - 1 else 0
      if (d > 0) dec <- min(dec, if (ax >= 1) max(0, max(d, 2) - intdig) else z + max(d, 2))
      sig <- if (ax >= 1) intdig + dec else dec - z
      if (sig < ed) return(NULL)
      s <- rnd("f", dec, ax)
      # rounding up may have added a digit in front
      if (nchar(sub("\\..*$", "", sub("^0", "", s))) > intdig) {
        return(if (ax < 1) fixed(1) else fixed(10^intdig))
      }
      if (grepl(".", s, fixed = TRUE)) s <- sub("\\.$", "", sub("0+$", "", s))
      s <- sub("^0\\.", ".", s)
      if (comma) s <- commas(s)
      s
    }
    f <- fixed(ax)
    if (is.null(f)) paste0(sgn, e_str(ax, ed)) else if (x < 0) paste0("-", f) else f
  } else if (type == "f") {
    s <- sprintf("%.*f", d, x)
    if (comma) s <- commas(s)
    if (nchar(s) > w) {
      # too wide: e notation, unless that is wider still
      ax <- abs(x)
      e <- e_str(ax, max(2, w - 4 - exp_width(ax)))
      if (x < 0) e <- paste0("-", e)
      if (nchar(e) <= w || nchar(e) < nchar(s)) s <- if (x < 0) e else paste0(" ", e)
    }
    s
  } else {
    ax <- abs(x)
    dd <- d
    s <- rnd("e", dd, ax)
    while (nchar(s) > w - 1 && dd > 1) {
      dd <- dd - 1
      s <- rnd("e", dd, ax)
    }
    paste0(sgn, s)
  }
  if (trim) return(trimws(s))
  if (left) s <- sub("^ ", "", s)
  if (nchar(s) >= w) return(s)
  pad <- strrep(if (zero && !left && type != "s") "0" else " ", w - nchar(s))
  if (left) paste0(s, pad) else if (zero && grepl("^-", s)) paste0("-", pad, substring(s, 2)) else paste0(pad, s)
}
stata_display <- function(...) {
  out <- ""
  col <- function() nchar(sub("^.*\n", "", out))
  for (p in list(...)) {
    out <- switch(p[[1]],
      text = paste0(out, p[[2]]),
      value = paste0(out, if (p[[3]] != "") stata_fmt(p[[2]], p[[3]]) else if (is.character(p[[2]])) p[[2]] else stata_fmt(p[[2]], "%10.0g", trim = TRUE)),
      newline = paste0(out, strrep("\n", p[[2]])),
      col = paste0(out, strrep(" ", max(0, p[[2]] - 1 - col()))),
      skip = paste0(out, strrep(" ", p[[2]])),
      dup = paste0(out, strrep(p[[3]], p[[2]])))
  }
  cat(out, "\n", sep = "")
  invisible(NULL)
}
stata_hex <- function(x) {
  if (is.na(x)) return("+1.0000000000000X+3ff")
  s <- if (x < 0 || (x == 0 && 1 / x < 0)) "-" else "+"
  ax <- abs(x)
  if (ax == 0) return(paste0(s, "0.0000000000000X-3ff"))
  if (ax < 2^-1022) {
    # subnormal: the bits are ax / 2^-1074, after a leading 0
    m <- ax * 2^1022 * 2^52
    lead <- "0"
    e <- -1023
  } else {
    e <- floor(log2(ax))
    while (ax / 2^e >= 2) e <- e + 1
    while (ax / 2^e < 1) e <- e - 1
    m <- (ax / 2^e - 1) * 2^52
    lead <- "1"
  }
  h <- character(13)
  for (i in 13:1) {
    r <- m %% 16
    h[i] <- c(0:9, letters[1:6])[r + 1]
    m <- (m - r) / 16
  }
  paste0(s, lead, ".", paste(h, collapse = ""), "X", if (e < 0) "-" else "+", sprintf("%03x", as.integer(abs(e))))
}"##;

/// What a model leaves in `e()`, built from what its translation left in R
/// (`stata_estimates`, `stata_b`, `stata_vcov`, `stata_df`, `stata_cluster`).
pub(super) const ERETURN_HELPER: &str = r##"stata_ereturn <- function(cmd, cmdline, depvar, vce = "oim", clustvar = NULL,
                          wtype = NULL, wexp = NULL, title = TRUE, extra = list()) {
  fit <- stata_estimates
  b <- stata_b
  V <- stata_vcov
  robust <- vce %in% c("robust", "cluster")
  G <- if (vce != "cluster") NULL else if (!is.null(fit[["G"]])) fit[["G"]] else if (cmd == "ivregress") length(unique(fit$cluster)) else if (cmd %in% c("stcox", "clogit", "asclogit")) stata_m$G else if (cmd == "reghdfe") min(fit$stata$G) else length(unique(stata_cluster))
  wald <- function(keep) {
    # no slopes (a constant alone): Stata's F(0, df) is 0
    if (!any(keep)) return(0)
    # on the scale of the errors (a correlation), so coefficients of very
    # different sizes do not make V look singular to solve()
    bk <- b[keep]
    # by name: a robust V (sandwich) leaves out the rows of an NA coefficient
    Vm <- as.matrix(V)
    Vk <- if (!is.null(rownames(Vm)) && all(names(bk) %in% rownames(Vm))) {
      Vm[names(bk), names(bk), drop = FALSE]
    } else {
      Vm[keep, keep, drop = FALSE]
    }
    s <- sqrt(diag(Vk))
    z <- bk / s
    # a variance of less than full rank (fewer clusters than slopes): Stata's test is missing
    C <- Vk / outer(s, s)
    if (any(!is.finite(C)) || qr(C)$rank < length(z)) return(NA_real_)
    drop(t(z) %*% solve(C, z))
  }
  # a term omitted for collinearity is no slope: Stata counts only the estimated ones
  slopes <- !grepl("(Intercept)", names(b), fixed = TRUE) & !startsWith(names(b), "/") & !is.na(b)
  cons <- any(grepl("(Intercept)", names(b), fixed = TRUE))
  fw <- identical(wtype, "fweight")
  e <- list()
  # matrices Stata lists after e(b) and e(V)
  after <- list()
  put <- function(...) {
    a <- list(...)
    for (n in names(a)) e[[n]] <<- a[[n]]
  }
  weights <- function() if (!is.null(wtype)) put(wexp = wexp, wtype = wtype)
  robust_macros <- function() {
    if (robust) put(vcetype = "Robust")
    if (vce == "cluster") put(clustvar = clustvar)
  }
  # the likelihood models: the count, the log likelihoods and the model test
  ml <- function(k_eq, k_eq_model = 1, ll_0 = NULL, aux = list()) {
    mf <- stats::model.frame(fit)
    w <- stats::model.weights(mf)
    N <- if (fw && !is.null(w)) sum(w) else NROW(mf)
    l <- stata_ml_ll(fit)
    if (!is.null(ll_0)) l[["ll_0"]] <- ll_0
    q <- sum(slopes)
    chi2 <- if (robust) wald(slopes) else 2 * (l[["ll"]] - l[["ll_0"]])
    put(rank = sum(!is.na(b)), N = N, k = length(b), k_eq = k_eq, k_dv = 1, converged = 1, rc = 0)
    if (!is.null(G)) put(N_clust = G)
    put(ll = l[["ll"]], k_eq_model = k_eq_model, ll_0 = l[["ll_0"]])
    for (n in names(aux)) e[[n]] <<- aux[[n]]
    put(df_m = q, chi2 = chi2, p = stats::pchisq(chi2, q, lower.tail = FALSE))
    list(ll = l[["ll"]], ll_0 = l[["ll_0"]])
  }
  r2_p <- function(l) put(r2_p = 1 - l$ll / l$ll_0)
  if (cmd %in% c("regress", "anova")) {
    mf <- stats::model.frame(fit)
    y <- stats::model.response(mf)
    w <- stats::model.weights(mf)
    n <- length(y)
    iw <- identical(wtype, "iweight")
    wn <- if (is.null(w)) rep(1, n) else if (fw || iw) w else w * n / sum(w)
    # importance weights: N is their sum rounded down, the root MSE divides by
    # the sum itself (both read off Stata 19)
    N <- if (fw) sum(w) else if (iw) floor(sum(w)) else n
    Wk <- if (iw) sum(w) else N
    u <- stats::residuals(fit)
    ybar <- sum(wn * y) / sum(wn)
    rss <- sum(wn * u^2)
    tss <- if (cons) sum(wn * (y - ybar)^2) else sum(wn * y^2)
    mss <- tss - rss
    # Stata 19 takes a fit whose residual sum of squares is below about 1e-14
    # of the total (centred) one as exact: rss 0, R-squared 1, no F or log
    # likelihood, and standard errors of 0 unless they are robust (observed
    # unweighted: zero below 9.87e-15, kept from 9.98e-15, whatever N and k)
    exact <- FALSE
    if (cmd == "regress" && tss > 0 && rss < 9.98e-15 * tss) {
      if (!is.null(w) || rss >= 9.87e-15 * tss)
        stop("regress: the residuals are so small that Stata 19 may take the fit as exact, by a rule not pinned down here; not translated yet", call. = FALSE)
      exact <- TRUE
      rss <- 0
      mss <- tss
      if (!robust) {
        V[] <- 0
        stata_vcov[] <<- 0
      }
    }
    k <- fit$rank
    df_m <- k - cons
    r2 <- mss / tss
    # Stata's log likelihood of the normal model; analytic weights add half
    # the sum of the log of the weights as Stata scales them (to sum to N)
    ll <- function(ss) {
      l <- -0.5 * N * (log(2 * pi) + log(ss / N) + 1)
      if (identical(wtype, "aweight")) l + 0.5 * sum(log(wn)) else l
    }
    if (cmd == "anova") {
      put(N = N, sum_w = if (is.null(w)) N else sum(w))
    } else {
      if (!is.null(G)) put(N_clust = G)
      put(N = N, sum_w = if (is.null(w)) N else sum(w))
    }
    put(df_m = df_m, df_r = stata_df, F = wald(slopes) / df_m, r2 = r2, rmse = sqrt(rss / (Wk - k)),
        mss = mss, rss = rss, r2_a = 1 - rss / (Wk - k) / (tss / (N - cons)), ll = if (exact) NA_real_ else ll(rss))
    # without the constant Stata keeps no e(ll_0)
    if (cons) put(ll_0 = ll(tss))
    put(rank = k)
    if (cmd == "anova") {
      for (n in names(extra)) e[[n]] <- extra[[n]]
      put(cmdline = cmdline, depvar = depvar, cmd = "anova", estat_cmd = "anova_estat", properties = "b V",
          predict = "regres_p", model = "ols")
    } else {
      put(cmdline = cmdline)
      if (title) put(title = "Linear regression")
      put(marginsok = "XB default", vce = vce, depvar = depvar, cmd = "regress", properties = "b V",
          predict = "regres_p", model = "ols", estat_cmd = "regress_estat")
      weights()
      robust_macros()
    }
  } else if (cmd %in% c("logit", "probit")) {
    l <- ml(1)
    r2_p(l)
    put(cmdline = cmdline, cmd = cmd, estat_cmd = paste0(cmd, "_estat"), predict = paste0(cmd, "_p"),
        marginsok = "default Pr",
        marginsnotok = if (cmd == "logit") "stdp DBeta DEviance DX2 DDeviance Hat Number Residuals RStandard SCore" else "stdp DEviance SCore",
        title = if (cmd == "logit") "Logistic regression" else "Probit regression",
        chi2type = if (robust) "Wald" else "LR")
    weights()
    robust_macros()
    put(vce = vce, depvar = depvar, properties = "b V")
  } else if (cmd == "poisson") {
    l <- ml(1)
    r2_p(l)
    put(cmdline = cmdline, cmd = "poisson", predict = "poisso_p", estat_cmd = "poisson_estat")
    if (!is.null(extra$offset)) put(offset = extra$offset)
    weights()
    put(chi2type = if (robust) "Wald" else "LR")
    robust_macros()
    put(vce = vce, title = "Poisson regression", depvar = depvar, properties = "b V")
  } else if (cmd == "nbreg") {
    mf <- stats::model.frame(fit)
    off <- stats::model.offset(mf)
    w <- stats::model.weights(mf)
    d0 <- data.frame(y = stats::model.response(mf), o = if (is.null(off)) 0 else off,
                     w = if (is.null(w)) 1 else w)
    f0 <- suppressWarnings(MASS::glm.nb(y ~ 1 + offset(o), data = d0, weights = w,
                                        control = stats::glm.control(epsilon = 1e-12, maxit = 100)))
    pois <- stats::glm(fit$y ~ stats::model.matrix(fit) - 1, family = stats::poisson(), offset = fit$offset,
                       weights = fit$prior.weights, control = stats::glm.control(epsilon = 1e-14, maxit = 100))
    l <- ml(2, ll_0 = as.numeric(stats::logLik(f0)), aux = list(rank0 = 2))
    put(ll_c = as.numeric(stats::logLik(pois)), alpha = exp(b[["/lnalpha"]]), k_aux = 1)
    r2_p(l)
    put(cmdline = cmdline, cmd = "nbreg", predict = "nbreg_p")
    if (!is.null(extra$offset)) put(offset = extra$offset)
    put(dispers = "mean")
    weights()
    put(chi2type = if (robust) "Wald" else "LR")
    robust_macros()
    put(vce = vce, title = "Negative binomial regression", depvar = depvar, properties = "b V")
  } else if (cmd %in% c("ologit", "oprobit")) {
    ncut <- length(fit$zeta)
    l <- ml(1 + ncut)
    put(k_cat = ncut + 1, k_aux = ncut)
    r2_p(l)
    put(cmdline = cmdline, cmd = cmd, predict = if (cmd == "ologit") "ologit_p" else "oprobi_p",
        marginsdefault = paste0("predict(pr outcome(", fit$lev, "))", collapse = " "),
        title = if (cmd == "ologit") "Ordered logistic regression" else "Ordered probit regression")
    weights()
    put(chi2type = if (robust) "Wald" else "LR")
    robust_macros()
    put(vce = vce, depvar = depvar, properties = "b V")
  } else if (cmd == "mlogit") {
    lev <- stata_levels
    ib <- match(as.character(stata_base), lev)
    l <- ml(length(lev), k_eq_model = length(lev))
    e$k_eq <- NULL
    e$k_eq_model <- NULL
    put(ibaseout = ib, baseout = as.numeric(stata_base), k_eq = length(lev), k_eq_model = length(lev),
        k_eq_base = ib)
    r2_p(l)
    put(k_out = length(lev))
    put(cmdline = cmdline, cmd = "mlogit",
        marginsdefault = paste0("predict(pr outcome(", lev, "))", collapse = " "),
        marginsnotok = "stdp stddp SCores", predict = "mlogit_p", title = "Multinomial logistic regression",
        eqnames = paste(lev, collapse = " "))
    weights()
    put(chi2type = if (robust) "Wald" else "LR")
    robust_macros()
    put(vce = vce, depvar = depvar, properties = "b V")
  } else if (cmd == "xtreg" && fit$type == "mle") {
    k <- sum(slopes)
    Ti <- fit$Ti
    put(ll_0 = fit$ll_0, ll = fit$ll, df_m = k, chi2 = fit$chi2, N_g = fit$n, g_min = min(Ti),
        g_avg = fit$N / fit$n, g_max = max(Ti), N = fit$N, rank = length(b), ll_c = fit$ll_c,
        chi2_c = fit$chi2_c, sigma_u = fit$sigma_u, sigma_e = fit$sigma_e, rho = fit$rho,
        # Stata's e(p) here is the test of sigma_u = 0, on 1 degree of freedom, not halved
        p = stats::pchisq(fit$chi2_c, 1, lower.tail = FALSE))
    put(cmdline = cmdline, estat_cmd = "xtreg_estat", depvar = depvar, cmd = "xtreg", ivar = fit$panel,
        marginsnotok = "E U UE SCore STDP XBU", predict = "xtrefe_p", vce = "oim", chi2_ct = "LR",
        properties = "b V", model = "ml", title = "Random-effects ML regression", distrib = "Gaussian",
        chi2type = "LR")
  } else if (cmd == "xtreg" && fit$type == "pa") {
    k <- sum(slopes)
    Ti <- fit$Ti
    chi2 <- wald(slopes)
    put(df_m = k, chi2 = chi2, p = stats::pchisq(chi2, k, lower.tail = FALSE), df_pear = fit$N, N = fit$N,
        N_g = fit$n, g_max = max(Ti), g_min = min(Ti), g_avg = fit$N / fit$n, tol = 1e-6, dif = fit$dif,
        phi = fit$phi, deviance = fit$phi * fit$N, dispers = fit$phi, chi2_dev = fit$phi * fit$N,
        chi2_dis = fit$phi, rc = 0, rank = length(b))
    put(estat_cmd = "xtreg_estat", cmdline = cmdline, model = "pa", cmd2 = "xtreg", predict = "xtreg_pa_p",
        depvar = depvar, cmd = "xtgee", marginsnotok = "SCore STDP", link = "identity", corr = fit$corr,
        family = "Gaussian", ivar = fit$panel, scale = "x2", vce = if (robust) "robust" else "conventional",
        chi2type = "Wald", properties = "b V")
    if (robust) put(vcetype = "Robust")
    # the working correlation, over the longest panel
    Tm <- max(Ti)
    R <- matrix(fit$alpha, Tm, Tm, dimnames = list(paste0("r", seq_len(Tm)), paste0("c", seq_len(Tm))))
    diag(R) <- 1
    after <- list(R = R)
  } else if (cmd == "xtreg") {
    k <- sum(slopes)
    Ti <- fit$Ti
    if (fit$type == "fe" && !robust) {
      put(F_f = fit$F_u, p_f = stats::pf(fit$F_u, fit$df_f[1], fit$df_f[2], lower.tail = FALSE))
    }
    put(N = fit$N, N_g = fit$n, g_min = min(Ti), g_avg = fit$N / fit$n, g_max = max(Ti),
        r2_w = fit$r2[["within"]], r2_b = fit$r2[["between"]], r2_o = fit$r2[["overall"]])
    if (fit$type != "be") put(sigma_u = fit$sigma_u, sigma_e = fit$sigma_e, rho = fit$rho)
    if (fit$type == "fe") put(corr = fit$corr, ll = fit$ll, ll_0 = fit$ll_0)
    if (is.finite(stata_df)) {
      F <- wald(slopes) / k
      put(df_r = stata_df, F = F, p = stats::pf(F, k, stata_df, lower.tail = FALSE))
    } else {
      chi2 <- wald(slopes)
      put(chi2 = chi2, p = stats::pchisq(chi2, k, lower.tail = FALSE))
    }
    if (!is.null(G)) put(N_clust = G)
    put(cmdline = cmdline, estat_cmd = "xtreg_estat", cmd = "xtreg")
    if (fit$type == "be") put(title = "Between regression (regression on group means)")
    put(marginsnotok = if (fit$type == "fe") "E U UE SCore STDP XBU XBD D" else "E U UE SCore STDP XBU",
        predict = if (fit$type == "re") "xtrere_p" else "xtrefe_p", model = fit$type,
        vce = if (vce == "oim") "conventional" else vce, ivar = fit$panel)
    if (fit$type == "fe") put(absvar = fit$panel)
    put(depvar = depvar)
    if (fit$type == "re") put(chi2type = "Wald")
    put(properties = "b V")
    robust_macros()
  } else if (cmd == "ivregress") {
    q <- sum(slopes)
    small <- is.finite(stata_df)
    tss <- fit$rss / (1 - fit$r2)
    put(N = fit$N, kappa = fit$kappa, mss = tss - fit$rss, rss = fit$rss, rmse = fit$rmse, rank = length(b))
    if (!is.null(G)) put(N_clust = G)
    put(df_r = if (small) stata_df else NA_real_, df_m = q, r2 = fit$r2, r2_a = fit$r2_a)
    if (small) put(F = fit$Wald / q) else put(chi2 = fit$Wald)
    put(k_endog = ncol(fit$X1))
    exog <- colnames(fit$X2)[colnames(fit$X2) != "(Intercept)"]
    put(cmdline = cmdline, cmd = "ivregress", estat_cmd = "ivregress_estat",
        vce = if (vce == "oim") "unadjusted" else vce, estimator = fit$estimator,
        exogr = paste(exog, collapse = " "), exog = paste(c(exog, colnames(fit$Zi)), collapse = " "),
        endog = paste(colnames(fit$X1), collapse = " "))
    if (small) put(small = "small")
    put(footnote = "ivreg_footnote",
        marginsnotok = if (fit$estimator == "2sls") "Residuals SCores XBD D" else "Residuals SCores",
        marginsok = "XB default", predict = "ivregress_p",
        title = paste0("Instrumental-variables ", toupper(fit$estimator), " regression"),
        properties = "b V", depvar = depvar)
    if (fit$estimator == "gmm") put(J = fit$J)
    robust_macros()
  } else if (cmd == "stcox") {
    m <- stata_m
    chi2 <- if (robust) wald(rep(TRUE, length(b))) else 2 * (m$ll - m$ll_0)
    put(N = m$N, ll_0 = m$ll_0, ll = m$ll, chi2 = chi2, r2_p = 1 - m$ll / m$ll_0, converged = 1,
        rank = length(b), df_m = length(b), N_sub = m$N, N_fail = m$N_fail, risk = m$risk)
    if (!is.null(G)) put(N_clust = G)
    put(cmdline = cmdline, cmd2 = "stcox", vce = vce, chi2type = if (robust) "Wald" else "LR",
        method = extra$ties, ties = extra$ties, t0 = "_t0", depvar = "_t", cmd = "cox", properties = "b V")
    robust_macros()
  } else if (cmd == "reghdfe") {
    e0 <- fit$stata
    # noabsorb absorbs the constant, as one group named _cons
    ab <- if (length(e0$absorb)) e0$absorb else "_cons"
    put(N = e0$N, rank = e0$df_m, df_r = e0$df_r, df_m = e0$df_m, tss = e0$tss, tss_within = e0$tss_within,
        rss = e0$rss, mss = e0$mss, rmse = e0$rmse, F = e0$F, ll = e0$ll, ll_0 = e0$ll_0, r2 = e0$r2,
        r2_within = e0$r2_within, r2_a = e0$r2_a, r2_a_within = e0$r2_a_within, sumweights = e0$N,
        converged = 1, report_constant = 1, N_full = e0$N_full, num_singletons = e0$num_singletons,
        drop_singletons = as.numeric(e0$num_singletons >= 0), df_a_nested = e0$df_a_nested,
        df_a_redundant = e0$df_a_redundant, df_a_initial = e0$df_a_initial, df_a = e0$df_a,
        N_hdfe = length(ab), N_hdfe_extended = length(ab))
    if (length(e0$cluster)) {
      put(N_clust = min(e0$G))
      if (length(e0$cluster) == 2) put(N_clust1 = e0$G[1], N_clust2 = e0$G[2], N_clustervars = 2)
    }
    put(extended_absvars = paste(ab, collapse = " "), absvars = paste(ab, collapse = " "),
        title2 = sprintf("Absorbing %d HDFE group%s", length(ab), if (length(ab) == 1) "" else "s"),
        dofmethod = "pairwise clusters continuous", vce = if (vce == "unadjusted") "ols" else vce,
        title = "HDFE Linear regression",
        indepvars = paste(c(setdiff(names(b), "(Intercept)"), "_cons"), collapse = " "), depvar = depvar,
        predict = "reghdfe_p", model = "ols", cmdline = cmdline, cmd = "reghdfe", properties = "b V")
    if (vce != "unadjusted") put(vcetype = "Robust")
    if (length(e0$cluster)) {
      put(clustvar = paste(e0$cluster, collapse = " "), title3 = "Statistics robust to heteroskedasticity")
      if (length(e0$cluster) == 2) put(clustvar1 = e0$cluster[1], clustvar2 = e0$cluster[2])
    }
  } else if (cmd == "cluster2") {
    e0 <- fit$stata
    put(N = e0$N, df_m = e0$df_m, df_r = e0$df_r, F = e0$F, r2 = e0$r2, rmse = e0$rmse, mss = e0$mss,
        rss = e0$rss, r2_a = e0$r2_a, ll = e0$ll, ll_0 = e0$ll_0)
    put(cmd = "cluster2", depvar = depvar, method = "2-dimension clustered SEs",
        title = "Linear regression with clustered SEs", properties = "b V")
  } else if (cmd == "robreg") {
    e0 <- fit$stata
    put(N = e0$N, k_omit = 0, rank = length(b), chi2 = e0$chi2,
        p = stats::pchisq(e0$chi2, e0$df_m, lower.tail = FALSE), k_eq = 1, k_eform = 1, scale = e0$scale,
        df_m = e0$df_m, df_r = e0$df_r, k = e0$k, efficiency = e0$efficiency, converged = 1,
        scale0 = e0$scale0, r2_w = e0$r2_w, r2_p = e0$r2_p)
    put(cmdline = cmdline, vcetype = "Robust", vce = "robust",
        indepvars = paste(setdiff(names(b), "(Intercept)"), collapse = " "), depvar = depvar,
        predict = "robreg predict", subcmd = "m", cmd = "robreg", obf = "huber", title = e0$title,
        properties = "b V")
  } else if (cmd == "asclogit") {
    m <- stata_m
    a <- stata_a
    q <- sum(slopes)
    put(rank = q, N = m$N, k = length(b), k_eq = 1, converged = 1, rc = 0, ll = m$ll)
    if (!is.null(G)) put(N_clust = G)
    put(k_eq_model = 1, df_m = q, chi2 = wald(slopes))
    put(p = stats::pchisq(e$chi2, q, lower.tail = FALSE), k_indvars = q, i_base = 1, k_casevars = 0,
        alt_avg = a$alt_avg, alt_max = a$alt_max, alt_min = a$alt_min, k_alt = length(a$alts),
        N_ic = a$N_case, N_case = a$N_case, const = 0)
    put(cmdline = cmdline, cmd = "asclogit", title = "Alternative-specific conditional logit",
        predict = "asclogit_p", estat_cmd = "asclogit_estat", marginsok = "default Pr XB",
        marginsnotok = "stdp SCores", marktype = "casewise",
        alteqs = paste(a$alts, collapse = " "))
    for (i in seq_along(a$alts)) e[[paste0("alt", i)]] <- as.character(a$alts[i])
    put(altvar = extra$altvar, key_N_ic = "cases", case = extra$case, depvar = depvar,
        indvars = extra$indvars, chi2type = "Wald")
    if (vce == "cluster") put(clustvar = clustvar)
    put(vce = vce)
    if (robust) put(vcetype = "Robust")
    put(properties = "b V")
  } else if (cmd == "clogit" && isTRUE(extra$svy)) {
    # svy: clogit (Stata 19): the design in e(), the adjusted Wald F on the
    # design df, the rank the estimated columns
    m <- stata_m
    q <- sum(slopes)
    W <- wald(slopes)
    dfr <- m$df_r
    Fa <- (dfr - q + 1) / (q * dfr) * W
    put(k = length(b), k_dv = 1, converged = 1, rc = 0, k_eq_model = 1, N_drop = m$N_drop,
        N_group_drop = m$N_group_drop, stages = 1, N = m$N, N_pop = m$N_pop, N_strata = 1,
        N_psu = m$df_r + 1, df_r = dfr, k_eq = 1, rank = q,
        p = stats::pf(Fa, q, dfr - q + 1, lower.tail = FALSE), F = Fa, df_m = q, census = 0,
        singleton = 0, N_strata_omit = 0)
    put(cmd = "clogit", cmdline = extra$svy_cmdline, prefix = "svy", cmdname = "clogit",
        command = extra$command, estat_cmd = "svy_estat", vce = "linearized", vcetype = "Linearized",
        title = "Survey: Conditional (fixed-effects) logistic regression",
        wexp = paste("=", extra$wvar), wtype = "pweight", wvar = extra$wvar, singleunit = "missing")
    if (nzchar(extra$psu)) put(su1 = extra$psu)
    put(properties = "b V", predict = "clogit_p",
        marginsnotok = "default Pc1 STDP DBeta DX2 GDBeta GDX2 Hat Residuals RSTAndard SCore",
        marginsok = "PU0 XB", marginsdefault = "predict(pu0)", group = m$group, depvar = depvar)
  } else if (cmd == "clogit") {
    m <- stata_m
    q <- sum(slopes)
    chi2 <- if (robust) wald(slopes) else 2 * (m$ll - m$ll_0)
    put(rank = q, N = m$N, k = length(b), k_eq = 1, k_dv = 1, converged = 1, rc = 0, ll = m$ll,
        k_eq_model = 1, ll_0 = m$ll_0)
    if (!is.null(G)) put(N_clust = G)
    put(df_m = q, chi2 = chi2, p = stats::pchisq(chi2, q, lower.tail = FALSE), N_drop = m$N_drop,
        N_group_drop = m$N_group_drop, r2_p = 1 - m$ll / m$ll_0)
    put(cmdline = cmdline, cmd = "clogit", predict = "clogit_p")
    if (m$multiple) put(multiple = "multiple")
    put(group = m$group, chi2type = if (robust) "Wald" else "LR", vce = vce,
        title = "Conditional (fixed-effects) logistic regression", depvar = depvar, properties = "b V")
    robust_macros()
  } else if (cmd == "margins") {
    put(N = extra$N, cmd = "margins", est_cmd = extra$est_cmd, cmdline = cmdline, properties = "b V")
  }
  # Stata's columns: base levels, left-out ones, each equation's constant last
  eb <- stata_eb(b, V, cmd, depvar)
  e$b <- eb$b
  e$V <- eb$V
  # Stata counts the base levels' columns too
  if (cmd == "clogit") e$k <- ncol(eb$b)
  for (n in names(after)) e[[n]] <- after[[n]]
  e
}"##;

/// The `e()` names Stata leaves after `cmd` that RCraft does not keep,
/// and why. Anything else not in `stata_e` is missing, as in Stata.
fn e_not_kept(cmd: &str, model: &str) -> &'static [&'static str] {
    match (cmd, model) {
        ("regress", _) => &["beta", "V_modelbased"],
        ("logit" | "probit", _) => &[
            "ic",
            "N_cdf",
            "N_cds",
            "mns",
            "rules",
            "ilog",
            "gradient",
            "V_modelbased",
        ],
        ("poisson" | "nbreg", _) => &["ic", "ilog", "gradient", "V_modelbased"],
        ("ologit" | "oprobit", _) => &["ic", "N_cd", "cat", "ilog", "gradient", "V_modelbased"],
        ("mlogit", _) => &["ic", "N_cd", "out", "ilog", "gradient", "V_modelbased"],
        ("xtreg", "fe") => &[
            "Tbar",
            "Tcon",
            "k_absorb",
            "sigma",
            "rank",
            "df_m",
            "df_b",
            "df_a",
            "tss",
            "rss",
            "mss",
            "rmse",
            "r2",
            "r2_a",
            "sum_w",
            "absvar",
            "kabsorb",
            "V_modelbased",
        ],
        ("xtreg", "re") => &[
            "rank",
            "df_m",
            "sigma",
            "rmse",
            "Tbar",
            "Tcon",
            "thta_min",
            "thta_5",
            "thta_50",
            "thta_95",
            "thta_max",
            "theta",
            "VCEf",
            "bf",
            "V_modelbased",
        ],
        ("xtreg", "be") => &[
            "sum_w", "df_m", "r2", "rmse", "mss", "rss", "r2_a", "ll", "ll_0", "rank", "Tbar",
            "Tcon", "beta",
        ],
        ("ivregress", _) => &[
            "iterations",
            "J",
            "cons",
            "wmatrix",
            "W",
            "S",
            "V_modelbased",
        ],
        ("reghdfe", _) => &["ic", "marginsnotok", "footnote", "estat_cmd", "dof_table"],
        ("robreg", _) => &[
            "level",
            "iterations",
            "b0",
            "b_init",
            "V_modelbased",
            "omit",
        ],
        ("asclogit", _) => &[
            "ic",
            "datasignaturevars",
            "datasignature",
            "deriv_useminbound",
            "opt",
            "user",
            "ml_method",
            "technique",
            "which",
            "stats",
            "alt_casevars",
            "altvals",
            "altfreq",
            "V_modelbased",
            "ilog",
            "gradient",
        ],
        ("clogit", _) => &[
            "ic",
            "ilog",
            "gradient",
            "Cns",
            "ml_method",
            "opt",
            "user",
            "technique",
            "which",
            "deriv_useminbound",
            "marginsnotok",
            "marginsok",
            "marginsdefault",
            "V_modelbased",
        ],
        ("stcox", _) => &[
            "datasignature",
            "datasignaturevars",
            "strata",
            "V_modelbased",
        ],
        ("anova", _) => &["beta", "sstype", "varnames"],
        ("margins", _) => &[
            "k_at",
            "k_by",
            "k_predict",
            "k_margins",
            "title",
            "model_vce",
            "vcetype",
            "expression",
            "predict1_label",
            "xvars",
            "derivatives",
            "continuous",
            "emptycells",
            "est_cmdline",
            "_N",
            "error",
            "Jacobian",
            "chainrule",
        ],
        _ => &[],
    }
}

/// Whether `e(name)` is one Stata has after `cmd` and RCraft does not keep.
fn e_refused(cmd: &str, model: &str, name: &str) -> bool {
    if e_not_kept(cmd, model).contains(&name) {
        return true;
    }
    // anova's terms, numbered: ss_1, df_1, F_1, term_1, ...
    cmd == "anova"
        && ["ss_", "df_", "F_", "term_"].iter().any(|p| {
            name.strip_prefix(p)
                .is_some_and(|k| !k.is_empty() && k.chars().all(|c| c.is_ascii_digit()))
        })
}

/// The `e()` and `r()` names that are macros (text) in Stata.
const TEXT_NAMES: &[&str] = &[
    "levels",
    "varlist",
    "badlist",
    "names",
    "cmd",
    "cmd2",
    "cmdline",
    "depvar",
    "title",
    "vce",
    "vcetype",
    "clustvar",
    "wtype",
    "wexp",
    "properties",
    // file read's r(status)
    "status",
    // bootstrap's
    "strata",
    "exp1",
    "exp2",
    "exp3",
    "exp4",
    "exp5",
    "exp6",
    "exp7",
    "exp8",
    "exp9",
    "predict",
    "model",
    "estat_cmd",
    "marginsok",
    "marginsnotok",
    "marginsdefault",
    "chi2type",
    "eqnames",
    "dispers",
    "ivar",
    "tvar",
    "absvar",
    "estimator",
    "exog",
    "exogr",
    "endog",
    "small",
    "footnote",
    "method",
    "ties",
    "t0",
    "est_cmd",
    "est_cmdline",
    "opt",
    "user",
    "ml_method",
    "technique",
    "which",
    "deriv_useminbound",
    "sstype",
    "varnames",
    "mcmethod",
    "emptycells",
    "expression",
    "model_vce",
    "predict1_label",
    "xvars",
    "derivatives",
    "continuous",
    "datasignature",
    "datasignaturevars",
    "marginstok",
    "wmatrix",
];

/// The matrices among them.
const MATRIX_NAMES: &[&str] = &[
    "b",
    "V",
    "C",
    "Nobs",
    "R",
    "sig",
    "beta",
    "V_modelbased",
    "mns",
    "rules",
    "ilog",
    "gradient",
    "cat",
    "out",
    "theta",
    "VCEf",
    "bf",
    "kabsorb",
    "table",
    "chainrule",
    "Jacobian",
    "error",
    "_N",
    "W",
    "S",
];

/// A model `estimates store` kept, and what the translator knew of it then.
#[derive(Debug, Clone)]
pub(super) struct StoredFit {
    estimates: Option<super::model::Estimates>,
    estimates_lost: Option<String>,
    e_ret: Returned,
    weighted_fit: bool,
    fit_fweight: bool,
    plain_regress: bool,
    fit_robust: bool,
    fit_cluster: bool,
    /// Its `stata_esample` still marks the rows, and its copy of the
    /// estimation sample is still the data's; a line since that moves the
    /// rows or changes the data clears these, as it does for the model held.
    pub(super) esample: bool,
    pub(super) changed_since_fit: bool,
}

/// The R objects a fit leaves, which `estimates store` keeps together.
const FIT_OBJECTS: &str = "c(\"stata_estimates\", \"stata_b\", \"stata_vcov\", \"stata_df\", \"stata_e\", \"stata_esample\", \"stata_data\", \"stata_cluster\", \"stata_levels\", \"stata_base\", \"stata_rows\")";

impl StoredFit {
    pub(super) fn estimates(&self) -> Option<&super::model::Estimates> {
        self.estimates.as_ref()
    }

    pub(super) fn robust_or_weighted(&self) -> bool {
        self.fit_robust || self.fit_cluster || self.weighted_fit
    }

    pub(super) fn changed_since(&self) -> bool {
        self.changed_since_fit
    }
}

impl Session {
    /// The results `estimates store` kept under `name`.
    pub(super) fn stored_fit(&self, name: &str) -> Option<&StoredFit> {
        self.stored_fits.get(name)
    }

    /// `estimates store`, `restore`, `drop` and `clear` (Stata 19: `sto` and
    /// `res` are the shortest; a name not stored is r(111), storing with no
    /// model r(301)).
    pub(super) fn estimates_cmd(&mut self, c: &Command, out: &mut Out) -> Result<(), Untranslated> {
        let (sub, rest) = super::first_word(&c.parts.main);
        if sub.len() >= 3 && "table".starts_with(sub) {
            return self.estimates_table(c, rest, out);
        }
        if !c.options.is_empty() {
            return Err(no(format!(
                "estimates {sub} with options is not translated yet"
            )));
        }
        let one_name = |what: &str| -> Result<String, Untranslated> {
            let (name, more) = super::first_word(rest);
            if name.is_empty() || !more.is_empty() {
                return Err(no(format!("estimates {what} needs one name")));
            }
            if !super::is_name(name) {
                return Err(no(format!("(Stata: {name} invalid name)")));
            }
            Ok(name.to_string())
        };
        match sub {
            s if s.len() >= 3 && "store".starts_with(s) => {
                let name = one_name("store")?;
                self.store_fit(&name, "estimates store", out)
            }
            s if s.len() >= 3 && "restore".starts_with(s) => {
                let name = one_name("restore")?;
                self.restore_fit(&name, out)
            }
            "clear" => {
                if !rest.is_empty() {
                    return Err(no("estimates clear takes no names"));
                }
                self.stored_fits.clear();
                self.active_store = None;
                out.line("stata_stored <- list()");
                Ok(())
            }
            "drop" => {
                let names: Vec<&str> = rest.split_whitespace().collect();
                self.drop_fits(&names, out)
            }
            other => Err(no(format!("estimates {other} is not translated yet"))),
        }
    }

    /// Keeps the results now active under `name` (`estimates store`, `eststo`).
    pub(super) fn store_fit(
        &mut self,
        name: &str,
        who: &str,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if matches!(self.e_ret, Returned::Nothing) {
            return Err(no(format!(
                "{who}: no model is fitted before it in this do-file (Stata: last estimation results not found, nothing to store)"
            )));
        }
        let name = name.to_string();
        // stored results carry their name, as e(_estimates_name)
        out.line(format!(
            "if (exists(\"stata_e\")) stata_e[[\"_estimates_name\"]] <- {}",
            r_text(&name)
        ));
        self.e_text.insert("_estimates_name".into());
        self.stored_fits.insert(
            name.clone(),
            StoredFit {
                estimates: self.estimates.clone(),
                estimates_lost: self.estimates_lost.clone(),
                e_ret: self.e_ret.clone(),
                weighted_fit: self.weighted_fit,
                fit_fweight: self.fit_fweight,
                plain_regress: self.plain_regress,
                fit_robust: self.fit_robust,
                fit_cluster: self.fit_cluster,
                esample: self.esample,
                changed_since_fit: self.changed_since_fit,
            },
        );
        out.line("if (!exists(\"stata_stored\")) stata_stored <- list()");
        out.line(format!(
            "stata_stored[[{}]] <- mget(intersect({FIT_OBJECTS}, ls()))",
            r_text(&name)
        ));
        self.active_store = Some(name);
        Ok(())
    }

    /// Makes the results kept under `name` the active ones (`estimates restore`).
    pub(super) fn restore_fit(&mut self, name: &str, out: &mut Out) -> Result<(), Untranslated> {
        let Some(f) = self.stored_fits.get(name).cloned() else {
            return self.not_stored("restore", name, out);
        };
        self.estimates = f.estimates;
        self.estimates_lost = f.estimates_lost;
        self.e_ret = f.e_ret;
        self.weighted_fit = f.weighted_fit;
        self.fit_fweight = f.fit_fweight;
        self.plain_regress = f.plain_regress;
        self.fit_robust = f.fit_robust;
        self.fit_cluster = f.fit_cluster;
        self.esample = f.esample;
        self.changed_since_fit = f.changed_since_fit;
        self.active_store = Some(name.to_string());
        let n = r_text(name);
        out.line(format!(
            "if (!exists(\"stata_stored\") || is.null(stata_stored[[{n}]])) stop({})",
            r_text(&format!("estimation result {name} not found"))
        ));
        out.line(format!("rm(list = intersect({FIT_OBJECTS}, ls()))"));
        out.line(format!(
            "invisible(list2env(stata_stored[[{n}]], environment()))"
        ));
        out.line(format!(
            "cat({})",
            r_text(&format!("(results {name} are active now)\n"))
        ));
        Ok(())
    }

    /// Drops the results kept under `names` (`_all` or `*` for all).
    pub(super) fn drop_fits(&mut self, names: &[&str], out: &mut Out) -> Result<(), Untranslated> {
        if names.is_empty() {
            return Err(no("estimates drop needs names"));
        }
        if names.iter().any(|n| *n == "_all" || *n == "*") {
            self.stored_fits.clear();
            self.active_store = None;
            out.line("stata_stored <- list()");
            return Ok(());
        }
        if let Some(n) = names.iter().find(|n| !self.stored_fits.contains_key(**n)) {
            return self.not_stored("drop", n, out);
        }
        for n in names {
            self.stored_fits.remove(*n);
            if self.active_store.as_deref() == Some(*n) {
                self.active_store = None;
            }
        }
        out.line(format!(
            "stata_stored[c({})] <- NULL",
            names
                .iter()
                .map(|n| r_text(n))
                .collect::<Vec<_>>()
                .join(", ")
        ));
        Ok(())
    }

    /// `estimates restore` or `drop` of a name `estimates store` did not
    /// keep: Stata's error when no line that was not translated could have
    /// stored it (eststo, a program), refused otherwise.
    fn not_stored(&mut self, what: &str, name: &str, out: &mut Out) -> Result<(), Untranslated> {
        if self.stores_lost {
            return Err(no(format!(
                "estimates {what} {name}: no line translated before it stored {name}, and one that was not may have"
            )));
        }
        out.line(format!(
            "stop({})",
            r_text(&format!("estimation result {name} not found"))
        ));
        Ok(())
    }
}

/// Who left the values `e()` or `r()` would read.
#[derive(Debug, Clone, Default)]
pub(super) enum Returned {
    /// Nothing yet in this do-file (R may hold an earlier run's values).
    #[default]
    Nothing,
    /// The command named left them, as Stata would. `model` is a detail of
    /// it (xtreg's fe/re/be) and `names` the r() names it leaves.
    Kept {
        cmd: String,
        model: String,
        names: &'static [&'static str],
    },
    /// A command since may have replaced them in Stata, so they are not known.
    Lost(String),
    /// An r-class program the do-file defines returned them, or (for e())
    /// `ereturn` posted them: the program's name (`ereturn` at the top), and
    /// the names that are text (any name not set is missing).
    Program { cmd: String, texts: Vec<String> },
}

/// Commands after which Stata 19 still holds the r() it held before
/// (checked one by one); every other command replaces or clears r().
const LEAVE_R_ALONE: &[&str] = &[
    // file open, write and close (file read sets r() itself)
    "file",
    // a program called and returning, and return inside it, set r() themselves
    "_rcraft_program_begin",
    "_rcraft_program_end",
    "return",
    "parmest",
    "outreg2",
    "eststo",
    "estadd",
    "generate",
    "replace",
    "sort",
    "gsort",
    "list",
    "label",
    "rename",
    "format",
    "order",
    "display",
    "predict",
    "save",
    "saveold",
    "use",
    "egen",
    "clear",
    "version",
    "set",
    "ereturn",
    "return",
    "notes",
    "note",
    "codebook_",
    "assert",
    "scalar",
];

/// Commands that never change e() (an untranslated command not here might
/// be an estimation command, after which e() is not known).
const LEAVE_E_ALONE: &[&str] = &[
    // write a table or look something up only, translated or not
    "putexcel",
    "outreg",
    "frmttable",
    "which",
    "file",
    "_rcraft_program_begin",
    "_rcraft_program_end",
    "return",
    "parmest",
    "fre",
    "use",
    "save",
    "saveold",
    "import",
    "export",
    "generate",
    "replace",
    "drop",
    "keep",
    "rename",
    "sort",
    "gsort",
    "order",
    "label",
    "count",
    "list",
    "describe",
    "set",
    "clear",
    "version",
    "recode",
    "egen",
    "merge",
    "append",
    "reshape",
    "collapse",
    "duplicates",
    "encode",
    "decode",
    "destring",
    "tostring",
    "summarize",
    "tabulate",
    "correlate",
    "tab1",
    "tab2",
    "pwcorr",
    "ttest",
    "predict",
    "test",
    "lincom",
    "estat",
    "histogram",
    "kdensity",
    "scatter",
    "line",
    "connected",
    "twoway",
    "graph",
    "lowess",
    "xtset",
    "stset",
    "display",
    "ereturn",
    "return",
    "format",
    "compress",
    "notes",
    "note",
    "assert",
    "matrix",
    "scalar",
    "tsset",
    "codebook",
    "inspect",
    "browse",
    "edit",
    "cd",
    "log",
    "capture",
    "quietly",
    "noisily",
    "help",
    "search",
    "preserve",
    "restore",
    "sample",
    "expand",
    "fillin",
    "contract",
    "xpose",
    "insheet",
    "outsheet",
    "infile",
    "export",
];

/// Commands that read the model Stata holds and leave it there, unless they
/// are told to `post` their own results.
const READ_THE_MODEL: &[&str] = &[
    "testparm",
    "testnl",
    "nlcom",
    "predictnl",
    "lrtest",
    "hausman",
    "contrast",
    "pwcompare",
    "margins",
    "marginsplot",
    "lroc",
    "lsens",
    "lstat",
    "vif",
    "hettest",
    "ovtest",
    "imtest",
    "dfbeta",
    "coefplot",
    "esttab",
    "estout",
    "outreg2",
];

/// Whether a line, translated or not, leaves e() and the model as they were:
/// `estimates store` and `estimates table` do, `estimates restore` and
/// `estimates use` do not.
fn leaves_e_alone(c: &Command) -> bool {
    let name = c.name.as_str();
    // ereturn sets e() but for list and display
    if name == "ereturn" {
        let sub = c.parts.main.split_whitespace().next().unwrap_or("");
        return sub.len() >= 2 && ("list".starts_with(sub) || "display".starts_with(sub));
    }
    if LEAVE_E_ALONE.contains(&name) {
        return true;
    }
    if READ_THE_MODEL.contains(&name) {
        return !c.options.iter().any(|o| o.is("post", 4));
    }
    // the estout package's store and add: the active results stay
    if name == "eststo" || name == "estadd" || name == "outreg2" {
        return true;
    }
    if name == "estimates" {
        let sub = c.parts.main.split_whitespace().next().unwrap_or("");
        let restores = sub == "use" || (sub.len() >= 3 && "restore".starts_with(sub));
        return !restores;
    }
    false
}

/// The r() each r-class command leaves here, as Stata 19's `return list`
/// shows them (the ones RCraft keeps).
pub(super) const R_SUMMARIZE: &[&str] = &["N", "sum_w", "mean", "Var", "sd", "min", "max", "sum"];
pub(super) const R_SUMMARIZE_DETAIL: &[&str] = &[
    "N", "sum_w", "mean", "Var", "sd", "skewness", "kurtosis", "sum", "min", "max", "p1", "p5",
    "p10", "p25", "p50", "p75", "p90", "p95", "p99",
];
pub(super) const R_MEANONLY: &[&str] = &["N", "sum_w", "mean", "min", "max", "sum"];
pub(super) const R_COUNT: &[&str] = &["N"];
/// What `file read` leaves in r().
pub(super) const R_FILE_READ: &[&str] = &["eof", "status"];
pub(super) const R_LEVELSOF: &[&str] = &["N", "r", "levels"];
/// What `split` leaves in r().
pub(super) const R_SPLIT: &[&str] = &["k_new", "varlist"];
pub(super) const R_ISVAR: &[&str] = &["varlist", "badlist"];
pub(super) const R_EST_TABLE: &[&str] = &["names"];
pub(super) const R_ALPHA: &[&str] = &["k"];
pub(super) const R_FILEFILTER: &[&str] = &["occurrences", "bytes_from", "bytes_to"];
pub(super) const R_ALPHA_CASEWISE: &[&str] = &["k", "alpha", "cov", "rho"];
/// What `test` leaves in r(), with r(dropped_1) and the like for each
/// constraint it drops (one of base levels only)
pub(super) const R_TEST: &[&str] = &[
    "dropped_1",
    "dropped_2",
    "dropped_3",
    "dropped_4",
    "dropped_5",
    "dropped_6",
    "dropped_7",
    "dropped_8",
    "dropped_9",
    "dropped_10",
    "drop",
    "df_r",
    "F",
    "df",
    "p",
    "chi2",
];
pub(super) const R_LINCOM: &[&str] = &["df", "ub", "lb", "p", "t", "z", "level", "se", "estimate"];
pub(super) const R_TAB1: &[&str] = &["N", "r"];
pub(super) const R_FRE: &[&str] = &[
    "N",
    "N_valid",
    "N_missing",
    "r",
    "r_valid",
    "r_missing",
    "depvar",
];
pub(super) const R_TAB2: &[&str] = &["N", "r", "c", "chi2", "p"];
pub(super) const R_CORRELATE: &[&str] = &["sum_w", "N", "rho", "cov_12", "Var_2", "Var_1"];
pub(super) const R_DESCRIBE: &[&str] = &["k", "N"];
pub(super) const R_TTEST: &[&str] = &[
    "N_1",
    "N_2",
    "mu_1",
    "mu_2",
    "sd_1",
    "sd_2",
    "se_1",
    "se_2",
    "lb_1",
    "ub_1",
    "lb_2",
    "ub_2",
    "N_combined",
    "mu_combined",
    "se_combined",
    "lb_combined",
    "ub_combined",
    "mu_diff",
    "sd_diff",
    "lb_diff",
    "ub_diff",
    "t",
    "df_t",
    "p",
    "p_l",
    "p_u",
    "se",
    "sd",
    "level",
];
pub(super) const R_MARGINS: &[&str] = &["N", "level", "cmd", "est_cmd", "est_cmdline", "b", "V"];

/// The r() Stata leaves that R has nothing to give for, and why.
fn r_not_kept(cmd: &str, name: &str) -> Option<&'static str> {
    match (cmd, name) {
        ("describe", "width") => Some(
            "it is the bytes a row takes in Stata's storage types, which R's columns do not have",
        ),
        ("describe", "changed") => Some(
            "it says whether the data changed since they were last saved, which R does not follow",
        ),
        _ => None,
    }
}

impl Session {
    /// After each command: who now holds e() and r().
    pub(super) fn track_returned(
        &mut self,
        c: &Command,
        ok: bool,
        r_left: Option<(&'static [&'static str], String)>,
    ) {
        let name = c.name.as_str();
        match r_left {
            // isvar clears r() and leaves only these: any other name is
            // missing, as after an r-class program
            Some((names, cmd)) if ok && cmd == "isvar" => {
                self.helpers.insert("stata_returned");
                self.r_ret = Returned::Program {
                    cmd,
                    texts: names.iter().map(|n| n.to_string()).collect(),
                }
            }
            Some((names, cmd)) if ok => {
                // what reads r() back (stata_rget) comes with it
                self.helpers.insert("stata_returned");
                self.r_ret = Returned::Kept {
                    cmd,
                    model: String::new(),
                    names,
                }
            }
            _ if LEAVE_R_ALONE.contains(&name) => {}
            // matrix list (the one matrix command translated) leaves it too
            _ if ok && name == "matrix" => {}
            _ => {
                self.r_ret = Returned::Lost(if ok {
                    format!("{name} replaces r() in Stata with results RCraft does not keep yet")
                } else {
                    format!("{name}, which was not translated, replaces r() in Stata")
                })
            }
        }
        if !ok && !leaves_e_alone(c) {
            self.e_ret = Returned::Lost(format!(
                "{name} was not translated, and if it is an estimation command it replaced e() in Stata"
            ));
        }
    }

    /// After a line that is not translated: if it may be an estimation
    /// command, the model Stata holds is not the one R fitted last, so
    /// `predict`, `estat` and `margins` cannot read it.
    pub(super) fn lose_estimates(&mut self, c: &Command, reason: &str) {
        // a line Stata stops at, or one only R can judge, fits nothing new
        if leaves_e_alone(c)
            || reason.contains("Stata stops here")
            || reason.contains("(Stata: ")
            || reason.contains(crate::dofile::KNOWN_WHEN_RUN)
        {
            return;
        }
        if self.estimates.take().is_some() {
            self.estimates_lost = Some(format!(
                "the model Stata holds here is not known: {}, which was not translated, may have fitted one",
                c.text.trim()
            ));
        }
    }

    /// `e(name)`, `r(name)`, `_b[key]` or `_se[key]` in an expression.
    pub(super) fn returned_value(
        &self,
        which: &str,
        name: &str,
    ) -> Result<(String, Kind), Untranslated> {
        match which {
            "ts" => self.ts_code(name).map(|code| (code, Kind::Number)),
            // el(e(V), i, j): the matrix itself, as matrix list takes it
            // a matrix the do-file made
            "m#matrix" => self.user_matrix(name).map(|m| (m, Kind::Number)),
            "e#matrix" | "r#matrix" => self
                .returned_matrix(which.get(..1).unwrap_or("e"), name)
                .map(|code| (code, Kind::Number)),
            "_b" | "_coef" | "_se" => {
                match &self.e_ret {
                    Returned::Kept { .. } => {}
                    Returned::Nothing => {
                        return Err(no(format!(
                            "{which}[{name}]: no model is fitted before it in this do-file (Stata: _b not allowed when e(b) is not present)"
                        )))
                    }
                    Returned::Lost(why) => return Err(no(format!("{which}[{name}] is not known here: {why}"))),
                    // posted by ereturn: read off e(b) and e(V)
                    Returned::Program { .. } => {
                        return Ok((
                            format!(
                                "stata_bpost({}{})",
                                r_text(name.trim_start_matches(':')),
                                if which == "_se" { ", se = TRUE" } else { "" }
                            ),
                            Kind::Number,
                        ))
                    }
                }
                Ok((
                    format!(
                        "stata_bget({}{})",
                        r_text(name.trim_start_matches(':')),
                        if which == "_se" { ", se = TRUE" } else { "" }
                    ),
                    Kind::Number,
                ))
            }
            "e" | "r" => {
                let state = if which == "e" {
                    &self.e_ret
                } else {
                    &self.r_ret
                };
                let shown = format!("{which}({name})");
                match state {
                    Returned::Nothing => Err(no(format!(
                        "{shown}: nothing before it in this do-file leaves {which}() (R might hold an earlier run's)"
                    ))),
                    Returned::Lost(why) => Err(no(format!("{shown} is not known here: {why}"))),
                    // what an r-class program returned; a name it did not is missing, as in Stata
                    // or what ereturn posted
                    Returned::Program { texts, .. } => {
                        let text = texts.iter().any(|t| t == name);
                        let get = if which == "e" { "stata_eget" } else { "stata_rget" };
                        Ok((
                            format!("{get}({}{})", r_text(name), if text { ", text = TRUE" } else { "" }),
                            if text { Kind::Text } else { Kind::Number },
                        ))
                    }
                    Returned::Kept { cmd, model, names } => {
                        if MATRIX_NAMES.contains(&name) {
                            return Err(no(format!(
                                "{shown} is a matrix, which an expression cannot take (Stata: type mismatch); matrix list {shown} shows it"
                            )));
                        }
                        if which == "e" && e_refused(cmd, model, name) {
                            return Err(no(format!(
                                "{shown}, which Stata leaves after {cmd}, is not kept in R yet"
                            )));
                        }
                        if which == "r" {
                            if let Some(why) = r_not_kept(cmd, name) {
                                return Err(no(format!("{shown} after {cmd} is not kept in R: {why}")));
                            }
                        }
                        if which == "r" && !names.contains(&name) {
                            return Err(no(format!(
                                "{shown} after {cmd} is not kept in R yet (RCraft keeps r() {})",
                                names.join(", ")
                            )));
                        }
                        let text = TEXT_NAMES.contains(&name) || (which == "e" && self.e_text.contains(name));
                        let get = if which == "e" { "stata_eget" } else { "stata_rget" };
                        Ok((
                            format!(
                                "{get}({}{})",
                                r_text(name),
                                if text { ", text = TRUE" } else { "" }
                            ),
                            if text { Kind::Text } else { Kind::Number },
                        ))
                    }
                }
            }
            "scalar" => self.scalar_value(name, false),
            // a bare name: a scalar only if it is one (else the variable)
            "scalar?" => self.scalar_value(name, true),
            _ => Err(no(format!("{which}() is not translated"))),
        }
    }

    /// The R of a matrix a command left, for `matrix list`.
    pub(super) fn returned_matrix(&self, which: &str, name: &str) -> Result<String, Untranslated> {
        let shown = format!("{which}({name})");
        let state = if which == "e" {
            &self.e_ret
        } else {
            &self.r_ret
        };
        match state {
            Returned::Nothing => Err(no(format!(
                "matrix list {shown}: nothing before it in this do-file leaves {which}()"
            ))),
            Returned::Lost(why) => Err(no(format!("matrix list {shown}: {why}"))),
            Returned::Program { cmd, .. } => {
                if which == "r" {
                    return Err(no(format!(
                        "matrix list {shown} after the program {cmd} is not translated yet"
                    )));
                }
                Ok(format!("stata_e[[{}]]", r_text(name)))
            }
            Returned::Kept { cmd, model, .. } => {
                if which == "e" {
                    if matches!(cmd.as_str(), "anova" | "margins")
                        && !matches!(model.as_str(), "suest" | "dydx")
                    {
                        return Err(no(format!(
                            "matrix list {shown} after {cmd} is not translated yet: Stata's columns there are not rebuilt in R"
                        )));
                    }
                    if e_refused(cmd, model, name) {
                        return Err(no(format!(
                            "{shown}, which Stata leaves after {cmd}, is not kept in R yet"
                        )));
                    }
                    Ok(format!("stata_e[[{}]]", r_text(name)))
                } else {
                    Ok(format!("stata_r[[{}]]", r_text(name)))
                }
            }
        }
    }
    /// The line that keeps a model's e() in R, after its translation.
    pub(super) fn keep_e(&mut self, c: &Command, out: &mut Out, args: &str, model: &str) {
        self.helpers.insert("stata_ereturn");
        self.helpers.insert("stata_eb");
        self.helpers.insert("stata_returned");
        self.helpers.insert("stata_names");
        self.helpers.insert("stata_ml_header");
        let cmdline = cmdline(c);
        out.line(format!(
            "stata_e <- stata_ereturn({}, {}{args})",
            r_text(&c.name),
            r_text(&cmdline)
        ));
        self.e_new = true;
        self.e_ret = Returned::Kept {
            cmd: c.name.clone(),
            model: model.to_string(),
            names: &[],
        };
    }

    /// `display`: text, expressions, formats and the column directives.
    pub(super) fn display(&mut self, c: &Command, out: &mut Out) -> Result<(), Untranslated> {
        let rest = after_name(c);
        let chars: Vec<char> = rest.chars().collect();
        let mut i = 0;
        let mut pieces: Vec<String> = Vec::new();
        let mut fmt: Option<String> = None;
        let at = |k: usize| chars.get(k).copied();
        while i < chars.len() {
            let Some(&ch) = chars.get(i) else { break };
            if ch.is_whitespace() {
                i += 1;
                continue;
            }
            // a comma prints a space
            if ch == ',' {
                pieces.push("list(\"text\", \" \")".into());
                i += 1;
                continue;
            }
            if ch == '"' || (ch == '`' && at(i + 1) == Some('"')) {
                let (text, end) =
                    quoted(&chars, i).ok_or_else(|| no("display: a quote is not closed"))?;
                match fmt.take() {
                    Some(f) => pieces.push(format!(
                        "list(\"value\", {}, {})",
                        r_text(&text),
                        r_text(&f)
                    )),
                    None => pieces.push(format!("list(\"text\", {})", r_text(&text))),
                }
                i = end;
                continue;
            }
            if ch == '%' {
                let end = (i..chars.len())
                    .find(|&k| at(k).is_some_and(char::is_whitespace))
                    .unwrap_or(chars.len());
                let f = text_of(&chars, i, end);
                if !valid_format(&f) {
                    return Err(no(format!("display: the format {f} is not translated (Stata: invalid %format, or a format RCraft does not have yet)")));
                }
                // datetime/C reads the leap seconds from the date helpers
                if f.contains("tC") {
                    self.helpers.insert("stata_dates");
                }
                fmt = Some(f);
                i = end;
                continue;
            }
            // directives: _n, _newline(#), _col(#), _skip(#), _dup(#), _continue,
            // and the colours (as text/result/error/input, in color)
            let word: String = tail(&chars, i)
                .iter()
                .take_while(|c| c.is_alphanumeric() || **c == '_')
                .collect();
            let arg = |from: usize| -> Option<(i64, usize)> {
                if at(from) != Some('(') {
                    return None;
                }
                let close = (from..chars.len()).find(|&k| at(k) == Some(')'))?;
                let inner = text_of(&chars, from + 1, close);
                Some((inner.trim().parse().ok()?, close + 1))
            };
            let directive = |names: &[(&str, usize)]| {
                names
                    .iter()
                    .any(|(n, min)| word.len() >= *min && n.starts_with(word.as_str()))
            };
            if word.starts_with('_') && word.len() > 1 {
                let j = i + word.len();
                if directive(&[("_newline", 2)]) {
                    let (k, end) = arg(j).unwrap_or((1, j));
                    pieces.push(format!("list(\"newline\", {k})"));
                    i = end;
                    continue;
                }
                if directive(&[("_column", 2)]) {
                    let (k, end) = arg(j).ok_or_else(|| no("display: _column() needs a number"))?;
                    pieces.push(format!("list(\"col\", {k})"));
                    i = end;
                    continue;
                }
                if directive(&[("_skip", 2)]) {
                    let (k, end) = arg(j).unwrap_or((1, j));
                    pieces.push(format!("list(\"skip\", {k})"));
                    i = end;
                    continue;
                }
                if directive(&[("_duplicate", 2)]) {
                    let (k, end) = arg(j).ok_or_else(|| no("display: _dup() needs a number"))?;
                    let rest = tail(&chars, end);
                    let skip = rest.iter().take_while(|c| c.is_whitespace()).count();
                    let (text, used) = quoted(rest, skip).ok_or_else(|| {
                        no("display: _dup() is translated before quoted text only")
                    })?;
                    pieces.push(format!("list(\"dup\", {k}, {})", r_text(&text)));
                    i = end + used;
                    continue;
                }
                if directive(&[("_continue", 2), ("_request", 2), ("_char", 2)]) {
                    return Err(no(format!("display: {word} is not translated yet")));
                }
            }
            if matches!(word.as_str(), "as" | "in") {
                // colours: R prints plain text
                let after = text_of(&chars, i + 2, chars.len());
                let next = after.trim_start();
                let colour: String = next.chars().take_while(|c| c.is_alphabetic()).collect();
                if word == "as"
                    && !["text", "txt", "result", "error", "err", "input", "inp"]
                        .contains(&colour.as_str())
                {
                    return Err(no(format!("display: as {colour} is not a style Stata has")));
                }
                i = i + 2 + (after.len() - next.len()) + colour.len();
                out.caveat("as/in colours are left out: R prints plain text");
                continue;
            }
            // an expression, as far as it goes
            let text = text_of(&chars, i, chars.len());
            let (_, used) = parse_prefix(&text)
                .map_err(|e| no(format!("display: Stata would not read it: {}", e.message)))?;
            let used_chars = used;
            let e = text_of(&chars, i, i + used_chars);
            let (code, kind) = self.display_value(e.trim(), out)?;
            let f = fmt.take().unwrap_or_default();
            if kind == Kind::Text && !f.is_empty() && !f.ends_with('s') {
                return Err(no(format!(
                    "display: {f} is a number format, given text (Stata: type mismatch)"
                )));
            }
            pieces.push(format!("list(\"value\", {code}, {})", r_text(&f)));
            i += used_chars;
        }
        self.helpers.insert("stata_fmt");
        out.line(format!("stata_display({})", pieces.join(", ")));
        Ok(())
    }

    /// One expression of `display`: a variable stands for its first row, as
    /// in Stata, so it is worked out over the data when there is any.
    pub(super) fn display_value(
        &self,
        e: &str,
        out: &mut Out,
    ) -> Result<(String, Kind), Untranslated> {
        // a random draw is taken once, as Stata does for the first row
        let (code, kind, _) = self.drawing(e, Use::Value, "1", out)?;
        let scalar = |n: &str| self.scalars.contains_key(n) && self.kind_of(n).is_none();
        let data_free = !mentions_data(e, &scalar);
        if data_free {
            return Ok((code, kind));
        }
        // a value from the data, with no data known: said why, not run without it
        let d = self.data()?;
        out.package("dplyr");
        Ok((
            format!("dplyr::transmute({d}, stata_v = {code})$stata_v[1]"),
            kind,
        ))
    }

    /// `ereturn list` and `return list`.
    pub(super) fn list_returned(&mut self, c: &Command, out: &mut Out) -> Result<(), Untranslated> {
        let which = if c.name == "ereturn" { "e" } else { "r" };
        let sub = c.parts.main.trim();
        if !matches!(sub, "list" | "li" | "lis") {
            return Err(no(format!("{} {sub} is not translated yet", c.name)));
        }
        let state = if which == "e" {
            &self.e_ret
        } else {
            &self.r_ret
        };
        match state {
            Returned::Nothing => {
                return Err(no(format!(
                    "{} list: nothing before it in this do-file leaves {which}()",
                    c.name
                )))
            }
            Returned::Lost(why) => return Err(no(format!("{} list: {why}", c.name))),
            Returned::Program { .. } => {}
            Returned::Kept { cmd, model, .. } => {
                if which == "e" {
                    let missing = e_not_kept(cmd, model);
                    if !missing.is_empty() {
                        out.caveat(format!(
                            "Stata also lists {} after {cmd}, which RCraft does not keep; and the optimizer's settings (e(opt), e(user), e(technique), ...)",
                            missing.join(", ")
                        ));
                    }
                } else if cmd == "describe" {
                    out.caveat(
                        "Stata also lists r(changed) and r(width) after describe, which RCraft does not keep",
                    );
                }
            }
        }
        self.helpers.insert("stata_returned");
        self.helpers.insert("stata_fmt");
        out.line(format!(
            "stata_list_returned({}, \"{which}\")",
            if which == "e" { "stata_e" } else { "stata_r" }
        ));
        Ok(())
    }
}

/// The command line as Stata keeps it in `e(cmdline)`: the line without
/// its prefixes.
pub(super) fn cmdline(c: &Command) -> String {
    let rest = after_name(c);
    let rest = rest.trim();
    if rest.is_empty() {
        c.written.clone()
    } else {
        format!("{} {rest}", c.written)
    }
}

/// The text of the line after the command word.
pub(super) fn after_name(c: &Command) -> String {
    let text = c.text.as_str();
    let mut from = 0;
    // past the prefixes: the command word is the first one that is it
    for (k, _) in text.match_indices(c.written.as_str()) {
        let before_ok = k == 0
            || text[..k]
                .chars()
                .last()
                .is_some_and(|ch| ch.is_whitespace() || ch == ':');
        let after = text[k + c.written.len()..].chars().next();
        let after_ok = after.is_none_or(|ch| !(ch.is_alphanumeric() || ch == '_'));
        if before_ok && after_ok {
            from = k + c.written.len();
            break;
        }
    }
    text[from..].to_string()
}

/// The characters from `from` to `to`, as text (none past the end).
fn text_of(chars: &[char], from: usize, to: usize) -> String {
    chars
        .get(from..to.min(chars.len()))
        .map(|s| s.iter().collect())
        .unwrap_or_default()
}

/// The characters from `from` on.
fn tail(chars: &[char], from: usize) -> &[char] {
    chars.get(from..).unwrap_or(&[])
}

/// A quoted string from `start` (`"..."` or `` `"..."' ``): its text and where
/// it ends.
fn quoted(chars: &[char], start: usize) -> Option<(String, usize)> {
    if chars.get(start) == Some(&'"') {
        let end = (start + 1..chars.len()).find(|&k| chars.get(k) == Some(&'"'))?;
        return Some((text_of(chars, start + 1, end), end + 1));
    }
    if chars.get(start) == Some(&'`') && chars.get(start + 1) == Some(&'"') {
        let end = (start + 2..chars.len())
            .find(|&k| chars.get(k) == Some(&'"') && chars.get(k + 1) == Some(&'\''))?;
        return Some((text_of(chars, start + 2, end), end + 2));
    }
    None
}

/// The display formats `stata_fmt` has.
pub(super) fn valid_format(f: &str) -> bool {
    // the hexadecimal form, which shows a double exactly
    if matches!(f, "%21x" | "%-21x") {
        return true;
    }
    if valid_date_format(f) {
        return true;
    }
    let b = f.as_bytes();
    if b.first() != Some(&b'%') {
        return false;
    }
    let mut k = 1;
    if b.get(k) == Some(&b'-') {
        k += 1;
    }
    if b.get(k) == Some(&b'0') {
        k += 1;
    }
    let w0 = k;
    while b.get(k).is_some_and(u8::is_ascii_digit) {
        k += 1;
    }
    if k == w0 {
        return false;
    }
    if b.get(k) == Some(&b'.') {
        k += 1;
        let d0 = k;
        while b.get(k).is_some_and(u8::is_ascii_digit) {
            k += 1;
        }
        if k == d0 {
            return false;
        }
    }
    match b.get(k) {
        Some(b'g' | b'f' | b'e') => {
            k += 1;
            if b.get(k) == Some(&b'c') {
                k += 1;
            }
        }
        Some(b's') => k += 1,
        _ => return false,
    }
    k == b.len()
}

/// Whether an expression reads the data: a name other than the ones that
/// stand for something else (`_n`, `_N`, `_pi`, functions).
fn mentions_data(e: &str, scalar: &dyn Fn(&str) -> bool) -> bool {
    fn walk(x: &Expr, scalar: &dyn Fn(&str) -> bool) -> bool {
        match x {
            Expr::Name(n) => !matches!(n.as_str(), "_pi" | "c" | "_rc") && !scalar(n),
            Expr::Subscript(n, i) => {
                !matches!(n.as_str(), "_b" | "_se" | "_coef") || walk(i, scalar)
            }
            // a matrix by name, then its indices
            Expr::Call(name, args)
                if matches!(
                    name.as_str(),
                    "rowsof" | "colsof" | "el" | "__matel" | "colnumb" | "rownumb"
                ) =>
            {
                args.iter().skip(1).any(|a| walk(a, scalar))
            }
            // functions of matrices, named
            Expr::Call(name, _)
                if matches!(
                    name.as_str(),
                    "trace" | "det" | "diag0cnt" | "issymmetric" | "matmissing" | "mreldif"
                ) =>
            {
                false
            }
            Expr::Call(name, args) => {
                !matches!(name.as_str(), "e" | "r" | "scalar")
                    && args.iter().any(|a| walk(a, scalar))
            }
            Expr::Unary(_, a) => walk(a, scalar),
            Expr::Binary(_, a, b) => walk(a, scalar) || walk(b, scalar),
            _ => false,
        }
    }
    // what does not parse is left to the full translation to refuse
    match crate::lang::expr::parse(e) {
        Ok(t) => walk(&t, scalar),
        Err(_) => true,
    }
}

/// `ttest`: Stata's r() of each of its four tests, in the order of its `return list`.
pub(super) const TTEST_R_HELPER: &str = r#"stata_ttest_r <- function(x, g = NULL, y = NULL, mu = NULL, paired = FALSE, level = 95, equal = TRUE) {
  # Stata's r() of ttest: one sample (mu), paired, two samples (y), or by(g)
  num <- function(v) { u <- unclass(v); attributes(u) <- NULL; as.numeric(u) }
  x <- num(x)
  q <- (1 + level / 100) / 2
  one <- function(v) {
    n <- length(v)
    se <- stats::sd(v) / sqrt(n)
    h <- stats::qt(q, n - 1) * se
    list(N = n, mu = mean(v), sd = stats::sd(v), se = se, lb = mean(v) - h, ub = mean(v) + h)
  }
  tests <- function(t, df) {
    list(p_u = stats::pt(t, df, lower.tail = FALSE), p_l = stats::pt(t, df), p = 2 * stats::pt(-abs(t), df),
         t = t, df_t = df)
  }
  if (!is.null(mu)) {
    a <- one(x[!is.na(x)])
    return(c(list(ub_1 = a$ub, lb_1 = a$lb, level = level, sd_1 = a$sd, se = a$se),
             tests((a$mu - mu) / a$se, a$N - 1), list(mu_1 = a$mu, N_1 = a$N)))
  }
  if (paired) {
    y <- num(y)
    ok <- !is.na(x) & !is.na(y)
    a <- one(x[ok])
    b <- one(y[ok])
    d <- one(x[ok] - y[ok])
    return(c(list(ub_diff = d$ub, lb_diff = d$lb, sd_diff = d$sd, mu_diff = d$mu, ub_2 = b$ub, lb_2 = b$lb,
                  se_2 = b$se, ub_1 = a$ub, lb_1 = a$lb, se_1 = a$se, level = level, sd_2 = b$sd,
                  sd_1 = a$sd, se = d$se),
             tests(d$mu / d$se, d$N - 1), list(mu_2 = b$mu, N_2 = b$N, mu_1 = a$mu, N_1 = a$N)))
  }
  if (is.null(g)) {
    y <- num(y)
    v1 <- x[!is.na(x)]
    v2 <- y[!is.na(y)]
  } else {
    g <- num(g)
    ok <- !is.na(x) & !is.na(g)
    x <- x[ok]
    g <- g[ok]
    lv <- sort(unique(g))
    if (length(lv) != 2) stop("more than 2 groups found, only 2 allowed")
    v1 <- x[g == lv[1]]
    v2 <- x[g == lv[2]]
  }
  a <- one(v1)
  b <- one(v2)
  all <- one(c(v1, v2))
  diff <- a$mu - b$mu
  if (equal) {
    df <- a$N + b$N - 2
    se <- sqrt(((a$N - 1) * a$sd^2 + (b$N - 1) * b$sd^2) / df * (1 / a$N + 1 / b$N))
  } else {
    s1 <- a$sd^2 / a$N
    s2 <- b$sd^2 / b$N
    se <- sqrt(s1 + s2)
    df <- (s1 + s2)^2 / (s1^2 / (a$N - 1) + s2^2 / (b$N - 1))
  }
  h <- stats::qt(q, df) * se
  c(list(ub_diff = diff + h, lb_diff = diff - h, mu_diff = diff,
         ub_combined = all$ub, lb_combined = all$lb, se_combined = all$se, mu_combined = all$mu,
         N_combined = all$N, ub_2 = b$ub, lb_2 = b$lb, se_2 = b$se, ub_1 = a$ub, lb_1 = a$lb, se_1 = a$se,
         level = level, sd = all$sd, sd_2 = b$sd, sd_1 = a$sd, se = se),
    tests(diff / se, df), list(mu_2 = b$mu, N_2 = b$N, mu_1 = a$mu, N_1 = a$N))
}"#;

/// The arguments of `stata_ereturn` every model passes: its outcome, what
/// Stata calls its variance (`ols`, `oim`, `robust`, `cluster`, ...), the
/// cluster variable and the weight.
pub(super) fn e_args(
    depvar: &str,
    vce: &str,
    cluster: Option<&str>,
    w: &Option<(super::weights::W, String)>,
) -> String {
    use super::weights::W;
    let mut s = format!(", depvar = {}, vce = {}", r_text(depvar), r_text(vce));
    if let Some(v) = cluster {
        s.push_str(&format!(", clustvar = {}", r_text(v)));
    }
    if let Some((k, v)) = w {
        let word = match k {
            W::F => "fweight",
            W::A => "aweight",
            W::P => "pweight",
            W::I => "iweight",
        };
        s.push_str(&format!(
            ", wtype = {}, wexp = {}",
            r_text(word),
            r_text(&format!("= {v}"))
        ));
    }
    s
}

/// A level as a fraction (0.95) as Stata's r(level) holds it (95).
pub(super) fn percent(level: f64) -> f64 {
    (level * 100.0 * 1e9).round() / 1e9
}

/// A call that prints, run as `quietly` asks: nothing shown, but what it
/// leaves (r()) still left.
pub(super) fn quietly(c: &Command, call: String) -> String {
    if super::silent(c) {
        format!("invisible(utils::capture.output({call}))")
    } else {
        call
    }
}

impl Session {
    /// `eststo name`: the active results stored, with `e(_estimates_name)`.
    pub(super) fn eststo_store(&mut self, name: &str, out: &mut Out) -> Result<(), Untranslated> {
        if !super::is_name(name) {
            return Err(no(format!("(Stata: {name} invalid name)")));
        }
        if !self.eststo_names.iter().any(|n| n == name) {
            self.eststo_names.push(name.to_string());
        }
        self.store_fit(name, "eststo", out)
    }

    /// `eststo [name] [, title()]`, `eststo clear`, `eststo drop names`
    /// (the SSC estout package).
    pub(super) fn eststo(
        &mut self,
        c: &Command,
        opts: &super::Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[("title", 2)]) {
            return Err(no(format!(
                "eststo with the option {o} is not translated yet"
            )));
        }
        let (sub, rest) = super::first_word(&c.parts.main);
        match sub {
            "clear" => {
                self.stored_fits.clear();
                self.eststo_names.clear();
                self.active_store = None;
                out.line("stata_stored <- list()");
                Ok(())
            }
            "drop" => {
                let names: Vec<&str> = rest.split_whitespace().collect();
                self.drop_fits(&names, out)
            }
            "" => {
                let name = format!("est{}", self.stored_fits.len() + 1);
                self.eststo_store(&name, out)
            }
            name if rest.is_empty() => self.eststo_store(name, out),
            _ => Err(no("eststo takes one name")),
        }
    }

    /// `estadd scalar name = exp [, replace]`, `estadd local name text`: a
    /// value added to e(), and to the stored copy when the active results are
    /// stored (as Stata's estout package does).
    pub(super) fn estadd(
        &mut self,
        c: &Command,
        opts: &super::Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[("replace", 3), ("quietly", 1)]) {
            return Err(no(format!(
                "estadd with the option {o} is not translated yet"
            )));
        }
        if matches!(self.e_ret, Returned::Nothing) {
            return Err(no(
                "estadd: no model is fitted before it in this do-file (Stata: last estimation results not found)",
            ));
        }
        let (sub, rest) = super::first_word(&c.parts.main);
        let (name, value, text) = match sub {
            "scalar" => {
                let (n, exp) = rest
                    .split_once('=')
                    .map(|(n, e)| (n.trim(), e.trim()))
                    .or_else(|| super::first_word(rest).into())
                    .ok_or_else(|| no("estadd scalar needs name = exp"))?;
                let n = n.trim_start_matches("e(").trim_end_matches(')');
                if !super::is_name(n) || exp.is_empty() {
                    return Err(no("estadd scalar needs name = exp"));
                }
                let (v, kind) = self.value(exp, out)?;
                if kind == Kind::Text {
                    return Err(no(
                        "estadd scalar of a string: Stata refuses this (type mismatch)",
                    ));
                }
                (n.to_string(), v, false)
            }
            "local" => {
                let (n, text) = super::first_word(rest);
                if !super::is_name(n) {
                    return Err(no("estadd local needs a name"));
                }
                let t = text.trim();
                let t = t
                    .strip_prefix("`\"")
                    .and_then(|s| s.strip_suffix("\"'"))
                    .or_else(|| t.strip_prefix('"').and_then(|s| s.strip_suffix('"')))
                    .unwrap_or(t);
                (n.to_string(), r_text(t), true)
            }
            other => return Err(no(format!("estadd {other} is not translated yet"))),
        };
        if text {
            self.e_text.insert(name.clone());
        } else {
            self.e_text.remove(&name);
        }
        let key = r_text(&name);
        out.line(format!("stata_e[[{key}]] <- {value}"));
        if let Some(s) = &self.active_store {
            out.line(format!(
                "if (exists(\"stata_stored\") && !is.null(stata_stored[[{}]])) stata_stored[[{}]]$stata_e[[{key}]] <- stata_e[[{key}]]",
                r_text(s),
                r_text(s)
            ));
        }
        Ok(())
    }
}

impl Session {
    /// The start of a program the do-file calls: an r-class one collects
    /// what it returns in a list of its own.
    pub(super) fn program_begin(&mut self, c: &Command, out: &mut Out) -> Result<(), Untranslated> {
        let mut w = c.parts.main.split_whitespace();
        let name = w.next().unwrap_or("").to_string();
        let class = w.next().unwrap_or("").to_string();
        if class == "rclass" {
            out.line(format!("stata_rnew_{} <- list()", self.programs.len() + 1));
        }
        self.programs.push(ProgramRun {
            name,
            class,
            ..ProgramRun::default()
        });
        Ok(())
    }

    /// The end of a program: an r-class one's r() is what it returned.
    pub(super) fn program_end(&mut self, c: &Command, out: &mut Out) -> Result<(), Untranslated> {
        let depth = self.programs.len();
        // the temporary variables it made are dropped, as Stata drops them
        let temps: Vec<String> = c
            .parts
            .main
            .split_whitespace()
            .skip(2)
            .map(str::to_string)
            .collect();
        if !temps.is_empty() {
            if let Some(d) = self.data.clone() {
                out.line(format!(
                    "{d} <- {d}[setdiff(names({d}), c({}))]",
                    temps
                        .iter()
                        .map(|t| r_text(t))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
                for t in &temps {
                    self.kinds.remove(t);
                }
            }
        }
        let Some(run) = self.programs.pop() else {
            return Ok(());
        };
        // a line in it not translated: what it returned is not known
        if run.refused {
            if let Some(outer) = self.programs.last_mut() {
                outer.refused = true;
            }
            if run.class == "rclass" {
                self.r_ret = Returned::Lost(format!(
                    "a line of the program {} was not translated, so what it returns is not known",
                    run.name
                ));
            }
            if run.class == "eclass" {
                self.e_ret = Returned::Lost(format!(
                    "a line of the program {} was not translated, so the e() it sets is not known",
                    run.name
                ));
            }
            return Ok(());
        }
        let ProgramRun {
            name, class, texts, ..
        } = run;
        if class == "rclass" {
            self.helpers.insert("stata_returned");
            out.line(format!("stata_r <- stata_rnew_{depth}"));
            out.line(format!("rm(stata_rnew_{depth})"));
            self.r_ret = Returned::Program { cmd: name, texts };
        }
        Ok(())
    }

    /// `return scalar name = exp`, `return local name text` (or `= exp`) and
    /// `return clear`, in an r-class program.
    pub(super) fn program_return(
        &mut self,
        c: &Command,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let depth = self.programs.len();
        let rclass = self.programs.last().is_some_and(|p| p.class == "rclass");
        if !rclass {
            return Err(no(
                "return outside an r-class program (Stata refuses it: the program needs rclass)",
            ));
        }
        let (sub, rest) = super::first_word(&c.parts.main);
        let list = format!("stata_rnew_{depth}");
        match sub {
            "clear" => {
                out.line(format!("{list} <- list()"));
                if let Some(p) = self.programs.last_mut() {
                    p.names.clear();
                    p.texts.clear();
                }
                Ok(())
            }
            "scalar" | "local" => {
                // name = exp, or (local) name followed by its text
                let (name, exp) = match rest.split_once('=') {
                    Some((n, e)) if super::is_name(n.trim()) => {
                        (n.trim().to_string(), Some(e.trim()))
                    }
                    _ => {
                        let (n, _) = super::first_word(rest);
                        (n.to_string(), None)
                    }
                };
                if !super::is_name(&name) {
                    return Err(no(format!("return {sub} needs a name")));
                }
                let (value, text) = match exp {
                    Some(e) => {
                        let (v, kind) = self.value(e, out)?;
                        if sub == "scalar" && kind == Kind::Text {
                            return Err(no(
                                "return scalar of text: Stata refuses it (type mismatch)",
                            ));
                        }
                        (v, kind == Kind::Text)
                    }
                    None if sub == "local" => {
                        let (_, t) = super::first_word(rest);
                        // the text as written, its outer quotes off
                        let t = t.trim();
                        let t = t
                            .strip_prefix("`\"")
                            .and_then(|s| s.strip_suffix("\"'"))
                            .or_else(|| t.strip_prefix('"').and_then(|s| s.strip_suffix('"')))
                            .unwrap_or(t);
                        (r_text(t), true)
                    }
                    None => return Err(no("return scalar needs name = exp")),
                };
                out.line(format!("{list}[[{}]] <- {value}", r_text(&name)));
                if let Some(p) = self.programs.last_mut() {
                    p.names.retain(|n| n != &name);
                    p.texts.retain(|n| n != &name);
                    p.names.push(name.clone());
                    if text {
                        p.texts.push(name);
                    }
                }
                Ok(())
            }
            other => Err(no(format!("return {other} is not translated yet"))),
        }
    }
}

/// A program being run: its name and class, the r() names an r-class one
/// has returned so far (all, and those that are text), and whether a line
/// in it was not translated.
#[derive(Debug, Clone, Default)]
pub(super) struct ProgramRun {
    pub(super) name: String,
    pub(super) class: String,
    pub(super) names: Vec<String>,
    pub(super) texts: Vec<String>,
    pub(super) refused: bool,
}

/// A date or time display format Stata takes (`%td`, `%-tcHH:MM:SS.sss`,
/// `%tmCCYY!mnn`, the old `%d`): its codes, `_ + !c` and `. , : - / \` and
/// space; no width. `%tb` (business calendars) is not translated.
pub(super) fn valid_date_format(f: &str) -> bool {
    const CODES: &[&str] = &[
        "DAYNAME", "Dayname", "Month", "month", "a.m.", "A.M.", ".sss", ".ss", ".s", "JJJ", "jjj",
        "Mon", "mon", "Day", "day", "Da", "da", "CC", "cc", "YY", "yy", "NN", "nn", "DD", "dd",
        "WW", "ww", "HH", "hH", "Hh", "hh", "MM", "mm", "SS", "ss", "am", "AM", "h", "q",
    ];
    let Some(body) = f.strip_prefix('%') else {
        return false;
    };
    let body = body.strip_prefix('-').unwrap_or(body);
    let details = if let Some(rest) = body.strip_prefix('t') {
        let mut cs = rest.chars();
        match cs.next() {
            Some('c' | 'C' | 'd' | 'w' | 'm' | 'q' | 'h' | 'y') => cs.as_str(),
            _ => return false,
        }
    } else if let Some(rest) = body.strip_prefix('d') {
        rest
    } else {
        return false;
    };
    let mut rest = details;
    while let Some(c) = rest.chars().next() {
        if let Some(code) = CODES.iter().find(|code| rest.starts_with(*code)) {
            rest = rest.get(code.len()..).unwrap_or("");
            continue;
        }
        rest = rest.get(c.len_utf8()..).unwrap_or("");
        match c {
            '+' | '_' | '.' | ',' | ':' | '-' | '/' | '\\' | ' ' => {}
            '!' => match rest.chars().next() {
                Some(l) => rest = rest.get(l.len_utf8()..).unwrap_or(""),
                None => return false,
            },
            _ => return false,
        }
    }
    true
}
