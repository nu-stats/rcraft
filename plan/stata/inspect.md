# inspect (2026-11-18)

Observed on Stata 19 SE with made-up data (corpus `inspect.do`: 19 results, exact). StatCraft's note
(`data-display.md`, probes 520 and 522) agrees; the printed blocks of probes 520 and 522 are identical to
Stata's (compared line by line). Translated in `crates/statatr/src/data_r/descr.rs` (`stata_inspect`).
Batch C of the StatCraft port.

- `inspect [varlist] [if] [in]`: per variable a heading `name:` (with `  label` when labelled, the text cut to
  40 columns), `Number of observations` at column 49, an underline (name + 3 dashes, or the heading's length
  with a label, then 39 dashes from column 40), the Negative, Zero, Positive, Total and Missing rows with
  the Total, Integers and Nonintegers columns (`-` for 0), a 6-row histogram, the smallest and largest
  value (`%9.0g`, the largest left out with one value, `.` and `-9.0e+307` with none), the sample's rows, and
  `(n unique values)` (`(More than 99 unique values)` above 99, singular for 0 and 1).
- The histogram: a column per value for 5 or fewer distinct values, else 5 equal bins from the smallest to the
  largest; a column is floor(6 n / most) high; a column with height 0 shows `.` on the bottom row. Columns
  sit at 4, 8, 12, 16, 20. Distinct values are counted as floats.
- Labelled variables add `x is labeled and all values are documented in the label.` or `... but n values are
  NOT documented in the label.` (n counts rows).
- r(): N_neg, N_negint, N_0, N_pos, N_posint, N (non-missing), N_unique (missing above 99), N_undoc (missing
  unless labelled), of the last variable. A string variable counts as all missing.
- Errors: an unknown option r(101) (not 198), a name the data lack r(111).
- Not translated: `by:`, patterns in the varlist.
