# pca (2026-11-14)

Observed on Stata 19 SE with made-up data (probes `r12/p1.do`, `r12/p2.do` in the session scratchpad, identical to
RCraft's output but for the last digit of e(lndet), e(cond), e(rho)); corpus `pca.do`. Translated in
`crates/statatr/src/data_r/pca.rs`. [MV] pca: "pca returns principal components signed so that 1′v > 0".

- The rows with every variable present; the correlation matrix (`covariance`: the covariance matrix) and its
  eigen decomposition, each eigenvector's sum made positive.
- First table: `Principal components/correlation` with Number of obs, Number of comp., Trace, Rho (the kept
  components' share) at the right, `    Rotation: (unrotated = principal)`; each component's Eigenvalue,
  Difference (. for the last), Proportion, Cumulative (%.6g, %.4f), indented 4.
- Second: `Principal components (eigenvectors) ` and the kept components' loadings (%9.4f in 10, the first 9)
  with each variable's Unexplained variance (the dropped components' eigenvalue × loading², %9.4g, 0 when all
  are kept).
- e(): N, f, rho, trace, lndet, cond (√(largest/smallest eigenvalue)); macros as Stata's; matrices sds, means,
  components (Eigenvalue Difference Proportion Cumulative by Comp#), C, Psi (row Unexplained), Ev (every
  eigenvalue, columns Eigenvalues:Comp#), L (the kept eigenvectors). r() empty.
