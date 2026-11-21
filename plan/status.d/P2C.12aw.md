## ROADMAP

- P2C.12aw Batch L holds (done 2026-11-21, in part)
  (`contrast` and `pwcompare` after `areg` (the Denominator is e(df_r), the absorbed groups are indicators in the design for the estimability check; after `vce(robust)` and `vce(cluster)` too) and after `xtregar` with factor variables (`i.g`: fe omits a factor constant within the panels, `contrast` prints `(not testable)`, `pwcompare` prints `0  (omitted)` for rows built only of omitted columns; a copy of a factor gives `(not testable)` / `(not estimable)`; re estimates it), `hausman` between `xtregar` fits (and beside regress; `sigmamore`, `sigmaless`, the `(V_b-V_B is not positive definite)` line) and `predictnl` after `xtregar` (all rows with covariates, `se()`, `ci()`, `variance()`), `pwcompare, cimargins` margins after `xtreg, fe` and `areg` (they printed `.`)) Verified (corpus `contrast_areg.do`, `contrast_xtregar.do`, `postest_xtregar.do`; note `plan/stata/postest.md`).
  **On hold** (listed, to be built): `contrast`/`pwcompare` after `areg` with a factor level omitted for collinearity (probe 443: areg itself refuses it; needs the batch E areg with omitted factor levels, `1o.g` columns and the `(omitted)` table rows), `manovatest` (needs `manova`, batch MV), `tebalance` (needs `teffects`, batch CA).
  Still refused: `xtregar` with interactions, `ib.` bases or time-series operators; `contrast` with `post`, `noestimcheck`, `pveffects`, `cieffects`, `groups`, `tukey`, `snk`, `duncan`, `dunnett`.
  Measured on top of P2C.12av (probes only; `observe/dofiles`): 80.7% -> 80.7% (23,399 -> 23,426 translated source lines of 29,011 judged; 5,612 -> 5,585 not translated); packages unchanged at 94.7% (4,025 of 4,249; none of these lines is in the 46 do-files).)

## STATUS

## P2C.12aw: batch L holds (contrast and pwcompare after areg and xtregar, hausman and predictnl after xtregar), 2026-11-21

Built in `crates/statatr/src/data_r/postest.rs`, with `xtregar.rs` taking factor variables and small hooks in `data_r.rs` and
`returned.rs`. Checked against Stata 19 in three corpus files (`contrast_areg`, `contrast_xtregar`, `postest_xtregar`), the full
corpus passes. Rules decoded are in `plan/stata/postest.md`. Probes 80.7% -> 80.7% (+27 translated lines), packages 94.7% unchanged.
On hold: contrast/pwcompare after `areg` with an omitted factor level (probe 443; areg refuses it), `manovatest`, `tebalance`.
