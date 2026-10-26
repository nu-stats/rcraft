//! Finds, installs and supervises R; manages the package library.
//!
//! P1.1 lives here: [`find`] looks for installed copies of R.

pub mod find;
pub mod pinokio;
pub mod procinfo;
pub mod session;

pub use session::{
    Event, InterruptOutcome, Limit, RError, RunOutput, RunStatus, Session, SessionConfig,
    SessionError, Stream,
};

pub use find::{find_installs, Env, Platform, RInstall, RVersion, MIN_VERSION};
