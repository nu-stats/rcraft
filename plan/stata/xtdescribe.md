# xtdescribe (2026-11-19)

Observed on Stata 19 SE with made-up data (corpus `xtpanel.do`: 787 results to 1e-9, shared with `xtsum` and `xttab`).
Ported from statcraft@9d1e0f8 (`panel.md`, probes 173 and 576) and decoded further here by running Stata; the printed
tables of about 60 cases of our own were compared line by line with Stata's log. Translated in
`crates/statatr/src/data_r/xtsum.rs` (`stata_xtdescribe`). Batch H of the StatCraft port.

## What it prints

- `xtdescribe [if] [in] [, patterns(#) width(#)]` needs `xtset` with a panel and a time variable.
- Two lines `{panel:>8}:  first, second, ..., last   n = {n:>10}` and the same for the time with `T =`, padded to 61
  columns. The second value is `.` when there is only one. Times are shown in their `%t` format, panel values as
  `%.8g` (the variable's format is not used). `T` counts a missing time as one more value; rows with a missing panel
  are left out.
- `Delta(t) = 1 unit`: the unit word comes from the time variable's format (`day`, `week`, `month`, `quarter`,
  `halfyear`, `year`; `unit` for a plain number); `2 units`, `2 quarters`. `Span(t)` is `(max - min) / delta + 1` periods.
  The last line says whether panel and time identify the rows uniquely.
- `Distribution of T_i:` min, 5, 25, 50, 75, 95 and max (summarize's percentiles) written with `%.0f` (so 2.5 shows 2 and
  5.5 shows 6).
- The pattern table: a character per period (`.` none, a digit the rows, `X` for 10 or more); most frequent first, ties
  by the text of the full pattern (`.` before a digit before `X`); `patterns(#)` rows (9), then `(other patterns)`;
  the last row has `X` wherever any panel has a row. `patterns(0)` drops the table.
- **Wide spans.** A pattern is shown whole up to `width()` (100 by default): `floor((span - 1) / (width - 1))`
  periods per column when that is 2 or more. Then a column is the count of present periods in its group (`.`, a digit, `X`
  for 10 or more), the header says `Pattern*`, frequencies are of the patterns as shown, and a rule and ` *Each column
  represents N periods.` follow. The rules are `max(floor(span / N + 0.25) + 3, 10)` dashes after the `+` (fitted to
  150 cases), or `max(span + 2, 9)` when not grouped.
- r() is `summarize T_i, detail` (N, sum_w, mean, Var, sd, skewness, kurtosis, sum, min, max, p1 to p99).

## Errors (all r(459) unless said)

- No `xtset`: `must specify panelvar and timevar; use xtset`; a panel without a time: `must specify timevar; use xtset`;
  fewer than two distinct nonmissing times (also an empty `if`): `t must have multiple distinct nonmissing values`.
- Weights and a varlist r(101); an unknown option r(198); `patterns(-1)` r(198) `invalid syntax`; `patterns(x)` r(198).

## Refused (R stops or the line is untranslated)

- `i()` and `t()` naming other variables than xtset holds (Stata resets the panel as the old `iis` did); `width()`
  below 3 (Stata prints an error and carries on); a `%tc` or `%tC` time; data that would need more than 1e8 cells.
- The time line of `xtset id t, quarterly` shows numbers, not dates, in RCraft's `xtset` (not this command).
