//! `nl (depvar = <expression with {b0 = #} parameters>) [if] [in] [, variables()
//! level() nolog iterate()]`: nonlinear least squares as Stata 19 runs it, worked
//! out from its output on probes (`plan/stata/nl.md`), not from its code.
//!
//! - The starting values as `%9.0g` writes them (123456.789 is 123456.8), 0 for a
//!   parameter given none; the rows are those where the dependent variable and the
//!   expression at the start are not missing (and variables() are not).
//! - Gauss-Newton with exact derivatives (central differences here), the step
//!   halved while the residual SS rises and stepped further while it falls (as ml
//!   steps); done, after a step, when the parameters moved by at most 1e-6 or the
//!   residual SS by at most 1e-7 (relative differences) and g'H^-1 g is at most
//!   1e-5. The iteration log is each point's residual SS: line for line on eight
//!   probe models (exponential, logistic, power, log-linear, a lognormal shape).
//! - A parameter whose derivative is the same on every row is the constant: R2 and
//!   the total SS are about the mean (with Stata's note), uncentered otherwise.
//! - The table: regress's ANOVA block with Number of obs, R-squared, Adj
//!   R-squared, Root MSE and Res. dev. (N (1 + ln(2 pi RSS / N))), and the
//!   parameters as /name, t on N - k; e(b) is the parameters, e(V) s2 (J'J)^-1.

use super::model::level;
use super::*;

/// The fit, the iteration log, Stata's table and e().
pub(super) const NL_HELPER: &str = r#"stata_nl <- function(d, depvar, init, f, vars = character(), level = 0.95, show = TRUE,
                     log = TRUE, iterate = 16000) {
  clash <- intersect(names(init), names(d))
  if (length(clash)) stop("RCraft: nl with a parameter named as a variable (", clash[1], ") is not translated yet", call. = FALSE)
  # the starting values as %9.0g writes them (Stata 19)
  b <- vapply(init, function(v) as.numeric(trimws(stata_fmt(v, "%9.0g"))), 0)
  ev <- function(b, d) as.numeric(f(c(as.list(d), as.list(b))))
  y <- as.numeric(d[[depvar]])
  f0 <- ev(b, d)
  use <- !is.na(y) & is.finite(f0)
  for (v in vars) use <- use & !is.na(d[[v]])
  d <- d[use, , drop = FALSE]
  y <- y[use]
  N <- length(y)
  k <- length(b)
  if (N <= k) stop("insufficient observations (Stata: r(2001))", call. = FALSE)
  rss <- function(b) sum((y - ev(b, d))^2)
  jac <- function(b) {
    vapply(seq_len(k), function(j) {
      h <- 1e-6 * (abs(b[j]) + 1e-3)
      up <- b; up[j] <- up[j] + h
      dn <- b; dn[j] <- dn[j] - h
      (ev(up, d) - ev(dn, d)) / (2 * h)
    }, numeric(N))
  }
  if (show && log) cat("\n")
  s <- rss(b)
  it <- 0
  converged <- FALSE
  repeat {
    if (show && log) cat("Iteration ", it, ":  Residual SS = ", stata_fmt(s, "%10.0g"), "  \n", sep = "")
    if (it >= iterate) break
    J <- matrix(jac(b), N, k)
    step <- qr.solve(J, y - ev(b, d), tol = 1e-12)
    # the full step, halved while the residual SS rises; else stepped further (1/8,
    # then doubling) while it falls (as Stata's ml steps)
    t <- 1
    s1 <- rss(b + step)
    halved <- FALSE
    while ((!is.finite(s1) || s1 > s) && t > 1e-10) {
      halved <- TRUE
      t <- t / 2
      s1 <- rss(b + t * step)
    }
    if (!halved) {
      ex <- 0.125
      repeat {
        s2 <- rss(b + (t + ex) * step)
        if (is.finite(s2) && s2 < s1) {
          t <- t + ex
          s1 <- s2
          ex <- 2 * ex
        } else break
      }
    }
    nb <- b + t * step
    moved <- max(abs(nb - b) / (abs(b) + 1))
    fell <- abs(s1 - s) / (abs(s) + 1)
    b <- nb
    s <- s1
    it <- it + 1
    # done (ml's rule on the residual SS): the parameters by at most 1e-6 or the
    # residual SS by at most 1e-7 (relative differences), and g'H^-1 g at most 1e-5
    Jn <- matrix(jac(b), N, k)
    rn <- y - ev(b, d)
    nr <- 2 * sum((Jn %*% qr.solve(Jn, rn, tol = 1e-12))^2)
    if ((moved <= 1e-6 || fell <= 1e-7) && nr <= 1e-5) {
      converged <- TRUE
      if (show && log) cat("Iteration ", it, ":  Residual SS = ", stata_fmt(s, "%10.0g"), "  \n", sep = "")
      break
    }
  }
  J <- matrix(jac(b), N, k)
  # a parameter whose derivative is the same on every row is the constant
  cons <- NA_character_
  for (j in seq_len(k)) {
    col <- J[, j]
    if (any(col != 0) && max(abs(col - col[1])) <= 1e-8 * max(abs(col))) {
      cons <- names(b)[j]
      break
    }
  }
  has_c <- !is.na(cons)
  tss <- if (has_c) sum((y - mean(y))^2) else sum(y^2)
  df_r <- N - k
  df_m <- if (has_c) k - 1 else k
  mss <- tss - s
  r2 <- mss / tss
  r2_a <- 1 - (1 - r2) * (if (has_c) N - 1 else N) / df_r
  rmse <- sqrt(s / df_r)
  dev <- N * (1 + log(2 * pi * s / N))
  V <- s / df_r * solve(crossprod(J))
  nm <- paste0("/", names(b))
  dimnames(V) <- list(nm, nm)
  if (show) {
    pad <- stata_tab_pad
    g <- function(x, w) pad(stata_fmt(x, sprintf("%%%d.0g", w - 1)), w)
    count <- stata_tab_count
    cat("\n      Source |      SS            df       MS\n")
    rule <- paste0(strrep("-", 13), "+", strrep("-", 34))
    left <- function(name, ss, df) paste0(pad(name, 12), " |", g(ss, 11), pad(format(df, scientific = FALSE), 11), g(ss / df, 12))
    cat(rule, "    Number of obs = ", pad(count(N), 10), "\n", sep = "")
    cat(left("Model", mss, df_m), "    R-squared     = ", pad(sprintf("%.4f", r2), 10), "\n", sep = "")
    cat(left("Residual", s, df_r), "    Adj R-squared = ", pad(sprintf("%.4f", r2_a), 10), "\n", sep = "")
    cat(rule, "    Root MSE      = ", pad(stata_fmt(rmse, "%9.0g"), 10), "\n", sep = "")
    cat(left("Total", tss, if (has_c) N - 1 else N), "    Res. dev.     = ", pad(stata_fmt(dev, "%9.0g"), 10), "\n", sep = "")
    cat("\n", strrep("-", 78), "\n", sep = "")
    qt <- stats::qt((1 + level) / 2, df_r)
    cat(pad(stata_tab_abbrev(depvar), 12), " | Coefficient  Std. err.      t    P>|t|     [", format(100 * level),
        "% conf. interval]\n", sep = "")
    cat(strrep("-", 13), "+", strrep("-", 64), "\n", sep = "")
    se <- sqrt(diag(V))
    for (j in seq_len(k)) {
      tv <- b[j] / se[j]
      ts <- sprintf("%.2f", tv)
      if (nchar(ts) > 8) ts <- sprintf("%.1e", tv)
      p <- 2 * stats::pt(-abs(tv), df_r)
      cat(pad(nm[j], 12), " |", pad(stata_tab_g(b[j]), 11), pad(stata_tab_g(se[j]), 11), pad(ts, 9),
          pad(sprintf("%.3f", p), 8), pad(stata_tab_g(b[j] - qt * se[j]), 13), pad(stata_tab_g(b[j] + qt * se[j]), 12),
          "\n", sep = "")
    }
    cat(strrep("-", 78), "\n", sep = "")
    if (has_c) cat("Note: Parameter ", cons, " is used as a constant term during estimation.\n", sep = "")
  }
  list(b = matrix(b, 1, k, dimnames = list("y1", nm)), V = V, N = N, df_r = df_r, mss = mss, rss = s, tss = tss,
       r2 = r2, r2_a = r2_a, rmse = rmse, dev = dev, k = k, ic = it, converged = as.numeric(converged),
       cmd = "nl", depvar = depvar, title = "", properties = "b V")
}"#;

impl Session {
    /// `nl (depvar = <sexp>) [if] [in] [, variables() level() nolog iterate()]`.
    pub(super) fn nl(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "nl")?;
        if let Some(o) = opts.other(&[("variables", 3), ("level", 3), ("nolog", 5), ("iterate", 4)])
        {
            return Err(no(format!("nl with the option {o} is not translated yet")));
        }
        if c.parts.weight.is_some() {
            return Err(no("nl with weights is not translated yet"));
        }
        let main = c.parts.main.trim();
        let inner = main
            .strip_prefix('(')
            .and_then(|m| m.strip_suffix(')'))
            .ok_or_else(|| {
                no("nl is translated for its interactive form, nl (depvar = <expression>)")
            })?;
        let (dep, sexp) = inner
            .split_once('=')
            .ok_or_else(|| no("nl needs (depvar = <expression>)"))?;
        let dep = dep.trim();
        if !is_name(dep) {
            return Err(no(format!(
                "nl with {dep} on the left is not translated yet (one variable name is)"
            )));
        }
        // the parameters: {name = #} or {name}, the first one giving the start
        let mut params: Vec<(String, String)> = Vec::new();
        let mut text = String::new();
        let mut rest = sexp;
        while let Some(open) = rest.find('{') {
            text.push_str(&rest[..open]);
            let close = rest[open..]
                .find('}')
                .map(|i| open + i)
                .ok_or_else(|| no("nl: a { without its }"))?;
            let body = rest[open + 1..close].trim();
            if body.contains(':') {
                return Err(no(
                    "nl with a linear combination ({xb: x1 x2}) is not translated yet",
                ));
            }
            let (name, init) = match body.split_once('=') {
                Some((n, v)) => (n.trim(), Some(v.trim())),
                None => (body, None),
            };
            if !is_name(name) {
                return Err(no(format!("nl: {{{body}}} is not a parameter")));
            }
            let start = match init {
                Some(v) => {
                    let x: f64 = v
                        .parse()
                        .map_err(|_| no(format!("nl: the starting value {v} is not a number")))?;
                    Some(r_number(x))
                }
                None => None,
            };
            match params.iter_mut().find(|(n, _)| n == name) {
                Some(p) => {
                    if let Some(s) = start {
                        p.1 = s;
                    }
                }
                None => params.push((name.to_string(), start.unwrap_or_else(|| "0".into()))),
            }
            text.push_str(name);
            rest = &rest[close + 1..];
        }
        text.push_str(rest);
        if params.is_empty() {
            return Err(no("nl needs at least one {parameter}"));
        }
        // e(b) and the table list them by name, in byte order (Mid before alpha), as Stata 19 does
        params.sort_by(|a, b| a.0.cmp(&b.0));
        // the parameters are numbers the expression reads, as the data's are
        let saved: Vec<(String, Option<Kind>)> = params
            .iter()
            .map(|(n, _)| (n.clone(), self.kinds.get(n).cloned()))
            .collect();
        for (n, _) in &params {
            self.kinds.insert(n.clone(), Kind::Number);
        }
        let expr = self.expr(&text, Use::Value, out);
        for (n, k) in saved {
            match k {
                Some(k) => {
                    self.kinds.insert(n, k);
                }
                None => {
                    self.kinds.remove(&n);
                }
            }
        }
        let expr = expr?;
        let level = level(opts)?;
        let vars: Vec<String> = match opts.arg("variables", 3) {
            Some(v) => names(&v)?,
            None => Vec::new(),
        };
        let iterate = match opts.arg("iterate", 4) {
            Some(i) => i
                .trim()
                .parse::<u32>()
                .map_err(|_| no(format!("nl, iterate({i}) needs a whole number")))?,
            None => 16000,
        };
        let d = self.data()?;
        let frame = match self.condition(c, out)? {
            Some(cond) => {
                out.package("dplyr");
                format!("dplyr::filter({d}, {cond})")
            }
            None => d.clone(),
        };
        for h in [
            "stata_fmt",
            "stata_tables",
            "stata_eclass",
            "stata_returned",
            "stata_nl",
        ] {
            self.helpers.insert(h);
        }
        out.line(format!(
            "stata_e <- stata_nl({frame}, {}, c({}), function(.v) with(.v, {expr}), vars = c({}), level = {}, show = {}, log = {}, iterate = {iterate})",
            r_text(dep),
            params
                .iter()
                .map(|(n, v)| format!("{} = {v}", r_name(n)))
                .collect::<Vec<_>>()
                .join(", "),
            vars.iter().map(|v| r_text(v)).collect::<Vec<_>>().join(", "),
            r_number(level),
            super::stats::r_logical(!self.quiet(c)),
            super::stats::r_logical(!opts.has("nolog", 5)),
        ));
        self.estimates = None;
        self.estimates_lost =
            Some("predict, test, lincom and margins are not translated after nl yet".into());
        self.active_store = None;
        self.e_text.clear();
        self.esample = false;
        self.e_ret = super::returned::Returned::Program {
            cmd: "nl".into(),
            texts: ["cmd", "depvar", "title", "properties"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
        };
        out.caveat("nl as Stata 19 runs it (its starting values, steps and stopping rule), matched to its output");
        Ok(())
    }
}
