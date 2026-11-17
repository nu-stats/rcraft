# lroc (P2C.12)

Observed in Stata 19 on made-up data (probes 1–6, and the corpus file `lroc.do`); translated in
`crates/statatr/src/data_r/lroc.rs`.

- Printout: a blank line, `Logistic model for y` (`Probit model for y` after probit), a blank line,
  `Number of observations = ` with N as `%8.0f`, `Area under ROC curve   = ` with the area as `%8.4f`. The same
  text with or without the graph; `quietly` prints nothing. r(): `area` and `N` (N is the sum of the frequency
  weights when there are any).
- The area: the predicted probability is a float (as `predict` makes it). The curve's points are taken from the
  highest distinct probability down: sensitivity (weighted positives at or above the cut over all positives) and
  1 − specificity, each stored as a float, starting at (0, 0). The area is Σ Δ(1 − spec) · (sens + sens₋₁)/2
  summed in double. That reproduces r(area) to every digit on all probes, including its float residue
  (`.5000000074505806` for an area of one half, `.6485714228238378` where the exact area is 227/350).
  Probit's area is logit's whenever the order of the probabilities is the same.
- The outcome is positive when it is not 0 (an outcome coded 0/2 gives the same area). A sample of one outcome
  gives an area of 0, not an error.
- Rows: by default e(sample). `if` and `in` **replace** e(sample) rather than narrow it: after a fit on
  `_n > 8`, `lroc in 1/9` uses nine rows. `all` uses every row with a probability and an outcome. In each case
  rows with a missing outcome, probability or weight drop out, and so do zero frequency weights.
- `beta(matname)` needs the outcome named (`lroc y, beta(b)`); the score is Σ b·x over the matrix's columns
  (variables and `_cons`), and the sample is every row with a score and an outcome, whatever the model's
  e(sample) and with or without `all` (`all` with if is allowed then). It reads no model: it works after
  regress, and the title is the active model's (`Probit` after probit, `Logistic` otherwise).
- Weights: the fit's frequency weights by default, or lroc's own `[fw=]` (after an unweighted fit as well).
  Non-integer frequency weights: `may not use noninteger frequency weights` r(401).
- Errors seen: no model, or after regress, poisson, glm and ologit: `last estimates not found` r(301) (not
  with beta()). `pweights not allowed` r(101) after a fit with pweights, and for lroc's own `[pw=]`, `[aw=]`
  (`aweights not allowed`) and `[iw=]` (`iweights not allowed`). A variable other than the outcome without
  beta(): `varlist not allowed (except with beta() option)` and `invalid syntax`, r(198); two variables: `too
  many variables specified` r(103); beta() without the outcome: `dependent variable required with beta()
  option` and `invalid syntax`, r(198); `all` with if or in (no beta()): `all option not allowed with if or in`
  and `invalid syntax`, r(198); beta() of a matrix not made: `matrix nosuch not found` r(111); no rows:
  `no observations` r(2000). With `nograph`, any other option is an error (`option name() not allowed`,
  `option level() not allowed`, r(198)); `nog` abbreviates nograph, while `all` and `beta()` take no
  abbreviation (`a`, `al`, `bet()`, `b()` are refused by Stata).
- Not translated: the graph (Stata draws the ROC curve unless `nograph`; RCraft prints the area and does not
  draw it, and `graph export` after it is refused), its graph options, beta() whose columns are not plain
  variables and `_cons` (Stata's message there, for a matrix with default column names, comes twice),
  beta() after a weighted fit, lroc after a fit with iweights, an abbreviated outcome, and `logistic`, which
  is not translated itself.
