//! `svyset` and the `svy:` prefix, as YFBPT6 runs them: `svyset psu
//! [pweight=w]` (one stage, no strata, no finite population correction) and
//! `svy: clogit` (`plan/stata/svy.md`, observed in Stata 19). The estimates
//! are the weighted model's; the variance is linearized over the PSUs, with
//! the design's degrees of freedom (PSUs less strata) for the t tests and
//! Stata's adjusted Wald F.

use super::*;

/// The header `svy: clogit` prints (Stata's lines; the table is R's).
pub(super) const SVY_HELPER: &str = r#"stata_svy_header <- function(title, m) {
  e <- stata_e
  cat("(running clogit on estimation sample)\n")
  cat(sprintf("note: %d group%s (%d obs) omitted because of all positive or\n      all negative outcomes.\n",
              as.integer(m$N_group_drop), if (m$N_group_drop > 1) "s" else "", as.integer(m$N_drop)))
  cat("\n", title, "\n\n", sep = "")
  row <- function(left, right) cat(formatC(left, width = -50, flag = "-"), right, "\n", sep = "")
  big <- function(x) format(as.integer(x), big.mark = ",")
  row(sprintf("Number of strata = %3s", big(e$N_strata)), sprintf("Number of obs   = %10s", format(e$N, big.mark = ",")))
  row(sprintf("Number of PSUs   = %3s", big(e$N_psu)), sprintf("Population size = %10s", formatC(e$N_pop, format = "fg", digits = 6, big.mark = ",")))
  row("", sprintf("Design df       = %10s", big(e$df_r)))
  row("", sprintf("%-16s= %10.2f", sprintf("F(%d, %d)", as.integer(e$df_m), as.integer(e$df_r - e$df_m + 1)), e$F))
  row("", sprintf("Prob > F        = %10.4f", e$p))
  cat("\n")
  invisible(NULL)
}"#;

/// The subpopulation `svy, subpop()` names.
#[derive(Clone, Debug)]
pub(super) enum SubPop {
    /// A variable: not 0 and not missing.
    Var(String),
    /// `if exp`.
    Cond(String),
}

/// What `svy` hands the command it runs.
#[derive(Clone, Debug)]
pub(super) struct SvyCall {
    pub design: svydesign::SvyDesign,
    /// The line as Stata keeps it in e(cmdline).
    pub line: String,
    /// The command after the colon.
    pub command: String,
    pub subpop: Option<SubPop>,
    /// `svy, level(#)`.
    pub level: Option<String>,
}

impl SvyCall {
    /// The first stage's PSU variable ("" for each row its own).
    pub(super) fn psu(&self) -> String {
        self.design
            .stages
            .first()
            .and_then(|s| s.su.clone())
            .unwrap_or_default()
    }
    /// The sampling-weight variable, when there is one.
    pub(super) fn wvar(&self) -> Option<String> {
        self.design.weight.as_ref().map(|(_, w)| w.clone())
    }
    /// The simplest design: one stage, no strata or fpc, `singleunit(missing)`.
    pub(super) fn plain(&self) -> bool {
        self.design.stages.len() == 1
            && self
                .design
                .stages
                .iter()
                .all(|s| s.strata.is_none() && s.fpc.is_none())
            && self.design.singleunit == "missing"
    }
}

impl Session {
    /// `svy [, subpop() level()]: clogit|mean|total|ratio|proportion ...`.
    pub(super) fn svy(&mut self, c: &Command, out: &mut Out) -> Result<(), Untranslated> {
        let text = c.text.trim();
        let colon = super::random::top_level(text, ':')
            .ok_or_else(|| no("svy needs a command after a colon (Stata: r(198))"))?;
        let head = text.get(..colon).unwrap_or("").trim();
        let inner_text = text.get(colon + 1..).unwrap_or("").trim();
        // what follows the word svy (quietly and the like come before it)
        let after = head
            .rsplit_once(c.written.as_str())
            .map_or(head, |x| x.1)
            .trim();
        let opts_text = match after.strip_prefix(',') {
            Some(o) => o.trim(),
            None if after.is_empty() => "",
            None => {
                return Err(no(format!(
                    "svy with options ({head}) is not translated yet"
                )))
            }
        };
        let mut subpop_text: Option<String> = None;
        let mut level: Option<String> = None;
        for (name, arg) in svydesign::read_options(opts_text) {
            let arg = arg.unwrap_or_default();
            match name.as_str() {
                "subpop" if subpop_text.is_none() => subpop_text = Some(arg),
                "level" if level.is_none() => level = Some(arg),
                "vce" if arg == "linearized" => {}
                other => {
                    return Err(no(format!(
                        "svy with the option {other} is not translated yet"
                    )))
                }
            }
        }
        let Some(design) = self.svy_design.clone() else {
            return Err(no(
                "svy: the data are not svyset before it in this do-file (Stata: r(119))",
            ));
        };
        let program = crate::reader::read(inner_text);
        let inner = match program.commands().as_slice() {
            [one] => (*one).clone(),
            _ => return Err(no("svy: of more than one command is not translated")),
        };
        // the bases fvset set, and the other rewrites every command gets
        let inner = self.restated(&inner).unwrap_or(inner);
        let kind = inner.name.as_str();
        if !matches!(kind, "clogit" | "mean" | "total" | "ratio" | "proportion") {
            return Err(no(format!(
                "svy: {kind} is not translated yet (svy: clogit, mean, total, ratio and proportion are)"
            )));
        }
        if inner.parts.weight.is_some() {
            return Err(no(
                "svy: with weights of its own is not allowed (Stata: r(101))",
            ));
        }
        if design.stages.len() > 1 {
            return Err(no(
                "svy under a design with later stages (the variance of a second stage with fpc) is not translated yet",
            ));
        }
        if matches!(&design.weight, Some((k, _)) if *k != "pweight") {
            return Err(no("svy with iweights set is not translated yet"));
        }
        let subpop = match subpop_text {
            None => None,
            Some(s) => {
                let s = s.trim();
                if let Some(cond) = s.strip_prefix("if ").or_else(|| s.strip_prefix("if(")) {
                    let _ = cond;
                    Some(SubPop::Cond(s.to_string()))
                } else if is_name(s) {
                    Some(SubPop::Var(s.to_string()))
                } else {
                    return Err(no(format!(
                        "svy, subpop({s}): a variable or `if exp` is translated"
                    )));
                }
            }
        };
        let line = if opts_text.is_empty() {
            format!("svy : {inner_text}")
        } else {
            format!("svy , {opts_text}: {inner_text}")
        };
        let call = SvyCall {
            design,
            line,
            command: inner_text.to_string(),
            subpop,
            level,
        };
        let opts = Opts(&inner.options);
        if kind == "clogit" {
            if !opts_text.is_empty() {
                return Err(no(
                    "svy with options before clogit (subpop(), level()) is not translated yet",
                ));
            }
            let (Some(_), true) = (call.wvar(), call.plain()) else {
                return Err(no(
                    "svy: clogit under a design with strata, fpc, later stages, singleunit() or without pweights is not translated yet",
                ));
            };
            self.svy_call = Some(call);
            let done = self.clogit(&inner, &opts, out);
            self.svy_call = None;
            return done;
        }
        self.svy_call = Some(call);
        let done = self.meanest(&inner, kind, &opts, out);
        self.svy_call = None;
        done
    }
}

/// The linearized variance of a design with one stage: the scores `Z` (a
/// row each) summed to PSU totals, `n_h / (n_h - 1)` times the squared
/// deviations of a stratum's totals from their mean, `1 - f_h` times that
/// with an fpc (a value up to 1 is a sampling rate, above 1 a population of
/// PSUs), summed over the strata (`plan/stata/svy.md`). Strata with one PSU
/// follow `singleunit()`: missing (no variance), certainty (no variance from
/// them), scaled (the rest times H / (H - singletons)) or centered (their
/// total about the mean of all PSU totals).
pub(super) const SVYVLIN_HELPER: &str = r#"stata_svy_vlin <- function(Z, st, pu, fpc = NULL, su = "missing") {
  Z <- as.matrix(Z)
  key <- paste(st, pu, sep = "\r")
  U <- rowsum(Z, key, reorder = FALSE)
  sk <- st[match(rownames(U), key)]
  K <- ncol(Z)
  strata <- unique(sk)
  nh <- vapply(strata, function(h) sum(sk == h), 0)
  single <- sum(nh == 1)
  rate <- function(h, n) {
    if (is.null(fpc)) return(0)
    fv <- unique(fpc[st == h])
    if (length(fv) > 1) stop("fpc() must be the same within a stratum (Stata stops)", call. = FALSE)
    if (is.na(fv) || fv < 0) stop("fpc() must not be missing or negative (Stata stops)", call. = FALSE)
    if (fv <= 1) return(fv)
    if (fv < n) stop("fpc() is smaller than the number of PSUs sampled (Stata stops)", call. = FALSE)
    n / fv
  }
  V <- matrix(0, K, K)
  for (i in seq_along(strata)) {
    if (nh[i] < 2) next
    Uh <- U[sk == strata[i], , drop = FALSE]
    C <- sweep(Uh, 2, colMeans(Uh))
    f <- rate(strata[i], nh[i])
    if (f >= 1) stop("a stratum sampled completely (fpc() at 1 or the population of PSUs) is not translated yet", call. = FALSE)
    V <- V + (1 - f) * nh[i] / (nh[i] - 1) * crossprod(C)
  }
  note <- NULL
  if (single > 0) {
    if (!is.null(fpc)) stop("strata with one PSU together with fpc() are not translated yet", call. = FALSE)
    note <- switch(su,
      missing = "Note: Missing standard error because of stratum with single sampling unit.",
      certainty = "Note: Strata with single sampling unit treated as certainty units.",
      scaled = "Note: Variance scaled to handle strata with a single sampling unit.",
      centered = "Note: Strata with single sampling unit centered at overall mean.")
    if (su == "missing") V[] <- 0
    else if (su == "scaled") V <- V * length(strata) / (length(strata) - single)
    else if (su == "centered") {
      ubar <- colMeans(U)
      for (i in which(nh == 1)) {
        d <- U[sk == strata[i], , drop = TRUE] - ubar
        V <- V + tcrossprod(d)
      }
    }
  }
  list(V = V, nstrata = length(strata), npsu = nrow(U), single = single, note = note, na = single > 0 && su == "missing")
}"#;
