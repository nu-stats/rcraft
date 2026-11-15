//! `infix` (column specifications) and `infile` (free format): raw text read
//! into a new dataset as Stata 19 reads it, observed on StatCraft's probe 524
//! and our probe RW (2026-11-15); Stata's code was not read.
//!
//! - A number is Stata's: digits with a point and an exponent, a sign, `.` for
//!   missing; anything else is missing with "'x' cannot be read as a number for
//!   v[#]", # the observation among those kept by `if`.
//! - infix: each field is its columns (1-based, inclusive), blanks trimmed; `str`
//!   alone is as wide as its columns, `str#` cut to #; `# lines` with `#:` taking
//!   a record of several lines; `in` counts records.
//! - infile: values separated by blanks or commas, a quoted string one value,
//!   flowing across lines; `_skip(#)` passes # values; strings cut to their width;
//!   an observation the file ends inside is kept, its last values missing, with
//!   "(eof not at end of obs)". A file with no extension is `.raw`.
//! - Both end with "(# observations read)", counting those `if` kept; new
//!   numeric variables are float unless typed (byte, int, long, double).

use super::*;

pub(super) const RAWREAD_HELPER: &str = r##"stata_rawnum <- function(s) {
  ok <- grepl("^[+-]?([0-9]+[.]?[0-9]*|[.][0-9]+)([eEdD][+-]?[0-9]+)?$", s)
  v <- rep(NA_real_, length(s))
  v[ok] <- as.numeric(sub("[dD]", "e", s[ok]))
  list(v = v, bad = !ok & s != "." & nzchar(s))
}
stata_rawstore <- function(x, type) {
  if (type == "float") stata_float(x) else x
}
stata_rawfinish <- function(cols, specs, msgs, cond, show) {
  d <- as.data.frame(cols, stringsAsFactors = FALSE, optional = TRUE)
  names(d) <- vapply(specs, function(s) s$name, "")
  n <- nrow(d)
  keep <- rep(TRUE, n)
  if (!is.null(cond) && n > 0) {
    k <- eval(cond, d, parent.frame(2))
    keep <- !is.na(k) & as.logical(k)
  }
  if (show && nrow(msgs)) {
    # the observation number among those kept before it, as Stata counts it
    at <- cumsum(c(0, keep))[msgs$obs] + 1
    cat(paste0(sprintf("'%s' cannot be read as a number for %s[%d]", msgs$text, msgs$var, as.integer(at)), "\n"), sep = "")
  }
  d <- d[keep, , drop = FALSE]
  rownames(d) <- NULL
  d
}
stata_infix <- function(path, specs, nlines = 1, first = 1, last = NULL, cond = NULL, show = TRUE) {
  if (!file.exists(path)) stop("file ", path, " not found (Stata: r(601))", call. = FALSE)
  l <- readLines(path, warn = FALSE, encoding = "UTF-8")
  nrec <- ceiling(length(l) / nlines)
  recs <- seq_len(nrec)
  if (is.null(last)) last <- nrec
  recs <- recs[recs >= first & recs <= last]
  msgs <- data.frame(obs = integer(), var = character(), text = character())
  cols <- list()
  for (s in specs) {
    li <- (recs - 1) * nlines + s$line
    txt <- ifelse(li <= length(l), l[pmin(li, length(l))], "")
    f <- trimws(substr(txt, s$from, s$to))
    if (startsWith(s$type, "str")) {
      w <- if (s$type == "str") s$to - s$from + 1 else as.integer(substring(s$type, 4))
      cols[[length(cols) + 1]] <- substr(f, 1, w)
    } else {
      p <- stata_rawnum(f)
      if (any(p$bad)) msgs <- rbind(msgs, data.frame(obs = which(p$bad), var = s$name, text = f[p$bad]))
      cols[[length(cols) + 1]] <- stata_rawstore(p$v, s$type)
    }
  }
  msgs <- msgs[order(msgs$obs, match(msgs$var, vapply(specs, function(s) s$name, ""))), , drop = FALSE]
  d <- stata_rawfinish(cols, specs, msgs, cond, show)
  if (show) cat("(", format(nrow(d), big.mark = ","), " observation", if (nrow(d) == 1) "" else "s", " read)\n", sep = "")
  d
}
stata_infile <- function(path, slots, cond = NULL, show = TRUE) {
  if (!file.exists(path)) stop("file ", path, " not found (Stata: r(601))", call. = FALSE)
  txt <- paste(readLines(path, warn = FALSE, encoding = "UTF-8"), collapse = "\n")
  # the values: quoted strings whole, else runs between blanks and commas
  m <- gregexpr('"[^"]*"|[^[:space:],]+', txt)[[1]]
  tok <- if (m[1] == -1) character() else regmatches(txt, list(m))[[1]]
  tok <- sub('^"(.*)"$', "\\1", tok)
  per <- sum(vapply(slots, function(s) if (is.null(s$skip)) 1 else s$skip, 1))
  nobs <- ceiling(length(tok) / per)
  partial <- length(tok) %% per != 0
  specs <- Filter(function(s) is.null(s$skip), slots)
  vals <- lapply(specs, function(s) rep(if (startsWith(s$type, "str")) "" else NA_character_, nobs))
  names(vals) <- vapply(specs, function(s) s$name, "")
  i <- 0
  for (o in seq_len(nobs)) for (s in slots) {
    if (!is.null(s$skip)) {
      i <- i + s$skip
      next
    }
    i <- i + 1
    if (i <= length(tok)) vals[[s$name]][o] <- tok[i]
  }
  msgs <- data.frame(obs = integer(), var = character(), text = character())
  cols <- list()
  for (s in specs) {
    v <- vals[[s$name]]
    if (startsWith(s$type, "str")) {
      cols[[length(cols) + 1]] <- substr(v, 1, as.integer(substring(s$type, 4)))
    } else {
      p <- stata_rawnum(ifelse(is.na(v), ".", v))
      if (any(p$bad)) msgs <- rbind(msgs, data.frame(obs = which(p$bad), var = s$name, text = v[p$bad]))
      cols[[length(cols) + 1]] <- stata_rawstore(p$v, s$type)
    }
  }
  msgs <- msgs[order(msgs$obs, match(msgs$var, names(vals))), , drop = FALSE]
  d <- stata_rawfinish(cols, specs, msgs, cond, show)
  if (show && partial) cat("(eof not at end of obs)\n")
  if (show) cat("(", format(nrow(d), big.mark = ","), " observation", if (nrow(d) == 1) "" else "s", " read)\n", sep = "")
  d
}"##;

/// Stata's r(18): a reader without clear while data is in memory.
const RAW_NOT_EMPTY: &str = "you must start with an empty dataset (Stata: r(18))";

/// A storage type word: byte, int, long, float, double, str or str#.
fn raw_type(w: &str) -> Option<String> {
    match w {
        "byte" | "int" | "long" | "float" | "double" | "str" => Some(w.to_string()),
        _ => w
            .strip_prefix("str")
            .and_then(|n| n.parse::<u32>().ok())
            .filter(|n| (1..=2045).contains(n))
            .map(|n| format!("str{n}")),
    }
}

impl Session {
    /// Before a reader that replaces the data: Stata's r(18) without clear.
    fn raw_needs_clear(&self, opts: &Opts, name: &str) -> Result<(), Untranslated> {
        if opts.has("clear", 5) {
            return Ok(());
        }
        if self.data.is_some() {
            return Err(no(RAW_NOT_EMPTY));
        }
        if let Some(why) = &self.data_lost {
            return Err(no(format!(
                "{name} without clear: whether data is in memory is not known here ({why})"
            )));
        }
        Ok(())
    }

    /// The new data's state after a reader: its variables and their storage.
    fn raw_new_data(&mut self, name: String, vars: &[(String, String)]) {
        self.data = Some(name);
        self.data_lost = None;
        self.file = None;
        self.kinds.clear();
        self.storage.clear();
        self.fv_base.clear();
        self.forget_ts();
        self.label_sets.clear();
        for (v, t) in vars {
            let (kind, storage) = match t.as_str() {
                "byte" => (Kind::Number, Storage::Byte),
                "int" => (Kind::Number, Storage::Int),
                "long" => (Kind::Number, Storage::Long),
                "double" => (Kind::Number, Storage::Double),
                "float" => (Kind::Number, Storage::Float),
                _ => (Kind::Text, Storage::Str),
            };
            self.kinds.insert(v.clone(), kind);
            self.storage.insert(v.clone(), storage);
        }
    }

    /// The `if` of a reader, as an R expression on the new columns (their kinds
    /// known while it is translated, the old ones back after).
    fn raw_cond(
        &mut self,
        c: &Command,
        out: &mut Out,
        vars: &[(String, String)],
    ) -> Result<String, Untranslated> {
        let Some(e) = c.parts.if_exp.clone() else {
            return Ok(String::new());
        };
        let saved = self.kinds.clone();
        for (v, t) in vars {
            let k = if t.starts_with("str") {
                Kind::Text
            } else {
                Kind::Number
            };
            self.kinds.insert(v.clone(), k);
        }
        let cond = self.expr(&e, Use::Condition, out);
        self.kinds = saved;
        Ok(format!(", cond = quote({})", cond?))
    }

    /// `infix [#] lines] [#:] [type] name #[-#] ... using file [if] [in], clear`.
    pub(super) fn infix(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[("clear", 5)]) {
            return Err(no(format!(
                "infix with the option {o} is not translated yet"
            )));
        }
        let file = c
            .using
            .clone()
            .ok_or_else(|| no("infix with a dictionary file is not translated yet"))?;
        let mut words: Vec<String> = c
            .parts
            .main
            .split_whitespace()
            .map(str::to_string)
            .collect();
        if words.is_empty() {
            return Err(no("infix using a dictionary is not translated yet"));
        }
        let mut nlines = 1u32;
        if words.len() >= 2 && words.get(1).map(String::as_str) == Some("lines") {
            nlines = words
                .first()
                .and_then(|w| w.parse::<u32>().ok())
                .filter(|n| *n >= 1)
                .ok_or_else(|| no("infix: # lines needs a whole number"))?;
            words.drain(..2);
        }
        let mut line = 1u32;
        let mut ty = String::new();
        let mut specs = Vec::new();
        let mut vars = Vec::new();
        let mut it = words.into_iter().peekable();
        while let Some(w) = it.next() {
            if let Some(n) = w.strip_suffix(':').and_then(|n| n.parse::<u32>().ok()) {
                if n < 1 || n > nlines {
                    return Err(no(format!(
                        "infix: line {n}: is not one of the {nlines} lines"
                    )));
                }
                line = n;
                continue;
            }
            if let Some(t) = raw_type(&w) {
                ty = t;
                continue;
            }
            if !is_name(&w) {
                return Err(no(format!("infix: {w} is not translated here")));
            }
            let cols = it
                .next()
                .ok_or_else(|| no(format!("infix: {w} needs its columns")))?;
            let (from, to) = match cols.split_once('-') {
                Some((a, b)) => (a.parse::<u32>().ok(), b.parse::<u32>().ok()),
                None => (cols.parse::<u32>().ok(), cols.parse::<u32>().ok()),
            };
            let (Some(from), Some(to)) = (from, to) else {
                return Err(no(format!("infix: {cols} is not columns")));
            };
            if from < 1 || to < from {
                out.line("stop(\"dictionary invalid (Stata: r(198))\", call. = FALSE)");
                return Ok(());
            }
            let t = if ty.is_empty() {
                "float".to_string()
            } else {
                std::mem::take(&mut ty)
            };
            let stored = if t == "str" {
                format!("str{}", to - from + 1)
            } else {
                t.clone()
            };
            specs.push(format!(
                "list(name = {}, type = {}, line = {line}, from = {from}, to = {to})",
                r_text(&w),
                r_text(&t)
            ));
            vars.push((w, stored));
        }
        if specs.is_empty() {
            return Err(no("infix needs variables and their columns"));
        }
        if let Err(e) = self.raw_needs_clear(opts, "infix") {
            if e.reason == RAW_NOT_EMPTY {
                out.line(format!("stop({}, call. = FALSE)", r_text(RAW_NOT_EMPTY)));
                return Ok(());
            }
            return Err(e);
        }
        let (first, last) = match &c.parts.in_range {
            Some(r) => in_bounds(r)?,
            None => ("1".to_string(), "NULL".to_string()),
        };
        let path = file_name(&file, "raw");
        let name = frame_name(&path);
        let cond = self.raw_cond(c, out, &vars)?;
        self.helpers.insert("stata_float");
        self.helpers.insert("stata_rawread");
        out.line(format!(
            "{name} <- stata_infix({}, list({}), nlines = {nlines}, first = {first}, last = {last}{cond}{})",
            r_text(&path),
            specs.join(", "),
            if self.quiet(c) { ", show = FALSE" } else { "" }
        ));
        self.raw_new_data(name, &vars);
        Ok(())
    }

    /// `infile [type] name ... [_skip(#)] using file [if], clear` (free format).
    pub(super) fn infile(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[("clear", 5)]) {
            return Err(no(format!(
                "infile with the option {o} is not translated yet"
            )));
        }
        if c.parts.in_range.is_some() {
            return Err(no("infile with in is not translated yet"));
        }
        let file = c
            .using
            .clone()
            .ok_or_else(|| no("infile needs using and a file"))?;
        let main = c.parts.main.trim();
        if main.is_empty() {
            return Err(no("infile using a dictionary is not translated yet"));
        }
        let mut ty = String::new();
        let mut slots = Vec::new();
        let mut vars = Vec::new();
        for w in main.split_whitespace() {
            if let Some(n) = w
                .strip_prefix("_skip(")
                .and_then(|r| r.strip_suffix(')'))
                .and_then(|n| n.parse::<u32>().ok())
            {
                slots.push(format!("list(skip = {n})"));
                continue;
            }
            if let Some(t) = raw_type(w) {
                if t == "str" {
                    return Err(no("infile: str needs its width (str#)"));
                }
                ty = t;
                continue;
            }
            let names: Vec<String> = match w.split_once('-') {
                Some((a, b)) => var_range(a, b)
                    .ok_or_else(|| no(format!("infile: {w} is not translated here")))?,
                None if is_name(w) => vec![w.to_string()],
                None => return Err(no(format!("infile: {w} is not translated here"))),
            };
            let t = if ty.is_empty() {
                "float".to_string()
            } else {
                std::mem::take(&mut ty)
            };
            for n in names {
                slots.push(format!(
                    "list(name = {}, type = {})",
                    r_text(&n),
                    r_text(&t)
                ));
                vars.push((n, t.clone()));
            }
        }
        if vars.is_empty() {
            return Err(no("infile needs variables"));
        }
        if let Err(e) = self.raw_needs_clear(opts, "infile") {
            if e.reason == RAW_NOT_EMPTY {
                out.line(format!("stop({}, call. = FALSE)", r_text(RAW_NOT_EMPTY)));
                return Ok(());
            }
            return Err(e);
        }
        let path = file_name(&file, "raw");
        let name = frame_name(&path);
        let cond = self.raw_cond(c, out, &vars)?;
        self.helpers.insert("stata_float");
        self.helpers.insert("stata_rawread");
        out.line(format!(
            "{name} <- stata_infile({}, list({}){cond}{})",
            r_text(&path),
            slots.join(", "),
            if self.quiet(c) { ", show = FALSE" } else { "" }
        ));
        self.raw_new_data(name, &vars);
        Ok(())
    }
}

/// x1-x3: the names from x1 to x3 (a stem and numbers, as infile takes them).
fn var_range(a: &str, b: &str) -> Option<Vec<String>> {
    let split = |s: &str| {
        let stem = s.trim_end_matches(|ch: char| ch.is_ascii_digit());
        let n = s.get(stem.len()..)?.parse::<u32>().ok()?;
        Some((stem.to_string(), n))
    };
    let (s1, n1) = split(a)?;
    let (s2, n2) = split(b)?;
    if s1 != s2 || n2 < n1 || !is_name(a) {
        return None;
    }
    Some((n1..=n2).map(|n| format!("{s1}{n}")).collect())
}

/// `in a/b` as the first and last record (numbers only).
fn in_bounds(r: &str) -> Result<(String, String), Untranslated> {
    let (a, b) = r.split_once('/').unwrap_or((r, r));
    let num = |s: &str| s.trim().parse::<u64>().ok().filter(|n| *n >= 1);
    match (num(a), num(b)) {
        (Some(a), Some(b)) if a <= b => Ok((a.to_string(), b.to_string())),
        _ => Err(no(format!("a reader with in {r} is not translated yet"))),
    }
}
