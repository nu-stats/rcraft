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
use crate::lang::expr::{is_ts_ops, parse, BinaryOp, Expr};
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
        // generate/replace new = exp: the variables of the expression
        "generate" | "replace" => {
            if let Some((_, rhs)) = c.parts.main.split_once('=') {
                add_expr(&mut out, rhs, scalars);
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

/// The plain variable names in an expression, read with the expression parser
/// (see the top of this file for what is left out). Text the parser cannot
/// read, or that holds a macro, gives nothing: R, or the line's own error, says
/// what is wrong with it.
fn add_expr(out: &mut Vec<String>, text: &str, scalars: &BTreeSet<String>) {
    if text.contains(['`', '$']) {
        return;
    }
    if let Ok(e) = parse(text) {
        let mut found = Vec::new();
        walk_names(&e, &mut found, scalars);
        // the reader runs `(x y)` together as one name: a name counts only if it is
        // written as a whole word in the text
        for n in found {
            if has_word(text, &n) {
                add(out, &n);
            }
        }
    }
}

fn has_word(text: &str, name: &str) -> bool {
    let word = |c: char| c.is_ascii_alphanumeric() || c == '_';
    text.match_indices(name).any(|(i, m)| {
        let before = text.get(..i).and_then(|t| t.chars().next_back());
        let after = text.get(i + m.len()..).and_then(|t| t.chars().next());
        !before.is_some_and(word) && !after.is_some_and(word)
    })
}

fn walk_names(e: &Expr, out: &mut Vec<String>, scalars: &BTreeSet<String>) {
    match e {
        Expr::Number(_) | Expr::Text(_) => {}
        Expr::Name(n) => {
            // L.x and the like, _n, _N, _pi, _rc, a scalar or a matrix: not variables
            let ts = n.split_once('.').is_some_and(|(ops, _)| is_ts_ops(ops));
            if !ts && !n.starts_with('_') && !n.contains('.') && !scalars.contains(n) {
                add(out, n);
            }
        }
        Expr::Subscript(n, index) => {
            if !n.starts_with('_') && !scalars.contains(n) {
                add(out, n);
                walk_names(index, out, scalars);
            }
        }
        // r(mean), e(N), c(pi), s(...): what is inside is not a variable
        Expr::Call(f, _) if matches!(f.as_str(), "r" | "e" | "c" | "s") => {}
        Expr::Call(_, args) => {
            for a in args {
                walk_names(a, out, scalars);
            }
        }
        Expr::Unary(_, a) => walk_names(a, out, scalars),
        Expr::Binary(_, a, b) => {
            walk_names(a, out, scalars);
            walk_names(b, out, scalars);
        }
    }
}

/// What the type checks found in an `if`.
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Types {
    /// A plain variable compared with a string (`true`) or a number (`false`):
    /// checked in R against the variable's own type.
    pub pairs: Vec<(String, bool)>,
    /// A string compared with a number whatever the data hold
    /// (`substr(s, 1, 1) == 3`, `abs(x) == "a"`).
    pub always: bool,
}

/// SW11, SW13: the comparisons in an `if` whose two sides are of different kinds,
/// for Stata's "type mismatch", r(109), before the line runs. Only a plain
/// variable beside a string or number, or two sides whose kinds the functions
/// fix (`abs` gives a number, `substr` text), count; any other expression is
/// left to R.
pub(super) fn types(c: &Command, scalars: &BTreeSet<String>) -> Types {
    let mut t = Types::default();
    if c.prefixes.iter().any(|p| matches!(p, Prefix::Frame(_))) {
        return t;
    }
    let data = DATA_COMMANDS.contains(&c.name.as_str()) || !varlist_names(c).is_empty();
    if let (Some(text), true) = (&c.parts.if_exp, data) {
        if !text.contains(['`', '$']) {
            if let Ok(e) = parse(text) {
                walk_types(&e, scalars, &mut t);
            }
        }
    }
    t
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fixed {
    Text,
    Number,
}

/// The kind an expression has whatever the data hold, when it is fixed.
fn fixed(e: &Expr) -> Option<Fixed> {
    const NUMBER_FNS: &[&str] = &[
        "abs", "log", "ln", "sqrt", "exp", "int", "round", "floor", "ceil", "real", "length",
        "strlen", "mod", "min", "max", "missing", "inlist", "inrange", "sign", "trunc",
    ];
    const TEXT_FNS: &[&str] = &[
        "substr",
        "lower",
        "upper",
        "trim",
        "ltrim",
        "rtrim",
        "itrim",
        "string",
        "strtrim",
        "proper",
        "subinstr",
        "reverse",
        "strreverse",
        "word",
    ];
    match e {
        Expr::Number(_) => Some(Fixed::Number),
        Expr::Text(_) => Some(Fixed::Text),
        Expr::Call(f, _) if NUMBER_FNS.contains(&f.as_str()) => Some(Fixed::Number),
        Expr::Call(f, _) if TEXT_FNS.contains(&f.as_str()) => Some(Fixed::Text),
        Expr::Unary(_, _) => Some(Fixed::Number),
        Expr::Binary(op, a, b) => match op {
            BinaryOp::Add => match (fixed(a), fixed(b)) {
                (Some(Fixed::Text), _) | (_, Some(Fixed::Text)) => Some(Fixed::Text),
                (Some(Fixed::Number), Some(Fixed::Number)) => Some(Fixed::Number),
                _ => None,
            },
            // arithmetic, comparisons and and/or all give numbers
            _ => Some(Fixed::Number),
        },
        _ => None,
    }
}

fn walk_types(e: &Expr, scalars: &BTreeSet<String>, t: &mut Types) {
    match e {
        Expr::Unary(_, a) => walk_types(a, scalars, t),
        Expr::Call(_, args) => {
            for a in args {
                walk_types(a, scalars, t);
            }
        }
        Expr::Binary(op, a, b) => {
            walk_types(a, scalars, t);
            walk_types(b, scalars, t);
            if matches!(
                op,
                BinaryOp::Eq
                    | BinaryOp::Ne
                    | BinaryOp::Lt
                    | BinaryOp::Le
                    | BinaryOp::Gt
                    | BinaryOp::Ge
            ) {
                let var = |x: &Expr| match x {
                    Expr::Name(n)
                        if !n.starts_with('_') && !n.contains('.') && !scalars.contains(n) =>
                    {
                        Some(n.clone())
                    }
                    _ => None,
                };
                match (fixed(a), fixed(b), var(a), var(b)) {
                    (Some(x), Some(y), _, _) if x != y => t.always = true,
                    (None, Some(k), Some(v), _) | (Some(k), None, _, Some(v)) => {
                        let pair = (v, k == Fixed::Text);
                        if !t.pairs.contains(&pair) {
                            t.pairs.push(pair);
                        }
                    }
                    _ => {}
                }
            }
        }
        _ => {}
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
        assert_eq!(exp("log(w) + x[_n-1] * 2.5e-3"), ["w", "x"]);
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
        // text the reader cannot take is left to the line's own error
        assert_eq!(exp("(x y)"), Vec::<String>::new());
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

    fn kinds(s: &str) -> Types {
        let mut t = Types::default();
        if let Ok(e) = parse(s) {
            walk_types(&e, &BTreeSet::new(), &mut t);
        }
        t
    }

    #[test]
    fn a_plain_variable_beside_a_literal_is_paired_with_its_kind() {
        let p = |v: &str, s: bool| (v.to_string(), s);
        assert_eq!(kinds("g == \"a\"").pairs, [p("g", true)]);
        assert_eq!(
            kinds("\"a\" != g & x > 3").pairs,
            [p("g", true), p("x", false)]
        );
        assert_eq!(
            kinds("x <= -1.5 | 2 < y").pairs,
            [p("x", false), p("y", false)]
        );
        // a function fixes the kind of the other side
        assert_eq!(kinds("g == substr(t, 1, 1)").pairs, [p("g", true)]);
        assert_eq!(kinds("s == abs(x)").pairs, [p("s", false)]);
    }

    #[test]
    fn two_sides_of_fixed_kinds_that_differ_always_mismatch() {
        assert!(kinds("abs(x) == \"a\"").always);
        assert!(kinds("substr(s, 1, 1) == 3").always);
        assert!(kinds("\"a\" + \"b\" > 2").always);
        assert!(!kinds("substr(s, 1, 1) == \"a\"").always);
        assert!(!kinds("abs(x) == 3").always);
        assert!(!kinds("x == y").always);
    }

    #[test]
    fn anything_not_fixed_is_left_to_r() {
        for e in [
            "f(x) == 3",
            "x[1] == \"a\"",
            "L.x == 3",
            "x == y",
            "_n == 3",
            "r(mean) > 3",
        ] {
            let t = kinds(e);
            assert!(t.pairs.is_empty() && !t.always, "{e}: {t:?}");
        }
        let s: BTreeSet<String> = ["cut".to_string()].into();
        let mut t = Types::default();
        if let Ok(e) = parse("cut == \"a\"") {
            walk_types(&e, &s, &mut t);
        }
        assert!(t.pairs.is_empty());
    }
}
