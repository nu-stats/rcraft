//! `winsor2` (Yujun Lian's add-on from SSC), as observed in Stata 19 on made-up
//! data (`plan/stata/winsor2.md`); the add-on's program was not read.
//!
//! `winsor2 varlist [if] [in] [, cuts(# #) suffix(str) replace trim by(varlist)
//! label]`: the values below the lower cut and above the upper cut become the
//! cuts (or missing with `trim`), the cuts being the percentiles `_pctile`
//! gives (Stata's own definition) among the nonmissing rows of `if`/`in`, by
//! group with `by()`. Without `replace` a new variable `x_w` (`x_tr` with
//! `trim`) of the old one's storage type; a 0 or a 100 leaves that end alone.

use super::*;

pub(super) const WINSOR2_HELPER: &str = r##"stata_winsor2 <- function(d, vars, cuts, trim = FALSE, replace = FALSE, suffix = "_w", by = character(),
                           label = FALSE, rows = NULL, stores = list()) {
  for (v in vars) if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
  for (b in by) if (!b %in% names(d)) stop("variable ", b, " not found\n(error in option by()) (Stata: r(111))", call. = FALSE)
  news <- if (replace) vars else paste0(vars, suffix)
  if (!replace) for (i in seq_along(vars)) if (news[i] %in% names(d))
    stop("variable ", news[i], " already exist, re-specify option -suffix()-\nSuffix ", suffix, " is invalid for ", vars[i], " (Stata: r(111))", call. = FALSE)
  for (v in vars) if (is.character(d[[v]])) stop("type mismatch (Stata: r(109))", call. = FALSE)
  lo <- cuts[1]
  hi <- cuts[2]
  n <- nrow(d)
  use <- if (is.null(rows)) rep(TRUE, n) else (!is.na(rows) & rows)
  grp <- if (length(by)) do.call(paste, c(lapply(by, function(b) { x <- unclass(d[[b]]); ifelse(is.na(x), "\001NA", as.character(x)) }), sep = "\002")) else rep("", n)
  # Stata's own percentile: the first value whose running count reaches n p / 100, the mean of it and
  # the next one when the count is exactly that
  pct <- function(s, p) {
    m <- length(s)
    P <- m * p / 100
    tol <- 1e-11 * max(1, abs(P))
    i <- max(1, min(m, ceiling(P - tol)))
    if (abs(i - P) <= tol && i < m) (s[i] + s[i + 1]) / 2 else s[i]
  }
  cl <- function(x) if (x == 0) "00" else sprintf("%.16g", x)
  for (k in seq_along(vars)) {
    v <- vars[k]
    orig <- d[[v]]
    y <- as.numeric(unclass(orig))
    res <- if (replace) y else rep(NA_real_, n)
    for (g in unique(grp[use])) {
      idx <- which(use & grp == g)
      ok <- idx[!is.na(y[idx])]
      if (!length(ok)) next
      s <- sort(y[ok])
      l <- if (lo > 0) pct(s, lo) else -Inf
      h <- if (hi < 100) pct(s, hi) else Inf
      if (trim) {
        z <- y[idx]
        z[!is.na(z) & (z < l | z > h)] <- NA
        res[idx] <- z
      } else res[idx] <- pmin(pmax(y[idx], l), h)
    }
    st <- stores[[v]]
    # a winsorized new variable made by group is a float whatever the old type (Stata 19)
    if (!replace && !trim && length(by)) st <- "float"
    lab0 <- attr(orig, "label", exact = TRUE)
    base <- if (is.null(lab0) || !nzchar(lab0)) v else lab0
    lab <- if (label) paste0(base, "-", if (trim) "Trim" else "Winsor", "(p", cl(lo), ",p", cl(hi), ")") else base
    if (replace) {
      if (!is.null(st) && st %in% c("byte", "int", "long") && any(!is.na(res) & res != trunc(res))) {
        if (st == "long") stop("winsor2, replace of a long variable with a cut that is not a whole number is not translated yet", call. = FALSE)
        res <- stata_float(res)
      } else if (identical(st, "float")) res <- stata_float(res)
      out <- res
      attributes(out) <- attributes(orig)
      attr(out, "label") <- lab
    } else {
      out <- switch(if (is.null(st)) "" else st, byte = , int = , long = trunc(res), float = stata_float(res), res)
      fmt <- switch(if (is.null(st)) "" else st, byte = , int = "%8.0g", long = "%12.0g", float = "%9.0g", double = "%10.0g", NULL)
      attributes(out) <- NULL
      attr(out, "label") <- lab
      if (!is.null(fmt)) attr(out, "format.stata") <- fmt
    }
    d[[news[k]]] <- out
  }
  d
}"##;

impl Session {
    /// `winsor2 varlist [if] [in], [cuts(# #) suffix() replace trim by() label]`.
    pub(super) fn winsor2(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[
            ("cuts", 4),
            ("suffix", 6),
            ("replace", 7),
            ("trim", 4),
            ("by", 2),
            ("label", 5),
            ("nolabel", 7),
        ]) {
            return Err(no(format!(
                "winsor2 with the option {o} is not translated yet"
            )));
        }
        let stop = |out: &mut Out, m: &str| {
            out.line(format!("stop({}, call. = FALSE)", r_text(m)));
        };
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
        if vars.is_empty() {
            stop(out, "varlist required (Stata: r(100))");
            return Ok(());
        }
        if let Some(v) = vars.iter().find(|v| !is_name(v)) {
            return Err(no(format!(
                "winsor2 {v}: names are translated (patterns and ranges are when the do-file runs)"
            )));
        }
        let replace = opts.has("replace", 7);
        let trim = opts.has("trim", 4);
        if replace && opts.has("suffix", 6) {
            stop(
                out,
                "suffix() cannot be specified with replace (Stata: r(198))",
            );
            return Ok(());
        }
        let suffix = match opts.arg("suffix", 6) {
            Some(s) => s.trim().to_string(),
            None if trim => "_tr".to_string(),
            None => "_w".to_string(),
        };
        if !suffix.is_empty()
            && !suffix
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        {
            return Err(no(format!(
                "winsor2, suffix({suffix}): a name's letters are translated"
            )));
        }
        let by: Vec<String> = opts
            .arg("by", 2)
            .map(|b| b.split_whitespace().map(str::to_string).collect())
            .unwrap_or_default();
        if let Some(b) = by.iter().find(|b| !is_name(b)) {
            return Err(no(format!("winsor2, by({b}): names are translated")));
        }
        // cuts(): two numbers from 0 to 100, the lower not above the upper (1 and 99 when absent)
        let cuts_text = opts.arg("cuts", 4);
        let cuts: Vec<f64> = match &cuts_text {
            None => vec![1.0, 99.0],
            Some(t) => {
                let mut v = Vec::new();
                for w in t.split(|ch: char| ch.is_whitespace() || ch == ',') {
                    if w.is_empty() {
                        continue;
                    }
                    match w.parse::<f64>() {
                        Ok(x) if x.is_finite() => v.push(x),
                        _ => return Err(no(format!("winsor2, cuts({t}): numbers are translated"))),
                    }
                }
                v
            }
        };
        let [lo, hi] = cuts.as_slice() else {
            return match cuts.len() {
                0 | 1 => {
                    stop(
                        out,
                        "cuts() invalid -- invalid numlist has too few elements (Stata: r(122))",
                    );
                    Ok(())
                }
                _ => Err(no(
                    "winsor2 with more than two numbers in cuts() is not translated yet",
                )),
            };
        };
        let (lo, hi) = (*lo, *hi);
        if lo < 0.0 || hi < 0.0 || lo > 100.0 || hi > 100.0 {
            stop(
                out,
                "cuts() invalid -- invalid numlist has elements outside of allowed range (Stata: r(125))",
            );
            return Ok(());
        }
        if lo > hi {
            stop(out, "__000001 not found (Stata: r(111))");
            return Ok(());
        }
        if lo == 0.0 && hi == 100.0 {
            // the add-on says so and does nothing, rc 0
            out.line(format!(
                "cat({})",
                r_text(&format!(
                    "option cuts({}) is incorrect, no action taken.\n",
                    cuts_text.as_deref().unwrap_or("").trim()
                ))
            ));
            return Ok(());
        }
        if (lo > 0.0 && lo < 1.0) || (hi > 0.0 && hi < 1.0) {
            // a cut below 1 asks for a warning
            let shown = |x: f64| -> String {
                let s = crate::dofile::format::g(x, 15).trim().to_string();
                s.strip_prefix("0.").map_or(s.clone(), |r| format!(".{r}"))
            };
            out.line(format!(
                "cat({})",
                r_text(&format!(
                    "Warning: cuts(1   99) means winsor at   1th percentile and 99th percentile,\n         cuts(0.1 90) means winsor at 0.1th percentile and 90th percentile,\n         make sure cuts({} {}) you specified is what you want. \n",
                    shown(lo),
                    shown(hi)
                ))
            ));
        }
        let d = self.data()?;
        let rows = match self.condition(c, out)? {
            Some(cond) => {
                format!(", rows = dplyr::pull(dplyr::mutate({d}, stata_rows = {cond}), stata_rows)")
            }
            None => String::new(),
        };
        let stores: Vec<String> = vars
            .iter()
            .filter_map(|v| {
                self.storage.get(v).and_then(|s| {
                    let name = match s {
                        Storage::Byte => "byte",
                        Storage::Int => "int",
                        Storage::Long => "long",
                        Storage::Float => "float",
                        Storage::Double => "double",
                        Storage::Str => return None,
                    };
                    Some(format!("{} = {}", r_name(v), r_text(name)))
                })
            })
            .collect();
        self.helpers.insert("stata_winsor2");
        self.helpers.insert("stata_float");
        out.package("dplyr");
        out.line(format!(
            "{d} <- stata_winsor2({d}, c({}), c({}, {}), trim = {}, replace = {}, suffix = {}, by = {}, label = {}{rows}, stores = list({}))",
            vars.iter().map(|v| r_text(v)).collect::<Vec<_>>().join(", "),
            r_number(lo),
            r_number(hi),
            super::stats::r_logical(trim),
            super::stats::r_logical(replace),
            r_text(&suffix),
            if by.is_empty() {
                "character()".to_string()
            } else {
                format!("c({})", by.iter().map(|b| r_text(b)).collect::<Vec<_>>().join(", "))
            },
            super::stats::r_logical(opts.has("label", 5)),
            stores.join(", "),
        ));
        for v in &vars {
            if replace {
                // an integer variable whose cut is not a whole number becomes float
                if matches!(
                    self.storage.get(v),
                    Some(Storage::Byte | Storage::Int | Storage::Long)
                ) {
                    self.storage.remove(v);
                }
                continue;
            }
            let n = format!("{v}{suffix}");
            self.kinds.insert(n.clone(), Kind::Number);
            match self.storage.get(v).copied() {
                _ if !trim && !by.is_empty() => {
                    self.storage.insert(n, Storage::Float);
                }
                Some(s) if s != Storage::Str => {
                    self.storage.insert(n, s);
                }
                _ => {
                    self.storage.remove(&n);
                }
            }
        }
        Ok(())
    }
}
