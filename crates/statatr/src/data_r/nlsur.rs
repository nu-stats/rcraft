//! `nlsur (y1 = <expression with {b0 = #} parameters>) (y2 = ...) [if] [in] [, nls fgnls ifgnls
//! nolog level() iterate() eps() noconstant initial()]`: systems of nonlinear
//! equations as Stata 19 runs them (`plan/stata/nlsur.md`).
//!
//! The iteration log is the Gauss-Newton path itself (ported from StatCraft's probe 537, then
//! checked against Stata on made-up data here):
//!
//! - Each step works on the residuals whitened by Sigma (the identity for NLS), with forward
//!   differences d = 4e-7 |b| (4e-7 itself at 0), the step halved
//!   while it raises the criterion and never stepped further, stopping once no parameter moved by
//!   more than eps() (1e-5, relative to its old size |b|). A parameter whose derivative column is zero or a
//!   combination of earlier ones' does not move in that step; those still dependent at the end
//!   show as `(constrained)`.
//! - NLS first (the log says "Residual SS"), then FGNLS with Sigma = R'R/N from the NLS residuals
//!   ("Scaled RSS"); `ifgnls` goes round again with Sigma from the last residuals until the
//!   parameters and Sigma each change by at most 1e-5.
//! - Variances: FGNLS (J'(Sigma^-1 kron I)J)^-1; NLS the sandwich (J'J)^-1 J'(Sigma kron I)J
//!   (J'J)^-1.

use super::*;

/// The fit, its log, Stata's tables and e().
pub(super) const NLSUR_HELPER: &str = r##"stata_nlsur <- function(d, eqs, init, level = 95, method = "fgnls", nolog = FALSE, noconst = FALSE,
                        iterate = 100L, eps = 1e-5, delta = 4e-7, cmdline = "", show = TRUE) {
  M <- length(eqs)
  pn <- names(init)
  k <- length(pn)
  clash <- intersect(pn, names(d))
  if (length(clash)) stop("RCraft: nlsur with a parameter named as a variable (", clash[1], ") is not translated yet", call. = FALSE)
  deps <- vapply(eqs, function(q) q$dep, "")
  syms <- unique(unlist(lapply(eqs, function(q) all.vars(body(q$f)))))
  miss <- Filter(function(s) !s %in% c(names(d), pn, ".v") && !exists(s, envir = globalenv()), syms)
  if (length(miss)) stop("variable ", miss[1], " not found (Stata: r(111))", call. = FALSE)
  for (v in deps) if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
  vars <- unique(c(deps, intersect(syms, names(d))))
  for (v in vars) if (is.character(d[[v]])) stop(v, ": string variable, type mismatch (Stata: r(109))", call. = FALSE)
  num <- function(u) { u <- unclass(u); attributes(u) <- NULL; as.numeric(u) }
  rows <- which(stats::complete.cases(d[deps]))
  n <- length(rows)
  if (show) cat("(obs = ", stata_tab_count(n), ")\n\n", sep = "")
  if (n * M < k) stop("cannot have fewer observations than parameters (Stata: r(2001))", call. = FALSE)
  base <- lapply(d[rows, vars, drop = FALSE], num)
  evq <- function(q, b) {
    r <- num(eqs[[q]]$f(c(base, as.list(b))))
    if (length(r) == 1L) rep(r, n) else r
  }
  ev <- function(b) unlist(lapply(seq_len(M), function(q) evq(q, b)), use.names = FALSE)
  y <- unlist(lapply(deps, function(v) base[[v]]), use.names = FALSE)
  b0 <- init
  wh <- function(v, Linv) if (is.null(Linv)) v else as.vector(matrix(v, n, M) %*% t(Linv))
  jac <- function(b) {
    f0 <- ev(b)
    matrix(vapply(seq_len(k), function(j) {
      bj <- b[[j]]
      dj <- if (bj == 0) delta else delta * abs(bj)
      up <- b
      up[j] <- bj + dj
      dj <- up[[j]] - bj
      (ev(up) - f0) / dj
    }, numeric(length(f0))), length(f0), k)
  }
  # the parameters whose columns in J'J are neither zero nor a combination of earlier ones'
  independent <- function(A) {
    s <- A
    free <- integer()
    for (p in seq_len(k)) {
      dg <- A[p, p]
      pv <- s[p, p]
      if (!is.finite(dg) || dg <= 0 || pv <= 1e-13 * dg) next
      free <- c(free, p)
      rp <- s[p, ]
      for (i in seq_len(k)) {
        if (i == p) next
        s[i, ] <- s[i, ] - (s[i, p] / pv) * rp
      }
    }
    free
  }
  mreldif <- function(a, b) max(abs(a - b) / (abs(b) + 1))
  gn <- function(b, Linv) {
    crit <- function(b) {
      s <- sum(wh(y - ev(b), Linv)^2)
      if (is.finite(s)) s else Inf
    }
    log <- numeric()
    conv <- FALSE
    for (it in seq_len(max(iterate, 1L))) {
      r <- wh(y - ev(b), Linv)
      cur <- sum(r^2)
      if (!is.finite(cur)) cur <- Inf
      J <- jac(b)
      Jl <- J
      Jw <- if (is.null(Linv)) J else matrix(vapply(seq_len(k), function(j) wh(J[, j], Linv), numeric(nrow(J))), nrow(J), k)
      A <- crossprod(Jw)
      g <- as.vector(crossprod(Jw, r))
      free <- independent(A)
      st <- numeric(k)
      if (length(free)) {
        sol <- tryCatch(solve(A[free, free, drop = FALSE], g[free]), error = function(e) NULL)
        if (is.null(sol) || !all(is.finite(sol))) stop("could not calculate numerical derivatives (Stata: r(481))", call. = FALSE)
        st[free] <- sol
      }
      t <- 1
      nxt <- crit(b + st)
      while (nxt > cur && t > 1e-15) {
        t <- t / 2
        nxt <- crit(b + t * st)
      }
      nb <- b + t * st
      log <- c(log, nxt)
      rel <- abs(nb - b) / abs(b)
      rel[nb == b] <- 0
      ch <- max(rel)
      b <- nb
      if (ch <= eps) {
        conv <- TRUE
        break
      }
    }
    list(b = b, log = log, converged = conv, J = Jl)
  }
  sigma_of <- function(r) crossprod(matrix(r, n, M)) / n
  linv <- function(S) solve(t(chol(S)))
  logs <- function(label, v) {
    for (i in seq_along(v)) {
      cat(sprintf("%-13s %s = %8s\n", paste0("Iteration ", i - 1L, ":"), label, trimws(stata_fmt(v[i], "%9.0g"))))
    }
  }
  calc <- function(what) if (show) cat("Calculating ", what, " estimates", if (nolog) " ..." else ":", "\n", sep = "")
  sci <- function(v) sub("e([+-])0([0-9]{2})$", "e\\1\\2", sprintf("%.2e", v))
  calc("NLS")
  for (q in seq_len(M)) {
    if (any(!is.finite(evq(q, b0)))) {
      stop("could not evaluate equation ", q, ": starting values invalid or some RHS variables have missing values (Stata: r(480))", call. = FALSE)
    }
  }
  nls <- gn(b0, NULL)
  if (show && !nolog) logs("Residual SS", nls$log)
  b <- nls$b
  lastgn <- nls
  conv <- nls$converged
  Sigma <- sigma_of(y - ev(b))
  Linv <- NULL
  if (method != "nls") {
    if (show && !nolog) cat("\n")
    calc("FGNLS")
    Linv <- linv(Sigma)
    fg <- gn(b, Linv)
    if (show && !nolog) logs("Scaled RSS", fg$log)
    b <- fg$b
    lastgn <- fg
    conv <- fg$converged
    if (method == "ifgnls") {
      conv <- FALSE
      for (round in 2:100) {
        Sigma <- sigma_of(y - ev(b))
        Linv <- linv(Sigma)
        if (show) {
          if (nolog) cat("FGNLS iteration ", round, " ...\n", sep = "") else cat("\nFGNLS iteration ", round, ":\n", sep = "")
        }
        nx <- gn(b, Linv)
        after <- sigma_of(y - ev(nx$b))
        pch <- max(abs(nx$b - b) / (abs(nx$b) + 1))
        cch <- mreldif(as.vector(after), as.vector(Sigma))
        if (show && !nolog) {
          logs("Scaled RSS", nx$log)
          cat("Parameter change         = ", sci(pch), "\n", sep = "")
          cat("Covariance matrix change = ", sci(cch), "\n", sep = "")
        }
        b <- nx$b
        lastgn <- nx
        if (pch <= 1e-5 && cch <= 1e-5) {
          conv <- TRUE
          break
        }
      }
    }
  }
  J <- if (method == "nls") jac(b) else lastgn$J
  free <- independent(crossprod(J))
  fr <- function(m) m[free, free, drop = FALSE]
  V <- matrix(0, k, k)
  if (length(free)) {
    small <- if (method == "nls") {
      bread <- solve(fr(crossprod(J)))
      L <- t(chol(Sigma))
      LJ <- matrix(vapply(seq_len(k), function(j) as.vector(matrix(J[, j], n, M) %*% L), numeric(nrow(J))), nrow(J), k)
      bread %*% fr(crossprod(LJ)) %*% bread
    } else {
      Jw <- matrix(vapply(seq_len(k), function(j) wh(J[, j], Linv), numeric(nrow(J))), nrow(J), k)
      solve(fr(crossprod(Jw)))
    }
    V[free, free] <- small
  }
  fit <- ev(b)
  res <- y - fit
  Sf <- sigma_of(res)
  ll <- -n / 2 * (M * (1 + log(2 * pi)) + as.numeric(determinant(Sf, logarithm = TRUE)$modulus))
  # each equation's constant: the first parameter whose derivative is the same, and not zero, on
  # every row (at the final estimates)
  const <- rep(NA_character_, M)
  if (!noconst) {
    Jf <- jac(b)
    for (q in seq_len(M)) {
      ix <- (q - 1L) * n + seq_len(n)
      for (p in eqs[[q]]$params) {
        col <- Jf[ix, match(p, pn)]
        if (is.finite(col[1]) && col[1] != 0 && max(abs(col - col[1])) <= 1e-6 * abs(col[1])) {
          const[q] <- p
          break
        }
      }
    }
  }
  stat <- lapply(seq_len(M), function(q) {
    ix <- (q - 1L) * n + seq_len(n)
    rss <- sum(res[ix]^2)
    tss <- if (is.na(const[q])) sum(y[ix]^2) else sum((y[ix] - mean(y[ix]))^2)
    mss <- tss - rss
    if (abs(mss) <= 1e-12 * tss) mss <- 0
    list(rss = rss, mss = mss, r2 = if (tss > 0) mss / tss else 0, rmse = sqrt(rss / n), k = length(eqs[[q]]$params))
  })
  if (!conv) stop("convergence not achieved (Stata: r(430)); RCraft stops here without the table, whose standard errors Stata computes in a way not reproduced for runs that have not converged", call. = FALSE)
  nm <- paste0("/", pn)
  se <- sqrt(pmax(diag(V), 0))
  if (show) {
    pad <- stata_tab_pad
    cat("\n", if (method == "nls") "Nonlinear least-squares regression " else "FGNLS regression ", "\n", sep = "")
    cat(strrep("-", 71), "\n", sep = "")
    cat("       Equation |        Obs   Parms       RMSE      R-sq     Constant\n")
    cat("----------------+------------------------------------------------------\n")
    for (q in seq_len(M)) {
      s <- stat[[q]]
      txt <- paste0(sprintf("%.4f", s$r2), if (is.na(const[q])) "*" else "")
      w <- max(12L, 19L - nchar(txt))
      cat(sprintf("%2d%13s |%11s%8d%11s    %s%s\n", q, stata_tab_abbrev(deps[q]), stata_tab_count(n), s$k,
                  trimws(stata_fmt(s$rmse, "%9.0g")), txt,
                  formatC(if (is.na(const[q])) "(none)" else const[q], width = w)))
    }
    cat(strrep("-", 71), "\n", sep = "")
    if (any(is.na(const))) cat("* Uncentered R-sq\n")
    cat("\n")
    stata_ls_table(nm, b, se, Inf, level, if (length(unique(deps)) == 1L) deps[1] else "",
                   constr = !(seq_len(k) %in% free))
  }
  e <- list(N = n)
  for (q in seq_len(M)) {
    s <- stat[[q]]
    e[[paste0("rss_", q)]] <- s$rss
    e[[paste0("mss_", q)]] <- s$mss
    e[[paste0("r2_", q)]] <- s$r2
    e[[paste0("rmse_", q)]] <- s$rmse
    e[[paste0("k_", q)]] <- s$k
  }
  e$ll <- ll
  e$n_eq <- M
  e$k <- k
  e$k_eq_model <- 0
  e$converged <- as.numeric(conv)
  e$rank <- length(free)
  e$k_eq <- M
  e$cmdline <- cmdline
  e$cmd <- "nlsur"
  e$predict <- "nlsur_p"
  e$estat_cmd <- "nlsur_estat"
  e$vce <- "gnr"
  e$method <- method
  e$constants <- if (noconst) " . ." else paste(ifelse(is.na(const), ".", const), collapse = " ")
  e$type <- "1"
  e$params <- paste(pn, collapse = " ")
  e$depvar <- paste(deps, collapse = " ")
  for (q in rev(seq_len(M))) {
    e[[paste0("params_", q)]] <- paste(eqs[[q]]$params, collapse = " ")
    e[[paste0("depvar_", q)]] <- deps[q]
  }
  for (q in rev(seq_len(M))) e[[paste0("sexp_", q)]] <- eqs[[q]]$sexp
  e$properties <- "b V"
  cn <- paste0(pn, ":_cons")
  e$b <- matrix(b, 1, k, dimnames = list("y1", cn))
  dimnames(V) <- list(cn, cn)
  e$V <- V
  e$init <- matrix(b0, 1, k, dimnames = list("r1", pn))
  SigmaOut <- if (method == "nls") diag(diag(Sigma), nrow = M) else if (method == "ifgnls") after else Sigma
  dimnames(SigmaOut) <- list(deps, deps)
  e$Sigma <- SigmaOut
  attr(e, "stata_rows") <- rows
  attr(e, "stata_b") <- stats::setNames(as.vector(b), cn)
  attr(e, "stata_V") <- V
  attr(e, "stata_df") <- Inf
  attr(e, "stata_eqs") <- eqs
  attr(e, "stata_pars") <- b
  e
}
# the fitted values (or residuals) of equation k on every row of d
stata_nlsur_pred <- function(e, d, k, resid = FALSE) {
  eqs <- attr(e, "stata_eqs")
  if (is.na(k) || k < 1L || k > length(eqs)) stop("equation number out of range (Stata: r(303))", call. = FALSE)
  num <- function(u) { u <- unclass(u); attributes(u) <- NULL; as.numeric(u) }
  v <- c(lapply(as.list(d), function(u) if (is.character(u)) u else num(u)), as.list(attr(e, "stata_pars")))
  r <- num(eqs[[k]]$f(v))
  if (length(r) == 1L) r <- rep(r, nrow(d))
  r[!is.finite(r)] <- NA_real_
  if (resid) num(d[[eqs[[k]]$dep]]) - r else r
}"##;

/// The `(...)` groups of an nlsur command, each without its parentheses.
fn equations(main: &str) -> Result<Vec<String>, Untranslated> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut cur = String::new();
    let mut seen_close = true;
    for ch in main.chars() {
        match ch {
            '(' => {
                if depth == 0 {
                    if !seen_close {
                        return Err(no("nlsur: expected an equation in parentheses"));
                    }
                    cur.clear();
                } else {
                    cur.push(ch);
                }
                depth += 1;
            }
            ')' => {
                if depth == 0 {
                    return Err(no("nlsur: an unmatched )"));
                }
                depth -= 1;
                if depth == 0 {
                    out.push(cur.clone());
                    seen_close = true;
                } else {
                    cur.push(ch);
                }
            }
            c if depth > 0 => cur.push(c),
            c if c.is_whitespace() => {}
            _ => return Err(no(
                "nlsur takes each equation in parentheses, (y1 = <expression>) (y2 = <expression>)",
            )),
        }
    }
    if depth != 0 {
        return Err(no("nlsur: an equation's parenthesis is not closed"));
    }
    Ok(out)
}

/// One parameter of an equation's expression.
struct Eq {
    dep: String,
    /// The expression as R, with the parameters as plain names.
    expr: String,
    /// The text `e(sexp_#)` holds.
    stored: String,
    params: Vec<String>,
}

impl Session {
    /// The command whose results are active, when it is `nlsur`.
    pub(super) fn nlsur_active(&self) -> bool {
        matches!(
            &self.e_ret,
            super::returned::Returned::Program { cmd, .. } if cmd == "nlsur"
        )
    }

    /// `nlsur (y1 = <sexp>) (y2 = <sexp>) ... [if] [in] [, options]`.
    pub(super) fn nlsur(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "nlsur")?;
        if let Some(o) = opts.other(&[
            ("nls", 3),
            ("fgnls", 5),
            ("ifgnls", 6),
            ("nolog", 5),
            ("level", 3),
            ("iterate", 4),
            ("eps", 3),
            ("noconstant", 3),
            ("initial", 4),
        ]) {
            return Err(no(format!(
                "nlsur with the option {o} is not translated yet"
            )));
        }
        if c.parts.weight.is_some() {
            return Err(no("nlsur with weights is not translated yet"));
        }
        let kinds_given = [
            opts.has("nls", 3),
            opts.has("fgnls", 5),
            opts.has("ifgnls", 6),
        ];
        if kinds_given.iter().filter(|b| **b).count() > 1 {
            super::sureg::eq_stop(
                out,
                "cannot specify more than one estimator (Stata: r(198))",
            );
            return Ok(());
        }
        let method = if opts.has("ifgnls", 6) {
            "ifgnls"
        } else if opts.has("nls", 3) {
            "nls"
        } else {
            "fgnls"
        };
        if method == "ifgnls" && opts.has("iterate", 4) {
            return Err(no(
                "nlsur, ifgnls with iterate() is not translated yet (Stata ends the rounds by its own rule)",
            ));
        }
        let texts = equations(c.parts.main.trim())?;
        if texts.is_empty() {
            return Err(no("nlsur needs at least one equation in parentheses"));
        }
        // the parameters in order of first appearance, with their starting values
        let mut pars: Vec<(String, f64)> = Vec::new();
        let mut eqs: Vec<Eq> = Vec::new();
        let mut parsed: Vec<(String, String, String, Vec<String>)> = Vec::new();
        for t in &texts {
            let Some((lhs, rhs)) = t.split_once('=') else {
                return Err(no("nlsur needs (depvar = <expression>)"));
            };
            let dep = lhs.trim();
            if !is_name(dep) {
                return Err(no(format!(
                    "nlsur with {dep} on the left is not translated yet (one variable name is)"
                )));
            }
            let mut text = String::new();
            let mut stored = String::new();
            let mut used: Vec<String> = Vec::new();
            let mut rest = rhs;
            while let Some(open) = rest.find('{') {
                let before = rest.get(..open).unwrap_or("");
                text.push_str(before);
                stored.push_str(before);
                let tail = rest.get(open..).unwrap_or("");
                let close = tail
                    .find('}')
                    .ok_or_else(|| no("nlsur: a { without its }"))?;
                let body = tail.get(1..close).unwrap_or("").trim();
                if body.contains(':') {
                    return Err(no(
                        "nlsur with a linear combination ({xb: x1 x2}) is not translated yet",
                    ));
                }
                let (name, init) = match body.split_once('=') {
                    Some((n, v)) => (n.trim(), Some(v.trim())),
                    None => (body, None),
                };
                if !is_name(name) {
                    return Err(no(format!("nlsur: {{{body}}} is not a parameter")));
                }
                let start = match init {
                    Some(v) => Some(v.parse::<f64>().map_err(|_| {
                        no(format!("nlsur: the starting value {v} is not a number"))
                    })?),
                    None => None,
                };
                match pars.iter_mut().find(|(n, _)| n == name) {
                    Some(p) => {
                        if let Some(s) = start {
                            p.1 = s;
                        }
                    }
                    None => pars.push((name.to_string(), start.unwrap_or(0.0))),
                }
                if !used.iter().any(|u| u == name) {
                    used.push(name.to_string());
                }
                text.push_str(name);
                stored.push_str(&format!(" {{{name}}} "));
                rest = tail.get(close + 1..).unwrap_or("");
            }
            text.push_str(rest);
            stored.push_str(rest);
            let stored = stored.split_whitespace().collect::<Vec<_>>().join(" ");
            if used.is_empty() {
                return Err(no("nlsur needs a {parameter} in every equation"));
            }
            parsed.push((dep.to_string(), text, stored, used));
        }
        // a parameter in two equations under nls: Stata's estimates are not the stacked least
        // squares then (they match each equation weighted by its residual variance), and how
        // that weighting reaches the variance is not decoded
        if method == "nls"
            && pars
                .iter()
                .any(|(n, _)| parsed.iter().filter(|p| p.3.iter().any(|u| u == n)).count() > 1)
        {
            return Err(no(
                "nlsur, nls with a parameter shared by two equations is not translated yet",
            ));
        }
        // initial(a 1 b 2): starting values beside the {name = #} ones
        if let Some(a) = opts.arg("initial", 4) {
            let words: Vec<&str> = a.split_whitespace().collect();
            if words.len() % 2 != 0 {
                return Err(no(
                    "nlsur: initial() takes pairs of a parameter and a number",
                ));
            }
            for pair in words.chunks(2) {
                let (Some(name), Some(v)) = (pair.first(), pair.get(1)) else {
                    continue;
                };
                let v: f64 = v
                    .parse()
                    .map_err(|_| no(format!("nlsur: the starting value {v} is not a number")))?;
                match pars.iter_mut().find(|(n, _)| n == name) {
                    Some(p) => p.1 = v,
                    None => {
                        return Err(no(format!(
                            "nlsur: initial() names {name}, which is not a parameter"
                        )))
                    }
                }
            }
        }
        // the parameters are numbers the expression reads, as the data's are
        let saved: Vec<(String, Option<Kind>)> = pars
            .iter()
            .map(|(n, _)| (n.clone(), self.kinds.get(n).cloned()))
            .collect();
        for (n, _) in &pars {
            self.kinds.insert(n.clone(), Kind::Number);
        }
        let mut failed = None;
        for (dep, text, stored, used) in parsed {
            match self.expr(&text, Use::Value, out) {
                Ok(expr) => eqs.push(Eq {
                    dep,
                    expr,
                    stored,
                    params: used,
                }),
                Err(e) => {
                    failed = Some(e);
                    break;
                }
            }
        }
        for (n, k) in saved {
            match k {
                Some(k) => {
                    self.kinds.insert(n, k);
                }
                None => {
                    self.kinds.remove(&n);
                }
            }
        }
        if let Some(e) = failed {
            return Err(e);
        }
        let level = super::model::level(opts)?;
        let num_opt = |name: &str, min: usize, default: f64| -> Result<f64, Untranslated> {
            match opts.arg(name, min) {
                Some(v) => v
                    .trim()
                    .parse::<f64>()
                    .map_err(|_| no(format!("nlsur, {name}({v}) needs a number"))),
                None => Ok(default),
            }
        };
        let iterate = match opts.arg("iterate", 4) {
            Some(i) => i
                .trim()
                .parse::<u32>()
                .map_err(|_| no(format!("nlsur, iterate({i}) needs a whole number")))?,
            None => 100,
        };
        let eps = num_opt("eps", 3, 1e-5)?;
        let rows = self.rows_for(c, out)?;
        let src = self.weighted_rows(rows, &None);
        for h in [
            "stata_fmt",
            "stata_tables",
            "stata_lsfam",
            "stata_eclass",
            "stata_returned",
            "stata_nlsur",
        ] {
            self.helpers.insert(h);
        }
        let eq_list = eqs
            .iter()
            .map(|q| {
                format!(
                    "list(dep = {}, f = function(.v) with(.v, {}), sexp = {}, params = c({}))",
                    r_text(&q.dep),
                    q.expr,
                    r_text(&q.stored),
                    q.params
                        .iter()
                        .map(|p| r_text(p))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        let init = pars
            .iter()
            .map(|(n, v)| format!("{} = {}", r_name(n), r_number(*v)))
            .collect::<Vec<_>>()
            .join(", ");
        let show = !self.quiet(c);
        out.line(format!(
            "stata_e <- stata_nlsur({src}, list({eq_list}), c({init}), level = {}, method = {}, nolog = {}, noconst = {}, iterate = {iterate}L, eps = {}, cmdline = {}, show = {})",
            r_number(level * 100.0),
            r_text(method),
            super::stats::r_logical(opts.has("nolog", 5)),
            super::stats::r_logical(opts.has("noconstant", 3)),
            r_number(eps),
            r_text(&super::returned::cmdline(c)),
            super::stats::r_logical(show)
        ));
        out.line("stata_b <- attr(stata_e, \"stata_b\")");
        out.line("stata_vcov <- attr(stata_e, \"stata_V\")");
        out.line("stata_df <- attr(stata_e, \"stata_df\")");
        out.line("stata_estimates <- list(rows = attr(stata_e, \"stata_rows\"))");
        out.line(self.keep_rows(&src, "stata_estimates$rows"));
        let d = self.data()?;
        out.package("dplyr");
        out.line(super::model::esample_rows_line(
            &d,
            &src,
            "stata_estimates$rows",
        ));
        self.esample_new = true;
        self.fit_robust = false;
        self.fit_cluster = false;
        self.estimates = None;
        self.estimates_lost =
            Some("test, lincom and margins are not translated after nlsur yet".into());
        self.active_store = None;
        self.e_text.clear();
        self.e_new = true;
        let mut texts: Vec<String> = [
            "cmdline",
            "cmd",
            "predict",
            "estat_cmd",
            "vce",
            "method",
            "constants",
            "type",
            "params",
            "depvar",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        for q in (1..=eqs.len()).rev() {
            texts.push(format!("params_{q}"));
            texts.push(format!("depvar_{q}"));
        }
        for q in (1..=eqs.len()).rev() {
            texts.push(format!("sexp_{q}"));
        }
        texts.push("properties".into());
        self.e_ret = super::returned::Returned::Program {
            cmd: "nlsur".into(),
            texts,
        };
        out.caveat(
            "nlsur as Stata 19 runs it: Gauss-Newton on the stacked equations with forward-difference derivatives, \
             NLS then FGNLS (Sigma from the NLS residuals), its log and stopping rule matched to its output",
        );
        Ok(())
    }

    /// `predict [type] newvar [if] [in] [, yhat residuals equation(#k)]` after `nlsur`.
    pub(super) fn nlsur_predict(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[("yhat", 1), ("residuals", 1), ("equation", 2)]) {
            return Err(no(format!(
                "predict after nlsur with the option {o} is not translated yet"
            )));
        }
        if opts.has("yhat", 1) && opts.has("residuals", 1) {
            return Err(no("predict takes one statistic at a time"));
        }
        let k = match opts.arg("equation", 2) {
            Some(e) => {
                let e = e.trim().to_string();
                match e.strip_prefix('#').and_then(|n| n.parse::<u32>().ok()) {
                    Some(k) => k,
                    None => {
                        super::sureg::eq_stop(
                            out,
                            "option equation() misspecified (Stata: r(198))",
                        );
                        return Ok(());
                    }
                }
            }
            None => 1,
        };
        let mut words: Vec<&str> = c.parts.main.split_whitespace().collect();
        let ty = match words.first() {
            Some(t) if storage(t).is_some() => {
                let t = *t;
                words.remove(0);
                t
            }
            _ => "float",
        };
        let st = storage(ty).ok_or_else(|| no(format!("{ty} is not a storage type")))?;
        if st == Storage::Str {
            return Err(no("predict makes a numeric variable, not a string"));
        }
        let [w] = words.as_slice() else {
            return Err(no("predict makes one new variable"));
        };
        if !is_name(w) {
            return Err(no(format!(
                "{w}: only a plain new variable name is translated"
            )));
        }
        let base = self.data()?;
        out.line(format!(
            "if ({} %in% names({base})) stop(\"variable {w} already defined (Stata: r(110))\", call. = FALSE)",
            r_text(w)
        ));
        let quiet = self.quiet(c);
        let resid = opts.has("residuals", 1);
        self.helpers.insert("stata_nlsur");
        self.helpers.insert("stata_float");
        if !resid && !opts.has("yhat", 1) && !quiet {
            out.line("cat(\"(option yhat assumed; fitted values)\\n\")");
        }
        let v = format!(
            "stata_nlsur_pred(stata_e, {base}, {k}L, resid = {})",
            super::stats::r_logical(resid)
        );
        let v = stored(&v, st, &mut self.helpers, out);
        let v = match self.condition(c, out)? {
            Some(cond) => format!("dplyr::if_else({cond}, {v}, NA_real_)"),
            None => v,
        };
        out.package("dplyr");
        out.line(format!(
            "{base} <- dplyr::mutate({base}, {} = {v})",
            r_name(w)
        ));
        self.kinds.insert(w.to_string(), Kind::Number);
        self.storage.insert(w.to_string(), st);
        Ok(())
    }
}
