//! Session state, history, log and the link between catalog and R.

pub mod engine;
pub mod history;
pub mod package_index;
pub mod results;
pub mod transcript;
pub mod workspace;

pub use engine::{Engine, EngineError, Polled, QueryId, QueryResult};
pub use history::{CommandId, History, HistoryEntry, Outcome, Recall, Source};
pub use package_index::{PackageIndexService, Status as IndexStatus};
pub use results::{Item, ResultBlock, Results};
pub use transcript::Transcript;
pub use workspace::WorkspaceItem;
