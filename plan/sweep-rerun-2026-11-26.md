# App sweep rerun on merged main, 2026-11-26

Same harness as `sweep-2026-11-26.md` (every catalog dialog, 136, and 75 Stata lines three
ways), run on main at 0ca7049 (SW1 to SW4 merged) against the user's own R 4.6.2 with 170
packages. Nothing was installed or downloaded. Stata lines: "no data" here means a data frame
made in R that the app has not opened, "menu" is File > Use my file, "use" is `use`.

## What is fixed (checked in the run)

| # | Problem | Now |
|---|---|---|
| 1 | Stata mode after File > Use my file or R-made data | `describe`, `summarize`, `tab`, `list`, `count`, `regress`, `predict`, `egen`, `collapse`, `save` and the rest all run in all three modes; no "no data is in memory". |
| 2 | One-line Command box | The multi-line box exists; looks at it still need a picture pass (SW8). |
| 3 | `summary`, `sysuse`, `webuse`, `help summarize` | `summary` says it is not a Stata command and suggests `summarize`; `sysuse` and `webuse` say why they are not run; `help summarize` opens Help (no Results block, as designed). |
| 4 | `regress y nosuchvar` wiped the earlier model | Says "variable nosuchvar not found (Stata: r(111))" and the following `predict` runs. Only for a variable the data lack: a fit that fails otherwise still wipes the model (SW5). |
| 7 | `list` warning about `stata_sep_` | Gone in all three modes. |
| 8 | Fit statistics, Average marginal effects on a non-model | Both now say "this works for fitted models (...)". |

## Still failing

| Where | What | Follow-up |
|---|---|---|
| 5. 20 dialogs | Need fixest, plm, survey, lavaan, vars, rstanarm, sampleSelection, which this R lacks (was 21; Panel serial-correlation test now runs as it needs no package). The dialog offers an install; the install flow itself was not run here (no installs allowed). | none; check by hand |
| 6. `sum x if g == "a"` when g is a labelled number (menu, use) | Raw dplyr error "Can't combine `x` <character> and `y` <double>"; Stata says "type mismatch" (r(109)). | suggestion below |
| 6. missing variable in other commands | `egen ... by()`, `bysort`, `keep`/`drop`, `collapse`, if-expressions, options still give raw R errors. | SW6 |
| 4. A fit that fails for another reason | Still clears the earlier model. | SW5 |

## New

- **`reg y x i.g` when g is a factor made in R (not through `use` or File > Use my file)
  fails with "subscript out of bounds | keys[[n]]"** (`stata_coef_rows` in
  `crates/statatr/src/data_r/tables.rs`). The same line runs when g is a labelled number
  (menu and use). Not fixed here; suggested as its own small task.
- Nothing else new: all other lines in all three modes ran with output.

## Catalog sweep

136 dialogs: 68 ran Ok, 20 need a package the user's R lacks, 20 the harness could not fill
(dialogs that reject the made-up choices: empty rule boxes, file names, exposure with the
wrong model), 28 errors. Each of the 28 judged:

- 17 are the harness choosing unsuitable variables, each answered with a clear message from
  the dialog or R: a 3-level factor for a t-test or Mann-Whitney, a non-binary outcome for
  logistic, a constant column for discriminant analysis, 60 groups in a permutation test,
  survival time and status taken from the wrong columns (five survival dialogs), a panel with
  no complete cases, too many levels for a mixed model, relevel to a level that does not exist,
  Fisher on a 60-level table, Labelled to factor on a column with no labels, treatment not 0/1.
- 11 are Objects > Model dialogs run on a data frame, not a model (Fit statistics, Average
  marginal effects, Predicted values, Diagnostics, Diagnostic plots, Robust standard errors,
  Test terms, Sums of squares, Intraclass correlation, Forecast, Model table). All answer
  "this works for ..." and name the models they take. Fit statistics and Average marginal
  effects were the two that showed raw R errors before (SW4).
- None of the 28 is a bug in a dialog.
