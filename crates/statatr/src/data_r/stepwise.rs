//! `stepwise [, pr(#) pe(#) forward hierarchical lockterm1 lr]: command depvar terms`
//! for regress, logit, probit and poisson with plain variables, as Stata 19 runs it
//! (observed on made-up data; notes in `plan/stata/stepwise.md`).
//!
//! The search runs in R, where the data are: it fits the models, tests each term
//! (Wald, or likelihood ratio with `lr`), prints Stata's trace and hands back the
//! formula of the terms it kept. The command is then translated as usual, with that
//! formula where the right-hand side would be, on the rows the full model uses.
//!
//! Not translated: weights, factor variables and operators, options of the command
//! other than level() and (regress) beta, other options of stepwise. A search that
//! would add a term back after removing one (or remove one after adding) stops with
//! a note when it runs: Stata's order for that is not decoded.

use super::*;
use crate::reader::Prefix;

pub(super) const STEPWISE_HELPER: &str = r##"stata_sw_rows <- function(d, v) d[stats::complete.cases(d[v]), , drop = FALSE]
stata_stepwise <- function(d, kind, lhs, dep, terms, pr = NULL, pe = NULL, forward = FALSE, hier = FALSE,
                           lock1 = FALSE, lr = FALSE, show = TRUE) {
  vars <- unique(c(dep, unlist(terms)))
  for (v in vars) if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
  # the rows the full model uses stay the sample through the search and the final fit
  dd <- stata_sw_rows(d, vars)
  if (!nrow(dd)) stop("no observations (Stata: r(2000))", call. = FALSE)
  nt <- length(terms)
  label <- vapply(terms, paste, "", collapse = " ")
  fam <- switch(kind, logit = stats::binomial("logit"), probit = stats::binomial("probit"), poisson = stats::poisson(), NULL)
  refuse <- function(...) stop("stepwise: ", ..., "; not translated", call. = FALSE)
  fit_of <- function(idx) {
    v <- unlist(terms[idx])
    f <- stats::as.formula(paste(lhs, "~", if (length(v)) paste0("`", v, "`", collapse = " + ") else "1"))
    fit <- if (kind == "regress") stats::lm(f, data = dd) else
      suppressWarnings(do.call(stats::glm, list(formula = f, family = fam, data = dd,
                                                control = stats::glm.control(epsilon = 1e-12, maxit = 100))))
    if (anyNA(stats::coef(fit))) refuse("the terms are collinear, where Stata leaves variables out with a note")
    # probit: Stata's own steps, and the variance from the observed information, as its test uses
    if (kind == "probit") fit <- suppressWarnings(stata_probit_newton(fit))
    list(fit = fit, vars = v, V = if (kind == "regress") stats::vcov(fit) else stata_glm_vcov(fit))
  }
  # the test that a term's coefficients are 0 in the fit m, or the likelihood ratio against s
  p_wald <- function(m, t) {
    pos <- match(terms[[t]], m$vars)
    cols <- which(attr(stats::model.matrix(m$fit), "assign") %in% pos)
    b <- stats::coef(m$fit)[cols]
    W <- drop(crossprod(b, solve(unname(m$V[cols, cols, drop = FALSE]), b)))
    q <- length(cols)
    if (kind == "regress") stats::pf(W / q, q, m$fit$df.residual, lower.tail = FALSE) else stats::pchisq(W, q, lower.tail = FALSE)
  }
  p_lr <- function(m, s) {
    q <- length(stats::coef(m$fit)) - length(stats::coef(s$fit))
    stats::pchisq(2 * (as.numeric(stats::logLik(m$fit)) - as.numeric(stats::logLik(s$fit))), q, lower.tail = FALSE)
  }
  p_remove <- function(inc, t) {
    m <- fit_of(inc)
    if (lr) p_lr(m, fit_of(setdiff(inc, t))) else p_wald(m, t)
  }
  p_add <- function(inc, t) {
    m <- fit_of(sort(c(inc, t)))
    if (lr) p_lr(m, fit_of(inc)) else p_wald(m, t)
  }
  # Stata fits the full model first, and leaves out for the whole search a term it cannot estimate
  # ("omitted because of estimability"), by a rule not matched: a collinear full model is refused
  fit_of(seq_len(nt))
  lock <- if (lock1) 1L else integer(0)
  inc <- if (forward) lock else seq_len(nt)
  say <- function(p, op, thr, verb, t) if (show) cat(sprintf("p = %6.4f %s %6.4f, %s %s\n", p, op, thr, verb, label[t]))
  if (show) cat("\n", if (lr) "LR" else "Wald", " test, begin with ",
                if (!forward) "full" else if (lock1) "term 1" else "empty", " model:\n", sep = "")
  # both pr() and pe(): which of a removal and an addition comes first is not decoded, so a search
  # where the other kind of step is ever possible is refused
  guard_add <- function(inc) for (t in setdiff(seq_len(nt), inc)) if (p_add(inc, t) < pe)
    refuse("pe() would add a term back after pr() removed one, in an order not matched yet")
  guard_remove <- function(inc) for (t in setdiff(inc, lock)) if (p_remove(inc, t) >= pr)
    refuse("pr() would remove a term after pe() added one, in an order not matched yet")
  changed <- FALSE
  if (!forward && !hier) {
    repeat {
      cand <- setdiff(inc, lock)
      if (!length(cand)) break
      ps <- vapply(cand, function(t) p_remove(inc, t), 0)
      k <- which.max(ps)
      if (ps[k] < pr) break
      say(ps[k], ">=", pr, "removing", cand[k])
      inc <- setdiff(inc, cand[k]); changed <- TRUE
      if (!is.null(pe)) guard_add(inc)
    }
  } else if (!forward) {
    repeat {
      cand <- setdiff(inc, lock)
      if (!length(cand)) break
      t <- max(cand); p <- p_remove(inc, t)
      if (p < pr) { say(p, "< ", pr, "keeping", t); break }
      say(p, ">=", pr, "removing", t)
      inc <- setdiff(inc, t); changed <- TRUE
    }
  } else if (!hier) {
    repeat {
      cand <- setdiff(seq_len(nt), inc)
      if (!length(cand)) break
      ps <- vapply(cand, function(t) p_add(inc, t), 0)
      k <- which.min(ps)
      if (ps[k] >= pe) break
      say(ps[k], "< ", pe, "adding", cand[k])
      inc <- c(inc, cand[k]); changed <- TRUE
      if (!is.null(pr)) guard_remove(inc)
    }
  } else {
    repeat {
      cand <- setdiff(seq_len(nt), inc)
      if (!length(cand)) break
      t <- min(cand); p <- p_add(inc, t)
      if (p >= pe) { say(p, ">=", pe, "testing", t); break }
      say(p, "< ", pe, "adding", t)
      inc <- c(inc, t); changed <- TRUE
    }
  }
  if (!changed && show) cat(sprintf("p %s %6.4f for all terms in model\n", if (forward) ">=" else "<", if (forward) pe else pr))
  # the terms kept, in the order written; a model of the constant alone is printed differently from
  # Stata's by the model printers (F(0, n), LR chi2(0)), so that is refused
  if (!length(inc)) refuse("no term is kept, and the printing of a model of the constant alone is not matched")
  # the final model lists the terms in the order the search entered them (the order written when
  # terms were only removed)
  stats::as.formula(paste(lhs, "~", paste0("`", unlist(terms[inc]), "`", collapse = " + ")))
}
"##;

/// What a stepwise search leaves for the translation of its command.
#[derive(Debug, Clone)]
pub(super) struct Sw {
    /// The outcome and every variable of the full model: the rows with none missing.
    vars: Vec<String>,
    /// e(cmdline): `stepwise ` and the line after its word, as typed.
    cmdline: String,
}

/// The terms of a stepwise command: a word, or a parenthesised group of words.
fn read_terms(text: &str) -> Option<Vec<Vec<String>>> {
    let mut terms = Vec::new();
    let mut rest = text.trim();
    while !rest.is_empty() {
        if let Some(inner) = rest.strip_prefix('(') {
            let (group, after) = inner.split_once(')')?;
            let names: Vec<String> = group.split_whitespace().map(str::to_string).collect();
            if names.is_empty() || !names.iter().all(|n| is_name(n)) {
                return None;
            }
            terms.push(names);
            rest = after.trim_start();
        } else {
            let end = rest
                .find(|c: char| c.is_whitespace() || c == '(')
                .unwrap_or(rest.len());
            let (word, after) = rest.split_at(end);
            if !is_name(word) {
                return None;
            }
            terms.push(vec![word.to_string()]);
            rest = after.trim_start();
        }
    }
    Some(terms)
}

/// e(cmdline) of a stepwise line: the word and what follows it, as typed.
fn stepwise_cmdline(c: &Command) -> String {
    let text = c.text.as_str();
    for (k, _) in text.match_indices("stepwise") {
        let before_ok = k == 0
            || text
                .get(..k)
                .and_then(|t| t.chars().last())
                .is_some_and(|ch| ch.is_whitespace() || ch == ':');
        let after = text.get(k + "stepwise".len()..).unwrap_or("");
        let after_ok = after
            .chars()
            .next()
            .is_none_or(|ch| !(ch.is_alphanumeric() || ch == '_'));
        if before_ok && after_ok {
            return format!("stepwise {}", after.trim());
        }
    }
    super::returned::cmdline(c)
}

impl Session {
    /// The model's formula: the one the search made, or `lhs ~ right`.
    pub(super) fn model_formula(&self, lhs: &str, right: &str) -> String {
        if self.sw.is_some() {
            "stata_sw_formula".to_string()
        } else {
            format!("{lhs} ~ {right}")
        }
    }

    /// The rows of a model under stepwise: those the full model uses.
    pub(super) fn sw_rows(&mut self, src: String) -> String {
        match &self.sw {
            None => src,
            Some(sw) => {
                self.helpers.insert("stata_stepwise");
                let vars: Vec<String> = sw.vars.iter().map(|v| r_text(v)).collect();
                format!("stata_sw_rows({src}, c({}))", vars.join(", "))
            }
        }
    }

    /// e(cmdline) for a model under stepwise, and whether the fit is one.
    pub(super) fn sw_cmdline(&self) -> Option<String> {
        self.sw.as_ref().map(|s| s.cmdline.clone())
    }

    fn sw_fail(&self, out: &mut Out, msg: &str, rc: u32) -> Result<(), Untranslated> {
        out.line(format!(
            "stop({}, call. = FALSE)",
            r_text(&format!("{msg} (Stata: r({rc}))"))
        ));
        Ok(())
    }

    /// `stepwise [, options]: regress|logit|probit|poisson depvar terms [if] [in] [, options]`.
    #[allow(clippy::too_many_lines)]
    pub(super) fn stepwise(&mut self, c: &Command, out: &mut Out) -> Result<(), Untranslated> {
        let text = c
            .prefixes
            .iter()
            .find_map(|p| match p {
                Prefix::Stepwise(o) => Some(o.clone()),
                _ => None,
            })
            .unwrap_or_default();
        let kind = c.name.as_str();
        if !matches!(kind, "regress" | "logit" | "probit" | "poisson") {
            return Err(no(format!("stepwise with {kind} is not translated")));
        }
        self.no_by(c, "stepwise")?;
        if c.parts.weight.is_some() {
            return Err(no("stepwise with weights is not translated yet"));
        }
        let given = crate::lang::syntax::options(&text);
        let so = Opts(&given);
        if given.iter().any(|o| o.name == "b") {
            return self.sw_fail(out, "option b not allowed", 198);
        }
        if let Some(o) = so.other(&[
            ("pr", 2),
            ("pe", 2),
            ("forward", 7),
            ("hierarchical", 12),
            ("lockterm1", 9),
            ("lr", 2),
        ]) {
            return Err(no(format!(
                "stepwise with the option {o} is not translated"
            )));
        }
        let opts = Opts(&c.options);
        let known: &[(&str, usize)] = if kind == "regress" {
            &[("level", 3), ("beta", 4)]
        } else {
            &[("level", 3)]
        };
        if let Some(o) = opts.other(known) {
            return Err(no(format!(
                "stepwise: {kind} with the option {o} is not translated"
            )));
        }
        // the terms
        let main = c.parts.main.trim();
        let (dep, rest) = match main.split_once(char::is_whitespace) {
            Some((d, r)) => (d.trim(), r.trim()),
            None => (main, ""),
        };
        if !is_name(dep) {
            return Err(no(
                "stepwise needs an outcome variable and terms of plain names",
            ));
        }
        let Some(terms) = read_terms(rest) else {
            return Err(no(
                "stepwise is translated with plain variable names, alone or in a parenthesised group",
            ));
        };
        let flat: Vec<&str> = terms.iter().flatten().map(String::as_str).collect();
        let repeats = {
            let mut seen = std::collections::HashSet::new();
            flat.iter().any(|w| !seen.insert(*w)) || flat.contains(&dep)
        };
        if repeats {
            return Err(no(
                "stepwise with a variable listed twice is not translated (Stata's notes differ)",
            ));
        }
        // Stata's own checks, in the order the messages were found
        let pr_text = so.arg("pr", 2);
        let pe_text = so.arg("pe", 2);
        let forward_opt = so.has("forward", 7);
        let hier = so.has("hierarchical", 12);
        let number = |t: &Option<String>| -> Result<Option<f64>, Untranslated> {
            match t {
                None => Ok(None),
                Some(s) => s
                    .trim()
                    .parse::<f64>()
                    .map(Some)
                    .map_err(|_| no(format!("stepwise, the number {s} is not translated"))),
            }
        };
        let (pr, pe) = (number(&pr_text)?, number(&pe_text)?);
        if pr.is_none() && pe.is_none() {
            return self.sw_fail(out, "at least one of pr(#) or pe(#) must be specified", 198);
        }
        if forward_opt && pe.is_none() {
            return self.sw_fail(out, "option forward invalid or option pe() is missing", 198);
        }
        for (name, v) in [("pr", pr), ("pe", pe)] {
            if v.is_some_and(|x| !(x > 0.0 && x < 1.0)) {
                return self.sw_fail(
                    out,
                    &format!("{name}() invalid -- invalid number, outside of allowed range"),
                    125,
                );
            }
        }
        if hier && pr.is_some() && pe.is_some() {
            return self.sw_fail(out, "hierarchical pe(#) pr(#) invalid", 198);
        }
        if let (Some(r), Some(e), Some(rt), Some(et)) = (pr, pe, &pr_text, &pe_text) {
            if r <= e {
                return self.sw_fail(
                    out,
                    &format!("pr({}) <= pe({}) invalid", rt.trim(), et.trim()),
                    198,
                );
            }
        }
        if terms.is_empty() {
            return self.sw_fail(out, "no independent variables", 198);
        }
        // forward with pe() alone; backward with pr() (alone or with pe())
        let forward = forward_opt || (pe.is_some() && pr.is_none());
        let src = self.rows_for(c, out)?;
        let dep_r = r_name(dep);
        let lhs = if matches!(kind, "logit" | "probit") {
            format!("as.numeric({dep_r} != 0)")
        } else {
            dep_r
        };
        let list: Vec<String> = terms
            .iter()
            .map(|t| {
                if let [one] = t.as_slice() {
                    r_text(one)
                } else {
                    format!(
                        "c({})",
                        t.iter().map(|v| r_text(v)).collect::<Vec<_>>().join(", ")
                    )
                }
            })
            .collect();
        let num = |v: Option<f64>| v.map_or("NULL".to_string(), r_number);
        self.helpers.insert("stata_stepwise");
        if kind != "regress" {
            self.helpers.insert("stata_glm_vcov");
        }
        if kind == "probit" {
            self.helpers.insert("stata_probit_newton");
        }
        out.line(format!(
            "stata_sw_formula <- stata_stepwise({src}, {}, {}, {}, list({}), pr = {}, pe = {}, forward = {}, hier = {}, lock1 = {}, lr = {}, show = {})",
            r_text(kind),
            r_text(&lhs),
            r_text(dep),
            list.join(", "),
            num(pr),
            num(pe),
            super::stats::r_logical(forward),
            super::stats::r_logical(hier),
            super::stats::r_logical(so.has("lockterm1", 9)),
            super::stats::r_logical(so.has("lr", 2)),
            super::stats::r_logical(!self.quiet(c)),
        ));
        // the command itself, on the plain list of the variables
        let mut inner = c.clone();
        inner
            .prefixes
            .retain(|p| !matches!(p, Prefix::Stepwise(_) | Prefix::Nestreg(_)));
        inner.parts.main = format!("{dep} {}", flat.join(" "));
        let mut vars = vec![dep.to_string()];
        vars.extend(flat.iter().map(|v| (*v).to_string()));
        self.sw = Some(Sw {
            vars,
            cmdline: stepwise_cmdline(c),
        });
        let done = self.dispatch(&inner, out);
        self.sw = None;
        out.caveat(
            "stepwise: the search fits the models in R and tests each term as Stata's does (Wald, \
             or likelihood ratio with lr); the final model is fitted on the rows the full model uses",
        );
        done
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
    fn a_search_makes_the_formula_the_model_uses() {
        let out = r("use d
stepwise, pr(.2): regress y x1 x2 (x3 x4)
stepwise, pe(.1) lr: logit z x1 x2");
        assert!(
            out[1].contains("stata_sw_formula <- stata_stepwise(")
                && out[1].contains("list(\"x1\", \"x2\", c(\"x3\", \"x4\"))")
                && out[1].contains("pr = 0.2, pe = NULL, forward = FALSE")
                && out[1].contains("stats::lm(stata_sw_formula, data = stata_sw_rows("),
            "{out:?}"
        );
        assert!(
            out[1].contains("\"stepwise , pr(.2): regress y x1 x2 (x3 x4)\""),
            "{out:?}"
        );
        assert!(
            out[2].contains("forward = TRUE")
                && out[2].contains("lr = TRUE")
                && out[2].contains("as.numeric(z != 0)")
                && out[2].contains("stats::glm(stata_sw_formula,"),
            "{out:?}"
        );
    }

    #[test]
    fn statas_errors_are_kept_and_the_rest_refused() {
        let out = r("use d
stepwise: regress y x1 x2
stepwise, pr(.1) pe(.2): regress y x1 x2
stepwise, pr(.2) forward: regress y x1 x2
stepwise, pr(1.5): regress y x1 x2
stepwise, pr(.2) b: regress y x1 x2
stepwise, pe(.1) pr(.2) hierarchical: regress y x1 x2
stepwise, pr(.2): regress y
stepwise, pr(.2): regress y i.g x1
stepwise, pr(.2): regress y x1 [aweight = w]
stepwise, pr(.2): regress y x1 x2, robust
stepwise, pr(.2): summarize y x1
stepwise, pr(.2) vce(robust): regress y x1 x2");
        let want = [
            (1, "at least one of pr(#) or pe(#) must be specified", "198"),
            (2, "pr(.1) <= pe(.2) invalid", "198"),
            (3, "option forward invalid or option pe() is missing", "198"),
            (
                4,
                "pr() invalid -- invalid number, outside of allowed range",
                "125",
            ),
            (5, "option b not allowed", "198"),
            (6, "hierarchical pe(#) pr(#) invalid", "198"),
            (7, "no independent variables", "198"),
        ];
        for (i, msg, rc) in want {
            assert!(
                out[i].contains(msg) && out[i].contains(&format!("(Stata: r({rc}))")),
                "{i}: {out:?}"
            );
        }
        for i in 8..=12 {
            assert!(out[i].starts_with("NOT: stepwise"), "{i}: {out:?}");
        }
    }
}
