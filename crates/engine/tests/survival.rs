//! Statistics menu survival analysis (P6.3) end to end against a real R
//! (skipped with a note when none is found). Each dialog's code is built from
//! its spec and run on made-up data; what it prints must equal survival's own
//! report for the call written out by hand, and its numbers must equal ones
//! worked out another way: the product-limit estimate and the log-rank
//! statistic by hand, the Cox coefficient by maximising Breslow's partial
//! likelihood directly, and the exponential model as its Poisson GLM.

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

/// Forty rows in two arms, some censored, with ties, an entry time before
/// each exit, and two centres.
const MAKE: &str = "d <- local({ \
     i <- 1:40; \
     arm <- factor(rep(c('a', 'b'), each = 20)); \
     age <- 40 + (i * 7) %% 30; \
     time <- round(5 + 30 * ((i * 37) %% 41) / 41 * ifelse(arm == 'b', 1.6, 1) + age / 20, 0); \
     died <- as.numeric((i * 11) %% 7 != 0); \
     start <- round(time * (i %% 3) / 10, 1); \
     centre <- c('x', 'y')[1 + i %% 2]; \
     data.frame(time, died, arm, age, start, centre) })";

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

fn holds(e: &mut Engine, exprs: &str) {
    let (outcome, text, _) = run(e, Source::Typed, &format!("stopifnot({exprs})"));
    assert_eq!(outcome, Outcome::Ok, "{exprs}\n{text}");
}

#[test]
fn kaplan_meier_is_the_product_limit_estimate() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
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
        "k <- survival::survfit(survival::Surv(time, died) ~ 1, data = d, conf.int = 0.95); print(k)",
    );
    assert!(report(&expected).len() >= 2, "{expected}");
    assert_eq!(report(&text), report(&expected), "{text}");
    // at each event time, the product of (1 - deaths / at risk) so far
    holds(
        &mut e,
        "local({ tt <- sort(unique(d$time[d$died == 1])); \
           s <- cumprod(vapply(tt, function(t) 1 - sum(d$time == t & d$died == 1) / sum(d$time >= t), 0)); \
           isTRUE(all.equal(km$surv[match(tt, km$time)], s, tolerance = 1e-12)) })",
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
    holds(&mut e, "identical(names(km2$strata), c('arm=a', 'arm=b'))");
}

#[test]
fn log_rank_is_the_statistic_worked_out_by_hand() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
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
    // (O - E)^2 / V for arm a, over the distinct event times
    holds(
        &mut e,
        "local({ tt <- sort(unique(d$time[d$died == 1])); o <- 0; ex <- 0; v <- 0; \
           for (t in tt) { r <- d$time >= t; n <- sum(r); n1 <- sum(r & d$arm == 'a'); \
             k <- sum(d$time == t & d$died == 1); \
             o <- o + sum(d$time == t & d$died == 1 & d$arm == 'a'); ex <- ex + k * n1 / n; \
             if (n > 1) v <- v + k * (n1 / n) * (1 - n1 / n) * (n - k) / (n - 1) }; \
           s <- survival::survdiff(survival::Surv(time, died) ~ arm, data = d); \
           isTRUE(all.equal(s$chisq, (o - ex)^2 / v, tolerance = 1e-10)) })",
    );
}

#[test]
fn cox_is_the_partial_likelihood_maximum() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
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

    // Breslow's partial likelihood for age, maximised directly
    let (outcome, text, _) = dialog(
        &mut e,
        &COX,
        &[
            ("time", "time"),
            ("event", "died"),
            ("ties", "breslow"),
            ("save", "cb"),
        ],
        &[("x", &["age"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    holds(
        &mut e,
        "local({ pl <- function(b) sum(vapply(which(d$died == 1), function(i) \
             b * d$age[i] - log(sum(exp(b * d$age[d$time >= d$time[i]]))), 0)); \
           b <- optimize(function(b) -pl(b), c(-1, 1), tol = 1e-12)$minimum; \
           isTRUE(all.equal(unname(coef(cb)), b, tolerance = 1e-6)) })",
    );

    // entry times, strata and a cluster: robust errors beside the model's
    let (outcome, text, _) = dialog(
        &mut e,
        &COX,
        &[
            ("time", "time"),
            ("event", "died"),
            ("entry", "start"),
            ("strata", "centre"),
            ("cluster", "arm"),
            ("save", "cs"),
        ],
        &[("x", &["age"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    holds(
        &mut e,
        "!is.null(cs$naive.var), identical(attr(cs$y, 'type'), 'counting'), \
         'robust se' %in% colnames(summary(cs)$coefficients), \
         identical(names(cs$coefficients), 'age')",
    );
}

#[test]
fn exponential_model_is_its_poisson_glm() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
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
    // the exponential AFT model is a Poisson GLM of the events with log time
    // as offset, its coefficients' signs turned
    holds(
        &mut e,
        "isTRUE(all.equal(unname(coef(aft)), \
           -unname(coef(glm(died ~ arm + age + offset(log(time)), family = poisson, data = d))), \
           tolerance = 1e-6))",
    );

    // Weibull: its log-likelihood is the one written out and maximised
    let (outcome, text, _) = dialog(
        &mut e,
        &PARAMETRIC,
        &[("time", "time"), ("event", "died"), ("save", "wb")],
        &[("x", &["age"])],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    holds(
        &mut e,
        "local({ ll <- function(p) { mu <- p[1] + p[2] * d$age; s <- exp(p[3]); \
             z <- (log(d$time) - mu) / s; \
             sum(d$died * (z - exp(z) - log(s) - log(d$time)) + (1 - d$died) * (-exp(z))) }; \
           o <- optim(c(coef(wb), log(wb$scale)), function(p) -ll(p), method = 'BFGS', \
             control = list(reltol = 1e-14)); \
           isTRUE(all.equal(-o$value, wb$loglik[2], tolerance = 1e-8)) && \
             isTRUE(all.equal(unname(o$par[1:2]), unname(coef(wb)), tolerance = 1e-4)) })",
    );
}
