# clonevar (2026-11-18)

Observed on Stata 19 SE with made-up data (corpus `clonevar.do`, values and return codes exact; the
`(# missing values generated)` line and `describe` read by hand). StatCraft's note
(`data-commands.md`) agrees. Translated in `crates/statatr/src/data_r/rowcol.rs`.

- `clonevar newvar = oldvar [if] [in]`: the storage type, display format, variable label, value label and
  characteristics come with it; a float keeps its rounding. Where `if`/`in` does not hold the value is
  missing (`.`, or `""` for a string). The new variable is last.
- Prints `(N missing values generated)` (`1 missing value`) with N the missing values in the new variable,
  whatever their kind (`.c` counts, and `""` in a string); nothing when there are none.
- Errors, in the order Stata gives them: no `=` or no name, `syntax is clonevar newvar = varname ...`
  r(198); nothing after `=`, `too few variables specified` r(102); more than one name after `=` or
  options or weights, `varlist not allowed` / `weights not allowed` r(101) (before the check that the new
  name is free); the new name taken, `variable x already defined` r(110); the old name (or one in `if`)
  not there, r(111); a name that is not one, r(198).
- `by:` and `bysort` (second round): Stata does not allow them with any of the six commands: `NAME may not be combined with by` r(190), after a by-variable the data lack (r(111)) and data not in order (`not sorted`, r(5); bysort sorts first, so it gives 190 and leaves the data sorted). Corpus: `rc_*_by` lines in each file.
- Not translated: options, a pattern or range
  after `=` (Stata: `x2 not allowed` r(101) for the first name it finds).
