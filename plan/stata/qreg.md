# qreg (2026-11-19, ties and the iteration log 2026-11-19)

Observed on Stata 19 SE with made-up data (corpus `qreg.do`, 344 results to 1e-8: continuous regressors, then dummies and
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
   that breakpoint, ties in breakpoint order by the row's place in the first sort of step 2.
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

| round 1 | runs | matched |
|---|---|---|
| WLS value `a` | 140 | 140 |
| final e(b), where Stata finishes | 126 | 123 |
| the logged objective sequence | 140 | 114 (all 26 misses are tied data) |
| the whole log (sequence and the notes) | 140 | 111 |
| continuous regressors (sequence) | 14 | 14 |

Round 2 (2026-11-20) found that ties in the breakpoint order go by the row's place in the **first sort** (the sort of all rows by weighted
deviation under the WLS fit that also picks the first vertex), not by row number: sequence 114 -> 124 of 140, notes 128 -> 134, e(b) 123 -> 124 of
126 (all 140 runs were used to tune it, so the figures are no longer held out). Written as `order(t, match(idx, ord))`.

| round 2 | runs | matched |
|---|---|---|
| WLS value `a` | 140 | 140 |
| final e(b), where Stata finishes | 126 | 124 |
| the logged objective sequence | 140 | 124 |
| `note: alternate solutions exist.` in the right places | 140 | 134 |

The two e(b) misses: one dummy at q = .1 and two dummies with a continuous regressor at q = .6. The first is rounding noise: the slope at the stop is
`-1.4e-14` in R and exactly zero or above in Stata, so R passes one breakpoint more (with the log's `note` given by hand the fit matches; treating
`|slope| < 1e-9` as zero everywhere is worse, 121 of 126). Corpus: both are left out, the other 115 new numbers match.

**Tried and not better** (each measured on the 140 runs): the entering row among tied breakpoints as the last, the highest row number, the largest or
smallest pivot (75-110 sequences); descending or key-ordered ties (109-122); the first vertex's zero-residual rows starting negative or with the
WLS residual's sign (98-102); the slope summed with R's long-double `sum()`, in the sorted order, reversed or by sign (no better than plain double
in row order); a tableau updated by pivoting in three standard forms instead of recomputing from the basis (122); the tolerance in the
breakpoint ties (0 to 1e-6: same). A depth-first search over every pricing, entering and stop choice that reproduces a logged sequence finds a path
for 137 of 140 runs, so the skeleton is right; the leftovers are which of several rows at the same breakpoint enters, which in Stata depends on
rounding noise in its own arithmetic (row 17 against three duplicated rows at one breakpoint, where Stata must put the duplicates first).

## Stata's errors

- **`VCE computation failed; try a different bandwidth or bsqreg` (r(498))** exactly when q - h <= 0 or q + h >= 1 (n = 18 at q = .1, .25,
  .75, .9; all 6 such runs, and no run inside (0, 1) had it). RCraft stops with that message after the log.
- **`VCE computation failed; try increasing the maximum number of iterations or try bsqreg` (r(498))** and **`convergence not achieved.` (r(430))**
  happen on two-integer-dummy data with heavy ties (8 of 14 runs on one data set). Run by hand at the q +/- h of those fits, Stata's logs for
  five of six fits are the ones R produces (same iterations and objectives; the sixth, q = .0061, Stata gives up after its fourth iteration while R makes
  five more steps that change nothing). So the paths are right and the give-up is a rule not found: it is not a count of equal steps (Stata finished
  one fit with four equal values in a row and gave up at the first repeat of another), not the main fit's iteration count (66 of 126 runs have a
  q +/- h fit longer than the main one and Stata finished them all), and a fit that finishes on its own can still fail as one of the q +/- h fits.
  RCraft prints a fit where Stata errors. **Not reproduced.**

## Open

`vce()`, other density and bandwidth methods, interactions, `iterate()`, `quantile()` in the log; Stata's give-up (above); the tied-breakpoint
order that rounding noise decides (16 of 140 logs differ, 2 of 126 e(b)).
