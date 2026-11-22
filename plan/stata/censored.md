# tobit, intreg, truncreg, cloglog, hetprobit, heckman (2026-11-20, batch J)

Observed on Stata 19 SE with made-up data (corpus `tobit.do`, `intreg.do`, `truncreg.do`, `cloglog.do`, `hetprobit.do`,
`heckman.do`: 1e-6, truncreg and heckman 1e-5). StatCraft's notes (`censored.md`, `truncreg.md`, `cloglog.md`,
`heckman.md`; ported from statcraft@b7d392e) gave the likelihoods, starting values and layouts; every rule below was run
again here. Black box only. The R is `crates/statatr/src/data_r/censored/*.R` (our own), driven from `censored.rs`.

## One engine for all six

Newton-Raphson with ml's step rules (`stata_cx_max`): halve while the likelihood falls, else step on by 1/8, 1/4, ... of the
step while it rises (each added to the point reached); where -H is not positive definite, modified Marquardt (direction
(-H + 1.1 |diag|)^-1 g as a unit vector, first length .01 |ll| / |g.u|, doubled while it helps); done when
g'(-H)^-1 g <= 1e-5 and (reldif(ll) <= 1e-7 or reldif(b) <= 1e-6), tested only where -H is positive definite. Log lines
carry `(not concave)` or `(backed up)`. Robust variance: (-H)^-1 (sum of scores s s') (-H)^-1 N/(N-1), clusters summed
first with G/(G-1); a frequency-weighted row counts once with its score over w.

## tobit

- Start: least squares with censored values at their limit, sigma^2 = RSS/N; the parameter is sigma^2 itself.
  Constant-only model fitted silently for e(ll_0). `ll`/`ul` alone censor at the smallest/largest value; values at or
  beyond a limit are censored.
- Log: blank, `Refining starting values:`, blank, `Grid node 0:  Log likelihood = ll(start)`, blank,
  `Fitting full model:`, blank, iterations, blank. With nothing censored no grid preamble; with `nolog` nothing but the blank line.
- Header: LR chi2 and Prob > chi2, or with robust/pweights/clusters `F(k, N-k)`: **the F test and every t test use N - k degrees of
  freedom with clusters too** (not G - 1; StatCraft's note had G - 1). Var row is `var(e.y)` with an interval on the log scale.
- Errors: `no observations` r(2000); `initial values not feasible` r(1400) (all values censored, e.g. `ul()` below the data);
  `observations with ll() > ul() not allowed` r(198); noninteger fweights r(401).
- Factor variables: base columns are zeros in e(b); e(rank) = k - bases, e(k_autoCns) = bases (tobit only; e(Cns) not kept).
- aweights are allowed by Stata (each row's variance scaled); not reproduced here, refused.
- predict: xb, stdp, stdf = sqrt(stdp^2 + sigma^2), pr(a,b), e(a,b), ystar(a,b) (limits numbers, variables or `.`).

## intreg

- `intreg lo hi x`: lo missing is left-censored at hi, hi missing right-censored at lo, equal exact, both missing dropped;
  `lo > hi` stops with r(498). Olsen's parameters (beta/sigma, (constant - m)/sigma, 1/sigma, m the weighted mean of the
  midpoints); results turned back with the delta method. Two logs (`Fitting constant-only model:`, `Fitting full model:`).
- e(b) names `model:x`, `lnsigma:_cons`. LR chi2, or Wald with robust. aweights refused as for tobit.
- predict as tobit with sigma = e(sigma).

## truncreg

- Rows at or outside the limits go: `(N obs truncated)` first (after `(sum of wgt is S)` for aweights); with fweights the
  count is weighted. sigma itself is the last parameter, start least squares on the rows kept. aweights are rescaled to sum
  to the rows kept; pweights are robust; iweights raw.
- Stata's derivatives are numerical, ours exact: estimates agree to about 1e-7, so the corpus tolerance is 1e-5.
- e(means), e(dummy) hold the means of the regressors and zeros. predict: xb, stdp only.

## cloglog

- Start: the logit fit (from the constant-only model, same stepping) taken one iterate before its end. e(ll_0) is the
  likelihood at ln(-ln(1 - ybar)) rounded to float. Weights fw, pw, iw (aweights r(101)). Log without a preamble.
- predict: pr (default, `(option pr assumed; Pr(y))`), xb. eform: column `exp(b)`, every row exponentiated, constant too.

## hetprobit

- `het(varlist)` required (r(198) without). Log: `Fitting probit model:` then `Fitting full model:`, from the probit
  fit with the variance coefficients at zero. Header counts zero/nonzero outcomes by rows even with fweights; e(ll_c) is
  missing with robust fits. predict: pr, xb, stdp, sigma.

## heckman

- ML from the two-step start (rho clipped to +-.99); weights fw, pw, iw (aweights r(101)); `twostep` with weights stops
  `weights not allowed with option twostep` r(198). The mean of delta in sigma's start is unweighted. The LR test of rho
  is against probit + regression (the regression ll counts the whole part of the weights' sum); robust fits use a Wald test.
- predict: xb, xbsel, psel, mills, ycond, yexp. e(depvar) is `y s` with `select(s = ...)`.

## Open (refused, not guessed)

- `from()`, `iterate()`, `noconstant`, `offset`, constraints; tobit/intreg with aweights; interactions and `ib.` bases in
  tobit/intreg; factor variables in truncreg, cloglog, hetprobit, heckman (and in `select()`); estat after these models other
  than `ic` (R's layout); `stepwise` and `nestreg` with tobit (on hold: their search runs on R fit objects).
