# mvdecode and mvencode (2026-11-18)

Observed on Stata 19 SE with made-up data (corpus `mvcode.do`: values, missing kinds and return codes,
exact; the printed notes read by hand against Stata's log). StatCraft's note (`data-commands.md`) only had
the one-value form. Translated in `crates/statatr/src/data_r/rowcol.rs` (`stata_mvcode`).

## mvdecode varlist [if] [in], mv(numlist | numlist = mvc [\ numlist = mvc ...])

- A value in the list becomes `.` (no `= mvc`) or `.a` to `.z`. Every clause needs `=` once there are
  several (`= expected`, r(198)); `1/2` is 1, 2. The value is compared in the variable's own type, so a
  float 1.1 is matched by `mv(1.1)`. The first clause to match a value wins.
- Prints `           b: 2 missing values generated` (name right-aligned in 12, longer names cut to 10
  characters, `~`, last character) for each variable it changed, nothing for the others; a string
  variable (named or in `_all`) prints `           s: string variable ignored`. Counts have no commas.
- Labels, formats and types stay. `if` and `in` work.
- Errors: no `mv()` `option mv() required` r(198); another option `option foo not allowed` r(198); `.` or
  `.a` or text in the list `X is not a valid numlist` r(121); a code that is not `.`/`.a`-`.z`
  `X is not a missing value code` r(198); a name the data lack `variable X not found` r(111).

## mvencode varlist [if] [in], mv(# | mvc = # [\ mvc = # ...]) [override]

- `mv(#)` turns every kind of missing value into `#`; `mv(.a = #)` only that kind (`.` is plain `.`).
  Prints `           b: 2 missing values recoded` for each variable it changed. A string variable:
  `string variable ignored`.
- If a selected variable already holds `#` (in the selected rows) it says `           b: already 5 in
     1 observation` (count in 4 columns) for each such variable, then `no action taken`, r(9), and changes
  nothing at all, in no variable, unless `override` is given.
- A value that does not fit the type recasts it: byte to int or long, int to long, long or int to double,
  and a fractional value into an integer type gives double; it prints `              (recast to int)`
  under the variable's line. A float or double never recasts (the value is rounded to float).
- Errors: `mv(1/2)` and `mv(2, 3)` `X is not a missing value code` r(198); `mv(.=)` `'' found where
  number expected` r(7); `mv(.=2 .a=3)` `.a found, where \ expected` r(198); the rest as mvdecode.

## What RCraft does

- Refused: `by:`, a numlist with a step (`1(2)9`), more than one clause for the same missing value in
  mvencode, and an mvencode value that may not fit the type when the variable's storage type is not known
  (R does not keep Stata's types for data read from a file; it is known for variables the do-file made).
  Afterwards the storage of those variables is unknown.
- The order of tests is: weights (r(101)), unknown option, varlist (r(100)), `mv()`, then the variables.
