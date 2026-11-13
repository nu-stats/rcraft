# tabstat (2026-11-12)

Observed on Stata 19 SE with made-up data (probes `ts/t1.do` to `t4.do` in the session scratchpad, each log
set line by line against RCraft's output); corpus `tabstat.do`. Translated in
`crates/statatr/src/data_r/tabstat.rs`.

## Statistics

- Names and the shortest forms Stata takes: mean (me), count (co) and n → N, sum (su), max (ma), min (mi),
  range (r), sd, variance (v), cv, semean (sem) → se(mean), skewness (sk), kurtosis (k), median (med) → p50,
  iqr, q → p25 p50 p75, and p1 p5 p10 p25 p50 p75 p90 p95 p99 only. `s`, `c`, `se`, `p`, `i`, `iq`, `p2`,
  `p33`, `p0`, `p100` are "unknown statistic: x", r(198). A statistic may be named twice.
- The values are summarize's: sd over n - 1 (fweights: their sum - 1; aweights: scaled to the rows),
  skewness and kurtosis over the whole sample, percentiles by Stata's rule (with weights, the first value
  whose running sum of weights passes p·W, the mean with the next one at equality). With aweights N is the
  rows and Sum is Σ w·x with the weights as given; with fweights N is Σ w. semean = sd/√N, cv = sd/mean.
- One value: N 1, the mean, sd, skewness, kurtosis, se(mean) and CV missing. No value in a group: N 0,
  Sum 0, the rest missing.
- Each variable uses its own non-missing rows (N differs by variable) unless `casewise`.
- A string variable: "string variables not allowed in varlist; st is a string variable", r(109). No rows:
  "no observations", r(2000). pweights and iweights: "pweights not allowed", r(101).

## The table

- Columns 10 wide: a space and the value in %9.0g (or `format()`), names right-aligned; variable names cut
  to 8 as `averyv~e`. Rules: dashes to the stub, `+`, dashes over the columns; the closing rule is the
  whole width.
- Without `by()`: one variable → the statistics across ("Variable" stub, 12 wide, `varwidth()` 8..32,
  names cut as `averyveryv~e`); more → the variables across ("Stats" stub, 8 wide). `columns(statistics)`
  / `columns(variables)` (c(s), col(v), ...) choose.
- With `by()`: "Summary for variables: x y" (statistics across) or "Summary statistics: Mean, SD"
  (variables across), then "Group variable: g " and the variable's label in parentheses when it has one.
  The groups in sorted order by their value labels, else their values in %9.0g, then "Total". A group of
  more than one row gets a rule after it (`noseparator`, nosep, drops them); one-row groups get none but
  the one before Total. Rows with a missing group are left out of the groups and the total unless
  `missing` (the group "."). `nototal` drops Total.
- The group's stub: unlabelled numbers 8; value labels the longest label of the whole label set, used or
  not; strings their storage width (str#). That, at most `labelwidth()` (16; 8..32, below 8 is "(option
  labelwidth() outside valid range 8..32; 8 assumed)"), at least 6 and never narrower than the group
  variable's name. Labels are cut to it, not abbreviated.
- `longstub`: the group's label left-aligned in its width, a space, then the "Stats" or "Variable" stub;
  no "Summary ..." lines. `varwidth()` with `by()` gives the long stub too.
- At the line size the columns go in blocks: as many as fit floor((linesize - stub - 2)/10), each block a
  table of its own after a blank line; the "Summary ..." lines once. Batch Stata's line size is 79.
- `format(%9.2f)` and `format(%8.3g)` keep the 10-wide columns; `format` alone uses each variable's own
  format and widens every column to the widest (see below).

## r()

- Without `save` r() is cleared. With it: r(StatTotal) (none with `by()` and `nototal`), r(Stat1) ... one
  a group, and r(name1) ... the groups' labels in full; the matrices are statistics by variables whatever
  `columns()` says, rows named by the labels (Mean, SD, N, p50, ...). `return list` lists the names from
  the last group down, then the matrices.

## Option forms Stata 19 takes

by(), s()/st()/stat()/stats()/statistics(), c()/col()/columns(), f()/format(), la()/labelwidth(),
va()/varwidth(), case/casewise, not/nototal, m/missing, nosep/noseparator, lo/longstub, save. `nos`, `sa`,
`ca`, `v()` are not options (r(198)).

## format and wider formats (2026-11-13; probe `ts/t5.do` identical)

- The columns are one wider than the widest format in use, never under 10 (`format(%12.4f)` 13, `%14.2f` 15),
  and the blocks at the line size count that width. `format` alone uses each variable's own display format:
  in its column when the variables go across, in its row when the statistics do (N included: `12.0000` under
  %12.4f); a left-justified format's text is right-aligned in the column with its padding (`%-9.2f`).
- RCraft knows a variable's display format when `format` set it or the data came from a .dta (haven keeps
  it); for a variable only made by `generate` (Stata: %9.0g for float, %10.0g for double) it stops and says so.
- (2026-11-13) A variable `generate` made has its storage type's default format, as Stata gives it: float
  %9.0g, double %10.0g, long %12.0g, int and byte %8.0g (probe `ts/t5.do` identical).
