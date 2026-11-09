//! Stata's matrix expressions (`matrix A = ...`) as R: the do-file's own
//! matrices kept in `stata_mat`, with Stata's row and column names, as
//! Stata 19 gives them (`plan/stata/matrices.md`).
//!
//! Precedence, tightest first: `'` (transpose) and subscripts, unary minus,
//! `#` (not yet), `*` and `/` (by a scalar), `+` and `-`, then `,` (columns
//! side by side) and `\` (rows one under another). A number, a scalar or a
//! 1×1 matrix times a matrix is a scalar product.

use super::{is_name, no, Out, Session};
use crate::expr_r::{Kind, Untranslated};

/// The R behind the matrices: Stata's names on every result.
pub(super) const MATRIX_HELPER: &str = r##"if (!exists("stata_mat")) stata_mat <- list()
stata_mdefault <- function(n, p) paste0(p, seq_len(n))
stata_mnamed <- function(m) {
  m <- as.matrix(m)
  storage.mode(m) <- "double"
  if (is.null(rownames(m))) rownames(m) <- stata_mdefault(nrow(m), "r")
  if (is.null(colnames(m))) colnames(m) <- stata_mdefault(ncol(m), "c")
  m
}
stata_mscalar <- function(x) is.numeric(x) && !is.matrix(x) || (is.matrix(x) && all(dim(x) == 1))
stata_mconform <- function() stop("conformability error (Stata: r(503))", call. = FALSE)
stata_mmul <- function(a, b) {
  if (stata_mscalar(a) && !(is.matrix(b) && all(dim(b) == 1) && !is.matrix(a))) {
    if (!is.matrix(b)) return(as.numeric(a) * b)
    return(stata_mnamed(as.numeric(a) * b))
  }
  if (stata_mscalar(b)) return(stata_mnamed(a * as.numeric(b)))
  a <- stata_mnamed(a); b <- stata_mnamed(b)
  if (ncol(a) != nrow(b)) stata_mconform()
  m <- a %*% b
  dimnames(m) <- list(rownames(a), colnames(b))
  m
}
stata_mdiv <- function(a, s) {
  if (!stata_mscalar(s)) stop("matrix / matrix: Stata divides by a scalar only (r(503))", call. = FALSE)
  if (!is.matrix(a)) return(a / as.numeric(s))
  stata_mnamed(a / as.numeric(s))
}
stata_madd <- function(a, b, sign = 1) {
  if (!is.matrix(a) && !is.matrix(b)) return(a + sign * b)
  a <- stata_mnamed(a); b <- stata_mnamed(b)
  if (any(dim(a) != dim(b))) stata_mconform()
  m <- a + sign * b
  # the right one's names, as Stata 19 gives them
  dimnames(m) <- dimnames(b)
  m
}
stata_mt <- function(a) {
  a <- stata_mnamed(a)
  t(a)
}
# side by side (,) and one under another (\): default names start again
stata_mjoin <- function(parts, rows) {
  parts <- lapply(Filter(Negate(is.null), parts), stata_mnamed)
  if (!length(parts)) return(NULL)
  along <- if (rows) nrow else ncol
  keep <- if (rows) ncol else nrow
  if (length(unique(vapply(parts, keep, 1))) > 1) stata_mconform()
  m <- if (rows) do.call(rbind, parts) else do.call(cbind, parts)
  nm <- unlist(lapply(parts, if (rows) rownames else colnames))
  p <- if (rows) "r" else "c"
  if (all(grepl(paste0("^", p, "[0-9]+$"), nm))) nm <- stata_mdefault(length(nm), p)
  if (rows) { rownames(m) <- nm; colnames(m) <- colnames(parts[[1]]) } else { colnames(m) <- nm; rownames(m) <- rownames(parts[[1]]) }
  m
}
stata_mJ <- function(r, c, v) stata_mnamed(matrix(as.numeric(v), r, c))
stata_mI <- function(n) stata_mnamed(diag(n))
stata_minvsym <- function(a) {
  a <- stata_mnamed(a)
  if (nrow(a) != ncol(a)) stata_mconform()
  m <- tryCatch(solve(a), error = function(e) NULL)
  if (is.null(m)) stop("invsym of a singular matrix (Stata's generalized inverse) is not translated yet", call. = FALSE)
  dimnames(m) <- list(colnames(a), rownames(a))
  m
}
stata_minv <- function(a) {
  a <- stata_mnamed(a)
  if (nrow(a) != ncol(a)) stata_mconform()
  m <- tryCatch(solve(a), error = function(e) NULL)
  if (is.null(m)) stop("matrix has missing values (Stata: r(504))", call. = FALSE)
  dimnames(m) <- list(colnames(a), rownames(a))
  m
}
stata_mnames <- function(m, names, rows) {
  m <- stata_mnamed(m)
  if (length(names) != if (rows) nrow(m) else ncol(m)) stata_mconform()
  if (rows) rownames(m) <- names else colnames(m) <- names
  m
}
stata_mkmat <- function(d, vars, nomissing) {
  miss <- setdiff(vars, names(d))
  if (length(miss)) stop("variable ", miss[1], " not found (Stata: r(111))", call. = FALSE)
  if (!all(vapply(d[vars], is.numeric, TRUE))) stop("mkmat takes numeric variables (Stata: type mismatch)", call. = FALSE)
  m <- as.matrix(as.data.frame(lapply(d[vars], as.numeric)))
  if (nomissing) m <- m[stats::complete.cases(m), , drop = FALSE]
  dimnames(m) <- list(stata_mdefault(nrow(m), "r"), vars)
  m
}
stata_mvecdiag <- function(a) {
  a <- stata_mnamed(a)
  if (nrow(a) != ncol(a)) stata_mconform()
  matrix(diag(a), 1, dimnames = list("r1", colnames(a)))
}
# a vector's names on both sides, or r1.. and c1.. when it has Stata's defaults
stata_mdiag <- function(v) {
  v <- stata_mnamed(v)
  if (nrow(v) != 1 && ncol(v) != 1) stata_mconform()
  nm <- if (nrow(v) == 1) colnames(v) else rownames(v)
  m <- diag(as.vector(v), length(v))
  dimnames(m) <- if (all(grepl("^[rc][0-9]+$", nm))) list(stata_mdefault(length(v), "r"), stata_mdefault(length(v), "c")) else list(nm, nm)
  m
}
# one element, by number or by name; missing outside it
stata_matel <- function(m, i, j, name) {
  m <- stata_mnamed(m)
  i <- if (is.character(i)) match(i, rownames(m)) else floor(i)
  j <- if (is.character(j)) match(j, colnames(m)) else floor(j)
  if (is.na(i) || is.na(j) || i < 1 || j < 1 || i > nrow(m) || j > ncol(m)) stop(name, "[", i, ",", j, "] invalid subscript (Stata: r(503))", call. = FALSE)
  unname(m[i, j])
}
# A[i, j] = value: a scalar there, or a matrix with its corner there
stata_mset <- function(m, i, j, v) {
  m <- stata_mnamed(m)
  v <- as.matrix(v)
  ri <- floor(i) + seq_len(nrow(v)) - 1
  cj <- floor(j) + seq_len(ncol(v)) - 1
  if (min(ri) < 1 || min(cj) < 1 || max(ri) > nrow(m) || max(cj) > ncol(m)) stata_mconform()
  m[ri, cj] <- v
  m
}"##;

/// What part of an expression gives: a number or a matrix.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum MKind {
    Scalar,
    Matrix,
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(String),
    Name(String),
    Sym(char),
    Text(String),
}

fn tokens(text: &str) -> Result<Vec<Tok>, Untranslated> {
    let mut out = Vec::new();
    let cs: Vec<char> = text.chars().collect();
    let at = |k: usize| cs.get(k).copied();
    let span = |a: usize, b: usize| -> String { cs.get(a..b).unwrap_or(&[]).iter().collect() };
    let mut i = 0;
    while let Some(c) = at(i) {
        if c.is_whitespace() {
            i += 1;
        } else if c.is_ascii_digit() || (c == '.' && at(i + 1).is_some_and(|d| d.is_ascii_digit()))
        {
            let s = i;
            while at(i).is_some_and(|d| d.is_ascii_digit() || d == '.') {
                i += 1;
            }
            if matches!(at(i), Some('e' | 'E')) {
                i += 1;
                if matches!(at(i), Some('+' | '-')) {
                    i += 1;
                }
                while at(i).is_some_and(|d| d.is_ascii_digit()) {
                    i += 1;
                }
            }
            out.push(Tok::Num(span(s, i)));
        } else if c == '.'
            && !at(i + 1).is_some_and(|d| d.is_ascii_alphanumeric() || d == '_' || d == '.')
        {
            // a missing value (J(3, 2, .)); .a and the like are refused below
            out.push(Tok::Num("NA_real_".into()));
            i += 1;
        } else if c.is_ascii_alphabetic() || c == '_' {
            let s = i;
            while at(i).is_some_and(|d| d.is_ascii_alphanumeric() || d == '_') {
                i += 1;
            }
            out.push(Tok::Name(span(s, i)));
        } else if c == '"' {
            let s = i + 1;
            i += 1;
            while at(i).is_some_and(|d| d != '"') {
                i += 1;
            }
            out.push(Tok::Text(span(s, i)));
            i += 1;
        } else if "+-*/#^\\,()[]'".contains(c) {
            out.push(Tok::Sym(c));
            i += 1;
        } else {
            return Err(no(format!(
                "{c} in a matrix expression is not translated yet"
            )));
        }
    }
    Ok(out)
}

/// Translates a matrix expression, with the session for the scalar parts
/// (a name that is not a matrix, an index) and the matrices it knows.
struct MParser<'a> {
    toks: Vec<Tok>,
    pos: usize,
    s: &'a Session,
    out: &'a mut Out,
}

impl Session {
    /// A matrix expression as R, and whether it gives a number or a matrix.
    pub(super) fn matrix_expr(
        &self,
        text: &str,
        out: &mut Out,
    ) -> Result<(String, MKind), Untranslated> {
        MParser::translate(text, self, out)
    }
}

impl<'a> MParser<'a> {
    fn translate(
        text: &str,
        s: &'a Session,
        out: &'a mut Out,
    ) -> Result<(String, MKind), Untranslated> {
        let mut p = MParser {
            toks: tokens(text)?,
            pos: 0,
            s,
            out,
        };
        let r = p.rows()?;
        if p.pos != p.toks.len() {
            return Err(no(format!(
                "this matrix expression is not translated yet: {text}"
            )));
        }
        Ok(r)
    }

    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }

    fn eat(&mut self, c: char) -> bool {
        if self.peek() == Some(&Tok::Sym(c)) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn rows(&mut self) -> Result<(String, MKind), Untranslated> {
        let first = self.cols()?;
        if self.peek() != Some(&Tok::Sym('\\')) {
            return Ok(first);
        }
        let mut parts = vec![first.0];
        while self.eat('\\') {
            parts.push(self.cols()?.0);
        }
        Ok((
            format!("stata_mjoin(list({}), rows = TRUE)", parts.join(", ")),
            MKind::Matrix,
        ))
    }

    fn cols(&mut self) -> Result<(String, MKind), Untranslated> {
        let first = self.sum()?;
        if self.peek() != Some(&Tok::Sym(',')) {
            return Ok(first);
        }
        let mut parts = vec![first.0];
        while self.eat(',') {
            parts.push(self.sum()?.0);
        }
        Ok((
            format!("stata_mjoin(list({}), rows = FALSE)", parts.join(", ")),
            MKind::Matrix,
        ))
    }

    fn sum(&mut self) -> Result<(String, MKind), Untranslated> {
        let (mut code, mut kind) = self.product()?;
        loop {
            let sign = if self.eat('+') {
                "1"
            } else if self.eat('-') {
                "-1"
            } else {
                break;
            };
            let (r, rk) = self.product()?;
            kind = if kind == MKind::Matrix || rk == MKind::Matrix {
                MKind::Matrix
            } else {
                MKind::Scalar
            };
            code = format!("stata_madd({code}, {r}, sign = {sign})");
        }
        Ok((code, kind))
    }

    fn product(&mut self) -> Result<(String, MKind), Untranslated> {
        let (mut code, mut kind) = self.unary()?;
        loop {
            if self.eat('*') {
                let (r, rk) = self.unary()?;
                kind = if kind == MKind::Matrix || rk == MKind::Matrix {
                    MKind::Matrix
                } else {
                    MKind::Scalar
                };
                code = format!("stata_mmul({code}, {r})");
            } else if self.eat('/') {
                let (r, _) = self.unary()?;
                code = format!("stata_mdiv({code}, {r})");
            } else if self.peek() == Some(&Tok::Sym('#')) {
                return Err(no(
                    "the Kronecker product (#) in a matrix expression is not translated yet",
                ));
            } else {
                break;
            }
        }
        Ok((code, kind))
    }

    fn unary(&mut self) -> Result<(String, MKind), Untranslated> {
        if self.eat('-') {
            let (c, k) = self.unary()?;
            return Ok((format!("stata_mmul(-1, {c})"), k));
        }
        if self.eat('+') {
            return self.unary();
        }
        let (mut code, kind) = self.primary()?;
        while self.eat('\'') {
            code = format!("stata_mt({code})");
        }
        // a power, of numbers only: right to left, tighter than a minus
        if self.eat('^') {
            let (e, ek) = self.unary()?;
            if kind == MKind::Matrix || ek == MKind::Matrix {
                return Err(no("^ of a matrix is not Stata's (Stata: type mismatch)"));
            }
            code = format!("({code})^({e})");
        }
        Ok((code, kind))
    }

    fn args(&mut self) -> Result<Vec<String>, Untranslated> {
        // the raw text of each argument, split at commas outside parentheses
        let mut out = Vec::new();
        let mut cur = Vec::new();
        let mut depth = 0usize;
        while let Some(t) = self.toks.get(self.pos).cloned() {
            self.pos += 1;
            match &t {
                Tok::Sym('(') => depth += 1,
                Tok::Sym(')') if depth == 0 => {
                    out.push(cur);
                    return Ok(out.into_iter().map(|ts| text_of(&ts)).collect());
                }
                Tok::Sym(')') => depth -= 1,
                Tok::Sym(',') if depth == 0 => {
                    out.push(std::mem::take(&mut cur));
                    continue;
                }
                _ => {}
            }
            cur.push(t);
        }
        Err(no("a ( without its ) in a matrix expression"))
    }

    fn primary(&mut self) -> Result<(String, MKind), Untranslated> {
        match self.toks.get(self.pos).cloned() {
            Some(Tok::Num(n)) => {
                self.pos += 1;
                Ok((n, MKind::Scalar))
            }
            Some(Tok::Sym('(')) => {
                self.pos += 1;
                let inner = self.rows()?;
                if !self.eat(')') {
                    return Err(no("a ( without its ) in a matrix expression"));
                }
                Ok(inner)
            }
            Some(Tok::Name(n)) => {
                self.pos += 1;
                if self.eat('(') {
                    let args = self.args()?;
                    return self.function(&n, &args);
                }
                if self.peek() == Some(&Tok::Sym('[')) {
                    self.pos += 1;
                    let start = self.pos;
                    let mut depth = 0usize;
                    while let Some(t) = self.toks.get(self.pos) {
                        match t {
                            Tok::Sym('[') => depth += 1,
                            Tok::Sym(']') if depth == 0 => break,
                            Tok::Sym(']') => depth -= 1,
                            _ => {}
                        }
                        self.pos += 1;
                    }
                    let inside = text_of(self.toks.get(start..self.pos).unwrap_or(&[]));
                    if !self.eat(']') {
                        return Err(no("a [ without its ] in a matrix expression"));
                    }
                    if inside.contains("..") {
                        return Err(no("a submatrix (A[1..2, 3]) is not translated yet"));
                    }
                    let code = self.scalar(&format!("{n}[{inside}]"))?;
                    return Ok((code, MKind::Scalar));
                }
                match self.matrix(&n)? {
                    Some(m) => Ok((m, MKind::Matrix)),
                    // not a matrix: a scalar, as Stata takes one
                    None => Ok((self.scalar(&n)?, MKind::Scalar)),
                }
            }
            _ => Err(no("this matrix expression is not translated yet")),
        }
    }

    /// A scalar part, through the expression translator; never a text.
    fn scalar(&mut self, text: &str) -> Result<String, Untranslated> {
        let (code, kind) = self.s.display_value(text, self.out)?;
        if kind == Kind::Text {
            return Err(no(format!(
                "{text} is a text, not a number, in a matrix expression (Stata: type mismatch)"
            )));
        }
        Ok(code)
    }

    /// A matrix the do-file made, by name; none when it made none of that
    /// name, and refused when a line not translated may have changed it.
    fn matrix(&self, name: &str) -> Result<Option<String>, Untranslated> {
        if self.s.user_mats.contains(name) || self.s.lost_mats.contains_key(name) {
            return self.s.user_matrix(name).map(Some);
        }
        Ok(None)
    }

    fn function(&mut self, name: &str, args: &[String]) -> Result<(String, MKind), Untranslated> {
        match (name, args) {
            ("e" | "r", [n]) => {
                let n = n.trim();
                if !is_name(n) {
                    return Err(no(format!(
                        "{name}({n}) in a matrix expression is not translated yet"
                    )));
                }
                Ok((self.s.returned_matrix(name, n)?, MKind::Matrix))
            }
            ("J", [r, c, v]) => {
                let r = self.scalar(r)?;
                let c = self.scalar(c)?;
                let v = self.scalar(v)?;
                Ok((format!("stata_mJ({r}, {c}, {v})"), MKind::Matrix))
            }
            ("I", [n]) => Ok((format!("stata_mI({})", self.scalar(n)?), MKind::Matrix)),
            ("invsym" | "inv", [m]) => {
                let (code, _) = MParser::translate(m, self.s, self.out)?;
                let f = if name == "inv" {
                    "stata_minv"
                } else {
                    "stata_minvsym"
                };
                Ok((format!("{f}({code})"), MKind::Matrix))
            }
            ("vecdiag" | "diag", [m]) => {
                let (code, _) = MParser::translate(m, self.s, self.out)?;
                Ok((format!("stata_m{name}({code})"), MKind::Matrix))
            }
            ("nullmat", [n]) => {
                let n = n.trim();
                Ok(match self.matrix(n)? {
                    Some(m) => (m, MKind::Matrix),
                    None => ("NULL".to_string(), MKind::Matrix),
                })
            }
            ("cholesky" | "corr" | "hadamard" | "get" | "matuniform" | "sweep" | "vec", _) => {
                Err(no(format!(
                    "{name}() in a matrix expression is not translated yet"
                )))
            }
            // any other function gives a number: the scalar translator's
            _ => Ok((
                self.scalar(&format!("{name}({})", args.join(",")))?,
                MKind::Scalar,
            )),
        }
    }
}

/// Tokens back to text, to hand a part to the scalar translator.
fn text_of(ts: &[Tok]) -> String {
    let mut s = String::new();
    for t in ts {
        match t {
            Tok::Num(n) | Tok::Name(n) => {
                if s.chars()
                    .last()
                    .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
                {
                    s.push(' ');
                }
                // the missing value is `.` again in Stata's text
                s.push_str(if n == "NA_real_" { "." } else { n });
            }
            Tok::Text(x) => {
                s.push('"');
                s.push_str(x);
                s.push('"');
            }
            Tok::Sym(c) => s.push(*c),
        }
    }
    s
}
