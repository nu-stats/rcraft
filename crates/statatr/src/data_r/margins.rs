//! P2B.4, `margins` after `regress`, `logit`, `probit`, `poisson` and
//! `nbreg`.
//!
//! Stata's `margins` averages a prediction over the estimation sample after
//! setting some variables to chosen values, and gives the standard error by
//! the delta method. The translation does the same in R, from the model
//! matrix, so the answers are Stata's and not those of an R package that
//! averages differently:
//!
//! - the rows are the estimation sample as it was when the model was fitted
//!   (`stata_data`, Stata's `e(sample)`); a command between the model and
//!   `margins` that may have changed the data makes `margins` refuse, since
//!   Stata would use the changed values;
//! - the prediction is the model's default one (`xb` after regress, `pr`
//!   after logit and probit, `n` after poisson and nbreg, so the exposure is
//!   in it), or the one `predict()` names;
//! - `dydx(x)` of a continuous variable is the average of the derivative of
//!   the prediction, worked out through the model matrix, so `c.x##c.x` and
//!   interactions are right; of a factor variable it is the average change
//!   from the base level to each other level, as Stata's is;
//! - `margins g` and `at()` set the variable for every row and average;
//!   `atmeans` sets the covariates to their means first;
//! - the standard errors are the delta method's with the model's own
//!   variance (robust or clustered when the model was), and the tests are t
//!   tests where the model has residual degrees of freedom, as Stata's are.

use super::model::Fit;
use super::stats::r_logical;
use super::*;

/// Predictive margins and average marginal effects with delta-method errors.
pub(super) const MARGINS_HELPER: &str = r#"stata_margins <- function(fit, data, b, V, df = Inf, type = "response", over = character(),
                          at = list(), dydx = character(), factors = character(), atmeans = FALSE,
                          level = 0.95, expression = "") {
  plain <- function(v) if (inherits(v, "haven_labelled")) as.vector(unclass(v)) else v
  data <- as.data.frame(lapply(data, plain), check.names = FALSE)
  tt <- stats::delete.response(stats::terms(fit))
  fam <- if (inherits(fit, "glm")) fit$family else stats::gaussian()
  cf <- names(stats::coef(fit))
  b <- b[cf]
  V <- V[cf, cf, drop = FALSE]
  design <- function(nd) {
    mf <- stats::model.frame(tt, nd, xlev = fit$xlevels, na.action = stats::na.pass)
    X <- stats::model.matrix(tt, mf, contrasts.arg = fit$contrasts)
    off <- stats::model.offset(mf)
    list(X = X[, cf, drop = FALSE], off = if (is.null(off)) 0 else off)
  }
  pred <- function(d, bb) {
    eta <- drop(d$X %*% bb) + d$off
    if (type == "link") eta else fam$linkinv(eta)
  }
  slope <- function(d, dX, bb) {
    s <- drop(dX %*% bb)
    if (type == "link") s else fam$mu.eta(drop(d$X %*% bb) + d$off) * s
  }
  vars <- all.vars(tt)
  offs <- attr(tt, "offset")
  if (length(offs) > 0) {
    used <- unlist(lapply(offs, function(i) all.vars(attr(tt, "variables")[[i + 1]])))
    vars <- setdiff(vars, used)
  }
  if (identical(dydx, "*")) dydx <- vars
  for (v in dydx) {
    if (!v %in% vars) stop("invalid dydx() option;\n", v, " not found in list of covariates")
  }
  for (v in over) {
    if (!v %in% vars) stop(v, " not found in list of covariates")
  }
  # Stata sets a variable to an at() value or a mean as a float, which moves
  # the margin in about the eighth digit (checked in Stata 19)
  fl <- function(x) readBin(writeBin(as.numeric(x), raw(), size = 4), "double", size = 4, n = length(x))
  if (atmeans) for (v in vars) data[[v]] <- fl(mean(data[[v]]))
  levels_of <- function(v) sort(unique(data[[v]]))
  setv <- function(vals) {
    nd <- data
    for (v in names(vals)) nd[[v]] <- fl(vals[[v]])
    nd
  }
  grids <- if (length(at) > 0) at else list(list())
  scen <- list()
  for (a in seq_along(grids)) {
    combos <- c(lapply(stats::setNames(over, over), levels_of), grids[[a]])
    grid <- if (length(combos) > 0) expand.grid(combos, KEEP.OUT.ATTRS = FALSE) else data.frame(row.names = 1)
    for (r in seq_len(nrow(grid))) {
      vals <- as.list(grid[r, , drop = FALSE])
      label <- if (length(vals) > 0) paste(paste0(names(vals), "=", unlist(vals)), collapse = ", ") else ""
      scen[[length(scen) + 1]] <- list(vals = vals, label = label)
    }
  }
  items <- list()
  labels <- character()
  add <- function(f, label) {
    items[[length(items) + 1]] <<- f
    labels <<- c(labels, label)
  }
  within <- function(label, s) if (nzchar(s$label)) paste0(label, " (", s$label, ")") else label
  for (s in scen) {
    nd <- setv(s$vals)
    if (length(dydx) == 0) {
      local({
        d <- design(nd)
        add(function(bb) mean(pred(d, bb)), if (nzchar(s$label)) s$label else "_cons")
      })
    }
    for (v in dydx) {
      if (v %in% names(factors)) {
        lv <- levels_of(v)
        base <- if (is.na(factors[[v]])) lv[1] else as.numeric(factors[[v]])
        d0 <- design(setv(c(s$vals, stats::setNames(list(base), v))))
        for (l in setdiff(lv, base)) {
          local({
            d1 <- design(setv(c(s$vals, stats::setNames(list(l), v))))
            add(function(bb) mean(pred(d1, bb) - pred(d0, bb)), within(paste0(l, ".", v), s))
          })
        }
      } else {
        local({
          x <- nd[[v]]
          h <- 1e-4 * (mean(abs(x)) + 1e-4)
          up <- nd
          up[[v]] <- x + h
          dn <- nd
          dn[[v]] <- x - h
          dX <- (design(up)$X - design(dn)$X) / (2 * h)
          d <- design(nd)
          add(function(bb) mean(slope(d, dX, bb)), within(v, s))
        })
      }
    }
  }
  m <- vapply(items, function(f) f(b), 0)
  J <- t(vapply(items, function(f) vapply(seq_along(b), function(j) {
    h <- 1e-5 * (abs(b[[j]]) + 1e-3)
    bp <- b
    bm <- b
    bp[j] <- b[j] + h
    bm[j] <- b[j] - h
    (f(bp) - f(bm)) / (2 * h)
  }, 0), numeric(length(b))))
  Vm <- J %*% V %*% t(J)
  names(m) <- labels
  dimnames(Vm) <- list(labels, labels)
  cat(if (length(dydx) > 0) "Average marginal effects\n" else "Predictive margins\n")
  cat(sprintf("Number of obs = %d\n", nrow(data)))
  cat("Expression:", expression, "\n")
  if (length(dydx) > 0) cat("dy/dx wrt:", paste(dydx, collapse = " "), "\n")
  if (atmeans) cat("at: the covariates at their means\n")
  cat("\n")
  tab <- stata_coefs(m, Vm, df, level, label = if (length(dydx) > 0) "dy/dx" else "Margin")
  invisible(list(b = m, V = Vm, table = tab))
}"#;

impl Session {
    pub(super) fn margins(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "margins")?;
        let est = self.estimates.clone().ok_or_else(|| {
            no("margins needs a model fitted earlier in this do-file (regress, logit, ...)")
        })?;
        let name = est.fit.name();
        if self.changed_since_fit {
            return Err(no(format!(
                "margins after a command that may have changed the data since {name}: Stata \
                 would average the changed rows, which is not translated yet"
            )));
        }
        if matches!(est.fit, Fit::Xtreg | Fit::Ivregress) {
            return Err(no(format!("margins after {name} is not translated yet")));
        }
        if !est.fit.one_outcome() {
            return Err(no(format!(
                "margins after {name} is not translated yet (it has a margin per outcome)"
            )));
        }
        if let Some(o) = opts.other(&[
            ("dydx", 4),
            ("at", 2),
            ("atmeans", 5),
            ("predict", 7),
            ("level", 3),
        ]) {
            return Err(no(format!(
                "margins with the option {o} is not translated yet"
            )));
        }
        // the marginlist: factor variables of the model
        let mut over = Vec::new();
        for w in c.parts.main.split_whitespace() {
            if w.contains('#') {
                return Err(no(format!(
                    "margins {w}: interactions in the marginlist are not translated yet"
                )));
            }
            let v = w.strip_prefix("i.").unwrap_or(w);
            if !est.factors.contains_key(v) {
                return Err(no(format!(
                    "margins {w}: {v} is not in the model as a factor variable (i.{v}), as Stata requires"
                )));
            }
            over.push(r_text(v));
        }
        let dydx = match opts.arg("dydx", 4) {
            None => "character()".to_string(),
            Some(list) => {
                let list = list.trim();
                if list == "*" || list == "_all" {
                    "\"*\"".to_string()
                } else {
                    let mut vs = Vec::new();
                    for w in list.split_whitespace() {
                        let v = w
                            .strip_prefix("c.")
                            .or_else(|| w.strip_prefix("i."))
                            .unwrap_or(w);
                        if !is_name(v) {
                            return Err(no(format!(
                                "dydx({list}): only variable names, * and _all are translated"
                            )));
                        }
                        vs.push(r_text(v));
                    }
                    if vs.is_empty() {
                        return Err(no("dydx() needs a variable"));
                    }
                    format!("c({})", vs.join(", "))
                }
            }
        };
        let mut ats = Vec::new();
        for o in c.options.iter().filter(|o| o.is("at", 2)) {
            ats.push(at_set(&o.arg())?);
        }
        let atmeans = opts.has("atmeans", 5);
        if atmeans && !est.factors.is_empty() {
            return Err(no(
                "margins, atmeans with factor variables is not translated yet (Stata uses the \
                 means of their indicators)",
            ));
        }
        let (ty, expression) = match (opts.arg("predict", 7).as_deref().map(str::trim), est.fit) {
            (Some("xb"), _) => ("link", "Linear prediction, predict(xb)".to_string()),
            (None, Fit::Regress) => ("response", "Linear prediction, predict()".to_string()),
            (None | Some("pr"), Fit::Logit | Fit::Probit) => {
                ("response", format!("Pr({}), predict()", est.dep))
            }
            (None | Some("n"), Fit::Poisson | Fit::Nbreg) => (
                "response",
                "Predicted number of events, predict()".to_string(),
            ),
            (Some(p), _) => {
                return Err(no(format!(
                    "margins, predict({p}) after {name} is not translated yet"
                )))
            }
            (None, _) => return Err(no(format!("margins after {name} is not translated yet"))),
        };
        let level = super::model::level(opts)?;
        let data = match self.condition(c, out)? {
            Some(cond) => {
                out.package("dplyr");
                format!("dplyr::filter(stata_data, {cond})")
            }
            None => "stata_data".to_string(),
        };
        let factors: Vec<String> = est
            .factors
            .iter()
            .map(|(v, expr)| {
                let base = factor_base(expr).map_or("NA_character_".to_string(), |b| r_text(&b));
                format!("{} = {base}", r_text(v))
            })
            .collect();
        let factors = if factors.is_empty() {
            "character()".to_string()
        } else {
            format!("c({})", factors.join(", "))
        };
        let over = if over.is_empty() {
            "character()".to_string()
        } else {
            format!("c({})", over.join(", "))
        };
        let at = if ats.is_empty() {
            "list()".to_string()
        } else {
            format!("list({})", ats.join(", "))
        };
        self.helpers.insert("stata_coefs");
        self.helpers.insert("stata_margins");
        out.line(format!(
            "stata_margins(stata_estimates, {data}, stata_b, stata_vcov, stata_df, type = \"{ty}\", over = {over}, at = {at}, dydx = {dydx}, factors = {factors}, atmeans = {}, level = {}, expression = {})",
            r_logical(atmeans),
            r_number(level),
            r_text(&expression)
        ));
        out.caveat("averaged over the estimation sample, Stata's e(sample)");
        out.caveat("delta-method standard errors from the model's own variance, as Stata's");
        Ok(())
    }
}

/// The base level of `stata_ib(g, "2")`; none for `factor(g)`, whose base
/// is the lowest value.
fn factor_base(expr: &str) -> Option<String> {
    let rest = expr.strip_prefix("stata_ib(")?.split_once(", \"")?.1;
    rest.split_once('"').map(|(b, _)| b.to_string())
}

/// One `at()`: `x=1 w=(1 2 3) z=(0(0.5)2)` as R's `list("x" = 1, ...)`.
fn at_set(text: &str) -> Result<String, Untranslated> {
    let mut parts = Vec::new();
    let mut rest = text.trim();
    while !rest.is_empty() {
        if rest.starts_with('(') {
            return Err(no(format!(
                "at({text}): statistics like (mean) are not translated yet"
            )));
        }
        let (name, after) = rest
            .split_once('=')
            .ok_or_else(|| no(format!("at({text}) needs name=value")))?;
        let name = name.trim();
        if !is_name(name) {
            return Err(no(format!("at({text}): {name} is not a variable name")));
        }
        let after = after.trim_start();
        let (values, next) = if let Some(inner) = after.strip_prefix('(') {
            // the ) that closes this (, past any a(d)b inside it
            let mut depth = 1usize;
            let close = inner.char_indices().find_map(|(i, ch)| {
                match ch {
                    '(' => depth += 1,
                    ')' => depth -= 1,
                    _ => {}
                }
                (depth == 0).then_some(i)
            });
            let close = close.ok_or_else(|| no(format!("at({text}): a ( without its )")))?;
            let (list, next) = inner.split_at(close);
            (numlist(list)?, next.get(1..).unwrap_or(""))
        } else {
            let end = after.find(char::is_whitespace).unwrap_or(after.len());
            let (v, next) = after.split_at(end);
            (numlist(v)?, next)
        };
        let values: Vec<String> = values.into_iter().map(r_number).collect();
        let values = if values.len() == 1 {
            values.join("")
        } else {
            format!("c({})", values.join(", "))
        };
        parts.push(format!("{} = {values}", r_text(name)));
        rest = next.trim_start();
    }
    if parts.is_empty() {
        return Err(no("at() needs name=value"));
    }
    Ok(format!("list({})", parts.join(", ")))
}

/// A Stata numlist: numbers, `a/b` (whole steps) and `a(d)b`.
fn numlist(text: &str) -> Result<Vec<f64>, Untranslated> {
    let bad = || no(format!("{text} is not a list of numbers"));
    let mut out = Vec::new();
    for tok in text.split_whitespace() {
        let range = |a: f64, d: f64, b: f64| -> Result<Vec<f64>, Untranslated> {
            if d == 0.0 || (b - a) / d < 0.0 || (b - a) / d > 10_000.0 {
                return Err(bad());
            }
            let n = ((b - a) / d + 1e-9).floor() as usize;
            Ok((0..=n).map(|i| a + d * i as f64).collect())
        };
        if let Some((a, rest)) = tok.split_once('(') {
            let (d, b) = rest.split_once(')').ok_or_else(bad)?;
            let (a, d, b) = (
                a.parse::<f64>().map_err(|_| bad())?,
                d.parse::<f64>().map_err(|_| bad())?,
                b.parse::<f64>().map_err(|_| bad())?,
            );
            out.extend(range(a, d, b)?);
        } else if let Some((a, b)) = tok.split_once('/') {
            let (a, b) = (
                a.parse::<f64>().map_err(|_| bad())?,
                b.parse::<f64>().map_err(|_| bad())?,
            );
            out.extend(range(a, 1.0, b)?);
        } else {
            out.push(tok.parse::<f64>().map_err(|_| bad())?);
        }
    }
    if out.is_empty() || out.iter().any(|v| !v.is_finite()) {
        return Err(bad());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numlists_are_read_as_stata_reads_them() {
        assert_eq!(numlist("1 2 3"), Ok(vec![1.0, 2.0, 3.0]));
        assert_eq!(numlist("1/3"), Ok(vec![1.0, 2.0, 3.0]));
        assert_eq!(numlist("0(0.5)1.5"), Ok(vec![0.0, 0.5, 1.0, 1.5]));
        assert!(numlist("a").is_err());
        assert!(numlist("3/1").is_err());
    }

    #[test]
    fn an_at_becomes_rs_list() {
        assert_eq!(
            at_set("x=(1 2) w = 0").as_deref(),
            Ok("list(\"x\" = c(1, 2), \"w\" = 0)")
        );
        assert_eq!(
            at_set("x=(0(1)2)").as_deref(),
            Ok("list(\"x\" = c(0, 1, 2))")
        );
        assert!(at_set("(mean) x").is_err(), "statistics are not translated");
        assert!(at_set("x").is_err());
    }

    #[test]
    fn the_base_level_is_read_back_from_the_r() {
        assert_eq!(factor_base("stata_ib(g, \"2\")").as_deref(), Some("2"));
        assert_eq!(factor_base("factor(g)"), None);
    }
}
