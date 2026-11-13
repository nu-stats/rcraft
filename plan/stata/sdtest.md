# sdtest, sdtesti (2026-11-13)

Observed on Stata 19 SE with made-up data (probes `r6/e1.do`, `r6/e2.do` in the session scratchpad, set line by line
against RCraft's output: identical but for p in the 15th digit); corpus `sdtest.do`. Translated in
`crates/statatr/src/data_r/sdtest.rs`.

- `sdtest y == #`: "One-sample test of variance", the summary row (Obs, Mean, Std. err., Std. dev. and the t
  interval of the mean), chi2 = (n − 1)s²/#², df n − 1; r(): p, p_l, p_u, sd, chi2, df, N.
- `sdtest y == x` (or `=`): each variable over its own rows; `sdtest y, by(g)`: the two groups, named by their
  value labels (more than 2: r(420)); "Variance ratio test", a Combined row of every value; F = s₁²/s₂² on
  (n₁ − 1, n₂ − 1); r(): sd (the combined), p, p_l, p_u, sd_2, sd_1, F, df_2, df_1, N.
- The foot: `    ratio = sd(a) / sd(b)` with `f = %8.4f` at the right of 78, `H0: ratio = 1` with `Degrees of freedom =
  %4d, %d` (one sample: `%8d`); the three alternatives centred on columns 11, 40 and 70 (start = centre −
  ⌊length/2⌋); `  Pr(F < f) = …         2*Pr(F < f) = …           Pr(F > f) = …`, the two-sided one naming the
  smaller tail.
- `sdtesti #obs #mean #sd #val` and `sdtesti #obs1 #mean1 #sd1 #obs2 #mean2 #sd2`: the same, named x and y, the
  Combined row pooled from the summaries; a mean of `.` prints `.` with no interval.
