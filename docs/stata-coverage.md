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
| 1. Data management and control flow | 154369 | 124365 | 17452 | 12552 | 87.7% |
| 2. Core statistics | 3225 | 583 | 2638 | 4 | 18.1% |
| 3. Panel, survival, survey, IV and other models | 6344 | 3615 | 2639 | 90 | 57.8% |
| 4. Graphs | 5992 | 5721 | 83 | 188 | 98.6% |
| 5. Programs and SSC commands | 182 | 12 | 169 | 1 | 6.6% |
| Other commands | 4036 | 17 | 3942 | 77 | 0.4% |
| **All** | 174148 | 134313 | 26923 | 12912 | 83.3% |

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

## Public replication packages (2026-11-03)

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

Then `estimates store`, `restore`, `drop` and `clear` (checked in `estimates.do`) took it to 53.1%, and `assert` and `tabulate, generate()` (`checks.do`) to 53.3%. `tsset` and the time-series operators (`timeseries.do`), with `replace x = L.x ...` done row by row as in Stata, took it to 53.8%; tier 3 went from 0.5% to 16.3%. `clogit` (`clogit.do`) and `vce(r)` and `vce(cl v)` as Stata abbreviates them took it to 55.9%. `gettoken` (`made.do`) took it to 56.5%. `xtile` (`xtile.do`) to 56.6%. `merge` of every type (`merge.do`), m:m among them, to 58.8%. The options that change only how a table looks (`baselevels`, `cformat()` and the like) took it to 59.1%. `robreg m` (`robreg.do`), which one package runs in loops over and over, took it to 79.3%. `reghdfe` (`reghdfe.do`), with two-way clustering, to 80.2%. `eststo` and `estadd` (`estout.do`) to 80.7%. `outreg2` (`outreg2.do`), its files byte for byte, to 81.9%. `fre`, and `margins` after `clogit` with `parmest` (`clogit_margins.do`), to 83.5%. `xpose` (`made.do`), which one package turns its simulated estimates with, took it to 83.7%. The graphs below took it to 83.9%: `twoway bar` and `rarea`, plots split at `||`, `graph box`, `graph bar (count)`, `xline()` and `yline()`, `graph drop _all`, and the look-only short forms Stata 19 accepts; tier 4 went from 0% to 16.2%.

| Tier | Lines | Translated | Not translated | Known when run | Share translated |
|---|---:|---:|---:|---:|---:|
| 1. Data management and control flow | 9393 | 7345 | 1374 | 674 | 84.2% |
| 2. Core statistics | 2035 | 1838 | 196 | 1 | 90.4% |
| 3. Panel, survival, survey, IV and other models | 376 | 265 | 110 | 1 | 70.7% |
| 4. Graphs | 117 | 19 | 98 | 0 | 16.2% |
| 5. Programs and SSC commands | 569 | 377 | 178 | 14 | 67.9% |
| Other commands | 2657 | 2065 | 334 | 258 | 86.1% |
| **All** | 15147 | 11909 | 2290 | 948 | 83.9% |

Most of what is left comes from add-ons: Petersen's `cluster2` (not on SSC), `coefplot`, `esttab`, and
`outreg2`'s LaTeX form. Of Stata's own commands, the first causes are:
- graphs (98 of 117): `graph export` after `coefplot`, `grstyle` and `marginsplot` (64), a second y axis (`yaxis(2)`, 7), `saving()` and `graph combine`;
- `suest`.

The 36 `xtreg` and `reg` lines with `f.`, `d.` and `l.` in one package read a file saved after `cd` to a folder
that a dry run does not know (`${root}`); when the do-file runs, the folder is known and its tsset comes back
with the file.

## Whole packages run in Stata 19 and on R (2026-11-03)

Ten of the packages (all but 6J6BAW) were also run whole, from their master do-files, in Stata 19 and on R,
each on a fresh copy outside the repository. In each copy, after every model, a line prints `@E`, e(N) and
the first six coefficients and variances. Nothing is installed or set permanently: `ssc install`,
`net install`, `ado update` and `, permanently` lines are commented out. The roots are set to the copy.
Stata runs `do file, nostop` so that it carries on past errors as RCraft does. RCraft's run
(`crates/engine/tests/replication_e2e.rs`, with `RCRAFT_E2E_DIR`) goes line by line through the Engine, as
the do-file window does. It writes R's output, the R it ran and each line's fate next to the copy.

| Package | Model lines in Stata | On R | The same (to 6 digits) | Different | Model refused in R | Stata only |
|---|---:|---:|---:|---:|---:|---:|
| ASKO93 | 241 | 140 | 140 | 0 | 0 | 101 |
| FMDLII | 7 | 7 | 7 | 0 | 0 | 0 |
| TQN6ZN | 231 | 231 | 231 | 0 | 0 | 0 |
| TZEN38 | 861 | 252 | 252 | 0 | 0 | 609 |
| YFBPT6 | 697 | 8942 | 385 | 312 | 0 | 0 |
| ZNMIXH | 899 | 216 | 117 | 7 | 76 | 699 |

Every model line both runs reached gives Stata's numbers in four packages. The rest come from what
neither could run, or what one ran on other data:
- YFBPT6's 312 differences all follow `xpose, clear varname`. It was not translated, so the file it makes was
  never written and later models ran on the data before it. `xpose` is now translated.
- ZNMIXH's differences come after lines R could not run on the data it had. Stata itself stops in 188 places
  there, on add-ons not installed (`winsor`, `center`, `coefplot`) and on files they would have written.
- The packages that reached no model (AXYMDI, DZ8BUS, SI3BGA) expect their data to be opened by hand. XURXZN
  reads processed files it does not ship.

The run also found, and fixed:
- `lincom` with a factor's base level (`1.g` where 1 is the base), which Stata counts as 0. When every term
  is a base, Stata shows `0 (omitted)`.
- `label values v name` with a set from the .dta file. haven keeps the labels but not the sets' names, so
  RCraft now reads them from the file.
- `xtreg, fe` with `i.year` in a balanced panel, which stopped in R (the between regression it does not need
  was singular).
- `recode` with labels in its rules, `decode`, `encode`'s label set, and `do file, nostop`.

## Checked against Stata on R (2026-11-03)

The 406 probes recorded in StatCraft's corpus are each run on R 4.6 in a fresh session, as the do-file window
runs a do-file: 190,347 lines ran, and 159,638 of them (83.9%) were translated (158,643 of 189,013 in 390 probes on 2026-11-01; 126,618 before the dates, P2C.14; 135,471 before `bsample`, P2C.16).

| | Lines | Before P2C.12 | Before P2C.11 |
|---|---:|---:|---:|
| Compared with what Stata 19 printed (`display`, `return list`, `ereturn list` at the top level) | 5,679 | 210 | 196 |
| The same to 8 significant digits | 5,561 | 205 | 192 |
| The same to 6, not 8 | 5 | 1 | 1 |
| Different | 113 | 4 | 3 |
| Not compared: after the probe's first R error, where the do-file window stops | 254 | 207 | 199 |
| Printed by Stata, not translated (counted from P2C.12) | 817 | | |
| Translated lines that R stopped at | 530 | 429 | 399 |

The 113 that differ are all `_natscale`, nearly all with n = 10 or 15. There Stata's r(min) is a residue of
its arithmetic (2^-55 or −2^-54) where RCraft gives 0, and a residue against 0 fails any relative check
(`plan/stata/errors-and-normal.md`). On 2026-11-03 part of that arithmetic was found from Stata's answers.
Stata's steps are 2, 5 and 10 times its own 10^k. For k below 0 that is not always the nearest double:
10^-2 is 0.01 plus one unit in the last place. So a step of 0.1 is 10 × 10^-2, one unit above 0.1. This fits
4,912 of the 4,915 sweeps in probes 225 and 226. The ends are not found yet. With lo = 0 the residue shows
only where the top label lies above the data, and only for 11 or 12 labels of 0.1 or some of 0.2. No form
in lo, hi, the step and the label counts gives it (a search of about 270,000 such expressions found none).
So the 113 lines stay as they are (`plan/stata/natscale-residue.md`).

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

Most of the 530 R errors come from the probes' setting rather than from RCraft:
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
- replace with a lead or difference of its own variable (`replace x = D.x`), which Stata does row by row;
- `estat`, `predict`, `test`, `lincom` and `margins` after a refused line that may have fitted a model;
- a global the do-file has not set, after a refused add-on or program.

## What would move the numbers most

1. `file` and the programs a do-file defines.
2. Interactions in the likelihood models' tables, and collinear terms in the panel, survival and IV models.
3. `rpoisson()`, `rbeta()` and `rt()`, and the models not yet translated (lasso, xtregar,
   newey, prais).
4. The last digits: `poisson`'s starting point and `_natscale`'s ends for n = 15 to 25 (its step is found).
