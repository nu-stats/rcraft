# frames: dir, copy, rename, put, prefix and block (P2C.12ap)

Written 2026-11-19. Beside `which-frames.md` (create, change, drop, pwf). Code: `crates/statatr/src/data_r/frames.rs`
(the R helper `stata_frame_dir`, `frame_dir`, `frame_copy`, `frame_rename`, `frame_put`, `frame_create_vars`, `frames_reset`,
`in_frame`), the dispatch in `data_r.rs` `frame_cmd`, and the block in `dofile.rs` (`Opener::Frame`). Checked by
`crates/engine/tests/stata_corpus/frames_more.do` (24 results, tolerance 1e-9). Observed on Stata 19 SE with made-up data.

## Rules

- **`frame dir`:** one line for each frame in the order of the names (byte order: `a_much_longer_name`, `alpha`, `bk`),
  a star and a blank, or two blanks, then the name padded to the longest, two blanks, `rows x columns` (no commas), and
  `; file.dta` (the file's name alone, without folders) for a frame whose data came from, or was saved to, a file. When
  any frame has a star, a blank line and a note that frames marked with a star hold unsaved data. A frame is starred
  when its data changed since they were read or saved, also a new frame with variables and no rows (`frame create
  other id str3 lab`), a frame from `frame put`, one with `set obs`. An empty frame is `0 x 0` with no star. `clear all`
  leaves `default  0 x 0`.
  - RCraft knows the star from what it follows of each frame (`set obs`, `generate`, `label`, `use`, `save`, `clear`;
    `sort`, `summarize`, `list` and the like leave it): where it does not know, `frame dir` is refused.
- **`frame copy from to [, replace]`:** the data as they are, with the file name and the unsaved mark (a copy of an
  unsaved frame is starred and keeps the file name). A frame there already: r(110) without `replace`; with it the frame
  is overwritten, the frame in use included (`frame copy rates default, replace`). A missing source: r(111).
- **`frame rename old new`:** r(111) when `old` is not there, r(110) when `new` is (also `default`).
- **`frame put varlist [if] [in], into(new)`:** a new frame with those variables and the selected rows; r(110) when
  `new` is there.
- **`cwf name`** is `frame change name`; **`frames reset`** drops every frame but `default` and empties it, from any
  frame (the frame in use is `default` after); **`clear all`** does the same.
- **`frame name: command`** and **`frame name { ... }`** (each line in the block) run the command in that frame and leave
  the frame in use as it was; a frame that is not there is r(111). A line that fails in the frame stops there as it would
  in the frame in use.
- **`c(frame)`** is the name of the frame in use (`default`).

## RCraft

- A command in another frame is translated as a change to that frame, the command, a change back (the same R state moves
  as for `frame change`), so `capture` around it works and the frame is always back where it was.
- Refused in a frame prefix: `frame`, `frames`, `cwf`, `pwf`, `clear`, `preserve` and `restore`. Refused: `frame copy`
  or `frames reset` while a `preserve` is pending, a variable list with wildcards in `frame put`, renaming `default`,
  `frame create` with types other than byte, int, long, float, double and str#.
- Not done (probes 233 and 234 still refuse them): `frlink`, `frget`, the function `frval()`, `frame dir` after one of those
  (the star is not known), `describe, short` with no data.
