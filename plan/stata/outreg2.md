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

- (Done 2026-11-20: its own number format, below.)
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

## onecol (2026-11-19)

As BraderTuckerDuell writes it (`outreg2 using "table2.tex", replace tex(frag) bdec(4) onecol`). Observed in Stata 19
with made-up data: after `regress`, `xtreg, fe` and `reghdfe`, with `tstat` or `se`, to .xls/.txt and to .tex with
`tex(fragment)`, and a column added to a table, the files written with `onecol` are byte for byte the ones written
without it (onecol puts the equations of a model of several in one column; these have one). RCraft takes `onecol`
after those three models (corpus `outreg2_onecol.do`) and refuses it after the others. The package's own lines stay
refused for other reasons: they follow `tobit`, and they have neither `tstat` nor `se` (outreg2's own layout) nor
`tdec()`, and write `tex(frag)`.

## Its own layout, number format, stats(), word, tex alone (2026-11-20, batch N)

Observed in Stata 19 with made-up data and the files compared byte for byte (corpus `outreg2_layout.do`, each file
read back whole). Ported from the behaviour StatCraft matches (probe 136), not from the package's code.

- **No `tstat`, no `se`**: the standard errors, as with `se`. `bdec(#)` alone: the coefficient has # decimals and the
  standard error 3; `sdec(#)` sets the standard error's decimals; `tdec()` is for `tstat` only (with `se` it is ignored).
- **Own number format** (no `bdec()`/`sdec()`/`tdec()`; also the p value, the confidence limits and a t statistic
  without `tdec()`), by the size of the number: 1000 up to a million without decimals, with commas
  (999,999.6 is `1,000,000`); 100 up, 1 decimal; 10 up, 2; 1 up, 3 (so 4 significant digits from 1 up); below 1 three
  significant digits, i.e. 3 decimals from 0.1, 4 from 0.01, 5 from 0.001, 6 from 0.0001; e-notation with 3 decimals from
  a million (`1.000e+06`) and with 2 below 0.0001 (`9.90e-05`); under about 5e-11 the number is written with no
  decimals (`0`, and `-0` for a tiny negative). The size is taken before rounding (9.9996 is `10.000`).
- **After logit, probit and poisson** the names are the plain ones (no equation name), `Constant`, and no R-squared
  (only Observations).
- **`nocons`**: no Constant row. **`noobs`**, **`nor2`**: no Observations row, no R-squared rows. **`noaster`**: no stars
  and only the first note under the table. **`alpha(a, b, c)` with `symbol(s, t, u)`**: the first symbol for the smallest
  level; the note lists them as typed (`** p<0.01, * p<0.05`). Taken with `replace` and a .txt or word only.
- **`stats(...)`**: `coef` alone: one line per variable; `coef se`: the same as the default; `coef tstat`, `coef pval`,
  `coef ci`: two lines (ci is `(low - high)`, 95%, in the own number format); `coef se tstat pval`: four lines, the
  standard error and the p value in parentheses, the t statistic not. The table has a row with the dependent
  variable's name above the variables, as tall as a variable (so no such row for `coef se`). The note names what is in
  parentheses (`pval in parentheses`, `se pval in parentheses`, `t-statistics in parentheses`, `ci in parentheses`).
  With `tstat` or `se` too Stata stops (r(198)). `beta` is not translated.
- **`tex`** alone: a whole document (`\documentclass[]{article}`, a `\setlength` line, `\begin{document}`, the tabular,
  `\end{document}`) in a .tex file (added when the name has no extension). `tex(frag)` is `tex(fragment)`. `tex(pretty)` is not
  translated.
- **`word`**: Rich Text in a .rtf (a name without extension gets .rtf; `.doc` is the same text), and the .txt as ever.
  A row per line of the .txt; the first row and the VARIABLES row ruled; the last row ruled below; the cell widths are
  144 twips per character of the longest cell plus one (the VARIABLES cell does not count for the first column); the
  notes follow; no line end after the last `}`.
- **Names**: `outreg2 using f.txt` writes f.txt and prints only `dir : seeout`; the name of the file is printed for
  .xls, .tex and word.
- Not translated: `label` with word, stats() with bdec/sdec/tdec, `beta`, the new options with `append`, with .xls or .tex.
