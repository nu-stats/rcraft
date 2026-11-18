# joinby (2026-11-18)

Observed on Stata 19 SE with made-up data (corpus `joinby.do`, 49 results, exact; the notes read against
Stata's log). StatCraft had only probe 138 and no note. Translated in
`crates/statatr/src/data_r/rowcol.rs` (`stata_joinby`).

- `joinby [varlist] using file [, unmatched(none|both|master|using) _merge(name)]`: every master row
  paired with every using row of the same key (many to many). The master's rows keep their order; an
  unmatched master row stays where it is (`unmatched(master|both)`); the using-only rows follow, in key
  order. The order of the rows paired within one key is not fixed in Stata (two runs gave 3,2 and 2,3):
  sort before looking.
- Columns: the master's, then `_merge` (byte, `%29.0g`, label `__MERGE`: 1 only in master data, 2 only in
  using data, 3 both in master and using data) when `unmatched()` is not `none`, then the using file's
  other variables. A variable in both files keeps the master's value; a using-only row gets the keys and
  nothing else of the master (the other common variables are missing there). No `_merge` without
  `unmatched()`, so an old `_merge` stays.
- No varlist: the variables in common, with the note `join on common variables: k uv`.
- An empty result says `(joinby on k tag is empty)`: no rows, the variables stay, r(0).
- Errors: no `using` r(100); a name the data lack r(111); no variable in common `no variables in common
  to master and using data` r(198); `unmatched(foo)` `foo is invalid value for unmatch()` r(198); another
  option r(198); `if` r(101); a file that is not there `file X.dta not found` r(601); a key that is a
  string in one file and a number in the other r(106) (Stata names the storage types; RCraft says
  string and numeric); `_merge` there when `unmatched()` asks for it `variable _merge already
  defined` r(110).
- `update` and `replace` also make `_merge` with other codes ("in both, master agrees with using data");
  they are refused. So is `by:`. Stata's hidden `9999 signature` entry of `__MERGE` is not made.
