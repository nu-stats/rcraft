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
survival, survey, IV and the other models, 4 graphs (with `_natscale`, which gives their axis labels), 5 programs
and chosen SSC commands (`statatr::coverage::tier`). The lines are the ones the do-files run, so a loop's body
counts once a round. The share is of the lines a dry run can judge.

The share has risen in five steps:
- Stata's own random draws (P2C.11) took it from 31.8% to 68.9%.
- P2C.12 took it to 76.1%: `invnormal()`, `normal()`, the `c()` constants and `_natscale`. One research probe
  of StatCraft's runs `_natscale` 5,399 times, so tier 4's share mostly reflects that probe.
- Stata's date and time functions on R (P2C.14) took it to 79.1% (tier 1 to 85.4%).
- `bsample` with Stata's own draws (P2C.16) took it to 83.1%: StatCraft's resampling probes run it in long loops.
- The forms found in published replication packages (P2C.9, below) took it to 83.3%.

| Tier | Lines | Translated | Not translated | Known when run | Share translated |
|---|---:|---:|---:|---:|---:|
| 1. Data management and control flow | 154369 | 124363 | 17454 | 12552 | 87.7% |
| 2. Core statistics | 3225 | 542 | 2679 | 4 | 16.8% |
| 3. Panel, survival, survey, IV and other models | 6344 | 3615 | 2639 | 90 | 57.8% |
| 4. Graphs | 5992 | 5721 | 83 | 188 | 98.6% |
| 5. Programs and SSC commands | 182 | 12 | 169 | 1 | 6.6% |
| Other commands | 4036 | 17 | 3942 | 77 | 0.4% |
| **All** | 174148 | 134270 | 26966 | 12912 | 83.3% |

17563 lines are not translated only because a line before them was not (the variable, scalar, macro, data or stored result it would have made). The first causes, most lines first:

| Lines | Reason |
|---:|---|
| 861 | file is not translated yet |
| 600 | lasso is not translated yet |
| 320 | lassoselect is not translated yet |
| 240 | this runs upd, a program the do-file defines; programs are not translated yet |
| 186 | this runs rho1, a program the do-file defines; programs are not translated yet |
| 186 | this runs rhoof, a program the do-file defines; programs are not translated yet |
| 174 | this runs pex, a program the do-file defines; programs are not translated yet |
| 171 | predict needs a model fitted earlier in this do-file (regress, logit, ...) |
| 162 | no data is in memory at this point (no use or import before it) |
| 128 | lassoknots is not translated yet |
| 121 | Clock() counts leap seconds (datetime/C), which is not translated yet; datetime/c (clock(), cofd(), ...) is |
| 116 | mi is not translated yet |
| 113 | format is not translated yet |
| 100 | lpoly is not translated yet |
| 78 | estat needs a model fitted earlier in this do-file (regress, logit, ...) |
| 77 | this runs dump, a program the do-file defines; programs are not translated yet |
| 71 | estimates is not translated yet |
| 68 | stepwise is not translated yet |
| 68 | xtregar is not translated yet |
| 62 | this runs show, a program the do-file defines; programs are not translated yet |

## Public replication packages (2026-11-02)

The second set of inputs is ordinary analysis code: 11 public replication packages from Harvard Dataverse,
small ones that have both do-files and data, with 46 do-files that run 15,147 lines. Their DOIs (all under
`10.7910/DVN/`) are 6J6BAW, ASKO93, AXYMDI, DZ8BUS, FMDLII, SI3BGA, TQN6ZN, TZEN38, XURXZN, YFBPT6 and ZNMIXH.
They come mostly from political science and economics. The packages stay outside the repository, and only
counts are written here. The same test measures them (`RCRAFT_DOFILES_DIR`); with `RCRAFT_COVERAGE_LINES` it
also writes every refused line with its reason, which is how the fixes below were chosen.

The first dry run translated 40.5% of the lines. These forms, each checked against Stata 19 in `replication.do`
in the corpus, took it to 52.1%:
- `erase` and `rm` (r(601) for a file that is not there, and r(602) for `save` over one);
- `compress` and `set niceness` / `set scheme`, which change nothing in R;
- `append, force`;
- `reshape long` without `j()`;
- `fvset base`, which stays with the data as in Stata;
- `cluster()` and `cl()` for `vce(cluster)`;
- the value label sets, now dropped with the data when new data is read, as in Stata.

| Tier | Lines | Translated | Not translated | Known when run | Share translated |
|---|---:|---:|---:|---:|---:|
| 1. Data management and control flow | 9315 | 6360 | 2281 | 674 | 73.6% |
| 2. Core statistics | 2035 | 211 | 1823 | 1 | 10.4% |
| 3. Panel, survival, survey, IV and other models | 376 | 2 | 373 | 1 | 0.5% |
| 4. Graphs | 117 | 0 | 117 | 0 | 0.0% |
| 5. Programs and SSC commands | 647 | 0 | 633 | 14 | 0.0% |
| Other commands | 2657 | 824 | 1575 | 258 | 34.3% |
| **All** | 15147 | 7397 | 6802 | 948 | 52.1% |

Most of what is left comes from add-ons from SSC: `robreg`, `outreg2`, `reghdfe`, `parmest`, `cluster2`, `eststo`,
`coefplot` and `esttab`. One package's 2,588-line file runs `robreg` and `predict` after it over a thousand
times, and that alone is why tier 2's share is low. Of Stata's own commands, the first causes are:
- `estimates store` and `restore` (148 lines);
- `clogit` (135);
- `gettoken` (78);
- `tsset` and the time-series operators (`f.`, `d.`, `l.`) in `xtreg` (about 130);
- `tabulate, generate()` (27);
- `xtile`, `assert` and `suest`.

## Checked against Stata on R (2026-11-01)

The 390 probes recorded in StatCraft's corpus are each run on R 4.4 in a fresh session, as the do-file window
runs a do-file: 189,013 lines ran, and 158,643 of them were translated (126,618 before the dates, P2C.14; 135,471 before `bsample`, P2C.16).

| | Lines | Before P2C.12 | Before P2C.11 |
|---|---:|---:|---:|
| Compared with what Stata 19 printed (`display`, `return list`, `ereturn list` at the top level) | 5,644 | 210 | 196 |
| The same to 8 significant digits | 5,526 | 205 | 192 |
| The same to 6, not 8 | 5 | 1 | 1 |
| Different | 113 | 4 | 3 |
| Not compared: after the probe's first R error, where the do-file window stops | 213 | 207 | 199 |
| Printed by Stata, not translated (counted from P2C.12) | 810 | | |
| Translated lines that R stopped at | 449 | 429 | 399 |

The 113 that differ are all `_natscale`, nearly all with n = 10 or 15. There Stata's r(min) is a residue of
its arithmetic (2^-55 or −2^-54) where RCraft gives 0, and a residue against 0 fails any relative check
(`plan/stata/errors-and-normal.md`).

The four lines that differed before P2C.12 now match or are refused with the reason:
- `capture noisily` now prints Stata's message, and `_rc` is Stata's 111 or 110.
- Probe 315's `estat ic` after a `qreg` that is not translated is refused. It used to run on the model before
  it.
- Probe 900's globals from an add-on are refused. They used to read as empty.

The 5 that agree to 6 digits, not 8:
- `poisson` (probe 064). Stata stops after one Newton step from a starting point not yet found. That step,
  from Stata's start, gives Stata's coefficients bit for bit.
- Four convergence values of `xtreg, pa` (`e(dif)`, `e(p)` near 0). They are compared for the first time now
  that the probes' data is made.

Most of the 449 R errors come from the probes' setting rather than from RCraft:
- graphs exported as SVG, which need the R package svglite;
- files another probe or StatCraft wrote (`use`, `import`, `list`);
- files the probe writes with `file write`, which is not translated.

Collinear regressors in `logit`, `poisson` and `regress`, reached for the first time once the probes' data was
made, are now omitted as Stata omits them (P2C.13). Two factors interacted (`i.g#i.h`) are laid out as Stata's since P2C.15.

The probe run also lists the lines Stata printed that were refused, by reason, most lines first. The largest
groups now are:
- `ereturn list` after models not translated (`newey`, `prais`, `var`);
- `return list` after `power`, `mvtest`, `estat` and `filefilter`.

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
- time-series operators;
- `estat`, `predict`, `test`, `lincom` and `margins` after a refused line that may have fitted a model;
- a global the do-file has not set, after a refused add-on or program.

## What would move the numbers most

1. `estimates store` and `restore`, `tsset` and the time-series operators, which both sets of do-files use,
   then `file` and the programs a do-file defines.
2. Interactions in the likelihood models' tables, and collinear terms in the panel, survival and IV models.
3. `gettoken`, `clogit`, `rpoisson()`, `rbeta()` and `rt()`, and the models not yet translated (lasso, xtregar,
   newey, prais).
4. The last digits: `poisson`'s starting point and `_natscale`'s residue for n = 10 and 15.
