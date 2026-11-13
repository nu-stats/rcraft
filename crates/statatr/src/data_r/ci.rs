//! `ci means|proportions|variances varlist [if] [in] [weight] [, level(#)
//! exact wald wilson agresti jeffreys]`: confidence intervals as Stata 19
//! prints them, and the r() of the last variable (`plan/stata/ci.md`).

use super::weights::{w_args, W};
use super::*;

pub(super) const CI_HELPER: &str = r##"stata_ci <- function(d, kind, vars, level = 95, method = "normal", w = NULL, wtype = "f", show = TRUE,
                     exposure = NULL, sd = FALSE) {
  num <- function(v) { u <- unclass(v); attributes(u) <- NULL; as.numeric(u) }
  for (v in c(vars, exposure)) if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
  weighted <- !is.null(w)
  wt <- if (weighted) num(w) else rep(1, nrow(d))
  ex <- if (is.null(exposure)) NULL else num(d[[exposure]])
  a <- (100 - level) / 200
  z <- stats::qnorm(1 - a)
  rows <- list()
  skipped <- FALSE
  for (v in vars) {
    if (is.character(d[[v]]) || is.factor(d[[v]])) { skipped <- TRUE; next }
    x <- num(d[[v]])
    ok <- !is.na(x) & (if (is.null(ex)) TRUE else !is.na(ex))
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
    if (kind == "means" && method == "poisson") {
      # the exact interval for a Poisson count, as rates over the exposure
      E <- if (is.null(ex)) k else sum(ex[ok])
      # Stata 19 takes the total count rounded (half up): 0.4 ten times is 4, 2.25 ten times 23
      tot <- floor(sum(x) + 0.5)
      lb <- if (tot > 0) stats::qgamma(a, tot) / E else 0
      ub <- stats::qgamma(1 - a, tot + 1) / E
      rows[[v]] <- list(N = E, est = tot / E, se = sqrt(tot) / E, lb = lb, ub = ub, mark = if (tot == 0) "*" else "")
    } else if (kind == "means") {
      se <- sqrt(var / N)
      t <- if (N > 1) stats::qt(1 - a, N - 1) else NA_real_
      rows[[v]] <- list(N = N, est = m, se = se, lb = m - t * se, ub = m + t * se)
    } else if (kind == "variances" && method == "bonett") {
      # Bonett (2006) as Stata's manual gives it: the kurtosis about a trimmed mean,
      # c = n/(n - z) and the interval on the log scale
      tm <- mean(x, trim = 1 / (2 * sqrt(k - 4)))
      kurt <- k * sum((x - tm)^4) / sum((x - m)^2)^2
      cc <- k / (k - z)
      se <- cc * sqrt((kurt - (k - 3) / k) / (k - 1))
      rows[[v]] <- list(N = N, est = var, lb = exp(log(cc * var) - z * se), ub = exp(log(cc * var) + z * se),
                        kurtosis = kurt)
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
    if (kind == "variances" && sd) r$sd <- sqrt(last$est)
    if (!is.null(last$kurtosis)) r$kurtosis <- last$kurtosis
    root <- if (kind == "variances" && sd) sqrt else identity
    r$lb <- root(last$lb)
    r$ub <- root(last$ub)
    r$level <- level
    r$citype <- method
    if (!is.null(exposure)) r$exposure <- exposure
  }
  # with sd, the standard deviations and their bounds
  if (kind == "variances" && sd) for (v in names(rows)) for (f in c("est", "lb", "ub")) rows[[v]][[f]] <- sqrt(rows[[v]][[f]])
  if (!show) return(r)
  g <- function(v) stata_fmt(v, "%9.0g", trim = TRUE)
  ab <- function(s) if (nchar(s) <= 12) s else paste0(substr(s, 1, 10), "~", substr(s, nchar(s), nchar(s)))
  lev <- paste0("[", stata_fmt(level, "%9.0g", trim = TRUE), "% conf. interval]")
  wide <- kind != "variances"
  titles <- c(exact = "Binomial exact", wald = "Binomial Wald", wilson = "Wilson",
              agresti = "Agresti–Coull", jeffreys = "Jeffreys", poisson = "Poisson exact", bonett = "Bonett")
  title <- if (method %in% names(titles)) titles[[method]] else ""
  cat("\n")
  if (nzchar(title)) {
    # centred over the interval's columns
    width <- if (wide) 77 else 66
    l <- width - 22 + (24 - nchar(title)) %/% 2
    cat(strrep(" ", l), title, strrep(" ", width - l - nchar(title)), "\n", sep = "")
  }
  if (wide) {
    cat("    Variable |", if (method == "poisson") "   Exposure  " else "        Obs  ",
        if (kind == "means") "      Mean" else "Proportion", "    Std. err.", sprintf("%27s", lev), "\n", sep = "")
  } else if (sd) {
    cat("    Variable |        Obs      Std. dev.", sprintf("%26s", lev), "\n", sep = "")
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
      cat(sprintf("%12s |%11s%12s%12s%16s%12s%s", ab(v), if (o$N == round(o$N)) format(o$N, scientific = FALSE) else g(o$N),
                  g(o$est), g(o$se), g(o$lb), g(o$ub),
                  if (is.null(o$mark)) "" else o$mark), "\n", sep = "")
    } else {
      cat(sprintf("%12s |%11s%14s%15s%12s", ab(v), format(o$N, scientific = FALSE), g(o$est), g(o$lb), g(o$ub)), "\n", sep = "")
    }
  }
  marks <- unlist(lapply(rows, function(o) o$mark))
  if (kind != "variances" && any(marks != "")) {
    if (method %in% c("exact", "poisson")) {
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
        match kind {
            "proportions" => known.extend(methods.iter().map(|m| (*m, m.len()))),
            "means" => known.extend([("poisson", 7), ("exposure", 8)]),
            _ => known.extend([("bonett", 6), ("sd", 2)]),
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
        let exposure = match opts.arg("exposure", 8).map(|e| e.trim().to_string()) {
            None => None,
            Some(e) if is_name(&e) => Some(e),
            Some(e) => return Err(no(format!("exposure({e}): one variable is translated"))),
        };
        let method = match kind {
            "proportions" => chosen.first().copied().unwrap_or("exact"),
            "means" if opts.has("poisson", 7) || exposure.is_some() => "poisson",
            "variances" if opts.has("bonett", 6) => "bonett",
            _ => "normal",
        };
        if method == "poisson" && c.parts.weight.is_some() {
            return Err(no("ci means, poisson with weights is not translated yet"));
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
            "stata_r <- stata_ci({rows}, {}, c({}), level = {level}, method = {}{wargs}{}{}{})",
            r_text(kind),
            list.join(", "),
            r_text(method),
            exposure
                .as_ref()
                .map(|e| format!(", exposure = {}", r_text(e)))
                .unwrap_or_default(),
            if kind == "variances" && opts.has("sd", 2) {
                ", sd = TRUE"
            } else {
                ""
            },
            if self.quiet(c) { ", show = FALSE" } else { "" }
        ));
        // the names this form leaves (Stata 19)
        let names: &'static [&'static str] =
            match (kind, method, opts.has("sd", 2), exposure.is_some()) {
                ("means", "poisson", _, true) => {
                    &["N", "mean", "se", "lb", "ub", "level", "citype", "exposure"]
                }
                ("variances", "bonett", true, _) => {
                    &["N", "Var", "sd", "kurtosis", "lb", "ub", "level", "citype"]
                }
                ("variances", "bonett", false, _) => {
                    &["N", "Var", "kurtosis", "lb", "ub", "level", "citype"]
                }
                ("variances", _, true, _) => &["N", "Var", "sd", "lb", "ub", "level", "citype"],
                _ => names,
            };
        self.r_new = Some((names, "ci".into()));
        Ok(())
    }
}
