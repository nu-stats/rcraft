//! `unique` (Michael Hills and Tony Brady's add-on from SSC), as observed in
//! Stata 19 on made-up data (`plan/stata/unique.md`); the add-on's program was
//! not read.
//!
//! `unique varlist [if] [in] [, by(varlist) generate(newvar) detail]`: the
//! number of distinct combinations of the varlist's values among the rows of
//! `if` and `in` (a missing value is a value), printed with the number of
//! records and left in r(N), r(sum) and r(unique). With `by()` the count
//! for each group is put in a new variable (`_Unique`) on the first row of the
//! group when the data are sorted by the by variables and the varlist (rows
//! in the data's order as ties), the other rows missing, and the groups and
//! their counts are listed.

use super::*;

pub(super) const UNIQUE_HELPER: &str = r##"stata_unique <- function(d, vars, rows = NULL, by = character(), gen = "_Unique", detail = FALSE) {
  for (v in vars) if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
  for (b in by) if (!b %in% names(d)) stop("variable ", b, " not found\n(error in option by()) (Stata: r(111))", call. = FALSE)
  n <- nrow(d)
  use <- if (is.null(rows)) rep(TRUE, n) else (!is.na(rows) & rows)
  key <- function(cols) do.call(paste, c(lapply(cols, function(v) {
    x <- d[[v]]
    if (is.character(x)) ifelse(is.na(x), "\001NA", paste0("s", x)) else { u <- unclass(x); attributes(u) <- NULL; ifelse(is.na(u), "\001NA", sprintf("%.17g", u)) }
  }), sep = "\002"))
  k <- key(vars)
  total <- length(unique(k[use]))
  records <- sum(use)
  title <- paste(vars, collapse = " ")
  cat("Number of unique values of ", title, " is  ", total, "\n", "Number of records is  ", records, "\n", sep = "")
  sum_detail <- function(counts, label) {
    f <- data.frame(stata_f_ = as.numeric(counts))
    attr(f$stata_f_, "label") <- label
    stata_summarize(f, detail = TRUE)
  }
  if (!length(by)) {
    if (detail) sum_detail(as.integer(table(k[use])), paste("Records per", title))
    return(list(N = records, sum = total, unique = total))
  }
  if (gen %in% names(d)) stop("variable ", gen, " already defined (Stata: r(110))", call. = FALSE)
  kb <- key(by)
  cols <- lapply(c(by, vars), function(v) { x <- d[[v]]; if (is.character(x)) x else { u <- unclass(x); attributes(u) <- NULL; u } })
  ord <- do.call(order, c(cols, list(seq_len(n), na.last = TRUE, method = "radix")))
  first <- ord[!duplicated(kb[ord])]
  out <- rep(NA_real_, n)
  cnt <- vapply(first, function(i) length(unique(k[use & kb == kb[i]])), 0)
  out[first] <- cnt
  # Stata's sort does not keep tied rows in order, so which of several tied first rows carries the
  # count is Stata's own choice (not reproduced): the first in the data's order here
  kall <- key(c(by, vars))
  if (any(vapply(first, function(i) sum(kall == kall[i]) > 1, TRUE)))
    message("note: rows tie at the top of a group; Stata's sort picks which of them holds ", gen, ", RCraft takes the first in the data's order")
  if (detail) {
    for (j in seq_along(first)) {
      i <- first[j]
      cat(strrep("-", 79), "\n", sep = "")
      cat("-> ", by, " = ", { x <- d[[by]]; if (is.na(x[i])) "." else format(x[i], digits = 15) }, "\n", sep = "")
      sum_detail(as.integer(table(k[use & kb == kb[i]])), paste("Records per", title))
    }
  }
  cat("variable ", gen, " contains number of unique values of ", title, " by ", paste(by, collapse = " "), "\n", sep = "")
  g <- stata_float(out)
  attr(g, "format.stata") <- "%9.0g"
  d[[gen]] <- g
  show <- d[first, c(by, gen), drop = FALSE]
  show$stata_obs_ <- seq_len(nrow(show))
  stata_list(show, noobs = TRUE)
  structure(list(N = records, sum = total, unique = total), data = d)
}"##;

impl Session {
    /// `unique varlist [if] [in] [, by(varlist) generate(name) detail]`.
    pub(super) fn unique(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[("by", 2), ("generate", 3), ("detail", 6)]) {
            return Err(no(format!(
                "unique with the option {o} is not translated yet"
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
                "unique {v}: names are translated (patterns and ranges are when the do-file runs)"
            )));
        }
        let by: Vec<String> = opts
            .arg("by", 2)
            .map(|b| b.split_whitespace().map(str::to_string).collect())
            .unwrap_or_default();
        if let Some(b) = by.iter().find(|b| !is_name(b)) {
            return Err(no(format!("unique, by({b}): names are translated")));
        }
        let detail = opts.has("detail", 6);
        if detail && by.len() > 1 {
            return Err(no(
                "unique, detail with by() of several variables is not translated yet (its group heading)",
            ));
        }
        let gen = match opts.arg("generate", 3) {
            Some(g) if is_name(g.trim()) => g.trim().to_string(),
            Some(g) => return Err(no(format!("unique, generate({g}): one new name is needed"))),
            None => "_Unique".to_string(),
        };
        let d = self.data()?;
        let rows = match self.condition(c, out)? {
            Some(cond) => {
                format!(", rows = dplyr::pull(dplyr::mutate({d}, stata_rows = {cond}), stata_rows)")
            }
            None => String::new(),
        };
        for h in [
            "stata_unique",
            "stata_summarize",
            "stata_list",
            "stata_fmt",
            "stata_tables",
            "stata_float",
        ] {
            self.helpers.insert(h);
        }
        out.package("dplyr");
        out.package("haven");
        let q = |v: &[String]| v.iter().map(|x| r_text(x)).collect::<Vec<_>>().join(", ");
        out.line(format!(
            "stata_r <- stata_unique({d}, c({}){rows}, by = {}, gen = {}, detail = {})",
            q(&vars),
            if by.is_empty() {
                "character()".to_string()
            } else {
                format!("c({})", q(&by))
            },
            r_text(&gen),
            super::stats::r_logical(detail),
        ));
        if !by.is_empty() {
            out.line(format!("{d} <- attr(stata_r, \"data\")"));
            out.line("attr(stata_r, \"data\") <- NULL");
            self.kinds.insert(gen.clone(), Kind::Number);
            self.storage.insert(gen, Storage::Float);
        }
        self.r_new = Some((&["N", "sum", "unique"], "unique".into()));
        Ok(())
    }
}
