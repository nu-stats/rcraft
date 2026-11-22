## ROADMAP

- P2C.12bb Maximize options, first piece (batch E follow-on, done 2026-11-22, in part)
  (`iterate(#)` on `logit`, `probit` and `poisson`: Stata's Newton path stopped after # steps, `convergence not achieved` (also under `nolog`), the estimates, variance, likelihood and tests where it stopped, the warning after the table, e(converged) 0; `iterate(0)`) Verified (corpus `ml_iterate.do`, 93 results to 1e-6; note `plan/stata/ml-options.md`).
  **On hold** (listed, to be built): `from()` (36 probe lines: matrices from earlier fits, copy, skip, name matching and its errors), `trace` and `showstep` (13 lines), `iterate()` on nbreg, ologit, oprobit and mlogit.
  Measured before the rebase onto PR 26 (not re-measured with both): probes 82.1% -> 82.2% (23,829 -> 23,854 translated of 30,115 source lines; the probe lines that combine `iterate()` with `from()` or `trace` stay refused), packages 94.8% unchanged.

## STATUS

## P2C.12bb: maximize options, first piece (iterate(#)), 2026-11-22

- **Done** (Stata 19 on made-up data; corpus `ml_iterate.do`, 93 results to 1e-6, the printed output compared with Stata's log line for line):
  `iterate(#)` on `logit`, `probit` and `poisson`. Note `plan/stata/ml-options.md`.
- **Found:** the path stops after # steps; `convergence not achieved` follows the log (also under `nolog`), `Warning: Convergence not achieved.` follows the table; poisson
  with iterate(0) prints no blank line above its one iteration line; Stata's `-0.00` for an iterate(0) LR test is last-bit noise on one data set, 0 on others.
- **On hold, listed:** `from()`, `trace`, `showstep`, and `iterate()` on nbreg, ologit, oprobit and mlogit.
- **Shares:** probes **82.2%** (23,854 translated, 5,157 not, 1,104 known only when run, of 30,115 source lines; was 23,829); packages **94.8%** (4,028 of 4,249), unchanged.
