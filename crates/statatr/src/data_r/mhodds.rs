//! `mhodds case exposure [controls] [fw] [if] [in] [, by(varlist)
//! compare(v1,v2) level(#)]`: Mantel and Haenszel's odds ratios as Stata 19
//! prints them and the r() it leaves (`plan/stata/mhodds.md`). A two-valued
//! exposure (or compare()) gives the Mantel-Haenszel odds ratio, its chi2 and
//! an interval on the variance ΣV/(R·S); more values give the score test for
//! trend, exp(U/V) with exp(U/V ± z/√V); by() adds the strata's estimates and
//! the approximate test of homogeneity. Checked against Stata 19 on made-up
//! data.

use super::*;

pub(super) const MHODDS_HELPER: &str = r##"stata_mhodds <- function(d, case, expo, ctl = character(), by = character(), cmp = NULL, fw = NULL,
                         level = 95, show = TRUE) {
  for (v in c(case, expo, ctl, by, fw)) if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
  refuse <- function(why) stop("mhodds: ", why, ", where Stata 19's output is not matched; not translated", call. = FALSE)
  err <- function(msg, rc) stop(msg, " (Stata: r(", rc, "))", call. = FALSE)
  for (v in c(case, expo, ctl, by, fw)) if (is.character(d[[v]])) refuse(paste0("the string variable ", v))
  for (v in by) if (!is.null(attr(d[[v]], "labels"))) refuse(paste0("a by() variable with value labels (", v, ")"))
  num <- function(v) { u <- unclass(d[[v]]); attributes(u) <- NULL; as.numeric(u) }
  y <- num(case); x <- num(expo); C <- lapply(ctl, num); B <- lapply(by, num)
  w <- if (is.null(fw)) rep(1, length(y)) else num(fw)
  ok <- !is.na(y) & !is.na(x) & !is.na(w)
  for (v in c(C, B)) ok <- ok & !is.na(v)
  if (any(w[ok] < 0 | w[ok] != round(w[ok]))) refuse("frequency weights that are negative or not whole")
  ok <- ok & w > 0
  y <- y[ok]; x <- x[ok]; w <- w[ok]; C <- lapply(C, `[`, ok); B <- lapply(B, `[`, ok)
  if (!length(y)) err("no observations", 2000)
  if (!all(y %in% c(0, 1))) err(paste0("response variable ", case, " not coded 0/1"), 198)
  lv <- sort(unique(x))
  binary <- TRUE
  if (!is.null(cmp)) {
    if (!all(cmp %in% lv) || cmp[1] == cmp[2]) err(paste0("option compare(v_1, v_2) must specify two categories of variable ", expo), 198)
    keep <- x %in% cmp
    y <- y[keep]; x <- x[keep]; w <- w[keep]; C <- lapply(C, `[`, keep); B <- lapply(B, `[`, keep)
    v1 <- cmp[1]; v0 <- cmp[2]
  } else if (length(lv) == 2) {
    v1 <- lv[2]; v0 <- lv[1]
  } else binary <- FALSE
  z <- stats::qnorm((1 + level / 100) / 2)
  key <- function(L, n) if (length(L)) do.call(paste, c(L, sep = "\r")) else rep("", n)
  # one stratum: the Mantel-Haenszel terms, the score and its variance under the null
  cell <- function(i) {
    yy <- y[i]; ww <- w[i]
    if (binary) {
      e1 <- x[i] == v1
      a <- sum(ww[e1 & yy == 1]); b <- sum(ww[e1 & yy == 0]); c <- sum(ww[!e1 & yy == 1]); dd <- sum(ww[!e1 & yy == 0])
      n <- a + b + c + dd
      V <- if (min(a + b, c + dd, a + c, b + dd) > 0) (a + b) * (c + dd) * (a + c) * (b + dd) / (n^2 * (n - 1)) else 0
      c(R = a * dd / n, S = b * c / n, U = a - (a + b) * (a + c) / n, V = V)
    } else {
      xx <- x[i]; N <- sum(ww); D <- sum(ww * yy)
      V <- if (D > 0 && D < N && length(unique(xx)) > 1) D * (N - D) / (N^2 * (N - 1)) * (N * sum(ww * xx^2) - sum(ww * xx)^2) else 0
      c(R = 0, S = 0, U = sum(ww * xx * yy) - D * sum(ww * xx) / N, V = max(V, 0))
    }
  }
  strata <- function(i) do.call(rbind, lapply(split(i, key(lapply(C, `[`, i), length(i))), cell))
  est <- function(m) {
    U <- sum(m[, "U"]); V <- sum(m[, "V"])
    chi <- if (V > 0) U^2 / V else NA_real_
    ci <- c(NA_real_, NA_real_)
    if (binary) {
      R <- sum(m[, "R"]); S <- sum(m[, "S"])
      or <- if (S > 0) R / S else NA_real_
      if (S > 0 && R > 0) ci <- exp(log(or) + c(-1, 1) * z * sqrt(V / (R * S)))
    } else if (V > 0) {
      or <- exp(U / V); ci <- exp(U / V + c(-1, 1) * z / sqrt(V))
    } else or <- NA_real_
    c(or = or, chi2 = chi, pvalue = if (is.na(chi)) NA_real_ else stats::pchisq(chi, 1, lower.tail = FALSE), ll = ci[1], ul = ci[2])
  }
  all_rows <- seq_along(y)
  m <- strata(all_rows)
  e <- est(m)
  R <- list(or = e[["or"]], lb_or = e[["ll"]], ub_or = e[["ul"]], chi2 = e[["chi2"]], p = e[["pvalue"]])
  vf <- function(v) stata_fmt(v, "%10.0g", trim = TRUE)
  notes_by <- character()
  if (length(by)) {
    bk <- key(B, length(y))
    first <- !duplicated(bk)
    bv <- lapply(B, function(v) v[first]); o <- do.call(order, bv)
    bv <- lapply(bv, function(v) v[o]); gk <- bk[first][o]
    gm <- lapply(gk, function(k) strata(which(bk == k)))
    if (length(ctl) && any(vapply(gm, function(s) any(s[, "V"] <= 0), logical(1))))
      refuse("by() with a control stratum that carries no information (the note's count was not seen)")
    # the overall estimate: stratified by the controls and the by() groups together
    m <- do.call(rbind, gm); e <- est(m)
    R <- list(or = e[["or"]], lb_or = e[["ll"]], ub_or = e[["ul"]], chi2 = e[["chi2"]], p = e[["pvalue"]])
    G <- do.call(rbind, lapply(gm, function(s) colSums(s)))
    TB <- do.call(rbind, lapply(gm, est))
    rownames(TB) <- vapply(seq_along(gk), function(i) paste(vapply(bv, function(v) vf(v[i]), ""), collapse = " "), "")
    inf <- G[, "V"] > 0
    if (!length(ctl) && !all(inf)) notes_by <- paste0("Only ", sum(inf), " of the ", length(inf), " strata formed in this analysis contribute information about the effect of the explanatory variable.")
    few <- sum(inf) < 2
    if (!few) {
      if (!all(inf)) refuse("by() groups without information beside two or more with it")
      if (binary) {
        psi <- sum(G[, "R"]) / sum(G[, "S"])
        if (!is.finite(psi) || psi <= 0) refuse("a test of homogeneity about an odds ratio of 0 or infinity")
        hom <- sum((G[, "R"] - psi * G[, "S"])^2 / (psi * G[, "V"]))
      } else hom <- sum(G[, "U"]^2 / G[, "V"]) - sum(G[, "U"])^2 / sum(G[, "V"])
      R$df_hom <- sum(inf) - 1; R$chi2_hom <- hom
    }
    R$strata_table <- TB
    if (!binary) notes_by <- c(notes_by, paste0("The Odds ratio estimate is an approximation to the odds ratio for a one-unit increase in ", expo, "."))
    if (length(notes_by) > 1) refuse("by() with both of Stata's notes")
  }
  if (!show) return(R)
  # %w.df as Stata rounds: from the double's exact digits, an exact tie to even (88.125 to 88.12)
  fk <- function(v, dg) {
    if (is.na(v)) return(".")
    s <- sprintf("%.45f", abs(v))
    cut <- regexpr(".", s, fixed = TRUE) + dg
    rest <- substring(s, cut + 1); nx <- substr(rest, 1, 1)
    pd <- substr(s, cut, cut); if (pd == ".") pd <- substr(s, cut - 1, cut - 1)
    up <- nx > "5" || (nx == "5" && (grepl("[1-9]", substring(rest, 2)) || pd %in% c("1", "3", "5", "7", "9")))
    q <- as.numeric(substr(s, 1, cut)) + if (up) 10^-dg else 0
    sprintf(paste0("%.", dg, "f"), if (v < 0) -q else q)
  }
  wrap <- function(text, W, first, cont) {
    words <- strsplit(text, " ", fixed = TRUE)[[1]]
    out <- character(); line <- paste0(first, words[1])
    for (wd in words[-1]) {
      if (nchar(line) + 1 + nchar(wd) <= W) line <- paste0(line, " ", wd)
      else { out <- c(out, line); line <- paste0(cont, wd) }
    }
    c(out, line)
  }
  notes <- function(ps, W) {
    if (length(ps) == 1) cat(wrap(ps, W, "Note: ", "      "), sep = "\n")
    else for (i in seq_along(ps)) cat(wrap(ps[i], W, if (i == 1) "Notes: " else "       ", "       "), sep = "\n")
  }
  jl <- function(v) if (length(v) < 3) paste(v, collapse = " and ") else paste0(paste(v[-length(v)], collapse = ", "), ", and ", v[length(v)])
  ci_lab <- paste0("[", level, "% conf. interval]")
  tab68 <- function(e) cat(strrep("-", 68), "\n", sprintf("%15s%11s%14s%28s", "Odds ratio", "chi2(1)", "P>chi2", ci_lab), "\n",
                           strrep("-", 68), "\n", sprintf(if (is.na(e[["pvalue"]])) "%15s%11s%13s%18s%11s" else "%15s%11s%14s%17s%11s",
                           fk(e[["or"]], 6), fk(e[["chi2"]], 2), fk(e[["pvalue"]], 4),
                           fk(e[["ll"]], 6), fk(e[["ul"]], 6)), "\n", strrep("-", 68), "\n", sep = "")
  W <- if (length(by)) 79 else 68
  head <- if (binary) paste0(if (length(ctl)) "Mantel–Haenszel" else "Maximum likelihood", " estimate of the odds ratio comparing ",
                             expo, "==", vf(v1), " vs. ", expo, "==", vf(v0)) else paste0("Score test for trend of odds with ", expo)
  title <- wrap(head, W, "", "  ")
  if (length(ctl)) title <- c(title, wrap(paste0("controlling for ", jl(ctl)), W, "  ", "  "))
  if (length(by)) title <- c(title, wrap(paste0("by ", jl(by)), W, "  ", "  "))
  cat("\n", paste0(title, "\n"), "\n", sep = "")
  if (!length(by)) {
    tab68(e)
    ps <- character()
    inf <- sum(m[, "V"] > 0)
    if (inf < nrow(m)) ps <- paste0("Only ", inf, " of the ", nrow(m), " strata formed in this analysis contribute information about the effect of the explanatory variable.")
    if (!binary) ps <- c(ps, paste0("The Odds ratio estimate is an approximation to the odds ratio for a one-unit increase in ", expo, "."))
    if (length(ps)) notes(ps, 68)
    return(R)
  }
  ab <- function(s) stata_tab_abbrev(s, 8)
  cat(strrep("-", 79), "\n", sep = "")
  if (length(by) == 1) {
    cat(sprintf("%9s |", ab(by[1])), sprintf("%11s%15s%15s%27s", "Odds ratio", "chi2(1)", "P>chi2", ci_lab), "\n",
        strrep("-", 10), "+", strrep("-", 68), "\n", sep = "")
    for (i in seq_len(nrow(TB))) cat(sprintf("%9s |%11s%15s%15s%16s%11s", vf(bv[[1]][i]), fk(TB[i, "or"], 6), fk(TB[i, "chi2"], 2),
                                            fk(TB[i, "pvalue"], 4), fk(TB[i, "ll"], 5), fk(TB[i, "ul"], 5)), "\n", sep = "")
  } else {
    cat(sprintf("%9s%10s |", ab(by[1]), ab(by[2])), sprintf("%11s%12s%12s%23s", "Odds ratio", "chi2(1)", "P>chi2", ci_lab), "\n",
        strrep("-", 20), "+", strrep("-", 58), "\n", sep = "")
    for (i in seq_len(nrow(TB))) cat(sprintf("%9s%10s |%11s%12s%12s%12s%11s", vf(bv[[1]][i]), vf(bv[[2]][i]), fk(TB[i, "or"], 6),
                                            fk(TB[i, "chi2"], 2), fk(TB[i, "pvalue"], 4), fk(TB[i, "ll"], 5), fk(TB[i, "ul"], 5)), "\n", sep = "")
  }
  cat(strrep("-", 79), "\n", sep = "")
  if (length(notes_by)) notes(notes_by, 79)
  cat("\n", paste0(wrap(paste0("Mantel–Haenszel estimate controlling for ", jl(c(ctl, by))), 68, "", "  "), "\n"), sep = "")
  tab68(e)
  if (is.null(R$chi2_hom)) cat("Note: Too few informative strata to test for unequal odds ratios.\n")
  else cat("Approximate test of homogeneity of odds ratios: ", formatC(paste0("chi2(", R$df_hom, ")"), width = -10), "=",
           sprintf("%9s", fk(R$chi2_hom, 2)), "\n", strrep(" ", 48), "Pr>chi2   =",
           sprintf("%9s", fk(stats::pchisq(R$chi2_hom, R$df_hom, lower.tail = FALSE), 4)), "\n", sep = "")
  R
}"##;

/// The r() names `mhodds` leaves, in Stata's order.
const R_MHODDS: &[&str] = &[
    "or",
    "lb_or",
    "ub_or",
    "chi2",
    "p",
    "df_hom",
    "chi2_hom",
    "strata_table",
];

impl Session {
    /// `mhodds case exposure [controls] [fw] [, by() compare() level()]`.
    pub(super) fn mhodds(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        use crate::lang::syntax::WeightKind;
        self.no_by(c, "mhodds")?;
        let stop = |msg: &str, rc: u32| {
            format!(
                "stop({}, call. = FALSE)",
                r_text(&format!("{msg} (Stata: r({rc}))"))
            )
        };
        let fw = match &c.parts.weight {
            None => None,
            Some(w) => {
                let kind = match w.kind {
                    WeightKind::Frequency => None,
                    WeightKind::Analytic => Some("aweights"),
                    WeightKind::Importance => Some("iweights"),
                    WeightKind::Sampling => Some("pweights"),
                    WeightKind::Unspecified => {
                        return Err(no("mhodds [weight = ...] is not translated"))
                    }
                };
                if let Some(k) = kind {
                    out.line(stop(&format!("{k} not allowed"), 101));
                    return Ok(());
                }
                if !is_name(w.exp.trim()) {
                    return Err(no(
                        "mhodds with a frequency weight other than a variable is not translated",
                    ));
                }
                Some(w.exp.trim().to_string())
            }
        };
        let vars: Vec<&str> = c.parts.main.split_whitespace().collect();
        if let Some(v) = vars.iter().find(|v| !is_name(v)) {
            return Err(no(format!(
                "mhodds {v}: the variables written out are translated"
            )));
        }
        if vars.len() < 2 {
            out.line(stop("too few variables specified", 102));
            return Ok(());
        }
        if let Some(o) = opts.other(&[("by", 2), ("compare", 7), ("level", 1)]) {
            return Err(no(format!(
                "mhodds with the option {o} is not translated yet"
            )));
        }
        let expo = vars.get(1).copied().unwrap_or_default();
        let mut args = String::new();
        let ctl = vars.get(2..).unwrap_or_default();
        if !ctl.is_empty() {
            args.push_str(&format!(", ctl = {}", r_names(ctl)));
        }
        if let Some(b) = opts.arg("by", 2) {
            let bv: Vec<&str> = b.split_whitespace().collect();
            if bv.is_empty() || bv.iter().any(|v| !is_name(v)) {
                return Err(no("mhodds, by(): the variables written out are translated"));
            }
            if bv.len() > 2 {
                return Err(no(
                    "mhodds, by() with more than two variables is not translated",
                ));
            }
            args.push_str(&format!(", by = {}", r_names(&bv)));
        }
        if let Some(cm) = opts.arg("compare", 7) {
            let parts: Vec<&str> = cm.split(',').map(str::trim).collect();
            if parts.len() != 2 || parts.iter().any(|p| p.is_empty()) {
                out.line(stop(
                    &format!(
                        "option compare(v_1, v_2) must specify two categories of variable {expo}"
                    ),
                    198,
                ));
                return Ok(());
            }
            let nums: Option<Vec<f64>> = parts.iter().map(|p| p.parse().ok()).collect();
            let Some(nums) = nums else {
                return Err(no(format!(
                    "mhodds, compare({cm}): two numbers are translated"
                )));
            };
            args.push_str(&format!(
                ", cmp = c({})",
                nums.iter()
                    .map(|v| r_number(*v))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if let Some(l) = opts.arg("level", 1) {
            let v: u32 = l
                .trim()
                .parse()
                .ok()
                .filter(|v| (10..=99).contains(v))
                .ok_or_else(|| {
                    no(format!(
                        "mhodds, level({l}): a whole level from 10 to 99 is translated"
                    ))
                })?;
            args.push_str(&format!(", level = {v}"));
        }
        if let Some(f) = &fw {
            args.push_str(&format!(", fw = {}", r_text(f)));
        }
        if self.quiet(c) {
            args.push_str(", show = FALSE");
        }
        let rows = self.rows_for(c, out)?;
        for h in [
            "stata_mhodds",
            "stata_fmt",
            "stata_tables",
            "stata_returned",
        ] {
            self.helpers.insert(h);
        }
        out.line(format!(
            "stata_r <- stata_mhodds({rows}, {}, {}{args})",
            r_text(vars.first().copied().unwrap_or_default()),
            r_text(expo)
        ));
        let names: &'static [&'static str] = R_MHODDS;
        self.r_new = Some((names, "mhodds".into()));
        Ok(())
    }
}

/// `c("a", "b")` of variable names.
fn r_names(v: &[&str]) -> String {
    format!(
        "c({})",
        v.iter().map(|s| r_text(s)).collect::<Vec<_>>().join(", ")
    )
}
