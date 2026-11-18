//! `nestreg [, quietly lr lrtable waldtable store(stub)]: command depvar (block) (block) ...`
//! for regress, logit, probit and poisson, as Stata 19 runs it (observed on made-up data; notes in
//! `plan/stata/nestreg.md`).
//!
//! Each block's model is the command on the variables of the blocks so far, translated as it would be
//! on its own (so its output is the command's), on the rows the full model uses. After each fit R
//! keeps what the tables need: the Wald test of the block's variables in that model, its R2 and its
//! log likelihood. The tables are printed once the last block is done, and r(wald) and r(lr) left.
//!
//! Not translated: weights, factor variables and operators, a variable listed twice, options of the
//! command other than level() (and, for regress, vce() and robust), commands other than the four.
//! When run, collinear variables and a number too wide for its column stop the line.

use super::stepwise::{read_terms, Sw};
use super::*;
use crate::reader::Prefix;

pub(super) const NESTREG_HELPER: &str = r##"stata_nr_start <- function(d, vars) {
  for (v in vars) if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
  stata_nr <<- list()
  invisible(NULL)
}
# after a block's model is fitted: the Wald test of the block's variables in it, and what the tables use
stata_nr_block <- function(vars_in_model, block_vars, kind) {
  fit <- stata_estimates
  b <- stata_b
  if (anyNA(b)) stop("nestreg: the variables are collinear, where Stata leaves some out with a note; not translated", call. = FALSE)
  cols <- which(attr(stats::model.matrix(fit), "assign") %in% match(block_vars, vars_in_model))
  V <- as.matrix(stata_vcov)
  bk <- unname(b[cols])
  W <- drop(crossprod(bk, solve(unname(V[cols, cols, drop = FALSE]), bk)))
  e <- stata_e
  stata_nr[[length(stata_nr) + 1]] <<- list(W = W, q = length(cols),
    df_r = if (kind == "regress") stata_df else NA_real_, r2 = if (kind == "regress") e$r2 else NA_real_,
    ll = e$ll, ll0 = e$ll_0, df_m = e$df_m, rank = e$rank, N = e$N)
  invisible(NULL)
}
# the tables (Wald, then likelihood ratio) and r(wald), r(lr)
stata_nr_end <- function(kind, show_wald, show_lr, quiet, show) {
  bl <- stata_nr
  k <- length(bl)
  g <- function(f) vapply(bl, f, 0)
  cell <- function(v, w, f) {
    s <- if (f == "g") stata_fmt(v, "%9.0g", trim = TRUE) else sprintf(f, v)
    if (nchar(s) > w) stop("nestreg: a number does not fit its column of the table, whose widening is not matched; not translated", call. = FALSE)
    formatC(s, width = w)
  }
  txt <- function(s, w) formatC(s, width = w)
  first <- function(i) sprintf(" %5d ", i)
  box <- function(widths, heads, rows) {
    inner <- sum(widths)
    c(paste0("  +", strrep("-", 7 + 1 + inner + 1), "+"),
      vapply(heads, function(h) paste0("  |", h[1], "|", paste0(h[-1], collapse = ""), " |"), ""),
      paste0("  |", strrep("-", 7), "+", strrep("-", inner + 1), "|"),
      vapply(rows, function(r) paste0("  |", r[1], "|", paste0(r[-1], collapse = ""), " |"), ""),
      paste0("  +", strrep("-", 7 + 1 + inner + 1), "+"))
  }
  res <- list(); out <- list()
  names_rows <- paste0("block", seq_len(k))
  if (show_lr) {
    ll <- g(function(x) x$ll); rank <- g(function(x) x$rank); N <- g(function(x) x$N); dfm <- g(function(x) x$df_m)
    LR <- 2 * (ll - c(bl[[1]]$ll0, ll[-k])); dfb <- diff(c(0, dfm))
    p <- stats::pchisq(LR, dfb, lower.tail = FALSE)
    AIC <- -2 * ll + 2 * rank; BIC <- -2 * ll + rank * log(N)
    m <- cbind(LL = ll, LR = LR, df_block = dfb, pvalue = p, AIC = AIC, BIC = BIC); rownames(m) <- names_rows
    res$lr <- m
    w <- c(10, 9, 7, 9, 10, 10)
    out$lr <- box(w, list(c(" Block ", txt("LL", 10), txt("LR", 9), txt("df", 7), txt("Pr > LR", 9), txt("AIC", 10), txt("BIC", 10))),
                  lapply(seq_len(k), function(i) c(first(i), cell(ll[i], 10, "g"), cell(LR[i], 9, "%.2f"), cell(dfb[i], 7, "%d"),
                                                   cell(p[i], 9, "%6.4f"), cell(AIC[i], 10, "g"), cell(BIC[i], 10, "g"))))
  }
  if (show_wald) {
    W <- g(function(x) x$W); q <- g(function(x) x$q)
    if (kind == "regress") {
      df_r <- g(function(x) x$df_r); Fv <- W / q; p <- stats::pf(Fv, q, df_r, lower.tail = FALSE)
      r2 <- g(function(x) x$r2); dr2 <- c(NA_real_, diff(r2))
      m <- cbind(F = Fv, df_block = q, df_r = df_r, pvalue = p, R2 = r2, deltaR2 = dr2)
      w <- c(8, 7, 10, 9, 9, 9)
      out$wald <- box(w, list(c("       ", txt("", 8), txt("Block", 7), txt("Residual", 10), txt("", 9), txt("", 9), txt("Change", 9)),
                              c(" Block ", txt("F", 8), txt("df", 7), txt("df", 10), txt("Pr > F", 9), txt("R2", 9), txt("in R2", 9))),
                      lapply(seq_len(k), function(i) c(first(i), cell(Fv[i], 8, "%.2f"), cell(q[i], 7, "%d"), cell(df_r[i], 10, "%d"),
                                                       cell(p[i], 9, "%6.4f"), cell(r2[i], 9, "%6.4f"),
                                                       if (i == 1) txt("", 9) else cell(dr2[i], 9, "%6.4f"))))
    } else {
      p <- stats::pchisq(W, q, lower.tail = FALSE)
      m <- cbind(`Wald chi2` = W, df_block = q, pvalue = p)
      w <- c(10, 7, 9)
      out$wald <- box(w, list(c(" Block ", txt("Wald chi2", 10), txt("df", 7), txt("Pr > F", 9))),
                      lapply(seq_len(k), function(i) c(first(i), cell(W[i], 10, "%.2f"), cell(q[i], 7, "%d"), cell(p[i], 9, "%6.4f"))))
    }
    rownames(m) <- names_rows
    res$wald <- m
  }
  # a blank line before each table, two before the first when the blocks' own output was shown; none after
  if (show) {
    lead <- if (quiet) 1 else 2
    for (t in c("wald", "lr")) if (!is.null(out[[t]])) {
      cat(strrep("\n", lead), paste0(out[[t]], "\n"), sep = "")
      lead <- 1
    }
  }
  res
}
"##;

/// e(cmdline) of one block's model: the command, the outcome and the variables so far, then what
/// followed the blocks (if, in, options) as typed.
fn model_cmdline(c: &Command, dep: &str, vars: &[&str]) -> String {
    let after = super::returned::after_name(c);
    let rest = after
        .trim()
        .strip_prefix(c.parts.main.trim())
        .unwrap_or("")
        .trim();
    let base = format!("{} {dep} {}", c.written, vars.join(" "));
    if rest.is_empty() {
        base
    } else {
        format!("{base} {rest}")
    }
}

impl Session {
    fn nr_fail(&self, out: &mut Out, msg: &str, rc: u32) -> Result<(), Untranslated> {
        out.line(format!(
            "stop({}, call. = FALSE)",
            r_text(&format!("{msg} (Stata: r({rc}))"))
        ));
        Ok(())
    }

    /// `nestreg [, options]: regress|logit|probit|poisson depvar (block) (block) ... [if] [in] [, options]`.
    #[allow(clippy::too_many_lines)]
    pub(super) fn nestreg(&mut self, c: &Command, out: &mut Out) -> Result<(), Untranslated> {
        let text = c
            .prefixes
            .iter()
            .find_map(|p| match p {
                Prefix::Nestreg(o) => Some(o.clone()),
                _ => None,
            })
            .unwrap_or_default();
        let kind = c.name.as_str();
        if kind == "summarize" {
            return self.nr_fail(out, "summarize is not supported by nestreg", 199);
        }
        if !matches!(kind, "regress" | "logit" | "probit" | "poisson") {
            return Err(no(format!("nestreg with {kind} is not translated")));
        }
        self.no_by(c, "nestreg")?;
        if c.parts.weight.is_some() {
            return Err(no("nestreg with weights is not translated yet"));
        }
        let given = crate::lang::syntax::options(&text);
        let so = Opts(&given);
        if let Some(o) = so.other(&[
            ("quietly", 5),
            ("lr", 2),
            ("lrtable", 7),
            ("waldtable", 9),
            ("store", 5),
        ]) {
            return self.nr_fail(out, &format!("option {o} not allowed"), 198);
        }
        let opts = Opts(&c.options);
        let known: &[(&str, usize)] = if kind == "regress" {
            &[("level", 3), ("vce", 3), ("robust", 1), ("cluster", 2)]
        } else {
            &[("level", 3)]
        };
        if let Some(o) = opts.other(known) {
            return Err(no(format!(
                "nestreg: {kind} with the option {o} is not translated"
            )));
        }
        let main = c.parts.main.trim();
        let (dep, rest) = match main.split_once(char::is_whitespace) {
            Some((d, r)) => (d.trim(), r.trim()),
            None => (main, ""),
        };
        if dep.starts_with('(') {
            return self.nr_fail(out, "depvar cannot be part of a variable block", 198);
        }
        if !is_name(dep) {
            return Err(no(
                "nestreg needs an outcome variable and blocks of plain names",
            ));
        }
        let Some(blocks) = read_terms(rest) else {
            return Err(no(
                "nestreg is translated with plain variable names, alone or in a parenthesised group",
            ));
        };
        let flat: Vec<&str> = blocks.iter().flatten().map(String::as_str).collect();
        let repeats = {
            let mut seen = std::collections::HashSet::new();
            flat.iter().any(|w| !seen.insert(*w)) || flat.contains(&dep)
        };
        if repeats {
            return Err(no(
                "nestreg with a variable listed twice is not translated (Stata's notes differ)",
            ));
        }
        if blocks.is_empty() {
            return self.nr_fail(out, "varblocklist required", 100);
        }
        let lr = so.has("lr", 2) || so.has("lrtable", 7);
        let wald = so.has("waldtable", 9) || !lr;
        let robust = opts.has("vce", 3) || opts.has("robust", 1) || opts.has("cluster", 2);
        if lr && robust {
            return self.nr_fail(
                out,
                "option lrtable not allowed with pweights, robust VCE, or clustered VCE",
                198,
            );
        }
        let stub = match so.arg("store", 5) {
            None => None,
            Some(s) if is_name(s.trim()) => Some(s.trim().to_string()),
            Some(s) => return Err(no(format!("nestreg, store({s}) is not translated"))),
        };
        let qopt = so.has("quietly", 5);
        // a quietly (or capture) before nestreg silences the block lines and the tables too
        let loud = !super::silent(c);
        let d = self.data()?;
        let dep_r = r_name(dep);
        let lhs = if matches!(kind, "logit" | "probit") {
            format!("as.numeric({dep_r} != 0)")
        } else {
            dep_r
        };
        for h in ["stata_nestreg", "stata_fmt"] {
            self.helpers.insert(h);
        }
        let mut vars = vec![dep.to_string()];
        vars.extend(flat.iter().map(|v| (*v).to_string()));
        out.line(format!(
            "stata_nr_start({d}, c({}))",
            vars.iter()
                .map(|v| r_text(v))
                .collect::<Vec<_>>()
                .join(", ")
        ));
        self.sw = Some(Sw {
            vars,
            cmdline: String::new(),
            mark: false,
        });
        let mut so_far: Vec<&str> = Vec::new();
        let mut result = Ok(());
        for (k, block) in blocks.iter().enumerate() {
            let words: Vec<&str> = block.iter().map(String::as_str).collect();
            so_far.extend(words.iter().copied());
            if loud {
                out.line(format!(
                    "cat({})",
                    r_text(&format!(
                        "{}Block {}: {}\n",
                        if qopt && k > 0 { "" } else { "\n" },
                        k + 1,
                        words.join(" ")
                    ))
                ));
            }
            let rhs = so_far
                .iter()
                .map(|v| format!("`{v}`"))
                .collect::<Vec<_>>()
                .join(" + ");
            out.line(format!(
                "stata_sw_formula <- stats::as.formula({})",
                r_text(&format!("{lhs} ~ {rhs}"))
            ));
            if let Some(sw) = self.sw.as_mut() {
                sw.cmdline = model_cmdline(c, dep, &so_far);
            }
            let mut inner = c.clone();
            inner.prefixes.retain(|p| !matches!(p, Prefix::Nestreg(_)));
            if qopt && loud {
                inner.prefixes.push(Prefix::Quietly);
            }
            inner.parts.main = format!("{dep} {}", so_far.join(" "));
            if let Err(e) = self.dispatch(&inner, out) {
                result = Err(e);
                break;
            }
            let c_of = |ws: &[&str]| {
                format!(
                    "c({})",
                    ws.iter().map(|v| r_text(v)).collect::<Vec<_>>().join(", ")
                )
            };
            out.line(format!(
                "stata_nr_block({}, {}, {})",
                c_of(&so_far),
                c_of(&words),
                r_text(kind)
            ));
            if let Some(stub) = &stub {
                if let Err(e) = self.eststo_store(&format!("{stub}{}", k + 1), out) {
                    result = Err(e);
                    break;
                }
            }
        }
        self.sw = None;
        result?;
        out.line(format!(
            "stata_r <- stata_nr_end({}, {}, {}, {}, {})",
            r_text(kind),
            super::stats::r_logical(wald),
            super::stats::r_logical(lr),
            super::stats::r_logical(qopt),
            super::stats::r_logical(loud)
        ));
        self.r_new = Some((
            match (wald, lr) {
                (true, true) => &["lr", "wald"],
                (true, false) => &["wald"],
                _ => &["lr"],
            },
            "nestreg".into(),
        ));
        out.caveat(
            "nestreg: each block's model is fitted on the rows the full model uses; its tables are \
             worked out in R from the models' estimates and variances",
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::data_r::translate;
    use crate::reader::read;

    fn r(source: &str) -> Vec<String> {
        translate(&read(source))
            .lines
            .into_iter()
            .map(|l| match l.result {
                Ok(r) => r.code,
                Err(e) => format!("NOT: {}", e.reason),
            })
            .collect()
    }

    #[test]
    fn each_block_is_the_model_so_far() {
        let out = r("use d
nestreg: regress y (x1) (x2 x3) x4 if x5 > 0");
        let code = &out[1];
        for want in [
            "stata_nr_start(",
            "Block 1: x1",
            "Block 2: x2 x3",
            "Block 3: x4",
            "stats::as.formula(\"y ~ `x1` + `x2` + `x3`\")",
            "\"regress y x1 x2 x3 if x5 > 0\"",
            "\"regress y x1 x2 x3 x4 if x5 > 0\"",
            "stata_nr_block(c(\"x1\", \"x2\", \"x3\", \"x4\"), c(\"x4\"), \"regress\")",
            "stata_r <- stata_nr_end(\"regress\", TRUE, FALSE, FALSE, TRUE)",
        ] {
            assert!(code.contains(want), "{want}: {out:?}");
        }
    }

    #[test]
    fn options_choose_the_tables() {
        let out = r("use d
nestreg, lr: logit z (x1) (x2)
nestreg, lrtable waldtable: logit z (x1) (x2)
nestreg, quietly store(m): regress y (x1) (x2), vce(robust)");
        assert!(
            out[1].contains("stata_nr_end(\"logit\", FALSE, TRUE, FALSE, TRUE)"),
            "{out:?}"
        );
        assert!(
            out[2].contains("stata_nr_end(\"logit\", TRUE, TRUE, FALSE, TRUE)"),
            "{out:?}"
        );
        assert!(
            out[3].contains("stata_nr_end(\"regress\", TRUE, FALSE, TRUE, TRUE)")
                && out[3].contains("\"regress y x1 x2 , vce(robust)\""),
            "{out:?}"
        );
    }

    #[test]
    fn statas_errors_are_kept_and_the_rest_refused() {
        let out = r("use d
nestreg: regress y
nestreg, foo: regress y (x1) (x2)
nestreg, lr: regress y (x1) (x2), vce(robust)
nestreg: regress y (x1) (x2) [aweight = w]
nestreg: regress y (x1 x2 x1)
nestreg: regress y (i.g) (x1)
nestreg: logit z (x1) (x2), robust
nestreg: summarize y (x1) (x2)");
        assert!(
            out[1].contains("varblocklist required") && out[1].contains("(Stata: r(100))"),
            "{out:?}"
        );
        assert!(
            out[2].contains("option foo not allowed") && out[2].contains("(Stata: r(198))"),
            "{out:?}"
        );
        assert!(
            out[3]
                .contains("option lrtable not allowed with pweights, robust VCE, or clustered VCE"),
            "{out:?}"
        );
        for i in 4..=7 {
            assert!(out[i].starts_with("NOT: nestreg"), "{i}: {out:?}");
        }
        assert!(
            out[8].contains("summarize is not supported by nestreg")
                && out[8].contains("(Stata: r(199))"),
            "{out:?}"
        );
    }

    #[test]
    fn a_quietly_before_nestreg_prints_nothing() {
        let out = r("use d
quietly nestreg: regress y (x1) (x2)
nestreg: regress (x1) (x2)");
        assert!(!out[1].contains("cat("), "{out:?}");
        assert!(
            out[1].contains("stata_nr_end(\"regress\", TRUE, FALSE, FALSE, FALSE)"),
            "{out:?}"
        );
        assert!(
            out[2].contains("depvar cannot be part of a variable block"),
            "{out:?}"
        );
    }
}
