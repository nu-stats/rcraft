# xtabond, xtdpdsys and xtdpd (2026-11-16)

Ported from StatCraft's decoding of Stata 19 (its `plan/stata/panel.md` and
`commands/xtabond.rs`, probes 326 and 420 to 430), our own work, written in R
(`data_r/xtdpd.rs`). Checked on those probes and in `xtdpd.do`, recorded from Stata 19.

## The fit

- Per panel, on the time axis (gaps and missing values cost only the equations that need them):
  the differenced equations t = p+2..T with GMM-type instruments, the lags of a series from `lo` to
  `hi` periods back, one column per period and lag (`L(2/.).y`, `maxldep(q)` makes it `L(2/q+1)`;
  pre() `L(1/.)`, endogenous() `L(2/.)`, maxlags() caps both, lagstruct(a, b) puts the lags 1..a among
  the regressors and instruments by `L(1/b).La.x`); standard instruments D.x for the exogenous
  regressors, inst() as they are, diffvars() in levels.
- The level equations t = p+1..T when there is a constant (its only instrument for xtabond) or
  level instruments: xtdpdsys's LD.y (and D.x of pre() or L2D.x of endogenous() variables), one
  column per period; xtdpd's lgmmiv() and liv().
- Columns that are zero in every panel are dropped; "Number of instruments" and e(zrank) are the
  rank of Z'Z.
- One step: W = (sum Z_i' H Z_i)^-1, H = A A' from the differencing, block diagonal when the level
  equations have instruments of their own (xtdpdsys, lgmmiv(), liv()); the Moore-Penrose inverse
  when it is rank deficient. sigma^2 = SSR of the differenced equations / (2(N - k)) from the final
  residuals. Two steps: W = S^-1, S from the one-step residuals; vce(robust) the sandwich, and
  Windmeijer's correction after two steps ("WC-robust").
- Number of obs and the groups count the level equations when there are any (xtabond: the
  differenced ones).

## The tests

- estat sargan: g'Wg / sigma^2 after one step, g' S^-1 g after two; "cannot calculate" after
  vce(robust).
- Arellano and Bond's m statistics pair each differenced residual with the one j **periods**
  before it, not j rows: on unbalanced panels with gaps the two differ (`xtdpd.do`'s unbal_ab:
  4.1149 by period, as Stata; 4.0996 by row).
- A one-step fit with vce(gmm) and a level equation has no AR tests: e(artests) = 0, no e(arm#),
  and estat abond is r(198) ("artests not computed for one-step system estimator with vce(gmm)"),
  for xtabond as for xtdpdsys (`xtdpd.do`; probe 430). Without a level equation (xtdpd,
  noconstant) they are there, with the homoskedastic variance (probe 427).
- estat abond with more orders than the fit's reprints the fit under "Dynamic panel-data
  estimation" and leaves the extra orders in r(arm) only: e(artests) and e(arm#) stay as the fit
  left them (`xtdpd.do`'s sys_arm: 2, e(arm3) missing).

## Errors and what is refused

- Kept: lags(0) r(198), maxldep(0)/maxlags(0) r(125), diffvars() naming a variable not in the model
  r(498), xtdpd without dgmmiv() r(198), lagrange(3 2) r(198), unknown options r(198), predict's
  other options r(198), estat abond after one step r(198), not xtset r(459).
- Stata **accepts** diffvars() with a constant when the variables are in the model (`xtdpd.do`:
  `xtdpdsys y x, diffvars(x)` is rc 0); that fit is not decoded, so it is refused.
- Not translated: weights, factor variables, abbreviated variable names, L(a/b) ranges among the
  regressors, the display options, e(datasignature) (Stata's checksum), estat abond after a
  one-step fit without a level equation, test/lincom/margins afterwards.
