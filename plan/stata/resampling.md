# Resampling: jackknife, statsby, rolling, permute, simulate (batch I)

Written 2026-11-19. Ported from statcraft@c5c9c07 (its `resampling.md`, `permute.md`, `simulate.md`) and checked
again on Stata 19.0 SE with made-up data: corpus `jackknife.do`, `statsby.do`, `permute.do` and `simulate.do`
(numbers compared to 8 significant digits). Code: `crates/statatr/src/data_r/{resample,statsby,permute,permute_run}.rs`
and, for `permute` and `simulate`, `crates/statatr/src/dofile/resample.rs`.

## How they run

- `jackknife`, `statsby` and `rolling` take the command after the colon, translate it once, and put its R inside one R
  loop (like `bootstrap`): the data minus one unit, the rows of a group, or the rows of a window. They run built-in
  commands only (`summarize`, `regress`, `logit`, ...), not programs the do-file defines.
- `permute` and `simulate` must run programs the do-file defines (the probes call `permcode`, `sim1`), and a program
  is run line by line by the do-file runner. So the runner writes the line out as the lines Stata runs: a begin line,
  then the command once more for each replication with a collect line after it (and for `permute` a shuffle line
  before it), then an end line. They are internal commands of the translator, named `_rcraft_pm_*` and `_rcraft_sm_*`
  (`dofile/resample.rs` has the picture). Each stands for the source line, so the coverage counts it once. At most
  2,000 replications are written out line by line. A built-in command after the colon is instead one line whose replications run in one R loop; a program
  whose second and third replications became the same R, with no value asked of R on the way, has the others in one R loop too (5,000 replications of a
  small program take about 12 seconds); only a program whose R changes between replications is held to 2,000.
- An error Stata gives before anything runs is one line that stops (`_rcraft_rs_fail`), so `capture` sees its `_rc`:
  `variable nosuch not found` r(111), `reps(0)` r(125), a missing comma (the options read as expressions) r(133).

## jackknife

- The command runs on the data, then once for each observation of its estimation sample (each cluster with
  `cluster()`) left out. A command with no `e(sample)` uses all observations, or with an `if` only the observations it picks (`e(N_reps)`
  is their number; with `in` the rows keep their places while one is left out, checked in Stata 19).
- The replicates are floats. V = (n - 1)/n times the sum of products of the deviations from their mean; t on n - 1 degrees
  of freedom (`e(df_r)`), `e(N_reps)` n.
- **Correction to StatCraft's note:** `e(b_jk)` is not the mean of the replicates. It is the bias-corrected value
  n times the observed minus (n - 1) times the mean of the replicates (the mean of the pseudo-values). For a mean it
  differs from the observed in the seventh digit (float replicates times 59); for a standard deviation by 0.006 here.
- With `_b` after `regress`, `logit` or `probit`, the header is the model's title with `Number of obs`, `Replications`,
  `F(k, n - 1)` (the Wald statistic over k) and `Prob > F`, then R-squared and Root MSE (or the log likelihood and
  Pseudo R2). `e(F)` and `e(p)`; with `cluster()` `e(p)` comes before `e(F)` in the list. `e(V_modelbased)` holds the model's V.
- `regress y x, vce(jackknife)` (and `logit`, `probit`) is the same run as `jackknife _b: regress y x`, with the line as
  typed in `e(cmdline)`.
- r() after it holds only `r(table)` (9 rows b, se, t, pvalue, ll, ul, df, crit, eform).
- A missing value of the cluster variable is r(459) (`missing values not allowed in cluster variable g`).
- Notes: options other than `cluster()`, `level()`, `nodots` and `notable` are refused; so are statistics other than
  `_b` after those three models and r() or e() expressions, and a replication that stops with an error (Stata drops it).

## statsby

- Groups in ascending order of the `by()` variables; a missing by value drops the row unless `by(vars, missing)`
  (**not** a `missing` option: Stata 19 says `option missing not allowed`, as StatCraft's note had it). `total` adds one
  more row, by variables missing, last, with no dot. The command runs on each group's rows, so a model sees only them.
- Results are floats labelled with the expression (`r(mean)`, `_b[x]`), format `%9.0g`; the by variables keep their
  type, format and value labels. Unnamed expressions are `_stat_1`, ...; `_b` and `_se` give `_b_x`, `_b_cons`, `_se_x`,
  all the `_b_` ones first, in the order they were given.
- Output: `(running cmd on estimation sample)`, a blank line, `Command:`, the named expressions and `By:` (`<none>` without
  `by()`), a blank line, `Statsby groups:` and a dot a group on a line of its own (no count); `nodots` drops the
  running line and the dots.
- `saving(file[, replace])` writes the results (data label `statsby: summarize`) and leaves the data in memory, sorted by the `by()`
  variables (rows with equal values may come in another order than Stata's); `clear` with `saving()` is r(198).
- After `logit` or `probit`, `_b` and `_se` name the variables after the equation (`yb_b_x`, `yb_se_cons`, labelled `[yb]_b[x]`).
- Neither `clear` nor `saving()`: Stata replaces data it counts as saved and stops with r(4) (`no; dataset in memory has changed since last saved`)
  otherwise. RCraft follows whether the data are saved: `use` and `save` make them so, and so does `clear`; `sort`, `gsort`, `order`, `tsset`, `xtset`,
  `summarize`, `list`, `describe`, `display`, `count`, `tabulate`, `codebook`, `correlate`, `ttest`, `regress`, `logit`, `return`, `ereturn`, `estimates`,
  `preserve` and `restore` keep it as it was; `generate`, `label`, `format`, `import`, `insheet`, `input` and `set obs` change it (each checked in Stata 19:
  `replace x = x` keeps it, so `replace` is not known, and nor is any other command). Not known: refused.
- **Refused:** `saving(, double)`,
  `subsets`, `basepop()`, `double`, `by()` of a list that is not plain names, `if` or `in` on the command.
- After it the data are the results, `tsset` is gone, r() and e() are not known.

## rolling

- Windows over the `tsset` time variable alone (not `xtset`): `window(#)` consecutive observations, `stepsize(#)`, and
  `recursive` (the start stays at the first observation). The windows are `floor((N - window)/step) + 1`. `start` and `end` are
  the time variable's values (its type and format) at the first and last row of the window.
- Output: `(running cmd on estimation sample)`, a blank line, `Rolling replications (41): ` with a dot a replication,
  the count every tenth, then ` done`; no header.
- A window is `window` periods of the time variable, whether or not each has an observation (checked with gaps: the windows run
  from the first time value to the last minus the window, and `start` and `end` are those time values, not observed rows); `recursive` keeps the start.
- `saving(file[, replace])` as in `statsby`.
- `rrecursive`: the window ends at the last period and its start moves on by `stepsize()` (checked in Stata 19).
- **Refused:** a time variable that is missing or not in order, `by()`, `keep`, `reject()`, `window()` that is not a whole number,
  `recursive` with `rrecursive`.

## permute

- `seed()` is set first; the command runs once on the data as they are (its own draws come first), then once for each
  replication. The permutation is drawn before the replication, on the data as the previous replication left them
  (a command that sorts the data changes what the next one acts on).
- A random integer in 0..k-1 is: the smallest power of two m >= k, `floor(u*m)` of a new uniform, again while it is k or more.
  The general shuffle is Fisher-Yates from the front, for a = 0..n-2 swap positions a and a + j(n-a). With `strata()` each stratum
  in ascending order of its value, its rows in data order, one stratum after another in the same stream.
- A variable with exactly two distinct values and no missing among the rows used takes another way, starting again each
  replication from its original values: the rows of the smaller group first (the smaller value when the groups are equal),
  then the others, each in data order; that array of values is shuffled with m = the smaller group's size steps
  (a = 0..m-1, swap a and a + j(n-a), j over n - a), and written back to the rows in that same order.
  **Correction to StatCraft's note:** the *values* are swapped and written back, not the row numbers. With strata it is done in each
  stratum (a stratum with one value only gets its own count), checked with 6 draws of a 4+4 data set.
- With a model (an `e(sample)`), only the rows of the estimation sample are permuted (rows with a missing value of the
  dependent variable or of the permuted variable stay out).
- Replications and T(obs) are floats (doubles with `saving(, double)`); c_lower counts T <= T(obs), c_upper T >= T(obs) among the non-missing.
  The first run is not a replication: n is the number of replications. p = c/n, two-sided 2 min(p_lower, p_upper) at most 1, SE sqrt(p(1-p)/n);
  the one-sided intervals are Clopper-Pearson at `level()` (two-sided, so alpha/2 in each tail), the two-sided interval p +/- z SE
  clipped to [0, 1], missing when SE is 0.
- Table, notes and `Number of ...` block as in Stata 19 (the block ends at column 79, numbers right-aligned to the widest). The
  warning that the command sets no `e(sample)` is wrapped at line size - 2 with a 9-column hanging indent.
- r(): N, n_reps, n_strata (with strata), k_exp, k_eexp (the number of coefficients with `_b`), level; cmd, command, permvar, strata, title,
  exp1...; matrices b, n, c_lower, c_upper, p_lower, p_upper, p_twosided, se_p_*, ci_p_* (2 rows ll and ul). Not kept: `r(rngstate)`.
  e() is empty afterwards. The data are put back.
- Default `reps()` is 100. `_b _se` together put `b:` and `se:` equations in the table; `_b` alone none; `saving()` names them `_b_x`, `_se_x`.
- An e-class program is the command like any model: its e(sample) is the sample, `_b` its model's. After a command whose r() RCraft keeps whole
  (`summarize` without options) a name it does not leave is missing, so `r(nosuch)` is r(322). After `logit` and `probit`, `_b` is named after the
  equation (`yb_b_x`; the table has an equation heading `yb`).
- `spearman y x` (two variables) prints Stata's p-value: a symmetric beta on [-c, c] with rho's permutation variance 1/(N - 1) and kurtosis
  3(25N^3 - 38N^2 - 35N + 72) / (25N(N + 1)(N - 1)), so a = 3(b2 - 1) / (2(3 - b2)), c = sqrt((2a + 1)/(N - 1)), p = 2 P(Beta(a, a) > (|rho| + c)/(2c)). It is
  Stata's to 4 decimals (35 samples, 4 to 100 observations); Stata's r(p) agrees exactly in 30 of 48 samples and in the 7th digit in the others, by a rule
  not found (Stata's own `ibetatail()` of the same numbers gives RCraft's value), so r(p), r(p_l) and r(p_u) stop with the reason when read. With ties
  Stata's kurtosis is not the exact permutation one (2.4235 against 2.4229 for an 8-row sample): a printed `spearman` with ties stops, `quietly` keeps r(rho)
  and r(N). Corpus `spearman.do`.
- **Refused:** `_se` after `logit` and `probit` (equation names not decoded), `r(nosuch)` after commands whose r() RCraft does not keep whole,
  `spearman` with ties printed (and its r(p) at all), every command not translated, `strata()` of
  several variables, `_b`/`_se` with other expressions or in another order, `_b` after commands other than `regress`, `logit` and `probit`,
  `left`, `right`, `eps()`, `saving(, every())`, a command that sorts the data with a two-valued variable (the original values are
  matched to the rows by position).

## simulate

- Each replication runs the program on the data **as the program left them** (Stata does not restore them: a program that
  makes a variable without dropping it stops in its second replication, and Stata then records a missing row; RCraft stops there with R's
  error instead and does not record missing rows). `seed()` is set before the first replication; there is no trial run.
- Results are floats labelled by the expression; unnamed expressions are `_sim_1`, ...; `_b` and `_se` as in statsby. The data are the results afterwards;
  r() is what the last replication's program left (checked: `return list` shows the last run's `r(sd)` and `r(mean)`), e() is empty.
- Output: a blank line, `Command:` and the named expressions, a blank line, `Simulations (25): ` and a dot a replication, the count every tenth,
  a line break after every fiftieth, ` done`.
- After `logit` and `probit`, `_b` and `_se` give `yb_b_x`, `yb_se_x`, labelled `[yb]_b[x]` (corpus `resample_logit.do`, compared to 1e-6: the
  results are floats, and Stata's and RCraft's maximum likelihood estimates agree to about 1e-8, so a float rounds differently now and then).
- A replication that stops with an error ends there (its lines after it do not run, as Stata ends the program) and leaves a row of missing values; the
  dots show `x` and the line `x: Error occurred when simulate executed prog.` (permute: n counts the others). Checked in Stata 19 (corpus).
- **Refused:** `saving(, double)`, `reps()` missing (Stata r(198)), e-class programs whose last line is not a model, `_b` after programs
  that end with anything but `regress`, `logit` or `probit`. (jackknife: a replication that gives a missing value is still refused.)

## Observed, short

- `statsby` and `rolling` after `preserve`: the data come back with `restore`; the labelled `describe` shows the Stata label
  `statsby: summarize` (RCraft keeps variable labels only).
- A float column built by `import delimited` stays float in Stata after `replace y = <expression>`; RCraft does not round the replaced
  value to float (not new here). The corpus uses values that are exact in a float (2.5, 3.25) after it.
- Stata's `describe` after `rolling` names a temporary file; RCraft shows its own list.
## Other things fixed with it

- `preserve` and `restore` leave r() and e() as the last command left them (checked in Stata 19: `restore` after a `summarize` inside the
  preserved block still shows that `r(mean)`); the translator had them replace r(). Corpus `restore_r.do`.
- `format varlist %fmt` takes patterns and ranges (`format c* %18.0f`).
- `drop _all` is not turned into a list of variable names (that left the observations): it removes them too, and is accepted with no data.
- `return scalar x = runiform()` takes Stata's draw.


## bootstrap: the names bs and bstrap, an r-class command's sample (2026-11-19)

Observed on Stata 19.0 SE with made-up data (corpus `bs.do`, 1e-9), for the BraderTuckerDuell package's
`bs mean = r(mean), reps(100) seed(010101) level(90): sum support if(...)`.

- **Names:** `bs` and `bstrap` run `bootstrap`. `bootstrap` itself is not abbreviated: `bst`, `bstr`, `bo`, `boo`
  and `boots` are unknown commands (r(199)); `boot` is some other command (it stops with "=exp not allowed",
  r(101)). The reader maps `bs` and `bstrap` to `bootstrap`; the others stay unknown.
- **Sample of an r-class command** (`summarize` and the like): the rows its `if` and `in` pick, missing values of its
  variables kept (`bs r(mean): summarize y` with 7 of 60 values missing has e(N) 60; with `if g==1` 30, with
  `in 10/40` 31, with both 15). Each replication resamples those rows. RCraft took all rows before, so `if` gave
  wrong replications; now the rows are picked once. With `in`, the command runs on those rows without its `if`
  and `in` (in a replication `in 10/40` would count the resampled rows). An estimation command with `in` is refused
  for now (its sample is e(sample), found after the fit).
- **Percentile interval:** the points are taken in percent from the level in percent, `(100 - 90) / 2 = 5`. RCraft
  computed `100 * (1 - 0.9) / 2`, which is 4.999...; with 20 replications the 5th percentile then sits at an exact
  position (20 x 5 / 100 = 1) and Stata averages the 1st and 2nd values, which the fraction missed.
- **Also as the package has it:** `seed(010101)` (leading zeros), `level(90)`, `if(...)` with no blank, the command over
  two lines (the package is in `#delimit ;` mode; the corpus uses `///`), then `matrix ci = e(ci_percentile)`, `local lb = ci[1,1]`, `matrix mean = e(b)`,
  `matrix N = e(N)`.
- **Open, refused:** `bs` (or `bootstrap`) of a program the do-file defines (`bs diff = r(diff), ...: diff`, four
  package lines and the lines after them). Like `permute`, it needs the runner to write each replication out
  (resample, run the program, collect), which is more than a word form; refused with "diff is not translated yet".
