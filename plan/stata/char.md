# char, note and notes (2026-11-19)

Observed on Stata 19 SE with made-up data (corpus `char.do` and `notes.do`, results exact; the printed lists were read by hand).
StatCraft's note (`data-commands.md`, "notes", ported from statcraft@7fa0429) agrees; the rest was decoded here. Translated in
`crates/statatr/src/data_r/meta.rs`.

## Characteristics

- `char var[name] text` sets, `char _dta[name] text` sets one of the data. The text loses blanks at both ends and a pair of quotes
  round all of it (`"two words"` is stored as `two words`); inner blanks, commas and the words `if` and `in` stay as typed. No text
  removes the characteristic. Names are case-sensitive (`Abc` and `abc` are two).
- A variable name may be abbreviated (an ambiguous one is r(111), `xy ambiguous abbreviation`; one not there is r(111)). A bad name
  (`x[a b]`, `x[]`) is `invalid syntax` r(198), after the variable has been looked for. `char list z` (no brackets) is r(198).
  `char define x[n] text` is the same as without `define`.
- `char list` lists `_dta` first, then each variable in the data's order, one `owner[name]:` line each with the text from column 31.
  `char list x[]` lists one variable, `char list x[n]` one characteristic. Order inside an owner: the one set last comes first
  (a changed characteristic moves up); Stata deals them in another order after `rename` and `restore`, which is not followed.
- Reading: the macro function `` `: char x[n]' `` and the plain macro `` `x[n]' `` (and `_dta[n]`) give the text; empty when there is
  none. They are asked of R while the do-file runs.
- Characteristics follow a variable through `rename`, `drop` of another, `keep`, `sort`, `preserve`/`restore`, and live in a .dta
  file: `save` writes them, `use` reads them.
- Not translated: `char rename`, `char dir`, text with a quote or backtick inside, and `char list` after `tsset`, `xtset`, `stset`,
  `svyset` or `fvset` (Stata lists the characteristics those set, `_TS...` and the like, which R does not hold; their
  macros, for names that start with an underscore, are refused the same way).

## Notes

See `notes.md` (note and notes are characteristics too).

## In R

Characteristics are the attribute `stata_char` on a column (or on the data frame for `_dta`), a named text vector in the order set.
The dplyr verbs keep them; `merge`, `append`, `reshape`, `collapse`, `contract`, `xpose`, `joinby`, `expand`, `expandcl`, `fillin`
and `stack` rebuild the data, so after them the characteristics are marked not known and the commands refuse.
`save` writes the characteristics section of the .dta file and `use` reads it (formats 117 to 119, from Stata's published
description: a length, the owner and the name padded to 33 or 129 bytes, the text and a NUL); the file's section sizes in its map
are made again. A file of an older format gives no characteristics, which the commands then refuse.
