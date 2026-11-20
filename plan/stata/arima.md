# arima and predict after it (2026-11-20)

Observed on Stata 19 SE with made-up data (corpus `arima.do`, `arima_path.do`, `arima_more.do`). StatCraft decoded the method
first (its `time-series.md`, probes 157, 271, 576, 582, 586 to 589; ported from statcraft@b7d392e); the code here is our own, in
R (`crates/statatr/src/data_r/arima/arima.R`, translator `arima.rs`).

- Model: y = x b + u, u an ARMA process (`ar()`, `ma()`, and the seasonal `mar()`, `mma()`, `sarima(P,0,Q,s)`); `arima(p,d,q)`
  differences y and the regressors d times first and drops the first d observations. Exact likelihood by the Kalman filter
  started from the stationary distribution; sigma is a parameter of its own; standard errors are the outer product of the scores.
  The Wald test covers the regressors (not the constant) and the ARMA terms; e(p) is missing.
- Starting values: least squares for the regression part; with AR terms and no MA, least squares of the residuals on their own
  lags (no constant), sigma from that fit; with MA terms, Hannan and Rissanen's two regressions on the residuals (the first of p = 5k+1
  lags, k = (n-2)/15 between 1 and 6; the second on the AR lags and the first fit's residuals at the MA lags, MA terms kept in
  +-.95, sigma from the second fit). Seasonal terms start at zero.
- Path: BHHH for iterations 1 to 5, BFGS for 6 to 15, and so on (`(setting optimization to BHHH)` first, a switch line before
  each change), each step halved until the likelihood does not fall; stop when the coefficients' reldif is at most
  `tolerance()` (1e-6) or the likelihood's at most `ltolerance()` (1e-7) and g'Vg is at most `nrtolerance()` (1e-5); `iterate()`
  caps it (300). The scores are central differences of each observation's likelihood, as Stata's.
- e(): equation names `y` (not `D.y`, whatever the differencing), `ARMA`, `ARMA12` (seasonal), `sigma`; e(b) columns `y:x`, `y:_cons`,
  `ARMA:L.ar`, `ARMA:L2.ma`, `sigma:_cons`; `_b[_cons]` without an equation is the first equation's.
- predict: `xb` (default, `(option xb assumed; predicted values)`) is the one-step prediction of the (differenced) dependent variable;
  `y` adds back what the lags of y give; `residuals`, `yresiduals`, `mse` (sigma^2 times the filter's F); `structural` (regression part
  alone) and `dynamic(t)` (no data seen from time t on) modify xb. Observations lost to differencing get the constant alone as xb.
  An unknown option is r(198), as is two statistics at once.

## Cases (corpus, Stata 19.0)

- Tight tolerances (1e-10): ll, sigma, chi2 and every coefficient agree to 3e-5 (the OPG standard errors to 2e-8 relative, chi2 to 4e-7;
  the difference is Stata's own derivative step).
- Default path, AR terms: every step of `iterate(0..4)`, the iteration count e(ic) and the stop agree to 1e-6; with a regressor,
  `arima(2,1,0)`, `noconstant`, and `ar(1 2 3)` too.
- Default path, MA(1) and ARMA(1,1): the stops differ by about 1e-4 (Stata's line search runs past the full step, by a rule not decoded).
  `ar(1 2) ma(1)` and `ma(2)` (starting values not decoded) land on Stata's optimum to 1e-5 at default tolerances, but a path limited
  with `iterate(k)` would not match.

## Open

Refused: time-series operators and factor variables in the varlist, weights, `by:`, `constraints()`, `vce()`, `condition`, `diffuse`,
`from()`, `technique()`, seasonal differencing, panel data, missing values or gaps inside the sample, collinear regressors, replaying
the last fit, `predict` with `stdp` or `t0()`, forecasts beyond the data, `xb` before the sample with regressors, `estat` and the rest
of the post-estimation commands. `trace` and `gradient` are accepted and show nothing.
