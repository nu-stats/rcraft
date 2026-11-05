# _natscale's last bits (P2C.12, continued)

Observed on a licensed Stata 19 SE, 2026-11-03, from StatCraft's probes 225 and 226 (4,915 `_natscale`
results printed with `%21x`) and Stata's own arithmetic on typed numbers. Only what Stata did is written here.

- `10^k` in Stata is not always the nearest double for k below 0. `10^-2` is `+1.47ae147ae147cX-007`, one
  unit in the last place above 0.01. `10^-1` is the nearest (`+1.999999999999aX-004`). `10^-3` is
  `+1.0624dd2f1a9fdX-00a`. `10^(k-1)*10` is not always `10^k`.
- `log10(0.1)` is −0.9999999999999999, not −1. `exp(-ln(10))` is `+1.9999999999998X-004`.
- `10^-2*10` is `+1.999999999999bX-004`, one unit above 0.1, and `10^-2*5` is one unit above 0.05.
- Every r(delta) of 0.1 in the sweeps is that value, one unit above 0.1 (n = 10 to 25). Every r(delta) of
  0.05 is one unit above 0.05. r(delta) of 0.2 is the nearest double except in 3 results. So the steps are 2,
  5 and 10 times Stata's 10^k, a step of 1 × 10^z being 10 × 10^(z−1). This fits 4,912 of the 4,915 results.
- With lo = 0, r(min) is 2^-55 or −2^-54 rather than 0. This happens only when the top label lies above the
  data and there are 11 or 12 labels of 0.1, or 9 to 12 labels of 0.2. It does not happen with 13 to 15
  labels of 0.1, or when the top label lies inside the data. r(max) then depends on hi too, by a unit or two
  in the last place, for the same label count. For n = 15 and labels 0 to 1.1: hi = 1.03, 1.06 and 1.09 give
  r(max) `1.1999999999999X+000` and r(min) 2^-55; hi = 1.12 gives `1.199999999999aX+000` and 0.
- Not found: forms tried with Stata's step (L·d, U·d, sums of steps up or down, ends from lo or hi plus or
  minus steps, centres) and a search of about 270,000 two-level expressions in lo, hi, the range, the step,
  the label counts and Stata's 10^k. None gives r(min) and r(max) on every result. The 113 probe lines that
  differ (r(min) 0 against 2^-55) stay as they are. The comparison is not loosened.

## More, 2026-11-04

- Stata's `10^k` for k below 0 is the product of k tenths: `0.1*0.1` is `+1.47ae147ae147cX-007`, its `10^-2`;
  `0.1^3` gives its `10^-3`.
- For lo = 0 and hi = 1.03, the step and the 12 labels are the same for n = 12 to 20, but n = 12, 13, 14 give
  r(min) 0 and r(max) `1.199999999999aX+000` (1.1), while n = 15 to 20 give r(min) 2^-55 and r(max)
  `1.1999999999999X+000` (1.1 less a unit). So the arithmetic of the ends follows a path that depends on n.
- hi from 1.03 to 1.099 gives the residue; 1.02 and below (11 labels) and 1.1 and above (top label inside the
  data) do not. Moving lo by ±1e-20 changes nothing; lo = 1 or 0.5 gives L·step exactly.
- Ruled out besides the earlier forms: r(min) as r(max) less k steps one at a time or at once, as U·step
  less k steps one at a time, as 0 plus k steps; r(max) as r(min) plus k steps one at a time.
- A sweep of n = 8 to 30 against hi = 1.00 to 1.40 (step 0.02), lo = 0: r(min) is 0 for n up to 14; from n = 15
  it is positive for hi = 1.04 to 1.08 and 1.14 to 1.20, a region that moves as n grows (1.24 to 1.40 join
  at n = 20; at n = 24 to 27 some are negative). It looks like a boundary in how the label set is chosen,
  not a simple formula: the next step is to find what decides the path on that boundary.

## Declined, 2026-11-04

- The residue is not only lo = 0 and n of 15 or more: `_natscale 0.04495973312684305 0.6669988136939091 10`
  (labels 0 to 0.7 by 0.1, the data starting above 0) gives r(min) 2^-55 too (probe 225, line 409).
- So a label at 0 with a step below 1 is declined (`data_r/natscale.rs`), whatever the data and n: no
  narrower rule holds on what has been seen. Steps of 1 or more are exact in Stata as well.
- How often: in probe 225's 480 varied ranges the residue shows once (0.2%) and the rule declines 30 (6%);
  in probe 226's sweeps, built around 0, the residue shows in 112 of 4,915 (2.3%) and the rule declines
  1,479. None of the 46 packages calls `_natscale`; RCraft's own graphs choose their labels without it.
- The probes then match on every compared `_natscale` line; the declined ones count as not translated.
