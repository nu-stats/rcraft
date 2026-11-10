//! P2C.9 continued, `fre` (the SSC package): a frequency table of each
//! variable, as it prints on Stata 19 with made-up data
//! (`plan/stata/fre.md`); only its output was read, never its code.
//!
//! - The title is the variable's name, with ` -- ` and its label when it has
//!   one. Each value is shown as %9.0g with its value label after it; a
//!   string's empty value is missing.
//! - The rows are Valid (one per value), a Total of them when there is more
//!   than one, Missing (one per missing value) and a Total of all when any is
//!   missing; the columns Freq., Percent, Valid and Cum., 11 wide.
//! - It leaves r(N), r(N_valid), r(N_missing), r(r), r(r_valid),
//!   r(r_missing) and r(depvar) of the last variable.

use super::*;

/// Prints the tables and gives the r() of the last one. Each missing value
/// has a row (`.`, then `.a` to `.z` with `tags`, their places from
/// `stata_mv`), a Total of them when there is more than one, and the Total of
/// all only when some value is valid (Stata 19).
pub(super) const FRE_HELPER: &str = r#"stata_fre <- function(d, vars, tags = FALSE) {
  r <- NULL
  for (v in vars) {
    x <- d[[v]]
    labs <- attr(x, "labels", exact = TRUE)
    vl <- attr(x, "label", exact = TRUE)
    is_s <- is.character(x)
    xv <- if (is_s) x else as.numeric(x)
    miss <- if (is_s) is.na(xv) | xv == "" else is.na(xv)
    N <- length(xv)
    nm <- sum(miss)
    nv <- N - nm
    vals <- if (is_s) sort(unique(xv[!miss]), method = "radix") else sort(unique(xv[!miss]))
    freq <- vapply(vals, function(u) sum(xv[!miss] == u), 0)
    text <- if (is_s) vals else vapply(vals, function(u) stata_fmt(u, "%9.0g", trim = TRUE), "")
    mp <- if (tags && !is_s) stata_mv(x)[miss] else rep(1, nm)
    mvals <- sort(unique(mp))
    mfreq <- vapply(mvals, function(p) sum(mp == p), 0)
    mtext <- if (is_s) rep("", length(mvals)) else c(".", paste0(".", letters))[mvals]
    if (!is_s && !is.null(labs)) {
      # with a label set, the values are padded to one width and the label
      # follows (`1  one`, `10 ten`, `.b dunno`)
      vw <- max(nchar(c(text, mtext)))
      m <- match(vals, as.numeric(labs))
      text <- sub(" +$", "", paste(formatC(text, width = -vw, flag = "-"), ifelse(is.na(m), "", names(labs)[m])))
      lt <- if (tags) haven::na_tag(as.numeric(labs)) else rep(NA_character_, length(labs))
      ml <- vapply(mvals, function(p) if (p > 1 && any(lt %in% letters[p - 1])) names(labs)[match(letters[p - 1], lt)] else "", "")
      mtext <- sub(" +$", "", paste(formatC(mtext, width = -vw, flag = "-"), ml))
    }
    vtot <- length(vals) > 1
    mtot <- length(mvals) > 1
    w <- max(c(nchar(text), if (vtot || mtot) 5, nchar(mtext), 0))
    W <- 8 + w + 1
    lab <- function(prefix, t) paste0(formatC(prefix, width = -8, flag = "-"), formatC(t, width = -w, flag = "-"), " ")
    cat("\n", v, if (!is.null(vl) && nzchar(vl)) paste(" --", vl), "\n", sep = "")
    cat(strrep("-", W + 45), "\n", sep = "")
    cat(strrep(" ", W), "|      Freq.    Percent      Valid       Cum.\n", sep = "")
    cat(strrep("-", W), "+", strrep("-", 44), "\n", sep = "")
    cum <- 0
    for (i in seq_along(vals)) {
      cum <- cum + 100 * freq[i] / nv
      cat(lab(if (i == 1) "Valid" else "", text[i]), sprintf("|%11.0f%11.2f%11.2f%11.2f\n", freq[i], 100 * freq[i] / N, 100 * freq[i] / nv, cum), sep = "")
    }
    if (vtot) cat(lab("", "Total"), sprintf("|%11.0f%11.2f%11.2f%11s\n", nv, 100 * nv / N, 100, ""), sep = "")
    for (i in seq_along(mvals)) {
      cat(lab(if (i == 1) "Missing" else "", mtext[i]), sprintf("|%11.0f%11.2f%22s\n", mfreq[i], 100 * mfreq[i] / N, ""), sep = "")
    }
    if (mtot) cat(lab("", "Total"), sprintf("|%11.0f%11.2f%22s\n", nm, 100 * nm / N, ""), sep = "")
    if (nm > 0 && nv > 0) {
      cat(formatC("Total", width = -W, flag = "-"), sprintf("|%11.0f%11.2f%22s\n", N, 100, ""), sep = "")
    }
    cat(strrep("-", W + 45), "\n", sep = "")
    r <- list(N = N, N_valid = nv, N_missing = nm, r = length(vals) + length(mvals), r_valid = length(vals),
              r_missing = length(mvals), depvar = v)
  }
  r
}"#;

impl Session {
    /// `fre varlist [if] [in]`.
    pub(super) fn fre(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "fre")?;
        if let Some(o) = opts.other(&[]) {
            return Err(no(format!("fre with the option {o} is not translated yet")));
        }
        if c.parts.weight.is_some() {
            return Err(no("fre with weights is not translated yet"));
        }
        let vars = names(&c.parts.main)?;
        if vars.is_empty() {
            return Err(no("fre needs variables"));
        }
        let src = self.rows_for(c, out)?;
        self.helpers.insert("stata_fre");
        self.helpers.insert("stata_fmt");
        let list: Vec<String> = vars.iter().map(|v| r_text(v)).collect();
        // each of . and .a to .z on a row of its own
        let tags = if self.mv_on(out) { ", tags = TRUE" } else { "" };
        out.line(format!(
            "stata_r <- stata_fre({src}, c({}){tags})",
            list.join(", ")
        ));
        self.r_new = Some((super::returned::R_FRE, "fre".into()));
        out.caveat("fre as the SSC package prints it, matched to its output (not its code)");
        Ok(())
    }
}
