//! Statistics menu tests and intervals (P4.2) end to end against a real R
//! (skipped with a note when none is found). Each dialog's code is built from
//! its spec and run; its printed report must be the report R gives for the call
//! written out here by hand (the `data:` line aside, which names the call), so
//! the options chosen in the dialog really reach R.

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
     x = c(5.1, 4.9, 6.2, 5.8, 6.0, 5.5, 6.1, 5.3, 5.7, 6.4), \
     y = c(5.5, 5.0, 6.8, 6.1, 6.3, 5.4, 6.6, 5.9, 6.0, 6.5), \
     g = factor(rep(c('a', 'b'), each = 5)), \
     three = factor(rep(c('p', 'q', 'r'), length.out = 10)), \
     s = c('yes', 'no', 'yes', 'yes', 'no', 'yes', 'yes', 'yes', 'no', 'yes'), \
     w = c('u', 'u', 'v', 'v', 'u', 'v', 'u', 'v', 'u', 'v'))";

fn context() -> Context {
    Context {
        dataset: "d".into(),
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
