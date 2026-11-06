# Date display formats, confirm, and compound-quoted files (2026-11-05)

## Date and time display formats

Ported from StatCraft's decoding of Stata 19 (its `crates/core/src/datetime/display.rs` and
`plan/stata/dates.md`, probe 033), our own work, written in R (`stata_tfmt`, used by `stata_fmt`):

- `%td`, `%tc`, `%tw`, `%tm`, `%tq`, `%th`, `%ty` and the old `%d`, with every code (`CCYY`, `cc`, `YY`,
  `NN`, `nn`, `DD`, `dd`, `JJJ`, `jjj`, `Month`, `month`, `Mon`, `mon`, `DAYNAME`, `Dayname`, `Day`,
  `day`, `Da`, `da`, `WW`, `ww`, `q`, `h`, `HH`, `hH`, `Hh`, `hh`, `MM`, `mm`, `SS`, `ss`, `.s`, `.ss`,
  `.sss`, `am`, `AM`, `a.m.`, `A.M.`), `_` for a space, `+` for nothing, `!c` for c, and the separators.
- The width is the widest text the codes can give (`Month` 9, `Da` 3); shorter text is padded to it,
  on the left unless `%-t`. Out of 01jan0100 to 31dec9999 the value is shown in `%w.0g`, w the width
  capped at 12. Fractions are truncated.
- Checked: all 1,755 of probe 033's lines that RCraft translates give Stata's text byte for byte (123
  of them in `stata_returned.rs`). `%tC` (leap seconds) is declined, as for the date functions; `.a`
  shows as `.`, the extended missing values being NA in R.

## confirm

From StatCraft's notes (probe 120): `[numeric|string] variable` (r(111); r(7) "'x' found where string
variable expected"), `new variable` (r(110)), `[integer] number` (r(7)), `file` (r(601)) and `new file`
(r(602)), stopping in R with Stata's message so `capture` leaves Stata's `_rc`. A variable may be
abbreviated. `confirm file` is declined after a line not translated that may have written files.

## Files in compound quotes

A file name in compound quotes (`` save `"`f'"' ``) had its quotes kept, so `save`, `use` and the rest
wrote or looked for the wrong file; they are taken off now.
