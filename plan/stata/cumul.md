# cumul (2026-11-18)

Observed on Stata 19 SE with made-up data (corpus `cumul.do`: 169 results, exact). StatCraft's note
(`data-display.md`, probes 520 and 522) agrees. Translated in `crates/statatr/src/data_r/descr.rs`
(`stata_cumul`). Batch C of the StatCraft port.

- `cumul varname [if] [in] [weight], generate(newvar) [freq by(varlist) equal]`. The new variable is a float
  (`%9.0g`) labelled `ECDF of x`: the running weight over the group's total, or the running weight itself with
  `freq`. Rows outside the sample, with x missing, or with a missing or zero weight get missing.
- Weights: f and a. pweights and iweights r(101); a negative weight r(402); a non-whole fweight r(401).
  aweights are not scaled (freq shows 3.3, 1.8, ... the running sums as they are).
- The running weight is kept in a float: the sums of weights are rounded to float once (not at each step), and so
  is the total, and the ratio is rounded again; this reproduces Stata's last digits with non-whole aweights.
- `by()`: a group per combination of values; a missing value (`.`) and `""` are groups of their own. A row
  whose by-variable is missing is still counted in its own group.
- `equal`: tied values all get the running weight after the last of them.
- A string variable is not an error: it makes a variable of all missing (as does a sample with no rows).
- r() is left as it was (checked: `summarize`, `cumul`, `return list`).
- Errors (return codes compared; messages by eye): no varlist r(100), two variables r(103), no generate()
  r(198), a bad or existing name r(198) or r(110), a name the data lack r(111), a repeated option r(198).
- **Tied values without `equal`:** Stata orders them by its own sort, which is arbitrary (checked: the order is
  neither the data order nor its reverse, 300 rows with 4 values show no pattern). RCraft gives them the order
  of the rows. What does not depend on the order (the values, their mean, minimum and maximum, every untied row)
  matches; which tied row gets which of the tied values may not. The corpus checks only the first kind.
- Not translated: `by:`, a pattern as the varlist, by() with a pattern or factor variable.
