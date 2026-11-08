# center (SSC) (2026-11-08)

ZNMIXH's 01_analysis.do standardizes its moderators with `center $hetvars, inplace standardize`
before splitting them at the median. center is Ben Jann's add-on from SSC, installed in Stata 19
with the user's OK on 2026-11-08; its program file was not read. Observed on made-up data;
`center.do` keeps the checks (to 1e-13).

- Each variable less its mean, divided by its standard deviation with `standardize` (summarize's:
  sqrt(sum((x - m)^2) / (n - 1)); aweights and fweights as summarize takes them).
- The rows: those of `if`, and with `casewise` (shortest `c`) only those with none of the variables
  missing. A new variable is missing on every other row. `inplace` keeps the rows outside `if` as
  they were and sets the rows casewise leaves out to missing.
- Without `double` the mean and standard deviation are rounded to float before they are used:
  a new variable is float((x - float(m)) / float(s)); `inplace` on a double gives
  (x - float(m)) / float(s), still a double; on a float, byte or int the result is float. With
  `double` (`d`) nothing is rounded and new variables are double.
- New variables are c_x (`prefix()`, shortest `pre`), format %9.0g (%10.0g double), labelled
  "the label (centered)" or "(standardized)", the variable's name when it has no label; `nolabel`
  (`nol`) leaves none. `inplace` (`inp`) relabels the variable the same way.
- `replace` (`r`) overwrites an existing c_x; without it r(110). A string variable: "string
  variables not allowed in varlist; s is a string variable" r(109). `inplace` with `prefix()`:
  "prefix() not allowed with inplace" r(198). No varlist: r(100).
- It shows "(generated variables: c_x c_y)" or "(modified variables: x y)".
- `in` behaves oddly: with `in 3/7, inplace` the rows in range are left unchanged and some rows
  outside it become missing. Refused.
- Not translated: pweights, iweights, generate(), meansave(), sdsave() (sdsave needs standardize).
- RCraft does not know the storage type of a variable read from a file: for `inplace` it is taken
  as double when a value cannot be a float, else as float (Stata's default), with a note.
