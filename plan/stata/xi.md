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

## Open

Refused: a factor in two terms, more than two parts in a term, `|` between two factors, `noomit` and other options,
noninteger or negative levels, names over 32 characters, two equally frequent levels under `prevalent`, a model
after the colon that is not translated. In a dry run the line is "known only when run" (the levels are R's).
