# esttab's stats(), varlabels(), mgroups(), starlevels(), compress, nogaps; estpost summarize and cells() (2026-11-10)

Follows `esttab.md`. Observed on a licensed Stata 19 SE with made-up data (four probes in the session
scratchpad, `esttab2`), estout as installed from SSC; its code and help were not read. RCraft's output of
the same probes was the same as Stata's in all 50 files (.tex, .csv, .txt) and on the screen.

## stats(names, fmt() labels())

- Only the rows named, in their order: N only when named; `r2`, `ar2`, `noobs`, `obslast` are left aside.
- `fmt()`: one per row, the last for the rest (`fmt(0 2)` gives the third row 2 decimals). Without
  `fmt()`, every row takes `b()`'s format, N too (`b(%9.3f)` prints N as `300.000`); without `b()`, a3.
- `labels()`: one per row; rows past them show their names (`r2`, `cm`), and N is `N` (not `\(N\)` in .tex,
  not `Observations` with `label`).

## varlabels(name label ...)

- Pairs in turn: `varlabels(Z "Treatment" "$\times$" x2 "Cov")` labels Z, then `$\times$` (no such
  coefficient) as x2; `Cov` is left over and x2 keeps its name.
- It takes away the blank lines (`[1em]` in .tex) between the coefficients, as `nogaps` does.
- With `label`, the variables not named keep their labels, but `_cons` stays `_cons` (`\_cons`), not
  `Constant`.
- In .csv the first blank line and those before the coefficients, the statistics and the notes stay.

## mgroups(labels, pattern() prefix() suffix() span)

- A row above the numbers. `pattern(1 0 1 0)`: a group starts at each 1; without `pattern()` one group of all
  the models (the first label; the others are left out). A pattern longer than the models: the rest is left
  out (ZNMIXH's TableF11, written by Stata).
- Without span: the label in the first model's column, right-aligned as the numbers are (screen, .txt,
  .tex: `&       G one         `; .csv `="G one"`).
- With span: the label from the left of the models it spans, padded to their width (screen: a space, then
  k × (1 + modelwidth + stars) − 1; .tex: k × (modelwidth + stars) + k − 1). `prefix(\multicolumn{@span}{c}{)
  suffix(})` wraps it, `@span` the number of models.
- Not checked: prefix()/suffix() without span or outside .tex, span in .csv.

## starlevels(* .1 ** .05 *** .01)

- estout's form of `star()`: the stars as typed (`***`, not `\sym{***}`, in .tex), in a column as wide as
  the longest symbol; the note as typed (`* p<.1, ** p<.05, *** p<.01`, in .tex too).

## compress, nogaps, nomtitles in .tex

- compress: model columns 9 wide, the name column 10 (16 with `label`), on the screen and in files.
- nogaps: no blank lines between the coefficients; in .csv no blank line at all.
- The header rows in .tex are each model's `\multicolumn{1}{c}{...}` padded from the left to the model's
  width (modelwidth + the stars' 9 or as typed). With `nomtitles` the numbers sit in the coefficients'
  column, right-aligned, with the stars' blank after them.
- Names in .tex: padded by their length as written, then `_` escaped (`Covariate\_two` + 7 spaces to 20);
  `\_cons` padded as it is printed.
- `tex` without `using` prints the .tex on the screen after a blank line; `tex booktabs` together is
  Stata's r(198) "only one allowed of smcl, fixed, ...".
- `esttab` without names on the screen, taking eststo's models: "(tabulating estimates stored by eststo;
  specify "." to tabulate the active results)" first (not when it writes a file).
- `%w.df` and the other Stata formats print as Stata's string() does: 12344.19 in `%8.3f` is `1.2e+04`.

## estpost summarize varlist [if] [in]

- e(count), e(sum_w), e(mean), e(Var), e(sd), e(min), e(max), e(sum): 1 × k, columns the variables, rows
  unnamed (`r1` in matrix list). e(N): the rows where any of the variables is nonmissing (x3 with 280 and x5
  with 10, disjoint: 290). e(k), e(cmd) `estpost`, e(subcmd) `summarize`. No e(b) and no e(sample); r() is
  cleared (`display r(mean)` is `.`).
- Its table: `e(count)` ... headed columns of 10 after a 12-wide name column and ` |`, %9.0g, a space after
  each; as many columns as fit the linesize (5 at 80), the rest in a block below; a blank line before each
  block. A string variable gets only count, sum_w and sum (0); no rows: "no observations", r(2000).

## esttab, cells("mean(fmt(%8.2f)) sd(fmt(%8.2f)) min max count(fmt(%8.0f))") after estpost

- A column for each statistic, a row for each variable, then N in the first column (`\(N\)`, `N`, or
  `Observations` with `label`); no notes, no stars.
- A statistic without `fmt()` takes the first one's format (`mean(fmt(2)) sd(fmt(a2)) count` prints count as
  `300.00`); when the first has none, %9.0g (`mean sd(fmt(2))`: `.252918`, `1.04`).
- .tex: `l*{1}{ccccc}`; `\multicolumn{5}{c}{(1)}` and the empty title `\multicolumn{5}{c}{}` padded to
  5 × 12 + 4; the statistics' names right-aligned in 12; names not cut. `noobs`, `nonumbers`, `nomtitles`
  (the numbers then in the first column, the others blank), `label`.
- Screen and .txt: columns of 1 + 12 (9 with compress), the number over the first, a blank title row; names cut.
- `esttab` of estpost's results without `cells()`, and cells() of other models, are not translated yet.
