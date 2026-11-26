## STATUS

## P2B.SW12: variables on the right of generate and replace, 2026-11-26

- "variable x not found (Stata: r(111))" before anything runs, for the variables in the right-hand side of `generate` and `replace` (`gen y = log(nosuch)`), and the `if` expressions are now read with RCraft's own expression reader (`crates/statatr/src/data_r/namecheck.rs`) instead of by scanning the text: functions, strings, `r()`/`e()`/`c()` values, time-series operators, `_n`-style names and scalars are never read as variables, and text the reader cannot take, or that holds a macro, is left to the line's own error. A name counts only if it is written whole in the text.
