# robvar (2026-11-18)

Observed on Stata 19 SE with made-up data (corpus `robvar.do`: 19 results, 9 digits). StatCraft's note
(`agreement.md`) agrees; the printed output of probe 250 is identical to Stata's (by eye). Translated in
`crates/statatr/src/data_r/descr.rs` (`stata_robvar`). Batch C of the StatCraft port.

- `robvar varname [if] [in], by(varname)`: the `tabulate g, summarize(x)` table (RCraft's `stata_oneway`
  summary), then Levene's W0 (deviations from the group mean), Brown and Forsythe's W50 (from the median)
  and W10 (from the 10% trimmed mean, which drops the integer part of 0.1 n values from each end: R's
  `mean(trim = .1)`), each the one-way F of the absolute deviations with df(k - 1, N - k), `Pr > F`.
- Printing: `W0  =  %.8g   df(a, b)     Pr > F = %.8g`, both numbers cut to 10 columns with their leading zero.
- r(): N, w10, p_w10, w50, p_w50, w0, p_w0, df_1, df_2. One group or constant data: W and p missing
  (`.`), no error; the table still prints.
- Errors: weights r(101), by() missing r(198), two by variables r(103), a string variable r(109), a name the
  data lack r(111), no variable r(100). A sample with no rows is not an error in Stata (an empty table); R
  stops with `no observations`.
- Not translated: more than one variable, `by:`, an empty sample, patterns.
