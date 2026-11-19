# xttab (2026-11-19)

Observed on Stata 19 SE with made-up data (corpus `xtpanel.do`). Ported from statcraft@9d1e0f8 (`panel.md`, probe 173),
decoded further here; printed tables compared with Stata's log. Translated in
`crates/statatr/src/data_r/xtsum.rs` (`stata_xttab`). Batch H of the StatCraft port.

- `xttab varname [if] [in] [, i(var)]`, one numeric variable, over the rows where it and the panel variable are present.
- Overall: rows and their percent. Between: the panels with the value at least once, and the percent of all panels (so
  the column can pass 100). Within: the mean, over those panels, of the share of their rows with the value (percent).
  Total row: rows, 100, the between total (and its percent of panels), and `n / between total` as the within percent;
  `(n = #)` under it.
- A value shows as its value label cut to 8 characters (blank when the variable has labels but not that value) or as
  `%9.0g` writes it, whatever the variable's format (a `%td` time shows its number). The header names the variable cut to
  9 characters. Values are grouped as doubles; r(results) keeps them as floats.
- r(results): value, Overall:Freq, Overall:Percent, Between:Freq, Between:Percent, Within:Percent, one row per value
  (`r1`, `r2`, ...) and a last row whose value is `.`; r(n_panels). `matrix list` shows the column equations.
- Errors: no varlist r(100); two variables r(103); a string variable r(109) `s must be numeric`; no rows r(2000); no `xtset`
  r(459); weights r(101); any option but `i()` (`missing` among them) r(198).
