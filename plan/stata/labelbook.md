# labelbook (2026-11-19)

Observed on Stata 19 SE with made-up data (corpus `labelbook.do`, r() exact; the printed pages and the problems table read against
Stata's log line by line, including a label with a leading blank). StatCraft's probes 520 and 522 name the command; the rules were
decoded here. Translated in `crates/statatr/src/data_r/meta.rs`.

- `labelbook [names] [, problems length(#)]` shows one page per value label set, sorted by name (also when the names are given in
  another order): a heading, the statistics, the `Definition` rows (value right-aligned in 12, three blanks, the text with its
  leading blanks taken off) and `Variables:` (the variables that use the set, in the data's order).
- The statistics: Range `[min,max]` and N (the number of labels), Gaps (an integer between the smallest and the largest value
  has no label), Missing `.*` (labels on `.a` to `.z`), String length `[min,max]` in bytes, Unique at full length, Unique at
  length 12 (`length(#)` changes the 12), Null string, Leading/trailing blanks, Numeric -> numeric (a label that reads as a number).
- r() holds macros only, each when it has a name: r(names) (the sets shown, sorted), r(gaps), r(notused) (no variable uses the
  set), r(numeric), r(blanks), r(nuniq) (not unique at full length), r(nuniq_sh) (not at the short length). They are the same
  with `problems`, which shows the table of problems instead of the pages: `Numeric -> numeric`, `Gaps in mapped values`,
  `Leading or trailing blanks`, `Duplicate labels`, `Duplicate lab. at length 12`, `Not used by variables`, each with its sets, or
  `no potential problems in dataset [unnamed]`.
- A name that is not a set is r(111) (`value label nosuch not found`), also when it comes second.
- Not translated: `list()` and the other options; a set with an empty label (null string: the name of its r() is not known),
  value labels read from a data file (R does not keep the set's name, so which variables use it is not known; a variable whose
  labels came with a file stops the command), `.a`-labelled values (`label define` refuses them already), `problems` with `length()`,
  and the dataset name in the heading (R prints `[unnamed]`).
