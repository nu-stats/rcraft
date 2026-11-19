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
- `update` and `replace` (2026-11-18, second round; corpus section "update and update replace", 146 results):
  - `update` fills a master value that is missing (`.` or any `.a`-`.z`) or `""` in a paired row with the
    using file's value *whatever it is* (master `.b` against using `.` gives `.`; against using `.c` gives
    `.c`). A master value that is there stays. `update replace` also overwrites a master value with the using
    value when that is not missing or `""` (a missing using value never replaces anything).
  - Only the variables in both files besides the keys are touched. One that is a string in one file and a
    number in the other is left alone (no error, the master's value stays, a using-only row has it missing).
  - A using-only row (`unmatched(using|both)`) takes the using file's values of all the variables in both
    files; without `update` it has the keys only. A master-only row stays as it is.
  - `_merge` is made only with `unmatched()`, as before. With `update` its label `__MERGE` has five
    entries (1 only in master data, 2 only in using data, 3 in both, master agrees with using data, 4 in
    both, missing in master data updated, 5 in both, master disagrees with using data) and the format is
    `%41.0g` (the longest label). **Every paired row is code 3**, updated or not; 4 and 5 never occur.
  - `replace` without `update`: r(198), after r(111) and r(601) (a missing file or name comes first) and
    after r(106). `upd` and `rep` are not accepted (198).
  - A variable in both files takes the wider of the two storage types in Stata; RCraft forgets the storage
    type of one the do-file knew differently in the two files (a file the do-file did not save is not
    compared).
- `by:` is refused. Stata's hidden `9999 signature` entry of `__MERGE` is not made.
