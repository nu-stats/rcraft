## ROADMAP

- P2C.12az estat after tobit, intreg, truncreg, cloglog, hetprobit and heckman (done 2026-11-22)
  (`estat ic [, n() df()]` with r(S), `estat vce [, correlation covariance format()]` with r(V), `estat summarize [varlist] [, equation labels noheader]` with r(stats), all in Stata's layout; weights, robust, clusters, twostep; every other subcommand stops with Stata's r(321)) Verified (corpus `estat_censored`; note `plan/stata/censored.md`).
  **On hold** (listed, to be built): `estat vce` and `estat summarize` with factor variables, `estat vce` wider than the line (Stata splits the columns), `estat vce` with equation(), block or diag, fractional n() or df(), `estat summarize` with a varlist and equation, fweights with clusters; `margins` after these models (margins.rs reads R fit objects, these keep e(); needs a delta-method engine over xb, pr(), e() and ystar() first).

## STATUS

## P2C.12az: estat after the censored and selection models, 2026-11-22

- **Done** (checked in Stata 19 on made-up data; corpus `estat_censored`, 1e-5, 1,550 values over the six commands with plain, fweight, pweight, iweight, robust and clustered fits, and heckman twostep): `estat ic` (n(), df(), r(S); Stata's r(321) after twostep), `estat vce` (covariance, correlation, format(), r(V), equation blocks), `estat summarize` (varlist, equation, labels, noheader, the `_weight` row, clusters block, nonmissing notes, r(stats)); `estat gof`, `classification`, `lroc`, `hettest` and the rest stop `not valid` r(321), `estat` alone `subcommand required`, `estat bootstrap` r(301), a varlist after ic or vce r(101), an unknown option of ic r(198), `n(0)` r(125). The printed tables match Stata's log line for line (truncreg's last digit aside: Stata's derivatives are numerical).
- **Found:** Stata's `estat ic` after the six is `estimates stats` with the row named `.`; N shows with thousands separators from 1,000; a name with a parenthesis in `estat vce` has its separating space in front of it; `estat summarize` with iweights divides by the sum of weights minus one, with aweights and pweights by N - 1 on rescaled weights, and the `_weight` row is unweighted; the labels layout shows min and max in %8.0g.
- **On hold, listed:** see ROADMAP P2C.12az.
