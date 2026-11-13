//! `tabulate x [y] [weight], summarize(v) [means standard freq obs nomeans
//! nostandard nofreq noobs missing nolabel wrap]`: the means, standard
//! deviations and counts of `v` in each group, as Stata 19 prints them
//! (`plan/stata/tabulate-summarize.md`). One way it is oneway's own summary
//! table (`stata_oneway` with no analysis of variance); two ways a table of
//! cells, one line for each statistic. r() is left empty, as in Stata.

use super::weights::W;
use super::*;

pub(super) const TABSUM_HELPER: &str = r##"stata_tabsum2 <- function(d, v, x, z, w = NULL, wtype = "f", cols = c(TRUE, TRUE, TRUE),
                          obs = TRUE, obs_always = FALSE, missing = FALSE, nolabel = FALSE) {
  for (n in c(v, x, z)) if (!n %in% names(d)) stop("variable ", n, " not found (Stata: r(111))", call. = FALSE)
  if (is.character(d[[v]])) stop(v, " is a string variable (Stata: type mismatch, r(109))", call. = FALSE)
  num <- function(u) { u <- unclass(u); attributes(u) <- NULL; as.numeric(u) }
  y <- num(d[[v]])
  wv <- if (is.null(w)) rep(1, length(y)) else num(w)
  key <- function(g) {
    k <- if (is.character(g)) ifelse(is.na(g), "", g) else num(g)
    if (missing && !is.character(g)) k[is.na(k)] <- Inf
    k
  }
  kx <- key(d[[x]])
  kz <- key(d[[z]])
  ok <- function(k, g) missing | (if (is.character(g)) k != "" else !is.na(k))
  keep <- !is.na(y) & !is.na(wv) & ok(kx, d[[x]]) & ok(kz, d[[z]])
  y <- y[keep]
  kx <- kx[keep]
  kz <- kz[keep]
  wv <- wv[keep]
  if (!length(y)) stop("no observations (Stata: r(2000))", call. = FALSE)
  wraw <- wv
  # analytic weights are scaled to sum to the rows
  if (!is.null(w) && wtype == "a") wv <- wv * length(y) / sum(wv)
  weighted <- (!is.null(w) && obs) || obs_always
  label <- function(g, lv) {
    vl <- if (nolabel || is.character(g)) NULL else attr(g, "labels", exact = TRUE)
    vapply(lv, function(u) {
      if (is.character(u)) return(substr(u, 1, 9))
      if (is.infinite(u)) return(".")
      k <- if (!is.null(vl)) names(vl)[match(u, num(vl))] else NA
      substr(if (length(k) && !is.na(k)) k else stata_fmt(u, "%9.0g", trim = TRUE), 1, 9)
    }, "")
  }
  lx <- sort(unique(kx), method = "radix")
  lz <- sort(unique(kz), method = "radix")
  rl <- label(d[[x]], lx)
  cl <- label(d[[z]], lz)
  # one cell's statistics: mean, sd (with analytic weights summarize's), Freq. and Obs
  stat <- function(s) {
    n <- sum(wv[s])
    rows <- sum(s)
    if (rows == 0) return(c(NA, NA, 0, 0))
    m <- sum(wv[s] * y[s]) / n
    ss <- sum(wv[s] * (y[s] - m)^2)
    sd <- if (!is.null(w) && wtype == "a") {
      if (rows > 1) sqrt(ss / n * rows / (rows - 1)) else 0
    } else if (n > 1) sqrt(ss / (n - 1)) else 0
    c(m, sd, sum(wraw[s]), if (!is.null(w) && wtype == "a") rows else n)
  }
  show <- c(cols, weighted)
  g9 <- function(u) if (is.na(u)) "." else stata_fmt(u, "%10.0g", trim = TRUE)
  whole <- function(u) format(u, scientific = FALSE)
  fmt <- list(g9, g9, whole, whole)
  cell <- function(s) {
    st <- stat(s)
    vapply(which(show), function(i) fmt[[i]](st[i]), "")
  }
  per <- (65 - 10) %/% 11
  if (length(lz) > per * 4) stop("tabulate, summarize() with this many columns is not translated yet", call. = FALSE)
  what <- c("Means", "Standard Deviations", "Frequencies", "Number of observations")[show]
  title <- paste(if (length(what) == 1) what else paste(paste(what[-length(what)], collapse = ", "), "and", what[length(what)]),
                 "of", v)
  pad <- function(s, n) formatC(s, width = n)
  cat("\n", strrep(" ", max(2, (75 - nchar(title)) %/% 2)), title, "\n", sep = "")
  blocks <- split(seq_along(lz), (seq_along(lz) - 1) %/% per)
  lines <- sum(show)
  for (b in blocks) {
    k <- length(b)
    cat("\n", strrep(" ", 10), " |", strrep(" ", max(0, ceiling((10 * k - nchar(z)) / 2))), z, "\n", sep = "")
    cat(pad(substr(x, 1, 10), 10), " |", paste(pad(cl[b], 10), collapse = " "), " |     Total\n", sep = "")
    rule <- paste0(strrep("-", 11), "+", strrep("-", 11 * k), "+", strrep("-", 10), "\n")
    cat(rule)
    group <- function(lab, sel) {
      cs <- lapply(b, function(j) cell(sel & kz == lz[j]))
      tot <- cell(sel)
      for (i in seq_len(lines)) {
        cat(pad(if (i == 1) lab else "", 10), " |", paste(pad(vapply(cs, `[`, "", i), 10), collapse = " "), " |",
            pad(tot[i], 10), "\n", sep = "")
      }
    }
    for (i in seq_along(lx)) {
      group(rl[i], kx == lx[i])
      if (lines > 1 || i == length(lx)) cat(rule)
    }
    group("Total", rep(TRUE, length(y)))
  }
  invisible(list())
}"##;

/// `matcell()`, `matrow()` and `matcol()`: the table's counts (with analytic
/// weights, scaled to the rows, as the table shows them) and the values of
/// its rows and columns, as Stata 19 leaves them (r1, r2 ... and c1, c2 ...).
pub(super) const TAB_MATS_HELPER: &str = r##"stata_tab_mats <- function(x, y = NULL, w = NULL, wtype = "f", missing = FALSE,
                           matrow = FALSE, matcol = FALSE) {
  if ((matrow && is.character(x)) || (matcol && is.character(y))) {
    stop("option ", if (matrow && is.character(x)) "matrow" else "matcol", "() not allowed (Stata: r(198))", call. = FALSE)
  }
  num <- function(u) { if (is.character(u)) return(u); u <- unclass(u); attributes(u) <- NULL; as.numeric(u) }
  x <- num(x)
  y <- if (is.null(y)) rep(1, length(x)) else num(y)
  wv <- if (is.null(w)) rep(1, length(x)) else num(w)
  keep <- !is.na(wv) & (missing | (!is.na(x) & !is.na(y)))
  x <- x[keep]
  y <- y[keep]
  wv <- wv[keep]
  lx <- sort(unique(x), na.last = TRUE, method = "radix")
  ly <- sort(unique(y), na.last = TRUE, method = "radix")
  at <- function(u, l) match(u, l, incomparables = NULL)
  t <- matrix(0, length(lx), length(ly))
  for (i in seq_along(wv)) t[at(x[i], lx), at(y[i], ly)] <- t[at(x[i], lx), at(y[i], ly)] + wv[i]
  if (!is.null(w) && wtype == "a") t <- t * length(wv) / sum(wv)
  dimnames(t) <- list(paste0("r", seq_along(lx)), paste0("c", seq_along(ly)))
  list(cell = t,
       row = if (!is.character(lx)) matrix(lx, ncol = 1, dimnames = list(paste0("r", seq_along(lx)), "c1")),
       col = if (!is.character(ly)) matrix(ly, nrow = 1, dimnames = list("r1", paste0("c", seq_along(ly)))))
}"##;

impl Session {
    /// matcell(), matrow() and matcol() of `tabulate`, after its table.
    pub(super) fn tab_mats(
        &mut self,
        opts: &Opts,
        out: &mut Out,
        src: &str,
        x: &str,
        y: Option<&str>,
        wargs: &str,
    ) -> Result<(), Untranslated> {
        let wanted: Vec<(&str, String)> =
            [("cell", "matcell"), ("row", "matrow"), ("col", "matcol")]
                .into_iter()
                .filter_map(|(k, o)| opts.arg(o, o.len()).map(|m| (k, m.trim().to_string())))
                .collect();
        if wanted.is_empty() {
            return Ok(());
        }
        if y.is_none() && wanted.iter().any(|(k, _)| *k == "col") {
            return Err(no("option matcol() not allowed (Stata: r(198))"));
        }
        for (_, m) in &wanted {
            if !is_name(m) {
                return Err(no(format!("{m} is not a matrix name")));
            }
        }
        self.helpers.insert("stata_tab_mats");
        self.helpers.insert("stata_matrix");
        let has = |k: &str| wanted.iter().any(|(w, _)| *w == k);
        out.line(format!(
            "stata_tm <- stata_tab_mats({src}${}{}{wargs}{}{}{})",
            r_name(x),
            y.map(|y| format!(", {src}${}", r_name(y)))
                .unwrap_or_default(),
            if opts.has("missing", 1) {
                ", missing = TRUE"
            } else {
                ""
            },
            if has("row") { ", matrow = TRUE" } else { "" },
            if has("col") { ", matcol = TRUE" } else { "" }
        ));
        for (k, m) in &wanted {
            out.line(format!("stata_mat[[{}]] <- stata_tm${k}", r_text(m)));
            self.matrix_made(m);
        }
        out.line("rm(stata_tm)");
        Ok(())
    }

    /// `tabulate x [y], summarize(v) ...`.
    pub(super) fn tabulate_summarize(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
        x: &str,
        y: Option<&str>,
        v: &str,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[
            ("summarize", 2),
            ("means", 5),
            ("standard", 8),
            ("freq", 4),
            ("obs", 3),
            ("nomeans", 7),
            ("nostandard", 10),
            ("nofreq", 6),
            ("noobs", 5),
            ("wrap", 4),
            ("missing", 1),
            ("nolabel", 3),
        ]) {
            return Err(no(format!(
                "tabulate, summarize() with the option {o} is not translated yet"
            )));
        }
        let v = v.trim();
        if !is_name(v) {
            return Err(no(format!(
                "summarize({v}): one variable name is translated"
            )));
        }
        let w = self.weight_of(c, &[W::F, W::A])?;
        let mut rows = self.rows_for(c, out)?;
        if w.is_some() {
            rows = self.weighted_rows(rows, &w);
            out.line(format!("stata_rows <- {rows}"));
            rows = "stata_rows".into();
            out.caveat(super::weights::weight_caveat(&w));
        }
        let wargs = super::weights::w_args(&rows, &w);
        // means, standard and freq choose the columns; nomeans and the like drop them
        let picked = [("means", 5), ("standard", 8), ("freq", 4)].map(|(o, m)| opts.has(o, m));
        let any = picked.iter().any(|p| *p);
        let cols = [
            ("nomeans", 7, picked[0]),
            ("nostandard", 10, picked[1]),
            ("nofreq", 6, picked[2]),
        ]
        .map(|(o, m, p)| (!any || p) && !opts.has(o, m));
        let mut args = String::new();
        if cols.iter().any(|c| !c) {
            let v: Vec<&str> = cols.iter().map(|c| super::stats::r_logical(*c)).collect();
            args.push_str(&format!(", cols = c({})", v.join(", ")));
        }
        if opts.has("noobs", 5) {
            args.push_str(", obs = FALSE");
        } else if opts.has("obs", 3) {
            args.push_str(", obs_always = TRUE");
        }
        if opts.has("missing", 1) {
            args.push_str(", missing = TRUE");
        }
        if opts.has("nolabel", 3) {
            args.push_str(", nolabel = TRUE");
        }
        self.helpers.insert("stata_fmt");
        self.helpers.insert("stata_returned");
        let call = match y {
            None => {
                self.helpers.insert("stata_oneway");
                format!(
                    "stata_r <- stata_oneway({rows}, {}, {}{wargs}, tabulate = TRUE, anova = FALSE{args}{})",
                    r_text(v),
                    r_text(x),
                    if self.quiet(c) { ", show = FALSE" } else { "" }
                )
            }
            Some(y) => {
                self.helpers.insert("stata_tabsum2");
                let call = format!(
                    "stata_r <- stata_tabsum2({rows}, {}, {}, {}{wargs}{args})",
                    r_text(v),
                    r_text(x),
                    r_text(y)
                );
                if self.quiet(c) {
                    format!("invisible(utils::capture.output({call}))")
                } else {
                    call
                }
            }
        };
        out.line(call);
        // Stata 19 leaves r() empty
        self.r_new = Some((&[], "tabulate".into()));
        Ok(())
    }
}
