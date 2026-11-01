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

#[cfg(test)]
mod tests {
    use super::*;

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
}
