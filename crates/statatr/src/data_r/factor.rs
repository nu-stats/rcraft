//! `factor varlist [if] [in] [, pf | pcf | ipf  factors(#) mineigen(#)]`:
//! factor analysis of the correlation matrix as Stata 19 prints it and the
//! e() it leaves (`plan/stata/factor.md`): principal factors (communalities
//! from the squared multiple correlations), principal-component factors and
//! iterated principal factors ([MV] factor), the eigenvalues of the reduced
//! correlation matrix, the likelihood-ratio test of independence, and the
//! loadings with each variable's uniqueness; each factor signed so that its
//! loadings sum to more than 0, as pca's.

use super::*;

pub(super) const FACTOR_HELPER: &str = r##"stata_factor <- function(d, vars, method = "pf", k = NULL, mineigen = NULL, citerate = NULL, show = TRUE, cmdline = "") {
  for (v in vars) {
    if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
    if (is.character(d[[v]])) stop(v, " is a string variable (Stata: type mismatch, r(109))", call. = FALSE)
  }
  X <- as.matrix(as.data.frame(lapply(d[vars], function(u) { u <- unclass(u); attributes(u) <- NULL; as.numeric(u) })))
  X <- X[stats::complete.cases(X), , drop = FALSE]
  n <- nrow(X)
  p <- length(vars)
  R <- stats::cor(X)
  dimnames(R) <- list(vars, vars)
  if (is.null(mineigen)) mineigen <- if (method == "pcf") 1 else 0
  sgn <- function(V) { for (j in seq_len(ncol(V))) if (sum(V[, j]) < 0) V[, j] <- -V[, j]; V }
  reduced <- function(h) { Rs <- R; diag(Rs) <- h; Rs }
  keep <- function(val) {
    f <- sum(val > mineigen)
    if (!is.null(k)) f <- min(f, k)
    max(f, 1)
  }
  if (method == "pcf") {
    ev <- eigen(R, symmetric = TRUE)
  } else {
    h <- 1 - 1 / diag(solve(R))
    ev <- eigen(reduced(h), symmetric = TRUE)
    if (method == "ipf") {
      # the communalities again from the loadings, until their change is small: Stata 19
      # stops when the sum of the squared changes is below 1e-8, keeping the eigenvalues it
      # had (citerate(#): that many rounds)
      f <- keep(ev$values)
      rounds <- if (is.null(citerate)) 1000 else citerate
      for (it in seq_len(rounds)) {
        V <- ev$vectors[, seq_len(f), drop = FALSE]
        L <- sweep(V, 2, sqrt(pmax(ev$values[seq_len(f)], 0)), `*`)
        h1 <- rowSums(L^2)
        if (is.null(citerate) && sum((h1 - h)^2) < 1e-8) break
        h <- h1
        ev <- eigen(reduced(h), symmetric = TRUE)
      }
    }
  }
  val <- ev$values
  f <- if (method == "ipf") keep(val) else keep(val)
  V <- sgn(ev$vectors[, seq_len(f), drop = FALSE])
  L <- sweep(V, 2, sqrt(pmax(val[seq_len(f)], 0)), `*`)
  fac <- paste0("Factor", seq_len(f))
  dimnames(L) <- list(vars, fac)
  psi <- 1 - rowSums(L^2)
  evsum <- sum(val)
  # Bartlett's test of independence: -(N - (2p + 5)/6) ln|R| (Stata 19)
  chi2 <- -(n - (2 * p + 5) / 6) * log(det(R))
  dfi <- p * (p - 1) / 2
  mtitle <- c(pf = "principal factors", pcf = "principal-component factors", ipf = "iterated principal factors")[[method]]
  E <- list(f = f, N = n, df_m = p * f - f * (f - 1) / 2, df_r = ((p - f)^2 - (p + f)) / 2, chi2_i = chi2, df_i = dfi,
            p_i = stats::pchisq(chi2, dfi, lower.tail = FALSE), evsum = evsum,
            cmdline = cmdline, cmd = "factor", marginsnotok = "_ALL", properties = "nob noV eigen", title = "Factor analysis",
            predict = "factor_p", estat_cmd = "factor_estat", rotate_cmd = "factor_rotate", mtitle = mtitle, method = method,
            sds = matrix(apply(X, 2, stats::sd), 1, dimnames = list("sd", vars)),
            means = matrix(colMeans(X), 1, dimnames = list("mean", vars)), C = R,
            Phi = diag(f) |> (\(m) { dimnames(m) <- list(fac, fac); m })(),
            L = L, Psi = matrix(psi, 1, dimnames = list("psi", vars)),
            Ev = matrix(val, 1, dimnames = list("Eigenvalue", paste0("Factor", seq_len(p)))))
  if (!show) return(E)
  rhs <- function(l, v) sprintf("%-17s=%11s", l, v)
  cat("(obs=", n, ")\n\n", sep = "")
  cat(formatC("Factor analysis/correlation", width = -49), rhs("Number of obs", stata_fmt(n, "%20.0fc", trim = TRUE)), "\n", sep = "")
  cat(formatC(paste0("    Method: ", mtitle), width = -49), rhs("Retained factors", f), "\n", sep = "")
  cat(formatC("    Rotation: (unrotated)", width = -49), rhs("Number of params", E$df_m), "\n\n", sep = "")
  cat("    ", strrep("-", 74), "\n", sep = "")
  cat("         Factor  |   Eigenvalue   Difference        Proportion   Cumulative\n")
  cat("    -------------+", strrep("-", 60), "\n", sep = "")
  cum <- cumsum(val) / evsum
  for (j in seq_len(p)) {
    cat(sprintf("    %11s  |%13.5f%13s%18.4f%13.4f", paste0("Factor", j), val[j], if (j < p) sprintf("%.5f", val[j] - val[j + 1]) else ".",
                val[j] / evsum, cum[j]), "\n", sep = "")
  }
  cat("    ", strrep("-", 74), "\n", sep = "")
  cat(sprintf("    LR test: independent vs. saturated:  chi2(%d) = %7.2f Prob>chi2 = %6.4f", as.integer(dfi), chi2, E$p_i), "\n\n", sep = "")
  cat("Factor loadings (pattern matrix) and unique variances\n\n")
  w <- 9 + 10 * (f - 1) + 1
  rule <- paste0("    ", strrep("-", 13 + 1 + w + 1 + 14), "\n")
  head <- c(sprintf("%9s", fac[1]), sprintf("%10s", fac[-1]))
  cat(rule, sprintf("    %12s |", "Variable"), paste0(head, collapse = ""), " |   Uniqueness \n", sep = "")
  cat("    -------------+", strrep("-", w), "+", strrep("-", 14), "\n", sep = "")
  for (i in seq_len(p)) {
    cells <- c(sprintf("%9.4f", L[i, 1]), sprintf("%10.4f", L[i, -1]))
    cat(sprintf("    %12s |", stata_tab_abbrev(vars[i], 12)), paste0(cells, collapse = ""), " |", sprintf("%12.4f", psi[i]), "  \n", sep = "")
  }
  cat(rule)
  E
}"##;

impl Session {
    /// `factor varlist [, pf pcf ipf factors(#) mineigen(#)]`.
    pub(super) fn factor(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "factor")?;
        if let Some(o) = opts.other(&[
            ("pf", 2),
            ("pcf", 3),
            ("ipf", 3),
            ("factors", 3),
            ("mineigen", 4),
            ("citerate", 2),
        ]) {
            return Err(no(format!(
                "factor with the option {o} is not translated yet"
            )));
        }
        if c.parts.weight.is_some() {
            return Err(no("factor with weights is not translated yet"));
        }
        // ranges and patterns (a1-a5) come here as the names (dofile.rs VARLIST)
        let vars: Vec<String> = c
            .parts
            .main
            .split_whitespace()
            .map(str::to_string)
            .collect();
        if let Some(v) = vars.iter().find(|v| !is_name(v)) {
            return Err(no(format!(
                "factor {v}: variable names written out are translated"
            )));
        }
        if vars.len() < 2 {
            return Err(no("factor needs two or more variables"));
        }
        let methods: Vec<&str> = [("pf", 2), ("pcf", 3), ("ipf", 3)]
            .iter()
            .filter(|(m, l)| opts.has(m, *l))
            .map(|(m, _)| *m)
            .collect();
        if methods.len() > 1 {
            return Err(no("factor: only one of pf, pcf and ipf (Stata: r(198))"));
        }
        let method = methods.first().copied().unwrap_or("pf");
        let k = match opts.arg("factors", 3) {
            None => None,
            Some(k) => Some(
                k.trim()
                    .parse::<u32>()
                    .ok()
                    .filter(|v| *v > 0)
                    .ok_or_else(|| no(format!("factors({k}) needs a whole number above 0")))?,
            ),
        };
        let mineigen = match opts.arg("mineigen", 4) {
            None => None,
            Some(m) => Some(
                m.trim()
                    .parse::<f64>()
                    .map_err(|_| no(format!("mineigen({m}) needs a number")))?,
            ),
        };
        let citerate = match opts.arg("citerate", 2) {
            None => None,
            Some(n) => Some(
                n.trim()
                    .parse::<u32>()
                    .map_err(|_| no(format!("citerate({n}) needs a whole number")))?,
            ),
        };
        if citerate.is_some() && method != "ipf" {
            return Err(no("factor, citerate() is for ipf (Stata: r(198))"));
        }
        let rows = self.rows_for(c, out)?;
        for h in ["stata_factor", "stata_fmt", "stata_tables"] {
            self.helpers.insert(h);
        }
        let q: Vec<String> = vars.iter().map(|v| r_text(v)).collect();
        out.line(format!(
            "stata_e <- stata_factor({rows}, c({}), method = {}{}{}{}{}, cmdline = {})",
            q.join(", "),
            r_text(method),
            k.map(|k| format!(", k = {k}")).unwrap_or_default(),
            mineigen
                .map(|m| format!(", mineigen = {}", r_number(m)))
                .unwrap_or_default(),
            citerate
                .map(|n| format!(", citerate = {n}"))
                .unwrap_or_default(),
            if self.quiet(c) { ", show = FALSE" } else { "" },
            r_text(format!("{} {}", c.written, super::returned::after_name(c).trim()).trim())
        ));
        // e() is factor's: no e(b) or e(V)
        out.line("if (exists(\"stata_b\")) rm(stata_b)");
        out.line("stata_r <- list()");
        self.estimates = None;
        self.estimates_lost =
            Some("factor's results are not a model RCraft's predict, test or margins read".into());
        self.e_new = true;
        for t in [
            "cmdline",
            "cmd",
            "marginsnotok",
            "properties",
            "title",
            "predict",
            "estat_cmd",
            "rotate_cmd",
            "mtitle",
            "method",
        ] {
            self.e_text.insert(t.to_string());
        }
        self.e_ret = super::returned::Returned::Kept {
            cmd: "factor".into(),
            model: String::new(),
            names: &[],
        };
        self.r_new = Some((&[], "factor".into()));
        Ok(())
    }
}
