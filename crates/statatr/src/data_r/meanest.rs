//! `mean`, `total`, `proportion` and `ratio` as Stata 19 estimates and
//! prints them (`plan/stata/mean.md`): e(b) and e(V) under Stata's names
//! (`y`, `c.y@0bn.b`, `1bn.g`, `_ratio_1`), the other e() and r(table).
//! The variances, checked against Stata 19 to the last digits: means over
//! their own rows (s²/n, frequency weights counting rows, analytic ones
//! scaled to the rows), each group of `over()` on its own; with pweights
//! the linearized (sandwich) variance, n/(n−1) Σ zᵢzᵢ′; totals are the sum
//! of the weights times the mean; proportions (diag(p) − pp′)/n with logit
//! intervals; ratios linearized.

use super::model::{Estimates, Fit};
use super::weights::W;
use super::*;

pub(super) const MEANEST_HELPER: &str = r##"stata_meanest <- function(kind, d, vars, w = NULL, wtype = "f", over = NULL, level = 95,
                          ratios = NULL, rnames = NULL, show = TRUE, cmdline = "", wexp = "") {
  num <- function(u) { u <- unclass(u); attributes(u) <- NULL; as.numeric(u) }
  # importance weights count as frequency weights (Stata 19: the same N and errors)
  wlab <- wtype
  if (identical(wtype, "i")) wtype <- "f"
  used <- unique(c(vars, over))
  for (v in used) {
    if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
    if (is.character(d[[v]])) stop(v, " is a string variable (Stata: type mismatch, r(109))", call. = FALSE)
  }
  keep <- stats::complete.cases(as.data.frame(lapply(d[used], num)))
  wv <- if (is.null(w)) rep(1, nrow(d)) else num(w)
  keep <- keep & !is.na(wv)
  if (!any(keep)) stop("no observations (Stata: r(2000))", call. = FALSE)
  n <- sum(keep)
  wv <- wv[keep]
  if (!is.null(w) && wtype == "a") wv <- wv * n / sum(wv)
  # frequency weights count rows
  nobs <- if (!is.null(w) && wtype == "f") sum(wv) else n
  lin <- !is.null(w) && wtype == "p"
  col <- function(v) num(d[[v]])[keep]
  labs_of <- function(v, lv) {
    vl <- attr(d[[v]], "labels", exact = TRUE)
    vapply(lv, function(u) {
      k <- if (!is.null(vl)) names(vl)[match(u, num(vl))] else NA
      if (length(k) && !is.na(k)) k else stata_fmt(u, "%9.0g", trim = TRUE)
    }, "")
  }
  lev_name <- function(u, first) paste0(stata_fmt(u, "%9.0g", trim = TRUE), if (first) "bn" else "", ".")
  # over(): each cell a combination of the variables' values, in their order (Stata 19: 1 0, 1 1, 2 0 ...)
  if (is.null(over)) {
    ov <- rep(1, n)
    ol <- 1
  } else {
    if (kind == "proportion") for (v in over) if (v %in% vars) stop("over variable ", v, " not allowed in varlist (Stata: r(198))", call. = FALSE)
    om <- matrix(vapply(over, col, numeric(n)), n)
    ov <- do.call(paste, c(as.data.frame(om), sep = "\r"))
    ord <- do.call(order, as.data.frame(om))
    first <- ord[!duplicated(ov[ord])]
    ocells <- om[first, , drop = FALSE]
    ol <- ov[first]
  }
  # a cell's name: each variable's level, its smallest written bn (c.y@1bn.g#0bn.b)
  oname <- function(j) paste(vapply(seq_along(over), function(i) {
    u <- ocells[j, i]
    paste0(lev_name(u, u == min(ocells[, i])), over[i])
  }, ""), collapse = "#")
  ohead <- paste(over, collapse = "#")
  # each estimate: its rows (a group of over()), the variable(s) it reads, its name and label
  est <- list()
  add <- function(e) est[[length(est) + 1]] <<- e
  if (kind %in% c("mean", "total")) {
    for (v in vars) for (j in seq_along(ol)) {
      add(list(sel = ov == ol[j], y = col(v), x = NULL, var = v, j = j,
               name = if (is.null(over)) v else paste0("c.", v, "@", oname(j))))
    }
  } else if (kind == "ratio") {
    for (r in seq_along(ratios)) for (j in seq_along(ol)) {
      add(list(sel = ov == ol[j], y = col(ratios[[r]][1]), x = col(ratios[[r]][2]), var = rnames[r], j = j,
               name = if (is.null(over)) rnames[r] else paste0("c.", rnames[r], "@", oname(j))))
    }
  } else {
    for (v in vars) {
      yv <- col(v)
      if (any(yv < 0 | yv != floor(yv))) stop(v, ": proportion takes values that are nonnegative integers (Stata: r(452))", call. = FALSE)
      lv <- sort(unique(yv))
      for (k in seq_along(lv)) for (j in seq_along(ol)) {
        add(list(sel = ov == ol[j], y = as.numeric(yv == lv[k]), x = NULL, var = v, j = j, k = k, val = lv[k],
                 name = paste0(lev_name(lv[k], k == 1), v, if (!is.null(over)) paste0("@", oname(j)))))
      }
    }
  }
  K <- length(est)
  b <- numeric(K)
  Wg <- numeric(K)
  Ng <- numeric(K)
  for (i in seq_len(K)) {
    e <- est[[i]]
    s <- e$sel
    Wg[i] <- sum(wv[s])
    # rows, whatever the weights (Stata 19)
    Ng[i] <- sum(s)
    b[i] <- if (kind == "ratio") sum(wv[s] * e$y[s]) / sum(wv[s] * e$x[s]) else sum(wv[s] * e$y[s]) / Wg[i]
  }
  V <- matrix(0, K, K)
  if (lin || kind == "ratio") {
    # linearized: z_i of each estimate, V = n/(n-1) sum z z' (frequency weights count rows)
    Z <- vapply(seq_len(K), function(i) {
      e <- est[[i]]
      s <- e$sel
      if (kind == "ratio") wv * s * (e$y - b[i] * e$x) / sum(wv[s] * e$x[s]) else wv * s * (e$y - b[i]) / Wg[i]
    }, numeric(n))
    Z <- matrix(Z, n)
    # each group of over() on its own: its rows' n/(n-1), nothing across groups (Stata 19)
    jj <- vapply(est, `[[`, 0, "j")
    for (j in seq_along(ol)) {
      s <- ov == ol[j]
      cols <- which(jj == j)
      Zj <- Z[s, cols, drop = FALSE]
      # ratio takes each group's rows, mean, total and proportion with pweights the whole
      # sample's (Stata 19: n/(n-1) of all n)
      own <- kind == "ratio"
      if (!is.null(w) && wtype == "f") {
        # each row its weight's copies: z per copy
        m <- if (own) sum(wv[s]) else nobs
        V[cols, cols] <- m / (m - 1) * crossprod(Zj / sqrt(wv[s]))
      } else {
        m <- if (own) sum(s) else n
        V[cols, cols] <- m / (m - 1) * crossprod(Zj)
      }
    }
  } else {
    for (i in seq_len(K)) for (h in seq_len(K)) {
      ei <- est[[i]]
      eh <- est[[h]]
      if (ei$j != eh$j) next
      s <- ei$sel
      if (kind == "proportion") {
        if (ei$var != eh$var) {
          # two variables' proportions: the means of their indicators
          V[i, h] <- (sum(wv[s] * ei$y[s] * eh$y[s]) / Wg[i] - b[i] * b[h]) / Wg[i]
        } else {
          V[i, h] <- ((i == h) * b[i] - b[i] * b[h]) / Wg[i]
        }
      } else {
        # analytic weights scaled to each group's rows
        ww <- if (!is.null(w) && wtype == "a") wv[s] * sum(s) / sum(wv[s]) else wv[s]
        V[i, h] <- sum(ww * (ei$y[s] - b[i]) * (eh$y[s] - b[h])) / (sum(ww) - 1) / sum(ww)
      }
    }
  }
  if (kind == "total") {
    V <- V * outer(Wg, Wg)
    b <- b * Wg
  }
  nm <- vapply(est, `[[`, "", "name")
  dimnames(V) <- list(nm, nm)
  se <- sqrt(pmax(diag(V), 0))
  dfr <- nobs - 1
  tq <- stats::qt(1 - (100 - level) / 200, dfr)
  if (kind == "proportion") {
    lg <- log(b / (1 - b))
    sl <- se / (b * (1 - b))
    lb <- 1 / (1 + exp(-(lg - tq * sl)))
    ub <- 1 / (1 + exp(-(lg + tq * sl)))
  } else {
    lb <- b - tq * se
    ub <- b + tq * se
  }
  tt <- b / se
  pv <- 2 * stats::pt(-abs(tt), dfr)
  tab <- rbind(b = b, se = se, t = tt, pvalue = pv, ll = lb, ul = ub, df = dfr, crit = tq, eform = 0)
  colnames(tab) <- nm
  title <- c(mean = "Mean", total = "Total", proportion = "Proportion", ratio = "Ratio")[[kind]]
  # Stata 19: ratio is linearized; mean, total and proportion keep analytic with pweights too
  vce <- if (kind == "ratio") "linearized" else "analytic"
  # e() in Stata 19's order: ratio with frequency weights puts N before N_over
  e <- if (kind == "ratio" && !is.null(w) && wtype == "f") list(df_r = dfr, N = nobs, N_over = length(ol))
       else list(df_r = dfr, N_over = length(ol), N = nobs)
  e <- c(e, list(k_eq = 1, rank = qr(V)$rank, cmdline = cmdline, cmd = kind,
                 vce = if (kind == "ratio") "linearized" else if (lin) "robust" else "analytic"))
  if (kind == "ratio") e$vcetype <- "Linearized"
  e$title <- paste(title, "estimation")
  if (!is.null(w)) {
    e$wexp <- paste("=", wexp)
    e$wtype <- c(f = "fweight", a = "aweight", p = "pweight", i = "iweight")[[wlab]]
  }
  if (kind == "ratio") e$namelist <- paste(rnames, collapse = " ")
  e$estat_cmd <- "estat_vce_only"
  e$varlist <- if (kind == "proportion") paste(unique(sub("@.*$", "", sub("bn\\.", ".", nm))), collapse = " ") else paste(vars, collapse = " ")
  e$marginsnotok <- "_ALL"
  if (!is.null(over)) e$over <- paste(over, collapse = " ")
  e$properties <- "b V"
  e$b <- matrix(b, 1, dimnames = list("y1", nm))
  e$V <- V
  if (kind == "mean") {
    # the standard deviations, the weights scaled to the rows (frequency weights counting copies)
    sdv <- vapply(seq_len(K), function(i) {
      s <- est[[i]]$sel
      ww <- if (!is.null(w) && wtype != "f") wv[s] * sum(s) / sum(wv[s]) else wv[s]
      sqrt(sum(ww * (est[[i]]$y[s] - b[i])^2) / (sum(ww) - 1))
    }, 0)
    e$sd <- matrix(sdv, 1, dimnames = list("y1", nm))
    # for estat sd: each row's head and label, as the table shows them
    if (!is.null(over)) {
      attr(e$sd, "stata_heads") <- vapply(est, function(x) paste0("c.", x$var, "@", ohead), "")
      attr(e$sd, "stata_labs") <- vapply(est, function(x) paste(vapply(seq_along(over), function(i) labs_of(over[i], ocells[x$j, i]), ""), collapse = " "), "")
    }
  }
  if (kind == "proportion") e$freq <- matrix(vapply(seq_len(K), function(i) sum(wv[est[[i]]$sel] * est[[i]]$y[est[[i]]$sel]) *
                                                     (if (!is.null(w) && wtype == "a") sum(w[keep]) / n else 1), 0),
                                             1, dimnames = list("r1", nm))
  # one estimate: its row named as its column (Stata 19)
  e$`_N` <- matrix(Ng, 1, dimnames = list(if (K == 1) nm else "r1", nm))
  e$error <- matrix(0, 1, K, dimnames = list("r1", nm))
  out <- list(e = e, b = stats::setNames(b, nm), V = V, df = dfr, r = list(table = tab))
  if (!show) return(out)
  g <- function(v) if (is.na(v)) "." else stata_fmt(v, "%9.0g", trim = TRUE)
  rhs <- paste0("Number of obs = ", stata_fmt(nobs, "%20.0fc", trim = TRUE))
  cat("\n", formatC(e$title, width = -(62 - nchar(rhs))), rhs, "\n", sep = "")
  if (kind == "ratio") {
    cat("\n")
    for (r in seq_along(ratios)) cat(sprintf("%13s: %s/%s", rnames[r], ratios[[r]][1], ratios[[r]][2]), "\n", sep = "")
  }
  rule <- paste0(strrep("-", 62), "\n")
  cat("\n", rule, sep = "")
  if (kind == "proportion") cat("             |                                   Logit\n")
  if (vce == "linearized") cat("             |             Linearized\n")
  cat(sprintf("             |%11s%s     [%s%% conf. interval]", title,
              if (vce == "linearized") "   std. err." else "   Std. err.", stata_fmt(level, "%9.0g", trim = TRUE)), "\n", sep = "")
  cat("-------------+", strrep("-", 48), "\n", sep = "")
  row <- function(lab, i) cat(sprintf("%12s |%11s%11s%14s%12s", lab, g(b[i]), g(se[i]), g(lb[i]), g(ub[i])), "\n", sep = "")
  olab <- if (!is.null(over)) vapply(seq_along(ol), function(j) paste(vapply(seq_along(over), function(i) labs_of(over[i], ocells[j, i]), ""), collapse = " "), "")
  last <- ""
  for (i in seq_len(K)) {
    e <- est[[i]]
    if (is.null(over) && kind != "proportion") {
      row(stata_tab_abbrev(e$var, 12), i)
      next
    }
    head <- if (kind == "proportion") {
      if (is.null(over)) e$var else paste0(e$var, "@", ohead)
    } else paste0("c.", e$var, "@", ohead)
    key <- if (kind == "proportion" && !is.null(over)) e$var else head
    if (key != last) {
      if (last != "") cat("             |\n")
      cat(sprintf("%12s |", stata_tab_abbrev(head, 12)), "\n", sep = "")
      last <- key
    }
    lab <- if (kind == "proportion") {
      vl <- labs_of(e$var, e$val)
      if (is.null(over)) vl else paste(vl, olab[e$j])
    } else olab[e$j]
    cat(sprintf("%11s  |%11s%11s%14s%12s", lab, g(b[i]), g(se[i]), g(lb[i]), g(ub[i])), "\n", sep = "")
  }
  cat(rule)
  out
}"##;

/// ratio's `y/x` pairs and their names.
type Ratios = (Vec<(String, String)>, Vec<String>);

impl Session {
    /// `mean varlist`, `total varlist`, `proportion varlist` and `ratio
    /// [name:] y/x ...`, `[weight] [if] [in] [, over(var) level(#)]`.
    pub(super) fn meanest(
        &mut self,
        c: &Command,
        kind: &str,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, kind)?;
        if let Some(o) = opts.other(&[("over", 4), ("level", 1)]) {
            return Err(no(format!(
                "{kind} with the option {o} is not translated yet"
            )));
        }
        let w = self.weight_of(c, &[W::F, W::A, W::P, W::I])?;
        match (&w, kind) {
            // Stata 19: total, proportion and ratio take no analytic weights (an error as the
            // line runs, which capture takes)
            (Some((W::A, _)), "total" | "proportion" | "ratio") => {
                out.line("stop(\"aweights not allowed (Stata: r(101))\", call. = FALSE)");
                return Ok(());
            }
            // importance weights: as frequency weights (Stata 19), not checked after ratio
            (Some((W::I, _)), "ratio") => {
                return Err(no("ratio with iweights is not translated yet"));
            }
            _ => {}
        }
        let over: Option<Vec<String>> = match opts.arg("over", 4) {
            None => None,
            Some(o) => {
                let vs: Vec<String> = o.split_whitespace().map(str::to_string).collect();
                if vs.is_empty() || vs.iter().any(|v| !is_name(v)) {
                    return Err(no(format!(
                        "over({o}): variables written out are translated"
                    )));
                }
                Some(vs)
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
        let text = c.parts.main.trim();
        let (vars, ratios): (Vec<String>, Option<Ratios>) = if kind == "ratio" {
            let mut pairs = Vec::new();
            let mut names = Vec::new();
            let items: Vec<String> = if text.starts_with('(') {
                text.split(')')
                    .map(|p| p.trim().trim_start_matches('(').trim().to_string())
                    .filter(|p| !p.is_empty())
                    .collect()
            } else {
                vec![text.to_string()]
            };
            for (i, it) in items.iter().enumerate() {
                let (name, rest) = match it.split_once(':') {
                    Some((n, r)) if is_name(n.trim()) => (n.trim().to_string(), r.trim()),
                    _ => (format!("_ratio_{}", i + 1), it.as_str()),
                };
                let Some((y, x)) = rest.split_once('/') else {
                    return Err(no(format!("ratio {it}: y/x is translated")));
                };
                let (y, x) = (y.trim(), x.trim());
                if !is_name(y) || !is_name(x) {
                    return Err(no(format!(
                        "ratio {it}: a variable over a variable is translated"
                    )));
                }
                pairs.push((y.to_string(), x.to_string()));
                names.push(name);
            }
            let mut vars: Vec<String> = Vec::new();
            for (y, x) in &pairs {
                for v in [y, x] {
                    if !vars.contains(v) {
                        vars.push(v.clone());
                    }
                }
            }
            (vars, Some((pairs, names)))
        } else {
            let vars: Vec<String> = text.split_whitespace().map(str::to_string).collect();
            if vars.is_empty() {
                return Err(no(format!("{kind} needs a variable")));
            }
            if let Some(v) = vars.iter().find(|v| !is_name(v)) {
                return Err(no(format!(
                    "{kind} {v}: variable names written out are translated"
                )));
            }
            (vars, None)
        };
        let mut rows = self.rows_for(c, out)?;
        if w.is_some() {
            rows = self.weighted_rows(rows, &w);
            out.line(format!("stata_rows <- {rows}"));
            rows = "stata_rows".into();
            out.caveat(super::weights::weight_caveat(&w));
        }
        let wargs = super::weights::w_args(&rows, &w);
        for h in [
            "stata_meanest",
            "stata_fmt",
            "stata_returned",
            "stata_tables",
            "stata_names",
        ] {
            self.helpers.insert(h);
        }
        let q = |v: &[String]| v.iter().map(|s| r_text(s)).collect::<Vec<_>>().join(", ");
        let ratio_args = match &ratios {
            None => String::new(),
            Some((pairs, names)) => format!(
                ", ratios = list({}), rnames = c({})",
                pairs
                    .iter()
                    .map(|(y, x)| format!("c({}, {})", r_text(y), r_text(x)))
                    .collect::<Vec<_>>()
                    .join(", "),
                q(names)
            ),
        };
        out.line(format!(
            "stata_me <- stata_meanest({}, {rows}, c({}){wargs}{}, level = {level}{ratio_args}{}, cmdline = {}{})",
            r_text(kind),
            q(&vars),
            over.as_ref()
                .map(|o| format!(", over = c({})", q(o)))
                .unwrap_or_default(),
            if self.quiet(c) { ", show = FALSE" } else { "" },
            r_text(format!("{} {}", c.written, super::returned::after_name(c).trim()).trim()),
            match &w {
                Some((_, v)) => format!(", wexp = {}", r_text(v)),
                None => String::new(),
            }
        ));
        // e() is the estimates', under Stata's names; test and lincom read them
        out.line("stata_b <- stata_me$b");
        out.line("stata_vcov <- stata_me$V");
        out.line("stata_df <- stata_me$df");
        out.line("stata_estimates <- list()");
        out.line("stata_e <- stata_me$e");
        out.line("stata_r <- stata_me$r");
        out.line("rm(stata_me)");
        self.estimates = Some(Estimates {
            fit: Fit::Margins,
            dep: String::new(),
            factors: Default::default(),
        });
        self.e_new = true;
        for t in [
            "cmd",
            "vce",
            "vcetype",
            "title",
            "namelist",
            "estat_cmd",
            "varlist",
            "marginsnotok",
            "over",
            "wtype",
            "wexp",
            "cmdline",
            "properties",
        ] {
            self.e_text.insert(t.to_string());
        }
        self.e_ret = super::returned::Returned::Kept {
            cmd: kind.into(),
            // what test and lincom can read: plain names, not factor levels
            model: if over.is_some() || kind == "proportion" {
                "levels".into()
            } else {
                String::new()
            },
            names: &[],
        };
        self.r_new = Some((&[], kind.into()));
        Ok(())
    }

    /// The command whose results `Fit::Margins` holds when it is one of
    /// mean, total, proportion or ratio, and whether its names are factor
    /// levels (over(), proportion).
    pub(super) fn posted_estimates(&self) -> Option<(&str, bool)> {
        if !self
            .estimates
            .as_ref()
            .is_some_and(|e| e.fit == Fit::Margins)
        {
            return None;
        }
        match &self.e_ret {
            super::returned::Returned::Kept { cmd, model, .. }
                if matches!(cmd.as_str(), "mean" | "total" | "proportion" | "ratio") =>
            {
                Some((cmd.as_str(), !model.is_empty()))
            }
            _ => None,
        }
    }
}
