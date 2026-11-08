# contract and recast (2026-11-07)

YFBPT6's representative.do counts respondents by country and age group or gender with `contract
country_v13 agecat, freq(n)`, and changes quota's storage with `recast double quota` and later
`recast float quota, force`. Observed in Stata 19 on made-up data; `contract.do` keeps the checks.

## contract

- One row per combination of the varlist's values, sorted by them: a missing value is a category
  of its own and sorts last; an empty string sorts first. The data are sorted by the varlist.
- The new variables follow the varlist in the order freq, cfreq, percent, cpercent. freq (`_freq`
  by default) has no label and is the smallest integer type that holds it (byte, int, long);
  cfreq is labelled "Cumulative frequency", percent "Percent", cpercent "Cumulative percent".
- percent is 100 * f / N and cpercent 100 * (cumulative f) / N, bit for bit (not f / N * 100, not a
  sum of percents). Doubles shown %8.2f; `float` makes them floats, `format()` sets the format and
  needs percent or cpercent ("percent or cpercent must be specified", r(198)).
- `nomiss` drops rows missing on any varlist variable ("" counts); `zero` adds every combination of
  the values seen (missing included unless nomiss) with frequency 0.
- `if` and `in` choose the rows; fweights add up, a row whose weight is missing is left out, a
  fraction is "may not use noninteger frequency weights" r(401); aweights and pweights r(101).
- A new name that is already a variable, even one contract would drop, is "variable a already
  defined" r(110); no rows is "no observations" r(2000). The data are then left as they were.
- The varlist's variable labels, value labels and formats stay; other variables go.
- Not translated: weights of 0 (not observed).

## recast

- byte, int, long: a value is truncated toward zero; outside the type's range (byte -127..100, int
  -32767..32740, long -2147483647..2147483620) it becomes missing. float: rounded to float.
- Without `force`, when any value would change nothing changes and Stata shows "x:  3 values would
  be changed; not changed" (the type stays); with force "x:  3 values changed" ("1 value"). Nothing
  is shown when no value changes. double never changes a value.
- A string variable to a number type, or the reverse, is "s:  float invalid" r(109). String types
  are not translated yet.
