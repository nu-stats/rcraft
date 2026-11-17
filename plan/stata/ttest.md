# ttest and ttesti (P2C.12, 2026-11-17)

Observed in Stata 19 on made-up data (probes w1–w3 in the session scratchpad); translated in
`crates/statatr/src/data_r/ttest.rs` (the table) and `returned.rs`'s `stata_ttest_r` (r()). Before this, ttest
printed R's `t.test()` and refused welch; ttesti was not translated.

- Titles: `One-sample t test`, `Paired t test`, `Two-sample t test with equal variances` / `with unequal
  variances` (unequal and welch alike).
- Table: 78 wide. Header `%8s |` (`Variable`, `   Group` with by(), blank for ttesti) then `     Obs`,
  `        Mean`, `    Std. err.`, `   Std. dev.`, `   [95% conf. interval]` (`%8s %12s %13s %12s %23s`). Rows
  `%8s |%8s` and five `%12s` of `%9.0g` values; names cut to 8 (`a_long~e`), by() groups by their value label (or
  value) cut to 8 and trimmed (`treated group` → `treated`); ttesti's rows `x` and `y`. Two samples: the
  groups, a line, `Combined`, a line, `diff` (no Obs or Std. dev.). Paired: the two variables, a line, `diff`
  with Obs and Std. dev.
- Below: `    mean = mean(a)` / `     mean(diff) = mean(a - b)` / `    diff = mean(control) - mean(treated)`
  (full names, labels cut to 8) with `t = %8.4f` ending at column 78; `H0: ... = 0` (paired one space in)
  with `Degrees of freedom = %8.0g` (`Satterthwaite's degrees of freedom` with unequal, `Welch's degrees of
  freedom` with welch) ending at 78; the three `Ha:` labels each centred over its p-value (centres at 10.5,
  39.5 and 69.5, starting at ⌈c + 0.5 − len/2⌉); ` Pr(T < t) = %6.4f         Pr(|T| > |t|) = %6.4f          Pr(T >
  t) = %6.4f`. The null is shown in %9.0g (`.5`).
- welch: df = (v₁ + v₂)² / (v₁²/(n₁ + 1) + v₂²/(n₂ + 1)) − 2, v = s²/n (Welch 1947); welch implies unequal.
- ttesti two samples: Combined is the pooled mean and the standard deviation of both groups together,
  √(((n₁ − 1)s₁² + (n₂ − 1)s₂² + n₁n₂/N (m₁ − m₂)²)/(N − 1)).
- Errors (r(198)): `welch option invalid for one-sample or paired tests`, `unequal option invalid for
  one-sample or paired tests` (unequal named first when both are given); by() with one group `1 group
  found, 2 required`, more `more than 2 groups found, only 2 allowed` (r(420)).
- r(): as before (`stata_ttest_r`), df_t Welch's with welch; they can differ from Stata's in the 16th digit.
