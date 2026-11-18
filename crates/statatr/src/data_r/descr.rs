//! Batch C of the StatCraft port, descriptive commands as Stata 19 does them:
//! `pctile` (here), and beside it `cumul`, `pcorr`, `bitest`, `robvar`,
//! `swilk`, `inspect`, `dstdize` and `pwmean` (`plan/stata/pctile.md` and the
//! notes beside it).

use super::weights::W;
use super::*;

fn stop(out: &mut Out, msg: &str) {
    out.line(format!("stop({}, call. = FALSE)", r_text(msg)));
}

/// The scalars `r(r1)`, `r(r2)`, ... `pctile` leaves, for `count` cut points.
/// The names are kept for the life of the program, one list per count (at
/// most 199 lists), so a long session does not grow.
fn pctile_names(count: usize) -> &'static [&'static str] {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};
    static NAMES: OnceLock<Mutex<HashMap<usize, &'static [&'static str]>>> = OnceLock::new();
    let cell = NAMES.get_or_init(|| Mutex::new(HashMap::new()));
    let Ok(mut map) = cell.lock() else {
        return &[];
    };
    if let Some(found) = map.get(&count) {
        return found;
    }
    let list: Vec<&'static str> = (1..=count)
        .map(|i| &*Box::leak(format!("r{i}").into_boxed_str()))
        .collect();
    let list: &'static [&'static str] = Box::leak(list.into_boxed_slice());
    map.insert(count, list);
    list
}

/// `pctile`: the cut points of `nquantiles()` equal groups, written to the
/// first rows of the new variable (`genp()` has their percentages) and left
/// in `r(r1)`, `r(r2)`, ...
///
/// A percentile is the first value whose running weight reaches `W p / 100`,
/// or the average of it and the next value when the running weight is
/// exactly that (Stata's own definition, the one `summarize, detail` uses;
/// without weights each row weighs 1). `altdef` is the interpolation at
/// `(n + 1) p / 100`. The limit on `nquantiles()` is the number of rows of
/// the data plus one, not the size of the sample.
pub(super) const PCTILE_HELPER: &str = r##"stata_pctile <- function(x, nq, w = NULL, altdef = FALSE, rows = 0) {
  if (nq > rows + 1) stop("nquantiles() must be less than or equal to number of observations plus one (Stata: r(198))", call. = FALSE)
  x <- as.numeric(x)
  w <- if (is.null(w)) rep(1, length(x)) else as.numeric(w)
  ok <- !is.na(x)
  if (!any(ok)) stop("no observations (Stata: r(2000))", call. = FALSE)
  o <- order(x[ok], method = "radix")
  x <- x[ok][o]
  w <- w[ok][o]
  n <- length(x)
  p <- 100 * seq_len(nq - 1) / nq
  if (altdef) {
    P <- (n + 1) * p / 100
    j <- floor(P + 4 * .Machine$double.eps)
    v <- ifelse(j < 1, x[1], ifelse(j >= n, x[n], x[pmax(j, 1)] + (P - j) * (x[pmin(j + 1, n)] - x[pmax(j, 1)])))
  } else {
    cw <- cumsum(w)
    P <- cw[n] * p / 100
    v <- vapply(P, function(q) {
      tol <- 1e-11 * max(1, abs(q))
      i <- which(cw >= q - tol)[1]
      if (is.na(i)) return(x[n])
      if (abs(cw[i] - q) <= tol && i < n) (x[i] + x[i + 1]) / 2 else x[i]
    }, 0)
  }
  r <- as.list(v)
  names(r) <- paste0("r", seq_along(v))
  attr(r, "pct") <- p
  r
}
stata_pctile_put <- function(d, name, r, label, store = "float", genp = NULL) {
  n <- nrow(d)
  fill <- function(v) c(v, rep(NA_real_, n - length(v)))
  col <- fill(unlist(r, use.names = FALSE))
  col <- switch(store, float = stata_float(col), double = col,
                { col <- trunc(col); col })
  attr(col, "label") <- label
  attr(col, "format.stata") <- switch(store, double = "%10.0g", float = "%9.0g", byte = , int = "%8.0g", "%12.0g")
  d[[name]] <- col
  if (!is.null(genp)) {
    g <- stata_float(fill(attr(r, "pct")))
    attr(g, "label") <- "percentages for __000002"
    attr(g, "format.stata") <- "%9.0g"
    d[[genp]] <- g
  }
  d
}"##;

impl Session {
    /// `pctile [type] newvar = exp [if] [in] [weight] [, nquantiles(#)
    /// genp(newvar) altdef]`.
    ///
    /// Observed in Stata 19: `nquantiles()` is 2 by default, a whole number
    /// of 2 or more, at most the rows of the data plus one; a name that is
    /// there r(110); no observations r(2000); a string r(109); weights may be
    /// f, a or p (iweights r(101)), a non-whole fweight r(401), and `altdef`
    /// with weights r(198).
    pub(super) fn pctile(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "pctile")?;
        if let Some(o) = opts.other(&[("nquantiles", 2), ("genp", 4), ("altdef", 3)]) {
            return Err(no(format!(
                "pctile with the option {o} is not translated yet"
            )));
        }
        let Some((lhs, rhs)) = c.parts.main.split_once('=') else {
            return Err(no("pctile needs `newvar = exp`"));
        };
        let words: Vec<&str> = lhs.split_whitespace().collect();
        let (store, var) = match words.as_slice() {
            [] => {
                stop(out, "varlist required (Stata: r(100))");
                return Ok(());
            }
            [v] => (Storage::Float, *v),
            [t, v] => match storage(t) {
                Some(s) if s != Storage::Str => (s, *v),
                _ => return Err(no(format!("pctile with the type {t} is not translated"))),
            },
            _ => return Err(no("pctile: more than a type and a name before `=`")),
        };
        if !is_name(var) {
            stop(out, &format!("{var} invalid name (Stata: r(198))"));
            return Ok(());
        }
        for (full, min) in [("nquantiles", 2), ("genp", 4), ("altdef", 3)] {
            if opts.0.iter().filter(|o| o.is(full, min)).count() > 1 {
                let shown = if full == "altdef" {
                    full.to_string()
                } else {
                    format!("{full}()")
                };
                stop(out, &format!("option {shown} not allowed (Stata: r(198))"));
                return Ok(());
            }
        }
        let genp = opts.arg("genp", 4).map(|g| g.trim().to_string());
        if let Some(g) = &genp {
            if !is_name(g) {
                return Err(no(format!("genp({g}): a variable name is translated")));
            }
        }
        let nq = match opts.arg("nquantiles", 2) {
            None => 2,
            Some(n) => match n.trim().parse::<i64>() {
                Ok(k) if k >= 2 => k,
                Ok(_) => {
                    stop(
                        out,
                        "nquantiles() must be greater than or equal to 2 (Stata: r(198))",
                    );
                    return Ok(());
                }
                Err(_) => {
                    stop(out, "option nq() incorrectly specified (Stata: r(198))");
                    return Ok(());
                }
            },
        };
        if nq > 5000 {
            return Err(no("pctile with more than 5000 quantiles is not translated"));
        }
        let altdef = opts.has("altdef", 3);
        let w = self.weight_of(c, &[W::F, W::A, W::P, W::I])?;
        if matches!(w, Some((W::I, _))) {
            stop(out, "iweights not allowed (Stata: r(101))");
            return Ok(());
        }
        if altdef && w.is_some() {
            stop(
                out,
                "altdef option cannot be used with weights (Stata: r(198))",
            );
            return Ok(());
        }
        let exp = rhs.trim();
        let (x, kind) = self.value(exp, out)?;
        if kind == Kind::Text {
            stop(out, "type mismatch (Stata: r(109))");
            return Ok(());
        }
        let d = self.data()?;
        let rows = self.rows_for(c, out)?;
        let rows = self.weighted_rows(rows, &w);
        for h in ["stata_pctile", "stata_float"] {
            self.helpers.insert(h);
        }
        out.package("dplyr");
        let mut taken = vec![var.to_string()];
        taken.extend(genp.clone());
        for v in &taken {
            out.line(format!(
                "if ({} %in% names({d})) stop({}, call. = FALSE)",
                r_text(v),
                r_text(&format!("variable {v} already defined (Stata: r(110))"))
            ));
        }
        if genp.as_deref() == Some(var) {
            stop(
                out,
                &format!("variable {var} already defined (Stata: r(110))"),
            );
            return Ok(());
        }
        out.line(format!(
            "stata_rows <- dplyr::mutate({rows}, stata_pcx = {x})"
        ));
        let wpart = match &w {
            Some((_, v)) => format!(", w = stata_rows[[{}]]", r_text(v)),
            None => String::new(),
        };
        out.line(format!(
            "stata_r <- stata_pctile(stata_rows$stata_pcx, {nq}{wpart}{}, rows = nrow({d}))",
            if altdef { ", altdef = TRUE" } else { "" }
        ));
        out.line("rm(stata_rows)");
        let t = match store {
            Storage::Byte => "byte",
            Storage::Int => "int",
            Storage::Long => "long",
            Storage::Double => "double",
            _ => "float",
        };
        let g = match &genp {
            Some(g) => format!(", genp = {}", r_text(g)),
            None => String::new(),
        };
        out.line(format!(
            "{d} <- stata_pctile_put({d}, {}, stata_r, {}, {}{g})",
            r_text(var),
            r_text(&format!("percentiles of {exp}")),
            r_text(t)
        ));
        self.kinds.insert(var.to_string(), Kind::Number);
        self.storage.insert(var.to_string(), store);
        if let Some(g) = genp {
            self.kinds.insert(g.clone(), Kind::Number);
            self.storage.insert(g, Storage::Float);
        }
        let count = usize::try_from(nq - 1).unwrap_or(1);
        self.r_new = Some((pctile_names(count), "pctile".into()));
        Ok(())
    }
}

/// `cumul`: the empirical distribution function of a variable, in a new
/// float variable: the running weight over the total, or the running
/// weight itself with `freq`, per `by()` group (missing values are a group
/// of their own). Rows outside the sample, with a missing value or a zero or
/// missing weight, get missing; a string variable gives a variable of all
/// missing, as in Stata. `equal` gives tied values the running weight after
/// the last of them. Without `equal` Stata orders tied values by its own
/// sort, which is arbitrary and cannot be reproduced; here they keep the
/// order of the rows (`plan/stata/cumul.md`).
pub(super) const CUMUL_HELPER: &str = r##"stata_cumul <- function(x, sel = NULL, w = NULL, wtype = "", by = NULL, freq = FALSE, equal = FALSE) {
  n <- length(x)
  x <- if (is.character(x)) rep(NA_real_, n) else as.numeric(x)
  use <- !is.na(x)
  if (!is.null(sel)) use <- use & rep_len(sel, n) & !is.na(rep_len(sel, n))
  ww <- if (is.null(w)) rep(1, n) else as.numeric(w)
  if (!is.null(w)) {
    use <- use & !is.na(ww)
    if (any(ww[use] < 0)) stop("negative weights encountered (Stata: r(402))", call. = FALSE)
    if (wtype == "f" && any(ww[use] != floor(ww[use]))) stop("may not use noninteger frequency weights (Stata: r(401))", call. = FALSE)
    use <- use & ww != 0
  }
  key <- if (is.null(by)) rep("", n) else do.call(paste, c(lapply(by, function(v) {
    v <- unclass(v)
    attributes(v) <- NULL
    ifelse(is.na(v), "\001NA", as.character(v))
  }), sep = "\002"))
  res <- rep(NA_real_, n)
  for (g in unique(key[use])) {
    idx <- which(use & key == g)
    idx <- idx[order(x[idx], method = "radix")]
    # the running weight is held in a float: each sum of the weights is rounded once, and so is the total
    cs <- cumsum(ww[idx])
    if (!is.null(w)) cs <- stata_float(cs)
    if (equal) {
      run <- rle(x[idx])
      cs <- rep(cs[cumsum(run$lengths)], run$lengths)
    }
    res[idx] <- if (freq) cs else cs / cs[length(cs)]
  }
  res <- stata_float(res)
  attr(res, "format.stata") <- "%9.0g"
  res
}"##;

impl Session {
    /// `cumul varname [if] [in] [weight], generate(newvar) [freq by(varlist)
    /// equal]`.
    ///
    /// Observed in Stata 19: weights f and a (p and i are r(101), a negative
    /// one r(402), a non-whole fweight r(401)); a missing generate() r(198);
    /// a name that is there r(110); two variables r(103); none r(100); a name
    /// the data lack r(111); the new variable is a float labelled `ECDF of x`;
    /// r() is left alone.
    pub(super) fn cumul(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "cumul")?;
        if let Some(o) = opts.other(&[("generate", 3), ("freq", 4), ("by", 2), ("equal", 2)]) {
            return Err(no(format!(
                "cumul with the option {o} is not translated yet"
            )));
        }
        for (full, min) in [("generate", 3), ("freq", 4), ("by", 2), ("equal", 2)] {
            if opts.0.iter().filter(|o| o.is(full, min)).count() > 1 {
                let shown = if matches!(full, "freq" | "equal") {
                    full.to_string()
                } else {
                    format!("{full}()")
                };
                stop(out, &format!("option {shown} not allowed (Stata: r(198))"));
                return Ok(());
            }
        }
        let w = self.weight_of(c, &[W::F, W::A, W::P, W::I])?;
        let vars: Vec<&str> = c.parts.main.split_whitespace().collect();
        let x = match vars.as_slice() {
            [] => {
                stop(out, "varlist required (Stata: r(100))");
                return Ok(());
            }
            [v] if is_name(v) => (*v).to_string(),
            [_] => return Err(no("cumul of an odd name is not translated")),
            _ => {
                stop(out, "too many variables specified (Stata: r(103))");
                return Ok(());
            }
        };
        if let Some((k @ (W::P | W::I), _)) = &w {
            let word = if *k == W::P { "pweights" } else { "iweights" };
            stop(out, &format!("{word} not allowed (Stata: r(101))"));
            return Ok(());
        }
        let Some(new) = opts.arg("generate", 3).map(|g| g.trim().to_string()) else {
            stop(out, "option generate() required (Stata: r(198))");
            return Ok(());
        };
        if !is_name(&new) {
            stop(out, &format!("{new} invalid name (Stata: r(198))"));
            return Ok(());
        }
        let by: Vec<String> = opts
            .arg("by", 2)
            .map(|b| b.split_whitespace().map(str::to_string).collect())
            .unwrap_or_default();
        if let Some(b) = by.iter().find(|b| !is_name(b)) {
            return Err(no(format!("cumul by({b}): variable names are translated")));
        }
        let d = self.data()?;
        let sel = self.condition(c, out)?;
        out.package("dplyr");
        self.helpers.insert("stata_cumul");
        self.helpers.insert("stata_float");
        out.line(format!(
            "if ({} %in% names({d})) stop({}, call. = FALSE)",
            r_text(&new),
            r_text(&format!("variable {new} already defined (Stata: r(110))"))
        ));
        let mut need = vec![x.clone()];
        need.extend(by.iter().cloned());
        if let Some((_, wv)) = &w {
            need.push(wv.clone());
        }
        let list = need
            .iter()
            .map(|v| r_text(v))
            .collect::<Vec<_>>()
            .join(", ");
        out.line(format!(
            "if (!all(c({list}) %in% names({d}))) stop(paste0(\"variable \", setdiff(c({list}), names({d}))[1], \" not found (Stata: r(111))\"), call. = FALSE)"
        ));
        let mut args = vec![r_name(&x), sel.unwrap_or_else(|| "NULL".into())];
        if let Some((k, wv)) = &w {
            args.push(format!("w = {}", r_name(wv)));
            args.push(format!("wtype = {}", r_text(k.code())));
        }
        if !by.is_empty() {
            args.push(format!(
                "by = list({})",
                by.iter().map(|b| r_name(b)).collect::<Vec<_>>().join(", ")
            ));
        }
        if opts.has("freq", 4) {
            args.push("freq = TRUE".into());
        }
        if opts.has("equal", 2) {
            args.push("equal = TRUE".into());
        }
        out.line(format!(
            "{d} <- dplyr::mutate({d}, {} = stata_cumul({}))",
            r_name(&new),
            args.join(", ")
        ));
        out.line(format!(
            "attr({d}[[{}]], \"label\") <- {}",
            r_text(&new),
            r_text(&format!("ECDF of {x}"))
        ));
        self.kinds.insert(new.clone(), Kind::Number);
        self.storage.insert(new, Storage::Float);
        out.caveat(
            "tied values keep the order of the rows; Stata orders them by its own sort unless equal is given",
        );
        Ok(())
    }
}

/// `pcorr y x1 x2 ...`: the partial and semipartial correlations of `y` with
/// each of the others, from the one regression of `y` on all of them (the
/// rows where every variable is present): with t the coefficient's t
/// statistic and df the residual degrees of freedom, the partial
/// correlation is t / sqrt(t^2 + df) and the semipartial one is
/// t sqrt((1 - R2) / df); the p value is the coefficient's. Frequency weights
/// count N as their sum, analytic weights keep it as the rows. A variable
/// repeated is counted once. A regressor collinear with the others is
/// refused (Stata drops the first of them and prints `(dropped)`; the matrices
/// it then leaves are not known).
pub(super) const PCORR_HELPER: &str = r##"stata_pcorr <- function(d, vars, w = NULL, wtype = "", show = TRUE) {
  miss <- setdiff(vars, names(d))
  if (length(miss)) stop("variable ", miss[1], " not found (Stata: r(111))", call. = FALSE)
  vars <- unique(vars)
  if (length(vars) < 3) stop("too few variables specified (Stata: r(102))", call. = FALSE)
  num <- function(x) {
    if (is.character(x)) return(rep(NA_real_, length(x)))
    x <- unclass(x)
    attributes(x) <- NULL
    as.numeric(x)
  }
  cols <- lapply(vars, function(v) num(d[[v]]))
  ok <- Reduce(`&`, lapply(cols, function(x) !is.na(x)))
  if (!any(ok)) stop("no observations (Stata: r(2000))", call. = FALSE)
  W <- if (is.null(w)) rep(1, nrow(d)) else num(d[[w]])
  W <- W[ok]
  y <- cols[[1]][ok]
  X <- cbind(1, do.call(cbind, lapply(cols[-1], function(x) x[ok])))
  n <- length(y)
  p <- ncol(X)
  N <- if (wtype == "f") sum(W) else n
  df <- N - p
  if (df < 1) stop("insufficient observations (Stata: r(2001))", call. = FALSE)
  if (qr(sqrt(W) * X)$rank < p) stop("pcorr with a collinear variable is not translated: Stata drops the first of them", call. = FALSE)
  B <- solve(crossprod(X * W, X))
  b <- B %*% crossprod(X * W, y)
  e <- as.numeric(y - X %*% b)
  rss <- sum(W * e^2)
  tss <- sum(W * (y - sum(W * y) / sum(W))^2)
  R2 <- 1 - rss / tss
  t <- (b / sqrt(diag(B) * rss / df))[-1]
  pc <- as.numeric(t / sqrt(t^2 + df))
  sp <- as.numeric(t * sqrt((1 - R2) / df))
  pv <- as.numeric(2 * stats::pt(-abs(t), df))
  nm <- vars[-1]
  if (show) {
    cat("(obs=", format(N, scientific = FALSE, trim = TRUE), ")\n\n", sep = "")
    cat("Partial and semipartial correlations of ", vars[1], " with\n\n", sep = "")
    cat("               Partial   Semipartial      Partial   Semipartial   Significance\n")
    cat("   Variable |    corr.         corr.      corr.^2       corr.^2          value\n")
    cat("------------+-----------------------------------------------------------------\n")
    for (i in seq_along(nm)) {
      cat(sprintf("%11s |%9.4f%14.4f%13.4f%14.4f%15.4f", stata_tab_abbrev(nm[i], 11), pc[i], sp[i], pc[i]^2, sp[i]^2, pv[i]), "\n", sep = "")
    }
  }
  mk <- function(v, col) matrix(v, ncol = 1, dimnames = list(nm, col))
  list(df = df, N = N, sp_corr = mk(sp, "Semipartial Correlation"), p_corr = mk(pc, "Partial Correlation"))
}"##;

const R_PCORR: &[&str] = &["df", "N", "sp_corr", "p_corr"];

impl Session {
    /// `pcorr varlist [if] [in] [weight]`.
    ///
    /// Observed in Stata 19: weights f and a (p and i r(101), a non-whole
    /// fweight r(401), a negative one r(402)); no variable r(100); fewer than
    /// three (after a repeated one is dropped) r(102); a name the data lack
    /// r(111); no rows, or a string variable, r(2000).
    pub(super) fn pcorr(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "pcorr")?;
        if let Some(o) = opts.other(&[]) {
            return Err(no(format!(
                "pcorr with the option {o} is not translated yet"
            )));
        }
        let w = self.weight_of(c, &[W::F, W::A, W::P, W::I])?;
        if let Some((k @ (W::P | W::I), _)) = &w {
            let word = if *k == W::P { "pweights" } else { "iweights" };
            stop(out, &format!("{word} not allowed (Stata: r(101))"));
            return Ok(());
        }
        let vars: Vec<&str> = c.parts.main.split_whitespace().collect();
        if vars.is_empty() {
            stop(out, "varlist required (Stata: r(100))");
            return Ok(());
        }
        if let Some(v) = vars.iter().find(|v| !is_name(v)) {
            return Err(no(format!(
                "pcorr {v}: only plain variable names are translated"
            )));
        }
        let rows = self.rows_for(c, out)?;
        let rows = self.weighted_rows(rows, &w);
        for h in ["stata_pcorr", "stata_tables", "stata_returned"] {
            self.helpers.insert(h);
        }
        let wpart = match &w {
            Some((k, v)) => format!(", w = {}, wtype = {}", r_text(v), r_text(k.code())),
            None => String::new(),
        };
        out.line(format!(
            "stata_r <- stata_pcorr({rows}, c({}){wpart}{})",
            vars.iter()
                .map(|v| r_text(v))
                .collect::<Vec<_>>()
                .join(", "),
            if self.quiet(c) { ", show = FALSE" } else { "" }
        ));
        self.r_new = Some((R_PCORR, "pcorr".into()));
        Ok(())
    }
}

/// `robvar y, by(g)`: the summary table of `y` in each group of `g`
/// (`tabulate g, summarize(y)`'s), then Levene's W0 and Brown and Forsythe's
/// W50 and W10: the one-way F statistic of the absolute deviations from each
/// group's mean, median and 10% trimmed mean (the trimmed mean drops the
/// integer part of 0.1 n values from each end). r() holds N, the three
/// statistics and their p values, and the degrees of freedom k - 1 and
/// N - k. With one group, or when the deviations are all alike, the
/// statistics are missing.
pub(super) const ROBVAR_HELPER: &str = r##"stata_robvar <- function(d, y, g, show = TRUE) {
  for (v in c(y, g)) if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
  if (is.character(d[[y]])) stop("type mismatch (Stata: r(109))", call. = FALSE)
  num <- function(v) { u <- unclass(v); attributes(u) <- NULL; as.numeric(u) }
  x <- num(d[[y]])
  gv <- d[[g]]
  gk <- if (is.character(gv)) gv else num(gv)
  keep <- !is.na(x) & !is.na(gk) & !(is.character(gk) & gk == "")
  if (!any(keep)) stop("no observations (Stata: r(2000))", call. = FALSE)
  if (show) invisible(stata_oneway(d, y, g, tabulate = TRUE, anova = FALSE))
  x <- x[keep]
  gk <- gk[keep]
  lv <- sort(unique(gk), method = "radix")
  k <- length(lv)
  N <- length(x)
  stat <- function(center) {
    z <- numeric(N)
    for (v in lv) { i <- gk == v; z[i] <- abs(x[i] - center(x[i])) }
    zbar <- mean(z)
    n <- vapply(lv, function(v) sum(gk == v), 0)
    zm <- vapply(lv, function(v) mean(z[gk == v]), 0)
    num <- sum(n * (zm - zbar)^2)
    den <- sum((z - zm[match(gk, lv)])^2)
    if (k < 2 || N <= k || den <= 0) return(c(NA_real_, NA_real_))
    f <- num / (k - 1) / (den / (N - k))
    c(f, stats::pf(f, k - 1, N - k, lower.tail = FALSE))
  }
  w0 <- stat(mean)
  w50 <- stat(stats::median)
  w10 <- stat(function(v) mean(v, trim = 0.1))
  if (show) {
    one <- function(lab, s) {
      # 8 significant digits, fewer when the text would pass 10 columns, with the leading zero
      fm <- function(v) { for (dg in 8:1) { t <- sprintf(paste0("%.", dg, "g"), v); if (nchar(t) <= 10) break }; t }
      f <- if (is.na(s[1])) " ." else fm(s[1])
      p <- if (is.na(s[2])) " ." else fm(s[2])
      cat("\n", sprintf("%-3s =  %s   df(%s, %s)     Pr > F = %s", lab, f, k - 1, N - k, p), "\n", sep = "")
    }
    one("W0", w0)
    one("W50", w50)
    one("W10", w10)
    cat("\n")
  }
  list(N = N, w10 = w10[1], p_w10 = w10[2], w50 = w50[1], p_w50 = w50[2], w0 = w0[1], p_w0 = w0[2],
       df_1 = k - 1, df_2 = N - k)
}"##;

const R_ROBVAR: &[&str] = &[
    "N", "w10", "p_w10", "w50", "p_w50", "w0", "p_w0", "df_1", "df_2",
];

impl Session {
    /// `robvar varname [if] [in], by(varname)`.
    ///
    /// Observed in Stata 19: no weights (r(101)); by() required r(198); two
    /// by variables r(103); a string `varname` r(109); a name the data lack
    /// r(111); no varname r(100).
    pub(super) fn robvar(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "robvar")?;
        if let Some(o) = opts.other(&[("by", 2)]) {
            return Err(no(format!(
                "robvar with the option {o} is not translated yet"
            )));
        }
        if c.parts.weight.is_some() {
            stop(out, "weights not allowed (Stata: r(101))");
            return Ok(());
        }
        let vars: Vec<&str> = c.parts.main.split_whitespace().collect();
        let [y] = vars.as_slice() else {
            if vars.is_empty() {
                stop(out, "varlist required (Stata: r(100))");
                return Ok(());
            }
            return Err(no("robvar with more than one variable is not translated"));
        };
        if !is_name(y) {
            return Err(no(format!(
                "robvar {y}: only a plain variable name is translated"
            )));
        }
        let Some(by) = opts.arg("by", 2) else {
            stop(out, "option by() required (Stata: r(198))");
            return Ok(());
        };
        let by: Vec<&str> = by.split_whitespace().collect();
        let [g] = by.as_slice() else {
            stop(out, "by(): too many variables specified (Stata: r(103))");
            return Ok(());
        };
        if !is_name(g) {
            return Err(no(format!(
                "robvar by({g}): only a plain variable name is translated"
            )));
        }
        let rows = self.rows_for(c, out)?;
        for h in [
            "stata_robvar",
            "stata_oneway",
            "stata_fmt",
            "stata_returned",
        ] {
            self.helpers.insert(h);
        }
        out.line(format!(
            "stata_r <- stata_robvar({rows}, {}, {}{})",
            r_text(y),
            r_text(g),
            if self.quiet(c) { ", show = FALSE" } else { "" }
        ));
        self.r_new = Some((R_ROBVAR, "robvar".into()));
        Ok(())
    }
}

/// `bitest` and `bitesti`: the exact binomial test of `k` successes in `N`
/// trials against `p`. The one-sided tails are `Pr(K >= k)` and `Pr(K <= k)`;
/// the two-sided probability is the tail on `k`'s side plus the opposite
/// tail from the outcome `k_opp` out, the first one whose probability is no
/// more than that of `k` (to a relative 1e-7, so equal probabilities count).
/// When `k` is the expected count the two-sided p is 1; when no outcome on
/// the other side is as unlikely the other tail is empty, `k_opp` is missing
/// and the two-sided p is the one tail (Stata says so in a note).
pub(super) const BITEST_HELPER: &str = r##"stata_bitest <- function(N, k, p, name = NULL, show = TRUE) {
  if (N > 1e7) stop("bitest with more than 10 million trials is not translated", call. = FALSE)
  pu <- if (k <= 0) 1 else stats::pbinom(k - 1, N, p, lower.tail = FALSE)
  pl <- stats::pbinom(k, N, p)
  mu <- N * p
  d <- stats::dbinom(0:N, N, p)
  lim <- d[k + 1] * (1 + 1e-7)
  kopp <- NA_real_
  p2 <- NA_real_
  if (abs(k - mu) <= 1e-9 * max(1, mu)) {
    kopp <- k
    p2 <- 1
  } else if (k < mu) {
    j0 <- ceiling(mu)
    hit <- if (j0 <= N) which(d[(j0 + 1):(N + 1)] <= lim) else integer(0)
    if (length(hit)) kopp <- j0 + hit[1] - 1
    p2 <- pl + if (is.na(kopp)) 0 else stats::pbinom(kopp - 1, N, p, lower.tail = FALSE)
  } else {
    j0 <- floor(mu)
    hit <- which(d[1:(j0 + 1)] <= lim)
    if (length(hit)) kopp <- hit[length(hit)] - 1
    p2 <- pu + if (is.na(kopp)) 0 else stats::pbinom(kopp, N, p)
  }
  p2 <- min(p2, 1)
  if (show) {
    fk <- function(v) formatC(v, format = "f", digits = 0, big.mark = ",")
    g <- function(v) stata_fmt(v, "%9.0g", trim = TRUE)
    cat("\nBinomial probability test\n\n")
    if (is.null(name)) {
      cat("            N   Observed k   Expected k   Assumed p   Observed p\n")
      cat(strrep("-", 64), "\n", sep = "")
      cat(sprintf("%13s%13s%13s%12.5f%13.5f", fk(N), fk(k), g(mu), p, k / N), "\n\n", sep = "")
    } else {
      cat("    Variable |            N   Observed k   Expected k   Assumed p   Observed p\n")
      cat(strrep("-", 13), "+", strrep("-", 64), "\n", sep = "")
      cat(sprintf("%12s |%13s%13s%13s%12.5f%13.5f", stata_tab_abbrev(name, 12), fk(N), fk(k), g(mu), p, k / N), "\n\n", sep = "")
    }
    kk <- fk(k)
    up <- paste0("Pr(k >= ", kk, ")")
    lo <- paste0("Pr(k <= ", kk, ")")
    two <- if (is.na(kopp)) {
      if (k < mu) lo else up
    } else if (kopp < k) {
      paste0("Pr(k <= ", fk(kopp), " or k >= ", kk, ")")
    } else {
      paste0("Pr(k <= ", kk, " or k >= ", fk(kopp), ")")
    }
    wd <- max(nchar(c(up, lo, two)))
    line <- function(l, v, tag) cat("  ", formatC(l, width = -wd), " = ", sprintf("%8.6f", v), "  (", tag, ")\n", sep = "")
    line(up, pu, "one-sided test")
    line(lo, pl, "one-sided test")
    line(two, p2, "two-sided test")
    if (is.na(kopp)) cat("  note: ", if (k < mu) "upper" else "lower", " tail of two-sided p-value is empty\n", sep = "")
  }
  list(k_opp = kopp, p = p2, k = k, P_p = p, N = N, p_l = pl, p_u = pu)
}
stata_bitest_data <- function(d, v, p, w = NULL, show = TRUE) {
  if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
  num <- function(x) { u <- unclass(x); attributes(u) <- NULL; as.numeric(u) }
  if (!(p > 0 && p < 1)) stop("success probability #p must be in (0,1) (Stata: r(450))", call. = FALSE)
  x <- if (is.character(d[[v]])) rep(NA_real_, nrow(d)) else num(d[[v]])
  wv <- if (is.null(w)) rep(1, nrow(d)) else num(d[[w]])
  ok <- !is.na(x) & !is.na(wv)
  if (any(x[ok] != 0 & x[ok] != 1)) stop(v, " is not a 0/1 variable (Stata: r(450))", call. = FALSE)
  if (!any(ok)) stop("no observations (Stata: r(2000))", call. = FALSE)
  stata_bitest(sum(wv[ok]), sum(wv[ok] * x[ok]), p, name = v, show = show)
}
stata_bitesti <- function(N, k, p, show = TRUE) {
  if (N != floor(N)) stop("invalid number of trials (Stata: r(7))", call. = FALSE)
  if (N < 1 || k < 0 || k != floor(k) || k > N) stop("invalid number of successes (Stata: r(450))", call. = FALSE)
  if (!(p > 0 && p < 1)) stop("invalid success probability (Stata: r(450))", call. = FALSE)
  stata_bitest(N, k, p, show = show)
}"##;

const R_BITEST: &[&str] = &["k_opp", "p", "k", "P_p", "N", "p_l", "p_u"];

impl Session {
    /// `bitest varname == #p [if] [in] [fweight]`.
    ///
    /// Observed in Stata 19: only frequency weights (a, p and i r(101), a
    /// non-whole one r(401)); a variable other than 0 and 1 r(450); #p outside
    /// (0,1) r(450); no `=exp` r(100); no rows or a string r(2000).
    pub(super) fn bitest(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "bitest")?;
        if let Some(o) = opts.other(&[]) {
            return Err(no(format!(
                "bitest with the option {o} is not translated yet"
            )));
        }
        let w = self.weight_of(c, &[W::F, W::A, W::P, W::I])?;
        if let Some((k, _)) = &w {
            if *k != W::F {
                let word = match k {
                    W::A => "aweights",
                    W::P => "pweights",
                    _ => "iweights",
                };
                stop(out, &format!("{word} not allowed (Stata: r(101))"));
                return Ok(());
            }
        }
        let Some((lhs, rhs)) = c.parts.main.split_once("==") else {
            stop(out, "=exp required (Stata: r(100))");
            return Ok(());
        };
        let vars: Vec<&str> = lhs.split_whitespace().collect();
        let [v] = vars.as_slice() else {
            if vars.is_empty() {
                stop(out, "varlist required (Stata: r(100))");
                return Ok(());
            }
            stop(out, "== invalid name (Stata: r(198))");
            return Ok(());
        };
        if !is_name(v) {
            return Err(no(format!(
                "bitest {v}: only a plain variable name is translated"
            )));
        }
        let p: f64 = rhs
            .trim()
            .parse()
            .map_err(|_| no("bitest var == #p: a number is translated for #p"))?;
        let rows = self.rows_for(c, out)?;
        let rows = self.weighted_rows(rows, &w);
        for h in [
            "stata_bitest",
            "stata_fmt",
            "stata_tables",
            "stata_returned",
        ] {
            self.helpers.insert(h);
        }
        let wpart = match &w {
            Some((_, wv)) => format!(", w = {}", r_text(wv)),
            None => String::new(),
        };
        out.line(format!(
            "stata_r <- stata_bitest_data({rows}, {}, {}{wpart}{})",
            r_text(v),
            r_number(p),
            if self.quiet(c) { ", show = FALSE" } else { "" }
        ));
        self.r_new = Some((R_BITEST, "bitest".into()));
        Ok(())
    }

    /// `bitesti #N #succ #p`.
    pub(super) fn bitesti(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "bitesti")?;
        if let Some(o) = opts.other(&[]) {
            return Err(no(format!(
                "bitesti with the option {o} is not translated yet"
            )));
        }
        let words: Vec<&str> = c.parts.main.split_whitespace().collect();
        let [n, k, p] = words.as_slice() else {
            stop(out, "invalid syntax (Stata: r(198))");
            return Ok(());
        };
        let num = |s: &str| {
            s.parse::<f64>()
                .map_err(|_| no("bitesti: numbers written out are translated"))
        };
        let (n, k, p) = (num(n)?, num(k)?, num(p)?);
        for h in [
            "stata_bitest",
            "stata_fmt",
            "stata_tables",
            "stata_returned",
        ] {
            self.helpers.insert(h);
        }
        out.line(format!(
            "stata_r <- stata_bitesti({}, {}, {}{})",
            r_number(n),
            r_number(k),
            r_number(p),
            if self.quiet(c) { ", show = FALSE" } else { "" }
        ));
        self.r_new = Some((R_BITEST, "bitest".into()));
        Ok(())
    }
}

/// `swilk varlist`: the Shapiro-Wilk W test for normal data. W is R's
/// `shapiro.test` (Royston's AS R94); V, z and the p value follow from it by
/// Royston's normalising transformation for n from 12 to 2000, which gives
/// Stata's z, V and p to 16 digits from Stata's own W. Stata's W itself
/// differs from R's by about 1e-8 relative (5e-9 in the probes), so every
/// value agrees to 7 digits and the printed table to its digits. With tied
/// values Stata's W differs by much more (a pair of equal values in 10 moved
/// it by 0.003), by a rule that is not decoded, so tied values are refused;
/// so are 4 to 11 values (V's rule there is not decoded) and more than 2000.
/// Fewer than 4 values print a row of dots, as Stata does.
pub(super) const SWILK_HELPER: &str = r##"stata_swilk <- function(d, vars, show = TRUE) {
  miss <- setdiff(vars, names(d))
  if (length(miss)) stop("variable ", miss[1], " not found (Stata: r(111))", call. = FALSE)
  one <- function(v) {
    x <- d[[v]]
    x <- if (is.character(x)) numeric(0) else { u <- unclass(x); attributes(u) <- NULL; as.numeric(u) }
    x <- x[!is.na(x)]
    n <- length(x)
    if (n >= 1 && all(x == x[1])) stop("no observations (Stata: r(2000))", call. = FALSE)
    if (n < 3) return(list(n = n, w = NA_real_, v = NA_real_, z = NA_real_, p = NA_real_))
    if (n < 12) stop("swilk of fewer than 12 values is not translated: Stata's V there is not decoded", call. = FALSE)
    if (n > 2000) stop("swilk of more than 2000 values is not translated", call. = FALSE)
    if (anyDuplicated(x)) stop("swilk with tied values is not translated: Stata's W differs from Royston's where values repeat", call. = FALSE)
    W <- unname(stats::shapiro.test(x)$statistic)
    l <- log(n)
    mu <- 0.0038915 * l^3 - 0.083751 * l^2 - 0.31082 * l - 1.5861
    sg <- exp(0.0030302 * l^2 - 0.082676 * l - 0.4803)
    z <- (log(1 - W) - mu) / sg
    list(n = n, w = W, v = (1 - W) / exp(mu), z = z, p = stats::pnorm(z, lower.tail = FALSE))
  }
  res <- lapply(vars, one)
  if (show) {
    cat("\n                   Shapiro–Wilk W test for normal data\n\n")
    cat("    Variable |        Obs       W           V         z       Prob>z\n")
    cat(strrep("-", 13), "+", strrep("-", 54), "\n", sep = "")
    f <- function(v, fmt) if (is.na(v)) formatC(".", width = as.integer(sub("%([0-9]+)\\..*", "\\1", fmt))) else sprintf(fmt, v)
    for (i in seq_along(vars)) {
      r <- res[[i]]
      cat(sprintf("%12s |%11d", stata_tab_abbrev(vars[i], 12), as.integer(r$n)), f(r$w, "%11.5f"), f(r$v, "%11.3f"), f(r$z, "%10.3f"), f(r$p, "%11.5f"), "\n", sep = "")
    }
    if (any(vapply(res, function(r) r$n < 3, TRUE))) {
      cat("Note: The normal approximation to the sampling distribution of W'\n      is valid for 4<=n<=2000.\n")
    }
    cat("\n")
  }
  r <- res[[length(res)]]
  list(p = r$p, z = r$z, V = r$v, W = r$w, N = r$n)
}"##;

const R_SWILK: &[&str] = &["p", "z", "V", "W", "N"];

impl Session {
    /// `swilk varlist [if] [in]`.
    ///
    /// Observed in Stata 19: no weights (r(101)); no variable r(100); a
    /// constant variable r(2000) "no observations"; a string variable or
    /// fewer than 4 values a row of dots (r(N) the count); r() holds the
    /// last variable's p, z, V, W and N.
    pub(super) fn swilk(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "swilk")?;
        if let Some(o) = opts.other(&[]) {
            stop(out, &format!("option {o} not allowed (Stata: r(198))"));
            return Ok(());
        }
        if c.parts.weight.is_some() {
            stop(out, "weights not allowed (Stata: r(101))");
            return Ok(());
        }
        let vars: Vec<&str> = c.parts.main.split_whitespace().collect();
        if vars.is_empty() {
            stop(out, "varlist required (Stata: r(100))");
            return Ok(());
        }
        if let Some(v) = vars.iter().find(|v| !is_name(v)) {
            return Err(no(format!(
                "swilk {v}: only plain variable names are translated"
            )));
        }
        let rows = self.rows_for(c, out)?;
        for h in ["stata_swilk", "stata_tables", "stata_returned"] {
            self.helpers.insert(h);
        }
        out.line(format!(
            "stata_r <- stata_swilk({rows}, c({}){})",
            vars.iter()
                .map(|v| r_text(v))
                .collect::<Vec<_>>()
                .join(", "),
            if self.quiet(c) { ", show = FALSE" } else { "" }
        ));
        self.r_new = Some((R_SWILK, "swilk".into()));
        Ok(())
    }
}

/// `inspect [varlist]`: for each variable the counts of negative, zero and
/// positive values (and how many of each are integers), the missing values,
/// a six-row histogram, the smallest and largest value, the number of
/// distinct values, and for a labelled variable whether every value is in
/// the label (`plan/stata/inspect.md`). The histogram has a column for each
/// value when there are five or fewer, else five equal bins from the
/// smallest to the largest; a column is `floor(6 n / most)` high and a column
/// with fewer shows a dot at the bottom. A string variable counts as all
/// missing. r() holds the last variable's counts.
pub(super) const INSPECT_HELPER: &str = r##"stata_inspect <- function(d, vars = NULL, show = TRUE) {
  if (is.null(vars)) vars <- names(d)
  miss <- setdiff(vars, names(d))
  if (length(miss)) stop("variable ", miss[1], " not found (Stata: r(111))", call. = FALSE)
  nr <- nrow(d)
  num <- function(v) { u <- unclass(v); attributes(u) <- NULL; as.numeric(u) }
  res <- NULL
  for (v in vars) {
    xv <- d[[v]]
    x <- if (is.character(xv)) rep(NA_real_, nr) else num(xv)
    ok <- !is.na(x)
    y <- x[ok]
    neg <- y < 0
    zer <- y == 0
    pos <- y > 0
    int <- y == trunc(y)
    cnt <- function(sel) c(sum(sel), sum(sel & int), sum(sel & !int))
    rn <- cnt(neg)
    rz <- cnt(zer)
    rp <- cnt(pos)
    rt <- c(length(y), sum(int), sum(!int))
    nmiss <- sum(!ok)
    fy <- stata_float(y)
    uq <- sort(unique(fy))
    nu <- length(uq)
    vl <- if (is.character(xv)) NULL else attr(xv, "labels", exact = TRUE)
    undoc <- if (is.null(vl)) NA_real_ else sum(!(y %in% num(vl)))
    cols <- integer(0)
    if (nu > 0 && nu <= 5) {
      cols <- vapply(uq, function(u) sum(fy == u), 0L)
    } else if (nu > 5) {
      lo <- min(y)
      hi <- max(y)
      b <- pmin(floor(5 * (y - lo) / (hi - lo)), 4) + 1
      cols <- tabulate(b, 5)
    }
    h <- if (length(cols)) floor(6 * cols / max(cols)) else integer(0)
    hist <- vapply(1:6, function(r) {
      s <- rep(" ", 20)
      s[1] <- "|"
      for (j in seq_along(h)) {
        pos_j <- 4 + 4 * (j - 1)
        if (h[j] >= 7 - r) s[pos_j] <- "#" else if (r == 6 && h[j] == 0) s[pos_j] <- "."
      }
      paste(s, collapse = "")
    }, "")
    if (show) {
      lab <- attr(xv, "label", exact = TRUE)
      full <- if (is.null(lab) || !nzchar(lab)) paste0(v, ":") else substr(paste0(v, ":  ", lab), 1, 40)
      under <- if (is.null(lab) || !nzchar(lab)) nchar(v) + 3 else nchar(full)
      shown <- sub("\\s+$", "", full)
      cat("\n", formatC(shown, width = -48), "Number of observations\n", sep = "")
      first <- strrep("-", under)
      cat(first, strrep(" ", max(0, 39 - under)), strrep("-", 39), "\n", sep = "")
      cat(sprintf("%50s%14s%14s", "Total", "Integers", "Nonintegers"), "\n", sep = "")
      f <- function(n) if (n == 0) "-" else format(n, scientific = FALSE, trim = TRUE)
      row <- function(i, label, r) cat(sprintf("%-29s%-8s%13s%14s%14s", hist[i], label, f(r[1]), f(r[2]), f(r[3])), "\n", sep = "")
      row(1, "Negative", rn)
      row(2, "Zero", rz)
      row(3, "Positive", rp)
      cat(sprintf("%-29s%s%s   %s   %s", hist[4], strrep(" ", 10), strrep("-", 11), strrep("-", 11), strrep("-", 11)), "\n", sep = "")
      row(5, "Total", rt)
      cat(sprintf("%-29s%-8s%13s", hist[6], "Missing", f(nmiss)), "\n", sep = "")
      cat(sprintf("%-39s%s", paste0("+", strrep("-", 22)), strrep("-", 11)), "\n", sep = "")
      g9 <- function(u) stata_fmt(u, "%9.0g", trim = TRUE)
      lo <- if (nu) g9(min(y)) else "."
      hi <- if (nu > 1) g9(max(y)) else if (nu == 0) "-9.0e+307" else ""
      cat(sprintf("%-9s%14s%27s", lo, hi, format(nr, scientific = FALSE, trim = TRUE)), "\n", sep = "")
      if (nu > 99) cat("(More than 99 unique values)\n") else cat(sprintf("%5s unique value%s)", paste0("(", nu), if (nu <= 1) "" else "s"), "\n", sep = "")
      if (!is.null(vl)) {
        if (undoc == 0) cat("\n      ", v, " is labeled and all values are documented in the label.\n", sep = "")
        else cat("\n      ", v, " is labeled but ", format(undoc, scientific = FALSE, trim = TRUE), " values are NOT documented in the label.\n", sep = "")
      }
    }
    res <- list(N_neg = rn[1], N_negint = rn[2], N_0 = rz[1], N_pos = rp[1], N_posint = rp[2], N = rt[1],
                N_unique = if (nu > 99) NA_real_ else nu, N_undoc = undoc)
  }
  res
}"##;

const R_INSPECT: &[&str] = &[
    "N_neg", "N_negint", "N_0", "N_pos", "N_posint", "N", "N_unique", "N_undoc",
];

impl Session {
    /// `inspect [varlist] [if] [in]`.
    pub(super) fn inspect(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "inspect")?;
        if let Some(o) = opts.other(&[]) {
            stop(out, &format!("option {o} not allowed (Stata: r(101))"));
            return Ok(());
        }
        let vars: Vec<&str> = c.parts.main.split_whitespace().collect();
        if let Some(v) = vars.iter().find(|v| !is_name(v)) {
            return Err(no(format!(
                "inspect {v}: only plain variable names are translated"
            )));
        }
        let rows = self.rows_for(c, out)?;
        for h in [
            "stata_inspect",
            "stata_fmt",
            "stata_float",
            "stata_returned",
        ] {
            self.helpers.insert(h);
        }
        let vpart = if vars.is_empty() {
            String::new()
        } else {
            format!(
                ", c({})",
                vars.iter()
                    .map(|v| r_text(v))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        out.line(format!(
            "stata_r <- stata_inspect({rows}{vpart}{})",
            if self.quiet(c) { ", show = FALSE" } else { "" }
        ));
        self.r_new = Some((R_INSPECT, "inspect".into()));
        Ok(())
    }
}

/// `dstdize cases pop stratum, by(group)`: direct standardization. The
/// standard population is every row's population by stratum; a group's
/// adjusted rate is the sum over its strata of the standard share times the
/// stratum's rate (a stratum the group lacks adds nothing), the crude rate is
/// its cases over its population, and the variance of the adjusted rate is
/// the sum of share^2 rate (1 - rate) / population, with a normal interval
/// (`plan/stata/dstdize.md`).
pub(super) const DSTDIZE_HELPER: &str = r##"stata_dstdize <- function(d, cases, pop, strata, by, level = 95, show = TRUE) {
  for (v in c(cases, pop, strata, by)) if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
  num <- function(v) { u <- unclass(v); attributes(u) <- NULL; as.numeric(u) }
  k <- num(d[[cases]])
  p <- num(d[[pop]])
  sv <- d[[strata]]
  bv <- d[[by]]
  skey <- if (is.character(sv)) sv else num(sv)
  bkey <- if (is.character(bv)) bv else num(bv)
  ok <- !is.na(k) & !is.na(p) & !is.na(skey) & !is.na(bkey) & !(is.character(skey) & skey == "") & !(is.character(bkey) & bkey == "")
  if (any(is.na(k) | is.na(p))) stop(cases, " has invalid syntax (Stata: r(198))", call. = FALSE)
  if (!any(ok)) stop("no observations (Stata: r(2000))", call. = FALSE)
  k <- k[ok]; p <- p[ok]; skey <- skey[ok]; bkey <- bkey[ok]
  if (any(k > p)) stop(cases, " must be <= ", pop, " (Stata: r(499))", call. = FALSE)
  if (anyDuplicated(data.frame(skey, bkey))) {
    # several rows for a stratum in a group are added up
    key <- paste(skey, bkey, sep = "")
    first <- !duplicated(key)
    k <- as.numeric(rowsum(k, key, reorder = FALSE)[key[first], 1])
    p <- as.numeric(rowsum(p, key, reorder = FALSE)[key[first], 1])
    skey <- skey[first]
    bkey <- bkey[first]
  }
  slv <- sort(unique(skey), method = "radix")
  glv <- sort(unique(bkey), method = "radix")
  spop <- vapply(slv, function(s) sum(p[skey == s]), 0)
  sdist <- spop / sum(spop)
  z <- stats::qnorm(1 - (100 - level) / 200)
  vl <- if (is.character(bv)) NULL else attr(bv, "labels", exact = TRUE)
  gname <- function(g) {
    if (is.character(g)) return(g)
    n <- if (!is.null(vl)) names(vl)[match(g, num(vl))] else NA
    if (length(n) && !is.na(n)) n else stata_fmt(g, "%9.0g", trim = TRUE)
  }
  comma <- function(x) formatC(x, format = "f", digits = 0, big.mark = ",")
  out <- lapply(glv, function(g) {
    i <- which(bkey == g)
    si <- match(skey[i], slv)
    o <- order(si)
    i <- i[o]; si <- si[o]
    P <- p[i]; K <- k[i]; r <- K / P; s <- sdist[si]
    adj <- stata_float(sum(s * r))
    v <- sum(s^2 * r * (1 - r) / P)
    se <- stata_float(sqrt(v))
    list(g = g, i = i, P = P, K = K, r = r, s = s, st = skey[i], N = sum(P), cases = sum(K), crude = sum(K) / sum(P),
         adj = adj, se = se, lb = stata_float(adj - z * se), ub = stata_float(adj + z * se))
  })
  if (show) {
    rule <- strrep("-", 61)
    cat("\nDirect standardization\n\n")
    for (o in out) {
      cat(rule, "\n-> ", by, " = ", gname(o$g), " \n", sep = "")
      cat("                         -----Unadjusted-----    Std.\n")
      cat("                                 Pop. Stratum    pop.  \n")
      cat("  Stratum        Pop.    Cases  dist.    rate   dist.     s*P\n")
      cat(rule, "\n", sep = "")
      for (j in seq_along(o$P)) {
        cat(sprintf("%9s%11s%10s%7.3f%8.4f%8.3f%8.4f", if (is.character(o$st[j])) o$st[j] else stata_fmt(o$st[j], "%9.0g", trim = TRUE),
                    comma(o$P[j]), format(o$K[j], scientific = FALSE, trim = TRUE), o$P[j] / o$N, o$r[j], o$s[j], o$r[j] * o$s[j]), "\n", sep = "")
      }
      cat(rule, "\n", sep = "")
      cat(sprintf("%-6s%14s%10s", "Total:", comma(o$N), format(o$cases, scientific = FALSE, trim = TRUE)), "\n", sep = "")
      cat("Note: s*P is Stratum rate multiplied by Std. pop. dist.\n\n")
      cat(sprintf("%54s %6.1f", "Adjusted cases =", o$adj * o$N), "\n", sep = "")
      cat(sprintf("%54s %6.4f", "Crude rate =", o$crude), "\n", sep = "")
      cat(sprintf("%54s %6.4f", "Adjusted rate =", o$adj), "\n", sep = "")
      cat(sprintf("                         %s%% conf. interval: [%6.4f, %6.4f]", stata_fmt(level, "%9.0g", trim = TRUE), o$lb, o$ub), "\n\n", sep = "")
    }
    cat("\nSummary of study populations\n\n")
    cat(sprintf("%9s             N   Crude rate    Adjusted rate      [%s%% conf. interval]", by, stata_fmt(level, "%9.0g", trim = TRUE)), "\n", sep = "")
    cat(strrep("-", 79), "\n", sep = "")
    for (o in out) {
      cat(sprintf("%9s%14s%13.6f%17.6f%15.6f%11.6f", gname(o$g), comma(o$N), o$crude, o$adj, o$lb, o$ub), "\n", sep = "")
    }
    cat(strrep("-", 79), "\n\n", sep = "")
  }
  cn <- paste0("c", seq_along(out))
  row <- function(nm, f) { m <- matrix(vapply(out, f, 0), nrow = 1, dimnames = list(nm, cn)); m }
  text <- function(g) paste0("`\"", if (is.character(g)) g else stata_fmt(g, "%9.0g", trim = TRUE), "\"'")
  r <- list(k = length(out), by = by)
  for (j in seq_along(out)) r[[cn[j]]] <- text(out[[j]]$g)
  r$se <- row("Se", function(o) o$se)
  r$ub_adj <- row("Right", function(o) o$ub)
  r$lb_adj <- row("Left", function(o) o$lb)
  r$adj <- row("Adjusted", function(o) o$adj)
  r$crude <- row("Crude", function(o) o$crude)
  r$Nobs <- row("Nobs", function(o) o$N)
  r
}"##;

/// Whether `name` is one of dstdize's `r(c1)`, `r(c2)`, ... (one for each group).
pub(super) fn dstdize_names(name: &str) -> bool {
    name.strip_prefix('c')
        .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
}

const R_DSTDIZE: &[&str] = &["k", "by", "se", "ub_adj", "lb_adj", "adj", "crude", "Nobs"];

impl Session {
    /// `dstdize casesvar popvar stratumvar [if] [in], by(groupvar) [level(#)]`.
    ///
    /// Observed in Stata 19: by() required r(198); fewer than three
    /// variables r(102); level() from 10 to 99.99 else r(198); a name the data
    /// lack r(111); cases above the population r(499); a missing value in
    /// the cases r(198).
    pub(super) fn dstdize(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "dstdize")?;
        if let Some(o) = opts.other(&[("by", 2), ("level", 1)]) {
            return Err(no(format!(
                "dstdize with the option {o} is not translated yet"
            )));
        }
        if c.parts.weight.is_some() {
            return Err(no("dstdize with weights is not translated"));
        }
        let vars: Vec<&str> = c.parts.main.split_whitespace().collect();
        if vars.len() < 3 {
            stop(out, "too few variables specified (Stata: r(102))");
            return Ok(());
        }
        let Some(by) = opts.arg("by", 2) else {
            stop(out, "option by() required (Stata: r(198))");
            return Ok(());
        };
        let [cases, pop, strata] = vars.as_slice() else {
            return Err(no(
                "dstdize with several strata variables is not translated yet",
            ));
        };
        let by = by.trim();
        if by.split_whitespace().count() != 1 {
            return Err(no(
                "dstdize with several by() variables is not translated yet",
            ));
        }
        for v in [cases, pop, strata, &by] {
            if !is_name(v) {
                return Err(no(format!(
                    "dstdize {v}: only plain variable names are translated"
                )));
            }
        }
        let level = match opts.arg("level", 1) {
            None => 95.0,
            Some(l) => {
                let v: f64 = l
                    .trim()
                    .parse()
                    .map_err(|_| no(format!("level({l}) needs a number")))?;
                if !(10.0..=99.99).contains(&v) {
                    stop(
                        out,
                        "level() must be between 10 and 99.99 inclusive (Stata: r(198))",
                    );
                    return Ok(());
                }
                v
            }
        };
        let rows = self.rows_for(c, out)?;
        for h in [
            "stata_dstdize",
            "stata_fmt",
            "stata_float",
            "stata_returned",
        ] {
            self.helpers.insert(h);
        }
        out.line(format!(
            "stata_r <- stata_dstdize({rows}, {}, {}, {}, {}, level = {}{})",
            r_text(cases),
            r_text(pop),
            r_text(strata),
            r_text(by),
            r_number(level),
            if self.quiet(c) { ", show = FALSE" } else { "" }
        ));
        self.r_new = Some((R_DSTDIZE, "dstdize".into()));
        Ok(())
    }
}
