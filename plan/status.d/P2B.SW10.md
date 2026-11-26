## STATUS

## P2B.SW10: i.g on a factor or text column made in R, 2026-11-26

- `regress y x i.g` stopped with "subscript out of bounds" when `g` was a factor or text column made in R (data not opened with `use` or File > Use my file). Two causes: a level named `c` came out as `c.g`, which the name canonicaliser reads as the continuous prefix and drops, and the coefficient-row layout (`stata_coef_rows`) only knew numeric levels and indexed its keys by name. Levels of such a column are now named by their place among the levels (`2.g`, `3.g`), labelled by their text as a labelled variable's are, the first level left out; `ib2.g` takes the 2nd level as the base. The layout code no longer indexes by name. Real-R test in `crates/engine/tests/stata_regress.rs` (factor, text column, `ib2.g`; numbers equal lm's).
- Not done: an interaction with a factor made in R (`i.g#c.x`) prints in R's layout, as an interaction with a continuous variable did not have its layout checked for numeric factors either.
