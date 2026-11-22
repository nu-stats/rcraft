//! `boxcox depvar [indepvars] [if] [in] [, model() notrans() lrtest nolog nologlr level()
//! iterate()]`: Box-Cox regression as Stata 19 reports it (`plan/stata/boxcox.md`).
//!
//! - The likelihood has beta and sigma profiled out: for the transform parameters p (theta on y,
//!   lambda on the transformed regressors) ll = -n/2 (1 + ln 2 pi + ln(RSS/n)) + (theta - 1)
//!   sum ln y, RSS from least squares of z(theta) on the untransformed columns, the constant and
//!   the transformed ones.
//! - Newton-Raphson on the parameters (a full model started at 1, the per-variable LR fits at 0),
//!   the derivatives taken by central differences of the profile; e(V) is zero except the
//!   transform parameters' block. Stata's own numerical Hessian moves its path in the seventh
//!   digit, so the estimates agree to about 1e-6, not to the last digit.
//! - Weights are refused as Stata refuses them.

use super::frac::r_vec;
use super::model::level;
use super::weights::W;
use super::*;

/// The fit, its tables and the predictions.
pub(super) const BOXCOX_HELPER: &str = r##"stata_bc_tr <- function(v, t) if (t == 0) log(v) else expm1(t * log(v)) / t
stata_bc_split <- function(model, p) switch(model, lhsonly = list(lam = 1, theta = p[1]), rhsonly = list(lam = p[1], theta = NULL),
  lambda = list(lam = p[1], theta = p[1]), theta = list(lam = p[1], theta = p[2]))
# the regression at the transform parameters p
stata_bc_prof <- function(y, N, Tr, model, p) {
  s <- stata_bc_split(model, p)
  W <- cbind(N, if (ncol(Tr)) apply(Tr, 2, stata_bc_tr, t = s$lam) else NULL)
  z <- if (is.null(s$theta)) y else stata_bc_tr(y, s$theta)
  f <- stats::lm.fit(W, z)
  rss <- sum(f$residuals^2); n <- length(y)
  jac <- if (is.null(s$theta)) 0 else (s$theta - 1) * sum(log(y))
  list(ll = -n / 2 * (1 + log(2 * pi) + log(rss / n)) + jac, beta = as.numeric(f$coefficients), rss = rss, resid = as.numeric(f$residuals), z = z)
}
# Newton-Raphson on the profile, derivatives from central differences
stata_bc_fit <- function(y, N, Tr, model, p0, maxit = 300) {
  llf <- function(p) stata_bc_prof(y, N, Tr, model, p)$ll
  np <- length(p0)
  derivs <- function(p) {
    h <- 1e-3 * pmax(1, abs(p))
    g <- numeric(np); H <- matrix(0, np, np)
    f0 <- llf(p)
    for (i in seq_len(np)) {
      e <- function(k) { q <- p; q[i] <- q[i] + k * h[i]; q }
      g[i] <- (llf(e(-2)) - 8 * llf(e(-1)) + 8 * llf(e(1)) - llf(e(2))) / (12 * h[i])
      H[i, i] <- (-llf(e(2)) + 16 * llf(e(1)) - 30 * f0 + 16 * llf(e(-1)) - llf(e(-2))) / (12 * h[i]^2)
    }
    if (np == 2) {
      f <- function(a, b) { q <- p; q[1] <- q[1] + a * h[1]; q[2] <- q[2] + b * h[2]; llf(q) }
      H[1, 2] <- H[2, 1] <- (f(1, 1) - f(1, -1) - f(-1, 1) + f(-1, -1)) / (4 * h[1] * h[2])
    }
    list(ll = f0, g = g, negH = -H, scores = NULL)
  }
  m <- stata_ml_max(llf, derivs, p0, maxit)
  list(p = m$b, ll = m$ll, log = m$log, converged = m$converged, V = solve(m$negH))
}
stata_bc_log <- function(lg) {
  for (i in seq_along(lg)) cat(formatC(paste0("Iteration ", i - 1, ":"), width = -14), "Log likelihood = ",
                               formatC(stata_fmt(lg[i], "%10.0g", trim = TRUE), width = 10), "  \n", sep = "")
}
stata_boxcox <- function(d, dep, regs, model = "lhsonly", notrans = character(), lrtest = FALSE, nolog = FALSE, nologlr = FALSE,
                         level = 95, maxit = 300, cmdline = "", show = TRUE) {
  num <- stata_frac_num
  untr <- if (model == "lhsonly") c(notrans, regs) else notrans
  tr <- if (model == "lhsonly") character() else regs
  allv <- unique(c(dep, untr, tr))
  for (v in allv) if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
  ok <- stats::complete.cases(d[allv])
  if (!any(ok)) stop("no observations (Stata: r(2000))", call. = FALSE)
  rows <- which(ok)
  mf <- d[rows, , drop = FALSE]
  n <- nrow(mf); y <- num(mf[[dep]])
  pos <- function(v) all(num(mf[[v]]) > 0)
  if (model != "rhsonly" && !pos(dep)) stop(dep, " contains observations that are not strictly positive (Stata: r(411))", call. = FALSE)
  for (v in tr) if (!pos(v)) stop(v, " contains observations that are not strictly positive (Stata: r(411))", call. = FALSE)
  mat <- function(vs) if (length(vs)) sapply(vs, function(v) num(mf[[v]])) else matrix(0, n, 0)
  Nm <- cbind(mat(untr), 1)
  Tm <- mat(tr)
  if (!is.matrix(Tm)) Tm <- matrix(Tm, n)
  np <- if (model == "theta") 2 else 1
  say <- function(...) if (show) cat(...)
  lg <- function(f) if (show && !nolog) { stata_bc_log(f$log); cat("\n") }
  # the comparison model: y by theta, constant only; rhsonly has none
  if (model != "rhsonly") {
    f0 <- stata_bc_fit(y, matrix(1, n, 1), matrix(0, n, 0), "lhsonly", 1, maxit)
    say("Fitting comparison model\n\n"); lg(f0)
    ll0 <- f0$ll; df_r <- 3
  } else {
    say("\n")
    ll0 <- stata_bc_prof(y, matrix(1, n, 1), matrix(0, n, 0), "rhsonly", 1)$ll; df_r <- 2
  }
  full <- stata_bc_fit(y, Nm, Tm, model, rep(1, np), maxit)
  say("Fitting full model\n\n"); lg(full)
  prof <- stata_bc_prof(y, Nm, Tm, model, full$p)
  lr <- list()
  if (lrtest) {
    say("Fitting comparison models for LR tests\n\n")
    nu <- ncol(Nm) - 1
    for (j in seq_len(nu + ncol(Tm))) {
      if (j <= nu) { f <- stata_bc_fit(y, Nm[, -j, drop = FALSE], Tm, model, rep(0, np), maxit); nm <- untr[j] }
      else { f <- stata_bc_fit(y, Nm, Tm[, -(j - nu), drop = FALSE], model, rep(0, np), maxit); nm <- tr[j - nu] }
      if (show && !nologlr) { stata_bc_log(f$log); cat("\n") }
      s <- 2 * (full$ll - f$ll)
      lr[[nm]] <- c(s, stats::pchisq(s, 1, lower.tail = FALSE))
    }
  }
  nu <- ncol(Nm) - 1; nt <- ncol(Tm); nb <- nu + 1 + nt
  df_m <- nb + np + 1
  chi2 <- 2 * (full$ll - ll0); cdf <- df_m - df_r
  pv <- stats::pchisq(chi2, cdf, lower.tail = FALSE)
  sigma <- sqrt(prof$rss / n)
  pnames <- switch(model, lhsonly = "theta", rhsonly = "lambda", lambda = "lambda", theta = c("lambda", "theta"))
  rest <- list()
  for (v in c(-1, 0, 1)) {
    llv <- stata_bc_prof(y, Nm, Tm, model, rep(v, np))$ll
    s <- 2 * (full$ll - llv)
    rest[[as.character(v)]] <- c(llv, s, 1 - stats::pchisq(s, 1))
  }
  pfx <- function(p, v) if (length(v)) paste0(p, v) else character()
  cn <- c(pfx("Notrans:", untr), "Notrans:_cons", pfx("Trans:", tr), paste0(pnames, ":_cons"), "sigma:_cons")
  kt <- length(cn)
  bF <- c(prof$beta, full$p, sigma)
  VF <- matrix(0, kt, kt); VF[nb + seq_len(np), nb + seq_len(np)] <- full$V
  e <- list(N = n, df_m = df_m)
  if (model == "lhsonly") e <- c(e, list(ll0 = ll0, df_r = df_r, chi2 = chi2)) else e <- c(e, list(df_r = df_r, chi2 = chi2, ll0 = ll0))
  e$ll <- full$ll
  for (k in list(c("t1", "1"), c("tm1", "-1"), c("t0", "0"))) {
    r <- rest[[k[2]]]
    e[[paste0("chi2_", k[1])]] <- r[2]; e[[paste0("p_", k[1])]] <- r[3]; e[[paste0("ll_", k[1])]] <- r[1]
  }
  e <- c(e, list(ic = length(full$log) - 1, rc = 0, rank = np))
  e$cmd <- "boxcox"; e$marginsnotok <- "_ALL"
  if (model == "lhsonly") { e$cmdline <- cmdline; e$ntrans <- "yes"; e$model <- model } else { e$model <- model; e$cmdline <- cmdline; e$ntrans <- "yes" }
  e$predict <- "boxcox_p"; e$chi2type <- "LR"
  if (lrtest) e$lrtest <- "lrtest"
  e$depvar <- dep; e$properties <- "b V"
  e$b <- matrix(bF, 1, kt, dimnames = list("y1", cn))
  e$V <- `dimnames<-`(VF, list(cn, cn))
  if (lrtest) {
    nmn <- paste0("Notrans:", names(lr))
    e$pm <- matrix(vapply(lr, `[`, 0, 2), 1, length(lr), dimnames = list("r1", nmn))
    e$df <- matrix(1, 1, length(lr), dimnames = list("r1", nmn))
    e$chi2m <- matrix(vapply(lr, `[`, 0, 1), 1, length(lr), dimnames = list("r1", nmn))
  }
  attr(e, "stata_rows") <- rows
  attr(e, "stata_b") <- stats::setNames(prof$beta, c(untr, "(Intercept)", tr))
  attr(e, "stata_V") <- matrix(0, nb, nb, dimnames = list(c(untr, "(Intercept)", tr), c(untr, "(Intercept)", tr)))
  attr(e, "stata_bc") <- list(model = model, untr = untr, tr = tr, resid = prof$resid, dep = dep)
  if (show) {
    g9 <- function(v) stata_fmt(v, "%9.0g", trim = FALSE)
    pad <- stata_tab_pad
    stat <- function(label, value) sprintf("%-16s= %10s", label, value)
    cat(strrep(" ", 50), stat("Number of obs", n), "\n", sep = "")
    cat(strrep(" ", 50), stat(sprintf("LR chi2(%d)", cdf), sprintf("%.2f", chi2)), "\n", sep = "")
    cat(formatC(paste0("Log likelihood = ", trimws(stata_fmt(full$ll, "%10.0g"))), width = -50), stat("Prob > chi2", sprintf("%.3f", pv)), "\n", sep = "")
    cat(" \n")
    stata_frac_tophead(dep, "Coefficient", FALSE, level)
    qz <- stats::qnorm((1 + level / 100) / 2)
    for (i in seq_len(np)) {
      b <- full$p[i]; se <- sqrt(full$V[i, i]); z <- b / se
      cat(pad(paste0("/", pnames[i]), 12), " |", pad(stata_tab_g(b), 11), pad(stata_tab_g(se), 11), pad(sprintf("%.2f", z), 9),
          pad(sprintf("%.3f", 2 * stats::pnorm(-abs(z))), 8), pad(stata_tab_g(b - qz * se), 13), pad(stata_tab_g(b + qz * se), 12), "\n", sep = "")
    }
    cat(strrep("-", 78), "\n \n", sep = "")
    cat("Estimates of scale-variant parameters\n")
    wide <- lrtest
    rule <- strrep("-", if (wide) 61 else 28)
    mid <- if (wide) "-------------+-----------------------------------------------" else "-------------+--------------"
    cat(rule, "\n", if (wide) "             | Coefficient chi2(df)  P>chi2(df)    df of chi2" else "             | Coefficient", "\n", mid, "\n", sep = "")
    cat(formatC("Notrans", width = -13), "|\n", sep = "")
    for (j in seq_len(nu)) {
      cat(pad(stata_tab_abbrev(untr[j]), 12), " |", pad(g9(prof$beta[j]), 11), sep = "")
      if (!is.null(lr[[untr[j]]])) cat(pad(sprintf("%.3f", lr[[untr[j]]][1]), 10), pad(sprintf("%.3f", lr[[untr[j]]][2]), 9), pad(1, 11), sep = "")
      cat("\n")
    }
    cat(pad("_cons", 12), " |", pad(g9(prof$beta[nu + 1]), 11), "\n", sep = "")
    if (nt > 0) {
      cat(mid, "\n", formatC("Trans", width = -13), "|\n", sep = "")
      for (j in seq_len(nt)) {
        cat(pad(stata_tab_abbrev(tr[j]), 12), " |", pad(g9(prof$beta[nu + 1 + j]), 11), sep = "")
        if (!is.null(lr[[tr[j]]])) cat(pad(sprintf("%.3f", lr[[tr[j]]][1]), 10), pad(sprintf("%.3f", lr[[tr[j]]][2]), 8), pad(1, 12), sep = "")
        cat("\n")
      }
    }
    cat(mid, "\n", pad("/sigma", 12), " |", pad(g9(sigma), 11), "\n", rule, "\n", sep = "")
    cat(if (model == "theta" || lrtest) " \n" else "\n", sep = "")
    th <- model == "theta"
    width <- if (th) 63 else 57; col <- if (th) 32 else 26
    label <- switch(model, lhsonly = "theta", rhsonly = "lambda", lambda = "lambda", theta = "theta=lambda")
    cat(strrep("-", width), "\n", sep = "")
    if (th) cat("   Test               Restricted     \n    H0:             log likelihood       chi2       Prob > chi2\n")
    else cat("   Test         Restricted     LR statistic\n    H0:       log likelihood       chi2       Prob > chi2\n")
    cat(strrep("-", width), "\n", sep = "")
    for (v in c(-1, 0, 1)) {
      r <- rest[[as.character(v)]]
      left <- sprintf("%s = %2d", label, v)
      cat(left, pad(trimws(stata_fmt(r[1], "%10.0g")), col - nchar(left)), pad(sprintf("%.2f", r[2]), 13), pad(sprintf("%.3f", r[3]), 16), "\n", sep = "")
    }
    cat(strrep("-", width), "\n", sep = "")
  }
  e
}
stata_boxcox_predict <- function(e, d, what, smear, back) {
  a <- attr(e, "stata_bc"); b <- e$b
  num <- stata_frac_num
  lam <- if ("lambda:_cons" %in% colnames(b)) b[1, "lambda:_cons"] else 1
  theta <- switch(a$model, rhsonly = NULL, lambda = lam, if ("theta:_cons" %in% colnames(b)) b[1, "theta:_cons"] else NULL)
  fit <- b[1, "Notrans:_cons"] + rep(0, nrow(d))
  for (v in a$untr) fit <- fit + num(d[[v]]) * b[1, paste0("Notrans:", v)]
  for (v in a$tr) { x <- num(d[[v]]); fit <- fit + b[1, paste0("Trans:", v)] * ifelse(x > 0, stata_bc_tr(ifelse(x > 0, x, 1), lam), NA_real_) }
  untr <- function(u, t) { if (t == 0) return(exp(u)); base <- t * u + 1; ifelse(base > 0, base^(1 / t), NA_real_) }
  yhat <- if (is.null(theta)) fit else if (back) untr(fit, theta) else vapply(fit, function(f) {
    if (is.na(f)) return(NA_real_)
    v <- untr(f + a$resid, theta)
    if (any(is.na(v)) || !length(v)) NA_real_ else mean(v)
  }, 0)
  if (what == "residuals") num(d[[a$dep]]) - yhat else yhat
}"##;

impl Session {
    /// `boxcox depvar [indepvars] [if] [in] [, model() notrans() lrtest nolog nologlr level()
    /// iterate()]`.
    pub(super) fn boxcox(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "boxcox")?;
        if let Some(o) = opts.other(&[
            ("model", 5),
            ("notrans", 5),
            ("lrtest", 6),
            ("nolog", 3),
            ("nologlr", 7),
            ("level", 3),
            ("iterate", 4),
        ]) {
            return Err(no(format!(
                "boxcox with the option {o} is not translated yet"
            )));
        }
        let Some(w) = self.frac_weight(c, &[W::F, W::I], out)? else {
            return Ok(());
        };
        if w.is_some() {
            return Err(no(
                "boxcox with fweights or iweights is not translated yet (Stata takes them; aweights and pweights it refuses)",
            ));
        }
        let spec = self.frac_spec(c, "boxcox")?;
        if spec.terms.iter().any(|t| !is_name(t)) {
            return Err(no("boxcox with factor variables is not translated yet"));
        }
        let model = match opts.arg("model", 5).as_deref().map(str::trim) {
            None => "lhsonly",
            Some(t) if t.len() >= 3 && "lhsonly".starts_with(t) => "lhsonly",
            Some(t) if t.len() >= 3 && "rhsonly".starts_with(t) => "rhsonly",
            Some(t) if t.len() >= 3 && "lambda".starts_with(t) => "lambda",
            Some("theta") => "theta",
            Some(t) => return Err(no(format!("model({t}) invalid"))),
        };
        let notrans: Vec<String> = match opts.arg("notrans", 5) {
            Some(h) => {
                let v: Vec<String> = h.split_whitespace().map(str::to_string).collect();
                if v.iter().any(|t| !is_name(t)) {
                    return Err(no("notrans() with operators is not translated yet"));
                }
                v
            }
            None => Vec::new(),
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
        let src = self.weighted_rows(rows, &None);
        for h in [
            "stata_ivprobit",
            "stata_lsfam",
            "stata_fmt",
            "stata_tables",
            "stata_returned",
            "stata_eclass",
            "stata_frac",
            "stata_boxcox",
        ] {
            self.helpers.insert(h);
        }
        let show = !self.quiet(c);
        out.line(format!(
            "stata_e <- stata_boxcox({src}, {}, {}, model = {}, notrans = {}, lrtest = {}, nolog = {}, nologlr = {}, level = {}, maxit = {maxit}, cmdline = {}, show = {})",
            r_text(&spec.dep),
            r_vec(&spec.terms),
            r_text(model),
            r_vec(&notrans),
            super::stats::r_logical(opts.has("lrtest", 6)),
            super::stats::r_logical(opts.has("nolog", 3)),
            super::stats::r_logical(opts.has("nologlr", 7)),
            r_number(level * 100.0),
            r_text(&super::returned::cmdline(c)),
            super::stats::r_logical(show)
        ));
        self.frac_keep(
            c,
            out,
            &src,
            "boxcox",
            &spec,
            &[
                "cmd",
                "marginsnotok",
                "cmdline",
                "ntrans",
                "model",
                "predict",
                "chi2type",
                "lrtest",
                "depvar",
                "properties",
            ],
        )?;
        out.caveat(
            "boxcox as Stata 19 reports it: the profile likelihood in the transform parameters by Newton-Raphson on central \
             differences; Stata's own numerical Hessian moves its path in the seventh digit, so estimates agree to about 1e-6 \
             and the iteration log can differ in its last digits",
        );
        Ok(())
    }
}
