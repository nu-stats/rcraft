//! `codebook [varlist] [if] [in]`: a description of each variable, as
//! Stata 19 lays it out (observed on made-up data): its label, range and
//! units, unique values and missing ones, then a tabulation when it has 9
//! values or fewer, or else the mean, standard deviation and percentiles
//! (Stata's, which average at a tie). R does not keep Stata's storage types,
//! so the type line names only numeric or string.

use super::*;

pub(super) const CODEBOOK_HELPER: &str = r##"stata_codebook <- function(d, vars, rows = NULL) {
  if (!is.null(rows)) d <- d[!is.na(rows) & rows, , drop = FALSE]
  rule <- strrep("-", 79)
  pad <- function(s, w) formatC(s, width = w)
  big <- function(n) format(n, big.mark = ",", scientific = FALSE, trim = TRUE)
  for (v in vars) {
    if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
    x <- d[[v]]
    lab <- attr(x, "label", exact = TRUE)
    vl <- attr(x, "labels", exact = TRUE)
    cat("\n", rule, "\n", sep = "")
    right <- if (is.null(lab) || !nzchar(lab)) "(unlabeled)" else lab
    cat(v, strrep(" ", max(1, 79 - nchar(v) - nchar(right))), right, "\n", rule, "\n\n", sep = "")
    n <- length(x)
    if (is.character(x)) {
      longest <- max(c(0, nchar(x, type = "bytes")))
      cat(pad("Type:", 22), " String, longest is str", longest, "\n\n", sep = "")
      miss <- sum(is.na(x) | x == "")
      u <- unique(x[!is.na(x)])
      cat(pad("Unique values:", 22), " ", formatC(big(length(u)), width = -26), "Missing \"\": ", big(miss), "/", big(n), "\n", sep = "")
      if (length(u) <= 9) {
        cat("\n", pad("Tabulation:", 22), " Freq.  Value\n", sep = "")
        tab <- table(factor(x, levels = sort(u, method = "radix")))
        for (k in names(tab)) cat(pad(big(tab[[k]]), 29), "  \"", k, "\"\n", sep = "")
      }
      next
    }
    x <- as.numeric(unclass(x))
    cat(pad("Type:", 22), " Numeric\n", sep = "")
    named <- !is.null(vl) && length(vl) > 0
    if (named) {
      set <- attr(vl, "name", exact = TRUE)
      cat(pad("Label:", 22), " ", if (is.null(set)) "(value labels)" else set, "\n", sep = "")
    }
    cat("\n")
    ok <- x[!is.na(x)]
    miss <- sum(is.na(x))
    if (length(ok) == 0) {
      cat(pad("Range:", 22), " [.,.]\n", sep = "")
    } else {
      units <- 1e-13
      for (k in 15:-12) {
        u <- 10^k
        if (all(abs(ok / u - round(ok / u)) <= 1e-9 * pmax(1, abs(ok / u)))) {
          units <- u
          break
        }
      }
      ushow <- if (units >= 1) format(units, scientific = FALSE) else if (units >= 1e-4) sub("^0", "", format(units, scientific = FALSE)) else sprintf("%.3e", units)
      range <- paste0("[", trimws(stata_fmt(min(ok), "%9.0g")), ",", trimws(stata_fmt(max(ok), "%9.0g")), "]")
      cat(pad("Range:", 22), " ", formatC(range, width = -30), "Units: ", ushow, "\n", sep = "")
    }
    u <- sort(unique(ok))
    cat(pad("Unique values:", 22), " ", formatC(big(length(u)), width = -26), "Missing .: ", big(miss), "/", big(n), "\n", sep = "")
    if (length(u) == 0) next
    if (length(u) <= 9) {
      cat("\n", pad("Tabulation:", 22), " Freq.", if (named) "   Numeric  Label" else "  Value", "\n", sep = "")
      for (val in u) {
        f <- sum(ok == val)
        shown <- trimws(stata_fmt(val, "%9.0g"))
        if (named) {
          l <- names(vl)[match(val, as.numeric(vl))]
          cat(pad(big(f), 29), pad(shown, 10), "  ", if (is.na(l)) "" else l, "\n", sep = "")
        } else {
          cat(pad(big(f), 29), "  ", shown, "\n", sep = "")
        }
      }
      if (miss > 0) {
        if (named) cat(pad(big(miss), 29), pad(".", 10), "\n", sep = "") else cat(pad(big(miss), 29), "  .\n", sep = "")
      }
    } else {
      cat("\n", pad("Mean:", 22), " ", trimws(stata_fmt(mean(ok), "%8.0g")), "\n", sep = "")
      cat(pad("Std. dev.:", 22), " ", trimws(stata_fmt(stats::sd(ok), "%8.0g")), "\n\n", sep = "")
      p <- vapply(c(10, 25, 50, 75, 90), function(q) stata_pctile(ok, q), 0)
      cat(pad("Percentiles:", 22), paste(pad(c("10%", "25%", "50%", "75%", "90%"), 9), collapse = " "), "\n", sep = "")
      cat(strrep(" ", 22), paste(pad(vapply(p, function(v) trimws(stata_fmt(v, "%8.0g")), ""), 9), collapse = " "), "\n", sep = "")
    }
  }
  invisible(NULL)
}"##;

impl Session {
    pub(super) fn codebook(&mut self, c: &Command, out: &mut Out) -> Result<(), Untranslated> {
        if !c.options.is_empty() {
            return Err(no(format!(
                "codebook with options is not translated yet ({})",
                c.options
                    .iter()
                    .map(|o| o.name.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            )));
        }
        let vars: Vec<&str> = c.parts.main.split_whitespace().collect();
        if let Some(v) = vars.iter().find(|v| !is_name(v)) {
            return Err(no(format!(
                "codebook {v}: names are translated (patterns are when the do-file runs)"
            )));
        }
        let d = self.data()?;
        let rows = match self.condition(c, out)? {
            Some(cond) => {
                format!(", rows = dplyr::pull(dplyr::mutate({d}, stata_rows = {cond}), stata_rows)")
            }
            None => String::new(),
        };
        if self.quiet(c) {
            return Ok(());
        }
        self.helpers.insert("stata_codebook");
        self.helpers.insert("stata_fmt");
        self.helpers.insert("stata_stats");
        let list = if vars.is_empty() {
            format!("names({d})")
        } else {
            format!(
                "c({})",
                vars.iter()
                    .map(|v| r_text(v))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        out.line(format!("stata_codebook({d}, {list}{rows})"));
        out.caveat(
            "printed in R's layout; R does not keep Stata's storage types, so they are not shown",
        );
        Ok(())
    }
}
