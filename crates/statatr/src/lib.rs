//! Stata mode: reads Stata do-files and translates them to R (ROADMAP Phases
//! 2B and 2C). This crate never starts R and never reads a dataset; it turns
//! text into structures and, later, structures into R code.
//!
//! # Vendored from StatCraft
//!
//! `lang` and `values` are copied from StatCraft (our own project, which has
//! the same two licences as RCraft), at the commit recorded in `lang/SYNC_MANIFEST`
//! (`nu-stats/statcraft`, from the crates `statcraft-lang` and
//! `statcraft-core`). They are copied and not depended on
//! because StatCraft's repository is private and RCraft's is public, so a
//! dependency would break every build outside it.
//!
//! - `lang/expr.rs`, `lang/script.rs`, `lang/syntax.rs`: `crates/lang/src/`.
//! - `values/missing.rs`, `values/hex.rs`: `crates/core/src/`.
//! - `values/datetime/{mod,parse,leap}.rs`: `crates/core/src/datetime/`; the
//!   `display` part (date formats) is left out, with its two `pub use` lines.
//!
//! Nobody edits those files by hand. `cargo xtask sync-statcraft-lang` copies
//! them from StatCraft's `origin/main`, rewrites the paths (`statcraft_core::`
//! becomes `crate::values::`), leaves out the `display` part and writes
//! `lang/SYNC_MANIFEST` (the StatCraft commit and a hash per file).
//! `cargo xtask ci` re-hashes the files and fails if one differs. Fix a parser
//! bug in StatCraft, then sync. `lang/mod.rs`, `values/mod.rs` and
//! `lang/commands.rs` (the command table, which StatCraft keeps in its engine)
//! are RCraft's own. The clean-room rules (AGENTS.md section 1) held for the
//! original and hold for this copy: nothing here comes from Stata's program
//! files.

pub mod complete;
pub mod coverage;
pub mod data_r;
pub mod differences;
pub mod dofile;
pub mod expr_r;
pub mod lang;
pub mod live;
pub mod reader;
pub mod values;
pub mod verified;
