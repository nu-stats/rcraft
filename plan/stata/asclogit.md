# asclogit, drawnorm, _pctile, and clogit on Stata's path (2026-11-06)

YFBPT6 runs `asclogit chosen ac1 ... op3, case(csid) alternatives(alt) vce(clu id_g) noconstant`, then
`drop if !e(sample)`, `estat summarize, equation labels`, `drawnorm beta1-beta9, n(1000) means(e(b))
cov(e(V)) clear`, simulated probabilities, and `_pctile prob10, p(2.5,97.5)`. Observed in Stata 19 with
made-up data shaped like it (`asclogit.do` keeps the checks).

## asclogit (alternative-specific variables only, noconstant)

- The estimates and variance are clogit's on the cases (`clogit ..., group(case)`), to the last digit.
- Notes first: `note: variable v has N cases that are not alternative-specific; there is no
  within-case variability.` for each variable constant within some cases (N of them).
- Header "Alternative-specific conditional logit": Number of obs, Case ID variable / Number of cases,
  Alternatives variable / Alts per case min avg max, Wald chi2(k), Log (pseudo)likelihood and Prob > chi2,
  the clusters line.
- e(): rank N ic k k_eq converged rc ll [N_clust] k_eq_model df_m chi2 p k_indvars i_base k_casevars
  alt_avg alt_max alt_min k_alt N_ic N_case const; macros cmdline cmd title predict(asclogit_p)
  estat_cmd marginsok marginsnotok marktype datasignaturevars datasignature alteqs alt1.. altvar
  key_N_ic case depvar indvars chi2type [clustvar] vce [vcetype] and the optimizer's; e(b) with the
  alternatives variable as its equation (alt:ac1). Not kept: ic, the data signature, stats, altvals,
  altfreq, alt_casevars, V_modelbased, ilog, gradient.
- `estat summarize`: "Estimation sample asclogit", Number of obs (and the clusters with min/avg/max),
  a depvar block and an alternatives block; the mean in 10 after the bar, std. dev. in 11, min and max
  in 9, each %9.0g; "Std. dev. not adjusted for clustering".

## drawnorm

Stata 19 against its own rnormal() stream: the draws are taken a variable at a time (n for the first,
then n for the next), each row is means + L z with L the lower Cholesky factor of cov(); the deviations
L z are kept in float, the means added in double, and a float variable is rounded again. Default float,
`double` keeps the sum. Bit for bit on two data sets.

## _pctile

Stata's default percentiles (the mean of the two values at a whole rank) into r(r1), r(r2), ...

## clogit on Stata's Newton-Raphson

YFBPT6's whole run showed clogit's estimates 6e-7 from Stata's: Stata stops its iterations short of the
maximum. With one positive outcome a group, RCraft now follows the path: an ordinary logit's slopes as
the start (StatCraft's decoding, probe 243), then Stata's ml steps and stopping rule (as for probit).
The per-country estimates then agree to 1.2e-12.

## YFBPT6 run whole

4,288 of 4,549 model values the same as Stata's (to 1e-6). The 261 others are robreg m in the macro*
files, on simulated data built from those estimates: 1e-12 upstream, some float draws round the other
way, and robreg's loops magnify it. Run on RCraft's own y1macro.dta, Stata's agg_merge.do gives the same
file bit for bit, and Stata's robreg on it gives RCraft's numbers (298 of 300). On 2026-11-04 the R run
read the files Stata's run had written; now it builds the chain itself.
