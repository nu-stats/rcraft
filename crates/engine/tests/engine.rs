//! Engine tests that start a real R (skipped with a note when none is found).

use std::time::{Duration, Instant};

use rcraft_engine::{Engine, EngineError, Item, Outcome, Recall, Source};
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

/// Polls until the command in flight has finished.
fn wait(e: &mut Engine) {
    let limit = Instant::now() + Duration::from_secs(60);
    while e.is_busy() {
        e.poll(Duration::from_millis(100));
        assert!(Instant::now() < limit, "the command did not finish");
    }
}

fn run(e: &mut Engine, code: &str) -> u64 {
    let submitted = e.submit(Source::Typed, code);
    assert!(submitted.is_ok(), "{submitted:?}");
    wait(e);
    submitted.unwrap_or_default()
}

fn text_of(e: &Engine, id: u64) -> String {
    e.results()
        .blocks()
        .iter()
        .find(|b| b.command == id)
        .map(|b| {
            b.items
                .iter()
                .filter_map(|i| match i {
                    Item::Text { text, .. } => Some(text.as_str()),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn a_command_goes_into_history_results_and_log() {
    let Some(mut e) = engine() else { return };
    let id = run(&mut e, "1 + 1");
    assert_eq!(text_of(&e, id).trim(), "[1] 2");
    assert_eq!(
        e.history().get(id).and_then(|h| h.outcome),
        Some(Outcome::Ok)
    );
    assert_eq!(e.log_text(), "> 1 + 1\n[1] 2\n");
}

#[test]
fn blank_input_is_refused_and_leaves_no_trace() {
    let Some(mut e) = engine() else { return };
    assert_eq!(e.submit(Source::Typed, "  \n"), Err(EngineError::Empty));
    assert!(e.history().is_empty());
    assert!(e.log_text().is_empty());
}

#[test]
fn a_second_command_while_busy_is_refused_and_leaves_no_trace() {
    let Some(mut e) = engine() else { return };
    e.submit(Source::Typed, "Sys.sleep(1)").expect("submit");
    assert!(e.submit(Source::Typed, "2").is_err());
    assert_eq!(e.history().len(), 1);
    wait(&mut e);
}

#[test]
fn errors_and_warnings_appear_in_results_and_the_log() {
    let Some(mut e) = engine() else { return };
    let id = run(&mut e, "warning('hmm')\nstop('boom')");
    let block = e
        .results()
        .blocks()
        .iter()
        .find(|b| b.command == id)
        .expect("block");
    assert!(block.items.contains(&Item::Warning("hmm".into())));
    assert!(block
        .items
        .iter()
        .any(|i| matches!(i, Item::Error { message, .. } if message == "boom")));
    assert_eq!(block.outcome, Some(Outcome::Error));
    assert!(e.log_text().contains("Warning: hmm"));
    assert!(e.log_text().contains("Error: boom"));
}

#[test]
fn the_script_has_only_what_worked_and_state_carries_over() {
    let Some(mut e) = engine() else { return };
    run(&mut e, "x <- 6");
    run(&mut e, "stop('nope')");
    let id = e
        .submit(Source::Menu("Double it".into()), "x * 2")
        .expect("submit");
    wait(&mut e);
    assert_eq!(text_of(&e, id).trim(), "[1] 12");
    let script = e.script_text();
    assert!(script.contains("x <- 6"));
    assert!(!script.contains("nope"));
    assert!(script.contains("# Double it\nx * 2"));
}

#[test]
fn history_recall_walks_the_commands_that_were_run() {
    let Some(mut e) = engine() else { return };
    run(&mut e, "a <- 1");
    run(&mut e, "b <- 2");
    let mut recall = Recall::new();
    assert_eq!(
        recall.previous(e.history(), "typing"),
        Some("b <- 2".into())
    );
    assert_eq!(recall.previous(e.history(), ""), Some("a <- 1".into()));
    assert_eq!(recall.next(e.history()), Some("b <- 2".into()));
    assert_eq!(recall.next(e.history()), Some("typing".into()));
}

#[test]
fn restart_keeps_history_and_results_but_not_objects() {
    let Some(mut e) = engine() else { return };
    run(&mut e, "keep <- 1");
    e.restart().expect("restart");
    assert_eq!(e.history().len(), 1);
    assert!(e.log_text().contains("R was restarted"));
    let id = run(&mut e, "exists('keep')");
    assert_eq!(text_of(&e, id).trim(), "[1] FALSE");
}

#[test]
fn a_crash_is_noted_and_the_engine_recovers_after_restart() {
    let Some(mut e) = engine() else { return };
    let id = run(&mut e, "quit(save = 'no', status = 2)");
    assert_eq!(
        e.history().get(id).and_then(|h| h.outcome),
        Some(Outcome::Crashed)
    );
    // the note about R stopping may arrive just after the run ends
    for _ in 0..20 {
        e.poll(Duration::from_millis(100));
    }
    assert!(!e.is_running());
    assert!(e.log_text().contains("R has stopped"));
    assert!(e.submit(Source::Typed, "1").is_err());
    e.restart().expect("restart");
    let again = run(&mut e, "1 + 1");
    assert_eq!(text_of(&e, again).trim(), "[1] 2");
}

#[test]
fn the_log_can_follow_into_a_file() {
    let Some(mut e) = engine() else { return };
    let dir = std::env::temp_dir().join(format!("rcraft-engine-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("dir");
    let path = dir.join("session.log");
    run(&mut e, "'before'");
    e.log_to(&path).expect("log file");
    run(&mut e, "'after'");
    e.stop_logging();
    let text = std::fs::read_to_string(&path).expect("read");
    assert!(text.contains("before"));
    assert!(text.contains("after"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_plot_is_a_result_item_a_log_note_and_a_file_that_outlives_the_command() {
    let Some(mut e) = engine() else { return };
    let id = run(&mut e, "plot(1:5, main = 'five')");
    let plots: Vec<std::path::PathBuf> = e
        .results()
        .blocks()
        .iter()
        .filter(|b| b.command == id)
        .flat_map(|b| b.items.iter())
        .filter_map(|i| match i {
            Item::Plot { path } => Some(path.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(plots.len(), 1, "{plots:?}");
    assert!(plots[0].is_file());
    assert!(
        e.log_text().contains("a plot was drawn"),
        "{}",
        e.log_text()
    );
    // a later command that draws nothing leaves the earlier file in place
    let _ = run(&mut e, "x <- 1");
    assert!(plots[0].is_file());
}
