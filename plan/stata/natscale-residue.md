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
