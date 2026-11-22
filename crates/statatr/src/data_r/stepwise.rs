//! `stepwise [, pr(#) pe(#) forward hierarchical lockterm1 lr]: command depvar terms`
//! for regress, logit, probit and poisson with plain variables, as Stata 19 runs it
//! (observed on made-up data; notes in `plan/stata/stepwise.md`).
//!
//! The search runs in R, where the data are: it fits the models, tests each term
//! (Wald, or likelihood ratio with `lr`), prints Stata's trace and hands back the
//! formula of the terms it kept. The command is then translated as usual, with that
//! formula where the right-hand side would be, on the rows the full model uses.
//!
//! Not translated: weights, factor variables and operators, options of the command
//! other than level() and (regress) beta, other options of stepwise. A search that
//! would add a term back after removing one (or remove one after adding) stops with
//! a note when it runs: Stata's order for that is not decoded.

use super::weights::W;
use super::*;
use crate::reader::Prefix;

pub(super) const STEPWISE_HELPER: &str = r##"stata_sw_rows <- function(d, v) d[stats::complete.cases(d[v]), , drop = FALSE]
stata_stepwise <- function(d, kind, lhs, dep, terms, pr = NULL, pe = NULL, forward = FALSE, hier = FALSE,
                           lock1 = FALSE, lr = FALSE, show = TRUE, wvar = NULL, wkind = "", robust = FALSE,
                           clvar = NULL, noconst = FALSE, ll = NULL, ul = NULL) {
  refuse <- function(...) stop("stepwise: ", ..., "; not translated", call. = FALSE)
  rawv <- function(x) { v <- unclass(x); attributes(v) <- NULL; v }
  bq <- function(v) if (make.names(v) == v) v else paste0("`", v, "`")
  lvl <- function(l) sprintf("%.0f", l)
  words <- unlist(lapply(terms, function(t) t$words), recursive = FALSE)
  vars <- unique(c(dep, unlist(lapply(words, function(w) c(w[["v"]], w[["a"]], w[["b"]])))))
  for (v in c(vars, wvar, clvar)) if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
  # the rows the full model uses stay the sample through the search and the final fit
  dd <- stata_sw_rows(d, vars)
  if (!is.null(wvar)) dd <- stata_wrows(dd, wvar, wkind)
  if (!nrow(dd)) stop("no observations (Stata: r(2000))", call. = FALSE)
  n <- nrow(dd)
  y <- as.numeric(rawv(eval(str2lang(lhs), dd, globalenv())))
  w <- if (is.null(wvar)) NULL else as.numeric(rawv(dd[[wvar]]))
  w1 <- if (is.null(w)) rep(1, n) else w
  wk <- if (is.null(wvar)) "a" else wkind
  rob <- robust || wkind == "p"
  cl <- if (is.null(clvar)) NULL else rawv(dd[[clvar]])
  if (!is.null(cl) && anyNA(cl)) stop("the cluster variable is missing in rows the model uses; Stata would leave those rows out", call. = FALSE)
  num <- function(v) {
    x <- rawv(dd[[v]])
    if (!is.numeric(x) && !is.logical(x)) refuse("variable ", v, " is not a number")
    as.numeric(x)
  }
  # every term is a column or a group of columns: a plain variable, a continuous interaction, one
  # indicator for each level of a factor but its base (Stata makes each its own term)
  cn <- character(0); cx <- list(); cw <- integer(0); crt <- character(0); notes <- character(0); fac <- list()
  add <- function(name, x, wi, rt) {
    cn <<- c(cn, name); cx[[length(cx) + 1L]] <<- x; cw <<- c(cw, wi); crt <<- c(crt, rt)
  }
  for (wi in seq_along(words)) {
    wd <- words[[wi]]
    if (wd$kind == "p") {
      add(wd$v, num(wd$v), wi, bq(wd$v))
    } else if (wd$kind == "x") {
      a <- num(wd$a); b <- num(wd$b)
      if (wd$full) { add(wd$a, a, wi, bq(wd$a)); add(wd$b, b, wi, bq(wd$b)) }
      add(paste0("c.", wd$a, "#c.", wd$b), a * b, wi, paste0(bq(wd$a), ":", bq(wd$b)))
    } else {
      x <- num(wd$v)
      if (any(x < 0 | x != floor(x))) refuse("the factor variable ", wd$v, " has values that are not whole numbers from 0 up")
      u <- sort(unique(x))
      base <- if (nzchar(wd$base)) as.numeric(wd$base) else u[1]
      if (!base %in% u) refuse("the base level ", wd$base, " of ", wd$v, " is not in the data")
      for (l in u[u != base]) add(paste0(lvl(l), ".", wd$v), as.numeric(x == l), wi, "")
      notes <- c(notes, paste0(lvl(base), "b.", wd$v))
      fac[[as.character(wi)]] <- list(v = wd$v, base = lvl(base))
    }
  }
  tcols <- list(); tlab <- character(0); wi <- 0L
  for (t in terms) {
    idx <- integer(0)
    for (wd in t$words) { wi <- wi + 1L; idx <- c(idx, which(cw == wi)) }
    if (!length(idx)) next
    if (t$group) {
      tcols[[length(tcols) + 1L]] <- idx; tlab <- c(tlab, paste(cn[idx], collapse = " "))
    } else for (i in idx) {
      tcols[[length(tcols) + 1L]] <- i; tlab <- c(tlab, cn[i])
    }
  }
  nt <- length(tcols)
  if (!nt) refuse("no term has a column to test")
  Xall <- do.call(cbind, cx)
  cons <- as.integer(!noconst)
  fam <- switch(kind, logit = stats::binomial("logit"), probit = stats::binomial("probit"), poisson = stats::poisson(), NULL)
  if (kind == "tobit") {
    lim <- function(v, extreme) if (is.null(v)) NULL else if (isTRUE(v)) extreme else v
    lo_l <- lim(ll, min(y)); hi_l <- lim(ul, max(y))
    if (!is.null(lo_l) && !is.null(hi_l) && lo_l > hi_l) stop("observations with ll() > ul() not allowed (Stata: r(198))", call. = FALSE)
    tlo <- y; thi <- y
    for (i in seq_len(n)) {
      if (!is.null(lo_l) && y[i] <= lo_l) { tlo[i] <- -Inf; thi[i] <- lo_l }
      else if (!is.null(hi_l) && y[i] >= hi_l) { tlo[i] <- hi_l; thi[i] <- Inf }
    }
    tmid <- ifelse(is.infinite(tlo), thi, ifelse(is.infinite(thi), tlo, y))
  }
  ctl <- stats::glm.control(epsilon = 1e-12, maxit = 100)
  # the log likelihood of a model with the linear predictor eta (what a likelihood-ratio test compares)
  ll_eta <- function(eta) {
    mu <- fam$linkinv(eta)
    xlogy <- function(a, b) ifelse(a == 0, 0, a * log(b))
    switch(kind,
      poisson = sum(w1 * (xlogy(y, mu) - mu - lgamma(y + 1))),
      sum(w1 * (xlogy(y, mu) + xlogy(1 - y, 1 - mu))))
  }
  ll_rss <- function(rss) {
    neff <- switch(wk, f = sum(w1), i = floor(sum(w1)), n)
    -neff / 2 * log(rss / neff)
  }
  fit_of <- function(idx) {
    ci <- unlist(tcols[idx])
    X1 <- cbind(if (cons) matrix(1, n, 1, dimnames = list(NULL, "(Intercept)")), Xall[, ci, drop = FALSE])
    k <- ncol(X1)
    if (k == 0) {
      return(list(k = 0L, ci = ci, ll = if (kind == "regress") ll_rss(sum(w1 * y^2)) else ll_eta(rep(0, n))))
    }
    if (kind == "tobit") {
      # tobit (censoring limits ll and ul of the search's rows): the slopes first, the constant and sigma^2 last
      Xs <- Xall[, ci, drop = FALSE]
      f <- stata_cx_nfit(tlo, thi, tmid, Xs, w, wk == "f", "tobit", 300)
      kx <- ncol(Xs)
      V <- stata_cx_vcov(f, rob, w, wk == "f", cl)
      ord <- c(kx + 1, seq_len(kx))
      neff <- if (wk == "f") sum(w1) else n
      return(list(k = k, ci = ci, b = f$b[ord], V = V[ord, ord, drop = FALSE], dfr = neff - kx, ll = f$ll))
    }
    df <- data.frame(yv = y)
    df$X <- X1
    if (kind == "regress") {
      fit <- do.call(stats::lm, list(formula = yv ~ 0 + X, data = df, weights = w1))
      b <- stats::coef(fit)
      if (anyNA(b)) refuse("the terms are collinear, where Stata leaves variables out with a note")
      V <- stata_lm_wvcov(fit, wk, robust = rob, cluster = cl)
      dfr <- attr(V, "df"); attr(V, "df") <- NULL
      return(list(k = k, ci = ci, b = unname(b), V = unname(V), dfr = dfr, ll = ll_rss(sum(w1 * stats::residuals(fit)^2))))
    }
    fit <- suppressWarnings(do.call(stats::glm, list(formula = yv ~ 0 + X, family = fam, data = df, weights = w, control = ctl)))
    b <- stats::coef(fit)
    if (anyNA(b)) refuse("the terms are collinear, where Stata leaves variables out with a note")
    # probit: Stata's own steps, and the variance from the observed information, as its test uses
    if (kind == "probit") fit <- suppressWarnings(stata_probit_newton(fit))
    b <- stats::coef(fit)
    V <- stata_glm_vcov(fit, robust = rob, cluster = cl, kind = if (is.null(wvar)) "" else wkind)
    list(k = k, ci = ci, b = unname(b), V = unname(V), ll = ll_eta(drop(X1 %*% b)))
  }
  # the test that a term's coefficients are 0 in the fit m, or the likelihood ratio against s
  p_wald <- function(m, t) {
    pos <- match(tcols[[t]], m$ci) + cons
    b <- m$b[pos]
    W <- drop(crossprod(b, solve(m$V[pos, pos, drop = FALSE], b)))
    q <- length(pos)
    if (kind %in% c("regress", "tobit")) stats::pf(W / q, q, m$dfr, lower.tail = FALSE) else stats::pchisq(W, q, lower.tail = FALSE)
  }
  p_lr <- function(m, s) stats::pchisq(max(0, 2 * (m$ll - s$ll)), m$k - s$k, lower.tail = FALSE)
  p_remove <- function(inc, t) {
    m <- fit_of(inc)
    if (lr) p_lr(m, fit_of(setdiff(inc, t))) else p_wald(m, t)
  }
  p_add <- function(inc, t) {
    m <- fit_of(c(inc, t))
    if (lr) p_lr(m, fit_of(inc)) else p_wald(m, t)
  }
  # Stata fits the full model first, and leaves out for the whole search a term it cannot estimate
  # ("omitted because of estimability"), by a rule not matched: a collinear full model is refused
  fit_of(seq_len(nt))
  lock <- if (lock1) 1L else integer(0)
  inc <- if (forward) lock else seq_len(nt)
  # Stata takes a term out by putting the last one in its place
  drop_term <- function(inc, t) {
    p <- match(t, inc)
    if (p < length(inc)) inc[p] <- inc[length(inc)]
    inc[-length(inc)]
  }
  say <- function(p, op, thr, verb, t) if (show) cat(sprintf("p = %6.4f %s %6.4f, %s %s\n", p, op, thr, verb, tlab[t]))
  if (show) {
    for (nb in notes) cat("note: ", nb, " omitted because of estimability.\n", sep = "")
    cat("\n", if (lr) "LR" else "Wald", " test, begin with ",
        if (!forward) "full" else if (lock1) "term 1" else "empty", " model:\n", sep = "")
  }
  # both pr() and pe(): which of a removal and an addition comes first is not decoded, so a search
  # where the other kind of step is ever possible is refused
  guard_add <- function(inc) for (t in setdiff(seq_len(nt), inc)) if (p_add(inc, t) < pe)
    refuse("pe() would add a term back after pr() removed one, in an order not matched yet")
  guard_remove <- function(inc) for (t in setdiff(inc, lock)) if (p_remove(inc, t) >= pr)
    refuse("pr() would remove a term after pe() added one, in an order not matched yet")
  changed <- FALSE
  if (!forward && !hier) {
    repeat {
      cand <- setdiff(inc, lock)
      if (!length(cand)) break
      ps <- vapply(cand, function(t) p_remove(inc, t), 0)
      k <- which.max(ps)
      if (ps[k] < pr) break
      say(ps[k], ">=", pr, "removing", cand[k])
      inc <- drop_term(inc, cand[k]); changed <- TRUE
      if (!is.null(pe)) guard_add(inc)
    }
  } else if (!forward) {
    repeat {
      cand <- setdiff(inc, lock)
      if (!length(cand)) break
      t <- cand[length(cand)]; p <- p_remove(inc, t)
      if (p < pr) { say(p, "< ", pr, "keeping", t); break }
      say(p, ">=", pr, "removing", t)
      inc <- drop_term(inc, t); changed <- TRUE
    }
  } else if (!hier) {
    repeat {
      cand <- setdiff(seq_len(nt), inc)
      if (!length(cand)) break
      ps <- vapply(cand, function(t) p_add(inc, t), 0)
      k <- which.min(ps)
      if (ps[k] >= pe) break
      say(ps[k], "< ", pe, "adding", cand[k])
      inc <- c(inc, cand[k]); changed <- TRUE
      if (!is.null(pr)) guard_remove(inc)
    }
  } else {
    repeat {
      cand <- setdiff(seq_len(nt), inc)
      if (!length(cand)) break
      t <- min(cand); p <- p_add(inc, t)
      if (p >= pe) { say(p, ">=", pe, "testing", t); break }
      say(p, "< ", pe, "adding", t)
      inc <- c(inc, t); changed <- TRUE
    }
  }
  if (!changed && show) cat(sprintf("p %s %6.4f for all terms in model\n", if (forward) ">=" else "<", if (forward) pe else pr))
  # the terms kept; a model of the constant alone is printed differently from Stata's by the model
  # printers (F(0, n), LR chi2(0)), so that is refused
  if (!length(inc)) refuse("no term is kept, and the printing of a model of the constant alone is not matched")
  # the final model lists the terms in the order the search left them; a factor's kept levels
  # stand together where its first one is
  kc <- unlist(tcols[inc])
  rhs <- character(0); done <- integer(0)
  for (c1 in kc) {
    wi <- cw[c1]
    f <- fac[[as.character(wi)]]
    if (is.null(f)) {
      rhs <- c(rhs, crt[c1])
    } else if (!wi %in% done) {
      done <- c(done, wi)
      lv <- sub("\\..*$", "", cn[kc[cw[kc] == wi]])
      lv <- lv[order(as.numeric(lv))]
      rhs <- c(rhs, sprintf("stata_ib(%s, \"%s=%s\")", bq(f$v), f$base, paste(lv, collapse = ",")))
    }
  }
  stats::as.formula(paste(lhs, "~", paste(c(rhs, if (noconst) "0"), collapse = " + ")), env = globalenv())
}
"##;

/// What a stepwise search or a nestreg run leaves for the translation of its models.
#[derive(Debug, Clone)]
pub(super) struct Sw {
    /// The outcome and every variable of the full model: the rows with none missing.
    pub(super) vars: Vec<String>,
    /// e(cmdline): for stepwise `stepwise ` and the line after its word, as typed; for
    /// nestreg the model's own command.
    pub(super) cmdline: String,
    /// Whether the fit is a stepwise one, which also leaves e(stepwise).
    pub(super) mark: bool,
}

/// The terms of a stepwise command: a word, or a parenthesised group of words.
pub(super) fn read_terms(text: &str) -> Option<Vec<Vec<String>>> {
    let mut terms = Vec::new();
    let mut rest = text.trim();
    while !rest.is_empty() {
        if let Some(inner) = rest.strip_prefix('(') {
            let (group, after) = inner.split_once(')')?;
            let names: Vec<String> = group.split_whitespace().map(str::to_string).collect();
            if names.is_empty() || !names.iter().all(|n| is_name(n)) {
                return None;
            }
            terms.push(names);
            rest = after.trim_start();
        } else {
            let end = rest
                .find(|c: char| c.is_whitespace() || c == '(')
                .unwrap_or(rest.len());
            let (word, after) = rest.split_at(end);
            if !is_name(word) {
                return None;
            }
            terms.push(vec![word.to_string()]);
            rest = after.trim_start();
        }
    }
    Some(terms)
}

/// e(cmdline) of a stepwise line: the word and what follows it, as typed.
fn stepwise_cmdline(c: &Command) -> String {
    let text = c.text.as_str();
    for (k, _) in text.match_indices("stepwise") {
        let before_ok = k == 0
            || text
                .get(..k)
                .and_then(|t| t.chars().last())
                .is_some_and(|ch| ch.is_whitespace() || ch == ':');
        let after = text.get(k + "stepwise".len()..).unwrap_or("");
        let after_ok = after
            .chars()
            .next()
            .is_none_or(|ch| !(ch.is_alphanumeric() || ch == '_'));
        if before_ok && after_ok {
            return format!("stepwise {}", after.trim());
        }
    }
    super::returned::cmdline(c)
}

/// A word of a stepwise term, as far as it is translated.
#[derive(Debug, Clone)]
enum Word {
    /// `x` or `c.x`.
    Plain(String),
    /// `i.g` or `ib3.g`: each level but the base a term of its own.
    Factor { var: String, base: Option<String> },
    /// `c.a#c.b`, or `c.a##c.b` with the two variables too.
    Cross { a: String, b: String, full: bool },
}

impl Word {
    fn vars(&self) -> Vec<&str> {
        match self {
            Word::Plain(v) | Word::Factor { var: v, .. } => vec![v.as_str()],
            Word::Cross { a, b, .. } => vec![a.as_str(), b.as_str()],
        }
    }

    /// The word as Stata writes it.
    fn text(&self) -> String {
        match self {
            Word::Plain(v) => v.clone(),
            Word::Factor { var, base: None } => format!("i.{var}"),
            Word::Factor { var, base: Some(b) } => format!("ib{b}.{var}"),
            Word::Cross { a, b, full } => {
                format!("c.{a}{}c.{b}", if *full { "##" } else { "#" })
            }
        }
    }

    /// The word for the R search.
    fn r_code(&self) -> String {
        match self {
            Word::Plain(v) => format!("list(kind = \"p\", v = {})", r_text(v)),
            Word::Factor { var, base } => format!(
                "list(kind = \"f\", v = {}, base = {})",
                r_text(var),
                r_text(base.as_deref().unwrap_or(""))
            ),
            Word::Cross { a, b, full } => format!(
                "list(kind = \"x\", a = {}, b = {}, full = {})",
                r_text(a),
                r_text(b),
                super::stats::r_logical(*full)
            ),
        }
    }
}

/// A term of the command: a word (each column of it is tested alone), or a
/// parenthesised group of words (tested together).
struct Term {
    group: bool,
    words: Vec<Word>,
}

fn parse_word(w: &str) -> Result<Word, Untranslated> {
    let refuse = || no(format!("stepwise with the term {w} is not translated"));
    let cont = |p: &str| -> Option<String> {
        let n = p.strip_prefix("c.")?;
        is_name(n).then(|| n.to_string())
    };
    if w.contains('#') {
        let full = w.contains("##");
        let parts: Vec<&str> = if full {
            w.split("##").collect()
        } else {
            w.split('#').collect()
        };
        if let [a, b] = parts.as_slice() {
            if let (Some(a), Some(b)) = (cont(a), cont(b)) {
                if a != b {
                    return Ok(Word::Cross { a, b, full });
                }
            }
        }
        return Err(refuse());
    }
    if let Some(v) = w.strip_prefix("i.") {
        return if is_name(v) {
            Ok(Word::Factor {
                var: v.to_string(),
                base: None,
            })
        } else {
            Err(refuse())
        };
    }
    if let Some((b, v)) = w.strip_prefix("ib").and_then(|r| r.split_once('.')) {
        if !b.is_empty() && b.chars().all(|c| c.is_ascii_digit()) && is_name(v) {
            return Ok(Word::Factor {
                var: v.to_string(),
                base: Some(b.to_string()),
            });
        }
        return Err(refuse());
    }
    if let Some(n) = cont(w) {
        return Ok(Word::Plain(n));
    }
    if is_name(w) {
        return Ok(Word::Plain(w.to_string()));
    }
    Err(refuse())
}

/// The terms of a stepwise command: words, and parenthesised groups of words.
fn read_sw_terms(text: &str) -> Result<Vec<Term>, Untranslated> {
    let mut terms = Vec::new();
    let mut rest = text.trim();
    while !rest.is_empty() {
        if let Some(inner) = rest.strip_prefix('(') {
            let Some((group, after)) = inner.split_once(')') else {
                return Err(no("stepwise: a parenthesis is not closed"));
            };
            let words = group
                .split_whitespace()
                .map(parse_word)
                .collect::<Result<Vec<_>, _>>()?;
            if words.is_empty() {
                return Err(no("stepwise: a group of terms is empty"));
            }
            terms.push(Term { group: true, words });
            rest = after.trim_start();
        } else {
            let end = rest
                .find(|c: char| c.is_whitespace() || c == '(')
                .unwrap_or(rest.len());
            let (word, after) = rest.split_at(end);
            terms.push(Term {
                group: false,
                words: vec![parse_word(word)?],
            });
            rest = after.trim_start();
        }
    }
    Ok(terms)
}

impl Session {
    /// The model's formula: the one the search made, or `lhs ~ right`.
    pub(super) fn model_formula(&self, lhs: &str, right: &str) -> String {
        if self.sw.is_some() {
            "stata_sw_formula".to_string()
        } else {
            format!("{lhs} ~ {right}")
        }
    }

    /// The rows of a model under stepwise: those the full model uses.
    pub(super) fn sw_rows(&mut self, src: String) -> String {
        match &self.sw {
            None => src,
            Some(sw) => {
                self.helpers.insert("stata_stepwise");
                let vars: Vec<String> = sw.vars.iter().map(|v| r_text(v)).collect();
                format!("stata_sw_rows({src}, c({}))", vars.join(", "))
            }
        }
    }

    /// e(cmdline) for a model under stepwise or nestreg, and whether e(stepwise) goes with it.
    pub(super) fn sw_cmdline(&self) -> Option<(String, bool)> {
        self.sw.as_ref().map(|s| (s.cmdline.clone(), s.mark))
    }

    fn sw_fail(&self, out: &mut Out, msg: &str, rc: u32) -> Result<(), Untranslated> {
        out.line(format!(
            "stop({}, call. = FALSE)",
            r_text(&format!("{msg} (Stata: r({rc}))"))
        ));
        Ok(())
    }

    /// `stepwise [, options]: regress|logit|probit|poisson depvar terms [if] [in] [, options]`.
    #[allow(clippy::too_many_lines)]
    pub(super) fn stepwise(&mut self, c: &Command, out: &mut Out) -> Result<(), Untranslated> {
        let text = c
            .prefixes
            .iter()
            .find_map(|p| match p {
                Prefix::Stepwise(o) => Some(o.clone()),
                _ => None,
            })
            .unwrap_or_default();
        let kind = c.name.as_str();
        if !matches!(kind, "regress" | "logit" | "probit" | "poisson" | "tobit") {
            return Err(no(format!("stepwise with {kind} is not translated")));
        }
        self.no_by(c, "stepwise")?;
        let given = crate::lang::syntax::options(&text);
        let so = Opts(&given);
        if given.iter().any(|o| o.name == "b") {
            return self.sw_fail(out, "option b not allowed", 198);
        }
        // options of the command, put before the colon
        for (full, min, said) in [
            ("vce", 3, "option vce() not allowed"),
            ("robust", 1, "option robust not allowed"),
            ("noconstant", 5, "option noconstant not allowed"),
            ("beta", 4, "option beta not allowed"),
        ] {
            if so.has(full, min) {
                return self.sw_fail(out, said, 198);
            }
        }
        if let Some(o) = so.other(&[
            ("pr", 2),
            ("pe", 2),
            ("forward", 7),
            ("hierarchical", 12),
            ("lockterm1", 9),
            ("lr", 2),
            ("level", 3),
        ]) {
            return Err(no(format!(
                "stepwise with the option {o} is not translated"
            )));
        }
        let opts = Opts(&c.options);
        let known: &[(&str, usize)] = if kind == "tobit" {
            &[("level", 3), ("ll", 2), ("ul", 2)]
        } else if kind == "regress" {
            &[
                ("level", 3),
                ("beta", 4),
                ("noconstant", 5),
                ("robust", 1),
                ("vce", 3),
            ]
        } else {
            &[("level", 3), ("noconstant", 5), ("robust", 1), ("vce", 3)]
        };
        if let Some(o) = opts.other(known) {
            return Err(no(format!(
                "stepwise: {kind} with the option {o} is not translated"
            )));
        }
        // the weight: regress takes all four kinds, the likelihood models all but analytic
        let w = self.weight_of(c, &[W::F, W::A, W::P, W::I])?;
        if kind != "regress" && matches!(w, Some((W::A, _))) {
            return self.sw_fail(out, "aweights not allowed", 101);
        }
        if kind == "tobit" && matches!(w, Some((W::A | W::P | W::I, _))) {
            return Err(no(
                "stepwise with tobit is translated without weights or with frequency weights only",
            ));
        }
        // the variance: Stata's own errors for a kind the command lacks, the rest as the command
        if let Some(v) = opts.arg("vce", 3) {
            let word = v.split_whitespace().next().unwrap_or("");
            let lacks = if kind == "regress" {
                matches!(word, "oim" | "opg")
            } else {
                matches!(word, "ols" | "hc2" | "hc3")
            };
            if lacks {
                return self.sw_fail(out, &format!("vcetype {word} not allowed"), 198);
            }
        }
        let (robust, cluster) = super::model::vce(&opts)?;
        let noconstant = opts.has("noconstant", 5);
        // level() belongs to stepwise: the command's own is not what the final table shows
        let sw_level = so.arg("level", 3);
        if let Some(l) = &sw_level {
            if opts.has("level", 3) {
                return self.sw_fail(
                    out,
                    &format!("option level() of stepwise conflicts with option level() of {kind}"),
                    198,
                );
            }
            match l.trim().parse::<f64>() {
                Err(_) => {
                    return self.sw_fail(
                        out,
                        &format!("{} found where number expected", l.trim()),
                        198,
                    )
                }
                Ok(x) if !(10.0..=99.99).contains(&x) => {
                    return self.sw_fail(out, "level() must be between 10 and 99.99 inclusive", 198)
                }
                Ok(_) => {}
            }
        }
        // the terms
        let main = c.parts.main.trim();
        let (dep, rest) = match main.split_once(char::is_whitespace) {
            Some((d, r)) => (d.trim(), r.trim()),
            None => (main, ""),
        };
        if !is_name(dep) {
            return Err(no(
                "stepwise needs an outcome variable and terms of plain names",
            ));
        }
        let terms = read_sw_terms(rest)?;
        let words: Vec<&Word> = terms.iter().flat_map(|t| t.words.iter()).collect();
        let repeats = {
            let mut seen = std::collections::HashSet::new();
            let mut vars: Vec<&str> = vec![dep];
            for w in &words {
                vars.extend(w.vars());
            }
            vars.iter().any(|v| !seen.insert(*v))
        };
        if repeats {
            return Err(no(
                "stepwise with a variable listed twice is not translated (Stata's notes differ)",
            ));
        }
        let factors = words.iter().any(|w| matches!(w, Word::Factor { .. }));
        if factors && noconstant {
            return Err(no(
                "stepwise with noconstant and factor variables is not translated yet",
            ));
        }
        // Stata's own checks, in the order the messages were found
        let pr_text = so.arg("pr", 2);
        let pe_text = so.arg("pe", 2);
        let forward_opt = so.has("forward", 7);
        let hier = so.has("hierarchical", 12);
        let lock1 = so.has("lockterm1", 9);
        let lr = so.has("lr", 2);
        let number = |t: &Option<String>| -> Result<Option<f64>, Untranslated> {
            match t {
                None => Ok(None),
                Some(s) => s
                    .trim()
                    .parse::<f64>()
                    .map(Some)
                    .map_err(|_| no(format!("stepwise, the number {s} is not translated"))),
            }
        };
        let (pr, pe) = (number(&pr_text)?, number(&pe_text)?);
        if pr.is_none() && pe.is_none() {
            return self.sw_fail(out, "at least one of pr(#) or pe(#) must be specified", 198);
        }
        if forward_opt && pe.is_none() {
            return self.sw_fail(out, "option forward invalid or option pe() is missing", 198);
        }
        for (name, v) in [("pr", pr), ("pe", pe)] {
            if v.is_some_and(|x| !(x > 0.0 && x < 1.0)) {
                return self.sw_fail(
                    out,
                    &format!("{name}() invalid -- invalid number, outside of allowed range"),
                    125,
                );
            }
        }
        if hier && pr.is_some() && pe.is_some() {
            return self.sw_fail(out, "hierarchical pe(#) pr(#) invalid", 198);
        }
        if let (Some(r), Some(e), Some(rt), Some(et)) = (pr, pe, &pr_text, &pe_text) {
            if r <= e {
                return self.sw_fail(
                    out,
                    &format!("pr({}) <= pe({}) invalid", rt.trim(), et.trim()),
                    198,
                );
            }
        }
        if terms.is_empty() {
            return self.sw_fail(out, "no independent variables", 198);
        }
        // lockterm1 keeps the first term whole, which a factor variable must say with parentheses
        if lock1 {
            if let Some(first) = terms.first() {
                let has_factor = first.words.iter().any(|w| matches!(w, Word::Factor { .. }));
                if has_factor && !first.group {
                    return self.sw_fail(
                        out,
                        "option lockterm1 requires that the first term be bound in parentheses if it contains a factor variable",
                        198,
                    );
                }
                if has_factor {
                    return Err(no(
                        "stepwise, lockterm1 with a factor variable in the first term is not translated yet",
                    ));
                }
            }
        }
        // likelihood-ratio tests need the plain likelihood
        if lr && (matches!(w, Some((W::P, _))) || (robust && cluster.is_none())) {
            return self.sw_fail(
                out,
                "option lr not allowed with pweights, robust VCE, or clustered VCE",
                198,
            );
        }
        // forward with pe() alone; backward with pr() (alone or with pe())
        let forward = forward_opt || (pe.is_some() && pr.is_none());
        let src = self.rows_for(c, out)?;
        let limits = if kind == "tobit" {
            format!(
                ", ll = {}, ul = {}",
                super::censored::limit_arg(&opts, "ll")?,
                super::censored::limit_arg(&opts, "ul")?
            )
        } else {
            String::new()
        };
        let dep_r = r_name(dep);
        let lhs = if matches!(kind, "logit" | "probit") {
            format!("as.numeric({dep_r} != 0)")
        } else {
            dep_r
        };
        let list: Vec<String> = terms
            .iter()
            .map(|t| {
                format!(
                    "list(group = {}, words = list({}))",
                    super::stats::r_logical(t.group),
                    t.words
                        .iter()
                        .map(Word::r_code)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })
            .collect();
        let num = |v: Option<f64>| v.map_or("NULL".to_string(), r_number);
        let (wvar, wkind) = match &w {
            Some((k, v)) => (r_text(v), r_text(k.code())),
            None => ("NULL".to_string(), "\"\"".to_string()),
        };
        self.helpers.insert("stata_stepwise");
        if w.is_some() {
            self.helpers.insert("stata_wrows");
        }
        if kind == "tobit" {
            for h in ["stata_cx", "stata_tobit", "stata_fmt"] {
                self.helpers.insert(h);
            }
        } else if kind == "regress" {
            self.helpers.insert("stata_lm_wvcov");
        } else {
            self.helpers.insert("stata_glm_vcov");
        }
        if kind == "probit" {
            self.helpers.insert("stata_probit_newton");
        }
        if factors {
            self.helpers.insert("stata_ib");
        }
        out.line(format!(
            "stata_sw_formula <- stata_stepwise({src}, {}, {}, {}, list({}), pr = {}, pe = {}, forward = {}, hier = {}, lock1 = {}, lr = {}, show = {}, wvar = {wvar}, wkind = {wkind}, robust = {}, clvar = {}, noconst = {}{limits})",
            r_text(kind),
            r_text(&lhs),
            r_text(dep),
            list.join(", "),
            num(pr),
            num(pe),
            super::stats::r_logical(forward),
            super::stats::r_logical(hier),
            super::stats::r_logical(lock1),
            super::stats::r_logical(lr),
            super::stats::r_logical(!self.quiet(c)),
            super::stats::r_logical(robust),
            cluster.as_deref().map_or("NULL".to_string(), r_text),
            super::stats::r_logical(noconstant),
        ));
        // the command itself, on the plain list of the variables
        let mut inner = c.clone();
        inner
            .prefixes
            .retain(|p| !matches!(p, Prefix::Stepwise(_) | Prefix::Nestreg(_)));
        // Stata prints the final table at stepwise's level(), 95 when there is none
        inner.options.retain(|o| !o.is("level", 3));
        if let Some(o) = given.iter().find(|o| o.is("level", 3)) {
            inner.options.push(o.clone());
        }
        inner.parts.main = format!(
            "{dep} {}",
            words.iter().map(|w| w.text()).collect::<Vec<_>>().join(" ")
        );
        let mut vars = vec![dep.to_string()];
        vars.extend(words.iter().flat_map(|w| w.vars()).map(str::to_string));
        self.sw = Some(Sw {
            vars,
            cmdline: stepwise_cmdline(c),
            mark: true,
        });
        let done = self.dispatch(&inner, out);
        self.sw = None;
        if factors {
            // the model keeps only some levels of a factor: its terms are not `i.g`'s
            if let Some(e) = self.estimates.as_mut() {
                e.factors.clear();
            }
            out.caveat(
                "stepwise: each level of a factor variable is a term of its own, as in Stata; \
                 the final model keeps the levels that stayed, the others pooled with the base",
            );
        }
        out.caveat(
            "stepwise: the search fits the models in R and tests each term as Stata's does (Wald, \
             or likelihood ratio with lr); the final model is fitted on the rows the full model uses",
        );
        done
    }
}

#[cfg(test)]
mod tests {
    use crate::data_r::translate;
    use crate::reader::read;

    fn r(source: &str) -> Vec<String> {
        translate(&read(source))
            .lines
            .into_iter()
            .map(|l| match l.result {
                Ok(r) => r.code,
                Err(e) => format!("NOT: {}", e.reason),
            })
            .collect()
    }

    #[test]
    fn a_search_makes_the_formula_the_model_uses() {
        let out = r("use d
stepwise, pr(.2): regress y x1 x2 (x3 x4)
stepwise, pe(.1) lr: logit z x1 x2");
        assert!(
            out[1].contains("stata_sw_formula <- stata_stepwise(")
                && out[1].contains("list(group = FALSE, words = list(list(kind = \"p\", v = \"x1\")))")
                && out[1].contains("list(group = TRUE, words = list(list(kind = \"p\", v = \"x3\"), list(kind = \"p\", v = \"x4\")))")
                && out[1].contains("pr = 0.2, pe = NULL, forward = FALSE")
                && out[1].contains("stats::lm(stata_sw_formula, data = stata_sw_rows("),
            "{out:?}"
        );
        assert!(
            out[1].contains("\"stepwise , pr(.2): regress y x1 x2 (x3 x4)\""),
            "{out:?}"
        );
        assert!(
            out[2].contains("forward = TRUE")
                && out[2].contains("lr = TRUE")
                && out[2].contains("as.numeric(z != 0)")
                && out[2].contains("stats::glm(stata_sw_formula,"),
            "{out:?}"
        );
    }

    #[test]
    fn statas_errors_are_kept_and_the_rest_refused() {
        let out = r("use d
stepwise: regress y x1 x2
stepwise, pr(.1) pe(.2): regress y x1 x2
stepwise, pr(.2) forward: regress y x1 x2
stepwise, pr(1.5): regress y x1 x2
stepwise, pr(.2) b: regress y x1 x2
stepwise, pe(.1) pr(.2) hierarchical: regress y x1 x2
stepwise, pr(.2): regress y
stepwise, pr(.2): summarize y x1
stepwise, pr(.2): regress y i.g#c.x1 x2
stepwise, pr(.2): regress y ibn.g x1
stepwise, pr(.2): regress y i.g x1, noconstant
stepwise, pr(.2): regress y x1 x1
stepwise, pr(.2): regress y x1 x2, vce(bootstrap)
stepwise, pr(.2) lockterm1: regress y (i.g x1) x2");
        let want = [
            (1, "at least one of pr(#) or pe(#) must be specified", "198"),
            (2, "pr(.1) <= pe(.2) invalid", "198"),
            (3, "option forward invalid or option pe() is missing", "198"),
            (
                4,
                "pr() invalid -- invalid number, outside of allowed range",
                "125",
            ),
            (5, "option b not allowed", "198"),
            (6, "hierarchical pe(#) pr(#) invalid", "198"),
            (7, "no independent variables", "198"),
        ];
        for (i, msg, rc) in want {
            assert!(
                out[i].contains(msg) && out[i].contains(&format!("(Stata: r({rc}))")),
                "{i}: {out:?}"
            );
        }
        for i in 8..=14 {
            assert!(out[i].starts_with("NOT: "), "{i}: {out:?}");
        }
    }

    #[test]
    fn weights_vce_and_noconstant_go_to_the_search_and_the_command() {
        let out = r("use d
stepwise, pr(.2): regress y x1 x2 [aweight = w]
stepwise, pe(.2) lr: logit z x1 x2 [fweight = n]
stepwise, pr(.2): regress y x1 x2, vce(cluster g)
stepwise, pr(.2): poisson c x1 x2, robust noconstant
stepwise, pr(.2) level(90): regress y x1 x2");
        assert!(
            out[1].contains("wvar = \"w\", wkind = \"a\"")
                && out[1].contains("stata_lm_wvcov(")
                && out[1].contains("weights = w"),
            "{out:?}"
        );
        assert!(
            out[2].contains("wvar = \"n\", wkind = \"f\"") && out[2].contains("lr = TRUE"),
            "{out:?}"
        );
        assert!(out[3].contains("clvar = \"g\""), "{out:?}");
        assert!(
            out[4].contains("robust = TRUE") && out[4].contains("noconst = TRUE"),
            "{out:?}"
        );
        // stepwise's level() is the final table's
        assert!(out[5].contains("level = 0.9"), "{out:?}");
    }

    #[test]
    fn statas_errors_for_weights_vce_level_and_lockterm1() {
        let out = r("use d
stepwise, pr(.2) lr: regress y x1 x2 [pweight = w]
stepwise, pr(.2) lr: regress y x1 x2, robust
stepwise, pr(.2) lr: logit z x1 x2, vce(robust)
stepwise, pr(.2): logit z x1 x2 [aweight = w]
stepwise, pr(.2): logit z x1 x2, vce(ols)
stepwise, pr(.2): regress y x1 x2, vce(oim)
stepwise, pr(.2) level(90): regress y x1 x2, level(80)
stepwise, pr(.2) level(190): regress y x1 x2
stepwise, pr(.2) lockterm1: regress y i.g x1
stepwise, pr(.2) vce(robust): regress y x1 x2
stepwise, pr(.2) robust: regress y x1 x2
stepwise, pr(.2) noconstant: regress y x1 x2
stepwise, pr(.2) beta: regress y x1 x2");
        let want = [
            (10, "option vce() not allowed", "198"),
            (11, "option robust not allowed", "198"),
            (12, "option noconstant not allowed", "198"),
            (13, "option beta not allowed", "198"),
            (
                1,
                "option lr not allowed with pweights, robust VCE, or clustered VCE",
                "198",
            ),
            (
                2,
                "option lr not allowed with pweights, robust VCE, or clustered VCE",
                "198",
            ),
            (
                3,
                "option lr not allowed with pweights, robust VCE, or clustered VCE",
                "198",
            ),
            (4, "aweights not allowed", "101"),
            (5, "vcetype ols not allowed", "198"),
            (6, "vcetype oim not allowed", "198"),
            (
                7,
                "option level() of stepwise conflicts with option level() of regress",
                "198",
            ),
            (8, "level() must be between 10 and 99.99 inclusive", "198"),
            (
                9,
                "option lockterm1 requires that the first term be bound in parentheses if it contains a factor variable",
                "198",
            ),
        ];
        for (i, msg, rc) in want {
            assert!(
                out[i].contains(msg) && out[i].contains(&format!("(Stata: r({rc}))")),
                "{i}: {out:?}"
            );
        }
    }

    #[test]
    fn factor_variables_and_interactions_are_words_of_the_search() {
        let out = r("use d
stepwise, pr(.2): regress y i.g ib3.h c.x1##c.x2 x3
stepwise, pe(.2): logit z (i.g x1) c.x2#c.x3");
        assert!(
            out[1].contains("list(kind = \"f\", v = \"g\", base = \"\")")
                && out[1].contains("list(kind = \"f\", v = \"h\", base = \"3\")")
                && out[1].contains("list(kind = \"x\", a = \"x1\", b = \"x2\", full = TRUE)"),
            "{out:?}"
        );
        assert!(
            out[2].contains("group = TRUE")
                && out[2].contains("list(kind = \"x\", a = \"x2\", b = \"x3\", full = FALSE)"),
            "{out:?}"
        );
    }
}
