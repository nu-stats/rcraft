# mean, total, proportion, ratio (2026-11-13)

Observed on Stata 19 SE with made-up data (probes `r4/c1.do` to `r4/c3.do` in the session scratchpad, set line by line
against RCraft's output: identical; the variances checked to the last digits on data exported at 17 digits);
corpus `meanest.do` (25 results, 1e-9). Translated in `crates/statatr/src/data_r/meanest.rs`.

- The rows: every variable present (and over()'s, and the weight). `Number of obs` is the rows, or the
  frequency weights' sum; the header is 62 wide, the count with commas at the right (`Number of obs = 12,345`).
- mean: s²/n of each mean (covariances alike), frequency weights counting copies (W − 1, W), analytic ones
  scaled to the rows, and with over() each group on its own (its own rows; analytic weights scaled to the
  group's rows; nothing across groups). pweights: n/(n−1) Σ zᵢzᵢ′ with zᵢ = wᵢ(yᵢ − ȳ)/Σw, n all the rows
  even with over(). e(vce) analytic, robust with pweights; no Linearized line.
- total: Σw times the mean, its variance (Σw)² times the mean's (so with pweights not the linearized total's).
- proportion: each value's share, V = (diag(p) − pp′)/n (n, not n − 1), across two variables the indicators'
  covariance over n; pweights linearized as mean's; logit intervals (logit p ± t·se/(p(1 − p)), t on n − 1 df),
  the `Logit` line over the heading. Values must be nonnegative integers.
- ratio: Σwy/Σwx, linearized, n/(n−1) Σ zᵢ² with zᵢ = wᵢ(yᵢ − Rxᵢ)/Σwx, with over() each group's own n
  (unlike mean's), `Linearized` and `std. err.` in the heading, the names `_ratio_#` or the ones given, each
  `name: y/x` over the table.
- Names: y; with over() c.y@0bn.b, c.y@1.b (the first level bn); proportion 1bn.g 2.g, 0bn.b@1bn.g; matrix list
  shows c.y@ over 0.b, 1. over g. The table: the variable (or over()'s `c.y@b`, `b@g`) heads its levels'
  rows (`%11s  |`), a blank `|` line between variables.
- e(): df_r (the rows less 1, or W − 1), N_over, N, k_eq, rank; cmdline, cmd, vce, (vcetype), title, (wexp
  "= wt", wtype), (namelist), estat_cmd estat_vce_only, varlist (proportion: 1.g 2.g ...), marginsnotok _ALL,
  (over), properties; b, V, sd (mean: the weights scaled to the rows), freq (proportion: the weight sums),
  _N (the rows of each group), error (0s). ratio with frequency weights lists N before N_over. A 1 × 1 _N's
  row is named as its column. r(table) as the other estimation commands'.
- test and lincom read them: lincom's stub is Mean, Total, Proportion or Ratio, and the lines name e(b)'s own
  levels (`1bn.g - 2.g = 0`, `c.y@0bn.b - c.y@1.b = 0`).
- Not yet: several over() variables, vce(), the display options, iweights, analytic weights with total,
  proportion and ratio, `prop` and the other abbreviations of proportion, `_b[c.y@0.b]` (the expression reader
  does not take @ in a name).
