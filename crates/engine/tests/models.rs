//! Statistics menu linear models and ANOVA (P4.3) end to end against a real R
//! (skipped with a note when none is found). Each dialog's code is built from
//! its spec and run; the model it saves must equal the model R fits for the
//! call written out here by hand, and the summary it prints must equal R's.

mod pinned;

use std::time::{Duration, Instant};

use rcraft_catalog::models::{FACTORIAL, LINEAR, ONE_WAY};
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
     y = c(10.1, 12.3, 11.8, 14.2, 15.1, 13.9, 17.2, 18.1, 16.5, 19.0, 20.2, 18.8), \
     x = 1:12, \
     z = c(2.1, 1.9, 3.2, 2.8, 3.9, 3.1, 4.5, 4.0, 5.2, 4.8, 6.1, 5.5), \
     a = factor(rep(c('lo', 'mid', 'hi'), 4)), \
     b = factor(rep(c('u', 'v'), each = 6)), \
     w = rep(c(1, 2), 6))";

fn context() -> Context {
    Context {
        dataset: "d".into(),
        frames: Vec::new(),
        columns: [
            ("y", VarKind::Number),
            ("x", VarKind::Number),
            ("z", VarKind::Number),
            ("a", VarKind::Factor),
            ("b", VarKind::Factor),
            ("w", VarKind::Number),
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
fn dialog(
    e: &mut Engine,
    spec: &Spec,
    set: &[(&str, &str)],
    lists: &[(&str, &[&str])],
    flags: &[&str],
) -> (Outcome, String) {
    let (made, _) = run(e, Source::Typed, MAKE);
    assert_eq!(made, Outcome::Ok);
    let ctx = context();
    let mut values = spec.defaults(&ctx);
    for (id, text) in set {
        values.set_text(id, *text);
    }
    for (id, names) in lists {
        values.set_list(id, names.iter().map(|n| n.to_string()).collect());
    }
    // exactly the named boxes are ticked, whatever the dialog starts with
    for w in spec.widgets {
        if let Widget::Check { id, .. } = w {
            values.set_flag(id, flags.contains(id));
        }
    }
    let code = spec.generate(&ctx, &values, Style::Base);
    assert!(code.is_ok(), "{code:?}");
    let code = code.unwrap_or_default();
    let (outcome, text) = run(e, Source::Menu(spec.menu.into()), &code);
    assert_eq!(outcome, Outcome::Ok, "{code}\n{text}");
    (outcome, text)
}

/// A report with spaces tidied, blank lines and the lines that echo the call
/// (which name the model) left out.
fn report(text: &str) -> Vec<String> {
    text.lines()
        .filter(|l| {
            let l = l.trim_start();
            !(l.starts_with("Call:")
                || l.starts_with("stats::")
                || l.starts_with("lm(")
                || l.starts_with("aov(")
                || l.starts_with("Fit:")
                || l.starts_with("warning:"))
        })
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|l| !l.is_empty())
        .collect()
}

/// The part of `text` before `marker`.
fn before<'a>(text: &'a str, marker: &str) -> &'a str {
    text.split(marker).next().unwrap_or(text)
}

/// Runs R that stops unless every one of `exprs` holds.
fn holds(e: &mut Engine, exprs: &str) {
    let (outcome, text) = run(e, Source::Typed, &format!("stopifnot({exprs})"));
    assert_eq!(outcome, Outcome::Ok, "{exprs}\n{text}");
}

#[test]
fn linear_model_is_the_model_r_fits() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);

    let (_, text) = dialog(&mut e, &LINEAR, &[("y", "y")], &[("x", &["x", "z"])], &[]);
    let (_, expected) = run(
        &mut e,
        Source::Typed,
        "print(summary(stats::lm(y ~ x + z, data = d)))",
    );
    let summary = report(before(&text, "Confidence intervals"));
    assert!(summary.len() > 8, "{text}");
    assert_eq!(summary, report(&expected));
    assert!(text.contains("2.5 %") && text.contains("97.5 %"), "{text}");
    holds(
        &mut e,
        "inherits(model, 'lm'), isTRUE(all.equal(coef(model), coef(lm(y ~ x + z, data = d))))",
    );

    // a categorical predictor, interactions, weights, a level, another name
    dialog(
        &mut e,
        &LINEAR,
        &[
            ("y", "y"),
            ("interactions", "two"),
            ("weights", "w"),
            ("conf", "0.9"),
            ("save", "fit2"),
        ],
        &[("x", &["x", "a"])],
        &[],
    );
    holds(
        &mut e,
        "isTRUE(all.equal(coef(fit2), coef(lm(y ~ (x + a)^2, data = d, weights = w)))), \
         'x:amid' %in% names(coef(fit2))",
    );
    let (_, text) = dialog(
        &mut e,
        &LINEAR,
        &[("y", "y"), ("conf", "0.9")],
        &[("x", &["x"])],
        &[],
    );
    assert!(text.contains("5 %") && text.contains("95 %"), "{text}");
}

#[test]
fn anova_dialogs_give_r_s_tables() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);

    let (_, text) = dialog(&mut e, &ONE_WAY, &[("y", "y"), ("g", "a")], &[], &["tukey"]);
    let (_, expected) = run(
        &mut e,
        Source::Typed,
        "m <- stats::aov(y ~ a, data = d); print(summary(m)); \
         cat('\\nGroup means\\n'); print(tapply(d$y, d$a, mean, na.rm = TRUE)); \
         print(stats::TukeyHSD(m))",
    );
    assert_eq!(report(&text), report(&expected), "{text}");
    assert!(text.contains("Tukey multiple comparisons"), "{text}");
    holds(&mut e, "inherits(anova, 'aov')");
    let (_, plain) = dialog(&mut e, &ONE_WAY, &[("y", "y"), ("g", "a")], &[], &[]);
    assert!(!plain.contains("Tukey"), "{plain}");

    let (_, text) = dialog(
        &mut e,
        &FACTORIAL,
        &[("y", "y")],
        &[("g", &["a", "b"])],
        &["tukey"],
    );
    let (_, expected) = run(
        &mut e,
        Source::Typed,
        "m <- stats::aov(y ~ a * b, data = d); print(summary(m)); \
         print(stats::TukeyHSD(m, which = c('a', 'b')))",
    );
    assert_eq!(report(&text), report(&expected), "{text}");
    holds(&mut e, "'a:b' %in% attr(terms(anova), 'term.labels')");
    dialog(
        &mut e,
        &FACTORIAL,
        &[("y", "y"), ("interactions", "none"), ("save", "main_only")],
        &[("g", &["a", "b"])],
        &[],
    );
    holds(
        &mut e,
        "identical(attr(terms(main_only), 'term.labels'), c('a', 'b'))",
    );
}

/// Only base R's stats is used.
const PACKAGES: &[&str] = &[];

/// The numbers each dialog's model gives, against the textbook formulas
/// written out in plain R (no lm or aov): least squares from the normal
/// equations, with and without weights; one-way ANOVA's sums of squares, F and
/// Tukey's honest differences from the studentized range; and a balanced
/// two-way table's sums of squares from the cell, row and column means.
#[test]
fn the_models_numbers_are_the_textbook_formulas() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);

    dialog(&mut e, &LINEAR, &[("y", "y")], &[("x", &["x", "z"])], &[]);
    let check = pinned::all_of(
        "X <- cbind(1, d$x, d$z); y <- d$y; n <- nrow(X); p <- ncol(X); \
         XtX <- crossprod(X); b <- as.vector(solve(XtX, crossprod(X, y))); \
         r <- y - X %*% b; s2 <- sum(r^2) / (n - p); se <- sqrt(diag(s2 * solve(XtX))); \
         tss <- sum((y - mean(y))^2); r2 <- 1 - sum(r^2) / tss; \
         f <- ((tss - sum(r^2)) / (p - 1)) / s2; sm <- summary(model); \
         ci <- cbind(b - qt(0.975, n - p) * se, b + qt(0.975, n - p) * se)",
        &[
            ("coef", "all.equal(unname(coef(model)), b, tolerance = 1e-12)"),
            ("se", "all.equal(unname(sm$coefficients[, 2]), se, tolerance = 1e-12)"),
            (
                "p",
                "all.equal(unname(sm$coefficients[, 4]), 2 * pt(-abs(b / se), n - p), tolerance = 1e-10)",
            ),
            ("r2", "all.equal(sm$r.squared, r2, tolerance = 1e-12)"),
            ("f", "all.equal(unname(sm$fstatistic[1]), f, tolerance = 1e-12)"),
            ("ci", "all.equal(unname(confint(model)), unname(ci), tolerance = 1e-12)"),
        ],
    );
    holds(&mut e, &check);

    // weighted least squares
    dialog(
        &mut e,
        &LINEAR,
        &[("y", "y"), ("weights", "w"), ("save", "wfit")],
        &[("x", &["x"])],
        &[],
    );
    let check = pinned::all_of(
        "X <- cbind(1, d$x); W <- diag(d$w); XWX <- t(X) %*% W %*% X; \
         b <- as.vector(solve(XWX, t(X) %*% W %*% d$y)); r <- d$y - X %*% b; \
         s2 <- sum(d$w * r^2) / (nrow(X) - 2); se <- sqrt(diag(s2 * solve(XWX)))",
        &[
            (
                "coef",
                "all.equal(unname(coef(wfit)), b, tolerance = 1e-12)",
            ),
            (
                "se",
                "all.equal(unname(sqrt(diag(vcov(wfit)))), se, tolerance = 1e-12)",
            ),
        ],
    );
    holds(&mut e, &check);

    // one-way ANOVA and Tukey's honest differences
    dialog(&mut e, &ONE_WAY, &[("y", "y"), ("g", "a")], &[], &["tukey"]);
    let check = pinned::all_of(
        "y <- d$y; g <- d$a; m <- tapply(y, g, mean); n <- tapply(y, g, length); \
         k <- length(m); N <- length(y); ssb <- sum(n * (m - mean(y))^2); \
         ssw <- sum((y - m[as.character(g)])^2); msw <- ssw / (N - k); \
         f <- (ssb / (k - 1)) / msw; tab <- summary(anova)[[1]]; \
         hsd <- stats::TukeyHSD(anova)$a; pairs <- utils::combn(levels(g), 2); \
         diffs <- as.vector(m[pairs[2, ]] - m[pairs[1, ]]); \
         q <- abs(diffs) / sqrt(msw / 2 * (1 / n[pairs[1, ]] + 1 / n[pairs[2, ]])); \
         p <- as.vector(ptukey(q, k, N - k, lower.tail = FALSE)); \
         rows <- paste(pairs[2, ], pairs[1, ], sep = '-')",
        &[
            ("ss", "all.equal(tab[['Sum Sq']], c(ssb, ssw), tolerance = 1e-12)"),
            ("f", "all.equal(tab[['F value']][1], f, tolerance = 1e-12)"),
            (
                "p",
                "all.equal(tab[['Pr(>F)']][1], pf(f, k - 1, N - k, lower.tail = FALSE), tolerance = 1e-10)",
            ),
            ("diff", "all.equal(unname(hsd[rows, 'diff']), diffs, tolerance = 1e-12)"),
            ("padj", "all.equal(unname(hsd[rows, 'p adj']), p, tolerance = 1e-6)"),
        ],
    );
    holds(&mut e, &check);

    // two-way ANOVA, balanced (two rows a cell)
    dialog(
        &mut e,
        &FACTORIAL,
        &[("y", "y")],
        &[("g", &["a", "b"])],
        &[],
    );
    let check = pinned::all_of(
        "y <- d$y; gm <- mean(y); ma <- ave(y, d$a); mb <- ave(y, d$b); \
         mc <- ave(y, d$a, d$b); ssa <- sum((ma - gm)^2); ssb <- sum((mb - gm)^2); \
         ssab <- sum((mc - ma - mb + gm)^2); sse <- sum((y - mc)^2); \
         tab <- summary(anova)[[1]]",
        &[
            (
                "ss",
                "all.equal(tab[['Sum Sq']], c(ssa, ssb, ssab, sse), tolerance = 1e-12)",
            ),
            ("df", "all.equal(tab[['Df']], c(2, 1, 2, 6))"),
        ],
    );
    holds(&mut e, &check);
}
