//! P2B.4, after a model: `predict`, `test`, `lincom` and `estat`.
//!
//! They read what the model left in R (see [`super::model`]): the fit in
//! `stata_estimates`, and Stata's `e(b)`, `e(V)` and `e(df_r)` as `stata_b`,
//! `stata_vcov` and `stata_df`. So `test` after `regress, robust` uses the
//! robust variance, and after `vce(cluster v)` an F with `G - 1` denominator
//! degrees of freedom, as Stata's does.
//!
//! Stata's rules kept here:
//!
//! - `predict` fills in every row that has the values it needs, inside the
//!   estimation sample or not, and stores a float unless told `double`.
//! - `predict, residuals` is the outcome less the linear prediction, in every
//!   such row.
//! - after `ologit` or `mlogit`, one new variable without `outcome()` is the
//!   first outcome's probability, and several are one per outcome in the
//!   order of the outcome values.
//! - `test` is an F test where the model has residual degrees of freedom
//!   (`regress`) and a chi-squared test otherwise.
//! - `estat ic` counts the coefficients as the model's degrees of freedom, as
//!   Stata does; R's `AIC()` on a linear model also counts the variance, so
//!   it is 2 higher.

use super::model::{Estimates, Fit};
use super::stats::r_logical;
use super::*;

/// The probability of each outcome after `ologit`, `oprobit` or `mlogit`,
/// one column per outcome in the order of the outcome values.
pub(super) const OUTCOME_PROBS_HELPER: &str = r#"stata_outcome_probs <- function(fit, newdata, levels) {
  P <- stats::predict(fit, newdata = newdata, type = "probs")
  if (is.null(dim(P))) {
    if (length(levels) == 2 && length(P) == NROW(newdata)) {
      P <- cbind(1 - P, P)
      colnames(P) <- fit$lev
    } else {
      P <- matrix(P, nrow = 1, dimnames = list(NULL, names(P)))
    }
  }
  P[, levels, drop = FALSE]
}"#;

/// Stata's `test`: the Wald test that each row's combination of
/// coefficients equals its value, as an F or a chi-squared.
pub(super) const TEST_HELPER: &str = r#"stata_test <- function(rows, r, labels) {
  res <- stata_resolve(rows, r, labels)
  rows <- res$rows
  r <- res$r
  labels <- res$labels
  b <- stata_b
  R <- matrix(0, length(rows), length(b), dimnames = list(NULL, names(b)))
  for (i in seq_along(rows)) {
    miss <- setdiff(names(rows[[i]]), names(b))
    if (length(miss) > 0) stop(miss[1], " not found")
    R[i, names(rows[[i]])] <- rows[[i]]
  }
  for (i in seq_along(labels)) cat(sprintf(" (%d)  %s\n", i, labels[i]))
  d <- R %*% b - r
  W <- drop(t(d) %*% solve(R %*% stata_vcov %*% t(R), d))
  q <- nrow(R)
  if (is.finite(stata_df)) {
    F <- W / q
    p <- stats::pf(F, q, stata_df, lower.tail = FALSE)
    cat(sprintf("\n       F(%d, %g) = %10.2f\n            Prob > F = %10.4f\n", q, stata_df, F, p))
    invisible(c(F = F, df = q, df_r = stata_df, p = p))
  } else {
    p <- stats::pchisq(W, q, lower.tail = FALSE)
    cat(sprintf("\n           chi2(%d) = %10.2f\n         Prob > chi2 = %10.4f\n", q, W, p))
    invisible(c(chi2 = W, df = q, p = p))
  }
}"#;

/// Stata's `lincom`: one combination of the coefficients, its standard
/// error, test and interval.
pub(super) const LINCOM_HELPER: &str = r#"stata_lincom <- function(a, label, level = 0.95, eform = FALSE, what = "Coef.") {
  a <- stata_resolve(list(a), 0, "")$rows[[1]]
  b <- stata_b
  miss <- setdiff(names(a), names(b))
  if (length(miss) > 0) stop(miss[1], " not found")
  L <- stats::setNames(numeric(length(b)), names(b))
  L[names(a)] <- a
  est <- sum(L * b)
  v <- drop(t(L) %*% stata_vcov %*% L)
  cat(sprintf(" ( 1)  %s = 0\n\n", label))
  stata_coefs(stats::setNames(est, "(1)"), matrix(v, 1, 1), stata_df, level, eform, what)
}"#;

/// `estat ic`, `estat hettest` and `estat ovtest`.
pub(super) const ESTAT_HELPER: &str = r#"stata_estat_ic <- function(fit) {
  ll <- as.numeric(stats::logLik(fit))
  n <- NROW(stats::fitted(fit))
  k <- length(stata_b)
  tab <- data.frame(N = n, "ll(model)" = ll, df = k, AIC = -2 * ll + 2 * k,
                    BIC = -2 * ll + log(n) * k, check.names = FALSE)
  rownames(tab) <- "."
  print(tab, digits = 8)
  invisible(tab)
}
stata_hettest <- function(fit, rhs = FALSE, vars = NULL) {
  e <- stats::residuals(fit)
  z <- e^2 / mean(e^2)
  if (rhs) {
    Z <- stats::model.matrix(fit)
    Z <- Z[, colnames(Z) != "(Intercept)", drop = FALSE]
  } else {
    Z <- NULL
  }
  if (!is.null(vars)) {
    # a varlist: those variables over the estimation sample, with rhs as well
    V <- as.matrix(as.data.frame(lapply(stata_data[vars], function(v) as.numeric(unclass(v)))))
    colnames(V) <- vars
    Z <- if (is.null(Z)) V else cbind(Z, V[, setdiff(vars, colnames(Z)), drop = FALSE])
  }
  if (is.null(Z)) Z <- stats::fitted(fit)
  aux <- stats::lm(z ~ Z)
  chi2 <- sum((stats::fitted(aux) - mean(z))^2) / 2
  df <- NCOL(Z)
  p <- stats::pchisq(chi2, df, lower.tail = FALSE)
  cat("Breusch-Pagan/Cook-Weisberg test for heteroskedasticity\n")
  cat(if (!is.null(vars)) paste0("Variables: ", paste(colnames(Z), collapse = " "), "\n")
      else if (rhs) "Variables: all independent variables\n" else "Variable: Fitted values\n")
  cat("H0: Constant variance (normal error terms)\n\n")
  cat(sprintf("    chi2(%d) = %10.2f\nProb > chi2 = %10.4f\n", df, chi2, p))
  invisible(c(chi2 = chi2, df = df, p = p))
}
stata_ovtest <- function(fit) {
  X <- stats::model.matrix(fit)
  y <- stats::model.response(stats::model.frame(fit))
  # as Stata does: the fitted values rescaled to [0, 1], and each power stored
  # as a float before the regression (which moves F in the sixth digit)
  h <- stats::fitted(fit)
  h <- (h - min(h)) / (max(h) - min(h))
  fl <- function(v) readBin(writeBin(as.numeric(v), raw(), size = 4), "double", size = 4, n = length(v))
  h2 <- fl(h^2)
  h3 <- fl(h^3)
  h4 <- fl(h^4)
  big <- stats::lm(y ~ X + h2 + h3 + h4 + 0)
  rss0 <- sum(stats::residuals(fit)^2)
  rss1 <- sum(stats::residuals(big)^2)
  df2 <- big$df.residual
  F <- ((rss0 - rss1) / 3) / (rss1 / df2)
  p <- stats::pf(F, 3, df2, lower.tail = FALSE)
  cat("Ramsey RESET test for omitted variables\nOmitted: powers of fitted values\n\n")
  cat(sprintf("F(3, %d) = %10.2f\nProb > F = %10.4f\n", df2, F, p))
  invisible(c(F = F, df = 3, df_r = df2, p = p))
}"#;

impl Session {
    fn estimates_for(&self, what: &str) -> Result<Estimates, Untranslated> {
        self.estimates.clone().ok_or_else(|| {
            no(format!(
                "{what} needs a model fitted earlier in this do-file (regress, logit, ...)"
            ))
        })
    }

    pub(super) fn predict(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "predict")?;
        let est = self.estimates_for("predict")?;
        let fit = est.fit;
        let name = fit.name();
        let known: &[(&str, usize)] = match fit {
            Fit::Regress => &[
                ("xb", 2),
                ("residuals", 1),
                ("stdp", 4),
                ("stdf", 4),
                ("stdr", 4),
                ("rstandard", 3),
                ("rstudent", 4),
                ("cooksd", 4),
                ("leverage", 3),
                ("hat", 3),
            ],
            Fit::Xtreg => &[("xb", 2)],
            Fit::Ivregress => &[("xb", 2), ("residuals", 1)],
            Fit::Logit | Fit::Probit => &[("pr", 2), ("xb", 2), ("stdp", 4), ("score", 5)],
            Fit::Poisson => &[("n", 1), ("ir", 2), ("xb", 2), ("stdp", 4), ("score", 5)],
            Fit::Nbreg => &[("n", 1), ("ir", 2), ("xb", 2), ("stdp", 4)],
            Fit::Ologit | Fit::Oprobit | Fit::Mlogit => &[("pr", 2), ("outcome", 3)],
            Fit::Margins => {
                return Err(no(
                    "predict after margins, post is not allowed, as in Stata",
                ))
            }
        };
        // one statistic worked out by stata_predict_stat, if one was asked for
        let stat = [
            ("stdp", 4, "stdp"),
            ("stdf", 4, "stdf"),
            ("stdr", 4, "stdr"),
            ("rstandard", 3, "rstandard"),
            ("rstudent", 4, "rstudent"),
            ("cooksd", 4, "cooksd"),
            ("leverage", 3, "leverage"),
            ("hat", 3, "leverage"),
            ("score", 5, "score"),
            ("ir", 2, "ir"),
        ]
        .into_iter()
        .find(|(o, min, _)| opts.has(o, *min) && known.iter().any(|(k, _)| k == o))
        .map(|(_, _, s)| s);
        if fit == Fit::Regress
            && !self.plain_regress
            && matches!(
                stat,
                Some("stdf" | "stdr" | "rstandard" | "rstudent" | "cooksd" | "leverage")
            )
        {
            return Err(no(
                "predict's residual statistics after robust, clustered or weighted regress are \
                 not translated yet",
            ));
        }
        if let Some(o) = opts.other(known) {
            return Err(no(format!(
                "predict after {name} with the option {o} is not translated yet"
            )));
        }
        if c.options.len() > 1 && !opts.has("outcome", 3) {
            return Err(no("predict takes one statistic at a time"));
        }
        let mut words: Vec<&str> = c.parts.main.split_whitespace().collect();
        let ty = match words.first() {
            Some(t) if storage(t).is_some() => {
                let t = *t;
                words.remove(0);
                t
            }
            _ => "float",
        };
        let st = storage(ty).ok_or_else(|| no(format!("{ty} is not a storage type")))?;
        if st == Storage::Str {
            return Err(no("predict makes a numeric variable, not a string"));
        }
        if words.is_empty() {
            return Err(no("predict needs a new variable name"));
        }
        if let Some(w) = words.iter().find(|w| !is_name(w)) {
            return Err(no(format!(
                "{w}: only plain new variable names are translated (not stub*)"
            )));
        }
        let d = self.data()?;
        let newdata =
            |ty: &str| format!("unname(stats::predict(stata_estimates, newdata = {d}{ty}))");
        let mut values = Vec::new();
        if fit.one_outcome() {
            if words.len() != 1 {
                return Err(no(format!("predict after {name} makes one new variable")));
            }
            if stat.is_some() {
                self.helpers.insert("stata_predict_stat");
            }
            let v = match fit {
                _ if stat.is_some() => format!(
                    "unname(stata_predict_stat(stata_estimates, {d}, {}))",
                    r_text(stat.unwrap_or_default())
                ),
                Fit::Regress if opts.has("residuals", 1) => {
                    format!("{} - {}", r_name(&est.dep), newdata(""))
                }
                Fit::Regress => newdata(""),
                Fit::Xtreg => format!("unname(stata_xt_xb(stata_estimates, {d}))"),
                Fit::Ivregress if opts.has("residuals", 1) => format!(
                    "{} - unname(stata_iv_xb(stata_estimates, {d}))",
                    r_name(&est.dep)
                ),
                Fit::Ivregress => format!("unname(stata_iv_xb(stata_estimates, {d}))"),
                _ if opts.has("xb", 2) => newdata(", type = \"link\""),
                _ => newdata(", type = \"response\""),
            };
            values.push(v);
        } else {
            self.helpers.insert("stata_outcome_probs");
            out.line(format!(
                "stata_p <- stata_outcome_probs(stata_estimates, {d}, stata_levels)"
            ));
            if let Some(o) = opts.arg("outcome", 3) {
                if words.len() != 1 {
                    return Err(no("predict with outcome() makes one new variable"));
                }
                let o = o.trim();
                let col = match o.strip_prefix('#') {
                    Some(k) => match k.trim().parse::<usize>() {
                        Ok(k) if k >= 1 => format!("stata_p[, {k}]"),
                        _ => return Err(no(format!("outcome({o}) is not an outcome number"))),
                    },
                    None if o.parse::<f64>().is_ok() => format!("stata_p[, {}]", r_text(o)),
                    None => {
                        return Err(no(format!(
                            "outcome({o}): only a value or #k is translated, not a label"
                        )))
                    }
                };
                values.push(col);
            } else if words.len() == 1 {
                out.caveat("one new variable and no outcome(): the first outcome's probability, as in Stata");
                values.push("stata_p[, 1]".into());
            } else {
                // Stata's two lines, word for word
                out.line(format!(
                    "if (ncol(stata_p) != {n}) stop(if (ncol(stata_p) > {n}) \"too few\" else \"too many\", \" variables specified\\n{dep} has \", ncol(stata_p), \" outcomes and so you must specify \", ncol(stata_p), \" new variables, or you can use the outcome() option and specify variables one at a time\")",
                    n = words.len(),
                    dep = est.dep
                ));
                values.extend((1..=words.len()).map(|i| format!("stata_p[, {i}]")));
            }
        }
        let cond = self.condition(c, out)?;
        let mut sets = Vec::new();
        for (w, v) in words.iter().zip(&values) {
            out.line(format!(
                "if ({} %in% names({d})) stop(\"variable {w} already defined\")",
                r_text(w)
            ));
            let v = stored(v, st, &mut self.helpers, out);
            let v = match &cond {
                Some(cond) => format!("dplyr::if_else({cond}, {v}, NA_real_)"),
                None => v,
            };
            sets.push(format!("{} = {v}", r_name(w)));
        }
        out.package("dplyr");
        out.line(format!("{d} <- dplyr::mutate({d}, {})", sets.join(", ")));
        if !fit.one_outcome() {
            out.line("rm(stata_p)");
        }
        for w in &words {
            self.kinds.insert(w.to_string(), Kind::Number);
        }
        out.caveat(
            "predicted for every row that has the values it needs, in the estimation sample or \
             not, as Stata does",
        );
        Ok(())
    }

    pub(super) fn test(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "test")?;
        let est = self.estimates_for("test")?;
        if let Some(o) = opts.other(&[]) {
            return Err(no(format!(
                "test with the option {o} is not translated yet"
            )));
        }
        let text = c.parts.main.trim();
        if text.is_empty() {
            return Err(no("test needs coefficients or an equation"));
        }
        let mut rows = Vec::new();
        if text.starts_with('(') {
            for eq in groups(text)? {
                rows.push(equation(&eq, &est)?);
            }
        } else if text.contains('=') {
            rows.push(equation(text, &est)?);
        } else {
            // `test x w`: each coefficient is zero, jointly; after mlogit a
            // name without an equation is in every equation, as in Stata
            for w in text.split_whitespace() {
                let mut row = equation(w, &est)?;
                row.coefs = rename_any(row.coefs, "stata:all:");
                rows.push(row);
            }
        }
        if rows
            .iter()
            .any(|r| r.coefs.keys().any(|k| k.starts_with("stata:any:")))
        {
            return Err(no(
                "after mlogit, name the equation of each coefficient in an equation, as [2]x",
            ));
        }
        let list: Vec<String> = rows.iter().map(|r| r.r_vector()).collect();
        let values: Vec<String> = rows.iter().map(|r| r_number(r.value)).collect();
        let labels: Vec<String> = rows.iter().map(|r| r_text(&r.label)).collect();
        self.helpers.insert("stata_test");
        self.helpers.insert("stata_names");
        out.line(format!(
            "stata_test(list({}), c({}), c({}))",
            list.join(", "),
            values.join(", "),
            labels.join(", ")
        ));
        Ok(())
    }

    pub(super) fn lincom(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "lincom")?;
        let est = self.estimates_for("lincom")?;
        if let Some(o) = opts.other(&[
            ("level", 3),
            ("or", 2),
            ("irr", 3),
            ("rrr", 3),
            ("eform", 5),
        ]) {
            return Err(no(format!(
                "lincom with the option {o} is not translated yet"
            )));
        }
        let text = c.parts.main.trim();
        if text.is_empty() || text.contains('=') {
            return Err(no(
                "lincom needs one combination of coefficients, without =",
            ));
        }
        let (coefs, constant) = linear(text, &est)?;
        // after mlogit a name without an equation is the first equation's, as in Stata
        let coefs = rename_any(coefs, "stata:first:");
        if constant != 0.0 {
            return Err(no("lincom takes coefficients only, not a constant"));
        }
        if coefs.is_empty() {
            return Err(no("lincom needs at least one coefficient"));
        }
        let level = super::model::level(opts)?;
        let (eform, what) = if opts.has("or", 2) {
            (true, "Odds ratio")
        } else if opts.has("irr", 3) {
            (true, "IRR")
        } else if opts.has("rrr", 3) {
            (true, "RRR")
        } else if opts.has("eform", 5) {
            (true, "exp(b)")
        } else {
            (false, "Coef.")
        };
        self.helpers.insert("stata_coefs");
        self.helpers.insert("stata_lincom");
        self.helpers.insert("stata_names");
        let row = Row {
            coefs,
            value: 0.0,
            label: String::new(),
        };
        out.line(format!(
            "stata_lincom({}, label = {}, level = {}, eform = {}, what = {})",
            row.r_vector(),
            r_text(&tidy(text)),
            r_number(level),
            r_logical(eform),
            r_text(what)
        ));
        Ok(())
    }

    pub(super) fn estat(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "estat")?;
        if self.weighted_fit {
            return Err(no("estat after a model with weights is not translated yet"));
        }
        let est = self.estimates_for("estat")?;
        let (sub, rest) = first_word(&c.parts.main);
        let is = |full: &str, min: usize| sub.len() >= min && full.starts_with(sub);
        let regress_only = |what: &str| -> Result<(), Untranslated> {
            if est.fit == Fit::Regress {
                Ok(())
            } else {
                Err(no(format!(
                    "estat {what} after {} is not translated (Stata has it after regress)",
                    est.fit.name()
                )))
            }
        };
        if sub.is_empty() {
            return Err(no("estat needs a subcommand (ic, vce, hettest, ovtest)"));
        }
        if !rest.is_empty() && !(sub.len() >= 4 && "hettest".starts_with(sub)) {
            return Err(no(format!(
                "estat {sub} with a varlist is not translated yet"
            )));
        }
        if est.fit == Fit::Ivregress && sub != "vce" {
            return self.estat_iv(sub, out);
        }
        if sub == "ic" && est.fit == Fit::Xtreg {
            return Err(no("estat ic after xtreg is not translated yet"));
        }
        if sub == "ic" {
            if let Some(o) = opts.other(&[]) {
                return Err(no(format!(
                    "estat ic with the option {o} is not translated yet"
                )));
            }
            self.helpers.insert("stata_estat");
            out.line("stata_estat_ic(stata_estimates)");
            out.caveat(
                "the degrees of freedom are the coefficients, as Stata counts them; R's AIC() on \
                 a linear model also counts the variance",
            );
        } else if sub == "vce" {
            if let Some(o) = opts.other(&[("correlation", 4)]) {
                return Err(no(format!(
                    "estat vce with the option {o} is not translated yet"
                )));
            }
            if opts.has("correlation", 4) {
                out.line("print(stats::cov2cor(stata_vcov))");
            } else {
                out.line("print(stata_vcov)");
            }
        } else if is("hettest", 4) {
            regress_only("hettest")?;
            if let Some(o) = opts.other(&[("rhs", 3)]) {
                return Err(no(format!(
                    "estat hettest with the option {o} is not translated yet"
                )));
            }
            let vars: Vec<&str> = rest.split_whitespace().collect();
            if let Some(v) = vars.iter().find(|v| !is_name(v)) {
                return Err(no(format!(
                    "estat hettest {v}: only variable names written out are translated"
                )));
            }
            let vars = if vars.is_empty() {
                String::new()
            } else {
                let v: Vec<String> = vars.iter().map(|v| r_text(v)).collect();
                format!(", vars = c({})", v.join(", "))
            };
            self.helpers.insert("stata_estat");
            out.line(format!(
                "stata_hettest(stata_estimates, rhs = {}{vars})",
                r_logical(opts.has("rhs", 3))
            ));
        } else if is("ovtest", 3) {
            regress_only("ovtest")?;
            if let Some(o) = opts.other(&[]) {
                return Err(no(format!(
                    "estat ovtest with the option {o} is not translated yet"
                )));
            }
            self.helpers.insert("stata_estat");
            out.line("stata_ovtest(stata_estimates)");
        } else {
            return Err(no(format!("estat {sub} is not translated yet")));
        }
        Ok(())
    }
}

/// One equation of `test`: coefficients (by R's names) times weights, equal
/// to a value.
struct Row {
    coefs: BTreeMap<String, f64>,
    value: f64,
    label: String,
}

impl Row {
    /// `c("x" = 1, "w" = -1)`
    fn r_vector(&self) -> String {
        let parts: Vec<String> = self
            .coefs
            .iter()
            .map(|(n, w)| format!("{} = {}", r_text(n), r_number(*w)))
            .collect();
        format!("c({})", parts.join(", "))
    }
}

/// `(x = 0) (w = 1)` as its groups.
fn groups(text: &str) -> Result<Vec<String>, Untranslated> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut cur = String::new();
    for ch in text.chars() {
        match ch {
            '(' => {
                if depth > 0 {
                    cur.push(ch);
                }
                depth += 1;
            }
            ')' => {
                if depth == 0 {
                    return Err(no("a ) without its ("));
                }
                depth -= 1;
                if depth == 0 {
                    out.push(std::mem::take(&mut cur));
                } else {
                    cur.push(ch);
                }
            }
            c if depth == 0 && !c.is_whitespace() => {
                return Err(no(
                    "test with equations in parentheses takes nothing between them",
                ))
            }
            c => {
                if depth > 0 {
                    cur.push(c);
                }
            }
        }
    }
    if depth != 0 {
        return Err(no("a ( without its )"));
    }
    Ok(out)
}

/// `lhs = rhs` (or a lone coefficient, meaning `= 0`) as one row.
fn equation(text: &str, est: &Estimates) -> Result<Row, Untranslated> {
    let (lhs, rhs) = match text.split_once('=') {
        Some((l, r)) => (l, r),
        None => (text, "0"),
    };
    if rhs.contains('=') {
        return Err(no(format!("{text}: one = only")));
    }
    let (mut coefs, lc) = linear(lhs, est)?;
    let (right, rc) = linear(rhs, est)?;
    for (n, w) in right {
        *coefs.entry(n).or_insert(0.0) -= w;
    }
    coefs.retain(|_, w| *w != 0.0);
    if coefs.is_empty() {
        return Err(no(format!("{text}: no coefficient is left to test")));
    }
    let label = format!(
        "{} = {}",
        tidy(lhs),
        if text.contains('=') {
            tidy(rhs)
        } else {
            "0".into()
        }
    );
    Ok(Row {
        coefs,
        value: rc - lc,
        label,
    })
}

/// `stata:any:x` (a name without an equation, after mlogit) as `to` says.
fn rename_any(coefs: BTreeMap<String, f64>, to: &str) -> BTreeMap<String, f64> {
    coefs
        .into_iter()
        .map(|(k, w)| match k.strip_prefix("stata:any:") {
            Some(n) => (format!("{to}{n}"), w),
            None => (k, w),
        })
        .collect()
}

/// Spaces made single, for the lines `test` and `lincom` print.
fn tidy(t: &str) -> String {
    t.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A sum of terms like `2*x`, `-w`, `_cons` and `0.5` as coefficient weights
/// (by R's names) and a constant.
fn linear(text: &str, est: &Estimates) -> Result<(BTreeMap<String, f64>, f64), Untranslated> {
    let mut coefs = BTreeMap::new();
    let mut constant = 0.0;
    for (sign, t) in signed_terms(text)? {
        let t = t.trim();
        let (k, name) = match t.split_once('*') {
            Some((a, b)) => match (a.trim().parse::<f64>(), b.trim().parse::<f64>()) {
                (Ok(k), Err(_)) => (k, Some(b.trim())),
                (Err(_), Ok(k)) => (k, Some(a.trim())),
                _ => {
                    return Err(no(format!(
                        "{t}: only a number times a coefficient is translated"
                    )))
                }
            },
            None => match t.parse::<f64>() {
                Ok(k) => (k, None),
                Err(_) => (1.0, Some(t)),
            },
        };
        if !k.is_finite() {
            return Err(no(format!("{t} is not a number")));
        }
        match name {
            Some(n) => *coefs.entry(coef_name(n, est)?).or_insert(0.0) += sign * k,
            None => constant += sign * k,
        }
    }
    Ok((coefs, constant))
}

/// Splits at `+` and `-` (not those inside a number such as `1e-3`).
fn signed_terms(text: &str) -> Result<Vec<(f64, String)>, Untranslated> {
    let mut out = Vec::new();
    let mut sign = 1.0;
    let mut signed = false;
    let mut cur = String::new();
    for ch in text.chars() {
        let in_exponent = (ch == '-' || ch == '+')
            && cur.ends_with(['e', 'E'])
            && cur
                .trim()
                .trim_end_matches(['e', 'E'])
                .parse::<f64>()
                .is_ok();
        if (ch == '+' || ch == '-') && !in_exponent {
            if cur.trim().is_empty() {
                if signed || !out.is_empty() {
                    return Err(no(format!("{text}: two signs in a row")));
                }
            } else {
                out.push((sign, std::mem::take(&mut cur)));
            }
            sign = if ch == '-' { -1.0 } else { 1.0 };
            signed = true;
            cur.clear();
        } else {
            if !ch.is_whitespace() {
                signed = false;
            }
            cur.push(ch);
        }
    }
    if cur.trim().is_empty() {
        return Err(no(format!(
            "`{}` is not a combination of coefficients",
            text.trim()
        )));
    }
    out.push((sign, cur));
    Ok(out)
}

/// A coefficient as Stata names it (`x`, `_cons`, `1.g`, `_b[x]`) by its
/// name among R's coefficients.
fn coef_name(t: &str, est: &Estimates) -> Result<String, Untranslated> {
    let t = t.trim();
    if est.fit == Fit::Margins {
        // after margins, post the names are Stata's own (1.g, x, 1._at)
        let n = t
            .strip_prefix("_b[")
            .and_then(|r| r.strip_suffix(']'))
            .unwrap_or(t);
        return Ok(format!("stata:{}", n.replace(' ', "")));
    }
    // `[2]x` and `[2]_b[x]` are `2:x`, as `_b[2:x]` is
    let owned;
    let t = match t.strip_prefix('[').and_then(|r| r.split_once(']')) {
        Some((eq, rest)) if !rest.trim().is_empty() => {
            let rest = rest.trim();
            let rest = rest
                .strip_prefix("_b[")
                .and_then(|r| r.strip_suffix(']'))
                .unwrap_or(rest);
            owned = format!("{}:{}", eq.trim(), rest.trim());
            owned.as_str()
        }
        Some((eq, _)) => {
            // `test [3]`: the whole equation
            return if est.fit == Fit::Mlogit {
                Ok(format!("stata:eq:{}", eq.trim()))
            } else {
                Err(no(format!(
                    "[{}]: {} has one equation",
                    eq.trim(),
                    est.fit.name()
                )))
            };
        }
        None => t
            .strip_prefix("_b[")
            .and_then(|r| r.strip_suffix(']'))
            .unwrap_or(t)
            .trim(),
    };
    let one_name = |s: &str| {
        s.split('#').all(|p| {
            let p = p.trim();
            let p = p.strip_prefix("c.").unwrap_or(p);
            is_name(p)
                || p.split_once('.').is_some_and(|(l, v)| {
                    !l.is_empty() && l.chars().all(|c| c.is_ascii_digit()) && is_name(v)
                })
        })
    };
    if let Some((eq, name)) = t.split_once(':') {
        if est.fit != Fit::Mlogit {
            return Err(no(format!("{t}: {} has one equation", est.fit.name())));
        }
        let name = name.trim();
        if !(name == "_cons" || one_name(name)) || eq.trim().parse::<f64>().is_err() {
            return Err(no(format!("{t} is not a coefficient name")));
        }
        return Ok(format!("stata:{}:{name}", eq.trim()));
    }
    // ologit's and oprobit's cut points are named as Stata names them
    if let Some(k) = t.strip_prefix("/cut") {
        if matches!(est.fit, Fit::Ologit | Fit::Oprobit) && k.parse::<usize>().is_ok() {
            return Ok(t.to_string());
        }
        return Err(no(format!("{t}: not a coefficient of {}", est.fit.name())));
    }
    if est.fit == Fit::Mlogit {
        // a name without an equation: which ones depends on the command
        return if t == "_cons" || one_name(t) {
            Ok(format!("stata:any:{t}"))
        } else {
            Err(no(format!("{t} is not a coefficient name")))
        };
    }
    if t == "_cons" {
        return Ok("(Intercept)".into());
    }
    if t.contains('#') {
        // looked up among R's coefficients once the model has run
        return if one_name(t) {
            Ok(format!("stata:{}", t.replace(' ', "")))
        } else {
            Err(no(format!("{t} is not a coefficient name")))
        };
    }
    if let Some(v) = t.strip_prefix("c.") {
        return if is_name(v) {
            Ok(v.to_string())
        } else {
            Err(no(format!("{v} is not a variable name")))
        };
    }
    if let Some((lvl, v)) = t.split_once('.') {
        if lvl.is_empty() || !lvl.chars().all(|c| c.is_ascii_digit()) {
            return Err(no(format!(
                "{t}: only a level number before the dot is translated"
            )));
        }
        let expr = est.factors.get(v).ok_or_else(|| {
            no(format!(
                "{t}: {v} is not in the model as a factor variable (i.{v})"
            ))
        })?;
        return Ok(format!("{expr}{lvl}"));
    }
    if is_name(t) {
        Ok(t.to_string())
    } else {
        Err(no(format!("{t} is not a coefficient name")))
    }
}

/// `predict`'s other statistics, for every row with the values they need
/// (in the estimation sample or not, as Stata does), decoded in Stata 19:
/// `stdp` is `sqrt(x V x')` with the model's own variance; after `regress`,
/// with `s` its root MSE and `h = x (X'X)^-1 x'`, `stdf = sqrt(stdp^2 + s^2)`,
/// `stdr = s sqrt(1 - h)`, `rstandard = e / stdr`, `rstudent` the same with
/// `s` worked out without the row, `cooksd = e^2 h / (k s^2 (1 - h)^2)` and
/// `leverage = h`; after logit, probit and poisson `score` is the derivative
/// of the row's log likelihood by its linear prediction, and `ir` is
/// `exp(xb)` without the exposure.
pub(super) const PREDICT_STAT_HELPER: &str = r#"stata_predict_stat <- function(fit, newdata, stat) {
  tt <- stats::delete.response(stats::terms(fit))
  mf <- stats::model.frame(tt, newdata, na.action = stats::na.pass, xlev = fit$xlevels)
  X <- stats::model.matrix(tt, mf)
  b <- stats::coef(fit)
  keep <- !is.na(b)
  X <- X[, names(b)[keep], drop = FALSE]
  xb <- drop(X %*% b[keep])
  V <- stata_vcov[colnames(X), colnames(X), drop = FALSE]
  stdp <- sqrt(rowSums((X %*% V) * X))
  if (stat == "stdp") return(stdp)
  if (stat == "ir") return(exp(xb))
  off <- stats::model.offset(mf)
  eta <- xb + if (is.null(off)) 0 else off
  y <- eval(stats::formula(fit)[[2]], newdata, environment(stats::formula(fit)))
  if (stat == "score") {
    link <- fit$family$link
    if (link == "logit") return(y - stats::plogis(eta))
    if (link == "probit") {
      P <- stats::pnorm(eta)
      f <- stats::dnorm(eta)
      return(ifelse(y == 1, f / P, -f / (1 - P)))
    }
    return(y - exp(eta))
  }
  Xf <- stats::model.matrix(fit)[, colnames(X), drop = FALSE]
  k <- ncol(Xf)
  n <- nrow(Xf)
  rss <- sum(stats::residuals(fit)^2)
  s2 <- rss / (n - k)
  h <- rowSums((X %*% solve(crossprod(Xf))) * X)
  e <- y - eta
  switch(stat,
    stdf = sqrt(stdp^2 + s2),
    stdr = sqrt(s2 * (1 - h)),
    rstandard = e / sqrt(s2 * (1 - h)),
    rstudent = e / sqrt((rss - e^2 / (1 - h)) / (n - k - 1) * (1 - h)),
    cooksd = e^2 * h / (k * s2 * (1 - h)^2),
    leverage = h
  )
}"#;

/// Stata's names for R's coefficients, so that `test` and `lincom` can find
/// the ones the translation cannot name before the model runs: interactions
/// (`1.g#c.x`, in either order, `c.` optional, as Stata takes them), squares
/// (`c.x#c.x`) and mlogit's equations (`[2]x`, `[3]_cons`, `_b[2:x]`). A key
/// `stata:...` in a row is looked up here; `stata:all:x` is `x` in every
/// equation but the base (mlogit's `test x`), `stata:eq:3` every coefficient
/// of equation 3 but its constant (`test [3]`), `stata:first:x` `x` in the
/// first equation (`lincom x`), as Stata 19 reads them.
pub(super) const NAMES_HELPER: &str = r##"stata_canon <- function(s) {
  eq <- ""
  if (grepl(":", s, fixed = TRUE)) {
    eq <- paste0(sub(":.*$", "", s), ":")
    s <- sub("^[^:]*:", "", s)
  }
  p <- strsplit(gsub(" ", "", s), "#", fixed = TRUE)[[1]]
  # Stata writes a base level as 0bn.g and an omitted one as 0o.g; both are 0.g
  p <- sub("^([0-9]+)(bn|b|o)\\.", "\\1.", sub("^c\\.", "", p))
  paste0(eq, paste(sort(p), collapse = "#"))
}
stata_bnames <- function() {
  part <- function(p) {
    if (p == "(Intercept)") return("_cons")
    m <- regmatches(p, regexec("^factor\\(([^()]+)\\)(.+)$", p))[[1]]
    if (length(m)) return(paste0(m[3], ".", m[2]))
    m <- regmatches(p, regexec("^stata_ib\\(([^,]+), \"[^\"]*\"\\)(.+)$", p))[[1]]
    if (length(m)) return(paste0(m[3], ".", m[2]))
    m <- regmatches(p, regexec("^I\\(([^()^]+)\\^([0-9]+)\\)$", p))[[1]]
    if (length(m)) return(paste(rep(m[2], as.integer(m[3])), collapse = "#"))
    p
  }
  multi <- inherits(stata_estimates, "multinom")
  vapply(names(stata_b), function(n) {
    eq <- ""
    if (multi) {
      eq <- paste0(sub(":.*$", "", n), ":")
      n <- sub("^[^:]*:", "", n)
    }
    stata_canon(paste0(eq, paste(vapply(strsplit(n, ":", fixed = TRUE)[[1]], part, ""), collapse = "#")))
  }, "")
}
stata_resolve <- function(rows, r, labels) {
  st <- stata_bnames()
  nm <- names(stata_b)
  multi <- inherits(stata_estimates, "multinom")
  eqs <- if (multi) unique(sub(":.*$", "", nm)) else character(0)
  find <- function(k) {
    if (multi && grepl(":", k, fixed = TRUE) && sub(":.*$", "", k) == stata_base) {
      return(NA_character_)  # Stata takes it as a zero
    }
    i <- match(stata_canon(k), st)
    if (is.na(i)) stop(k, " not found")
    nm[i]
  }
  out <- list(rows = list(), r = numeric(0), labels = character(0))
  add <- function(row, v, l) {
    out$rows[[length(out$rows) + 1]] <<- row
    out$r <<- c(out$r, v)
    out$labels <<- c(out$labels, l)
  }
  for (i in seq_along(rows)) {
    row <- rows[[i]]
    keys <- names(row)
    if (length(row) == 1 && startsWith(keys, "stata:all:")) {
      v <- sub("^stata:all:", "", keys)
      for (e in eqs) add(stats::setNames(row[[1]], find(paste0(e, ":", v))), r[i], paste0("[", e, "]", v, " = 0"))
      next
    }
    if (length(row) == 1 && startsWith(keys, "stata:eq:")) {
      e <- sub("^stata:eq:", "", keys)
      if (multi && e == stata_base) stop("[", e, "] is the base outcome's equation, which Stata fixes at zero")
      these <- nm[startsWith(nm, paste0(e, ":")) & !endsWith(nm, ":(Intercept)")]
      if (length(these) == 0) stop("equation [", e, "] not found")
      for (n in these) add(stats::setNames(1, n), 0, paste0("[", e, "]", sub("^[^:]*:", "", n), " = 0"))
      next
    }
    for (j in seq_along(keys)) {
      k <- keys[j]
      if (startsWith(k, "stata:first:")) {
        lv <- as.character(sort(as.numeric(stata_estimates$lev)))
        keys[j] <- find(paste0(lv[1], ":", sub("^stata:first:", "", k)))
      } else if (startsWith(k, "stata:")) {
        keys[j] <- find(sub("^stata:", "", k))
      }
    }
    keep <- !is.na(keys)
    row <- row[keep]
    names(row) <- keys[keep]
    if (length(row) == 0) {
      stop("[", stata_base, "] is the base outcome's equation, which Stata fixes at zero, so nothing is left to test")
    }
    add(row, r[i], labels[i])
  }
  out
}"##;

#[cfg(test)]
mod tests {
    use super::*;

    fn est() -> Estimates {
        let mut factors = BTreeMap::new();
        factors.insert("g".to_string(), "factor(g)".to_string());
        Estimates {
            fit: Fit::Regress,
            dep: "y".into(),
            factors,
        }
    }

    #[test]
    fn statas_coefficient_names_become_rs() {
        let e = est();
        assert_eq!(coef_name("x", &e).as_deref(), Ok("x"));
        assert_eq!(coef_name("_cons", &e).as_deref(), Ok("(Intercept)"));
        assert_eq!(coef_name("_b[x]", &e).as_deref(), Ok("x"));
        assert_eq!(coef_name("1.g", &e).as_deref(), Ok("factor(g)1"));
        assert!(
            coef_name("1.h", &e).is_err(),
            "h is not a factor in the model"
        );
        // an interaction is looked up among R's coefficients once the model has run
        assert_eq!(coef_name("1.g#c.x", &e).as_deref(), Ok("stata:1.g#c.x"));
        assert!(coef_name("[2]x", &e).is_err(), "regress has one equation");
    }

    #[test]
    fn a_combination_is_weights_and_a_constant() {
        let e = est();
        let (c, k) = linear("2*x - w + 0.5", &e).unwrap_or_default();
        assert_eq!(c.get("x"), Some(&2.0));
        assert_eq!(c.get("w"), Some(&-1.0));
        assert!((k - 0.5).abs() < 1e-12);
        let (c, _) = linear("-x + x*3", &e).unwrap_or_default();
        assert_eq!(c.get("x"), Some(&2.0));
        let (_, k) = linear("1e-3", &e).unwrap_or_default();
        assert!((k - 0.001).abs() < 1e-15, "an exponent is not a minus sign");
        assert!(linear("x +", &e).is_err());
    }

    #[test]
    fn an_equation_moves_everything_to_one_side() {
        let e = est();
        let r = equation("x = w", &e).unwrap_or_else(|_| Row {
            coefs: BTreeMap::new(),
            value: f64::NAN,
            label: String::new(),
        });
        assert_eq!(r.coefs.get("x"), Some(&1.0));
        assert_eq!(r.coefs.get("w"), Some(&-1.0));
        assert_eq!(r.value, 0.0);
        assert_eq!(r.label, "x = w");
        let r = equation("x + 1 = 3", &e).map(|r| r.value);
        assert_eq!(r, Ok(2.0));
        assert!(equation("x = x", &e).is_err(), "nothing left to test");
        assert_eq!(
            groups("(x = 0) (w = 1)").map(|g| g.len()),
            Ok(2),
            "two equations"
        );
        assert!(groups("(x = 0) w").is_err());
    }
}
