# esttab matrix() and %f near a half (2026-11-13)

Observed on Stata 19 SE (probes `etm/m1.do`, `m2.do`, `mini/rnd/s.do` in the session scratchpad) and
YFBPT6's representative.do; corpus `fmtround.do` for the rounding.

## esttab matrix(M[, fmt()]) [using f] [, options]

- Rows and columns by the matrix's names; a first head row of the model label: the matrix's name in
  the first column (right-aligned), `mlabels(" ")` a blank row (kept, trailing blanks and all),
  `mlabels("Title")` that text, `mlabels(none)` no row.
- Cells: without fmt() %9.0g (1234567.891 is 1234568, .000123 stays); fmt(2) two decimals with no width
  limit (1234567.89); fmt(2 3) a format a column; a missing value `.`; -0.001 is -0.00.
- Screen and .txt: rules of 12 + 13k dashes, names 12 wide on the left, cells 13 wide on the right;
  title() above the first rule. The screen has a blank line before and after; a .txt file has neither.
- .tex: `\begin{tabular}{l*{k}{c}}`, `\hline\hline`, the head rows (`name&cell...\\`, names
  12 wide on the left, cells 12 wide on the right), `\hline`, the rows, `\hline\hline`,
  `\end{tabular}`. With `fragment`: the head rows, `\hline`, the rows only; posthead() in place of the
  `\hline`, prefoot() and postfoot() after the rows. Lines end CR LF.
- .csv: `="..."` cells joined by commas, the model-label row first.
- Not observed, refused: fmt(%fmt) (Stata switches 1234567.891 in %9.3f to 1.23e+06), prehead(),
  heads outside .tex or without fragment, title() in a file, other options, names or cells wider than
  12.
- YFBPT6's representative1.tex and representative2.tex are now written byte for byte as Stata's.

## %f of a value next to a half

- Stata rounds the double: 0.005 (0.005000000000000000104) is 0.01 in %9.2f, 0.025 is 0.03,
  0.25000000000000006 is 0.3 in %9.1f, 0.5000000000000001 is 1 in %9.0f; exact halves go to even
  (0.125 is 0.12, 2.5 is 2).
- R on Windows printed 0.00, 0.02, 0.2 and 0 for those (its printf rounds from 17 digits). stata_fmt
  now decides a value within 1e-6 of a half from the double's own bits (m 2^k 10^q against the half,
  in 16-bit whole numbers), so every %f display, not only esttab's, matches on every platform.

## A float through save, use and merge

- representative.do's quotas are floats (`recast float quota, force`), saved to a tempfile and merged
  back; `replace quota = floor((quota * 100) * 100) / 100` then stores a float in Stata (9.1199999 for
  9.12). RCraft had forgotten the storage after the merge and kept 9.119999999999999, and one row read
  9.11 where Stata has 9.12 (0.0912 * 10000 below 912 in double, above it in float).
- A file this do-file saved now keeps the kinds and storage types the translator knew: `use` restores
  them, `merge` takes them for the variables the data in memory lacks (the keys stay the master's), and
  R stops if the data in memory turns out to have one of them already (its type unknown). `append` is
  left as it was: there the files share most variables and Stata widens their types.
  Corpus `savedfloat.do`.
