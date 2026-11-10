# graph bar (percent), over(..., sort() descending), blabel(bar), hbar's order, twoway text() (2026-11-10)

Observed on Stata 19 SE with made-up data, the graphs exported as SVG and PNG (probe `gb/g1.do` in the
session scratchpad).

- `graph bar (percent), over(x)`: each group's share of the rows, the rows with x missing left out (No 40,
  Yes 60 of five rows, one missing); the value axis titled `percent`. `over(x, missing)` is r(198) there.
  `(percent)` with variables is not translated.
- `over(lab, sort(pct) descending)`: the groups in the order of the bars' values, the largest first;
  `sort(1)` the first variable's bars; without `descending` the smallest first. Without sort(): the
  groups' sorted order.
- `graph hbar`: the first group at the top (RCraft had drawn it at the bottom).
- `blabel(bar, format(%3.1f))`: each bar's value at its end in the format (`40.0`); `blabel(bar)` in
  `%9.0g` (`40`). Other blabel() kinds are not translated.
- One series: the value axis titled with it (`mean of pct`), and no legend.
- `twoway ..., text(5 1.025 "1.6%" ...)`: the text at y = 5, x = 1.025 (`text(y x "text")`, several
  places in one text(), compound quotes as in ZNMIXH); its colour and size are its look.
