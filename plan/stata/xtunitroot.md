# xtunitroot (panel unit-root tests)

Ported from statcraft@7f17240 (its `panel.md`, probes 327 to 330) and checked against Stata 19 SE on made-up data of our own:
corpus `xtunitroot.do` (99 results, 1e-8) and, for `llc` (decoded here), `xtunitroot_llc.do` (146 results, 1e-8). Code: `crates/statatr/src/data_r/xtunitroot.rs`; the published tables it reads
(Im, Pesaran and Shin, Tables 1 to 3) are carried over from StatCraft.

## Rule

- `xtunitroot test varname [if] [in]`, one plain variable, strongly balanced panels with no gap in time inside the sample.
  A demeaned series (`demean`: the cross-sectional mean of each time removed) exists only inside the sample; an undemeaned one
  is read from the periods before the sample when the lags need them (`if t > 5` regresses on the lag of the first sample
  period). r(): the scalars `N N_t N_g` and the test's own, then the texts `test`, `deterministics` (not for fisher),
  `demean` (only with the option; absent otherwise), and for fisher `options` and `urtest`.
- **ips:** the ADF regression of each panel (`lags(0 to 8)`, constant or `trend`); t-bar and t-tilde-bar are the means of
  the panels' statistics. Without `lags()`: Z-t-tilde-bar (Table 1's moments, linear in T, the no-trend moments also with
  a trend) with p, and t-bar's exact critical values from Table 2 at the next tabulated N and T at or above the sample's;
  r(): `N_t N_g p_zttildebar cv_10 cv_5 cv_1 zttildebar zt ttildebar tbar`. With `lags()` (even `lags(0)`): W-t-bar from
  Table 3's moments at T (linear), and r() has `p_wtbar wtbar lags` but no `tbar`. `r(N)` is the number of observations the
  regressions used. `noconstant` is r(198).
- **ht:** the pooled AR coefficient of the detrended `y_t` on the detrended `y_{t-1}` (each detrended on its own pairs), then
  Harris and Tzavalis's fixed-T mean and variance at T (T - 1 with `altt`). `r(N)` is `n * T`; r() has `p z rho` only (no
  `mean`, `Var`).
- **breitung:** each panel scaled by the standard deviation of its differences; the statistic is the sum of `v x` over the
  root of the sum of `x^2` with Breitung's `x` and `v`; p is the normal cdf. Only the sample's own rows are used, also when it
  starts late. r(): `lags(0) p lambda N_t N_g N`.
- **fisher:** `dfuller` (with `lags()`, `trend`, `drift`) or `pperron` p-value of each panel, then Choi's P, Z, L*, Pm. L*'s
  scale uses `3 (5N + 3) / (pi^2 N (5N + 2))` and `t(5N + 4)`. The panel's dfuller reads the periods before the sample.
- **hadri:** KPSS-style partial sums of each panel's residuals on the constant (and trend); pooled variance unless `robust`;
  with `kernel(bartlett #)` or `kernel(parzen #)` the long-run variance over T. The p-value is the upper tail of the normal
  (use the upper tail, not `1 - cdf`: the values reach 1e-39).
- **llc** (decoded here, 2026-11-19, black box against Stata 19 SE; corpus `xtunitroot_llc.do`, 146 results, 1e-8; code in
  `data_r/xtunitroot.rs`): strongly balanced panels in the sample (anything else stops with r(498), "requires strongly
  balanced data"; a gap in time that all panels share is still refused here). Per panel: the ADF regression of `D.y` on `L.y`,
  `lags(#)` (default 1) lagged differences and a constant (or constant and trend; none with `noconstant`); the residuals `e` of
  `D.y` and `v` of `L.y` on the lagged differences and the deterministic terms, both divided by the panel's `s`, the residual
  standard deviation of the whole ADF regression (divisor: the regression's observations, no degrees-of-freedom term). The
  pooled regression of `e` on `v` gives `delta`, `se_delta` (from `Var_ep` = the residuals' mean square over all `N * ttilde`
  observations) and the unadjusted `td`. `sbar` is the mean over panels of the long-run standard deviation of `D.y` over `s`:
  `D.y` minus its mean (constant), detrended on a constant and a time trend (trend), or as it is (noconstant); the Bartlett or
  Parzen kernel at the panel's bandwidth (the weights `1 - l/(K+1)`, and Parzen's `1 - 6x^2 + 6x^3`, `2 (1 - x)^3`), terms
  beyond the length of the series dropped. `tds = (td - N ttilde sbar / Var_ep * se_delta * mu) / sigma`, p = normal cdf.
  `mu` and `sigma` are Levin, Lin and Chu's Table 2 at `ttilde` (the observations each panel's regression used), read from
  Stata's own values by probing every `ttilde` from 7 to 1000: rows 25, 30, 35, 40, 45, 50, 60, 70, 80, 90, 100, 250 and the
  limit; linear between rows, the 25 row below 25, the limit above 250. (Stata's trend sigma at 40 is .971, not the .871 a
  smooth column would have; the table is in `stata_xtur_llc_tab`.)
  - **The default bandwidth** of a panel is `floor(3.21 * R^(1/3))` where R is the number of rows the panel has in the *data*
    (not in the sample, whatever `if`, `in` or missing values do); `r(hac_lagm)` and the header's "lags average" are the mean over
    the panels in the sample (20.2 when one panel of five has 150 rows and the others 300). `kernel(bartlett #)` or
    `kernel(parzen #)` give every panel `#` (r(hac_lags), no `hac_method`); `kernel(bartlett llc)` is the default.
  - **A sample that starts late** reads the one period before it: `D.y` at the first sample period is formed (so `ttilde` is
    `T - lags` instead of `T - lags - 1`), a lagged difference before that is not read. A demeaned series (`demean`) exists only
    inside the sample (`ttilde = T - lags - 1`). Starting at the first row of the data is the ordinary case.
  - r(): the scalars `p_td` (noconstant only, the unadjusted t's p-value), `p_tds sig_adj mu_adj delta se_delta Var_ep sbar
    ttilde td tds N_g N_t N hac_lagm` (or `hac_lags`) and `adf_lags`, the texts `test deterministics hac_kernel hac_method`
    (not with an explicit lag count) and `demean`. A text r() name that is absent reads `.` in an expression, as Stata does
    (`stata_rget` now returns `.`; the macro form is empty).
  - Header: `Asymptotics: N/T -> 0` (`root(N)/T -> 0` and `Panel means: Not included` with `noconstant`); `ADF regressions:
    # lags`; `LR variance:  Bartlett kernel, 9.00 lags average (chosen by LLC)` (or `Bartlett kernel, 3 lags`); the table has
    `Unadjusted t` (with its p-value for noconstant) and `Adjusted t*`.
  - Errors: `kernel(qs #)`, `kernel(bartlett)`, a negative or fractional number of lags in `kernel()` or `lags()`, a repeated
    `lags()`, `kernel()` or `trend`, `robust`, `altt`, `trend` with `noconstant` are r(198); unbalanced panels r(498). Stata
    also has `lags(aic|bic|hqic #)` (the header says "0.75 lags average (chosen by AIC)") and `kernel(bartlett|parzen nwest)`
    ("3.12 lags average (chosen by Newey-West)"): refused, the selection rules are not decoded. With the lags nearly as many as
    the periods Stata prints huge values (a perfect fit); refused.
- Error codes: no test or an unknown one 198, an unknown option 198, `noconstant` with `trend` 198, `ips` or `hadri` with
  `noconstant` 198, fisher without `dfuller` or `pperron`, with both, without `lags()`, with `trend` and `drift` 198, a
  second variable 103, no variable 100, a name the data lack 111, no panel set 459. With one panel in the sample Stata runs
  every test (r(0)); so does RCraft.

## Open (refused)

- `llc` with `lags(aic|bic|hqic #)` or `kernel(... nwest)`, with a gap in time that all panels share, or with the lags nearly as
  many as the periods (a perfect fit): see the llc rule above.
- `breitung` with `robust` or `lags()`: not reproduced (second round, 2026-11-19, tried against Stata 19 on probe-330-style data):
  - `lags(p)`: `r(N)` stays `N * T`. Prewhitening by the residuals of `D.y` and `L.y` on the lagged differences (LLC style), and by
    filtering `D.y` and `L.y` with the coefficients of the lagged-difference regression (with or without a constant, or the whole
    ADF regression), with about 100 variants of the scale and the base period, gave -1.830 for Stata's -1.834 at `lags(1)` but
    -1.62 for -1.72 at `lags(2)` and -1.60 for -1.53 at `lags(3)`; no variant is within 2e-3 at all three. Every period of every
    panel moves lambda, so no row is simply dropped. Stata's panel-by-panel results (one panel, then pairs of panels) imply
    denominators `sum x^2 / s^2` that none of the candidates matched.
  - `robust` (r() adds `lrobust` and `p_lrobust`; `lambda` and `p` stay the plain ones; the display says `lambda*`): with one
    panel it is the ordinary t statistic of the pooled regression with the residual variance `sum r^2 / (m - 1)` (m = T - 1;
    checked on four panels to 7 digits), but with several panels neither a GLS statistic nor the sandwich over the periods, with
    the cross-section covariance taken from the pooled or the panel-by-panel residuals and either divisor, gives Stata's value
    (-2.4080 on the 12-panel probe; the GLS forms give -2.04 to -2.62, the OLS statistic with `x' Omega x` -2.47 and -2.16).
- `ips` with `lags()` above 8, `lags(aic #)`, unbalanced panels (Stata's W-t-bar over unequal T) and gaps in time inside the
  sample, `hadri` with `kernel(qs)` or an automatic bandwidth, `fisher` with `noconstant`.
- `fisher` with `pperron` and a trend, `demean`, or a sample that starts after the data do: Stata's p-values are not the
  standalone `pperron` ones there (the sum of the panels' `pperron` p-values differs from its P).
- `fisher` when a panel's p-value is exactly 0 or 1: Stata leaves that panel out of the logit and normal sums and rescales
  L* with the panels left (found with `L*`; Z, Pm and the degrees of freedom there are not decoded).
- `ht` with a trend on a sample that starts after the data do (Stata's rho there matches none of the forms tried).
- Fewer than 4 periods (Stata prints missing statistics).
