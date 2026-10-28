//! The CRAN package index: read, cache and search offline.
//!
//! CRAN publishes two files that matter here (checked 2026-10-28):
//!
//! - `src/contrib/PACKAGES.gz`: plain text (Debian control format), one record
//!   per package with the version, dependencies, licence and whether it needs
//!   compiling. No titles. Read by [`dcf`].
//! - `web/packages/packages.rds`: R's own package database, saved with
//!   `saveRDS`: the above plus title, description, authors, maintainer, links,
//!   system requirements and the date published. Read by [`rds`], which
//!   understands only the part of R's serialization format this file uses and
//!   reports anything else as an error rather than guessing.
//!
//! [`Index`] holds the packages, sorted by name, and answers lookups with no
//! network and no R. [`cache`] keeps a trimmed copy on disk, written to a
//! temporary name and renamed so a half-finished write never replaces a good
//! cache. [`refresh`] is the only code that asks for the network, through the
//! [`refresh::Fetcher`] trait, so every test uses a local stand-in.
//!
//! Package metadata is data, never instructions: nothing here runs it.

pub mod cache;
pub mod dcf;
pub mod describe;
pub mod http;
pub mod index;
pub mod lockfile;
pub mod plan;
pub mod rds;
pub mod refresh;
pub mod search;
pub mod table;
pub mod unpack;

use std::fmt;

/// What can go wrong reading, caching or fetching the index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The bytes ended before a value was complete.
    Truncated,
    /// Not the format expected, with what was wrong.
    Format(String),
    /// A well-formed file that uses something this reader does not handle.
    Unsupported(String),
    /// A count or size beyond the limits set for hostile input.
    TooBig(String),
    /// The cache or another file could not be read or written.
    Io(String),
    /// The network request failed.
    Fetch(String),
    /// The user pressed cancel.
    Cancelled,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Truncated => write!(f, "the package list ends too soon"),
            Error::Format(s) => write!(f, "the package list is not in the expected format: {s}"),
            Error::Unsupported(s) => {
                write!(f, "the package list uses something not supported: {s}")
            }
            Error::TooBig(s) => write!(f, "the package list is too large: {s}"),
            Error::Io(s) => write!(f, "{s}"),
            Error::Fetch(s) => write!(f, "could not download the package list: {s}"),
            Error::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;
