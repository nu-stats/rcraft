# Forms from published replication packages (P2C.9)

Observed on a licensed Stata 19 SE with made-up data, 2026-11-02. These are the commands and options that
real replication packages use most where RCraft refused them. Only what Stata did is written here.

## Files

- `erase f.dta` deletes the file and prints nothing. When the file is not there it prints
  `file f.dta not found` and the return code is 601.
- `rm` behaves the same way as `erase`, with the same message and code.
- `erase f` with no extension looks for `f` itself, not `f.dta`: `file f not found`, r(601).
- `save f` without `replace` over a file that exists prints `file f.dta already exists`, r(602).
- `compress` on data read by `import delimited` prints `  (0 bytes saved)`.

## Settings that change nothing in the results

- `set niceness 6` and `set scheme s1mono` print nothing and change nothing that a later display shows.

## reshape long without j()

- `reshape long a, i(id)` names the new variable `_j`. Its values are the suffixes (1 2 for `a1 a2`). It
  prints `(j = 1 2)` and its usual table, and the rows are sorted by `id _j`.

## append with force

- When a variable is a string in one file and a number in the other, `append` without `force` stops:
  `variable s is float in master but str2 in using data`, r(106).
- With `force` the master's type wins. The appended rows hold missing in that variable (`.` for a
  number, an empty string for a string). Stata prints one note per variable:
  `(note: variable s was str2 in the using data, but will be float now)`.

## cluster() and cl()

- `regress y x, cluster(g)` gives the same table as `vce(cluster g)`, and so does the shortest
  abbreviation `cl(g)`.
- `robust` beside `cluster()` changes nothing.
- `logit` with `cluster(g)` gives the clustered standard errors as well.

## fvset

- `fvset base 3 h` makes later models leave level 3 of `i.h` out, exactly as `ib3.h` would. The
  coefficient names in `e(b)` are those of `ib3.h` (`3b.h`).
- A bare variable in an interaction (`h#f`) takes the base too: the cell left out is the one where both
  are at their base (3 and 1).
- With `i.h#c.x` every level has its slope, and the base is still marked `3b`.
- `fvset base default h` goes back to the lowest level. `fvset clear h` removes the setting.
- `fvset base 2 h g` sets both variables.
- `fvset report` lists the variables with a setting, each with its base.
- The setting stays with the data:
  - `save` then `use` brings it back;
  - `preserve`, `fvset clear` and then `restore` brings it back;
  - `import delimited ..., clear` drops it.

## estimates

- `estimates store m1` prints nothing.
- `estimates restore m1` prints `(results m1 are active now)`, and `_b[]` is m1's again.
- `estimates dir` lists name, command, dependent variable, number of parameters and title.
- `estimates clear` drops them all. A later `estimates restore m1` prints `estimation result m1 not found`,
  r(111). `estimates drop` of a name not stored prints the same message, r(111).
- `estimates drop *` drops them all. `estimates dir` with nothing stored prints nothing.
- Storing again under a name already stored replaces it.
- `estimates store m0` before any model prints `last estimation results not found, nothing to store`,
  r(301).
- `estimates store 1bad` prints `1bad invalid name`, r(7).
- Abbreviations: `est sto` and `est res` (and `rest`) work. `est st`, `estimates s` and `est cl` print
  `subcommand estimates st is unrecognized` (and so on), r(198). `est r m1` is taken, but it is not restore:
  it prints no note.
- After `estimates restore a`, `predict` uses model a, `_b[]` and `e()` are a's, and `e(cmd)` is
  `regress`.

## assert

- `assert exp` that holds prints nothing.
- When it is false somewhere, it prints `23 contradictions in 60 observations` (`1 contradiction` for one; counts with
  thousands commas, `2,143 contradictions in 2,500 observations`), then `assertion is false`, r(9). Over a single
  observation only `assertion is false` is printed.
- `if` and `in` limit the rows counted. A missing value counts as larger than any number (`assert x < 2` is false
  where x is missing; `assert x > 0` holds there).
- With no rows, it prints `(null assertion)` and r(0). With `null` and no rows: `(null assertion)`, r(8).
- `rc0` prints the same messages and r(0). `fast` prints only `assertion is false`, r(9).
- Under `quietly` the messages are hidden, and r(9) still stops the do-file.

## tabulate, generate()

- `tabulate x, generate(d)` prints the table and makes `d1`, `d2`, ... (byte), one per value in sorted order.
- Each is 1 where x has that value and 0 elsewhere. It is missing where x is missing, and on rows that
  `if` or `in` leave out.
- With `missing`, a missing x gets an indicator of its own (the last), and the others are 0 there.
- The variable labels are `x==` followed by:
  - the value's label when x has one (`f==one`);
  - the text for a string (`s==a`);
  - otherwise the number in 11 characters, padded on the left: `%11.4f`, or `%11.4e` when that is wider
    (`cat3==     1.0000`, `v==100000.0000`, `v== 1.2346e+08`, `v==-1.2346e+05`, `v==    -0.0000` for -1.5e-7);
  - and for a missing value `x==      .` (six spaces).
- `describe` shows these labels with the runs of spaces narrowed to one.
