# xtsum (2026-11-19)

Observed on Stata 19 SE with made-up data (corpus `xtpanel.do`). Ported from statcraft@9d1e0f8 (`panel.md`, probes 173
and 576), decoded further here; printed tables compared with Stata's log. Translated in
`crates/statatr/src/data_r/xtsum.rs` (`stata_xtsum`). Batch H of the StatCraft port.

- `xtsum [varlist] [if] [in] [, i(var)]` needs `xtset` (a panel is enough). Without a varlist: every variable in data order.
- Per variable, over the rows where it is not missing (a missing panel value is a panel of its own):
  overall mean, sd (n - 1), min, max; between: the sd (n - 1), min and max of the panel means; within: x - panel mean +
  overall mean, its sd about the overall mean on N - 1, min and max. Counts: `N`, `n` and `T-bar` (`T` when every panel
  has as many rows), written with `%8.0g`; the numbers with `%9.0g`.
- Layout: names cut to 8 characters as Stata abbreviates them (`abcdef~p`), `overall`, `between`, `within`, a blank
  separator row between variables, a blank line before and after. With one panel the between sd is `.`; with no rows
  all is `.` and `T = .`. A string variable shows as no rows (no error).
- r() is the last variable's, in this order: max_w min_w sd_w max_b min_b sd_b max min sd mean Tbar n N.
- `i()` naming the panel variable is accepted; another variable is refused (Stata resets the panel and warns).
- Errors: no `xtset` r(459) `must specify panelvar; use xtset`; a name the data lack r(111); weights r(101); an unknown
  option r(198).
