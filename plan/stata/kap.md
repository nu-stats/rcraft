# kap (2026-11-16)

Observed on Stata 19 SE with made-up data (StatCraft probe 251 and probes `kap/k1`, `k2`, `kv` in the session
scratchpad: identical to RCraft's output but r() digits noted below); corpus `kap.do`. Translated in
`crates/statatr/src/data_r/kap.rs`.

- Two raters: the ratings are the values either rater used (sorted); po = Σ wᵢⱼ pᵢⱼ, pe = Σ wᵢⱼ pᵢ. p.ⱼ,
  κ = (po − pe)/(1 − pe); unweighted, Stata works from the counts (κ = (Σnᵢᵢ − Σnᵢ.n.ᵢ/N)/(N − Σnᵢ.n.ᵢ/N)),
  which is its last digit with no missing values; with a rating missing, r() can differ in the 16th digit.
- wgt(w): 1 − |i − j|/(k − 1); wgt(w2): 1 − ((i − j)/(k − 1))², on the ratings' ranks, or with absolute on their
  values (span max − min). Printed under `Ratings weighted by:` (%9.4f).
- SE under no agreement beyond chance (Fleiss, Cohen and Everitt): √(Σ pᵢ.p.ⱼ(wᵢⱼ − (w̄ᵢ. + w̄.ⱼ))² − pe²) /
  ((1 − pe)√N); Z = κ/SE, Prob>Z one-sided. fweights count rows. r(): se, z, kappa, prop_e, prop_o, N.
- The line: `%7.2f%%%11.2f%%%11.4f%11.4f%11.2f%12.4f` under `             Expected` / `Agreement   agreement ...`.
  tab: tabulate's two-way table of the values (value labels are not used), then the rest with no blank line.
- Three or more raters (each variable a rater): Fleiss's κⱼ = 1 − Σᵢxᵢⱼ(n − xᵢⱼ)/(Nn(n − 1)pⱼqⱼ), combined
  κ = 1 − (Nn² − Σxᵢⱼ²)/(Nn(n − 1)Σpⱼqⱼ), SE(κⱼ) = √(2/(Nn(n − 1))), SE(κ) = √2/(Σpq √(Nn(n−1))) ·
  √((Σpq)² − Σpq(q − p)); `There are n raters per subject:` and the Outcome table. r(): z, kappa.
- Errors: one variable `too few variables specified` (r(102)); tab with three or more `option tab not allowed with 3
  or more raters`, wgt() there `option opts not allowed` (r(198)).
- Refused: kapwgt and its named weights; raters per subject that vary (Stata prints kappas with dots and a note,
  by a formula not matched); a chance agreement that leaves no test (Stata's `.`s, its rounding); tab with more
  than five ratings (its table is not split as tabulate's at linesize 80); weights with three or more raters;
  string ratings.
