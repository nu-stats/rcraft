//! `lpoly`: kernel-weighted local polynomial smoothing, as Stata 19 does it.
//! The method is StatCraft's decoding of Stata's (its `plan/stata/lpoly.md`,
//! probes 552 and 553), our own work, written here in R:
//!
//! - at each grid point a weighted least-squares fit of degree `degree()` in
//!   (x - x0) with weights K((x - x0) / h) / h times the observation's weight;
//!   the smooth is the intercept, on min(N, 50) evenly spaced points unless
//!   `n()` or `at()` say otherwise;
//! - the default bandwidth, Stata's rule of thumb (a global polynomial of
//!   degree p + 3, its (p + 1)th derivative integrated over the middle 90% of
//!   the range, Fan and Gijbels' constant by the trapezoid rule);
//! - the standard errors from the degree p + 2 fit at the pilot bandwidth;
//! - `generate()`, `at()`, `se()`, and r().
//!
//! The kernels and the elimination come from `npregress.rs`'s helper.

use super::*;

pub(super) const LPOLY_HELPER: &str = r##"# Fan and Gijbels' constant C(p, K), integrated as Stata does: the trapezoid rule on 101 points over
# +-sqrt(5) (Epanechnikov), +-5 (Gaussian) or +-1.25 (the rest), for the equivalent kernel of degree p
stata_lp_const <- function(k, p) {
  l <- switch(k, epanechnikov = sqrt(5), gaussian = 5, 1.25)
  m <- 100; i <- 0:m
  u <- l * (2 * i - m) / m
  wt <- rep(2 * l / m, m + 1); wt[c(1, m + 1)] <- wt[1] / 2
  kv <- stata_np_kernel(u, k)
  S <- matrix(0, p + 1, p + 1)
  for (a in 0:p) for (b in 0:p) S[a + 1, b + 1] <- sum(wt * u^(a + b) * kv)
  cf <- stata_np_solve(S, c(1, rep(0, p)))
  if (is.null(cf)) return(NA_real_)
  star <- kv * as.vector(outer(u, 0:p, "^") %*% cf)
  r <- sum(wt * star^2); mu <- sum(wt * u^(p + 1) * star)
  (factorial(p + 1)^2 * r / (2 * (p + 1) * mu * mu))^(1 / (2 * p + 3))
}
# the rule-of-thumb bandwidth for degree p (an even degree uses the next odd one)
stata_lp_rot <- function(x, y, w, total, k, p) {
  p <- if (p %% 2 == 0) p + 1 else p
  deg <- p + 3
  lo_x <- min(x); hi_x <- max(x); rng <- hi_x - lo_x
  cen <- (hi_x + lo_x) / 2; s <- rng / 2
  if (!(s > 0)) return(NA_real_)
  # the global polynomial in u = (x - centre) / half range: the same fit, well conditioned
  u <- (x - cen) / s
  X <- outer(u, 0:deg, "^"); sw <- sqrt(w)
  qrf <- qr(X * sw, tol = 1e-12)
  if (qrf$rank < deg + 1) return(NA_real_)
  b <- qr.coef(qrf, y * sw)
  s2 <- sum(w * (y - as.vector(X %*% b))^2) / total
  js <- (p + 1):deg
  der <- b[js + 1] * factorial(js) / factorial(js - p - 1)
  sq <- numeric(2 * length(der) - 1)
  for (i in seq_along(der)) for (j in seq_along(der)) sq[i + j - 1] <- sq[i + j - 1] + der[i] * der[j]
  prim <- function(t) sum(sq * t^seq_along(sq) / seq_along(sq))
  integral <- (prim(0.9) - prim(-0.9)) * s^(1 - 2 * (p + 1))
  h <- stata_lp_const(k, p) * (s2 * 0.9 * 2 * s / (total * integral))^(1 / (2 * p + 3))
  if (is.finite(h) && h > 0) h else NA_real_
}
# a local fit needs two points; a column that is collinear with the ones before it (fewer distinct points than
# coefficients) is left out, as Stata's regress leaves it, so two points give the line through them
stata_lp_local <- function(x, y, w, x0, h, k, p) {
  kw <- w * stata_np_kernel((x - x0) / h, k) / h
  nz <- which(kw > 0)
  if (length(nz) < 2) return(NULL)
  X <- outer((x - x0)[nz], 0:p, "^")
  A <- crossprod(X, kw[nz] * X); b <- as.vector(crossprod(X, kw[nz] * y[nz]))
  # the columns kept are picked by the largest remaining pivot, as Stata does: three points and a cubic
  # leave out the slope, not the cube
  idx <- integer(); rest <- seq_len(p + 1)
  repeat {
    best <- 0L; bp <- -Inf
    for (j in rest) {
      piv <- if (length(idx)) A[j, j] - sum(A[j, idx] * solve(A[idx, idx, drop = FALSE], A[idx, j])) else A[j, j]
      if (is.finite(piv) && piv > 1e-11 * A[j, j] && piv > bp) { bp <- piv; best <- j }
    }
    if (best == 0L) break
    idx <- c(idx, best); rest <- setdiff(rest, best)
    if (!length(rest)) break
  }
  idx <- sort(idx)
  bs <- stata_np_solve(A[idx, idx, drop = FALSE], b[idx])
  if (is.null(bs)) return(NULL)
  beta <- numeric(p + 1); beta[idx] <- bs
  list(beta = beta, A = A, kw = kw, full = length(idx) == p + 1)
}
stata_lp_inv <- function(a) {
  n <- nrow(a)
  cols <- lapply(seq_len(n), function(j) { e <- numeric(n); e[j] <- 1; stata_np_solve(a, e) })
  if (any(vapply(cols, is.null, TRUE))) NULL else do.call(cbind, cols)
}
# the sandwich (X'WX)^-1 (X'W^2 X) (X'WX)^-1 times sigma^2(x0): var(), else the weighted residual sum
# of squares of the degree p + 2 fit at the pilot bandwidth over tr(W - W X (X'WX)^-1 X'W)
stata_lp_se <- function(x, y, w, x0, fit, pilot, var, k, p) {
  if (!fit$full) return(NA_real_)
  ai <- stata_lp_inv(fit$A)
  if (is.null(ai)) return(NA_real_)
  nz <- which(fit$kw != 0)
  X <- outer((x - x0)[nz], 0:p, "^")
  B <- crossprod(X, (fit$kw[nz]^2 / w[nz]) * X)
  f <- ai[1, ]
  v00 <- sum(f * (B %*% f))
  if (!is.null(var)) s2 <- var
  else {
    q <- p + 2
    if (!is.finite(pilot) || pilot <= 0) return(NA_real_)
    if (sum(w * stata_np_kernel((x - x0) / pilot, k) / pilot > 0) <= q + 1) return(NA_real_)
    fit2 <- stata_lp_local(x, y, w, x0, pilot, k, q)
    if (is.null(fit2) || !fit2$full) return(NA_real_)
    ai2 <- stata_lp_inv(fit2$A)
    if (is.null(ai2)) return(NA_real_)
    nz2 <- which(fit2$kw != 0)
    X2 <- outer((x - x0)[nz2], 0:q, "^"); k2 <- fit2$kw[nz2]
    rss <- sum(k2 * (y[nz2] - as.vector(X2 %*% fit2$beta))^2)
    tr <- sum(k2^2 * rowSums((X2 %*% ai2) * X2))
    den <- sum(fit2$kw) - tr
    if (!(den > 0)) return(NA_real_)
    s2 <- rss / den
  }
  v <- v00 * s2
  if (is.finite(v) && v >= 0) sqrt(v) else NA_real_
}
# lpoly y x: the smooth on a grid or at the rows of at(), generate(), se(), and r()
stata_lpoly <- function(d, sel, w = NULL, wtype = "", yname, xname, k = "epanechnikov", degree = 0, bw = NULL, n = NULL,
                        at = NULL, gen = character(), se = NULL, wantse = FALSE, pwidth = NULL, var = NULL, level = 95) {
  for (v in c(yname, xname)) if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
  if (is.character(d[[yname]]) || is.character(d[[xname]])) stop("type mismatch (Stata: r(109))", call. = FALSE)
  num <- function(v) { u <- unclass(d[[v]]); attributes(u) <- NULL; as.numeric(u) }
  if (length(gen) == 1 && is.null(at)) stop("at() must be specified for generate() to work with one variable (Stata: r(198))", call. = FALSE)
  if (!is.null(se) && is.null(at) && !length(gen)) stop("se() requires the specification of at() or generate() (Stata: r(198))", call. = FALSE)
  for (nm in c(gen, se)) if (nm %in% names(d)) stop("variable ", nm, " already defined (Stata: r(110))", call. = FALSE)
  xv <- num(xname); yv <- num(yname)
  ok <- sel & !is.na(xv) & !is.na(yv)
  if (!is.null(w)) {
    w <- as.numeric(w)
    ok <- ok & !is.na(w)
    if (any(w[ok] < 0)) stop("negative weights encountered (Stata: r(402))", call. = FALSE)
    if (wtype == "fweight" && any(w[ok] != round(w[ok]))) stop("may not use noninteger frequency weights (Stata: r(401))", call. = FALSE)
  }
  rows <- which(ok)
  if (!length(rows)) stop("no observations (Stata: r(2000))", call. = FALSE)
  x <- xv[rows]; y <- yv[rows]
  if (is.null(w)) { wv <- rep(1, length(rows)); total <- length(rows) }
  else if (wtype == "fweight") { wv <- w[rows]; total <- sum(wv) }
  else { wv <- w[rows] / sum(w[rows]) * length(rows); total <- length(rows) }
  rot <- stata_lp_rot(x, y, wv, total, k, degree)
  h <- if (is.null(bw)) rot else bw
  usable <- function(v) is.finite(v) && v > 0
  nobs <- nrow(d)
  if (!is.null(at)) {
    if (!at %in% names(d)) stop("variable ", at, " not found
(error in option at()) (Stata: r(111))", call. = FALSE)
    grid <- num(at)
  } else {
    npts <- if (is.null(n) || n == 1 || n != round(n)) min(length(rows), 50) else n
    if (npts > length(rows)) { npts <- length(rows); cat("(n() set to ", npts, ")\n", sep = "") }
    lo <- min(x); hi <- max(x)
    grid <- if (npts == 1) lo else { st <- (hi - lo) / (npts - 1); g <- lo + st * (0:(npts - 1)); g[npts] <- hi; g }
  }
  fits <- lapply(grid, function(g) if (is.na(g) || !usable(h)) NULL else stata_lp_local(x, y, wv, g, h, k, degree))
  smooth <- vapply(fits, function(f) if (is.null(f)) NA_real_ else f$beta[1], 0)
  pilot <- if (!is.null(pwidth)) pwidth else if (usable(rot)) 1.5 * rot else NA_real_
  ses <- if (wantse) vapply(seq_along(grid), function(i) {
    f <- fits[[i]]
    if (is.null(f) || !(usable(pilot) || !is.null(var))) NA_real_ else stata_lp_se(x, y, wv, grid[i], f, pilot, var, k, degree)
  }, 0) else numeric(0)
  column <- function(v, label) {
    cc <- rep(NA_real_, nobs); m <- min(length(v), nobs); cc[seq_len(m)] <- v[seq_len(m)]
    structure(cc, label = label, format.stata = "%10.0g")
  }
  slabel <- paste0("lpoly smooth: ", yname)
  if (!is.null(at)) {
    if (length(gen) == 1) d[[gen[1]]] <- structure(smooth, label = slabel, format.stata = "%10.0g")
    if (length(gen) == 2) {
      d[[gen[1]]] <- structure(grid, label = "lpoly smoothing grid", format.stata = "%10.0g")
      d[[gen[2]]] <- structure(smooth, label = slabel, format.stata = "%10.0g")
    }
    if (!is.null(se)) d[[se]] <- structure(ses, label = "lpoly SEs", format.stata = "%10.0g")
  } else {
    if (length(gen) == 2) {
      d[[gen[1]]] <- column(grid, "lpoly smoothing grid")
      d[[gen[2]]] <- column(smooth, slabel)
    }
    if (!is.null(se)) d[[se]] <- column(ses, "lpoly SEs")
  }
  ngrid <- if (!is.null(at)) sum(!is.na(grid)) else length(grid)
  r <- list(degree = degree, ngrid = ngrid, N = total, bwidth = h)
  if (wantse) r$pwidth <- pilot
  r$kernel <- k
  stata_r <<- r
  z <- stats::qnorm((100 + level) / 200)
  stata_lp <<- data.frame(x = grid, s = smooth, lo = if (wantse) smooth - z * ses else NA_real_, hi = if (wantse) smooth + z * ses else NA_real_)
  stata_lp_pts <<- data.frame(x = x, y = y)
  attr(stata_lp, "bwidth") <<- h
  d
}
"##;

/// The kernels `kernel()` takes, with their shortest abbreviations.
const KERNELS: &[(&str, usize)] = &[
    ("epanechnikov", 2),
    ("biweight", 2),
    ("cosine", 3),
    ("gaussian", 3),
    ("parzen", 3),
    ("rectangle", 3),
    ("triangle", 3),
];

/// The r() names `lpoly` leaves, without and with standard errors.
const R_LPOLY: &[&str] = &["degree", "ngrid", "N", "bwidth", "kernel"];
const R_LPOLY_SE: &[&str] = &["degree", "ngrid", "N", "bwidth", "pwidth", "kernel"];

impl Session {
    /// `lpoly yvar xvar [if] [in] [weight] [, kernel() bwidth(#) degree(#)
    /// generate([newx] newsmooth) n(#) at(var) se(newvar) pwidth(#) var(#)
    /// nograph noscatter ci level(#)]`.
    #[allow(clippy::too_many_lines)]
    pub(super) fn lpoly(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "lpoly")?;
        // Stata's errors come as R's, so that capture sees them
        let stata = |out: &mut Out, msg: &str, rc: u32| {
            out.line(format!(
                "stop({}, call. = FALSE)",
                r_text(&format!("{msg} (Stata: r({rc}))"))
            ));
        };
        let names: Vec<&str> = c.parts.main.split_whitespace().collect();
        let (y, x) = match names.as_slice() {
            [y, x] if is_name(y) && is_name(x) => (*y, *x),
            [] | [_] => {
                stata(out, "too few variables specified", 102);
                return Ok(());
            }
            [_, _] => {
                return Err(no(
                    "lpoly with names that are not plain variables is not translated",
                ))
            }
            _ => {
                stata(out, "too many variables specified", 103);
                return Ok(());
            }
        };
        let own = [
            ("kernel", 1),
            ("bwidth", 2),
            ("degree", 3),
            ("generate", 3),
            ("n", 1),
            ("at", 2),
            ("se", 2),
            ("pwidth", 2),
            ("var", 3),
            ("nograph", 3),
            ("noscatter", 3),
            ("ci", 2),
            ("level", 3),
        ];
        let graph = !opts.has("nograph", 3);
        if !graph {
            if let Some(o) = opts.other(&own) {
                return Err(no(format!(
                    "lpoly with the option {o} is not translated yet"
                )));
            }
        }
        let num = |name: &str, min: usize| -> Result<Option<f64>, Untranslated> {
            match opts.arg(name, min) {
                None => Ok(None),
                Some(a) => a
                    .trim()
                    .parse::<f64>()
                    .map(Some)
                    .map_err(|_| no(format!("lpoly, {name}({}) needs a number", a.trim()))),
            }
        };
        let kernel = match opts.arg("kernel", 1) {
            None => "epanechnikov".to_string(),
            Some(a) => {
                // the name as typed: Stata does not read Gaussian for gaussian
                let k = a.trim().to_string();
                if k == "epan2" {
                    "epan2".to_string()
                } else {
                    match KERNELS
                        .iter()
                        .find(|(n, m)| k.len() >= *m && n.starts_with(k.as_str()))
                    {
                        Some((n, _)) => (*n).to_string(),
                        None => {
                            stata(out, "invalid kernel function", 198);
                            return Ok(());
                        }
                    }
                }
            }
        };
        // a degree that is not a whole number is read as 0 (Stata 19); above 5 was not observed
        let degree = match num("degree", 3)? {
            None => 0.0,
            Some(d) if d < 0.0 => {
                stata(out, "degree() must be a nonnegative number", 198);
                return Ok(());
            }
            Some(d) if d.fract() != 0.0 => 0.0,
            Some(d) if d <= 5.0 => d,
            Some(_) => return Err(no("lpoly, degree() above 5 is not translated")),
        };
        let bw = num("bwidth", 2)?;
        let n = num("n", 1)?;
        let pwidth = num("pwidth", 2)?;
        let var = num("var", 3)?;
        let level = match num("level", 3)? {
            None => 95.0,
            Some(l) if (10.0..=99.99).contains(&l) => l,
            Some(_) => return Err(no("lpoly, level() outside 10 to 99.99 is not translated")),
        };
        if n.is_some_and(|v| v <= 0.0) {
            stata(out, "n() must be a positive number", 198);
            return Ok(());
        }
        for (v, what) in [(bw, "bwidth"), (pwidth, "pwidth")] {
            if v.is_some_and(|b| !(b > 0.0)) {
                stata(out, &format!("{what}() must be a positive number"), 198);
                return Ok(());
            }
        }
        if var.is_some_and(|v| v < 0.0) {
            return Err(no("lpoly, a negative var() is not translated"));
        }
        let at = opts.arg("at", 2).map(|a| a.trim().to_string());
        if let Some(a) = &at {
            if !is_name(a) {
                return Err(no(format!("lpoly, at({a}) needs a variable")));
            }
        }
        let gen: Vec<String> = opts
            .arg("generate", 3)
            .map(|g| g.split_whitespace().map(str::to_string).collect())
            .unwrap_or_default();
        if gen.len() > 2 {
            stata(out, "generate(): too many variables specified", 198);
            return Ok(());
        }
        let se = opts.arg("se", 2).map(|a| a.trim().to_string());
        for nm in gen.iter().chain(se.iter()) {
            if !is_name(nm) {
                return Err(no(format!("lpoly: {nm} is not a plain variable name")));
            }
        }
        let wtype = match &c.parts.weight {
            None => None,
            Some(w) => match w.kind {
                crate::lang::syntax::WeightKind::Frequency => Some("fweight"),
                crate::lang::syntax::WeightKind::Analytic
                | crate::lang::syntax::WeightKind::Unspecified => Some("aweight"),
                crate::lang::syntax::WeightKind::Importance => {
                    stata(out, "iweights not allowed", 101);
                    return Ok(());
                }
                crate::lang::syntax::WeightKind::Sampling => {
                    stata(out, "pweights not allowed", 101);
                    return Ok(());
                }
            },
        };
        let wantse = se.is_some() || opts.has("ci", 2);
        let d = self.data()?;
        let sel = match self.condition(c, out)? {
            Some(cond) => {
                out.package("dplyr");
                format!("local({{ s <- dplyr::transmute({d}, s = {cond})$s; !is.na(s) & s }})")
            }
            None => format!("rep(TRUE, nrow({d}))"),
        };
        let w = match (&c.parts.weight, wtype) {
            (Some(w), Some(t)) => {
                out.package("dplyr");
                format!(
                    ", w = dplyr::pull(dplyr::mutate({d}, stata_rows = {}), stata_rows), wtype = {}",
                    self.expr(&w.exp, Use::Value, out)?,
                    r_text(t)
                )
            }
            _ => String::new(),
        };
        let numr = |v: Option<f64>| v.map_or("NULL".to_string(), r_number);
        let strs = |v: &[String]| {
            if v.is_empty() {
                "character()".to_string()
            } else {
                format!(
                    "c({})",
                    v.iter().map(|s| r_text(s)).collect::<Vec<_>>().join(", ")
                )
            }
        };
        for h in [
            "stata_lpoly",
            "stata_npregress",
            "stata_fmt",
            "stata_returned",
        ] {
            self.helpers.insert(h);
        }
        out.line(format!(
            "{d} <- stata_lpoly({d}, {sel}{w}, yname = {}, xname = {}, k = {}, degree = {}, bw = {}, n = {}, at = {}, gen = {}, se = {}, wantse = {}, pwidth = {}, var = {}, level = {})",
            r_text(y),
            r_text(x),
            r_text(&kernel),
            r_number(degree),
            numr(bw),
            numr(n),
            at.as_deref().map_or("NULL".to_string(), r_text),
            strs(&gen),
            se.as_deref().map_or("NULL".to_string(), r_text),
            super::stats::r_logical(wantse),
            numr(pwidth),
            numr(var),
            r_number(level),
        ));
        for nm in gen.iter().chain(se.iter()) {
            self.kinds.insert(nm.clone(), Kind::Number);
            self.storage.insert(nm.clone(), Storage::Double);
        }
        self.r_new = Some((if wantse { R_LPOLY_SE } else { R_LPOLY }, "lpoly".into()));
        if !graph {
            return Ok(());
        }
        // the data under the smooth (noscatter leaves them out), the smooth, and its band (ci)
        let mut layers = Vec::new();
        if !opts.has("noscatter", 3) {
            layers.push(
                "ggplot2::geom_point(data = stata_lp_pts, ggplot2::aes(x = x, y = y))".to_string(),
            );
        }
        if opts.has("ci", 2) {
            layers.push(
                "ggplot2::geom_ribbon(data = stata_lp, ggplot2::aes(x = x, ymin = lo, ymax = hi), alpha = 0.3)"
                    .to_string(),
            );
        }
        layers.push("ggplot2::geom_line(data = stata_lp, ggplot2::aes(x = x, y = s))".to_string());
        layers.push("ggplot2::labs(title = \"Local polynomial smooth\")".to_string());
        let rows = d.clone();
        out.caveat(
            "Stata's local polynomial smooth: the Epanechnikov kernel unless kernel() names another, the degree 0 \
             unless degree(), the rule-of-thumb bandwidth unless bwidth(), at min(N, 50) points",
        );
        self.helpers.insert("stata_graph");
        self.helpers.insert("stata_tab_values");
        self.draw(
            c,
            opts,
            out,
            &own,
            &layers,
            &format!("stata_varlab({rows}, {})", r_text(x)),
            &format!("stata_varlab({rows}, {})", r_text(y)),
        )
    }
}

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
    fn lpoly_translates_its_options_and_keeps_its_variables() {
        let out = r("use d
lpoly y x, nograph degree(1) bwidth(0.8) kernel(gau) generate(gx gs) se(se) n(20)
lpoly y x [aweight=w] if x < 5, nograph generate(g2) at(x)
summarize gx gs se g2");
        assert!(
            out[1].contains("stata_lpoly(")
                && out[1].contains("k = \"gaussian\"")
                && out[1].contains("degree = 1, bw = 0.8, n = 20, at = NULL")
                && out[1].contains("gen = c(\"gx\", \"gs\"), se = \"se\", wantse = TRUE"),
            "{out:?}"
        );
        assert!(
            out[2].contains("wtype = \"aweight\"") && out[2].contains("at = \"x\""),
            "{out:?}"
        );
        assert!(!out[3].starts_with("NOT:"), "{out:?}");
    }

    #[test]
    fn statas_errors_are_kept_and_the_rest_refused() {
        let out = r("use d
lpoly y
lpoly y x, nograph kernel(foo)
lpoly y x [pweight=w], nograph
lpoly y x [iweight=w], nograph
lpoly y x, nograph generate(a b c)
lpoly y x, nograph vce(robust)
lpoly y x, nograph bwidth(-1)");
        assert!(
            out[1].contains("too few variables specified (Stata: r(102))"),
            "{out:?}"
        );
        assert!(
            out[2].contains("invalid kernel function (Stata: r(198))"),
            "{out:?}"
        );
        assert!(
            out[3].contains("pweights not allowed (Stata: r(101))"),
            "{out:?}"
        );
        assert!(
            out[4].contains("iweights not allowed (Stata: r(101))"),
            "{out:?}"
        );
        assert!(
            out[5].contains("too many variables specified (Stata: r(198))"),
            "{out:?}"
        );
        assert!(
            out[6].starts_with("NOT: lpoly with the option vce"),
            "{out:?}"
        );
        assert!(
            out[7].contains("bwidth() must be a positive number (Stata: r(198))"),
            "{out:?}"
        );
    }
}
