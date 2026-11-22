# frlink, frget and frval() (P2C.12ax)

Written 2026-11-19. Code: `crates/statatr/src/data_r/framelink.rs` (the R helpers `stata_frlink`, `stata_frget_col`,
`stata_frval`; `frlink`, `frget`, `frval_value`, `links_after`), the `frval` arm in `expr_r.rs`, and `describe` with no data
in `data_r.rs`. Checked by `crates/engine/tests/stata_corpus/frlink.do` (116 results, tolerance 1e-9). Observed on
Stata 19 SE with made-up data.

## Rules

- **`frlink 1:1 | m:1 varlist, frame(name [varlist]) [generate(newvar)]`:** a variable in the frame in use (named after the
  linked frame unless `generate()`; byte, int or long by the size of the linked frame; `%10.0g`) holding, for each row,
  the row number in the linked frame of the row with the same key values, missing when none matches. Missing keys match
  missing keys of the same kind (`.` with `.`). Strings and numbers; the two sides' variables are matched by position
  (`frame(name k1 k2)` names the other frame's).
  - **The linked frame is sorted by its keys** (its `Sorted by:` shows them): the row numbers are positions after the sort.
    It stays unsaved or saved as it was (a saved frame is not starred by it); the frame in use is starred (a variable was added).
  - Note printed: `(all observations in frame default matched)`, `(no observations in default matched!)` (no `frame`), or
    `(N observations in frame default unmatched)` (`1 observation`). A frame in use with no rows is all matched.
  - Errors: a frame that is not there r(111); a variable not found r(111); the link name in use r(110); `1:m` r(197); keys
    that do not identify the rows (the frame in use under `1:1`, or the linked frame under either) r(459). r() is cleared.
  - `frlink` stops on duplicate keys only after sorting the linked frame by a sort that puts tied rows in no set order
    (1,486 of about 2,400 tied pairs came out of order), so a failure under `capture` leaves it in an order not reproducible.
- **`frget varlist | newvar = var ..., from(linkvar) [prefix() | suffix()]`:** copies the variables of the linked frame to
  the frame in use through the link: the value for each row, missing (empty for text) where the link is missing, with the
  variable label, display format and storage type; value labels come with the variable (the set is renamed `name1` when
  the frame in use holds one of that name). Printed: for each variable with missing values `(N missing values generated)`
  (counting a missing in the linked frame as well), then `(N variables copied from linked frame)`. A variable named twice is
  copied once. A match variable of the link is not copied under its own name (`(variable not copied from linked frame: k)`,
  also with `prefix()`; `new = k` copies it). `_all` copies all but the match variables. A name in use r(110); an `if` or
  `in` r(101) (`if not allowed`, `in range not allowed`); no variables r(100); a variable that is not a link r(198).
  - **Stata checks the link at each use:** the linked frame having more rows than at the link r(459) (`too many observations`),
    fewer r(459) (`too few`), a key variable's values changed r(459) (`frlink rebuild`). A change of a variable that is not a key,
    of the frame in use's own keys or of the link variable's values does not matter (the link variable is only row numbers).
- **`frval(link, var [, default])`:** the same lookup inside an expression: the linked frame's value, the default (or missing, or
  empty for text) where the link is missing. The default's type must be the variable's (r(109)); two arguments needed
  (r(198)); a name that is not a variable r(111); one that is not a link r(459). It checks only that the linked frame is
  still sorted (`data in frame t not sorted`, r(5), when a key was changed).
- **`describe` with no data** (cleared or reset) describes an empty data set: r(N) = 0, r(k) = 0.

## RCraft

- The link is followed through the do-file (`Session::links`, `links_after`): a link is kept while the linked frame is run
  only read-only commands (`summarize`, `list`, `describe`, `display`, `count`, the models ...); any other command in that
  frame, or a `frame copy`, `rename`, `drop`, `frames reset`, `clear all`, drops it, and so does another `frlink` to the frame
  on other keys (the sort changes). The frame holding the link may change its rows (`sort`, `drop if`) but not be rebuilt
  (`use`, `merge`, `append`, `reshape`, `collapse`, `clear`, `rename` ...) or lose the variable. A `frget` or `frval()` on a
  link not kept is refused (not translated), where Stata stops with r(459) or r(5) only if the values changed: refuse-rather-than-differ.
- Refused: `frlink` and `frget` under `capture` or another prefix (see the tied-row sort above); `1:m`, `frlink rebuild`,
  `frlink describe`; a frame in use as its own linked frame; extended missing values in the keys; keys of different kinds;
  `frget` of `_all` or a pattern, of a variable with value labels (or from a frame that holds label sets), with both
  `prefix()` and `suffix()`, with `newvar = var` and a prefix; `frval()` with other than names for its first two arguments.
