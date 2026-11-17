# lroc (P2C.12)

Observed in Stata 19 on made-up data (probes 1–10, and the corpus file `lroc.do`); translated in
`crates/statatr/src/data_r/lroc.rs`.

- Printout: a blank line, `Logistic model for y` (`Probit model for y` after probit), a blank line,
  `Number of observations = ` with N as `%8.0f`, `Area under ROC curve   = ` with the area as `%8.4f`. The same
  text with or without the graph; `quietly` prints nothing. r(): `area` and `N`.
- The area: the predicted probability is a float (as `predict` makes it). The curve's points are taken from the
  highest distinct probability down: sensitivity (weighted positives at or above the cut over all positives) and
  1 − specificity, each stored as a float, starting at (0, 0). The area is Σ Δ(1 − spec) · (sens + sens₋₁)/2
  summed in double. That reproduces r(area) to every digit on all probes, including its float residue
  (`.5000000074505806` for an area of one half, `.6485714228238378` where the exact area is 227/350).
  Probit's area is logit's whenever the order of the probabilities is the same. A score that is the same for
  every row (beta() of `_cons` alone) gives exactly .5.
- The outcome is positive when it is not 0 (0/2, or 0/1/3/4, work). A sample of one outcome gives an area of 0,
  not an error.
- Rows: by default e(sample). `if` and `in` **replace** e(sample) rather than narrow it: after a fit on
  `_n > 8`, `lroc in 1/9` uses nine rows. `all` uses every row with a probability and an outcome. In each case
  rows with a missing outcome, probability or weight drop out, and so do zero weights.
- Weights:
  - the fit's frequency weights by default; N is their sum;
  - after a fit with iweights, those weigh the curve (w and w/2 give the same area) while N counts the rows
    (a zero iweight drops its row);
  - lroc's own `[fw=]`, after an unweighted fit or after one with pweights or iweights;
  - `[w=]` is taken as frequency weights, with `(frequency weights assumed)` printed first (not under
    quietly).
- beta(matname) needs the outcome named (`lroc y, beta(b)`). It reads no model:
  - the score is Σ b·x over the matrix's columns: variables, factor levels (`1.g` is g == 1, `0b.g` too; `o.`
    columns such as `2o.g` and `o.g2` add nothing) and `_cons`;
  - with equations, only the first equation's columns count;
  - a missing coefficient makes every score missing, so `no observations` r(2000);
  - the sample is every row with a score and an outcome, whatever the model's e(sample) and with or without
    `all` (`all` with if is allowed then);
  - the fit's weights are ignored (after fweights, pweights or iweights alike), lroc's own count;
  - it works after regress; the title is the active model's (`Probit` after probit, `Logistic` otherwise);
  - a string outcome gives `no observations` r(2000).
- Errors seen:
  - after any model but logit and probit (regress, also with aweights, poisson, nbreg, glm with either link,
    ologit, oprobit, mlogit, clogit, xtreg, ivregress, stcox, suest, margins post, reghdfe, robreg, cluster2)
    and with no model: `last estimates not found` r(301), unless beta();
  - `pweights not allowed` r(101) after a fit with pweights; lroc's own `[pw=]`, `[aw=]` (`aweights not
    allowed`) and `[iw=]` (`iweights not allowed`), r(101);
  - `may not use noninteger frequency weights` r(401) (before the next), `negative weights encountered` r(402);
  - a variable other than the outcome without beta(): `varlist not allowed (except with beta() option)` and
    `invalid syntax`, r(198); two variables (with or without beta()): `too many variables specified` r(103);
  - beta() without the outcome: `dependent variable required with beta() option` and `invalid syntax`, r(198);
  - `all` with if or in (no beta()): `all option not allowed with if or in` and `invalid syntax`, r(198);
  - beta() of a matrix not made: `matrix nosuch not found` r(111); with more than one row, or a column
    vector: `conformability error` r(503); a column that is no variable (default names c1, c2 too): `variable
    c1 not found` twice, r(111), the first such column;
  - no rows: `no observations` r(2000);
  - `by g:` — `not sorted` r(5) on rows out of order, then `lroc may not be combined with by` r(190);
  - with `nograph`, the first other option: `option title() not allowed`, `option foo not allowed` (as typed,
    `ti()` for `ti(hi)`), r(198). Without nograph the curve's options are taken (`title()`, `ti()`, `scheme()`,
    `legend()`, `nodraw`); an unknown one is twoway's `option foo not allowed` and `invalid syntax`.
  - `nog` abbreviates nograph, while `all` and `beta()` take no abbreviation (`a`, `al`, `bet()`, `b()` are
    refused by Stata).
- Not translated:
  - the graph: Stata draws the ROC curve unless `nograph`; RCraft prints the area and does not draw it, and
    `graph export` after it is refused. Look-only options (titles, labels, scales, legend, scheme, regions,
    sizes, nodraw; full names, and `ti`) are taken and change nothing; others (name(), saving(), the curve's
    line and marker options, addplot()) are refused, since which are valid twoway options is not known here;
  - `bysort` and `by (...)`: Stata sorts the data before it stops;
  - beta() columns with interactions or `c.`; a string variable as a column;
  - `matrix colnames` with factor or equation names is the matrix translation's (refused there), so those
    columns reach lroc only through e(b);
  - an abbreviated outcome when the data's names are not known (the do-file reader writes it out in full when
    they are);
  - negative iweights; `logistic`, which is not translated itself.
