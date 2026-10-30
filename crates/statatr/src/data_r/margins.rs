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

use super::model::{Estimates, Fit};
use super::stats::r_logical;
use super::*;

/// Predictive margins and average marginal effects with delta-method errors.
pub(super) const MARGINS_HELPER: &str = r##"stata_margins <- function(fit, data, b, V, df = Inf, type = "response", terms = list(),
                          within = character(), at = list(), dydx = character(), eyex = character(),
                          dyex = character(), eydx = character(), factors = character(),
                          atmeans = FALSE, level = 0.95, expression = "", outcomes = NULL,
                          compare = "") {
  plain <- function(v) if (inherits(v, "haven_labelled")) as.vector(unclass(v)) else v
  data <- as.data.frame(lapply(data, plain), check.names = FALSE)
  tt <- stats::delete.response(stats::terms(fit))
  kind <- if (inherits(fit, "polr")) "polr" else if (inherits(fit, "multinom")) "multinom" else "glm"
  fam <- if (inherits(fit, "glm")) fit$family else stats::gaussian()
  xnames <- if (kind == "multinom") colnames(stats::model.matrix(fit)) else names(stats::coef(fit))
  if (kind == "glm") {
    b <- b[xnames]
    V <- V[xnames, xnames, drop = FALSE]
  }
  # Stata sets a variable to an at() value or a mean as a float, which moves
  # the margin in about the eighth digit (checked in Stata 19)
  fl <- function(x) readBin(writeBin(as.numeric(x), raw(), size = 4), "double", size = 4, n = length(x))
  # atmeans: each variable at its mean, a factor's indicators at theirs, and
  # every column of the design the product of its parts
  part_mean <- function(p, nd, sv = NULL, h = 0) {
    if (p == "(Intercept)") return(1)
    m <- regmatches(p, regexec("^(factor|stata_ib)\\(([^,()]+)(, \"[^\"]*\")?\\)(.+)$", p))[[1]]
    if (length(m)) return(fl(mean(as.character(nd[[m[3]]]) == m[5])))
    m <- regmatches(p, regexec("^I\\(([^()^]+)\\^([0-9]+)\\)$", p))[[1]]
    if (length(m)) return((fl(mean(nd[[m[2]]])) + if (identical(m[2], sv)) h else 0)^as.integer(m[3]))
    fl(mean(nd[[p]])) + if (identical(p, sv)) h else 0
  }
  # sv, h: a variable moved by h for a derivative (after the float at means)
  design <- function(nd, sv = NULL, h = 0) {
    if (!atmeans && !is.null(sv)) nd[[sv]] <- nd[[sv]] + h
    mf <- stats::model.frame(tt, nd, xlev = fit$xlevels, na.action = stats::na.pass)
    X <- stats::model.matrix(tt, mf, contrasts.arg = fit$contrasts)[, xnames, drop = FALSE]
    off <- stats::model.offset(mf)
    if (is.null(off)) off <- 0
    if (atmeans) {
      row <- vapply(colnames(X), function(cn) prod(vapply(strsplit(cn, ":", fixed = TRUE)[[1]], part_mean, 0, nd = nd, sv = sv, h = h)), 0)
      X <- matrix(row, 1, dimnames = list(NULL, colnames(X)))
      off <- mean(off)
    }
    list(X = X, off = off)
  }
  # the probability of each outcome, in the order of the outcome values
  probs <- function(d, bb) {
    n <- nrow(d$X)
    if (kind == "polr") {
      eta <- drop(d$X %*% bb[xnames])
      z <- c(-Inf, bb[paste0("/cut", seq_along(fit$zeta))], Inf)
      F <- if (fit$method == "probit") stats::pnorm else stats::plogis
      return(matrix(vapply(seq_len(length(z) - 1), function(k) F(z[k + 1] - eta) - F(z[k] - eta), numeric(n)), n))
    }
    lev <- fit$lev
    E <- matrix(vapply(lev, function(l) {
      if (l == lev[1]) numeric(n) else drop(d$X %*% bb[paste0(l, ":", xnames)])
    }, numeric(n)), n)
    P <- exp(E) / rowSums(exp(E))
    P[, order(as.numeric(lev)), drop = FALSE]
  }
  pred <- function(d, bb, o = NULL) {
    if (!is.null(o)) return(probs(d, bb)[, o])
    eta <- drop(d$X %*% bb) + d$off
    if (type == "link") eta else fam$linkinv(eta)
  }
  vars <- all.vars(tt)
  offs <- attr(tt, "offset")
  if (length(offs) > 0) {
    used <- unlist(lapply(offs, function(i) all.vars(attr(tt, "variables")[[i + 1]])))
    vars <- setdiff(vars, used)
  }
  if (identical(dydx, "*")) dydx <- vars
  for (v in c(dydx, eyex, dyex, eydx)) {
    if (!v %in% vars) stop("invalid dydx() option;\n", v, " not found in list of covariates")
  }
  for (v in unlist(terms)) {
    if (!v %in% vars) stop(v, " not found in list of covariates")
  }
  levels_of <- function(v) sort(unique(data[[v]]))
  setv <- function(nd, vals) {
    for (v in names(vals)) nd[[v]] <- fl(vals[[v]])
    nd
  }
  # at(): a statistic of the estimation sample, or the values given
  stat <- function(s, v) {
    x <- sort(data[[v]][!is.na(data[[v]])])
    switch(s,
      mean = mean(x), median = , p50 = unname(stats::quantile(x, 0.5, type = 2)),
      min = min(x), max = max(x),
      unname(stats::quantile(x, as.numeric(sub("^p", "", s)) / 100, type = 2)))
  }
  ats <- list()
  for (a in at) {
    a <- lapply(stats::setNames(names(a), names(a)), function(v) {
      x <- a[[v]]
      if (is.character(x)) stat(x, v) else x
    })
    g <- expand.grid(a, KEEP.OUT.ATTRS = FALSE)
    for (r in seq_len(nrow(g))) ats[[length(ats) + 1]] <- as.list(g[r, , drop = FALSE])
  }
  at_names <- if (length(ats) > 0) paste0(seq_along(ats), "._at") else ""
  if (length(ats) == 0) ats <- list(list())
  # the marginlist: each term's cells, a term with # setting its variables together
  cells <- list(list(vals = list(), name = ""))
  if (length(terms) > 0) {
    cells <- list()
    for (tm in terms) {
      g <- expand.grid(rev(lapply(stats::setNames(tm, tm), levels_of)), KEEP.OUT.ATTRS = FALSE)[tm]
      for (r in seq_len(nrow(g))) {
        vals <- as.list(g[r, , drop = FALSE])
        cells[[length(cells) + 1]] <- list(vals = vals, name = paste(paste0(unlist(vals), ".", names(vals)), collapse = "#"))
      }
    }
  }
  # over(): each group of the estimation sample on its own
  groups <- list(list(rows = rep(TRUE, nrow(data)), name = ""))
  if (length(within) > 0) {
    g <- unique(data[within])
    g <- g[do.call(order, unname(as.list(g))), , drop = FALSE]
    groups <- lapply(seq_len(nrow(g)), function(r) {
      rows <- Reduce(`&`, lapply(within, function(w) data[[w]] == g[r, w]))
      list(rows = rows, name = paste(paste0(unlist(g[r, ]), ".", within), collapse = "#"))
    })
  }
  outs <- if (is.null(outcomes)) list(NULL) else as.list(outcomes)
  items <- list()
  labels <- character()
  add <- function(f, label) {
    items[[length(items) + 1]] <<- f
    labels <<- c(labels, label)
  }
  join <- function(...) {
    p <- c(...)
    paste(p[nzchar(p)], collapse = "#")
  }
  effects <- list(dydx = dydx, eyex = eyex, dyex = dyex, eydx = eydx)
  any_effect <- length(unlist(effects)) > 0
  for (o in outs) {
    oname <- if (is.null(o)) "" else paste0(o, "._predict")
    for (ai in seq_along(ats)) {
      for (cl in cells) {
        for (gr in groups) {
          nd <- setv(setv(data[gr$rows, , drop = FALSE], ats[[ai]]), cl$vals)
          where <- join(cl$name, at_names[min(ai, length(at_names))], gr$name)
          if (!any_effect) {
            local({
              d <- design(nd)
              oo <- o
              j <- join(oname, where)
              add(function(bb) mean(pred(d, bb, oo)), if (nzchar(j)) j else "_cons")
            })
          }
          for (e in names(effects)) {
            for (v in effects[[e]]) {
              lab <- if (nzchar(join(oname, where))) paste0(v, ":", join(oname, where)) else v
              if (v %in% names(factors)) {
                if (e != "dydx") stop(e, "(): ", v, " is a factor variable, which Stata does not allow here")
                lv <- levels_of(v)
                base <- if (is.na(factors[[v]])) lv[1] else as.numeric(factors[[v]])
                local({
                  d0 <- design(setv(nd, stats::setNames(list(base), v)))
                  for (l in setdiff(lv, base)) {
                    local({
                      d1 <- design(setv(nd, stats::setNames(list(l), v)))
                      oo <- o
                      lab1 <- if (nzchar(join(oname, where))) paste0(l, ".", v, ":", join(oname, where)) else paste0(l, ".", v)
                      add(function(bb) mean(pred(d1, bb, oo) - pred(d0, bb, oo)), lab1)
                    })
                  }
                })
              } else {
                local({
                  x <- nd[[v]]
                  h <- 1e-4 * (mean(abs(x)) + 1e-4)
                  du <- design(nd, v, h)
                  dd <- design(nd, v, -h)
                  d <- design(nd)
                  xv <- if (atmeans) fl(mean(x)) else x
                  oo <- o
                  ee <- e
                  dX <- (du$X - dd$X) / (2 * h)
                  add(function(bb) {
                    # one outcome: the chain rule through the design, exactly;
                    # several: the outcome probabilities differenced
                    s <- if (is.null(oo)) {
                      g <- drop(dX %*% bb)
                      if (type == "link") g else fam$mu.eta(drop(d$X %*% bb) + d$off) * g
                    } else {
                      (pred(du, bb, oo) - pred(dd, bb, oo)) / (2 * h)
                    }
                    y <- pred(d, bb, oo)
                    mean(switch(ee, dydx = s, eyex = s * xv / y, dyex = s * xv, eydx = s / y))
                  }, lab)
                })
              }
            }
          }
        }
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
  what <- if (length(dydx) > 0) "dy/dx" else if (length(eyex) > 0) "ey/ex" else if (length(dyex) > 0) "dy/ex" else if (length(eydx) > 0) "ey/dx" else "Margin"
  cat(if (any_effect) "Average marginal effects\n" else "Predictive margins\n")
  cat(sprintf("Number of obs = %d\n", nrow(data)))
  cat("Expression:", expression, "\n")
  if (any_effect) cat(what, "wrt:", paste(unlist(effects), collapse = " "), "\n")
  if (atmeans) cat("at: the covariates at their means\n")
  cat("\n")
  if (compare == "") {
    tab <- stata_coefs(m, Vm, df, level, label = what)
    return(invisible(list(b = m, V = Vm, table = tab)))
  }
  # contrasts of the margins of one term: each level against the first (r.),
  # every pair (pwcompare), or the joint test that they are all equal
  k <- length(m)
  if (k < 2) stop("a contrast needs at least two margins")
  pairs <- if (compare == "pwcompare") utils::combn(k, 2) else rbind(1, 2:k)
  L <- matrix(0, ncol(pairs), k)
  for (i in seq_len(ncol(pairs))) {
    L[i, pairs[2, i]] <- 1
    L[i, pairs[1, i]] <- -1
  }
  lv <- sub("\\..*$", "", labels)
  rownames(L) <- paste0(if (compare == "pwcompare") "" else "r", lv[pairs[2, ]], "vs", lv[pairs[1, ]], sub("^[^.]*", "", labels[1]))
  cc <- drop(L %*% m)
  Vc <- L %*% Vm %*% t(L)
  dimnames(Vc) <- list(rownames(L), rownames(L))
  if (compare == "contrast") {
    W <- drop(t(cc) %*% solve(Vc, cc))
    q <- length(cc)
    if (is.finite(df)) {
      F <- W / q
      p <- stats::pf(F, q, df, lower.tail = FALSE)
      cat(sprintf("Contrasts of predictive margins\n   df = %d   F = %.2f   P>F = %.4f   (denominator %g)\n", q, F, p, df))
      return(invisible(list(b = cc, V = Vc, F = F, df = q, df_r = df, p = p)))
    }
    p <- stats::pchisq(W, q, lower.tail = FALSE)
    cat(sprintf("Contrasts of predictive margins\n   df = %d   chi2 = %.2f   P>chi2 = %.4f\n", q, W, p))
    return(invisible(list(b = cc, V = Vc, chi2 = W, df = q, p = p)))
  }
  tab <- stata_coefs(cc, Vc, df, level, label = "Contrast")
  invisible(list(b = cc, V = Vc, table = tab))
}"##;

impl Session {
    pub(super) fn margins(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "margins")?;
        if self.weighted_fit {
            return Err(no(
                "margins after a model with weights is not translated yet",
            ));
        }
        let est = self.estimates.clone().ok_or_else(|| {
            no("margins needs a model fitted earlier in this do-file (regress, logit, ...)")
        })?;
        let name = est.fit.name();
        if est.fit == Fit::Ivregress {
            return Err(no(format!("margins after {name} is not translated yet")));
        }
        if est.fit == Fit::Margins {
            return Err(no("margins after margins, post is not translated yet"));
        }
        if let Some(o) = opts.other(&[
            ("dydx", 4),
            ("eyex", 4),
            ("dyex", 4),
            ("eydx", 4),
            ("at", 2),
            ("atmeans", 5),
            ("over", 4),
            ("predict", 7),
            ("level", 3),
            ("post", 4),
            ("pwcompare", 2),
            ("contrast", 4),
            ("vce", 3),
        ]) {
            return Err(no(format!(
                "margins with the option {o} is not translated yet"
            )));
        }
        if let Some(v) = opts.arg("vce", 3) {
            return Err(match v.trim() {
                "delta" => no("margins, vce(delta) is the default; leave it out"),
                "unconditional" => no(
                    "margins, vce(unconditional) is not translated yet (Stata allows it only \
                     after robust, semirobust, or linearized standard errors)",
                ),
                other => no(format!("margins, vce({other}) is not translated yet")),
            });
        }
        // the marginlist: factor variables of the model, alone or joined by #,
        // or one r.v (each level against the first)
        let mut terms = Vec::new();
        let mut compare = "";
        for w in c.parts.main.split_whitespace() {
            let w = match w.strip_prefix("r.") {
                Some(v) => {
                    compare = "r";
                    v
                }
                None => w,
            };
            let mut term = Vec::new();
            for part in w.split('#') {
                let v = part.strip_prefix("i.").unwrap_or(part);
                if !est.factors.contains_key(v) {
                    return Err(no(format!(
                        "margins {w}: {v} is not in the model as a factor variable (i.{v}), as Stata requires"
                    )));
                }
                term.push(r_text(v));
            }
            terms.push(format!("c({})", term.join(", ")));
        }
        if opts.has("pwcompare", 2) {
            if compare == "r" {
                return Err(no("margins: r. and pwcompare together are not translated"));
            }
            if let Some(a) = opts.arg("pwcompare", 2) {
                if !matches!(a.trim(), "" | "effects") {
                    return Err(no(format!("pwcompare({a}) is not translated yet")));
                }
            }
            compare = "pwcompare";
        }
        if opts.has("contrast", 4) {
            if opts
                .arg("contrast", 4)
                .is_some_and(|a| !a.trim().is_empty())
            {
                return Err(no(
                    "margins, contrast() with suboptions is not translated yet",
                ));
            }
            compare = "contrast";
        }
        if !compare.is_empty() && terms.len() != 1 {
            return Err(no(
                "margins: a contrast of the margins of one marginlist term is translated, not more",
            ));
        }
        let list = |o: &str, min: usize| -> Result<String, Untranslated> {
            match opts.arg(o, min) {
                None => Ok("character()".to_string()),
                Some(list) => {
                    let list = list.trim();
                    if o == "dydx" && (list == "*" || list == "_all") {
                        return Ok("\"*\"".to_string());
                    }
                    let mut vs = Vec::new();
                    for w in list.split_whitespace() {
                        let v = w
                            .strip_prefix("c.")
                            .or_else(|| w.strip_prefix("i."))
                            .unwrap_or(w);
                        if !is_name(v) {
                            return Err(no(format!(
                                "{o}({list}): only variable names are translated"
                            )));
                        }
                        vs.push(r_text(v));
                    }
                    if vs.is_empty() {
                        return Err(no(format!("{o}() needs a variable")));
                    }
                    Ok(format!("c({})", vs.join(", ")))
                }
            }
        };
        let (dydx, eyex, dyex, eydx) = (
            list("dydx", 4)?,
            list("eyex", 4)?,
            list("dyex", 4)?,
            list("eydx", 4)?,
        );
        let effects = [&dydx, &eyex, &dyex, &eydx]
            .iter()
            .filter(|e| e.as_str() != "character()")
            .count();
        if effects > 1 {
            return Err(no(
                "margins: one of dydx(), eyex(), dyex() and eydx() at a time",
            ));
        }
        if effects > 0 && !compare.is_empty() {
            return Err(no(
                "margins: contrasts of marginal effects are not translated yet",
            ));
        }
        let mut ats = Vec::new();
        for o in c.options.iter().filter(|o| o.is("at", 2)) {
            ats.push(at_set(&o.arg())?);
        }
        let within = match opts.arg("over", 4) {
            None => "character()".to_string(),
            Some(v) => {
                let vs: Vec<&str> = v.split_whitespace().collect();
                if vs.is_empty() || vs.iter().any(|w| !is_name(w)) {
                    return Err(no(format!("over({v}): only variable names are translated")));
                }
                let vs: Vec<String> = vs.iter().map(|w| r_text(w)).collect();
                format!("c({})", vs.join(", "))
            }
        };
        let atmeans = opts.has("atmeans", 5);
        let mut outcomes = "NULL".to_string();
        let (ty, expression) = match (opts.arg("predict", 7).as_deref().map(str::trim), est.fit) {
            (Some("xb"), f) if f.one_outcome() => {
                ("link", "Linear prediction, predict(xb)".to_string())
            }
            (None, Fit::Regress | Fit::Xtreg) => {
                ("response", "Linear prediction, predict()".to_string())
            }
            (None | Some("pr"), Fit::Logit | Fit::Probit) => {
                ("response", format!("Pr({}), predict()", est.dep))
            }
            (None | Some("n"), Fit::Poisson | Fit::Nbreg) => (
                "response",
                "Predicted number of events, predict()".to_string(),
            ),
            (None | Some("pr"), f) if !f.one_outcome() => {
                outcomes = "seq_along(stata_levels)".to_string();
                ("response", "Pr of each outcome, predict()".to_string())
            }
            (Some(p), f) if !f.one_outcome() => {
                let k = p
                    .strip_prefix("outcome(")
                    .and_then(|r| r.strip_suffix(')'))
                    .map(str::trim)
                    .ok_or_else(|| {
                        no(format!(
                            "margins, predict({p}) after {name} is not translated yet"
                        ))
                    })?;
                outcomes = match k.strip_prefix('#') {
                    Some(n) => match n.trim().parse::<usize>() {
                        Ok(n) if n >= 1 => format!("{n}"),
                        _ => return Err(no(format!("outcome({k}) is not an outcome number"))),
                    },
                    None if k.parse::<f64>().is_ok() => format!(
                        "match({}, as.character(sort(as.numeric(stata_levels))))",
                        r_text(k)
                    ),
                    None => {
                        return Err(no(format!(
                            "outcome({k}): only a value or #k is translated, not a label"
                        )))
                    }
                };
                ("response", format!("Pr({}=={k}), predict({p})", est.dep))
            }
            (Some(p), _) => {
                return Err(no(format!(
                    "margins, predict({p}) after {name} is not translated yet"
                )))
            }
            (None, _) => return Err(no(format!("margins after {name} is not translated yet"))),
        };
        let level = super::model::level(opts)?;
        // the estimation sample: as it was fitted, or, once the data may have
        // changed, the current values of its rows (what Stata averages)
        let rows = if !self.changed_since_fit {
            "stata_data".to_string()
        } else if self.esample {
            out.caveat(
                "the data changed since the model, so the current values of its rows are used, \
                 as Stata does",
            );
            format!("{}[stata_esample, , drop = FALSE]", self.data()?)
        } else {
            return Err(no(format!(
                "margins after a command that may have changed the rows since {name} (Stata \
                 would average the rows it then finds in e(sample)), which is not translated yet"
            )));
        };
        let data = match self.condition(c, out)? {
            Some(cond) => {
                out.package("dplyr");
                format!("dplyr::filter({rows}, {cond})")
            }
            None => rows,
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
        let terms = format!("list({})", terms.join(", "));
        let at = if ats.is_empty() {
            "list()".to_string()
        } else {
            format!("list({})", ats.join(", "))
        };
        self.helpers.insert("stata_coefs");
        self.helpers.insert("stata_margins");
        let call = format!(
            "stata_margins(stata_estimates, {data}, stata_b, stata_vcov, stata_df, type = \"{ty}\", terms = {terms}, within = {within}, at = {at}, dydx = {dydx}, eyex = {eyex}, dyex = {dyex}, eydx = {eydx}, factors = {factors}, atmeans = {}, level = {}, expression = {}, outcomes = {outcomes}, compare = {})",
            r_logical(atmeans),
            r_number(level),
            r_text(&expression),
            r_text(compare)
        );
        if opts.has("post", 4) {
            // the margins become e(b) and e(V), as for a model
            out.line(format!("stata_m <- {call}"));
            out.line("stata_b <- stata_m$b");
            out.line("stata_vcov <- stata_m$V");
            out.line("rm(stata_m)");
            self.estimates = Some(Estimates {
                fit: Fit::Margins,
                dep: est.dep.clone(),
                factors: est.factors.clone(),
            });
            out.caveat("post: the margins are now e(b) and e(V), named as Stata names them (1.g, x, 1._at)");
        } else {
            out.line(call);
        }
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
    // `(mean) x w`: a statistic of the estimation sample for the names after it
    let mut stat: Option<String> = None;
    while !rest.is_empty() {
        if let Some(inner) = rest.strip_prefix('(') {
            let (s, next) = inner
                .split_once(')')
                .ok_or_else(|| no(format!("at({text}): a ( without its )")))?;
            let s = s.trim();
            let known = matches!(s, "mean" | "median" | "min" | "max")
                || s.strip_prefix('p')
                    .and_then(|k| k.parse::<u32>().ok())
                    .is_some_and(|k| (1..=99).contains(&k));
            if !known {
                return Err(no(format!("at({text}): ({s}) is not translated yet")));
            }
            stat = Some(s.to_string());
            rest = next.trim_start();
            continue;
        }
        if let Some(s) = &stat {
            // names up to the next = or (
            let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
            let (word, next) = rest.split_at(end);
            if !word.contains('=') && !next.trim_start().starts_with('=') {
                if !is_name(word) {
                    return Err(no(format!("at({text}): {word} is not a variable name")));
                }
                parts.push(format!("{} = {}", r_text(word), r_text(s)));
                rest = next.trim_start();
                continue;
            }
            stat = None;
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
        assert_eq!(
            at_set("(mean) x w=0.5").as_deref(),
            Ok("list(\"x\" = \"mean\", \"w\" = 0.5)")
        );
        assert!(at_set("(asobserved) x").is_err(), "not translated yet");
        assert!(at_set("x").is_err());
    }

    #[test]
    fn the_base_level_is_read_back_from_the_r() {
        assert_eq!(factor_base("stata_ib(g, \"2\")").as_deref(), Some("2"));
        assert_eq!(factor_base("factor(g)"), None);
    }
}
