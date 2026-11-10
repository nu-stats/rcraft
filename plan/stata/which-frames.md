# which, frame and set obs (2026-11-10)

Observed on a licensed Stata 19 SE (probes in the session scratchpad: `which/w.do`, `frame/f.do`,
`frame/so.do`, `frame/so2.do`). Corpus: `which.do`, `frames.do`.

## capture which name

- Prints nothing; _rc 0 when found (`regress`, `esttab`, `estout`), 111 when not (`nosuchcmd_xyz`, and
  package names that are no command: `blindschemes`, `lassopack`). A program defined in the do-file is
  not found (111): which looks for files.
- `quietly which` leaves _rc as it was (only capture sets it).
- `capture noisily which nosuch`: "command nosuch not found as either built-in or ado-file"; without
  capture the same and r(111).
- RCraft answers for itself: found when it translates the command (the estout package for esttab,
  eststo, estadd and estpost); an add-on it does not translate counts as not installed. Without capture
  or quietly it is refused (Stata lists the file and its version lines, not reproduced).

## frame create | change | drop name, frame pwf

- `capture frame drop nosuch`: 111. `frame create` of a frame there already: 110. `frame change nosuch`:
  111. Dropping the current frame (`default` while current, or the one changed to): 119.
- A new frame is empty (`_N` 0, `c(k)` 0); each frame keeps its own data; e() stays across frames
  (`e(N)` after `frame change`).
- `frame pwf`: `  (current frame is default)`. `frame dir`: `* default  5 x 1` and so on, then "Note:
  Frames marked with * contain unsaved data." (not translated). `frame fc: display "inside"` runs the
  command in that frame (not translated).

## set obs n

- "Number of observations (_N) was 0, now 5." each time, also when n is the number there already
  ("was 5, now 5."), with commas from 1,000 on ("was 12,345, now 1,234,567."); nothing under quietly.
- Fewer than there are: "observation number out of range", r(198). RCraft had printed nothing; it now
  prints the line as Stata does.

## preserve in frames

- preserve is a frame's own: after `preserve` in default and `frame change b`, `capture restore` is 622;
  a preserve in b and one in default each come back with their own frame's `restore` (probe `pr.do`:
  `_N` 3, 5 after restore in default, 1, 2 after restore in b). RCraft keeps each frame's pending
  preserve with it (`stata_frames_preserved`).

## drop and r(), capture drop of what is not there

- A drop that fails keeps r(): `summarize x, detail` then `capture drop nosuch` or `capture drop Z*b`
  (no match): _rc 111, r(p50) and r(N) as before. A drop that runs clears r() (`drop y`, `drop Z*a`:
  r(p50) is `.`); `drop if` leaves r(N_drop); `keep if` clears r() as well. `which`, `frame create`,
  `change`, `drop` and `set obs` keep r(); `frame pwf` clears it (probes `drop/d.do`, `r.do`, `r2.do`).
- RCraft: where it knows the data's variables, `capture drop` or `capture keep` of a name or pattern the
  data does not have stops in R with r(111), so _rc is Stata's and r() and the data stay (it had refused
  a pattern, and lost r() for a name). After a drop that runs, r() is still taken as not known (Stata
  has cleared it), so what reads it is refused, not guessed.

## string(x [, fmt])

- `string(n)` is `%9.0g` (`1234567`, `.3333333`, `.0000123`, `1.23e+11`, `1.00e+20`); `string(n, "%4.1f")`
  in the format given, `%12.2fc` with commas; leading blanks taken off, trailing kept (`"%-8.2f"`:
  `3.00    `); `.` for missing, `.a` for .a. `strofreal()` the same. RCraft: Stata's format through
  stata_fmt, value by value (it had written R's digits for `string(n)` and refused a format); `.a` still
  prints as `.` (as `display .a` does in RCraft).
