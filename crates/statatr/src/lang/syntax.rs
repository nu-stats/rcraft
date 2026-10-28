//! Standard command syntax: `[varlist] [=exp] [if exp] [in range] [, options]`.
//!
//! Behaviour reference: `plan/stata/data-commands.md`. Only the text is taken
//! apart here; resolving a varlist or an `in` range against a dataset is the
//! engine's job.

/// A command line taken apart.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Parts {
    /// what comes before `if`/`in`/`,` (a varlist, `name = exp`, ...)
    pub main: String,
    pub if_exp: Option<String>,
    pub in_range: Option<String>,
    /// `[aweight = w]` and the like
    pub weight: Option<Weight>,
    pub options: String,
}

/// The kinds of weight Stata accepts (`[U] 11.1.6 weight`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeightKind {
    Frequency,
    Analytic,
    Importance,
    Sampling,
    /// `[weight = w]`: each command says what it means
    Unspecified,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Weight {
    pub kind: WeightKind,
    pub exp: String,
}

/// Takes a `[kind = exp]` clause out of `text`, where one starts the text
/// or follows a space (so a subscript like `x[1]` stays put).
pub fn take_weight(text: &str) -> (String, Option<Weight>) {
    let mut start = None;
    let mut quote = false;
    for (i, c) in text.char_indices() {
        match c {
            '"' => quote = !quote,
            '[' if !quote
                && text
                    .get(..i)
                    .is_some_and(|b| b.is_empty() || b.ends_with(char::is_whitespace)) =>
            {
                start = Some(i);
                break;
            }
            _ => {}
        }
    }
    let unchanged = || (text.to_string(), None);
    let Some(open) = start else {
        return unchanged();
    };
    let after = text.get(open + 1..).unwrap_or_default();
    let Some(len) = after.find(']') else {
        return unchanged();
    };
    let inside = after.get(..len).unwrap_or_default();
    // `[]`, as a program passes on `[`weight'`exp']` with no weight
    if inside.trim().is_empty() {
        let rest = format!(
            "{} {}",
            text.get(..open).unwrap_or_default().trim_end(),
            after.get(len + 1..).unwrap_or_default().trim_start()
        );
        return (rest.trim().to_string(), None);
    }
    let Some((word, exp)) = inside.split_once('=') else {
        return unchanged();
    };
    let kind = match word.trim() {
        "fweight" | "fw" | "frequency" | "fweigh" | "fweig" | "fwei" | "fwe" => {
            WeightKind::Frequency
        }
        "aweight" | "aw" | "cellsize" | "aweigh" | "aweig" | "awei" | "awe" => WeightKind::Analytic,
        "iweight" | "iw" | "iweigh" | "iweig" | "iwei" | "iwe" => WeightKind::Importance,
        "pweight" | "pw" | "pweigh" | "pweig" | "pwei" | "pwe" => WeightKind::Sampling,
        "weight" | "w" | "weigh" | "weig" | "wei" | "we" => WeightKind::Unspecified,
        _ => return unchanged(),
    };
    let rest = format!(
        "{} {}",
        text.get(..open).unwrap_or_default(),
        after.get(len + 1..).unwrap_or_default()
    );
    (
        rest.trim().to_string(),
        Some(Weight {
            kind,
            exp: exp.trim().to_string(),
        }),
    )
}

/// Splits at the top level (outside quotes, parentheses and brackets) on the
/// words `if` and `in` and the first comma.
pub fn parts(text: &str) -> Parts {
    let chars: Vec<char> = text.chars().collect();
    let mut depth = 0i32;
    let mut quote = false;
    let mut cuts: Vec<(usize, &'static str)> = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars.get(i).copied().unwrap_or(' ');
        match c {
            '"' => quote = !quote,
            '(' | '[' if !quote => depth += 1,
            ')' | ']' if !quote => depth -= 1,
            ',' if !quote && depth == 0 && !cuts.iter().any(|(_, k)| *k == ",") => {
                cuts.push((i, ","))
            }
            _ => {}
        }
        if !quote && depth == 0 && !cuts.iter().any(|(_, k)| *k == ",") {
            let before_ok = i == 0 || chars.get(i - 1).is_some_and(|c| c.is_whitespace());
            for word in ["if", "in"] {
                let w: Vec<char> = word.chars().collect();
                let matches = chars.get(i..i + 2) == Some(&w[..]);
                let after_ok = chars
                    .get(i + 2)
                    .is_none_or(|c| c.is_whitespace() || *c == '(');
                if before_ok && matches && after_ok && !cuts.iter().any(|(_, k)| *k == word) {
                    cuts.push((i, word));
                }
            }
        }
        i += 1;
    }
    cuts.sort_by_key(|(at, _)| *at);
    let mut p = Parts::default();
    let piece = |from: usize, to: usize| -> String {
        chars
            .get(from..to)
            .unwrap_or_default()
            .iter()
            .collect::<String>()
            .trim()
            .to_string()
    };
    let first = cuts.first().map_or(chars.len(), |(at, _)| *at);
    let (main, mut weight) = take_weight(&piece(0, first));
    p.main = main;
    for (k, (at, kind)) in cuts.iter().enumerate() {
        let end = cuts.get(k + 1).map_or(chars.len(), |(next, _)| *next);
        let skip = if *kind == "," { 1 } else { 2 };
        let mut body = piece(at + skip, end);
        if *kind != "," && weight.is_none() {
            let (rest, w) = take_weight(&body);
            body = rest;
            weight = w;
        }
        match *kind {
            "if" => p.if_exp = Some(body),
            "in" => p.in_range = Some(body),
            _ => p.options = body,
        }
    }
    p.weight = weight;
    p
}

/// `name` matches a varlist pattern: `*` and `~` any run of characters, `?`
/// one character.
pub fn wildcard(pattern: &str, name: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let s: Vec<char> = name.chars().collect();
    fn go(p: &[char], s: &[char]) -> bool {
        match p.split_first() {
            None => s.is_empty(),
            Some(('*' | '~', rest)) => {
                (0..=s.len()).any(|k| go(rest, s.get(k..).unwrap_or_default()))
            }
            Some(('?', rest)) => !s.is_empty() && go(rest, s.get(1..).unwrap_or_default()),
            Some((c, rest)) => s.first() == Some(c) && go(rest, s.get(1..).unwrap_or_default()),
        }
    }
    go(&p, &s)
}

/// `word` is `full` or an abbreviation of it at least `min` characters long.
pub fn abbrev(word: &str, full: &str, min: usize) -> bool {
    word.len() >= min && full.starts_with(word)
}

/// One option: its name and the text in its parentheses, if any (outer
/// quotes removed), and that text as typed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Opt {
    pub name: String,
    pub arg: Option<String>,
    pub raw: Option<String>,
}

impl Opt {
    pub fn is(&self, full: &str, min: usize) -> bool {
        abbrev(&self.name, full, min)
    }

    pub fn arg(&self) -> String {
        self.arg.clone().unwrap_or_default()
    }
}

/// Splits an option list: `clear sheet("Data") cellrange(A1:B3)`.
pub fn options(text: &str) -> Vec<Opt> {
    let mut out = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars.get(i).copied().unwrap_or(' ');
        if c.is_whitespace() || c == ',' {
            i += 1;
            continue;
        }
        let start = i;
        while chars
            .get(i)
            .is_some_and(|c| !c.is_whitespace() && *c != '(')
        {
            i += 1;
        }
        let name: String = chars.get(start..i).unwrap_or_default().iter().collect();
        let mut arg = None;
        let mut raw = None;
        // blanks may stand between a name and its parentheses
        // (`parse (" <")` in outreg2)
        let mut ahead = i;
        while chars.get(ahead).is_some_and(|c| *c == ' ' || *c == '\t') {
            ahead += 1;
        }
        if ahead > i && chars.get(ahead) == Some(&'(') {
            i = ahead;
        }
        if chars.get(i) == Some(&'(') {
            let mut depth = 0;
            let mut quote = false;
            let open = i;
            while let Some(&c) = chars.get(i) {
                match c {
                    '"' => quote = !quote,
                    '(' if !quote => depth += 1,
                    ')' if !quote => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
            let inner: String = chars.get(open + 1..i).unwrap_or_default().iter().collect();
            raw = Some(inner.clone());
            let inner = inner.trim();
            let unquoted = inner
                .strip_prefix('"')
                .and_then(|s| s.strip_suffix('"'))
                .filter(|s| !s.contains('"'))
                .unwrap_or(inner);
            arg = Some(unquoted.to_string());
            i += 1;
        }
        out.push(Opt { name, arg, raw });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_qualifiers() {
        let p = parts("y = 99 if _n <= 3 in 1/5, replace");
        assert_eq!(p.main, "y = 99");
        assert_eq!(p.if_exp.as_deref(), Some("_n <= 3"));
        assert_eq!(p.in_range.as_deref(), Some("1/5"));
        assert_eq!(p.options, "replace");
        let q = parts(r#"s = "improve" if strpos(s, "pub") > 0"#);
        assert_eq!(q.main, r#"s = "improve""#);
        assert_eq!(q.if_exp.as_deref(), Some(r#"strpos(s, "pub") > 0"#));
        assert_eq!(
            parts("x if inlist(1, 2)").if_exp.as_deref(),
            Some("inlist(1, 2)")
        );
        assert_eq!(parts("width").main, "width");
        assert!(wildcard("x*", "x12"));
        assert!(!wildcard("x?", "x12"));
    }

    #[test]
    fn weights() {
        let p = parts("x [aweight = w], detail");
        assert_eq!(p.main, "x");
        let w = p.weight.unwrap();
        assert_eq!((w.kind, w.exp.as_str()), (WeightKind::Analytic, "w"));
        let q = parts("x if g == 1 [fw=n] in 1/9");
        assert_eq!(q.if_exp.as_deref(), Some("g == 1"));
        assert_eq!(q.weight.unwrap().kind, WeightKind::Frequency);
        let r = parts("y = x[1] if z[2] > 0");
        assert_eq!(r.main, "y = x[1]");
        assert!(r.weight.is_none());
    }
}
