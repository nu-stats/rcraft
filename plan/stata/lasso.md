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

`lasso cox`, `cvplot`, factor variables (`i.fold`), weights, `cluster()`, `offset()` for the
linear lasso, `tolerance()` other than the default, `dtolerance()`; test, lincom and margins after
lasso. (lasso logit, probit, poisson, elasticnet linear, sqrtlasso and dsregress, poregress,
xporegress, dslogit: below, 2026-11-16.)

# lasso logit, probit, poisson and elasticnet linear (2026-11-16)

Translated: `lasso logit|probit|poisson` with every selection `lasso linear` has, always-in
variables, `penaltywt()`, `noconstant` (not with cross-validation), `offset()` and, for Poisson,
`exposure()`; `elasticnet linear` with `alphas()` (`alpha()`), `selection(none | bic | cv[,
folds() serule stopok strict gridminok])`, the grid, `stop()`, `rseed()`, always-in variables,
`penaltywt()`, `noconstant`; lassoselect, lassoknots, lassocoef, lassoinfo and predict after both.
StatCraft's rules (its probes 472, 474, 481, 598) ported again and checked on Stata 19 here.

## Found here

- **The first lambda is 1e-10 above lambda_max**, for every model, linear included: after
  `lassoselect id = 1`, e(lambda_sel) is lambda_max + 1e-10, and so is e(lambda_serule) or
  e(lambda_bic) when ID 1 is chosen, while e(lambda_gmax) is lambda_max (Poisson showed 5e-10 once;
  RCraft adds 1e-10). With several elasticnet alphas e(lambda_gmax) is the shared grid's first
  lambda with the 1e-10 in it.
- **e(properties)** stays after lassoselect when the results had a selection before (it is not
  there after `selection(none)`), right after e(sel_criterion).
- **The CV search for logit, probit and Poisson ends by the linear rule** (0.1% above the running
  minimum three times in a row; probe 474's logit and probit runs), not after five rises
  (StatCraft's fit). elasticnet's BIC search ends by the linear BIC rule too (1% twice), not 10
  lambdas past the minimum.
- **GLM noconstant** fits the centred predictors without a constant (Stata's log likelihood to
  2e-10); which constant Stata's held-out and predicted values then use is not known, so
  cross-validation and predict with it are refused.
- **offset(), exposure()**: e(offset) or e(exposure) comes between e(allvars) and e(depvar) (after
  e(depvar) after lassoselect). Errors: `outcome must be 0 or 1` r(2000), `variable y must be
  greater than or equal to zero` r(459), `option exposure() must be greater than zero` r(459),
  `option exposure() not allowed with model logit` r(198).
- **Stata's GLM descent is not decoded.** With `tolerance(1e-12)` Stata's coefficients equal the
  optimum RCraft solves to (to about 1e-11), so the difference at the default is only where its
  descent stops. Tried and failed at tolerance(.5), (.1), (.01) and (1e-3) (logit and Poisson,
  grid(6)): IRLS with the inner sweeps to the linear rule and the outer step stopped by the same
  rule, by the first inner sweep, by the deviance, or after one step; one sweep per IRLS step;
  the constant updated first, last, by weighted centring or by Newton to convergence; coordinate
  Newton steps with the weights renewed per coordinate or per sweep; glmnet's curvature-weighted
  change; the 1/4 bound for logit; reverse order. None comes within 1e-2 at tolerance(.5). So: the
  selected models' log likelihood, BIC and CV error agree to 8 digits in the probes and the
  corpus; at small lambdas coefficients differ by up to about 1e-7 relative (path-end L1 and L2
  norms 1e-8 to 8e-8, Poisson's e(L2_max) in probe 474 1.14e-8), extreme predicted probabilities
  by 4e-7. The corpus `lasso_glm.do` compares at 1e-6 for that.
- **elasticnet**: each alpha (largest first) is a lasso path of `alpha |b| + (1 - alpha) b^2 / 2`
  (the coordinate update divided by 1 + lambda (1 - alpha), Stata's descent rule otherwise: probe
  472's coefficients to 1e-15) on the shared grid (StatCraft's augmented grid), with the same folds;
  each alpha's search ends by its own rule (probe 472: 44, 44, 44 and 48, 48, 53 lambdas; the
  corpus 45, 44, 43 and 50, 49, 51 — not "as many as the first alpha"). The selected alpha and
  lambda have the smallest BIC or CV error over all alphas; the one-s.e. rule works within the
  winning alpha. e(n_lambda) and e(lambda_last) are the winning alpha's, the nonzero, deviance
  ratio, L1 and L2 extremes are over every alpha's lambdas (probe 472: e(k_nonzero_max) 7 where the
  winning alpha has 5). Several alphas: no deviance stop (e(stop) 0). Duplicate alphas count once.

## Not yet (these two)

GLM noconstant with cross-validation, predict after offset()/exposure()/noconstant, lassoknots
display() after a GLM (its headings), elasticnet logit/probit/poisson, alpha 0 (ridge: probes 503,
505), several alphas with selection(none), alllambdas and BIC postselection with elasticnet,
lassoknots after lassoselect for elasticnet, lassoselect lambda = # with several alphas, lassoinfo
after elasticnet selection(none), a CV function without a minimum with elasticnet.

# sqrtlasso (2026-11-16)

Translated: `sqrtlasso depvar [(alwaysvars)] othervars` with selection none, bic, cv and plugin
(heteroskedastic, homoskedastic), the grid, stop(), rseed(), noconstant, penaltywt(), always-in
variables; lassoselect, lassoknots, lassocoef, lassoinfo (`sq linear`), predict. Stata's errors:
`selection(adaptive)` ("Only one of cv, none, plugin, or bic is allowed.") and `lasso sqrt`
("only one of linear, logit, probit, poisson, or cox is allowed"), both r(198).

## Found here

- **Stata's solver.** Not the lasso at penalty lambda sigma / N iterated to sigma's fixed point
  (StatCraft's; that is the exact optimum, 1.5e-8 off at probe 473's path end): cyclic coordinate
  descent on the square-root objective itself, each coefficient set to its exact minimiser
  b = a - sign(a) t sqrt((q - a^2) / (1 - t^2)) when |a| > t sqrt(q), a = z_j'(r + z_j b_j)/N,
  q the mean squared partial residual, t = lambda w_j / N, stopped by the linear lasso's rule
  (tolerance() relative change in the variable's own units). Probe 473's path end: L1 to 6e-14,
  L2 to 1e-13, deviance ratio to 1e-15.
- **StatCraft's entry rule is that sweep.** A zero coefficient is tested against the q kept from
  the last nonzero coefficient before it in the sweep (that one's partial residual), or the sweep's
  own at its start. That is StatCraft's "root mean squared error with the nearest earlier active
  predictor left out": with it every CV error of probe 473 is Stata's, and the grid lines of
  probes 506, 507 (735 and 606 lines) and 597 are Stata's to the character.
- **Plug-in rounds start from the last round's fit**, as the linear lasso's do (probe 473's
  e(L2_max) to 1e-15; from zero it is 1.2e-8 off).
- **A syntax error keeps e()**: after `capture lasso sqrt ...` Stata's e() is still the sqrtlasso
  before it (probe 473's predict); RCraft keeps e() after every lasso line that only stops with
  Stata's own error.
- Probes 473, 506, 507, 597 on R: 5 of 5 compared lines to 8 digits. Corpus `sqrtlasso.do`: 17
  results, at 1e-7 (at the path's end coefficients and norms differ by up to 4e-8, as for the
  linear lasso).

# dsregress, poregress, xporegress, dslogit (2026-11-16)

Translated: `dsregress`, `poregress`, `xporegress` and `dslogit depvar varsofinterest,
controls([(always)] others)` with selection plugin (the default), cv, bic and adaptive,
`lasso(varlist, selection(...))`, `sqrtlasso` (linear), `rseed()`, `vce(robust | ols | hc2 | hc3 |
cluster v)` (not cluster for poregress; xporegress robust only, vce(ols) gives robust as in Stata),
`level()`, `coeflegend`, `nolog`, `coef`/`or` (dslogit), `xfolds()` and `resample()`; the log, the
table, e() in Stata's order, e(sample), `lassoinfo` (plug-in lassos), `predict` (xb only: "partial
linear prediction"; other options are Stata's r(198)), Stata's errors (`option controls()
required`, `varsofinterest required`, a variable in both, all r(198); `outcome must be 0 or 1`
r(2000)). StatCraft's rules (its probes 475, 480), checked on Stata 19 here.

## How

Each lasso is the lasso RCraft already runs as Stata does (`stata_lasso_run`, inner): the outcome
and each variable of interest on the controls, for `dslogit` the outcome by the logit lasso on the
variables of interest and the controls (logit's homoskedastic plug-in by default). Double
selection: least squares (logit for dslogit) of the outcome on the variables of interest and the
union of the selected controls; partialing out: the outcome's residual on the variables' residuals,
each from least squares on its own selected controls, no constant; cross-fit: folds from one
`runiform()` per observation, rank r to fold r mod K, the held-out residuals pooled, the robust
variance with the moment and score matrices averaged over folds, resamples averaged with the
spread of their coefficients added.

## Found here

- With no `selection()` the lassos are the plug-in ones (not lasso's own default, cv).
- `ereturn list` puts an e() name longer than 17 characters on two lines (`e(lasso_depvars_uni` /
  `q)`), for every command.
- The display options (`coeflegend`) are the commands' own: they print Stata's table.
- `controls(x1-x10)`: the run makes the option's range the data's names, as it does for varlists.
- `poregress, vce(cluster g)`: the variance is 1.8% off Stata's in lasso_infer.do while
  dsregress's agrees (probe 475 and the corpus); refused.

## Checked

Probes 475 and 480 on R: 7 of 7 compared lines (ereturn list after dsregress, poregress,
xporegress with and without resamples, dslogit) to 8 digits; every e(b) and e(V) printed with
%21x to 2e-12; every coefficient table (robust, ols, hc3, cluster, level(), coeflegend, cv, bic,
adaptive, lasso(), sqrtlasso, always-in controls, cross-fit with 2-10 folds and resamples, dslogit
odds ratios) line for line. Corpus `lasso_infer.do`: 23 results at 1e-7 (all at 1e-8 but
dslogit's variance, 1.4e-8 off).

## Not yet

pologit, xpologit, dspoisson, popoisson, xpopoisson (StatCraft did not reproduce them); cv or
lasso() inside xporegress; xfolds(1); lassocoef and lassoinfo after cv lassos; test, lincom,
margins after these commands.
