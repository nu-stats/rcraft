# coefplot (SSC, Ben Jann, P2C.9)

Observed on a licensed Stata 19 SE with made-up data, 2026-11-03 (coefplot 1.8.8, installed from SSC that
day with the user's yes). Only what coefplot did is written here: the variables its `gen()` leaves and the
text of its graphs exported as SVG. Its code and help were not read.

- `gen(p_)` leaves `p_by`, `p_plot`, `p_at` (the row's place), `p_b`, `p_V`, `p_se`, `p_t`, `p_df`, `p_pval`,
  `p_ll1`, `p_ul1` (floats) and `p_mlbl`, `p_mlpos`, one row per coefficient shown, in the first rows.
- The rows are the model's coefficients by their names in e(b), `_cons` included. A base level (`1.g`) is
  left out unless `baselevels`, then it is 0 with se 0 and interval 0 to 0. A term left out for collinearity
  (`o.x`) likewise with `omitted`.
- The intervals are b ± t·se with the model's residual df (55 after `regress`); a logit's use the normal (df
  missing). `levels(90)` gives 90% intervals.
- Several models (`coefplot m1 m2 m3`): one row for each name, in the order they first appear across the
  models, `_cons` last (m3's `2.h` comes before m1's `_cons`). k plots sit 1/(k+1) apart around the row: two
  at row ∓ 1/6, three at row − 1/4, row, row + 1/4. `offset(.15)` puts a plot at row − .15; once one plot
  has an offset, the others stay on the row.
- `keep(x 2.g)` and `drop(_cons)` pick by name, with `*` wildcards; `keep(*:)` keeps all of a one-equation
  model. `order(. w x . *.g)` lists rows in that order, `.` an empty row (which `gen()` leaves as a row with
  b missing); rows not listed follow.
- `headings(x = "H1" 2.g = "H2")`: a row of its own before each named coefficient; the first takes one row,
  each later one an empty row and its own (x at 2, `2.g` at 6). `headings(..., nogap)`: no empty row (`2.g`
  at 5). Heading rows are not in `gen()`'s rows.
- `coefplot m1 || m3`: subgraphs side by side, rows placed across both; `bylabel()` names one. The options
  after the last `||` are every subgraph's, unless a subgraph has its own (`, bylabel(Safety) || ...`).
- Labels on the axis: the variable's label (`Exposure`), else its name (`w`); a factor level's value label
  (`low`, `mid`), else `g=2`; `_cons` as `_cons`. `coeflabels()` replaces them. With several plots the
  legend shows each plot's `label()`, else its model's name.
- `coefplot` with no name plots the results in memory (`p_plot` shows `.`).
- `saving(sv, replace)` writes `sv.gph`.
