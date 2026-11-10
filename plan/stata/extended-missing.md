# Extended missing values .a to .z (2026-11-09)

ZNMIXH's `01_analysis.do` line 51, `replace church_survey_b=. if church_survey_b==.d |
church_survey_b==.w`, was refused (a comparison with `.d`), and about 300 lines after it with it.
Its `.dta` holds 273 `.d` and 458 `.w` in that variable. Observed in Stata 19 on made-up data
(`input double x` with 1 . .a .d .w .z, and a second variable); `extended_missing.do` records it.

## The order

- Numbers < `.` < `.a` < `.b` < ... < `.z`, in every comparison. Over 1 . .a .d .w .z the counts of
  `x op m` for ==, !=, <, <=, >, >= are: `.` 1 5 1 2 4 5; `.d` 1 5 3 4 2 3; `1` 1 5 0 1 5 6.
  `x >= .a` counts 4. Two written out compare by their place (`.d == .d` and `. < .a` are true).
- Two variables: `.d == .d` is true, `.d == .w` false, `. < .d`, `.a < .w` true.
- `missing(x)` is true for all of them; `inlist(x, .)` only for `.`, `inlist(x, .d, 1)` for `.d`
  and 1. `inrange()` is false for any missing first argument, `.a` to `.z` included.
- `summarize` counts only the numbers.

## What keeps the tag

- A copy keeps it: `generate f = x` (float), `generate double`, `generate byte`; a row of a
  variable (`x[_n-1]`, `x[1]`, `L.x`, `F.x`); `generate y = x if ...` (the rows left out are `.`);
  `cond()` gives its value itself (`cond(x > 5, y, x)` gives `.w` where y is `.w`, also the
  fourth, missing-condition value).
- These functions keep it: `float()`, `int()`, `round(x)` and `round(x, 0.5)`, `floor()`, `ceil()`.
- Arithmetic gives `.`: `-x`, `x + 0`, `x * 1`, `x - 0`, `x / 1`, `x ^ 1`, `x + .d`; so do `abs()`,
  `sqrt()`, `exp()`, `ln()`, `mod()`, `sign()`, `dofm()`, `max(x, .)`, `max(x, x)`; `min(x, 5)` is 5.
- `generate lit = .d` and `replace lit = .w in 2` store the tags.
- A `+` sign goes only before a number written out (`+5`, `2^+2`, `x*+2`): `+x`, `1 - +x`,
  `2^+x`, `(+x)` are r(198) "+x invalid name", `+(x)` and `+abs(x)` r(133) "unknown function
  +()", `+-x` r(198) "+ invalid name", `+.` and `+.d` r(198) invalid syntax. The parser refuses
  them as Stata does.

## recode

- `(.d = 9) (.w = .)` changes only those; `(. = 8)` only `.`; `(missing = 7)` all six missing;
  `(nonmissing = 4)` only the numbers; `(1 = .a)` stores `.a`; `(.a/.w = 5)` takes `.a`, `.d`, `.w`
  (a range of missing values by the order).
- A range goes by the order too, `min` and `max` being the numbers': `(1/.b = 9)` takes 1, 2, `.`,
  `.b`; `(2/. = 8)` 2 and `.`; `(min/.d = 5)` every number and `.` to `.d`; `(1/.z)` everything;
  `(max/.z)` the largest number and every missing value. From a missing value to a number or
  `max` (`(.b/max)`, `(.d/1)`) changes nothing.

## Commands (observed 2026-11-10, `extended_missing_commands.do`)

- `egen rowmax()` and `rowfirst()` of a row with no number give the last variable's value
  (`.z`, `.a`, `.b`, `.`), `rowlast()` the first variable's; `rowmin()`, `rowmean()`,
  `rowmedian()`, `rowpctile()` and `rowtotal(), missing` give `.`; `rowtotal()` 0.
- By group, `egen max()`, `min()`, `mean()`, `median()`, `total(), missing` of an all-missing group
  are `.`; `total()` 0. `egen group(x), missing` makes each of `.`, `.b`, `.d`, `.z` a group of its
  own, numbered in that order after the numbers; `tag(x), missing` tags the first row of each.
- `collapse`: `(max)` and `(min)` of a group with no number give its largest and smallest missing
  value (`.z`, `.d`; `(min)` of `.w .` is `.`); `(first)`, `(last)` the value itself; `(firstnm)` and
  `(lastnm)` of a group with no number its first and last value. `(mean)`, `(median)`, `(p25)`,
  `(sd)` are `.`, `(sum)` and `(count)` 0.
- `sort` puts `. .a .d .w .z` after the numbers in that order; `gsort -x` the numbers descending,
  then `.z .w .d .a .`; with `mfirst` those come first in that order.
- `tabulate x, missing` (`tab1`, `tab2`) has a row (and a column) for each of `.`, `.a`, ...,
  after the numbers; `sort` orders by count, ties in that order. A labelled `.b` shows its label
  (`dunno`), `nolabel` shows `.b`. `generate()` makes an indicator for each, labelled `x==      .`
  and then `x==         .b` (11 wide).
- `fre`: a Missing row for each (`.`, `.b`, ...), a Total of them when there is more than one, the
  Total of all only when some value is valid; with a label set the values are padded to one width
  (`1  one`, `10 ten`, `.b dunno`). r(r_missing) counts the missing values, r(r) both.

## Not translated by these rules yet

- `by` groups (`by x:`, `bysort x:`, `egen ..., by()`, `collapse, by()`) and `merge` keys still
  take `.a` to `.z` as one value; `duplicates`, `levelsof`, `contract` and `egen rank()` too.

## In R

- haven reads `.a` to `.z` as tagged NAs (`haven::tagged_na`, `haven::na_tag`); R's arithmetic,
  `ifelse()`, `dplyr::if_else()`, `dplyr::lag()`, `floor()`, `trunc()` and copies keep the tag
  (`round()` drops it), and `%in%` and `match()` take every NA for every other.
- Once the data may hold them (a `.dta` read by `use`, `merge` or `append`, or a value stored by
  `input`, an expression or `recode`), the translation goes by their place: `stata_mv(x)` is 0
  for a number, 1 for `.`, 2 to 27 for `.a` to `.z`, so `x == .d` is `stata_mv(x) == 5`; `x < .`
  and `x >= .` stay `!is.na(x)` and `is.na(x)`; two values that may both be missing go through
  `stata_compare()`; a computed value is made `.` by `stata_untag()`; `stata_float()` keeps a tag;
  `.d` as a value is `haven::tagged_na("d")`. Without them the R is as short as before.
- Commands with them: `sort` and `egen group()`/`tag()`, `missing` sort by `stata_sort_keys()`
  (each number column, then its places); `gsort` adds `stata_mv(x)` after `x`; `tabulate`,
  `tab1`, `tab2` and `fre` take `tags = TRUE`; `collapse` uses `stata_mv_max()`, `stata_mv_min()`,
  `stata_mv_firstnm()`, `stata_mv_lastnm()`; `rowmax` puts the last variable's value in a row
  with no number, `rowmin` is untagged.
