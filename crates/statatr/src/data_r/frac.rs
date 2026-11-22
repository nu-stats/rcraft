//! Batch M of the StatCraft port (P2C.12ar): fraction and count models fitted by
//! maximum likelihood, as Stata 19 reports them: `fracreg` (`plan/stata/fracreg.md`), and
//! the others as they are added (`betareg`, `boxcox`, `zip`, `zinb`, `tpoisson`, `tnbreg`,
//! `ztp`, `ztnb`).
//!
//! The likelihoods are written out in R (a separate process, never linked) and maximised by
//! Stata's own Newton-Raphson steps (`stata_ml_max`, shared with `ivprobit`): a full step,
//! halved while the likelihood falls, else stepped on by 1/8, 1/4, ... while it rises, and
//! the modified Marquardt step where -H is not positive definite. The iteration log is each
//! point's log likelihood.

use super::model::{level, Estimates, Fit, Spec};
use super::weights::{Weighting, W};
use super::*;

/// The design, the table, the fits and the predictions.
pub(super) const FRAC_HELPER: &str = r##"stata_frac_num <- function(u) { u <- unclass(u); attributes(u) <- NULL; as.numeric(u) }
# the rows of d a model uses, its design matrix (the constant last) and the names e(b) gives the columns,
# a factor's base level a zero column of its own
stata_frac_load <- function(d, dep, terms, zvars = character(), wvar = NULL, cluster = NULL, extra = character()) {
  tvars <- setdiff(unique(unlist(regmatches(terms, gregexpr("[A-Za-z_][A-Za-z0-9_]*", terms)))), "factor")
  allv <- unique(c(dep, tvars, zvars, wvar, cluster, extra))
  for (v in allv) if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
  ok <- stats::complete.cases(d[allv])
  if (!any(ok)) stop("no observations (Stata: r(2000))", call. = FALSE)
  rows <- which(ok)
  mf <- d[rows, , drop = FALSE]
  n <- nrow(mf)
  y <- stata_frac_num(mf[[dep]])
  if (length(terms)) {
    fo <- stats::as.formula(paste("~", paste(terms, collapse = " + ")))
    X <- stats::model.matrix(fo, mf)
    X <- X[, colnames(X) != "(Intercept)", drop = FALSE]
    xlev <- stats::.getXlevels(stats::terms(fo), stats::model.frame(fo, mf))
  } else {
    fo <- NULL
    X <- matrix(0, n, 0)
    xlev <- list()
  }
  rn <- function(s) ifelse(grepl("^factor\\(", s), sub("^factor\\(([^)]*)\\)(.*)$", "\\2.\\1", s), s)
  nmx <- rn(colnames(X))
  full <- character()
  for (j in seq_along(nmx)) {
    cj <- colnames(X)[j]
    fv <- if (grepl("^factor\\(", cj)) sub("^(factor\\([^)]*\\)).*$", "\\1", cj) else NA_character_
    if (!is.na(fv) && (j == 1 || !startsWith(colnames(X)[j - 1], fv))) {
      v <- sub("^factor\\((.*)\\)$", "\\1", fv)
      full <- c(full, paste0(xlev[[fv]][1], "b.", v))
    }
    full <- c(full, nmx[j])
  }
  Z <- if (length(zvars)) sapply(zvars, function(v) stata_frac_num(mf[[v]])) else matrix(0, n, 0)
  if (!is.matrix(Z)) Z <- matrix(Z, n)
  w <- if (is.null(wvar)) rep(1, n) else stata_frac_num(mf[[wvar]])
  list(rows = rows, mf = mf, n = n, y = y, X = cbind(X, 1), xnames = c(nmx, "_cons"), colnames = c(colnames(X), "_cons"),
       full = c(full, "_cons"), Z = Z, znames = zvars, w = w, fo = fo, xlev = xlev,
       cl = if (is.null(cluster)) NULL else mf[[cluster]])
}
# the sandwich over weighted scores: a frequency-weighted row once, with factor N/(N - 1) (G/(G - 1) over clusters)
stata_frac_sandwich <- function(Ainv, S, w, fw = FALSE, cl = NULL) {
  if (!is.null(cl)) {
    U <- rowsum(S, cl)
    G <- nrow(U)
    Ainv %*% crossprod(U) %*% Ainv * G / (G - 1)
  } else {
    n <- if (fw) sum(w) else nrow(S)
    M <- if (fw) crossprod(S / sqrt(w)) else crossprod(S)
    Ainv %*% M %*% Ainv * n / (n - 1)
  }
}
# the coefficient rows of one equation, as Stata prints them
stata_frac_rows_print <- function(names, b, se, level, eform = FALSE, data = NULL, df = Inf) {
  pad <- stata_tab_pad
  rows <- stata_coef_rows(names, data)
  if (is.null(rows)) stop("RCraft: a model with a term whose table layout was not checked is not translated yet", call. = FALSE)
  qz <- if (is.finite(df)) stats::qt((1 + level / 100) / 2, df) else stats::qnorm((1 + level / 100) / 2)
  for (r in rows) {
    if (r$kind == "blank") { cat(strrep(" ", 13), "|\n", sep = ""); next }
    if (r$kind == "head") { cat(pad(stata_tab_abbrev(r$label), 12), " |\n", sep = ""); next }
    if (r$kind == "wrap") { cat(pad(r$label, 11), " #|\n", sep = ""); next }
    label <- if (r$kind == "level") paste0(pad(r$label, 11), " ") else pad(stata_tab_abbrev(r$label), 12)
    if (!is.null(r$note)) { cat(label, " |", pad(if (eform) "1" else "0", 11), "  ", r$note, "\n", sep = ""); next }
    j <- match(r$col, names)
    bi <- b[[j]]; si <- se[[j]]
    z <- bi / si
    p <- if (is.finite(df)) 2 * stats::pt(-abs(z), df) else 2 * stats::pnorm(-abs(z))
    lo <- bi - qz * si; hi <- bi + qz * si
    if (eform) { si <- exp(bi) * si; lo <- exp(lo); hi <- exp(hi); bi <- exp(bi) }
    cat(label, " |", pad(stata_tab_g(bi), 11), pad(stata_tab_g(si), 11), pad(sprintf("%.2f", z), 9),
        pad(sprintf("%.3f", p), 8), pad(stata_tab_g(lo), 13), pad(stata_tab_g(hi), 12), "\n", sep = "")
  }
}
# the iteration log
stata_frac_log <- function(lg, label) {
  for (i in seq_along(lg)) cat(formatC(paste0("Iteration ", i - 1, ":"), width = -14), label, " = ",
                               formatC(stata_fmt(lg[i], "%10.0g", trim = TRUE), width = 10), "  \n", sep = "")
}
# the right-hand block of an ML header: Number of obs, Wald or LR chi2, Prob > chi2 and Pseudo R2 beside the title
stata_frac_header <- function(title, n, stat_label, stat, df, ll_label, ll, r2, pseudo = TRUE) {
  vals <- c(stata_tab_count(n), if (is.na(stat)) "." else sprintf("%.2f", stat),
            if (df == 0 || is.na(stat)) "." else sprintf("%.4f", stats::pchisq(stat, df, lower.tail = FALSE)), sprintf("%.4f", r2))
  wv <- max(6, nchar(vals))
  row <- function(left, label, value) cat(formatC(left, width = -(56 - (wv - 6)), flag = "-"), formatC(label, width = -13, flag = "-"), " = ",
                                         stata_tab_pad(value, wv), "\n", sep = "")
  llt <- paste0(ll_label, " = ", trimws(stata_fmt(ll, "%10.0g")))
  if (df == 0) {
    row(title, "Number of obs", vals[1])
    row(llt, "Pseudo R2", vals[4])
  } else {
    row(title, "Number of obs", vals[1])
    row("", sprintf("%s(%d)", stat_label, df), vals[2])
    row("", "Prob > chi2", vals[3])
    row(llt, "Pseudo R2", vals[4])
  }
  cat("\n")
}
stata_frac_tophead <- function(dep, label, robust, level, w = 12) {
  lvl <- stata_fmt(level, "%9.0g", trim = TRUE)
  cat(strrep("-", 78), "\n", sep = "")
  if (robust) cat(strrep(" ", w + 1), "|               Robust\n", sep = "")
  cat(stata_tab_pad(stata_tab_abbrev(dep), w), " | ", formatC(formatC(label, width = 10), width = -13, flag = "-"),
      if (robust) "std. err." else "Std. err.", "      z    P>|z|     [", lvl, "% conf. interval]\n", sep = "")
  cat(strrep("-", w + 1), "+", strrep("-", 64), "\n", sep = "")
}
# the variance matrix as Stata keeps it: symmetric
stata_frac_sym <- function(V) (V + t(V)) / 2
# (-H)^-1, or where -H is not positive definite the inverse of -H + lambda I for the smallest lambda (doubling from
# 1e-4 of the largest diagonal) that makes it so
stata_frac_cinv <- function(A) {
  pd <- function(M) !inherits(tryCatch(chol(M), error = function(e) e), "error")
  if (pd(A)) return(list(inv = solve(A), nc = FALSE))
  lam <- 1e-4 * max(max(abs(diag(A))), 1)
  for (i in 1:80) {
    M <- A; diag(M) <- diag(M) + lam
    if (pd(M)) return(list(inv = solve(M), nc = TRUE))
    lam <- lam * 2
  }
  stop("could not calculate numerical derivatives; missing values encountered (Stata: r(430))", call. = FALSE)
}
# Wald statistic of the coefficients idx
stata_frac_wald <- function(b, V, idx) if (length(idx)) drop(t(b[idx]) %*% solve(V[idx, idx, drop = FALSE], b[idx])) else NA_real_

stata_fracreg_terms <- function(model, y, eta) {
  if (model == "logit") {
    mu <- stats::plogis(eta)
    list(ll = y * stats::plogis(eta, log.p = TRUE) + (1 - y) * stats::plogis(-eta, log.p = TRUE), a = y - mu, d = -mu * (1 - mu))
  } else {
    lq <- stats::pnorm(eta, log.p = TRUE); lr <- stats::pnorm(-eta, log.p = TRUE)
    ld <- stats::dnorm(eta, log = TRUE)
    rq <- exp(ld - lq); rr <- exp(ld - lr)
    list(ll = y * lq + (1 - y) * lr, a = y * rq - (1 - y) * rr,
         d = y * (-eta * rq - rq^2) - (1 - y) * (-eta * rr + rr^2))
  }
}
# the quasi-likelihood, its score, -H and each row's weighted score over (beta, gamma)
stata_fracreg_fit <- function(model, y, X, Z, w, b0, maxit = 300) {
  kx <- ncol(X); kz <- ncol(Z); het <- kz > 0
  parts <- function(b) {
    xb <- drop(X %*% b[seq_len(kx)])
    if (het) { wt <- exp(-drop(Z %*% b[kx + seq_len(kz)])); eta <- xb * wt } else { wt <- rep(1, length(xb)); eta <- xb }
    list(xb = xb, wt = wt, eta = eta)
  }
  llf <- function(b) { p <- parts(b); sum(w * stata_fracreg_terms(model, y, p$eta)$ll) }
  derivs <- function(b) {
    p <- parts(b); tm <- stata_fracreg_terms(model, y, p$eta)
    a <- w * tm$a; dd <- w * tm$d
    G <- cbind(X * p$wt, if (het) -p$eta * Z else NULL)
    S <- a * G
    H2 <- crossprod(G * dd, G)
    if (het) {
      bi <- seq_len(kx); gi <- kx + seq_len(kz)
      H2[bi, gi] <- H2[bi, gi] + crossprod(X * (-a * p$wt), Z)
      H2[gi, bi] <- t(H2[bi, gi])
      H2[gi, gi] <- H2[gi, gi] + crossprod(Z * (a * p$eta), Z)
    }
    list(ll = sum(w * tm$ll), g = colSums(S), negH = -H2, scores = S)
  }
  stata_ml_max(llf, derivs, b0, maxit)
}
stata_fracreg_start <- function(model, y, X, Z, w, maxit) {
  ols <- function(X) as.numeric(stats::lm.wfit(X, y, w)$coefficients)
  if (ncol(Z) == 0) return(ols(X))
  m <- stata_fracreg_fit("probit", y, X, Z[, 0, drop = FALSE], w, ols(X), maxit)
  b <- m$b
  for (i in 1:20) {
    dd <- stata_fracreg_fit("probit", y, X, Z[, 0, drop = FALSE], w, b, 0)
    step <- as.numeric(solve(dd$negH, dd$g))
    nb <- b + step
    if (identical(nb, b)) break
    b <- nb
  }
  c(b, rep(0, ncol(Z)))
}
stata_fracreg <- function(d, dep, terms, het = character(), model = "logit", wvar = NULL, wtype = NULL, cluster = NULL, odds = FALSE,
                          level = 95, nolog = FALSE, maxit = 300, cmdline = "", wtext = "", all = NULL, show = TRUE) {
  L <- stata_frac_load(d, dep, terms, het, wvar, cluster)
  y <- L$y; w <- L$w; X <- L$X; Z <- L$Z; n <- L$n
  if (any(y < 0 | y > 1)) stop(dep, " has values outside [0,1] (Stata: r(459))", call. = FALSE)
  fw <- identical(wtype, "f")
  N <- if (fw) sum(w) else n
  kx <- ncol(X); kz <- ncol(Z)
  mdl <- if (model == "hetprobit") "probit" else model
  fit <- stata_fracreg_fit(mdl, y, X, Z, w, stata_fracreg_start(mdl, y, X, Z, w, maxit), maxit)
  null <- {
    X0 <- X[, kx, drop = FALSE]
    b0 <- if (kz) c(stata_fracreg_start(mdl, y, X0, Z[, 0, drop = FALSE], w, maxit), rep(0, kz)) else stata_fracreg_start(mdl, y, X0, Z, w, maxit)
    stata_fracreg_fit(mdl, y, X0, Z, w, b0, maxit)
  }
  Ainv <- solve(fit$negH)
  V <- stata_frac_sym(stata_frac_sandwich(Ainv, fit$scores, w, fw, L$cl))
  b <- fit$b; p <- length(b)
  k1 <- kx - 1
  chi2 <- if (k1 > 0) drop(t(b[seq_len(k1)]) %*% solve(V[seq_len(k1), seq_len(k1), drop = FALSE], b[seq_len(k1)])) else NA_real_
  ll <- fit$ll; ll0 <- null$ll; r2 <- 1 - ll / ll0
  title <- switch(model, probit = "Fractional probit regression", logit = "Fractional logistic regression",
                  "Fractional heteroskedastic probit regression")
  pn <- c(paste0(dep, ":", L$full[-length(L$full)]), paste0(dep, ":_cons"), if (kz) paste0("lnsigma:", L$znames))
  # e(b) and e(V) carry a factor's base level as a zero column
  ind <- match(c(L$xnames[-length(L$xnames)], "_cons"), L$full)
  nf <- length(L$full) + kz
  pos <- c(ind, if (kz) length(L$full) + seq_len(kz))
  bF <- numeric(nf); bF[pos] <- b
  VF <- matrix(0, nf, nf); VF[pos, pos] <- V
  VmF <- matrix(0, nf, nf); VmF[pos, pos] <- stata_frac_sym(Ainv)
  cn <- c(paste0(dep, ":", L$full), if (kz) paste0("lnsigma:", L$znames))
  mns <- NULL
  if (is.null(all)) all <- d
  mm <- vapply(L$colnames[-length(L$colnames)], function(v) mean(stata_frac_num(all[[v]]), na.rm = TRUE), 0)
  mns <- c(rev(mm), 1)
  e <- list(rank = p, N = N, ic = length(fit$log) - 1, k = nf, k_eq = if (kz) 2 else 1, k_dv = 1, converged = as.numeric(fit$converged), rc = 0)
  if (!is.null(L$cl)) e$N_clust <- length(unique(L$cl))
  e <- c(e, list(ll = ll, ll_0 = ll0, r2_p = r2, df_m = k1, k_eq_model = 1, chi2 = chi2, p = if (k1 > 0) stats::pchisq(chi2, k1, lower.tail = FALSE) else NA_real_))
  e$cmdline <- cmdline
  e$title <- title
  e$estimator <- switch(model, probit = "probit", logit = "logit", "hetprobit")
  e$cmd <- "fracreg"
  e$estat_cmd <- "fracreg_estat"
  e$predict <- "fracreg_p"
  e$chi2type <- "Wald"
  e$marginsnotok <- "stdp scores"
  if (!is.null(wtype)) { e$wexp <- paste0("= ", wtext); e$wtype <- switch(wtype, f = "fweight", p = "pweight", i = "iweight") }
  e$ml_method <- "lf2"
  e$opt <- "moptimize"
  e$which <- "max"
  e$technique <- "nr"
  e$depvar <- dep
  e$vcetype <- "Robust"
  if (!is.null(cluster)) e$clustvar <- cluster
  e$vce <- if (is.null(cluster)) "robust" else "cluster"
  e$properties <- "b V"
  e$b <- matrix(bF, 1, nf, dimnames = list("y1", cn))
  e$V <- `dimnames<-`(VF, list(cn, cn))
  e$V_modelbased <- `dimnames<-`(VmF, list(cn, cn))
  mnn <- L$xnames
  e$mns <- matrix(mns, 1, length(mns), dimnames = list("r1", mnn))
  gF <- numeric(nf); gF[pos] <- fit$g
  e$gradient <- matrix(gF, 1, nf, dimnames = list("r1", cn))
  il <- numeric(20); m <- min(20, length(fit$log)); il[seq_len(m)] <- fit$log[seq_len(m)]
  e$ilog <- matrix(il, 1, 20, dimnames = list("r1", paste0("c", 1:20)))
  attr(e, "stata_rows") <- L$rows
  rname <- c(L$colnames[-length(L$colnames)], "_cons")
  rname[rname == "_cons"] <- "(Intercept)"
  attr(e, "stata_b") <- stats::setNames(b[seq_len(kx)], rname)
  attr(e, "stata_V") <- { Vn <- V[seq_len(kx), seq_len(kx), drop = FALSE]; dimnames(Vn) <- list(rname, rname); Vn }
  attr(e, "stata_areg") <- list(fo = L$fo, xlev = L$xlev, nmx = L$xnames[-length(L$xnames)], znames = L$znames, model = model, dep = dep)
  if (show) {
    cat("\n")
    if (!nolog) stata_frac_log(fit$log, "Log pseudolikelihood")
    cat("\n")
    stata_frac_header(title, N, "Wald chi2", chi2, k1, "Log pseudolikelihood", ll, r2)
    if (!is.null(L$cl)) cat(stata_tab_pad(sprintf("(Std. err. adjusted for %s clusters in %s)", stata_tab_count(length(unique(L$cl))), cluster), 78), "\n", sep = "")
    stata_frac_tophead(dep, if (odds) "Odds ratio" else "Coefficient", TRUE, level)
    se <- sqrt(diag(V))
    if (kz) cat(formatC(stata_tab_abbrev(dep), width = -13), "|\n", sep = "")
    stata_frac_rows_print(L$xnames, b[seq_len(kx)], se[seq_len(kx)], level, odds, L$mf)
    if (kz) {
      cat(strrep("-", 13), "+", strrep("-", 64), "\n", sep = "")
      cat(formatC("lnsigma", width = -13), "|\n", sep = "")
      stata_frac_rows_print(L$znames, b[kx + seq_len(kz)], se[kx + seq_len(kz)], level)
    }
    cat(strrep("-", 78), "\n", sep = "")
    if (odds) cat("Note: _cons estimates baseline odds.\n")
    if (!fit$converged) cat("Warning: Convergence not achieved.\n")
  }
  e
}
stata_frac_predict <- function(e, d, what) {
  a <- attr(e, "stata_areg")
  b <- e$b
  num <- stata_frac_num
  xb <- if (is.null(a$fo)) rep(0, nrow(d)) else {
    mf <- stats::model.frame(a$fo, d, xlev = a$xlev, na.action = stats::na.pass)
    X <- stats::model.matrix(a$fo, mf)
    X <- X[, colnames(X) != "(Intercept)", drop = FALSE]
    drop(X %*% b[1, paste0(a$dep, ":", a$nmx)])
  }
  xb <- xb + b[1, paste0(a$dep, ":_cons")]
  if (what == "xb") return(xb)
  eta <- if (length(a$znames)) xb / exp(Reduce(`+`, lapply(a$znames, function(v) num(d[[v]]) * b[1, paste0("lnsigma:", v)]))) else xb
  if (a$model == "logit") stats::plogis(eta) else stats::pnorm(eta)
}"##;

impl Session {
    /// Whether a model of this batch is the one last fitted.
    pub(super) fn frac_active(&self) -> Option<String> {
        match &self.e_ret {
            super::returned::Returned::Program { cmd, .. }
                if FRAC_COMMANDS.contains(&cmd.as_str())
                    && self.estimates.as_ref().is_some_and(|e| e.fit == Fit::Frac) =>
            {
                Some(cmd.clone())
            }
            _ => None,
        }
    }

    /// The weight of a command of this batch, only the kinds `ok`; a kind Stata refuses stops
    /// the run in its words (`None`: the stop was written).
    pub(super) fn frac_weight(
        &self,
        c: &Command,
        ok: &[W],
        out: &mut Out,
    ) -> Result<Option<Weighting>, Untranslated> {
        let w = self.weight_of(c, &[W::F, W::A, W::P, W::I])?;
        if let Some((k, _)) = &w {
            if !ok.contains(k) {
                let word = match k {
                    W::F => "fweights",
                    W::A => "aweights",
                    W::P => "pweights",
                    W::I => "iweights",
                };
                out.line(format!(
                    "stop({}, call. = FALSE)",
                    r_text(&format!("{word} not allowed (Stata: r(101))"))
                ));
                return Ok(None);
            }
        }
        Ok(Some(w))
    }

    /// The terms of a model of this batch: plain variables and `i.` factors only.
    pub(super) fn frac_spec(&self, c: &Command, name: &str) -> Result<Spec, Untranslated> {
        let spec = Spec::read(c, name)?;
        if !is_name(&spec.dep) {
            return Err(no(format!(
                "{name} with operators on the outcome is not translated yet"
            )));
        }
        let plain = |t: &str| {
            is_name(t)
                || (t.starts_with("factor(") && t.ends_with(')') && is_name(&t[7..t.len() - 1]))
        };
        if spec.terms.iter().any(|t| !plain(t)) {
            return Err(no(format!(
                "{name} with interactions, ib. bases or operators is not translated yet"
            )));
        }
        Ok(spec)
    }

    /// The part every fit of the batch shares after its R call: what a later `predict`,
    /// `_b[]` and `e()` read.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn frac_keep(
        &mut self,
        c: &Command,
        out: &mut Out,
        src: &str,
        cmd: &str,
        spec: &Spec,
        texts: &[&str],
    ) -> Result<(), Untranslated> {
        out.line("stata_b <- attr(stata_e, \"stata_b\")");
        out.line("stata_vcov <- attr(stata_e, \"stata_V\")");
        out.line("stata_df <- Inf");
        out.line("stata_estimates <- list(rows = attr(stata_e, \"stata_rows\"))");
        out.line(self.keep_rows(src, "stata_estimates$rows"));
        let d = self.data()?;
        out.package("dplyr");
        out.line(super::model::esample_rows_line(
            &d,
            src,
            "stata_estimates$rows",
        ));
        self.esample_new = true;
        self.fit_robust = false;
        self.fit_cluster = false;
        self.estimates = Some(Estimates {
            fit: Fit::Frac,
            dep: spec.dep.clone(),
            factors: spec.factors.clone(),
        });
        self.e_new = true;
        self.e_ret = super::returned::Returned::Program {
            cmd: cmd.into(),
            texts: texts.iter().map(|s| s.to_string()).collect(),
        };
        let _ = c;
        Ok(())
    }

    /// `fracreg probit|logit depvar [indepvars] [if] [in] [weight] [, het() or level() nolog
    /// iterate() vce(cluster v)]`.
    pub(super) fn fracreg(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "fracreg")?;
        let mut c2 = c.clone();
        let mut words = c.parts.main.split_whitespace();
        let sub = words.next().unwrap_or("");
        let model = if !sub.is_empty() && sub.len() >= 2 && "probit".starts_with(sub) {
            "probit"
        } else if sub.len() >= 3 && "logit".starts_with(sub) {
            "logit"
        } else {
            return Err(no(format!("fracreg {sub}: invalid estimator")));
        };
        c2.parts.main = words.collect::<Vec<_>>().join(" ");
        if let Some(o) = opts.other(&[
            ("het", 3),
            ("or", 2),
            ("level", 3),
            ("nolog", 3),
            ("iterate", 4),
            ("vce", 3),
        ]) {
            return Err(no(format!(
                "fracreg with the option {o} is not translated yet"
            )));
        }
        let Some(w) = self.frac_weight(c, &[W::F, W::P, W::I], out)? else {
            return Ok(());
        };
        let spec = self.frac_spec(&c2, "fracreg")?;
        let het: Vec<String> = match opts.arg("het", 3) {
            Some(h) => {
                if model != "probit" {
                    return Err(no("option het() allowed only with fracreg probit"));
                }
                let v: Vec<String> = h.split_whitespace().map(str::to_string).collect();
                if v.iter().any(|t| !is_name(t)) {
                    return Err(no(
                        "het() with factor variables or operators is not translated yet",
                    ));
                }
                v
            }
            None => Vec::new(),
        };
        let odds = opts.has("or", 2);
        if odds && model != "logit" {
            return Err(no("option or not allowed"));
        }
        let cluster = match opts.arg("vce", 3) {
            None => None,
            Some(v) => {
                let v = v.trim().to_string();
                match v.split_once(char::is_whitespace) {
                    Some((k, rest))
                        if k.len() >= 2 && "cluster".starts_with(k) && is_name(rest.trim()) =>
                    {
                        Some(rest.trim().to_string())
                    }
                    _ if v == "robust" || v == "r" => None,
                    _ => return Err(no(format!("fracreg with vce({v}) is not translated yet"))),
                }
            }
        };
        let maxit = match opts.arg("iterate", 4) {
            None => 300,
            Some(t) => t
                .trim()
                .parse::<usize>()
                .map_err(|_| no("option iterate() incorrectly specified"))?,
        };
        let level = level(opts)?;
        let rows = self.rows_for(c, out)?;
        let src = self.weighted_rows(rows, &w);
        for h in [
            "stata_ivprobit",
            "stata_lsfam",
            "stata_fmt",
            "stata_tables",
            "stata_returned",
            "stata_eclass",
            "stata_frac",
        ] {
            self.helpers.insert(h);
        }
        let wargs = match &w {
            Some((k, v)) => format!(
                ", wvar = {}, wtype = {}, wtext = {}",
                r_text(v),
                r_text(k.code()),
                r_text(v)
            ),
            None => String::new(),
        };
        let cl = match &cluster {
            Some(v) => format!(", cluster = {}", r_text(v)),
            None => String::new(),
        };
        let show = !self.quiet(c);
        let all = self.data()?;
        out.line(format!(
            "stata_e <- stata_fracreg({src}, {}, c({}), het = {}, model = {}{wargs}{cl}, odds = {}, level = {}, nolog = {}, maxit = {maxit}, cmdline = {}, all = {all}, show = {})",
            r_text(&spec.dep),
            spec.terms.iter().map(|t| r_text(t)).collect::<Vec<_>>().join(", "),
            r_vec(&het),
            r_text(if het.is_empty() { model } else { "hetprobit" }),
            super::stats::r_logical(odds),
            r_number(level * 100.0),
            super::stats::r_logical(opts.has("nolog", 3)),
            r_text(&super::returned::cmdline(c)),
            super::stats::r_logical(show)
        ));
        self.frac_keep(
            c,
            out,
            &src,
            "fracreg",
            &spec,
            &[
                "cmdline",
                "title",
                "estimator",
                "cmd",
                "estat_cmd",
                "predict",
                "chi2type",
                "marginsnotok",
                "wtype",
                "wexp",
                "ml_method",
                "opt",
                "which",
                "technique",
                "depvar",
                "vcetype",
                "clustvar",
                "vce",
                "properties",
            ],
        )?;
        out.caveat(
            "fracreg as Stata 19 reports it: the Bernoulli quasi-likelihood by Newton-Raphson with the analytic Hessian from a \
             least squares start, the robust sandwich variance, the Wald test of the slopes",
        );
        Ok(())
    }

    /// `predict [type] newvar [if] [in] [, cm xb]` after a model of this batch.
    pub(super) fn frac_predict(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
        cmd: &str,
    ) -> Result<(), Untranslated> {
        // each statistic (option, shortest form), the first the default Stata assumes
        let stats: &[(&str, usize)] = match cmd {
            "fracreg" => &[("cm", 2), ("xb", 2)],
            "betareg" => &[
                ("cmean", 2),
                ("cvariance", 4),
                ("xb", 2),
                ("xbscale", 5),
                ("stdp", 4),
            ],
            "boxcox" => &[
                ("yhat", 4),
                ("residuals", 3),
                ("smearing", 5),
                ("btransform", 2),
            ],
            "tpoisson" | "tnbreg" | "ztp" | "ztnb" => &[("n", 1), ("xb", 2)],
            _ => &[],
        };
        if matches!(cmd, "zip" | "zinb") {
            return Err(no(format!("predict after {cmd} is not translated yet")));
        }
        let allowed = stats;
        if let Some(o) = opts.other(allowed) {
            return Err(no(format!(
                "predict after {cmd} with the option {o} is not translated yet"
            )));
        }
        if c.options.len() > 1 && cmd != "boxcox" {
            return Err(no("predict takes one statistic at a time"));
        }
        let (first, _) = stats.first().copied().unwrap_or(("cm", 2));
        let what = stats
            .iter()
            .find(|(f, m)| opts.has(f, *m))
            .map_or(first, |(f, _)| *f);
        let mut words: Vec<&str> = c.parts.main.split_whitespace().collect();
        let ty = match words.first() {
            Some(t) if storage(t).is_some() => {
                let t = *t;
                words.remove(0);
                t
            }
            _ => "float",
        };
        let st = storage(ty).ok_or_else(|| no(format!("{ty} is not a storage type")))?;
        if st == Storage::Str {
            return Err(no("predict makes a numeric variable, not a string"));
        }
        let [w] = words.as_slice() else {
            return Err(no("predict makes one new variable"));
        };
        if !is_name(w) {
            return Err(no(format!(
                "{w}: only a plain new variable name is translated"
            )));
        }
        let base = self.data()?;
        out.line(format!(
            "if ({} %in% names({base})) stop(\"variable {w} already defined (Stata: r(110))\", call. = FALSE)",
            r_text(w)
        ));
        if cmd == "boxcox" {
            // what Stata says about the statistic and the smearing it assumed
            let resid = opts.has("residuals", 3);
            let back = opts.has("btransform", 2);
            let smear = opts.has("smearing", 5);
            let yhat = opts.has("yhat", 4);
            let msg = if resid {
                (!smear && !back).then_some("(option smearing assumed to compute residuals)")
            } else if back {
                (!yhat).then_some("(statistic yhat assumed)")
            } else if !yhat && !smear {
                Some("(statistic yhat and option smearing are assumed)")
            } else if !yhat {
                Some("(statistic yhat assumed)")
            } else if !smear {
                Some("(option smearing assumed)")
            } else {
                None
            };
            if let Some(m) = msg {
                if !self.quiet(c) {
                    out.line(format!("cat({}, \"\\n\")", r_text(m)));
                }
            }
        } else if c.options.is_empty() && !self.quiet(c) {
            let note = if first == "n" {
                "(option n assumed; predicted number of events)".to_string()
            } else {
                format!("(option {first} assumed)")
            };
            out.line(format!("cat({}, \"\\n\")", r_text(&note)));
        }
        self.helpers.insert("stata_frac");
        let call = match cmd {
            "boxcox" => {
                self.helpers.insert("stata_boxcox");
                "stata_boxcox_predict"
            }
            "tpoisson" | "tnbreg" | "ztp" | "ztnb" => {
                self.helpers.insert("stata_zcount");
                "stata_zc_predict"
            }
            "betareg" => {
                self.helpers.insert("stata_betareg");
                "stata_betareg_predict"
            }
            _ => "stata_frac_predict",
        };
        let extra = if cmd == "boxcox" {
            format!(
                ", {}, {}",
                super::stats::r_logical(opts.has("smearing", 5)),
                super::stats::r_logical(opts.has("btransform", 2))
            )
        } else {
            String::new()
        };
        let v = format!("{call}(stata_e, {base}, {}{extra})", r_text(what));
        let v = stored(&v, st, &mut self.helpers, out);
        let v = match self.condition(c, out)? {
            Some(cond) => format!("dplyr::if_else({cond}, {v}, NA_real_)"),
            None => v,
        };
        out.package("dplyr");
        out.line(format!(
            "{base} <- dplyr::mutate({base}, {} = {v})",
            r_name(w)
        ));
        self.kinds.insert(w.to_string(), Kind::Number);
        self.storage.insert(w.to_string(), st);
        Ok(())
    }
}

/// The commands of this batch, as `e(cmd)` names them.
pub(super) const FRAC_COMMANDS: &[&str] = &[
    "fracreg", "betareg", "boxcox", "zip", "zinb", "tpoisson", "tnbreg", "ztp", "ztnb",
];

/// An R character vector.
pub(super) fn r_vec(v: &[String]) -> String {
    if v.is_empty() {
        "character()".to_string()
    } else {
        format!(
            "c({})",
            v.iter().map(|t| r_text(t)).collect::<Vec<_>>().join(", ")
        )
    }
}
