//! Session state, history, log and the link between catalog and R.

pub mod engine;
pub mod history;
pub mod results;
pub mod transcript;
pub mod workspace;

pub use engine::{Engine, EngineError, Polled};
pub use history::{CommandId, History, HistoryEntry, Outcome, Recall, Source};
pub use results::{Item, ResultBlock, Results};
pub use transcript::Transcript;
pub use workspace::WorkspaceItem;
