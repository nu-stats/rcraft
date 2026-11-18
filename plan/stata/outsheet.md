# outsheet (2026-11-18)

Observed on Stata 19 SE with made-up data; corpus `outsheet.do` (every file is read back line by line
and compared; 19 results). Translated in `crates/statatr/src/data_r/outfiles.rs`, writing through the
`export delimited` helper (`stata_export_delim`, `sheet = TRUE`). StatCraft's note (raw-files.md) is
right on the layout; what it did not say is below.

- `outsheet [varlist] using file [if] [in] [, comma delimiter(char) nonames nolabel noquote replace]`;
  `if` may stand before `using`. Extension `.out` when the name has none.
- Numbers are written in their display formats, trimmed (`%8.3f` gives `1.500`, a float `%9.0g` gives
  `.1`, a double `%10.0g` gives `.33333333`, `%12.0gc` gives no commas, too wide for the format gives the
  e form `1.235e+11`); dates in their date format (`30jun2017`). A missing number is an empty field;
  `.a` to `.z` are written as `.a`.
- Strings and value labels are in double quotes unless `noquote`; an empty string is `""`; a quote
  inside is not doubled (`"has "quote""`). A value with no label in a labelled variable is a plain number.
- Nothing is printed when the file is written. `(file x not found)` only with `replace` and no file.
- Errors: file exists without replace `file x already exists` r(602); `variable x not found` r(111);
  no using `using required` r(100); `option bogus not allowed` r(198) (`tab` too);
  `delimiter(ab)` and `delim(tab)` `option delimiter() incorrectly specified` r(198) (one character
  only, `" "` and `"|"` work); `comma` with `delimiter()` `both comma and delimiter() not allowed`
  r(198). No observations or no variables write the file and give r(0).
- Abbreviations seen: `outs`, `noq`, `nol`, `non`, `delim()`.

Refused: a variable list with ranges or wildcards.
