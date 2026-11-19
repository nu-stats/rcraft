# cd without a folder (2026-11-19)

Observed on Stata 19.0 SE for Windows with made-up do-files, for the TZEN38 package's `cd "${dirdata}"` run with
the global never set (16 package lines).

- **Rule (Windows):** `cd` alone and `cd ""` print the current folder and stay there; `c(pwd)` is unchanged and `_rc`
  is 0. `cd "${unset}"` is `cd ""`. `cd "",` with a comma stops with r(198).
- **Elsewhere:** Stata's manual says `cd` with no folder goes to the home folder on macOS and Unix. Not observed here
  (Stata runs on Windows on this machine).
- **RCraft:** prints `getwd()` on Windows and leaves the folder as it was (files named after it are found where
  they were). On other systems the R stops with a message, rather than guess the home-folder rule.
- **Not in the corpus:** a corpus line holds numbers only and the corpus runs on all three systems, where the R
  stops off Windows; `crates/statatr/src/data_r.rs` has the translation's test.
