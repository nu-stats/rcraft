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
- `se` with `tex(fragment)`, `lab`, `ctitle()`, `addtext()`, `addstat()`: done on 2026-11-06, below.

## import delimited

- Stata skips an empty line but keeps one of delimiters alone as a row of missing values (blank strings).

## se, tex(fragment), label, ctitle(), addtext(), addstat() (2026-11-06)

As TZEN38 runs it after `xtreg` (`outreg2 using f.tex, bdec(3) tdec(3) se tex(fragment) lab ctitle(..)
keep(..) addtext(..) [addstat(R-squared, e(r2_o))]`). Observed in Stata 19 with made-up data, 26 files
matched byte for byte (`outreg2tex.do` keeps the checks):

- `se`: the standard error in parentheses with 3 decimals whatever `tdec()` says; the footer is
  "Robust standard errors in parentheses" when the first column's `e(vcetype)` is Robust, else
  "Standard errors in parentheses"; a column added keeps the table's footer.
- `tex(fragment)`: f.tex beside f.txt: `\begin{tabular}{l` and a `c` per column `} \hline`, the rows
  joined by ` & ` and ended by ` \`, the VARIABLES row and the last statistic ending ` \ \hline`, the
  last statistic with a space in front, then `\multicolumn{k+1}{c}{ footer} \`, the stars line with
  `$<$`, `\end{tabular}`. In a row's first cell `_`, `%` and `#` are escaped, nothing else; addtext()
  values stay as written.
- `label`: a LABELS column after the names in f.txt (the variable's label, its name when it has none,
  nothing for an interaction, Constant), and the label in place of the name in the .tex; without
  ctitle() the column's heading is the dependent variable's label. A column without label added to a
  table with LABELS keeps the column.
- `ctitle(text)` is the column's heading. `addtext(name, text, ...)` rows as written; `addstat(name,
  exp, ...)` rows with 3 significant digits (12346, 0.0123, -0.500).
- The statistics rows: a column alone has its model's order (Observations, R-squared, Adjusted
  R-squared; xtreg fe: Observations, Number of id, R-squared; xtreg re: Observations, Number of id),
  then e(), addtext() and addstat() rows. A column added to a table: Observations and the R-squareds
  first, the Number of rows, the e() rows sorted by name, then the other rows as they were in the
  table, then this column's. An addstat() named R-squared is a row of its own where a model gives none.
- `keep(c.a##c.b)` is a, b and c.a#c.b; factor variables are taken when keep() leaves them out.

Found on the way: Stata 19 keeps backslashes on a do-file line as written (probe 139 again: only
version 8.2 and before halve a run); RCraft's reader halved `\` (StatCraft's front end does too).
