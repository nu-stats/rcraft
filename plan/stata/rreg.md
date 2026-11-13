# rreg (2026-11-13)

Observed on Stata 19 SE with made-up data (probes `rreg/r1.do`, `p2.do`, `p3.do`, `p4.do` in the
session scratchpad: five datasets of 25 to 200 rows, one and two regressors, doubles exported at full
precision) and YFBPT6's macro.do; corpus `rreg.do`. Its code was not read: every rule below was
fitted to its iteration logs, e(b), e(V), e() and genwt() weights.

## The fit

- Least squares on the rows with no missing value; Cook's distance from it; rows with D > 1 are left
  out with no message (N then counts the others; genwt() leaves them missing).
- Huber iterations: weights w = min(1, 2M/|r|), M = median(|r - median(r)|) of the current residuals
  (2M matched the first iteration's printed change to 1e-8). Each iteration fits weighted least squares
  and prints "Maximum difference in weights" (the largest |w_new - w_old|, %10.0g). The Huber phase
  ends when that change is below 0.05: Stata went on at .0545, .0760, .1440, .2363 and stopped at
  .0419, .0376, .0207, .0176, .0116, .0100, .0035 and 0 (all weights 1).
- Biweight iterations: u = r/s, s = M/0.6745, w = (1 - (u/4.685)^2)^2 for |u| < 4.685, else 0, until
  the change is below 0.01 (tolerance()'s default). The iteration numbers run on from the Huber
  phase. The logs and coefficients of all five datasets match to 12 digits.

## The variance (pseudovalues)

- V = rmse^2 (X'X)^-1, X'X unweighted (Stata's V / (X'X)^-1 was one constant, e(rmse)^2, in every
  case).
- rmse^2 = sum((w_i r_i)^2) / (n - p) / m^2 * (1 + (p / n') (1 - m) / m)^2, where w are the final
  weights, r the final residuals, s the scale of the last biweight iteration (M/0.6745 of the
  residuals the final weights came from), m the mean over all n rows of psi'(r/s) =
  (1 - (u/c)^2)(1 - 5(u/c)^2) for |u| < c = 4.685, and n' the number of rows with a nonzero weight.
  Exact to 1e-15 on all five datasets; the forms with n in place of n', or with Huber's
  var(psi')/m^2, were 1e-4 to 3e-3 off.

## e()

- N, sum_w (= N), df_m, df_r = N - p, F (the Wald test of the slopes, b' V^-1 b / df_m), rmse,
  mss = F df_m rmse^2, rss = rmse^2 df_r, r2 = mss/(mss + rss), r2_a = 1 - (1 - r2)(N - 1)/df_r,
  rank; cmd, cmdline, title "Robust regression", marginsok "XB default", predict rreg_p, depvar,
  properties "b V", model "ols"; b, V, beta (each slope times sd(x) over sqrt((mss + rss)/(N - 1))).

## The output

- A blank line, the iteration lines ("%8s iteration %d:  Maximum difference in weights = %10.0g"),
  a blank line, "Robust regression" with "Number of obs     = %10s" (commas), "F(%3d,%10d) = %10.2f",
  "Prob > F          = %10.4f" from column 49, a blank line, and the regress-style coefficient table
  with t statistics on df_r.

## YFBPT6

- macro.do:47 `rreg est_o3_c trust_cc if sim=="1"` (9 rows): its log, header and table come out as
  Stata's; robreg on the next line replaces its e().
