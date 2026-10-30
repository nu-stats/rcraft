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
stata_hettest <- function(fit, rhs = FALSE) {
  e <- stats::residuals(fit)
  z <- e^2 / mean(e^2)
  if (rhs) {
    Z <- stats::model.matrix(fit)
    Z <- Z[, colnames(Z) != "(Intercept)", drop = FALSE]
  } else {
    Z <- stats::fitted(fit)
  }
  aux <- stats::lm(z ~ Z)
  chi2 <- sum((stats::fitted(aux) - mean(z))^2) / 2
  df <- NCOL(Z)
  p <- stats::pchisq(chi2, df, lower.tail = FALSE)
  cat("Breusch-Pagan/Cook-Weisberg test for heteroskedasticity\n")
  cat(if (rhs) "Variables: all independent variables\n" else "Variable: Fitted values\n")
  cat("H0: Constant variance (normal error terms)\n\n")
  cat(sprintf("    chi2(%d) = %10.2f\nProb > chi2 = %10.4f\n", df, chi2, p))
  invisible(c(chi2 = chi2, df = df, p = p))
}
stata_ovtest <- function(fit) {
  X <- stats::model.matrix(fit)
  y <- stats::model.response(stats::model.frame(fit))
  h <- stats::fitted(fit)
  big <- stats::lm(y ~ X + I(h^2) + I(h^3) + I(h^4) + 0)
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
            Fit::Regress => &[("xb", 2), ("residuals", 1)],
            Fit::Logit | Fit::Probit => &[("pr", 2), ("xb", 2)],
            Fit::Poisson | Fit::Nbreg => &[("n", 1), ("xb", 2)],
            Fit::Ologit | Fit::Oprobit | Fit::Mlogit => &[("pr", 2), ("outcome", 3)],
        };
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
            let v = match fit {
                Fit::Regress if opts.has("residuals", 1) => {
                    format!("{} - {}", r_name(&est.dep), newdata(""))
                }
                Fit::Regress => newdata(""),
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
                out.line(format!(
                    "if (ncol(stata_p) != {n}) stop(\"you must specify \", ncol(stata_p), \" new variables\")",
                    n = words.len()
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
        if !est.fit.one_outcome() {
            return Err(no(format!(
                "test after {} is not translated yet (its coefficients are in several equations)",
                est.fit.name()
            )));
        }
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
            // `test x w`: each coefficient is zero, jointly
            for w in text.split_whitespace() {
                rows.push(equation(w, &est)?);
            }
        }
        let list: Vec<String> = rows.iter().map(|r| r.r_vector()).collect();
        let values: Vec<String> = rows.iter().map(|r| r_number(r.value)).collect();
        let labels: Vec<String> = rows.iter().map(|r| r_text(&r.label)).collect();
        self.helpers.insert("stata_test");
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
        if !est.fit.one_outcome() {
            return Err(no(format!(
                "lincom after {} is not translated yet (its coefficients are in several equations)",
                est.fit.name()
            )));
        }
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
        if !rest.is_empty() {
            return Err(no(format!(
                "estat {sub} with a varlist is not translated yet"
            )));
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
            self.helpers.insert("stata_estat");
            out.line(format!(
                "stata_hettest(stata_estimates, rhs = {})",
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
    let t = t
        .strip_prefix("_b[")
        .and_then(|r| r.strip_suffix(']'))
        .unwrap_or(t)
        .trim();
    if t == "_cons" {
        return Ok("(Intercept)".into());
    }
    if t.contains('#') {
        return Err(no(format!(
            "{t}: interaction coefficients are not translated in test and lincom yet"
        )));
    }
    if t.contains('[') || t.contains(':') {
        return Err(no(format!("{t}: equation names are not translated yet")));
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
        assert!(coef_name("1.g#c.x", &e).is_err(), "interactions not yet");
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
