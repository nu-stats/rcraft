## STATUS

## P2B.SW6: missing-variable checks in more places, 2026-11-26

- "variable x not found (Stata: r(111))" before anything runs, instead of a raw dplyr or R error, now also for the variables in a `by`/`bysort` prefix, `keep`/`drop` lists, `egen fn(varlist)` and its `by()`, `collapse` (variables, `new = var` and `by()`), the `if` of keep, drop, count, egen, collapse, generate, replace, tabstat and the models, and the `by()`, `over()`, `cluster()`, `absorb()` and `vce(cluster x)` options. The check is `crates/statatr/src/data_r/namecheck.rs`; it names only plain variables: not functions, `L.x`, names next to a macro, `_n`, r()/e() values, scalars or user matrices, anything in quotes, `drop _all`, or a line run in another frame.
- Not done: variables in the right-hand side of `generate`/`replace` (`gen y = log(nosuch)`) still give R's own error; a Stata type mismatch (a number compared with text) still gives dplyr's message.
