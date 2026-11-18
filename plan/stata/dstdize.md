# dstdize (2026-11-18)

Observed on Stata 19 SE with made-up data (corpus `dstdize.do`: 20 results, tolerance 1e-6). StatCraft's note
(`epitab.md`) agrees; the printed output of probe 314 is identical to Stata's (by eye). Translated in
`crates/statatr/src/data_r/descr.rs` (`stata_dstdize`). Batch C of the StatCraft port.

- `dstdize casesvar popvar stratumvar [if] [in], by(groupvar) [level(#)]`. The standard population is every
  row's population by stratum; a group's adjusted rate is the sum of the standard share times the stratum rate
  (a stratum the group lacks adds nothing, no renormalising), the crude rate its cases over its population,
  the variance of the adjusted rate the sum of share^2 rate (1 - rate) / population, and the interval normal.
  Several rows for one stratum in a group (by() on a coarser variable) are added up.
- Printed: the per-group table (stratum, population with commas, cases, population and stratum distribution,
  rate, standard distribution, s*P), adjusted cases, crude and adjusted rates, the interval, and the summary
  table of the groups.
- r(): k, by, c1 ... ck (the groups as `"A"' text), matrices se, ub_adj, lb_adj, adj, crude, Nobs (1 x k,
  rows Se, Right, Left, Adjusted, Crude, Nobs, columns c1 ...).
- **Precision:** Stata stores adj and se as floats, and its figures sometimes sit one float step (about 7e-8
  relative) from the rounded double result, in a pattern not found; the corpus compares to 1e-6. Crude is a
  double.
- Errors: by() missing r(198); fewer than 3 variables r(102); level() outside 10 to 99.99 r(198); a name the
  data lack r(111); cases above the population r(499); a missing value in the cases r(198).
- Not translated: several by() or strata variables, `using()`, `base()`, `saving()`, `format()`, `print`,
  `nore`, weights.
