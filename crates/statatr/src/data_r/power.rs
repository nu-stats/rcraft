//! `power onemean | twomeans | pairedmeans | oneproportion | twoproportions`:
//! the sample size, the power or the target value, as Stata 19 works them out
//! and prints them (`plan/stata/power.md`). The power functions are the
//! [PSS] manual's: t (noncentral t) or z tests of means, Satterthwaite's t
//! for unequal standard deviations, the score or Wald test of one proportion,
//! Pearson's chi-squared (the pooled z) test of two with Fleiss's continuity
//! correction. Sample sizes are the smallest whole numbers reaching the
//! power (the second group's from the first's, rounded up, times nratio()),
//! the target values R's root of the power function.

use super::*;

pub(super) const POWER_HELPER: &str = r##"
stata_power <- function(method, v1, v2 = NULL, alpha = 0.05, power = NULL, n = NULL, n1 = NULL, n2 = NULL,
                        nratio = NULL, onesided = FALSE, direction = "upper", sd = NULL, sd1 = NULL, sd2 = NULL,
                        known = FALSE, sddiff = NULL, corr = NULL, test = "score", continuity = FALSE,
                        effect = "diff", show = TRUE) {
  two <- method %in% c("twomeans", "twoproportions")
  props <- method %in% c("oneproportion", "twoproportions")
  stop_r <- function(m) stop(m, call. = FALSE)
  # what is worked out: the sample size, the power, or the effect (the second value)
  find <- if (is.null(v2)) "effect" else if (is.null(power) && (!is.null(n) || !is.null(n1) || !is.null(n2))) "power" else "n"
  if (find == "effect" && (is.null(power) || (is.null(n) && is.null(n1)))) stop_r("power: the target needs n() and power() (Stata: r(198))")
  target <- if (is.null(power)) 0.8 else power
  if (is.null(nratio)) nratio <- if (!is.null(n1) && !is.null(n2)) n2 / n1 else 1
  # the standard deviations
  unequal <- FALSE
  if (method == "onemean") s <- if (is.null(sd)) 1 else sd
  if (method == "pairedmeans") {
    if (!is.null(corr)) {
      sdc <- if (is.null(sd)) 1 else sd
      s <- sdc * sqrt(2 * (1 - corr))
    } else s <- if (is.null(sddiff)) 1 else sddiff
  }
  if (method == "twomeans") {
    unequal <- !is.null(sd1) || !is.null(sd2)
    if (unequal) {
      sd1 <- if (is.null(sd1)) 1 else sd1
      sd2 <- if (is.null(sd2)) 1 else sd2
      unequal <- sd1 != sd2
    }
    if (!unequal) {
      s <- if (!is.null(sd)) sd else if (!is.null(sd1)) sd1 else 1
      sd1 <- s
      sd2 <- s
    }
  }
  side <- if (onesided) 1 else 2
  zc <- stats::qnorm(1 - alpha / side)
  # the power at group sizes a (and b) for the second value x
  pw <- function(a, b, x) suppressWarnings(pw0(a, b, x))
  pw0 <- function(a, b, x) {
    up <- function(cut, df, l) if (is.null(df)) 1 - stats::pnorm(cut - l) else 1 - stats::pt(cut, df, l)
    dn <- function(cut, df, l) if (is.null(df)) stats::pnorm(-cut - l) else stats::pt(-cut, df, l)
    tails <- function(l, df) {
      cut <- if (is.null(df)) zc else stats::qt(1 - alpha / side, df)
      if (!onesided) up(cut, df, l) + dn(cut, df, l) else if (l >= 0) up(cut, df, l) else dn(cut, df, l)
    }
    if (method %in% c("onemean", "pairedmeans")) return(tails(sqrt(a) * (x - v1) / s, if (known) NULL else a - 1))
    if (method == "twomeans") {
      if (!unequal) return(tails((x - v1) / (s * sqrt(1 / a + 1 / b)), if (known) NULL else a + b - 2))
      va <- sd1^2 / a
      vb <- sd2^2 / b
      return(tails((x - v1) / sqrt(va + vb), if (known) NULL else (va + vb)^2 / (va^2 / (a - 1) + vb^2 / (b - 1))))
    }
    if (method == "oneproportion") {
      s0 <- sqrt(v1 * (1 - v1))
      sa <- sqrt(x * (1 - x))
      d <- x - v1
      if (test == "wald") s0 <- sa
      upp <- stats::pnorm((sqrt(a) * d - zc * s0) / sa)
      low <- stats::pnorm((-sqrt(a) * d - zc * s0) / sa)
      return(if (!onesided) upp + low else if (d >= 0) upp else low)
    }
    pb <- (a * v1 + b * x) / (a + b)
    d <- x - v1
    s0 <- sqrt(pb * (1 - pb) * (1 / a + 1 / b))
    sa <- sqrt(v1 * (1 - v1) / a + x * (1 - x) / b)
    # Fleiss's continuity correction, taken off in both tails (Stata 19)
    cc <- if (continuity) (1 / a + 1 / b) / 2 else 0
    upp <- stats::pnorm((abs(d) - cc - zc * s0) / sa)
    low <- stats::pnorm((-abs(d) - cc - zc * s0) / sa)
    if (!onesided) upp + low else upp
  }
  # the group sizes from n(), n1(), n2(); a total that does not split into whole groups at
  # nratio() is cut to the groups rounded down, as Stata 19's "Actual sample sizes"
  actual <- FALSE
  sizes <- function() {
    if (!two) return(c(n, NA))
    if (!is.null(n1) && !is.null(n2)) return(c(n1, n2))
    a <- n / (1 + nratio)
    if (abs(a - round(a)) < 1e-8 && abs(nratio * round(a) - (n - round(a))) < 1e-8) return(c(round(a), n - round(a)))
    actual <<- TRUE
    a <- floor(a + 1e-9)
    c(a, floor(nratio * a + 1e-9))
  }
  tol <- 1e-12
  iterated <- TRUE
  closed <- onesided && (known || props) && !(find == "effect" && props)
  if (find == "power") {
    g <- sizes()
    pv <- pw(g[1], g[2], v2)
    N <- if (!two) g[1] else if (actual) n else sum(g)
    x2 <- v2
    iterated <- FALSE
  } else if (find == "n") {
    f <- function(a) pw(a, nratio * a, v2) - target
    # the smallest group where the test is defined (a t test needs 2; with continuity the corrected
    # difference must stay above 0)
    lo <- if (known || props) 1 else 2 + 1e-9
    if (continuity) lo <- max(lo, (1 + 1 / nratio) / (2 * abs(v2 - v1)) + 1e-6)
    if (f(lo) >= 0) a <- lo else a <- stats::uniroot(f, c(lo, 10), extendInt = "upX", tol = tol)$root
    if (two) {
      A <- ceiling(a - 1e-9)
      g <- c(A, ceiling(nratio * A - 1e-9))
      N <- sum(g)
    } else {
      g <- c(ceiling(a - 1e-9), NA)
      N <- g[1]
    }
    pv <- target
    x2 <- v2
    iterated <- !closed
  } else {
    g <- sizes()
    N <- if (!two) g[1] else if (actual) n else sum(g)
    upper <- direction != "lower"
    f <- function(x) pw(g[1], g[2], x) - target
    if (props) {
      eps <- 1e-10
      x2 <- if (upper) stats::uniroot(f, c(v1 + eps, 1 - eps), tol = tol)$root else stats::uniroot(f, c(eps, v1 - eps), tol = tol)$root
    } else {
      sc <- if (method == "twomeans" && unequal) max(sd1, sd2) else s
      x2 <- if (upper) stats::uniroot(f, c(v1 + 1e-9 * sc, v1 + sc), extendInt = "upX", tol = tol)$root else
        stats::uniroot(f, c(v1 - sc, v1 - 1e-9 * sc), extendInt = "downX", tol = tol)$root
    }
    pv <- target
    iterated <- !closed
  }
  diff <- x2 - v1
  delta <- switch(method, onemean = diff / s, pairedmeans = diff / s, twomeans = diff, oneproportion = diff,
                  twoproportions = switch(effect, diff = diff, ratio = x2 / v1, oratio = (x2 / (1 - x2)) / (v1 / (1 - v1))))
  # r() in the order of Stata 19's return list for this form
  it <- if (iterated) list(converged = 1, maxiter = 500, tolerance = 1e-12, ftolerance = 1e-12) else list()
  head <- list(power = pv, beta = 1 - pv, alpha = alpha)
  nf <- if (find != "power" && method %in% c("onemean", "pairedmeans", "oneproportion") || two) list(nfractional = 0) else list()
  if (method %in% c("onemean", "pairedmeans", "oneproportion") && find != "n") nf <- list()
  os <- list(onesided = as.numeric(onesided))
  # the total given, then (when it did not split) the total of the groups
  NN <- c(list(N = N), if (actual) list(N_a = sum(g)))
  ks <- if (method == "twomeans") (if (known) list(knownsds = 1) else list(knownsds = 0, unequal = as.numeric(unequal))) else list()
  r <- switch(method,
    onemean = {
      tail <- if (iterated && find == "n") list(m0 = v1, delta = delta, knownsd = as.numeric(known), sd = s, diff = diff, ma = x2)
        else if (iterated) list(m0 = v1, ma = x2, delta = delta, knownsd = as.numeric(known), sd = s, diff = diff)
        else list(m0 = v1, ma = x2, diff = diff, sd = s, delta = delta, knownsd = as.numeric(known))
      c(head, list(N = N), nf, os, it, tail)
    },
    pairedmeans = {
      sds <- if (is.null(corr)) list(sd_d = s) else list(corr = corr, sd2 = sdc, sd1 = sdc, sd = sdc, sd_d = s)
      tail <- if (find == "power") c(list(d0 = 0, da = diff, ma1 = v1, ma2 = x2, sd_d = s),
                                     if (!is.null(corr)) list(sd = sdc, sd1 = sdc, sd2 = sdc, corr = corr),
                                     list(delta = delta, knownsd = as.numeric(known)))
        else if (find == "n") c(list(d0 = 0, delta = delta, knownsd = as.numeric(known)), sds, list(ma2 = x2, ma1 = v1, da = diff))
        else c(list(d0 = 0, da = diff, delta = delta, knownsd = as.numeric(known)), sds, list(ma2 = x2, ma1 = v1))
      c(head, list(N = N), nf, os, it, tail)
    },
    oneproportion = {
      tail <- if (!iterated) list(p0 = v1, pa = x2, delta = delta, diff = diff, continuity = 0)
        else if (find == "n") list(p0 = v1, delta = delta, continuity = 0, diff = diff, pa = x2)
        else list(p0 = v1, pa = x2, delta = delta, continuity = 0, diff = diff)
      c(head, list(N = N), nf, os, it, tail)
    },
    twomeans = {
      sdl <- if (unequal) list(sd1 = sd1, sd2 = sd2) else list(sd = s, sd1 = sd1, sd2 = sd2)
      sdr <- if (unequal) list(sd2 = sd2, sd1 = sd1) else list(sd2 = sd2, sd1 = sd1, sd = s)
      ksr <- rev(ks)
      tail <- if (!iterated) c(NN, list(N1 = g[1], N2 = g[2], nratio = nratio, m1 = v1, m2 = x2, diff = diff), sdl, ks)
        else if (find == "n") c(list(N = N), ksr, sdr, list(diff = diff, m2 = x2, m1 = v1, nratio = nratio, N2 = g[2], N1 = g[1]))
        else c(NN, list(N1 = g[1]), ksr, sdr, list(diff = diff, m2 = x2, m1 = v1, nratio = nratio, N2 = g[2]))
      c(head, list(delta = delta), nf, os, it, tail)
    },
    twoproportions = {
      na <- g[2] / g[1]
      tail <- if (!iterated) c(NN, list(N1 = g[1], N2 = g[2], nratio = nratio, p1 = v1, p2 = x2, continuity = as.numeric(continuity)))
        else if (find == "n") c(list(N = N, continuity = as.numeric(continuity), p2 = x2, p1 = v1),
                                if (abs(na - nratio) > 1e-12) list(nratio_a = na), list(nratio = nratio, N2 = g[2], N1 = g[1]))
        else c(NN, list(N1 = g[1], continuity = as.numeric(continuity), p2 = x2, p1 = v1, nratio = nratio, N2 = g[2]))
      c(head, list(delta = delta), nf, os, it, tail)
    })
  r <- c(r, list(type = "test", method = method), if (find == "effect") list(direction = if (direction == "lower") "lower" else "upper"),
         if (method == "oneproportion") list(test = test), if (method == "twoproportions") list(test = "chi2", effect = effect))
  if (!show) return(r)
  # the printout
  title <- switch(method, onemean = "one-sample mean test", pairedmeans = "two-sample paired-means test",
                  twomeans = "two-sample means test", oneproportion = "one-sample proportion test",
                  twoproportions = "two-sample proportions test")
  what <- if (find == "power") "power" else if (find == "n") (if (two) "sample sizes" else "sample size") else
    switch(method, onemean = "target mean", pairedmeans = "target parameters", twomeans = "experimental-group mean",
           oneproportion = "target proportion", twoproportions = "experimental-group proportion")
  testline <- switch(method,
    onemean = if (known) "z test" else "t test",
    pairedmeans = paste0(if (known) "Paired z test" else "Paired t test", if (!is.null(corr)) " assuming sd1 = sd2 = sd" else ""),
    twomeans = if (unequal) (if (known) "z test assuming unequal variances" else "Satterthwaite's t test assuming unequal variances")
               else if (known) "z test assuming sd1 = sd2 = sd" else "t test assuming sd1 = sd2 = sd",
    oneproportion = if (test == "wald") "Wald z test" else "Score z test",
    twoproportions = "Pearson's chi-squared test ")
  sym <- switch(method, onemean = c("m", "m0", "ma"), pairedmeans = c("d", "d0", "da"), twomeans = c("m2", "m1", "m2"),
                oneproportion = c("p", "p0", "pa"), twoproportions = c("p2", "p1", "p2"))
  rel <- if (!onesided) "!=" else if (diff >= 0) ">" else "<"
  h0 <- sprintf("H0: %s = %s  versus  Ha: %s %s %s", sym[1], sym[2], sym[1], rel, sym[2])
  # the target's side: always after one sample, after two only when two-sided (Stata 19)
  if (find == "effect" && (!onesided || !two)) h0 <- paste0(h0, "; ", sym[3], if (direction == "lower") " < " else " > ", sym[2])
  num <- function(k, v, extra = "") sprintf("%13s = %9.4f%s", k, v, extra)
  int <- function(k, v) sprintf("%13s = %9.0f", k, v)
  eff <- if (method == "twoproportions") paste0("  (", switch(effect, diff = "difference", ratio = "ratio", oratio = "odds ratio"), ")") else ""
  sizes_lines <- function(tot = N) {
    if (!two) return(int("N", tot))
    c(int("N", tot), if (g[1] == g[2]) int("N per group", g[1]) else c(int("N1", g[1]), int("N2", g[2])))
  }
  given_n <- if (!two) int("N", N) else if (actual) c(int("N", N), if (nratio != 1) num("N2/N1", nratio)) else
    c(sizes_lines(), if (g[1] != g[2]) num("N2/N1", g[2] / g[1]))
  given <- c(num("alpha", alpha), if (find != "power") num("power", target), if (find != "n") given_n,
             if (find != "effect") num("delta", delta, eff))
  right <- character()
  vals <- switch(method,
    onemean = c(num("m0", v1), if (find != "effect") num("ma", x2), num("sd", s)),
    pairedmeans = {
      right <- c(num("ma1", v1), if (find != "effect") num("ma2", x2), if (!is.null(corr)) c(num("sd", sdc), num("corr", corr)))
      c(num("d0", 0), if (find != "effect") num("da", diff), num("sd_d", s))
    },
    twomeans = c(num("m1", v1), if (find != "effect") num("m2", x2), if (unequal) c(num("sd1", sd1), num("sd2", sd2)) else num("sd", s),
                 if (find == "n" && nratio != 1) num("N2/N1", nratio)),
    oneproportion = c(num("p0", v1), if (find != "effect") num("pa", x2)),
    twoproportions = c(num("p1", v1), if (find != "effect") num("p2", x2), if (find == "n" && nratio != 1) num("N2/N1", nratio)))
  left <- c(given, vals)
  if (length(right)) left <- paste0(left, c(right, rep("", length(left) - length(right))))
  cat("\n")
  if (iterated) cat("Performing iteration ...\n\n")
  cat("Estimated ", what, " for a ", title, "\n", testline, "\n", h0, "\n\nStudy parameters:\n\n", sep = "")
  cat(left, sep = "\n")
  if (actual) {
    cat("\nActual sample sizes:\n\n")
    cat(sizes_lines(sum(g)), sep = "\n")
  }
  if (find == "power") {
    cat("\nEstimated power:\n\n", num("power", pv), "\n", sep = "")
  } else if (find == "n") {
    cat("\nEstimated ", what, ":\n\n", sep = "")
    cat(sizes_lines(), sep = "\n")
    if (two && g[1] != g[2] && abs(g[2] / g[1] - nratio) > 1e-12) cat(num("N2/N1", g[2] / g[1]), "\n", sep = "")
  } else {
    cat("\nEstimated effect size and ", what, ":\n\n", sep = "")
    cat(num("delta", delta, eff), "\n", sep = "")
    cat(switch(method, onemean = num("ma", x2), pairedmeans = c(num("da", diff), num("ma2", x2)), twomeans = num("m2", x2),
               oneproportion = num("pa", x2), twoproportions = num("p2", x2)), sep = "\n")
  }
  cat("\n")
  invisible(r)
}
"##;

/// The r() power leaves (Stata 19), but its search's own r(init), r(iter),
/// r(deltax) and r(function).
const R_POWER: &[&str] = &[
    "power",
    "beta",
    "alpha",
    "N",
    "N_a",
    "nfractional",
    "onesided",
    "converged",
    "maxiter",
    "tolerance",
    "ftolerance",
    "delta",
    "diff",
    "m0",
    "ma",
    "sd",
    "knownsd",
    "m1",
    "m2",
    "sd1",
    "sd2",
    "knownsds",
    "unequal",
    "N1",
    "N2",
    "nratio",
    "nratio_a",
    "d0",
    "da",
    "ma1",
    "ma2",
    "sd_d",
    "corr",
    "p0",
    "pa",
    "p1",
    "p2",
    "continuity",
    "type",
    "method",
    "direction",
    "test",
    "effect",
];

/// The methods translated, and the options each takes beyond alpha(),
/// power(), n(), onesided and direction(), at their full names (no
/// abbreviation was checked).
const METHODS: &[(&str, &[(&str, usize)])] = &[
    ("onemean", &[("sd", 2), ("knownsd", 7)]),
    (
        "twomeans",
        &[
            ("sd", 2),
            ("sd1", 3),
            ("sd2", 3),
            ("knownsds", 8),
            ("n1", 2),
            ("n2", 2),
            ("nratio", 6),
        ],
    ),
    (
        "pairedmeans",
        &[("sddiff", 6), ("sd", 2), ("corr", 4), ("knownsd", 7)],
    ),
    ("oneproportion", &[("test", 4)]),
    (
        "twoproportions",
        &[
            ("n1", 2),
            ("n2", 2),
            ("nratio", 6),
            ("continuity", 10),
            ("effect", 6),
            ("test", 4),
        ],
    ),
];

/// One number of the line: a value, or an option's.
fn number(what: &str, text: &str) -> Result<f64, Untranslated> {
    let t = text.trim();
    if t.split_whitespace().count() > 1 || t.contains('(') {
        return Err(no(format!(
            "power: {what} {t}: a list of values (Stata's table of results) is not translated yet"
        )));
    }
    t.parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
        .ok_or_else(|| no(format!("power: {what} {t} needs a number")))
}

/// What an option's number must be.
#[derive(Clone, Copy)]
enum Range {
    /// Between 0 and 1.
    Prob,
    /// Above 0.
    Positive,
    /// A whole number, 1 or more.
    Whole,
    /// Between -1 and 1.
    Corr,
}

impl Range {
    fn holds(self, x: f64) -> bool {
        match self {
            Range::Prob => x > 0.0 && x < 1.0,
            Range::Positive => x > 0.0,
            Range::Whole => x >= 1.0 && x.fract() == 0.0,
            Range::Corr => x > -1.0 && x < 1.0,
        }
    }

    fn why(self) -> &'static str {
        match self {
            Range::Prob => "must be between 0 and 1",
            Range::Positive => "must be above 0",
            Range::Whole => "must be a whole number above 0",
            Range::Corr => "must be between -1 and 1",
        }
    }
}

/// The number options, with their shortest forms and ranges.
const NUMBERS: &[(&str, usize, Range)] = &[
    ("alpha", 5, Range::Prob),
    ("power", 5, Range::Prob),
    ("n", 1, Range::Whole),
    ("n1", 2, Range::Whole),
    ("n2", 2, Range::Whole),
    ("nratio", 6, Range::Positive),
    ("sd", 2, Range::Positive),
    ("sd1", 3, Range::Positive),
    ("sd2", 3, Range::Positive),
    ("sddiff", 6, Range::Positive),
    ("corr", 4, Range::Corr),
];

impl Session {
    pub(super) fn power(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "power")?;
        if c.parts.weight.is_some() {
            return Err(no("power takes no weights (Stata: r(101))"));
        }
        let main = c.parts.main.trim();
        let (method, rest) = first_word(main);
        let Some((method, extra)) = METHODS.iter().find(|(m, _)| *m == method) else {
            return Err(no(format!(
                "power {method} is not translated yet (onemean, twomeans, pairedmeans, oneproportion and twoproportions are)"
            )));
        };
        let mut known: Vec<(&str, usize)> = vec![
            ("alpha", 5),
            ("power", 5),
            ("n", 1),
            ("onesided", 8),
            ("direction", 9),
        ];
        known.extend_from_slice(extra);
        if let Some(o) = opts.other(&known) {
            return Err(no(format!(
                "power {method} with the option {o} is not translated yet"
            )));
        }
        if rest.contains('(') {
            return Err(no(format!(
                "power {method} {rest}: a list of values (Stata's table of results) is not translated yet"
            )));
        }
        let values: Vec<&str> = rest.split_whitespace().collect();
        if values.is_empty() || values.len() > 2 {
            return Err(no(format!(
                "power {method} takes one or two values (Stata: r(198))"
            )));
        }
        let props = method.contains("proportion");
        let mut args = vec![r_text(method)];
        for (i, v) in values.iter().enumerate() {
            let x = number("the value", v)?;
            if props && !(x > 0.0 && x < 1.0) {
                return Err(no(format!(
                    "power {method}: {v} must be between 0 and 1 (Stata: r(198))"
                )));
            }
            args.push(if i == 0 {
                r_number(x)
            } else {
                format!("v2 = {}", r_number(x))
            });
        }
        for &(name, min, range) in NUMBERS {
            if !known.iter().any(|(k, _)| *k == name) {
                continue;
            }
            if let Some(t) = opts.arg(name, min) {
                let x = number(&format!("{name}()"), &t)?;
                if !range.holds(x) {
                    return Err(no(format!(
                        "power {method}: {name}({}) {} (Stata: r(198))",
                        t.trim(),
                        range.why()
                    )));
                }
                args.push(format!("{name} = {}", r_number(x)));
            }
        }
        let has_n = opts.has("n", 1);
        let (n1, n2) = (opts.has("n1", 2), opts.has("n2", 2));
        if n1 != n2 {
            return Err(no(format!(
                "power {method}: n1() or n2() alone is not translated yet; give both"
            )));
        }
        if n1 && (has_n || opts.has("nratio", 6)) {
            return Err(no(format!(
                "power {method}: n() or nratio() with n1() and n2() is not translated yet"
            )));
        }
        if opts.has("sddiff", 6) && opts.has("corr", 4) {
            return Err(no(
                "power pairedmeans: sddiff() with corr() is not translated yet",
            ));
        }
        if opts.has("sd", 2) && (opts.has("sd1", 3) || opts.has("sd2", 3)) {
            return Err(no(
                "power twomeans: sd() with sd1() or sd2() is not translated yet",
            ));
        }
        if values.len() == 1 && !(opts.has("power", 5) && (has_n || n1)) {
            return Err(no(format!(
                "power {method} {rest}: the target value needs n() and power() (Stata: r(198))"
            )));
        }
        if opts.has("onesided", 8) {
            args.push("onesided = TRUE".into());
        }
        if let Some(d) = opts.arg("direction", 9) {
            let d = d.trim().to_lowercase();
            if !matches!(d.as_str(), "upper" | "lower") {
                return Err(no(format!(
                    "power {method}: direction({d}) is upper or lower (Stata: r(198))"
                )));
            }
            args.push(format!("direction = {}", r_text(&d)));
        }
        if opts.has("knownsd", 7) || opts.has("knownsds", 8) {
            args.push("known = TRUE".into());
        }
        if let Some(t) = opts.arg("test", 4) {
            let t = t.trim().to_lowercase();
            match (*method, t.as_str()) {
                ("oneproportion", "score" | "wald") | ("twoproportions", "chi2") => {
                    args.push(format!("test = {}", r_text(&t)))
                }
                _ => {
                    return Err(no(format!(
                        "power {method}, test({t}) is not translated yet"
                    )))
                }
            }
        }
        if opts.has("continuity", 10) {
            args.push("continuity = TRUE".into());
        }
        if let Some(e) = opts.arg("effect", 6) {
            let e = e.trim().to_lowercase();
            if !matches!(e.as_str(), "diff" | "ratio" | "oratio") {
                return Err(no(format!(
                    "power twoproportions, effect({e}) is not translated yet"
                )));
            }
            args.push(format!("effect = {}", r_text(&e)));
        }
        if self.quiet(c) {
            args.push("show = FALSE".into());
        }
        for h in ["stata_power", "stata_returned"] {
            self.helpers.insert(h);
        }
        out.line(format!("stata_r <- stata_power({})", args.join(", ")));
        self.r_new = Some((R_POWER, "power".into()));
        Ok(())
    }
}
