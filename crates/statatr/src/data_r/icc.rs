//! `icc depvar target [rater] [if] [in] [, mixed consistency absolute
//! testvalue(#) level(#) format(%fmt)]`: intraclass correlations as Stata 19
//! prints them and the r() it leaves (`plan/stata/icc.md`): one-way and two-way
//! random-effects models and the two-way mixed-effects model, absolute
//! agreement or consistency, individual and average ICCs with McGraw and
//! Wong's confidence intervals and F tests. Checked against Stata 19 on
//! made-up data.

use super::*;

pub(super) const ICC_HELPER: &str = r##"stata_icc <- function(d, depvar, target, rater = NULL, mixed = FALSE, type = NULL, r0 = 0, level = 95,
                      fmt = "%9.0g", show = TRUE) {
  for (v in c(depvar, target, rater)) if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
  if (is.character(d[[depvar]])) stop("icc with a string outcome is not translated", call. = FALSE)
  num <- function(v) { u <- unclass(d[[v]]); attributes(u) <- NULL; u }
  y <- as.numeric(num(depvar)); tg <- num(target); rt <- if (is.null(rater)) NULL else num(rater)
  ok <- !is.na(y) & !is.na(tg) & (if (is.null(rt)) TRUE else !is.na(rt))
  y <- y[ok]; tg <- tg[ok]; if (!is.null(rt)) rt <- rt[ok]
  refuse <- function(why) stop("icc: ", why, ", where Stata 19's output is not matched; not translated", call. = FALSE)
  err <- function(msg, rc) stop(msg, " (Stata: r(", rc, "))", call. = FALSE)
  if (!length(y)) err("no observations", 2000)
  if (is.null(type)) type <- if (mixed) "consistency" else "absolute"
  deftype <- identical(type, if (mixed) "consistency" else "absolute")
  if (is.null(rt)) {
    cnt <- table(tg)
    k <- max(cnt)
    drop <- names(cnt)[cnt < k]
    keep <- !(as.character(tg) %in% drop)
    y <- y[keep]; tg <- tg[keep]
    why <- "because of unbalanced data"
  } else {
    if (any(table(tg, rt) > 1)) err("multiple observations per target and rater not allowed", 498)
    raters <- unique(rt); k <- length(raters)
    full <- tapply(rt, tg, function(r) length(unique(r)) == k)
    drop <- names(full)[!full]
    keep <- !(as.character(tg) %in% drop)
    y <- y[keep]; tg <- tg[keep]; rt <- rt[keep]
    why <- "because not rated by all raters"
  }
  omitted <- length(drop)
  if (show && omitted) cat("(", omitted, if (omitted == 1) " target" else " targets", " omitted from computation ", why, ")\n", sep = "")
  tt <- factor(tg); n <- nlevels(tt)
  if (!n) err("no observations", 2000)
  if (n < 2) err("insufficient number of targets", 2001)
  if (k < 2) err("insufficient number of raters (observations per target)", 2001)
  gm <- mean(y); mt <- tapply(y, tt, mean)
  MSR <- k * sum((mt - gm)^2) / (n - 1)
  q <- (1 + level / 100) / 2
  if (is.null(rt)) {
    MSW <- sum((y - mt[tt])^2) / (n * (k - 1)); dfe <- n * (k - 1)
    F0 <- MSR / MSW
    i1 <- (MSR - MSW) / (MSR + (k - 1) * MSW); ik <- (MSR - MSW) / MSR
    FL <- F0 / stats::qf(q, n - 1, dfe); FU <- F0 * stats::qf(q, dfe, n - 1)
    ci1 <- c((FL - 1) / (FL + k - 1), (FU - 1) / (FU + k - 1)); cik <- c(1 - 1 / FL, 1 - 1 / FU)
    Fs <- c(F0 * (1 - r0) / (1 + (k - 1) * r0), F0 * (1 - r0)); df2 <- c(dfe, dfe)
    model <- "one-way random effects"; mtitle <- "One-way random-effects model"
  } else {
    rr <- factor(rt); mr <- tapply(y, rr, mean)
    MSC <- n * sum((mr - gm)^2) / (k - 1)
    dfe <- (n - 1) * (k - 1)
    MSE <- sum((y - mt[tt] - mr[rr] + gm)^2) / dfe
    if (type == "consistency") {
      F0 <- MSR / MSE
      i1 <- (MSR - MSE) / (MSR + (k - 1) * MSE); ik <- (MSR - MSE) / MSR
      FL <- F0 / stats::qf(q, n - 1, dfe); FU <- F0 * stats::qf(q, dfe, n - 1)
      ci1 <- c((FL - 1) / (FL + k - 1), (FU - 1) / (FU + k - 1)); cik <- c(1 - 1 / FL, 1 - 1 / FU)
      Fs <- c(F0 * (1 - r0) / (1 + (k - 1) * r0), F0 * (1 - r0)); df2 <- c(dfe, dfe)
    } else {
      i1 <- (MSR - MSE) / (MSR + (k - 1) * MSE + k * (MSC - MSE) / n); ik <- (MSR - MSE) / (MSR + (MSC - MSE) / n)
      sat <- function(a, b) (a * MSC + b * MSE)^2 / ((a * MSC)^2 / (k - 1) + (b * MSE)^2 / dfe)
      v <- sat(k * i1 / (n * (1 - i1)), 1 + k * i1 * (n - 1) / (n * (1 - i1)))
      Fa <- stats::qf(q, n - 1, v); Fb <- stats::qf(q, v, n - 1)
      L1 <- n * (MSR - Fa * MSE) / (Fa * (k * MSC + (k * n - k - n) * MSE) + n * MSR)
      U1 <- n * (Fb * MSR - MSE) / (k * MSC + (k * n - k - n) * MSE + n * Fb * MSR)
      ci1 <- c(L1, U1); cik <- c(L1 * k / (1 + L1 * (k - 1)), U1 * k / (1 + U1 * (k - 1)))
      if (r0 == 0) {
        Fs <- c(MSR / MSE, MSR / MSE); df2 <- c(dfe, dfe)
      } else {
        a0 <- k * r0 / (n * (1 - r0)); b0 <- 1 + k * r0 * (n - 1) / (n * (1 - r0))
        c0 <- r0 / (n * (1 - r0)); d0 <- 1 + r0 * (n - 1) / (n * (1 - r0))
        Fs <- c(MSR / (a0 * MSC + b0 * MSE), MSR / (c0 * MSC + d0 * MSE)); df2 <- c(sat(a0, b0), sat(c0, d0))
      }
    }
    model <- if (mixed) "two-way mixed effects" else "two-way random effects"
    mtitle <- if (mixed) "Two-way mixed-effects model" else "Two-way random-effects model"
  }
  if (any(!is.finite(c(i1, ik, ci1, cik, Fs, df2)))) refuse("ICCs or bounds that cannot be computed")
  ps <- stats::pf(Fs, n - 1, df2, lower.tail = FALSE)
  R <- list(N_target = n, N_rater = k,
            icc_i = i1, icc_i_F = Fs[1], icc_i_df1 = n - 1, icc_i_df2 = df2[1], icc_i_p = ps[1], icc_i_lb = ci1[1], icc_i_ub = ci1[2],
            icc_avg = ik, icc_avg_F = Fs[2], icc_avg_df1 = n - 1, icc_avg_df2 = df2[2], icc_avg_p = ps[2], icc_avg_lb = cik[1],
            icc_avg_ub = cik[2], testvalue = r0, level = level, model = model, depvar = depvar, target = target)
  if (!is.null(rater)) R$rater <- rater
  R$type <- type
  if (!show) return(R)
  # %w.df as Stata rounds: from the double's exact digits, an exact tie to even (88.125 to 88.12)
  fk <- function(v, d) {
    s <- sprintf("%.45f", abs(v))
    cut <- regexpr(".", s, fixed = TRUE) + d
    rest <- substring(s, cut + 1); nx <- substr(rest, 1, 1)
    pd <- substr(s, cut, cut); if (pd == ".") pd <- substr(s, cut - 1, cut - 1)
    up <- nx > "5" || (nx == "5" && (grepl("[1-9]", substring(rest, 2)) || pd %in% c("1", "3", "5", "7", "9")))
    qq <- as.numeric(substr(s, 1, cut)) + if (up) 10^-d else 0
    sprintf(paste0("%.", d, "f"), if (v < 0) -qq else qq)
  }
  g <- function(v) stata_fmt(v, fmt, trim = TRUE)
  cat("\nIntraclass correlations\n", mtitle, "\n", if (type == "absolute") "Absolute agreement" else "Consistency of agreement", "\n\n", sep = "")
  cat("Random effects: ", formatC(target, width = -17), "Number of targets =", sprintf("%10s", n), "\n", sep = "")
  if (is.null(rater)) cat(strrep(" ", 33), "Number of raters  =", sprintf("%10s", k), "\n", sep = "")
  else cat(if (mixed) " Fixed effects: " else "Random effects: ", formatC(rater, width = -17), "Number of raters  =", sprintf("%10s", k), "\n", sep = "")
  cat("\n", strrep("-", 62), "\n", sep = "")
  cat(sprintf("%22s |", stata_tab_abbrev(depvar, 22)), "        ICC       [", format(level), "% conf. interval]\n", sep = "")
  cat(strrep("-", 23), "+", strrep("-", 38), "\n", sep = "")
  cat(sprintf("%22s |%11s%15s%12s", "Individual", g(i1), g(ci1[1]), g(ci1[2])), "\n", sep = "")
  cat(sprintf("%22s |%11s%15s%12s", "Average", g(ik), g(cik[1]), g(cik[2])), "\n", sep = "")
  cat(strrep("-", 62), "\n", sep = "")
  cat("F test that\n")
  line <- function(lab, Fv, d2, p) cat(formatC(paste0("  ", lab, "=", fk(r0, 2), ": F(", fk(n - 1, 1), ", ", fk(d2, 1), ") = ", fk(Fv, 2)), width = -46),
                                      "Prob > F = ", fk(p, 3), "\n", sep = "")
  if (r0 == 0) line("ICC", Fs[1], df2[1], ps[1]) else {
    line("ICC(1)", Fs[1], df2[1], ps[1])
    line("ICC(k)", Fs[2], df2[2], ps[2])
  }
  if (deftype) cat("\nNote: ICCs estimate correlations between individual measurements\n      and between average measurements made on the same target.\n")
  R
}"##;

/// The r() names `icc` leaves, in Stata's order.
const R_ICC: &[&str] = &[
    "N_target",
    "N_rater",
    "icc_i",
    "icc_i_F",
    "icc_i_df1",
    "icc_i_df2",
    "icc_i_p",
    "icc_i_lb",
    "icc_i_ub",
    "icc_avg",
    "icc_avg_F",
    "icc_avg_df1",
    "icc_avg_df2",
    "icc_avg_p",
    "icc_avg_lb",
    "icc_avg_ub",
    "testvalue",
    "level",
    "model",
    "depvar",
    "target",
    "rater",
    "type",
];

impl Session {
    /// `icc depvar target [rater] [, options]`.
    pub(super) fn icc(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "icc")?;
        let stop = |msg: &str, rc: u32| {
            format!(
                "stop({}, call. = FALSE)",
                r_text(&format!("{msg} (Stata: r({rc}))"))
            )
        };
        if c.parts.weight.is_some() {
            out.line(stop("weights not allowed", 101));
            return Ok(());
        }
        let vars: Vec<&str> = c.parts.main.split_whitespace().collect();
        if let Some(v) = vars.iter().find(|v| !is_name(v)) {
            return Err(no(format!(
                "icc {v}: the variables written out are translated"
            )));
        }
        if vars.len() > 3 {
            return Err(no("icc with more than three variables is not translated"));
        }
        if let Some(o) = opts.other(&[
            ("mixed", 5),
            ("consistency", 11),
            ("absolute", 8),
            ("testvalue", 9),
            ("level", 5),
            ("format", 6),
        ]) {
            return Err(no(format!("icc with the option {o} is not translated yet")));
        }
        if vars.len() < 2 {
            out.line(stop("No target variable specified", 198));
            return Ok(());
        }
        let mixed = opts.has("mixed", 5);
        let cons = opts.has("consistency", 11);
        let abs = opts.has("absolute", 8);
        if cons && abs {
            out.line(stop(
                "only one of consistency or absolute may be specified",
                198,
            ));
            return Ok(());
        }
        if vars.len() == 2 && mixed {
            out.line(stop(
                "you must specify the rater variable with mixed-effects models",
                198,
            ));
            return Ok(());
        }
        if vars.len() == 2 && cons {
            out.line(stop(
                "consistency is not allowed with one-way random-effects models",
                198,
            ));
            return Ok(());
        }
        let mut args = String::new();
        if mixed {
            args.push_str(", mixed = TRUE");
        }
        if cons {
            args.push_str(", type = \"consistency\"");
        }
        if abs {
            args.push_str(", type = \"absolute\"");
        }
        if let Some(t) = opts.arg("testvalue", 9) {
            let v: f64 = t
                .trim()
                .parse()
                .ok()
                .filter(|v: &f64| (0.0..1.0).contains(v))
                .ok_or_else(|| {
                    no(format!(
                        "icc, testvalue({t}): a value from 0 to below 1 is translated"
                    ))
                })?;
            args.push_str(&format!(", r0 = {}", r_number(v)));
        }
        if let Some(l) = opts.arg("level", 5) {
            let v: f64 = l
                .trim()
                .parse()
                .ok()
                .filter(|v: &f64| (10.0..=99.99).contains(v))
                .ok_or_else(|| {
                    no(format!(
                        "icc, level({l}): a level from 10 to 99.99 is translated"
                    ))
                })?;
            args.push_str(&format!(", level = {}", r_number(v)));
        }
        if let Some(f) = opts.arg("format", 6) {
            let f = f.trim();
            let ok = f.starts_with('%')
                && (f.ends_with('f') || f.ends_with('g'))
                && f.get(1..f.len() - 1).is_some_and(|m| {
                    m.split('.')
                        .all(|p| !p.is_empty() && p.chars().all(|ch| ch.is_ascii_digit()))
                });
            if !ok {
                return Err(no(format!(
                    "icc, format({f}): only %w.df and %w.dg are translated"
                )));
            }
            args.push_str(&format!(", fmt = {}", r_text(f)));
        }
        if self.quiet(c) {
            args.push_str(", show = FALSE");
        }
        let rows = self.rows_for(c, out)?;
        for h in ["stata_icc", "stata_fmt", "stata_tables", "stata_returned"] {
            self.helpers.insert(h);
        }
        out.line(format!(
            "stata_r <- stata_icc({rows}, {}, {}{}{args})",
            r_text(vars.first().copied().unwrap_or_default()),
            r_text(vars.get(1).copied().unwrap_or_default()),
            vars.get(2)
                .map(|r| format!(", rater = {}", r_text(r)))
                .unwrap_or_default()
        ));
        let names: &'static [&'static str] = R_ICC;
        self.r_new = Some((names, "icc".into()));
        Ok(())
    }
}
