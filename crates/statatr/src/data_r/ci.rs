//! `ci means|proportions|variances varlist [if] [in] [weight] [, level(#)
//! exact wald wilson agresti jeffreys]`: confidence intervals as Stata 19
//! prints them, and the r() of the last variable (`plan/stata/ci.md`).

use super::weights::{w_args, W};
use super::*;

pub(super) const CI_HELPER: &str = r##"stata_ci <- function(d, kind, vars, level = 95, method = "normal", w = NULL, wtype = "f", show = TRUE) {
  num <- function(v) { u <- unclass(v); attributes(u) <- NULL; as.numeric(u) }
  for (v in vars) if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
  weighted <- !is.null(w)
  wt <- if (weighted) num(w) else rep(1, nrow(d))
  a <- (100 - level) / 200
  z <- stats::qnorm(1 - a)
  rows <- list()
  skipped <- FALSE
  for (v in vars) {
    if (is.character(d[[v]]) || is.factor(d[[v]])) { skipped <- TRUE; next }
    x <- num(d[[v]])
    ok <- !is.na(x)
    x <- x[ok]
    vw <- wt[ok]
    k <- length(x)
    if (kind == "proportions" && !all(x %in% c(0, 1))) { skipped <- TRUE; next }
    W <- sum(vw)
    N <- if (weighted && wtype == "f") W else k
    m <- if (k > 0) sum(vw * x) / W else NA_real_
    ss <- if (k > 0) sum(vw * (x - m)^2) else NA_real_
    var <- if (N <= 1) NA_real_ else if (!weighted || wtype == "f") ss / (N - 1) else ss / W * k / (k - 1)
    mark <- ""
    if (kind == "means") {
      se <- sqrt(var / N)
      t <- if (N > 1) stats::qt(1 - a, N - 1) else NA_real_
      rows[[v]] <- list(N = N, est = m, se = se, lb = m - t * se, ub = m + t * se)
    } else if (kind == "variances") {
      lb <- if (N > 1) (N - 1) * var / stats::qchisq(1 - a, N - 1) else NA_real_
      ub <- if (N > 1) (N - 1) * var / stats::qchisq(a, N - 1) else NA_real_
      rows[[v]] <- list(N = N, est = var, lb = lb, ub = ub)
    } else {
      n <- k
      s <- sum(x)
      p <- if (n > 0) s / n else NA_real_
      se <- sqrt(p * (1 - p) / n)
      if (method == "exact") {
        lb <- if (s == 0) 0 else stats::qbeta(a, s, n - s + 1)
        ub <- if (s == n) 1 else stats::qbeta(1 - a, s + 1, n - s)
        if (s == 0 || s == n) mark <- "*"
      } else if (method == "wald") {
        lb <- p - z * se
        ub <- p + z * se
        if (lb <= 0) { lb <- 0; mark <- "*" } else if (ub >= 1) { ub <- 1; mark <- "**" }
        if (ub >= 1) ub <- 1
      } else if (method == "wilson") {
        cen <- (s + z^2 / 2) / (n + z^2)
        half <- z * sqrt(n) / (n + z^2) * sqrt(p * (1 - p) + z^2 / (4 * n))
        lb <- max(0, cen - half)
        ub <- min(1, cen + half)
        if (s == 0) { lb <- 0; mark <- "*" }
      } else if (method == "agresti") {
        nt <- n + z^2
        pt <- (s + z^2 / 2) / nt
        h <- z * sqrt(pt * (1 - pt) / nt)
        lb <- pt - h
        ub <- pt + h
        if (lb < 0) { lb <- 0; mark <- "*" } else if (ub > 1) { ub <- 1; mark <- "**" }
        if (ub > 1) ub <- 1
      } else {
        lb <- if (s == 0) 0 else stats::qbeta(a, s + .5, n - s + .5)
        ub <- if (s == n) 1 else stats::qbeta(1 - a, s + .5, n - s + .5)
      }
      rows[[v]] <- list(N = n, est = p, se = se, lb = lb, ub = ub, mark = mark)
    }
  }
  if (kind == "proportions" && !length(rows)) {
    # Stata 19 says so and goes on (_rc 0), with r() left empty
    if (show) cat("no binary (0/1) variables found; nothing to compute\n")
    return(list())
  }
  last <- if (length(rows)) rows[[length(rows)]] else NULL
  r <- list()
  if (!is.null(last)) {
    r$N <- last$N
    if (kind == "variances") r$Var <- last$est else r[[if (kind == "means") "mean" else "proportion"]] <- last$est
    if (kind != "variances") r$se <- last$se
    r$lb <- last$lb
    r$ub <- last$ub
    r$level <- level
    r$citype <- if (kind == "proportions") method else "normal"
  }
  if (!show) return(r)
  g <- function(v) stata_fmt(v, "%9.0g", trim = TRUE)
  ab <- function(s) if (nchar(s) <= 12) s else paste0(substr(s, 1, 10), "~", substr(s, nchar(s), nchar(s)))
  lev <- paste0("[", stata_fmt(level, "%9.0g", trim = TRUE), "% conf. interval]")
  wide <- kind != "variances"
  title <- c(exact = "Binomial exact", wald = "Binomial Wald", wilson = "Wilson",
             agresti = "Agresti–Coull", jeffreys = "Jeffreys")[[if (kind == "proportions") method else "exact"]]
  cat("\n")
  if (kind == "proportions") {
    l <- 55 + (24 - nchar(title)) %/% 2
    cat(strrep(" ", l), title, strrep(" ", 77 - l - nchar(title)), "\n", sep = "")
  }
  if (wide) {
    cat("    Variable |        Obs  ", if (kind == "means") "      Mean" else "Proportion", "    Std. err.", sprintf("%27s", lev), "\n", sep = "")
  } else {
    cat("    Variable |        Obs      Variance", sprintf("%27s", lev), "\n", sep = "")
  }
  rule <- paste0(strrep("-", 13), "+", strrep("-", if (wide) 63 else 52), "\n")
  cat(rule)
  i <- 0
  for (v in names(rows)) {
    i <- i + 1
    if (i > 1 && (i - 1) %% 5 == 0) cat(rule)
    o <- rows[[v]]
    if (wide) {
      cat(sprintf("%12s |%11s%12s%12s%16s%12s%s", ab(v), format(o$N, scientific = FALSE), g(o$est), g(o$se), g(o$lb), g(o$ub),
                  if (is.null(o$mark)) "" else o$mark), "\n", sep = "")
    } else {
      cat(sprintf("%12s |%11s%14s%15s%12s", ab(v), format(o$N, scientific = FALSE), g(o$est), g(o$lb), g(o$ub)), "\n", sep = "")
    }
  }
  marks <- unlist(lapply(rows, function(o) o$mark))
  if (kind == "proportions" && any(marks != "")) {
    if (method == "exact") {
      cat("(*) one-sided, ", stata_fmt((100 + level) / 2, "%9.0g", trim = TRUE), "% confidence interval\n", sep = "")
    } else {
      short <- c(wald = "Wald", wilson = "Wilson", agresti = "Agresti–Coull")[[method]]
      if (any(marks == "*")) cat("(*) The ", short, " interval was clipped at the lower endpoint\n", sep = "")
      if (any(marks == "**")) cat("(**) The ", short, " interval was clipped at the upper endpoint\n", sep = "")
    }
  }
  if (kind == "proportions" && skipped) cat("Note: The results are produced only for binary (0/1) variables.\n")
  invisible(r)
}"##;

/// The r() each `ci` leaves (Stata 19), the last variable's.
const R_CI_MEANS: &[&str] = &["N", "mean", "se", "lb", "ub", "level", "citype"];
const R_CI_PROPORTIONS: &[&str] = &["N", "proportion", "se", "lb", "ub", "level", "citype"];
const R_CI_VARIANCES: &[&str] = &["N", "Var", "lb", "ub", "level", "citype"];

impl Session {
    pub(super) fn ci(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "ci")?;
        let (sub, rest) = first_word(&c.parts.main);
        let (kind, names): (&str, &'static [&'static str]) = match sub {
            "means" => ("means", R_CI_MEANS),
            "proportions" => ("proportions", R_CI_PROPORTIONS),
            "variances" => ("variances", R_CI_VARIANCES),
            "" => {
                return Err(no(
                    "you must specify one of means, proportions, or variances following ci (Stata: r(198))",
                ))
            }
            s => {
                return Err(no(format!(
                    "ci {s}: ci means, ci proportions and ci variances are translated"
                )))
            }
        };
        let methods = ["exact", "wald", "wilson", "agresti", "jeffreys"];
        let mut known: Vec<(&str, usize)> = vec![("level", 1)];
        if kind == "proportions" {
            known.extend(methods.iter().map(|m| (*m, m.len())));
        }
        if let Some(o) = opts.other(&known) {
            return Err(no(format!(
                "ci {kind} with the option {o} is not translated yet"
            )));
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
        let method = if kind == "proportions" {
            chosen.first().copied().unwrap_or("exact")
        } else {
            "normal"
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
        let vars: Vec<&str> = rest.split_whitespace().collect();
        if vars.is_empty() {
            return Err(no(format!("ci {kind} needs a varlist")));
        }
        if let Some(v) = vars.iter().find(|v| !is_name(v)) {
            return Err(no(format!(
                "ci {kind} {v}: patterns and abbreviations are not translated here yet"
            )));
        }
        let allowed: &[W] = if kind == "means" { &[W::F, W::A] } else { &[] };
        let w = self.weight_of(c, allowed)?;
        let mut rows = self.rows_for(c, out)?;
        if w.is_some() {
            rows = self.weighted_rows(rows, &w);
            out.line(format!("stata_rows <- {rows}"));
            rows = "stata_rows".into();
            out.caveat(super::weights::weight_caveat(&w));
        }
        let wargs = w_args(&rows, &w);
        for h in ["stata_ci", "stata_fmt", "stata_returned"] {
            self.helpers.insert(h);
        }
        let list: Vec<String> = vars.iter().map(|v| r_text(v)).collect();
        out.line(format!(
            "stata_r <- stata_ci({rows}, {}, c({}), level = {level}, method = {}{wargs}{})",
            r_text(kind),
            list.join(", "),
            r_text(method),
            if self.quiet(c) { ", show = FALSE" } else { "" }
        ));
        self.r_new = Some((names, "ci".into()));
        Ok(())
    }
}
