//! The engine: one R session plus the history, results and transcript that
//! the interface reads. The interface stays thin (AGENTS.md section 3): it
//! calls [`Engine::submit`] and [`Engine::poll`] and draws what it finds.

use std::path::Path;
use std::time::Duration;

use rcraft_rhost::{
    Event, InterruptOutcome, Limit, RunStatus, Session, SessionConfig, SessionError,
};
use rcraft_rproto::RunId;

use crate::history::{CommandId, History, Outcome, Source};
use crate::results::{Item, Results};
use crate::transcript::Transcript;

/// The most events handled in one [`Engine::poll`], so a flood of output
/// cannot freeze an interface that polls once per frame.
const MAX_EVENTS_PER_POLL: usize = 500;

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
        // ask R first, so a refused command (busy, not running) leaves no trace
        let run = self.session.run(code)?;
        let id = self.history.push(source, code);
        self.results.begin(id);
        self.transcript.command(code);
        self.current = Some((run, id));
        self.last_command = Some(id);
        Ok(id)
    }

    /// Takes in what R has produced, waiting up to `wait` for the first
    /// event.
    pub fn poll(&mut self, wait: Duration) -> Polled {
        let mut polled = Polled::default();
        let mut wait = wait;
        for _ in 0..MAX_EVENTS_PER_POLL {
            let Some(event) = self.session.next_event(wait) else {
                break;
            };
            wait = Duration::ZERO;
            polled.changed = true;
            if let Some(id) = self.apply(event) {
                polled.finished = Some(id);
            }
        }
        polled
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
    }
}
