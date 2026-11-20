# constraint and cnsreg (2026-11-19)

Observed on Stata 19 SE with made-up data (corpus `cnsreg.do`, 242 results to 1e-8). StatCraft's note
(`constraints.md`, probe 530; ported from statcraft@b7d392e) gave the formulas and the layout; the forms and
errors below were decoded here by running Stata. Translated in `crates/statatr/src/data_r/lsfam.rs`
(batch E of the StatCraft port).

## constraint

- `constraint [define] # text` keeps the text as typed (inner spaces kept, ends trimmed), 1 to 1999; 0 is r(198)
  "constraint number must be between 1 and 1999"; a number above 1999 is accepted without a message
  (refused here); the text is checked only when a command uses it, and an empty text or one without `=` is stored
  (rc 0). A quoted text is r(198) "invalid syntax".
- `constraint list` and `dir` (same) print `%6d:  text`, all of them or the numlist's (an undefined number prints
  nothing). `drop numlist|_all` passes over undefined numbers; a numlist outside 1 to 1999 is r(125).
- `constraint get #` leaves r(defined) 1 and r(contents), or r(defined) 0 alone; `free` leaves r(free), the
  highest number not in use, as text. Only these two change r(); define, list and drop leave it.

## cnsreg

- Each constraint is read as a linear equation in the model's coefficients (names, `_cons`, `_b[x]`, numbers, `+ - * /`,
  parentheses): the coefficients are the left side minus the right, the constant moves to the right (`x2 = x1` is
  `- x1 + x2 = 0`, `3 = x1` is `- x1 = -3`). A name outside the model (or `_cons` with `noconstant`) is r(111), a product
  or power of coefficients r(131), `[eq]x` r(303); each prints `(note: constraint number N caused error r(rc))` and is
  dropped; none left is r(111) "matrix e(Cns) not found". Undefined numbers are passed over silently.
- Equal constraints (after the move, equal coefficients and right side) count once; proportional ones (`x1 = x2` with
  `2*x1 = 2*x2`), `x1 = x1`, and contradictions are r(412) "redundant or inconsistent constraints".
- Fit: b = b0 + T a (b0 the smallest b meeting R b = r, T an orthonormal basis of what is left free), weighted least
  squares on X T. df_r = N - (k - m). aweights are scaled to sum N for the sums of squares and
  print `(sum of wgt is #)` first; **pweights are not scaled** (rss, rmse and e(ll) come from the raw weights; robust
  is implied); fweights give N = their sum. iweights are refused here.
- Robust: the sandwich of w e z times N/df_r. Clustered: times G/(G - 1) (N - 1)/df_r, t and F on G - 1, the header's
  Prob > F from them, but **e(p) from F on the residual degrees of freedom N - (k - m)**, and e(V_modelbased) kept.
- F is the Wald test that the slopes are zero given the constraints: e(df_m) the rank of what the constraints leave of
  the slopes. When the constraints contradict zero slopes (a slope fixed at a value, `2*x1 - x2 = x3 + 1`) there is no F,
  no Prob > F, and e(df_m), e(F), e(p) are missing. e(rank) = k - m. e(ll) is the normal log likelihood at rss/N.
- Header: `Constrained linear regression`, the right block (`Number of obs`, F, Prob > F, `Root MSE` with `%.4f`)
  ending at column 78 with the width of its widest value (6 at least), a blank line, ` ( 1)  x1 - x2 = 0` lines (model
  order, a coefficient of 1 not written, `.5*x1`), the clusters line, the table; a coefficient the constraints fix
  shows `(constrained)` with no standard error. `cnsreg` alone shows the last one again (level() allowed).
- e(Cns) is m x (k + 1): the constraint rows and `_r`, rows `r1`..., columns the coefficients, no `_cons` with
  `noconstant`. Macros: cmdline, (vcetype, vce, clustvar), cmd, title, predict `tobit_p`, depvar, marginsok, properties.
- `predict` (xb, residuals) and `test` work. **Residuals are y - float(xb)** (Stata's `tobit_p` takes the stored
  float prediction), so `predict r, residuals` differs from a double xb by 1e-8 relative.


## constraints() in logit, probit, poisson and logistic (2026-11-20)

Observed on Stata 19 SE with made-up data (corpus `ml_cns.do`, 232 results to 1e-6, and the printed output line
for line: iteration logs, header, constraint lines, table). StatCraft's note (`constraints.md`, probe 530) had the fit
and the header; the starts, the Wald test's degrees of freedom and the e() changes were decoded here. Translated in
`crates/statatr/src/data_r/ml.rs` (`stata_ml_cns`).

- The constraints are read as for cnsreg; `[y]x` with the outcome's name is the coefficient x. The fit is the
  likelihood over a in b = b0 + T a, by Stata's Newton-Raphson (`stata_ml_path`), the variance T (T'(-H)T)^-1 T'
  (robust and clustered: the usual sandwich around it). A fixed coefficient has a variance of exactly 0.
- **Start:** the unconstrained start projected onto the constraints, b0 + T T'(s - b0). logit and probit: the
  constant-only fit (iteration 0 is its likelihood; `_cons = 2` gives -53.07712 on the corpus data). poisson: its
  least-squares start after one full Newton step (as without constraints), weighted for fweights.
- One halved step is **not** marked in Stata's log (`constraints(5)`: iteration 2 at half the step, no `(backed up)`);
  the log is printed then, and left out where a step was halved twice or -H was not concave.
- Header: `Wald chi2(q)` with `Log likelihood = ` beside `Prob > chi2`, no pseudo R2; a blank line, then
  ` ( 1)  [b]x1 - [b]x2 = 0` (as `test` writes them, with the equation), then the table. A fixed coefficient's row
  is its value and `(constrained)` (with `or`/`irr` its exp).
- The Wald test is of the slopes on their variance with the fixed directions left out: q is the rank left
  (`x1 = x2` with `x3 = -.2`: chi2(1)), even where the slopes all zero contradicts the constraints (unlike cnsreg's F).
- e(): rank k - m, df_m q, chi2 and p the Wald test, chi2type `Wald`; no e(ll_0); logit and probit no e(r2_p),
  poisson e(r2_p) missing; e(Cns) after e(V) (columns `b:x1 ... b:_cons _Cns:_r`).

Refused: constraints() with exposure(), offset(), noconstant, iweights, or a regressor omitted for collinearity; those
that leave no constraint, are redundant or fix every coefficient. Not checked: the log with pweights in poisson
(started from the fit, so the log is left out).


## cnsreg with factor variables (2026-11-20)

Observed on Stata 19 SE (corpus `cnsreg.do`, 523 results to 1e-8, and the printed output line for line). Translated in
`crates/statatr/src/data_r/lsfam.rs`.

- `i.g` and `ib#.g` are taken. The columns are Stata's (regress's e(b) code names them on a fitted layout): the base level
  is a column of e(b) and e(V), zero, and a constraint of its own, `1b.g = 0`, comes **first** in e(Cns) (one row per
  base level, in column order, before the user's rows). It is not a line of the header; it counts in the degrees of freedom
  (df_r = N - (k - m) with k the columns including the base ones, m all the rows) and in e(rank) = k - m.
- A constraint may name `1.g` for the base level: it reads `1b.g` (so `1.g = 0` is the base constraint again, equal rows count
  once and Stata prints nothing for it), and with `ib2.g` the constraint `2.g = 3.g` reads `2b.g - 3.g = 0`, which fixes 3.g at 0.
  `_b[2.g]` works; the header shows the name as written in the column (`2b.g - 3.g = 0`).
- A coefficient the constraints fix **at 0** prints `0  (omitted)` (a level, and a plain variable too: `x = 0`), one fixed at
  another value `(constrained)`. The table hides the base row.
- The residual after predict is in double with factor variables (plain cnsreg's is `y - float(xb)`).
- A factor collinear with another (hh a function of g) makes Stata add a constraint row of its own for the omitted column; not
  translated (the corpus avoids it). Clustering on the factor's own variable gives a rank-limited variance whose e(rank) and
  F differ; not matched.

Refused: noconstant with factor variables (Stata prints no base row and no header then), interactions and powers.

## Open

Refused: interactions and operators in the model, factor variables with noconstant or collinear with another, iweights, `collinear`, `mse1`, a constraint with more than one `=` or
`==` (Stata reads `x1 = x2 = 2` as `x1 - x2 = 0` and `x1 x2 = 0` as `x1 = 0` with x1 omitted: odd, not followed),
every coefficient fixed, numbers above 1999, and the `constraints()` option of other models (sspace, spregress; logit, probit and poisson: above).
