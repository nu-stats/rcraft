# svyset and svy: clogit (2026-11-06)

YFBPT6's weighted analysis: `svyset id_g [pweight=weight_rake]`, `svy: clogit chosen i.actor i.reform
i.opinion, group(csid) baselevels`, `predict pc1`, `predict pu0, pu0`, `margins, dydx(*) post
predict(pu0) asobserved vce(unconditional) mcomp(bonferroni)`, parmest, coefplot. Observed in Stata 19
with made-up data shaped like it (`svy.do` keeps the checks).

## svyset

`svyset psu [pweight=w]` prints the design (Sampling weights, VCE linearized, Subpop. df off, Single
unit missing, Strata 1 <one>, Sampling unit 1, FPC 1 <zero>). Strata, stages and fpc are declined.

## svy: clogit

- "(running clogit on estimation sample)", then clogit's note on groups left out, printed even for 0
  ("note: 0 group (0 obs) omitted ...").
- The estimates: clogit with the group's weight (the weights must be constant within a group), on
  Stata's Newton-Raphson path from a weighted logit's slopes (as for clogit).
- The variance, linearized: V = D (G/(G-1) sum over PSUs of u u') D, D the inverse of the weighted
  information, u a PSU's sum of w_i (y_i - p_i) x_i. Design df = PSUs - strata (here 119).
- t tests on the design df; the model test Stata's adjusted Wald F = (df - k + 1) / (k df) W with
  (k, df - k + 1) degrees of freedom.
- e(): k k_dv converged rc k_eq_model N_drop N_group_drop stages N N_pop (the groups' weights)
  N_strata N_psu df_r k_eq rank p F df_m census singleton N_strata_omit; macros cmd(clogit) cmdline
  ("svy : clogit ...") prefix(svy) cmdname command estat_cmd(svy_estat) vce(linearized)
  vcetype(Linearized) title("Survey: ...") wexp wtype(pweight) wvar singleunit su1 properties predict
  marginsnotok marginsok marginsdefault group depvar. Not kept: ic, V_modelbased, V_srs, Cns, the
  strata matrices, ilog, gradient.

## After it

- predict pc1 and pu0 as after clogit.
- margins with vce(unconditional): the margins are weighted means (Population size the rows' weights);
  each row's part of the variance is w_i (f_i - m) / sum(w) plus its influence on b through the
  Jacobian, summed by PSU with G/(G-1); t tests on the design df, Bonferroni as asked. One standard
  error in `svy.do` agrees to 6 digits (margins' numerical derivatives), the rest to 8.

## YFBPT6 itself

The weighted analysis does not reach svy yet: `weight_rake` comes from rake.do's `survwgt rake`
(an SSC add-on, raking to the age-by-gender quotas), which is not translated, so `weighted_data.dta`
is saved without it and `keep id weight_rake` stops in R. rake.do also needs `isid`. Those two are next
for this package (its `save` to a tempfile, line 37, stopped on an earlier run's file, now removed).

## Found on the way

- `svy:` arrived as a command (like bootstrap:); the model after the colon now gets the rewrites every
  command gets (fvset's bases were lost).
- `ereturn list` and `return list` count as leaving the data alone (margins, vce(unconditional) after
  them had been refused as "changed data").
