# mprobit, slogit, nlogitgen, nlogittree, nlogit (batch MV, 2026-11-22)

Ported from statcraft@b7d392e (its `models.md`, probes 534, 536, 539 and 540), then checked against Stata 19 on made-up
data: corpus `mprobit.do`, `slogit.do` and `nlogit.do`. Translated in `crates/statatr/src/data_r/multi/choice_cmds.rs`,
`multi/nlogit_cmds.rs` and the R in `multi/choice.R`, `multi/nlogit.R`.

All three maximize the log likelihood by Newton's method on numerical derivatives (4th-order differences of the log
likelihood, the Hessian from the gradient's), so they stop at the optimum; Stata stops where its own iteration does, which
differs in the 7th or 8th digit (`technique(nr)`) or the 5th (the default `bfgs` of nlogit). Iteration logs, `e(ic)`,
`e(gradient)`, `e(ilog)` and `e(Cns)` are not reproduced.

## mprobit

- `mprobit y x [, baseoutcome(#) probitparam nolog]`. P(y = j) is the integral over a standard normal z of the product over
  m != j of Phi(z + eta_j - eta_m), the base outcome (the most frequent, or `baseoutcome()`) with zero coefficients. Stata
  reports 15 points, but its e(ll) is the exact integral to 3e-10, so 64 Gauss-Hermite nodes are used. Wald chi2 of every slope.
- `probitparam`: coefficients divided by sqrt(2) and the variance by sqrt(2) (not 2), as observed.
- `predict p1 p2 p3` (one variable per outcome) or `predict p, outcome(#)`; `xb` and the other options are not translated.
- Not translated: weights, factor variables, `vce()`, `constraints()`, `intpoints()`, `noconstant`.

## slogit

- `slogit y x [, dimension(#) baseoutcome(#) nolog]`: eta_j = theta_j - sum_d phi_dj x'beta_d, the base outcome (the highest, or
  `baseoutcome()`) with eta 0, and the first D other outcomes with phi_dj = 1 for d = their place and 0 otherwise. The start is
  the multinomial logit reduced to D dimensions by its singular value decomposition. e(b) names are `dim#:x`, `phi#_j:_cons`,
  `theta#:_cons`, the constrained ones included. e(ll_0) is the log likelihood of the outcomes' frequencies.
- `dimension()` above min(outcomes - 1, regressors) stops with r(198) in Stata's words.
- `predict p1 p2 p3 p4` or `predict v, outcome(#)`, and `predict v, xb` (outcome 1's index, or `outcome()`).

## nlogitgen, nlogittree, nlogit

- `nlogitgen type = alt(ground: car | bus, other: train | plane)` makes a numeric variable with a value label from the
  alternatives' value labels; `nlogittree alt type, choice(choice)` prints the tree.
- `nlogit depvar vars || nest: [vars], base(group) || alt:, case(id) [noconstant technique(nr|bfgs) nolog]`: a two-level
  RUM-consistent model, P(group) = exp(Y_g + tau_g I_g) / sum, P(j | g) = exp(V_j / tau_g - I_g). e(b): `alt:var`,
  `group:var` for the non-base groups, `alt:_cons` constants (all but the first), then `/nest:group_tau`. Wald chi2 over the
  variables only; the LR test of tau = 1 (e(ll_c), e(chi2_c), e(p_c)) refits with the taus fixed at 1.
- `predict q*` (or two names): the probability of the group (q1) and of the alternative (q2).
- Not translated: more than two levels, variables at the alternative level beyond the first equation, type-level variables
  together with alternative constants (the order of e(b) is not known), `nonnormalized` and the other options, estat.
