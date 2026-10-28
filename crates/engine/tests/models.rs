//! Statistics menu linear models and ANOVA (P4.3) end to end against a real R
//! (skipped with a note when none is found). Each dialog's code is built from
//! its spec and run; the model it saves must equal the model R fits for the
//! call written out here by hand, and the summary it prints must equal R's.

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
    Engine::start(SessionConfig::new(install)).ok()
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
