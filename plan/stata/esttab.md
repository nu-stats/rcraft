# esttab (SSC estout, P2C.9)

Observed on a licensed Stata 19 SE with made-up data, 2026-11-03 (estout as installed from SSC on
2026-11-02). Only what esttab did is written here; its code and help were not read.

## The screen table (`esttab m1 m2`)

- A blank line, a rule of hyphens, the model numbers `(1)` `(2)`, the dependent variables, a rule, one block
  per coefficient (the estimate, then its t statistic in parentheses below it, then a blank line between
  blocks), a rule, `N`, a rule, then `t statistics in parentheses` and `* p<0.05, ** p<0.01, *** p<0.001`.
- Widths: the name column is 12 wide (20 with `label`); each model column is a space, the number
  right-aligned in `modelwidth()` (12 unless given), and the stars left-aligned in 3. With two models the
  rules are 44 hyphens (48 with `label modelwidth(10)`).
- `esttab` with no names shows the models stored with `eststo`.
- A base level shows `0` with `(.)` below it, under its name `1.g`; `nobaselevels` leaves it out.
- With `label`: variable labels; factor levels without value labels as `g=2`; `_cons` as `Constant`; `N` as
  `Observations`; `ar2` as `Adjusted R-squared`. The dependent variable row stays `y` (no label on y).
- `se` puts standard errors in the parentheses, and the note reads `Standard errors in parentheses`.
- `title("Parties")` is a line before the top rule.

## Numbers

- The estimates and standard errors use esttab's automatic format (a3). An exact integer prints with no
  decimals (`0`, `1`, `10`). Otherwise, below 1 in size, 3 significant digits (0.123, 0.0123, 0.0000123,
  0.0500, -0.500); from 1 up, max(1, 3 − floor(log10 |x|)) decimals (1.234, 12.34, 123.4, 1234.5, 1234567.8);
  rounding can carry over (0.9996 is `1.000`, 9.9996 `10.000`, 99.996 `100.00`). 1e-9 is `1.00e-09`.
- t statistics have 2 decimals; `N` is a whole number; `ar2` 0.401 and 0.538 (3 decimals here).
- Stars: `*` p<0.05, `**` p<0.01, `***` p<0.001.

## Files

All end their lines with CR LF.
- `.txt` (`using t.txt, se`): the screen table without its first blank line.
- `.csv`: every cell as `="..."`, comma-separated, a blank line before the header, between blocks and
  before the notes; the notes as `="t statistics in parentheses"`.
- `.tex, booktabs`: `{`, `\def\sym#1{\ifmmode^{#1}\else\(^{#1}\)\fi}`, `\begin{tabular}{l*{2}{c}}`,
  `\toprule`, header cells `&\multicolumn{1}{c}{(1)}`, `\midrule`, cells `&       0.601\sym{***}` with the
  stars as `\sym{...}` (padded to the star width when fewer), `\addlinespace` between blocks, `\_cons`,
  `\(N\)`, `\bottomrule`, the notes as `\multicolumn{3}{l}{\footnotesize ...}\\` (`\textit{t} statistics`,
  `\sym{*} \(p<0.05\)`, ...), `\end{tabular}`, `}`. With `label b(%9.2f) se(%9.2f)`: `0.60`, `(0.09)`, the
  base `g=1` as `0.00` over `(.)`, and the name column 20 wide.

## More, observed 2026-11-04

- The exponent form starts below 1e-6: 0.0000012345 is `0.00000123`, 1e-7 is `1.00e-07`. Stata's log10 is a
  little below at the powers of 10, so 0.00001 gets 8 decimals (`0.00001000`). Large values keep one
  decimal (`123456789.5`); whole numbers none (`10000000000`).
- With `label`, a label longer than the 20 of the name column is cut as Stata cuts names, `A rather long
  labe~e`; the dependent variable row shows its label (`Outcome`); a factor level its value label (`low`,
  `middle group`). In .tex the name column is not cut.
- `nostar`: no star column (models 13 wide); `p`: p-values to 3 decimals, `p-values in parentheses`; `r2` is
  `R-sq` (`R-squared` with `label`), `ar2` `adj. R-sq`; `aic`, `bic`; their order N, R-sq, adj. R-sq, AIC,
  BIC, then `scalars()` by name; `obslast` puts N last; `noobs` leaves it out. The statistics take `b()`'s
  format unless given their own (`r2(4)`), N a count.
- `star(* 0.10 ** 0.05 *** 0.01)`, its note `* p<0.10, ** p<0.05, *** p<0.01`; `mtitles("First" "Second")`
  in place of the dependent variables; `nonumbers`; `nonotes addnotes("Note one" "Note two")`;
  `varwidth(8) modelwidth(9)`; `keep(x w) order(w x)`; `drop(*.g _cons)`.
- `not`: no parentheses and no blank lines between the coefficients.
- `.tex` without booktabs: `\hline\hline`, `\hline`, `[1em]` between blocks; with `title()` it is
  `\begin{table}[htbp]\centering`, `\caption{...}` after the `\sym` line, and `\end{table}` at the end.
- `.rtf` carries the time it was written (`\creatim\yr2026...`), so it cannot be matched byte for byte.

## Checked

13 screen tables and 8 files (.tex booktabs and plain, .csv, .txt) from RCraft were the same as Stata 19's,
byte for byte; `esttab.do` reads four of them back.

## Not translated yet

`wide`, `cells()` of models other than estpost summarize's, `matrix()`, `mlabels()` with its suboptions, `.rtf`,
`not` with a file. `stats()`, `varlabels()`, `mgroups()`, `starlevels()`, `compress`, `nogaps`, `estpost summarize`
and its `cells()`: `esttab-options.md`.
