# centile (2026-11-13)

Observed on Stata 19 SE with made-up data (probe `cn/n1.do` in the session scratchpad, its log set line by line
against RCraft's output: identical but for the refused forms); corpus `centile.do`. Translated in
`crates/statatr/src/data_r/centile.rs`.

- The qth centile of n sorted values: at rank R = (n + 1)q/100, x(⌊R⌋) + (R − ⌊R⌋)(x(⌊R⌋+1) − x(⌊R⌋)); the
  smallest value for R < 1, the largest for R ≥ n. Each variable uses its own non-missing rows.
- The default interval ("Binom. interp."), with F the Binomial(n, q/100) cdf and γ = (100 − level)/200 for
  the lower bound, 1 − γ for the upper: the j with F(j − 1) ≤ γ < F(j), and the bound
  x(j) + (γ − F(j − 1))/(F(j) − F(j − 1))·(x(j + 1) − x(j)). Where x(j) or x(j + 1) is not in the sample
  (j < 1, or j ≥ n) the bound is the sample's minimum or maximum, the row is marked `*` after the upper
  bound, and the table ends with "* Lower (upper) confidence limit held at minimum (maximum) of sample".
  Checked by hand on n = 10, 4, 2 and in the corpus.
- The table: "Binom. interp." above at column 59; `    Variable |       Obs  Percentile    Centile`,
  the level as `[95% conf. interval]` right-aligned in 28; rule 13 + 61; Obs in 10, the percentile in 11,
  the centile in 12, the bounds in 16 and 12, all in %9.0g; a variable's later centiles on rows of their
  own with the stub and Obs blank. No observations: `           x |       0` and nothing else.
- r(): the last variable's n_cent, N, then ub_k, lb_k, c_k from the last centile down, and r(centiles)
  the list as written ("25 50 75").
- Not translated: weights, numlists with ranges.

## cci, normal, meansd (2026-11-13, from Stata's manual, [R] centile, Methods and formulas; probe `cn/n1.do` identical)

- `cci` ("Binomial exact" above the table): with F the Binomial(n, q/100) cdf, F(-1) = 0, x(0) = x(1) and
  x(n+1) = x(n): r with F(r) ≤ γ < F(r+1) and s with 1 − F(s) ≤ γ < 1 − F(s−1); the interval is
  (x(r+1), x(s+1)), marked `*` when an end is x(0) or x(n+1).
- `normal` ("-- Normal, based on observed centiles --" after 35 spaces): the centile as by default, its
  se √(q(100 − q)) / (100·√n·φ(c; x̄, s)), s the sample sd, and c ± z·se.
- `meansd` ("-- Normal, based on mean and std. dev.--"): the centile x̄ + z_q·s, its se
  s·√(1/n + z_q²/(2(n − 1))), and ± z·se.
