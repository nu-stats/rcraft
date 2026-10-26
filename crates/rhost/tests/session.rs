//! Tests that start a real R. They use the first R the finder reports and
//! are skipped (with a note) on a machine that has none.

use std::time::Duration;

use rcraft_rhost::{
    find_installs, Env, Event, InterruptOutcome, Limit, RunStatus, Session, SessionConfig,
    SessionError,
};

const LIMIT: Duration = Duration::from_secs(60);

fn session() -> Option<Session> {
    let install = find_installs(&Env::from_process())
        .into_iter()
        .find(|i| i.is_supported());
    let Some(install) = install else {
        eprintln!("skipped: no supported R on this machine");
        return None;
    };
    match Session::start(SessionConfig::new(install)) {
        Ok(s) => Some(s),
        Err(e) => {
            eprintln!("could not start R: {e}");
            None
        }
    }
}

#[test]
fn runs_code_and_returns_printed_output() {
    let Some(mut s) = session() else { return };
    assert!(s.r_version().starts_with("R version"));
    let out = s.run_to_end("1 + 1", LIMIT).expect("run");
    assert_eq!(out.status, RunStatus::Ok);
    assert_eq!(out.stdout.trim(), "[1] 2");
    assert!(out.error.is_none());
}

#[test]
fn state_persists_between_runs() {
    let Some(mut s) = session() else { return };
    s.run_to_end("x <- 21", LIMIT).expect("assign");
    let out = s.run_to_end("x * 2", LIMIT).expect("use");
    assert_eq!(out.stdout.trim(), "[1] 42");
}

#[test]
fn invisible_results_print_nothing() {
    let Some(mut s) = session() else { return };
    let out = s.run_to_end("y <- 5\ninvisible(7)", LIMIT).expect("run");
    assert_eq!(out.stdout, "");
}

#[test]
fn errors_are_reported_with_message_and_call() {
    let Some(mut s) = session() else { return };
    let out = s.run_to_end("stop('boom')", LIMIT).expect("run");
    assert_eq!(out.status, RunStatus::Error);
    let error = out.error.expect("error");
    assert_eq!(error.message, "boom");
    // the session is still usable afterwards
    let again = s.run_to_end("2 + 2", LIMIT).expect("run");
    assert_eq!(again.stdout.trim(), "[1] 4");
}

#[test]
fn a_parse_error_runs_nothing() {
    let Some(mut s) = session() else { return };
    let out = s.run_to_end("z <- 1\nthis is not R", LIMIT).expect("run");
    assert_eq!(out.status, RunStatus::Error);
    let check = s.run_to_end("exists('z')", LIMIT).expect("run");
    assert_eq!(check.stdout.trim(), "[1] FALSE");
}

#[test]
fn code_after_an_error_does_not_run() {
    let Some(mut s) = session() else { return };
    let out = s
        .run_to_end("cat('a\\n')\nstop('x')\ncat('b\\n')", LIMIT)
        .expect("run");
    assert_eq!(out.status, RunStatus::Error);
    assert_eq!(out.stdout.trim(), "a");
}

#[test]
fn warnings_and_messages_are_kept_apart() {
    let Some(mut s) = session() else { return };
    let out = s
        .run_to_end("warning('careful')\nmessage('note')\ncat('done\\n')", LIMIT)
        .expect("run");
    assert_eq!(out.status, RunStatus::Ok);
    assert_eq!(out.warnings, ["careful"]);
    assert!(out.stderr.contains("note"));
    assert_eq!(out.stdout.trim(), "done");
}

#[test]
fn text_with_unicode_and_the_word_token_survives() {
    let Some(mut s) = session() else { return };
    let out = s
        .run_to_end(
            "cat('caf\u{e9} \u{4e2d}\u{6587}\\n')\ncat('rcraft\\tEND\\t1\\tok\\n')",
            LIMIT,
        )
        .expect("run");
    assert_eq!(out.status, RunStatus::Ok);
    assert!(out.stdout.contains("caf\u{e9} \u{4e2d}\u{6587}"));
    // printed text that looks like a frame is just output
    assert!(out.stdout.contains("rcraft\tEND\t1\tok"));
}

#[test]
fn streams_output_as_events() {
    let Some(mut s) = session() else { return };
    let id = s.run("cat('one\\n')\ncat('two\\n')").expect("run");
    let mut text = String::new();
    loop {
        match s.next_event(LIMIT).expect("event") {
            Event::Output { run, text: t, .. } if run == id => text.push_str(&t),
            Event::Finished { run, status } if run == id => {
                assert_eq!(status, RunStatus::Ok);
                break;
            }
            _ => {}
        }
    }
    assert_eq!(text, "one\ntwo\n");
}

#[test]
fn a_second_run_while_busy_is_refused() {
    let Some(mut s) = session() else { return };
    s.run("Sys.sleep(2)").expect("run");
    assert_eq!(s.run("1"), Err(SessionError::Busy));
    s.interrupt();
}

#[test]
fn huge_output_is_cut_off() {
    let Some(mut s) = session() else { return };
    let mut config = SessionConfig::new(
        find_installs(&Env::from_process())
            .into_iter()
            .next()
            .expect("r"),
    );
    config.max_output_bytes = 1000;
    drop(s);
    s = Session::start(config).expect("start");
    let out = s
        .run_to_end("for (i in 1:2000) cat('line', i, '\\n')", LIMIT)
        .expect("run");
    assert!(out.truncated);
    assert!(out.stdout.len() <= 1000);
    assert_eq!(out.status, RunStatus::Ok);
}

#[test]
fn a_very_long_line_does_not_stall() {
    let Some(mut s) = session() else { return };
    let out = s
        .run_to_end("cat(strrep('a', 300000), '\\n')", LIMIT)
        .expect("run");
    assert_eq!(out.status, RunStatus::Ok);
    assert!(out.stdout.len() >= 300000);
}

#[test]
fn interrupt_ends_a_long_run() {
    let Some(mut s) = session() else { return };
    s.run("Sys.sleep(120)").expect("run");
    std::thread::sleep(Duration::from_millis(500));
    let outcome = s.interrupt();
    let mut status = None;
    while let Some(event) = s.next_event(Duration::from_secs(30)) {
        if let Event::Finished { status: st, .. } = event {
            status = Some(st);
            break;
        }
    }
    assert_eq!(status, Some(RunStatus::Interrupted));
    match outcome {
        InterruptOutcome::Signalled => {
            // R is still up and usable
            let out = s.run_to_end("1 + 1", LIMIT).expect("run");
            assert_eq!(out.stdout.trim(), "[1] 2");
        }
        InterruptOutcome::SessionStopped => {
            assert!(!s.is_running() || s.run("1").is_err());
        }
        InterruptOutcome::Idle => panic!("a run was in progress"),
    }
}

#[test]
fn interrupt_when_idle_does_nothing() {
    let Some(mut s) = session() else { return };
    assert_eq!(s.interrupt(), InterruptOutcome::Idle);
}

#[test]
fn restart_gives_a_clean_session() {
    let Some(mut s) = session() else { return };
    s.run_to_end("keep <- 1", LIMIT).expect("run");
    s.restart().expect("restart");
    let out = s.run_to_end("exists('keep')", LIMIT).expect("run");
    assert_eq!(out.stdout.trim(), "[1] FALSE");
}

#[test]
fn a_crash_is_reported_and_the_session_can_restart() {
    let Some(mut s) = session() else { return };
    let out = s
        .run_to_end("quit(save = 'no', status = 3)", LIMIT)
        .expect("run");
    assert_eq!(out.status, RunStatus::Crashed);
    assert!(!s.is_running());
    assert_eq!(s.run("1"), Err(SessionError::NotRunning));
    s.restart().expect("restart");
    let again = s.run_to_end("1 + 1", LIMIT).expect("run");
    assert_eq!(again.stdout.trim(), "[1] 2");
}

fn session_with(set: impl FnOnce(&mut SessionConfig)) -> Option<Session> {
    let install = find_installs(&Env::from_process())
        .into_iter()
        .find(|i| i.is_supported())?;
    let mut config = SessionConfig::new(install);
    set(&mut config);
    Session::start(config).ok()
}

#[test]
fn r_reports_its_own_process_id() {
    let Some(mut s) = session() else { return };
    assert!(s.r_pid() > 0);
    let out = s.run_to_end("Sys.getpid()", LIMIT).expect("run");
    assert_eq!(out.stdout.trim(), format!("[1] {}", s.r_pid()));
}

#[test]
fn a_run_over_the_time_limit_is_stopped() {
    let Some(mut s) = session_with(|c| c.max_run_time = Some(Duration::from_secs(1))) else {
        return;
    };
    let started = std::time::Instant::now();
    let out = s.run_to_end("Sys.sleep(120)", LIMIT).expect("run");
    assert_eq!(out.limit, Some(Limit::Time));
    assert_eq!(out.status, RunStatus::Interrupted);
    assert!(started.elapsed() < Duration::from_secs(30));
}

#[test]
fn a_run_under_the_time_limit_is_left_alone() {
    let Some(mut s) = session_with(|c| c.max_run_time = Some(Duration::from_secs(30))) else {
        return;
    };
    let out = s.run_to_end("1 + 1", LIMIT).expect("run");
    assert_eq!(out.limit, None);
    assert_eq!(out.status, RunStatus::Ok);
}

#[test]
fn a_run_over_the_memory_limit_is_stopped() {
    let Some(mut s) = session_with(|c| c.max_memory_bytes = Some(300 * 1024 * 1024)) else {
        return;
    };
    let out = s
        .run_to_end("x <- numeric(8e7); x[] <- 1; Sys.sleep(120)", LIMIT)
        .expect("run");
    assert_eq!(out.limit, Some(Limit::Memory));
    assert_eq!(out.status, RunStatus::Interrupted);
    assert!(!s.is_running());
    // the session can be restarted afterwards
    s.restart().expect("restart");
    let again = s.run_to_end("1 + 1", LIMIT).expect("run");
    assert_eq!(again.stdout.trim(), "[1] 2");
}

#[test]
fn messages_are_never_lost_to_the_end_of_the_run() {
    // standard error and standard output are read by different threads; the
    // run must not be reported finished before the last message is in
    let Some(mut s) = session() else { return };
    for round in 0..15 {
        let out = s
            .run_to_end("for (i in 1:50) message('m', i)\ncat('done\\n')", LIMIT)
            .expect("run");
        assert_eq!(out.status, RunStatus::Ok, "round {round}");
        assert!(
            out.stderr.contains("m50\n"),
            "round {round}: {:?}",
            out.stderr
        );
        assert_eq!(out.stderr.lines().count(), 50, "round {round}");
    }
}
