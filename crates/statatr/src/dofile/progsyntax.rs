//! `syntax` in a program: the call's text (`0'`) read against the program's
//! description of it, setting the locals Stata sets (`varlist`, `if`, `in`,
//! one for each option), or the reason it cannot be done here.
//!
//! The description: a main part (`varlist`, `varname`, `newvarlist`,
//! `newvarname`, `namelist`, `name`, `anything`, with its own options in
//! parentheses), `[if]`, `[in]`, and options after a comma, each as Stata
//! writes them: capital letters the shortest abbreviation (`Detail`,
//! `noCONstant`), an argument's kind in parentheses (`BY(varlist)`,
//! `file(string)`, `KLOW(real 1)`, `AR(integer 1)`, `at(name)`), `*` the rest.
//! Square brackets make a part optional.

use crate::lang::syntax::{options, parts};

/// One option of the description.
struct OptSpec {
    /// The local it sets: the name in lower case, without a leading `no`.
    local: String,
    /// The name in lower case, `no` kept.
    full: String,
    /// The shortest abbreviation.
    min: usize,
    /// An argument's kind (`string`, `real`, `integer`, `varlist`, `name`, ...)
    /// and its default; none for a flag.
    arg: Option<(String, String)>,
}

/// The locals `syntax` sets for this call, or why not.
pub fn locals(spec: &str, call: &str) -> Result<Vec<(String, String)>, String> {
    let (main_spec, opt_spec) = split_comma(spec);
    let opts_required = !opt_spec.is_empty() && !main_spec.trim_end().ends_with('[');
    let main_spec = main_spec.trim().trim_end_matches('[').trim();
    let opt_spec = opt_spec.trim().trim_end_matches(']').trim();
    let given = parts(call);
    let mut out = Vec::new();

    // the main part and [if] [in]
    let words = spec_words(main_spec);
    let mut main_seen = false;
    let mut if_allowed = false;
    let mut in_allowed = false;
    for (word, optional) in &words {
        let base = word.split('(').next().unwrap_or("").to_ascii_lowercase();
        match base.as_str() {
            "varlist" | "varname" | "newvarlist" | "newvarname" | "namelist" | "name"
            | "anything" => {
                main_seen = true;
                let local = match base.as_str() {
                    "varlist" | "varname" | "newvarlist" | "newvarname" => "varlist",
                    "namelist" | "name" => "namelist",
                    _ => "anything",
                };
                let text = given.main.split_whitespace().collect::<Vec<_>>().join(" ");
                let none = word.contains("default=none");
                if text.is_empty() && !optional && !none {
                    return Err(format!("{local} required (Stata: r(100))"));
                }
                if local == "varlist" && text.chars().any(|ch| matches!(ch, '*' | '?' | '-' | '~'))
                {
                    return Err(
                        "a varlist with a pattern or a range, read by syntax, is not translated yet".into(),
                    );
                }
                if text.is_empty() && !none && local == "varlist" {
                    return Err("syntax with an optional varlist that is not given (all the data's variables in Stata) is not translated yet".into());
                }
                out.push((local.to_string(), text));
            }
            "if" => if_allowed = true,
            "in" => in_allowed = true,
            "" => {}
            other => {
                return Err(format!("syntax with {other} is not translated yet"));
            }
        }
    }
    if !main_seen && !given.main.trim().is_empty() {
        return Err("varlist not allowed (Stata: r(101))".into());
    }
    match (&given.if_exp, if_allowed) {
        (Some(e), true) => out.push(("if".into(), format!("if {}", e.trim()))),
        (Some(_), false) => return Err("if not allowed (Stata: r(101))".into()),
        (None, _) => out.push(("if".into(), String::new())),
    }
    match (&given.in_range, in_allowed) {
        (Some(r), true) => out.push(("in".into(), format!("in {}", r.trim()))),
        (Some(_), false) => return Err("in not allowed (Stata: r(101))".into()),
        (None, _) => out.push(("in".into(), String::new())),
    }
    if given.weight.is_some() {
        return Err("weights read by syntax are not translated yet".into());
    }

    // the options
    let specs = option_specs(opt_spec)?;
    let star = specs.iter().any(|s| s.full == "*");
    let call_opts = options(&given.options);
    if opts_required && call_opts.is_empty() {
        return Err("options required (Stata: r(198))".into());
    }
    let mut set: Vec<(String, String)> = specs
        .iter()
        .filter(|s| s.full != "*")
        .map(|s| {
            let default = match &s.arg {
                Some((kind, d)) if matches!(kind.as_str(), "real" | "integer" | "int") => d.clone(),
                _ => String::new(),
            };
            (s.local.clone(), default)
        })
        .collect();
    let mut rest = Vec::new();
    for o in &call_opts {
        let found = specs.iter().find(|s| s.full != "*" && o.is(&s.full, s.min));
        match found {
            Some(s) => {
                let value = match &s.arg {
                    None => s.full.clone(),
                    Some((kind, _)) => {
                        let a = o.arg();
                        if matches!(kind.as_str(), "real" | "integer" | "int")
                            && a.trim().parse::<f64>().is_err()
                        {
                            return Err(format!(
                                "option {}() needs a number (Stata: r(198))",
                                s.full
                            ));
                        }
                        a.trim().to_string()
                    }
                };
                if let Some(slot) = set.iter_mut().find(|(n, _)| n == &s.local) {
                    slot.1 = value;
                }
            }
            None if star => rest.push(format!(
                "{}{}",
                o.name,
                o.raw.as_ref().map(|r| format!("({r})")).unwrap_or_default()
            )),
            None => return Err(format!("option {} not allowed (Stata: r(198))", o.name)),
        }
    }
    if star {
        set.push(("options".into(), rest.join(" ")));
    }
    out.extend(set);
    Ok(out)
}

/// The description's options.
fn option_specs(text: &str) -> Result<Vec<OptSpec>, String> {
    let mut out = Vec::new();
    for (word, _) in spec_words(text) {
        if word == "*" {
            out.push(OptSpec {
                local: "options".into(),
                full: "*".into(),
                min: 1,
                arg: None,
            });
            continue;
        }
        let (name, arg) = match word.split_once('(') {
            Some((n, a)) => (n, Some(a.trim_end_matches(')').trim().to_string())),
            None => (word.as_str(), None),
        };
        if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(format!(
                "syntax with the option {word} is not translated yet"
            ));
        }
        let caps = name.chars().take_while(|c| c.is_ascii_uppercase()).count();
        // `noCONstant`: the abbreviation counts the no and the capitals
        let lower = name.to_ascii_lowercase();
        let (local, min) = if let Some(after) = name
            .strip_prefix("no")
            .filter(|a| a.starts_with(|c: char| c.is_ascii_uppercase()))
        {
            let caps2 = after.chars().take_while(|c| c.is_ascii_uppercase()).count();
            (after.to_ascii_lowercase(), 2 + caps2)
        } else {
            (lower.clone(), if caps > 0 { caps } else { name.len() })
        };
        let arg = match arg {
            None => None,
            Some(a) => {
                let mut w = a.split_whitespace();
                let kind = w.next().unwrap_or("").to_ascii_lowercase();
                let default = w.collect::<Vec<_>>().join(" ");
                if !matches!(
                    kind.as_str(),
                    "string"
                        | "real"
                        | "integer"
                        | "int"
                        | "varlist"
                        | "varname"
                        | "name"
                        | "namelist"
                        | "numlist"
                        | "passthru"
                ) {
                    return Err(format!(
                        "syntax with the option {word} is not translated yet"
                    ));
                }
                if kind == "passthru" {
                    return Err(format!(
                        "syntax with the option {word} is not translated yet"
                    ));
                }
                Some((kind, default))
            }
        };
        out.push(OptSpec {
            local,
            full: lower,
            min,
            arg,
        });
    }
    Ok(out)
}

/// The words of a description, each with whether it is in square brackets;
/// parentheses keep a word whole.
fn spec_words(text: &str) -> Vec<(String, bool)> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let (mut paren, mut bracket) = (0usize, 0usize);
    let flush = |cur: &mut String, bracket: usize, out: &mut Vec<(String, bool)>| {
        let w = cur.trim().to_string();
        if !w.is_empty() {
            out.push((w, bracket > 0));
        }
        cur.clear();
    };
    for ch in text.chars() {
        match ch {
            '(' => {
                paren += 1;
                cur.push(ch);
            }
            ')' => {
                paren = paren.saturating_sub(1);
                cur.push(ch);
            }
            '[' if paren == 0 => {
                flush(&mut cur, bracket, &mut out);
                bracket += 1;
            }
            ']' if paren == 0 => {
                flush(&mut cur, bracket, &mut out);
                bracket = bracket.saturating_sub(1);
            }
            ',' if paren == 0 => flush(&mut cur, bracket, &mut out),
            c if c.is_whitespace() && paren == 0 => flush(&mut cur, bracket, &mut out),
            c => cur.push(c),
        }
    }
    flush(&mut cur, bracket, &mut out);
    out
}

/// The description before its options' comma (outside parentheses), and after.
fn split_comma(text: &str) -> (&str, &str) {
    let mut paren = 0usize;
    for (i, ch) in text.char_indices() {
        match ch {
            '(' => paren += 1,
            ')' => paren = paren.saturating_sub(1),
            ',' if paren == 0 => {
                return (text.get(..i).unwrap_or(""), text.get(i + 1..).unwrap_or(""))
            }
            _ => {}
        }
    }
    (text, "")
}

#[cfg(test)]
mod tests {
    use super::locals;

    fn get(v: &[(String, String)], k: &str) -> String {
        v.iter()
            .find(|(n, _)| n == k)
            .map(|(_, x)| x.clone())
            .unwrap_or_else(|| "<none>".into())
    }

    #[test]
    fn varlist_if_in_and_options() {
        let v = locals(
            "varlist(numeric) [if] [in] [, Detail noCONstant BY(varlist) KLOW(real 1) *]",
            "x1 x2 if x3 > .5, det nocons by(g) other(3)",
        );
        let v = v.unwrap_or_default();
        assert_eq!(get(&v, "varlist"), "x1 x2");
        assert_eq!(get(&v, "if"), "if x3 > .5");
        assert_eq!(get(&v, "in"), "");
        assert_eq!(get(&v, "detail"), "detail");
        assert_eq!(get(&v, "constant"), "noconstant");
        assert_eq!(get(&v, "by"), "g");
        assert_eq!(get(&v, "klow"), "1");
        assert_eq!(get(&v, "options"), "other(3)");
    }

    #[test]
    fn what_stata_refuses_or_is_not_translated() {
        assert!(locals("varlist", "").is_err());
        assert!(locals("varlist [, Detail]", "x, foo").is_err());
        assert!(locals("varlist", "x if y > 1").is_err());
        assert!(locals("varlist", "x*").is_err());
        assert!(locals("anything", "3.5").is_ok_and(|v| get(&v, "anything") == "3.5"));
        assert!(
            locals(", file1(string) file2(string)", ", file1(a.dta) file2(b)").is_ok_and(|v| get(
                &v, "file1"
            )
                == "a.dta"
                && get(&v, "file2") == "b")
        );
    }
}
