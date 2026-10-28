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

// Regression tests for the rproto/session review (P3 session fixes).

#[test]
fn a_sink_left_on_does_not_hang_the_session() {
    let Some(mut s) = session() else { return };
    let out = s
        .run_to_end("sink(tempfile()); warning('inside'); 1", LIMIT)
        .expect("run");
    assert_eq!(out.status, RunStatus::Ok);
    assert_eq!(out.warnings.first().map(String::as_str), Some("inside"));
    assert!(out.warnings.iter().any(|w| w.contains("sink()")), "{out:?}");
    // the sink was turned off, so output shows again
    let again = s.run_to_end("1 + 1", LIMIT).expect("run");
    assert_eq!(again.stdout.trim(), "[1] 2");
    assert!(again.warnings.is_empty());
}

#[test]
fn a_message_sink_left_on_does_not_hang_the_session() {
    let Some(mut s) = session() else { return };
    let out = s
        .run_to_end(
            "zz <- file(tempfile(), 'w'); sink(zz, type = 'message'); message('hidden')",
            LIMIT,
        )
        .expect("run");
    assert_eq!(out.status, RunStatus::Ok);
    let again = s.run_to_end("message('seen')", LIMIT).expect("run");
    assert!(again.stderr.contains("seen"), "{again:?}");
}

#[test]
fn capture_output_does_not_capture_rcraft_frames() {
    let Some(mut s) = session() else { return };
    let out = s
        .run_to_end("x <- capture.output({warning('w'); cat('a\\n')}); x", LIMIT)
        .expect("run");
    assert_eq!(out.status, RunStatus::Ok);
    assert_eq!(out.stdout.trim(), "[1] \"a\"");
    assert_eq!(out.warnings, ["w"]);
}

#[test]
fn a_frame_after_long_unterminated_output_is_found() {
    let Some(mut s) = session() else { return };
    // the END frame's marker starts on either side of the 64 KiB read boundary
    for n in [65513, 65520, 65526, 65535, 65536] {
        let out = s
            .run_to_end(&format!("cat(strrep('a', {n}))"), LIMIT)
            .expect("run");
        assert_eq!(out.status, RunStatus::Ok, "{n}");
        assert_eq!(out.stdout.len(), n, "{n}");
    }
}

#[test]
fn a_character_on_the_read_boundary_is_not_garbled() {
    let Some(mut s) = session() else { return };
    let out = s
        .run_to_end("cat(strrep('a', 65535), '\\u00e9\\n', sep = '')", LIMIT)
        .expect("run");
    assert!(!out.stdout.contains('\u{fffd}'));
    assert!(out.stdout.ends_with("a\u{e9}\n"));
}

#[test]
fn an_interrupt_racing_the_end_of_a_run_does_not_hang_it() {
    let Some(mut s) = session() else { return };
    for round in 0..20 {
        let id = s.run("1").expect("run");
        let outcome = s.interrupt();
        let mut finished = false;
        while let Some(event) = s.next_event(Duration::from_secs(20)) {
            if matches!(event, Event::Finished { run, .. } if run == id) {
                finished = true;
                break;
            }
        }
        assert!(finished, "round {round}: the run never ended");
        if outcome == InterruptOutcome::SessionStopped {
            // no signals on Windows: the interrupt stopped R
            s.restart().expect("restart");
        }
        let out = s.run_to_end("2 + 2", LIMIT).expect("run");
        assert_eq!(out.stdout.trim(), "[1] 4", "round {round}");
    }
}

#[test]
fn an_interrupt_r_does_not_answer_ends_in_stopping_r() {
    let Some(mut s) = session() else { return };
    let id = s.run("suspendInterrupts(Sys.sleep(60))").expect("run");
    std::thread::sleep(Duration::from_millis(500));
    let started = std::time::Instant::now();
    s.interrupt();
    let mut status = None;
    while let Some(event) = s.next_event(Duration::from_secs(30)) {
        if let Event::Finished { run, status: st } = event {
            if run == id {
                status = Some(st);
                break;
            }
        }
    }
    assert_eq!(status, Some(RunStatus::Interrupted));
    assert!(started.elapsed() < Duration::from_secs(25));
}

#[test]
fn a_timed_out_run_to_end_leaves_the_session_free() {
    let Some(mut s) = session() else { return };
    let out = s.run_to_end("Sys.sleep(60)", Duration::from_secs(1));
    assert_eq!(out, Err(SessionError::TimedOut));
    assert!(!s.is_busy());
    if !s.is_running() {
        s.restart().expect("restart");
    }
    let again = s.run_to_end("1 + 1", LIMIT).expect("run");
    assert_eq!(again.stdout.trim(), "[1] 2");
}

#[test]
fn options_warn_is_respected() {
    let Some(mut s) = session() else { return };
    let strict = s
        .run_to_end("options(warn = 2); warning('strict')", LIMIT)
        .expect("run");
    assert_eq!(strict.status, RunStatus::Error);
    assert!(strict
        .error
        .as_ref()
        .is_some_and(|e| e.message.contains("strict")));
    let quiet = s
        .run_to_end("options(warn = -1); warning('quiet'); 1", LIMIT)
        .expect("run");
    assert_eq!(quiet.status, RunStatus::Ok);
    assert!(quiet.warnings.is_empty(), "{quiet:?}");
    let normal = s
        .run_to_end("options(warn = 0); warning('normal')", LIMIT)
        .expect("run");
    assert_eq!(normal.warnings, ["normal"]);
}

#[test]
fn a_warning_with_no_muffle_restart_is_not_an_error() {
    let Some(mut s) = session() else { return };
    let out = s
        .run_to_end(
            "invisible(signalCondition(simpleWarning('bare'))); 1",
            LIMIT,
        )
        .expect("run");
    assert_eq!(out.status, RunStatus::Ok);
    assert_eq!(out.warnings, ["bare"]);
    assert_eq!(out.stdout.trim(), "[1] 1");
}

#[test]
fn r_s_last_words_on_standard_error_are_kept() {
    let Some(mut s) = session() else { return };
    for round in 0..10 {
        let out = s
            .run_to_end(
                "cat('dying words\\n', file = stderr()); quit(save = 'no', status = 4)",
                LIMIT,
            )
            .expect("run");
        assert_eq!(out.status, RunStatus::Crashed, "round {round}");
        assert!(out.stderr.contains("dying words"), "round {round}: {out:?}");
        s.restart().expect("restart");
    }
}

#[test]
fn close_all_connections_does_not_end_the_session() {
    let Some(mut s) = session() else { return };
    let out = s.run_to_end("closeAllConnections()", LIMIT).expect("run");
    assert_eq!(out.status, RunStatus::Ok);
    let again = s.run_to_end("1 + 1", LIMIT).expect("run");
    assert_eq!(again.stdout.trim(), "[1] 2");
    // R gives the closed connection's number to the next one opened
    s.run_to_end(
        "closeAllConnections(); keep <- file(tempfile(), 'w')",
        LIMIT,
    )
    .expect("run");
    let third = s.run_to_end("2 + 2", LIMIT).expect("run");
    assert_eq!(third.stdout.trim(), "[1] 4");
    assert!(s.is_running());
}

#[test]
fn output_without_a_final_line_break_does_not_hang_the_run() {
    let Some(mut s) = session() else { return };
    let out = s.run_to_end("cat('hi')", LIMIT).expect("run");
    assert_eq!(out.status, RunStatus::Ok);
    assert_eq!(out.stdout, "hi");
    // and the next run is unaffected
    let out = s
        .run_to_end("cat('a'); cat('b', file = stderr()); 1", LIMIT)
        .expect("run");
    assert_eq!(out.stdout, "a[1] 1\n");
    assert_eq!(out.stderr, "b");
}

/// Runs `code` and returns the plots it drew, in order, and how it ended.
fn plots_of(s: &mut Session, code: &str) -> (Vec<std::path::PathBuf>, RunStatus) {
    let Ok(id) = s.run(code) else {
        return (Vec::new(), RunStatus::Crashed);
    };
    let mut plots = Vec::new();
    loop {
        match s.next_event(LIMIT) {
            Some(Event::Plot { run, path }) if run == id => plots.push(path),
            Some(Event::Finished { run, status }) if run == id => return (plots, status),
            Some(_) => {}
            // nothing came back in time: the tests that call this then fail on what they expect
            None => return (plots, RunStatus::Crashed),
        }
    }
}

fn is_png(path: &std::path::Path) -> bool {
    std::fs::read(path)
        .is_ok_and(|b| b.starts_with(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]))
}

#[test]
fn a_plot_comes_back_as_a_png_file() {
    let Some(mut s) = session() else { return };
    let (plots, status) = plots_of(&mut s, "plot(1:10, main = 'ten')");
    assert_eq!(status, RunStatus::Ok);
    assert_eq!(plots.len(), 1, "{plots:?}");
    assert!(is_png(&plots[0]), "{:?}", plots[0]);
    assert!(std::fs::metadata(&plots[0]).map_or(0, |m| m.len()) > 1000);
}

#[test]
fn each_page_is_its_own_file_in_page_order() {
    let Some(mut s) = session() else { return };
    let (plots, status) = plots_of(&mut s, "plot(1:3)\nhist(rnorm(50))\nboxplot(1:5)");
    assert_eq!(status, RunStatus::Ok);
    assert_eq!(plots.len(), 3, "{plots:?}");
    let names: Vec<String> = plots
        .iter()
        .filter_map(|p| p.file_name().and_then(|n| n.to_str()).map(String::from))
        .collect();
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted);
    assert!(plots.iter().all(|p| is_png(p)));
}

#[test]
fn code_that_draws_nothing_brings_no_plot_and_runs_do_not_share_plots() {
    let Some(mut s) = session() else { return };
    let (plots, _) = plots_of(&mut s, "x <- 1 + 1; print(x)");
    assert!(plots.is_empty());
    let (first, _) = plots_of(&mut s, "plot(1:3)");
    assert_eq!(first.len(), 1);
    let (none, _) = plots_of(&mut s, "cat('no plot here\n')");
    assert!(
        none.is_empty(),
        "the earlier run's plot must not come back: {none:?}"
    );
    let (second, _) = plots_of(&mut s, "plot(3:1)");
    assert_eq!(second.len(), 1);
    assert_ne!(first, second);
    // the first run's file is still there, so a Graph window can keep a history
    assert!(first[0].is_file());
}

#[test]
fn a_plot_drawn_before_an_error_is_kept() {
    let Some(mut s) = session() else { return };
    let (plots, status) = plots_of(&mut s, "plot(1:5)\nstop('after the plot')");
    assert_eq!(status, RunStatus::Error);
    assert_eq!(plots.len(), 1);
    assert!(is_png(&plots[0]));
    // and the session still works afterwards
    let out = s.run_to_end("1 + 1", LIMIT).expect("run");
    assert_eq!(out.status, RunStatus::Ok);
}

#[test]
fn a_run_stopped_midway_still_closes_its_device() {
    let Some(mut s) = session() else { return };
    let id = s.run("plot(1:5)\nSys.sleep(30)").expect("run");
    // wait for the run to begin, then stop it
    loop {
        if matches!(s.next_event(LIMIT).expect("event"), Event::Started { run } if run == id) {
            break;
        }
    }
    std::thread::sleep(std::time::Duration::from_millis(600));
    let _ = s.interrupt();
    let mut plots = Vec::new();
    loop {
        match s.next_event(LIMIT).expect("event") {
            Event::Plot { run, path } if run == id => plots.push(path),
            Event::Finished { run, .. } if run == id => break,
            Event::Exited { .. } => break,
            _ => {}
        }
    }
    // where an interrupt stops R and it restarts, there is nothing to say about
    // the plot; where R survives, the page it had drawn is complete
    for p in &plots {
        assert!(is_png(p), "{p:?}");
    }
}

#[test]
fn the_plot_folder_goes_away_with_the_session() {
    let Some(mut s) = session() else { return };
    let (plots, _) = plots_of(&mut s, "plot(1:3)");
    let file = plots.first().cloned().expect("a plot");
    assert!(file.is_file());

    drop(s);
    assert!(!file.exists(), "{file:?}");
}

#[test]
fn a_plot_can_be_added_to_by_the_next_command() {
    let Some(mut s) = session() else { return };
    let (first, status) = plots_of(&mut s, "plot(1:10)");
    assert_eq!(status, RunStatus::Ok);
    assert_eq!(first.len(), 1);
    // abline() needs the plot of the earlier command: the device stays open
    let (second, status) = plots_of(&mut s, "abline(h = 5, col = 'red')");
    assert_eq!(status, RunStatus::Ok, "abline must find the earlier plot");
    assert_eq!(second.len(), 1, "{second:?}");
    assert_ne!(first, second);
    // the added line changes the picture
    assert_ne!(
        std::fs::read(&first[0]).unwrap_or_default(),
        std::fs::read(&second[0]).unwrap_or_default()
    );
    // a command that adds nothing leaves the history alone
    let (none, _) = plots_of(&mut s, "x <- 1");
    assert!(none.is_empty(), "{none:?}");
    // and a new plot starts a new page
    let (third, _) = plots_of(&mut s, "plot(10:1)");
    assert_eq!(third.len(), 1);
}

#[test]
fn closing_the_device_in_the_user_s_code_does_not_break_the_next_plot() {
    let Some(mut s) = session() else { return };
    let (a, _) = plots_of(&mut s, "plot(1:3)\ndev.off()");
    let _ = a;
    let (b, status) = plots_of(&mut s, "plot(3:1)");
    assert_eq!(status, RunStatus::Ok);
    assert_eq!(b.len(), 1, "{b:?}");
    assert!(is_png(&b[0]));
}

#[test]
fn a_file_device_the_user_opens_is_left_alone() {
    let Some(mut s) = session() else { return };
    let dir = std::env::temp_dir().join(format!("rcraft-userplot-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let file = dir.join("mine.png").to_string_lossy().replace('\\', "/");
    let (plots, status) = plots_of(&mut s, &format!("png('{file}')\nplot(1:3)\ndev.off()"));
    assert_eq!(status, RunStatus::Ok);
    assert!(plots.is_empty(), "the user chose a file: {plots:?}");
    assert!(std::path::Path::new(&file).is_file());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn par_before_the_first_plot_does_not_make_a_blank_page() {
    let Some(mut s) = session() else { return };
    // barplot() calls par() before plot.new(), which opens the device first
    let (plots, status) = plots_of(&mut s, "barplot(c(a = 1, b = 2))");
    assert_eq!(status, RunStatus::Ok);
    assert_eq!(plots.len(), 1, "{plots:?}");
    // and so does an earlier plot followed by a par() call and a new plot
    let (again, _) = plots_of(&mut s, "op <- par(yaxs = 'i')\nplot(1:3)");
    assert_eq!(again.len(), 1, "{again:?}");
}

#[test]
fn the_panels_of_one_page_are_one_plot() {
    let Some(mut s) = session() else { return };
    let (plots, status) = plots_of(&mut s, "par(mfrow = c(2, 2))\nfor (i in 1:4) plot(i)");
    assert_eq!(status, RunStatus::Ok);
    assert_eq!(plots.len(), 1, "{plots:?}");
    // a scatter plot matrix draws a grid of panels
    let (matrix, _) = plots_of(
        &mut s,
        "par(mfrow = c(1, 1))\npairs(data.frame(a = 1:5, b = c(2, 1, 4, 3, 5), c = 5:1))",
    );
    assert_eq!(matrix.len(), 1, "{matrix:?}");
    // a command that only changes the layout is not a plot
    let (layout, _) = plots_of(&mut s, "par(mfrow = c(2, 1))");
    assert!(layout.is_empty(), "{layout:?}");
}
