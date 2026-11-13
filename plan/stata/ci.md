# ci means, ci proportions, ci variances (2026-11-13)

Observed on Stata 19 SE with made-up data (probes `ci/c1.do`, `c2.do` in the session scratchpad, their logs set
line by line against RCraft's output: identical but for the refused forms); corpus `ci.do`. Translated in
`crates/statatr/src/data_r/ci.rs`.

- `ci` alone or with another word: "you must specify one of means, proportions, or variances following ci",
  r(198).
- The table: `    Variable |        Obs        Mean    Std. err.       [95% conf. interval]` (Proportion for
  proportions; `Obs      Variance` and no Std. err. for variances), a rule of 13 + 63 (52) dashes, Obs in
  11, the values in %9.0g in 12, 12, 16, 12 (variances 14, 15, 12); names cut to 12 as `averyveryv~e`; the
  level as `[99.5% conf. interval]`, right-aligned; a rule after every fifth variable. r() is the last
  variable's: N, mean (proportion, Var), se (not for variances), lb, ub, level, citype ("normal" or the
  method).
- Means: mean ± t(N - 1)·sd/√N. Frequency weights count rows that many times (N their sum); analytic
  weights as summarize takes them (N the rows). One observation: se, lb and ub missing; none: Obs 0 and the
  rest missing. A string variable is left out silently.
- Proportions, only of 0/1 variables (an all-0 or all-1 variable counts): the others are left out with
  "Note: The results are produced only for binary (0/1) variables." after the table; none left: "no binary
  (0/1) variables found; nothing to compute", and _rc is 0. Above the table the method, centred over the
  interval's columns: Binomial exact (the default), Binomial Wald, Wilson, Agresti–Coull, Jeffreys.
  - exact: Clopper-Pearson; at 0 or n successes one-sided, the bound marked `*` and "(*) one-sided, 97.5%
    confidence interval" ((100 + level)/2).
  - wald: p ± z·se; a lower bound at or below 0 is 0 and marked `*`, an upper one at or above 1 is 1 and
    marked `**`; "(*) The Wald interval was clipped at the lower endpoint", "(**) ... upper endpoint".
  - wilson: clipped (`*`) only at 0 successes, as Stata 19 marks it; agresti: Agresti–Coull, clipped below
    0 (`*`) or above 1 (`**`) with the same notes; jeffreys: qbeta(α/2, k + .5, n − k + .5), 0 at 0
    successes and 1 at n, unmarked.
- Variances: (N − 1)s² over the chi2 quantiles.
- Not translated: `cii`, weights for proportions and variances and with `poisson`.

## poisson, exposure(), bonett, sd (2026-11-13; probes `ci/c3.do`, `c4.do` identical; Stata's manual, [R] ci)

- `ci means x, poisson [exposure(e)]` ("Poisson exact"): an Exposure column (the rows, or the sum of e) in
  place of Obs; the total of x **rounded to a whole number** (half up: 0.4 ten times is 4, 2.25 ten times 23)
  is k; mean k/E, se √k/E; the exact interval (gammap quantiles: G⁻¹(γ; k), G⁻¹(1 − γ; k + 1)) over E; at k = 0
  the lower bound 0 and the upper one-sided, marked `*` with "(*) one-sided, 97.5% confidence interval".
  r(N) is the exposure; r(citype) "poisson", r(exposure) the variable's name.
- `ci variances x, bonett` ("Bonett"): the trimmed mean m with trim 1/(2√(n − 4)) from each end, the kurtosis
  γ4 = n·Σ(x − m)⁴/(Σ(x − x̄)²)², c = n/(n − z), se = c·√((γ4 − (n − 3)/n)/(n − 1)), the interval
  exp(ln(c·s²) ± z·se). r() adds kurtosis after Var.
- `sd`: "Std. dev." in place of Variance, the square roots of the estimate and bounds; r() adds sd after Var.
