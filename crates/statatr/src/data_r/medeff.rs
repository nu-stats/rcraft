//! `medeff (regress m t x ...) (regress y m t x ...), mediate(m) treat(t) sims(#)
//! seed(#)` (Hicks and Tingley's `mediation` package, SSC): causal mediation by
//! simulation, as Stata 19 runs it, worked out from its output on made-up data, not
//! from its code (`plan/stata/medeff.md`).
//!
//! - "Using 0 and 1 as treatment values", then the mediator model and the outcome
//!   model refitted with the treatment first and the mediator second (`regress y t m
//!   x`), each on the rows complete in every variable of both; e() is the outcome
//!   model's.
//! - The draws: `set seed #`, then sims rows of the mediator model's coefficients,
//!   b + L z with L the lower Cholesky factor of V and z = invnormal(runiform()) taken
//!   row by row (a draw's k coefficients from k consecutive uniforms); `set seed #`
//!   again and the same for the outcome model, which leaves the generator there.
//! - Each draw's ACME a*b (float), direct effect (float), total effect float(a*b + c);
//!   the means and the 2.5 and 97.5 percentiles (`_pctile`'s rule); the share mediated
//!   is float(mean ACME / total) for each draw, its median and percentiles.
//! - The table, and r(): delta0 delta1 zeta0 zeta1 tau navg with lo and hi.

use super::*;

/// The draws, the summaries, Stata's table and r().
pub(super) const MEDEFF_HELPER: &str = r#"stata_medeff_check <- function(d, treat, cond, show = TRUE) {
  v <- as.numeric(d[[treat]])[cond]
  if (any(!v %in% c(0, 1))) stop("RCraft: medeff with a treatment other than 0 and 1 is not translated yet", call. = FALSE)
  if (show) cat("Using 0 and 1 as treatment values\n")
  invisible(NULL)
}
stata_medeff <- function(mm, mo, treat, med, sims, seed, n, show = TRUE) {
  # in Stata's order, the constant last (R's coef() has it first)
  stata_order <- function(P) {
    k <- c(setdiff(names(P$b), "(Intercept)"), intersect(names(P$b), "(Intercept)"))
    list(b = P$b[k], V = as.matrix(P$V)[k, k, drop = FALSE])
  }
  mm <- stata_order(mm)
  mo <- stata_order(mo)
  draws <- function(P) {
    b <- as.numeric(P$b)
    k <- length(b)
    L <- t(chol(as.matrix(P$V)))
    Z <- matrix(stats::qnorm(stata_rng_u(stata_rng_words(sims * k))), k)
    t(b + L %*% Z)
  }
  stata_set_seed(seed)
  A <- draws(mm)
  stata_set_seed(seed)
  C <- draws(mo)
  a <- A[, match(treat, names(mm$b))]
  cm <- C[, match(med, names(mo$b))]
  ct <- C[, match(treat, names(mo$b))]
  delta <- stata_float(a * cm)
  zeta <- stata_float(ct)
  tau <- stata_float(a * cm + ct)
  share <- stata_float(mean(delta) / tau)
  # _pctile: the average of two order statistics when n p is whole, else the next one up
  pct <- function(v, p) {
    v <- sort(v)
    k <- length(v) * p
    if (abs(k - round(k)) < 1e-9) (v[round(k)] + v[min(length(v), round(k) + 1)]) / 2 else v[ceiling(k)]
  }
  row <- function(x, centre) c(centre, pct(x, 0.025), pct(x, 0.975))
  rs <- list(delta = row(delta, mean(delta)), zeta = row(zeta, mean(zeta)), tau = row(tau, mean(tau)),
             navg = row(share, pct(share, 0.5)))
  if (show) {
    if (n < sims) cat("The number of observations in the data is less than the number of simulations. Expanding the data to the number of simulations\n")
    g <- function(v) stata_fmt(v, "%9.0g")
    line <- function(lab, r) cat("        ", formatC(lab, width = -23, flag = "-"), "| ", g(r[1]), "     ", g(r[2]), "     ", g(r[3]), "\n", sep = "")
    cat(strrep("-", 84), "\n", sep = "")
    cat("        Effect                 |  Mean           [95% Conf. Interval]\n")
    cat(strrep("-", 31), "+", strrep("-", 52), "\n", sep = "")
    line("ACME", rs$delta)
    line("Direct Effect", rs$zeta)
    line("Total Effect", rs$tau)
    line("% of Tot Eff mediated", rs$navg)
    cat(strrep("-", 84), "\n", sep = "")
  }
  list(navglo = rs$navg[2], navghi = rs$navg[3], navg = rs$navg[1], tauhi = rs$tau[3], taulo = rs$tau[2],
       tau = rs$tau[1], zeta0hi = rs$zeta[3], zeta0lo = rs$zeta[2], zeta0 = rs$zeta[1], zeta1hi = rs$zeta[3],
       zeta1lo = rs$zeta[2], zeta1 = rs$zeta[1], delta0hi = rs$delta[3], delta0lo = rs$delta[2],
       delta0 = rs$delta[1], delta1hi = rs$delta[3], delta1lo = rs$delta[2], delta1 = rs$delta[1])
}"#;

/// The r() medeff leaves (Stata 19).
pub(super) const R_MEDEFF: &[&str] = &[
    "navglo", "navghi", "navg", "tauhi", "taulo", "tau", "zeta0hi", "zeta0lo", "zeta0", "zeta1hi",
    "zeta1lo", "zeta1", "delta0hi", "delta0lo", "delta0", "delta1hi", "delta1lo", "delta1",
];

/// One `(regress dep vars)` of medeff: the outcome and the regressors.
fn model(text: &str) -> Result<(String, Vec<String>), Untranslated> {
    let mut w = text.split_whitespace();
    let cmd = w.next().unwrap_or("");
    if !(cmd.len() >= 3 && "regress".starts_with(cmd)) {
        return Err(no(format!(
            "medeff ({text}): models other than regress are not translated yet"
        )));
    }
    let vars: Vec<String> = w.map(str::to_string).collect();
    if vars.len() < 2 || vars.iter().any(|v| !is_name(v)) {
        return Err(no(format!(
            "medeff ({text}): an outcome and variables (no factor variables, if or options) are translated"
        )));
    }
    let dep = vars.first().cloned().unwrap_or_default();
    Ok((dep, vars.get(1..).unwrap_or_default().to_vec()))
}

impl Session {
    /// `medeff (regress ...) (regress ...), mediate(m) treat(t) sims(#) seed(#)`.
    pub(super) fn medeff(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "medeff")?;
        if let Some(o) = opts.other(&[("mediate", 3), ("treat", 5), ("sims", 4), ("seed", 4)]) {
            return Err(no(format!(
                "medeff with the option {o} is not translated yet"
            )));
        }
        if c.parts.if_exp.is_some() || c.parts.in_range.is_some() || c.parts.weight.is_some() {
            return Err(no("medeff with if, in or weights is not translated yet"));
        }
        // the two models in parentheses
        let main = c.parts.main.trim();
        let mut groups = Vec::new();
        let mut rest = main;
        while let Some(open) = rest.strip_prefix('(') {
            let close = open
                .find(')')
                .ok_or_else(|| no("medeff: a ( without its )"))?;
            groups.push(open.get(..close).unwrap_or("").trim().to_string());
            rest = open.get(close + 1..).unwrap_or("").trim_start();
        }
        let ([mtext, otext], true) = (groups.as_slice(), rest.is_empty()) else {
            return Err(no("medeff is translated with two models in parentheses"));
        };
        let (mdep, mvars) = model(mtext)?;
        let (odep, ovars) = model(otext)?;
        let word = |o: &str, m: usize| -> Result<String, Untranslated> {
            let a = opts
                .arg(o, m)
                .ok_or_else(|| no(format!("medeff needs {o}()")))?;
            let a = a.trim().to_string();
            if !is_name(&a) {
                return Err(no(format!(
                    "medeff, {o}({a}): one variable is translated (not treatment values)"
                )));
            }
            Ok(a)
        };
        let med = word("mediate", 3)?;
        let treat = word("treat", 5)?;
        let number = |o: &str, m: usize| -> Result<i64, Untranslated> {
            opts.arg(o, m)
                .and_then(|a| a.trim().parse::<i64>().ok())
                .filter(|v| *v > 0 && *v < 1 << 31)
                .ok_or_else(|| no(format!("medeff is translated with {o}(#), a whole number")))
        };
        let sims = number("sims", 4)?;
        let seed = number("seed", 4)?;
        if mdep != med || !mvars.contains(&treat) {
            return Err(no(
                "medeff: the first model is the mediator's, with the treatment in it",
            ));
        }
        if !ovars.contains(&med) || !ovars.contains(&treat) || odep == med {
            return Err(no(
                "medeff: the second model is the outcome's, with the mediator and the treatment",
            ));
        }
        // the rows complete in every variable of both models
        let mut all: Vec<&String> = std::iter::once(&mdep)
            .chain(&mvars)
            .chain(std::iter::once(&odep))
            .chain(&ovars)
            .collect();
        all.dedup();
        let cond = all
            .iter()
            .map(|v| format!("!missing({v})"))
            .collect::<Vec<_>>()
            .join(" & ");
        let show = !self.quiet(c);
        let d = self.data()?;
        self.helpers.insert("stata_medeff");
        self.helpers.insert("stata_rng");
        self.helpers.insert("stata_float");
        self.helpers.insert("stata_fmt");
        let rcond = all
            .iter()
            .map(|v| format!("!is.na({d}[[{}]])", r_text(v)))
            .collect::<Vec<_>>()
            .join(" & ");
        out.line(format!(
            "stata_medeff_check({d}, {}, {rcond}, show = {})",
            r_text(&treat),
            super::stats::r_logical(show)
        ));
        // the two fits, as regress prints them (the outcome's with the treatment first)
        let others: Vec<&String> = ovars
            .iter()
            .filter(|v| **v != med && **v != treat)
            .collect();
        let fits = [
            format!("regress {mdep} {} if {cond}", mvars.join(" ")),
            format!(
                "regress {odep} {treat} {med} {} if {cond}",
                others
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
        ];
        for (k, text) in fits.iter().enumerate() {
            let program = crate::reader::read(text);
            let mut inner = match program.commands().as_slice() {
                [one] => (*one).clone(),
                _ => return Err(no("medeff: a model RCraft could not read")),
            };
            inner.prefixes = c
                .prefixes
                .iter()
                .filter(|p| matches!(p, crate::reader::Prefix::Quietly))
                .cloned()
                .collect();
            let iopts = Opts(&inner.options);
            self.regress(&inner, &iopts, out)?;
            out.line(format!(
                "stata_med_{} <- list(b = stata_b, V = stata_vcov)",
                if k == 0 { "m" } else { "o" }
            ));
        }
        out.line(format!(
            "stata_r <- stata_medeff(stata_med_m, stata_med_o, {}, {}, {sims}, {seed}, n = stata_e[[\"N\"]], show = {})",
            r_text(&treat),
            r_text(&med),
            super::stats::r_logical(show)
        ));
        out.line("rm(stata_med_m, stata_med_o)");
        self.r_new = Some((R_MEDEFF, "medeff".into()));
        self.rng_lost = None;
        out.caveat(
            "medeff (the mediation package) as Stata 19 runs it, matched to its results, not its code: \
             the draws from Stata's own generator, the effects stored as floats",
        );
        Ok(())
    }
}
