//! `center` (Ben Jann's center, SSC), as observed in Stata 19 on made-up data
//! (`plan/stata/center.md`); the add-on's program was not read.
//!
//! - Each variable less its mean, divided by its standard deviation with
//!   `standardize`, over the rows of `if` and, with `casewise`, the rows
//!   with none of the variables missing; a new variable is missing on every
//!   other row; `inplace` keeps the rows outside `if` as they were.
//!   `in` is refused: there the add-on leaves the rows in range unchanged.
//! - Without `double` the mean and the standard deviation are rounded to
//!   float first: a new variable is float((x - float(m)) / float(s)), and
//!   `inplace` on a double gives (x - float(m)) / float(s) in double; with
//!   `double` neither is rounded. The standard deviation is summarize's,
//!   sqrt(sum((x - m)^2) / (n - 1)); aweights and fweights as summarize's.
//! - New variables are c_x (or `prefix()`), float (double with `double`),
//!   labelled "x's label (centered)" or "(standardized)" (none with
//!   `nolabel`); `inplace` keeps a double double and makes any other type
//!   float, and relabels the variable the same way.
//! - "(generated variables: c_x c_y)" or "(modified variables: x y)".

use super::*;

pub(super) const CENTER_HELPER: &str = r##"stata_center <- function(d, vars, rows = NULL, w = NULL, wtype = "", prefix = "c_", inplace = FALSE,
                         standardize = FALSE, casewise = FALSE, double = FALSE, replace = FALSE,
                         nolabel = FALSE, known = list(), quiet = FALSE) {
  for (v in vars) {
    if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
    if (is.character(d[[v]])) stop("string variables not allowed in varlist;\n", v, " is a string variable (Stata: r(109))", call. = FALSE)
  }
  new <- if (inplace) vars else paste0(prefix, vars)
  if (!inplace && !replace) for (v in new) if (v %in% names(d)) stop("variable ", v, " already defined (Stata: r(110))", call. = FALSE)
  within <- if (is.null(rows)) rep(TRUE, nrow(d)) else (!is.na(rows) & rows)
  use <- within
  if (!is.null(w)) use <- use & !is.na(w)
  if (casewise) for (v in vars) use <- use & !is.na(d[[v]])
  fl <- function(v) if (double) v else stata_float(v)
  floated <- character()
  for (j in seq_along(vars)) {
    v <- vars[j]
    x <- as.numeric(unclass(d[[v]]))
    ok <- use & !is.na(x)
    n <- sum(ok)
    if (is.null(w)) {
      m <- mean(x[ok])
      s <- sqrt(sum((x[ok] - m)^2) / (n - 1))
    } else {
      ww <- as.numeric(w[ok])
      m <- sum(ww * x[ok]) / sum(ww)
      s <- if (wtype == "fweight") sqrt(sum(ww * (x[ok] - m)^2) / (sum(ww) - 1)) else sqrt(sum(ww * (x[ok] - m)^2) / sum(ww) * n / (n - 1))
    }
    val <- x - fl(m)
    if (standardize) val <- val / fl(s)
    # missing on the rows within if that casewise (or a missing weight) leaves
    # out; outside if, a new variable is missing and inplace keeps the value
    val[within & !use] <- NA
    if (inplace) val[!within] <- x[!within] else val[!within] <- NA
    lab <- attr(d[[v]], "label", exact = TRUE)
    lab <- if (nolabel) NULL else paste0(if (is.null(lab) || !nzchar(lab)) v else lab, if (standardize) " (standardized)" else " (centered)")
    if (inplace) {
      # a double stays double; any other type becomes float
      dbl <- known[[v]]
      if (is.null(dbl) || is.na(dbl)) {
        dbl <- any(!is.na(x) & stata_float(x) != x)
        if (!dbl) floated <- c(floated, v)
      }
      if (!dbl) val <- stata_float(val)
      fmt <- attr(d[[v]], "format.stata", exact = TRUE)
      d[[v]] <- structure(val, label = lab, format.stata = fmt)
    } else {
      d[[new[j]]] <- structure(if (double) val else stata_float(val), label = lab,
                               format.stata = if (double) "%10.0g" else "%9.0g")
    }
  }
  if (!quiet) cat("(", if (inplace) "modified" else "generated", " variables: ", paste(new, collapse = " "), ")\n", sep = "")
  if (length(floated)) message("note: ", paste(floated, collapse = ", "), ": storage type not known here; taken as float, Stata's default, since every value fits a float")
  d
}"##;

impl Session {
    /// `center varlist [if] [in] [aw|fw] [, inplace prefix() standardize
    /// casewise double replace nolabel]`.
    pub(super) fn center(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[
            ("inplace", 3),
            ("standardize", 1),
            ("casewise", 1),
            ("double", 1),
            ("replace", 1),
            ("prefix", 3),
            ("nolabel", 3),
        ]) {
            return Err(no(format!(
                "center with the option {o} is not translated yet"
            )));
        }
        let inplace = opts.has("inplace", 3);
        let prefix = opts.arg("prefix", 3).map(|p| p.trim().to_string());
        if inplace && prefix.is_some() {
            // Stata's error, in R so that capture sees it
            out.line("stop(\"prefix() not allowed with inplace (Stata: r(198))\", call. = FALSE)");
            return Ok(());
        }
        let prefix = prefix.unwrap_or_else(|| "c_".into());
        if !is_name(&format!("{prefix}x")) {
            return Err(no(format!(
                "center, prefix({prefix}): a name's start is needed"
            )));
        }
        let vars: Vec<String> = c
            .parts
            .main
            .split_whitespace()
            .map(str::to_string)
            .collect();
        if vars.is_empty() {
            out.line("stop(\"varlist required (Stata: r(100))\", call. = FALSE)");
            return Ok(());
        }
        if let Some(v) = vars.iter().find(|v| !is_name(v)) {
            return Err(no(format!(
                "center {v}: names are translated (patterns are when the do-file runs)"
            )));
        }
        let wtype = match &c.parts.weight {
            None => None,
            Some(w) => match w.kind {
                crate::lang::syntax::WeightKind::Frequency => Some("fweight"),
                crate::lang::syntax::WeightKind::Analytic => Some("aweight"),
                _ => return Err(no("center with pweights or iweights is not translated yet")),
            },
        };
        let d = self.data()?;
        if c.parts.in_range.is_some() {
            return Err(no(
                "center with in is not translated: the add-on leaves rows inside the range unchanged and sets some outside it to missing (observed in Stata 19), which is not reproduced",
            ));
        }
        let rows = match self.condition(c, out)? {
            Some(cond) => {
                format!(", rows = dplyr::pull(dplyr::mutate({d}, stata_rows = {cond}), stata_rows)")
            }
            None => String::new(),
        };
        let w = match (&c.parts.weight, wtype) {
            (Some(w), Some(t)) => format!(
                ", w = dplyr::pull(dplyr::mutate({d}, stata_rows = {}), stata_rows), wtype = {}",
                self.expr(&w.exp, Use::Value, out)?,
                r_text(t)
            ),
            _ => String::new(),
        };
        // inplace: a double stays double, what the translator knows
        let known: Vec<String> = vars
            .iter()
            .filter_map(|v| {
                self.storage.get(v).map(|s| {
                    format!(
                        "{} = {}",
                        r_name(v),
                        if *s == Storage::Double {
                            "TRUE"
                        } else {
                            "FALSE"
                        }
                    )
                })
            })
            .collect();
        let double = opts.has("double", 1);
        self.helpers.insert("stata_center");
        self.helpers.insert("stata_float");
        out.package("dplyr");
        out.line(format!(
            "{d} <- stata_center({d}, c({}){rows}{w}, prefix = {}, inplace = {}, standardize = {}, casewise = {}, double = {}, replace = {}, nolabel = {}, known = list({}), quiet = {})",
            vars.iter().map(|v| r_text(v)).collect::<Vec<_>>().join(", "),
            r_text(&prefix),
            super::stats::r_logical(inplace),
            super::stats::r_logical(opts.has("standardize", 1)),
            super::stats::r_logical(opts.has("casewise", 1)),
            super::stats::r_logical(double),
            super::stats::r_logical(opts.has("replace", 1)),
            super::stats::r_logical(opts.has("nolabel", 3)),
            known.join(", "),
            super::stats::r_logical(self.quiet(c)),
        ));
        for v in &vars {
            if inplace {
                match self.storage.get(v).copied() {
                    Some(Storage::Double) => {}
                    Some(_) => {
                        self.storage.insert(v.clone(), Storage::Float);
                    }
                    None => {
                        self.storage.remove(v);
                    }
                }
            } else {
                let n = format!("{prefix}{v}");
                self.kinds.insert(n.clone(), Kind::Number);
                self.storage.insert(
                    n,
                    if double {
                        Storage::Double
                    } else {
                        Storage::Float
                    },
                );
            }
        }
        if inplace && vars.iter().any(|v| !self.storage.contains_key(v)) {
            out.caveat("inplace: a variable whose storage type RCraft does not know is taken as double if a value cannot be a float, else as float (Stata's default type), with a note");
        }
        Ok(())
    }
}
