# qreg (2026-11-19)

Observed on Stata 19 SE with made-up data (corpus `qreg.do`, 51 results to 1e-8, `nolog`, continuous regressors).
StatCraft's note (`qreg.md`, probe 245; ported from statcraft@b7d392e) had the variance and layout; its vertex descent
is written here in R (`stata_qreg_solve`, `crates/statatr/src/data_r/qreg.rs`). R's `quantreg` was tried first
and **does not** match Stata where the minimum is not unique.

- Estimates: the minimum of the weighted sum of q r (r > 0) and (1 - q)(-r), by descent along the edges of the
  vertices (start: the k rows least squares fits best; steepest edge; stop at the first residual that turns the slope up).
- **Not unique minima.** With dummy regressors the median (and many other quantiles) often has a flat edge. Stata ends at
  one end of it by its own path (WLS start, simplex), and which end is not a simple rule: on made-up data with a dummy
  the descent matched Stata at q = .5 and .25 and missed at q = .75 and with fweights (the other end of the same
  interval); `quantreg`'s Barrodale-Roberts picked the other end at q = .25. RCraft stops when the descent ends with a
  flat edge (any edge slope below 1e-10) rather than print a vertex that may differ.
- `Raw sum of deviations` is about the sample quantile (smallest y with at least a share q at or below it, weighted);
  `Min sum of deviations`; `Pseudo R2` = 1 - min/raw. e(q_v) is that quantile, e(df_m) k, e(df_r) N - k - 1.
- V = q (1 - q) s^2 (X'WX)^-1 with s = xbar'(b(q + h) - b(q - h))/((q + h) - (q - h)) and
  h = N^(-1/3) z^(2/3) [1.5 phi(Phi^-1(q))^2 / (2 Phi^-1(q)^2 + 1)]^(1/3), **z from level()** (`level(90)` changes the
  bandwidth). t tests on N - k - 1.
- Header: `Median regression` or `.25 Quantile regression` (quantile(75) is .75) and `Number of obs` as `%-14s=%11s`;
  `  Raw sum of deviations` + `%9.0g` + ` (about #)`; `  Min sum of deviations` + `%9.0g` and `Pseudo R2` at column 78.
- fweights work (N is their sum); aweights, pweights and iweights stop with r(101). predict xb and residuals.

## Open

Without `nolog` Stata prints its own iteration log (refused); `vce()`, other density and bandwidth methods, interactions;
the not-unique fits above.
