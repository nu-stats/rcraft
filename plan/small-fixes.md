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
