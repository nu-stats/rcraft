# npregress kernel (2026-11-17)

Ported from StatCraft's decoding of Stata 19 (its `plan/stata/npregress.md` and
`commands/npregress.rs`, probes 554 and 555), our own work, written in R
(`data_r/npregress.rs`). Checked on those probes and in `npregress.do`, recorded from Stata 19.

- One continuous covariate. The mean at each point by the local-linear fit (or Nadaraya-Watson with
  `estimator(constant) noderivatives`), the derivative by the local-linear slope at its own
  bandwidth; kdensity's kernels and the triweight.
- The mean's bandwidth: Stata's one-dimensional Nelder-Mead on the leave-one-out criterion
  ln(sum r^2 / N) (ln(sum r^2) / N for local-constant), from sd(x) N^(-1/5). The derivative's: he =
  hm N^(2/35) p, p searched from 1 on the mean squared distance between the leave-one-out slope and
  the full-sample local-cubic slope, capped at p = N/50 by max(y).
- Doksum and Samarov's R-squared, E(Kernel obs) = N hm rounded, the identification rule (a point
  whose local design is singular at hm is left out and marked in `_unident_sample`), e() in
  Stata's order, `_Mean_y` and `_d_Mean_y_dx` (or predict()'s names).
- On R: probes 554 and 555 every line translated but `describe, fullnames`, 27 of 27 compared lines to
  8 digits, the logs and tables line for line, every %21x bandwidth to about 1e-15 (all nine
  kernels); `npregress.do` (17 results) at 1e-9.
- Not translated: more covariates, factor variables, imaic, vce(bootstrap) and reps(), bandwidths
  given as a matrix or by name, predict and margins afterwards, e(datasignature).
