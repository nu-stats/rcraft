# xi (2026-11-19)

Observed on Stata 19 SE with made-up data (corpus `xi.do`, 54 results to 1e-8). StatCraft's note (`factor-variables.md`,
probe 297; ported from statcraft@b7d392e) had the naming and the labels; the e(cmdline) stubs, the drop rule and the
header columns were decoded here. Translated in `crates/statatr/src/data_r/xi.rs`, with a question to R in the do-file
runner (`dofile.rs`) because the dummies' names depend on the data's levels.

- `xi: cmd`, `xi [, prefix(str)]: cmd` and `xi terms`. `i.g` makes a byte `_Ig_#` for each level of g but the omitted
  one, labelled `g==#` (a value label is not used: `h==1`, not `h==yes`), missing where g is missing. The omitted level is
  the lowest, the most frequent with `char _dta[omit] prevalent`, or `#` with `char g[omit] #`.
- `i.g*i.h`: the dummies of both, then `_IgXh_#_#` (byte, `g==2 & h==1`) for each non-omitted pair, g outermost.
  `i.g*x`: the g dummies, x and the float `_IgXx_#` (`(g==2)*x`, format `%9.0g`). `i.g|x`: the same variables, but the
  command gets `x _IgXx_#` only (the dummies stay in the data).
- The command runs with each term replaced by the names (`regress y _Ig_2 _Ig_3 x`), **e(cmdline) keeps the stubs**:
  `regress y _Ig_* x`, `regress y _Ig_* _Ih_* _IgXh_*`, `regress y _Ig_* x _IgXx_*`, `regress y x _IgXx_*`.
- Printed first, one line per term (a factor in `i.g*i.h` gets its own line before the cross): the term padded to 18
  columns, the names padded to 20 (`_Ig_1-3`, first and last level of all levels; `_IgXh_#_#`; `_IgXx_#`), then
  `(naturally coded; _Ig_1 omitted)` or `(coded as above)`.
- Each run drops the variables an earlier run made **with the same prefix**; ones with another prefix stay (after
  `xi, prefix(_D): ...` the `_Ig_*` variables are still there, and the next `xi: ...` replaces them).

## Round 2 (2026-11-20): other levels, noomit, the product's type

Corpus `xi.do` now 126 results (was 54), the labels and cmdline checked against Stata.

- **Naming.** A factor whose values are all whole numbers from 0 up is named by value (`_Ic_2-9`, `_Ibig_0-1000000`,
  any size). Anything else (a negative or fractional value, a string) is named by rank, 1, 2, 3, ... in sorted order
  (strings in byte order, a missing or empty string left out): `i.m  _Im_1-3  (_Im_1 for m==-1 omitted)` where a natural
  one reads `(naturally coded; _Ig_1 omitted)`. The labels use the value as `g==label` writes it (up to 16 digits, no
  leading 0: `r==.3333333333333333`, `f==.1000000014901161` for a float, `m==-1`, `s==lo`); in a cross each factor keeps
  its own naming (`_ImXa_2_1`, label `m==0 & a==1`).
- `char s[omit] lo` works for strings and for the by-rank levels: `(_Is_2 for s==lo omitted)`. A char naming a level the
  variable does not have is a message in Stata and still stops here.
- **noomit** (an option before the terms: `xi, noomit i.g` and `xi, noomit: cmd`): a dummy for every level, for the main
  factors and the products, and no line for a plain `i.g` (only the product line is printed). Not changed: the command
  after the colon decides what it does with the collinear columns; Stata and RCraft pick different ones to omit when
  every dummy of two factors is in a model (a `regress` limit, not xi's), so that case is not in the corpus.
- **The product's type.** `i.g*x` has x's type: a double x gives a double with `%10.0g`, a float `%9.0g`.
- **Not changed:** `xi i.g i.g*x` (the factor in two terms: Stata prints its line once and the command gets the names
  twice), `i.g|i.h` (Stata's `I.xxx|I.yyy not allowed` r(198)), more than two parts, other options.

## Open

Refused: a factor in two terms, more than two parts in a term, `|` between two factors, other options,
names over 32 characters, two equally frequent levels under `prevalent`, a model
after the colon that is not translated. In a dry run the line is "known only when run" (the levels are R's).
