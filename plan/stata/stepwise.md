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
- The final model lists its terms in the order they were entered (the order written when only removing).
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
- Not translated (refused when read): weights, factor variables and operators, a variable listed twice, other
  options of stepwise (vce(), ...) or of the command (noconstant, robust, vce(), ...), commands other than
  the four. Refused when run: a collinear full model; no term kept (the constant-only model's printing is
  not matched); a search that, with both pr() and pe(), could take the other kind of step (add a term back
  after removing, or remove after adding): none of the 30 datasets did, so the order is not decoded.
- Left as it was: `ereturn list` after regress does not list e(beta), which Stata 19 stores (plain regress
  too); that is regress's own gap.
