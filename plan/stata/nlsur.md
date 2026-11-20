# nlsur (2026-11-19, rounds 4 to 6 2026-11-20)

Observed on Stata 19 SE with made-up data (corpus `nlsur.do`, 978 results to 1e-6; the iteration log compared by eye
on scratch do-files, line for line, and the corpus run's printed lines against Stata's log). StatCraft's note (probe
537; ported from statcraft@b7d392e) had the step rule and the formulas; what changed here is the derivative step, the
stopping rule, which variance matrix Stata keeps, the constant, the rows used, the `nls` pass and the layout details
below. Round 4 added weights, `vce(robust)`, `vce(cluster)`, the hidden `nls` pass, the iteration cap, the table of a run
that runs out of steps, `{name: varlist}`, `variables()` and postestimation; round 5 iweights, clusters with missing
values, a linear combination written twice, e() after r(430), margins, `estat summarize` and `estat vce`'s layout, and
the function-evaluator form; round 6 its options (`hasconstants()`, `nparameters()`, weights, `variables()`,
`noconstant`), `predict` and `margins` after it, and margins' terms, `over()` and `post`. Translated in
`crates/statatr/src/data_r/nlsur.rs`.

Stata's log is the Gauss-Newton path itself, so it reproduces exactly once the step and the stopping rule are right.

## Rule

`nlsur (y1 = <sexp>) (y2 = <sexp>) ... [if] [in] [weight] [, nls fgnls ifgnls nolog level() iterate() eps() delta()
noconstant initial() variables() vce()]`. A parameter is `{name}` or `{name = #}`; names are shared across equations,
kept in order of first appearance (not sorted, unlike `nl`), and the start is 0 when none is given. `e(sexp_#)` is the
expression with a space before each `{` and after each `}`, runs of spaces made one.

**Linear combinations.** `{xb: x1 x3}` is the parameters `xb_x1` and `xb_x3` (start 0, in that place in the order), and
the stored expression has `({xb_x1} *x1 + {xb_x3} *x3)` in its place (no space after the parenthesis). Written again
with the same variables (in the same or another equation) it is the same parameters: e(params) lists them once and
e(params_2) has them too. (`{xb:}` with no variables is not Stata's syntax: "too few variables specified", r(102).)

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
(e(converged) = 0; e() is kept, the table's results, and RCraft keeps it too). Under `ifgnls`, `iterate()` caps each stage's steps but running
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
pweights are scaled to sum to the rows; N is the rows. pweights imply `vce(robust)`. iweights are taken as they are,
as fweights (N their sum, printed `(obs = N)`, not the sum-of-weights line); `vce(robust)` with them is r(101), "robust
variances not available with iweights", before the log. Weighted otherwise, the first line is
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
parameters); a row whose cluster variable is missing is left out (obs = 149 with one missing); the table has
`(Std. err. adjusted for G clusters in v)` and the `Robust` header. e(vcetype) `Robust`, e(vce)
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
lincom's and nlcom's tables have no variable over them. `estat ic`: N = e(N), ll(null) `.`, df = e(rank).
`estat summarize`: `  Estimation sample nlsur` and `Number of obs = ` (`%10s`, commas) on one line, then the dependent
variables' mean, std. dev., min and max over e(sample) (`%9.0g` in fields of 13, 14, 13, 13 after the bar; the table
indented by 2, 67 wide). `estat vce` (and `, correlation`, `%8.4f`): each parameter its own equation, a header row of the
names (left in 10, after a space) and a row of `_cons`, then for each parameter a rule, its name row and its `_cons` row
of the lower triangle (`%10.0g` right in 10 between single spaces); the rules and name rows end one column short.
Other subcommands are Stata's own error (`estat gof not valid`, r(321)).

**margins.** The model has no covariates: `margins` is the predictive margin, the mean fitted value of equation k
(`predict(equation(#k))`, yhat by default) over e(sample) and the rows if/in keep (weighted as the fit was: fweights in N),
with the delta-method variance from e(V). Layout: a blank line, `Predictive margins` with `Number of obs = N` ending at
column 78, `Model VCE: GNR` (`Robust` after vce(robust)), a blank line, `Expression: Equation k fitted values, predict(...)`
with predict()'s text as typed (`predict()` when none), a blank line, then the z table with `Delta-method` over
`Margin   std. err.` and the row `_cons`. r(b), r(V), r(N), r(level). `dydx()` and its kind are r(322) ("invalid dydx()
option; x1 not found in list of covariates"), `at()` r(322) ("model contains no indepvars"), `predict(residuals)`
r(498) ("prediction is a function of possibly stochastic quantities other than e(b)").
**margins terms, over() and post.** A term is always Stata's error, the model having no covariates: `margins x1`
with a fractional x1 is r(452) ("x1:  factor variables may not contain noninteger values"), `margins grp` with whole
numbers r(322) ("factor grp not found in list of covariates"); `margins x1 grp` stops on x1. `over(v)` gives one margin
for each value of v among the rows (`Over:       v` after the Expression line; the table's first row `v |`, then each
level right in 11 and two spaces before the bar), r(b) named `1.grp`, `2.grp`, ... and their covariances; `over(v w)`
crosses them (`grp#g2`, rows `1 10`, names `1.grp#10.g2`, in order of v then w). A fractional over() variable is r(198)
("only integer values are allowed in over() and within() variables"). `post` makes the margins e(b) and e(V) (row
`y1`), with e() in this order: scalars k_at (0), k_by (the number of margins), k_predict (1), k_margins (0), N; macros cmd
(`margins`), title (`Predictive margins`), model_vce (the fit's e(vce)), model_vcetype (when the fit has one), vce
(`delta`), vcetype (`Delta-method`), expression, predict1_label (`Equation k fitted values`), predict1_opts (predict()'s
text, when given), by, over, by1, by2, ... (under over()), emptycells (`strict`), est_cmd, est_cmdline, cmdline (`margins`
and what followed it as typed, so `margins , post` for `margins, post`), properties; matrices b, V, _N (each margin's
count), error (0s) and Jacobian (the derivatives of each margin by the parameters, columns `a1:_cons` ...).

**The function-evaluator form.** `nlsur name @ y1 y2 x1 x2, parameters(a1 b1 a2 b2 c) neq(2) [initial(a1 1 ...)]` calls
the program `nlsurname` with the first neq() variables to fill with fitted values (copies), the others, `if` the
estimation rows and `at()` a row vector of the parameters. The rows are those with every variable of the varlist present
(and if/in). The fit is the same Gauss-Newton path as the substitutable form (the log and the estimates agree with the
expressions written out, to every digit); the table's Parms column is `.`, no equation has a constant (uncentred R-squared
with `*`, `(none)`), e(constants) `. .`, e(type) `3`, e(rhs) the other variables, e(funcprog) the program; there are no
e(k_i), e(params_i) or e(sexp_i). RCraft runs the program's lines once, at the starting values on copies of the dependent
variables, and makes the same R the function each step calls; a program line that is not translated, or that needs a
value from R while it is translated, refuses the fit. `parameters()` without the evaluator form is r(198)
("parameters(a1 b1 a2 b2 c) not allowed").

**The evaluator form's options.** `nparameters(#)` names the parameters `b1`, `b2`, ... . `hasconstants(a1 a2)` names
the equations' constants: each equation's is the named parameter whose derivative is the same, and not zero, on its rows,
so R-squared is centred and the column shows it (e(constants) `a1 a2`). Weights work as in the substitutable form
(`(sum of wgt is #)`); `variables()` leaves out the rows where one of them is missing (149 with one); `noconstant` makes
e(constants) ` . .`. `predict` (yhat, residuals, equation(#k), if/in) and `margins` call the program again on the data
as they are then.

## Cases

- Zero starts on `{a2} + {b2}*exp({c}*x2)`: the b2 and c columns are zero or dependent at the start, only a2 moves,
  b2 and c end as `(constrained)` at 0, e(rank) = 3, Sigma's off-diagonal is the NLS one.
- Shared parameters under FGNLS (default): the NLS stage is the stacked least squares; FGNLS with the full Sigma follows.
- `iterate(2)`: both stages stop after two steps, the table is printed, then r(430) with no message; e() holds that fit.
- `ifgnls, iterate(1)`: five rounds of one step, r(0), e(converged) = 1.
- Two equations with the same dependent variable work (the table is headed `y1`).
- fweights 1-4 on 150 rows: `(sum of wgt is 375)`, Obs 375, e(N) 375; the same as iweights: `(obs = 375)`.

## Open

- Refused, with a message: `hasconstants()` where an equation has none of the named constants (not seen in Stata);
  margins with several terms whose first is a whole-number variable, `over()` with value labels, missing or negative
  values or levels wider than the column, and `post` after `vce(cluster)` (none probed); a linear combination over
  other variables under a name already used; factor variables; a parameter named as a variable; `vce()` other than gnr,
  robust and cluster; iweights whose sum is not a whole number (the `(obs = #)` line not seen); `estat` subcommands
  not seen in Stata (only `gof` was probed as r(321)); `neq()`, `nparameters()` or `hasconstants()` outside the
  evaluator form; an evaluator program with a line that is not translated or needs a value from R.
- `eps()` far below the default: with `eps(1e-300)` Stata printed `(none)` and an uncentred R-squared for an equation
  whose constant it finds at the default; the constant check seems to use a tolerance tied to eps(); not decoded.
- Noise-level paths: a parameter whose true value is about 0 (`{xb: x1 x3}` on y1, where x3's coefficient is -0.001)
  takes 8 steps in Stata and 6 here, the estimate agreeing to 7e-9 absolute (6e-6 relative); the last `ifgnls` round's
  changes (about 1e-6) can differ in the third digit; a printed standard error can differ in its seventh digit. e()
  scalars carry the last 1e-11 of noise (rss, ll). `estat ic` prints R's table, as after every model in RCraft.
