//! `filefilter oldfile newfile, from() to() [replace]`, as Stata 19 does it
//! (observed on made-up files, `plan/stata/filefilter.md`): every
//! occurrence of from()'s bytes, left to right and not overlapping, becomes
//! to()'s; the rest of the file is copied byte for byte.
//!
//! - Codes: \BS backslash, \Q ", \LQ `, \RQ ', \$ $, \t tab, \n and \U a
//!   line feed, \r and \M a carriage return, \W CR LF, \###d a byte by its
//!   decimal code (0-255). Any other backslash is r(198).
//! - r(occurrences), r(bytes_from) and r(bytes_to) (the codes' lengths in
//!   bytes). An empty from() copies the file with no occurrence.
//! - Errors: a missing old file r(601); a new file that exists without
//!   `replace`, or the old file itself, r(602).

use super::*;

pub(super) const FILEFILTER_HELPER: &str = r##"stata_filefilter <- function(from_file, to_file, pat, rep, replace = FALSE) {
  if (!file.exists(from_file)) stop("file ", from_file, " not found (Stata: r(601))", call. = FALSE)
  if (normalizePath(to_file, mustWork = FALSE) == normalizePath(from_file, mustWork = FALSE) || (file.exists(to_file) && !replace))
    stop("file ", to_file, " already exists (Stata: r(602))", call. = FALSE)
  x <- readBin(from_file, "raw", n = file.info(from_file)$size)
  n <- 0
  out <- x
  if (length(pat) > 0 && length(x) >= length(pat)) {
    starts <- grepRaw(pat, x, fixed = TRUE, all = TRUE)
    # left to right, a match starting inside the one before it is no match
    keep <- integer()
    last <- 0
    for (s in starts) if (s > last) {
      keep <- c(keep, s)
      last <- s + length(pat) - 1
    }
    n <- length(keep)
    if (n > 0) {
      pieces <- list()
      at <- 1
      for (s in keep) {
        if (s > at) pieces[[length(pieces) + 1]] <- x[at:(s - 1)]
        pieces[[length(pieces) + 1]] <- rep
        at <- s + length(pat)
      }
      if (at <= length(x)) pieces[[length(pieces) + 1]] <- x[at:length(x)]
      out <- do.call(c, pieces)
      if (is.null(out)) out <- raw(0)
    }
  }
  writeBin(out, to_file)
  list(occurrences = n, bytes_from = length(pat), bytes_to = length(rep))
}"##;

/// The bytes a from() or to() stands for, Stata's codes decoded.
fn decode(text: &str) -> Result<Vec<u8>, String> {
    let b = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(&c) = b.get(i) {
        if c != b'\\' {
            out.push(c);
            i += 1;
            continue;
        }
        let rest = b.get(i + 1..).unwrap_or(&[]);
        let code: &[(&str, &[u8])] = &[
            ("BS", b"\\"),
            ("LQ", b"`"),
            ("RQ", b"'"),
            ("Q", b"\""),
            ("$", b"$"),
            ("t", b"\t"),
            ("n", b"\n"),
            ("r", b"\r"),
            ("W", b"\r\n"),
            ("U", b"\n"),
            ("M", b"\r"),
        ];
        if let Some((name, bytes)) = code.iter().find(|(n, _)| rest.starts_with(n.as_bytes())) {
            out.extend_from_slice(bytes);
            i += 1 + name.len();
            continue;
        }
        // \###d: a byte by its decimal code
        let digits = rest.get(..3).unwrap_or(&[]);
        if digits.len() == 3 && digits.iter().all(u8::is_ascii_digit) && rest.get(3) == Some(&b'd')
        {
            let v: u32 = std::str::from_utf8(digits)
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(999);
            if v > 255 {
                return Err(format!("\\{}d is not a byte", v));
            }
            out.push(v as u8);
            i += 5;
            continue;
        }
        return Err("a backslash code filefilter does not know".into());
    }
    Ok(out)
}

/// A from() or to() argument without its quotes (simple or compound).
fn unquote(a: &str) -> &str {
    let a = a.trim();
    if let Some(s) = a.strip_prefix("`\"").and_then(|s| s.strip_suffix("\"'")) {
        return s;
    }
    a.strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(a)
}

fn raw(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return "raw(0)".into();
    }
    format!(
        "as.raw(c({}))",
        bytes
            .iter()
            .map(|b| format!("0x{b:02x}"))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

impl Session {
    pub(super) fn filefilter(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[("from", 4), ("to", 2), ("replace", 7)]) {
            return Err(no(format!(
                "filefilter with the option {o} is not translated yet"
            )));
        }
        let files = crate::dofile::macros::tokens(&c.parts.main);
        let [old, new] = files.as_slice() else {
            return Err(no("filefilter needs an old file and a new file"));
        };
        let clean = |f: &str| f.trim().trim_matches('"').to_string();
        let (old, new) = (clean(old), clean(new));
        let (Some(from), Some(to)) = (opts.arg("from", 4), opts.arg("to", 2)) else {
            out.line("stop(\"options from() and to() required (Stata: r(198))\", call. = FALSE)");
            return Ok(());
        };
        let (pat, rep) = match (decode(unquote(&from)), decode(unquote(&to))) {
            (Ok(p), Ok(r)) => (p, r),
            (Err(e), _) | (_, Err(e)) => {
                out.line(format!(
                    "stop({}, call. = FALSE)",
                    r_text(&format!("filefilter: {e} (Stata: r(198))"))
                ));
                return Ok(());
            }
        };
        self.helpers.insert("stata_filefilter");
        self.helpers.insert("stata_returned");
        out.line(format!(
            "stata_r <- stata_filefilter({}, {}, {}, {}, replace = {})",
            r_text(&old),
            r_text(&new),
            raw(&pat),
            raw(&rep),
            super::stats::r_logical(opts.has("replace", 7))
        ));
        self.r_new = Some((super::returned::R_FILEFILTER, "filefilter".into()));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_stata_19s() {
        assert_eq!(decode(r"\BShline \BS\BS\n").unwrap(), b"\\hline \\\\\n");
        assert_eq!(decode(r"\Q\LQ\RQ\$\t\W\U\M").unwrap(), b"\"`'$\t\r\n\n\r");
        assert_eq!(decode(r"\097d\065d").unwrap(), b"aA");
        assert!(decode(r"\h").is_err() && decode(r"\256d").is_err());
        assert_eq!(unquote("`\"a \"b\"'"), "a \"b");
    }
}
