## ROADMAP

- P2C.12az lpoly (batch LP of the StatCraft port, done 2026-11-22)
  (`lpoly y x` with `kernel()` (every kernel, Stata's abbreviations), `degree()` 0 to 5, `bwidth()`, the rule-of-thumb bandwidth and the pilot bandwidth, `n()`, `at()`, `generate()` with one or two names, `se()`, `pwidth()`, `var()`, aweights and fweights, `if` and `in`, r() with `pwidth` after `se()`, Stata's errors; `ci`, `noscatter` and the graph as ggplot2's drawing) Verified (corpus `lpoly.do`, `lpoly_bandwidth.do`; note `plan/stata/lpoly.md`).
  Not translated: degrees above 5; a window of exactly p + 3 points, where Stata's standard error is rounding noise (RCraft: missing); Stata's own graph look and scatter/line options.
  Measured on top of origin/main at df4efce (probes only; `observe/dofiles`): 82.1% -> 82.3% (23,829 -> 23,877 translated source lines of 30,115; 5,182 -> 5,134 not translated); packages unchanged at 94.8% (4,028 of 4,247).

## STATUS

## P2C.12az: lpoly (batch LP of the StatCraft port), 2026-11-22

Built in `crates/statatr/src/data_r/lpoly.rs` (R helper plus the translator method; it uses `npregress.rs`'s kernels and elimination), with
small hooks in `data_r.rs`, `coverage.rs`, and the per-command files for completion, differences and verified. Checked against Stata 19 in two corpus files (`lpoly.do`: 103
results; `lpoly_bandwidth.do`: 82, probe 553's bandwidths) at 1e-6. Rules, including what Stata 19 does that StatCraft's note did not say (a window of
one point is missing, fewer points than coefficients drop the smaller pivots, `n(1)` and a fractional `n()` or `degree()` are read as the default,
the exact error messages), are in `plan/stata/lpoly.md`. Probes 82.1% -> 82.3% (+48 translated lines, every `lpoly` line of probes 552 and 553),
packages 94.8% unchanged. On hold: degrees above 5, the p + 3 point standard error, Stata's graph styling.
