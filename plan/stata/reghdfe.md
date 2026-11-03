# reghdfe (SSC), as it behaves (P2C.9)

Installed with the user's go-ahead (`ssc install ftools`, `ssc install reghdfe`, and `require`, which reghdfe
asks for, 2026-11-03) and run on Stata 19 SE with made-up data. Only its output and e() were read, never
its code. Values below were reproduced in R before the translation was written.

## What it prints

- `(dropped 1 singleton observations)`, `(MWFE estimator converged in 3 iterations)`, then `HDFE Linear
  regression` with Number of obs, `Absorbing 2 HDFE groups`, F(2, 97), Prob > F, R-squared, Adj R-squared,
  Within R-sq., Root MSE; with clusters `Statistics robust to heteroskedasticity`, `Number of clusters (c)`
  and `(Std. err. adjusted for 5 clusters in c q)`; then the table, then a table of the absorbed degrees of
  freedom (Categories, Redundant, Num. Coefs; `*` for a fixed effect nested in a cluster variable).
- e(): N, N_full, num_singletons, df_r, df_m, df_a, df_a_initial, df_a_redundant, df_a_nested, rss, tss,
  tss_within, mss, rmse, F, ll, ll_0, r2, r2_a, r2_within, r2_a_within, N_clust (and N_clust1, N_clust2 for two
  ways), sumweights; macros absvars, vce (`ols` unadjusted), vcetype, clustvar, clustvar1, clustvar2, title,
  title2, title3, indepvars (`x w _cons`), depvar, cmd.

## What its numbers are (checked against them)

- A row alone in a group of an absorbed variable is dropped (and again, until none are left); with
  `keepsingletons` it stays, with a warning.
- The slopes are least squares on the variables with the absorbed means taken out. `_cons` and its standard
  error come from that fit with the grand means added back and a constant.
- df_a: an absorbed variable counts its levels; the second one less one for each connected group the two
  make (100 + 6 - 1 = 105). One nested in a cluster variable (each of its groups within one cluster) counts
  0. With `noabsorb`, df_a is 1. K is the slopes plus df_a, plus 1 when a fixed effect was nested.
- Unadjusted: s^2 (X'X)^-1, s^2 = rss / (N - K), df_r = N - K. `vce(robust)`: the sandwich times
  N / (N - K). Clustered: times (N - 1) / (N - K) G / (G - 1), df_r = G - 1. Two-way: the c and q sandwiches
  less the one on their intersection, all times (N - 1) / (N - K) G / (G - 1) with G the smaller count,
  df_r = G - 1. When that is not positive semi-definite it says `Warning: VCV matrix was non-positive
  semi-definite; adjustment from Cameron, Gelbach & Miller applied.`
- F is the Wald test of the slopes over their number.
- r2_a = 1 - (1 - r2) (N - 1) / (N - K - df_a) and r2_a_within = 1 - (1 - r2_within) (N - df_a) / (N - K -
  df_a), with df_a counting a nested fixed effect too (they do not change with the clusters).
- ll and ll_0 are -N/2 (1 + log(2 pi ss / N)) with the residual and the within sums of squares.
