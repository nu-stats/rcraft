# ereturn in programs that estimate (2026-11-05)

Read off Stata 19 by running made-up do-files in batch mode (never its ado or help files).

- Where it works: `ereturn post` and `ereturn clear` work anywhere, at the top of a do-file too and in a
  program that is not e-class. `ereturn scalar`, `local` and `matrix` work only in an e-class program;
  elsewhere "non e-class program may not set e()", r(152), checked before the value (a string given to
  `ereturn scalar` there is still r(152)).
- `ereturn post b V`: moves both matrices into e(). After it `matrix list b` is "matrix b not found",
  r(111). e(b)'s one row is named `y1` whatever `depname()` says. A matrix made without names has columns
  `c1 c2 ...`, and so does e(b).
  - The options: `obs(#)` gives e(N) and `dof(#)` e(df_r), in that order among the scalars; e(properties)
    is "b V" (or "b" with no V, or what `properties()` says), and `depname()` gives e(depvar) after it.
  - `esample(v)`: e(sample) is v's rows, and v leaves the data. Without it no row is in e(sample)
    (`count if e(sample)` is 0) and `ereturn list` shows no "functions:" part.
  - Errors: a V not k x k for a 1 x k b, or a b that is a column, "conformability error", r(503); V's row
    or column names not b's column names (`I(2)` against `c1 c2`), "name conflict", r(507); a matrix not
    there, r(111). A V that is not symmetric is posted with "warning: variance matrix is nonsymmetric or
    highly singular" (RCraft declines it). A failed post leaves the matrices where they were.
  - It leaves r() as it was.
- `ereturn list` order (as StatCraft's probe 147 found too): scalars in the order set while there are 20
  or fewer, later ones first; a scalar set again goes last. Macros newest first; set again, first. e(b)
  and e(V) first among the matrices, the others newest first. `ereturn local w` with nothing (or `= ""`)
  removes e(w) (probe 150). `ereturn local t = 2 + 3` holds "5".
- `ereturn matrix name = M` and `ereturn matrix name M` both move M (`, copy` keeps it); `ereturn matrix
  C` alone is r(198).
- After `regress` in an e-class program, `ereturn scalar` and `local` add to regress's e(): the scalar
  after regress's, the macro first.
- `_b[x]` and `_se[x]` read e(b) and e(V); `_se[]` with no e(V) is "_se not allowed when e(V) is not
  present", r(111); a name not in e(b) is r(111). `[eq]_b[x]` and `_b[eq:x]` name an equation.
- `ereturn display`: a table as regress's, headed by e(depvar) (blank without it), t with e(df_r) and z
  without, `level()`; e(b) alone shows only the Coefficient column; equations are headed by their names
  with a rule between them. It leaves r(table) (9 x k). With no e(b), "last estimates not found", r(301).
- `ereturn repost b = M` works when e(b) is there; `V = M` with no e(V) is r(503). RCraft declines repost.
- `predict` after posted results runs (no error); RCraft declines it, and test, lincom and margins.
