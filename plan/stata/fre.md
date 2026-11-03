# fre (SSC), as it behaves (P2C.9)

Installed with the user's go-ahead (`ssc install fre`, 2026-11-03) and run on Stata 19 SE with made-up data.
Only its output was read, never its code. RCraft's tables were compared with these character by character.

- A blank line, the variable's name (with ` -- ` and its label when it has one), a line of dashes, a header
  `|      Freq.    Percent      Valid       Cum.`, a line of dashes with `+`.
- One row per value, the first starting `Valid   `: the value as %9.0g (`.5`, `1234567`) and its value
  label after a space (`0 zero`); then `        Total` when there is more than one value; then `Missing .`
  (a string's empty value: `Missing ` with a blank) and `Total` when any value is missing; a line of dashes.
- The label column is 8 characters, the longest entry (Total counted when its row is shown) and a blank;
  the four numbers are 11 wide: the count, its percent of all rows, of the valid ones, and the cumulative
  valid percent (blank on the Total and Missing rows).
- `fre a b` prints a table for each, a blank line between them; r() is the last one's.
- r(): N, N_valid, N_missing, r (values, missing ones too), r_valid, r_missing; r(depvar); r(lab_valid),
  r(lab_missing) and the matrices r(valid) and r(missing).
