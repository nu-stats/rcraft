//! P2B.4, `anova`: Stata's analysis of variance in R.
//!
//! Stata's `anova` is a linear model in which a variable is categorical
//! unless it is written `c.x`, so `anova y g h g#h` is `regress y i.g i.h
//! i.g#i.h` with a different table. The fit is kept with Stata's dummy coding
//! (base level the lowest), as after `regress`, so `test`, `predict` and
//! `margins` work after it the same way.
//!
//! The table is Stata's:
//!
//! - partial sums of squares by default, which are Type III: the same model
//!   refitted with sum-to-zero contrasts, each term's sum of squares being
//!   the Wald statistic for its columns. Checked against Stata 19 itself,
//!   including a continuous variable interacted with a factor, to 15 digits;
//! - with an empty cell Stata still gives partial sums of squares, but by a
//!   rule not matched yet (it is not Type III, Type II, or a reference-coded
//!   model with any choice of base levels), so the R stops and says so;
//! - `sequential` gives Type I, R's own `anova()`, which does not depend on
//!   the coding;
//! - Model is the total less the residual sum of squares, with R-squared,
//!   adjusted R-squared and root MSE above the table.
//!
//! Nested terms (`a|b`) and error terms (`/`) are not translated yet.

use super::model::{keep_sample, Estimates, Fit, Spec};
use super::stats::r_logical;
use super::*;

/// Stata's anova table from a fitted linear model.
pub(super) const ANOVA_HELPER: &str = r##"stata_anova <- function(fit, data, sequential = FALSE) {
  rss <- sum(stats::residuals(fit)^2)
  dfr <- fit$df.residual
  mse <- rss / dfr
  y <- stats::model.response(stats::model.frame(fit))
  n <- length(y)
  tss <- sum((y - mean(y))^2)
  labels <- attr(stats::terms(fit), "term.labels")
  if (sequential) {
    a <- stats::anova(fit)
    ss <- a[["Sum Sq"]][seq_along(labels)]
    df <- a[["Df"]][seq_along(labels)]
    kind <- "Seq. SS"
  } else {
    fac <- names(Filter(is.factor, stats::model.frame(fit)))
    f3 <- stats::lm(stats::formula(fit), data = data,
                    contrasts = stats::setNames(rep(list("contr.sum"), length(fac)), fac))
    if (anyNA(stats::coef(f3))) {
      stop("the design has an empty cell: Stata still gives partial sums of squares here, but by a rule not matched yet (they are not Type III), so this is not translated; sequential ones are")
    }
    X <- stats::model.matrix(f3)
    assign <- attr(X, "assign")
    b <- stats::coef(f3)
    XtXi <- solve(crossprod(X))
    ss <- numeric(length(labels))
    df <- numeric(length(labels))
    for (j in seq_along(labels)) {
      k <- which(assign == j)
      ss[j] <- drop(t(b[k]) %*% solve(XtXi[k, k, drop = FALSE], b[k]))
      df[j] <- length(k)
    }
    kind <- "Partial SS"
  }
  stata_label <- function(s) {
    s <- gsub('stata_ib\\(([^,]+), "[^"]*"\\)', "\\1", s)
    s <- gsub("factor\\(([^)]+)\\)", "\\1", s)
    s <- gsub("I\\(([A-Za-z_][A-Za-z0-9_]*)\\^2\\)", "c.\\1#c.\\1", s)
    gsub(":", "#", s, fixed = TRUE)
  }
  tab <- data.frame(ss = c(tss - rss, ss, rss, tss), df = c(n - 1 - dfr, df, dfr, n - 1))
  tab$ms <- tab$ss / tab$df
  tab$F <- c(tab$ms[seq_len(length(labels) + 1)] / mse, NA, NA)
  tab$p <- stats::pf(tab$F, tab$df, dfr, lower.tail = FALSE)
  rownames(tab) <- c("Model", stata_label(labels), "Residual", "Total")
  names(tab) <- c(kind, "df", "MS", "F", "Prob>F")
  cat(sprintf("Number of obs = %d    R-squared     = %.4f\n", n, 1 - rss / tss))
  cat(sprintf("Root MSE      = %.6g    Adj R-squared = %.4f\n\n", sqrt(mse), 1 - mse / (tss / (n - 1))))
  print(tab, digits = 7, na.print = "")
  invisible(tab)
}"##;

impl Session {
    pub(super) fn anova(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "anova")?;
        if let Some(o) = opts.other(&[("sequential", 3), ("partial", 4)]) {
            return Err(no(format!(
                "anova with the option {o} is not translated yet"
            )));
        }
        let sequential = opts.has("sequential", 3);
        let mut words = c.parts.main.split_whitespace();
        let Some(dep) = words.next() else {
            return Err(no("anova needs a variable to explain"));
        };
        if !is_name(dep) {
            return Err(no(format!("{dep} is not a variable name")));
        }
        let mut terms = Vec::new();
        for w in words {
            if w.contains('|') || w.contains('/') {
                return Err(no(format!(
                    "anova {w}: nested terms (|) and error terms (/) are not translated yet"
                )));
            }
            let sep = if w.contains("##") { "##" } else { "#" };
            let parts: Vec<String> = w.split(sep).map(categorical).collect();
            if parts.iter().any(|p| p.contains('#')) {
                return Err(no(format!(
                    "anova {w}: # and ## in one term is not translated"
                )));
            }
            terms.push(parts.join(sep));
        }
        if terms.is_empty() {
            return Err(no("anova needs at least one term"));
        }
        let spec = Spec::from_terms(dep, terms.iter().map(String::as_str))?;
        self.helpers_for(&spec);
        let src = self.rows_for(c, out)?;
        out.line(format!(
            "stata_estimates <- stats::lm({} ~ {}, data = {src})",
            r_name(dep),
            spec.right(false)
        ));
        out.line(keep_sample(&src));
        out.line("stata_b <- stats::coef(stata_estimates)");
        out.line("stata_vcov <- stats::vcov(stata_estimates)");
        out.line("stata_df <- stata_estimates$df.residual");
        self.estimates = Some(Estimates {
            fit: Fit::Regress,
            dep: dep.to_string(),
            factors: spec.factors.clone(),
        });
        if Self::quiet(c) {
            return Ok(());
        }
        self.helpers.insert("stata_anova");
        out.line(format!(
            "stata_anova(stata_estimates, stata_data, sequential = {})",
            r_logical(sequential)
        ));
        if sequential {
            out.caveat("sequential sums of squares, in the order the terms are written (Type I)");
        } else {
            out.caveat(
                "partial (Type III) sums of squares, as Stata's; a design with an empty cell \
                 stops the R, since Stata's rule there is not matched yet",
            );
        }
        out.caveat("a variable is categorical unless written c.x, as in Stata's anova");
        out.caveat("a row with a missing value anywhere in the model is dropped, as in Stata");
        Ok(())
    }
}

/// A part of an anova term: categorical unless it says otherwise.
fn categorical(p: &str) -> String {
    let p = p.trim();
    let marked =
        p.starts_with("c.") || p.starts_with("i.") || (p.starts_with("ib") && p.contains('.'));
    if marked {
        p.to_string()
    } else {
        format!("i.{p}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anova_variables_are_categorical_unless_marked() {
        assert_eq!(categorical("g"), "i.g");
        assert_eq!(categorical("c.x"), "c.x");
        assert_eq!(categorical("ib2.g"), "ib2.g");
        assert_eq!(categorical("ibx"), "i.ibx");
    }
}
