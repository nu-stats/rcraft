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
- Baseline counts on the 11,140 swept cases (h20.R), both ends bit for bit: L·d and U·d (RCraft's
  rule today, without its decline) 3,675; the labels read back from locals, M(L·d) and M(U·d), 3,112;
  one or the other 4,954.
- A finding about RCraft today: of the 11,140, it declines 6,271 (a label at 0 with a step below 1)
  and translates 4,869, of which 1,196 are exact and 3,673 differ from Stata in the last bits (none at
  7 significant digits). The probes (225, 226) are built around 0 and round numbers and did not show
  this; ranges like 0.3 to 0.348 do (Stata's r(min) is 0.3 exactly, RCraft's 0.30000000000000004).
- The bottom's step count (h19.R) is not a function of n minus the label count alone.

## 2026-11-15, third pass: the step's base, an on-label tolerance, and five models ruled out

Data: the earlier sweeps (c, e, f, g, b) and probes 225/226 put together, 28,406 results (11,679
distinct ones with a step below 1 other than 0.5), plus two new probes printed with `%21x` through
scalars: lo and hi moved by single ulps around labels.

- r(delta) is always q × 10^k with q one of 2, 5, 10, 20 and Stata's 10^k: a step of 1 × 10^z is never
  1 × 10^z but 10 × 10^(z-1) (0.1 is always 0.1 + 1 ulp). A step of 0.2 is 2 × 10^-1 (the nearest double)
  or 20 × 10^-2 (one ulp above) depending on n: 0 to 1.03 gives the first for n 5, 6 and the second for
  n 7 to 9; 17 to 17.84 the first for n 4 and the second for n 5 to 7. So the search goes power by power
  with those multipliers, and which power it is at when it settles depends on n. The base k is not
  floor(log10) of range/n, range/(n-1) or range/(n+1) (no constant splits 0.72 at n 6/7, 1.03 at 6/7,
  1.5 at 14/15, 2.06 at 20/21, 0.84 at 4/5).
- Stata's own floor(), ceil(), mod(), round() and int() are exact at ±300 ulps around labels (checked
  on x/d for 17.9, 0.9, 19.4, -1.6, 17, 0.3, -2.5, 5): any tolerance below is _natscale's own.
- An on-label clamp, read exactly (scalars, `%21x`, every ulp offset compared bit for bit). Let B be
  the end Stata returns when lo or hi is a little off the label. Then r(min) is lo itself when
  lo <= B and lo/d is within a tolerance of a whole number, and r(max) is hi itself when hi >= B and
  hi/d is within it; on the far side (lo above B, hi below B) the end is B. `_natscale
  16.999999999999893 17.84 10` (17 − 30 ulp) gives r(min) = 17 − 30 ulp; 17 + 3 ulp gives B. Windows
  seen, in ulps of lo or hi (inside / first outside): lo 17, step 0.1: −30 / −100; lo 17, step 0.2:
  −100 / −300; lo 1234, 0.1: 0 / −1; lo 0.3, 0.05: −1000 / −3000; lo −2.5, 0.1: −300 / −1000; lo 5,
  0.1: −100 / −300; hi 19.4 and 19.2, 0.2: +100 / +300; hi 17.9, 0.1: +30 / +100; hi 0.9, 0.05:
  +300 / +1000; hi −1.6, 0.1: +300 / +1000. One tolerance fits all twelve edges when it is measured
  as |x/d − round(x/d)| in double: inside up to 1.82e-12, outside from 2.22e-12 (1234 is in at one
  ulp of 12340, 1.819e-12, and out at three). In ulps of x, in |x|, in the range or in d no single
  value fits. (An earlier reading here, from offsets against a rounded decimal, gave wrong edges for
  0.9 and 1234; the edges above replace it.)
- On all 27,795 distinct results: r(min) = lo in 16,427, and lo/d is within 2.22e-12 of a whole
  number in all but 6; r(max) = hi in 694, hi/d within it in all but 6. With lo on a label, r(min)
  is below lo in 1,781 (B below lo, e.g. lo = −2.5 gives −2.5 − 1 ulp = −25·d) and above lo in 1,243:
  those are the lo = 0 residues (2^-56, 2^-55 ...), so the clamp comes before the final walk back by
  the live step that leaves the residue.
- B is not always L·d and depends on more than L and d: 17 to 17.84 with step 0.1 + 1 ulp and L = 170
  gives B = 17 + 1 ulp (= 170·d) for n 8 (top label 178) and 17 + 2 ulp for n 10 (top label 179). The
  latter is no product, no walk down from the top, and no walk up from 0; it is two or three steps
  above walked down by d. Stata's B at lo = 1234 from below is 1234 exactly, while 12340·d is
  1234 + 1 ulp.
- Off a label, hi's last bits do not matter (hi ± 40 ulps leaves r() unchanged for 17–19.26,
  17–17.84, 0–1.03, 0–0.72, 0.3–0.866), except at a near-tie: 0.3 to 0.459 with n 21 flips between two
  answers as hi moves by 1, 5, 10, 20, 40 ulps, not monotonically. A cost comparison sits at a tie there.
- The lo = 0 residue does not come from lo: for 0 to 0.72 with n 20, any lo in [−0.001, 0.001] gives
  r(min) = 2^-56 (r(max) does change, 0.75 for lo < 0 and 0.75 − 1 ulp for lo > 0).
- Away from 0 the ends do not depend on n once the step is fixed (17 to 17.84, 18.55, 19.26, 19.97 and
  1234 to 1234.84 ...: n only moves L, U and d). The n-dependence of the last bits is in the lo = 0
  family, where labels go below 0.
- With a product start ((L − e)·d, then e additions of d) every lo = 0 bottom has an e that reproduces
  it: e of 0 to 2 (bits as if no extension) or 3 or more. e reaches 3 at n ≈ 1.3–1.46 × (hi/d): 0 to
  0.53 (0.05) at n 15, 0.72 (0.05) at 20, 0.9 (0.1) at 12, 1.03 (0.1) at 15, 1.5 (0.1) at 21, 2.06 (0.2)
  at 14; never up to n 30 for 1.03 (0.05), 0.9 (0.05), 2.06 (0.1). No single factor fits (0.9 needs
  below 1.333, 1.03 above 1.359). The top is walked down from above in nearly every lo = 0 case, even
  with no extension (0 to 0.72, step 0.1: from 3 or 4 steps above).
- Ruled out this pass (both ends bit for bit, on the 11,679): one walk through both ends from a common
  start (667 of 3,166 distinct); decimal starts on coarser grids, intersected over the steps a range
  gets (empty for most ranges); ends carried from coarse steps to fine ones and trimmed with each step
  (1,896, below L·d and U·d at 3,464); snapping by mod (hi + d − mod(hi, d), hi − mod(hi, d) + d,
  lo − mod(lo, d), d·ceil(hi/d) ...: at most 2,356 outside tops and 6,419 outside bottoms).
- Where it stands: r(n) and the label set are StatCraft's cost rule; r(delta) is q × 10^k with an
  n-dependent base; each end is lo/hi inside a tolerance window, otherwise a label reached by walking
  with the live step from a start that is not yet known, and in the lo = 0 family the start lies e
  steps beyond with e growing with n. Next experiments: pin the clamp tolerance (between 1.82e-12 and
  2.22e-12 of x/d; is it 2e-12?) and find how B is built (17 to 17.84: B moves with the top label);
  sweep n for fixed lo = 17.x ranges to find the n at which 0.2 moves
  from 2 × 10^-1 to 20 × 10^-2; and probe the top's start with hi just off a label at large magnitude
  (1234.x), where every walk length leaves distinct bits.

## 2026-11-15, fourth pass: ends are extended and walked away from 0 too

New probe (756 results, `%21x` through scalars): lo = 17.03, 17.07, 17.13, 17.17, 1234.03 ..., 3.03 ...
with hi = lo + 0.84, 1.37, 2.26, n 4 to 24. Counts are on these plus the earlier results: 11,632
distinct results with steps 10 × 10^-2, 5 × 10^-2, 2 × 10^-1, 2 × 10^-2 and 10 × 10^-3.

- Away from 0 and off a label, the ends are also extended and walked back, and the extension grows with
  n: 17.13 to 17.97 (step 0.1) needs the bottom to start 3 or 4 steps below for n 9 to 11 and 5 to 7 for
  n 13 and 14; 17.07 to 18.44 needs the top 0, 2–3, 4–6 and 7–8 steps above for n 16–17, 18–19, 22, 24.
  (The lo = 17 family looked n-free only because the on-label clamp hides the bottom.)
- Step 10 × 10^-2 (4,592 results), "the clamp, else start e steps beyond on the final step's product
  grid (j·d) and add or subtract d e times", for some e in 0..12: bottoms 4,592 of 4,592, tops 4,083,
  both 4,083. Letting a top start on the 2 × 10^-1, 5 × 10^-1 or 1 grid, from either side: 4,507. The 85
  left are ranges ending at or below 0 (−1.03 to −0.168 gives r(max) = −0.1 − 17 ulp, a long walk at
  magnitude 1 that no start within 25 labels reproduces).
- All five steps: unclamped bottoms 7,269, of which 7,256 fit some e from below; unclamped tops 11,207,
  of which 10,141 fit some e from above.
- How e follows from lo, hi, n is not found. Best single formulas (e inside the fitting set):
  bottoms round((n − range/d)/2) 5,448 of 7,269, ceil((n − cnt)/2) 5,305; tops ceil((n − range/d)/2)
  5,737 of 11,207. A start of m labels centred on the data (m = n − 1, n, n + 1; floor, round, ceil):
  best m = n + 1, ceil: bottoms 9,660 to 9,847 of 11,632 with the clamp, tops 5,855, both 5,096.
- Not translated: no rule yet gives every case, so `natscale.rs` still declines these steps.

## 2026-11-15, fifth pass: e against n

Two new probes (`%21x` through scalars), n 4 to 40: lo = 17.03, 17.13, 12345.03, 12345.13 with
hi = lo + 0.84 and + 1.37 (296 results), and lo = 0.313, 0.347, 0.431, 0.469 with hi = lo + 0.42 and
+ 0.61 (296 results).

- The product-walk model is not chance. With the same e range (0 to 12), the 7,269 unclamped bottoms fit
  in 7,256; the same targets moved by one ulp fit in about 2,080, by two ulps in about 1,330.
- What can be read is not e itself but a block of e that all give the same bits, and the block moves in
  jumps as n grows, not by one label per n. Step 0.05 at 17 (17.13–17.97, 17.03–18.4, 17.13–18.5): the
  bottom's block changes exactly at n − cnt = 2 and at n − cnt = 11, in all three ranges (e.g. 17.13 to
  17.97, cnt 19: [0:4] for n 19–20, [5:9] for n 21–29, [10:15] for n 30–40). Step 0.1 at 17: one change,
  at n − cnt = 3 ([2:3] or [3:4] before, [4:6] or [5:7] after). Step 0.02 + 1 ulp below 1 (0.313–0.733,
  0.347–0.767, cnt 23): changes at n − cnt = 2 or 3 and at 11.
- At 17, walks from different decimal starts merge (16.5 + (0.05 + 1 ulp) rounds to the double nearest
  16.55), so many starts give the same bits: 16.5, 16.6, 16.4 and products 16.65..16.85 all fit
  17.13–17.97 n 21. At 12345 the blocks are wider still ([1:5]). Below 1 the blocks are narrow but often
  fit from both sides (0.313–0.733 step 0.05: from 13–17 below or 9–11 above).
- Best formula for e found by a search over floor/ceil/round((a·n + b·cnt + c·range/d + k)/m), with
  small integer a, b, c, k, m: ceil((n − range/d + 4)/3), inside the bottom's block for 5,371 of 7,832
  bottoms and 2,783 of the 3,398 whose block excludes 0. Not a rule.
- The walk is not through locals. On 1,500 sampled unclamped bottoms, "start e steps below, each step
  kept as %18.0g text, the last step live" fits 613 (403 as a walk, 479 as the text of the label
  itself), against 1,498 for the plain product walk; every bottom the text form fits, the product walk
  fits too. Tops (1,500 sampled): product walk from above 1,372, any text form 1,055, either 1,419. So
  the earlier reading that ends go out and back through locals (the third section) does not hold up:
  the steps are taken in doubles, from a product start j·d.
- What changes at the jumps (n − cnt about 2–3 and 11), first look. The start is on the final step's own
  grid, not a coarser one. For steps below 0.2 (5,709 bottoms off lo and 0), "(L − e)·d plus e steps"
  fits 5,673, and 2,057 when the targets are moved one ulp; a start on Stata's 2 × 10^-1 grid fits 4,355
  but 2,816 when moved, 5 × 10^-1 3,528 (1,608), 1 2,592 (845). So the 0.2-grid starts that fit near 17
  (16.8, 16.6, 16.4 for 17.13–17.97, step 0.05) are mostly coincidence.
- Below 1, where the blocks are narrow, e takes three values in steps: 0.313–0.733 with step 0.02 + 1 ulp
  (cnt 23) has e in [0:1] for n − cnt ≤ 2, [2:5] for 3 to 10, and 6 or more from 11; 0.347–0.767 the
  same with [0:3], [4:7], [8:27]. So e is a step function of n − cnt with steps at 2 or 3 and at 11, the
  same places as near 17. What those two points are is still open: they are not where a coarser or
  finer step's label count crosses n, and not a fixed number of final steps (9 or 8 apart).

## 2026-11-15, sixth pass: the jumps against the cost comparisons

- The jumps are not cost events. Over the two probes (n 4 to 40), 494 pairs n → n + 1 keep the same step
  and bottom label; the bottom's start set changes in 46 of them, and only 3 of those 46 fall where any
  comparison of StatCraft's rule changes (the order of the candidate steps, or any step's choice of
  inner or outer end; 14 such changes in all). Between 21 and 40 the ranking 0.02 < 0.05 < 0.1 < 0.2
  never changes in the ranges below 1, yet the bottom jumps twice.
- The jumps sit at fixed values, not at fixed n − cnt. Written as the absolute start index s = L − e
  (start s·d), every range with step 0.02 + 1 ulp falls into the same blocks: s in 22:23, 19:21, 14:18,
  10:13, then 9 or below (starts 0.44–0.46, 0.38–0.42, 0.28–0.36, 0.20–0.26, ≤ 0.18), whatever lo, hi or
  cnt. At 17 with step 0.05 the blocks are 338:340, 333:337, 327:332. These are where walks from nearby
  starts round to the same bits. So the "steps of e at n − cnt 2 and 11" were the start crossing two of
  these block edges; the start itself moves smoothly down as n grows.
- How fast it moves is not one constant. Fitting s = floor(A − r·n) per range: step 0.02 ranges give r
  in [0.445, 0.570] (0.313–0.733), [0.405, 0.495] (0.347–0.767), [0.430, 0.495] (0.469–0.889),
  [0.505, 0.710] (0.469–1.079); step 0.05 at 17 [0.505, 0.62]; 0.431–0.851 fits no straight line (it
  drops 5 steps between n 24 and 32, then 3 or fewer from 32 to 40). Near half a step per n, as if a
  label were added on each side for every two of n, but not exactly half, and not straight.
- Next: probes built for narrow blocks (values where walks merge least) so that s(n) can be read
  one n at a time, then fit s against lo, hi, n and range.

## 2026-11-15, seventh pass: ranges where walk starts can be told apart

- Where to look. In R, for each step and bottom label, how many of 25 walk starts (L − 24 .. L) give
  different bits: with step 0.05 + 1 ulp the best labels are 3.15–3.85 (11 or 12 distinct results, runs
  of 3 at most), with 0.2 they are 12.6–15.2; below 1 with 0.02 never more than 5. Probe 8 (`%21x`
  through scalars, 1,080 runs, n 4 to 48): lo 3.13, 3.16, 3.22, 3.28, 3.31, 3.37 with widths 0.52, 0.62,
  0.72, and lo 12.63, 12.87, 13.11 with widths 2.06, 2.62.
- Most ends are the plain product. Of the 1,034 rows with an inexact step, 793 of the 872 bottoms off lo
  are exactly L·d and 809 of the 1,033 tops off hi exactly U·d. In 59 of the 74 groups with the same
  range, step and labels over three n or more, neither end changes with n at all: every range at
  3.1–4.1 with step 0.05 keeps its bottom bits from its first n to its last (n − cnt up to 21), and
  12.63–15.25 with step 0.1 has r(min) = 126·d exactly for n 27 to 48 (only start 126 gives those bits).
- A few ranges drift, and they switch on at an n of their own. Step 0.1, width 2.62, 28 labels, three
  positions: 12.63 never moves (to n 48); 12.87 holds to n 41, then at n 42 the bottom start jumps from
  within 2 steps of L to 8–11 steps below in one n; 13.11 moves at n 30, 37, 40 and 47, its r(min) going
  +2, +1, 0, −1 ulp from the decimal and r(max) +2, 0, −1, −3, −5. Same width, step and label count;
  only the position differs. Others that move: 12.63–15.25 step 0.2 (n 20, 23), the three 2.06-wide
  ranges with step 0.05 (n 42 to 46), and the top alone in a few step 0.02 and 0.1 ranges at 3.1–3.3.
- Ruled out on these data: one upward walk from the bottom's start also giving the top (56 of 871); a
  formula in n, cnt, range/d and the end distances (best 1,084 of 1,448 bottoms, and the best ones do
  not use n); a start on the grid of 5·d (bottoms 851 of 1,448, but 624 and 776 when moved one ulp);
  an end that goes out e steps and back e (fits 1,059 bottoms, but almost all of those are just L·d).
  Which ends are not the product does not follow the step or how far the end is outside the data.
- So the earlier picture (a start that slides down about half a step per n) was taken from ranges where
  the walk shows; on most ranges there is no walk. The open question is now what switches it on, and why
  at n 30 for 13.11–15.73 but not by n 48 for 12.63–15.25.

## 2026-11-15, eighth pass: what switches the walk on

- At 17.03–17.87 and 17.13–17.97 (step 0.05) the two ends move in turn: the bottom's start changes at
  n 21 and 30, the top's at 26 and 36 (and the top falls back for n = 30 alone). Both ranges give the
  same starts at every n.
- Probe 9 (`%21x` through scalars, 1,196 runs): step 0.1, lo from 12.61 to 13.11 by 0.02, widths 2.06
  (n 21–40) and 2.62 (n 26–51). For each lo the first n where an end stops being the plain product:
  - Width 2.06 (22 labels): it depends on where the data ends sit against the labels, and repeats
    every 0.1. Bottom label 0.1 or 0.3 step outside, top 0.3 or 0.1 outside: no walk to n 40 (five
    positions each). Bottom 0.5 outside, top 0.1 inside: the bottom walks from n 29 (12.65, 12.85, 13.05)
    or 33 (12.75, 12.95). Bottom 0.1 inside, top 0.5 outside: from n 24 (12.69 to 12.99), 21 at 13.09.
  - Width 2.62 (28 labels) instead depends on which label block lo is in, not on the gaps: L = 126
    never by n 51; 127: n 46; 128: 42; 129: 34; 130: 30 (32 at 13.07); 131: 30. Within a block the
    four positions (gaps 0.1 to 0.7) switch at the same n, both ends together.
- After the switch the ends are reproduced by walking from round values on the 0.5 grid (exact
  decimals, then steps of 0.1 + 1 ulp): bottoms from 13, 12.5, 12, tops from 15.5, 16, 16.5, and the same
  values serve many positions (12.73–15.35: 12.5 and 15.5 from n 46, 12 and 16.5 from 49; 12.83–15.45:
  the same from 42 and 47; 13.03–15.65: 13 and 16 from 30, 12.5 from 38, 16.5 from 40, 12 from 47).
  The ends jump outward in these coarse units at n shared across positions (40, 47).
- Ruled out: a start floor((lo − p)/0.5)·0.5 and ceil((hi + q)/0.5)·0.5 with p, q depending on n only
  (no p, q fits every lo at most n). Coarse starts in general are weak evidence alone: on the 400
  walked bottoms of probes 6–8, starts on the 5·d grid fit 349 but 294–320 after a one-ulp shift. The
  evidence here is that one value fits each end over a whole run of n and across positions.
- Next: probe lo finely across a 0.5 block (several widths) at fixed n to map which coarse value each
  end starts from, and read the n at which each value takes over.

## 2026-11-15, ninth pass: one 0.5 block, lo by 0.01

- Probe 10 (`%21x` through scalars, 561 runs): step 0.1, lo 12.50 to 13.00 by 0.01, width 2.62 at
  n 30, 34, 38, 42, 46, 50 and width 2.06 at n 24, 28, 32, 36, 40.
- Starts on the 0.5 grid are not the general form: many ends have none (e.g. every top for lo 12.61 to
  12.79 at width 2.62). Starts at the exact decimal of a label (what a %18.0g local gives back) cover
  more, but not all either. The clearest view is each end's offset in ulps from the decimal label,
  marked where it is also the product L·d.
- Width 2.06 (22 labels) is ruled by whether each label is inside or outside the data:
  - both labels outside (bottom 0.1 to 0.4 step below lo, top 0.1 to 0.4 above hi; and 0.7/0.7): both
    ends are the product L·d and U·d at every n (25 positions);
  - bottom label inside (lo 0.01–0.02 below it): r(min) is the decimal label, or 1 ulp above it, at
    n 24 and 28, then 1 ulp lower from n 32 (labels 126, 128 … lo 12.58, 12.88) or 36 (127, 129, 130);
    the top stays as it was;
  - top label inside (gap −0.1, −0.2): r(min) leaves the product and drops 1 ulp from the same n,
    32 or 36, and r(max) moves too.
  So below n 32 an end's form follows from its own side (outside: product; inside: decimal); from
  n 32 or 36 an inside label on either side moves the bottom one step further.
- Width 2.62 (27–28 labels) moves even with both labels outside, at an n set by the label block: L 125
  and 126 never (to n 50), 127 at 46, 128 at 42, 129 at 34 (top) and 42 (bottom), with steps of −1 and
  −2 ulps after. A lo 0.01 below its label behaves unlike one 0.02 below: 12.99, 12.89, 12.79, 12.69
  stay at the decimal label to n 50, while 12.98, 12.88, 12.78, 12.68 drop at n 38 or 42.
- Reading: this is what a search that moves ends one step at a time would leave. An end the cost puts
  inside the data is reached by a step inward (decimal, or a short walk). For n well above the label
  count the search first extends outward and then comes back, each move rounded. The positions where
  that happens shift with the rounding of the comparisons. The earlier hypothesis in "The likely
  mechanism" (moves kept in locals) fits these maps better than a fixed start rule.
- Next: simulate that search (extend while labels < about n, trim where the cost falls, each move
  through a %18.0g local, result computed at return) with StatCraft's costs, and fit it to probes 8–10
  map by map, starting with width 2.06.

## 2026-11-15: parked

The user parked the search to move on to graphs, esttab and coefplot. `natscale.rs` stays as 4af4a35
left it: it translates only where it matches Stata 19 bit for bit and declines the rest.

- Why parked: no safe subset exists to widen into. Over all 15,245 swept results with steps below 1
  (other than 0.5), RCraft's labels agree with Stata's in 15,244, but the last bits do not follow any
  simple condition: with both labels outside the data and n at or below the label count, Stata gives
  L·d and U·d in only 296 of 589; where L·d equals the decimal label at both ends, in 326 of 946. Going
  through a macro does not hide it (`%18.0g` keeps 16 digits: `.5000000000000002`, `1.38777878078e-17`).
  Of the 511 corpus do-files only the three `_natscale` probes (225–227) call it; no replication package
  does. It holds the probes' share below 89.1% (about 10.9 points).
- Where to pick up: simulate the search that moves the ends one step at a time (extend while there are
  fewer labels than about n, trim where the cost falls, each move through a `%18.0g` local, the result
  computed at return), with StatCraft's costs, and fit it map by map: probe 10 at width 2.06 first (its
  ends follow inside/outside labels and switch at n 32 or 36), then probes 8 and 9, then the sweeps.
  The 0.2 step's bit form (2×10^-1 in 2,680 results, 20×10^-2 in 363) is not decided by range/n's decade
  and needs its own rule. Probe logs and the R scripts are in the session scratchpad (`n/`, `n/st/`).

## 2026-11-15: resumed, with a scoreboard

The user reversed the park ("keep working to get that 10.9 points"). Work now goes through one scoreboard:
a rule gives r(min) and r(max) from lo, hi, n, the labels and Stata's step, and is scored bit for bit
on all 15,244 swept results with steps below 1 (other than 0.5) where RCraft's labels agree with
Stata's (15,244 of 15,245). The scripts and the pooled data are in the shared notes folder
(`natscale-work/`, `harness.R`), outside the repo.

- Step bits: Stata's d is always q × p with p = 10^k as a product of tenths; q = 5 always 5 × 10^k,
  q = 1 always 10 × 10^(k−1), 0.2 is 2 × 10^-1 in 2,680 and 20 × 10^-2 in 363 (rule still open; not
  the decade of range/n, range/(n−1) or range/(n+1)).
- Baseline: L·d and U·d plus the on-label clamp: 4,850 both ends (bottoms 9,473, tops 6,129).
- Bottom with the label inside the data (lo just below it): (L − 1)·d + d, one live step in from the
  outside label. Exact in 278 of 278 when n < cnt and 357 of 385 when n = cnt. Score: 4,891 both
  (bottoms 9,616). Outside labels with n ≤ cnt are L·d in 1,628 of 1,731.
- lo = 0: the walk up from about −3·d appears only when the top label is outside the data, and from
  an n that is constant over bands of hi: step 0.1 from n 12 for hi ≤ 0.9, 15 for 0.9 < hi ≤ 1.2,
  20 for 1.2 < hi ≤ 1.5; step 0.05 from 15 for hi ≤ 0.6, 20 for 0.6 < hi ≤ 0.9; step 0.2 from 12 below
  2.0, 15 above. The bands are not yet explained.
- Tops are the main gap (9,115 misses). No single form wins in any class: L·d + (U − L)·d fits 699 of
  1,131 outside tops with n = cnt; U·d 598. Next: find the top's form class by class on the scoreboard,
  then the extension (n > cnt).

## 2026-11-15: the tops, first pass on the scoreboard

Scoreboard unchanged at 4,891 both ends (tops 6,129 of 15,244); no top rule beat U·d overall.

- r(n) always equals the label count (4,185 probe results), so it carries no trace of the search.
- In the cleanest family (probe 10, width 2.06, step 0.1, both labels outside, n 24–40) r(max) =
  L·d + (U − L)·d for every label pair (10 of 10): the bottom label plus the span, not U·d. Over all
  tops off hi that form fits 6,918 of 14,698 (U·d 6,101); with a free base L·d ± j ulp, 9,568. It holds
  at large magnitudes (step 0.1 above 10: 72 of 76, above 1000: 20 of 20) and fails below 1 and for
  negative ranges.
- Negative ranges show what the top really is: lo −1.03, step 0.05. r(max) is reached by walking down
  from a start above U, and the start depends on n and not in one direction: hi −0.723 (U −14) starts 3
  or more steps up for n 7–8, at most 2 for n 9–11, 3 or more again for n 12–15; hi −0.612 (U −12) is
  U·d for n 11 only, more than 12 steps up for n 9–10, 1–5 up for n 12–20. The StatCraft ranking of the
  candidate steps does not change at those n, so the start is not simply the runner-up's top.
- Mod-based forms (hi − mod(hi, d) + d and kin, with Stata's exact mod) fit fewer than U·d (at most
  6,101).
- Next: for the negative family, sweep n finely at fixed hi with lo moved, and read where the top's
  start sits as an exact decimal on coarser grids (0.25, 0.5, 1), to see which candidate's labels it is
  taken from at each n.

## 2026-11-15: the negative family, a cascade, and lo moved by ulps

Scoreboard unchanged at 4,891 both ends.

- Probe 11 (672 runs): lo −1.01 to −1.09, hi −0.76, −0.723, −0.686, −0.612, n 5–25, mostly step 0.05.
  Near zero, walks from many starts merge, so a top's start is a set: read as exact quarter values it
  is one of {0.25}, {0.75}, {−0.5, 0, 0.5} or {−1, 0, 1}, and which one changes with n back and forth
  (lo −1.09, hi −0.723: 0.25 at n 8, 0.75 at 9–11, 0.25 at 12–14, 0.75 at 15–18). Bottoms likewise
  {−0.75, −0.25} or {−2.25, −1, 2.5}. This family does not pin the start.
- A cascade (each end put on the coarsest visited grid, then walked by each finer visited step to its
  outside label, the last step walked to Stata's label) fits both ends in 1,177 of 2,000 sampled cases
  for some path, against 470–504 with the targets moved one ulp. But 770 of the 1,177 are plain
  products; on the 1,230 others it fits 407 (33%), little above chance. Full 1-2-5 cascades fit at most
  223. So a cascade is not the mechanism.
- Probe 12 (540 runs): 40 cases with residues at both ends and 20 without, each with lo and hi moved by
  −2 to +2 ulps. In the residue cases r(min) changed in 5 of 320 runs and r(max) in 5; when they changed
  they changed together. So the residue does not come from arithmetic on lo or hi themselves (no
  lo − mod(lo, d) and the like): it comes from the labels and the steps, and a single decision moves both
  ends at once.

## 2026-11-15: where both ends flipped, a tie broken by rounding

Probe 13 (795 runs): the two residue cases from probe 12 whose ends flipped (lo 12.91, hi 15.53, n 41,
step 0.1; lo −1.03, hi 0.202, n 24, step 0.1), with lo moved −30..30 ulps, hi moved −30..30, and a
13 × 13 grid of both. (The third flip, lo −1.03 with step 0.01, is only the on-label clamp: lo/d = −103.)

- The flips are not thresholds. About 1 move in 10, scattered: lo +1 ulp flips, +2 does not; the grid
  shows no pattern along constant hi − lo or constant hi + lo. Such a fingerprint is what a floating
  comparison gives when the two sides are equal in exact arithmetic for every lo and hi nearby (for
  example the cost of extending at the bottom against the top, both outside): the last bits decide.
- In 12.91–15.53 the three results are the same labels reached by walking in from different distances,
  and the distance is the same at both ends: 236 points need e 9–12 below and 8–10 above (shared 9–10),
  17 points 4–8 and 5–7 (5–7), 12 points 0–3 and 0–2 (0–2). One number decides both ends, and the
  rounding of a tied comparison picks it: mostly the largest, sometimes it stops early. A loop that
  extends while the cost does not rise, with ties at each step, would leave exactly this.
- Over probes 6–8 a shared e is found in 82% of cases with both ends walked against 72% for tops paired
  at random: weak in general because the sets are wide, sharp where they are narrow.
- Next: write the loop (extend while the cost does not rise, with a candidate cost expression in
  floating point) and fit its exact evaluation to the 530 fingerprint points of these two cases. A
  formula that matches every point's choice is very unlikely by chance; that pins the cost arithmetic.

## 2026-11-15: the padding against n at fixed points (probe 14)

Probe 14 (616 runs): the base point and three points of each other result from probe 13, each at n 5–60.
e is how far each end walks in (product start (L − e)·d, then live steps).

- 12.91–15.53 (step 0.1, labels 129–156): e is a staircase in n. Base: n 28–33 e 0–2 at both ends,
  n 34–39 e 5–7, n 40–43 e 9–10, n 44–52 bottom 9–12 and top 11–12. The points moved by a few ulps
  follow the same staircase but drop back one or two levels at single n: lo +1 ulp at n 39, 41 and 43
  (to e 0–2), lo −30 ulps at n 41 and 43 (to 5–7), lo −13 at n 38 (0–2) and 41 (5–7), lo −16 and −7 at
  n 41 (0–2). So the tie is not "n against a threshold" (that would tie at one n only): it ties at
  several n, mostly odd ones, as a choice between two options that cost the same whatever n is would.
- −1.03–0.202 (step 0.1, labels −11–2): the padding moves from one end to the other as n grows. n 19–21
  bottom e 1–3 and top 5–7; n 22–24 bottom 4–6 and top 1–4. At n 24 the moved points (lo −25 and −20
  ulps) keep n 19–21's split.
- With step 0.05 (n 53–60 at 12.91–15.53) the levels are 1–3, 3–5, 5–7 instead.
- Next: fit a loop that adds one label at a time to whichever side is cheaper, with the comparison in
  floating point, to these staircases and the probe 13 grid.

## 2026-11-15: the published algorithms, and a centred span that fits

Read (the user downloaded the papers): Heckbert, "Nice numbers for graph labels" (Graphics Gems, 1990);
Lewart, CACM Algorithm 463, SCALE1/SCALE2 (1973); Nelder, Applied Statistics AS 96, "A simple
algorithm for scaling graphs" (1976). None is copied here; they are used as models to test.

- Heckbert: step = nicenum(range/(ntick − 1)), labels floor(min/d)·d to ceil(max/d)·d, ticks made by
  adding d in a loop. Lewart SCALE1: step 1/2/5 × 10^k from the geometric means √2, √10, √50, a
  tolerance DEL on the floors, ends as products, then clamped to the data if round-off put them inside
  (the same idea as Stata's on-label clamp). Both always enclose the data, which `_natscale` does not
  (Statalist: 39.6–145.3 with n 5 gives 40 to 140 by 20).
- Nelder AS 96 does what `_natscale` does: exactly N marks, centred on the data, lowest mark
  STEP·floor(½(1 + (min + max − RANGE)/STEP) − bias), highest = lowest + RANGE, so the end marks may sit
  inside the data. Its steps (1, 1.2, 1.6, 2, 2.5, 3, 4, 5, 6, 8) are not Stata's (1, 2, 5).
- Tested: bottom start d·floor(½(1 + (lo + hi − R)/d)), top start = that + R (a sum, not a product),
  each walked in by d to Stata's labels, for some span R. It reproduces both ends of 12.91–15.53 at
  every n from 26 to 60 and of the ulp-shifted points (the flips are R changing), and of −1.03–0.202 at
  all but n 19–21. On 3,000 sampled sweep cases some span fits both ends in 2,884 (96%), against 943 and
  963 with the targets moved one ulp; on the 1,858 cases that are not the plain product it fits 1,742
  (94%) against 540 and 578. So the ends are a centred, padded label set over a span R, trimmed back
  to the labels by live steps.
- What sets R is open. It is a step function of n (12.91–15.53, step 0.1: R/d about 27–30 for n 26–33,
  37–43 for 34–39, 44–46 for 40–43, 47–51 for 44–52) and R/d is usually within a few of n, but no
  simple formula (multiples of n or U − L rounded up, nice values of n·d with 1-2-5, 1-2-2.5-5 or
  Nelder's steps) is in the fitting set for more than 1,384 of the 2,884. Next: treat R as chosen by a
  cost over centred spans (the ulp flips say a tied comparison picks it), fitted to these sets.

## 2026-11-15: the span R across positions (probes 9 and 10)

For each run of probes 9 and 10 (step 0.1, lo 12.50 to 13.11), the spans R/d for which the centred
model reproduces both ends:

- Width 2.06 (U − L = 21): with both labels outside the data (lo 0.01–0.04 above a label, hi 0.06–0.09
  below one), R/d = 21 exactly at every n from 21 to 40: no padding at all, even at n = 40. Padding comes
  only with a label inside: top inside (12.55, 12.65, 12.85, 13.05: R/d 22–26 up to n 28, then 32–36,
  12.65 exactly 22 then exactly 32 from n 29); bottom inside (12.68, 12.69: 26–30, then exactly 34 from
  n 32; 12.78: 27–31, then 37–41; 12.88: 28–30, then 43–45; 13.09: 41, then 42–44).
- Width 2.62 (U − L = 27): R/d = 27 at every n for lo 12.51–12.57 and 12.61–12.67; padding from n 46
  for 12.71–12.77, n 42 for 12.81–12.87, n 34 for 12.91–12.97 (27 → 37–43 → 44–46 → 47–51) and n 30 for
  13.01–13.11. So with both labels outside, padding depends on where the labels sit, not only on n.
- Some runs fit no span (12.58 and 12.59 at width 2.06; 12.75, 12.76, 12.98, 12.99 at higher n): all
  with a label inside the data, so the inside-label path is not yet modelled right.
- Padded sets read from exact spans, e.g. 12.65–14.71: [12.6, 14.8] (23 labels) for n ≤ 28, then
  [12.1, 15.3] (33 labels) from n 29; 12.68–14.74: [12.0, 15.4] (35 labels) from n 32. The counts exceed
  n, so the span is not "n marks" as in Nelder.

## 2026-11-15: padding split around the labels; what does not pick it

- The runs the data-centred span missed (all with a label inside the data, or at high n) have
  consistent (bottom start, top span) pairs whose start is one index above the centred floor. A second
  form, the padding p split around the labels (bottom start L − ⌈p/2⌉ or L − ⌊p/2⌋, top start = bottom
  start + (U − L + p)·d as a sum, both walked in), fits both ends in 2,862 (ceiling) or 2,840 (floor) of
  the same 3,000 sampled cases, against 984 and 901 with targets moved one ulp. Either split, or the
  data-centred span: 2,915 of 3,000 (97%).
- The padded label count M = cnt + p is usually near n (narrow sets: M − n from −2 to +4 in most), but
  often p = 0 where n is far above cnt. No rule in n alone: p = max(n + c − cnt, 0) for c from −2 to 3
  is in the fitting set for at most 1,338 of 2,862, and with "or p = 0" at most 2,117.
- A cost over symmetric padded sets ((0.67·inside + wo·outside)/range or /span + wn·|M − n|/n, argmin
  over p) picks a fitting p in at most 1,351 of 2,862 for any wo in 0.1–1 and wn in 0.05–5. So the
  padding is not that cost's choice either.
- Next: read p exactly on a family where only p varies (fixed lo, hi; n swept finely; several steps)
  and look at which candidate steps' label sets the padded ends coincide with.

## 2026-11-16: padded ends against coarser grids

Every (bottom start, top start) pair that reproduces both ends was listed per n for 12.91–15.53 and
−1.03–0.202 (probe 14, step 0.1). The padded ends do not sit on one coarser candidate's labels: only
n 40–43 of 12.91–15.53 has a pair with both ends on the 0.5 grid ([12, 16.5]); n 34–39 allow
[12.1–12.5, 16.1–16.4] (no [12.5, 16]); n 44–52 allow [11.7–12, 16.6–16.9] (no 17); −1.03–0.202 has
[−1.5, 2] at n 22–24 only. So the padding is not "the enclosing labels of a coarser step". Also
checked by hand: Heckbert's loose labels with ntick = n (range = nicenum(hi − lo), step =
nicenum(range/(n − 1)), rounded) change step at n 35 for this range, not at 34, 40 and 44.

## 2026-11-16: padded sets read exactly; Nelder's own scale tried

- Of 6,000 sampled cases, 272 have exactly one (bottom start, top start) pair that reproduces both ends;
  135 of them are padded. The padded label count M is often the same across n and across ranges:
  lo 0 to 0.05 with hi 0.69–0.85 and step 0.05 all give [−0.2, 0.95]; lo 0 to 0.05 with hi 1.43–1.59
  and step 0.1 all give [−0.4, 1.9]; both are 24 labels for every n from 20 to 28 (the same problem
  scaled by 2). Elsewhere M is the label count plus 1 or 2 (lo −2.5, hi −2.045, step 0.05: 12 labels
  for n 9 to 18).
- In the lo = 0 sweep with step 0.1 the padding starts at n 12, 15 and 20 for hi up to 0.9, 1.2 and 1.5
  (bands at multiples of 3 steps), and the smallest padded count that fits at the start is 16, 19 and 24:
  four more than the n where it starts. Read as sets: [−0.3, 1.2], [−0.3, 1.5], [−0.4, 1.9].
- Nelder's AS 96 run as written (his steps 1, 1.2, 1.6, 2, 2.5, 3, 4, 5, 6, 8; N = n − 1, n or n + 1;
  bias 1e-4; his zero rules on or off), each end then walked by Stata's step to Stata's label: at most
  1,389 of 15,244 both ends (bottoms 6,373, tops 2,515), below the plain product's 4,850. With Stata's
  1-2-5 steps instead: 2,576. So the padded set is centred like Nelder's, but not his scale.

## 2026-11-16: the bands away from 0 (probe 15)

Probe 15 (`%21x` through scalars, 3,933 runs): lo 0.51, 1.01, 5.01; widths 0.51 to 2.19 by 0.03;
n 8 to 30. The centred-span model fits both ends in 3,790; padding is forced (no fit without it) in
1,856.

- The lo = 0 bands (padding from n 12, 15, 20 for hi up to 0.9, 1.2, 1.5) are in value, not in steps:
  at lo 0 their edges sit at multiples of 0.3 for steps 0.05 (0.6, 0.9), 0.1 (0.9, 1.2, 1.5) and 0.2
  (about 2.1). Away from 0 they do not hold. With lo 5.01 and step 0.1 the first padded n follows the
  width: 8 up to width 0.8, then 9, 10, … 19 for widths 0.90 to 1.92, about width/step.
- Away from 0 the padding goes with the top label sitting inside the data. lo 5.01, step 0.1: hi 6.21
  (top label 6.2, 0.1 step inside) pads, R/d 14–15 for n 12–14 and 18–19 from n 15; hi 6.24 (0.4 step
  inside) only reaches out to the next label (R/d = U − L + 1 = 13) at every n; hi 6.27 (top outside)
  never pads (R/d = 13 = U − L). hi 6.51 (0.1 inside): R/d 16–18 for n 15–17, 21 at n 18, 22 from
  n 19 to 29. So an end just inside the data is what lets the padding grow.
- Once padding starts, the padded count stays put as n grows, and at the start it is about four more
  than that n: lo 0 (16, 19, 24 from n 12, 15, 20), lo 5.01 hi 6.21 (19–20 from n 15), hi 6.51 (22 at
  n 18, then 23). What fixes it there is the open question.

## 2026-11-16: the lo = 0 onsets over the whole sweep

First n with a bottom residue (the walk from below 0), lo = 0, by runs of hi:

- Step 0.1: none while the top label sits inside the data (hi 0.81–0.82, 0.91–0.92, 1.00–1.02, …,
  1.70–1.72); otherwise n 12 for hi 0.76–0.90 (top label 0.8 or 0.9), 15 for 0.93–1.20 (1.0 to 1.3),
  20 for 1.23–1.79 (1.3 to 1.9), none from 1.80 to 2.53 within n ≤ 30.
- Step 0.05: 15 (hi 0.515–0.60), 20 (0.62–0.89), 25 (1.11), 27 (1.17–1.20); none where the top label is
  inside.
- Step 0.2: 12 (1.65–1.99), 15 (2.05–2.38), 20 (2.47–3.04), 25 (3.07–3.22), with single-hi exceptions
  (14 at 1.46 and 2.06, 17 at 2.97, 21 at 1.50).
- So at lo = 0 the onset is a staircase of round counts (12, 15, 20, 25) that rises with the top label,
  not with hi itself, and only when the top label is outside or on the data. In units of n·d the
  onsets are 1.2, 1.5, 2.0 (step 0.1), 0.75, 1.0, 1.25 (step 0.05), 2.4, 3.0, 4.0, 5.0 (step 0.2).
  Taken with the padded counts (16, 19, 24 = onset + 4), the onset n reads as a count on a round
  scale (12, 15, 20, 25 ...), and the padded set as that count plus four. Away from 0 (probe 15) the
  onset instead follows width/step and needs an end label just inside the data, so the two regimes
  differ in what triggers the padding. Scoreboard unchanged at 4,891 (no rule yet to score).

## 2026-11-16: fractional n, and the padded count at the onsets

- `_natscale` accepts a fractional n (rc 0). Probe 17 (1,414 runs, n by 0.01 across each onset for 14
  ranges: lo 0 with hi 0.78 to 1.75, lo 5.01 with hi 6.21 and 6.51, step 0.1 and 0.05): every change
  of result falls between n k.49 and k.50 (11.49/11.50, 14.49/14.50, 17.49/17.50, 19.49/19.50). So n is
  rounded to the nearest integer before use; fractional n adds no information.
- Read as a centred Nelder-style scale with N marks and Stata's step (span = d·(N − 1)), the padded
  count at lo = 0 is the smallest N whose step rounds up to d (N = 16 for hi 1.45, 12 for 1.05,
  10 for 0.85, the label count itself) until the onset, and then jumps to N2 = 24, 19, 16 (onset 20,
  15, 12) and stays there. N2 is the onset plus 4 in all three bands; the valid N for step 0.1 run to
  29, 21 and 17, so N2 is not the largest valid N either.
- Scoreboard unchanged at 4,891.

## 2026-11-16: a search over the mark count N: n + 4, capped

Treating the padded set as a centred scale of N marks with Stata's step (span d·(N − 1)):

- lo 0, hi 1.45, step 0.1: N from 16 to 29 all round up to step 0.1. Before the onset the bits do not
  pin N to the label count: n 15–19 allow any N from 16 to 23; n 20–28 allow only 24. So N = n + 4
  fits every n here, capped at 24. A cost over N cannot give this (at n 20 it would have to prefer 24
  marks to 20, with more outside and further from n).
- On 3,000 sampled cases some N fits both ends in 2,883; N = n + c fits 1,013 (c 0), 1,279 (1), 1,332
  (2), 1,335 (3), 1,311 (4), 1,025 (5). Where n + 4 misses, the fitting set lies below it in 1,062
  (a cap), above it in 75, and n + 4 falls in a gap of it in 435.
- Per range and step (1,481 groups of the sample), a cap C with N = min(n + 4, C) fits every n in
  1,303 groups, and binds in 1,239. The caps vary: C = the label count (no padding) for several ranges
  (lo −1.03, hi −0.982 and lo −2.5, hi −2.452, step 0.005), 16 for 10.3 steps from 0 (lo 0 with hi
  0.0515 at step 0.005 and hi 0.206 at step 0.02, the same problem), 24 for lo 0, hi 1.45. Not a fixed
  share of the range or of the label count, and not a fixed outside distance. Next: what sets the cap.
- Scoreboard unchanged at 4,891 (no rule for the cap yet).

## 2026-11-16: the cap, and the angles suggested by the top session

- hi − lo alone does not decide the result: probes 9 and 10 hold the width fixed (2.06, 2.62) and move
  lo by 0.01; whether and how much the ends are padded changes with the position (see the 2026-11-15
  span map). So lo and hi matter separately.
- The cap on N = min(n + 4, cap), read where it is pinned exactly (271 groups): no single outside limit
  explains it. At the cap the padded ends sit up to 8 steps outside the data in some groups and 0 in
  others, and one more mark adds a step on either side, with no common threshold in steps, in shares of
  the range, or in shares of the padded span (e.g. lo 0, hi 1.45: 24 allowed with 0.17 and 0.20 of the
  span outside, 25 refused at 0.21 and 0.20; lo 0, hi 0.0515, step 0.005: 16 allowed at 0.13 and 0.18,
  17 refused at 0.19 and 0.17).
- 24 is the most common pinned cap (lo 0 with hi 0.52–0.89 at step 0.05 and 1.43–1.88 at step 0.1,
  lo −1.03 with hi −0.02 and −0.205, lo 0.27 and 0.3 with hi 1.43–1.57, lo 3.13 with hi 5.39 where 24 is
  the label count). A fixed cap fits few cases: N = min(n + 4, 24) is in the fitting set for 1,253 of
  2,883 (23: 1,038; 25: 1,191; 30: 1,292); with "or the label count", 2,113.
- Not yet tried: values kept in shorter display formats (%9.0g, %10.0g, %12.0g) at some step; the axis
  labels `graph twoway` draws for the same ranges as a second view; a score comparison between two
  candidate counts that flips at n 12, 15, 20, 25.
- Scoreboard unchanged at 4,891.

## 2026-11-16: what StatCraft has

StatCraft (`C:\pinokio\api\statcraft`, `crates/graph/src/scale.rs`, `plan/stata/graph.md`, commit 0967751)
has the label rule RCraft already uses (the 0.67/0.33/0.12 score) and probes 224–227. It matched r()
to 1e-15 and accepted the last-bit residue ("Stata's carry the residue of its arithmetic"); it never
decoded the bits. Its notes list the same open edges: exact ties at n = 4 broken by last-bit
rounding, n = 2, `_natscale 5.12 14.99 5` (5 to 15 by 5) and `.0515 2.688 5` (0 to 3 by 1).
Its note that `_natscale` "reads its arguments as text" (`10.0` against `10`) was checked here
(probe 18): `10`/`10.0`/`20.0`, `0`/`0.0`, `12.910`/`12.91`, `1.4500000000000000`/`1.45` give the same
bits. The one pair that differed, `15.53` against `15.530000000000001`, is two different doubles one
ulp apart, which is the ulp sensitivity already seen in probe 13.

## 2026-11-16: graph twoway's axis labels are %18.0g of _natscale's r()

- Probe 19: 400 sampled swept cases with a non-exact step (d < 1, d ≠ 0.5), each run through
  `_natscale lo hi n` and through `twoway scatter y x, xlabel(#n) nodraw` with x = lo, hi as doubles,
  then reading `.g.xaxis1.major.min`, `.max`, `.delta` and `.g.plotregion1.xscale.curmin`, `.curmax`
  with `%21x`. `_natscale`'s side reproduced the pooled data in 400 of 400.
- The graph picks the same labels in all 400, but its bits differ from r() (same min in 216, same max
  in 196). Stata's `%18.0g` text applied to r() gives the graph's min, max and delta in 400 of 400
  each: up to 16 digits with the leading zero dropped and a sign slot kept, so 0.1 + ulp → 0.1,
  2.7755575615628914e-17 → 2.77555756156e-17, 280.25000000000006 → 280.2500000000001. The graph's
  scale range is the data range widened to the labels (398 and 397 of 400).
- So the graph is downstream of the same r() and carries no second view of the padded count. What it
  does give: matching a twoway axis needs r() only up to `%18.0g`. Over the 15,244 cases, L·d and U·d
  after `%18.0g` match Stata's r() after `%18.0g` at both ends in 9,133 (bottoms 12,125, tops 11,096),
  against 4,891 bit for bit; `%18.0g` erases the residue completely in 7,970.

## 2026-11-16: two candidate counts, and a new best on the scoreboard (5,563)

Fitting mark counts N (centred scale, Stata's step) computed for all 15,244 cases (`Sall.R`; some N
fits 14,589).

- At lo = 0 the count is a staircase in n, not n + 4: step 0.1, hi 0.85 gives N = 10 (the labels) for
  n 9–11, then 16–22 from n 12; hi 0.62 at step 0.05 steps at n 14, 20, 24 and 26. Read end by end,
  the top walk's start index at lo = 0 does not depend on hi: 15–17 for every family at n 15–19,
  18–22 at n 20–24 (step 0.1). So the top start follows a round count c(n) and N ≈ c(n) + 4, with
  c(n) the largest of 1, 1.2, 1.5, 2, 2.5, 3, 4, 5, 6, 8 × 10^k not above n. The later step (to N ≈ 30)
  moves with hi (n 24 at hi 1.36, 26 at 1.46) and is not monotone in n at hi 1.36 (28–33 at n 24,
  24–27 at n 25, 28–33 at n 26).
- N = c(n) + 4 is in the fitting set for 6,908 of 14,589 (n + 4: 6,483; with 1-2-5 counts: 5,332);
  "c(n) + 4 or the label count" for 10,963. The two candidates are the unpadded labels (A) and the
  padded c(n) + 4 set (B). A linear score in StatCraft's form (outside distance over the range, count
  distance over n, inside distance) picks the fitting one in at most 4,157 of the 8,397 cases where only
  one fits, below "always B" (4,342). A decision tree on gaps, n and the label count needs dozens of
  leaves. So no score flip between the two counts was found.
- On the scoreboard (bit for bit, both ends): the centred scale with N = the label count gives 5,519
  (bottoms 9,475, tops 6,955). Its top with the benchmark bottom (L·d, (L − 1)·d + d for an inside
  label, the on-label clamp) gives **5,563**, up from 4,891. Tops alone: N = max(cnt, c(n) + 4) gives
  8,004, and with N = cnt where the top label is inside, 8,440; those tops pair badly with any bottom
  tried (at most 5,485 both).
- Still far from bit for bit everywhere, so `natscale.rs` keeps declining non-exact steps.

## 2026-11-16: the padding trigger; scoreboard 5,866

Scripts `map0.R`, `cls.R`, `ctr.R`, `tabS.R` in `natscale-work/`.

- Map at lo = 0 (step 0.1, top label outside or on the data): families with top label U = 10, 11,
  13, 14, 16, 17 are padded at every n; U = 9, 12, 15 pad only from n 12, 15, 20; U = 8 never within
  n ≤ 14. Steps 0.05 and 0.2 show the same three-label period. A result whose bits the unpadded set
  also gives cannot say whether padding happened, so "unpadded" is only certain where the label count
  is the sole fitting N.
- Where `c(n) + 4` misses in padded cases, the nearest fitting N is mostly one off (878 at −1, 808 at
  +1). Centring variants (data midpoint with floor, round or ceil; label midpoint; padding split low or
  high) change the score by at most a few cases, so the off-by-one is not the centring's rounding.
- Ceiling for a two-candidate trigger: choosing per case between the unpadded set (centred top, benchmark
  bottom) and the padded `c(n) + 4` set would give 8,560 both ends; adding counts up to cnt + 6, 10,269.
- At lo = 0 the unpadded set wins where `c(n) + 4` is below the label count (U 19 at c 15, U 24 at 20,
  U 29 at 25) and for low tops (U 6–9 with c up to 10–12); the padded set wins elsewhere, with a few
  mixed cells (U 15 and U 18 at c 20).
- Rule found: top from the centred `c(n) + 4` set when c(n) ≤ the label count, else from the centred
  label-count set; bottom as the benchmark (L·d, (L − 1)·d + d for an inside label, the on-label clamp).
  **5,866 both ends** (bottoms 9,616, tops 7,481), up from 5,563. Thresholds on the excess
  c(n) + 4 − cnt other than 4 score 5,427–5,781.
- A parallel session (`plan/stata/natscale-literature.md`) is scoring published scaling algorithms
  (AS 21, 44, 96, 168, CACM 463, Dixon–Kronmal, Heckbert, Talbot et al.) on the same scoreboard.

## 2026-11-16: padding when the labels are near zero; scoreboard 7,365

Scripts `base5866.R`, `split.R`, `clampb.R`, `topJ.R`, `topN.R`, `padp.R`, `ttree.R`, `trig.R`,
`btrig.R`, `unify.R`, `rule7365.R` in `natscale-work/`.

- The 5,866 rule's misses: 1,615 bottom only, 3,750 top only, 4,013 both. Of the bottom misses, 3,033
  are on-label bottoms (lo on a label), where Stata gives a walked value, not lo: that is the lo = 0
  padding. Choosing per case between the two top candidates (unpadded and `c(n) + 4` centred) would give
  11,525 tops; the rule had 7,481.
- No single top start wins: (L + c(n))·d, L·d + c(n)·d, L·d + (n − 1)·d and others each get
  6,500–7,300 tops, each with about 3,000 the rule misses. A fixed pad below L (walk up p steps, top
  c(n) + p steps above) gives at most 4,244 both.
- A shallow tree choosing among five top candidates splits first on |U| (the top label's index): large
  |U| (≥ 24) is mostly unpadded; small |U| with the top label outside and n ≥ 12 is mostly padded.
  Depth 8 (68 leaves) reaches 10,841 of 12,716 decidable cases.
- Rule found (thresholds fitted, not yet explained): padding when c(n) ≥ 12 and the labels are near 0.
  Bottom from the data-centred `c(n) + 4` set walked up (no on-label clamp) when also
  max(|L|, |U|) < 2·c(n), else the benchmark; top from the label-centred `c(n) + 4` set when
  |U| < 2·c(n) and the top label is outside the data by at least 0.02 step, else the centred
  label-count set. **7,365 both ends** (bottoms 10,292, tops 9,936), up from 5,866. "Near 0" cutoffs
  of other shapes (|U| < 30, < 1.5·n, |L|, |U| < 20) give 6,530–6,555 for tops alone, so the exact
  form of that condition is open. The 12 is the first round count where padding shows at lo = 0.
