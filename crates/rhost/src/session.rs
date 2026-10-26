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
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use rcraft_rproto::{
    encode_run, new_token, parse_frame, Frame, ProtoError, RunId, Status, BOOTSTRAP_R, QUIT,
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
        }
    }
}

/// How often the limits are looked at while a caller waits for events.
const POLL_INTERVAL: Duration = Duration::from_millis(200);
/// How long an interrupted run may carry on before R is stopped outright.
const ESCALATE_AFTER: Duration = Duration::from_secs(10);
/// How often R's memory is read (each read starts a system tool).
const MEMORY_CHECK_INTERVAL: Duration = Duration::from_secs(2);

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
    limit_hit: Option<Instant>,
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
    /// The run that standard-error text belongs to (0 when idle).
    current_id: Arc<AtomicU64>,
    stopped_by_us: bool,
    exited: bool,
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
        let current_id = Arc::new(AtomicU64::new(0));
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
            current_id,
            stopped_by_us: false,
            exited: false,
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
            limit_hit: None,
            end: None,
            synced: false,
        });
        self.current_id.store(id, Ordering::SeqCst);
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
        if let Some(at) = self.current.as_ref().and_then(|s| s.limit_hit) {
            if at.elapsed() > ESCALATE_AFTER && !self.stopped_by_us && !self.exited {
                self.stop_now();
            }
            return;
        }
        let (run, over_time, check_memory) = match self.current.as_ref() {
            Some(state) if state.limit_hit.is_none() && !self.exited => (
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
            self.mark_limit_hit();
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
                    self.mark_limit_hit();
                    self.pending.push_back(Event::LimitReached {
                        run,
                        limit: Limit::Memory,
                    });
                    self.stop_now();
                }
            }
        }
    }

    fn mark_limit_hit(&mut self) {
        if let Some(state) = self.current.as_mut() {
            state.limit_hit = Some(Instant::now());
        }
    }

    /// Kills R now and remembers that RCraft did it, so the run in progress
    /// is reported as interrupted rather than crashed.
    fn stop_now(&mut self) {
        self.stopped_by_us = true;
        kill_tree(&mut self.child);
    }

    /// Stops the current run. See [`InterruptOutcome`].
    pub fn interrupt(&mut self) -> InterruptOutcome {
        if self.current.is_none() || self.exited {
            return InterruptOutcome::Idle;
        }
        if send_interrupt(self.r_pid) {
            return InterruptOutcome::Signalled;
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
    /// timeout the run is interrupted and `TimedOut` is returned.
    pub fn run_to_end(&mut self, code: &str, limit: Duration) -> Result<RunOutput, SessionError> {
        let id = self.run(code)?;
        let deadline = Instant::now() + limit;
        let mut out = RunOutput::default();
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                self.interrupt();
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
            Raw::Closed(Stream::Stdout) => self.finish_exit(),
            Raw::Closed(Stream::Stderr) => {}
        }
    }

    fn handle_frame(&mut self, frame: Frame) {
        match frame {
            Frame::Ready { .. } => {}
            Frame::Begin { run } => self.pending.push_back(Event::Started { run }),
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
        self.current_id.store(0, Ordering::SeqCst);
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

    /// R's standard output closed: the process is gone.
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
        self.current_id.store(0, Ordering::SeqCst);
        self.pending.push_back(Event::Exited { code });
    }

    /// Asks R to quit, then makes sure it is gone and cleans up.
    fn stop(&mut self) {
        if let Some(mut stdin) = self.stdin.take() {
            let _ = stdin.write_all(QUIT);
            let _ = stdin.flush();
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
        read_chunks(stdout, |chunk| {
            let item = match parse_frame(&token, &chunk) {
                Some(Ok(frame)) => Raw::Frame(frame),
                // a frame we cannot read is shown as text rather than lost
                Some(Err(_)) | None => Raw::Text(Stream::Stdout, chunk),
            };
            tx.send(item).is_ok()
        });
        let _ = tx.send(Raw::Closed(Stream::Stdout));
    });
}

fn spawn_stderr_reader(stderr: impl Read + Send + 'static, token: String, tx: SyncSender<Raw>) {
    thread::spawn(move || {
        read_chunks(stderr, |chunk| {
            // the only frame on standard error is `Sync`; anything else is text
            let item = match parse_frame(&token, &chunk) {
                Some(Ok(frame @ Frame::Sync { .. })) => Raw::Frame(frame),
                _ => Raw::Text(Stream::Stderr, chunk),
            };
            tx.send(item).is_ok()
        });
        let _ = tx.send(Raw::Closed(Stream::Stderr));
    });
}

/// Reads lines, or pieces of at most [`MAX_CHUNK_BYTES`] when a line is
/// longer, and passes each to `f`. Stops when `f` returns false or the
/// stream ends.
fn read_chunks(source: impl Read, mut f: impl FnMut(String) -> bool) {
    let mut reader = BufReader::new(source);
    let mut buf = Vec::new();
    loop {
        buf.clear();
        match reader
            .by_ref()
            .take(MAX_CHUNK_BYTES)
            .read_until(b'\n', &mut buf)
        {
            Ok(0) | Err(_) => return,
            Ok(_) => {
                // Windows R writes CRLF; callers see the same text everywhere.
                if buf.ends_with(b"\r\n") {
                    buf.truncate(buf.len() - 2);
                    buf.push(b'\n');
                }
                if !f(String::from_utf8_lossy(&buf).into_owned()) {
                    return;
                }
            }
        }
    }
}

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
        read_chunks(&data[..], |c| {
            chunks.push(c.len());
            true
        });
        assert_eq!(chunks.iter().sum::<usize>(), data.len());
        assert!(chunks.iter().all(|&n| n as u64 <= MAX_CHUNK_BYTES));
    }

    #[test]
    fn lines_come_out_whole() {
        let mut chunks = Vec::new();
        read_chunks(&b"one\ntwo\nthree"[..], |c| {
            chunks.push(c);
            true
        });
        assert_eq!(chunks, ["one\n", "two\n", "three"]);
    }

    #[test]
    fn crlf_becomes_lf() {
        let mut chunks = Vec::new();
        read_chunks(&b"a\r\nb\r\nc"[..], |c| {
            chunks.push(c);
            true
        });
        assert_eq!(chunks, ["a\n", "b\n", "c"]);
    }

    #[test]
    fn invalid_utf8_does_not_panic() {
        let mut chunks = Vec::new();
        read_chunks(&[0xff, 0xfe, b'\n'][..], |c| {
            chunks.push(c);
            true
        });
        assert_eq!(chunks.len(), 1);
    }
}
