//! P2B.4: Stata's statistics commands in R.
//!
//! `summarize`, `tabulate`, `correlate` and `ttest` so far. As everywhere in
//! this crate, nothing is guessed: where R cannot do what Stata does, the
//! command is not translated and says why. One refusal worth naming:
//! `ttest, welch` uses Welch's degrees of freedom, which R's `t.test` does not
//! offer (it uses Satterthwaite's, which is Stata's `unequal`).
//!
//! RCraft's own menus have dialogs for these statistics (Summary statistics,
//! Frequency table, Cross-tabulation, Correlation, the three t-tests). This is
//! not that code and must not become it. Those dialogs follow R's conventions,
//! these follow Stata's, and the two give different numbers on purpose:
//!
//! - `summarize` reports the sample standard deviation and, with `detail`, the
//!   population skewness and kurtosis (so a normal sample's kurtosis is near
//!   3, not near 0).
//! - Its percentiles are Stata's: `x[ceil(np)]`, or the mean of the two
//!   neighbours when `np` is a whole number. That is R's `type = 2`, not R's
//!   default `type = 7` that the dialogs use.
//! - `correlate` drops a row with a missing value in any of the variables
//!   (casewise), which is R's `use = "complete.obs"`.
//! - `tabulate` leaves missing values out unless told otherwise, orders
//!   categories by the value underneath, and its `chi2` has no continuity
//!   correction.
//!
//! The crate has no dependencies (it sits below the catalog), so the R here is
//! written out in full rather than borrowed from a dialog.

use super::*;

/// `summarize`, and the statistics `correlate, means` prints.
pub(super) const SUMMARIZE_HELPER: &str = r#"stata_summarize <- function(d, detail = FALSE) {
  num <- function(v) {
    if (is.character(v) || is.factor(v)) return(rep(NA_real_, length(v)))
    u <- unclass(v)
    attributes(u) <- NULL
    if (is.numeric(u)) as.numeric(u) else rep(NA_real_, length(v))
  }
  if (length(names(d)) == 0) {
    cat("no variables\n")
    return(invisible(NULL))
  }
  pcts <- c(1, 5, 10, 25, 50, 75, 90, 95, 99)
  one <- function(n) {
    x <- num(d[[n]])
    x <- sort(x[!is.na(x)])
    k <- length(x)
    mom <- function(r) if (k > 0) mean((x - mean(x))^r) else NA_real_
    s <- data.frame(
      variable = n,
      obs = k,
      mean = if (k > 0) mean(x) else NA_real_,
      sd = if (k > 1) stats::sd(x) else NA_real_,
      min = if (k > 0) min(x) else NA_real_,
      max = if (k > 0) max(x) else NA_real_,
      stringsAsFactors = FALSE
    )
    if (!detail) return(s)
    m2 <- mom(2)
    ok <- k > 1 && is.finite(m2) && m2 > 0
    s$variance <- if (k > 1) stats::var(x) else NA_real_
    s$skewness <- if (ok) mom(3) / m2^1.5 else NA_real_
    s$kurtosis <- if (ok) mom(4) / m2^2 else NA_real_
    p <- if (k > 0) unname(stats::quantile(x, pcts / 100, type = 2)) else rep(NA_real_, length(pcts))
    for (i in seq_along(pcts)) s[[paste0("p", pcts[i])]] <- p[i]
    s
  }
  out <- do.call(rbind, lapply(names(d), one))
  if (!detail) {
    print(out, row.names = FALSE)
  } else {
    for (i in seq_len(nrow(out))) {
      r <- out[i, ]
      x <- num(d[[r$variable]])
      x <- sort(x[!is.na(x)])
      cat(r$variable, "\n", sep = "")
      cat("  obs ", r$obs, "  mean ", r$mean, "  sd ", r$sd, "  variance ", r$variance, "\n", sep = "")
      cat("  min ", r$min, "  max ", r$max, "  skewness ", r$skewness, "  kurtosis ", r$kurtosis, "\n", sep = "")
      cat("  percentiles 1 5 10 25 50 75 90 95 99: ",
          paste(unlist(r[paste0("p", pcts)]), collapse = " "), "\n", sep = "")
      cat("  smallest ", paste(utils::head(x, 4), collapse = " "),
          "   largest ", paste(utils::tail(x, 4), collapse = " "), "\n", sep = "")
    }
  }
  invisible(out)
}"#;

/// A column as `tabulate` shows it: its value labels when it has them, in the
/// order of the values underneath (which is Stata's order), the plain values
/// when it has none or `nolabel` was given.
pub(super) const TAB_VALUES_HELPER: &str = r#"stata_tab_values <- function(v, nolabel = FALSE) {
  if (is.factor(v)) return(v)
  u <- unclass(v)
  attributes(u) <- NULL
  l <- attr(v, "labels")
  if (nolabel || is.null(l) || length(l) == 0) return(u)
  vals <- sort(unique(u[!is.na(u)]))
  labs <- names(l)[match(vals, l)]
  labs[is.na(labs)] <- format(vals[is.na(labs)])
  factor(labs[match(u, vals)], levels = unique(labs))
}"#;

/// One-way and two-way `tabulate`.
pub(super) const TABULATE_HELPER: &str = r#"stata_tabulate <- function(x, y = NULL, missing = FALSE, sort = FALSE,
                            row = FALSE, column = FALSE, cell = FALSE,
                            chi2 = FALSE) {
  use_na <- if (missing) "ifany" else "no"
  if (is.null(y)) {
    t <- table(x, useNA = use_na)
    if (sort) t <- sort(t, decreasing = TRUE)
    freq <- as.integer(t)
    total <- sum(freq)
    pct <- if (total > 0) 100 * freq / total else rep(NA_real_, length(freq))
    out <- data.frame(
      value = ifelse(is.na(names(t)), ".", names(t)),
      freq = freq,
      percent = round(pct, 2),
      cum = round(cumsum(pct), 2),
      stringsAsFactors = FALSE
    )
    print(out, row.names = FALSE)
    cat("  total ", total, "\n", sep = "")
    return(invisible(out))
  }
  keep <- if (missing) rep(TRUE, length(x)) else !is.na(x) & !is.na(y)
  t <- table(x[keep], y[keep], useNA = use_na)
  print(stats::addmargins(t))
  if (row) {
    cat("\nrow percentages\n")
    print(round(100 * prop.table(t, 1), 2))
  }
  if (column) {
    cat("\ncolumn percentages\n")
    print(round(100 * prop.table(t, 2), 2))
  }
  if (cell) {
    cat("\ncell percentages\n")
    print(round(100 * prop.table(t), 2))
  }
  if (chi2) {
    r <- suppressWarnings(stats::chisq.test(t, correct = FALSE))
    cat("\nPearson chi2(", unname(r$parameter), ") = ",
        format(unname(r$statistic), digits = 6),
        "   Pr = ", format(r$p.value, digits = 4), "\n", sep = "")
    attr(t, "chi2") <- c(chi2 = unname(r$statistic), df = unname(r$parameter), p = r$p.value)
  }
  invisible(t)
}"#;

/// `correlate`: casewise, like Stata.
pub(super) const CORRELATE_HELPER: &str = r#"stata_correlate <- function(d, covariance = FALSE, means = FALSE) {
  # as Stata 19 does: a string variable is left out with a note, and only
  # when none is left is it an error
  for (n in names(d)) {
    if (is.character(d[[n]]) || is.factor(d[[n]])) {
      cat("(", n, " ignored because string variable)\n", sep = "")
      d[[n]] <- NULL
    }
  }
  if (length(d) == 0) stop("too few variables specified")
  m <- as.matrix(as.data.frame(lapply(d, function(v) {
    u <- unclass(v)
    attributes(u) <- NULL
    as.numeric(u)
  })))
  colnames(m) <- names(d)
  m <- m[stats::complete.cases(m), , drop = FALSE]
  cat("(obs=", nrow(m), ")\n", sep = "")
  if (nrow(m) < 2) {
    cat("too few rows are left once the ones with a missing value are dropped\n")
    return(invisible(NULL))
  }
  r <- if (covariance) stats::cov(m) else stats::cor(m)
  print(round(r, 4))
  if (means) stata_summarize(as.data.frame(m))
  invisible(r)
}"#;

impl Session {
    // ---------------- summarize ----------------

    pub(super) fn summarize(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "summarize")?;
        if let Some(o) = opts.other(&[("detail", 1), ("separator", 3), ("format", 1)]) {
            return Err(no(format!(
                "summarize with the option {o} is not translated yet"
            )));
        }
        if Self::quiet(c) {
            return Ok(());
        }
        let src = self.frame_for(c, out)?;
        self.helpers.insert("stata_summarize");
        let detail = opts.has("detail", 1);
        out.line(format!(
            "stata_summarize({src}{})",
            if detail { ", detail = TRUE" } else { "" }
        ));
        out.caveat("printed in R's layout, not Stata's table");
        if detail {
            out.caveat(
                "skewness and kurtosis are Stata's, over the whole sample, so a normal sample's \
                 kurtosis is near 3",
            );
            out.caveat(
                "the percentiles are Stata's rule, which is not R's default (quantile type 2)",
            );
        }
        Ok(())
    }

    // ---------------- tabulate ----------------

    pub(super) fn tabulate(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "tabulate")?;
        let vars: Vec<&str> = c.parts.main.split_whitespace().collect();
        let (x, y) =
            match vars.as_slice() {
                [x] => (*x, None),
                [x, y] => (*x, Some(*y)),
                [] => return Err(no("tabulate needs a variable")),
                _ => return Err(no(
                    "tabulate takes one or two variables (tab1 and tab2 are not translated yet)",
                )),
            };
        let known: &[(&str, usize)] = if y.is_some() {
            &[
                ("missing", 1),
                ("nolabel", 3),
                ("row", 3),
                ("column", 3),
                ("cell", 4),
                ("chi2", 4),
            ]
        } else {
            &[("missing", 1), ("nolabel", 3), ("sort", 4)]
        };
        if let Some(o) = opts.other(known) {
            return Err(no(format!(
                "tabulate with the option {o} is not translated yet"
            )));
        }
        for v in [Some(x), y].into_iter().flatten() {
            if !is_name(v) {
                return Err(no(format!("{v} is not a variable name")));
            }
        }
        if Self::quiet(c) {
            return Ok(());
        }
        let src = self.rows_for(c, out)?;
        self.helpers.insert("stata_tab_values");
        self.helpers.insert("stata_tabulate");
        let nolabel = opts.has("nolabel", 3);
        let value = |v: &str| {
            format!(
                "stata_tab_values({src}${}{})",
                r_name(v),
                if nolabel { ", nolabel = TRUE" } else { "" }
            )
        };
        let mut args = vec![value(x)];
        if let Some(y) = y {
            args.push(value(y));
        }
        for (name, min) in [
            ("missing", 1),
            ("sort", 4),
            ("row", 3),
            ("column", 3),
            ("cell", 4),
            ("chi2", 4),
        ] {
            if opts.has(name, min) {
                args.push(format!("{name} = TRUE"));
            }
        }
        out.line(format!("stata_tabulate({})", args.join(", ")));
        out.caveat("printed in R's layout, not Stata's table");
        if opts.has("chi2", 4) {
            out.caveat("Pearson's chi-squared, with no continuity correction, as Stata does it");
        }
        Ok(())
    }

    // ---------------- correlate ----------------

    pub(super) fn correlate(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "correlate")?;
        if let Some(o) = opts.other(&[("covariance", 3), ("means", 1)]) {
            return Err(no(format!(
                "correlate with the option {o} is not translated yet"
            )));
        }
        if Self::quiet(c) {
            return Ok(());
        }
        let src = self.frame_for(c, out)?;
        self.helpers.insert("stata_correlate");
        let mut args = vec![src];
        if opts.has("covariance", 3) {
            args.push("covariance = TRUE".into());
        }
        if opts.has("means", 1) {
            args.push("means = TRUE".into());
            self.helpers.insert("stata_summarize");
        }
        out.line(format!("stata_correlate({})", args.join(", ")));
        out.caveat("a row with a missing value in any of the variables is dropped, as Stata does");
        out.caveat("printed in R's layout, not Stata's table");
        Ok(())
    }

    // ---------------- ttest ----------------

    pub(super) fn ttest(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "ttest")?;
        if opts.has("welch", 5) {
            return Err(no(
                "ttest, welch asks for Welch's degrees of freedom; R's t.test offers only \
                 Satterthwaite's, which is Stata's unequal",
            ));
        }
        if let Some(o) = opts.other(&[("by", 2), ("unequal", 3), ("unpaired", 5), ("level", 3)]) {
            return Err(no(format!(
                "ttest with the option {o} is not translated yet"
            )));
        }
        if Self::quiet(c) {
            return Ok(());
        }
        let level = match opts.arg("level", 3) {
            Some(l) => {
                let n: f64 = l
                    .trim()
                    .parse()
                    .map_err(|_| no(format!("level({l}) is not a number")))?;
                if !(0.0..100.0).contains(&n) {
                    return Err(no(format!("level({l}) is not between 0 and 100")));
                }
                Some(n / 100.0)
            }
            None => None,
        };
        let equal = !opts.has("unequal", 3);
        let src = self.rows_for(c, out)?;
        let main = c.parts.main.trim().to_string();
        let group = opts.arg("by", 2);
        let mut args: Vec<String> = Vec::new();
        let mut two_sample = false;
        match main.split_once("==") {
            Some((left, right)) => {
                let (left, right) = (left.trim(), right.trim());
                if !is_name(left) {
                    return Err(no(format!("{left} is not a variable name")));
                }
                if group.is_some() {
                    return Err(no("ttest takes either == or by(), not both"));
                }
                args.push(format!("{src}${}", r_name(left)));
                if is_name(right) {
                    args.push(format!("{src}${}", r_name(right)));
                    if opts.has("unpaired", 5) {
                        two_sample = true;
                        args.push(format!("var.equal = {}", r_logical(equal)));
                    } else {
                        args.push("paired = TRUE".into());
                    }
                } else {
                    let v: f64 = right
                        .parse()
                        .map_err(|_| no(format!("{right} is neither a variable nor a number")))?;
                    if opts.has("unpaired", 5) {
                        return Err(no("ttest var == number has nothing to unpair"));
                    }
                    args.push(format!("mu = {}", r_number(v)));
                }
            }
            None => {
                let v = main.as_str();
                if !is_name(v) {
                    return Err(no(format!("{v} is not a variable name")));
                }
                let Some(g) = group else {
                    return Err(no("ttest of one variable needs == or by()"));
                };
                let g = g.trim();
                if !is_name(g) {
                    return Err(no(format!("by({g}) is not a variable name")));
                }
                two_sample = true;
                args.push(format!("{} ~ {}", r_name(v), r_name(g)));
                args.push(format!("data = {src}"));
                args.push(format!("var.equal = {}", r_logical(equal)));
            }
        }
        if let Some(l) = level {
            args.push(format!("conf.level = {}", r_number(l)));
        }
        out.line(format!("print(stats::t.test({}))", args.join(", ")));
        out.caveat("R prints the two-sided p-value; Stata prints the one-sided ones beside it");
        if two_sample && equal {
            out.caveat(
                "the groups are taken to have the same variance, as Stata does without unequal",
            );
        }
        out.caveat("printed in R's layout, not Stata's table");
        Ok(())
    }

    // ---------------- shared ----------------

    pub(super) fn no_by(&self, c: &Command, name: &str) -> Result<(), Untranslated> {
        if c.prefixes.iter().any(|p| matches!(p, Prefix::By { .. })) {
            return Err(no(format!("by with {name} is not translated yet")));
        }
        Ok(())
    }

    /// The data frame a statistics command works on: the rows an `if`/`in`
    /// keeps, with every column.
    pub(super) fn rows_for(&mut self, c: &Command, out: &mut Out) -> Result<String, Untranslated> {
        let d = self.data()?;
        match self.condition(c, out)? {
            Some(cond) => {
                out.package("dplyr");
                Ok(format!("dplyr::filter({d}, {cond})"))
            }
            None => Ok(d),
        }
    }

    /// The same, with the varlist's columns chosen (all of them when the
    /// command names none).
    fn frame_for(&mut self, c: &Command, out: &mut Out) -> Result<String, Untranslated> {
        let rows = self.rows_for(c, out)?;
        if c.parts.main.trim().is_empty() {
            return Ok(rows);
        }
        out.package("dplyr");
        Ok(format!("dplyr::select({rows}, {})", select(&c.parts.main)?))
    }
}

pub(super) fn r_logical(b: bool) -> &'static str {
    if b {
        "TRUE"
    } else {
        "FALSE"
    }
}
