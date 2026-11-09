# outreg and frmttable (SSC) (2026-11-08)

ZNMIXH builds its balance tables (Table 2, I1, I2) with John Luke Gallup's outreg package, already
in Stata's PLUS folder; its programs were not read. Observed in Stata 19 on made-up data;
`outreg.do` reads the .tex back line by line.

- After a model, `outreg, keep(Z) rtitle("Age") stats(b) noautosumm starlevels(10 5 1)
  starloc(1) store(row1)`: one row, the coefficient as %.3f with thousands commas (2,964.462;
  -712.250; 0.000 for a tiny one) and one star per level its p-value is below (in percent; t with
  the residual df after regress).
- `frmttable, statmat(M) store(s) sfmt(g,f,...)`: `f` cells with sdec() decimals (2 by default,
  -0.00 keeps its sign), `g` cells of whole numbers as they are (108); a missing cell is empty; no
  row titles, so a file of it alone has no `l` column. (`g` of 2.25 gives 2.3, not pinned down:
  refused.)
- `replay(name)` takes a stored table; one not stored yet is empty before `append()` (the loop's
  first pass), and an error otherwise. `append(m)` adds m's rows below; `merge(m)` m's columns to
  the right, by position when the table has no row titles (the titles then m's). `ctitles()`
  sets the title rows: `\` between rows, commas between cells, the row-title column first; a
  merged table keeps each side's titles over its columns. `note("")` empties the note; `store()`.
- `using f.tex, tex fragment plain nocenter replace multicol(r,c,n;...)`:
  `\begin{tabular}{l` and a `c` per column `}`; the title rows, the first after `\hline `; the
  body rows, the first after `\hline `; cells joined by ` & `, each row ending `\\`;
  `\multicolumn{n}{c}{title}` for cell c of title row r over n columns; then
  `\hline\end{tabular}\\` and `\hfil\\` (empty note). Lines end CR LF.
- Not translated: standard errors or t under the coefficients, summary statistics, outreg's own
  note text, Word and RTF files, tables without fragment.
