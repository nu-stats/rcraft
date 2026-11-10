//! `estpost summarize` (Ben Jann's estout package, SSC), as observed in
//! Stata 19 on made-up data (`plan/stata/esttab-options.md`); the add-on's
//! program was not read.
//!
//! - For each variable, over its nonmissing values in the rows of `if`/`in`:
//!   e(count), e(sum_w) (the same count, without weights), e(mean), e(Var),
//!   e(sd), e(min), e(max) and e(sum), each a 1 x k matrix with the
//!   variables' names as its columns (rows unnamed). e(N) is the number of
//!   rows where any of them is nonmissing; e(k) the number of variables;
//!   e(cmd) "estpost", e(subcmd) "summarize". No e(b); r() is cleared.
//! - The table: e(name) heads columns of 10 after a 12-wide name column
//!   (names cut as Stata cuts them), numbers in %9.0g; as many columns as
//!   the linesize takes, the rest in a block below, a blank line before each.
//! - No observations: "no observations", r(2000).

use super::*;

pub(super) const ESTPOST_HELPER: &str = r##"stata_estpost_summarize <- function(d, print = TRUE) {
  vars <- names(d)
  for (v in vars) {
    if (is.character(d[[v]])) stop("RCraft: estpost summarize of a string variable (", v, ") is not translated yet", call. = FALSE)
  }
  X <- lapply(vars, function(v) as.numeric(unclass(d[[v]])))
  seen <- Reduce(`|`, lapply(X, function(x) !is.na(x)), rep(FALSE, nrow(d)))
  N <- sum(seen)
  if (N == 0) stop("no observations (Stata: r(2000))", call. = FALSE)
  st <- c("count", "sum_w", "mean", "Var", "sd", "min", "max", "sum")
  vals <- vapply(seq_along(X), function(j) {
    x <- X[[j]][!is.na(X[[j]])]
    n <- length(x)
    if (n == 0) stop("RCraft: estpost summarize of a variable with no observations (", vars[j], ") is not translated yet", call. = FALSE)
    m <- mean(x)
    v <- if (n > 1) sum((x - m)^2) / (n - 1) else NA
    c(n, n, m, v, sqrt(v), min(x), max(x), sum(x))
  }, numeric(8))
  e <- list(N = N, k = length(vars), cmd = "estpost", subcmd = "summarize")
  for (i in seq_along(st)) e[[st[i]]] <- matrix(vals[i, ], 1, dimnames = list(NULL, vars))
  attr(e, "stata_sample") <- FALSE
  if (print) {
    ls <- if (exists("stata_linesize")) stata_linesize else 80
    ab <- function(s) ifelse(nchar(s) > 12, paste0(substr(s, 1, 10), "~", substr(s, nchar(s), nchar(s))), s)
    per <- max(1, (ls - 15) %/% 11)
    g <- function(x) if (is.na(x)) "." else stata_fmt(x, "%9.0g", trim = TRUE)
    for (b in split(seq_along(st), ceiling(seq_along(st) / per))) {
      cat("\n")
      cat(strrep(" ", 13), "|", paste0(formatC(paste0("e(", st[b], ")"), width = 10), " ", collapse = ""), "\n", sep = "")
      cat(strrep("-", 13), "+", strrep("-", 11 * length(b)), "\n", sep = "")
      for (j in seq_along(vars)) {
        cat(formatC(ab(vars[j]), width = 12), " |",
            paste0(formatC(vapply(vals[b, j], g, ""), width = 10), " ", collapse = ""), "\n", sep = "")
      }
    }
  }
  e
}"##;

impl Session {
    /// `estpost summarize varlist [if] [in]`.
    pub(super) fn estpost(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "estpost")?;
        let main = c.parts.main.trim();
        let (sub, rest) = main.split_once(char::is_whitespace).unwrap_or((main, ""));
        if sub != "summarize" {
            return Err(no(format!(
                "estpost {sub} is not translated yet (estpost summarize is)"
            )));
        }
        if let Some(o) = opts.other(&[]) {
            return Err(no(format!(
                "estpost summarize with the option {o} is not translated yet"
            )));
        }
        if c.parts.weight.is_some() {
            return Err(no("estpost summarize with weights is not translated yet"));
        }
        if rest.trim().is_empty() {
            return Err(no(
                "estpost summarize without a varlist is not translated yet",
            ));
        }
        let mut vars = c.clone();
        vars.parts.main = rest.trim().to_string();
        let frame = self.frame_for(&vars, out)?;
        self.helpers.insert("stata_estpost");
        self.helpers.insert("stata_fmt");
        out.line(format!(
            "stata_e <- stata_estpost_summarize({frame}{})",
            if self.quiet(c) { ", print = FALSE" } else { "" }
        ));
        // what it posts is read off stata_e; no model is left for predict
        self.e_ret = super::returned::Returned::Program {
            cmd: "estpost".into(),
            texts: vec!["cmd".into(), "subcmd".into()],
        };
        self.estimates = None;
        self.esample = false;
        out.caveat(
            "estpost summarize as the estout package posts and prints it, matched to its output \
             (not its code)",
        );
        Ok(())
    }
}
