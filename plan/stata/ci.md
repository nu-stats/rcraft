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

## Weights, cii (2026-11-13; probes `rest/r1.do`, `r2.do` identical but for Stata's errors)

- Frequency weights in `ci proportions`, `ci variances` and `ci means, poisson` (the counts, the exposure and
  the variance's n are the weight sums); analytic weights there are Stata's error "aweights not allowed",
  r(101). `centile` takes no weights at all: "weights not allowed", r(101).
- `cii means #obs #mean #sd`, `cii means #exposure #events, poisson`, `cii proportions #obs #succ [, method]`,
  `cii variances #obs #variance [, sd]` (with sd the second number is the sd) and `cii variances #obs
  #variance #kurtosis, bonett`: ci's table with an empty Variable column and ci's r() (bonett: r(kurtosis) is
  the number typed). With one observation the se is sd/√1 and the bounds missing.
- cii's notes where a bound went past 0 or 1, in place of ci's marks: "Note: The Wald CI was truncated at 0 at
  the lower endpoint.", "... truncated at 1 at the upper endpoint." — Wald and Agresti–Coull when the bound
  crosses (at 0 successes Wald's lower bound is 0 exactly: no note), Wilson's lower at 0 successes;
  exact and Poisson keep the `*` and "(*) one-sided" line.
- Stata's errors (refused with them): #obs not a positive integer (r(198)), more successes than
  observations (r(198)), bonett without the kurtosis (r(198)). A proportion or rate typed in place of the
  count ("or between 0 and 1"): see below.

## A proportion or rate typed to cii (2026-11-13; probe `last/q1.do` identical; corpus `ci.do`)

- `cii proportions #obs #p` with #p strictly between 0 and 1 is the count floor(#p · #obs + .5) (60 .31 is
  19, 7 .5 is 4), and the table and r() are that count's; `cii means #exposure #rate, poisson` likewise
  takes floor(#rate · #exposure + .5) events. Otherwise a fraction is Stata's r(7): "second argument,
  #succ, must be a nonnegative integer or between 0 and 1" (#events: "a positive integer").
