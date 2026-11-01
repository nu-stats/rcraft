# Errors under capture, the normal distribution, `_natscale`, and lines a refused line may have changed

Observed in Stata 19 SE on this machine, with made-up data (P2C.12, 2026-11-01). Only what Stata printed
was looked at.

## `capture` and `capture noisily`

- A name that is neither a variable nor a scalar, in `display`, `generate`, `replace` or `keep if`, gives
  `_rc` 111. Under `noisily` Stata prints `name not found` (for example `a not found`) and nothing else: no
  `r(111);` line.
- `summarize nosuch` and `drop nosuch` print `variable nosuch not found`, also 111.
- `scalar(zz)` for a scalar that is not there prints `zz not found`, 111.
- `generate x = 1` when `x` exists prints `variable x already defined`, 110.
- `display log(` prints `too few ')' or ']'`, 132.
- `display r(nosuch)` prints `.` and `_rc` is 0.
- `display "a" + 1` prints `a1`, with `_rc` 0 (noted, not translated).

RCraft: under `capture`, an R error that says a name is not there (R's `object 'a' not found`, or a
helper's `... not found`) gives `_rc` 111. A helper's `variable ... already defined` gives 110. Under
`noisily`, Stata's message is printed: `a not found`. Any other R error keeps `_rc` missing, and under
`noisily` R's own message is printed.

## `invnormal()` and `normal()`

Stata's values for 20,000 arguments were exported in `%21x` and set against R's. The arguments were:
- `p` from `runiform()`;
- `p^8` and `1 - p^8`;
- `1e-300 · u`;
- `z` from `3 · rnormal()` and uniform on −38 to 38.

The results:
- `invnormal(p)` matches `stats::qnorm(p)` bit for bit. It is missing for p ≤ 0, p ≥ 1 and missing p.
- `normal(z)` matches `stats::pnorm(z)` bit for bit, tails included: `normal(-37.5)` is
  `4.6053530095819552e-308`, `normal(-40)` is 0 and `normal(40)` is 1.
- `normalden(z)` and `lnnormal(z)` agree with `dnorm` and `pnorm(log.p = TRUE)` only to about 4e-9 relative
  in the far tails. They are left untranslated.

## A line that was not translated

- **A model.** A refused line that may be an estimation command (`qreg`, say) replaces the model Stata
  holds. `estat`, `predict`, `test`, `lincom` and `margins` after it are refused until a model is
  translated again, as `e()` already was. Before this, `capture noisily estat ic` after `qreg` ran on the
  model before it, so `_rc` was 0 where Stata gave 321.
- **A global.** An add-on, or a program the do-file defines, may set any global; `mtable` sets
  `$mtable_cspec`. After such a line is refused, a global the do-file has not set is refused rather than
  read as empty. Stata's own commands that are refused do not do this.

## `poisson` to 8 digits (not done)

Probe 064's `poisson y x1 x2` agrees with Stata only to 6 digits. Stata logs two iterations; its answer
is one Newton step (score and observed information) from the coefficients that `iterate(0)` reports.
That step, done in R from those coefficients, gives Stata's coefficients bit for bit. The maximum R
reaches differs from Stata's in the eighth digit.

What is not known yet is where Stata's starting coefficients come from. Their log likelihood is already
−379.33336. These were tried and none gives them exactly:
- IRLS from y + 0.1, y + 0.5 or (y + ȳ)/2, one to four steps;
- Newton from the constant-only model;
- OLS of ln(y + .5).

The nearest, three IRLS steps from y + 0.5, is about 1e-5 away. A one-step model needs its start found
by observation, on several made-up data sets, before `poisson` can match to 8 digits.

## `_natscale`

The rule is StatCraft's, decoded from Stata's answers alone (`plan/stata/graph.md` in StatCraft). RCraft
works it out when it translates the line and leaves r(n), r(min), r(max) and r(delta) in R as exact
doubles.

Set against the 4,915 sweeps of probe 226, with the values as L·d, U·d and d:
- r(delta) is q · 10^k bit for bit in 4,804 cases;
- r(min) is L·d in 4,803;
- r(max) is r(min) + (labels − 1) · d in 4,648.

In 112 sweeps, almost all with n = 10 or 15, Stata's r(min) is not 0 but 2^-55 or −2^-54. Its r(delta)
there is one unit above q · 10^k (0.1 as `0x1.999999999999bp-4`). So Stata computes those steps another
way, not yet found. Those lines differ only in that residue, but a residue against 0 fails any relative
check, so they count as different.
