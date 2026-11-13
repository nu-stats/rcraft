# tabulate's summarize(), matrices, lrchi2 (2026-11-13)

Observed on Stata 19 SE with made-up data (probes `r2/a1.do` to `r2/a4.do` in the session scratchpad, set line by
line against RCraft's output: identical); corpus `tabulate_more.do`. Translated in
`crates/statatr/src/data_r/tabsum.rs` and `stats.rs`.

- `tabulate g, summarize(y)`: oneway's summary table exactly (`Summary of y` over Mean, Std. dev., Freq.; the
  title on two lines when the columns are narrower; labels cut at 9; Obs with weights or `obs`; analytic
  weights: the weight sums as Freq., the rows as Obs, summarize's sd). Rows with y missing are left out.
- Two ways: a title naming the statistics shown ("Means, Standard Deviations and Frequencies of y", "Means of
  y", "Means and Frequencies of y", "..., Frequencies and Number of observations of y" with weights) at
  max(2, (75 − length) %/% 2) spaces, the same at line size 80 and 255; the column variable's name at
  ceiling((10k − length)/2) past the bar; the frequency table's columns (row stub 10, cells 10 wide with a
  space between, `|` and Total); each statistic a line, %10.0g (8 digits), `.` for an empty cell's mean and sd
  (its Freq. 0), sd 0 for one observation; a rule after each group when a cell has more than one line; labels
  cut at 9; five columns a block, the title once.
- `means`, `standard`, `freq` choose the statistics, `nomeans`, `nostandard`, `nofreq`, `noobs` drop them;
  `obs` adds Obs. r() is empty after either form.
- `matcell()`, `matrow()`, `matcol()`: the counts (analytic weights scaled to the rows), the row values as a
  column and the column values as a row, named r1.., c1..; `matrow()` of a string variable: "option
  matrow() not allowed", before the table.
- `lrchi2`: " Likelihood-ratio chi2(2) =   1.1692   Pr = 0.557" after the Pearson line (a blank line before it
  when alone), with frequency weights too; r(chi2_lr), r(p_lr) after r(chi2), r(p). `nokey` drops the Key box.
- `summarize y, mean` is `meanonly`.
