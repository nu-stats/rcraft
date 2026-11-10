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
//! - The extended missing values `.a` to `.z` come after `.` (numbers < `.` <
//!   `.a` < ... < `.z`). R holds them as haven's tagged NAs, which `use` reads
//!   from a `.dta` file; a comparison with a missing value written out goes by
//!   its place in that order (`stata_mv`), two values that may both be missing
//!   by `stata_compare`. As in Stata 19, a copy (`x`, `x[_n-1]`, `L.x`,
//!   `cond()`), `float()`, `int()`, `round()`, `floor()` and `ceil()` keep the
//!   tag, and arithmetic and other functions give `.` (`stata_untag`).
//! - `round()` goes halfway up with a tiny allowance (as StatCraft found Stata
//!   does: 2.345 rounds to 2.35), `mod(x, y)` is missing for `y <= 0`, and
//!   `inrange()` is false for a missing first argument.
//!
//! Where R cannot say the same thing, the expression is not translated and the
//! reason is given ([`Untranslated`]); where the R is right but loses a detail
//! (regular-expression dialects, how `string()` formats numbers), a caveat
//! goes with it. The rules for
//! `round`, `mod`, `max`/`min`, `missing`, `inrange`, `inlist`, `cond` and `sum`
//! follow StatCraft's implementations, which are checked against Stata's output.

use crate::lang::expr::{parse, BinaryOp, Expr, UnaryOp};
use crate::values::datetime::parse_date;
use crate::values::missing::{is_missing, missing_code, missing_label};

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
    /// How many of Stata's random draws it takes (`runiform()`, ...).
    pub draws: usize,
    /// The R helpers its code calls (`stata_dates`, ...).
    pub helpers: Vec<&'static str>,
    /// It writes an extended missing value (`.a` to `.z`) as a value, which
    /// R keeps as a tagged NA (`haven::tagged_na`).
    pub tagged: bool,
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
    translate_with(
        text,
        kind_of,
        &|which: &str, name: &str| {
            if which == "ts" {
                return Err(no(format!(
                    "{name}: time-series operators need the time tsset gave, which only the commands of a do-file carry"
                )));
            }
            let shown = if which.starts_with('_') {
                format!("{which}[{name}]")
            } else {
                format!("{which}({name})")
            };
            Err(no(format!(
                "{shown} reads the last estimation or r-class results, which only the commands of a do-file carry"
            )))
        },
        use_,
    )
}

/// A lookup for the values a command left: `("e", "N")`, `("r", "mean")`,
/// `("_b", "x")`, `("_se", "2:x")`; it gives the R and its kind, or why not.
pub type Returned<'a> = dyn Fn(&str, &str) -> Result<(String, Kind), Untranslated> + 'a;

/// [`translate`] with `e()`, `r()`, `_b[]` and `_se[]` read through `returned`.
pub fn translate_with(
    text: &str,
    kind_of: &dyn Fn(&str) -> Option<Kind>,
    returned: &Returned,
    use_: Use,
) -> Result<RExpr, Untranslated> {
    translate_drawing(text, kind_of, returned, use_, None, false)
}

/// [`translate_with`] where Stata's random draws may be taken: `rows` is the
/// R for the rows that draw, a count (`dplyr::n()`, `1`) or a condition, one
/// draw a row in order (Stata draws only for the rows an `if` or `in`
/// selects). The draws come from `stata_rng` (`data_r::random`); with `None`,
/// a random function is refused.
///
/// `tags` says the data may hold extended missing values (`.a` to `.z`, as
/// haven's tagged NAs): a file was read or one was stored. Then a comparison
/// with a missing value tells them apart and a computed value is made `.`, as
/// Stata's arithmetic makes it; without, the R stays as short as before (an
/// extended value written out in the expression turns it on).
pub fn translate_drawing(
    text: &str,
    kind_of: &dyn Fn(&str) -> Option<Kind>,
    returned: &Returned,
    use_: Use,
    rows: Option<&str>,
    tags: bool,
) -> Result<RExpr, Untranslated> {
    let tree = parse(text).map_err(|e| no(format!("Stata would not read it: {}", e.message)))?;
    let tagged = has_extended(&tree);
    let mut t = Translator {
        kind_of,
        returned,
        caveats: Vec::new(),
        rows,
        draws: 0,
        draw_call: None,
        helpers: Vec::new(),
        bytes_ok: false,
        tags: tags || tagged,
    };
    let (code, kind) = t.expr(&tree)?;
    let (code, kind) = match use_ {
        Use::Condition => (truth(&code, kind)?, Kind::Logical),
        Use::Value if kind == Kind::Logical => (format!("as.numeric({code})"), Kind::Number),
        // arithmetic and functions give `.` where Stata's do; R would keep the tag
        Use::Value if t.tags && kind == Kind::Number && !t.carries(&tree) && reads_data(&tree) => {
            t.helpers.push("stata_untag");
            (format!("stata_untag({code})"), kind)
        }
        Use::Value => (code, kind),
    };
    // the draw is taken once: where the R writes it more than once (the
    // missing rule of a comparison, round()), it is drawn first and named
    let code = match &t.draw_call {
        Some(call) if code.matches(DRAW).count() == 1 => code.replace(DRAW, call),
        Some(call) => format!("local({{{DRAW} <- {call}; {code}}})"),
        None => code,
    };
    let mut caveats = t.caveats;
    caveats.dedup();
    Ok(RExpr {
        code,
        kind,
        caveats,
        draws: t.draws,
        helpers: t.helpers,
        tagged: tagged && use_ == Use::Value,
    })
}

struct Translator<'a> {
    kind_of: &'a dyn Fn(&str) -> Option<Kind>,
    returned: &'a Returned<'a>,
    caveats: Vec<String>,
    rows: Option<&'a str>,
    draws: usize,
    /// The R of the expression's random draw, written as [`DRAW`] in its code.
    draw_call: Option<String>,
    helpers: Vec<&'static str>,
    /// Inside subinstr()'s search and replacement, which Stata works on as
    /// bytes: there char(128) to char(255) are single bytes R can match too.
    bytes_ok: bool,
    /// The data may hold extended missing values ([`translate_drawing`]).
    tags: bool,
}

/// Stata's date and time functions of numbers, the R helper each is
/// (`data_r/dates.rs`), and how many arguments each takes.
const DATE_FUNCTIONS: &[(&str, usize)] = &[
    ("year", 1),
    ("month", 1),
    ("day", 1),
    ("dow", 1),
    ("doy", 1),
    ("week", 1),
    ("quarter", 1),
    ("halfyear", 1),
    ("wofd", 1),
    ("mofd", 1),
    ("qofd", 1),
    ("hofd", 1),
    ("yofd", 1),
    ("dofw", 1),
    ("dofm", 1),
    ("dofq", 1),
    ("dofh", 1),
    ("dofy", 1),
    ("cofd", 1),
    ("dofc", 1),
    ("hh", 1),
    ("mm", 1),
    ("ss", 1),
    ("hours", 1),
    ("minutes", 1),
    ("seconds", 1),
    ("msofhours", 1),
    ("msofminutes", 1),
    ("msofseconds", 1),
    ("isleapyear", 1),
    ("yw", 2),
    ("ym", 2),
    ("yq", 2),
    ("yh", 2),
    ("mdy", 3),
    ("hms", 3),
    ("dhms", 4),
    ("mdyhms", 6),
    // datetime/C, with the leap seconds
    ("Cofc", 1),
    ("cofC", 1),
    ("Cofd", 1),
    ("dofC", 1),
    ("hhC", 1),
    ("mmC", 1),
    ("ssC", 1),
    ("Chms", 3),
    ("Cdhms", 4),
    ("Cmdyhms", 6),
];

/// Strings read as dates against a mask: Stata's name and the helper's.
const DATE_READERS: &[(&str, &str)] = &[
    ("date", "date"),
    ("daily", "date"),
    ("clock", "clock"),
    ("Clock", "Clock"),
    ("weekly", "weekly"),
    ("monthly", "monthly"),
    ("quarterly", "quarterly"),
    ("halfyearly", "halfyearly"),
    ("yearly", "yearly"),
];

/// datetime/C, which counts leap seconds: not translated yet.
const LEAP_SECOND_FUNCTIONS: &[&str] = &[];

/// The name a random draw goes by in an expression's R until it is placed.
const DRAW: &str = ".stata_draw";

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
pub fn r_text(s: &str) -> String {
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
pub fn r_number(v: f64) -> String {
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

/// A missing value written out, by its place in Stata's order: 1 for `.`,
/// 2 to 27 for `.a` to `.z` (0 would be a number; `stata_mv` in R).
fn missing_place(e: &Expr) -> Option<u32> {
    match e {
        Expr::Number(v) => missing_code(*v).map(|c| u32::from(c) + 1),
        _ => None,
    }
}

/// The last place, `.z`'s.
const LAST_PLACE: u32 = 27;

/// `a op b` for two places in Stata's order.
fn holds(a: u32, op: BinaryOp, b: u32) -> bool {
    match op {
        BinaryOp::Eq => a == b,
        BinaryOp::Ne => a != b,
        BinaryOp::Lt => a < b,
        BinaryOp::Le => a <= b,
        BinaryOp::Gt => a > b,
        _ => a >= b,
    }
}

/// An extended missing value (`.a` to `.z`) written out anywhere in `e`.
fn has_extended(e: &Expr) -> bool {
    match e {
        Expr::Number(_) => missing_place(e).is_some_and(|p| p > 1),
        Expr::Subscript(_, i) => has_extended(i),
        Expr::Unary(_, a) => has_extended(a),
        Expr::Binary(_, a, b) => has_extended(a) || has_extended(b),
        Expr::Call(_, args) => args.iter().any(has_extended),
        _ => false,
    }
}

/// Whether `e` reads the data (a variable), so its R may carry a tag along.
fn reads_data(e: &Expr) -> bool {
    match e {
        Expr::Name(n) => !matches!(n.as_str(), "_n" | "_N" | "_pi" | "_rc"),
        Expr::Subscript(n, i) => !matches!(n.as_str(), "_b" | "_se" | "_coef") || reads_data(i),
        Expr::Unary(_, a) => reads_data(a),
        Expr::Binary(_, a, b) => reads_data(a) || reads_data(b),
        // e(), r(), scalars and matrices hold no tags
        Expr::Call(f, _) if matches!(f.as_str(), "e" | "r" | "scalar" | "c") => false,
        Expr::Call(_, args) => args.iter().any(reads_data),
        _ => false,
    }
}

impl Translator<'_> {
    fn caveat(&mut self, text: impl Into<String>) {
        let t = text.into();
        if !self.caveats.contains(&t) {
            self.caveats.push(t);
        }
    }

    /// Whether the value of `e` may be an extended missing value in Stata 19:
    /// a variable, a row of one (`x[_n-1]`, `L.x`), one written out, and what
    /// keeps it (`cond()`'s values, `float()`, `int()`, `round()`, `floor()`,
    /// `ceil()`). Arithmetic and the other functions give `.` (`-x`, `x + 0`,
    /// `abs()`, `sqrt()`, `exp()`, `mod()`, `max()`, ...).
    fn carries(&self, e: &Expr) -> bool {
        match e {
            Expr::Number(_) => missing_place(e).is_some_and(|p| p > 1),
            Expr::Name(n) => {
                !matches!(n.as_str(), "_n" | "_N" | "_pi" | "_rc")
                    && match (self.kind_of)(n) {
                        Some(k) => k != Kind::Text,
                        // a scalar holds no tag
                        None => (self.returned)("scalar?", n).is_err(),
                    }
            }
            Expr::Subscript(n, _) => !matches!(n.as_str(), "_b" | "_se" | "_coef"),
            Expr::Unary(UnaryOp::Plus, inner) => self.carries(inner),
            Expr::Call(f, args) => match f.as_str() {
                "cond" => args.iter().skip(1).any(|a| self.carries(a)),
                "float" | "int" | "round" | "floor" | "ceil" => {
                    args.first().is_some_and(|a| self.carries(a))
                }
                _ => false,
            },
            _ => false,
        }
    }

    /// The R of `e`, made `.` where Stata's value has no tag but R's may.
    fn untagged(&mut self, e: &Expr, code: String) -> String {
        if self.tags && !self.carries(e) && reads_data(e) {
            self.helpers.push("stata_untag");
            format!("stata_untag({code})")
        } else {
            code
        }
    }

    fn expr(&mut self, e: &Expr) -> Result<(String, Kind), Untranslated> {
        match e {
            Expr::Number(v) if is_missing(*v) => match missing_code(*v) {
                // kept as haven's tagged NA, which tells it apart from `.`
                Some(c) if c > 0 => Ok((
                    format!(
                        "haven::tagged_na({})",
                        r_text(missing_label(c).trim_start_matches('.'))
                    ),
                    Kind::Number,
                )),
                _ => Ok(("NA_real_".into(), Kind::Number)),
            },
            Expr::Number(v) => Ok((r_number(*v), Kind::Number)),
            Expr::Text(s) => Ok((r_text(s), Kind::Text)),
            Expr::Name(n)
                if n.split_once('.')
                    .is_some_and(|(ops, _)| crate::lang::expr::is_ts_ops(ops)) =>
            {
                // the session knows the time and panel tsset gave
                let r = (self.returned)("ts", n)?;
                self.helpers.push("stata_ts");
                Ok(r)
            }
            Expr::Name(n) => Ok(match n.as_str() {
                "_n" => ("dplyr::row_number()".into(), Kind::Number),
                "_N" => ("dplyr::n()".into(), Kind::Number),
                "_pi" => ("pi".into(), Kind::Number),
                // what the last capture came to: 0, or missing after an error
                "_rc" => {
                    self.caveat(
                        "_rc is 0 when the line under capture ran and missing when it failed: R's errors are not Stata's, so its error numbers (111, 198, ...) are not known",
                    );
                    (
                        "get0(\"stata_rc\", envir = globalenv(), ifnotfound = 0)".into(),
                        Kind::Number,
                    )
                }
                // a variable before a scalar of the same name, as in Stata
                _ => match (self.kind_of)(n) {
                    Some(k) => (r_name(n), k),
                    None => (self.returned)("scalar?", n).unwrap_or((r_name(n), Kind::Unknown)),
                },
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
        // a coefficient of the last model, or its standard error
        if matches!(name, "_b" | "_se" | "_coef") {
            return match index {
                Expr::Text(key) => (self.returned)(name, key),
                _ => Err(no(format!("{name}[] needs a coefficient name"))),
            };
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
            // a number times a string repeats it (3*"ab" is "ababab"); a count written out
            BinaryOp::Mul if text => {
                let (s, n) = if ak == Kind::Text { (&ac, b) } else { (&bc, a) };
                match plain_number(n) {
                    Some(k) if k >= 0.0 && k.fract() == 0.0 && (ak == Kind::Text) != (bk == Kind::Text) => {
                        Ok((format!("strrep({s}, {k})"), Kind::Text))
                    }
                    _ => Err(no("a string repeated a number of times that is not a whole number written out is not translated yet")),
                }
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
    /// every number, and `.` < `.a` < ... < `.z`.
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
        // two missing values written out: their order decides
        if let (Some(pa), Some(pb)) = (missing_place(a), missing_place(b)) {
            let code = if holds(pa, op, pb) { "TRUE" } else { "FALSE" };
            return Ok((code.into(), Kind::Logical));
        }
        // x op . (or x op .d): by x's place in the order, 0 for a number
        let lit = match (missing_place(a), missing_place(b)) {
            (_, Some(k)) => Some((ac.clone(), a, op, k)),
            (Some(k), _) => Some((bc.clone(), b, flip(op), k)),
            _ => None,
        };
        if let Some((x, xe, op, k)) = lit {
            // a computed value is a number or `.`; a variable may be any
            let top = if self.tags && self.carries(xe) {
                LAST_PLACE
            } else {
                1
            };
            let all: Vec<bool> = (0..=top).map(|p| holds(p, op, k)).collect();
            let code = if all.iter().all(|t| *t) {
                "TRUE".to_string()
            } else if all.iter().all(|t| !*t) {
                "FALSE".to_string()
            } else if all.iter().enumerate().all(|(p, t)| *t == (p > 0)) {
                format!("is.na({x})")
            } else if all.iter().enumerate().all(|(p, t)| *t == (p == 0)) {
                format!("!is.na({x})")
            } else {
                self.helpers.push("stata_mv");
                format!("(stata_mv({x}) {} {k})", r_op(op))
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
            // both may be missing and one may be .a to .z: by the order
            (false, false) if self.tags && (self.carries(a) || self.carries(b)) => {
                let ac = self.untagged(a, ac);
                let bc = self.untagged(b, bc);
                self.helpers.push("stata_mv");
                self.helpers.push("stata_compare");
                format!("stata_compare({ac}, {}, {bc})", r_text(sym))
            }
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
            | "sign" | "log10" => Some(name),
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
            // the nearest single-precision value, as Stata's float()
            "float" => {
                want(1)?;
                let (x, xk) = self.expr(arg(0)?)?;
                let x = number(&x, xk)?;
                self.helpers.push("stata_float");
                Ok((format!("stata_float({x})"), Kind::Number))
            }
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
            // R's qnorm and pnorm give Stata 19's doubles bit for bit
            // (checked on 20,000 values, tails and subnormals included);
            // invnormal() outside 0 < p < 1 is missing in Stata
            "invnormal" => {
                want(1)?;
                let (p, pk) = self.expr(arg(0)?)?;
                let p = number(&p, pk)?;
                Ok((
                    format!(
                        "(\\(q) replace(q, !is.finite(q), NA_real_))(suppressWarnings(stats::qnorm({p})))"
                    ),
                    Kind::Number,
                ))
            }
            // c(): the constants of Stata's storage types (read off Stata 19
            // in %21x; the same on every machine). Settings and the session's
            // own values (c(N), c(pwd), c(rngstate), ...) are not translated.
            "c" => {
                want(1)?;
                let Some(Expr::Name(what)) = args.first() else {
                    return Err(no("c() takes a name"));
                };
                let v = match what.as_str() {
                    "pi" => "pi",
                    "maxbyte" => "100",
                    "minbyte" => "-127",
                    "maxint" => "32740",
                    "minint" => "-32767",
                    "maxlong" => "2147483620",
                    "minlong" => "-2147483647",
                    "maxfloat" => "0x1.fffffep126",
                    "minfloat" => "-0x1.fffffep126",
                    "maxdouble" => "0x1.fffffffffffffp1022",
                    "mindouble" => "-0x1.fffffffffffffp1022",
                    "smallestdouble" => "0x1p-1022",
                    "epsfloat" => "0x1p-23",
                    "epsdouble" => "0x1p-52",
                    "maxstrvarlen" => "2045",
                    "maxstrlvarlen" => "2000000000",
                    other => {
                        return Err(no(format!(
                            "c({other}) is not translated yet: only the constants of the storage types (c(maxlong), c(epsdouble), c(pi), ...) are"
                        )))
                    }
                };
                Ok((v.to_string(), Kind::Number))
            }
            "normal" => {
                want(1)?;
                let (z, zk) = self.expr(arg(0)?)?;
                Ok((format!("stats::pnorm({})", number(&z, zk)?), Kind::Number))
            }
            // invlogit(x) = e^x/(1+e^x); logit(p) = ln(p/(1-p)), missing outside 0 < p < 1
            "invlogit" => {
                want(1)?;
                let (z, zk) = self.expr(arg(0)?)?;
                Ok((format!("stats::plogis({})", number(&z, zk)?), Kind::Number))
            }
            "logit" => {
                want(1)?;
                let (p, pk) = self.expr(arg(0)?)?;
                let p = number(&p, pk)?;
                Ok((format!("(function(p) ifelse(!is.na(p) & p > 0 & p < 1, stats::qlogis(p), NA_real_))({p})"), Kind::Number))
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
                // R's %in% finds any NA for any other, so where .a to .z may be
                // held, a list with more than numbers is z == a | z == b | ...
                let z0 = arg(0)?;
                if self.tags
                    && self.carries(z0)
                    && args.iter().skip(1).any(|a| plain_number(a).is_none())
                {
                    let (z, zk) = self.expr(z0)?;
                    let z = number(&z, zk)?;
                    let mut any = Vec::new();
                    for a in args.iter().skip(1) {
                        let (c, k) = self.expr(a)?;
                        let c = number(&c, k)?;
                        any.push(self.compare(BinaryOp::Eq, z0, a, z.clone(), c, zk, k)?.0);
                    }
                    return Ok((format!("({})", any.join(" | ")), Kind::Logical));
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
                // cond() gives its value itself, a tag kept: where one value
                // may carry one, a computed value beside it is made `.`
                let ae = args.get(1).ok_or_else(|| no("cond() needs two values"))?;
                let be = args.get(2).ok_or_else(|| no("cond() needs two values"))?;
                let keeps = self.tags && args.iter().skip(1).any(|a| self.carries(a));
                let (a, ak) = self.expr(ae)?;
                let (b, bk) = self.expr(be)?;
                let one = |me: &mut Self, x: &Expr, code: String, k: Kind| {
                    if keeps && k == Kind::Number {
                        me.untagged(x, code)
                    } else {
                        code
                    }
                };
                let a = one(self, ae, a, ak);
                let b = one(self, be, b, bk);
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
                    Some(me) if args.len() == 4 => {
                        let (m, mk) = self.expr(me)?;
                        let m = one(self, me, m, mk);
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
            // length() and strlen() count bytes in Stata ("café" is 5), ustrlen()
            // characters
            "strlen" | "length" => {
                want(1)?;
                let (s, _) = self.expr(arg(0)?)?;
                Ok((format!("nchar({s}, type = \"bytes\")"), Kind::Number))
            }
            "ustrlen" => self.text_fn("nchar", args, Kind::Number),
            // words are what spaces and tabs separate; quotes are not special
            // (Stata 19: wordcount(`"a "b c" d"') is 4)
            "wordcount" => {
                want(1)?;
                let (s, _) = self.expr(arg(0)?)?;
                Ok((
                    format!("as.numeric(lengths(regmatches({s}, gregexpr(\"[^ \\t]+\", {s}))))"),
                    Kind::Number,
                ))
            }
            "upper" | "strupper" | "ustrupper" => self.text_fn("toupper", args, Kind::Text),
            "lower" | "strlower" | "ustrlower" => self.text_fn("tolower", args, Kind::Text),
            // Stata's trim, strtrim, ltrim, ... take spaces only; the ustr ones
            // tabs, returns and new lines as well (read off Stata 19)
            "trim" | "strtrim" | "ltrim" | "strltrim" | "rtrim" | "strrtrim" | "ustrtrim"
            | "ustrltrim" | "ustrrtrim" => {
                want(1)?;
                let (s, _) = self.expr(arg(0)?)?;
                let which = match name.trim_start_matches("ustr").trim_start_matches("str") {
                    "ltrim" => ", which = \"left\"",
                    "rtrim" => ", which = \"right\"",
                    _ => "",
                };
                let space = if name.starts_with("ustr") {
                    ""
                } else {
                    ", whitespace = \" \""
                };
                Ok((format!("trimws({s}{which}{space})"), Kind::Text))
            }
            // char(n): the ASCII character; "" when missing or past 255 (Stata 19)
            "char" => {
                want(1)?;
                // a code written out that is a single byte: declined here, not in R
                if let Some(k) = plain_number(arg(0)?).map(f64::trunc) {
                    // a single byte where subinstr() looks for or puts bytes
                    if self.bytes_ok && (128.0..=255.0).contains(&k) {
                        return Ok((format!("rawToChar(as.raw({k}))"), Kind::Text));
                    }
                    if k == 0.0 || (128.0..=255.0).contains(&k) {
                        return Err(no(format!(
                            "char({k}) is a single byte, which R's text does not hold as Stata does; not translated yet"
                        )));
                    }
                }
                let (n, nk) = self.expr(arg(0)?)?;
                let n = number(&n, nk)?;
                self.helpers.push("stata_char");
                Ok((format!("stata_char({n})"), Kind::Text))
            }
            // strmatch(s, pattern): * any characters, ? one, the rest as written
            "strmatch" => {
                want(2)?;
                let (s, sk) = self.expr(arg(0)?)?;
                let (p, pk) = self.expr(arg(1)?)?;
                // a variable of a type not known here is checked in R
                if matches!(sk, Kind::Number | Kind::Logical)
                    || matches!(pk, Kind::Number | Kind::Logical)
                {
                    return Err(no("strmatch() takes two strings (Stata: type mismatch)"));
                }
                self.helpers.push("stata_strmatch");
                Ok((format!("stata_strmatch({s}, {p})"), Kind::Number))
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
                // subinstr() works on bytes in Stata: so does R with useBytes,
                // and char(128) to char(255) can be searched for and put in
                let bytes = name == "subinstr";
                self.bytes_ok = bytes;
                let a = self.expr(arg(1)?);
                let b = self.expr(arg(2)?);
                self.bytes_ok = false;
                let ((a, _), (b, _)) = (a?, b?);
                let ub = if bytes { ", useBytes = TRUE" } else { "" };
                if is_missing_literal(arg(3)?) {
                    Ok((format!("gsub({a}, {b}, {s}, fixed = TRUE{ub})"), Kind::Text))
                } else if plain_number(arg(3)?) == Some(1.0) {
                    Ok((format!("sub({a}, {b}, {s}, fixed = TRUE{ub})"), Kind::Text))
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
            // string(x [, "fmt"]): x in the format (%9.0g without one), its leading
            // blanks taken off and the trailing kept, as Stata 19 writes it
            "string" | "strofreal" if args.len() == 1 || args.len() == 2 => {
                let (x, k) = self.expr(arg(0)?)?;
                let fmt = match args.get(1) {
                    None => "%9.0g".to_string(),
                    Some(Expr::Text(f)) if stata_number_format(f) => f.clone(),
                    Some(_) => {
                        return Err(no(
                            "string(x, fmt) is translated with a numeric format written out, as \"%4.1f\"",
                        ))
                    }
                };
                self.helpers.push("stata_fmt");
                Ok((
                    format!(
                        "vapply({}, function(v) sub(\"^ +\", \"\", stata_fmt(v, {})), \"\", USE.NAMES = FALSE)",
                        number(&x, k)?,
                        r_text(&fmt)
                    ),
                    Kind::Text,
                ))
            }
            // e(sample): the rows of the last model, kept in R as stata_esample
            // (the translator lets it through only while those rows are unmoved)
            "e" => match args {
                [Expr::Name(n)] if n == "sample" => Ok(("stata_esample".into(), Kind::Logical)),
                [Expr::Name(n)] => (self.returned)("e", n),
                _ => Err(no("e() takes one name, as e(N)")),
            },
            "scalar" => match args {
                [Expr::Name(n)] => (self.returned)("scalar", n),
                _ => Err(no("scalar() takes one scalar name")),
            },
            "r" => match args {
                [Expr::Name(n)] => (self.returned)("r", n),
                _ => Err(no("r() takes one name, as r(mean)")),
            },
            // el(e(V), i, j): one element of a matrix e() or r() keeps;
            // missing outside it, as in Stata
            // colsof(e(b)), rowsof(r(V)): the size of a matrix e() or r() keeps
            "colsof" | "rowsof" => {
                want(1)?;
                let m = match arg(0)? {
                    Expr::Call(w, a) if (w == "e" || w == "r") && a.len() == 1 => match a.first() {
                        Some(Expr::Name(n)) if n != "sample" => {
                            (self.returned)(&format!("{w}#matrix"), n)?;
                            format!("stata_{w}[[{}]]", r_text(n))
                        }
                        _ => return Err(no(format!("{name}() of this matrix is not translated yet"))),
                    },
                    // a matrix the do-file made
                    Expr::Name(n) => {
                        self.helpers.push("stata_matrix");
                        (self.returned)("m#matrix", n)?.0
                    }
                    _ => {
                        return Err(no(format!(
                            "{name}() is translated for the matrices e() and r() keep (colsof(e(b))); not here yet"
                        )))
                    }
                };
                let f = if name == "colsof" { "NCOL" } else { "NROW" };
                Ok((format!("as.numeric({f}(as.matrix({m})))"), Kind::Number))
            }
            // A[i, j] (the parser's __matel): an element of a matrix the
            // do-file made, by number or by its row and column names
            "__matel" => {
                want(3)?;
                let Some(Expr::Text(n)) = args.first() else {
                    return Err(no("a matrix element needs the matrix's name"));
                };
                self.helpers.push("stata_matrix");
                let m = (self.returned)("m#matrix", n)?.0;
                let index = |me: &mut Self, e: &Expr| -> Result<String, Untranslated> {
                    let (v, k) = me.expr(e)?;
                    Ok(if k == Kind::Text { v } else { number(&v, k)? })
                };
                let i = index(self, arg(1)?)?;
                let j = index(self, arg(2)?)?;
                Ok((
                    format!("stata_matel({m}, {i}, {j}, {})", r_text(n)),
                    Kind::Number,
                ))
            }
            "el" => {
                want(3)?;
                let m = match arg(0)? {
                    // through the same check as e(name): what is held must be known
                    Expr::Call(w, a) if (w == "e" || w == "r") && a.len() == 1 => match a.first() {
                        Some(Expr::Name(n)) if n != "sample" => {
                            (self.returned)(&format!("{w}#matrix"), n)?;
                            format!("stata_{w}[[{}]]", r_text(n))
                        }
                        _ => return Err(no("el() of this matrix is not translated yet")),
                    },
                    Expr::Name(n) => {
                        self.helpers.push("stata_matrix");
                        (self.returned)("m#matrix", n)?.0
                    }
                    _ => {
                        return Err(no(
                            "el() is translated for the matrices e() and r() keep (el(e(V), 1, 1)); not here yet",
                        ))
                    }
                };
                let (i, ik) = self.expr(arg(1)?)?;
                let (j, jk) = self.expr(arg(2)?)?;
                let (i, j) = (number(&i, ik)?, number(&j, jk)?);
                Ok((
                    format!(
                        "(\\(m, i, j) if (is.null(m) || is.na(i) || is.na(j) || i < 1 || j < 1 || i > NROW(m) || j > NCOL(m)) NA_real_ else unname(as.matrix(m)[floor(i), floor(j)]))({m}, {i}, {j})"
                    ),
                    Kind::Number,
                ))
            }
            // numbers from a matrix the do-file made or one e() or r() keeps,
            // named (Stata takes no expression there): trace() and det() of a
            // matrix that is not square are missing, as in Stata 19
            "trace" | "det" | "diag0cnt" | "issymmetric" | "matmissing" => {
                want(1)?;
                let m = self.matrix_arg(name, arg(0)?)?;
                Ok((format!("stata_m{name}({m})"), Kind::Number))
            }
            "mreldif" => {
                want(2)?;
                let a = self.matrix_arg(name, arg(0)?)?;
                let b = self.matrix_arg(name, arg(1)?)?;
                Ok((format!("stata_mmreldif({a}, {b})"), Kind::Number))
            }
            "colnumb" | "rownumb" => {
                want(2)?;
                let m = self.matrix_arg(name, arg(0)?)?;
                let (s, k) = self.expr(arg(1)?)?;
                if k != Kind::Text {
                    return Err(no(format!(
                        "{name}() takes a name as text (Stata: type mismatch)"
                    )));
                }
                let side = if name == "colnumb" {
                    "colnames"
                } else {
                    "rownames"
                };
                Ok((
                    format!("as.numeric(match({s}, {side}(stata_mnamed({m}))))"),
                    Kind::Number,
                ))
            }
            "runiform" | "uniform" | "rnormal" | "rgamma" | "rexponential" | "rlogistic"
            | "rweibull" | "rweibullph" | "rcauchy" | "rlaplace" | "rpareto" | "rgeometric"
            | "rigaussian" | "rchi2" | "runiformint" | "rbinomial" | "rnbinomial"
            | "rhypergeometric" | "rt" | "rbeta" => self.draw(name, args),
            n if DATE_FUNCTIONS.iter().any(|(f, _)| *f == n) => self.date_fn(n, args),
            n if DATE_READERS.iter().any(|(f, _)| *f == n) => self.date_reader(n, args),
            n if LEAP_SECOND_FUNCTIONS.contains(&n) => Err(no(format!(
                "{n}() counts leap seconds (datetime/C), which is not translated yet; \
                 datetime/c (clock(), cofd(), ...) is"
            ))),
            _ => Err(no(format!("the function {name}() is not translated yet"))),
        }
    }

    /// A date or time function of numbers (P2C.14): the R helper of the
    /// same name, `stata_mdy()` and so on, with Stata's missing values.
    /// A matrix named as a function's argument: one the do-file made, or one
    /// e() or r() keeps.
    fn matrix_arg(&mut self, f: &str, e: &Expr) -> Result<String, Untranslated> {
        self.helpers.push("stata_matrix");
        match e {
            Expr::Call(w, a) if (w == "e" || w == "r") && a.len() == 1 => match a.first() {
                Some(Expr::Name(n)) if n != "sample" => {
                    (self.returned)(&format!("{w}#matrix"), n)?;
                    Ok(format!("stata_{w}[[{}]]", r_text(n)))
                }
                _ => Err(no(format!("{f}() of this matrix is not translated yet"))),
            },
            Expr::Name(n) => Ok((self.returned)("m#matrix", n)?.0),
            _ => Err(no(format!(
                "{f}() takes a matrix's name (Stata: an expression there is not found, r(111))"
            ))),
        }
    }

    fn date_fn(&mut self, name: &str, args: &[Expr]) -> Result<(String, Kind), Untranslated> {
        let n = DATE_FUNCTIONS
            .iter()
            .find(|(f, _)| *f == name)
            .map_or(0, |(_, n)| *n);
        if args.len() != n {
            return Err(no(format!(
                "{name}() takes {n} argument{} (Stata: invalid syntax)",
                if n == 1 { "" } else { "s" }
            )));
        }
        let mut vals = Vec::new();
        for a in args {
            let (c, k) = self.expr(a)?;
            if k == Kind::Text {
                return Err(no(format!("{name}() of a string (Stata: type mismatch)")));
            }
            vals.push(number(&c, k)?);
        }
        self.helpers.push("stata_dates");
        Ok((format!("stata_{name}({})", vals.join(", ")), Kind::Number))
    }

    /// `date(s, mask [, topyear])`, `clock()`, `monthly()` and the others:
    /// strings read against Stata's mask in R. A date written out is read
    /// here.
    fn date_reader(&mut self, name: &str, args: &[Expr]) -> Result<(String, Kind), Untranslated> {
        let helper = DATE_READERS
            .iter()
            .find(|(f, _)| *f == name)
            .map_or("date", |(_, h)| *h);
        if !(2..=3).contains(&args.len()) {
            return Err(no(format!(
                "{name}() takes a string, a mask and a top year (Stata: invalid syntax)"
            )));
        }
        if let [Expr::Text(s), Expr::Text(mask)] = args {
            if helper == "date" {
                let v = parse_date(s, mask, None);
                return Ok((
                    if is_missing(v) {
                        "NA_real_".into()
                    } else {
                        r_number(v)
                    },
                    Kind::Number,
                ));
            }
        }
        let mut vals = Vec::new();
        for (i, a) in args.iter().enumerate() {
            let (c, k) = self.expr(a)?;
            let text = i < 2;
            if text && k == Kind::Number || !text && k == Kind::Text {
                return Err(no(format!(
                    "{name}() with these arguments (Stata: type mismatch)"
                )));
            }
            vals.push(if text { c } else { number(&c, k)? });
        }
        self.helpers.push("stata_dates");
        Ok((format!("stata_{helper}({})", vals.join(", ")), Kind::Number))
    }

    /// One of Stata's random draws, from `stata_rng` (P2C.11): the R helper
    /// takes the rows that draw first, then Stata's arguments.
    fn draw(&mut self, name: &str, args: &[Expr]) -> Result<(String, Kind), Untranslated> {
        let Some(rows) = self.rows else {
            return Err(no(format!(
                "{name}() draws random numbers; Stata's own draws are translated in generate, \
                 replace, expand, keep, drop, display, scalar and macro values, not here yet"
            )));
        };
        if self.draws > 0 {
            return Err(no(format!(
                "{name}() is a second random draw in one expression: Stata takes them in turn \
                 for each row; not translated yet"
            )));
        }
        self.draws += 1;
        let n = args.len();
        let fits = match name {
            "runiform" => n == 0 || n == 2,
            "uniform" => n == 0,
            "rnormal" | "rlogistic" => n <= 2,
            "rexponential" | "rgeometric" | "rchi2" | "rt" => n == 1,
            "rweibull" | "rweibullph" => n == 2 || n == 3,
            "rhypergeometric" => n == 3,
            _ => n == 2,
        };
        let r = if name == "uniform" {
            "stata_runiform".to_string()
        } else {
            format!("stata_{name}")
        };
        if !fits {
            return Err(no(format!("{name}() with {n} arguments is not translated")));
        }
        // rbeta() with equal shapes above 1 is not Stata's yet; shapes written out say so now
        if let [Expr::Number(a), Expr::Number(b)] = args {
            if name == "rbeta" && a == b && *a > 1.0 && *a <= 1e5 {
                return Err(no(
                    "rbeta() with equal shapes above 1 is not translated yet: Stata's method there was not matched",
                ));
            }
        }
        let mut parts = vec![rows.to_string()];
        // rlogistic(s) is the scale, about a mean of 0
        if name == "rlogistic" && n == 1 {
            parts.push("0".into());
        }
        for a in args {
            let (c, k) = self.expr(a)?;
            parts.push(number(&c, k)?);
        }
        self.draw_call = Some(format!("{r}({})", parts.join(", ")));
        Ok((DRAW.to_string(), Kind::Number))
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

/// A Stata numeric format written out: `%[-][0]w.d` then `f`, `g` or `e`, a
/// `c` after `f` or `g` for commas.
fn stata_number_format(f: &str) -> bool {
    let digits = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
    let Some(rest) = f.strip_prefix('%') else {
        return false;
    };
    let rest = rest.strip_prefix('-').unwrap_or(rest);
    let (rest, comma) = match rest.strip_suffix('c') {
        Some(r) => (r, true),
        None => (rest, false),
    };
    let Some((body, last)) = rest
        .char_indices()
        .last()
        .and_then(|(i, c)| Some((rest.get(..i)?, c)))
    else {
        return false;
    };
    if !matches!(last, 'f' | 'g' | 'e') || (comma && last == 'e') {
        return false;
    }
    let Some((w, d)) = body.split_once('.') else {
        return false;
    };
    digits(w) && digits(d)
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
    fn the_normal_distribution_is_rs_own() {
        assert_eq!(
            code("invnormal(p)", Use::Value),
            "(\\(q) replace(q, !is.finite(q), NA_real_))(suppressWarnings(stats::qnorm(p)))"
        );
        assert_eq!(code("normal(z / 2)", Use::Value), "stats::pnorm((z / 2))");
        assert_eq!(code("c(maxlong) + 1", Use::Value), "(2147483620 + 1)");
        assert!(translate("c(pwd)", &none, Use::Value).is_err());
        // the draw inside is taken once a row
        let r = translate_drawing(
            "invnormal(runiform())",
            &none,
            &|_, _| Err(no("")),
            Use::Value,
            Some("n"),
            false,
        );
        let r = r.map(|r| (r.code, r.draws)).map_err(|e| e.reason);
        assert_eq!(
            r,
            Ok((
                "(\\(q) replace(q, !is.finite(q), NA_real_))(suppressWarnings(stats::qnorm(stata_runiform(n))))".into(),
                1
            ))
        );
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
            "gsub(\"a\", \"b\", s, fixed = TRUE, useBytes = TRUE)"
        );
        // a date written out becomes its number (days since 1 January 1960)
        assert_eq!(code("td(01jan1960) + 1", Use::Value), "(0 + 1)");
        assert_eq!(code("date(\"2 Jan 1960\", \"DMY\")", Use::Value), "1");
        // on a column, Stata's rules run in R (data_r/dates.rs)
        assert_eq!(
            code("date(s, \"DMY\", 2020)", Use::Value),
            "stata_date(s, \"DMY\", 2020)"
        );
        assert_eq!(
            code("monthly(s, \"YM\")", Use::Value),
            "stata_monthly(s, \"YM\")"
        );
        assert_eq!(
            code("year(d) + mdy(1, 2, y)", Use::Value),
            "(stata_year(d) + stata_mdy(1, 2, y))"
        );
        let r = translate("dofm(m)", &none, Use::Value);
        assert!(r.is_ok_and(|r| r.helpers == vec!["stata_dates"]));
    }

    #[test]
    fn what_is_not_translated_says_why_and_caveats_are_given() {
        for (text, why) in [
            ("foo(x)", "foo() is not translated"),
            ("substr(s, -2, 1)", "from the end"),
            ("year(s, 2)", "takes 1 argument"),
            ("x +", "Stata would not read it"),
            ("_se[x]", "estimation"),
        ] {
            let r = translate(text, &none, Use::Value);
            assert!(
                r.as_ref().is_err_and(|e| e.reason.contains(why)),
                "{text}: {r:?}"
            );
        }
        let r = translate("regexm(s, \"^a\")", &none, Use::Condition);
        assert!(r.is_ok_and(|r| r.caveats.iter().any(|c| c.contains("dialects"))));
    }

    /// With the data's tags on (a .dta file was read).
    fn tagged(text: &str, use_: Use) -> String {
        match translate_drawing(text, &none, &|_, _| Err(no("")), use_, None, true) {
            Ok(r) => r.code,
            Err(e) => panic!("{text}: {}", e.reason),
        }
    }

    #[test]
    fn extended_missing_values_compare_by_their_place() {
        // Stata 19's counts over 1 . .a .d .w .z (plan/stata/extended-missing.md)
        assert_eq!(tagged("x == .d", Use::Condition), "(stata_mv(x) == 5)");
        assert_eq!(tagged("x == .", Use::Condition), "(stata_mv(x) == 1)");
        assert_eq!(tagged("x != .", Use::Condition), "(stata_mv(x) != 1)");
        assert_eq!(tagged("x > .", Use::Condition), "(stata_mv(x) > 1)");
        assert_eq!(tagged("x <= .", Use::Condition), "(stata_mv(x) <= 1)");
        // where only "is it missing" is asked, the short form stays
        assert_eq!(tagged("x < .", Use::Condition), "!is.na(x)");
        assert_eq!(tagged("x >= .", Use::Condition), "is.na(x)");
        assert_eq!(tagged("x <= .z", Use::Condition), "TRUE");
        assert_eq!(tagged(".d < x", Use::Condition), "(stata_mv(x) > 5)");
        assert_eq!(tagged(".d == .d", Use::Condition), "TRUE");
        assert_eq!(tagged(". < .a", Use::Condition), "TRUE");
        // a computed value is a number or `.`
        assert_eq!(tagged("(x + 0) == .", Use::Condition), "is.na((x + 0))");
        assert_eq!(tagged("-x == .d", Use::Condition), "FALSE");
        assert_eq!(
            tagged("round(x) == .d", Use::Condition),
            "(stata_mv(floor(x + 0.5 + abs(x) * 4 * .Machine$double.eps)) == 5)"
        );
        // two values that may both be missing
        assert_eq!(
            tagged("x < y", Use::Condition),
            "stata_compare(x, \"<\", y)"
        );
        assert_eq!(
            tagged("x + 1 == y", Use::Condition),
            "stata_compare(stata_untag((x + 1)), \"==\", y)"
        );
        assert_eq!(tagged("x > 5", Use::Condition), "(is.na(x) | x > 5)");
        // without tags, as short as before; an extended value written out turns them on
        assert_eq!(code("x == .", Use::Condition), "is.na(x)");
        assert_eq!(code("x == .d", Use::Condition), "(stata_mv(x) == 5)");
        assert_eq!(
            tagged("inlist(x, .d, 1)", Use::Condition),
            "((stata_mv(x) == 5) | (!is.na(x) & x == 1))"
        );
        assert_eq!(
            tagged("inlist(x, 1, 2)", Use::Condition),
            "(x %in% c(1, 2))"
        );
    }

    #[test]
    fn tags_are_kept_by_copies_and_dropped_by_arithmetic() {
        let r = translate(".d", &none, Use::Value);
        assert!(r.is_ok_and(|r| r.tagged
            && r.code == "haven::tagged_na(\"d\")"
            && r.caveats.is_empty()));
        assert_eq!(tagged("x", Use::Value), "x");
        assert_eq!(tagged("x[_n-1]", Use::Value), "dplyr::lag(x, 1)");
        assert_eq!(tagged("float(x)", Use::Value), "stata_float(x)");
        assert_eq!(tagged("int(x)", Use::Value), "trunc(x)");
        assert_eq!(tagged("x + 0", Use::Value), "stata_untag((x + 0))");
        assert_eq!(tagged("abs(x)", Use::Value), "stata_untag(abs(x))");
        assert_eq!(tagged("-x", Use::Value), "stata_untag(-x)");
        assert_eq!(tagged("_n + 1", Use::Value), "(dplyr::row_number() + 1)");
        assert_eq!(
            tagged("cond(x > 5, x, 0)", Use::Value),
            "ifelse((is.na(x) | x > 5), x, 0)"
        );
        assert_eq!(
            tagged("cond(g, x + 1, y)", Use::Value),
            "ifelse((is.na(g) | g != 0), stata_untag((x + 1)), y)"
        );
        assert_eq!(code("x + 0", Use::Value), "(x + 0)");
    }
}
