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

## onevariance, twovariances, onecorrelation, twocorrelations, pairedproportions (2026-11-16)

Observed on Stata 19 SE with no data (probes v1, c1 to c4 in the session scratchpad: about 90 power lines with
`return list`, set line by line against RCraft's: every printed line identical, r() equal to 12 digits or
more); corpus `power2.do`.

- `onevariance v0 va` (`sd`: s0 sa): the chi-squared test, (n − 1)s²/v0 against χ²(n − 1) scaled by
  v0/va. delta = r(ratio) = va/v0 (sa/s0 with `sd`), r(scale) "variance" or "standard deviation"; the title
  says "one-sample standard-deviation test" with `sd`.
- `twovariances v1 v2` (`sd`): F on **n1 − 1 and n2 − 1** df (in that order) scaled by v2/v1: the cut is
  F(1 − α; n1 − 1, n2 − 1)/delta. With unequal groups this is what Stata 19 does (n1(30) n2(50) onesided,
  0.5: power 0.6494811776988958; F on n2 − 1, n1 − 1 would give 0.6762); delta = v2/v1 (s2/s1).
- `onecorrelation r0 ra`, `twocorrelations r1 r2`: Fisher's z, se 1/√(n − 3) or √(1/(n1 − 3) + 1/(n2 − 3));
  delta = ra − r0 (r2 − r1).
- `pairedproportions p12 p21`: the large-sample McNemar test, pd = p12 + p21, δ = p21 − p12:
  Φ((δ√n − z·√pd)/√(pd − δ²)) and its lower tail. One-sided, the **power** form takes the upper tail
  whatever δ's sign (`0.3 0.2, n(200) onesided`: 0.000116, printed with "Ha: p21 < p12"); the sample size
  uses |δ| (`0.25 0.1, onesided` alpha(0.1) power(0.9): 99). effect(ratio): delta = p21/p12.
  `prdiscordant(pd)` with n() and power() (no values): p21 searched in (pd/2, pd) (direction(lower): below
  pd/2), printed as "Estimated discordant proportions", study line `    p12 + p21 =`, r(prdiscordant) last.
- Searches: "Performing iteration ..." for every two-sided form; one-sided, the sample sizes of
  onevariance and twovariances are searched and the rest are closed forms, the target values are closed
  forms but pairedproportions' (searched). Targets are searched on log(v) or Fisher's z.
- H0 lines: `v = v0` (`s = s0`), `v2 = v1` (`s2 = s1`), `r = r0`, `r2 = r1`, `p21 = p12`. The target's
  side is added after one sample always, after two samples and pairedproportions only two-sided.
- r(), in Stata's order: one sample: power beta alpha N [nfractional in the sample-size form] onesided
  [search constants]; then onevariance v0 delta ratio va (sample size) or v0 va delta ratio; onecorrelation
  r0 delta ra (searched sample size) or r0 ra delta; pairedproportions delta p12 p21 [prdiscordant]. Two
  samples: power beta alpha delta nfractional onesided [search constants], then as twomeans (closed: N
  [N_a] N1 N2 nratio [nratio_a] v1 v2; searched sample size: N v2 v1 [nratio_a] nratio N2 N1; searched
  target: N [N_a] N1 v2 v1 nratio N2). Macros: onevariance type method [direction] scale; twovariances
  scale type method [direction]; pairedproportions effect type method [direction]; correlations type
  method [direction].
- Refused: n() below the test's smallest group (Stata prints a missing power: 1 for variances, 3 for
  correlations), equal values, pairedproportions' corr() form (values are the marginals p1+ p+1), diff(),
  ratio() and prdiscordant() with values, effect(oratio|rrisk), and n1() or n2() alone (twocorrelations
  n1(60) alone gives N2 = N1 and prints N1 and N2, not "N per group").
- After a power line that fails in Stata, `return list` shows the earlier r() in reverse order; RCraft
  refuses the failed line, so that is not followed.
