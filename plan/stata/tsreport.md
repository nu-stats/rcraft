# tsreport (2026-11-19)

Observed on Stata 19 SE with made-up data (corpus `tspanel.do`, shared with `tsappend`: 365 results to 1e-9). Ported from
statcraft@9d1e0f8 (`time-series.md`, probe 153), which had the basic report; the varlist forms, `casewise`, the sample
rule, widths and r() were decoded here. Printed reports compared with Stata's log. Translated in
`crates/statatr/src/data_r/xtsum.rs` (`stata_tsreport`). Batch H of the StatCraft port.

- `tsreport [varlist] [if] [in] [, detail casewise panel]` needs `tsset`/`xtset` with a time.
- The report: a blank line, `Panel variable: id` and `Time variable:  t` (or `Time variable: t` alone), a rule of 18 plus
  the widest value, `Starting period`, `Ending period`, `Number of obs`, `Number of gaps` (right-aligned together; periods in
  the time's format), `(includes panel changes)` after the gaps whenever there is a panel and no `panel` option.
- A gap: two rows of a panel (among the rows with the variable) that are more than delta apart; the obs numbers are the
  rows' positions in the data (`7-8`, joined by an en dash). With a panel each panel's last row also counts, as a
  gap starting at the next period, and the **last row of the sample counts only when its time is before the sample's
  latest time** (so the data's last row does not, `in 1/12` does count row 12). `panel` leaves these out.
- `detail` adds `Gap report`: obs, the panel, the first and last missing period and how many (`.` for a panel's end);
  nothing when there are no gaps. Column widths: panel `max(11, width of its format)`, Start and End `max(12, w + 2)`
  (no panel: Start `max(11, w + 1)`), N. obs 13, with w the width of the time's format (`%12.0g` for a long widens it).
- A varlist: one report per variable with `detail`; without, `Gap summary report` (Variable, Start, End, Number of obs
  and gaps), names cut to 12; `casewise` (two or more variables) makes one report over the rows complete in all, headed
  `Variables: x y`; a single variable's report is headed `Variable: x`.
- r(): N_gaps N_obs start end (numbered 1, 2, ... with a varlist, none with `casewise`), text panelvar timevar
  tsfmt (and var1, var2 ...), and with `detail` and gaps the matrix `table` (`table1`, ...) of ObsBefore, ObsAfter,
  [panel], StartTime, EndTime, Obs, its rows named `5` or `7--8`. r(start) and r(end) are the numbers, not dates.
- Errors: no `tsset` r(111) `time variable not set, use tsset varname ...`; a name the data lack r(111); a string variable
  r(109); `casewise` with fewer than two variables r(198); an unknown option r(198); weights r(101).
- Refused: data not sorted by panel and time (Stata sorts), a `%tc` or `%tC` time.
