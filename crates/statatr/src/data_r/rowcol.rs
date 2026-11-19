//! `clonevar`, `mvdecode`, `mvencode`, `fillin`, `expandcl` and `joinby`, as
//! Stata 19 does them on made-up data (batch A1 of the StatCraft port;
//! `plan/stata/clonevar.md` and the notes beside it).

use super::*;

/// The storage types a .dta file's header holds (haven does not keep them),
/// named by variable: `byte`, `int`, `long`, `float`, `double` or `str`. The
/// file formats 117 to 119 (Stata 13 and later); older files give none.
pub(super) const DTA_TYPES_HELPER: &str = r#"stata_dta_types <- function(path) {
  none <- setNames(character(0), character(0))
  if (!file.exists(path)) return(none)
  raw <- readBin(path, "raw", n = 2000000L)
  at <- function(tag) {
    p <- grepRaw(charToRaw(tag), raw, fixed = TRUE)
    if (length(p)) p[1] else NA_integer_
  }
  rel <- at("<release>")
  bo <- at("<byteorder>")
  k0 <- at("<K>")
  tv <- at("<variable_types>")
  nm <- at("<varnames>")
  if (anyNA(c(rel, bo, k0, tv, nm))) return(none)
  release <- suppressWarnings(as.integer(rawToChar(raw[(rel + 9L):(rel + 11L)])))
  if (is.na(release) || release < 117L) return(none)
  endian <- if (rawToChar(raw[(bo + 11L):(bo + 13L)]) == "MSF") "big" else "little"
  k0 <- k0 + 3L
  k <- if (release >= 119L) readBin(raw[k0:(k0 + 3L)], "integer", size = 4L, endian = endian) else readBin(raw[k0:(k0 + 1L)], "integer", size = 2L, signed = FALSE, endian = endian)
  tv <- tv + 16L
  codes <- readBin(raw[tv:(tv + 2L * k - 1L)], "integer", n = k, size = 2L, signed = FALSE, endian = endian)
  nm <- nm + 10L
  width <- if (release >= 118L) 129L else 33L
  names <- vapply(seq_len(k), function(i) {
    b <- raw[(nm + (i - 1L) * width):(nm + i * width - 1L)]
    z <- which(b == as.raw(0L))
    rawToChar(b[seq_len(if (length(z)) z[1] - 1L else width)])
  }, "")
  types <- ifelse(codes == 65530L, "byte", ifelse(codes == 65529L, "int", ifelse(codes == 65528L, "long", ifelse(codes == 65527L, "float", ifelse(codes == 65526L, "double", "str")))))
  setNames(types, names)
}"#;

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
    /// `by` or `bysort` before one of the six commands: Stata does not allow it
    /// and says `NAME may not be combined with by` r(190), after a name the data
    /// lack (r(111)) and the rows not being in order (`not sorted`, r(5); `bysort`
    /// sorts them first). True when the prefix was there and the stop is written.
    fn by_not_allowed(
        &mut self,
        c: &Command,
        name: &str,
        out: &mut Out,
    ) -> Result<bool, Untranslated> {
        let Some((varlist, sort)) = c.prefixes.iter().find_map(|p| match p {
            Prefix::By { varlist, sort, .. } => Some((varlist.clone(), *sort)),
            _ => None,
        }) else {
            return Ok(false);
        };
        let (group, within) = match varlist.split_once('(') {
            Some((g, w)) => (g.to_string(), w.trim_end_matches(')').to_string()),
            None => (varlist.clone(), String::new()),
        };
        let plain = |s: &str| -> Result<Vec<String>, Untranslated> {
            s.split_whitespace()
                .map(|w| {
                    if is_name(w) {
                        Ok(w.to_string())
                    } else {
                        Err(no(format!(
                            "by {w}: only variable names written out are translated in a by prefix"
                        )))
                    }
                })
                .collect()
        };
        let (group, within) = (plain(&group)?, plain(&within)?);
        if group.is_empty() {
            return Err(no("by needs a variable"));
        }
        let d = self.data()?;
        let vector = |v: &[String]| {
            let v: Vec<String> = v.iter().map(|x| r_text(x)).collect();
            format!("c({})", v.join(", "))
        };
        for v in group.iter().chain(&within) {
            out.line(format!(
                "if (!{} %in% names({d})) stop({}, call. = FALSE)",
                r_text(v),
                r_text(&format!("variable {v} not found (Stata: r(111))"))
            ));
        }
        if sort {
            if let Some((Some(arrange), _)) = self.by(c, out)? {
                out.line(format!("{d} <- {d} |> {arrange}"));
            }
        } else {
            self.helpers.insert("stata_by_groups");
            let tags = self.mv_on(out);
            out.line(format!(
                "invisible(stata_by_groups({d}, {}, within = {}, tags = {}))",
                vector(&group),
                vector(&within),
                super::stats::r_logical(tags)
            ));
        }
        out.line(format!(
            "stop({}, call. = FALSE)",
            r_text(&format!(
                "{name} may not be combined with by (Stata: r(190))"
            ))
        ));
        Ok(true)
    }

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
        if self.by_not_allowed(c, "clonevar", out)? {
            return Ok(());
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
pub(super) const MVCODE_HELPER: &str = r#"stata_mvcode <- function(d, vars, kind, vals, codes, keep = NULL, st = character(0), override = FALSE, show = TRUE, ft = character(0), must_know = FALSE) {
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
    if (kind != "decode" && is.na(tag) && v %in% names(ft)) tag <- unname(ft[v])
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
    used <- integer(0)
    for (k in seq_along(vals)) {
      val <- vals[[k]]
      want <- if (codes[k] == "*") p$mk > 0 else p$mk == (if (codes[k] == "") 1 else match(codes[k], letters) + 1)
      hit <- p$sel & !done & want
      if (p$fl) val <- stata_float(val)
      if (any(hit)) {
        y[hit] <- val
        used <- c(used, k)
      }
      done <- done | hit
    }
    n <- sum(done)
    if (n > 0 && must_know && is.na(p$tag)) stop("RCraft: mvencode needs the storage type of ", v, ", which this data does not carry; not translated", call. = FALSE)
    if (n > 0 && must_know && identical(p$tag, "float") && any(abs(unlist(vals[used])) > 1.7e38)) stop("RCraft: mvencode of a value too large for a float is not translated yet", call. = FALSE)
    if (n > 0) {
      if (show) cat(ab(v), ": ", cnt(n), " missing ", s1(n, "value", "values"), " recoded\n", sep = "")
      if (!is.na(p$tag) && p$tag %in% names(lim)) {
        types <- vapply(unlist(vals[used]), fit, "")
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

/// One element of a numlist.
enum NumItem {
    Num(f64),
    Many(Vec<f64>),
}

/// `lo` to `hi` in steps of `step` (its sign is ignored and the ends may come in
/// either order, as Stata's numlist takes them). A value is kept to 14 digits, so
/// `0(.1).3` holds the .3 a variable holds.
fn mv_steps(a: f64, b: f64, step: f64) -> Result<Vec<f64>, bool> {
    let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
    let s = step.abs();
    if s.is_nan() || s <= 0.0 || !lo.is_finite() || !hi.is_finite() {
        return Err(false);
    }
    let n = ((hi - lo) / s + 1e-9).floor();
    if n.is_nan() {
        return Err(false);
    }
    if n > 10_000.0 {
        return Err(true);
    }
    Ok((0..=n as u64)
        .map(|i| {
            let x = lo + i as f64 * s;
            format!("{x:.13e}").parse::<f64>().unwrap_or(x)
        })
        .collect())
}

/// A numlist as numbers: `1 2`, `5/7` (by 1), `1(2)9`, `1 3 to 9` and `1 3 : 9`
/// (the step is the gap between the first two), commas between numbers, spaces
/// around parentheses. Stata says `TEXT is not a valid numlist` r(121) for any
/// other form (`5 to 8`, `1(0)3`, `1(2)3(4)5`, `1,,3`).
fn mv_numlist(text: &str) -> Result<Vec<f64>, Mv> {
    let bad = || Mv::Error(format!("{text} is not a valid numlist"), 121);
    let t = text.trim();
    if t.starts_with(',') || t.ends_with(',') || t.contains(",,") || t.contains(", ,") {
        return Err(bad());
    }
    // spaces next to a parenthesis and commas are nothing
    let mut s = String::new();
    for ch in t.chars() {
        match ch {
            ',' => s.push(' '),
            '(' | ')' => {
                while s.ends_with(' ') {
                    s.pop();
                }
                s.push(ch);
            }
            c => s.push(c),
        }
    }
    let s = s
        .replace("( ", "(")
        .replace(") ", ")")
        .replace(" /", "/")
        .replace("/ ", "/");
    let steps = |a: f64, b: f64, st: f64| {
        mv_steps(a, b, st).map_err(|big| {
            if big {
                Mv::Refused(no(
                    "mvdecode with a range of more than 10,000 values is not translated",
                ))
            } else {
                bad()
            }
        })
    };
    let num = |w: &str| w.parse::<f64>().ok().filter(|x| x.is_finite());
    let words: Vec<&str> = s.split_whitespace().collect();
    let mut items: Vec<NumItem> = Vec::new();
    let mut i = 0;
    while let Some(&w) = words.get(i) {
        if w == "to" || w == ":" {
            // `n1 n2 to n3`
            let (Some(NumItem::Num(b)), Some(NumItem::Num(a))) = (items.pop(), items.pop()) else {
                return Err(bad());
            };
            let Some(c) = words.get(i + 1).and_then(|c| num(c)) else {
                return Err(bad());
            };
            let v = steps(a, c, b - a)?;
            items.push(NumItem::Many(v));
            i += 2;
            continue;
        }
        if let Some((a, rest)) = w.split_once('(') {
            let Some((st, b)) = rest.split_once(')') else {
                return Err(bad());
            };
            let (Some(a), Some(st), Some(b)) = (num(a), num(st), num(b)) else {
                return Err(bad());
            };
            let v = steps(a, b, st)?;
            items.push(NumItem::Many(v));
        } else if let Some((a, b)) = w.split_once('/') {
            let (Some(a), Some(b)) = (num(a), num(b)) else {
                return Err(bad());
            };
            let v = steps(a, b, 1.0)?;
            items.push(NumItem::Many(v));
        } else if let Some(x) = num(w) {
            items.push(NumItem::Num(x));
        } else {
            return Err(bad());
        }
        i += 1;
    }
    let mut out = Vec::new();
    for it in items {
        match it {
            NumItem::Num(x) => out.push(x),
            NumItem::Many(v) => out.extend(v),
        }
    }
    if out.is_empty() {
        return Err(bad());
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
    // a backslash at the end is nothing (`1 = .a \`); one anywhere else with
    // nothing beside it is not a list or a code
    let parts: Vec<&str> = match parts.split_last() {
        Some((last, rest)) if last.is_empty() && !rest.is_empty() => rest.to_vec(),
        _ => parts,
    };
    if !single && parts.iter().any(|p| p.is_empty()) {
        return Err(if decode {
            Mv::Error("\\ is not a valid numlist".into(), 121)
        } else {
            Mv::Error("\\ is not a missing value code".into(), 198)
        });
    }
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
        if self.by_not_allowed(c, &c.name, out)? {
            return Ok(());
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
        let mut file_types: Option<String> = None;
        if needs_type {
            let unknown = !all_plain
                || plain.iter().any(|v| {
                    !self.storage.contains_key(v) && self.kinds.get(v) != Some(&Kind::Text)
                });
            // the types of a file `use` read are in its header, read when the R runs
            if unknown && self.file_types.is_none() {
                return Err(no(
                    "mvencode with a value that may not fit the variable's type needs the storage \
                     type, which is known only for variables the do-file made or a file it has \
                     read and not changed since (R does not keep Stata's types)",
                ));
            }
            if unknown {
                file_types = self.file_types.clone();
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
        let types = match &file_types {
            Some(path) => {
                self.helpers.insert("stata_dta_types");
                format!(", ft = stata_dta_types({}), must_know = TRUE", r_text(path))
            }
            None => String::new(),
        };
        out.line(format!(
            "{d} <- stata_mvcode({d}, {vars}, {}, {vals_r}, {codes_r}, keep = {keep}, st = {st}, override = {}, show = {show}{types})",
            r_text(if decode { "decode" } else { "encode" }),
            super::stats::r_logical(!decode && opts.has("override", 4)),
        ));
        // a value that did not fit may have changed the type: no longer known
        if needs_type {
            for v in &plain {
                self.storage.remove(v);
            }
            // a type may have changed: the file's header no longer says
            self.file_types = None;
        }
        Ok(())
    }
}

/// `fillin`: a row for every combination of the variables' values that the
/// data lack (`plan/stata/fillin.md`). The levels of a variable are the
/// values it has, `.` and each of `.a` to `.z` a level of its own and `""`
/// one; the added rows have the other variables missing and `_fillin` 1
/// (0 on the rows that were there); the data end sorted by the variables
/// (missing last, rows that repeat a combination kept in their order).
pub(super) const FILLIN_HELPER: &str = r#"stata_fillin <- function(d, vars) {
  for (v in vars) if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
  if (anyDuplicated(vars)) stop("using data not sorted (Stata: r(5))", call. = FALSE)
  if ("_fillin" %in% names(d)) stop("variable _fillin already defined (Stata: r(110))", call. = FALSE)
  n <- nrow(d)
  if (n == 0) stop("fillin of data with no observations is not translated yet", call. = FALSE)
  strip <- function(x) { if (is.factor(x)) x <- as.integer(x); attributes(x) <- NULL; x }
  bare <- lapply(d[vars], strip)
  kinds <- function(b) lapply(b, function(x) if (is.double(x) && anyNA(x)) stata_mv(x) else rep(0, length(x)))
  mv <- kinds(bare)
  levels <- lapply(seq_along(vars), function(j) which(!duplicated(data.frame(a = bare[[j]], b = mv[[j]]))))
  size <- prod(vapply(levels, length, 1))
  if (size > 5e7) stop("too many observations would be made (Stata: r(901))", call. = FALSE)
  grid <- expand.grid(levels, KEEP.OUT.ATTRS = FALSE)
  key <- function(rows) {
    cols <- c(lapply(seq_along(vars), function(j) bare[[j]][rows[[j]]]), lapply(seq_along(vars), function(j) mv[[j]][rows[[j]]]))
    names(cols) <- paste0("k", seq_along(cols))
    as.data.frame(cols, stringsAsFactors = FALSE)
  }
  have <- key(rep(list(seq_len(n)), length(vars)))
  want <- key(as.list(grid))
  new <- which(is.na(vctrs::vec_match(want, have)))
  d[["_fillin"]] <- structure(rep(0, n), format.stata = "%8.0g")
  keep <- lapply(d, attributes)
  if (length(new)) {
    add <- lapply(names(d), function(v) {
      j <- match(v, vars)
      x <- if (!is.na(j)) d[[v]][grid[[j]][new]] else if (v == "_fillin") rep(1, length(new)) else if (is.character(d[[v]])) rep("", length(new)) else d[[v]][rep(NA_integer_, length(new))]
      mostattributes(x) <- attributes(d[[v]])
      x
    })
    names(add) <- names(d)
    d <- dplyr::bind_rows(d, as.data.frame(add, stringsAsFactors = FALSE, optional = TRUE))
  }
  b2 <- lapply(d[vars], strip)
  m2 <- kinds(b2)
  by <- unlist(lapply(seq_along(vars), function(j) list(b2[[j]], m2[[j]])), recursive = FALSE)
  d <- d[do.call(order, c(by, list(na.last = TRUE, method = "radix"))), , drop = FALSE]
  # the variable label and display format stay on every column
  for (v in names(d)) for (a in c("label", "format.stata")) {
    if (is.null(attr(d[[v]], a, exact = TRUE)) && !is.null(keep[[v]][[a]])) attr(d[[v]], a) <- keep[[v]][[a]]
  }
  rownames(d) <- NULL
  d
}"#;

impl Session {
    /// `fillin varlist`.
    ///
    /// Errors, in the order Stata gives them (observed): no variables
    /// `varlist required` r(100); one variable `too few variables specified`
    /// r(102); a name the data lack r(111); a variable twice r(5); `_fillin`
    /// already there r(110).
    pub(super) fn fillin(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let stop = |out: &mut Out, msg: &str| {
            out.line(format!("stop({}, call. = FALSE)", r_text(msg)));
        };
        if self.by_not_allowed(c, "fillin", out)? {
            return Ok(());
        }
        if c.parts.weight.is_some() {
            stop(out, "weights not allowed (Stata: r(101))");
            return Ok(());
        }
        if c.parts.if_exp.is_some() || c.parts.in_range.is_some() {
            stop(out, "if not allowed (Stata: r(101))");
            return Ok(());
        }
        if let Some(o) = opts.other(&[]) {
            stop(out, &format!("option {o} not allowed (Stata: r(198))"));
            return Ok(());
        }
        let words: Vec<&str> = c.parts.main.split_whitespace().collect();
        if words.is_empty() {
            stop(out, "varlist required (Stata: r(100))");
            return Ok(());
        }
        if words.len() == 1 {
            stop(out, "too few variables specified (Stata: r(102))");
            return Ok(());
        }
        let vars = names(&c.parts.main)?;
        let d = self.data()?;
        self.helpers.insert("stata_fillin");
        self.helpers.insert("stata_mv");
        out.package("dplyr");
        out.package("vctrs");
        out.package("haven");
        out.line(format!(
            "{d} <- stata_fillin({d}, c({}))",
            vars.iter()
                .map(|v| r_text(v))
                .collect::<Vec<_>>()
                .join(", ")
        ));
        self.kinds.insert("_fillin".into(), Kind::Number);
        self.storage.insert("_fillin".into(), Storage::Byte);
        Ok(())
    }
}

/// `expandcl` (`plan/stata/expandcl.md`): each selected observation becomes
/// `n` copies (a count that is missing, zero or negative keeps the one
/// observation, a fraction is rounded), one cluster at a time; `gen` numbers
/// the (cluster, copy) pairs, the clusters in the sort order of their
/// variables (missing last), a cluster with k copies taking k numbers in a
/// row; rows outside `if`/`in` are 1. Stata puts the copies of one row in an
/// order of its own; here they follow the row, in the order of their numbers.
pub(super) const EXPANDCL_HELPER: &str = r#"stata_expandcl <- function(d, n, keep, cluster, gen, show = TRUE) {
  for (v in cluster) if (!v %in% names(d)) stop(v, " not found (Stata: r(111))", call. = FALSE)
  if (gen %in% names(d)) stop("variable ", gen, " already defined (Stata: r(110))", call. = FALSE)
  rows <- nrow(d)
  sel <- if (is.null(keep)) rep(TRUE, rows) else (!is.na(keep) & keep)
  n <- as.numeric(n)
  strip <- function(x) { if (is.factor(x)) x <- as.integer(x); attributes(x) <- NULL; x }
  bare <- lapply(d[cluster], strip)
  mv <- lapply(bare, function(x) if (is.double(x) && anyNA(x)) stata_mv(x) else rep(0, length(x)))
  keys <- as.data.frame(c(setNames(bare, paste0("k", seq_along(bare))), setNames(mv, paste0("m", seq_along(mv)))), stringsAsFactors = FALSE)
  g <- rep(NA_integer_, rows)
  g[sel] <- vctrs::vec_group_id(keys[sel, , drop = FALSE])
  idx <- which(sel)
  # one count for the whole cluster
  for (k in unique(g[idx])) {
    v <- n[idx][g[idx] == k]
    if (length(unique(v)) > 1) stop("expression is not constant within clusters (Stata: r(198))", call. = FALSE)
  }
  say <- function(k, one, many) if (k > 0) cat("(", k, " ", if (k == 1) one else many, ")\n", sep = "")
  ns <- n[idx]
  if (show) {
    say(sum(is.na(ns)), "missing count ignored; observation not deleted", "missing counts ignored; observations not deleted")
    say(sum(!is.na(ns) & ns != floor(ns)), "noninteger count rounded to integer", "noninteger counts rounded to integer")
  }
  r <- floor(ns + 0.5)
  if (show) {
    say(sum(!is.na(r) & r < 0), "negative count ignored; observation not deleted", "negative counts ignored; observations not deleted")
    say(sum(!is.na(r) & r == 0), "zero count ignored; observation not deleted", "zero counts ignored; observations not deleted")
  }
  r[is.na(r) | r < 1] <- 1
  kk <- rep(1, rows)
  kk[idx] <- r
  # the clusters in sort order, missing last
  by <- unlist(lapply(seq_along(bare), function(j) list(bare[[j]], mv[[j]])), recursive = FALSE)
  ord <- do.call(order, c(by, list(na.last = TRUE, method = "radix")))
  ord <- ord[sel[ord]]
  first <- g[ord][!duplicated(g[ord])]
  size <- vapply(first, function(k) max(kk[which(g == k)]), 1)
  start <- c(0, cumsum(size))[seq_along(first)] + 1
  begin <- rep(1, rows)
  begin[idx] <- start[match(g[idx], first)]
  take <- rep(seq_len(rows), times = kk)
  copy <- sequence(kk)
  out <- d[take, , drop = FALSE]
  for (v in names(d)) {
    x <- out[[v]]
    a <- attributes(d[[v]])
    a$names <- NULL
    for (nm in names(a)) if (is.null(attr(x, nm, exact = TRUE))) attr(x, nm) <- a[[nm]]
    out[[v]] <- x
  }
  out[[gen]] <- structure(begin[take] + copy - 1, label = "New cluster ID from expandcl", format.stata = "%8.0g")
  rownames(out) <- NULL
  if (show) cat("(", nrow(out) - rows, if (nrow(out) - rows == 1) " observation created" else " observations created", ")\n", sep = "")
  out
}"#;

impl Session {
    /// `expandcl [=]exp [if] [in], cluster(varlist) generate(newvar)`.
    ///
    /// Observed in Stata 19: both options are required (`option generate()
    /// required` r(198), then cluster()); no expression is r(198); a string
    /// expression r(109); `newvar` that is there r(110); an expression that
    /// is not the same for every selected observation of a cluster r(198).
    pub(super) fn expandcl(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let stop = |out: &mut Out, msg: &str| {
            out.line(format!("stop({}, call. = FALSE)", r_text(msg)));
        };
        if self.by_not_allowed(c, "expandcl", out)? {
            return Ok(());
        }
        if c.parts.weight.is_some() {
            stop(out, "weights not allowed (Stata: r(101))");
            return Ok(());
        }
        if let Some(o) = opts.other(&[("cluster", 2), ("generate", 3)]) {
            stop(out, &format!("option {o} not allowed (Stata: r(198))"));
            return Ok(());
        }
        let Some(gen) = opts.arg("generate", 3) else {
            stop(out, "option generate() required (Stata: r(198))");
            return Ok(());
        };
        let Some(cluster) = opts.arg("cluster", 2) else {
            stop(out, "option cluster() required (Stata: r(198))");
            return Ok(());
        };
        let exp = c.parts.main.trim().trim_start_matches('=').trim();
        if exp.is_empty() {
            stop(out, "invalid syntax (Stata: r(198))");
            return Ok(());
        }
        let gen = gen.trim().to_string();
        if !is_name(&gen) {
            return Err(no(format!(
                "expandcl, generate({gen}): one new name is needed"
            )));
        }
        let cluster = names(&cluster)?;
        let d = self.data()?;
        let (cond, n, kind) = self.drawing_value(c, exp, out)?;
        if kind == Kind::Text {
            stop(out, "type mismatch (Stata: r(109))");
            return Ok(());
        }
        let keep = match cond {
            Some(k) => format!("dplyr::pull(dplyr::mutate({d}, stata_rows = {k}), stata_rows)"),
            None => "NULL".into(),
        };
        self.helpers.insert("stata_expandcl");
        self.helpers.insert("stata_mv");
        out.package("dplyr");
        out.package("vctrs");
        out.package("haven");
        let show = if self.quiet(c) { "FALSE" } else { "TRUE" };
        out.line(format!(
            "{d} <- stata_expandcl({d}, dplyr::transmute({d}, stata_n = {n})$stata_n, {keep}, c({}), {}, show = {show})",
            cluster
                .iter()
                .map(|v| r_text(v))
                .collect::<Vec<_>>()
                .join(", "),
            r_text(&gen)
        ));
        out.caveat(
            "Stata puts the copies of one observation in an order of its own; here they follow the \
             observation, in the order of their cluster numbers",
        );
        self.kinds.insert(gen, Kind::Number);
        Ok(())
    }
}

/// `joinby` (`plan/stata/joinby.md`): every pairing of a master row with
/// the using rows that have the same key; the master's rows stay in their
/// order (an unmatched one where it is), the using-only rows follow in key
/// order. The new variables are the using file's other variables, after
/// `_merge` when there is one. A variable in both files keeps the master's
/// value; a using-only row has the keys and nothing else of the master.
/// With `update` a master value that is missing (any kind) or "" in a paired
/// row takes the using file's value, whatever it is; with `replace` as well a
/// master value is replaced by a using value that is not missing or "". A
/// using-only row then has the using file's values of the variables in both.
/// A variable that is a string in one file and a number in the other is left
/// as the master has it. The paired rows are always `_merge` 3.
pub(super) const JOINBY_HELPER: &str = r#"stata_joinby <- function(d, u, keys, common = FALSE, unmatched = "none", merge = "_merge", update = FALSE, replace = FALSE) {
  if (common) {
    keys <- intersect(names(d), names(u))
    if (!length(keys)) stop("no variables in common to master and using data (Stata: r(198))", call. = FALSE)
    cat("join on common variables: ", paste(keys, collapse = " "), "\n", sep = "")
  }
  for (k in keys) if (!k %in% names(d) || !k %in% names(u)) stop("variable ", k, " not found (Stata: r(111))", call. = FALSE)
  for (k in keys) if (is.character(d[[k]]) != is.character(u[[k]])) stop("key variable ", k, " is ", if (is.character(d[[k]])) "a string" else "numeric", " in master but ", if (is.character(u[[k]])) "a string" else "numeric", " in using data (Stata: r(106))", call. = FALSE)
  if (replace && !update) stop("option replace may not be specified without update (Stata: r(198))", call. = FALSE)
  if (unmatched != "none" && merge %in% names(d)) stop("variable ", merge, " already defined (Stata: r(110))", call. = FALSE)
  cv <- setdiff(intersect(names(d), names(u)), keys)
  cv <- cv[vapply(cv, function(v) is.character(d[[v]]) == is.character(u[[v]]), TRUE)]
  strip <- function(x) { if (is.factor(x)) x <- as.integer(x); attributes(x) <- NULL; x }
  key <- function(z) {
    b <- lapply(z[keys], strip)
    m <- lapply(b, function(x) if (is.double(x) && anyNA(x)) stata_mv(x) else rep(0, length(x)))
    as.data.frame(c(setNames(b, paste0("k", seq_along(b))), setNames(m, paste0("m", seq_along(m)))), stringsAsFactors = FALSE)
  }
  kd <- key(d)
  ku <- key(u)
  g <- vctrs::vec_group_id(rbind(kd, ku))
  gd <- g[seq_len(nrow(d))]
  gu <- g[nrow(d) + seq_len(nrow(u))]
  byg <- split(seq_along(gu), gu)
  hits <- byg[as.character(gd)]
  n <- vapply(hits, length, 1L)
  keep_master <- unmatched %in% c("master", "both")
  reps <- ifelse(n > 0, n, if (keep_master) 1L else 0L)
  midx <- rep(seq_len(nrow(d)), reps)
  uidx <- unlist(lapply(seq_along(hits), function(i) if (n[i] > 0) hits[[i]] else if (keep_master) NA_integer_ else integer(0)))
  if (is.null(uidx)) uidx <- integer(0)
  code <- ifelse(is.na(uidx), 1, 3)
  if (unmatched %in% c("using", "both")) {
    only <- which(!gu %in% gd)
    if (length(only)) {
      by <- unlist(lapply(seq_along(keys), function(j) list(ku[[j]][only], ku[[length(keys) + j]][only])), recursive = FALSE)
      only <- only[do.call(order, c(by, list(na.last = TRUE, method = "radix")))]
      midx <- c(midx, rep(NA_integer_, length(only)))
      uidx <- c(uidx, only)
      code <- c(code, rep(2, length(only)))
    }
  }
  from_u <- is.na(midx)
  out <- list()
  for (v in names(d)) {
    x <- d[[v]]
    xv <- as.vector(unclass(x))[midx]
    if (v %in% keys && any(from_u)) {
      yv <- as.vector(unclass(u[[v]]))[uidx]
      if (is.character(xv)) xv[from_u] <- yv[from_u] else xv[from_u] <- yv[from_u]
    }
    if (update && v %in% cv) {
      yv <- as.vector(unclass(u[[v]]))[uidx]
      both <- !from_u & !is.na(uidx)
      mx <- if (is.character(xv)) is.na(xv) | xv == "" else is.na(xv)
      my <- if (is.character(yv)) is.na(yv) | yv == "" else is.na(yv)
      take <- from_u | (both & (mx | (replace & !my)))
      xv[take] <- yv[take]
    }
    if (is.character(xv)) xv[is.na(xv)] <- ""
    a <- attributes(x)
    a$names <- NULL
    for (nm in names(a)) attr(xv, nm) <- a[[nm]]
    out[[v]] <- xv
  }
  if (unmatched != "none") {
    lab <- if (update) c("only in master data" = 1, "only in using data" = 2, "in both, master agrees with using data" = 3, "in both, missing in master data updated" = 4, "in both, master disagrees with using data" = 5) else c("only in master data" = 1, "only in using data" = 2, "both in master and using data" = 3)
    out[[merge]] <- structure(haven::labelled(as.numeric(code), lab), format.stata = if (update) "%41.0g" else "%29.0g")
  }
  for (v in setdiff(names(u), names(d))) {
    x <- u[[v]]
    xv <- as.vector(unclass(x))[uidx]
    if (is.character(xv)) xv[is.na(xv)] <- ""
    a <- attributes(x)
    a$names <- NULL
    for (nm in names(a)) attr(xv, nm) <- a[[nm]]
    out[[v]] <- xv
  }
  res <- as.data.frame(out, stringsAsFactors = FALSE, optional = TRUE)
  names(res) <- names(out)
  if (!nrow(res)) cat("(joinby on ", paste(keys, collapse = " "), " is empty)\n", sep = "")
  res
}"#;

impl Session {
    /// `joinby [varlist] using filename [, unmatched(none|both|master|using)
    /// _merge(varname)]`.
    ///
    /// Observed in Stata 19: no `using` is r(100); a name the data lack
    /// r(111); a file that is not there r(601); `unmatched(foo)` `foo is
    /// invalid value for unmatch()` r(198); another option r(198); `if` is
    /// r(101); no variable in common r(198); a key that is a string in one
    /// file and a number in the other r(106); `_merge` there when
    /// `unmatched()` asks for it r(110). With no varlist it joins on the
    /// variables in common and says `join on common variables: k`.
    pub(super) fn joinby(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let stop = |out: &mut Out, msg: &str| {
            out.line(format!("stop({}, call. = FALSE)", r_text(msg)));
        };
        if self.by_not_allowed(c, "joinby", out)? {
            return Ok(());
        }
        let Some(file) = c.using.clone() else {
            stop(out, "using required (Stata: r(100))");
            return Ok(());
        };
        if c.parts.if_exp.is_some() || c.parts.in_range.is_some() {
            stop(out, "if not allowed (Stata: r(101))");
            return Ok(());
        }
        if let Some(o) = opts.other(&[
            ("unmatched", 1),
            ("_merge", 2),
            ("update", 6),
            ("replace", 7),
        ]) {
            stop(out, &format!("option {o} not allowed (Stata: r(198))"));
            return Ok(());
        }
        let update = opts.has("update", 6);
        let replace = opts.has("replace", 7);
        let unmatched = match opts.arg("unmatched", 1) {
            None => "none".to_string(),
            Some(u) => {
                let u = u.trim().to_ascii_lowercase();
                let full = ["none", "both", "master", "using"];
                match full
                    .iter()
                    .find(|f| !u.is_empty() && f.starts_with(u.as_str()))
                {
                    Some(f) => f.to_string(),
                    None => {
                        stop(
                            out,
                            &format!("{u} is invalid value for unmatch() (Stata: r(198))"),
                        );
                        return Ok(());
                    }
                }
            }
        };
        let merge = match opts.arg("_merge", 2) {
            Some(m) if is_name(m.trim()) => m.trim().to_string(),
            Some(m) => return Err(no(format!("joinby, _merge({m}): one new name is needed"))),
            None => "_merge".to_string(),
        };
        let words: Vec<&str> = c.parts.main.split_whitespace().collect();
        let common = words.is_empty();
        let keys = if common {
            Vec::new()
        } else {
            names(&c.parts.main)?
        };
        let d = self.data()?;
        let path = file_name(&file, "dta");
        self.saved_types(&path, &d, &keys, out);
        // a variable in both files takes the wider type: not known here, so the
        // storage of one the two files hold differently is no longer known
        if let Some(kept) = self.saved.get(&self.file_key(&path)).cloned() {
            for (v, s) in &kept.storage {
                if self.storage.get(v).is_some_and(|m| m != s) {
                    self.storage.remove(v);
                }
            }
        }
        // the file may hold .a to .z (tagged NAs)
        self.tags.set(true);
        self.helpers.insert("stata_joinby");
        self.helpers.insert("stata_mv");
        out.package("haven");
        out.package("vctrs");
        let p = r_text(&path);
        out.line(format!(
            "if (!file.exists({p})) stop({}, call. = FALSE)",
            r_text(&format!("file {path} not found (Stata: r(601))"))
        ));
        out.line(format!(
            "{d} <- stata_joinby({d}, haven::read_dta({p}), c({}), common = {}, unmatched = {}, merge = {}, update = {}, replace = {})",
            keys.iter().map(|k| r_text(k)).collect::<Vec<_>>().join(", "),
            super::stats::r_logical(common),
            r_text(&unmatched),
            r_text(&merge),
            super::stats::r_logical(update),
            super::stats::r_logical(replace)
        ));
        if unmatched != "none" {
            self.kinds.insert(merge, Kind::Number);
        }
        out.caveat(
            "Stata puts the rows paired within one key in an order of its own, and the order \
             here is the using file's; sort before looking at them",
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::data_r::translate;
    use crate::reader::read;

    fn r(source: &str) -> Vec<String> {
        translate(&read(source))
            .lines
            .into_iter()
            .map(|l| match l.result {
                Ok(r) => r.code,
                Err(e) => format!("NOT: {}", e.reason),
            })
            .collect()
    }

    #[test]
    fn a_file_the_do_file_read_gives_the_storage_types_for_mvencode() {
        let out = r("use survey, clear\nmvencode a b, mv(-999)\nsort a\nmvencode a b, mv(-998)");
        assert!(
            out[1].contains("ft = stata_dta_types(\"survey.dta\"), must_know = TRUE"),
            "{out:?}"
        );
        // the first mvencode may have recast a variable, so the file no longer says
        assert!(
            out[3].starts_with("NOT:"),
            "the first mvencode may have recast: {out:?}"
        );
    }

    #[test]
    fn a_value_that_fits_a_byte_needs_no_types() {
        let out = r("use survey, clear\nreplace a = 1\nmvencode a b, mv(-99)");
        assert!(!out[2].contains("stata_dta_types"), "{out:?}");
        assert!(!out[2].starts_with("NOT:"), "{out:?}");
    }

    #[test]
    fn a_change_after_use_takes_the_types_back() {
        for change in [
            "replace a = 1",
            "recode a (1 = 2)",
            "rename a z",
            "merge 1:1 id using other",
            "compress",
        ] {
            let out = r(&format!(
                "use survey, clear\n{change}\nmvencode b, mv(-999)"
            ));
            assert!(
                out[2].starts_with("NOT:") && out[2].contains("storage"),
                "{change}: {out:?}"
            );
        }
    }

    #[test]
    fn a_file_the_do_file_saved_keeps_its_own_types() {
        let out =
            r("generate byte a = 1\nsave made, replace\nuse made, clear\nmvencode a, mv(-999)");
        assert!(!out[3].contains("stata_dta_types"), "{out:?}");
    }
}
