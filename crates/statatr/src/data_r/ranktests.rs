//! The rank tests as Stata 19 prints them and the r() they leave
//! (`plan/stata/rank-tests.md`): `ranksum var, by(g)`, `signrank var = exp`,
//! `signtest var = exp`, `kwallis var, by(g)` and `spearman var1 var2
//! [var3 ...]`. Midranks for ties, the variances adjusted for them, and
//! (for ranksum and signrank with up to 200 rows, as Stata's default) the
//! exact p-values from the permutation distribution of the midranks.

use super::*;

pub(super) const RANKTESTS_HELPER: &str = r##"stata_num <- function(u) { u <- unclass(u); attributes(u) <- NULL; as.numeric(u) }
stata_vlabel <- function(g, v) {
  vl <- attr(g, "labels", exact = TRUE)
  vapply(v, function(x) {
    k <- if (!is.null(vl)) names(vl)[match(x, stata_num(vl))] else NA
    if (length(k) && !is.na(k)) k else stata_fmt(x, "%9.0g", trim = TRUE)
  }, "")
}
# the counts of each sum of k of the (doubled, so whole) midranks r: P(sum <= s) and P(sum >= s)
stata_exact_tails <- function(r2, k, s2) {
  tot <- sum(r2)
  f <- matrix(0, k + 1, tot + 1)
  f[1, 1] <- 1
  for (i in seq_along(r2)) {
    ri <- r2[i]
    for (j in min(k, i):1) {
      f[j + 1, (ri + 1):(tot + 1)] <- f[j + 1, (ri + 1):(tot + 1)] + f[j, 1:(tot + 1 - ri)]
    }
  }
  d <- f[k + 1, ] / sum(f[k + 1, ])
  idx <- 0:tot
  c(lower = sum(d[idx <= s2 + 1e-9]), upper = sum(d[idx >= s2 - 1e-9]))
}
stata_ranksum <- function(x, g, xname, gname, show = TRUE) {
  x <- stata_num(x)
  gv <- stata_num(g)
  keep <- !is.na(x) & !is.na(gv)
  x <- x[keep]
  gv <- gv[keep]
  lv <- sort(unique(gv))
  if (length(lv) != 2) stop(gname, " takes on ", length(lv), " values, not 2 (Stata: r(420))", call. = FALSE)
  N <- length(x)
  r <- rank(x)
  in1 <- gv == lv[1]
  n1 <- sum(in1)
  n2 <- N - n1
  w1 <- sum(r[in1])
  w2 <- sum(r[!in1])
  e1 <- n1 * (N + 1) / 2
  e2 <- n2 * (N + 1) / 2
  v0 <- n1 * n2 * (N + 1) / 12
  t <- table(x)
  adj <- -n1 * n2 * sum(t^3 - t) / (12 * N * (N - 1))
  va <- v0 + adj
  z <- (w1 - e1) / sqrt(va)
  rr <- list(N = N, N_1 = n1, N_2 = n2, group1 = lv[1], sum_obs = w1, sum_exp = e1, Var_a = va, z = z,
             p = 2 * stats::pnorm(-abs(z)), p_l = stats::pnorm(z), p_u = stats::pnorm(-z))
  if (N <= 200) {
    ex <- stata_exact_tails(round(2 * r), n1, round(2 * w1))
    rr$p_exact <- min(1, 2 * min(ex[["lower"]], ex[["upper"]]))
    rr$p_l_exact <- ex[["lower"]]
    rr$p_u_exact <- ex[["upper"]]
  }
  if (!show) return(rr)
  g10 <- function(v) stata_fmt(v, "%10.0g", trim = TRUE)
  lab <- stata_vlabel(g, lv)
  cat("\nTwo-sample Wilcoxon rank-sum (Mann–Whitney) test\n\n")
  cat(sprintf("%12s |      Obs    Rank sum    Expected", stata_tab_abbrev(gname, 12)), "\n", sep = "")
  mid <- paste0(strrep("-", 13), "+", strrep("-", 33), "\n")
  cat(mid)
  cat(sprintf("%12s |%9s%12s%12s", lab[1], n1, g10(w1), g10(e1)), "\n", sep = "")
  cat(sprintf("%12s |%9s%12s%12s", lab[2], n2, g10(w2), g10(e2)), "\n", sep = "")
  cat(mid)
  cat(sprintf("%12s |%9s%12s%12s", "Combined", N, g10(w1 + w2), g10(e1 + e2)), "\n\n", sep = "")
  cat(sprintf("%-21s%10.2f\n%-21s%10.2f\n%21s----------\n%-21s%10.2f\n\n", "Unadjusted variance", v0, "Adjustment for ties", adj, "",
              "Adjusted variance", va))
  v <- vapply(lv, stata_fmt, "", "%9.0g", trim = TRUE)
  cat("H0: ", xname, "(", gname, "==", v[1], ") = ", xname, "(", gname, "==", v[2], ")\n", sep = "")
  cat(sprintf("%10s = %6.3f\nProb > |z| = %6.4f\n", "z", z, rr$p))
  if (!is.null(rr$p_exact)) cat(sprintf("Exact prob = %6.4f\n", rr$p_exact))
  rr
}
stata_signrank <- function(x, y, lhs, rhs, show = TRUE) {
  x <- stata_num(x)
  y <- rep_len(stata_num(y), length(x))
  keep <- !is.na(x) & !is.na(y)
  d <- x[keep] - y[keep]
  N <- length(d)
  r <- rank(abs(d))
  pos <- d > 0
  neg <- d < 0
  zer <- d == 0
  sp <- sum(r[pos])
  sn <- sum(r[neg])
  sz <- sum(r[zer])
  e <- (sp + sn) / 2
  v0 <- N * (N + 1) * (2 * N + 1) / 24
  t <- table(abs(d))
  adj_t <- -sum(t^3 - t) / 48
  z0 <- sum(zer)
  adj_z <- -z0 * (z0 + 1) * (2 * z0 + 1) / 24
  va <- v0 + adj_t + adj_z
  z <- (sp - e) / sqrt(va)
  rr <- list(N = N, N_pos = sum(pos), N_neg = sum(neg), N_tie = z0, sum_pos = sp, sum_neg = sn, Var_a = va, z = z,
             p = 2 * stats::pnorm(-abs(z)), p_l = stats::pnorm(z), p_u = stats::pnorm(-z))
  if (N <= 200) {
    # each nonzero difference positive or negative with chance 1/2: the sum of the positive ranks
    r2 <- round(2 * r[!zer])
    tot <- sum(r2)
    f <- numeric(tot + 1)
    f[1] <- 1
    for (ri in r2) f <- f + c(rep(0, ri), f[seq_len(tot + 1 - ri)])
    dd <- f / sum(f)
    idx <- 0:tot
    s2 <- round(2 * sp)
    lo <- sum(dd[idx <= s2])
    hi <- sum(dd[idx >= s2])
    rr$p_exact <- min(1, 2 * min(lo, hi))
    rr$p_l_exact <- lo
    rr$p_u_exact <- hi
  }
  if (!show) return(rr)
  g10 <- function(v) stata_fmt(v, "%10.0g", trim = TRUE)
  cat("\nWilcoxon signed-rank test\n\n")
  cat("        Sign |      Obs   Sum ranks    Expected\n")
  mid <- paste0(strrep("-", 13), "+", strrep("-", 33), "\n")
  cat(mid)
  cat(sprintf("%12s |%9s%12s%12s", "Positive", sum(pos), g10(sp), g10(e)), "\n", sep = "")
  cat(sprintf("%12s |%9s%12s%12s", "Negative", sum(neg), g10(sn), g10(e)), "\n", sep = "")
  cat(sprintf("%12s |%9s%12s%12s", "Zero", z0, g10(sz), g10(sz)), "\n", sep = "")
  cat(mid)
  cat(sprintf("%12s |%9s%12s%12s", "All", N, g10(sp + sn + sz), g10(2 * e + sz)), "\n\n", sep = "")
  cat(sprintf("%-21s%10.2f\n%-21s%10.2f\n%-21s%10.2f\n%21s----------\n%-21s%10.2f\n\n", "Unadjusted variance", v0,
              "Adjustment for ties", adj_t, "Adjustment for zeros", adj_z, "", "Adjusted variance", va))
  cat("H0: ", lhs, " = ", rhs, "\n", sep = "")
  cat(sprintf("%10s = %6.3f\nProb > |z| = %6.4f\n", "z", z, rr$p))
  if (!is.null(rr$p_exact)) cat(sprintf("Exact prob = %6.4f\n", rr$p_exact))
  rr
}
stata_signtest <- function(x, y, lhs, rhs, show = TRUE) {
  x <- stata_num(x)
  y <- rep_len(stata_num(y), length(x))
  keep <- !is.na(x) & !is.na(y)
  d <- x[keep] - y[keep]
  N <- length(d)
  np <- sum(d > 0)
  nn <- sum(d < 0)
  nz <- sum(d == 0)
  m <- np + nn
  pu <- stats::pbinom(np - 1, m, .5, lower.tail = FALSE)
  pl <- stats::pbinom(nn - 1, m, .5, lower.tail = FALSE)
  big <- max(np, nn)
  p <- min(1, 2 * stats::pbinom(big - 1, m, .5, lower.tail = FALSE))
  rr <- list(N = N, N_pos = np, N_neg = nn, N_tie = nz, p = p, p_l = pl, p_u = pu)
  if (!show) return(rr)
  cat("\nSign test\n\n        Sign |    Observed    Expected\n")
  mid <- paste0(strrep("-", 13), "+", strrep("-", 24), "\n")
  cat(mid)
  e <- m / 2
  g <- function(v) stata_fmt(v, "%10.0g", trim = TRUE)
  cat(sprintf("%12s |%12s%12s", "Positive", np, g(e)), "\n", sep = "")
  cat(sprintf("%12s |%12s%12s", "Negative", nn, g(e)), "\n", sep = "")
  cat(sprintf("%12s |%12s%12s", "Zero", nz, nz), "\n", sep = "")
  cat(mid)
  cat(sprintf("%12s |%12s%12s", "All", N, N), "\n", sep = "")
  dif <- paste0(lhs, " - ", rhs)
  cat("\nOne-sided tests:\n")
  cat("  H0: median of ", dif, " = 0 vs.\n  Ha: median of ", dif, " > 0\n", sep = "")
  cat("      Pr(#positive >= ", np, ") =\n", sep = "")
  cat(sprintf("         Binomial(n = %d, x >= %d, p = 0.5) = %6.4f", m, np, pu), "\n\n", sep = "")
  cat("  H0: median of ", dif, " = 0 vs.\n  Ha: median of ", dif, " < 0\n", sep = "")
  cat("      Pr(#negative >= ", nn, ") =\n", sep = "")
  cat(sprintf("         Binomial(n = %d, x >= %d, p = 0.5) = %6.4f", m, nn, pl), "\n\n", sep = "")
  cat("Two-sided test:\n")
  cat("  H0: median of ", dif, " = 0 vs.\n  Ha: median of ", dif, " != 0\n", sep = "")
  cat("      Pr(#positive >= ", big, " or #negative >= ", big, ") =\n", sep = "")
  cat(sprintf("         min(1, 2*Binomial(n = %d, x >= %d, p = 0.5)) = %6.4f", m, big, p), "\n", sep = "")
  rr
}
stata_kwallis <- function(x, g, gname, show = TRUE) {
  x <- stata_num(x)
  gv <- stata_num(g)
  keep <- !is.na(x) & !is.na(gv)
  x <- x[keep]
  gv <- gv[keep]
  lv <- sort(unique(gv))
  N <- length(x)
  r <- rank(x)
  n <- vapply(lv, function(v) sum(gv == v), 0)
  rs <- vapply(lv, function(v) sum(r[gv == v]), 0)
  H <- 12 / (N * (N + 1)) * sum(rs^2 / n) - 3 * (N + 1)
  t <- table(x)
  Ha <- H / (1 - sum(t^3 - t) / (N^3 - N))
  df <- length(lv) - 1
  # in Stata 19's order
  rr <- list(p_adj = stats::pchisq(Ha, df, lower.tail = FALSE), p = stats::pchisq(H, df, lower.tail = FALSE),
             chi2_adj = Ha, df = df, chi2 = H)
  if (!show) return(rr)
  lab <- stata_vlabel(g, lv)
  w1 <- max(nchar(gname), nchar(lab))
  obs <- format(n, scientific = FALSE)
  w2 <- max(3, nchar(obs))
  sums <- sprintf("%.2f", rs)
  w3 <- max(8, nchar(sums))
  pad <- function(s, w) formatC(s, width = w)
  edge <- paste0("  +", strrep("-", w1 + w2 + w3 + 8), "+\n")
  cat("\nKruskal–Wallis equality-of-populations rank test\n\n", edge, sep = "")
  cat("  | ", pad(gname, w1), " | ", pad("Obs", w2), " | ", pad("Rank sum", w3), " |\n", sep = "")
  cat("  |", strrep("-", w1 + 2), "+", strrep("-", w2 + 2), "+", strrep("-", w3 + 2), "|\n", sep = "")
  for (i in seq_along(lv)) cat("  | ", pad(lab[i], w1), " | ", pad(obs[i], w2), " | ", pad(sums[i], w3), " |\n", sep = "")
  cat(edge, "\n", sep = "")
  cat(sprintf("  chi2(%d) = %6.3f\n     Prob = %6.4f\n\n", df, H, rr$p))
  cat(sprintf("  chi2(%d) with ties = %6.3f\n%19s = %6.4f\n", df, Ha, "Prob", rr$p_adj))
  rr
}
stata_spearman <- function(d, vars, show = TRUE) {
  X <- as.matrix(as.data.frame(lapply(d[vars], stata_num)))
  X <- X[stats::complete.cases(X), , drop = FALSE]
  N <- nrow(X)
  R <- stats::cor(apply(X, 2, rank))
  dimnames(R) <- list(vars, vars)
  if (length(vars) == 2) {
    rho <- R[1, 2]
    t <- rho * sqrt((N - 2) / (1 - rho^2))
    rr <- list(N = N, rho = rho, p = 2 * stats::pt(-abs(t), N - 2), p_l = stats::pt(t, N - 2), p_u = stats::pt(-t, N - 2))
    if (!show) return(list(N = N, rho = rho))
    cat(sprintf("\nNumber of observations = %6d\n%22s = %6.4f\n\nTest of H0: %s and %s are independent\n%22s = %6.4f\n",
                N, "Spearman's rho", rho, vars[1], vars[2], "Prob", rr$p))
    return(rr)
  }
  rr <- list(N = N, rho = R[nrow(R), nrow(R) - 1], Rho = R)
  if (!show) return(rr)
  k <- length(vars)
  cat("\nNumber of observations = ", N, "\n\n", sep = "")
  cat("             |", paste0(formatC(stata_tab_abbrev(vars, 8), width = 9), collapse = ""), "\n", sep = "")
  cat(strrep("-", 13), "+", strrep("-", 9 * k), "\n", sep = "")
  for (i in seq_len(k)) {
    cells <- vapply(seq_len(i), function(j) sprintf("%9.4f", R[i, j]), "")
    cat(sprintf("%12s |", stata_tab_abbrev(vars[i], 12)), paste0(cells, collapse = ""), " \n", sep = "")
  }
  rr
}
# Kendall's tau of two variables: the score S over pairs, tau-a S/n0, tau-b with the ties taken out,
# the variance of S corrected for ties, the two-sided p continuity corrected
stata_ktau_pair <- function(x, y) {
  x15 <- signif(x, 15); y15 <- signif(y, 15)
  n <- length(x)
  S <- 0
  if (n > 1) for (i in seq_len(n - 1)) {
    j <- (i + 1):n
    S <- S + sum(sign(x15[i] - x15[j]) * sign(y15[i] - y15[j]))
  }
  n0 <- n * (n - 1) / 2
  # the ties for tau-b and the variance: equal values exactly
  tx <- tabulate(match(x, unique(x))); ty <- tabulate(match(y, unique(y)))
  t1 <- sum(tx * (tx - 1) / 2); u1 <- sum(ty * (ty - 1) / 2)
  v <- (n * (n - 1) * (2 * n + 5) - sum(tx * (tx - 1) * (2 * tx + 5)) - sum(ty * (ty - 1) * (2 * ty + 5))) / 18 +
    sum(tx * (tx - 1) * (tx - 2)) * sum(ty * (ty - 1) * (ty - 2)) / (9 * n * (n - 1) * (n - 2)) +
    sum(tx * (tx - 1)) * sum(ty * (ty - 1)) / (2 * n * (n - 1))
  se <- sqrt(v)
  p <- 2 * stats::pnorm(-max(abs(S) - 1, 0) / se)
  fin <- function(z) if (is.finite(z)) z else NA_real_
  c(N = n, tau_a = fin(S / n0), tau_b = fin(S / sqrt((n0 - t1) * (n0 - u1))), score = S, se_score = fin(se),
    p = fin(min(1, p)), ties = (t1 + u1) > 0)
}
stata_ktau <- function(d, vars, stats = "taua", pw = FALSE, adjust = "none", print = NA, star = NA,
                       matrix = FALSE, show = TRUE) {
  # the score compares values to 15 significant digits (one bit apart: tied; 1e-12 apart: not), the ties
  # for tau-b and the variance are exact equality, as Stata 19 counts them
  X <- lapply(vars, function(v) stata_num(d[[v]]))
  k <- length(vars)
  if (!pw) {
    ok <- Reduce(`&`, lapply(X, function(x) !is.na(x)))
    X <- lapply(X, function(x) x[ok])
  }
  pair <- function(i, j) {
    ok <- !is.na(X[[i]]) & !is.na(X[[j]])
    stata_ktau_pair(X[[i]][ok], X[[j]][ok])
  }
  if (k == 2 && !matrix) {
    o <- pair(2, 1)
    rr <- as.list(o[c("N", "tau_a", "tau_b", "score", "se_score", "p")])
    if (!show) return(rr)
    f <- function(v, w) if (is.na(v)) sprintf(paste0("%", w, "s"), ".") else sprintf(paste0("%", w, ".4f"), v)
    cat("\n  Number of obs = ", sprintf("%7.0f", o[["N"]]), "\n", sep = "")
    cat("Kendall's tau-a = ", f(o[["tau_a"]], 12), "\n", sep = "")
    cat("Kendall's tau-b = ", f(o[["tau_b"]], 12), "\n", sep = "")
    cat("Kendall's score = ", sprintf("%7.0f", o[["score"]]), "\n", sep = "")
    cat("    SE of score = ", sprintf("%11.3f", o[["se_score"]]), if (o[["ties"]] == 1) "   (corrected for ties)", "\n\n", sep = "")
    cat("Test of H0: ", vars[1], " and ", vars[2], " are independent\n", sep = "")
    cat("     Prob > |z| = ", f(o[["p"]], 12), "  (continuity corrected)\n", sep = "")
    return(invisible(rr))
  }
  m <- function() matrix(NA_real_, k, k, dimnames = list(vars, vars))
  M <- list(Nobs = m(), Tau_a = m(), Tau_b = m(), Score = m(), Se_Score = m(), P = m())
  for (i in seq_len(k)) for (j in seq_len(i)) {
    o <- pair(i, j)
    vals <- c(o[["N"]], o[["tau_a"]], o[["tau_b"]], o[["score"]], o[["se_score"]], if (i == j) NA else o[["p"]])
    for (s in seq_along(M)) M[[s]][i, j] <- M[[s]][j, i] <- vals[s]
  }
  np <- k * (k - 1) / 2
  if (adjust == "bonferroni") M$P[] <- pmin(1, M$P * np)
  if (adjust == "sidak") M$P[] <- pmin(1, 1 - (1 - M$P)^np)
  rr <- list(N = M$Nobs[k, k - 1], tau_a = M$Tau_a[k, k - 1], tau_b = M$Tau_b[k, k - 1], score = M$Score[k, k - 1],
             se_score = M$Se_Score[k, k - 1], p = M$P[k, k - 1])
  rr <- c(rr, M)
  if (!show) return(rr)
  order <- c("taua", "taub", "score", "se", "obs", "p")
  st <- order[order %in% stats]
  src <- list(taua = "Tau_a", taub = "Tau_b", score = "Score", se = "Se_Score", obs = "Nobs", p = "P")
  key <- c(taua = "tau_a", taub = "tau_b", score = "score", se = "se of score", obs = "Number of obs", p = "p-value")
  cat("\n")
  if (!"obs" %in% st) {
    nn <- M$Nobs[lower.tri(M$Nobs, diag = TRUE)]
    if (length(unique(nn)) > 1) cat("note: number of observations varies.\n\n") else cat("Number of observations = ", nn[1], "\n\n", sep = "")
  }
  if (!identical(st, "taua")) {
    cat("+-----------------+\n|  Key            |\n|-----------------|\n")
    for (s in st) cat(sprintf("|   %-13s |", key[[s]]), "\n", sep = "")
    cat("+-----------------+\n\n")
  }
  cat("             |", paste0(" ", formatC(stata_tab_abbrev(vars, 8), width = 10), collapse = ""), "\n", sep = "")
  cat(strrep("-", 13), "+", strrep("-", 11 * k), "\n", sep = "")
  cell <- function(s, i, j) {
    v <- M[[src[[s]]]][i, j]
    if (is.na(v)) return(sprintf("%10s", "."))
    sprintf(if (s == "obs") "%10.0f" else "%10.4f", v)
  }
  for (i in seq_len(k)) {
    for (s in seq_along(st)) {
      cells <- vapply(seq_len(i), function(j) {
        p <- M$P[i, j]
        if (!is.na(print) && !is.na(p) && p > print) return(strrep(" ", 11))
        mark <- if (s == 1 && !is.na(star) && !is.na(p) && p <= star) "*" else " "
        paste0(cell(st[s], i, j), mark)
      }, "")
      lab <- if (s == 1) stata_tab_abbrev(vars[i], 12) else ""
      cat(sprintf("%12s | ", lab), paste0(cells, collapse = ""), "\n", sep = "")
    }
    if (length(st) > 1) cat("             |\n")
  }
  invisible(rr)
}"##;

const R_RANKSUM: &[&str] = &[
    "N",
    "N_1",
    "N_2",
    "group1",
    "sum_obs",
    "sum_exp",
    "Var_a",
    "z",
    "p",
    "p_l",
    "p_u",
    "p_exact",
    "p_l_exact",
    "p_u_exact",
];
const R_SIGNRANK: &[&str] = &[
    "N",
    "N_pos",
    "N_neg",
    "N_tie",
    "sum_pos",
    "sum_neg",
    "Var_a",
    "z",
    "p",
    "p_l",
    "p_u",
    "p_exact",
    "p_l_exact",
    "p_u_exact",
];
const R_SIGNTEST: &[&str] = &["N", "N_pos", "N_neg", "N_tie", "p", "p_l", "p_u"];
const R_KWALLIS: &[&str] = &["df", "chi2", "p", "chi2_adj", "p_adj"];
const R_SPEARMAN: &[&str] = &["N", "rho", "p", "p_l", "p_u"];
/// What `quietly spearman x y` keeps: the p-values are not Stata's.
const R_SPEARMAN_QUIET: &[&str] = &["N", "rho"];
const R_KTAU: &[&str] = &[
    "N", "tau_a", "tau_b", "score", "se_score", "p", "Nobs", "Tau_a", "Tau_b", "Score", "Se_Score",
    "P",
];

impl Session {
    /// `ranksum`, `signrank`, `signtest`, `kwallis` and `spearman`.
    pub(super) fn rank_test(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let cmd = c.name.as_str();
        self.no_by(c, cmd)?;
        if c.parts.weight.is_some() {
            return Err(no(format!("{cmd} with weights is not translated yet")));
        }
        if cmd == "ktau" {
            return self.ktau(c, opts, out);
        }
        let known: &[(&str, usize)] = match cmd {
            "ranksum" | "kwallis" => &[("by", 2)],
            _ => &[],
        };
        if let Some(o) = opts.other(known) {
            return Err(no(format!(
                "{cmd} with the option {o} is not translated yet"
            )));
        }
        let rows = self.rows_for(c, out)?;
        let col = |v: &str| format!("{rows}[[{}]]", r_text(v));
        let main = c.parts.main.trim();
        let quiet = if self.quiet(c) { ", show = FALSE" } else { "" };
        let (call, names): (String, &'static [&'static str]) = match cmd {
            "ranksum" | "kwallis" => {
                let g = opts
                    .arg("by", 2)
                    .map(|g| g.trim().to_string())
                    .ok_or_else(|| no(format!("{cmd} needs by() (Stata: r(198))")))?;
                if !is_name(main) || !is_name(&g) {
                    return Err(no(format!(
                        "{cmd} var, by(g): one variable each is translated"
                    )));
                }
                if cmd == "ranksum" {
                    (
                        format!(
                            "stata_ranksum({}, {}, {}, {}{quiet})",
                            col(main),
                            col(&g),
                            r_text(main),
                            r_text(&g)
                        ),
                        R_RANKSUM,
                    )
                } else {
                    (
                        format!(
                            "stata_kwallis({}, {}, {}{quiet})",
                            col(main),
                            col(&g),
                            r_text(&g)
                        ),
                        R_KWALLIS,
                    )
                }
            }
            "signrank" | "signtest" => {
                let Some((l, r)) = main.split_once('=') else {
                    return Err(no(format!("{cmd} var = exp is translated")));
                };
                let (l, r) = (
                    l.trim().trim_end_matches('='),
                    r.trim().trim_start_matches('='),
                );
                let (l, r) = (l.trim(), r.trim());
                if !is_name(l) {
                    return Err(no(format!("{cmd} {l}: one variable is translated")));
                }
                let rhs = if is_name(r) {
                    col(r)
                } else if let Ok(v) = r.parse::<f64>() {
                    r_number(v)
                } else {
                    return Err(no(format!(
                        "{cmd} {l} = {r}: a variable or a number is translated"
                    )));
                };
                let f = if cmd == "signrank" {
                    "stata_signrank"
                } else {
                    "stata_signtest"
                };
                (
                    format!(
                        "{f}({}, {rhs}, {}, {}{quiet})",
                        col(l),
                        r_text(l),
                        r_text(r)
                    ),
                    if cmd == "signrank" {
                        R_SIGNRANK
                    } else {
                        R_SIGNTEST
                    },
                )
            }
            _ => {
                let vars: Vec<&str> = main.split_whitespace().collect();
                if vars.len() < 2 || vars.iter().any(|v| !is_name(v)) {
                    return Err(no("spearman takes two or more variable names written out"));
                }
                // Stata 19 takes the p-value of two from a four-parameter beta fitted to the
                // permutation moments ([R] spearman), not reproduced yet: the matrix of three or
                // more only
                // Under quietly the rank correlation and N are all that is kept: r(rho) and r(N) are
                // exact; the p-value is not (r(p), r(p_l) and r(p_u) are then not known).
                if vars.len() == 2 && quiet.is_empty() {
                    return Err(no(
                        "spearman of two variables (its p-value, from a four-parameter beta as Stata 19 fits it) is not translated yet; quietly spearman gives r(rho) and r(N)",
                    ));
                }
                let q: Vec<String> = vars.iter().map(|v| r_text(v)).collect();
                (
                    format!("stata_spearman({rows}, c({}){quiet})", q.join(", ")),
                    if vars.len() == 2 {
                        R_SPEARMAN_QUIET
                    } else {
                        R_SPEARMAN
                    },
                )
            }
        };
        for h in [
            "stata_ranktests",
            "stata_fmt",
            "stata_tables",
            "stata_returned",
        ] {
            self.helpers.insert(h);
        }
        out.line(format!("stata_r <- {call}"));
        if cmd == "spearman" && names.len() != 2 {
            // the matrix form's r() was not checked against Stata
            self.r_ret = super::returned::Returned::Lost(
                "r() after spearman of three or more variables was not checked against Stata"
                    .into(),
            );
        } else {
            self.r_new = Some((names, cmd.into()));
        }
        Ok(())
    }

    /// `ktau varlist [if] [in] [, stats(...) print(#) star(#) bonferroni sidak pw matrix]`.
    fn ktau(&mut self, c: &Command, opts: &Opts, out: &mut Out) -> Result<(), Untranslated> {
        let known: &[(&str, usize)] = &[
            ("stats", 2),
            ("print", 2),
            ("star", 2),
            ("bonferroni", 3),
            ("sidak", 3),
            ("pw", 2),
            ("matrix", 3),
        ];
        if let Some(o) = opts
            .0
            .iter()
            .find(|o| !known.iter().any(|(f, m)| o.is(f, *m)))
        {
            let paren = if o.arg.is_some() { "()" } else { "" };
            out.line(format!(
                "stop({}, call. = FALSE)",
                r_text(&format!(
                    "option {}{paren} not allowed (Stata: r(198))",
                    o.name
                ))
            ));
            return Ok(());
        }
        let vars: Vec<&str> = c.parts.main.split_whitespace().collect();
        if let Some(v) = vars.iter().find(|v| !is_name(v)) {
            return Err(no(format!(
                "ktau {v}: the variables written out are translated"
            )));
        }
        if vars.len() < 2 {
            out.line(format!(
                "stop({}, call. = FALSE)",
                r_text("too few variables specified (Stata: r(102))")
            ));
            return Ok(());
        }
        let mut args = String::new();
        if let Some(s) = opts.arg("stats", 2) {
            let mut st = Vec::new();
            for w in s.split_whitespace() {
                if !matches!(w, "taua" | "taub" | "score" | "se" | "obs" | "p") {
                    return Err(no(format!("ktau, stats({w}) is not translated yet")));
                }
                st.push(r_text(w));
            }
            if !st.is_empty() {
                args.push_str(&format!(", stats = c({})", st.join(", ")));
            }
        }
        for (o, m) in [("print", 2), ("star", 2)] {
            if let Some(v) = opts.arg(o, m) {
                let x: f64 = v
                    .trim()
                    .parse()
                    .ok()
                    .filter(|x: &f64| (0.0..=1.0).contains(x))
                    .ok_or_else(|| {
                        no(format!("ktau, {o}({v}): a level from 0 to 1 is translated"))
                    })?;
                args.push_str(&format!(", {o} = {}", r_number(x)));
            }
        }
        if opts.has("bonferroni", 3) && opts.has("sidak", 3) {
            return Err(no("ktau: bonferroni and sidak together are not translated"));
        }
        if opts.has("bonferroni", 3) {
            args.push_str(", adjust = \"bonferroni\"");
        }
        if opts.has("sidak", 3) {
            args.push_str(", adjust = \"sidak\"");
        }
        if opts.has("pw", 2) {
            args.push_str(", pw = TRUE");
        }
        if opts.has("matrix", 3) {
            args.push_str(", matrix = TRUE");
        }
        if self.quiet(c) {
            args.push_str(", show = FALSE");
        }
        let rows = self.rows_for(c, out)?;
        for h in [
            "stata_ranktests",
            "stata_fmt",
            "stata_tables",
            "stata_returned",
        ] {
            self.helpers.insert(h);
        }
        let q: Vec<String> = vars.iter().map(|v| r_text(v)).collect();
        out.line(format!(
            "stata_r <- stata_ktau({rows}, c({}){args})",
            q.join(", ")
        ));
        self.r_new = Some((R_KTAU, "ktau".into()));
        Ok(())
    }
}
