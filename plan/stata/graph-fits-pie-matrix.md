# twoway lfitci, qfitci, area; qnorm, pnorm, rvfplot; graph pie, matrix, dir, display, drop; esttab, wide (2026-11-16)

Observed on Stata 19 SE with made-up data: the plotted values read from each graph's serset (`serset use`
after the graph, `serset set #` for a graph's second one), the titles and fills from SVG exports, and the
printed output of `graph dir` and `esttab` (probes in the session scratchpad, `gr/`).

- `twoway lfit y x`: 3 points from the data's least x to its greatest; `qfit`, `lfitci` and `qfitci`: 100.
  `n(#)` sets the number; `range(a b)` the line's ends, `.` the data's own end.
- `lfitci` / `qfitci`: the band is the least-squares fit plus or minus t(df, (1 + level/100)/2) times the
  standard error of the mean prediction (`stdp`, the default) or of a forecast (`stdf`), at `level()` (95).
  R's `predict(lm(...), interval = "confidence" | "prediction")` gives the serset's values to float precision.
- Titles: `lfit` or `qfit` alone is titled `Fitted values` on the y axis; a graph with `lfitci` or `qfitci`
  has no y title; the legend reads `95% CI` (the level as given) and `Fitted values`, then the other plots.
- `twoway area y x`: filled down to `base()`, or without one to the bottom of the y axis (its lowest label,
  5 for data from 5.3 to 8), not to 0 and not to the data's minimum.
- `qnorm y`: the sorted values against mean + sd · invnormal(i/(N+1)), with the line y = x from the first
  point's x to the last's; titled with the variable's label and `Inverse normal`. `pnorm y`: Φ((value −
  mean)/sd) against i/(N+1), the same diagonal line, the axes 0 to 1 by .25 shown as `0.00` ... `1.00`,
  titled `Normal F[(y-m)/s]` and `Empirical P[i] = i/(N+1)`. `, grid` adds percentile lines and a note:
  not translated yet. `qnorm` alone is r(100), with two variables r(103).
- `rvfplot` after `regress`: residuals against fitted values of the estimation sample, titled `Residuals` and
  `Fitted values`.
- `graph pie, over(g)`: a slice for each group's rows; `graph pie y, over(g)` each group's sum of y;
  `graph pie a b c` each variable's sum; labelled by the group's value (label) or the variable's name. The
  first slice starts at twelve o'clock and the rest go counterclockwise.
- `graph matrix a b c [, half]`: the serset keeps every row; a missing value leaves out only the points
  that need it. `half`: the cells below the diagonal.
- `graph dir`: the graphs in memory, an unnamed one as `Graph`, sorted with capitals first, in columns
  two wider than the longest name after four spaces, filled down, as many as fit the line (four of 17
  wide at linesize 80). `graph display name` shows it; `graph display` alone shows `Graph`. A name not in
  memory: `graph a not found`, r(111); `graph drop` of one: `graph zz not found` / `no graphs dropped`,
  r(111), and none of the names is dropped.
- `esttab, wide`: each model's statistic in a column of its own after the coefficient's stars, the label
  column 12, each model 13 + 3 + 13 wide, no blank lines between the coefficients; with `se` or `label` as
  without `wide`. Recorded in the corpus as `esttab4.do`.
- `kdensity x, nograph at(x) generate(d)`: one new variable, the density at each row's x.
