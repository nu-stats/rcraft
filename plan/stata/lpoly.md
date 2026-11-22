# lpoly (2026-11-22)

Ported from statcraft@09a0ba7 (its `plan/stata/lpoly.md`, probes 552 and 553), then checked again in
Stata 19 SE on made-up data and extended; our own work, written in R (`data_r/lpoly.rs`). Corpus
`lpoly.do` (see the corpus) and `lpoly_bandwidth.do` (the `%21x` bandwidths).

## Rule

- The smooth at x0 is the intercept of a weighted least-squares fit of y on 1, (x - x0), ..., (x - x0)^p
  with weights w K((x - x0) / h) / h. Degree 0 by default; a degree that is not a whole number is read as 0
  and a negative one stops with r(198). Degrees above 5 were not observed and are not translated.
- Kernels as `kdensity`'s (`npregress.rs`'s helper holds them). `epan2` exactly; the others by prefix, at
  least 2 letters for epanechnikov and biweight, 3 for cosine, gaussian, parzen, rectangle and triangle;
  case matters; triweight and anything else: "invalid kernel function", r(198).
- Grid: min(N, 50) points evenly from min x to max x. `n(k)` for a whole k of 2 or more; `n(1)` and a k that
  is not a whole number give the default; `n(0)` or less: r(198). `n()` above N prints `(n() set to N)`.
  `at(var)` uses that variable's values (every row, whatever `if`/`in`) and ignores `n()`.
- `generate(gx gs)` makes the grid and the smooth, in the first rows, the rest missing; with `at()`,
  `generate(gs)` makes the smooth at each row, and two names also make the grid (the at values). Three
  names: r(198). All doubles, `%10.0g`, labels "lpoly smoothing grid", "lpoly smooth: y", "lpoly SEs".
  `se(newvar)` needs `at()` or `generate()` (r(198) otherwise). Names already in use: r(110); `at()` of a
  variable that is not there: r(111).
- Weights: aweights are scaled to sum to N; fweights make N their sum (non-whole: r(401); negative:
  r(402)); pweights and iweights: "not allowed", r(101). No rows: r(2000). r(): degree, ngrid, N, bwidth,
  pwidth (only with `se()` or `ci`) and the macro kernel.
- `bwidth()`, `pwidth()`: positive, else r(198).

## The default bandwidth and the standard errors

As StatCraft's note: for an even degree the odd degree above is used; a global polynomial of degree p + 3
gives sigma^2 (weighted residual sum of squares over N) and the (p + 1)th derivative, whose square is
integrated exactly over the middle 90% of the range; h = C(p, K) [sigma^2 (0.9 range) / (N integral)]^(1/(2p+3)),
C by the trapezoid rule on 101 points over +-sqrt(5) (Epanechnikov), +-5 (Gaussian) or +-1.25 (the rest).
Standard errors: the sandwich times sigma^2(x0), `var()` or the degree p + 2 local fit's residual variance
at the pilot bandwidth (1.5 times the rule, or `pwidth()`).

## Found here, not in StatCraft's note

- A window with one point gives a missing smooth; with two or more it is fitted, and when there are fewer
  points than coefficients the columns are picked by the largest remaining pivot (three points and a cubic
  leave out the slope, not the cube; two points give the line through them). The standard error is then
  missing.
- With constant x the rule of thumb is missing (r(bwidth) is `.`) but a given `bwidth()` works.
- `lpoly y x z` with z in the data, `level()` outside 10 to 99.99, `var()` below 0 and degrees above 5 were
  not observed and are refused or left to run when the do-file runs.

## Open

- A window with exactly p + 3 points: Stata fills the standard error with rounding noise (0 or 1e-6 in
  runs here); RCraft shows missing.
- The graph is ggplot2's: the data, the smooth, a band with `ci`; Stata's own look and its scatter and line
  options are not carried over.
- The rule-of-thumb bandwidth agrees to 1e-6 (Stata's own regression of the global polynomial loses digits
  at degree 4 and 5; RCraft fits it in scaled coordinates).
