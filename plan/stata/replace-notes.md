# replace's and generate's notes, tsset's Delta (2026-11-13)

Observed on Stata 19 SE (probe `r7/f1.do` in the session scratchpad, identical to RCraft's output). In `data_r.rs`
(CHANGES_HELPER) and `ts.rs`.

- replace: `(7 real changes made)`, `(1 real change made)`, `(0 real changes made)`; for numbers the changes to a
  missing value too, `(2 real changes made, 2 to missing)` (`.` to `.a` is a change, and counts as to missing);
  strings never say "to missing". Counted inside the mutate, a group at a time, so `bysort s: replace` counts the
  same as Stata. Nothing under quietly or capture (capture noisily prints).
- generate: `(6 missing values generated)` (`(1 missing value generated)`), strings' empty values counted, nothing
  when none.
- tsset: `        Delta: 1 unit`, the delta in %9.0g trimmed (it was padded to 10).
