//! Stata's string functions of batch F in R: `word()`, `strproper()`,
//! `stritrim()`, `strrpos()`, `strreverse()`, `subinword()`, `regexr()`,
//! `regexm()` with `regexs()`, `plural()`, `bsubstr()` and `udsubstr()`, as
//! Stata 19 gives them (`plan/stata/functions-f.md`).
//!
//! What was observed and is kept: the words of `word()` and `wordcount()`
//! are what blanks separate (a tab or a new line is part of a word and
//! quotes are not special); `strproper()` and `stritrim()` work on ASCII
//! letters and blanks only; `strrpos()`, `strreverse()`, `subinword()`,
//! `regexr()` and `bsubstr()` work on bytes; `regexr()` puts the new text in
//! as it stands (`\1` is two characters) and gives the string back whole
//! when the expression is not a valid one; and the expressions are
//! leftmost-first with lookahead, `(?i)` and `\<` `\>`, as PCRE.

use super::*;

/// The R helpers, one a function (`stata_word()` for `word()`).
pub(crate) const STRFN_HELPER: &str = r##"stata_sf_chr <- function(x) {
  if (!is.character(x)) stop("type mismatch (Stata: r(109))", call. = FALSE)
  x[is.na(x)] <- ""
  x
}
stata_sf_n <- function(...) {
  l <- lengths(list(...))
  if (any(l == 0L)) 0L else max(l)
}
stata_sf_u8 <- function(x) {
  Encoding(x) <- "UTF-8"
  x
}
stata_sf_bytes <- function(r) stata_sf_u8(rawToChar(r))
stata_word <- function(s, n) {
  s <- stata_sf_chr(s)
  m <- stata_sf_n(s, n)
  s <- rep_len(s, m)
  n <- trunc(rep_len(as.numeric(n), m))
  vapply(seq_len(m), function(i) {
    w <- strsplit(s[i], " ", fixed = TRUE, useBytes = TRUE)[[1]]
    w <- w[nzchar(w)]
    k <- n[i]
    if (is.na(k) || k == 0 || abs(k) > length(w)) return("")
    stata_sf_u8(if (k > 0) w[k] else w[length(w) + k + 1])
  }, "", USE.NAMES = FALSE)
}
stata_wordcount <- function(s) {
  s <- stata_sf_chr(s)
  as.numeric(lengths(regmatches(s, gregexpr("[^ ]+", s, useBytes = TRUE))))
}
stata_strproper <- function(s) {
  s <- stata_sf_chr(s)
  vapply(s, function(x) {
    b <- as.integer(charToRaw(x))
    if (!length(b)) return("")
    lo <- b >= 97L & b <= 122L
    up <- b >= 65L & b <= 90L
    prev <- c(FALSE, (lo | up)[-length(b)])
    b[lo & !prev] <- b[lo & !prev] - 32L
    b[up & prev] <- b[up & prev] + 32L
    stata_sf_bytes(as.raw(b))
  }, "", USE.NAMES = FALSE)
}
stata_stritrim <- function(s) {
  s <- stata_sf_chr(s)
  stata_sf_u8(gsub("(?<=[^ ]) {2,}(?=[^ ])", " ", s, perl = TRUE, useBytes = TRUE))
}
stata_strrpos <- function(s, t) {
  s <- stata_sf_chr(s)
  t <- stata_sf_chr(t)
  m <- stata_sf_n(s, t)
  s <- rep_len(s, m)
  t <- rep_len(t, m)
  vapply(seq_len(m), function(i) {
    a <- charToRaw(s[i])
    b <- charToRaw(t[i])
    la <- length(a)
    lb <- length(b)
    if (lb == 0L) return(as.numeric(la))
    if (lb > la || !grepl(t[i], s[i], fixed = TRUE, useBytes = TRUE)) return(0)
    for (p in seq.int(la - lb + 1L, 1L)) if (all(a[p:(p + lb - 1L)] == b)) return(as.numeric(p))
    0
  }, 0, USE.NAMES = FALSE)
}
# strpos(): the byte where t first starts in s, 0 for none; an empty t is at 1 unless s is empty too
stata_strpos <- function(s, t) {
  s <- stata_sf_chr(s)
  t <- stata_sf_chr(t)
  m <- stata_sf_n(s, t)
  s <- rep_len(s, m)
  t <- rep_len(t, m)
  vapply(seq_len(m), function(i) {
    if (!nzchar(t[i])) return(as.numeric(nzchar(s[i])))
    as.numeric(max(0L, regexpr(t[i], s[i], fixed = TRUE, useBytes = TRUE)[1]))
  }, 0, USE.NAMES = FALSE)
}
stata_strreverse <- function(s) {
  s <- stata_sf_chr(s)
  vapply(s, function(x) stata_sf_bytes(rev(charToRaw(x))), "", USE.NAMES = FALSE)
}
# old a word of s (blanks or an end on each side) is replaced, the first n of them (all for missing n; n
# below 1 or above 2147483647 replaces none)
stata_subinword <- function(s, old, new, n) {
  s <- stata_sf_chr(s)
  old <- stata_sf_chr(old)
  new <- stata_sf_chr(new)
  m <- stata_sf_n(s, old, new, n)
  s <- rep_len(s, m)
  old <- rep_len(old, m)
  new <- rep_len(new, m)
  n <- trunc(rep_len(as.numeric(n), m))
  vapply(seq_len(m), function(i) {
    k <- if (is.na(n[i])) Inf else n[i]
    if (!is.na(n[i]) && (k < 1 || k > 2147483647)) return(s[i])
    if (!nzchar(old[i]) || !grepl(old[i], s[i], fixed = TRUE, useBytes = TRUE)) return(s[i])
    a <- charToRaw(s[i])
    o <- charToRaw(old[i])
    r <- charToRaw(new[i])
    la <- length(a)
    lo <- length(o)
    out <- raw(0)
    p <- 1L
    done <- 0
    while (p <= la) {
      hit <- done < k && p + lo - 1L <= la && (p == 1L || a[p - 1L] == as.raw(32L)) &&
        (p + lo - 1L == la || a[p + lo] == as.raw(32L)) && all(a[p:(p + lo - 1L)] == o)
      if (hit) {
        out <- c(out, r)
        p <- p + lo
        done <- done + 1
      } else {
        out <- c(out, a[p])
        p <- p + 1L
      }
    }
    stata_sf_bytes(out)
  }, "", USE.NAMES = FALSE)
}
# Stata reads \< and \> as the ends of a word; PCRE does not
stata_re <- function(re) {
  if (!grepl("\\\\[<>]", re)) return(re)
  t <- regmatches(re, gregexpr("\\\\.|[^\\\\]+|\\\\$", re, perl = TRUE))[[1]]
  paste0(ifelse(t == "\\<", "\\b(?=\\w)", ifelse(t == "\\>", "\\b(?<=\\w)", t)), collapse = "")
}
stata_regexr <- function(s, re, new) {
  s <- stata_sf_chr(s)
  re <- stata_sf_chr(re)
  new <- stata_sf_chr(new)
  m <- stata_sf_n(s, re, new)
  s <- rep_len(s, m)
  re <- rep_len(re, m)
  new <- rep_len(new, m)
  vapply(seq_len(m), function(i) {
    r <- tryCatch(suppressWarnings(regexpr(stata_re(re[i]), s[i], perl = TRUE, useBytes = TRUE)), error = function(e) NULL)
    if (is.null(r) || r[1] < 1L) return(s[i])
    a <- charToRaw(s[i])
    st <- as.integer(r[1])
    len <- as.integer(attr(r, "match.length")[1])
    stata_sf_bytes(c(a[seq_along(a) < st], charToRaw(new[i]), a[seq_along(a) >= st + len]))
  }, "", USE.NAMES = FALSE)
}
# regexm() keeps the groups of its last match (a row of them for each row it was given), regexs() reads them
stata_regexm <- function(s, re) {
  s <- stata_sf_chr(s)
  re <- stata_sf_chr(re)
  m <- stata_sf_n(s, re)
  s <- rep_len(s, m)
  re <- rep_len(re, m)
  g <- matrix("", m, 10L)
  hit <- logical(m)
  for (i in seq_len(m)) {
    r <- tryCatch(suppressWarnings(regexec(stata_re(re[i]), s[i], perl = TRUE, useBytes = TRUE)[[1]]), error = function(e) NULL)
    if (is.null(r) || r[1] < 1L) next
    hit[i] <- TRUE
    st <- as.integer(r)
    len <- as.integer(attr(r, "match.length"))
    a <- charToRaw(s[i])
    for (j in seq_len(min(length(st), 10L))) {
      if (st[j] >= 1L) g[i, j] <- stata_sf_bytes(a[seq_along(a) >= st[j] & seq_along(a) < st[j] + len[j]])
    }
  }
  assign("stata_rx_state", g, envir = globalenv())
  hit
}
stata_regexs <- function(n) {
  g <- get0("stata_rx_state", envir = globalenv(), ifnotfound = matrix("", 1L, 10L))
  if (nrow(g) == 0L) return(character(0))
  n <- trunc(as.numeric(n))
  m <- max(length(n), nrow(g))
  n <- rep_len(n, m)
  r <- rep_len(seq_len(nrow(g)), m)
  ok <- !is.na(n) & n >= 0 & n <= 9
  out <- rep("", m)
  out[ok] <- g[cbind(r[ok], n[ok] + 1)]
  out
}
# n of 1 or -1 gives the word itself; otherwise s and "s", or s2 as it is, as "+x" added, as "-x" cut off its end
stata_plural <- function(n, s, s2 = NULL) {
  s <- stata_sf_chr(s)
  n <- as.numeric(n)
  m <- if (is.null(s2)) stata_sf_n(n, s) else stata_sf_n(n, s, stata_sf_chr(s2))
  n <- rep_len(n, m)
  s <- rep_len(s, m)
  if (is.null(s2)) {
    p <- paste0(s, "s")
  } else {
    p <- rep_len(stata_sf_chr(s2), m)
    cut <- startsWith(p, "-")
    add <- startsWith(p, "+")
    rest <- substring(p, 2L)
    keep <- cut & !endsWith(s, rest)
    zero <- cut & nchar(rest, type = "bytes") == 0L
    p[cut] <- vapply(which(cut), function(i) {
      a <- charToRaw(s[i])
      if (keep[i] || zero[i]) return(s[i])
      stata_sf_bytes(a[seq_len(length(a) - length(charToRaw(rest[i])))])
    }, "", USE.NAMES = FALSE)
    p[add] <- paste0(s[add], rest[add])
  }
  one <- !is.na(n) & abs(n) == 1
  p[one] <- s[one]
  p
}
# substr() by bytes: n1 below 0 counts from the end, n1 of 0 or beyond the string gives "", n2 missing is the rest
stata_bsubstr <- function(s, n1, n2) {
  s <- stata_sf_chr(s)
  m <- stata_sf_n(s, n1, n2)
  s <- rep_len(s, m)
  n1 <- trunc(rep_len(as.numeric(n1), m))
  n2 <- trunc(rep_len(as.numeric(n2), m))
  vapply(seq_len(m), function(i) {
    a <- charToRaw(s[i])
    la <- length(a)
    if (is.na(n1[i]) || n1[i] == 0) return("")
    st <- if (n1[i] < 0) la + n1[i] + 1 else n1[i]
    if (st < 1 || st > la) return("")
    k <- if (is.na(n2[i])) Inf else n2[i]
    if (k < 1) return("")
    stata_sf_bytes(a[seq_along(a) >= st & seq_along(a) < st + k])
  }, "", USE.NAMES = FALSE)
}
# usubstr(): the same by characters
stata_usubstr <- function(s, n1, n2) {
  s <- stata_sf_chr(s)
  m <- stata_sf_n(s, n1, n2)
  s <- rep_len(s, m)
  n1 <- trunc(rep_len(as.numeric(n1), m))
  n2 <- trunc(rep_len(as.numeric(n2), m))
  la <- nchar(s, type = "chars", allowNA = TRUE)
  la[is.na(la)] <- 0L
  st <- ifelse(!is.na(n1) & n1 < 0, la + n1 + 1, n1)
  k <- ifelse(is.na(n2), Inf, n2)
  none <- is.na(n1) | n1 == 0 | is.na(st) | st < 1 | st > la | k < 1
  r <- substr(s, ifelse(none, 1, st), ifelse(none, 0, pmin(la, st + k - 1)))
  r[none] <- ""
  r
}
# udsubstr(): n1 counts characters as usubstr() does, n2 display columns (two for a wide character); whole characters only
stata_udsubstr <- function(s, n1, n2) {
  s <- stata_sf_chr(s)
  m <- stata_sf_n(s, n1, n2)
  s <- rep_len(s, m)
  n1 <- trunc(rep_len(as.numeric(n1), m))
  n2 <- trunc(rep_len(as.numeric(n2), m))
  vapply(seq_len(m), function(i) {
    ch <- tryCatch(strsplit(s[i], "")[[1]], error = function(e) character(0))
    la <- length(ch)
    if (is.na(n1[i]) || n1[i] == 0) return("")
    st <- if (n1[i] < 0) la + n1[i] + 1 else n1[i]
    if (st < 1 || st > la) return("")
    k <- if (is.na(n2[i])) Inf else n2[i]
    if (k < 1) return("")
    rest <- ch[st:la]
    fits <- cumsum(nchar(rest, type = "width")) <= k
    cnt <- if (all(fits)) length(fits) else which(!fits)[1] - 1L
    if (cnt < 1L) "" else paste0(rest[seq_len(cnt)], collapse = "")
  }, "", USE.NAMES = FALSE)
}"##;

/// The functions here: Stata's name, the R helper, what each argument is
/// (`s` a string, `n` a number) and what it gives.
const FUNCTIONS: &[(&str, &str, &str, Kind)] = &[
    ("bsubstr", "stata_bsubstr", "snn", Kind::Text),
    ("index", "stata_strpos", "ss", Kind::Number),
    ("itrim", "stata_stritrim", "s", Kind::Text),
    ("proper", "stata_strproper", "s", Kind::Text),
    ("regexr", "stata_regexr", "sss", Kind::Text),
    ("regexs", "stata_regexs", "n", Kind::Text),
    ("reverse", "stata_strreverse", "s", Kind::Text),
    ("stritrim", "stata_stritrim", "s", Kind::Text),
    ("strpos", "stata_strpos", "ss", Kind::Number),
    ("strproper", "stata_strproper", "s", Kind::Text),
    ("strreverse", "stata_strreverse", "s", Kind::Text),
    ("strrpos", "stata_strrpos", "ss", Kind::Number),
    ("subinword", "stata_subinword", "sssn", Kind::Text),
    ("substr", "stata_bsubstr", "snn", Kind::Text),
    ("udsubstr", "stata_udsubstr", "snn", Kind::Text),
    ("usubstr", "stata_usubstr", "snn", Kind::Text),
    ("word", "stata_word", "sn", Kind::Text),
];

pub(super) fn is_string_function(name: &str) -> bool {
    name == "plural" || FUNCTIONS.iter().any(|(f, ..)| *f == name)
}

impl Translator<'_> {
    /// A string function of this batch: its R helper on the arguments, each
    /// checked to be the string or number Stata wants.
    pub(super) fn string_function(
        &mut self,
        name: &str,
        args: &[Expr],
    ) -> Result<(String, Kind), Untranslated> {
        let (helper, want, kind) = if name == "plural" {
            let want = match args.len() {
                2 => "ns",
                3 => "nss",
                _ => {
                    return Err(no(
                        "plural() takes 2 or 3 arguments (Stata: invalid syntax)".to_string(),
                    ))
                }
            };
            ("stata_plural", want, Kind::Text)
        } else {
            let Some((_, h, w, k)) = FUNCTIONS.iter().find(|(f, ..)| *f == name) else {
                return Err(no(format!("the function {name}() is not translated yet")));
            };
            (*h, *w, *k)
        };
        if args.len() != want.len() {
            return Err(no(format!(
                "{name}() takes {} argument{} (Stata: invalid syntax)",
                want.len(),
                if want.len() == 1 { "" } else { "s" }
            )));
        }
        let mut vals = Vec::new();
        for (a, w) in args.iter().zip(want.chars()) {
            let (c, k) = self.expr(a)?;
            if w == 's' {
                if matches!(k, Kind::Number | Kind::Logical) {
                    return Err(no(format!("{name}() of a number (Stata: type mismatch)")));
                }
                vals.push(c);
            } else {
                if k == Kind::Text {
                    return Err(no(format!("{name}() of a string (Stata: type mismatch)")));
                }
                vals.push(number(&c, k)?);
            }
        }
        if name == "regexs" {
            self.caveat(
                "regexs() gives the groups of the regexm() of the same command, row by row; \
                 Stata gives those of the last row regexm() read, which differs only after \
                 a regexm() in an earlier command",
            );
        }
        self.helpers.push("stata_strfn");
        Ok((format!("{helper}({})", vals.join(", ")), kind))
    }
}

#[cfg(test)]
mod tests {
    use super::super::{translate, Use};

    fn none(_: &str) -> Option<super::Kind> {
        None
    }

    #[test]
    fn string_functions_call_their_helpers() {
        let code = |t: &str| translate(t, &none, Use::Value).map(|r| (r.code, r.helpers));
        assert_eq!(
            code("word(s, -1)"),
            Ok(("stata_word(s, -1)".to_string(), vec!["stata_strfn"]))
        );
        assert_eq!(
            code("proper(s)").map(|r| r.0),
            Ok("stata_strproper(s)".to_string())
        );
        assert_eq!(
            code("plural(2, \"cat\", \"+s\")").map(|r| r.0),
            Ok("stata_plural(2, \"cat\", \"+s\")".to_string())
        );
        assert_eq!(
            code("subinword(s, \"a\", \"b\", .)").map(|r| r.0),
            Ok("stata_subinword(s, \"a\", \"b\", NA_real_)".to_string())
        );
        let refused = |t: &str| translate(t, &none, Use::Value).err().map(|e| e.reason);
        assert_eq!(
            refused("word(\"a b\")"),
            Some("word() takes 2 arguments (Stata: invalid syntax)".to_string())
        );
        assert_eq!(
            refused("word(5, 1)"),
            Some("word() of a number (Stata: type mismatch)".to_string())
        );
        assert_eq!(
            refused("word(\"a b\", \"1\")"),
            Some("word() of a string (Stata: type mismatch)".to_string())
        );
        assert_eq!(
            refused("plural(2)"),
            Some("plural() takes 2 or 3 arguments (Stata: invalid syntax)".to_string())
        );
    }

    #[test]
    fn the_table_is_sorted_and_has_no_twins() {
        let names: Vec<&str> = super::FUNCTIONS.iter().map(|(f, ..)| *f).collect();
        let mut sorted = names.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(names, sorted);
    }
}
