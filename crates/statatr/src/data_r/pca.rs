//! `pca varlist [if] [in] [, components(#) covariance]`: principal components
//! of the correlation (or covariance) matrix as Stata 19 prints them and the
//! e() they leave (`plan/stata/pca.md`): the eigenvalues with their
//! differences and proportions, then the eigenvectors of the components kept
//! and each variable's unexplained variance. As Stata's manual ([MV] pca)
//! gives it, each eigenvector is signed so that its elements sum to more
//! than 0.

use super::*;

pub(super) const PCA_HELPER: &str = r##"stata_pca <- function(d, vars, k = NULL, covariance = FALSE, show = TRUE, cmdline = "") {
  for (v in vars) {
    if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
    if (is.character(d[[v]])) stop(v, " is a string variable (Stata: type mismatch, r(109))", call. = FALSE)
  }
  X <- as.matrix(as.data.frame(lapply(d[vars], function(u) { u <- unclass(u); attributes(u) <- NULL; as.numeric(u) })))
  X <- X[stats::complete.cases(X), , drop = FALSE]
  n <- nrow(X)
  if (n < 2) stop("insufficient observations (Stata: r(2001))", call. = FALSE)
  C <- if (covariance) stats::cov(X) else stats::cor(X)
  dimnames(C) <- list(vars, vars)
  ev <- eigen(C, symmetric = TRUE)
  val <- ev$values
  vec <- ev$vectors
  # each component signed so that its elements sum to more than 0 (Stata's manual)
  for (j in seq_len(ncol(vec))) if (sum(vec[, j]) < 0) vec[, j] <- -vec[, j]
  p <- length(vars)
  if (is.null(k)) k <- p
  k <- min(k, p)
  trace <- sum(diag(C))
  rho <- sum(val[seq_len(k)]) / trace
  L <- vec[, seq_len(k), drop = FALSE]
  comp <- paste0("Comp", seq_len(k))
  dimnames(L) <- list(vars, comp)
  unexpl <- if (k < p) rowSums(sweep(vec[, -seq_len(k), drop = FALSE]^2, 2, val[-seq_len(k)], `*`)) else rep(0, p)
  E <- list(N = n, f = k, rho = rho, trace = trace, lndet = sum(log(val)), cond = sqrt(max(val) / min(val)),
            cmdline = cmdline, cmd = "pca", title = "Principal components", marginsnotok = "_ALL",
            estat_cmd = "pca_estat", rotate_cmd = "pca_rotate", predict = "pca_p",
            Ctype = if (covariance) "covariance" else "correlation", properties = "nob noV eigen",
            sds = matrix(apply(X, 2, stats::sd), 1, dimnames = list("sd", vars)),
            means = matrix(colMeans(X), 1, dimnames = list("mean", vars)),
            components = local({
              cmat <- cbind(val, c(-diff(val), NA), val / trace, cumsum(val) / trace)
              dimnames(cmat) <- list(paste0("Comp", seq_len(p)), c("Eigenvalue", "Difference", "Proportion", "Cumulative"))
              cmat
            }),
            C = C, Psi = matrix(unexpl, 1, dimnames = list("Unexplained", vars)),
            Ev = matrix(val, 1, dimnames = list("r1", paste0("Eigenvalues:Comp", seq_len(p)))), L = L)
  if (!show) return(E)
  g6 <- function(v) sub("^(-?)0\\.", "\\1.", sprintf("%.6g", v))
  rhs <- function(l, v) sprintf("%-17s=%11s", l, v)
  cat("\n", formatC(paste0("Principal components/", E$Ctype), width = -49), rhs("Number of obs", stata_fmt(n, "%20.0fc", trim = TRUE)), "\n", sep = "")
  cat(strrep(" ", 49), rhs("Number of comp.", k), "\n", sep = "")
  cat(strrep(" ", 49), rhs("Trace", g6(trace)), "\n", sep = "")
  cat(formatC("    Rotation: (unrotated = principal)", width = -49), rhs("Rho", sprintf("%.4f", rho)), "\n\n", sep = "")
  cat("    ", strrep("-", 74), "\n", sep = "")
  cat("       Component |   Eigenvalue   Difference         Proportion   Cumulative\n")
  cat("    -------------+", strrep("-", 60), "\n", sep = "")
  cum <- cumsum(val) / trace
  for (j in seq_len(p)) {
    cat(sprintf("    %12s |%13s%13s%19s%13s", paste0("Comp", j), g6(val[j]), if (j < p) g6(val[j] - val[j + 1]) else ".",
                sprintf("%.4f", val[j] / trace), sprintf("%.4f", cum[j])), "\n", sep = "")
  }
  cat("    ", strrep("-", 74), "\n", sep = "")
  w <- 9 + 10 * (k - 1) + 1
  cat("\nPrincipal components (eigenvectors) \n\n")
  cat("    ", strrep("-", 13 + w + 1 + 14), "\n", sep = "")
  cat(sprintf("    %12s |", "Variable"), paste0(formatC(comp, width = 10)[1] |> sub(pattern = "^ ", replacement = ""),
      paste0(formatC(comp[-1], width = 10), collapse = "")), " | Unexplained \n", sep = "")
  cat("    -------------+", strrep("-", w), "+", strrep("-", 13), "\n", sep = "")
  for (i in seq_len(p)) {
    cells <- sprintf("%10.4f", L[i, ])
    cells[1] <- sub("^ ", "", cells[1])
    u <- if (abs(unexpl[i]) < 1e-12) "0" else stata_fmt(unexpl[i], "%9.4g", trim = TRUE)
    cat(sprintf("    %12s |", stata_tab_abbrev(vars[i], 12)), paste0(cells, collapse = ""), " |", sprintf("%12s", u), " \n", sep = "")
  }
  cat("    ", strrep("-", 13 + w + 1 + 14), "\n", sep = "")
  E
}"##;

impl Session {
    /// `pca varlist [, components(#) covariance]`.
    pub(super) fn pca(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "pca")?;
        if let Some(o) = opts.other(&[("components", 3), ("covariance", 3)]) {
            return Err(no(format!("pca with the option {o} is not translated yet")));
        }
        if c.parts.weight.is_some() {
            return Err(no("pca with weights is not translated yet"));
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
                "pca {v}: variable names written out are translated"
            )));
        }
        if vars.len() < 2 {
            return Err(no("pca needs two or more variables"));
        }
        let k = match opts.arg("components", 3) {
            None => None,
            Some(k) => Some(
                k.trim()
                    .parse::<u32>()
                    .ok()
                    .filter(|v| *v > 0)
                    .ok_or_else(|| no(format!("components({k}) needs a whole number above 0")))?,
            ),
        };
        let rows = self.rows_for(c, out)?;
        for h in ["stata_pca", "stata_fmt", "stata_tables"] {
            self.helpers.insert(h);
        }
        let q: Vec<String> = vars.iter().map(|v| r_text(v)).collect();
        out.line(format!(
            "stata_e <- stata_pca({rows}, c({}){}{}{}, cmdline = {})",
            q.join(", "),
            k.map(|k| format!(", k = {k}")).unwrap_or_default(),
            if opts.has("covariance", 3) {
                ", covariance = TRUE"
            } else {
                ""
            },
            if self.quiet(c) { ", show = FALSE" } else { "" },
            r_text(format!("{} {}", c.written, super::returned::after_name(c).trim()).trim())
        ));
        // e() is pca's: no e(b) or e(V), so test, lincom and predict have nothing to read
        out.line("if (exists(\"stata_b\")) rm(stata_b)");
        self.estimates = None;
        self.estimates_lost =
            Some("pca's results are not a model RCraft's predict, test or margins read".into());
        self.e_new = true;
        for t in [
            "cmdline",
            "cmd",
            "title",
            "marginsnotok",
            "estat_cmd",
            "rotate_cmd",
            "predict",
            "Ctype",
            "properties",
        ] {
            self.e_text.insert(t.to_string());
        }
        self.e_ret = super::returned::Returned::Kept {
            cmd: "pca".into(),
            model: String::new(),
            names: &[],
        };
        // Stata 19 leaves r() empty
        out.line("stata_r <- list()");
        self.r_new = Some((&[], "pca".into()));
        Ok(())
    }
}
