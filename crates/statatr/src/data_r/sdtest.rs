//! `sdtest var == #`, `sdtest var1 == var2`, `sdtest var, by(g)` and
//! `sdtesti #obs #mean #sd #val` / `sdtesti #obs1 #mean1 #sd1 #obs2 #mean2
//! #sd2`: the chi2 test of one standard deviation and the F test of two, as
//! Stata 19 prints them and the r() they leave (`plan/stata/sdtest.md`).

use super::*;

pub(super) const SDTEST_HELPER: &str = r##"stata_sd_summ <- function(v) {
  v <- as.numeric(v)
  v <- v[!is.na(v)]
  c(n = length(v), m = if (length(v)) mean(v) else NA_real_, s = if (length(v) > 1) stats::sd(v) else NA_real_)
}
stata_sd_by <- function(x, g, gname) {
  x <- as.numeric(x)
  lab <- attr(g, "labels", exact = TRUE)
  gv <- if (is.character(g)) g else as.numeric(unclass(g))
  keep <- !is.na(x) & !is.na(gv) & !(is.character(gv) & gv == "")
  lv <- sort(unique(gv[keep]), method = "radix")
  if (length(lv) > 2) stop("more than 2 groups found, only 2 allowed (Stata: r(420))", call. = FALSE)
  if (length(lv) < 2) stop("1 group found, 2 required (Stata: r(420))", call. = FALSE)
  name <- function(v) {
    if (is.character(v)) return(v)
    k <- if (!is.null(lab)) names(lab)[match(v, as.numeric(lab))] else NA
    if (length(k) && !is.na(k)) k else stata_fmt(v, "%9.0g", trim = TRUE)
  }
  list(groups = lapply(lv, function(v) stata_sd_summ(x[keep & gv == v])), names = vapply(lv, name, ""),
       combined = stata_sd_summ(x[keep]))
}
stata_sd_pool <- function(a, b) {
  n <- a[["n"]] + b[["n"]]
  m <- (a[["n"]] * a[["m"]] + b[["n"]] * b[["m"]]) / n
  v <- ((a[["n"]] - 1) * a[["s"]]^2 + (b[["n"]] - 1) * b[["s"]]^2 + a[["n"]] * (a[["m"]] - m)^2 + b[["n"]] * (b[["m"]] - m)^2) / (n - 1)
  c(n = n, m = m, s = sqrt(v))
}
stata_sdtest <- function(groups, names, head, value = NULL, combined = NULL, level = 95, show = TRUE) {
  two <- is.null(value)
  if (two) {
    a <- groups[[1]]
    b <- groups[[2]]
    F <- a[["s"]]^2 / b[["s"]]^2
    d1 <- a[["n"]] - 1
    d2 <- b[["n"]] - 1
    pl <- stats::pf(F, d1, d2)
    pu <- stats::pf(F, d1, d2, lower.tail = FALSE)
    r <- list(sd = combined[["s"]], p = 2 * min(pl, pu), p_l = pl, p_u = pu, sd_2 = b[["s"]], sd_1 = a[["s"]],
              F = F, df_2 = d2, df_1 = d1, N = a[["n"]] + b[["n"]])
  } else {
    a <- groups[[1]]
    df <- a[["n"]] - 1
    c2 <- df * a[["s"]]^2 / value^2
    pl <- stats::pchisq(c2, df)
    pu <- stats::pchisq(c2, df, lower.tail = FALSE)
    r <- list(p = 2 * min(pl, pu), p_l = pl, p_u = pu, sd = a[["s"]], chi2 = c2, df = df, N = a[["n"]])
  }
  if (!show) return(r)
  g9 <- function(v) if (is.na(v)) "." else stata_fmt(v, "%9.0g", trim = TRUE)
  row <- function(lab, s) {
    se <- s[["s"]] / sqrt(s[["n"]])
    t <- if (s[["n"]] > 1) stats::qt(1 - (100 - level) / 200, s[["n"]] - 1) else NA_real_
    cat(sprintf("%8s |%8s%12s%12s%12s%12s%12s", stata_tab_abbrev(lab, 8), format(s[["n"]], scientific = FALSE),
                g9(s[["m"]]), g9(se), g9(s[["s"]]), g9(s[["m"]] - t * se), g9(s[["m"]] + t * se)), "\n", sep = "")
  }
  rule <- strrep("-", 78)
  mid <- paste0(strrep("-", 9), "+", strrep("-", 68))
  cat("\n", if (two) "Variance ratio test" else "One-sample test of variance", "\n", rule, "\n", sep = "")
  cat(sprintf("%8s |     Obs        Mean    Std. err.   Std. dev.   [%s%% conf. interval]", head,
              stata_fmt(level, "%9.0g", trim = TRUE)), "\n", mid, "\n", sep = "")
  for (i in seq_along(groups)) row(names[i], groups[[i]])
  if (two) {
    cat(mid, "\n", sep = "")
    row("Combined", combined)
  }
  cat(rule, "\n", sep = "")
  side <- function(l, rgt) paste0(formatC(l, width = -(78 - nchar(rgt))), rgt)
  # each alternative centred on columns 11, 40 and 70 (Stata 19)
  ha <- function(a, b, c) {
    put <- function(line, s, centre) {
      start <- centre - nchar(s) %/% 2
      paste0(line, strrep(" ", max(1, start - 1 - nchar(line))), s)
    }
    put(put(put("", a, 11), b, 40), c, 70)
  }
  if (two) {
    cat(side(sprintf("    ratio = sd(%s) / sd(%s)", names[1], names[2]), sprintf("f = %8.4f", r$F)), "\n", sep = "")
    cat(side("H0: ratio = 1", sprintf("Degrees of freedom = %4d, %d", as.integer(r$df_1), as.integer(r$df_2))), "\n\n", sep = "")
    cat(ha("Ha: ratio < 1", "Ha: ratio != 1", "Ha: ratio > 1"), "\n", sep = "")
    cat(sprintf("  Pr(F < f) = %6.4f         2*Pr(F %s f) = %6.4f           Pr(F > f) = %6.4f",
                r$p_l, if (pl < pu) "<" else ">", r$p, r$p_u), "\n", sep = "")
  } else {
    v <- stata_fmt(value, "%9.0g", trim = TRUE)
    cat(side(sprintf("    sd = sd(%s)", names[1]), sprintf("c = chi2 = %8.4f", r$chi2)), "\n", sep = "")
    cat(side(paste0("H0: sd = ", v), sprintf("Degrees of freedom = %8d", as.integer(r$df))), "\n\n", sep = "")
    cat(ha(paste("Ha: sd <", v), paste("Ha: sd !=", v), paste("Ha: sd >", v)), "\n", sep = "")
    cat(sprintf("  Pr(C < c) = %6.4f         2*Pr(C %s c) = %6.4f           Pr(C > c) = %6.4f",
                r$p_l, if (pl < pu) "<" else ">", r$p, r$p_u), "\n", sep = "")
  }
  invisible(r)
}"##;

const R_SDTEST1: &[&str] = &["p", "p_l", "p_u", "sd", "chi2", "df", "N"];
const R_SDTEST2: &[&str] = &[
    "sd", "p", "p_l", "p_u", "sd_2", "sd_1", "F", "df_2", "df_1", "N",
];

fn level(opts: &Opts) -> Result<String, Untranslated> {
    match opts.arg("level", 1) {
        None => Ok("95".to_string()),
        Some(l) => {
            let v: f64 = l
                .trim()
                .parse()
                .map_err(|_| no(format!("level({l}) needs a number")))?;
            if !(10.0..=99.99).contains(&v) {
                return Err(no(format!(
                    "level({l}): levels from 10 to 99.99 are translated"
                )));
            }
            Ok(r_number(v))
        }
    }
}

impl Session {
    /// `sdtest` on the data.
    pub(super) fn sdtest(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "sdtest")?;
        if let Some(o) = opts.other(&[("by", 2), ("level", 1)]) {
            return Err(no(format!(
                "sdtest with the option {o} is not translated yet"
            )));
        }
        let level = level(opts)?;
        let rows = self.rows_for(c, out)?;
        let main = c.parts.main.trim();
        let col = |v: &str| format!("{rows}[[{}]]", r_text(v));
        let (setup, call, names): (String, String, &'static [&'static str]) = if let Some(g) =
            opts.arg("by", 2)
        {
            let g = g.trim().to_string();
            if !is_name(main) || !is_name(&g) {
                return Err(no("sdtest var, by(g): one variable each is translated"));
            }
            (
                    format!(
                        "stata_sd <- stata_sd_by({}, {}, {})",
                        col(main),
                        col(&g),
                        r_text(&g)
                    ),
                    "stata_sdtest(stata_sd$groups, stata_sd$names, \"   Group\", combined = stata_sd$combined"
                        .to_string(),
                    R_SDTEST2,
                )
        } else {
            let Some((l, r)) = main.split_once("==").or_else(|| main.split_once('=')) else {
                return Err(no(
                    "sdtest var == #, var1 == var2 or var, by(g) is translated",
                ));
            };
            let (l, r) = (l.trim(), r.trim());
            if !is_name(l) {
                return Err(no(format!("sdtest {l}: one variable is translated")));
            }
            if let Ok(v) = r.parse::<f64>() {
                if !(v > 0.0 && v.is_finite()) {
                    return Err(no("sd must be positive (Stata: r(198))"));
                }
                (
                    format!("stata_sd <- list(stata_sd_summ({}))", col(l)),
                    format!(
                        "stata_sdtest(stata_sd, {}, \"Variable\", value = {}",
                        r_text(l),
                        r_number(v)
                    ),
                    R_SDTEST1,
                )
            } else if is_name(r) {
                (
                        format!(
                            "stata_sd <- list(stata_sd_summ({}), stata_sd_summ({}))",
                            col(l),
                            col(r)
                        ),
                        format!(
                            "stata_sdtest(stata_sd, c({}, {}), \"Variable\", combined = stata_sd_summ(c({}, {}))",
                            r_text(l),
                            r_text(r),
                            col(l),
                            col(r)
                        ),
                        R_SDTEST2,
                    )
            } else {
                return Err(no(format!(
                    "sdtest {l} == {r}: a number or a variable is translated"
                )));
            }
        };
        for h in [
            "stata_sdtest",
            "stata_fmt",
            "stata_returned",
            "stata_tables",
        ] {
            self.helpers.insert(h);
        }
        out.line(setup);
        out.line(format!(
            "stata_r <- {call}, level = {level}{})",
            if self.quiet(c) { ", show = FALSE" } else { "" }
        ));
        out.line("rm(stata_sd)");
        self.r_new = Some((names, "sdtest".into()));
        Ok(())
    }

    /// `sdtesti` on numbers typed in.
    pub(super) fn sdtesti(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "sdtesti")?;
        if let Some(o) = opts.other(&[("level", 1)]) {
            return Err(no(format!(
                "sdtesti with the option {o} is not translated yet"
            )));
        }
        let level = level(opts)?;
        let nums: Vec<Option<f64>> = c
            .parts
            .main
            .split_whitespace()
            .map(|w| {
                if w == "." {
                    Ok(None)
                } else {
                    w.parse::<f64>().map(Some)
                }
            })
            .collect::<Result<_, _>>()
            .map_err(|_| no("sdtesti: numbers written out are translated"))?;
        let num = |v: Option<f64>| v.map(r_number).unwrap_or_else(|| "NA_real_".into());
        let group =
            |n: Option<f64>, m: Option<f64>, s: Option<f64>| -> Result<String, Untranslated> {
                match (n, s) {
                    (Some(n), Some(s)) if n >= 1.0 && n.fract() == 0.0 && s > 0.0 => Ok(format!(
                        "c(n = {}, m = {}, s = {})",
                        r_number(n),
                        num(m),
                        r_number(s)
                    )),
                    _ => Err(no(
                        "sdtesti: a positive whole #obs and a positive #sd are translated",
                    )),
                }
            };
        let (call, names): (String, &'static [&'static str]) = match nums.as_slice() {
            [n, m, s, Some(v)] => {
                if *v <= 0.0 {
                    return Err(no("sd must be positive (Stata: r(198))"));
                }
                (
                    format!(
                        "stata_sdtest(list({}), \"x\", \"        \", value = {}",
                        group(*n, *m, *s)?,
                        r_number(*v)
                    ),
                    R_SDTEST1,
                )
            }
            [n1, m1, s1, n2, m2, s2] => {
                if m1.is_none() || m2.is_none() {
                    return Err(no("sdtesti: the two means written out are translated"));
                }
                let (a, b) = (group(*n1, *m1, *s1)?, group(*n2, *m2, *s2)?);
                (
                    format!(
                        "stata_sdtest(list({a}, {b}), c(\"x\", \"y\"), \"        \", combined = stata_sd_pool({a}, {b})"
                    ),
                    R_SDTEST2,
                )
            }
            _ => {
                return Err(no(
                    "sdtesti takes #obs #mean #sd #val, or #obs1 #mean1 #sd1 #obs2 #mean2 #sd2 (Stata: r(198))",
                ))
            }
        };
        for h in [
            "stata_sdtest",
            "stata_fmt",
            "stata_returned",
            "stata_tables",
        ] {
            self.helpers.insert(h);
        }
        out.line(format!(
            "stata_r <- {call}, level = {level}{})",
            if self.quiet(c) { ", show = FALSE" } else { "" }
        ));
        self.r_new = Some((names, "sdtest".into()));
        Ok(())
    }
}
