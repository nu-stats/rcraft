# table (Stata 17+) (2026-11-14)

Observed on Stata 19 SE with made-up data (probes `r8/t1.do` to `r8/t3.do` in the session scratchpad, line by line
identical to RCraft's output; the forms the 473 probes use are all among them). Translated in
`crates/statatr/src/data_r/table.rs`; an engine test holds `t3`'s lines.

- Dimensions: `table g` (rows), `table g h` and `table (g) (h)` (rows, columns), `table (g h) ()` (two row
  variables nested), `table () (g)` (columns only). A third dimension is refused.
- Statistics: frequency (the default; with fweights "Sum of weights"), percent (of the table's total), mean, sd,
  median and p# (Stata's percentiles, R's type 2), min, max, sum, count, iqr. Their labels: Frequency, Percent,
  Mean, Standard deviation, Median, #th percentile (1st, 2nd, 3rd, 11th ...), Minimum value, Maximum value, Total,
  Number of nonmissing values, Interquartile range. Without a column variable several statistics (or one statistic
  of several variables, then headed by the variables) are columns; with one, each level gets a line per
  statistic.
- Layout: each column 2 spaces (3 after the first) and as wide as its widest header or value; the stub as wide as
  the widest label and 1, each level 2 in from its variable's name (or variable label); the column variable's name
  at 2 + ceiling((span − 2 − length)/2); rules the full width, `+` under the stub. Numbers %9.0gc (2,395;
  4,716.4), percent %9.2f, nformat(%fmt [statistics]) for those or all. A cell with no rows is blank, one with rows
  but no values `.`. Rows with a missing row or column value are left out unless `missing` (a `.` level);
  `nototals` drops the Total row and column.
- `contents()` (the syntax before Stata 17): "option contents() not allowed since Stata 17; see help table for
  updated syntax".
