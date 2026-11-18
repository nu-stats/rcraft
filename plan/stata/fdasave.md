# fdasave and fdause (2026-11-18)

Observed on Stata 19 SE with made-up data. The files RCraft writes were set against Stata's, byte for byte
with the two 16-byte time stamps masked (the files of StatCraft's probes 524 and 525 recorded outside the
repo, and ones made here): `fdasave` of the same data gives the same `.xpt` (f1, f3, f4, f5, f6, f7), the
same renaming messages and the same `.sas` apart from the folder. The R reads Stata's files and writes
them again byte for byte. Corpus `fdasave.do` (values read back, return codes; the bytes cannot be read
in a do-file). Written in R in `crates/statatr/src/data_r/xport/` (`xport.R` the format, `fdasave.R`,
`fdause.R`), translated in `data_r/xport.rs`. The format and the messages are StatCraft's
(raw-files.md, probes 524 to 529); what was added is below.

- **Order, with replace:** after the no-variables check (r(111), the file stays) an old `file.xpt` is
  **erased first**, so that no observations (r(2000)), a number of 9.046e+74 or more or a string over 200
  bytes (r(459)), a value label not defined (r(111)) and the names that must change (r(110)) all leave no
  file. Without `replace`: no variables, no observations, then `file x.xpt already exists` r(602).
- `label drop` on a label a variable uses leaves the variable with the label's name and no entries:
  `fdasave` then says `value label zz not found` r(111); a read-back `decode` is `label zz not found` r(111)
  (a variable with no label at all is `x not labeled` r(182)).
- `fdasave, replace` with no file name: `varlist required` r(100). `fdause` with no file: `invalid file
  specification` r(198); a second word `invalid 'b'` r(198). Bad option `option x not allowed` r(198);
  a bad `vallabfile()` r(198). A name or a file that is not there r(601).
- With data in memory `fdause` needs `clear`: `no; dataset in memory has changed since last saved` r(4).
  Not translated when the data is not empty (Stata may not stop when the data is unchanged).
- Tagged missing values (`.a` to `.z`) are written as the letter in capitals and read back as `.a` to
  `.z`; a plain missing is `.`.
- Not translated: `export sasxport5`, `import sasxport5`, `fdause`'s `member()` and the dataset label
  (`label data` is not written by Stata either).
- A string's width: R has no declared width; RCraft keeps it as the display format `%20s` on the column for
  `input str#`, `generate str#` and `fdause`, else the longest value, so a string made another way can
  come out narrower than Stata's. The name of a value label is kept on the column (`vallab`) by `label
  values` and `fdause`; a labelled column without a name (read from a .dta) is refused.
