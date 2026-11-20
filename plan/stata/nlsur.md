# nlsur (2026-11-19, round 4 2026-11-20)

Observed on Stata 19 SE with made-up data (corpus `nlsur.do`, 680 results to 1e-6; the iteration log compared by eye
on scratch do-files, line for line, and the corpus run's printed lines against Stata's log). StatCraft's note (probe
537; ported from statcraft@b7d392e) had the step rule and the formulas; what changed here is the derivative step, the
stopping rule, which variance matrix Stata keeps, the constant, the rows used, the `nls` pass and the layout details
below. Round 4 added weights, `vce(robust)`, `vce(cluster)`, the hidden `nls` pass, the iteration cap, the table of a run
that runs out of steps, `{name: varlist}`, `variables()` and postestimation. Translated in
`crates/statatr/src/data_r/nlsur.rs`.

Stata's log is the Gauss-Newton path itself, so it reproduces exactly once the step and the stopping rule are right.

## Rule

`nlsur (y1 = <sexp>) (y2 = <sexp>) ... [if] [in] [weight] [, nls fgnls ifgnls nolog level() iterate() eps() delta()
noconstant initial() variables() vce()]`. A parameter is `{name}` or `{name = #}`; names are shared across equations,
kept in order of first appearance (not sorted, unlike `nl`), and the start is 0 when none is given. `e(sexp_#)` is the
expression with a space before each `{` and after each `}`, runs of spaces made one.

**Linear combinations.** `{xb: x1 x3}` is the parameters `xb_x1` and `xb_x3` (start 0, in that place in the order), and
the stored expression has `({xb_x1} *x1 + {xb_x3} *x3)` in its place (no space after the parenthesis).

**Gauss-Newton, as Stata takes it.** Residuals stacked equation by equation, each row times the square root of its
weight, whitened by Sigma (the identity in the NLS stage). Derivatives are forward differences, each parameter moved by
**d = delta (|b| + delta)** (delta() 4e-7 by default; so delta squared at a zero start), d taken as stored. Found in
round 4: the earlier rule d = delta |b| (delta at 0) agreed on the round-3 fits, whose zero starts were all linear, and
missed four probes in the fifth to seventh digit of the log (a start c = 0 inside `exp(c*x)`, `delta(1e-4)` and
`delta(1e-3)`, and an ill-conditioned first step); d = delta (|b| + delta) matches all four to every printed digit, and
d = delta (|b| + 1) does not. A step solves the normal equations over the parameters whose columns
are neither zero nor a combination of earlier ones' (swept in order, tolerance 1e-13 of the diagonal); the others do not
move. The step is halved while it raises the criterion, never stepped further (unlike `nl`). The log shows the criterion
after each step, so "Iteration 0" is the value after the first step (`%9.0g`, right-aligned in 8).

**Stopping.** After a step, done when no parameter moved by more than `eps()` (1e-5) **relative to its old size**,
`|db| / |b|` (a parameter that did not move counts as 0). This is not `|db| / (|b| + 1)`: that form agrees on most
paths and stops one iteration early on others (found on a fit whose last step was 5e-6 against a size of 0.19).
`iterate()` caps the steps per stage; **the default is 300** (seen with `eps(1e-300)`: Iteration 0 to 299 in each stage).
Running out of steps in a stage does not stop the next stage; at the end of `nls` or `fgnls` the table is printed as
usual (same variance rule as below, matched on `iterate(2)` to 4e-8) and the command ends with r(430) and **no message**
(e(converged) = 0; Stata keeps e(), RCraft does not). Under `ifgnls`, `iterate()` caps each stage's steps but running
out is not an error: the rounds go on until their own rule holds (`iterate(1)`: five rounds of one step each, r(0),
e(converged) = 1).

**Stages.** NLS (log "Residual SS"; with any weight "Scaled RSS"), then FGNLS with Sigma = R'WR / N from the NLS
residuals ("Scaled RSS"). `ifgnls` goes round again from the last estimates with Sigma from the last residuals, each
round showing `FGNLS iteration r:`, its own log, `Parameter change` (max |db| / (|b_new| + 1)) and `Covariance matrix
change` (max |Sigma_new - Sigma_used| / (|Sigma_new| + 1), the new Sigma under the +1 as for the parameters; found in
round 4 on the first round of `iterate(1)`, 1.97e-02 where the old Sigma gave 1.93e-02), both `%9.2e`, stopping when
both are at most 1e-5. With `nolog` the lines are `Calculating NLS estimates ...`, `Calculating FGNLS estimates ...`
and, for `ifgnls`, `FGNLS iteration r ...`, no blank lines between.

**`nls` method.** After the stacked NLS stage (the one the log shows), Stata runs one more Gauss-Newton stage, not
logged, with Sigma = the diagonal of the NLS Sigma: each equation weighted by 1/variance of the stacked fit's residuals.
When each parameter belongs to one equation that pass does not move the estimates; with a parameter shared by two
equations it does (the earlier finding "the estimates equal each equation weighted by 1/variance"). The variance is the
FGNLS rule with that diagonal Sigma, e(Sigma) the diagonal. Checked to 1e-9 on two shared models, under `iterate(2)`
(where the hidden pass also takes two steps, which is why the round-3 table differed) and on `nls` with aweights and
`delta()`. With separate parameters it equals the round-3 sandwich on the diagonal blocks, and **the cross-equation
covariances are 0** (round 3 printed them from the full Sigma; corrected).

**Weights.** fweights are copies: N is their sum (the Obs column and e(N) too), Sigma divides by it. aweights and
pweights are scaled to sum to the rows; N is the rows. pweights imply `vce(robust)`. Weighted, the first line is
`(sum of wgt is #)` with the sum of the weights as given, `%12.0gc` (`225.6666667`, `2,256,666.7`, `.0002256667`,
`2.25667e+11`, `150,000`), in place of `(obs = N)`. rss, mss and R-squared are weighted (mss against the weighted mean),
RMSE sqrt(rss/N). e(wtype), e(wexp) (`= w`) after e(estat_cmd).

**Variance.** (J'(Sigma^-1 kron W)J)^-1 of the last Gauss-Newton step, with J taken at the **start of that step** and the
Sigma it used. Using the final J is off by 1e-5 where the last step was loose (decided on a dozen fits). The parameters
left out of the step are `(constrained)` rows with zero variance. e(Sigma) is the Sigma used (diagonal under `nls`),
except under `ifgnls`, where it is the one the final residuals give. e(ll) = -N/2 (M (1 + ln 2 pi) + ln |Sigma_final|)
with Sigma_final the full matrix from the final residuals (also under `nls`).

**Robust and clustered.** A^-1 B A^-1 with A the matrix above and B the sum of the scores s_i = w_i J_i' Sigma^-1 r_i, J and r
at the **final** estimates (A from the start of the last step, scores from the end: 3e-9 on the diagonal; the other
combinations are 1e-6 to 1e-2 off). No small-sample factor for robust. Frequency weights count each copy's score once
(w_i s s', not (w s)(w s)'). Clusters: the scores summed within each cluster, times G/(G-1) (N-1)/(N-k) (k the
parameters); the table has `(Std. err. adjusted for G clusters in v)` and the `Robust` header. e(vcetype) `Robust`, e(vce)
`robust` or `cluster`, e(clustvar), e(N_clust) (after k_eq_model; with clusters the scalars end converged, k_eq, rank).
Off-diagonal entries that are small beside the diagonal agree only to about 1e-6 (derivative noise); the corpus checks
the diagonal and the larger ones.

**Rows.** The rows with every dependent variable and every `variables()` variable present (and `if`/`in`, and a weight
not missing or 0); `(obs = N)` is printed first (commas from 1,000), then a blank line. A missing value on the right-hand
side is **not** a reason to drop a row: the run stops with "could not evaluate equation q" (r(480)) after the
`Calculating NLS estimates` line.

**Equation table.** `Equation | Obs Parms RMSE R-sq Constant`, 71-wide rules, the equation number, dependent variable
and counts as `%2d%13s |%11s%8d%11s`, then R-squared as `%.4f` after four spaces. The equation's **constant** is its
first parameter whose derivative is the same, and not zero, on every row, **at the final estimates** (so `{a}*x^{p=0}` is
not a constant when p ends up elsewhere); R-squared is centred when there is one, otherwise uncentred and marked `*` with
the footnote `* Uncentered R-sq`; the constant column is its name or `(none)`, right-aligned in `max(12, 19 - width of the
R-squared text)` (so a negative R-squared of 8 characters moves the column). R-squared is `mss / tss` with
`mss = tss - rss` unrestricted (negative R-squared shows); `noconstant` makes every equation uncentred. e(constants) is
the names, `.` for an equation without one (`. .` under both, with a leading space under the `noconstant` option).

**Coefficient table.** The z table with rows `/name`; with one equation or the same dependent variable throughout, the
variable's name heads the first column. `level()` is in the header and the interval.

**e().** Scalars in order N, rss_i, mss_i, r2_i, rmse_i, k_i, ll, n_eq, k, k_eq_model (0), converged, rank (free
parameters), k_eq; macros cmdline, cmd, predict (`nlsur_p`), estat_cmd, [wexp, wtype], [clustvar], [vcetype], vce (`gnr`),
method (`nls`, `fgnls`, `ifgnls`), constants, type (1), params, depvar, then params_i and depvar_i from the last equation
down, sexp_i from the last down, properties; matrices b (colnames `a1:_cons`, row `y1`), V, init (row `r1`, colnames the
parameter names) and Sigma (dependent variable names).

**predict.** `predict [type] newvar [if] [in] [, yhat residuals equation(#k)]`: fitted values, or y - fitted, of equation
k (default 1); a bare `predict` prints `(option yhat assumed; fitted values)`. `equation(name)` is r(198)
("misspecified"), a number past the last equation an error.

**Postestimation.** `test`, `lincom` and `nlcom` work on e(b) and e(V) with z: `/a1`, `[a1]_cons`, `_b[/a1]` and
`_b[a1:_cons]` all name the parameter, and the lines show `[a1]_cons` (`test /a1 = 1` prints `( 1)  [a1]_cons = 1`).
nlcom's table has no variable over it. `estat ic`: N = e(N), ll(null) `.`, df = e(rank). `estat vce` the usual table.

## Cases

- Zero starts on `{a2} + {b2}*exp({c}*x2)`: the b2 and c columns are zero or dependent at the start, only a2 moves,
  b2 and c end as `(constrained)` at 0, e(rank) = 3, Sigma's off-diagonal is the NLS one.
- Shared parameters under FGNLS (default): the NLS stage is the stacked least squares; FGNLS with the full Sigma follows.
- `iterate(2)`: both stages stop after two steps, the table is printed, then r(430) with no message.
- `ifgnls, iterate(1)`: five rounds of one step, r(0), e(converged) = 1.
- Two equations with the same dependent variable work (the table is headed `y1`).
- fweights 1-4 on 150 rows: `(sum of wgt is 375)`, Obs 375, e(N) 375.

## Open

- Refused, with a message: iweights, `parameters()` (Stata: "not allowed" outside the evaluator form),
  `hasconstants()`, the function-evaluator form, a linear combination named twice (`{xb:}` reused), factor variables,
  a parameter named as a variable, `vce(cluster v)` with missing values of v (not probed), `vce()` other than gnr,
  robust and cluster, `margins` and `estat` other than ic and vce after `nlsur`.
- After r(430) Stata keeps e() (the table's results); RCraft's run stops in R, so e() and `_b[]` after a captured
  unconverged `nlsur` are not R's.
- `eps()` far below the default: with `eps(1e-300)` Stata printed `(none)` and an uncentred R-squared for an equation
  whose constant it finds at the default; the constant check seems to use a tolerance tied to eps(); not decoded.
- Noise-level paths: a parameter whose true value is about 0 (`{xb: x1 x3}` on y1, where x3's coefficient is -0.001)
  takes 8 steps in Stata and 6 here, the estimate agreeing to 7e-9 absolute (6e-6 relative); the last `ifgnls` round's
  changes (about 1e-6) can differ in the third digit. e() scalars carry the last 1e-11 of noise (rss, ll).
