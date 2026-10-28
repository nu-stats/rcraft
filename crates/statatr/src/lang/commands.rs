//! Stata command names and the shortest abbreviation Stata accepts.
//!
//! Vendored from StatCraft `crates/engine/src/commands/mod.rs` at commit
//! `a245039` (the table and `resolve`, unchanged). See the crate's `lib.rs`.

/// Full command names and the shortest abbreviation Stata accepts
/// (probe 090 and the manuals).
pub const COMMANDS: &[(&str, usize)] = &[
    ("display", 2),
    ("local", 2),
    ("global", 2),
    ("macro", 2),
    ("clear", 5),
    ("set", 3),
    ("program", 3),
    ("help", 1),
    ("_estimates", 4),
    ("version", 4),
    ("generate", 1),
    ("replace", 7),
    ("summarize", 2),
    ("describe", 1),
    ("list", 1),
    ("tabulate", 2),
    ("regress", 3),
    ("correlate", 3),
    ("count", 3),
    ("sort", 2),
    ("gsort", 5),
    ("drop", 4),
    ("keep", 4),
    ("label", 2),
    ("rename", 3),
    ("format", 4),
    ("merge", 3),
    ("append", 2),
    ("test", 2),
    ("estimates", 3),
    ("logit", 4),
    ("probit", 4),
    ("save", 2),
    ("saveold", 7),
    ("preserve", 8),
    ("restore", 7),
    ("erase", 5),
    ("use", 3),
    ("compress", 8),
    ("matrix", 3),
    ("scalar", 3),
    ("mata", 1),
    ("edit", 1),
    ("browse", 2),
    ("for", 3),
    ("log", 3),
    ("ereturn", 4),
    ("sreturn", 4),
    ("order", 5),
    ("encode", 2),
    ("decode", 3),
    ("return", 3),
    ("svydescribe", 6),
];

/// The full name of a command as written (`su` is `summarize`); a word that
/// is not in the table comes back unchanged.
pub fn resolve(word: &str) -> &str {
    if word.is_empty() {
        return "";
    }
    COMMANDS
        .iter()
        .find(|(name, min)| word.len() >= *min && name.starts_with(word))
        .map_or(word, |(name, _)| name)
}
