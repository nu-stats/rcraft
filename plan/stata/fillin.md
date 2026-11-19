# fillin (2026-11-18)

Observed on Stata 19 SE with made-up data (corpus `fillin.do`: 66 results, exact). StatCraft's note
(`data-commands.md`, probe 573) agrees. Translated in `crates/statatr/src/data_r/rowcol.rs`
(`stata_fillin`).

- `fillin varlist` adds a row for every combination of the variables' values that the data lack. A
  variable's levels are the values it has; `.`, each of `.a` to `.z` and `""` are levels of their own.
  The other variables are missing in the added rows (`""` for strings); value labels, variable labels and
  formats stay.
- `_fillin` (byte, `%8.0g`) is 0 on the rows that were there and 1 on the added ones; complete data gain
  no rows but still get `_fillin`. Nothing is printed.
- The data end sorted by the varlist (missing last, `""` first); rows that repeat a combination are kept,
  in their order.
- Errors, in this order: no varlist r(100); one variable `too few variables specified` r(102); a name the
  data lack r(111); the same variable twice r(5) (`using data not sorted`, Stata's internal merge);
  `_fillin` already there `variable _fillin already defined` r(110). Weights and `if` are r(101).
- `by:`: see clonevar.md (r(190)).
- Not translated: a pattern or range as the varlist, data with no rows. More than 50 million
  rows is stopped with r(901), as StatCraft does.
