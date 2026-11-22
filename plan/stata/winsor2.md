# winsor2 (SSC) (2026-11-22)

winsor2 (Yujun Lian) installed in Stata 19 with the user's OK on 2026-11-22; its program file was not read.
Observed on made-up data; ported also from the behaviour StatCraft matches (probe 142, statcraft@b7d392e).
`winsor2.do` keeps the checks (tolerance 1e-13).

- `winsor2 varlist [if] [in] [, cuts(# #) suffix(str) replace trim by(varlist) label]`; names only (patterns are decided
  when the do-file runs).
- The cuts are Stata's own percentile (the value `_pctile` gives: the first value whose running count reaches
  n p/100, the mean of it and the next when it is exactly that) among the nonmissing rows of `if`/`in`, in each group of
  `by()` (a missing by value is a group, so is an empty string). Default `cuts(1 99)`. A 0 or a 100 leaves that end alone.
  Winsor sets values beyond the cuts to them; `trim` sets them to missing (a value equal to a cut stays).
- Without `replace`: new variable `x_w` (`x_tr` with `trim`; `suffix()` changes the tail), missing outside `if`/`in`. It has
  the old variable's storage type (a byte, int or long takes the cut cut short: 11.5 is 11), the default display format of
  that type, and the old label or, with none, the variable's name; `label` makes it `Income-Winsor(p1,p99)` (`-Trim(`
  with `trim`), the cuts written as numbers (`p1.5`, `p10.25`, `p10` for 10.0, and `p00` for 0). A winsorized variable made
  with `by()` is a float whatever the old type (a trimmed one keeps the type).
- With `replace`: the variable keeps its type, format and value labels, and only the rows of `if`/`in` change; a byte or int
  taking a cut that is not whole becomes float; the label is the old label or the name (or `label`'s).
- Errors (rc): no varlist 100; a variable not found 111 (`by(nosuch)` says `(error in option by())`); a new name that exists
  111 `variable x_w already exist, re-specify option -suffix()-` with `Suffix _w is invalid for x`, before any variable is
  made; a string 109 `type mismatch`; `suffix()` with `replace` 198; weights 101; `cuts(5)` 122 and a cut below 0 or above
  100 125 (`cuts() invalid -- invalid numlist ...`); a lower cut above the upper 111 `__000001 not found`; `cuts(0 100)`
  prints `option cuts(0 100) is incorrect, no action taken.` and does nothing (rc 0). A cut strictly between 0 and 1 prints
  a three-line warning first.
- Not translated: patterns and ranges in the varlist, weights (the reader refuses them), more than two numbers in `cuts()`,
  `replace` of a long variable with a fractional cut.
