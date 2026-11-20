//! `xi`, the prefix that expands `i.` terms into dummy variables, as Stata 19 does it
//! (`plan/stata/xi.md`): `xi: regress y i.g x`, `xi i.g`, `xi, prefix(_D): ...`.
//!
//! - `i.g` makes a byte variable `_Ig_#` for each level of g but the omitted one (the lowest;
//!   the most frequent with `char _dta[omit] prevalent`; `#` with `char g[omit] #`), labelled
//!   `g==#`; `i.g*i.h` adds both sets and `_IgXh_#_#` (`g==2 & h==1`); `i.g*x` adds the dummies and
//!   the float `_IgXx_#` (`(g==2)*x`); `i.g|x` makes the same and leaves the dummies themselves out
//!   of the command.
//! - Each run first drops the variables earlier runs made with the same prefix (those with another prefix stay). Before the
//!   command it prints a line per term: the term in 18 columns, the names in 20, then
//!   `(naturally coded; _Ig_1 omitted)` or `(coded as above)`.
//! - The levels, which only R knows, are asked for when the do-file runs (the do-file runner
//!   passes them to [`Session::xi`]); a dry run leaves the line as known only when run.

use super::meta::CHAR_HELPER;
use super::*;

/// A factor variable's levels as Stata's xi reads them (probes 2026-11-20): the sorted values
/// (a string variable's non-empty ones, in byte order), their labels as `g==label` writes them
/// (a number with up to 16 digits and no leading zero), and the names' tokens: the values
/// where all are whole numbers from 0 up ("naturally coded"), else 1, 2, 3 ... by rank.
macro_rules! xi_levels {
    () => {
        r##"stata_xi_levels <- function(d, v) {
  x <- d[[v]]
  if (is.null(x)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
  if (is.character(x)) {
    lv <- sort(unique(x[!is.na(x) & x != ""]), method = "radix")
    lab <- lv
    natural <- FALSE
  } else {
    xn <- unclass(x)
    attributes(xn) <- NULL
    xn <- as.numeric(xn)
    lv <- sort(unique(xn[!is.na(xn)]))
    lab <- vapply(lv, function(z) sub("^(-?)0[.]", "\\1.", format(z, digits = 16, scientific = FALSE, trim = TRUE)), "")
    natural <- length(lv) > 0 && all(lv == floor(lv) & lv >= 0)
  }
  list(values = lv, labels = lab, tokens = if (natural) lab else as.character(seq_along(lv)), natural = natural)
}
"##
    };
}

/// The levels of each factor variable and the level xi leaves out, one line per variable:
/// `v`, the name tokens, the labels, the omitted level's place and whether the coding is natural,
/// separated by tabs (the tokens and labels by the unit separator).
const XI_PLAN: &str = concat!(
    xi_levels!(),
    r##"stata_xi_plan <- function(d, vars) {
  for (v in vars) {
    lv <- stata_xi_levels(d, v)
    if (length(lv$values) < 2) stop("RCraft: xi of a variable with fewer than two values is not translated yet", call. = FALSE)
    om <- stata_char_get(d, v, "omit")
    if (nzchar(om)) {
      idx <- if (is.character(lv$values)) match(om, lv$values) else match(suppressWarnings(as.numeric(om)), lv$values)
      if (is.na(idx)) stop("RCraft: char ", v, "[omit] names a level the variable does not have, which is not translated yet", call. = FALSE)
    } else if (identical(stata_char_get(d, "_dta", "omit"), "prevalent")) {
      x <- d[[v]]
      if (!is.character(x)) { x <- unclass(x); attributes(x) <- NULL }
      tb <- tabulate(match(x, lv$values), length(lv$values))
      idx <- which(tb == max(tb))
      if (length(idx) > 1) stop("RCraft: xi, prevalent with two equally frequent levels is not translated yet", call. = FALSE)
    } else {
      idx <- 1L
    }
    cat(v, "\t", paste(lv$tokens, collapse = "\x1f"), "\t", paste(lv$labels, collapse = "\x1f"), "\t", idx, "\t", as.integer(lv$natural), "\n", sep = "")
  }
}"##
);

/// Makes the variables, prints the term lines, and keeps the names made.
pub(super) const XI_HELPER: &str = concat!(
    xi_levels!(),
    r##"stata_xi_make <- function(d, prefix, headers, vars, show = TRUE) {
  num <- function(u) { u <- unclass(u); attributes(u) <- NULL; as.numeric(u) }
  val <- function(u) if (is.character(u)) u else num(u)
  gone <- function(u) if (is.character(u)) is.na(u) | u == "" else is.na(u)
  made <- get0("stata_xi_made", ifnotfound = character())
  for (n in intersect(made, names(d))) if (startsWith(n, prefix)) d[[n]] <- NULL
  made <- made[made %in% names(d)]
  if (show) for (h in headers) cat(sprintf("%-18s%-20s%s\n", h[[1]], h[[2]], h[[3]]))
  for (s in vars) {
    if (s$name %in% names(d)) stop("variable ", s$name, " already defined (Stata: r(110))", call. = FALSE)
    la <- stata_xi_levels(d, s$a)
    xa <- val(d[[s$a]])
    va <- la$values[s$ia]
    if (s$kind == "dummy") {
      col <- as.numeric(xa == va)
      lab <- paste0(s$a, "==", la$labels[s$ia])
      fmt <- "%8.0g"
    } else if (s$kind == "ff") {
      lb <- stata_xi_levels(d, s$b)
      xb <- val(d[[s$b]])
      col <- ifelse(gone(xb), NA_real_, as.numeric(xa == va & xb == lb$values[s$ib]))
      lab <- paste0(s$a, "==", la$labels[s$ia], " & ", s$b, "==", lb$labels[s$ib])
      fmt <- "%8.0g"
    } else {
      xx <- num(d[[s$b]])
      col <- as.numeric(xa == va) * xx
      lab <- paste0("(", s$a, "==", la$labels[s$ia], ")*", s$b)
      fmt <- s$fmt
    }
    col[gone(xa)] <- NA_real_
    attr(col, "label") <- lab
    attr(col, "format.stata") <- fmt
    d[[s$name]] <- col
    made <- c(made, s$name)
  }
  list(d = d, made = made)
}"##
);

/// One `i.` term of a command, as typed.
#[derive(Debug, Clone, PartialEq)]
enum Term {
    /// `i.g`
    Factor(String),
    /// `i.g*i.h`
    Cross(String, String),
    /// `i.g*x` (`star`) or `i.g|x`
    WithVar { f: String, x: String, star: bool },
}

impl Term {
    fn factors(&self) -> Vec<&str> {
        match self {
            Term::Factor(f) => vec![f],
            Term::Cross(a, b) => vec![a, b],
            Term::WithVar { f, .. } => vec![f],
        }
    }
}

/// `i.g` (or `I.g`): the variable.
fn factor_of(s: &str) -> Option<&str> {
    let v = s.strip_prefix("i.").or_else(|| s.strip_prefix("I."))?;
    is_name(v).then_some(v)
}

/// A word that is an xi term, or None when it is an ordinary word. An error for a form
/// that is not translated.
fn term_of(w: &str) -> Result<Option<Term>, Untranslated> {
    if !(w.starts_with("i.") || w.starts_with("I.")) {
        return Ok(None);
    }
    let op = w.find(['*', '|']);
    let Some(k) = op else {
        return Ok(factor_of(w).map(|v| Term::Factor(v.to_string())));
    };
    let (left, right) = (w.get(..k).unwrap_or(""), w.get(k + 1..).unwrap_or(""));
    let star = w.get(k..k + 1) == Some("*");
    let Some(f) = factor_of(left) else {
        return Ok(None);
    };
    if right.contains(['*', '|']) {
        return Err(no(format!(
            "{w}: xi with more than two parts in a term is not translated yet"
        )));
    }
    if let Some(g) = factor_of(right) {
        if !star {
            return Err(no(format!(
                "{w}: xi's | between two factors is not translated yet"
            )));
        }
        return Ok(Some(Term::Cross(f.to_string(), g.to_string())));
    }
    if is_name(right) {
        return Ok(Some(Term::WithVar {
            f: f.to_string(),
            x: right.to_string(),
            star,
        }));
    }
    Err(no(format!(
        "{w}: xi with this kind of term is not translated yet"
    )))
}

/// What the line says: its prefix option, the text before the colon's options, the words with
/// their terms, and the command after the colon, if any.
struct Parsed {
    prefix: String,
    noomit: bool,
    words: Vec<String>,
    inner: Option<String>,
}

fn parse(rest: &str) -> Result<Parsed, Untranslated> {
    let rest = rest.trim();
    // the colon that ends the options, outside parentheses
    let mut depth = 0i32;
    let mut colon = None;
    for (i, ch) in rest.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth -= 1,
            ':' if depth == 0 => {
                colon = Some(i);
                break;
            }
            _ => {}
        }
    }
    let (head, inner) = match colon {
        Some(i) => (
            rest.get(..i).unwrap_or(""),
            Some(rest.get(i + 1..).unwrap_or("").trim().to_string()),
        ),
        None => (rest, None),
    };
    let (main, opts) = match head.split_once(',') {
        Some((m, o)) => (m.trim(), o.trim()),
        None => (head.trim(), ""),
    };
    // `xi, noomit i.g`: the terms may follow the options when there is no colon
    let mut tail_terms: Vec<String> = Vec::new();
    let opts = if inner.is_none() {
        let mut keep: Vec<String> = Vec::new();
        for w in words_of(opts) {
            if w.starts_with("i.") || w.starts_with("I.") {
                tail_terms.push(w);
            } else {
                keep.push(w);
            }
        }
        keep.join(" ")
    } else {
        opts.to_string()
    };
    let mut prefix = "_I".to_string();
    let mut noomit = false;
    for o in crate::lang::syntax::options(&opts) {
        if o.is("noomit", 3) {
            noomit = true;
        } else if o.is("prefix", 3) {
            let p = o.arg();
            let p = p.trim();
            if p.is_empty() || !is_name(p) {
                return Err(no("xi, prefix() needs a name"));
            }
            prefix = p.to_string();
        } else {
            return Err(no(format!(
                "xi with the option {} is not translated yet",
                o.name
            )));
        }
    }
    let words = if inner.is_some() {
        if !main.is_empty() {
            return Err(no("xi: terms before the colon are not translated"));
        }
        Vec::new()
    } else {
        main.split_whitespace()
            .map(str::to_string)
            .chain(tail_terms)
            .collect()
    };
    Ok(Parsed {
        prefix,
        noomit,
        words,
        inner,
    })
}

/// The words of the command that come before its options (outside parentheses).
fn words_of(inner: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut depth = 0i32;
    for ch in inner.chars() {
        match ch {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => break,
            _ => {}
        }
        if ch.is_whitespace() && depth == 0 {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
        } else {
            cur.push(ch);
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// The levels of a factor and the one left out, from R's plan line.
#[derive(Debug, Clone)]
struct Plan {
    /// the name tokens (the values where naturally coded, else 1, 2, 3 ...)
    levels: Vec<String>,
    /// each level as `g==label` writes it
    labels: Vec<String>,
    /// the place (1-based) of the level left out
    omit: usize,
    natural: bool,
}

fn read_plan(text: &str) -> BTreeMap<String, Plan> {
    let mut m = BTreeMap::new();
    for line in text.lines() {
        let mut p = line.split('\t');
        if let (Some(v), Some(l), Some(lab), Some(o), Some(n)) =
            (p.next(), p.next(), p.next(), p.next(), p.next())
        {
            let Ok(omit) = o.trim().parse::<usize>() else {
                continue;
            };
            m.insert(
                v.trim().to_string(),
                Plan {
                    levels: l.split('\u{1f}').map(str::to_string).collect(),
                    labels: lab.split('\u{1f}').map(str::to_string).collect(),
                    omit,
                    natural: n.trim() == "1",
                },
            );
        }
    }
    m
}

impl Session {
    /// The R that asks for the levels of the factor variables a `xi` line names, or None when
    /// the line is not one this translates (the line itself then says why).
    pub(super) fn xi_question(&self, c: &Command) -> Option<String> {
        if c.name != "xi" {
            return None;
        }
        let d = self.data.clone()?;
        let p = parse(&super::returned::after_name(c)).ok()?;
        let words = match &p.inner {
            Some(i) => words_of(i),
            None => p.words.clone(),
        };
        let mut vars: Vec<String> = Vec::new();
        for w in &words {
            if let Some(t) = term_of(w).ok()? {
                for f in t.factors() {
                    if !vars.iter().any(|v| v == f) {
                        vars.push(f.to_string());
                    }
                }
            }
        }
        if vars.is_empty() {
            return None;
        }
        Some(format!(
            "cat(local({{\n{CHAR_HELPER}\n{XI_PLAN}\nstata_xi_plan({d}, c({}))\n}}))",
            vars.iter()
                .map(|v| r_text(v))
                .collect::<Vec<_>>()
                .join(", ")
        ))
    }

    /// `xi [, prefix(str)] : command` and `xi [, prefix(str)] terms`.
    pub(super) fn xi(&mut self, c: &Command, out: &mut Out) -> Result<(), Untranslated> {
        self.no_by(c, "xi")?;
        let p = parse(&super::returned::after_name(c))?;
        let words = match &p.inner {
            Some(i) => words_of(i),
            None => p.words.clone(),
        };
        let mut terms: Vec<(String, Term)> = Vec::new();
        for w in &words {
            if let Some(t) = term_of(w)? {
                terms.push((w.clone(), t));
            }
        }
        if terms.is_empty() {
            return Err(no("xi without an i. term is not translated yet"));
        }
        let mut seen: Vec<&str> = Vec::new();
        for (_, t) in &terms {
            for f in t.factors() {
                if seen.contains(&f) {
                    return Err(no(format!(
                        "xi with {f} in two terms is not translated yet"
                    )));
                }
                seen.push(f);
            }
        }
        let Some(plan_text) = self.xi_plan.take() else {
            return Err(no(format!(
                "xi needs the levels of its factor variables, {}",
                crate::dofile::KNOWN_WHEN_RUN
            )));
        };
        if let Some(why) = plan_text.strip_prefix('\u{1}') {
            return Err(no(format!("xi: {why}")));
        }
        let plan = read_plan(&plan_text);
        let d = self.data()?;
        let prefix = &p.prefix;
        let level_of = |v: &str| plan.get(v);
        // the names of a factor's dummies (the omitted level has none)
        // each as (name, place among the levels), the omitted one left out unless noomit
        let dummies = |v: &str| -> Result<Vec<(String, usize)>, Untranslated> {
            let pl =
                level_of(v).ok_or_else(|| no(format!("xi: the levels of {v} are not known")))?;
            Ok(pl
                .levels
                .iter()
                .enumerate()
                .filter(|(i, _)| p.noomit || i + 1 != pl.omit)
                .map(|(i, l)| (format!("{prefix}{v}_{l}"), i + 1))
                .collect())
        };
        let mut headers: Vec<String> = Vec::new();
        let mut specs: Vec<String> = Vec::new();
        let mut made: Vec<(String, Storage)> = Vec::new();
        let mut repl: Vec<(String, String)> = Vec::new();
        let mut wild: Vec<(String, String)> = Vec::new();
        // la and lb are places among the levels (1-based); fmt the format of an `i.g*x` column
        let spec = |kind: &str, name: &str, a: &str, ia: usize, b: &str, ib: usize, fmt: &str| {
            format!(
                "list(kind = {}, name = {}, a = {}, ia = {}L, b = {}, ib = {}L, fmt = {})",
                r_text(kind),
                r_text(name),
                r_text(a),
                ia,
                r_text(b),
                ib,
                r_text(fmt)
            )
        };
        let head = |term: &str, stub: &str, note: &str| {
            format!("c({}, {}, {})", r_text(term), r_text(stub), r_text(note))
        };
        let main_head = |v: &str| -> Result<String, Untranslated> {
            let pl =
                level_of(v).ok_or_else(|| no(format!("xi: the levels of {v} are not known")))?;
            let (Some(first), Some(last)) = (pl.levels.first(), pl.levels.last()) else {
                return Err(no("xi: a variable without levels"));
            };
            Ok(head(
                &format!("i.{v}"),
                &format!("{prefix}{v}_{first}-{last}"),
                &if pl.natural {
                    format!(
                        "(naturally coded; {prefix}{v}_{} omitted)",
                        pl.levels
                            .get(pl.omit.saturating_sub(1))
                            .map_or("", |s| s.as_str())
                    )
                } else {
                    format!(
                        "({prefix}{v}_{} for {v}=={} omitted)",
                        pl.levels
                            .get(pl.omit.saturating_sub(1))
                            .map_or("", |s| s.as_str()),
                        pl.labels
                            .get(pl.omit.saturating_sub(1))
                            .map_or("", |s| s.as_str())
                    )
                },
            ))
        };
        for (word, t) in &terms {
            let mut words_here: Vec<String> = Vec::new();
            let mut wild_here: Vec<String> = Vec::new();
            match t {
                Term::Factor(v) => {
                    wild_here.push(format!("{prefix}{v}_*"));
                    if !p.noomit {
                        headers.push(main_head(v)?);
                    }
                    for (n, i) in dummies(v)? {
                        specs.push(spec("dummy", &n, v, i, "", 0, ""));
                        made.push((n.clone(), Storage::Byte));
                        words_here.push(n);
                    }
                }
                Term::Cross(a, b) => {
                    wild_here.extend([
                        format!("{prefix}{a}_*"),
                        format!("{prefix}{b}_*"),
                        format!("{prefix}{a}X{b}_*"),
                    ]);
                    if !p.noomit {
                        headers.push(main_head(a)?);
                        headers.push(main_head(b)?);
                    }
                    headers.push(head(
                        word,
                        &format!("{prefix}{a}X{b}_#_#"),
                        "(coded as above)",
                    ));
                    for v in [a, b] {
                        for (n, i) in dummies(v)? {
                            specs.push(spec("dummy", &n, v, i, "", 0, ""));
                            made.push((n.clone(), Storage::Byte));
                            words_here.push(n);
                        }
                    }
                    for (_, ia) in dummies(a)? {
                        for (_, ib) in dummies(b)? {
                            let (Some(ta), Some(tb)) = (
                                level_of(a).and_then(|pl| pl.levels.get(ia - 1)),
                                level_of(b).and_then(|pl| pl.levels.get(ib - 1)),
                            ) else {
                                return Err(no("xi: a level is missing from the plan"));
                            };
                            let n = format!("{prefix}{a}X{b}_{ta}_{tb}");
                            specs.push(spec("ff", &n, a, ia, b, ib, ""));
                            made.push((n.clone(), Storage::Byte));
                            words_here.push(n);
                        }
                    }
                }
                Term::WithVar { f, x, star } => {
                    if *star {
                        wild_here.push(format!("{prefix}{f}_*"));
                    }
                    wild_here.extend([x.clone(), format!("{prefix}{f}X{x}_*")]);
                    if !p.noomit {
                        headers.push(main_head(f)?);
                    }
                    headers.push(head(
                        word,
                        &format!("{prefix}{f}X{x}_#"),
                        "(coded as above)",
                    ));
                    // the product has x's type: a double keeps %10.0g, a float %9.0g
                    let (xfmt, xstore) = match self.storage.get(x.as_str()) {
                        Some(Storage::Double) => ("%10.0g", Storage::Double),
                        _ => ("%9.0g", Storage::Float),
                    };
                    for (n, i) in dummies(f)? {
                        specs.push(spec("dummy", &n, f, i, "", 0, ""));
                        made.push((n.clone(), Storage::Byte));
                        if *star {
                            words_here.push(n);
                        }
                    }
                    words_here.push(x.clone());
                    for (_, i) in dummies(f)? {
                        let Some(tok) = level_of(f).and_then(|pl| pl.levels.get(i - 1)) else {
                            return Err(no("xi: a level is missing from the plan"));
                        };
                        let n = format!("{prefix}{f}X{x}_{tok}");
                        specs.push(spec("fx", &n, f, i, x, 0, xfmt));
                        made.push((n.clone(), xstore));
                        words_here.push(n);
                    }
                }
            }
            repl.push((word.clone(), words_here.join(" ")));

            wild.push((word.clone(), wild_here.join(" ")));
        }
        if let Some((n, _)) = made.iter().find(|(n, _)| n.chars().count() > 32) {
            return Err(no(format!("xi: the name {n} is longer than 32 characters")));
        }
        // the variables an earlier run made are gone from what Rust knows too
        for n in std::mem::take(&mut self.xi_made) {
            if n.starts_with(prefix.as_str()) {
                self.kinds.remove(&n);
                self.storage.remove(&n);
            } else {
                self.xi_made.push(n);
            }
        }
        self.helpers.insert("stata_xi");
        let show = !self.quiet(c);
        out.line(format!(
            "stata_xi_r <- stata_xi_make({d}, {}, list({}), list({}), show = {})",
            r_text(prefix),
            headers.join(", "),
            specs.join(", "),
            super::stats::r_logical(show)
        ));
        out.line(format!(
            "{d} <- stata_xi_r$d; stata_xi_made <- stata_xi_r$made; rm(stata_xi_r)"
        ));
        for (n, store) in &made {
            self.kinds.insert(n.clone(), Kind::Number);
            self.storage.insert(n.clone(), *store);
            self.xi_made.push(n.clone());
        }
        let Some(inner) = &p.inner else {
            return Ok(());
        };
        // the command with its terms made into the variables
        let cut = inner.find(',').unwrap_or(inner.len());
        let (before, after) = inner.split_at(cut);
        let new_before: Vec<String> = before
            .split_whitespace()
            .map(|w| {
                repl.iter()
                    .find(|(o, _)| o == w)
                    .map_or_else(|| w.to_string(), |(_, n)| n.clone())
            })
            .collect();
        let text = format!("{}{}", new_before.join(" "), after);
        // e(cmdline) keeps the stubs, `_Ig_*`, where the command runs on the names made
        let shown_before: Vec<String> = before
            .split_whitespace()
            .map(|w| {
                wild.iter()
                    .find(|(o, _)| o == w)
                    .map_or_else(|| w.to_string(), |(_, n)| n.clone())
            })
            .collect();
        let shown = format!("{}{}", shown_before.join(" "), after);
        let program = crate::reader::read(&text);
        let [crate::reader::Item::Command(inner_c)] = program.items.as_slice() else {
            return Err(no("xi: the command after the colon is not one this reads"));
        };
        let mut inner_c = inner_c.clone();
        inner_c.prefixes = c.prefixes.clone();
        inner_c.span = c.span;
        inner_c.text = shown;
        self.fit_robust = false;
        self.fit_cluster = false;
        self.dispatch(&inner_c, out)
    }
}
