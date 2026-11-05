# xtregar, prais, newey, input, and replace row by row (2026-11-05)

Ported from StatCraft's decoding of Stata 19 (its `plan/stata/panel.md`, `time-series.md` and
`data-commands.md`, probes 154, 155, 229 and 334 to 418), our own work, written in R
(`data_r/xtregar.rs`, `data_r/tsreg.rs`, `data_r/more.rs`). Checked in `arpanel.do` and on the probes.

## xtregar

- rho from Prais-Winsten on the within-demeaned data (no constant), iterated until it moves by less
  than 1e-6, by the `rhotype()` rule (dw, regress, freg, tscorr, theil, nagar, onestep), one round
  with `twostep`, or `rhof()`.
- fe: Baltagi and Wu's transform after each panel's first observation, kept in single precision;
  the panel means out and the grand mean back; least squares with a constant then divided by
  1 - rho (its standard error is not). sigma_u from the panel means of y - xb; R2s, corr(u_i, Xb),
  F_f, ll as StatCraft's notes say.
- re: every observation, the first scaled by sqrt(1 - rho^2); the GLS step with each panel's theta.
  sigma_u and what follows from it agree with Stata to about 1 part in 1e8 (StatCraft's tests accept
  a few parts in 1e7: Stata's single-precision path there is not decoded).
- Least squares from the cross products, as Stata and StatCraft work it, so the residuals rounded to
  single precision fall the same way.
- Not translated yet: weights, factor variables and time-series operators, predict, margins and
  estat after it.

## prais and newey

- prais: iteration 0 is least squares; rho from the residuals on the original data (or 1 - DW/2);
  the transformed constant counts as a constant in the sums of squares; `corc` drops the first
  observation; `vce(robust)` is HC1 on the transformed data ("Semirobust"). Data without gaps only.
- newey: V = n/(n - k) (X'X)^-1 S (X'X)^-1, S the White sum plus the Bartlett-weighted cross products
  of observations l time units apart.

## input

Into empty data: float unless a type says otherwise (`double(a b)` types a group), strings as
written, `.` and `.a` missing. Declined: a value a numeric variable cannot read (Stata skips the
line), lines with the wrong number of values, input into data already in memory.

## replace row by row, and storage

- `replace x = ... L.x ...` or `x[_n-k]` with random draws: Stata draws for each row as it goes.
  RCraft repeats the replace until nothing changes, with Stata's generator put back before each
  round, so each round draws the same numbers and ends where one round leaves it.
- `x[_n-k]` (earlier rows) now runs that way too; `x[_n]`, `x[_n+k]` and `x[_N]` read rows not yet
  replaced, as R does; `x = x[1]` alone gives every row the first row's value. Other own subscripts
  are declined.
- `replace` keeps the storage type `generate` (or `input`) gave a variable: a float stays a float,
  each row rounded before the next reads it. A command that makes the variable again forgets it.

## Found on the way

- `length()` and `strlen()` count bytes in Stata (`length("café")` is 5); they were R's characters.
- `subinstr()` works on bytes in Stata; R's `gsub(fixed = TRUE, useBytes = TRUE)` too, so `char(128)`
  to `char(255)` can be searched for and put in there (one package's text cleaning replaces old Windows
  bytes that way). Elsewhere a single byte is still declined, and `char(0)` everywhere, since R's text
  cannot hold it.
- `capture do f` and `capture run f`: RCraft runs each line of f under capture, where Stata stops f at
  its first error; when a line in f is not translated, `_rc` after it is not known (probe 590).
  `capture include` stops with r(199) in Stata 19 and is declined.
