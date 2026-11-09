# alpha (2026-11-08)

ZNMIXH builds several survey indexes with `alpha items, gen(index)`. Observed in Stata 19 on
made-up data; `alpha.do` keeps the checks (to 1e-13).

- Test scale = mean(unstandardized items), or mean(standardized items) with `std`; "Reversed
  item:  c" names the items entered with a minus sign. An item reversed is its negative (-c, not
  max + min - c). Which: those with a negative loading on the first principal factor of the
  complete rows' correlations (squared multiple correlations on the diagonal), the loadings
  oriented to sum above 0: it matched Stata in 200 of 200 random data sets of five items, where
  the sign of the correlation with the other items matched in 35 and the principal component in 181.
- generate(): a float labelled like the test scale. Each row: the items present, at least min()
  of them (1 by default; with `casewise` all of them, else missing), summed in the varlist's order
  with the running sum rounded to float at each step, the mean then rounded to float. (The plain
  double mean rounded once differs in the last bit for about one row in six.) With `std` each item
  is first standardized by its own mean and standard deviation over its nonmissing rows.
- r(k) always. With `casewise`: r(cov) is the mean of the off-diagonal covariances of the signed
  items over the complete rows, r(alpha) = k cov / (var + (k - 1) cov), var the mean variance;
  with `std`, r(rho) the mean correlation, r(alpha) = k rho / (1 + (k - 1) rho). All bit for bit.
- Without `casewise` Stata's average covariance uses the incomplete rows in a way not matched yet
  (pairwise covariances, with N or N - 1, and global means were all tried): not kept.
