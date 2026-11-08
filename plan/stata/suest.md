# suest, estimates table, isvar and an exact fit (2026-11-07)

Observed in Stata 19 on made-up data. isvar is Nick Cox's add-on from SSC, already in Stata's PLUS
folder on this machine; its program file was not read. Corpus files: `suest.do`, `isvar.do`,
`regress_exact.do`, `clogit_margins.do`.

## suest (YFBPT6 analysis_polknow.do)

The package fits `clogit chosen i.actor i.reform i.opinion if know==1, group(csid)` and the same
with copies reform2 and opinion2 if know==0, stores them as co and fls, and runs `suest co fls,
vce(clu id_g) coefl`, then `margins, dydx(1.reform 1.reform2)
expression(invlogit(predict(equation(co_chosen))) + invlogit(predict(equation(fls_chosen)))) post`
and `test 1.reform = 1.reform2`.

- e(b): each model's own e(b), base levels as zeros, one after another; the equations are named
  name_depvar (co_chosen, fls_chosen). e(N) counts the rows of any model, e(N_clust) the clusters,
  e(rank) the coefficients estimated; e(eqnames1) "chosen", e(names) "co fls", e(vcetype) "Robust".
- e(V) = D (U'U) D G / (G - 1): D the models' own (model-based) variances side by side; U the rows'
  scores, each model's on its own rows and zero elsewhere, summed within clusters. A clogit row's
  score is x times (y less its probability of being the group's positive). Without vce(cluster)
  the clusters are clogit's groups (csid), as clogit's own robust errors take them. All of e(b) and
  e(V) match to 1e-15 relative.
- margins after it: the mean over suest's rows of the discrete change in the expression from the
  base level, the delta method through e(V); posted, e(b) holds each level asked for and its base
  in level order (1.reform 4b.reform 1.reform2 4b.reform2). The test after it is the usual Wald test.
- Cross terms between two equations whose clusters do not overlap are exactly zero in R and about
  1e-14 in Stata (its numerical derivatives).

## estimates table

Base levels are left out; with `star`, * ** *** at .05, .01 and .001; with `stats()`, those e()
scalars at the foot; `se` puts standard errors under the coefficients. It leaves r(names), r(coef)
and r(stats). RCraft prints the table in R's layout and keeps r(names) only.

## margins dydx(#.v) and e(b) after margins, post

- `dydx(1.reform 3.opinion)`: the effects of those levels only; r(b) holds each and its base level,
  in level order (1.reform 4b.reform 1b.opinion 3.opinion), as e(b) does after post.
- After `margins, dydx() post` with effects alone, e(b) is laid out as Stata's, so `matrix list
  e(b)` is translated there (R checks the layout and stops otherwise).

## isvar (SSC)

- Each word that names variables as a varlist would (an abbreviation, a pattern, a range) gives
  them to r(varlist), in order, repeats kept; any other word goes to r(badlist) as written: an
  unknown or ambiguous name, a different case, `_n`, `1x`, `i.x`, `l.x`.
- It shows "variable(s): ..." and "not variable(s): ..."; r() is cleared and an empty list is not
  set. With no names, r(100).

## wordcount()

Words are what spaces and tabs separate; quotes are not special (`wordcount(`"a "b c" d"')` is 4).

## A fit Stata takes as exact (regress)

- When the residual sum of squares is below about 1e-14 of the centred total sum of squares,
  Stata reports e(rss) 0, e(mss) the total, e(r2) and e(r2_a) exactly 1, e(rmse) 0, e(F) and e(ll)
  missing, and standard errors of 0 (robust ones are kept). Above it, e(rss) is the residuals'.
- The cut-off is relative to the centred total (shifting y changes nothing) and the same for N 12,
  40 and 200 and one or two regressors: zero up to 9.87e-15, kept from 9.98e-15. R refuses the band
  between, and the case with weights (not observed).
- A package's own program (ZNMIXH lassoClean.ado, run as part of the package) relies on it:
  `if e(r2)==1` drops a variable that another explains fully.
