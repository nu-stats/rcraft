# testparm (2026-11-12)

Observed on Stata 19 SE with made-up data (probes `tp/p1.do` to `p3.do` in the session scratchpad, each
log set against RCraft's output); corpus `testparm.do`. Translated in
`crates/statatr/src/data_r/testparm.rs`.

- `testparm varlist` tests jointly that the coefficients the varlist names are 0, with `test`'s lines and
  r(): r(drop), r(df_r), r(F), r(df), r(p) after regress and xtreg, r(drop), r(chi2), r(df), r(p) after the
  likelihood models.
- The coefficients come in e(b)'s order, whatever the varlist's (`testparm z3 i.g x1` tests 2.g 3.g 4.g x1
  z3). Base levels and omitted or empty cells are left out.
- `i.g` names g's levels; a bare `g` is a continuous g, so after `i.g` in the model it names nothing.
  `c.age` and `age` both name `age`. `c.age#c.age` names the square. An interaction matches in either
  order (`i.female#i.g` finds 2.g#1.female; `c.age#i.female` finds 1.female#c.age). `i.g##i.female` is
  the main effects and the interaction. `x*` is the data's variables that match.
- `equal`: each coefficient against the first, labelled `- x1 + x2 = 0`, `- x1 + z3 = 0`. With one
  coefficient: "too few variables specified", r(102).
- Nothing matched: "no such variables; / the specified varlist does not identify any testable
  coefficients", r(111). A wildcard that matches no variable: "variable zz* not found", r(111).
- The labels: after logit, probit, poisson, nbreg, ologit and oprobit each coefficient is named with its
  equation, `[yb]x1 = 0`, `- [yb]2.g + [yb]3.g = 0` (as `test` does there); after regress and xtreg it
  is not. A continuous variable inside an interaction is written `c.age`, alone `age`.

## test's layout (shared with testparm)

- Each constraint ` ( 1)  x1 = 0` (the number in 2), a blank line, then `       F(  2,   192) =   24.21`
  (df in 3, the residual df in 5, F in %7.2f) and `            Prob > F =    0.0000` (%9.4f); or
  `           chi2(  2) =   33.48` and `         Prob > chi2 =    0.0000`. RCraft's `test` printed
  ` (1)` and `F(2, 192) =      24.21`; it now prints Stata's.

## More models (2026-11-13; probe `tp/p4.do` identical; corpus `testparm_models.do`)

- xtreg re, ivregress and stcox: chi2 and no equation in the lines; ivregress, small: F; clogit: chi2 with
  `[yb]x` lines like the likelihood models.

## After mlogit (2026-11-13; probe `rest/r3.do` identical; corpus `mlogit_tests.do`, to 6 digits)

- Each coefficient matched is tested in every outcome's equation, the coefficient first and the equations
  inside: `[1]z [2]z [3]o.z [1]w ...`; the base outcome's written `o.z` (`2o.g` for a factor) and dropped
  ("Constraint 3 dropped"). `equal` compares the coefficients in the first equation only
  (`- [1]z + [1]w = 0`). Interactions: see below.

## Interactions after mlogit (2026-11-13; probe `last/q1.do` identical; corpus `mlogit_tests.do`)

- `testparm i.g#c.w` and `c.w#i.g` (the same test): each level's interaction in every equation, the factor
  part written first whichever order was typed, `1b.g#co.w` for the base level, `o.` on the base outcome's
  parts, `c.`/`co.` on the continuous one; those Stata drops are dropped. Interactions of two continuous
  variables after mlogit are still refused.
