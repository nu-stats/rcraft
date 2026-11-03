//! P2B.5: Stata's graph commands drawn with ggplot2.
//!
//! `histogram`, `kdensity`, `scatter`, `line`, `connected`, `twoway` (with
//! `scatter`, `line`, `connected`, `lfit`, `qfit`, `lowess`, `histogram`,
//! `kdensity`, `bar` and `rarea` layers, in `( )` or split at `||`),
//! `lowess`, `graph bar`/`hbar`, `graph box`/`hbox`, `graph export` and
//! `graph drop _all`.
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
//! - `lfit` and `qfit`: least squares lines, `qfit` over 100 points;
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
  l <- attr(d[[v]], "label")
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
stata_fitline <- function(d, y, x, degree = 1, n = 100) {
  xv <- stata_num(d[[x]])
  yv <- stata_num(d[[y]])
  ok <- !is.na(xv) & !is.na(yv)
  g <- seq(min(xv[ok]), max(xv[ok]), length.out = n)
  X <- if (degree == 1) cbind(1, xv[ok]) else cbind(1, xv[ok], xv[ok]^2)
  b <- solve(crossprod(X), crossprod(X, yv[ok]))
  G <- if (degree == 1) cbind(1, g) else cbind(1, g, g^2)
  data.frame(x = g, y = drop(G %*% b))
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
    val <- tapply(x, key, function(v) f(stats[j], v))
    data.frame(group = factor(names(val), levels = levels(key)), series = if (nzchar(vars[j])) paste0(stats[j], " of ", vars[j]) else stats[j],
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
}"#;

/// Options that change only how a graph looks; ggplot2's defaults stand in.
pub(super) const LOOK: &[(&str, usize)] = &[
    ("msymbol", 2),
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
            // generate(x d): the points and the densities as new variables
            let names: Vec<&str> = g.split_whitespace().collect();
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
        let ytitle = match first.ys.first() {
            Some(y) if same_y => format!("stata_varlab({}, {})", first.rows, r_text(y)),
            _ => "NULL".to_string(),
        };
        // several layers (or y variables) get a legend, labelled as Stata labels them
        let legend = parsed.len() > 1 || parsed.iter().any(|l| l.ys.len() > 1);
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
        } else if w == "rarea" {
            "rarea"
        } else {
            return Err(no(format!(
                "twoway {w} is not translated yet (scatter, line, connected, lfit, qfit, lowess, \
                 histogram, kdensity, bar and rarea are)"
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
                    if matches!(kind, "lfit" | "qfit" | "lowess" | "bar") && ys.len() != 1 {
                        return Err(no(format!("twoway {kind} takes one y and one x")));
                    }
                    if kind == "rarea" && ys.len() != 2 {
                        return Err(no("twoway rarea takes two y variables and one x"));
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
            "lfit" | "qfit" => {
                let y = r_text(l.ys.first().map(String::as_str).unwrap_or(""));
                let deg = if l.kind == "lfit" { 1 } else { 2 };
                g.push(format!(
                    "ggplot2::geom_line(data = stata_fitline({rows}, {y}, {x}, degree = {deg}), ggplot2::aes(x = x, y = y{}))",
                    fixed("Fitted values".to_string())
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
        if sub == "box" || sub == "hbox" {
            return self.graph_box(c, rest, opts, out, sub == "hbox");
        }
        if abbrev(sub, "export", 3) {
            return self.graph_export(c, rest, opts, out);
        }
        if sub == "drop" {
            // the graphs Stata keeps by name; R keeps only the last, which
            // `graph drop _all` leaves nothing of to export
            let names = rest.trim();
            if names != "_all" {
                return Err(no(
                    "graph drop of graphs by name is not translated yet (graph drop _all is)",
                ));
            }
            self.graph_drawn = false;
            out.line("if (exists(\"stata_graph\")) rm(stata_graph)");
            return Ok(());
        }
        Err(no(format!("graph {sub} is not translated yet")))
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
                    "mean" | "median" | "sum" | "count" | "min" | "max" | "sd"
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
        if vars.is_empty() {
            // (count) alone counts the rows; another statistic needs a
            // variable (Stata: r(100))
            if stat != "count" {
                return Err(no("graph bar needs a variable (Stata: r(100))"));
            }
            stats.push(r_text(&stat));
            vars.push(r_text(""));
        }
        let mut over = Vec::new();
        for o in c.options.iter().filter(|o| o.is("over", 4)) {
            let v = o.arg();
            let v = v.split(',').next().unwrap_or("").trim().to_string();
            if !is_name(&v) {
                return Err(no(format!("over({v}): one variable is translated")));
            }
            over.push(r_text(&v));
        }
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
        let mut layers = vec![format!(
            "ggplot2::geom_col(data = stata_bar, ggplot2::aes(x = group, y = value, fill = series), position = \"{position}\")"
        )];
        layers.push("ggplot2::labs(fill = NULL)".to_string());
        if horizontal {
            layers.push("ggplot2::coord_flip()".to_string());
        }
        let known = [("over", 4), ("stack", 5)];
        self.draw(c, opts, out, &known, &layers, "NULL", "NULL")
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
    fn draw(
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
        known.extend([("xline", 5), ("yline", 5)]);
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
