//! P2C.9 continued, `parmest` (the SSC package): a dataset of the active
//! estimates, as it saves them on Stata 19 with made-up data
//! (`plan/stata/parmest.md`); only the files it saved were read, never its
//! code.
//!
//! - One row per column of e(b): `idnum` (with `idnum()`), `eq` when e(b) has
//!   equation names, `parm`, `estimate`, `stderr`, `z`, `p`, `min95` and
//!   `max95`, with its variable labels. A base level is 0 with z and p
//!   missing.
//! - With `saving()` the data in memory stay as they are.

use super::*;

/// Saves the estimates of `stata_e` to `file`.
pub(super) const PARMEST_HELPER: &str = r#"stata_parmest <- function(file, idnum = NULL, level = 95) {
  b <- stata_e$b
  V <- as.matrix(stata_e$V)
  nm <- colnames(b)
  est <- as.numeric(b)
  se <- sqrt(pmax(diag(V), 0))
  has_eq <- any(grepl(":", nm, fixed = TRUE))
  q <- stats::qnorm(1 - (1 - level / 100) / 2)
  z <- ifelse(se > 0, est / se, NA_real_)
  lab <- function(x, l, f) structure(x, label = l, format.stata = f)
  d <- list()
  if (!is.null(idnum)) d$idnum <- lab(rep(idnum, length(nm)), "Numeric ID", "%10.0g")
  if (has_eq) d$eq <- lab(sub(":.*$", "", nm), "Equation name", "%9s")
  d$parm <- lab(sub("^.*:", "", nm), "Parameter name", "%10s")
  d$estimate <- lab(est, "Parameter estimate", "%10.0g")
  d$stderr <- lab(se, "SE of parameter estimate", "%10.0g")
  d$z <- lab(z, "Standard normal deviate", "%10.0g")
  d$p <- lab(2 * stats::pnorm(-abs(z)), "P-value", "%10.0g")
  d[[paste0("min", level)]] <- lab(est - q * se, sprintf("Lower %g%% confidence limit", level), "%10.0g")
  d[[paste0("max", level)]] <- lab(est + q * se, sprintf("Upper %g%% confidence limit", level), "%10.0g")
  haven::write_dta(as.data.frame(d, check.names = FALSE), file)
  cat(sprintf("file %s saved\n", file))
  invisible(NULL)
}"#;

impl Session {
    /// `parmest, saving(file[, replace]) [idnum(#)]` after a model.
    pub(super) fn parmest(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if !c.parts.main.trim().is_empty() {
            return Err(no("parmest takes only options here"));
        }
        if let Some(o) = opts.other(&[("saving", 3), ("idnum", 3)]) {
            return Err(no(format!(
                "parmest with the option {o} is not translated yet"
            )));
        }
        if matches!(self.e_ret, super::returned::Returned::Nothing) {
            return Err(no(
                "parmest: no model is fitted before it in this do-file (Stata: last estimates not found)",
            ));
        }
        if let super::returned::Returned::Lost(why) = &self.e_ret {
            return Err(no(format!(
                "parmest: the estimates are not known here: {why}"
            )));
        }
        if self.estimates.as_ref().is_some_and(|e| {
            !matches!(
                e.fit,
                super::model::Fit::Clogit
                    | super::model::Fit::Margins
                    | super::model::Fit::Logit
                    | super::model::Fit::Probit
                    | super::model::Fit::Poisson
            )
        }) {
            return Err(no(
                "parmest after a model with t tests is not translated yet (its normal-deviate form is)",
            ));
        }
        let saving = opts.arg("saving", 3).ok_or_else(|| {
            no("parmest without saving() (it would replace the data) is not translated yet")
        })?;
        let (file, sub) = match saving.split_once(',') {
            Some((f, s)) => (f.trim(), s.trim()),
            None => (saving.trim(), ""),
        };
        let replace = match sub {
            "" => false,
            s if s.len() >= 3 && "replace".starts_with(s) => true,
            s => {
                return Err(no(format!(
                    "parmest, saving(..., {s}) is not translated yet"
                )))
            }
        };
        let path = file_name(file, "dta");
        let id = match opts.arg("idnum", 3) {
            Some(v) => match v.trim().parse::<f64>() {
                Ok(n) => format!(", idnum = {}", r_number(n)),
                Err(_) => return Err(no(format!("idnum({v}): only a number is translated"))),
            },
            None => String::new(),
        };
        if !replace {
            out.line(format!(
                "if (file.exists({p})) stop(\"file {e} already exists\")",
                p = r_text(&path),
                e = path.replace('"', "")
            ));
        }
        out.package("haven");
        self.helpers.insert("stata_parmest");
        out.line(format!("stata_parmest({}{id})", r_text(&path)));
        out.caveat(
            "parmest as the SSC package saves its dataset, matched to the files it saves (not its \
             code), for models with normal-deviate tests",
        );
        Ok(())
    }
}
