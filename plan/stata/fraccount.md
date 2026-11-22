# fracreg, betareg, boxcox, zip, zinb, tpoisson, tnbreg, ztp, ztnb (2026-11-22)

Observed on Stata 19 SE with made-up data (corpus `fracreg.do`, `betareg.do`, `boxcox.do`, `zcount.do`). Ported from
statcraft@b7d392e (its notes `fracreg.md`, `betareg.md`, `boxcox.md`, `models.md`, `weights.md`; probes 162, 268-270, 277, 281-283, 285,
289, 534), then checked line by line against Stata runs of our own data. Black box only: Stata's printed numbers and `e()`, no ado or
help source read. R is the engine: `data_r/frac.rs` (design, sandwich, table, `fracreg`, shared predict), `betareg.rs`, `boxcox.rs`,
`zcount.rs`; `stata_ml_max` (in `ivprobit.rs`) takes Stata's Newton-Raphson steps for all but `betareg` and `boxcox`'s log.

## Rules

- **fracreg**: quasi-likelihood sum y ln F(eta) + (1 - y) ln(1 - F(eta)), F logistic or normal; `het(z)`: eta = x'b / exp(z'g). Start:
  weighted least squares of y on x (het: the probit optimum, gamma 0). Newton-Raphson with the analytic Hessian. Always robust:
  sandwich with N/(N - 1) (G/(G - 1) with `vce(cluster)`, a frequency-weighted row counted w times). Wald test over the mean equation's
  slopes; ll_0 the constant-only fit (het: with the lnsigma equation kept). `e(mns)` holds each regressor's mean over the **whole**
  data, in reverse order under the regressors' names. aweights stop (r(101)).
- **betareg**: mu = g^-1(x'b), phi = h^-1(z'c). Starts: least squares of g(y) on x; h(mu (1 - mu) / var(y) - 1) on z (variance unweighted
  unless fweights). Then `Initial`, `Rescale` (whole vector x .5 else x 2, kept while it rises), `Rescale eq` (each equation), then BHHH for
  iterations 0-9 (`(setting technique to bhhh)`), `(switching technique to nr)` before iteration 10, NR with a Hessian from central
  differences of the analytic scores: equation l's predictor moved by (|mean| + 1e-4) 1e-4, block (k, l), k >= l, from that move only.
  Forward steps count only when they gain more than 1e-12 relative. LR test (Wald with robust, pweights, cluster); robust fits store no
  `e(ll_0)` unless weighted. aweights stop.
- **boxcox**: profile likelihood in the transform parameters (see StatCraft's note); Newton-Raphson, full model started at 1, per-variable
  LR fits at 0. The LR-comparison fits' logs print **even with `nolog`** (only `nologlr` hides them). fweights and iweights are taken by
  Stata (not translated here: refused); aweights and pweights stop (r(101)).
- **zip, zinb**: constant-only count model first (constant at ln ybar, inflation at 0, ln alpha at 0), then the full model from there; scores
  analytic, -H the central difference of the gradient (step 1e-5 max(|b|, 1)). `e(p)` uses np - 1 degrees of freedom (the number of
  parameters but one) when not robust, df_m when robust. The not-concave step is ml's modified Marquardt, **and where no length helps the
  ordinary step with -H + lambda I** (without that second rule zinb on probe 162's data stalls at its constant-only fit).
- **tpoisson**: from the Poisson fit to the truncated sample; ll_0 is the likelihood at the constant ln ybar, unfitted. **tnbreg / ztnb**: truncated
  Poisson fit, the constant-only fit, then the full model; the estimates are polished with a second run from where the first stops.
- **e() of tnbreg** (seen in Stata 19; StatCraft's note had it wrong): the same list as ztnb (`rank0`, `ll_c`, `chi2_c`, `alpha`, `k_aux`, `r2_p`, no
  `p_c`), macros `title`, `llopt`, `cmd`, `predict`, `dispers`, `chi2_ct`, user `tnbreg_mean`.
- Header ll formats: zip/zinb `%9.0g` trimmed; tnbreg, ztnb, ztp trimmed `%10.0g`; tpoisson and betareg right-aligned `%10.0g`.
- predict after tpoisson, tnbreg, ztp, ztnb: `n` (the default, `(option n assumed; predicted number of events)`) is exp(xb) over every row,
  not the truncated mean (`cm`).

## Not matched

- **tnbreg and ztnb**: Stata's start of the full model. Its iteration 0 is higher than every start tried (-203.1438 on probe 534's data; ours
  -203.4388): tpoisson slopes with ln alpha from the constant-only fit, with ln alpha optimised, with the constant and ln alpha optimised, Poisson
  slopes. The estimates and e() agree (1e-7), the log's first lines do not.
- **betareg** `e(ll_0)`: Stata's hidden constant-only fit stops 1e-9 relative above the true maximum at iterate-dependent points; the LR chi2 differs
  from the seventh digit (corpus tolerance 1e-6).
- **boxcox**: Stata's numerical Hessian moves its path in the seventh digit; estimates agree to about 1e-6.
- predict after zip and zinb; `predict, cm` and `pr()` after the truncated models; offset(), exposure(); probit inflation; factor variables in
  `inflate()`, `scale()` and `het()`; interactions and `ib.` in the main equation; boxcox with fweights or iweights.
