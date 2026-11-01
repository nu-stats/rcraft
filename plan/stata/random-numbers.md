# Stata's random numbers on R (P2C.11)

Written 2026-11-01. `set seed`, `runiform()`, `rnormal()` and `rgamma()` now give Stata 19's own draws when RCraft
runs Stata on R. Code: `crates/statatr/src/data_r/random.rs` (the R helper `stata_rng` and the translator's side)
and `crates/statatr/src/expr_r.rs` (`translate_drawing`). Checked by `crates/engine/tests/stata_corpus/random.do`.

## The decision: the draws come from R

Two ways were weighed.

- **(a) Stata's generator written in R**, in the helper the translated script defines once.
- **(b) The generator in Rust**, the do-file runner asking R how many rows draw (`Need`/`Ask`) and handing R the
  numbers.

(a) was chosen:

- **The R stands on its own.** Every R line RCraft shows can be copied and run in plain R (`AGENTS.md` section 3).
  With (a), a saved R script or Quarto file reproduces Stata's draws by itself. With (b), the numbers would have
  to be written into the script (megabytes for a large `set obs`) or read from a file that does not outlive
  the run.
- **The state follows the do-file by itself.** The generator's state lives in R (`stata_rng_state`), so it goes
  through every line, `preserve` and `restore`, `by` groups and typed lines without the runner keeping a copy
  in step. With (b), the runner would have to ask R for each line's selected rows, and for each row's
  arguments where they are variables (`rnormal(x, s)`, a shape that varies for `rgamma`), before it could draw.
- **Values R works out need nothing new.** `local r = runiform()`, `scalar s = rnormal()` and
  `if runiform() < .5 {` already go to R through the existing `Need::Value` path. R draws once, out of sight,
  and the run goes on with the answer.

What (a) costs:

- **No unsigned 64-bit integers in R.** Each 64-bit word is four 16-bit pieces in integer vectors.
  `bitwXor`/`bitwAnd` work on them, and shifts move bits between the pieces. The twist is done in three
  vectorised blocks (words 1–156 from the old state, 157–311 using the new 1–155, then word 312).
- **Speed.** A million uniforms take about 0.6 s on this machine, and a million normals about 3.7 s. Most
  normals take exactly three uniforms, so they are worked out for whole vectors. Only the rare wedge and tail
  draws go one at a time. `rgamma()` goes one draw at a time (a few hundred thousand a second for shape 2.5).
- **R's own exp and log.** On Windows they differ from the C runtime's (Rust's, and Stata's) in about 0.1–0.5%
  of calls, by one unit in the last place (1 million arguments compared). `sqrt` and arithmetic agree. In
  practice this did not show: all 240,000 draws of the check below matched bit for bit on Windows. A
  last-place difference could only flip a rejection test in a near-tie, or change a gamma draw in its last
  bit. The corpus file compares exactly (tolerance 0) on all three CI platforms, so a platform where it shows
  will fail there and be seen.
- **No `cumsum` or `sum`.** R adds those in long double. The G4PE areas are added one by one in double, as
  Stata does.

## How the draws follow the do-file

- `stata_rng_state` is made at Stata's starting seed (123456789) the first time it is used, as in a fresh Stata.
  A do-file run again in the same R session goes on from where the last one left off, as it would in the same
  Stata session. `set seed #` translates to `stata_set_seed(#)`, which starts again from |#|.
- In data (generate, replace, expand, keep, drop), the helper is given the rows that draw: `dplyr::n()`, or the
  `if`/`in` condition as a logical vector. It draws one value per selected row, in row order, and leaves the
  other rows missing. After `group_by` (`by`), dplyr evaluates the groups in order, so the draws go in data
  order, as Stata's do.
- In display, scalars and macro values, it draws once (`stata_runiform(1)`), even when the expression reads the
  data (`display x + runiform()`: Stata uses the first row and one draw).
- Where the translated R writes a value twice (the missing rule of a comparison, `round()`), the draw is taken
  once and named: `local({.stata_draw <- stata_runiform(dplyr::n()); (!is.na(.stata_draw) & .stata_draw < 0.5)})`.
- The variates draw a pool of uniforms ahead and put back the ones they did not use, so the next draw gets them
  in order.
- Typed Stata lines (`statatr::live`) replay only the translation, never R, so the state in R is never run
  twice. A typed `local r = runiform()` is worked out by R out of sight and takes its draw once (tested in the
  app with a real R).

## Refused, with the reason

- Two random draws in one expression (`runiform() + rnormal()`), or in both the value and the `if`. Stata takes
  them in turn for each row.
- A draw in an `if` together with `in` (not observed).
- Draws anywhere else (`summarize if runiform() < .5`, `egen`, the models' `if`).
- The other random functions (`runiformint()`, `rbinomial()`, `rpoisson()`, `rt()`, `rbeta()`, ...).
- **After a line that may have drawn and was not translated**, the draws are refused until the next `set seed`.
  Such a line names a random function, is a command that draws (`bsample`, `bootstrap`, `simulate`, `permute`,
  `sample`, `splitsample`, `mi`, `lasso`, `cluster`, `mata`, ...), mentions `bootstrap` (`vce(bootstrap)`), is
  `set seed`/`set rng...` not translated, or runs a program the do-file defines.
  - Not covered: an add-on (ado) command that draws while it is refused for another reason.
- **Under `version` 13 or earlier.** Stata then draws from its older generator, KISS32. Stata 19 gives
  `uniform()` a different value after `set seed 5` under `version 13`.

## Observed in Stata 19 SE (made-up data, 2026-11-01)

Each case was followed by a `runiform()` marker, which shows how many uniforms the case used. The logs stayed
in a scratch folder and are not kept.

- A missing argument, `runiform(a, b)` with a ≥ b, `rnormal(m, s)` with s < 0, and `rgamma(a, b)` with a ≤ 0
  or b ≤ 0 give missing and use no uniform. `rnormal(m, 0)` is m and uses none. This holds row by row: a row
  whose argument is missing does not draw, and the next row takes the next draw.
- `generate ... if` and `in`, `replace ... if`, and `expand ... if` draw only for the rows selected, in order.
  `by g: ... if _n == 1` draws once per group, in data order. `by` without the data sorted stops with
  `not sorted`.
- `cond(c, runiform(), y)` draws for every row. So do `c & runiform() < x` and `c | ...`: there is no short
  cut.
- `display x + runiform()` draws once. The pieces of one `display` draw left to right. `quietly display` draws.
- `sort`, `preserve` and `restore` do not draw. `set seed -1` gives the same draws as `set seed 1`, and
  `set seed -7` the same as `set seed 7`.
- `runiform(a, b)` is a + b·u − a·u, worked left to right. All 20,000 draws of `runiform(-3, 7)` match that
  bit for bit; a + (b − a)·u differs in the last place in 13,617 of them.
- `uniform()` is `runiform()` under `version 19`.
- A G4PE left-tail point at or below 0 (`rgamma(a)`, a > 1) is turned down **before** the uniform for its
  height is drawn: the next uniform starts the next try. Found at the 18,482nd `rgamma(2.5, 1)` draw after
  seed 20261101 and the draws before it. Stata gave 2.5966848742675825, the next uniform's rectangle point.
  StatCraft's `gamma_g4pe` draws that uniform first, so StatCraft differs there (noted for StatCraft).

The check: after `set seed 20261101`, 20,000 rows each of `runiform()`, `rnormal()`, `rgamma()` with shapes .05,
.5, 1, 2.5, 30 and 700, `runiform(-3, 7)`, `rnormal(10, 3)`, a float `rnormal()` and a closing `runiform()`.
Stata's values were exported in `%21x` and compared with R's: all 240,000 are the same, bit for bit, on
Windows.

## Not done

- `runiformint()`, `rbinomial()`, `rpoisson()`, `rchi2()`, `rbeta()`, `rt()` and the others. StatCraft has
  several exactly (`plan/stata/random-numbers.md` there). `stata_rng_words()` already gives the raw 64-bit
  outputs that `runiformint()` needs.
- `c(rngstate)` and `set rngstate`, `set rng`, `set sortseed`. KISS32 under `version` 13 or earlier.
- Two draws in one expression: they would need a row-by-row helper.
- `bsample`, `bootstrap`, `simulate`: they draw, and they are not translated.

## `bsample` (P2C.16)

The rule is StatCraft's decoding (`plan/stata/resampling.md` there, probes 177 to 191), written in R as
`stata_bsample` beside the generator (`data_r/random.rs`):
- Each of the N units draws k, then each draws v (2N uniforms). Unit i falls in bin floor(N·k), and a
  bin's whole count goes to the unit in it with the largest v.
- The units drawn come in ascending k, their extra copies after them.
- Below N (`bsample n`), a third uniform each keeps the n rows with the smallest, in that order.
- With `cluster()` the counts go elsewhere, through the clusters' ranks by (bin, v).

Two rules were observed here in Stata 19 (`q9`, 280 samples, each with the next uniform after it), and
all 280 match:
- **`if`:** with `if`, every row of the data still draws (2·_N uniforms), and the rows selected take the
  first ones in order.
- **`idcluster()`:** each cluster's copies stay together, numbered in cluster order; without it, extra
  copies follow.

`bsample.do` in the corpus checks the rows, their order and what follows (18 results, including five
draws in a `preserve`/`restore` loop). Not translated yet: `bsample` with `if` and a size or `cluster()`
(not observed), `strata()`, `weight()`, and the `bootstrap` prefix.
