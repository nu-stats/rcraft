# xtreg, fe: e(r2) and the sums of squares (2026-11-09)

TZEN38 rounds `e(r2)` after `xtreg, fe` into a macro for `putexcel` (72 lines, and 72 more that
used them). Observed in Stata 19 on made-up panels (20 panels of 10, an `if`, `i.` levels);
`xtreg_r2.do` records them.

- `e(r2)` is the within R-squared, `1 - rss / within total` (the digits of `e(r2_w)`).
- `e(rss)` the within residuals' sum of squares; `e(mss)` the within total less `rss`; `e(tss)` the
  total about the grand mean (not the within total).
- `e(r2_a) = 1 - (1 - r2) (N - 1) / df` and `e(rmse) = sqrt(rss / df)`, where df is N - k - the
  panels, or N - k - 1 with `vce(robust)` or `vce(cluster)` (on the panel or on groups of panels:
  the same), k the slopes kept.
- `re` and `mle` leave `e(r2)` missing.
- Not kept yet (refused as before): `e(df_m)` (21 plain but 1 with robust errors on the same data,
  rule not known), `e(df_a)`, `e(df_b)`, `e(rank)`, `e(sigma)`, `e(Tbar)`, `e(Tcon)`; and these after
  `be`.
