//! P2B.4: Stata's statistics commands in R.
//!
//! `summarize`, `tabulate`, `correlate` and `ttest` so far. As everywhere in
//! this crate, nothing is guessed: where R cannot do what Stata does, the
//! command is not translated and says why. One refusal worth naming:
//! `ttest, welch` uses Welch's degrees of freedom, which R's `t.test` does not
//! offer (it uses Satterthwaite's, which is Stata's `unequal`).
//!
//! RCraft's own menus have dialogs for these statistics (Summary statistics,
//! Frequency table, Cross-tabulation, Correlation, the three t-tests). This is
//! not that code and must not become it. Those dialogs follow R's conventions,
//! these follow Stata's, and the two give different numbers on purpose:
//!
//! - `summarize` reports the sample standard deviation and, with `detail`, the
//!   population skewness and kurtosis (so a normal sample's kurtosis is near
//!   3, not near 0).
//! - Its percentiles are Stata's: `x[ceil(np)]`, or the mean of the two
//!   neighbours when `np` is a whole number. That is R's `type = 2`, not R's
//!   default `type = 7` that the dialogs use.
//! - `correlate` drops a row with a missing value in any of the variables
//!   (casewise), which is R's `use = "complete.obs"`.
//! - `tabulate` leaves missing values out unless told otherwise, orders
//!   categories by the value underneath, and its `chi2` has no continuity
//!   correction.
//!
//! The crate has no dependencies (it sits below the catalog), so the R here is
//! written out in full rather than borrowed from a dialog.

use super::weights::{Weighting, W};
use super::*;

/// `summarize`, and the statistics `correlate, means` prints.
pub(super) const SUMMARIZE_HELPER: &str = r#"stata_summarize <- function(d, detail = FALSE, meanonly = FALSE, w = NULL, wtype = "f", ret = FALSE,
                            print = TRUE, table = TRUE) {
  # ret: leave r() of the last variable as stata_r where it was called from;
  # table: Stata's table (correlate, means prints its own, without Obs)
  env <- parent.frame()
  rr <- NULL
  num <- function(v) {
    if (is.character(v) || is.factor(v)) return(rep(NA_real_, length(v)))
    u <- unclass(v)
    attributes(u) <- NULL
    if (is.numeric(u)) as.numeric(u) else rep(NA_real_, length(v))
  }
  weighted <- !is.null(w)
  if (!weighted) w <- rep(1, nrow(d))
  w <- num(w)
  # one variable's values (sorted) with their weights
  vals <- function(n) {
    x <- num(d[[n]])
    ok <- !is.na(x) & !is.na(w)
    o <- order(x[ok])
    list(x = x[ok][o], w = w[ok][o])
  }
  # Stata's count, sum of weights, mean and variance for each kind of weight
  moments <- function(x, v) {
    k <- length(x)
    W <- sum(v)
    m <- if (k > 0) sum(v * x) / W else NA_real_
    N <- if (weighted && wtype == "f") W else k
    ss <- sum(v * (x - m)^2)
    var <- if (!weighted || wtype == "f") ss / (N - 1) else if (wtype == "a") ss / W * k / (k - 1) else ss / (W - 1)
    list(N = N, sum_w = W, mean = m, var = if (N > 1) var else NA_real_, sum = sum(v * x))
  }
  if (meanonly) {
    # Stata's r() after meanonly, of the last variable, and nothing printed
    if (length(names(d)) == 0) return(invisible(NULL))
    a <- vals(utils::tail(names(d), 1))
    m <- moments(a$x, a$w)
    k <- length(a$x)
    # in the order of Stata's return list
    return(invisible(list(N = m$N, sum_w = m$sum_w, sum = m$sum, mean = m$mean,
                          min = if (k > 0) min(a$x) else NA_real_,
                          max = if (k > 0) max(a$x) else NA_real_)))
  }
  if (length(names(d)) == 0) {
    cat("no variables\n")
    return(invisible(NULL))
  }
  pcts <- c(1, 5, 10, 25, 50, 75, 90, 95, 99)
  # Stata's percentile with weights: the first value whose running sum of
  # weights passes p W, or the mean with the next value where it equals it;
  # with no weights that is R's quantile type 2
  wq <- function(x, v, p) {
    W <- cumsum(v)
    P <- sum(v) * p
    i <- which(W >= P)[1]
    if (W[i] == P && i < length(x)) (x[i] + x[i + 1]) / 2 else x[i]
  }
  one <- function(n) {
    a <- vals(n)
    x <- a$x
    k <- length(x)
    m <- moments(x, a$w)
    mom <- function(r) if (k > 0) sum(a$w * (x - m$mean)^r) / m$sum_w else NA_real_
    s <- data.frame(
      variable = n,
      obs = m$N,
      mean = m$mean,
      sd = sqrt(m$var),
      min = if (k > 0) min(x) else NA_real_,
      max = if (k > 0) max(x) else NA_real_,
      stringsAsFactors = FALSE
    )
    rr <<- list(N = m$N, sum_w = m$sum_w, mean = m$mean, Var = m$var, sd = sqrt(m$var), min = s$min, max = s$max, sum = m$sum)
    if (weighted) s$sum_w <- m$sum_w
    if (!detail) return(s)
    m2 <- mom(2)
    ok <- k > 1 && is.finite(m2) && m2 > 0
    s$variance <- m$var
    s$skewness <- if (ok) mom(3) / m2^1.5 else NA_real_
    s$kurtosis <- if (ok) mom(4) / m2^2 else NA_real_
    p <- if (k == 0) {
      rep(NA_real_, length(pcts))
    } else if (weighted) {
      vapply(pcts / 100, function(q) wq(x, a$w, q), 0)
    } else {
      unname(stats::quantile(x, pcts / 100, type = 2))
    }
    for (i in seq_along(pcts)) s[[paste0("p", pcts[i])]] <- p[i]
    rr <<- c(rr[1:5], list(skewness = s$skewness, kurtosis = s$kurtosis, sum = m$sum, min = s$min, max = s$max),
             stats::setNames(as.list(p), paste0("p", pcts)))
    s
  }
  out <- do.call(rbind, lapply(names(d), one))
  if (ret) assign("stata_r", rr, envir = env)
  if (!print) return(invisible(out))
  if (table && !detail && !weighted) {
    # Stata's table, as Stata 19 prints it
    string <- vapply(out$variable, function(n) is.character(d[[n]]) || is.factor(d[[n]]), TRUE)
    stata_sum_print(out$variable, out$obs, out$mean, out$sd, out$min, out$max, string)
  } else if (!detail) {
    print(out, row.names = FALSE)
  } else {
    for (i in seq_len(nrow(out))) {
      r <- out[i, ]
      x <- num(d[[r$variable]])
      x <- sort(x[!is.na(x)])
      cat(r$variable, "\n", sep = "")
      cat("  obs ", r$obs, "  mean ", r$mean, "  sd ", r$sd, "  variance ", r$variance, "\n", sep = "")
      cat("  min ", r$min, "  max ", r$max, "  skewness ", r$skewness, "  kurtosis ", r$kurtosis, "\n", sep = "")
      cat("  percentiles 1 5 10 25 50 75 90 95 99: ",
          paste(unlist(r[paste0("p", pcts)]), collapse = " "), "\n", sep = "")
      cat("  smallest ", paste(utils::head(x, 4), collapse = " "),
          "   largest ", paste(utils::tail(x, 4), collapse = " "), "\n", sep = "")
    }
  }
  invisible(out)
}"#;

/// A column as `tabulate` shows it: its value labels when it has them, in the
/// order of the values underneath (which is Stata's order), the plain values
/// when it has none or `nolabel` was given. With `tags` the places of `.a` to
/// `.z` (`stata_mv`) go with it, so that `missing` shows each on a row of its
/// own, after the values and `.` (Stata 19).
pub(super) const TAB_VALUES_HELPER: &str = r#"stata_tab_values <- function(v, nolabel = FALSE, tags = FALSE) {
  if (is.factor(v)) return(v)
  m <- if (tags && is.double(v) && anyNA(v)) stata_mv(v)
  l <- attr(v, "labels")
  mark <- function(r) {
    if (any(m > 1)) {
      attr(r, "stata_mv") <- m
      # each row's name: a label of .a to .z when there is one
      ml <- rep(NA_character_, 26)
      if (!nolabel && length(l)) ml <- names(l)[match(letters, haven::na_tag(as.numeric(l)))]
      attr(r, "stata_mv_names") <- c(".", ifelse(is.na(ml), paste0(".", letters), ml))
    }
    r
  }
  u <- unclass(v)
  attributes(u) <- NULL
  if (nolabel || is.null(l) || length(l) == 0) return(mark(u))
  vals <- sort(unique(u[!is.na(u)]))
  labs <- names(l)[match(vals, l)]
  labs[is.na(labs)] <- format(vals[is.na(labs)])
  mark(factor(labs[match(u, vals)], levels = unique(labs)))
}"#;

/// `tabulate x, generate(stub)`: one indicator per value, in sorted order,
/// labelled `x==` and the value's label, the text, or the number as Stata 19
/// writes it there (`%11.4f`, or `%11.4e` when that is wider). Rows the table
/// leaves out (missing x, if, in) are missing; with `missing` a missing x
/// has its own indicator, labelled `x==` and six spaces before `.`; with
/// `tags` each of `.a` to `.z` has one after it, labelled with it 11 wide
/// (`x==         .b`, Stata 19).
pub(super) const TAB_GENERATE_HELPER: &str = r#"stata_tab_generate <- function(d, var, stub, sel = TRUE, missing = FALSE, nolabel = FALSE, tags = FALSE) {
  x <- d[[var]]
  text <- is.character(x)
  labs <- if (nolabel || text) NULL else attr(x, "labels")
  if (!text) x <- as.numeric(unclass(x))
  sel <- rep_len(as.logical(sel), nrow(d))
  sel[is.na(sel)] <- FALSE
  miss <- if (text) is.na(x) | x == "" else is.na(x)
  use <- sel & (missing | !miss)
  vals <- unique(x[use & !miss])
  vals <- if (text) sort(vals, method = "radix") else sort(vals)
  label_of <- function(v) {
    if (text) return(v)
    if (!is.null(labs) && any(labs == v)) return(names(labs)[match(v, labs)][1])
    s <- sprintf("%11.4f", v)
    if (nchar(s) > 11) s <- sprintf("%11.4e", v)
    s
  }
  cols <- lapply(vals, function(v) list(is = !miss & x == v, label = label_of(v)))
  if (missing && any(use & miss)) {
    if (text) stop("tabulate, generate() with missing for a string variable is not translated yet")
    p <- if (tags) stata_mv(x) else as.numeric(miss)
    lv <- c(".", paste0(".", letters))
    for (k in sort(unique(p[use & miss]))) {
      cols <- c(cols, list(list(is = p == k, label = if (k == 1) "      ." else formatC(lv[k], width = 11))))
    }
  }
  made <- paste0(stub, seq_along(cols))
  if (any(made %in% names(d))) stop("variable ", made[made %in% names(d)][1], " already defined")
  for (k in seq_along(cols)) {
    name <- made[k]
    v <- ifelse(use, as.numeric(cols[[k]]$is), NA_real_)
    attr(v, "label") <- paste0(var, "==", cols[[k]]$label)
    d[[name]] <- v
  }
  d
}"#;

/// One-way and two-way `tabulate`.
pub(super) const TABULATE_HELPER: &str = r#"stata_tabulate <- function(x, y = NULL, missing = FALSE, sort = FALSE,
                            row = FALSE, column = FALSE, cell = FALSE,
                            chi2 = FALSE, w = NULL, wtype = "f", ret = FALSE,
                            xname = NULL, yname = NULL, nofreq = FALSE, lrchi2 = FALSE, key = TRUE) {
  # ret: leave Stata's r() as stata_r where it was called from; with the
  # variables' names, the table is Stata's (counts, or frequency weights)
  env <- parent.frame()
  counts <- is.null(w) || wtype == "f"
  use_na <- if (missing) "ifany" else "no"
  # with missing, each of . and .a to .z (their places from stata_tab_values)
  # is a value of its own, after the others, in Stata's order
  split <- function(v) {
    m <- attr(v, "stata_mv", exact = TRUE)
    if (!missing || is.null(m)) return(v)
    lv <- attr(v, "stata_mv_names", exact = TRUE)
    f <- if (is.factor(v)) v else factor(v)
    s <- factor(as.character(f), levels = unique(c(levels(f), lv[sort(unique(m[m > 0]))])))
    s[m > 0] <- lv[m[m > 0]]
    s
  }
  x <- split(x)
  if (!is.null(y)) y <- split(y)
  # weighted counts: sums of the weights, analytic ones scaled to the rows
  wtab <- function(keep, ...) {
    if (missing) keep[] <- TRUE
    f <- function(v) if (missing) factor(v[keep], exclude = NULL) else factor(v[keep])
    v <- lapply(list(...), f)
    t <- if (length(v) == 1) stats::xtabs(w[keep] ~ v[[1]]) else stats::xtabs(w[keep] ~ v[[1]] + v[[2]])
    if (wtype == "a") t <- t * sum(keep) / sum(t)
    t
  }
  if (is.null(y)) {
    t <- if (is.null(w)) table(x, useNA = use_na) else wtab(!is.na(x), x)
    if (sort) t <- sort(t, decreasing = TRUE)
    freq <- as.numeric(t)
    total <- sum(freq)
    pct <- if (total > 0) 100 * freq / total else rep(NA_real_, length(freq))
    out <- data.frame(
      value = ifelse(is.na(names(t)), ".", names(t)),
      freq = freq,
      percent = round(pct, 2),
      cum = round(cumsum(pct), 2),
      stringsAsFactors = FALSE
    )
    # a string's column is as wide as its Stata storage type, which R does not keep
    if (!is.null(xname) && counts && !is.character(x)) {
      stata_tab1_print(xname, out$value, freq)
    } else {
      print(out, row.names = FALSE)
      cat("  total ", total, "\n", sep = "")
    }
    if (ret) assign("stata_r", list(N = total, r = length(freq)), envir = env)
    return(invisible(out))
  }
  keep <- if (missing) rep(TRUE, length(x)) else !is.na(x) & !is.na(y)
  t <- if (is.null(w)) table(x[keep], y[keep], useNA = use_na) else wtab(keep, x, y)
  if (chi2) {
    r <- suppressWarnings(stats::chisq.test(t, correct = FALSE))
    attr(t, "chi2") <- c(chi2 = unname(r$statistic), df = unname(r$parameter), p = r$p.value)
  }
  # the likelihood-ratio chi2: 2 sum O ln(O/E), its line after Pearson's (Stata 19)
  lr <- NULL
  if (lrchi2) {
    E <- outer(rowSums(t), colSums(t)) / sum(t)
    O <- matrix(as.numeric(t), nrow(t))
    G <- 2 * sum(ifelse(O > 0, O * log(O / E), 0))
    dfl <- (nrow(t) - 1) * (ncol(t) - 1)
    lr <- c(chi2 = G, df = dfl, p = stats::pchisq(G, dfl, lower.tail = FALSE))
  }
  lrline <- function() {
    if (is.null(lr)) return(invisible(NULL))
    if (!chi2) cat("\n")
    cat(sprintf(" Likelihood-ratio chi2(%d) = %8.4f   Pr = %5.3f", as.integer(lr[["df"]]), lr[["chi2"]], lr[["p"]]), "\n", sep = "")
  }
  rets <- function() {
    rr <- list(N = sum(t), r = nrow(t), c = ncol(t))
    if (chi2) rr <- c(rr, list(chi2 = attr(t, "chi2")[["chi2"]], p = attr(t, "chi2")[["p"]]))
    if (!is.null(lr)) rr <- c(rr, list(chi2_lr = lr[["chi2"]], p_lr = lr[["p"]]))
    assign("stata_r", rr, envir = env)
  }
  dot <- function(v) ifelse(is.na(v), ".", v)
  rlabs <- dot(rownames(t))
  clabs <- dot(colnames(t))
  # Stata 19: nofreq with no percentage prints nothing and leaves r() empty
  # with chi2, only its line, and r() as always
  if (nofreq && !row && !column && !cell) {
    if (chi2 || lrchi2) {
      s <- attr(t, "chi2")
      if (chi2) cat("\n", strrep(" ", 10), sprintf("Pearson chi2(%d) = %8.4f   Pr = %5.3f", as.integer(s[["df"]]), s[["chi2"]], s[["p"]]), "\n", sep = "")
      lrline()
      if (ret) rets()
    } else if (ret) {
      assign("stata_r", list(), envir = env)
    }
    return(invisible(t))
  }
  if (!is.null(xname) && !is.null(yname) && counts &&
      !is.character(x) && !is.character(y) &&
      stata_tab2_fits(xname, yname, clabs)) {
    m <- matrix(as.numeric(t), nrow(t), ncol(t))
    if (row || column || cell || nofreq) {
      # percentages in Stata 19's layout (nofreq alone prints nothing)
      stata_tab2_pct_print(xname, yname, rlabs, clabs, m, freq = !nofreq, row = row, column = column,
                           cell = cell, chi2 = if (chi2) attr(t, "chi2"), key = key)
    } else {
      stata_tab2_print(xname, yname, rlabs, clabs, m, if (chi2) attr(t, "chi2"))
    }
    lrline()
    if (ret) rets()
    return(invisible(t))
  }
  if (!nofreq) print(stats::addmargins(t))
  if (row) {
    cat("\nrow percentages\n")
    print(round(100 * prop.table(t, 1), 2))
  }
  if (column) {
    cat("\ncolumn percentages\n")
    print(round(100 * prop.table(t, 2), 2))
  }
  if (cell) {
    cat("\ncell percentages\n")
    print(round(100 * prop.table(t), 2))
  }
  if (chi2) {
    s <- attr(t, "chi2")
    cat("\nPearson chi2(", s[["df"]], ") = ", format(s[["chi2"]], digits = 6),
        "   Pr = ", format(s[["p"]], digits = 4), "\n", sep = "")
  }
  lrline()
  if (ret) rets()
  invisible(t)
}"#;

/// `correlate`: casewise, like Stata.
pub(super) const CORRELATE_HELPER: &str = r#"stata_correlate <- function(d, covariance = FALSE, means = FALSE, w = NULL, wtype = "f", ret = FALSE) {
  env <- parent.frame()
  # as Stata 19 does: a string variable is left out with a note, and only
  # when none is left is it an error
  for (n in names(d)) {
    if (is.character(d[[n]]) || is.factor(d[[n]])) {
      cat("(", n, " ignored because string variable)\n", sep = "")
      d[[n]] <- NULL
    }
  }
  if (length(d) == 0) stop("too few variables specified")
  m <- as.matrix(as.data.frame(lapply(d, function(v) {
    u <- unclass(v)
    attributes(u) <- NULL
    as.numeric(u)
  })))
  colnames(m) <- names(d)
  ok <- stats::complete.cases(m)
  m <- m[ok, , drop = FALSE]
  if (!is.null(w)) w <- as.numeric(w[ok])
  cat("(obs=", if (!is.null(w) && wtype == "f") sum(w) else nrow(m), ")\n", sep = "")
  if (nrow(m) < 2) {
    cat("too few rows are left once the ones with a missing value are dropped\n")
    return(invisible(NULL))
  }
  r <- if (is.null(w)) {
    if (covariance) stats::cov(m) else stats::cor(m)
  } else {
    C <- stata_wcov(m, w, wtype)
    if (covariance) C else stats::cov2cor(C)
  }
  print(round(r, 4))
  if (means) {
    if (is.null(w)) stata_summarize(as.data.frame(m), table = FALSE) else stata_summarize(as.data.frame(m), w = w, wtype = wtype, table = FALSE)
  }
  # Stata's r(): the sum of analytic weights, the count, the first pair's
  # correlation (or with covariance the first two variances and their
  # covariance, 0 when there is one variable) and the matrix
  if (ret) {
    rr <- list()
    if (!is.null(w) && wtype == "a") rr$sum_w <- sum(w)
    rr$N <- if (!is.null(w) && wtype == "f") sum(w) else nrow(m)
    two <- ncol(m) > 1
    if (covariance) {
      rr$cov_12 <- if (two) r[1, 2] else 0
      rr$Var_2 <- if (two) r[2, 2] else 0
      rr$Var_1 <- r[1, 1]
    } else {
      rr$rho <- if (two) r[1, 2] else NA_real_
    }
    rr$C <- r
    assign("stata_r", rr, envir = env)
  }
  invisible(r)
}"#;

impl Session {
    // ---------------- summarize ----------------

    pub(super) fn summarize(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "summarize")?;
        if let Some(o) = opts.other(&[
            ("detail", 1),
            ("meanonly", 4),
            ("separator", 3),
            ("format", 1),
        ]) {
            return Err(no(format!(
                "summarize with the option {o} is not translated yet"
            )));
        }
        let detail = opts.has("detail", 1);
        let allowed: &[W] = if detail {
            &[W::F, W::A]
        } else {
            &[W::F, W::A, W::I]
        };
        let w = self.weight_of(c, allowed)?;
        if opts.has("meanonly", 4) {
            if detail {
                return Err(no("only one of meanonly or detail may be specified"));
            }
            // prints nothing; what it keeps is Stata's r() of the last variable
            let (_, src, wargs) = self.weighted_frame(c, out, &w)?;
            self.helpers.insert("stata_summarize");
            out.line(format!(
                "stata_r <- stata_summarize({src}, meanonly = TRUE{wargs})"
            ));
            out.caveat("prints nothing, as in Stata; r() is kept in R as stata_r");
            self.r_new = Some((super::returned::R_MEANONLY, "summarize".into()));
            return Ok(());
        }
        let (_, src, wargs) = self.weighted_frame(c, out, &w)?;
        for h in ["stata_summarize", "stata_tables", "stata_fmt"] {
            self.helpers.insert(h);
        }
        if !matches!(w, Some((W::I, _))) {
            self.r_new = Some((
                if detail {
                    super::returned::R_SUMMARIZE_DETAIL
                } else {
                    super::returned::R_SUMMARIZE
                },
                "summarize".into(),
            ));
        }
        out.line(format!(
            "stata_summarize({src}{}{wargs}, ret = TRUE{})",
            if detail { ", detail = TRUE" } else { "" },
            if self.quiet(c) { ", print = FALSE" } else { "" }
        ));
        if self.quiet(c) {
            return Ok(());
        }
        if detail || w.is_some() {
            out.caveat("printed in R's layout, not Stata's table");
        }
        if detail {
            out.caveat(
                "skewness and kurtosis are Stata's, over the whole sample, so a normal sample's \
                 kurtosis is near 3",
            );
            out.caveat(
                "the percentiles are Stata's rule, which is not R's default (quantile type 2)",
            );
        }
        Ok(())
    }

    // ---------------- tabulate ----------------

    pub(super) fn tabulate(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "tabulate")?;
        let vars: Vec<&str> = c.parts.main.split_whitespace().collect();
        let (x, y) =
            match vars.as_slice() {
                [x] => (*x, None),
                [x, y] => (*x, Some(*y)),
                [] => return Err(no("tabulate needs a variable")),
                _ => return Err(no(
                    "too many variables specified: tabulate takes one or two (tab1 and tab2 take \
                     more)",
                )),
            };
        if let Some(v) = opts.arg("summarize", 2) {
            let v = v.to_string();
            for n in [Some(x), y].into_iter().flatten() {
                if !is_name(n) || n.len() > 10 {
                    return Err(no(format!(
                        "tabulate {n}, summarize(): a variable name of up to 10 characters is translated"
                    )));
                }
            }
            return self.tabulate_summarize(c, opts, out, x, y, &v);
        }
        let known: &[(&str, usize)] = if y.is_some() {
            &[
                ("missing", 1),
                ("nolabel", 3),
                ("row", 3),
                ("column", 3),
                ("cell", 4),
                ("chi2", 4),
                ("lrchi2", 6),
                ("nofreq", 3),
                ("nokey", 5),
                ("matcell", 7),
                ("matrow", 6),
                ("matcol", 6),
            ]
        } else {
            &[
                ("missing", 1),
                ("nolabel", 3),
                ("sort", 4),
                ("generate", 3),
                ("matcell", 7),
                ("matrow", 6),
            ]
        };
        if let Some(o) = opts.other(known) {
            return Err(no(format!(
                "tabulate with the option {o} is not translated yet"
            )));
        }
        for v in [Some(x), y].into_iter().flatten() {
            if !is_name(v) {
                return Err(no(format!("{v} is not a variable name")));
            }
        }
        let w = self.weight_of(c, &[W::F, W::A, W::I])?;
        tab_chi2_weight(opts, &w)?;
        let (src, _, wargs) = self.weighted_frame(c, out, &w)?;
        self.helpers.insert("stata_tab_values");
        self.helpers.insert("stata_tabulate");
        self.helpers.insert("stata_tabpct");
        // importance weights' r() was not checked against Stata
        if !matches!(w, Some((W::I, _))) {
            self.r_new = Some((
                if y.is_some() {
                    super::returned::R_TAB2
                } else {
                    super::returned::R_TAB1
                },
                "tabulate".into(),
            ));
        }
        let nolabel = opts.has("nolabel", 3);
        // with missing, .a to .z each get a row of their own
        let tags = opts.has("missing", 1) && self.mv_on(out);
        let value = |v: &str| {
            format!(
                "stata_tab_values({src}${}{}{})",
                r_name(v),
                if nolabel { ", nolabel = TRUE" } else { "" },
                if tags { ", tags = TRUE" } else { "" }
            )
        };
        let mut args = vec![value(x)];
        if let Some(y) = y {
            args.push(value(y));
        }
        for (name, min) in [
            ("missing", 1),
            ("sort", 4),
            ("row", 3),
            ("column", 3),
            ("cell", 4),
            ("chi2", 4),
            ("lrchi2", 6),
            ("nofreq", 3),
        ] {
            if opts.has(name, min) {
                args.push(format!("{name} = TRUE"));
            }
        }
        if opts.has("nokey", 5) {
            args.push("key = FALSE".into());
        }
        // the names make the table Stata's
        args.push(format!("xname = {}", r_text(x)));
        if let Some(y) = y {
            args.push(format!("yname = {}", r_text(y)));
        }
        self.helpers.insert("stata_tables");
        self.helpers.insert("stata_fmt");
        // generate(): the indicators, on every row of the data
        let generate = match opts.arg("generate", 3) {
            Some(stub) => {
                let stub = stub.trim();
                if !is_name(stub) {
                    return Err(no(format!("generate({stub}) needs a name")));
                }
                let d = self.data()?;
                let sel = match self.condition(c, out)? {
                    Some(cond) => {
                        format!("dplyr::pull(dplyr::mutate({d}, stata_sel = {cond}), stata_sel)")
                    }
                    None => "TRUE".into(),
                };
                self.helpers.insert("stata_tab_generate");
                out.package("dplyr");
                Some(format!(
                    "{d} <- stata_tab_generate({d}, {}, {}, sel = {sel}{}{}{})",
                    r_text(x),
                    r_text(stub),
                    if opts.has("missing", 1) {
                        ", missing = TRUE"
                    } else {
                        ""
                    },
                    if nolabel { ", nolabel = TRUE" } else { "" },
                    if tags { ", tags = TRUE" } else { "" }
                ))
            }
            None => None,
        };
        let call = format!("stata_tabulate({}{wargs}, ret = TRUE)", args.join(", "));
        // the matrices first: Stata stops before the table when one is not allowed
        self.tab_mats(opts, out, &src, x, y, &wargs)?;
        if self.quiet(c) {
            // nothing shown, r() still left
            out.line(format!("invisible(utils::capture.output({call}))"));
            out.code.extend(generate);
            return Ok(());
        }
        out.line(call);
        out.code.extend(generate);
        if matches!(w, Some((W::A | W::I, _)))
            || (y.is_some() && x.len() > 10)
            || y.is_some_and(|y| y.len() > 33)
        {
            out.caveat("printed in R's layout, not Stata's table");
        }
        if opts.has("chi2", 4) {
            out.caveat("Pearson's chi-squared, with no continuity correction, as Stata does it");
        }
        Ok(())
    }

    // ---------------- correlate ----------------

    pub(super) fn correlate(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "correlate")?;
        if let Some(o) = opts.other(&[("covariance", 3), ("means", 1)]) {
            return Err(no(format!(
                "correlate with the option {o} is not translated yet"
            )));
        }
        let w = self.weight_of(c, &[W::F, W::A])?;
        let (_, src, wargs) = self.weighted_frame(c, out, &w)?;
        self.helpers.insert("stata_correlate");
        if w.is_some() {
            self.helpers.insert("stata_wcov");
        }
        let mut args = vec![src + &wargs];
        if opts.has("covariance", 3) {
            args.push("covariance = TRUE".into());
        }
        if opts.has("means", 1) {
            args.push("means = TRUE".into());
            self.helpers.insert("stata_summarize");
        }
        args.push("ret = TRUE".into());
        self.r_new = Some((super::returned::R_CORRELATE, "correlate".into()));
        let call = format!("stata_correlate({})", args.join(", "));
        if self.quiet(c) {
            out.line(format!("invisible(utils::capture.output({call}))"));
            return Ok(());
        }
        out.line(call);
        out.caveat("a row with a missing value in any of the variables is dropped, as Stata does");
        out.caveat("printed in R's layout, not Stata's table");
        Ok(())
    }

    // ---------------- ttest ----------------

    pub(super) fn ttest(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "ttest")?;
        if opts.has("welch", 5) {
            return Err(no(
                "ttest, welch asks for Welch's degrees of freedom; R's t.test offers only \
                 Satterthwaite's, which is Stata's unequal",
            ));
        }
        if let Some(o) = opts.other(&[("by", 2), ("unequal", 3), ("unpaired", 5), ("level", 3)]) {
            return Err(no(format!(
                "ttest with the option {o} is not translated yet"
            )));
        }
        let level = match opts.arg("level", 3) {
            Some(l) => {
                let n: f64 = l
                    .trim()
                    .parse()
                    .map_err(|_| no(format!("level({l}) is not a number")))?;
                if !(0.0..100.0).contains(&n) {
                    return Err(no(format!("level({l}) is not between 0 and 100")));
                }
                Some(n / 100.0)
            }
            None => None,
        };
        let equal = !opts.has("unequal", 3);
        let src = self.rows_for(c, out)?;
        let main = c.parts.main.trim().to_string();
        let group = opts.arg("by", 2);
        let mut args: Vec<String> = Vec::new();
        let mut two_sample = false;
        // what stata_ttest_r() takes to leave Stata's r() of this test
        let r_args;
        match main.split_once("==") {
            Some((left, right)) => {
                let (left, right) = (left.trim(), right.trim());
                if !is_name(left) {
                    return Err(no(format!("{left} is not a variable name")));
                }
                if group.is_some() {
                    return Err(no("ttest takes either == or by(), not both"));
                }
                let first = format!("{src}${}", r_name(left));
                args.push(first.clone());
                if is_name(right) {
                    let second = format!("{src}${}", r_name(right));
                    args.push(second.clone());
                    if opts.has("unpaired", 5) {
                        two_sample = true;
                        args.push(format!("var.equal = {}", r_logical(equal)));
                        r_args = format!("{first}, y = {second}");
                    } else {
                        args.push("paired = TRUE".into());
                        r_args = format!("{first}, y = {second}, paired = TRUE");
                    }
                } else {
                    let v: f64 = right
                        .parse()
                        .map_err(|_| no(format!("{right} is neither a variable nor a number")))?;
                    if opts.has("unpaired", 5) {
                        return Err(no("ttest var == number has nothing to unpair"));
                    }
                    args.push(format!("mu = {}", r_number(v)));
                    r_args = format!("{first}, mu = {}", r_number(v));
                }
            }
            None => {
                let v = main.as_str();
                if !is_name(v) {
                    return Err(no(format!("{v} is not a variable name")));
                }
                let Some(g) = group else {
                    return Err(no("ttest of one variable needs == or by()"));
                };
                let g = g.trim();
                if !is_name(g) {
                    return Err(no(format!("by({g}) is not a variable name")));
                }
                two_sample = true;
                r_args = format!("{src}${}, {src}${}", r_name(v), r_name(g));
                args.push(format!("{} ~ {}", r_name(v), r_name(g)));
                args.push(format!("data = {src}"));
                args.push(format!("var.equal = {}", r_logical(equal)));
            }
        }
        if let Some(l) = level {
            args.push(format!("conf.level = {}", r_number(l)));
        }
        // Stata's r() of the test, quietly or not
        self.helpers.insert("stata_ttest_r");
        out.line(format!(
            "stata_r <- stata_ttest_r({r_args}, level = {}, equal = {})",
            r_number(super::returned::percent(level.unwrap_or(0.95))),
            r_logical(equal)
        ));
        self.r_new = Some((super::returned::R_TTEST, "ttest".into()));
        if self.quiet(c) {
            return Ok(());
        }
        out.line(format!("print(stats::t.test({}))", args.join(", ")));
        out.caveat("R prints the two-sided p-value; Stata prints the one-sided ones beside it");
        if two_sample && equal {
            out.caveat(
                "the groups are taken to have the same variance, as Stata does without unequal",
            );
        }
        out.caveat("printed in R's layout, not Stata's table");
        Ok(())
    }

    // ---------------- shared ----------------

    pub(super) fn no_by(&self, c: &Command, name: &str) -> Result<(), Untranslated> {
        if c.prefixes.iter().any(|p| matches!(p, Prefix::By { .. })) {
            // Stata's own words for the commands that read a model
            if matches!(name, "predict" | "test" | "lincom" | "margins" | "estat") {
                return Err(no(format!("{name} may not be combined with by")));
            }
            return Err(no(format!("by with {name} is not translated yet")));
        }
        Ok(())
    }

    /// The data frame a statistics command works on: the rows an `if`/`in`
    /// keeps, with every column.
    pub(super) fn rows_for(&mut self, c: &Command, out: &mut Out) -> Result<String, Untranslated> {
        let mut d = self.data()?;
        // L.x and the like are worked out over every row, before if and in
        let cols = self.ts_columns(&c.parts.main)?;
        if MODELS.contains(&c.name.as_str()) {
            self.fit_ts = cols.clone();
        }
        if let Some(cols) = cols {
            out.package("dplyr");
            d = format!("dplyr::mutate({d}, {cols})");
        }
        match self.condition(c, out)? {
            Some(cond) => {
                out.package("dplyr");
                Ok(format!("dplyr::filter({d}, {cond})"))
            }
            None => Ok(d),
        }
    }

    /// The same, with the varlist's columns chosen (all of them when the
    /// command names none).
    pub(super) fn frame_for(&mut self, c: &Command, out: &mut Out) -> Result<String, Untranslated> {
        let rows = self.rows_for(c, out)?;
        if c.parts.main.trim().is_empty() {
            return Ok(rows);
        }
        out.package("dplyr");
        Ok(format!("dplyr::select({rows}, {})", select(&c.parts.main)?))
    }
}

pub(super) fn r_logical(b: bool) -> &'static str {
    if b {
        "TRUE"
    } else {
        "FALSE"
    }
}

/// `tab1`: a one-way table of each variable, each on its own rows.
pub(super) const TAB1_HELPER: &str = r#"stata_tab1 <- function(d, nolabel = FALSE, tags = FALSE, ret = FALSE, ...) {
  out <- list()
  for (n in names(d)) {
    cat("\n-> tabulation of ", n, "  \n", sep = "")
    out[[n]] <- stata_tabulate(stata_tab_values(d[[n]], nolabel, tags), ret = ret, xname = n, ...)
  }
  # Stata leaves the last table's r()
  if (ret) assign("stata_r", stata_r, envir = parent.frame())
  invisible(out)
}"#;

/// `tab2`: a two-way table of every pair (of the first with each other with
/// `firstonly`), each pair on the rows where both are present.
pub(super) const TAB2_HELPER: &str = r#"stata_tab2 <- function(d, firstonly = FALSE, nolabel = FALSE, tags = FALSE, ret = FALSE, ...) {
  n <- names(d)
  if (length(n) < 2) stop("too few variables specified")
  out <- list()
  for (i in seq_len(if (firstonly) 1 else length(n) - 1)) {
    for (j in (i + 1):length(n)) {
      cat("\n-> tabulation of ", n[i], " by ", n[j], "  \n", sep = "")
      out[[paste(n[i], n[j])]] <- stata_tabulate(stata_tab_values(d[[n[i]]], nolabel, tags),
                                                  stata_tab_values(d[[n[j]]], nolabel, tags), ret = ret,
                                                  xname = n[i], yname = n[j], ...)
    }
  }
  # Stata leaves the last table's r()
  if (ret) assign("stata_r", stata_r, envir = parent.frame())
  invisible(out)
}"#;

/// `pwcorr`: each correlation on the rows where both variables are present
/// (all of them with `listwise`), its p-value from t on n - 2 degrees of
/// freedom, and Bonferroni's and Sidak's adjustments over the number of pairs.
/// `star()` and `print()` read the adjusted p-value, as Stata's do.
pub(super) const PWCORR_HELPER: &str = r#"stata_pwcorr <- function(d, obs = FALSE, sig = FALSE, star = NULL, print = NULL,
                          bonferroni = FALSE, sidak = FALSE, listwise = FALSE,
                          w = NULL, wtype = "f", ret = FALSE) {
  for (n in names(d)) {
    if (is.character(d[[n]]) || is.factor(d[[n]])) {
      cat("(", n, " ignored because string variable)\n", sep = "")
      d[[n]] <- NULL
    }
  }
  if (length(d) < 2) stop("too few variables specified")
  m <- as.matrix(as.data.frame(lapply(d, function(v) {
    u <- unclass(v)
    attributes(u) <- NULL
    as.numeric(u)
  })))
  colnames(m) <- names(d)
  if (!is.null(w)) w <- as.numeric(w)
  if (listwise) {
    cc <- stats::complete.cases(m)
    m <- m[cc, , drop = FALSE]
    if (!is.null(w)) w <- w[cc]
  }
  k <- ncol(m)
  pairs <- k * (k - 1) / 2
  dn <- list(names(d), names(d))
  C <- matrix(NA_real_, k, k, dimnames = dn)
  P <- matrix(NA_real_, k, k, dimnames = dn)
  N <- matrix(0, k, k, dimnames = dn)
  for (i in seq_len(k)) {
    for (j in seq_len(i)) {
      ok <- !is.na(m[, i]) & !is.na(m[, j])
      n <- if (!is.null(w) && wtype == "f") sum(w[ok]) else sum(ok)
      N[i, j] <- N[j, i] <- n
      if (n < 2) next
      r <- if (is.null(w)) suppressWarnings(stats::cor(m[ok, i], m[ok, j])) else {
        S <- stata_wcov(m[ok, c(i, j), drop = FALSE], w[ok], wtype)
        S[1, 2] / sqrt(S[1, 1] * S[2, 2])
      }
      C[i, j] <- C[j, i] <- r
      if (i == j || n < 3 || is.na(r)) next
      t <- r * sqrt((n - 2) / (1 - r^2))
      p <- 2 * stats::pt(-abs(t), n - 2)
      if (bonferroni) p <- min(1, p * pairs)
      if (sidak) p <- min(1, 1 - (1 - p)^pairs)
      P[i, j] <- P[j, i] <- p
    }
  }
  # Stata 19's table: the names cut to 8 over the columns and 12 beside the rows, each
  # correlation a mark (* for star(), else a blank) and %8.4f, the mark after the last
  # ending the line; the p-values and counts below each row, then a blank row; seven
  # columns to a block at 80 characters
  cut <- function(s, w) stata_tab_abbrev(s, w)
  f8 <- function(v, digits) if (is.na(v)) formatC(".", width = 8) else formatC(sprintf(paste0("%.", digits, "f"), v), width = 8)
  shown <- function(i, j) i == j || is.null(print) || (!is.na(P[i, j]) && P[i, j] <= print)
  ls <- if (exists("stata_linesize")) stata_linesize else 80
  per <- max(1, (ls - 15) %/% 9)
  lines <- character()
  blocks <- split(seq_len(k), (seq_len(k) - 1) %/% per)
  for (bi in seq_along(blocks)) {
    cols <- blocks[[bi]]
    if (bi > 1) lines <- c(lines, "")
    lines <- c(lines, paste0(strrep(" ", 13), "| ", paste(formatC(cut(names(d)[cols], 8), width = 8), collapse = " ")),
               paste0(strrep("-", 13), "+", strrep("-", 9 * length(cols))))
    for (i in cols[1]:k) {
      js <- cols[cols <= i]
      mark <- " "
      r_line <- ""
      for (j in js) {
        r_line <- paste0(r_line, mark, if (shown(i, j)) f8(C[i, j], 4) else strrep(" ", 8))
        mark <- if (i != j && !is.null(star) && shown(i, j) && !is.na(P[i, j]) && P[i, j] <= star) "*" else " "
      }
      lines <- c(lines, paste0(formatC(cut(names(d)[i], 12), width = 12), " |", r_line, mark))
      if (sig) {
        off <- js[js != i]
        lines <- c(lines, paste0(strrep(" ", 13), "|", paste0(vapply(off, function(j) paste0(" ", if (shown(i, j)) f8(P[i, j], 4) else strrep(" ", 8)), ""), collapse = "")))
      }
      if (obs) lines <- c(lines, paste0(strrep(" ", 13), "|", paste0(" ", formatC(sprintf("%.0f", N[i, js]), width = 8), collapse = "")))
      if (sig || obs) lines <- c(lines, paste0(strrep(" ", 13), "|"))
    }
  }
  cat("\n", paste0(lines, "\n"), sep = "")
  # Stata's r(): the first pair's count and correlation, and the matrices
  if (ret) assign("stata_r", c(list(N = N[2, 1], rho = C[2, 1], Nobs = N, C = C), if (sig) list(sig = P)),
                  envir = parent.frame())
  invisible(list(C = C, sig = P, Nobs = N))
}"#;

impl Session {
    // ---------------- tab1, tab2 ----------------

    pub(super) fn tab1(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "tab1")?;
        let w = self.weight_of(c, &[W::F, W::A, W::I])?;
        if let Some(o) = opts.other(&[("missing", 1), ("nolabel", 3), ("sort", 4)]) {
            return Err(no(format!(
                "tab1 with the option {o} is not translated yet"
            )));
        }
        if c.parts.main.trim().is_empty() {
            return Err(no("varlist required"));
        }
        let (_, src, wargs) = self.weighted_frame(c, out, &w)?;
        for h in [
            "stata_tab_values",
            "stata_tabulate",
            "stata_tabpct",
            "stata_tab1",
            "stata_tables",
            "stata_fmt",
        ] {
            self.helpers.insert(h);
        }
        let mut args = tab_args(src + &wargs, opts, &[("missing", 1), ("sort", 4)]);
        if opts.has("missing", 1) && self.mv_on(out) {
            args.push("tags = TRUE".into());
        }
        // importance weights' r() was not checked against Stata
        if !matches!(w, Some((W::I, _))) {
            args.push("ret = TRUE".into());
            self.r_new = Some((super::returned::R_TAB1, "tab1".into()));
        }
        let call = format!("stata_tab1({})", args.join(", "));
        if self.quiet(c) {
            // nothing shown, r() still left
            out.line(format!("invisible(utils::capture.output({call}))"));
            return Ok(());
        }
        out.line(call);
        out.caveat("each table has the rows where its own variable is present, as in Stata");
        if matches!(w, Some((W::A | W::I, _))) {
            out.caveat("printed in R's layout, not Stata's table");
        }
        Ok(())
    }

    pub(super) fn tab2(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "tab2")?;
        let w = self.weight_of(c, &[W::F, W::A, W::I])?;
        tab_chi2_weight(opts, &w)?;
        let known = [
            ("firstonly", 9),
            ("missing", 1),
            ("nolabel", 3),
            ("row", 3),
            ("column", 3),
            ("cell", 4),
            ("chi2", 4),
        ];
        if let Some(o) = opts.other(&known) {
            return Err(no(format!(
                "tab2 with the option {o} is not translated yet"
            )));
        }
        if c.parts.main.trim().is_empty() {
            return Err(no("varlist required"));
        }
        let (_, src, wargs) = self.weighted_frame(c, out, &w)?;
        for h in [
            "stata_tab_values",
            "stata_tabulate",
            "stata_tabpct",
            "stata_tab2",
            "stata_tables",
            "stata_fmt",
        ] {
            self.helpers.insert(h);
        }
        let mut args = tab_args(
            src + &wargs,
            opts,
            &[
                ("missing", 1),
                ("row", 3),
                ("column", 3),
                ("cell", 4),
                ("chi2", 4),
            ],
        );
        if opts.has("missing", 1) && self.mv_on(out) {
            args.push("tags = TRUE".into());
        }
        if opts.has("firstonly", 9) {
            args.push("firstonly = TRUE".into());
        }
        // importance weights' r() was not checked against Stata
        if !matches!(w, Some((W::I, _))) {
            args.push("ret = TRUE".into());
            self.r_new = Some((super::returned::R_TAB2, "tab2".into()));
        }
        let call = format!("stata_tab2({})", args.join(", "));
        if self.quiet(c) {
            // nothing shown, r() still left
            out.line(format!("invisible(utils::capture.output({call}))"));
            return Ok(());
        }
        out.line(call);
        out.caveat("each table has the rows where both of its variables are present, as in Stata");
        if opts.has("chi2", 4) {
            out.caveat("Pearson's chi-squared, with no continuity correction, as Stata does it");
        }
        let percents = [("row", 3), ("column", 3), ("cell", 4)]
            .iter()
            .any(|(o, m)| opts.has(o, *m));
        if percents || matches!(w, Some((W::A | W::I, _))) {
            out.caveat("printed in R's layout, not Stata's table");
        }
        Ok(())
    }

    // ---------------- pwcorr ----------------

    pub(super) fn pwcorr(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "pwcorr")?;
        let w = self.weight_of(c, &[W::F, W::A])?;
        if let Some(o) = opts.other(&[
            ("obs", 1),
            ("sig", 3),
            ("star", 2),
            ("print", 2),
            ("bonferroni", 1),
            ("sidak", 4),
            ("listwise", 5),
            ("casewise", 4),
        ]) {
            return Err(no(format!(
                "pwcorr with the option {o} is not translated yet"
            )));
        }
        if opts.has("bonferroni", 1) && opts.has("sidak", 4) {
            return Err(no(
                "invalid syntax: bonferroni and sidak may not be combined",
            ));
        }
        let (_, src, wargs) = self.weighted_frame(c, out, &w)?;
        self.helpers.insert("stata_pwcorr");
        self.helpers.insert("stata_tables");
        if w.is_some() {
            self.helpers.insert("stata_wcov");
        }
        let mut args = vec![src + &wargs];
        if opts.has("print", 2) && opts.has("obs", 1) {
            return Err(no(
                "pwcorr with print() and obs is not translated yet (not observed in Stata 19)",
            ));
        }
        for (name, min) in [("obs", 1), ("sig", 3), ("bonferroni", 1), ("sidak", 4)] {
            if opts.has(name, min) {
                args.push(format!("{name} = TRUE"));
            }
        }
        let listwise = opts.has("listwise", 5) || opts.has("casewise", 4);
        if listwise {
            args.push("listwise = TRUE".into());
        }
        for (name, min) in [("star", 2), ("print", 2)] {
            if let Some(a) = opts.arg(name, min) {
                let p: f64 = a
                    .trim()
                    .parse()
                    .map_err(|_| no(format!("{name}({a}) is not a number")))?;
                args.push(format!("{name} = {}", r_number(p)));
            }
        }
        args.push("ret = TRUE".into());
        self.r_new = Some((super::returned::R_CORRELATE, "pwcorr".into()));
        let call = format!("stata_pwcorr({})", args.join(", "));
        if self.quiet(c) {
            // nothing shown, r() still left
            out.line(format!("invisible(utils::capture.output({call}))"));
            return Ok(());
        }
        out.line(call);
        if listwise {
            out.caveat(
                "a row with a missing value in any of the variables is dropped, as Stata does",
            );
        } else {
            out.caveat("each correlation has the rows where both of its variables are present");
        }
        Ok(())
    }
}

/// The source and the `name = TRUE` flags a tabulation passes on.
fn tab_args(src: String, opts: &Opts, flags: &[(&str, usize)]) -> Vec<String> {
    let mut args = vec![src];
    if opts.has("nolabel", 3) {
        args.push("nolabel = TRUE".into());
    }
    for (name, min) in flags {
        if opts.has(name, *min) {
            args.push(format!("{name} = TRUE"));
        }
    }
    args
}

/// The groups of a `by` prefix, in the order of the rows: which rows, and the
/// `g = 0, h = 1` heading Stata prints (value labels where there are some, `.`
/// for missing). Without `bysort` the rows must already be in order, missing
/// last and strings by their bytes, or it stops with Stata's `not sorted`. With
/// `tags` each of `.` and `.a` to `.z` is a group of its own, in that order.
pub(super) const BY_GROUPS_HELPER: &str = r#"stata_by_groups <- function(d, vars, within = character(0), sorted = FALSE, tags = FALSE) {
  value <- function(v) {
    if (is.factor(v)) v <- as.character(v)
    u <- unclass(v)
    attributes(u) <- NULL
    u
  }
  n <- nrow(d)
  keys <- function(vs) unlist(lapply(vs, function(v) {
    u <- value(d[[v]])
    if (tags && is.double(u) && anyNA(u)) list(u, stata_mv(u)) else list(u)
  }), recursive = FALSE)
  group <- keys(vars)
  key <- c(group, keys(within))
  if (!sorted && n > 1) {
    o <- do.call(order, c(unname(key), list(na.last = TRUE, method = "radix")))
    if (any(o != seq_len(n))) stop("not sorted")
  }
  if (n == 0) return(list())
  same <- function(a) {
    x <- a[-1]
    y <- a[-n]
    (is.na(x) & is.na(y)) | (!is.na(x) & !is.na(y) & x == y)
  }
  new <- if (n > 1) c(TRUE, !Reduce(`&`, lapply(group, same))) else TRUE
  label <- function(v, i) {
    x <- d[[v]]
    u <- value(x)[i]
    if (is.na(u)) return(if (is.character(u)) "" else if (tags) c(".", paste0(".", letters))[stata_mv(u)] else ".")
    l <- attr(x, "labels")
    if (!is.null(l) && is.numeric(u) && any(l == u)) return(names(l)[match(u, l)])
    as.character(u)
  }
  lapply(unname(split(seq_len(n), cumsum(new))), function(r) {
    list(rows = r, label = paste(vapply(vars, function(v) paste(v, "=", label(v, r[1])), ""),
                                 collapse = ", "))
  })
}"#;

/// Stata takes `chi2` only with frequency weights.
fn tab_chi2_weight(opts: &Opts, w: &Weighting) -> Result<(), Untranslated> {
    match w {
        Some((W::A, _)) if opts.has("chi2", 4) => Err(no("chi2 not allowed with aweights")),
        Some((W::I, _)) if opts.has("chi2", 4) => Err(no("option chi2 not allowed")),
        // not seen in Stata with these weights
        Some((W::A | W::I, _)) if opts.has("lrchi2", 6) => Err(no(
            "tabulate, lrchi2 with analytic or importance weights is not translated yet",
        )),
        _ => Ok(()),
    }
}

/// What `_pctile` leaves in r(): r(r1), r(r2), ... (one for each percentile
/// asked for; a number past them reads as missing, as in Stata).
pub(super) const R_PCTILE: &[&str] = &[
    "r1", "r2", "r3", "r4", "r5", "r6", "r7", "r8", "r9", "r10", "r11", "r12", "r13", "r14", "r15",
    "r16", "r17", "r18", "r19", "r20", "r21", "r22", "r23", "r24", "r25", "r26", "r27", "r28",
    "r29", "r30",
];

impl Session {
    /// `_pctile varname [if] [in], percentiles(numlist) | nquantiles(#)`:
    /// Stata's default percentiles (the mean of the two values at a whole
    /// rank), into r(r1), r(r2), ...
    pub(super) fn pctile_r(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[("percentiles", 1), ("nquantiles", 1)]) {
            return Err(no(format!(
                "_pctile with the option {o} is not translated yet"
            )));
        }
        if c.parts.weight.is_some() {
            return Err(no("_pctile with weights is not translated yet"));
        }
        let var = c.parts.main.trim();
        if !is_name(var) {
            return Err(no("_pctile with one variable name is translated"));
        }
        let ps: Vec<f64> = match (opts.arg("percentiles", 1), opts.arg("nquantiles", 1)) {
            (Some(p), None) => {
                let mut v = Vec::new();
                for w in p
                    .split(|ch: char| ch == ',' || ch.is_whitespace())
                    .filter(|w| !w.is_empty())
                {
                    let x: f64 = w.parse().map_err(|_| {
                        no(format!(
                            "_pctile, percentiles({p}): only numbers are translated (not ranges)"
                        ))
                    })?;
                    if !(0.0..=100.0).contains(&x) {
                        return Err(no(format!("_pctile, percentiles({p}): each must be between 0 and 100 (Stata: r(198))")));
                    }
                    v.push(x);
                }
                v
            }
            (None, Some(n)) => {
                let n: u32 = n
                    .trim()
                    .parse()
                    .map_err(|_| no("_pctile, nquantiles(): a whole number is needed"))?;
                if n < 2 {
                    return Err(no("_pctile, nquantiles() needs 2 or more (Stata: r(198))"));
                }
                (1..n)
                    .map(|i| 100.0 * f64::from(i) / f64::from(n))
                    .collect()
            }
            (None, None) => vec![50.0],
            _ => {
                return Err(no(
                    "_pctile takes percentiles() or nquantiles(), not both (Stata: r(198))",
                ))
            }
        };
        if ps.len() > R_PCTILE.len() {
            return Err(no(format!(
                "_pctile with more than {} percentiles is not translated yet",
                R_PCTILE.len()
            )));
        }
        let src = self.rows_for(c, out)?;
        self.helpers.insert("stata_stats");
        let list: Vec<String> = ps.iter().map(|p| r_number(*p)).collect();
        out.line(format!(
            "stata_r <- local({{ x <- as.numeric({src}[[{}]]); p <- c({}); stats::setNames(as.list(vapply(p, function(q) stata_pctile(x, q), 0)), paste0(\"r\", seq_along(p))) }})",
            r_text(var),
            list.join(", ")
        ));
        self.r_new = Some((R_PCTILE, "_pctile".into()));
        Ok(())
    }
}
