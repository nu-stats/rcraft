//! Expressions whose value is known without the data: `local n = 2*3`,
//! `if `i' > 1 {`, `while `k' <= 5 {`. Only numbers, text, the operators and
//! a few functions whose rules are plain are worked out here; anything that
//! reads the data, a stored result or a scalar (or a function not listed) is
//! left to R when the do-file runs, so nothing is guessed.

use crate::lang::expr::{BinaryOp, Expr, UnaryOp};
use crate::values::missing::{is_missing, SYSMISS};

use super::format::g;
use super::macros::words;

/// A value: a number (missing values as the doubles Stata uses) or text.
#[derive(Debug, Clone, PartialEq)]
pub enum Val {
    Num(f64),
    Str(String),
}

impl Val {
    /// As a macro keeps it.
    pub fn macro_text(&self) -> String {
        match self {
            Val::Num(x) => g(*x, 18),
            Val::Str(s) => s.clone(),
        }
    }
}

/// Why a value is not worked out here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotHere {
    /// It needs the data, a stored result or a function left to R.
    Later,
    /// Stata stops with this error.
    Stata(String),
}

fn mismatch() -> NotHere {
    NotHere::Stata("type mismatch".into())
}

fn num(x: f64) -> Val {
    Val::Num(if x.is_finite() { x } else { SYSMISS })
}

fn truth(b: bool) -> Val {
    Val::Num(if b { 1.0 } else { 0.0 })
}

/// The value of `e`, if it is known without R.
pub fn eval(e: &Expr) -> Result<Val, NotHere> {
    match e {
        Expr::Number(x) => Ok(Val::Num(*x)),
        Expr::Text(s) => Ok(Val::Str(s.clone())),
        Expr::Name(n) if n == "_pi" => Ok(Val::Num(std::f64::consts::PI)),
        Expr::Name(_) | Expr::Subscript(..) => Err(NotHere::Later),
        Expr::Unary(op, a) => {
            let a = eval(a)?;
            match (op, a) {
                (UnaryOp::Plus, Val::Num(x)) => Ok(Val::Num(x)),
                (UnaryOp::Neg, Val::Num(x)) => Ok(Val::Num(if is_missing(x) { x } else { -x })),
                (UnaryOp::Not, Val::Num(x)) => Ok(truth(x == 0.0)),
                _ => Err(mismatch()),
            }
        }
        Expr::Binary(op, a, b) => binary(*op, eval(a)?, eval(b)?),
        Expr::Call(name, args) => call(name, args),
    }
}

fn binary(op: BinaryOp, a: Val, b: Val) -> Result<Val, NotHere> {
    use BinaryOp::*;
    match (op, a, b) {
        (Add, Val::Str(a), Val::Str(b)) => Ok(Val::Str(a + &b)),
        (Mul, Val::Str(s), Val::Num(n)) | (Mul, Val::Num(n), Val::Str(s)) => {
            let times = if is_missing(n) || n < 1.0 {
                0
            } else {
                n.trunc() as usize
            };
            if times.saturating_mul(s.len()) > 1_000_000 {
                return Err(NotHere::Later);
            }
            Ok(Val::Str(s.repeat(times)))
        }
        (Eq, Val::Str(a), Val::Str(b)) => Ok(truth(a == b)),
        (Ne, Val::Str(a), Val::Str(b)) => Ok(truth(a != b)),
        (Lt, Val::Str(a), Val::Str(b)) => Ok(truth(a < b)),
        (Le, Val::Str(a), Val::Str(b)) => Ok(truth(a <= b)),
        (Gt, Val::Str(a), Val::Str(b)) => Ok(truth(a > b)),
        (Ge, Val::Str(a), Val::Str(b)) => Ok(truth(a >= b)),
        (op, Val::Num(a), Val::Num(b)) => {
            let missing = is_missing(a) || is_missing(b);
            Ok(match op {
                Or => truth(a != 0.0 || b != 0.0),
                And => truth(a != 0.0 && b != 0.0),
                // missing values are larger than every number, as Stata orders them
                Eq => truth(a == b),
                Ne => truth(a != b),
                Lt => truth(a < b),
                Le => truth(a <= b),
                Gt => truth(a > b),
                Ge => truth(a >= b),
                _ if missing => Val::Num(SYSMISS),
                Add => num(a + b),
                Sub => num(a - b),
                Mul => num(a * b),
                Div if b == 0.0 => Val::Num(SYSMISS),
                Div => num(a / b),
                Pow => num(a.powf(b)),
            })
        }
        _ => Err(mismatch()),
    }
}

fn call(name: &str, args: &[Expr]) -> Result<Val, NotHere> {
    // c(pi) is a constant; every other c() is the running Stata's
    if name == "c" {
        return match args {
            [Expr::Name(n)] if n == "pi" => Ok(Val::Num(std::f64::consts::PI)),
            _ => Err(NotHere::Later),
        };
    }
    let vals = args.iter().map(eval).collect::<Result<Vec<_>, _>>()?;
    let n = |i: usize| match vals.get(i) {
        Some(Val::Num(x)) => Ok(*x),
        Some(Val::Str(_)) => Err(mismatch()),
        None => Err(NotHere::Later),
    };
    let s = |i: usize| match vals.get(i) {
        Some(Val::Str(x)) => Ok(x.clone()),
        Some(Val::Num(_)) => Err(mismatch()),
        None => Err(NotHere::Later),
    };
    let one = |f: fn(f64) -> f64| -> Result<Val, NotHere> {
        if vals.len() != 1 {
            return Err(NotHere::Later);
        }
        let x = n(0)?;
        Ok(if is_missing(x) {
            Val::Num(SYSMISS)
        } else {
            num(f(x))
        })
    };
    match (name, vals.len()) {
        ("abs", _) => one(f64::abs),
        ("ceil", _) => one(f64::ceil),
        ("floor", _) => one(f64::floor),
        ("int" | "trunc", _) => one(f64::trunc),
        ("sqrt", _) => {
            if n(0)? < 0.0 {
                return Ok(Val::Num(SYSMISS));
            }
            one(f64::sqrt)
        }
        ("exp", _) => one(f64::exp),
        ("ln" | "log", _) | ("log10", _) => {
            if n(0)? <= 0.0 {
                return Ok(Val::Num(SYSMISS));
            }
            one(if name == "log10" { f64::log10 } else { f64::ln })
        }
        // seen in Stata 19: round(2.5) is 3, round(3.14159, .01) is 3.14;
        // negative values are left to R
        ("round", 1 | 2) => {
            let x = n(0)?;
            let unit = if vals.len() == 2 { n(1)? } else { 1.0 };
            if is_missing(x) || is_missing(unit) {
                return Ok(Val::Num(SYSMISS));
            }
            if x < 0.0 || unit <= 0.0 {
                return Err(NotHere::Later);
            }
            Ok(num((x / unit + 0.5).floor() * unit))
        }
        ("mod", 2) => {
            let (x, y) = (n(0)?, n(1)?);
            if is_missing(x) || is_missing(y) || y == 0.0 {
                return Ok(Val::Num(SYSMISS));
            }
            if x < 0.0 || y < 0.0 {
                return Err(NotHere::Later);
            }
            Ok(num(x - y * (x / y).floor()))
        }
        // missing values are left out: min(1, ., 3) is 1
        ("min" | "max", k) if k >= 1 => {
            let mut best: Option<f64> = None;
            for i in 0..k {
                let x = n(i)?;
                if is_missing(x) {
                    continue;
                }
                best = Some(match best {
                    None => x,
                    Some(b) if name == "min" => b.min(x),
                    Some(b) => b.max(x),
                });
            }
            Ok(Val::Num(best.unwrap_or(SYSMISS)))
        }
        ("strlen", 1) => Ok(Val::Num(s(0)?.len() as f64)),
        ("upper" | "strupper", 1) => Ok(Val::Str(s(0)?.to_ascii_uppercase())),
        ("lower" | "strlower", 1) => Ok(Val::Str(s(0)?.to_ascii_lowercase())),
        ("strtrim" | "trim", 1) => Ok(Val::Str(s(0)?.trim_matches(' ').to_string())),
        ("wordcount", 1) => Ok(Val::Num(words(&s(0)?).len() as f64)),
        ("word", 2) => {
            let i = n(1)?;
            let w = words(&s(0)?);
            if is_missing(i) || i < 1.0 {
                return Err(NotHere::Later);
            }
            Ok(Val::Str(w.get(i as usize - 1).cloned().unwrap_or_default()))
        }
        // substr("hello", 2, 3) is "ell"; negative and missing places are left to R
        ("substr", 3) => {
            let text = s(0)?;
            let (from, len) = (n(1)?, n(2)?);
            if from < 1.0 || len < 0.0 || is_missing(from) || is_missing(len) || !text.is_ascii() {
                return Err(NotHere::Later);
            }
            let start = (from as usize - 1).min(text.len());
            let end = start.saturating_add(len as usize).min(text.len());
            Ok(Val::Str(text.get(start..end).unwrap_or("").to_string()))
        }
        ("string", 1) => {
            let x = n(0)?;
            Ok(Val::Str(g(x, 9)))
        }
        _ => Err(NotHere::Later),
    }
}

/// The value of an expression written out, if it is known without R.
pub fn eval_text(text: &str) -> Result<Val, NotHere> {
    let e = crate::lang::expr::parse(text).map_err(|e| NotHere::Stata(e.message))?;
    eval(&e)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(t: &str) -> String {
        eval_text(t)
            .map(|v| v.macro_text())
            .unwrap_or_else(|e| format!("{e:?}"))
    }

    /// The values Stata 19 kept for `local x = exp`.
    #[test]
    fn what_stata_keeps() {
        assert_eq!(v("1/3"), ".3333333333333333");
        assert_eq!(v("\"a\" + \"b\""), "ab");
        assert_eq!(v("\"ab\"*3"), "ababab");
        assert_eq!(v("strlen(\"abc\")"), "3");
        assert_eq!(v("round(2.5)"), "3");
        assert_eq!(v("round(3.14159, .01)"), "3.14");
        assert_eq!(v("mod(7,3)"), "1");
        assert_eq!(v("int(-2.5)"), "-2");
        assert_eq!(v("sqrt(2)"), "1.414213562373095");
        assert_eq!(v("exp(1)"), "2.718281828459045");
        assert_eq!(v("ln(10)"), "2.302585092994046");
        assert_eq!(v("substr(\"hello\",2,3)"), "ell");
        assert_eq!(v("string(1/3)"), ".3333333");
        assert_eq!(v("word(\"a b c\", 2)"), "b");
        assert_eq!(v("wordcount(\"a b c\")"), "3");
        assert_eq!(v("upper(\"ab\") + lower(\"CD\")"), "ABcd");
        assert_eq!(v("strtrim(\"  a b  \")"), "a b");
        assert_eq!(v("max(1,5,3)"), "5");
        assert_eq!(v("min(1,.,3)"), "1");
        assert_eq!(v("1 + ."), ".");
        assert_eq!(v(". > 5"), "1");
        assert_eq!(v("7/0"), ".");
        assert_eq!(v("floor(2.7) + ceil(2.1)"), "5");
        assert_eq!(v("log10(1000)"), "3");
        assert_eq!(v("10/4"), "2.5");
        assert_eq!(v("\"3\" == \"3\""), "1");
        assert_eq!(v("3>2"), "1");
        assert_eq!(v("_pi"), "3.141592653589793");
        assert_eq!(v("c(pi)"), "3.141592653589793");
        assert_eq!(v(".a"), ".a");
        assert_eq!(v("1e300*1e10"), ".");
        assert_eq!(v("-0"), "0");
        assert_eq!(v("1.0"), "1");
    }

    #[test]
    fn the_data_and_stored_results_are_left_to_r() {
        for t in [
            "r(mean)",
            "_N",
            "price[1]",
            "e(N) + 1",
            "c(N)",
            "runiform()",
        ] {
            assert_eq!(eval_text(t), Err(NotHere::Later), "{t}");
        }
        assert_eq!(eval_text("\"a\" + 1"), Err(mismatch()));
    }
}
