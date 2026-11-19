# note and notes (2026-11-19)

Observed on Stata 19 SE with made-up data (corpus `notes.do`, results exact; the printed lists were read by hand). StatCraft's note
(`data-commands.md`, "notes", ported from statcraft@7fa0429) agrees; the rest was decoded here. Notes are characteristics
(`char.md`), kept in `crates/statatr/src/data_r/meta.rs`.


- `note: text` (or `notes: text`) adds to the data, `note var: text` to a variable. The count is kept in `note0`, each note in
  `note1`, `note2`, ...; adding sets `note0` and then the new note. A note with no text says `(no note added)`.
- `notes drop` takes `_dta`, `_all` or variables and `in #`, `#/#` or a list: `(1 note dropped)`, `(3 notes dropped)`, `(0 notes dropped)`
  when none is there (no error). `note0` becomes the highest number left (and goes when none is left), so dropping the top note makes
  the next one reuse its number, and dropping a lower one leaves a gap.
- `notes replace var in # : text` says `(note # for var replaced)`, `(no note replaced)` when that number is not there.
- `notes renumber` makes the numbers 1 to n again. `notes search text` lists the notes holding the text (case-sensitive, one
  word), or says `text: not found`.
- `notes`, `notes list`, `notes x y`, `notes x in 2` list `_dta` then variables as a blank line, `name:` and `  1.  text`.
- Errors: a variable that is not there is r(111) (`notes x`, `note x: text`, `notes drop x`). `note x in 1: text` is r(198).
- Not translated: a note holding the word `TS` (Stata puts the date and time there; the stamp is the time the line runs, so
  it is not repeatable), quotes in a note's text, `notes search` with several words.

