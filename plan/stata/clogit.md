# clogit, and vce() abbreviations (P2C.9)

Observed on a licensed Stata 19 SE with made-up data, 2026-11-02. Only what Stata did is written here.

## clogit

- `clogit y x w i.a, group(g)` takes y as positive where it is not 0.
- Groups with several positives are fitted, after `note: multiple positive outcomes within groups
  encountered.`
- Groups whose rows are all positive or all negative are left out:
  `note: 2 groups (8 obs) omitted because of all positive or` and, on the next line indented six spaces,
  `all negative outcomes.` e(N) counts the rows left (112 of 120); e(N_drop) is 8 and e(N_group_drop) 2.
- A variable that does not vary within any group: `note: xg omitted because of no within-group variance.`;
  it is a zero in e(b), marked `o.`.
- The header: `Conditional (fixed-effects) logistic regression` with `Number of obs`, `LR chi2(4)`,
  `Prob > chi2`, `Log likelihood` and `Pseudo R2`; with robust or clustered errors `Wald chi2(4)` and
  `Log pseudolikelihood`, and the line `(Std. err. adjusted for 10 clusters in c)`, or for `vce(robust)`
  `(Std. err. adjusted for clustering on g)`.
- e(ll_0) is the log likelihood with every coefficient 0 (not the one at Stata's iteration 0); the LR chi2
  is 2 (ll - ll_0). e(r2_p) is 1 - ll / ll_0. e(k) counts the factor's base column (5 for x, w and
  three levels of a); e(rank) and e(df_m) do not (4). e(N_group) is not set. e(group) is the group variable,
  e(multiple) is `multiple` when some group had several positives.
- e(b)'s columns are in equation `y:`; `_b[x]`, `_b[2.a]` read them; the base level is `1b.a`.
- `vce(robust)` clusters on the group: e(vce) is `robust` and e(N_clust) is not set. `vce(cluster c)`
  sets e(N_clust). When a group spans clusters: `groups (strata) are not nested within clusters`, r(459).
- Without `group()`: `group() required`, r(198). `strata(g)` gives the same fit as `group(g)`.
- `or` shows odds ratios; `_b[x]` stays the coefficient.
- `predict p` says `(option pc1 assumed; probability of success given one success within group)` and gives
  a value on every row, also in the groups left out: exp(xb) over its sum in the group (mean 0.25 with four
  rows a group). `predict xb, xb` gives xb on every row. `predict u, pu0` gives invlogit(xb), the
  probability with the fixed effect taken as zero.

## vce() abbreviations

- `vce(r)` and `vce(rob)` are `vce(robust)`; `vce(cl g)` and `vce(clu g)` are `vce(cluster g)`.
- `vce(c g)` is refused (r(198)), and so are `vce(o)`, `vce(oi)` and `vce(ol)`: `oim` and `ols` are taken
  only in full.
