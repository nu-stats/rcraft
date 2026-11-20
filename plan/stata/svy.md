# svyset and svy: clogit (2026-11-06), the design and the estimators (2026-11-19)

YFBPT6's weighted analysis: `svyset id_g [pweight=weight_rake]`, `svy: clogit chosen i.actor i.reform
i.opinion, group(csid) baselevels`, `predict pc1`, `predict pu0, pu0`, `margins, dydx(*) post
predict(pu0) asobserved vce(unconditional) mcomp(bonferroni)`, parmest, coefplot. Observed in Stata 19
with made-up data shaped like it (`svy.do` keeps the checks).

## svyset

`svyset psu [pweight=w]` prints the design (Sampling weights, VCE linearized, Subpop. df off, Single
unit missing, Strata 1 <one>, Sampling unit 1, FPC 1 <zero>). Strata, stages and fpc followed in batch 5 (below).

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

Reached since `survwgt rake` and `isid` are translated (`survwgt.md`): the nine countries' svy clogit
and margins tables match Stata's to every printed digit (all nine svy tables, 81 rows, and Sweden's margins checked). Stata's header writes
counts with thousands separators (1,316 PSUs), as RCraft now does.

## Found on the way

- `svy:` arrived as a command (like bootstrap:); the model after the colon now gets the rewrites every
  command gets (fvset's bases were lost).
- `ereturn list` and `return list` count as leaving the data alone (margins, vce(unconditional) after
  them had been refused as "changed data").

## Batch 5 (2026-11-19, observed in Stata 19 on made-up data; corpus `svy_design.do`, `svy_mean.do`)

Cross-checked with StatCraft's `survey.md` (ported from statcraft@b7d392e: the variance formula, the printing rules); every
number below was also seen in Stata 19 here.

### svyset

- Forms: `svyset psu [pweight=w], strata(s) fpc(f) singleunit(rule) vce(linearized)`, later stages after `||` with
  their own `strata()` and `fpc()`, `[iweight=w]`, no psu (`_n`, each row a unit), no weight, bare `svyset` (shows the design,
  or says none is set), `svyset, clear`. A trailing `||` adds nothing.
- The design block aligns its labels to the longest ("Importance weights" is two longer than "Sampling weights").
- A stage with no fpc is sampled with replacement: when stages follow it they are dropped, with a note before the block
  (wrapped at the line size). With fpc at stage one and none at stage two, a third stage is dropped the same way.
- r(): `stages` as a scalar; the macros in the order Stata lists them: `settings`, then for the last stage first `fpc#`,
  `strata#`, `su#` (a stage with `_n` has no `su#`), `vce`, `singleunit`, `wexp` (`= w`), `wvar`, `wtype`. `settings` is the
  command as Stata rewrites it (`[pweight= w]`), with `singleunit()` and `vce()` after the last stage's options.
  `svyset, clear` leaves only `settings` = `, clear`.
- Refused: replicate-weight and other variance methods, poststrata, `[fweight]`/`[aweight]`, an expression for the weight,
  options written out of order, abbreviated `singleunit()` words, more than five stages. Stata stops on a fweight, an
  aweight and an `if`, and on a missing variable; those lines are refused or stop in R.

### svydescribe

- `svydescribe [varlist] [if] [in] [, stage(1) finalstage]`: a title, the design block, and a table of the first
  stage's units by stratum. Without a varlist: units, observations, and the least, mean (`%.1f`) and most observations
  per unit. With one: units included (some row has all of the variables) and omitted, rows with complete and with missing
  data, and the per-unit counts over the included units. `finalstage` lists each unit (stratum, unit value) in the
  one-stage design with a PSU variable.
- A unit is a (stratum, PSU) pair, so the same PSU value in two strata is two units. Rows with a missing stratum,
  PSU or weight, and the rows `if` and `in` leave out, are counted in lines under the table (the number of rows left out, then
  the total; the footer sits at column 28, 43 or 33 by table). A zero weight keeps a row; a negative
  one stops the command.
- r(): `N_single`, `N_mstrata` (strata whose units are all omitted), `N_munits` (units omitted), `N_miss` (rows with
  missing varlist data, 0 without a varlist), `N_strata`, `N_units` (omitted ones too), `N` (rows in the sample, missing
  data included).
- Refused: later stages (`stage(2)`), `finalstage` over several stages or with `_n` for units, `single`, `generate()`, a
  stratum with one unit (Stata marks it `1*`), a stratum whose units are all omitted when no unit has data at all,
  strata labels wider than 8 characters or not whole numbers.

### svy: mean, total, ratio, proportion

- Output: `(running mean on estimation sample)`, the title, the design on the left (`Number of strata`, `Number of
  PSUs`, with `subpop()` `Subpop. no. PSUs`) and the counts on the right (`Number of obs`, `Population size`, `Subpop. no.
  obs`, `Subpop. size`, `Design df`) ending at column 62, each block's numbers as wide as its widest, with thousands
  separators; the table has a `Linearized` line over `std. err.` (for proportion, one line `Linearized  Logit`).
- The estimation sample: rows where the variables, the PSU, strata, fpc and weight variables, and a `subpop()` variable
  are all present (`if` narrows it). `subpop()` only zeroes the scores outside the subpopulation, so the PSU and strata
  counts and the design df stay those of the sample. `subpop(if exp)` leaves out a row where the expression is missing,
  as a variable does.
- Variance: scores summed to PSU totals, per stratum `(1 - f) n/(n - 1)` times the squared deviations about the stratum's
  mean total, summed. Scores: mean `w (y - b) / W`; total `w y`; ratio `w (y - R x) / X`; proportion as a mean of an
  indicator. Estimates of one command (variables, `over()` groups, proportion levels) share the PSU totals, so they
  covary. `fpc` of at most 1 is a sampling rate, above 1 a count of PSUs (rate = n / count). Design df = PSUs - strata
  (strata with one PSU included, whatever the rule).
- `singleunit()`: missing (e(V) 0 and every standard error and interval missing, `e(rank)` 0, a note under the table),
  certainty (their variance is 0), scaled (the rest times H / (H - singletons), H the strata), centered (each such total
  about the mean of all PSU totals, factor 1). The notes wrap at 62 columns.
- e(): scalars `df_r`, `N_subpsu`*, `N_strata_omit`, `singleton`, `census`, `N_subpop`*, `N_sub`*, `N_pop`, `N_psu`, `N_strata`,
  `N_over`, `N`, `stages`, `k_eq`, `rank` (* with subpop()); macros `cmdline` (`svy : mean y` or `svy , subpop(sub): mean y`),
  `cmd`, `prefix`, `cmdname`, `command` (`mean y,` with `over(...)`; ratio `(name:y/x)`; proportion the levels), `title`,
  `vcetype`, `vce`, `namelist` (ratio), `estat_cmd`, `varlist`, `marginsnotok`, the weight macros, `singleunit`, `su1`,
  `strata1`, `fpc1`, `over`, `subpop`, `properties`; matrices `b`, `V`, `_N_overpsu`, `_N_overstrata`, `freq`, `_N_subp` (the
  weighted size of each estimate's rows), `_N`, `error`, `_N_strata_certain`, `_N_strata_single`, `_N_strata`. Not kept:
  `V_srs`, `V_srssub`, `V_srswr` (`ereturn list` says so).
- Refused: `over()` of more than one variable or with more than one analysis variable, over() with ratio or proportion
  (Stata widens the table for the long row heads) or with `subpop()`; a stratum sampled completely (fpc at 1 or the whole
  PSU count); strata with one PSU together with `fpc()`; `subpop()` taking values other than 0 and 1 (Stata prints a note
  twice); designs with later stages (their variance adds a second-stage term not decoded); iweights; `svy` options
  other than `subpop()`, `level()` and `vce(linearized)`; `test`, `lincom`, `estat` and `predict` after these.
- Not done: `svy: regress`, `logit`, `probit` and `poisson` (their header with F and R-squared, the iteration log and
  the adjusted Wald `test`; svy poisson stops after one iteration from a start not reproduced).
