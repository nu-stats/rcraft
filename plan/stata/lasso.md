# lasso linear and what follows it (2026-11-15)

Translated: `lasso linear depvar [(alwaysvars)] othervars [if] [in]` with `selection(none | bic[,
postselection] | cv[, folds() serule alllambdas gridminok stopok strict] | adaptive[, steps()
power() folds()] | plugin[, heteroskedastic homoskedastic c() gamma()])`, `grid(#[, ratio() min()])`,
`stop()`, `rseed()`, `noconstant`, `penaltywt()`, `nolog`; `lassoselect id = # | lambda = #`,
`lassoknots [, display() alllambdas]`, `lassocoef [names] [, display(coef[, ...]) sort(coef[, ...])
nolegend]`, `lassoinfo [names]`, `predict [type] newvar [, xb penalized postselection]`, and Stata's
own errors (r(100), r(198), r(430), r(2000)) where its words are known. Code:
`crates/statatr/src/data_r/lasso.rs` (R helper `stata_lasso_*`).

## Where the rules come from

StatCraft (our own project) decoded Stata 19's lasso from its probes 380-385 and 470-504 (its
`plan/stata/lasso.md`): standardising with divisor N, the grid (lambda_max down to 1e-4 of it, 1e-2
when p >= N; `min()` exact), the deviance stop, BIC, the folds (one `runiform()` per observation of
the sample, ranks cut into K blocks, the first N mod K one larger), CV as the mean of fold errors (the
pooled mean when folds differ in size), the one-s.e. rule, Stata's "minimum found" rule (CV 0.1%
above the running minimum three times in a row, BIC 1% twice), the adaptive weights, the plug-in
rounds, the tables and e(). Ported to R here, and checked again on those probes' Stata 19 logs.

## Found here

- **Stata's descent.** Stata does not solve each lambda to the optimum. Cyclic coordinate descent over
  every predictor in order, warm started along the grid, stopped after the first sweep in which no
  coefficient, in its variable's own units, moved by more than 1e-7 relative to max(|b|, 1),
  reproduces e(L1_max), e(L2_max), e(devratio_max) and e(ll_sel) of probes 380, 381, 382 and 470 to
  about 1e-15 (the exact optimum is 1e-8 away and failed the 8-digit check), and probe 470's
  `tolerance(1e-3)` case too.
- **Not fully decoded.** On the corpus data (`lasso.do`: sin/cos predictors with sd 0.76) Stata
  stops one sweep earlier than that rule at the path's end (its L1 norm equals sweep 7 of the last
  lambda exactly; the rule stops after sweep 8), and at `tolerance(1e-3)` it takes 4, 3, 2 sweeps at
  lambdas 3-5 (probe 470's data: 4, 4, 2). No per-coefficient test |Δb|/max(|b|, c), with c a
  constant or sd(x), sd(y) or their ratio, and no norm-based test (largest, summed or Euclidean change
  over the norm of b, deviance or objective change) gives all of these. So the corpus compares at
  1e-7 (L1 and L2 at the path's end differ by up to 3e-8 there), and `tolerance()` other than 1e-7
  is refused (6th digit at 1e-3).
- **Plug-in.** Starting each round from the last round's solution comes closest (to about 2e-9 in
  probes 381, 385 and 471, where a start from zero is 1e-8 off with an always-in variable); Stata's
  exact start is not known.
- **e() over every adaptive step.** e(k_nonzero_max) and the other lambda statistics are over all
  steps' lambdas (probe 383: 9 nonzero in step 1, 8 at most in step 2), as e(n_lambda) is.
- **e(rngstate)** is `XAA`, the 312 words of mt64 in hex, `0001`, `0000`, the position and four
  check digits (StatCraft's reading). RCraft's generator draws ahead and hands words back, so the
  state is taken where the unused words came from, in the array before the last twist when they
  reach into it (probe 380 then matches).
- **ereturn list** cuts a macro too long for the line: the line is 254 characters at linesize 255
  (225 characters of the value, then `..`), linesize - 2 below it (every such line in the probes'
  logs, lasso's and logit's).
- **lassoknots** without the variables column: with one statistic besides lambda, `ID` meets the bar
  (`     ID|`, probe 501); with two or more there is a space (`    ID |`, probe 382).
- **Errors.** `lasso linear` alone is r(100) and `lasso linear y` r(198), with the same words. An
  option not in lasso's syntax is `option bogus not allowed` (198), a second `selection()` `option
  selection() not allowed` (198) (probe 384). After `lambda not selected` (r(430)) e() is posted all
  the same, so `display e(n_lambda)` after `capture` reads it (probe 504).

## Checked

- StatCraft's probes run on R (`engine/tests/stata_probes.rs`, the lasso folders renamed so file and
  folder agree): 49 of 49 compared lines (ereturn list, display) match Stata 19 to 8 digits in 25
  lasso probes; the 276 `RES` lines RCraft prints in the loop probes 500-504 (folds, stopping rules,
  the path's end) are Stata's to the character.
- Corpus `lasso.do` (31 results recorded from Stata 19): none + lassoselect, BIC, BIC
  postselection, CV with rseed and serule, adaptive, plug-in both ways, an always-in variable,
  penaltywt(), noconstant, if with stop(), predict, the error codes and r(430).

## Not yet

`lasso logit/probit/poisson/cox`, `elasticnet`, `sqrtlasso`, `dsregress` and the other inference
commands, `cvplot`, factor variables (`i.fold`), weights, `cluster()`, `offset()`, `tolerance()`
other than the default, `dtolerance()`; test, lincom and margins after lasso. StatCraft has the rules
for most of these (logit/probit/Poisson, elasticnet, sqrtlasso and the inference commands): the next
lasso step is to port them the same way, starting with `lasso logit` (probe 474) and `elasticnet`
(472, 505), then `sqrtlasso` (473, 506, 507, 597).
