# misstable (2026-11-19)

Observed on Stata 19 SE with made-up data (corpus `misstable.do`, r() exact; the table read against Stata's log line by line, for
every case in it). StatCraft's `data-commands.md` (ported from statcraft@7fa0429) describes the table. Translated in
`crates/statatr/src/data_r/meta.rs`.

- `misstable summarize [varlist] [if] [in] [, all]` shows the numeric variables with a missing value (all numeric ones with `all`; string
  variables never): `Obs=.` (system missing), `Obs>.` (`.a` to `.z`), `Obs<.` (the rest, always printed even when 0), the number of
  unique values (`>500` past 500) and the minimum and maximum in `%9.0g` (`.` with no value). A count of 0 in the first two is blank.
  A name longer than 12 is shortened with a `~`. `if` and `in` choose the rows.
- With nothing to show it says `(variables nonmissing or string)`.
- r() holds the last variable shown: r(N_lt_dot), r(N_eq_dot), r(N_gt_dot), r(K_uniq), r(min), r(max) (full precision; a float's are
  its float values) and the macro r(vartype) = `numeric`. With nothing shown the numbers are all `.` and r(vartype) is `none`.
- A name that is not there is r(111).
- Not translated: `misstable patterns`, `tree`, `nested` (what they print and return follows rules not decoded: the order of the
  variables in a pattern, the percents, r(K) counting every variable) and `generate()`, `exok` and the other options.
