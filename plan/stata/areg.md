# areg (2026-11-19)

Observed on Stata 19 SE with made-up data (corpus `areg.do`, 77 results to 1e-8: robust, clustered on the absorbed
variable and on another one, fw/aw/pw, factor variables, level(), predict). StatCraft's note (`areg.md`, probe 296;
ported from statcraft@b7d392e) had the rules; the layout, e() and the cluster quirks were decoded here. Translated in
`crates/statatr/src/data_r/areg.rs`.

- Slopes from the within fit: each group's weighted means removed, the weighted grand means added back, so the
  constant is ybar - xbar b and V = s^2 (X*'W X*)^-1 with s^2 = rss/df_r, **df_r = N - k - G** (k slopes, G groups).
  R-squared, rss, mss, tss are the model with the indicators; R-squared adjusted divides by df_r; F is the Wald
  test of the slopes (df_m = k); rmse = sqrt(rss/df_r). e(ll_0) is the constant-only likelihood of the within data.
  The F test of the absorbed indicators compares the pooled fit with this one (df_a = G - 1, e(F_absorb), e(p_absorb)).
- Robust: times N/df_r. Clustered: times G_c/(G_c - 1) (N - 1)/df_r, t and F on G_c - 1; sampling weights imply robust.
  **Clustered fits have e(rank) = k (one less), and with clusters other than the absorbed variable no e(sum_w).**
- Header: title and `Absorbed variable: id` on the left, the right block `Number of obs`, `No. of categories`, F, Prob > F,
  R-squared, Adj R-squared, Root MSE as `%-18s=%7s`, ending at column 78; aweights and pweights print
  `(sum of wgt is #)` (full `%18.0g`) first. The footer `F test of absorbed indicators: F(a, b) = #.### ... Prob > F = #.###`
  is right-aligned to column 78 and is not printed after robust or clustered errors.
- A factor variable's base level is a zero column of e(b) and e(V) (`1b.g`); the table shows `g |` with its levels.
- predict: xb (default, `(option xb assumed; fitted values)`), residuals (y - xb - d), `d` (ybar_g - xbar_g b - _cons) and `xbd`.
- iweights stop with r(101); no absorb() is r(198).

- **Collinear regressors** (2026-11-20, corpus `areg.do` now 137 results): a regressor the absorbed groups make collinear
  (constant within each group, or constant) is omitted wherever it stands (`areg y u x z` omits u, the first); then the
  later of a collinear set, in the order written (x2 = 2 x after x, xz = x + z after both). Each prints
  `note: u omitted because of collinearity.` before the header, a `0  (omitted)` row, and an `o.u` column of zeros in e(b)
  and e(V). df_r = N - k - G and e(rank) count only the slopes kept; e(df_m) too. Checked by sequential projection of
  the within columns, tolerance 1e-7 of the column's norm.

## Open

Refused: a factor level omitted for collinearity, every regressor omitted, interactions, `ib.` bases,
e(datasignature), predict's other statistics.
