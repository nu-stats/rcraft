# nlcom (2026-11-13)

Observed on Stata 19 SE with made-up data (probe `nl/l1.do` in the session scratchpad, its log set line by
line against RCraft's output: every nlcom table and return list identical); corpus `nlcom.do` (to 1e-8).
Translated in `crates/statatr/src/data_r/nlcom.rs`.

- `nlcom exp` or `nlcom (name: exp) (name: exp) ...`: the expressions of the coefficients, their
  delta-method variance J·V·J′. Always z and the normal interval, after regress too.
- Printed: after a blank line each expression as `name: text` with the name right-aligned in 12 (`_nl_1`,
  `_nl_2`, ... when not named), the text as typed; a blank line; a rule of 78; the coefficient table with
  the dependent variable in the stub, `Coefficient  Std. err.      z    P>|z|` and the level's interval;
  values in %9.0g in 11, 11, then z %9.2f, P %8.3f, the bounds in 13 and 12; a rule of 78.
- r(): level, N (the model's), and the matrices r(table) (9 × k: b se z pvalue ll ul df crit eform, df
  missing, eform 0), r(b) (1 × k, the row named c1) and r(V). e() is left as it was.
- RCraft takes the derivatives by central differences extrapolated (Richardson) on Stata mode's own
  translation of each expression, evaluated with the coefficients moved one at a time.
- `df(#)`: t in place of z (the headings, the critical value, r(table)'s t and df rows) and r(df_r) after
  r(level); `noheader` leaves out the expressions' lines; `iterate()` changes nothing in the results
  (probe `nl/l3.do` identical; corpus `nlcom.do`).
- Not translated: after mlogit and the models not listed below; `post` (it replaces e()); expressions of the
  data's variables.

## More models (2026-11-13; probe `nl/l2.do`)

- The same table after xtreg (fe, re), ivregress, nbreg (`_b[/lnalpha]` too), ologit and oprobit (the cut
  points too), clogit, stcox (the stub `_t`) and regress with robust errors. The values follow the model's:
  to 1e-9 after the linear models and stcox, to about 1e-7 after ologit, oprobit and clogit, where Stata
  stops iterating (corpus `nlcom_models.do`, to 6 digits).
