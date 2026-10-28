//! P2B.2: Stata expressions in R.
//!
//! An expression is parsed with the vendored StatCraft parser
//! ([`crate::lang::expr`]) and written as R for a dplyr data-masking context
//! (inside `mutate()` or `filter()`, after `group_by()` for `by:`), where
//! columns are bare names and `dplyr::row_number()` and `dplyr::n()` stand for
//! `_n` and `_N`.
//!
//! Stata's rules that R does not share are written out, never assumed away:
//!
//! - A missing number is larger than every other number in a comparison
//!   (`x > 5` is true when `x` is missing; `x < .` means "not missing").
//! - True and false are numbers: anything not zero is true, missing included,
//!   and a comparison gives 1 or 0.
//! - `x / 0` is missing; `+` joins strings; `max()` and `min()` skip missing.
//! - `round()` goes halfway up with a tiny allowance (as StatCraft found Stata
//!   does: 2.345 rounds to 2.35), `mod(x, y)` is missing for `y <= 0`, and
//!   `inrange()` is false for a missing first argument.
//!
//! Where R cannot say the same thing, the expression is not translated and the
//! reason is given ([`Untranslated`]); where the R is right but loses a detail
//! (Stata's extended missing values `.a` to `.z`, regular-expression dialects,
//! how `string()` formats numbers), a caveat goes with it. The rules for
//! `round`, `mod`, `max`/`min`, `missing`, `inrange`, `inlist`, `cond` and `sum`
//! follow StatCraft's implementations, which are checked against Stata's output.

use crate::lang::expr::{parse, BinaryOp, Expr, UnaryOp};
use crate::values::datetime::parse_date;
use crate::values::missing::{is_missing, missing_code};

/// What an R expression holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// TRUE/FALSE, never NA (comparisons are written so).
    Logical,
    Number,
    Text,
    /// A variable whose type is not known: treated as a number.
    Unknown,
}

/// What the expression is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Use {
    /// A value (`generate y = exp`): true and false become 1 and 0.
    Value,
    /// A condition (`if exp`, `keep if exp`): TRUE or FALSE for each row.
    Condition,
}

/// An expression in R.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RExpr {
    pub code: String,
    pub kind: Kind,
    /// Details the R does not keep, in words.
    pub caveats: Vec<String>,
}

/// Why an expression was not translated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Untranslated {
    pub reason: String,
}

fn no(reason: impl Into<String>) -> Untranslated {
    Untranslated {
        reason: reason.into(),
    }
}

/// Translates `text` for `use_`. `kind_of` gives the type of a variable when it
/// is known (from the data in memory); `None` means unknown, taken as a number.
pub fn translate(
    text: &str,
    kind_of: &dyn Fn(&str) -> Option<Kind>,
    use_: Use,
) -> Result<RExpr, Untranslated> {
    let tree = parse(text).map_err(|e| no(format!("Stata would not read it: {}", e.message)))?;
    let mut t = Translator {
        kind_of,
        caveats: Vec::new(),
    };
    let (code, kind) = t.expr(&tree)?;
    let (code, kind) = match use_ {
        Use::Condition => (truth(&code, kind)?, Kind::Logical),
        Use::Value if kind == Kind::Logical => (format!("as.numeric({code})"), Kind::Number),
        Use::Value => (code, kind),
    };
    let mut caveats = t.caveats;
    caveats.dedup();
    Ok(RExpr {
        code,
        kind,
        caveats,
    })
}

struct Translator<'a> {
    kind_of: &'a dyn Fn(&str) -> Option<Kind>,
    caveats: Vec<String>,
}

const R_RESERVED: &[&str] = &[
    "if",
    "else",
    "repeat",
    "while",
    "function",
    "for",
    "next",
    "break",
    "TRUE",
    "FALSE",
    "NULL",
    "Inf",
    "NaN",
    "NA",
    "NA_integer_",
    "NA_real_",
    "NA_character_",
    "in",
];

/// A Stata name as an R name: backticks when R would not read it bare.
pub fn r_name(name: &str) -> String {
    let syntactic = name
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '.')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_')
        && !R_RESERVED.contains(&name);
    if syntactic {
        name.to_string()
    } else {
        format!("`{}`", name.replace('`', "\\`"))
    }
}

/// A string as an R literal.
fn r_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// A number as R writes it.
fn r_number(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        format!("{v:?}")
    }
}

/// `code` as TRUE/FALSE with Stata's rule: not zero is true, missing included.
fn truth(code: &str, kind: Kind) -> Result<String, Untranslated> {
    match kind {
        Kind::Logical => Ok(code.to_string()),
        Kind::Number | Kind::Unknown => Ok(format!("(is.na({code}) | {code} != 0)")),
        Kind::Text => Err(no(
            "a string is used as true or false, which Stata does not allow",
        )),
    }
}

/// `code` as a number (TRUE/FALSE become 1/0).
fn number(code: &str, kind: Kind) -> Result<String, Untranslated> {
    match kind {
        Kind::Logical => Ok(format!("as.numeric({code})")),
        Kind::Number | Kind::Unknown => Ok(code.to_string()),
        Kind::Text => Err(no("a string is used where Stata wants a number")),
    }
}

fn is_missing_literal(e: &Expr) -> bool {
    matches!(e, Expr::Number(v) if is_missing(*v))
}

fn plain_number(e: &Expr) -> Option<f64> {
    match e {
        Expr::Number(v) if !is_missing(*v) => Some(*v),
        Expr::Unary(UnaryOp::Neg, inner) => plain_number(inner).map(|v| -v),
        _ => None,
    }
}

impl Translator<'_> {
    fn caveat(&mut self, text: impl Into<String>) {
        let t = text.into();
        if !self.caveats.contains(&t) {
            self.caveats.push(t);
        }
    }

    fn expr(&mut self, e: &Expr) -> Result<(String, Kind), Untranslated> {
        match e {
            Expr::Number(v) if is_missing(*v) => {
                if missing_code(*v).is_some_and(|c| c > 0) {
                    self.caveat(
                        "Stata's extended missing values (.a to .z) are all NA in R, so they are \
                         no longer told apart",
                    );
                }
                Ok(("NA_real_".into(), Kind::Number))
            }
            Expr::Number(v) => Ok((r_number(*v), Kind::Number)),
            Expr::Text(s) => Ok((r_text(s), Kind::Text)),
            Expr::Name(n) => Ok(match n.as_str() {
                "_n" => ("dplyr::row_number()".into(), Kind::Number),
                "_N" => ("dplyr::n()".into(), Kind::Number),
                "_pi" => ("pi".into(), Kind::Number),
                _ => (r_name(n), (self.kind_of)(n).unwrap_or(Kind::Unknown)),
            }),
            Expr::Subscript(name, index) => self.subscript(name, index),
            Expr::Unary(op, inner) => {
                let (code, kind) = self.expr(inner)?;
                match op {
                    UnaryOp::Plus => Ok((code, kind)),
                    UnaryOp::Neg => Ok((format!("-{}", wrap(&number(&code, kind)?)), Kind::Number)),
                    UnaryOp::Not => Ok((format!("!{}", wrap(&truth(&code, kind)?)), Kind::Logical)),
                }
            }
            Expr::Binary(op, a, b) => self.binary(*op, a, b),
            Expr::Call(name, args) => self.call(name, args),
        }
    }

    fn subscript(&mut self, name: &str, index: &Expr) -> Result<(String, Kind), Untranslated> {
        if name == "_b" || name == "_se" {
            return Err(no(format!(
                "{name}[] (a coefficient of the last estimation) is translated with the \
                 estimation commands, not yet"
            )));
        }
        let var = r_name(name);
        let kind = (self.kind_of)(name).unwrap_or(Kind::Unknown);
        let lag = |e: &Expr| -> Option<i64> {
            // _n - k and _n + k, as written
            match e {
                Expr::Name(n) if n == "_n" => Some(0),
                Expr::Binary(op @ (BinaryOp::Sub | BinaryOp::Add), l, r) => {
                    match (l.as_ref(), plain_number(r)) {
                        (Expr::Name(n), Some(k)) if n == "_n" && k.fract() == 0.0 => {
                            let k = k as i64;
                            Some(if *op == BinaryOp::Sub { -k } else { k })
                        }
                        _ => None,
                    }
                }
                _ => None,
            }
        };
        if let Some(shift) = lag(index) {
            return Ok((
                match shift {
                    0 => var,
                    s if s < 0 => format!("dplyr::lag({var}, {})", -s),
                    s => format!("dplyr::lead({var}, {s})"),
                },
                kind,
            ));
        }
        match index {
            Expr::Number(v) if *v == 1.0 => Ok((format!("dplyr::first({var})"), kind)),
            Expr::Name(n) if n == "_N" => Ok((format!("dplyr::last({var})"), kind)),
            other => {
                let (i, ik) = self.expr(other)?;
                // an index outside the rows is missing in both
                Ok((format!("{var}[{}]", number(&i, ik)?), kind))
            }
        }
    }

    fn binary(&mut self, op: BinaryOp, a: &Expr, b: &Expr) -> Result<(String, Kind), Untranslated> {
        let (ac, ak) = self.expr(a)?;
        let (bc, bk) = self.expr(b)?;
        let text = ak == Kind::Text || bk == Kind::Text;
        match op {
            BinaryOp::And | BinaryOp::Or => {
                let sym = if op == BinaryOp::And { "&" } else { "|" };
                Ok((
                    format!("({} {sym} {})", truth(&ac, ak)?, truth(&bc, bk)?),
                    Kind::Logical,
                ))
            }
            BinaryOp::Add if text => {
                if ak != Kind::Text && ak != Kind::Unknown
                    || bk != Kind::Text && bk != Kind::Unknown
                {
                    return Err(no(
                        "a string is added to a number, which Stata does not allow",
                    ));
                }
                Ok((format!("paste0({ac}, {bc})"), Kind::Text))
            }
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Pow => {
                let sym = match op {
                    BinaryOp::Add => "+",
                    BinaryOp::Sub => "-",
                    BinaryOp::Mul => "*",
                    _ => "^",
                };
                Ok((
                    format!("({} {sym} {})", number(&ac, ak)?, number(&bc, bk)?),
                    Kind::Number,
                ))
            }
            BinaryOp::Div => {
                let num = number(&ac, ak)?;
                let den = number(&bc, bk)?;
                // x / 0 is missing in Stata, Inf in R
                let den = match plain_number(b) {
                    Some(v) if v != 0.0 => den,
                    _ => format!("dplyr::na_if({den}, 0)"),
                };
                Ok((format!("({num} / {den})"), Kind::Number))
            }
            BinaryOp::Eq
            | BinaryOp::Ne
            | BinaryOp::Lt
            | BinaryOp::Le
            | BinaryOp::Gt
            | BinaryOp::Ge => {
                if text {
                    return Ok((format!("({ac} {} {bc})", r_op(op)), Kind::Logical));
                }
                self.compare(op, a, b, number(&ac, ak)?, number(&bc, bk)?, ak, bk)
            }
        }
    }

    /// A numeric comparison with Stata's missing rule: missing is larger than
    /// every number.
    #[allow(clippy::too_many_arguments)]
    fn compare(
        &mut self,
        op: BinaryOp,
        a: &Expr,
        b: &Expr,
        ac: String,
        bc: String,
        ak: Kind,
        bk: Kind,
    ) -> Result<(String, Kind), Untranslated> {
        // x op . (or . op x): only "is it missing" is asked
        if is_missing_literal(b) || is_missing_literal(a) {
            let (x, op) = if is_missing_literal(b) {
                (ac, op)
            } else {
                (bc, flip(op))
            };
            let code = match op {
                BinaryOp::Eq | BinaryOp::Ge => format!("is.na({x})"),
                BinaryOp::Ne | BinaryOp::Lt => format!("!is.na({x})"),
                BinaryOp::Le => "TRUE".into(),
                _ => "FALSE".into(),
            };
            return Ok((code, Kind::Logical));
        }
        // a side that cannot be missing (a number written out, or a TRUE/FALSE)
        let a_sure = plain_number(a).is_some() || ak == Kind::Logical;
        let b_sure = plain_number(b).is_some() || bk == Kind::Logical;
        let sym = r_op(op);
        let code = match (a_sure, b_sure) {
            (true, true) => format!("({ac} {sym} {bc})"),
            (false, true) => missing_one_side(&ac, op, &bc),
            (true, false) => missing_one_side(&bc, flip(op), &ac),
            (false, false) => {
                format!("(dplyr::coalesce({ac}, Inf) {sym} dplyr::coalesce({bc}, Inf))")
            }
        };
        Ok((code, Kind::Logical))
    }

    fn call(&mut self, name: &str, args: &[Expr]) -> Result<(String, Kind), Untranslated> {
        let want = |n: usize| -> Result<(), Untranslated> {
            if args.len() == n {
                Ok(())
            } else {
                Err(no(format!("{name}() takes {n} arguments here")))
            }
        };
        let arg = |i: usize| -> Result<&Expr, Untranslated> {
            args.get(i)
                .ok_or_else(|| no(format!("{name}() needs more arguments")))
        };
        let mut nums = Vec::new();
        let simple: Option<&str> = match name {
            "abs" | "exp" | "sqrt" | "sin" | "cos" | "tan" | "asin" | "acos" | "atan" | "floor"
            | "log10" => Some(name),
            "ln" | "log" => Some("log"),
            "ceil" => Some("ceiling"),
            "int" => Some("trunc"),
            "atan2" => Some("atan2"),
            _ => None,
        };
        if let Some(r) = simple {
            want(if r == "atan2" { 2 } else { 1 })?;
            for a in args {
                let (c, k) = self.expr(a)?;
                nums.push(number(&c, k)?);
            }
            return Ok((format!("{r}({})", nums.join(", ")), Kind::Number));
        }
        match name {
            "round" => {
                let (x, xk) =
                    self.expr(args.first().ok_or_else(|| no("round() needs a value"))?)?;
                let x = number(&x, xk)?;
                match args.get(1) {
                    None => Ok((
                        format!("floor({x} + 0.5 + abs({x}) * 4 * .Machine$double.eps)"),
                        Kind::Number,
                    )),
                    Some(y) if args.len() == 2 => {
                        if plain_number(y) == Some(0.0) {
                            return Ok((x, Kind::Number));
                        }
                        if plain_number(y).is_none() {
                            self.caveat("round(x, y) with y not written out: y = 0 gives NA in R, x in Stata");
                        }
                        let (yc, yk) = self.expr(y)?;
                        let y = number(&yc, yk)?;
                        let q = format!("{x} / {y}");
                        Ok((
                            format!(
                                "(floor({q} + 0.5 + abs({q}) * 4 * .Machine$double.eps) * {y})"
                            ),
                            Kind::Number,
                        ))
                    }
                    _ => Err(no("round() takes 1 or 2 arguments")),
                }
            }
            "mod" => {
                want(2)?;
                let (x, xk) = self.expr(arg(0)?)?;
                let (y, yk) = self.expr(arg(1)?)?;
                let (x, y) = (number(&x, xk)?, number(&y, yk)?);
                Ok(match plain_number(arg(1)?) {
                    Some(v) if v > 0.0 => (format!("({x} %% {y})"), Kind::Number),
                    _ => (
                        format!("ifelse({y} > 0, {x} %% {y}, NA_real_)"),
                        Kind::Number,
                    ),
                })
            }
            "max" | "min" => {
                if args.is_empty() {
                    return Err(no(format!("{name}() needs values")));
                }
                for a in args {
                    let (c, k) = self.expr(a)?;
                    nums.push(number(&c, k)?);
                }
                let f = if name == "max" { "pmax" } else { "pmin" };
                Ok((
                    format!("{f}({}, na.rm = TRUE)", nums.join(", ")),
                    Kind::Number,
                ))
            }
            "missing" | "mi" => {
                if args.is_empty() {
                    return Err(no("missing() needs values"));
                }
                let mut parts = Vec::new();
                for a in args {
                    let (c, k) = self.expr(a)?;
                    parts.push(match k {
                        Kind::Number | Kind::Logical => format!("is.na({c})"),
                        Kind::Text => format!("{c} %in% c(\"\", NA)"),
                        Kind::Unknown => format!("(is.na({c}) | {c} %in% \"\")"),
                    });
                }
                Ok((format!("({})", parts.join(" | ")), Kind::Logical))
            }
            "inrange" => {
                want(3)?;
                let (z, zk) = self.expr(arg(0)?)?;
                let (a, ak) = self.expr(arg(1)?)?;
                let (b, bk) = self.expr(arg(2)?)?;
                if zk == Kind::Text || ak == Kind::Text || bk == Kind::Text {
                    return Ok((format!("({a} <= {z} & {z} <= {b})"), Kind::Logical));
                }
                let lo = match plain_number(arg(1)?) {
                    Some(_) => a,
                    None => format!("dplyr::coalesce({a}, Inf)"),
                };
                let hi = match plain_number(arg(2)?) {
                    Some(_) => b,
                    None => format!("dplyr::coalesce({b}, Inf)"),
                };
                Ok((
                    format!("(!is.na({z}) & {lo} <= {z} & {z} <= {hi})"),
                    Kind::Logical,
                ))
            }
            "inlist" => {
                if args.len() < 2 {
                    return Err(no("inlist() needs a value and a list"));
                }
                let (z, _) = self.expr(arg(0)?)?;
                let mut rest = Vec::new();
                for a in args.iter().skip(1) {
                    rest.push(self.expr(a)?.0);
                }
                Ok((format!("({z} %in% c({}))", rest.join(", ")), Kind::Logical))
            }
            "cond" => {
                let (c, ck) =
                    self.expr(args.first().ok_or_else(|| no("cond() needs a condition"))?)?;
                let (a, ak) =
                    self.expr(args.get(1).ok_or_else(|| no("cond() needs two values"))?)?;
                let (b, _) =
                    self.expr(args.get(2).ok_or_else(|| no("cond() needs two values"))?)?;
                let kind = if ak == Kind::Logical {
                    Kind::Number
                } else {
                    ak
                };
                let (a, b) = if ak == Kind::Logical {
                    (format!("as.numeric({a})"), format!("as.numeric({b})"))
                } else {
                    (a, b)
                };
                match args.get(3) {
                    None if args.len() == 3 => {
                        Ok((format!("ifelse({}, {a}, {b})", truth(&c, ck)?), kind))
                    }
                    Some(m) if args.len() == 4 => {
                        let (m, _) = self.expr(m)?;
                        let c = number(&c, ck)?;
                        Ok((
                            format!("ifelse(is.na({c}), {m}, ifelse({c} != 0, {a}, {b}))"),
                            kind,
                        ))
                    }
                    _ => Err(no("cond() takes 3 or 4 arguments")),
                }
            }
            "sum" => {
                want(1)?;
                let (x, k) = self.expr(arg(0)?)?;
                Ok((
                    format!("cumsum(dplyr::coalesce({}, 0))", number(&x, k)?),
                    Kind::Number,
                ))
            }
            "strlen" | "ustrlen" | "length" => self.text_fn("nchar", args, Kind::Number),
            "upper" | "strupper" | "ustrupper" => self.text_fn("toupper", args, Kind::Text),
            "lower" | "strlower" | "ustrlower" => self.text_fn("tolower", args, Kind::Text),
            "trim" | "strtrim" | "ustrtrim" => self.text_fn("trimws", args, Kind::Text),
            "ltrim" | "ustrltrim" => {
                want(1)?;
                let (s, _) = self.expr(arg(0)?)?;
                Ok((format!("trimws({s}, which = \"left\")"), Kind::Text))
            }
            "rtrim" | "ustrrtrim" => {
                want(1)?;
                let (s, _) = self.expr(arg(0)?)?;
                Ok((format!("trimws({s}, which = \"right\")"), Kind::Text))
            }
            "substr" | "usubstr" => {
                want(3)?;
                let (s, _) = self.expr(arg(0)?)?;
                match (plain_number(arg(1)?), arg(2)?) {
                    (Some(n1), e) if n1 >= 1.0 && n1.fract() == 0.0 => {
                        let start = n1 as i64;
                        if is_missing_literal(e) {
                            return Ok((format!("substr({s}, {start}, nchar({s}))"), Kind::Text));
                        }
                        match plain_number(e) {
                            Some(n2) if n2 >= 1.0 && n2.fract() == 0.0 => Ok((
                                format!("substr({s}, {start}, {})", start + n2 as i64 - 1),
                                Kind::Text,
                            )),
                            _ => Err(no("substr() with a length that is not a whole number written out is not translated yet")),
                        }
                    }
                    _ => Err(no("substr() counting from the end, or from a position that is not written out, is not translated yet")),
                }
            }
            "strpos" | "ustrpos" => {
                want(2)?;
                let (s, _) = self.expr(arg(0)?)?;
                let (t, _) = self.expr(arg(1)?)?;
                Ok((
                    // as.numeric drops regexpr's match attributes
                    format!("pmax(as.numeric(regexpr({t}, {s}, fixed = TRUE)), 0)"),
                    Kind::Number,
                ))
            }
            "subinstr" | "usubinstr" => {
                want(4)?;
                let (s, _) = self.expr(arg(0)?)?;
                let (a, _) = self.expr(arg(1)?)?;
                let (b, _) = self.expr(arg(2)?)?;
                if is_missing_literal(arg(3)?) {
                    Ok((format!("gsub({a}, {b}, {s}, fixed = TRUE)"), Kind::Text))
                } else if plain_number(arg(3)?) == Some(1.0) {
                    Ok((format!("sub({a}, {b}, {s}, fixed = TRUE)"), Kind::Text))
                } else {
                    Err(no("subinstr() replacing a count other than 1 or all (.) is not translated yet"))
                }
            }
            "regexm" | "ustrregexm" => {
                want(2)?;
                let (s, _) = self.expr(arg(0)?)?;
                let (re, _) = self.expr(arg(1)?)?;
                self.caveat("regular expressions: Stata's and R's dialects differ in some details");
                Ok((format!("grepl({re}, {s})"), Kind::Logical))
            }
            "real" => {
                want(1)?;
                let (s, _) = self.expr(arg(0)?)?;
                Ok((format!("suppressWarnings(as.numeric({s}))"), Kind::Number))
            }
            "string" | "strofreal" if args.len() == 1 => {
                let (x, k) = self.expr(arg(0)?)?;
                self.caveat(
                    "string(): R writes numbers with up to 15 digits, Stata with its %9.0g format",
                );
                Ok((format!("as.character({})", number(&x, k)?), Kind::Text))
            }
            "mdy" => {
                want(3)?;
                let mut v = Vec::new();
                for a in args {
                    let (c, k) = self.expr(a)?;
                    v.push(number(&c, k)?);
                }
                let [m, d, y] = v.as_slice() else {
                    return Err(no("mdy() takes 3 arguments here"));
                };
                Ok((
                    format!(
                        "as.numeric(as.Date(ISOdate({y}, {m}, {d})) - as.Date(\"1960-01-01\"))"
                    ),
                    Kind::Number,
                ))
            }
            "year" | "month" | "day" | "dow" | "doy" => {
                want(1)?;
                let (d, k) = self.expr(arg(0)?)?;
                let lt = format!(
                    "as.POSIXlt(as.Date({}, origin = \"1960-01-01\"))",
                    number(&d, k)?
                );
                let code = match name {
                    "year" => format!("({lt}$year + 1900)"),
                    "month" => format!("({lt}$mon + 1)"),
                    "day" => format!("{lt}$mday"),
                    "dow" => format!("{lt}$wday"),
                    _ => format!("({lt}$yday + 1)"),
                };
                Ok((code, Kind::Number))
            }
            "date" => match (args.first(), args.get(1), args.len()) {
                (Some(Expr::Text(s)), Some(Expr::Text(mask)), 2) => {
                    let v = parse_date(s, mask, None);
                    if is_missing(v) {
                        Ok(("NA_real_".into(), Kind::Number))
                    } else {
                        Ok((r_number(v), Kind::Number))
                    }
                }
                _ => Err(no(
                    "date() of text that is only known when the file runs is not translated yet",
                )),
            },
            _ => Err(no(format!("the function {name}() is not translated yet"))),
        }
    }

    fn text_fn(
        &mut self,
        r: &str,
        args: &[Expr],
        kind: Kind,
    ) -> Result<(String, Kind), Untranslated> {
        let [one] = args else {
            return Err(no("this string function takes one argument"));
        };
        let (s, _) = self.expr(one)?;
        Ok((format!("{r}({s})"), kind))
    }
}

fn wrap(code: &str) -> String {
    if code.starts_with('(')
        || code
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
    {
        code.to_string()
    } else {
        format!("({code})")
    }
}

fn r_op(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Eq => "==",
        BinaryOp::Ne => "!=",
        BinaryOp::Lt => "<",
        BinaryOp::Le => "<=",
        BinaryOp::Gt => ">",
        _ => ">=",
    }
}

/// The same comparison with its sides swapped (`a < b` is `b > a`).
fn flip(op: BinaryOp) -> BinaryOp {
    match op {
        BinaryOp::Lt => BinaryOp::Gt,
        BinaryOp::Le => BinaryOp::Ge,
        BinaryOp::Gt => BinaryOp::Lt,
        BinaryOp::Ge => BinaryOp::Le,
        other => other,
    }
}

/// `x op c` where only `x` can be missing, and missing is larger than `c`.
fn missing_one_side(x: &str, op: BinaryOp, c: &str) -> String {
    let sym = r_op(op);
    match op {
        // missing is larger: true for > and >=, and for != (missing is not c)
        BinaryOp::Gt | BinaryOp::Ge | BinaryOp::Ne => format!("(is.na({x}) | {x} {sym} {c})"),
        _ => format!("(!is.na({x}) & {x} {sym} {c})"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn none(_: &str) -> Option<Kind> {
        None
    }

    fn code(text: &str, use_: Use) -> String {
        match translate(text, &none, use_) {
            Ok(r) => r.code,
            Err(e) => panic!("{text}: {}", e.reason),
        }
    }

    #[test]
    fn comparisons_follow_the_missing_rule() {
        assert_eq!(code("x > 5", Use::Condition), "(is.na(x) | x > 5)");
        assert_eq!(code("x < 5", Use::Condition), "(!is.na(x) & x < 5)");
        assert_eq!(code("5 < x", Use::Condition), "(is.na(x) | x > 5)");
        assert_eq!(code("x == 1", Use::Condition), "(!is.na(x) & x == 1)");
        assert_eq!(code("x != 1", Use::Condition), "(is.na(x) | x != 1)");
        assert_eq!(code("x < .", Use::Condition), "!is.na(x)");
        assert_eq!(code("x == .", Use::Condition), "is.na(x)");
        assert_eq!(code("x >= .", Use::Condition), "is.na(x)");
        assert_eq!(
            code("x > y", Use::Condition),
            "(dplyr::coalesce(x, Inf) > dplyr::coalesce(y, Inf))"
        );
    }

    #[test]
    fn true_and_false_are_numbers_in_values_and_any_non_zero_is_true() {
        assert_eq!(code("x > 5", Use::Value), "as.numeric((is.na(x) | x > 5))");
        assert_eq!(
            code("foreign", Use::Condition),
            "(is.na(foreign) | foreign != 0)"
        );
        assert_eq!(
            code("!foreign", Use::Condition),
            "!(is.na(foreign) | foreign != 0)"
        );
        assert_eq!(
            code("a & b > 1", Use::Condition),
            "((is.na(a) | a != 0) & (is.na(b) | b > 1))"
        );
    }

    #[test]
    fn arithmetic_division_and_strings() {
        assert_eq!(code("x / y", Use::Value), "(x / dplyr::na_if(y, 0))");
        assert_eq!(code("x / 2", Use::Value), "(x / 2)");
        assert_eq!(code("2 ^ x - 1", Use::Value), "((2 ^ x) - 1)");
        let kinds = |n: &str| (n == "s").then_some(Kind::Text);
        let r = translate("s + \"x\"", &kinds, Use::Value).map(|r| r.code);
        assert_eq!(r, Ok("paste0(s, \"x\")".to_string()));
        assert_eq!(code("s == \"a\"", Use::Condition), "(s == \"a\")");
        assert!(translate("s + 1", &kinds, Use::Value).is_err());
        assert!(translate("s", &kinds, Use::Condition).is_err());
    }

    #[test]
    fn row_numbers_lags_and_names() {
        assert_eq!(code("_n", Use::Value), "dplyr::row_number()");
        assert_eq!(code("x[_n-1]", Use::Value), "dplyr::lag(x, 1)");
        assert_eq!(code("x[_n+2]", Use::Value), "dplyr::lead(x, 2)");
        assert_eq!(code("x[1]", Use::Value), "dplyr::first(x)");
        assert_eq!(code("x[_N]", Use::Value), "dplyr::last(x)");
        assert_eq!(code("_x + 1", Use::Value), "(`_x` + 1)");
        assert_eq!(r_name("if"), "`if`");
        assert!(translate("_b[x]", &none, Use::Value).is_err());
    }

    #[test]
    fn functions_follow_statas_rules() {
        assert_eq!(
            code("max(a, b, .)", Use::Value),
            "pmax(a, b, NA_real_, na.rm = TRUE)"
        );
        assert_eq!(code("mod(x, 3)", Use::Value), "(x %% 3)");
        assert_eq!(
            code("mod(x, y)", Use::Value),
            "ifelse(y > 0, x %% y, NA_real_)"
        );
        assert_eq!(
            code("round(x)", Use::Value),
            "floor(x + 0.5 + abs(x) * 4 * .Machine$double.eps)"
        );
        assert_eq!(code("round(x, 0)", Use::Value), "x");
        assert_eq!(
            code("inrange(x, 1, 5)", Use::Condition),
            "(!is.na(x) & 1 <= x & x <= 5)"
        );
        assert_eq!(
            code("inlist(x, 1, 2, .)", Use::Condition),
            "(x %in% c(1, 2, NA_real_))"
        );
        assert_eq!(
            code("missing(x)", Use::Condition),
            "((is.na(x) | x %in% \"\"))"
        );
        assert_eq!(
            code("cond(x > 1, 1, 2)", Use::Value),
            "ifelse((is.na(x) | x > 1), 1, 2)"
        );
        assert_eq!(code("sum(x)", Use::Value), "cumsum(dplyr::coalesce(x, 0))");
        assert_eq!(code("substr(s, 2, 3)", Use::Value), "substr(s, 2, 4)");
        assert_eq!(
            code("subinstr(s, \"a\", \"b\", .)", Use::Value),
            "gsub(\"a\", \"b\", s, fixed = TRUE)"
        );
        // a date written out becomes its number (days since 1 January 1960)
        assert_eq!(code("td(01jan1960) + 1", Use::Value), "(0 + 1)");
        assert_eq!(code("date(\"2 Jan 1960\", \"DMY\")", Use::Value), "1");
    }

    #[test]
    fn what_is_not_translated_says_why_and_caveats_are_given() {
        for (text, why) in [
            ("foo(x)", "foo() is not translated"),
            ("substr(s, -2, 1)", "from the end"),
            ("date(s, \"DMY\")", "only known when the file runs"),
            ("x +", "Stata would not read it"),
            ("_se[x]", "estimation"),
        ] {
            let r = translate(text, &none, Use::Value);
            assert!(
                r.as_ref().is_err_and(|e| e.reason.contains(why)),
                "{text}: {r:?}"
            );
        }
        let r = translate("x == .a", &none, Use::Condition);
        assert!(r.is_ok_and(|r| r.caveats.iter().any(|c| c.contains("extended missing"))));
        let r = translate("regexm(s, \"^a\")", &none, Use::Condition);
        assert!(r.is_ok_and(|r| r.caveats.iter().any(|c| c.contains("dialects"))));
    }
}
