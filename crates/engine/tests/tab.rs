//! Tab completion against a real R (skipped with a note when none is found):
//! the code the window sends is run as a hidden query and the answer is read
//! back, so what is tested is what Tab does.

use std::time::{Duration, Instant};

use rcraft_engine::{Engine, Source};
use rcraft_rcodegen::tab::{apply, completion_code, parse_answer};
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

/// Like `tab`, but nothing offered is an empty answer, which the checks then name.
fn offer(e: &mut Engine, line: &str) -> (Vec<String>, String) {
    tab(e, line).unwrap_or_default()
}

fn run(e: &mut Engine, code: &str) {
    assert!(e.submit(Source::Typed, code).is_ok());
    settle(e);
}

/// What Tab would offer for `line`, as the choices and the line after Tab.
fn tab(e: &mut Engine, line: &str) -> Option<(Vec<String>, String)> {
    let id = e.query(&completion_code(line));
    settle(e);
    let r = e.query_result(id)?;
    assert!(r.ok, "completion code failed: {}", r.text);
    let c = parse_answer(line, &r.text)?;
    let (done, _) = apply(line, &c);
    Some((c.options, done))
}

#[test]
fn names_columns_packages_and_files_complete() {
    let Some(mut e) = engine() else { return };
    let dir = std::env::temp_dir().join(format!("rcraft-tab-{}", std::process::id()));
    assert!(std::fs::create_dir_all(&dir).is_ok());
    std::fs::write(dir.join("survey_2020.csv"), "a\n1\n").expect("file");
    std::fs::write(dir.join("survey_2021.csv"), "a\n1\n").expect("file");
    let wd = dir
        .to_string_lossy()
        .replace(std::path::MAIN_SEPARATOR, "/");
    run(
        &mut e,
        &format!("setwd(\"{wd}\"); my_data <- data.frame(alpha = 1, beta = 2)"),
    );

    // an object, in the middle of a line
    let (options, done) = offer(&mut e, "x <- my_d");
    assert_eq!(options, vec!["my_data"]);
    assert_eq!(done, "x <- my_data");

    // a column after $
    let (options, done) = offer(&mut e, "summary(my_data$a");
    assert_eq!(options, vec!["my_data$alpha"]);
    assert_eq!(done, "summary(my_data$alpha");

    // a function, which opens its call
    let (options, _) = offer(&mut e, "mea");
    assert!(options.iter().any(|o| o == "mean("), "{options:?}");

    // package::name and library(
    let (options, _) = offer(&mut e, "stats::rnor");
    assert_eq!(options, vec!["stats::rnorm"]);
    let (options, done) = offer(&mut e, "library(tool");
    assert_eq!(options, vec!["tools"]);
    assert_eq!(done, "library(tools");

    // file names inside quotes, completed as far as they agree
    let (options, done) = offer(&mut e, "read.csv(\"surv");
    assert_eq!(options, vec!["survey_2020.csv", "survey_2021.csv"]);
    assert_eq!(done, "read.csv(\"survey_202");
    let (_, done) = offer(&mut e, "read.csv('survey_2021");
    assert_eq!(done, "read.csv('survey_2021.csv");

    // nothing to offer: no answer, and no harm done
    assert!(tab(&mut e, "zzzzqqq").is_none());

    // odd lines never break the session
    for line in ["print(\"a\\", "'", "x$", "((((", "é", "a\"b\"c\"", "\\"] {
        let _ = tab(&mut e, line);
    }
    run(&mut e, "1 + 1");
    assert!(e.history().len() >= 2);
    let _ = std::fs::remove_dir_all(&dir);
}
