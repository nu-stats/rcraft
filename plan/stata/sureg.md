# sureg and mvreg (2026-11-19)

Observed on Stata 19 SE with made-up data (corpus `sureg.do`, 240 results to 1e-8, weights, small, dfk, isure and the
equation tests included). StatCraft's notes (`sureg.md`, `mvreg.md`, probes 266, 267, 281; ported from
statcraft@b7d392e) had the formulas; the weights, the correlation layout, the error messages and the single-equation
forms were decoded here. Translated in `crates/statatr/src/data_r/sureg.rs`.

## mvreg

- `mvreg y1 y2 = x1 x2 [if] [in] [fweight aweight] [, noconstant corr level() noheader notable]`. Each equation by
  (weighted) least squares on the same rows; aweights scaled to sum N, fweights count N. e(Sigma) = E'WE/(N - k),
  V = Sigma kron (X'WX)^-1, t tests on N - k. F per equation is its own regress F (slopes), R-squared centred
  (uncentred with noconstant, F on k).
- Tables: `Equation  Obs  Parms  RMSE  "R-sq"  F  P>F` (74 rule; name 12, Obs 12, Parms 8, RMSE 12 %9.0g, R 10 %.4f, F 11
  %9.0g, P 9), then the coefficient table with an equation heading per equation. `corr`: the lower triangle of the
  residual correlation, **labels right-aligned in the widest name, columns max(9, width + 2) wide**, then the Breusch-
  Pagan `chi2(M(M-1)/2) = N sum r^2` line (with an en dash). `noheader` drops the first table, `notable` the second.
- pweights and iweights stop with `pweights not allowed` / `iweights not allowed`, r(101); an unknown option r(198).
  One dependent variable works but prints a plain regress-like table (refused here).
- e(): k df_r k_eq N rank, then the macros, with F rmse p_F r2 as space-led lists (one value per equation).
- predict (xb, residuals, `equation(name|#k)`; the default prints `(option xb assumed; fitted values)`) and
  `test [y1]x1 = [y2]x1` (F on N - k) work.

## sureg

- `sureg (y1 x1 x2) (y2 x1 x3) [if] [in] [fweight aweight] [, small dfk isure corr level() nolog noheader notable]`.
  **No `noconstant`** (r(198) `option noconstant not allowed`). Two-step feasible GLS; Sigma from the equations' weighted
  least-squares residuals over N (or sqrt((N - ki)(N - kj)) with dfk); b and V from X'(Sigma^-1 kron W)X; e(Sigma) the first
  step's (isure: the last used). isure repeats until max |db|/(|b| + 1) < 1e-6.
- R-squared centred, rmse sqrt(rss/N) (N - k with small or dfk), chi2 per equation the Wald test of its slopes,
  small: F = chi2/df_m, p from F(df_m, N M - k) and t on N M - k. e(ll) = -N/2 (M (1 + ln 2 pi) + ln|E'WE/N|).
- Header: a blank line, the title (`, iterated ` with a trailing space for isure), a 78 rule, `Equation Obs Params RMSE
  "R-squared" chi2 P>chi2` (F and P>F with small), the rows, a rule; **with fweights one more blank line** before the
  coefficient table; `dfk` adds Stata's two-line note after it; `corr` columns are max(8, width + 2) wide.
- The same dependent variable in two equations works in Stata (the second named `2y1`): refused here. One equation
  works too (refused). pweights and iweights are r(101).
- predict and test as mvreg's; test is chi2 unless small.

## Open

Refused: isure without nolog (the iteration log), equation names, the `y1 y2 = x` form, constraints(), factor variables,
one equation, a repeated dependent variable. e(dfk2_adj) is N here (Stata's has 1e-13 of noise).
