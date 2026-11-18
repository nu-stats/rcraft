//! `clonevar`, `mvdecode`, `mvencode`, `fillin`, `expandcl` and `joinby`, as
//! Stata 19 does them on made-up data (batch A1 of the StatCraft port;
//! `plan/stata/clonevar.md` and the notes beside it).

use super::*;

/// A copy of `x` that keeps its label, display format and value label; rows
/// outside `if`/`in` become missing (`.` or `""`).
pub(super) const CLONE_HELPER: &str = r#"stata_clone <- function(x, keep = NULL) {
  if (!is.null(keep)) {
    keep <- rep_len(keep, length(x)) & !is.na(keep)
    if (is.character(x)) x[!keep] <- "" else x[!keep] <- NA
  }
  x
}"#;

impl Session {
    /// `clonevar newvar = varname [if] [in]`: the variable's type, display
    /// format, variable label, value label and characteristics, and its
    /// values where the condition holds (missing elsewhere).
    ///
    /// Observed in Stata 19: a new name that is there is r(110) (checked
    /// first); more than one name after `=` is r(101) "varlist not allowed";
    /// no `=` is r(198) "syntax is clonevar newvar = varname ..."; an empty
    /// right side is r(102); weights are r(101); a name the data lack
    /// (also in `if`) is r(111). It says `(# missing values generated)` with
    /// the count of missing values in the new variable.
    pub(super) fn clonevar(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[]) {
            return Err(no(format!(
                "clonevar with the option {o} is not translated yet"
            )));
        }
        if c.prefixes.iter().any(|p| matches!(p, Prefix::By { .. })) {
            return Err(no("clonevar with by: is not translated yet"));
        }
        let stop = |out: &mut Out, msg: &str| {
            out.line(format!("stop({}, call. = FALSE)", r_text(msg)));
        };
        let Some((lhs, rhs)) = c.parts.main.split_once('=') else {
            stop(
                out,
                "syntax is clonevar newvar = varname ... (Stata: r(198))",
            );
            return Ok(());
        };
        let new: Vec<&str> = lhs.split_whitespace().collect();
        let old: Vec<&str> = rhs.split_whitespace().collect();
        let [new] = new.as_slice() else {
            stop(
                out,
                "syntax is clonevar newvar = varname ... (Stata: r(198))",
            );
            return Ok(());
        };
        if !is_name(new) {
            stop(out, &format!("{new} invalid name (Stata: r(198))"));
            return Ok(());
        }
        let d = self.data()?;
        let exists = format!(
            "if ({} %in% names({d})) stop({}, call. = FALSE)",
            r_text(new),
            r_text(&format!("variable {new} already defined (Stata: r(110))"))
        );
        let old = match old.as_slice() {
            [] => {
                stop(out, "too few variables specified (Stata: r(102))");
                return Ok(());
            }
            [one] if is_name(one) => one.to_string(),
            [_] => {
                return Err(no(
                    "clonevar of a name pattern or an odd name is not translated yet",
                ))
            }
            _ => {
                stop(out, "varlist not allowed (Stata: r(101))");
                return Ok(());
            }
        };
        if c.parts.weight.is_some() {
            stop(out, "weights not allowed (Stata: r(101))");
            return Ok(());
        }
        out.line(exists);
        out.line(format!(
            "if (!{} %in% names({d})) stop({}, call. = FALSE)",
            r_text(&old),
            r_text(&format!("variable {old} not found (Stata: r(111))"))
        ));
        let keep = self.condition(c, out)?;
        let keep = keep.unwrap_or_else(|| "NULL".into());
        self.helpers.insert("stata_clone");
        out.package("dplyr");
        out.line(format!(
            "{d} <- {d} |> dplyr::mutate({} = stata_clone({}, {keep}))",
            r_name(new),
            r_name(&old)
        ));
        if !self.quiet(c) {
            self.helpers.insert("stata_chg");
            self.helpers.insert("stata_mv");
            out.line(format!("stata_gen_note({d}[[{}]])", r_text(new)));
        }
        if let Some(k) = self.kinds.get(&old).copied() {
            self.kinds.insert(new.to_string(), k);
        }
        if let Some(s) = self.storage.get(&old).copied() {
            self.storage.insert(new.to_string(), s);
        }
        Ok(())
    }
}

/// `mvdecode` and `mvencode`, as Stata 19 prints them (`plan/stata/mvdecode.md`).
///
/// - a string variable: `           s: string variable ignored` (the name
///   right-aligned in 12, abbreviated with `~` past 12);
/// - mvdecode says `           b: 2 missing values generated` for the
///   observations it changed, nothing when none; the values are compared in
///   the variable's own storage (a float 1.1 is matched by 1.1);
/// - mvencode says `           b: 2 missing values recoded`, and
///   `              (recast to int)` when the value does not fit the type; if
///   any selected variable already holds the value it says `b: already 5 in
///   12 observations` for each such variable and `no action taken` (r(9))
///   and changes nothing, unless `override` is given. Counts have no commas.
///
/// `codes`: for mvdecode the missing value each clause makes ("" is `.`,
/// else the letter of `.a` to `.z`); for mvencode the kind each clause takes
/// ("*" every kind, "" is `.`, or a letter).
pub(super) const MVCODE_HELPER: &str = r#"stata_mvcode <- function(d, vars, kind, vals, codes, keep = NULL, st = character(0), override = FALSE, show = TRUE) {
  ab <- function(v) sprintf("%12s", if (nchar(v) <= 12) v else paste0(substr(v, 1, 10), "~", substr(v, nchar(v), nchar(v))))
  s1 <- function(n, one, many) if (n == 1) one else many
  cnt <- function(n) format(n, scientific = FALSE, trim = TRUE)
  lim <- list(byte = c(-127, 100), int = c(-32767, 32740), long = c(-2147483647, 2147483620))
  rank <- c(byte = 1, int = 2, long = 3, float = 4, double = 5)
  for (v in vars) if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
  fit <- function(val) {
    if (val == trunc(val)) for (t in names(lim)) if (val >= lim[[t]][1] && val <= lim[[t]][2]) return(t)
    "double"
  }
  plan <- list()
  clash <- FALSE
  for (v in vars) {
    x <- d[[v]]
    if (is.character(x)) {
      if (show) cat(ab(v), ": string variable ignored\n", sep = "")
      next
    }
    y <- as.vector(unclass(x))
    if (!is.double(y)) y <- as.numeric(y)
    sel <- if (is.null(keep)) rep(TRUE, length(y)) else keep
    tag <- unname(st[v])
    known <- !is.na(tag)
    fl <- known && tag == "float"
    if (kind == "decode") {
      done <- rep(FALSE, length(y))
      for (k in seq_along(vals)) {
        val <- vals[[k]]
        if (!known) val <- c(val, stata_float(val)) else if (fl) val <- stata_float(val)
        hit <- sel & !done & !is.na(y) & y %in% val
        if (any(hit)) y[hit] <- if (codes[k] == "") NA_real_ else haven::tagged_na(codes[k])
        done <- done | hit
      }
      n <- sum(done)
      if (n > 0 && show) cat(ab(v), ": ", cnt(n), " missing ", s1(n, "value", "values"), " generated\n", sep = "")
      attributes(y) <- attributes(x)
      d[[v]] <- y
    } else {
      mk <- stata_mv(y)
      for (k in seq_along(vals)) {
        val <- vals[[k]]
        if (fl) val <- stata_float(val)
        there <- sum(sel & !is.na(y) & y == val)
        if (there > 0 && !override) {
          clash <- TRUE
          if (show) cat(ab(v), ": already ", formatC(vals[[k]], digits = 9, format = "g", width = 1), " in ", sprintf("%4d", there), " observation", s1(there, "", "s"), "\n", sep = "")
        }
      }
      plan[[v]] <- list(x = x, y = y, sel = sel, mk = mk, tag = tag, fl = fl)
    }
  }
  if (kind == "decode") return(d)
  if (clash) stop("no action taken (Stata: r(9))", call. = FALSE)
  for (v in names(plan)) {
    p <- plan[[v]]
    y <- p$y
    n <- 0
    done <- rep(FALSE, length(y))
    for (k in seq_along(vals)) {
      val <- vals[[k]]
      want <- if (codes[k] == "*") p$mk > 0 else p$mk == (if (codes[k] == "") 1 else match(codes[k], letters) + 1)
      hit <- p$sel & !done & want
      if (p$fl) val <- stata_float(val)
      if (any(hit)) y[hit] <- val
      done <- done | hit
    }
    n <- sum(done)
    if (n > 0) {
      if (show) cat(ab(v), ": ", cnt(n), " missing ", s1(n, "value", "values"), " recoded\n", sep = "")
      if (!is.na(p$tag) && p$tag %in% names(lim)) {
        types <- vapply(unlist(vals), fit, "")
        need <- types[which.max(rank[types])]
        if (rank[need] > rank[p$tag] && show) cat("              (recast to ", need, ")\n", sep = "")
      }
    }
    attributes(y) <- attributes(p$x)
    d[[v]] <- y
  }
  d
}"#;

/// What reading the `mv()` text of mvdecode or mvencode can end in.
enum Mv {
    /// Stata's own error: its words and return code.
    Error(String, u32),
    /// A form that is not translated.
    Refused(Untranslated),
}

/// `1 2 5/7` as numbers (a range counts by 1).
fn mv_numlist(text: &str) -> Result<Vec<f64>, Mv> {
    let mut out = Vec::new();
    for tok in text.split_whitespace() {
        if tok.contains('(') || tok.contains(')') {
            return Err(Mv::Refused(no(
                "mvdecode with a numlist that has a step, as 1(2)9, is not translated yet",
            )));
        }
        if let Some((a, b)) = tok.split_once('/') {
            if let (Ok(a), Ok(b)) = (a.parse::<f64>(), b.parse::<f64>()) {
                if a.is_finite() && b.is_finite() && a <= b {
                    if b - a > 10_000.0 {
                        return Err(Mv::Refused(no(
                            "mvdecode with a range of more than 10,000 values is not translated",
                        )));
                    }
                    let mut x = a;
                    while x <= b {
                        out.push(x);
                        x += 1.0;
                    }
                    continue;
                }
            }
        } else if let Ok(x) = tok.parse::<f64>() {
            if x.is_finite() {
                out.push(x);
                continue;
            }
        }
        return Err(Mv::Error(format!("{tok} is not a valid numlist"), 121));
    }
    if out.is_empty() {
        return Err(Mv::Error(" is not a valid numlist".into(), 121));
    }
    Ok(out)
}

/// `.` or `.a` to `.z` as the letter ("" for `.`).
fn mv_code(text: &str) -> Result<String, Mv> {
    let t = text.trim();
    match t.strip_prefix('.') {
        Some("") => Ok(String::new()),
        Some(l) if l.len() == 1 && l.chars().all(|c| c.is_ascii_lowercase()) => Ok(l.to_string()),
        _ => Err(Mv::Error(
            format!("{text} is not a missing value code"),
            198,
        )),
    }
}

/// A finite number.
fn mv_number(text: &str) -> Option<f64> {
    text.trim().parse::<f64>().ok().filter(|x| x.is_finite())
}

/// The clauses of an `mv()` option: the values of each, and the code of each
/// (for mvencode `*` is every kind of missing value, one value each).
fn mv_clauses(mv: &str, decode: bool) -> Result<(Vec<Vec<f64>>, Vec<String>), Mv> {
    let text = mv.trim();
    let parts: Vec<&str> = text.split('\\').map(str::trim).collect();
    let single = parts.len() == 1 && !text.contains('=');
    let mut vals = Vec::new();
    let mut codes = Vec::new();
    if decode {
        if single {
            vals.push(mv_numlist(text)?);
            codes.push(String::new());
        } else {
            for p in &parts {
                let Some((l, r)) = p.split_once('=') else {
                    return Err(Mv::Error("= expected".into(), 198));
                };
                vals.push(mv_numlist(l)?);
                codes.push(mv_code(r)?);
            }
        }
    } else if single {
        let x = mv_number(text)
            .ok_or_else(|| Mv::Error(format!("{text} is not a missing value code"), 198))?;
        vals.push(vec![x]);
        codes.push("*".into());
    } else {
        for p in &parts {
            let Some((l, r)) = p.split_once('=') else {
                return Err(Mv::Error(format!("{p} is not a missing value code"), 198));
            };
            let code = mv_code(l)?;
            let mut rest = r.split_whitespace();
            let first = rest.next().unwrap_or("");
            if let Some(extra) = rest.next() {
                return Err(Mv::Error(format!("{extra} found, where \\ expected"), 198));
            }
            let x = mv_number(first)
                .ok_or_else(|| Mv::Error(format!("'{first}' found where number expected"), 7))?;
            vals.push(vec![x]);
            codes.push(code);
        }
    }
    Ok((vals, codes))
}

impl Session {
    /// `mvdecode varlist [if] [in], mv(numlist | numlist = mvc [\ ...])` and
    /// `mvencode varlist [if] [in], mv(# | mvc = # [\ ...]) [override]`.
    pub(super) fn mvcode(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let decode = c.name == "mvdecode";
        let stop = |out: &mut Out, msg: &str, rc: u32| {
            out.line(format!(
                "stop({}, call. = FALSE)",
                r_text(&format!("{msg} (Stata: r({rc}))"))
            ));
        };
        if c.prefixes.iter().any(|p| matches!(p, Prefix::By { .. })) {
            return Err(no(format!("{} with by: is not translated yet", c.name)));
        }
        let known: &[(&str, usize)] = if decode {
            &[("mv", 2)]
        } else {
            &[("mv", 2), ("override", 4)]
        };
        if c.parts.weight.is_some() {
            stop(out, "weights not allowed", 101);
            return Ok(());
        }
        if let Some(o) = opts.other(known) {
            stop(out, &format!("option {o} not allowed"), 198);
            return Ok(());
        }
        if c.parts.main.trim().is_empty() {
            stop(out, "varlist required", 100);
            return Ok(());
        }
        let Some(mv) = opts.arg("mv", 2) else {
            stop(out, "option mv() required", 198);
            return Ok(());
        };
        let d = self.data()?;
        let (vals, codes) = match mv_clauses(&mv, decode) {
            Ok(v) => v,
            Err(Mv::Error(msg, rc)) => {
                stop(out, &msg, rc);
                return Ok(());
            }
            Err(Mv::Refused(r)) => return Err(r),
        };
        if !decode {
            // clauses that could take the same observation twice are not translated
            let mut seen = BTreeSet::new();
            let every = codes.iter().any(|k| k == "*");
            if (every && codes.len() > 1) || !codes.iter().all(|k| seen.insert(k.clone())) {
                return Err(no(
                    "mvencode with several clauses for the same missing value is not translated yet",
                ));
            }
        }
        // names written out, or a pattern
        let words: Vec<&str> = c.parts.main.split_whitespace().collect();
        let plain: Vec<String> = words
            .iter()
            .filter(|t| is_name(t))
            .map(|t| t.to_string())
            .collect();
        let all_plain = plain.len() == words.len() && !words.contains(&"_all");
        let vars = if all_plain {
            format!(
                "c({})",
                plain
                    .iter()
                    .map(|v| r_text(v))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        } else {
            format!("names(dplyr::select({d}, {}))", select(&c.parts.main)?)
        };
        // a value that may not fit the variable's type changes it (observed)
        let needs_type = !decode
            && vals
                .iter()
                .flatten()
                .any(|x| *x != x.trunc() || *x < -127.0 || *x > 100.0);
        if needs_type {
            let unknown = !all_plain
                || plain.iter().any(|v| {
                    self.storage.get(v).is_none() && self.kinds.get(v) != Some(&Kind::Text)
                });
            if unknown {
                return Err(no(
                    "mvencode with a value that may not fit the variable's type needs the storage \
                     type, which is known only for variables the do-file made (R does not keep \
                     Stata's types)",
                ));
            }
            if vals.iter().flatten().any(|x| x.abs() > 1.7e38)
                && plain
                    .iter()
                    .any(|v| self.storage.get(v) == Some(&Storage::Float))
            {
                return Err(no(
                    "mvencode of a value too large for a float is not translated yet",
                ));
            }
        }
        let keep = self.condition(c, out)?;
        let keep = match keep {
            Some(k) => format!("dplyr::pull(dplyr::mutate({d}, stata_rows = {k}), stata_rows)"),
            None => "NULL".into(),
        };
        let st: Vec<String> = self
            .storage
            .iter()
            .filter_map(|(v, s)| {
                let t = match s {
                    Storage::Byte => "byte",
                    Storage::Int => "int",
                    Storage::Long => "long",
                    Storage::Float => "float",
                    Storage::Double => "double",
                    Storage::Str => return None,
                };
                Some(format!("{} = {}", r_name(v), r_text(t)))
            })
            .collect();
        let st = if st.is_empty() {
            "character(0)".to_string()
        } else {
            format!("c({})", st.join(", "))
        };
        let vals_r = format!(
            "list({})",
            vals.iter()
                .map(|v| format!(
                    "c({})",
                    v.iter()
                        .map(|x| r_number(*x))
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
                .collect::<Vec<_>>()
                .join(", ")
        );
        let codes_r = format!(
            "c({})",
            codes
                .iter()
                .map(|k| r_text(k))
                .collect::<Vec<_>>()
                .join(", ")
        );
        self.helpers.insert("stata_mvcode");
        self.helpers.insert("stata_mv");
        self.helpers.insert("stata_float");
        out.package("dplyr");
        out.package("haven");
        let show = if self.quiet(c) { "FALSE" } else { "TRUE" };
        out.line(format!(
            "{d} <- stata_mvcode({d}, {vars}, {}, {vals_r}, {codes_r}, keep = {keep}, st = {st}, override = {}, show = {show})",
            r_text(if decode { "decode" } else { "encode" }),
            super::stats::r_logical(!decode && opts.has("override", 4)),
        ));
        // a value that did not fit may have changed the type: no longer known
        if needs_type {
            for v in &plain {
                self.storage.remove(v);
            }
        }
        Ok(())
    }
}
