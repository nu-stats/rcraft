//! Stata's returned values (P2B.6): `e()` after a model, `r()` after the
//! commands that leave results, `_b[]` and `_se[]`, and `display`,
//! `ereturn list` and `return list`.
//!
//! In R each translated model leaves `stata_e`, and each r-class command
//! `stata_r`: a named list holding Stata's names, numbers as numbers and
//! macros as text, with `b` and `V` as matrices. What each list holds, and
//! when, was read off Stata 19's `ereturn list` and `return list` (see the
//! P2B.6 notes). The translator keeps track of which command left them, so
//! `e(name)` and `r(name)` are refused where Stata would hold a value RCraft
//! does not keep, rather than coming out missing.

use super::{no, Out, Session};
use crate::expr_r::{r_text, Kind, Untranslated};
use crate::lang::expr::{parse_prefix, Expr};
use crate::reader::Command;

/// Reading `stata_e`, `stata_r`, `_b[]` and `_se[]` in R, and the lists.
pub(super) const RETURNED_HELPER: &str = r##"stata_eget <- function(name, text = FALSE) {
  v <- if (exists("stata_e")) stata_e[[name]] else NULL
  if (is.null(v) || is.matrix(v)) return(if (text) "" else NA_real_)
  v
}
stata_rget <- function(name, text = FALSE) {
  v <- if (exists("stata_r")) stata_r[[name]] else NULL
  if (is.null(v) || is.matrix(v)) return(if (text) "" else NA_real_)
  v
}
stata_bget <- function(key, se = FALSE) {
  st <- stata_bnames()
  nm <- names(stata_b)
  eq <- ""
  if (grepl(":", key, fixed = TRUE) && !startsWith(key, "/")) {
    eq <- sub(":.*$", "", key)
    key <- sub("^[^:]*:", "", key)
  }
  shown <- if (eq == "") key else paste0(eq, ":", key)
  if (inherits(stata_estimates, "multinom")) {
    # the equations are the outcomes in order, the base one all zeros
    if (eq == "") eq <- stata_levels[1]
    if (!eq %in% stata_levels) stop("equation [", eq, "] not found")
    k <- stata_canon(key)
    there <- k %in% sub("^[^:]*:", "", st) || (k == "_cons" && any(endsWith(nm, ":(Intercept)")))
    if (eq == as.character(stata_base)) {
      if (!there) stop("[", shown, "] not found")
      return(0)
    }
    key <- paste0(eq, ":", key)
  } else if (eq != "") {
    stop("equation [", eq, "] not found")
  }
  i <- match(stata_canon(key), st)
  if (is.na(i)) {
    # a base or omitted level of a factor variable: Stata holds a zero
    if (stata_base_level(stata_canon(key))) return(0)
    stop("[", shown, "] not found")
  }
  if (se) sqrt(stata_vcov[nm[i], nm[i]]) else unname(stata_b[i])
}
stata_base_level <- function(k) {
  xl <- stata_estimates$xlevels
  if (is.null(xl)) return(FALSE)
  vars <- sub("^stata_ib\\(([^,]+),.*$", "\\1", sub("^factor\\(([^()]+)\\)$", "\\1", names(xl)))
  parts <- strsplit(k, "#", fixed = TRUE)[[1]]
  all(vapply(parts, function(p) {
    m <- regmatches(p, regexec("^([0-9]+)\\.(.+)$", p))[[1]]
    if (length(m) == 0) return(p %in% vars || p %in% all.vars(stats::formula(stata_estimates)))
    j <- match(m[3], vars)
    !is.na(j) && m[2] %in% xl[[j]]
  }, TRUE)) && any(grepl("^[0-9]+\\.", parts))
}
stata_list_returned <- function(x, p) {
  if (is.null(x)) return(invisible(NULL))
  num <- names(x)[vapply(x, function(v) is.numeric(v) && !is.matrix(v), TRUE)]
  txt <- names(x)[vapply(x, is.character, TRUE)]
  mat <- names(x)[vapply(x, is.matrix, TRUE)]
  key <- function(n) sprintf("%22s", paste0(p, "(", n, ")"))
  if (length(num)) {
    cat("\nscalars:\n")
    for (n in num) cat(key(n), " =  ", stata_fmt(x[[n]], "%18.0g", trim = TRUE), "\n", sep = "")
  }
  if (length(txt)) {
    cat("\nmacros:\n")
    for (n in txt) cat(key(n), " : \"", x[[n]], "\"\n", sep = "")
  }
  if (length(mat)) {
    cat("\nmatrices:\n")
    for (n in mat) cat(key(n), " :  ", nrow(x[[n]]), " x ", ncol(x[[n]]), "\n", sep = "")
  }
  if (p == "e") cat("\nfunctions:\n", key("sample"), "   \n", sep = "")
  invisible(NULL)
}"##;

/// Stata's display formats, `%w.dg`, `%w.df`, `%w.de` (each with `c` for
/// commas, `-` to justify left, `0` to fill with zeros) and `%ws`, decoded in
/// Stata 19 (the P2B.6 notes have the rules), and `display` itself.
pub(super) const FORMAT_HELPER: &str = r##"stata_fmt <- function(x, fmt = "%10.0g", trim = FALSE) {
  m <- regmatches(fmt, regexec("^%(-?)(0?)([0-9]+)(\\.([0-9]+))?([gfes])(c?)$", fmt))[[1]]
  if (length(m) == 0) stop("invalid %format")
  left <- m[2] == "-"
  zero <- m[3] == "0"
  w <- as.integer(m[4])
  d <- if (m[6] == "") 0L else as.integer(m[6])
  type <- m[7]
  comma <- m[8] == "c"
  if (is.character(x) != (type == "s")) stop("type mismatch")
  commas <- function(s) {
    int <- sub("\\..*$", "", s)
    rest <- substring(s, nchar(int) + 1)
    while (grepl("[0-9]{4}", int)) int <- sub("([0-9])([0-9]{3})(,|$)", "\\1,\\2\\3", int)
    paste0(int, rest)
  }
  # g and e round an exact half away from zero (f, like C, to even): a tie
  # is a value whose digits past the last one kept are 5 and then zeros
  # (read off the 17 digits that pin a double down: R's printf past those
  # is not exact on every platform)
  rnd <- function(kind, digits, ax) {
    out <- sprintf(paste0("%.", digits, kind), ax)
    if (ax == 0) return(out)
    s17 <- sprintf("%.16e", ax)
    keep <- if (kind == "e") digits + 1 else digits + as.integer(sub("^.*e", "", s17)) + 1
    mant <- gsub(".", "", sub("e.*$", "", s17), fixed = TRUE)
    if (keep >= 1 && keep < 17 && grepl("^50*$", substring(mant, keep + 1))) {
      out <- sprintf(paste0("%.", digits, kind), ax * (1 + 2^-50))
    }
    out
  }
  e_str <- function(ax, digits) rnd("e", max(digits, 1) - 1, ax)
  exp_width <- function(ax) if (ax != 0 && abs(floor(log10(ax))) >= 100) 3 else 2
  # the sign's place: a minus, or a space before a positive number in e
  # notation even past the width, as Stata prints it
  sgn <- if (!is.character(x) && !is.na(x) && x < 0) "-" else " "
  s <- if (type == "s") {
    x
  } else if (is.na(x)) {
    "."
  } else if (x == 0 && type == "g") {
    "0"
  } else if (type == "g") {
    ax <- abs(x)
    budget <- w - 1
    ed <- max(2, w - 4 - exp_width(ax))
    if (d > 0) ed <- min(max(d, 2), ed)
    fixed <- function(ax) {
      intdig <- if (ax >= 1) nchar(sprintf("%.0f", floor(ax))) else 0
      ints <- intdig + if (comma && intdig > 3) (intdig - 1) %/% 3 else 0
      if (ints + 1 > budget) return(NULL)
      dec <- budget - ints - 1
      z <- if (ax < 1) -floor(log10(ax)) - 1 else 0
      if (d > 0) dec <- min(dec, if (ax >= 1) max(0, max(d, 2) - intdig) else z + max(d, 2))
      sig <- if (ax >= 1) intdig + dec else dec - z
      if (sig < ed) return(NULL)
      s <- rnd("f", dec, ax)
      # rounding up may have added a digit in front
      if (nchar(sub("\\..*$", "", sub("^0", "", s))) > intdig) {
        return(if (ax < 1) fixed(1) else fixed(10^intdig))
      }
      if (grepl(".", s, fixed = TRUE)) s <- sub("\\.$", "", sub("0+$", "", s))
      s <- sub("^0\\.", ".", s)
      if (comma) s <- commas(s)
      s
    }
    f <- fixed(ax)
    if (is.null(f)) paste0(sgn, e_str(ax, ed)) else if (x < 0) paste0("-", f) else f
  } else if (type == "f") {
    s <- sprintf("%.*f", d, x)
    if (comma) s <- commas(s)
    if (nchar(s) > w) {
      # too wide: e notation, unless that is wider still
      ax <- abs(x)
      e <- e_str(ax, max(2, w - 4 - exp_width(ax)))
      if (x < 0) e <- paste0("-", e)
      if (nchar(e) <= w || nchar(e) < nchar(s)) s <- if (x < 0) e else paste0(" ", e)
    }
    s
  } else {
    ax <- abs(x)
    dd <- d
    s <- rnd("e", dd, ax)
    while (nchar(s) > w - 1 && dd > 1) {
      dd <- dd - 1
      s <- rnd("e", dd, ax)
    }
    paste0(sgn, s)
  }
  if (trim) return(trimws(s))
  if (left) s <- sub("^ ", "", s)
  if (nchar(s) >= w) return(s)
  pad <- strrep(if (zero && !left && type != "s") "0" else " ", w - nchar(s))
  if (left) paste0(s, pad) else if (zero && grepl("^-", s)) paste0("-", pad, substring(s, 2)) else paste0(pad, s)
}
stata_display <- function(...) {
  out <- ""
  col <- function() nchar(sub("^.*\n", "", out))
  for (p in list(...)) {
    out <- switch(p[[1]],
      text = paste0(out, p[[2]]),
      value = paste0(out, if (p[[3]] != "") stata_fmt(p[[2]], p[[3]]) else if (is.character(p[[2]])) p[[2]] else stata_fmt(p[[2]], "%10.0g", trim = TRUE)),
      newline = paste0(out, strrep("\n", p[[2]])),
      col = paste0(out, strrep(" ", max(0, p[[2]] - 1 - col()))),
      skip = paste0(out, strrep(" ", p[[2]])),
      dup = paste0(out, strrep(p[[3]], p[[2]])))
  }
  cat(out, "\n", sep = "")
  invisible(NULL)
}"##;

/// What a model leaves in `e()`, built from what its translation left in R
/// (`stata_estimates`, `stata_b`, `stata_vcov`, `stata_df`, `stata_cluster`).
pub(super) const ERETURN_HELPER: &str = r##"stata_ereturn <- function(cmd, cmdline, depvar, vce = "oim", clustvar = NULL,
                          wtype = NULL, wexp = NULL, title = TRUE, extra = list()) {
  fit <- stata_estimates
  b <- stata_b
  V <- stata_vcov
  robust <- vce %in% c("robust", "cluster")
  G <- if (vce != "cluster") NULL else if (!is.null(fit[["G"]])) fit[["G"]] else if (cmd == "ivregress") length(unique(fit$cluster)) else if (cmd == "stcox") stata_m$G else length(unique(stata_cluster))
  wald <- function(keep) {
    bk <- b[keep]
    drop(t(bk) %*% solve(V[keep, keep, drop = FALSE], bk))
  }
  slopes <- !grepl("(Intercept)", names(b), fixed = TRUE) & !startsWith(names(b), "/")
  cons <- any(grepl("(Intercept)", names(b), fixed = TRUE))
  fw <- identical(wtype, "fweight")
  e <- list()
  put <- function(...) {
    a <- list(...)
    for (n in names(a)) e[[n]] <<- a[[n]]
  }
  weights <- function() if (!is.null(wtype)) put(wexp = wexp, wtype = wtype)
  robust_macros <- function() {
    if (robust) put(vcetype = "Robust")
    if (vce == "cluster") put(clustvar = clustvar)
  }
  # the likelihood models: the count, the log likelihoods and the model test
  ml <- function(k_eq, k_eq_model = 1, ll_0 = NULL, aux = list()) {
    mf <- stats::model.frame(fit)
    w <- stats::model.weights(mf)
    N <- if (fw && !is.null(w)) sum(w) else NROW(mf)
    l <- stata_ml_ll(fit)
    if (!is.null(ll_0)) l[["ll_0"]] <- ll_0
    q <- sum(slopes)
    chi2 <- if (robust) wald(slopes) else 2 * (l[["ll"]] - l[["ll_0"]])
    put(rank = length(b), N = N, k = length(b), k_eq = k_eq, k_dv = 1, converged = 1, rc = 0)
    if (!is.null(G)) put(N_clust = G)
    put(ll = l[["ll"]], k_eq_model = k_eq_model, ll_0 = l[["ll_0"]])
    for (n in names(aux)) e[[n]] <<- aux[[n]]
    put(df_m = q, chi2 = chi2, p = stats::pchisq(chi2, q, lower.tail = FALSE))
    list(ll = l[["ll"]], ll_0 = l[["ll_0"]])
  }
  r2_p <- function(l) put(r2_p = 1 - l$ll / l$ll_0)
  if (cmd %in% c("regress", "anova")) {
    mf <- stats::model.frame(fit)
    y <- stats::model.response(mf)
    w <- stats::model.weights(mf)
    n <- length(y)
    wn <- if (is.null(w)) rep(1, n) else if (fw) w else w * n / sum(w)
    N <- if (fw) sum(w) else n
    u <- stats::residuals(fit)
    ybar <- sum(wn * y) / sum(wn)
    rss <- sum(wn * u^2)
    tss <- if (cons) sum(wn * (y - ybar)^2) else sum(wn * y^2)
    mss <- tss - rss
    k <- fit$rank
    df_m <- k - cons
    r2 <- mss / tss
    # Stata's log likelihood of the normal model; analytic weights add half
    # the sum of the log of the weights as Stata scales them (to sum to N)
    ll <- function(ss) {
      l <- -0.5 * N * (log(2 * pi) + log(ss / N) + 1)
      if (identical(wtype, "aweight")) l + 0.5 * sum(log(wn)) else l
    }
    if (cmd == "anova") {
      put(N = N, sum_w = if (is.null(w)) N else sum(w))
    } else {
      if (!is.null(G)) put(N_clust = G)
      put(N = N, sum_w = if (is.null(w)) N else sum(w))
    }
    put(df_m = df_m, df_r = stata_df, F = wald(slopes) / df_m, r2 = r2, rmse = sqrt(rss / (N - k)),
        mss = mss, rss = rss, r2_a = 1 - (1 - r2) * (N - cons) / (N - k), ll = ll(rss),
        ll_0 = if (cons) ll(tss) else NA_real_, rank = k)
    if (cmd == "anova") {
      for (n in names(extra)) e[[n]] <- extra[[n]]
      put(cmdline = cmdline, depvar = depvar, cmd = "anova", estat_cmd = "anova_estat", properties = "b V",
          predict = "regres_p", model = "ols")
    } else {
      put(cmdline = cmdline)
      if (title) put(title = "Linear regression")
      put(marginsok = "XB default", vce = vce, depvar = depvar, cmd = "regress", properties = "b V",
          predict = "regres_p", model = "ols", estat_cmd = "regress_estat")
      weights()
      robust_macros()
    }
  } else if (cmd %in% c("logit", "probit")) {
    l <- ml(1)
    r2_p(l)
    put(cmdline = cmdline, cmd = cmd, estat_cmd = paste0(cmd, "_estat"), predict = paste0(cmd, "_p"),
        marginsok = "default Pr",
        marginsnotok = if (cmd == "logit") "stdp DBeta DEviance DX2 DDeviance Hat Number Residuals RStandard SCore" else "stdp DEviance SCore",
        title = if (cmd == "logit") "Logistic regression" else "Probit regression",
        chi2type = if (robust) "Wald" else "LR")
    weights()
    robust_macros()
    put(vce = vce, depvar = depvar, properties = "b V")
  } else if (cmd == "poisson") {
    l <- ml(1)
    r2_p(l)
    put(cmdline = cmdline, cmd = "poisson", predict = "poisso_p", estat_cmd = "poisson_estat")
    weights()
    put(chi2type = if (robust) "Wald" else "LR")
    robust_macros()
    put(vce = vce, title = "Poisson regression", depvar = depvar, properties = "b V")
  } else if (cmd == "nbreg") {
    mf <- stats::model.frame(fit)
    off <- stats::model.offset(mf)
    w <- stats::model.weights(mf)
    d0 <- data.frame(y = stats::model.response(mf), o = if (is.null(off)) 0 else off,
                     w = if (is.null(w)) 1 else w)
    f0 <- suppressWarnings(MASS::glm.nb(y ~ 1 + offset(o), data = d0, weights = w,
                                        control = stats::glm.control(epsilon = 1e-12, maxit = 100)))
    pois <- stats::glm(fit$y ~ stats::model.matrix(fit) - 1, family = stats::poisson(), offset = fit$offset,
                       weights = fit$prior.weights, control = stats::glm.control(epsilon = 1e-14, maxit = 100))
    l <- ml(2, ll_0 = as.numeric(stats::logLik(f0)), aux = list(rank0 = 2))
    put(ll_c = as.numeric(stats::logLik(pois)), alpha = exp(b[["/lnalpha"]]), k_aux = 1)
    r2_p(l)
    put(cmdline = cmdline, cmd = "nbreg", predict = "nbreg_p", dispers = "mean")
    weights()
    put(chi2type = if (robust) "Wald" else "LR")
    robust_macros()
    put(vce = vce, title = "Negative binomial regression", depvar = depvar, properties = "b V")
  } else if (cmd %in% c("ologit", "oprobit")) {
    ncut <- length(fit$zeta)
    l <- ml(1 + ncut)
    put(k_cat = ncut + 1, k_aux = ncut)
    r2_p(l)
    put(cmdline = cmdline, cmd = cmd, predict = if (cmd == "ologit") "ologit_p" else "oprobi_p",
        marginsdefault = paste0("predict(pr outcome(", fit$lev, "))", collapse = " "),
        title = if (cmd == "ologit") "Ordered logistic regression" else "Ordered probit regression")
    weights()
    put(chi2type = if (robust) "Wald" else "LR")
    robust_macros()
    put(vce = vce, depvar = depvar, properties = "b V")
  } else if (cmd == "mlogit") {
    lev <- stata_levels
    ib <- match(as.character(stata_base), lev)
    l <- ml(length(lev), k_eq_model = length(lev))
    e$k_eq <- NULL
    e$k_eq_model <- NULL
    put(ibaseout = ib, baseout = as.numeric(stata_base), k_eq = length(lev), k_eq_model = length(lev),
        k_eq_base = ib)
    r2_p(l)
    put(k_out = length(lev))
    put(cmdline = cmdline, cmd = "mlogit",
        marginsdefault = paste0("predict(pr outcome(", lev, "))", collapse = " "),
        marginsnotok = "stdp stddp SCores", predict = "mlogit_p", title = "Multinomial logistic regression",
        eqnames = paste(lev, collapse = " "))
    weights()
    put(chi2type = if (robust) "Wald" else "LR")
    robust_macros()
    put(vce = vce, depvar = depvar, properties = "b V")
  } else if (cmd == "xtreg") {
    k <- sum(slopes)
    Ti <- fit$Ti
    if (fit$type == "fe" && !robust) {
      put(F_f = fit$F_u, p_f = stats::pf(fit$F_u, fit$df_f[1], fit$df_f[2], lower.tail = FALSE))
    }
    put(N = fit$N, N_g = fit$n, g_min = min(Ti), g_avg = fit$N / fit$n, g_max = max(Ti),
        r2_w = fit$r2[["within"]], r2_b = fit$r2[["between"]], r2_o = fit$r2[["overall"]])
    if (fit$type != "be") put(sigma_u = fit$sigma_u, sigma_e = fit$sigma_e, rho = fit$rho)
    if (fit$type == "fe") put(corr = fit$corr, ll = fit$ll, ll_0 = fit$ll_0)
    if (is.finite(stata_df)) {
      F <- wald(slopes) / k
      put(df_r = stata_df, F = F, p = stats::pf(F, k, stata_df, lower.tail = FALSE))
    } else {
      chi2 <- wald(slopes)
      put(chi2 = chi2, p = stats::pchisq(chi2, k, lower.tail = FALSE))
    }
    if (!is.null(G)) put(N_clust = G)
    put(cmdline = cmdline, estat_cmd = "xtreg_estat", cmd = "xtreg")
    if (fit$type == "be") put(title = "Between regression (regression on group means)")
    put(marginsnotok = if (fit$type == "fe") "E U UE SCore STDP XBU XBD D" else "E U UE SCore STDP XBU",
        predict = if (fit$type == "re") "xtrere_p" else "xtrefe_p", model = fit$type,
        vce = if (vce == "oim") "conventional" else vce, ivar = fit$panel)
    if (fit$type == "fe") put(absvar = fit$panel)
    put(depvar = depvar)
    if (fit$type == "re") put(chi2type = "Wald")
    put(properties = "b V")
    robust_macros()
  } else if (cmd == "ivregress") {
    q <- sum(slopes)
    small <- is.finite(stata_df)
    tss <- fit$rss / (1 - fit$r2)
    put(N = fit$N, kappa = fit$kappa, mss = tss - fit$rss, rss = fit$rss, rmse = fit$rmse, rank = length(b))
    if (!is.null(G)) put(N_clust = G)
    put(df_r = if (small) stata_df else NA_real_, df_m = q, r2 = fit$r2, r2_a = fit$r2_a)
    if (small) put(F = fit$Wald / q) else put(chi2 = fit$Wald)
    put(k_endog = ncol(fit$X1))
    exog <- colnames(fit$X2)[colnames(fit$X2) != "(Intercept)"]
    put(cmdline = cmdline, cmd = "ivregress", estat_cmd = "ivregress_estat",
        vce = if (vce == "oim") "unadjusted" else vce, estimator = fit$estimator,
        exogr = paste(exog, collapse = " "), exog = paste(c(exog, colnames(fit$Zi)), collapse = " "),
        endog = paste(colnames(fit$X1), collapse = " "))
    if (small) put(small = "small")
    put(footnote = "ivreg_footnote",
        marginsnotok = if (fit$estimator == "2sls") "Residuals SCores XBD D" else "Residuals SCores",
        marginsok = "XB default", predict = "ivregress_p",
        title = paste0("Instrumental-variables ", toupper(fit$estimator), " regression"),
        properties = "b V", depvar = depvar)
    if (fit$estimator == "gmm") put(J = fit$J)
    robust_macros()
  } else if (cmd == "stcox") {
    m <- stata_m
    chi2 <- if (robust) wald(rep(TRUE, length(b))) else 2 * (m$ll - m$ll_0)
    put(N = m$N, ll_0 = m$ll_0, ll = m$ll, chi2 = chi2, r2_p = 1 - m$ll / m$ll_0, converged = 1,
        rank = length(b), df_m = length(b), N_sub = m$N, N_fail = m$N_fail, risk = m$risk)
    if (!is.null(G)) put(N_clust = G)
    put(cmdline = cmdline, cmd2 = "stcox", vce = vce, chi2type = if (robust) "Wald" else "LR",
        method = extra$ties, ties = extra$ties, t0 = "_t0", depvar = "_t", cmd = "cox", properties = "b V")
    robust_macros()
  } else if (cmd == "margins") {
    put(N = extra$N, cmd = "margins", est_cmd = extra$est_cmd, cmdline = cmdline, properties = "b V")
  }
  # Stata's columns: base levels, left-out ones, each equation's constant last
  eb <- stata_eb(b, V, cmd, depvar)
  e$b <- eb$b
  e$V <- eb$V
  e
}"##;

/// The `e()` names Stata leaves after `cmd` that RCraft does not keep,
/// and why. Anything else not in `stata_e` is missing, as in Stata.
fn e_not_kept(cmd: &str, model: &str) -> &'static [&'static str] {
    match (cmd, model) {
        ("regress", _) => &["beta", "V_modelbased"],
        ("logit" | "probit", _) => &[
            "ic",
            "N_cdf",
            "N_cds",
            "mns",
            "rules",
            "ilog",
            "gradient",
            "V_modelbased",
        ],
        ("poisson" | "nbreg", _) => &["ic", "ilog", "gradient", "V_modelbased"],
        ("ologit" | "oprobit", _) => &["ic", "N_cd", "cat", "ilog", "gradient", "V_modelbased"],
        ("mlogit", _) => &["ic", "N_cd", "out", "ilog", "gradient", "V_modelbased"],
        ("xtreg", "fe") => &[
            "Tbar",
            "Tcon",
            "k_absorb",
            "sigma",
            "rank",
            "df_m",
            "df_b",
            "df_a",
            "tss",
            "rss",
            "mss",
            "rmse",
            "r2",
            "r2_a",
            "sum_w",
            "absvar",
            "kabsorb",
            "V_modelbased",
        ],
        ("xtreg", "re") => &[
            "rank",
            "df_m",
            "sigma",
            "rmse",
            "Tbar",
            "Tcon",
            "thta_min",
            "thta_5",
            "thta_50",
            "thta_95",
            "thta_max",
            "theta",
            "VCEf",
            "bf",
            "V_modelbased",
        ],
        ("xtreg", "be") => &[
            "sum_w", "df_m", "r2", "rmse", "mss", "rss", "r2_a", "ll", "ll_0", "rank", "Tbar",
            "Tcon", "beta",
        ],
        ("ivregress", _) => &[
            "iterations",
            "J",
            "cons",
            "wmatrix",
            "W",
            "S",
            "V_modelbased",
        ],
        ("stcox", _) => &[
            "datasignature",
            "datasignaturevars",
            "strata",
            "V_modelbased",
        ],
        ("anova", _) => &["beta", "sstype", "varnames"],
        ("margins", _) => &[
            "k_at",
            "k_by",
            "k_predict",
            "k_margins",
            "title",
            "model_vce",
            "vcetype",
            "expression",
            "predict1_label",
            "xvars",
            "derivatives",
            "continuous",
            "emptycells",
            "est_cmdline",
            "_N",
            "error",
            "Jacobian",
            "chainrule",
        ],
        _ => &[],
    }
}

/// Whether `e(name)` is one Stata has after `cmd` and RCraft does not keep.
fn e_refused(cmd: &str, model: &str, name: &str) -> bool {
    if e_not_kept(cmd, model).contains(&name) {
        return true;
    }
    // anova's terms, numbered: ss_1, df_1, F_1, term_1, ...
    cmd == "anova"
        && ["ss_", "df_", "F_", "term_"].iter().any(|p| {
            name.strip_prefix(p)
                .is_some_and(|k| !k.is_empty() && k.chars().all(|c| c.is_ascii_digit()))
        })
}

/// The `e()` and `r()` names that are macros (text) in Stata.
const TEXT_NAMES: &[&str] = &[
    "cmd",
    "cmd2",
    "cmdline",
    "depvar",
    "title",
    "vce",
    "vcetype",
    "clustvar",
    "wtype",
    "wexp",
    "properties",
    "predict",
    "model",
    "estat_cmd",
    "marginsok",
    "marginsnotok",
    "marginsdefault",
    "chi2type",
    "eqnames",
    "dispers",
    "ivar",
    "tvar",
    "absvar",
    "estimator",
    "exog",
    "exogr",
    "endog",
    "small",
    "footnote",
    "method",
    "ties",
    "t0",
    "est_cmd",
    "est_cmdline",
    "opt",
    "user",
    "ml_method",
    "technique",
    "which",
    "deriv_useminbound",
    "sstype",
    "varnames",
    "mcmethod",
    "emptycells",
    "expression",
    "model_vce",
    "predict1_label",
    "xvars",
    "derivatives",
    "continuous",
    "datasignature",
    "datasignaturevars",
    "marginstok",
    "wmatrix",
];

/// The matrices among them.
const MATRIX_NAMES: &[&str] = &[
    "b",
    "V",
    "C",
    "Nobs",
    "sig",
    "beta",
    "V_modelbased",
    "mns",
    "rules",
    "ilog",
    "gradient",
    "cat",
    "out",
    "theta",
    "VCEf",
    "bf",
    "kabsorb",
    "table",
    "chainrule",
    "Jacobian",
    "error",
    "_N",
    "W",
    "S",
];

/// Who left the values `e()` or `r()` would read.
#[derive(Debug, Clone, Default)]
pub(super) enum Returned {
    /// Nothing yet in this do-file (R may hold an earlier run's values).
    #[default]
    Nothing,
    /// The command named left them, as Stata would. `model` is a detail of
    /// it (xtreg's fe/re/be) and `names` the r() names it leaves.
    Kept {
        cmd: String,
        model: String,
        names: &'static [&'static str],
    },
    /// A command since may have replaced them in Stata, so they are not known.
    Lost(String),
}

/// Commands after which Stata 19 still holds the r() it held before
/// (checked one by one); every other command replaces or clears r().
const LEAVE_R_ALONE: &[&str] = &[
    "generate",
    "replace",
    "sort",
    "gsort",
    "list",
    "label",
    "rename",
    "format",
    "order",
    "display",
    "predict",
    "save",
    "saveold",
    "use",
    "egen",
    "clear",
    "version",
    "set",
    "ereturn",
    "return",
    "notes",
    "note",
    "codebook_",
    "assert",
    "scalar",
];

/// Commands that never change e() (an untranslated command not here might
/// be an estimation command, after which e() is not known).
const LEAVE_E_ALONE: &[&str] = &[
    "use",
    "save",
    "saveold",
    "import",
    "export",
    "generate",
    "replace",
    "drop",
    "keep",
    "rename",
    "sort",
    "gsort",
    "order",
    "label",
    "count",
    "list",
    "describe",
    "set",
    "clear",
    "version",
    "recode",
    "egen",
    "merge",
    "append",
    "reshape",
    "collapse",
    "duplicates",
    "encode",
    "decode",
    "destring",
    "tostring",
    "summarize",
    "tabulate",
    "correlate",
    "tab1",
    "tab2",
    "pwcorr",
    "ttest",
    "predict",
    "test",
    "lincom",
    "estat",
    "histogram",
    "kdensity",
    "scatter",
    "line",
    "connected",
    "twoway",
    "graph",
    "lowess",
    "xtset",
    "stset",
    "display",
    "ereturn",
    "return",
    "format",
    "compress",
    "notes",
    "note",
    "assert",
    "matrix",
    "scalar",
    "tsset",
    "codebook",
    "inspect",
    "browse",
    "edit",
    "cd",
    "log",
    "capture",
    "quietly",
    "noisily",
    "help",
    "search",
    "preserve",
    "restore",
    "sample",
    "expand",
    "fillin",
    "contract",
    "xpose",
    "insheet",
    "outsheet",
    "infile",
    "export",
];

/// The r() each r-class command leaves here, as Stata 19's `return list`
/// shows them (the ones RCraft keeps).
pub(super) const R_SUMMARIZE: &[&str] = &["N", "sum_w", "mean", "Var", "sd", "min", "max", "sum"];
pub(super) const R_SUMMARIZE_DETAIL: &[&str] = &[
    "N", "sum_w", "mean", "Var", "sd", "skewness", "kurtosis", "sum", "min", "max", "p1", "p5",
    "p10", "p25", "p50", "p75", "p90", "p95", "p99",
];
pub(super) const R_MEANONLY: &[&str] = &["N", "sum_w", "mean", "min", "max", "sum"];
pub(super) const R_COUNT: &[&str] = &["N"];
pub(super) const R_TEST: &[&str] = &["drop", "df_r", "F", "df", "p", "chi2"];
pub(super) const R_LINCOM: &[&str] = &["df", "ub", "lb", "p", "t", "z", "level", "se", "estimate"];
pub(super) const R_TAB1: &[&str] = &["N", "r"];
pub(super) const R_TAB2: &[&str] = &["N", "r", "c", "chi2", "p"];
pub(super) const R_CORRELATE: &[&str] = &["sum_w", "N", "rho", "cov_12", "Var_2", "Var_1"];
pub(super) const R_DESCRIBE: &[&str] = &["k", "N"];
pub(super) const R_TTEST: &[&str] = &[
    "N_1",
    "N_2",
    "mu_1",
    "mu_2",
    "sd_1",
    "sd_2",
    "se_1",
    "se_2",
    "lb_1",
    "ub_1",
    "lb_2",
    "ub_2",
    "N_combined",
    "mu_combined",
    "se_combined",
    "lb_combined",
    "ub_combined",
    "mu_diff",
    "sd_diff",
    "lb_diff",
    "ub_diff",
    "t",
    "df_t",
    "p",
    "p_l",
    "p_u",
    "se",
    "sd",
    "level",
];
pub(super) const R_MARGINS: &[&str] = &["N", "level", "cmd", "est_cmd", "est_cmdline"];

/// The r() Stata leaves that R has nothing to give for, and why.
fn r_not_kept(cmd: &str, name: &str) -> Option<&'static str> {
    match (cmd, name) {
        ("describe", "width") => Some(
            "it is the bytes a row takes in Stata's storage types, which R's columns do not have",
        ),
        ("describe", "changed") => Some(
            "it says whether the data changed since they were last saved, which R does not follow",
        ),
        _ => None,
    }
}

impl Session {
    /// After each command: who now holds e() and r().
    pub(super) fn track_returned(
        &mut self,
        c: &Command,
        ok: bool,
        r_left: Option<(&'static [&'static str], String)>,
    ) {
        let name = c.name.as_str();
        match r_left {
            Some((names, cmd)) if ok => {
                self.r_ret = Returned::Kept {
                    cmd,
                    model: String::new(),
                    names,
                }
            }
            _ if LEAVE_R_ALONE.contains(&name) => {}
            // matrix list (the one matrix command translated) leaves it too
            _ if ok && name == "matrix" => {}
            _ => {
                self.r_ret = Returned::Lost(if ok {
                    format!("{name} replaces r() in Stata with results RCraft does not keep yet")
                } else {
                    format!("{name}, which was not translated, replaces r() in Stata")
                })
            }
        }
        if !ok && !LEAVE_E_ALONE.contains(&name) {
            self.e_ret = Returned::Lost(format!(
                "{name} was not translated, and if it is an estimation command it replaced e() in Stata"
            ));
        }
    }

    /// `e(name)`, `r(name)`, `_b[key]` or `_se[key]` in an expression.
    pub(super) fn returned_value(
        &self,
        which: &str,
        name: &str,
    ) -> Result<(String, Kind), Untranslated> {
        match which {
            "_b" | "_coef" | "_se" => {
                match &self.e_ret {
                    Returned::Kept { .. } => {}
                    Returned::Nothing => {
                        return Err(no(format!(
                            "{which}[{name}]: no model is fitted before it in this do-file (Stata: _b not allowed when e(b) is not present)"
                        )))
                    }
                    Returned::Lost(why) => return Err(no(format!("{which}[{name}] is not known here: {why}"))),
                }
                Ok((
                    format!(
                        "stata_bget({}{})",
                        r_text(name.trim_start_matches(':')),
                        if which == "_se" { ", se = TRUE" } else { "" }
                    ),
                    Kind::Number,
                ))
            }
            "e" | "r" => {
                let state = if which == "e" {
                    &self.e_ret
                } else {
                    &self.r_ret
                };
                let shown = format!("{which}({name})");
                match state {
                    Returned::Nothing => Err(no(format!(
                        "{shown}: nothing before it in this do-file leaves {which}() (R might hold an earlier run's)"
                    ))),
                    Returned::Lost(why) => Err(no(format!("{shown} is not known here: {why}"))),
                    Returned::Kept { cmd, model, names } => {
                        if MATRIX_NAMES.contains(&name) {
                            return Err(no(format!(
                                "{shown} is a matrix, which an expression cannot take (Stata: type mismatch); matrix list {shown} shows it"
                            )));
                        }
                        if which == "e" && e_refused(cmd, model, name) {
                            return Err(no(format!(
                                "{shown}, which Stata leaves after {cmd}, is not kept in R yet"
                            )));
                        }
                        if which == "r" {
                            if let Some(why) = r_not_kept(cmd, name) {
                                return Err(no(format!("{shown} after {cmd} is not kept in R: {why}")));
                            }
                        }
                        if which == "r" && !names.contains(&name) {
                            return Err(no(format!(
                                "{shown} after {cmd} is not kept in R yet (RCraft keeps r() {})",
                                names.join(", ")
                            )));
                        }
                        let text = TEXT_NAMES.contains(&name);
                        let get = if which == "e" { "stata_eget" } else { "stata_rget" };
                        Ok((
                            format!(
                                "{get}({}{})",
                                r_text(name),
                                if text { ", text = TRUE" } else { "" }
                            ),
                            if text { Kind::Text } else { Kind::Number },
                        ))
                    }
                }
            }
            "scalar" => self.scalar_value(name, false),
            // a bare name: a scalar only if it is one (else the variable)
            "scalar?" => self.scalar_value(name, true),
            _ => Err(no(format!("{which}() is not translated"))),
        }
    }

    /// The R of a matrix a command left, for `matrix list`.
    pub(super) fn returned_matrix(&self, which: &str, name: &str) -> Result<String, Untranslated> {
        let shown = format!("{which}({name})");
        let state = if which == "e" {
            &self.e_ret
        } else {
            &self.r_ret
        };
        match state {
            Returned::Nothing => Err(no(format!(
                "matrix list {shown}: nothing before it in this do-file leaves {which}()"
            ))),
            Returned::Lost(why) => Err(no(format!("matrix list {shown}: {why}"))),
            Returned::Kept { cmd, model, .. } => {
                if which == "e" {
                    if matches!(cmd.as_str(), "anova" | "margins") {
                        return Err(no(format!(
                            "matrix list {shown} after {cmd} is not translated yet: Stata's columns there are not rebuilt in R"
                        )));
                    }
                    if e_refused(cmd, model, name) {
                        return Err(no(format!(
                            "{shown}, which Stata leaves after {cmd}, is not kept in R yet"
                        )));
                    }
                    Ok(format!("stata_e[[{}]]", r_text(name)))
                } else {
                    Ok(format!("stata_r[[{}]]", r_text(name)))
                }
            }
        }
    }
    /// The line that keeps a model's e() in R, after its translation.
    pub(super) fn keep_e(&mut self, c: &Command, out: &mut Out, args: &str, model: &str) {
        self.helpers.insert("stata_ereturn");
        self.helpers.insert("stata_eb");
        self.helpers.insert("stata_returned");
        self.helpers.insert("stata_names");
        self.helpers.insert("stata_ml_header");
        let cmdline = cmdline(c);
        out.line(format!(
            "stata_e <- stata_ereturn({}, {}{args})",
            r_text(&c.name),
            r_text(&cmdline)
        ));
        self.e_new = true;
        self.e_ret = Returned::Kept {
            cmd: c.name.clone(),
            model: model.to_string(),
            names: &[],
        };
    }

    /// `display`: text, expressions, formats and the column directives.
    pub(super) fn display(&mut self, c: &Command, out: &mut Out) -> Result<(), Untranslated> {
        let rest = after_name(c);
        let chars: Vec<char> = rest.chars().collect();
        let mut i = 0;
        let mut pieces: Vec<String> = Vec::new();
        let mut fmt: Option<String> = None;
        let at = |k: usize| chars.get(k).copied();
        while i < chars.len() {
            let Some(&ch) = chars.get(i) else { break };
            if ch.is_whitespace() {
                i += 1;
                continue;
            }
            // a comma prints a space
            if ch == ',' {
                pieces.push("list(\"text\", \" \")".into());
                i += 1;
                continue;
            }
            if ch == '"' || (ch == '`' && at(i + 1) == Some('"')) {
                let (text, end) =
                    quoted(&chars, i).ok_or_else(|| no("display: a quote is not closed"))?;
                match fmt.take() {
                    Some(f) => pieces.push(format!(
                        "list(\"value\", {}, {})",
                        r_text(&text),
                        r_text(&f)
                    )),
                    None => pieces.push(format!("list(\"text\", {})", r_text(&text))),
                }
                i = end;
                continue;
            }
            if ch == '%' {
                let end = (i..chars.len())
                    .find(|&k| at(k).is_some_and(char::is_whitespace))
                    .unwrap_or(chars.len());
                let f = text_of(&chars, i, end);
                if !valid_format(&f) {
                    return Err(no(format!("display: the format {f} is not translated (Stata: invalid %format, or a format RCraft does not have yet)")));
                }
                fmt = Some(f);
                i = end;
                continue;
            }
            // directives: _n, _newline(#), _col(#), _skip(#), _dup(#), _continue,
            // and the colours (as text/result/error/input, in color)
            let word: String = tail(&chars, i)
                .iter()
                .take_while(|c| c.is_alphanumeric() || **c == '_')
                .collect();
            let arg = |from: usize| -> Option<(i64, usize)> {
                if at(from) != Some('(') {
                    return None;
                }
                let close = (from..chars.len()).find(|&k| at(k) == Some(')'))?;
                let inner = text_of(&chars, from + 1, close);
                Some((inner.trim().parse().ok()?, close + 1))
            };
            let directive = |names: &[(&str, usize)]| {
                names
                    .iter()
                    .any(|(n, min)| word.len() >= *min && n.starts_with(word.as_str()))
            };
            if word.starts_with('_') && word.len() > 1 {
                let j = i + word.len();
                if directive(&[("_newline", 2)]) {
                    let (k, end) = arg(j).unwrap_or((1, j));
                    pieces.push(format!("list(\"newline\", {k})"));
                    i = end;
                    continue;
                }
                if directive(&[("_column", 2)]) {
                    let (k, end) = arg(j).ok_or_else(|| no("display: _column() needs a number"))?;
                    pieces.push(format!("list(\"col\", {k})"));
                    i = end;
                    continue;
                }
                if directive(&[("_skip", 2)]) {
                    let (k, end) = arg(j).unwrap_or((1, j));
                    pieces.push(format!("list(\"skip\", {k})"));
                    i = end;
                    continue;
                }
                if directive(&[("_duplicate", 2)]) {
                    let (k, end) = arg(j).ok_or_else(|| no("display: _dup() needs a number"))?;
                    let rest = tail(&chars, end);
                    let skip = rest.iter().take_while(|c| c.is_whitespace()).count();
                    let (text, used) = quoted(rest, skip).ok_or_else(|| {
                        no("display: _dup() is translated before quoted text only")
                    })?;
                    pieces.push(format!("list(\"dup\", {k}, {})", r_text(&text)));
                    i = end + used;
                    continue;
                }
                if directive(&[("_continue", 2), ("_request", 2), ("_char", 2)]) {
                    return Err(no(format!("display: {word} is not translated yet")));
                }
            }
            if matches!(word.as_str(), "as" | "in") {
                // colours: R prints plain text
                let after = text_of(&chars, i + 2, chars.len());
                let next = after.trim_start();
                let colour: String = next.chars().take_while(|c| c.is_alphabetic()).collect();
                if word == "as"
                    && !["text", "txt", "result", "error", "err", "input", "inp"]
                        .contains(&colour.as_str())
                {
                    return Err(no(format!("display: as {colour} is not a style Stata has")));
                }
                i = i + 2 + (after.len() - next.len()) + colour.len();
                out.caveat("as/in colours are left out: R prints plain text");
                continue;
            }
            // an expression, as far as it goes
            let text = text_of(&chars, i, chars.len());
            let (_, used) = parse_prefix(&text)
                .map_err(|e| no(format!("display: Stata would not read it: {}", e.message)))?;
            let used_chars = used;
            let e = text_of(&chars, i, i + used_chars);
            let (code, kind) = self.display_value(e.trim(), out)?;
            let f = fmt.take().unwrap_or_default();
            if kind == Kind::Text && !f.is_empty() && !f.ends_with('s') {
                return Err(no(format!(
                    "display: {f} is a number format, given text (Stata: type mismatch)"
                )));
            }
            pieces.push(format!("list(\"value\", {code}, {})", r_text(&f)));
            i += used_chars;
        }
        self.helpers.insert("stata_fmt");
        out.line(format!("stata_display({})", pieces.join(", ")));
        Ok(())
    }

    /// One expression of `display`: a variable stands for its first row, as
    /// in Stata, so it is worked out over the data when there is any.
    pub(super) fn display_value(
        &self,
        e: &str,
        out: &mut Out,
    ) -> Result<(String, Kind), Untranslated> {
        let (code, kind) = self.value(e, out)?;
        let scalar = |n: &str| self.scalars.contains_key(n) && self.kind_of(n).is_none();
        let data_free = !mentions_data(e, &scalar);
        Ok(match (&self.data, data_free) {
            (Some(d), false) => {
                out.package("dplyr");
                (
                    format!("dplyr::transmute({d}, stata_v = {code})$stata_v[1]"),
                    kind,
                )
            }
            _ => (code, kind),
        })
    }

    /// `ereturn list` and `return list`.
    pub(super) fn list_returned(&mut self, c: &Command, out: &mut Out) -> Result<(), Untranslated> {
        let which = if c.name == "ereturn" { "e" } else { "r" };
        let sub = c.parts.main.trim();
        if !matches!(sub, "list" | "li" | "lis") {
            return Err(no(format!("{} {sub} is not translated yet", c.name)));
        }
        let state = if which == "e" {
            &self.e_ret
        } else {
            &self.r_ret
        };
        match state {
            Returned::Nothing => {
                return Err(no(format!(
                    "{} list: nothing before it in this do-file leaves {which}()",
                    c.name
                )))
            }
            Returned::Lost(why) => return Err(no(format!("{} list: {why}", c.name))),
            Returned::Kept { cmd, model, .. } => {
                if which == "e" {
                    let missing = e_not_kept(cmd, model);
                    if !missing.is_empty() {
                        out.caveat(format!(
                            "Stata also lists {} after {cmd}, which RCraft does not keep; and the optimizer's settings (e(opt), e(user), e(technique), ...)",
                            missing.join(", ")
                        ));
                    }
                } else if cmd == "describe" {
                    out.caveat(
                        "Stata also lists r(changed) and r(width) after describe, which RCraft does not keep",
                    );
                }
            }
        }
        self.helpers.insert("stata_returned");
        self.helpers.insert("stata_fmt");
        out.line(format!(
            "stata_list_returned({}, \"{which}\")",
            if which == "e" { "stata_e" } else { "stata_r" }
        ));
        Ok(())
    }
}

/// The command line as Stata keeps it in `e(cmdline)`: the line without
/// its prefixes.
pub(super) fn cmdline(c: &Command) -> String {
    let rest = after_name(c);
    let rest = rest.trim();
    if rest.is_empty() {
        c.written.clone()
    } else {
        format!("{} {rest}", c.written)
    }
}

/// The text of the line after the command word.
pub(super) fn after_name(c: &Command) -> String {
    let text = c.text.as_str();
    let mut from = 0;
    // past the prefixes: the command word is the first one that is it
    for (k, _) in text.match_indices(c.written.as_str()) {
        let before_ok = k == 0
            || text[..k]
                .chars()
                .last()
                .is_some_and(|ch| ch.is_whitespace() || ch == ':');
        let after = text[k + c.written.len()..].chars().next();
        let after_ok = after.is_none_or(|ch| !(ch.is_alphanumeric() || ch == '_'));
        if before_ok && after_ok {
            from = k + c.written.len();
            break;
        }
    }
    text[from..].to_string()
}

/// The characters from `from` to `to`, as text (none past the end).
fn text_of(chars: &[char], from: usize, to: usize) -> String {
    chars
        .get(from..to.min(chars.len()))
        .map(|s| s.iter().collect())
        .unwrap_or_default()
}

/// The characters from `from` on.
fn tail(chars: &[char], from: usize) -> &[char] {
    chars.get(from..).unwrap_or(&[])
}

/// A quoted string from `start` (`"..."` or `` `"..."' ``): its text and where
/// it ends.
fn quoted(chars: &[char], start: usize) -> Option<(String, usize)> {
    if chars.get(start) == Some(&'"') {
        let end = (start + 1..chars.len()).find(|&k| chars.get(k) == Some(&'"'))?;
        return Some((text_of(chars, start + 1, end), end + 1));
    }
    if chars.get(start) == Some(&'`') && chars.get(start + 1) == Some(&'"') {
        let end = (start + 2..chars.len())
            .find(|&k| chars.get(k) == Some(&'"') && chars.get(k + 1) == Some(&'\''))?;
        return Some((text_of(chars, start + 2, end), end + 2));
    }
    None
}

/// The display formats `stata_fmt` has.
pub(super) fn valid_format(f: &str) -> bool {
    let b = f.as_bytes();
    if b.first() != Some(&b'%') {
        return false;
    }
    let mut k = 1;
    if b.get(k) == Some(&b'-') {
        k += 1;
    }
    if b.get(k) == Some(&b'0') {
        k += 1;
    }
    let w0 = k;
    while b.get(k).is_some_and(u8::is_ascii_digit) {
        k += 1;
    }
    if k == w0 {
        return false;
    }
    if b.get(k) == Some(&b'.') {
        k += 1;
        let d0 = k;
        while b.get(k).is_some_and(u8::is_ascii_digit) {
            k += 1;
        }
        if k == d0 {
            return false;
        }
    }
    match b.get(k) {
        Some(b'g' | b'f' | b'e') => {
            k += 1;
            if b.get(k) == Some(&b'c') {
                k += 1;
            }
        }
        Some(b's') => k += 1,
        _ => return false,
    }
    k == b.len()
}

/// Whether an expression reads the data: a name other than the ones that
/// stand for something else (`_n`, `_N`, `_pi`, functions).
fn mentions_data(e: &str, scalar: &dyn Fn(&str) -> bool) -> bool {
    fn walk(x: &Expr, scalar: &dyn Fn(&str) -> bool) -> bool {
        match x {
            Expr::Name(n) => !matches!(n.as_str(), "_pi" | "c") && !scalar(n),
            Expr::Subscript(n, i) => {
                !matches!(n.as_str(), "_b" | "_se" | "_coef") || walk(i, scalar)
            }
            Expr::Call(name, args) => {
                !matches!(name.as_str(), "e" | "r" | "scalar")
                    && args.iter().any(|a| walk(a, scalar))
            }
            Expr::Unary(_, a) => walk(a, scalar),
            Expr::Binary(_, a, b) => walk(a, scalar) || walk(b, scalar),
            _ => false,
        }
    }
    // what does not parse is left to the full translation to refuse
    match crate::lang::expr::parse(e) {
        Ok(t) => walk(&t, scalar),
        Err(_) => true,
    }
}

/// `ttest`: Stata's r() of each of its four tests, in the order of its `return list`.
pub(super) const TTEST_R_HELPER: &str = r#"stata_ttest_r <- function(x, g = NULL, y = NULL, mu = NULL, paired = FALSE, level = 95, equal = TRUE) {
  # Stata's r() of ttest: one sample (mu), paired, two samples (y), or by(g)
  num <- function(v) { u <- unclass(v); attributes(u) <- NULL; as.numeric(u) }
  x <- num(x)
  q <- (1 + level / 100) / 2
  one <- function(v) {
    n <- length(v)
    se <- stats::sd(v) / sqrt(n)
    h <- stats::qt(q, n - 1) * se
    list(N = n, mu = mean(v), sd = stats::sd(v), se = se, lb = mean(v) - h, ub = mean(v) + h)
  }
  tests <- function(t, df) {
    list(p_u = stats::pt(t, df, lower.tail = FALSE), p_l = stats::pt(t, df), p = 2 * stats::pt(-abs(t), df),
         t = t, df_t = df)
  }
  if (!is.null(mu)) {
    a <- one(x[!is.na(x)])
    return(c(list(ub_1 = a$ub, lb_1 = a$lb, level = level, sd_1 = a$sd, se = a$se),
             tests((a$mu - mu) / a$se, a$N - 1), list(mu_1 = a$mu, N_1 = a$N)))
  }
  if (paired) {
    y <- num(y)
    ok <- !is.na(x) & !is.na(y)
    a <- one(x[ok])
    b <- one(y[ok])
    d <- one(x[ok] - y[ok])
    return(c(list(ub_diff = d$ub, lb_diff = d$lb, sd_diff = d$sd, mu_diff = d$mu, ub_2 = b$ub, lb_2 = b$lb,
                  se_2 = b$se, ub_1 = a$ub, lb_1 = a$lb, se_1 = a$se, level = level, sd_2 = b$sd,
                  sd_1 = a$sd, se = d$se),
             tests(d$mu / d$se, d$N - 1), list(mu_2 = b$mu, N_2 = b$N, mu_1 = a$mu, N_1 = a$N)))
  }
  if (is.null(g)) {
    y <- num(y)
    v1 <- x[!is.na(x)]
    v2 <- y[!is.na(y)]
  } else {
    g <- num(g)
    ok <- !is.na(x) & !is.na(g)
    x <- x[ok]
    g <- g[ok]
    lv <- sort(unique(g))
    if (length(lv) != 2) stop("more than 2 groups found, only 2 allowed")
    v1 <- x[g == lv[1]]
    v2 <- x[g == lv[2]]
  }
  a <- one(v1)
  b <- one(v2)
  all <- one(c(v1, v2))
  diff <- a$mu - b$mu
  if (equal) {
    df <- a$N + b$N - 2
    se <- sqrt(((a$N - 1) * a$sd^2 + (b$N - 1) * b$sd^2) / df * (1 / a$N + 1 / b$N))
  } else {
    s1 <- a$sd^2 / a$N
    s2 <- b$sd^2 / b$N
    se <- sqrt(s1 + s2)
    df <- (s1 + s2)^2 / (s1^2 / (a$N - 1) + s2^2 / (b$N - 1))
  }
  h <- stats::qt(q, df) * se
  c(list(ub_diff = diff + h, lb_diff = diff - h, mu_diff = diff,
         ub_combined = all$ub, lb_combined = all$lb, se_combined = all$se, mu_combined = all$mu,
         N_combined = all$N, ub_2 = b$ub, lb_2 = b$lb, se_2 = b$se, ub_1 = a$ub, lb_1 = a$lb, se_1 = a$se,
         level = level, sd = all$sd, sd_2 = b$sd, sd_1 = a$sd, se = se),
    tests(diff / se, df), list(mu_2 = b$mu, N_2 = b$N, mu_1 = a$mu, N_1 = a$N))
}"#;

/// The arguments of `stata_ereturn` every model passes: its outcome, what
/// Stata calls its variance (`ols`, `oim`, `robust`, `cluster`, ...), the
/// cluster variable and the weight.
pub(super) fn e_args(
    depvar: &str,
    vce: &str,
    cluster: Option<&str>,
    w: &Option<(super::weights::W, String)>,
) -> String {
    use super::weights::W;
    let mut s = format!(", depvar = {}, vce = {}", r_text(depvar), r_text(vce));
    if let Some(v) = cluster {
        s.push_str(&format!(", clustvar = {}", r_text(v)));
    }
    if let Some((k, v)) = w {
        let word = match k {
            W::F => "fweight",
            W::A => "aweight",
            W::P => "pweight",
            W::I => "iweight",
        };
        s.push_str(&format!(
            ", wtype = {}, wexp = {}",
            r_text(word),
            r_text(&format!("= {v}"))
        ));
    }
    s
}

/// A level as a fraction (0.95) as Stata's r(level) holds it (95).
pub(super) fn percent(level: f64) -> f64 {
    (level * 100.0 * 1e9).round() / 1e9
}

/// A call that prints, run as `quietly` asks: nothing shown, but what it
/// leaves (r()) still left.
pub(super) fn quietly(c: &Command, call: String) -> String {
    if c.prefixes.contains(&crate::reader::Prefix::Quietly) {
        format!("invisible(utils::capture.output({call}))")
    } else {
        call
    }
}
