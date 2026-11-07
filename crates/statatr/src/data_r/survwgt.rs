//! `survwgt rake` (Nick Winter's survwgt, SSC, version 1.2.4 in Stata 19),
//! as observed on made-up data (`plan/stata/survwgt.md`): raking (iterative
//! proportional fitting) of a weight to control totals, one dimension per
//! `by()` variable, adjusted in order within each pass. Only the add-on's
//! behaviour was observed; its program file was not read.

use super::*;

/// The raking. `quiet` keeps the warning to itself, as `quietly` does.
///
/// Observed in Stata 19:
/// - each pass adjusts the dimensions in order: w = w * T / S, S the
///   category's weighted total;
/// - it stops after the first pass in which no category, at the step that
///   adjusts it, was 0.001 or more from its total; maxrep() passes at most
///   (10 by default), and the warning when the passes reach maxrep();
/// - with totals in the thousands it can stop sooner, by a rule not yet
///   pinned down, once the margins agree to about 3e-6 relatively; that
///   is refused rather than guessed (one dimension is exact either way);
/// - a missing weight stays missing and is left out; a missing by() value
///   is a category of its own; a total must be the same throughout a
///   category (r(198)); the dimensions' totals must agree to 0.001 or
///   1e-5 of the first's (r(199));
/// - the new variable is a double labelled "(raked)", and the data are
///   left sorted by the last dimension.
pub(super) const SURVWGT_HELPER: &str = r#"stata_survwgt_rake <- function(d, pre, by, tot, gen, maxrep = 10, quiet = FALSE) {
  if (length(by) != length(tot)) stop("number of by() variables must equal number of totvars() (Stata: r(198))", call. = FALSE)
  for (v in c(pre, by, tot)) if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
  if (gen %in% names(d)) stop("variable ", gen, " already defined (Stata: r(110))", call. = FALSE)
  w <- as.numeric(d[[pre]])
  use <- !is.na(w)
  keys <- lapply(by, function(v) factor(d[[v]][use], exclude = NULL))
  T <- lapply(tot, function(v) as.numeric(d[[v]][use]))
  sums <- numeric(length(by))
  for (j in seq_along(by)) {
    same <- tapply(T[[j]], keys[[j]], function(t) all(is.na(t)) || (!anyNA(t) && all(t == t[1])))
    if (!all(same)) stop("Control total ", tot[j], " not constant within categories of dimension ", by[j], " (Stata: r(198))", call. = FALSE)
    if (anyNA(T[[j]])) stop("survwgt rake: a category whose total is missing is not translated yet", call. = FALSE)
    sums[j] <- sum(tapply(T[[j]], keys[[j]], `[`, 1))
  }
  for (j in seq_along(by)[-1]) {
    if (abs(sums[j] - sums[1]) > max(.001, 1e-5 * abs(sums[1])))
      stop("totals across dimensions 1 and ", j, " are not equal (Stata: r(199))", call. = FALSE)
  }
  x <- w[use]
  done <- NA
  close <- FALSE
  for (rep in seq_len(maxrep)) {
    far <- 0
    rel <- 0
    for (j in seq_along(by)) {
      S <- ave(x, keys[[j]], FUN = sum)
      if (any(S == 0)) stop("survwgt rake: a category whose weights are all zero is not translated yet", call. = FALSE)
      far <- max(far, abs(S - T[[j]]))
      rel <- max(rel, abs(S - T[[j]]) / abs(T[[j]]))
      x <- x * T[[j]] / S
    }
    if (far < .001) {
      done <- rep
      break
    }
    # Stata's other stopping rule may fire on a pass like this one
    if (length(by) > 1 && rel < 1e-5) close <- TRUE
  }
  if (close) stop("survwgt rake: these totals are large enough that Stata may stop raking by a rule not translated yet", call. = FALSE)
  if ((is.na(done) || done == maxrep) && !quiet) cat("Warning: variable ", pre, " reached maximum\niterations before convergence.\n", sep = "")
  out <- rep(NA_real_, nrow(d))
  out[use] <- x
  attr(out, "label") <- "(raked)"
  d[[gen]] <- out
  d |> dplyr::arrange(.data[[by[length(by)]]])
}"#;

impl Session {
    /// `survwgt rake weight, by(varlist) totvars(varlist) generate(newvar)
    /// [maxrep(#)]`; the add-on's other subcommands are not translated.
    pub(super) fn survwgt(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let mut words = c.parts.main.split_whitespace();
        let sub = words.next().unwrap_or("");
        if sub != "rake" {
            return Err(no(format!(
                "survwgt {sub} is not translated yet (survwgt rake is)"
            )));
        }
        let pre: Vec<&str> = words.collect();
        let [pre] = pre.as_slice() else {
            return Err(no("survwgt rake needs one weight variable"));
        };
        if !is_name(pre) {
            return Err(no(format!(
                "survwgt rake {pre}: a variable's name is needed"
            )));
        }
        if c.parts.if_exp.is_some() || c.parts.in_range.is_some() || c.parts.weight.is_some() {
            return Err(no(
                "survwgt rake with if, in or weights is not translated yet",
            ));
        }
        if let Some(o) = opts.other(&[("by", 2), ("totvars", 7), ("generate", 3), ("maxrep", 6)]) {
            return Err(no(format!(
                "survwgt rake with the option {o} is not translated yet"
            )));
        }
        let names = |name: &str, min: usize| -> Result<Vec<String>, Untranslated> {
            let list: Vec<String> = opts
                .arg(name, min)
                .unwrap_or_default()
                .split_whitespace()
                .map(str::to_string)
                .collect();
            if list.is_empty() {
                return Err(no(format!("survwgt rake needs {name}() (Stata: r(198))")));
            }
            if let Some(w) = list.iter().find(|w| !is_name(w)) {
                return Err(no(format!(
                    "survwgt rake, {name}({w}): only variable names are translated"
                )));
            }
            Ok(list)
        };
        let by = names("by", 2)?;
        let tot = names("totvars", 7)?;
        let gen = names("generate", 3)?;
        let [gen] = gen.as_slice() else {
            return Err(no("survwgt rake, generate() takes one new name"));
        };
        let maxrep = match opts.arg("maxrep", 6) {
            Some(m) => m.trim().parse::<u32>().map_err(|_| {
                no(format!(
                    "survwgt rake, maxrep({m}): only a whole number is translated"
                ))
            })?,
            None => 10,
        };
        let d = self.data()?;
        let list = |v: &[String]| {
            format!(
                "c({})",
                v.iter().map(|n| r_text(n)).collect::<Vec<_>>().join(", ")
            )
        };
        out.package("dplyr");
        self.helpers.insert("stata_survwgt");
        out.line(format!(
            "{d} <- stata_survwgt_rake({d}, {}, {}, {}, {}, maxrep = {maxrep}, quiet = {})",
            r_text(pre),
            list(&by),
            list(&tot),
            r_text(gen),
            if self.quiet(c) { "TRUE" } else { "FALSE" }
        ));
        self.kinds.insert(gen.clone(), Kind::Number);
        self.storage.insert(gen.clone(), Storage::Double);
        out.caveat(
            "the data are left sorted by the last by() variable, as Stata leaves them; \
             rows that tie keep their order in R, which Stata does not promise",
        );
        Ok(())
    }
}
