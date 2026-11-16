# cs, csi, cc, cci, ir, iri (2026-11-14)

Observed on Stata 19 SE with made-up data (probes `r17/e1.do` to `r17/e3.do` in the session scratchpad, set line by
line against RCraft's output, `return list` included: identical but for r() digits past the 12th); corpus
`epitab.do`. Translated in `crates/statatr/src/data_r/epitab.rs`.

- Counts: a nonzero case or exposure value counts as one (an exposure of 0, 1, 2 puts 1 and 2 together as
  exposed); rows with any of the variables missing are left out.
- cs: risk difference ± z·√(p₁q₁/n₁ + p₀q₀/n₀); risk ratio exp(log RR ± z·√(1/a − 1/n₁ + 1/b − 1/n₀));
  attributable fraction among the exposed (RR − 1)/RR with the interval from the RR's ends, in the population
  (p − p₀)/p. Below 1 the rows read "Prev. frac. ex." 1 − RR (its interval 1 − the RR's ends, swapped) and
  "Prev. frac. pop" (p₀ − p)/p₀, still kept as r(afe) and r(afp). Pearson's chi2 (no correction); with exact,
  Fisher's two-sided p and the one-sided one in the direction of the data, in place of chi2.
- cc (woolf or tb): odds ratio ad/bc; Woolf: exp(log OR ± z·√(1/a + 1/b + 1/c + 1/d)); tb: OR^(1 ± z/√χ²) with
  the Mantel–Haenszel χ² = (N − 1)/N Pearson's (found by matching `cci 30 20 50 100, tb`); attributable
  fraction (OR − 1)/OR, in the population that times the proportion of cases exposed (prevented: (1 − OR) times
  the proportion of controls exposed).
- ir: rate difference ± z·√(a/t₁² + b/t₀²); the rate ratio's exact interval is R's `poisson.test` (the same
  as Stata's to 1e-15); mid-p and exact p from the binomial of the exposed cases among all, p = t₁/(t₁ + t₀);
  two-sided 2·min. Rows named by the case and time variables' labels (else their names); the mid-p lines
  are aligned on the `=`.
- Layout: `%16s |%10s%12s  |%11s`, estimates `%17s       |%12s%12s %s` with %9.0g values (a trailing space
  when no method is named), the exposure's label (else its name) centred over the 24 columns (the immediate
  forms have no such row); cc's `Proportion` / `exposed` column `%13.4f`.
- Refused: cc's and cci's default exact interval of the odds ratio and cs's `or` (Cornfield): both come from
  Stata's own searches, whose bounds differ from the exact roots by about 1e-4 (the tail probabilities at
  Stata's bounds are .02495 to .02510, not .025), so they cannot be printed as Stata does; by() and the
  stratified options, weights.

## replace keeps the variable label

Found here: `replace` dropped a variable's label (ir then named its row by the variable's name). The change
counter now carries the label and the display format over, as Stata does.

## by(), and tabodds (2026-11-15)

Observed on Stata 19 SE with made-up data (probes `r19/e1.do`, `r19/e2.do`, `r19/t1.do` in the session scratchpad and
StatCraft's probe 314, set line by line against RCraft's output, `return list` included: identical but for r() digits
past the 14th); corpus `epitab_by.do`.

- Strata: rows with the stratifying variable missing are left out; each stratum named by its value label (cut to 16),
  else its value; the variable's name heads the column.
- cs, by(): each stratum's risk ratio with the log interval and the Mantel–Haenszel weight b·n₁/N; the crude ratio;
  the M–H combined ratio Σ(a·n₀/N)/Σ(b·n₁/N) with Greenland and Robins's (1985) variance; the test of homogeneity
  Σ(log RRᵢ − log RR_MH)²/vᵢ. Stata keeps that chi2 as a float, from the strata's ratios and variances as floats:
  float(Σ(log(float RRᵢ) − log RR_MH)²/float vᵢ) is Stata's to the last digit for cs, cc and ir. istandard: the
  exposed as the standard, Σa/Σ(n₁·b/n₀), the variance Σn₁p₁q₁/O² + Σn₁²p₀q₀/n₀/E², the weights n₁, no test. or
  woolf: Woolf's intervals, the combined odds ratio Σ(ad/N)/Σ(bc/N) with Robins, Breslow and Greenland's interval,
  the M–H test (Σ(a − E))²/ΣV. afp is the crude one.
- cc, by() with woolf or tb: as cs's or woolf, the crude row tagged; tb's intervals on each table's (N − 1)/N
  Pearson chi2, the combined one's on the M–H chi2. r(chi2_p) is the homogeneity chi2.
- ir, by(): each stratum's exact interval (poisson.test), the M–H rate ratio Σ(a·t₀/T)/Σ(b·t₁/T) with Greenland and
  Robins's variance, the pooled (inverse-variance on log IRR) one in r(), the test of homogeneity printed
  `chi2(k) = %-12.2f`.
- tabodds: cases, controls, odds and Woolf's interval of each level (labels abbreviated to 9); the test of equal
  odds, (N − 1)/N the k × 2 Pearson chi2; the score test for trend on the levels' values. R's printf on Windows rounds
  0.66500000000000004 (the double of .665) down at two places, Stata up: the tests' lines are rounded from the
  double's exact digits.
- tabodds, or (2026-11-16): each level against the base() level (its place, 1 to k, not its value) as mhodds,
  compare(level, base) gives it: the Mantel–Haenszel odds ratio, chi2 (U²/V) and exp(ln OR ± z√(V/(R·S)));
  table `%12s |` `%12.6f %11.2f %13.4f %14.6f %11.6f`, the base row `1.000000` with `.` (a missing p in 12, the
  lower bound in 15), the name abbreviated to 12, labels to 9; the tests below and r() unchanged. Errors: `option
  base() invalid; must be between 1 and k` (checked first), `option base() only valid with option or` (r(198)),
  `'1.5' found where integer expected` / `error in option base()` (r(7)); fweights count, aweights, iweights and
  pweights `... not allowed` (r(101)).
- Stata's %w.df rounds the double's exact digits with an exact tie to even (88.125 → 88.12, 0.125 → 0.12,
  0.75 → 0.8, 1.4453125 → 1.445312); kap's 88.13 is a double above 88.125, which Stata gets by adding the
  weighted cells one by one along the rows (R's sum() keeps long doubles and lands on 88.125).
- tabodds, adjust(varlist) (2026-11-16): implies or; `Mantel–Haenszel odds ratios adjusted for a, b and c` (no
  Oxford comma, never wrapped) above the same table, each level against the base across the strata as mhodds,
  compare() with the controls gives it; then only `Score test for trend of odds: chi2(1)  =` `%8.2f` and 30
  spaces `Pr>chi2  =` `%8.4f` (mhodds's stratified trend test); r() chi2_tr and p_trend alone. A missing value in
  an adjusting variable drops the record. A level never sharing a stratum with the base prints all `.`.
- tabodds, binomial(n): case is the cases of n trials on each record (controls n − case); weights with it:
  `error with option binomial()` / `    weights are not allowed with binomial frequency records` (r(198)).
  Stata takes nonsense counts as they come (cases above n give negative controls, a missing n counts the
  cases only); those are refused here.
- Refused: cc's exact intervals with by() too; cs, by() or without woolf (Cornfield); estandard, ustandard();
  tabodds's graphs (graph lane); binomial() counts that are missing, negative, not whole or above the trials.
