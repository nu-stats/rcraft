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
- `+x` is not Stata's (r(198), "+x invalid name"); RCraft accepts it, which is a gap to close.

## recode

- `(.d = 9) (.w = .)` changes only those; `(. = 8)` only `.`; `(missing = 7)` all six missing;
  `(nonmissing = 4)` only the numbers; `(1 = .a)` stores `.a`; `(.a/.w = 5)` takes `.a`, `.d`, `.w`
  (a range of missing values by the order). A range from a number to a missing value is not
  observed, so it is still refused.

## Seen, not translated by these rules yet

- `egen max()`, `mean()` by group: an all-missing group is `.`; `total()` 0. `egen rowmax()` and
  `rowfirst()` of a row all missing give the last variable's value (`.d`, `.w`, or `.`).
- `collapse`: `(mean)` `.`, `(sum)` 0, `(count)` 0; `(max)` and `(min)` of an all-missing group give
  the largest and smallest missing value (`.z`, `.d`); `(first)`, `(last)` and `(firstnm)` the value
  itself.
- `tabulate x, missing` has a row for each of `.`, `.a`, ...; `sort` puts `. .a .d .w .z` after the
  numbers in that order, and `gsort -x` the numbers (descending) then `.z .w .d .a .`.
- RCraft's R for these keeps one NA as before; a later comparison of such a result with a missing
  value written out may differ from Stata's.

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
