//! P2C.9 continued, time series: `tsset` and the operators `L.`, `F.`, `D.`
//! and `S.` (observed on Stata 19, `plan/stata/timeseries.md`).
//!
//! - `tsset t` and `tsset id t` (the same as `xtset id t`) check the times,
//!   sort the data by panel and time and say what they found. A row whose
//!   time or panel is missing takes no part.
//! - `L2.x` is x at time t - 2 delta in the same panel, missing when no row
//!   has that time; `F.` looks ahead; `D.x` is x - L.x, `D2.x` is
//!   x - 2 L.x + L2.x and `S2.x` is x - L2.x. A chain is expanded into one
//!   sum over x, so `D.F.x` is F.x - x and needs the rows at t + 1 and t
//!   only. The operators are worked out over every row, whatever `if` keeps.
//! - A chain is one canonical name: the net lag (or lead), then the
//!   differences, then the seasonal difference (`d.l.x` is `LD.x`, `L.F.x`
//!   is `x`, `S2D.x` is `DS2.x`). Models name their coefficients so.
//! - When the data are not in panel and time order Stata stops with "not
//!   sorted", r(5).

use super::*;

/// `tsset`: checks the times, sorts the data by panel and time, and prints
/// what Stata prints.
pub(super) const TSSET_HELPER: &str = r#"stata_tsset <- function(d, time, panel = NULL, delta = 1) {
  tv <- d[[time]]
  for (v in c(panel, time)) {
    if (is.character(d[[v]])) stop("string variables not allowed in varlist;\n", v, " is a string variable")
  }
  pv <- if (is.null(panel)) NULL else d[[panel]]
  ok <- !is.na(tv)
  if (!is.null(pv)) ok <- ok & !is.na(pv)
  if (any(tv[ok] != round(tv[ok]))) stop("time variable must contain only integer values")
  g <- if (is.null(pv)) rep(1, sum(ok)) else pv[ok]
  if (anyDuplicated(data.frame(g, tv[ok]))) {
    stop(if (is.null(pv)) "repeated time values in sample" else "repeated time values within panel")
  }
  o <- if (is.null(pv)) order(tv) else order(pv, tv)
  d <- dplyr::slice(d, o)
  tv <- tv[o]
  ok <- ok[o]
  g <- if (is.null(pv)) rep(1, sum(ok)) else pv[o][ok]
  t1 <- tv[ok]
  same <- utils::head(g, -1) == utils::tail(g, -1)
  dt <- diff(t1)[same]
  if (any(dt < delta)) stop("time values with period less than delta() found")
  gaps <- sum(dt > delta)
  n <- function(v) sprintf("%.0f", v)
  span <- paste0(time, ", ", n(min(t1)), " to ", n(max(t1)),
                 if (gaps == 1) ", but with a gap" else if (gaps > 1) ", but with gaps" else "")
  unit <- paste0(stata_fmt(delta), if (delta == 1) " unit" else " units")
  if (is.null(pv)) {
    cat("Time variable: ", span, "\n        Delta: ", unit, "\n", sep = "")
  } else {
    sets <- lapply(split(t1, g), sort)
    kind <- if (all(vapply(sets, identical, TRUE, sets[[1]]))) "strongly balanced"
      else if (length(unique(lengths(sets))) == 1) "weakly balanced" else "unbalanced"
    cat("Panel variable: ", panel, " (", kind, ")\n Time variable: ", span,
        "\n         Delta: ", unit, "\n", sep = "")
  }
  invisible(d)
}"#;

/// The time-series operators on `x`: lagged by `shift` (a lead when
/// negative), then differenced `d` times, then seasonally by `s`, with
/// `time` and `panel` from `tsset`.
pub(super) const TS_HELPER: &str = r#"stata_ts <- function(x, shift, d, s, time, panel = NULL, delta = 1) {
  o <- if (is.null(panel)) order(time) else order(panel, time)
  if (any(o != seq_along(o))) stop("not sorted")
  x <- as.numeric(x)
  ok <- !is.na(time)
  if (!is.null(panel)) ok <- ok & !is.na(panel)
  key <- function(tt) {
    k <- if (is.null(panel)) as.character(tt) else paste(panel, tt, sep = "\r")
    k[!ok | is.na(tt)] <- NA
    k
  }
  here <- key(time)
  back <- function(k) x[match(key(time - k * delta), here, incomparables = NA)]
  # Stata expands the operators into one sum over x: D.F.x is F.x - x, so it
  # needs the rows at t + 1 and t, not one at t - 1
  pw <- shift
  cf <- 1
  times <- function(k) {
    all <- tapply(c(cf, -cf), c(pw, pw + k), sum)
    pw <<- as.numeric(names(all))
    cf <<- as.numeric(all)
  }
  for (i in seq_len(d)) times(1)
  if (s > 0) times(s)
  out <- 0
  for (j in seq_along(pw)) if (cf[j] != 0) out <- out + cf[j] * back(pw[j])
  out
}"#;

/// `replace x = L.x + 1 if ...` as Stata does it, row by row: `f` gives the
/// replaced column from `cur`, which its lags read; it is applied again until
/// nothing changes (each row reads only rows at earlier times, so at most
/// once a row).
pub(super) const ROWS_REPLACE_HELPER: &str = r#"stata_rows_replace <- function(f, x) {
  cur <- x
  for (i in seq_len(length(x) + 1)) {
    new <- f(cur)
    if (identical(new, cur)) break
    cur <- new
  }
  cur
}"#;

/// A chain of operators as one: the net lag (negative for a lead), the
/// number of differences and the seasonal lag (0 for none).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) struct TsOps {
    pub(super) shift: i64,
    pub(super) d: u32,
    pub(super) s: u32,
}

impl TsOps {
    fn none(self) -> bool {
        self == TsOps::default()
    }

    /// Stata's name for the chain, as e(b) shows it: `L`, `F2`, `LD`,
    /// `DS2`; empty when it does nothing.
    fn prefix(self) -> String {
        let count = |k: u64| if k == 1 { String::new() } else { k.to_string() };
        let mut p = String::new();
        if self.shift > 0 {
            p.push_str(&format!("L{}", count(self.shift.unsigned_abs())));
        } else if self.shift < 0 {
            p.push_str(&format!("F{}", count(self.shift.unsigned_abs())));
        }
        if self.d > 0 {
            p.push_str(&format!("D{}", count(u64::from(self.d))));
        }
        if self.s > 0 {
            p.push_str(&format!("S{}", count(u64::from(self.s))));
        }
        p
    }
}

/// `L.x`, `l.d.x`, `L2D.x`: the operators and the variable. `None` when the
/// word is not a variable with time-series operators, or uses a form not
/// translated (two seasonal differences).
pub(super) fn parse_ts(word: &str) -> Option<(TsOps, &str)> {
    let (ops, var) = word.rsplit_once('.')?;
    if !is_name(var) || ops.is_empty() {
        return None;
    }
    let mut t = TsOps::default();
    for part in ops.split('.') {
        if !crate::lang::expr::is_ts_ops(part) {
            return None;
        }
        let mut chars = part.chars().peekable();
        while let Some(c) = chars.next() {
            let mut digits = String::new();
            while let Some(d) = chars.next_if(char::is_ascii_digit) {
                digits.push(d);
            }
            let k: u32 = if digits.is_empty() {
                1
            } else {
                digits.parse().ok()?
            };
            match c.to_ascii_uppercase() {
                'L' => t.shift += i64::from(k),
                'F' => t.shift -= i64::from(k),
                'D' => t.d += k,
                'S' if t.s == 0 => t.s = k,
                _ => return None,
            }
        }
    }
    Some((t, var))
}

/// The canonical name of a word with operators (`LD.x`, or `x` when they
/// cancel); `None` when it has none.
pub(super) fn ts_name(word: &str) -> Option<String> {
    let (ops, var) = parse_ts(word)?;
    Some(if ops.none() {
        var.to_string()
    } else {
        format!("{}.{var}", ops.prefix())
    })
}

/// A model's words with Stata's lists of operators written out and each
/// chain in its canonical form: `L(1/2).x` is `L.x L2.x`, `L(0/1).(x y)` is
/// `x L.x y L.y`, `l.d.x` is `LD.x`. Unchanged when there are none.
pub(super) fn expand_ts(main: &str) -> Result<String, Untranslated> {
    let chars: Vec<char> = main.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        let at_start = i == 0
            || chars
                .get(i - 1)
                .is_some_and(|c| !(c.is_alphanumeric() || *c == '_' || *c == '.'));
        let op = chars.get(i).map(|c| c.to_ascii_uppercase());
        if at_start && matches!(op, Some('L' | 'F' | 'D' | 'S')) && chars.get(i + 1) == Some(&'(') {
            if let Some((counts, end)) = numlist(&chars, i + 2) {
                if chars.get(end) == Some(&'.') {
                    let (vars, after) = if chars.get(end + 1) == Some(&'(') {
                        let close = (end + 2..chars.len())
                            .find(|&k| chars.get(k) == Some(&')'))
                            .ok_or_else(|| no(format!("{main}: a ( without its )")))?;
                        let inner: String = chars
                            .get(end + 2..close)
                            .unwrap_or_default()
                            .iter()
                            .collect();
                        (
                            inner.split_whitespace().map(str::to_string).collect(),
                            close + 1,
                        )
                    } else {
                        let stop = (end + 1..chars.len())
                            .find(|&k| {
                                chars
                                    .get(k)
                                    .is_some_and(|c| !(c.is_alphanumeric() || *c == '_'))
                            })
                            .unwrap_or(chars.len());
                        (
                            vec![chars
                                .get(end + 1..stop)
                                .unwrap_or_default()
                                .iter()
                                .collect::<String>()],
                            stop,
                        )
                    };
                    let letter = op.unwrap_or('L');
                    let mut words = Vec::new();
                    for v in &vars {
                        if !is_name(v) {
                            return Err(no(format!(
                                "{v}: only variable names are translated inside a list of time-series operators"
                            )));
                        }
                        for k in &counts {
                            let w = format!("{letter}{k}.{v}");
                            words.push(ts_name(&w).unwrap_or(w));
                        }
                    }
                    out.push_str(&words.join(" "));
                    i = after;
                    continue;
                }
            }
        }
        if at_start && matches!(op, Some('L' | 'F' | 'D' | 'S')) {
            // L.(x y): one operator on a list of variables
            let stop = (i..chars.len())
                .find(|&k| chars.get(k).is_some_and(|c| *c == '.'))
                .unwrap_or(chars.len());
            let ops: String = chars.get(i..stop).unwrap_or_default().iter().collect();
            if crate::lang::expr::is_ts_ops(&ops) && chars.get(stop + 1) == Some(&'(') {
                let close = (stop + 2..chars.len())
                    .find(|&k| chars.get(k) == Some(&')'))
                    .ok_or_else(|| no(format!("{main}: a ( without its )")))?;
                let inner: String = chars
                    .get(stop + 2..close)
                    .unwrap_or_default()
                    .iter()
                    .collect();
                let mut words = Vec::new();
                for v in inner.split_whitespace() {
                    let w = format!("{ops}.{v}");
                    words.push(ts_name(&w).ok_or_else(|| {
                        no(format!(
                            "{w}: these time-series operators are not translated"
                        ))
                    })?);
                }
                out.push_str(&words.join(" "));
                i = close + 1;
                continue;
            }
        }
        out.extend(chars.get(i));
        i += 1;
    }
    // each word on its own: l.d.x is LD.x
    let words: Vec<String> = out
        .split(' ')
        .map(|w| ts_name(w).unwrap_or_else(|| w.to_string()))
        .collect();
    Ok(words.join(" "))
}

/// `1/3`, `0 2`, `2/1` up to the `)` that closes them: the counts in
/// order, and where the `)` ends.
fn numlist(chars: &[char], from: usize) -> Option<(Vec<u32>, usize)> {
    let close = (from..chars.len()).find(|&k| chars.get(k) == Some(&')'))?;
    let text: String = chars.get(from..close)?.iter().collect();
    let mut counts = Vec::new();
    for part in text.split_whitespace() {
        match part.split_once('/') {
            Some((a, b)) => {
                let (a, b): (u32, u32) = (a.parse().ok()?, b.parse().ok()?);
                if a > b {
                    return None;
                }
                counts.extend(a..=b);
            }
            None => counts.push(part.parse().ok()?),
        }
    }
    (!counts.is_empty()).then_some((counts, close + 1))
}

/// The words of a varlist or model with time-series operators, in their
/// canonical names, once each (`L.x`, `D.y`).
pub(super) fn ts_words(text: &str) -> Vec<String> {
    let mut seen = Vec::new();
    for w in text.split(|c: char| c.is_whitespace() || "()=#,+-*/^<>!&|~[]".contains(c)) {
        let w = w.strip_prefix("c.").unwrap_or(w);
        if let Some((ops, _)) = parse_ts(w) {
            if !ops.none() {
                if let Some(n) = ts_name(w) {
                    if !seen.contains(&n) {
                        seen.push(n);
                    }
                }
            }
        }
    }
    seen
}

impl Session {
    /// New data: no `tsset` or `xtset`.
    pub(super) fn forget_ts(&mut self) {
        self.panel = None;
        self.panel_time = None;
        self.ts_delta = None;
        self.ts_unknown = false;
    }

    /// The R for a variable with time-series operators (`L.x`), from the
    /// time and panel `tsset` or `xtset` gave.
    pub(super) fn ts_code(&self, word: &str) -> Result<String, Untranslated> {
        let (ops, var) = parse_ts(word).ok_or_else(|| {
            no(format!(
                "{word}: these time-series operators are not translated (two seasonal differences)"
            ))
        })?;
        let Some(time) = &self.panel_time else {
            return Err(no(if self.ts_unknown {
                format!(
                    "{word}: time-series operators need tsset, and the data were read from a file whose tsset settings are not known here; set them again with tsset"
                )
            } else {
                format!("{word}: time variable not set, use tsset varname ... (Stata: r(111))")
            }));
        };
        if self.kind_of(var) == Some(Kind::Text) {
            return Err(no(format!(
                "{word}: time-series operators on a string variable are not translated"
            )));
        }
        if ops.none() {
            return Ok(r_name(var));
        }
        Ok(format!(
            "stata_ts({}, {}, {}, {}, {}, {}, {})",
            r_name(var),
            ops.shift,
            ops.d,
            ops.s,
            r_name(time),
            self.panel
                .as_deref()
                .map(r_name)
                .unwrap_or_else(|| "NULL".into()),
            self.ts_delta.as_deref().unwrap_or("1")
        ))
    }

    /// `dplyr::mutate()`'s arguments that add the columns a varlist's
    /// operators name (`L.x = stata_ts(...)`); `None` when it has none.
    pub(super) fn ts_columns(&mut self, text: &str) -> Result<Option<String>, Untranslated> {
        let words = ts_words(text);
        if words.is_empty() {
            return Ok(None);
        }
        let mut cols = Vec::new();
        for w in &words {
            cols.push(format!("{} = {}", r_name(w), self.ts_code(w)?));
        }
        self.helpers.insert("stata_ts");
        Ok(Some(cols.join(", ")))
    }

    /// `tsset [panel] time [, delta(#) clear]`.
    pub(super) fn tsset(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(TS_OPTIONS) {
            return Err(no(format!(
                "tsset with the option {o} is not translated yet"
            )));
        }
        if opts.has("clear", 5) {
            self.forget_ts();
            return Ok(());
        }
        unit_caveat(c, out);
        let words: Vec<&str> = c.parts.main.split_whitespace().collect();
        let (panel, time) = match words.as_slice() {
            [] => match &self.panel_time {
                Some(t) => (self.panel.clone(), t.clone()),
                None => {
                    return Err(no(
                        "time variable not set, use tsset varname ... (Stata: r(111))",
                    ))
                }
            },
            [t] => (None, t.to_string()),
            [p, t] => (Some(p.to_string()), t.to_string()),
            _ => return Err(no("tsset takes a panel variable and a time variable")),
        };
        let delta = match opts.arg("delta", 1) {
            Some(d) => Some(delta_number(&d)?),
            None if words.is_empty() => self.ts_delta.clone(),
            None => None,
        };
        self.set_ts(panel.as_deref(), &time, delta, out)
    }

    /// Checks the times, sorts the data and keeps the settings (`tsset`, and
    /// `xtset` with a time).
    pub(super) fn set_ts(
        &mut self,
        panel: Option<&str>,
        time: &str,
        delta: Option<String>,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        for v in [panel, Some(time)].into_iter().flatten() {
            if !is_name(v) {
                return Err(no(format!("{v} is not a variable name")));
            }
        }
        let d = self.data()?;
        self.helpers.insert("stata_tsset");
        self.helpers.insert("stata_fmt");
        out.package("dplyr");
        out.line(format!(
            "{d} <- stata_tsset({d}, {}{}{})",
            r_text(time),
            panel
                .map(|p| format!(", {}", r_text(p)))
                .unwrap_or_default(),
            delta
                .as_deref()
                .map(|v| format!(", delta = {v}"))
                .unwrap_or_default()
        ));
        self.panel = panel.map(str::to_string);
        self.panel_time = Some(time.to_string());
        self.ts_delta = delta;
        self.ts_unknown = false;
        Ok(())
    }
}

/// The options `tsset` and `xtset` take that are translated.
pub(super) const TS_OPTIONS: &[(&str, usize)] = &[
    ("clocktime", 5),
    ("daily", 1),
    ("weekly", 1),
    ("monthly", 1),
    ("quarterly", 1),
    ("halfyearly", 1),
    ("yearly", 1),
    ("generic", 1),
    ("delta", 1),
    ("format", 1),
    ("noquery", 5),
    ("clear", 5),
];

/// A unit (`daily`, `yearly`) or `format()` changes how Stata shows the time
/// (as dates) and gives it that display format; the lags do not change.
pub(super) fn unit_caveat(c: &Command, out: &mut Out) {
    let shown = TS_OPTIONS
        .iter()
        .filter(|(u, _)| !matches!(*u, "delta" | "noquery" | "clear" | "generic"));
    if c.options
        .iter()
        .any(|o| shown.clone().any(|(u, m)| o.is(u, *m)))
    {
        out.caveat(
            "the time unit or format() makes Stata show the time as dates and gives the time \n             variable that display format; here the time shows as a number and keeps its format; \n             the lags are the same",
        );
    }
}

/// `delta(2)` as R: a positive number (units such as `delta(7 days)` are not
/// translated).
pub(super) fn delta_number(text: &str) -> Result<String, Untranslated> {
    match text.trim().parse::<f64>() {
        Ok(v) if v > 0.0 && v.is_finite() => Ok(r_number(v)),
        _ => Err(no(format!(
            "delta({}): only a positive number is translated",
            text.trim()
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chains_take_statas_names() {
        for (w, want) in [
            ("L.x", Some("L.x")),
            ("l.d.x", Some("LD.x")),
            ("d.f.x", Some("FD.x")),
            ("D.L.x", Some("LD.x")),
            ("S.L.x", Some("LS.x")),
            ("L.F.x", Some("x")),
            ("L2.L.x", Some("L3.x")),
            ("S.D.x", Some("DS.x")),
            ("F.L2.x", Some("L.x")),
            ("S2D.x", Some("DS2.x")),
            ("L.D.F.x", Some("D.x")),
            ("D2L.x", Some("LD2.x")),
            ("S1.x", Some("S.x")),
            ("F0.x", Some("x")),
            ("L1.x", Some("L.x")),
            ("i.x", None),
            ("c.x", None),
            ("S.S.x", None),
            ("x", None),
            ("L.1x", None),
        ] {
            assert_eq!(ts_name(w).as_deref(), want, "{w}");
        }
    }

    #[test]
    fn lists_of_operators_are_written_out() {
        for (main, want) in [
            ("y L(0/2).x", "y x L.x L2.x"),
            ("y L(1/2).(x y)", "y L.x L2.x L.y L2.y"),
            ("y l.x x L(1/2).w", "y L.x x L.w L2.w"),
            ("y L(1 2).x", "y L.x L2.x"),
            ("y L.(x w)", "y L.x L.w"),
            (
                "f.d.post RT d.pre l.d.pre d.f.infl",
                "FD.post RT D.pre LD.pre FD.infl",
            ),
            ("y i.g c.x", "y i.g c.x"),
            ("y x", "y x"),
        ] {
            assert_eq!(expand_ts(main).ok().as_deref(), Some(want), "{main}");
        }
        assert_eq!(
            ts_words("(x = L.z) D.w c.L.x i.g D.w"),
            vec!["L.z", "D.w", "L.x"]
        );
    }
}
