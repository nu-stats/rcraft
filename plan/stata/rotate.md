# rotate (2026-11-15)

Observed on Stata 19 SE with made-up data (StatCraft probe 252 and probes `rot/r1`–`r4`, `v1`, `v2` in the
session scratchpad: identical to RCraft's output line for line); corpus `rotate.do`. Translated in
`crates/statatr/src/data_r/rotate.rs`. Every rule below was found by matching Stata's e(r_T) to 1e-15 on several
data sets (pf, pcf and ipf factors, pca, 2 and 3 factors, float and double data).

- varimax (the default) rotates factor's loadings (pca's eigenvectors) from T = I:
  - two factors: Kaiser's angle, φ = atan2(D − 2AB/p, C − (A² − B²)/p)/4 with u = a² − b², v = 2ab, A = Σu, B = Σv,
    C = Σ(u² − v²), D = 2Σuv: the optimum itself;
  - three or more: Kaiser's SVD iteration, T = UV′ from the SVD of A′(Z³ − Z diag(Σᵢz²/p)), Z = AT, stopped once
    the sum of the singular values grows by less than 1e-8 of itself (so Stata's T is not the exact optimum: it
    sits about 1e-6 short of it).
- quartimax and equamax: the orthogonal gradient projection algorithm on the orthomax criterion
  −¼Σλ⁴ + γ/(4p)Σⱼ(Σᵢλ²)² (γ = 0; equamax γ = p/2, p the number of variables, not f/2), step 1 doubled each round
  and halved until the criterion falls by half the squared projected gradient times the step; stopped after the
  round in which T changes by less than 1e-6 (mreldif) and the new projected gradient is below 1e-6.
- promax(k) (k = 3; at least 1, else `promax() should be at least 1`, r(198)): from the varimax W, target
  P = B∘|B|^(k−1) of the varimax loadings B, U = (B′B)⁻¹B′P scaled so diag((U′U)⁻¹) = 1, W ← WU. Under normalize
  the target comes from the normalized loadings and the regression from the raw ones.
- normalize (or horst): Kaiser's normalization, rows divided by √(communality) for the rotation.
- Then each factor is signed so its loadings sum to more than 0 and the factors are ordered by variance, largest
  first.
- After factor: e(r_L) = LW, e(r_T) = (W⁻¹)′ (W when orthogonal), e(r_Phi) = T′T, e(r_Ev) the variances, for an
  oblique rotation the column sums of the squared structure loadings L_r Φ. After pca: e(r_L) = VW (the
  eigenvectors), e(r_T) = W, e(r_Ev) = diag(W′ΛW), no e(r_Phi).
- e() adds r_f (after the scalars), r_normalization (`none`, `kaiser`), r_class, r_criterion, r_ctitle (`varimax`,
  `promax(3)`, `promax(2.5)`) before the macros (pca orders them r_normalization, r_ctitle, r_criterion, r_class)
  and r_Ev, r_Phi, r_T, r_L before the matrices; `rotate, clear` removes them, printing nothing. r(rtext) is
  `" orthogonal varimax (Kaiser off)"` (a leading space).
- The tables: factor's (or pca's) header with `Rotation: <rtext>`; the variances as factor's eigenvalues
  (`Variance` for `Eigenvalue`; oblique: Variance and Proportion only, `Rotated factors are correlated`, pca
  `Rotated comp. are correlated`); `Rotated factor loadings (pattern matrix) and unique variances` (pca `Rotated
  components `); `Factor rotation matrix` (%7.4f in 9 columns; pca `Component rotation matrix`, %9.4f in 10).
  blanks(#) blanks |loading| < # and adds `    (blanks represent abs(loading)<.3)` under factor's table (pca:
  `  (blanks are abs(loading)<.3)` on its title line).
- Two varimax factors that come out reflected (a sign turned or the two swapped) make T exactly symmetric, and
  factor's rotation-matrix table then shows only its lower triangle (seen 10 times); pca's and quartimax's show the
  whole matrix.
- Errors: `varlist not allowed`, `if not allowed` (r(101)); `no rotation command found in e(rotate_cmd)` after
  another model (r(198)); `multiple rotation criteria specified` (r(198)); `promax is always oblique` (r(198));
  `the varimax criterion is not suitable for oblique rotations` (oblique alone too, r(496)).
- Refused: the other criteria (oblimin, quartimin, parsimax, ...), factors()/components() (a partial rotation,
  whose off-block zeros Stata prints as −0.0000 by its own rounding), the maximize options, random starts,
  blanks() below 0 (Stata prints the header and variances before its r(125)), `in`, combinations whose message was
  not seen, rotate after pca with every component kept (Stata's answer there is its own rounding: variances 1,
  differences 1e-7), e(r_fmin) (what quartimax and equamax minimized, on a scale not matched: −1.706 where the
  criterion above gives 0.278; Stata keeps it through later rotations), matrix list e(r_Phi) after an orthogonal
  rotation (Stata's off-diagonal 0s are 1e-16 noise) and estat rotatecompare. `display` of an e(r_*) after
  `rotate, clear` shows nothing in Stata, "." in RCraft (a general difference, not rotate's).
