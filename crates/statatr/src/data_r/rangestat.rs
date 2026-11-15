//! `rangestat` (Robert Picard, Nick Cox and Roberto Ferrer, SSC) and `nsplit`
//! (SSC), as observed in Stata 19 on made-up data (probes RS and NS,
//! 2026-11-14); the add-ons' programs were not read.
//!
//! rangestat (stat) [new=]var ... [if] [in], interval(key low high) [by(vars)]:
//! - for each row with a key, the rows whose key lies in [key + low, key + high]
//!   (`.` for low or high: no end), in the same by() group and among the rows of
//!   if/in, the row itself included; rows outside if/in or with no key get missing;
//! - a window with no rows gives missing; one whose values are all missing gives
//!   (sum) 0, (count) 0, (obs) its rows, the others missing; (count) counts the
//!   values, (obs) the rows; (sd) is the sample one, missing for one value;
//! - the new variable is var_stat or new, a double labelled "sum of x"; one in use
//!   stops: "cannot create -x_sum-; variable(s) already defined", r(110).
//!
//! nsplit var, digits(# ...) [generate(new ...)]:
//! - each value's digits cut from the left into parts of those lengths, as
//!   numbers (20060131 with 4 2 2: 2006, 1, 31); missing stays missing;
//! - the new variables are var1, var2, ... or generate()'s, labelled
//!   "Part 1 of var"; byte for one or two digits, int to four, long beyond;
//! - every value must have as many digits as the pattern adds up to, else
//!   "The pattern 4 1 is not a valid pattern for variable d." (r(459)), after
//!   "note: values of d vary in length" when they vary; "variable f contains
//!   fractional values"; new names in use: "new variable list  d1 d2 invalid".

use super::*;

pub(super) const RANGESTAT_HELPER: &str = r##"stata_rangestat <- function(d, key, low, high, specs, by = NULL, rows = NULL) {
  n <- nrow(d)
  for (s in specs) if (s$new %in% names(d)) stop("cannot create -", s$new, "-; variable(s) already defined (Stata: r(110))", call. = FALSE)
  k <- as.numeric(unclass(d[[key]]))
  use <- if (is.null(rows)) rep(TRUE, n) else (!is.na(rows) & rows)
  grp <- if (is.null(by)) rep("", n) else do.call(paste, c(lapply(by, function(v) as.character(unclass(d[[v]]))), sep = "\r"))
  vals <- lapply(specs, function(s) as.numeric(unclass(d[[s$var]])))
  res <- lapply(specs, function(s) rep(NA_real_, n))
  pool <- which(use & !is.na(k))
  for (g in unique(grp[pool])) {
    idx <- pool[grp[pool] == g]
    idx <- idx[order(k[idx])]
    kk <- k[idx]
    for (i in idx) {
      lo <- if (is.na(low)) -Inf else k[i] + low
      hi <- if (is.na(high)) Inf else k[i] + high
      j <- idx[kk >= lo & kk <= hi]
      if (!length(j)) next
      for (m in seq_along(specs)) {
        x <- vals[[m]][j]
        v <- x[!is.na(x)]
        res[[m]][i] <- switch(specs[[m]]$stat,
          sum = sum(v),
          count = length(v),
          obs = length(j),
          mean = if (length(v)) mean(v) else NA_real_,
          min = if (length(v)) min(v) else NA_real_,
          max = if (length(v)) max(v) else NA_real_,
          sd = if (length(v) > 1) stats::sd(v) else NA_real_)
      }
    }
  }
  for (m in seq_along(specs)) {
    s <- specs[[m]]
    d[[s$new]] <- structure(res[[m]], label = paste(s$stat, "of", s$var), format.stata = "%10.0g")
  }
  d
}"##;

pub(super) const NSPLIT_HELPER: &str = r##"stata_nsplit <- function(d, var, digits, new) {
  x <- as.numeric(unclass(d[[var]]))
  v <- x[!is.na(x)]
  if (any(v != floor(v))) stop("variable ", var, " contains fractional values", call. = FALSE)
  if (any(v < 0)) stop("nsplit with negative values is not translated yet", call. = FALSE)
  len <- nchar(format(v, scientific = FALSE, trim = TRUE))
  bad <- function() stop("The pattern ", paste(digits, collapse = " "), " is not a valid pattern for variable ", var, ". (Stata: r(459))", call. = FALSE)
  if (length(unique(len)) > 1) {
    cat("note: values of ", var, " vary in length\n", sep = "")
    bad()
  }
  if (length(len) && sum(digits) != len[1]) bad()
  if (any(new %in% names(d))) stop("new variable list  ", paste(new, collapse = " "), " invalid", call. = FALSE)
  ends <- cumsum(digits)
  for (i in seq_along(digits)) {
    part <- floor(x / 10^(sum(digits) - ends[i])) %% 10^digits[i]
    d[[new[i]]] <- structure(part, label = paste("Part", i, "of", var), format.stata = "%8.0g")
  }
  d
}"##;

impl Session {
    /// `rangestat (stat) [new=]var ... [if] [in], interval(key low high) [by(vars)]`.
    pub(super) fn rangestat(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[("interval", 3), ("by", 2)]) {
            return Err(no(format!(
                "rangestat with the option {o} is not translated yet"
            )));
        }
        let Some(iv) = opts.arg("interval", 3) else {
            return Err(no("rangestat needs interval(keyvar low high)"));
        };
        let parts: Vec<&str> = iv.split_whitespace().collect();
        let [key, low, high] = parts.as_slice() else {
            return Err(no("rangestat: interval() takes a variable and two bounds"));
        };
        if !is_name(key) {
            return Err(no(format!("rangestat: {key} is not a variable name")));
        }
        let bound = |b: &str| -> Result<String, Untranslated> {
            if b == "." {
                return Ok("NA_real_".to_string());
            }
            b.parse::<f64>().map(r_number).map_err(|_| {
                no(format!(
                    "rangestat: a bound that is a variable ({b}) is not translated yet"
                ))
            })
        };
        let (low, high) = (bound(low)?, bound(high)?);
        // (stat) [new=]var ...
        let mut specs = Vec::new();
        let mut news = Vec::new();
        let mut stat = String::new();
        let mut rest = c.parts.main.trim().to_string();
        while !rest.is_empty() {
            if let Some(r) = rest.strip_prefix('(') {
                let end = r
                    .find(')')
                    .ok_or_else(|| no("rangestat: a (stat) is not closed"))?;
                stat = r.get(..end).unwrap_or("").trim().to_string();
                if !matches!(
                    stat.as_str(),
                    "sum" | "count" | "obs" | "mean" | "min" | "max" | "sd"
                ) {
                    return Err(no(format!("rangestat ({stat}) is not translated yet")));
                }
                rest = r.get(end + 1..).unwrap_or("").trim().to_string();
                continue;
            }
            if stat.is_empty() {
                return Err(no("rangestat needs a (stat) before its variables"));
            }
            let (tok, after) = first_word(&rest);
            let tok = tok.to_string();
            rest = after.trim().to_string();
            let (new, var) = match tok.split_once('=') {
                Some((n, v)) if !v.is_empty() => (n.to_string(), v.to_string()),
                Some((n, _)) => {
                    let (v, after) = first_word(&rest);
                    let v = v.to_string();
                    rest = after.trim().to_string();
                    (n.to_string(), v)
                }
                None if rest.starts_with('=') => {
                    let (v, after) = first_word(rest.trim_start_matches('=').trim());
                    let v = v.to_string();
                    rest = after.trim().to_string();
                    (tok.clone(), v)
                }
                None => (format!("{tok}_{stat}"), tok.clone()),
            };
            if !is_name(&new) || !is_name(&var) {
                return Err(no(
                    "rangestat: only plain variable names are translated here",
                ));
            }
            specs.push(format!(
                "list(new = {}, stat = {}, var = {})",
                r_text(&new),
                r_text(&stat),
                r_text(&var)
            ));
            news.push(new);
        }
        if specs.is_empty() {
            return Err(no("rangestat needs a (stat) and a variable"));
        }
        let by = match opts.arg("by", 2) {
            Some(b) => {
                let vs = b.split_whitespace().map(str::to_string).collect::<Vec<_>>();
                if vs.is_empty() || !vs.iter().all(|v| is_name(v)) {
                    return Err(no("rangestat: by() takes variable names"));
                }
                format!(
                    ", by = c({})",
                    vs.iter().map(|v| r_text(v)).collect::<Vec<_>>().join(", ")
                )
            }
            None => String::new(),
        };
        let d = self.data()?;
        let rows = match self.condition(c, out)? {
            Some(cond) => {
                out.package("dplyr");
                format!(", rows = dplyr::pull(dplyr::mutate({d}, stata_rows = {cond}), stata_rows)")
            }
            None => String::new(),
        };
        self.helpers.insert("stata_rangestat");
        out.line(format!(
            "{d} <- stata_rangestat({d}, {}, {low}, {high}, list({}){by}{rows})",
            r_text(key),
            specs.join(", ")
        ));
        for n in news {
            self.kinds.insert(n.clone(), Kind::Number);
            self.storage.insert(n, Storage::Double);
        }
        Ok(())
    }

    /// `nsplit var, digits(# ...) [generate(new ...)]`.
    pub(super) fn nsplit(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[("digits", 1), ("generate", 1)]) {
            return Err(no(format!(
                "nsplit with the option {o} is not translated yet"
            )));
        }
        if c.parts.if_exp.is_some() || c.parts.in_range.is_some() {
            return Err(no("nsplit with if or in is not translated yet"));
        }
        let var = c.parts.main.trim();
        if !is_name(var) {
            return Err(no("nsplit takes one variable name"));
        }
        let Some(dg) = opts.arg("digits", 1) else {
            return Err(no("nsplit without digits() is not translated yet"));
        };
        let digits = dg
            .split_whitespace()
            .map(|w| w.parse::<u32>().ok().filter(|n| (1..=9).contains(n)))
            .collect::<Option<Vec<u32>>>()
            .filter(|v| !v.is_empty())
            .ok_or_else(|| {
                no(format!(
                    "nsplit, digits({dg}): whole numbers from 1 to 9 are translated"
                ))
            })?;
        let new: Vec<String> = match opts.arg("generate", 1) {
            Some(g) => g.split_whitespace().map(str::to_string).collect(),
            None => (1..=digits.len()).map(|i| format!("{var}{i}")).collect(),
        };
        if new.len() != digits.len() || !new.iter().all(|n| is_name(n)) {
            return Err(no("nsplit: generate() needs one new name for each part"));
        }
        let d = self.data()?;
        self.helpers.insert("stata_nsplit");
        out.line(format!(
            "{d} <- stata_nsplit({d}, {}, c({}), c({}))",
            r_text(var),
            digits
                .iter()
                .map(|n| n.to_string())
                .collect::<Vec<_>>()
                .join(", "),
            new.iter().map(|n| r_text(n)).collect::<Vec<_>>().join(", ")
        ));
        for (n, k) in new.into_iter().zip(digits) {
            self.kinds.insert(n.clone(), Kind::Number);
            let s = match k {
                1 | 2 => Storage::Byte,
                3 | 4 => Storage::Int,
                _ => Storage::Long,
            };
            self.storage.insert(n, s);
        }
        Ok(())
    }
}
