//! P2B.4, the models: `regress` so far, with Stata's factor-variable notation
//! and its robust and clustered standard errors.
//!
//! The fitted model is kept in R as `stata_estimates`, which is where the
//! commands that read a fit afterwards (`predict`, `test`, `margins`, Stata's
//! `e()`) will look when they are translated.
//!
//! Stata's rules kept here:
//!
//! - `i.x` is a factor whose base level is its lowest value, which is also
//!   what R's `factor()` leaves out, so the two agree without being told.
//! - `#` is an interaction and `##` a full factorial, which are R's `:` and
//!   `*`; `c.x` is the plain variable.
//! - `robust` (and `vce(robust)`) is the HC1 correction, not HC0 or HC3.
//! - `vce(cluster v)` is HC1 with the cluster correction `G / (G - 1)`, which
//!   is what `sandwich::vcovCL` does with `cadjust = TRUE` and `type = "HC1"`.
//! - a row with a missing value anywhere in the model is dropped, which is R's
//!   `na.omit`.

use super::*;

impl Session {
    pub(super) fn regress(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "regress")?;
        if let Some(o) = opts.other(&[("noconstant", 5), ("robust", 1), ("vce", 3), ("level", 3)]) {
            return Err(no(format!(
                "regress with the option {o} is not translated yet"
            )));
        }
        let mut words = c.parts.main.split_whitespace();
        let Some(dep) = words.next() else {
            return Err(no("regress needs a variable to explain"));
        };
        if !is_name(dep) {
            return Err(no(format!("{dep} is not a variable name")));
        }
        let terms: Vec<String> = words.map(term).collect::<Result<Vec<_>, Untranslated>>()?;
        let level = match opts.arg("level", 3) {
            Some(l) => {
                let n: f64 = l
                    .trim()
                    .parse()
                    .map_err(|_| no(format!("level({l}) is not a number")))?;
                if !(0.0..100.0).contains(&n) {
                    return Err(no(format!("level({l}) is not between 0 and 100")));
                }
                n / 100.0
            }
            None => 0.95,
        };
        let (robust, cluster) = self.vce(opts)?;
        let src = self.rows_for(c, out)?;
        let right = if terms.is_empty() {
            "1".to_string()
        } else {
            terms.join(" + ")
        };
        let right = if opts.has("noconstant", 5) {
            format!("{right} + 0")
        } else {
            right
        };
        out.line(format!(
            "stata_estimates <- stats::lm({} ~ {right}, data = {src})",
            r_name(dep)
        ));
        if Self::quiet(c) {
            return Ok(());
        }
        match (robust, cluster) {
            (false, None) => {
                out.line("print(summary(stata_estimates))");
                out.line(format!(
                    "print(stats::confint(stata_estimates, level = {}))",
                    r_number(level)
                ));
            }
            (_, cluster) => {
                let vcov = match &cluster {
                    Some(v) => {
                        out.caveat(
                            "the clusters are the rows the model kept, so a row dropped for a \
                             missing value is out of its cluster too",
                        );
                        format!(
                            "sandwich::vcovCL(stata_estimates, cluster = {src}${}[match(rownames(stats::model.frame(stata_estimates)), rownames({src}))], type = \"HC1\")",
                            r_name(v)
                        )
                    }
                    None => "sandwich::vcovHC(stata_estimates, type = \"HC1\")".to_string(),
                };
                out.package("sandwich");
                out.package("lmtest");
                out.line(format!(
                    "print(lmtest::coeftest(stata_estimates, vcov. = {vcov}))"
                ));
                out.line(format!(
                    "print(lmtest::coefci(stata_estimates, vcov. = {vcov}, level = {}))",
                    r_number(level)
                ));
                out.caveat("HC1, which is what Stata's robust means; R's own default would be HC3");
                out.caveat(
                    "only the coefficient table is corrected: R's F and R-squared come from the \
                     ordinary fit",
                );
            }
        }
        out.caveat("a row with a missing value anywhere in the model is dropped, as in Stata");
        out.caveat("printed in R's layout, not Stata's table");
        Ok(())
    }

    /// `robust` and `vce()` together: whether the standard errors are robust,
    /// and the variable to cluster on.
    fn vce(&self, opts: &Opts) -> Result<(bool, Option<String>), Untranslated> {
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
            "ols" if rest.is_empty() => Ok((robust, None)),
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
}

/// One term of a model in R: `i.x` is a factor, `c.x` the plain variable,
/// `a#b` an interaction and `a##b` a full factorial, as Stata writes them.
fn term(t: &str) -> Result<String, Untranslated> {
    if t.contains("##") {
        let parts: Result<Vec<_>, _> = t.split("##").map(one).collect();
        return Ok(parts?.join(" * "));
    }
    if t.contains('#') {
        let parts: Result<Vec<_>, _> = t.split('#').map(one).collect();
        return Ok(parts?.join(":"));
    }
    one(t)
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
            return Ok(format!(
                "stats::relevel(factor({}), ref = {})",
                r_name(v),
                r_text(base)
            ));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statas_factor_notation_becomes_rs() {
        assert_eq!(term("x").as_deref(), Ok("x"));
        assert_eq!(term("c.x").as_deref(), Ok("x"));
        assert_eq!(term("i.g").as_deref(), Ok("factor(g)"));
        assert_eq!(term("i.a#i.b").as_deref(), Ok("factor(a):factor(b)"));
        assert_eq!(term("i.a##c.x").as_deref(), Ok("factor(a) * x"));
        assert_eq!(
            term("ib2.g").as_deref(),
            Ok("stats::relevel(factor(g), ref = \"2\")")
        );
        assert!(term("L.x").is_err(), "lags are not translated");
        assert!(term("i.2x").is_err(), "not a variable name");
    }
}
