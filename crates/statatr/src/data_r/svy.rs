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

impl Session {
    /// `svyset psu [pweight=w]`.
    pub(super) fn svyset(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[("vce", 3)]) {
            return Err(no(format!(
                "svyset with the option {o} is not translated yet"
            )));
        }
        if let Some(v) = opts.arg("vce", 3) {
            if !"linearized".starts_with(v.trim()) || v.trim().len() < 3 {
                return Err(no(format!("svyset, vce({v}) is not translated yet")));
            }
        }
        let psu = c.parts.main.trim();
        if psu.is_empty() || !(is_name(psu) || psu == "_n") {
            return Err(no("svyset with one PSU variable (or _n) is translated; strata, stages and fpc are not yet"));
        }
        let Some(w) = &c.parts.weight else {
            return Err(no("svyset without weights is not translated yet"));
        };
        if w.kind != crate::lang::syntax::WeightKind::Sampling {
            return Err(no(
                "svyset with weights other than pweight is not translated yet",
            ));
        }
        let wv = w.exp.trim().to_string();
        if !is_name(&wv) {
            return Err(no(
                "svyset with a pweight that is not a variable name is not translated yet",
            ));
        }
        let d = self.data()?;
        for v in [psu, wv.as_str()] {
            if v != "_n" {
                out.line(format!(
                    "if (!{0} %in% names({d})) stop(\"variable {1} not found (Stata: r(111))\", call. = FALSE)",
                    r_text(v),
                    v
                ));
            }
        }
        if !self.quiet(c) {
            out.line(format!(
                "cat(\"\\nSampling weights: {wv}\\n             VCE: linearized\\n      Subpop. df: off\\n     Single unit: missing\\n        Strata 1: <one>\\n Sampling unit 1: {}\\n           FPC 1: <zero>\\n\")",
                if psu == "_n" { "<observations>" } else { psu }
            ));
        }
        self.svy_design = Some((
            if psu == "_n" {
                String::new()
            } else {
                psu.to_string()
            },
            wv,
        ));
        Ok(())
    }

    /// `svy: clogit ...` (the one model translated under svy yet).
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
        if !after.is_empty() {
            return Err(no(format!(
                "svy with options ({head}) is not translated yet"
            )));
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
        if inner.name != "clogit" {
            return Err(no(format!(
                "svy: {} is not translated yet (svy: clogit is)",
                inner.name
            )));
        }
        if inner.parts.weight.is_some() {
            return Err(no(
                "svy: with weights of its own is not allowed (Stata: r(101))",
            ));
        }
        let opts = Opts(&inner.options);
        self.svy_call = Some((
            design,
            format!("svy : {inner_text}"),
            inner_text.to_string(),
        ));
        let done = self.clogit(&inner, &opts, out);
        self.svy_call = None;
        done
    }
}
