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

## Found, 2026-11-15: the residue is trimming from a wider label set

- Read with scalars, not locals: a local keeps a value as text and loses the last bits (an earlier
  probe of ours printed 1.fffffffffdb59X-038 for 2^-55 that way).
- On 88 cases (lo = 0; hi 1.03, 1.05, 1.08, 1.1, 1.12, 0.7, 0.53, 2.06; n = 10 to 20), every r(min) and
  r(max) is reproduced bit for bit by: start from labels L0·d and U0·d (j·d, with Stata's d, e.g.
  0.1 + 1 ulp), then step inward one d at a time (min + d, max - d, each rounded) to the final L·d and
  U·d. The residue is that arithmetic: (-3·d) + d + d + d is 2^-55, not 0. Each end can start at a
  range of L0 (or U0) that give the same bits; the sets per case are in the session's
  scratchpad (nat/h2.R output).
- What is left: how L0 and U0 follow from lo, hi and n. They jump at n = 15 for 0 to 1.03-1.08
  (U0 from 12..14 to 15..17, L0 from 0..-2 to -3..-6), at 14 for 0 to 2.06 (L0) and at 20 for 0 to 0.7.
  Next: a dense sweep in n and hi (and lo other than 0), then fit L0 and U0.
- Dense sweep (lo = 0, hi 0.51 to 2.20 by 0.01, n 5 to 30: 4,420 cases, scratchpad nat/c.txt): the
  trimming model reproduces r(min) in all 4,420 and r(max) in 4,204. The other 216 tops (e.g. hi 0.72,
  n 20: 0.7499999999999999 = 0.8 - (0.05 + 1 ulp)) start from a label on another step's grid: j·0.1
  (nearest double) explains 129 of them, j·0.2 93, j·0.02 36. So the wider set's ends seem to come from
  the grid of a candidate step tried before the final one, then are trimmed with the final d.
- Next: find which candidate step gives each end (and L0, U0 from lo, hi, n), then build _natscale
  from that and check all 4,915 probe results and these 4,420 bit for bit.
- Starting grids (scratchpad nat/h7.R, h8.R): some candidate grid j·s (s = 1, 2 or 5 × 10^k, nearest or
  Stata's) explains both ends of 4,414 of the 4,420 cases, but no one grid explains all: Stata's 0.05
  (5 × 0.1 × 0.1) fits every case whose step is 0.2 and most others.
- Ruled out (h9.R, h10.R): going out e steps and back e steps (fits 2,898 of 4,420); e from simple
  formulas of n, the label count and hi/d (n - labels, half of it, n - U, ...: at most 3,314 bottoms and
  2,667 tops).
- A clue: 0 to 1.03 (step 0.1) and 0 to 2.06 (step 0.2) are the same problem in steps (10.3 steps, 12
  labels), yet the residue starts at n = 15 for one and n = 14 for the other. So a rounded quantity
  (hi / d, range / n, or a cost compared with n) decides where the wider set starts, and its rounding
  differs between the two. Next: sweep hi/d near 10.3 at several scales (0.103, 1.03, 10.3, 2.06,
  5.15) and n, and look at the scoring rule's comparisons (StatCraft's costs) at those n.
- Scale sweep (lo = 0, n 10 to 22; scratchpad nat/e.txt): a residue at the bottom for hi = 0.0103 (n 16
  to 20), 0.0206 (16 to 20), 0.515 (15 to 22), 1.03 (15 to 20), 2.06 (14 to 20); none for 0.0515, 0.103,
  0.206, 5.15 and every scale with a step of 1 or more. Same problem in steps, different answers: what
  decides is the step's own bits (0.1 + 1 ulp, 0.2, 0.05 + 1 ulp, 0.001 + ...), and how many steps the
  wider set starts below. A model that fits the bottoms seen so far: the start is e = min(n - c, K)
  steps below (c about the label count, K a cap of 3 to 6), and a residue shows only when e reaches a
  count whose repeated additions do not land exactly (3 or more for 0.1 + 1 ulp). Not yet checked
  against the tops.

## The likely mechanism: values kept in local macros (2026-11-15, later)

- Stata 19 keeps a number put into a local (`local a = exp`) as text in %18.0g: 0.1 + 1 ulp becomes
  `.1`, 0.30000000000000004 `.3`, 1.2000000000000002 `1.2`, 2^-55 `2.77555756156e-17`, 1/3
  `.3333333333333333` (observed; RCraft's stata_fmt(x, "%18.0g") gives the same text in every case
  tried). A number read back from such a local is the nearest double to that text.
- Stata's r(min) and r(max) are not such text: 2^-55 and 1.0999999999999999 cannot be read from it.
  They fit an expression evaluated when the result is returned, one local label and the live step:
  (-.1) + (0.1 + 1 ulp) = 2^-55 and 1.2 - (0.1 + 1 ulp) = 1.0999999999999999. r(delta) keeps
  0.1 + 1 ulp, so the step is computed live (10 × 10^-2, Stata's 10^k a product of tenths), not read
  from a local.
- That explains why no pure floating-point formula fitted: the labels round to 16 or 17 significant
  digits at each step that goes through a local, and how many digits fit in %18.0g depends on the
  sign and the scale (`-.1` against `.10000000000000002`; leading zeros of 0.0103).
- Tried on 11,140 cases (c.txt and f.txt): simple forms (M(L·d), M((L-1)·d) + d, M((U+1)·d) - d, and
  so on, M the %18.0g round trip) explain some form at the bottom in 9,926 and at the top in 8,068.
  Next: model the loop itself, labels kept in locals and stepped by the live d, with the stopping
  rule from StatCraft's costs, and fit when each end gets its final + d or - d.
- Tried next (scratchpad nat/h16.R): each end kept in a local e steps beyond its final label and the
  result computed at return as that local ± e·d (all at once, or one d at a time). At most 8,713 of
  11,140 cases fit at both ends (one d at a time; 7,401 all at once). The one-d-at-a-time form fits
  10,471 bottoms. So the shape of the loop is still not right; the stopping rule (StatCraft's costs)
  and which values go through a local are the next things to model.
- A hypothesis that fits the clue of 1.03 against 2.06: Stata's search moves an end one step at a
  time (extending while there are fewer labels than about n, and trimming where the cost falls), and
  each move is stored in a local as %18.0g text. In steps, 0 to 1.03 and 0 to 2.06 are the same
  problem, so extending the top or the bottom costs exactly the same in exact arithmetic; rounding
  breaks that tie one way for steps of 0.1 + 1 ulp and the other for 0.2, which would move where the
  bottom first goes below 0 (n 15 against 14). Next: simulate that loop with StatCraft's costs, the
  %18.0g round trip at each move, and the result computed at return, and fit it to every sweep case.
- The loop form tried (scratchpad nat/h17.R): each end starts at M(J·d) (M = the %18.0g round trip),
  moves outward e times with each move kept in a local (v = M(v ∓ d)), and the result at return comes
  back e steps (v ± d, one at a time). It gives r(min) = M(L·d) when an end does not move (Stata's 0.3
  exactly for 0.3 to 0.348, `1.1` for 0 to 1.03 with n 12 to 14), and for 0 to 1.03 with n 15 to 20 it
  fits with e = 2 at both ends (out to -.2 and 1.3, back to 2^-55 and 1.0999999999999999). Over all
  11,140 cases it fits both ends in 6,347; bottoms outside the data still miss in 1,670 of 10,256,
  and tops inside the data (r(max) below hi) need another form: 0 to 1.03 with n 10 gives
  1.0000000000000002 = 0 + 10·d, not M(10·d).
