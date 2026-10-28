//! StatCraft's do-file language front end: reading a do-file into lines
//! (`script`), splitting a command line into its parts (`syntax`) and parsing
//! expressions (`expr`). Pure text in, structures out. Vendored; see the
//! crate's `lib.rs`.

pub mod commands;
pub mod expr;
pub mod script;
pub mod syntax;

pub use expr::{parse, parse_prefix, BinaryOp, Expr, ExprError, UnaryOp};
