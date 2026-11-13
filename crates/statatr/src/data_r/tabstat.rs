//! `tabstat varlist [if] [in] [fw aw], statistics() by() columns() ...`:
//! Stata 19's table of summary statistics and the r() `save` leaves,
//! observed with made-up data (`plan/stata/tabstat.md`).
//!
//! The statistics are `summarize`'s (the same weights, Stata's percentile
//! rule). The table is Stata's: 10-wide columns in `%9.0g` (or `format()`),
//! a stub of 8 for the statistics ("Stats") and of 12 for the variables
//! ("Variable", `varwidth()`), the group's stub as wide as its value labels
//! or values (at least 6, at most `labelwidth()`, 16, and never narrower
//! than the group variable's name), rules between groups that take more than
//! one row, and the table cut into blocks of columns at the line size.

use super::weights::{w_args, W};
use super::*;

pub(super) const TABSTAT_HELPER: &str = r##"stata_tabstat <- function(d, vars, stats, labs, by = NULL, w = NULL, wtype = "f", cols = "",
                          fmt = "%9.0g", lw = 16, vw = 12, total = TRUE, missing = FALSE, sep = TRUE,
                          longstub = FALSE, casewise = FALSE, save = FALSE, show = TRUE, full = d,
                          defaults = character(0)) {
  num <- function(v) { u <- unclass(v); attributes(u) <- NULL; as.numeric(u) }
  for (v in c(vars, by)) if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
  for (v in vars) if (is.character(d[[v]]) || is.factor(d[[v]]))
    stop("string variables not allowed in varlist; ", v, " is a string variable (Stata: r(109))", call. = FALSE)
  weighted <- !is.null(w)
  wt <- if (weighted) num(w) else rep(1, nrow(d))
  X <- lapply(vars, function(v) num(d[[v]]))
  keep <- rep(TRUE, nrow(d))
  gv <- NULL
  if (!is.null(by)) {
    gv <- d[[by]]
    # the labels and the width from all the data (weighted rows lose attributes)
    ga <- full[[by]]
    miss <- if (is.character(gv)) is.na(gv) | gv == "" else is.na(num(gv))
    if (!missing) keep <- keep & !miss
  }
  if (casewise) for (x in X) keep <- keep & !is.na(x)
  if (!any(keep)) stop("no observations (Stata: r(2000))", call. = FALSE)
  pct <- function(x, v, p) {
    if (!weighted) return(unname(stats::quantile(x, p, type = 2)))
    W <- cumsum(v)
    P <- sum(v) * p
    i <- which(W >= P)[1]
    if (W[i] == P && i < length(x)) (x[i] + x[i + 1]) / 2 else x[i]
  }
  one <- function(x, v) {
    ok <- !is.na(x)
    x <- x[ok]
    v <- v[ok]
    o <- order(x)
    x <- x[o]
    v <- v[o]
    k <- length(x)
    W <- sum(v)
    N <- if (weighted && wtype == "f") W else k
    out <- rep(NA_real_, length(stats))
    if (k == 0) {
      out[stats %in% c("N", "sum")] <- 0
      return(out)
    }
    m <- sum(v * x) / W
    ss <- sum(v * (x - m)^2)
    var <- if (N <= 1) NA_real_ else if (!weighted || wtype == "f") ss / (N - 1) else ss / W * k / (k - 1)
    m2 <- ss / W
    mom <- function(r) sum(v * (x - m)^r) / W
    higher <- k > 1 && is.finite(m2) && m2 > 0
    q <- function(p) pct(x, v, p / 100)
    for (i in seq_along(stats)) out[i] <- switch(s <- stats[i],
      mean = m, N = N, sum = sum(v * x), max = max(x), min = min(x), range = max(x) - min(x),
      sd = sqrt(var), variance = var, cv = sqrt(var) / m, semean = sqrt(var) / sqrt(N),
      skewness = if (higher) mom(3) / m2^1.5 else NA_real_,
      kurtosis = if (higher) mom(4) / m2^2 else NA_real_,
      iqr = q(75) - q(25),
      q(as.numeric(substring(s, 2))))
    out
  }
  table_of <- function(rows) {
    m <- vapply(X, function(x) one(x[rows], wt[rows]), numeric(length(stats)))
    m <- matrix(m, nrow = length(stats), dimnames = list(labs, vars))
    m
  }
  # the groups, in sorted order, with the text Stata shows for each
  groups <- list()
  if (!is.null(by)) {
    g <- if (is.character(gv)) ifelse(is.na(gv), "", gv) else num(gv)
    lv <- sort(unique(g[keep]), method = "radix", na.last = TRUE)
    vl <- attr(ga, "labels", exact = TRUE)
    name <- function(v) {
      if (is.character(v)) return(v)
      if (is.na(v)) return(stata_fmt(v, "%9.0g", trim = TRUE))
      k <- if (!is.null(vl)) names(vl)[match(v, num(vl))] else NA
      if (length(k) && !is.na(k)) k else stata_fmt(v, "%9.0g", trim = TRUE)
    }
    for (v in lv) {
      rows <- keep & (if (is.na(v)) is.na(g) else !is.na(g) & g == v)
      groups[[length(groups) + 1]] <- list(name = name(v), m = table_of(rows))
    }
  }
  tot <- table_of(keep)
  r <- list()
  if (save) {
    for (i in rev(seq_along(groups))) r[[paste0("name", i)]] <- groups[[i]]$name
    for (i in rev(seq_along(groups))) r[[paste0("Stat", i)]] <- groups[[i]]$m
    if (is.null(by) || total) r[["StatTotal"]] <- tot
  }
  if (!show) return(r)
  ab <- function(s, n) stata_tab_abbrev(s, n)
  statcols <- if (cols == "s") TRUE else if (cols == "v") FALSE else length(vars) == 1
  heads <- if (statcols) labs else vapply(vars, ab, "", n = 8)
  # each group's rows: one a variable (statistics across) or one a statistic
  # each variable's display format: format() for all, or (format alone, fmt "") its own
  # (a format set by format or read with the data, else the storage type's default from generate)
  fmts <- if (nzchar(fmt)) rep(fmt, length(vars)) else vapply(vars, function(v) {
    f <- attr(d[[v]], "format.stata", exact = TRUE)
    if (is.null(f) && v %in% names(defaults)) f <- defaults[[v]]
    if (is.null(f)) stop("tabstat, format: the display format of ", v, " is not known here (one set by format, ",
                         "made by generate or read with the data is); give format(%fmt)", call. = FALSE)
    f
  }, "")
  # the columns are one wider than the widest format, at least 10 (Stata 19)
  fw <- suppressWarnings(as.integer(sub("^%-?0?([0-9]+).*$", "\\1", fmts)))
  colw <- max(10, 1 + max(fw, na.rm = TRUE))
  rows_of <- function(m) {
    if (statcols) lapply(seq_along(vars), function(j) list(name = ab(vars[j], vw), v = m[, j], f = rep(fmts[j], nrow(m))))
    else lapply(seq_along(labs), function(i) list(name = labs[i], v = m[i, ], f = fmts))
  }
  cell <- function(x, f) sprintf("%*s", colw, stata_fmt(x, f))
  iw <- if (statcols) vw else 8
  inner <- if (statcols) "Variable" else "Stats"
  ls <- if (exists("stata_linesize")) stata_linesize else 79
  blocks <- list()
  if (is.null(by)) {
    sw <- iw
    head_stub <- sprintf("%*s", iw, inner)
    body <- list(list(rows = rows_of(tot), label = ""))
    stub <- function(lab, name, first) sprintf("%*s", iw, name)
    many <- FALSE
  } else {
    w0 <- if (is.character(gv)) {
      f <- attr(ga, "format.stata", exact = TRUE)
      if (!is.null(f) && grepl("^%-?[0-9]+s$", f)) as.integer(gsub("[^0-9]", "", f))
      else max(c(1, nchar(ga[!is.na(ga)])))
    } else {
      vl <- attr(ga, "labels", exact = TRUE)
      if (length(vl)) max(nchar(names(vl))) else 8
    }
    gw <- max(min(lw, w0), 6, nchar(by))
    body <- lapply(groups, function(g) list(rows = rows_of(g$m), label = substr(g$name, 1, gw)))
    if (total) body[[length(body) + 1]] <- list(rows = rows_of(tot), label = "Total", total = TRUE)
    if (longstub) {
      sw <- gw + 1 + iw
      head_stub <- sprintf("%-*s %*s", gw, by, iw, inner)
      stub <- function(lab, name, first) sprintf("%-*s %*s", gw, if (first) lab else "", iw, name)
    } else {
      sw <- gw
      head_stub <- sprintf("%*s", gw, by)
      stub <- function(lab, name, first) sprintf("%*s", gw, if (first) lab else "")
      cat("\n")
      if (statcols) cat("Summary for variables: ", paste(vars, collapse = " "), "\n", sep = "")
      else cat("Summary statistics: ", paste(labs, collapse = ", "), "\n", sep = "")
      vlab <- attr(ga, "label", exact = TRUE)
      cat("Group variable: ", by, " ", if (!is.null(vlab) && nzchar(vlab)) paste0("(", vlab, ")") else "", "\n", sep = "")
    }
    many <- length(body[[1]]$rows) > 1
  }
  n <- length(heads)
  per <- max(1, floor((ls - sw - 2) / colw))
  for (from in seq(1, n, by = per)) {
    js <- from:min(n, from + per - 1)
    k <- length(js)
    rule <- paste0(strrep("-", sw + 1), "+", strrep("-", colw * k))
    cat("\n", head_stub, " |", paste0(sprintf("%*s", colw, heads[js]), collapse = ""), "\n", rule, "\n", sep = "")
    for (b in seq_along(body)) {
      g <- body[[b]]
      if (b > 1 && (isTRUE(g$total) || (many && sep))) cat(rule, "\n", sep = "")
      for (i in seq_along(g$rows)) {
        rw <- g$rows[[i]]
        cat(stub(g$label, rw$name, i == 1), " |", paste0(mapply(cell, rw$v[js], rw$f[js]), collapse = ""), "\n", sep = "")
      }
    }
    cat(strrep("-", sw + 2 + colw * k), "\n", sep = "")
  }
  invisible(r)
}"##;

/// Each statistic `tabstat` takes, by the shortest abbreviation Stata 19
/// accepts, with its code in R and its label in the table.
const STATS: &[(&str, usize, &str, &str)] = &[
    ("mean", 2, "mean", "Mean"),
    ("count", 2, "N", "N"),
    ("n", 1, "N", "N"),
    ("sum", 2, "sum", "Sum"),
    ("max", 2, "max", "Max"),
    ("min", 2, "min", "Min"),
    ("range", 1, "range", "Range"),
    ("sd", 2, "sd", "SD"),
    ("variance", 1, "variance", "Variance"),
    ("cv", 2, "cv", "CV"),
    ("semean", 3, "semean", "se(mean)"),
    ("skewness", 2, "skewness", "Skewness"),
    ("kurtosis", 1, "kurtosis", "Kurtosis"),
    ("median", 3, "p50", "p50"),
    ("iqr", 3, "iqr", "IQR"),
];

const PERCENTILES: &[&str] = &["p1", "p5", "p10", "p25", "p50", "p75", "p90", "p95", "p99"];

/// The statistics a `statistics()` list names, as (code, label), or the
/// word Stata does not know.
fn statistics(list: &str) -> Result<Vec<(&'static str, &'static str)>, String> {
    let mut out = Vec::new();
    for w in list.split_whitespace() {
        let w = w.to_ascii_lowercase();
        if w == "q" {
            out.extend([("p25", "p25"), ("p50", "p50"), ("p75", "p75")]);
        } else if let Some(p) = PERCENTILES.iter().find(|p| **p == w) {
            out.push((p, p));
        } else if let Some((_, _, code, label)) = STATS
            .iter()
            .find(|(full, min, _, _)| w.len() >= *min && full.starts_with(w.as_str()))
        {
            out.push((code, label));
        } else {
            return Err(w);
        }
    }
    Ok(out)
}

/// A width option (`labelwidth()`, `varwidth()`): Stata takes 8 to 32 and
/// puts 8 in for less, with a note.
fn width(
    opts: &Opts,
    name: &str,
    min: usize,
    default: u32,
    out: &mut Out,
) -> Result<u32, Untranslated> {
    let Some(a) = opts.arg(name, min) else {
        return Ok(default);
    };
    let v: i64 = a
        .trim()
        .parse()
        .map_err(|_| no(format!("{name}({a}) needs a whole number")))?;
    if v > 32 {
        return Err(no(format!(
            "{name}({a}): widths above 32 are not translated yet"
        )));
    }
    if v < 8 {
        out.line(format!(
            "cat(\"(option {name}() outside valid range 8..32; 8 assumed)\\n\")"
        ));
        return Ok(8);
    }
    u32::try_from(v).map_err(|_| no(format!("{name}({a})")))
}

impl Session {
    pub(super) fn tabstat(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "tabstat")?;
        if let Some(o) = opts.other(&[
            ("by", 2),
            ("statistics", 1),
            ("stats", 5),
            ("columns", 1),
            ("format", 1),
            ("labelwidth", 2),
            ("varwidth", 2),
            ("casewise", 4),
            ("nototal", 3),
            ("missing", 1),
            ("noseparator", 5),
            ("longstub", 2),
            ("save", 4),
        ]) {
            return Err(no(format!(
                "tabstat with the option {o} is not translated yet"
            )));
        }
        let vars: Vec<String> = c
            .parts
            .main
            .split_whitespace()
            .map(str::to_string)
            .collect();
        if vars.is_empty() {
            return Err(no("tabstat needs a varlist (Stata: varlist required)"));
        }
        if let Some(v) = vars.iter().find(|v| !is_name(v)) {
            return Err(no(format!(
                "tabstat {v}: patterns and abbreviations are taken once the data's variables are known"
            )));
        }
        let by = match opts.arg("by", 2).map(|b| b.trim().to_string()) {
            None => None,
            Some(b) if is_name(&b) => Some(b),
            Some(b) => return Err(no(format!("tabstat, by({b}): one variable is translated"))),
        };
        let stats = match opts.arg("statistics", 1).or_else(|| opts.arg("stats", 5)) {
            None => vec![("mean", "Mean")],
            Some(s) => {
                statistics(&s).map_err(|w| no(format!("unknown statistic: {w} (Stata: r(198))")))?
            }
        };
        if stats.is_empty() {
            return Err(no("tabstat, statistics() needs a statistic"));
        }
        let cols = match opts
            .arg("columns", 1)
            .map(|a| a.trim().to_ascii_lowercase())
        {
            None => "",
            Some(a) if !a.is_empty() && "statistics".starts_with(a.as_str()) => "s",
            Some(a) if !a.is_empty() && "variables".starts_with(a.as_str()) => "v",
            Some(a) => {
                return Err(no(format!(
                    "columns({a}): Stata takes variables or statistics"
                )))
            }
        };
        let fmt = if opts.has("format", 1) {
            match opts.raw("format", 1) {
                // format alone: each variable's own display format
                None => String::new(),
                Some(f) => {
                    let f = f.trim().to_string();
                    if !number_format(&f) {
                        return Err(no(format!(
                            "tabstat, format({f}): formats %w.dg, %w.df and %w.de (with - and c) up to 30 wide are translated"
                        )));
                    }
                    f
                }
            }
        } else {
            "%9.0g".to_string()
        };
        let lw = width(opts, "labelwidth", 2, 16, out)?;
        let vw = width(opts, "varwidth", 2, 12, out)?;
        // varwidth() with by() gives the long stub (Stata 19)
        let longstub = opts.has("longstub", 2) || (by.is_some() && opts.has("varwidth", 2));
        let w = self.weight_of(c, &[W::F, W::A])?;
        let full = self.data()?;
        let mut rows = self.rows_for(c, out)?;
        if w.is_some() {
            rows = self.weighted_rows(rows, &w);
            out.line(format!("stata_rows <- {rows}"));
            rows = "stata_rows".into();
            out.caveat(super::weights::weight_caveat(&w));
        }
        let wargs = w_args(&rows, &w);
        for h in [
            "stata_tabstat",
            "stata_fmt",
            "stata_returned",
            "stata_tables",
        ] {
            self.helpers.insert(h);
        }
        let list = |v: &[&str]| -> String {
            format!(
                "c({})",
                v.iter().map(|s| r_text(s)).collect::<Vec<_>>().join(", ")
            )
        };
        let names: Vec<&str> = vars.iter().map(String::as_str).collect();
        let codes: Vec<&str> = stats.iter().map(|s| s.0).collect();
        let labels: Vec<&str> = stats.iter().map(|s| s.1).collect();
        let mut args = format!(
            "{rows}, {}, {}, {}",
            list(&names),
            list(&codes),
            list(&labels)
        );
        if let Some(b) = &by {
            args.push_str(&format!(", by = {}", r_text(b)));
        }
        args.push_str(&wargs);
        if by.is_some() {
            args.push_str(&format!(", full = {full}"));
        }
        if !cols.is_empty() {
            args.push_str(&format!(", cols = {}", r_text(cols)));
        }
        if fmt != "%9.0g" {
            args.push_str(&format!(", fmt = {}", r_text(&fmt)));
        }
        if fmt.is_empty() {
            // the display formats Stata gives the storage types generate made
            let defaults: Vec<String> = vars
                .iter()
                .filter_map(|v| {
                    let f = match self.storage.get(v.as_str())? {
                        super::Storage::Float => "%9.0g",
                        super::Storage::Double => "%10.0g",
                        super::Storage::Long => "%12.0g",
                        super::Storage::Int | super::Storage::Byte => "%8.0g",
                        super::Storage::Str => return None,
                    };
                    Some(format!("{} = {}", r_text(v), r_text(f)))
                })
                .collect();
            if !defaults.is_empty() {
                args.push_str(&format!(", defaults = c({})", defaults.join(", ")));
            }
        }
        if lw != 16 {
            args.push_str(&format!(", lw = {lw}"));
        }
        if vw != 12 {
            args.push_str(&format!(", vw = {vw}"));
        }
        for (flag, on) in [
            ("total = FALSE", opts.has("nototal", 3)),
            ("missing = TRUE", opts.has("missing", 1)),
            ("sep = FALSE", opts.has("noseparator", 5)),
            ("longstub = TRUE", longstub),
            ("casewise = TRUE", opts.has("casewise", 4)),
            ("save = TRUE", opts.has("save", 4)),
            ("show = FALSE", self.quiet(c)),
        ] {
            if on {
                args.push_str(", ");
                args.push_str(flag);
            }
        }
        // Stata clears r() and, with save, leaves the matrices and the groups' names
        out.line(format!("stata_r <- stata_tabstat({args})"));
        if let Some(b) = &by {
            if self.kind_of(b) == Some(Kind::Text) {
                out.caveat(
                    "the group's column is as wide as its longest value; Stata's is the string's \
                     storage width (str#), which is wider if the variable was made wider",
                );
            }
        }
        self.r_new = Some((&[], "tabstat".into()));
        Ok(())
    }
}

/// `%w.dg`, `%w.df` or `%w.de` (with `-` and `c`), at most 30 wide: tabstat's columns are one
/// wider than the format, at least 10 (Stata 19).
fn number_format(f: &str) -> bool {
    let Some(body) = f
        .strip_prefix('%')
        .map(|b| b.strip_prefix('-').unwrap_or(b))
    else {
        return false;
    };
    let body = body.strip_suffix('c').unwrap_or(body);
    let Some(kind) = body.chars().last() else {
        return false;
    };
    if !matches!(kind, 'g' | 'f' | 'e') {
        return false;
    }
    let digits = &body[..body.len() - 1];
    let Some((w, d)) = digits.split_once('.') else {
        return false;
    };
    let ok = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
    ok(w) && ok(d) && w.parse::<u32>().is_ok_and(|w| (1..=30).contains(&w))
}

/// r() after `tabstat, save`: the matrices (`StatTotal`, `Stat1`, ...) and
/// the groups' names (`name1`, ...), which are text.
pub(super) fn tabstat_matrix(name: &str) -> bool {
    name == "StatTotal" || numbered(name, "Stat")
}

pub(super) fn tabstat_text(name: &str) -> bool {
    numbered(name, "name")
}

fn numbered(name: &str, stem: &str) -> bool {
    name.strip_prefix(stem).is_some_and(|n| {
        !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()) && !n.starts_with('0')
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statistics_are_named_as_stata_abbreviates_them() {
        let s =
            statistics("me co n su ma mi r sd v cv sem sk k med iqr q p1 p99").unwrap_or_default();
        let codes: Vec<&str> = s.iter().map(|s| s.0).collect();
        assert_eq!(
            codes,
            [
                "mean", "N", "N", "sum", "max", "min", "range", "sd", "variance", "cv", "semean",
                "skewness", "kurtosis", "p50", "iqr", "p25", "p50", "p75", "p1", "p99"
            ]
        );
        for bad in ["s", "c", "se", "p", "i", "iq", "p2", "p33", "p100", "foo"] {
            assert_eq!(statistics(bad), Err(bad.to_string()), "{bad}");
        }
    }

    #[test]
    fn formats_that_fit_the_columns() {
        for f in [
            "%9.2f", "%8.3g", "%9.0gc", "%6.2e", "%12.4f", "%-9.2f", "%14.2fc",
        ] {
            assert!(number_format(f), "{f}");
        }
        for f in ["%31.2f", "%9s", "9.2f", "%9.f", "%td", "%--9.2f"] {
            assert!(!number_format(f), "{f}");
        }
        assert!(tabstat_matrix("StatTotal") && tabstat_matrix("Stat12"));
        assert!(!tabstat_matrix("Stat0") && !tabstat_matrix("Stat"));
        assert!(tabstat_text("name3") && !tabstat_text("names"));
    }
}
