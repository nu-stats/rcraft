## ROADMAP

- P2C.12ay Batch J round 2: the options and the selection commands (done 2026-11-22, in part)
  (`iterate()` and `offset()` in tobit, intreg, truncreg, cloglog, hetprobit and heckman; `noconstant` in intreg, cloglog, hetprobit and heckman twostep; `from()` in tobit; `stepwise` and `nestreg` with tobit) Verified (corpus: the six files extended, new `tobit_select.do`; note `plan/stata/censored.md`).
  **On hold** (listed, to be built): `constraints()` (Stata's constrained Newton path is not decoded), `noconstant` in tobit (its estimates, ll, F test and e(Cns) match; its starting values do not), in truncreg (variances 1e-3 off) and in heckman by maximum likelihood (a different start); aweights in tobit and intreg; interactions in tobit/intreg; factor variables in the other five; stepwise with tobit and robust variances or weights other than fweights; margins and `estat` other than `ic` after these models (own thread).
  Measured: refused source lines in the 11 probe files that hold batch-J commands 161 -> 110 (242, 244, 263, 274, 277, 284, 289, 292, 315, 333, 508; some of the drop is other batches in the same files); probes 82.2% (23,839 of 30,115 translated; main before this PR was 80.5% at PR 15 plus later batches); packages 94.8% unchanged.

## STATUS

## P2C.12ay: batch J round 2, 2026-11-22

- **Done** (checked in Stata 19 on made-up data; corpus `tobit`, `intreg`, `truncreg`, `cloglog`, `hetprobit`, `heckman` extended, `tobit_select` new, 1e-6, truncreg and heckman 1e-5): `iterate(#)` with Stata's `convergence not achieved` line and warning in all six; `offset()` in all six (a `(offset)` table row, e(offset) or e(offset1), predict xb with it); `noconstant` in intreg, cloglog, hetprobit and heckman `twostep`; `from(matrix [, skip])` in tobit; `stepwise` and `nestreg` with tobit (F block table, LR table, r(wald), r(lr), the Wald and LR searches, forward, backward, hierarchical, lockterm1, fweights).
- **Found:** `copy` is not allowed in tobit's from() (r(198)); the offset moves the censoring limits with the outcome and enters the constant-only fits; tobit noconstant keeps a zero `_cons` under a constraint (e(Cns), `( 1)  [y]_cons = 0`, F test, `_cons 0 (omitted)`) and uses the mean-zero model as ll_0 (matched), but Stata's start differs; truncreg's unconverged iterations come from numerical derivatives and are refused (iterate() that converges is fine).
- **On hold, listed:** see ROADMAP P2C.12ay.
- **Shares** (dry run, `many_dofiles.rs`): probes **82.2%** (23,839 translated, 5,172 not, 1,104 known only when run, of 30,115 source lines); packages **94.8%** (4,028 of 4,247), unchanged. The refused lines in the probe files that hold batch-J commands fell from 161 to 110; one batch-J line is left refused (probe 289, `tobit i.g##c.x`).
