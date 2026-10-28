//! Stata mode: reads Stata do-files and translates them to R (ROADMAP Phases
//! 2B and 2C). This crate never starts R and never reads a dataset; it turns
//! text into structures and, later, structures into R code.
//!
//! # Vendored from StatCraft
//!
//! `lang` and `values` are copied from StatCraft (our own project, which has
//! the same two licences as RCraft), at commit `a245039`
//! (`nu-stats/statcraft`, branch `lang/shared-parsers`, from the crates
//! `statcraft-lang` and `statcraft-core`). They are copied and not depended on
//! because StatCraft's repository is private and RCraft's is public, so a
//! dependency would break every build outside it.
//!
//! - `lang/expr.rs`, `lang/script.rs`, `lang/syntax.rs`: `crates/lang/src/`.
//! - `values/missing.rs`, `values/hex.rs`: `crates/core/src/`.
//! - `values/datetime/{mod,parse,leap}.rs`: `crates/core/src/datetime/`; the
//!   `display` part (date formats) is left out, with its two `pub use` lines.
//!
//! The only edits are the paths (`statcraft_core::` became `crate::values::`)
//! and that left-out part. Fix a parser bug in both places; `plan/STATUS.md`
//! says so. The clean-room rules (AGENTS.md section 1) held for the original
//! and hold for this copy: nothing here comes from Stata's program files.

pub mod lang;
pub mod reader;
pub mod values;
