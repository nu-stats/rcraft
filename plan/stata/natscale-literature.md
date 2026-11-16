# _natscale's last bits against published axis-scaling methods (P2C.12)

A side study for `natscale-residue.md`, 2026-11-16. Question: does a published scaling method reproduce
Stata 19 `_natscale`'s r(min) and r(max) bit for bit where the step is not exact (d < 1, d ≠ 0.5), or
narrow the gap? Every method was written in R from its paper or a published description, or run as a
black box, and then scored on the shared scoreboard (`harness.R`: 15,244 swept results, both ends bit
for bit, the on-label clamp applied). The best rule when this study started was 5,563 (`scC.R`), and
7,365 by the time it ended (`rule7365.R`). Result: Lewart's CACM 463 is the closest; its pieces lift
7,365 to 7,781. No Stata file
was read. No new Stata probe was needed. The scripts are `lit_*.R` in the shared `natscale-work/` folder.

## Sources

- Lewart, CACM Algorithm 463, SCALE1/SCALE2/SCALE3 (1973): read from the user's PDF (text and Fortran
  listing).
- Heckbert, "Nice numbers for graph labels", Graphics Gems (1990): read from the user's ScienceDirect zip
  (unzipped into its own folder; the zip holds 100 Graphics Gems chapters, and this one is the only
  one on scales).
- Stirling, AS 168 (1981): the published listing on StatLib (`lib.stat.cmu.edu/apstat/168`), read for
  me as a prose description of the steps. The journal's text was not reached.
- Nelder, AS 96 (1976): as before (`as96.R`), plus `labeling::nelder`.
- Thayer & Storer AS 21 (1969), Sparks AS 44 (1971), Wilkinson (Grammar of Graphics), and Talbot, Lin &
  Hanrahan (2010), "extended": the originals were not reachable (StatLib 404 for 21 and 44, ACM 403,
  and vis.stanford.edu's certificate chain is untrusted here, so it was not bypassed). They were run
  through CRAN `labeling` 0.4.3 (Talbot's MIT-licensed implementations of these methods, already in the
  user's R library) as black boxes, with their help pages as the description. Later the user supplied
  the AS 21, AS 44 and Talbot et al. PDFs, and each was then written in R from its paper (the last
  section).
- Dixon & Kronmal (1965), JACM: not reachable at first. The user later supplied it, and the parent session
  scored it, with AS 168's journal text and Giammo (1964), in `natscale-residue.md`.

## Scores (both ends, bit for bit, of 15,244; 5,563 was the best when this table was made)

"As written" uses the method's own labels with m = n marks. "Walked" moves each of the method's end
labels by Stata's step d, one live step at a time, to Stata's L and U. That counts only when the
method's end lies on Stata's grid, so a method that picks another step loses the case. "Some m" is the
best any m in n − 1 .. n + 4 can do (an oracle, not a rule).

| Method | As written | Walked, m = n | Best fixed m, walked | Some m | Verdict |
|---|---|---|---|---|---|
| Lewart SCALE2, Stata's step forced, unpadded (N = NP), walked | | **6,047** | | 14,364 (any N) | New best, +484; same bottoms as the benchmark, better tops |
| Lewart SCALE2 as written (Stata's 10^k) | 761 (N = n − 1), 513 (N = n) | | | | Its steps carry Stata's bit forms; its labels are not Stata's |
| Lewart SCALE1 as written (Stata's 10^k) | 2,924 (N = n − 1) | | | | Encloses the data, which `_natscale` does not |
| Stirling AS 168, own R (Stata's 10^k) | 683 | 2,268 | 2,577 (n + 1) | 5,415 | Its units are the c(n) staircase; its scale is not Stata's |
| Nelder AS 96 (`labeling`) | 493 | 1,463 | 1,472 (n − 1) | 4,096 | As before (`as96.R`: at most 1,389) |
| Nelder AS 96, 1-2-5 units | 400 | 3,530 | 3,692 (n + 1) | 7,602 | Below the benchmark (4,891) |
| Thayer & Storer AS 21 (`labeling`) | 491 | 2,031 | 2,089 (n + 1) | 5,276 | No |
| Sparks AS 44 (`labeling`), on the 4,053 cases it finished | 0 | 482 | (m = n only) | | No; the implementation hangs on ranges below about 1 |
| Heckbert, loose (`labeling`) | 1,820 | 3,658 | 3,698 (n + 1) | 4,060 | No |
| Heckbert, own R from the chapter (ticks by adding d; nearest / Stata's 10^k) | 753 / 706 | 2,205 / 2,547 | 2,279 / 2,570 | 2,759 / 3,523 | The accumulated top is worse than a product |
| Wilkinson (`labeling`; Q as published / 1-2-5) | 1,989 / n.a. | 3,273 / 3,232 | 3,273 / 3,305 | 3,704 / 3,582 | No |
| Talbot, Lin & Hanrahan extended (`labeling`; Q as published / 1-2-5 / 1-2-5 loose) | 1,837 / 1,703 / 1,560 | 3,671 / 3,561 / 3,386 | 3,744 / 3,742 / 3,448 (n − 1) | 4,459 / 4,256 / 3,729 | No |
| Dixon & Kronmal (1965) | not scored | | | | Paper not reachable, no faithful description found |

The benchmark (L·d and U·d, the on-label clamp, (L − 1)·d + d for an inside bottom label) is 4,891.
Every method that picks its own step and labels scores below it. Only Lewart's arithmetic, given Stata's
step, beats the earlier best.

## Lewart (CACM 463) is the closest model

What Lewart's SCALE2 does (my reading of the listing): A = range/N; NAL = trunc(log10 A), one lower if
A < 1; B = A/10^NAL; the first of 1, 2, 5 with B < unit + DEL, else 10; DIST = unit·10^NAL. The ends
are gated: MI = floor(XMIN/DIST), raised by one when XMIN/DIST is within DEL below the next integer;
M2 likewise from above. If M2 − MI > N, a second pass moves to the next unit (the list goes on to 20).
Then it pads: NX = (N − NP) div 2, XMINP = DIST·MI − NX·DIST, and XMAXP = XMINP + N·DIST, a sum and
not a product. Finally XMINP = XMIN if round-off put it above the data, and XMAXP likewise.

Four things in it match what was observed of `_natscale`:

- **Step bits.** With Stata's 10^k (a product of tenths), SCALE2's own step has Stata's exact bits in
  13,964 of 15,244 with N = n and 14,228 with N = n + 1; with the nearest double for 10^k, 2,463. Its
  unit list (1, 2, 5, 10, 20) can only make the forms Stata shows. A 1 × 10^k step comes only when B
  is within DEL of 1, so 0.1 is in practice always 10 × 10^-2, as in Stata. 5 is always 5 × 10^k. 0.2
  is 2 × 10^-1 on a first pass and 20 × 10^-2 on a second pass past 10, which explains why Stata has
  both. SCALE2 does not decide which 0.2 a case gets, though: 2,464 of 3,043, against 2,680 for
  "always 2 × 10^-1". Neither does 10 × range split them by n (`lit_steps.R`).
- **The on-label clamp.** A gate of DEL on x/DIST, then clamping to the data, is the shape of Stata's
  clamp (r(min) = lo when lo/d is within about 2e-12 below a label). DEL = 2e-12 scores 6,047, Lewart's
  own 2e-5 scores 6,044.
- **Top as a sum.** XMAXP = XMINP + N·DIST is the "bottom label plus span" form seen in the cleanest
  families. With Stata's step and no padding (N = NP, so XMAXP = MI·d + (M2 − MI)·d), walked down to
  Stata's top label, it fits 7,523 tops. The centred form fitted 6,955, U·d 6,129. Its bottom, MI·d
  walked up to L, is the benchmark bottom (9,616). Both together: **6,047** (`lit_lewart5.R`). That is
  a complete rule, not a mix.
- **Padding split around the labels.** NX = (N − NP) div 2 below and the rest above is the
  "padding split around the labels" form of the 2026-11-15 notes. For some N, SCALE2's padded ends
  (Stata's step forced) walk to both of Stata's ends in 14,364 of 15,244. The centred form fits
  14,589, and the two together 14,944. On 3,000 sampled cases it fits 2,824, and 1,191 with the
  targets moved one ulp (1,895 against 947 on the cases that are not plain products), so the fit is not
  chance (`lit_lewart_chance.R`).

What Lewart does not give is N. SCALE2 takes N as given, and no fixed N does better than unpadded:
N = max(n + k, NP) scores 5,252 to 5,358 for k from −1 to 3, and 4,908 with c(n) + 3. Even a perfect
choice between unpadded and one padded N reaches only 8,934 (unpadded or n + 3; `lit_lewart5.R`). So
what decides the padding is still open, and N is not a function of n alone.

## The round-count staircase

c(n) in 1, 1.2, 1.5, 2, 2.5, 3, 4, 5, 6, 8 × 10^k is exactly Stirling's AS 168 unit list (12, 15, 20,
25, 30, 40, 50, 60, 80, 100, 120, 150). AS 168 uses it for the step: the first unit at or above
range·(1 + 2·bias)/n, scaled into (10, 100]. Nelder's AS 96 list differs only at 1.6, and the
others use 1-2-5 (Lewart, Heckbert), 1-5-2-2.5-4-3 (Talbot) or a longer list (Wilkinson). None of the
papers uses round counts for the number of marks.

AS 168's step choice is not the padding trigger (`lit_as168_pad.R`). Where padding is forced, AS 168
picks a step finer than d in 5,489 of 7,968; where the unpadded set fits, in 3,610 of 6,621. At lo 0,
hi 0.85, step 0.1, it drops from 0.1 to 0.08 at n 11, while the padding starts at n 12.

## Smaller notes

- Heckbert writes that he computed 10^n by multiplying or dividing in a loop because pow was inexact
  then. Stata's 10^k for k < 0 is such a loop of tenths.
- Heckbert's ticks are made by adding d from the bottom label; that top, walked to Stata's top label, fits
  only 3,284 tops (Stata's 10^k), below U·d's 6,129.
- `labeling::sparks` never returns for every range below 1 that I tried (`sparks(0, 0.0103, 10)`,
  `(0.3, 0.348, 8)`, `(5, 5.09, 10)`, …), and gives 0 to 4.5 for 0 to 0.9. So Sparks was scored only
  on the first 4,053 cases (lo = 0, widths 0.51 to 2.2), before the first hang. AS 44's text was not
  reachable to check it against.

## Lewart's pieces in the 7,365 rule: 7,781

While this study ran, the parent's rule reached 7,365 (`rule7365.R`: padding near zero from c(n) 12).
Putting Lewart's forms into it (`lit_on7365.R`, `lit_rule7781.R`):

- Its unpadded top (the centred label-count set, 6,956 tops), replaced by Lewart's unpadded top
  MI·d + NP·d walked down (7,523 tops): **7,710** both ends (tops 10,472).
- Also its padded bottom (the data-centred c(n) + 4 set, used when Pb), replaced by Lewart's padded
  bottom MI·d − NX·d with N = c(n) + 4 intervals, NX = (N − NP) div 2, walked up with no on-label
  clamp: **7,781** both ends (bottoms 10,429, tops 10,472). With c(n) + 3 intervals: 7,766; c(n) + 2:
  7,561.
- Lewart's padded top in place of the label-centred c(n) + 4 top (when Pt) is worse (7,449 at best), so
  that piece stays.

## For the translator

`natscale.rs` was not touched. Lewart's SCALE2 is the published method closest to `_natscale`: its
step-bit forms, its gate-and-clamp, its top as a sum, and its padding split all match what has been seen.
Alone, with Stata's step and no padding, it scores 6,047. Inside the current rule its top and its padded
bottom lift the scoreboard from 7,365 to **7,781**. That is still far from bit for bit everywhere, so
declining non-exact steps stays right. The open question is unchanged: which N (padding) Stata uses.
No paper read chooses N the way `_natscale` seems to.

## Is the "near zero" padding condition a published rule? (`lit_zero.R`, `lit_on5866.R`)

- Against 5,866 (`ctr.R`'s rule): Lewart's unpadded top in place of the centred label-count top gives
  5,960; Lewart unpadded alone (benchmark bottom) 6,047.
- The only published rule that depends on where zero sits is Nelder's AS 96 zero rule. It sets the lowest
  mark to 0 when the data are non-negative and the scale's span reaches down to 0, and mirrors this for
  non-positive data. Put in place of the fitted max(|L|, |U|) < 2·c(n) inside the 7,781 rule (labels
  straddling 0, or 0 within the span; span c(n) + k or n + k steps, k 3 to 8): at most 7,735 with the
  c(n) ≥ 12 gate, and 7,052–7,185 without it. So the gate at 12 is needed, and the zero rule does no
  better than the fitted condition.
- None of the papers read starts anything at a mark count of 12. The nearest coincidence is AS 168's
  unit list, which starts at 12 within its decade (12, 15, …, 100), the same as c(n) ≥ 12 ⇔ n ≥ 12.
  The condition stays fitted, not explained.

## From the papers themselves: AS 21, AS 44, Talbot et al. (`lit_paper.R`)

The user supplied the three papers. Each was written in R from its text and listing, and checked on
the papers' own examples: AS 21 gives 0 to 2 by 0.5 for 0.37–1.92 (4 intervals) and −28 to −4 by 4 for
−25.6 to −5.51 (6). Extended gives 8 to 14 by 2 for 8.1–14.1 (m 4), as the package does. m marks means
m − 1 intervals. Each was run with Fortran-style 10.**K (exact powers, dividing) and with Stata's 10^k
(a product of tenths). The scores compare against 7,781.

| Method (own R) | Step bits = Stata's d (m = n) | As written | Walked, m = n | Best m in n − 1..n + 1 | Some m |
|---|---|---|---|---|---|
| Thayer & Storer AS 21, 10.**K / Stata's 10^k | 1,065 / 4,694 | 162 / 195 | 1,241 / 1,206 | 1,313 / 1,296 | 2,186 / 2,280 |
| Sparks AS 44, 10.**K / Stata's 10^k | 1,058 / 3,856 | 530 / 577 | 1,780 / 2,258 | 1,915 / 2,512 | 3,739 / 4,683 |
| Talbot et al. extended, paper weights, 10.**K / Stata's 10^k | 1,462 / 4,565 | 2,019 / 2,579 | 3,396 / 4,215 | 3,503 / 4,215 | 3,938 / 4,948 |
| extended, Stata's 10^k, Q = 1, 5, 2 | 5,416 | 2,617 | 4,267 | 4,332 | 4,844 |
| extended, Stata's 10^k, `labeling`'s weights | 5,295 | 2,496 | 4,560 | 4,560 | 5,319 |
| Wilkinson as Talbot et al. describe it, 10.**K / Stata's 10^k | 1,185 / 5,733 | 2,086 / 2,482 | 3,575 / 4,483 | 3,575 / 4,483 | 3,752 / 4,793 |

Verdicts: none comes near 7,781, or the 4,891 benchmark. The best is extended at 4,560 walked.

- **AS 21:** the minimum is truncated at the digit where the range lies in (1, 10]. The interval is the
  next whole digit up, with 7 and 9 skipped and 1.1–1.5 taken as 1.5, so its steps are 1.5, 2, 3, 4,
  5, 6, 8, 10. A second pass carries one more digit when coverage is below 0.8. The labels are made by
  adding the interval to the minimum. That accumulated top is its weakest part (tops at most 1,882).
- **AS 44:** Thayer–Storer with Lauckner's (1971) correction: steps 1.5, 2, 3, …, 10, with 7 and 9
  allowed. The second pass ends after one try. The labels are products XMIN + k·XSTEP.
- **The `labeling::sparks` hang is the port's, not the paper's.** My own AS 44 terminates on every
  case. The port divides where the paper multiplies: its ratio is (dmax − dmin) × (fm·fstep) where the
  paper has RATIO = (FMAX − FMIN)/(FM·FSTEP). For small ranges that ratio never passes 0.8. The port
  also has no "stop after the second pass" exit, so it loops forever. It reruns the second pass
  without resetting the range and minimum, and its last label is fstep·(m − 1) instead of
  XMIN + FM·XSTEP (hence 0 to 4.5 for 0 to 0.9). The earlier Sparks row (0 and 482, on 4,053 cases) is
  therefore the port's, not AS 44's; the row above replaces it.
- **Talbot et al.:** the paper's weights are 0.2 simplicity, 0.25 coverage, 0.5 density and 0.05
  legibility; `labeling`'s default swaps the first two. With Stata's 10^k the swap scores a little
  better (4,560 against 4,215). Its labels are lmin = start·lstep/j and lmax = lmin + (k − 1)·lstep, a
  sum like Lewart's. With Stata's step and labels that form is L·d + (U − L)·d, which was already scored
  (5,562 with the benchmark bottom, `lit_lewart4.R`), below Lewart's MI·d + NP·d top (6,047).
- **Wilkinson:** built from Talbot et al.'s description (the book's Q = 1, 5, 2, 2.5, 3; loose
  labelings; simplicity, coverage with a 0.8 floor, and granularity summed). It differs from
  `labeling::wilkinson`, which follows Wilkinson's Java implementation (8.1–14.1, m 4: mine 7.5 to 15 by
  2.5, the package 8 to 15 by 1).
- The related-work section names Thayer & Storer, Sparks, Nelder, Stirling and Heckbert. It does not
  cite Lewart or Dixon & Kronmal, and none of the methods it covers chooses a padded mark count.
- AS 168 from the journal text (divisor reset fixed) and Dixon & Kronmal are in `natscale-residue.md`
  (parent session): AS 168 walks to 2,349 at m = n and 2,658 at n + 1; Dixon & Kronmal's centring with
  a floor fits some S in 13,516, against 14,589 for the rounded centring. Neither beats 7,781.
