# nlsur (2026-11-19)

Observed on Stata 19 SE with made-up data (corpus `nlsur.do`, 279 results to 1e-6; the iteration log compared by eye
on four scratch do-files holding about 35 `nlsur` lines in all, line for line). StatCraft's note (probe 537; ported from
statcraft@b7d392e) had the step rule and the formulas; what changed here is the stopping rule, which variance matrix
Stata keeps, the constant, the rows used, the `nls` weighting and the layout details below. Translated in
`crates/statatr/src/data_r/nlsur.rs`.

This note is what the earlier "on hold" entry lacked: the log is decoded. Stata's log is the Gauss-Newton path itself,
so it reproduces exactly once the step and the stopping rule are right.

## Rule

`nlsur (y1 = <sexp>) (y2 = <sexp>) ... [if] [in] [, nls fgnls ifgnls nolog level() iterate() eps() delta() noconstant
initial()]`. A parameter is `{name}` or `{name = #}`; names are shared across equations, kept in order of first
appearance (not sorted, unlike `nl`), and the start is 0 when none is given. `e(sexp_#)` is the expression with a
space before each `{` and after each `}`, runs of spaces made one.

**Gauss-Newton, as Stata takes it.** Residuals stacked equation by equation, whitened by Sigma (the identity in the NLS
stage). Derivatives are forward differences, each parameter moved by d = 4e-7 |b| (4e-7 itself at 0), d taken as
stored. A step solves the normal equations over the parameters whose columns are neither zero nor a combination of
earlier ones' (swept in order, tolerance 1e-13 of the diagonal); the others do not move. The step is halved while it
raises the criterion, never stepped further (unlike `nl`). The log shows the criterion after each step, so "Iteration 0"
is the value after the first step (`%9.0g`, right-aligned in 8).

**Stopping.** After a step, done when no parameter moved by more than `eps()` (1e-5) **relative to its old size**,
`|db| / |b|` (a parameter that did not move counts as 0). This is not `|db| / (|b| + 1)`: that form agrees on most
paths and stops one iteration early on others (found on a fit whose last step was 5e-6 against a size of 0.19).
`iterate()` caps the steps per stage; running out ends the command with r(430) after the table (e(converged) = 0).

**Stages.** NLS (log "Residual SS"), then FGNLS with Sigma = R'R / N from the NLS residuals ("Scaled RSS"). `ifgnls` goes
round again from the last estimates with Sigma from the last residuals, each round showing `FGNLS iteration r:`, its own
log, `Parameter change` (max |db| / (|b_new| + 1), here the new size and the +1) and `Covariance matrix change` (max
relative difference of the Sigma the new residuals give against the Sigma used, with the same +1 form), both `%9.2e`,
stopping when both are at most 1e-5. With `nolog` the lines are `Calculating NLS estimates ...`, `Calculating FGNLS
estimates ...` and, for `ifgnls`, `FGNLS iteration r ...`, no blank lines between.

**`nls` method.** e(Sigma) is diagonal (each equation's variance from the NLS residuals). The estimates are the NLS
estimates when each parameter belongs to one equation. **With a parameter shared by two equations the `nls` estimates are
not the stacked least squares**: they equal each equation weighted by 1/variance of the stacked fit's residuals (checked
to 8 digits on one model), while the log still shows the stacked sum of squares. How that weighting reaches the variance
is not known, so RCraft stops on that form.

**Variance.** FGNLS: (J'(Sigma^-1 kron I)J)^-1 with the Sigma it used; NLS: the sandwich (J'J)^-1 J'(Sigma kron I)J
(J'J)^-1 (for the separate-parameter case the diagonal Sigma gives the same). Which Jacobian: for FGNLS the one at the
**start of the last Gauss-Newton step**; for NLS the one at the final estimates. Using the final one for FGNLS is off by
1e-5 where the last step was loose, and the other way round for NLS (decided on a dozen fits, six of each kind). The
parameters left out of the step are `(constrained)` rows with zero variance. e(Sigma) is the Sigma used, except under
`ifgnls`, where it is the one the final residuals give. e(ll) = -N/2 (M (1 + ln 2 pi) + ln |Sigma_final|) with Sigma_final
from the final residuals.

**Rows.** The rows with every dependent variable present (and `if`/`in`); `(obs = N)` is printed first (commas from
1,000), then a blank line. A missing value on the right-hand side is **not** a reason to drop a row: the run stops with
"could not evaluate equation q" (r(480)) after the `Calculating NLS estimates` line.

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
parameters), k_eq; macros cmdline, cmd, predict (`nlsur_p`), estat_cmd, vce (`gnr`), method (`nls`, `fgnls`, `ifgnls`),
constants, type (1), params, depvar, then params_i and depvar_i from the last equation down, sexp_i from the last down,
properties; matrices b (colnames `a1:_cons`, row `y1`), V, init (row `r1`, colnames the parameter names) and Sigma
(dependent variable names).

**predict.** `predict [type] newvar [if] [in] [, yhat residuals equation(#k)]`: fitted values, or y - fitted, of equation
k (default 1); a bare `predict` prints `(option yhat assumed; fitted values)`. `equation(name)` is r(198)
("misspecified"), a number past the last equation an error. `_b[a:_cons]` and `_se[a:_cons]` read e(b) and e(V) (`_b[/a]`
is accepted by Stata and not by RCraft).

## Cases

- Zero starts on `{a2} + {b2}*exp({c}*x2)`: the b2 and c columns are zero or dependent at the start, only a2 moves,
  b2 and c end as `(constrained)` at 0, e(rank) = 3, Sigma's off-diagonal is the NLS one.
- Shared parameters under FGNLS (default): the NLS stage is the stacked least squares; FGNLS with the full Sigma follows.
- `iterate(2)`: both stages stop after two steps, the table is printed, then r(430). RCraft prints the log and stops with
  a message before the table (the standard errors of such a run came out different in Stata; the rule was not found).
- `ifgnls, iterate(1)`: Stata ends with r(0) and e(converged) = 1; RCraft refuses the combination.
- Two equations with the same dependent variable work (the table is headed `y1`).
- `delta(1e-4)` moves the iteration values in the seventh digit from RCraft's; `delta()` is refused.

## Open

Refused, with a message: weights (fweights and aweights print `(sum of wgt is #)`, pweights give a robust table),
`vce()`, `variables()`, `parameters()` (Stata: "not allowed" outside the evaluator form), `hasconstants()`, the
function-evaluator form, `{name: varlist}` (Stata names the parameters `xb_x1`), `delta()`, `nls` with a shared
parameter, `ifgnls` with `iterate()`, factor variables, a parameter named as a variable. The cap on steps (100) is an
assumption (the real default was not reached by any fit tried). `estat`, `test`, `lincom`, `nlcom` and `margins` after
`nlsur` are not translated. e() scalars carry the last 1e-11 of noise (rss, ll).
