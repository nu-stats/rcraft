//! `file open`, `file write` and `file close`, as Stata 19 writes a file
//! (`plan/stata/file.md`): text by default (`_newline` as the system writes
//! it, `\r\n` on Windows; every other byte as written, `_char(10)` too) or
//! binary (only `%1bu` and `%1bs` pieces, as Stata takes only formats there); `replace`
//! and `append`; strings, `_newline(n)`, `_tab(n)`, `_skip(n)`, `_char(n)`,
//! `(exp)` and `%fmt (exp)`, and in binary files `%1bu` and `%1bs` bytes.
//! `file read h name` reads a text file's next line into a local, with
//! r(eof) and r(status) as Stata 19 leaves them. `seek`, `query`, `set` and
//! binary reads are not translated yet.

use super::{is_name, no, Out, Session};
use crate::expr_r::{r_text, Kind, Untranslated};
use crate::reader::Command;

pub(super) const FILE_HELPER: &str = r##"if (!exists("stata_files")) stata_files <- list()
stata_file_open <- function(h, path, append = FALSE, binary = FALSE, replace = FALSE, show = TRUE) {
  if (!is.null(stata_files[[h]])) stop("file handle ", h, " already exists (Stata: r(110))", call. = FALSE)
  there <- file.exists(path)
  if (there && !append && !replace) stop("file ", path, " already exists (Stata: r(602))", call. = FALSE)
  if (!there && replace && show) cat("(file ", path, " not found)\n", sep = "")
  # bytes as written; a text file's _newline is the system's (\r\n on Windows), as Stata's
  mode <- if (append) "ab" else "wb"
  stata_files[[h]] <<- list(con = file(path, open = mode), binary = binary)
  invisible(NULL)
}
# read: the file's bytes, and where the next line starts
stata_file_open_read <- function(h, path) {
  if (!is.null(stata_files[[h]])) stop("file handle ", h, " already exists (Stata: r(110))", call. = FALSE)
  if (!file.exists(path)) stop("file ", path, " not found (Stata: r(601))", call. = FALSE)
  bytes <- readBin(path, "raw", n = file.size(path))
  stata_files[[h]] <<- list(con = NULL, binary = FALSE, read = TRUE, bytes = bytes, pos = 1L)
  invisible(NULL)
}
# the next line, without its end, into stata_file_line; r(status) says how it
# ended (win for CR LF, unix for LF, mac for CR, none at the end of the file)
# and r(eof) is 1, the line empty, past the last one
stata_file_read <- function(h) {
  f <- stata_file_get(h)
  if (!isTRUE(f$read)) stop("may not read from write-only file handle ", h, " (Stata: r(198))", call. = FALSE)
  b <- f$bytes
  n <- length(b)
  p <- f$pos
  if (p > n) {
    stata_file_line <<- ""
    stata_r <<- list(eof = 1, status = "eof")
    return(invisible(NULL))
  }
  ends <- which(b[p:n] == as.raw(10) | b[p:n] == as.raw(13))
  if (length(ends) == 0) {
    k <- n + 1L
    status <- "none"
    nxt <- n + 1L
  } else {
    k <- p + ends[1] - 1L
    if (b[k] == as.raw(10)) {
      status <- "unix"
      nxt <- k + 1L
    } else if (k < n && b[k + 1L] == as.raw(10)) {
      status <- "win"
      nxt <- k + 2L
    } else {
      status <- "mac"
      nxt <- k + 1L
    }
  }
  line <- if (k > p) b[p:(k - 1L)] else raw(0)
  if (any(line == as.raw(0))) stop("RCraft: file read of a line with a NUL byte is not translated yet", call. = FALSE)
  s <- rawToChar(line)
  Encoding(s) <- "UTF-8"
  stata_files[[h]]$pos <<- nxt
  stata_file_line <<- s
  stata_r <<- list(eof = 0, status = status)
  invisible(NULL)
}
stata_file_get <- function(h) {
  f <- stata_files[[h]]
  if (is.null(f)) stop("file handle ", h, " not found (Stata: r(111))", call. = FALSE)
  f
}
# the pieces: text, or raw bytes (binary files only)
stata_file_write <- function(h, ...) {
  f <- stata_file_get(h)
  if (isTRUE(f$read)) stop("may not write to read-only file handle ", h, " (Stata: r(198))", call. = FALSE)
  for (p in list(...)) {
    if (is.raw(p)) {
      if (!f$binary) stop("bytes can be written to a binary file only (Stata: r(198))", call. = FALSE)
      writeBin(p, f$con)
    } else {
      writeBin(charToRaw(enc2utf8(p)), f$con)
    }
  }
  invisible(NULL)
}
stata_file_close <- function(h = NULL) {
  hs <- if (is.null(h)) names(stata_files) else h
  for (k in hs) {
    con <- stata_file_get(k)$con
    if (!is.null(con)) close(con)
    stata_files[[k]] <<- NULL
  }
  invisible(NULL)
}
# _newline(k) in a text file
stata_file_nl <- function(k) strrep(if (.Platform$OS.type == "windows") "\r\n" else "\n", k)
# %1bu and %1bs: one byte, from 0 to 255 or -127 to 127 (missing as Stata's ends)
stata_file_byte <- function(v, signed = FALSE) {
  v <- trunc(v)
  if (is.na(v) || (signed && (v < -127 || v > 127)) || (!signed && (v < 0 || v > 255)))
    stop("a byte outside what %1b", if (signed) "s" else "u", " holds is not translated yet", call. = FALSE)
  as.raw(if (v < 0) v + 256 else v)
}"##;

/// One piece of a `file write` line, as R.
fn pieces(s: &Session, text: &str, out: &mut Out) -> Result<Vec<String>, Untranslated> {
    let cs: Vec<char> = text.chars().collect();
    let at = |k: usize| cs.get(k).copied();
    let span = |a: usize, b: usize| -> String { cs.get(a..b).unwrap_or(&[]).iter().collect() };
    // the text from `from` to the parenthesis that closes the one there
    let closing = |from: usize| -> Option<usize> {
        let mut depth = 0usize;
        let mut quoted = false;
        let mut k = from;
        while let Some(ch) = at(k) {
            match ch {
                '"' => quoted = !quoted,
                '(' if !quoted => depth += 1,
                ')' if !quoted => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        return Some(k);
                    }
                }
                _ => {}
            }
            k += 1;
        }
        None
    };
    let count = |arg: Option<String>| -> Result<usize, Untranslated> {
        match arg {
            None => Ok(1),
            Some(a) => a
                .trim()
                .parse::<usize>()
                .ok()
                .filter(|n| *n <= 10_000)
                .ok_or_else(|| no(format!("({a}) here is a whole number in file write"))),
        }
    };
    let mut out_pieces = Vec::new();
    let mut i = 0;
    while let Some(ch) = at(i) {
        if ch.is_whitespace() {
            i += 1;
        } else if ch == '"' {
            let s0 = i + 1;
            i += 1;
            while at(i).is_some_and(|d| d != '"') {
                i += 1;
            }
            out_pieces.push(r_text(&span(s0, i)));
            i += 1;
        } else if ch == '`' && at(i + 1) == Some('"') {
            // `"..."': the text up to "'
            let s0 = i + 2;
            let mut k = s0;
            while at(k).is_some() && !(at(k) == Some('"') && at(k + 1) == Some('\'')) {
                k += 1;
            }
            out_pieces.push(r_text(&span(s0, k)));
            i = k + 2;
        } else if ch == '_' {
            let s0 = i;
            while at(i).is_some_and(|d| d.is_ascii_alphanumeric() || d == '_') {
                i += 1;
            }
            let word = span(s0, i);
            let arg = if at(i) == Some('(') {
                let end = closing(i).ok_or_else(|| no("a ( without its ) in file write"))?;
                let a = span(i + 1, end);
                i = end + 1;
                Some(a)
            } else {
                None
            };
            let piece = match word.as_str() {
                "_n" | "_ne" | "_new" | "_newl" | "_newli" | "_newlin" | "_newline" => {
                    format!("stata_file_nl({})", count(arg)?)
                }
                "_t" | "_ta" | "_tab" => r_text(&"\t".repeat(count(arg)?)),
                "_s" | "_sk" | "_ski" | "_skip" => r_text(&" ".repeat(count(arg)?)),
                "_c" | "_ch" | "_cha" | "_char" => {
                    let a = arg.ok_or_else(|| no("_char needs its number"))?;
                    // a code written out that is a single byte: declined here, not in R
                    if let Ok(k) = a.trim().parse::<f64>().map(f64::trunc) {
                        if k == 0.0 || (128.0..=255.0).contains(&k) {
                            return Err(no(format!(
                                "_char({k}) is a single byte, which R's text does not hold as Stata does; not translated yet"
                            )));
                        }
                    }
                    let (v, k) = s.display_value(a.trim(), out)?;
                    if k == Kind::Text {
                        return Err(no("_char() takes a number (Stata: type mismatch)"));
                    }
                    out.helpers.push("stata_char");
                    format!("stata_char({v})")
                }
                other => return Err(no(format!("{other} in file write is not translated yet"))),
            };
            out_pieces.push(piece);
        } else if ch == '%' || ch == '(' {
            // an optional %format, then (exp)
            let mut fmt = None;
            if ch == '%' {
                let s0 = i;
                while at(i).is_some_and(|d| !d.is_whitespace() && d != '(') {
                    i += 1;
                }
                fmt = Some(span(s0, i));
                while at(i).is_some_and(char::is_whitespace) {
                    i += 1;
                }
            }
            if at(i) != Some('(') {
                return Err(no("file write takes (exp) after a %format"));
            }
            let end = closing(i).ok_or_else(|| no("a ( without its ) in file write"))?;
            let exp = span(i + 1, end);
            i = end + 1;
            let (v, k) = s.display_value(exp.trim(), out)?;
            let piece = match fmt.as_deref() {
                Some(b @ ("%1bu" | "%1bs")) => {
                    if k == Kind::Text {
                        return Err(no("a byte takes a number (Stata: type mismatch)"));
                    }
                    format!(
                        "stata_file_byte({v}{})",
                        if b == "%1bs" { ", signed = TRUE" } else { "" }
                    )
                }
                Some(b)
                    if b.starts_with("%2b")
                        || b.starts_with("%4b")
                        || b.starts_with("%8z")
                        || b.starts_with("%4z") =>
                {
                    return Err(no(format!(
                        "{b} in file write is not translated yet (%1bu and %1bs are)"
                    )))
                }
                Some(f) => {
                    if !super::returned::valid_format(f) {
                        return Err(no(format!("{f} is not a format file write takes")));
                    }
                    out.helpers.push("stata_fmt");
                    format!("stata_fmt({v}, {})", r_text(f))
                }
                None if k == Kind::Text => v,
                None => {
                    out.helpers.push("stata_fmt");
                    format!("stata_fmt({v}, \"%10.0g\", trim = TRUE)")
                }
            };
            out_pieces.push(piece);
        } else {
            let rest: String = span(i, (i + 20).min(cs.len()));
            return Err(no(format!("file write: {rest} is not translated yet")));
        }
    }
    Ok(out_pieces)
}

impl Session {
    /// A handle no translated `file open` opened: Stata stops (r(111)), and so
    /// does R, unless a `file open` that was not translated may have opened it.
    fn no_handle(&mut self, handle: &str, out: &mut Out) -> Result<(), Untranslated> {
        if self.file_lost.contains(handle) || !is_name(handle) {
            return Err(no(format!(
                "file handle {handle} is not known here: a file open before it was not translated"
            )));
        }
        out.line(format!("invisible(stata_file_get({}))", r_text(handle)));
        Ok(())
    }
}

impl Session {
    /// `file open|write|close`.
    pub(super) fn file(&mut self, c: &Command, out: &mut Out) -> Result<(), Untranslated> {
        let rest = super::returned::after_name(c);
        let rest = rest.trim();
        let (word, tail) = match rest.split_once(char::is_whitespace) {
            Some((w, t)) => (w, t.trim()),
            None => (rest, ""),
        };
        self.helpers.insert("stata_file");
        match word {
            "open" => {
                let handle = tail.split_whitespace().next().unwrap_or("");
                if !is_name(handle) {
                    return Err(no("file open needs a handle name"));
                }
                let file = c
                    .using
                    .clone()
                    .ok_or_else(|| no("file open needs using and a file"))?;
                let opts = super::Opts(&c.options);
                if let Some(o) = opts.other(&[
                    ("write", 1),
                    ("read", 1),
                    ("text", 1),
                    ("binary", 1),
                    ("replace", 7),
                    ("append", 1),
                    ("all", 3),
                ]) {
                    return Err(no(format!(
                        "file open with the option {o} is not translated yet"
                    )));
                }
                if opts.has("read", 1) {
                    if opts.has("write", 1) {
                        return Err(no(
                            "file open for reading and writing together is not translated yet",
                        ));
                    }
                    if opts.has("binary", 1) {
                        return Err(no(
                            "file open, read binary (file read with formats) is not translated yet",
                        ));
                    }
                    if opts.has("replace", 7) || opts.has("append", 1) {
                        return Err(no("file open, read with replace or append (Stata: r(198))"));
                    }
                    let path = super::file_name(&file, "");
                    self.file_lost.remove(handle);
                    out.line(format!(
                        "stata_file_open_read({}, {})",
                        r_text(handle),
                        r_text(&path)
                    ));
                    self.file_handles.insert(handle.to_string(), false);
                    return Ok(());
                }
                if !opts.has("write", 1) {
                    return Err(no("file open needs read or write (Stata: r(198))"));
                }
                if opts.has("text", 1) && opts.has("binary", 1) {
                    return Err(no("file open: text and binary together (Stata: r(198))"));
                }
                if opts.has("replace", 7) && opts.has("append", 1) {
                    return Err(no("file open: replace and append together (Stata: r(198))"));
                }
                let path = super::file_name(&file, "");
                let binary = opts.has("binary", 1);
                self.file_lost.remove(handle);
                out.line(format!(
                    "stata_file_open({}, {}, append = {}, binary = {}, replace = {}, show = {})",
                    r_text(handle),
                    r_text(&path),
                    if opts.has("append", 1) {
                        "TRUE"
                    } else {
                        "FALSE"
                    },
                    if binary { "TRUE" } else { "FALSE" },
                    if opts.has("replace", 7) {
                        "TRUE"
                    } else {
                        "FALSE"
                    },
                    if self.quiet(c) { "FALSE" } else { "TRUE" },
                ));
                self.file_handles.insert(handle.to_string(), binary);
                Ok(())
            }
            "write" | "w" | "wr" | "wri" | "writ" => {
                let (handle, items) = match tail.split_once(char::is_whitespace) {
                    Some((h, t)) => (h, t),
                    None => (tail, ""),
                };
                let Some(binary) = self.file_handles.get(handle).copied() else {
                    return self.no_handle(handle, out);
                };
                let ps = pieces(self, items, out)?;
                // a binary file takes only formatted pieces (Stata 19); %#s is not translated
                if binary && ps.iter().any(|p| !p.starts_with("stata_file_byte(")) {
                    return Err(no(
                        "file write to a binary file: only %1bu and %1bs pieces are translated, and Stata takes no plain strings, _n or (exp) there (Stata: r(198))",
                    ));
                }
                if !binary && ps.iter().any(|p| p.starts_with("stata_file_byte(")) {
                    return Err(no(
                        "%1bu and %1bs write to a binary file only (Stata: r(198))",
                    ));
                }
                if ps.is_empty() {
                    return Ok(());
                }
                out.line(format!(
                    "stata_file_write({}, {})",
                    r_text(handle),
                    ps.join(", ")
                ));
                Ok(())
            }
            "close" => {
                let handle = tail.trim();
                if handle == "_all" {
                    self.file_handles.clear();
                    out.line("stata_file_close()");
                    return Ok(());
                }
                if self.file_handles.remove(handle).is_none() {
                    return self.no_handle(handle, out);
                }
                out.line(format!("stata_file_close({})", r_text(handle)));
                Ok(())
            }
            "read" | "r" | "re" | "rea" => {
                let words: Vec<&str> = tail.split_whitespace().collect();
                let [handle, name] = words.as_slice() else {
                    if words.len() == 1 {
                        return Err(no(
                            "file read needs a local's name to read into (Stata: invalid syntax, r(198))",
                        ));
                    }
                    return Err(no(
                        "file read with formats (binary files) is not translated yet",
                    ));
                };
                if !is_name(name) || name.starts_with('%') {
                    return Err(no(
                        "file read with formats (binary files) is not translated yet",
                    ));
                }
                if !self.file_handles.contains_key(*handle) {
                    return self.no_handle(handle, out);
                }
                self.helpers.insert("stata_returned");
                out.line(format!("stata_file_read({})", r_text(handle)));
                self.r_new = Some((super::returned::R_FILE_READ, "file read".into()));
                Ok(())
            }
            other => Err(no(format!("file {other} is not translated yet"))),
        }
    }
}

/// The handle a `file open` line names, if it is one.
pub(super) fn opened(line: &str) -> Option<String> {
    let read = crate::reader::read(line.trim());
    let c = read.commands().first().copied()?;
    if c.name != "file" {
        return None;
    }
    let rest = super::returned::after_name(c);
    let mut ws = rest.split_whitespace();
    (ws.next() == Some("open"))
        .then(|| ws.next().map(str::to_string))
        .flatten()
        .filter(|h| is_name(h))
}
