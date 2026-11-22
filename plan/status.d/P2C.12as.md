## ROADMAP

- P2C.12as SSC add-ons (batch N of the StatCraft port, done 2026-11-22, in part)
  (`outreg2` in its own layout: no `tstat` or `se` (standard errors), outreg2's own number format by the size of the number, `bdec()` alone, `sdec()`, `nocons`, `noobs`,
  `nor2`, `noaster`, `alpha()` with `symbol()`, `stats()` (coef, coef se, coef with tstat, pval or ci, coef se tstat pval), `tex` alone, `word` (.rtf or .doc), a .txt name, and
  after logit, probit and poisson; `winsor2` (`cuts()`, `trim`, `replace`, `suffix()`, `by()` with missing groups, `label`, if and in, storage types, the add-on's errors);
  `unique` (counts, if and in, `by()` with the new variable, `generate()`, `detail`); `distinct` (`joint`, `missing`, `minimum()`, `maximum()`, `abbrev()`, if and in))
  Verified (corpus `outreg2_layout.do`, `winsor2.do`, `unique_distinct.do`; notes `plan/stata/{outreg2,winsor2,unique,distinct}.md`).
  **On hold, to be built:** the SPost13 commands `listcoef`, `fitstat`, `mtable`, `mchange` and `mgen` (about 15 probe lines of probe 158: their layouts differ by model and `mtable`,
  `mchange` and `mgen` need the delta method of `margins`; note `plan/stata/spost13.md`); `outreg2`'s `stats(... beta)`, `tex(pretty)`, `label` with `word`, and the new options with
  `append`, .xls or .tex; `unique, by()` puts the count on the first of tied rows where Stata's own sort picks another (message); `distinct` with `in` and several variables
  (the add-on counts the later variables oddly); `winsor2` with weights and patterns.
  Measured on this branch (origin/main at 54be4ac plus this batch; the total includes the other batches already on main) (probes: StatCraft b7d392e observe/dofiles; packages: the 46 do-files): probes 82.9% (24,038 translated, 4,973 not, 1,104 known only when run, of 30,115 source lines); packages 94.8% (4,028 of 4,247 judged; was 4,025 of 4,249).

## STATUS

## P2C.12as: SSC add-ons (batch N of the StatCraft port), 2026-11-22

- **Done** (each checked in Stata 19 on made-up data, to the byte for outreg2's files): `outreg2` in its own layout (corpus `outreg2_layout.do`, 27 whole-file digests,
  tolerance 0; probe 136's eight files match), `winsor2` (corpus `winsor2.do`, 39 results to 1e-13), `unique` and `distinct` (corpus `unique_distinct.do`, 43 results; their
  printed tables match probes 143 and 144 line for line). Notes `plan/stata/{outreg2,winsor2,unique,distinct}.md`.
- **Found:** `outreg2` writes numbers by size when bdec() is absent (4 significant digits from 1 up, 3 below, e-notation from 1e6 and below 1e-4; under 5e-11 no decimals);
  winsor2's cuts are `_pctile`'s, a winsorized new variable made by group is a float, `cuts(0 100)` does nothing with a message, and a replaced integer variable taking a fractional
  cut becomes float; `unique, by()` puts the count on the first row in Stata's sort of the whole data and its sort does not keep ties in order; `distinct` with `in` counts the later
  variables' rows oddly (41 of 41 rows with a missing). R's `$` partly matched `e$r2` to `e$r2_p` after logit (outreg2 showed a pseudo R-squared as R-squared; fixed with `[["r2"]]`).
- **On hold:** SPost13 (see ROADMAP).
- **Shares:** probes **82.9%** (24,038 of the 29,011 lines a dry run judges); packages **94.8%** (4,028 of 4,247; was 94.7%, 4,025): the package gain is the three `outreg2` lines that use its own layout. This batch's own refused lines were 10 (`outreg2`), 8 (`winsor2`), 7 (`unique`) and 8 (`distinct`) of probes 136, 142, 143 and 144, plus the lines after them.
