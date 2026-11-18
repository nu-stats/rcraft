# pcorr (2026-11-18)

Observed on Stata 19 SE with made-up data (corpus `pcorr.do`: 27 results, exact to 8 digits). StatCraft's
note (`data-display.md`, probes 520 and 522) agrees; the printed table was also compared with Stata's log by
eye (probe 520: identical). Translated in `crates/statatr/src/data_r/descr.rs` (`stata_pcorr`). Batch C of the
StatCraft port. (`power pcorr` is a different command, done earlier.)

- `pcorr y x1 x2 ...` fits y on all the others over the rows where every variable is present; with t the
  coefficient's t statistic and df the residual degrees of freedom: partial = t / sqrt(t^2 + df), semipartial
  = t sqrt((1 - R2) / df), significance the coefficient's two-sided p. The table lists the squares too.
- Weights: f and a. fweights count N as their sum (r(N) 114, df 111 where the rows are 39 and 36 at weights 1);
  aweights keep N as the rows, are scaled away. p and i r(101), non-whole fweight r(401), negative r(402).
- r(): scalars r(df), r(N); matrices r(sp_corr) and r(p_corr) (k x 1, rows the regressors).
- A variable listed twice counts once (`pcorr y x x` is two variables). Errors: none listed r(100); fewer than
  three r(102); a name the data lack r(111); no rows, or a string variable (its values count as missing), r(2000).
- Not translated: a regressor collinear with the others. Stata drops the *first* of the collinear variables,
  prints `(dropped)` in its row and reduces df; the shape of the two matrices then is not known, so R stops
  with a message. Also not translated: `by:`, a pattern or time-series operator in the varlist.
