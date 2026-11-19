//! P2B.5: Stata's graph commands drawn with ggplot2.
//!
//! `histogram`, `kdensity`, `scatter`, `line`, `connected`, `twoway` (with
//! `scatter`, `line`, `connected`, `lfit`, `qfit`, `lfitci`, `qfitci`,
//! `lowess`, `histogram`, `kdensity`, `bar`, `area`, `rarea`, `rcap` and
//! `rspike` layers, in `( )` or split at `||`), `lowess`, `qnorm`, `pnorm`,
//! `rvfplot`, `graph bar`/`hbar`, `graph box`/`hbox`, `graph pie`, `graph
//! matrix`, `graph export`, `graph save`, `graph combine`, `graph dir`,
//! `graph display` and `graph drop`.
//!
//! What a graph shows is worked out here the way Stata 19 works it out, and
//! only then handed to ggplot2 to draw, since R's own defaults differ at
//! every step (all checked against Stata 19):
//!
//! - `histogram`: int(min(sqrt(N), 10 log10 N)) bins from the minimum, each
//!   closed below (the last also above), heights as densities unless
//!   `fraction`, `frequency` or `percent`; `width()` and `start()` as Stata
//!   reads them; `discrete` one bar per value, the width the smallest gap.
//!   R's `hist()` would choose Sturges' bins and other breaks;
//! - `kdensity`: the Epanechnikov kernel on +-sqrt(5) (R's default is the
//!   Gaussian), the bandwidth 0.9 min(sd, IQR / 1.349) N^(-1/5) with Stata's
//!   quartiles, whatever the kernel, at min(N, 50) points from min - h to
//!   max + h; the other kernels Stata names;
//! - `lowess`: Stata's, a local line with tricube weights over int((0.8 N -
//!   0.5) / 2) neighbours on each side and no robustness steps, which is not
//!   R's `lowess()`;
//! - `lfit` and `qfit`: least squares lines, `lfit` at 3 points and `qfit` at
//!   100 (`n()`, `range()`); `lfitci` and `qfitci` add the band of plus or
//!   minus t times the standard error of the mean prediction (`stdf`: of a
//!   forecast), at `level()`; `area` fills to `base()` or the axis bottom;
//! - `graph bar`: the statistic (mean unless named) of each variable within
//!   each `over()` group; `(count)` alone counts the rows;
//! - `graph box`: Stata's quartiles of each group, whiskers to the adjacent
//!   values (the farthest within 1.5 IQR of the box), the rest as points;
//! - `twoway bar`: bars from `base()` (0) to y, `barwidth()` (1) wide;
//!   `xline()` and `yline()` draw their lines.
//!
//! Options that only change the look (marker symbols, colours, schemes, axis
//! labels, ...) are left to ggplot2's defaults and named in a note; options
//! that would change what is shown are translated or refused.

use super::*;
use crate::lang::syntax::abbrev;

/// The numbers behind the graphs, worked out as Stata does.
pub(super) const GRAPH_HELPER: &str = r#"stata_num <- function(v) {
  u <- unclass(v)
  attributes(u) <- NULL
  as.numeric(u)
}
stata_varlab <- function(d, v) {
  l <- attr(d[[v]], "label", exact = TRUE)
  if (is.null(l) || !nzchar(l)) v else l
}
stata_hist_data <- function(x, name = "x", bin = NULL, width = NULL, start = NULL, discrete = FALSE,
                            type = "density") {
  x <- stata_num(x)
  x <- x[!is.na(x)]
  N <- length(x)
  if (discrete) {
    v <- sort(unique(x))
    w <- if (is.null(width)) { if (length(v) > 1) min(diff(v)) else 1 } else width
    cnt <- as.numeric(table(factor(x, levels = v)))
    mid <- v
  } else {
    lo0 <- if (is.null(start)) min(x) else start
    if (lo0 > min(x)) stop("option start() may not be larger than minimum of ", name)
    if (!is.null(width)) {
      w <- width
      k <- max(1, ceiling((max(x) - lo0) / w))
    } else {
      k <- if (is.null(bin)) floor(min(sqrt(N), 10 * log10(N))) else bin
      w <- (max(x) - lo0) / k
    }
    j <- pmin(floor((x - lo0) / w) + 1, k)
    cnt <- tabulate(j, k)
    mid <- lo0 + (seq_len(k) - 0.5) * w
  }
  h <- switch(type, density = cnt / (N * w), fraction = cnt / N, frequency = cnt, percent = 100 * cnt / N)
  data.frame(mid = mid, height = h, count = cnt, width = w)
}
stata_kernel <- function(z, k) {
  a <- abs(z)
  switch(k,
    epanechnikov = ifelse(a < sqrt(5), 3 / (4 * sqrt(5)) * (1 - z^2 / 5), 0),
    epan2 = ifelse(a < 1, 0.75 * (1 - z^2), 0),
    biweight = ifelse(a < 1, 15 / 16 * (1 - z^2)^2, 0),
    cosine = ifelse(a < 0.5, 1 + cos(2 * pi * z), 0),
    gaussian = stats::dnorm(z),
    parzen = ifelse(a <= 0.5, 4 / 3 - 8 * z^2 + 8 * a^3, ifelse(a <= 1, 8 * (1 - a)^3 / 3, 0)),
    rectangle = ifelse(a < 1, 0.5, 0),
    triangle = ifelse(a < 1, 1 - a, 0))
}
stata_kdensity <- function(x, kernel = "epanechnikov", bw = NULL, n = NULL, at = NULL) {
  x <- stata_num(x)
  x <- x[!is.na(x)]
  N <- length(x)
  if (is.null(bw)) {
    q <- unname(stats::quantile(x, c(0.25, 0.75), type = 2))
    bw <- 0.9 * min(stats::sd(x), (q[2] - q[1]) / 1.349) * N^(-1 / 5)
  }
  g <- if (!is.null(at)) stata_num(at) else seq(min(x) - bw, max(x) + bw, length.out = if (is.null(n)) min(N, 50) else n)
  d <- vapply(g, function(t) if (is.na(t)) NA_real_ else mean(stata_kernel((t - x) / bw, kernel)) / bw, 0)
  out <- data.frame(x = g, density = d)
  attr(out, "bwidth") <- bw
  out
}
stata_lowess <- function(x, y, bw = 0.8) {
  x <- stata_num(x)
  y <- stata_num(y)
  ok <- !is.na(x) & !is.na(y)
  out <- rep(NA_real_, length(x))
  xs <- x[ok]
  ys <- y[ok]
  o <- order(xs)
  xs <- xs[o]
  ys <- ys[o]
  N <- length(xs)
  k <- floor((N * bw - 0.5) / 2)
  s <- numeric(N)
  for (i in seq_len(N)) {
    lo <- max(1, i - k)
    hi <- min(N, i + k)
    idx <- lo:hi
    D <- 1.0001 * max(xs[hi] - xs[i], xs[i] - xs[lo])
    w <- (1 - (abs(xs[idx] - xs[i]) / D)^3)^3
    f <- stats::lm.wfit(cbind(1, xs[idx]), ys[idx], w)
    s[i] <- sum(f$coefficients * c(1, xs[i]))
  }
  r <- numeric(N)
  r[o] <- s
  out[ok] <- r
  out
}
stata_fitline <- function(d, y, x, degree = 1, n = 100, from = NA, to = NA, level = NA, stdf = FALSE) {
  xv <- stata_num(d[[x]])
  yv <- stata_num(d[[y]])
  ok <- !is.na(xv) & !is.na(yv)
  # range(a b): the line's ends, `.` (NA) the data's own
  if (is.na(from)) from <- min(xv[ok])
  if (is.na(to)) to <- max(xv[ok])
  g <- seq(from, to, length.out = n)
  X <- if (degree == 1) cbind(1, xv[ok]) else cbind(1, xv[ok], xv[ok]^2)
  XtXi <- solve(crossprod(X))
  b <- XtXi %*% crossprod(X, yv[ok])
  G <- if (degree == 1) cbind(1, g) else cbind(1, g, g^2)
  out <- data.frame(x = g, y = drop(G %*% b))
  # lfitci and qfitci: the fit plus or minus t times the standard error of the
  # mean prediction (stdp), or of a forecast (stdf), as Stata 19 draws them
  if (!is.na(level)) {
    df <- sum(ok) - ncol(X)
    s2 <- sum((yv[ok] - drop(X %*% b))^2) / df
    se <- sqrt(s2 * rowSums((G %*% XtXi) * G) + if (stdf) s2 else 0)
    t <- stats::qt(1 - (1 - level / 100) / 2, df)
    out$lo <- out$y - t * se
    out$hi <- out$y + t * se
  }
  out
}
stata_bar_data <- function(d, stats, vars, over = character()) {
  f <- function(s, v) {
    v <- v[!is.na(v)]
    if (s == "mean") return(mean(v))
    if (s == "median") return(unname(stats::quantile(v, 0.5, type = 2)))
    if (grepl("^p[0-9]+$", s)) return(unname(stats::quantile(v, as.numeric(sub("p", "", s)) / 100, type = 2)))
    switch(s, sum = sum(v), count = length(v), min = min(v), max = max(v), sd = stats::sd(v))
  }
  key <- if (length(over)) interaction(lapply(over, function(o) stata_tab_values(d[[o]])), drop = TRUE, lex.order = TRUE) else factor(rep("", nrow(d)))
  out <- do.call(rbind, lapply(seq_along(vars), function(j) {
    # (count) with no variable: the rows
    x <- if (nzchar(vars[j])) stata_num(d[[vars[j]]]) else rep(1, nrow(d))
    # (percent) with no variable: each group's share of the rows, those with a
    # missing over() variable left out (Stata 19)
    # (asis): each group's one value as it is, the series named by the variable's label
    if (stats[j] == "asis" && any(duplicated(key[!is.na(key)]))) {
      stop("variables ", paste(over, collapse = " "), " and _variables do not uniquely identify the observations (Stata: r(459))", call. = FALSE)
    }
    val <- if (stats[j] == "percent") 100 * tapply(x, key, length) / sum(!is.na(key)) else if (stats[j] == "asis") tapply(x, key, function(v) v[1]) else tapply(x, key, function(v) f(stats[j], v))
    data.frame(group = factor(names(val), levels = levels(key)), series = if (stats[j] == "asis") stata_varlab(d, vars[j]) else if (nzchar(vars[j])) paste0(stats[j], " of ", vars[j]) else stats[j],
               value = as.numeric(val), stringsAsFactors = FALSE)
  }))
  out$series <- factor(out$series, levels = unique(out$series))
  out
}
stata_axes <- function(v1, v2) {
  r1 <- range(v1, na.rm = TRUE)
  r2 <- range(v2, na.rm = TRUE)
  if (!all(is.finite(c(r1, r2)))) stop("yaxis(2): no values to draw on one of the axes")
  if (r1[1] == r1[2]) r1 <- r1 + c(-1, 1)
  if (r2[1] == r2[2]) r2 <- r2 + c(-1, 1)
  k <- (r1[2] - r1[1]) / (r2[2] - r2[1])
  list(to = function(v) r1[1] + (v - r2[1]) * k, back = function(x) r2[1] + (x - r1[1]) / k)
}
stata_box_data <- function(d, vars, over = character()) {
  key <- if (length(over)) interaction(lapply(over, function(o) stata_tab_values(d[[o]])), drop = TRUE, lex.order = TRUE) else factor(rep("", nrow(d)))
  box <- list()
  pts <- list()
  for (v in vars) {
    x <- stata_num(d[[v]])
    lab <- stata_varlab(d, v)
    for (g in levels(key)) {
      y <- x[key == g & !is.na(key) & !is.na(x)]
      if (!length(y)) next
      q <- unname(stats::quantile(y, c(0.25, 0.5, 0.75), type = 2))
      iqr <- q[3] - q[1]
      lo <- min(y[y >= q[1] - 1.5 * iqr])
      hi <- max(y[y <= q[3] + 1.5 * iqr])
      box[[length(box) + 1]] <- data.frame(group = g, series = lab, lo = lo, q1 = q[1], q2 = q[2], q3 = q[3], hi = hi)
      o <- y[y < lo | y > hi]
      if (length(o)) pts[[length(pts) + 1]] <- data.frame(group = g, series = lab, value = o)
    }
  }
  box <- do.call(rbind, box)
  box$group <- factor(box$group, levels = levels(key))
  box$series <- factor(box$series, levels = unique(box$series))
  out <- if (length(pts)) do.call(rbind, pts) else data.frame(group = character(), series = character(), value = numeric())
  out$group <- factor(out$group, levels = levels(key))
  out$series <- factor(out$series, levels = levels(box$series))
  list(box = box, out = out)
}
# the graphs kept by name (an unnamed one is `Graph`), as Stata 19 keeps them in memory
stata_graph_dir <- function() {
  nm <- if (exists("stata_graphs")) names(stata_graphs) else character()
  if (!length(nm)) return(invisible(NULL))
  # Stata's order (capitals first), in columns two wider than the longest name,
  # filled down, as many as fit the line after four spaces
  nm <- nm[order(nm, method = "radix")]
  w <- max(nchar(nm)) + 2
  ls <- if (exists("stata_linesize")) stata_linesize else 80
  nc <- max(1, (ls - 4) %/% w)
  nr <- ceiling(length(nm) / nc)
  for (r in seq_len(nr)) {
    idx <- r + (seq_len(nc) - 1) * nr
    idx <- idx[idx <= length(nm)]
    cat("    ", sub(" +$", "", paste(formatC(nm[idx], width = -w), collapse = "")), "\n", sep = "")
  }
  invisible(NULL)
}
stata_graph_get <- function(name) {
  if (exists("stata_graphs") && !is.null(stata_graphs[[name]])) return(stata_graphs[[name]])
  stop("graph ", name, " not found (Stata: r(111))", call. = FALSE)
}
stata_graph_show <- function(g) {
  if (inherits(g, "ggplot")) print(g) else { grid::grid.newpage(); grid::grid.draw(g) }
  invisible(g)
}"#;

/// Options that change only how a graph looks; ggplot2's defaults stand in.
pub(super) const LOOK: &[(&str, usize)] = &[
    // m() is msymbol() as ZNMIXH writes it (Stata takes it)
    ("msymbol", 1),
    ("mcolor", 2),
    ("msize", 3),
    ("mfcolor", 3),
    ("mlcolor", 3),
    ("mstyle", 3),
    ("mlabel", 4),
    ("mlabposition", 5),
    ("mlabvposition", 5),
    ("mlabsize", 5),
    ("mlabcolor", 5),
    ("mlabangle", 9),
    ("mlabgap", 7),
    ("mlwidth", 3),
    ("msangle", 7),
    ("lcolor", 2),
    ("lwidth", 2),
    ("lpattern", 2),
    ("clpattern", 3),
    ("clcolor", 3),
    ("clwidth", 3),
    ("pstyle", 6),
    ("bgcolor", 3),
    ("lstyle", 3),
    ("color", 3),
    ("fcolor", 2),
    ("fintensity", 2),
    ("bcolor", 2),
    ("blwidth", 3),
    ("lalign", 4),
    ("barwidth", 4),
    ("bar", 3),
    ("blcolor", 3),
    ("bfcolor", 3),
    ("scheme", 3),
    ("name", 4),
    ("xlabel", 3),
    ("ylabel", 3),
    ("xtick", 5),
    ("xmtick", 3),
    ("ymtick", 3),
    ("ytick", 5),
    ("xmlabel", 4),
    ("ymlabel", 4),
    ("xscale", 3),
    ("yscale", 3),
    ("graphregion", 6),
    ("plotregion", 5),
    ("aspectratio", 6),
    ("t1title", 7),
    ("t2title", 7),
    ("b1title", 7),
    ("b2title", 7),
    ("l1title", 7),
    ("l2title", 7),
    ("r1title", 7),
    ("r2title", 7),
    ("xsize", 5),
    ("ysize", 5),
    ("nodraw", 6),
    ("legend", 3),
    ("bargap", 4),
    ("intensity", 3),
    ("blabel", 4),
    ("lcolor", 2),
];

/// The title options, as ggplot2's `labs()` names them.
const TITLES: &[(&str, usize, &str)] = &[
    ("title", 2, "title"),
    ("subtitle", 3, "subtitle"),
    ("xtitle", 2, "x"),
    ("ytitle", 2, "y"),
    ("note", 4, "caption"),
    ("caption", 3, "caption"),
    // b1title(): the title below the x axis (graph bar's groups)
    ("b1title", 7, "x"),
];

/// One layer of a twoway graph.
struct Layer {
    kind: &'static str,
    ys: Vec<String>,
    x: String,
    /// The R expression of the rows it draws.
    rows: String,
    opts: Vec<crate::lang::syntax::Opt>,
    /// drawn against the second y axis, yaxis(2)
    y2: bool,
}

impl Session {
    // ---------------- histogram ----------------

    pub(super) fn histogram(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "histogram")?;
        let known = [
            ("bin", 3),
            ("width", 1),
            ("start", 5),
            ("discrete", 1),
            ("density", 3),
            ("fraction", 4),
            ("frequency", 4),
            ("percent", 1),
            ("normal", 4),
            ("kdensity", 5),
        ];
        let v = self.graph_var(c, "histogram")?;
        let rows = self.rows_for(c, out)?;
        let layer = self.hist_layer(&v, &rows, opts, out)?;
        let mut layers = vec![layer];
        if opts.has("normal", 4) {
            layers.push(format!(
                "ggplot2::geom_line(data = local({{ x <- stata_num({rows}${0}); x <- x[!is.na(x)]; g <- seq(min(x), max(x), length.out = 100); data.frame(x = g, y = stats::dnorm(g, mean(x), stats::sd(x)) * stata_hist_scale) }}), ggplot2::aes(x = x, y = y))",
                r_name(&v)
            ));
        }
        if opts.has("kdensity", 5) {
            layers.push(format!(
                "ggplot2::geom_line(data = local({{ k <- stata_kdensity({rows}${}); data.frame(x = k$x, y = k$density * stata_hist_scale) }}), ggplot2::aes(x = x, y = y))",
                r_name(&v)
            ));
        }
        let ytitle = hist_type(opts).1;
        self.draw(
            c,
            opts,
            out,
            &known,
            &layers,
            &format!("stata_varlab({rows}, {})", r_text(&v)),
            &r_text(ytitle),
        )
    }

    /// The bars of a histogram, with `stata_hist_scale` the factor that puts a
    /// density on the bars' scale (for `normal` and `kdensity`).
    fn hist_layer(
        &mut self,
        v: &str,
        rows: &str,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<String, Untranslated> {
        let num = |o: &str, m: usize| -> Result<Option<f64>, Untranslated> {
            match opts.arg(o, m) {
                None => Ok(None),
                Some(a) => a
                    .trim()
                    .parse::<f64>()
                    .map(Some)
                    .map_err(|_| no(format!("{o}({a}) needs a number"))),
            }
        };
        let (bin, width, start) = (num("bin", 3)?, num("width", 1)?, num("start", 5)?);
        if bin.is_some() && width.is_some() {
            return Err(no("options bin() and width() may not be combined"));
        }
        let (ty, _) = hist_type(opts);
        self.helpers.insert("stata_graph");
        self.helpers.insert("stata_tab_values");
        let mut args = vec![
            format!("{rows}${}", r_name(v)),
            format!("name = {}", r_text(v)),
        ];
        if let Some(b) = bin {
            args.push(format!("bin = {}", r_number(b)));
        }
        if let Some(w) = width {
            args.push(format!("width = {}", r_number(w)));
        }
        if let Some(s) = start {
            args.push(format!("start = {}", r_number(s)));
        }
        if opts.has("discrete", 1) {
            args.push("discrete = TRUE".into());
        }
        args.push(format!("type = {}", r_text(ty)));
        out.line(format!("stata_h <- stata_hist_data({})", args.join(", ")));
        out.line(format!(
            "stata_hist_scale <- switch({}, density = 1, fraction = stata_h$width[1], frequency = sum(stata_h$count) * stata_h$width[1], percent = 100 * stata_h$width[1])",
            r_text(ty)
        ));
        out.caveat(
            "the bins are Stata's: int(min(sqrt(N), 10 log10 N)) of them from the minimum, unless \
             bin(), width() or discrete say otherwise",
        );
        Ok(
            "ggplot2::geom_col(data = stata_h, ggplot2::aes(x = mid, y = height), width = stata_h$width[1], fill = \"grey70\", colour = \"grey30\")"
                .to_string(),
        )
    }

    // ---------------- kdensity ----------------

    pub(super) fn kdensity(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "kdensity")?;
        let known = [
            ("kernel", 1),
            ("bwidth", 1),
            ("n", 1),
            ("at", 2),
            ("normal", 3),
            ("generate", 3),
            ("nograph", 4),
        ];
        let v = self.graph_var(c, "kdensity")?;
        let rows = self.rows_for(c, out)?;
        let call = self.kd_call(&v, &rows, opts)?;
        out.line(format!("stata_k <- {call}"));
        out.caveat(
            "Stata's kernel density: the Epanechnikov kernel unless kernel() names another, the \
             bandwidth 0.9 min(sd, IQR / 1.349) N^(-1/5) unless bwidth(), at min(N, 50) points",
        );
        if let Some(g) = opts.arg("generate", 3) {
            let names: Vec<&str> = g.split_whitespace().collect();
            // with at(v): one new variable, the density at each row's v
            if let ([gd], true) = (names.as_slice(), opts.arg("at", 2).is_some()) {
                if !is_name(gd) {
                    return Err(no("kdensity, generate() needs a new variable name"));
                }
                if c.parts.if_exp.is_some() || c.parts.in_range.is_some() {
                    return Err(no(
                        "kdensity, at() generate() with if or in is not translated yet",
                    ));
                }
                let d = self.data()?;
                out.line(format!("{d}[[{}]] <- stata_k$density", r_text(gd)));
                self.kinds.insert((*gd).to_string(), Kind::Number);
                if opts.has("nograph", 4) {
                    return Ok(());
                }
                return Err(no("kdensity, at() with a graph is not translated yet"));
            }
            // generate(x d): the points and the densities as new variables
            let [gx, gd] = names.as_slice() else {
                return Err(no("kdensity, generate() takes two new variable names here"));
            };
            if !is_name(gx) || !is_name(gd) {
                return Err(no("kdensity, generate() needs new variable names"));
            }
            let d = self.data()?;
            out.line(format!(
                "{d}[[{gx}]] <- c(stata_k$x, rep(NA_real_, max(0, nrow({d}) - nrow(stata_k))))[seq_len(nrow({d}))]",
                gx = r_text(gx)
            ));
            out.line(format!(
                "{d}[[{gd}]] <- c(stata_k$density, rep(NA_real_, max(0, nrow({d}) - nrow(stata_k))))[seq_len(nrow({d}))]",
                gd = r_text(gd)
            ));
            for n in [gx, gd] {
                self.kinds.insert(n.to_string(), Kind::Number);
            }
        }
        if opts.has("nograph", 4) {
            return Ok(());
        }
        let mut layers = vec![
            "ggplot2::geom_line(data = stata_k, ggplot2::aes(x = x, y = density))".to_string(),
        ];
        if opts.has("normal", 3) {
            layers.push(format!(
                "ggplot2::geom_line(data = local({{ x <- stata_num({rows}${0}); x <- x[!is.na(x)]; data.frame(x = stata_k$x, y = stats::dnorm(stata_k$x, mean(x), stats::sd(x))) }}), ggplot2::aes(x = x, y = y), linetype = \"dashed\")",
                r_name(&v)
            ));
        }
        out.line(
            "cat(\"kernel = \", stata_kernel_name, \", bandwidth = \", format(attr(stata_k, \"bwidth\"), digits = 4), \"\\n\", sep = \"\")",
        );
        // Stata's own title and note (title() and note() still replace them)
        layers.push(
            "ggplot2::labs(title = \"Kernel density estimate\", caption = paste0(\"kernel = \", stata_kernel_name, \", bandwidth = \", format(attr(stata_k, \"bwidth\"), digits = 4)))"
                .to_string(),
        );
        self.draw(
            c,
            opts,
            out,
            &known,
            &layers,
            &format!("stata_varlab({rows}, {})", r_text(&v)),
            "\"Density\"",
        )
    }

    fn kd_call(&mut self, v: &str, rows: &str, opts: &Opts) -> Result<String, Untranslated> {
        let kernels = [
            ("epanechnikov", 1),
            ("epan2", 5),
            ("biweight", 2),
            ("cosine", 2),
            ("gaussian", 1),
            ("parzen", 1),
            ("rectangle", 1),
            ("triangle", 1),
        ];
        let kernel = match opts.arg("kernel", 1) {
            None => "epanechnikov",
            Some(k) => {
                let k = k.trim();
                kernels
                    .iter()
                    .find(|(full, m)| abbrev(k, full, *m) && !(k == "epan" && *full == "epan2"))
                    .map(|(full, _)| *full)
                    .ok_or_else(|| no(format!("kernel({k}) is not a kernel Stata names")))?
            }
        };
        self.helpers.insert("stata_graph");
        self.helpers.insert("stata_tab_values");
        let mut args = vec![
            format!("{rows}${}", r_name(v)),
            format!("kernel = {}", r_text(kernel)),
        ];
        if let Some(b) = opts.arg("bwidth", 1) {
            let b: f64 = b
                .trim()
                .parse()
                .map_err(|_| no(format!("bwidth({b}) needs a number")))?;
            args.push(format!("bw = {}", r_number(b)));
        }
        if let Some(n) = opts.arg("n", 1) {
            let n: usize = n
                .trim()
                .parse()
                .map_err(|_| no(format!("n({n}) needs a whole number")))?;
            args.push(format!("n = {n}"));
        }
        if let Some(a) = opts.arg("at", 2) {
            let a = a.trim();
            if !is_name(a) {
                return Err(no(format!("at({a}) needs a variable")));
            }
            args.push(format!("at = {rows}${}", r_name(a)));
        }
        Ok(format!(
            "local({{ stata_kernel_name <<- {}; stata_kdensity({}) }})",
            r_text(kernel),
            args.join(", ")
        ))
    }

    // ---------------- qnorm, pnorm, rvfplot ----------------

    /// `qnorm y` and `pnorm y`: the normal quantile and probability plots, as
    /// Stata 19 draws them (its sersets): the sorted values against mean + sd
    /// · invnormal(i/(N+1)), or Φ((value − mean)/sd) against i/(N+1), each
    /// with the line y = x from the first point's x to the last's.
    pub(super) fn normal_plot(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
        quantile: bool,
    ) -> Result<(), Untranslated> {
        let name = if quantile { "qnorm" } else { "pnorm" };
        self.no_by(c, name)?;
        if opts.has("grid", 4) {
            return Err(no(format!("{name}, grid is not translated yet")));
        }
        if c.parts.weight.is_some() {
            return Err(no("weights in a graph are not translated yet"));
        }
        let words: Vec<&str> = c.parts.main.split_whitespace().collect();
        let v = match words.as_slice() {
            [v] if is_name(v) => *v,
            [] => {
                out.line("stop(\"varlist required (Stata: r(100))\", call. = FALSE)");
                return Ok(());
            }
            [_, _, ..] => {
                out.line("stop(\"too many variables specified (Stata: r(103))\", call. = FALSE)");
                return Ok(());
            }
            _ => return Err(no(format!("{name} needs a variable name"))),
        };
        let rows = self.rows_for(c, out)?;
        self.helpers.insert("stata_graph");
        self.helpers.insert("stata_tab_values");
        out.line(format!(
            "stata_np <- local({{ v <- sort(stata_num({rows}${})); n <- length(v); m <- mean(v); s <- stats::sd(v); p <- seq_len(n) / (n + 1); \
             if (n == 0) stop(\"no observations (Stata: r(2000))\", call. = FALSE); {} }})",
            r_name(v),
            if quantile {
                "data.frame(x = m + s * stats::qnorm(p), y = v)"
            } else {
                "data.frame(x = p, y = stats::pnorm((v - m) / s))"
            }
        ));
        let line = "ggplot2::geom_line(data = data.frame(x = range(stata_np$x), y = range(stata_np$x)), ggplot2::aes(x = x, y = y))";
        let mut layers = vec![
            "ggplot2::geom_point(data = stata_np, ggplot2::aes(x = x, y = y))".to_string(),
            line.to_string(),
        ];
        let (xtitle, ytitle) = if quantile {
            (
                r_text("Inverse normal"),
                format!("stata_varlab({rows}, {})", r_text(v)),
            )
        } else {
            // pnorm's axes from 0 to 1 by .25, shown with two decimals
            for ax in ["x", "y"] {
                layers.push(format!(
                    "ggplot2::scale_{ax}_continuous(limits = c(0, 1), breaks = seq(0, 1, 0.25), labels = sprintf(\"%.2f\", seq(0, 1, 0.25)))"
                ));
            }
            (
                r_text("Empirical P[i] = i/(N+1)"),
                r_text("Normal F[(y-m)/s]"),
            )
        };
        self.draw(c, opts, out, &[("grid", 4)], &layers, &xtitle, &ytitle)
    }

    /// `rvfplot` after `regress`: the residuals against the fitted values of
    /// the estimation sample, titled `Residuals` and `Fitted values` (Stata 19).
    pub(super) fn rvfplot(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if !c.parts.main.trim().is_empty() || c.parts.if_exp.is_some() || c.parts.in_range.is_some()
        {
            return Err(no("rvfplot takes no varlist, if or in"));
        }
        let est = self.estimates_for("rvfplot")?;
        if est.fit != super::model::Fit::Regress {
            return Err(no(format!(
                "rvfplot after {} is not translated yet (after regress it is)",
                est.fit.name()
            )));
        }
        self.helpers.insert("stata_graph");
        self.helpers.insert("stata_tab_values");
        out.line(
            "stata_rvf <- data.frame(x = unname(stats::fitted(stata_estimates)), y = unname(stats::residuals(stata_estimates)))",
        );
        let layers =
            vec!["ggplot2::geom_point(data = stata_rvf, ggplot2::aes(x = x, y = y))".to_string()];
        self.draw(
            c,
            opts,
            out,
            &[],
            &layers,
            &r_text("Fitted values"),
            &r_text("Residuals"),
        )
    }

    /// `avplot x` after `regress`: the residuals of y on the other regressors
    /// against those of x, the line through 0 of slope x's coefficient, and
    /// Stata 19's note `coef = .81065199, se = .07891904, t = 10.27` (`%10.0g`,
    /// t to two decimals, `(robust) se` with a variance other than OLS's).
    pub(super) fn avplot(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let words: Vec<&str> = c.parts.main.split_whitespace().collect();
        let x = match words.as_slice() {
            [x] if is_name(x) => *x,
            _ => return Err(no("avplot takes one regressor's name")),
        };
        if c.parts.if_exp.is_some() || c.parts.in_range.is_some() {
            return Err(no("avplot takes no if or in"));
        }
        let est = self.estimates_for("avplot")?;
        if est.fit != super::model::Fit::Regress {
            return Err(no(format!(
                "avplot after {} is not translated yet (after regress it is)",
                est.fit.name()
            )));
        }
        self.helpers.insert("stata_graph");
        self.helpers.insert("stata_tab_values");
        self.helpers.insert("stata_fmt");
        out.line(format!(
            "stata_av <- local({{ X <- stats::model.matrix(stata_estimates); j <- match({x}, colnames(X)); \
             if (is.na(j)) stop(\"{xs} is not among the regressors (Stata: r(398))\", call. = FALSE); \
             y <- stats::model.response(stats::model.frame(stata_estimates)); w <- stats::weights(stata_estimates); if (is.null(w)) w <- rep(1, length(y)); \
             ey <- stats::lm.wfit(X[, -j, drop = FALSE], y, w)$residuals; ex <- stats::lm.wfit(X[, -j, drop = FALSE], X[, j], w)$residuals; \
             b <- unname(stats::coef(stata_estimates)[j]); se <- sqrt(diag(as.matrix(stata_vcov)))[j]; \
             robust <- !isTRUE(all.equal(unname(as.matrix(stata_vcov)), unname(as.matrix(stats::vcov(stata_estimates))))); \
             o <- order(ex); list(points = data.frame(x = ex[o], y = ey[o]), b = b, \
             note = paste0(\"coef = \", stata_fmt(b, \"%10.0g\", trim = TRUE), \", \", if (robust) \"(robust) \" else \"\", \"se = \", stata_fmt(se, \"%10.0g\", trim = TRUE), \", t = \", sprintf(\"%.2f\", b / se))) }})",
            x = r_text(x),
            xs = x
        ));
        let layers = vec![
            "ggplot2::geom_point(data = stata_av$points, ggplot2::aes(x = x, y = y))".to_string(),
            "ggplot2::geom_line(data = stata_av$points, ggplot2::aes(x = x, y = stata_av$b * x))"
                .to_string(),
            "ggplot2::labs(caption = stata_av$note)".to_string(),
        ];
        let dep = est.dep.clone();
        self.draw(
            c,
            opts,
            out,
            &[],
            &layers,
            &r_text(&format!("e( {x} | X )")),
            &r_text(&format!("e( {dep} | X )")),
        )
    }

    /// `screeplot` after `pca` or `factor`: the eigenvalues against their
    /// number, titled as Stata 19 titles it.
    pub(super) fn screeplot(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if !c.parts.main.trim().is_empty() || c.parts.if_exp.is_some() || c.parts.in_range.is_some()
        {
            return Err(no("screeplot takes no varlist, if or in"));
        }
        let after = match &self.e_ret {
            super::returned::Returned::Kept { cmd, .. } if cmd == "pca" || cmd == "factor" => {
                cmd.clone()
            }
            _ => {
                return Err(no(
                    "screeplot is translated after pca or factor in this do-file",
                ))
            }
        };
        self.helpers.insert("stata_graph");
        self.helpers.insert("stata_tab_values");
        out.line(
            "stata_scree <- data.frame(x = seq_along(stata_e$Ev), y = as.numeric(stata_e$Ev))",
        );
        let layers = vec![
            "ggplot2::geom_point(data = stata_scree, ggplot2::aes(x = x, y = y))".to_string(),
            "ggplot2::geom_line(data = stata_scree, ggplot2::aes(x = x, y = y))".to_string(),
            format!(
                "ggplot2::labs(title = {})",
                r_text(&format!("Scree plot of eigenvalues after {after}"))
            ),
        ];
        self.draw(
            c,
            opts,
            out,
            &[],
            &layers,
            &r_text("Number"),
            &r_text("Eigenvalues"),
        )
    }

    // ---------------- lowess ----------------

    pub(super) fn lowess(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "lowess")?;
        let known = [("bwidth", 2), ("generate", 3), ("nograph", 4)];
        let words: Vec<&str> = c.parts.main.split_whitespace().collect();
        let [y, x] = words.as_slice() else {
            return Err(no("lowess takes a y and an x variable"));
        };
        if !is_name(y) || !is_name(x) {
            return Err(no("lowess needs variable names"));
        }
        let rows = self.rows_for(c, out)?;
        let bw = match opts.arg("bwidth", 2) {
            Some(b) => b
                .trim()
                .parse::<f64>()
                .map_err(|_| no(format!("bwidth({b}) needs a number")))?,
            None => 0.8,
        };
        self.helpers.insert("stata_graph");
        self.helpers.insert("stata_tab_values");
        out.line(format!(
            "stata_lw <- stata_lowess({rows}${}, {rows}${}, bw = {})",
            r_name(x),
            r_name(y),
            r_number(bw)
        ));
        out.caveat(
            "Stata's lowess: a local line with tricube weights over the nearest 80% of the data, \
             no robustness steps (not R's lowess())",
        );
        if let Some(g) = opts.arg("generate", 3) {
            let g = g.trim();
            if !is_name(g) {
                return Err(no("lowess, generate() needs a new variable name"));
            }
            if c.parts.if_exp.is_some() || c.parts.in_range.is_some() {
                return Err(no("lowess, generate() with if or in is not translated yet"));
            }
            let d = self.data()?;
            out.line(format!("{d}[[{}]] <- stata_lw", r_text(g)));
            self.kinds.insert(g.to_string(), Kind::Number);
        }
        if opts.has("nograph", 4) {
            return Ok(());
        }
        let layers = vec![
            format!(
                "ggplot2::geom_point(data = {rows}, ggplot2::aes(x = stata_num(.data[[{x}]]), y = stata_num(.data[[{y}]])))",
                x = r_text(x),
                y = r_text(y)
            ),
            format!(
                "ggplot2::geom_line(data = data.frame(x = stata_num({rows}${}), y = stata_lw), ggplot2::aes(x = x, y = y))",
                r_name(x)
            ),
        ];
        self.draw(
            c,
            opts,
            out,
            &known,
            &layers,
            &format!("stata_varlab({rows}, {})", r_text(x)),
            &format!("stata_varlab({rows}, {})", r_text(y)),
        )
    }

    // ---------------- twoway ----------------

    /// `twoway (layer) (layer) ...`, `twoway a || b`, and `scatter`, `line`
    /// and `connected` alone, which are one-layer twoways.
    pub(super) fn twoway(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
        kind: Option<&'static str>,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "twoway")?;
        // without ( ): the line as written, plots split at ||, each with its
        // own options, which hold the graph's options too
        let written = format!(
            "{}{}{}{}",
            c.parts.main,
            c.parts
                .if_exp
                .as_ref()
                .map(|e| format!(" if {e}"))
                .unwrap_or_default(),
            c.parts
                .in_range
                .as_ref()
                .map(|r| format!(" in {r}"))
                .unwrap_or_default(),
            if c.parts.options.trim().is_empty() {
                String::new()
            } else {
                format!(", {}", c.parts.options)
            }
        );
        let bare = kind.is_none() && !c.parts.main.trim_start().starts_with('(');
        let texts: Vec<String> = match kind {
            // scatter, line and connected alone: one layer with all of their options
            None if bare => split_plots(&written)?,
            Some(k) => vec![format!(
                "{k} {}{}{}{}",
                c.parts.main,
                c.parts
                    .if_exp
                    .as_ref()
                    .map(|e| format!(" if {e}"))
                    .unwrap_or_default(),
                c.parts
                    .in_range
                    .as_ref()
                    .map(|r| format!(" in {r}"))
                    .unwrap_or_default(),
                if c.parts.options.trim().is_empty() {
                    String::new()
                } else {
                    format!(", {}", c.parts.options)
                }
            )],
            None => layer_texts(&c.parts.main)?,
        };
        if texts.is_empty() {
            return Err(no("twoway needs at least one plot"));
        }
        let mut layers = Vec::new();
        let mut parsed = Vec::new();
        for t in &texts {
            let l = self.layer(t, out)?;
            parsed.push(l);
        }
        // yaxis(2): its plots are mapped onto the first axis's range from the
        // values of each axis, and the right axis shows their own scale
        let any_y2 = parsed.iter().any(|l| l.y2);
        let mut second_axis = None;
        if any_y2 {
            if parsed.iter().all(|l| l.y2) {
                return Err(no(
                    "twoway with every plot on yaxis(2) is not translated yet",
                ));
            }
            self.y2_count += 1;
            let k = self.y2_count;
            self.y2_fn = format!("stata_y2_{k}");
            let values = |two: bool| -> Result<String, Untranslated> {
                let mut v = Vec::new();
                for l in parsed.iter().filter(|l| l.y2 == two) {
                    for y in &l.ys {
                        v.push(format!("stata_num({}${})", l.rows, r_name(y)));
                    }
                    if l.kind == "bar" {
                        let base = match Opts(&l.opts).arg("base", 4) {
                            Some(b) => b
                                .trim()
                                .parse::<f64>()
                                .map_err(|_| no(format!("base({b}) needs a number")))?,
                            None => 0.0,
                        };
                        v.push(r_number(base));
                    }
                }
                Ok(v.join(", "))
            };
            out.line(format!(
                "stata_ax_{k} <- stata_axes(c({}), c({}))",
                values(false)?,
                values(true)?
            ));
            out.line(format!("stata_y2_{k} <- stata_ax_{k}$to"));
            // the right axis's title: its one y's label, as Stata's
            let twos: Vec<&Layer> = parsed.iter().filter(|l| l.y2).collect();
            let title = match twos.first().and_then(|l| l.ys.first().map(|y| (l, y))) {
                Some((l0, y))
                    if twos
                        .iter()
                        .all(|l| l.ys.len() == 1 && l.ys.first() == Some(y)) =>
                {
                    format!("stata_varlab({}, {})", l0.rows, r_text(y))
                }
                _ => "NULL".to_string(),
            };
            second_axis = Some(format!(
                "ggplot2::scale_y_continuous(sec.axis = ggplot2::sec_axis(stata_ax_{k}$back, name = {title}))"
            ));
        }
        let ones: Vec<&Layer> = parsed.iter().filter(|l| !l.y2).collect();
        let first = ones
            .first()
            .copied()
            .ok_or_else(|| no("twoway needs at least one plot"))?;
        let xtitle = format!("stata_varlab({}, {})", first.rows, r_text(&first.x));
        // the y title when every layer (on the first axis) has the same one y, as Stata shows it
        let same_y = ones
            .iter()
            .all(|l| l.ys.len() == 1 && l.ys.first() == first.ys.first());
        // (Stata 19: lfit or qfit alone is titled `Fitted values`, and a graph
        // with lfitci or qfitci has no y title)
        let ytitle = if ones.iter().any(|l| matches!(l.kind, "lfitci" | "qfitci")) {
            "NULL".to_string()
        } else if ones.iter().all(|l| matches!(l.kind, "lfit" | "qfit")) {
            r_text("Fitted values")
        } else {
            match first.ys.first() {
                Some(y) if same_y => format!("stata_varlab({}, {})", first.rows, r_text(y)),
                _ => "NULL".to_string(),
            }
        };
        // several layers (or y variables) get a legend, labelled as Stata labels them
        // (lfitci and qfitci draw two series, the band and the line, so they always do)
        let legend = parsed.len() > 1
            || parsed
                .iter()
                .any(|l| l.ys.len() > 1 || matches!(l.kind, "lfitci" | "qfitci"));
        // a one-layer command's own options are the graph's too
        let mut layer_opts: Vec<(&str, usize)> = vec![("sort", 4), ("by", 2)];
        for l in &parsed {
            layers.extend(self.layer_geoms(l, legend)?);
        }
        layers.extend(second_axis);
        if legend {
            layers.push("ggplot2::labs(colour = NULL, fill = NULL)".to_string());
        }
        if kind.is_some() || bare {
            // the plots' options, checked by each layer, are the graph's too
            let mut all = Vec::new();
            for l in &parsed {
                layer_opts.extend(kind_options(l.kind));
                all.extend(l.opts.iter().cloned());
            }
            if bare {
                return self.draw(c, &Opts(&all), out, &layer_opts, &layers, &xtitle, &ytitle);
            }
        }
        self.draw(c, opts, out, &layer_opts, &layers, &xtitle, &ytitle)
    }

    /// One twoway layer: its kind, variables and rows.
    fn layer(&mut self, text: &str, out: &mut Out) -> Result<Layer, Untranslated> {
        let program = crate::reader::read(text);
        let Some(crate::reader::Item::Command(cmd)) = program.items.first() else {
            return Err(no(format!("{text}: not a plot Stata knows here")));
        };
        let w = cmd.name.as_str();
        let kind = if abbrev(w, "scatter", 2) {
            "scatter"
        } else if w == "line" {
            "line"
        } else if abbrev(w, "connected", 4) {
            "connected"
        } else if w == "lfit" {
            "lfit"
        } else if w == "qfit" {
            "qfit"
        } else if w == "lowess" {
            "lowess"
        } else if abbrev(w, "histogram", 4) {
            "histogram"
        } else if abbrev(w, "kdensity", 5) {
            "kdensity"
        } else if w == "bar" {
            "bar"
        } else if w == "rcap" {
            "rcap"
        } else if w == "rspike" {
            "rspike"
        } else if w == "rarea" {
            "rarea"
        } else if w == "lfitci" {
            "lfitci"
        } else if w == "qfitci" {
            "qfitci"
        } else if w == "area" {
            "area"
        } else {
            return Err(no(format!(
                "twoway {w} is not translated yet (scatter, line, connected, lfit, qfit, lfitci, \
                 qfitci, lowess, histogram, kdensity, bar, area, rarea, rcap and rspike are)"
            )));
        };
        let vars: Vec<String> = cmd
            .parts
            .main
            .split_whitespace()
            .map(str::to_string)
            .collect();
        if let Some(bad) = vars.iter().find(|v| !is_name(v)) {
            return Err(no(format!(
                "{bad}: only variable names are translated in a plot"
            )));
        }
        let one = matches!(kind, "histogram" | "kdensity");
        let (ys, x) = if one {
            match vars.as_slice() {
                [v] => (vec![v.clone()], v.clone()),
                _ => return Err(no(format!("twoway {kind} takes one variable"))),
            }
        } else {
            match vars.split_last() {
                Some((x, ys)) if !ys.is_empty() => {
                    if matches!(
                        kind,
                        "lfit" | "qfit" | "lfitci" | "qfitci" | "lowess" | "bar" | "area"
                    ) && ys.len() != 1
                    {
                        return Err(no(format!("twoway {kind} takes one y and one x")));
                    }
                    if matches!(kind, "rarea" | "rcap" | "rspike") && ys.len() != 2 {
                        return Err(no(format!("twoway {kind} takes two y variables and one x")));
                    }
                    (ys.to_vec(), x.clone())
                }
                _ => return Err(no(format!("twoway {kind} needs a y and an x variable"))),
            }
        };
        if cmd.parts.weight.is_some() {
            return Err(no("weights in a graph are not translated yet"));
        }
        let rows = self.rows_for(cmd, out)?;
        // yaxis(1) is the default axis; yaxis(2) the second, on the right
        let y2 = match Opts(&cmd.options)
            .arg("yaxis", 5)
            .map(|a| a.trim().to_string())
        {
            None => false,
            Some(a) if a == "1" => false,
            Some(a) if a == "2" => {
                if !matches!(kind, "scatter" | "line" | "connected" | "bar" | "rarea") {
                    return Err(no(format!(
                        "twoway {kind} with yaxis(2) is not translated yet"
                    )));
                }
                true
            }
            Some(a) => {
                return Err(no(format!(
                    "yaxis({a}) is not translated yet (1 and 2 are)"
                )))
            }
        };
        Ok(Layer {
            kind,
            ys,
            x,
            rows,
            opts: cmd.options.clone(),
            y2,
        })
    }

    /// The ggplot2 layers that draw one twoway layer.
    fn layer_geoms(&mut self, l: &Layer, legend: bool) -> Result<Vec<String>, Untranslated> {
        self.helpers.insert("stata_graph");
        self.helpers.insert("stata_tab_values");
        let lo = Opts(&l.opts);
        let mut known: Vec<(&str, usize)> = LOOK.to_vec();
        known.extend([
            ("sort", 4),
            ("by", 2),
            ("xline", 5),
            ("yline", 5),
            ("yaxis", 5),
            ("saving", 4),
        ]);
        known.extend(TITLES.iter().map(|(o, m, _)| (*o, *m)));
        known.extend(kind_options(l.kind));
        if let Some(o) = lo.other(&known) {
            return Err(no(format!(
                "twoway {} with the option {o} is not translated yet",
                l.kind
            )));
        }
        let rows = &l.rows;
        // a layer on the second y axis is drawn mapped onto the first (stata_y2)
        let y2w = if l.y2 {
            self.y2_fn.clone()
        } else {
            String::new()
        };
        let x = r_text(&l.x);
        // each layer's legend label, as Stata's: the variable's label, `Fitted
        // values`, `lowess y x`, `Kernel density estimate`
        let series = |y: &str| -> String {
            if legend {
                format!(", colour = stata_varlab({rows}, {})", r_text(y))
            } else {
                String::new()
            }
        };
        let fixed = |label: String| -> String {
            if legend {
                format!(", colour = {}", r_text(&label))
            } else {
                String::new()
            }
        };
        let mut g = Vec::new();
        match l.kind {
            "scatter" | "line" | "connected" => {
                for y in &l.ys {
                    let aes = format!(
                        "ggplot2::aes(x = stata_num(.data[[{x}]]), y = {y2w}(stata_num(.data[[{}]])){})",
                        r_text(y),
                        series(y)
                    );
                    // line joins the points in the data's order unless sort
                    let geom = if lo.has("sort", 4) {
                        "geom_line"
                    } else {
                        "geom_path"
                    };
                    if l.kind != "line" {
                        g.push(format!("ggplot2::geom_point(data = {rows}, {aes})"));
                    }
                    if l.kind != "scatter" {
                        g.push(format!("ggplot2::{geom}(data = {rows}, {aes})"));
                    }
                }
            }
            "lfit" | "qfit" | "lfitci" | "qfitci" => {
                let y = r_text(l.ys.first().map(String::as_str).unwrap_or(""));
                let deg = if matches!(l.kind, "lfit" | "lfitci") {
                    1
                } else {
                    2
                };
                // Stata 19 draws lfit at 3 points and the others at 100; n() sets it
                let n = match lo.arg("n", 1) {
                    Some(v) => match v.trim().parse::<u32>() {
                        Ok(k) if k >= 2 => k,
                        _ => return Err(no(format!("n({v}) needs a whole number of 2 or more"))),
                    },
                    None if l.kind == "lfit" => 3,
                    None => 100,
                };
                // range(a b): the line's ends, `.` the data's own
                let (from, to) = match lo.arg("range", 1) {
                    None => ("NA".to_string(), "NA".to_string()),
                    Some(r) => {
                        let ends: Vec<&str> = r.split_whitespace().collect();
                        let end = |s: &str| -> Result<String, Untranslated> {
                            if s == "." {
                                return Ok("NA".into());
                            }
                            s.parse::<f64>()
                                .map(r_number)
                                .map_err(|_| no(format!("range({r}) needs two numbers or .")))
                        };
                        match ends.as_slice() {
                            [a, b] => (end(a)?, end(b)?),
                            _ => return Err(no(format!("range({r}) needs two numbers or ."))),
                        }
                    }
                };
                let ci = matches!(l.kind, "lfitci" | "qfitci");
                let level = if ci {
                    match lo.arg("level", 3) {
                        Some(v) => match v.trim().parse::<f64>() {
                            Ok(p) if (10.0..=99.99).contains(&p) => p,
                            _ => {
                                return Err(no(format!(
                                    "level({v}) needs a number from 10 to 99.99 (Stata: r(198))"
                                )))
                            }
                        },
                        None => 95.0,
                    }
                } else {
                    f64::NAN
                };
                if ci && lo.has("stdp", 4) && lo.has("stdf", 4) {
                    return Err(no(
                        "options stdp and stdf may not be combined (Stata: r(198))",
                    ));
                }
                let data = format!(
                    "stata_fitline({rows}, {y}, {x}, degree = {deg}, n = {n}, from = {from}, to = {to}{})",
                    if ci {
                        format!(
                            ", level = {}, stdf = {}",
                            r_number(level),
                            if lo.has("stdf", 4) { "TRUE" } else { "FALSE" }
                        )
                    } else {
                        String::new()
                    }
                );
                if ci {
                    // the band first, then the line, as Stata draws them; the
                    // legend says `95% CI` and `Fitted values`
                    let band = if legend {
                        format!(", fill = {}", r_text(&format!("{}% CI", fmt_level(level))))
                    } else {
                        String::new()
                    };
                    g.push(format!(
                        "ggplot2::geom_ribbon(data = {data}, ggplot2::aes(x = x, ymin = lo, ymax = hi{band}), alpha = 0.5)"
                    ));
                    g.push(format!(
                        "ggplot2::geom_line(data = {data}, ggplot2::aes(x = x, y = y{}))",
                        fixed("Fitted values".to_string())
                    ));
                } else {
                    g.push(format!(
                        "ggplot2::geom_line(data = {data}, ggplot2::aes(x = x, y = y{}))",
                        fixed("Fitted values".to_string())
                    ));
                }
            }
            "area" => {
                // the area under y down to base(), or to the bottom of the y axis
                // when there is no base() (Stata 19, observed in an SVG export)
                let y = l.ys.first().map(String::as_str).unwrap_or("");
                let base = match lo.arg("base", 4) {
                    Some(b) => r_number(
                        b.trim()
                            .parse::<f64>()
                            .map_err(|_| no(format!("base({b}) needs a number")))?,
                    ),
                    None => "-Inf".to_string(),
                };
                let fill = if legend {
                    format!(", fill = stata_varlab({rows}, {})", r_text(y))
                } else {
                    String::new()
                };
                g.push(format!(
                    "ggplot2::geom_ribbon(data = {rows}, ggplot2::aes(x = stata_num(.data[[{x}]]), ymin = {base}, ymax = stata_num(.data[[{yv}]]){fill}), orientation = \"x\")",
                    yv = r_text(y)
                ));
            }
            "lowess" => {
                let y = l.ys.first().map(String::as_str).unwrap_or("");
                let bw = match lo.arg("bwidth", 2) {
                    Some(b) => b
                        .trim()
                        .parse::<f64>()
                        .map_err(|_| no(format!("bwidth({b}) needs a number")))?,
                    None => 0.8,
                };
                g.push(format!(
                    "ggplot2::geom_line(data = data.frame(x = stata_num({rows}${xv}), y = stata_lowess({rows}${xv}, {rows}${yv}, bw = {bw})), ggplot2::aes(x = x, y = y{lab}))",
                    lab = fixed(format!("lowess {y} {}", l.x)),
                    xv = r_name(&l.x),
                    yv = r_name(y),
                    bw = r_number(bw)
                ));
            }
            "histogram" => {
                let mut out = Out::default();
                let layer = self.hist_layer(&l.x, rows, &lo, &mut out)?;
                // the bars' data is made inline here
                let data_line = out.code.first().cloned().unwrap_or_default();
                let data = data_line.trim_start_matches("stata_h <- ").to_string();
                g.push(
                    layer
                        .replace("data = stata_h", &format!("data = {data}"))
                        .replace(
                            "width = stata_h$width[1]",
                            &format!("width = ({data})$width[1]"),
                        ),
                );
            }
            "bar" => {
                // a bar from base() (0) to y at each x, barwidth() (1) wide
                let y = l.ys.first().map(String::as_str).unwrap_or("");
                let num = |o: &str, m: usize, d: f64| -> Result<f64, Untranslated> {
                    match lo.arg(o, m) {
                        Some(v) => v
                            .trim()
                            .parse::<f64>()
                            .map_err(|_| no(format!("{o}({v}) needs a number"))),
                        None => Ok(d),
                    }
                };
                let base = num("base", 4, 0.0)?;
                let w = num("barwidth", 4, 1.0)?;
                let fill = if legend {
                    format!(", fill = stata_varlab({rows}, {})", r_text(y))
                } else {
                    String::new()
                };
                g.push(format!(
                    "ggplot2::geom_rect(data = {rows}, ggplot2::aes(xmin = stata_num(.data[[{x}]]) - {h}, xmax = stata_num(.data[[{x}]]) + {h}, ymin = {y2w}({b}), ymax = {y2w}(stata_num(.data[[{yv}]])){fill}), na.rm = TRUE)",
                    h = r_number(w / 2.0),
                    b = r_number(base),
                    yv = r_text(y)
                ));
            }
            "rarea" => {
                // the area between two y's over x, in the data's order of x
                let (y1, y2) = match l.ys.as_slice() {
                    [a, b] => (r_text(a), r_text(b)),
                    _ => return Err(no("twoway rarea takes two y variables and one x")),
                };
                let fill = if legend {
                    format!(
                        ", fill = paste(stata_varlab({rows}, {y1}), \"/\", stata_varlab({rows}, {y2}))"
                    )
                } else {
                    String::new()
                };
                g.push(format!(
                    "ggplot2::geom_ribbon(data = {rows}, ggplot2::aes(x = stata_num(.data[[{x}]]), ymin = {y2w}(stata_num(.data[[{y1}]])), ymax = {y2w}(stata_num(.data[[{y2}]])){fill}), orientation = \"x\")"
                ));
            }
            "rcap" | "rspike" => {
                // a vertical line from one y to the other at each x, capped for rcap
                let (y1, y2) = match l.ys.as_slice() {
                    [a, b] => (r_text(a), r_text(b)),
                    _ => {
                        return Err(no(format!(
                            "twoway {} takes two y variables and one x",
                            l.kind
                        )))
                    }
                };
                let colour = if legend {
                    format!(
                        ", colour = paste(stata_varlab({rows}, {y1}), \"/\", stata_varlab({rows}, {y2}))"
                    )
                } else {
                    String::new()
                };
                let geom = if l.kind == "rcap" {
                    "geom_errorbar"
                } else {
                    "geom_linerange"
                };
                let width = if l.kind == "rcap" {
                    ", width = 0.2"
                } else {
                    ""
                };
                if lo.has("horizontal", 3) {
                    // horizontal: from one value to the other along x, at each y
                    if l.y2 {
                        return Err(no(format!(
                            "twoway {}, horizontal with yaxis(2) is not translated yet",
                            l.kind
                        )));
                    }
                    g.push(format!(
                        "ggplot2::{geom}(data = {rows}, ggplot2::aes(y = stata_num(.data[[{x}]]), xmin = stata_num(.data[[{y1}]]), xmax = stata_num(.data[[{y2}]]){colour}), orientation = \"y\"{width})"
                    ));
                } else {
                    g.push(format!(
                        "ggplot2::{geom}(data = {rows}, ggplot2::aes(x = stata_num(.data[[{x}]]), ymin = {y2w}(stata_num(.data[[{y1}]])), ymax = {y2w}(stata_num(.data[[{y2}]])){colour}){width})"
                    ));
                }
            }
            "kdensity" => {
                let call = self.kd_call(&l.x, rows, &lo)?;
                g.push(format!(
                    "ggplot2::geom_line(data = {call}, ggplot2::aes(x = x, y = density{}))",
                    fixed("Kernel density estimate".to_string())
                ));
            }
            _ => {}
        }
        Ok(g)
    }

    // ---------------- graph ----------------

    pub(super) fn graph(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let (sub, rest) = first_word(&c.parts.main);
        if abbrev(sub, "twoway", 2) {
            let mut inner = c.clone();
            inner.parts.main = rest.to_string();
            return self.twoway(&inner, opts, out, None);
        }
        if sub == "bar" || sub == "hbar" {
            return self.graph_bar(c, rest, opts, out, sub == "hbar");
        }
        if sub == "pie" {
            return self.graph_pie(c, rest, opts, out);
        }
        if abbrev(sub, "matrix", 3) {
            return self.graph_matrix(c, rest, opts, out);
        }
        if sub == "box" || sub == "hbox" {
            return self.graph_box(c, rest, opts, out, sub == "hbox");
        }
        if abbrev(sub, "export", 3) {
            return self.graph_export(c, rest, opts, out);
        }
        if abbrev(sub, "save", 2) {
            return self.graph_save(rest, opts, out);
        }
        if abbrev(sub, "combine", 3) {
            return self.graph_combine(c, rest, opts, out);
        }
        if sub == "drop" {
            // the graphs Stata keeps by name (an unnamed one is Graph)
            let names: Vec<&str> = rest.split_whitespace().collect();
            if names.is_empty() {
                return Err(no("graph drop needs graph names or _all (Stata: r(198))"));
            }
            if names == ["_all"] {
                self.graph_drawn = false;
                out.line("if (exists(\"stata_graph\")) rm(stata_graph)");
                out.line("if (exists(\"stata_graphs\")) rm(stata_graphs)");
                return Ok(());
            }
            if let Some(bad) = names.iter().find(|n| !is_name(n)) {
                return Err(no(format!(
                    "graph drop {bad}: graph names with wildcards are not translated yet"
                )));
            }
            // Stata drops none of them when one is not there
            let list = names
                .iter()
                .map(|n| r_text(n))
                .collect::<Vec<_>>()
                .join(", ");
            out.line(format!(
                "local({{ stata_dn <- c({list}); stata_miss <- setdiff(stata_dn, if (exists(\"stata_graphs\")) names(stata_graphs) else character()); \
                 if (length(stata_miss)) stop(\"graph \", stata_miss[1], \" not found\\nno graphs dropped (Stata: r(111))\", call. = FALSE); \
                 stata_graphs[stata_dn] <<- NULL }})"
            ));
            return Ok(());
        }
        if sub == "dir" {
            if let Some(o) = opts.other(&[("memory", 3)]) {
                return Err(no(format!(
                    "graph dir with the option {o} is not translated yet"
                )));
            }
            if !rest.trim().is_empty() {
                return Err(no(
                    "graph dir with graph names or a pattern is not translated yet",
                ));
            }
            self.helpers.insert("stata_graph");
            out.line("stata_graph_dir()");
            return Ok(());
        }
        if abbrev(sub, "display", 2) {
            if let Some(o) = opts.other(&[]) {
                return Err(no(format!(
                    "graph display with the option {o} is not translated yet"
                )));
            }
            let name = match rest.split_whitespace().collect::<Vec<_>>().as_slice() {
                [] => "Graph".to_string(),
                [n] if is_name(n) => (*n).to_string(),
                _ => return Err(no("graph display takes one graph name")),
            };
            self.helpers.insert("stata_graph");
            out.line(format!("stata_graph <- stata_graph_get({})", r_text(&name)));
            if !self.quiet(c) {
                out.line("stata_graph_show(stata_graph)");
            }
            self.graph_drawn = true;
            return Ok(());
        }
        Err(no(format!("graph {sub} is not translated yet")))
    }

    /// `graph pie, over(g)`, `graph pie y, over(g)` and `graph pie a b c`: a
    /// slice for each group's rows, each group's sum of y, or each variable's
    /// sum (Stata 19's sersets), the first at twelve o'clock and the rest
    /// counterclockwise, labelled by the group's value or the variable.
    fn graph_pie(
        &mut self,
        c: &Command,
        rest: &str,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "graph pie")?;
        if c.parts.weight.is_some() {
            return Err(no("weights in a graph are not translated yet"));
        }
        let vars: Vec<&str> = rest.split_whitespace().collect();
        if let Some(bad) = vars.iter().find(|v| !is_name(v)) {
            return Err(no(format!(
                "graph pie {bad}: only variable names are translated"
            )));
        }
        let over = match opts.arg("over", 4) {
            Some(o) => {
                let (v, sub) = o.split_once(',').unwrap_or((o.as_str(), ""));
                if !sub.trim().is_empty() {
                    return Err(no(format!(
                        "graph pie, over({o}): over()'s own options are not translated yet"
                    )));
                }
                let v = v.trim().to_string();
                if !is_name(&v) {
                    return Err(no(format!("over({v}): one variable is translated")));
                }
                Some(v)
            }
            None => None,
        };
        if over.is_some() && vars.len() > 1 {
            return Err(no(
                "graph pie with several variables and over() is not translated yet",
            ));
        }
        if over.is_none() && vars.is_empty() {
            return Err(no("graph pie needs variables or over() (Stata: r(100))"));
        }
        let rows = self.rows_for(c, out)?;
        self.helpers.insert("stata_graph");
        self.helpers.insert("stata_tab_values");
        let (stats, names) = if vars.is_empty() {
            (vec![r_text("count")], vec![r_text("")])
        } else {
            (
                vars.iter().map(|_| r_text("sum")).collect(),
                vars.iter().map(|v| r_text(v)).collect(),
            )
        };
        let data = match &over {
            Some(g) => format!(
                "local({{ b <- stata_bar_data({rows}, c({}), c({}), {}); data.frame(label = b$group, value = b$value) }})",
                stats.join(", "),
                names.join(", "),
                r_text(g)
            ),
            None => format!(
                "local({{ b <- stata_bar_data({rows}, c({}), c({})); l <- vapply(c({}), function(v) stata_varlab({rows}, v), \"\"); \
                 data.frame(label = factor(l, levels = unique(l)), value = b$value) }})",
                stats.join(", "),
                names.join(", "),
                names.join(", ")
            ),
        };
        out.line(format!("stata_pie <- {data}"));
        let layers = vec![
            "ggplot2::geom_col(data = stata_pie, ggplot2::aes(x = 1, y = value, fill = label), width = 1, position = ggplot2::position_stack(reverse = TRUE))".to_string(),
            "ggplot2::coord_polar(theta = \"y\", direction = -1)".to_string(),
            "ggplot2::theme(axis.text = ggplot2::element_blank(), axis.ticks = ggplot2::element_blank(), panel.grid = ggplot2::element_blank())".to_string(),
            "ggplot2::labs(fill = NULL)".to_string(),
        ];
        self.draw(c, opts, out, &[("over", 4)], &layers, "NULL", "NULL")
    }

    /// `graph matrix a b c [, half]`: a scatter of each variable against each
    /// other one, the row's variable up and the column's across, the names
    /// on the diagonal, and only the cells below it with `half`.
    fn graph_matrix(
        &mut self,
        c: &Command,
        rest: &str,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "graph matrix")?;
        if c.parts.weight.is_some() {
            return Err(no("weights in a graph are not translated yet"));
        }
        let vars: Vec<&str> = rest.split_whitespace().collect();
        if let Some(bad) = vars.iter().find(|v| !is_name(v)) {
            return Err(no(format!(
                "graph matrix {bad}: only variable names are translated"
            )));
        }
        if vars.len() < 2 {
            return Err(no(
                "graph matrix needs two variables or more (Stata: r(102))",
            ));
        }
        let rows = self.rows_for(c, out)?;
        self.helpers.insert("stata_graph");
        self.helpers.insert("stata_tab_values");
        let list = vars
            .iter()
            .map(|v| r_text(v))
            .collect::<Vec<_>>()
            .join(", ");
        out.line(format!(
            "stata_gm <- local({{ v <- c({list}); lab <- vapply(v, function(x) stata_varlab({rows}, x), \"\"); \
             k <- length(v); cells <- expand.grid(i = seq_len(k), j = seq_len(k)); cells <- cells[cells$i {} cells$j, ]; \
             pts <- do.call(rbind, lapply(seq_len(nrow(cells)), function(r) {{ i <- cells$i[r]; j <- cells$j[r]; \
               data.frame(row = factor(lab[i], levels = lab), col = factor(lab[j], levels = lab), x = stata_num({rows}[[v[j]]]), y = stata_num({rows}[[v[i]]])) }})); \
             mid <- vapply(v, function(x) mean(range(stata_num({rows}[[x]]), na.rm = TRUE)), 0); \
             list(points = pts, diag = data.frame(row = factor(lab, levels = lab), col = factor(lab, levels = lab), x = mid, y = mid, label = lab)) }})",
            if opts.has("half", 4) { ">" } else { "!=" }
        ));
        let layers = vec![
            "ggplot2::geom_point(data = stata_gm$points, ggplot2::aes(x = x, y = y), na.rm = TRUE)".to_string(),
            "ggplot2::geom_text(data = stata_gm$diag, ggplot2::aes(x = x, y = y, label = label))".to_string(),
            "ggplot2::facet_grid(rows = ggplot2::vars(row), cols = ggplot2::vars(col), scales = \"free\")".to_string(),
            "ggplot2::theme(strip.text = ggplot2::element_blank())".to_string(),
        ];
        self.draw(c, opts, out, &[("half", 4)], &layers, "NULL", "NULL")
    }

    fn graph_bar(
        &mut self,
        c: &Command,
        rest: &str,
        opts: &Opts,
        out: &mut Out,
        horizontal: bool,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "graph bar")?;
        // (stat) var var (stat) var ...; the mean unless a statistic is named
        let mut stat = "mean".to_string();
        let mut stats = Vec::new();
        let mut vars = Vec::new();
        for w in rest.split_whitespace() {
            if let Some(s) = w.strip_prefix('(').and_then(|s| s.strip_suffix(')')) {
                let ok = matches!(
                    s,
                    "mean" | "median" | "sum" | "count" | "min" | "max" | "sd" | "percent" | "asis"
                ) || s
                    .strip_prefix('p')
                    .is_some_and(|k| k.parse::<u32>().is_ok_and(|k| (1..=99).contains(&k)));
                if !ok {
                    return Err(no(format!("graph bar ({s}) is not translated yet")));
                }
                stat = s.to_string();
            } else if is_name(w) {
                stats.push(r_text(&stat));
                vars.push(r_text(w));
            } else {
                return Err(no(format!(
                    "graph bar {w}: only variable names are translated"
                )));
            }
        }
        if stat == "percent" && !vars.is_empty() {
            return Err(no("graph bar (percent) of variables is not translated yet"));
        }
        if vars.is_empty() {
            // (count) and (percent) alone count the rows; another statistic
            // needs a variable (Stata: r(100))
            if stat != "count" && stat != "percent" {
                return Err(no("graph bar needs a variable (Stata: r(100))"));
            }
            stats.push(r_text(&stat));
            vars.push(r_text(""));
        }
        let mut over = Vec::new();
        // over(v, sort(var | #) descending): the groups in the order of a bar's values
        let mut sort: Option<String> = None;
        let mut descending = false;
        for o in c.options.iter().filter(|o| o.is("over", 4)) {
            let arg = o.arg();
            let (v, sub) = arg.split_once(',').unwrap_or((arg.as_str(), ""));
            let v = v.trim().to_string();
            if !is_name(&v) {
                return Err(no(format!("over({v}): one variable is translated")));
            }
            for so in crate::lang::syntax::options(sub) {
                if so.is("sort", 4) {
                    let s = so.arg().trim().to_string();
                    let series = if let Ok(k) = s.parse::<usize>() {
                        k
                    } else {
                        vars.iter().position(|x| *x == r_text(&s)).map(|i| i + 1).ok_or_else(|| {
                            no(format!("over(..., sort({s})): sorting by a variable not drawn is not translated yet"))
                        })?
                    };
                    if series == 0 || series > vars.len() {
                        return Err(no(format!("over(..., sort({s})): no such bar")));
                    }
                    sort = Some(series.to_string());
                } else if so.is("descending", 4) {
                    descending = true;
                }
            }
            over.push(r_text(&v));
        }
        if sort.is_some() && over.len() != 1 {
            return Err(no(
                "graph bar, over(..., sort()) with other than one over() is not translated yet",
            ));
        }
        // blabel(bar [, format(fmt)]): each bar's value at its end
        let blabel = match opts.arg("blabel", 4) {
            None => None,
            Some(b) => {
                let (what, sub) = b.split_once(',').unwrap_or((b.as_str(), ""));
                if what.trim() != "bar" {
                    return Err(no(format!(
                        "blabel({}) is not translated yet (blabel(bar) is)",
                        what.trim()
                    )));
                }
                let mut fmt = "%9.0g".to_string();
                for so in crate::lang::syntax::options(sub) {
                    if so.is("format", 3) {
                        fmt = so.arg().trim().to_string();
                    }
                }
                Some(r_text(&fmt))
            }
        };
        let rows = self.rows_for(c, out)?;
        self.helpers.insert("stata_graph");
        self.helpers.insert("stata_tab_values");
        out.line(format!(
            "stata_bar <- stata_bar_data({rows}, c({}), c({}), over = c({}))",
            stats.join(", "),
            vars.join(", "),
            over.join(", ")
        ));
        let position = if opts.has("stack", 5) {
            "stack"
        } else {
            "dodge"
        };
        if let Some(k) = &sort {
            out.line(format!(
                "stata_bar$group <- factor(stata_bar$group, levels = local({{ s <- stata_bar[stata_bar$series == levels(stata_bar$series)[{k}], ]; as.character(s$group[order(s$value, decreasing = {})]) }}))",
                super::stats::r_logical(descending)
            ));
        }
        let mut layers = vec![format!(
            "ggplot2::geom_col(data = stata_bar, ggplot2::aes(x = group, y = value, fill = series), position = {})", if position == "stack" { "ggplot2::position_stack(reverse = TRUE)" } else { "\"dodge\"" }
        )];
        if let Some(fmt) = &blabel {
            self.helpers.insert("stata_fmt");
            let dodge = if position == "stack" {
                "ggplot2::position_stack(vjust = 1, reverse = TRUE)"
            } else {
                "ggplot2::position_dodge(width = 0.9)"
            };
            let just = if horizontal {
                "hjust = -0.2"
            } else {
                "vjust = -0.4"
            };
            layers.push(format!(
                "ggplot2::geom_text(data = stata_bar, ggplot2::aes(x = group, y = value, group = series, label = vapply(value, function(v) trimws(stata_fmt(v, {fmt})), \"\")), position = {dodge}, {just})"
            ));
            // room for the labels past the longest bar
            layers.push(
                "ggplot2::expand_limits(y = max(stata_bar$value, na.rm = TRUE) * 1.08)".to_string(),
            );
        }
        layers.push("ggplot2::labs(fill = NULL)".to_string());
        // legend(order(3 "text" 2 "text" ...)): those series, in that order, with those texts
        if let Some(l) = opts.arg("legend", 3) {
            let lo = crate::lang::syntax::options(&l);
            if let Some(o) = lo.iter().find(|o| o.is("order", 5)) {
                let mut keys = Vec::new();
                let mut texts = Vec::new();
                let w = super::coefplot::quoted_words_kept(&o.raw.clone().unwrap_or_default());
                let mut it = w.into_iter().peekable();
                while let Some(n) = it.next() {
                    let k: usize = n
                        .parse()
                        .ok()
                        .filter(|k| *k >= 1 && *k <= vars.len())
                        .ok_or_else(|| {
                            no(format!(
                                "graph bar, legend(order({n})): a series' number is translated"
                            ))
                        })?;
                    keys.push(k.to_string());
                    texts.push(match it.next_if(|t| t.starts_with('"')) {
                        Some(t) => r_text(t.trim_matches('"')),
                        None => format!("levels(stata_bar$series)[{k}]"),
                    });
                }
                if !keys.is_empty() {
                    layers.push(format!(
                        "ggplot2::scale_fill_discrete(breaks = levels(stata_bar$series)[c({})], labels = c({}))",
                        keys.join(", "),
                        texts.join(", ")
                    ));
                }
            }
        }
        // one series: no legend, as Stata draws it
        if vars.len() == 1 {
            layers.push("ggplot2::theme(legend.position = \"none\")".to_string());
        }
        if horizontal {
            // hbar: the first group at the top, as Stata draws it
            layers.push("ggplot2::scale_x_discrete(limits = rev)".to_string());
            layers.push("ggplot2::coord_flip()".to_string());
        }
        let known = [("over", 4), ("stack", 5)];
        // the value axis is titled with the statistic when there is one series
        let ytitle = if vars.len() == 1 {
            if stat == "percent" {
                r_text("percent")
            } else if vars.first() == Some(&r_text("")) {
                "NULL".to_string()
            } else {
                "levels(stata_bar$series)[1]".to_string()
            }
        } else {
            "NULL".to_string()
        };
        self.draw(c, opts, out, &known, &layers, "NULL", &ytitle)
    }

    /// `graph box`: each variable's quartiles within each `over()` group,
    /// whiskers to the adjacent values (the farthest within 1.5 IQR of the
    /// box) and the values beyond them as points.
    fn graph_box(
        &mut self,
        c: &Command,
        rest: &str,
        opts: &Opts,
        out: &mut Out,
        horizontal: bool,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "graph box")?;
        let mut vars = Vec::new();
        for w in rest.split_whitespace() {
            if !is_name(w) {
                return Err(no(format!(
                    "graph box {w}: only variable names are translated"
                )));
            }
            vars.push(r_text(w));
        }
        if vars.is_empty() {
            return Err(no("graph box needs a variable"));
        }
        let mut over = Vec::new();
        for o in c.options.iter().filter(|o| o.is("over", 4)) {
            let arg = o.arg();
            let (v, sub) = arg.split_once(',').unwrap_or((arg.as_str(), ""));
            let v = v.trim().to_string();
            if !is_name(&v) {
                return Err(no(format!("over({v}): one variable is translated")));
            }
            // relabel() and label() only rename the groups on the axis
            for so in crate::lang::syntax::options(sub) {
                if !(so.is("relabel", 5) || so.is("label", 3)) {
                    return Err(no(format!(
                        "graph box over() with {} is not translated yet",
                        so.name
                    )));
                }
            }
            over.push(r_text(&v));
        }
        let rows = self.rows_for(c, out)?;
        self.helpers.insert("stata_graph");
        self.helpers.insert("stata_tab_values");
        out.line(format!(
            "stata_box <- stata_box_data({rows}, c({}), over = c({}))",
            vars.join(", "),
            over.join(", ")
        ));
        let mut layers = vec![
            "ggplot2::geom_boxplot(data = stata_box$box, ggplot2::aes(x = group, ymin = lo, lower = q1, middle = q2, upper = q3, ymax = hi, fill = series), stat = \"identity\")".to_string(),
            "ggplot2::geom_point(data = stata_box$out, ggplot2::aes(x = group, y = value, group = series), position = ggplot2::position_dodge(width = 0.9))".to_string(),
            "ggplot2::labs(fill = NULL)".to_string(),
        ];
        if horizontal {
            layers.push("ggplot2::coord_flip()".to_string());
        }
        out.caveat("graph box: the quartiles are Stata's percentiles; relabel() and the look of the boxes are R's");
        let known = [("over", 4)];
        self.draw(c, opts, out, &known, &layers, "NULL", "NULL")
    }

    fn graph_export(
        &mut self,
        _c: &Command,
        rest: &str,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[
            ("replace", 7),
            ("width", 5),
            ("height", 6),
            ("as", 2),
            ("name", 4),
            // a preview picture, fonts and colours in the file: its look only
            ("preview", 7),
            ("fontface", 8),
            ("cmyk", 4),
        ]) {
            return Err(no(format!(
                "graph export with the option {o} is not translated yet"
            )));
        }
        if !self.graph_drawn {
            return Err(no(
                "graph export needs a graph drawn earlier in this do-file",
            ));
        }
        let file = rest.trim();
        if file.is_empty() {
            return Err(no("graph export needs a file name"));
        }
        let path = file_name(file, "png");
        let ext = path.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
        if !matches!(
            ext.as_str(),
            "png" | "pdf" | "svg" | "eps" | "jpg" | "jpeg" | "tif" | "tiff"
        ) {
            return Err(no(format!("graph export to .{ext} is not translated")));
        }
        if !opts.has("replace", 7) {
            out.line(format!(
                "if (file.exists({p})) stop(\"file \", {p}, \" already exists\")",
                p = r_text(&path)
            ));
        }
        let mut size = String::new();
        for (o, m, arg) in [("width", 5, "width"), ("height", 6, "height")] {
            if let Some(v) = opts.arg(o, m) {
                let n: f64 = v
                    .trim()
                    .parse()
                    .map_err(|_| no(format!("{o}({v}) needs a number")))?;
                size.push_str(&format!(", {arg} = {}", r_number(n)));
            }
        }
        if !size.is_empty() {
            size.push_str(", units = \"px\", dpi = 96");
        }
        out.package("ggplot2");
        out.line(format!(
            "ggplot2::ggsave({}, plot = stata_graph{size})",
            r_text(&path)
        ));
        out.caveat("the graph is written by ggplot2, so it looks like R's, not Stata's");
        Ok(())
    }

    // ---------------- shared ----------------

    /// The one variable of `histogram` or `kdensity`.
    fn graph_var(&self, c: &Command, what: &str) -> Result<String, Untranslated> {
        let words: Vec<&str> = c.parts.main.split_whitespace().collect();
        match words.as_slice() {
            [v] if is_name(v) => Ok(v.to_string()),
            _ => Err(no(format!("{what} takes one variable"))),
        }
    }

    /// Puts the layers on one ggplot2 graph with the titles, prints it and
    /// keeps it as `stata_graph`.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
        own: &[(&str, usize)],
        layers: &[String],
        xtitle: &str,
        ytitle: &str,
    ) -> Result<(), Untranslated> {
        let mut known: Vec<(&str, usize)> = LOOK.to_vec();
        known.extend(own.iter().copied());
        known.extend([("xline", 5), ("yline", 5), ("saving", 4), ("text", 4)]);
        known.extend(TITLES.iter().map(|(o, m, _)| (*o, *m)));
        if let Some(o) = opts.other(&known) {
            return Err(no(format!(
                "{} with the option {o} is not translated yet",
                c.name
            )));
        }
        let mut labs = vec![format!("x = {xtitle}"), format!("y = {ytitle}")];
        for (o, m, name) in TITLES {
            if let Some(t) = opts.arg(o, *m) {
                let t = t.trim().trim_matches('"').to_string();
                labs.retain(|l| !l.starts_with(&format!("{name} =")));
                labs.push(format!("{name} = {}", r_text(&t)));
            }
        }
        let mut plot = vec!["ggplot2::ggplot()".to_string()];
        plot.extend(layers.iter().cloned());
        // xline() and yline(): lines at the numbers before any comma
        for (o, geom, at) in [
            ("xline", "geom_vline", "xintercept"),
            ("yline", "geom_hline", "yintercept"),
        ] {
            for opt in opts.0.iter().filter(|x| x.is(o, 5)) {
                let arg = opt.arg();
                let nums: Vec<String> = arg
                    .split(',')
                    .next()
                    .unwrap_or("")
                    .split_whitespace()
                    .map(|w| {
                        w.parse::<f64>()
                            .map(r_number)
                            .map_err(|_| no(format!("{o}({arg}): numbers are translated")))
                    })
                    .collect::<Result<_, _>>()?;
                if nums.is_empty() {
                    return Err(no(format!("{o}() needs a number")));
                }
                plot.push(format!("ggplot2::{geom}({at} = c({}))", nums.join(", ")));
            }
        }
        // text(y x "text" [y x "text" ...] [, look]): the text at those coordinates
        for opt in opts.0.iter().filter(|x| x.is("text", 4)) {
            let raw = opt.raw.clone().unwrap_or_default();
            let (spec, _) = super::coefplot::split_top_comma(&raw);
            for (y, x, t) in text_triples(spec)? {
                plot.push(format!(
                    "ggplot2::annotate(\"text\", x = {}, y = {}, label = {})",
                    r_number(x),
                    r_number(y),
                    r_text(&t)
                ));
            }
        }
        plot.push(format!("ggplot2::labs({})", labs.join(", ")));
        if let Some(l) = opts.arg("legend", 3) {
            if l.trim() == "off" {
                plot.push("ggplot2::theme(legend.position = \"none\")".to_string());
            }
        }
        if let Some(b) = opts.arg("by", 2) {
            let v = b.split(',').next().unwrap_or("").trim();
            if !is_name(v) {
                return Err(no(format!("by({b}): one variable is translated")));
            }
            plot.push(format!(
                "ggplot2::facet_wrap(ggplot2::vars(stata_tab_values(.data[[{}]])))",
                r_text(v)
            ));
        }
        out.package("ggplot2");
        out.line(format!("stata_graph <- {}", plot.join(" +\n  ")));
        self.keep_graph(opts, out)?;
        if !self.quiet(c) && !opts.has("nodraw", 6) {
            out.line("print(stata_graph)");
        }
        let looks: Vec<String> = c
            .options
            .iter()
            .filter(|o| {
                LOOK.iter().any(|(n, m)| o.is(n, *m))
                    && !(o.is("legend", 3) && o.arg().trim() == "off")
            })
            .map(|o| o.name.clone())
            .collect();
        if !looks.is_empty() {
            out.caveat(format!(
                "drawn with ggplot2's own look: {} only change how Stata's graph looks and are not \
                 carried over",
                looks.join(", ")
            ));
        }
        out.caveat("drawn by ggplot2, so it looks like R's graphs, not Stata's; what it shows is worked out as Stata does");
        self.graph_drawn = true;
        Ok(())
    }
}

/// text()'s places: `y x "text"` again and again, the text in quotes or compound
/// quotes; one text to a place.
fn text_triples(spec: &str) -> Result<Vec<(f64, f64, String)>, Untranslated> {
    let mut toks: Vec<(bool, String)> = Vec::new();
    let chars: Vec<char> = spec.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars.get(i).copied().unwrap_or(' ');
        if c.is_whitespace() {
            i += 1;
        } else if c == '`' && chars.get(i + 1) == Some(&'"') {
            // `"text"': up to the "' that closes it
            let mut j = i + 2;
            let mut s = String::new();
            while j < chars.len()
                && !(chars.get(j) == Some(&'"') && chars.get(j + 1) == Some(&'\''))
            {
                s.push(chars.get(j).copied().unwrap_or(' '));
                j += 1;
            }
            toks.push((true, s));
            i = j + 2;
        } else if c == '"' {
            let mut j = i + 1;
            let mut s = String::new();
            while j < chars.len() && chars.get(j) != Some(&'"') {
                s.push(chars.get(j).copied().unwrap_or(' '));
                j += 1;
            }
            toks.push((true, s));
            i = j + 1;
        } else {
            let mut j = i;
            let mut s = String::new();
            while j < chars.len() && !chars.get(j).is_some_and(|c| c.is_whitespace()) {
                s.push(chars.get(j).copied().unwrap_or(' '));
                j += 1;
            }
            toks.push((false, s));
            i = j;
        }
    }
    let mut out = Vec::new();
    for t in toks.chunks(3) {
        let [(false, y), (false, x), (true, s)] = t else {
            return Err(no(format!(
                "text({spec}): places as y x \"text\", one text to each, are translated"
            )));
        };
        let num = |v: &str| {
            v.parse::<f64>()
                .map_err(|_| no(format!("text({spec}): {v} is not a number")))
        };
        out.push((num(y)?, num(x)?, s.clone()));
    }
    if out.is_empty() {
        return Err(no("text() needs a place and a text"));
    }
    Ok(out)
}

/// The kind of heights a histogram has, and its y title.
fn hist_type(opts: &Opts) -> (&'static str, &'static str) {
    if opts.has("fraction", 4) {
        ("fraction", "Fraction")
    } else if opts.has("frequency", 4) {
        ("frequency", "Frequency")
    } else if opts.has("percent", 1) {
        ("percent", "Percent")
    } else {
        ("density", "Density")
    }
}

/// A confidence level as Stata writes it in a legend: `95`, `99.5`.
fn fmt_level(p: f64) -> String {
    if p.fract() == 0.0 {
        format!("{}", p as i64)
    } else {
        format!("{p}")
    }
}

/// The layers of `twoway (a) (b)` or `twoway a || b`.
/// The options of one kind of twoway plot that change what it shows.
fn kind_options(kind: &str) -> Vec<(&'static str, usize)> {
    match kind {
        "histogram" => vec![
            ("bin", 3),
            ("width", 1),
            ("start", 5),
            ("discrete", 1),
            ("density", 3),
            ("fraction", 4),
            ("frequency", 4),
            ("percent", 1),
        ],
        "kdensity" => vec![("kernel", 1), ("bwidth", 1), ("n", 1)],
        "lowess" => vec![("bwidth", 2)],
        "bar" => vec![("base", 4), ("vertical", 4)],
        "area" => vec![("base", 4)],
        "lfit" | "qfit" => vec![("n", 1), ("range", 1)],
        "lfitci" | "qfitci" => vec![
            ("n", 1),
            ("range", 1),
            ("level", 3),
            ("stdp", 4),
            ("stdf", 4),
        ],
        "rcap" | "rspike" => vec![("horizontal", 3), ("vertical", 4)],
        _ => Vec::new(),
    }
}

/// `plot ..., opts || plot ..., opts`: the plots, split at `||` outside
/// quotes and parentheses.
pub(super) fn split_plots(text: &str) -> Result<Vec<String>, Untranslated> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let (mut depth, mut quote) = (0usize, false);
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '"' => quote = !quote,
            '(' if !quote => depth += 1,
            ')' if !quote => depth = depth.saturating_sub(1),
            '|' if !quote && depth == 0 && chars.peek() == Some(&'|') => {
                chars.next();
                out.push(std::mem::take(&mut cur).trim().to_string());
                continue;
            }
            _ => {}
        }
        cur.push(ch);
    }
    out.push(cur.trim().to_string());
    out.retain(|s| !s.is_empty());
    if quote || depth != 0 {
        return Err(no("twoway: a quote or ( without its end"));
    }
    Ok(out)
}

fn layer_texts(main: &str) -> Result<Vec<String>, Untranslated> {
    let main = main.trim();
    if main.starts_with('(') {
        let mut out = Vec::new();
        let mut depth = 0usize;
        let mut cur = String::new();
        let mut quote = false;
        for ch in main.chars() {
            match ch {
                '"' => {
                    quote = !quote;
                    if depth > 0 {
                        cur.push(ch);
                    }
                }
                '(' if !quote => {
                    if depth > 0 {
                        cur.push(ch);
                    }
                    depth += 1;
                }
                ')' if !quote => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        out.push(std::mem::take(&mut cur).trim().to_string());
                    } else {
                        cur.push(ch);
                    }
                }
                _ if depth > 0 => cur.push(ch),
                c if c.is_whitespace() => {}
                _ => return Err(no("twoway: text between the (plots) is not translated")),
            }
        }
        if depth != 0 {
            return Err(no("twoway: a ( without its )"));
        }
        Ok(out)
    } else {
        Ok(main
            .split("||")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect())
    }
}

impl Session {
    /// A graph just made is kept in memory under `name(n[, replace])`, or as
    /// `Graph` without one, as Stata 19 keeps it (for `graph dir`, `graph
    /// display`, `graph combine` and `graph drop`).
    pub(super) fn remember_graph(
        &mut self,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let name = match opts.arg("name", 4) {
            Some(n) => {
                let (name, _) = super::coefplot::split_top_comma(&n);
                let name = name.trim().to_string();
                if !is_name(&name) {
                    return Err(no(format!("name({n}): a graph name is a plain name")));
                }
                name
            }
            None => "Graph".to_string(),
        };
        out.line(format!(
            "if (!exists(\"stata_graphs\")) stata_graphs <- list(); stata_graphs[[{}]] <- stata_graph",
            r_text(&name)
        ));
        Ok(())
    }

    /// After a graph is drawn: `name(n[, replace])` keeps it by name for
    /// `graph combine`, `saving(f[, replace])` in a file (R's form under the
    /// .gph name, which Stata cannot open).
    pub(super) fn keep_graph(&mut self, opts: &Opts, out: &mut Out) -> Result<(), Untranslated> {
        self.remember_graph(opts, out)?;
        if let Some(s) = opts.arg("saving", 4) {
            let (name, o) = super::coefplot::split_top_comma(&s);
            let file = file_name(name.trim(), "gph");
            let replace = crate::lang::syntax::options(o)
                .iter()
                .any(|x| x.is("replace", 7));
            self.save_graph_line(&file, replace, out);
        }
        Ok(())
    }

    fn save_graph_line(&mut self, file: &str, replace: bool, out: &mut Out) {
        if !replace {
            out.line(format!(
                "if (file.exists({p})) stop(\"file {p2} already exists (Stata: r(602))\", call. = FALSE)",
                p = r_text(file),
                p2 = file.replace('"', "")
            ));
        }
        out.line(format!("saveRDS(stata_graph, {})", r_text(file)));
        out.caveat("the graph is kept in R's form under the .gph name, which Stata cannot open");
    }

    /// `graph save [name] file[.gph] [, replace]`.
    fn graph_save(&mut self, rest: &str, opts: &Opts, out: &mut Out) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[("replace", 7), ("asis", 4)]) {
            return Err(no(format!(
                "graph save with the option {o} is not translated yet"
            )));
        }
        let words: Vec<&str> = rest.split_whitespace().collect();
        let (graph, file) = match words.as_slice() {
            [f] => (None, *f),
            [g, f] if is_name(g) => (Some(*g), *f),
            _ => {
                return Err(no(
                    "graph save takes a file name, after a graph's name if one is given",
                ))
            }
        };
        match graph {
            Some(g) => out.line(format!(
                "stata_graph <- if (exists(\"stata_graphs\") && !is.null(stata_graphs[[{0}]])) stata_graphs[[{0}]] else stop(\"graph {1} not found (Stata: r(111))\", call. = FALSE)",
                r_text(g),
                g
            )),
            None if !self.graph_drawn => {
                return Err(no("graph save needs a graph drawn earlier in this do-file"))
            }
            None => {}
        }
        let file = file_name(file, "gph");
        self.save_graph_line(&file, opts.has("replace", 7), out);
        Ok(())
    }

    /// `graph combine g1 g2 ...` (graphs kept by name, or .gph files RCraft
    /// saved): in a grid (ggplot2 and gtable), `cols()` and `rows()` as asked.
    fn graph_combine(
        &mut self,
        c: &Command,
        rest: &str,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let mut known: Vec<(&str, usize)> = LOOK.to_vec();
        known.extend([
            ("cols", 3),
            ("rows", 3),
            ("saving", 4),
            ("xsize", 5),
            ("ysize", 5),
            ("ycommon", 5),
            ("xcommon", 5),
            ("imargin", 4),
            // l1title() and l2title(): titles on the left (Stata 19 takes l1() and l2())
            ("l1title", 2),
            ("l2title", 2),
        ]);
        known.extend(TITLES.iter().map(|(o, m, _)| (*o, *m)));
        if let Some(o) = opts.other(&known) {
            return Err(no(format!(
                "graph combine with the option {o} is not translated yet"
            )));
        }
        let parts: Vec<&str> = rest.split_whitespace().collect();
        if parts.is_empty() {
            return Err(no("graph combine needs the graphs to combine"));
        }
        let mut pieces = Vec::new();
        for p in &parts {
            let unq = p.trim_matches('"');
            if is_name(unq) {
                pieces.push(format!(
                    "(if (exists(\"stata_graphs\") && !is.null(stata_graphs[[{0}]])) stata_graphs[[{0}]] else if (file.exists({1})) readRDS({1}) else stop(\"graph {2} not found (Stata: r(111))\", call. = FALSE))",
                    r_text(unq),
                    r_text(&file_name(unq, "gph")),
                    unq
                ));
            } else {
                let f = file_name(unq, "gph");
                pieces.push(format!(
                    "(if (file.exists({0})) readRDS({0}) else stop(\"file {1} not found (Stata: r(601))\", call. = FALSE))",
                    r_text(&f),
                    f
                ));
            }
        }
        let mut layout = Vec::new();
        for (o, arg) in [("cols", "ncol"), ("rows", "nrow")] {
            if let Some(v) = opts.arg(o, 3) {
                let n: u32 = v
                    .trim()
                    .parse()
                    .map_err(|_| no(format!("{o}({v}) needs a whole number")))?;
                layout.push(format!("{arg} = {n}"));
            }
        }
        out.package("ggplot2");
        self.helpers.insert("stata_combine");
        // a title's text is what comes before its look options (`l2(text, c(black))`)
        let text = |o: &str, m: usize| match opts.raw(o, m) {
            Some(t) => r_text(&title_text(&t)),
            None => "NULL".into(),
        };
        let title = text("title", 1);
        let (l1, l2) = (text("l1title", 2), text("l2title", 2));
        out.line(format!(
            "stata_graph <- stata_combine(list({}){}, title = {title}, l1 = {l1}, l2 = {l2})",
            pieces.join(", "),
            layout.iter().map(|l| format!(", {l}")).collect::<String>()
        ));
        self.keep_graph(opts, out)?;
        if !self.quiet(c) && !opts.has("nodraw", 6) {
            out.line("grid::grid.newpage(); grid::grid.draw(stata_graph)");
        }
        self.graph_drawn = true;
        out.caveat("the graphs are put side by side by ggplot2 and gtable, so the layout looks like R's, not Stata's");
        Ok(())
    }
}

/// A title's text: what comes before a comma outside quotes and parentheses (the
/// textbox options after it, `c(black) si(medium)`, are Stata's look), without
/// the quotes around it.
fn title_text(arg: &str) -> String {
    let (text, _) = super::coefplot::split_top_comma(arg);
    text.trim().trim_matches('"').to_string()
}

/// `graph combine`'s R: the graphs (ggplots, or graphs combined already) in a
/// grid, by rows, as many columns as asked or Stata's near-square default.
pub(super) const COMBINE_HELPER: &str = r##"stata_combine <- function(plots, ncol = NULL, nrow = NULL, title = NULL, l1 = NULL, l2 = NULL) {
  n <- length(plots)
  if (is.null(ncol) && is.null(nrow)) ncol <- ceiling(sqrt(n))
  if (is.null(ncol)) ncol <- ceiling(n / nrow)
  if (is.null(nrow)) nrow <- ceiling(n / ncol)
  grobs <- lapply(plots, function(p) if (inherits(p, "gtable")) p else ggplot2::ggplotGrob(p))
  while (length(grobs) < nrow * ncol) grobs <- c(grobs, list(grid::nullGrob()))
  g <- gtable::gtable_matrix("stata_combine", matrix(grobs[seq_len(nrow * ncol)], nrow, ncol, byrow = TRUE),
                             widths = grid::unit(rep(1, ncol), "null"), heights = grid::unit(rep(1, nrow), "null"))
  # l1title() beside the graphs, l2title() to the left of it, read upwards
  for (left in list(l1, l2)) if (!is.null(left)) {
    g <- gtable::gtable_add_cols(g, grid::unit(2, "lines"), pos = 0)
    g <- gtable::gtable_add_grob(g, grid::textGrob(left, rot = 90), t = 1, b = nrow, l = 1, name = "stata_left_title")
  }
  if (!is.null(title)) {
    g <- gtable::gtable_add_rows(g, grid::unit(2, "lines"), pos = 0)
    g <- gtable::gtable_add_grob(g, grid::textGrob(title), t = 1, l = 1, r = ncol(g))
  }
  g
}"##;
