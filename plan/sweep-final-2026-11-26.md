# App sweep, final run on main, 2026-11-26

Same harness as `sweep-2026-11-26.md` (every catalog dialog with its choices filled in on a
made-up data frame, and 75 Stata lines three ways: no data, data opened with File > Use my
file, data opened with `use`). Run on main at fd77e81 (includes SW1 to SW16, #60 to #71),
against the user's own R 4.6.2 with RCraft's own library (172 packages: the survey package and
mitools were added by hand earlier today with the user's OK). Nothing was installed or
downloaded in this run.

## The 8 original problems

| # | Problem | Still broken? |
|---|---|---|
| 1 | Stata mode after File > Use my file or R-made data | No: all 75 lines run in all three modes |
| 2 | One-line Command box | No in the run; the layout pictures (SW8) were checked in #66 |
| 3 | `summary`, `sysuse`, `webuse`, `help summarize` | No: each says what it does; `help summarize` opens Help |
| 4 | A failed fit wipes the earlier model | No: `regress y nosuchvar` says variable not found and the following `predict` runs |
| 5 | Dialogs that need a package the R lacks | Partly: 15 dialogs (was 20 in the rerun) still wait for fixest, plm, rstanarm, sampleSelection, lavaan, vars. The survey dialogs no longer wait for a package (survey installed by hand; 5 dialogs, now failing only on the harness's choices, see below). Install flow tested with fakes only. |
| 6 | Raw R errors for a missing or wrong-type variable | No in this run: `sum x if g == "a"` now says type mismatch (r(109)); `egen ... by()`, `bysort`, `keep`/`drop`, `collapse` ran with no errors |
| 7 | `list` warning about `stata_sep_` | No |
| 8 | Fit statistics and Average marginal effects on a non-model | No: both say "this works for fitted models (...)" |
| New in rerun | `reg y x i.g` on a factor made in R (SW10) | No: runs in all three modes |

## Stata lines

75 lines times three modes. Every line produced output; the only errors are the two Stata
gives too: `regress y nosuchvar` (variable not found, r(111), all three modes) and
`sum x if g == "a"` on a labelled number (type mismatch, r(109), menu and use). `summary`,
`sysuse`, `webuse` print a "RCraft did not run this line" note by design; `help summarize`
opens Help (no Results block). No "no data is in memory", no raw R errors, no `stata_sep_`
warning.

## Catalog sweep

137 dialogs (one more than the first sweep; the P6.11e meta-analysis work is in): 68 ran Ok,
15 need a package the R lacks, 21 the harness could not fill (empty rule boxes, file names
without the right ending, exposure with the wrong model, meta-analysis subgroups), 33 errors.
Each of the 33 judged:

- 21 are the harness picking unsuitable variables, each answered with a clear message: a 3-level
  factor for t-test and Mann-Whitney, non-binary outcome for logistic, a constant column for
  discriminant analysis, 60 groups for a permutation test, survival time and status from the
  wrong columns (Kaplan-Meier, log-rank, Cox, competing risks, parametric), a panel with no complete
  cases, too many levels for a mixed model, relevel to a missing level, Fisher on a 60-level
  table, Labelled to factor on a column with no labels, treatment not 0/1, and the five survey
  dialogs given a made-up `fpc = ~id + t` ("FPC implies >100% sampling"; the design is built and
  R's own message follows).
- 11 are Objects > Model dialogs run on a data frame, not a model; all say "this works for ...".
- 1 is a follow-on of the harness's failed Cox fit: see finding F1.
- None of the 33 is a bug in a dialog.

## Findings

| # | What | Severity |
|---|---|---|
| F1 | Statistics > Survival > Survival curves from a Cox model, when no Cox model exists in the session, ends with R's "object 'cox' not found" and an empty call. The other Model dialogs say what they need ("this works for ..."); this one only has its own `stopifnot` text after an object lookup that fails first. Not fixed here. | low |

Nothing else new.

## Not covered

As before: clicks and layout, the do-file window, the Packages window, the Graph window, the data
editor, sessions and projects. The install flow for the six missing packages was not run (no
installs allowed in the sweep). A hands-on or picture pass is still the way to cover these.
