# ranksum, signrank, signtest, kwallis, spearman (2026-11-14)

Observed on Stata 19 SE with made-up data (probe `r14/n1.do` in the session scratchpad, set line by line against
RCraft's output: identical but for the last digits); corpus `ranktests.do` (tolerance 1e-6, for kwallis's chi2).
Translated in `crates/statatr/src/data_r/ranktests.rs`.

- `ranksum x, by(g)`: the two groups named by their value labels, `obs`, `rank sum`, `expected` and a combined
  row; then `unadjusted variance`, `adjustment for ties`, `adjusted variance` (`%-21s%10.2f`), the H0 line with
  the two values as `%9.0g` writes them, `z = %6.3f`, `Prob > |z| = %6.4f`, and `Exact prob` when N ≤ 200. The
  exact p is worked out over every way of drawing group 1's ranks: a count, by dynamic programming, of the
  subsets of size n₁ of the doubled midranks by their sum. r(): N_1, N_2, z, Var_a, sum_obs, sum_exp, p, p_l,
  p_u, p_exact, p_l_exact, p_u_exact (the one-sided exact p are kept, as Stata keeps them).
- `signrank x = w` (or `= #`): positive, negative and zero rows with obs, sum ranks and expected; the variance
  lines with the ties' adjustment −Σ(t³ − t)/48 and the zeros' −z₀(z₀ + 1)(2z₀ + 1)/24; z, Prob > |z| and the
  exact p over every sign of the nonzero ranks (the zeros ranked and left aside, as Stata does).
- `signtest x = w`: the sign table, then the one-sided tests (`Pr(#positive >= k)` and `Pr(#negative >= k)`,
  binomial with p = 0.5 over the nonzero signs) and the two-sided one; r(): N_pos, N_neg, N_tie, p, p_l, p_u.
- `kwallis x, by(g)`: a boxed table of each group's Obs and Rank sum, `chi2(df) = %6.3f` with `Prob`, and the same
  with ties. r() in Stata's order: p_adj, p, chi2_adj, df, chi2. chi2 agrees with Stata's to 1.6e-7 relative:
  a difference not traced yet; the corpus checks it to 1e-6.
- `spearman x w z` (three or more): the lower triangle of the rank correlations, `%9.4f`, over the rows with
  none of the variables missing. r() after it was not checked: left as lost.
- Refused: `spearman` of two variables (Stata 19's p-value is from its own approximation to the permutation
  distribution and differs from R's t form); `swilk` (Stata's W is not R's `shapiro.test`); weights; the
  options other than by(). A varlist pattern or range (spearman x*) is made the data's names when the do-file runs.

## After xtregar (the same step)

- `test`, `lincom`, `nlcom` and `estat vce` after `xtregar, fe` and `xtregar, re` read xtregar's e(b) and e(V)
  as Stata 19 does: F and t on e(df_r) after fe, chi2 and z after re; the stub is e(depvar). Probe `r13/x1.do`
  identical; corpus `xtregar_tests.do` (tolerance 1e-7: the re test's p, about 4e-8, is Stata's to 5e-8
  relative).

## pca and factor with varlist ranges

- `pca x1-x5`, `factor q*`: the names are found in R (`dplyr::select`) before the call, as the corpus and the
  probes' do-files write them.

## ktau (2026-11-15)

Observed on Stata 19 SE with made-up data (StatCraft's probe 530 and `r20/k1.do` to `r20/k5.do` in the session
scratchpad, set line by line against RCraft's output: identical but for r() digits past the 14th and p-values near
1e-14); corpus `ktau.do`. Translated in `stata_ktau` (`crates/statatr/src/data_r/ranktests.rs`).

- Score S = Σ sign(xᵢ − xⱼ)·sign(yᵢ − yⱼ); tau-a S/n₀; tau-b S/√((n₀ − T)(n₀ − U)); the variance of S corrected for
  ties (Kendall's), p = 2Φ(−(|S| − 1)/se). "(corrected for ties)" when either variable has a tie.
- Ties: Stata 19 counts the score's ties loosely and the corrections' exactly. x4 = −x1 + mod(_n, 3) has 22 tied
  pairs by exact equality (Stata's `duplicates report` agrees) but Stata's tau-a of x4 with itself is 1 − 26/780, and its
  tau-b of x4 with itself 754/758. Values one bit apart are tied in the score, values 1e-9 or 1e-12 apart are not;
  RCraft compares the score's values at 15 significant digits, which fits every case seen (the true rule lies between).
- Three or more (or matrix): listwise unless pw; the lower triangle of tau-a by default; stats() in the order taua,
  taub, score, se, obs, p, with the Key box unless tau-a alone; "Number of observations = N" unless obs is one of the
  stats ("note: number of observations varies." when pw makes them differ); print() blanks a cell whose p is above it,
  star() marks the first line of a cell; bonferroni multiplies p by the k(k − 1)/2 pairs, sidak 1 − (1 − p)^pairs.
  Cells `%10.4f` (obs `%10.0f`) and a marker column; names abbreviated to 8 over the columns, 12 for the rows.
- r(): the last pair's (row k, column k − 1) N, tau_a, tau_b, score, se_score, p; r(Nobs), r(Tau_a), r(Tau_b),
  r(Score), r(Se_Score), r(P) (adjusted; missing on the diagonal).
- Errors: one variable "too few variables specified", r(102); an unknown option r(198).
