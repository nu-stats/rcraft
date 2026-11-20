# contrast, pwcompare, hausman, lrtest, predictnl (batch L, 2026-11-20)

Observed on Stata 19 SE with made-up data (corpus `contrast.do`, `contrast_glm.do`, `contrast_omitted.do`, `hausman.do`,
`lrtest.do`, `predictnl.do`). StatCraft's notes (`contrast.md`, `panel.md`, `predictnl.md`; ported from statcraft@b7d392e)
had the rules for estimability, the multiple-comparison methods and hausman; the rest was found here by runs. Black box
only. `crates/statatr/src/data_r/postest.rs` is the result.

## contrast and pwcompare

- Margins are "asbalanced": a term's variables at their levels, every other factor of the model averaged over its levels
  with equal weights, continuous covariates at zero. Each margin is the mean of R's own model matrix over those cells.
  The table of a model with `i.g##c.x1` (checked: contrast g gives F 28.55 where the plain model gives 28.83) agrees with
  covariates at zero.
- Operators: plain (all margins equal: df = levels - 1), `r.` (against the factor's base level, so `ib2.g` gives
  `(1 vs 2)`), `ar.` (each against the one before), `a.` (each against the one after: `(1 vs 2)`, `(2 vs 3)`),
  `gw.` (against the mean weighted by the sample's counts), `g.` (against the plain mean); both label `(j vs mean)`.
- Terms: `g#h` (the interaction's double differences, one test), `g@h` (g's test within each level of h, then Joint),
  `r.g@h` (rows by contrast, then level: `(2 vs 1) 1`), `r.g#r.h` (`(2 vs 1) (2 vs 1)`), a plain variable beside an
  operator one reads `(joint)`; several terms in one command each print their rows, a blank `|` line between them.
  A term with one row prints it on the term's own line (no Joint). A term given twice is refused here.
- The estimates table is shown for terms whose variables all have operators, and for any term with `effects` or an
  `mcompare()`; a plain term then reads `(2 vs base)`, `(2 vs base) (2 vs base)`. Without `effects` it has the interval
  only (48 wide); with it t (or z) and P (64 wide).
- Tests: F with e(df_r) after regress, anova, xtreg fe and the other linear models (a `Denominator` row), chi-squared
  after logit, probit and poisson (no Denominator). The rank comes from a generalized inverse made by sweeping the
  largest remaining variance first (after `vce(cluster)` on two clusters the 2 contrasts have rank 1: Stata's F uses the
  contrast whose variance is larger and df 1, and its Denominator is the clusters - 1).
- `mcompare(bonferroni|sidak|scheffe)`: p x m, 1 - (1 - p)^m, F = t^2 / rank(term); the interval uses alpha / m,
  1 - (1 - alpha)^(1/m), sqrt(rank x F critical); m is the number of rows of the term. The tests table gets an adjusted
  column (a header with the method's name; Joint has none) when a term shows rows; below it
  `Note: <Method>-adjusted p-values are reported for tests on individual contrasts only.` word-wrapped at the table's
  width with six blanks on the next line; then the `Number of comparisons` table. `Scheffe` and `Sidak` are spelled
  without accents. The estimates table's method label sits centred on column 33 (effects: and again 21 later) or 38
  (interval only) after the bar.
- `pwcompare`: every pair of cells, `j vs i` in the order of the first cell (`(2 1) vs (1 1)` for `g#h`); after
  logit, probit and poisson the equation (the dependent variable) heads the table. `cimargins` shows the margins.
- Not estimable (probes 442 and 443): the contrast's weights must lie in the row space of the design including the columns
  Stata omitted (so averaging over a factor that is a copy of another gives `(not testable)` / `.  (not estimable)`), and
  after `xtreg, fe` an indicator for each panel is in the design (a factor constant within panels is not estimable).
- r(): contrast leaves r(b), r(V), r(table) (the contrasts: one column per row of every term, plain terms too),
  and r(F) (or r(chi2)), r(df), r(p), r(df2): one column per displayed row of each term, then its Joint when it has several.
  pwcompare leaves r(b_vs), r(V_vs), r(table_vs) (the comparisons, adjusted) and r(b), r(V), r(table) (the margins,
  unadjusted), r(mcmethod_vs), r(mctitle_vs), r(balanced) = 0. Not kept: r(L), r(M), r(error), r(k_groups), r(groupsN).
- Error: a variable that is not a factor of the model: `factor x1 not found in list of covariates`, r(111).

## hausman

`hausman name1 [name2] [, constant sigmamore sigmaless]` over stored results or `.`: the coefficients both sets have (the
constant only with `constant`, a coefficient of variance 0 skipped), chi2 = d'(V_b - V_B)^-1 d with the inverse taken by
Gauss-Jordan and df its rank; a negative chi2 prints with Stata's warning and p = 1. `sigmamore` multiplies V_b by
(rmse_B / rmse_b)^2 and `sigmaless` V_B by the inverse; after `xtreg, re` e(rmse) is the root MSE of the quasi-demeaned
regression (`rmse_gls` in the fit). A name not stored: `estimation result X not found`, r(111). r(): rank, chi2, df, p.

## lrtest

`lrtest name1 [name2]` (one name: the other is `.`): twice the difference in log likelihoods on the difference in e(rank);
`Assumption: small nested within full` names the model with fewer parameters. r(p), r(chi2), r(df). Stops: the same model
twice (`models full specified more than once`, r(198)), equal degrees of freedom (`df(unrestricted) = df(restricted) = 3`,
r(498): also when the rows differ), a robust model (`LR test likely invalid for models with robust VCE`, r(498): refused
here), a name not stored (r(111)).

## predictnl

`predictnl [double] newvar = exp [if] [in] [, se() variance() wald() p() ci() level()]` after regress, logit, probit and
poisson: the expression at each row (data variables, `_b[]`, `xb()`, `predict(xb|pr|n)`), the standard error by the
delta method with the derivatives taken numerically at e(b) (as nlcom does; the values agree with Stata's to 1e-6).
New variables are float unless `double`. Messages, in order: `(# missing values generated)`, `warning: prediction constant
over observations; perhaps you meant to run nlcom.`, `note: confidence intervals calculated using Z critical values.`,
`note: p-values are with respect to the chi-squared(1) distribution.` An old name: `variable p already defined`, r(110).

## Open (refused here)

- `areg` and `xtregar` as the model (their results are not held as a model here), so probes 418 and 443 lines for them stay
  refused; contrast after other models; `post`, `noestimcheck`, `atequal`, `asobserved`, `pveffects`, `cieffects`,
  `groups`, `tukey`, `snk`, `duncan`, `dunnett` (as for `pwmean` in StatCraft, not decoded).
- `manovatest` needs `manova` (batch MV), `tebalance` needs `teffects` (batch CA): on hold.
- `lrtest`: `stats`, `force`, `using`, `lrtest` of models with different numbers of observations beyond the df rule.
- `hausman`: `alleqs`, `equations()`, `df()`, `tconsistent`, `rhs`; sets other than regress, xtreg, ivregress, logit, probit, poisson.
