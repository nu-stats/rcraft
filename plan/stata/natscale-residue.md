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
