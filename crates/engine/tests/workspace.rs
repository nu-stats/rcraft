//! Hidden workspace queries against a real R (skipped with a note when none
//! is found).

use std::time::{Duration, Instant};

use rcraft_engine::{Engine, Source};
use rcraft_rdata::Data;
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

/// Polls until nothing is running or waiting.
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

fn run(e: &mut Engine, code: &str) {
    let r = e.submit(Source::Typed, code);
    assert!(r.is_ok(), "{r:?}");
    settle(e);
}

#[test]
fn the_workspace_is_listed_without_a_trace_in_the_log() {
    let Some(mut e) = engine() else { return };
    settle(&mut e); // the first listing: empty
    assert!(e.workspace().is_empty());
    run(
        &mut e,
        "df <- data.frame(a = 1:3); f <- function(x) x; `we\"ird` <- 1",
    );
    let names: Vec<(&str, bool)> = e
        .workspace()
        .iter()
        .map(|w| (w.name.as_str(), w.is_data_frame()))
        .collect();
    assert_eq!(names, [("df", true), ("f", false), ("we\"ird", false)]);
    let df = e.workspace().iter().find(|w| w.name == "df");
    assert_eq!(df.and_then(|w| w.dims), Some((3, 1)));
    // only the user's command is in the history, the log and the results
    assert_eq!(e.history().len(), 1);
    assert!(!e.log_text().contains("ls(globalenv"));
    assert_eq!(e.results().blocks().len(), 1);
}

#[test]
fn a_watched_table_arrives_and_follows_the_commands() {
    let Some(mut e) = engine() else { return };
    run(&mut e, "df <- data.frame(a = 1:3, b = c('x', NA, 'z'))");
    e.watch("df");
    settle(&mut e);
    let t = e.table("df");
    assert!(t.is_some(), "{:?}", e.table_error("df"));
    let Some(t) = t else { return };
    assert_eq!((t.nrow, t.columns.len()), (3, 2));

    // a command that changes it brings it over again
    run(&mut e, "df$c <- df$a * 2");
    let Some(t) = e.table("df") else {
        panic!("{:?}", e.table_error("df"))
    };
    assert_eq!(t.columns.len(), 3);
    assert_eq!(
        t.columns.get(2).map(|c| &c.data),
        Some(&Data::Double(vec![2.0, 4.0, 6.0]))
    );

    // removing it, or making it something else, is reported, not stale data
    run(&mut e, "df <- 5");
    assert!(e.table("df").is_none());
    assert!(e.table_error("df").is_some());
    run(&mut e, "df <- data.frame(z = 1)");
    e.refresh();
    settle(&mut e);
    assert!(
        e.table("df").is_some(),
        "{:?} {:?}",
        e.table_error("df"),
        e.workspace()
    );

    e.unwatch("df");
    assert!(e.table("df").is_none());
    assert!(e.table_error("df").is_none());
}

#[test]
fn watching_something_that_is_not_a_table_gives_a_reason() {
    let Some(mut e) = engine() else { return };
    run(&mut e, "x <- 1:3");
    e.watch("x");
    settle(&mut e);
    assert!(e.table("x").is_none());
    assert!(e.table_error("x").is_some_and(|m| m.contains("data frame")));
    e.watch("nothing_here");
    settle(&mut e);
    assert!(e.table_error("nothing_here").is_some());
}

#[test]
fn a_command_typed_during_a_hidden_query_is_not_refused() {
    let Some(mut e) = engine() else { return };
    run(&mut e, "df <- data.frame(a = 1:100000)");
    e.watch("df");
    // do not settle: a fetch is about to start or running
    for _ in 0..5 {
        e.poll(Duration::from_millis(1));
        let r = e.submit(Source::Typed, "1 + 1");
        assert!(r.is_ok(), "{r:?}");
        settle(&mut e);
    }
    assert!(e.table("df").is_some());
    assert_eq!(e.history().len(), 6);
}

#[test]
fn restart_empties_the_workspace_and_asks_for_it_again() {
    let Some(mut e) = engine() else { return };
    run(&mut e, "df <- data.frame(a = 1)");
    assert_eq!(e.workspace().len(), 1);
    assert!(e.restart().is_ok());
    settle(&mut e);
    assert!(e.workspace().is_empty());
}
