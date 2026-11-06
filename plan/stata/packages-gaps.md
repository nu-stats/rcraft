# Packages' gaps: ibn., format, postfile, graph save and combine (2026-11-06)

The public replication packages' lines that were not translated and need no add-on installed.

## ibn. factor variables

`regress y x ibn.g, noconstant` gives every level of `g` a coefficient (`1bn.g`, `2bn.g`, ...),
no base. R: `stata_ibn(g)` is `factor(g)`; with no constant, R's model matrix already keeps every
level of the first factor, so the coefficient names map back to `kbn.g` in `e(b)` and `_b[kbn.g]`.
`ibn.` without `noconstant` is declined (Stata then drops a level as collinear, which RCraft does
not reproduce yet). After `regress ..., noconstant` Stata leaves no `e(ll_0)`; RCraft leaves none
too. Checked in `postfile.do` against Stata 19.

## format

`format varlist %fmt` keeps the format as the variable's `format.stata` attribute, which haven
writes to a saved .dta; the values do not change. Patterns and ranges of variables are declined.

## postfile, post, postclose

From StatCraft's decoding (`plan/stata/postfile.md` there), checked in Stata 19:
- a type name applies to the next variable only, or to each name in `type(a b c)`; the default is float;
- `post` stores each value in its variable's type: a float is rounded to single precision as it is
  posted (so `mean` comes back as 12.8333330154418945), a double is kept, a string is cut to its width;
- `replace` with no file prints `(file name.dta not found)`; an existing file without replace is r(602);
- a wrong count of expressions (r(198)) or a string for a number (r(109)) closes the postfile;
- `postclose` writes the file. RCraft writes it with haven; the values are Stata's exactly (they are
  kept as numbers, not printed and read back, which lost the last digits in a first try).
`every(#)` is taken and changes nothing: it only sets how often Stata writes to the file.

## graph name(), saving(), graph save, graph combine

A graph drawn with `name(n)` is kept as `stata_graphs[["n"]]`; `saving(f)` and `graph save [n] f`
keep the ggplot object in the file with `saveRDS` (the file is RCraft's own, not a Stata .gph).
`graph combine n1 n2 ...` (names or files saved that way) lays the plots out with gtable, `cols()`,
`rows()` and `title()` as Stata's; it is what `graph export` after it writes.
