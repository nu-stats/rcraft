# sspace, ucm and dfactor and predict after them (2026-11-20)

Observed on Stata 19 SE with made-up data (corpus `sspace.do`, `ucm.do`, `dfactor.do`). StatCraft decoded the method first (its
`sspace.md`, probes 549 to 551; ported from statcraft@c5865ea); the code here is our own, in R
(`crates/statatr/src/data_r/sspace/sspace.R`, translator `sspace.rs`).

## Model and likelihood

- States `x_t = A x_{t-1} + C e_t`, observations `y_t = off_t + D x_t + v_t`, `Var(e) = Q` and `Var(v) = R` diagonal; `off_t` holds the
  constant and the exogenous variables of the observation equations.
- A stationary model starts from the states' unconditional variance. A model whose transition has a unit root fixed by constraints
  (`[lev]L.lev = 1`) and every `ucm` trend (`llevel`, `rwalk`, `lltrend`) start diffuse (exact diffuse filter, one observation at a time,
  one series only): each diffuse step adds `-ln(2 pi)/2` to the log likelihood, `e(stationary)` is 0, the table ends with
  `Note: Model is not stationary.`, and the one-step predictions are missing there (the first period; the first two for `lltrend`).
  The filtered states are missing at every diffuse step too.
- `sspace`: parameters in order: lags of the states (start 0.5), the observation equations' coefficients (states 1, exogenous 0,
  constant the mean of the series), the states' variances, the observation variances (starts half the largest variance of the series
  and half each series' variance). A constraint `[eq]term = #` fixes one; `[u]L.u` and `[u]L1.u` are the same term.
- `ucm`: regressors (start 0), then `var(level)`, `var(slope)` (starts half and a hundredth of the variance of the first differences),
  then the noise variance (a quarter of the variance of the series); `rwalk` has no noise term.
- `dfactor`: the factor has variance 1 and a first-order autoregression or none, each loading starts at 0.5, each error variance at half
  the series' variance. With `ar(1)` on the observation equations (no factor) each error is a state with no observation noise.

## Fit and variances

- Stata's own search (`searching for initial values`, BHHH, NR, `Refining estimates:`) is not reproduced and its log, `e(ic)`,
  `e(tech_steps)`, `e(technique)`, `e(ilog)`, `e(gradient)`, `e(gamma)`, `e(Cns)` and the data signature are not kept. RCraft's search is
  BFGS and then Newton on the working scale (variances as logarithms; BFGS ends when a step gains nothing or the gradient is under 1e-4),
  to Stata's optimum.
- Standard errors: observed information by numerical second derivatives of the log likelihood on the natural scale, steps in proportion
  to each parameter (1.2e-3 and 6e-4 times |x| + 0.01) with Richardson extrapolation. Variances are tested one-sided, their intervals
  truncated at zero, with the two-line note. The Wald test covers the free coefficients that are not constants and not variances;
  without any, the header has no chi2 line, `e(df_m)` is 0 and `e(p)`, `e(chi2)` are missing.
- A variance at its boundary (zero) leaves the Hessian singular; Stata prints it with a missing standard error, RCraft stops with
  the r(430) message.

## e() and names

- `e(b)` columns: `u:L.u`, `y:u`, `y:_cons`, `y3:xr`, `/state:var(u)`, `/observable:var(y)` (sspace); `y2:xr`, `/:var(level)`, `/:var(slope)`,
  `/:var(y2)` (ucm); `f:L.f`, `y1:f`, `/observable:var(e.y1)`, and `e.y1:L.e.y1` with `ar(1)` on the observation equations (dfactor).
  `_b[/state:var(u)]` and `_se[...]` work; a bare name is the first equation's.
- `e(k_eq)` counts the equations with coefficients plus `/state` and `/observable` (sspace), 1 (ucm), the factor, each series and
  `/observable` (dfactor). Scalars in Stata's order (without `e(ic)`), macros as StatCraft lists them (without `e(tech_steps)`,
  `e(technique)`, `e(datasignature)`).

## predict

- `xb`: the one-step prediction of the first observation, exogenous part included (sspace, ucm; for ucm with a regressor `xb` is the
  level plus the regression, `trend` the level alone). After `dfactor`, `xb` is the regression on exogenous variables alone: 0 here.
- `states, equation(name)` (sspace), `trend` (ucm), `factors` (dfactor): `smethod(onestep)` (default, a(t|t-1)) or `smethod(filter)`
  (a(t|t)). Stored as float.

## Cases (corpus, Stata 19.0)

- Estimates, standard errors and the log likelihood agree to about 3e-7 relative (the corpus tolerance is 1e-5); chi2 to 3e-7, the
  p-value of a very large chi2 to 2e-6 (probes 549, 551). Every corpus model ends at the same optimum as Stata's: AR(1) with noise, random
  walk with noise (diffuse), a constant, a regressor, a sub-sample (`if`), local level, random walk, local level with a regressor, local
  linear trend, one AR(1) factor behind three series, autoregressive errors.

## Open

Refused: constants or exogenous variables in state equations, lags beyond the first, `covstate()`, `covobserved()`, `from()`,
`technique()`, `level()`, `constraints()` that are not `[eq]term = #`, `ucm` without `model()` or with other models, cycles, seasonals,
`dfactor` with constants, exogenous variables, more than one factor, `ar()` above 1, a factor together with `ar()` on the errors, missing
values or gaps inside the sample, panel data, `estat`, `margins`, `test` and the rest of the post-estimation commands, `predict` with
`smoothed`, `rmse()`, `dynamic()`, residuals, or without a statistic.
