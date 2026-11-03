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

## Still to observe

- The thresholds of the exponent form; a long label (does the name column grow past 20?); value labels in
  factor names; `star()`, `nostar`, `p`, `r2`, `scalars()`, `mtitles()`, `nonumbers`, `append`, `.rtf`; `tex`
  without booktabs; `cells()` and `matrix()` (used by two packages); `mlabels(..., prefix() suffix())`.
