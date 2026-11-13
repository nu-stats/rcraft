//! `nlcom exp` and `nlcom (name: exp) (name: exp) ... [, level(#)]`:
//! nonlinear combinations of the coefficients with delta-method standard
//! errors, as Stata 19 prints them after a model, and the r() it leaves
//! (`plan/stata/nlcom.md`). The expressions are Stata mode's own (`_b[x]`,
//! `exp()`, `invlogit()`, ...), worked out at the coefficients and, for the
//! derivatives, at the coefficients moved one at a time (central
//! differences, extrapolated).

use super::model::Fit;
use super::*;

pub(super) const NLCOM_HELPER: &str = r##"stata_nlcom <- function(exprs, names, texts, depvar, level = 95, show = TRUE) {
  b0 <- stata_b
  env <- environment(stata_bget)
  on.exit(assign("stata_b", b0, envir = env), add = TRUE)
  at <- function(bv) {
    assign("stata_b", bv, envir = env)
    vapply(exprs, function(f) as.numeric(f()), 0)
  }
  est <- at(b0)
  if (any(!is.finite(est))) stop("expression is not a number at the estimates (Stata: r(498))", call. = FALSE)
  k <- length(b0)
  J <- matrix(0, length(exprs), k)
  for (j in seq_len(k)) {
    if (is.na(b0[j])) next
    d <- function(h) {
      up <- b0
      dn <- b0
      up[j] <- up[j] + h
      dn[j] <- dn[j] - h
      (at(up) - at(dn)) / (2 * h)
    }
    h <- 1e-3 * (abs(b0[j]) + 1e-3)
    J[, j] <- (4 * d(h / 2) - d(h)) / 3
  }
  assign("stata_b", b0, envir = env)
  Vb <- as.matrix(stata_vcov)
  if (!is.null(rownames(Vb)) && all(names(b0) %in% rownames(Vb))) Vb <- Vb[names(b0), names(b0), drop = FALSE]
  Vb[is.na(Vb)] <- 0
  V <- J %*% Vb %*% t(J)
  dimnames(V) <- list(names, names)
  se <- sqrt(diag(V))
  z <- est / se
  p <- 2 * stats::pnorm(-abs(z))
  crit <- stats::qnorm(1 - (100 - level) / 200)
  lb <- est - crit * se
  ub <- est + crit * se
  bm <- matrix(est, 1, dimnames = list("c1", names))
  tab <- rbind(b = est, se = se, z = z, pvalue = p, ll = lb, ul = ub, df = NA_real_, crit = crit, eform = 0)
  colnames(tab) <- names
  N <- tryCatch(stata_e[["N"]], error = function(e) NULL)
  r <- list(level = level, N = if (is.null(N)) NA_real_ else N, table = tab, b = bm, V = V)
  if (!show) return(r)
  g <- function(v) stata_fmt(v, "%9.0g", trim = TRUE)
  ab <- function(s) if (nchar(s) <= 12) s else paste0(substr(s, 1, 10), "~", substr(s, nchar(s), nchar(s)))
  cat("\n")
  for (i in seq_along(names)) cat(sprintf("%12s: %s", names[i], texts[i]), "\n", sep = "")
  rule <- strrep("-", 78)
  cat("\n", rule, "\n", sep = "")
  cat(sprintf("%12s | Coefficient  Std. err.      z    P>|z|%25s", ab(depvar),
              paste0("[", stata_fmt(level, "%9.0g", trim = TRUE), "% conf. interval]")), "\n", sep = "")
  cat(strrep("-", 13), "+", strrep("-", 64), "\n", sep = "")
  for (i in seq_along(names)) {
    cat(sprintf("%12s |%11s%11s%9.2f%8.3f%13s%12s", ab(names[i]), g(est[i]), g(se[i]), z[i], p[i], g(lb[i]), g(ub[i])), "\n", sep = "")
  }
  cat(rule, "\n", sep = "")
  invisible(r)
}"##;

/// The r() nlcom leaves (Stata 19), with the matrices r(b), r(V), r(table).
const R_NLCOM: &[&str] = &["level", "N"];

/// `(name: exp) (exp) ...` as its parts, or the whole text as one.
fn parts(text: &str) -> Result<Vec<(Option<String>, String)>, Untranslated> {
    let t = text.trim();
    if !t.starts_with('(') {
        return Ok(vec![(None, t.to_string())]);
    }
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    for (i, ch) in t.char_indices() {
        match ch {
            '(' => {
                if depth == 0 {
                    start = i + 1;
                }
                depth += 1;
            }
            ')' => {
                depth = depth
                    .checked_sub(1)
                    .ok_or_else(|| no("nlcom: unbalanced parentheses (Stata: r(198))"))?;
                if depth == 0 {
                    let inner = t.get(start..i).unwrap_or_default().trim();
                    out.push(match inner.split_once(':') {
                        Some((n, e)) if is_name(n.trim()) && !e.trim_start().starts_with(':') => {
                            (Some(n.trim().to_string()), e.trim().to_string())
                        }
                        _ => (None, inner.to_string()),
                    });
                }
            }
            c if depth == 0 && !c.is_whitespace() => {
                // `(a) + 1`: an expression in parentheses, not a list
                return Ok(vec![(None, t.to_string())]);
            }
            _ => {}
        }
    }
    if depth != 0 {
        return Err(no("nlcom: unbalanced parentheses (Stata: r(198))"));
    }
    Ok(out)
}

impl Session {
    pub(super) fn nlcom(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "nlcom")?;
        let est = self.estimates_for("nlcom")?;
        if let Some(o) = opts.other(&[("level", 1)]) {
            return Err(no(format!(
                "nlcom with the option {o} is not translated yet"
            )));
        }
        // the single-equation models whose nlcom table Stata 19 was seen to print (the
        // dependent variable in the stub, _t after stcox)
        if !matches!(
            est.fit,
            Fit::Regress
                | Fit::Logit
                | Fit::Probit
                | Fit::Poisson
                | Fit::Nbreg
                | Fit::Ologit
                | Fit::Oprobit
                | Fit::Xtreg
                | Fit::Ivregress
                | Fit::Clogit
                | Fit::Stcox
        ) {
            return Err(no(format!(
                "nlcom after {} is not translated yet",
                est.fit.name()
            )));
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
        let list = parts(&c.parts.main)?;
        if list.is_empty() || list.iter().any(|(_, e)| e.is_empty()) {
            return Err(no("nlcom needs an expression"));
        }
        let mut fns = Vec::new();
        let mut names = Vec::new();
        let mut texts = Vec::new();
        for (i, (name, e)) in list.iter().enumerate() {
            if !e.contains("_b[") {
                return Err(no(format!(
                    "nlcom {e}: an expression of the coefficients (_b[x]) is translated"
                )));
            }
            let (code, kind) = self.display_value(e, out)?;
            if kind == Kind::Text {
                return Err(no(format!("nlcom {e}: a string (Stata: type mismatch)")));
            }
            if code.contains("stata_v = ") {
                return Err(no(format!(
                    "nlcom {e}: an expression of the data's variables is not translated"
                )));
            }
            fns.push(format!("function() {code}"));
            names.push(r_text(
                &name.clone().unwrap_or_else(|| format!("_nl_{}", i + 1)),
            ));
            texts.push(r_text(e));
        }
        for h in ["stata_nlcom", "stata_fmt", "stata_returned"] {
            self.helpers.insert(h);
        }
        out.line(format!(
            "stata_r <- stata_nlcom(list({}), c({}), c({}), {}, level = {level}{})",
            fns.join(", "),
            names.join(", "),
            texts.join(", "),
            r_text(&est.dep),
            if self.quiet(c) { ", show = FALSE" } else { "" }
        ));
        self.r_new = Some((R_NLCOM, "nlcom".into()));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::parts;

    #[test]
    fn named_and_plain_expressions() {
        let p = |t: &str| parts(t).map_err(|e| e.reason);
        assert_eq!(
            p("_b[z] / _b[w]"),
            Ok(vec![(None, "_b[z] / _b[w]".to_string())])
        );
        assert_eq!(
            p("(ratio: _b[z] / _b[w]) (prod: _b[z] * _b[w])"),
            Ok(vec![
                (Some("ratio".to_string()), "_b[z] / _b[w]".to_string()),
                (Some("prod".to_string()), "_b[z] * _b[w]".to_string())
            ])
        );
        assert_eq!(
            p("(_b[z] + 1) * 2"),
            Ok(vec![(None, "(_b[z] + 1) * 2".to_string())])
        );
        assert!(p("(a: _b[z]").is_err());
    }
}
