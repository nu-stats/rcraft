//! `revrs varlist [, replace]` (SSC, Kyle Longest), as Stata 19 runs it
//! (observed with made-up data; its code and help were not read, see
//! `plan/stata/revrs.md`).
//!
//! A variable with a value label: the labelled values in the data, in order,
//! take the codes k, ..., 1 (1, 2, 5 become 3, 2, 1); a value without a label
//! is missing; the new label set `rev<name>` gives each code the old text;
//! byte (int past 100 codes), `%8.0g`, the variable label kept. Without a
//! value label: (max + 1) - x, missing values as they are. `rev<name>` is a
//! new variable at the end; with `replace` a labelled variable is dropped and
//! comes back at the end under its name (label set `rev<name>`), an unlabelled
//! one keeps its place and its storage. `rev<name>` there already: r(110), but
//! for a labelled variable without `replace` Stata leaves it as it is.

use super::*;

pub(super) const REVRS_HELPER: &str = r##"stata_revrs <- function(d, vars, replace = FALSE) {
  for (v in vars) {
    if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
    x <- d[[v]]
    if (is.character(x)) stop("string variables not allowed in varlist;\n", v, " is a string variable (Stata: r(109))", call. = FALSE)
    nv <- paste0("rev", v)
    l <- attr(x, "labels", exact = TRUE)
    if (!is.null(l)) {
      # rev<name> there already: with replace r(110); without, Stata leaves it as it is
      if (nv %in% names(d)) { if (replace) stop("variable ", nv, " already defined (Stata: r(110))", call. = FALSE) else next }
      if (anyNA(l)) stop("RCraft: revrs of a variable whose value label names missing values is not translated yet", call. = FALSE)
      num <- as.numeric(x)
      lv <- sort(unique(num[!is.na(num) & num %in% as.numeric(l)]))
      k <- length(lv)
      if (k > 32740) stop("RCraft: revrs of more than 32,740 labelled values is not translated yet", call. = FALSE)
      codes <- as.numeric(k + 1 - seq_len(k))
      labs <- stats::setNames(codes, names(l)[match(lv, as.numeric(l))])
      labs <- labs[order(codes)]
      newv <- haven::labelled(as.numeric(k + 1 - match(num, lv)), labels = labs,
                              label = attr(x, "label", exact = TRUE))
      attr(newv, "format.stata") <- "%8.0g"
      assign(paste0("label_", nv), labs, envir = parent.frame())
      if (replace) {
        d[[v]] <- NULL
        d[[v]] <- newv
      } else {
        d[[nv]] <- newv
      }
    } else {
      num <- as.numeric(x)
      ok <- !is.na(num)
      if (any(num[ok] != round(num[ok]) & num[ok] * 2 != round(num[ok] * 2))) {
        stop("RCraft: revrs of values that are not whole numbers or halves is not translated yet", call. = FALSE)
      }
      if (replace) {
        if (any(ok)) x[ok] <- max(num[ok]) + 1 - num[ok]
        d[[v]] <- x
      } else {
        if (nv %in% names(d)) stop("variable ", nv, " already defined (Stata: r(110))", call. = FALSE)
        y <- rep(NA_real_, length(num))
        if (any(ok)) y[ok] <- max(num[ok]) + 1 - num[ok]
        d[[nv]] <- y
      }
    }
  }
  d
}"##;

impl Session {
    pub(super) fn revrs(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[("replace", 7)]) {
            return Err(no(format!(
                "revrs with the option {o} is not translated yet"
            )));
        }
        if c.parts.if_exp.is_some() || c.parts.in_range.is_some() {
            return Err(no("revrs with if or in is not translated yet"));
        }
        let vars: Vec<String> = c
            .parts
            .main
            .split_whitespace()
            .map(str::to_string)
            .collect();
        if vars.is_empty() {
            return Err(no("revrs needs a varlist (Stata: varlist required)"));
        }
        for v in &vars {
            if !is_name(v) {
                return Err(no(format!(
                    "revrs {v}: patterns and abbreviations are taken once the data's variables are known"
                )));
            }
            if v.len() + 3 > 32 {
                return Err(no(format!(
                    "revrs {v}: a name rev{v} longer than 32 characters is not translated yet"
                )));
            }
        }
        let replace = opts.has("replace", 7);
        let d = self.data()?;
        self.helpers.insert("stata_revrs");
        out.package("haven");
        out.line(format!(
            "{d} <- stata_revrs({d}, c({}), replace = {})",
            vars.iter()
                .map(|v| r_text(v))
                .collect::<Vec<_>>()
                .join(", "),
            super::stats::r_logical(replace)
        ));
        for v in &vars {
            if replace {
                // a labelled one comes back as byte or int; an unlabelled one keeps its storage
                // (which only R knows here)
                self.storage.remove(v);
            } else {
                self.kinds.insert(format!("rev{v}"), Kind::Number);
            }
        }
        out.caveat("revrs as the SSC package runs it, matched to its output (not its code)");
        Ok(())
    }
}
