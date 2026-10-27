//! P1.3: one supervised R process.
//!
//! A [`Session`] starts `Rscript` running the protocol loop from `rproto`,
//! sends it code, and turns what comes back into [`Event`]s: printed output,
//! messages, warnings, errors and the end of each run. R is a separate
//! process (AGENTS.md section 1), so if it crashes, hangs or is stopped the
//! app is unaffected and the session can be restarted.
//!
//! One run at a time. Output reaches the caller through a bounded channel, so
//! a caller that stops reading slows R down instead of growing memory, and a
//! run that prints more than the cap is cut off with a note.
//!
//! Limits worth knowing:
//! - Standard output and standard error are read by separate threads, so the
//!   order between a message and the output around it is approximate.
//! - R buffers its standard output when piped; output appears when R flushes
//!   (after each top-level expression and at the end of a run, or when the
//!   code calls `flush.console()`).
//! - Interrupting a run sends SIGINT on Unix. Windows has no way to do that
//!   without `unsafe`, which RCraft forbids, so there the interrupt stops the
//!   session and the caller restarts it (state in memory is lost).

use std::collections::VecDeque;
use std::fmt;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, RecvTimeoutError, SyncSender};
use std::thread;
use std::time::{Duration, Instant};

use rcraft_rproto::{
    encode_run, frame_marker, new_token, parse_frame, parse_frame_after_text, Frame, ProtoError,
    RunId, Status, BOOTSTRAP_R, QUIT,
};

use crate::find::RInstall;
use crate::procinfo;

/// Longest chunk read from R in one piece when it prints no newline.
const MAX_CHUNK_BYTES: u64 = 64 * 1024;
/// Events buffered between the reader threads and the caller.
const CHANNEL_CAPACITY: usize = 1024;
/// How much of R's standard error is kept for start-up failure messages.
const STDERR_TAIL_BYTES: usize = 4096;

/// Which output stream a piece of text came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stream {
    Stdout,
    Stderr,
}

/// How a run ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunStatus {
    Ok,
    Error,
    Interrupted,
    /// R died during the run (crash, out of memory, killed from outside).
    Crashed,
}

/// Which limit a run went over.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Limit {
    /// It ran longer than `max_run_time`: interrupted.
    Time,
    /// R used more than `max_memory_bytes`: stopped, because a process that
    /// is out of memory may not answer an interrupt. Restart to go on.
    Memory,
}

/// Something that happened in the session.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    Started {
        run: RunId,
    },
    Output {
        run: RunId,
        stream: Stream,
        text: String,
    },
    Warning {
        run: RunId,
        message: String,
    },
    Error {
        run: RunId,
        message: String,
        call: String,
    },
    /// The run printed more than the cap; later output of this run is dropped.
    Truncated {
        run: RunId,
    },
    /// The run went over a limit from the settings, and RCraft is stopping it.
    /// A `Finished` event follows.
    LimitReached {
        run: RunId,
        limit: Limit,
    },
    Finished {
        run: RunId,
        status: RunStatus,
    },
    /// The R process has ended; the session must be restarted to use R again.
    Exited {
        code: Option<i32>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionError {
    Spawn(String),
    /// R did not report ready in time; carries the end of its error output.
    StartTimeout(String),
    /// R ended before it was ready; carries the end of its error output.
    StartFailed(String),
    Busy,
    NotRunning,
    Proto(ProtoError),
    Io(String),
    TimedOut,
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SessionError::Spawn(e) => write!(f, "could not start R: {e}"),
            SessionError::StartTimeout(e) => write!(f, "R did not start in time. {e}"),
            SessionError::StartFailed(e) => write!(f, "R stopped while starting. {e}"),
            SessionError::Busy => write!(f, "R is still running the previous code"),
            SessionError::NotRunning => write!(f, "R is not running; restart the session"),
            SessionError::Proto(e) => write!(f, "{e}"),
            SessionError::Io(e) => write!(f, "could not talk to R: {e}"),
            SessionError::TimedOut => write!(f, "the run took longer than the time limit"),
        }
    }
}

impl From<ProtoError> for SessionError {
    fn from(e: ProtoError) -> Self {
        SessionError::Proto(e)
    }
}

/// What [`Session::interrupt`] did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InterruptOutcome {
    /// SIGINT was sent; the run ends as `Interrupted` and R stays up.
    Signalled,
    /// R was stopped (no way to signal it on this system). Restart to go on.
    SessionStopped,
    /// Nothing was running.
    Idle,
}

#[derive(Clone, Debug)]
pub struct SessionConfig {
    pub install: RInstall,
    pub working_dir: Option<PathBuf>,
    pub startup_timeout: Duration,
    /// Per run, across both streams.
    pub max_output_bytes: usize,
    /// A run longer than this is interrupted (`Limit::Time`). `None`: no limit.
    pub max_run_time: Option<Duration>,
    /// R using more than this many bytes is stopped (`Limit::Memory`).
    /// Checked every couple of seconds while a run is going. `None`: no limit.
    pub max_memory_bytes: Option<u64>,
    /// RCraft's own package library, put first on R's search path when the
    /// folder exists. `None`: leave R's libraries as they are.
    pub library: Option<PathBuf>,
}

impl SessionConfig {
    pub fn new(install: RInstall) -> SessionConfig {
        SessionConfig {
            install,
            working_dir: None,
            startup_timeout: Duration::from_secs(60),
            max_output_bytes: 16 * 1024 * 1024,
            max_run_time: None,
            max_memory_bytes: None,
            library: crate::library::user_library_dir(&crate::find::Env::from_process()),
        }
    }
}

/// How often the limits are looked at while a caller waits for events.
const POLL_INTERVAL: Duration = Duration::from_millis(200);
/// How long an interrupted run may carry on before R is stopped outright.
const ESCALATE_AFTER: Duration = Duration::from_secs(10);
/// How often R's memory is read (each read starts a system tool).
const MEMORY_CHECK_INTERVAL: Duration = Duration::from_secs(2);
/// How long standard error may stay open after standard output has closed;
/// R's last words (why it stopped) are on standard error.
const STDERR_GRACE: Duration = Duration::from_secs(2);
/// How long [`Session::stop`] lets an idle R quit on its own.
const QUIT_GRACE: Duration = Duration::from_millis(500);

/// What the reader threads send.
enum Raw {
    Frame(Frame),
    Text(Stream, String),
    Closed(Stream),
}

struct RunState {
    id: RunId,
    bytes: usize,
    truncated: bool,
    started: Instant,
    last_memory_check: Instant,
    /// When RCraft asked the run to stop (a limit, or an interrupt). A run
    /// still going [`ESCALATE_AFTER`] later is stopped outright.
    stopping_since: Option<Instant>,
    /// R has sent the run's `Begin`. Before that an interrupt would reach R
    /// while it still drops interrupts left over from the run before, so it
    /// waits (`interrupt_wanted`) and is sent when `Begin` arrives.
    begun: bool,
    interrupt_wanted: bool,
    /// The end frame from standard output, held until standard error has
    /// caught up (its `Sync`), so no message is lost to a race.
    end: Option<Status>,
    synced: bool,
}

pub struct Session {
    config: SessionConfig,
    child: Child,
    stdin: Option<ChildStdin>,
    raw: Receiver<Raw>,
    pending: VecDeque<Event>,
    next_id: RunId,
    current: Option<RunState>,
    stopped_by_us: bool,
    exited: bool,
    /// When standard output closed; the exit is reported once standard error
    /// has closed too, or [`STDERR_GRACE`] later.
    stdout_closed: Option<Instant>,
    stderr_closed: bool,
    stderr_tail: String,
    r_version: String,
    r_pid: u32,
    script_dir: PathBuf,
}

impl Session {
    /// Starts R and waits until it reports ready.
    pub fn start(config: SessionConfig) -> Result<Session, SessionError> {
        let token = new_token();
        let script_dir = make_script_dir()?;
        let script = script_dir.join("bootstrap.R");
        std::fs::write(&script, BOOTSTRAP_R).map_err(|e| SessionError::Io(e.to_string()))?;

        let mut command = Command::new(&config.install.rscript);
        command
            .arg("--vanilla")
            .arg(&script)
            .arg(&token)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(dir) = &config.working_dir {
            command.current_dir(dir);
        }
        if !config.install.path_prepend.is_empty() {
            let existing = std::env::var_os("PATH").unwrap_or_default();
            let parts = config
                .install
                .path_prepend
                .iter()
                .cloned()
                .chain(std::env::split_paths(&existing));
            if let Ok(joined) = std::env::join_paths(parts) {
                command.env("PATH", joined);
            }
        }
        if let Some(lib) = config.library.as_ref().filter(|l| l.is_dir()) {
            let existing = std::env::var_os("R_LIBS_USER");
            if let Some(value) = crate::library::libs_user_value(existing.as_deref(), lib) {
                command.env("R_LIBS_USER", value);
            }
        }
        no_console_window(&mut command);

        let mut child = command.spawn().map_err(|e| {
            let _ = std::fs::remove_dir_all(&script_dir);
            SessionError::Spawn(e.to_string())
        })?;
        let stdin = child.stdin.take();
        let (stdout, stderr) = match (child.stdout.take(), child.stderr.take()) {
            (Some(o), Some(e)) => (o, e),
            _ => {
                kill_tree(&mut child);
                let _ = std::fs::remove_dir_all(&script_dir);
                return Err(SessionError::Spawn("no output pipes".into()));
            }
        };

        let (tx, raw) = sync_channel(CHANNEL_CAPACITY);
        spawn_stdout_reader(stdout, token.clone(), tx.clone());
        spawn_stderr_reader(stderr, token, tx);

        let mut session = Session {
            config,
            child,
            stdin,
            raw,
            pending: VecDeque::new(),
            next_id: 1,
            current: None,
            stopped_by_us: false,
            exited: false,
            stdout_closed: None,
            stderr_closed: false,
            stderr_tail: String::new(),
            r_version: String::new(),
            r_pid: 0,
            script_dir,
        };
        session.wait_until_ready()?;
        Ok(session)
    }

    /// The version string R reported, such as `R version 4.4.3 (...)`.
    pub fn r_version(&self) -> &str {
        &self.r_version
    }

    pub fn is_running(&self) -> bool {
        !self.exited
    }

    pub fn is_busy(&self) -> bool {
        self.current.is_some()
    }

    /// Sends code to R and returns at once; read the result with
    /// [`Session::next_event`].
    pub fn run(&mut self, code: &str) -> Result<RunId, SessionError> {
        if self.exited {
            return Err(SessionError::NotRunning);
        }
        if self.current.is_some() {
            return Err(SessionError::Busy);
        }
        let id = self.next_id;
        let bytes = encode_run(id, code)?;
        let stdin = self.stdin.as_mut().ok_or(SessionError::NotRunning)?;
        stdin
            .write_all(&bytes)
            .and_then(|()| stdin.flush())
            .map_err(|e| SessionError::Io(e.to_string()))?;
        self.next_id += 1;
        let now = Instant::now();
        self.current = Some(RunState {
            id,
            bytes: 0,
            truncated: false,
            started: now,
            last_memory_check: now,
            stopping_since: None,
            begun: false,
            interrupt_wanted: false,
            end: None,
            synced: false,
        });
        Ok(id)
    }

    /// The next event, waiting up to `timeout`. `None` means nothing happened
    /// in that time (or the session has ended and everything was read).
    pub fn next_event(&mut self, timeout: Duration) -> Option<Event> {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(event) = self.pending.pop_front() {
                return Some(event);
            }
            self.check_limits();
            if !self.exited
                && self
                    .stdout_closed
                    .is_some_and(|at| at.elapsed() >= STDERR_GRACE)
            {
                self.finish_exit();
            }
            if !self.pending.is_empty() {
                continue;
            }
            let left = deadline.saturating_duration_since(Instant::now());
            // wake regularly so the limits are looked at even when R is silent
            match self.raw.recv_timeout(left.min(POLL_INTERVAL)) {
                Ok(raw) => self.handle(raw),
                Err(RecvTimeoutError::Timeout) => {
                    if Instant::now() >= deadline {
                        return None;
                    }
                }
                Err(RecvTimeoutError::Disconnected) => {
                    self.finish_exit();
                    return self.pending.pop_front();
                }
            }
        }
    }

    /// Looks at the time and memory limits for the run in progress.
    fn check_limits(&mut self) {
        // A run that was interrupted but is still going (R can be deep in
        // code that does not look for interrupts) is stopped after a while.
        if let Some(at) = self.current.as_ref().and_then(|s| s.stopping_since) {
            if at.elapsed() > ESCALATE_AFTER && !self.stopped_by_us && !self.exited {
                self.stop_now();
            }
            return;
        }
        let (run, over_time, check_memory) = match self.current.as_ref() {
            Some(state) if !self.exited => (
                state.id,
                self.config
                    .max_run_time
                    .is_some_and(|max| state.started.elapsed() > max),
                self.config.max_memory_bytes.is_some()
                    && state.last_memory_check.elapsed() >= MEMORY_CHECK_INTERVAL,
            ),
            _ => return,
        };
        if over_time {
            self.pending.push_back(Event::LimitReached {
                run,
                limit: Limit::Time,
            });
            self.interrupt();
            return;
        }
        if check_memory {
            if let Some(state) = self.current.as_mut() {
                state.last_memory_check = Instant::now();
            }
            let used = procinfo::memory_bytes(self.r_pid);
            if let (Some(used), Some(max)) = (used, self.config.max_memory_bytes) {
                if used > max {
                    self.mark_stopping();
                    self.pending.push_back(Event::LimitReached {
                        run,
                        limit: Limit::Memory,
                    });
                    self.stop_now();
                }
            }
        }
    }

    fn mark_stopping(&mut self) {
        if let Some(state) = self.current.as_mut() {
            state.stopping_since.get_or_insert_with(Instant::now);
        }
    }

    /// Kills R now and remembers that RCraft did it, so the run in progress
    /// is reported as interrupted rather than crashed.
    fn stop_now(&mut self) {
        self.stopped_by_us = true;
        kill_tree(&mut self.child);
    }

    /// Stops the current run. See [`InterruptOutcome`]. A run that is still
    /// going [`ESCALATE_AFTER`] later is stopped outright, with R.
    pub fn interrupt(&mut self) -> InterruptOutcome {
        if self.current.is_none() || self.exited {
            return InterruptOutcome::Idle;
        }
        self.mark_stopping();
        if CAN_SIGNAL {
            match self.current.as_mut() {
                Some(state) if !state.begun => {
                    state.interrupt_wanted = true;
                    return InterruptOutcome::Signalled;
                }
                _ => {
                    if send_interrupt(self.r_pid) {
                        return InterruptOutcome::Signalled;
                    }
                }
            }
        }
        self.stop_now();
        InterruptOutcome::SessionStopped
    }

    /// R's own process id.
    pub fn r_pid(&self) -> u32 {
        self.r_pid
    }

    /// Ends this R and starts a fresh one with the same settings. Objects in
    /// the old session are gone.
    pub fn restart(&mut self) -> Result<(), SessionError> {
        self.stop();
        let fresh = Session::start(self.config.clone())?;
        // The old value is dropped here; its process is already stopped.
        *self = fresh;
        Ok(())
    }

    /// Runs `code` and collects everything, waiting at most `limit`. On
    /// timeout the run is interrupted, its end is waited for (R is stopped if
    /// it does not answer), and `TimedOut` is returned; the session is then
    /// free for the next run, or no longer running.
    pub fn run_to_end(&mut self, code: &str, limit: Duration) -> Result<RunOutput, SessionError> {
        let id = self.run(code)?;
        let deadline = Instant::now() + limit;
        let mut out = RunOutput::default();
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                self.interrupt();
                self.wait_for_end(id);
                return Err(SessionError::TimedOut);
            }
            let Some(event) = self.next_event(left.min(Duration::from_millis(250))) else {
                if self.exited && self.current.is_none() {
                    out.status = RunStatus::Crashed;
                    return Ok(out);
                }
                continue;
            };
            match event {
                Event::Output { run, stream, text } if run == id => match stream {
                    Stream::Stdout => out.stdout.push_str(&text),
                    Stream::Stderr => out.stderr.push_str(&text),
                },
                Event::Warning { run, message } if run == id => out.warnings.push(message),
                Event::Error { run, message, call } if run == id => {
                    out.error = Some(RError { message, call });
                }
                Event::Truncated { run } if run == id => out.truncated = true,
                Event::LimitReached { run, limit } if run == id => out.limit = Some(limit),
                Event::Finished { run, status } if run == id => {
                    out.status = status;
                    return Ok(out);
                }
                _ => {}
            }
        }
    }

    /// Reads events until run `id` has ended, dropping them. Bounded: past
    /// [`ESCALATE_AFTER`] the limits check stops R, which ends the run.
    fn wait_for_end(&mut self, id: RunId) {
        let deadline = Instant::now() + ESCALATE_AFTER + STDERR_GRACE + Duration::from_secs(5);
        while self.current.as_ref().is_some_and(|s| s.id == id) && Instant::now() < deadline {
            let _ = self.next_event(POLL_INTERVAL);
        }
    }

    fn wait_until_ready(&mut self) -> Result<(), SessionError> {
        let deadline = Instant::now() + self.config.startup_timeout;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            match self.raw.recv_timeout(left) {
                Ok(Raw::Frame(Frame::Ready { r_version, pid })) => {
                    self.r_pid = pid;
                    self.r_version = r_version;
                    return Ok(());
                }
                Ok(Raw::Closed(Stream::Stdout)) | Err(RecvTimeoutError::Disconnected) => {
                    // let the stderr reader deliver what it has
                    while let Ok(raw) = self.raw.try_recv() {
                        self.note_stderr(&raw);
                    }
                    let tail = self.stderr_tail.clone();
                    self.stop();
                    return Err(SessionError::StartFailed(tail));
                }
                Err(RecvTimeoutError::Timeout) => {
                    let tail = self.stderr_tail.clone();
                    self.stop();
                    return Err(SessionError::StartTimeout(tail));
                }
                Ok(other) => self.note_stderr(&other),
            }
        }
    }

    fn note_stderr(&mut self, raw: &Raw) {
        if let Raw::Text(Stream::Stderr, text) = raw {
            self.stderr_tail.push_str(text);
            let len = self.stderr_tail.len();
            if len > STDERR_TAIL_BYTES {
                let mut cut = len - STDERR_TAIL_BYTES;
                while !self.stderr_tail.is_char_boundary(cut) {
                    cut += 1;
                }
                self.stderr_tail.drain(..cut);
            }
        }
    }

    fn handle(&mut self, raw: Raw) {
        self.note_stderr(&raw);
        match raw {
            Raw::Frame(frame) => self.handle_frame(frame),
            Raw::Text(stream, text) => self.handle_text(stream, text),
            // the process is gone, but what it wrote last on standard error
            // (often why it stopped) may still be on its way
            Raw::Closed(Stream::Stdout) => {
                self.stdout_closed.get_or_insert_with(Instant::now);
                if self.stderr_closed {
                    self.finish_exit();
                }
            }
            Raw::Closed(Stream::Stderr) => {
                self.stderr_closed = true;
                if self.stdout_closed.is_some() {
                    self.finish_exit();
                }
            }
        }
    }

    fn handle_frame(&mut self, frame: Frame) {
        match frame {
            Frame::Ready { .. } => {}
            Frame::Begin { run } => {
                let mut wanted = false;
                if let Some(state) = self.current.as_mut().filter(|s| s.id == run) {
                    state.begun = true;
                    wanted = std::mem::take(&mut state.interrupt_wanted);
                }
                if wanted && !self.exited && !send_interrupt(self.r_pid) {
                    self.stop_now();
                }
                self.pending.push_back(Event::Started { run });
            }
            Frame::Warning { run, message } => {
                self.pending.push_back(Event::Warning { run, message });
            }
            Frame::Error { run, message, call } => {
                self.pending.push_back(Event::Error { run, message, call });
            }
            Frame::End { run, status } => {
                if let Some(state) = self.current.as_mut().filter(|s| s.id == run) {
                    state.end = Some(status);
                }
                self.finish_if_ready(run);
            }
            Frame::Sync { run } => {
                if let Some(state) = self.current.as_mut().filter(|s| s.id == run) {
                    state.synced = true;
                }
                self.finish_if_ready(run);
            }
        }
    }

    /// A run is over once R has said so on standard output and everything R
    /// wrote on standard error before that has been read too. The two streams
    /// are read by separate threads, so either may arrive first.
    fn finish_if_ready(&mut self, run: RunId) {
        let status = match self.current.as_ref() {
            Some(state) if state.id == run && state.synced => state.end,
            _ => None,
        };
        let Some(status) = status else { return };
        self.current = None;
        let status = match status {
            Status::Ok => RunStatus::Ok,
            Status::Error => RunStatus::Error,
            Status::Interrupted => RunStatus::Interrupted,
        };
        self.pending.push_back(Event::Finished { run, status });
    }

    fn handle_text(&mut self, stream: Stream, text: String) {
        let Some(state) = self.current.as_mut() else {
            // output with no run in progress (R start-up noise, late output)
            return;
        };
        let run = state.id;
        if state.truncated {
            return;
        }
        state.bytes = state.bytes.saturating_add(text.len());
        if state.bytes > self.config.max_output_bytes {
            state.truncated = true;
            self.pending.push_back(Event::Truncated { run });
            return;
        }
        self.pending.push_back(Event::Output { run, stream, text });
    }

    /// R's output streams have closed: the process is gone.
    fn finish_exit(&mut self) {
        if self.exited {
            return;
        }
        self.exited = true;
        self.stdin = None;
        let code = wait_briefly(&mut self.child);
        if let Some(state) = self.current.take() {
            // R may have reported the end before it went away
            let status = match state.end {
                Some(Status::Ok) => RunStatus::Ok,
                Some(Status::Error) => RunStatus::Error,
                Some(Status::Interrupted) => RunStatus::Interrupted,
                None if self.stopped_by_us => RunStatus::Interrupted,
                None => RunStatus::Crashed,
            };
            self.pending.push_back(Event::Finished {
                run: state.id,
                status,
            });
        }
        self.pending.push_back(Event::Exited { code });
    }

    /// Asks R to quit (and, when it is idle, gives it a moment to), then
    /// makes sure it is gone and cleans up.
    fn stop(&mut self) {
        if let Some(mut stdin) = self.stdin.take() {
            let asked = stdin.write_all(QUIT).and_then(|()| stdin.flush()).is_ok();
            drop(stdin);
            if asked && self.current.is_none() {
                let deadline = Instant::now() + QUIT_GRACE;
                while matches!(self.child.try_wait(), Ok(None)) && Instant::now() < deadline {
                    thread::sleep(Duration::from_millis(10));
                }
            }
        }
        kill_tree(&mut self.child);
        self.exited = true;
        let _ = std::fs::remove_dir_all(&self.script_dir);
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.stop();
    }
}

/// What [`Session::run_to_end`] collects.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunOutput {
    pub status: RunStatus,
    pub stdout: String,
    pub stderr: String,
    pub warnings: Vec<String>,
    pub error: Option<RError>,
    pub truncated: bool,
    /// Set when the run went over a limit from the settings.
    pub limit: Option<Limit>,
}

impl Default for RunOutput {
    fn default() -> Self {
        RunOutput {
            status: RunStatus::Ok,
            stdout: String::new(),
            stderr: String::new(),
            warnings: Vec::new(),
            error: None,
            truncated: false,
            limit: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RError {
    pub message: String,
    pub call: String,
}

fn make_script_dir() -> Result<PathBuf, SessionError> {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("rcraft-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).map_err(|e| SessionError::Io(e.to_string()))?;
    Ok(dir)
}

fn spawn_stdout_reader(stdout: impl Read + Send + 'static, token: String, tx: SyncSender<Raw>) {
    thread::spawn(move || {
        let marker = frame_marker(&token);
        read_chunks(stdout, marker.as_bytes(), |chunk| {
            let item = match parse_frame(&token, &chunk) {
                Some(Ok(frame)) => Raw::Frame(frame),
                // a frame we cannot read is shown as text rather than lost
                Some(Err(_)) => Raw::Text(Stream::Stdout, chunk),
                None => {
                    // text with no line break (`cat("hi")`) comes first, then the frame
                    if let Some((text, Ok(frame))) = parse_frame_after_text(&token, &chunk) {
                        if tx
                            .send(Raw::Text(Stream::Stdout, text.to_string()))
                            .is_err()
                        {
                            return false;
                        }
                        Raw::Frame(frame)
                    } else {
                        Raw::Text(Stream::Stdout, chunk)
                    }
                }
            };
            tx.send(item).is_ok()
        });
        let _ = tx.send(Raw::Closed(Stream::Stdout));
    });
}

fn spawn_stderr_reader(stderr: impl Read + Send + 'static, token: String, tx: SyncSender<Raw>) {
    thread::spawn(move || {
        let marker = frame_marker(&token);
        read_chunks(stderr, marker.as_bytes(), |chunk| {
            // the only frame on standard error is `Sync`; anything else is text
            let item = match parse_frame(&token, &chunk) {
                Some(Ok(frame @ Frame::Sync { .. })) => Raw::Frame(frame),
                _ => {
                    if let Some((text, Ok(frame @ Frame::Sync { .. }))) =
                        parse_frame_after_text(&token, &chunk)
                    {
                        if tx
                            .send(Raw::Text(Stream::Stderr, text.to_string()))
                            .is_err()
                        {
                            return false;
                        }
                        Raw::Frame(frame)
                    } else {
                        Raw::Text(Stream::Stderr, chunk)
                    }
                }
            };
            tx.send(item).is_ok()
        });
        let _ = tx.send(Raw::Closed(Stream::Stderr));
    });
}

/// Reads lines, or pieces of at most [`MAX_CHUNK_BYTES`] when a line is
/// longer, and passes each to `f`. Stops when `f` returns false or the
/// stream ends.
///
/// A piece cut short of a line break never ends inside a frame (`marker`
/// starts one) or inside a character: that tail is carried into the next
/// piece, so a frame glued to long unterminated output is still found.
fn read_chunks(source: impl Read, marker: &[u8], mut f: impl FnMut(String) -> bool) {
    let mut reader = BufReader::new(source);
    let mut buf: Vec<u8> = Vec::new();
    loop {
        let room = MAX_CHUNK_BYTES.saturating_sub(buf.len() as u64).max(1);
        let read = reader.by_ref().take(room).read_until(b'\n', &mut buf);
        if matches!(read, Ok(0) | Err(_)) {
            if !buf.is_empty() {
                let _ = f(String::from_utf8_lossy(&buf).into_owned());
            }
            return;
        }
        let carry = if buf.ends_with(b"\n") {
            Vec::new()
        } else {
            let cut = safe_cut(&buf, marker);
            buf.split_off(cut)
        };
        // Windows R writes CRLF; callers see the same text everywhere.
        if buf.ends_with(b"\r\n") {
            buf.truncate(buf.len() - 2);
            buf.push(b'\n');
        }
        if !f(String::from_utf8_lossy(&buf).into_owned()) {
            return;
        }
        buf = carry;
    }
}

/// Where to end a piece that has no line break: before a frame marker that
/// starts after the beginning, before a tail that could be the start of one,
/// and never inside a character or between `\r` and `\n`. Never 0, so a
/// piece always makes progress.
fn safe_cut(buf: &[u8], marker: &[u8]) -> usize {
    let mut cut = buf.len();
    let found = (!marker.is_empty())
        .then(|| buf.windows(marker.len()).position(|w| w == marker))
        .flatten();
    match found {
        Some(at) if at > 0 => cut = at,
        Some(_) => {}
        None => {
            for k in (1..marker.len().min(buf.len() + 1)).rev() {
                if marker.get(..k).is_some_and(|start| buf.ends_with(start)) {
                    cut = buf.len() - k;
                    break;
                }
            }
        }
    }
    // the last character must be whole: step back over continuation bytes to
    // its first byte and see whether all of it is in
    for i in (cut.saturating_sub(4)..cut).rev() {
        let Some(&b) = buf.get(i) else { break };
        if b & 0xC0 == 0x80 {
            continue;
        }
        let len = match b {
            0xF0.. => 4,
            0xE0.. => 3,
            0xC0.. => 2,
            _ => 1,
        };
        if i + len > cut {
            cut = i;
        }
        break;
    }
    if cut > 1 && buf.get(cut - 1) == Some(&b'\r') {
        cut -= 1;
    }
    if cut == 0 {
        buf.len()
    } else {
        cut
    }
}

/// Whether this system can interrupt R without stopping it.
const CAN_SIGNAL: bool = cfg!(unix);

#[cfg(unix)]
fn send_interrupt(pid: u32) -> bool {
    // `kill -INT 0` would signal a whole process group
    if pid == 0 {
        return false;
    }
    Command::new("kill")
        .args(["-INT", &pid.to_string()])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn send_interrupt(_pid: u32) -> bool {
    false
}

/// Stops the process and everything it started. On Windows `Rscript.exe`
/// starts R as a child, so the whole tree is stopped.
fn kill_tree(child: &mut Child) {
    // Only act on a process we know is still ours: a finished process's id
    // may already belong to something else.
    #[cfg(windows)]
    if matches!(child.try_wait(), Ok(None)) {
        let mut taskkill = Command::new("taskkill");
        taskkill
            .args(["/PID", &child.id().to_string(), "/T", "/F"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        no_console_window(&mut taskkill);
        let _ = taskkill.status();
    }
    let _ = child.kill();
    let _ = child.wait();
}

/// The exit code, waiting a moment for the process to finish.
fn wait_briefly(child: &mut Child) -> Option<i32> {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status.code(),
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(10)),
            _ => return None,
        }
    }
}

#[cfg(windows)]
fn no_console_window(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn no_console_window(_command: &mut Command) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_lines_are_split_not_buffered_whole() {
        let data = vec![b'a'; (MAX_CHUNK_BYTES as usize) * 2 + 10];
        let mut chunks = Vec::new();
        read_chunks(&data[..], b"", |c| {
            chunks.push(c.len());
            true
        });
        assert_eq!(chunks.iter().sum::<usize>(), data.len());
        assert!(chunks.iter().all(|&n| n as u64 <= MAX_CHUNK_BYTES));
    }

    #[test]
    fn lines_come_out_whole() {
        let mut chunks = Vec::new();
        read_chunks(&b"one\ntwo\nthree"[..], b"", |c| {
            chunks.push(c);
            true
        });
        assert_eq!(chunks, ["one\n", "two\n", "three"]);
    }

    #[test]
    fn crlf_becomes_lf() {
        let mut chunks = Vec::new();
        read_chunks(&b"a\r\nb\r\nc"[..], b"", |c| {
            chunks.push(c);
            true
        });
        assert_eq!(chunks, ["a\n", "b\n", "c"]);
    }

    #[test]
    fn invalid_utf8_does_not_panic() {
        let mut chunks = Vec::new();
        read_chunks(&[0xff, 0xfe, b'\n'][..], b"", |c| {
            chunks.push(c);
            true
        });
        assert_eq!(chunks.len(), 1);
    }

    const MARKER: &[u8] = b"rcraft0123456789abcdef\t";

    fn chunks_of(data: &[u8]) -> Vec<String> {
        let mut chunks = Vec::new();
        read_chunks(data, MARKER, |c| {
            chunks.push(c);
            true
        });
        chunks
    }

    #[test]
    fn a_frame_across_the_piece_boundary_stays_whole() {
        // the review found END lost when its marker straddled 64 KiB
        let cap = MAX_CHUNK_BYTES as usize;
        for before in [
            cap - 1,
            cap - 10,
            cap - MARKER.len(),
            cap - MARKER.len() + 1,
            cap,
        ] {
            let mut data = vec![b'a'; before];
            data.extend_from_slice(MARKER);
            data.extend_from_slice(b"END\t1\tok\n");
            let chunks = chunks_of(&data);
            let last = chunks.last().map(String::as_str).unwrap_or_default();
            let marker = std::str::from_utf8(MARKER).unwrap_or_default();
            assert!(
                last.starts_with(marker) || last.contains(&format!("a{marker}")),
                "{before}: the frame was split: {:?}",
                chunks.iter().map(|c| c.len()).collect::<Vec<_>>()
            );
            assert!(last.ends_with("END\t1\tok\n"), "{before}");
            assert_eq!(chunks.concat().len(), data.len());
        }
    }

    #[test]
    fn a_character_across_the_piece_boundary_stays_whole() {
        let cap = MAX_CHUNK_BYTES as usize;
        for before in [cap - 1, cap - 2, cap - 3] {
            let mut data = vec![b'a'; before];
            data.extend_from_slice("é中😀\n".as_bytes());
            let text = chunks_of(&data).concat();
            assert!(!text.contains('\u{fffd}'), "{before}");
            assert!(text.ends_with("é中😀\n"));
        }
    }

    #[test]
    fn crlf_across_the_piece_boundary_becomes_lf() {
        let mut data = vec![b'a'; MAX_CHUNK_BYTES as usize - 1];
        data.extend_from_slice(b"\r\nnext\n");
        let text = chunks_of(&data).concat();
        assert!(!text.contains('\r'));
        assert!(text.ends_with("a\nnext\n"));
    }

    #[test]
    fn safe_cut_always_makes_progress() {
        assert_eq!(safe_cut(MARKER, MARKER), MARKER.len());
        assert_eq!(safe_cut(b"\r", MARKER), 1);
        assert_eq!(safe_cut(&[0xE4], MARKER), 1);
        assert_eq!(safe_cut(b"abc", b""), 3);
    }
}
