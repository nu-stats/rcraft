# distinct (SSC) (2026-11-22)

distinct (Gary Longton, Nicholas J. Cox) installed in Stata 19 with the user's OK on 2026-11-22; its program was not read.
Observed on made-up data; ported also from the behaviour StatCraft matches (probe 144). Corpus `unique_distinct.do`.

- `distinct [varlist] [if] [in] [, minimum(#) maximum(#) joint missing abbrev(#)]`; no varlist is every variable.
- A blank line, a two-line heading over `Observations` (`total` and `distinct`), a rule, one row per variable shown (the name
  right-justified in at least 6 columns, `%11.0f` for each of the two counts, `|` after the name), a blank line. The name
  column is as wide as the longest name shown plus one, 6 at least.
- A missing value (an empty string too) is left out of both counts unless `missing`. `joint`: one line with no names, the
  rows with no missing value (all rows with `missing`) and the distinct combinations.
- `minimum(#)`, `maximum(#)`: the variables whose distinct count is within them, a table of no rows when none is. A minimum
  above the maximum is swapped with `min(7) max(2) interpreted as min(2) max(7)`. A negative minimum lets all in; a
  fraction or text is rc 198 `option min() incorrectly specified`.
- `abbrev(#)`: a name longer than # (5 when # is below 5) shows as its first # - 2 letters, `~`, its last letter; text is
  rc 198 `option abbrev() incorrectly specified`; 0 changes nothing.
- r(ndistinct) and r(N) are those of the last variable of the varlist (shown or not), or of the combination with `joint`.
- Errors: not found 111; `by()` 198 `option by() not allowed`; weights 101.
- With `in` and several variables the later variables' totals are the rows of the range, missing or not (41 of 41 rows with
  one missing): not reproduced, refused; one variable with `in` is as with `if`.
