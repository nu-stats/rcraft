# logistic (2026-11-19)

Observed on Stata 19 SE (corpus `logistic.do`, 24 results to 1e-6, the tolerance of the other likelihood models).
Translated in `crates/statatr/src/data_r/lsfam.rs`.

- `logistic y x...` is `logit y x..., or` without the iteration log, then the same header (Logistic regression), the
  `Odds ratio` table, and the line `Note: _cons estimates baseline odds.`; `coef` shows the coefficients (no note);
  `level()`, `vce()`, `robust`, `if`, `in` and weights as logit's. e() is logit's, with e(cmd) `logistic`, e(predict)
  `logistic_p`, e(cmdline) as typed.
- `logistic, noconstant` leaves e(ll_0) and e(r2_p) missing and a Wald chi2 in e(chi2): refused (a gap of
  `logit, noconstant` too).
