# margins after clogit, and parmest (P2C.9)

Observed on Stata 19 SE with made-up conjoint data (1,200 rows, pairs of profiles, 100 respondents as
clusters, three factors with fvset bases), 2026-11-03. parmest was installed from SSC with the user's
go-ahead; only the files it saved were read, never its code.

## margins after clogit

- The default prediction is pu0 (`e(marginsdefault)` is `predict(pu0)`): `Expression: Pr(chosen|fixed effect
  is 0), predict(pu0)`.
- `margins ar.actor ar.reform ar.opinion, predict(pu0)`: `Contrasts of predictive margins`, `Model VCE:
  Robust`; a table of df, chi2 and P>chi2 for each contrast (`(2 vs 1)`, `(3 vs 2)`, ...) and a Joint row per
  term; then the contrasts with Delta-method standard errors and intervals. r(b) is 1 x 9 with columns
  `ar2vs1.actor` ... `ar3vs2.opinion`; r(V) beside it. Each contrast is the predictive margin at a level less
  the one at the level before.
- `margins, dydx(*) post predict(pu0) asobserved vce(unconditional) mcomp(bonferroni)`: `Average marginal
  effects`, `(Std. err. adjusted for 100 clusters in id_g)`, a table of the number of comparisons per term
  (4, 3, 2), and the effects with `Unconditional` standard errors and `Bonferroni` p-values and intervals.
  e(b) is 1 x 12 with the base levels as zeros in their place (`1.actor` ... `4.actor 5b.actor 1.reform ...
  4b.reform 1b.opinion 2.opinion 3.opinion`); e(V) is not adjusted for Bonferroni; e(N_clust) is 100,
  e(vce) `unconditional`.
- The unconditional standard errors match the rows' scores of the exact likelihood times the inverse
  information, added to each row's part of the margin, summed by cluster, times G / (G - 1).

## parmest

- `parmest, idnum(7) saving(f, replace)` saves 12 rows, one per column of e(b), and leaves the data in memory.
- After clogit (and after margins without post, when e() is still clogit's): idnum (byte, `Numeric ID`),
  eq (`Equation name`, `chosen`), parm (`Parameter name`, `1.actor` ... `5b.actor`), estimate (`Parameter
  estimate`), stderr (`SE of parameter estimate`), z (`Standard normal deviate`), p (`P-value`), min95 and max95
  (`Lower 95% confidence limit`, `Upper ...`).
- After margins, post: the same without eq. A base level: estimate and stderr 0, z and p missing, min95 and
  max95 0. p and the limits are not adjusted for Bonferroni.
