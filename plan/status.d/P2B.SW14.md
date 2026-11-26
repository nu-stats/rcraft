## STATUS

## P2B.SW14: a factor beside a continuous variable, 2026-11-26

- `regress y z i.g#c.x` and `i.g##c.x` print Stata's layout for a factor or text column made in R (the group `g#c.x`, levels named by their text, the base level of `##` left out), as they already did for numbers; and a labelled number from a .dta now shows its value labels in the `g#c.x` group, as Stata 19 does (observed on made-up data: label text in both groups, no base row in `g#c.x` alone). The coefficient names for such a column number the levels in order (`1.g#c.x`), the same as SW10's. Real-R test in `crates/engine/tests/stata_regress.rs`.
