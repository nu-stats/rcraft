# Survival: stdescribe, stsum, sts, stptime, stir, ltable, streg, stcurve (batch K)

Written 2026-11-19. Ported from statcraft@c5c9c07 (its `survival.md`) and checked again on Stata 19.0 SE with made-up data
(corpus `survival.do`, `streg.do`; numbers compared to 8 significant digits, `streg` with Stata's own tolerances set tight, see below).
Code: `crates/statatr/src/data_r/{survival,streg}.rs`; `st.rs` (`stset`, `stcox`) gained `time0()`, `stset, clear` and `stcox, nolog`.

## What `stset` leaves for them

The commands read `_st`, `_d`, `_t` and `_t0`. `stset` now also remembers what the headers print: the failure variable as typed
(`1 (meaning all fail)` without `failure()`), the analysis time (`t`, or `t/10` with `scale(10)`) and the `id()` variable. `time0(var)` gives each
record's start (so a subject can have gaps; without it each record starts where the one before ended), and `stset, clear` drops the four
variables. Every st command prints a blank line, `Failure _d:`, `Analysis time _t:`, `ID variable:` (if any) and a blank line first.
Data that are not st: Stata stops with r(119).

## stdescribe

Per subject (the `id()` variable, else each row): records, entry (first `_t0`), exit (last `_t`), gap (the sum of `t0 - previous t` over the
subject's records in time order), time at risk, failures. The table shows total, mean, min, median, max. `r()` holds 30 values (`N_sub`,
`N_total`, `N_mean`.., `t0_*`, `t1_*`, `gap`, `N_gap`, `gap_*`, `tr`, `tr_*`, `N_fail`, `f_*`). The `gap_*` numbers are over the subjects that have a gap
(missing without one). Checked with several records and gaps.

## stsum

Total (and by group) time at risk, failures over it as the rate, subjects, and the 25%, 50% and 75% survival times: the first time the Kaplan-Meier
estimate is at or below 1 - p (a small tolerance, 1e-10, for the float product). r() is the total's: `p75 p50 p25 N_sub ir risk`, also with `by()`.
With late entry the risk set at a time is the records with `t0 < time <= t`.

## sts

- **list**: one row per distinct exit time. Greenwood's error, `S^exp(+-z se_ll)` with `se_ll = sqrt(sum d/(n(n-d)))/|ln S|`; error and interval are
  missing where S is 1 or 0. `failure` is 1 - S with the interval reversed; `na` is Nelson-Aalen (sum d/n, error sqrt(sum d/n^2), interval
  `H exp(+-z se/H)`; its error is 0, not missing, at the first row of the list). `by()` shows `Net lost` (lost minus entered) and a note; `at()` reports at
  the last listed time at or before each requested one, failures counted since the one before. r() afterwards is Stata's stale summarize values: not kept (refused if read).
- **generate** (`s se(s) lb(s) ub(s) na se(na) lb(na) ub(na) h n d`, `by()`, `level()`): the value of the record's own exit time; `h`, `n` and `d` are missing at
  times with no failure; `se(s)`, `lb`, `ub` missing where S is 1, `se(na)` missing where the cumulative hazard is 0. An existing name stops with r(110).
- **test**: log-rank weight 1, `wilcoxon` weight n, `tware` sqrt(n), `peto` the Peto-Peto estimate prod(1 - d/(n + 1)), `fh(p q)` S(t-)^p (1 - S(t-))^q
  (Kaplan-Meier). Expected `d n_k/n`, hypergeometric variance, chi2 on the first K-1 groups; `strata()` sums scores and variances over strata. r(chi2), r(df).
- **graph**: step plot of the survivor (`failure`, `na`, `by()`, `ci`) drawn with ggplot2; not compared with Stata's.
- All of them stop (R) with late entry, except `stsum`, `stptime`, `stdescribe`.

## stptime

Needs `id()` (r(198) otherwise). Person-time sum of `t - t0`, failures, rate times `per()`, interval `rate exp(+-z/sqrt(d))` (missing with no failure).
`at(list)`: pieces (a, b], the time and failures past the last cut as `> last` unless `trim` (then the total is the sum of the pieces); `by()` repeats the
pieces per group. Display `%10.0g` for the rate, `%9.0g` for the rest, `dd()` fixes decimals. r(): `ptime failures rate lb ub` of the total, the rate scaled by `per()`
(person-time is Stata's float sum, so `r(ptime)` is 1421.0000009 where the exact sum is 1421; 8 digits agree).

## stir

`ir` on failures and time at risk by exposure; the larger of two values is exposed. `strata()` is `ir, by()`. r() is `ir`'s. Refused: an exposure variable
without exactly two values.

## ltable

`ltable time [fail] [if] [in] [fweight=w] [, by() intervals() survival failure hazard noadjust level() test]`. Intervals `[kw, (k+1)w)`, only those with records;
n' = n - lost/2 (`noadjust`: n); S = prod(1 - d/n'); error `S sqrt(sum d/(n'(n'-d)))`, 0 at S = 1, missing at S = 0; log-log interval, missing at S = 0 or 1.
`failure` 1 - S; `hazard` `d/(w(n' - d/2))`, error `h sqrt((1 - (hw/2)^2)/d)`, normal interval cut at 0, missing without deaths. The tables were compared number for number
with Stata's log (not recorded: ltable stores nothing). `test`: the likelihood-ratio statistic of equal exponential rates (printed from the closed form; Stata's own is
about 7e-6 relative above it, an iterative fit) and the log-rank test; only r(chi2) and r(df), the log-rank's, are kept and recorded. Without a failure variable every record fails.

## streg

- Distributions `exponential`, `weibull`, `lognormal`, `loglogistic`, `gompertz`. Options: `nohr`, `tr`, `time`, `level()`, `robust`, `vce(robust|cluster v)`, `cluster()`,
  `nolog`, `noshow`, and `tolerance()`, `ltolerance()`, `nrtolerance()` (taken and ignored: R's fit is run to its limit).
- Log likelihood: Stata reports that of log time, `sum d (ln f0(z) - ln sigma) + (1-d) ln S0(z)`, **without** the `-ln t` that the density of t has (StatCraft's
  note says the same in other words). With late entry `- ln S0(z0)` at the entry time. Newton with the exact derivatives (location-scale in `(b, ln sigma)`; Gompertz in
  `(b, gamma)` with `h = exp(xb + gamma t)`), from the least-squares fit, step-halving, two polishing steps.
- Metric: exponential and Weibull are PH by default (`b_PH = -b_AFT/sigma`, `ln_p = -ln sigma`, delta-method V), AFT with `time` or `tr` (which implies the AFT estimates,
  `_b[x]` is then the AFT coefficient); lognormal and loglogistic are AFT only (`/lnsigma`, `/lngamma`); Gompertz PH (`/gamma`).
- Variance: robust is the sandwich scaled `N/(N-1)`, clustered `G/(G-1)`; with `id()` set, robust means clustered on the subject (N_clust = subjects, `e(vce)` cluster).
  The robust Wald test uses the reported metric. When the slopes' variance is of less than full rank (2 clusters) Stata shows `Wald chi2(0)` and missing: `e(df_m)` is 0, `e(chi2)` and `e(p)` missing.
- e(): `rank N k k_eq k_dv converged rc [N_clust] ll k_eq_model ll_0 [rank0] df_m chi2 p`, then `aux_p k_aux risk N_fail N_sub` (Weibull), `N_sub N_fail risk` (exponential)
  or `k_aux risk N_fail N_sub` and `sigma` or `gamma`; texts `cmd` (`ereg weibull lnormal llogistic gompertz`), `cmd2`, `title`, `depvar`, `chi2type`, `vce`, `vcetype`, `clustvar`...; `e(b)`,
  `e(V)` with columns `_t:x ... /:ln_p`. Not kept: `e(ic)`, `e(ilog)`, `e(gradient)`, `e(datasignature)`.
- **Stata stops short of the maximum at its default tolerance** (lognormal: coefficients 1e-7 off, the robust Wald statistic 1e-6 off). The corpus runs Stata with
  `tolerance(1e-13) ltolerance(1e-15) nrtolerance(1e-13)`; then all 36 results agree to 1e-8 but one: the coefficient of x of the loglogistic fit with late entry is 3e-8 off even
  then, so that single number is compared to 7 decimals (a `round()` in the do-file, with the reason in its header).
- `_b[/ln_p]` (and `/lnsigma`, ...) now read the `/:ln_p` column of e(b).

## stcurve

After `streg` only: 101 equally spaced times from the first to the last exit of the estimation sample, `at(var=value ...)` (or `at1()`, `at2()`) at the means of the other covariates
(means over the estimation sample); survivor, failure, hazard, cumulative hazard from the fitted distribution; ggplot2. The curves are not compared with Stata's (a graph).

## Open: refused, not guessed

`streg` generalized gamma, factor variables, `strata()`, `ancillary()`, `frailty()`, `offset()`, `noconstant`, other `vce()`, `predict`, `test`, `lincom`, `margins` and `estat` afterwards;
`stcurve` after `stcox` (the Kalbfleisch-Prentice baseline is described in StatCraft's note, not built here), `stcurve` hazard smoothing; `sts` with late entry, `sts graph` options other than
`by() failure na ci level()`, `sts test, cox` and `trend` and `detail`, `sts list` with `enter`/`compare`/`adjustfor`/`strata`; `stptime` with `smr()`, `using()`, models; `stir` with an exposure of
other than two values; `ltable` with `graph`, `overlay`, `tvid`, `saving`; `stdescribe, simple`; `stset` with weights.
