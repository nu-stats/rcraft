# sktest (2026-11-13)

The formulas are those of Stata's public manual, [R] sktest, Methods and formulas (D'Agostino, Belanger and
D'Agostino 1990, with Royston's 1991c adjustment); the layout and r() were observed on Stata 19 SE with made-up
data (probe `sk/s1.do` in the session scratchpad: 63 lines of values to 7e-14, the tables identical); corpus
`sktest.do` (60 results to 1e-9). Translated in `crates/statatr/src/data_r/sktest.rs`.

- From summarize's skewness √b1 and kurtosis b2 (weighted ones with weights) and n (the sum of frequency
  weights, else the rows):
  - skewness: Y = √b1·√((n+1)(n+3)/(6(n−2))); β2 = 3(n² + 27n − 70)(n+1)(n+3)/((n−2)(n+5)(n+7)(n+9));
    W² = −1 + √(2(β2 − 1)); δ = 1/√(ln W); α = √(2/(W² − 1)); Z1 = δ·ln(Y/α + √((Y/α)² + 1)).
  - kurtosis: E = 3(n−1)/(n+1); var = 24n(n−2)(n−3)/((n+1)²(n+3)(n+5)); x = (b2 − E)/√var;
    √β1 = 6(n² − 5n + 2)/((n+7)(n+9))·√(6(n+3)(n+5)/(n(n−2)(n−3))); A = 6 + (8/√β1)(2/√β1 + √(1 + 4/β1));
    Z2 = (1 − 2/(9A) − ((1 − 2/A)/(1 + x√(2/(A−4))))^(1/3)) / √(2/(9A)), the cube root keeping its sign.
  - K² = Z1² + Z2². Royston's adjustment (the default): Zc = −Φ⁻¹(exp(−K²/2)); ω = 0.55n^0.2 − 0.21;
    a1 = (−5 + 3.46 ln n)exp(−1.37 ln n); b1 = 1 + (0.854 − 0.148 ln n)exp(−0.55 ln n);
    b2 = 2.13/(1 − 2.37 ln n) + b1; a2 = a1 − (2.13/(1 − 2.37 ln n))ω (so the two pieces meet at ω);
    Z = Zc below −1, a1 + b1·Zc below ω, else a2 + b2·Zc; P = Φ(−Z); adjusted chi2 = −2 ln P. `noadjust`: K².
  - Pr(skewness) and Pr(kurtosis) two-sided normal; Prob>chi2 from chi2(2).
- The table: "Skewness and kurtosis tests for normality", "----- Joint test -----" (`--- Joint test ---` with
  noadjust) over the last two columns; Obs with commas in 10, the p-values %.4f in 15, Adj chi2(2) %.2f in 14
  (chi2(2) in 10), Prob>chi2 in 11. The names' column is 12 at the default line size, as wide as the longest
  name when the line has room (min(longest, linesize − 67)), names cut as `avery_long~e`.
- Fewer than 8 observations: every test missing ("." in columns 14, 15, 15 and 11 wide). A chi2 too large for
  the adjustment (exp(−K²/2) underflows) is "." with its p-value.
- r(): the last variable's N, p_skew, p_kurt, chi2, p_chi2, and r(table) (one row a variable: N p_skew
  p_kurt chi2 p_chi2).
