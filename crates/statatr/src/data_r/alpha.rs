//! `alpha varlist [if] [in] [, generate(newvar) asis casewise std min(#)]`:
//! Cronbach's alpha and the scale it makes, as Stata 19 does them (observed
//! on made-up data, `plan/stata/alpha.md`).
//!
//! - An item whose loading on the first principal factor (correlations of
//!   the complete rows, squared multiple correlations on the diagonal,
//!   loadings summing above 0) is negative is reversed unless `asis`.
//! - The scale (`generate()`): in each row, the mean of the items present
//!   (at least `min()`, 1 by default; all with `casewise`), summed in the
//!   varlist's order with the running sum rounded to float at each step, then
//!   the mean rounded to float; with `std` each item is first standardized by
//!   its own mean and standard deviation. A float, labelled
//!   "mean(unstandardized items)" or "mean(standardized items)".
//! - r(k); with `casewise`, r(alpha) and r(cov) (r(rho) with `std`) from the
//!   complete rows, bit for bit. Without it Stata's average covariance pairs
//!   the data in a way not pinned down yet, so they are not kept.

use super::*;

pub(super) const ALPHA_HELPER: &str = r##"stata_alpha <- function(d, vars, gen = NULL, rows = NULL, asis = FALSE, casewise = FALSE, std = FALSE,
                        min = 1, quiet = FALSE) {
  for (v in vars) {
    if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
    if (is.character(d[[v]])) stop("string variables not allowed in varlist;\n", v, " is a string variable (Stata: r(109))", call. = FALSE)
  }
  if (!is.null(gen) && gen %in% names(d)) stop("variable ", gen, " already defined (Stata: r(110))", call. = FALSE)
  k <- length(vars)
  X <- sapply(vars, function(v) as.numeric(unclass(d[[v]])))
  if (!is.matrix(X)) X <- matrix(X, nrow = nrow(d), dimnames = list(NULL, vars))
  use <- if (is.null(rows)) rep(TRUE, nrow(d)) else (!is.na(rows) & rows)
  Xu <- X[use, , drop = FALSE]
  cc <- stats::complete.cases(Xu)
  sg <- rep(1, k)
  if (!asis) {
    # the first principal factor of the complete rows' correlations (squared
    # multiple correlations on the diagonal), its loadings summing to more
    # than 0: an item loading below 0 is reversed (200 of 200 data sets)
    R <- stats::cor(Xu[cc, , drop = FALSE])
    Ri <- tryCatch(solve(R), error = function(e) NULL)
    if (is.null(Ri) || anyNA(R)) stop("alpha: the items' correlations cannot be inverted, so which items Stata reverses is not worked out; not translated", call. = FALSE)
    R2 <- R
    diag(R2) <- 1 - 1 / diag(Ri)
    e <- eigen(R2, symmetric = TRUE)$vectors[, 1]
    if (sum(e) < 0) e <- -e
    if (any(abs(e) < 1e-12) || abs(sum(e)) < 1e-12) stop("alpha: an item's loading is 0, so whether Stata reverses it is not worked out; not translated", call. = FALSE)
    sg <- sign(e)
  }
  rev <- vars[sg < 0]
  Z <- sweep(X, 2, sg, "*")
  if (std) {
    mu <- colMeans(X[use, , drop = FALSE], na.rm = TRUE)
    sdv <- apply(X[use, , drop = FALSE], 2, stats::sd, na.rm = TRUE)
    Z <- sweep(sweep(sweep(X, 2, mu), 2, sdv, "/"), 2, sg, "*")
  }
  ok <- function(z) {
    s <- 0
    n <- 0
    for (v in z) if (!is.na(v)) {
      s <- stata_float(s + v)
      n <- n + 1
    }
    if (n < min || (casewise && n < length(z)) || n == 0) NA_real_ else stata_float(s / n)
  }
  out <- NULL
  if (!is.null(gen)) {
    out <- apply(Z, 1, ok)
    out[!use] <- NA
    d[[gen]] <- structure(out, label = if (std) "mean(standardized items)" else "mean(unstandardized items)", format.stata = "%9.0g")
  }
  r <- list(k = k)
  if (casewise) {
    C <- if (std) stats::cor(Z[use, , drop = FALSE][cc, , drop = FALSE]) else stats::cov(Z[use, , drop = FALSE][cc, , drop = FALSE])
    cb <- mean(C[upper.tri(C)])
    vb <- mean(diag(C))
    r$alpha <- k * cb / (vb + (k - 1) * cb)
    if (std) r$rho <- cb else r$cov <- cb
  }
  if (!quiet) {
    cat("\nTest scale = ", if (std) "mean(standardized items)" else "mean(unstandardized items)", "\n\n", sep = "")
    if (length(rev)) cat(if (length(rev) == 1) "Reversed item: " else "Reversed items: ", paste0(" ", rev, collapse = ""), "\n", sep = "")
    if (casewise) {
      cat(formatC(if (std) "Average interitem correlation:" else "Average interitem covariance:", width = -32), formatC(trimws(stata_fmt(cb, "%9.0g")), width = 9), "\n", sep = "")
    }
    cat(formatC("Number of items in the scale:", width = -32), formatC(k, width = 9), "\n", sep = "")
    if (casewise) cat(formatC("Scale reliability coefficient:", width = -32), formatC(sprintf("%.4f", r$alpha), width = 9), "\n", sep = "")
    else cat("(the average covariance and alpha without casewise are not worked out yet)\n")
  }
  list(d = d, r = r)
}"##;

impl Session {
    pub(super) fn alpha(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[
            ("generate", 3),
            ("asis", 4),
            ("casewise", 1),
            ("std", 3),
            ("min", 3),
        ]) {
            return Err(no(format!(
                "alpha with the option {o} is not translated yet"
            )));
        }
        if c.parts.weight.is_some() {
            return Err(no("alpha with weights is not translated yet"));
        }
        let vars: Vec<&str> = c.parts.main.split_whitespace().collect();
        if vars.len() < 2 {
            return Err(no("alpha needs two or more items"));
        }
        if let Some(v) = vars.iter().find(|v| !is_name(v)) {
            return Err(no(format!(
                "alpha {v}: names are translated (patterns are when the do-file runs)"
            )));
        }
        let gen = match opts.arg("generate", 3) {
            Some(g) => {
                let g = g.trim().to_string();
                if !is_name(&g) {
                    return Err(no(format!("alpha, generate({g}): one new name is needed")));
                }
                Some(g)
            }
            None => None,
        };
        let min = match opts.arg("min", 3) {
            Some(m) => m
                .trim()
                .parse::<u32>()
                .map_err(|_| no(format!("alpha, min({m}): a whole number is needed")))?,
            None => 1,
        };
        let casewise = opts.has("casewise", 1);
        let std = opts.has("std", 3);
        if std && casewise && gen.is_some() {
            return Err(no("alpha, std casewise generate() is not translated yet"));
        }
        let d = self.data()?;
        let rows = match self.condition(c, out)? {
            Some(cond) => {
                format!(", rows = dplyr::pull(dplyr::mutate({d}, stata_rows = {cond}), stata_rows)")
            }
            None => String::new(),
        };
        self.helpers.insert("stata_alpha");
        self.helpers.insert("stata_float");
        self.helpers.insert("stata_fmt");
        self.helpers.insert("stata_returned");
        out.package("dplyr");
        out.line(format!(
            "stata_v <- stata_alpha({d}, c({}), gen = {}{rows}, asis = {}, casewise = {}, std = {}, min = {min}, quiet = {})",
            vars.iter().map(|v| r_text(v)).collect::<Vec<_>>().join(", "),
            gen.as_deref().map(r_text).unwrap_or_else(|| "NULL".into()),
            super::stats::r_logical(opts.has("asis", 4)),
            super::stats::r_logical(casewise),
            super::stats::r_logical(std),
            super::stats::r_logical(self.quiet(c)),
        ));
        out.line(format!("{d} <- stata_v$d"));
        out.line("stata_r <- stata_v$r");
        out.line("rm(stata_v)");
        if let Some(g) = gen {
            self.kinds.insert(g.clone(), Kind::Number);
            self.storage.insert(g, Storage::Float);
        }
        self.r_new = Some((
            if casewise {
                super::returned::R_ALPHA_CASEWISE
            } else {
                super::returned::R_ALPHA
            },
            "alpha".into(),
        ));
        out.caveat("printed in R's layout");
        Ok(())
    }
}
