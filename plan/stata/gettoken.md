# gettoken (P2C.9)

Observed on a licensed Stata 19 SE with made-up text, 2026-11-02. Shown as `[first][rest]`. Only what
Stata did is written here.

- `gettoken f r : list` with list `a b c`: `[a][ b c]`. The rest keeps the blank before it.
- `"a b" c`: `[a b][ c]`; the quotes come off. With `quotes`, the token keeps them as a compound quote.
- `x y|z w` with `parse("|")`: `[x y][|z w]`: without a blank among the parse characters, blanks are part of
  the token, and the rest starts at the `|`. Then `gettoken p b : b, parse("|")` gives `[|][z w]`.
- Leading blanks are skipped: `  x|y` with `parse("|")` gives `[x][|y]`; `  x y` gives `[x][ y]`.
- An empty macro gives `[][]`.
- `x<=5` with `parse("<= ")` gives `[x][<=5]`, and then `[<=][5]`: `<=` is one token. `x==1` with
  `parse("= ")` gives `[x][==1]`, then `[==][1]`.
- `a   b  ` gives `[a][   b  ]`: the rest keeps every blank.
- `gettoken (global) H (local) K : (global) G` with G `one two` gives H `one` and K ` two`.
- `gettoken depvar explvar: varlist` (no blank before `:`) with `y x1 x2` gives `[y][ x1 x2]`.
- `a,b` with `parse(",")` gives `[a][,b]`.
- `| x | y` with `parse("|")` gives `[|][ x | y]`, then `[x ][| y]`: the token keeps the blank before the
  next `|`.
- `a` gives `[a][]`, and again `[][]`.
- `a(b c)d e` gives `[a(b][ c)d e]`; with `match(m)` the same and m empty.
- `(b c) d` with `match(m)` gives `[b c][ d]` and m `(`.
- `gettoken a : s` with s `a b` gives a `a` and leaves s as it was.
