//! SW6: the variables a line names, for the check that each is in the data before
//! the line runs ("variable x not found", Stata's r(111)), instead of the raw
//! dplyr or R error.
//!
//! Only names that are plainly variables are returned. A name R or the session
//! might know by another route is left out, so a line that runs is never
//! stopped: a function (`log(`), a name followed by `.` or next to a macro
//! (`L.x`, `x`i'`), one starting with an underscore (`_n`, `_b`), `r()`/`e()`/
//! `c()`/`s()` values, a scalar defined earlier, anything in quotes.

use std::collections::BTreeSet;

use super::{is_name, varlist_names};
use crate::reader::{Command, Prefix};

/// Commands whose `if` and options (`by()`, `over()`, `cluster()`, `absorb()`)
/// are read as data.
const DATA_COMMANDS: &[&str] = &[
    "keep", "drop", "count", "egen", "collapse", "generate", "replace", "tabstat",
];

/// The names the line reads from the data, each once, in the order they come.
pub(super) fn names(c: &Command, scalars: &BTreeSet<String>) -> Vec<String> {
    // a command run in another frame reads that frame's data
    if c.prefixes.iter().any(|p| matches!(p, Prefix::Frame(_))) {
        return Vec::new();
    }
    let mut out: Vec<String> = varlist_names(c);
    let data = DATA_COMMANDS.contains(&c.name.as_str()) || !out.is_empty();
    // by varlist: / bysort varlist:
    for p in &c.prefixes {
        if let Prefix::By { varlist, .. } = p {
            add_list(&mut out, varlist);
        }
    }
    match c.name.as_str() {
        "keep" | "drop" => add_list(&mut out, &c.parts.main),
        // egen new = fn(varlist): the plain names in the argument
        "egen" => {
            if let Some((_, rhs)) = c.parts.main.split_once('=') {
                let rhs = rhs.trim();
                if let (Some(open), true) = (rhs.find('('), rhs.ends_with(')')) {
                    if let Some(args) = rhs.get(open + 1..rhs.len() - 1) {
                        add_list(&mut out, args);
                    }
                }
            }
        }
        // collapse (stat) var newvar=var ...
        "collapse" => {
            let mut depth = 0;
            let mut plain = String::new();
            for ch in c.parts.main.chars() {
                match ch {
                    '(' => depth += 1,
                    ')' => depth -= 1,
                    _ if depth == 0 => plain.push(ch),
                    _ => {}
                }
            }
            // `sy = y` and `sy=y` alike: what follows an equals sign is the variable
            let plain = plain.replace(" =", "=").replace("= ", "=");
            for t in plain.split_whitespace() {
                let t = t.rsplit('=').next().unwrap_or(t);
                add(&mut out, t);
            }
        }
        _ => {}
    }
    if data {
        if let Some(e) = &c.parts.if_exp {
            add_expr(&mut out, e, scalars);
        }
        for opt in ["by", "over", "cluster", "absorb"] {
            if let Some(arg) = option_arg(&c.parts.options, opt) {
                add_list(&mut out, arg.split(',').next().unwrap_or(""));
            }
        }
        if let Some(arg) = option_arg(&c.parts.options, "vce") {
            let mut w = arg.split_whitespace();
            if matches!(w.next(), Some("cluster")) {
                add_list(&mut out, w.next().unwrap_or(""));
            }
        }
    }
    out
}

fn add(out: &mut Vec<String>, t: &str) {
    if is_name(t) && t != "_all" && !out.iter().any(|o| o == t) {
        out.push(t.to_string());
    }
}

/// A varlist: plain names only (a wildcard, range, operator or the `(x)` of
/// `bysort g (x)` is not one), `i.` and `c.` taken off.
fn add_list(out: &mut Vec<String>, text: &str) {
    for t in text.split(|c: char| c.is_whitespace() || c == '(' || c == ')') {
        let t = t
            .strip_prefix("i.")
            .or_else(|| t.strip_prefix("c."))
            .unwrap_or(t);
        add(out, t);
    }
}

/// The text inside `name(...)` among the options, where `name` starts an option.
fn option_arg<'a>(options: &'a str, name: &str) -> Option<&'a str> {
    let mut from = 0;
    while let Some(i) = options.get(from..)?.find(name) {
        let at = from + i;
        let before_ok = at == 0
            || options
                .get(..at)
                .and_then(|s| s.chars().next_back())
                .is_some_and(|c| c.is_whitespace() || c == ',');
        let rest = options.get(at + name.len()..)?;
        if before_ok && rest.starts_with('(') {
            let mut depth = 0;
            for (k, ch) in rest.char_indices() {
                match ch {
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth == 0 {
                            return rest.get(1..k);
                        }
                    }
                    _ => {}
                }
            }
            return None;
        }
        from = at + name.len();
    }
    None
}

/// The plain variable names in an expression (see the top of this file for
/// what is left out).
fn add_expr(out: &mut Vec<String>, text: &str, scalars: &BTreeSet<String>) {
    let chars: Vec<char> = text.chars().collect();
    let at = |i: usize| chars.get(i).copied();
    let word = |c: char| c.is_ascii_alphanumeric() || c == '_';
    let mut i = 0;
    while i < chars.len() {
        let Some(c) = at(i) else { break };
        match c {
            // a string: "..." (a quote inside is doubled or the line is not understood)
            '"' => {
                i += 1;
                while i < chars.len() && at(i).is_some_and(|d| d != '"') {
                    i += 1;
                }
                i += 1;
            }
            // a macro: `name' (nested ones too)
            '`' => {
                let mut depth = 0;
                while i < chars.len() {
                    match at(i).unwrap_or(' ') {
                        '`' => depth += 1,
                        '\'' => {
                            depth -= 1;
                            if depth == 0 {
                                i += 1;
                                break;
                            }
                        }
                        _ => {}
                    }
                    i += 1;
                }
            }
            // a global macro, or anything this does not read
            '$' => {
                i += 1;
                while at(i).is_some_and(word) {
                    i += 1;
                }
            }
            _ if c.is_ascii_digit() => {
                // a number, 1e-5 and 3.2 included; 1.x is not a thing
                while at(i).is_some_and(|d| word(d) || d == '.') {
                    i += 1;
                }
            }
            _ if c.is_ascii_alphabetic() || c == '_' => {
                let start = i;
                while at(i).is_some_and(word) {
                    i += 1;
                }
                let name: String = chars.get(start..i).unwrap_or_default().iter().collect();
                let prev = if start == 0 { None } else { at(start - 1) };
                let next = at(i);
                // _b[x], _se[x], r(mean), e(N): what is inside is not a variable
                if next == Some('[') && name.starts_with('_') {
                    while i < chars.len() && at(i).is_some_and(|d| d != ']') {
                        i += 1;
                    }
                    continue;
                }
                if next == Some('(') && matches!(name.as_str(), "r" | "e" | "c" | "s") {
                    let mut depth = 0;
                    while i < chars.len() {
                        match at(i).unwrap_or(' ') {
                            '(' => depth += 1,
                            ')' => {
                                depth -= 1;
                                if depth == 0 {
                                    i += 1;
                                    break;
                                }
                            }
                            _ => {}
                        }
                        i += 1;
                    }
                    continue;
                }
                // the name of a function, an operator-prefixed term (L.x), a name
                // run into a macro, a missing code (.a), or a name R keeps
                // elsewhere: not read as a variable
                if next == Some('(')
                    || next == Some('.')
                    || next == Some('`')
                    || next == Some('[')
                    || prev == Some('.')
                    || prev == Some('\'')
                    || prev == Some('}')
                    || name.starts_with('_')
                    || scalars.contains(&name)
                {
                    continue;
                }
                add(out, &name);
            }
            _ => i += 1,
        }
    }
}

/// SW11: the variables an `if` compares with a string literal (`true`) or with a
/// number (`false`) directly (`g == "a"`, `3 < x`), for the check that each is
/// of that kind before the line runs: a number or labelled number against a
/// string, a string against a number, is Stata's "type mismatch", r(109).
/// Only a plain variable beside a literal counts; anything built (`f(x)`, `x[1]`,
/// `L.x`, a scalar or a name next to a macro) is left to R.
pub(super) fn type_pairs(c: &Command, scalars: &BTreeSet<String>) -> Vec<(String, bool)> {
    if c.prefixes.iter().any(|p| matches!(p, Prefix::Frame(_))) {
        return Vec::new();
    }
    let data = DATA_COMMANDS.contains(&c.name.as_str()) || !varlist_names(c).is_empty();
    match (&c.parts.if_exp, data) {
        (Some(e), true) => compared(e, scalars),
        _ => Vec::new(),
    }
}

/// One side of a comparison.
#[derive(Debug, PartialEq)]
enum Side {
    Var(String),
    Str,
    Num,
    Other,
}

fn compared(text: &str, scalars: &BTreeSet<String>) -> Vec<(String, bool)> {
    let chars: Vec<char> = text.chars().collect();
    let at = |i: usize| chars.get(i).copied();
    let mut out: Vec<(String, bool)> = Vec::new();
    let mut i = 0;
    while let Some(c) = at(i) {
        match c {
            // a string: its contents are not operators
            '"' => {
                i += 1;
                while at(i).is_some_and(|d| d != '"') {
                    i += 1;
                }
                i += 1;
            }
            '=' | '!' | '~' | '<' | '>' => {
                let two = at(i + 1) == Some('=');
                // `=` alone is no comparison; `!` and `~` need the `=`
                let len = match (c, two) {
                    (_, true) => 2,
                    ('<' | '>', false) => 1,
                    _ => 0,
                };
                if len == 0 {
                    i += 1;
                    continue;
                }
                let left = side_before(&chars, i, scalars);
                let right = side_after(&chars, i + len, scalars);
                match (left, right) {
                    (Side::Var(v), Side::Str) | (Side::Str, Side::Var(v)) => {
                        push(&mut out, v, true)
                    }
                    (Side::Var(v), Side::Num) | (Side::Num, Side::Var(v)) => {
                        push(&mut out, v, false)
                    }
                    _ => {}
                }
                i += len;
            }
            _ => i += 1,
        }
    }
    out
}

fn push(out: &mut Vec<(String, bool)>, v: String, string: bool) {
    if !out.iter().any(|(o, s)| *o == v && *s == string) {
        out.push((v, string));
    }
}

fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// The operand that ends just before position `end`.
fn side_before(chars: &[char], end: usize, scalars: &BTreeSet<String>) -> Side {
    let mut j = end;
    while j > 0 && chars.get(j - 1).is_some_and(|c| c.is_whitespace()) {
        j -= 1;
    }
    let Some(last) = j.checked_sub(1).and_then(|k| chars.get(k)).copied() else {
        return Side::Other;
    };
    if last == '"' {
        return Side::Str;
    }
    if !is_word(last) {
        return Side::Other;
    }
    let mut k = j;
    while k > 0 && chars.get(k - 1).is_some_and(|c| is_word(*c) || *c == '.') {
        k -= 1;
    }
    let text: String = chars.get(k..j).unwrap_or_default().iter().collect();
    // beside a macro or a closing bracket: not a plain operand
    let before = k.checked_sub(1).and_then(|p| chars.get(p)).copied();
    if matches!(before, Some('\'' | '`' | '$' | '}' | ']' | ')')) {
        return Side::Other;
    }
    classify(&text, scalars)
}

/// The operand that starts at or after position `start`.
fn side_after(chars: &[char], start: usize, scalars: &BTreeSet<String>) -> Side {
    let mut j = start;
    while chars.get(j).is_some_and(|c| c.is_whitespace()) {
        j += 1;
    }
    if chars.get(j) == Some(&'"') {
        return Side::Str;
    }
    if chars.get(j) == Some(&'-') && chars.get(j + 1).is_some_and(|c| c.is_ascii_digit()) {
        j += 1;
    }
    let mut k = j;
    while chars.get(k).is_some_and(|c| is_word(*c) || *c == '.') {
        k += 1;
    }
    let text: String = chars.get(j..k).unwrap_or_default().iter().collect();
    // a call, a subscript or a macro follows: not a plain operand
    if matches!(chars.get(k), Some('(' | '[' | '`')) {
        return Side::Other;
    }
    classify(&text, scalars)
}

fn classify(text: &str, scalars: &BTreeSet<String>) -> Side {
    let digit = text.chars().next().is_some_and(|c| c.is_ascii_digit());
    if text.is_empty() {
        return Side::Other;
    }
    if digit {
        return if text.parse::<f64>().is_ok() {
            Side::Num
        } else {
            Side::Other
        };
    }
    // a name with a dot (L.x, a missing code .a) or an underscore start is left to R
    if text.starts_with('_') || scalars.contains(text) || !is_name(text) {
        return Side::Other;
    }
    Side::Var(text.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exp(s: &str) -> Vec<String> {
        let mut out = Vec::new();
        add_expr(&mut out, s, &BTreeSet::new());
        out
    }

    #[test]
    fn an_expression_gives_its_variables_and_nothing_else() {
        assert_eq!(exp("x > 0 & g == \"a\""), ["x", "g"]);
        assert_eq!(exp("log(w) + x[_n-1] * 2.5e-3"), ["w"]);
        assert_eq!(exp("r(mean) < x | _n == 1"), ["x"]);
        assert_eq!(
            exp("L.x > 3 & x`i' < 2 & `v' == 1 & $g == 4"),
            Vec::<String>::new()
        );
        assert_eq!(
            exp("inlist(g, \"a\", \"b\") & !missing(y) & z != .a"),
            ["g", "y", "z"]
        );
        assert_eq!(exp("_b[x] > 0 & e(N) > c(pi)"), Vec::<String>::new());
    }

    #[test]
    fn a_scalar_is_not_a_variable() {
        let s: BTreeSet<String> = ["cut".to_string()].into();
        let mut out = Vec::new();
        add_expr(&mut out, "x > cut", &s);
        assert_eq!(out, ["x"]);
    }

    #[test]
    fn option_arguments_are_found_by_name() {
        assert_eq!(option_arg("by(g) sort", "by"), Some("g"));
        assert_eq!(option_arg("over(g, total) nolab", "over"), Some("g, total"));
        assert_eq!(option_arg("nobyte", "by"), None);
        assert_eq!(option_arg("vce(cluster id)", "vce"), Some("cluster id"));
    }
}

#[cfg(test)]
mod type_tests {
    use super::*;

    fn pairs(s: &str) -> Vec<(String, bool)> {
        compared(s, &BTreeSet::new())
    }

    #[test]
    fn a_plain_variable_beside_a_literal_is_paired_with_its_kind() {
        let p = |v: &str, s: bool| (v.to_string(), s);
        assert_eq!(pairs("g == \"a\""), [p("g", true)]);
        assert_eq!(pairs("\"a\" != g & x > 3"), [p("g", true), p("x", false)]);
        assert_eq!(pairs("x <= -1.5 | 2 < y"), [p("x", false), p("y", false)]);
        assert_eq!(pairs("s ~= \"b\""), [p("s", true)]);
    }

    #[test]
    fn anything_built_is_left_to_r() {
        for e in [
            "f(x) == 3",
            "x[1] == \"a\"",
            "L.x == 3",
            "x == y",
            "x = 3",
            "_n == 3",
            "x == .",
            "x == `v'",
            "r(mean) > 3",
        ] {
            assert_eq!(pairs(e), Vec::<(String, bool)>::new(), "{e}");
        }
        let mut s = BTreeSet::new();
        s.insert("cut".to_string());
        assert!(compared("cut == \"a\"", &s).is_empty());
    }
}
