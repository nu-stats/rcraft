## ROADMAP

- P2C.12ax Batch MV, multivariate and choice models (done 2026-11-22): `manova`, `canon`, `discrim lda` with `predict` and `estat classtable`, `mds`, `cluster`, `mprobit`, `slogit`, `nlogit`, `nlogitgen`, `nlogittree`
  (`manova depvars = terms` with factors, `#`, `##` and `c.x`: the four statistics and Stata's e(); `canon (a) (b)` with `lc()`, `first()`, `stdcoef`; `discrim lda` with `priors()`, `notable`, canonical functions, `predict`, `estat classtable`; `mds` with `dimension()` and `std`; `cluster` single, complete, average and Ward's linkage, kmeans, kmedians, generate, list, dir, drop with the objects in the data's characteristics; `mprobit` (64-node Gauss-Hermite likelihood), `slogit` with `dimension()`, two-level `nlogit` with `nlogitgen` and `nlogittree`, and `predict` after the three) Verified (corpus `manova.do`, `canon_discrim_mds.do`, `cluster.do`, `mprobit.do`, `slogit.do`, `nlogit.do`; notes `plan/stata/multivariate.md`, `cluster.md`, `choice-models.md`).
  **Not translated:** `manovatest` (batch L), `discrim` other than `lda`, a value-labelled group variable in `discrim`, `mds` modern and nonmetric, `cluster` stopping rules and other unfinished options, `mprobit` and `slogit` weights/factor variables/vce(), `nlogit` with more than two levels. `nlogit` with its default bfgs differs from Stata in about the 5th digit (Stata stops early); iteration logs, `e(ic)` and `e(gradient)` are not reproduced.

## STATUS

## P2C.12ax: batch MV (manova, canon, discrim lda, mds, cluster, mprobit, slogit, nlogit), 2026-11-22

Built in `crates/statatr/src/data_r/multi.rs` and `multi/` (`cmds.rs`, `clusters.rs`, `choice_cmds.rs`, `nlogit_cmds.rs` and the R helpers), with
one-line hooks in `data_r.rs` (dispatch and helpers) and `post.rs` (predict and estat after these commands). Checked against Stata 19 in six
corpus files. Rules in `plan/stata/multivariate.md`, `cluster.md` and `choice-models.md` (ported from statcraft@b7d392e, then re-checked).
