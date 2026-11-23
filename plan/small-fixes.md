# Small fixes list

Small differences from Stata (or rough edges) found while building, each too small to hold up its
task. Each is a candidate follow-up: add a line when you find one, strike it through when fixed.
The owner asked for this list on 2026-11-23.

| # | Where | What differs | Size | Fix | Status |
|---|---|---|---|---|---|
| 1 | Factor analysis (P6.7) | `factanal`'s likelihood-ratio chi-square multiplies by N-1; Stata multiplies by N (6.74 against 6.93 on the test data, the same as 36.5 against 35.5 in the multiplier). Loadings and uniquenesses are not affected | One printed number and its p-value | Recompute the statistic and p-value in the dialog's code with N, then match Stata's line | open |
| 2 | Factor analysis (P6.7) | `factor, ml` stops about 3e-5 short of the maximum at its default tolerance; R goes to the maximum | Loadings agree to 1e-4 (1e-6 with Stata's tolerance tightened) | None wanted unless the owner asks for 8 digits against Stata's default stop; would mean copying its stopping rule from observation | open, probably leave |
| 3 | Factor analysis (P6.7) | `stats::varimax` stops about 5e-9 short of the exact two-factor rotation | 5e-9 | Own rotation loop to a tighter tolerance | open, probably leave |
| 4 | Principal components (P6.7) | Stata signs each component so its elements sum to more than 0; the dialog makes the largest loading positive (the owner's order said so) | Sign only | Add a "sign as Stata" choice if wanted | open |
| 5 | Discriminant analysis (P6.7) | `MASS::lda` weighs the group means by the priors; Stata by the group sizes, so the canonical functions match Stata only with proportional priors (the classification table matches with either) | Canonical functions with equal priors | Compute the canonical functions in the dialog code with the group sizes | open |
| 6 | Panel unit-root test, Im-Pesaran-Shin (P6.1) | W-t-bar is standardised with plm's moment table; Stata's differs: -2.8824 against -2.9180 (one lag, a mean), and the same size on other settings. The t-bar itself equals Stata's (with plm's degrees-of-freedom correction). Levin-Lin-Chu matches exactly | The statistic and p-value in the second decimal | Own table of the mean and variance of the Dickey-Fuller t for each T and lag order, read off Stata's output on made-up data by solving for them, or taken from the published Table 3 of Im, Pesaran and Shin | open |
| 7 | Dynamic panel model (P6.1) | After one step plm's conventional standard errors are unscaled and its Sargan statistic differs from Stata's (11.67 against 10.56), so the dialog offers robust errors only after one step and prints no Sargan test | Two options | Compute Stata's one-step scaling (sigma^2) and Sargan in the dialog code | open |
| 8 | Dynamic panel model (P6.1) | No time effects (plm `twoways` is not matched to Stata: `xtabond` with year dummies refused the two-step estimator on the test data), no system GMM, no limit on instrument lags, no constant | Missing options | Probe `xtabond` with year dummies and `maxldep`, `xtdpdsys`, and extend the formula | open |
| 10 | Panel instrumental variables (P6.1) | Conventional errors only; the within fit reports no constant where `xtivreg, fe` does; no EC2SLS or first-difference estimators | Missing options | Add `vcov` choices and the other estimators | open |
