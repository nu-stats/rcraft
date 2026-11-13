//! `centile varlist [if] [in] [, centile(numlist) level(#)]`: centiles with
//! Stata 19's binomial interval, interpolated between order statistics, its
//! table and r() (`plan/stata/centile.md`).

use super::*;

pub(super) const CENTILE_HELPER: &str = r##"stata_centile <- function(d, vars, q = 50, qtext = "50", level = 95, show = TRUE,
                          method = "interp") {
  num <- function(v) { u <- unclass(v); attributes(u) <- NULL; as.numeric(u) }
  for (v in vars) if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
  a <- (100 - level) / 200
  # the qth centile: at rank (n + 1)q/100, between the order statistics
  at <- function(x, R) {
    n <- length(x)
    if (R < 1) return(x[1])
    if (R >= n) return(x[n])
    j <- floor(R)
    x[j] + (R - j) * (x[j + 1] - x[j])
  }
  # a bound: where the binomial's cdf passes g, between the order statistics; held at the
  # sample's end (and marked) where the order statistic needed is not there
  bound <- function(x, p, g) {
    n <- length(x)
    F <- stats::pbinom(0:n, n, p)
    j <- which(F > g)[1] - 1
    lo <- if (j >= 1) F[j] else 0
    if (j < 1) return(list(v = x[1], held = TRUE))
    if (j >= n) return(list(v = x[n], held = TRUE))
    list(v = x[j] + (g - lo) / (F[j + 1] - lo) * (x[j + 1] - x[j]), held = FALSE)
  }
  out <- list()
  for (v in vars) {
    x <- sort(num(d[[v]]))
    n <- length(x)
    rows <- list()
    z <- stats::qnorm(1 - a)
    if (n > 0) for (k in seq_along(q)) {
      p <- q[k] / 100
      cq <- at(x, (n + 1) * p)
      rows[[k]] <- switch(method,
        # cci, as Stata's manual gives it: x(r+1) and x(s+1), with x(0) = x(1) and x(n+1) = x(n)
        cci = {
          F <- stats::pbinom(0:n, n, p)
          lo <- which(F > a)[1] - 1
          hi <- which(F >= 1 - a)[1]
          list(c = cq, lb = x[max(1, lo)], ub = x[min(n, hi)], held = lo < 1 || hi > n)
        },
        # normal: the centile's se from a normal density at it, with the sample's mean and sd
        normal = {
          se <- sqrt(q[k] * (100 - q[k])) / (100 * sqrt(n) * stats::dnorm(cq, mean(x), stats::sd(x)))
          list(c = cq, lb = cq - z * se, ub = cq + z * se, held = FALSE)
        },
        # meansd: the centile of a normal with the sample's mean and sd
        meansd = {
          zq <- stats::qnorm(p)
          s <- stats::sd(x)
          cm <- mean(x) + zq * s
          se <- s * sqrt(1 / n + zq^2 / (2 * (n - 1)))
          list(c = cm, lb = cm - z * se, ub = cm + z * se, held = FALSE)
        },
        {
          lb <- bound(x, p, a)
          ub <- bound(x, p, 1 - a)
          list(c = cq, lb = lb$v, ub = ub$v, held = lb$held || ub$held)
        })
    }
    out[[v]] <- list(n = n, rows = rows)
  }
  last <- out[[length(out)]]
  r <- list(n_cent = length(q), N = last$n)
  if (last$n > 0) for (k in rev(seq_along(q))) {
    r[[paste0("ub_", k)]] <- last$rows[[k]]$ub
    r[[paste0("lb_", k)]] <- last$rows[[k]]$lb
    r[[paste0("c_", k)]] <- last$rows[[k]]$c
  }
  r$centiles <- qtext
  if (!show) return(r)
  g <- function(v) stata_fmt(v, "%9.0g", trim = TRUE)
  ab <- function(s) if (nchar(s) <= 12) s else paste0(substr(s, 1, 10), "~", substr(s, nchar(s), nchar(s)))
  cat("\n", switch(method,
    cci = paste0(strrep(" ", 58), "Binomial exact   "),
    normal = paste0(strrep(" ", 35), "-- Normal, based on observed centiles --"),
    meansd = paste0(strrep(" ", 35), "-- Normal, based on mean and std. dev.--"),
    paste0(strrep(" ", 58), "Binom. interp.   ")), "\n", sep = "")
  cat("    Variable |       Obs  Percentile    Centile",
      sprintf("%28s", paste0("[", stata_fmt(level, "%9.0g", trim = TRUE), "% conf. interval]")), "\n", sep = "")
  cat(strrep("-", 13), "+", strrep("-", 61), "\n", sep = "")
  held <- FALSE
  for (v in names(out)) {
    o <- out[[v]]
    if (o$n == 0) {
      cat(sprintf("%12s |%8s", ab(v), "0"), "\n", sep = "")
      next
    }
    for (k in seq_along(q)) {
      w <- o$rows[[k]]
      held <- held || w$held
      cat(sprintf("%12s |%10s%11s%12s%16s%12s%s", if (k == 1) ab(v) else "", if (k == 1) format(o$n, scientific = FALSE) else "",
                  g(q[k]), g(w$c), g(w$lb), g(w$ub), if (w$held) "*" else ""), "\n", sep = "")
    }
  }
  if (held) cat("* Lower (upper) confidence limit held at minimum (maximum) of sample\n")
  invisible(r)
}"##;

/// The r() centile leaves (Stata 19): the last variable's, c_1 ... for each
/// centile (only these names; any other is missing).
pub(super) fn centile_names(name: &str) -> bool {
    matches!(name, "n_cent" | "N" | "centiles")
        || ["c_", "lb_", "ub_"].iter().any(|p| {
            name.strip_prefix(p)
                .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
        })
}

impl Session {
    pub(super) fn centile(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "centile")?;
        if let Some(o) = opts.other(&[
            ("centile", 7),
            ("level", 1),
            ("cci", 3),
            ("normal", 6),
            ("meansd", 6),
        ]) {
            return Err(no(format!(
                "centile with the option {o} is not translated yet"
            )));
        }
        let methods: Vec<&str> = ["cci", "normal", "meansd"]
            .into_iter()
            .filter(|m| opts.has(m, m.len()))
            .collect();
        if methods.len() > 1 {
            return Err(no("only one of cci, normal and meansd (Stata: r(198))"));
        }
        let method = methods.first().copied().unwrap_or("interp");
        if c.parts.weight.is_some() {
            return Err(no("centile with weights is not translated yet"));
        }
        let vars: Vec<&str> = c.parts.main.split_whitespace().collect();
        if vars.is_empty() {
            return Err(no("centile needs a varlist"));
        }
        if let Some(v) = vars.iter().find(|v| !is_name(v)) {
            return Err(no(format!(
                "centile {v}: patterns and abbreviations are taken once the data's variables are known"
            )));
        }
        let (q, qtext) = match opts.arg("centile", 7) {
            None => (vec![50.0], "50".to_string()),
            Some(list) => {
                let mut q = Vec::new();
                for w in list.split_whitespace() {
                    let v: f64 = w.parse().map_err(|_| {
                        no(format!(
                            "centile({list}): only numbers are translated (not ranges)"
                        ))
                    })?;
                    if !(v > 0.0 && v < 100.0) {
                        return Err(no(format!(
                            "centile({list}): each must be above 0 and below 100 (Stata: r(198))"
                        )));
                    }
                    q.push(v);
                }
                if q.is_empty() || q.len() > 100 {
                    return Err(no(format!(
                        "centile({list}): 1 to 100 centiles are translated"
                    )));
                }
                (q, list.split_whitespace().collect::<Vec<_>>().join(" "))
            }
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
        let rows = self.rows_for(c, out)?;
        for h in ["stata_centile", "stata_fmt", "stata_returned"] {
            self.helpers.insert(h);
        }
        let names: Vec<String> = vars.iter().map(|v| r_text(v)).collect();
        let qs: Vec<String> = q.iter().map(|v| r_number(*v)).collect();
        out.line(format!(
            "stata_r <- stata_centile({rows}, c({}), q = c({}), qtext = {}, level = {level}, method = {}{})",
            names.join(", "),
            qs.join(", "),
            r_text(&qtext),
            r_text(method),
            if self.quiet(c) { ", show = FALSE" } else { "" }
        ));
        self.r_new = Some((&["n_cent", "N", "centiles"], "centile".into()));
        Ok(())
    }
}
