# merge (P2C.9)

Observed on a licensed Stata 19 SE with made-up data, 2026-11-02. Only what Stata did is written here.

## Pairing

- `merge m:m k using f`: within a key the k-th master row goes with the k-th using row. When one side
  has fewer rows for the key, its last row stands in for the rest: a key with 3 master rows and 1 using
  row gives 3 rows, each with that using row; a key with 2 master rows and 3 using rows gives 3 rows, the
  third with the master's second row. It is not every combination.
- The same rule gives 1:1, m:1 and 1:m: in `1:m`, a master row with two using rows becomes two rows.
- `merge 1:1 _n using f` pairs by row number: 6 master rows and 4 using rows give 4 matched and 2 master
  only.

## Order

- The master's rows come first, sorted by the key. A row past the master's own (the third row of a key
  above, the second using row of a 1:m key) and the rows only the using file has come after them, sorted
  by the key.
- Rows with the same key do not keep their order: after `m:1` the master rows of key 1 came out as `11`
  then `10` though they were `10` then `11`; after `1:m` the using rows `10 11` came out `11` first.
- A variable in both files keeps the master's value; a row only the using file has takes the using value.
  The using file's other variables come after the master's, and `_merge` last.

## _merge and the report

- `_merge` is a byte with the value label `_merge`: 1 `Master only (1)`, 2 `Using only (2)`,
  3 `Matched (3)`, 4 `Missing updated (4)`, 5 `Nonmissing conflict (5)`; its variable label is
  `Matching result from merge`. `generate(how)` names it how; the label is still `_merge`.
- The report, after a blank line:

      Result                      Number of obs
      -----------------------------------------
      Not matched                             4
          from master                         1  (_merge==1)
          from using                          3  (_merge==2)

      Matched                                 6  (_merge==3)
      -----------------------------------------

  The numbers end in column 45 and have thousands commas (`1,197`). When nothing is unmatched the two
  `from` lines are left out. After `nogenerate` the `(_merge==1)` are two blanks. It counts the rows
  `keep()` kept: `keep(using)` reports `Matched 0`. `noreport` and `quietly` hide it.

## update and replace

- `update`: a variable in both files that is missing in the master takes the using value, `_merge` 4. Two
  nonmissing values that differ are a conflict, `_merge` 5, and the master's value stays. Equal values,
  or a missing using value (`.` or `.a`), leave `_merge` 3. With update the report's Matched line has no
  `(_merge==3)`; three lines follow: `not updated`, `missing updated`, `nonmissing conflict`.
- `update replace`: in a conflict the using value replaces the master's; `_merge` is still 5.
- With `update`, `keep(match)` keeps `_merge` 3 only; `match_update` is 4 and `match_conflict` 5.

## keep() and assert()

- `keep(mas mat)`, `keep(us)`, `keep(master match)`, `keep(using)` all work.
- `assert(match)` when some rows did not match: `after merge, not all observations matched` and
  `(merged result left in memory)`, r(9), and no report. It is checked before `keep()`: with
  `keep(match)` too, all 6 rows stay in memory, with `_merge`.

## Keys

- String keys: an empty string matches an empty string; it sorts first.
- `merge 1:1 k` where k repeats: `variable k does not uniquely identify observations in the master
  data`, r(459).
- A key the using file does not have: `variable k not found`, r(111).
