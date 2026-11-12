//! Statistics menu survival analysis (P6.3) end to end against a real R
//! (skipped with a note when none is found). Each dialog's code is built from
//! its spec and run on made-up data (40 rows in two arms, ties, censoring,
//! entry times, two centres, eight sites, two ages missing), and its numbers
//! are checked three ways:
//!
//! - against the formulas written out here in plain R: the product limit with
//!   Greenwood's variance and log-log intervals, the log-rank and Peto-Peto
//!   statistics, Newton-Raphson on Breslow's and Efron's partial likelihoods
//!   with the inverse information, and the exponential model as a Poisson GLM;
//! - against Stata 19's own results for the same data, recorded below (`sts`,
//!   `sts test`, `stcox` with each kind of ties, strata, entry times and
//!   clusters, `streg` with each distribution), to 1e-9;
//! - and against survival's report for the call written out by hand.
//!
//! Where R and Stata define a thing differently, the test says so: Stata's
//! robust Cox errors carry G/(G-1) (or N/(N-1)) and survival's none; Stata's
//! `sts test, peto` weights by Prentice's product of (1 - d/(n+1)) and
//! survdiff's rho = 1 by the Kaplan-Meier estimate; Stata's `estat phtest` is
//! Grambsch and Therneau's approximation and cox.zph (survival 3) the full
//! score test.
//!
//! The test records the R and survival versions it ran with (AGENTS.md
//! section 5), and checks R is the one `ENVIRONMENT` pins.

use std::time::{Duration, Instant};

use rcraft_catalog::survival::{COX, KAPLAN_MEIER, LOG_RANK, PARAMETRIC};
use rcraft_engine::{Engine, Item, Outcome, Source};
use rcraft_rcodegen::dialog::{ColumnInfo, Context, Spec, Style, VarKind, Widget};
use rcraft_rhost::{find_installs, Env, SessionConfig};

fn engine() -> Option<Engine> {
    let install = find_installs(&Env::from_process())
        .into_iter()
        .find(|i| i.is_supported());
    let Some(install) = install else {
        eprintln!("skipped: no supported R on this machine");
        return None;
    };
    Engine::start(SessionConfig::new(install)).ok()
}

fn settle(e: &mut Engine) {
    let limit = Instant::now() + Duration::from_secs(120);
    loop {
        e.poll(Duration::from_millis(50));
        if !e.is_working() {
            break;
        }
        assert!(Instant::now() < limit, "the engine did not settle");
    }
}

/// What a run printed, and the plots it drew.
fn run(e: &mut Engine, source: Source, code: &str) -> (Outcome, String, usize) {
    let id = e.submit(source, code);
    assert!(id.is_ok(), "{id:?}");
    let id = id.unwrap_or_default();
    settle(e);
    let outcome = e
        .history()
        .entries()
        .last()
        .and_then(|h| h.outcome)
        .unwrap_or(Outcome::Error);
    let mut plots = 0;
    let text = e
        .results()
        .blocks()
        .iter()
        .find(|b| b.command == id)
        .map(|b| {
            b.items
                .iter()
                .map(|i| match i {
                    Item::Text { text, .. } => text.clone(),
                    Item::Warning(w) => format!("warning: {w}\n"),
                    Item::Error { message, .. } => format!("error: {message}\n"),
                    Item::Notice(n) => n.clone(),
                    Item::Plot { .. } => {
                        plots += 1;
                        String::new()
                    }
                })
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default();
    (outcome, text, plots)
}

/// The R and survival versions, recorded.
fn versions(e: &mut Engine) {
    let (outcome, text, _) = run(
        e,
        Source::Typed,
        "cat(paste(R.version$major, R.version$minor, sep = '.'), \
         as.character(packageVersion('survival')))",
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let mut words = text.split_whitespace();
    let (r, survival) = (words.next().unwrap_or(""), words.next().unwrap_or(""));
    let pinned = include_str!("../../../ENVIRONMENT")
        .lines()
        .find_map(|l| l.strip_prefix("R_VERSION="))
        .unwrap_or("")
        .trim();
    eprintln!("ran with R {r} and survival {survival} (ENVIRONMENT pins R {pinned})");
    if r != pinned {
        eprintln!("note: this R is not the pinned one, so the run is not a Verified one");
    }
}

/// Forty rows in two arms, 35 events with ties, an entry time before each
/// exit, two centres, eight sites, and age missing in rows 6 and 29.
const MAKE: &str = "d <- local({ \
     i <- 1:40; \
     arm <- factor(rep(c('a', 'b'), each = 20)); \
     age <- 40 + (i * 7) %% 30; \
     time <- round(5 + 30 * ((i * 37) %% 41) / 41 * ifelse(arm == 'b', 1.6, 1) + age / 20, 0); \
     died <- as.numeric((i * 11) %% 7 != 0); \
     start <- round(time * (i %% 3) / 10, 1); \
     centre <- c('x', 'y')[1 + i %% 2]; \
     site <- 1 + (i * 3) %% 8; \
     age[c(6, 29)] <- NA; \
     data.frame(time, died, arm, age, start, centre, site) })";

/// The formulas in plain R: `dc` the rows with an age, `X` its arm b and age
/// columns, and `cox_hand`, Newton-Raphson on Breslow's or Efron's partial
/// likelihood, returning the estimate and the inverse information's errors.
const ORACLE: &str = "dc <- d[!is.na(d$age), ]; X <- cbind(as.numeric(dc$arm == 'b'), dc$age); \
     G <- length(unique(dc$site)); \
     cox_hand <- function(X, tm, ev, efron) { b <- rep(0, ncol(X)); \
       for (it in 1:50) { U <- rep(0, ncol(X)); I <- matrix(0, ncol(X), ncol(X)); \
         for (t in sort(unique(tm[ev == 1]))) { R <- tm >= t; D <- tm == t & ev == 1; m <- sum(D); \
           r <- as.vector(exp(X %*% b)); \
           for (l in 0:(m - 1)) { w <- r * (R - (if (efron) l / m else 0) * D); s0 <- sum(w); \
             s1 <- colSums(w * X); U <- U - s1 / s0; \
             I <- I + crossprod(X, w * X) / s0 - tcrossprod(s1) / s0^2 }; \
           U <- U + colSums(X[D, , drop = FALSE]) }; \
         step <- solve(I, U); b <- b + step; if (max(abs(step)) < 1e-13) break }; \
       list(b = b, se = sqrt(diag(solve(I)))) }";

fn context() -> Context {
    Context {
        dataset: "d".into(),
        frames: Vec::new(),
        columns: [
            ("time", VarKind::Number),
            ("died", VarKind::Number),
            ("arm", VarKind::Factor),
            ("age", VarKind::Number),
            ("start", VarKind::Number),
            ("centre", VarKind::Text),
            ("site", VarKind::Number),
        ]
        .into_iter()
        .map(|(name, kind)| ColumnInfo {
            name: name.into(),
            kind,
        })
        .collect(),
    }
}

fn dialog(
    e: &mut Engine,
    spec: &Spec,
    set: &[(&str, &str)],
    lists: &[(&str, &[&str])],
    flags: &[&str],
) -> (Outcome, String, usize) {
    let (made, text, _) = run(e, Source::Typed, MAKE);
    assert_eq!(made, Outcome::Ok, "{text}");
    let ctx = context();
    let mut values = spec.defaults(&ctx);
    for (id, text) in set {
        values.set_text(id, *text);
    }
    for (id, names) in lists {
        values.set_list(id, names.iter().map(|n| n.to_string()).collect());
    }
    for w in spec.widgets {
        if let Widget::Check { id, .. } = w {
            values.set_flag(id, flags.contains(id));
        }
    }
    let code = spec.generate(&ctx, &values, Style::Base);
    assert!(code.is_ok(), "{code:?}");
    let code = code.unwrap_or_default();
    run(e, Source::Menu(spec.menu.into()), &code)
}

/// A report with spaces tidied, blank lines and the lines that echo the call
/// (which name the model or the data) left out.
fn report(text: &str) -> Vec<String> {
    text.lines()
        .filter(|l| {
            let l = l.trim_start();
            !(l.starts_with("Call:")
                || l.starts_with("survival::")
                || l.starts_with("survfit(")
                || l.starts_with("coxph(")
                || l.starts_with("survreg(")
                || l.starts_with("survdiff(")
                || l.starts_with("warning:"))
        })
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|l| !l.is_empty())
        .collect()
}

/// Runs R that stops unless every one of `exprs` holds.
fn holds(e: &mut Engine, exprs: &[String]) {
    let exprs = exprs.join(", ");
    let (outcome, text, _) = run(e, Source::Typed, &format!("stopifnot({exprs})"));
    assert_eq!(outcome, Outcome::Ok, "{exprs}\n{text}");
}

fn define(e: &mut Engine, code: &str) {
    let (outcome, text, _) = run(e, Source::Typed, code);
    assert_eq!(outcome, Outcome::Ok, "{code}\n{text}");
}

/// `all.equal` to `tol`, unnamed.
fn within(a: &str, b: &str, tol: &str) -> String {
    format!("isTRUE(all.equal(unname(as.vector({a})), unname(as.vector({b})), tolerance = {tol}))")
}

/// `all.equal` to 1e-9.
fn same(a: &str, b: &str) -> String {
    within(a, b, "1e-9")
}

#[test]
fn kaplan_meier_is_the_product_limit_and_statas() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    versions(&mut e);
    let (outcome, text, plots) = dialog(
        &mut e,
        &KAPLAN_MEIER,
        &[("time", "time"), ("event", "died")],
        &[],
        &["draw"],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert_eq!(plots, 1, "{text}");
    let (_, expected, _) = run(
        &mut e,
        Source::Typed,
        "k <- survival::survfit(survival::Surv(time, died) ~ 1, data = d, conf.int = 0.95, \
         conf.type = 'log-log'); print(k)",
    );
    assert!(report(&expected).len() >= 2, "{expected}");
    assert_eq!(report(&text), report(&expected), "{text}");
    // at each event time the product of (1 - d/n) so far, Greenwood's
    // variance S^2 sum d/(n(n - d)), and the interval S^exp(-+z se / log S)
    // on the log-log scale, for every time where 0 < S < 1
    holds(
        &mut e,
        &[
            "local({ s <- summary(km); tt <- sort(unique(d$time[d$died == 1])); \
               nr <- vapply(tt, function(t) sum(d$time >= t), 0); \
               dd <- vapply(tt, function(t) sum(d$time == t & d$died == 1), 0); \
               S <- cumprod(1 - dd / nr); g <- cumsum(dd / (nr * (nr - dd))); z <- qnorm(0.975); \
               ok <- S > 0; q <- z * sqrt(g[ok]) / log(S[ok]); \
               isTRUE(all.equal(s$time, tt, tolerance = 0)) && \
               isTRUE(all.equal(s$surv, S, tolerance = 1e-12)) && \
               isTRUE(all.equal(s$std.err[ok], S[ok] * sqrt(g[ok]), tolerance = 1e-12)) && \
               isTRUE(all.equal(s$lower[ok], S[ok]^exp(-q), tolerance = 1e-12)) && \
               isTRUE(all.equal(s$upper[ok], S[ok]^exp(q), tolerance = 1e-12)) })"
                .to_string(),
            // Stata 19: sts generate s = s se = se(s) lb = lb(s) ub = ub(s), at
            // the first five event times (8, 9, 11, 12, 15)
            same(
                "summary(km)$std.err[1:5]",
                "c(.0246855220726644, .0344601218802255, .0474341649025257, .0564579489531811, .0632455532033676)",
            ),
            same(
                "summary(km)$lower[1:5]",
                "c(.835485459111506, .814546120144527, .755141054497997, .696236216579165, .639644761459237)",
            ),
            same(
                "summary(km)$upper[1:5]",
                "c(.99644018301373, .987255523860308, .961245191640206, .929648166286604, .894551195032394)",
            ),
        ],
    );

    // by arm: two curves, one picture with its legend
    let (outcome, text, plots) = dialog(
        &mut e,
        &KAPLAN_MEIER,
        &[
            ("time", "time"),
            ("event", "died"),
            ("group", "arm"),
            ("save", "km2"),
        ],
        &[],
        &["draw"],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert_eq!(plots, 1, "{text}");
    holds(
        &mut e,
        &["identical(names(km2$strata), c('arm=a', 'arm=b'))".to_string()],
    );
}

#[test]
fn log_rank_and_peto_are_the_statistics_by_hand() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    versions(&mut e);
    let (outcome, text, _) = dialog(
        &mut e,
        &LOG_RANK,
        &[("time", "time"), ("event", "died"), ("group", "arm")],
        &[],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let (_, expected, _) = run(
        &mut e,
        Source::Typed,
        "print(survival::survdiff(survival::Surv(time, died) ~ arm, data = d, rho = 0))",
    );
    assert_eq!(report(&text), report(&expected), "{text}");
    let (outcome, text, _) = dialog(
        &mut e,
        &LOG_RANK,
        &[
            ("time", "time"),
            ("event", "died"),
            ("group", "arm"),
            ("rho", "1"),
        ],
        &[],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    // (sum w (O - E))^2 / sum w^2 V for arm a over the distinct event times,
    // w = 1 (log-rank) or the pooled Kaplan-Meier S(t-) (survdiff's rho = 1);
    // with Prentice's weights, prod (1 - d/(n + 1)) up to t, it is Stata's
    // sts test, peto (6.20077570067929) instead
    define(
        &mut e,
        "chisq <- function(weight) { tt <- sort(unique(d$time[d$died == 1])); \
           u <- 0; v <- 0; S <- 1; P <- 1; \
           for (t in tt) { r <- d$time >= t; n <- sum(r); n1 <- sum(r & d$arm == 'a'); \
             k <- sum(d$time == t & d$died == 1); o <- sum(d$time == t & d$died == 1 & d$arm == 'a'); \
             w <- switch(weight, one = 1, km = S, prentice = P * (1 - k / (n + 1))); \
             u <- u + w * (o - k * n1 / n); \
             if (n > 1) v <- v + w^2 * k * (n1 / n) * (1 - n1 / n) * (n - k) / (n - 1); \
             S <- S * (1 - k / n); P <- P * (1 - k / (n + 1)) }; u^2 / v }",
    );
    holds(
        &mut e,
        &[
            same(
                "survival::survdiff(survival::Surv(time, died) ~ arm, data = d)$chisq",
                "chisq('one')",
            ),
            same(
                "survival::survdiff(survival::Surv(time, died) ~ arm, data = d, rho = 1)$chisq",
                "chisq('km')",
            ),
            same("chisq('prentice')", "6.20077570067929"),
            // Stata 19: sts test arm
            same(
                "survival::survdiff(survival::Surv(time, died) ~ arm, data = d)$chisq",
                "9.50583311247835",
            ),
        ],
    );
}

#[test]
fn cox_is_the_partial_likelihood_and_statas() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    versions(&mut e);
    let (outcome, text, _) = dialog(
        &mut e,
        &COX,
        &[("time", "time"), ("event", "died")],
        &[("x", &["arm", "age"])],
        &["zph"],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let (_, expected, _) = run(
        &mut e,
        Source::Typed,
        "m <- survival::coxph(survival::Surv(time, died) ~ arm + age, data = d, ties = 'efron'); \
         print(summary(m, conf.int = 0.95)); print(survival::cox.zph(m))",
    );
    assert!(report(&expected).len() > 10, "{expected}");
    assert_eq!(report(&text), report(&expected), "{text}");
    for (ties, save) in [("breslow", "cb"), ("exact", "cx")] {
        let (outcome, text, _) = dialog(
            &mut e,
            &COX,
            &[
                ("time", "time"),
                ("event", "died"),
                ("ties", ties),
                ("save", save),
            ],
            &[("x", &["arm", "age"])],
            &[],
        );
        assert_eq!(outcome, Outcome::Ok, "{text}");
    }
    define(&mut e, ORACLE);
    holds(
        &mut e,
        &[
            same(
                "c(coef(cox), sqrt(diag(vcov(cox))))",
                "unlist(cox_hand(X, dc$time, dc$died, efron = TRUE))",
            ),
            same(
                "c(coef(cb), sqrt(diag(vcov(cb))))",
                "unlist(cox_hand(X, dc$time, dc$died, efron = FALSE))",
            ),
            // Stata 19: stcox b age, efron | breslow | exactp
            same(
                "c(coef(cox), sqrt(diag(vcov(cox))))",
                "c(-1.30044056059355, -.0142006923607791, .42997633061103, .0210286021134452)",
            ),
            same(
                "c(coef(cb), sqrt(diag(vcov(cb))))",
                "c(-1.28409462670782, -.0136226053651508, .430014463307846, .0209238027223064)",
            ),
            same(
                "c(coef(cx), sqrt(diag(vcov(cx))))",
                "c(-1.32079095403218, -.0140464231633298, .435645435312931, .0212529289358215)",
            ),
        ],
    );

    // clusters, strata and entry times. survival's robust errors carry no
    // small-sample factor and Stata's G/(G - 1), so Stata's are survival's
    // times sqrt(G/(G - 1)); the coefficients and model errors are the same
    let (outcome, text, _) = dialog(
        &mut e,
        &COX,
        &[
            ("time", "time"),
            ("event", "died"),
            ("cluster", "site"),
            ("save", "cc"),
        ],
        &[("x", &["arm", "age"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let (outcome, text, _) = dialog(
        &mut e,
        &COX,
        &[
            ("time", "time"),
            ("event", "died"),
            ("strata", "centre"),
            ("save", "cs"),
        ],
        &[("x", &["age"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let (outcome, text, _) = dialog(
        &mut e,
        &COX,
        &[
            ("time", "time"),
            ("event", "died"),
            ("entry", "start"),
            ("strata", "centre"),
            ("cluster", "site"),
            ("save", "ce"),
        ],
        &[("x", &["age"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    define(&mut e, ORACLE);
    holds(
        &mut e,
        &[
            "identical(attr(ce$y, 'type'), 'counting')".to_string(),
            // Stata 19: stcox b age, efron vce(cluster site)
            same(
                "sqrt(diag(vcov(cc)) * G / (G - 1))",
                "c(.274552022053807, .0140228803443851)",
            ),
            same("coef(cc)", "coef(cox)"),
            // Stata 19: stcox age, efron strata(centre)
            same(
                "c(coef(cs), sqrt(vcov(cs)))",
                "c(-.0155421955294665, .0211779354350675)",
            ),
            // Stata 19: stset time, failure(died) enter(time start); stcox
            // age, efron strata(centre) vce(cluster site)
            same(
                "c(coef(ce), sqrt(vcov(ce) * G / (G - 1)))",
                "c(-.0155470568895314, .0195450196205295)",
            ),
        ],
    );
}

#[test]
fn parametric_models_are_streg() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    versions(&mut e);
    let (outcome, text, _) = dialog(
        &mut e,
        &PARAMETRIC,
        &[("time", "time"), ("event", "died"), ("dist", "exponential")],
        &[("x", &["arm", "age"])],
        &["ratios"],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let (_, expected, _) = run(
        &mut e,
        Source::Typed,
        "m <- survival::survreg(survival::Surv(time, died) ~ arm + age, data = d, dist = 'exponential'); \
         print(summary(m)); cat('\\nTime ratios\\n'); \
         print(exp(cbind(time_ratio = stats::coef(m), stats::confint(m, level = 0.95))))",
    );
    assert_eq!(report(&text), report(&expected), "{text}");
    define(&mut e, ORACLE);
    // the exponential AFT model is a Poisson GLM of the events with log time
    // as offset, its coefficients' signs turned; survreg stops at its own
    // relative tolerance (1e-9), so it is held to 1e-8
    holds(
        &mut e,
        &[within(
            "coef(aft)",
            "-coef(glm(died ~ arm + age + offset(log(time)), family = poisson, data = dc, \
               control = glm.control(epsilon = 1e-14, maxit = 100)))",
            "1e-8",
        )],
    );
    for dist in ["weibull", "lognormal", "loglogistic"] {
        let (outcome, text, _) = dialog(
            &mut e,
            &PARAMETRIC,
            &[
                ("time", "time"),
                ("event", "died"),
                ("dist", dist),
                ("save", dist),
            ],
            &[("x", &["arm", "age"])],
            &[],
        );
        assert_eq!(outcome, Outcome::Ok, "{text}");
    }
    // each model's (arm b, age, constant) coefficients and errors, then the
    // log of its scale and that error. Stata 19: streg b age, d(weibull) time
    // (whose ln_p is minus survreg's log scale), d(exponential) time,
    // d(lognormal) (ln_sig), d(loglogistic) (ln_gam); both programs stop at
    // their own tolerances, so the figures agree to about 1e-9 and are held
    // to 1e-8
    let fit = |m: &str| {
        format!(
            "c(coef({m})[c(2, 3, 1)], sqrt(diag(vcov({m})))[c(2, 3, 1)], \
             if (length(coef({m})) < ncol(vcov({m}))) c(log({m}$scale), sqrt(vcov({m})[4, 4])))"
        )
    };
    holds(
        &mut e,
        &[
            within(
                &fit("aft"),
                "c(.492448471687336, .00965179926616794, 2.67648548164385, \
                   .348688290923803, .0204448546260555, 1.14282877564802)",
                "1e-8",
            ),
            within(
                &fit("weibull"),
                "c(.447024167291494, .00491436842111037, 2.98160971038593, \
                   .125038425116652, .00753139713867603, .419454621556066, \
                   -1.02492428716786, .143159719602074)",
                "1e-8",
            ),
            within(
                &fit("lognormal"),
                "c(.463565207628942, .0124421276465664, 2.35882548511727, \
                   .160471851489594, .00916746678998833, .517890168441257, \
                   -.73490151015812, .123680757530595)",
                "1e-8",
            ),
            within(
                &fit("loglogistic"),
                "c(.459442249213437, .0100537079972889, 2.52474859822369, \
                   .165073203885982, .00947951705393789, .544758643436734, \
                   -1.26402277944453, .142342110760198)",
                "1e-8",
            ),
        ],
    );
}
