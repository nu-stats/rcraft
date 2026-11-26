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
