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

## Lists of values: Stata's table (2026-11-16)

Observed on Stata 19 SE with no data (probes t1 to t4, f1 to f3: about 45 tables set line by line against
RCraft's, every line identical but Stata's failed computations below); corpus `power3.do`.

- Any option or value given as a list (`(0.4 0.5)`, `(0.4(0.1)0.6)`, `n(10(10)30)`), or `table`, prints a
  table instead of the study parameters: the head lines ("Performing iteration ..." when a row searched),
  then `  +---+` rules around `|` rows, each column 8 wide (or its label + 1: `prdiscordant` is 13),
  numbers `%7.4g`, N, N1 and N2 `%7.0gc` (whose commas are left out when they do not fit: 50000 in 7 is
  `50000`, a stata_fmt fix that holds for every %gc).
- Rows: every combination, outer to inner: the values (first, then second) > alpha > power > n > n1 > n2 >
  nratio > the method's own numbers (sd > corr, sd1 > sd2); the last varies fastest.
- Columns: alpha power N [N1 N2] [N2/N1 when nratio() is given, even 1] delta, then onemean m0 ma sd;
  twomeans m1 m2 sd (sd1 sd2 when unequal); pairedmeans d0 da ma1 ma2 sd_d [sd corr]; oneproportion p0
  pa; twoproportions p1 p2; onevariance v0 va (s0 sa); twovariances v1 v2 (s1 s2); correlations r0 ra,
  r1 r2; pairedproportions p12 p21 [prdiscordant].
- `   Note: N1 and N2 have been rounded down.` under the table when a total did not split.
- r(): the last row's, with r(separator) = r(divider) = 0, macros r(columns) (nratio for N2/N1),
  r(labels) (each quoted), r(widths), r(formats) after method/direction, and the matrix r(pss_table)
  (rows r1, r2, ...). Where separator and divider fall in `return list` varies even between one-sample
  forms (after alpha for onemean's power, at the end for its sample size; split for knownsd one-sided):
  RCraft refuses `return list` after a table, every r() can be read.
- `notable` prints the head only; `parallel`, `table()` with columns and `graph` are refused.
- At n(2000000) with a delta of 1.5 or more Stata's t test fails ("." and "Warning: Some of the
  computations failed"); R works the power out (1). Not followed.

A total that does not split at nratio() (found on the way, true of the single form too): N1 = ⌊n/(1 +
nratio)⌋, N2 = ⌊n·nratio/(1 + nratio)⌋ (n(50) nratio(2): 16 and 33, not 2·16), "Actual sample sizes"
adds `N2/N1` when the groups miss nratio(), and r(nratio_a) is kept in every two-sample form whose groups
miss nratio(): after r(nratio) where Stata does not search, before it where it does.

## oneway (2026-11-17)

Observed on Stata 19 SE with no data (probes o1, o2 and the oneway lines of c2: 25 power oneway lines,
every printed line identical to RCraft's; r() values equal to 12 digits or more); corpus `power4.do`.

- `power oneway m1 ... mk` or `ngroups(k) varmeans(Var_m)`, with varerror() (1), n(), npergroup(),
  n1() to nk(), grweights() (sample size only), alpha(), power(). Var_m = the means' variance about
  their weighted mean, the weights summing to 1 (weights: n#(), grweights(), else equal); delta =
  √(Var_m/Var_e).
- The F test on k − 1 and N − k df with noncentrality N·Var_m/Var_e. R's pf() with ncp agrees with
  Stata's only to about 1e-9, so the noncentral F is summed (Poisson weights of pbeta) to full
  precision; r(beta) is 1 − r(power), as Stata's (it shows at 1e-14).
- Sample size: r(N_a) is the fractional total where the power is reached; balanced groups get
  ⌈N_a/k⌉ each, weighted ones ⌈N_a·w_i/Σw⌉ (grweights(1 2 1): 22.55 gives 6, 12, 6). "Performing
  iteration ..." is printed when the means or grweights() are given, not with varmeans() alone (though
  N_a is fractional there too); the between-group variance (n() or npergroup() and power(), no means) is
  searched.
- n() that does not split into k groups: ⌊n/k⌋ each, "Actual sample sizes", r(N) the total given and
  r(N_a) the groups'. Unequal groups print `    Average N =` (%9.4f) and N1 ... Nk.
- r(): Var_m Var_e [grwgt#] balanced, N_per_group or N1 ... Nk, N_avg N_g N_a [m#] power beta alpha
  delta [search constants] onesided nfractional N; macros type method. Stata's `return list` puts power,
  beta, alpha, delta, onesided, nfractional, N in an order that changes with the number of groups and the
  form (with 5 means: delta onesided nfractional N alpha beta power), like a hash table's: RCraft keeps
  every value but refuses `return list` after power oneway.
- Reading r(beta) after any power line had been refused as regress's matrix e(beta); fixed.
- Refused: lists of values (Stata's table) for oneway, more than 20 groups, grweights() with n() or
  npergroup(), contrasts and the other options.

## logrank and cox (2026-11-17)

Observed on Stata 19 SE with no data (probes s1 to s5: about 45 lines with return list or every r()
displayed, set against RCraft's: every printed line identical, r() to 12 digits or more); corpus
`power5.do`.

- `power logrank [s1 [s2]]`, hratio() (s2 = s1^hratio; with s1 and s2, hratio = ln s2 / ln s1),
  schoenfeld, n(), n1() n2(), nratio(), onesided, direction(): the number of events E by Freedman's
  method, (z_α + z_β)²(1 + R·HR)²/(R(1 − HR)²), or Schoenfeld's, (z_α + z_β)²(1 + R)²/(R·ln²HR); the
  probability of an event Pr_E = 1 − (N1·s1 + N2·s2)/N (1 without s1: no censoring; r(Pr_w) = 0).
- `power cox [b1]`, hratio() (b1 = ln hratio), sd() (0.5), r2(), eventprob() (1): Hsieh and Lavori,
  E = (z_α + z_β)²/(sd²(1 − R2)·b1²).
- Power: Stata's is the one tail on the effect's side, Φ(effect·√E − z) (both tails would give 0.4291 for
  0.4290 at n(300)), with E = N·Pr_E as it is (n(200) with Pr_E 0.4201: 84.02); r(E) is that rounded up.
- Sample size: E = ⌈E_frac⌉ (with no censoring E is the N below); N_frac = E_frac/Pr_E; logrank's
  groups ⌈N_frac/(1 + R)⌉ and ⌈R·N_frac/(1 + R)⌉ (293 from 292.12 at nratio(1.5) is 117 and 176), with
  N2/N1 printed under them when they miss nratio(), and r(nratio_a); cox's N = ⌈N_frac⌉ (eventprob(0.4):
  E 121, N 301).
- Targets (n() and power(), no effect): direction() lower by default (HR < 1, b1 < 0). Closed forms but
  logrank with s1 (s2 and Pr_E follow the hazard ratio: searched, "Performing iteration ..."). With a total
  that does not split, the target is worked out from the total given (n(401) nratio(1.5): groups 160 and
  240, E from 401·Pr_E) but r(E) from the groups.
- Printout: "Estimated sample sizes for two-sample comparison of survivor functions" / "Estimated sample
  size for Cox PH regression"; tests "Log-rank test, Freedman method" (Schoenfeld method), "Wald test"; H0
  `HR = 1`, `ln(HR) = 0`, `beta1 = 0`, the target's side only two-sided; blocks "Censoring:" (sample
  size) or "Number of events and censoring:" with E, s1, s2, Pr_E; delta labelled (hazard ratio), (log
  hazard-ratio), (coefficient); Schoenfeld's target ends with `   ln(hratio) =`.
- Every power method prints whole numbers %9.0fc (`N =     3,142`): fixed for all of them.
- r(): logrank power beta alpha delta nfractional onesided [search] N [N_a] N1 N2 nratio [nratio_a]
  hratio E, then s1 Pr_w Pr_E s2 (sample size) or s1 s2 Pr_w Pr_E, or Pr_E Pr_w without s1; the searched
  target N [N_a] N1 Pr_w Pr_E s2 s1 E hratio nratio N2. cox power beta alpha delta N [nfractional]
  onesided E [hratio] b1 sd R2 Pr_E Pr_w. Macros type method [direction] test effect (logrank: Freedman
  hratio, Schoenfeld lnhratio), effect coefficient (cox).
- Not followed: after cox with b1 given or worked out, Stata keeps r(hratio) = exp(b1) hidden from
  `return list`; RCraft has no r(hratio) then. Refused: lists, wdprob(), simulation and the other
  options, n1() or n2() alone.

## One group's size (2026-11-17)

Observed on Stata 19 SE (probes m1, m2); corpus `power6.do`.

- n1() alone: N2 = nratio·N1 (nratio 1 by default); n2() alone: N1 = N2/nratio; n() with n1() or n2():
  the other group is the rest (r(nratio) their ratio). Every two-sample method, power and target forms.
- Whenever n1() or n2() is given Stata lists `N1 =` and `N2 =`, even equal (never `N per group`): fixed
  for n1() n2() given together too.
- n(), n1() and n2() together, or nratio() with both, are Stata errors (r(198)): refused. Refused too: n()
  with nratio() and one group, lists in these options, and a group that is not whole (n2(41)
  nratio(2): Stata prints only N2 and N2/N1 among the study parameters, then actual sample sizes 20 and
  41; not followed yet).
- logrank's Pr_E is 1 − (s1 + R·s2)/(1 + R), R = N2/N1, in that order: Stata's doubles then give
  0.55000000000000004 for 0.5 and 0.4, and E = ⌈200 × that⌉ is 111 (110.00000000000001), not 110. E is
  rounded up with no tolerance.

## The two left over (2026-11-17)

- After `power cox` with b1 given or worked out, Stata 19 sets r(hratio) = exp(b1) but leaves it out of
  `return list` (`display r(hratio)` after `power cox -0.4, ...` is .670320046035639). RCraft keeps it the
  same way: readable, not listed (an attribute of stata_r that stata_rget reads).
- Stata's noncentral t takes noncentralities up to 1000 (√n·d = 999 gives a power, 1001 a missing one).
  Past that: two means stop with r(498) ("failure to compute power; ..."), and in a table their row's
  power is "." with "   Warning: Some of the computations failed; execute each computation separately to
  see the specific error." under it; one mean (and paired means) show the power as "." with no error
  and no warning. RCraft does the same. And where R's noncentral t passes 1 at large n (1 + 5e-12),
  the power is 1, as Stata's.
