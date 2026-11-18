# swilk (2026-11-18)

Observed on Stata 19 SE with made-up data (corpus `swilk.do`: 15 results, tolerance 1e-6). StatCraft's note
(`tests.md`) agrees on W. Translated in `crates/statatr/src/data_r/descr.rs` (`stata_swilk`). Batch C of the
StatCraft port.

**What agrees, and to how many digits.** R's `shapiro.test` W (Royston's AS R94) differs from Stata's by about
5e-9 to 9e-9 in the untied cases tried (n = 10, 12, 25, 40, 120), so 7 digits agree. V, z and the p value are
computed from W by Royston's normalising transformation for n from 12 to 2000 (checked: given Stata's own
W they reproduce Stata's V, z and p to 16 digits), so they carry W's difference: relative 1e-7 to 4e-7 in p
(0.13038343 against 0.13038338). The printed table (W to 5 places, V and z to 3, p to 5) agrees except where
a value sits within that rounding of a boundary. The corpus compares to 1e-6.

**What does not.** With tied values Stata's W differs from R's by much more (a pair of equal values among 10
moved it by 0.003; with several tie groups by 0.01): not Sheppard's correction, and not the average-rank normal
scores either (that matched a single tied pair to 4e-9 and missed with more groups). The rule is not decoded,
so tied values stop with a message. For 3 to 11 values Stata's V comes from a rule not decoded either (it is
not the n >= 12 formula; StatCraft has the same gap), so those stop too, as does n above 2000.

- `swilk varlist [if] [in]`: no weights (r(101)); no varlist r(100); a name the data lack r(111); an option
  r(198); a constant variable r(2000) "no observations" (not a row of dots). A string variable, or fewer than
  3 values, prints a row of dots and a note (4<=n<=2000) with r(N) the count and the rest missing.
- r(): p, z, V, W, N of the last variable. (`r(W)` and `r(V)` are numbers here; RCraft otherwise treats those
  names as matrices.)
