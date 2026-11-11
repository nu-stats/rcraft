# pdslasso and rlasso (SSC: pdslasso, lassopack), twoway rcap and rspike (2026-11-11)

pdslasso, and ivreg2 and ranktest which it needs, installed from SSC that day with the user's yes
(lassopack was there already); observed on Stata 19 SE with made-up data and ZNMIXH's own (probes
`pds/d1.do` to `d5.do`, `pdsz/z.do` in the session scratchpad); their code and help were not read.
Corpus `pdslasso.do`.

## rlasso y X, robust (what pdslasso runs for y and for d)

- n the rows with nothing missing; p the controls. c = 1.1, gamma = 0.1 / log(n) (e(gamma)
  .0167397 at n 393); lambda0 = 2 c sqrt(n) qnorm(1 - gamma / (2p)) (e(lambda0) 145.68).
- The first residuals: OLS of y on the five controls most correlated with it. Then, `maxpsiiter` (2)
  times: sigma = sqrt(mean(e^2)); the loadings psi_j = sqrt(mean(x_j^2 e^2)) / sigma, x_j centred
  (e(Psi); e(sPsi) is the same on standardized data); lambda = lambda0 sigma (e(lambda) 207.627 after one
  pass, 208.769 after two, = lambda0 times e(rmseOLS)); the lasso minimizing (1/n) RSS + (lambda/n)
  sum psi_j |b_j|, the constant unpenalized; the next residuals from OLS on the selected controls.
  `maxpsiiter(0)` is the same as 1.
- Checked: the loadings to 1e-9, the lasso estimates (.5572318, .2128437, -.4725665) to 1e-9, the
  selections exactly.

## pdslasso y d (controls), robust

- Prints "1.  (PDS/CHS) Selecting HD controls for dep var y..." and "Selected: x1 x3 x5" (in the
  controls' order; "Selected: " with nothing), the same for "exog regressor d", two blank lines,
  "Estimation results:", "Specification:" with the labels in 39 columns and the variable lists wrapped
  greedily at 40 characters under column 39, "Structural equation:", then three tables with z:
  - "OLS using CHS lasso-orthogonalized vars": y and d less their lasso fits, d's coefficient without a
    constant, HC0;
  - "OLS using CHS post-lasso-orthogonalized vars": the same with the post-lasso OLS residuals;
  - "OLS with PDS-selected variables and full regressor set": OLS of y on d, the union of the selections
    and the constant, HC0, then "Standard errors and test statistics valid for the following variables
    only:" and d.
- e(b), e(V): that last OLS (d, the selected controls in their order, _cons); _b[d] .117259031918289
  on the probe, ZNMIXH's .414118802807860675 and .121349437168122923 (RCraft to 5e-11).
- Table names: abbreviated to 12 by keeping more of the end until the short form matches one variable of
  the data alone (`_age_surve~n` in one dataset, `_age_surv~on` in ZNMIXH's, `abcdefgh~xyz` beside
  `...wXyz`); regress and summarize were seen to keep the first ten and the last one when nothing else
  matches (not checked when something does).

## twoway rcap and rspike

- `rcap y1 y2 x`: a vertical line from y1 to y2 at each x with caps; `rspike` without caps. Drawn with
  ggplot2's geom_errorbar and geom_linerange.
