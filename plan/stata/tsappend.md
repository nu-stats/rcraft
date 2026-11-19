# tsappend (2026-11-19)

Observed on Stata 19 SE with made-up data (corpus `tspanel.do`). Ported from statcraft@9d1e0f8 (`time-series.md`, probes
153 and 548); two things there were wrong or missing and were decoded here: it also **fills the gaps**, and it keeps r(add).
Translated in `crates/statatr/src/data_r/xtsum.rs` (`stata_tsappend`). Batch H of the StatCraft port.

- `tsappend, add(#)` adds # periods after each panel's own last row; `tsappend, last(date) tsfmt(fn)` adds periods
  up to `fn(date)` (`td`, `tw`, `tm`, `tq`, `th`; `last(25jun1965) tsfmt(td)`).
- Then the periods missing inside each panel (up to its new end) are filled too, as `tsfill` does, so the data have no gaps
  afterwards. The new rows have the panel and time set and every other variable missing (`""` for text); the data end in
  panel and time order; the settings of `tsset` stay.
- r(add): `#` with `add()`; with `last()` the number of periods added after the panels' ends (not the fills).
- `last()` that is not past the data: nothing is appended (r(add) = 0), and Stata says `{t} is already in the dataset`
  (one series, last at or before the end) or `Some panels have observations later than {t}` (a panel, some panel
  ending at or after it); `{t}` is the number. The gaps are filled all the same.
- Errors: neither option r(198) `you must specify either add() or last()`; `add()` that is not a positive whole number
  r(198); `add()` with `last()` or `tsfmt()` r(198) `specify add() or tsfmt(), not both`; `last()` without `tsfmt()` r(198);
  a missing time r(198) `the time variable may not be missing`; an unknown option r(198); a varlist, `if`, `in` and weights
  r(101); no `tsset` r(111).
- Refused: data not in panel and time order (Stata keeps the order it finds), a missing panel value, `tsfmt()` other
  than `td`, `tw`, `tm`, `tq`, `th` (Stata stops on `ty`: `unknown function ty()`).
