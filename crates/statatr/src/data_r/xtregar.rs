//! `xtregar`, fixed and random effects with AR(1) disturbances, as Stata 19
//! fits them. The method is StatCraft's decoding of Stata's (its
//! `plan/stata/panel.md`, probes 334 to 417), our own work, written here in R:
//!
//! - rho from `prais` on the within-demeaned data (no constant), iterated
//!   Prais-Winsten until rho moves by less than 1e-6 (one round with
//!   `twostep`), by the `rhotype()` rule (dw by default), or `rhof()`;
//! - fe: Baltagi and Wu's transform after each panel's first observation
//!   (kept in single precision, as Stata keeps it), the panel means taken
//!   out and the grand mean put back, least squares with a constant that is
//!   then divided by 1 - rho (its standard error is not);
//! - re: every observation, the first scaled by sqrt(1 - rho^2), and the GLS
//!   step with each panel's theta.
//!
//! Weights, factor variables and time-series operators, and `predict`
//! afterwards, are not translated yet.

use super::{is_name, names, no, Opts, Out, Session};
use crate::expr_r::{r_number, r_text, Untranslated};
use crate::reader::Command;

pub(super) const XTREGAR_HELPER: &str = r##"stata_xtregar <- function(d, y, x, panel, time, delta = 1, fe = FALSE, rhotype = "dw", rhof = NULL,
                          twostep = FALSE, lbi = FALSE, level = 0.95, show = TRUE) {
  stop_ <- function(...) stop(..., call. = FALSE)
  vars <- c(y, x, panel, time)
  miss <- setdiff(vars, names(d))
  if (length(miss)) stop_("variable ", miss[1], " not found (Stata: r(111))")
  num <- function(v) { v <- unclass(v); attributes(v) <- NULL; as.numeric(v) }
  rows <- which(stats::complete.cases(as.data.frame(lapply(d[vars], num))))
  rows <- rows[order(num(d[[panel]])[rows], num(d[[time]])[rows], rows)]
  m <- length(rows)
  if (m == 0) stop_("no observations (Stata: r(2000))")
  Y <- num(d[[y]])[rows]
  X <- vapply(x, function(v) num(d[[v]])[rows], numeric(m))
  if (!is.matrix(X)) X <- matrix(X, m, dimnames = list(NULL, x))
  id <- num(d[[panel]])[rows]
  tm <- num(d[[time]])[rows] / delta
  newp <- c(TRUE, id[-1] != id[-m])
  g <- cumsum(newp)
  ng <- max(g)
  gap <- c(NA, diff(tm))
  gap[newp] <- NA
  if (any(!is.na(gap) & gap < 0.5)) stop_("repeated time values within panel (Stata: r(451))")
  has_prev <- !is.na(gap)
  dd <- ifelse(has_prev, round(gap), NA)
  pair <- has_prev & dd == 1
  ip <- which(pair)
  # least squares from the cross products, as Stata (and StatCraft) work it:
  # residuals rounded to single precision then fall the same way
  ols <- function(yv, Xm) {
    if (qr(Xm)$rank < ncol(Xm)) return(NULL)
    inv <- tryCatch(solve(crossprod(Xm)), error = function(e) NULL)
    if (is.null(inv)) return(NULL)
    b <- drop(inv %*% crossprod(Xm, yv))
    list(b = b, inv = inv, e = drop(yv - Xm %*% b))
  }
  demean <- function(z) z - stats::ave(z, g)
  yw <- demean(Y)
  Xw <- apply(X, 2, demean)
  if (!is.matrix(Xw)) Xw <- matrix(Xw, m)
  if (sum(yw^2) <= 1e-300) stop_(y, " is constant and zero (Stata: r(459))")
  # the regressors each adding something to those before them
  independent <- function(space, start = NULL) {
    basis <- start
    keep <- integer(0)
    for (j in seq_len(ncol(space))) {
      r <- space[, j]
      for (bv in basis) {
        bb <- sum(bv * bv)
        if (bb > 0) r <- r - sum(r * bv) / bb * bv
      }
      cc <- sum(space[, j]^2)
      if (cc > 0 && sum(r * r) > 1e-10 * cc) {
        basis <- c(basis, list(r))
        keep <- c(keep, j)
      }
    }
    keep
  }
  keep_w <- independent(Xw)
  keep <- if (fe) keep_w else independent(X, list(rep(1, m)))
  dropped <- setdiff(seq_along(x), keep)
  if (show) for (j in dropped) cat("note: ", x[j], " omitted because of collinearity.\n", sep = "")
  kk <- length(keep)
  if (kk == 0) stop_("xtregar with no regressor left is not translated yet")
  Xk <- X[, keep, drop = FALSE]
  Xwk <- Xw[, keep_w, drop = FALSE]
  collinear <- function() stop_(y, " collinear with ", paste(x[keep], collapse = " "), " (Stata: r(459))")
  # rho from residuals, by the rule
  rho_from <- function(e) {
    a <- sum(e[ip] * e[ip - 1]); cl <- sum(e[ip - 1]^2); cf <- sum(e[ip]^2)
    bs <- sum((e[ip] - e[ip - 1])^2); cc <- sum(e^2)
    n <- m; k <- ncol(Xwk) + 1; P <- length(ip)
    switch(rhotype,
      dw = 1 - 0.5 * bs / cc,
      regress = a / cl,
      freg = a / cf,
      tscorr = a / cc,
      theil = a / cc * (n - k) / P,
      nagar = { dw <- 1 - 0.5 * bs / cc; (dw * n^2 + k^2) / (n^2 - k^2) },
      onestep = a / cc * n / P)
  }
  rho <- if (!is.null(rhof)) rhof else {
    f0 <- ols(yw, Xwk)
    if (is.null(f0)) collinear()
    e <- f0$e
    if (sum(e^2) <= 1e-20 * sum(yw^2)) collinear()
    if (rhotype == "onestep") rho_from(e) else {
      r <- 0
      for (ic in 1:100) {
        nr <- rho_from(e)
        if (!is.finite(nr) || abs(nr) >= 1) stop_("no observations (Stata: r(2000))")
        sc <- sqrt(1 - nr^2)
        tr <- function(z) { out <- sc * z; out[ip] <- z[ip] - nr * z[ip - 1]; out }
        f1 <- ols(tr(yw), apply(Xwk, 2, tr))
        if (is.null(f1)) collinear()
        e <- drop(yw - Xwk %*% f1$b)
        done <- twostep || abs(nr - r) < 1e-6 || ic >= 100
        r <- nr
        if (done) break
      }
      r
    }
  }
  if (!is.finite(rho) || abs(rho) >= 1) stop_("no observations (Stata: r(2000))")
  # Baltagi and Wu's transform, in single precision as Stata keeps it
  bw <- function(z, first) {
    s0 <- sqrt(1 - rho^2)
    out <- rep(NA_real_, m)
    if (first) out[!has_prev] <- s0 * z[!has_prev]
    i <- which(has_prev)
    rd <- rho^dd[i]
    out[i] <- sqrt((1 - rho^2) / (1 - rd^2)) * (z[i] - rd * z[i - 1])
    stata_float(out)
  }
  pmean <- function(z, gi) stats::ave(z, gi)
  e <- list()
  if (fe) {
    u <- which(has_prev)
    ys <- bw(Y, FALSE)[u]
    Xs <- apply(Xk, 2, function(z) bw(z, FALSE)[u])
    if (!is.matrix(Xs)) Xs <- matrix(Xs, length(u))
    cs <- bw(rep(1, m), FALSE)[u]
    gu <- g[u]
    used <- unique(gu)
    n_eff <- length(used)
    wdm <- function(z) z - pmean(z, gu) + mean(z)
    yt <- wdm(ys)
    Xt <- cbind(apply(Xs, 2, wdm), 1)
    f <- ols(yt, Xt)
    if (is.null(f)) collinear()
    b <- f$b
    N <- length(u)
    df <- N - n_eff - kk
    if (df < 0) collinear()
    if (df == 0) stop_("estimates post: matrix has missing values (Stata: r(504))")
    rss <- sum(f$e^2)
    s2 <- rss / df
    V <- f$inv * s2
    b[kk + 1] <- b[kk + 1] / (1 - rho)
    tss <- sum((yt - mean(yt))^2)
    mss <- tss - rss
    r2w <- mss / tss
    xb <- drop(Xk[u, , drop = FALSE] %*% b[seq_len(kk)])
    yo <- Y[u]
    wo <- yo - xb
    wbar <- as.numeric(tapply(wo, gu, mean))
    sigma_u <- stats::sd(wbar)
    sigma_e <- sqrt(s2)
    uo <- pmean(wo, gu)
    corr_u <- stats::cor(uo, xb)
    r2b <- stats::cor(pmean(yo, gu), pmean(xb, gu))^2
    r2o <- stats::cor(yo, xb)^2
    fp <- ols(ys, cbind(Xs, cs))
    if (is.null(fp)) collinear()
    rss_p <- sum(fp$e^2)
    df_a <- n_eff - 1
    F_f <- ((rss_p - rss) / df_a) / (rss / df)
    F <- (mss / kk) / (rss / df)
    ll <- -0.5 * N * (log(2 * pi) + log(rss / N) + 1)
    ll_0 <- -0.5 * N * (log(2 * pi) + log(tss / N) + 1)
    sizes <- as.numeric(table(gu))
    ngroups <- n_eff
    small <- TRUE
  } else {
    if (ng < 2) stop_("insufficient observations (Stata: r(2001))")
    ys <- bw(Y, TRUE)
    Xs <- apply(Xk, 2, function(z) bw(z, TRUE))
    if (!is.matrix(Xs)) Xs <- matrix(Xs, m)
    cs <- bw(rep(1, m), TRUE)
    f0 <- ols(ys, cbind(Xs, cs))
    if (is.null(f0)) collinear()
    eps <- stata_float(f0$e)
    omega <- as.numeric(tapply(cs^2, g, sum))
    sce <- as.numeric(tapply(cs * eps, g, sum))
    sq <- sum(sce^2 / omega)
    s2e <- (sum(eps^2) - sq) / (m - ng)
    if (!is.finite(s2e) || s2e <= 0) collinear()
    s2u <- max(0, (sq - ng * s2e) / sum(omega))
    theta <- 1 - sqrt(s2e / (s2e + omega * s2u))
    gls <- function(z) { zs <- as.numeric(tapply(cs * z, g, sum)); z - theta[g] * zs[g] / omega[g] * cs }
    y2 <- gls(ys)
    X2 <- cbind(apply(Xs, 2, gls), gls(cs))
    f <- ols(y2, X2)
    if (is.null(f)) collinear()
    b <- f$b
    rss <- sum(f$e^2)
    V <- f$inv * (rss / (m - kk - 1))
    bs <- b[seq_len(kk)]
    chi2 <- drop(t(bs) %*% solve(V[seq_len(kk), seq_len(kk), drop = FALSE], bs))
    xb <- drop(Xk %*% bs)
    r2b <- stats::cor(as.numeric(tapply(Y, g, mean)), as.numeric(tapply(xb, g, mean)))^2
    r2o <- stats::cor(Y, xb)^2
    r2w <- stats::cor(demean(Y), demean(xb))^2
    sigma_u <- sqrt(s2u)
    sigma_e <- sqrt(s2e)
    N <- m
    sizes <- as.numeric(table(g))
    ngroups <- ng
    small <- FALSE
    thetas <- sort(theta[g])
    pct <- function(p) {
      n <- length(thetas); np <- n * p / 100; i <- floor(np)
      at <- function(k) thetas[max(1, min(n, k))]
      if (abs(np - i) < 1e-12) 0.5 * (at(i) + at(i + 1)) else at(i + 1)
    }
  }
  rho_fov <- sigma_u^2 / (sigma_u^2 + sigma_e^2)
  g_min <- min(sizes); g_max <- max(sizes)
  Tbar <- if (all(sizes == sizes[1])) sizes[1] else length(sizes) / sum(1 / sizes)
  put <- function(...) { a <- list(...); for (n in names(a)) e[[n]] <<- a[[n]] }
  lbi_v <- NULL
  if (lbi) {
    e0 <- ols(yw, Xwk)$e
    ss <- sum(e0^2)
    i <- which(has_prev)
    d1 <- sum((e0[i] - ifelse(dd[i] == 1, e0[i - 1], 0))^2)
    last <- c(newp[-1], TRUE)
    nxt_gap <- c(gap[-1], NA)
    d2 <- sum(e0[!last & !is.na(nxt_gap) & abs(nxt_gap - 1) > 1e-9]^2)
    lbi_v <- c(d1 / ss, (d1 + d2 + sum(e0[newp]^2) + sum(e0[last]^2)) / ss)
  }
  sc <- list(N = N, rank = kk + 1, r2_o = r2o, r2_b = r2b, r2_w = r2w, sigma_u = sigma_u, sigma_e = sigma_e,
             rho_fov = rho_fov, rho_ar = rho, N_g = ngroups, g_max = g_max, g_min = g_min, g_avg = N / ngroups,
             Tbar = Tbar, Tcon = as.numeric(g_min == g_max))
  if (fe) {
    sc <- c(sc, list(corr = corr_u, F = F, F_f = F_f, r2_a = 1 - (1 - r2w) * (N - 1) / df, mss = mss, ll = ll,
                     ll_0 = ll_0, rss = rss, df_r = df, df_b = kk, df_a = df_a, df_m = df_a + kk,
                     rmse = sqrt(rss / (N - kk - 1))))
    order <- if (lbi) c("d1", "LBI", "N_LBI", "df_m", "df_a", "df_b", "df_r", "rss", "ll_0", "ll", "mss", "r2_a",
                        "r2_w", "F_f", "F", "sigma_e", "rho_fov", "sigma_u", "rho_ar", "r2_o", "r2_b", "corr",
                        "rank", "Tcon", "Tbar", "rmse", "g_avg", "g_min", "g_max", "N_g", "N")
             else c("rank", "corr", "r2_b", "r2_o", "rho_ar", "sigma_u", "rho_fov", "sigma_e", "F", "F_f", "r2_w",
                    "r2_a", "mss", "ll", "ll_0", "rss", "df_r", "df_b", "df_a", "df_m", "Tcon", "Tbar", "rmse",
                    "g_avg", "g_min", "g_max", "N_g", "N")
  } else {
    sc <- c(sc, list(df_m = kk + 1, chi2 = chi2, thta_min = thetas[1], thta_5 = pct(5), thta_50 = pct(50),
                     thta_95 = pct(95), thta_max = thetas[length(thetas)]))
    order <- c(if (lbi) c("d1", "LBI", "N_LBI"), "N", "rank", "r2_o", "r2_b", "r2_w", "sigma_u", "sigma_e",
               "rho_fov", "df_m", "N_g", "g_max", "g_min", "g_avg", "Tbar", "Tcon", "chi2", "thta_min", "thta_5",
               "thta_50", "thta_95", "rho_ar", "thta_max")
  }
  if (lbi) sc <- c(sc, list(d1 = lbi_v[1], LBI = lbi_v[2], N_LBI = m))
  sc <- sc[order]
  nm <- c(x[keep], "(Intercept)")
  b <- stats::setNames(b, nm)
  dimnames(V) <- list(nm, nm)
  # the regressors left out stay in e(b), omitted
  if (length(dropped)) {
    full <- c(x, "(Intercept)")
    bf <- stats::setNames(rep(NA_real_, length(full)), full)
    bf[nm] <- b
    Vf <- matrix(0, length(full), length(full), dimnames = list(full, full))
    Vf[nm, nm] <- V
    b <- bf
    V <- Vf
  }
  if (show) {
    cat(if (fe) "FE (within)" else "RE GLS", " regression with AR(1) disturbances\n", sep = "")
    cat("Group variable: ", panel, "   Number of obs = ", N, "   Number of groups = ", ngroups, "\n", sep = "")
    cat(sprintf("R-squared: within = %.4f  between = %.4f  overall = %.4f\n", r2w, r2b, r2o))
    if (fe) cat(sprintf("F(%d,%s) = %.2f   corr(u_i, Xb) = %.4f\n", kk, format(df), F, corr_u))
    else cat(sprintf("Wald chi2(%d) = %.2f\n", kk + 1, chi2))
    cat("\n")
    ok <- !is.na(b)
    stata_coefs(b[ok], V[ok, ok, drop = FALSE], if (small) df else Inf, level)
    cat(sprintf("rho_ar = %.8f   sigma_u = %.8f   sigma_e = %.8f   rho_fov = %.8f\n", rho, sigma_u, sigma_e, rho_fov))
    if (fe) cat(sprintf("F test that all u_i=0: F(%d,%s) = %.2f\n", df_a, format(df), F_f))
    if (lbi) cat(sprintf("modified Bhargava et al. Durbin-Watson = %.7f\nBaltagi-Wu LBI = %.7f\n", lbi_v[1], lbi_v[2]))
  }
  out <- list(b = b, vcov = V, df = if (small) df else Inf, rows = rows, e = sc, fe = fe, rhotype = rhotype,
              panel = panel, time = time, depvar = y)
  invisible(out)
}
# e() after xtregar: the scalars in Stata's order, then the macros, b and V
stata_xtregar_e <- function(fit, cmdline) {
  e <- fit$e
  mac <- if (fit$fe) {
    list(cmd = "xtregar", cmdline = cmdline, dw = if ("LBI" %in% names(e)) "lbi", marginsnotok = "ue u e",
         marginsok = "XB default", predict = "xtrar_p", model = "fe", tvar = fit$time, ivar = fit$panel,
         depvar = fit$depvar, rhotype = fit$rhotype, properties = "b V")
  } else {
    list(cmd = "xtregar", cmdline = cmdline, dw = if ("LBI" %in% names(e)) "lbi", rhotype = fit$rhotype,
         tvar = fit$time, marginsnotok = "ue u e", marginsok = "XB default", predict = "xtrar_p", model = "re",
         chi2type = "Wald", ivar = fit$panel, depvar = fit$depvar, properties = "b V")
  }
  mac <- Filter(Negate(is.null), mac)
  eb <- stata_eb(fit$b, fit$vcov, "xtregar", fit$depvar)
  c(e, mac, list(b = eb$b, V = eb$V))
}"##;

impl Session {
    /// `xtregar depvar indepvars [if] [in] [, fe re rhotype() rhof() twostep lbi level()]`.
    pub(super) fn xtregar(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "xtregar")?;
        if c.parts.weight.is_some() {
            return Err(no("xtregar with weights is not translated yet"));
        }
        if let Some(o) = opts.other(&[
            ("re", 2),
            ("fe", 2),
            ("rhotype", 4),
            ("rhof", 4),
            ("twostep", 3),
            ("lbi", 3),
            ("level", 3),
        ]) {
            return Err(no(format!(
                "xtregar with the option {o} is not translated yet"
            )));
        }
        let fe = opts.has("fe", 2);
        if fe && opts.has("re", 2) {
            return Err(no("specify re or fe, not both (Stata: r(198))"));
        }
        let rhotype = match opts.arg("rhotype", 4) {
            None => "dw",
            Some(t) => {
                let t = t.trim();
                [
                    "dw", "regress", "freg", "tscorr", "theil", "nagar", "onestep",
                ]
                .into_iter()
                .find(|n| !t.is_empty() && n.starts_with(t) && (t.len() >= 2 || *n == "dw"))
                .ok_or_else(|| no(format!("option {t} not allowed (Stata: r(198))")))?
            }
        };
        let rhof = match opts.arg("rhof", 4) {
            None => None,
            Some(r) => match r.trim().parse::<f64>() {
                Ok(v) if v.abs() < 1.0 => Some(v),
                _ => return Err(no("|rho|<1, specify a valid rho (Stata: r(198))")),
            },
        };
        let vars = names(&c.parts.main).map_err(|e| {
            no(format!("{} (factor variables and time-series operators with xtregar are not translated yet)", e.reason))
        })?;
        let Some((dep, regs)) = vars.split_first() else {
            return Err(no("varlist required (Stata: r(100))"));
        };
        if regs.is_empty() {
            return Err(no("xtregar with no regressors is not translated yet"));
        }
        let panel = self.panel.clone().ok_or_else(|| {
            no(if self.ts_unknown {
                "the data were read from a file whose xtset or tsset settings are not known here; set them again with xtset"
            } else {
                "must specify panelvar and timevar; use xtset (Stata: r(459))"
            })
        })?;
        let time = self
            .panel_time
            .clone()
            .ok_or_else(|| no("must specify timevar; use xtset (Stata: r(459))"))?;
        if !is_name(&panel) || !is_name(&time) {
            return Err(no("xtregar's panel and time variables must be plain names"));
        }
        let level = match opts.arg("level", 3) {
            None => 0.95,
            Some(l) => match l.trim().parse::<f64>() {
                Ok(v) if (10.0..=99.99).contains(&v) => v / 100.0,
                _ => {
                    return Err(no(format!(
                        "level({l}) is not a valid confidence level (Stata: r(198))"
                    )))
                }
            },
        };
        let src = self.rows_for(c, out)?;
        for h in [
            "stata_xtregar",
            "stata_float",
            "stata_coefs",
            "stata_eb",
            "stata_returned",
            "stata_names",
        ] {
            self.helpers.insert(h);
        }
        let xs = regs
            .iter()
            .map(|v| r_text(v))
            .collect::<Vec<_>>()
            .join(", ");
        let delta = self.ts_delta.clone().unwrap_or_else(|| "1".into());
        let call = format!(
            "stata_xtregar({src}, {}, c({xs}), {}, {}, delta = {delta}, fe = {}, rhotype = {}, rhof = {}, twostep = {}, lbi = {}, level = {}, show = {})",
            r_text(dep),
            r_text(&panel),
            r_text(&time),
            if fe { "TRUE" } else { "FALSE" },
            r_text(rhotype),
            rhof.map_or("NULL".into(), r_number),
            if opts.has("twostep", 3) { "TRUE" } else { "FALSE" },
            if opts.has("lbi", 3) { "TRUE" } else { "FALSE" },
            r_number(level),
            if self.quiet(c) { "FALSE" } else { "TRUE" },
        );
        out.line(format!("stata_estimates <- {call}"));
        out.line("stata_b <- stata_estimates$b");
        out.line("stata_vcov <- stata_estimates$vcov");
        out.line("stata_df <- stata_estimates$df");
        out.line(format!(
            "stata_e <- stata_xtregar_e(stata_estimates, {})",
            r_text(&super::returned::cmdline(c))
        ));
        let d = self.data()?;
        out.package("dplyr");
        out.line(super::model::esample_rows_line(
            &d,
            &src,
            "stata_estimates$rows",
        ));
        self.esample_new = true;
        self.e_new = true;
        self.e_ret = super::returned::Returned::Kept {
            cmd: "xtregar".into(),
            model: if fe { "fe" } else { "re" }.into(),
            names: &[],
        };
        // predict, margins and the like after xtregar are not translated yet
        self.estimates = None;
        self.estimates_lost =
            Some("predict, margins and estat after xtregar are not translated yet".into());
        out.caveat(
            "rho, the transform and the fit as Stata's xtregar makes them (StatCraft's decoding of Stata 19); \
             printed in R's layout, not Stata's table",
        );
        Ok(())
    }
}
