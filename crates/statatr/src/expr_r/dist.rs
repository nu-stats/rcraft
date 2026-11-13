//! Stata's distribution functions in R (`ttail()`, `invFtail()`,
//! `chi2tail()`, `normalden()`, `ibeta()`, `binomial()`, `comb()`, ...), with
//! Stata 19's missing values where R gives a number, observed on 3,700
//! arguments (`plan/stata/distributions.md`).
//!
//! R's `pt`, `pf`, `pchisq`, `pbeta`, `pgamma`, `pbinom`, `ppois` and their
//! densities give Stata's doubles to 1e-12 or closer. The rules R does not
//! share are written out here: a degree of freedom, shape or standard
//! deviation must be above 0 (R takes 0 as a point mass), the t, F and chi2
//! degrees of freedom at most 2e17 and a beta or gamma shape at most 1e17
//! (Stata gives missing above), the counts of the discrete distributions are
//! cut to whole numbers, and `comb()` and `lnfactorial()` take whole numbers
//! only. R's `qf` is not used: it gives 0 for small p and swaps to chi2 for
//! df2 above 4e5, where Stata's `invF()` is right; the F quantile comes from
//! `qbeta` instead, on whichever side of one half keeps its digits.

use super::*;

/// Each distribution function, with the numbers of arguments it takes.
pub(super) const FUNCTIONS: &[(&str, &[usize])] = &[
    ("F", &[3]),
    ("Fden", &[3]),
    ("Ftail", &[3]),
    ("betaden", &[3]),
    ("binomial", &[3]),
    ("binomialp", &[3]),
    ("binomialtail", &[3]),
    ("chi2", &[2]),
    ("chi2den", &[2]),
    ("chi2tail", &[2]),
    ("comb", &[2]),
    ("digamma", &[1]),
    ("gammaden", &[4]),
    ("gammap", &[2]),
    ("gammaptail", &[2]),
    ("ibeta", &[3]),
    ("ibetatail", &[3]),
    ("invF", &[3]),
    ("invFtail", &[3]),
    ("invchi2", &[2]),
    ("invchi2tail", &[2]),
    ("invgammap", &[2]),
    ("invgammaptail", &[2]),
    ("invibeta", &[3]),
    ("invibetatail", &[3]),
    ("invt", &[2]),
    ("invttail", &[2]),
    ("lnfactorial", &[1]),
    ("lngamma", &[1]),
    ("lnnormal", &[1]),
    ("lnnormalden", &[1, 2, 3]),
    ("normalden", &[1, 2, 3]),
    ("poisson", &[2]),
    ("poissonp", &[2]),
    ("poissontail", &[2]),
    ("t", &[2]),
    ("tden", &[2]),
    ("tprob", &[2]),
    ("trigamma", &[1]),
    ("ttail", &[2]),
];

pub(super) fn is_distribution(name: &str) -> bool {
    FUNCTIONS.iter().any(|(f, _)| *f == name)
}

/// The R helpers, one a function (`stata_d_ttail()` for `ttail()`).
pub(crate) const DIST_HELPER: &str = r##"stata_d_keep <- function(r, ok) {
  ok <- rep_len(ok, length(r))
  r[is.na(ok) | !ok | !is.finite(r)] <- NA_real_
  r
}
stata_d_pos <- function(d, hi = Inf) !is.na(d) & d > 0 & d <= hi
stata_d_whole <- function(v) !is.na(v) & is.finite(v) & v == floor(v)
stata_d_prob <- function(p) !is.na(p) & p >= 0 & p <= 1
stata_d_run <- function(e) suppressWarnings(e)
stata_d_t <- function(d, x) stata_d_keep(stata_d_run(stats::pt(x, d)), stata_d_pos(d, 2e17))
stata_d_ttail <- function(d, x) stata_d_keep(stata_d_run(stats::pt(x, d, lower.tail = FALSE)), stata_d_pos(d, 2e17))
stata_d_tden <- function(d, x) stata_d_keep(stata_d_run(stats::dt(x, d)), stata_d_pos(d))
stata_d_tprob <- function(d, x) stata_d_keep(stata_d_run(2 * stats::pt(-abs(x), d)), stata_d_pos(d, 2e17))
stata_d_invt <- function(d, p) stata_d_keep(stata_d_run(stats::qt(p, d)), stata_d_pos(d, 2e17))
stata_d_invttail <- function(d, p) stata_d_keep(stata_d_run(stats::qt(p, d, lower.tail = FALSE)), stata_d_pos(d, 2e17))
stata_d_F <- function(a, b, f) stata_d_keep(stata_d_run(stats::pf(f, a, b)), stata_d_pos(a, 2e17) & stata_d_pos(b, 2e17))
stata_d_Ftail <- function(a, b, f) stata_d_keep(stata_d_run(stats::pf(f, a, b, lower.tail = FALSE)), stata_d_pos(a, 2e17) & stata_d_pos(b, 2e17))
stata_d_Fden <- function(a, b, f) stata_d_keep(stata_d_run(stats::df(f, a, b)), stata_d_pos(a) & stata_d_pos(b))
stata_d_invF_side <- function(a, b, p, upper) {
  x <- stata_d_run(stats::qbeta(p, a / 2, b / 2, lower.tail = !upper))
  y <- stata_d_run(stats::qbeta(p, b / 2, a / 2, lower.tail = upper))
  r <- ifelse(!is.na(x) & x < 0.5, (b / a) * x / (1 - x), (b / a) * (1 - y) / y)
  stata_d_keep(r, stata_d_pos(a, 2e17) & stata_d_pos(b, 2e17))
}
stata_d_invF <- function(a, b, p) stata_d_invF_side(a, b, p, FALSE)
stata_d_invFtail <- function(a, b, p) stata_d_invF_side(a, b, p, TRUE)
stata_d_chi2 <- function(d, x) stata_d_keep(stata_d_run(stats::pchisq(x, d)), stata_d_pos(d, 2e17))
stata_d_chi2tail <- function(d, x) stata_d_keep(stata_d_run(stats::pchisq(x, d, lower.tail = FALSE)), stata_d_pos(d, 2e17))
stata_d_chi2den <- function(d, x) stata_d_keep(stata_d_run(stats::dchisq(x, d)), stata_d_pos(d, 2e17))
stata_d_invchi2 <- function(d, p) stata_d_keep(stata_d_run(stats::qchisq(p, d)), stata_d_pos(d, 2e17))
stata_d_invchi2tail <- function(d, p) stata_d_keep(stata_d_run(stats::qchisq(p, d, lower.tail = FALSE)), stata_d_pos(d, 2e17))
stata_d_normalden <- function(z) stata_d_keep(stats::dnorm(z), TRUE)
stata_d_normalden_s <- function(z, s) {
  r <- stata_d_run(stats::dnorm(z, 0, s))
  n <- length(r)
  ok <- rep_len(stata_d_pos(s), n)
  # Stata 19: normalden(., s) is 0, not missing, for a valid s
  r[is.na(rep_len(z, n)) & ok] <- 0
  stata_d_keep(r, ok)
}
stata_d_normalden_ms <- function(z, m, s) stata_d_keep(stata_d_run(stats::dnorm(z, m, s)), stata_d_pos(s))
stata_d_lnnormal <- function(z) stata_d_keep(stats::pnorm(z, log.p = TRUE), TRUE)
stata_d_lnnormalden <- function(z) stata_d_keep(stats::dnorm(z, log = TRUE), TRUE)
stata_d_lnnormalden_s <- function(z, s) stata_d_keep(stata_d_run(stats::dnorm(z, 0, s, log = TRUE)), stata_d_pos(s))
stata_d_lnnormalden_ms <- function(z, m, s) stata_d_keep(stata_d_run(stats::dnorm(z, m, s, log = TRUE)), stata_d_pos(s))
stata_d_ibeta <- function(a, b, x) stata_d_keep(stata_d_run(stats::pbeta(x, a, b)), stata_d_pos(a, 1e17) & stata_d_pos(b, 1e17))
stata_d_ibetatail <- function(a, b, x) stata_d_keep(stata_d_run(stats::pbeta(x, a, b, lower.tail = FALSE)), stata_d_pos(a, 1e17) & stata_d_pos(b, 1e17))
stata_d_betaden <- function(a, b, x) stata_d_keep(stata_d_run(stats::dbeta(x, a, b)), stata_d_pos(a) & stata_d_pos(b))
stata_d_invibeta <- function(a, b, p) stata_d_keep(stata_d_run(stats::qbeta(p, a, b)), stata_d_pos(a, 1e17) & stata_d_pos(b, 1e17))
stata_d_invibetatail <- function(a, b, p) stata_d_keep(stata_d_run(stats::qbeta(p, a, b, lower.tail = FALSE)), stata_d_pos(a, 1e17) & stata_d_pos(b, 1e17))
stata_d_gammap <- function(a, x) stata_d_keep(stata_d_run(stats::pgamma(x, a)), stata_d_pos(a, 1e17))
stata_d_gammaptail <- function(a, x) stata_d_keep(stata_d_run(stats::pgamma(x, a, lower.tail = FALSE)), stata_d_pos(a, 1e17))
stata_d_gammaden <- function(a, b, g, x) stata_d_keep(stata_d_run(stats::dgamma(x - g, shape = a, scale = b)), stata_d_pos(a) & stata_d_pos(b))
stata_d_invgammap <- function(a, p) stata_d_keep(stata_d_run(stats::qgamma(p, a)), stata_d_pos(a, 1e17))
stata_d_invgammaptail <- function(a, p) stata_d_keep(stata_d_run(stats::qgamma(p, a, lower.tail = FALSE)), stata_d_pos(a, 1e17))
stata_d_lngamma <- function(x) stata_d_keep(stata_d_run(lgamma(x)), TRUE)
stata_d_digamma <- function(x) stata_d_keep(stata_d_run(digamma(x)), TRUE)
stata_d_trigamma <- function(x) stata_d_keep(stata_d_run(trigamma(x)), TRUE)
stata_d_lnfactorial <- function(n) stata_d_keep(stata_d_run(lgamma(n + 1)), stata_d_whole(n) & n >= 0)
stata_d_comb <- function(n, k) {
  ok <- stata_d_whole(n) & stata_d_whole(k) & k >= 0 & k <= n
  stata_d_keep(stata_d_run(choose(n, k)), ok)
}
stata_d_binok <- function(n, p) !is.na(n) & n >= 0 & n <= 1e17 & stata_d_prob(p)
stata_d_binomial <- function(n, k, p) stata_d_keep(stata_d_run(stats::pbinom(floor(k), floor(n), p)), stata_d_binok(n, p))
stata_d_binomialtail <- function(n, k, p) stata_d_keep(stata_d_run(stats::pbinom(floor(k) - 1, floor(n), p, lower.tail = FALSE)), stata_d_binok(n, p))
stata_d_binomialp <- function(n, k, p) {
  ok <- stata_d_binok(n, p) & floor(n) >= 1 & !is.na(k) & floor(k) >= 0 & floor(k) <= floor(n)
  stata_d_keep(stata_d_run(stats::dbinom(floor(k), floor(n), p)), ok)
}
stata_d_poisok <- function(m, k) stata_d_pos(m) & !is.na(k) & k >= 0
stata_d_poisson <- function(m, k) stata_d_keep(stata_d_run(stats::ppois(floor(k), m)), stata_d_poisok(m, k))
stata_d_poissontail <- function(m, k) stata_d_keep(stata_d_run(stats::ppois(floor(k) - 1, m, lower.tail = FALSE)), stata_d_poisok(m, k))
stata_d_poissonp <- function(m, k) stata_d_keep(stata_d_run(stats::dpois(floor(k), m)), stata_d_poisok(m, k))"##;

impl Translator<'_> {
    /// A distribution function: its R helper on the arguments' numbers.
    pub(super) fn distribution(
        &mut self,
        name: &str,
        args: &[Expr],
    ) -> Result<(String, Kind), Untranslated> {
        let takes = FUNCTIONS
            .iter()
            .find(|(f, _)| *f == name)
            .map_or(&[][..], |(_, n)| *n);
        if !takes.contains(&args.len()) {
            let n = match (takes.first(), takes.last()) {
                (Some(a), Some(b)) if a != b => format!("{a} to {b}"),
                (Some(a), _) => a.to_string(),
                _ => "no".to_string(),
            };
            return Err(no(format!(
                "{name}() takes {n} argument{} (Stata: invalid syntax)",
                if takes == [1] { "" } else { "s" }
            )));
        }
        let mut vals = Vec::new();
        for a in args {
            let (c, k) = self.expr(a)?;
            if k == Kind::Text {
                return Err(no(format!("{name}() of a string (Stata: type mismatch)")));
            }
            vals.push(number(&c, k)?);
        }
        self.helpers.push("stata_dist");
        let suffix = match (name, args.len()) {
            ("normalden" | "lnnormalden", 2) => "_s",
            ("normalden" | "lnnormalden", 3) => "_ms",
            _ => "",
        };
        Ok((
            format!("stata_d_{name}{suffix}({})", vals.join(", ")),
            Kind::Number,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::super::{translate, Use};

    fn none(_: &str) -> Option<super::Kind> {
        None
    }

    #[test]
    fn distribution_functions_call_their_helpers() {
        let code = |t: &str| translate(t, &none, Use::Value).map(|r| (r.code, r.helpers));
        assert_eq!(
            code("2*ttail(df, abs(t))"),
            Ok((
                "(2 * stata_d_ttail(df, abs(t)))".to_string(),
                vec!["stata_dist"]
            ))
        );
        assert_eq!(
            code("normalden(x, 2)").map(|r| r.0),
            Ok("stata_d_normalden_s(x, 2)".to_string())
        );
        assert_eq!(
            code("invFtail(3, 20, .05)").map(|r| r.0),
            Ok("stata_d_invFtail(3, 20, 0.05)".to_string())
        );
        let refused = |t: &str| translate(t, &none, Use::Value).err().map(|e| e.reason);
        assert_eq!(
            refused("ttail(5)"),
            Some("ttail() takes 2 arguments (Stata: invalid syntax)".to_string())
        );
        assert_eq!(
            refused("normalden(1, 2, 3, 4)"),
            Some("normalden() takes 1 to 3 arguments (Stata: invalid syntax)".to_string())
        );
        assert_eq!(
            refused("chi2tail(\"a\", 1)"),
            Some("chi2tail() of a string (Stata: type mismatch)".to_string())
        );
    }

    #[test]
    fn the_table_is_sorted_and_has_no_twins() {
        let names: Vec<&str> = super::FUNCTIONS.iter().map(|(f, _)| *f).collect();
        let mut sorted = names.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(names, sorted);
    }
}
