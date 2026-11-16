# icc (2026-11-16)

Observed on Stata 19 SE with made-up data (StatCraft probe 251 and probes `icc/i1`, `iv` in the session scratchpad:
every line identical to RCraft's, r() bounds and p-values apart past the 13th digit); corpus `icc.do`. Translated in
`crates/statatr/src/data_r/icc.rs`.

- One-way (`icc y target`): MSR between targets, MSW within (df n(k − 1)); ICC(1) = (MSR − MSW)/(MSR + (k − 1)MSW),
  ICC(k) = (MSR − MSW)/MSR; F = MSR/MSW. Two-way (`icc y target rater`): MSR, MSC (raters), MSE (df (n − 1)(k − 1)).
  Consistency: ICC(C,1) = (MSR − MSE)/(MSR + (k − 1)MSE), ICC(C,k) = (MSR − MSE)/MSR. Absolute agreement:
  ICC(A,1) = (MSR − MSE)/(MSR + (k − 1)MSE + k(MSC − MSE)/n), ICC(A,k) = (MSR − MSE)/(MSR + (MSC − MSE)/n).
- Intervals (McGraw and Wong 1996): consistency and one-way from F/F_q(n − 1, dfe) and F·F_q(dfe, n − 1);
  absolute agreement with Satterthwaite's v of (a·MSC + b·MSE), a = kρ/(n(1 − ρ)), b = 1 + kρ(n − 1)/(n(1 − ρ)) at
  ρ = ICC(A,1), its bounds n(MSR − F*MSE)/(F*(kMSC + (kn − k − n)MSE) + nMSR) and the like; the average's by
  Spearman–Brown of the individual's.
- testvalue(r0): consistency/one-way F·(1 − r0)/(1 + (k − 1)r0) and F·(1 − r0); absolute MSR/(a₀MSC + b₀MSE) and
  MSR/(c₀MSC + d₀MSE) with Satterthwaite's df (`F(14.0, 74.5)`), printed `ICC(1)=0.30:` and `ICC(k)=0.30:`;
  r0 = 0 prints one line `ICC=0.00:` with MSR/MSE.
- Defaults: random → absolute, mixed → consistency (mixed absolute gives random's numbers); the Note is printed
  only for the model's default type. One-way allows neither mixed nor consistency.
- Missing: a target not rated by all raters (two-way), or with fewer ratings (one-way), is left out with
  `(1 target omitted from computation because not rated by all raters)` / `because of unbalanced data`.
- r(): N_target, N_rater, icc_i, icc_i_F, _df1, _df2, _p, _lb, _ub, the same for icc_avg, testvalue, level, model,
  depvar, target, rater (two-way), type. Stata's F tail with fractional df is about 1e-6 of itself off R's far in
  the tail (p 1.1578738e-11 against 1.1578745e-11); the bounds' last digits follow its F quantiles.
- More than one target left out: `(2 targets omitted from computation because ...)`. One-way takes k as the
  largest number of ratings a target has, so with one target rated twice as often all the others go.
- One-way `absolute` is the default and prints the same; `mixed` (with or without a type) without a rater is an
  error. A rating with a missing score is dropped before the repeated-rating check.
- Errors, in Stata's order of checks: weights `weights not allowed` r(101); `No target variable specified`,
  `only one of consistency or absolute may be specified`, `you must specify the rater variable with
  mixed-effects models`, `consistency is not allowed with one-way random-effects models` (each r(198)); no
  observations left (before or after targets are left out) `no observations` r(2000); `multiple observations
  per target and rater not allowed` r(498); after the omitted message, one target `insufficient number of
  targets` and one rater `insufficient number of raters (observations per target)` (r(2001), targets checked
  first).
- Two targets give Stata's numbers too, an average bound above the upper one included (4.025153, .9984704).
