# outfile (2026-11-18)

Observed on Stata 19 SE with made-up data; corpus `outfile.do` (31 files read back line by line, every
line compared; made with a generator from Stata's own files, so the expectations are Stata's). Translated in
`crates/statatr/src/data_r/outfiles.rs` (`stata_outfile`). The layout rules are StatCraft's
(raw-files.md, probes 590 and 593), confirmed; what was added is below.

- `outfile [varlist] using file [if] [in] [, comma noquote nolabel runtogether wide replace]`; `.raw` is
  added to a name with no extension. `missing` is not an option (r(198)).
- Free layout: a number right-aligned in its display format's width (the storage type's default when
  none: byte and int `%8.0g`, long `%12.0g`, float `%9.0g`, double `%10.0g`; a `-` in the format and the
  `c` are ignored), a string quoted and left-aligned in max(declared width + 2, 11), a labelled value
  quoted and left-aligned in max(format width, longest label of its value label), two blanks after
  each field. A labelled variable's value with no label is a number right-aligned in the same width.
  Missing numbers are `.` and `.a` to `.z`.
- **noquote:** a string's field is max(declared width, 9) wide (not width + 2), a label's field stays;
  nothing is quoted. A quote inside a string is never doubled: `"has "quote""`.
- Without `wide`: a field that would bring the line to column 80 or more starts a new line, and lines
  lose their trailing blanks; `wide` keeps one line per observation with the blanks.
- **comma:** fields trimmed, joined with commas, strings and labels quoted, **a missing number empty (`.a`
  too)**; `noquote comma` writes them bare; `comma wide` is the same as `comma`.
- **runtogether:** only for string variables (`type mismatch`, r(109), if any variable is numeric or
  labelled); the strings are written one after the other with no quotes or blanks (`annplain`).
- Errors: file exists `file x already exists` r(602); `variable x not found` r(111); no using `using
  required` r(100); `option bogus not allowed` r(198).
- `(file x not found)` is printed only with `replace` and no file there; nothing when the file is written.
- `dictionary` (writes `.dct` with the data) was observed but is not translated.
- String width: R has no declared width. RCraft keeps it as the display format `%20s` on the column for
  `input str#` and `generate str#` (and `replace` keeps it), which a saved .dta also keeps; a column
  made any other way uses the longest value.
