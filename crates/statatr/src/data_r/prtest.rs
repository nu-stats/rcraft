//! `prtest var [if] [in], by(g) [level(#)]`: the two-sample test of
//! proportions, as Stata 19 prints it and the r() it leaves (observed with
//! made-up data and AXYMDI's Stata log, `plan/stata/prtest.md`): each group's
//! mean with its standard error sqrt(p(1 - p)/n) and normal interval, the
//! difference with sqrt(se1^2 + se2^2), and the test under H0 with the pooled
//! proportion. Groups in sorted order, named by their value labels (else
//! their values), cut at 12 characters.

use super::*;

pub(super) const PRTEST_HELPER: &str = r##"stata_prtest <- function(x, g, gname, level = 95, show = TRUE) {
  x <- as.numeric(x)
  lab <- attr(g, "labels", exact = TRUE)
  gv <- if (is.character(g)) g else as.numeric(g)
  keep <- !is.na(x) & !is.na(gv) & !(is.character(gv) & gv == "")
  x <- x[keep]
  gv <- gv[keep]
  lv <- sort(unique(gv), method = "radix")
  if (length(lv) != 2) stop(gname, " takes on ", length(lv), " values, not 2 (Stata: r(450))", call. = FALSE)
  name <- function(v) {
    s <- if (is.character(v)) v else {
      k <- if (!is.null(lab)) names(lab)[match(v, as.numeric(lab))] else NA
      if (!is.na(k)) k else trimws(stata_fmt(v, "%9.0g"))
    }
    substr(s, 1, 12)
  }
  n <- vapply(lv, function(v) sum(gv == v), 1)
  p <- vapply(lv, function(v) mean(x[gv == v]), 1)
  for (pi in p) if (pi < 0 || pi > 1) stop(sprintf("%.15g", pi), " not in [0,1] (Stata: r(198))", call. = FALSE)
  se <- sqrt(p * (1 - p) / n)
  q <- stats::qnorm(1 - (1 - level / 100) / 2)
  d <- p[1] - p[2]
  sed <- sqrt(sum(se^2))
  pp <- sum(p * n) / sum(n)
  se0 <- sqrt(pp * (1 - pp) * (1 / n[1] + 1 / n[2]))
  z <- d / se0
  r <- list(N1 = n[1], N2 = n[2], P1 = p[1], P2 = p[2], P_diff = d, se1 = se[1], se2 = se[2],
            se_diff0 = se0, se_diff = sed, lb1 = p[1] - q * se[1], ub1 = p[1] + q * se[1],
            lb2 = p[2] - q * se[2], ub2 = p[2] + q * se[2], lb_diff = d - q * sed, ub_diff = d + q * sed,
            z = z, p_l = stats::pnorm(z), p = 2 * stats::pnorm(-abs(z)), p_u = stats::pnorm(-z), level = level)
  r <- lapply(r, unname)
  if (show) {
    g9 <- function(v) stata_fmt(v, "%9.0g")
    nm <- vapply(lv, name, "")
    rule <- strrep("-", 78)
    mid <- paste0(strrep("-", 13), "+", strrep("-", 64))
    cat("\n")
    cat(sprintf("%-30s%22s: Number of obs = %8.0f\n", "Two-sample test of proportions", nm[1], n[1]))
    cat(sprintf("%52s: Number of obs = %8.0f\n", nm[2], n[2]))
    cat(rule, "\n", sep = "")
    cat(sprintf("       Group |       Mean   Std. err.      z    P>|z|     [%s%% conf. interval]\n", stata_fmt(level, "%9.0g", trim = TRUE)))
    cat(mid, "\n", sep = "")
    row <- function(l, m, s, lo, hi) cat(sprintf("%12s |  %9s  %9s%17s    %9s   %9s\n", l, g9(m), g9(s), "", g9(lo), g9(hi)))
    row(nm[1], p[1], se[1], r$lb1, r$ub1)
    row(nm[2], p[2], se[2], r$lb2, r$ub2)
    cat(mid, "\n", sep = "")
    row("diff", d, sed, r$lb_diff, r$ub_diff)
    cat(sprintf("%12s |  under H0:  %9s %8.2f %7.3f\n", "", g9(se0), z, r$p))
    cat(rule, "\n", sep = "")
    cat(sprintf("%-66sz = %8.4f\n", sprintf("        diff = prop(%s) - prop(%s)", nm[1], nm[2]), z))
    cat("    H0: diff = 0\n\n")
    cat("    Ha: diff < 0                 Ha: diff != 0                 Ha: diff > 0\n")
    cat(sprintf(" Pr(Z < z) = %6.4f         Pr(|Z| > |z|) = %6.4f          Pr(Z > z) = %6.4f\n", r$p_l, r$p, r$p_u))
  }
  r
}"##;

/// The r() prtest leaves (Stata 19).
pub(super) const R_PRTEST: &[&str] = &[
    "N1", "N2", "P1", "P2", "P_diff", "se1", "se2", "se_diff0", "se_diff", "lb1", "ub1", "lb2",
    "ub2", "lb_diff", "ub_diff", "z", "p_l", "p", "p_u", "level",
];

impl Session {
    pub(super) fn prtest(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[("by", 2), ("level", 1)]) {
            return Err(no(format!(
                "prtest with the option {o} is not translated yet"
            )));
        }
        let main = c.parts.main.trim();
        if !is_name(main) {
            return Err(no(
                "prtest is translated for one variable with by() (prtest var, by(g))",
            ));
        }
        let Some(g) = opts.arg("by", 2).map(|g| g.trim().to_string()) else {
            return Err(no("prtest without by() is not translated yet"));
        };
        if !is_name(&g) {
            return Err(no(format!("prtest, by({g}): one variable is translated")));
        }
        let level = match opts.arg("level", 1) {
            None => "95".to_string(),
            Some(l) => {
                let v: f64 = l
                    .trim()
                    .parse()
                    .map_err(|_| no(format!("level({l}) needs a number")))?;
                if !(10.0..=99.99).contains(&v) {
                    return Err(no(format!(
                        "level({l}): levels from 10 to 99.99 are translated"
                    )));
                }
                r_number(v)
            }
        };
        let rows = self.rows_for(c, out)?;
        for h in ["stata_prtest", "stata_fmt", "stata_returned"] {
            self.helpers.insert(h);
        }
        out.line(format!(
            "stata_r <- stata_prtest({rows}[[{}]], {rows}[[{}]], {}, level = {level}, show = {})",
            r_text(main),
            r_text(&g),
            r_text(&g),
            super::stats::r_logical(!self.quiet(c))
        ));
        self.r_new = Some((R_PRTEST, "prtest".into()));
        Ok(())
    }
}
