//! `permute` and `simulate` (batch I of the StatCraft port). The do-file runner
//! (`dofile::resample`) writes each of them out as a run of lines: a begin
//! line, the command (a program the do-file defines, or a built-in) once per
//! replication with a line before or after that draws the permutation or
//! collects the results, and an end line. The lines here are the translator's
//! side of those internal commands. The rules are in `plan/stata/resampling.md`.

use super::resample::{exp_items, split_prefixed};
use super::*;

/// What the runner decided before it wrote the lines out.
#[derive(Debug, Clone)]
pub struct Plan {
    /// How many replications.
    pub reps: usize,
    /// The command after the colon.
    pub inner: String,
}

/// Reads `permute` or `simulate` as far as the runner needs: the number of
/// replications and the command. `Err((code, message))` is an error Stata
/// gives before it runs anything.
pub fn plan(kind: &str, after_name: &str) -> Result<Plan, (u32, String)> {
    let c = crate::reader::read(&format!("{kind} {after_name}"));
    let Some(c) = c.commands().first().map(|c| (*c).clone()) else {
        return Err((198, "invalid syntax".into()));
    };
    let p = split_prefixed(&c, kind).map_err(|e| (198, e.reason))?;
    // options without a comma are taken for expressions: unknown function
    for item in exp_items(&p.explist) {
        if let Some((name, _)) = item.split_once('(') {
            if !name.is_empty()
                && !name.contains(['=', '[', ' '])
                && !matches!(name, "r" | "e" | "_b" | "_se" | "s")
                && name
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
                && matches!(
                    name,
                    "reps" | "seed" | "strata" | "saving" | "level" | "nodots" | "dots"
                )
            {
                return Err((133, format!("unknown function {name}()")));
            }
        }
    }
    let opts = Opts(&p.options);
    let reps = match opts.arg("reps", 1) {
        Some(r) => match r.trim().parse::<i64>() {
            Ok(n) if n >= 1 => n as usize,
            _ => {
                return Err((
                    125,
                    "reps() invalid -- invalid number, outside of allowed range".into(),
                ))
            }
        },
        None if kind == "simulate" => {
            return Err((198, "option reps() required".into()));
        }
        None => 100,
    };
    Ok(Plan {
        reps,
        inner: p.inner,
    })
}

/// R's side: the permutations, the replications, the table and r().
pub(super) const PERMUTE_HELPER: &str = r##"stata_pm_stream <- function(n) {
  s <- new.env()
  s$p <- stata_rng_pool(max(n, 16))
  s$pos <- 0L
  s
}
stata_pm_u <- function(s) {
  if (s$pos >= length(s$p$u)) stata_rng_more(s$p, max(length(s$p$u), 16))
  s$pos <- s$pos + 1L
  s$p$u[s$pos]
}
# a random integer 0..k-1: the smallest power of two m >= k, floor(u m), again while it is k or more
stata_pm_int <- function(s, k) {
  m <- 1
  while (m < k) m <- m * 2
  repeat {
    j <- floor(stata_pm_u(s) * m)
    if (j < k) return(j)
  }
}
stata_pm_wrap <- function(text, indent = 9) {
  ls <- if (exists("stata_linesize")) stata_linesize else 80
  width <- ls - 2
  words <- strsplit(text, " ", fixed = TRUE)[[1]]
  lines <- character()
  cur <- ""
  for (w in words) {
    if (nchar(cur) == 0) cur <- w
    else if (nchar(cur) + 1 + nchar(w) <= width) cur <- paste(cur, w)
    else { lines <- c(lines, cur); cur <- paste0(strrep(" ", indent), w) }
  }
  paste(c(lines, cur), collapse = "\n")
}
stata_pm_begin <- function(d, permvar, strata, reps, nodots, saving = NULL, replace = FALSE, level = 95) {
  st <- new.env()
  st$err <- NULL
  st$permvar <- permvar
  st$strata <- strata
  st$reps <- reps
  st$nodots <- nodots
  st$saving <- saving
  st$level <- level
  st$d0 <- d
  st$vals <- NULL
  st$done <- 0L
  for (v in c(permvar, strata)) {
    if (!v %in% names(d)) { st$err <- paste0("variable ", v, " not found (Stata: r(111))"); break }
  }
  if (is.null(st$err) && !is.null(saving) && !replace && file.exists(saving)) {
    st$err <- paste0("file ", saving, " already exists (Stata: r(602))")
  }
  stata_pm_st <<- st
  invisible(NULL)
}
stata_pm_first <- function(d, vals, exps, eclass, cmdname, esample = NULL) {
  st <- stata_pm_st
  if (!is.null(st$err)) return(invisible(NULL))
  miss <- which(is.na(vals))
  if (length(miss)) {
    st$err <- paste0(exps[miss[1]], " evaluated to missing in unpermuted data (Stata: r(322))")
    return(invisible(NULL))
  }
  st$obs <- vals
  names(st$obs) <- names(vals)
  st$cmdname <- cmdname
  st$rows <- if (eclass && !is.null(esample)) which(esample) else seq_len(nrow(d))
  x <- d[[st$permvar]]
  attributes(x) <- NULL
  st$x0 <- x
  st$nstrata <- if (is.null(st$strata)) NULL else length(unique(d[[st$strata]][st$rows]))
  xs <- x[st$rows]
  u <- unique(xs)
  st$two <- length(u) == 2 && !anyNA(xs)
  if (st$two) st$small <- if (sum(xs == min(u)) <= sum(xs == max(u))) min(u) else max(u)
  if (!st$nodots) cat("(running ", cmdname, " on estimation sample)\n", sep = "")
  if (!eclass) {
    cat("\n", stata_pm_wrap(paste0("warning: ", cmdname, " does not set e(sample), so no observations will be excluded from the permutations because of missing values or other reasons. To exclude observations, press Break, save the data, drop any observations that are to be excluded, and rerun permute.")), "\n", sep = "")
  }
  if (!is.null(st$saving) && !file.exists(st$saving)) cat("(file ", st$saving, " not found)\n", sep = "")
  if (!st$nodots) cat("\nPermutations (", st$reps, "): ", sep = "")
  invisible(NULL)
}
stata_pm_shuffle <- function(d) {
  st <- stata_pm_st
  if (!is.null(st$err)) return(d)
  x <- d[[st$permvar]]
  rows <- st$rows
  if (length(rows) > nrow(d)) rows <- rows[rows <= nrow(d)]
  keys <- if (is.null(st$strata)) NULL else d[[st$strata]]
  groups <- if (is.null(keys)) list(rows) else {
    k <- keys[rows]
    attributes(k) <- NULL
    ord <- order(k, na.last = TRUE, method = "radix")
    unname(split(rows[ord][order(match(k[ord], unique(k[ord])), seq_along(ord))], match(k[ord], unique(k[ord]))))
  }
  s <- stata_pm_stream(length(rows) * 3)
  for (P in groups) {
    n <- length(P)
    if (n < 2) next
    if (st$two) {
      x0 <- st$x0[P]
      sm <- which(x0 == st$small)
      ord <- c(sm, setdiff(seq_len(n), sm))
      m <- length(sm)
      arr <- x0[ord]
      for (a in seq_len(m) - 1L) {
        b <- a + stata_pm_int(s, n - a)
        tmp <- arr[a + 1L]; arr[a + 1L] <- arr[b + 1L]; arr[b + 1L] <- tmp
      }
      xx <- x[P]
      xx[ord] <- arr
      x[P] <- xx
    } else {
      v <- x[P]
      for (a in seq_len(n - 1L) - 1L) {
        b <- a + stata_pm_int(s, n - a)
        tmp <- v[a + 1L]; v[a + 1L] <- v[b + 1L]; v[b + 1L] <- tmp
      }
      x[P] <- v
    }
  }
  stata_rng_done(s$p, s$pos)
  d[[st$permvar]] <- x
  d
}
stata_pm_row <- function(vals) {
  st <- stata_pm_st
  if (!is.null(st$err)) return(invisible(NULL))
  st$done <- st$done + 1L
  # a replication that stopped with an error: its results are missing
  failed <- is.null(vals)
  if (failed) {
    vals <- rep(NA_real_, length(st$obs))
    st$failed <- TRUE
  }
  st$vals <- rbind(st$vals, vals)
  if (!st$nodots) {
    cat(if (failed) "x" else if (st$done %% 10L == 0L) st$done else ".")
    if (st$done == st$reps) {
      cat(" done\n")
      if (isTRUE(st$failed)) cat("x: Error occurred when permute executed ", st$cmdname, ".\n", sep = "")
    }
  }
  invisible(NULL)
}
stata_pm_end <- function(permvar, command, byvec, eq, named_exps, saving_double = FALSE) {
  st <- stata_pm_st
  if (!is.null(st$err)) {
    msg <- st$err
    rm(stata_pm_st, envir = globalenv())
    stop(msg, call. = FALSE)
  }
  reps <- if (saving_double) st$vals else matrix(stata_float(st$vals), nrow(st$vals), ncol(st$vals))
  obs <- if (saving_double) st$obs else stats::setNames(stata_float(st$obs), names(st$obs))
  k <- length(obs)
  nm0 <- names(obs)
  info <- lapply(nm0, function(n) get0(n, envir = stata_sb_reg, inherits = FALSE))
  bare <- function(i, n) if (is.null(info[[i]])) n else info[[i]]$coef
  nm <- if (byvec) vapply(seq_along(nm0), function(i) {
    x <- info[[i]]
    if (is.null(x)) return(nm0[i])
    if (eq) paste0(x$kind, ":", x$coef) else if (!is.null(x$eq)) paste0(x$eq, ":", x$coef) else x$coef
  }, "") else nm0
  eq <- eq || (byvec && any(grepl(":", nm, fixed = TRUE)))
  labels <- vapply(seq_along(nm0), function(i) if (is.null(info[[i]])) nm0[i] else info[[i]]$label, "")
  expstexts <- if (byvec) vapply(seq_along(nm0), function(i) if (is.null(info[[i]])) nm0[i] else paste0("_", info[[i]]$kind, "[", info[[i]]$coef, "]"), "") else named_exps$exp
  if (!byvec) labels <- named_exps$exp
  n <- vapply(seq_len(k), function(j) sum(!is.na(reps[, j])), 0)
  cl <- vapply(seq_len(k), function(j) sum(reps[, j] <= obs[j], na.rm = TRUE), 0)
  cu <- vapply(seq_len(k), function(j) sum(reps[, j] >= obs[j], na.rm = TRUE), 0)
  pl <- cl / n
  pu <- cu / n
  p2 <- pmin(1, 2 * pmin(pl, pu))
  sef <- function(p) sqrt(p * (1 - p) / n)
  level <- st$level
  a <- (1 - level / 100) / 2
  z <- stats::qnorm(1 - a)
  cp <- function(cc) {
    lo <- ifelse(cc == 0, 0, stats::qbeta(a, cc, n - cc + 1))
    hi <- ifelse(cc == n, 1, stats::qbeta(1 - a, cc + 1, n - cc))
    rbind(lo, hi)
  }
  ci2 <- function(p, se) {
    lo <- ifelse(se == 0, NA_real_, pmax(p - z * se, 0))
    hi <- ifelse(se == 0, NA_real_, pmin(p + z * se, 1))
    rbind(lo, hi)
  }
  Nobs <- length(st$rows)
  strata_n <- st$nstrata
  # the results table
  pad <- function(s, w) formatC(as.character(s), width = w)
  p4 <- function(v) ifelse(is.na(v), ".", sub("^0\\.", ".", sprintf("%.4f", v)))
  nobs_txt <- as.character(Nobs)
  vals <- c(nobs_txt, if (!is.null(st$strata)) as.character(strata_n), as.character(st$reps))
  w <- max(nchar(vals))
  left <- c("Monte Carlo permutation results", paste0("Permutation variable: ", permvar))
  right <- c(paste0("Number of observations = ", pad(nobs_txt, w)))
  if (!is.null(st$strata)) {
    right <- c(right, paste0("Number of strata       = ", pad(as.character(strata_n), w)))
  }
  right <- c(right, paste0("Number of permutations = ", pad(as.character(st$reps), w)))
  left <- c(left, rep("", length(right) - length(left)))
  col <- 79 - nchar(right[1])
  cat("\n")
  for (i in seq_along(right)) cat(formatC(left[i], width = -col), right[i], "\n", sep = "")
  cat("\n", pad("Command", 13), ": ", command, "\n", sep = "")
  for (j in seq_along(named_exps$name)) cat(pad(named_exps$name[j], 13), ": ", named_exps$exp[j], "\n", sep = "")
  cat("\n", strrep("-", 79), "\n", sep = "")
  cat("             |                                               Monte Carlo error\n")
  cat("             |                                              -------------------\n")
  cat("           T |    T(obs)       Test       c       n      p  SE(p)   [", stata_fmt(level, "%9.0g", trim = TRUE), "% CI(p)]\n", sep = "")
  cat("-------------+", strrep("-", 65), "\n", sep = "")
  lo_ci <- cp(cl); hi_ci <- cp(cu)
  se_l <- sef(pl); se_u <- sef(pu); se_2 <- sef(p2)
  c2 <- ci2(p2, se_2)
  short <- function(s) ifelse(nchar(s) > 12, paste0(substr(s, 1, 10), "~", substr(s, nchar(s), nchar(s))), s)
  g9 <- function(v) stata_fmt(v, "%9.0g")
  one <- function(j, label) {
    cat(pad(short(label), 12), " |", pad(g9(obs[j]), 10), pad("lower", 11), pad(cl[j], 8), pad(n[j], 8),
        pad(p4(pl[j]), 7), pad(p4(se_l[j]), 7), pad(p4(lo_ci[1, j]), 7), pad(p4(lo_ci[2, j]), 7), "\n", sep = "")
    cat("             |", pad("upper", 21), pad(cu[j], 8), pad(n[j], 8),
        pad(p4(pu[j]), 7), pad(p4(se_u[j]), 7), pad(p4(hi_ci[1, j]), 7), pad(p4(hi_ci[2, j]), 7), "\n", sep = "")
    cat("             |", pad("two-sided", 21), strrep(" ", 16),
        pad(p4(p2[j]), 7), pad(p4(se_2[j]), 7), pad(p4(c2[1, j]), 7), pad(p4(c2[2, j]), 7), "\n", sep = "")
  }
  eqs <- if (eq) unique(sub(":.*$", "", nm)) else NULL
  if (eq) {
    for (e in eqs) {
      cat(formatC(e, width = -13), "|\n", sep = "")
      idx <- which(startsWith(nm, paste0(e, ":")))
      for (i in seq_along(idx)) {
        one(idx[i], sub("^[^:]*:", "", nm[idx[i]]))
        if (i < length(idx)) cat("             |\n")
      }
      if (e != eqs[length(eqs)]) cat("-------------+", strrep("-", 65), "\n", sep = "")
    }
  } else {
    for (j in seq_len(k)) {
      one(j, nm[j])
      if (j < k) cat("             |\n")
    }
  }
  cat(strrep("-", 79), "\n", sep = "")
  cat("Notes: For lower one-sided test, c = #{T <= T(obs)} and p = p_lower = c/n.\n",
      "       For upper one-sided test, c = #{T >= T(obs)} and p = p_upper = c/n.\n",
      "       For two-sided test, p = 2*min(p_lower, p_upper); SE and CI approximate.\n", sep = "")
  # r()
  mat <- function(v, r = "y1") matrix(v, 1, k, dimnames = list(r, nm))
  mat2 <- function(m) { dimnames(m) <- list(c("ll", "ul"), nm); m }
  r <- list(N = Nobs, n_reps = st$reps)
  if (!is.null(st$strata)) r$n_strata <- strata_n
  r$k_exp <- if (byvec) 0 else k
  r$k_eexp <- if (byvec) k else 0
  r$level <- level
  r$cmd <- "permute"
  r$command <- command
  r$permvar <- permvar
  if (!is.null(st$strata)) r$strata <- st$strata
  r$title <- "Monte Carlo permutation results"
  for (j in seq_along(expstexts)) r[[paste0("exp", j)]] <- expstexts[j]
  r$b <- mat(obs, "y1")
  r$n <- mat(n, "y1")
  r$c_lower <- mat(cl, "y1")
  r$c_upper <- mat(cu, "y1")
  r$p_lower <- mat(pl, "y1")
  r$p_upper <- mat(pu, "y1")
  r$p_twosided <- mat(p2, "y1")
  r$se_p_lower <- mat(se_l, "y1")
  r$se_p_upper <- mat(se_u, "y1")
  r$se_p_twosided <- mat(se_2, "y1")
  r$ci_p_lower <- mat2(lo_ci)
  r$ci_p_upper <- mat2(hi_ci)
  r$ci_p_twosided <- mat2(c2)
  if (!is.null(st$saving)) {
    out <- as.data.frame(reps, stringsAsFactors = FALSE)
    vn <- nm0
    colnames(out) <- vn
    for (j in seq_len(k)) attr(out[[j]], "label") <- labels[j]
    haven::write_dta(out, st$saving, label = paste0("permute ", permvar, " : ", st$cmdname))
  }
  rm(stata_pm_st, envir = globalenv())
  r
}"##;

/// R's `simulate`: the replications and the dataset of results.
pub(super) const SIMULATE_HELPER: &str = r##"stata_sm_begin <- function(reps, nodots, names, exps, command, saving = NULL, replace = FALSE) {
  st <- new.env()
  st$reps <- reps
  st$nodots <- nodots
  st$res <- vector("list", reps)
  st$cmdname <- strsplit(trimws(command), "[ ,]")[[1]][1]
  st$i <- 0L
  st$err <- NULL
  if (!is.null(saving) && !replace && file.exists(saving)) st$err <- paste0("file ", saving, " already exists (Stata: r(602))")
  stata_sm_st <<- st
  pad <- function(s) formatC(s, width = 13)
  cat("\n", pad("Command"), ": ", command, "\n", sep = "")
  for (j in seq_along(names)) cat(pad(names[j]), ": ", exps[j], "\n", sep = "")
  cat("\n")
  if (!nodots) cat("Simulations (", reps, "): ", sep = "")
  invisible(NULL)
}
stata_sm_row <- function(v) {
  st <- stata_sm_st
  st$i <- st$i + 1L
  # a replication that stopped with an error leaves a row of missing values
  if (!is.null(v)) st$res[st$i] <- list(v) else st$failed <- TRUE
  if (!st$nodots) {
    cat(if (is.null(v)) "x" else if (st$i %% 10L == 0L) st$i else ".")
    if (st$i %% 50L == 0L && st$i < st$reps) cat("\n")
    if (st$i == st$reps) {
      cat(" done\n")
      if (isTRUE(st$failed)) cat("x: Error occurred when simulate executed ", st$cmdname, ".\n", sep = "")
    }
  }
  invisible(NULL)
}
stata_sm_end <- function(exps, saving = NULL, cmdname = "") {
  st <- stata_sm_st
  tb <- stata_sb_table(st$res, exps)
  out <- list()
  for (j in seq_len(ncol(tb$m))) {
    nm <- colnames(tb$m)[j]
    out[[nm]] <- structure(stata_float(tb$m[, j]), label = unname(tb$lab[j]), format.stata = "%9.0g")
  }
  out <- as.data.frame(out, stringsAsFactors = FALSE, optional = TRUE)
  if (!length(tb$lab)) out <- data.frame(row.names = seq_len(st$reps))
  rownames(out) <- NULL
  if (!is.null(saving)) haven::write_dta(out, saving, label = paste0("simulate: ", cmdname))
  rm(stata_sm_st, envir = globalenv())
  out
}"##;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_plan_reads_the_replications_and_the_command() {
        let p = plan("permute", "x r(mean), reps(12) seed(3): summarize y")
            .unwrap_or_else(|e| panic!("{e:?}"));
        assert_eq!((p.reps, p.inner.as_str()), (12, "summarize y"));
        let p = plan("permute", "x r(mean): summarize y").unwrap_or_else(|e| panic!("{e:?}"));
        assert_eq!(p.reps, 100);
        let p =
            plan("simulate", "m=r(m), reps(7) nodots: prog").unwrap_or_else(|e| panic!("{e:?}"));
        assert_eq!((p.reps, p.inner.as_str()), (7, "prog"));
    }

    #[test]
    fn the_plan_gives_the_errors_stata_gives_before_it_runs() {
        let code = |k: &str, t: &str| plan(k, t).err().map(|e| e.0);
        assert_eq!(
            code("permute", "y r(mean), reps(0): summarize y"),
            Some(125)
        );
        assert_eq!(code("permute", "y r(mean) reps(5): summarize y"), Some(133));
        assert_eq!(code("permute", "y r(mean), reps(5)"), Some(198));
        assert_eq!(code("simulate", "m=r(m): prog"), Some(198));
    }
}
