//! The epidemiological tables `cs`, `csi`, `cc`, `cci`, `ir` and `iri` as
//! Stata 19 prints them, and the r() they leave (`plan/stata/epitab.md`):
//! risks, odds or rates of the exposed and the unexposed, their difference
//! and ratio with Wald (log) intervals, Woolf's or the test-based interval
//! of the odds ratio, the exact Poisson interval of the rate ratio, the
//! attributable (or, below 1, prevented) fractions, Pearson's chi2 and
//! Fisher's exact p. A nonzero exposure or case value counts as one, as in
//! Stata.

use super::*;

pub(super) const EPITAB_HELPER: &str = r##"stata_epi_counts <- function(d, case, expo, time = NULL) {
  y <- stata_epi_num(d[[case]])
  x <- stata_epi_num(d[[expo]])
  t <- if (is.null(time)) NULL else stata_epi_num(d[[time]])
  ok <- !is.na(y) & !is.na(x) & (if (is.null(t)) TRUE else !is.na(t))
  y <- y[ok] != 0
  x <- x[ok] != 0
  lab <- function(v, name) {
    l <- attr(d[[v]], "label", exact = TRUE)
    if (is.character(l) && length(l) == 1 && nzchar(l)) l else name
  }
  r <- list(expo = lab(expo, expo), case = lab(case, case))
  if (is.null(t)) {
    c(r, list(a = sum(y & x), b = sum(y & !x), c = sum(!y & x), d = sum(!y & !x)))
  } else {
    t <- t[ok]
    c(r, list(a = sum(y & x), b = sum(y & !x), t1 = sum(t[x]), t0 = sum(t[!x]), time = lab(time, time)))
  }
}
stata_epi_num <- function(v) {
  if (inherits(v, "haven_labelled")) v <- unclass(v)
  as.numeric(v)
}
stata_epi <- function(kind, a, b, c = NA, d = NA, t1 = NA, t0 = NA, level = 95, ci = "woolf", exact = FALSE,
                      expo = NULL, case = NULL, time = NULL, show = TRUE) {
  z <- stats::qnorm(1 - (100 - level) / 200)
  g <- function(v) stata_fmt(v, "%9.0g", trim = TRUE)
  rowl <- function(lab, x, y, w) sprintf("%16s |%10s%12s  |%11s", lab, x, y, w)
  est <- function(lab, v, lo = NULL, hi = NULL, how = "") {
    if (is.null(lo)) return(sprintf("%16s |%17s       |", lab, g(v)))
    sprintf("%16s |%17s       |%12s%12s %s", lab, g(v), g(lo), g(hi), how)
  }
  center <- function(s, w) {
    left <- max(0, (w - nchar(s)) %/% 2)
    paste0(strrep(" ", left), s, strrep(" ", max(0, w - left - nchar(s))))
  }
  top <- sprintf("%16s |      Point estimate    |    [%s%% conf. interval]", "", g(level))
  bar <- paste0(strrep(" ", 17), "|", strrep("-", 24), "+", strrep("-", 24))
  foot <- paste0(strrep(" ", 17), "+", strrep("-", 49))
  gap <- paste0(strrep(" ", 17), "|", strrep(" ", 24), "|")
  frac <- function(ratio, lo, hi) {
    # attributable above 1, prevented below (Stata 19)
    if (ratio >= 1) list(f = (ratio - 1) / ratio, lo = (lo - 1) / lo, hi = (hi - 1) / hi, word = "Attr.")
    else list(f = 1 - ratio, lo = 1 - hi, hi = 1 - lo, word = "Prev.")
  }
  if (kind == "ir") {
    n <- a + b
    tt <- t1 + t0
    ird <- a / t1 - b / t0
    sd <- sqrt(a / t1^2 + b / t0^2)
    ci_r <- stats::poisson.test(c(a, b), c(t1, t0), conf.level = level / 100)$conf.int
    irr <- (a / t1) / (b / t0)
    fr <- frac(irr, ci_r[1], ci_r[2])
    r0 <- b / t0
    rt <- n / tt
    afp <- if (irr >= 1) (rt - r0) / rt else (r0 - rt) / r0
    pr <- t1 / tt
    lower_e <- stats::pbinom(a, n, pr)
    upper_e <- stats::pbinom(a - 1, n, pr, lower.tail = FALSE)
    at <- stats::dbinom(a, n, pr)
    lower_m <- lower_e - at / 2
    upper_m <- upper_e - at / 2
    r <- list(ird = ird, lb_ird = ird - z * sd, ub_ird = ird + z * sd, irr = irr, lb_irr = ci_r[1], ub_irr = ci_r[2],
              afe = fr$f, lb_afe = fr$lo, ub_afe = fr$hi, afp = afp, p_lower_midp = lower_m, p_upper_midp = upper_m,
              p_twosided_midp = min(1, 2 * min(lower_m, upper_m)), p_lower_exact = lower_e, p_upper_exact = upper_e,
              p_twosided_exact = min(1, 2 * min(lower_e, upper_e)))
    if (!show) return(r)
    cn <- if (is.null(case)) "Cases" else case
    tn <- if (is.null(time)) "Person-time" else time
    cat("\nIncidence-rate comparison\n\n")
    if (!is.null(expo)) cat(sprintf("%16s |%s|", "", center(expo, 24)), "\n", sep = "")
    cat(rowl("", "Exposed", "Unexposed", "Total"), "\n", sep = "")
    cat(strrep("-", 17), "+", strrep("-", 24), "+", strrep("-", 11), "\n", sep = "")
    cat(rowl(cn, g(a), g(b), g(n)), "\n", rowl(tn, g(t1), g(t0), g(tt)), "\n", sep = "")
    cat(strrep("-", 17), "+", strrep("-", 24), "+", strrep("-", 11), "\n", sep = "")
    cat(gap, "\n", rowl("Incidence rate", g(a / t1), g(r0), g(rt)), "\n", gap, "\n", top, "\n", bar, "\n", sep = "")
    cat(est("Inc. rate diff.", ird, r$lb_ird, r$ub_ird), "\n", sep = "")
    cat(est("Inc. rate ratio", irr, ci_r[1], ci_r[2], "(exact)"), "\n", sep = "")
    cat(est(paste(fr$word, "frac. ex."), fr$f, fr$lo, fr$hi, "(exact)"), "\n", sep = "")
    cat(est(paste(fr$word, "frac. pop"), afp), "\n", foot, "\n", sep = "")
    cat("Mid-p-values for tests of incidence-rate difference:\n")
    cw <- if (is.null(case)) "cases" else case
    l1 <- sprintf("  Adj Pr(Exposed %s <= %s)", cw, g(a))
    l2 <- sprintf("  Adj Pr(Exposed %s >= %s)", cw, g(a))
    cat(sprintf("%s = %6.4f (lower one-sided)", l1, lower_m), "\n", sep = "")
    cat(sprintf("%s = %6.4f (upper one-sided)", l2, upper_m), "\n", sep = "")
    cat(sprintf("%*s = %6.4f", nchar(l1), "Two-sided p-value", r$p_twosided_midp), "\n\n", sep = "")
    return(invisible(r))
  }
  n1 <- a + c
  n0 <- b + d
  m1 <- a + b
  m0 <- c + d
  N <- n1 + n0
  e <- (m1 * n1 / N)
  chi2 <- N * (a * d - b * c)^2 / (m1 * m0 * n1 * n0)
  p <- stats::pchisq(chi2, 1, lower.tail = FALSE)
  if (kind == "cs") {
    p1 <- a / n1
    p0 <- b / n0
    pt <- m1 / N
    rd <- p1 - p0
    sd <- sqrt(p1 * (1 - p1) / n1 + p0 * (1 - p0) / n0)
    rr <- p1 / p0
    sl <- sqrt(1 / a - 1 / n1 + 1 / b - 1 / n0)
    lr <- exp(log(rr) - z * sl)
    ur <- exp(log(rr) + z * sl)
    fr <- frac(rr, lr, ur)
    afp <- if (rr >= 1) (pt - p0) / pt else (p0 - pt) / p0
    r <- list(afp = afp, ub_afe = fr$hi, lb_afe = fr$lo, afe = fr$f, ub_rr = ur, lb_rr = lr, rr = rr,
              ub_rd = rd + z * sd, lb_rd = rd - z * sd, rd = rd)
    if (exact) {
      m <- matrix(c(a, c, b, d), 2)
      ft <- stats::fisher.test(m)$p.value
      one <- min(stats::fisher.test(m, alternative = "greater")$p.value, stats::fisher.test(m, alternative = "less")$p.value)
      r <- c(r, list(p_exact = ft, p1_exact = one))
    } else {
      r <- c(r, list(p = p, chi2 = chi2))
    }
    if (!show) return(r)
    cat("\n")
    if (!is.null(expo)) cat(sprintf("%16s |%s|", "", center(expo, 24)), "\n", sep = "")
    cat(rowl("", "Exposed", "Unexposed", "Total"), "\n", sep = "")
    cat(strrep("-", 17), "+", strrep("-", 24), "+", strrep("-", 11), "\n", sep = "")
    cat(rowl("Cases", g(a), g(b), g(m1)), "\n", rowl("Noncases", g(c), g(d), g(m0)), "\n", sep = "")
    cat(strrep("-", 17), "+", strrep("-", 24), "+", strrep("-", 11), "\n", sep = "")
    cat(rowl("Total", g(n1), g(n0), g(N)), "\n", gap, "\n", rowl("Risk", g(p1), g(p0), g(pt)), "\n", gap, "\n", sep = "")
    cat(top, "\n", bar, "\n", sep = "")
    cat(est("Risk difference", rd, r$lb_rd, r$ub_rd), "\n", est("Risk ratio", rr, lr, ur), "\n", sep = "")
    cat(est(paste(fr$word, "frac. ex."), fr$f, fr$lo, fr$hi), "\n", est(paste(fr$word, "frac. pop"), afp), "\n", sep = "")
    cat(foot, "\n", sep = "")
  } else {
    or <- (a * d) / (b * c)
    if (ci == "tb") {
      # the test-based interval on the Mantel-Haenszel chi2, (N - 1)/N Pearson's (Stata 19)
      ends <- or^(1 + c(-1, 1) * z / sqrt(chi2 * (N - 1) / N))
      lo <- min(ends)
      hi <- max(ends)
      how <- "(tb)"
    } else {
      sw <- sqrt(1 / a + 1 / b + 1 / c + 1 / d)
      lo <- exp(log(or) - z * sw)
      hi <- exp(log(or) + z * sw)
      how <- "(Woolf)"
    }
    fr <- frac(or, lo, hi)
    afp <- if (or >= 1) fr$f * a / m1 else fr$f * c / m0
    r <- list(afp = afp, ub_afe = fr$hi, lb_afe = fr$lo, chi2 = chi2, p = p, afe = fr$f, ub_or = hi, lb_or = lo, or = or)
    if (!show) return(r)
    cat("\n")
    if (!is.null(expo)) {
      cat(sprintf("%16s |%s|%24s", "", center(expo, 24), "Proportion"), "\n", sep = "")
    } else {
      cat(strrep(" ", 57), "Proportion\n", sep = "")
    }
    pr <- function(lab, x, y, w, q) paste0(rowl(lab, x, y, w), sprintf("%13.4f", q))
    cat(rowl("", "Exposed", "Unexposed", "Total"), "      exposed\n", sep = "")
    cat(strrep("-", 17), "+", strrep("-", 24), "+", strrep("-", 24), "\n", sep = "")
    cat(pr("Cases", g(a), g(b), g(m1), a / m1), "\n", pr("Controls", g(c), g(d), g(m0), c / m0), "\n", sep = "")
    cat(strrep("-", 17), "+", strrep("-", 24), "+", strrep("-", 24), "\n", sep = "")
    cat(pr("Total", g(n1), g(n0), g(N), n1 / N), "\n", gap, "\n", top, "\n", bar, "\n", sep = "")
    cat(est("Odds ratio", or, lo, hi, how), "\n", est(paste(fr$word, "frac. ex."), fr$f, fr$lo, fr$hi, how), "\n", sep = "")
    cat(est(paste(fr$word, "frac. pop"), afp), "\n", foot, "\n", sep = "")
  }
  if (kind == "cs" && exact) {
    cat(sprintf("%34s1-sided Fisher's exact P = %6.4f", "", r$p1_exact), "\n", sep = "")
    cat(sprintf("%34s2-sided Fisher's exact P = %6.4f", "", r$p_exact), "\n\n", sep = "")
  } else {
    cat(sprintf("%31schi2(1) = %8.2f  Pr>chi2 = %6.4f", "", chi2, p), "\n\n", sep = "")
  }
  invisible(r)
}"##;

/// The r() each leaves (Stata 19).
const R_CS: &[&str] = &[
    "afp", "ub_afe", "lb_afe", "afe", "ub_rr", "lb_rr", "rr", "ub_rd", "lb_rd", "rd", "p", "chi2",
];
const R_CS_EXACT: &[&str] = &[
    "afp", "ub_afe", "lb_afe", "afe", "ub_rr", "lb_rr", "rr", "ub_rd", "lb_rd", "rd", "p_exact",
    "p1_exact",
];
const R_CC: &[&str] = &[
    "afp", "ub_afe", "lb_afe", "chi2", "p", "afe", "ub_or", "lb_or", "or",
];
const R_IR: &[&str] = &[
    "ird",
    "lb_ird",
    "ub_ird",
    "irr",
    "lb_irr",
    "ub_irr",
    "afe",
    "lb_afe",
    "ub_afe",
    "afp",
    "p_lower_midp",
    "p_upper_midp",
    "p_twosided_midp",
    "p_lower_exact",
    "p_upper_exact",
    "p_twosided_exact",
];

impl Session {
    pub(super) fn epitab(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let cmd = c.name.as_str();
        self.no_by(c, cmd)?;
        if c.parts.weight.is_some() {
            return Err(no(format!("{cmd} with weights is not translated yet")));
        }
        let base = cmd.trim_end_matches('i');
        let immediate = cmd.ends_with('i');
        let known: &[(&str, usize)] = match base {
            "cs" => &[("level", 1), ("exact", 5)],
            "cc" => &[("level", 1), ("woolf", 5), ("tb", 2)],
            _ => &[("level", 1)],
        };
        if let Some(o) = opts.other(known) {
            return Err(no(format!(
                "{cmd} with the option {o} is not translated yet"
            )));
        }
        if base == "cc" && !opts.has("woolf", 5) && !opts.has("tb", 2) {
            return Err(no(format!(
                "{cmd}'s exact interval of the odds ratio (its default) is not translated yet: Stata 19's search for it stops short of the exact bound; woolf and tb are translated"
            )));
        }
        if opts.has("woolf", 5) && opts.has("tb", 2) {
            return Err(no(format!("{cmd}: woolf and tb together (Stata: r(198))")));
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
        let main = c.parts.main.trim();
        let words: Vec<&str> = main.split_whitespace().collect();
        let mut extra = format!(", level = {level}");
        if base == "cc" && opts.has("tb", 2) {
            extra.push_str(", ci = \"tb\"");
        }
        if base == "cs" && opts.has("exact", 5) {
            extra.push_str(", exact = TRUE");
        }
        if self.quiet(c) {
            extra.push_str(", show = FALSE");
        }
        let kind = r_text(base);
        let call = if immediate {
            if words.len() != 4 {
                return Err(no(format!("{cmd} takes four numbers (Stata: r(198))")));
            }
            let mut v = Vec::new();
            for w in &words {
                let x: f64 = w
                    .parse()
                    .ok()
                    .filter(|x: &f64| x.is_finite() && *x >= 0.0)
                    .ok_or_else(|| no(format!("{cmd} {main}: four counts are translated")))?;
                v.push(r_number(x));
            }
            let [a, b, c3, d] = v.as_slice() else {
                return Err(no(format!("{cmd} takes four numbers (Stata: r(198))")));
            };
            if base == "ir" {
                format!("stata_epi({kind}, {a}, {b}, t1 = {c3}, t0 = {d}{extra})")
            } else {
                format!("stata_epi({kind}, {a}, {b}, {c3}, {d}{extra})")
            }
        } else {
            let need = if base == "ir" { 3 } else { 2 };
            if words.len() != need || words.iter().any(|w| !is_name(w)) {
                return Err(no(format!(
                    "{cmd}: {} variable names written out are translated",
                    if need == 3 {
                        "the case, exposure and time"
                    } else {
                        "the case and exposure"
                    }
                )));
            }
            let rows = self.rows_for(c, out)?;
            let q: Vec<String> = words.iter().map(|w| r_text(w)).collect();
            out.line(format!(
                "stata_epi_n <- stata_epi_counts({rows}, {})",
                q.join(", ")
            ));
            if base == "ir" {
                format!("stata_epi({kind}, stata_epi_n$a, stata_epi_n$b, t1 = stata_epi_n$t1, t0 = stata_epi_n$t0, expo = stata_epi_n$expo, case = stata_epi_n$case, time = stata_epi_n$time{extra})")
            } else {
                format!("stata_epi({kind}, stata_epi_n$a, stata_epi_n$b, stata_epi_n$c, stata_epi_n$d, expo = stata_epi_n$expo{extra})")
            }
        };
        for h in ["stata_epitab", "stata_fmt", "stata_returned"] {
            self.helpers.insert(h);
        }
        out.line(format!("stata_r <- {call}"));
        let names = match base {
            "cs" if opts.has("exact", 5) => R_CS_EXACT,
            "cs" => R_CS,
            "cc" => R_CC,
            _ => R_IR,
        };
        self.r_new = Some((names, cmd.into()));
        Ok(())
    }
}
