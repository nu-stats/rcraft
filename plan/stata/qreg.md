# qreg (2026-11-19, ties and the iteration log 2026-11-19)

Observed on Stata 19 SE with made-up data (corpus `qreg.do`, 234 results to 1e-8: continuous regressors, then dummies and
rounded data where the minimum is not unique, with and without `nolog`, fweights). StatCraft's note (`qreg.md`, probe 245;
ported from statcraft@b7d392e) had the variance and layout. Black box only: Stata's log numbers and `e()`, no ado or help
source read. `crates/statatr/src/data_r/qreg.rs` (`stata_qreg_ir`, `stata_qreg_solve`) is the result.

## Stata's path (found from its iteration log)

`qreg y x` prints `Iteration 1:  WLS sum of weighted deviations = a` then `Iteration k:  Sum of abs. weighted deviations = s_k`
lines. Each `s_k` is the objective at a **vertex** (a line through k rows), one pivot per line, found by listing every vertex of a
small data set and matching the logged numbers. Rebuilt in R:

1. **WLS step** (the `a` value reproduces to 8 digits on all 140 probes): OLS fit `b0`, residuals `r`; rows with `|r| < 1e-10 max(1, |y|)`
   are dropped (weight 0, also from the linear term below); `w_i = fw_i / |r_i|`; solve
   `X'WX b = X'Wy + (2q - 1) X'(fw)` (the check loss rho_q(r) = |r|/2 + (q - 1/2) r, so q = .5 is plain reweighted LS and the
   linear term is the q shift; at q = .5 `a` is the one-step IRLS value). The printed value is the check-loss sum at this `b`.
   Only this one step is taken (more steps were tried: they do not match `a`).
2. **First vertex**: the k rows with the smallest weighted deviation rho_q(r_i) of that WLS fit (not |r_i|: that matched on 5 of 9 quantiles,
   rho_q on all 9), skipping rows that make the set dependent.
3. **Descent over vertices**: for each of the 2k edges (increase or decrease the residual of one basic row) the slope is
   `2 (w_h c_h + sum over the other rows of w_i term_i)`, `term` the rho_q slope of row i along the edge (residual side `sr`).
   Pricing is the slope **per unit of the basic row's weight** (`slope / w_h`: with fweights this is the weighted LAD turned into an unweighted
   one by scaling rows, and it is what makes weighted fits match: dividing by nothing gave 4 of 7, by w_h 7 of 7), the most negative; ties
   (within 1e-9) go to the **last** edge. Along the edge it passes every breakpoint (a row whose residual reaches zero) until
   the cumulative slope reaches **zero or above** (`>= 0`, with the slope summed in plain double precision in row order: the exact-zero
   stop at a flat piece depends on that rounding, and plain `sum()` (long double) lost 2 of 14 on one data set), the entering row the one at
   that breakpoint, ties in breakpoint order by lowest row.
4. **Rows at a residual of zero** (duplicated points: dummies, rounded data) keep a **side**: positive at the start, flipped when a
   breakpoint is passed, the leaving row takes the side it left on. The slope uses that side, which is why Stata makes a pivot that
   changes nothing (`42` then `42`) and then stops: it is the same degenerate step.
5. **`note: alternate solutions exist.`** is printed after iteration k when the stop in step 3 has a slope of exactly zero (the flat piece).
   Reproduced on 128 of 140 probe runs (same positions); the other 12 are runs whose path already differed.
6. The count is as in Stata (`Iteration 1:`, one space less after `Iteration 10:`: `%-13s` then a space), numbers `%10.0g`, a blank line before
   the first WLS line and between it and the iterations.

## Measured on Stata 19 (made-up data, 7 quantiles, with and without fweights)

140 runs on 10 data sets (continuous x with two regressors; one dummy; two dummies and a continuous; rounded y; three levels as dummies;
2x2 cells; integer y; a small n = 18 set), first 56 used to find the rules, 84 held out:

| | runs | matched |
|---|---|---|
| WLS value `a` | 140 | 140 |
| final e(b), where Stata finishes | 126 | 123 |
| the logged objective sequence | 140 | 114 (all 26 misses are tied data) |
| the whole log (sequence and the notes) | 140 | 111 |
| continuous regressors (sequence) | 14 | 14 |

The three e(b) misses (one dummy at q = .1; two dummies and a continuous regressor at q = .6; a dummy and a continuous regressor at q = .25 with fweights) end at
another optimal vertex of the same flat face; those are data where the flat piece's end comes down to rounding noise in Stata. Not in the
corpus.

## Header and variance (unchanged from the first round)

- `Raw sum of deviations` is about the sample quantile; `Min sum of deviations`; `Pseudo R2` = 1 - min/raw. e(df_m) k, e(df_r) N - k - 1.
  The sample quantile (e(q_v), the "about" value) is the **floor(q (N + 1))-th** smallest y (weights count): probed on y = 1..40 over 24
  fractional parts, and the earlier "smallest y with share q at or below it" is wrong for non-integer q N (n = 18, q = .4: Stata 6, 7 before).
- V = q (1 - q) s^2 (X'WX)^-1 with s = xbar'(b(q + h) - b(q - h))/((q + h) - (q - h)) and
  h = N^(-1/3) z^(2/3) [1.5 phi(Phi^-1(q))^2 / (2 Phi^-1(q)^2 + 1)]^(1/3), **z from level()**. t tests on N - k - 1.
  The two extra fits use the same path as the main one (the vertex they end at matters for s).
- Header: `Median regression` or `.25 Quantile regression` and `Number of obs` as `%-14s=%11s`.
- fweights work (N is their sum); aweights, pweights and iweights stop with r(101). predict xb and residuals.

## Stata's errors

- **`VCE computation failed; try a different bandwidth or bsqreg` (r(498))** exactly when q - h <= 0 or q + h >= 1 (n = 18 at q = .1, .25,
  .75, .9; all 6 such runs, and no run inside (0, 1) had it). RCraft stops with that message after the log.
- **`VCE computation failed; try increasing the maximum number of iterations or try bsqreg` (r(498))** and **`convergence not achieved.` (r(430))**
  happen on data with two integer dummies and heavy ties (8 of 14 runs on one data set): Stata's own pivoting cycles. RCraft's path does not
  cycle there and prints a fit. **Not reproduced.**

## Open

`vce()`, other density and bandwidth methods, interactions, `iterate()`, `quantile()` in the log; Stata's cycling on heavily tied integer data;
the three flat-face endings above.
