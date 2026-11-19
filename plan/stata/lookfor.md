# lookfor (2026-11-19)

Observed on Stata 19 SE with made-up data (corpus `lookfor.do`, r(varlist) exact; which variables each query finds was checked
against Stata's log). Translated in `crates/statatr/src/data_r/meta.rs`.

- `lookfor word ...` finds the variables whose name or variable label holds one of the words (any one, not all), ignoring
  capitals; a quoted phrase is one word. Value labels, display formats and notes are not searched. The variables come in the data's
  order and are left in r(varlist); nothing found prints nothing and leaves no r(varlist).
- No words is r(198) (`nothing to look for`).
- Stata prints its `describe` rows for them (storage type, display format, value label, variable label). R has no storage types or
  display formats, so RCraft prints the name, R's class and the label.
