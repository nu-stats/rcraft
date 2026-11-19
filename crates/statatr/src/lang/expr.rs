// Vendored from StatCraft crates/lang/src/expr.rs at b7d392e87dcc. Do not edit: run
// `cargo xtask sync-statcraft-lang` (see the `lib.rs` of this crate).
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
    /// a character no token begins with (`\`, `@`, `;`)
    Bad,
    /// a number that does not read (`1e`, `0x10`)
    BadNumber(String),
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
        // the seconds may be left out: `tc(1jan2020 10:00)` (probe 705)
        "tc" | "tC" => {
            let leap = word == "tC";
            let v = parse_clock(text, "DMYhms", None, leap);
            if crate::values::missing::is_missing(v) {
                parse_clock(text, "DMYhm", None, leap)
            } else {
                v
            }
        }
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
/// The tokens of an expression with where each starts and ends (char
/// positions). A character no token can begin, or a malformed number,
/// becomes a `Bad` or `BadNumber` token and ends the list: what follows
/// is never read (probe 704).
type Lexed = (Vec<Token>, Vec<usize>, Vec<usize>);

fn lex(text: &str, partial: bool) -> Result<Lexed, ExprError> {
    let chars: Vec<char> = text.chars().collect();
    let mut tokens = Vec::new();
    let mut starts = Vec::new();
    let mut ends = Vec::new();
    let mut i = 0;
    while let Some(&c) = chars.get(i) {
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        match next_token(&chars, i) {
            Some((token, end)) => {
                tokens.push(token);
                starts.push(i);
                ends.push(end);
                i = end;
            }
            None => {
                let next_digit = chars.get(i + 1).is_some_and(char::is_ascii_digit);
                let token = if c.is_ascii_digit() || (c == '.' && next_digit) {
                    let word: String = chars
                        .get(i..)
                        .unwrap_or_default()
                        .iter()
                        .take_while(|d| d.is_ascii_alphanumeric() || **d == '.' || **d == '_')
                        .collect();
                    Token::BadNumber(word)
                } else if matches!(c, '"' | '`' | '.') {
                    // an open quote or a stray `.name`
                    if partial {
                        break;
                    }
                    return Err(ExprError::invalid(text));
                } else {
                    Token::Bad
                };
                tokens.push(token);
                starts.push(i);
                ends.push(i + 1);
                break;
            }
        }
    }
    Ok((tokens, starts, ends))
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
        // `_b[x]`, `_b[eq:x]` and sem's `_b[/:var(e.y)]`: the inside is a
        // name, not an expression
        let word: String = chars.get(i..j)?.iter().collect();
        if (word == "_b" || word == "_se") && at(j) == Some('[') {
            if let Some(close) = (j + 1..chars.len()).find(|&k| chars.get(k) == Some(&']')) {
                let inside: String = chars.get(j + 1..close)?.iter().collect();
                let inside = inside.trim();
                if !inside.is_empty()
                    && inside.chars().all(|c| {
                        is_name_char(c) || matches!(c, ':' | '#' | '/' | '.' | '(' | ')' | ',')
                    })
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
        // operators no expression accepts; read so the message can name
        // them as Stata does (probe 704)
        '=' => Token::Op("="),
        '#' => Token::Op("#"),
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
    starts: Vec<usize>,
    ends: Vec<usize>,
    chars: Vec<char>,
    /// for each read character, its place in the original text
    map: Vec<usize>,
    pos: usize,
    text: &'a str,
    depth: usize,
}

fn syntax_error() -> ExprError {
    ExprError {
        message: "invalid syntax".into(),
        code: 198,
    }
}

/// Characters that end the word Stata names after a character no token
/// begins with: `\5+6` is named `\5` (probe 704).
fn ends_word(c: char) -> bool {
    c.is_whitespace() || "+-*/^&|<>=!~()[],\"#".contains(c)
}

/// Parentheses and brackets outside quotes must pair up: `too few ')' or
/// ']'` or `too many ')' or ']'`, r(132), before anything is evaluated
/// (probe 704).
pub fn check_balance(text: &str) -> Result<(), ExprError> {
    let chars: Vec<char> = text.chars().collect();
    // ( and [ are counted apart: `(x + 1]` is too many (probe 705)
    let (mut parens, mut brackets): (i64, i64) = (0, 0);
    let mut i = 0;
    while let Some(&c) = chars.get(i) {
        match c {
            '`' if chars.get(i + 1) == Some(&'"') => {
                // a compound quote, nested ones counted
                let mut nest = 1;
                i += 2;
                while nest > 0 && i < chars.len() {
                    if chars.get(i) == Some(&'`') && chars.get(i + 1) == Some(&'"') {
                        nest += 1;
                        i += 1;
                    } else if chars.get(i) == Some(&'"') && chars.get(i + 1) == Some(&'\'') {
                        nest -= 1;
                        i += 1;
                    }
                    i += 1;
                }
                continue;
            }
            '"' => {
                i += 1;
                while chars.get(i).is_some_and(|&d| d != '"') {
                    i += 1;
                }
            }
            '(' => parens += 1,
            '[' => brackets += 1,
            ')' | ']' => {
                if c == ')' {
                    parens -= 1;
                } else {
                    brackets -= 1;
                }
                if parens < 0 || brackets < 0 {
                    return Err(ExprError {
                        message: "too many ')' or ']'".into(),
                        code: 132,
                    });
                }
            }
            _ => {}
        }
        i += 1;
    }
    if parens > 0 || brackets > 0 {
        return Err(ExprError {
            message: "too few ')' or ']'".into(),
            code: 132,
        });
    }
    Ok(())
}

/// Inside parentheses and brackets Stata drops blanks before it reads the
/// expression (probe 705): `(3 4)` is 34, `(x y)` the variable xy,
/// `(1 e3)` 1000 and `(3 < = 4)` 1. Quoted text keeps its blanks, and so
/// do date literals (`tc(1jan2020 10:00)`). Returns the text and, for
/// each of its characters, where it was in the original.
fn drop_inner_blanks(text: &str) -> (Vec<char>, Vec<usize>) {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::with_capacity(chars.len());
    let mut map = Vec::with_capacity(chars.len());
    // the kind of each open group: true when its blanks are dropped
    let mut groups: Vec<bool> = Vec::new();
    let mut i = 0;
    let keep = |c: char, k: usize, out: &mut Vec<char>, map: &mut Vec<usize>| {
        out.push(c);
        map.push(k);
    };
    while let Some(&c) = chars.get(i) {
        let dropping = groups.last().copied().unwrap_or(false);
        match c {
            '"' => {
                // to the closing quote; a compound quote `"..."' runs to
                // its own `"'`, nested ones counted, so an inner `"` does
                // not end it
                let compound = i > 0 && chars.get(i - 1) == Some(&'`');
                keep(c, i, &mut out, &mut map);
                i += 1;
                let mut nest = 1;
                while let Some(&d) = chars.get(i) {
                    keep(d, i, &mut out, &mut map);
                    i += 1;
                    if !compound {
                        if d == '"' {
                            break;
                        }
                    } else if d == '`' && chars.get(i) == Some(&'"') {
                        nest += 1;
                        keep('"', i, &mut out, &mut map);
                        i += 1;
                    } else if d == '"' && chars.get(i) == Some(&'\'') {
                        nest -= 1;
                        keep('\'', i, &mut out, &mut map);
                        i += 1;
                        if nest == 0 {
                            break;
                        }
                    }
                }
                continue;
            }
            '(' | '[' => {
                // a date literal's parentheses keep their blanks
                let word: String = out
                    .iter()
                    .rev()
                    .take_while(|d| is_name_char(**d))
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect();
                let date = c == '('
                    && matches!(
                        word.as_str(),
                        "td" | "tw" | "tm" | "tq" | "th" | "ty" | "tc" | "tC"
                    );
                groups.push(!date && groups.last().copied().unwrap_or(true));
            }
            ')' | ']' => {
                groups.pop();
            }
            c if c.is_whitespace() && dropping => {
                i += 1;
                continue;
            }
            _ => {}
        }
        keep(c, i, &mut out, &mut map);
        i += 1;
    }
    (out, map)
}

impl<'a> Parser<'a> {
    fn new(text: &'a str, partial: bool) -> Result<Self, ExprError> {
        let (chars, map) = drop_inner_blanks(text);
        let read: String = chars.iter().collect();
        let (tokens, starts, ends) = lex(&read, partial)?;
        Ok(Parser {
            tokens,
            starts,
            ends,
            chars,
            map,
            pos: 0,
            text,
            depth: 0,
        })
    }

    /// Where the read text's position `end` (exclusive) was in the
    /// original text.
    fn original_end(&self, end: usize) -> usize {
        match end.checked_sub(1).and_then(|k| self.map.get(k)) {
            Some(&k) => k + 1,
            None => 0,
        }
    }

    fn too_few() -> ExprError {
        ExprError {
            message: "too few ')' or ']'".into(),
            code: 132,
        }
    }

    fn too_many() -> ExprError {
        ExprError {
            message: "too many ')' or ']'".into(),
            code: 132,
        }
    }

    /// The text of tokens `from` to `to` (inclusive), blanks removed: how
    /// Stata names the piece of an expression that went wrong.
    fn squeezed(&self, from: usize, to: usize) -> String {
        let start = self.starts.get(from).copied().unwrap_or(0);
        let end = self.ends.get(to).copied().unwrap_or(start);
        self.chars
            .get(start..end.max(start))
            .unwrap_or_default()
            .iter()
            .filter(|c| !c.is_whitespace())
            .collect()
    }

    fn named(&self, from: usize, to: usize) -> ExprError {
        ExprError {
            message: format!("{} invalid name", self.squeezed(from, to)),
            code: 198,
        }
    }

    /// The word starting at a `Bad` token: up to a blank or an operator.
    fn bad_word(&self, at: usize) -> ExprError {
        let start = self.starts.get(at).copied().unwrap_or(0);
        let mut word = String::new();
        for (k, &c) in self.chars.iter().enumerate().skip(start) {
            if k > start && ends_word(c) {
                break;
            }
            word.push(c);
        }
        ExprError {
            message: format!("{word} invalid name"),
            code: 198,
        }
    }

    /// The token where a whole expression was expected to end:
    /// `invalid '5' ` (a character no token begins with is named alone).
    fn leftover(&self) -> ExprError {
        let at = self.pos;
        let start = self.starts.get(at).copied().unwrap_or(0);
        let shown: String = match self.tokens.get(at) {
            Some(Token::Bad) => self
                .chars
                .get(start)
                .map(|c| c.to_string())
                .unwrap_or_default(),
            // a string shows its opening quote only: `invalid '"s' `
            Some(Token::Text(s)) => format!("\"{s}"),
            _ => self.squeezed(at, at),
        };
        ExprError {
            message: format!("invalid '{shown}' "),
            code: 198,
        }
    }

    /// After a complete operand: `!` or `#` there is an operator no
    /// expression has, named with both sides (`7 ! 4` is `7!4`).
    fn no_such_operator(&mut self, start: usize) -> Result<(), ExprError> {
        if matches!(self.peek(), Some(Token::Op("!" | "#"))) {
            self.pos += 1;
            if self.peek().is_none() {
                return Err(syntax_error());
            }
            self.unary()?;
            return Err(self.named(start, self.pos.saturating_sub(1)));
        }
        Ok(())
    }

    /// The token after the binary operator at `op` (with its left side
    /// starting at `start`) cannot begin its right side: `3 & | 4` is
    /// `3&`, `(3 -)` is `3-`, `5 -` at the end is invalid syntax. An
    /// operator that binds tighter is left to the operand (`3 + * 4`).
    fn check_right(&self, start: usize, op: usize, level: u8) -> Result<(), ExprError> {
        match self.peek() {
            None => Err(syntax_error()),
            Some(Token::Close | Token::CloseBracket | Token::Comma) => Err(self.named(start, op)),
            Some(Token::Op(o)) if !matches!(*o, "-" | "+" | "!") && level_of(o) <= level => {
                Err(self.named(start, op))
            }
            Some(Token::Bad) => Err(self.bad_word(self.pos)),
            _ => Ok(()),
        }
    }

    /// The tokens inside the parenthesis or bracket at `open`, up to its
    /// partner, named blanks removed (`(4 @ 5)` is `4@5`).
    fn inside(&self, open: usize) -> ExprError {
        self.inside_from(self.ends.get(open).copied().unwrap_or(0), false)
    }

    /// What is inside a group from char `start` up to its partner or, for
    /// a function's argument, a comma: `max(1 @ 2, 3)` is `1@2`.
    fn inside_from(&self, start: usize, at_comma: bool) -> ExprError {
        // read from the text: a `Bad` token ends the token list early
        let mut depth = 1usize;
        let mut quoted = false;
        let mut shown = String::new();
        for &c in self.chars.iter().skip(start) {
            match c {
                '"' => quoted = !quoted,
                '(' | '[' if !quoted => depth += 1,
                ',' if !quoted && at_comma && depth == 1 => break,
                ')' | ']' if !quoted => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
            if !c.is_whitespace() {
                shown.push(c);
            }
        }
        if shown.is_empty() {
            return self.fail();
        }
        ExprError {
            message: format!("{shown} invalid name"),
            code: 198,
        }
    }
}

/// How tightly a binary operator binds (higher binds tighter).
fn level_of(op: &str) -> u8 {
    match op {
        "|" => 1,
        "&" => 2,
        "==" | "!=" | "~=" | "<" | "<=" | ">" | ">=" => 3,
        "+" => 4,
        "-" => 5,
        "*" => 6,
        "/" => 7,
        "^" => 9,
        _ => 0,
    }
}

/// Parses an expression.
pub fn parse(text: &str) -> Result<Expr, ExprError> {
    parse_with(text)
}

fn parse_with(text: &str) -> Result<Expr, ExprError> {
    let dated = date_literals(text);
    let text = dated.as_str();
    check_balance(text)?;
    let mut p = Parser::new(text, false)?;
    if p.tokens.is_empty() {
        return Err(syntax_error());
    }
    let e = p.or()?;
    p.no_such_operator(0)?;
    if p.pos != p.tokens.len() {
        return Err(match p.tokens.get(p.pos) {
            Some(Token::BadNumber(w)) => ExprError {
                message: format!("{w} invalid name"),
                code: 198,
            },
            Some(Token::Op("=")) => syntax_error(),
            _ => p.leftover(),
        });
    }
    Ok(e)
}

/// Parses the longest expression at the start of `text`, returning it and
/// the character position where it ends. Used by `display`, where an
/// expression runs until something that cannot continue it.
pub fn parse_prefix(text: &str) -> Result<(Expr, usize), ExprError> {
    let mut p = Parser::new(text, true)?;
    if p.tokens.is_empty() {
        return Err(ExprError::invalid(text));
    }
    let e = p.or()?;
    p.no_such_operator(0)?;
    // a ) or ] after the item: `display 1 2)` shows 1, then too many
    if matches!(p.peek(), Some(Token::Close | Token::CloseBracket)) {
        return Err(Parser::too_many());
    }
    let read_end = p
        .pos
        .checked_sub(1)
        .and_then(|i| p.ends.get(i))
        .copied()
        .unwrap_or(0);
    let end = p.original_end(read_end);
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

    fn unknown_function(&self, from: usize, to: usize) -> ExprError {
        ExprError {
            message: format!("unknown function {}()", self.squeezed(from, to)),
            code: 133,
        }
    }

    /// The prefix `+` at token `at`. Stata reads a + sign only as part of a
    /// number written out (`+5`, `2^+2`, `x*+2`); before anything else it
    /// is an error named with the sign and the word after it (probe 752):
    /// `+x` is `+x invalid name`, `+(x)` and `+abs(x)` are unknown
    /// functions `+()` and `+abs()` (r(133)), `+.a` is invalid syntax, and
    /// a sign before another operator names the sign alone (`+-3`, `++3`).
    fn plus_operand(&self, at: usize) -> Result<(), ExprError> {
        let n = at + 1;
        match self.tokens.get(n) {
            Some(Token::Number(v)) if !crate::values::missing::is_missing(*v) => Ok(()),
            Some(Token::Number(_)) | None => Err(syntax_error()),
            Some(Token::Name(_)) if matches!(self.tokens.get(n + 1), Some(Token::Open)) => {
                Err(self.unknown_function(at, n))
            }
            Some(Token::Open) => Err(self.unknown_function(at, at)),
            Some(Token::Name(_) | Token::Text(_) | Token::BadNumber(_)) => Err(self.named(at, n)),
            Some(Token::Coef(which, _)) => Err(ExprError {
                message: format!("+{which} invalid name"),
                code: 198,
            }),
            Some(Token::Op("+")) => {
                // `++3` names one sign, `+++3` two: the last + belongs
                // to the number
                let mut k = n;
                while matches!(self.tokens.get(k), Some(Token::Op("+"))) {
                    k += 1;
                }
                let last = if matches!(self.tokens.get(k), Some(Token::Op(_))) {
                    k - 1
                } else {
                    k - 2
                };
                Err(self.named(at, last))
            }
            Some(Token::Op("!")) => {
                // `+!x` is named whole: the signs, then the word
                let mut q = n;
                while matches!(self.tokens.get(q), Some(Token::Op("!"))) {
                    q += 1;
                }
                match self.tokens.get(q) {
                    None => Err(syntax_error()),
                    Some(Token::Open) => Err(self.unknown_function(at, q - 1)),
                    Some(Token::Name(_)) if matches!(self.tokens.get(q + 1), Some(Token::Open)) => {
                        Err(self.unknown_function(at, q))
                    }
                    Some(_) => Err(self.named(at, q)),
                }
            }
            Some(Token::Op(_) | Token::OpenBracket) => Err(self.named(at, at)),
            // `)`, `,` and a stray character are named by the operand reader
            Some(_) => Ok(()),
        }
    }

    /// A `+` right after the binary `+` at token `op` (`1 + +2`, `x++2`):
    /// Stata refuses it, naming the operand before and the `+`
    /// (`1+ invalid name`), except after a string written out, where
    /// `"a" + +"b"` is `ab` (the sign is dropped; probe 752).
    fn plus_after_plus(&mut self, op: usize) -> Result<(), ExprError> {
        let Some(before) = op.checked_sub(1) else {
            return Ok(());
        };
        match self.tokens.get(before) {
            Some(Token::Text(_)) => {
                self.pos += 1;
                Ok(())
            }
            Some(Token::Number(v)) if crate::values::missing::is_missing(*v) => Err(syntax_error()),
            Some(Token::Number(_) | Token::Name(_) | Token::BadNumber(_)) => {
                // a signed number is one word: `+5 + +5` is `+5+`
                let signed = matches!(self.tokens.get(before), Some(Token::Number(_)))
                    && before >= 1
                    && matches!(self.tokens.get(before - 1), Some(Token::Op("+")))
                    && (before < 2
                        || matches!(
                            self.tokens.get(before - 2),
                            Some(Token::Op(_) | Token::Open | Token::OpenBracket | Token::Comma)
                        ));
                let from = if signed { before - 1 } else { before };
                Err(self.named(from, op))
            }
            Some(Token::Close) => {
                let owner = self
                    .opener(before, false)
                    .and_then(|m| self.tokens.get(m.checked_sub(1)?));
                match owner {
                    // r(), e() and s() take it as invalid syntax
                    Some(Token::Name(n)) if matches!(n.as_str(), "r" | "e" | "s") => {
                        Err(syntax_error())
                    }
                    _ => Err(ExprError {
                        message: "unknown function ()".into(),
                        code: 133,
                    }),
                }
            }
            Some(Token::CloseBracket) => {
                let inner = self
                    .opener(before, true)
                    .map(|m| self.squeezed(m + 1, before.saturating_sub(1)))
                    .unwrap_or_default();
                Err(ExprError {
                    message: format!("equation [{inner}] not found"),
                    code: 303,
                })
            }
            _ => Ok(()),
        }
    }

    /// The token index of the `(` (or `[`) that the closer at `close` ends.
    fn opener(&self, close: usize, bracket: bool) -> Option<usize> {
        let (open, shut) = if bracket {
            (Token::OpenBracket, Token::CloseBracket)
        } else {
            (Token::Open, Token::Close)
        };
        let mut depth = 0usize;
        for i in (0..close).rev() {
            let t = self.tokens.get(i)?;
            if std::mem::discriminant(t) == std::mem::discriminant(&shut) {
                depth += 1;
            } else if std::mem::discriminant(t) == std::mem::discriminant(&open) {
                if depth == 0 {
                    return Some(i);
                }
                depth -= 1;
            }
        }
        None
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
        let start = self.pos;
        let mut left = next(self)?;
        while let Some(op) = self.eat_op(ops) {
            self.check_right(start, self.pos - 1, level_of(op))?;
            if op == "+" && matches!(self.peek(), Some(Token::Op("+"))) {
                self.plus_after_plus(self.pos - 1)?;
            }
            let right = next(self)?;
            left = Expr::Binary(binary(op), Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn or(&mut self) -> Result<Expr, ExprError> {
        self.binary_left(&["|"], Self::and)
    }

    fn and(&mut self) -> Result<Expr, ExprError> {
        self.binary_left(&["&"], Self::compare)
    }

    fn compare(&mut self) -> Result<Expr, ExprError> {
        self.binary_left(&["==", "!=", "~=", "<", "<=", ">", ">="], Self::sum)
    }

    /// Stata binds `-` tighter than `+`, and `/` tighter than `*`, each
    /// left to right: `a+b-c` is `a+(b-c)`, `a-b+c` is `(a-b)+c`,
    /// `a*b/c` is `a*(b/c)` and `a/b*c` is `(a/b)*c` (probe 659, 300 of
    /// 300 for every form; `plan/stata/expressions.md`).
    fn sum(&mut self) -> Result<Expr, ExprError> {
        self.binary_left(&["+"], Self::difference)
    }

    fn difference(&mut self) -> Result<Expr, ExprError> {
        self.binary_left(&["-"], Self::product)
    }

    fn product(&mut self) -> Result<Expr, ExprError> {
        self.binary_left(&["*"], Self::quotient)
    }

    fn quotient(&mut self) -> Result<Expr, ExprError> {
        self.binary_left(&["/"], Self::unary)
    }

    /// Prefix `-`, `+` and `!` bind tighter than every binary operator but
    /// `^`: `!0+1` is 2, `!0==5` is 0, `!2^0` is 0; `!-1` is invalid
    /// syntax while `-!0` is -1 (probe 690).
    fn unary(&mut self) -> Result<Expr, ExprError> {
        if let Some(op) = self.eat_op(&["-", "+", "!"]) {
            if op == "!" && matches!(self.peek(), Some(Token::Op("-" | "+"))) {
                return Err(syntax_error());
            }
            if op == "+" {
                self.plus_operand(self.pos - 1)?;
            }
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
        let start = self.pos;
        let mut left = self.atom()?;
        while self.eat_op(&["^"]).is_some() {
            self.check_right(start, self.pos - 1, level_of("^"))?;
            let right = if let Some(op) = self.eat_op(&["-", "+"]) {
                if op == "+" {
                    self.plus_operand(self.pos - 1)?;
                }
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
            if op == "+" {
                self.plus_operand(self.pos - 1)?;
            }
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
        let at = self.pos;
        match self.next() {
            Some(Token::Number(v)) => Ok(Expr::Number(v)),
            Some(Token::Text(s)) => Ok(Expr::Text(s)),
            Some(Token::Open) => {
                self.deeper()?;
                let e = self.or()?;
                self.no_such_operator(at + 1)?;
                self.depth -= 1;
                match self.next() {
                    Some(Token::Close) => Ok(e),
                    None => Err(Self::too_few()),
                    _ => Err(self.inside(at)),
                }
            }
            None => Err(syntax_error()),
            // an empty operand before a comma: `abs(,)` names the comma
            Some(Token::Comma) => Err(self.named(at, at)),
            Some(Token::Bad) => Err(self.bad_word(at)),
            Some(Token::BadNumber(word)) => Err(ExprError {
                message: format!("{word} invalid name"),
                code: 198,
            }),
            // an operator where an operand belongs, named with the operand
            // after it (`3 + * 4` is `*4`, `# 5` is `#5`)
            Some(Token::Op(_)) => {
                if self.peek().is_none() {
                    return Err(syntax_error());
                }
                let last = match self.peek() {
                    Some(Token::Open | Token::OpenBracket) => {
                        let mut depth = 0usize;
                        let mut last = self.pos;
                        for (k, t) in self.tokens.iter().enumerate().skip(self.pos) {
                            match t {
                                Token::Open | Token::OpenBracket => depth += 1,
                                Token::Close | Token::CloseBracket => {
                                    depth = depth.saturating_sub(1);
                                }
                                _ => {}
                            }
                            last = k;
                            if depth == 0 {
                                break;
                            }
                        }
                        last
                    }
                    _ => self.pos,
                };
                Err(self.named(at, last))
            }
            Some(Token::Coef(which, key)) => Ok(Expr::Subscript(which, Box::new(Expr::Text(key)))),
            // square brackets group like parentheses: `3*[2+1]` is 9
            Some(Token::OpenBracket) => {
                self.deeper()?;
                let e = self.or()?;
                self.no_such_operator(at + 1)?;
                self.depth -= 1;
                match self.next() {
                    Some(Token::CloseBracket) => Ok(e),
                    None => Err(Self::too_few()),
                    _ => Err(self.inside(at)),
                }
            }
            // a call or subscript needs its `(` or `[` right after the name:
            // `x (2)` is x and a leftover `(` (probe 705; blanks inside
            // parentheses are gone before this, so `(x (2))` is a call)
            Some(Token::Name(name))
                if self.starts.get(self.pos) != self.ends.get(at)
                    && matches!(self.peek(), Some(Token::Open | Token::OpenBracket)) =>
            {
                Ok(Expr::Name(name))
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
                        let first = self.pos;
                        args.push(self.or()?);
                        self.no_such_operator(first)?;
                        match self.next() {
                            Some(Token::Comma) => continue,
                            Some(Token::Close) => break,
                            None => return Err(Self::too_few()),
                            // the argument as written (`max(1 @ 2, 3)` is `1@2`)
                            _ => {
                                let start = self.starts.get(first).copied().unwrap_or(0);
                                return Err(self.inside_from(start, true));
                            }
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
                    let open = self.pos;
                    self.pos += 1;
                    self.deeper()?;
                    let index = self.or()?;
                    self.depth -= 1;
                    match self.next() {
                        Some(Token::CloseBracket) => Ok(Expr::Subscript(name, Box::new(index))),
                        None => Err(Self::too_few()),
                        // `x[1 @ 2]` is `1@2`
                        Some(Token::Bad | Token::Number(_) | Token::Name(_) | Token::Text(_)) => {
                            Err(self.inside(open))
                        }
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
    fn minus_binds_tighter_than_plus_and_divide_than_times() {
        // probe 659: a+b-c is a+(b-c), a-b+c is (a-b)+c, a*b/c is
        // a*(b/c), a/b*c is (a/b)*c
        let b = |op, l: Box<Expr>, r: Box<Expr>| Box::new(Expr::Binary(op, l, r));
        assert_eq!(
            parse("1+2-3").unwrap(),
            *b(Add, n(1.0), b(Sub, n(2.0), n(3.0)))
        );
        assert_eq!(
            parse("1-2+3").unwrap(),
            *b(Add, b(Sub, n(1.0), n(2.0)), n(3.0))
        );
        assert_eq!(
            parse("1-2-3").unwrap(),
            *b(Sub, b(Sub, n(1.0), n(2.0)), n(3.0))
        );
        assert_eq!(
            parse("1*2/3").unwrap(),
            *b(Mul, n(1.0), b(Div, n(2.0), n(3.0)))
        );
        assert_eq!(
            parse("1/2*3").unwrap(),
            *b(Mul, b(Div, n(1.0), n(2.0)), n(3.0))
        );
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

    /// probe 704: what `scalar s = ...` and `generate` report
    #[test]
    fn errors_match_probe_704() {
        let cases = [
            ("4 \\ 5", "invalid '\\' "),
            ("4 @5", "invalid '@' "),
            ("4 5", "invalid '5' "),
            ("5 -", "invalid syntax"),
            ("(4 @ 5)", "4@5 invalid name"),
            ("2 * (3 - ) + 1", "3- invalid name"),
            ("7 ! 4", "7!4 invalid name"),
            ("3 + * 4", "*4 invalid name"),
            ("3 & | 4", "3& invalid name"),
            ("2 + 1e 3", "1e invalid name"),
            ("4 + (3 + * 4)", "*4 invalid name"),
            ("(7 ! 4)", "7!4 invalid name"),
            ("(3 + 4", "too few ')' or ']'"),
            ("3 + 4)", "too many ')' or ']'"),
        ];
        for (text, message) in cases {
            assert_eq!(parse(text).unwrap_err().message, message, "{text}");
        }
        assert_eq!(parse("(3 + 4").unwrap_err().code, 132);
        // parentheses inside quotes do not count
        assert!(parse("\"(\" + \")\"").is_ok());
    }

    /// probe 705: blanks inside parentheses go before reading; leftovers
    /// and function arguments
    #[test]
    fn errors_match_probe_705() {
        let n = |t: &str| parse(t).unwrap();
        assert_eq!(n("(3 4)"), Expr::Number(34.0));
        assert_eq!(n("(1 e3)"), Expr::Number(1000.0));
        assert_eq!(n("(x y)"), Expr::Name("xy".into()));
        assert_eq!(n("(\"a b\")"), Expr::Text("a b".into()));
        let cases = [
            ("x \"s\"", "invalid '\"s' "),
            ("x (2)", "invalid '(' "),
            ("x 56", "invalid '56' "),
            ("x = 2", "invalid syntax"),
            ("(x + 1]", "too many ')' or ']'"),
            ("(x[1)", "too few ')' or ']'"),
            ("max(1 @ 2, 3)", "1@2 invalid name"),
            ("max(1, 2 @ 3)", "2@3 invalid name"),
            ("abs(,)", ", invalid name"),
            ("x[1 @ 2]", "1@2 invalid name"),
            ("(3 +) + 1", "3+ invalid name"),
        ];
        for (text, message) in cases {
            assert_eq!(parse(text).unwrap_err().message, message, "{text}");
        }
        // in display, an item's parentheses: `1 2)` reads 1, then 2)
        assert_eq!(parse_prefix("2)").unwrap_err().code, 132);
        assert!(parse_prefix("tc(1jan2020 10:00)").is_ok());
    }

    /// probe 753: a compound string keeps every blank, also past an inner
    /// `"`; Stata 19 gives length 5, 4 and 11 for the first three
    #[test]
    fn compound_quote_blanks_match_probe_753() {
        let len = |t: &str| match parse(&format!("length({t})")).unwrap() {
            Expr::Call(_, args) => match args.first() {
                Some(Expr::Text(s)) => s.chars().count(),
                other => panic!("{other:?}"),
            },
            other => panic!("{other:?}"),
        };
        assert_eq!(len("`\"d\t\" e\"'"), 5);
        assert_eq!(len("`\"a\" b\"'"), 4);
        assert_eq!(len("`\"x `\"y z\"' w\"'"), 11);
        assert_eq!(len("\"a b\""), 3);
        // a plain string is unaffected, and the blank outside still goes
        assert_eq!(
            parse("(\"a b\" == \"a b\")").unwrap(),
            parse("(\"a b\"==\"a b\")").unwrap()
        );
        assert_eq!(
            parse("(`\"c\" d\"' + 3 4)").unwrap(),
            parse("(`\"c\" d\"'+34)").unwrap()
        );
    }

    #[test]
    fn errors_match_probe_110() {
        // the probe wraps each in parentheses
        for bad in ["5-", "1e", "0x10", "12_000", "1,2"] {
            let e = parse(&format!("({bad})")).unwrap_err();
            assert_eq!(e.code, 198);
            assert_eq!(e.message, format!("{bad} invalid name"));
        }
        let deep = "(".repeat(500) + "1" + &")".repeat(500);
        assert!(parse(&deep).is_err());
    }

    /// Probe 752 (Stata 19): a + sign belongs to a number written out;
    /// before a name, function, string or another sign it is an error.
    #[test]
    fn unary_plus_matches_probe_752() {
        let bad: &[(&str, &str, u16)] = &[
            ("+x", "+x invalid name", 198),
            ("+x[1]", "+x invalid name", 198),
            ("+_n", "+_n invalid name", 198),
            ("+_pi", "+_pi invalid name", 198),
            ("+_b[x]", "+_b invalid name", 198),
            ("+\"a\"", "+\"a\" invalid name", 198),
            ("1 - +x", "+x invalid name", 198),
            ("2 * +x", "+x invalid name", 198),
            ("2^+x", "+x invalid name", 198),
            ("abs(+x)", "+x invalid name", 198),
            ("max(1, +x)", "+x invalid name", 198),
            ("+(5)", "unknown function +()", 133),
            ("+(x)", "unknown function +()", 133),
            ("abs(+(5))", "unknown function +()", 133),
            ("+abs(-5)", "unknown function +abs()", 133),
            ("+c(pi)", "unknown function +c()", 133),
            ("+r(N)", "unknown function +r()", 133),
            ("+ abs(1)", "unknown function +abs()", 133),
            ("+-3", "+ invalid name", 198),
            ("+-x", "+ invalid name", 198),
            ("++3", "+ invalid name", 198),
            ("+++3", "++ invalid name", 198),
            ("-+-3", "+ invalid name", 198),
            ("2^+-2", "+ invalid name", 198),
            ("2^++2", "+ invalid name", 198),
            ("1 - ++2", "+ invalid name", 198),
            ("+[5]", "+ invalid name", 198),
            ("+!x", "+!x invalid name", 198),
            ("+~0", "+~0 invalid name", 198),
            ("+!!0", "+!!0 invalid name", 198),
            ("+!(0)", "unknown function +!()", 133),
            ("+!abs(1)", "unknown function +!abs()", 133),
            ("+.", "invalid syntax", 198),
            ("+.a", "invalid syntax", 198),
            ("2^+.", "invalid syntax", 198),
            ("-+.", "invalid syntax", 198),
            ("1 + +2", "1+ invalid name", 198),
            ("1++2", "1+ invalid name", 198),
            ("1 + + 2", "1+ invalid name", 198),
            ("1 + +x", "1+ invalid name", 198),
            ("1 + +(2)", "1+ invalid name", 198),
            ("1 + +-2", "1+ invalid name", 198),
            ("2*3 + +4", "3+ invalid name", 198),
            ("1 + 2 + +3", "2+ invalid name", 198),
            ("1 - 2 + +3", "2+ invalid name", 198),
            ("1 / 2 + +3", "2+ invalid name", 198),
            ("1 | 2 + +3", "2+ invalid name", 198),
            ("2 ^ 3 + +4", "3+ invalid name", 198),
            ("1 + +2 + 3", "1+ invalid name", 198),
            ("-1 + +2", "1+ invalid name", 198),
            ("+5 + +5", "+5+ invalid name", 198),
            ("1e3 + +2", "1e3+ invalid name", 198),
            ("sc + +2", "sc+ invalid name", 198),
            ("x + +x", "x+ invalid name", 198),
            ("-x + +2", "x+ invalid name", 198),
            ("(1 + +2)", "1+ invalid name", 198),
            ("abs(1 + +2)", "1+ invalid name", 198),
            ("s + +\"b\"", "s+ invalid name", 198),
            (". + +2", "invalid syntax", 198),
            (".a + +2", "invalid syntax", 198),
            ("r(N) + +2", "invalid syntax", 198),
            ("(1) + +2", "unknown function ()", 133),
            ("abs(1) + +2", "unknown function ()", 133),
            ("(x) + +2", "unknown function ()", 133),
            ("x[1] + +2", "equation [1] not found", 303),
            ("!+3", "invalid syntax", 198),
            ("~+3", "invalid syntax", 198),
            ("!+x", "invalid syntax", 198),
            ("!!+3", "invalid syntax", 198),
            ("-!+3", "invalid syntax", 198),
            ("1 + !+3", "invalid syntax", 198),
            ("1 * !+3", "invalid syntax", 198),
        ];
        for (text, message, code) in bad {
            match parse(text) {
                Ok(_) => panic!("{text} should not parse"),
                Err(e) => assert_eq!((e.message.as_str(), e.code), (*message, *code), "{text}"),
            }
        }
        let good = [
            "+5",
            "+5.5",
            "+1e3",
            "+.5",
            "+5.",
            "+0",
            "+007",
            "+ 5",
            "1 - +2",
            "2 * +3",
            "6 / +3",
            "2^+2",
            "2^+3^2",
            "2 ^ + 2",
            "1 == +1",
            "1 < +2",
            "1 & +1",
            "0 | +1",
            "-+3",
            "2^-+2",
            "+5^2",
            "abs(+5)",
            "max(+1, +2)",
            "(+5)",
            "((+5))",
            "3*[+2]",
            "[+5]",
            "x[+1]",
            "1 + (+2)",
            "2 + 3 * +4",
            "1 + 2 * +3",
            "1 + 2 ^ +3",
            "1 - 2 - +3",
            "1 + -2",
            "1 - -2",
            "1 + !0",
            "1 + ~0",
            "\"a\" + +\"b\"",
            "\"a\" + + \"b\"",
            "\"a\" + \"b\" + +\"c\"",
            "\"a\" * +2",
            "\"a\" + +s",
            "\"a\" + +x",
            "1 + \"a\" + +\"b\"",
        ];
        for text in good {
            assert!(parse(text).is_ok(), "{text}");
        }
    }
}
