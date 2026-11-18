# stepwise (2026-11-17)

Observed on Stata 19 SE with made-up data (30 datasets of 150 rows, 13 option sets for regress, logit and
probit with Wald and lr, poisson, groups of terms; plus single probes for the errors and the odd cases),
set against RCraft's output line by line: the trace, the final table and e() agree for regress; the
likelihood models agree except in the seventh digit of a coefficient (Stata stops iterating at its tolerance).
Corpus: `stepwise.do` (regress, 1e-8) and `stepwise_glm.do` (logit, probit, poisson, 1e-6).
Translated in `crates/statatr/src/data_r/stepwise.rs`; StatCraft's `plan/stata/selection.md` (read first) agreed with all of
this that it covers.

- `stepwise [, pr(#) pe(#) forward hierarchical lockterm1 lr]: command depvar terms [if] [in] [, options]`.
  Terms are variables or parenthesised groups. Translated for regress, logit, probit and poisson, with
  level() (and beta for regress). The command line is read by the reader as a prefix (`Prefix::Stepwise`).
- Search: backward with pr() (the default), forward with pe() alone or `forward`. Backward removes the term
  with the largest p when p >= pr; forward adds the smallest p below pe. hierarchical tries only the last
  term in the model (backward) or the next one in the list (forward). lockterm1 keeps term 1 in (and starts
  a forward search from it). The test is Wald on the term in the model (regress: F; the others chi2), or
  with lr the likelihood-ratio test against the model without it (backward) or with it (forward).
- The rows are those with no missing value in the outcome or any term of the full model (and if/in), for the
  search and the final model (N stays 175 of 200 after the variable that had the missing values is
  removed).
- Output (all after one blank line): `Wald test, begin with full model:` (`empty model`, `term 1 model`;
  `LR test, ...` with lr), one line per step, `p = 0.9696 >= 0.2000, removing x5` / `p = 0.0000 <  0.1000,
  adding x1` (%6.4f, `<` padded to two characters), `keeping x3` (hierarchical backward stops at the first p <
  pr), `testing x5` (hierarchical forward stops at the first p >= pe), a group by its words (`removing x4 x5`).
  When nothing changed: `p < 0.2000 for all terms in model` (backward) or `p >= 0.1000 for all terms in
  model` (forward), after the header. Then the command's own output, without an iteration log.
- The final model lists its terms in the order they were entered; when removing, see the end (the last term takes the removed one's place).
- e(): the command's own, e(cmdline) = `stepwise ` and the line after its word as typed (`stepwise ,
  pr(.2): regress ...`), e(stepwise) = "stepwise" right after it.
- Errors (return codes 198 unless said): neither pr() nor pe(): `at least one of pr(#) or pe(#) must be
  specified`; `forward` without pe(): `option forward invalid or option pe() is missing`; pr() or pe() not
  in (0, 1): `pr() invalid -- invalid number, outside of allowed range` (125); both with hierarchical:
  `hierarchical pe(#) pr(#) invalid`; pr <= pe with both: `pr(.1) <= pe(.2) invalid` (the numbers as typed);
  `option b not allowed`; no terms: `no independent variables`; a command stepwise does not run: `summarize
  is not supported by stepwise` (199).
- Odd cases seen: a collinear full model: Stata notes `x4 omitted because of estimability` and leaves that
  term out of the whole search (even in a forward search, whatever its own p), by a rule not decoded (not
  the later one: x4 went, not x5). A model of the constant alone at the end (nothing passes pe()): Stata runs
  the command on no terms, `LR chi2(0) = -0.00`, `F(0, 149) = 0.00`.
- (Superseded 2026-11-18, see the end: weights, factor variables, vce(), robust and noconstant are translated.) Not translated (refused when read): weights, factor variables and operators, a variable listed twice, other
  options of stepwise (vce(), ...) or of the command (noconstant, robust, vce(), ...), commands other than
  the four. Refused when run: a collinear full model; no term kept (the constant-only model's printing is
  not matched); a search that, with both pr() and pe(), could take the other kind of step (add a term back
  after removing, or remove after adding): none of the 30 datasets did, so the order is not decoded.
- Left as it was: `ereturn list` after regress does not list e(beta), which Stata 19 stores (plain regress
  too); that is regress's own gap.

## The forms added 2026-11-18 (weights, vce(), robust, noconstant, factor variables, level())

Observed on Stata 19 SE with made-up data (probes of 150 rows: every weight kind x Wald and lr x the four
commands, 14 vce/robust/noconstant forms, 13 factor-variable specifications x 8 option sets x regress and
logit, set against RCraft's output line by line, p-values to the four digits Stata prints; corpus
`stepwise_weights.do`, `stepwise_factors.do` (regress, 1e-8), `stepwise_weights_glm.do`,
`stepwise_factors_glm.do` (logit, probit, poisson, 1e-6)).

- **Weights.** regress takes all four kinds, the likelihood models fweight, pweight and iweight; `aweights not
  allowed` (r(101)) for logit, probit and poisson. The sample is the rows with no missing term (as before) and
  a non-zero, non-missing weight. Each test is made on the model's own variance: regress's `e(V)` and `e(df_r)`
  for the weight (fweight N - k, iweight floor(sum w) - k, pweight robust N - k), a chi2 from the
  likelihood models' `e(V)` (pweight robust). With `lr`, the log likelihood is the weighted one: regress
  `-Neff/2 ln(RSS/Neff)` with Neff = sum of fweights, n for aweights, floor(sum iweights); a likelihood model
  the sum of w x each row's log likelihood (R's own `logLik` rounds non-integer weights).
- **vce(), robust, noconstant.** Wald tests use the command's variance: regress vce(robust) HC1 (F, N - k),
  vce(cluster v) F(q, G - 1), likelihood models robust N/(N-1), cluster G/(G-1) and vce(oim). `vce(ols)`
  is regress's, `vce(oim)` the likelihood models'; `vcetype oim not allowed` (regress), `vcetype ols/hc2/hc3
  not allowed` (the others), r(198). `lr` with pweights, robust or vce(robust) stops: `option lr not allowed
  with pweights, robust VCE, or clustered VCE` (198); **vce(cluster) with lr does not stop**, the test is then
  the plain likelihood-ratio one. noconstant leaves the constant out of every fit; the empty model is then
  the zero predictor. Not translated: vce(bootstrap), vce(jackknife), vce(hc2), vce(hc3), vce(opg), cluster().
- **Options before the colon.** `vce()`, `robust`, `noconstant` and `beta` belong to the command:
  `option vce() not allowed` and so on, r(198). `level(#)` belongs to stepwise: it sets the level of the final
  table (the command's own `level(90)` is ignored there, Stata prints 95); both give `option level() of
  stepwise conflicts with option level() of regress`; outside 10 to 99.99 `level() must be between 10 and
  99.99 inclusive`; (the earlier translation passed the command's level() through, which Stata does not.)
- **Factor variables.** `stepwise` takes each column of a factor-variable term as a term of its own: `i.g` is
  one term for each level but the base (`2.g 3.g 4.g`, tested one at a time, removed and added one at a time,
  named `3.g` in the trace), and `c.x1##c.x2` is three terms (`x1`, `x2`, `c.x1#c.x2`). A parenthesised group
  is one term of all its columns, named by them (`2.g3 3.g3 x1`). The base level is noted first, once for each
  factor in the order given: `note: 1b.g omitted because of estimability.` (`3b.g` with ib3.g), then the blank
  line and the header. Hierarchical backward tests the last column, forward the first not yet in. `lockterm1`
  locks the first column (the first term when it is a group); with a factor variable in the first term it stops
  unless that term is in parentheses: `option lockterm1 requires that the first term be bound in parentheses if
  it contains a factor variable` (198). The final model has only the kept columns, so e(b) has no base
  columns (`2.g 4.g x4 x1 _cons`); a factor's kept levels stand together at the place of its first one, and
  the table shows them as the usual block.
- **The order the final model lists its terms in** (decoded here; the earlier note, "written order", was
  right only when the removals happened to leave it so): removing a term puts the last one of the list in its
  place (`x6 x2 x3 x4 x5 x1` keeping x1 and x5 gives `x1 x5`, where the written order would give `x5 x1`);
  adding appends. This holds with and without factor variables.
- **How RCraft builds the final model.** The kept columns go to the command as a formula; the kept levels of a
  factor as `stata_ib(g, "1=2,4")` (base 1, levels 2 and 4: a contrast with only those columns, so the other
  levels count as the base and e(b) names `2.g` `4.g`, with no base column); `e(b)` naming reads that string.
  The model's `factors` map is cleared, so margins by factor after such a model is not offered.
- **Not translated** (refused when read): factor terms other than `i.g`, `ib#.g` and `c.x#c.y` / `c.x##c.y`
  (`g#h`, `i.g#c.x`, `ibn.g`, `2.g`, operators), noconstant with factor variables, lockterm1 with a factor
  variable in a parenthesised first term, a variable listed twice (Stata notes `x1 omitted because of
  estimability` and goes on), weights as expressions. Refused when run: as before, and a base level `ib#.g`
  that the data lack.
