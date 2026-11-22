//! Zero-inflated and zero-truncated counts as Stata 19 reports them (`plan/stata/fraccount.md`):
//! `zip` and `zinb` (`inflate()`), `tpoisson` and `tnbreg`, and their old names `ztp` and `ztnb`.
//!
//! - The scores are analytic and Stata's `-H` is the central difference of the gradient (step
//!   1e-5 max(|b|, 1)); the steps are Stata's Newton-Raphson (`stata_ml_max`).
//! - `zip` and `zinb` fit the constant-only count model first (the constant at ln ybar, the
//!   inflation equation at zero, ln alpha at zero) and the full model from there; their
//!   LR test counts the other parameters but one in e(p) (a quirk of Stata's, probe 162).
//! - `tpoisson` starts from the Poisson fit to the truncated sample; `tnbreg` goes on through
//!   the truncated Poisson fit with ln alpha from the constant-only fit and, as Stata's
//!   estimates sit at the optimum, polishes the result with a second run.
//! - Not matched: Stata's own start of the full model of `tnbreg` and `ztnb` (its iteration 0 is
//!   higher), so those logs differ from Stata's from their first line.

use super::frac::r_vec;
use super::model::level;
use super::weights::W;
use super::*;

/// The fits, their tables and the predictions.
pub(super) const ZCOUNT_HELPER: &str = r##"# ln f(y), d/d eta and d/d theta (theta = ln alpha) of the negative binomial (mean mu, m = 1/alpha)
stata_zc_nb <- function(y, eta, theta) {
  m <- exp(-theta); mu <- exp(eta); mm <- m + mu
  ymax <- max(y)
  if (all(y == floor(y)) && ymax < 1e5) {
    j <- m + seq_len(ymax + 1) - 1
    lg <- c(0, cumsum(log(j)))[y + 1]; dg <- c(0, cumsum(1 / j))[y + 1]
  } else {
    lg <- lgamma(y + m) - lgamma(m); dg <- digamma(y + m) - digamma(m)
  }
  ll <- lg - lgamma(y + 1) + y * eta - y * log(mm) - m * log1p(mu / m)
  d_m <- dg - log1p(mu / m) + (mu - y) / mm
  list(ll = ll, s_eta = (y - mu) * m / mm, s_th = -m * d_m)
}
stata_zc_obs <- function(which, y, X, Z, b) {
  kx <- ncol(X); zi <- which %in% c("zip", "zinb"); kz <- if (zi) ncol(Z) else 0
  nb <- which %in% c("zinb", "tnbreg")
  eta <- drop(X %*% b[seq_len(kx)])
  th <- if (nb) b[length(b)] else 0
  if (nb) {
    f1 <- stata_zc_nb(y, eta, th); f0 <- stata_zc_nb(rep(0, length(y)), eta, th)
    lf <- f1$ll; df_eta <- f1$s_eta; df_th <- f1$s_th; l0 <- f0$ll; d0_eta <- f0$s_eta; d0_th <- f0$s_th
  } else {
    mu <- exp(eta)
    lf <- y * eta - mu - lgamma(y + 1); df_eta <- y - mu; df_th <- 0; l0 <- -mu; d0_eta <- -mu; d0_th <- 0
  }
  f0v <- exp(l0)
  if (zi) {
    g <- drop(Z %*% b[kx + seq_len(kz)]); pi <- stats::plogis(g)
    z0 <- y == 0
    lik <- pi + (1 - pi) * f0v
    ll <- ifelse(z0, log(lik), log(1 - pi) + lf)
    s_eta <- ifelse(z0, (1 - pi) * f0v * d0_eta / lik, df_eta)
    s_g <- ifelse(z0, pi * (1 - pi) * (1 - f0v) / lik, -pi)
    S <- cbind(X * s_eta, Z * s_g, if (nb) ifelse(z0, (1 - pi) * f0v * d0_th / lik, df_th))
  } else {
    ll <- lf - log1p(-f0v); k <- f0v / (1 - f0v)
    S <- cbind(X * (df_eta + k * d0_eta), if (nb) df_th + k * d0_th)
  }
  list(ll = ll, S = S)
}
stata_zc_fit <- function(which, y, X, Z, w, start, maxit = 300, tol = 1e-6, ltol = 1e-7, nrtol = 1e-5) {
  np <- length(start)
  obs <- function(b) { o <- stata_zc_obs(which, y, X, Z, b); list(ll = w * o$ll, S = o$S * w) }
  llf <- function(b) sum(obs(b)$ll)
  grad <- function(b) colSums(obs(b)$S)
  derivs <- function(b) {
    o <- obs(b); g <- colSums(o$S)
    H <- matrix(0, np, np)
    for (j in seq_len(np)) {
      h <- 1e-5 * max(abs(b[j]), 1); up <- b; dn <- b; up[j] <- up[j] + h; dn[j] <- dn[j] - h
      H[, j] <- -(grad(up) - grad(dn)) / (2 * h)
    }
    list(ll = sum(o$ll), g = g, negH = (H + t(H)) / 2, scores = o$S)
  }
  m <- stata_ml_max(llf, derivs, start, maxit, tol, ltol, nrtol)
  m$llf <- llf
  m
}
stata_zc_say <- function(lg, label) stata_frac_log(lg, label)
stata_zcount <- function(d, cmd, dep, terms, inflate = character(), wvar = NULL, wtype = NULL, wtext = "", cluster = NULL, robust = FALSE,
                         irr = FALSE, level = 95, nolog = FALSE, maxit = 300, tol = 1e-6, ltol = 1e-7, nrtol = 1e-5,
                         cmdline = "", show = TRUE) {
  which <- switch(cmd, zip = "zip", zinb = "zinb", tpoisson = "tpoisson", ztp = "tpoisson", tnbreg = "tnbreg", ztnb = "tnbreg")
  zi <- which %in% c("zip", "zinb"); nb <- which %in% c("zinb", "tnbreg")
  L <- stata_frac_load(d, dep, terms, inflate, wvar, cluster)
  y <- L$y; w <- L$w; n <- L$n
  fw <- identical(wtype, "f")
  X <- L$X; Z <- if (zi) cbind(L$Z, 1) else matrix(0, n, 0)
  kx <- ncol(X); kz <- ncol(Z); k <- kx - 1
  if (any(y < 0)) stop(dep, " must be greater than or equal to zero (Stata: r(459))", call. = FALSE)
  if (!zi && any(y == 0)) stop(dep, " must be greater than zero (Stata: r(459))", call. = FALSE)
  N <- if (fw) sum(w) else n
  n_zero <- if (fw) sum(w[y == 0]) else sum(y == 0)
  ybar <- sum(w * y) / sum(w)
  X0 <- X[, kx, drop = FALSE]
  fitf <- function(X, Z, start) stata_zc_fit(which, y, X, Z, w, start, maxit, tol, ltol, nrtol)
  ll_c <- NULL; plog <- NULL; clog <- NULL
  if (zi) {
    cf <- fitf(X0, Z, c(log(ybar), rep(0, kz), if (nb) 0))
    full <- fitf(X, Z, c(rep(0, k), cf$b))
    ll0 <- cf$ll; clog <- cf$log
  } else {
    X0 <- X[, kx, drop = FALSE]
    pois <- suppressWarnings(stats::glm.fit(X, y, weights = w, family = stats::poisson()))
    if (which == "tpoisson") {
      full <- fitf(X, Z, as.numeric(pois$coefficients))
      ll0 <- stata_zc_fit(which, y, X0, Z, w, log(ybar), 0)$llf(log(ybar))
    } else {
      cf <- fitf(X0, Z, c(log(ybar), 0))
      tp <- stata_zc_fit("tpoisson", y, X, Z, w, as.numeric(pois$coefficients), maxit, tol, ltol, nrtol)
      full <- fitf(X, Z, c(tp$b, cf$b[length(cf$b)]))
      pol <- fitf(X, Z, full$b)
      full$b <- pol$b; full$negH <- pol$negH; full$ll <- pol$ll; full$scores <- pol$scores; full$g <- pol$g
      ll0 <- cf$ll; ll_c <- tp$ll; plog <- tp$log; clog <- cf$log
    }
  }
  b <- full$b; np <- length(b)
  Ainv <- solve(full$negH)
  V <- if (robust) stata_frac_sandwich(Ainv, full$scores, w, fw, L$cl) else Ainv
  V <- stata_frac_sym(V)
  ll <- full$ll
  chi2 <- if (robust) stata_frac_wald(b, V, seq_len(k)) else 2 * (ll - ll0)
  sampling <- identical(wtype, "p")
  # the columns: the count equation, inflate, /lnalpha
  cn <- c(paste0(dep, ":", L$full), if (zi) paste0("inflate:", c(L$znames, "_cons")), if (nb) "/:lnalpha")
  ind <- match(L$xnames, L$full)
  nf <- length(cn); extra <- nf - length(L$full)
  pos <- c(ind, length(L$full) + seq_len(extra))
  bF <- numeric(nf); bF[pos] <- b
  VF <- matrix(0, nf, nf); VF[pos, pos] <- V
  VmF <- matrix(0, nf, nf); VmF[pos, pos] <- stata_frac_sym(Ainv)
  gF <- numeric(nf); gF[pos] <- full$g
  ic <- length(full$log) - 1
  e <- list(rank = np, N = N, ic = ic, k = nf, k_eq = if (zi) (if (nb) 3 else 2) else (if (nb) 2 else 1), k_dv = 1,
            converged = as.numeric(full$converged), rc = 0)
  if (!is.null(L$cl)) e$N_clust <- length(unique(L$cl))
  e$ll <- ll; e$k_eq_model <- 1
  if (!(which == "tpoisson" && sampling)) e$ll_0 <- ll0
  p_lr <- function(df) stats::pchisq(chi2, df, lower.tail = FALSE)
  if (zi) {
    e <- c(e, list(chi2 = chi2, p = p_lr(if (robust) k else np - 1), N_zero = n_zero, df_m = k))
    if (nb) e$k_aux <- 1
  } else if (which == "tpoisson") {
    e <- c(e, list(df_m = k, chi2 = chi2, p = p_lr(k), r2_p = if (sampling) NA_real_ else 1 - ll / ll0))
  } else {
    chi2_c <- 2 * (ll - ll_c)
    ord <- list(df_m = k, chi2 = chi2, p = p_lr(k), r2_p = 1 - ll / ll0, k_aux = 1, alpha = exp(b[np]), ll_c = ll_c, chi2_c = chi2_c,
                p_c = 0.5 * stats::pchisq(chi2_c, 1, lower.tail = FALSE))
    if (cmd %in% c("ztnb", "tnbreg")) {
      e <- c(e, ord[c("df_m", "chi2", "p")])
      e <- append(e, list(rank0 = 2), after = match("ll_0", names(e)))
      e <- c(e, ord[c("ll_c", "chi2_c", "alpha", "k_aux", "r2_p")])
    } else e <- c(e, ord)
  }
  title <- switch(cmd, zip = "Zero-inflated Poisson regression", zinb = "Zero-inflated negative binomial regression",
                  tpoisson = "Truncated Poisson regression", ztp = "Zero-truncated Poisson regression",
                  tnbreg = "Truncated negative binomial regression", ztnb = "Zero-truncated negative binomial regression")
  usr <- switch(cmd, zip = "zip_llf", zinb = "zinb_llf", tpoisson = , ztp = "tpoiss_d2", tnbreg = "tnbreg_mean", ztnb = "trnb_mean")
  pred <- switch(cmd, zip = , zinb = "zip_p", tpoisson = "tpoisson_p", ztp = "ztp_p", tnbreg = "tnbreg_p", ztnb = "ztnb_p")
  e$cmdline <- cmdline
  if (cmd == "ztnb") { e$cmd <- cmd; e$predict <- pred; e$dispers <- "mean"; e$chi2_ct <- "LR" }
  else if (cmd == "tnbreg") { e$title <- title; e$llopt <- "0"; e$cmd <- cmd; e$predict <- pred; e$dispers <- "mean"; e$chi2_ct <- "LR" }
  else if (!zi) { e$title <- title; e$llopt <- "0"; e$cmd <- cmd; e$predict <- pred }
  else { e$cmd <- cmd; e$predict <- pred; e$inflate <- "logit" }
  if (!is.null(wtype)) { e$wexp <- paste0("= ", wtext); e$wtype <- switch(wtype, f = "fweight", p = "pweight", i = "iweight") }
  e$chi2type <- if (robust) "Wald" else "LR"
  e$deriv_useminbound <- "off"; e$opt <- "moptimize"
  if (robust) e$vcetype <- "Robust"
  if (!is.null(cluster)) e$clustvar <- cluster
  e$vce <- if (!is.null(cluster)) "cluster" else if (robust) "robust" else "oim"
  if (zi || cmd == "ztnb") e$title <- title
  e$user <- usr; e$ml_method <- "e2"; e$technique <- "nr"; e$which <- "max"; e$depvar <- dep; e$properties <- "b V"
  e$b <- matrix(bF, 1, nf, dimnames = list("y1", cn))
  e$V <- `dimnames<-`(VF, list(cn, cn))
  il <- numeric(20); m <- min(20, length(full$log)); il[seq_len(m)] <- full$log[seq_len(m)]
  e$ilog <- matrix(il, 1, 20, dimnames = list("r1", paste0("c", 1:20)))
  e$gradient <- matrix(gF, 1, nf, dimnames = list("r1", cn))
  if (robust) e$V_modelbased <- `dimnames<-`(VmF, list(cn, cn))
  attr(e, "stata_rows") <- L$rows
  rname <- c(L$colnames[-length(L$colnames)], "_cons"); rname[rname == "_cons"] <- "(Intercept)"
  attr(e, "stata_b") <- stats::setNames(b[seq_len(kx)], rname)
  attr(e, "stata_V") <- { Vn <- V[seq_len(kx), seq_len(kx), drop = FALSE]; dimnames(Vn) <- list(rname, rname); Vn }
  attr(e, "stata_areg") <- list(fo = L$fo, xlev = L$xlev, nmx = L$xnames[-length(L$xnames)], dep = dep, cmd = cmd)
  if (show) {
    ll_label <- if (robust) "Log pseudolikelihood" else "Log likelihood"
    pad <- stata_tab_pad
    if (!nolog) {
      cat("\n")
      if (which == "tnbreg") {
        cat(if (cmd == "ztnb") "Fitting Zero-truncated poisson model:" else "Fitting truncated Poisson model:", "\n\n", sep = "")
        stata_zc_say(plog, ll_label); cat("\n")
        cat("Fitting constant-only model:\n\n"); stata_zc_say(clog, ll_label); cat("\n")
        cat("Fitting full model:\n\n")
      }
      if (zi) { cat("Fitting constant-only model:\n\n"); stata_zc_say(clog, ll_label); cat("\n"); cat("Fitting full model:\n\n") }
      stata_zc_say(full$log, ll_label)
    }
    cat("\n")
    wv <- max(6, nchar(c(stata_tab_count(N), sprintf("%.2f", chi2))))
    stat <- function(label, value) sprintf("%-13s = %s", label, pad(value, wv))
    chi2_label <- sprintf("%s chi2(%d)", if (robust) "Wald" else "LR", k)
    probs <- sprintf("%.4f", stats::pchisq(chi2, k, lower.tail = FALSE))
    cnt <- function(v) stata_tab_count(v)
    if (zi) {
      cat(formatC(title, width = -56), stat("Number of obs", cnt(N)), "\n", sep = "")
      cat(formatC("Inflation model: logit", width = -56), stat("Nonzero obs", cnt(N - n_zero)), "\n", sep = "")
      cat(strrep(" ", 56), stat("Zero obs", cnt(n_zero)), "\n", sep = "")
      cat(strrep(" ", 56), stat(chi2_label, sprintf("%.2f", chi2)), "\n", sep = "")
      cat(formatC(paste0(ll_label, " = ", trimws(stata_fmt(ll, "%9.0g"))), width = -56), stat("Prob > chi2", probs), "\n", sep = "")
    } else if (which == "tpoisson" && cmd == "ztp") {
      cat(formatC(title, width = -56), stat("Number of obs", cnt(N)), "\n", sep = "")
      cat(strrep(" ", 56), stat(chi2_label, sprintf("%.2f", chi2)), "\n", sep = "")
      cat(strrep(" ", 56), stat("Prob > chi2", probs), "\n", sep = "")
      cat(formatC(paste0(ll_label, " = ", trimws(stata_fmt(ll, "%10.0g"))), width = -56), stat("Pseudo R2", sprintf("%.4f", e$r2_p)), "\n", sep = "")
    } else if (which == "tpoisson") {
      right <- function(label, value) sprintf("%-18s= %10s", label, value)
      wl <- nchar(ll_label)
      lower <- sprintf("%-*s = %10s", wl, sprintf("Limits:%*s", wl - 7, "lower"), "0")
      upper <- sprintf("%*s = %10s", wl, "upper", "+inf")
      llline <- paste0(ll_label, " = ", stata_fmt(ll, "%10.0g"))
      nt <- right("Number of obs", cnt(N)); ct <- right(chi2_label, sprintf("%.2f", chi2)); pt <- right("Prob > chi2", probs)
      if (is.null(e$ll_0)) {
        cat(title, "\n", formatC(lower, width = -48), nt, "\n", formatC(upper, width = -48), ct, "\n", formatC(llline, width = -48), pt, "\n", sep = "")
      } else {
        cat(formatC(title, width = -48), nt, "\n", formatC(lower, width = -48), ct, "\n", formatC(upper, width = -48), pt, "\n",
            formatC(llline, width = -48), right("Pseudo R2", sprintf("%.4f", e$r2_p)), "\n", sep = "")
      }
    } else {
      cat(formatC(title, width = -56), stat("Number of obs", cnt(N)), "\n", sep = "")
      cat(formatC(if (cmd == "ztnb") "" else "Truncation point = 0", width = -56), stat(chi2_label, sprintf("%.2f", chi2)), "\n", sep = "")
      cat(formatC("Dispersion: mean", width = -56), stat("Prob > chi2", probs), "\n", sep = "")
      cat(formatC(paste0(ll_label, " = ", trimws(stata_fmt(ll, "%10.0g"))), width = -56), stat("Pseudo R2", sprintf("%.4f", e$r2_p)), "\n", sep = "")
    }
    cat("\n")
    if (!is.null(L$cl)) cat(pad(sprintf("(Std. err. adjusted for %s clusters in %s)", cnt(length(unique(L$cl))), cluster), 78), "\n", sep = "")
    stata_frac_tophead(dep, if (irr) "IRR" else "Coefficient", robust, level)
    se <- sqrt(diag(V))
    mid <- paste0(strrep("-", 13), "+", strrep("-", 64), "\n")
    if (zi) cat(formatC(stata_tab_abbrev(dep), width = -13), "|\n", sep = "")
    stata_frac_rows_print(L$xnames, b[seq_len(kx)], se[seq_len(kx)], level, irr, L$mf)
    if (zi) {
      cat(mid, formatC("inflate", width = -13), "|\n", sep = "")
      zn <- c(L$znames, "_cons")
      stata_frac_rows_print(zn, b[kx + seq_len(kz)], se[kx + seq_len(kz)], level)
    }
    crit <- stats::qnorm((100 + level) / 200)
    g9 <- function(v) stata_fmt(v, "%9.0g", trim = FALSE)
    if (nb) {
      la <- b[np]; sl <- se[np]
      cat(mid)
      if (which == "zinb") {
        z <- la / sl
        cat(pad("/lnalpha", 12), " |", pad(stata_tab_g(la), 11), pad(stata_tab_g(sl), 11), pad(sprintf("%.2f", z), 9),
            pad(sprintf("%.3f", 2 * stats::pnorm(-abs(z))), 8), pad(stata_tab_g(la - crit * sl), 13), pad(stata_tab_g(la + crit * sl), 12), "\n", sep = "")
      } else {
        cat(pad("/lnalpha", 12), " |", pad(g9(la), 11), pad(g9(sl), 11), pad("", 17), pad(g9(la - crit * sl), 13), pad(g9(la + crit * sl), 12), "\n", sep = "")
      }
      cat(mid)
      a <- exp(la)
      cat(pad("alpha", 12), " |", pad(g9(a), 11), pad(g9(a * sl), 11), pad("", 17), pad(g9(exp(la - crit * sl)), 13), pad(g9(exp(la + crit * sl)), 12), "\n", sep = "")
    }
    cat(strrep("-", 78), "\n", sep = "")
    if (which == "tnbreg") {
      cc <- e$chi2_c
      shown <- if (cc < 0.005) "0.0e+00" else sprintf("%.2f", cc)
      pc <- 0.5 * stats::pchisq(cc, 1, lower.tail = FALSE)
      if (cmd == "ztnb") cat(sprintf("Likelihood-ratio test of alpha=0:  chibar2(01) = %7.2f Prob>=chibar2 = %.3f\n", cc, pc))
      else cat(formatC(paste0("LR test of alpha=0: chibar2(01) = ", shown), width = -55), sprintf("Prob >= chibar2 = %.3f\n", pc), sep = "")
    }
    if (irr) {
      if (zi) cat("Note: Estimates are transformed only in the first equation to incidence-rate\n      ratios.\n")
      cat("Note: _cons estimates baseline incidence rate.\n")
    }
    if (!full$converged) cat("Warning: Convergence not achieved.\n")
  }
  e
}
stata_zc_predict <- function(e, d, what) {
  a <- attr(e, "stata_areg"); b <- e$b
  X <- if (is.null(a$fo)) matrix(0, nrow(d), 0) else {
    mf <- stats::model.frame(a$fo, d, xlev = a$xlev, na.action = stats::na.pass)
    X <- stats::model.matrix(a$fo, mf)
    X[, colnames(X) != "(Intercept)", drop = FALSE]
  }
  xb <- (if (ncol(X)) drop(X %*% b[1, paste0(a$dep, ":", a$nmx)]) else rep(0, nrow(d))) + b[1, paste0(a$dep, ":_cons")]
  if (what == "xb") xb else exp(xb)
}"##;

impl Session {
    /// `zip`/`zinb depvar indepvars, inflate(varlist)`, `tpoisson`/`tnbreg`/`ztp`/`ztnb depvar indepvars`
    /// `[if] [in] [weight] [, irr nolog level() vce() robust iterate() tolerance() ltolerance()
    /// nrtolerance()]`.
    pub(super) fn zcount(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let cmd = c.name.as_str();
        self.no_by(c, cmd)?;
        let zi = matches!(cmd, "zip" | "zinb");
        let mut known: Vec<(&str, usize)> = vec![
            ("irr", 3),
            ("level", 3),
            ("nolog", 3),
            ("iterate", 4),
            ("vce", 3),
            ("robust", 1),
            ("tolerance", 3),
            ("ltolerance", 3),
            ("nrtolerance", 3),
        ];
        if zi {
            known.push(("inflate", 3));
        }
        if let Some(o) = opts.other(&known) {
            return Err(no(format!(
                "{cmd} with the option {o} is not translated yet"
            )));
        }
        let Some(w) = self.frac_weight(c, &[W::F, W::P, W::I], out)? else {
            return Ok(());
        };
        let spec = self.frac_spec(c, cmd)?;
        let inflate: Vec<String> = if zi {
            match opts.arg("inflate", 3) {
                None => return Err(no("option inflate() required")),
                Some(h) => {
                    let v: Vec<String> = h
                        .split(|ch: char| ch.is_whitespace() || ch == ',')
                        .filter(|t| !t.is_empty() && *t != "_cons")
                        .map(str::to_string)
                        .collect();
                    if h.contains(',') || v.iter().any(|t| !is_name(t)) {
                        return Err(no(
                            "inflate() with options, factor variables or operators is not translated yet",
                        ));
                    }
                    v
                }
            }
        } else {
            Vec::new()
        };
        let mut robust = opts.has("robust", 1) || matches!(&w, Some((W::P, _)));
        let cluster = match opts.arg("vce", 3) {
            None => None,
            Some(v) => {
                let v = v.trim().to_string();
                match v.split_once(char::is_whitespace) {
                    Some((k, rest))
                        if k.len() >= 2 && "cluster".starts_with(k) && is_name(rest.trim()) =>
                    {
                        robust = true;
                        Some(rest.trim().to_string())
                    }
                    _ if v == "robust" || v == "r" => {
                        robust = true;
                        None
                    }
                    _ if v == "oim" => None,
                    _ => return Err(no(format!("{cmd} with vce({v}) is not translated yet"))),
                }
            }
        };
        let irr = opts.has("irr", 3);
        let num = |name: &str, min: usize, dflt: &str| -> Result<String, Untranslated> {
            match opts.arg(name, min) {
                None => Ok(dflt.to_string()),
                Some(t) => {
                    let t = t.trim();
                    t.parse::<f64>()
                        .map_err(|_| no(format!("option {name}() incorrectly specified")))?;
                    Ok(t.to_string())
                }
            }
        };
        let maxit = num("iterate", 4, "300")?;
        let tol = num("tolerance", 3, "1e-6")?;
        let ltol = num("ltolerance", 3, "1e-7")?;
        let nrtol = num("nrtolerance", 3, "1e-5")?;
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
            "stata_zcount",
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
        out.line(format!(
            "stata_e <- stata_zcount({src}, {}, {}, {}, inflate = {}{wargs}{cl}, robust = {}, irr = {}, level = {}, nolog = {}, maxit = {maxit}, tol = {tol}, ltol = {ltol}, nrtol = {nrtol}, cmdline = {}, show = {})",
            r_text(cmd),
            r_text(&spec.dep),
            r_vec(&spec.terms),
            r_vec(&inflate),
            super::stats::r_logical(robust),
            super::stats::r_logical(irr),
            r_number(level * 100.0),
            super::stats::r_logical(opts.has("nolog", 3)),
            r_text(&super::returned::cmdline(c)),
            super::stats::r_logical(show)
        ));
        let texts: &[&str] = match cmd {
            "tnbreg" => &[
                "cmdline",
                "title",
                "llopt",
                "cmd",
                "predict",
                "dispers",
                "chi2_ct",
                "wexp",
                "wtype",
                "chi2type",
                "deriv_useminbound",
                "opt",
                "vcetype",
                "clustvar",
                "vce",
                "user",
                "ml_method",
                "technique",
                "which",
                "depvar",
                "properties",
            ],
            "ztnb" => &[
                "cmdline",
                "cmd",
                "predict",
                "dispers",
                "chi2_ct",
                "wexp",
                "wtype",
                "chi2type",
                "deriv_useminbound",
                "opt",
                "vcetype",
                "clustvar",
                "vce",
                "title",
                "user",
                "ml_method",
                "technique",
                "which",
                "depvar",
                "properties",
            ],
            "zip" | "zinb" => &[
                "cmdline",
                "cmd",
                "predict",
                "inflate",
                "wexp",
                "wtype",
                "chi2type",
                "deriv_useminbound",
                "opt",
                "vcetype",
                "clustvar",
                "vce",
                "title",
                "user",
                "ml_method",
                "technique",
                "which",
                "depvar",
                "properties",
            ],
            _ => &[
                "cmdline",
                "title",
                "llopt",
                "cmd",
                "predict",
                "wexp",
                "wtype",
                "chi2type",
                "deriv_useminbound",
                "opt",
                "vcetype",
                "clustvar",
                "vce",
                "user",
                "ml_method",
                "technique",
                "which",
                "depvar",
                "properties",
            ],
        };
        self.frac_keep(c, out, &src, cmd, &spec, texts)?;
        out.caveat(
            "as Stata 19 reports it: analytic scores, -H from central differences of the gradient, Stata's Newton-Raphson steps; \
             for tnbreg and ztnb Stata's own start of the full model is not matched, so their iteration logs differ from Stata's",
        );
        Ok(())
    }
}
