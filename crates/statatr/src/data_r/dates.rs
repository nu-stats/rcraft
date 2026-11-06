//! P2C.14, Stata's date and time functions on R, for whole columns.
//!
//! The rules are those of `values/datetime` (StatCraft's decoding, checked
//! there against probes 033 to 035 of Stata 19), written again in R so they
//! work on the data where it lives:
//! - dates count days from 01jan1960;
//! - datetime/c counts milliseconds, 86,400 seconds a day;
//! - weeks, months, quarters and half-years count periods from 1960, 52
//!   weeks a year;
//! - anything outside 0100 to 9999, or not a whole number where one is
//!   needed, is missing.
//!
//! Strings are read against Stata's masks (`DMY`, `YMDhms`, `20YMD`, `#` to
//! skip): punctuation separates, letters and digits are split, and a run of
//! digits longer than its element is cut to fit. Each distinct string is
//! read once. The leap-second family (`Clock()`, `Cofd()`, `hhC()`, ...) is
//! not translated here.

/// The R side: `stata_dt_*`, vectorised over the rows.
pub(super) const DATES_HELPER: &str = r##"stata_dt_n <- function(...) max(lengths(list(...)))
stata_dt_int <- function(v) !is.na(v) & is.finite(v) & v == trunc(v) & abs(v) < 1e15
stata_dt_flr <- function(v) !is.na(v) & is.finite(v) & abs(v) < 1e15
stata_dt_leap <- function(y) (y %% 4 == 0 & y %% 100 != 0) | y %% 400 == 0
stata_dt_dim <- function(y, m) {
  out <- rep(0, length(m))
  ok <- !is.na(m) & m >= 1 & m <= 12
  out[ok] <- c(31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31)[m[ok]] + (m[ok] == 2 & stata_dt_leap(y[ok]))
  out
}
# day number (01jan1960 = 0) of a valid date, and back (Howard Hinnant's)
stata_dt_dn <- function(y, m, d) {
  y <- y - (m <= 2)
  era <- floor(y / 400)
  yoe <- y - era * 400
  mp <- (m + 9) %% 12
  doy <- floor((153 * mp + 2) / 5) + d - 1
  doe <- yoe * 365 + floor(yoe / 4) - floor(yoe / 100) + doy
  era * 146097 + doe - 719468 + 3653
}
stata_dt_civil <- function(day) {
  z <- day - 3653 + 719468
  era <- floor(z / 146097)
  doe <- z - era * 146097
  yoe <- floor((doe - floor(doe / 1460) + floor(doe / 36524) - floor(doe / 146096)) / 365)
  doy <- doe - (365 * yoe + floor(yoe / 4) - floor(yoe / 100))
  mp <- floor((5 * doy + 2) / 153)
  d <- doy - floor((153 * mp + 2) / 5) + 1
  m <- ifelse(mp < 10, mp + 3, mp - 9)
  list(y = yoe + era * 400 + (m <= 2), m = m, d = d)
}
# f(day, civil) for the days Stata can show (0100 to 9999), else missing
stata_dt_on <- function(d, f) {
  day <- floor(d)
  ok <- !is.na(d) & is.finite(d) & day >= -679350 & day <= 2936549
  out <- rep(NA_real_, length(d))
  if (any(ok)) out[ok] <- f(day[ok], stata_dt_civil(day[ok]))
  out
}
stata_mdy <- function(m, d, y) {
  n <- stata_dt_n(m, d, y)
  m <- rep_len(as.numeric(m), n); d <- rep_len(as.numeric(d), n); y <- rep_len(as.numeric(y), n)
  ok <- stata_dt_int(m) & stata_dt_int(d) & stata_dt_int(y)
  ok[ok] <- y[ok] >= 100 & y[ok] <= 9999 & m[ok] >= 1 & m[ok] <= 12 & d[ok] >= 1 & d[ok] <= stata_dt_dim(y[ok], m[ok])
  out <- rep(NA_real_, n)
  out[ok] <- stata_dt_dn(y[ok], m[ok], d[ok])
  out
}
stata_isleapyear <- function(y) {
  ok <- stata_dt_int(y) & y >= 100 & y <= 9999
  ifelse(ok, as.numeric(stata_dt_leap(y)), NA_real_)
}
stata_year <- function(d) stata_dt_on(d, function(day, c) c$y)
stata_month <- function(d) stata_dt_on(d, function(day, c) c$m)
stata_day <- function(d) stata_dt_on(d, function(day, c) c$d)
# 0 is Sunday; Stata also answers for 01jan10000
stata_dow <- function(d) {
  out <- stata_dt_on(d, function(day, c) (day + 5) %% 7)
  out[!is.na(d) & floor(d) == 2936550] <- (2936550 + 5) %% 7
  out
}
stata_doy <- function(d) stata_dt_on(d, function(day, c) day - stata_dt_dn(c$y, 1, 1) + 1)
# weeks run from 1 January in blocks of 7 days; week 52 takes the rest
stata_dt_week <- function(day, y) pmin(floor((day - stata_dt_dn(y, 1, 1)) / 7) + 1, 52)
stata_week <- function(d) stata_dt_on(d, function(day, c) stata_dt_week(day, c$y))
stata_quarter <- function(d) stata_dt_on(d, function(day, c) floor((c$m - 1) / 3) + 1)
stata_halfyear <- function(d) stata_dt_on(d, function(day, c) floor((c$m - 1) / 6) + 1)
stata_wofd <- function(d) stata_dt_on(d, function(day, c) (c$y - 1960) * 52 + stata_dt_week(day, c$y) - 1)
stata_mofd <- function(d) stata_dt_on(d, function(day, c) (c$y - 1960) * 12 + c$m - 1)
stata_qofd <- function(d) stata_dt_on(d, function(day, c) (c$y - 1960) * 4 + floor((c$m - 1) / 3))
stata_hofd <- function(d) stata_dt_on(d, function(day, c) (c$y - 1960) * 2 + floor((c$m - 1) / 6))
stata_yofd <- function(d) stata_dt_on(d, function(day, c) c$y)
# the first day of period p: per periods a year, each months long
stata_dt_first <- function(p, per, months) {
  ok <- stata_dt_flr(p)
  out <- rep(NA_real_, length(p))
  q <- floor(p[ok])
  y <- 1960 + floor(q / per)
  k <- q %% per
  good <- y >= 100 & y <= 9999
  v <- rep(NA_real_, length(q))
  v[good] <- stata_dt_dn(y[good], k[good] * months + 1, 1)
  out[ok] <- v
  out
}
stata_dofw <- function(w) {
  ok <- stata_dt_flr(w)
  out <- rep(NA_real_, length(w))
  q <- floor(w[ok])
  y <- 1960 + floor(q / 52)
  good <- y >= 100 & y <= 9999
  v <- rep(NA_real_, length(q))
  v[good] <- stata_dt_dn(y[good], 1, 1) + 7 * (q[good] %% 52)
  out[ok] <- v
  out
}
stata_dofm <- function(m) stata_dt_first(m, 12, 1)
stata_dofq <- function(q) stata_dt_first(q, 4, 3)
stata_dofh <- function(h) stata_dt_first(h, 2, 6)
stata_dofy <- function(y) {
  ok <- stata_dt_flr(y)
  ok[ok] <- floor(y[ok]) >= 100 & floor(y[ok]) <= 9999
  out <- rep(NA_real_, length(y))
  out[ok] <- stata_dt_dn(floor(y[ok]), 1, 1)
  out
}
# yw(), ym(), yq(), yh(): the year 100 is refused here, though mdy() takes it
stata_dt_period <- function(y, k, per) {
  n <- stata_dt_n(y, k)
  y <- rep_len(as.numeric(y), n); k <- rep_len(as.numeric(k), n)
  ok <- stata_dt_int(y) & stata_dt_int(k)
  ok[ok] <- y[ok] >= 101 & y[ok] <= 9999 & k[ok] >= 1 & k[ok] <= per
  ifelse(ok, (y - 1960) * per + k - 1, NA_real_)
}
stata_yw <- function(y, w) stata_dt_period(y, w, 52)
stata_ym <- function(y, m) stata_dt_period(y, m, 12)
stata_yq <- function(y, q) stata_dt_period(y, q, 4)
stata_yh <- function(y, h) stata_dt_period(y, h, 2)
# times: datetime/c in milliseconds
stata_cofd <- function(d) {
  ok <- !is.na(d) & is.finite(d) & floor(d) >= -679350 & floor(d) <= 2936549
  ifelse(ok, d * 86400000, NA_real_)
}
stata_dofc <- function(c) ifelse(!is.na(c) & is.finite(c), floor(c / 86400000), NA_real_)
stata_dt_msday <- function(c) {
  c <- floor(c)
  c - floor(c / 86400000) * 86400000
}
stata_hh <- function(c) floor(stata_dt_msday(c) / 3600000)
stata_mm <- function(c) floor((stata_dt_msday(c) %% 3600000) / 60000)
stata_ss <- function(c) (stata_dt_msday(c) %% 60000) / 1000
stata_hours <- function(ms) ms / 3600000
stata_minutes <- function(ms) ms / 60000
stata_seconds <- function(ms) ms / 1000
stata_msofhours <- function(h) h * 3600000
stata_msofminutes <- function(m) m * 60000
stata_msofseconds <- function(s) s * 1000
stata_dt_time <- function(h, m, s) {
  ok <- !is.na(h) & !is.na(m) & !is.na(s) & is.finite(h) & is.finite(m) & is.finite(s)
  ok[ok] <- h[ok] >= 0 & h[ok] < 24 & m[ok] >= 0 & m[ok] < 60 & s[ok] >= 0 & s[ok] < 60
  ifelse(ok, (floor(h) * 3600 + floor(m) * 60) * 1000 + s * 1000, NA_real_)
}
stata_hms <- function(h, m, s) {
  n <- stata_dt_n(h, m, s)
  stata_dt_time(rep_len(as.numeric(h), n), rep_len(as.numeric(m), n), rep_len(as.numeric(s), n))
}
stata_dhms <- function(d, h, m, s) {
  n <- stata_dt_n(d, h, m, s)
  d <- rep_len(as.numeric(d), n)
  t <- stata_hms(rep_len(h, n), rep_len(m, n), rep_len(s, n))
  ifelse(stata_dt_flr(d), floor(d) * 86400000 + t, NA_real_)
}
stata_mdyhms <- function(mo, d, y, h, mi, s) {
  n <- stata_dt_n(mo, d, y, h, mi, s)
  day <- stata_mdy(rep_len(mo, n), rep_len(d, n), rep_len(y, n))
  day * 86400000 + stata_hms(rep_len(h, n), rep_len(mi, n), rep_len(s, n))
}
# strings against a mask
stata_dt_mask <- function(mask, period) {
  parts <- list()
  century <- ""
  for (ch in strsplit(mask, "", fixed = TRUE)[[1]]) {
    if (grepl("^[0-9]$", ch)) {
      century <- paste0(century, ch)
      next
    }
    if (ch == " ") next
    if (ch == "Y") {
      parts[[length(parts) + 1]] <- list(p = "Y", century = if (nzchar(century)) as.numeric(century) else NA_real_)
      century <- ""
      next
    }
    p <- switch(ch, D = "D", M = "M", h = "h", m = "m", s = "s", "#" = "#",
                Q = if (identical(period, "quarter")) "Q",
                W = if (identical(period, "week")) "W",
                H = if (identical(period, "half")) "H",
                NULL)
    if (is.null(p) || nzchar(century)) return(NULL)
    parts[[length(parts) + 1]] <- list(p = p, century = NA_real_)
  }
  if (nzchar(century)) NULL else parts
}
stata_dt_pieces <- function(text) {
  ch <- strsplit(text, "", fixed = TRUE)[[1]]
  n <- length(ch)
  dig <- grepl("^[0-9]$", ch)
  alpha <- grepl("^[[:alpha:]]$", ch, perl = TRUE)
  out <- list()
  cur <- character(0)
  flush <- function() {
    if (length(cur)) {
      s <- paste(cur, collapse = "")
      out[[length(out) + 1]] <<- if (all(grepl("^[0-9]$", cur))) list(t = "N", v = s) else list(t = "W", v = tolower(s))
      cur <<- character(0)
    }
  }
  for (i in seq_len(n)) {
    if (dig[i] || alpha[i]) {
      if (length(cur) && grepl("^[0-9]$", cur[length(cur)]) != dig[i]) flush()
      cur <- c(cur, ch[i])
    } else {
      flush()
      if (ch[i] == "." && i > 1 && i < n && dig[i - 1] && dig[i + 1]) out[[length(out) + 1]] <- list(t = "P", v = ".")
    }
  }
  flush()
  out
}
stata_dt_months <- c("january", "february", "march", "april", "may", "june", "july", "august",
                     "september", "october", "november", "december")
stata_dt_match <- function(text, mask, period, split) {
  parts <- stata_dt_mask(mask, period)
  if (is.null(parts)) return(NULL)
  q <- stata_dt_pieces(text)
  if (!is.null(period)) {
    wanted <- switch(period, year = NA, month = "M", quarter = "Q", half = "H", week = "W")
    ps <- vapply(parts, function(x) x$p, "")
    years <- sum(ps == "Y")
    own <- if (is.na(wanted)) 0 else sum(ps == wanted)
    if (years != 1 || own != !is.na(wanted) || length(ps) != years + own) return(NULL)
    # in 2006m1 or 2006q3 the letter only separates the numbers
    q <- Filter(function(x) !(x$t == "W" && x$v %in% c("m", "q", "h", "w")), q)
  }
  i <- 1
  front <- function() if (i <= length(q)) q[[i]] else NULL
  take <- function(width, split) {
    f <- front()
    if (is.null(f) || f$t != "N") return(NULL)
    if (split && nchar(f$v) > width) {
      q[[i]] <<- list(t = "N", v = substr(f$v, width + 1, nchar(f$v)))
      return(substr(f$v, 1, width))
    }
    i <<- i + 1
    f$v
  }
  found <- list()
  for (k in seq_along(parts)) {
    part <- parts[[k]]
    while (!is.null(front()) && front()$t == "P") i <- i + 1
    if (part$p == "#") {
      if (k == length(parts)) i <- length(q) + 1 else if (is.null(front())) return(NULL) else i <- i + 1
      next
    }
    if (part$p == "M" && !is.null(front()) && front()$t == "W") {
      w <- front()$v
      m <- if (nchar(w) >= 3) which(startsWith(stata_dt_months, w))[1] else NA
      if (is.na(m)) return(NULL)
      found$month <- m
      i <- i + 1
      next
    }
    nl <- if (!is.null(front()) && front()$t == "N") nchar(front()$v) else 0
    width <- if (part$p == "Y") {
      if (!is.na(part$century)) (if (nl == 4) 4 else 2) else 4
    } else 2
    digits <- take(width, split)
    if (is.null(digits)) return(NULL)
    if (part$p == "Y" && !is.na(part$century) && !(nchar(digits) %in% c(2, 4))) return(NULL)
    value <- as.numeric(digits)
    switch(part$p,
      D = found$day <- value,
      M = found$month <- value,
      Y = found$year <- c(value, nchar(digits), part$century),
      h = found$hour <- value,
      m = found$minute <- value,
      s = {
        sec <- value
        if (!is.null(front()) && front()$t == "P") {
          i <- i + 1
          frac <- take(Inf, FALSE)
          if (is.null(frac)) return(NULL)
          sec <- sec + as.numeric(paste0("0.", frac))
        }
        found$second <- sec
      },
      found$sub <- value)
    if (part$p %in% c("h", "m", "s")) {
      f <- front()
      if (!is.null(f) && f$t == "W" && f$v %in% c("am", "pm")) {
        found$pm <- f$v == "pm"
        i <- i + 1
      } else if (!is.null(f) && f$t == "W" && f$v %in% c("a", "p") && i + 1 <= length(q) &&
                 q[[i + 1]]$t == "W" && q[[i + 1]]$v == "m") {
        found$pm <- f$v == "p"
        i <- i + 2
      }
    }
  }
  rest <- if (i <= length(q)) q[i:length(q)] else list()
  if (length(Filter(function(x) x$t != "P", rest))) return(NULL)
  found
}
stata_dt_fullyear <- function(found, top) {
  if (is.null(found$year)) return(1960)
  value <- found$year[1]; digits <- found$year[2]; century <- found$year[3]
  y <- if (!is.na(century) && digits <= 2) {
    century * 100 + value
  } else if (!is.na(century)) {
    value
  } else if (!is.na(top) && digits <= 2) {
    t <- floor(top)
    y <- t - ((t - value) %% 100)
    if (y > t) y - 100 else y
  } else {
    value
  }
  if (y >= 100 && y <= 9999) y else NULL
}
stata_dt_dayof <- function(found, top) {
  y <- stata_dt_fullyear(found, top)
  if (is.null(y)) return(NULL)
  m <- if (is.null(found$month)) 1 else found$month
  d <- if (is.null(found$day)) 1 else found$day
  if (m < 1 || m > 12 || d < 1 || d > stata_dt_dim(y, m)) return(NULL)
  stata_dt_dn(y, m, d)
}
stata_dt_timeof <- function(found) {
  h <- if (is.null(found$hour)) 0 else found$hour
  mi <- if (is.null(found$minute)) 0 else found$minute
  s <- if (is.null(found$second)) 0 else found$second
  if (!is.null(found$pm)) {
    if (h < 1 || h > 12) return(NULL)
    h <- if (h == 12) (if (found$pm) 12 else 0) else if (found$pm) h + 12 else h
  }
  if (h < 0 || h >= 24 || mi < 0 || mi >= 60 || s < 0 || s >= 61) return(NULL)
  c(((h * 60 + mi) * 60) * 1000, s)
}
stata_dt_one <- function(text, mask, kind, top) {
  if (is.na(text) || is.na(mask)) return(NA_real_)
  period <- if (kind %in% c("date", "clock")) NULL else kind
  found <- stata_dt_match(text, mask, period, is.null(period))
  if (is.null(found)) return(NA_real_)
  if (kind == "date") {
    t <- stata_dt_timeof(found)
    if (is.null(t) || t[2] >= 60) return(NA_real_)
    d <- stata_dt_dayof(found, top)
    return(if (is.null(d)) NA_real_ else d)
  }
  if (kind == "clock") {
    d <- stata_dt_dayof(found, top)
    t <- stata_dt_timeof(found)
    # 23:59:60 is a leap second, which only datetime/C has
    if (is.null(d) || is.null(t) || t[2] >= 60) return(NA_real_)
    return(d * 86400000 + t[1] + t[2] * 1000)
  }
  y <- stata_dt_fullyear(found, top)
  if (is.null(y)) return(NA_real_)
  k <- if (is.null(found$sub)) 1 else found$sub
  switch(kind,
    year = y,
    month = {
      m <- if (is.null(found$month)) 1 else found$month
      if (m >= 1 && m <= 12) (y - 1960) * 12 + m - 1 else NA_real_
    },
    quarter = if (k >= 1 && k <= 4) (y - 1960) * 4 + k - 1 else NA_real_,
    half = if (k >= 1 && k <= 2) (y - 1960) * 2 + k - 1 else NA_real_,
    week = if (k >= 1 && k <= 52) (y - 1960) * 52 + k - 1 else NA_real_)
}
# each distinct string, mask and topyear is read once
stata_dt_read <- function(s, mask, kind, top = NA_real_) {
  n <- stata_dt_n(s, mask, top)
  s <- rep_len(as.character(s), n); mask <- rep_len(as.character(mask), n); top <- rep_len(as.numeric(top), n)
  key <- paste(s, mask, top, sep = "\r")
  first <- !duplicated(key)
  vals <- mapply(stata_dt_one, s[first], mask[first], MoreArgs = list(kind = kind), top[first], USE.NAMES = FALSE)
  as.numeric(vals[match(key, key[first])])
}
stata_date <- function(s, mask, top = NA_real_) stata_dt_read(s, mask, "date", top)
stata_clock <- function(s, mask, top = NA_real_) stata_dt_read(s, mask, "clock", top)
stata_weekly <- function(s, mask, top = NA_real_) stata_dt_read(s, mask, "week", top)
stata_monthly <- function(s, mask, top = NA_real_) stata_dt_read(s, mask, "month", top)
stata_quarterly <- function(s, mask, top = NA_real_) stata_dt_read(s, mask, "quarter", top)
stata_halfyearly <- function(s, mask, top = NA_real_) stata_dt_read(s, mask, "half", top)
stata_yearly <- function(s, mask, top = NA_real_) stata_dt_read(s, mask, "year", top)
# datetime/C: the 27 IERS leap seconds, each added at the end of these days
# (StatCraft's decoding, probe 034)
stata_dt_leapdays <- function() {
  ymd <- matrix(c(1972,6,30, 1972,12,31, 1973,12,31, 1974,12,31, 1975,12,31, 1976,12,31, 1977,12,31,
                  1978,12,31, 1979,12,31, 1981,6,30, 1982,6,30, 1983,6,30, 1985,6,30, 1987,12,31,
                  1989,12,31, 1990,12,31, 1992,6,30, 1993,6,30, 1994,6,30, 1995,12,31, 1997,6,30,
                  1998,12,31, 2005,12,31, 2008,12,31, 2012,6,30, 2015,6,30, 2016,12,31), ncol = 3, byrow = TRUE)
  stata_dt_dn(ymd[, 1], ymd[, 2], ymd[, 3])
}
stata_dt_midnights <- function() (stata_dt_leapdays() + 1) * 86400000
stata_Cofc <- function(c) {
  c <- floor(c)
  out <- c + 1000 * findInterval(c, stata_dt_midnights())
  out[is.na(c) | !is.finite(c)] <- NA_real_
  out
}
# the clock's time for a datetime/C value, and whether it is inside a leap second
stata_dt_wall <- function(big) {
  mids <- stata_dt_midnights()
  one <- function(b) {
    if (is.na(b) || !is.finite(b)) return(c(NA_real_, 0))
    ins <- 0
    for (m in mids) {
      st <- m + 1000 * ins
      if (b < st) break
      if (b < st + 1000) return(c(m - 1000 + (b - st), 1))
      ins <- ins + 1
    }
    c(b - 1000 * ins, 0)
  }
  r <- vapply(as.numeric(big), one, numeric(2))
  list(wall = r[1, ], leap = r[2, ] == 1)
}
stata_cofC <- function(big) {
  mids <- stata_dt_midnights()
  one <- function(b) {
    if (is.na(b) || !is.finite(b)) return(NA_real_)
    b <- floor(b)
    ins <- 0
    for (m in mids) {
      st <- m + 1000 * ins
      if (b < st) break
      if (b < st + 1000) return(m)
      ins <- ins + 1
    }
    b - 1000 * ins
  }
  vapply(as.numeric(big), one, 0)
}
stata_Cofd <- function(d) stata_Cofc(stata_cofd(d))
# the day the clock shows: a leap second is still its own day (Stata 19)
stata_dofC <- function(big) stata_dofc(stata_dt_wall(big)$wall)
stata_hhC <- function(big) stata_hh(stata_dt_wall(big)$wall)
stata_mmC <- function(big) stata_mm(stata_dt_wall(big)$wall)
stata_ssC <- function(big) { w <- stata_dt_wall(big); stata_ss(w$wall) + ifelse(w$leap, 1, 0) }
stata_Cdhms <- function(d, h, m, s) stata_Cofc(stata_dhms(d, h, m, s))
stata_Chms <- function(h, m, s) stata_Cofc(stata_hms(h, m, s))
stata_Cmdyhms <- function(mo, d, y, h, mi, s) {
  day <- stata_mdy(mo, d, y)
  out <- stata_Cofc(stata_mdyhms(mo, d, y, h, mi, s))
  # 23:59:60 only on a day a leap second was added
  sec60 <- !is.na(s) & s >= 60
  ok60 <- sec60 & !is.na(day) & h == 23 & mi == 59 & s < 61 & day %in% stata_dt_leapdays()
  out[sec60] <- NA_real_
  out[ok60] <- stata_Cofc(day[ok60] * 86400000 + 86399000) + 1000 + (s[ok60] - 60) * 1000
  out
}
# Clock(): clock() read with leap seconds counted; 23:59:60 is not read yet
stata_Clock <- function(s, mask, top = NA_real_) {
  c <- stata_clock(s, mask, top)
  if (any(is.na(c) & grepl(":60", s, fixed = TRUE))) stop("Clock() of 23:59:60 (a leap second) is not translated yet", call. = FALSE)
  stata_Cofc(c)
}"##;
