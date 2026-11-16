# tetrachoric (2026-11-16)

Observed on Stata 19 SE with made-up data (StatCraft probe 251 and probes `tet/t1`, `tv` in the session scratchpad:
identical to RCraft's output but r(rho) and r(se_rho) in the 16th digit); corpus `tetrachoric.do`. Translated in
`crates/statatr/src/data_r/tetrachoric.rs`.

- Each pair's 2×2 table (frequency weights count rows): thresholds h = Φ⁻¹(P(x = 0)), k = Φ⁻¹(P(y = 0)) from the
  margins, rho the root of the likelihood's score φ₂(h, k, ρ)(n₀₀/p₀₀ − n₀₁/p₀₁ − n₁₀/p₁₀ + n₁₁/p₁₁) (method
  `ml`), the bivariate normal by Sheppard's integral over the angle. Its SE is the (3, 3) element of the inverse
  information of (h, k, ρ), n Σ ∇p∇p′/p over the cells (Stata's full ML; the one-parameter information gives a
  different SE). Stata's own iteration leaves the 16th digit of rho and its SE apart from any exact root (two
  integration schemes put the root ±5e-16 around Stata's).
- edwards: a = n₀₀n₁₁/(n₀₁n₁₀), rho = (a^(π/4) − 1)/(a^(π/4) + 1), SE = (π/4)·2a^(π/4)/(a^(π/4)+1)² ·
  √(Σ1/nᵢⱼ); `r(method)` is `Edwards`.
- The p-value is Fisher's exact test (`2-sided exact P`); bonferroni and sidak adjust it by the number of pairs.
- Two variables: the report (`%16s =%13s`), the test line; more (or matrix): `(obs=N)` (the first pair's N, also
  under pw) unless stats() has obs, the Key box when stats() has more than rho (rho, Std. error, 2-sided exact P,
  Number of obs, in that order), and pwcorr's matrix (8-wide cells after a mark, * for star(), print() blanks;
  the diagonal's se is 0.0000, its p blank, then a blank row). notable prints nothing.
- r(): p, se_rho, nneg (negative eigenvalues of Rho), rho, N of the first pair (row 2, column 1), method, and
  P, Se_Rho, Rho, Nobs. Listwise by default, pw pairwise.
- Errors: `variable x is not 0/1 coded` (r(198)), `too few variables specified` (r(102)).
- Refused: an empty cell (Stata gives ±1 with SE `.`; zeroadjust), rho within .001 of ±1, posdef when the matrix
  is not positive definite, weights other than a frequency-weight variable.
