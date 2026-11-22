# unique (SSC) (2026-11-22)

unique (Michael Hills, Tony Brady) installed in Stata 19 with the user's OK on 2026-11-22; its program was not read.
Observed on made-up data; ported also from the behaviour StatCraft matches (probe 143). Corpus `unique_distinct.do`.

- `unique varlist [if] [in] [, by(varlist) generate(newvar) detail]`. Prints `Number of unique values of a b is  22` and
  `Number of records is  40` (two spaces before each number) and leaves r(N) the records, r(sum) and r(unique) the count.
  A missing value (or an empty string) is a value; strings are taken.
- `detail`: after those two lines, `summarize, detail` of the records per combination, titled `Records per a b`. With `by()` one
  such block for each group, after a line of 79 dashes and `-> b = 0`.
- `by()`: a variable (`_Unique`, or `generate()`; float, `%9.0g`) with each group's count on **one row of the group** (the
  rest missing), then `variable _Unique contains number of unique values of a by b` and the groups with their counts as a
  `list, noobs` table, in the order of the by variables (missing last). The row is the first of the group when the whole
  data are sorted by the by variables and then the varlist (not only the rows of `if`); Stata's sort does not keep tied rows
  in order, so among tied first rows Stata picks one (the first, the middle) that is not reproduced: RCraft takes the first in
  the data's order and says so in a message. The data's order is left as it was. A group the `if` leaves empty is not observed.
- `generate()` with no `by()` makes nothing (even if the name is in use). With `by()` a name in use: the two lines are
  printed, then rc 110 `variable id already defined`.
- Errors: no varlist 100; not found 111 (`by(nosuch)` adds `(error in option by())`); weights 101.
- Not translated: `detail` with `by()` of several variables, patterns and ranges, weights (the reader refuses them).
