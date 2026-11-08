//! `suest` of conditional logits, and `margins` after it with an
//! `expression()` of the equations' predictions, as Stata 19 does them on
//! made-up data (`plan/stata/suest.md`).
//!
//! - e(b) is each model's own e(b) (base levels as zeros) one after another,
//!   its equation named `name_depvar` (`co_chosen`);
//! - e(V) = D (U'U) D G / (G - 1): D the models' own variances side by side,
//!   U the rows' scores (each model's on its rows, zero elsewhere) summed
//!   within the clusters of vce(cluster v), or else within clogit's groups;
//! - e(N) counts the rows of any of the models; e(rank) the coefficients
//!   estimated.
//! - margins, dydx(#.v ...) expression(invlogit(predict(equation(eq))) + ...):
//!   the mean over those rows of the discrete change from the base level, its
//!   variance by the delta method through e(V); with post, e(b) holds each
//!   level asked for and its base, as after any margins of factor levels.

use super::model::{Estimates, Fit};
use super::returned::Returned;
use super::stats::r_logical;
use super::*;

pub(super) const SUEST_HELPER: &str = r##"stata_suest <- function(names, data, cluster = NULL) {
  g <- globalenv()
  st <- get0("stata_stored", envir = g, ifnotfound = list())
  # a stored result's own names, read with it put in place of the model held
  swap <- function(s, f) {
    keys <- names(s)
    had <- vapply(keys, function(k) exists(k, envir = g, inherits = FALSE), NA)
    old <- lapply(keys, function(k) if (had[[k]]) get(k, envir = g) else NULL)
    names(old) <- keys
    on.exit({
      for (k in keys) {
        if (had[[k]]) assign(k, old[[k]], envir = g)
        else if (exists(k, envir = g, inherits = FALSE)) rm(list = k, envir = g)
      }
    })
    for (k in keys) assign(k, s[[k]], envir = g)
    f()
  }
  esym <- function(w, k) {
    a <- c(1, numeric(k))
    if (k > 0) for (v in w) a[2:(k + 1)] <- a[2:(k + 1)] + v * a[1:k]
    a[k + 1]
  }
  eqs <- list()
  bs <- list()
  Ds <- list()
  Ss <- list()
  rowsets <- list()
  for (n in names) {
    s <- st[[n]]
    if (is.null(s)) stop("estimation result ", n, " not found (Stata: r(111))", call. = FALSE)
    fit <- s$stata_estimates
    su <- fit$stata_suest
    if (is.null(su) || !is.null(fit$stata_weights) || !is.null(fit$stata_influence))
      stop("suest: ", n, " is not a model RCraft combines yet (clogit without weights or robust errors is)", call. = FALSE)
    depvar <- s$stata_e$depvar
    nm <- swap(s, function() {
      eb <- stata_eb(stata_b, stata_vcov, "clogit", depvar)
      list(full = sub("^[^:]*:", "", colnames(eb$b)), kept = stata_bnames()[match(su$cols, names(stata_b))])
    })
    pos <- match(nm$kept, nm$full)
    if (anyNA(pos)) stop("suest: the coefficients of ", n, " (", paste(nm$kept, collapse = " "), ") could not be matched to Stata's names (", paste(nm$full, collapse = " "), ")", call. = FALSE)
    k <- length(nm$full)
    eq <- paste0(n, "_", depvar)
    b <- stats::setNames(rep(0, k), paste0(eq, ":", nm$full))
    b[pos] <- su$b
    D <- matrix(0, k, k)
    D[pos, pos] <- su$Vinfo
    eta <- drop(su$X %*% su$b)
    S <- matrix(0, nrow(su$X), k)
    for (h in split(seq_along(su$gi), su$gi)) {
      w <- exp(eta[h] - max(eta[h]))
      kk <- sum(su$y[h])
      all <- esym(w, kk)
      p <- vapply(seq_along(h), function(j) w[j] * esym(w[-j], kk - 1) / all, 0)
      S[h, pos] <- (su$y[h] - p) * su$X[h, , drop = FALSE]
    }
    eqs[[n]] <- list(name = n, eq = eq, depvar = depvar, fit = fit, cols = su$cols, pos = pos, k = k)
    bs[[n]] <- b
    Ds[[n]] <- D
    Ss[[n]] <- S
    # the model's rows in the data in memory: its e(sample), in data order
    es <- s$stata_esample
    if (is.null(es) || length(es) != nrow(data) || sum(es) != length(su$rows))
      stop("suest: the data in memory are not the rows ", n, " was fitted on; not translated", call. = FALSE)
    rowsets[[n]] <- which(es)
  }
  rows <- sort(unique(unlist(rowsets)))
  K <- sum(vapply(bs, length, 0L))
  Sbig <- matrix(0, length(rows), K)
  D <- matrix(0, K, K)
  at <- 0
  for (n in names) {
    idx <- at + seq_along(bs[[n]])
    Sbig[match(rowsets[[n]], rows), idx] <- Ss[[n]]
    D[idx, idx] <- Ds[[n]]
    eqs[[n]]$at <- at
    at <- at + length(idx)
  }
  # without vce(cluster), each clogit group is a cluster, as clogit's own robust errors take them
  if (is.null(cluster)) {
    grp <- unique(vapply(eqs, function(e) e$fit$stata_group, ""))
    if (length(grp) != 1) stop("suest of clogits with different group() variables is not translated yet", call. = FALSE)
    cl <- data[[grp]][rows]
  } else {
    cl <- data[[cluster]][rows]
  }
  if (anyNA(cl)) stop("suest: missing values in the cluster variable are not translated yet", call. = FALSE)
  U <- rowsum(Sbig, cl)
  G <- nrow(U)
  V <- D %*% crossprod(U) %*% D * G / (G - 1)
  b <- unlist(unname(bs))
  dimnames(V) <- list(names(b), names(b))
  list(b = b, V = V, N = length(rows), G = G, rank = sum(vapply(eqs, function(e) length(e$pos), 0L)),
       rows = rows, eqs = eqs)
}
stata_suest_print <- function(m, names, clustvar = NULL) {
  cat(formatC(paste0("Simultaneous results for ", paste(names, collapse = ", ")), width = -56, flag = "-"),
      sprintf("Number of obs = %6s\n", format(m$N, big.mark = ",")), sep = "")
  if (!is.null(clustvar)) {
    cat("\n", formatC(sprintf("(Std. err. adjusted for %s clusters in %s)", format(m$G, big.mark = ","), clustvar), width = 78), "\n", sep = "")
  }
  cat("\n")
  keep <- diag(m$V) > 0
  invisible(stata_coefs(m$b[keep], m$V[keep, keep, drop = FALSE], Inf, 0.95))
}
stata_suest_margins <- function(m, data, levels, terms, bases = character(), level = 0.95, mcomp = "", expression = "", quiet = FALSE) {
  plain <- function(v) if (inherits(v, "haven_labelled")) as.vector(unclass(v)) else v
  if (max(m$rows) > nrow(data)) stop("margins after suest: the data in memory are not the data suest used; not translated", call. = FALSE)
  d <- data[m$rows, , drop = FALSE]
  d[] <- lapply(d, plain)
  eq_of <- function(q) {
    for (e in m$eqs) if (e$eq == q) return(e)
    stop("equation ", q, " not found (Stata: r(303))", call. = FALSE)
  }
  design <- function(e, nd) {
    fit <- e$fit
    mf <- stats::model.frame(fit$stata_terms, nd, xlev = fit$stata_xlevels, na.action = stats::na.pass)
    X <- stats::model.matrix(fit$stata_terms, mf)[, e$cols, drop = FALSE]
    if (anyNA(X)) stop("margins after suest: rows with missing values are not translated yet", call. = FALSE)
    X
  }
  items <- list()
  labels <- character()
  base_of <- list()
  for (v in names(levels)) {
    if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
    lv <- sort(unique(d[[v]]))
    base <- if (v %in% names(bases) && !is.na(bases[[v]])) as.numeric(bases[[v]]) else lv[1]
    base_of[[v]] <- base
    for (l in levels[[v]]) {
      if (!l %in% lv || l == base) stop("margins: dydx() of a base or unseen level is not translated yet", call. = FALSE)
      nd1 <- d
      nd1[[v]] <- l
      nd0 <- d
      nd0[[v]] <- base
      items[[length(items) + 1]] <- lapply(terms, function(t) {
        e <- eq_of(t$eq)
        list(f = t$f, idx = e$at + e$pos, X1 = design(e, nd1), X0 = design(e, nd0))
      })
      labels <- c(labels, paste0(l, ".", v))
    }
  }
  b <- m$b
  est <- numeric(length(items))
  J <- matrix(0, length(items), length(b))
  for (i in seq_along(items)) {
    for (p in items[[i]]) {
      x1 <- drop(p$X1 %*% b[p$idx])
      x0 <- drop(p$X0 %*% b[p$idx])
      if (p$f == "invlogit") {
        est[i] <- est[i] + mean(stats::plogis(x1) - stats::plogis(x0))
        J[i, p$idx] <- J[i, p$idx] + colMeans(stats::dlogis(x1) * p$X1 - stats::dlogis(x0) * p$X0)
      } else {
        est[i] <- est[i] + mean(x1 - x0)
        J[i, p$idx] <- J[i, p$idx] + colMeans(p$X1 - p$X0)
      }
    }
  }
  Vm <- J %*% m$V %*% t(J)
  names(est) <- labels
  dimnames(Vm) <- list(labels, labels)
  if (!quiet) {
    cat(formatC("Average marginal effects", width = -56, flag = "-"), sprintf("Number of obs = %6s\n", format(m$N, big.mark = ",")), sep = "")
    cat("Model VCE: Robust\n\n")
    cat("Expression:", expression, "\n")
    cat("dy/dx wrt: ", paste(labels, collapse = " "), "\n\n")
    if (identical(mcomp, "bonferroni")) {
      # each term's levels are its comparisons; p-values and intervals adjusted
      term <- sub("^[^.]*\\.", "", labels)
      k <- as.numeric(table(term)[term])
      se <- sqrt(diag(Vm))
      z <- est / se
      p <- pmin(1, k * 2 * stats::pnorm(-abs(z)))
      q <- stats::qnorm(1 - (1 - level) / (2 * k))
      cat(sprintf("%12s | %10s %10s %8s %7s   [%2.0f%% conf. interval] (Bonferroni)\n", "", "dy/dx", "std. err.", "z", "P>|z|", 100 * level))
      for (j in seq_along(est)) cat(sprintf("%12s | %10.7g %10.7g %8.2f %7.3f %12.7g %12.7g\n", labels[j], est[j], se[j], z[j], p[j], est[j] - q[j] * se[j], est[j] + q[j] * se[j]))
    } else {
      stata_coefs(est, Vm, Inf, level, label = "dy/dx")
    }
    cat("Note: dy/dx for factor levels is the discrete change from the base level.\n")
  }
  # e(b) after post: each level asked for and its base, in level order
  nm <- character()
  src <- integer()
  for (v in names(levels)) {
    for (l in sort(unique(c(levels[[v]], base_of[[v]])))) {
      if (l == base_of[[v]]) {
        nm <- c(nm, paste0(l, "b.", v))
        src <- c(src, NA)
      } else {
        nm <- c(nm, paste0(l, ".", v))
        src <- c(src, match(paste0(l, ".", v), labels))
      }
    }
  }
  have <- !is.na(src)
  full_b <- stats::setNames(rep(0, length(nm)), nm)
  full_b[have] <- est[src[have]]
  full_V <- matrix(0, length(nm), length(nm), dimnames = list(nm, nm))
  full_V[have, have] <- Vm[src[have], src[have]]
  invisible(list(b = est, V = Vm, full_b = full_b, full_V = full_V, N = m$N))
}"##;

impl Session {
    /// `suest name name ..., [vce(cluster v)]` of stored clogit results.
    pub(super) fn suest(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[("vce", 3), ("coeflegend", 5)]) {
            return Err(no(format!(
                "suest with the option {o} is not translated yet"
            )));
        }
        if c.parts.if_exp.is_some() || c.parts.in_range.is_some() || c.parts.weight.is_some() {
            return Err(no("suest with if, in or weights is not translated yet"));
        }
        let names: Vec<String> = c
            .parts
            .main
            .split_whitespace()
            .map(str::to_string)
            .collect();
        if names.len() < 2 {
            return Err(no(
                "suest of fewer than two stored results is not translated yet",
            ));
        }
        let mut factors = BTreeMap::new();
        for n in &names {
            let Some(f) = self.stored_fit(n) else {
                return Err(no(format!(
                    "suest: {n} is not stored by estimates store earlier in this do-file (Stata: estimation result {n} not found)"
                )));
            };
            let Some(est) = f.estimates() else {
                return Err(no(format!(
                    "suest: the model stored as {n} is not known here"
                )));
            };
            if est.fit != Fit::Clogit {
                return Err(no(format!(
                    "suest of {} results is not translated yet (clogit is)",
                    est.fit.name()
                )));
            }
            if f.robust_or_weighted() {
                return Err(no(format!(
                    "suest: {n} was fitted with robust errors or weights, which suest does not take here"
                )));
            }
            if f.changed_since() {
                return Err(no(format!(
                    "suest: the data changed after {n} was fitted; not translated"
                )));
            }
            factors.extend(est.factors.clone());
        }
        let cluster = match opts.arg("vce", 3).as_deref().map(str::trim) {
            None => None,
            Some(v) if !v.is_empty() && "robust".starts_with(v) => None,
            Some(v) => {
                let mut w = v.split_whitespace();
                let kind = w.next().unwrap_or("");
                let var = w.next().unwrap_or("");
                if kind.len() >= 2
                    && "cluster".starts_with(kind)
                    && is_name(var)
                    && w.next().is_none()
                {
                    Some(var.to_string())
                } else {
                    return Err(no(format!("suest, vce({v}) is not translated yet")));
                }
            }
        };
        let d = self.data()?;
        self.helpers.insert("stata_suest");
        self.helpers.insert("stata_eb");
        self.helpers.insert("stata_names");
        self.helpers.insert("stata_coefs");
        self.helpers.insert("stata_returned");
        let list = names
            .iter()
            .map(|n| r_text(n))
            .collect::<Vec<_>>()
            .join(", ");
        let cl = match &cluster {
            Some(v) => r_text(v),
            None => "NULL".into(),
        };
        out.line(format!(
            "stata_m <- stata_suest(c({list}), {d}, cluster = {cl})"
        ));
        if !self.quiet(c) {
            out.line(format!(
                "stata_suest_print(stata_m, c({list}), clustvar = {cl})"
            ));
        }
        out.line("stata_b <- stata_m$b");
        out.line("stata_vcov <- stata_m$V");
        out.line("stata_df <- Inf");
        out.line(format!(
            "stata_esample <- seq_len(nrow({d})) %in% stata_m$rows"
        ));
        let eqnames: Vec<String> = (1..=names.len())
            .map(|i| format!("eqnames{i} = stata_m$eqs[[{i}]]$depvar"))
            .collect();
        out.line(format!(
            "stata_e <- list(N = stata_m$N, N_clust = {}, rank = stata_m$rank, cmd = \"suest\", {}, names = {}, vcetype = \"Robust\", vce = {}{}, properties = \"b V\", b = matrix(stata_m$b, 1, dimnames = list(\"y1\", names(stata_m$b))), V = stata_m$V)",
            if cluster.is_some() { "stata_m$G" } else { "NULL" },
            eqnames.join(", "),
            r_text(&names.join(" ")),
            if cluster.is_some() { "\"cluster\"" } else { "\"robust\"" },
            match &cluster {
                Some(v) => format!(", clustvar = {}", r_text(v)),
                None => String::new(),
            }
        ));
        out.line("stata_estimates <- structure(stata_m, class = \"stata_suest\")");
        out.line("rm(stata_m)");
        out.caveat(
            "printed in R's layout; e(b) and e(V) as Stata's, the equations named name_depvar",
        );
        self.estimates = Some(Estimates {
            fit: Fit::Suest,
            dep: String::new(),
            factors,
        });
        self.estimates_lost = None;
        self.e_new = true;
        self.e_ret = Returned::Kept {
            cmd: "suest".into(),
            model: String::new(),
            names: &[],
        };
        self.fit_robust = true;
        self.fit_cluster = cluster.is_some();
        self.weighted_fit = false;
        self.esample = true;
        self.changed_since_fit = false;
        Ok(())
    }

    /// `margins, dydx(#.v ...) expression(invlogit(predict(equation(eq))) +
    /// ...) [post]` after suest.
    pub(super) fn margins_suest(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
        est: &Estimates,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[
            ("dydx", 4),
            ("expression", 3),
            ("post", 4),
            ("mcompare", 5),
            ("level", 3),
        ]) {
            return Err(no(format!(
                "margins after suest with the option {o} is not translated yet"
            )));
        }
        if c.parts.if_exp.is_some() || c.parts.in_range.is_some() || !c.parts.main.trim().is_empty()
        {
            return Err(no(
                "margins after suest with a marginlist, if or in is not translated yet",
            ));
        }
        if self.changed_since_fit {
            return Err(no(
                "margins after suest: the data changed after suest; not translated",
            ));
        }
        let Some(dydx) = opts.arg("dydx", 4) else {
            return Err(no(
                "margins after suest without dydx() is not translated yet",
            ));
        };
        let mut levels: Vec<(String, Vec<String>)> = Vec::new();
        for w in dydx.split_whitespace() {
            let Some((l, v)) = w.split_once('.') else {
                return Err(no(format!(
                    "margins after suest, dydx({dydx}): levels of factor variables (#.name) are translated"
                )));
            };
            if l.is_empty() || !l.bytes().all(|b| b.is_ascii_digit()) || !is_name(v) {
                return Err(no(format!(
                    "margins after suest, dydx({dydx}): levels of factor variables (#.name) are translated"
                )));
            }
            if !est.factors.contains_key(v) {
                return Err(no(format!(
                    "margins after suest: {v} is not a factor variable of the models"
                )));
            }
            match levels.iter_mut().find(|(n, _)| n == v) {
                Some((_, ls)) => ls.push(l.to_string()),
                None => levels.push((v.to_string(), vec![l.to_string()])),
            }
        }
        let Some(expression) = opts.arg("expression", 3) else {
            return Err(no(
                "margins after suest without expression() is not translated yet",
            ));
        };
        let expression = expression.trim().to_string();
        let mut terms = Vec::new();
        for t in expression.split('+') {
            let t: String = t.chars().filter(|c| !c.is_whitespace()).collect();
            let (f, inner) = match t
                .strip_prefix("invlogit(")
                .and_then(|r| r.strip_suffix(')'))
            {
                Some(r) => ("invlogit", r.to_string()),
                None => ("xb", t.clone()),
            };
            let eq = inner
                .strip_prefix("predict(equation(")
                .or_else(|| inner.strip_prefix("predict(eq("))
                .and_then(|r| r.strip_suffix("))"))
                .or_else(|| inner.strip_prefix("xb(").and_then(|r| r.strip_suffix(')')));
            match eq {
                Some(eq) if is_name(eq) => terms.push(format!(
                    "list(eq = {}, f = {})",
                    r_text(eq),
                    r_text(f)
                )),
                _ => {
                    return Err(no(format!(
                        "margins after suest, expression({expression}): sums of invlogit(predict(equation(eq))) and predict(equation(eq)) are translated"
                    )))
                }
            }
        }
        let level = match opts.arg("level", 3) {
            Some(l) => {
                l.trim()
                    .parse::<f64>()
                    .map_err(|_| no("level() must be a number"))?
                    / 100.0
            }
            None => 0.95,
        };
        let mcomp = match opts.arg("mcompare", 5).as_deref().map(str::trim) {
            None | Some("noadjust") => "",
            Some(m) if m.len() >= 3 && "bonferroni".starts_with(m) => "bonferroni",
            Some(m) => return Err(no(format!("margins, mcompare({m}) is not translated yet"))),
        };
        let bases: Vec<String> = est
            .factors
            .iter()
            .filter_map(|(v, expr)| {
                super::margins::factor_base(expr).map(|b| format!("{} = {}", r_text(v), r_text(&b)))
            })
            .collect();
        let d = self.data()?;
        self.helpers.insert("stata_suest");
        self.helpers.insert("stata_coefs");
        let call = format!(
            "stata_suest_margins(stata_estimates, {d}, levels = list({}), terms = list({}), bases = c({}), level = {}, mcomp = {}, expression = {}, quiet = {})",
            levels
                .iter()
                .map(|(v, ls)| format!("{} = c({})", r_name(v), ls.join(", ")))
                .collect::<Vec<_>>()
                .join(", "),
            terms.join(", "),
            bases.join(", "),
            r_number(level),
            r_text(mcomp),
            r_text(&expression),
            r_logical(self.quiet(c)),
        );
        out.line(format!("stata_m <- {call}"));
        if opts.has("post", 4) {
            out.line("stata_b <- stata_m$full_b");
            out.line("stata_vcov <- stata_m$full_V");
            out.line("stata_e <- list(N = stata_m$N, cmd = \"margins\", est_cmd = \"suest\", properties = \"b V\", vce = \"delta\", b = matrix(stata_m$full_b, 1, dimnames = list(\"y1\", names(stata_m$full_b))), V = stata_m$full_V)");
            self.estimates = Some(Estimates {
                fit: Fit::Margins,
                dep: String::new(),
                factors: est.factors.clone(),
            });
            self.e_new = true;
            self.e_ret = Returned::Kept {
                cmd: "margins".into(),
                model: "suest".into(),
                names: &[],
            };
            out.caveat(
                "post: the margins are now e(b) and e(V), named as Stata names them (1.g, 4b.g)",
            );
        } else {
            out.line("stata_r <- list(N = stata_m$N, cmd = \"margins\", est_cmd = \"suest\", b = matrix(stata_m$full_b, 1, dimnames = list(\"r1\", names(stata_m$full_b))), V = stata_m$full_V)");
            self.r_new = Some((super::returned::R_MARGINS, "margins".into()));
        }
        out.line("rm(stata_m)");
        out.caveat("averaged over the rows of the models suest combined, with delta-method errors through its e(V)");
        Ok(())
    }
}
