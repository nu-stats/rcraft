# xtunitroot (panel unit-root tests)

Ported from statcraft@7f17240 (its `panel.md`, probes 327 to 330) and checked against Stata 19 SE on made-up data of our own:
corpus `xtunitroot.do` (99 results, 1e-8). Code: `crates/statatr/src/data_r/xtunitroot.rs`; the published tables it reads
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
- Error codes: no test or an unknown one 198, an unknown option 198, `noconstant` with `trend` 198, `ips` or `hadri` with
  `noconstant` 198, fisher without `dfuller` or `pperron`, with both, without `lags()`, with `trend` and `drift` 198, a
  second variable 103, no variable 100, a name the data lack 111, no panel set 459. With one panel in the sample Stata runs
  every test (r(0)); so does RCraft.

## Open (refused)

- `llc` (Levin, Lin and Chu): needs their Table 2 (the mean and standard-deviation adjustments by T and lag) and the kernel
  bandwidth rule of Stata's default; neither was in StatCraft, which also refuses it. Not decoded.
- `breitung` with `robust` or `lags()` (Breitung and Das's robust statistic; the prewhitening): StatCraft could not
  reproduce them.
- `ips` with `lags()` above 8, `lags(aic #)`, unbalanced panels (Stata's W-t-bar over unequal T) and gaps in time inside the
  sample, `hadri` with `kernel(qs)` or an automatic bandwidth, `fisher` with `noconstant`.
- `fisher` with `pperron` and a trend, `demean`, or a sample that starts after the data do: Stata's p-values are not the
  standalone `pperron` ones there (the sum of the panels' `pperron` p-values differs from its P).
- `fisher` when a panel's p-value is exactly 0 or 1: Stata leaves that panel out of the logit and normal sums and rescales
  L* with the panels left (found with `L*`; Z, Pm and the degrees of freedom there are not decoded).
- `ht` with a trend on a sample that starts after the data do (Stata's rho there matches none of the forms tried).
- Fewer than 4 periods (Stata prints missing statistics).
