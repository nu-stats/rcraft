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

| Tier | Lines | Translated | Not translated | Known when run | Share translated |
|---|---:|---:|---:|---:|---:|
| 1. Data management and control flow | 154258 | 50629 | 91079 | 12550 | 35.7% |
| 2. Core statistics | 3195 | 249 | 2942 | 4 | 7.8% |
| 3. Panel, survival, survey, IV and other models | 6310 | 85 | 6135 | 90 | 1.4% |
| 4. Graphs | 593 | 269 | 136 | 188 | 66.4% |
| 5. Programs and SSC commands | 182 | 12 | 169 | 1 | 6.6% |
| Other commands | 9420 | 0 | 9343 | 77 | 0.0% |
| **All** | 173958 | 51244 | 109804 | 12910 | 31.8% |

85646 lines are not translated only because a line before them was not (the variable, scalar, macro, data or stored result it would have made). The first causes, most lines first:

| Lines | Reason |
|---:|---|
| 5399 | _natscale is not translated yet |
| 3489 | bsample is not translated yet |
| 2133 | the function runiform() is not translated yet |
| 1013 | the function clock() is not translated yet |
| 861 | file is not translated yet |
| 605 | the function yw() is not translated yet |
| 504 | the function halfyearly() is not translated yet |
| 504 | the function monthly() is not translated yet |
| 504 | the function quarterly() is not translated yet |
| 504 | the function weekly() is not translated yet |
| 504 | the function yearly() is not translated yet |
| 500 | the function rnormal() is not translated yet |
| 320 | lassoselect is not translated yet |
| 240 | this runs upd, a program the do-file defines; programs are not translated yet |
| 186 | this runs rho1, a program the do-file defines; programs are not translated yet |
| 186 | this runs rhoof, a program the do-file defines; programs are not translated yet |
| 176 | the function invnormal() is not translated yet |
| 174 | predict needs a model fitted earlier in this do-file (regress, logit, ...) |
| 174 | this runs pex, a program the do-file defines; programs are not translated yet |
| 154 | the function ym() is not translated yet |

## Checked against Stata on R (2026-11-01)

The 385 probes recorded in StatCraft's corpus, each run on R 4.4 in a fresh session, as the do-file window runs
a do-file. 179,207 lines ran, and 55,454 of them were translated.

| | Lines |
|---|---:|
| Compared with what Stata 19 printed (`display`, `return list`, `ereturn list` at the top level) | 196 |
| The same to 8 significant digits | 192 |
| The same to 6, not 8 (a Poisson model, which Stata stops at its convergence tolerance) | 1 |
| Different | 3 |
| Not compared: after the probe's first R error, where the do-file window stops | 199 |
| Translated lines that R stopped at | 399 |

The 3 that differ:
- `capture noisily` does not print R's error message, where Stata prints its own.
- Probe 900 reads two globals that an add-on it ran would have set.

Most of the 399 R errors come from the probes' setting rather than from RCraft:
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

1. **Stata's random numbers.** `runiform()`, `rnormal()` and `rgamma()` make the data in most probes, and about
   85,000 refused lines follow from them. R draws different numbers from the same seed. StatCraft has Stata's
   generators exactly (the 64-bit Mersenne Twister and Stata's normal and gamma methods, checked bit for bit
   against Stata 19), so RCraft could give R Stata's own draws.
2. The date functions (`clock()`, `monthly()`, `yw()`, ...) and time-series operators.
3. Programs (`program define`), `gettoken`, `file`, `bsample`, and the models not yet translated (lasso,
   xtregar).
