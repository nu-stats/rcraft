//! Macros in a line of a do-file: `` `name' ``, `$name`, `${name}`, the
//! nested `` `x`i'' `` (inside first), `` `=exp' ``, `` `: word count ...' ``,
//! `` `++i' `` and the escapes `` \` `` and `\$`. Stata replaces them in the
//! text of a line before it reads the line, and so does this.
//!
//! The rules here were observed in Stata 19 with made-up values
//! (`plan/STATUS.md`, P2C): an undefined macro is empty, `$g_x` is the
//! global `g_x`, the text a macro holds is looked at again for macros (so an
//! escaped `` \`i' `` kept in one is replaced where that one is used), and
//! a word list's quotes group words.

/// What replacing a line's macros needs from the do-file being run.
pub trait Env {
    /// A local macro's text; an error says why it is not known.
    fn local(&mut self, name: &str) -> Result<String, Stop>;
    fn global(&mut self, name: &str) -> Result<String, Stop>;
    /// Sets a local (`` `++i' ``); kept until the line is done.
    fn set_local(&mut self, name: &str, value: String);
    /// `` `=exp' ``.
    fn inline(&mut self, exp: &str) -> Result<String, Stop>;
    /// `` `: function ...' ``.
    fn extended(&mut self, text: &str) -> Result<String, Stop>;
}

/// Why a line's macros could not all be replaced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stop {
    /// R works it out when the do-file runs: the value of this expression,
    /// or (`display`) the text `display` prints.
    Ask { exp: String, display: bool },
    /// It is known only when the do-file runs, or R could not work it out;
    /// why, in words.
    Later(String),
    /// It is not translated, and why.
    Not(String),
    /// Stata stops here with this error.
    Stata(String),
}

const ESC_TICK: char = '\u{E000}';
const ESC_DOLLAR: char = '\u{E001}';
/// Most replacements in one line (a macro that holds itself would not end).
const MAX_ROUNDS: usize = 10_000;
/// Longest line made.
const MAX_LEN: usize = 4_000_000;

/// `text` with its macros replaced.
pub fn expand(text: &str, env: &mut dyn Env) -> Result<String, Stop> {
    let mut s = String::with_capacity(text.len());
    let mut it = text.chars().peekable();
    while let Some(c) = it.next() {
        match (c, it.peek()) {
            ('\\', Some('`')) => {
                it.next();
                s.push(ESC_TICK);
            }
            ('\\', Some('$')) => {
                it.next();
                s.push(ESC_DOLLAR);
            }
            _ => s.push(c),
        }
    }
    for _ in 0..MAX_ROUNDS {
        if s.len() > MAX_LEN {
            return Err(Stop::Stata(
                "macro substitution results in a line too long".into(),
            ));
        }
        let chars: Vec<char> = s.chars().collect();
        if let Some((open, close)) = innermost_local(&chars) {
            let inside: String = chars.get(open + 1..close).unwrap_or(&[]).iter().collect();
            let inside = expand_globals(&inside, env)?;
            let value = local_value(inside.trim_matches(' '), env)?;
            let before: String = chars.get(..open).unwrap_or(&[]).iter().collect();
            let after: String = chars.get(close + 1..).unwrap_or(&[]).iter().collect();
            s = format!("{before}{value}{after}");
            continue;
        }
        let next = expand_first_global(&s, env)?;
        match next {
            Some(t) => s = t,
            None => break,
        }
    }
    Ok(s.replace(ESC_TICK, "`").replace(ESC_DOLLAR, "$"))
}

/// The first `'` that closes a macro, and the backtick that opens it (the
/// nearest one before it, so the innermost). A backtick before `"` opens
/// compound quotes, not a macro.
fn innermost_local(chars: &[char]) -> Option<(usize, usize)> {
    let mut open: Option<usize> = None;
    for (i, c) in chars.iter().enumerate() {
        match c {
            '`' if chars.get(i + 1) != Some(&'"') => open = Some(i),
            '\'' => {
                if let Some(o) = open {
                    return Some((o, i));
                }
            }
            _ => {}
        }
    }
    None
}

fn local_value(inside: &str, env: &mut dyn Env) -> Result<String, Stop> {
    if let Some(exp) = inside.strip_prefix('=') {
        return env.inline(exp);
    }
    if let Some(f) = inside.strip_prefix(':') {
        return env.extended(f.trim());
    }
    for (pre, post, step) in [
        ("++", "", 1i64),
        ("--", "", -1),
        ("", "++", 1),
        ("", "--", -1),
    ] {
        let name = inside
            .strip_prefix(pre)
            .and_then(|n| n.strip_suffix(post))
            .filter(|n| (pre.len() + post.len()) > 0 && is_name(n));
        if let Some(name) = name {
            let old = env.local(name)?;
            let x: f64 = if old.trim().is_empty() {
                0.0
            } else {
                old.trim()
                    .parse()
                    .map_err(|_| Stop::Stata(format!("{inside}: `{name}' is not a number")))?
            };
            let new = super::format::g(x + step as f64, 18);
            env.set_local(name, new.clone());
            return Ok(if pre.is_empty() { old } else { new });
        }
    }
    env.local(inside)
}

fn is_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

pub(super) fn is_name(s: &str) -> bool {
    let mut c = s.chars();
    matches!(c.next(), Some(f) if f.is_ascii_alphabetic() || f == '_')
        && s.chars().all(is_name_char)
}

/// `$name` and `${name}` replaced everywhere in `text`.
fn expand_globals(text: &str, env: &mut dyn Env) -> Result<String, Stop> {
    let mut s = text.to_string();
    for _ in 0..MAX_ROUNDS {
        match expand_first_global(&s, env)? {
            Some(t) => s = t,
            None => return Ok(s),
        }
    }
    Ok(s)
}

/// The first global replaced, if there is one.
fn expand_first_global(text: &str, env: &mut dyn Env) -> Result<Option<String>, Stop> {
    let chars: Vec<char> = text.chars().collect();
    for (i, c) in chars.iter().enumerate() {
        if *c != '$' {
            continue;
        }
        let (name, end) = if chars.get(i + 1) == Some(&'{') {
            let Some(close) = (i + 2..chars.len()).find(|&k| chars.get(k) == Some(&'}')) else {
                continue;
            };
            let name: String = chars.get(i + 2..close).unwrap_or(&[]).iter().collect();
            (name, close + 1)
        } else {
            let len = chars
                .get(i + 1..)
                .unwrap_or(&[])
                .iter()
                .take_while(|c| is_name_char(**c))
                .count();
            let name: String = chars
                .get(i + 1..i + 1 + len)
                .unwrap_or(&[])
                .iter()
                .collect();
            (name, i + 1 + len)
        };
        if !is_name(name.trim()) {
            continue;
        }
        let value = env.global(name.trim())?;
        let before: String = chars.get(..i).unwrap_or(&[]).iter().collect();
        let after: String = chars.get(end..).unwrap_or(&[]).iter().collect();
        return Ok(Some(format!("{before}{value}{after}")));
    }
    Ok(None)
}

/// The words of a list, as `foreach ... in`, `word count` and `word # of`
/// take them: split at spaces, with `"a b"` and `` `"a "b""' `` one word
/// each, their outer quotes taken off.
pub fn words(text: &str) -> Vec<String> {
    tokens(text).into_iter().map(|t| unquote(&t)).collect()
}

/// The words of a list as written, quotes kept (`: list` keeps them).
pub fn tokens(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars.get(i).is_some_and(|c| c.is_whitespace()) {
            i += 1;
            continue;
        }
        let start = i;
        if chars.get(i) == Some(&'`') && chars.get(i + 1) == Some(&'"') {
            let mut depth = 0i32;
            while i < chars.len() {
                if chars.get(i) == Some(&'`') && chars.get(i + 1) == Some(&'"') {
                    depth += 1;
                    i += 2;
                } else if chars.get(i) == Some(&'"') && chars.get(i + 1) == Some(&'\'') {
                    depth -= 1;
                    i += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    i += 1;
                }
            }
        } else if chars.get(i) == Some(&'"') {
            i += 1;
            while i < chars.len() && chars.get(i) != Some(&'"') {
                i += 1;
            }
            i = (i + 1).min(chars.len());
        } else {
            while i < chars.len() && !chars.get(i).is_some_and(|c| c.is_whitespace()) {
                i += 1;
            }
        }
        out.push(chars.get(start..i).unwrap_or(&[]).iter().collect());
    }
    out
}

/// One pair of outer quotes taken off: `"a b"` and `` `"a "b""' ``.
pub fn unquote(t: &str) -> String {
    if let Some(inner) = t.strip_prefix("`\"").and_then(|r| r.strip_suffix("\"'")) {
        return inner.to_string();
    }
    if t.len() >= 2 {
        if let Some(inner) = t.strip_prefix('"').and_then(|r| r.strip_suffix('"')) {
            return inner.to_string();
        }
    }
    t.to_string()
}

/// The text `local name text` keeps: one pair of outer quotes taken off
/// (`"a" "b"` keeps `a" "b`), else the text without spaces around it.
pub fn definition_text(rest: &str) -> String {
    let t = rest.trim();
    let quoted = (t.starts_with("`\"") && t.ends_with("\"'"))
        || (t.len() >= 2 && t.starts_with('"') && t.ends_with('"'));
    if quoted {
        unquote(t)
    } else {
        t.to_string()
    }
}

/// `: list ...` on lists the caller has looked up, as Stata 19 does them.
pub enum ListOp {
    Uniq,
    Dups,
    Sort,
    Sizeof,
    Retokenize,
    Minus,
    Union,
    Intersect,
    In,
    Same,
    SameSet,
}

pub fn list_op(op: ListOp, a: &str, b: &str) -> String {
    let a = tokens(a);
    let b = tokens(b);
    let join = |v: Vec<String>| v.join(" ");
    let bool_text = |x: bool| if x { "1" } else { "0" }.to_string();
    match op {
        ListOp::Uniq => {
            let mut out: Vec<String> = Vec::new();
            for t in a {
                if !out.contains(&t) {
                    out.push(t);
                }
            }
            join(out)
        }
        ListOp::Dups => {
            let mut seen: Vec<&String> = Vec::new();
            let mut out = Vec::new();
            for t in &a {
                if seen.contains(&t) {
                    out.push(t.clone());
                } else {
                    seen.push(t);
                }
            }
            join(out)
        }
        ListOp::Sort => {
            let mut v = a;
            v.sort_by_key(|t| unquote(t));
            join(v)
        }
        ListOp::Sizeof => a.len().to_string(),
        ListOp::Retokenize => join(a),
        // the last copy of each element of b goes
        ListOp::Minus => {
            let mut v = a;
            for t in &b {
                if let Some(at) = v.iter().rposition(|x| x == t) {
                    v.remove(at);
                }
            }
            join(v)
        }
        ListOp::Union => {
            let mut v = a;
            for t in b {
                if !v.contains(&t) {
                    v.push(t);
                }
            }
            join(v)
        }
        // each element of b matches one in a, in a's order
        ListOp::Intersect => {
            let mut left = b;
            let mut out = Vec::new();
            for t in a {
                if let Some(at) = left.iter().position(|x| *x == t) {
                    left.remove(at);
                    out.push(t);
                }
            }
            join(out)
        }
        ListOp::In => bool_text(a.iter().all(|t| b.contains(t))),
        ListOp::Same => bool_text(a == b),
        ListOp::SameSet => {
            bool_text(a.iter().all(|t| b.contains(t)) && b.iter().all(|t| a.contains(t)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[derive(Default)]
    struct Map {
        locals: BTreeMap<String, String>,
        globals: BTreeMap<String, String>,
    }

    impl Env for Map {
        fn local(&mut self, name: &str) -> Result<String, Stop> {
            Ok(self.locals.get(name).cloned().unwrap_or_default())
        }
        fn global(&mut self, name: &str) -> Result<String, Stop> {
            Ok(self.globals.get(name).cloned().unwrap_or_default())
        }
        fn set_local(&mut self, name: &str, value: String) {
            self.locals.insert(name.into(), value);
        }
        fn inline(&mut self, exp: &str) -> Result<String, Stop> {
            super::super::eval::eval_text(exp)
                .map(|v| v.macro_text())
                .map_err(|_| Stop::Later(exp.into()))
        }
        fn extended(&mut self, text: &str) -> Result<String, Stop> {
            Ok(format!("<{text}>"))
        }
    }

    fn env() -> Map {
        let mut m = Map::default();
        for (k, v) in [
            ("i", "1"),
            ("x1", "hello"),
            ("a", "x"),
            ("esc", "`i'"),
            ("n", "3"),
        ] {
            m.locals.insert(k.into(), v.into());
        }
        m.globals.insert("g".into(), "world".into());
        m
    }

    /// What Stata 19 printed for the same lines.
    #[test]
    fn macros_are_replaced_as_stata_does() {
        let mut m = env();
        let mut e = |t: &str| expand(t, &mut m).unwrap();
        assert_eq!(e("di \"[`x`i'']\""), "di \"[hello]\"");
        assert_eq!(e("[`=1+2']"), "[3]");
        assert_eq!(e("[`undefined']"), "[]");
        assert_eq!(
            e("[$g] [${g}] [$g_x] [${g}_x]"),
            "[world] [world] [] [world_x]"
        );
        assert_eq!(e("[`=2*`n'']"), "[6]");
        // an escaped macro is kept as text, and replaced where that text is used
        assert_eq!(e("local esc \"\\`i'\""), "local esc \"`i'\"");
        assert_eq!(e("[`esc']"), "[1]");
        assert_eq!(e("di `\"[`a'] \"q\"\"'"), "di `\"[x] \"q\"\"'");
        assert_eq!(e("[`: word count a b']"), "[<word count a b>]");
        assert_eq!(e("a $ b $1"), "a $ b $1");
    }

    #[test]
    fn plus_plus_changes_the_local() {
        let mut m = env();
        assert_eq!(expand("[`++i'] [`i']", &mut m).unwrap(), "[2] [2]");
        assert_eq!(expand("[`i++'] [`i']", &mut m).unwrap(), "[2] [3]");
        assert_eq!(expand("[`--i']", &mut m).unwrap(), "[2]");
    }

    #[test]
    fn words_and_definitions() {
        assert_eq!(words("\"a b\" c `\"d \"e\"\"'"), ["a b", "c", "d \"e\""]);
        assert_eq!(
            definition_text("    spaced   words   here  "),
            "spaced   words   here"
        );
        assert_eq!(definition_text("\"a\" \"b\""), "a\" \"b");
        assert_eq!(definition_text("`\"a \"b\" c\"'"), "a \"b\" c");
        assert_eq!(definition_text("\"  lead\""), "  lead");
    }

    /// `: list` in Stata 19, with repeated elements.
    #[test]
    fn list_functions_are_statas() {
        let a = "a b c b a";
        assert_eq!(list_op(ListOp::Minus, a, "b x"), "a b c a");
        assert_eq!(list_op(ListOp::Minus, a, "a a"), "b c b");
        assert_eq!(list_op(ListOp::Minus, a, "b a z"), "a b c");
        assert_eq!(list_op(ListOp::Union, a, "b x"), "a b c b a x");
        assert_eq!(list_op(ListOp::Union, "a a", "b a z"), "a a b z");
        assert_eq!(list_op(ListOp::Intersect, a, "b a z"), "a b");
        assert_eq!(list_op(ListOp::Intersect, "a a", a), "a a");
        assert_eq!(list_op(ListOp::Intersect, "b a z", "a a"), "a");
        assert_eq!(list_op(ListOp::In, "b x", a), "0");
        assert_eq!(list_op(ListOp::In, "a a", a), "1");
        assert_eq!(list_op(ListOp::Same, "c b a", "a b c"), "0");
        assert_eq!(list_op(ListOp::SameSet, "c b a", "a b c"), "1");
        assert_eq!(list_op(ListOp::Uniq, a, ""), "a b c");
        assert_eq!(list_op(ListOp::Dups, a, ""), "b a");
        assert_eq!(list_op(ListOp::Sort, a, ""), "a a b b c");
        assert_eq!(list_op(ListOp::Sort, "b \"a c\" a", ""), "a \"a c\" b");
        assert_eq!(list_op(ListOp::Sort, "10 9 2 1", ""), "1 10 2 9");
        assert_eq!(
            list_op(ListOp::Retokenize, " x   y   \"z w\"  ", ""),
            "x y \"z w\""
        );
        assert_eq!(list_op(ListOp::Sizeof, "x y \"z w\"", ""), "3");
    }
}
