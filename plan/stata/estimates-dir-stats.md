# estimates dir, stats, replay (2026-11-13)

Observed on Stata 19 SE (probes `r6/e1.do`, `r6/e4.do` in the session scratchpad, identical to RCraft's output);
corpus `sdtest.do` (estimates stats). Translated in `returned.rs` (estimates_cmd) with helpers in `estat.rs`.

- `estimates dir [names]`: a table of 64: `        Name | Command    variable     param.  Title ` under
  `Dependent  Number of`, each stored model's e(cmd) (%-8s), e(depvar) (%11s), e(rank) (%11s) and e(title).
- `estimates stats [names]`: "Akaike's information criterion and Bayesian information criterion", a table of 77:
  N, ll(null), ll(model), df (e(rank)), AIC = −2ll + 2df, BIC = −2ll + ln(N)df (%9.0g), and the note "BIC uses
  N = number of observations. See [R] IC note."; with no names the active results, named as stored or `.`;
  r(S) (rows the names, columns N ll0 ll df AIC BIC).
- `estimates replay [names]`: for each, a rule as wide as the line size (79 when not set, as Stata's batch
  mode), `Model name` (or `active results` when the active results are not a stored model), the rule, then the
  model's own output again. Translated for regress without weights or robust errors (stata_regress_print on the
  stored results).
- Replay of regress with vce(robust) or clustered errors too (probe `r7/f4.do` identical); replay of logit, probit,
  poisson waits on those models' Stata layout (their output is R's in RCraft), and is refused.
- Replay of logit, probit and poisson (2026-11-14; probe `r9/g2.do` identical, with the models' Stata layout
  from 4316e18): Stata 19 replays the coefficients at 95% whatever `or`, `irr` or `level()` the fit had, keeps
  robust errors (Robust, Log pseudolikelihood, Wald chi2), and prints no iteration log. A model the layout can't
  place falls back to R's, as at the fit.
