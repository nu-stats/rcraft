//! Batch E of the StatCraft port, the least-squares family as Stata 19 does
//! it (`plan/stata/constraint.md`): `constraint` and `cnsreg` here, and
//! beside them `areg`, `qreg`, `sureg`, `mvreg`, `nlsur`, `xi` and
//! `logistic`.
//!
//! `constraint` keeps the text of each constraint in R (`stata_cns`, a list
//! named by number), so a do-file that defines them in a branch or a loop
//! gets what Stata gets. `cnsreg` reads them when it runs: each is read as a
//! linear equation in the model's coefficients, equal copies are one, and
//! the fit is restricted least squares by projection (b = b0 + T a, with b0
//! the smallest b meeting R b = r and T an orthonormal basis of what the
//! constraints leave free).

use super::model::{level, Estimates, Fit, Spec};
use super::weights::W;
use super::*;

/// The R of the family: the shared header and table, the constraints and
/// `cnsreg`.
pub(super) const LSFAM_HELPER: &str = r##"# the right-hand block of a header (Number of obs, F, Root MSE ...), its values right-aligned,
# the whole block ending at column 78 as Stata's does; `left` is the title on the first line
stata_ls_header <- function(left, items, extra = character(), width = 78L) {
  vals <- vapply(items, function(i) i[[2]], "")
  w <- max(6L, nchar(vals))
  cat("\n")
  for (k in seq_along(items)) {
    blk <- sprintf("%-13s = %s", items[[k]][[1]], formatC(vals[k], width = w))
    l <- if (k == 1) left else ""
    cat(formatC(l, width = -(width - nchar(blk))), blk, "\n", sep = "")
  }
  cat("\n")
}
# Stata's coefficient table for plain names; `constr` marks the coefficients the constraints fix
stata_ls_table <- function(nm, b, se, df, level, dep, constr = rep(FALSE, length(nm)), robust = FALSE,
                           clusters = NULL, clustvar = NULL, omitted = rep(FALSE, length(nm))) {
  pad <- stata_tab_pad
  if (!is.null(clusters)) {
    cat(pad(sprintf("(Std. err. adjusted for %s clusters in %s)", stata_tab_count(clusters), clustvar), 78), "\n", sep = "")
  }
  q <- if (is.finite(df)) stats::qt((1 + level / 100) / 2, df) else stats::qnorm((1 + level / 100) / 2)
  cat(strrep("-", 78), "\n", sep = "")
  if (robust) cat("             |               Robust\n")
  cat(pad(stata_tab_abbrev(dep), 12), " | Coefficient  ", if (robust) "std. err." else "Std. err.",
      if (is.finite(df)) "      t    P>|t|" else "      z    P>|z|", "     [", format(level), "% conf. interval]\n", sep = "")
  cat(strrep("-", 13), "+", strrep("-", 64), "\n", sep = "")
  for (i in seq_along(nm)) {
    label <- pad(stata_tab_abbrev(nm[i]), 12)
    if (omitted[i]) {
      cat(label, " |", pad(stata_tab_g(0), 11), "  (omitted)\n", sep = "")
    } else if (constr[i]) {
      cat(label, " |", pad(stata_tab_g(b[i]), 11), "  (constrained)\n", sep = "")
    } else {
      z <- b[i] / se[i]
      p <- if (is.finite(df)) 2 * stats::pt(-abs(z), df) else 2 * stats::pnorm(-abs(z))
      cat(label, " |", pad(stata_tab_g(b[i]), 11), pad(stata_tab_g(se[i]), 11), pad(sprintf("%.2f", z), 9),
          pad(sprintf("%.3f", p), 8), pad(stata_tab_g(b[i] - q * se[i]), 13), pad(stata_tab_g(b[i] + q * se[i]), 12), "\n", sep = "")
    }
  }
  cat(strrep("-", 78), "\n", sep = "")
  invisible(NULL)
}
# the linear prediction of a plain-names model from e(b), on every row of d
stata_ls_xb <- function(e, d, which = NULL) {
  b <- e$b
  if (!is.null(which)) b <- b[, grepl(paste0("^", which, ":"), colnames(b)), drop = FALSE]
  nm <- sub("^[^:]*:", "", colnames(b))
  xb <- rep(0, nrow(d))
  for (j in seq_along(nm)) {
    if (nm[j] == "_cons") {
      xb <- xb + b[1, j]
    } else {
      v <- unclass(d[[nm[j]]])
      attributes(v) <- NULL
      xb <- xb + b[1, j] * as.numeric(v)
    }
  }
  xb
}
stata_numlist <- function(text, lo = 1L, hi = 1999L) {
  out <- integer()
  for (tok in strsplit(trimws(text), "[[:space:],]+")[[1]]) {
    if (!nzchar(tok)) next
    if (identical(tok, "_all")) {
      out <- c(out, -1L)
      next
    }
    m <- regmatches(tok, regexec("^(-?[0-9]+)(?:[/-]([0-9]+))?$", tok, perl = TRUE))[[1]]
    if (!length(m)) stop("invalid numlist (Stata: r(121))", call. = FALSE)
    a <- as.integer(m[2])
    b <- if (nzchar(m[3])) as.integer(m[3]) else a
    if (b < a) stop("invalid numlist (Stata: r(121))", call. = FALSE)
    if (a < lo || b > hi) stop("invalid numlist has elements outside of allowed range (Stata: r(125))", call. = FALSE)
    out <- c(out, a:b)
  }
  out
}
# constraint define | dir | list | get | free | drop; the list is returned (with r() in `attr(, "r")`)
stata_constraint <- function(cns, what, arg = "", text = "") {
  keys <- function() sort(as.integer(names(cns)))
  show <- function(ids) for (i in ids) if (!is.null(cns[[as.character(i)]])) cat(sprintf("%6d:  %s\n", i, cns[[as.character(i)]]))
  if (what == "define") {
    n <- suppressWarnings(as.integer(arg))
    if (is.na(n) || n < 1L || n > 1999L) stop("constraint number must be between 1 and 1999 (Stata: r(198))", call. = FALSE)
    cns[[as.character(n)]] <- trimws(text)
  } else if (what %in% c("list", "dir")) {
    ids <- if (nzchar(trimws(arg))) stata_numlist(arg) else -1L
    show(if (identical(ids, -1L) || -1L %in% ids) keys() else ids)
  } else if (what == "drop") {
    ids <- stata_numlist(arg)
    if (-1L %in% ids) cns <- list() else for (i in ids) cns[[as.character(i)]] <- NULL
  } else if (what == "get") {
    n <- suppressWarnings(as.integer(arg))
    if (is.na(n) || n < 1L || n > 1999L) stop("constraint number must be between 1 and 1999 (Stata: r(198))", call. = FALSE)
    t <- cns[[as.character(n)]]
    attr(cns, "r") <- if (is.null(t)) list(defined = 0) else list(defined = 1, contents = t)
  } else if (what == "free") {
    free <- setdiff(1999:1, as.integer(names(cns)))
    if (!length(free)) stop("no free constraint numbers (Stata: r(900))", call. = FALSE)
    attr(cns, "r") <- list(free = as.character(free[1]))
  }
  cns
}
# one constraint as a linear equation in the coefficients `nm`: a vector of coefficients and the
# right-hand side, or the return code Stata reports for it
stata_cns_parse <- function(text, nm) {
  rc <- function(n) structure(n, class = "stata_cns_rc")
  bad <- function(n) stop(structure(class = c("stata_cns_bad", "error", "condition"), list(message = "", call = NULL, rc = n)))
  txt <- trimws(text)
  if (grepl("[", txt, fixed = TRUE)) {
    # _b[name] is the coefficient; an equation name ([y]x) is Stata's r(303) here
    txt <- gsub("_b[[]([A-Za-z_][A-Za-z0-9_]*)[]]", "\\1", txt)
    if (grepl("[", txt, fixed = TRUE)) return(rc(303L))
  }
  eq <- gregexpr("=", txt, fixed = TRUE)[[1]]
  if (eq[1] < 0) return(rc(111L))
  if (length(eq) > 1) stop("RCraft: a constraint with more than one = (or ==) is not translated yet", call. = FALSE)
  sides <- strsplit(txt, "=", fixed = TRUE)[[1]]
  if (length(sides) != 2L || !nzchar(trimws(sides[1])) || !nzchar(trimws(sides[2]))) return(rc(111L))
  k <- length(nm)
  lin <- function(ex) {
    const <- function(v) c(rep(0, k), v)
    if (is.numeric(ex)) return(const(ex))
    if (is.name(ex)) {
      j <- match(as.character(ex), nm)
      if (is.na(j)) bad(111L)
      v <- const(0)
      v[j] <- 1
      return(v)
    }
    if (!is.call(ex)) stop("RCraft: a constraint of this form is not translated yet", call. = FALSE)
    op <- as.character(ex[[1]])
    if (op == "(") return(lin(ex[[2]]))
    if (length(ex) == 2L && op %in% c("+", "-")) return(if (op == "-") -lin(ex[[2]]) else lin(ex[[2]]))
    if (length(ex) != 3L || !op %in% c("+", "-", "*", "/")) bad(131L)
    a <- lin(ex[[2]])
    b <- lin(ex[[3]])
    ca <- all(a[seq_len(k)] == 0)
    cb <- all(b[seq_len(k)] == 0)
    switch(op, "+" = a + b, "-" = a - b,
           "*" = if (ca) a[k + 1] * b else if (cb) b[k + 1] * a else bad(131L),
           "/" = if (cb) a / b[k + 1] else bad(131L))
  }
  quote_names <- function(s) gsub("(?<![0-9.A-Za-z_])([A-Za-z_][A-Za-z0-9_]*)", "`\\1`", s, perl = TRUE)
  ev <- function(s) {
    ex <- tryCatch(parse(text = quote_names(s), keep.source = FALSE), error = function(e) NULL)
    if (is.null(ex) || length(ex) != 1L) stop("RCraft: a constraint of this form is not translated yet", call. = FALSE)
    tryCatch(lin(ex[[1]]), stata_cns_bad = function(e) rc(e$rc))
  }
  l <- ev(sides[1])
  if (inherits(l, "stata_cns_rc")) return(l)
  r <- ev(sides[2])
  if (inherits(r, "stata_cns_rc")) return(r)
  d <- l - r
  list(a = d[seq_len(k)], rhs = -d[k + 1])
}
stata_cns_line <- function(a, rhs, nm) {
  f <- function(v) stata_fmt(v, "%10.0g", trim = TRUE)
  s <- ""
  for (j in which(a != 0)) {
    c1 <- a[j]
    mag <- if (abs(c1) == 1) nm[j] else paste0(f(abs(c1)), "*", nm[j])
    s <- if (!nzchar(s)) paste0(if (c1 < 0) "- " else "", mag) else paste0(s, if (c1 < 0) " - " else " + ", mag)
  }
  paste0(s, " = ", f(if (rhs == 0) 0 else rhs))
}
stata_cnsreg <- function(d, dep, xs, cns, noconst = FALSE, wvar = NULL, wtype = NULL, robust = FALSE,
                         cluster = NULL, level = 95, cmdline = "", show = TRUE, cns_list = NULL) {
  for (v in c(dep, xs, wvar, cluster)) if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
  num <- function(u) { u <- unclass(u); attributes(u) <- NULL; as.numeric(u) }
  for (v in c(dep, xs, wvar)) if (is.character(d[[v]])) stop(v, ": string variable, type mismatch (Stata: r(109))", call. = FALSE)
  ok <- stats::complete.cases(d[unique(c(dep, xs, wvar, cluster))])
  if (!any(ok)) stop("no observations (Stata: r(2000))", call. = FALSE)
  rows <- which(ok)
  y <- num(d[[dep]])[rows]
  X <- vapply(xs, function(v) num(d[[v]])[rows], numeric(length(rows)))
  X <- matrix(X, length(rows), length(xs), dimnames = list(NULL, xs))
  if (!noconst) X <- cbind(X, `_cons` = 1)
  nm <- colnames(X)
  k <- length(nm)
  n <- length(y)
  w <- if (is.null(wvar)) rep(1, n) else num(d[[wvar]])[rows]
  cl <- if (is.null(cluster)) NULL else d[[cluster]][rows]
  # the constraints that are defined, in the order given
  cns_list <- get0("stata_cns", ifnotfound = list())
  rowsR <- list()
  rhs <- numeric()
  for (i in cns) {
    t <- cns_list[[as.character(i)]]
    if (is.null(t)) next
    p <- stata_cns_parse(t, nm)
    if (inherits(p, "stata_cns_rc")) {
      cat("(note: constraint number ", i, " caused error r(", unclass(p), "))\n", sep = "")
      next
    }
    rowsR[[length(rowsR) + 1]] <- p$a
    rhs <- c(rhs, p$rhs)
  }
  if (!length(rowsR)) stop("matrix e(Cns) not found (Stata: r(111))", call. = FALSE)
  R <- do.call(rbind, rowsR)
  keep <- !duplicated(cbind(R, rhs))
  R <- R[keep, , drop = FALSE]
  rhs <- rhs[keep]
  m <- nrow(R)
  sv <- svd(R, nu = 0, nv = k)
  if (m > k || sum(sv$d > 1e-10 * max(sv$d, 1)) < m || any(rowSums(abs(R)) == 0)) {
    stop("redundant or inconsistent constraints (Stata: r(412))", call. = FALSE)
  }
  if (!is.null(wtype) && wtype == "f") {
    Nn <- sum(w)
    wn <- w
  } else if (identical(wtype, "p")) {
    # sampling weights are not scaled to the rows in the sums of squares (Stata 19)
    Nn <- n
    wn <- w
  } else if (!is.null(wtype)) {
    Nn <- n
    wn <- w * n / sum(w)
  } else {
    Nn <- n
    wn <- w
  }
  if (!is.null(wtype) && wtype == "a" && show) cat("(sum of wgt is ", stata_fmt(sum(w), "%9.0gc", trim = TRUE), ")\n", sep = "")
  if (!is.null(wtype) && wtype == "p") robust <- TRUE
  if (k - m < 1) stop("RCraft: cnsreg with every coefficient fixed by the constraints is not translated yet", call. = FALSE)
  b0 <- drop(t(R) %*% solve(R %*% t(R), rhs))
  Tm <- sv$v[, seq(m + 1, k), drop = FALSE]
  if (m == k) Tm <- matrix(0, k, 0)
  Z <- X %*% Tm
  yy <- y - drop(X %*% b0)
  A <- crossprod(Z * wn, Z)
  Ainv <- solve(A)
  a <- drop(Ainv %*% crossprod(Z * wn, yy))
  b <- b0 + drop(Tm %*% a)
  res <- y - drop(X %*% b)
  rss <- sum(wn * res^2)
  dfr <- Nn - (k - m)
  s2 <- rss / dfr
  Vm <- s2 * (Tm %*% Ainv %*% t(Tm))
  V <- Vm
  df <- dfr
  if (robust || !is.null(cl)) {
    if (is.null(cl)) {
      sc <- Z * (if (identical(wtype, "f")) sqrt(w) * res else wn * res)
      V <- Tm %*% (Ainv %*% crossprod(sc) %*% Ainv) %*% t(Tm) * Nn / dfr
    } else {
      sg <- rowsum(Z * (wn * res), cl)
      G <- nrow(sg)
      V <- Tm %*% (Ainv %*% crossprod(sg) %*% Ainv) %*% t(Tm) * G / (G - 1) * (Nn - 1) / dfr
      df <- G - 1
    }
  }
  dimnames(V) <- dimnames(Vm) <- list(nm, nm)
  constr <- rowSums(abs(Tm)) < 1e-12
  se <- sqrt(diag(V))
  # F: the slopes all zero, given the constraints, when that does not contradict them
  S <- which(nm != "_cons")
  ST <- Tm[S, , drop = FALSE]
  Fv <- NA_real_
  dfm <- NA_real_
  pF <- NA_real_
  if (length(S)) {
    rk <- if (ncol(ST)) sum(svd(ST)$d > 1e-9) else 0L
    fit0 <- if (ncol(ST)) { z0 <- qr.coef(qr(ST, tol = 1e-10), -b0[S]); z0[is.na(z0)] <- 0; z0 } else numeric()
    feasible <- rk > 0 && all(abs(drop(ST %*% fit0) + b0[S]) <= 1e-8 * (1 + abs(b0[S])))
    if (feasible) {
      ev <- eigen(V[S, S, drop = FALSE], symmetric = TRUE)
      pos <- ev$values > 1e-10 * max(ev$values)
      u <- drop(t(ev$vectors[, pos, drop = FALSE]) %*% b[S])
      Fv <- sum(u^2 / ev$values[pos]) / rk
      dfm <- rk
      # e(p) uses the residual degrees of freedom even where the clusters set the header's
      pF <- stats::pf(Fv, dfm, dfr, lower.tail = FALSE)
    }
  }
  Cns <- cbind(R, `_r` = rhs)
  dimnames(Cns) <- list(paste0("r", seq_len(m)), colnames(Cns))
  ll <- -0.5 * Nn * (log(2 * pi) + log(rss / Nn) + 1) + if (identical(wtype, "a")) 0.5 * sum(log(wn)) else 0
  e <- list()
  put <- function(...) { a <- list(...); for (nme in names(a)) e[[nme]] <<- a[[nme]] }
  put(df_m = dfm, F = Fv, p = pF)
  if (!is.null(cl)) put(N_clust = G <- length(unique(cl)))
  put(N = Nn, df_r = df, rmse = sqrt(rss / dfr), ll = ll, rank = k - m)
  if (!is.null(cl)) {
    # N_clust comes before df_m in Stata's list
    e <- e[c("N", "rmse", "ll", "N_clust", "df_m", "F", "p", "df_r", "rank")]
  } else {
    e <- e[c("df_m", "F", "p", "N", "df_r", "rmse", "ll", "rank")]
  }
  put(cmdline = cmdline)
  if (robust || !is.null(cl)) put(vcetype = "Robust")
  put(vce = if (!is.null(cl)) "cluster" else if (robust) "robust" else "ols")
  if (!is.null(cl)) put(clustvar = cluster)
  put(cmd = "cnsreg", title = "Constrained linear regression", predict = "tobit_p", depvar = dep,
      marginsok = "default XB", properties = "b V")
  bm <- matrix(b, 1, k, dimnames = list("y1", nm))
  put(b = bm, V = V, Cns = Cns)
  if (robust || !is.null(cl)) put(V_modelbased = Vm)
  attr(e, "stata_rows") <- rows
  attr(e, "stata_b") <- stats::setNames(b, sub("^_cons$", "(Intercept)", nm))
  attr(e, "stata_V") <- { Vn <- V; dimnames(Vn) <- list(sub("^_cons$", "(Intercept)", nm), sub("^_cons$", "(Intercept)", nm)); Vn }
  attr(e, "stata_constr") <- constr
  if (show) stata_cnsreg_show(e, level)
  e
}
stata_cnsreg_show <- function(e, level = 95) {
  nm <- colnames(e$b)
  b <- as.vector(e$b)
  Cns <- e$Cns
  m <- nrow(Cns)
  R <- Cns[, -ncol(Cns), drop = FALSE]
  rhs <- Cns[, ncol(Cns)]
  robust <- !is.null(e[["vcetype"]])
  cl <- e[["N_clust"]]
  Fv <- e$F
  stata_ls_header("Constrained linear regression",
                  c(list(list("Number of obs", stata_tab_count(e$N))),
                    if (is.finite(Fv)) list(list(sprintf("F(%d, %d)", as.integer(e$df_m), as.integer(e$df_r)), sprintf("%.2f", Fv)),
                                            list("Prob > F", sprintf("%.4f", stats::pf(Fv, e$df_m, e$df_r, lower.tail = FALSE)))),
                    list(list("Root MSE", sprintf("%.4f", e$rmse)))))
  cat(paste0(sprintf(" (%2d)  ", seq_len(m)), vapply(seq_len(m), function(i) stata_cns_line(R[i, ], rhs[i], nm), ""), "\n"), sep = "")
  stata_ls_table(nm, b, sqrt(diag(e$V)), e$df_r, level, e$depvar, attr(e, "stata_constr"), robust = robust,
                 clusters = cl, clustvar = e[["clustvar"]])
  invisible(NULL)
}
stata_cnsreg_replay <- function(e, level = 95) {
  if (is.null(e) || !identical(e[["cmd"]], "cnsreg")) stop("last estimates not found (Stata: r(301))", call. = FALSE)
  stata_cnsreg_show(e, level)
}"##;

fn stop(out: &mut Out, msg: &str) {
    out.line(format!("stop({}, call. = FALSE)", r_text(msg)));
}

/// The names `r()` holds after `constraint get` and `constraint free`, which
/// are text.
const R_CONSTRAINT_GET: &[&str] = &["contents"];
const R_CONSTRAINT_FREE: &[&str] = &["free"];

impl Session {
    /// `constraint [define] # exp`, `constraint dir | list [numlist]`,
    /// `constraint get #`, `constraint free` and `constraint drop numlist`.
    pub(super) fn constraint(&mut self, c: &Command, out: &mut Out) -> Result<(), Untranslated> {
        self.no_by(c, "constraint")?;
        if let Some(o) = c.options.first() {
            return Err(no(format!(
                "constraint with the option {} is not translated yet",
                o.name
            )));
        }
        let (first, rest) = first_word(&c.parts.main);
        self.helpers.insert("stata_lsfam");
        for h in ["stata_fmt", "stata_tables"] {
            self.helpers.insert(h);
        }
        let call = |what: &str, arg: &str, text: &str| {
            format!(
                "stata_cns <- stata_constraint(get0(\"stata_cns\", ifnotfound = list()), {}, {}, {})",
                r_text(what),
                r_text(arg),
                r_text(text)
            )
        };
        let is = |full: &str, min: usize| first.len() >= min && full.starts_with(first);
        if first.is_empty() {
            stop(out, "invalid syntax (Stata: r(198))");
            return Ok(());
        }
        if first.chars().all(|ch| ch.is_ascii_digit()) || is("define", 3) {
            let (num, text) = if is("define", 3) && !first.chars().all(|ch| ch.is_ascii_digit()) {
                first_word(rest)
            } else {
                (first, rest)
            };
            if !num.chars().all(|ch| ch.is_ascii_digit()) || num.is_empty() {
                stop(out, "invalid syntax (Stata: r(198))");
                return Ok(());
            }
            if text.contains('"') || text.contains('`') {
                // Stata stops with invalid syntax on a quoted text; compound quotes were not checked
                stop(out, "invalid syntax (Stata: r(198))");
                return Ok(());
            }
            match num.parse::<u32>() {
                Ok(n) if (1..=1999).contains(&n) => {}
                Ok(0) => {
                    stop(
                        out,
                        "constraint number must be between 1 and 1999 (Stata: r(198))",
                    );
                    return Ok(());
                }
                _ => {
                    return Err(no(
                        "constraint numbers above 1999 are not translated yet (Stata accepts them without a message)",
                    ))
                }
            }
            out.line(call("define", num, text));
            return Ok(());
        }
        if is("list", 1) || is("dir", 3) {
            out.line(call("list", rest, ""));
            return Ok(());
        }
        if is("drop", 3) {
            if rest.trim().is_empty() {
                stop(out, "invalid syntax (Stata: r(198))");
                return Ok(());
            }
            out.line(call("drop", rest, ""));
            return Ok(());
        }
        if is("get", 3) {
            if rest.trim().is_empty() {
                stop(out, "invalid syntax (Stata: r(198))");
                return Ok(());
            }
            out.line(call("get", rest.trim(), ""));
            out.line("stata_r <- attr(stata_cns, \"r\"); attr(stata_cns, \"r\") <- NULL");
            self.r_new = Some((R_CONSTRAINT_GET, "constraint".into()));
            return Ok(());
        }
        if is("free", 3) {
            out.line(call("free", "", ""));
            out.line("stata_r <- attr(stata_cns, \"r\"); attr(stata_cns, \"r\") <- NULL");
            self.r_new = Some((R_CONSTRAINT_FREE, "constraint".into()));
            return Ok(());
        }
        stop(out, "invalid syntax (Stata: r(198))");
        Ok(())
    }

    /// `cnsreg depvar indepvars [if] [in] [weight], constraints(numlist) [noconstant
    /// vce(robust | cluster v) level(#)]`.
    pub(super) fn cnsreg(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "cnsreg")?;
        if let Some(o) = opts.other(&[
            ("constraints", 1),
            ("noconstant", 5),
            ("robust", 1),
            ("vce", 3),
            ("level", 3),
        ]) {
            return Err(no(format!(
                "cnsreg with the option {o} is not translated yet"
            )));
        }
        let spec_words = c.parts.main.split_whitespace().count();
        if spec_words == 0 {
            // a bare cnsreg shows the last cnsreg again
            return self.cnsreg_replay(c, opts, out);
        }
        let spec = Spec::read(c, "cnsreg")?;
        if !is_name(&spec.dep) || spec.terms.is_empty() || spec.terms.iter().any(|t| !is_name(t)) {
            return Err(no(
                "cnsreg with factor variables, operators or no independent variable is not translated yet",
            ));
        }
        let Some(list) = opts.arg("constraints", 1) else {
            stop(out, "constraints() required (Stata: r(198))");
            return Ok(());
        };
        let w = self.weight_of(c, &[W::F, W::A, W::P, W::I])?;
        if matches!(w, Some((W::I, _))) {
            return Err(no("cnsreg with iweights is not translated yet"));
        }
        let level = level(opts)?;
        let (robust, cluster) = super::model::vce(opts)?;
        let rows = self.rows_for(c, out)?;
        let src = self.weighted_rows(rows, &w);
        for h in [
            "stata_lsfam",
            "stata_fmt",
            "stata_tables",
            "stata_returned",
            "stata_eclass",
        ] {
            self.helpers.insert(h);
        }
        let wargs = match &w {
            Some((k, v)) => format!(", wvar = {}, wtype = {}", r_text(v), r_text(k.code())),
            None => String::new(),
        };
        let cl = match &cluster {
            Some(v) => format!(", cluster = {}", r_text(v)),
            None => String::new(),
        };
        let show = !self.quiet(c);
        out.line(format!(
            "stata_e <- stata_cnsreg({src}, {}, c({}), stata_numlist({}), noconst = {}{wargs}, robust = {}{cl}, level = {}, cmdline = {}, show = {})",
            r_text(&spec.dep),
            spec.terms
                .iter()
                .map(|t| r_text(t))
                .collect::<Vec<_>>()
                .join(", "),
            r_text(&list),
            super::stats::r_logical(opts.has("noconstant", 5)),
            super::stats::r_logical(robust),
            r_number(level * 100.0),
            r_text(&super::returned::cmdline(c)),
            super::stats::r_logical(show)
        ));
        self.cnsreg_after(c, &spec.dep, &src, out)?;
        if w.is_some() {
            out.caveat(super::weights::weight_caveat(&w));
        }
        out.caveat(
            "cnsreg as Stata 19 fits it: restricted least squares by projection, equal constraints counted once, \
             the F test of the slopes only when it does not contradict the constraints",
        );
        Ok(())
    }

    /// What stays in R after a `cnsreg`: the coefficients, their variance, the
    /// rows used and e().
    fn cnsreg_after(
        &mut self,
        c: &Command,
        dep: &str,
        src: &str,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        out.line("stata_b <- attr(stata_e, \"stata_b\")");
        out.line("stata_vcov <- attr(stata_e, \"stata_V\")");
        out.line("stata_df <- stata_e$df_r");
        out.line("stata_estimates <- list(rows = attr(stata_e, \"stata_rows\"))");
        out.line(self.keep_rows(src, "stata_estimates$rows"));
        let d = self.data()?;
        out.package("dplyr");
        out.line(super::model::esample_rows_line(
            &d,
            src,
            "stata_estimates$rows",
        ));
        self.esample_new = true;
        self.fit_robust = false;
        self.fit_cluster = false;
        self.estimates = Some(Estimates {
            fit: Fit::Cnsreg,
            dep: dep.to_string(),
            factors: BTreeMap::new(),
        });
        self.e_new = true;
        self.e_ret = super::returned::Returned::Program {
            cmd: "cnsreg".into(),
            texts: [
                "cmdline",
                "vcetype",
                "vce",
                "clustvar",
                "cmd",
                "title",
                "predict",
                "depvar",
                "marginsok",
                "properties",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
        };
        let _ = c;
        Ok(())
    }

    /// A bare `cnsreg`: the last cnsreg's results again.
    fn cnsreg_replay(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let level = level(opts)?;
        if opts.has("constraints", 1) {
            return Err(no(
                "cnsreg with constraints() but no variables is not translated yet",
            ));
        }
        for h in [
            "stata_lsfam",
            "stata_fmt",
            "stata_tables",
            "stata_returned",
            "stata_eclass",
        ] {
            self.helpers.insert(h);
        }
        if !matches!(&self.e_ret, super::returned::Returned::Program { cmd, .. } if cmd == "cnsreg")
        {
            return Err(no(
                "a bare cnsreg after another command is not translated yet",
            ));
        }
        if !self.quiet(c) {
            out.line(format!(
                "stata_cnsreg_replay(stata_e, level = {})",
                r_number(level * 100.0)
            ));
        }
        Ok(())
    }
}

impl Session {
    /// `logistic depvar indepvars [if] [in] [weight] [, coef level() vce() robust noconstant]`:
    /// `logit` that shows odds ratios (`coef` the coefficients) and prints no iteration log; e()
    /// is logit's with `e(cmd)` `logistic` (checked in Stata 19, `plan/stata/logistic.md`).
    pub(super) fn logistic(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if opts.has("or", 2) {
            return Err(no("logistic with the option or: Stata does not allow it"));
        }
        if opts.has("noconstant", 5) {
            return Err(no(
                "logistic, noconstant is not translated: Stata leaves e(ll_0) missing and a Wald chi2 there",
            ));
        }
        let coef = opts.has("coef", 4);
        let mut c2 = c.clone();
        c2.name = "logit".into();
        c2.options.retain(|o| !o.is("coef", 4));
        if !coef {
            c2.options.push(crate::lang::syntax::Opt {
                name: "or".into(),
                arg: None,
                raw: None,
            });
        }
        if !c2.options.iter().any(|o| o.is("nolog", 5)) {
            c2.options.push(crate::lang::syntax::Opt {
                name: "nolog".into(),
                arg: None,
                raw: None,
            });
        }
        let o2 = super::Opts(&c2.options);
        self.ml(&c2, &o2, out, Fit::Logit)?;
        out.line("stata_e[[\"cmd\"]] <- \"logistic\"; stata_e[[\"predict\"]] <- \"logistic_p\"");
        Ok(())
    }
}
