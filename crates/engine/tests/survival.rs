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

use rcraft_catalog::postest::{self, COMPARE_MODELS, MODEL_FIT};
use rcraft_catalog::survival::{
    COMPETING_RISKS, COX, COX_CURVES, KAPLAN_MEIER, LOG_RANK, PARAMETRIC,
};
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
     cause <- ifelse(died == 0, 0, ifelse(i %% 3 == 0, 2, 1)); \
     age[c(6, 29)] <- NA; \
     data.frame(time, died, arm, age, start, centre, site, cause, time2 = time + i / 100) })";

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
            ("cause", VarKind::Number),
            ("time2", VarKind::Number),
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

/// Runs a dialog on an R object that is not `d`: the data frame or model
/// named in `dataset`, already made by an earlier step.
fn dialog_on(
    e: &mut Engine,
    dataset: &str,
    columns: &[(&str, VarKind)],
    spec: &Spec,
    set: &[(&str, &str)],
    lists: &[(&str, &[&str])],
    flags: &[&str],
) -> (Outcome, String) {
    let ctx = Context {
        dataset: dataset.into(),
        frames: Vec::new(),
        columns: columns
            .iter()
            .map(|(name, kind)| ColumnInfo {
                name: (*name).into(),
                kind: *kind,
            })
            .collect(),
    };
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
    let (outcome, text, _) = run(e, Source::Menu(spec.menu.into()), &code.unwrap_or_default());
    (outcome, text)
}

#[test]
fn competing_risks_match_stcrreg() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    versions(&mut e);
    let (outcome, text, _) = dialog(
        &mut e,
        &COMPETING_RISKS,
        &[("time", "time"), ("status", "cause"), ("ties", "breslow")],
        &[("x", &["arm", "age"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(text.contains("subhazard ratio"), "{text}");
    // the same without tied times (time2 is time plus a row-number fraction,
    // so no two times are equal)
    let (outcome, text, _) = dialog(
        &mut e,
        &COMPETING_RISKS,
        &[
            ("time", "time2"),
            ("status", "cause"),
            ("ties", "breslow"),
            ("save", "fg2"),
        ],
        &[("x", &["arm", "age"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    define(&mut e, ORACLE);
    // survival::finegray's rows and coxph with the subdistribution weights and
    // each subject a cluster, on the rows with no missing value: the dialog's
    // rows are finegray's when no censoring time equals another time
    define(
        &mut e,
        "fg0 <- local({ dd <- d[!is.na(d$age), ]; dd$r <- seq_len(nrow(dd)); \
           survival::finegray(survival::Surv(time2, factor(cause)) ~ arm + age + r, data = dd, \
             etype = '1', id = r) }); \
         m0 <- survival::coxph(survival::Surv(fgstart, fgstop, fgstatus) ~ arm + age, data = fg0, \
           weights = fgwt, ties = 'breslow', cluster = r); \
         dc2 <- dc; dc2$time <- dc2$time2",
    );
    // Fine and Gray's partial likelihood by hand, Breslow's ties, with the
    // censoring distribution G taken just before each time (the convention
    // Stata's stcrreg follows): a subject with the competing event stays in
    // every later risk set with weight G(t-) / G(its time-)
    define(
        &mut e,
        "gfun <- function(t, tm, cs) { g <- 1; \
           for (u in sort(unique(tm[cs == 0]))) \
             if (u < t) g <- g * (1 - sum(tm == u & cs == 0) / sum(tm >= u)); g }; \
         fg_hand <- function(X, tm, cs) { b <- rep(0, ncol(X)); \
           gt <- sapply(tm, gfun, tm = tm, cs = cs); \
           for (it in 1:100) { U <- rep(0, ncol(X)); I <- matrix(0, ncol(X), ncol(X)); \
             for (t in sort(unique(tm[cs == 1]))) { \
               w <- ifelse(tm >= t, 1, ifelse(cs == 2, gfun(t, tm, cs) / gt, 0)); \
               r <- as.vector(w * exp(X %*% b)); D <- tm == t & cs == 1; \
               s0 <- sum(r); s1 <- colSums(r * X); \
               U <- U + colSums(X[D, , drop = FALSE]) - sum(D) * s1 / s0; \
               I <- I + sum(D) * (crossprod(X, r * X) / s0 - tcrossprod(s1) / s0^2) }; \
             st <- solve(I, U); b <- b + st; if (max(abs(st)) < 1e-12) break }; b }",
    );
    holds(
        &mut e,
        &[
            // 22 failures (Stata's No. failed) from 38 subjects, each weight
            // between 0 and 1, and the rows named after the model
            "sum(fg_data$fgstatus) == 22 && length(unique(fg_data$.row)) == 38".to_string(),
            "all(fg_data$fgwt > 0 & fg_data$fgwt <= 1)".to_string(),
            "sum(fg2_data$fgstatus) == 22 && length(unique(fg2_data$.row)) == 38".to_string(),
            // without ties the rows and coefficients are finegray's
            same("coef(fg2)", "coef(m0)"),
            // the coefficients are Fine and Gray's estimator by hand, with
            // and without tied times
            same("coef(fg)", "fg_hand(X, dc$time, dc$cause)"),
            same("coef(fg2)", "fg_hand(X, dc2$time, dc2$cause)"),
            // Stata 19: stset time, failure(cause==1); stcrreg armb age,
            // compete(cause==2), then the same with time2 (the figures agree
            // to what Stata's own iteration keeps)
            within(
                "coef(fg)",
                "c(-.640945403838436, .0239356115333248)",
                "1e-7",
            ),
            within(
                "coef(fg2)",
                "c(-.653805260618764, .0251259667836071)",
                "1e-7",
            ),
            // the robust errors: Fine and Gray's sandwich with the term for
            // estimating the censoring distribution (a censoring time equal to
            // another time counts, as in Stata), times n / (n - 1), equal
            // stcrreg's to 1e-6 with and without tied times, and not the plain
            // cluster sandwich coxph gives
            within(
                "sqrt(diag(vcov(fg2)))",
                "c(.431533559256939, .0245366406)",
                "1e-6",
            ),
            within(
                "sqrt(diag(vcov(fg)))",
                "c(.432947492509726, .0240724012871915)",
                "1e-6",
            ),
            "!isTRUE(all.equal(sqrt(diag(vcov(fg2))), sqrt(diag(vcov(m0))), tolerance = 1e-3))"
                .to_string(),
        ],
    );

    // an effect that changes with time: age also times log(t), as stcrreg,
    // tvc(age) texp(ln(_t)); the rows are cut at every event time
    let (outcome, text, _) = dialog(
        &mut e,
        &COMPETING_RISKS,
        &[
            ("time", "time"),
            ("status", "cause"),
            ("ties", "breslow"),
            ("texp", "log(t)"),
            ("save", "fgt"),
        ],
        &[("x", &["arm", "age"]), ("tvc", &["age"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(text.contains("tvc_age"), "{text}");
    // Fine and Gray's estimator by hand with the covariates at each event time
    define(
        &mut e,
        "fg_tvc <- function(X, Z, g, tm, cs) { b <- rep(0, ncol(X) + ncol(Z)); \
           gt <- sapply(tm, gfun, tm = tm, cs = cs); \
           for (it in 1:100) { U <- rep(0, length(b)); I <- matrix(0, length(b), length(b)); \
             for (t in sort(unique(tm[cs == 1]))) { Xt <- cbind(X, Z * g(t)); \
               w <- ifelse(tm >= t, 1, ifelse(cs == 2, gfun(t, tm, cs) / gt, 0)); \
               r <- as.vector(w * exp(Xt %*% b)); D <- tm == t & cs == 1; \
               s0 <- sum(r); s1 <- colSums(r * Xt); \
               U <- U + colSums(Xt[D, , drop = FALSE]) - sum(D) * s1 / s0; \
               I <- I + sum(D) * (crossprod(Xt, r * Xt) / s0 - tcrossprod(s1) / s0^2) }; \
             st <- solve(I, U); b <- b + st; if (max(abs(st)) < 1e-12) break }; b }",
    );
    holds(
        &mut e,
        &[
            same(
                "coef(fgt)",
                "fg_tvc(X, cbind(dc$age), log, dc$time, dc$cause)",
            ),
            same("fgt_data$tvc_age", "fgt_data$age * log(fgt_data$fgstop)"),
            // Stata 19: stcrreg armb age, compete(cause==2) tvc(age)
            // texp(ln(_t)): armb, age, age times ln(_t), and their robust errors
            within(
                "coef(fgt)",
                "c(-.640930954418586, .098391950212153, -.0232352615126033)",
                "1e-7",
            ),
            within(
                "sqrt(diag(vcov(fgt)))",
                "c(.428202198789318, .128693633331725, .0414195497789158)",
                "1e-6",
            ),
        ],
    );
    // Stata's stcrreg takes no strata(), so the dialog offers none; a variable
    // in tvc() need not have a main effect, as in Stata
    let mut values = COMPETING_RISKS.defaults(&context());
    values.set_text("time", "time");
    values.set_text("status", "cause");
    values.set_list("x", vec!["arm".into()]);
    values.set_list("tvc", vec!["age".into()]);
    assert!(COMPETING_RISKS.problems(&context(), &values).is_empty());
    assert!(!COMPETING_RISKS
        .widgets
        .iter()
        .any(|w| matches!(w, Widget::Variable { id: "strata", .. })));
}

#[test]
fn cox_curves_are_the_baseline_to_the_exp_xb_and_statas() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    versions(&mut e);
    for (ties, save) in [("breslow", "cb"), ("exact", "cx"), ("efron", "ce")] {
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
    let times = "10, 20, 30, 40";
    let (outcome, text, plots) = dialog(
        &mut e,
        &COX_CURVES,
        &[
            ("model", "cb"),
            ("values", "arm = 'b', age = 50"),
            ("times", times),
            ("save", "c1"),
        ],
        &[],
        &["draw"],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert_eq!(plots, 1, "{text}");
    assert!(text.contains("survival"), "{text}");
    for (model, save, values) in [
        ("cb", "c2", ""),
        ("cx", "c3", "arm = 'b', age = 50"),
        ("ce", "c5", "arm = 'b', age = 50"),
        ("ce", "c6", ""),
        ("cx", "c7", ""),
        ("cb", "c4", "arm = c('a', 'b'), age = c(40, 60)"),
    ] {
        let (outcome, text, plots) = dialog(
            &mut e,
            &COX_CURVES,
            &[
                ("model", model),
                ("values", values),
                ("times", times),
                ("save", save),
            ],
            &[],
            &["draw"],
        );
        assert_eq!(outcome, Outcome::Ok, "{text}");
        assert_eq!(plots, 1, "{text}");
    }
    define(&mut e, ORACLE);
    // Breslow's baseline hazard h_j = d_j / (sum over the risk set of exp(xb)) at
    // each event time (Breslow's), the baseline survival the product over event
    // times up to t of the factors a_j that solve the likelihood equation
    // sum over the events at t_j of r / (1 - a^r) = sum over the risk set of r
    // (a_j = (1 - r / sum r)^(1 / r) for one event; Kalbfleisch and Prentice's
    // product limit, which Stata's predict, basesurv gives), and then
    // S(t | x) = S0(t)^exp(xb); with no values given, the covariates are the
    // means of the design matrix's columns (a factor's 0 and 1 column too)
    define(
        &mut e,
        "cox_surv <- function(m, x, at) { r <- as.vector(exp(X %*% coef(m))); \
           tt <- sort(unique(dc$time[dc$died == 1])); \
           alpha <- vapply(tt, function(t) { D <- dc$time == t & dc$died == 1; R <- dc$time >= t; \
             if (sum(D) == 1) (1 - r[D] / sum(r[R]))^(1 / r[D]) \
             else uniroot(function(a) sum(r[D] / (1 - a^r[D])) - sum(r[R]), \
                          c(1e-12, 1 - 1e-12), tol = 1e-14)$root }, 0); \
           vapply(at, function(a) exp(sum(log(alpha[tt <= a])) * exp(sum(coef(m) * x))), 0) }; \
         ts <- c(10, 20, 30, 40); xbar <- colMeans(X); \
         surv_at <- function(f) as.vector(summary(f, times = ts)$surv)",
    );
    holds(
        &mut e,
        &[
            same("surv_at(c1)", "cox_surv(cb, c(1, 50), ts)"),
            same("surv_at(c2)", "cox_surv(cb, xbar, ts)"),
            // curves for two values of each predictor, one per row of values
            "identical(dim(c4$surv)[2], 2L)".to_string(),
            same(
                "summary(c4, times = 20)$surv[1, ]",
                "c(cox_surv(cb, c(0, 40), 20), cox_surv(cb, c(1, 60), 20))",
            ),
            // Stata 19: stcox armb age, breslow; predict bs, basesurv; at the
            // last bs at or before each time, bs^exp(xb) for armb = 1, age = 50
            // and for the means (armb .5, the mean age)
            within(
                "surv_at(c1)",
                "c(.97502622070734, .86145734539009, .651790904025156, .374152445685131)",
                "1e-8",
            ),
            within(
                "surv_at(c2)",
                "c(.955709221248276, .765578093579227, .464544371523078, .171884382207356)",
                "1e-8",
            ),
            // the same with exactp ties
            within(
                "surv_at(c3)",
                "c(.975676242432655, .86473280436271, .658331995816286, .379456865751947)",
                "1e-8",
            ),
            within(
                "surv_at(c7)",
                "c(.956148552199715, .767467267385156, .467070156239883, .171253904597434)",
                "1e-8",
            ),
            // and with efron ties, where Stata's baseline is Efron's: stcox
            // armb age, efron; predict bs, basesurv
            within(
                "surv_at(c5)",
                "c(.975280521073835, .862713330611936, .654555980101737, .376014294082014)",
                "1e-8",
            ),
            within(
                "surv_at(c6)",
                "c(.955912333672961, .766426440824921, .466066765358928, .171702559124698)",
                "1e-8",
            ),
            // the confidence limits keep their order around the curve
            "all(summary(c5, times = ts)$lower < summary(c5, times = ts)$surv & \
             summary(c5, times = ts)$surv < summary(c5, times = ts)$upper)"
                .to_string(),
        ],
    );
}

#[test]
fn cox_hazard_and_cumulative_hazard_are_stcurves() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    versions(&mut e);
    for (ties, save) in [("breslow", "cb"), ("exact", "cx"), ("efron", "ce")] {
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
    // the cumulative hazard at arm b, age 50, for each kind of ties
    for (model, save) in [("cb", "h1"), ("cx", "h2"), ("ce", "h3")] {
        let (outcome, text, plots) = dialog(
            &mut e,
            &COX_CURVES,
            &[
                ("model", model),
                ("kind", "cumhaz"),
                ("values", "arm = 'b', age = 50"),
                ("times", "8, 11, 29, 40, 54"),
                ("save", save),
            ],
            &[],
            &["draw"],
        );
        assert_eq!(outcome, Outcome::Ok, "{text}");
        assert_eq!(plots, 1, "{text}");
        assert!(text.contains("cumhaz"), "{text}");
    }
    // the smoothed hazard: at arm b, age 50 (Breslow and exact), at the means and
    // for two rows of values (Efron), and with a width of 5
    for (model, save, values, width) in [
        ("cb", "z1", "arm = 'b', age = 50", ""),
        ("cx", "z2", "arm = 'b', age = 50", ""),
        ("ce", "z3", "", ""),
        ("ce", "z4", "arm = c('a', 'b'), age = c(45, 60)", ""),
        ("cb", "z5", "arm = 'b', age = 50", "5"),
    ] {
        let (outcome, text, plots) = dialog(
            &mut e,
            &COX_CURVES,
            &[
                ("model", model),
                ("kind", "hazard"),
                ("values", values),
                ("width", width),
                ("times", "20, 50"),
                ("save", save),
            ],
            &[],
            &["draw"],
        );
        assert_eq!(outcome, Outcome::Ok, "{text}");
        assert_eq!(plots, 1, "{text}");
        assert!(text.contains("Epanechnikov kernel of width"), "{text}");
    }
    define(&mut e, ORACLE);
    // Breslow's cumulative hazard, the sum over event times of d / (sum over the
    // risk set of exp(xb)), times exp(xb); the hazard contributions of a curve,
    // one less its survival ratio at each event time (Breslow and exact ties),
    // or 1 - (1 - h0)^exp(xb) with Efron's h0, the sum over the tied events of
    // 1 - (1 - r / d_l)^(1 / r); the Epanechnikov kernel smoother; the width
    // 0.9 min(sd, IQR / 1.349) n^(-1/5) of the distinct event times
    define(
        &mut e,
        "tt <- sort(unique(dc$time[dc$died == 1])); \
         breslow_H <- function(m, x, at) { r <- as.vector(exp(X %*% coef(m))); \
           dH <- vapply(tt, function(t) sum(dc$time == t & dc$died == 1) / sum(r[dc$time >= t]), 0); \
           vapply(at, function(a) sum(dH[tt <= a]), 0) * exp(sum(coef(m) * x)) }; \
         efron_h0 <- function(m) { r <- as.vector(exp(X %*% coef(m))); \
           vapply(tt, function(t) { D <- which(dc$time == t & dc$died == 1); k <- length(D); \
             den <- sum(r[dc$time >= t]) - (seq_len(k) - 1) / k * sum(r[D]); \
             sum(1 - (1 - r[D] / den)^(1 / r[D])) }, 0) }; \
         kp_contrib <- function(m, x) { s <- vapply(tt, function(t) { f <- survival::survfit(m, \
             newdata = data.frame(arm = c('a', 'b')[x[1] + 1], age = x[2]), stype = 1); \
             f$surv[f$time == t] }, 0); 1 - s / c(1, head(s, -1)) }; \
         q <- stats::quantile(tt, c(0.25, 0.75), type = 2, names = FALSE); \
         w0 <- 0.9 * min(sd(tt), (q[2] - q[1]) / 1.349) * length(tt)^(-1 / 5); \
         ep <- function(u) ifelse(abs(u) < sqrt(5), 3 / 4 * (1 - u^2 / 5) / sqrt(5), 0); \
         smooth_at <- function(at, cc, w = w0) vapply(at, function(v) sum(ep((v - tt) / w) * cc) / w, 0); \
         grid <- function(w = w0) { g <- seq(8, 54, length.out = 101); g[g >= 8 + w & g <= 54 - w] }",
    );
    holds(
        &mut e,
        &[
            same(
                "as.vector(summary(h1, times = c(8, 11, 29, 40, 54))$cumhaz)",
                "breslow_H(cb, c(1, 50), c(8, 11, 29, 40, 54))",
            ),
            // Stata 19: stcox armb age, breslow (exactp, efron); predict H,
            // basechazard; H * exp(xb) at armb = 1, age = 50 (stcurve, cumhaz)
            same(
                "as.vector(summary(h1, times = c(8, 11, 29, 40, 54))$cumhaz)",
                "c(.0121029717911526, .0511229713756994, .402375613647139, .910127525267337, 2.61355922135538)",
            ),
            same(
                "as.vector(summary(h2, times = c(8, 11, 29, 40, 54))$cumhaz)",
                "c(.0117794060444646, .0497791823885296, .392756138054102, .89598194446619, 2.60258248718795)",
            ),
            same(
                "as.vector(summary(h3, times = c(8, 11, 29, 40, 54))$cumhaz)",
                "c(.0119757514275733, .0510548808627151, .411394933592066, .918147303522454, 2.62589697545722)",
            ),
            // Efron's h0 is Stata's predict, basehc after stcox, efron (at 8, 11,
            // 12, 27 and 29, ties at all but the first)
            same(
                "efron_h0(ce)[match(c(8, 11, 12, 27, 29), tt)]",
                "c(.0873898217239446, .191889210774882, .204657402123903, .392957520933578, .703736628784472)",
            ),
            // the dialog's hazard is the smoother on the grid, by hand
            same("attr(z1, 'width')", "w0"),
            same("z1$time", "grid()"),
            same("z1$hazard", "smooth_at(grid(), kp_contrib(cb, c(1, 50)))"),
            same("z2$hazard", "smooth_at(grid(), kp_contrib(cx, c(1, 50)))"),
            same(
                "z3$hazard",
                "smooth_at(grid(), 1 - (1 - efron_h0(ce))^exp(sum(colMeans(X) * coef(ce))))",
            ),
            same(
                "z4$hazard2",
                "smooth_at(grid(), 1 - (1 - efron_h0(ce))^exp(sum(c(1, 60) * coef(ce))))",
            ),
            same("z5$time", "grid(5)"),
            same("z5$hazard", "smooth_at(grid(5), kp_contrib(cb, c(1, 50)), 5)"),
            // Stata 19: stcurve, hazard at(armb=1 age=50) outfile(): 73 points from
            // 14.44 to 47.56; Stata keeps the curve in single precision, so 2e-7
            "nrow(z1) == 73 && nrow(z3) == 73".to_string(),
            within(
                "z1$hazard[c(1, 2, 20, 73)]",
                "c(.0115549890324473, .0119493333622813, .0246726609766483, .0911883413791656)",
                "2e-7",
            ),
            // exactp ties
            within(
                "z2$hazard[1:2]",
                "c(.0112710688263178, .0116566903889179)",
                "2e-7",
            ),
            // efron at the means, and at (armb 0, age 45) and (armb 1, age 60)
            within(
                "z3$hazard[c(1, 2, 73)]",
                "c(.0210135839879513, .0217470433562994, .142648905515671)",
                "2e-7",
            ),
            within(
                "c(z4$hazard1[1:2], z4$hazard2[1:2])",
                "c(.0447285659611225, .0462591499090195, .0102442298084497, .0106050195172429)",
                "2e-7",
            ),
            // width(5): from 13.06
            within(
                "z5$hazard[1:2]",
                "c(.0104881804436445, .0108795175328851)",
                "2e-7",
            ),
            same("z5$time[1]", "13.06"),
        ],
    );
    // the hazard refuses a stratified model, as Stata's stcurve does
    let (outcome, text, _) = dialog(
        &mut e,
        &COX,
        &[
            ("time", "time"),
            ("event", "died"),
            ("strata", "centre"),
            ("save", "cs"),
        ],
        &[("x", &["arm", "age"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let (outcome, text, _) = dialog(
        &mut e,
        &COX_CURVES,
        &[("model", "cs"), ("kind", "hazard"), ("save", "z6")],
        &[],
        &[],
    );
    assert_ne!(outcome, Outcome::Ok, "{text}");
    assert!(text.contains("without strata"), "{text}");
}
#[test]
fn time_varying_covariates_match_stsplit() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    versions(&mut e);
    let fit = |e: &mut Engine, extra: &[(&str, &str)], lists: &[(&str, &[&str])]| {
        let mut set = vec![("time", "time"), ("event", "died"), ("ties", "breslow")];
        set.extend_from_slice(extra);
        let (outcome, text, _) = dialog(e, &COX, &set, lists, &[]);
        assert_eq!(outcome, Outcome::Ok, "{text}");
    };
    fit(&mut e, &[("save", "cb")], &[("x", &["arm", "age"])]);
    fit(
        &mut e,
        &[("split", "10, 20"), ("save", "csp")],
        &[("x", &["arm", "age"])],
    );
    fit(
        &mut e,
        &[("split", "10, 20"), ("save", "cv")],
        &[("x", &["arm", "age"]), ("varying", &["age"])],
    );
    let both = |m: &str| format!("c(coef({m}), sqrt(diag(vcov({m}))))");
    holds(
        &mut e,
        &[
            // 105 rows once cut at 10 and 20 (Stata's stsplit, at(10 20)), with
            // the interval as a factor of 1, 2 and 3
            "nrow(split_data) == 105".to_string(),
            "identical(levels(split_data$interval), c('1', '2', '3'))".to_string(),
            // cutting changes nothing for a model with no interval terms
            same(&both("csp"), &both("cb")),
            // Stata 19: stset time, failure(died) id(rowid); stcox armb age,
            // breslow, before and after stsplit period, at(10 20)
            same(
                &both("csp"),
                "c(-1.28409462670782, -.0136226053651508, .430014463307847, .0209238027223065)",
            ),
            // Stata 19: stcox armb c.age#i.period, breslow after the split
            within(
                &both("cv"),
                "c(-1.29993547069813, .00118221607062002, -.0583060611772343, \
                   .0013359962378508, .429569033962738, .0820610186282193, \
                   .0436045091332839, .0253027541269418)",
                "1e-8",
            ),
        ],
    );
    // the start, stop input: the cut rows fitted as they are, through the
    // start time box, are the same model
    let columns = [
        ("tstart", VarKind::Number),
        ("time", VarKind::Number),
        ("died", VarKind::Number),
        ("arm", VarKind::Factor),
        ("age", VarKind::Number),
    ];
    let (outcome, text) = dialog_on(
        &mut e,
        "split_data",
        &columns,
        &COX,
        &[
            ("time", "time"),
            ("event", "died"),
            ("entry", "tstart"),
            ("ties", "breslow"),
            ("save", "css"),
        ],
        &[("x", &["arm", "age"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    holds(
        &mut e,
        &[
            "identical(attr(css$y, 'type'), 'counting')".to_string(),
            same(&both("css"), &both("csp")),
        ],
    );
}

/// Survival curves from a Cox model, asked of a name that is no model in the
/// workspace (or not a Cox model), say what the dialog needs instead of R's
/// "object 'cox' not found" (SW18), for the curves and the smoothed hazard.
#[test]
fn cox_curves_without_a_cox_model_say_what_they_need() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    for kind in ["survival", "cumhaz", "hazard"] {
        for model in ["no_such_cox_model", "time"] {
            let (outcome, text, _) = dialog(
                &mut e,
                &COX_CURVES,
                &[("model", model), ("kind", kind), ("save", "c_none")],
                &[],
                &[],
            );
            assert_eq!(outcome, Outcome::Error, "{kind} {model}: {text}");
            assert!(
                text.contains("needs a Cox model fitted with coxph"),
                "{kind} {model}: {text}"
            );
            assert!(!text.contains("not found"), "{kind} {model}: {text}");
        }
    }
}

#[test]
fn survival_models_have_the_model_menu() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    versions(&mut e);
    for class in ["coxph", "survreg"] {
        assert!(postest::is_model_class(class), "{class}");
        let entries: Vec<&str> = postest::model_specs(class)
            .iter()
            .map(|s| s.entry)
            .collect();
        assert_eq!(entries, ["Model fit", "Compare models"], "{class}");
    }
    let fit = |e: &mut Engine, save: &str, x: &[&str]| {
        let (outcome, text, _) = dialog(
            e,
            &COX,
            &[
                ("time", "time"),
                ("event", "died"),
                ("ties", "efron"),
                ("save", save),
            ],
            &[("x", x)],
            &[],
        );
        assert_eq!(outcome, Outcome::Ok, "{text}");
    };
    fit(&mut e, "cb", &["arm", "age"]);
    fit(&mut e, "cage", &["age"]);
    let (outcome, text, _) = dialog(
        &mut e,
        &PARAMETRIC,
        &[("time", "time"), ("event", "died"), ("save", "aft")],
        &[("x", &["arm", "age"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    // Model fit on a Cox model: the rows used and the events, and the
    // log-likelihood, which is Stata's e(ll) (stcox armb age, efron)
    let (outcome, text) = dialog_on(&mut e, "cb", &[], &MODEL_FIT, &[], &[], &[]);
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(
        text.contains("Observations: 38") && text.contains("Events: 33") && text.contains("logLik"),
        "{text}"
    );
    // Model fit on a Weibull model: survreg's log-likelihood is on the log-time
    // scale, Stata streg's adds the log of each event time
    let (outcome, text) = dialog_on(&mut e, "aft", &[], &MODEL_FIT, &[], &[], &[]);
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(
        text.contains("Observations: 38")
            && text.contains("Log-likelihood on the time scale (as Stata reports it)"),
        "{text}"
    );
    // Compare models: the Cox model against its sub-model, with the
    // likelihood-ratio test
    let (outcome, text) = dialog_on(
        &mut e,
        "cb",
        &[],
        &COMPARE_MODELS,
        &[("others", "cage")],
        &[],
        &["nested"],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(text.contains("AIC") && text.contains("Chisq"), "{text}");
    holds(
        &mut e,
        &[
            // Stata 19: stcox armb age, efron: e(ll); stcox age, efron: e(ll)
            same("as.numeric(logLik(cb))", "-85.8986670222571"),
            same("as.numeric(logLik(cage))", "-90.8194063452229"),
            same(
                "anova(cb, cage)$Chisq[2]",
                "2 * (-85.8986670222571 - -90.8194063452229)",
            ),
            // Stata 19: streg armb age, d(weibull) time: e(ll)
            same(
                "as.numeric(logLik(aft)) + sum(log(aft$y[aft$y[, 2] == 1, 1]))",
                "-23.9688007745029",
            ),
        ],
    );
}
