# revrs (SSC, Kyle Longest), 2026-11-10

Installed from SSC that day with the user's yes; observed on Stata 19 SE with made-up data (probes
`revrs/r1.do` to `r4.do` in the session scratchpad); its code and help were not read. Corpus `revrs.do`.

## A variable with a value label

- The labelled values in the data, in order, take the codes k, ..., 1: values 1, 2, 5 (labels 1 to 5)
  become 3, 2, 1; labels of values not in the data are left out; a value in the data without a label
  becomes missing (`d` 3, 5, ., 4 with labels for 3 and 5: 2, 1, ., .).
- The new label set is `rev<name>`, each code with the old text (`reva: 1 five, 2 two, 3 one`); the
  variable is byte (int with 150 codes), `%8.0g`, its variable label copied.
- Without replace: `rev<name>` at the end. With `replace`: the variable is dropped and comes back at the
  end under its own name, label set `rev<name>`.
- `rev<name>` there already: with `replace` "variable reva already defined", r(110); without, Stata
  leaves everything as it is (_rc 0).
- A value label naming `.a` (`.a "miss a"`) gives `.a` the code after the reversed ones (4), not
  reversed: not translated.

## A variable without one

- (max + 1) - x: 2, 4, 6 give 5, 3, 1; -2, 0, 3 give 6, 4, 1; 1.5, 2.5, 4 give 3.5, 2.5, 1; a constant
  gives 1. Missing values stay, `.a` and `.b` too.
- Without replace: `rev<name>` float `%9.0g` at the end (r(110) if there). With `replace`: in place, its
  storage kept (int stays int, long long).
- A double of thirds (7/3, ..., 11/3) gives values at float precision (2.3333335, 2, 1.6666667,
  1.3333334, 1) that no simple formula reproduced: values that are not whole numbers or halves are
  refused.

## Both

- r() is cleared. `revrs nosuch`: "variable nosuch not found", r(111); a string variable: "string
  variables not allowed in varlist; / s is a string variable", r(109).
