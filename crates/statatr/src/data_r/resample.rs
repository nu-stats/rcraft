//! Batch I of the StatCraft port, resampling and repeated runs as Stata 19
//! does them: `jackknife`, `statsby` and `rolling` (here), `permute` and
//! `simulate` (the runner expands those, `dofile::resample`). The rules are in
//! `plan/stata/resampling.md`. The sums are made with the same loops as
//! `bootstrap` (`random.rs`), so the order of the additions is the same.

use super::random::top_level;
use super::*;

/// What stands before the colon of a prefix command, and after it.
pub(super) struct Prefixed {
    /// The expression list (the head up to the comma).
    pub(super) explist: String,
    /// The options after the comma, read as a command's options.
    pub(super) options: Vec<crate::lang::syntax::Opt>,
    /// The command after the colon.
    pub(super) inner: String,
}

/// Splits `exps, options: command` of a prefix command.
pub(super) fn split_prefixed(c: &Command, name: &str) -> Result<Prefixed, Untranslated> {
    let text = super::returned::after_name(c);
    let colon = top_level(&text, ':').ok_or_else(|| {
        no(format!(
            "{name} needs a command after a colon (Stata: r(198))"
        ))
    })?;
    let head = text.get(..colon).unwrap_or("").trim().to_string();
    let inner = text.get(colon + 1..).unwrap_or("").trim().to_string();
    let (explist, opt_text) = match top_level(&head, ',') {
        Some(i) => (
            head.get(..i).unwrap_or("").trim().to_string(),
            head.get(i + 1..).unwrap_or("").to_string(),
        ),
        None => (head.clone(), String::new()),
    };
    let program = crate::reader::read(&format!("{name} , {opt_text}"));
    let options = program
        .commands()
        .first()
        .map(|c| c.options.clone())
        .unwrap_or_default();
    Ok(Prefixed {
        explist,
        options,
        inner,
    })
}

/// The items of an expression list (names, `name=exp`, `(exp)`), split at
/// spaces outside parentheses.
pub(super) fn exp_items(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut depth = 0i32;
    for ch in text.chars() {
        match ch {
            '(' | '[' => depth += 1,
            ')' | ']' => depth -= 1,
            _ => {}
        }
        if ch.is_whitespace() && depth == 0 {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
        } else {
            cur.push(ch);
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    // `name = exp` with spaces around the =
    let mut joined: Vec<String> = Vec::new();
    let mut words = out.into_iter().peekable();
    while let Some(w) = words.next() {
        let after_eq = joined.last().is_some_and(|l| l.ends_with('='));
        if w == "=" || after_eq {
            if let Some(last) = joined.last_mut() {
                last.push_str(&w);
                continue;
            }
        }
        if words
            .peek()
            .is_some_and(|n| n.starts_with('=') && !n.starts_with("=="))
        {
            let next = words.next().unwrap_or_default();
            joined.push(format!("{w}{next}"));
            continue;
        }
        joined.push(w);
    }
    joined
}

/// One statistic of an expression list: its result name and its expression.
pub(super) struct Stat {
    pub(super) name: String,
    pub(super) exp: String,
    pub(super) code: String,
}

/// R's `jackknife`: the replicates, kept as floats, and Stata's table.
pub(super) const JACKKNIFE_HELPER: &str = r##"stata_jk_begin <- function(cmdname, reps, nodots) {
  if (!nodots) cat("(running ", cmdname, " on estimation sample)\n\nJackknife replications (", reps, "): ", sep = "")
  invisible(NULL)
}
stata_jk_report <- function(obs, reps, exps, command, cmdname, nfun, n_used, level = 0.95, model = NULL,
                            cluster = NULL, n_units = NULL, depvar = "", cmdline = "", notable = FALSE) {
  names <- names(obs)
  k <- length(obs)
  if (anyNA(reps)) stop("jackknife with a replication that gave a missing value is not translated yet", call. = FALSE)
  R <- nrow(reps)
  mean <- vapply(seq_len(k), function(j) stata_bs_sum(reps[, j]) / R, 0)
  V <- matrix(0, k, k, dimnames = list(names, names))
  for (a in seq_len(k)) for (c in seq_len(k)) {
    v <- (R - 1) / R * stata_bs_sum((reps[, a] - mean[a]) * (reps[, c] - mean[c]))
    V[a, c] <- if (abs(v) < 1e-300) 0 else v
  }
  se <- sqrt(pmax(diag(V), 0))
  df <- R - 1
  alpha <- (1 - level) / 2
  crit <- stats::qt(1 - alpha, df)
  pad <- function(s, w) formatC(as.character(s), width = w)
  short <- function(s) ifelse(nchar(s) > 12, paste0(substr(s, 1, 10), "~", substr(s, nchar(s), nchar(s))), s)
  g9 <- function(x) stata_fmt(x, "%9.0g")
  cmd <- if (is.null(model)) NULL else model[["cmd"]]
  title <- if (!is.null(model) && !is.null(model[["title"]])) model[["title"]] else "Jackknife results"
  slopes <- names != "_cons"
  q <- sum(slopes)
  wald <- if (!is.null(model) && q > 0) {
    s <- which(slopes)
    drop(t(obs[s]) %*% solve(V[s, s, drop = FALSE], obs[s]))
  } else NA_real_
  Fval <- wald / q
  pF <- if (is.na(Fval)) NA_real_ else stats::pf(Fval, q, df, lower.tail = FALSE)
  if (!notable || TRUE) {
    cat("\n")
    right <- function(l, v) paste0(formatC(l, width = -14), "= ", pad(v, 6))
    if (!is.null(cmd) && cmd %in% c("regress", "logit", "probit")) {
      lines <- list(c(title, right("Number of obs", format(n_used, big.mark = ","))),
                    c("", right("Replications", R)),
                    c("", right(sprintf("F(%d, %d)", q, df), sprintf("%.2f", Fval))),
                    c("", right("Prob > F", sprintf("%.4f", pF))))
      if (cmd == "regress") {
        lines <- c(lines, list(c("", right("R-squared", sprintf("%.4f", model$r2))),
                               c("", right("Adj R-squared", sprintf("%.4f", model$r2_a))),
                               c("", right("Root MSE", sprintf("%.4f", model$rmse)))))
      } else {
        lines <- c(lines, list(c(paste0("Log likelihood = ", trimws(stata_fmt(model$ll, "%10.0g"))),
                                 right("Pseudo R2", sprintf("%.4f", model$r2_p)))))
      }
      for (l in lines) cat(formatC(l[1], width = -56), l[2], "\n", sep = "")
    } else {
      w <- max(nchar(n_used), nchar(R))
      cat(formatC(title, width = -60), "Number of obs = ", pad(n_used, w), "\n", sep = "")
      cat(strrep(" ", 60), "Replications  = ", pad(R, w), "\n\n", sep = "")
      cat(pad("Command", 13), ": ", command, "\n", sep = "")
      for (j in seq_len(k)) cat(pad(short(names[j]), 13), ": ", exps[j], "\n", sep = "")
      cat(pad("n()", 13), ": ", nfun, "\n", sep = "")
    }
    cat("\n")
    if (!notable) {
      if (!is.null(cluster)) cat(pad(sprintf("(Replications based on %d clusters in %s)", n_units, cluster), 78), "\n", sep = "")
      cat(strrep("-", 78), "\n", sep = "")
      cat("             |              Jackknife\n")
      cat(pad(short(depvar), 12), " | Coefficient  std. err.      t    P>|t|     [",
          stata_fmt(100 * level, "%9.0g", trim = TRUE), "% conf. interval]\n", sep = "")
      cat("-------------+----------------------------------------------------------------\n")
      for (j in seq_len(k)) {
        b <- obs[j]
        if (V[j, j] <= 0) {
          cat(pad(short(names[j]), 12), " |", pad(g9(b), 11), pad(".", 11), pad(".", 9), pad(".", 8),
              pad(".", 13), pad(".", 12), "\n", sep = "")
          next
        }
        t <- b / se[j]
        p <- 2 * stats::pt(-abs(t), df)
        cat(pad(short(names[j]), 12), " |", pad(g9(b), 11), pad(g9(se[j]), 11), pad(sprintf("%.2f", t), 9),
            pad(sprintf("%.3f", p), 8), pad(g9(b - crit * se[j]), 13), pad(g9(b + crit * se[j]), 12), "\n", sep = "")
      }
      cat(strrep("-", 78), "\n", sep = "")
    }
  }
  row <- function(v, r) matrix(v, 1, k, dimnames = list(r, names))
  e <- list()
  if (!is.null(cmd) && cmd == "regress") {
    for (n in c("sum_w", "r2", "rmse", "mss", "rss", "r2_a", "ll", "ll_0")) if (!is.null(model[[n]])) e[[n]] <- model[[n]]
  } else if (!is.null(cmd)) {
    for (n in c("ll", "ll_0", "r2_p")) if (!is.null(model[[n]])) e[[n]] <- model[[n]]
  }
  e$df_r <- df
  e$N_reps <- R
  e$N_misreps <- 0
  e$rank <- sum(diag(V) > 0)
  e$k_eq <- 1
  e$k_exp <- if (is.null(model)) k else 0
  e$k_eexp <- if (is.null(model)) 0 else k
  e$k_extra <- 0
  e$singleton <- NA_real_
  if (!is.null(cluster)) e$N_clust <- n_units
  e$N <- n_used
  if (!is.null(model)) {
    e$df_m <- q
    if (!is.null(cluster)) { e$p <- pF; e$F <- Fval } else { e$F <- Fval; e$p <- pF }
  }
  e$cmdline <- cmdline
  e$cmd <- if (is.null(cmd)) "jackknife" else cmd
  e$prefix <- "jackknife"
  e$cmdname <- cmdname
  e$nfunction <- nfun
  e$command <- command
  for (j in rev(seq_len(k))) e[[paste0("exp", j)]] <- exps[j]
  e$title <- title
  if (!is.null(cluster)) { e$clustvar <- cluster; e$cluster <- cluster }
  e$vcetype <- "Jackknife"
  e$vce <- "jackknife"
  if (!is.null(model)) for (n in c("estat_cmd", "model", "predict", "depvar", "marginsok")) if (!is.null(model[[n]])) e[[n]] <- model[[n]]
  e$properties <- "b V"
  e$b <- row(obs, "y1")
  e$V <- V
  if (!is.null(model) && !is.null(model$V)) e$V_modelbased <- model$V
  e$b_jk <- row(R * obs - (R - 1) * mean, "y1")
  attr(e, "table") <- rbind(b = obs, se = se, t = ifelse(se > 0, obs / se, NA_real_),
                            pvalue = ifelse(se > 0, 2 * stats::pt(-abs(obs / se), df), NA_real_),
                            ll = ifelse(se > 0, obs - crit * se, NA_real_), ul = ifelse(se > 0, obs + crit * se, NA_real_),
                            df = rep(df, k), crit = rep(crit, k), eform = rep(0, k))
  e
}"##;
/// Whether the options hold `vce(jackknife)` (and nothing after the word).
pub(super) fn vce_jackknife(opts: &Opts) -> bool {
    opts.arg("vce", 3).is_some_and(|v| {
        let v = v.trim();
        v.len() >= 4 && "jackknife".starts_with(v)
    })
}

impl Session {
    /// `jackknife exp_list [, options]: command`: the command run on the
    /// data and on each observation (or cluster) left out in turn.
    pub(super) fn jackknife(&mut self, c: &Command, out: &mut Out) -> Result<(), Untranslated> {
        self.no_by(c, "jackknife")?;
        let p = split_prefixed(c, "jackknife")?;
        let cmdline = format!("jackknife {}", super::returned::after_name(c).trim());
        self.jackknife_run(p, &cmdline, out)
    }

    /// `regress y x, vce(jackknife)`: the same as `jackknife _b: regress y x`,
    /// with the line as typed in e(cmdline).
    pub(super) fn jackknife_vce(&mut self, c: &Command, out: &mut Out) -> Result<(), Untranslated> {
        self.no_by(c, &c.name)?;
        let text = c.text.trim().to_string();
        let lower = text.to_ascii_lowercase();
        let start = lower
            .find("vce(")
            .ok_or_else(|| no("vce(jackknife) was not found in the line"))?;
        let close = lower
            .get(start..)
            .and_then(|s| s.find(')'))
            .map(|i| start + i + 1)
            .ok_or_else(|| no("vce( is not closed"))?;
        let mut inner = format!(
            "{}{}",
            text.get(..start).unwrap_or(""),
            text.get(close..).unwrap_or("")
        );
        // the comma left with no option after it
        let t = inner.trim_end().to_string();
        inner = t
            .strip_suffix(',')
            .map_or(t.clone(), |s| s.trim_end().to_string());
        let inner = inner.split_whitespace().collect::<Vec<_>>().join(" ");
        let p = Prefixed {
            explist: "_b".into(),
            options: Vec::new(),
            inner,
        };
        self.jackknife_run(p, &text, out)
    }

    fn jackknife_run(
        &mut self,
        p: Prefixed,
        cmdline: &str,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let opt_list = p.options.clone();
        let opts = Opts(&opt_list);
        if let Some(o) = opts.other(&[("nodots", 4), ("notable", 4), ("cluster", 2), ("level", 1)])
        {
            return Err(no(format!(
                "jackknife with the option {o} is not translated yet"
            )));
        }
        let level = super::model::level(&opts)?;
        let nodots = opts.has("nodots", 4);
        let notable = opts.has("notable", 4);
        let cluster = opts.arg("cluster", 2).map(|v| v.trim().to_string());
        if let Some(v) = &cluster {
            if !super::is_name(v) {
                return Err(no(format!("{v} is not a variable name")));
            }
        }
        let d = self.data()?;
        let program = crate::reader::read(&p.inner);
        let mut inner = match program.commands().as_slice() {
            [one] => (*one).clone(),
            _ => return Err(no("jackknife of more than one command is not translated")),
        };
        if !inner.prefixes.is_empty()
            || matches!(
                inner.name.as_str(),
                "bootstrap"
                    | "bsample"
                    | "jackknife"
                    | "permute"
                    | "simulate"
                    | "statsby"
                    | "rolling"
            )
        {
            return Err(no(format!(
                "jackknife of {} with a prefix, or of another resampling command, is not translated yet",
                inner.name
            )));
        }
        inner.prefixes.push(crate::reader::Prefix::Quietly);
        let mut body = Out::default();
        self.e_new = false;
        self.r_new = None;
        self.dispatch(&inner, &mut body)?;
        let eclass = self.e_new;
        let r_left = self.r_new.take();
        self.track_returned(&inner, true, false, r_left);
        if !eclass && (inner.parts.if_exp.is_some() || inner.parts.in_range.is_some()) {
            return Err(no(
                "jackknife of a command without e(sample) with if or in is not translated yet",
            ));
        }
        let (stats, by_b) = self.jackknife_stats(&p.explist, &inner, eclass, &mut body)?;
        let nfun = if eclass { "e(N)" } else { "r(N)" };
        let (ncode, _) = self.value(nfun, &mut body)?;
        for h in [
            "stata_rng",
            "stata_bootstrap",
            "stata_jackknife",
            "stata_fmt",
            "stata_float",
            "stata_names",
            "stata_returned",
        ] {
            self.helpers.insert(h);
        }
        for h in std::mem::take(&mut body.helpers) {
            self.helpers.insert(h);
        }
        for pk in &body.packages {
            out.package(pk);
        }
        let code = body.code.join("\n");
        let cmdname = inner.name.clone();
        let command = p.inner.trim().to_string();
        let depvar = if by_b {
            self.estimates
                .as_ref()
                .map(|e| e.dep.clone())
                .unwrap_or_default()
        } else {
            String::new()
        };
        let (obs, rep_stats, names, exps) = if by_b {
            (
                "stata_bs_b()".to_string(),
                "stata_float(stata_bs_b())".to_string(),
                String::new(),
                String::new(),
            )
        } else {
            let named = format!(
                "stats::setNames(c({}), c({}))",
                stats
                    .iter()
                    .map(|s| s.code.clone())
                    .collect::<Vec<_>>()
                    .join(", "),
                stats
                    .iter()
                    .map(|s| super::r_text(&s.name))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            (
                named.clone(),
                format!("stata_float({named})"),
                stats
                    .iter()
                    .map(|s| super::r_text(&s.name))
                    .collect::<Vec<_>>()
                    .join(", "),
                stats
                    .iter()
                    .map(|s| super::r_text(&s.exp))
                    .collect::<Vec<_>>()
                    .join(", "),
            )
        };
        let k = if by_b {
            "length(stata_jk_obs)".to_string()
        } else {
            stats.len().to_string()
        };
        let exps_arg = if by_b {
            "paste0(\"_b[\", names(stata_jk_obs), \"]\")".to_string()
        } else {
            format!("c({exps})")
        };
        let _ = names;
        out.line(format!("stata_jk_full <- {d}"));
        out.line(code.clone());
        out.line("stata_fit_failed <- FALSE");
        out.line(format!("stata_jk_obs <- {obs}"));
        out.line(format!("stata_jk_n <- {ncode}"));
        out.line(if eclass {
            "stata_jk_rows <- stata_esample".to_string()
        } else {
            "stata_jk_rows <- rep(TRUE, nrow(stata_jk_full))".to_string()
        });
        out.line(if by_b {
            "stata_jk_model <- stata_e"
        } else {
            "stata_jk_model <- NULL"
        });
        out.line("stata_jk_sub <- stata_jk_full[stata_jk_rows, , drop = FALSE]");
        match &cluster {
            Some(v) => out.line(format!(
                "stata_jk_unit <- match(stata_jk_sub[[{0}]], sort(unique(stata_jk_sub[[{0}]])))\nif (anyNA(stata_jk_sub[[{0}]])) stop(\"jackknife with a missing value of the cluster variable is not translated yet\", call. = FALSE)",
                super::r_text(v)
            )),
            None => out.line("stata_jk_unit <- seq_len(nrow(stata_jk_sub))"),
        }
        out.line("stata_jk_units <- max(stata_jk_unit)");
        out.line(format!(
            "stata_jk_begin({}, stata_jk_units, {})",
            super::r_text(&cmdname),
            super::stats::r_logical(nodots)
        ));
        out.line(format!(
            "stata_jk_m <- matrix(NA_real_, stata_jk_units, {k})"
        ));
        out.line(format!(
            "for (stata_jk_i in seq_len(stata_jk_units)) {{\n{d} <- stata_jk_sub[stata_jk_unit != stata_jk_i, , drop = FALSE]\nstata_jk_m[stata_jk_i, ] <- tryCatch({{\n{code}\n{rep_stats}\n}}, error = function(e) NA_real_)\nstata_bs_dot(stata_jk_i, stata_jk_units, {})\n}}",
            super::stats::r_logical(nodots)
        ));
        out.line(format!("{d} <- stata_jk_full"));
        let n_units = cluster
            .as_ref()
            .map_or("NULL".to_string(), |_| "stata_jk_units".to_string());
        out.line(format!(
            "stata_e <- stata_jk_report(stata_jk_obs, stata_jk_m, exps = {exps_arg}, command = {}, cmdname = {}, nfun = {}, n_used = stata_jk_n, level = {}, model = stata_jk_model{}, n_units = {n_units}, depvar = {}, cmdline = {}, notable = {})",
            super::r_text(&command),
            super::r_text(&cmdname),
            super::r_text(nfun),
            super::r_number(level),
            cluster.as_ref().map_or(String::new(), |v| format!(", cluster = {}", super::r_text(v))),
            super::r_text(&depvar),
            super::r_text(cmdline),
            super::stats::r_logical(notable)
        ));
        out.line("stata_r <- list(table = attr(stata_e, \"table\"))");
        out.line("attr(stata_e, \"table\") <- NULL");
        out.line("stata_b <- stats::setNames(as.vector(stata_e$b), colnames(stata_e$b))");
        out.line("stata_vcov <- stata_e$V");
        out.line("stata_estimates <- NULL");
        out.line("rm(stata_jk_full, stata_jk_obs, stata_jk_n, stata_jk_rows, stata_jk_model, stata_jk_sub, stata_jk_unit, stata_jk_units, stata_jk_m)");
        self.drew.set(false);
        self.e_ret = super::returned::Returned::Kept {
            cmd: "jackknife".into(),
            model: String::new(),
            names: &[],
        };
        self.e_new = true;
        self.r_new = Some((&["table"], "jackknife".into()));
        self.estimates = None;
        self.estimates_lost = Some(
            "the model here is jackknife's: predict and estat after jackknife are not translated yet"
                .into(),
        );
        self.esample = false;
        out.caveat("each replication runs the command on the data with one observation (or cluster) left out, as Stata's jackknife does");
        Ok(())
    }

    /// The statistics of a prefix command's expression list: `_b` (after
    /// regress, logit or probit), `name=exp` or `exp` (named `_jk_1`, ...).
    pub(super) fn jackknife_stats(
        &mut self,
        explist: &str,
        inner: &Command,
        eclass: bool,
        body: &mut Out,
    ) -> Result<(Vec<Stat>, bool), Untranslated> {
        let items = exp_items(explist);
        let items = if items.is_empty() && eclass {
            vec!["_b".to_string()]
        } else {
            items
        };
        if items.is_empty() {
            return Err(no("jackknife: expression list required (Stata: r(100))"));
        }
        let mut stats = Vec::new();
        let mut unnamed = 0;
        let mut by_b = false;
        for item in &items {
            if item == "_b" {
                if !eclass || !matches!(inner.name.as_str(), "regress" | "logit" | "probit") {
                    return Err(no(
                        "jackknife _b is translated after regress, logit and probit; not here yet",
                    ));
                }
                if items.len() > 1 {
                    return Err(no(
                        "jackknife _b with other expressions is not translated yet",
                    ));
                }
                by_b = true;
                continue;
            }
            let (name, exp) = match item.split_once('=') {
                Some((n, e)) if super::is_name(n.trim()) && !e.starts_with('=') => {
                    (n.trim().to_string(), e.trim().to_string())
                }
                _ => {
                    unnamed += 1;
                    (format!("_jk_{unnamed}"), item.clone())
                }
            };
            let exp = exp
                .strip_prefix('(')
                .and_then(|e| e.strip_suffix(')'))
                .map_or(exp.clone(), str::to_string);
            let (code, kind) = self.value(&exp, body)?;
            if kind == Kind::Text {
                return Err(no(format!("{exp} is a string (Stata: type mismatch)")));
            }
            stats.push(Stat { name, exp, code });
        }
        Ok((stats, by_b))
    }
}
