# logistic (2026-11-19)

Observed on Stata 19 SE (corpus `logistic.do`, 24 results to 1e-6, the tolerance of the other likelihood models).
Translated in `crates/statatr/src/data_r/lsfam.rs`.

- `logistic y x...` is `logit y x..., or` without the iteration log, then the same header (Logistic regression), the
  `Odds ratio` table, and the line `Note: _cons estimates baseline odds.`; `coef` shows the coefficients (no note);
  `level()`, `vce()`, `robust`, `if`, `in` and weights as logit's. e() is logit's, with e(cmd) `logistic`, e(predict)
  `logistic_p`, e(cmdline) as typed.
- `noconstant` (2026-11-20, corpus `ml_cns.do`): logit's, as below; `constraints()` too (`plan/stata/constraint.md`).

## logit and probit with noconstant (2026-11-20)

Observed on Stata 19 SE (corpus `ml_cns.do`, the printed output line for line). Translated through the constrained fit
with no constraints (`stata_ml_cns` in `crates/statatr/src/data_r/ml.rs`).

- The log starts with every coefficient 0 (iteration 0 is n ln(1/2) unweighted) and follows Stata's Newton-Raphson.
- The header is the constrained one: `Wald chi2(k)` (all the slopes), `Log likelihood = ` beside `Prob > chi2`, no
  pseudo R2; then the table. e(): no e(ll_0) and no e(r2_p), chi2type `Wald`, chi2 and p that Wald test.
- Not changed: logit and probit with noconstant and an offset or exposure (the old path), poisson with noconstant.
