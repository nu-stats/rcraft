//! `rotate [, varimax | quartimax | equamax | promax[(#)] normalize blanks(#) clear]`
//! after `factor` or `pca`, as Stata 19 prints it and the e() and r() it leaves
//! (`plan/stata/rotate.md`). Found by observing Stata 19 on made-up data:
//!
//! - varimax, and the varimax that promax starts from, is Kaiser's iteration
//!   (T from the SVD of A'(Z^3 - Z diag(colSums(Z^2)/p)), from T = I), stopped
//!   when the sum of the singular values grows by less than 1e-8 of itself;
//! - quartimax and equamax are the orthogonal gradient projection algorithm
//!   (step 1, doubled each round and halved until the criterion falls by half
//!   the squared gradient times the step), stopped when T changes by less
//!   than 1e-6 (Stata's mreldif) and the projected gradient is below 1e-6;
//!   equamax's weight is p/2 (p the number of variables);
//! - promax(k) regresses the target B|B|^(k-1) of the varimax loadings
//!   (normalized ones under normalize) on the varimax loadings themselves;
//! - each factor is then signed so that its loadings sum to more than 0 and
//!   the factors are ordered by their variance, largest first.
//!
//! After factor the rotated loadings are L W and e(r_T) is W^-1' (W itself
//! when the rotation is orthogonal), the factors' correlations T'T and an
//! oblique factor's variance the column sum of its squared structure
//! loadings. After pca the eigenvectors are rotated, L W, e(r_T) is W and a
//! component's variance diag(W' Lambda W).

use super::*;

pub(super) const ROTATE_HELPER: &str = r##"stata_rotate <- function(E, crit = "varimax", power = 3, normalize = FALSE, blanks = NULL, clear = FALSE, show = TRUE) {
  rn <- c("r_f", "r_normalization", "r_class", "r_criterion", "r_ctitle", "r_Ev", "r_Phi", "r_T", "r_L")
  keepattr <- function(x) { for (a in setdiff(names(attributes(E)), "names")) attr(x, a) <- attr(E, a); x }
  if (clear) return(list(e = keepattr(E[setdiff(names(E), rn)]), r = list()))
  pca <- identical(E$cmd, "pca")
  A0 <- E$L
  p <- nrow(A0); k <- ncol(A0)
  if (pca && k == p) stop("rotate after pca with all components kept is not translated: Stata's answer there rests on its own rounding", call. = FALSE)
  w <- if (normalize) sqrt(rowSums(A0^2)) else rep(1, p)
  A <- A0 / w
  kaiser <- function(A) {
    # two factors: Kaiser's angle, the optimum itself (Stata 19's answer to the last digit)
    if (k == 2) {
      a <- A[, 1]; b <- A[, 2]; u <- a^2 - b^2; v <- 2 * a * b
      phi <- atan2(2 * sum(u * v) - 2 * sum(u) * sum(v) / p, sum(u^2 - v^2) - (sum(u)^2 - sum(v)^2) / p) / 4
      return(matrix(c(cos(phi), sin(phi), -sin(phi), cos(phi)), 2))
    }
    T <- diag(k); d0 <- 0
    for (it in seq_len(1000)) {
      Z <- A %*% T
      s <- svd(t(A) %*% (Z^3 - sweep(Z, 2, colSums(Z^2) / p, `*`)))
      T <- s$u %*% t(s$v)
      d1 <- sum(s$d)
      if ((d1 - d0) / d1 < 1e-8) return(T)
      d0 <- d1
    }
    stop("rotate: the varimax iteration did not settle in 1000 rounds, where Stata's message is not matched; not translated", call. = FALSE)
  }
  g <- switch(crit, quartimax = 0, equamax = p / 2, 1)
  cr <- function(Z) { Z2 <- Z^2; C <- sweep(Z2, 2, g * colMeans(Z2)); list(f = -sum(Z2 * C) / 4, G = -Z * C) }
  gpf <- function(A) {
    T <- diag(k); v <- cr(A %*% T); G <- t(A) %*% v$G; al <- 1
    for (it in seq_len(1000)) {
      M <- t(T) %*% G; Gp <- G - T %*% ((M + t(M)) / 2); s <- sqrt(sum(Gp^2))
      al <- 2 * al
      for (i in 0:10) {
        sv <- svd(T - al * Gp); Tt <- sv$u %*% t(sv$v); vt <- cr(A %*% Tt)
        if (vt$f < v$f - 0.5 * s^2 * al) break
        al <- al / 2
      }
      dT <- max(abs(Tt - T) / (abs(Tt) + 1))
      T <- Tt; v <- vt; G <- t(A) %*% v$G
      M <- t(T) %*% G; s <- sqrt(sum((G - T %*% ((M + t(M)) / 2))^2))
      if (dT < 1e-6 && s < 1e-6) return(list(T = T, f = v$f))
    }
    stop("rotate: the ", crit, " iteration did not settle in 1000 rounds, where Stata's message is not matched; not translated", call. = FALSE)
  }
  if (k == 1) {
    W <- diag(1)
  } else if (crit %in% c("varimax", "promax")) {
    W <- kaiser(A)
  } else {
    W <- gpf(A)$T
  }
  oblique <- crit == "promax"
  if (oblique && k > 1) {
    B <- A0 %*% W
    Bn <- A %*% W
    P <- Bn * abs(Bn)^(power - 1)
    U <- solve(crossprod(B), crossprod(B, P))
    U <- U %*% diag(sqrt(diag(solve(crossprod(U)))), k)
    W <- W %*% U
  }
  Lr <- A0 %*% W
  T <- if (oblique && !pca) t(solve(W)) else W
  # each factor signed so that its loadings sum to more than 0, then ordered by variance
  sg <- ifelse(colSums(Lr) < 0, -1, 1)
  Lr <- sweep(Lr, 2, sg, `*`); T <- sweep(T, 2, sg, `*`); W <- sweep(W, 2, sg, `*`)
  ev <- if (pca) E$Ev[1, seq_len(k)] else NULL
  Phi <- crossprod(T)
  var <- if (pca) diag(t(W) %*% diag(ev, k) %*% W) else colSums((Lr %*% Phi)^2)
  o <- order(-var)
  Lr <- Lr[, o, drop = FALSE]; T <- T[, o, drop = FALSE]; Phi <- Phi[o, o, drop = FALSE]; var <- var[o]
  fac <- if (pca) paste0("Comp", seq_len(k)) else paste0("Factor", seq_len(k))
  dimnames(Lr) <- list(rownames(E$L), fac)
  dimnames(T) <- list(fac, fac)
  dimnames(Phi) <- list(fac, fac)
  if (oblique) Phi <- (Phi + t(Phi)) / 2
  cls <- if (oblique) "oblique" else "orthogonal"
  ptxt <- sub("^\\.", "0.", stata_fmt(power, "%9.0g", trim = TRUE))
  ctitle <- if (oblique) paste0("promax(", ptxt, ")") else crit
  kai <- if (normalize) "Kaiser on" else "Kaiser off"
  rtext <- paste0(" ", cls, " ", crit, " (", kai, ")")
  old <- E[setdiff(names(E), rn)]
  txt <- if (pca) list(r_normalization = if (normalize) "kaiser" else "none", r_ctitle = ctitle, r_criterion = crit, r_class = cls)
         else list(r_normalization = if (normalize) "kaiser" else "none", r_class = cls, r_criterion = crit, r_ctitle = ctitle)
  mats <- list(r_Ev = matrix(var, 1, dimnames = list("r1", fac)))
  if (!pca) mats$r_Phi <- Phi
  mats$r_T <- T
  mats$r_L <- Lr
  E2 <- c(txt, mats, old)
  E2$r_f <- k
  for (a in setdiff(names(attributes(E)), "names")) attr(E2, a) <- attr(E, a)
  R <- list(rtext = rtext)
  if (!show) return(list(e = E2, r = R))
  # Kaiser's two-factor rotation, a factor's sign turned or the two swapped, is exactly
  # symmetric, and factor's table of it then shows the lower triangle (Stata 19)
  symm <- !pca && crit == "varimax" && k == 2 && det(T) < 0
  rhs <- function(l, v) sprintf("%-17s=%11s", l, v)
  rot <- formatC(paste0("    Rotation:", rtext), width = -49)
  g6 <- function(v) sub("^(-?)0\\.", "\\1.", sprintf("%.6g", v))
  bl <- if (is.null(blanks)) "" else stata_fmt(blanks, "%9.0g", trim = TRUE)
  cell <- function(v, wd) if (!is.null(blanks) && abs(v) < blanks) strrep(" ", wd) else sprintf(paste0("%", wd, ".4f"), v)
  vars <- rownames(E$L)
  if (pca) {
    cat("\n", formatC(paste0("Principal components/", E$Ctype), width = -49), rhs("Number of obs", stata_fmt(E$N, "%20.0fc", trim = TRUE)), "\n", sep = "")
    cat(strrep(" ", 49), rhs("Number of comp.", k), "\n", sep = "")
    cat(strrep(" ", 49), rhs("Trace", g6(E$trace)), "\n", sep = "")
    cat(rot, rhs("Rho", sprintf("%.4f", E$rho)), "\n\n", sep = "")
    cat("    ", strrep("-", 74), "\n", sep = "")
    if (oblique) cat("       Component |     Variance   Proportion     Rotated comp. are correlated\n")
    else cat("       Component |     Variance   Difference         Proportion   Cumulative\n")
    cat("    -------------+", strrep("-", 60), "\n", sep = "")
    cum <- cumsum(var) / E$trace
    for (j in seq_len(k)) {
      if (oblique) cat(sprintf("    %12s |%13s%13s", fac[j], g6(var[j]), sprintf("%.4f", var[j] / E$trace)), "\n", sep = "")
      else cat(sprintf("    %12s |%13s%13s%19s%13s", fac[j], g6(var[j]), if (j < k) g6(var[j] - var[j + 1]) else ".",
                       sprintf("%.4f", var[j] / E$trace), sprintf("%.4f", cum[j])), "\n", sep = "")
    }
    cat("    ", strrep("-", 74), "\n", sep = "")
    wd <- 9 + 10 * (k - 1) + 1
    cat("\nRotated components ", if (!is.null(blanks)) paste0(" (blanks are abs(loading)<", bl, ")"), "\n\n", sep = "")
    cat("    ", strrep("-", 13 + wd + 1 + 14), "\n", sep = "")
    cat(sprintf("    %12s |", "Variable"), paste0(formatC(fac, width = 10)[1] |> sub(pattern = "^ ", replacement = ""),
        paste0(formatC(fac[-1], width = 10), collapse = "")), " | Unexplained \n", sep = "")
    cat("    -------------+", strrep("-", wd), "+", strrep("-", 13), "\n", sep = "")
    un <- E$Psi[1, ]
    for (i in seq_len(p)) {
      cells <- c(cell(Lr[i, 1], 9), vapply(Lr[i, -1], cell, "", wd = 10))
      u <- if (abs(un[i]) < 1e-12) "0" else stata_fmt(un[i], "%9.4g", trim = TRUE)
      cat(sprintf("    %12s |", stata_tab_abbrev(vars[i], 12)), paste0(cells, collapse = ""), " |", sprintf("%12s", u), " \n", sep = "")
    }
    cat("    ", strrep("-", 13 + wd + 1 + 14), "\n", sep = "")
    cat("\nComponent rotation matrix\n\n")
    cat("    ", strrep("-", 14 + 10 * k), "\n", sep = "")
    cat(sprintf("    %12s |", ""), paste0(sprintf("%9s ", fac), collapse = ""), "\n", sep = "")
    cat("    -------------+", strrep("-", 10 * k), "\n", sep = "")
    for (i in seq_len(k)) cat(sprintf("    %12s |", fac[i]), paste0(sprintf("%9.4f ", T[i, ]), collapse = ""), "\n", sep = "")
    cat("    ", strrep("-", 14 + 10 * k), "\n", sep = "")
  } else {
    cat("\n", formatC("Factor analysis/correlation", width = -49), rhs("Number of obs", stata_fmt(E$N, "%20.0fc", trim = TRUE)), "\n", sep = "")
    cat(formatC(paste0("    Method: ", E$mtitle), width = -49), rhs("Retained factors", k), "\n", sep = "")
    cat(rot, rhs("Number of params", E$df_m), "\n\n", sep = "")
    cat("    ", strrep("-", 74), "\n", sep = "")
    if (oblique) cat("         Factor  |     Variance   Proportion    Rotated factors are correlated\n")
    else cat("         Factor  |     Variance   Difference        Proportion   Cumulative\n")
    cat("    -------------+", strrep("-", 60), "\n", sep = "")
    cum <- cumsum(var) / E$evsum
    for (j in seq_len(k)) {
      if (oblique) cat(sprintf("    %11s  |%13.5f%13.4f", fac[j], var[j], var[j] / E$evsum), "\n", sep = "")
      else cat(sprintf("    %11s  |%13.5f%13s%18.4f%13.4f", fac[j], var[j], if (j < k) sprintf("%.5f", var[j] - var[j + 1]) else ".",
                       var[j] / E$evsum, cum[j]), "\n", sep = "")
    }
    cat("    ", strrep("-", 74), "\n", sep = "")
    cat(sprintf("    LR test: independent vs. saturated:  chi2(%d) = %7.2f Prob>chi2 = %6.4f", as.integer(E$df_i), E$chi2_i, E$p_i), "\n\n", sep = "")
    cat("Rotated factor loadings (pattern matrix) and unique variances\n\n")
    wd <- 9 + 10 * (k - 1) + 1
    rule <- paste0("    ", strrep("-", 13 + 1 + wd + 1 + 14), "\n")
    head <- c(sprintf("%9s", fac[1]), sprintf("%10s", fac[-1]))
    cat(rule, sprintf("    %12s |", "Variable"), paste0(head, collapse = ""), " |   Uniqueness \n", sep = "")
    cat("    -------------+", strrep("-", wd), "+", strrep("-", 14), "\n", sep = "")
    psi <- E$Psi[1, ]
    for (i in seq_len(p)) {
      cells <- c(cell(Lr[i, 1], 9), vapply(Lr[i, -1], cell, "", wd = 10))
      cat(sprintf("    %12s |", stata_tab_abbrev(vars[i], 12)), paste0(cells, collapse = ""), " |", sprintf("%12.4f", psi[i]), "  \n", sep = "")
    }
    cat(rule)
    if (!is.null(blanks)) cat("    (blanks represent abs(loading)<", bl, ")\n", sep = "")
    cat("\nFactor rotation matrix\n\n")
    cat("    ", strrep("-", 14 + 9 * k), "\n", sep = "")
    cat(sprintf("    %12s |", ""), paste0(sprintf(" %7s ", fac), collapse = ""), "\n", sep = "")
    cat("    -------------+", strrep("-", 9 * k), "\n", sep = "")
    for (i in seq_len(k)) {
      cells <- sprintf(" %7.4f ", T[i, ])
      if (symm) cells[seq_len(k) > i] <- strrep(" ", 9)
      cat(sprintf("    %12s |", fac[i]), paste0(cells, collapse = ""), "\n", sep = "")
    }
    cat("    ", strrep("-", 14 + 9 * k), "\n", sep = "")
  }
  list(e = E2, r = R)
}"##;

/// The r() names `rotate` leaves.
const R_ROTATE: &[&str] = &["rtext"];

/// The rotations translated: the option's name and its shortest form.
const CRITERIA: &[(&str, usize)] = &[
    ("varimax", 7),
    ("quartimax", 9),
    ("equamax", 7),
    ("promax", 6),
];

impl Session {
    /// `rotate [, varimax quartimax equamax promax[(#)] orthogonal oblique
    /// normalize horst blanks(#) clear]` after factor or pca.
    pub(super) fn rotate(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "rotate")?;
        let after = match &self.e_ret {
            super::returned::Returned::Kept { cmd, .. } => cmd.clone(),
            super::returned::Returned::Nothing => {
                return Err(no(
                    "rotate: no factor or pca before it in this do-file (R might hold an earlier run's)",
                ))
            }
            super::returned::Returned::Lost(why) => {
                return Err(no(format!("rotate: what e() holds is not known here: {why}")))
            }
            super::returned::Returned::Program { .. } => {
                return Err(no("rotate after results ereturn posted is not translated"))
            }
        };
        if let Some(o) = opts.other(&[
            ("varimax", 7),
            ("quartimax", 9),
            ("equamax", 7),
            ("promax", 6),
            ("orthogonal", 10),
            ("oblique", 7),
            ("normalize", 9),
            ("nonormalize", 11),
            ("horst", 5),
            ("blanks", 6),
            ("clear", 5),
        ]) {
            return Err(no(format!(
                "rotate with the option {o} is not translated yet"
            )));
        }
        let stop = |msg: &str, rc: u32| {
            format!(
                "stop({}, call. = FALSE)",
                r_text(&format!("{msg} (Stata: r({rc}))"))
            )
        };
        if c.parts.weight.is_some() {
            return Err(no("rotate with weights is not translated"));
        }
        if c.parts.in_range.is_some() {
            return Err(no("rotate with in is not translated (Stata: r(101))"));
        }
        let named: Vec<&str> = CRITERIA
            .iter()
            .filter(|(n, l)| opts.has(n, *l))
            .map(|(n, _)| *n)
            .collect();
        let clear = opts.has("clear", 5);
        let normalize = opts.has("normalize", 9) || opts.has("horst", 5);
        let nonormalize = opts.has("nonormalize", 11);
        let oblique = opts.has("oblique", 7);
        let orthogonal = opts.has("orthogonal", 10);
        let crit = named.first().copied().unwrap_or("varimax");
        // the combinations whose message was not observed in Stata 19
        if clear && opts.0.len() > 1 {
            return Err(no(
                "rotate, clear with other options is not translated (Stata: r(198))",
            ));
        }
        if normalize && nonormalize {
            return Err(no(
                "rotate, normalize nonormalize is not translated (Stata: r(198))",
            ));
        }
        if orthogonal && oblique {
            return Err(no("rotate, orthogonal oblique is not translated"));
        }
        if oblique && crit != "varimax" && crit != "promax" {
            return Err(no(format!(
                "rotate, oblique {crit} is not translated (Stata: r(496))"
            )));
        }
        for (n, l) in [
            ("varimax", 7),
            ("quartimax", 9),
            ("equamax", 7),
            ("clear", 5),
            ("normalize", 9),
            ("horst", 5),
        ] {
            if opts.raw(n, l).is_some() {
                return Err(no(format!(
                    "rotate, {n}() is not translated (Stata: r(198))"
                )));
            }
        }
        let mut power = 3.0;
        let mut power_low = false;
        if let Some(arg) = opts.raw("promax", 6) {
            match arg.trim().parse::<f64>() {
                Ok(v) if v >= 1.0 && v.is_finite() => power = v,
                Ok(v) if v.is_finite() => power_low = true,
                _ => {
                    return Err(no(format!(
                        "rotate, promax({arg}): only a number is translated"
                    )))
                }
            }
        }
        let mut blanks: Option<f64> = None;

        if let Some(arg) = opts.raw("blanks", 6) {
            match arg.trim().parse::<f64>() {
                Ok(v) if v >= 0.0 && v.is_finite() => blanks = Some(v),
                Ok(v) if v.is_finite() => {
                    // Stata 19 prints the rotation's header and variances, then stops
                    return Err(no(
                        "rotate, blanks() below 0 is not translated: Stata 19 prints part of the table before its error (r(125))",
                    ));
                }
                _ => {
                    return Err(no(format!(
                        "rotate, blanks({arg}): only a number is translated"
                    )))
                }
            }
        } else if opts.has("blanks", 6) {
            return Err(no("rotate, blanks without a number is not translated"));
        }
        // Stata 19's own errors, in the order it was seen to check them
        let error = if !c.parts.main.trim().is_empty() {
            Some(stop("varlist not allowed", 101))
        } else if c.parts.if_exp.is_some() {
            Some(stop("if not allowed", 101))
        } else if after != "factor" && after != "pca" {
            Some(stop("no rotation command found in e(rotate_cmd)", 198))
        } else if named.len() > 1 {
            Some(stop("multiple rotation criteria specified", 198))
        } else if crit == "promax" && orthogonal {
            Some(stop("promax is always oblique", 198))
        } else if crit == "varimax" && oblique {
            Some(stop(
                "the varimax criterion is not suitable for oblique rotations",
                496,
            ))
        } else if power_low {
            Some(stop("promax() should be at least 1", 198))
        } else {
            None
        };
        if let Some(e) = error {
            out.line(e);
            return Ok(());
        }
        for h in ["stata_rotate", "stata_fmt", "stata_tables"] {
            self.helpers.insert(h);
        }
        out.line(format!(
            "stata_rot <- stata_rotate(stata_e, {}{}{}{}{}{})",
            r_text(crit),
            if crit == "promax" && power != 3.0 {
                format!(", power = {}", r_number(power))
            } else {
                String::new()
            },
            if normalize { ", normalize = TRUE" } else { "" },
            blanks
                .map(|b| format!(", blanks = {}", r_number(b)))
                .unwrap_or_default(),
            if clear { ", clear = TRUE" } else { "" },
            if self.quiet(c) || clear {
                ", show = FALSE"
            } else {
                ""
            },
        ));
        out.line("stata_e <- stata_rot$e");
        out.line("stata_r <- stata_rot$r");
        out.line("rm(stata_rot)");
        for t in ["r_normalization", "r_class", "r_criterion", "r_ctitle"] {
            if clear {
                self.e_text.remove(t);
            } else {
                self.e_text.insert(t.to_string());
            }
        }
        // e() stays factor's or pca's; its detail says how it is rotated
        self.e_ret = super::returned::Returned::Kept {
            cmd: after,
            model: if clear {
                String::new()
            } else if crit == "promax" {
                "oblique".into()
            } else {
                "orthogonal".into()
            },
            names: &[],
        };
        self.r_new = Some((if clear { &[] } else { R_ROTATE }, "rotate".into()));
        Ok(())
    }
}
