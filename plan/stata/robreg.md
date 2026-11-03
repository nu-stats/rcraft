# robreg m (SSC), as it behaves (P2C.9)

Installed with the user's go-ahead (`ssc install moremata`, `ssc install robreg`, 2026-11-03) and run on
Stata 19 SE with made-up data. Only its output and e() were read, never its code. Values below were
reproduced in R before the translation was written.

## What it prints

- `obtaining LAD starting values ... done`, `iterating RLS ..... done`, `fitting empty model ... done`,
  `computing standard errors ... done`, then a header: `M regression (95% efficiency)`, Number of obs,
  Wald chi2(2), Prob > chi2, Pseudo R2, Huber k (`1.3449975`), Scale; then the table with `Robust`
  standard errors and t tests.
- e(): `N`, `df_m`, `df_r` (n - p), `scale`, `scale0`, `k` (1.34499751), `efficiency` (95), `r2_w`, `r2_p`,
  `chi2`, `p`, `rank`, `k_omit`, `converged`, `iterations`; macros `cmd` robreg, `subcmd` m, `obf` huber,
  `vce` robust, `vcetype` Robust, `title`, `depvar`, `indepvars`, `predict` `robreg predict`; matrices `b`,
  `V`, `b_init`, `b0`, `V_modelbased`, `omit`.

## What its numbers are (checked against them)

- `e(b_init)` is a least-absolute-deviations fit (the same as `qreg`'s when that is unique).
- `e(scale)` is the median of the absolute LAD residuals with the p smallest (the fit's zero residuals)
  left out, over qnorm(0.75): 1.1119512 here, where the median of all of them gives 0.6178.
- With that scale fixed, Huber's M estimate (k = 1.3449975, 95% efficiency) by reweighted least squares
  gives e(b) to the last digit.
- e(V) is the sandwich s^2 A^-1 B A^-1 (A = X' diag(psi') X, B = X' diag(psi^2) X) times n / (n - p).
- The empty model starts at the median of y (the two middle values averaged when n is even) and its scale
  is the same rule with one parameter: e(scale0). e(b0) is its M estimate of location.
- e(r2_p) is 1 - sum rho(r / s) / sum rho(r0 / s) with Huber's rho and the model's scale; e(r2_w) is the
  weighted R-squared of the final fit.
- `predict` gives xb by default, `residuals`, and `weights` (min(1, k / |r / s|)).
- `efficiency(90)` gives k = 0.98180232.

## When the LAD fit is not unique

- With an indicator variable (`i.g`), robreg's `e(b_init)` was not `qreg`'s: `2.g` was 0.3187 where qreg
  gave 0.2273, with only 3 zero residuals for 4 coefficients. It starts from a point inside the LAD
  solutions, as an interior-point solver would, not the median of that coefficient's interval (0.4197).
