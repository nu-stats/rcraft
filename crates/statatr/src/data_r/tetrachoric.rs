//! `tetrachoric varlist [if] [in] [fweight] [, stats(rho se p obs) edwards
//! print(#) star(#) bonferroni sidak pw matrix notable posdef]`: tetrachoric
//! correlations of 0/1 variables as Stata 19 prints them and the r() it leaves
//! (`plan/stata/tetrachoric.md`). Each pair's thresholds come from its margins
//! and rho maximizes the 2x2 table's likelihood (the full model's, so its
//! standard error is the inverse information's of the three parameters);
//! edwards is Edwards and Edwards's (a^(pi/4) - 1)/(a^(pi/4) + 1) of the odds
//! ratio a, its standard error by the delta method; the test is Fisher's exact
//! test. Checked against Stata 19 on made-up data.

use super::*;

pub(super) const TETRACHORIC_HELPER: &str = r##"stata_tetrachoric <- function(d, vars, stats = "rho", edwards = FALSE, pw = FALSE, adjust = "none",
                              print = NA, star = NA, matrix = FALSE, table = TRUE, posdef = FALSE, fw = NULL, show = TRUE) {
  for (v in c(vars, fw)) {
    if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
    if (is.character(d[[v]])) stop("tetrachoric with a string variable is not translated", call. = FALSE)
  }
  num <- function(v) { u <- unclass(d[[v]]); attributes(u) <- NULL; as.numeric(u) }
  M <- sapply(vars, num)
  if (!is.matrix(M)) M <- matrix(M, ncol = length(vars))
  f <- if (is.null(fw)) rep(1, nrow(M)) else num(fw)
  keep <- !is.na(f)
  if (!pw) keep <- keep & stats::complete.cases(M)
  M <- M[keep, , drop = FALSE]; f <- f[keep]
  if (any(f < 0 | f != round(f))) stop("may not use noninteger frequency weights (Stata: r(401))", call. = FALSE)
  for (j in seq_along(vars)) if (!all(M[, j] %in% c(0, 1, NA))) stop("variable ", vars[j], " is not 0/1 coded (Stata: r(198))", call. = FALSE)
  refuse <- function(why) stop("tetrachoric: ", why, ", where Stata 19's answer is not matched; not translated", call. = FALSE)
  # P(X < h, Y < k) at correlation r: Sheppard's integral over the angle, smooth and finite
  bvn <- function(h, k, r) {
    if (r == 0) return(stats::pnorm(h) * stats::pnorm(k))
    g <- function(t) exp(-(h^2 - 2 * h * k * sin(t) + k^2) / (2 * cos(t)^2))
    stats::pnorm(h) * stats::pnorm(k) + stats::integrate(g, 0, asin(r), rel.tol = 1e-13, abs.tol = 0)$value / (2 * pi)
  }
  pair <- function(i, j) {
    ok <- !is.na(M[, i]) & !is.na(M[, j])
    x <- M[ok, i]; y <- M[ok, j]; w <- f[ok]
    n00 <- sum(w[x == 0 & y == 0]); n01 <- sum(w[x == 0 & y == 1]); n10 <- sum(w[x == 1 & y == 0]); n11 <- sum(w[x == 1 & y == 1])
    n <- n00 + n01 + n10 + n11
    if (min(n00, n01, n10, n11) == 0) refuse("a 2x2 table with an empty cell")
    p <- stats::fisher.test(matrix(c(n00, n10, n01, n11), 2))$p.value
    if (edwards) {
      a <- n00 * n11 / (n01 * n10); c4 <- pi / 4
      rho <- (a^c4 - 1) / (a^c4 + 1)
      se <- c4 * 2 * a^c4 / (a^c4 + 1)^2 * sqrt(1 / n00 + 1 / n01 + 1 / n10 + 1 / n11)
      return(c(rho = rho, se = se, p = p, N = n))
    }
    h <- stats::qnorm((n00 + n01) / n); k <- stats::qnorm((n00 + n10) / n)
    phi2 <- function(r) exp(-(h^2 - 2 * r * h * k + k^2) / (2 * (1 - r^2))) / (2 * pi * sqrt(1 - r^2))
    cells <- function(r) { p00 <- bvn(h, k, r); c(p00, stats::pnorm(h) - p00, stats::pnorm(k) - p00, 1 - stats::pnorm(h) - stats::pnorm(k) + p00) }
    score <- function(r) { q <- cells(r); phi2(r) * (n00 / q[1] - n01 / q[2] - n10 / q[3] + n11 / q[4]) }
    lo <- -0.999; hi <- 0.999
    if (!(score(lo) > 0 && score(hi) < 0)) refuse("a correlation at the edge, -1 or 1")
    rho <- stats::uniroot(score, c(lo, hi), tol = 1e-15)$root
    # the information of (h, k, rho): n times the sum over the cells of their gradients' outer products over p
    s <- sqrt(1 - rho^2)
    g00 <- c(stats::dnorm(h) * stats::pnorm((k - rho * h) / s), stats::dnorm(k) * stats::pnorm((h - rho * k) / s), phi2(rho))
    G <- rbind(g00, c(stats::dnorm(h), 0, 0) - g00, c(0, stats::dnorm(k), 0) - g00, c(-stats::dnorm(h), -stats::dnorm(k), 0) + g00)
    q <- cells(rho)
    I <- n * crossprod(G / sqrt(q))
    se <- sqrt(solve(I)[3, 3])
    c(rho = rho, se = se, p = p, N = n)
  }
  k <- length(vars)
  m <- function(v) matrix(v, k, k, dimnames = list(vars, vars))
  Rho <- m(NA_real_); Se <- m(NA_real_); P <- m(NA_real_); Nobs <- m(NA_real_)
  for (i in seq_len(k)) {
    ok <- !is.na(M[, i]); Nobs[i, i] <- sum(f[ok]); Rho[i, i] <- 1; Se[i, i] <- 0
    for (j in seq_len(i - 1)) {
      o <- pair(i, j)
      Rho[i, j] <- Rho[j, i] <- o[["rho"]]; Se[i, j] <- Se[j, i] <- o[["se"]]
      P[i, j] <- P[j, i] <- o[["p"]]; Nobs[i, j] <- Nobs[j, i] <- o[["N"]]
    }
  }
  np <- k * (k - 1) / 2
  if (adjust == "bonferroni") P[] <- pmin(1, P * np)
  if (adjust == "sidak") P[] <- pmin(1, 1 - (1 - P)^np)
  ev <- eigen(Rho, symmetric = TRUE, only.values = TRUE)$values
  if (posdef && any(ev < 0)) refuse("posdef with a correlation matrix that is not positive definite")
  R <- list(p = P[2, 1], se_rho = Se[2, 1], nneg = sum(ev < 0), rho = Rho[2, 1], N = Nobs[2, 1],
            method = if (edwards) "Edwards" else "ml", P = P, Se_Rho = Se, Rho = Rho, Nobs = Nobs)
  if (!show || !table) return(R)
  if (k == 2 && !matrix) {
    cat("\n", sprintf("%16s =%13s", "Number of obs", format(R$N, scientific = FALSE)), "\n", sep = "")
    cat(sprintf("%16s =%13.4f", "Tetrachoric rho", R$rho), "\n", sep = "")
    cat(sprintf("%16s =%13.4f", "Std error", R$se_rho), "\n\n", sep = "")
    cat("Test of H0: ", vars[1], " and ", vars[2], " are independent\n", sep = "")
    cat(sprintf("%16s =%13.4f", "2-sided exact P", R$p), "\n", sep = "")
    return(R)
  }
  order <- c("rho", "se", "p", "obs")
  st <- order[order %in% stats]
  if (!"obs" %in% st) cat("(obs=", format(R$N, scientific = FALSE), ")\n", sep = "")
  cat("\n")
  if (length(st) > 1) {
    key <- c(rho = "rho", se = "Std. error", p = "2-sided exact P", obs = "Number of obs")
    cat("+-------------------+\n|  Key              |\n|-------------------|\n")
    for (s in st) cat(sprintf("|   %-15s |", key[[s]]), "\n", sep = "")
    cat("+-------------------+\n\n")
  }
  cat("             |", paste0(" ", formatC(stata_tab_abbrev(vars, 8), width = 8), collapse = ""), "\n", sep = "")
  cat(strrep("-", 13), "+", strrep("-", 9 * k), "\n", sep = "")
  shown <- function(i, j) i == j || is.na(print) || (!is.na(P[i, j]) && P[i, j] <= print)
  for (i in seq_len(k)) {
    for (s in seq_along(st)) {
      js <- seq_len(i)
      if (st[s] == "p") js <- js[js != i]
      # each cell 8 wide after a mark (* for star(), else a blank), the last mark ending the line
      cells <- vapply(js, function(j) {
        if (!shown(i, j)) return(strrep(" ", 8))
        v <- switch(st[s], rho = Rho[i, j], se = Se[i, j], p = P[i, j], obs = Nobs[i, j])
        if (st[s] == "obs") sprintf("%8.0f", v) else sprintf("%8.4f", v)
      }, "")
      marks <- vapply(js, function(j) {
        if (st[s] == "rho" && i != j && !is.na(star) && shown(i, j) && P[i, j] <= star) "*" else " "
      }, "")
      line <- paste0(paste0(c(" ", marks[-length(marks)])[seq_along(cells)], cells, collapse = ""), if (length(cells)) marks[length(marks)] else " ")
      lab <- if (s == 1) stata_tab_abbrev(vars[i], 12) else ""
      cat(sprintf("%12s |", lab), line, "\n", sep = "")
    }
    if (length(st) > 1) cat("             |\n")
  }
  R
}"##;

/// The r() names `tetrachoric` leaves, in Stata's order.
const R_TETRACHORIC: &[&str] = &[
    "p", "se_rho", "nneg", "rho", "N", "method", "P", "Se_Rho", "Rho", "Nobs",
];

impl Session {
    /// `tetrachoric varlist [if] [in] [fw] [, options]`.
    pub(super) fn tetrachoric(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "tetrachoric")?;
        let vars: Vec<&str> = c.parts.main.split_whitespace().collect();
        if let Some(v) = vars.iter().find(|v| !is_name(v)) {
            return Err(no(format!(
                "tetrachoric {v}: the variables written out are translated"
            )));
        }
        if vars.len() < 2 {
            out.line(format!(
                "stop({}, call. = FALSE)",
                r_text("too few variables specified (Stata: r(102))")
            ));
            return Ok(());
        }
        if let Some(o) = opts.other(&[
            ("stats", 5),
            ("edwards", 7),
            ("print", 5),
            ("star", 4),
            ("bonferroni", 10),
            ("sidak", 5),
            ("pw", 2),
            ("matrix", 6),
            ("notable", 7),
            ("posdef", 6),
        ]) {
            return Err(no(format!(
                "tetrachoric with the option {o} is not translated yet"
            )));
        }
        let fw = match &c.parts.weight {
            None => None,
            Some(w)
                if matches!(w.kind, crate::lang::syntax::WeightKind::Frequency)
                    && is_name(w.exp.trim()) =>
            {
                Some(w.exp.trim().to_string())
            }
            Some(_) => return Err(no(
                "tetrachoric with weights other than a frequency-weight variable is not translated",
            )),
        };
        let mut args = String::new();
        if let Some(s) = opts.arg("stats", 5) {
            let mut st = Vec::new();
            for w in s.split_whitespace() {
                if !matches!(w, "rho" | "se" | "p" | "obs") {
                    return Err(no(format!("tetrachoric, stats({w}) is not translated yet")));
                }
                st.push(r_text(w));
            }
            if !st.is_empty() {
                args.push_str(&format!(", stats = c({})", st.join(", ")));
            }
        }
        for (o, m) in [("print", 5), ("star", 4)] {
            if let Some(v) = opts.arg(o, m) {
                let x: f64 = v
                    .trim()
                    .parse()
                    .ok()
                    .filter(|x: &f64| (0.0..=1.0).contains(x))
                    .ok_or_else(|| {
                        no(format!(
                            "tetrachoric, {o}({v}): a level from 0 to 1 is translated"
                        ))
                    })?;
                args.push_str(&format!(", {o} = {}", r_number(x)));
            }
        }
        if opts.has("bonferroni", 10) && opts.has("sidak", 5) {
            return Err(no(
                "tetrachoric: bonferroni and sidak together are not translated",
            ));
        }
        if opts.has("bonferroni", 10) {
            args.push_str(", adjust = \"bonferroni\"");
        }
        if opts.has("sidak", 5) {
            args.push_str(", adjust = \"sidak\"");
        }
        for (o, m, a) in [
            ("edwards", 7, ", edwards = TRUE"),
            ("pw", 2, ", pw = TRUE"),
            ("matrix", 6, ", matrix = TRUE"),
            ("notable", 7, ", table = FALSE"),
            ("posdef", 6, ", posdef = TRUE"),
        ] {
            if opts.has(o, m) {
                args.push_str(a);
            }
        }
        if let Some(f) = &fw {
            args.push_str(&format!(", fw = {}", r_text(f)));
        }
        if self.quiet(c) {
            args.push_str(", show = FALSE");
        }
        let rows = self.rows_for(c, out)?;
        for h in [
            "stata_tetrachoric",
            "stata_fmt",
            "stata_tables",
            "stata_returned",
        ] {
            self.helpers.insert(h);
        }
        let q: Vec<String> = vars.iter().map(|v| r_text(v)).collect();
        out.line(format!(
            "stata_r <- stata_tetrachoric({rows}, c({}){args})",
            q.join(", ")
        ));
        self.r_new = Some((R_TETRACHORIC, "tetrachoric".into()));
        Ok(())
    }
}
