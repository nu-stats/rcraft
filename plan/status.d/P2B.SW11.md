## STATUS

## P2B.SW11: type mismatch (r(109)) in an if, 2026-11-26

- A plain variable compared with a string literal or a number in the `if` of summarize, tabulate, list, count, regress, keep, drop and the other commands with a variable check now stops before anything runs with "type mismatch (Stata: r(109))" when the kinds differ: a number or labelled number against a string, a string against a number. Before, `sum x if g == "a"` with a labelled `g` gave dplyr's "Can't combine `x` <character> and `y` <double>". A missing variable is still r(111) first, and the right kinds run as before. The scan (`type_pairs` in `crates/statatr/src/data_r/namecheck.rs`) takes only a plain variable beside a literal; function calls, subscripts, `L.x`, macros, scalars and variable-to-variable comparisons are left to R. Real-R test in `crates/engine/tests/stata_regress.rs`.
- Not done: a mismatch built inside an expression (`x + 1 == "a"` is caught, `f(x) == "a"` is not) and a `generate`/`replace` right-hand side still give R's own message.
