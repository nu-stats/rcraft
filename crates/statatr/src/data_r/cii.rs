//! `cii means #obs #mean #sd`, `cii means #exposure #events, poisson`,
//! `cii proportions #obs #succ [, method]` and `cii variances #obs #variance
//! [#kurtosis] [, sd bonett]`: `ci` on numbers typed in, as Stata 19 prints it
//! (`plan/stata/ci.md`): an empty Variable column, and notes such as "Note:
//! The Wald CI was truncated at 0 at the lower endpoint." where `ci` marks the
//! bound instead.

use super::*;

pub(super) const CII_HELPER: &str = r##"stata_cii <- function(kind, n, a2, a3 = NA_real_, level = 95, method = "normal", sd = FALSE, show = TRUE) {
  a <- (100 - level) / 200
  z <- stats::qnorm(1 - a)
  mark <- ""
  notes <- character(0)
  title <- c(exact = "Binomial exact", wald = "Binomial Wald", wilson = "Wilson",
             agresti = "Agresti–Coull", jeffreys = "Jeffreys", poisson = "Poisson exact", bonett = "Bonett")
  short <- c(wald = "Wald", wilson = "Wilson", agresti = "Agresti–Coull")
  if (kind == "means" && method == "poisson") {
    E <- n
    tot <- a2
    o <- list(N = E, est = tot / E, se = sqrt(tot) / E, lb = if (tot > 0) stats::qgamma(a, tot) / E else 0,
              ub = stats::qgamma(1 - a, tot + 1) / E)
    if (tot == 0) mark <- "*"
  } else if (kind == "means") {
    se <- a3 / sqrt(n)
    t <- if (n > 1) stats::qt(1 - a, n - 1) else NA_real_
    o <- list(N = n, est = a2, se = se, lb = a2 - t * se, ub = a2 + t * se)
  } else if (kind == "variances") {
    v <- if (sd) a2^2 else a2
    if (method == "bonett") {
      cc <- n / (n - z)
      se <- cc * sqrt((a3 - (n - 3) / n) / (n - 1))
      o <- list(N = n, est = v, lb = exp(log(cc * v) - z * se), ub = exp(log(cc * v) + z * se))
    } else {
      o <- list(N = n, est = v, lb = (n - 1) * v / stats::qchisq(1 - a, n - 1),
                ub = (n - 1) * v / stats::qchisq(a, n - 1))
    }
  } else {
    s <- a2
    p <- s / n
    se <- sqrt(p * (1 - p) / n)
    if (method == "exact") {
      lb <- if (s == 0) 0 else stats::qbeta(a, s, n - s + 1)
      ub <- if (s == n) 1 else stats::qbeta(1 - a, s + 1, n - s)
      if (s == 0 || s == n) mark <- "*"
    } else if (method == "wald") {
      lb <- p - z * se
      ub <- p + z * se
    } else if (method == "wilson") {
      cen <- (s + z^2 / 2) / (n + z^2)
      half <- z * sqrt(n) / (n + z^2) * sqrt(p * (1 - p) + z^2 / (4 * n))
      # Stata 19 notes the lower bound at 0 successes only (its own rounding puts it below 0)
      lb <- if (s == 0) -1 else max(0, cen - half)
      ub <- min(1, cen + half)
    } else if (method == "agresti") {
      nt <- n + z^2
      pt <- (s + z^2 / 2) / nt
      lb <- pt - z * sqrt(pt * (1 - pt) / nt)
      ub <- pt + z * sqrt(pt * (1 - pt) / nt)
    } else {
      lb <- if (s == 0) 0 else stats::qbeta(a, s + .5, n - s + .5)
      ub <- if (s == n) 1 else stats::qbeta(1 - a, s + .5, n - s + .5)
    }
    if (lb < 0) {
      lb <- 0
      notes <- c(notes, paste0("Note: The ", short[[method]], " CI was truncated at 0 at the lower endpoint."))
    }
    if (ub > 1) {
      ub <- 1
      notes <- c(notes, paste0("Note: The ", short[[method]], " CI was truncated at 1 at the upper endpoint."))
    }
    o <- list(N = n, est = p, se = se, lb = lb, ub = ub)
  }
  r <- list(N = o$N)
  if (kind == "variances") {
    r$Var <- o$est
    if (sd) r$sd <- sqrt(o$est)
    if (method == "bonett") r$kurtosis <- a3
  } else {
    r[[if (kind == "means") "mean" else "proportion"]] <- o$est
    r$se <- o$se
  }
  root <- if (kind == "variances" && sd) sqrt else identity
  r$lb <- root(o$lb)
  r$ub <- root(o$ub)
  r$level <- level
  r$citype <- method
  if (!show) return(r)
  g <- function(v) stata_fmt(v, "%9.0g", trim = TRUE)
  lev <- paste0("[", stata_fmt(level, "%9.0g", trim = TRUE), "% conf. interval]")
  wide <- kind != "variances"
  cat("\n")
  if (method %in% names(title)) {
    width <- if (wide) 77 else 66
    l <- width - 22 + (24 - nchar(title[[method]])) %/% 2
    cat(strrep(" ", l), title[[method]], strrep(" ", width - l - nchar(title[[method]])), "\n", sep = "")
  }
  if (wide) {
    cat("    Variable |", if (method == "poisson") "   Exposure  " else "        Obs  ",
        if (kind == "means") "      Mean" else "Proportion", "    Std. err.", sprintf("%27s", lev), "\n", sep = "")
    cat(strrep("-", 13), "+", strrep("-", 63), "\n", sep = "")
    cat(sprintf("%12s |%11s%12s%12s%16s%12s%s", "", format(o$N, scientific = FALSE), g(o$est), g(o$se),
                g(o$lb), g(o$ub), mark), "\n", sep = "")
  } else {
    cat("    Variable |        Obs",
        if (sd) paste0("      Std. dev.", sprintf("%26s", lev)) else paste0("      Variance", sprintf("%27s", lev)),
        "\n", sep = "")
    cat(strrep("-", 13), "+", strrep("-", 52), "\n", sep = "")
    cat(sprintf("%12s |%11s%14s%15s%12s", "", format(o$N, scientific = FALSE), g(root(o$est)), g(root(o$lb)),
                g(root(o$ub))), "\n", sep = "")
  }
  if (mark == "*") cat("(*) one-sided, ", stata_fmt((100 + level) / 2, "%9.0g", trim = TRUE), "% confidence interval\n", sep = "")
  for (s in notes) cat(s, "\n", sep = "")
  invisible(r)
}"##;

const R_MEANS: &[&str] = &["N", "mean", "se", "lb", "ub", "level", "citype"];
const R_PROPORTIONS: &[&str] = &["N", "proportion", "se", "lb", "ub", "level", "citype"];

impl Session {
    pub(super) fn cii(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "cii")?;
        let (sub, rest) = first_word(&c.parts.main);
        let kind = match sub {
            "means" | "proportions" | "variances" => sub,
            "" => {
                return Err(no(
                    "you must specify one of means, proportions, or variances following cii (Stata: r(198))",
                ))
            }
            s => {
                return Err(no(format!(
                    "cii {s}: cii means, cii proportions and cii variances are translated"
                )))
            }
        };
        let methods = ["exact", "wald", "wilson", "agresti", "jeffreys"];
        let mut known: Vec<(&str, usize)> = vec![("level", 1)];
        match kind {
            "proportions" => known.extend(methods.iter().map(|m| (*m, m.len()))),
            "means" => known.push(("poisson", 7)),
            _ => known.extend([("bonett", 6), ("sd", 2)]),
        }
        if let Some(o) = opts.other(&known) {
            return Err(no(format!(
                "cii {kind} with the option {o} is not translated yet"
            )));
        }
        let nums: Vec<f64> = rest
            .split_whitespace()
            .map(|w| w.parse::<f64>())
            .collect::<Result<_, _>>()
            .map_err(|_| no(format!("cii {kind}: numbers written out are translated")))?;
        let whole = |v: f64| v.is_finite() && v.fract() == 0.0;
        let poisson = kind == "means" && opts.has("poisson", 7);
        let bonett = kind == "variances" && opts.has("bonett", 6);
        let want = match (kind, poisson, bonett) {
            ("means", false, _) | ("variances", _, true) => 3,
            _ => 2,
        };
        if nums.len() != want {
            return Err(no(format!(
                "cii {kind} takes {want} numbers here (Stata: r(198))"
            )));
        }
        let (Some(&n), Some(&a2)) = (nums.first(), nums.get(1)) else {
            return Err(no(format!("cii {kind} needs its numbers")));
        };
        if !(whole(n) && n > 0.0) {
            return Err(no(
                "first argument, #obs, must be a positive integer (Stata: r(198))",
            ));
        }
        if kind == "proportions" && !(whole(a2) && a2 >= 0.0) {
            return Err(no(
                "cii proportions with a proportion in place of #succ is not translated yet",
            ));
        }
        if kind == "proportions" && a2 > n {
            return Err(no(format!(
                "the number of successes, {a2}, cannot exceed the number of observations, {n} (Stata: r(198))"
            )));
        }
        if poisson && !(whole(a2) && a2 >= 0.0) {
            return Err(no(
                "cii means, poisson with a rate in place of #events is not translated yet",
            ));
        }
        let chosen: Vec<&str> = methods
            .iter()
            .copied()
            .filter(|m| opts.has(m, m.len()))
            .collect();
        if chosen.len() > 1 {
            return Err(no(
                "only one of exact, wald, wilson, agresti and jeffreys (Stata: r(198))",
            ));
        }
        let method = match kind {
            "proportions" => chosen.first().copied().unwrap_or("exact"),
            _ if poisson => "poisson",
            _ if bonett => "bonett",
            _ => "normal",
        };
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
        for h in ["stata_cii", "stata_fmt", "stata_returned"] {
            self.helpers.insert(h);
        }
        let sd = kind == "variances" && opts.has("sd", 2);
        out.line(format!(
            "stata_r <- stata_cii({}, {}, {}{}, level = {level}, method = {}{}{})",
            r_text(kind),
            r_number(n),
            r_number(a2),
            nums.get(2)
                .map(|v| format!(", {}", r_number(*v)))
                .unwrap_or_default(),
            r_text(method),
            if sd { ", sd = TRUE" } else { "" },
            if self.quiet(c) { ", show = FALSE" } else { "" }
        ));
        let names: &'static [&'static str] = match (kind, method, sd) {
            ("means", _, _) => R_MEANS,
            ("proportions", _, _) => R_PROPORTIONS,
            (_, "bonett", true) => &["N", "Var", "sd", "kurtosis", "lb", "ub", "level", "citype"],
            (_, "bonett", false) => &["N", "Var", "kurtosis", "lb", "ub", "level", "citype"],
            (_, _, true) => &["N", "Var", "sd", "lb", "ub", "level", "citype"],
            _ => &["N", "Var", "lb", "ub", "level", "citype"],
        };
        self.r_new = Some((names, "cii".into()));
        Ok(())
    }
}
