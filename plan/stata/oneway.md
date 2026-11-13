# oneway (2026-11-12)

Observed on Stata 19 SE with made-up data (probes `ow/o1.do` to `o3.do` in the session scratchpad, their
logs set line by line against RCraft's output); corpus `oneway.do`. Translated in
`crates/statatr/src/data_r/oneway.rs`.

- `oneway y g` uses the rows where y and g are not missing (an empty string group counts as missing). A
  string g is a group variable like any other.
- The analysis of variance, after a blank line:

                          Analysis of variance
      Source              SS         df      MS            F     Prob > F
  ------------------------------------------------------------------------
  Between groups      456.414578      3   152.138193     15.57     0.0000
   Within groups      1104.21487    113   9.77181301
  ------------------------------------------------------------------------
      Total           1560.62945    116   13.4537021

  SS and MS in %11.0g (9 significant digits) right-aligned in 16 and 13, df in 7, F %10.2f, Prob %11.4f.
  With one group: SS 0, df 0, MS "." and no F.
- Then, after a blank line, `Bartlett's equal-variances test: chi2(3) =   0.4461    Prob>chi2 = 0.931`
  (%8.4f, %5.3f), over the groups with a positive variance; with single-observation groups, after a blank
  line, "note: Bartlett's test performed on cells with positive variance:" and
  "      1 single-observation cells not used".
- `tabulate`: before it, the groups' table, "Summary of y" centred over 36 columns, the stub 11 wide (the
  group variable's name split over two header lines past 11), labels cut at 9; Mean and Std. dev. in
  %10.0g (8 significant digits) and Freq. in 12-wide columns; a single observation's Std. dev. is 0.
  `nolabel` shows the values.
- r(): N, mss, df_m, rss, df_r, F, chi2bart, df_bart and r(ANOVA), 3 × 5 (rows "Between groups", "Within
  groups", "Total"; columns SS df MS F p, F and p only on the first row). With `noanova`, r() is left
  empty and, without `tabulate`, nothing is printed.
- Not translated: weights, `bonferroni`, `scheffe`, `sidak` (a "Comparison of y by g" table),
  `nomeans`, `nostandard` (the title then wraps), `nofreq`, `noobs`, `missing`, `wrap`; Bartlett's test with
  a group of several equal values (not observed) stops with that reason.
