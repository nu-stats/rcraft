# snapshot (2026-11-18)

Observed on Stata 19 SE with made-up data; corpus `snapshot.do` (return codes, r(snapshot) and the data
after each restore). The printed lines (wrapping included) were set against Stata's log by hand. Translated
in `crates/statatr/src/data_r/snapshot.rs`. StatCraft's note (data-display.md) is right; what it did not say
is below.

- `snapshot save [, label(text)]` prints `snapshot N (text) created at 18 Nov 2026 09:49` (no parentheses
  without text, or with `label("")`) and leaves r(snapshot). **N is the highest number there plus one**
  (1 when none): after erasing the highest the number comes back, and after `erase _all` it is 1 again.
- `snapshot list [_all|numlist]`: one line each; a number that is not there `snapshot N not found` r(111);
  none at all prints nothing.
- `snapshot label # ["text"]`: no text clears the label; `restore` and `label` with no snapshots at all:
  `no snapshots exist` r(198); a number below 1 or above the highest: `snapshot number must be between 1
  and N` r(198) (N the highest number there); one in range that was erased: `snapshot N not found` r(111).
  No number: `'' found where integer expected` r(7); a word: `'a' found where integer expected` r(7); two
  numbers `invalid syntax` r(198).
- `snapshot erase _all|numlist`: a number that is not there gives r(111) and **nothing is erased** (the
  numbers are checked first, `1/3` with 2 missing keeps 1 and 3); none or a word: `invalid numlist has too
  few elements` r(122), `invalid numlist` r(121). `erase _all` with none is fine.
- `snapshot` alone or a word: `invalid syntax` r(198); `save, bogus`: `option bogus not allowed`; `save x`:
  `varlist not allowed` r(101).
- Lines wrap at the line size, greedy at spaces to at most the line size (80 fits at linesize 80), continued
  with two blanks; a word too long for a line goes on lines of its own, the first `  ` plus text, the
  rest `  > ` plus text, and the words after it continue on the last of them.
- A restore brings back the variables, values, labels and strings; snapshots outlive `clear`. The time of
  day is the computer's; a day under 10 is printed without a leading zero here, which was not observed.

Not followed: a snapshot of empty data restores it, but RCraft has no empty data, so the next line that
needs rows is refused.
