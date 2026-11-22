//! `betareg depvar indepvars [if] [in] [weight] [, scale() link() slink() vce() level() nolog
//! iterate()]`: beta regression as Stata 19 fits it (`plan/stata/betareg.md`).
//!
//! - The mean is mu = g^-1(x'b) (logit, probit, cloglog, loglog) and the scale phi = h^-1(z'c)
//!   (log, root, identity); y is Beta(mu phi, (1 - mu) phi), 0 < y < 1.
//! - Stata's path is rebuilt from its log: least squares starts, a rescale of the whole vector
//!   and then of each equation (the `Initial`, `Rescale`, `Rescale eq` lines), ten BHHH
//!   iterations, then Newton-Raphson with a Hessian from central differences of the analytic
//!   scores (equation by equation).
//! - The model's variance is that Hessian's inverse, or with `vce(robust)`, `vce(cluster)` and
//!   sampling weights the sandwich. `e(ll_0)` is the constant-only fit.

use super::frac::r_vec;
use super::model::level;
use super::weights::W;
use super::*;

/// The fit, its table and the predictions.
pub(super) const BETAREG_HELPER: &str = r##"stata_beta_mean <- function(link, eta) switch(link,
  logit = { mu <- stats::plogis(eta); list(mu = mu, d = mu * (1 - mu)) },
  probit = list(mu = stats::pnorm(eta), d = stats::dnorm(eta)),
  cloglog = { e <- exp(eta); q <- exp(-e); list(mu = 1 - q, d = e * q) },
  loglog = { e <- exp(-eta); mu <- exp(-e); list(mu = mu, d = e * mu) })
stata_beta_scale <- function(slink, s) switch(slink,
  log = { phi <- exp(s); list(phi = phi, d = phi) },
  root = list(phi = s * s, d = 2 * s),
  identity = list(phi = s, d = rep(1, length(s))))
stata_beta_of <- function(link, u) switch(link, logit = log(u / (1 - u)), probit = stats::qnorm(u),
  cloglog = log(-log(1 - u)), loglog = -log(-log(u)))
stata_beta_sof <- function(slink, u) switch(slink, log = log(u), root = sqrt(u), identity = u)
stata_beta_fit <- function(link, slink, y, X, Z, w, fw, maxit = 300) {
  kx <- ncol(X); kz <- ncol(Z); p <- kx + kz; n <- length(y)
  ly <- log(y); l1y <- log(1 - y)
  pred <- function(b) list(eta = drop(X %*% b[seq_len(kx)]), s = drop(Z %*% b[kx + seq_len(kz)]))
  obs_ll <- function(eta, s) {
    m <- stata_beta_mean(link, eta); f <- stata_beta_scale(slink, s)
    a <- m$mu * f$phi; cc <- (1 - m$mu) * f$phi
    v <- suppressWarnings(lgamma(f$phi) - lgamma(a) - lgamma(cc) + (a - 1) * ly + (cc - 1) * l1y)
    v[!(a > 0 & cc > 0)] <- NaN
    v
  }
  llf <- function(b) { q <- pred(b); sum(w * obs_ll(q$eta, q$s)) }
  obs_sc <- function(eta, s) {
    m <- stata_beta_mean(link, eta); f <- stata_beta_scale(slink, s)
    a <- m$mu * f$phi; cc <- (1 - m$mu) * f$phi
    da <- digamma(a); dc <- digamma(cc)
    list(s1 = f$phi * (dc - da + ly - l1y) * m$d,
         s2 = (digamma(f$phi) - m$mu * da - (1 - m$mu) * dc + m$mu * ly + (1 - m$mu) * l1y) * f$d)
  }
  scores <- function(b) { q <- pred(b); sc <- obs_sc(q$eta, q$s); cbind(X * (w * sc$s1), Z * (w * sc$s2)) }
  # -H from central differences of the equation scores: equation l's predictor moved by (|mean| + 1e-4) 1e-4,
  # block (k, l), k >= l, from that move only
  numhess <- function(b) {
    q <- pred(b)
    h <- matrix(0, p, p)
    eqs <- list(list(X, 0), list(Z, kx))
    for (l in 1:2) {
      base <- if (l == 1) q$eta else q$s
      st <- (abs(mean(base)) + 1e-4) * 1e-4
      moved <- function(t) {
        sc <- if (l == 1) obs_sc(q$eta + t, q$s) else obs_sc(q$eta, q$s + t)
        list(w * sc$s1, w * sc$s2)
      }
      up <- moved(st); dn <- moved(-st)
      for (k in l:2) {
        dk <- (up[[k]] - dn[[k]]) / (2 * st)
        Ck <- eqs[[k]][[1]]; Cl <- eqs[[l]][[1]]
        blk <- -crossprod(Ck * dk, Cl)
        ik <- eqs[[k]][[2]] + seq_len(ncol(Ck)); il <- eqs[[l]][[2]] + seq_len(ncol(Cl))
        h[ik, il] <- blk
        h[il, ik] <- t(blk)
      }
    }
    h
  }
  once <- if (fw) w else rep(1, n)
  point <- function(b, bhhh) {
    sc <- scores(b)
    neg <- if (bhhh) crossprod(sc / sqrt(once)) else numhess(b)
    ci <- stata_frac_cinv(neg)
    list(ll = llf(b), g = colSums(sc), inv = ci$inv, nc = ci$nc, scores = sc)
  }
  rescale <- function(b, ll, idx) {
    improved <- FALSE
    for (f in c(0.5, 2)) {
      repeat {
        cb <- b; cb[idx] <- cb[idx] * f
        v <- llf(cb)
        if (!is.na(v) && v > ll) { b <- cb; ll <- v; improved <- TRUE } else break
      }
      if (improved) break
    }
    list(b = b, ll = ll, improved = improved)
  }
  # the start: least squares of g(y) on x, then of h(mu (1 - mu) / var(y) - 1) on z
  gy <- stata_beta_of(link, y)
  beta <- as.numeric(stats::lm.wfit(X, gy, w)$coefficients)
  if (fw) { tw <- sum(w); yb <- sum(w * y) / tw; v <- sum(w * (y - yb)^2) / (tw - 1) } else v <- stats::var(y)
  b <- c(beta, rep(0, kz))
  mu <- stata_beta_mean(link, pred(b)$eta)$mu
  tt <- stata_beta_sof(slink, mu * (1 - mu) / v - 1)
  gamma <- as.numeric(stats::lm.wfit(Z, tt, w)$coefficients)
  b <- c(beta, gamma)
  ll <- llf(b); initial <- ll
  r <- rescale(b, ll, seq_len(p)); b <- r$b; ll <- r$ll
  after <- ll
  repeat {
    any_ <- FALSE
    for (idx in list(seq_len(kx), kx + seq_len(kz))) { r <- rescale(b, ll, idx); b <- r$b; ll <- r$ll; any_ <- any_ || r$improved }
    if (!any_) break
  }
  search <- c(initial, after, ll)
  rd <- function(a, b) abs(a - b) / (abs(b) + 1)
  lg <- numeric(); ncf <- logical(); bkf <- logical()
  prev <- NULL; backed <- FALSE; conv <- FALSE
  pt <- point(b, TRUE)
  repeat {
    lg <- c(lg, pt$ll); ncf <- c(ncf, pt$nc); bkf <- c(bkf, backed)
    step <- drop(pt$inv %*% pt$g)
    if (!is.null(prev) && !pt$nc) {
      nr <- sum(pt$g * step)
      if (abs(nr) <= 1e-5 && (rd(pt$ll, prev$ll) <= 1e-7 || max(rd(b, prev$b)) <= 1e-6)) { conv <- TRUE; break }
    }
    if (length(lg) > maxit) break
    t <- 1; nx <- llf(b + t * step); halved <- FALSE
    while ((is.na(nx) || nx < pt$ll) && t > 1e-15) { halved <- TRUE; t <- t / 2; nx <- llf(b + t * step) }
    backed <- halved && (is.na(nx) || nx < pt$ll)
    if (!halved) {
      extra <- 0.125
      repeat {
        fur <- llf(b + (t + extra) * step)
        if (isTRUE(fur - nx > 1e-12 * max(abs(nx), 1))) { t <- t + extra; nx <- fur; extra <- extra * 2 } else break
      }
    }
    prev <- list(b = b, ll = pt$ll)
    b <- b + t * step
    pt <- point(b, length(lg) < 10)
  }
  list(b = b, ll = llf(b), log = lg, nc = ncf, backed = bkf, search = search, converged = conv, numhess = numhess, scores = scores,
       llf = llf)
}
stata_betareg <- function(d, dep, terms, scale = character(), link = "logit", slink = "log", wvar = NULL, wtype = NULL, wtext = "",
                          robust = FALSE, cluster = NULL, level = 95, nolog = FALSE, maxit = 300, cmdline = "", show = TRUE) {
  L <- stata_frac_load(d, dep, terms, scale, wvar, cluster)
  y <- L$y; w <- L$w; n <- L$n
  if (any(!(y > 0 & y < 1))) stop(dep, " must be greater than 0 and less than 1 (Stata: r(459))", call. = FALSE)
  fw <- identical(wtype, "f")
  N <- if (fw) sum(w) else n
  X <- L$X; Z <- cbind(L$Z, 1); kx <- ncol(X); kz <- ncol(Z); p <- kx + kz
  znames <- c(L$znames, "_cons")
  fit <- stata_beta_fit(link, slink, y, X, Z, w, fw, maxit)
  b <- fit$b
  H <- fit$numhess(b); Ainv <- solve(H)
  V <- if (robust) stata_frac_sym(stata_frac_sandwich(Ainv, fit$scores(b), w, fw, L$cl)) else Ainv
  null <- stata_beta_fit(link, slink, y, X[, kx, drop = FALSE], matrix(1, n, 1), w, fw, maxit)
  ll <- fit$ll; ll0 <- null$llf(null$b)
  slopes <- c(seq_len(kx - 1), kx + seq_len(kz - 1))
  chi2 <- if (robust) stata_frac_wald(b, V, slopes) else 2 * (ll - ll0)
  df_m <- length(slopes)
  ll_label <- if (robust) "Log pseudolikelihood" else "Log likelihood"
  lt <- switch(link, logit = c("log(u/(1-u))", "Logit"), probit = c("invnormal(u)", "Probit"),
               cloglog = c("log(-log(1-u))", "Comp. log-log"), loglog = c("-log(-log(u))", "Log-log"))
  st <- switch(slink, log = c("log(u)", "Log"), root = c("sqrt(u)", "Square root"), identity = c("u", "Identity"))
  ind <- match(L$xnames, L$full)
  nf <- length(L$full) + kz
  pos <- c(ind, length(L$full) + seq_len(kz))
  cn <- c(paste0(dep, ":", L$full), paste0("scale:", znames))
  bF <- numeric(nf); bF[pos] <- b
  VF <- matrix(0, nf, nf); VF[pos, pos] <- V
  e <- list(rank = p, N = N, ic = length(fit$log) - 1, k = nf, k_eq = 2, k_dv = 1, converged = as.numeric(fit$converged), rc = 0)
  if (!is.null(L$cl)) e$N_clust <- length(unique(L$cl))
  e$ll <- ll
  if (!robust || !is.null(wvar)) e$ll_0 <- ll0
  e <- c(e, list(k_eq_model = 2, df_m = df_m, chi2 = chi2, p = stats::pchisq(chi2, df_m, lower.tail = FALSE)))
  e$cmd <- "betareg"
  e$chi2type <- if (robust) "Wald" else "LR"
  e$marginsnotok <- "stdp SCores"
  e$marginsok <- "default CMean CVARiance xb XBSCAle"
  e$predict <- "betareg_p"
  e$cmdline <- cmdline
  e$depvar <- dep
  if (robust) e$vcetype <- "Robust"
  if (!is.null(cluster)) e$clustvar <- cluster
  e$vce <- if (!is.null(cluster)) "cluster" else if (robust) "robust" else "oim"
  if (!is.null(wtype)) { e$wexp <- wtext; e$wtype <- switch(wtype, f = "fweight", p = "pweight", i = "iweight") }
  e$title <- "Beta regression"
  e$slinkt <- st[2]; e$slinkf <- st[1]; e$linkt <- lt[2]; e$linkf <- lt[1]
  e$deriv_useminbound <- "off"
  e$opt <- "moptimize"
  e$user <- "_betall()"
  e$ml_method <- "lf1"
  e$technique <- "bhhh 10 nr 1000"
  e$which <- "max"
  e$properties <- "b V"
  e$b <- matrix(bF, 1, nf, dimnames = list("y1", cn))
  e$V <- `dimnames<-`(VF, list(cn, cn))
  il <- numeric(20); m <- min(20, length(fit$log)); il[seq_len(m)] <- fit$log[seq_len(m)]
  e$ilog <- matrix(il, 1, 20, dimnames = list("r1", paste0("c", 1:20)))
  gF <- numeric(nf); gF[pos] <- colSums(fit$scores(b))
  e$gradient <- matrix(gF, 1, nf, dimnames = list("r1", cn))
  if (robust) { VmF <- matrix(0, nf, nf); VmF[pos, pos] <- stata_frac_sym(Ainv); e$V_modelbased <- `dimnames<-`(VmF, list(cn, cn)) }
  attr(e, "stata_rows") <- L$rows
  rname <- c(L$colnames[-length(L$colnames)], "_cons")
  rname[rname == "_cons"] <- "(Intercept)"
  attr(e, "stata_b") <- stats::setNames(b[seq_len(kx)], rname)
  attr(e, "stata_V") <- { Vn <- V[seq_len(kx), seq_len(kx), drop = FALSE]; dimnames(Vn) <- list(rname, rname); Vn }
  attr(e, "stata_areg") <- list(fo = L$fo, xlev = L$xlev, nmx = L$xnames[-length(L$xnames)], znames = L$znames, link = link, slink = slink, dep = dep)
  if (show) {
    g10 <- function(v) formatC(stata_fmt(v, "%10.0g", trim = TRUE), width = 10)
    if (!nolog) {
      for (i in 1:3) cat(formatC(c("Initial:", "Rescale:", "Rescale eq:")[i], width = -14), ll_label, " = ", g10(fit$search[i]), "\n", sep = "")
      cat("(setting technique to bhhh)\n")
      for (i in seq_along(fit$log)) {
        if (i == 11) cat("(switching technique to nr)\n")
        cat(formatC(paste0("Iteration ", i - 1, ":"), width = -14), ll_label, " = ", g10(fit$log[i]), "  ",
            if (fit$nc[i]) "(not concave)" else if (fit$backed[i]) "(backed up)" else "", "\n", sep = "")
      }
    }
    cat("\n")
    right <- function(label, value) sprintf("%-18s= %10s", label, value)
    cat(formatC("Beta regression", width = -48), right("Number of obs", N), "\n", sep = "")
    cat(strrep(" ", 48), right(sprintf("%s chi2(%d)", if (robust) "Wald" else "LR", df_m), sprintf("%.2f", chi2)), "\n", sep = "")
    cat(strrep(" ", 48), right("Prob > chi2", sprintf("%.4f", e$p)), "\n\n", sep = "")
    wl <- nchar(ll_label); field <- 48 - (wl + 4)
    cat(formatC("Link function", width = -wl), " :  ", formatC(paste0("g(u) = ", lt[1]), width = -field), "[", lt[2], "]\n", sep = "")
    cat(formatC("Slink function", width = -wl), " :  ", formatC(paste0("g(u) = ", st[1]), width = -field), "[", st[2], "]\n\n", sep = "")
    cat(ll_label, " = ", stata_fmt(ll, "%10.0g"), "\n\n", sep = "")
    if (!is.null(L$cl)) cat(stata_tab_pad(sprintf("(Std. err. adjusted for %s clusters in %s)", stata_tab_count(length(unique(L$cl))), cluster), 78), "\n", sep = "")
    stata_frac_tophead(dep, "Coefficient", robust, level)
    se <- sqrt(diag(V))
    cat(formatC(stata_tab_abbrev(dep), width = -13), "|\n", sep = "")
    stata_frac_rows_print(L$xnames, b[seq_len(kx)], se[seq_len(kx)], level, FALSE, L$mf)
    cat(strrep("-", 13), "+", strrep("-", 64), "\n", sep = "")
    cat(formatC("scale", width = -13), "|\n", sep = "")
    stata_frac_rows_print(znames, b[kx + seq_len(kz)], se[kx + seq_len(kz)], level)
    cat(strrep("-", 78), "\n", sep = "")
    if (!fit$converged) cat("Warning: Convergence not achieved.\n")
  }
  e
}
stata_betareg_predict <- function(e, d, what) {
  a <- attr(e, "stata_areg"); b <- e$b
  X <- if (is.null(a$fo)) matrix(0, nrow(d), 0) else {
    mf <- stats::model.frame(a$fo, d, xlev = a$xlev, na.action = stats::na.pass)
    X <- stats::model.matrix(a$fo, mf)
    X[, colnames(X) != "(Intercept)", drop = FALSE]
  }
  xb <- if (ncol(X)) drop(X %*% b[1, paste0(a$dep, ":", a$nmx)]) else rep(0, nrow(d))
  eta <- xb + b[1, paste0(a$dep, ":_cons")]
  s <- b[1, "scale:_cons"] + (if (length(a$znames)) Reduce(`+`, lapply(a$znames, function(v) stata_frac_num(d[[v]]) * b[1, paste0("scale:", v)])) else 0)
  switch(what,
    xb = eta, xbscale = s,
    cmean = stata_beta_mean(a$link, eta)$mu,
    cvariance = { mu <- stata_beta_mean(a$link, eta)$mu; mu * (1 - mu) / (1 + stata_beta_scale(a$slink, s)$phi) },
    stdp = {
      V <- e$V; dep <- a$dep
      cols <- paste0(dep, ":", c(a$nmx, "_cons"))
      Xf <- cbind(X, 1)
      sqrt(rowSums((Xf %*% V[cols, cols, drop = FALSE]) * Xf))
    })
}"##;

impl Session {
    /// `betareg depvar indepvars [if] [in] [weight] [, scale() link() slink() vce() level()
    /// nolog iterate()]`.
    pub(super) fn betareg(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "betareg")?;
        if let Some(o) = opts.other(&[
            ("scale", 3),
            ("link", 2),
            ("slink", 3),
            ("vce", 3),
            ("robust", 1),
            ("level", 3),
            ("nolog", 3),
            ("iterate", 4),
        ]) {
            return Err(no(format!(
                "betareg with the option {o} is not translated yet"
            )));
        }
        let Some(w) = self.frac_weight(c, &[W::F, W::P, W::I], out)? else {
            return Ok(());
        };
        let spec = self.frac_spec(c, "betareg")?;
        let scale: Vec<String> = match opts.arg("scale", 3) {
            Some(h) => {
                let v: Vec<String> = h.split_whitespace().map(str::to_string).collect();
                if v.iter().any(|t| !is_name(t)) {
                    return Err(no(
                        "scale() with factor variables or operators is not translated yet",
                    ));
                }
                v
            }
            None => Vec::new(),
        };
        let link = match opts.arg("link", 2).as_deref().map(str::trim) {
            None => "logit",
            Some(t) if t.len() >= 2 && "logit".starts_with(t) => "logit",
            Some(t) if t.len() >= 4 && "probit".starts_with(t) => "probit",
            Some(t) if t.len() >= 4 && "cloglog".starts_with(t) => "cloglog",
            Some(t) if t.len() >= 4 && "loglog".starts_with(t) => "loglog",
            Some(t) => return Err(no(format!("link({t}) not allowed"))),
        };
        let slink = match opts.arg("slink", 3).as_deref().map(str::trim) {
            None | Some("log") => "log",
            Some("root") => "root",
            Some(t) if t.len() >= 4 && "identity".starts_with(t) => "identity",
            Some(t) => return Err(no(format!("slink({t}) not allowed"))),
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
                    _ => return Err(no(format!("betareg with vce({v}) is not translated yet"))),
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
            "stata_lsfam",
            "stata_fmt",
            "stata_tables",
            "stata_returned",
            "stata_eclass",
            "stata_frac",
            "stata_betareg",
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
            "stata_e <- stata_betareg({src}, {}, c({}), scale = {}, link = {}, slink = {}{wargs}{cl}, robust = {}, level = {}, nolog = {}, maxit = {maxit}, cmdline = {}, show = {})",
            r_text(&spec.dep),
            spec.terms.iter().map(|t| r_text(t)).collect::<Vec<_>>().join(", "),
            r_vec(&scale),
            r_text(link),
            r_text(slink),
            super::stats::r_logical(robust),
            r_number(level * 100.0),
            super::stats::r_logical(opts.has("nolog", 3)),
            r_text(&super::returned::cmdline(c)),
            super::stats::r_logical(show)
        ));
        self.frac_keep(
            c,
            out,
            &src,
            "betareg",
            &spec,
            &[
                "cmd",
                "chi2type",
                "marginsnotok",
                "marginsok",
                "predict",
                "cmdline",
                "depvar",
                "vcetype",
                "clustvar",
                "vce",
                "wexp",
                "wtype",
                "title",
                "slinkt",
                "slinkf",
                "linkt",
                "linkf",
                "deriv_useminbound",
                "opt",
                "user",
                "ml_method",
                "technique",
                "which",
                "properties",
            ],
        )?;
        out.caveat(
            "betareg as Stata 19 reports it: Stata's own path (least squares starts, the rescale lines, ten BHHH iterations, \
             then Newton-Raphson on a Hessian from central differences of the analytic scores); e(ll_0) differs from Stata's \
             in the seventh digit (Stata's constant-only fit stops short of the maximum)",
        );
        Ok(())
    }
}
