//! More of `estat` as Stata 19 prints it and the r() it leaves
//! (`plan/stata/estat-tests.md`): after regress `vif [, uncentered]`,
//! `imtest [, white]`, and on tsset data `bgodfrey`, `durbinalt` (`lags()`,
//! `nomiss0`) and `dwatson`; after logit and probit `classification
//! [, cutoff()]`; after logit `gof [, group() table]`. The formulas are
//! those of Stata's public manuals ([R] regress postestimation, [R] regress
//! postestimation time series, [R] estat gof), checked against Stata 19.

use super::model::Fit;
use super::*;

pub(super) const ESTAT_MORE_HELPER: &str = r##"stata_float <- function(v) readBin(writeBin(as.numeric(v), raw(), size = 4), "double", size = 4, n = length(v))
stata_uR2 <- function(y, M) {
  q <- qr(M)
  f <- qr.fitted(q, y)
  list(r2 = sum(f^2) / sum(y^2), rank = q$rank)
}
stata_vif <- function(fit, uncentered = FALSE) {
  X <- stats::model.matrix(fit)
  X <- X[, !is.na(stats::coef(fit)), drop = FALSE]
  cons <- colnames(X) == "(Intercept)"
  if (!any(cons) && !uncentered) stop("not appropriate after regress, nocons; use option uncentered to get uncentered VIFs (Stata: r(301))", call. = FALSE)
  st <- stata_bnames()
  st <- ifelse(colnames(X) %in% names(st), st[colnames(X)], colnames(X))
  st[colnames(X) == "(Intercept)"] <- "_cons"
  use <- if (uncentered) seq_len(ncol(X)) else which(!cons)
  vif <- vapply(use, function(j) {
    y <- X[, j]
    others <- X[, -j, drop = FALSE]
    r2 <- if (uncentered) stata_uR2(y, others)$r2 else {
      f <- stats::lm.fit(others, y)
      1 - sum(f$residuals^2) / sum((y - mean(y))^2)
    }
    stata_float(1 / (1 - r2))
  }, 0)
  nm <- st[use]
  factors <- any(grepl("[.#]", nm))
  # Stata 19 sorts by VIF, largest first, unless the model has factor variables
  if (!factors) {
    o <- order(-vif)
    vif <- vif[o]
    nm <- nm[o]
  }
  cat("\n    Variable |       VIF       1/VIF  \n")
  cat("-------------+----------------------\n")
  last <- ""
  for (i in seq_along(nm)) {
    n <- nm[i]
    m <- regmatches(n, regexec("^([0-9]+)[bno]*\\.(.+)$", n))[[1]]
    if (length(m) && !grepl("#", n)) {
      if (m[3] != last) cat(sprintf("%12s |\n", m[3]))
      last <- m[3]
      lab <- paste0(m[2], " ")
    } else {
      last <- ""
      lab <- if (n == "_cons") "_cons" else n
    }
    cat(sprintf("%12s |%10.2f%12.6f", lab, vif[i], 1 / vif[i]), "\n", sep = "")
  }
  cat("-------------+----------------------\n")
  cat(sprintf("    Mean VIF |%10.2f", mean(vif)), "\n", sep = "")
  r <- list()
  for (i in rev(seq_along(nm))) r[[paste0("vif_", i)]] <- vif[i]
  for (i in rev(seq_along(nm))) r[[paste0("name_", i)]] <- nm[i]
  invisible(r)
}
stata_imtest <- function(fit, white = FALSE) {
  e <- stats::residuals(fit)
  N <- length(e)
  X <- stats::model.matrix(fit)
  X <- X[, colnames(X) != "(Intercept)" & !is.na(stats::coef(fit)), drop = FALSE]
  k <- ncol(X)
  Z <- NULL
  for (i in seq_len(k)) for (j in i:k) Z <- cbind(Z, X[, i] * X[, j])
  s2 <- sum(e^2) / N
  one <- matrix(1, N, 1)
  h <- stata_uR2(e^2 - s2, cbind(one, X, Z))
  s <- stata_uR2(e^3 - 3 * s2 * e, cbind(one, X))
  kk <- stata_uR2(e^4 - 6 * s2 * e^2 + 3 * s2^2, one)
  ch <- N * h$r2; dh <- h$rank - 1
  cs <- N * s$r2; ds <- s$rank - 1
  ck <- N * kk$r2; dk <- 1
  ct <- ch + cs + ck; dt <- dh + ds + dk
  pv <- function(c, d) stats::pchisq(c, d, lower.tail = FALSE)
  r <- list()
  if (white) {
    a <- stats::lm.fit(cbind(one, X, Z), e^2)
    r2 <- 1 - sum(a$residuals^2) / sum((e^2 - mean(e^2))^2)
    w <- N * r2
    dw <- a$rank - 1
    cat("\nWhite's test\nH0: Homoskedasticity\nHa: Unrestricted heteroskedasticity\n\n")
    cat(sprintf("%11s = %6.2f\nProb > chi2 = %6.4f\n", paste0("chi2(", dw, ")"), w, pv(w, dw)))
    r <- list(p = pv(w, dw), df = dw, chi2 = w)
  }
  cat("\nCameron & Trivedi's decomposition of IM-test\n\n")
  rule <- paste0(strrep("-", 50), "\n")
  mid <- paste0(strrep("-", 21), "+", strrep("-", 28), "\n")
  cat(rule, sprintf("%20s |%11s%7s%10s\n", "Source", "chi2", "df", "p"), mid, sep = "")
  row <- function(l, c, d) cat(sprintf("%20s |%11.2f%7d%10.4f\n", l, c, as.integer(d), pv(c, d)))
  row("Heteroskedasticity", ch, dh)
  row("Skewness", cs, ds)
  row("Kurtosis", ck, dk)
  cat(mid)
  row("Total", ct, dt)
  cat(rule)
  invisible(c(list(df_t = dt, chi2_t = ct, df_k = dk, chi2_k = ck, df_s = ds, chi2_s = cs, df_h = dh, chi2_h = ch), r))
}
stata_tsresid <- function(fit, time, delta = 1) {
  e <- stats::residuals(fit)
  if (length(time) != length(e)) stop("the time variable does not match the estimation sample", call. = FALSE)
  if (anyNA(time)) stop("time variable has missing values (Stata: r(459))", call. = FALSE)
  o <- order(time)
  e <- unname(e[o])
  time <- time[o]
  gaps <- sum(diff(time) != delta)
  list(e = e, time = time, gaps = gaps, X = stats::model.matrix(fit)[o, !is.na(stats::coef(fit)), drop = FALSE])
}
stata_gaps_note <- function(g) if (g > 0) cat("\nNumber of gaps in sample = ", g, "\n", sep = "")
stata_autocorr <- function(fit, time, delta = 1, lags = 1, nomiss0 = FALSE, durbin = FALSE) {
  s <- stata_tsresid(fit, time, delta)
  e <- s$e
  N <- length(e)
  lagged <- function(j) e[match(s$time - j * delta, s$time)]
  chi2 <- df <- p <- numeric(0)
  for (q in lags) {
    L <- vapply(seq_len(q), lagged, numeric(N))
    L <- matrix(L, N)
    if (!nomiss0) L[is.na(L)] <- 0
    keep <- stats::complete.cases(L)
    M <- cbind(s$X, L)[keep, , drop = FALSE]
    y <- e[keep]
    if (durbin) {
      a <- stats::lm.fit(M, y)
      nl <- ncol(L)
      cols <- (ncol(M) - nl + 1):ncol(M)
      s2 <- sum(a$residuals^2) / (nrow(M) - a$rank)
      XtX <- solve(crossprod(M))
      b <- a$coefficients[cols]
      c2 <- drop(t(b) %*% solve(s2 * XtX[cols, cols, drop = FALSE], b))
    } else {
      c2 <- nrow(M) * (1 - sum(stats::lm.fit(M, y)$residuals^2) / sum((y - mean(y))^2))
    }
    chi2 <- c(chi2, c2)
    df <- c(df, q)
    p <- c(p, stats::pchisq(c2, q, lower.tail = FALSE))
  }
  stata_gaps_note(s$gaps)
  cat("\n", if (durbin) "Durbin's alternative test for autocorrelation" else "Breusch–Godfrey LM test for autocorrelation", "\n", sep = "")
  rule <- paste0(strrep("-", 75), "\n")
  cat(rule, "    lags(p)  |          chi2               df                 Prob > chi2\n",
      "-------------+", strrep("-", 61), "\n", sep = "")
  for (i in seq_along(lags)) cat(sprintf("%8d     |%15.3f%16d%25.4f\n", as.integer(lags[i]), chi2[i], as.integer(df[i]), p[i]))
  cat(rule, strrep(" ", 24), "H0: no serial correlation\n", sep = "")
  m <- function(v) matrix(v, 1, dimnames = list("r1", paste0("lags:", lags)))
  invisible(list(k = ncol(s$X), N = N, N_gaps = s$gaps, lags = paste(lags, collapse = " "), p = m(p), df = m(df), chi2 = m(chi2)))
}
stata_dwatson <- function(fit, time, delta = 1) {
  s <- stata_tsresid(fit, time, delta)
  e <- s$e
  nxt <- match(s$time + delta, s$time)
  d <- e[nxt] - e
  dw <- sum(d^2, na.rm = TRUE) / sum(e^2)
  stata_gaps_note(s$gaps)
  cat(sprintf("\nDurbin–Watson d-statistic(%3d, %5d) = %9s\n", ncol(s$X), length(e), stata_fmt(dw, "%9.0g", trim = TRUE)))
  invisible(list(dw = dw, k = ncol(s$X), N = length(e), N_gaps = s$gaps))
}
stata_lclass <- function(fit, dep, model, cutoff = 0.5) {
  y <- stats::model.response(stats::model.frame(fit))
  D <- as.numeric(y) != 0
  pr <- stats::fitted(fit) >= cutoff
  a <- sum(pr & D); b <- sum(pr & !D); c <- sum(!pr & D); d <- sum(!pr & !D)
  n <- a + b + c + d
  pc <- function(x, t) if (t > 0) 100 * x / t else NA_real_
  cat("\n", model, " model for ", dep, "\n\n", sep = "")
  cat("              -------- True --------\n")
  cat("Classified |", sprintf("%10s%14s  |%11s", "D", "~D", "Total"), "\n", sep = "")
  mid <- "-----------+--------------------------+-----------\n"
  cat(mid)
  cat("     +     |", sprintf("%10d%14d  |%11d", a, b, a + b), "\n", sep = "")
  cat("     -     |", sprintf("%10d%14d  |%11d", c, d, c + d), "\n", sep = "")
  cat(mid)
  cat("   Total   |", sprintf("%10d%14d  |%11d", a + c, b + d, n), "\n", sep = "")
  cat("\nClassified + if predicted Pr(D) >= ", stata_fmt(cutoff, "%9.0g", trim = TRUE), "\n", sep = "")
  cat("True D defined as ", dep, " != 0\n", sep = "")
  rule <- paste0(strrep("-", 50), "\n")
  line <- function(l, w, v) cat(sprintf("%-32s%-9s%8.2f%%\n", l, w, v))
  r <- list(P_corr = pc(a + d, n), P_p1 = pc(a, a + c), P_n0 = pc(d, b + d), P_p0 = pc(b, b + d), P_n1 = pc(c, a + c),
            P_1p = pc(a, a + b), P_0n = pc(d, c + d), P_0p = pc(b, a + b), P_1n = pc(c, c + d))
  cat(rule)
  line("Sensitivity", "Pr( +| D)", r$P_p1)
  line("Specificity", "Pr( -|~D)", r$P_n0)
  line("Positive predictive value", "Pr( D| +)", r$P_1p)
  line("Negative predictive value", "Pr(~D| -)", r$P_0n)
  cat(rule)
  line("False + rate for true ~D", "Pr( +|~D)", r$P_p0)
  line("False - rate for true D", "Pr( -| D)", r$P_n1)
  line("False + rate for classified +", "Pr(~D| +)", r$P_0p)
  line("False - rate for classified -", "Pr( D| -)", r$P_1n)
  cat(rule)
  line("Correctly classified", "", r$P_corr)
  cat(rule)
  r <- r[c("P_1n", "P_0p", "P_0n", "P_1p", "P_n1", "P_p0", "P_n0", "P_p1", "P_corr")]
  r$ctable <- matrix(c(a, c, a + c, b, d, b + d, a + b, c + d, n), 3, dimnames = list(c("+", "-", "Total"), c("D", "~D", "Total")))
  invisible(r)
}
stata_lgof <- function(fit, dep, group = NULL, table = FALSE) {
  y <- as.numeric(as.numeric(stats::model.response(stats::model.frame(fit))) != 0)
  p <- unname(stats::fitted(fit))
  N <- length(y)
  k <- sum(!is.na(stats::coef(fit)))
  if (is.null(group)) {
    X <- stats::model.matrix(fit)
    key <- do.call(paste, c(as.data.frame(X), sep = "\r"))
    pat <- match(key, unique(key))
    M <- tabulate(pat)
    Y <- vapply(seq_along(M), function(j) sum(y[pat == j]), 0)
    P <- p[match(seq_along(M), pat)]
    chi2 <- sum((Y - M * P)^2 / (M * P * (1 - P)))
    m <- length(M)
    df <- m - k
    cat("\nGoodness-of-fit test after logistic model\nVariable: ", dep, "\n\n", sep = "")
    cat(sprintf("      Number of observations = %6d\nNumber of covariate patterns = %6d\n%28s = %6.2f\n                 Prob > chi2 = %6.4f\n",
                N, m, paste0("Pearson chi2(", df, ")"), chi2, stats::pchisq(chi2, df, lower.tail = FALSE)))
    return(invisible(list(p = stats::pchisq(chi2, df, lower.tail = FALSE), df = df, chi2 = chi2, m = m, N = N)))
  }
  ps <- sort(p)
  # the upper boundary of quantile j: the smallest index whose share reaches j/g (Stata's manual),
  # the mean of it and the next where the share is reached exactly (as Stata's percentiles)
  cut <- unique(vapply(seq_len(group), function(j) {
    a <- N * j / group
    i <- ceiling(a - 1e-9)
    if (abs(a - round(a)) < 1e-9 && i < N) (ps[i] + ps[i + 1]) / 2 else ps[i]
  }, 0))
  g <- vapply(p, function(v) which(v <= cut)[1], 0)
  G <- length(cut)
  o1 <- vapply(seq_len(G), function(j) sum(y[g == j]), 0)
  e1 <- vapply(seq_len(G), function(j) sum(p[g == j]), 0)
  tot <- tabulate(g, G)
  chi2 <- sum((o1 - e1)^2 / e1 + ((tot - o1) - (tot - e1))^2 / (tot - e1))
  df <- G - 2
  cat("note: obs collapsed on ", group, " quantiles of estimated probabilities.\n", sep = "")
  cat("\nGoodness-of-fit test after logistic model\nVariable: ", dep, "\n\n", sep = "")
  if (table) {
    cat("  Table collapsed on quantiles of estimated probabilities\n")
    edge <- paste0("  +", strrep("-", 56), "+\n")
    sep <- "  |-------+--------+-------+-------+-------+-------+-------|\n"
    cat(edge, "  | Group |   Prob | Obs_1 | Exp_1 | Obs_0 | Exp_0 | Total |\n", sep, sep = "")
    for (j in seq_len(G)) {
      cat(sprintf("  | %5d | %6.4f | %5d | %5.1f | %5d | %5.1f | %5d |\n", j, cut[j], as.integer(o1[j]), e1[j],
                  as.integer(tot[j] - o1[j]), tot[j] - e1[j], as.integer(tot[j])))
      if (j %% 5 == 0 && j < G) cat(sep)
    }
    cat(edge, "\n", sep = "")
  }
  cat(sprintf(" Number of observations = %6d\n       Number of groups = %6d\n%23s = %6.2f\n            Prob > chi2 = %6.4f\n",
              N, G, paste0("Hosmer–Lemeshow chi2(", df, ")"), chi2, stats::pchisq(chi2, df, lower.tail = FALSE)))
  invisible(list(p = stats::pchisq(chi2, df, lower.tail = FALSE), df = df, chi2 = chi2, m = G, N = N))
}"##;

const R_VIF: &[&str] = &[
    "vif_1", "vif_2", "vif_3", "vif_4", "vif_5", "vif_6", "vif_7", "vif_8", "vif_9", "vif_10",
    "vif_11", "vif_12", "vif_13", "vif_14", "vif_15", "vif_16", "vif_17", "vif_18", "vif_19",
    "vif_20", "name_1", "name_2", "name_3", "name_4", "name_5", "name_6", "name_7", "name_8",
    "name_9", "name_10", "name_11", "name_12", "name_13", "name_14", "name_15", "name_16",
    "name_17", "name_18", "name_19", "name_20",
];
const R_IMTEST: &[&str] = &[
    "df_t", "chi2_t", "df_k", "chi2_k", "df_s", "chi2_s", "df_h", "chi2_h", "p", "df", "chi2",
];
const R_AUTOCORR: &[&str] = &["k", "N", "N_gaps", "lags"];
const R_DWATSON: &[&str] = &["dw", "k", "N", "N_gaps"];
const R_CLASS: &[&str] = &[
    "P_1n", "P_0p", "P_0n", "P_1p", "P_n1", "P_p0", "P_n0", "P_p1", "P_corr",
];
const R_GOF: &[&str] = &["p", "df", "chi2", "m", "N"];

impl Session {
    /// `estat vif`, `imtest`, `bgodfrey`, `durbinalt`, `dwatson`,
    /// `classification` and `gof`: Ok(false) when `sub` is none of them.
    pub(super) fn estat_more(
        &mut self,
        c: &Command,
        sub: &str,
        fit: Fit,
        dep: &str,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<bool, Untranslated> {
        let is = |full: &str, min: usize| sub.len() >= min && full.starts_with(sub);
        let only = |fits: &[Fit], what: &str| -> Result<(), Untranslated> {
            if fits.contains(&fit) {
                Ok(())
            } else {
                Err(no(format!(
                    "estat {what} after {} is not translated (Stata has it after {})",
                    fit.name(),
                    fits.iter()
                        .map(|f| f.name())
                        .collect::<Vec<_>>()
                        .join(" and ")
                )))
            }
        };
        let refuse_others = |known: &[(&str, usize)], what: &str| -> Result<(), Untranslated> {
            match opts.other(known) {
                Some(o) => Err(no(format!(
                    "estat {what} with the option {o} is not translated yet"
                ))),
                None => Ok(()),
            }
        };
        let (call, names): (String, &'static [&'static str]) = if is("vif", 3) {
            only(&[Fit::Regress], "vif")?;
            refuse_others(&[("uncentered", 3)], "vif")?;
            (
                format!(
                    "stata_vif(stata_estimates{})",
                    if opts.has("uncentered", 3) {
                        ", uncentered = TRUE"
                    } else {
                        ""
                    }
                ),
                R_VIF,
            )
        } else if is("imtest", 2) {
            only(&[Fit::Regress], "imtest")?;
            refuse_others(&[("white", 5)], "imtest")?;
            (
                format!(
                    "stata_imtest(stata_estimates{})",
                    if opts.has("white", 5) {
                        ", white = TRUE"
                    } else {
                        ""
                    }
                ),
                R_IMTEST,
            )
        } else if is("bgodfrey", 3) || is("durbinalt", 3) || is("dwatson", 3) {
            let what = if is("bgodfrey", 3) {
                "bgodfrey"
            } else if is("durbinalt", 3) {
                "durbinalt"
            } else {
                "dwatson"
            };
            only(&[Fit::Regress], what)?;
            if what == "dwatson" {
                refuse_others(&[], what)?;
            } else {
                refuse_others(&[("lags", 1), ("nomiss0", 7)], what)?;
            }
            if self.panel.is_some() {
                return Err(no(format!(
                    "estat {what} on panel data is not translated yet"
                )));
            }
            let Some(t) = self.panel_time.clone() else {
                if self.ts_unknown {
                    return Err(no(format!(
                        "estat {what}: whether the data are tsset is not known here"
                    )));
                }
                out.line("stop(\"time variable not set, use tsset varname ... (Stata: r(111))\", call. = FALSE)");
                return Ok(true);
            };
            let delta = self.ts_delta.clone().unwrap_or_else(|| "1".into());
            let time = format!("stata_data[[{}]]", r_text(&t));
            if what == "dwatson" {
                (
                    format!("stata_dwatson(stata_estimates, {time}, delta = {delta})"),
                    R_DWATSON,
                )
            } else {
                let lags: Vec<u32> = match opts.arg("lags", 1) {
                    None => vec![1],
                    Some(l) => l
                        .split_whitespace()
                        .map(|w| w.parse::<u32>().ok().filter(|v| *v > 0))
                        .collect::<Option<Vec<_>>>()
                        .ok_or_else(|| {
                            no(format!(
                                "lags({l}): whole numbers written out are translated"
                            ))
                        })?,
                };
                if lags.is_empty() {
                    return Err(no("lags() needs a number"));
                }
                (
                    format!(
                        "stata_autocorr(stata_estimates, {time}, delta = {delta}, lags = c({}){}{})",
                        lags.iter().map(|l| l.to_string()).collect::<Vec<_>>().join(", "),
                        if opts.has("nomiss0", 7) {
                            ", nomiss0 = TRUE"
                        } else {
                            ""
                        },
                        if what == "durbinalt" {
                            ", durbin = TRUE"
                        } else {
                            ""
                        }
                    ),
                    R_AUTOCORR,
                )
            }
        } else if is("classification", 4) {
            only(&[Fit::Logit, Fit::Probit], "classification")?;
            refuse_others(&[("cutoff", 3)], "classification")?;
            let cutoff = match opts.arg("cutoff", 3) {
                None => "0.5".to_string(),
                Some(v) => {
                    let x: f64 = v
                        .trim()
                        .parse()
                        .map_err(|_| no(format!("cutoff({v}) needs a number")))?;
                    r_number(x)
                }
            };
            (
                format!(
                    "stata_lclass(stata_estimates, {}, {}, cutoff = {cutoff})",
                    r_text(dep),
                    r_text(if fit == Fit::Logit {
                        "Logistic"
                    } else {
                        "Probit"
                    })
                ),
                R_CLASS,
            )
        } else if is("gof", 3) {
            only(&[Fit::Logit], "gof")?;
            refuse_others(&[("group", 1), ("table", 1)], "gof")?;
            let group = match opts.arg("group", 1) {
                None => None,
                Some(g) => Some(
                    g.trim()
                        .parse::<u32>()
                        .ok()
                        .filter(|v| *v > 1)
                        .ok_or_else(|| no(format!("group({g}) needs a whole number above 1")))?,
                ),
            };
            if opts.has("table", 1) && group.is_none() {
                return Err(no("estat gof, table without group() is not translated yet"));
            }
            (
                format!(
                    "stata_lgof(stata_estimates, {}{}{})",
                    r_text(dep),
                    group.map(|g| format!(", group = {g}")).unwrap_or_default(),
                    if opts.has("table", 1) {
                        ", table = TRUE"
                    } else {
                        ""
                    }
                ),
                R_GOF,
            )
        } else {
            return Ok(false);
        };
        for h in [
            "stata_estat_more",
            "stata_fmt",
            "stata_names",
            "stata_returned",
        ] {
            self.helpers.insert(h);
        }
        let call = format!("stata_r <- {call}");
        out.line(if self.quiet(c) {
            format!("invisible(utils::capture.output({call}))")
        } else {
            call
        });
        self.r_new = Some((names, "estat".into()));
        Ok(true)
    }
}
