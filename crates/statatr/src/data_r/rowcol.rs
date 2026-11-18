//! `clonevar`, `mvdecode`, `mvencode`, `fillin`, `expandcl` and `joinby`, as
//! Stata 19 does them on made-up data (batch A1 of the StatCraft port;
//! `plan/stata/clonevar.md` and the notes beside it).

use super::*;

/// A copy of `x` that keeps its label, display format and value label; rows
/// outside `if`/`in` become missing (`.` or `""`).
pub(super) const CLONE_HELPER: &str = r#"stata_clone <- function(x, keep = NULL) {
  if (!is.null(keep)) {
    keep <- rep_len(keep, length(x)) & !is.na(keep)
    if (is.character(x)) x[!keep] <- "" else x[!keep] <- NA
  }
  x
}"#;

impl Session {
    /// `clonevar newvar = varname [if] [in]`: the variable's type, display
    /// format, variable label, value label and characteristics, and its
    /// values where the condition holds (missing elsewhere).
    ///
    /// Observed in Stata 19: a new name that is there is r(110) (checked
    /// first); more than one name after `=` is r(101) "varlist not allowed";
    /// no `=` is r(198) "syntax is clonevar newvar = varname ..."; an empty
    /// right side is r(102); weights are r(101); a name the data lack
    /// (also in `if`) is r(111). It says `(# missing values generated)` with
    /// the count of missing values in the new variable.
    pub(super) fn clonevar(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[]) {
            return Err(no(format!(
                "clonevar with the option {o} is not translated yet"
            )));
        }
        if c.prefixes.iter().any(|p| matches!(p, Prefix::By { .. })) {
            return Err(no("clonevar with by: is not translated yet"));
        }
        let stop = |out: &mut Out, msg: &str| {
            out.line(format!("stop({}, call. = FALSE)", r_text(msg)));
        };
        let Some((lhs, rhs)) = c.parts.main.split_once('=') else {
            stop(
                out,
                "syntax is clonevar newvar = varname ... (Stata: r(198))",
            );
            return Ok(());
        };
        let new: Vec<&str> = lhs.split_whitespace().collect();
        let old: Vec<&str> = rhs.split_whitespace().collect();
        let [new] = new.as_slice() else {
            stop(
                out,
                "syntax is clonevar newvar = varname ... (Stata: r(198))",
            );
            return Ok(());
        };
        if !is_name(new) {
            stop(out, &format!("{new} invalid name (Stata: r(198))"));
            return Ok(());
        }
        let d = self.data()?;
        let exists = format!(
            "if ({} %in% names({d})) stop({}, call. = FALSE)",
            r_text(new),
            r_text(&format!("variable {new} already defined (Stata: r(110))"))
        );
        let old = match old.as_slice() {
            [] => {
                stop(out, "too few variables specified (Stata: r(102))");
                return Ok(());
            }
            [one] if is_name(one) => one.to_string(),
            [_] => {
                return Err(no(
                    "clonevar of a name pattern or an odd name is not translated yet",
                ))
            }
            _ => {
                stop(out, "varlist not allowed (Stata: r(101))");
                return Ok(());
            }
        };
        if c.parts.weight.is_some() {
            stop(out, "weights not allowed (Stata: r(101))");
            return Ok(());
        }
        out.line(exists);
        out.line(format!(
            "if (!{} %in% names({d})) stop({}, call. = FALSE)",
            r_text(&old),
            r_text(&format!("variable {old} not found (Stata: r(111))"))
        ));
        let keep = self.condition(c, out)?;
        let keep = keep.unwrap_or_else(|| "NULL".into());
        self.helpers.insert("stata_clone");
        out.package("dplyr");
        out.line(format!(
            "{d} <- {d} |> dplyr::mutate({} = stata_clone({}, {keep}))",
            r_name(new),
            r_name(&old)
        ));
        if !self.quiet(c) {
            self.helpers.insert("stata_chg");
            self.helpers.insert("stata_mv");
            out.line(format!("stata_gen_note({d}[[{}]])", r_text(new)));
        }
        if let Some(k) = self.kinds.get(&old).copied() {
            self.kinds.insert(new.to_string(), k);
        }
        if let Some(s) = self.storage.get(&old).copied() {
            self.storage.insert(new.to_string(), s);
        }
        Ok(())
    }
}
