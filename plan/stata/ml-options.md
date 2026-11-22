# iterate(#) on logit, probit and poisson (2026-11-22)

Observed on Stata 19 SE with made-up data (corpus `ml_iterate.do`, 93 results to 1e-6, and the printed output compared
with Stata's log line for line). Translated in `crates/statatr/src/data_r/ml.rs` (`stata_ml_cap_fit`, the `stata_maxit`
flag read by `stata_ml_iterations` and `stata_ml_count`).

- Stata's Newton-Raphson (`stata_ml_path`) stops after # steps: iteration lines 0 to # are printed, then
  `convergence not achieved` straight under them (also with `nolog`, where only that line is printed). A path that
  converges inside the cap (poisson iterate(1)) prints as usual, with no message.
- The model is the one at the point where the path stopped: e(b), e(V) (the observed information there; robust too),
  e(ll), e(ll_0) (the constant-only maximum), the LR or Wald chi2. e(converged) is 0, e(rc) 0; e(ic) is not kept (as before).
- After the table: `Warning: Convergence not achieved.` with no blank line.
- iterate(0): logit and probit start at the constant-only fit, so the likelihood equals e(ll_0) and the LR test is 0 (Stata
  shows `-0.00` and `-0.0000` when its two likelihoods differ in the last bit: data dependent, RCraft prints 0.00; on
  other data sets Stata prints 0.00 too). poisson prints its single line with **no blank line above it**.
- logit, probit and poisson only; with weights (poisson), noconstant, an offset, constraints() or a path that halves a step the
  fit stops with a message.

## Open

Not done: `from()` (probe 508: matrices from earlier fits, copy, skip, the name-matching errors), `trace`, `showstep`, and
`iterate()` on nbreg, ologit, oprobit and mlogit.
