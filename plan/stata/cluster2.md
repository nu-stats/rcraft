# cluster2 (Petersen's add-on), observed in Stata 19 (2026-11-06)

Mitchell Petersen's `cluster2.ado` (Kellogg; not on SSC) is installed on this machine in
`C:\Users\jstow\ado\personal` with Stata's `copy` from his web page, at the user's go-ahead. Only
its results were read, never the file. One replication package (XURXZN `Tables.do`) runs it 88 times,
always `cluster2 y x, fcluster(a) tcluster(q)` followed by `outreg2 ..., tstat adjr2`.

## What it does (made-up data, matched to 14-15 digits)

- b: least squares with a constant on the rows with no missing value in y and x.
- V = Vc(fcluster) + Vc(tcluster) - Vc(cell), where Vc(g) = (X'X)^-1 (sum over groups of X'e e'X)
  (X'X)^-1 G/(G-1) (N-1)/(N-K) and a cell is a fcluster-tcluster pair. With one row per cell, the
  last is the robust variance N/(N-K) (the same formula with G = N).
- e(F) and e(df_r) are those of the last fit it runs: with one row per cell, `regress, robust`
  (df_r = N-K); else clustered by cell (df_r = cells - 1). F is the Wald test of the slopes with that
  fit's variance, not V's. The t tests and intervals use that df_r.
- A diagonal of V below 0 is kept in e(V); the table shows `.` for that standard error.
- e(): N df_m df_r F r2 rmse mss rss r2_a ll ll_0; cmd "cluster2", depvar, method "2-dimension
  clustered SEs", title "Linear regression with clustered SEs", properties "b V"; b and V. No e(sample).
- Header "Linear regression with 2D clustered SEs" with Number of obs, F, Prob > F, Number of
  clusters (a) and (q), R-squared, Root MSE; footer "SE clustered by a and q", with
  " (multiple obs per a-q)" when cells repeat.
- Errors: weights r(101) "weights not allowed"; no tcluster() r(198) "option tcluster() required";
  `robust` r(198) "option robust not allowed".
- A missing value in a cluster variable: b uses every row with y and x (399), while the clustered
  parts drop the rows with the missing cluster value. RCraft stops the R script there.

Checked in `cluster2.do` (Stata's results recorded with the add-on installed).
