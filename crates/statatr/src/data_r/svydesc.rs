//! `svydescribe [varlist] [if] [in] [, stage(1) finalstage]`: the design's
//! lines and a table of its first stage's sampling units by stratum, as
//! Stata 19 prints them (`plan/stata/svy.md`), and its r(). Later stages,
//! `single` and `generate()` are refused.

use super::*;

/// What `svydescribe` leaves in r().
pub(super) const R_SVYDESC: &[&str] = &[
    "N_single",
    "N_mstrata",
    "N_munits",
    "N_miss",
    "N_strata",
    "N_units",
    "N",
];

/// The tables. `all` is the data, `d` the rows `if` and `in` keep; the
/// observations Stata leaves out (a missing stratum, unit or weight, and
/// those `if` and `in` drop) are counted under the table.
pub(super) const SVYDESCRIBE_HELPER: &str = r##"stata_svydescribe <- function(all, d, strata, su, wvar, vars, final = FALSE, show = TRUE) {
  for (v in c(strata, su, wvar, vars)) if (!v %in% names(all)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
  raw <- function(x) { x <- unclass(x); attributes(x) <- NULL; x }
  miss <- function(x) is.na(x) | (is.character(x) & x == "")
  n_all <- nrow(all)
  n <- nrow(d)
  S <- if (is.null(strata)) rep(1, n) else raw(d[[strata]])
  U <- if (is.null(su)) seq_len(n) else raw(d[[su]])
  w <- if (is.null(wvar)) NULL else as.numeric(raw(d[[wvar]]))
  ok <- !miss(S) & !miss(U)
  if (!is.null(w)) ok <- ok & !is.na(w)
  if (!is.null(w) && any(w[ok] < 0)) stop("negative weights encountered (Stata stops)", call. = FALSE)
  if (is.numeric(S) && any(S[ok] != round(S[ok]))) stop("svydescribe with strata that are not whole numbers is not translated yet", call. = FALSE)
  if (is.numeric(U) && any(U[ok] != round(U[ok]))) stop("svydescribe with units that are not whole numbers is not translated yet", call. = FALSE)
  cc <- rep(TRUE, n)
  for (v in vars) cc <- cc & !miss(raw(d[[v]]))
  Ss <- S[ok]; Us <- U[ok]; cs <- cc[ok]
  N <- length(Ss)
  excl <- n_all - N
  lab <- function(s) if (is.character(s)) s else sprintf("%.0f", s)
  lv <- unique(Ss)
  lv <- lv[order(lv, method = "radix")]
  if (any(nchar(vapply(lv, lab, "")) > 8)) stop("svydescribe with a stratum label wider than 8 characters is not translated yet", call. = FALSE)
  st <- lapply(lv, function(s) {
    i <- which(Ss == s)
    u <- Us[i]
    ul <- unique(u)
    ul <- ul[order(ul, method = "radix")]
    k <- match(u, ul)
    list(s = s, units = ul, obs = tabulate(k, length(ul)), complete = tabulate(k[cs[i]], length(ul)))
  })
  has_vars <- length(vars) > 0
  nm <- function(x) if (is.character(x)) formatC(x, width = 10) else formatC(x, width = 10, format = "d")
  n8 <- function(x) formatC(x, width = 8, format = "d")
  rule <- function(k) cat(strrep("-", k), "\n", sep = "")
  tail_note <- function(edge, trail) {
    cat(formatC(excl, width = edge), " = # obs with missing values in the", if (trail) " " else "", "\n", sep = "")
    cat(strrep(" ", edge - 8), strrep("-", 8), "   survey characteristics\n", sep = "")
    cat(formatC(n_all, width = edge), "\n", sep = "")
  }
  res <- list(N_single = 0, N_mstrata = 0, N_munits = 0, N_miss = 0, N_strata = length(st),
              N_units = sum(vapply(st, function(x) length(x$units), 0)), N = N)
  if (has_vars) {
    res$N_miss <- sum(!cs)
    res$N_munits <- sum(vapply(st, function(x) sum(x$complete == 0), 0))
    res$N_mstrata <- sum(vapply(st, function(x) all(x$complete == 0), TRUE))
  }
  if (!final) {
    if (!has_vars) {
      if (any(vapply(st, function(x) length(x$units) == 1, TRUE))) stop("svydescribe with a stratum of one sampling unit is not translated yet", call. = FALSE)
      if (show) {
        cat(strrep(" ", 36), "Number of obs per unit\n", sep = "")
        cat(" Stratum   # units     # obs       Min      Mean       Max\n")
        rule(58)
        for (x in st) cat(formatC(lab(x$s), width = 8), nm(length(x$units)), nm(sum(x$obs)), nm(min(x$obs)),
                          formatC(sprintf("%.1f", mean(x$obs)), width = 10), nm(max(x$obs)), "\n", sep = "")
        rule(58)
        allo <- unlist(lapply(st, function(x) x$obs))
        cat(n8(length(st)), nm(length(allo)), nm(sum(allo)), nm(min(allo)),
            formatC(sprintf("%.1f", mean(allo)), width = 10), nm(max(allo)), "\n", sep = "")
        if (excl > 0) { cat("\n"); tail_note(28, TRUE) }
      }
    } else {
      if (any(vapply(st, function(x) sum(x$complete > 0) == 1, TRUE))) stop("svydescribe with a stratum of one included sampling unit is not translated yet", call. = FALSE)
      if (show) {
        cat(strrep(" ", 30), "Number of obs with\n", sep = "")
        cat("             Number of units  complete   missing       # obs per included unit\n")
        cat(" Stratum  included   omitted      data      data       Min      Mean       Max\n")
        rule(78)
        for (x in st) {
          inc <- x$complete > 0
          ni <- sum(inc)
          cat(formatC(lab(x$s), width = 8), nm(ni), nm(sum(!inc)), nm(sum(x$complete)), nm(sum(x$obs) - sum(x$complete)), sep = "")
          if (ni == 0) cat(nm(0), formatC("0", width = 8), formatC("0", width = 12), "\n", sep = "")
          else cat(nm(min(x$complete[inc])), formatC(sprintf("%.1f", mean(x$complete[inc])), width = 10), nm(max(x$complete[inc])), "\n", sep = "")
        }
        rule(78)
        cm <- unlist(lapply(st, function(x) x$complete[x$complete > 0]))
        if (length(cm) == 0) stop("svydescribe where no unit has complete data is not translated yet", call. = FALSE)
        cat(n8(length(st)), nm(length(cm)), nm(res$N_units - length(cm)), nm(sum(cm)), nm(res$N_miss), nm(min(cm)),
            formatC(sprintf("%.1f", mean(cm)), width = 10), nm(max(cm)), "\n", sep = "")
        cat(strrep(" ", 30), strrep("-", 18), "\n", sep = "")
        if (excl > 0) { cat(formatC(N, width = 43), "\n", sep = ""); tail_note(43, TRUE) }
        else cat(formatC(N, width = 43), "\n", sep = "")
      }
    }
  } else {
    if (is.character(Us)) stop("svydescribe, finalstage with a string unit variable is not translated yet", call. = FALSE)
    if (show) {
      if (!has_vars) {
        cat("Stratum       Unit     # obs  \n")
        rule(28)
        for (x in st) for (j in seq_along(x$units)) cat(formatC(lab(x$s), width = 8), nm(sprintf("%.0f", x$units[j])), nm(x$obs[j]), "\n", sep = "")
        rule(28)
        cat(n8(length(st)), nm(res$N_units), nm(N), "\n", sep = "")
        if (excl > 0) tail_note(28, FALSE)
      } else {
        cat(strrep(" ", 20), "Number of obs with\n", sep = "")
        cat(strrep(" ", 20), "complete   missing   \n", sep = "")
        cat("Stratum       Unit      data      data  \n")
        rule(38)
        for (x in st) for (j in seq_along(x$units)) cat(formatC(lab(x$s), width = 8), nm(sprintf("%.0f", x$units[j])), nm(x$complete[j]), nm(x$obs[j] - x$complete[j]), "\n", sep = "")
        rule(38)
        cat(n8(length(st)), nm(res$N_units), nm(N - res$N_miss), nm(res$N_miss), "\n", sep = "")
        cat(strrep(" ", 20), strrep("-", 18), "\n", sep = "")
        if (excl > 0) { cat(formatC(N, width = 33), "\n", sep = ""); tail_note(33, FALSE) }
        else cat(formatC(n_all, width = 33), "\n", sep = "")
      }
    }
  }
  res
}"##;

impl Session {
    /// `svydescribe`.
    pub(super) fn svydescribe(&mut self, c: &Command, out: &mut Out) -> Result<(), Untranslated> {
        self.no_by(c, "svydescribe")?;
        if let Some(o) = Opts(&c.options).other(&[("stage", 3), ("finalstage", 5)]) {
            return Err(no(format!(
                "svydescribe with the option {o} is not translated yet"
            )));
        }
        let opts = Opts(&c.options);
        if c.parts.weight.is_some() {
            return Err(no(
                "svydescribe with weights is not allowed (Stata: r(101))",
            ));
        }
        if let Some(s) = opts.arg("stage", 3) {
            if s.trim() != "1" {
                return Err(no(
                    "svydescribe, stage(): only the first stage is translated yet",
                ));
            }
            if opts.has("finalstage", 5) {
                return Err(no(
                    "svydescribe with stage() and finalstage is not translated",
                ));
            }
        }
        let vars: Vec<&str> = c.parts.main.split_whitespace().collect();
        if let Some(v) = vars.iter().find(|v| !is_name(v)) {
            return Err(no(format!(
                "svydescribe {v}: the variables written out are translated"
            )));
        }
        let Some(design) = self.svy_design.clone() else {
            return Err(no(
                "svydescribe: the data are not svyset before it in this do-file (Stata: r(119))",
            ));
        };
        let finalstage = opts.has("finalstage", 5);
        let first = design
            .stages
            .first()
            .cloned()
            .unwrap_or(svydesign::SvyStage {
                strata: None,
                su: None,
                fpc: None,
            });
        if finalstage && (design.stages.len() > 1 || first.su.is_none()) {
            return Err(no(
                "svydescribe, finalstage is translated for one stage with a sampling unit variable",
            ));
        }
        let all = self.data()?;
        let rows = self.rows_for(c, out)?;
        self.helpers.insert("stata_svydescribe");
        let title = if finalstage {
            "final stage".to_string()
        } else {
            "stage 1".to_string()
        };
        let head = format!(
            "\nSurvey: Describing {title} sampling units\n\n{}\n",
            design.lines()
        );
        let quiet = self.quiet(c);
        if !quiet {
            out.line(format!("cat({})", r_text(&head)));
        }
        let opt = |v: &Option<String>| v.as_deref().map_or("NULL".to_string(), r_text);
        let list: Vec<String> = vars.iter().map(|v| r_text(v)).collect();
        out.line(format!(
            "stata_r <- stata_svydescribe({all}, {rows}, {}, {}, {}, c({}), final = {}, show = {})",
            opt(&first.strata),
            opt(&first.su),
            opt(&design.weight.as_ref().map(|(_, w)| w.clone())),
            list.join(", "),
            stats::r_logical(finalstage),
            stats::r_logical(!quiet),
        ));
        self.r_new = Some((R_SVYDESC, "svydescribe".into()));
        Ok(())
    }
}
