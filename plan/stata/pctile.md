# pctile (2026-11-18)

Observed on Stata 19 SE with made-up data (corpus `pctile.do`: 55 results, exact). StatCraft's note
(`data-commands.md`) agrees on the cut points. Translated in `crates/statatr/src/data_r/descr.rs`
(`stata_pctile`). Batch C of the StatCraft port.

- `pctile [type] newvar = exp [if] [in] [weight], nquantiles(#) genp(newvar) altdef`. `nquantiles()` is 2 by
  default; the cut points go to the first # - 1 rows of the data (not the first rows of the sample); `genp()`
  has 100 j / #. New variables are float (`%9.0g`) unless a type is given (`double` `%10.0g`, `int` `%8.0g`);
  labels `percentiles of <exp as typed>` and `percentages for __000002` (a tempvar name Stata leaves in).
- r(r1) ... r(r#-1), doubles (not rounded to float), in `return list`.
- A percentile is the first value whose running weight reaches W p / 100, or the average of it and the next
  value when the running weight is exactly that (ties of exact fractions count: 9 rows, nq(3) gives 3.5 and
  6.5). Same as `summarize, detail` (checked with fweights and aweights). aweights and pweights scale away.
  `altdef` is R's quantile type 6 and may not be used with weights (r(198)).
- Errors (return codes compared in the corpus; messages by eye): nquantiles() below 2, or above the rows of
  the data plus one (the whole data, not the sample), r(198); not a whole number r(198) ("option nq()
  incorrectly specified"); no observations r(2000) (after the nquantiles check); a name or genp name there
  r(110) (also genp equal to the new name); a string r(109); iweights r(101); non-whole fweight r(401);
  a repeated option r(198); a bad name r(198).
- Not translated: `by:`, `percentiles()` (it belongs to `_pctile`, which Stata's pctile refuses),
  string types, more than 5000 quantiles.
- The storage type and display format were checked against Stata's describe by eye; the corpus cannot ask
  for them (`: type` is not translated).
