//! `streg`: parametric survival regression on the data `stset` made, for the
//! exponential, Weibull, lognormal, loglogistic and Gompertz distributions.
//! Written for R from the model's likelihood alone (`plan/stata/survival.md`):
//! the log likelihood is that of the time itself (each failure adds the
//! log of the density of t, not of log t), late entry subtracts the log
//! survivor function at the entry time, and Newton's method with the exact
//! first and second derivatives is run to its limit, so the estimates are
//! the maximum's and agree with Stata's, which stops at its tolerance, to
//! the digits Stata's own convergence gives.

use super::*;

pub(super) const STREG_HELPER: &str = r##"stata_sl_fam <- function(dist) {
  switch(dist,
    exponential = , weibull = list(
      lf = function(z) z - exp(z), l1 = function(z) 1 - exp(z), l2 = function(z) -exp(z),
      ls = function(z) -exp(z), s1 = function(z) -exp(z), s2 = function(z) -exp(z)),
    lognormal = {
      lam <- function(z) exp(stats::dnorm(z, log = TRUE) - stats::pnorm(z, lower.tail = FALSE, log.p = TRUE))
      list(lf = function(z) -z^2 / 2 - 0.5 * log(2 * pi), l1 = function(z) -z, l2 = function(z) rep(-1, length(z)),
           ls = function(z) stats::pnorm(z, lower.tail = FALSE, log.p = TRUE), s1 = function(z) -lam(z),
           s2 = function(z) { l <- lam(z); -l * (l - z) })
    },
    loglogistic = {
      lg <- function(z) pmax(z, 0) + log1p(exp(-abs(z)))
      list(lf = function(z) z - 2 * lg(z), l1 = function(z) 1 - 2 * stats::plogis(z),
           l2 = function(z) { p <- stats::plogis(z); -2 * p * (1 - p) },
           ls = function(z) -lg(z), s1 = function(z) -stats::plogis(z),
           s2 = function(z) { p <- stats::plogis(z); -p * (1 - p) })
    })
}
# log likelihood, gradient, Hessian and the rows' scores of a location-scale model in (b, ln sigma)
stata_sl_eval <- function(theta, y, y0, dd, X, fam, fixed_s) {
  k <- ncol(X)
  b <- theta[seq_len(k)]
  s <- if (fixed_s) 0 else theta[k + 1]
  sg <- exp(s)
  xb <- drop(X %*% b)
  z <- (y - xb) / sg
  ent <- is.finite(y0)
  z0 <- (y0 - xb) / sg
  p1 <- dd * fam$l1(z) + (1 - dd) * fam$s1(z)
  p2 <- dd * fam$l2(z) + (1 - dd) * fam$s2(z)
  q1 <- ifelse(ent, -fam$s1(ifelse(ent, z0, 0)), 0)
  q2 <- ifelse(ent, -fam$s2(ifelse(ent, z0, 0)), 0)
  ll <- sum(dd * (fam$lf(z) - s) + (1 - dd) * fam$ls(z)) + sum(ifelse(ent, -fam$ls(ifelse(ent, z0, 0)), 0))
  gb <- -(p1 + q1) / sg
  sc <- X * gb
  Hbb <- crossprod(X * ((p2 + q2) / sg^2), X)
  g <- colSums(sc)
  H <- Hbb
  if (!fixed_s) {
    gs <- -p1 * z - q1 * ifelse(ent, z0, 0) - dd
    Hbs <- colSums(X * ((p2 * z + q2 * ifelse(ent, z0, 0)) / sg + (p1 + q1) / sg))
    Hss <- sum(p2 * z^2 + q2 * ifelse(ent, z0, 0)^2 + p1 * z + q1 * ifelse(ent, z0, 0))
    g <- c(g, sum(gs))
    sc <- cbind(sc, gs)
    H <- rbind(cbind(Hbb, Hbs), c(Hbs, Hss))
  }
  list(ll = ll, g = g, H = H, sc = sc)
}
stata_sl_newton <- function(f, theta) {
  cur <- f(theta)
  if (!is.finite(cur$ll)) stop("initial values not feasible (Stata: r(1400))", call. = FALSE)
  newton <- function(theta, cur, full = FALSE) {
    A <- -cur$H
    lam <- 0
    repeat {
      ch <- try(chol(A + diag(lam, nrow(A))), silent = TRUE)
      if (!inherits(ch, "try-error")) break
      lam <- if (lam == 0) 1e-6 * max(1, max(abs(diag(A)))) else lam * 10
      if (lam > 1e12) stop("convergence not achieved (Stata: r(430))", call. = FALSE)
    }
    step <- drop(chol2inv(ch) %*% cur$g)
    alpha <- 1
    repeat {
      new <- try(f(theta + alpha * step), silent = TRUE)
      if (!inherits(new, "try-error") && is.finite(new$ll) && (full || new$ll >= cur$ll - 1e-14 * abs(cur$ll))) break
      alpha <- alpha / 2
      if (alpha < 1e-14) stop("convergence not achieved (Stata: r(430))", call. = FALSE)
    }
    list(theta = theta + alpha * step, cur = new, step = alpha * step)
  }
  for (it in seq_len(500)) {
    nw <- newton(theta, cur)
    theta <- nw$theta
    cur <- nw$cur
    if (max(abs(nw$step)) < 1e-13 * max(1, max(abs(theta)))) break
    if (it == 500) stop("convergence not achieved (Stata: r(430))", call. = FALSE)
  }
  for (j in 1:2) {
    nw <- newton(theta, cur)
    theta <- nw$theta
    cur <- nw$cur
  }
  list(theta = theta, fit = cur)
}
# Gompertz: h = exp(xb + gamma t)
stata_sl_g1 <- function(u, order) {
  big <- abs(u) >= 0.5
  small <- !big
  out <- numeric(length(u))
  ub <- u[big]
  eu <- exp(ub)
  out[big] <- switch(order + 1, expm1(ub) / ub, (ub * eu - eu + 1) / ub^2, (ub^2 * eu - 2 * ub * eu + 2 * eu - 2) / ub^3)
  us <- u[small]
  tot <- rep(0, length(us))
  for (j in 0:24) {
    term <- switch(order + 1, 1 / factorial(j + 1), (j + 1) / factorial(j + 2), (j + 1) * (j + 2) / factorial(j + 3))
    tot <- tot + term * us^j
  }
  out[small] <- tot
  out
}
stata_gomp_eval <- function(theta, t, t0, dd, X) {
  k <- ncol(X)
  b <- theta[seq_len(k)]
  gam <- theta[k + 1]
  eta <- drop(X %*% b)
  E <- exp(eta)
  H <- function(tt, ord) tt^(ord + 1) * stata_sl_g1(gam * tt, ord)
  dH <- H(t, 0) - H(t0, 0)
  dH1 <- H(t, 1) - H(t0, 1)
  dH2 <- H(t, 2) - H(t0, 2)
  ll <- sum(dd * (eta + gam * t + log(t)) - E * dH)
  gb <- dd - E * dH
  gg <- sum(dd * t - E * dH1)
  Hbb <- -crossprod(X * (E * dH), X)
  Hbg <- -colSums(X * (E * dH1))
  Hgg <- -sum(E * dH2)
  list(ll = ll, g = c(colSums(X * gb), gg), H = rbind(cbind(Hbb, Hbg), c(Hbg, Hgg)), sc = cbind(X * gb, dd * t - E * dH1))
}
stata_streg <- function(d, use, info, xs, dist, aft = FALSE, nohr = FALSE, tr = FALSE, level = 95, robust = FALSE,
                        cluster = NULL, cmdline = "", show = TRUE, id = NULL) {
  num <- stata_sv_num
  full <- d
  sub <- d[use, , drop = FALSE]
  X <- if (length(xs)) do.call(cbind, lapply(xs, function(v) num(sub[[v]]))) else NULL
  X <- cbind(X, rep(1, nrow(sub)))
  colnames(X) <- c(xs, "_cons")
  t <- num(sub[["_t"]]); t0 <- num(sub[["_t0"]]); dd <- as.numeric(num(sub[["_d"]]) != 0)
  cl <- if (is.null(cluster)) NULL else sub[[cluster]]
  ok <- stats::complete.cases(X) & !is.na(t) & (if (is.null(cl)) TRUE else !is.na(cl))
  es <- logical(nrow(full))
  es[which(use)[ok]] <- TRUE
  stata_esample <<- es
  X <- X[ok, , drop = FALSE]; t <- t[ok]; t0 <- t0[ok]; dd <- dd[ok]
  if (!is.null(cl)) cl <- cl[ok]
  n <- length(t)
  k <- ncol(X)
  idv <- if (is.null(id)) NULL else sub[[id]][ok]
  nsub <- if (is.null(idv)) n else length(unique(idv))
  if (any(t <= 0)) stop("streg: a zero or negative analysis time (Stata: r(459))", call. = FALSE)
  y <- log(t)
  y0 <- ifelse(t0 > 0, log(pmax(t0, 1e-300)), -Inf)
  gomp <- dist == "gompertz"
  fixed <- dist == "exponential"
  fam <- if (gomp) NULL else stata_sl_fam(dist)
  fit <- function(Xm, theta0 = NULL) {
    if (gomp) {
      f <- function(th) stata_gomp_eval(th, t, t0, dd, Xm)
      if (is.null(theta0)) {
        f0 <- function(th) stata_sl_eval(th, y, y0, dd, Xm, stata_sl_fam("exponential"), TRUE)
        st0 <- stats::lm.fit(Xm, y)$coefficients
        r0 <- stata_sl_newton(f0, st0)
        theta0 <- c(-r0$theta, 0.001)
      }
      stata_sl_newton(f, theta0)
    } else {
      f <- function(th) stata_sl_eval(th, y, y0, dd, Xm, fam, fixed)
      if (is.null(theta0)) {
        lmf <- stats::lm.fit(Xm, y)
        theta0 <- c(lmf$coefficients, if (!fixed) log(max(stats::sd(lmf$residuals), 1e-3)))
      }
      stata_sl_newton(f, theta0)
    }
  }
  main <- fit(X)
  null <- fit(X[, k, drop = FALSE])
  theta <- main$theta
  Vth <- solve(-main$fit$H)
  ll <- main$fit$ll
  ll0 <- null$fit$ll
  # the robust or clustered variance of theta
  vtype <- "oim"
  G <- NULL
  if (robust || !is.null(cl)) {
    cc <- if (!is.null(cl)) cl else if (!is.null(idv)) idv else seq_len(n)
    G <- length(unique(cc))
    S <- rowsum(main$fit$sc, as.character(cc))
    meat <- crossprod(S)
    Vth0 <- Vth
    Vth <- Vth %*% meat %*% Vth * (if (!is.null(cl) || !is.null(idv)) G / (G - 1) else n / (n - 1))
    vtype <- if (!is.null(cl)) "cluster" else if (!is.null(idv)) "cluster" else "robust"
    if (!is.null(cl)) cluster <- cluster else if (!is.null(idv)) cluster <- id
  } else Vth0 <- NULL
  # the reported metric
  ph <- (dist %in% c("exponential", "weibull")) && !aft
  names_b <- colnames(X)
  if (gomp) {
    b <- theta[seq_len(k)]; aux <- theta[k + 1]
    J <- diag(k + 1)
    auxname <- "gamma"
  } else {
    bb <- theta[seq_len(k)]
    s <- if (fixed) 0 else theta[k + 1]
    if (ph) {
      b <- -bb * exp(-s)
      aux <- -s
      J <- diag(-exp(-s), k + (!fixed), k + (!fixed))
      if (!fixed) { J[1:k, k + 1] <- -b; J[k + 1, k + 1] <- -1; J[k + 1, 1:k] <- 0 }
    } else {
      b <- bb
      aux <- if (fixed) NA_real_ else if (dist == "weibull") -s else s
      J <- diag(k + (!fixed))
      if (!fixed && dist == "weibull") J[k + 1, k + 1] <- -1
    }
    auxname <- switch(dist, weibull = "ln_p", lognormal = "lnsigma", loglogistic = "lngamma", NA_character_)
  }
  hasaux <- !fixed
  Vr <- J %*% Vth %*% t(J)
  slope <- seq_len(k - 1)
  q <- length(slope)
  Vb <- Vr[seq_len(k), seq_len(k), drop = FALSE]
  chi2 <- if (vtype == "oim") 2 * (ll - ll0) else if (q == 0) 0 else {
    Vs <- Vb[slope, slope, drop = FALSE]
    if (qr(Vs)$rank < q) NA_real_ else drop(t(b[slope]) %*% solve(Vs, b[slope]))
  }
  dfm <- if (is.na(chi2)) 0 else q
  pchi <- if (is.na(chi2)) NA_real_ else stats::pchisq(chi2, q, lower.tail = FALSE)
  est <- if (hasaux) c(b, aux) else b
  cn <- c(paste0("_t:", names_b), if (hasaux) paste0("/:", auxname))
  Bm <- matrix(est, 1, length(est), dimnames = list("y1", cn))
  Vm <- Vr
  dimnames(Vm) <- list(cn, cn)
  title <- paste0(switch(dist, exponential = "Exponential", weibull = "Weibull", lognormal = "Lognormal", loglogistic = "Loglogistic", gompertz = "Gompertz"),
                  if (ph) " PH regression" else if (gomp) " PH regression" else " AFT regression")
  e <- list(rank = if (hasaux) k + 1 else k, N = n, k = length(est), k_eq = if (hasaux) 2 else 1, k_dv = if (fixed) 2 else 1, converged = 1, rc = 0)
  if (!is.null(G)) e$N_clust <- G
  e <- c(e, list(ll = ll, k_eq_model = 1, ll_0 = ll0))
  if (!(dist %in% c("exponential", "weibull"))) e$rank0 <- 2
  e <- c(e, list(df_m = dfm, chi2 = chi2, p = pchi))
  risk <- sum(t - t0); nfail <- sum(dd)
  if (dist == "weibull") {
    sg <- exp(theta[k + 1])
    e <- c(e, list(aux_p = 1 / sg, k_aux = 1, risk = risk, N_fail = nfail, N_sub = nsub))
  } else if (fixed) {
    e <- c(e, list(N_sub = nsub, N_fail = nfail, risk = risk))
  } else {
    e <- c(e, list(k_aux = 1, risk = risk, N_fail = nfail, N_sub = nsub))
    if (dist == "lognormal") e$sigma <- exp(theta[k + 1])
    if (dist == "loglogistic") e$gamma <- exp(theta[k + 1])
    if (gomp) e$gamma <- theta[k + 1]
  }
  cmdname <- c(exponential = "ereg", weibull = "weibull", lognormal = "lnormal", loglogistic = "llogistic", gompertz = "gompertz")[[dist]]
  e <- c(e, list(cmdline = cmdline, predict = "streg_p", cmd2 = "streg", title = title, cmd = cmdname, depvar = "_t", dead = "_d",
                 frm2 = if (ph || gomp) "hazard" else "time", t0 = "_t0", chi2type = if (vtype == "oim") "LR" else "Wald"))
  e$vce <- vtype
  if (vtype != "oim") e$vcetype <- "Robust"
  if (!is.null(cluster) && vtype == "cluster") e$clustvar <- cluster
  e <- c(e, list(properties = "b V", b = Bm, V = Vm))
  if (vtype != "oim") {
    Vm0 <- J %*% Vth0 %*% t(J)
    dimnames(Vm0) <- list(cn, cn)
    e$V_modelbased <- Vm0
  }
  e$stata_streg <- list(dist = dist, aft = aft, theta = theta, k = k, xs = xs)
  attr(e, "stata_sample") <- TRUE
  stata_e <<- e
  if (show) {
    stata_sv_head(info, id = TRUE)
    cat(title, "\n\n", sep = "")
    cat(sprintf("No. of subjects = %6s                                 Number of obs = %7d\n", formatC(nsub, big.mark = ","), n))
    cat(sprintf("No. of failures = %6s\n", formatC(nfail, big.mark = ",")))
    cat(sprintf("Time at risk    = %6s\n", formatC(risk, digits = 7, format = "g", big.mark = ",")))
    if (vtype == "oim") {
      cat(sprintf("%56sLR chi2(%d)    = %7.2f\n", "", q, chi2))
      cat(sprintf("Log likelihood = %-12s%27sProb > chi2   = %6.4f\n", format(ll, digits = 8), "", pchi))
    } else {
      cat(sprintf("%56sWald chi2(%d)  = %7.2f\n", "", q, chi2))
      cat(sprintf("Log pseudolikelihood = %-12s%22sProb > chi2   = %6.4f\n", format(ll, digits = 8), "", pchi))
    }
    cat("\n")
    ef <- (ph || gomp) && !nohr || (aft && tr)
    lab <- if (ef && (ph || gomp)) "Haz. ratio" else if (ef) "Time ratio" else "Coefficient"
    eb <- b; names(eb) <- names_b
    stata_coefs(eb, Vb, level = level / 100, eform = ef, label = lab)
    if (hasaux) {
      se <- sqrt(Vr[k + 1, k + 1])
      z <- stats::qnorm(1 - (100 - level) / 200)
      rows <- data.frame(Estimate = aux, `Std. err.` = se, lo = aux - z * se, hi = aux + z * se, check.names = FALSE)
      rownames(rows) <- paste0("/", auxname)
      cat("\n")
      print(rows, digits = 7)
      if (!gomp) {
        tr1 <- exp(aux)
        r2 <- data.frame(Estimate = c(if (dist == "weibull") c(tr1, 1 / tr1) else tr1), `Std. err.` = c(if (dist == "weibull") c(tr1 * se, se / tr1) else tr1 * se),
                         lo = exp(c(if (dist == "weibull") c(aux - z * se, -aux - z * se) else aux - z * se)),
                         hi = exp(c(if (dist == "weibull") c(aux + z * se, -aux + z * se) else aux + z * se)), check.names = FALSE)
        if (dist == "weibull") { r2$lo[2] <- exp(-(aux + z * se)); r2$hi[2] <- exp(-(aux - z * se)) }
        rownames(r2) <- switch(dist, weibull = c("p", "1/p"), lognormal = "sigma", loglogistic = "gamma")
        print(r2, digits = 7)
      }
    }
    if (ph || gomp) { if (!nohr) cat("Note: _cons estimates baseline hazard.\n") } else if (aft && tr) cat("Note: _cons estimates baseline time.\n")
  }
  invisible(NULL)
}"##;

/// The e() names that are text after `streg`.
pub(super) const STREG_TEXTS: &[&str] = &[
    "cmdline",
    "predict",
    "cmd2",
    "title",
    "cmd",
    "depvar",
    "dead",
    "frm2",
    "t0",
    "chi2type",
    "vce",
    "vcetype",
    "clustvar",
    "properties",
];

impl Session {
    #[allow(dead_code)]
    pub(super) fn streg_active(&self) -> bool {
        matches!(&self.e_ret, super::returned::Returned::Program { cmd, .. } if cmd == "streg")
    }

    pub(super) fn streg(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "streg")?;
        if let Some(o) = opts.other(&[
            ("distribution", 4),
            ("dist", 4),
            ("nohr", 4),
            ("tr", 2),
            ("time", 2),
            ("level", 3),
            ("robust", 1),
            ("vce", 3),
            ("cluster", 2),
            ("nolog", 3),
            ("noshow", 4),
            ("tolerance", 3),
            ("ltolerance", 3),
            ("nrtolerance", 3),
        ]) {
            return Err(no(format!(
                "streg with the option {o} is not translated yet"
            )));
        }
        let Some(st) = self.stset.clone() else {
            out.line("stop(\"data not st (Stata: r(119))\", call. = FALSE)");
            return Ok(());
        };
        let dist_word = opts
            .arg("distribution", 4)
            .or_else(|| opts.arg("dist", 4))
            .map(|s| s.trim().to_lowercase());
        let Some(dw) = dist_word else {
            return Err(no(
                "streg needs distribution(): exponential, weibull, lognormal, loglogistic and gompertz are translated",
            ));
        };
        let dist = ["exponential", "weibull", "lognormal", "loglogistic", "gompertz"]
            .into_iter()
            .find(|d| !dw.is_empty() && d.starts_with(dw.as_str()) && (dw.len() >= 2))
            .ok_or_else(|| {
                no(format!(
                    "streg, distribution({dw}) is not translated yet (exponential, weibull, lognormal, loglogistic and gompertz are)"
                ))
            })?;
        let tr = opts.has("tr", 2);
        let aft = opts.has("time", 2) || tr;
        if opts.has("time", 2) && tr {
            return Err(no("streg: time and tr together"));
        }
        if matches!(dist, "lognormal" | "loglogistic") && (aft || opts.has("nohr", 4)) {
            return Err(no(format!(
                "streg, distribution({dist}) with time, tr or nohr is not translated"
            )));
        }
        if dist == "gompertz" && (aft || tr) {
            return Err(no(
                "streg, distribution(gompertz) with time or tr is not translated",
            ));
        }
        if matches!(dist, "lognormal" | "loglogistic") && tr {
            return Err(no(format!(
                "streg, distribution({dist}) with tr is not translated"
            )));
        }
        if tr && !(dist == "weibull" || dist == "exponential") {
            return Err(no("streg, tr is translated for exponential and weibull"));
        }
        // the cluster variable: vce(cluster v) or cluster(v)
        let (robust, cluster) = match opts.arg("cluster", 2) {
            Some(v) if is_name(v.trim()) => (true, Some(v.trim().to_string())),
            Some(v) => return Err(no(format!("streg, cluster({v}) needs one variable name"))),
            None => super::model::vce(opts)?,
        };
        let level = super::model::level(opts)?;
        let words: Vec<&str> = c.parts.main.split_whitespace().collect();
        if words.iter().any(|w| !is_name(w)) {
            return Err(no(
                "streg: variable names written out are translated (no factor variables or operators yet)",
            ));
        }
        let xs: Vec<String> = words.iter().map(|w| w.to_string()).collect();
        let d = self.data()?;
        let cond = self.condition(c, out)?;
        let used = match cond {
            Some(cond) => {
                out.package("dplyr");
                format!("local({{ s <- dplyr::transmute({d}, s = {cond})$s; !is.na(s) & s }}) & !is.na({d}[[\"_st\"]]) & {d}[[\"_st\"]] == 1")
            }
            None => format!("!is.na({d}[[\"_st\"]]) & {d}[[\"_st\"]] == 1"),
        };
        for h in [
            "stata_survival",
            "stata_streg",
            "stata_coefs",
            "stata_eclass",
            "stata_fmt",
            "stata_returned",
        ] {
            self.helpers.insert(h);
        }
        let xl = if xs.is_empty() {
            "character(0)".to_string()
        } else {
            format!(
                "c({})",
                xs.iter().map(|x| r_text(x)).collect::<Vec<_>>().join(", ")
            )
        };
        let info = Self::st_info(&st);
        let id = st.id.as_deref().map_or("NULL".to_string(), r_text);
        out.line(format!(
            "stata_streg({d}, {used}, {info}, {xl}, {}, aft = {}, nohr = {}, tr = {}, level = {}, robust = {}, cluster = {}, cmdline = {}, show = {}, id = {id})",
            r_text(dist),
            super::stats::r_logical(aft),
            super::stats::r_logical(opts.has("nohr", 4)),
            super::stats::r_logical(tr),
            r_number(level * 100.0),
            super::stats::r_logical(robust),
            cluster.as_deref().map_or("NULL".to_string(), r_text),
            r_text(&super::returned::cmdline(c)),
            super::stats::r_logical(!self.quiet(c)),
        ));
        self.esample_new = true;
        self.e_new = true;
        self.estimates = None;
        self.estimates_lost = Some(
            "predict, test, lincom, margins and estat are not translated after streg yet".into(),
        );
        self.e_text.clear();
        self.fit_ts = None;
        self.e_ret = super::returned::Returned::Program {
            cmd: "streg".into(),
            texts: STREG_TEXTS.iter().map(|s| s.to_string()).collect(),
        };
        out.caveat(
            "streg's likelihood is maximised exactly (Newton's method, analytic derivatives): \
             Stata stops at its tolerance, so its last digits can differ",
        );
        Ok(())
    }
}
