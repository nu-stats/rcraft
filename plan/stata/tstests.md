# dfuller, pperron, dfgls, corrgram, wntestq, varsoc (time-series tests)

Ported from statcraft@7f17240 (its `ts-tests.md`, probes 318, 319 and 271) and checked against Stata 19 SE on made-up data of
our own: corpus `tstests.do` (dfuller, pperron, dfgls, corrgram, wntestq; 1e-8) and `varsoc.do` (1e-8). Code:
`crates/statatr/src/data_r/tstests.rs`.

## Rule

- All of them work on one series on the grid of the `tsset` times: a time with no row, or a missing value, is a missing
  value, as `L.` and `D.` see it. A panel set by `xtset` is allowed when `if` leaves one panel; two panels in the sample
  stop with r(459). `if` and `in` choose the rows of the regression, but the lags are read from every row.
- **dfuller:** `D.y` on `L.y`, `lags(#)` lagged differences, `_trend` with `trend`, a constant unless `noconstant`.
  Z(t) is the t statistic of `L.y`. Critical values are Fuller's table (linear in the number of observations between the
  rows 25, 50, 100, 250, 500; the 500 row beyond). The p-value is MacKinnon's approximation, 0 or 1 beyond the ends of its
  range; `drift` uses the regression's t distribution, with `p = P(t < Z)`. No p-value with `noconstant`. A negative `lags()`
  counts as 0. r(): `lags N Zt p cv_1 cv_5 cv_10` (no `p` with `noconstant`).
- **pperron:** `y` on `L.y` (+ trend, constant); Newey-West lags `floor(4 (T/100)^(2/9))` unless `lags()` (negative gives the
  default); the autocovariances of the residuals are taken over the grid of times, a missing residual leaving a pair out,
  divided by T. r(): `N lags Zrho Zt pval`, and the 2 x 4 matrix `cvalues` whose rows are Z(rho) and Z(t) and whose columns are
  the statistic and the 1%, 5% and 10% values.
- **dfgls:** GLS detrending with `a = 1 + c/T` (c = -13.5 with a trend, -7 with `notrend`), then the regressions of `D.yd` on
  `L.yd` and lagged differences from `maxlag()` (Schwert's `floor(12 (T/100)^(1/4))`) down to 1, all on the same observations.
  The 1% value is from ERS (Fuller's no-constant tau with `notrend`), the 5% and 10% from Cheung and Lai's surface (all from ERS
  with `ers`). Ng-Perron's sequential choice, minimum SIC and MAIC are printed when `maxlag() > 1`. A gap in the sample stops
  with r(498) (it prints the number of gaps first). r(): `maxlag N siclag maiclag maicrmse maicmin sicrmse sicmin optrmse
  optlag` and the matrices `cvalues` (k, DFGLS, the three values) and `results` (k, MAIC, SIC, RMSE, DFGLS); with `maxlag(1)`
  only `maxlag`, `N` and the matrices; with `maxlag(0)` only `N cv_1 cv_5 cv_10 rmse0 dft0`.
- **corrgram:** autocorrelation = sum of products over the pairs of times both present, about the mean of the values present,
  over the sum of squares; partial autocorrelation = last coefficient of the regression on the first k lags (rows with all
  present), or Durbin-Levinson from the autocorrelations with `yw`; Q is Ljung and Box's with n the values present. Default
  lags `min(floor(n/2) - 2, 40)`; `lags(0)` and `lags(n or more)` stop with r(498). A gap is noted ("time series has k
  gaps": runs of missing values between the first and last value). r(): `lags` and, for lags 1 to 10 only, `p# q# pac# ac#`
  (last lag first), and the 1 x lags matrices `P Q PAC AC` with every lag. The bars are eighths of the correlation, truncated.
- **wntestq:** Q at the last lag, `chi2(lags)` p-value; r(): `df p stat`.
- **varsoc:** each lag order from 0 to `maxlag()` (default 4) fitted by least squares on the sample the longest order leaves;
  Sigma is the residual cross-product over n; `LL = -n/2 (K ln 2pi + ln|Sigma| + K)`, `FPE = |Sigma| ((n + k)/(n - k))^K`, AIC,
  HQIC, SBIC from LL with k parameters per equation times K. LR of an order is twice the change of LL, df K^2. Stars: the last
  order whose LR rejects at 5%, and the smallest of each other criterion. r(): `N_gaps mlag N tmax tmin`, the texts `exog
  endog`, matrix `stats` (lag LL LR df p FPE AIC HQIC SBIC; lag 0 has missing LR, df, p).
- Error codes seen: no tsset 111; a name the data lack 111; a second variable 103; no variable 100 (198 for `varsoc`);
  an unknown option 198; `lags(2.5)` 198 (`maxlag(2.5)` of dfgls 126); trend with noconstant, drift with trend or
  noconstant 198; a string variable 109 (r(2000) for dfgls and wntestq, which treat it as no values); no observations 2000;
  `dfgls` with `maxlag` leaving one observation 2001.
- The number lines of the reports are right-aligned to column 62, so the widths follow the number of digits of N and of the
  lag count (the larger sets the width).

## Open (refused or not done)

- `arch` (second round, 2026-11-19: probed, not translated). What Stata 19 does, seen with `trace gradient showstep` on made-up
  data (`arch y, arch(1)`, 300 observations): `e(tech)` is `technique(bhhh 5 bfgs 10)` (BHHH for 5 iterations, then BFGS for 10,
  cycling), `e(ml_method)` is `rdu0`, the standard errors are OPG. Start values: the mean of `y` (or the OLS fit), then the OLS of
  the squared residuals on their first lags: the constant and the ARCH coefficient (matched to 9 digits, as are the
  log likelihood at the start, with the presample `e^2` the mean squared residual, and the first BHHH step to 4e-9). The first
  iteration takes the full BHHH step `inv(sum s s') * sum s` and prints "initial step good"; it then steps forward by 1/8 of the
  step, and doubles the extra step each time (1/4, ...) while the log likelihood rises, and "ignores the last step" at the
  first fall (the point kept is the last good one). It stops when the log likelihood or the parameters have stopped moving and
  `g inv(H) g'` is under 1e-5 (2.3e-7 after 2 iterations here). So the path is plain; what is missing is the numerical score:
  Stata's gradient differs from the exact one in the 7th digit (-.3834467 against -.3834473 at the start), which moves the
  first iterate by 2e-9 relative (.0880127224804 for the exact-score .0880127241657): 8 digits fail by one. Central
  differences with fixed or relative steps from 1e-2 to 1e-8, and the adaptive-step rule found for spregress (a change of the
  total log likelihood between 1e-4 and 1e-3), do not reproduce it; a fit puts the steps near 4e-5, 1.5e-5 and 5.7e-5 for the
  three parameters, which follow no relation to the parameters tried. The rest of the path (the BFGS stage, `garch()`,
  `iterate()`) was not decoded. Refused: `arch` is "not translated yet".
- `ac`, `pac` (graphs) and `corrgram` after `var`-style estimation are not translated; `varsoc` without a varlist (after `var`),
  `noconstant` (Stata's lag-0 statistics and later rows differ from the plain formula, not decoded), `varsoc` after `xtset`.
- A sample too small for the regression (Stata shows a missing statistic, `.`) is refused for dfuller, pperron and dfgls.
- A collinear regression (Stata drops a term) is refused.
- `estat dwatson`, `bgodfrey`, `durbinalt` and `archlm` are not part of this batch.
