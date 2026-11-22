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

## Round 2 (2026-11-22)

- `iterate(#)`: the search stops after # iterations, `convergence not achieved` follows the log, `Warning: Convergence not achieved.` ends the output (before the LR test line in hetprobit, after it in heckman), e(converged) is 0. In intreg the constant-only fit stops too; in tobit the constant-only fit is limited as well (its ll_0 changes); the probit and logit starts of hetprobit, heckman and cloglog run to convergence.
- `offset(var)`: the outcome moves by it (limits too), a row `var  1  (offset)` ends the table, e(offset) (e(offset1) in truncreg, hetprobit, heckman and tobit), predict xb adds it. The constant-only models take it.
- `noconstant`: intreg, cloglog and hetprobit and heckman twostep match to the iteration (no constant-only fit, a Wald test, no ll_0). Not matched, so refused: tobit (the estimates, ll, F test and e(Cns) match, Stata's starting values and so its log do not), truncreg (variances differ by about 1e-3 from the exact Hessian), heckman by maximum likelihood (different start).
- `from(matname [, skip])` in tobit: columns matched by `eq:name` (no equation means the first), unnamed parameters start at 0 and sigma^2 at the mean squared residual; `copy` stops `invalid from() option; option copy not allowed` r(198); an extra column r(111) with a leading blank line. A vector naming nothing is refused (Stata's path from zeros takes one iteration less).
- `stepwise` and `nestreg` with tobit (no weights, or fweights in stepwise): F tests on e(df_r) in nestreg (Block F df Residual df), the search tests with F (Wald) or LR; the final model prints without a log in stepwise and with it per block in nestreg.

## Open (refused, not guessed)

- `constraints()` (the constrained Newton path is not decoded), tobit/intreg with aweights, interactions and `ib.` bases in tobit/intreg, factor variables in truncreg, cloglog, hetprobit, heckman (and `select()`), the refused `noconstant` forms above, margins after these models (see the estat section below).

## estat after the six (2026-11-22, P2C.12be)

Observed on Stata 19 SE (corpus `estat_censored.do`, 1e-5, 1,550 recorded values; the printed layouts were also compared line by line with Stata's log, apart from the last digit of truncreg's numerical variances).

- **Accepted by Stata after all six**, plain, weighted, robust, clustered: `estat ic`, `estat vce`, `estat summarize`. Every other subcommand (gof, classification, lroc, wcorr, sbcusum, esize, mfx, ovtest, hettest, imtest, se, or a made-up one) stops `estat X not valid` r(321); `estat` alone `subcommand required` r(321); `estat bootstrap` `last estimates not found` r(301).
- **`estat ic [, n(#) df(#)]`**: the `estimates stats` table with the one row `.`, `ll(null)` shown as `.` where there is no null model (truncreg, hetprobit, heckman), N the weighted count for fweights, `Note: BIC uses N = ...` only when n() is absent, N with commas from 1,000. r(S) is 1 x 6 (N ll0 ll df AIC BIC, row `.`). `n(#)` below 1 is r(125); a varlist is r(101); an unknown option r(198). After `heckman, twostep` there is no likelihood: `likelihood information not found in last estimation results` r(321).
- **`estat vce [, correlation covariance format()]`**: the lower triangle in one block per equation (tobit: the dependent variable then `/ var(e.y)`; intreg `model` and `lnsigma`; truncreg `eq1` and `/ sigma`; cloglog the dependent variable; hetprobit it and `lnsigma`; heckman it, the selection equation and `/` athrho lnsigma, or `/mills` lambda after twostep). Widths: first column of a block fw+1, the others fw+2, a block closed by a space; a name with a parenthesis (`var(e.y)`) has its separating space in front. r(V) holds the matrix (the correlations with `correlation`) named `eq:name`. Wider than the line is refused (Stata splits the columns).
- **`estat summarize [varlist] [, equation labels noheader]`**: mean, standard deviation, min and max over e(sample), the dependent variables first (intreg both bounds, heckman both outcome and selection variable), then the regressors, each once; `equation` groups them under `depvar` and each equation. Weights: fweights count each row w times (N is the sum, the standard deviation divides by the sum - 1), iweights divide by the sum - 1, aweights and pweights by N - 1 on the rescaled weights; a `_weight` row (unweighted statistics of the weight) follows a separator. A cluster fit adds the clusters block (min, avg, max of the cluster sizes) and `Std. dev. not adjusted for clustering`; a variable with missing values in the sample gets `<v> statistics based on K nonmissing observations` (heckman's outcome, intreg's lower bound). `labels` uses the narrower table with a Label column. r(stats) is the matrix (rows `depvar:y`, `y:x1` ... with `equation`, `_weight` or `weight:_weight`).
- **On hold:** `estat vce` and `estat summarize` with factor variables (Stata adds base-level rows and level headings), `estat vce` wider than the line (Stata breaks the columns into panels), `estat vce` with equation(), block or diag, fractional n() or df() (Stata shows them rounded), `estat summarize` with a varlist and equation, fweights with clusters, `margins` after these models (margins.rs works from R fit objects; these keep their results in e(), so it needs a delta-method engine over predict xb, pr(), e() and ystar() first).
