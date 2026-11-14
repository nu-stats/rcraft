# factor (2026-11-14)

Observed on Stata 19 SE with made-up data (probes `r12/f1.do`, `r12/f2.do` in the session scratchpad, identical to
RCraft's output but for the last digits of e(chi2_i), e(evsum)); corpus `factor.do`. Translated in
`crates/statatr/src/data_r/factor.rs`.

- pf (default): the correlation matrix with the squared multiple correlations (1 − 1/[R⁻¹]ᵢᵢ) on the diagonal; pcf:
  the correlation matrix itself; ipf: from pf's communalities, the communalities again from the kept factors'
  loadings, round after round, Stata 19 stopping when Σ(change)² < 1e-8 and keeping the eigenvalues it had (found
  by matching citerate(1), (5), (50) and the default round by round); citerate(#): that many rounds.
- Factors kept: eigenvalue > mineigen() (0; pcf 1), at most factors(); each signed so its loadings sum > 0.
- `(obs=N)`, the header (Method, Retained factors, Number of params = pf − f(f−1)/2), the eigenvalues (%13.5f,
  proportions of the sum of all eigenvalues, negatives in it), `LR test: independent vs. saturated:  chi2(df) =`
  with −(N − (2p+5)/6) ln|R| on p(p−1)/2, then the loadings (%9.4f) and Uniqueness (1 − communality).
- e(): f, N, df_m, df_r ((p−f)² − (p+f))/2, chi2_i, df_i, p_i, evsum; macros (but rngstate, Stata's random-number
  state); sds, means, C, Phi (identity), L, Psi, Ev.
