# estat vif, imtest, bgodfrey, durbinalt, dwatson, classification, gof (2026-11-13)

Observed on Stata 19 SE with made-up data (probes `r3/b1.do`, `r3/b3.do`, `r3/b4.do` in the session scratchpad, set
line by line against RCraft's output); formulas from Stata's public manuals ([R] regress postestimation, [R] regress
postestimation time series, [R] estat gof). Corpus `estat_tests.do` (1e-9) and `estat_fit.do` (1e-5). Translated
in `crates/statatr/src/data_r/estat.rs`.

- vif: 1/(1 − R²) of each regressor on the others and the constant; `uncentered`: the uncentred R², the
  constant's own row. Rows largest first, except with factor variables (then e(b)'s order, a factor's levels
  under its name). `    Variable |       VIF       1/VIF  ` (%10.2f, %12.6f), `    Mean VIF`. r(vif_#) are
  floats (1.468942403793335), r(name_#) the names in the order shown.
- imtest: N × uncentred R² of e² − σ² on [1, X, the products of X] (heteroskedasticity, df the rank less 1),
  e³ − 3σ²e on [1, X] (skewness), e⁴ − 6σ²e² + 3σ⁴ on 1 (kurtosis), σ² = RSS/N; `white`: N R² of e² on the
  same with a constant, before the table. Stata's values differ from these by 4e-8 (heteroskedasticity) to
  3e-6 (kurtosis) relative, not by float storage of the residuals or their powers (tried); what is printed
  agrees.
- bgodfrey: N R² of the residual on X and its lags (missing lags 0, or with `nomiss0` the rows dropped);
  durbinalt: the Wald chi2 of the lags in the same regression (OLS variance). Lags follow the time variable:
  a row whose t − 1 is not in the sample has a missing lag; "Number of gaps in sample = #" before the table.
  The table: 75 wide, `%8d     |%15.3f%16d%25.4f`, "H0: no serial correlation". r(): k, N, N_gaps, lags and
  1 × p matrices chi2, df, p with columns lags:1 ... dwatson: Σ(e_t − e_t−1)² over consecutive t / Σe²,
  `Durbin–Watson d-statistic(  3,   117) =  .8954601`, r(dw), k, N, N_gaps.
- classification: `Logistic model for yb` (Probit), the table of + and − by D and ~D, "Classified + if predicted
  Pr(D) >= .5", the eight rates and the share correctly classified (%-32s%-9s%8.2f%%); r(P_1n) ... r(P_corr)
  and r(ctable) (rows + − Total, columns D ~D Total).
- gof: Pearson's chi2 over the covariate patterns (rows of the model matrix), df patterns − coefficients;
  `group(#)`: Hosmer–Lemeshow over the quantiles of the predicted probabilities, each quantile the value at
  the smallest index whose share reaches j/g, the mean of it and the next when n·j/g is whole; the groups are
  the probabilities up to each; df groups − 2. `table` lists Group, Prob (the quantile), Obs_1, Exp_1, Obs_0,
  Exp_0, Total, with a rule after every five groups. r(): N, m, chi2, df, p.
- matrix list: a column is as wide as its equation name too (lags: over 1).

## Errors on panel data and after robust (2026-11-13; probe `r7/f4.do` identical)

- After regress, vce(robust): vif and imtest as without it; bgodfrey and durbinalt "This command does not work
  after  regress, robust" (two spaces), before anything else; on panel data all three "sample may not include
  multiple panels". vif: equal VIFs list the later variable first.
