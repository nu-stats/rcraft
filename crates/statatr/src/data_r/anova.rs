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
pub(super) const ANOVA_HELPER: &str = r##"stata_anova <- function(fit, data, sequential = FALSE, stata_names = NULL, errors = NULL,
                        repeated = NULL, yname = NULL, nested = list()) {
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
    crossed <- setdiff(labels, names(nested))
    if (length(nested) == 0) {
      f3 <- stats::lm(stats::terms(fit), data = data,
                      contrasts = stats::setNames(rep(list("contr.sum"), length(fac)), fac))
      if (anyNA(stats::coef(f3))) {
        stop("the design has an empty cell: Stata still gives partial sums of squares here, but by a rule not matched yet (they are not Type III), so this is not translated; sequential ones are")
      }
      X <- stats::model.matrix(f3)
      assign <- attr(X, "assign")
    } else {
      # a nested term b|a: within each level of a, sum-to-zero contrasts among
      # the levels of b found there; the crossed terms as before
      X <- stats::model.matrix(stats::reformulate(if (length(crossed)) crossed else "1"), data,
                               contrasts.arg = stats::setNames(rep(list("contr.sum"), length(fac)), fac))
      # term numbers of the crossed columns (0, the constant, stays 0)
      assign <- c(0L, match(attr(stats::terms(stats::reformulate(if (length(crossed)) crossed else "1")), "term.labels"), labels))[attr(X, "assign") + 1]
      assign[is.na(assign)] <- 0L
      for (t in names(nested)) {
        inner <- as.vector(unclass(data[[nested[[t]][1]]]))
        cell <- interaction(lapply(nested[[t]][-1], function(v) as.vector(unclass(data[[v]]))), drop = TRUE)
        for (lev in levels(cell)) {
          r <- which(cell == lev)
          il <- sort(unique(inner[r]))
          if (length(il) < 2) next
          C <- stats::contr.sum(length(il))
          M <- matrix(0, nrow(data), ncol(C))
          M[r, ] <- C[match(inner[r], il), , drop = FALSE]
          X <- cbind(X, M)
          assign <- c(assign, rep(match(t, labels), ncol(C)))
        }
      }
      if (qr(X)$rank < ncol(X)) {
        stop("the nested design has an empty cell, whose partial sums of squares are not translated yet")
      }
    }
    b <- drop(solve(crossprod(X), crossprod(X, y)))
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
  # each term against its error term: the residual (0), another term (its
  # index), or none, for an error term Stata does not test (-1)
  if (is.null(errors)) errors <- rep(0L, length(labels))
  nt <- length(labels)
  den_ms <- ifelse(errors > 0, tab$ms[errors + 1], mse)
  den_df <- ifelse(errors > 0, tab$df[errors + 1], dfr)
  Fs <- ifelse(errors < 0, NA_real_, tab$ms[seq_len(nt) + 1] / den_ms)
  tab$F <- c(tab$ms[1] / mse, Fs, NA, NA)
  tab$p <- stats::pf(tab$F, tab$df, c(dfr, den_df, NA, NA), lower.tail = FALSE)
  rownames(tab) <- c("Model", if (is.null(stata_names)) stata_label(labels) else stata_names, "Residual", "Total")
  names(tab) <- c(kind, "df", "MS", "F", "Prob>F")
  cat(sprintf("Number of obs = %d    R-squared     = %.4f\n", n, 1 - rss / tss))
  cat(sprintf("Root MSE      = %.6g    Adj R-squared = %.4f\n\n", sqrt(mse), 1 - mse / (tss / (n - 1))))
  print(tab, digits = 7, na.print = "")
  if (!is.null(repeated)) {
    # the epsilons from the covariance of the repeated measures within the
    # subjects, pooled over the between-subjects groups
    d <- as.data.frame(lapply(data, function(v) as.vector(unclass(v))))
    Y <- tapply(d[[yname]], list(d[[repeated$subject]], d[[repeated$var]]), mean)
    if (anyNA(Y)) stop("repeated(): every subject needs every level of ", repeated$var)
    grp <- if (length(repeated$groups)) {
      interaction(lapply(repeated$groups, function(g) tapply(d[[g]], d[[repeated$subject]], function(v) v[1])), drop = TRUE)
    } else {
      factor(rep(1, nrow(Y)))
    }
    R <- Y
    for (G in levels(grp)) {
      r <- grp == G
      R[r, ] <- sweep(Y[r, , drop = FALSE], 2, colMeans(Y[r, , drop = FALSE]))
    }
    k <- ncol(Y)
    N <- nrow(Y)
    g <- nlevels(grp)
    S <- crossprod(R) / (N - g)
    C <- qr.Q(qr(cbind(1, stats::contr.poly(k))))[, -1, drop = FALSE]
    M <- t(C) %*% S %*% C
    gg <- sum(diag(M))^2 / ((k - 1) * sum(diag(M %*% M)))
    hf <- min(1, (N * (k - 1) * gg - 2) / ((k - 1) * (N - g - (k - 1) * gg)))
    box <- 1 / (k - 1)
    cat(sprintf("\nRepeated variable: %s\n  Huynh-Feldt epsilon        = %.4f\n  Greenhouse-Geisser epsilon = %.4f\n  Box's conservative epsilon = %.4f\n\n",
                repeated$var, hf, gg, box))
    rows <- which(vapply(labels, function(l) repeated$var %in% all.vars(stats::as.formula(paste("~", l))), TRUE))
    adj <- data.frame(df = tab$df[rows + 1], F = tab$F[rows + 1], check.names = FALSE)
    eps <- c(Regular = 1, "H-F" = hf, "G-G" = gg, Box = box)
    for (nm in names(eps)) {
      adj[[nm]] <- stats::pf(adj$F, adj$df * eps[[nm]], den_df[rows] * eps[[nm]], lower.tail = FALSE)
    }
    rownames(adj) <- rownames(tab)[rows + 1]
    print(adj, digits = 4)
    attr(tab, "epsilon") <- c(hf = hf, gg = gg, box = box)
  }
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
        if let Some(o) = opts.other(&[("sequential", 3), ("partial", 4), ("repeated", 3)]) {
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
        // the terms, in groups split by `/` (each group's terms tested against
        // the first term of the next), each with Stata's label and, for a
        // nested term b|a, its inner and outer variables
        struct Term {
            stata: String,
            term: String,
            nested: Option<(String, Vec<String>)>,
        }
        let mut groups: Vec<Vec<Term>> = vec![Vec::new()];
        let mut pieces = Vec::new();
        for w in words {
            let mut rest = w;
            while let Some(i) = rest.find('/') {
                if i > 0 {
                    pieces.push(rest.get(..i).unwrap_or("").to_string());
                }
                pieces.push("/".to_string());
                rest = rest.get(i + 1..).unwrap_or("");
            }
            if !rest.is_empty() {
                pieces.push(rest.to_string());
            }
        }
        for w in &pieces {
            if w == "/" {
                groups.push(Vec::new());
                continue;
            }
            let t = if let Some((inner, outer)) = w.split_once('|') {
                let outer: Vec<String> = outer.split('#').map(str::to_string).collect();
                if !is_name(inner) || outer.iter().any(|v| !is_name(v)) {
                    return Err(no(format!(
                        "anova {w}: only variables written out are translated in a nested term"
                    )));
                }
                let parts: Vec<String> = outer
                    .iter()
                    .chain(std::iter::once(&inner.to_string()))
                    .map(|v| format!("i.{v}"))
                    .collect();
                Term {
                    stata: w.clone(),
                    term: parts.join("#"),
                    nested: Some((inner.to_string(), outer)),
                }
            } else {
                let sep = if w.contains("##") { "##" } else { "#" };
                let parts: Vec<String> = w.split(sep).map(categorical).collect();
                if parts.iter().any(|p| p.contains('#')) {
                    return Err(no(format!(
                        "anova {w}: # and ## in one term is not translated"
                    )));
                }
                Term {
                    stata: w.clone(),
                    term: parts.join(sep),
                    nested: None,
                }
            };
            if let Some(last) = groups.last_mut() {
                last.push(t);
            }
        }
        let slashes = groups.len() > 1;
        let all: Vec<&Term> = groups.iter().flatten().collect();
        if all.is_empty() {
            return Err(no("anova needs at least one term"));
        }
        let special = slashes || all.iter().any(|t| t.nested.is_some());
        if special && all.iter().any(|t| t.term.contains("##")) {
            return Err(no(
                "anova: ## together with / or | is not translated; write the terms out",
            ));
        }
        let spec = Spec::from_terms(dep, all.iter().map(|t| t.term.as_str()))?;
        if special && spec.terms.len() != all.len() {
            return Err(no("anova: these terms with / or | are not translated"));
        }
        // each term's error term: 0 the residual, k the k-th term, -1 none
        let mut errors = Vec::new();
        let mut index = 0usize;
        let starts: Vec<usize> = groups
            .iter()
            .scan(1usize, |n, g| {
                let s = *n;
                *n += g.len();
                Some(s)
            })
            .collect();
        for (i, g) in groups.iter().enumerate() {
            for p in 0..g.len() {
                index += 1;
                let e: i64 = if !slashes {
                    0
                } else if i + 1 < groups.len() {
                    if groups.get(i + 1).is_none_or(Vec::is_empty) {
                        0
                    } else {
                        starts.get(i + 1).copied().unwrap_or(0) as i64
                    }
                } else if p == 0 {
                    -1
                } else {
                    0
                };
                errors.push(e);
            }
        }
        let _ = index;
        // repeated(v): the subjects are the between-subjects error term, the
        // first term of the last group (subj, or subj|grp nested in groups)
        let repeated = match opts.arg("repeated", 3) {
            None => None,
            Some(v) => {
                let v = v.trim();
                if !is_name(v) {
                    return Err(no(
                        "repeated() with more than one variable is not translated yet",
                    ));
                }
                let between = groups
                    .last()
                    .and_then(|g| g.first())
                    .filter(|_| slashes)
                    .ok_or_else(|| {
                        no("repeated() needs the between-subjects error term written after a /")
                    })?;
                let (subject, outer) = match &between.nested {
                    Some((inner, outer)) => (inner.clone(), outer.clone()),
                    None if is_name(&between.stata) => (between.stata.clone(), Vec::new()),
                    None => {
                        return Err(no(format!(
                            "repeated(): the error term {} is not a subject variable",
                            between.stata
                        )))
                    }
                };
                let outer: Vec<String> = outer.iter().map(|o| r_text(o)).collect();
                Some(format!(
                    "list(var = {}, subject = {}, groups = c({}))",
                    r_text(v),
                    r_text(&subject),
                    outer.join(", ")
                ))
            }
        };
        self.helpers_for(&spec);
        let src = self.rows_for(c, out)?;
        // with / or | the terms keep the order written (R would put the main
        // effects first), since each one's error term is named by its place
        if special {
            out.line(format!(
                "stata_estimates <- stats::lm(stats::terms({} ~ {}, keep.order = TRUE), data = {src})",
                r_name(dep),
                spec.right(false)
            ));
        } else {
            out.line(format!(
                "stata_estimates <- stats::lm({} ~ {}, data = {src})",
                r_name(dep),
                spec.right(false)
            ));
        }
        out.line(keep_sample(&src));
        out.line("stata_b <- stats::coef(stata_estimates)");
        out.line("stata_vcov <- stats::vcov(stata_estimates)");
        out.line("stata_df <- stata_estimates$df.residual");
        self.estimates = Some(Estimates {
            fit: Fit::Regress,
            dep: dep.to_string(),
            factors: spec.factors.clone(),
        });
        if special {
            self.e_ret = super::returned::Returned::Lost(
                "e() after anova with error terms, nesting or repeated() is not kept in R yet"
                    .into(),
            );
            self.e_new = true;
        } else {
            self.keep_e(
                c,
                out,
                &super::returned::e_args(dep, "ols", None, &None),
                "",
            );
        }
        if Self::quiet(c) {
            return Ok(());
        }
        self.helpers.insert("stata_anova");
        let mut extra = String::new();
        if special {
            let names: Vec<String> = all.iter().map(|t| r_text(&t.stata)).collect();
            let errs: Vec<String> = errors.iter().map(|e| format!("{e}L")).collect();
            extra.push_str(&format!(
                ", stata_names = c({}), errors = c({})",
                names.join(", "),
                errs.join(", ")
            ));
            let nested: Vec<String> = all
                .iter()
                .zip(&spec.terms)
                .filter_map(|(t, r)| {
                    t.nested.as_ref().map(|(inner, outer)| {
                        let o: Vec<String> = outer.iter().map(|v| r_text(v)).collect();
                        format!("{} = c({}, {})", r_text(r), r_text(inner), o.join(", "))
                    })
                })
                .collect();
            if !nested.is_empty() {
                extra.push_str(&format!(", nested = list({})", nested.join(", ")));
            }
        }
        if let Some(r) = &repeated {
            extra.push_str(&format!(", repeated = {r}, yname = {}", r_text(dep)));
        }
        out.line(format!(
            "stata_anova(stata_estimates, stata_data, sequential = {}{extra})",
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
