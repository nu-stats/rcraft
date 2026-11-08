//! A varlist with the data's variable names known (P2C): an abbreviation
//! made the one name it stands for, `x*`, `x?` and `x~` the names they
//! match, `a-c` the names from `a` to `c` in the data's order, `_all` every
//! name. The rules and Stata's errors are the ones Stata 19 showed with
//! made-up data: `x` with x1 and x2 in the data is `x ambiguous
//! abbreviation`, `zz*` with nothing to match is `variable zz* not found`.
//!
//! Only the variables are touched: the operators of factor variables
//! (`i.`, `c.`, `ib2.`, `L.`), `#`, parentheses and `=` (ivregress) are kept
//! as written.

/// What one name, pattern or range stands for.
pub fn resolve(atom: &str, vars: &[String]) -> Result<Vec<String>, String> {
    if atom == "_all" || atom == "*" {
        return Ok(vars.to_vec());
    }
    if atom.contains(['*', '?', '~']) {
        let matched: Vec<String> = vars.iter().filter(|v| wildcard(atom, v)).cloned().collect();
        if matched.is_empty() {
            return Err(format!("variable {atom} not found"));
        }
        if atom.contains('~') && matched.len() > 1 {
            return Err(format!("{atom} ambiguous abbreviation"));
        }
        return Ok(matched);
    }
    if let Some((a, b)) = atom.split_once('-') {
        let first = one(a, vars)?;
        let last = one(b, vars)?;
        let i = vars.iter().position(|v| *v == first).unwrap_or(0);
        let j = vars.iter().position(|v| *v == last).unwrap_or(0);
        if j < i {
            return Err(format!(
                "{atom}: invalid varlist (the range runs backwards)"
            ));
        }
        return Ok(vars.get(i..=j).unwrap_or_default().to_vec());
    }
    match unique(atom, vars) {
        Ok(Some(v)) => Ok(vec![v]),
        // a name the data does not have: as written (a new variable, or R says)
        Ok(None) => Ok(vec![atom.to_string()]),
        Err(e) => Err(e),
    }
}

/// A name or the one variable it abbreviates, for the ends of a range.
fn one(word: &str, vars: &[String]) -> Result<String, String> {
    match unique(word, vars)? {
        Some(v) => Ok(v),
        None => Err(format!("variable {word} not found")),
    }
}

/// The variable a name or a unique abbreviation of one means; `None` when
/// none begins with it.
fn unique(word: &str, vars: &[String]) -> Result<Option<String>, String> {
    if vars.iter().any(|v| v == word) {
        return Ok(Some(word.to_string()));
    }
    let mut found = vars.iter().filter(|v| v.starts_with(word));
    match (found.next(), found.next()) {
        (Some(v), None) => Ok(Some(v.clone())),
        (Some(_), Some(_)) => Err(format!("{word} ambiguous abbreviation")),
        _ => Ok(None),
    }
}

/// Stata's wildcards: `*` and `~` any characters, `?` one.
fn wildcard(pattern: &str, name: &str) -> bool {
    fn go(p: &[char], n: &[char]) -> bool {
        match p.split_first() {
            None => n.is_empty(),
            Some(('*' | '~', rest)) => (0..=n.len()).any(|k| go(rest, n.get(k..).unwrap_or(&[]))),
            Some(('?', rest)) => n.split_first().is_some_and(|(_, tail)| go(rest, tail)),
            Some((c, rest)) => n
                .split_first()
                .is_some_and(|(d, tail)| c == d && go(rest, tail)),
        }
    }
    let p: Vec<char> = pattern.chars().collect();
    let n: Vec<char> = name.chars().collect();
    go(&p, &n)
}

fn is_atom_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_' || c == '*' || c == '?'
}

fn is_atom_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '*' | '?' | '~' | '-')
}

/// One token of a varlist (no spaces in it): `x*`, `i.g*`, `(x1-x3`,
/// `c.x#c.w`.
fn token(tok: &str, vars: &[String]) -> Result<String, String> {
    // parentheses and = around the variables stay as they are
    let lead = tok.len() - tok.trim_start_matches(['(', '=']).len();
    let trail = tok.len() - tok.trim_end_matches([')', '=']).len();
    let core = tok.get(lead..tok.len() - trail).unwrap_or("");
    if core.is_empty() {
        return Ok(tok.to_string());
    }
    let parts: Vec<&str> = core.split('#').collect();
    let mut out = Vec::new();
    for part in &parts {
        // an operator before the variable: i., c., ib2., L., i(1 2).
        let (op, atom) = match part.rfind('.') {
            Some(i) => (
                part.get(..=i).unwrap_or(""),
                part.get(i + 1..).unwrap_or(""),
            ),
            None => ("", *part),
        };
        if atom.is_empty()
            || !atom.chars().next().is_some_and(is_atom_start)
            || !atom.chars().all(is_atom_char)
        {
            out.push(vec![part.to_string()]);
            continue;
        }
        let names = resolve(atom, vars)?;
        out.push(names.iter().map(|n| format!("{op}{n}")).collect());
    }
    if parts.len() > 1 && out.iter().any(|v| v.len() != 1) {
        return Err(format!(
            "{core}: a pattern or range inside an interaction is not translated yet"
        ));
    }
    let joined = if parts.len() > 1 {
        out.iter().map(|v| v.join("")).collect::<Vec<_>>().join("#")
    } else {
        out.into_iter().flatten().collect::<Vec<_>>().join(" ")
    };
    Ok(format!(
        "{}{joined}{}",
        tok.get(..lead).unwrap_or(""),
        tok.get(tok.len() - trail..).unwrap_or("")
    ))
}

/// A varlist with every variable in it as the data names it.
pub fn expand(varlist: &str, vars: &[String]) -> Result<String, String> {
    let mut out = Vec::new();
    for tok in varlist.split_whitespace() {
        out.push(token(tok, vars)?);
    }
    Ok(out.join(" "))
}

/// The names a varlist writes (without operators, factor prefixes,
/// parentheses or patterns): each may be an abbreviation, which only the
/// data's names settle.
pub fn varlist_words(varlist: &str) -> Vec<String> {
    let mut out = Vec::new();
    for tok in varlist.split_whitespace() {
        let tok = tok.trim_matches(['(', ')', '=']);
        for part in tok.split('#') {
            let atom = part.rsplit('.').next().unwrap_or(part);
            if atom.chars().next().is_some_and(is_atom_start)
                && atom.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
            {
                out.push(atom.to_string());
            }
        }
    }
    out
}
/// Whether a varlist has anything only the data's names settle: a
/// pattern, a range or `_all` (an abbreviation cannot be told from a name
/// without them, and a name not in the data makes R say so).
pub fn needs_names(varlist: &str) -> bool {
    varlist.split_whitespace().any(|t| {
        let t = t.trim_matches(['(', ')', '=']);
        t == "_all"
            || t == "*"
            || t.split('#').any(|p| {
                let atom = p.rsplit('.').next().unwrap_or(p);
                atom.contains(['*', '?', '~'])
                    || (atom.contains('-')
                        && atom
                            .chars()
                            .next()
                            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_'))
            })
    })
}

/// The names in an expression that may be a variable or an abbreviation
/// of one: not in a string, not a function (a name before `(`), not a
/// value label (after `:`), not inside `[...]`, not a number or `.a`, and
/// not a system name (`_n`, `_b`, ... : any name beginning `_`).
fn expression_names(exp: &str) -> Vec<(usize, usize)> {
    let b = exp.as_bytes();
    let mut found = Vec::new();
    let mut i = 0;
    let next_non_space = |from: usize| {
        b.get(from..).and_then(|s| {
            s.iter()
                .position(|c| !c.is_ascii_whitespace())
                .map(|k| from + k)
        })
    };
    while let Some(&c) = b.get(i) {
        match c {
            b'"' => {
                // a string, to its closing quote
                i += 1;
                while b.get(i).is_some_and(|&c| c != b'"') {
                    i += 1;
                }
                i += 1;
            }
            b'`' if b.get(i + 1) == Some(&b'"') => {
                // a compound string, to "'
                i += 2;
                while i < b.len() && !(b.get(i) == Some(&b'"') && b.get(i + 1) == Some(&b'\'')) {
                    i += 1;
                }
                i += 2;
            }
            b'[' => {
                let mut depth = 0;
                while let Some(&c) = b.get(i) {
                    if c == b'[' {
                        depth += 1;
                    } else if c == b']' {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    i += 1;
                }
                i += 1;
            }
            b'0'..=b'9' => {
                // a number: digits, a point, an exponent
                while b.get(i).is_some_and(|c| c.is_ascii_digit() || *c == b'.') {
                    i += 1;
                }
                if b.get(i).is_some_and(|c| *c == b'e' || *c == b'E')
                    && b.get(i + 1)
                        .is_some_and(|c| c.is_ascii_digit() || *c == b'-' || *c == b'+')
                {
                    i += 2;
                    while b.get(i).is_some_and(u8::is_ascii_digit) {
                        i += 1;
                    }
                }
                // a date literal's month (01jan2020) or the like: not a name
                while b
                    .get(i)
                    .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_')
                {
                    i += 1;
                }
            }
            b'.' => {
                // a decimal (.5) or an extended missing value (.a); after an
                // operator (l.x) what follows is read as usual
                i += 1;
                if b.get(i).is_some_and(u8::is_ascii_digit) {
                    while b.get(i).is_some_and(u8::is_ascii_digit) {
                        i += 1;
                    }
                } else if b.get(i).is_some_and(u8::is_ascii_lowercase)
                    && !b
                        .get(i + 1)
                        .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_' || *c == b'.')
                {
                    i += 1;
                }
            }
            c if c.is_ascii_alphabetic() || c == b'_' => {
                let start = i;
                while b
                    .get(i)
                    .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_')
                {
                    i += 1;
                }
                let after = next_non_space(i).and_then(|k| b.get(k).copied());
                let before = exp.get(..start).and_then(|s| s.trim_end().chars().last());
                // a system name, a value label (after :), or an operator: l.x, d.x, i.g
                if c == b'_' || before == Some(':') || b.get(i) == Some(&b'.') {
                    continue;
                }
                match after {
                    // a function: what it is given stays as written only for
                    // the date ones (td(01jan2020)), others hold expressions
                    Some(b'(') => {
                        let name = exp.get(start..i).unwrap_or("");
                        if matches!(
                            name,
                            "td" | "tc"
                                | "tC"
                                | "tm"
                                | "tq"
                                | "tw"
                                | "th"
                                | "ty"
                                | "c"
                                | "e"
                                | "r"
                                | "s"
                        ) {
                            let mut depth = 0;
                            while let Some(&c) = b.get(i) {
                                if c == b'(' {
                                    depth += 1;
                                } else if c == b')' {
                                    depth -= 1;
                                    if depth == 0 {
                                        break;
                                    }
                                }
                                i += 1;
                            }
                        }
                    }
                    // a matrix's element, m[1,2]: not a variable
                    Some(b'[')
                        if exp
                            .get(i..)
                            .and_then(|s| s.find(']').and_then(|e| s.get(..e)))
                            .is_some_and(|s| s.contains(',')) => {}
                    _ => found.push((start, i)),
                }
            }
            _ => i += 1,
        }
    }
    found
}

/// The names in an expression that may be a variable or an abbreviation of
/// one (only the data's names settle which).
pub fn expression_words(exp: &str) -> Vec<&str> {
    expression_names(exp)
        .into_iter()
        .filter_map(|(a, b)| exp.get(a..b))
        .collect()
}

/// An expression with each abbreviation of a variable made the variable's
/// name, as Stata 19 reads it (a variable comes before a scalar of the same
/// name); a name no variable begins with stays as written.
pub fn expand_expression(exp: &str, vars: &[String]) -> Result<String, String> {
    let mut out = String::new();
    let mut last = 0;
    for (a, b) in expression_names(exp) {
        let word = exp.get(a..b).unwrap_or("");
        out.push_str(exp.get(last..a).unwrap_or(""));
        match unique(word, vars)? {
            Some(v) => out.push_str(&v),
            None => out.push_str(word),
        }
        last = b;
    }
    out.push_str(exp.get(last..).unwrap_or(""));
    Ok(out)
}

/// `isvar` (SSC), as observed in Stata 19: each word that names variables
/// as a varlist would (an abbreviation, a pattern, a range) gives them, in
/// order and repeats kept; any other word (an operator, an ambiguous or
/// unknown name, `_n`, a number) goes to the second list as written.
pub fn isvar_split(words: &str, vars: &[String]) -> (Vec<String>, Vec<String>) {
    let mut good = Vec::new();
    let mut bad = Vec::new();
    for w in words.split_whitespace() {
        let plain = w.chars().next().is_some_and(is_atom_start) && w.chars().all(is_atom_char);
        match resolve(w, vars) {
            Ok(names) if plain && !names.is_empty() && names.iter().all(|n| vars.contains(n)) => {
                good.extend(names)
            }
            _ => bad.push(w.to_string()),
        }
    }
    (good, bad)
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abbreviations_in_expressions_are_the_variables() {
        let vars: Vec<String> = ["country_v13", "group_c0110", "x1", "x2", "s1", "_merge"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let e = |s: &str| expand_expression(s, &vars);
        assert_eq!(
            e("country==1 & group==0").unwrap(),
            "country_v13==1 & group_c0110==0"
        );
        // functions, strings, value labels, numbers, system names and matrices stay
        assert_eq!(
            e("missing(coun) & s == \"coun\" & coun == \"a\":coun & 1e-5 < .a & _n > 2 & M[1,2] > td(01jan2020)").unwrap(),
            "missing(country_v13) & s1 == \"coun\" & country_v13 == \"a\":coun & 1e-5 < .a & _n > 2 & M[1,2] > td(01jan2020)"
        );
        assert_eq!(e("coun[_n-1] + 2").unwrap(), "country_v13[_n-1] + 2");
        assert_eq!(e("l.d.coun + L2.x1").unwrap(), "l.d.country_v13 + L2.x1");
        assert_eq!(e("x > 1").unwrap_err(), "x ambiguous abbreviation");
        assert_eq!(e("newname + 1").unwrap(), "newname + 1");
    }

    fn data() -> Vec<String> {
        ["id", "x1", "x2", "x3", "price", "s", "g", "f", "d"]
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    /// What Stata 19 made of these with the same variables.
    #[test]
    fn varlists_are_statas() {
        let v = data();
        let e = |t: &str| expand(t, &v).unwrap_or_else(|e| format!("error: {e}"));
        assert_eq!(e("x*"), "x1 x2 x3");
        assert_eq!(e("x1-x3 price"), "x1 x2 x3 price");
        assert_eq!(e("pri"), "price");
        assert_eq!(e("x?"), "x1 x2 x3");
        assert_eq!(e("_all"), "id x1 x2 x3 price s g f d");
        assert_eq!(e("x"), "error: x ambiguous abbreviation");
        assert_eq!(e("zz*"), "error: variable zz* not found");
        assert_eq!(e("x~"), "error: x~ ambiguous abbreviation");
        assert_eq!(e("p~"), "price");
        // operators and interactions kept, names made whole
        assert_eq!(e("i.g c.pri#c.x1"), "i.g c.price#c.x1");
        assert_eq!(e("i.x*"), "i.x1 i.x2 i.x3");
        assert_eq!(e("(x1 = x2-x3)"), "(x1 = x2 x3)");
        assert_eq!(e("ib2.g newvar"), "ib2.g newvar");
        assert!(e("c.x*#i.g").starts_with("error: c.x*#i.g: a pattern"));
        assert!(needs_names("y x*") && needs_names("x1-x3") && needs_names("_all"));
        assert!(needs_names("(x = z1-z3)") && needs_names("i.g*"));
        assert!(!needs_names("y x1 x2") && !needs_names("i.g c.x#c.w"));
    }

    #[test]
    fn isvar_splits_names_as_stata_19_does() {
        let vars: Vec<String> = ["alpha", "beta", "gamma1", "gamma2", "name"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let (good, bad) = isvar_split(
            "beta zz alp gamma* gamma alpha-gamma1 name _n 1x Alpha i.alpha",
            &vars,
        );
        assert_eq!(
            good,
            ["beta", "alpha", "gamma1", "gamma2", "alpha", "beta", "gamma1", "name"]
        );
        assert_eq!(bad, ["zz", "gamma", "_n", "1x", "Alpha", "i.alpha"]);
    }
}
