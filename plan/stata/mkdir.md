# mkdir and rmdir (2026-11-18)

Observed on Stata 19 SE with made-up folders; corpus `mkdir.do` (every result is a return code, so
the match is exact). Translated in `crates/statatr/src/data_r/outfiles.rs`. StatCraft's note had
"already exists" for a folder that is there; Stata 19 says `could not create directory NAME`.

- `mkdir folder [, public]`: one folder, in the folder the do-file runs from. The parent must be there:
  `mkdir a/b/c` with no `a` is not made (r(693)), `mkdir a` twice is r(693) the second time, a file
  with the name is r(693), a trailing slash on a folder that is there is r(693). Quotes around a name
  with blanks work. `public` is accepted and means nothing here. The message is
  `could not create directory NAME`, with the name as typed.
- No name: `invalid syntax`, r(198). Two names: `varlist not allowed`, r(101).
- `rmdir folder`: removes an empty folder (r(0)); one with something in it, one that is not there and a
  file are `could not remove directory NAME`, r(693); no name is `invalid syntax` r(198).
