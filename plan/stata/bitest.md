# bitest and bitesti (2026-11-18)

Observed on Stata 19 SE with made-up data (corpus `bitest.do`: 39 results, 9 digits). StatCraft's note
(`tests.md`) agrees; the printed output of probe 236 is identical to Stata's (by eye). Translated in
`crates/statatr/src/data_r/descr.rs` (`stata_bitest`). Batch C of the StatCraft port.

- `bitest varname == #p [if] [in] [fweight]` and `bitesti #N #succ #p`: exact binomial test. One-sided
  tails Pr(k >= observed), Pr(k <= observed); two-sided: the tail on k's side plus the opposite tail from
  `k_opp` out, `k_opp` being the first outcome on the other side of the expected count whose probability is
  no more than that of k. Equal probabilities count (to a relative 1e-7: 9 trials at p = .5 give 1.000000).
- k equal to the expected count: the two-sided p is 1 and k_opp = k. No outcome as unlikely on the other side:
  `k_opp` is missing, the two-sided p is the one tail, its label is that tail's, and a note says
  `upper tail of two-sided p-value is empty` (or `lower`).
- r(): k_opp, p, k, P_p, N, p_l, p_u (that order in `return list`). N and k are sums of fweights.
- Errors (return codes compared; messages by eye): a, p and i weights r(101), non-whole fweight r(401);
  a variable that is not 0 and 1, or #p outside (0,1), r(450); no `== #p` r(100); no rows or a string r(2000).
  bitesti: N not a whole number r(7), k above N or negative r(450), #p outside (0,1) r(450), wrong number of
  arguments r(198); k = 0 and N = 1 are allowed.
- Not translated: the `detail` option (it lists more outcomes than the two sides), `by:`, a #p that is not a
  number written out, more than 10 million trials.
