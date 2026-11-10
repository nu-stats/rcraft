# xtreg, fe: which within-collinear column Stata omits (2026-11-09)

Follows `xtset-on-use.md`, where the choice looked like rounding alone. Observed in Stata 19
(probes in the session scratchpad, `fecol2`), never from Stata's own files.

## Two paths, chosen by the vce

- Plain, `vce(robust)` or `vce(cluster panelvar)`: the later column of the set in the order
  written, "note: 168.FOMCMEETING_id omitted because of collinearity." TZEN38's first model
  without its cluster option omits 168; with the dummies written first, age. 16 made-up panels
  (15 to 64 panels, 30 to 165 meetings, age = year - birth, eight meetings a year): the last
  meeting each time. This is what RCraft did already.
- `vce(cluster v)` with v not the panel: "note: 167.FOMCMEETING_id omitted because of collinearity
  with the absorbed variable." `areg ..., absorb(panel)` does the same with vce(cluster v) and
  takes the plain path without it.

## The absorbed path

The within cross products are swept, the column with the most left next (absolute, as
`regress` sweeps); the column left with nothing at the end is omitted. Without ties it gives
Stata's column in all 9 of TZEN38's untied models and all 13 untied made-up ones (16 made-up sets
clustered on ceil(id/3)).

A tie: the last two contenders are left with the same in exact arithmetic when their
coefficients in the dependency are equal (meetings of one year, age = year - birth). Stata's
rounding then decides, and it went either way: in 16 tied cases it kept the later column 8
times and the earlier 8 times. Order-dependent double sums in panel and time order, three ways
of centring and three of sweeping, did not reproduce its rounding (best 8 of 13 TZEN38 ties).
RCraft keeps the later (omits the earlier) on a tie (relative gap under 1e-12) and prints a note
naming the coefficients that may differ; with no tie, no note.

TZEN38's 22 fe models with age, clustered on BankGovernor_round_id: Stata's column in 17 (it was
7); the 5 others are ties. Corpus `xtabsorbed.do`: a made-up case without a tie, absorbed,
clustered on the panel, and plain (102.meet against 104.meet).
