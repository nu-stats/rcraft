# Two factors interacted (P2C.15)

The rules come from StatCraft's decoding (`plan/stata/factor-variables.md` there, probes 286 to 289). They
were checked again on Stata 19 SE with made-up data (a do-file of nine regressions, P2C.15) and in the
corpus file `interactions.do` (16 results).

## Which cells are estimated

A cell is left out when one of its factors is at its base and the term made of its other factors is in
the model. The constant counts as that term even with `noconstant`.
- **`i.g##i.h`:** the cells with either factor at its base are left out. This is R's treatment coding of
  `factor(g)*factor(h)`.
- **`i.g i.g#i.h`:** the cells with h at its base are left out. R's coding of `factor(g) + factor(g):factor(h)`
  is the same.
- **`i.g#i.h` alone:** only the cell where both are at their base is left out. R would give every cell a
  column and leave out the last one, as collinear. RCraft codes the pair as one factor of their cells
  (`stata_cells`), every combination of levels, the base cell first, so R's treatment coding leaves out
  Stata's cell by construction. A first try put each factor's base level last and leaned on R to find
  the last column collinear. That held on this machine, but on Windows CI `glm`'s tight check missed it
  and the logit failed. An empty cell is a level with no rows, which R drops; `_b[]` still gives its 0,
  and `e(b)` its `o` column.
- **A bare variable in an interaction is a factor:** `g#h` is `i.g#i.h`, and `g##c.x` makes g a factor.
  Before P2C.15 RCraft read it as a continuous variable, a different model, without saying so.

## Names in `e(b)`

Every cell is named, the first factor outermost and levels ascending. A factor at its base is marked
`b`, and a factor in a cell that is not estimated is marked `o`:
- `i.g##i.h` gives `1b.g#0b.h`, `1b.g#1o.h`, `2o.g#0b.h`, `2.g#1.h`;
- `i.g#i.h` gives `1b.g#0b.h` left out but `1b.g#1.h` estimated, so its `b` stays.

An empty cell is `o` marked. `matrix list` wraps at `set linesize`, which RCraft now keeps in R
(`stata_linesize`).

## The table (regress)

- **Cell rows:** `2 1` with a space when neither factor has value labels, `2#yes` or `alpha#1` when either
  has.
- **A single factor:** with one shown level and no labels it is inline (`1.h`); with labels it is a group
  (`h`, then `yes`).
- **Long labels:** a label past 11 characters is shortened to 9 and `..`. In a cell, a long first label
  goes on a line of its own ending in ` #|`, with the second label below.
- **Shown cells:** those estimated, and those left out with no factor at its base. An empty cell is
  `0  (empty)`.
- **Separators:** a blank `|` line goes between two terms when either is an interaction or a group of
  more than one row.
- Any other shape (a long second label, say) is printed in R's layout, as before.

## Not done

- Three or more factors interacted alone (`i.a#i.b#i.c` without lower terms): refused with the reason.
- Interactions in the likelihood models' tables, which RCraft still prints in R's layout. Their `e(b)` is
  Stata's.
