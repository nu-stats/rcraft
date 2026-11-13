# tabulate x y, row column cell nofreq (2026-11-12)

Observed on Stata 19 SE with made-up data (probe `tb/b1.do` in the session scratchpad, its log set line by
line against RCraft's output: identical); corpus `tabulate_pct.do`. Printed by
`crates/statatr/src/data_r/tabpct.rs`.

- Each cell is a stack of lines: the frequency, then the row, column and cell percentages asked for, in
  that order whatever the order of the options; percentages in `%.2f` right-aligned in the 10-wide
  columns, the Total column too.
- More than one line a cell: a Key box before the table, as wide as the longest of its lines plus 2,
  "Key" left-aligned after a space, each line centred (the extra space on the right):

      +-------------------+
      | Key               |
      |-------------------|
      |     frequency     |
      |  row percentage   |
      | column percentage |
      +-------------------+

  and a rule after every row of the table (the one before Total among them). One line a cell (one
  percentage with `nofreq`): no Key and only the rule before Total, as in the counts table.
- Row percentages: each row's counts over its total (its Total 100.00), the Total row the column totals
  over N. Column percentages: each count over its column's total (the Total row 100.00), the Total column
  the row totals over N. Cell percentages: each over N.
- `nofreq` alone prints nothing, and r(N), r(r) and r(c) are then missing: r() is left empty.
- Widths, the column variable's name over its columns, the blocks of columns and the chi2 line are the
  counts table's. `missing` adds the "." column. r() is as without the percentages.
- Not observed, so refused: `nofreq chi2` with no percentage.
