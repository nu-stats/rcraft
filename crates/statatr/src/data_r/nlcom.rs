//! `nlcom exp` and `nlcom (name: exp) (name: exp) ... [, level(#)]`:
//! nonlinear combinations of the coefficients with delta-method standard
//! errors, as Stata 19 prints them after a model, and the r() it leaves
//! (`plan/stata/nlcom.md`). The expressions are Stata mode's own (`_b[x]`,
//! `exp()`, `invlogit()`, ...), worked out at the coefficients and, for the
//! derivatives, at the coefficients moved one at a time (central
//! differences, extrapolated).

use super::model::Fit;
use super::*;

pub(super) const NLCOM_HELPER: &str = r##"stata_nlcom <- function(exprs, names, texts, depvar, level = 95, show = TRUE,
                        df = Inf, header = TRUE) {
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
  small <- is.finite(df)
  p <- if (small) 2 * stats::pt(-abs(z), df) else 2 * stats::pnorm(-abs(z))
  crit <- if (small) stats::qt(1 - (100 - level) / 200, df) else stats::qnorm(1 - (100 - level) / 200)
  lb <- est - crit * se
  ub <- est + crit * se
  bm <- matrix(est, 1, dimnames = list("c1", names))
  tab <- rbind(b = est, se = se, z = z, pvalue = p, ll = lb, ul = ub, df = if (small) df else NA_real_, crit = crit, eform = 0)
  if (small) rownames(tab)[3] <- "t"
  colnames(tab) <- names
  N <- tryCatch(stata_e[["N"]], error = function(e) NULL)
  r <- c(list(level = level), if (small) list(df_r = df), list(N = if (is.null(N)) NA_real_ else N, table = tab, b = bm, V = V))
  if (!show) return(r)
  g <- function(v) stata_fmt(v, "%9.0g", trim = TRUE)
  # cut as Stata cuts names: unique among the data's variables
  ab <- function(s) stata_tab_abbrev(s, 12)
  if (header) {
    cat("\n")
    for (i in seq_along(names)) cat(sprintf("%12s: %s", names[i], texts[i]), "\n", sep = "")
  }
  rule <- strrep("-", 78)
  cat("\n", rule, "\n", sep = "")
  cat(sprintf("%12s | Coefficient  Std. err.      %s    P>|%s|%25s", ab(depvar), if (small) "t" else "z", if (small) "t" else "z",
              paste0("[", stata_fmt(level, "%9.0g", trim = TRUE), "% conf. interval]")), "\n", sep = "")
  cat(strrep("-", 13), "+", strrep("-", 64), "\n", sep = "")
  for (i in seq_along(names)) {
    cat(sprintf("%12s |%11s%11s%9.2f%8.3f%13s%12s", ab(names[i]), g(est[i]), g(se[i]), z[i], p[i], g(lb[i]), g(ub[i])), "\n", sep = "")
  }
  cat(rule, "\n", sep = "")
  invisible(r)
}"##;

/// `testnl`: the Wald test of nonlinear restrictions, the derivatives as for nlcom
/// (`plan/stata/nlcom.md`).
pub(super) const TESTNL_HELPER: &str = r##"stata_testnl <- function(exprs, texts, mtest = NA, show = TRUE) {
  b0 <- stata_b
  env <- environment(stata_bget)
  on.exit(assign("stata_b", b0, envir = env), add = TRUE)
  at <- function(bv) {
    assign("stata_b", bv, envir = env)
    vapply(exprs, function(f) as.numeric(f()), 0)
  }
  m <- length(exprs)
  # the equations first: Stata lists them before it works anything out (and stops after them)
  if (show) {
    cat("\n")
    for (i in seq_len(m)) cat(sprintf("  (%d)  %s", i, texts[i]), "\n", sep = "")
  }
  g <- at(b0)
  if (any(!is.finite(g))) stop("expression is not a number at the estimates (Stata: r(498))", call. = FALSE)
  k <- length(b0)
  J <- matrix(0, m, k)
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
  W <- J %*% Vb %*% t(J)
  chi2 <- drop(t(g) %*% solve(W, g))
  rn <- paste0("r", seq_len(m))
  # Stata's names and order: _cons last
  nm <- tryCatch(stata_bnames(), error = function(e) names(b0))
  o <- c(which(nm != "_cons"), which(nm == "_cons"))
  G <- matrix(J[, o, drop = FALSE], m, k, dimnames = list(rn, nm[o]))
  R <- matrix(g, m, 1, dimnames = list(rn, "c1"))
  r <- list(p = stats::pchisq(chi2, m, lower.tail = FALSE), chi2 = chi2, df = m)
  if (!is.na(mtest)) {
    c1 <- g^2 / diag(W)
    p0 <- stats::pchisq(c1, 1, lower.tail = FALSE)
    r$mtmethod <- mtest
    r$mtest <- matrix(c(c1, rep(1, m), p0), m, 3, dimnames = list(rn, c("chi2", "df", "p")))
    p1 <- p0
    # adjusted: a fourth column, adjustp (Stata 19)
    if (mtest == "bonferroni") {
      p1 <- pmin(1, p0 * m)
      r$mtest <- cbind(r$mtest, adjustp = p1)
    }
  }
  r$G <- G
  r$R <- R
  if (!show) return(r)
  if (is.na(mtest)) {
    cat("\n", sprintf("%22s =%12.2f", paste0("chi2(", m, ")"), chi2), "\n", sep = "")
    cat(sprintf("%22s =%14.4f", "Prob > chi2", r$p), "\n", sep = "")
  } else {
    cat("\n--------------------------------------\n       |        chi2     df   p > chi2\n-------+------------------------------\n")
    for (i in seq_len(m)) cat(sprintf("  %-5s|%12.2f%7d%11.4f*", paste0("(", i, ")"), c1[i], 1L, p1[i]), "\n", sep = "")
    cat("-------+------------------------------\n")
    cat(sprintf("  All  |%12.2f%7d%11.4f", chi2, m, r$p), "\n", sep = "")
    cat("--------------------------------------\n")
    cat(if (mtest == "bonferroni") "* Bonferroni-adjusted p-values\n" else "* Unadjusted p-values\n")
  }
  invisible(r)
}"##;

/// The r() nlcom leaves (Stata 19), with the matrices r(b), r(V), r(table).
const R_NLCOM: &[&str] = &["level", "df_r", "N"];

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
        let est = self.tested_estimates("nlcom")?;
        // iterate() bounds Stata's search for the derivatives' steps; the results are the
        // same (Stata 19), so it is taken and left aside
        if let Some(o) = opts.other(&[
            ("level", 1),
            ("noheader", 8),
            ("df", 2),
            ("iterate", 7),
            ("post", 4),
        ]) {
            return Err(no(format!(
                "nlcom with the option {o} is not translated yet"
            )));
        }
        let df = match opts.arg("df", 2) {
            None => None,
            Some(d) => {
                let v: f64 = d
                    .trim()
                    .parse()
                    .map_err(|_| no(format!("df({d}) needs a number")))?;
                if !(v > 0.0 && v.is_finite()) {
                    return Err(no(format!("df({d}) must be above 0 (Stata: r(198))")));
                }
                Some(r_number(v))
            }
        };
        // the single-equation models whose nlcom table Stata 19 was seen to print (the
        // dependent variable in the stub, _t after stcox), and nlcom's own results posted
        if !self.after_nlcom_post()
            && !self.xtregar_active()
            && !matches!(
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
                    | Fit::Mlogit
            )
        {
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
        for h in ["stata_nlcom", "stata_fmt", "stata_returned", "stata_tables"] {
            self.helpers.insert(h);
        }
        out.line(format!(
            "stata_r <- stata_nlcom(list({}), c({}), c({}), {}, level = {level}{}{}{})",
            fns.join(", "),
            names.join(", "),
            texts.join(", "),
            if self.xtregar_active() {
                "stata_e[[\"depvar\"]]".to_string()
            } else {
                r_text(&est.dep)
            },
            df.map(|d| format!(", df = {d}")).unwrap_or_default(),
            if opts.has("noheader", 8) {
                ", header = FALSE"
            } else {
                ""
            },
            if self.quiet(c) { ", show = FALSE" } else { "" }
        ));
        if opts.has("post", 4) {
            // the combinations become e(b) and e(V), named as Stata names them; test,
            // lincom and nlcom then use z (Stata 19's e() after nlcom, post)
            out.line("stata_b <- stats::setNames(c(stata_r$b), colnames(stata_r$b))");
            out.line("stata_vcov <- stata_r$V");
            out.line("stata_df <- Inf");
            out.line("stata_estimates <- list()");
            out.line(format!(
                "stata_e <- list(N = stata_r$N, rank = qr(stata_r$V)$rank, predict = \"nlcom_p\", cmd = \"nlcom\", properties = \"b V\", depvar = {}, b = matrix(stata_b, 1, dimnames = list(\"y1\", names(stata_b))), V = stata_r$V)",
                r_text(&est.dep)
            ));
            self.estimates = Some(super::model::Estimates {
                fit: Fit::Margins,
                dep: est.dep.clone(),
                factors: est.factors.clone(),
            });
            self.e_new = true;
            for t in ["predict", "properties", "cmd", "depvar"] {
                self.e_text.insert(t.to_string());
            }
            self.e_ret = super::returned::Returned::Kept {
                cmd: "nlcom".into(),
                model: String::new(),
                names: &[],
            };
            self.r_ret = super::returned::Returned::Lost(
                "r() after nlcom, post was not checked against Stata".into(),
            );
            out.caveat("post: the combinations are now e(b) and e(V), named as Stata names them");
            return Ok(());
        }
        self.r_new = Some((R_NLCOM, "nlcom".into()));
        Ok(())
    }

    /// `testnl exp = exp` and `testnl (exp = exp) (exp = exp) ... [, mtest[(opt)] iterate(#)]`.
    pub(super) fn testnl(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "testnl")?;
        let est = self.tested_estimates("testnl")?;
        // an option Stata does not take: its error, r(198); iterate() bounds Stata's
        // search for the derivatives' steps, taken and left aside as for nlcom
        if let Some(o) = opts
            .0
            .iter()
            .find(|o| !o.is("mtest", 2) && !o.is("iterate", 4))
        {
            let paren = if o.arg.is_some() { "()" } else { "" };
            out.line(format!(
                "stop({}, call. = FALSE)",
                r_text(&format!(
                    "option {}{paren} not allowed (Stata: r(198))",
                    o.name
                ))
            ));
            return Ok(());
        }
        let mtest = match opts.raw("mtest", 2).map(|m| m.trim().to_string()) {
            _ if !opts.has("mtest", 2) => "NA",
            None => "\"noadjust\"",
            Some(m) if m.is_empty() || "noadjust".starts_with(m.as_str()) => "\"noadjust\"",
            Some(m) if !m.is_empty() && "bonferroni".starts_with(m.as_str()) => "\"bonferroni\"",
            Some(m) => {
                return Err(no(format!(
                    "testnl, mtest({m}) is not translated yet: no adjustment and bonferroni are"
                )))
            }
        };
        if !self.after_nlcom_post()
            && !self.xtregar_active()
            && !matches!(
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
                    | Fit::Mlogit
            )
        {
            return Err(no(format!(
                "testnl after {} is not translated yet",
                est.fit.name()
            )));
        }
        let list = parts(&c.parts.main)?;
        if list.is_empty() {
            return Err(no("testnl needs an equation"));
        }
        let mut fns = Vec::new();
        let mut texts = Vec::new();
        for (i, (name, e)) in list.iter().enumerate() {
            // a `name:` part is part of the equation's text here
            let text = match name {
                Some(n) => format!("{n}:{e}"),
                None => e.clone(),
            };
            if ["<=", ">=", "!=", "~="].iter().any(|op| text.contains(op)) {
                return Err(no(format!(
                    "testnl {text}: an equation with one = is translated"
                )));
            }
            let pieces: Vec<&str> = text.split('=').collect();
            let [lhs, rhs] = pieces.as_slice() else {
                if pieces.len() == 1 {
                    out.line(format!(
                        "stop({}, call. = FALSE)",
                        r_text(&format!(
                            "equation ({}) has no = sign (Stata: r(198))",
                            i + 1
                        ))
                    ));
                    return Ok(());
                }
                return Err(no(format!(
                    "testnl {text}: an equation with one = is translated"
                )));
            };
            if !text.contains("_b[") {
                return Err(no(format!(
                    "testnl {text}: an equation of the coefficients (_b[x]) is translated"
                )));
            }
            let (code, kind) =
                self.display_value(&format!("({}) - ({})", lhs.trim(), rhs.trim()), out)?;
            if kind == Kind::Text {
                return Err(no(format!(
                    "testnl {text}: a string (Stata: type mismatch)"
                )));
            }
            if code.contains("stata_v = ") {
                return Err(no(format!(
                    "testnl {text}: an expression of the data's variables is not translated"
                )));
            }
            fns.push(format!("function() {code}"));
            texts.push(r_text(text.trim()));
        }
        for h in ["stata_testnl", "stata_fmt", "stata_returned", "stata_names"] {
            self.helpers.insert(h);
        }
        out.line(format!(
            "stata_r <- stata_testnl(list({}), c({}), mtest = {mtest}{})",
            fns.join(", "),
            texts.join(", "),
            if self.quiet(c) { ", show = FALSE" } else { "" }
        ));
        self.r_new = Some((R_TESTNL, "testnl".into()));
        Ok(())
    }
}

/// The r() testnl leaves (Stata 19).
const R_TESTNL: &[&str] = &["p", "chi2", "df", "mtmethod", "mtest", "G", "R"];

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
