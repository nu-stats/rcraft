# compare (2026-11-19)

Observed on Stata 19 SE with made-up data (corpus `compare.do`: r(N) and the errors exact; the tables were read against Stata's
log line by line, for numbers, strings, missing values and a float against a double). StatCraft's probes 520 and 522 name it.
Translated in `crates/statatr/src/data_r/meta.rs`.

- `compare var1 var2 [if] [in]` counts the rows where `var1 < var2`, `=` and `>`, then `Jointly defined`, the rows missing in one
  only (`var1 missing only`), `Jointly missing` and `Total`. A row with a count of 0 is left out, and so is `Jointly defined` when
  it is 0; the dashed separator before `Total` stays. The three number columns (minimum, average, maximum of var1 - var2) are in
  `%9.0g`, shown for the `<` and `>` rows and `Jointly defined`; the `=` row has a count only.
- Missing is `.` to `.z` for numbers and `""` for strings. Equality is exact (a float 1.1 and a double 1.1 differ by 2.38e-08).
- Strings compare in byte order and have a count-only table headed `count`.
- r(N) is the number of rows in the sample, also when 0. Nothing else is returned.
- Errors: one variable is `too few variables specified` r(102), three `too many variables specified` r(103), a weight is
  r(101) (`weights not allowed`), a variable that is not there r(111).
- Not translated: a number against a string (Stata shows only the counts of what is defined, by a rule not decoded; R stops),
  options, and names that are not plain variable names.
