//! `tabm varlist [if] [, missing]` (tab_chi, SSC, Nick Cox), as Stata 19 runs
//! it (observed with made-up data; its code and help were not read, see
//! `plan/stata/tabm.md`): the variables stacked, one table of `variable`
//! (each variable a row, in the varlist's order) by `values` (each value a
//! column, labelled with the first variable's value label), laid out and split
//! as tabulate does; r(N), r(r) and r(c) as tabulate's.

use super::*;

pub(super) const TABM_HELPER: &str = r##"stata_tabm <- function(d, vars, missing = FALSE) {
  for (v in vars) {
    if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
    if (is.character(d[[v]])) stop("RCraft: tabm of a string variable is not translated yet", call. = FALSE)
  }
  vals <- unlist(lapply(vars, function(v) as.numeric(d[[v]])), use.names = FALSE)
  if (missing && any(haven::is_tagged_na(vals))) {
    stop("RCraft: tabm, missing with .a to .z in the data is not translated yet", call. = FALSE)
  }
  x <- haven::labelled(rep(seq_along(vars), each = nrow(d)), labels = stats::setNames(seq_along(vars), vars))
  l <- attr(d[[vars[1]]], "labels", exact = TRUE)
  y <- if (is.null(l)) vals else haven::labelled(vals, labels = l)
  r <- stata_tabulate(stata_tab_values(x), stata_tab_values(y), missing = missing, ret = TRUE,
                      xname = "variable", yname = "values")
  assign("stata_r", stata_r, envir = parent.frame())
  invisible(r)
}"##;

/// The r() tabm leaves (Stata 19: r(N), the number of values counted).
const R_TABM: &[&str] = &["N", "r", "c"];

impl Session {
    pub(super) fn tabm(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[("missing", 1)]) {
            return Err(no(format!(
                "tabm with the option {o} is not translated yet"
            )));
        }
        if c.parts.in_range.is_some() {
            return Err(no("tabm with in is not translated yet"));
        }
        if c.parts.weight.is_some() {
            return Err(no("tabm with weights is not translated yet"));
        }
        let vars: Vec<String> = c
            .parts
            .main
            .split_whitespace()
            .map(str::to_string)
            .collect();
        if vars.is_empty() {
            return Err(no("tabm needs a varlist (Stata: varlist required)"));
        }
        if let Some(v) = vars.iter().find(|v| !is_name(v)) {
            return Err(no(format!(
                "tabm {v}: patterns and abbreviations are taken once the data's variables are known"
            )));
        }
        let d = self.data()?;
        let src = match self.condition(c, out)? {
            Some(cond) => {
                out.package("dplyr");
                format!("dplyr::filter({d}, {cond})")
            }
            None => d.clone(),
        };
        for h in [
            "stata_tabm",
            "stata_tabulate",
            "stata_tab_values",
            "stata_tables",
            "stata_fmt",
        ] {
            self.helpers.insert(h);
        }
        out.package("haven");
        let call = format!(
            "stata_tabm({src}, c({}){})",
            vars.iter()
                .map(|v| r_text(v))
                .collect::<Vec<_>>()
                .join(", "),
            if opts.has("missing", 1) {
                ", missing = TRUE"
            } else {
                ""
            }
        );
        out.line(if self.quiet(c) {
            format!("invisible(utils::capture.output({call}))")
        } else {
            call
        });
        self.r_new = Some((R_TABM, "tabm".into()));
        out.caveat("tabm as the SSC package runs it, matched to its output (not its code)");
        Ok(())
    }
}
