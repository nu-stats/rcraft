//! `table` (Stata 17+): one or two row variables, nested, and at most one
//! column variable, with frequencies or `statistic()`s, `nformat()`,
//! `nototals` and `missing`, laid out as Stata 19 prints them
//! (`plan/stata/table.md`): each column two spaces (three after the first)
//! and as wide as its widest entry, the stub as wide as its widest label and
//! one, levels indented two, the column variable's name centred over the
//! columns, a cell with no rows blank and one with no values `.`.

use super::*;

pub(super) const TABLE_HELPER: &str = r##"stata_table <- function(d, rows = character(), cols = character(), stats = list(list(s = "frequency", v = NA_character_)),
                        fmts = list(), totals = TRUE, missing = FALSE, w = NULL) {
  num <- function(u) { if (is.character(u)) return(u); u <- unclass(u); attributes(u) <- NULL; as.numeric(u) }
  for (v in c(rows, cols, unlist(lapply(stats, `[[`, "v")))) {
    if (!is.na(v) && !v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
  }
  dims <- c(rows, cols)
  keep <- rep(TRUE, nrow(d))
  gone <- function(u) if (is.character(u)) is.na(u) | u == "" else is.na(u)
  if (!missing) for (v in dims) keep <- keep & !gone(d[[v]])
  # frequency weights: the frequency is the sum of the weights (Stata 19: "Sum of weights")
  wv <- if (is.null(w)) rep(1, nrow(d)) else as.numeric(unclass(w))
  wv <- wv[keep]
  d <- d[keep, , drop = FALSE]
  head_of <- function(v) { l <- attr(d[[v]], "label", exact = TRUE); if (is.null(l) || l == "") v else l }
  levels_of <- function(v) {
    u <- num(d[[v]])
    lv <- sort(unique(u[!is.na(u)]), method = "radix")
    if (any(is.na(u))) lv <- c(lv, NA)
    vl <- attr(d[[v]], "labels", exact = TRUE)
    lab <- vapply(lv, function(x) {
      if (is.na(x)) return(".")
      if (is.character(x)) return(x)
      k <- if (!is.null(vl)) names(vl)[match(x, num(vl))] else NA
      if (length(k) && !is.na(k)) k else stata_fmt(x, "%9.0g", trim = TRUE)
    }, "")
    list(v = lv, lab = lab)
  }
  is_at <- function(v, x) { u <- num(d[[v]]); if (is.na(x)) is.na(u) else !is.na(u) & u == x }
  ord <- function(k) { s <- c("st", "nd", "rd"); if (k %% 100 %in% 11:13 || k %% 10 == 0 || k %% 10 > 3) "th" else s[k %% 10] }
  label_of <- function(s) {
    fixed <- c(frequency = if (is.null(w)) "Frequency" else "Sum of weights", percent = "Percent", mean = "Mean", sd = "Standard deviation", median = "Median",
               min = "Minimum value", max = "Maximum value", sum = "Total", count = "Number of nonmissing values",
               iqr = "Interquartile range")
    if (s %in% names(fixed)) return(fixed[[s]])
    k <- as.integer(sub("^p", "", s))
    paste0(k, ord(k), " percentile")
  }
  ntot <- nrow(d)
  value <- function(st, sel) {
    if (!any(sel)) return(NA_character_)
    s <- st$s
    if (s == "frequency") return(fmt(s, sum(wv[sel])))
    if (s == "percent") return(fmt(s, 100 * sum(sel) / ntot))
    x <- num(d[[st$v]])[sel]
    x <- x[!is.na(x)]
    if (!length(x)) return(".")
    q <- function(p) stats::quantile(x, p, type = 2, names = FALSE)
    r <- switch(s, mean = mean(x), sd = if (length(x) > 1) stats::sd(x) else NA_real_, median = q(.5), min = min(x),
                max = max(x), sum = sum(x), count = length(x), iqr = q(.75) - q(.25),
                q(as.numeric(sub("^p", "", s)) / 100))
    fmt(s, r)
  }
  fmt <- function(s, x) {
    if (is.na(x)) return(".")
    f <- fmts[[s]]
    if (is.null(f)) f <- fmts[["_all"]]
    if (is.null(f)) f <- if (s == "percent") "%9.2f" else "%9.0gc"
    stata_fmt(x, f, trim = TRUE)
  }
  # the columns: the column variable's levels (and Total), or the statistics
  multi <- length(stats) > 1
  colsel <- list()
  colhead <- character()
  if (length(cols)) {
    cl <- levels_of(cols)
    colsel <- lapply(cl$v, function(x) is_at(cols, x))
    colhead <- cl$lab
    if (totals) {
      colsel <- c(colsel, list(rep(TRUE, nrow(d))))
      colhead <- c(colhead, "Total")
    }
  }
  vars <- vapply(stats, `[[`, "", "v")
  one_stat <- length(unique(vapply(stats, `[[`, "", "s"))) == 1
  statcols <- !length(cols)
  if (statcols) {
    colhead <- if (multi && one_stat && length(unique(vars)) == length(vars)) vars else vapply(stats, function(st) label_of(st$s), "")
  }
  lines <- list()
  add <- function(lab, cells = NULL) lines[[length(lines) + 1]] <<- list(lab = lab, cells = cells)
  cells_for <- function(sel, st_list) {
    if (statcols) vapply(st_list, function(st) value(st, sel), "")
    else vapply(colsel, function(cs) value(st_list[[1]], sel & cs), "")
  }
  leaf <- function(sel, indent) {
    if (!statcols && multi) {
      for (st in stats) add(paste0(strrep(" ", indent), label_of(st$s)), cells_for(sel, list(st)))
      return(invisible())
    }
    cells_for(sel, stats)
  }
  walk <- function(k, sel, indent) {
    v <- rows[k]
    add(paste0(strrep(" ", indent), head_of(v)))
    lv <- levels_of(v)
    groups <- c(lapply(lv$v, function(x) sel & is_at(v, x)), if (totals) list(sel))
    labs <- c(lv$lab, if (totals) "Total")
    for (i in seq_along(groups)) {
      lab <- paste0(strrep(" ", indent + 2), labs[i])
      if (k < length(rows)) {
        add(lab)
        walk(k + 1, groups[[i]], indent + 4)
      } else if (!statcols && multi) {
        add(lab)
        leaf(groups[[i]], indent + 4)
      } else {
        add(lab, leaf(groups[[i]], indent + 2))
      }
    }
  }
  all <- rep(TRUE, nrow(d))
  if (length(rows)) {
    walk(1, all, 0)
  } else {
    for (st in stats) add(label_of(st$s), cells_for(all, list(st)))
  }
  k <- length(colhead)
  shown <- function(x) if (is.na(x)) "" else x
  w <- vapply(seq_len(k), function(j) max(nchar(colhead[j]), vapply(lines, function(l) if (is.null(l$cells)) 0L else nchar(shown(l$cells[j])), 0L)), 0L)
  lead <- c(2, rep(3, max(0, k - 1)))
  span <- sum(lead + w)
  stub <- max(vapply(lines, function(l) nchar(l$lab), 0L)) + 1
  rule <- strrep("-", stub + 1 + span)
  pad <- function(s, n) vapply(seq_along(s), function(i) formatC(s[i], width = rep_len(n, length(s))[i]), "")
  row <- function(cells) paste0(strrep(" ", lead), pad(cells, w), collapse = "")
  cat("\n", rule, "\n", sep = "")
  if (length(cols)) {
    nm <- head_of(cols)
    left <- 2 + ceiling((span - 2 - nchar(nm)) / 2)
    cat(strrep(" ", stub), "|", strrep(" ", left), nm, strrep(" ", max(0, span - left - nchar(nm))), "\n", sep = "")
  }
  cat(strrep(" ", stub), "|", row(colhead), "\n", sep = "")
  cat(strrep("-", stub), "+", strrep("-", span), "\n", sep = "")
  for (l in lines) {
    cells <- if (is.null(l$cells)) rep("", k) else vapply(l$cells, shown, "")
    cat(formatC(l$lab, width = -stub), "|", row(cells), "\n", sep = "")
  }
  cat(rule, "\n", sep = "")
  invisible(NULL)
}"##;

/// The statistics `table` is translated for, with Stata's names.
fn stat_name(s: &str) -> Option<String> {
    let s = s.trim();
    let fixed = [
        ("frequency", 4),
        ("percent", 4),
        ("mean", 4),
        ("sd", 2),
        ("median", 3),
        ("min", 3),
        ("max", 3),
        ("sum", 3),
        ("count", 5),
        ("iqr", 3),
    ];
    if let Some((f, _)) = fixed
        .iter()
        .find(|(f, m)| s.len() >= *m && f.starts_with(s))
    {
        return Some((*f).to_string());
    }
    let k: u32 = s.strip_prefix('p')?.parse().ok()?;
    (1..=99).contains(&k).then(|| format!("p{k}"))
}

impl Session {
    /// `table rows [cols] [if] [in] [, statistic(...) nformat(...) nototals missing]`.
    pub(super) fn table(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "table")?;
        // the syntax before Stata 17: its own error, as the line runs
        if opts.has("contents", 1) {
            out.line("stop(\"option contents() not allowed since Stata 17; see help table for updated syntax (Stata: r(198))\", call. = FALSE)");
            return Ok(());
        }
        if let Some(o) = opts.other(&[
            ("statistic", 4),
            ("nformat", 4),
            ("nototals", 6),
            ("missing", 4),
        ]) {
            return Err(no(format!(
                "table with the option {o} is not translated yet"
            )));
        }
        let w = self.weight_of(c, &[super::weights::W::F])?;
        // the dimensions: (rows) (cols), or rows written out (a second variable the columns)
        let main = c.parts.main.trim();
        let names = |s: &str| -> Result<Vec<String>, Untranslated> {
            let v: Vec<String> = s.split_whitespace().map(str::to_string).collect();
            match v.iter().find(|w| !is_name(w)) {
                Some(w) => Err(no(format!(
                    "table {w}: variable names written out are translated"
                ))),
                None => Ok(v),
            }
        };
        let (rows, cols) = if main.starts_with('(') {
            // `(g) (h)`, `(g h) ()` and `() (g)`: the parentheses as written
            let inner: Vec<String> = main
                .split('(')
                .skip(1)
                .map(|g| g.split(')').next().unwrap_or("").trim().to_string())
                .collect();
            match inner.as_slice() {
                [r] => (names(r)?, Vec::new()),
                [r, k] => (names(r)?, names(k)?),
                _ => {
                    return Err(no(
                        "table with a third dimension (tables) is not translated yet",
                    ))
                }
            }
        } else {
            let v = names(main)?;
            match v.as_slice() {
                [r] => (vec![r.clone()], Vec::new()),
                [r, k] => (vec![r.clone()], vec![k.clone()]),
                _ => {
                    return Err(no(
                        "table with a third dimension (tables) is not translated yet",
                    ))
                }
            }
        };
        if rows.len() > 2 || cols.len() > 1 {
            return Err(no(
                "table with more than two row variables or one column variable is not translated yet",
            ));
        }
        if rows.is_empty() && cols.is_empty() {
            return Err(no("table needs a row or a column variable"));
        }
        // statistic(name varlist) ...: one entry per statistic and variable, in order
        let mut stats: Vec<(String, Option<String>)> = Vec::new();
        for o in c.options.iter().filter(|o| o.is("statistic", 4)) {
            let a = o.arg();
            let mut w = a.split_whitespace();
            let s = w.next().unwrap_or("");
            let name = stat_name(s).ok_or_else(|| {
                no(format!(
                    "statistic({a}): the statistic {s} is not translated yet"
                ))
            })?;
            let vars: Vec<&str> = w.collect();
            if matches!(name.as_str(), "frequency" | "percent") {
                if !vars.is_empty() {
                    return Err(no(format!("statistic({a}): {name} takes no variables")));
                }
                stats.push((name, None));
            } else {
                if vars.is_empty() {
                    return Err(no(format!(
                        "statistic({a}) needs a variable (Stata: r(198))"
                    )));
                }
                for v in vars {
                    if !is_name(v) {
                        return Err(no(format!(
                            "statistic({a}): variable names written out are translated"
                        )));
                    }
                    stats.push((name.clone(), Some(v.to_string())));
                }
            }
        }
        if stats.is_empty() {
            stats.push(("frequency".into(), None));
        }
        let kinds: std::collections::BTreeSet<&str> =
            stats.iter().map(|(s, _)| s.as_str()).collect();
        let vars: std::collections::BTreeSet<&str> =
            stats.iter().filter_map(|(_, v)| v.as_deref()).collect();
        if stats.len() > 1 {
            if !cols.is_empty() && stats.iter().any(|(_, v)| v.is_some()) && vars.len() > 1 {
                return Err(no("table with a column variable and statistics of several variables is not translated yet"));
            }
            if cols.is_empty() && kinds.len() > 1 && vars.len() > 1 {
                return Err(no(
                    "table with several statistics of several variables is not translated yet",
                ));
            }
            if !cols.is_empty() && rows.is_empty() {
                return Err(no(
                    "table () (g) with several statistics is not translated yet",
                ));
            }
        }
        // nformat(%fmt [statistics]): for those statistics, or all
        let mut fmts: Vec<(String, String)> = Vec::new();
        for o in c.options.iter().filter(|o| o.is("nformat", 4)) {
            let a = o.arg();
            let mut w = a.split_whitespace();
            let f = w.next().unwrap_or("").to_string();
            if !f.starts_with('%') {
                return Err(no(format!("nformat({a}) needs a format")));
            }
            let rest: Vec<&str> = w.collect();
            if rest.is_empty() {
                fmts.push(("_all".into(), f));
            } else {
                for s in rest {
                    let name = stat_name(s)
                        .ok_or_else(|| no(format!("nformat({a}): {s} is not a statistic")))?;
                    fmts.push((name, f.clone()));
                }
            }
        }
        if w.is_some() && stats.iter().any(|(s, _)| s != "frequency") {
            return Err(no(
                "table with frequency weights and statistics other than the frequency is not translated yet",
            ));
        }
        let mut rows_r = self.rows_for(c, out)?;
        if w.is_some() {
            rows_r = self.weighted_rows(rows_r, &w);
            out.line(format!("stata_rows <- {rows_r}"));
            rows_r = "stata_rows".into();
        }
        let wargs = match &w {
            Some((_, v)) => format!(", w = {rows_r}${}", r_name(v)),
            None => String::new(),
        };
        for h in ["stata_table", "stata_fmt"] {
            self.helpers.insert(h);
        }
        let q = |v: &[String]| v.iter().map(|s| r_text(s)).collect::<Vec<_>>().join(", ");
        let stats_r = stats
            .iter()
            .map(|(s, v)| {
                format!(
                    "list(s = {}, v = {})",
                    r_text(s),
                    v.as_deref()
                        .map(r_text)
                        .unwrap_or_else(|| "NA_character_".into())
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        let fmts_r = fmts
            .iter()
            .map(|(s, f)| format!("{} = {}", r_name(s), r_text(f)))
            .collect::<Vec<_>>()
            .join(", ");
        let call = format!(
            "stata_table({rows_r}, rows = c({}), cols = c({}), stats = list({stats_r}), fmts = list({fmts_r}){}{}{wargs})",
            q(&rows),
            q(&cols),
            if opts.has("nototals", 6) { ", totals = FALSE" } else { "" },
            if opts.has("missing", 4) { ", missing = TRUE" } else { "" }
        );
        if !self.quiet(c) {
            out.line(call);
        }
        self.r_ret =
            super::returned::Returned::Lost("r() after table was not checked against Stata".into());
        Ok(())
    }
}
