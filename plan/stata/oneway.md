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
- Not translated: weights with `tabulate`, `noobs`, `missing`, `wrap`; the comparisons of more groups than the
  line holds; Bartlett's test with both single-observation and equal-valued groups (not observed).

## Weights, comparisons, nomeans/nostandard/nofreq (2026-11-13; probes `ow/o1.do`, `o6.do` identical; corpus)

- Frequency weights count rows (N their sum); analytic weights are scaled to sum to the rows. The groups'
  means and sums of squares are weighted. Bartlett's test: with frequency weights each group counts its weights
  (so one row of weight 3 is a group of several equal values, "      1 multiple-observation cells not used" in
  the note); with **analytic weights each group counts its rows**, the weighted sum of squares over the rows
  less one (checked: 0.6123 where the weight sums would give 0.5434).
- `bonferroni`, `scheffe`, `sidak`, each its table after Bartlett's test: "Comparison of y by g" centred
  (ceiling((77 − length)/2) spaces), "(Bonferroni)" under it (floor), the stub `Row Mean-|` / `Col Mean |`,
  the groups' labels cut at 8 in 11-wide columns (the first k − 1), a rule 9 + 11(k − 1); for each later group
  the differences of means in %8.0g and under them the p-values %11.3f, a `         |` line between groups.
  p: the pairwise t on the within-groups mean square, Bonferroni × k(k−1)/2 (at most 1), Sidak
  1 − (1 − p)^(k(k−1)/2), Scheffe F = t²/(k − 1) on (k − 1, df_r).
- `nomeans`, `nostandard`, `nofreq` drop those columns of the summary table; a title as wide as the columns
  goes on two lines, "Summary of" over the variable's name, each centred.

## Weights with tabulate, both kinds of left-out group (2026-11-13; probe `rest/r1.do` identical)

- With weights the summary table gains an `Obs` column (48 wide, the title centred over it): frequency
  weights show their sums as Freq. and Obs; analytic weights the raw weight sums as Freq., the rows as Obs,
  and summarize's standard deviation (Σw(x − m)²/Σw · k/(k − 1), the groups' and the total's).
- With both a single-observation group and a group of equal values, Bartlett's note has both lines, the
  single-observation one first.
