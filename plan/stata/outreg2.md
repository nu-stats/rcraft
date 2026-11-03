# outreg2 (SSC), as it behaves (P2C.9)

Installed with the user's go-ahead (`ssc install outreg2`, 2026-11-03) and run on Stata 19 SE with made-up
data. Only the files it wrote were read, never its code. RCraft's files were compared byte for byte with these.

## The files

- `outreg2 using t1.xls, ...` writes t1.txt and t1.xls and prints `t1.xls` and `dir : seeout`. With a name
  and no extension it writes the .txt alone.
- t1.txt: cells separated by tabs, every row as wide as the table, lines ending in CR LF on Windows. Rows: a
  blank cell and `(1)`, `(2)`, ...; `VARIABLES` and each model's dependent variable; a blank row; for each
  variable its coefficient and, below, its t statistic in parentheses; `Constant` last; a blank row; the
  statistics; then `t-statistics in parentheses` and `*** p<0.01, ** p<0.05, * p<0.1`.
- t1.xls: Excel's XML spreadsheet on one line, no line end after it. The same 2,101 characters before the
  worksheet in every file; then one empty row and a row per line of the .txt, every cell a String; styles
  s31 and s21 (a top border) for the first row and the row after the variable names, s33 and s23 (a bottom
  border) for the last statistics row, s32 and s22 otherwise, and s24 for the two notes, which have one cell;
  the first column 6 points per character of its longest entry (notes aside), the others 58; `<` written
  `&lt;`.
- Without `replace` the model is a new column of the table in t1.txt. A variable it adds comes before
  Constant, in the model's order; `keep(w)` keeps w and Constant.

## Numbers (with tstat bdec(3) tdec(2) parenthesis(tstat), as the packages write it)

- Coefficients: bdec decimals and thousands commas (`536,186.716***`); stars at p < 0.01, 0.05 and 0.1 from
  the model's t (or z) tests (reghdfe with clusters on G - 1).
- t statistics: tdec decimals in parentheses.
- Observations: the count. `R-squared` (shown unless adjr2 alone is asked for, or with r2) and `Adjusted
  R-squared` (adjr2): 3 decimals. `e(name)`: 3 significant digits (`0.0396`), a whole number without decimals.
- The statistics rows: Observations, R-squared, Adjusted R-squared, then the e() ones in the order of their
  names (`N_clust`, `r2_a`, `r2_a_within`), whichever column brought them.

## Not matched yet

- Its own number format without bdec() and tdec() (`0.000887`, `(7.83e-05)`, `-1,011`, `(994.5)`).
- `se` with `tex(fragment)`, `lab`, `ctitle()`, `addtext()`, `addstat()`.

## import delimited

- Stata skips an empty line but keeps one of delimiters alone as a row of missing values (blank strings).
