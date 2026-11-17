//! `npregress kernel` with one continuous covariate, as Stata 19 fits it.
//! The method is StatCraft's decoding of Stata's (its `plan/stata/npregress.md`,
//! probes 554 and 555), our own work, written here in R:
//!
//! - the mean at each point by the local-linear (or local-constant) fit, the
//!   derivative by the local-linear slope at its own bandwidth;
//! - the mean's bandwidth by leave-one-out cross-validation, the derivative's
//!   by a local-cubic proxy, each found by Stata's one-dimensional Nelder-Mead;
//! - Doksum and Samarov's R-squared, the identification rule, the stored
//!   results and the two variables Stata leaves.
//!
//! More covariates, factor variables, imaic, vce(bootstrap), bandwidths
//! given as matrices, and predict or margins afterwards are not translated.

use super::returned::Returned;
use super::*;

pub(super) const NPREGRESS_HELPER: &str = r##"stata_np_kernel <- function(z, k) {
  a <- abs(z)
  switch(k,
    epanechnikov = ifelse(a < sqrt(5), 0.75 * (1 - z * z / 5) / sqrt(5), 0),
    epan2 = ifelse(a < 1, 0.75 * (1 - z * z), 0),
    biweight = ifelse(a < 1, 15 / 16 * (1 - z * z)^2, 0),
    cosine = ifelse(a < 0.5, 1 + cos(2 * pi * z), 0),
    gaussian = exp(-0.5 * z * z) / sqrt(2 * pi),
    parzen = ifelse(a <= 0.5, 4 / 3 - 8 * z * z + 8 * a^3, ifelse(a <= 1, 8 * (1 - a)^3 / 3, 0)),
    rectangle = ifelse(a < 1, 0.5, 0),
    triangle = pmax(1 - a, 0),
    triweight = ifelse(a < 1, 35 / 32 * (1 - z * z)^3, 0))
}
# Gaussian elimination with partial pivoting; NULL when a pivot is under 1e-14 of the largest diagonal
stata_np_solve <- function(a, b) {
  n <- length(b); scale <- max(abs(diag(a)))
  for (c in seq_len(n)) {
    col <- abs(a[c:n, c]); p <- c - 1 + max(which(col == max(col)))
    if (p != c) { t <- a[c, ]; a[c, ] <- a[p, ]; a[p, ] <- t; t <- b[c]; b[c] <- b[p]; b[p] <- t }
    pv <- a[c, c]
    if (abs(pv) <= scale * 1e-14 || pv == 0) return(NULL)
    if (c < n) for (r in (c + 1):n) {
      f <- a[r, c] / pv
      if (f != 0) { a[r, c:n] <- a[r, c:n] - f * a[c, c:n]; b[r] <- b[r] - f * b[c] }
    }
  }
  x <- numeric(n)
  for (r in n:1) x[r] <- (b[r] - (if (r < n) sum(a[r, (r + 1):n] * x[(r + 1):n]) else 0)) / a[r, r]
  x
}
# the local polynomial of degree deg at x0 (level, slope, ...), NULL when not identified
stata_np_local <- function(x, y, x0, h, k, deg, skip = 0L) {
  d <- x - x0; w <- stata_np_kernel(d / h, k)
  if (skip > 0) w[skip] <- 0
  keep <- w != 0; d <- d[keep]; w <- w[keep]; yy <- y[keep]
  m <- deg + 1
  pw <- matrix(1, length(d), 2 * m - 1)
  if (2 * m - 1 > 1) for (j in 2:(2 * m - 1)) pw[, j] <- pw[, j - 1] * d
  S <- colSums(w * pw); Tm <- colSums(w * pw[, seq_len(m), drop = FALSE] * yy)
  if (!length(d) || S[1] <= 0) return(NULL)
  if (deg == 0) return(Tm[1] / S[1])
  A <- matrix(0, m, m); for (r in 1:m) for (c in 1:m) A[r, c] <- S[r + c - 1]
  stata_np_solve(A, Tm)
}
stata_np_meancrit <- function(x, y, h, k, deg) {
  if (is.na(h) || h <= 0) return(Inf)
  ss <- 0
  for (i in seq_along(x)) {
    f <- stata_np_local(x, y, x[i], h, k, deg, i)
    if (is.null(f)) return(Inf)
    ss <- ss + (y[i] - f[1])^2
  }
  if (deg == 0) log(ss) / length(x) else log(ss / length(x))
}
stata_np_effcrit <- function(x, y, h, k) {
  if (is.na(h) || h <= 0) return(NULL)
  ss <- 0
  for (i in seq_along(x)) {
    g <- stata_np_local(x, y, x[i], h, k, 1, i); cc <- stata_np_local(x, y, x[i], h, k, 3)
    if (is.null(g) || is.null(cc)) return(NULL)
    ss <- ss + (g[2] - cc[2])^2
  }
  ss / length(x)
}
# Stata's one-dimensional Nelder-Mead (StatCraft's decoding, plan/stata/npregress.md)
stata_np_nm <- function(f, p0, maxit) {
  rd <- function(u, v) abs(u - v) / (abs(v) + 1)
  f0 <- f(p0); log <- f0; pt <- c(p0, p0 + 0.05); val <- c(f0, f(p0 + 0.05)); k <- 0
  repeat {
    if (val[2] < val[1]) { b <- pt[2]; fb <- val[2]; wv <- pt[1]; fw <- val[1]; w <- 1 } else { b <- pt[1]; fb <- val[1]; wv <- pt[2]; fw <- val[2]; w <- 2 }
    if (k > 0 && (rd(fb, fw) < 1e-5 || rd(b, wv) < 1e-4)) return(list(best = b, log = log, converged = TRUE))
    if (k >= maxit) return(list(best = b, log = log, converged = FALSE))
    r <- b + (b - wv); fr <- f(r)
    if (fr < fb) { e <- b + 2 * (b - wv); fe <- f(e); nw <- if (fe < fr) c(e, fe) else c(r, fr) }
    else if (fr < fw) { cc <- b + 0.5 * (r - b); fc <- f(cc); nw <- if (fc <= fr) c(cc, fc) else { m <- b + 0.5 * (wv - b); c(m, f(m)) } }
    else { cc <- b + 0.5 * (wv - b); nw <- c(cc, f(cc)) }
    pt[w] <- nw[1]; val[w] <- nw[2]; k <- k + 1; log <- c(log, min(fb, nw[2]))
  }
}
stata_np_para <- function(text) {
  width <- max((if (exists("stata_linesize")) stata_linesize else 80) - 2, 20)
  line <- ""
  for (w in strsplit(text, " ", fixed = TRUE)[[1]]) {
    if (nzchar(trimws(line)) && nchar(line) + 1 + nchar(w) > width) { cat(line, "\n", sep = ""); line <- paste0("         ", w) }
    else if (!nzchar(line)) line <- w else line <- paste(line, w)
  }
  cat(line, "\n", sep = "")
}
stata_np_show <- function(e) {
  g9 <- function(v) stata_fmt(v, "%9.0g")
  bw <- e$bwidth; cols <- ncol(bw); xname <- sub("^.*:", "", colnames(bw)[1])
  short <- function(s) if (nchar(s) > 12) paste0(substr(s, 1, 10), "~", substr(s, nchar(s), nchar(s))) else s
  width <- 13 + 11 * cols
  cat("\nBandwidth\n", strrep("-", width), "\n", sep = "")
  cat("             |", sprintf("%10s ", "Mean"), if (cols > 1) sprintf("%10s ", "Effect"), "\n", sep = "")
  cat(strrep("-", 13), "+", strrep("-", 11 * cols - 1), "\n", sep = "")
  cat(sprintf("%12s |", short(xname)), paste(sprintf("%10s ", vapply(bw[1, ], g9, "")), collapse = ""), "\n", sep = "")
  cat(strrep("-", width), "\n\n", sep = "")
  given <- isTRUE(attr(e, "stata_np_given"))
  kl <- if (given) paste0("Kernel: ", e$kname) else paste0("Kernel   : ", e$kname)
  bl <- if (given) "" else "Bandwidth: cross-validation"
  right <- function(l, v) paste0(formatC(l, width = -19), "=", formatC(v, width = 15))
  pad <- function(s) formatC(s, width = -43)
  cat(pad(e$title), right("Number of obs", stata_fmt(e$N, "%15.0f", trim = TRUE)), "\n", sep = "")
  cat(pad(kl), right("E(Kernel obs)", stata_fmt(e$nh, "%15.0f", trim = TRUE)), "\n", sep = "")
  cat(pad(bl), right("R-squared", stata_fmt(e$r2, "%15.4f", trim = TRUE)), "\n", sep = "")
  rule <- paste0(strrep("-", 13), "+", strrep("-", 64))
  cat(strrep("-", 78), "\n", sprintf("%12s |   Estimate", short(e$depvar)), "\n", rule, "\n", sep = "")
  b <- e$b
  if (ncol(b) > 1) {
    cat("Mean         |\n", sprintf("%12s |  %9s", short(e$depvar), g9(b[1, 1])), "\n", rule, "\n", sep = "")
    cat("Effect       |\n", sprintf("%12s |  %9s", short(xname), g9(b[1, 2])), "\n", sep = "")
  } else cat(sprintf("%12s |  %9s", short(e$depvar), g9(b[1, 1])), "\n", sep = "")
  cat(strrep("-", 78), "\n", "Note: Effect estimates are averages of derivatives.\n",
      "Note: You may compute standard errors using vce(bootstrap) or reps().\n", sep = "")
  if (e$converged == 0) cat("Warning: Convergence not achieved.\n")
}
stata_np_replay <- function() {
  if (!identical(stata_e$cmd, "npregress")) stop("last estimates not found (Stata: r(301))", call. = FALSE)
  stata_np_show(stata_e)
}
# npregress kernel y x: the fit, its stored results and its two variables (returned with the data)
stata_npregress <- function(d, sel, yname, xname, k = "epanechnikov", constant = FALSE, noderiv = FALSE, nolog = FALSE,
                            maxit = 300, gm = NULL, ge = NULL, bothgiven = FALSE, from = NULL, mvar, dvar = NULL,
                            unident = NULL, dname = NULL, cmdline = "", show = TRUE) {
  for (v in c(yname, xname)) if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
  num <- function(v) { u <- unclass(d[[v]]); attributes(u) <- NULL; as.numeric(u) }
  if (is.character(d[[yname]]) || is.character(d[[xname]])) stop("type mismatch (Stata: r(109))", call. = FALSE)
  xv <- num(xname); yv <- num(yname)
  rows <- which(sel & !is.na(xv) & !is.na(yv))
  if (!length(rows)) stop("no observations (Stata: r(2000))", call. = FALSE)
  say <- show && !nolog
  if (show && constant && !noderiv) {
    cat("\n"); stata_np_para("note: Derivatives for continuous regressors are only valid with default local-linear estimator. Estimator has been changed to local linear. If you want the local-constant estimator, specify the noderivative option.")
  }
  if (constant && !noderiv) constant <- FALSE
  deg <- if (constant) 0 else 1
  x <- xv[rows]; y <- yv[rows]; n <- length(x)
  logline <- function(lg) for (i in seq_along(lg)) cat(formatC(paste0("Iteration ", i - 1, ":"), width = -14), "Cross-validation criterion = ",
                                                      formatC(stata_fmt(lg[i], "%10.0g", trim = TRUE), width = 10), "  \n", sep = "")
  if (say) cat("\nComputing mean function\n")
  cm <- FALSE; mlog <- numeric(0)
  if (is.null(gm)) {
    h0 <- if (is.null(from)) sqrt(sum((x - mean(x))^2) / (n - 1)) * n^(-0.2) else from
    s <- stata_np_nm(function(h) stata_np_meancrit(x, y, h, k, deg), h0, maxit)
    if (say) { cat("  \nMinimizing cross-validation function:\n  \n"); logline(s$log); if (!s$converged) cat("convergence not achieved\n") }
    cm <- s$converged; mlog <- s$log; hm <- s$best
  } else hm <- gm
  fits <- lapply(x, function(x0) { f <- stata_np_local(x, y, x0, hm, k, deg); if (is.null(f)) NULL else f[1] })
  bad <- vapply(fits, is.null, TRUE)
  if (any(bad)) {
    vname <- if (is.null(unident)) "_unident_sample" else unident
    flag <- rep(NA_real_, nrow(d)); flag[rows] <- as.numeric(bad)
    d[[vname]] <- structure(flag, label = "1 if no identification 0 otherwise")
    nb <- sum(bad)
    cnt <- if (nb == 1) "1 observation was" else paste(nb, "observations were")
    place <- if (is.null(unident)) paste0("in the system variable ", vname, ". You may use the unidentsample() option to use a different variable name.") else paste0("in the variable ", vname, ".")
    if (show) { cat("\n"); stata_np_para(paste0("warning: ", cnt, " not used to compute the mean function because ", if (nb == 1) "it" else "they",
      " violated the model identification assumptions. ", if (nb == 1) "This observation is" else "These observations are", " marked as 1 ", place)) }
    if (nb == n) {
      if (show) cat("\n")
      # Stata keeps the variable that marks them
      if (!is.null(dname)) assign(dname, d, envir = globalenv())
      stop("all observations violate identification condition\n    Most likely, you have included a discrete covariate without specifying factor variable notation. npregress interprets the variable to be continuous.  Please use i.varname for discrete covariates. (Stata: r(198))", call. = FALSE)
    }
  }
  keep <- which(!bad); xs <- x[keep]; ys <- y[keep]; ms <- unlist(fits[keep]); nn <- length(xs)
  ce <- FALSE; elog <- numeric(0); he <- NULL
  if (!noderiv) {
    if (is.null(ge)) {
      s0 <- hm * nn^(2 / 35); cap <- nn / 50; pen <- max(ys)
      s <- stata_np_nm(function(p) {
        inside <- p < cap || (p != 1 && p <= cap * (1 + 1e-12))
        if (!inside) return(pen)
        v <- stata_np_effcrit(xs, ys, s0 * p, k); if (is.null(v)) pen else v
      }, 1, maxit)
      if (say) { cat("  \nComputing optimal derivative bandwidth\n  \n"); logline(s$log); if (!s$converged) cat("convergence not achieved\n") }
      ce <- s$converged; elog <- s$log; he <- s0 * s$best
    } else he <- ge
  }
  slopes <- if (!is.null(he)) vapply(xs, function(x0) { f <- stata_np_local(xs, ys, x0, he, k, 1); if (is.null(f)) NA_real_ else f[2] }, 0) else numeric(0)
  mean_ <- sum(ms) / nn; effect <- if (length(slopes)) mean(slopes[!is.na(slopes)]) else NA
  yb <- sum(ys) / nn
  r2 <- sum((ys - yb) * (ms - yb))^2 / (sum((ys - yb)^2) * sum((ms - yb)^2))
  nh <- min(floor(nn * hm + 0.5), nn)
  mc <- rep(NA_real_, nrow(d)); mc[rows[keep]] <- ms
  d[[mvar]] <- structure(mc, label = "Mean function", format.stata = "%10.0g")
  if (!is.null(dvar) && !noderiv) { dc <- rep(NA_real_, nrow(d)); dc[rows[keep]] <- slopes; d[[dvar]] <- structure(dc, label = paste0("derivative of mean function w.r.t ", xname), format.stata = "%10.0g") }
  converged <- if (bothgiven) TRUE else if (noderiv) cm else cm && ce
  ce <- ce || bothgiven
  e <- list(N = nn, converged = as.numeric(converged))
  if (!noderiv) e$converged_effect <- as.numeric(ce)
  e <- c(e, list(converged_mean = as.numeric(cm), mean = mean_, nh = nh, r2 = r2))
  title <- if (constant) "Local-constant regression" else "Local-linear regression"
  e <- c(e, list(cmdline = cmdline, vce = "none", predict = "npregress_kernel_p", estat_cmd = "npregress_kernel_estat", title = title,
                 estimator = if (constant) "constant" else "linear", marginsprop = "nochainrule", marginsok = "default",
                 bselector = "cross validation", cmd = "npregress", kname = k, datasignaturevars = paste(yname, xname, mvar),
                 properties = "b", depvar = yname))
  mn <- paste0("Mean:", xname); en <- paste0("Effect:", xname)
  row <- function(v, cn, rn = "r1") matrix(v, 1, length(v), dimnames = list(rn, cn))
  il <- function(lg) { v <- numeric(20); m <- min(length(lg), 20); if (m) v[1:m] <- -lg[1:m]; row(v, paste0("c", 1:20)) }
  if (!noderiv) {
    e <- c(e, list(b = row(c(mean_, effect), c(paste0("Mean:", yname), en), "y1"), bwidth = row(c(hm, he), c(mn, en)),
                   derivbwidth = row(he, en), meanbwidth = row(hm, mn), ilog_mean = il(mlog), ilog_effect = il(elog)))
  } else {
    e <- c(e, list(b = row(mean_, paste0("Mean:", yname), "y1"), bwidth = row(hm, mn), meanbwidth = row(hm, mn), ilog_mean = il(mlog)))
  }
  attr(e, "stata_sample") <- TRUE
  attr(e, "stata_np_given") <- bothgiven
  stata_e <<- e
  smp <- logical(nrow(d)); smp[rows[keep]] <- TRUE
  stata_esample <<- smp
  if (show) stata_np_show(e)
  d
}
"##;

/// The kernels `kernel()` takes, with their shortest abbreviations.
const KERNELS: &[(&str, usize)] = &[
    ("epanechnikov", 2),
    ("biweight", 2),
    ("cosine", 2),
    ("gaussian", 3),
    ("parzen", 3),
    ("rectangle", 3),
    ("triangle", 3),
    ("triweight", 4),
];

/// A Stata error the line raises, or a line that is not translated.
enum Stop {
    Stata(String, u32),
    No(Untranslated),
}

impl From<Untranslated> for Stop {
    fn from(u: Untranslated) -> Self {
        Stop::No(u)
    }
}

fn stata(msg: impl Into<String>, rc: u32) -> Stop {
    Stop::Stata(msg.into(), rc)
}

/// `bwidth(# #, copy)` and the like: the numbers, or Stata's error.
fn bandwidths(option: &str, spec: &str, dim: usize) -> Result<Vec<f64>, Stop> {
    let (list, sub) = spec.split_once(',').unwrap_or((spec, ""));
    let copy = sub.split_whitespace().any(|w| w == "copy");
    let words: Vec<&str> = list.split_whitespace().collect();
    let numbers: Vec<Option<f64>> = words.iter().map(|w| w.parse::<f64>().ok()).collect();
    if words.is_empty() || !numbers.iter().all(Option::is_some) {
        return Err(Stop::No(no(format!(
            "npregress, {option}() with a matrix or names is not translated (numbers with copy are)"
        ))));
    }
    if !copy {
        return Err(stata(
            format!(
                "option {option}() specified incorrectly; list of numbers requires copy option"
            ),
            198,
        ));
    }
    let values: Vec<f64> = numbers.into_iter().flatten().collect();
    if values.len() != dim {
        return Err(stata(
            format!("option {option}() specified incorrectly; matrix must be dimension {dim}"),
            503,
        ));
    }
    if values.iter().any(|v| !v.is_finite() || *v <= 0.0) {
        return Err(stata(
            format!(
                "{option}() specified incorrectly\n    The bandwidth vector should be of dimension {dim} and have no negative values or zero values for continuous covariates."
            ),
            198,
        ));
    }
    Ok(values)
}

impl Session {
    /// Whether the active results are npregress's.
    pub(super) fn npregress_active(&self) -> bool {
        matches!(&self.e_ret, Returned::Program { cmd, .. } if cmd == "npregress")
    }

    /// `npregress kernel depvar indepvar [if] [in] [, options]`, or a replay.
    pub(super) fn npregress(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "npregress")?;
        let main = c.parts.main.trim();
        if main.is_empty() && opts.0.is_empty() {
            return match &self.e_ret {
                Returned::Lost(why) if !self.npregress_active() => Err(no(format!(
                    "npregress alone replays the last fit, which is not known here: {why}"
                ))),
                _ => {
                    self.helpers.insert("stata_npregress");
                    self.helpers.insert("stata_fmt");
                    out.line("stata_np_replay()");
                    self.e_kept = true;
                    Ok(())
                }
            };
        }
        match self.npregress_fit(c, opts, out) {
            Ok(()) => Ok(()),
            Err(Stop::Stata(m, rc)) => {
                out.line(format!(
                    "stop({}, call. = FALSE)",
                    r_text(&format!("{m} (Stata: r({rc}))"))
                ));
                self.e_kept = true;
                Ok(())
            }
            Err(Stop::No(u)) => Err(u),
        }
    }

    #[allow(clippy::too_many_lines)]
    fn npregress_fit(&mut self, c: &Command, opts: &Opts, out: &mut Out) -> Result<(), Stop> {
        let (sub, rest) = first_word(&c.parts.main);
        if sub != "kernel" {
            return Err(Stop::No(no(format!(
                "npregress {sub} is not translated (npregress kernel is)"
            ))));
        }
        if c.parts.weight.is_some() {
            return Err(Stop::No(no("npregress with weights is not translated")));
        }
        let names: Vec<&str> = rest.split_whitespace().collect();
        let (y, x) =
            match names.as_slice() {
                [] => return Err(stata("varlist required", 100)),
                [_] => return Err(stata("You must specify at least one covariate", 198)),
                [y, x] if is_name(y) && is_name(x) => (*y, *x),
                _ => return Err(Stop::No(no(
                    "npregress kernel is translated with one continuous covariate (plain names)",
                ))),
            };
        const KNOWN: &[(&str, usize)] = &[
            ("estimator", 3),
            ("kernel", 3),
            ("predict", 4),
            ("noderivatives", 5),
            ("nolog", 5),
            ("iterate", 4),
            ("bwidth", 2),
            ("meanbwidth", 5),
            ("derivbwidth", 6),
            ("from", 4),
            ("unidentsample", 6),
        ];
        if let Some(o) = opts.other(KNOWN) {
            return Err(Stop::No(no(format!(
                "npregress with the option {o} is not translated yet"
            ))));
        }
        // local-constant with derivatives: Stata notes it and fits local-linear (in R)
        let constant = match opts.arg("estimator", 3).as_deref().map(str::trim) {
            None | Some("linear") => false,
            Some("constant") => true,
            Some(_) => {
                return Err(stata(
                    "invalid estimator() \n    estimator() should be constant or linear",
                    198,
                ))
            }
        };
        let kernel = match opts.arg("kernel", 3) {
            None => "epanechnikov",
            Some(a) => {
                let k = a.trim().to_lowercase();
                if k == "epan2" {
                    "epan2"
                } else {
                    KERNELS
                        .iter()
                        .find(|(n, m)| k.len() >= *m && n.starts_with(k.as_str()))
                        .map(|(n, _)| *n)
                        .ok_or_else(|| stata(format!("option {} not allowed", a.trim()), 198))?
                }
            }
        };
        let (mut pnames, mut preplace, mut pnoderiv) = (None, false, false);
        if let Some(a) = opts.arg("predict", 4) {
            let (list, sub) = a.split_once(',').unwrap_or((a.as_str(), ""));
            preplace = sub.split_whitespace().any(|w| w == "replace");
            pnoderiv = sub.split_whitespace().any(|w| w.starts_with("noder"));
            pnames = Some(
                list.split_whitespace()
                    .map(str::to_string)
                    .collect::<Vec<_>>(),
            );
        }
        let noderiv = opts.has("noderivatives", 5) || pnoderiv;
        let maxit = match opts.arg("iterate", 4) {
            None => 300,
            Some(a) => a
                .trim()
                .parse::<u32>()
                .map_err(|_| stata("option iterate() incorrectly specified", 198))?,
        };
        let (mut gm, mut ge) = (None, None);
        if let Some(a) = opts.arg("bwidth", 2) {
            let v = bandwidths("bwidth", &a, if noderiv { 1 } else { 2 })?;
            gm = v.first().copied();
            ge = v.get(1).copied();
        }
        if let Some(a) = opts.arg("meanbwidth", 5) {
            gm = bandwidths("meanbwidth", &a, 1)?.first().copied();
        }
        if let Some(a) = opts.arg("derivbwidth", 6) {
            ge = bandwidths("derivbwidth", &a, 1)?.first().copied();
        }
        let from = match opts.arg("from", 4) {
            None => None,
            Some(a) => {
                let (list, sub) = a.split_once(',').unwrap_or((a.as_str(), ""));
                if !sub.split_whitespace().any(|w| w == "copy") {
                    return Err(stata(
                        "from() specified incorrectly; list of numbers requires copy option",
                        198,
                    ));
                }
                Some(
                    list.trim()
                        .parse::<f64>()
                        .map_err(|_| stata("from() specified incorrectly", 198))?,
                )
            }
        };
        let want = if noderiv { 1 } else { 2 };
        let (mvar, dvar, replace) = match &pnames {
            Some(list) => {
                let list: Vec<String> = match list.as_slice() {
                    [stub] if stub.ends_with('*') => {
                        let s = stub.trim_end_matches('*');
                        (1..=want).map(|j| format!("{s}{j}")).collect()
                    }
                    _ => list.clone(),
                };
                if list.len() < want {
                    return Err(stata(
                        "too few variables specified with predict()\n    There should be a variable name for the mean function and for the derivatives of the mean function.",
                        102,
                    ));
                }
                if list.len() > want {
                    return Err(stata("too many variables specified with predict()", 103));
                }
                if let Some(bad) = list.iter().find(|n| !is_name(n)) {
                    return Err(Stop::No(no(format!("{bad}: not a plain variable name"))));
                }
                (
                    list.first().cloned().unwrap_or_default(),
                    list.get(1).cloned(),
                    preplace,
                )
            }
            None => (
                format!("_Mean_{y}"),
                (!noderiv).then(|| format!("_d_Mean_{y}_d{x}")),
                true,
            ),
        };
        let unident = opts.arg("unidentsample", 6).map(|a| a.trim().to_string());
        if let Some(u) = &unident {
            if !is_name(u) {
                return Err(Stop::No(no(format!("{u}: not a plain variable name"))));
            }
        }
        let d = self.data()?;
        let sel = match self.condition(c, out)? {
            Some(cond) => {
                out.package("dplyr");
                format!("local({{ s <- dplyr::transmute({d}, s = {cond})$s; !is.na(s) & s }})")
            }
            None => format!("rep(TRUE, nrow({d}))"),
        };
        for n in std::iter::once(&mvar).chain(dvar.iter()) {
            if !replace {
                out.line(format!(
                    "if ({} %in% names({d})) stop(\"option predict() specified incorrectly; variable {n} already defined (Stata: r(110))\", call. = FALSE)",
                    r_text(n)
                ));
            }
        }
        if let Some(u) = &unident {
            out.line(format!(
                "if ({} %in% names({d})) stop(\"variable {u} already defined (Stata: r(110))\", call. = FALSE)",
                r_text(u)
            ));
        }
        for h in ["stata_npregress", "stata_fmt", "stata_returned"] {
            self.helpers.insert(h);
        }
        let num = |v: Option<f64>| v.map_or("NULL".to_string(), r_number);
        let after = super::returned::after_name(c);
        let after = after.trim();
        let tail = after.strip_prefix("kernel").map(str::trim).unwrap_or(after);
        let cmdline = match tail.split_once(',') {
            Some((head, opts_text)) => {
                format!(
                    "npregress kernel {}, {} ",
                    head.trim_end(),
                    opts_text.trim()
                )
            }
            None => format!("npregress kernel {tail},  "),
        };
        out.line(format!(
            "{d} <- stata_npregress({d}, {sel}, {}, {}, k = {}, constant = {}, noderiv = {}, nolog = {}, maxit = {maxit}, gm = {}, ge = {}, bothgiven = {}, from = {}, mvar = {}, dvar = {}, unident = {}, dname = {}, cmdline = {}, show = {})",
            r_text(y),
            r_text(x),
            r_text(kernel),
            super::stats::r_logical(constant),
            super::stats::r_logical(noderiv),
            super::stats::r_logical(opts.has("nolog", 5)),
            num(gm),
            num(ge),
            super::stats::r_logical(opts.has("bwidth", 2)),
            num(from),
            r_text(&mvar),
            dvar.as_deref().map_or("NULL".to_string(), r_text),
            unident.as_deref().map_or("NULL".to_string(), r_text),
            r_text(&d),
            r_text(&cmdline),
            super::stats::r_logical(!self.quiet(c)),
        ));
        for n in std::iter::once(&mvar).chain(dvar.iter()) {
            self.kinds.insert(n.clone(), Kind::Number);
            self.storage.insert(n.clone(), Storage::Double);
        }
        let flag = unident.unwrap_or_else(|| "_unident_sample".into());
        self.kinds.insert(flag.clone(), Kind::Number);
        self.storage.insert(flag, Storage::Byte);
        self.esample_new = true;
        self.e_new = true;
        self.estimates = None;
        self.estimates_lost = Some(
            "predict, margins and the other post-estimation commands are not translated after npregress yet".into(),
        );
        self.e_text.clear();
        self.fit_ts = None;
        self.e_ret = Returned::Program {
            cmd: "npregress".into(),
            texts: NP_TEXTS.iter().map(|s| s.to_string()).collect(),
        };
        out.caveat(
            "npregress's bandwidth search as StatCraft decoded Stata 19's; Stata's e(datasignature) is not kept",
        );
        Ok(())
    }
}

/// The e() macros npregress leaves, read back as text.
const NP_TEXTS: &[&str] = &[
    "cmdline",
    "vce",
    "predict",
    "estat_cmd",
    "title",
    "estimator",
    "marginsprop",
    "marginsok",
    "bselector",
    "cmd",
    "kname",
    "datasignaturevars",
    "properties",
    "depvar",
];

#[cfg(test)]
mod tests {
    use crate::data_r::translate;
    use crate::reader::read;

    fn r(source: &str) -> Vec<String> {
        translate(&read(source))
            .lines
            .into_iter()
            .map(|l| match l.result {
                Ok(r) => r.code,
                Err(e) => format!("NOT: {}", e.reason),
            })
            .collect()
    }

    #[test]
    fn npregress_kernel_fits_and_keeps_its_variables() {
        let out = r("use d
npregress kernel y x, kernel(gaussian) bwidth(0.5 0.8, copy) predict(m dd)
npregress
npregress kernel y x if x > 1, estimator(constant) noderivatives nolog
summarize m dd");
        assert!(
            out[1].contains("stata_npregress(")
                && out[1].contains("k = \"gaussian\"")
                && out[1].contains("gm = 0.5, ge = 0.8, bothgiven = TRUE")
                && out[1].contains("mvar = \"m\", dvar = \"dd\"")
                && out[1].contains("cmdline = \"npregress kernel y x, kernel(gaussian) bwidth(0.5 0.8, copy) predict(m dd) \""),
            "{out:?}"
        );
        assert!(out[2].contains("stata_np_replay()"), "{out:?}");
        assert!(
            out[3].contains("constant = TRUE, noderiv = TRUE, nolog = TRUE")
                && out[3].contains("dvar = NULL"),
            "{out:?}"
        );
        assert!(!out[4].starts_with("NOT:"), "{out:?}");
    }

    #[test]
    fn statas_errors_are_kept_and_the_rest_refused() {
        let out = r("use d
npregress kernel y
npregress kernel y x, kernel(nonsense)
npregress kernel y x, estimator(cubic)
npregress kernel y x, bwidth(0.5)
npregress kernel y x, bwidth(0.5 0.8 0.9, copy)
npregress kernel y x, predict(m)
npregress kernel y x z
npregress series y x
npregress kernel y x, vce(bootstrap)");
        assert!(
            out[1].contains("at least one covariate (Stata: r(198))"),
            "{out:?}"
        );
        assert!(
            out[2].contains("option nonsense not allowed (Stata: r(198))"),
            "{out:?}"
        );
        assert!(
            out[3].contains("estimator() should be constant or linear"),
            "{out:?}"
        );
        assert!(
            out[4].contains("requires copy option (Stata: r(198))"),
            "{out:?}"
        );
        assert!(
            out[5].contains("matrix must be dimension 2 (Stata: r(503))"),
            "{out:?}"
        );
        assert!(out[6].contains("(Stata: r(102))"), "{out:?}");
        assert!(
            out[7].starts_with("NOT: npregress kernel is translated with one"),
            "{out:?}"
        );
        assert!(out[8].starts_with("NOT: npregress series"), "{out:?}");
        assert!(
            out[9].starts_with("NOT: npregress with the option vce"),
            "{out:?}"
        );
    }
}
