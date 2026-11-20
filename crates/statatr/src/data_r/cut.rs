//! `egen newvar = cut(exp), at(numlist) | group(#) [icodes label]` (P2C.12ao),
//! as Stata 19 does it on made-up data: `plan/stata/egen-cut.md`.

use super::*;

/// Stata's `cut()`: the value is read as a single-precision number (a double
/// 0.7 is below the cut point 0.7), then `at()` gives the left end of the bin
/// [a, b) it falls in (missing below the first point and from the last one
/// up), or its code with `icodes`; `group(k)` cuts at the smallest value and
/// the j/k percentiles of the selected rows (Stata's default percentile rule:
/// on a whole position the mean of it and the next, else the next up) and
/// gives each value the number of cut points at or below it, less one. The
/// cut points go with the result as `cuts`: for `group()` followed by the
/// largest value plus one, which the labels show.
pub(super) const CUT_HELPER: &str = r#"stata_cut <- function(x, sel, at = NULL, group = NULL, icodes = FALSE) {
  x <- stata_float(x)
  n <- length(x)
  sel <- rep_len(sel, n)
  ok <- !is.na(sel) & sel & !is.na(x)
  out <- rep(NA_real_, n)
  if (is.null(at)) {
    v <- sort(x[ok])
    m <- length(v)
    if (m == 0) stop("no observations (Stata: r(2000))", call. = FALSE)
    if (group > m + 1) stop("group() is larger than the number of observations plus one (Stata: r(198))", call. = FALSE)
    p <- m * seq_len(group - 1)
    lo <- p %/% group
    cuts <- c(v[1], ifelse(p %% group == 0, (v[pmax(lo, 1)] + v[lo + 1]) / 2, v[lo + 1]))
    out[ok] <- findInterval(x[ok], cuts) - 1
    cuts <- c(cuts, v[m] + 1)
  } else {
    i <- findInterval(x, at)
    keep <- ok & i >= 1 & i < length(at)
    out[keep] <- if (icodes) i[keep] - 1 else at[i[keep]]
    cuts <- at
  }
  attr(out, "cuts") <- cuts
  out
}
stata_cut_labels <- function(cuts) {
  stats::setNames(seq_along(cuts) - 1, paste0(vapply(cuts, function(v) sub("^ +", "", stata_fmt(v, "%9.0g")), ""), "-"))
}"#;

/// A numlist of plain numbers, `a(d)b` and `a/b`, commas as blanks.
fn cut_points(text: &str) -> Result<Vec<f64>, Untranslated> {
    let bad = || {
        no(format!(
            "egen cut() with at({}) is not translated",
            text.trim()
        ))
    };
    let text = text.replace(',', " ");
    let mut out = Vec::new();
    let num = |s: &str| s.trim().parse::<f64>().ok().filter(|v| v.is_finite());
    for tok in text.split_whitespace() {
        if let Some((a, rest)) = tok.split_once('(') {
            let (d, b) = rest.split_once(')').ok_or_else(bad)?;
            let (Some(a), Some(d), Some(b)) = (num(a), num(d), num(b)) else {
                return Err(bad());
            };
            let steps = (b - a) / d;
            if d == 0.0 || !steps.is_finite() || steps < -1e-9 || steps > 10_000.0 {
                return Err(bad());
            }
            let n = (steps + 1e-9).floor() as usize;
            out.extend((0..=n).map(|i| a + d * i as f64));
        } else if let Some((a, b)) = tok.split_once('/') {
            let (Some(a), Some(b)) = (num(a), num(b)) else {
                return Err(bad());
            };
            if b < a || b - a > 10_000.0 {
                return Err(bad());
            }
            let n = (b - a + 1e-9).floor() as usize;
            out.extend((0..=n).map(|i| a + i as f64));
        } else {
            out.push(num(tok).ok_or_else(bad)?);
        }
    }
    if out.is_empty() || out.len() > 10_000 {
        return Err(bad());
    }
    Ok(out)
}

impl Session {
    /// `egen [type] newvar = cut(exp) [if] [in], at(numlist) | group(#) [icodes label]`.
    pub(super) fn egen_cut(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
        ty: Option<&str>,
        var: &str,
        args: &str,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[
            ("at", 2),
            ("group", 5),
            ("icodes", 6),
            ("label", 5),
            ("by", 2),
        ]) {
            return Err(no(format!(
                "egen cut() with the option {o} is not translated yet"
            )));
        }
        // cut() may not be combined with by (r(190)), as a prefix or as by()
        let by = opts.has("by", 2) || c.prefixes.iter().any(|p| matches!(p, Prefix::By { .. }));
        let stop = |out: &mut Out, msg: &str| {
            out.line(format!("stop({}, call. = FALSE)", r_text(msg)));
        };
        let at_text = opts.arg("at", 2);
        let group_text = opts.arg("group", 5);
        let icodes = opts.has("icodes", 6);
        let label = opts.has("label", 5);
        // group(0) is no group() to Stata; 1, a negative or a fraction is r(198)
        let mut group_bad = false;
        let group = match &group_text {
            None => None,
            Some(g) => {
                let k: f64 = g
                    .trim()
                    .parse()
                    .map_err(|_| no(format!("egen cut() with group({g}) is not translated")))?;
                if k == 0.0 {
                    None
                } else if k.fract() == 0.0 && (2.0..=1e9).contains(&k) {
                    Some(k)
                } else {
                    group_bad = true;
                    None
                }
            }
        };
        let storage = match ty {
            None if self.type_double => Storage::Double,
            None | Some("float") => Storage::Float,
            Some("double") => Storage::Double,
            // the type asked for holds the values when they fit (whole numbers in
            // its range); fractions make a byte or int a float and a long a double
            Some("byte" | "int") => Storage::Float,
            Some("long") => Storage::Double,
            Some(t) => {
                return Err(no(format!(
                    "egen {t} newvar = cut() is not translated (float and double are)"
                )))
            }
        };
        let d = self.data()?;
        out.package("dplyr");
        let exists = format!(
            "if ({} %in% names({d})) stop(\"variable {var} already defined (Stata: r(110))\", call. = FALSE)",
            r_text(var)
        );
        let at = match (&at_text, group) {
            (None, None) | (Some(_), Some(_)) if by => {
                out.line(exists);
                stop(
                    out,
                    "egen cut() may not be combined with by (Stata: r(190))",
                );
                return Ok(());
            }
            (None, None) => {
                out.line(exists);
                stop(
                    out,
                    if group_bad {
                        "group() needs a whole number of 2 or more (Stata: r(198))"
                    } else {
                        "cut() needs one of at() or group() (Stata: r(198))"
                    },
                );
                return Ok(());
            }
            (Some(_), Some(_)) => {
                out.line(exists);
                stop(out, "at() and group() cannot both be given (Stata: r(198))");
                return Ok(());
            }
            (Some(_), None) if group_bad => {
                return Err(no("egen cut() with at() and a group() that is not a whole number of 2 or more is not translated"));
            }
            (Some(t), None) => {
                let pts = cut_points(t)?;
                if pts.windows(2).any(|w| matches!(w, [a, b] if a >= b)) {
                    out.line(exists);
                    stop(out, "at() is not in ascending order (Stata: r(124))");
                    return Ok(());
                }
                if by {
                    out.line(exists);
                    stop(
                        out,
                        "egen cut() may not be combined with by (Stata: r(190))",
                    );
                    return Ok(());
                }
                Some(pts)
            }
            (None, Some(_)) if by => {
                out.line(exists);
                stop(
                    out,
                    "egen cut() may not be combined with by (Stata: r(190))",
                );
                return Ok(());
            }
            (None, Some(_)) => None,
        };
        let cond = self.condition(c, out)?;
        let sel = cond.unwrap_or_else(|| "TRUE".into());
        let x = self.expr(args, Use::Value, out)?;
        self.helpers.insert("stata_cut");
        self.helpers.insert("stata_float");
        self.helpers.insert("stata_fmt");
        let call = match (&at, group) {
            (Some(pts), _) => {
                let p: Vec<String> = pts.iter().map(|v| r_number(*v)).collect();
                format!(
                    "stata_cut({x}, {sel}, at = c({}), icodes = {})",
                    p.join(", "),
                    if icodes || label { "TRUE" } else { "FALSE" }
                )
            }
            (None, Some(k)) => format!("stata_cut({x}, {sel}, group = {})", r_number(k)),
            (None, None) => String::new(),
        };
        out.line(exists);
        let col = r_text(var);
        if label {
            out.package("haven");
            out.line(format!(
                "{d} <- {d} |> dplyr::mutate({} = {call})",
                r_name(var)
            ));
            out.line(format!(
                "{} <- stata_cut_labels(attr({d}[[{col}]], \"cuts\"))",
                label_object(var)
            ));
            out.line(format!(
                "{d}[[{col}]] <- structure(haven::labelled(as.numeric({d}[[{col}]]), labels = {}), vallab = {col})",
                label_object(var)
            ));
            // the label's pairs are made when the data run: only its name is known here
            self.label_sets
                .insert(var.to_string(), vec![(f64::NAN, String::new())]);
        } else {
            let value = if storage == Storage::Double {
                format!("as.numeric({call})")
            } else {
                format!("stata_float({call})")
            };
            out.line(format!(
                "{d} <- {d} |> dplyr::mutate({} = {value})",
                r_name(var)
            ));
        }
        if !self.quiet(c) {
            self.helpers.insert("stata_chg");
            self.helpers.insert("stata_mv");
            out.line(format!("stata_gen_note({d}[[{col}]])"));
        }
        self.kinds.insert(var.to_string(), Kind::Number);
        if matches!(ty, Some("byte" | "int" | "long")) {
            // the type it ends as depends on the values
            self.storage.remove(var);
        } else {
            self.storage.insert(var.to_string(), storage);
        }
        Ok(())
    }
}
