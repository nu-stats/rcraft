//! Statistics menu, Bayesian regression (P6.9): a Gaussian, logit, Poisson,
//! negative binomial or Gamma regression fitted by Hamiltonian Monte Carlo with
//! `rstanarm::stan_glm`, or an ordered logit with `rstanarm::stan_polr`, with
//! rstanarm's default weakly informative priors or a normal, Student t or
//! Cauchy prior on the coefficients (one for all, or one for each) and on the
//! intercept. It keeps the fit in the workspace under the name the dialog asks
//! for and prints each parameter's posterior mean, standard deviation, credible
//! interval, Monte Carlo standard error, effective sample size and R-hat. It
//! does not change the data. The Objects pane's Model menu then offers, on the
//! fit (P6.9b), posterior predictions, trace plots and posterior predictive
//! checks, and a comparison of models by leave-one-out cross-validation.

use rcraft_rcodegen::dialog::{Args, Opt, Problem, Spec, TextDefault, TextRule};
use rcraft_rcodegen::dialog::{VarKind, Widget};
use rcraft_rcodegen::{r_name, r_string};

use crate::data::BASE;

const OUTCOMES: &[VarKind] = &[VarKind::Number, VarKind::Factor, VarKind::Text];
const PREDICTORS: &[VarKind] = &[
    VarKind::Number,
    VarKind::Factor,
    VarKind::Text,
    VarKind::Logical,
];

const FAMILIES: &[Opt] = &[
    Opt {
        value: "gaussian",
        label: "Linear (Gaussian outcome)",
    },
    Opt {
        value: "binomial",
        label: "Logit (0/1 outcome)",
    },
    Opt {
        value: "poisson",
        label: "Poisson (count outcome)",
    },
    Opt {
        value: "neg_binomial",
        label: "Negative binomial (overdispersed counts)",
    },
    Opt {
        value: "gamma",
        label: "Gamma, log link (positive outcome)",
    },
    Opt {
        value: "ordinal",
        label: "Ordered logit (ordered categories)",
    },
];

const PRIORS: &[Opt] = &[
    Opt {
        value: "default",
        label: "The default priors (rstanarm's weakly informative ones, or Stata's when ticked)",
    },
    Opt {
        value: "normal",
        label: "Normal(location, scale)",
    },
    Opt {
        value: "student_t",
        label: "Student t(df, location, scale)",
    },
    Opt {
        value: "cauchy",
        label: "Cauchy(location, scale)",
    },
];

const LEVELS: &[Opt] = &[
    Opt {
        value: "0.95",
        label: "95%",
    },
    Opt {
        value: "0.9",
        label: "90%",
    },
    Opt {
        value: "0.99",
        label: "99%",
    },
];

/// A whole number in `least..=most`, or a problem on the widget.
fn whole(a: &Args, id: &'static str, what: &str, least: u32, most: u32, out: &mut Vec<Problem>) {
    match a.number(id).trim().parse::<u32>() {
        Ok(n) if (least..=most).contains(&n) => {}
        _ => out.push(Problem::new(
            id,
            format!("{what} must be a whole number from {least} to {most}."),
        )),
    }
}

/// The numbers of a box that takes one number or several separated by commas;
/// `None` when one is not a finite number.
fn numbers(text: &str) -> Option<Vec<f64>> {
    let parts: Vec<&str> = text.split(',').map(str::trim).collect();
    if parts.iter().any(|p| p.is_empty()) {
        return None;
    }
    parts
        .iter()
        .map(|p| p.parse::<f64>().ok().filter(|v| v.is_finite()))
        .collect()
}

/// The numbers of a list box as R: `2` or `c(2, 3)` (the parsed numbers, so
/// nothing else can reach the code).
fn r_numbers(text: &str) -> String {
    let v = numbers(text).unwrap_or_default();
    let items: Vec<String> = v.iter().map(|x| format!("{x}")).collect();
    match items.len() {
        1 => items.concat(),
        _ => format!("c({})", items.join(", ")),
    }
}

/// Checks a prior's boxes: locations any numbers, scales and degrees of
/// freedom above 0.
fn prior_check(
    a: &Args,
    kind: &str,
    ids: (&'static str, &'static str, &'static str),
    out: &mut Vec<Problem>,
) {
    if kind == "default" {
        return;
    }
    let (loc, scale, df) = ids;
    if numbers(a.values.text(loc)).is_none() {
        out.push(Problem::new(
            loc,
            "Type a number, or one for each coefficient separated by commas.",
        ));
    }
    let positive =
        |id: &str| numbers(a.values.text(id)).is_some_and(|v| v.iter().all(|x| *x > 0.0));
    if !positive(scale) {
        out.push(Problem::new(
            scale,
            "Each scale must be a number above 0 (one, or one for each coefficient).",
        ));
    }
    if kind == "student_t" && !positive(df) {
        out.push(Problem::new(
            df,
            "Each degrees of freedom must be a number above 0.",
        ));
    }
}

/// A prior as rstanarm code, on the scale of the data (`autoscale = FALSE`).
fn prior_code(a: &Args, kind: &str, ids: (&str, &str, &str)) -> String {
    let (loc, scale, df) = ids;
    let (loc, scale, df) = (
        r_numbers(a.values.text(loc)),
        r_numbers(a.values.text(scale)),
        r_numbers(a.values.text(df)),
    );
    match kind {
        "student_t" => format!(
            "rstanarm::student_t(df = {df}, location = {loc}, scale = {scale}, autoscale = FALSE)"
        ),
        "cauchy" => {
            format!("rstanarm::cauchy(location = {loc}, scale = {scale}, autoscale = FALSE)")
        }
        _ => format!("rstanarm::normal(location = {loc}, scale = {scale}, autoscale = FALSE)"),
    }
}

const COEFFICIENTS: (&str, &str, &str) = ("prior_mean", "prior_sd", "prior_df");
const INTERCEPT: (&str, &str, &str) = ("int_mean", "int_sd", "int_df");

// ---- Stata's priors, sampled in plain R (small-fixes rows 14 and 22) ----

/// A log prior density of the vector `b` as R: Stata's default
/// normal(0, 10000) (a variance; the sd is 100), or the typed normal, Student t
/// or Cauchy, each coefficient on its own.
fn stata_prior_r(a: &Args, kind: &str, ids: (&str, &str, &str)) -> String {
    let (loc, scale, df) = ids;
    let (loc, scale, df) = (
        r_numbers(a.values.text(loc)),
        r_numbers(a.values.text(scale)),
        r_numbers(a.values.text(df)),
    );
    match kind {
        "normal" => format!("sum(stats::dnorm(b, {loc}, {scale}, log = TRUE))"),
        "student_t" => {
            format!("sum(stats::dt((b - {loc}) / {scale}, {df}, log = TRUE) - log({scale}))")
        }
        "cauchy" => format!("sum(stats::dcauchy(b, {loc}, {scale}, log = TRUE))"),
        _ => "sum(stats::dnorm(b, 0, 100, log = TRUE))".to_string(),
    }
}

/// The R that checks a typed prior's lists against the number of
/// coefficients `n` (an R expression).
fn stata_prior_lengths(a: &Args, kind: &str, n: &str) -> String {
    if kind == "default" {
        return String::new();
    }
    let (loc, scale, df) = COEFFICIENTS;
    let mut lists = vec![
        r_numbers(a.values.text(loc)),
        r_numbers(a.values.text(scale)),
    ];
    if kind == "student_t" {
        lists.push(r_numbers(a.values.text(df)));
    }
    format!(
        "  if (!all(lengths(list({})) %in% c(1, {n})))\n    \
         stop(\"Type one location and scale for every coefficient, or one for each of the \", \
         {n}, \" coefficients.\", call. = FALSE)\n",
        lists.join(", ")
    )
}

/// The random-walk Metropolis-Hastings sampler, as Stata's `bayes` samples:
/// started at the maximum-likelihood fit (jittered by one standard error per
/// chain), its proposal covariance and scale adapted during the warm-up
/// (aiming at an acceptance rate of 0.234), then kept fixed. `lp` is the log
/// posterior, `start` and `S` the start and its covariance, `back` maps the
/// sampled parameters to the reported ones.
const MH_SAMPLER: &str = "  p <- length(start); warm <- @ITER@ %/% 2
  set.seed(@SEED@)
  draws <- array(NA_real_, c(@ITER@ - warm, @CHAINS@, p), dimnames = list(NULL, NULL, names(start)))
  accepted <- numeric(@CHAINS@)
  for (chain in seq_len(@CHAINS@)) {
    L <- t(chol(S)); th <- start + as.vector(L %*% stats::rnorm(p))
    cur <- lp(th)
    if (!is.finite(cur)) { th <- start; cur <- lp(th) }
    sc <- 2.38^2 / p; kept <- matrix(NA_real_, warm, p)
    for (i in seq_len(@ITER@)) {
      prop <- th + sqrt(sc) * as.vector(L %*% stats::rnorm(p))
      new <- lp(prop)
      ok <- is.finite(new) && log(stats::runif(1)) < new - cur
      if (ok) { th <- prop; cur <- new }
      if (i <= warm) {
        kept[i, ] <- th
        sc <- sc * exp((ok - 0.234) / sqrt(i))
        if (i %% 50 == 0 && i >= 200) {
          C <- stats::cov(kept[(i %/% 2):i, , drop = FALSE])
          R <- tryCatch(chol(C + diag(1e-10 * max(diag(C)) + 1e-300, p)), error = function(e) NULL)
          if (!is.null(R)) L <- t(R)
        }
      } else {
        draws[i - warm, chain, ] <- back(th); accepted[chain] <- accepted[chain] + ok
      }
    }
  }
";

/// The family's part of the Stata-prior fit: the outcome checked, the
/// maximum-likelihood start and its covariance, and `lp`, the log posterior.
fn stata_family(a: &Args, family: &str) -> String {
    let coef = stata_prior_r(a, a.choice("prior"), COEFFICIENTS);
    let constant = stata_prior_r(a, a.choice("int_prior"), INTERCEPT);
    let lengths = stata_prior_lengths(a, a.choice("prior"), "sum(!int)");
    let priors = format!(
        "  int <- colnames(X) == \"(Intercept)\"\n\
         {lengths}  \
         prior_b <- function(b) {coef}\n  \
         prior_c <- function(b) {constant}\n  \
         lpb <- function(b) prior_b(b[!int]) + prior_c(b[int])\n"
    );
    match family {
        "ordinal" => {
            let lengths = stata_prior_lengths(a, a.choice("prior"), "ncol(X)");
            format!(
                "  X <- X[, colnames(X) != \"(Intercept)\", drop = FALSE]\n  \
                 if (ncol(X) == 0) stop(\"The ordered logit needs a predictor.\", call. = FALSE)\n  \
                 y <- if (is.ordered(y)) y else factor(y, ordered = TRUE)\n  \
                 y <- droplevels(y); lev <- levels(y)\n  \
                 if (length(lev) < 3) stop(\"An ordered logit needs three categories or more.\", call. = FALSE)\n\
                 {lengths}  \
                 g <- MASS::polr(y ~ X, Hess = TRUE)\n  \
                 start <- c(g$coefficients, g$zeta); S <- solve(g$Hessian)\n  \
                 names(start) <- c(colnames(X), paste(lev[-length(lev)], lev[-1], sep = \"|\"))\n  \
                 k <- ncol(X); y <- as.integer(y); nc <- length(lev) - 1\n  \
                 prior_b <- function(b) {coef}\n  \
                 lp <- function(t) {{\n    \
                 cuts <- t[k + seq_len(nc)]\n    \
                 if (any(diff(cuts) <= 0)) return(-Inf)\n    \
                 xb <- as.vector(X %*% t[seq_len(k)]); cc <- c(-Inf, cuts, Inf)\n    \
                 sum(log(stats::plogis(cc[y + 1] - xb) - stats::plogis(cc[y] - xb))) + prior_b(t[seq_len(k)])\n  \
                 }}\n"
            )
        }
        "gaussian" => format!(
            "  if (!is.numeric(y)) stop(\"The outcome must be a number.\", call. = FALSE)\n\
             {priors}  \
             g <- stats::lm.fit(X, y); n <- length(y); k <- ncol(X); v0 <- sum(g$residuals^2) / n\n  \
             start <- c(g$coefficients, log(v0)); names(start) <- c(colnames(X), \"sigma2\")\n  \
             S <- rbind(cbind(v0 * solve(crossprod(X)), 0), c(rep(0, k), 2 / n))\n  \
             # sigma2 is sampled on its log: igamma(0.01, 0.01) and the Jacobian\n  \
             lp <- function(t) {{\n    \
             v <- exp(t[k + 1])\n    \
             sum(stats::dnorm(y, as.vector(X %*% t[1:k]), sqrt(v), log = TRUE)) + lpb(t[1:k]) +\n      \
             stats::dgamma(1 / v, 0.01, rate = 0.01, log = TRUE) - log(v)\n  \
             }}\n  \
             back <- function(t) {{ t[k + 1] <- exp(t[k + 1]); t }}\n"
        ),
        "neg_binomial" => format!(
            "  if (!is.numeric(y) || any(y < 0 | y != round(y))) stop(\"The outcome must be a count \
             (whole numbers from 0).\", call. = FALSE)\n\
             {priors}  \
             g <- MASS::glm.nb(y ~ X - 1); k <- ncol(X)\n  \
             start <- c(g$coefficients, -log(g$theta)); names(start) <- c(colnames(X), \"lnalpha\")\n  \
             S <- rbind(cbind(stats::vcov(g), 0), c(rep(0, k), (g$SE.theta / g$theta)^2))\n  \
             lp <- function(t) sum(stats::dnbinom(y, size = exp(-t[k + 1]), mu = exp(as.vector(X %*% t[1:k])), \
             log = TRUE)) +\n    lpb(t[1:k]) + stats::dnorm(t[k + 1], 0, 100, log = TRUE)\n"
        ),
        _ => {
            let (check, fam, ll) = match family {
                "binomial" => (
                    "  y <- as.numeric(if (is.factor(y)) y != levels(y)[1] else if (is.character(y)) \
                     y != sort(unique(y))[1] else y != 0)\n",
                    "stats::binomial()",
                    "sum(stats::plogis(ifelse(y == 1, eta, -eta), log.p = TRUE))",
                ),
                "poisson" => (
                    "  if (!is.numeric(y) || any(y < 0 | y != round(y))) stop(\"The outcome must be a \
                     count (whole numbers from 0).\", call. = FALSE)\n",
                    "stats::poisson()",
                    "sum(stats::dpois(y, exp(eta), log = TRUE))",
                ),
                // Stata's bayes: glm, family(gamma) fixes the scale at 1
                _ => (
                    "  if (!is.numeric(y) || any(y <= 0)) stop(\"The outcome must be above 0.\", \
                     call. = FALSE)\n",
                    "stats::Gamma(link = \"log\")",
                    "sum(stats::dexp(y, exp(-eta), log = TRUE))",
                ),
            };
            format!(
                "{check}{priors}  \
                 g <- stats::glm.fit(X, y, family = {fam})\n  \
                 start <- g$coefficients; names(start) <- colnames(X)\n  \
                 S <- chol2inv(qr.R(g$qr))[order(g$qr$pivot), order(g$qr$pivot)]\n  \
                 lp <- function(t) {{ eta <- as.vector(X %*% t); {ll} + lpb(t) }}\n"
            )
        }
    }
}

/// The whole Stata-prior fit, kept as a list of class `bayes_mh`: the draws
/// (iterations by chains by parameters), the acceptance rates and the priors.
fn stata_fit(a: &Args, m: &str, y: &str, x: &str) -> String {
    let family = a.choice("family");
    let sampler = MH_SAMPLER
        .replace("@ITER@", a.number("iter").trim())
        .replace("@CHAINS@", a.number("chains").trim())
        .replace("@SEED@", a.number("seed").trim());
    let back = if family == "gaussian" {
        ""
    } else {
        "  back <- identity\n"
    };
    format!(
        "{m} <- local({{\n  \
         f <- {y} ~ {x}\n  \
         d <- stats::model.frame(f, data = {})\n  \
         X <- stats::model.matrix(f, d); y <- stats::model.response(d)\n\
         {}{back}{sampler}  \
         structure(list(draws = draws, acceptance = accepted / (@ITER@ - warm), family = {}, \
         formula = f, nobs = nrow(d)), class = \"bayes_mh\")\n}})",
        a.data(),
        stata_family(a, family),
        r_string(family),
    )
    .replace("@ITER@", a.number("iter").trim())
}

/// The priors of a Stata-prior fit, as Stata names them.
fn stata_priors_text(a: &Args) -> String {
    let typed = |kind: &str, ids: (&str, &str, &str)| {
        let (loc, scale, df) = ids;
        let (loc, scale, df) = (
            r_numbers(a.values.text(loc)),
            r_numbers(a.values.text(scale)),
            r_numbers(a.values.text(df)),
        );
        match kind {
            "normal" => format!("normal(mean {loc}, sd {scale})"),
            "student_t" => format!("Student t(df {df}, location {loc}, scale {scale})"),
            "cauchy" => format!("Cauchy(location {loc}, scale {scale})"),
            _ => "normal(0, 10000) (a variance)".to_string(),
        }
    };
    let mut lines = vec![format!(
        "coefficients: {}",
        typed(a.choice("prior"), COEFFICIENTS)
    )];
    match a.choice("family") {
        "ordinal" => lines.push("cutpoints: flat (1)".into()),
        family => {
            lines.push(format!(
                "constant: {}",
                typed(a.choice("int_prior"), INTERCEPT)
            ));
            match family {
                "gaussian" => lines.push("sigma2: igamma(0.01, 0.01)".into()),
                "neg_binomial" => lines.push("lnalpha: normal(0, 10000)".into()),
                "gamma" => lines.push("scale: fixed at 1, as Stata's bayes: glm".into()),
                _ => {}
            }
        }
    }
    lines.join("; ")
}

fn bayes_check(a: &Args) -> Vec<Problem> {
    let y = a.var_text("y");
    let mut out = Vec::new();
    if a.vars("x").contains(&y) {
        out.push(Problem::new(
            "x",
            format!("{y} is the outcome; it cannot also be a predictor."),
        ));
    }
    let ordinal = a.choice("family") == "ordinal";
    let stata = a.flag("stata");
    if stata && a.flag("prior_pd") {
        out.push(Problem::new(
            "prior_pd",
            "Sampling the priors alone is done by rstanarm; Stata's normal(0, 10000) priors \
             are too wide to show anything. Untick one of the two.",
        ));
    }
    if stata && ordinal {
        prior_check(a, a.choice("prior"), COEFFICIENTS, &mut out);
        if a.choice("int_prior") != "default" {
            out.push(Problem::new(
                "int_prior",
                "An ordered logit has cutpoints, not a constant; Stata's priors on them are flat.",
            ));
        }
    } else if stata {
        prior_check(a, a.choice("prior"), COEFFICIENTS, &mut out);
        prior_check(a, a.choice("int_prior"), INTERCEPT, &mut out);
    } else if ordinal && a.choice("prior") != "default" {
        out.push(Problem::new(
            "prior",
            "rstanarm's ordered logit (stan_polr) takes no normal, t or Cauchy prior on the \
             coefficients; it is fitted with flat priors on them.",
        ));
    } else {
        prior_check(a, a.choice("prior"), COEFFICIENTS, &mut out);
    }
    if stata {
        // checked above
    } else if ordinal && a.choice("int_prior") != "default" {
        out.push(Problem::new(
            "int_prior",
            "An ordered logit has cutpoints, not an intercept; their prior is rstanarm's \
             Dirichlet on the category probabilities.",
        ));
    } else {
        prior_check(a, a.choice("int_prior"), INTERCEPT, &mut out);
    }
    whole(a, "chains", "The number of chains", 1, 64, &mut out);
    whole(
        a,
        "iter",
        "The number of iterations",
        100,
        1_000_000,
        &mut out,
    );
    whole(a, "seed", "The seed", 0, 2_147_483_647, &mut out);
    if a.name_text("save") == a.ctx.dataset {
        out.push(Problem::new(
            "save",
            format!("{} is the data frame; choose another name.", a.ctx.dataset),
        ));
    }
    out
}

fn bayes_code(a: &Args) -> String {
    let m = a.name("save");
    let y = a.var("y").unwrap_or_default();
    let x = a
        .vars("x")
        .iter()
        .map(|n| r_name(n))
        .collect::<Vec<_>>()
        .join(" + ");
    if a.flag("stata") {
        return stata_code(a, &m, &y, &x);
    }
    let mut priors = String::new();
    if a.choice("prior") != "default" {
        priors.push_str(&format!(
            ", prior = {}",
            prior_code(a, a.choice("prior"), COEFFICIENTS)
        ));
    }
    if a.choice("int_prior") != "default" {
        priors.push_str(&format!(
            ", prior_intercept = {}",
            prior_code(a, a.choice("int_prior"), INTERCEPT)
        ));
    }
    if a.flag("prior_pd") {
        priors.push_str(", prior_PD = TRUE");
    }
    let (chains, iter, seed) = (
        a.number("chains").trim(),
        a.number("iter").trim(),
        a.number("seed").trim(),
    );
    let sampler = format!(
        "chains = {chains}, iter = {iter}, seed = {seed}, init_r = 0.5, cores = 1, refresh = 0"
    );
    let fit = if a.choice("family") == "ordinal" {
        let ys = a.var_string("y").unwrap_or_default();
        format!(
            "{m} <- local({{\n  d <- {}\n  \
             d[[{ys}]] <- if (is.ordered(d[[{ys}]])) d[[{ys}]] else factor(d[[{ys}]], ordered = TRUE)\n  \
             rstanarm::stan_polr({y} ~ {x}, data = d, prior = NULL{priors}, {sampler})\n}})",
            a.data()
        )
    } else {
        let family = match a.choice("family") {
            "binomial" => "stats::binomial()",
            "poisson" => "stats::poisson()",
            "neg_binomial" => "rstanarm::neg_binomial_2()",
            "gamma" => "stats::Gamma(link = \"log\")",
            _ => "stats::gaussian()",
        };
        format!(
            "{m} <- rstanarm::stan_glm({y} ~ {x}, data = {}, family = {family}{priors}, {sampler})",
            a.data()
        )
    };
    let level = a.choice("level");
    let percent = match level {
        "0.9" => "90",
        "0.99" => "99",
        _ => "95",
    };
    let what = if a.flag("prior_pd") {
        "Prior summary (the data are not used)"
    } else {
        "Posterior summary"
    };
    let mut code = format!(
        "{fit}\n\
         cat(\"{what}: {chains} chains of {iter} iterations (the first half warm-up), \
         {percent}% credible interval\\n\")\n\
         local({{\n  s <- unclass(summary({m}, probs = c((1 - {level}) / 2, \
         (1 + {level}) / 2)))\n  \
         s <- s[!grepl(\"^mean_PPD\", rownames(s)) & rownames(s) != \"log-posterior\", , drop = FALSE]\n  \
         print(round(s, 4))\n  \
         if (any(s[, \"Rhat\"] > 1.05 | s[, \"n_eff\"] < 100, na.rm = TRUE))\n    \
         cat(\"\\nWarning: the chains have not converged (R-hat above 1.05 or fewer than \
         100 effective draws); do not trust these numbers.\\nRun more iterations or use a \
         tighter prior.\\n\")\n  invisible(NULL)\n}})"
    );
    if a.flag("show_priors") {
        code.push_str(&format!("\nprint(rstanarm::prior_summary({m}))"));
    }
    code
}

/// The fit under Stata's priors and its summary table (the same columns as
/// rstanarm's, from `rstan::monitor`), with the acceptance rate.
fn stata_code(a: &Args, m: &str, y: &str, x: &str) -> String {
    let (chains, iter, level) = (
        a.number("chains").trim(),
        a.number("iter").trim(),
        a.choice("level"),
    );
    let percent = match level {
        "0.9" => "90",
        "0.99" => "99",
        _ => "95",
    };
    format!(
        "{}\n\
         cat(\"Posterior summary: Stata's priors, random-walk Metropolis-Hastings, {chains} chains of \
         {iter} iterations (the first half warm-up), {percent}% credible interval\\n\")\n\
         cat(\"Priors: {}\\n\")\n\
         local({{\n  \
         s <- rstan::monitor({m}$draws, warmup = 0, probs = c((1 - {level}) / 2, (1 + {level}) / 2), \
         print = FALSE)\n  \
         s <- as.matrix(as.data.frame(s)[, 1:7]); colnames(s)[2] <- \"mcse\"\n  \
         print(round(s, 4))\n  \
         cat(\"\\nAcceptance rate:\", round(mean({m}$acceptance), 3), \"\\n\")\n  \
         if (any(s[, \"Rhat\"] > 1.05 | s[, \"n_eff\"] < 100, na.rm = TRUE))\n    \
         cat(\"\\nWarning: the chains have not converged (R-hat above 1.05 or fewer than \
         100 effective draws); do not trust these numbers.\\nRun more iterations.\\n\")\n  \
         invisible(NULL)\n}})",
        stata_fit(a, m, y, x),
        stata_priors_text(a).replace('\\', "\\\\").replace('"', "\\\""),
    )
}

pub const BAYESIAN_REGRESSION: Spec = Spec {
    entry: "Bayesian regression",
    menu: "Statistics > Bayesian regression",
    about: "Fits a regression by Bayes' rule, sampling the posterior with Hamiltonian Monte \
            Carlo (rstanarm): a linear model with a Gaussian outcome, a logit model of a 0/1 \
            outcome, a Poisson or negative binomial model of counts, a Gamma model (log link) \
            of a positive outcome (stan_glm), or an ordered logit of ordered categories \
            (stan_polr: a factor's own order, numbers in their order, text in alphabetical \
            order). The priors are rstanarm's default weakly informative ones, or a normal, \
            Student t or Cauchy prior on the coefficients as typed, on the scale of the data: \
            one location and scale for every coefficient, or one for each, separated by \
            commas. The intercept takes its own prior the same way; rstanarm puts it on the \
            intercept at the centred predictors, which is Stata's constant when the \
            predictors have mean 0. The ordered logit's coefficients have flat priors and its \
            cutpoints rstanarm's Dirichlet prior on the category probabilities. The error \
            scale (sigma), the negative binomial's reciprocal dispersion and the Gamma shape \
            keep rstanarm's exponential prior, which is not Stata's. The table gives each \
            parameter's posterior mean, standard deviation and equal-tailed credible interval, \
            its Monte Carlo standard error (mcse), its effective sample size (n_eff) and R-hat, \
            which is close to 1 once the chains agree. The iterations are per chain, the first \
            half of them warm-up. The same seed gives the same draws. A warning follows the \
            table when a chain has not converged (R-hat above 1.05, or fewer than 100 \
            effective draws). Sampling from the priors only (the data unused) shows what the \
            priors imply. Posterior means agree with Stata's bayes to within Monte Carlo error \
            only under priors that are the same. With \"Stata's priors and sampler\" ticked the \
            fit is Stata's instead: its default priors (normal(0, 10000) on every coefficient \
            and on the constant itself, not at the centred predictors; igamma(0.01, 0.01) on \
            sigma-squared, which the table reports; normal(0, 10000) on the negative \
            binomial's log alpha; flat cutpoints; the Gamma scale fixed at 1, as Stata's bayes: \
            glm fixes it), or the typed normal, t or Cauchy priors, sampled as Stata samples, \
            by random-walk Metropolis-Hastings started at the maximum-likelihood fit, its \
            proposal tuned during the warm-up. Such a sampler needs more iterations than \
            Hamiltonian Monte Carlo for the same precision (Stata draws 10,000 after 2,500 \
            warm-up). When a count shows little overdispersion, log alpha's posterior under \
            normal(0, 10000) keeps a long tail toward minus infinity (alpha near 0) that a \
            random-walk sampler, Stata's too, does not reach; its mean is then not the \
            posterior's, though the coefficients barely depend on it. The Objects pane's Model \
            menu has posterior predictions, trace plots and predictive checks, and loo \
            comparison for an rstanarm fit. Rows with a missing value are left out.",
    widgets: &[
        Widget::Variable {
            id: "y",
            label: "Outcome",
            kinds: OUTCOMES,
            required: true,
            hint: "A 0/1 variable for the logit model, a count for the Poisson and negative \
                   binomial models, above 0 for the Gamma model, ordered categories for the \
                   ordered logit.",
        },
        Widget::Variables {
            id: "x",
            label: "Predictors",
            kinds: PREDICTORS,
            min: 1,
            hint: "A text or factor variable becomes dummy variables for its levels.",
        },
        Widget::Choice {
            id: "family",
            label: "Model",
            options: FAMILIES,
            default: "gaussian",
            hint: "",
        },
        Widget::Check {
            id: "stata",
            label: "Stata's priors and sampler (as Stata's bayes)",
            default: false,
            hint: "Stata's default priors (normal(0, 10000) on every coefficient and the \
                   constant, igamma(0.01, 0.01) on sigma-squared, normal(0, 10000) on the \
                   negative binomial's log alpha, flat cutpoints, the Gamma scale fixed at 1), or \
                   the priors typed below on the coefficients and the constant itself, sampled by \
                   random-walk Metropolis-Hastings in R. Not an rstanarm fit, so the Model menu \
                   does not take it.",
        },
        Widget::Choice {
            id: "prior",
            label: "Prior on the coefficients",
            options: PRIORS,
            default: "default",
            hint: "The default priors scale with the data; a typed prior is used as it is, on \
                   the scale of the data.",
        },
        Widget::Text {
            id: "prior_mean",
            label: "Location(s)",
            default: TextDefault::Fixed("0"),
            rule: TextRule::Any,
            hint: "Used with a typed prior. One number for every coefficient, or one for each, \
                   separated by commas, in the order of the model's coefficients (a factor's \
                   levels after its first).",
        },
        Widget::Text {
            id: "prior_sd",
            label: "Scale(s)",
            default: TextDefault::Fixed("10"),
            rule: TextRule::Any,
            hint: "Above 0; the standard deviation of a normal prior.",
        },
        Widget::Text {
            id: "prior_df",
            label: "Degrees of freedom (Student t)",
            default: TextDefault::Fixed("3"),
            rule: TextRule::Any,
            hint: "Above 0; one, or one for each coefficient.",
        },
        Widget::Choice {
            id: "int_prior",
            label: "Prior on the intercept",
            options: PRIORS,
            default: "default",
            hint: "rstanarm puts it on the intercept at the centred predictors; with Stata's priors it is on the constant itself.",
        },
        Widget::Text {
            id: "int_mean",
            label: "Intercept location",
            default: TextDefault::Fixed("0"),
            rule: TextRule::Any,
            hint: "",
        },
        Widget::Text {
            id: "int_sd",
            label: "Intercept scale",
            default: TextDefault::Fixed("10"),
            rule: TextRule::Any,
            hint: "Above 0.",
        },
        Widget::Text {
            id: "int_df",
            label: "Intercept degrees of freedom (Student t)",
            default: TextDefault::Fixed("3"),
            rule: TextRule::Any,
            hint: "",
        },
        Widget::Text {
            id: "chains",
            label: "Chains",
            default: TextDefault::Fixed("4"),
            rule: TextRule::Number,
            hint: "Independent runs of the sampler; R-hat compares them.",
        },
        Widget::Text {
            id: "iter",
            label: "Iterations per chain",
            default: TextDefault::Fixed("2000"),
            rule: TextRule::Number,
            hint: "Warm-up included: the first half is thrown away.",
        },
        Widget::Text {
            id: "seed",
            label: "Seed",
            default: TextDefault::Fixed("1"),
            rule: TextRule::Number,
            hint: "The same seed gives the same draws.",
        },
        Widget::Choice {
            id: "level",
            label: "Credible interval",
            options: LEVELS,
            default: "0.95",
            hint: "Equal-tailed: the same probability left in each tail.",
        },
        Widget::Check {
            id: "show_priors",
            label: "Show the priors used",
            default: false,
            hint: "Prints rstanarm's prior summary, with the scales it adjusted to the data.",
        },
        Widget::Check {
            id: "prior_pd",
            label: "Sample from the priors only (the data unused)",
            default: false,
            hint: "To see what the priors imply before the data are used.",
        },
        Widget::Text {
            id: "save",
            label: "Save the model as",
            default: TextDefault::Fixed("bayes"),
            rule: TextRule::Name,
            hint: "The fitted model stays in R's workspace under this name; a name already in use \
                   is replaced.",
        },
    ],
    needs: &["rstanarm"],
    styles: BASE,
    check: bayes_check,
    code: |a, _| bayes_code(a),
};

// ---- The Model menu on a Bayesian fit (P6.9b) ----

const PREDICTION_KINDS: &[Opt] = &[
    Opt {
        value: "epred",
        label: "The expected outcome (its posterior mean and interval)",
    },
    Opt {
        value: "predict",
        label: "A new outcome (the posterior predictive mean and interval)",
    },
    Opt {
        value: "linpred",
        label: "The linear predictor",
    },
];

fn predictions_check(a: &Args) -> Vec<Problem> {
    let mut out = Vec::new();
    if a.name_text("data").is_empty() {
        out.push(Problem::new("data", "Type the data frame's name."));
    }
    if a.name_text("prefix").is_empty() {
        out.push(Problem::new("prefix", "Name the new variables."));
    }
    whole(a, "seed", "The seed", 0, 2_147_483_647, &mut out);
    out
}

pub const POSTERIOR_PREDICTIONS: Spec = Spec {
    entry: "Posterior predictions",
    menu: "Objects > Model > Posterior predictions",
    about: "Adds the fit's predictions for each row of a data frame as new variables: the \
            posterior mean and equal-tailed credible interval of the expected outcome \
            (rstanarm::posterior_epred: the mean, the probability or the expected count), of a \
            new outcome drawn from the model (posterior_predict, with a seed), or of the linear \
            predictor (posterior_linpred). For an ordered logit it adds each category's \
            posterior mean probability, worked out from the linear predictor and the \
            cutpoints of every draw. Rows with a missing predictor get missing values. A name \
            already in the data frame is replaced.",
    widgets: &[
        Widget::Text {
            id: "data",
            label: "Data frame",
            default: TextDefault::Empty,
            rule: TextRule::Name,
            hint: "The data frame to predict for (the one the model was fitted to, or new rows \
                   with the same predictors); it is changed in place.",
        },
        Widget::Choice {
            id: "kind",
            label: "Predict",
            options: PREDICTION_KINDS,
            default: "epred",
            hint: "",
        },
        Widget::Text {
            id: "prefix",
            label: "New variables",
            default: TextDefault::Fixed("pred"),
            rule: TextRule::Any,
            hint: "The mean is this name; its interval is name_lower and name_upper (for an \
                   ordered logit, name_ and each category).",
        },
        Widget::Choice {
            id: "level",
            label: "Credible interval",
            options: LEVELS,
            default: "0.95",
            hint: "",
        },
        Widget::Text {
            id: "seed",
            label: "Seed",
            default: TextDefault::Fixed("1"),
            rule: TextRule::Number,
            hint: "For new outcomes, which are drawn at random.",
        },
    ],
    needs: &["rstanarm"],
    styles: BASE,
    check: predictions_check,
    code: |a, _| {
        let (m, d) = (a.data(), a.name("data"));
        let p = a.name_text("prefix");
        let level = a.choice("level");
        let draws = match a.choice("kind") {
            "predict" => format!(
                "rstanarm::posterior_predict(fit, newdata = nd[ok, , drop = FALSE], seed = {})",
                a.number("seed")
            ),
            "linpred" => {
                "rstanarm::posterior_linpred(fit, newdata = nd[ok, , drop = FALSE])".to_string()
            }
            _ => "rstanarm::posterior_epred(fit, newdata = nd[ok, , drop = FALSE])".to_string(),
        };
        format!(
            "{d} <- local({{\n  fit <- {m}\n  nd <- {d}\n  \
             stopifnot(\"this works on a model fitted with Bayesian regression\" = inherits(fit, \"stanreg\"))\n  \
             ok <- stats::complete.cases(nd[, intersect(all.vars(stats::formula(fit))[-1], names(nd)), drop = FALSE])\n  \
             added <- character()\n  \
             put <- function(name, v) {{ nd[[name]] <<- NA_real_; nd[[name]][ok] <<- v; added <<- c(added, name) }}\n  \
             if (inherits(fit, \"polr\") && {}) {{\n    \
             eta <- rstanarm::posterior_linpred(fit, newdata = nd[ok, , drop = FALSE])\n    \
             cuts <- as.matrix(fit)[, names(fit$zeta), drop = FALSE]\n    \
             cum <- c(lapply(seq_len(ncol(cuts)), function(k) stats::plogis(cuts[, k] - eta)), list(1))\n    \
             levs <- levels(rstanarm::get_y(fit))\n    \
             for (k in seq_along(levs)) put(paste0({}, \"_\", levs[k]), \
             colMeans(if (k == 1) cum[[1]] else cum[[k]] - cum[[k - 1]]))\n  \
             }} else {{\n    draws <- {draws}\n    \
             if (inherits(fit, \"polr\")) draws <- matrix(match(draws, levels(rstanarm::get_y(fit))), nrow(draws))\n    \
             put({}, colMeans(draws))\n    \
             put({}, apply(draws, 2, stats::quantile, probs = (1 - {level}) / 2, names = FALSE))\n    \
             put({}, apply(draws, 2, stats::quantile, probs = (1 + {level}) / 2, names = FALSE))\n  }}\n  \
             cat({}, paste(added, collapse = \", \"), \"\\n\", sep = \"\")\n  nd\n}})",
            if a.choice("kind") == "epred" { "TRUE" } else { "FALSE" },
            r_string(p),
            r_string(p),
            r_string(&format!("{p}_lower")),
            r_string(&format!("{p}_upper")),
            r_string(&format!("Added to {}: ", a.name_text("data"))),
        )
    },
};

const STATISTICS: &[(&str, &str, &str)] = &[
    ("ppc_mean", "mean", "mean"),
    ("ppc_sd", "sd", "stats::sd"),
    ("ppc_min", "min", "min"),
    ("ppc_max", "max", "max"),
];

fn checks_check(a: &Args) -> Vec<Problem> {
    let mut out = Vec::new();
    whole(a, "seed", "The seed", 0, 2_147_483_647, &mut out);
    let any =
        a.flag("trace") || a.flag("density") || STATISTICS.iter().any(|(id, _, _)| a.flag(id));
    if !any {
        out.push(Problem::new(
            "trace",
            "Choose at least one plot or statistic.",
        ));
    }
    out
}

pub const BAYES_CHECKS: Spec = Spec {
    entry: "Trace plots and predictive checks",
    menu: "Objects > Model > Trace plots and predictive checks",
    about: "Checks a Bayesian fit. The trace plot shows each parameter's draws chain by chain \
            (they should overlap and wander without trend); the density plot overlays the \
            outcome's distribution on those of fifty data sets drawn from the posterior \
            predictive distribution (rstanarm::pp_check). The posterior predictive p-values \
            are the share of the data sets drawn from the posterior predictive distribution \
            (one per posterior draw, with the seed) whose mean, standard deviation, minimum or \
            maximum is at least the observed one, as Stata's bayesstats ppvalues reports them; \
            values near 0 or 1 show what the model misses. An ordered outcome is taken as its \
            category numbers. The p-values are kept in `name$ppvalues`.",
    widgets: &[
        Widget::Check {
            id: "trace",
            label: "Trace plot",
            default: true,
            hint: "",
        },
        Widget::Check {
            id: "density",
            label: "Posterior predictive density overlay",
            default: true,
            hint: "",
        },
        Widget::Check {
            id: "ppc_mean",
            label: "p-value of the mean",
            default: true,
            hint: "",
        },
        Widget::Check {
            id: "ppc_sd",
            label: "p-value of the standard deviation",
            default: true,
            hint: "",
        },
        Widget::Check {
            id: "ppc_min",
            label: "p-value of the minimum",
            default: false,
            hint: "",
        },
        Widget::Check {
            id: "ppc_max",
            label: "p-value of the maximum",
            default: false,
            hint: "",
        },
        Widget::Text {
            id: "seed",
            label: "Seed",
            default: TextDefault::Fixed("1"),
            rule: TextRule::Number,
            hint: "The replicated data sets are drawn at random.",
        },
    ],
    needs: &["rstanarm"],
    styles: BASE,
    check: checks_check,
    code: |a, _| {
        let m = a.data();
        let mut code = format!(
            "stopifnot(\"this works on a model fitted with Bayesian regression\" = inherits({m}, \"stanreg\"))"
        );
        if a.flag("trace") {
            code.push_str(&format!("\nprint(plot({m}, \"trace\"))"));
        }
        if a.flag("density") {
            code.push_str(&format!(
                "\nprint(rstanarm::pp_check({m}, nreps = 50, seed = {}))",
                a.number("seed")
            ));
        }
        let stats: Vec<String> = STATISTICS
            .iter()
            .filter(|(id, _, _)| a.flag(id))
            .map(|(_, name, f)| format!("{name} = {f}"))
            .collect();
        if !stats.is_empty() {
            code.push_str(&format!(
                "\n{m}$ppvalues <- local({{\n  \
                 y <- rstanarm::get_y({m})\n  \
                 yrep <- rstanarm::posterior_predict({m}, seed = {})\n  \
                 if (is.factor(y)) {{\n    yrep <- matrix(match(yrep, levels(y)), nrow(yrep))\n    \
                 y <- as.integer(y)\n  }}\n  \
                 y <- as.numeric(y)\n  \
                 stats <- list({})\n  \
                 data.frame(observed = vapply(stats, function(f) f(y), 0), \
                 replicated = vapply(stats, function(f) mean(apply(yrep, 1, f)), 0), \
                 p = vapply(stats, function(f) mean(apply(yrep, 1, f) >= f(y)), 0))\n}})\n\
                 cat(\"Posterior predictive p-values (share of replicated data sets at least as large)\\n\")\n\
                 print({m}$ppvalues)",
                a.number("seed"),
                stats.join(", ")
            ));
        }
        code
    },
};

fn loo_names<'a>(a: &'a Args) -> Vec<&'a str> {
    a.values
        .text("others")
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect()
}

pub const BAYES_LOO: Spec = Spec {
    entry: "Compare Bayesian models (loo)",
    menu: "Objects > Model > Compare Bayesian models (loo)",
    about: "Estimates how well each Bayesian fit predicts a row it has not seen, by Pareto \
            smoothed importance sampling leave-one-out cross-validation (rstanarm::loo): the \
            expected log predictive density (elpd_loo, higher is better), the effective number \
            of parameters (p_loo) and looic (-2 elpd_loo), with standard errors, then the \
            models ranked by loo::loo_compare, each one's difference from the best and the \
            standard error of that difference. Each model's loo is kept in `name$loo`. A \
            Pareto k above 0.7 flags rows whose estimate cannot be trusted. Stata's bayes has \
            no leave-one-out comparison (its bayesstats ic gives DIC and the marginal \
            likelihood). The models should be fitted to the same rows.",
    widgets: &[Widget::Text {
        id: "others",
        label: "Other models",
        default: TextDefault::Empty,
        rule: TextRule::Any,
        hint: "The names of other Bayesian fits in the workspace, separated by commas; this \
               model comes first. Leave empty for this model's loo alone.",
    }],
    needs: &["rstanarm"],
    styles: BASE,
    check: |_| Vec::new(),
    code: |a, _| {
        let mut names = vec![a.ctx.dataset.as_str()];
        names.extend(loo_names(a));
        let mut code = String::new();
        for n in &names {
            let m = r_name(n);
            code.push_str(&format!(
                "stopifnot(\"this works on models fitted with Bayesian regression\" = inherits({m}, \"stanreg\"))\n\
                 {m}$loo <- rstanarm::loo({m})\n"
            ));
        }
        let list = names
            .iter()
            .map(|n| format!("{} = {}$loo", r_name(n), r_name(n)))
            .collect::<Vec<_>>()
            .join(", ");
        code.push_str(&format!(
            "local({{\n  loos <- list({list})\n  \
             print(do.call(rbind, lapply(names(loos), function(n) {{\n    \
             e <- loos[[n]]$estimates\n    \
             data.frame(model = n, elpd_loo = e[\"elpd_loo\", 1], se = e[\"elpd_loo\", 2], \
             p_loo = e[\"p_loo\", 1], looic = e[\"looic\", 1], \
             bad_k = sum(loos[[n]]$diagnostics$pareto_k > 0.7))\n  }})), row.names = FALSE)\n  \
             if (length(loos) > 1) {{\n    cat(\"\\nRanked by elpd_loo\\n\")\n    \
             print(loo::loo_compare(loos))\n  }}\n  invisible(NULL)\n}})"
        ));
        code
    },
};

/// The Model menu on a Bayesian fit (`stanreg`).
pub const MODEL_SPECS: &[&Spec] = &[&POSTERIOR_PREDICTIONS, &BAYES_CHECKS, &BAYES_LOO];

pub const SPECS: &[&Spec] = &[&BAYESIAN_REGRESSION];

#[cfg(test)]
mod tests {
    use super::*;
    use rcraft_rcodegen::dialog::{ColumnInfo, Context, Style, Values};

    fn ctx() -> Context {
        let col = |name: &str, kind| ColumnInfo {
            name: name.into(),
            kind,
        };
        Context {
            dataset: "d".into(),
            frames: Vec::new(),
            columns: vec![
                col("y", VarKind::Number),
                col("arm span", VarKind::Number),
                col("age", VarKind::Number),
                col("group", VarKind::Factor),
            ],
        }
    }

    fn values(set: &[(&str, &str)], x: &[&str]) -> Values {
        let mut v = BAYESIAN_REGRESSION.defaults(&ctx());
        v.set_text("y", "y");
        v.set_list("x", x.iter().map(|n| n.to_string()).collect());
        for (id, text) in set {
            v.set_text(id, *text);
        }
        v
    }

    fn widgets(v: &Values) -> Vec<&'static str> {
        BAYESIAN_REGRESSION
            .problems(&ctx(), v)
            .iter()
            .map(|p| p.widget)
            .collect()
    }

    #[test]
    fn writes_stan_glm() {
        let v = values(&[], &["arm span", "group"]);
        let c = BAYESIAN_REGRESSION
            .generate(&ctx(), &v, Style::Base)
            .unwrap_or_default();
        assert_eq!(
            c,
            "bayes <- rstanarm::stan_glm(y ~ `arm span` + group, data = d, family = stats::gaussian(), \
             chains = 4, iter = 2000, seed = 1, init_r = 0.5, cores = 1, refresh = 0)\n\
             cat(\"Posterior summary: 4 chains of 2000 iterations (the first half warm-up), 95% credible interval\\n\")\n\
             local({\n  s <- unclass(summary(bayes, probs = c((1 - 0.95) / 2, (1 + 0.95) / 2)))\n  \
             s <- s[!grepl(\"^mean_PPD\", rownames(s)) & rownames(s) != \"log-posterior\", , drop = FALSE]\n  \
             print(round(s, 4))\n  \
             if (any(s[, \"Rhat\"] > 1.05 | s[, \"n_eff\"] < 100, na.rm = TRUE))\n    \
             cat(\"\\nWarning: the chains have not converged (R-hat above 1.05 or fewer than \
             100 effective draws); do not trust these numbers.\\nRun more iterations or use a \
             tighter prior.\\n\")\n  invisible(NULL)\n})"
        );
        let mut v = values(
            &[
                ("family", "poisson"),
                ("prior", "normal"),
                ("prior_mean", "0.5"),
                ("prior_sd", "3"),
                ("level", "0.9"),
                ("save", "post"),
            ],
            &["age"],
        );
        v.set_flag("show_priors", true);
        let c = BAYESIAN_REGRESSION
            .generate(&ctx(), &v, Style::Base)
            .unwrap_or_default();
        assert!(
            c.starts_with(
                "post <- rstanarm::stan_glm(y ~ age, data = d, family = stats::poisson(), \
                 prior = rstanarm::normal(location = 0.5, scale = 3, autoscale = FALSE), "
            ),
            "{c}"
        );
        assert!(c.contains("90% credible interval"), "{c}");
        assert!(c.ends_with("\nprint(rstanarm::prior_summary(post))"), "{c}");
    }

    #[test]
    fn refuses_what_cannot_run() {
        assert!(widgets(&values(&[], &["age"])).is_empty());
        assert_eq!(widgets(&values(&[], &["y"])), vec!["x"]);
        assert_eq!(
            widgets(&values(&[("prior", "normal"), ("prior_sd", "0")], &["age"])),
            vec!["prior_sd"]
        );
        assert_eq!(
            widgets(&values(&[("chains", "0")], &["age"])),
            vec!["chains"]
        );
        assert_eq!(widgets(&values(&[("iter", "10")], &["age"])), vec!["iter"]);
        assert_eq!(widgets(&values(&[("seed", "1.5")], &["age"])), vec!["seed"]);
        assert_eq!(widgets(&values(&[("save", "d")], &["age"])), vec!["save"]);
        // a typed prior is not checked while the default is chosen
        assert!(widgets(&values(&[("prior_sd", "0")], &["age"])).is_empty());
        // lists must be numbers; scales and degrees of freedom above 0
        assert_eq!(
            widgets(&values(
                &[
                    ("prior", "student_t"),
                    ("prior_mean", "0, x"),
                    ("prior_sd", "1, -2"),
                    ("prior_df", "0"),
                    ("int_prior", "cauchy"),
                    ("int_sd", "")
                ],
                &["age"]
            )),
            vec!["prior_mean", "prior_sd", "prior_df", "int_sd"]
        );
        // an ordered logit takes no typed priors
        assert_eq!(
            widgets(&values(
                &[
                    ("family", "ordinal"),
                    ("prior", "normal"),
                    ("int_prior", "normal")
                ],
                &["age"]
            )),
            vec!["prior", "int_prior"]
        );
    }

    fn code(set: &[(&str, &str)], x: &[&str], flags: &[&str]) -> String {
        let mut v = values(set, x);
        for f in flags {
            v.set_flag(f, true);
        }
        let out = BAYESIAN_REGRESSION.generate(&ctx(), &v, Style::Base);
        assert!(out.is_ok(), "{out:?}");
        out.unwrap_or_default()
    }

    #[test]
    fn writes_families_and_priors() {
        let c = code(
            &[
                ("family", "neg_binomial"),
                ("prior", "student_t"),
                ("prior_mean", "0, 0.5"),
                ("prior_sd", "0.5,1"),
                ("prior_df", "3, 5"),
                ("int_prior", "cauchy"),
                ("int_mean", "0"),
                ("int_sd", "5"),
            ],
            &["age", "arm span"],
            &["prior_pd"],
        );
        assert!(
            c.starts_with(
                "bayes <- rstanarm::stan_glm(y ~ age + `arm span`, data = d, \
                 family = rstanarm::neg_binomial_2(), \
                 prior = rstanarm::student_t(df = c(3, 5), location = c(0, 0.5), scale = c(0.5, 1), autoscale = FALSE), \
                 prior_intercept = rstanarm::cauchy(location = 0, scale = 5, autoscale = FALSE), \
                 prior_PD = TRUE, chains = 4"
            ) && c.contains("Prior summary (the data are not used)"),
            "{c}"
        );
        let c = code(&[("family", "gamma")], &["age"], &[]);
        assert!(c.contains("family = stats::Gamma(link = \"log\")"), "{c}");
        let c = code(&[("family", "ordinal")], &["age"], &[]);
        assert!(
            c.starts_with(
                "bayes <- local({\n  d <- d\n  d[[\"y\"]] <- if (is.ordered(d[[\"y\"]]))"
            ) && c.contains("rstanarm::stan_polr(y ~ age, data = d, prior = NULL, chains = 4"),
            "{c}"
        );
    }

    #[test]
    fn writes_statas_priors_and_sampler() {
        let c = code(&[("iter", "6000")], &["age"], &["stata"]);
        assert!(
            c.starts_with(
                "bayes <- local({\n  f <- y ~ age\n  d <- stats::model.frame(f, data = d)\n  \
                 X <- stats::model.matrix(f, d); y <- stats::model.response(d)\n"
            ) && c.contains("prior_b <- function(b) sum(stats::dnorm(b, 0, 100, log = TRUE))")
                && c.contains("prior_c <- function(b) sum(stats::dnorm(b, 0, 100, log = TRUE))")
                && c.contains("stats::dgamma(1 / v, 0.01, rate = 0.01, log = TRUE) - log(v)")
                && c.contains("names(start) <- c(colnames(X), \"sigma2\")")
                && c.contains("warm <- 6000 %/% 2")
                && c.contains("acceptance = accepted / (6000 - warm), family = \"gaussian\"")
                && c.contains("class = \"bayes_mh\")\n})")
                && c.contains("Stata's priors, random-walk Metropolis-Hastings, 4 chains of 6000")
                && c.contains(
                    "Priors: coefficients: normal(0, 10000) (a variance); constant: \
                     normal(0, 10000) (a variance); sigma2: igamma(0.01, 0.01)"
                )
                && c.contains("rstan::monitor(bayes$draws, warmup = 0")
                && !c.contains("rstanarm::"),
            "{c}"
        );
        let c = code(
            &[
                ("family", "poisson"),
                ("prior", "student_t"),
                ("prior_mean", "0, 1"),
                ("prior_sd", "0.5"),
                ("prior_df", "3"),
                ("int_prior", "cauchy"),
                ("int_sd", "2"),
            ],
            &["age", "arm span"],
            &["stata"],
        );
        assert!(
            c.contains(
                "prior_b <- function(b) sum(stats::dt((b - c(0, 1)) / 0.5, 3, log = TRUE) - log(0.5))"
            ) && c.contains("prior_c <- function(b) sum(stats::dcauchy(b, 0, 2, log = TRUE))")
                && c.contains("if (!all(lengths(list(c(0, 1), 0.5, 3)) %in% c(1, sum(!int))))")
                && c.contains("sum(stats::dpois(y, exp(eta), log = TRUE)) + lpb(t)"),
            "{c}"
        );
        let c = code(&[("family", "neg_binomial")], &["age"], &["stata"]);
        assert!(
            c.contains("stats::dnorm(t[k + 1], 0, 100, log = TRUE)")
                && c.contains("\"lnalpha\"")
                && c.contains("lnalpha: normal(0, 10000)"),
            "{c}"
        );
        let c = code(&[("family", "gamma")], &["age"], &["stata"]);
        assert!(
            c.contains("sum(stats::dexp(y, exp(-eta), log = TRUE))")
                && c.contains("scale: fixed at 1"),
            "{c}"
        );
        let c = code(
            &[
                ("family", "ordinal"),
                ("prior", "normal"),
                ("prior_sd", "2"),
            ],
            &["age"],
            &["stata"],
        );
        assert!(
            c.contains("MASS::polr(y ~ X, Hess = TRUE)")
                && c.contains("if (any(diff(cuts) <= 0)) return(-Inf)")
                && c.contains("prior_b <- function(b) sum(stats::dnorm(b, 0, 2, log = TRUE))")
                && c.contains("cutpoints: flat (1)")
                && !c.contains("prior_c"),
            "{c}"
        );
        // no prior sampling, and no constant's prior on an ordered logit
        let mut v = values(&[], &["age"]);
        v.set_flag("stata", true);
        v.set_flag("prior_pd", true);
        assert_eq!(
            BAYESIAN_REGRESSION
                .problems(&ctx(), &v)
                .iter()
                .map(|p| p.widget)
                .collect::<Vec<_>>(),
            vec!["prior_pd"]
        );
        let mut v = values(&[("family", "ordinal"), ("int_prior", "normal")], &["age"]);
        v.set_flag("stata", true);
        assert_eq!(widgets(&v), vec!["int_prior"]);
        // the ordered logit takes typed coefficient priors with Stata's
        let mut v = values(&[("family", "ordinal"), ("prior", "normal")], &["age"]);
        v.set_flag("stata", true);
        assert!(widgets(&v).is_empty());
    }

    fn model_ctx() -> Context {
        Context {
            dataset: "my fit".into(),
            frames: Vec::new(),
            columns: Vec::new(),
        }
    }

    fn model_code(spec: &Spec, set: &[(&str, &str)]) -> Result<String, Vec<String>> {
        let mut v = spec.defaults(&model_ctx());
        for (id, text) in set {
            v.set_text(id, *text);
        }
        spec.generate(&model_ctx(), &v, Style::Base)
            .map_err(|p| p.iter().map(|p| p.message.clone()).collect())
    }

    #[test]
    fn model_menu_dialogs_write_rstanarm() {
        let c = model_code(
            &POSTERIOR_PREDICTIONS,
            &[("data", "d"), ("kind", "predict"), ("seed", "7")],
        )
        .unwrap_or_default();
        assert!(
            c.starts_with("d <- local({\n  fit <- `my fit`\n  nd <- d\n")
                && c.contains(
                    "rstanarm::posterior_predict(fit, newdata = nd[ok, , drop = FALSE], seed = 7)"
                )
                && c.contains("put(\"pred_upper\""),
            "{c}"
        );
        assert!(model_code(&POSTERIOR_PREDICTIONS, &[]).is_err());
        let c = model_code(&BAYES_CHECKS, &[]).unwrap_or_default();
        assert!(
            c.contains("print(plot(`my fit`, \"trace\"))")
                && c.contains("rstanarm::pp_check(`my fit`, nreps = 50, seed = 1)")
                && c.contains("stats <- list(mean = mean, sd = stats::sd)"),
            "{c}"
        );
        let c = model_code(&BAYES_LOO, &[("others", "m2, m3")]).unwrap_or_default();
        assert!(
            c.contains("`my fit`$loo <- rstanarm::loo(`my fit`)")
                && c.contains("m3$loo <- rstanarm::loo(m3)")
                && c.contains("loos <- list(`my fit` = `my fit`$loo, m2 = m2$loo, m3 = m3$loo)")
                && c.contains("loo::loo_compare(loos)"),
            "{c}"
        );
    }
}
