# ivprobit (2026-11-17)

Ported from StatCraft's decoding of Stata 19 (its `plan/stata/ivprobit.md`, `commands/ivprobit.rs` and
`estimate/src/ivprobit.rs`, probes 450, 451 and 596), our own work, written in R (`data_r/ivprobit.rs`).
Checked on those probes and in `ivprobit.do`, recorded from Stata 19.

- One endogenous regressor. The exogenous probit (the outcome on the endogenous variable, the
  exogenous ones and the first-stage residual) starts the search: slopes over sqrt(1 + c^2 s^2),
  rho at asinh(c s), the reduced form at least squares, ln sigma at ln s (s^2 with the degrees-of-
  freedom correction). Then ml's Newton steps on the joint likelihood: halving while the likelihood
  falls, else lengthened by 1/8, 1/4, ... while it rises (the point reached plus each, StatCraft's
  probe 512; the same rule this session found for logit), modified Marquardt where -H is not positive
  definite, ml's documented stop. -H from five-point central differences of the analytic gradient.
- Both iteration logs, e(b), e(V), the table, corr() and sd() rows, the exogeneity test, Endogenous
  and Exogenous lines match probe 450 line for line; the variance to about 1e-9 on the scale of the
  standard errors. Newey's two-step estimator to the last unit or two.
- After a Stata error in an ivprobit line, Stata may hold no results (predict then gives r(301) in
  `ivprobit.do`'s first recording); RCraft marks e() as not known there.
- Not translated (see the end of this note for the run-time refusals): weights, several endogenous variables, factor variables, twostep with first or
  vce(), iterate() below the default, test/lincom/nlcom/margins/estat afterwards.

## Two gaps closed (2026-11-17, observed on Stata 19 with made-up data)

- `predict, pr` after `twostep`: Stata stops with r(198), "probabilities and average structural function
  not available with two-step estimator"; xb, stdp and the default work. RCraft stops the same way.
  `predict, asf` and `fix()` are refused (Stata answers some of those combinations).
- An instrument collinear with the other instruments (`ivprobit y x (y1 = z1 z2 z3)` with z3 = 2 z1):
  Stata prints `note: z3 omitted because of collinearity.` before the first blank line (after "Checking
  reduced-form model..." with twostep), drops the later one of the set (z1 if the list is `z3 z1 z2`), and
  fits the model without it, so the logs, table, e(ll) and chi2 are the model's without it. e(b), e(V) and
  e(gradient) keep a 0 for `y1:o.z3`; e(rank) counts the fitted ones (9), e(k) all (10); e(exog) and e(insts)
  name `o.z3` in place (`x z1 z2 o.z3`; e(insts) is also set without a collinear column). The two-step e(b)
  is the same three columns. Matched in corpus `ivprobit.do`.
- Refused when run, not guessed: a regressor collinear with another regressor, or an instrument collinear
  with a regressor (Stata then shows an `(omitted)` row, and which variable goes is not the later one: with
  `(y1 = z1 z2 x2)` and x2 = 3 x it drops x); `first` with an omitted instrument; a variable listed twice.
