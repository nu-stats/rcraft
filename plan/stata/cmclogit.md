# cmset, cmclogit, margins after it, coefplot of several equations (2026-11-12)

Observed on Stata 19 SE with made-up data (probes `cm/c1.do`, `c4.do`, `c7.do` in the session
scratchpad) and YFBPT6's analysisall.do and analysisall_subgov.do; corpus `cmclogit.do`.

## cmset caseid altvar

- Prints "\n     Case ID variable: X\nAlternatives variable: Y\n" (the labels right-aligned).

## cmclogit depvar [alt terms], casevars(terms) [noconstant vce(cluster v)]

- The conditional logit: each case one stratum, P = exp(xb) / sum over the case's alternatives.
- Alternative-specific terms: `x`, `i.x`, `ib#.x` (fvset base respected); a factor's base level a 0
  column shown as `(base)`.
- Case-specific terms (casevars() and the constants): one coefficient for each alternative but the base
  one. The base alternative is the one most frequent in the data, the first of a tie (not the one most
  chosen). Each such equation is named by the alternative's value (`2:2.country`, `2:_cons`).
- vce(cluster v): H^-1 (sum over clusters of S_g S_g') H^-1 times G/(G - 1); Wald chi2 of the free
  coefficients.
- e(): N, N_case, N_clust, ll, df_m, chi2, p, k_alt, alt_min/avg/max, i_base, cmd, depvar, caseid,
  altvar, title, clustvar, vce, vcetype, properties, b, V. The header and the coefficient table as
  Stata prints them; the iteration log is not (Stata's starting values were not found).

## margins, dydx(i.x ...) outcome(#) alt(#) asobserved post [vce(unconditional)] [cformat()]

- For each level k of each factor: the mean over cases of Pr(outcome) with the alternative's x at k
  less Pr(outcome) with it at the base level, the other alternatives as observed.
- The delta method with the analytic gradient P_o (x_o - sum_j P_j x_j); vce(unconditional) clusters
  each case's term (e_i - mean)/n + IF_b J' by vce(cluster) of the model.
- The table: "Unconditional" over Std. err., "(base outcome)" for the base levels, and the note "dy/dx
  for factor levels is the discrete change from the base level."
- e(b) has an equation for each level: `1.actor:_cons`, the base `3b.actor:o._cons` (0, its variance
  0).

## coefplot of a model with several equations

- With no keep(), only the first equation's coefficients are shown; `keep(*:)` shows all.
- `omitted` shows the `o._cons` base rows (0); `baselevels` alone does not.
- Coefficient lists: `eq: name` (the equation kept for the names after it: `keep(2.actor:_cons
  _cons)` is one row), `eq:` alone all of its names, a bare name the first equation's.
- headings() and coeflabels() keys match equation names exactly (`1.reform:` does not match
  `1b.reform`). With noeqlabels, the rows are 1..n with no gaps between equations; headings() as
  for one equation (an empty row before each after the first); headings(..., offset(-1)) moves the
  heading's label one row up.
- coeflabels(k = `" "line one" "line two" "') is a label of two lines; coeflabels(..., wrap(#))
  breaks each line at blanks to at most # characters.
- Without noeqlabels the equations' names are drawn as groups with empty rows between them: refused.
