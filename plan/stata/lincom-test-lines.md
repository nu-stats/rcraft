# test's and lincom's lines, lincom's table (2026-11-13)

Observed on Stata 19 SE with made-up data (probes `last/q2.do`, `last/q3.do`, `last/q4.do` in the session
scratchpad, set line by line against RCraft's output: identical); corpus `lincom_post.do` (to 6 digits: the likelihood models).
Translated in `crates/statatr/src/data_r/post.rs`.

- Each constraint's line ` ( 1)  ...`: every coefficient on the left in the order typed (both sides
  gathered, the right side's negated, the same name added up), the constant on the right: `test x = w` is
  `x - w = 0`, `test 2*x = 3*w + 1` `2*x - 3*w = 1`, `test x = -w` `x + w = 0`, `test -x + w = 0.25`
  `- x + w = .25`, `test 1.5*x - 2 = w` `1.5*x - w = 2`, `test x = 2*x` `- x = 0`, `test x + x = 1`
  `2*x = 1`. A weight of 1 is left out; numbers in %9.0g (`.5`); `_b[x]` is `x`.
- The names carry the dependent variable's equation after logit, probit, poisson, nbreg, ologit, oprobit
  and clogit (`[yb]x - [yb]w = 0`), none after regress, xtreg, ivregress, stcox and margins or nlcom,
  post. After mlogit a name without an equation is the first equation's (`[1]x`) and the base outcome's
  are omitted ones (`[2]x - [3]o.x = 0`).
- lincom: the same line for the combination with `= 0` and its constant moved right (`lincom x + w - 1` is
  `x + w = 1`, `lincom 2 + x` `x = -2`); the estimate includes the constant. Then a blank line, a rule of
  78 and the table: the dependent variable in the stub (blank after margins, post; `_t` after stcox),
  `Coefficient  Std. err.` or, with or/irr/rrr/eform, `Odds ratio`, `IRR`, ... right-aligned in 11 then
  `   Std. err.`; t after regress (and margins after regress), z otherwise; the row `(1)` with the values in
  %9.0g (11, 11), t %9.2f, P %8.3f, the bounds in 13 and 12; a rule of 78. Only base levels:
  `(1) |          0  (omitted)`.
- After stcox lincom shows `Coefficient` (not the hazard ratio) unless asked.
- A constant with `or`: "additive constant term not allowed with or option", r(198).
