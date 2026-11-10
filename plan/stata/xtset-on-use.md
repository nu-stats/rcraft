# xtset kept in a .dta, and xtreg's collinearity (2026-11-06)

## The settings a .dta keeps

Stata writes `xtset`/`tsset` into the file as characteristics of `_dta`: `_TSpanel`, `_TStvar`,
`_TSdelta` (the delta as %21x, `+1.0000000000000X+000`), `_TSitrvl`, and the older `iis` and `tis`.
`use` puts them back. RCraft did not know them for a file the do-file had not saved itself, so every
`xtreg`, `L.x` or `D.x` after such a `use` was refused ("the data were read from a file whose xtset or
tsset settings are not known here"). TZEN38 opens a shipped `FOMC_Database.dta` like that before
its 87 `xtreg` lines.

Now, when the do-file runs and the file is there (not written earlier in the run), the runner asks
R for them before translating `use` (`DTA_TS_READER` in `data_r/ts.rs`, from Stata's published
description of formats 117 to 119: the `<map>` offsets, then each `<ch>` with its 4-byte length,
the variable and characteristic names of 129 bytes, 33 in 117, and the value up to its NUL). A
quiet `xtset panel time[, delta(#)]` or `tsset time` follows the `use` line. Older formats and
`use varlist using` are left as before. Checked on files Stata saved (delta 2, tsset alone, none)
and in a run against Stata 19 (lags and xtreg the same).

## xtreg's collinear columns

Checked in Stata 19:
- a column the constant and the earlier ones explain is omitted, the later one of the set (four
  shares adding to 1: the fourth; `gv omitted because of collinearity`), with re, fe and be;
- fe omits a column that does not vary within the panels;
- fe with a column collinear with the others only within the panels: Stata omitted the last level
  of `i.meet` in one data set, and `167.FOMCMEETING_id` ("collinearity with the absorbed variable")
  where R's order would omit 168 in TZEN38. Not known: R stops with a message there;
- re with more columns than panels (TZEN38: 154 for 51): the between regression has no degrees of
  freedom, and Stata has sigma_u = 0, theta = 0, least squares;
- with fewer clusters than slopes the variance is not of full rank; `e(chi2)` and `e(p)` are missing
  and `e(df_m)` is the rank (G - 1). RCraft's Wald tests are missing whenever the variance is not of
  full rank.

## After a fit that stops in R

The model lines now start by removing the last fit's results (`stata_e`, `stata_b`, ...) and set
`stata_fit_failed`, cleared at their end. If R stops in the fit, `e()` and `_b[]` read after it
stop with "R stopped in the model before it", where they had shown the model before.

## TZEN38 run whole

From 870 lines run to 1,567, and from 252 model values the same as Stata's to 707, none different;
154 of Stata's 861 are not printed (fits declined: 21 fe with the within collinearity above, and
lines refused).

## xtreg fe with columns collinear within the panels: fitted, with a note (2026-11-06)

Which column Stata leaves out was probed further (`scratchpad/fecol`, 6 made-up data sets, TZEN38's 22
models):
- Exact collinearity (made-up data, integers): "note: 10.meet omitted because of collinearity", the
  later column of the set in the order written (dummies written first: age).
- TZEN38: "note: 167.FOMCMEETING_id omitted because of collinearity with the absorbed variable", and
  165, 166, 167 or 168 by model. Fixed for a data set (rows sorted, shuffled, doubles: the same), but
  it rests on rounding: Stata's dependency leaves a residual of 1e-14 to 1e-13 of the columns, which
  decides whether the in-order check (1e-13) catches it, and then a largest-pivot sweep picks between
  dummies whose pivots tie to rounding (167 and 168 have the same attendees). Largest pivot first fits
  where there is no tie; no rule fits every model.
- What does not depend on the choice: the coefficients outside the set and their standard errors, N,
  the R-squareds, sigma_u, sigma_e, rho, F, the predictions. What does: the coefficients of the set
  (age and the year's meeting levels) and _cons.

Decided with the user (option C of three: decline, flag the coefficients, or fit with a note): RCraft
fits the model leaving out the later column (Stata's choice where the collinearity is exact) and
prints a note naming the coefficients that may differ. TZEN38: 846 of 861 model values the same as
Stata's, the 15 others in those sets (14 age, 1 meeting level); Tables 1 and 5 the published ones
(line endings aside), Tables 2a to 4 too but for _cons in the fe columns where Stata left out
another level. Also `age age` (o.age after age, as Stata) and g#c.x without g (every level in
e(b); a zero base column had been put in).

Superseded in part (2026-11-09, `xtreg-absorbed.md`): the absorbed note comes with `vce(cluster v)` on
another variable than the panel, and Stata then omits what a largest-pivot sweep leaves; only ties
rest on rounding.
