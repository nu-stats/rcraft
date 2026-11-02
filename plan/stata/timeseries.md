# tsset and the time-series operators (P2C.9)

Observed on a licensed Stata 19 SE with made-up data, 2026-11-02. Only what Stata did is written here.

## tsset

- `tsset t` prints `Time variable: t, 1 to 10, but with a gap` and `        Delta: 1 unit` (eight spaces).
  With more than one gap it says `but with gaps`; with none, nothing after the range.
- `tsset id t` prints `Panel variable: id (unbalanced)`, ` Time variable: t, 1 to 5, but with a gap` and
  `         Delta: 1 unit`. Panels with the same times are `(strongly balanced)`; panels with as many
  times each but not the same ones are `(weakly balanced)`.
- `xtset id t` prints exactly what `tsset id t` prints. `xtset id` alone prints `Panel variable: id (balanced)`.
- `tsset` sorts the data by panel and time.
- A time that is not a whole number: `time variable must contain only integer values`, r(451).
- A time repeated: `repeated time values in sample`, r(451); within a panel, `repeated time values within
  panel`, r(451).
- `delta(2)` makes `L.` look 2 time units back and prints `Delta: 2 units`. Two times closer than delta:
  `time values with period less than delta() found`, r(451).
- A string panel or time variable: `string variables not allowed in varlist;` and `s is a string variable`,
  r(109).
- A row with a missing time or panel is left out: the range ignores it and its lags are missing.
- `tsset` alone prints the settings again. `tsset, clear` forgets them, and `L.x` after it is r(111).
- `yearly` prints `Delta: 1 year`; `daily` changes the time variable's format to `%td`, prints dates for the
  range and `Delta: 1 day`, with a warning when the variable had another period's format.

## The operators

- Without `tsset`, `generate z = L.x` stops: r(111).
- `L.x` is x at time t - 1 in the same panel, missing where no row has that time (a gap, the first time).
  `F.x` looks one time ahead. `L2.x` looks two back. `L0.x` is x itself.
- `D.x` is x - L.x. `D2.x` is x - 2 L.x + L2.x. `S.x` is x - L.x and `S2.x` is x - L2.x.
- A chain is worked out as one sum over x, not one operator after another: `D.F.x` is F.x - x, so it
  needs only the rows at t + 1 and t (it is 3 at t = 1 where x is 1 at t = 1 and 4 at t = 2, though no
  row has t = 0).
- `generate lxif = L.x if g == 1` takes the lag from the row before even when that row has `g == 0`: the
  operators read every row, whatever `if` keeps.
- After `gsort -t`, `generate`, `summarize` and `regress` with `L.x` stop with `not sorted`, r(5). So does
  `by g, sort: generate z = L.x`.
- `count if D.x > 0` counts the rows where D.x is missing, as missing is larger than any number.
- `L.s` of a string variable gave missing numbers on every row.

## Names in models

- `regress y L.x D.x`: e(b)'s columns are `L.x`, `D.x` and `_cons`; `_b[L.x]` reads the first. The table
  shows `x` with rows `L1.` and `D1.`.
- A chain takes one canonical name: the net lag or lead, then the differences, then the seasonal
  difference. `l.d.x` and `D.L.x` are `LD.x`; `d.f.x` and `F.D.x` are `FD.x`; `S.L.x` is `LS.x`;
  `L2.L.x` is `L3.x`; `S.D.x` and `D.S.x` are `DS.x`; `S2D.x` is `DS2.x`; `D2L.x` is `LD2.x`; `F.L2.x` is
  `L.x`; `L.F.x` and `F0.x` are `x` (with all 30 rows); `S1.x` is `S.x`.
- `L(0/2).x` is `x L.x L2.x`; `L(1/2).(x y)` is `L.x L2.x L.y L2.y`; `L(1 2).x` is `L.x L2.x`.
- The same term twice is omitted as collinear (`oL.x`).
- `regress L.y x` uses 29 rows, and e(depvar) is `L.y`.
- `regress y L.x if t > 10` keeps 20 rows: the lag of t = 11 is the row t = 10, outside the `if`.
- `predict p` after `regress y LD.x S2.x` gives 28 values, the rows where both operators have values.

## replace that reads its own earlier times

- `replace k = L.k + 1 if k == .`, with k 1 in the first row and missing below, makes k count 1 to 30:
  the rows are replaced one after another, and each sees the rows before it already replaced.
- `replace m = L.m if m == .` carries the last value down a run of missing values.
- `replace mo = L.mo + 1 if mo == . & L.mo != .` fills a run of missing months one after another.
- `replace x = L.x if t == 5` changes one row.
