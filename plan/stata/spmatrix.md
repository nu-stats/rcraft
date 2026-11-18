# spset, spmatrix (translated) and spregress (on hold)

Sources: StatCraft's `plan/stata/spatial.md` (probes 730-737, its PR #23) and our own probes on
made-up data in Stata 19 SE, 2026-11-18. Translated in `crates/statatr/src/data_r/spatial.rs`;
corpus `crates/engine/tests/stata_corpus/spmatrix.do` (43 results, 1e-13).

## spset

As StatCraft decoded it, and checked again: `spset id, coord(x y)` adds `_ID` (id's values and
format, label "Spatial-unit ID"), `_CX`, `_CY` and prints the block. Seen here as well:

- `spset, coord(x y)` on data not spset: "option modify required" r(198).
- An id that repeats: "variable _ID does not uniquely identify the observations" and "    Do these
  data need to be xtset?" r(459).
- `spset id` alone: "Coordinates: <none>"; `spmatrix create idistance` then says "coordinate
  variables must be numeric" r(498).
- `coord(x)`: "coord():  too few variables specified" r(102).
- After `clear` with nothing in memory: "no dataset in use" r(3); `spmatrix create` says "data not
  spset" r(459).
- `coordsys(latlong)` changes the block to "_CY, _CX (latitude-and-longitude, kilometers)": refused.
- `spregress` uses `_ID`, not the id variable: changing id after spset changes nothing.

## spmatrix

- Matrices outlive `clear` (dir lists them after it). RCraft keeps them in R (`stata_spw`).
- Last bits: Stata's inverse distance is what R computes as `1 / exp(log(d2) / 2)`, not
  `d2^(-1/2)` or `1 / sqrt(d2)` (those miss by 1 ulp on dyadic coordinates); summarize's mean is
  `sum(rowSums(W) / n) / n`. Spectral normalization divides by the largest eigenvalue modulus; R's
  general solver gave Stata's bits under R 4.6.2 but differs by a few ulps under R 4.4.3, so the
  corpus allows 1e-13.
- A failed `summarize` empties r() (r(n) is missing after it).
- `fromdata`: a matrix whose diagonal is not all 0: "matrix main diagonal must all be zero" r(498);
  not square: "matname: number of rows must be equal to number of columns" r(498) (StatCraft had
  3200).
- `create contiguity` without a shapefile: Stata's two-line message, r(601) (not 459).
- `note W, drop`: "invalid syntax" r(198); a second note replaces the first.
- `save`/`use`: Stata's .stswm format is its own; RCraft writes R's serialization under the same
  name, so a matrix saved and used by the same do-file works, a file from Stata does not.

## spregress, ml: on hold

The likelihood, grid start, table, messages, e() and predict were written and checked against
probe 732 (the patch and probes are in the session notes,
`side-session-notes/local_5e7d5349.../spregress-ml-partial.patch` and `spatial-probes/`). The
estimates cannot be matched yet, so `spregress` is refused:

- Stata 19 does not report the likelihood's maximum. Its concentrated stage takes Newton steps with
  d0 numerical derivatives and stops by ml's rules; on a row-normalized lag model (probe s3) it
  stopped after one step at rho = .6991283 where the maximum is .6993025 (4th digit); on probe 732
  it is 2.7e-7 off (7th digit).
- `errorlag()` with a spectral-normalized W reached rho = 6.07 (ll -45.63, above the -59.25 at the
  maximum inside the usual range): Stata's likelihood apparently takes ln|det(I - rho W)|, and its steps leave
  (-1/|lambda_min|, 1/lambda_max).
- The derivatives (Mata optimize/deriv, traced on toy d0 functions): the first step is
  h = 1e-3 (|b| + 1e-3); a relative change of f between about 1e-4 and 1e-3 is accepted;
  otherwise h is scaled linearly toward a change of 5.5e-4; g and H are the central differences
  with the accepted h (this gave the first Newton step's point to 15 digits). After an overshoot
  the next h follows a rule not decoded (13.739 -> 13.361 -> .583 -> 1.078 in one trace).
- To finish: decode that search (more traces with optimize_evaluate on linear and quadratic f),
  then follow Stata's path: the grid, the concentrated Newton steps with these derivatives and
  ml's convergence (nrtol 1e-5, ltol 1e-7, ptol 1e-6), ln|det|, the unconcentrated stage (its
  backed-up step), e(iterations) = the unconcentrated stage's lines, e(ilog) its log likelihoods.
