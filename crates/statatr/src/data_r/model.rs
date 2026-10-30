//! P2B.4, the models: `regress` here, the maximum-likelihood models in
//! [`super::ml`], and what reads a fit afterwards in [`super::post`].
//!
//! Every model leaves four things in R, which are Stata's `e()` results:
//!
//! - `stata_estimates`, the fit itself;
//! - `stata_b`, the coefficients (`e(b)`);
//! - `stata_vcov`, their variance (`e(V)`), robust or clustered when asked,
//!   so that `test` and `lincom` use the same one Stata would;
//! - `stata_df`, the residual degrees of freedom for t and F tests
//!   (`e(df_r)`), or `Inf` where Stata uses z and chi-squared.
//!
//! Stata's rules kept here:
//!
//! - `i.x` is a factor whose base level is its lowest value, which is also
//!   what R's `factor()` leaves out, so the two agree without being told.
//! - `#` is an interaction and `##` a full factorial, which are R's `:` and
//!   `*`; `c.x` is the plain variable.
//! - `robust` (and `vce(robust)`) is the HC1 correction, not HC0 or HC3.
//! - `vce(cluster v)` is HC1 with the cluster correction `G / (G - 1)`, which
//!   is what `sandwich::vcovCL` does with `cadjust = TRUE` and `type = "HC1"`,
//!   and its t tests and intervals use `G - 1` degrees of freedom, not
//!   `N - K`.
//! - a row with a missing value anywhere in the model is dropped, which is R's
//!   `na.omit`.

use super::weights::W;
use super::*;

/// Which model `stata_estimates` holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Fit {
    Regress,
    Logit,
    Probit,
    Poisson,
    Nbreg,
    Ologit,
    Oprobit,
    Mlogit,
    Xtreg,
    Ivregress,
    /// `margins, post`: the margins are e(b) and e(V)
    Margins,
}

impl Fit {
    pub(super) fn name(self) -> &'static str {
        match self {
            Fit::Regress => "regress",
            Fit::Logit => "logit",
            Fit::Probit => "probit",
            Fit::Poisson => "poisson",
            Fit::Nbreg => "nbreg",
            Fit::Ologit => "ologit",
            Fit::Oprobit => "oprobit",
            Fit::Mlogit => "mlogit",
            Fit::Xtreg => "xtreg",
            Fit::Ivregress => "ivregress",
            Fit::Margins => "margins",
        }
    }

    /// One outcome per row of the data, so a prediction is one variable.
    pub(super) fn one_outcome(self) -> bool {
        !matches!(self, Fit::Ologit | Fit::Oprobit | Fit::Mlogit)
    }
}

/// What the translator remembers of the model last fitted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Estimates {
    pub(super) fit: Fit,
    /// The variable explained, as written.
    pub(super) dep: String,
    /// Factor variables (`i.g`, `ib2.g`) and the R that made each one, so
    /// that Stata's `1.g` can be found among R's coefficient names.
    pub(super) factors: BTreeMap<String, String>,
}

/// Stata's coefficient table: estimate, standard error, t or z, p-value and
/// interval, from the coefficients and the variance given. With `eform`
/// (`or`, `irr`, `rrr`) the estimate and interval are exponentiated and the
/// standard error is the delta method's, as Stata shows them; `eform` may
/// name only some of the rows (cut points are never exponentiated).
pub(super) const COEFS_HELPER: &str = r#"stata_coefs <- function(b, V, df = Inf, level = 0.95, eform = FALSE, label = "Coef.", beta = NULL) {
  se <- sqrt(diag(as.matrix(V)))
  stat <- b / se
  small <- is.finite(df)
  p <- if (small) 2 * stats::pt(-abs(stat), df) else 2 * stats::pnorm(-abs(stat))
  q <- if (small) stats::qt((1 + level) / 2, df) else stats::qnorm((1 + level) / 2)
  lo <- b - q * se
  hi <- b + q * se
  e <- rep_len(eform, length(b))
  est <- ifelse(e, exp(b), b)
  se <- ifelse(e, exp(b) * se, se)
  lo <- ifelse(e, exp(lo), lo)
  hi <- ifelse(e, exp(hi), hi)
  tab <- data.frame(est, se, stat, p, lo, hi)
  names(tab) <- c(label, "Std. err.", if (small) "t" else "z", if (small) "P>|t|" else "P>|z|",
                  paste0("[", format(100 * level), "% conf."), "interval]")
  if (!is.null(beta)) {
    # regress, beta: the standardised coefficient in place of the interval
    tab <- tab[1:4]
    tab$Beta <- unname(beta)
  }
  rownames(tab) <- names(b)
  print(tab, digits = 7)
  invisible(tab)
}"#;

/// Stata's `ib2.x`: a factor whose first level, the base, is the one chosen.
/// Unlike `relevel()` it works on data that lack the base level, as the rows
/// `predict` and `margins` make do (every row set to one level).
pub(super) const IB_HELPER: &str = r#"stata_ib <- function(x, base) {
  lv <- as.character(sort(unique(x[!is.na(x)])))
  factor(x, levels = c(base, setdiff(lv, base)))
}"#;

impl Session {
    /// The helpers a model's terms call.
    pub(super) fn helpers_for(&mut self, spec: &Spec) {
        if spec.factors.values().any(|e| e.starts_with("stata_ib(")) {
            self.helpers.insert("stata_ib");
        }
    }

    pub(super) fn regress(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "regress")?;
        if let Some(o) = opts.other(&[
            ("noconstant", 5),
            ("robust", 1),
            ("vce", 3),
            ("level", 3),
            ("beta", 4),
        ]) {
            return Err(no(format!(
                "regress with the option {o} is not translated yet"
            )));
        }
        let spec = Spec::read(c, "regress")?;
        self.helpers_for(&spec);
        let level = level(opts)?;
        let (robust, cluster) = vce(opts)?;
        let beta = opts.has("beta", 4);
        if beta && cluster.is_some() {
            return Err(no(
                "options vce(cluster clustvar) and beta may not be combined",
            ));
        }
        let w = self.weight_of(c, &[W::F, W::A, W::P, W::I])?;
        if w.is_some() && beta {
            return Err(no("regress, beta with weights is not translated yet"));
        }
        if matches!(w, Some((W::I, _))) && (robust || cluster.is_some()) {
            return Err(no(
                "robust or clustered errors with iweights are not translated yet",
            ));
        }
        // sampling weights mean robust errors
        let robust = robust || matches!(w, Some((W::P, _)));
        self.plain_regress = !robust && cluster.is_none() && w.is_none();
        self.fit_robust = (robust || cluster.is_some()) && w.is_none();
        self.fit_cluster = cluster.is_some() && w.is_none();
        let rows = self.rows_for(c, out)?;
        let src = self.weighted_rows(rows, &w);
        let right = spec.right(opts.has("noconstant", 5));
        let weights = match &w {
            Some((_, v)) => format!(", weights = {}", r_name(v)),
            None => String::new(),
        };
        out.line(format!(
            "stata_estimates <- stats::lm({} ~ {right}, data = {src}{weights})",
            r_name(&spec.dep)
        ));
        out.line(keep_sample(&src));
        let d = self.data()?;
        out.package("dplyr");
        out.line(esample_line(&d, &src));
        self.esample_new = true;
        out.line("stata_b <- stats::coef(stata_estimates)");
        if let Some((k, _)) = &w {
            self.helpers.insert("stata_lm_wvcov");
            let cl = match &cluster {
                Some(v) => {
                    out.line(format!("stata_cluster <- {}", cluster_rows(&src, v)));
                    out.line(super::ml::no_missing_cluster());
                    ", cluster = stata_cluster"
                }
                None => "",
            };
            out.line(format!(
                "stata_vcov <- stata_lm_wvcov(stata_estimates, {}, robust = {}{cl})",
                r_text(k.code()),
                super::stats::r_logical(robust)
            ));
            out.line("stata_df <- attr(stata_vcov, \"df\")");
            out.line("attr(stata_vcov, \"df\") <- NULL");
            out.caveat(super::weights::weight_caveat(&w));
        }
        match &cluster {
            _ if w.is_some() => {}
            None if !robust => {
                out.line("stata_vcov <- stats::vcov(stata_estimates)");
                out.line("stata_df <- stata_estimates$df.residual");
            }
            None => {
                out.package("sandwich");
                out.line("stata_vcov <- sandwich::vcovHC(stata_estimates, type = \"HC1\")");
                out.line("stata_df <- stata_estimates$df.residual");
            }
            Some(v) => {
                out.package("sandwich");
                out.line(format!("stata_cluster <- {}", cluster_rows(&src, v)));
                out.line(super::ml::no_missing_cluster());
                out.line(
                    "stata_vcov <- sandwich::vcovCL(stata_estimates, cluster = stata_cluster, type = \"HC1\")",
                );
                out.line("stata_df <- length(unique(stata_cluster)) - 1");
            }
        }
        self.estimates = Some(Estimates {
            fit: Fit::Regress,
            dep: spec.dep.clone(),
            factors: spec.factors.clone(),
        });
        if cluster.is_some() {
            out.caveat(
                "the clusters are the rows the model kept, so a row dropped for a missing value \
                 is out of its cluster too",
            );
            out.caveat("t tests and intervals use G - 1 degrees of freedom, as Stata's do");
        }
        if beta {
            // e(beta): b times the standard deviation of its column over that of
            // the outcome, both over the estimation sample; none for the constant
            out.line(
                "stata_beta <- local({ X <- stats::model.matrix(stata_estimates); \
                 b <- stata_b * apply(X, 2, stats::sd) / \
                 stats::sd(stats::model.response(stats::model.frame(stata_estimates))); \
                 b[colnames(X) == \"(Intercept)\"] <- NA; b })",
            );
        }
        if Self::quiet(c) {
            return Ok(());
        }
        if robust || cluster.is_some() || beta || w.is_some() {
            self.helpers.insert("stata_coefs");
            out.line(format!(
                "stata_coefs(stata_b, stata_vcov, stata_df, level = {}{})",
                r_number(level),
                if beta { ", beta = stata_beta" } else { "" }
            ));
            if robust || cluster.is_some() {
                out.caveat("HC1, which is what Stata's robust means; R's own default would be HC3");
            }
            if beta {
                out.caveat(
                    "Beta is the coefficient times sd(x) / sd(y) over the rows the model used, \
                     shown in place of the interval as Stata does",
                );
            }
        } else {
            out.line("print(summary(stata_estimates))");
            out.line(format!(
                "print(stats::confint(stata_estimates, level = {}))",
                r_number(level)
            ));
        }
        out.caveat("a row with a missing value anywhere in the model is dropped, as in Stata");
        out.caveat("printed in R's layout, not Stata's table");
        Ok(())
    }
}

/// The variable a model explains and the terms that explain it.
pub(super) struct Spec {
    pub(super) dep: String,
    pub(super) terms: Vec<String>,
    pub(super) factors: BTreeMap<String, String>,
}

impl Spec {
    pub(super) fn read(c: &Command, name: &str) -> Result<Spec, Untranslated> {
        let mut words = c.parts.main.split_whitespace();
        let Some(dep) = words.next() else {
            return Err(no(format!("{name} needs a variable to explain")));
        };
        if !is_name(dep) {
            return Err(no(format!("{dep} is not a variable name")));
        }
        Spec::from_terms(dep, words)
    }

    /// The variable explained and the terms as Stata writes them.
    pub(super) fn from_terms<'a>(
        dep: &str,
        words: impl IntoIterator<Item = &'a str>,
    ) -> Result<Spec, Untranslated> {
        let mut terms = Vec::new();
        let mut factors = BTreeMap::new();
        for w in words {
            terms.push(term(w)?);
            for part in w.split('#').filter(|p| !p.is_empty()) {
                if let Some(v) = factor_variable(part) {
                    factors.insert(v.to_string(), one(part)?);
                }
            }
        }
        Ok(Spec {
            dep: dep.to_string(),
            terms,
            factors,
        })
    }

    /// The right-hand side of R's formula.
    pub(super) fn right(&self, noconstant: bool) -> String {
        let right = if self.terms.is_empty() {
            "1".to_string()
        } else {
            self.terms.join(" + ")
        };
        if noconstant {
            format!("{right} + 0")
        } else {
            right
        }
    }
}

/// The variable of `i.g` or `ib2.g`.
fn factor_variable(part: &str) -> Option<&str> {
    let part = part.trim();
    if let Some(v) = part.strip_prefix("i.") {
        return Some(v);
    }
    let rest = part.strip_prefix("ib")?;
    rest.split_once('.').map(|(_, v)| v)
}

/// `level()` as a proportion; 95% when it is not given.
pub(super) fn level(opts: &Opts) -> Result<f64, Untranslated> {
    match opts.arg("level", 3) {
        Some(l) => {
            let n: f64 = l
                .trim()
                .parse()
                .map_err(|_| no(format!("level({l}) is not a number")))?;
            if !(10.0..100.0).contains(&n) {
                return Err(no(format!("level({l}) is not between 10 and 99.99")));
            }
            Ok(n / 100.0)
        }
        None => Ok(0.95),
    }
}

/// `robust` and `vce()` together: whether the standard errors are robust,
/// and the variable to cluster on.
pub(super) fn vce(opts: &Opts) -> Result<(bool, Option<String>), Untranslated> {
    let robust = opts.has("robust", 1);
    let Some(v) = opts.arg("vce", 3) else {
        return Ok((robust, None));
    };
    let v = v.trim();
    let (kind, rest) = match v.split_once(char::is_whitespace) {
        Some((a, b)) => (a.trim(), b.trim()),
        None => (v, ""),
    };
    match kind {
        "ols" | "oim" if rest.is_empty() => Ok((robust, None)),
        "robust" if rest.is_empty() => Ok((true, None)),
        "cluster" => {
            if !is_name(rest) {
                return Err(no(format!("vce(cluster {rest}) needs one variable name")));
            }
            Ok((true, Some(rest.to_string())))
        }
        other => Err(no(format!("vce({other}) is not translated yet"))),
    }
}

/// The rows the model used, as they were when it was fitted: Stata's
/// `e(sample)`, which `margins` averages over.
pub(super) fn keep_sample(src: &str) -> String {
    format!(
        "stata_data <- {src}[match(rownames(stats::model.frame(stata_estimates)), rownames({src})), , drop = FALSE]"
    )
}

/// The cluster variable for the rows the model kept.
pub(super) fn cluster_rows(src: &str, v: &str) -> String {
    format!(
        "{src}${}[match(rownames(stats::model.frame(stata_estimates)), rownames({src}))]",
        r_name(v)
    )
}

/// One term of a model in R: `i.x` is a factor, `c.x` the plain variable,
/// `a#b` an interaction and `a##b` a full factorial, as Stata writes them.
///
/// `a##b` is written out as `a + b + a:b` rather than R's `a * b`, because
/// R simplifies `x * x` to `x` where Stata's `c.x##c.x` is `x` and its square.
fn term(t: &str) -> Result<String, Untranslated> {
    if t.contains("##") {
        let parts: Vec<&str> = t.split("##").collect();
        if parts.len() > 8 {
            return Err(no(format!(
                "{t}: Stata allows at most eight variables in ##"
            )));
        }
        let mut terms: Vec<String> = Vec::new();
        for mask in 1u32..(1 << parts.len()) {
            let chosen: Vec<&str> = parts
                .iter()
                .enumerate()
                .filter(|(i, _)| (mask >> i) & 1 == 1)
                .map(|(_, p)| *p)
                .collect();
            let p = product(&chosen)?;
            if !terms.contains(&p) {
                terms.push(p);
            }
        }
        return Ok(terms.join(" + "));
    }
    if t.contains('#') {
        return product(&t.split('#').collect::<Vec<_>>());
    }
    one(t)
}

/// An interaction: R's `:` between the parts, with a continuous variable
/// that appears more than once raised to its power (R reads `x:x` as `x`).
fn product(parts: &[&str]) -> Result<String, Untranslated> {
    let mut seen: Vec<(String, usize)> = Vec::new();
    for p in parts {
        let r = one(p)?;
        match seen.iter_mut().find(|(s, _)| *s == r) {
            Some(e) => e.1 += 1,
            None => seen.push((r, 1)),
        }
    }
    let mut out = Vec::new();
    for (r, k) in seen {
        if k == 1 {
            out.push(r);
        } else if r.starts_with("factor(") || r.starts_with("stata_ib(") {
            return Err(no(format!(
                "{}: a factor variable interacted with itself is not translated",
                parts.join("#")
            )));
        } else {
            out.push(format!("I({r}^{k})"));
        }
    }
    Ok(out.join(":"))
}

/// One part of a term, without `#`.
fn one(t: &str) -> Result<String, Untranslated> {
    let t = t.trim();
    if t.is_empty() {
        return Err(no("a model term is empty"));
    }
    if let Some(v) = t.strip_prefix("c.") {
        return if is_name(v) {
            Ok(r_name(v))
        } else {
            Err(no(format!("{v} is not a variable name")))
        };
    }
    if let Some(v) = t.strip_prefix("i.") {
        return if is_name(v) {
            Ok(format!("factor({})", r_name(v)))
        } else {
            Err(no(format!("{v} is not a variable name")))
        };
    }
    // ib2.x and the like: the base level is chosen
    if let Some(rest) = t.strip_prefix("ib") {
        if let Some((base, v)) = rest.split_once('.') {
            if !is_name(v) {
                return Err(no(format!("{v} is not a variable name")));
            }
            if base.is_empty() || !base.chars().all(|c| c.is_ascii_digit()) {
                return Err(no(format!(
                    "{t}: only a number can name the base level (ib2.x)"
                )));
            }
            return Ok(format!("stata_ib({}, {})", r_name(v), r_text(base)));
        }
    }
    if t.contains('.') {
        return Err(no(format!(
            "{t}: only i., c. and ib#. are translated (lags like L.x are not)"
        )));
    }
    if is_name(t) {
        Ok(r_name(t))
    } else {
        Err(no(format!("{t} is not a variable name")))
    }
}

/// `e(sample)` for a fit that keeps the rows it used, as positions in the
/// rows its data expression gives (`xtreg`, `ivregress`).
pub(super) fn esample_rows_line(d: &str, src: &str, rows: &str) -> String {
    format!(
        "stata_esample <- local({{ {d} <- dplyr::mutate({d}, stata_row = seq_len(nrow({d}))); s <- {src}; seq_len(nrow({d})) %in% s$stata_row[{rows}] }})"
    )
}

/// Stata's `e(sample)` as `stata_esample`: for each row of the data frame
/// `d`, whether the model used it. The model's rows are found by running its
/// data expression again on `d` with each row's number added (the name `d`
/// is shadowed inside `local()`, so `src` reads the numbered copy).
pub(super) fn esample_line(d: &str, src: &str) -> String {
    format!(
        "stata_esample <- local({{ {d} <- dplyr::mutate({d}, stata_row = seq_len(nrow({d}))); s <- {src}; seq_len(nrow({d})) %in% s$stata_row[match(rownames(stats::model.frame(stata_estimates)), rownames(s))] }})"
    )
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statas_factor_notation_becomes_rs() {
        assert_eq!(term("x").as_deref(), Ok("x"));
        assert_eq!(term("c.x").as_deref(), Ok("x"));
        assert_eq!(term("i.g").as_deref(), Ok("factor(g)"));
        assert_eq!(term("i.a#i.b").as_deref(), Ok("factor(a):factor(b)"));
        assert_eq!(
            term("i.a##c.x").as_deref(),
            Ok("factor(a) + x + factor(a):x")
        );
        // R would read x * x and x:x as plain x; Stata's are the square
        assert_eq!(term("c.x##c.x").as_deref(), Ok("x + I(x^2)"));
        assert_eq!(term("c.x#c.x").as_deref(), Ok("I(x^2)"));
        assert_eq!(term("c.x#c.x#i.g").as_deref(), Ok("I(x^2):factor(g)"));
        assert_eq!(
            term("a##b##c").as_deref(),
            Ok("a + b + a:b + c + a:c + b:c + a:b:c")
        );
        assert!(term("i.g#i.g").is_err());
        assert_eq!(term("ib2.g").as_deref(), Ok("stata_ib(g, \"2\")"));
        assert!(term("L.x").is_err(), "lags are not translated");
        assert!(term("i.2x").is_err(), "not a variable name");
    }

    #[test]
    fn factor_variables_are_remembered_with_their_r() {
        assert_eq!(factor_variable("i.g"), Some("g"));
        assert_eq!(factor_variable("ib3.g"), Some("g"));
        assert_eq!(factor_variable("c.x"), None);
        assert_eq!(factor_variable("x"), None);
    }
}
