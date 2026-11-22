## ROADMAP

- P2C.12ba Fraction and count ML models (batch M of the StatCraft port, done 2026-11-22, in part)
  (`fracreg`, `betareg`, `boxcox`, `zip`, `zinb`, `tpoisson`, `tnbreg`, `ztp`, `ztnb`: Stata's logs and paths, fw/pw/iw weights where Stata takes them, robust and clustered errors,
  `irr`, `or`, `het()`, `scale()`, `link()`, `inflate()`, factor variables in `fracreg`, predict) Verified
  (corpus `fracreg.do`, `betareg.do`, `boxcox.do`, `zcount.do`; note `plan/stata/fraccount.md`; R in `data_r/{frac,betareg,boxcox,zcount}.rs`).
  **On hold** (listed, to be built): predict after `zip` and `zinb`, `predict, cm` and `pr()` after the truncated models, `offset()`, `exposure()`, probit inflation, factor variables in
  `inflate()`, `scale()`, `het()`, `boxcox` with fweights and iweights, and Stata's own start of the full model of `tnbreg`/`ztnb` (their logs differ from the first line; estimates agree).
  Measured: probes 82.1% -> 82.4% (23,829 -> 23,917 translated source lines of 30,115 judged), packages unchanged at 94.8% (no package calls these commands).

## STATUS

## P2C.12ba: fraction and count ML models (batch M of the StatCraft port), 2026-11-22

- **Done** (each checked in Stata 19 on made-up data; corpus `fracreg.do` 1e-8, `betareg.do` 1e-6, `boxcox.do` 1e-6, `zcount.do` 1e-7; note `plan/stata/fraccount.md`):
  `fracreg` (probit, logit, `het()`, `or`, fw/pw/iw, `vce(cluster)`, factor variables, `e(mns)`, predict cm xb), `betareg` (the four links and three scale links, `scale()`,
  Stata's rescale/BHHH/NR path, robust, cluster, fw/pw/iw, predict cmean cvariance xb xbscale stdp), `boxcox` (four models, `notrans()`, `lrtest`, restricted fits, predict yhat residuals
  btransform smearing), `zip` and `zinb` (`inflate()`, `irr`, weights, `vce()`), `tpoisson`, `tnbreg`, `ztp`, `ztnb` (three logs, LR test of alpha = 0, predict n xb). aweights stop as Stata
  stops (r(101)); boxcox refuses fweights and iweights (Stata takes them).
- **Found:** Stata 19's `tnbreg` leaves the same `e()` as `ztnb` (`rank0`, no `p_c`; StatCraft's note had `p_c`); the LR-comparison logs of `boxcox, lrtest` print under `nolog`; `zinb`'s
  Marquardt step needs the regularised ordinary step where no length helps (also fixes `stata_ml_max`, shared with ivprobit); `e(p)` of zip/zinb counts np - 1 degrees of freedom.
- **On hold, listed:** as in ROADMAP P2C.12ba.
- **Shares** (dry run, `many_dofiles.rs`, StatCraft `observe/dofiles` b7d392e and the 46 package do-files, measured on `origin/main` df4efce plus this branch): probes **82.4%** (23,917 of
  30,115 source lines translated, 5,094 not, 1,104 known only when run; was 23,829 / 5,182, 82.1%); packages **94.8%** (4,028 translated, 219 not), unchanged.
