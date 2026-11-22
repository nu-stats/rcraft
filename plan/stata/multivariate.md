# manova, canon, discrim lda, mds (batch MV, 2026-11-22)

Ported from statcraft@b7d392e (its `models.md`, `multivariate.md`, probes 533 and 535), then checked against Stata 19 on
made-up data of our own: corpus `manova.do` (115 values) and `canon_discrim_mds.do` (148 values), every matrix element
agreeing to 1e-9. Translated in `crates/statatr/src/data_r/multi.rs`, `multi/cmds.rs` and the R in `multi/*.R`.

## manova

- `manova depvars = terms [if] [in]`; a variable is a factor unless written `c.x`; `a#b` is one term, `a##b` the main
  effects and the interaction. No `=`: `equal sign required between dependent variables and terms of the model`, r(198).
- E is the residual cross products of the full model. A term's H is its Wald form on the effects-coded fit
  (`B_k' (X'X)^-1_kk^-1 B_k`), which is the Type III change in the residual cross products; the model's H is against the
  constant alone. The four statistics (Wilks with Rao's F, Pillai, Lawley-Hotelling, Roy as an upper bound unless s = 1)
  use the term's df and the residual df.
- e(): N, k, rank (`k_eq` times the non-base columns), df_m, df_r, k_eq, df_#; text cmdline, marginsnotok, marginsdefault,
  p_F, F (%9.0g), r2, rmse, cmd, predict, estat_cmd, properties, depvar, indepvars, term_#; matrices b, V, eigvals_# and
  eigvals_m (the first min(df, k) eigenvalues of E^-1 H), aux_# (s, m, n), stat_# (rows Wilks, Pillai, Lawley, Roy;
  columns statistic, F, df1, df2, pvalue, exact, where `exact` is 1 for the rows the table marks `e`), H_#, E, B, xpxinv.
  b, B and xpxinv are the reference-coded fit, base levels shown as zeros: `1b.g`, `2.g`, and in an interaction the
  non-base level beside a base one is `2o.h`.
- Not translated: weights, options, ranges in the variable lists, `i.`-less bases (`ib#.`), nesting. `manovatest` belongs to
  batch L and reads this e().

## canon

- `canon (varlist1) (varlist2) [, lc(#) first(#) stdcoef]`. Correlations from the covariances; the correlations are the
  square roots of the eigenvalues of `S11^-1/2 S12 S22^-1 S21 S11^-1/2`; first-set vectors have unit variance and a
  positive first coefficient; the second set's is `S22^-1 S21 a / r`.
- The tests are manova's four statistics for the first set regressed on the second.
- `lc(#)` and `first(#)` cut e() as well as the printout: e(n_cc) is the number kept, e(b) and the coefficient and loading
  matrices have those columns, e(rank) = kept * k; ccorr and the stat_# stay whole.
- e(stat_#) is Wilks for correlations # onward (columns stat, df1, df2, F, p, exact; exact = min(p,q) <= 2).

## discrim lda

- Normal groups with the pooled within covariance; classification function k is `S^-1 mu_k` with constant `-mu_k' S^-1 mu_k / 2`
  in e(C): the log prior is *not* in e(C), it is added when classifying. Priors equal unless `priors(proportional)`; ties are
  missing. `predict newvar` is the classification, `estat classtable` the resubstitution table.
- Canonical discriminant functions: `Q = inv(chol(S))` (e(sqrtSinv) is its transpose), the eigenvectors of `Q' B Q / (N - g)`
  give `U = Q v` with unit within-group variance; e(L_unstd) adds the constant `-xbar' U`, e(L_raw) = U / sqrt(N - g),
  e(L_std) scales by the pooled sds, e(L_totalstd) by sqrt(diag(T)/(N-1)), e(canstruct) is `diag(1/sd) S U`, e(cmeans) the
  group means of the scores. Signs: the eigenvectors as R's LAPACK gives them agree with Stata's in the probe.
- e(candisc_stat): canonical correlation, eigenvalue, proportion, cumulative, Wilks (of this function onward), Rao's F with
  `ve = N - g`, df1, df2, p, exact. e(W_eigvals) and e(W_eigvecs) are the eigen decomposition of the within SSCP.
- Not translated: qda, knn, logistic, `priors(#...)`, `ties()`, `estat` other than classtable, value-labelled group
  variables (Stata names the groups by their labels; the R stops and says so), `predict` with `pr`.

## mds

- Classical metric scaling of L2 distances (`std`: standardized variables, held as floats by Stata, so rounded to float);
  `B = -J D^2 J / 2`, the positive eigenvalues (relative 1e-10), dimension() at most that many, default 2. Mardia fit 1 and 2.
- Each coordinate column is signed so the first observation's coordinate is positive (matched in the probe, both columns).
- Duplicate id values only print `(id() has duplicate values)`. e(unique) is `1` whatever the ids (observed).
- e(Ev) holds all N eigenvalues; the ones that are zero in exact arithmetic are noise in both programs (1e-14) and not
  compared. e(linearf) is the regression of the configuration distances on the dissimilarities.
- `mds` creates no variables and draws nothing here. Not translated: `loss`, `transform`, `method(modern|nonmetric)`,
  `dissim()`, `measure()`, `generate()`, `mdsmat`, `mdslong`, estat and plots.
