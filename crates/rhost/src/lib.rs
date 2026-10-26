//! Finds, installs and supervises R; manages the package library.
//!
//! P1.1 lives here: [`find`] looks for installed copies of R.

pub mod find;
pub mod pinokio;

pub use find::{find_installs, Env, Platform, RInstall, RVersion, MIN_VERSION};
