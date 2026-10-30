//! The messages the translated R gives where it does what Stata does, word
//! for word as Stata 19 SE gives them (each was run in Stata in batch mode on
//! 2026-10-30, on the same small data sets as here).
//!
//! These are the errors and notes a do-file meets in Stata itself: a variable
//! already defined, a file that exists, a key that is not unique, a string in
//! `correlate`, too few new variables after `mlogit`. They are not the
//! translator's own "not translated yet" reasons, which have no Stata
//! counterpart.
//!
//! Without a supported R, or without haven, dplyr, tidyr and nnet, the test
//! is skipped with a note.

#![allow(clippy::panic, clippy::excessive_precision)]

use std::time::{Duration, Instant};

use rcraft_engine::{Engine, Item, Outcome, Source};
use rcraft_rhost::{find_installs, Env, SessionConfig};
use rcraft_statatr::data_r::translate;
use rcraft_statatr::reader::read;

fn engine() -> Option<Engine> {
    let install = find_installs(&Env::from_process())
        .into_iter()
        .find(|i| i.is_supported());
    let Some(install) = install else {
        eprintln!("skipped: no supported R on this machine");
        return None;
    };
    let mut e = Engine::start(SessionConfig::new(install)).ok()?;
    let (have, _) = run(
        &mut e,
        "stopifnot(requireNamespace(\"dplyr\", quietly = TRUE), requireNamespace(\"haven\", quietly = TRUE), \
         requireNamespace(\"tidyr\", quietly = TRUE), requireNamespace(\"nnet\", quietly = TRUE))",
    );
    if have != Outcome::Ok {
        eprintln!("skipped: the R packages dplyr, haven, tidyr and nnet are needed");
        return None;
    }
    Some(e)
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

fn run(e: &mut Engine, code: &str) -> (Outcome, String) {
    let id = e.submit(Source::Typed, code);
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
                    Item::Warning(w) => format!("warning: {w}"),
                    Item::Error { message, .. } => format!("error: {message}"),
                    Item::Notice(n) => n.clone(),
                    Item::Plot { .. } => String::new(),
                })
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default();
    (outcome, text)
}

fn ok(e: &mut Engine, code: &str) -> String {
    let (outcome, text) = run(e, code);
    assert_eq!(outcome, Outcome::Ok, "{code}\n{text}");
    text
}

/// Translates a do-file and insists every line became R.
fn script(dofile: &str) -> String {
    let t = translate(&read(dofile));
    for l in &t.lines {
        assert!(
            l.result.is_ok(),
            "not translated: {}\n{:?}",
            l.stata,
            l.result
        );
    }
    t.script()
}

/// The small data sets, typed in as the Stata run typed them.
fn data(e: &mut Engine) -> String {
    let dir = std::env::temp_dir().join(format!(
        "rcraft-stata-messages-{}-{}",
        std::process::id(),
        fresh()
    ));
    let _ = std::fs::create_dir_all(&dir);
    let d = dir.display().to_string().replace('\\', "/");
    ok(
        e,
        &format!(
            "local({{
  w <- function(x, f) haven::write_dta(x, file.path('{d}', f))
  w(data.frame(id = c(1, 1, 2), t = c(1, 1, 1), v = c(5, 6, 7)), 'dup.dta')
  w(data.frame(id = c(1, 2), u = c(3, 4)), 'one.dta')
  w(data.frame(id = c(1, 1, 2), t = c(1, 2, 1), v = c(5, 6, 7), w = c(1, 2, 3), z = c(1, 3, 4)), 'vary.dta')
  w(data.frame(id = c(1, 1), v1 = c(5, 7), v2 = c(6, 8)), 'wide.dta')
  set.seed(7)
  x <- round(stats::rnorm(30), 2)
  w(data.frame(x = x, y = round(1 + x + stats::rnorm(30), 2), cat3 = rep(c(1, 2, 3), 10),
               s = 'a', s2 = 'b'), 'fit.dta')
}})"
        ),
    );
    d
}

/// Runs a do-file's R and returns whether it stopped, and everything it said.
fn says(e: &mut Engine, dofile: &str) -> (bool, String) {
    let (outcome, text) = run(e, &script(dofile));
    (outcome == Outcome::Error, text)
}

#[test]
fn the_messages_are_statas_word_for_word() {
    let Some(mut e) = engine() else { return };
    let d = data(&mut e);
    let fit = format!("use \"{d}/fit.dta\"\n");
    // (do-file, stops, what Stata says)
    let cases: Vec<(String, bool, Vec<&str>)> = vec![
        (format!("{fit}generate x = 1"), true, vec!["variable x already defined"]),
        (format!("{fit}egen x = mean(y)"), true, vec!["variable x already defined"]),
        (format!("{fit}replace zzz = 1"), true, vec!["variable zzz not found"]),
        (format!("{fit}encode s, generate(x)"), true, vec!["variable x already defined"]),
        (format!("{fit}tostring s, replace"), false, vec!["s already string; no replace"]),
        (
            format!("{fit}correlate x s y"),
            false,
            vec!["(s ignored because string variable)"],
        ),
        (
            format!("{fit}correlate s s2"),
            true,
            vec![
                "(s ignored because string variable)",
                "(s2 ignored because string variable)",
                "too few variables specified",
            ],
        ),
        (format!("{fit}regress y x\npredict x"), true, vec!["variable x already defined"]),
        (format!("{fit}regress y x\ntest zzz"), true, vec!["zzz not found"]),
        (format!("{fit}regress y x\nlincom zzz"), true, vec!["zzz not found"]),
        (
            format!("{fit}regress y x\nmargins, dydx(zzz)"),
            true,
            vec!["invalid dydx() option;", "zzz not found in list of covariates"],
        ),
        (
            format!("{fit}mlogit cat3 x\npredict q1 q2"),
            true,
            vec![
                "too few variables specified",
                "cat3 has 3 outcomes and so you must specify 3 new variables, or you can use the outcome() option and specify variables one at a time",
            ],
        ),
        (
            format!("{fit}mlogit cat3 x\npredict q1 q2 q3 q4"),
            true,
            vec!["too many variables specified", "cat3 has 3 outcomes"],
        ),
        (
            format!("use \"{d}/one.dta\"\nmerge 1:1 id using \"{d}/dup.dta\""),
            true,
            vec!["variable id does not uniquely identify observations in the using data"],
        ),
        (
            format!("use \"{d}/dup.dta\"\nmerge 1:1 id using \"{d}/dup.dta\""),
            true,
            vec!["variable id does not uniquely identify observations in the master data"],
        ),
        (
            format!("use \"{d}/dup.dta\"\nreshape wide v, i(id) j(t)"),
            true,
            vec!["values of variable t not unique within id"],
        ),
        (
            format!("use \"{d}/vary.dta\"\nreshape wide v, i(id) j(t)"),
            true,
            vec!["variable w not constant within id", "variable z not constant within id"],
        ),
        (
            format!("use \"{d}/wide.dta\"\nreshape long v, i(id) j(t)"),
            true,
            vec!["variable id does not uniquely identify the observations"],
        ),
        (
            format!("use \"{d}/dup.dta\"\nduplicates drop id"),
            true,
            vec!["force option required with duplicates drop varlist"],
        ),
    ];
    for (dofile, stops, want) in cases {
        let (stopped, text) = says(&mut e, &format!("{dofile}\n"));
        assert_eq!(stopped, stops, "{dofile}\n{text}");
        for w in want {
            assert!(
                text.contains(w),
                "{dofile}: Stata says {w:?}; the R said\n{text}"
            );
        }
    }
    // save over a file that exists: Stata names the file and nothing more
    let (stopped, text) = says(
        &mut e,
        &format!("use \"{d}/one.dta\"\nsave \"{d}/one.dta\"\n"),
    );
    assert!(stopped, "{text}");
    assert!(text.contains("one.dta already exists"), "{text}");
    assert!(
        !text.contains("replace option"),
        "Stata adds no hint: {text}"
    );
}

/// A number no other call in this run has had: the tests in a file run at
/// once, and each writing its data to the same folder let one read a file
/// another was still writing.
fn fresh() -> usize {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}
