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
