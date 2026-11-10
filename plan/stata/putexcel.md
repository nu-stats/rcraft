# putexcel (2026-11-09)

TZEN38 fills `policy_predictability.xlsx` with 216 cells: `putexcel set "…xlsx", sheet("…") modify`
then `putexcel C7 = "`coeff'"` (a coefficient or its standard error, rounded into a macro). Observed
in Stata 19 on made-up workbooks; `putexcel.do` writes one and reads it back with `import excel`.
Written in R with openxlsx2 (MIT; installed with the user's OK, and in CI).

- `putexcel set file [, sheet(name) modify | replace]`: `modify` keeps the other sheets and cells and
  makes the file if it is missing; `replace` starts the workbook again at the first write, with the
  note "file will be replaced when the first putexcel command is issued."; neither, on a file that
  exists, is r(602) ("file … already exists; you must specify either option modify or option
  replace"), and the set before it stays. Without `sheet()` the sheet is `Sheet1`, made if missing
  and put after the others. A new sheet goes after the others.
- `putexcel cell = value …`: text is a text cell (Stata uses the shared-string table, and so does
  RCraft), a number a number written as `%.17g` (0.33333333333333331, 9.9999999999999995e-21, -0;
  R's own text has 15 digits, so the helper writes Stata's), `.` an empty cell (a value there is
  cleared), `( expression )` its value. Several to a line; lower-case cells; a cell written again
  is replaced. `file … saved` after each line.
- Errors: no `set` before is r(198) "no Excel file specified"; a column past XFD r(198)
  "ZZZZ1: Excel column out of range".
- One difference left: after `putexcel A6 = .` Stata leaves an empty `<c r="A6"/>`, openxlsx2 no
  cell. Excel and readxl show the same; only Stata's `import excel` counts a trailing empty cell as
  a row (6, where readxl reads 1 even on Stata's own file: an `import excel` difference, noted for
  later). The corpus keeps a filled cell after the emptied one.
- Not yet (refused): ranges, `matrix()`, `etable`, `formula()`, pictures, formatting options (bold,
  nformat(), …), `sheet(, replace)`, .xls, `putexcel save`/`close`/`describe`.

TZEN38's workbook, written by R from scratch: the same four sheets in the same order and all 207
cells the same type and text as Stata's.
