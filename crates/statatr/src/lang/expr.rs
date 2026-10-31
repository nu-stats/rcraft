//! Expressions: lexing and parsing into a tree.
//!
//! Behaviour reference: `plan/stata/expressions.md`, from probe
//! `110-expressions`. Precedence, lowest first:
//!
//! | Level | Operators | Associativity |
//! |---|---|---|
//! | 1 | `\|` | left |
//! | 2 | `&` | left |
//! | 3 | `!` `~` (prefix) | |
//! | 4 | `==` `!=` `~=` `<` `<=` `>` `>=` | left (`3>2>1` is 0) |
//! | 5 | `+` `-` | left |
//! | 6 | `*` `/` | left |
//! | 7 | `-` `+` (prefix) | |
//! | 8 | `^` | left (`2^3^2` is 64; `-2^2` is -4) |

use crate::values::hex::from_hex;
use crate::values::missing::parse_missing;

/// Deepest nesting accepted, so hostile input cannot overflow the stack.
const MAX_DEPTH: usize = 64;

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Number(f64),
    Text(String),
    /// a variable, scalar or other name
    Name(String),
    /// `name[exp]`
    Subscript(String, Box<Expr>),
    Call(String, Vec<Expr>),
    Unary(UnaryOp, Box<Expr>),
    Binary(BinaryOp, Box<Expr>, Box<Expr>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Plus,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Or,
    And,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Add,
    Sub,
    Mul,
    Div,
    Pow,
}

/// Why an expression could not be read. `code` is Stata's return code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExprError {
    pub message: String,
    pub code: u16,
}

impl ExprError {
    /// Stata reports most malformed expressions as `<text> invalid name`
    /// with r(198) (probe 110: `5-`, `1e`, `0x10`, `1,2`).
    fn invalid(text: &str) -> Self {
        // outer parentheses are left out: `(5-)` is reported as `5-`
        let mut shown = text.trim();
        while let Some(inner) = shown.strip_prefix('(').and_then(|s| s.strip_suffix(')')) {
            shown = inner.trim();
        }
        Self {
            message: format!("{shown} invalid name"),
            code: 198,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Number(f64),
    Text(String),
    Name(String),
    Op(&'static str),
    Open,
    Close,
    OpenBracket,
    CloseBracket,
    Comma,
    /// `_b[x]`, `[eq]_b[x]`, `_se[eq:x]`: which (`_b` or `_se`) and the
    /// coefficient, `eq:name` when an equation is named
    Coef(String, String),
}

/// `td(...)`, `tw()`, `tm()`, `tq()`, `th()`, `ty()`, `tc()` and `tC()`
/// with the `(` at `open`: the date as a number, and where it ends.
fn date_literal(word: &str, chars: &[char], open: usize) -> Option<(Token, usize)> {
    use crate::values::datetime::{parse_clock, parse_date, parse_period, Period};
    let close = (open + 1..chars.len()).find(|&k| chars.get(k) == Some(&')'))?;
    let text: String = chars.get(open + 1..close)?.iter().collect();
    let text = text.trim();
    let v = match word {
        "td" => parse_date(text, "DMY", None),
        "tw" => parse_period(text, "YW", None, Period::Week),
        "tm" => parse_period(text, "YM", None, Period::Month),
        "tq" => parse_period(text, "YQ", None, Period::Quarter),
        "th" => parse_period(text, "YH", None, Period::Half),
        "ty" => parse_period(text, "Y", None, Period::Year),
        "tc" => parse_clock(text, "DMYhms", None, false),
        "tC" => parse_clock(text, "DMYhms", None, true),
        _ => return None,
    };
    if crate::values::missing::is_missing(v) {
        return None;
    }
    Some((Token::Number(v), close + 1))
}

/// `[eq]_b[name]` or `[eq]_se[name]` starting at `open` (the `[`).
fn equation_coef(chars: &[char], open: usize) -> Option<(Token, usize)> {
    let close = (open + 1..chars.len()).find(|&k| chars.get(k) == Some(&']'))?;
    let eq: String = chars.get(open + 1..close)?.iter().collect();
    let eq = eq.trim().to_string();
    if eq.is_empty()
        || !eq
            .chars()
            .all(|c| is_name_char(c) || matches!(c, '#' | '/' | '.'))
    {
        return None;
    }
    let rest: String = chars.get(close + 1..)?.iter().collect();
    let which = if rest.starts_with("_b[") {
        "_b"
    } else if rest.starts_with("_se[") {
        "_se"
    } else {
        return None;
    };
    let start = close + 1 + which.len() + 1;
    let end = (start..chars.len()).find(|&k| chars.get(k) == Some(&']'))?;
    let name: String = chars.get(start..end)?.iter().collect();
    Some((
        Token::Coef(which.to_string(), format!("{eq}:{}", name.trim())),
        end + 1,
    ))
}

/// Whether `word` is a run of time-series operators: `L`, `F`, `D`, `S`
/// (either case), each with an optional count (`L2`, `LD`, `S12`).
pub fn is_ts_ops(word: &str) -> bool {
    let mut chars = word.chars().peekable();
    let mut any = false;
    while let Some(c) = chars.next() {
        if !matches!(c.to_ascii_uppercase(), 'L' | 'F' | 'D' | 'S') {
            return false;
        }
        any = true;
        while chars.peek().is_some_and(char::is_ascii_digit) {
            chars.next();
        }
    }
    any
}

fn is_name_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

fn is_name_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Tokens and the character position just after each. In `partial` mode
/// lexing stops quietly at the first character that cannot start a token
/// (so `display` can stop an expression at `%9.2f` or `_col(5)`).
fn lex(text: &str, partial: bool) -> Result<(Vec<Token>, Vec<usize>), ExprError> {
    let chars: Vec<char> = text.chars().collect();
    let mut tokens = Vec::new();
    let mut ends = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars.get(i).is_some_and(|c| c.is_whitespace()) {
            i += 1;
            continue;
        }
        match next_token(&chars, i) {
            Some((token, end)) => {
                tokens.push(token);
                ends.push(end);
                i = end;
            }
            None if partial => break,
            None => return Err(ExprError::invalid(text)),
        }
    }
    Ok((tokens, ends))
}

/// The token starting at `i`, and where it ends.
fn next_token(chars: &[char], i: usize) -> Option<(Token, usize)> {
    let at = |i: usize| chars.get(i).copied();
    let c = at(i)?;
    // compound string `"..."'
    if c == '`' && at(i + 1) == Some('"') {
        let start = i + 2;
        let mut depth = 1;
        let mut j = start;
        while j < chars.len() {
            if at(j) == Some('`') && at(j + 1) == Some('"') {
                depth += 1;
                j += 2;
                continue;
            }
            if at(j) == Some('"') && at(j + 1) == Some('\'') {
                depth -= 1;
                if depth == 0 {
                    break;
                }
                j += 2;
                continue;
            }
            j += 1;
        }
        if j >= chars.len() {
            return None;
        }
        let text = chars.get(start..j)?.iter().collect();
        return Some((Token::Text(text), j + 2));
    }
    if c == '"' {
        let start = i + 1;
        let end = (start..chars.len()).find(|&j| at(j) == Some('"'))?;
        let text = chars.get(start..end)?.iter().collect();
        // `"text"'` (a compound quote's end after a plain opening) reads
        // as the string, as Stata reads it
        let after = if at(end + 1) == Some('\'') {
            end + 2
        } else {
            end + 1
        };
        return Some((Token::Text(text), after));
    }
    if c.is_ascii_digit() || (c == '.' && at(i + 1).is_some_and(|d| d.is_ascii_digit())) {
        let mut j = i;
        while at(j).is_some_and(|d| d.is_ascii_alphanumeric() || d == '.')
            || (matches!(at(j), Some('+' | '-'))
                && j > i
                && matches!(at(j - 1), Some('e' | 'E' | 'x' | 'X')))
        {
            j += 1;
        }
        let word: String = chars.get(i..j)?.iter().collect();
        return Some((Token::Number(number(&word)?), j));
    }
    if c == '.' {
        // . or .a to .z
        return match at(i + 1) {
            Some(l) if l.is_ascii_lowercase() && !at(i + 2).is_some_and(is_name_char) => {
                Some((Token::Number(parse_missing(&format!(".{l}"))?), i + 2))
            }
            Some(l) if is_name_char(l) => None,
            _ => Some((Token::Number(parse_missing(".")?), i + 1)),
        };
    }
    // `[eq]_b[x]` and `[#2]_se[x]`
    if c == '[' {
        if let Some(t) = equation_coef(chars, i) {
            return Some(t);
        }
    }
    if is_name_start(c) {
        let mut j = i;
        while at(j).is_some_and(is_name_char) {
            j += 1;
        }
        // `_b[x]` and `_b[eq:x]`: the inside is a name, not an expression
        let word: String = chars.get(i..j)?.iter().collect();
        if matches!(word.as_str(), "_b" | "_se" | "_coef") && at(j) == Some('[') {
            if let Some(close) = (j + 1..chars.len()).find(|&k| chars.get(k) == Some(&']')) {
                let inside: String = chars.get(j + 1..close)?.iter().collect();
                let inside = inside.trim();
                if !inside.is_empty()
                    && inside
                        .chars()
                        .all(|c| is_name_char(c) || matches!(c, ':' | '#' | '/' | '.'))
                {
                    return Some((Token::Coef(word, inside.to_string()), close + 1));
                }
            }
        }
        // date literals: `tq(2020q1)`, `td(1jan2020)`, `tc(1jan2020 10:00)`
        let word: String = chars.get(i..j)?.iter().collect();
        if at(j) == Some('(') {
            if let Some(v) = date_literal(&word, chars, j) {
                return Some(v);
            }
        }
        // time-series operators: `L.x`, `L2.x`, `F.L.x`, `LD.x`
        loop {
            let word: String = chars.get(i..j)?.iter().collect();
            let last = word.rsplit('.').next().unwrap_or(&word);
            if at(j) == Some('.') && is_ts_ops(last) && at(j + 1).is_some_and(is_name_start) {
                j += 1;
                while at(j).is_some_and(is_name_char) {
                    j += 1;
                }
            } else {
                break;
            }
        }
        return Some((Token::Name(chars.get(i..j)?.iter().collect()), j));
    }
    let two: String = [c, at(i + 1).unwrap_or(' ')].iter().collect();
    if let Some(op) = ["==", "!=", "~=", "<=", ">="]
        .into_iter()
        .find(|o| *o == two)
    {
        return Some((Token::Op(op), i + 2));
    }
    let token = match c {
        '(' => Token::Open,
        ')' => Token::Close,
        '[' => Token::OpenBracket,
        ']' => Token::CloseBracket,
        ',' => Token::Comma,
        '+' => Token::Op("+"),
        '-' => Token::Op("-"),
        '*' => Token::Op("*"),
        '/' => Token::Op("/"),
        '^' => Token::Op("^"),
        '!' | '~' => Token::Op("!"),
        '&' => Token::Op("&"),
        '|' => Token::Op("|"),
        '<' => Token::Op("<"),
        '>' => Token::Op(">"),
        _ => return None,
    };
    Some((token, i + 1))
}

/// A numeric literal: decimal (`1e3`, `.5`, `5.`) or `%21x` hex (`1.8X+001`).
fn number(word: &str) -> Option<f64> {
    if word.contains(['x', 'X']) {
        return from_hex(word);
    }
    let valid = word
        .chars()
        .all(|c| c.is_ascii_digit() || matches!(c, '.' | 'e' | 'E' | '+' | '-'));
    if !valid || word.ends_with(['e', 'E', '+', '-']) {
        return None;
    }
    word.parse().ok()
}

struct Parser<'a> {
    tokens: Vec<Token>,
    pos: usize,
    text: &'a str,
    depth: usize,
}

/// Parses an expression.
pub fn parse(text: &str) -> Result<Expr, ExprError> {
    let dated = date_literals(text);
    let text = dated.as_str();
    let (tokens, _) = lex(text, false)?;
    if tokens.is_empty() {
        return Err(ExprError {
            message: "invalid syntax".into(),
            code: 198,
        });
    }
    let mut p = Parser {
        tokens,
        pos: 0,
        text,
        depth: 0,
    };
    let e = p.or()?;
    if p.pos != p.tokens.len() {
        return Err(ExprError::invalid(text));
    }
    Ok(e)
}

/// Parses the longest expression at the start of `text`, returning it and
/// the character position where it ends. Used by `display`, where an
/// expression runs until something that cannot continue it.
pub fn parse_prefix(text: &str) -> Result<(Expr, usize), ExprError> {
    let (tokens, ends) = lex(text, true)?;
    if tokens.is_empty() {
        return Err(ExprError::invalid(text));
    }
    let mut p = Parser {
        tokens,
        pos: 0,
        text,
        depth: 0,
    };
    let e = p.or()?;
    let end = p
        .pos
        .checked_sub(1)
        .and_then(|i| ends.get(i))
        .copied()
        .unwrap_or(0);
    Ok((e, end))
}

impl Parser<'_> {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn next(&mut self) -> Option<Token> {
        let t = self.tokens.get(self.pos).cloned();
        self.pos += 1;
        t
    }

    fn eat_op(&mut self, ops: &[&str]) -> Option<&'static str> {
        match self.peek() {
            Some(Token::Op(op)) if ops.contains(op) => {
                let op = *op;
                self.pos += 1;
                Some(op)
            }
            _ => None,
        }
    }

    fn fail(&self) -> ExprError {
        ExprError::invalid(self.text)
    }

    fn deeper(&mut self) -> Result<(), ExprError> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err(ExprError {
                message: "expression too long".into(),
                code: 130,
            });
        }
        Ok(())
    }

    fn binary_left(
        &mut self,
        ops: &[&str],
        next: fn(&mut Self) -> Result<Expr, ExprError>,
    ) -> Result<Expr, ExprError> {
        let mut left = next(self)?;
        while let Some(op) = self.eat_op(ops) {
            let right = next(self)?;
            left = Expr::Binary(binary(op), Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn or(&mut self) -> Result<Expr, ExprError> {
        self.binary_left(&["|"], Self::and)
    }

    fn and(&mut self) -> Result<Expr, ExprError> {
        self.binary_left(&["&"], Self::not)
    }

    fn not(&mut self) -> Result<Expr, ExprError> {
        if self.eat_op(&["!"]).is_some() {
            self.deeper()?;
            let inner = self.not()?;
            self.depth -= 1;
            return Ok(Expr::Unary(UnaryOp::Not, Box::new(inner)));
        }
        self.compare()
    }

    fn compare(&mut self) -> Result<Expr, ExprError> {
        self.binary_left(&["==", "!=", "~=", "<", "<=", ">", ">="], Self::sum)
    }

    fn sum(&mut self) -> Result<Expr, ExprError> {
        self.binary_left(&["+", "-"], Self::product)
    }

    fn product(&mut self) -> Result<Expr, ExprError> {
        self.binary_left(&["*", "/"], Self::unary)
    }

    fn unary(&mut self) -> Result<Expr, ExprError> {
        if let Some(op) = self.eat_op(&["-", "+", "!"]) {
            self.deeper()?;
            let inner = self.unary()?;
            self.depth -= 1;
            let op = match op {
                "-" => UnaryOp::Neg,
                "+" => UnaryOp::Plus,
                _ => UnaryOp::Not,
            };
            return Ok(Expr::Unary(op, Box::new(inner)));
        }
        self.power()
    }

    /// `^` is left-associative; its right operand may carry a sign
    /// (`2^-1`).
    fn power(&mut self) -> Result<Expr, ExprError> {
        let mut left = self.atom()?;
        while self.eat_op(&["^"]).is_some() {
            let right = if let Some(op) = self.eat_op(&["-", "+"]) {
                self.deeper()?;
                let inner = self.power_operand()?;
                self.depth -= 1;
                let op = if op == "-" {
                    UnaryOp::Neg
                } else {
                    UnaryOp::Plus
                };
                Expr::Unary(op, Box::new(inner))
            } else {
                self.atom()?
            };
            left = Expr::Binary(BinaryOp::Pow, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn power_operand(&mut self) -> Result<Expr, ExprError> {
        if let Some(op) = self.eat_op(&["-", "+"]) {
            self.deeper()?;
            let inner = self.power_operand()?;
            self.depth -= 1;
            let op = if op == "-" {
                UnaryOp::Neg
            } else {
                UnaryOp::Plus
            };
            return Ok(Expr::Unary(op, Box::new(inner)));
        }
        self.atom()
    }

    fn atom(&mut self) -> Result<Expr, ExprError> {
        match self.next() {
            Some(Token::Number(v)) => Ok(Expr::Number(v)),
            Some(Token::Text(s)) => Ok(Expr::Text(s)),
            Some(Token::Open) => {
                self.deeper()?;
                let e = self.or()?;
                self.depth -= 1;
                match self.next() {
                    Some(Token::Close) => Ok(e),
                    _ => Err(self.fail()),
                }
            }
            Some(Token::Coef(which, key)) => Ok(Expr::Subscript(which, Box::new(Expr::Text(key)))),
            // square brackets group like parentheses: `3*[2+1]` is 9
            Some(Token::OpenBracket) => {
                self.deeper()?;
                let e = self.or()?;
                self.depth -= 1;
                match self.next() {
                    Some(Token::CloseBracket) => Ok(e),
                    _ => Err(self.fail()),
                }
            }
            Some(Token::Name(name)) => match self.peek() {
                Some(Token::Open) => {
                    self.pos += 1;
                    let mut args = Vec::new();
                    if self.peek() == Some(&Token::Close) {
                        self.pos += 1;
                        return Ok(Expr::Call(name, args));
                    }
                    self.deeper()?;
                    loop {
                        args.push(self.or()?);
                        match self.next() {
                            Some(Token::Comma) => continue,
                            Some(Token::Close) => break,
                            _ => return Err(self.fail()),
                        }
                    }
                    self.depth -= 1;
                    Ok(Expr::Call(name, args))
                }
                // `L.x[3]` is not allowed
                Some(Token::OpenBracket) if name.contains('.') => Err(ExprError {
                    message: format!("{name} invalid name"),
                    code: 198,
                }),
                Some(Token::OpenBracket) => {
                    self.pos += 1;
                    self.deeper()?;
                    let index = self.or()?;
                    self.depth -= 1;
                    match self.next() {
                        Some(Token::CloseBracket) => Ok(Expr::Subscript(name, Box::new(index))),
                        // `A[1,2]` and `A["mpg",1]`: a matrix element
                        Some(Token::Comma) => {
                            self.deeper()?;
                            let col = self.or()?;
                            self.depth -= 1;
                            match self.next() {
                                Some(Token::CloseBracket) => Ok(Expr::Call(
                                    "__matel".into(),
                                    vec![Expr::Text(name), index, col],
                                )),
                                _ => Err(self.fail()),
                            }
                        }
                        _ => Err(self.fail()),
                    }
                }
                _ => Ok(Expr::Name(name)),
            },
            _ => Err(self.fail()),
        }
    }
}

fn binary(op: &str) -> BinaryOp {
    match op {
        "|" => BinaryOp::Or,
        "&" => BinaryOp::And,
        "==" => BinaryOp::Eq,
        "!=" | "~=" => BinaryOp::Ne,
        "<" => BinaryOp::Lt,
        "<=" => BinaryOp::Le,
        ">" => BinaryOp::Gt,
        ">=" => BinaryOp::Ge,
        "+" => BinaryOp::Add,
        "-" => BinaryOp::Sub,
        "*" => BinaryOp::Mul,
        "/" => BinaryOp::Div,
        _ => BinaryOp::Pow,
    }
}

/// `d(23 Oct 2026)` and `td(...)`: a date written as is, turned into
/// `date("23 Oct 2026", "DMY")` before the expression is read.
fn date_literals(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(i) = rest.find("d(") {
        let (before, after) = rest.split_at(i);
        let prev = before.chars().last();
        let name_len = if before.ends_with('t') { 1 } else { 0 };
        // only a whole name: `d(` or `td(`, not `round(` or `mod(`
        let start = before.len() - name_len;
        let lead = before.get(..start).and_then(|b| b.chars().last());
        let whole = match (name_len, prev) {
            (1, _) => !lead.is_some_and(|c| c.is_alphanumeric() || c == '_'),
            _ => !prev.is_some_and(|c| c.is_alphanumeric() || c == '_'),
        };
        let inner_end = after.find(')');
        match (whole, inner_end) {
            (true, Some(end)) => {
                let inner = after.get(2..end).unwrap_or("").trim();
                out.push_str(before.get(..start).unwrap_or(""));
                out.push_str(&format!("date(\"{inner}\",\"DMY\")"));
                rest = after.get(end + 1..).unwrap_or("");
            }
            _ => {
                out.push_str(before);
                out.push_str("d(");
                rest = after.get(2..).unwrap_or("");
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use BinaryOp::*;

    fn n(v: f64) -> Box<Expr> {
        Box::new(Expr::Number(v))
    }

    #[test]
    fn precedence_matches_probe_110() {
        // -2^2 is -(2^2)
        assert_eq!(
            parse("-2^2").unwrap(),
            Expr::Unary(UnaryOp::Neg, Box::new(Expr::Binary(Pow, n(2.0), n(2.0))))
        );
        // 2^3^2 is (2^3)^2
        assert_eq!(
            parse("2^3^2").unwrap(),
            Expr::Binary(Pow, Box::new(Expr::Binary(Pow, n(2.0), n(3.0))), n(2.0))
        );
        // 1|0&0 is 1|(0&0)
        assert_eq!(
            parse("1|0&0").unwrap(),
            Expr::Binary(Or, n(1.0), Box::new(Expr::Binary(And, n(0.0), n(0.0))))
        );
        assert!(matches!(parse("2^-1").unwrap(), Expr::Binary(Pow, _, _)));
        assert!(matches!(parse("x[_n-1]").unwrap(), Expr::Subscript(_, _)));
        assert!(matches!(parse("max(1,2,.)").unwrap(), Expr::Call(_, args) if args.len() == 3));
        assert_eq!(
            parse(".a").unwrap(),
            Expr::Number(crate::values::missing::missing(1))
        );
        assert_eq!(parse("1.8X+001").unwrap(), Expr::Number(3.0));
        assert_eq!(parse(r#"`"a "b""'"#).unwrap(), Expr::Text("a \"b\"".into()));
    }

    #[test]
    fn prefixes_stop_where_the_expression_does() {
        let (e, end) = parse_prefix(r#""E|x|" %21x (1)"#).unwrap();
        assert_eq!(e, Expr::Text("E|x|".into()));
        assert_eq!(end, 6);
        let (_, end) = parse_prefix("1 + 2 _col(5)").unwrap();
        assert_eq!(end, 5);
        let (_, end) = parse_prefix(r#""a" + "b" "c""#).unwrap();
        assert_eq!(end, 9);
    }

    #[test]
    fn errors_match_probe_110() {
        for bad in ["5-", "1e", "0x10", "12_000", "1,2"] {
            let e = parse(bad).unwrap_err();
            assert_eq!(e.code, 198);
            assert_eq!(e.message, format!("{bad} invalid name"));
        }
        let deep = "(".repeat(500) + "1" + &")".repeat(500);
        assert!(parse(&deep).is_err());
    }
}
