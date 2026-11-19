# ds (2026-11-19)

Observed on Stata 19 SE with made-up data (corpus `ds.do`, r(varlist) exact; the columns it prints read against Stata's log, also
with 14 long names). StatCraft's `data-commands.md` (ported from statcraft@7fa0429) has the first rules. Translated in
`crates/statatr/src/data_r/meta.rs`.

- `ds [varlist]` prints the names in columns as `describe, simple` does (each name shortened to 12 with a `~`, the column two wider
  than the longest, filled down) and leaves them in r(varlist), full names, separated by blanks. Nothing found prints nothing and
  leaves no r(varlist).
- `ds varlist, not` is the variables the varlist leaves out, in the data's order. `alpha` sorts by name. A name given twice is
  listed twice; a name that is not there is r(111).
- `has(spec)` keeps and `not(spec)` drops the variables with a property (`has()` and `not()` together are r(198), `may not combine
  options has() and not()`): `type string`, `type numeric` (both words keep every variable), `varlabel` (has a variable label),
  `varlabel pattern` (the whole label matches: `*` any text, `?` one character, anything else as it stands, capitals matter; the
  pattern needs no quotes without blanks) and `vallab` (has a value label). Stata also takes `char name`, `format pattern`, `vallab pattern`
  and the storage types (`type double`). `varl` and `vall` are accepted for the first words.
- Not translated: the storage types, `format`, `char`, `vallab pattern` (R does not keep Stata's types or formats, nor the name of
  a value label set that came with a file) and `detail`, `varwidth()`, `skip()`.
