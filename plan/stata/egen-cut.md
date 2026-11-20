# egen cut() (P2C.12ap)

Written 2026-11-19. `egen [type] newvar = cut(exp) [if] [in], at(numlist) | group(#) [icodes label]`. Code:
`crates/statatr/src/data_r/cut.rs` (the translator and the R helper `stata_cut`). Checked by
`crates/engine/tests/stata_corpus/egen_cut.do` (699 results, tolerance 1e-9). Ported from the decoding in
statcraft's `plan/stata/egen.md` (probe 581), then checked and extended on Stata 19 SE with made-up data.

## Rules

- **at():** the value gets the left end of the bin [a, b) it falls in; below the first point, and at or above
  the last one, it is missing. One point alone makes every value missing and is no error. Points are read as a
  numlist: blanks or commas, `a(d)b`, `a/b`.
- **icodes** (and `label`, which implies it): the bin's number from 0 instead of its left end.
- **group(k):** k bins of about equal count. Cut points are the smallest value and the j/k percentiles of the
  selected rows (j = 1 ... k-1), by Stata's default percentile rule: with n values, position n·j/k; a whole
  position is the mean of that value and the next one up in order, otherwise the next one up. A value's code is
  the number of cut points at or below it, less one, so ties at a cut point go to the upper bin and equal cut
  points skip codes (n = 40 values 0..3 ten times each, `group(6)`: codes 1, 2, 4, 5). `icodes` changes nothing
  under group().
- **The value is read as a single-precision number**, whatever its type: a double 0.7 is below the cut point 0.7
  (float(0.7) is smaller), a double 0.5999999999 reaches the cut point 0.6 (it rounds up to float(0.6)), and a
  double 2.9999999999 reaches 3. Cut points stay doubles. Percentiles for group() come from the same
  single-precision values, so a double variable and a float copy of it fall in the same groups.
- **Type:** float unless `set type double` or `egen double`; with double the result is the point itself, 0.6
  exactly. `egen byte` with at() made a float in Stata (not whole numbers); `byte`, `int` and `long` are refused.
- **label:** a value label named after the variable (a label already there is replaced); its entries are the
  codes 0, 1, ... with the text of the cut point in `%9.0g` and a `-` after it (`18-`, `30.5-`, `1.23e+07-`).
  at(): every point, the last included. group(): the k cut points and, last, the largest value plus one (even for
  fractions: 2.85 gives `3.85-`).
- **Message:** `(N missing values generated)` after it, counting rows left out by `if` and `in`.
- **if/in:** only the selected rows count for the percentiles; the others are missing.

## Errors (the return codes; RCraft's text is its own words)

- neither at() nor group() (or `group(0)`): r(198); both: r(198); points out of order: r(124); a name already in
  use: r(110).
- group(1), a negative number or a fraction: r(198); more groups than the number of selected values plus one:
  r(198) (13 groups of 12 values work); no value selected under group(): r(2000); under at() it is all missing,
  r(0).

## Cases

On `age = 18 + mod(13n, 50)` with n = 40: at(18 30 50 70) gives 9, 18 and 13 rows; at(20(10)60) leaves seven
missing; group(4) gives 10 rows in each group with cut points 19, 30.5, 42, 55 (and 68 as the last label).

## RCraft refuses

`by`, `by()`, other options, `byte`, `int`, `long` and string types, a numlist form other than the three above,
a group() that is a macro or a word.
