# filefilter (2026-11-09)

ZNMIXH edits the LaTeX table outreg wrote with two `filefilter`s (an extra rule row and cmidrules
in Table 2's header). Observed in Stata 19 on made-up files; `filefilter.do` reads the result
back. Every output file of the probes matched Stata's byte for byte.

- Each occurrence of from()'s bytes becomes to()'s, left to right and not overlapping (from("aa")
  on "aaaa" is 2); everything else is copied as it is, line ends included.
- Codes: \BS a backslash, \Q ", \LQ `, \RQ ', \$ $, \t tab, \n and \U a line feed (alone, also
  in a file of CR LF lines), \r and \M a carriage return, \W CR LF, \###d a byte by its decimal
  code. Any other backslash (\h, \x) is r(198), as is \256d.
- r(occurrences); r(bytes_from) and r(bytes_to), the decoded lengths. An empty from() copies the
  file with 0 occurrences.
- A missing old file is r(601); a new file that exists without `replace`, or the old file
  itself (even with replace), is r(602).
- Not observed yet, so refused: hexadecimal codes (\##h).
