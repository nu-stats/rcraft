//! `kap rater1 rater2 [rater3 ...] [if] [in] [fweight] [, tab wgt(w|w2) absolute]`:
//! interrater agreement as Stata 19 prints it and the r() it leaves
//! (`plan/stata/kap.md`). Two raters: Cohen's kappa, weighted by
//! 1 - |i - j|/(k - 1) (w) or 1 - ((i - j)/(k - 1))^2 (w2) over the ratings
//! both raters used (their values with absolute), and its standard error
//! under no agreement beyond chance (Fleiss, Cohen and Everitt's); three or
//! more: Fleiss's kappa for each outcome and combined, with their z tests.
//! Every formula was checked against Stata 19 on made-up data.

use super::*;

pub(super) const KAP_HELPER: &str = r##"stata_kap <- function(d, vars, wt = NULL, absolute = FALSE, tab = FALSE, fw = NULL, show = TRUE) {
  for (v in c(vars, fw)) {
    if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
    if (is.character(d[[v]])) stop("kap with a string variable is not translated", call. = FALSE)
  }
  num <- function(v) { u <- unclass(d[[v]]); attributes(u) <- NULL; as.numeric(u) }
  M <- sapply(vars, num)
  if (!is.matrix(M)) M <- matrix(M, ncol = length(vars))
  bad <- function(x) any(!is.finite(x))
  # %w.df as Stata rounds: from the double's exact digits, a 5 rounding up (88.125 to 88.13)
  fk <- function(v, w, d) {
    s <- sprintf("%.45f", abs(v))
    cut <- regexpr(".", s, fixed = TRUE) + d
    up <- substr(substring(s, cut + 1), 1, 1) >= "5"
    q <- as.numeric(substr(s, 1, cut)) + if (up) 10^-d else 0
    sprintf(paste0("%", w, ".", d, "f"), if (v < 0) -q else q)
  }
  refuse <- function(why) stop("kap: ", why, ", where Stata 19's output is not matched; not translated", call. = FALSE)
  if (length(vars) == 2) {
    f <- if (is.null(fw)) rep(1, nrow(M)) else num(fw)
    ok <- stats::complete.cases(M) & !is.na(f)
    M <- M[ok, , drop = FALSE]; f <- f[ok]
    if (any(f < 0 | f != round(f))) stop("may not use noninteger frequency weights (Stata: r(401))", call. = FALSE)
    keep <- f > 0
    M <- M[keep, , drop = FALSE]; f <- f[keep]
    if (nrow(M) == 0) stop("no observations (Stata: r(2000))", call. = FALSE)
    cats <- sort(unique(as.vector(M)))
    k <- length(cats); N <- sum(f)
    P <- matrix(0, k, k)
    r <- match(M[, 1], cats); s <- match(M[, 2], cats)
    for (i in seq_along(r)) P[r[i], s[i]] <- P[r[i], s[i]] + f[i]
    counts <- P
    P <- P / N
    pos <- if (absolute) cats else seq_len(k)
    span <- max(pos) - min(pos)
    W <- if (is.null(wt)) diag(k) else if (k < 2) matrix(1, 1, 1)
         else if (wt == "w") 1 - abs(outer(pos, pos, "-")) / span else 1 - (outer(pos, pos, "-") / span)^2
    pr <- rowSums(P); pc <- colSums(P)
    po <- sum(W * P); pe <- sum(W * outer(pr, pc))
    kappa <- (po - pe) / (1 - pe)
    # unweighted, Stata 19 works from the counts (its last digit)
    if (is.null(wt)) {
      sc <- sum(rowSums(counts) * colSums(counts)) / N
      po <- sum(diag(counts)) / N
      kappa <- (sum(diag(counts)) - sc) / (N - sc)
    }
    wi <- as.vector(W %*% pc); wj <- as.vector(t(W) %*% pr)
    rad <- sum(outer(pr, pc) * (W - outer(wi, wj, "+"))^2) - pe^2
    se <- sqrt(max(rad, 0)) / ((1 - pe) * sqrt(N))
    z <- kappa / se
    if (bad(c(kappa, se, z)) || rad < 1e-10 || pe >= 1) refuse("the agreement expected by chance leaves no test")
    if (tab && k > 5) refuse("tab with more than five ratings (Stata's table is not split as tabulate's)")
    R <- list(se = se, z = z, kappa = kappa, prop_e = pe, prop_o = po, N = N)
    if (!show) return(R)
    if (tab) {
      lab <- vapply(cats, function(v) stata_fmt(v, "%9.0g", trim = TRUE), "")
      stata_tab2_print(stata_tab_abbrev(vars[1], 10), stata_tab_abbrev(vars[2], 33), lab, lab, counts)
    }
    if (!is.null(wt)) {
      if (!tab) cat("\n")
      cat("Ratings weighted by:\n")
      for (i in seq_len(k)) cat(paste0(vapply(W[i, ], fk, "", w = 9, d = 4), collapse = ""), "\n", sep = "")
    }
    if (!tab || !is.null(wt)) cat("\n")
    cat("             Expected\n")
    cat("Agreement   agreement     Kappa   Std. err.         Z      Prob>Z\n")
    cat(strrep("-", 65), "\n", sep = "")
    cat(fk(100 * po, 7, 2), "%", fk(100 * pe, 11, 2), "%", fk(kappa, 11, 4), fk(se, 11, 4), fk(z, 11, 2),
        fk(stats::pnorm(z, lower.tail = FALSE), 12, 4), "\n", sep = "")
    return(R)
  }
  # three or more raters: every subject rated by all of them
  if (anyNA(M)) refuse("subjects rated by different numbers of raters")
  cats <- sort(unique(as.vector(M)))
  n <- ncol(M); N <- nrow(M)
  if (N == 0) stop("no observations (Stata: r(2000))", call. = FALSE)
  x <- sapply(cats, function(v) rowSums(M == v))
  if (!is.matrix(x)) x <- matrix(x, nrow = N)
  p <- colSums(x) / (N * n); q <- 1 - p
  kj <- 1 - colSums(x * (n - x)) / (N * n * (n - 1) * p * q)
  kappa <- 1 - (N * n^2 - sum(x^2)) / (N * n * (n - 1) * sum(p * q))
  sej <- sqrt(2 / (N * n * (n - 1)))
  se <- sqrt(2) / (sum(p * q) * sqrt(N * n * (n - 1))) * sqrt(sum(p * q)^2 - sum(p * q * (q - p)))
  z <- kappa / se
  if (bad(c(kj, kappa, se, z))) refuse("an outcome every rater gave or none did")
  R <- list(z = z, kappa = kappa)
  if (!show) return(R)
  cat("\nThere are ", n, " raters per subject:\n\n", sep = "")
  cat("         Outcome |    Kappa          Z     Prob>Z\n")
  cat("-----------------+-------------------------------\n")
  for (j in seq_along(cats)) {
    zj <- kj[j] / sej
    cat(sprintf("%16s |", stata_fmt(cats[j], "%9.0g", trim = TRUE)), fk(kj[j], 10, 4), fk(zj, 11, 2),
        fk(stats::pnorm(zj, lower.tail = FALSE), 10, 4), "\n", sep = "")
  }
  cat("-----------------+-------------------------------\n")
  cat(sprintf("%16s |", "combined"), fk(kappa, 10, 4), fk(z, 11, 2), fk(stats::pnorm(z, lower.tail = FALSE), 10, 4), "\n", sep = "")
  R
}"##;

/// The r() names `kap` leaves with two raters, in Stata's order.
const R_KAP2: &[&str] = &["se", "z", "kappa", "prop_e", "prop_o", "N"];
/// With three or more.
const R_KAPM: &[&str] = &["z", "kappa"];

impl Session {
    /// `kap varlist [if] [in] [fw] [, tab wgt(w|w2) absolute]`.
    pub(super) fn kap(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "kap")?;
        let stop = |msg: &str, rc: u32| {
            format!(
                "stop({}, call. = FALSE)",
                r_text(&format!("{msg} (Stata: r({rc}))"))
            )
        };
        let vars: Vec<&str> = c.parts.main.split_whitespace().collect();
        if let Some(v) = vars.iter().find(|v| !is_name(v)) {
            return Err(no(format!(
                "kap {v}: the variables written out are translated"
            )));
        }
        if vars.len() < 2 {
            out.line(stop("too few variables specified", 102));
            return Ok(());
        }
        if let Some(o) = opts.other(&[("tab", 3), ("wgt", 3), ("absolute", 8)]) {
            return Err(no(format!("kap with the option {o} is not translated yet")));
        }
        let fw = match &c.parts.weight {
            None => None,
            Some(w)
                if matches!(w.kind, crate::lang::syntax::WeightKind::Frequency)
                    && is_name(w.exp.trim()) =>
            {
                Some(w.exp.trim().to_string())
            }
            Some(_) => {
                return Err(no(
                    "kap with weights other than a frequency-weight variable is not translated",
                ))
            }
        };
        if vars.len() > 2 {
            if fw.is_some() {
                return Err(no(
                    "kap with weights and three or more raters is not translated",
                ));
            }
            if opts.has("tab", 3) {
                out.line(stop("option tab not allowed with 3 or more raters", 198));
                return Ok(());
            }
            if !opts.0.is_empty() {
                // what Stata 19 says to wgt() here
                if opts.has("wgt", 3) && opts.0.len() == 1 {
                    out.line(stop("option opts not allowed", 198));
                    return Ok(());
                }
                return Err(no(
                    "kap with options and three or more raters is not translated",
                ));
            }
        }
        let wt = match opts.arg("wgt", 3) {
            None => None,
            Some(w) => match w.trim() {
                "w" => Some("w"),
                "w2" => Some("w2"),
                other => {
                    return Err(no(format!(
                        "kap, wgt({other}): only w and w2 are translated"
                    )))
                }
            },
        };
        let mut args = String::new();
        if let Some(w) = wt {
            args.push_str(&format!(", wt = {}", r_text(w)));
        }
        if opts.has("absolute", 8) {
            args.push_str(", absolute = TRUE");
        }
        if opts.has("tab", 3) {
            args.push_str(", tab = TRUE");
        }
        if let Some(f) = &fw {
            args.push_str(&format!(", fw = {}", r_text(f)));
        }
        if self.quiet(c) {
            args.push_str(", show = FALSE");
        }
        let rows = self.rows_for(c, out)?;
        for h in ["stata_kap", "stata_fmt", "stata_tables", "stata_returned"] {
            self.helpers.insert(h);
        }
        let q: Vec<String> = vars.iter().map(|v| r_text(v)).collect();
        out.line(format!(
            "stata_r <- stata_kap({rows}, c({}){args})",
            q.join(", ")
        ));
        self.r_new = Some((if vars.len() == 2 { R_KAP2 } else { R_KAPM }, "kap".into()));
        Ok(())
    }
}
