# power (2026-11-14)

Observed on Stata 19 SE with no data (probes `r16/p1.do` to `r16/p5.do` in the session scratchpad: 82 power lines (81 translated, test(lrchi2) refused) set
line by line against RCraft's output, `return list` included: identical but for r() digits past the 12th);
corpus `power.do`. Translated in `crates/statatr/src/data_r/power.rs`. The power functions are those of the
public [PSS] manual.

- Methods: `onemean m0 ma`, `twomeans m1 m2`, `pairedmeans ma1 ma2`, `oneproportion p0 pa`, `twoproportions p1
  p2`. Two values and no n(): the sample size at power(0.8) by default; with n() (or n1() and n2()): the power;
  one value with n() and power(): the target value, above the first unless direction(lower).
- Tests: t on n − 1 (paired, one sample) or n₁ + n₂ − 2 with a noncentral t, Satterthwaite's df for sd1() ≠
  sd2(), z with knownsd/knownsds; oneproportion's score test (or test(wald), whose null sd is the
  alternative's); twoproportions' Pearson chi-squared as the pooled z, with continuity Fleiss's (1/n₁ + 1/n₂)/2
  taken off |p₂ − p₁| in both tails (the lower tail too: Φ((−|d| − c − z·s₀)/s), found by matching
  `continuity n(150)` to 1e-16). pairedmeans with sd() and corr(): sd_d = sd·√(2(1 − corr)).
- Sample sizes: the root of power(n) = target, rounded up; two groups: N1 rounded up, N2 = ⌈nratio·N1⌉
  (`nratio(1.5) continuity`: 87 and 131, actual ratio 1.5057 printed and kept as r(nratio_a)). A total n() that
  does not split at nratio(): N1 = ⌊n/(1 + nratio)⌋, N2 = ⌊nratio·N1⌋, printed under "Actual sample sizes:",
  r(N) the total given and r(N_a) the groups'.
- "Performing iteration ..." is printed when Stata searches: always for t tests and for two-sided z tests;
  not for one-sided z tests of means or for one-sided sample sizes of proportions (closed forms), but yes for
  one-sided target proportions.
- The H0 line: `H0: m = m0  versus  Ha: m != m0` (> or < one-sided, by the sign of the difference); the
  target value's form adds `; ma > m0` (or <, direction(lower)) after one sample always, after two only
  two-sided.
- Study parameters: `%13s = %9.4f` (whole numbers `%9.0f`), pairedmeans in two columns (ma1, ma2, sd, corr on
  the right); twoproportions' delta with `  (difference)`, `  (ratio)` or `  (odds ratio)`.
- r(): in Stata's order for each form (it differs between the power, sample-size and target forms, and
  between the closed and searched ones); r(converged), r(maxiter), r(tolerance) and r(ftolerance) are
  Stata's constants. r(init), r(iter), r(deltax) and r(function) are details of Stata's own search and are
  not kept: reading them is refused, and `return list` says so in a note.
- Refused: lists of values (tables, graphs), the other methods, test(lrchi2)/test(fisher)/test(binomial),
  n1() or n2() alone, n() with n1() and n2(), and options not named here.
