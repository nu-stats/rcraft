# marginsplot, margins' r(table) and at() order, putexcel matrix(), _est_ variables (2026-11-13)

Observed on Stata 19 SE with made-up data (probes `mp/m1.do` to `m6.do` in the session scratchpad)
and FMDLII's do-file run whole in Stata 19 (`do file, nostop`, nothing installed); corpus
`marginsplot.do`.

## margins (not posted)

- r(table) is 9 x k: b, se, z (t with the model's df after regress), pvalue, ll, ul, df (. for z),
  crit, eform (0). After `xtreg` (fe and re, with or without vce(cluster)) the statistics are z.
- The names are Stata's full ones: the first level of each term is `#bn.`: `1bn._at 2._at`,
  `1bn.g 2.g 3.g`; `matrix list` shows `1.` over `_at`; plain `margins` is `_cons`.
- at() over several variables: the grid takes the variables in the model's order (e(b)'s, first
  appearance: `regress y w c.a##c.p` is w, a, p) whatever their order in at(), the last changing
  fastest (`1._at: w=0 a=0 p=0`, `2._at: w=0 a=0 p=1`, ...). RCraft had the first changing fastest.

## marginsplot

- Prints "\nVariables that uniquely identify margins: a p" (the at() variables with several values, in
  at()'s own order; `margins g, at(a=(0 1))` gives `a g`; after dydx() or plain margins nothing after
  the colon) and draws the graph.
- r() is left as margins left it (return list the same before and after).

## putexcel

- `putexcel set` and its writes leave r() (FMDLII writes r(table) after them).
- `cell = matrix(M)`: the values from the cell; `names`: column names on the cell's row from the
  next column, row names down the cell's column from the next row, the corner left empty;
  `rownames` and `colnames` one of them; numbers to 17 digits; a missing value an empty cell.

## _est_name (estimates store, eststo)

- `estimates store name` and `eststo name:` add the byte variable `_est_name`, labelled
  "esample() from estimates store", %8.0g, 1 in e(sample) and 0 elsewhere, at the end of the data;
  storing a name again drops it and adds it again at the end.
- `estimates drop name` and `eststo drop name` drop it; `estimates clear` drops them all; `eststo
  clear` only eststo's own (`_est_a3` of `estimates store a3` stays).
- FMDLII's HDL_LA_equilibria.dta has `_est_level`: RCraft's saved file now has the same 14 variables
  and values as Stata's.
