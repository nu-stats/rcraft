//! The engine: one R session plus the history, results and transcript that
//! the interface reads. The interface stays thin (AGENTS.md section 3): it
//! calls [`Engine::submit`] and [`Engine::poll`] and draws what it finds.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use rcraft_rdata::Table;
use rcraft_rhost::{
    Event, InterruptOutcome, Limit, RunStatus, Session, SessionConfig, SessionError, Stream,
};
use rcraft_rproto::RunId;

use crate::history::{CommandId, History, Outcome, Source};
use crate::results::{Item, Results};
use crate::transcript::Transcript;
use crate::workspace::{parse_listing, WorkspaceItem, AUTO_REFRESH_CELLS, LISTING_R};

/// The most events handled in one [`Engine::poll`], so a flood of output
/// cannot freeze an interface that polls once per frame.
const MAX_EVENTS_PER_POLL: usize = 500;

/// How long a new command waits for a hidden query to end.
const INTERNAL_WAIT: Duration = Duration::from_secs(30);

/// Most text kept from a hidden query's output.
const MAX_INTERNAL_BYTES: usize = 4 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EngineError {
    /// Nothing but whitespace was submitted.
    Empty,
    Session(SessionError),
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EngineError::Empty => write!(f, "there is nothing to run"),
            EngineError::Session(e) => write!(f, "{e}"),
        }
    }
}

impl From<SessionError> for EngineError {
    fn from(e: SessionError) -> Self {
        EngineError::Session(e)
    }
}

/// What a call to [`Engine::poll`] found.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Polled {
    /// Results, history or the log changed: redraw.
    pub changed: bool,
    /// The command that was running has ended.
    pub finished: Option<CommandId>,
    /// The workspace listing or a watched table changed.
    pub data_changed: bool,
}

pub struct Engine {
    session: Session,
    history: History,
    results: Results,
    transcript: Transcript,
    /// The command in flight, with the run id R knows it by.
    current: Option<(RunId, CommandId)>,
    /// The newest command, for notes that arrive after it has finished.
    last_command: Option<CommandId>,
    /// A hidden query in flight (the workspace listing, a table fetch).
    internal: Option<InternalRun>,
    workspace: Vec<WorkspaceItem>,
    /// The workspace may have changed since it was last listed.
    stale: bool,
    /// Data frames the interface wants kept up to date.
    watched: BTreeSet<String>,
    tables: BTreeMap<String, Table>,
    table_errors: BTreeMap<String, String>,
    fetch_queue: VecDeque<String>,
    /// Questions the interface asked R, waiting their turn.
    query_queue: VecDeque<(QueryId, String)>,
    query_results: BTreeMap<QueryId, QueryResult>,
    next_query: QueryId,
}

/// Names a question asked with [`Engine::query`].
pub type QueryId = u64;

/// R's answer to a question: what it printed, or the error message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryResult {
    pub ok: bool,
    pub text: String,
}

/// Most answers kept.
const MAX_QUERY_RESULTS: usize = 32;

/// A query the engine runs for itself. Its output never reaches the history,
/// the log or the Results window.
struct InternalRun {
    run: RunId,
    kind: InternalKind,
    out: String,
    failed: bool,
}

enum InternalKind {
    Query(QueryId),
    Listing,
    Fetch { name: String, path: PathBuf },
}

impl Engine {
    pub fn start(config: SessionConfig) -> Result<Engine, SessionError> {
        Ok(Engine {
            session: Session::start(config)?,
            history: History::new(),
            results: Results::new(),
            transcript: Transcript::new(),
            current: None,
            last_command: None,
            internal: None,
            workspace: Vec::new(),
            stale: true,
            watched: BTreeSet::new(),
            tables: BTreeMap::new(),
            table_errors: BTreeMap::new(),
            fetch_queue: VecDeque::new(),
            query_queue: VecDeque::new(),
            query_results: BTreeMap::new(),
            next_query: 1,
        })
    }

    pub fn history(&self) -> &History {
        &self.history
    }

    pub fn results(&self) -> &Results {
        &self.results
    }

    pub fn r_version(&self) -> &str {
        self.session.r_version()
    }

    pub fn is_busy(&self) -> bool {
        self.current.is_some()
    }

    pub fn is_running(&self) -> bool {
        self.session.is_running()
    }

    /// The session log as text.
    pub fn log_text(&self) -> &str {
        self.transcript.log_text()
    }

    /// The code that ran without error, as an R script.
    pub fn script_text(&self) -> String {
        self.transcript.script_text()
    }

    /// Also write the log to a file from now on.
    pub fn log_to(&mut self, path: &Path) -> std::io::Result<()> {
        self.transcript.log_to(path)
    }

    pub fn stop_logging(&mut self) {
        self.transcript.stop_logging();
    }

    /// Runs `code` as a new command. Returns at once; call [`Engine::poll`]
    /// to collect the result.
    pub fn submit(&mut self, source: Source, code: &str) -> Result<CommandId, EngineError> {
        if code.trim().is_empty() {
            return Err(EngineError::Empty);
        }
        // a hidden query is short; let it end rather than refuse the command
        let limit = Instant::now() + INTERNAL_WAIT;
        while self.internal.is_some() && Instant::now() < limit {
            self.drain(Duration::from_millis(50));
        }
        // ask R first, so a refused command (busy, not running) leaves no trace
        let run = self.session.run(code)?;
        let id = self.history.push(source, code);
        self.results.begin(id);
        self.transcript.command(code);
        self.current = Some((run, id));
        self.last_command = Some(id);
        Ok(id)
    }

    /// Records a command that RCraft answered itself, without R (the `rc`
    /// commands): `shown` is the line the user typed, `text` what to show under
    /// it. It appears in History, Results and the log like any command; in the
    /// script it is a comment, so a saved script stays valid R.
    pub fn echo(&mut self, shown: &str, text: &str, outcome: Outcome) -> CommandId {
        let id = self.history.push(Source::Typed, shown);
        self.results.begin(id);
        self.transcript.command(shown);
        if !text.is_empty() {
            let item = Item::Text {
                stream: Stream::Stdout,
                text: if text.ends_with('\n') {
                    text.to_string()
                } else {
                    format!("{text}\n")
                },
            };
            self.transcript.item(&item);
            self.results.add(id, item);
        }
        self.history.set_outcome(id, outcome);
        self.results.finish(id, outcome);
        let comment = format!("# {}", shown.trim().replace('\n', "\n# "));
        self.transcript.finished(&Source::Typed, &comment, outcome);
        id
    }

    /// Takes in what R has produced, waiting up to `wait` for the first
    /// event.
    pub fn poll(&mut self, wait: Duration) -> Polled {
        let mut polled = self.drain(wait);
        let before = (
            self.workspace.len(),
            self.tables.len(),
            self.table_errors.len(),
        );
        self.start_internal();
        if before
            != (
                self.workspace.len(),
                self.tables.len(),
                self.table_errors.len(),
            )
        {
            polled.data_changed = true;
        }
        polled
    }

    /// Handles the events that are ready, waiting up to `wait` for the first.
    fn drain(&mut self, wait: Duration) -> Polled {
        let mut polled = Polled::default();
        let mut wait = wait;
        for _ in 0..MAX_EVENTS_PER_POLL {
            let Some(event) = self.session.next_event(wait) else {
                break;
            };
            wait = Duration::ZERO;
            let (workspace, tables, errors) = (
                self.workspace.len(),
                self.tables.len(),
                self.table_errors.len(),
            );
            if let Some(id) = self.apply(event) {
                polled.finished = Some(id);
            }
            if self.internal.is_none() || self.current.is_some() {
                polled.changed = true;
            }
            if (workspace, tables, errors)
                != (
                    self.workspace.len(),
                    self.tables.len(),
                    self.table_errors.len(),
                )
            {
                polled.data_changed = true;
                polled.changed = true;
            }
        }
        polled
    }

    /// The objects in R's workspace, as of the last listing.
    pub fn workspace(&self) -> &[WorkspaceItem] {
        &self.workspace
    }

    /// Asks for the workspace to be listed again, and watched tables brought
    /// over again, as soon as R is free.
    pub fn refresh(&mut self) {
        self.stale = true;
        let names: Vec<String> = self.watched.iter().cloned().collect();
        for name in names {
            self.queue_fetch(&name);
        }
    }

    /// Keeps the data frame `name` brought over and up to date. It arrives
    /// through later calls to [`Engine::poll`]; see [`Engine::table`].
    pub fn watch(&mut self, name: &str) {
        if self.watched.insert(name.to_string()) {
            self.table_errors.remove(name);
            self.queue_fetch(name);
        }
    }

    /// Stops keeping `name`, and frees it.
    pub fn unwatch(&mut self, name: &str) {
        self.watched.remove(name);
        self.tables.remove(name);
        self.table_errors.remove(name);
        self.fetch_queue.retain(|n| n != name);
    }

    /// Asks R to run `code` out of sight and keep what it prints. The answer
    /// arrives through later calls to [`Engine::poll`]; see
    /// [`Engine::query_result`]. Nothing reaches the history, log or Results.
    pub fn query(&mut self, code: &str) -> QueryId {
        let id = self.next_query;
        self.next_query += 1;
        self.query_queue.push_back((id, code.to_string()));
        id
    }

    /// The answer to a question, once R has given it.
    pub fn query_result(&self, id: QueryId) -> Option<&QueryResult> {
        self.query_results.get(&id)
    }

    /// The data frame `name` as last brought over.
    pub fn table(&self, name: &str) -> Option<&Table> {
        self.tables.get(name)
    }

    /// Why `name` could not be brought over, if it could not.
    pub fn table_error(&self, name: &str) -> Option<&str> {
        self.table_errors.get(name).map(String::as_str)
    }

    /// True while R runs a command or a hidden query, or one is waiting.
    pub fn is_working(&self) -> bool {
        self.current.is_some()
            || self.internal.is_some()
            || self.stale
            || !self.fetch_queue.is_empty()
            || !self.query_queue.is_empty()
    }

    fn queue_fetch(&mut self, name: &str) {
        if !self.fetch_queue.iter().any(|n| n == name) {
            self.fetch_queue.push_back(name.to_string());
        }
    }

    /// Starts the next hidden query if R is free.
    fn start_internal(&mut self) {
        if self.current.is_some() || self.internal.is_some() || !self.session.is_running() {
            return;
        }
        let (kind, code) = if let Some((id, code)) = self.query_queue.pop_front() {
            (InternalKind::Query(id), code)
        } else if let Some(name) = self.fetch_queue.pop_front() {
            let path = std::env::temp_dir().join(format!(
                "rcraft-{}-table-{}.rcb",
                std::process::id(),
                next_table_number()
            ));
            let variable = format!(
                "get({}, envir = globalenv())",
                rcraft_rdata::r_string(&name)
            );
            let code =
                rcraft_rdata::write_code(&variable, &path.to_string_lossy().replace('\\', "/"));
            (InternalKind::Fetch { name, path }, code)
        } else if self.stale {
            self.stale = false;
            (InternalKind::Listing, LISTING_R.to_string())
        } else {
            return;
        };
        match self.session.run(&code) {
            Ok(run) => {
                self.internal = Some(InternalRun {
                    run,
                    kind,
                    out: String::new(),
                    failed: false,
                });
            }
            Err(_) => {
                // R is busy or gone; try again on a later poll
                match kind {
                    InternalKind::Listing => self.stale = true,
                    InternalKind::Query(id) => self.query_queue.push_front((id, code)),
                    InternalKind::Fetch { name, .. } => self.fetch_queue.push_front(name),
                }
            }
        }
    }

    /// A hidden query ended: use what it found.
    fn finish_internal(&mut self, done: InternalRun) {
        match done.kind {
            InternalKind::Query(id) => {
                self.query_results.insert(
                    id,
                    QueryResult {
                        ok: !done.failed,
                        text: done.out,
                    },
                );
                while self.query_results.len() > MAX_QUERY_RESULTS {
                    self.query_results.pop_first();
                }
            }
            InternalKind::Listing => {
                if !done.failed {
                    self.workspace = parse_listing(&done.out);
                    // a watched object that is gone or no longer a data frame
                    let gone: Vec<String> = self
                        .watched
                        .iter()
                        .filter(|n| {
                            !self
                                .workspace
                                .iter()
                                .any(|w| &w.name == *n && w.is_data_frame())
                        })
                        .cloned()
                        .collect();
                    for name in gone {
                        self.tables.remove(&name);
                        self.table_errors
                            .insert(name, "this is no longer a data frame".to_string());
                    }
                }
            }
            InternalKind::Fetch { name, path } => {
                if !self.watched.contains(&name) {
                    let _ = std::fs::remove_file(&path);
                } else if done.failed {
                    let why = done.out.trim();
                    let why = if why.is_empty() {
                        "R could not send this object".to_string()
                    } else {
                        why.to_string()
                    };
                    self.tables.remove(&name);
                    self.table_errors.insert(name, why);
                    let _ = std::fs::remove_file(&path);
                } else {
                    let read = rcraft_rdata::read_file(&path);
                    let _ = std::fs::remove_file(&path);
                    match read {
                        Ok(table) => {
                            self.table_errors.remove(&name);
                            self.tables.insert(name, table);
                        }
                        Err(e) => {
                            self.tables.remove(&name);
                            self.table_errors.insert(name, e.to_string());
                        }
                    }
                }
            }
        }
    }

    /// Stops the command in flight. See [`InterruptOutcome`]; on Windows the
    /// session is stopped and [`Engine::restart`] is needed.
    pub fn interrupt(&mut self) -> InterruptOutcome {
        self.session.interrupt()
    }

    /// A fresh R session. History, results and the log are kept; the objects
    /// in R are gone.
    pub fn restart(&mut self) -> Result<(), SessionError> {
        self.session.restart()?;
        self.current = None;
        self.drop_internal();
        self.workspace.clear();
        self.tables.clear();
        self.table_errors.clear();
        self.stale = true;
        let names: Vec<String> = self.watched.iter().cloned().collect();
        self.fetch_queue.clear();
        self.query_queue.clear();
        self.query_results.clear();
        for name in names {
            self.queue_fetch(&name);
        }
        self.transcript.item(&Item::Notice(
            "R was restarted; objects from before are gone".into(),
        ));
        Ok(())
    }

    /// Clears the Results window (the history and log are kept).
    pub fn clear_results(&mut self) {
        self.results.clear();
    }

    /// Handles one event. Returns the command id if it was the end of one.
    fn apply(&mut self, event: Event) -> Option<CommandId> {
        if let Event::Exited { code } = &event {
            let note = match code {
                Some(c) => format!("R has stopped (exit code {c}); restart the session"),
                None => "R has stopped; restart the session".to_string(),
            };
            if let Some(command) = self.current.map(|c| c.1).or(self.last_command) {
                self.add(command, Item::Notice(note));
            }
            return None;
        }
        if self.take_internal_event(&event) {
            return None;
        }
        let (run, command) = self.current?;
        let item = match event {
            Event::Started { .. } => return None,
            Event::Output {
                run: r,
                stream,
                text,
            } if r == run => Item::Text { stream, text },
            Event::Warning { run: r, message } if r == run => Item::Warning(message),
            Event::Error {
                run: r,
                message,
                call,
            } if r == run => Item::Error { message, call },
            Event::Plot { run: r, path } if r == run => Item::Plot { path },
            Event::Truncated { run: r } if r == run => {
                Item::Notice("output cut off: more than the limit was printed".into())
            }
            Event::LimitReached { run: r, limit } if r == run => Item::Notice(match limit {
                Limit::Time => "stopped: the command ran longer than the time limit".into(),
                Limit::Memory => {
                    "stopped: R used more memory than the limit; restart the session".into()
                }
            }),
            Event::Finished { run: r, status } if r == run => {
                let outcome = match status {
                    RunStatus::Ok => Outcome::Ok,
                    RunStatus::Error => Outcome::Error,
                    RunStatus::Interrupted => Outcome::Interrupted,
                    RunStatus::Crashed => Outcome::Crashed,
                };
                self.finish(command, outcome);
                return Some(command);
            }
            _ => return None,
        };
        self.add(command, item);
        None
    }

    /// Gives an event to the hidden query it belongs to, if any.
    fn take_internal_event(&mut self, event: &Event) -> bool {
        let Some(internal) = self.internal.as_mut() else {
            return false;
        };
        let run = match event {
            Event::Started { run }
            | Event::Output { run, .. }
            | Event::Warning { run, .. }
            | Event::Error { run, .. }
            | Event::Truncated { run }
            | Event::LimitReached { run, .. }
            | Event::Plot { run, .. }
            | Event::Finished { run, .. } => *run,
            Event::Exited { .. } => return false,
        };
        if run != internal.run {
            return false;
        }
        match event {
            Event::Output {
                stream: Stream::Stdout,
                text,
                ..
            } if internal.out.len() < MAX_INTERNAL_BYTES => internal.out.push_str(text),
            Event::Error { message, .. } => {
                internal.failed = true;
                // an R error is the reason, shown if the table cannot come over
                internal.out = message.clone();
            }
            Event::Truncated { .. } | Event::LimitReached { .. } => internal.failed = true,
            Event::Finished { status, .. } => {
                if *status != RunStatus::Ok {
                    internal.failed = true;
                }
                if let Some(done) = self.internal.take() {
                    self.finish_internal(done);
                }
            }
            _ => {}
        }
        true
    }

    /// Forgets a hidden query that can no longer finish (R was stopped).
    fn drop_internal(&mut self) {
        if let Some(InternalRun {
            kind: InternalKind::Fetch { path, .. },
            ..
        }) = self.internal.take()
        {
            let _ = std::fs::remove_file(path);
        }
    }

    fn add(&mut self, command: CommandId, item: Item) {
        self.transcript.item(&item);
        self.results.add(command, item);
    }

    fn finish(&mut self, command: CommandId, outcome: Outcome) {
        self.history.set_outcome(command, outcome);
        self.results.finish(command, outcome);
        if let Some(entry) = self.history.get(command) {
            let (source, code) = (entry.source.clone(), entry.code.clone());
            self.transcript.finished(&source, &code, outcome);
        }
        self.current = None;
        // the command may have changed the workspace and the tables
        self.stale = true;
        let names: Vec<String> = self
            .watched
            .iter()
            .filter(|n| {
                self.workspace
                    .iter()
                    .find(|w| &w.name == *n)
                    .is_none_or(|w| w.cells() <= AUTO_REFRESH_CELLS)
            })
            .cloned()
            .collect();
        for name in names {
            self.queue_fetch(&name);
        }
    }
}

/// A number no other table file in this process has, so engines that share a
/// process (tests, several windows) never share a file.
fn next_table_number() -> u64 {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    COUNTER.fetch_add(1, Ordering::SeqCst)
}
