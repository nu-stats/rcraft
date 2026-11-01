# Stata mode: coverage

How much of a set of real do-files RCraft runs on R (ROADMAP P2C.8 and P2C.9). The menus' own coverage is in
`coverage.md`.

## Where the numbers come from

The first inputs are StatCraft's 381 probe do-files: our own, on made-up data, written to pin down how Stata 19
behaves. They are not typical analysis files. Most of them build their data with Stata's random-number functions
and then test one corner of one command, so they ask far more of RCraft than an ordinary do-file does. The
numbers below are a floor, not an estimate of everyday use.

Two measurements, each an opt-in test that reads the do-files from outside the repository:

- **Translated** (`crates/statatr/tests/many_dofiles.rs`, set `RCRAFT_DOFILES_DIR`): every do-file run dry, as
  the do-file window starts one, without R. Each line is translated, not translated (with the reason), or
  known only when the do-file runs (a value only R knows, such as `if _rc` or `local m = r(mean)`). Set
  `RCRAFT_COVERAGE_TABLE` to a file and the test writes the table below.
- **Checked against Stata** (`crates/engine/tests/stata_probes.rs`, set `RCRAFT_STATA_PROBES` to StatCraft's
  private corpus): every do-file runs line by line in a fresh R, and each `display`, `return list` and
  `ereturn list` at the top level of a probe, before its first R error, is set against what Stata 19 printed
  for it. Numbers must agree to 8 significant digits. `%21x` is read back to the double. Stata's logs carry
  its licence banner, so they stay in the corpus folder and are never read into the repository; only counts
  are written here.

A command is **Verified** only through RCraft's own corpus (`crates/engine/tests/stata_corpus`), whose recorded
results are committed and checked by every CI run. The probes are a wider check run on this machine.

## Translated, by tier (2026-11-01)

The tiers: 1 data management and control flow (macros and loops included), 2 core statistics, 3 panel,
survival, survey, IV and the other models, 4 graphs, 5 programs and chosen SSC commands
(`statatr::coverage::tier`). The lines are the ones the do-files run, so a loop's body counts once a round.
The share is of the lines a dry run can judge.

With Stata's own random draws (P2C.11, `set seed`, `runiform()`, `rnormal()`, `rgamma()`), the share rose from
31.8% to 68.9% (tier 1 from 35.7% to 77.7%), and the lines refused only because a line before them was fell
from 85,646 to 25,350.

| Tier | Lines | Translated | Not translated | Known when run | Share translated |
|---|---:|---:|---:|---:|---:|
| 1. Data management and control flow | 154294 | 110201 | 31542 | 12551 | 77.7% |
| 2. Core statistics | 3222 | 373 | 2845 | 4 | 11.6% |
| 3. Panel, survival, survey, IV and other models | 6310 | 88 | 6132 | 90 | 1.4% |
| 4. Graphs | 593 | 280 | 125 | 188 | 69.1% |
| 5. Programs and SSC commands | 182 | 12 | 169 | 1 | 6.6% |
| Other commands | 9431 | 2 | 9352 | 77 | 0.0% |
| **All** | 174032 | 110956 | 50165 | 12911 | 68.9% |

25350 lines are not translated only because a line before them was not (the variable, scalar, macro, data or stored result it would have made). The first causes, most lines first:

| Lines | Reason |
|---:|---|
| 5399 | _natscale is not translated yet |
| 3489 | bsample is not translated yet |
| 1013 | the function clock() is not translated yet |
| 861 | file is not translated yet |
| 802 | the random draws here are not Stata's: a line before it that draws was not translated (bsample); a set seed after it starts them again |
| 605 | the function yw() is not translated yet |
| 573 | lasso is not translated yet |
| 504 | the function halfyearly() is not translated yet |
| 504 | the function monthly() is not translated yet |
| 504 | the function quarterly() is not translated yet |
| 504 | the function weekly() is not translated yet |
| 504 | the function yearly() is not translated yet |
| 320 | lassoselect is not translated yet |
| 301 | the random draws here are not Stata's: a line before it that draws was not translated (bsample 1); a set seed after it starts them again |
| 301 | the random draws here are not Stata's: a line before it that draws was not translated (bsample, cluster(g)); a set seed after it starts them again |
| 300 | the random draws here are not Stata's: a line before it that draws was not translated (bsample 2); a set seed after it starts them again |
| 240 | this runs upd, a program the do-file defines; programs are not translated yet |
| 188 | predict needs a model fitted earlier in this do-file (regress, logit, ...) |
| 186 | this runs rho1, a program the do-file defines; programs are not translated yet |
| 186 | this runs rhoof, a program the do-file defines; programs are not translated yet |

## Checked against Stata on R (2026-11-01)

The 387 probes recorded in StatCraft's corpus, each run on R 4.4 in a fresh session, as the do-file window runs
a do-file. 179,330 lines ran, and 114,916 of them were translated (55,454 before Stata's random draws, P2C.11).

| | Lines | Before P2C.11 |
|---|---:|---:|
| Compared with what Stata 19 printed (`display`, `return list`, `ereturn list` at the top level) | 210 | 196 |
| The same to 8 significant digits | 205 | 192 |
| The same to 6, not 8 (a Poisson model, which Stata stops at its convergence tolerance) | 1 | 1 |
| Different | 4 | 3 |
| Not compared: after the probe's first R error, where the do-file window stops | 207 | 199 |
| Translated lines that R stopped at | 429 | 399 |

The 4 that differ:
- `capture noisily` does not print R's error message, where Stata prints its own.
- Probe 900 reads two globals that an add-on it ran would have set (2 lines).
- Probe 315, reached now that its data is made: `qreg` is not translated, but the refused model does not
  clear `e()`, so `capture noisily estat ic` runs on the model before it and `_rc` is 0 where Stata gives
  321.

Most of the 429 R errors come from the probes' setting rather than from RCraft (more lines run now, so there
are more of them):
- graphs exported as SVG, which need the R package svglite;
- files another probe or StatCraft wrote (`use`, `import`, `list`);
- files the probe writes with `file write`, which is not translated.

## What the probes changed

The probes found missing features, which are now translated:
- `set obs`;
- one-line `if exp command` and `else command`;
- `tokenize`;
- `display %21x`;
- `preserve` / `restore`;
- `expand`;
- `_rc` after `capture`.

They also found places where RCraft gave a wrong answer without saying so. Those lines are now refused with the
reason:
- a line that uses a variable, scalar or macro whose making line was not translated;
- a line after a refused line that replaces the data or changes its rows;
- a `local` that could not be set, which had kept its old value;
- `quietly display`, which printed;
- comparisons with `.a`–`.z`;
- time-series operators.

The R errors on translated lines fell from 1,494 to 399 as these lines came to be refused.

## What would move the numbers most

Stata's random numbers were first here. Since P2C.11 (2026-11-01), `set seed`, `runiform()`, `rnormal()` and
`rgamma()` give Stata's own draws on R (`plan/stata/random-numbers.md`). What is left:

1. `_natscale` (5,399 lines) and `bsample` (3,489, and the draws after it). `invnormal()` and `normal()` are not
   translated either, and `invnormal(runiform())` is how many probes make their data.
2. The date functions (`clock()`, `monthly()`, `yw()`, ...) and time-series operators.
3. Programs (`program define`), `gettoken`, `file`, the other random functions (`runiformint()`, `rbinomial()`,
   ...), and the models not yet translated (lasso, xtregar).
