//! `cmset caseid altvar` and `cmclogit depvar [alternative-specific terms] [if] [in],
//! casevars(terms) [noconstant vce(cluster v)]` with `margins, dydx(factors)
//! outcome(#) alt(#) [asobserved post vce(unconditional) cformat()]` after it, as
//! Stata 19 runs them (observed with made-up data, `plan/stata/cmclogit.md`).
//!
//! cmclogit is the conditional logit of the choice on the alternatives' own terms
//! (factor levels as dummies, the base one at 0), with each case-specific term
//! entering every alternative but the base one (the most frequent in the data, the lowest
//! of a tie), its equation named by the alternative's value; its estimates and its
//! cluster-robust variance G/(G-1) H^-1 (sum of cluster scores)^2 H^-1 are
//! Stata's to 1e-10 on small data, to about 1e-7 on large (Stata stops short). margins' dy/dx of a factor level is the average over cases
//! of Pr(outcome chosen) with the alternative's term at that level less at the base
//! level; its unconditional variance clusters the cases' influence (their effect
//! less the mean, over n, plus the Jacobian times the coefficients' influence).

use super::*;

pub(super) const CM_HELPER: &str = r##"stata_cmclogit <- function(d, dep, alt_terms, case_terms, case, alt, cluster = NULL, cons = TRUE, show = TRUE) {
  num <- function(v) { u <- unclass(d[[v]]); attributes(u) <- NULL; as.numeric(u) }
  vars <- unique(c(dep, case, alt, cluster, vapply(c(alt_terms, case_terms), function(t) t$var, "")))
  for (v in vars) if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
  ok <- stats::complete.cases(as.data.frame(lapply(vars, num)))
  rows <- which(ok)
  dd <- d[rows, , drop = FALSE]
  o <- order(num(case)[rows], num(alt)[rows])
  rows <- rows[o]
  dd <- d[rows, , drop = FALSE]
  cs <- num(case)[rows]; av <- num(alt)[rows]; y <- as.numeric(num(dep)[rows] != 0)
  idx <- match(cs, unique(cs))
  if (any(tapply(y, cs, sum) != 1)) stop("RCraft: cmclogit with cases that do not choose exactly one alternative is not translated yet", call. = FALSE)
  alts <- sort(unique(av))
  # the base alternative: the most frequent in the data, the first of a tie (Stata 19)
  freq <- vapply(alts, function(a) sum(av == a), 1)
  base_alt <- alts[which.max(freq)]
  # the columns: each term's levels (the base one kept at 0), Stata's names
  cols <- list(); X <- NULL
  # a level's value label, else its value, as Stata's table names it
  vlab <- function(var, l) {
    s <- attr(d[[var]], "labels", exact = TRUE)
    i <- if (is.null(s)) NA else match(l, as.numeric(s))
    if (is.na(i)) as.character(l) else names(s)[i]
  }
  add <- function(name, x, eq, base = FALSE, term = NA, level = NA, lab = NA) {
    cols[[length(cols) + 1]] <<- list(name = name, eq = eq, base = base, term = term, level = level, lab = lab)
    X <<- cbind(X, x)
  }
  for (t in alt_terms) {
    v <- num(t$var)[rows]
    if (isTRUE(t$factor)) {
      lv <- sort(unique(v)); b0 <- if (is.null(t$base)) lv[1] else t$base
      for (l in lv) add(paste0(l, if (l == b0) "b" else "", ".", t$var), (v == l) * (l != b0), "alt", l == b0, t$var, l, vlab(t$var, l))
    } else add(t$var, v, "alt", FALSE, t$var, NA)
  }
  for (a in setdiff(alts, base_alt)) {
    for (t in case_terms) {
      v <- num(t$var)[rows]
      if (isTRUE(t$factor)) {
        lv <- sort(unique(v)); b0 <- if (is.null(t$base)) lv[1] else t$base
        for (l in lv) add(paste0(l, if (l == b0) "b" else "", ".", t$var), (v == l) * (l != b0) * (av == a), as.character(a), l == b0, t$var, l, vlab(t$var, l))
      } else add(t$var, v * (av == a), as.character(a), FALSE, t$var, NA)
    }
    if (cons) add("_cons", as.numeric(av == a), as.character(a))
  }
  free <- !vapply(cols, function(x) x$base, TRUE)
  Xf <- X[, free, drop = FALSE]
  fit <- function(Xf) {
    b <- rep(0, ncol(Xf))
    for (it in 1:100) {
      eta <- as.numeric(Xf %*% b); ex <- exp(eta - max(eta)); P <- ex / rowsum(ex, idx, reorder = FALSE)[idx]
      R <- Xf - rowsum(Xf * P, idx, reorder = FALSE)[idx, , drop = FALSE]
      g <- colSums(R * (y - P)); H <- crossprod(R * sqrt(P))
      step <- solve(H, g); b <- b + step
      if (max(abs(step)) < 1e-12) break
    }
    eta <- as.numeric(Xf %*% b); ex <- exp(eta - max(eta)); P <- ex / rowsum(ex, idx, reorder = FALSE)[idx]
    R <- Xf - rowsum(Xf * P, idx, reorder = FALSE)[idx, , drop = FALSE]
    list(b = b, P = P, H = crossprod(R * sqrt(P)), R = R)
  }
  f <- fit(Xf)
  Hi <- solve(f$H)
  sc <- rowsum(f$R * (y - f$P), cs)
  clus <- if (!is.null(cluster)) num(cluster)[rows] else NULL
  if (!is.null(clus)) {
    cl_case <- tapply(clus, cs, function(x) x[1])
    scg <- rowsum(sc, cl_case[as.character(rownames(sc))]); G <- nrow(scg)
    Vf <- G / (G - 1) * Hi %*% crossprod(scg) %*% Hi
  } else {
    G <- NA; Vf <- Hi
  }
  k <- length(cols)
  bfull <- numeric(k); bfull[free] <- f$b
  V <- matrix(0, k, k); V[free, free] <- Vf
  nm <- vapply(cols, function(x) paste0(x$eq, ":", x$name), "")
  bm <- matrix(bfull, 1, dimnames = list("y1", nm)); dimnames(V) <- list(nm, nm)
  ll <- sum(y * log(f$P))
  chi2 <- as.numeric(t(f$b) %*% solve(Vf, f$b))
  sz <- as.numeric(table(cs))
  e <- list(N = length(y), N_case = length(sz), N_clust = if (!is.null(clus)) G else NULL, ll = ll,
            df_m = sum(free), chi2 = chi2, p = stats::pchisq(chi2, sum(free), lower.tail = FALSE),
            k_alt = length(alts), alt_min = min(sz), alt_avg = mean(sz), alt_max = max(sz), i_base = match(base_alt, alts),
            cmd = "cmclogit", depvar = dep, caseid = case, altvar = alt, title = "Conditional logit choice model",
            clustvar = cluster, vce = if (!is.null(clus)) "cluster" else "oim", vcetype = if (!is.null(clus)) "Robust" else NULL,
            properties = "b V", b = bm, V = V)
  e <- e[!vapply(e, is.null, TRUE)]
  attr(e, "stata_sample") <- FALSE
  # kept for margins: the design and the fit
  stata_cm <<- list(X = X, free = free, cols = cols, b = f$b, Hi = Hi, sc = sc, cs = cs, idx = idx, av = av, y = y,
                    clus = clus, alts = alts, base_alt = base_alt, Vf = Vf, rows = rows)
  if (show) stata_cm_print(e, cols, bfull, V, alts, base_alt, case, alt, dep, cluster, X, cs)
  e
}
stata_cm_wrap <- function(s, w = 77, indent = "      ") {
  words <- strsplit(s, " ")[[1]]; lines <- character(); cur <- ""
  for (x in words) {
    if (!nzchar(cur)) cur <- x else if (nchar(cur) + 1 + nchar(x) <= w) cur <- paste(cur, x) else { lines <- c(lines, cur); cur <- paste0(indent, x) }
  }
  paste(c(lines, cur), collapse = "\n")
}
stata_cm_print <- function(e, cols, b, V, alts, base_alt, case, alt, dep, cluster, X, cs) {
  for (j in seq_along(cols)) {
    cl <- cols[[j]]
    if (cl$eq != "alt" || cl$base) next
    n <- sum(tapply(X[, j], cs, function(x) length(unique(x)) == 1))
    if (n > 0) cat(stata_cm_wrap(sprintf("note: variable %s has %d cases that are not alternative-specific; there is no within-case variability.", cl$name, n)), "\n", sep = "")
  }
  cat("\n")
  row <- function(left, right) cat(formatC(left, width = -47, flag = "-"), right, "\n", sep = "")
  row("Conditional logit choice model", sprintf("Number of obs      = %10s", format(e$N, big.mark = ",")))
  row(paste("Case ID variable:", case), sprintf("Number of cases    = %10d", as.integer(e$N_case)))
  cat("\n")
  row(paste("Alternatives variable:", alt), sprintf("Alts per case: min = %10d", as.integer(e$alt_min)))
  row("", sprintf("               avg = %10.1f", e$alt_avg))
  row("", sprintf("               max = %10d", as.integer(e$alt_max)))
  cat("\n")
  cat(formatC("", width = 50), sprintf("%-16s= %10.2f\n", sprintf("Wald chi2(%d)", as.integer(e$df_m)), e$chi2), sep = "")
  lab <- if (is.null(cluster)) "Log likelihood" else "Log pseudolikelihood"
  row(sprintf("%s = %.8g", lab, e$ll), sprintf("   Prob > chi2     = %10.4f", e$p))
  cat("\n")
  if (!is.null(cluster)) cat(formatC(sprintf("(Std. err. adjusted for %s clusters in %s)", format(as.integer(e$N_clust), big.mark = ","), cluster), width = 78), "\n", sep = "")
  rule <- strrep("-", 78); mid <- paste0(strrep("-", 13), "+", strrep("-", 64))
  g9 <- function(v) stata_fmt(v, "%9.0g")
  cat(rule, "\n", sep = "")
  cat(if (is.null(cluster)) "             |\n" else "             |               Robust\n")
  cat(sprintf("%12s | Coefficient  std. err.      z    P>|z|     [95%% conf. interval]\n", dep))
  cat(mid, "\n", sep = "")
  q <- stats::qnorm(0.975)
  val <- function(lab, j) {
    se <- sqrt(V[j, j]); z <- b[j] / se
    cat(sprintf("%12s | %10s %10s %8.2f %7.3f %12s %11s\n", lab, g9(b[j]), g9(se), z, 2 * stats::pnorm(-abs(z)), g9(b[j] - q * se), g9(b[j] + q * se)))
  }
  eqblock <- function(eq) {
    js <- which(vapply(cols, function(x) x$eq == eq && !x$base, TRUE))
    terms <- unique(vapply(cols[js], function(x) x$term, ""))
    started <- FALSE
    for (t in terms) {
      jt <- js[vapply(cols[js], function(x) identical(x$term, t), TRUE)]
      fac <- !is.na(cols[[jt[1]]]$level)
      if (fac && length(jt) > 1) {
        if (started) cat("             |\n")
        cat(sprintf("%12s |\n", t))
        # labels longer than 11 cut to 9 and ".."
        for (j in jt) {
          s <- cols[[j]]$lab
          if (nchar(s) > 11) s <- paste0(substr(s, 1, 9), "..")
          val(sprintf("%11s ", s), j)
        }
        started <- TRUE
        next
      }
      if (started && fac) cat("             |\n")
      for (j in jt) val(cols[[j]]$name, j)
      started <- TRUE
    }
  }
  cat(sprintf("%-12s |\n", "alt"))
  eqblock("alt")
  for (a in alts) {
    cat(mid, "\n", sep = "")
    if (a == base_alt) { cat(sprintf("%-12s |  (base alternative)\n", a)); next }
    cat(sprintf("%-12s |\n", a))
    eqblock(as.character(a))
  }
  cat(rule, "\n", sep = "")
}
# margins, dydx(factors) outcome(o) alt(a): the average over cases of Pr(o chosen)
# with alternative a's factor at each level less at its base level
stata_cm_margins <- function(terms, outcome, altv, unconditional = TRUE, cformat = NULL, show = TRUE) {
  m <- stata_cm
  free <- m$free; cols <- m$cols; X <- m$X; cs <- m$cs; idx <- m$idx
  orow <- which(m$av == outcome)
  if (length(orow) != length(unique(cs))) stop("RCraft: margins after cmclogit with cases without the outcome's alternative is not translated yet", call. = FALSE)
  # Pr(outcome) in each case and its gradient P_o (x_o - sum_j P_j x_j)
  pr <- function(Xm) {
    Xf <- Xm[, free, drop = FALSE]
    eta <- as.numeric(Xf %*% m$b); ex <- exp(eta - max(eta)); P <- ex / rowsum(ex, idx, reorder = FALSE)[idx]
    xbar <- rowsum(Xf * P, idx, reorder = FALSE)
    po <- P[orow]
    list(p = po, g = po * (Xf[orow, , drop = FALSE] - xbar))
  }
  at <- m$av == altv
  rowsn <- character(); based <- logical(); pairs <- list()
  for (t in terms) {
    js <- which(vapply(cols, function(x) identical(x$term, t) && x$eq == "alt", TRUE))
    if (!length(js) || is.na(cols[[js[1]]]$level)) stop("RCraft: margins, dydx(", t, ") of a term that is not an alternative-specific factor of cmclogit is not translated yet", call. = FALSE)
    off <- X
    for (q in js) off[at, q] <- 0
    p_off <- NULL
    for (j in js) {
      lev <- cols[[j]]$level
      rowsn <- c(rowsn, paste0(lev, if (cols[[j]]$base) "b" else "", ".", t))
      based <- c(based, cols[[j]]$base)
      if (cols[[j]]$base) { pairs[length(pairs) + 1] <- list(NULL); next }
      if (is.null(p_off)) p_off <- pr(off)
      on <- off
      on[at, j] <- 1
      pairs[[length(pairs) + 1]] <- list(on = pr(on), off = p_off)
    }
  }
  nr <- length(rowsn)
  est <- numeric(nr); J <- matrix(0, nr, length(m$b)); E <- matrix(0, length(unique(cs)), nr)
  for (i in seq_len(nr)) {
    if (based[i]) next
    pp <- pairs[[i]]
    e1 <- pp$on$p - pp$off$p
    est[i] <- mean(e1); E[, i] <- e1
    J[i, ] <- colMeans(pp$on$g - pp$off$g)
  }
  n <- nrow(E)
  if (unconditional) {
    IF <- sweep(E, 2, colMeans(E)) / n + (m$sc %*% t(m$Hi)) %*% t(J)
    IF[, based] <- 0
    if (!is.null(m$clus)) {
      cl_case <- tapply(m$clus, cs, function(x) x[1])
      IFg <- rowsum(IF, cl_case); G <- nrow(IFg)
      V <- G / (G - 1) * crossprod(IFg)
    } else {
      V <- n / (n - 1) * crossprod(IF)
    }
  } else V <- J %*% m$Vf %*% t(J)
  nm <- paste0(rowsn, ":", ifelse(based, "o._cons", "_cons"))
  bm <- matrix(est, 1, dimnames = list("y1", nm)); dimnames(V) <- list(nm, nm)
  if (show) {
    fm <- function(v) if (is.null(cformat)) stata_fmt(v, "%9.0g") else stata_fmt(v, cformat)
    cat("\n")
    # the count ends at column 78
    nobs <- format(length(m$y), big.mark = ",")
    cat(formatC("Average marginal effects", width = -(62 - nchar(nobs)), flag = "-"), "Number of obs = ", nobs, "\n", sep = "")
    cat("\n")
    cat(sprintf("Expression:  Pr(%s|%s selected), predict()\n", stata_e$altvar, outcome))
    cat(sprintf("Alternative: %s\n", altv))
    cat(sprintf("Outcome:     %s\n", outcome))
    # the terms wrapped at 78 columns, the next lines under the first term
    wrt <- "dy/dx wrt:  "
    for (r in rowsn[!based]) {
      last <- sub("^.*\n", "", wrt)
      wrt <- if (nchar(last) + 1 + nchar(r) > 78) paste0(wrt, "\n", strrep(" ", 13), r) else paste0(wrt, " ", r)
    }
    cat(wrt, "\n", sep = "")
    cat("\n")
    if (!is.null(m$clus)) cat(formatC(sprintf("(Std. err. adjusted for %s clusters in %s)", format(length(unique(m$clus)), big.mark = ","), stata_e$clustvar), width = 78), "\n", sep = "")
    rule <- strrep("-", 78); mid <- paste0(strrep("-", 13), "+", strrep("-", 64))
    cat(rule, "\n", sep = "")
    cat(if (unconditional) "             |            Unconditional\n" else "             |            Delta-method\n")
    cat("             |      dy/dx   std. err.      z    P>|z|     [95% conf. interval]\n")
    q <- stats::qnorm(0.975)
    for (i in seq_len(nr)) {
      cat(mid, "\n", sep = "")
      lab <- sub("b\\.", ".", rowsn[i])
      if (based[i]) { cat(sprintf("%-12s |  (base outcome)\n", lab)); next }
      cat(sprintf("%-12s |\n", lab))
      se <- sqrt(V[i, i]); z <- est[i] / se
      cat(sprintf("%12s | %10s %10s %8.2f %7.3f %12s %11s\n", "_cons", fm(est[i]), fm(se), z, 2 * stats::pnorm(-abs(z)), fm(est[i] - q * se), fm(est[i] + q * se)))
    }
    cat(rule, "\n", sep = "")
    cat("Note: dy/dx for factor levels is the discrete change from the base level.\n")
  }
  e <- list(N = length(m$y), N_clust = if (!is.null(m$clus)) length(unique(m$clus)) else NULL,
            cmd = "margins", title = "Average marginal effects", vce = if (unconditional) "unconditional" else "delta",
            vcetype = if (unconditional) "Unconditional" else "Delta-method", clustvar = stata_e$clustvar,
            est_cmd = "cmclogit", properties = "b V", b = bm, V = V)
  e <- e[!vapply(e, is.null, TRUE)]
  attr(e, "stata_sample") <- FALSE
  e
}"##;

/// One term of cmclogit: `x`, `i.x`, or `ib#.x` (the base level `#`).
fn cm_term(t: &str, fv_base: &BTreeMap<String, String>) -> Result<String, Untranslated> {
    let (factor, base, var) = if let Some(v) = t.strip_prefix("i.") {
        (true, fv_base.get(v).cloned(), v)
    } else if let Some(rest) = t.strip_prefix("ib") {
        let (b, v) = rest
            .split_once('.')
            .ok_or_else(|| no(format!("cmclogit {t}: not a term")))?;
        if b.parse::<f64>().is_err() {
            return Err(no(format!(
                "cmclogit {t}: ib{b}. is not translated yet (a number is)"
            )));
        }
        (true, Some(b.to_string()), v)
    } else {
        (false, None, t)
    };
    if !is_name(var) {
        return Err(no(format!(
            "cmclogit {t}: variables, i.x and ib#.x are translated (not interactions or operators)"
        )));
    }
    if base.as_deref().is_some_and(|b| b.parse::<f64>().is_err()) {
        return Err(no(format!(
            "cmclogit {t}: a base other than a number is not translated yet"
        )));
    }
    Ok(format!(
        "list(var = {}, factor = {}{})",
        r_text(var),
        super::stats::r_logical(factor),
        base.map(|b| format!(", base = {b}")).unwrap_or_default()
    ))
}

impl Session {
    /// `cmset caseid altvar`.
    pub(super) fn cmset(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[]) {
            return Err(no(format!(
                "cmset with the option {o} is not translated yet"
            )));
        }
        let w: Vec<&str> = c.parts.main.split_whitespace().collect();
        let [case, alt] = w.as_slice() else {
            return Err(no(
                "cmset is translated with a case variable and an alternatives variable",
            ));
        };
        if !is_name(case) || !is_name(alt) {
            return Err(no("cmset takes two variable names"));
        }
        if !self.quiet(c) {
            out.line(format!(
                "cat({})",
                r_text(&format!(
                    "\n     Case ID variable: {case}\nAlternatives variable: {alt}\n"
                ))
            ));
        }
        self.cmset = Some((case.to_string(), alt.to_string()));
        Ok(())
    }

    /// `cmclogit depvar [terms] [if] [in], casevars(terms) [noconstant vce(cluster v)]`.
    pub(super) fn cmclogit(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "cmclogit")?;
        if let Some(o) = opts.other(&[("casevars", 5), ("noconstant", 5), ("vce", 3)]) {
            return Err(no(format!(
                "cmclogit with the option {o} is not translated yet"
            )));
        }
        if c.parts.weight.is_some() {
            return Err(no("cmclogit with weights is not translated yet"));
        }
        let Some((case, alt)) = self.cmset.clone() else {
            return Err(no(
                "cmclogit needs cmset before it in this do-file (Stata: data not cmset, r(459))",
            ));
        };
        let (robust, cluster) = super::model::vce(opts)?;
        if robust && cluster.is_none() {
            return Err(no(
                "cmclogit with vce(robust) is not translated yet (vce(cluster v) is)",
            ));
        }
        let words: Vec<&str> = c.parts.main.split_whitespace().collect();
        let Some((dep, terms)) = words.split_first() else {
            return Err(no("cmclogit needs a dependent variable"));
        };
        if !is_name(dep) {
            return Err(no(format!(
                "cmclogit {dep}: the dependent variable is a name"
            )));
        }
        let alt_terms = terms
            .iter()
            .map(|t| cm_term(t, &self.fv_base))
            .collect::<Result<Vec<_>, _>>()?;
        let case_terms = match opts.arg("casevars", 5) {
            None => Vec::new(),
            Some(a) => a
                .split_whitespace()
                .map(|t| cm_term(t, &self.fv_base))
                .collect::<Result<Vec<_>, _>>()?,
        };
        let src = self.rows_for(c, out)?;
        for h in ["stata_cm", "stata_fmt", "stata_returned", "stata_eclass"] {
            self.helpers.insert(h);
        }
        out.line(format!(
            "stata_e <- stata_cmclogit({src}, {}, list({}), list({}), {}, {}, cluster = {}, cons = {}, show = {})",
            r_text(dep),
            alt_terms.join(", "),
            case_terms.join(", "),
            r_text(&case),
            r_text(&alt),
            cluster.as_deref().map(r_text).unwrap_or_else(|| "NULL".into()),
            super::stats::r_logical(!opts.has("noconstant", 5)),
            super::stats::r_logical(!self.quiet(c))
        ));
        self.estimates = None;
        self.estimates_lost =
            Some("predict, test and lincom are not translated after cmclogit yet".into());
        self.active_store = None;
        self.e_text.clear();
        self.esample = false;
        self.e_ret = super::returned::Returned::Program {
            cmd: "cmclogit".into(),
            texts: [
                "cmd",
                "depvar",
                "caseid",
                "altvar",
                "title",
                "clustvar",
                "vce",
                "vcetype",
                "properties",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
        };
        out.caveat(
            "cmclogit's fit as Stata 19's to about 1e-7 (Stata stops its iterations a little short of the maximum, RCraft goes to it); its iteration log is not shown",
        );
        Ok(())
    }

    /// `margins, dydx(factors) outcome(#) alt(#) [asobserved post vce(unconditional)
    /// cformat()]` after cmclogit.
    pub(super) fn cm_margins(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[
            ("dydx", 4),
            ("outcome", 3),
            ("alternative", 3),
            ("asobserved", 5),
            ("post", 4),
            ("vce", 3),
            ("cformat", 7),
        ]) {
            return Err(no(format!(
                "margins after cmclogit with the option {o} is not translated yet"
            )));
        }
        if !c.parts.main.trim().is_empty() || c.parts.if_exp.is_some() || c.parts.in_range.is_some()
        {
            return Err(no(
                "margins after cmclogit with a varlist, if or in is not translated yet",
            ));
        }
        let terms: Vec<String> = opts
            .arg("dydx", 4)
            .ok_or_else(|| no("margins after cmclogit is translated with dydx()"))?
            .split_whitespace()
            .map(|t| {
                t.strip_prefix("i.")
                    .filter(|v| is_name(v))
                    .map(str::to_string)
                    .ok_or_else(|| {
                        no(format!(
                            "margins, dydx({t}) after cmclogit: factors i.x are translated"
                        ))
                    })
            })
            .collect::<Result<_, _>>()?;
        let num = |o: &str, m: usize| -> Result<String, Untranslated> {
            let v = opts
                .arg(o, m)
                .ok_or_else(|| no(format!("margins after cmclogit is translated with {o}()")))?;
            v.trim().parse::<f64>().map(r_number).map_err(|_| {
                no(format!(
                    "margins, {o}({v}): an alternative's value is translated"
                ))
            })
        };
        let outcome = num("outcome", 3)?;
        let alt = num("alternative", 3)?;
        let unconditional = match opts.arg("vce", 3) {
            None => false,
            Some(v) if v.trim() == "unconditional" => true,
            Some(v) if v.trim() == "delta" => false,
            Some(v) => {
                return Err(no(format!(
                    "margins, vce({v}) after cmclogit is not translated yet"
                )))
            }
        };
        let cformat = match opts.arg("cformat", 7) {
            None => "NULL".to_string(),
            Some(f) => r_text(f.trim()),
        };
        let call = format!(
            "stata_cm_margins(c({}), {outcome}, {alt}, unconditional = {}, cformat = {cformat}, show = {})",
            terms.iter().map(|t| r_text(t)).collect::<Vec<_>>().join(", "),
            super::stats::r_logical(unconditional),
            super::stats::r_logical(!self.quiet(c))
        );
        if opts.has("post", 4) {
            out.line(format!("stata_e <- {call}"));
            self.e_ret = super::returned::Returned::Program {
                cmd: "margins".into(),
                texts: [
                    "cmd",
                    "title",
                    "vce",
                    "vcetype",
                    "clustvar",
                    "est_cmd",
                    "properties",
                ]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            };
        } else {
            return Err(no(
                "margins after cmclogit without post is not translated yet (its r() is not kept)",
            ));
        }
        out.caveat(
            "margins after cmclogit as Stata 19's, from the model's estimates (to about 1e-7)",
        );
        Ok(())
    }
}
