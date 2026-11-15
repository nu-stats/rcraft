# mvtest (2026-11-15)

Observed on Stata 19 SE with made-up data (StatCraft's probe 531 and probes `r18/m1.do` to `r18/m7.do` in the
session scratchpad, set line by line against RCraft's output, `return list` and `matrix list` included: identical
but for r() digits past the 13th); corpus `mvtest.do`. Translated in `crates/statatr/src/data_r/mvtest.rs`.

- Rows: any of the variables missing leaves the row out; `missing` (with by()) makes the missing by() values a
  group of their own, after the others. Subcommands from `m`, `cov`, `corr`, `norm`; options at Stata's shortest
  (`e`, `z`, `het`, `lr`, `diag`, `sph`, `comp`, `biv`, `uni`, `st`, `miss`, `prot`). Any other option, or one the
  form does not take (`zero` with by(), `lr` without, a hypothesis with by()), is Stata's `option x not allowed`,
  r(198); `mvtest bogus` is `unknown subcommand of mvtest: bogus`, r(198).
- means: T² = n·(Cx̄)′(CSC′)⁻¹(Cx̄) with the contrasts x_j − x_1 (zero: x̄′S⁻¹x̄), F = (n − q)/(q(n − 1))·T², q the
  contrasts' rank (collinear variables lose one, the inverse dropping a column in order, as invsym() does;
  with fewer rows than contrasts, which ones Stata keeps was not worked out, so that is refused). by(): the four
  MANOVA statistics from the eigenvalues of W⁻¹B: Wilks with Rao's F (exact when min(p, k − 1) ≤ 2), Pillai,
  Lawley–Hotelling (F with 2(s·n + 1) denominator df), Roy's upper bound; r(stat_m) holds them. lr: the common
  mean by maximum likelihood (Σᵢ = Sᵢ + (x̄ᵢ − μ)(x̄ᵢ − μ)′, iterated), LR = Σ nᵢ log(|Σᵢ|/|Sᵢ|); r(M) is μ,
  about 1e-10 from Stata's (Stata stops sooner), r(rc) 0.
- covariances by(): Box's M with c₁, c₂, the F's df₂ = (df₁ + 2)/(c₂ − c₁²) (printed `%9.1f`, a trailing `.0`
  dropped), chi2 = M(1 − c₁); one sample: −(n − 1 − (2p + 5)/6) log|R| (diagonal), Bartlett's sphericity
  test, Box's (1950) compound-symmetry test.
- correlations: Lawley's (1963) test; by(): Jennrich's (1970) with nᵢ weights.
- normality: Doornik–Hansen on the data rotated by the correlation matrix's inverse root (D'Agostino's z₁,
  the gamma z₂; r(U_dh) holds b1 = √b₁, b2, z1, z2 by row); Henze–Zirkler with the ML covariance, its lognormal
  E and V, chi2(1) = z²; Mardia's skewness with the small-sample factor (p + 1)(n + 1)(n + 3)/(n((n + 1)(p + 1) − 6)),
  kurtosis z = (b₂ − p(p + 2))/√(8p(p + 2)/n). stats(): dhansen, hzirkler, skewness, kurtosis from two letters,
  all; printed skewness, kurtosis, HZ, DH; r() in Stata's order (dh, hz, kurtosis, skewness). univariate is
  sktest's numbers (r(Utest)); bivariate the DH test of each pair (r(Btest)).
- A singular matrix is Stata's r(506): "sample covariance matrix is singular" (covariances, normality),
  "sample correlation matrix is singular", "collinearity among the dependent variables" (means by()),
  "singular covariance matrix" (lr).
- Refused: `heterogeneous` (for more than two groups Stata 19 takes the p-value of James' (1954) approximation
  from its own search: Stata's p-values sit 3 to 4e-6 from the root of χ²(A + Bχ²) = W in every case tried,
  either side; two groups print the MNV F, matched, but which form applies is known only from the data),
  `protect()`, weights, `missing` without by(), and `matrix list r(Btest)` (its rows are eq:name pairs).
