# SPost13 (listcoef, fitstat, mtable, mchange, mgen): on hold (2026-11-22)

J. Scott Long and Jeremy Freese's SPost13 (`net install spost13_ado, from(https://spost.su.domains/)`) was installed in
Stata 19 with the user's OK on 2026-11-22; no .ado was read. Not translated yet: the probe (StatCraft's 158, statcraft@b7d392e)
runs `listcoef`, `fitstat`, `mtable`, `mchange` and `mgen` after `logit`, `ologit`, `mlogit` and `poisson`.

What each needs, from StatCraft's note of what it matches:

- `listcoef`: b, z, P>|z|, e^b, e^bStdX (or the percent changes) and SDofX per variable; the header text and the columns
  depend on the model (odds of 1 vs 0; odds of >m vs <=m for ologit; one block per outcome pair for mlogit; factor change in
  the expected count for poisson).
- `fitstat`: one column of log-likelihoods, deviance, LR, the pseudo R-squareds (McFadden, adjusted, McKelvey and Zavoina,
  Cox-Snell, Cragg-Uhler, Efron, Tjur, count and adjusted count), AIC, BIC and the variance of e and y-star; the rows differ
  for ologit and poisson.
- `mtable`, `mchange`, `mgen`: predicted probabilities and discrete or marginal changes, with p values and confidence limits
  by the delta method, over `at()` values and at the means; they need `margins`-style machinery for each of the four models.

On hold, to be built next (the delta-method code is the same as `margins`'s); each piece is to be checked against Stata 19
with the package installed (it is in PLUS now).
