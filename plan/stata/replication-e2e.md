# Whole packages, and what they found (P2C.9)

Observed on a licensed Stata 19 SE with made-up data, 2026-11-03. Only what Stata did is written here.

- `lincom _cons + 1.g + 3.g` after `xtreg y i.g ..., fe`, where 1 is g's base, counts `1.g` as 0. The
  header writes it `1b.g + 3.g + _cons = 0`, in e(b)'s order. `lincom 1.g + 2.h` with both bases prints
  `(1) | 0 (omitted)`, and r(estimate) and r(se) are 0.
- `recode age (10/29 = 1 "young") (30/59 = 2 "mid"), gen(agegrp)` makes a label set `agegrp` (the new
  variable's name) and attaches it; `label(mylab)` names it instead. Two rules giving one value two labels
  keep the later. A set of that name already there gains the new labels (`label define a5 7 "seven"`, then a
  rule for 1, lists 1 and 7). Without `generate()`: "Rules defining value labels not allowed when
  overwriting a variable", r(198).
- `decode x, gen(s)`: the label of each value, and "" where the value has none or is missing. With `if`, ""
  outside it. Without labels: "x not labeled". A string variable: "not possible with string variable".
- `xpose, clear`: variables v1 to vN (one for each observation), float. String variables become rows of
  missing values. `varname` adds `_varname` (str) with the old names. `promote` makes them double, and a
  float's value stays the float's (1.1 as `+1.19999a0000000X+000`). Without `clear`: "you must specify ',
  clear'".
- `do file, nostop` carries on past errors in that file (ZNMIXH's run went on past 188 of them).
- In batch mode a package's master file stops at its first error (`tabm`, `coefplot`, `winsor`, files the
  package expects opened by hand). With `nostop`, Stata reached 2,936 model lines across seven packages.
