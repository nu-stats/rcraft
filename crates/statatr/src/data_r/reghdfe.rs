//! P2C.9 continued, `reghdfe` (the SSC package), as it behaves on Stata 19
//! with made-up data (`plan/stata/reghdfe.md`); only its results were read,
//! never its code.
//!
//! - Rows alone in a group of an absorbed variable (singletons) are dropped,
//!   again until none are left, unless `keepsingletons`.
//! - The slopes are least squares on the data with the absorbed means taken
//!   out; `_cons` is the fit with the grand means added back.
//! - Degrees of freedom: an absorbed variable counts its levels, the second
//!   one less one for each connected group the two make; one nested in a
//!   cluster variable counts none, and then the constant counts one. K is
//!   the slopes plus those.
//! - The variance: s^2 (X'X)^-1 with s^2 = rss / (N - K); robust times
//!   N / (N - K); clustered times (N - 1) / (N - K) G / (G - 1); two-way, the
//!   two less their intersection, all three with G the smaller count, and
//!   Cameron, Gelbach and Miller's fix when that is not positive. Clustered,
//!   t tests on G - 1.

use super::model::{level, Estimates, Fit, Spec};
use super::*;

/// The fit, its variance, its e() and Stata's header.
pub(super) const REGHDFE_HELPER: &str = r#"stata_reghdfe <- function(formula, data, absorb = character(), cluster = character(),
                          robust = FALSE, keepsingletons = FALSE, notes = TRUE) {
  vars <- unique(c(all.vars(formula), absorb, cluster))
  ok <- stats::complete.cases(data[vars])
  rows <- which(ok)
  N_full <- length(rows)
  # singletons, again until none are left
  dropped <- 0
  if (!keepsingletons && length(absorb)) {
    repeat {
      alone <- rep(FALSE, length(rows))
      for (a in absorb) {
        g <- as.character(data[[a]][rows])
        alone <- alone | stats::ave(seq_along(g), g, FUN = length) == 1
      }
      if (!any(alone)) break
      dropped <- dropped + sum(alone)
      rows <- rows[!alone]
    }
    if (notes && dropped > 0) cat("(dropped ", dropped, " singleton observations)\n", sep = "")
  }
  d <- data[rows, , drop = FALSE]
  mf <- stats::model.frame(formula, d, na.action = stats::na.pass)
  tt <- attr(mf, "terms")
  y <- as.numeric(stats::model.response(mf))
  X <- stats::model.matrix(tt, mf)
  X <- X[, colnames(X) != "(Intercept)", drop = FALSE]
  N <- length(y)
  fes <- lapply(absorb, function(a) as.character(d[[a]]))
  within <- function(v) {
    if (!length(fes)) return(v - mean(v))
    for (i in 1:100000) {
      o <- v
      for (f in fes) v <- v - stats::ave(v, f)
      if (max(abs(v - o)) < 1e-13 * max(1, max(abs(v)))) break
    }
    v
  }
  Xw <- apply(X, 2, within)
  if (!is.matrix(Xw)) Xw <- matrix(Xw, nrow = N, dimnames = list(NULL, colnames(X)))
  yw <- within(y)
  # a variable the absorbed ones explain is left out, as reghdfe does
  q <- qr(Xw, tol = 1e-9)
  keep <- rep(FALSE, ncol(X))
  keep[q$pivot[seq_len(q$rank)]] <- TRUE
  if (notes) for (j in which(!keep)) cat("note: ", colnames(X)[j], " is probably collinear with the fixed effects (all partialled-out values are close to zero; tol = 1.0e-09)\n", sep = "")
  K <- sum(keep)
  mx <- colMeans(X[, keep, drop = FALSE])
  Xa <- cbind(Xw[, keep, drop = FALSE] + rep(mx, each = N), 1)
  ya <- yw + mean(y)
  B <- solve(crossprod(Xa))
  b <- drop(B %*% crossprod(Xa, ya))
  e <- drop(ya - Xa %*% b)
  # the absorbed degrees of freedom
  nested <- vapply(absorb, function(a) {
    length(cluster) > 0 && any(vapply(cluster, function(cv) {
      all(tapply(as.character(d[[cv]]), as.character(d[[a]]), function(v) length(unique(v))) == 1)
    }, TRUE))
  }, TRUE)
  levels_of <- vapply(fes, function(f) length(unique(f)), 0)
  components <- function(f1, f2) {
    lab <- match(f1, unique(f1))
    o2 <- match(f2, unique(f2))
    repeat {
      m2 <- tapply(lab, o2, min)
      l2 <- m2[o2]
      m1 <- tapply(l2, lab, min)
      new <- m1[as.character(lab)]
      if (all(new == lab)) break
      lab <- unname(new)
    }
    length(unique(lab))
  }
  redundant <- rep(0, length(fes))
  if (length(fes) >= 2) redundant[2] <- components(fes[[1]], fes[[2]])
  counts <- ifelse(nested, 0, levels_of - redundant)
  df_a <- if (length(fes)) sum(counts) else 1
  df_a_full <- if (length(fes)) sum(levels_of - redundant) else 1
  Kt <- K + df_a + if (any(nested)) 1 else 0
  rss <- sum(e^2)
  meat <- function(cl) crossprod(rowsum(Xa * e, cl))
  G <- NA
  if (length(cluster) == 2) {
    c1 <- as.character(d[[cluster[1]]])
    c2 <- as.character(d[[cluster[2]]])
    Gs <- c(length(unique(c1)), length(unique(c2)))
    G <- min(Gs)
    V <- B %*% (meat(c1) + meat(c2) - meat(paste(c1, c2, sep = "\r"))) %*% B * (N - 1) / (N - Kt) * G / (G - 1)
    ev <- eigen((V + t(V)) / 2, symmetric = TRUE)
    if (any(ev$values < 0)) {
      if (notes) cat("Warning: VCV matrix was non-positive semi-definite; adjustment from Cameron, Gelbach & Miller applied.\n")
      V <- ev$vectors %*% diag(pmax(ev$values, 0), length(ev$values)) %*% t(ev$vectors)
    }
    df_r <- G - 1
  } else if (length(cluster) == 1) {
    cl <- as.character(d[[cluster]])
    Gs <- G <- length(unique(cl))
    V <- B %*% meat(cl) %*% B * (N - 1) / (N - Kt) * G / (G - 1)
    df_r <- G - 1
  } else if (robust) {
    Gs <- NULL
    V <- B %*% meat(seq_len(N)) %*% B * N / (N - Kt)
    df_r <- N - Kt
  } else {
    Gs <- NULL
    V <- B * rss / (N - Kt)
    df_r <- N - Kt
  }
  nm <- c(colnames(X)[keep], "(Intercept)")
  names(b) <- nm
  dimnames(V) <- list(nm, nm)
  bf <- stats::setNames(rep(NA_real_, ncol(X) + 1), c(colnames(X), "(Intercept)"))
  bf[nm] <- b
  Vf <- matrix(0, length(bf), length(bf), dimnames = list(names(bf), names(bf)))
  Vf[nm, nm] <- V
  slopes <- seq_len(K)
  Fs <- if (K > 0) drop(t(b[slopes]) %*% solve(V[slopes, slopes, drop = FALSE], b[slopes])) / K else NA_real_
  tss <- sum((y - mean(y))^2)
  tss_w <- sum(yw^2)
  r2 <- 1 - rss / tss
  r2_w <- 1 - rss / tss_w
  ll <- function(ss) -N / 2 * (1 + log(2 * pi * ss / N))
  fit <- stats::lm(formula, data = d)
  fit$stata <- list(N = N, N_full = N_full, num_singletons = dropped, df_r = df_r, df_m = K, df_a = df_a,
                    df_a_initial = if (length(fes)) sum(levels_of) else 1, df_a_redundant = sum(redundant) + sum(ifelse(nested, levels_of - redundant, 0)),
                    df_a_nested = sum(ifelse(nested, levels_of, 0)), rss = rss, tss = tss, tss_within = tss_w,
                    mss = tss - rss, rmse = sqrt(rss / (N - Kt)), F = Fs, r2 = r2, r2_within = r2_w,
                    r2_a = 1 - (1 - r2) * (N - 1) / (N - K - df_a_full),
                    r2_a_within = 1 - (1 - r2_w) * (N - df_a_full) / (N - K - df_a_full),
                    ll = ll(rss), ll_0 = ll(tss_w), G = Gs, absorb = absorb, cluster = cluster)
  list(fit = fit, b = bf, V = Vf, df = df_r, rows = rows)
}
stata_reghdfe_header <- function(fit) {
  e <- fit$stata
  pad <- function(s) formatC(s, width = -50, flag = "-")
  row <- function(left, right) cat(pad(left), right, "\n", sep = "")
  nh <- length(e$absorb)
  row("HDFE Linear regression", sprintf("Number of obs   = %10s", format(e$N, big.mark = ",")))
  row(if (nh) sprintf("Absorbing %d HDFE group%s", nh, if (nh == 1) "" else "s") else "",
      sprintf("F(%4d,%6d) = %10.2f", e$df_m, e$df_r, e$F))
  row(if (length(e$cluster)) "Statistics robust to heteroskedasticity" else "",
      sprintf("Prob > F        = %10.4f", stats::pf(e$F, e$df_m, e$df_r, lower.tail = FALSE)))
  row("", sprintf("R-squared       = %10.4f", e$r2))
  row("", sprintf("Adj R-squared   = %10.4f", e$r2_a))
  row("", sprintf("Within R-sq.    = %10.4f", e$r2_within))
  for (j in seq_along(e$cluster)) cat(formatC(sprintf("Number of clusters (%s)", e$cluster[j]), width = -29, flag = "-"), sprintf("= %10d\n", e$G[j]), sep = "")
  row("", sprintf("Root MSE        = %10.4f", e$rmse))
  if (length(e$cluster)) cat(formatC(sprintf("(Std. err. adjusted for %d clusters in %s)", min(e$G), paste(e$cluster, collapse = " ")), width = 78), "\n", sep = "")
  cat("\n")
  invisible(NULL)
}"#;

impl Session {
    /// `reghdfe depvar indepvars [if] [in], absorb(a [b]) | noabsorb
    /// [vce(robust | cluster c [c2]) cluster(c [c2]) keepsingletons]`.
    pub(super) fn reghdfe(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "reghdfe")?;
        if let Some(o) = opts.other(&[
            ("absorb", 1),
            ("noabsorb", 3),
            ("vce", 3),
            ("cluster", 2),
            ("keepsingletons", 7),
            ("level", 3),
        ]) {
            return Err(no(format!(
                "reghdfe with the option {o} is not translated yet"
            )));
        }
        if c.parts.weight.is_some() {
            return Err(no("reghdfe with weights is not translated yet"));
        }
        let absorb: Vec<String> = match opts.arg("absorb", 1) {
            Some(a) => {
                let mut v = Vec::new();
                for w in a.split_whitespace() {
                    let w = w.strip_prefix("i.").unwrap_or(w);
                    if !is_name(w) {
                        return Err(no(format!(
                            "absorb({a}): only variable names are translated (not interactions or savefe)"
                        )));
                    }
                    v.push(w.to_string());
                }
                v
            }
            None if opts.has("noabsorb", 3) => Vec::new(),
            None => return Err(no("reghdfe needs absorb() or noabsorb (Stata: r(198))")),
        };
        if absorb.len() > 2 {
            return Err(no(
                "reghdfe with more than two absorbed variables is not translated yet",
            ));
        }
        let mut robust = false;
        let mut cluster: Vec<String> = Vec::new();
        if let Some(v) = opts.arg("vce", 3) {
            let v = v.trim();
            let (kind, rest) = v.split_once(char::is_whitespace).unwrap_or((v, ""));
            if !kind.is_empty() && "robust".starts_with(kind) && rest.is_empty() {
                robust = true;
            } else if kind.len() >= 2 && "cluster".starts_with(kind) {
                cluster = rest.split_whitespace().map(str::to_string).collect();
            } else if !matches!(kind, "unadjusted" | "ols") {
                return Err(no(format!("reghdfe, vce({v}) is not translated yet")));
            }
        }
        if let Some(cl) = opts.arg("cluster", 2) {
            cluster = cl.split_whitespace().map(str::to_string).collect();
        }
        if cluster.len() > 2 || cluster.iter().any(|v| !is_name(v)) {
            return Err(no(
                "reghdfe: one or two cluster variables are translated (names only)",
            ));
        }
        let level = level(opts)?;
        let spec = Spec::read(c, "reghdfe")?;
        self.helpers_for(&spec);
        let right = spec.right(false)?;
        let src = self.rows_for(c, out)?;
        self.helpers.insert("stata_reghdfe");
        self.helpers.insert("stata_coefs");
        let list = |v: &[String]| {
            format!(
                "c({})",
                v.iter().map(|s| r_text(s)).collect::<Vec<_>>().join(", ")
            )
        };
        let mut args = format!(
            "{} ~ {right}, data = {src}, absorb = {}",
            r_name(&spec.dep),
            list(&absorb)
        );
        if !cluster.is_empty() {
            args.push_str(&format!(", cluster = {}", list(&cluster)));
        }
        if robust {
            args.push_str(", robust = TRUE");
        }
        if opts.has("keepsingletons", 7) {
            args.push_str(", keepsingletons = TRUE");
        }
        if self.quiet(c) {
            args.push_str(", notes = FALSE");
        }
        out.line(format!("stata_m <- stata_reghdfe({args})"));
        out.line("stata_estimates <- stata_m$fit");
        out.line("stata_b <- stata_m$b");
        out.line("stata_vcov <- stata_m$V");
        out.line("stata_df <- stata_m$df");
        let d = self.data()?;
        out.line(self.keep_rows(&src, "stata_m$rows"));
        out.package("dplyr");
        out.line(super::model::esample_rows_line(&d, &src, "stata_m$rows"));
        self.esample_new = true;
        self.fit_robust = robust || !cluster.is_empty();
        self.fit_cluster = !cluster.is_empty();
        self.estimates = Some(Estimates {
            fit: Fit::Reghdfe,
            dep: spec.dep.clone(),
            factors: spec.factors.clone(),
        });
        let vce = if !cluster.is_empty() {
            "cluster"
        } else if robust {
            "robust"
        } else {
            "unadjusted"
        };
        let joined = cluster.join(" ");
        let cl = (!cluster.is_empty()).then_some(joined.as_str());
        let args = super::returned::e_args(&spec.dep, vce, cl, &None);
        self.keep_e(c, out, &args, "");
        if !self.quiet(c) {
            out.line("stata_reghdfe_header(stata_estimates)");
            out.line(format!(
                "stata_coefs(stata_b, stata_vcov, stata_df, level = {}, label = \"Coefficient\")",
                r_number(level)
            ));
        }
        out.line("rm(stata_m)");
        out.caveat(
            "reghdfe as the SSC package fits it, matched to its results (not its code): \
             singletons dropped, the absorbed means taken out, its degrees of freedom and its \
             small-sample factors for robust, clustered and two-way clustered errors",
        );
        out.caveat("printed in R's layout, not Stata's table");
        Ok(())
    }
}
