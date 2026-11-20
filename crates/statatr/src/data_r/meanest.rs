//! `mean`, `total`, `proportion` and `ratio` as Stata 19 estimates and
//! prints them (`plan/stata/mean.md`): e(b) and e(V) under Stata's names
//! (`y`, `c.y@0bn.b`, `1bn.g`, `_ratio_1`), the other e() and r(table).
//! The variances, checked against Stata 19 to the last digits: means over
//! their own rows (s²/n, frequency weights counting rows, analytic ones
//! scaled to the rows), each group of `over()` on its own; with pweights
//! the linearized (sandwich) variance, n/(n−1) Σ zᵢzᵢ′; totals are the sum
//! of the weights times the mean; proportions (diag(p) − pp′)/n with logit
//! intervals; ratios linearized.

use super::model::{Estimates, Fit};
use super::weights::W;
use super::*;

pub(super) const MEANEST_HELPER: &str = r##"stata_meanest <- function(kind, d, vars, w = NULL, wtype = "f", over = NULL, level = 95,
                          ratios = NULL, rnames = NULL, show = TRUE, cmdline = "", wexp = "", svy = NULL) {
  num <- function(u) { u <- unclass(u); attributes(u) <- NULL; as.numeric(u) }
  if (!is.null(svy) && show) cat("(running ", kind, " on estimation sample)\n", sep = "")
  # importance weights count as frequency weights (Stata 19: the same N and errors)
  wlab <- wtype
  if (identical(wtype, "i")) wtype <- "f"
  used <- unique(c(vars, over))
  for (v in used) {
    if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
    if (is.character(d[[v]])) stop(v, " is a string variable (Stata: type mismatch, r(109))", call. = FALSE)
  }
  keep <- stats::complete.cases(as.data.frame(lapply(d[used], num)))
  wv <- if (is.null(w)) rep(1, nrow(d)) else num(w)
  keep <- keep & !is.na(wv)
  # svy: the design variables must be present too; a subpop() variable too
  if (!is.null(svy)) {
    miss <- function(x) is.na(x) | (is.character(x) & x == "")
    for (v in c(svy$strata, svy$psu, svy$fpc, svy$subvar)) {
      if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
      keep <- keep & !miss(unclass(d[[v]]))
    }
    if (any(wv[keep] < 0)) stop("negative weights encountered (Stata stops)", call. = FALSE)
  }
  if (!any(keep)) stop("no observations (Stata: r(2000))", call. = FALSE)
  n <- sum(keep)
  wv <- wv[keep]
  if (!is.null(w) && wtype == "a") wv <- wv * n / sum(wv)
  # frequency weights count rows
  nobs <- if (!is.null(w) && wtype == "f") sum(wv) else n
  lin <- !is.null(w) && wtype == "p"
  col <- function(v) num(d[[v]])[keep]
  # the subpopulation (svy, subpop()): rows outside it keep their place in the design
  subp <- rep(TRUE, n)
  if (!is.null(svy) && !is.null(svy$subvar)) {
    sv <- num(d[[svy$subvar]])[keep]
    if (any(!sv %in% c(0, 1))) stop("subpop() with values other than 0 and 1 is not translated yet", call. = FALSE)
    subp <- sv != 0
  }
  labs_of <- function(v, lv) {
    vl <- attr(d[[v]], "labels", exact = TRUE)
    vapply(lv, function(u) {
      k <- if (!is.null(vl)) names(vl)[match(u, num(vl))] else NA
      if (length(k) && !is.na(k)) k else stata_fmt(u, "%9.0g", trim = TRUE)
    }, "")
  }
  lev_name <- function(u, first) paste0(stata_fmt(u, "%9.0g", trim = TRUE), if (first) "bn" else "", ".")
  # over(): each cell a combination of the variables' values, in their order (Stata 19: 1 0, 1 1, 2 0 ...)
  if (is.null(over)) {
    ov <- rep(1, n)
    ol <- 1
  } else {
    if (kind == "proportion") for (v in over) if (v %in% vars) stop("over variable ", v, " not allowed in varlist (Stata: r(198))", call. = FALSE)
    om <- matrix(vapply(over, col, numeric(n)), n)
    ov <- do.call(paste, c(as.data.frame(om), sep = "\r"))
    ord <- do.call(order, as.data.frame(om))
    first <- ord[!duplicated(ov[ord])]
    ocells <- om[first, , drop = FALSE]
    ol <- ov[first]
  }
  # a cell's name: each variable's level, its smallest written bn (c.y@1bn.g#0bn.b)
  oname <- function(j) paste(vapply(seq_along(over), function(i) {
    u <- ocells[j, i]
    paste0(lev_name(u, u == min(ocells[, i])), over[i])
  }, ""), collapse = "#")
  ohead <- paste(over, collapse = "#")
  # each estimate: its rows (a group of over()), the variable(s) it reads, its name and label
  est <- list()
  add <- function(e) est[[length(est) + 1]] <<- e
  if (kind %in% c("mean", "total")) {
    for (v in vars) for (j in seq_along(ol)) {
      add(list(sel = (ov == ol[j]) & subp, y = col(v), x = NULL, var = v, j = j,
               name = if (is.null(over)) v else paste0("c.", v, "@", oname(j))))
    }
  } else if (kind == "ratio") {
    for (r in seq_along(ratios)) for (j in seq_along(ol)) {
      add(list(sel = (ov == ol[j]) & subp, y = col(ratios[[r]][1]), x = col(ratios[[r]][2]), var = rnames[r], j = j,
               name = if (is.null(over)) rnames[r] else paste0("c.", rnames[r], "@", oname(j))))
    }
  } else {
    for (v in vars) {
      yv <- col(v)
      if (any(yv < 0 | yv != floor(yv))) stop(v, ": proportion takes values that are nonnegative integers (Stata: r(452))", call. = FALSE)
      lv <- sort(unique(yv))
      for (k in seq_along(lv)) for (j in seq_along(ol)) {
        add(list(sel = (ov == ol[j]) & subp, y = as.numeric(yv == lv[k]), x = NULL, var = v, j = j, k = k, val = lv[k],
                 name = paste0(lev_name(lv[k], k == 1), v, if (!is.null(over)) paste0("@", oname(j)))))
      }
    }
  }
  K <- length(est)
  b <- numeric(K)
  Wg <- numeric(K)
  Ng <- numeric(K)
  for (i in seq_len(K)) {
    e <- est[[i]]
    s <- e$sel
    Wg[i] <- sum(wv[s])
    # rows, whatever the weights (Stata 19)
    Ng[i] <- sum(s)
    b[i] <- if (kind == "ratio") sum(wv[s] * e$y[s]) / sum(wv[s] * e$x[s]) else sum(wv[s] * e$y[s]) / Wg[i]
  }
  V <- matrix(0, K, K)
  design <- NULL
  if (!is.null(svy)) {
    raw <- function(v) { x <- unclass(d[[v]]); attributes(x) <- NULL; x[keep] }
    st <- if (is.null(svy$strata)) rep(1, n) else raw(svy$strata)
    pu <- if (is.null(svy$psu)) seq_len(n) else raw(svy$psu)
    fp <- if (is.null(svy$fpc)) NULL else as.numeric(raw(svy$fpc))
    Z <- vapply(seq_len(K), function(i) {
      e <- est[[i]]
      s <- e$sel
      if (kind == "ratio") wv * s * (e$y - b[i] * e$x) / sum(wv[s] * e$x[s])
      else if (kind == "total") wv * s * e$y
      else wv * s * (e$y - b[i]) / Wg[i]
    }, numeric(n))
    design <- stata_svy_vlin(matrix(Z, n), st, pu, fp, svy$singleunit)
    V <- design$V
  } else if (lin || kind == "ratio") {
    # linearized: z_i of each estimate, V = n/(n-1) sum z z' (frequency weights count rows)
    Z <- vapply(seq_len(K), function(i) {
      e <- est[[i]]
      s <- e$sel
      if (kind == "ratio") wv * s * (e$y - b[i] * e$x) / sum(wv[s] * e$x[s]) else wv * s * (e$y - b[i]) / Wg[i]
    }, numeric(n))
    Z <- matrix(Z, n)
    # each group of over() on its own: its rows' n/(n-1), nothing across groups (Stata 19)
    jj <- vapply(est, `[[`, 0, "j")
    for (j in seq_along(ol)) {
      s <- ov == ol[j]
      cols <- which(jj == j)
      Zj <- Z[s, cols, drop = FALSE]
      # ratio takes each group's rows, mean, total and proportion with pweights the whole
      # sample's (Stata 19: n/(n-1) of all n)
      own <- kind == "ratio"
      if (!is.null(w) && wtype == "f") {
        # each row its weight's copies: z per copy
        m <- if (own) sum(wv[s]) else nobs
        V[cols, cols] <- m / (m - 1) * crossprod(Zj / sqrt(wv[s]))
      } else {
        m <- if (own) sum(s) else n
        V[cols, cols] <- m / (m - 1) * crossprod(Zj)
      }
    }
  } else {
    for (i in seq_len(K)) for (h in seq_len(K)) {
      ei <- est[[i]]
      eh <- est[[h]]
      if (ei$j != eh$j) next
      s <- ei$sel
      if (kind == "proportion") {
        if (ei$var != eh$var) {
          # two variables' proportions: the means of their indicators
          V[i, h] <- (sum(wv[s] * ei$y[s] * eh$y[s]) / Wg[i] - b[i] * b[h]) / Wg[i]
        } else {
          V[i, h] <- ((i == h) * b[i] - b[i] * b[h]) / Wg[i]
        }
      } else {
        # analytic weights scaled to each group's rows
        ww <- if (!is.null(w) && wtype == "a") wv[s] * sum(s) / sum(wv[s]) else wv[s]
        V[i, h] <- sum(ww * (ei$y[s] - b[i]) * (eh$y[s] - b[h])) / (sum(ww) - 1) / sum(ww)
      }
    }
  }
  if (kind == "total") {
    if (is.null(svy)) V <- V * outer(Wg, Wg)
    b <- b * Wg
  }
  nm <- vapply(est, `[[`, "", "name")
  dimnames(V) <- list(nm, nm)
  se <- if (isTRUE(design$na)) rep(NA_real_, K) else sqrt(pmax(diag(V), 0))
  dfr <- if (is.null(svy)) nobs - 1 else design$npsu - design$nstrata
  tq <- stats::qt(1 - (100 - level) / 200, dfr)
  if (kind == "proportion") {
    lg <- log(b / (1 - b))
    sl <- se / (b * (1 - b))
    lb <- 1 / (1 + exp(-(lg - tq * sl)))
    ub <- 1 / (1 + exp(-(lg + tq * sl)))
  } else {
    lb <- b - tq * se
    ub <- b + tq * se
  }
  tt <- b / se
  pv <- 2 * stats::pt(-abs(tt), dfr)
  tab <- rbind(b = b, se = se, t = tt, pvalue = pv, ll = lb, ul = ub, df = dfr, crit = tq, eform = 0)
  colnames(tab) <- nm
  title <- c(mean = "Mean", total = "Total", proportion = "Proportion", ratio = "Ratio")[[kind]]
  # Stata 19: ratio is linearized; mean, total and proportion keep analytic with pweights too
  vce <- if (kind == "ratio") "linearized" else "analytic"
  # e() in Stata 19's order: ratio with frequency weights puts N before N_over
  e <- if (kind == "ratio" && !is.null(w) && wtype == "f") list(df_r = dfr, N = nobs, N_over = length(ol))
       else list(df_r = dfr, N_over = length(ol), N = nobs)
  e <- c(e, list(k_eq = 1, rank = if (anyNA(V) || isTRUE(design$na)) 0 else qr(V)$rank, cmdline = cmdline, cmd = kind,
                 vce = if (kind == "ratio") "linearized" else if (lin) "robust" else "analytic"))
  if (kind == "ratio") e$vcetype <- "Linearized"
  e$title <- paste(title, "estimation")
  if (!is.null(w)) {
    e$wexp <- paste("=", wexp)
    e$wtype <- c(f = "fweight", a = "aweight", p = "pweight", i = "iweight")[[wlab]]
  }
  if (kind == "ratio") e$namelist <- paste(rnames, collapse = " ")
  e$estat_cmd <- "estat_vce_only"
  e$varlist <- if (kind == "proportion") paste(unique(sub("@.*$", "", sub("bn\\.", ".", nm))), collapse = " ") else paste(vars, collapse = " ")
  e$marginsnotok <- "_ALL"
  if (!is.null(over)) e$over <- paste(over, collapse = " ")
  e$properties <- "b V"
  e$b <- matrix(b, 1, dimnames = list("y1", nm))
  e$V <- V
  if (kind == "mean") {
    # the standard deviations, the weights scaled to the rows (frequency weights counting copies)
    sdv <- vapply(seq_len(K), function(i) {
      s <- est[[i]]$sel
      ww <- if (!is.null(w) && wtype != "f") wv[s] * sum(s) / sum(wv[s]) else wv[s]
      sqrt(sum(ww * (est[[i]]$y[s] - b[i])^2) / (sum(ww) - 1))
    }, 0)
    e$sd <- matrix(sdv, 1, dimnames = list("y1", nm))
    # for estat sd: each row's head and label, as the table shows them
    if (!is.null(over)) {
      attr(e$sd, "stata_heads") <- vapply(est, function(x) paste0("c.", x$var, "@", ohead), "")
      attr(e$sd, "stata_labs") <- vapply(est, function(x) paste(vapply(seq_along(over), function(i) labs_of(over[i], ocells[x$j, i]), ""), collapse = " "), "")
    }
  }
  if (kind == "proportion") e$freq <- matrix(vapply(seq_len(K), function(i) sum(wv[est[[i]]$sel] * est[[i]]$y[est[[i]]$sel]) *
                                                     (if (!is.null(w) && wtype == "a") sum(w[keep]) / n else 1), 0),
                                             1, dimnames = list("r1", nm))
  # one estimate: its row named as its column (Stata 19)
  e$`_N` <- matrix(Ng, 1, dimnames = list(if (K == 1) nm else "r1", nm))
  e$error <- matrix(0, 1, K, dimnames = list("r1", nm))
  if (!is.null(svy)) {
    # e() under svy: Stata 19's names and order
    npsub <- if (is.null(svy$subvar)) NA else length(unique(paste(st[subp], pu[subp])))
    has_sub <- !is.na(npsub)
    sc <- list(df_r = dfr)
    if (has_sub) sc$N_subpsu <- npsub
    sc <- c(sc, list(N_strata_omit = 0, singleton = design$single, census = 0))
    if (has_sub) sc <- c(sc, list(N_subpop = sum(wv[subp]), N_sub = sum(subp)))
    sc <- c(sc, list(N_pop = sum(wv), N_psu = design$npsu, N_strata = design$nstrata, N_over = length(ol),
                     N = n, stages = 1, k_eq = 1, rank = if (isTRUE(design$na)) 0 else qr(V)$rank))
    vl <- if (kind == "ratio") paste(unlist(ratios), collapse = " ") else e$varlist
    spec <- if (kind == "ratio") paste(sprintf("(%s:%s/%s)", rnames, vapply(ratios, `[`, "", 1), vapply(ratios, `[`, "", 2)), collapse = " ") else vl
    e2 <- c(sc, list(cmdline = svy$line, cmd = kind, prefix = "svy", cmdname = kind,
                     command = paste0(kind, " ", spec, ",", if (!is.null(over)) paste0(" over(", paste(over, collapse = " "), ")") else ""),
                     title = paste0("Survey: ", title, " estimation"), vcetype = "Linearized", vce = "linearized"))
    if (kind == "ratio") e2$namelist <- paste(rnames, collapse = " ")
    e2$estat_cmd <- "svy_estat"
    e2$varlist <- vl
    e2$marginsnotok <- "_ALL"
    if (!is.null(w)) {
      e2$wtype <- "pweight"
      e2$wvar <- svy$w
      e2$wexp <- paste("=", svy$w)
    }
    e2$singleunit <- svy$singleunit
    if (!is.null(svy$psu)) e2$su1 <- svy$psu
    if (!is.null(svy$strata)) e2$strata1 <- svy$strata
    if (!is.null(svy$fpc)) e2$fpc1 <- svy$fpc
    if (!is.null(over)) e2$over <- paste(over, collapse = " ")
    if (has_sub) e2$subpop <- svy$subtext
    e2$properties <- "b V"
    e2$b <- e$b
    e2$V <- V
    if (!is.null(over)) {
      cellpsu <- vapply(est, function(x) length(unique(paste(st[ov == ol[x$j]], pu[ov == ol[x$j]]))), 0)
      cellstr <- vapply(est, function(x) length(unique(st[ov == ol[x$j]])), 0)
      e2$`_N_overpsu` <- matrix(cellpsu, 1, dimnames = list("r1", nm))
      e2$`_N_overstrata` <- matrix(cellstr, 1, dimnames = list("r1", nm))
    }
    if (kind == "proportion") e2$freq <- e$freq
    e2$`_N_subp` <- matrix(Wg, 1, dimnames = list(if (K == 1) nm else "r1", nm))
    e2$`_N` <- e$`_N`
    e2$error <- e$error
    one <- function(v) matrix(v, 1, 1, dimnames = list("r1", "c1"))
    e2$`_N_strata_certain` <- one(0)
    e2$`_N_strata_single` <- one(design$single)
    e2$`_N_strata` <- one(design$nstrata)
    e <- e2
  }
  out <- list(e = e, b = stats::setNames(b, nm), V = V, df = dfr, r = list(table = tab))
  if (!show) return(out)
  g <- function(v) if (is.na(v)) "." else stata_fmt(v, "%9.0g", trim = TRUE)
  if (!is.null(svy)) {
    cat("\n", e$title, "\n\n", sep = "")
    fm <- function(x) format(x, big.mark = ",", scientific = FALSE, trim = TRUE)
    left <- list(c("Number of strata", fm(design$nstrata)), c("Number of PSUs  ", fm(design$npsu)))
    right <- list(c("Number of obs  ", fm(n)), c("Population size", fm(sum(wv))))
    if (has_sub) {
      left[[3]] <- c("Subpop. no. PSUs", fm(npsub))
      right <- c(right, list(c("Subpop. no. obs", fm(sum(subp))), c("Subpop. size   ", fm(sum(wv[subp])))))
    }
    right[[length(right) + 1]] <- c("Design df      ", fm(dfr))
    wl <- max(nchar(vapply(left, `[`, "", 2)))
    wr <- max(nchar(vapply(right, `[`, "", 2)))
    lw <- 19 + wl
    rw <- 18 + wr
    for (i in seq_along(right)) {
      l <- if (i <= length(left)) sprintf("%s = %*s", left[[i]][1], wl, left[[i]][2]) else ""
      cat(formatC(l, width = -lw), strrep(" ", 62 - lw - rw), sprintf("%s = %*s", right[[i]][1], wr, right[[i]][2]), "\n", sep = "")
    }
    vce <- "linearized"
    if (kind == "ratio") {
      cat("\n")
      for (r in seq_along(ratios)) cat(sprintf("%13s: %s/%s", rnames[r], ratios[[r]][1], ratios[[r]][2]), "\n", sep = "")
    }
  } else
  if (kind == "ratio") {
    cat("\n")
    for (r in seq_along(ratios)) cat(sprintf("%13s: %s/%s", rnames[r], ratios[[r]][1], ratios[[r]][2]), "\n", sep = "")
  }
  rule <- paste0(strrep("-", 62), "\n")
  cat("\n", rule, sep = "")
  if (!is.null(svy) && kind == "proportion") cat("             |             Linearized            Logit\n")
  else {
    if (kind == "proportion") cat("             |                                   Logit\n")
    if (vce == "linearized") cat("             |             Linearized\n")
  }
  cat(sprintf("             |%11s%s     [%s%% conf. interval]", title,
              if (vce == "linearized") "   std. err." else "   Std. err.", stata_fmt(level, "%9.0g", trim = TRUE)), "\n", sep = "")
  cat("-------------+", strrep("-", 48), "\n", sep = "")
  row <- function(lab, i) cat(sprintf("%12s |%11s%11s%14s%12s", lab, g(b[i]), g(se[i]), g(lb[i]), g(ub[i])), "\n", sep = "")
  olab <- if (!is.null(over)) vapply(seq_along(ol), function(j) paste(vapply(seq_along(over), function(i) labs_of(over[i], ocells[j, i]), ""), collapse = " "), "")
  last <- ""
  for (i in seq_len(K)) {
    e <- est[[i]]
    if (is.null(over) && kind != "proportion") {
      row(stata_tab_abbrev(e$var, 12), i)
      next
    }
    head <- if (kind == "proportion") {
      if (is.null(over)) e$var else paste0(e$var, "@", ohead)
    } else paste0("c.", e$var, "@", ohead)
    key <- if (kind == "proportion" && !is.null(over)) e$var else head
    if (key != last) {
      if (last != "") cat("             |\n")
      cat(sprintf("%12s |", stata_tab_abbrev(head, 12)), "\n", sep = "")
      last <- key
    }
    lab <- if (kind == "proportion") {
      vl <- labs_of(e$var, e$val)
      if (is.null(over)) vl else paste(vl, olab[e$j])
    } else olab[e$j]
    cat(sprintf("%11s  |%11s%11s%14s%12s", lab, g(b[i]), g(se[i]), g(lb[i]), g(ub[i])), "\n", sep = "")
  }
  cat(rule)
  if (!is.null(svy) && !is.null(design$note)) cat(paste(strwrap(design$note, width = 63, exdent = 6), collapse = "\n"), "\n", sep = "")
  out
}"##;

/// ratio's `y/x` pairs and their names.
type Ratios = (Vec<(String, String)>, Vec<String>);

impl Session {
    /// `mean varlist`, `total varlist`, `proportion varlist` and `ratio
    /// [name:] y/x ...`, `[weight] [if] [in] [, over(var) level(#)]`.
    pub(super) fn meanest(
        &mut self,
        c: &Command,
        kind: &str,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, kind)?;
        if let Some(o) = opts.other(&[("over", 4), ("level", 1)]) {
            return Err(no(format!(
                "{kind} with the option {o} is not translated yet"
            )));
        }
        let svy = self.svy_call.clone();
        let w = match &svy {
            Some(call) => call.wvar().map(|v| (W::P, v)),
            None => self.weight_of(c, &[W::F, W::A, W::P, W::I])?,
        };
        match (&w, kind) {
            // Stata 19: total, proportion and ratio take no analytic weights (an error as the
            // line runs, which capture takes)
            (Some((W::A, _)), "total" | "proportion" | "ratio") => {
                out.line("stop(\"aweights not allowed (Stata: r(101))\", call. = FALSE)");
                return Ok(());
            }
            // importance weights: as frequency weights (Stata 19), not checked after ratio
            (Some((W::I, _)), "ratio") => {
                return Err(no("ratio with iweights is not translated yet"));
            }
            _ => {}
        }
        let over: Option<Vec<String>> = match opts.arg("over", 4) {
            None => None,
            Some(o) => {
                let vs: Vec<String> = o.split_whitespace().map(str::to_string).collect();
                if vs.is_empty() || vs.iter().any(|v| !is_name(v)) {
                    return Err(no(format!(
                        "over({o}): variables written out are translated"
                    )));
                }
                Some(vs)
            }
        };
        let level_text = match (
            opts.arg("level", 1),
            svy.as_ref().and_then(|s| s.level.clone()),
        ) {
            (Some(_), Some(_)) => {
                return Err(no(
                    "level() both in the svy prefix and in the command is not translated",
                ))
            }
            (a, b) => a.or(b),
        };
        let level = match level_text {
            None => "95".to_string(),
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
                r_number(v)
            }
        };
        let text = c.parts.main.trim();
        let (vars, ratios): (Vec<String>, Option<Ratios>) = if kind == "ratio" {
            let mut pairs = Vec::new();
            let mut names = Vec::new();
            let items: Vec<String> = if text.starts_with('(') {
                text.split(')')
                    .map(|p| p.trim().trim_start_matches('(').trim().to_string())
                    .filter(|p| !p.is_empty())
                    .collect()
            } else {
                vec![text.to_string()]
            };
            for (i, it) in items.iter().enumerate() {
                let (name, rest) = match it.split_once(':') {
                    Some((n, r)) if is_name(n.trim()) => (n.trim().to_string(), r.trim()),
                    _ => (format!("_ratio_{}", i + 1), it.as_str()),
                };
                let Some((y, x)) = rest.split_once('/') else {
                    return Err(no(format!("ratio {it}: y/x is translated")));
                };
                let (y, x) = (y.trim(), x.trim());
                if !is_name(y) || !is_name(x) {
                    return Err(no(format!(
                        "ratio {it}: a variable over a variable is translated"
                    )));
                }
                pairs.push((y.to_string(), x.to_string()));
                names.push(name);
            }
            let mut vars: Vec<String> = Vec::new();
            for (y, x) in &pairs {
                for v in [y, x] {
                    if !vars.contains(v) {
                        vars.push(v.clone());
                    }
                }
            }
            (vars, Some((pairs, names)))
        } else {
            let vars: Vec<String> = text.split_whitespace().map(str::to_string).collect();
            if vars.is_empty() {
                return Err(no(format!("{kind} needs a variable")));
            }
            if let Some(v) = vars.iter().find(|v| !is_name(v)) {
                return Err(no(format!(
                    "{kind} {v}: variable names written out are translated"
                )));
            }
            (vars, None)
        };
        if let Some(call) = &svy {
            if over.is_some() && (!matches!(kind, "mean" | "total") || vars.len() != 1) {
                return Err(no(
                    "svy: over() is translated for mean and total of one variable",
                ));
            }
            if over.as_ref().is_some_and(|o| o.len() != 1) {
                return Err(no(
                    "svy: over() with more than one variable is not translated yet",
                ));
            }
            if over.is_some() && call.subpop.is_some() {
                return Err(no(
                    "svy: over() together with subpop() is not translated yet",
                ));
            }
        }
        let mut rows = self.rows_for(c, out)?;
        if w.is_some() {
            if svy.is_none() {
                rows = self.weighted_rows(rows, &w);
            }
            out.line(format!("stata_rows <- {rows}"));
            rows = "stata_rows".into();
            if svy.is_none() {
                out.caveat(super::weights::weight_caveat(&w));
            }
        }
        let mut svy_arg = String::new();
        if let Some(call) = &svy {
            let mut sub = String::new();
            match &call.subpop {
                Some(super::svy::SubPop::Var(v)) => {
                    sub = format!(", subvar = {}, subtext = {}", r_text(v), r_text(v));
                }
                Some(super::svy::SubPop::Cond(t)) => {
                    let e = t.trim().trim_start_matches("if").trim();
                    // a value: a row where it is missing is left out, as with a variable
                    let value = self.expr(e, Use::Value, out)?;
                    out.package("dplyr");
                    out.line(format!(
                        "stata_rows <- dplyr::mutate({rows}, stata_subpop = as.numeric({value}))"
                    ));
                    rows = "stata_rows".into();
                    sub = format!(", subvar = \"stata_subpop\", subtext = {}", r_text(t));
                }
                None => {}
            }
            let first = call.design.stages.first();
            let opt = |v: Option<&String>| v.map_or("NULL".to_string(), |s| r_text(s));
            svy_arg = format!(
                ", svy = list(strata = {}, psu = {}, fpc = {}, w = {}, singleunit = {}, line = {}{sub})",
                opt(first.and_then(|s| s.strata.as_ref())),
                opt(first.and_then(|s| s.su.as_ref())),
                opt(first.and_then(|s| s.fpc.as_ref())),
                opt(call.wvar().as_ref()),
                r_text(&call.design.singleunit),
                r_text(&call.line),
            );
            self.helpers.insert("stata_svy_vlin");
            out.caveat("printed in Stata 19's layout for survey estimates: the design on the left, the counts on the right");
        }
        let wargs = super::weights::w_args(&rows, &w);
        for h in [
            "stata_meanest",
            "stata_fmt",
            "stata_returned",
            "stata_tables",
            "stata_names",
        ] {
            self.helpers.insert(h);
        }
        let q = |v: &[String]| v.iter().map(|s| r_text(s)).collect::<Vec<_>>().join(", ");
        let ratio_args = match &ratios {
            None => String::new(),
            Some((pairs, names)) => format!(
                ", ratios = list({}), rnames = c({})",
                pairs
                    .iter()
                    .map(|(y, x)| format!("c({}, {})", r_text(y), r_text(x)))
                    .collect::<Vec<_>>()
                    .join(", "),
                q(names)
            ),
        };
        out.line(format!(
            "stata_me <- stata_meanest({}, {rows}, c({}){wargs}{}, level = {level}{ratio_args}{}, cmdline = {}{}{svy_arg})",
            r_text(kind),
            q(&vars),
            over.as_ref()
                .map(|o| format!(", over = c({})", q(o)))
                .unwrap_or_default(),
            if self.quiet(c) { ", show = FALSE" } else { "" },
            r_text(format!("{} {}", c.written, super::returned::after_name(c).trim()).trim()),
            match &w {
                Some((_, v)) => format!(", wexp = {}", r_text(v)),
                None => String::new(),
            }
        ));
        // e() is the estimates', under Stata's names; test and lincom read them
        out.line("stata_b <- stata_me$b");
        out.line("stata_vcov <- stata_me$V");
        out.line("stata_df <- stata_me$df");
        out.line("stata_estimates <- list()");
        out.line("stata_e <- stata_me$e");
        out.line("stata_r <- stata_me$r");
        out.line("rm(stata_me)");
        self.estimates = if svy.is_some() {
            None
        } else {
            Some(Estimates {
                fit: Fit::Margins,
                dep: String::new(),
                factors: Default::default(),
            })
        };
        self.e_new = true;
        for t in [
            "prefix",
            "cmdname",
            "command",
            "wvar",
            "singleunit",
            "su1",
            "strata1",
            "fpc1",
            "subpop",
            "cmd",
            "vce",
            "vcetype",
            "title",
            "namelist",
            "estat_cmd",
            "varlist",
            "marginsnotok",
            "over",
            "wtype",
            "wexp",
            "cmdline",
            "properties",
        ] {
            self.e_text.insert(t.to_string());
        }
        self.e_ret = super::returned::Returned::Kept {
            cmd: if svy.is_some() {
                format!("svy_{kind}")
            } else {
                kind.into()
            },
            // what test and lincom can read: plain names, not factor levels
            model: if over.is_some() || kind == "proportion" {
                "levels".into()
            } else {
                String::new()
            },
            names: &[],
        };
        self.r_new = Some((&[], kind.into()));
        Ok(())
    }

    /// The command whose results `Fit::Margins` holds when it is one of
    /// mean, total, proportion or ratio, and whether its names are factor
    /// levels (over(), proportion).
    pub(super) fn posted_estimates(&self) -> Option<(&str, bool)> {
        if !self
            .estimates
            .as_ref()
            .is_some_and(|e| e.fit == Fit::Margins)
        {
            return None;
        }
        match &self.e_ret {
            super::returned::Returned::Kept { cmd, model, .. }
                if matches!(cmd.as_str(), "mean" | "total" | "proportion" | "ratio") =>
            {
                Some((cmd.as_str(), !model.is_empty()))
            }
            _ => None,
        }
    }
}
