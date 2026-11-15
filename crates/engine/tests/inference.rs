//! Statistics menu tests and intervals (P4.2) end to end against a real R
//! (skipped with a note when none is found). Each dialog's code is built from
//! its spec and run; its printed report must be the report R gives for the call
//! written out here by hand (the `data:` line aside, which names the call), so
//! the options chosen in the dialog really reach R.

mod pinned;

use std::time::{Duration, Instant};

use rcraft_catalog::inference::{
    BINOMIAL, FISHER, KRUSKAL, MANN_WHITNEY, ONE_T, PAIRED_T, PROPORTIONS, SIGNED_RANK, TWO_T,
    VARIANCE,
};
use rcraft_engine::{Engine, Item, Outcome, Source};
use rcraft_rcodegen::dialog::{ColumnInfo, Context, Spec, Style, VarKind};
use rcraft_rhost::{find_installs, Env, SessionConfig};

fn engine() -> Option<Engine> {
    let install = find_installs(&Env::from_process())
        .into_iter()
        .find(|i| i.is_supported());
    let Some(install) = install else {
        eprintln!("skipped: no supported R on this machine");
        return None;
    };
    let mut e = Engine::start(SessionConfig::new(install)).ok()?;
    pinned::record(&mut e, PACKAGES);
    Some(e)
}

fn settle(e: &mut Engine) {
    let limit = Instant::now() + Duration::from_secs(60);
    loop {
        e.poll(Duration::from_millis(50));
        if !e.is_working() {
            break;
        }
        assert!(Instant::now() < limit, "the engine did not settle");
    }
}

fn run(e: &mut Engine, source: Source, code: &str) -> (Outcome, String) {
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
                    Item::Plot { .. } => String::new(),
                })
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default();
    (outcome, text)
}

const MAKE: &str = "d <- data.frame(\
     x = c(5.1, 4.9, 6.2, 5.8, 6.0, 5.5, 6.1, 5.3, 5.7, 6.4), \
     y = c(5.5, 5.0, 6.8, 6.1, 6.3, 5.4, 6.6, 5.9, 6.0, 6.5), \
     g = factor(rep(c('a', 'b'), each = 5)), \
     three = factor(rep(c('p', 'q', 'r'), length.out = 10)), \
     s = c('yes', 'no', 'yes', 'yes', 'no', 'yes', 'yes', 'yes', 'no', 'yes'), \
     w = c('u', 'u', 'v', 'v', 'u', 'v', 'u', 'v', 'u', 'v'))";

fn context() -> Context {
    Context {
        dataset: "d".into(),
        frames: Vec::new(),
        columns: [
            ("x", VarKind::Number),
            ("y", VarKind::Number),
            ("g", VarKind::Factor),
            ("three", VarKind::Factor),
            ("s", VarKind::Text),
            ("w", VarKind::Text),
        ]
        .into_iter()
        .map(|(name, kind)| ColumnInfo {
            name: name.into(),
            kind,
        })
        .collect(),
    }
}

/// What the dialog's code prints, with the made-up data in R.
fn dialog(e: &mut Engine, spec: &Spec, set: &[(&str, &str)]) -> (Outcome, String) {
    let (made, _) = run(e, Source::Typed, MAKE);
    assert_eq!(made, Outcome::Ok);
    let ctx = context();
    let mut values = spec.defaults(&ctx);
    for (id, text) in set {
        values.set_text(id, *text);
    }
    let code = spec.generate(&ctx, &values, Style::Base);
    assert!(code.is_ok(), "{code:?}");
    let code = code.unwrap_or_default();
    let (outcome, text) = run(e, Source::Menu(spec.menu.into()), &code);
    if outcome == Outcome::Ok {
        assert!(!text.contains("error:"), "{code}\n{text}");
    }
    (outcome, text)
}

/// The report without the line naming the call, spaces and blank lines tidied.
fn report(text: &str) -> Vec<String> {
    text.lines()
        .filter(|l| !l.starts_with("data:") && !l.starts_with("warning:"))
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|l| !l.is_empty())
        .collect()
}

/// The dialog must print what R prints for `reference`.
fn matches_r(e: &mut Engine, spec: &Spec, set: &[(&str, &str)], reference: &str) {
    let (outcome, text) = dialog(e, spec, set);
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let (ref_outcome, expected) = run(e, Source::Typed, &format!("print({reference})"));
    assert_eq!(ref_outcome, Outcome::Ok, "{reference}\n{expected}");
    assert!(
        report(&expected).len() >= 2,
        "the reference printed nothing useful: {expected}"
    );
    assert_eq!(report(&text), report(&expected), "{}", spec.entry);
}

#[test]
fn t_tests_pass_their_options_to_r() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);

    matches_r(
        &mut e,
        &ONE_T,
        &[
            ("y", "x"),
            ("mu", "5.5"),
            ("alternative", "greater"),
            ("conf", "0.99"),
        ],
        "stats::t.test(d$x, mu = 5.5, alternative = 'greater', conf.level = 0.99)",
    );
    // the options make a difference: the defaults give another report
    let (_, default) = dialog(&mut e, &ONE_T, &[("y", "x"), ("mu", "5.5")]);
    let (_, one_sided) = dialog(
        &mut e,
        &ONE_T,
        &[("y", "x"), ("mu", "5.5"), ("alternative", "greater")],
    );
    assert_ne!(report(&default), report(&one_sided));

    matches_r(
        &mut e,
        &TWO_T,
        &[("y", "x"), ("g", "g")],
        "stats::t.test(x ~ g, data = d)",
    );
    matches_r(
        &mut e,
        &TWO_T,
        &[
            ("y", "x"),
            ("g", "g"),
            ("variance", "pooled"),
            ("alternative", "less"),
        ],
        "stats::t.test(x ~ g, data = d, var.equal = TRUE, alternative = 'less')",
    );
    matches_r(
        &mut e,
        &PAIRED_T,
        &[("x", "x"), ("y", "y"), ("conf", "0.9")],
        "stats::t.test(d$x, d$y, paired = TRUE, conf.level = 0.9)",
    );

    // three groups: R refuses and says why
    let (outcome, text) = dialog(&mut e, &TWO_T, &[("y", "x"), ("g", "three")]);
    assert_eq!(outcome, Outcome::Error, "{text}");
    assert!(text.contains("exactly 2 levels"), "{text}");
}

#[test]
fn proportion_and_exact_tests_count_the_successes() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);

    // 7 of 10 are "yes"
    matches_r(
        &mut e,
        &PROPORTIONS,
        &[("y", "s"), ("success", "yes"), ("p0", "0.4")],
        "stats::prop.test(7, 10, p = 0.4)",
    );
    // compared between groups: yes is 2 of 5 in u and 5 of 5 in v
    matches_r(
        &mut e,
        &PROPORTIONS,
        &[("y", "s"), ("success", "yes"), ("g", "w")],
        "stats::prop.test(c(2, 5), c(5, 5))",
    );
    matches_r(
        &mut e,
        &BINOMIAL,
        &[
            ("y", "s"),
            ("success", "yes"),
            ("p0", "0.5"),
            ("alternative", "greater"),
        ],
        "stats::binom.test(7, 10, p = 0.5, alternative = 'greater')",
    );
    matches_r(
        &mut e,
        &FISHER,
        &[("row", "s"), ("col", "w")],
        "stats::fisher.test(table(d$s, d$w))",
    );
}

#[test]
fn rank_and_variance_tests_match_r() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);

    matches_r(
        &mut e,
        &MANN_WHITNEY,
        &[("y", "x"), ("g", "g")],
        "stats::wilcox.test(x ~ g, data = d)",
    );
    matches_r(
        &mut e,
        &SIGNED_RANK,
        &[("x", "x"), ("y", "y"), ("alternative", "less")],
        "stats::wilcox.test(d$x, d$y, paired = TRUE, alternative = 'less')",
    );
    matches_r(
        &mut e,
        &KRUSKAL,
        &[("y", "x"), ("g", "three")],
        "stats::kruskal.test(x ~ three, data = d)",
    );
    matches_r(
        &mut e,
        &VARIANCE,
        &[("y", "x"), ("g", "three"), ("test", "bartlett")],
        "stats::bartlett.test(x ~ three, data = d)",
    );
    matches_r(
        &mut e,
        &VARIANCE,
        &[("y", "x"), ("g", "g"), ("test", "f")],
        "stats::var.test(x ~ g, data = d)",
    );
    // Brown-Forsythe: the F test of the spread around each group's median
    let (outcome, text) = dialog(&mut e, &VARIANCE, &[("y", "x"), ("g", "three")]);
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(text.contains("Brown-Forsythe"), "{text}");
    let (_, expected) = run(
        &mut e,
        Source::Typed,
        "spread <- abs(d$x - ave(d$x, d$three, FUN = median)); print(anova(lm(spread ~ d$three)))",
    );
    let f_row = |t: &str| {
        t.lines()
            .find(|l| l.trim_start().starts_with("g ") || l.trim_start().starts_with("d$three"))
            .map(|l| l.split_whitespace().skip(1).collect::<Vec<_>>().join(" "))
    };
    assert!(f_row(&expected).is_some(), "{expected}");
    assert_eq!(f_row(&text), f_row(&expected), "{text}\n{expected}");
}

/// Base R's stats only.
const PACKAGES: &[&str] = &[];

/// Runs R that stops unless `exprs` holds.
fn holds(e: &mut Engine, exprs: &str) {
    let (outcome, text) = run(e, Source::Typed, &format!("stopifnot({exprs})"));
    assert_eq!(outcome, Outcome::Ok, "{exprs}\n{text}");
}

/// Runs the dialog's code as the menu does and keeps the test it gives as
/// `res`.
fn kept(e: &mut Engine, spec: &Spec, set: &[(&str, &str)]) {
    let (made, _) = run(e, Source::Typed, MAKE);
    assert_eq!(made, Outcome::Ok);
    let ctx = context();
    let mut values = spec.defaults(&ctx);
    for (id, text) in set {
        values.set_text(id, *text);
    }
    let code = spec
        .generate(&ctx, &values, Style::Base)
        .unwrap_or_default();
    let (outcome, text) = run(
        e,
        Source::Menu(spec.menu.into()),
        &pinned::keep_value(&code, "res"),
    );
    assert_eq!(outcome, Outcome::Ok, "{code}\n{text}");
}

/// `res$statistic` (or another part) is `want` to 1e-12.
fn same(part: &str, want: &str) -> String {
    format!("all.equal(unname({part}), {want}, tolerance = 1e-12)")
}

/// Each test's statistic, p-value and interval against the textbook formula
/// written out in plain R: Student's and Welch's t, the score test of a
/// proportion with Yates' correction, the binomial and hypergeometric sums,
/// the Mann-Whitney W with its exact distribution counted from every split of
/// the ranks, the signed-rank V with the normal approximation for ties,
/// Kruskal-Wallis H with its tie correction, Bartlett's K2 and the F ratio.
#[test]
fn test_statistics_are_the_textbook_formulas() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    let stat = "res$statistic";
    let p = "res$p.value";

    kept(
        &mut e,
        &ONE_T,
        &[
            ("y", "x"),
            ("mu", "5.5"),
            ("alternative", "greater"),
            ("conf", "0.99"),
        ],
    );
    holds(
        &mut e,
        &pinned::all_of(
            "x <- d$x; n <- length(x); se <- sd(x) / sqrt(n); t <- (mean(x) - 5.5) / se",
            &[
                ("t", &same(stat, "t")),
                ("p", &same(p, "pt(t, n - 1, lower.tail = FALSE)")),
                (
                    "ci",
                    &same("res$conf.int[1]", "mean(x) - qt(0.99, n - 1) * se"),
                ),
            ],
        ),
    );

    kept(&mut e, &TWO_T, &[("y", "x"), ("g", "g")]);
    holds(
        &mut e,
        &pinned::all_of(
            "a <- d$x[d$g == 'a']; b <- d$x[d$g == 'b']; va <- var(a) / 5; vb <- var(b) / 5; \
             se <- sqrt(va + vb); t <- (mean(a) - mean(b)) / se; \
             df <- se^4 / (va^2 / 4 + vb^2 / 4)",
            &[
                ("t", &same(stat, "t")),
                ("df", &same("res$parameter", "df")),
                ("p", &same(p, "2 * pt(-abs(t), df)")),
            ],
        ),
    );

    kept(
        &mut e,
        &TWO_T,
        &[
            ("y", "x"),
            ("g", "g"),
            ("variance", "pooled"),
            ("alternative", "less"),
        ],
    );
    holds(
        &mut e,
        &pinned::all_of(
            "a <- d$x[d$g == 'a']; b <- d$x[d$g == 'b']; sp2 <- (4 * var(a) + 4 * var(b)) / 8; \
             t <- (mean(a) - mean(b)) / sqrt(sp2 * (1 / 5 + 1 / 5))",
            &[("t", &same(stat, "t")), ("p", &same(p, "pt(t, 8)"))],
        ),
    );

    kept(
        &mut e,
        &PAIRED_T,
        &[("x", "x"), ("y", "y"), ("conf", "0.9")],
    );
    holds(
        &mut e,
        &pinned::all_of(
            "dd <- d$x - d$y; n <- length(dd); se <- sd(dd) / sqrt(n); t <- mean(dd) / se; \
             ci <- mean(dd) + c(-1, 1) * qt(0.95, n - 1) * se",
            &[
                ("t", &same(stat, "t")),
                ("p", &same(p, "2 * pt(-abs(t), n - 1)")),
                ("ci", &same("as.vector(res$conf.int)", "ci")),
            ],
        ),
    );

    // 7 of 10 are yes; against 0.4, with Yates' correction
    kept(
        &mut e,
        &PROPORTIONS,
        &[("y", "s"), ("success", "yes"), ("p0", "0.4")],
    );
    holds(
        &mut e,
        &pinned::all_of(
            "x <- sum(d$s == 'yes'); n <- nrow(d); dev <- abs(x - n * 0.4); \
             x2 <- (dev - min(0.5, dev))^2 / (n * 0.4 * 0.6)",
            &[
                ("x2", &same(stat, "x2")),
                ("p", &same(p, "pchisq(x2, 1, lower.tail = FALSE)")),
                ("estimate", "unname(res$estimate) == 0.7"),
            ],
        ),
    );

    // between groups: 2 of 5 in u, 5 of 5 in v
    kept(
        &mut e,
        &PROPORTIONS,
        &[("y", "s"), ("success", "yes"), ("g", "w")],
    );
    holds(
        &mut e,
        &pinned::all_of(
            "x <- c(2, 5); n <- c(5, 5); pp <- sum(x) / sum(n); O <- cbind(x, n - x); \
             E <- cbind(n * pp, n * (1 - pp)); yates <- min(0.5, abs(x - n * pp)); \
             x2 <- sum((abs(O - E) - yates)^2 / E)",
            &[
                ("x2", &same(stat, "x2")),
                ("p", &same(p, "pchisq(x2, 1, lower.tail = FALSE)")),
            ],
        ),
    );

    kept(
        &mut e,
        &BINOMIAL,
        &[
            ("y", "s"),
            ("success", "yes"),
            ("p0", "0.5"),
            ("alternative", "greater"),
        ],
    );
    holds(
        &mut e,
        &pinned::all_of("", &[("p", &same(p, "sum(choose(10, 7:10)) / 2^10"))]),
    );

    // s by w: no and u 3, no and v 0, yes and u 2, yes and v 5
    kept(&mut e, &FISHER, &[("row", "s"), ("col", "w")]);
    holds(
        &mut e,
        &pinned::all_of(
            "a <- 0:3; pr <- choose(5, a) * choose(5, 3 - a) / choose(10, 3); \
             want <- sum(pr[pr <= pr[a == 3] * (1 + 1e-7)])",
            &[("p", &same(p, "want"))],
        ),
    );

    kept(&mut e, &MANN_WHITNEY, &[("y", "x"), ("g", "g")]);
    holds(
        &mut e,
        &pinned::all_of(
            "r <- rank(d$x); w <- sum(r[d$g == 'a']) - 15; \
             all_w <- apply(utils::combn(10, 5), 2, function(i) sum(r[i])) - 15; \
             want <- min(1, 2 * min(mean(all_w <= w), mean(all_w >= w)))",
            &[("w", &same(stat, "w")), ("p", &same(p, "want"))],
        ),
    );

    kept(
        &mut e,
        &SIGNED_RANK,
        &[("x", "x"), ("y", "y"), ("alternative", "less")],
    );
    holds(
        &mut e,
        &pinned::all_of(
            "dd <- d$x - d$y; n <- length(dd); r <- rank(abs(dd)); v <- sum(r[dd > 0]); \
             ties <- table(r); sigma <- sqrt(n * (n + 1) * (2 * n + 1) / 24 - \
             sum(ties^3 - ties) / 48); z <- (v - n * (n + 1) / 4 + 0.5) / sigma",
            &[("v", &same(stat, "v")), ("p", &same(p, "pnorm(z)"))],
        ),
    );

    kept(&mut e, &KRUSKAL, &[("y", "x"), ("g", "three")]);
    holds(
        &mut e,
        &pinned::all_of(
            "r <- rank(d$x); N <- length(r); rs <- tapply(r, d$three, sum); \
             ns <- tapply(r, d$three, length); t <- table(r); \
             h <- (12 / (N * (N + 1)) * sum(rs^2 / ns) - 3 * (N + 1)) / \
             (1 - sum(t^3 - t) / (N^3 - N))",
            &[
                ("h", &same(stat, "h")),
                ("p", &same(p, "pchisq(h, 2, lower.tail = FALSE)")),
            ],
        ),
    );

    kept(
        &mut e,
        &VARIANCE,
        &[("y", "x"), ("g", "three"), ("test", "bartlett")],
    );
    holds(
        &mut e,
        &pinned::all_of(
            "v <- tapply(d$x, d$three, var); n <- tapply(d$x, d$three, length); \
             k <- length(v); N <- sum(n); sp2 <- sum((n - 1) * v) / (N - k); \
             k2 <- ((N - k) * log(sp2) - sum((n - 1) * log(v))) / \
             (1 + (sum(1 / (n - 1)) - 1 / (N - k)) / (3 * (k - 1)))",
            &[
                ("k2", &same(stat, "k2")),
                ("p", &same(p, "pchisq(k2, k - 1, lower.tail = FALSE)")),
            ],
        ),
    );

    kept(&mut e, &VARIANCE, &[("y", "x"), ("g", "g"), ("test", "f")]);
    holds(
        &mut e,
        &pinned::all_of(
            "f <- var(d$x[d$g == 'a']) / var(d$x[d$g == 'b'])",
            &[
                ("f", &same(stat, "f")),
                (
                    "p",
                    &same(p, "2 * min(pf(f, 4, 4), pf(f, 4, 4, lower.tail = FALSE))"),
                ),
            ],
        ),
    );
}
