## ROADMAP

- P2C.12ax Batch MV, multivariate and choice models (started 2026-11-22; first piece done: `manova`, `canon`, `discrim lda` with `predict` and `estat classtable`, `mds`)
  (`manova depvars = terms` with factors, `#`, `##` and `c.x`: the four statistics, b, V, E, H_#, B, xpxinv, stat_#, aux_#, eigvals_#; `canon (a) (b)` with `lc()`, `first()`, `stdcoef`; `discrim lda` with `priors(equal|proportional)`, `notable`, the canonical discriminant matrices, `predict` (the classification) and `estat classtable`; `mds` with `dimension()` and `std`) Verified (corpus `manova.do`, `canon_discrim_mds.do`, 263 values, every matrix element to 1e-9; note `plan/stata/multivariate.md`).
  **Still to build in this batch:** `cluster` (probe 248), `mprobit`, `slogit`, `nlogit`, `nlogitgen`, `nlogittree`. **Not translated:** `manovatest` (batch L), `discrim` other than `lda`, a value-labelled group variable in `discrim`, `mds` modern and nonmetric.
  Measured: probes 533 and 535 refused 29 lines before, 3 now (`manovatest` and what depends on it); all probes 82.2% (23,855 of 29,011 judged lines, with the other merged batches); packages 94.7% unchanged (none of the 46 do-files uses these commands).)

## STATUS

## P2C.12ax: batch MV, first piece (manova, canon, discrim lda, mds), 2026-11-22

Built in `crates/statatr/src/data_r/multi.rs`, `multi/cmds.rs` and `multi/{manova,canon,discrim,mds}.R`, with one-line hooks in
`data_r.rs` (dispatch and helpers) and `post.rs` (predict and estat after `discrim`). Checked against Stata 19 in two corpus
files; rules in `plan/stata/multivariate.md` (ported from statcraft@b7d392e, then re-checked: e(C) has no log prior,
`unique` is always 1, `lc()` cuts e()).
Probes 533 and 535: 29 refused lines -> 3 (`manovatest`, batch L's). Packages unchanged.
