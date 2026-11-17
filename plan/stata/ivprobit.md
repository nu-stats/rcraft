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
- Not translated: weights, several endogenous variables, factor variables, twostep with first or
  vce(), iterate() below the default, test/lincom/nlcom/margins/estat afterwards.
