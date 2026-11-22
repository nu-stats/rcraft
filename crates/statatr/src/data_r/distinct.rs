//! `distinct` (Gary Longton and Nicholas J. Cox's add-on from SSC), as observed
//! in Stata 19 on made-up data (`plan/stata/distinct.md`); the add-on's program
//! was not read.
//!
//! `distinct [varlist] [if] [in] [, minimum(#) maximum(#) joint missing
//! abbrev(#)]`: for each variable the number of rows and of distinct values
//! (missing values, and empty strings, left out unless `missing`), in a table
//! of the variables whose distinct count is within the limits; `joint` counts
//! the combinations of the varlist over the rows that have no missing value.
//! r(ndistinct) and r(N) are those of the last variable (or of the
//! combination).

use super::*;

pub(super) const DISTINCT_HELPER: &str = r##"stata_distinct <- function(d, vars = names(d), rows = NULL, joint = FALSE, missing = FALSE, lo = 0, hi = Inf, abbrev = 0) {
  for (v in vars) if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
  n <- nrow(d)
  use <- if (is.null(rows)) rep(TRUE, n) else (!is.na(rows) & rows)
  val <- function(v) {
    x <- d[[v]]
    if (is.character(x)) list(k = paste0("s", x), miss = is.na(x) | x == "")
    else { u <- unclass(x); attributes(u) <- NULL; list(k = sprintf("%.17g", u), miss = is.na(u)) }
  }
  vs <- lapply(vars, val)
  names(vs) <- vars
  if (lo > hi) {
    cat("min(", format(lo), ") max(", format(hi), ") interpreted as min(", format(hi), ") max(", format(lo), ")\n", sep = "")
    t <- lo; lo <- hi; hi <- t
  }
  count <- function(ok, k) c(total = sum(ok), distinct = length(unique(k[ok])))
  if (joint) {
    ok <- use
    if (!missing) for (v in vars) ok <- ok & !vs[[v]]$miss
    k <- do.call(paste, c(lapply(vs, function(z) z$k), sep = "\002"))
    r <- count(ok, k)
    cat("\n        Observations\n      total   distinct\n", sprintf("%11.0f%11.0f", r[1], r[2]), "\n\n", sep = "")
    return(list(ndistinct = unname(r[2]), N = unname(r[1])))
  }
  res <- lapply(vs, function(z) count(use & (missing | !z$miss), z$k))
  shown <- names(vs)[vapply(res, function(r) r[2] >= lo && r[2] <= hi, TRUE)]
  lab <- shown
  if (abbrev > 0) {
    m <- max(abbrev, 5)
    lab <- ifelse(nchar(shown) > m, paste0(substr(shown, 1, m - 2), "~", substr(shown, nchar(shown), nchar(shown))), shown)
  }
  w <- max(5, nchar(lab)) + 1
  cat("\n", strrep(" ", w + 1), "|        Observations\n", strrep(" ", w + 1), "|      total   distinct\n",
      strrep("-", w + 1), "+", strrep("-", 22), "\n", sep = "")
  for (i in seq_along(shown)) cat(formatC(lab[i], width = w), " |", sprintf("%11.0f%11.0f", res[[shown[i]]][1], res[[shown[i]]][2]), "\n", sep = "")
  cat("\n")
  last <- if (length(res)) res[[length(res)]] else c(0, 0)
  list(ndistinct = unname(last[2]), N = unname(last[1]))
}"##;

impl Session {
    /// `distinct [varlist] [if] [in] [, minimum(#) maximum(#) joint missing abbrev(#)]`.
    pub(super) fn distinct(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let stop = |out: &mut Out, m: &str| {
            out.line(format!("stop({}, call. = FALSE)", r_text(m)));
        };
        if opts.has("by", 2) {
            stop(out, "option by() not allowed (Stata: r(198))");
            return Ok(());
        }
        if let Some(o) = opts.other(&[
            ("minimum", 3),
            ("maximum", 3),
            ("joint", 5),
            ("missing", 7),
            ("abbrev", 6),
        ]) {
            return Err(no(format!(
                "distinct with the option {o} is not translated yet"
            )));
        }
        if c.parts.weight.is_some() {
            stop(out, "weights not allowed (Stata: r(101))");
            return Ok(());
        }
        let vars: Vec<String> = c
            .parts
            .main
            .split_whitespace()
            .map(str::to_string)
            .collect();
        if let Some(v) = vars.iter().find(|v| !is_name(v)) {
            return Err(no(format!(
                "distinct {v}: names are translated (patterns and ranges are when the do-file runs)"
            )));
        }
        let joint = opts.has("joint", 5);
        // with in, the add-on's later variables count the rows of the range, missing or not
        // (observed in Stata 19: 41 for a variable with one missing of 41 rows), which is not
        // reproduced
        if c.parts.in_range.is_some() && vars.len() != 1 {
            return Err(no(
                "distinct with in and no single variable is not translated: the add-on counts the later variables oddly (observed in Stata 19)",
            ));
        }
        // min(), max(): whole numbers (a negative min lets everything in, a max of 0 nothing)
        let mut limit =
            |name: &str, min: usize, option: &str| -> Result<Option<f64>, Untranslated> {
                match opts.arg(name, min) {
                    None => Ok(None),
                    Some(a) => match a.trim().parse::<i64>() {
                        Ok(v) => Ok(Some(v as f64)),
                        Err(_) => {
                            stop(
                                out,
                                &format!("option {option}() incorrectly specified (Stata: r(198))"),
                            );
                            Err(no("handled"))
                        }
                    },
                }
            };
        let (lo, hi, abbrev) = match (limit("minimum", 3, "min"), limit("maximum", 3, "max")) {
            (Ok(lo), Ok(hi)) => (lo, hi, opts.arg("abbrev", 6)),
            _ => return Ok(()),
        };
        if joint && (lo.is_some() || hi.is_some()) {
            return Err(no(
                "distinct, joint with minimum() or maximum() is not translated yet",
            ));
        }
        let abbrev = match abbrev {
            None => 0,
            Some(a) => match a.trim().parse::<u32>() {
                // below 5 Stata uses 5
                Ok(n) => n,
                Err(_) => {
                    stop(out, "option abbrev() incorrectly specified (Stata: r(198))");
                    return Ok(());
                }
            },
        };
        let d = self.data()?;
        let rows = match self.condition(c, out)? {
            Some(cond) => {
                format!(", rows = dplyr::pull(dplyr::mutate({d}, stata_rows = {cond}), stata_rows)")
            }
            None => String::new(),
        };
        out.package("dplyr");
        self.helpers.insert("stata_distinct");
        let mut args = String::new();
        if !vars.is_empty() {
            args.push_str(&format!(
                ", c({})",
                vars.iter()
                    .map(|v| r_text(v))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        out.line(format!(
            "stata_r <- stata_distinct({d}{args}{rows}, joint = {}, missing = {}, lo = {}, hi = {}, abbrev = {abbrev})",
            super::stats::r_logical(joint),
            super::stats::r_logical(opts.has("missing", 7)),
            lo.map_or("0".to_string(), r_number),
            hi.map_or("Inf".to_string(), r_number),
        ));
        self.r_new = Some((&["ndistinct", "N"], "distinct".into()));
        Ok(())
    }
}
