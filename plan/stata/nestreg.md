# nestreg (2026-11-18)

Observed on Stata 19 SE with made-up data (200 rows): regress, logit, probit and poisson, with the Wald and
likelihood-ratio tables, vce(robust), vce(cluster), store(), if, in, level(), one block and bare words, the
option and syntax errors, and `quietly` before the command. RCraft's output was set against Stata's log line
by line (blank lines included): the only differences left are `.` where Stata shows `.z`, the 15th digit of a matrix cell and, for
the likelihood models, in the 7th digit.
Corpus: `nestreg.do` (regress, 1e-8) and `nestreg_glm.do` (logit, probit, poisson, 1e-5: a Wald p-value agrees to about 6 digits).
Translated in `crates/statatr/src/data_r/nestreg.rs`.

- `nestreg [, quietly lr lrtable waldtable store(stub)]: command depvar (block) (block) ... [if] [in] [, options]`.
  A bare word is a block of its own. Read as a prefix (`Prefix::Nestreg`). Translated for regress, logit,
  probit and poisson, with level() (and vce(), robust, cluster() for regress).
- Each block's model is the command on the variables of the blocks so far, on the rows the full model uses (N
  is that of the full model with `if`, `in` and missing values). Its own output is shown, iteration log
  included, unless `quietly` is an option of nestreg.
- Output: a blank line, `Block k: vars` and the command's own output, for each block. With the `quietly`
  option the `Block` lines come together after one blank line. Then the tables: two blank lines before the
  first (one with the `quietly` option), one between tables, none after. With `quietly` before nestreg
  nothing is printed at all (and r() is still left).
- Tables are boxes: `  +---+`, cells ` Block ` (7 wide) then right-aligned columns, closing ` |`. Wald,
  regress: widths 8, 7, 10, 9, 9, 9 (F, Block df, Residual df, Pr > F, R2, Change in R2; two header lines;
  block 1's change blank); F `%.2f`, p and R2 `%6.4f`. Wald, other models: `Wald chi2` (10), df (7),
  `Pr > F` (9), one header line. LR: LL (10, %9.0g), LR (9, %.2f), df (7), Pr > LR (9), AIC (10, %9.0g), BIC
  (10, %9.0g). Default: Wald only; `lr` or `lrtable`: LR only; `waldtable` with them: both, Wald first. A
  number wider than its column was not seen, so the widening is not matched (the line stops when run).
- r(wald): rows block1..; regress: F df_block df_r pvalue R2 deltaR2 (deltaR2 of block 1 is missing:
  it is `.z` in Stata, `matrix list` shows `.z`; RCraft stores a plain missing value, shown `.`: not matched); other models: `Wald chi2` df_block
  pvalue. r(lr): LL LR df_block pvalue AIC BIC (LR of block 1 against the constant-only model; df_block from
  e(df_m)); AIC = -2LL + 2 rank, BIC = -2LL + rank ln N. r() holds only wald and/or lr.
- The Wald test of a block is the test of its variables in that block's model: regress F = W/q on (q, the
  model's residual df), the others chi2(q). With vce(robust) the robust variance and the model's df (n - k);
  with vce(cluster) the clustered one and df = clusters - 1.
- e() afterwards is the last block's model: e(cmdline) = `regress y x1 x2 x3 x4` plus the `if`/`in` text and
  then ` , options` as typed after the variables (`regress y x1 x2 x3 , vce(robust) level(90)`), no nestreg in
  it. `store(stub)` stores stub1, stub2, ... (`estimates dir` lists them with the command, the outcome and
  the parameter count).
- Errors (return codes 198 unless said): no blocks `varblocklist required` (100); `option foo not allowed`;
  `summarize is not supported by nestreg` (199); a first word that is a block: `depvar cannot be part of a
  variable block`; `nestreg, lr: regress ..., vce(robust)` (or cluster): `option lrtable not allowed with
  pweights, robust VCE, or clustered VCE`.
- A variable listed twice: Stata prints `note: x1 omitted because of estimability.` once before the first
  block, drops it from the block that repeats it and goes on; refused here (the rule for which copy goes was
  not decoded beyond the one probe).
- Not translated (refused when read): weights, factor variables and operators (`c.x1##c.x2`, `i.g`), a
  variable listed twice, tobit, ologit, nbreg and the other models (their Wald table form is not decoded:
  StatCraft's notes say an F test without R2 drops the last two columns, other commands give `Wald chi2`),
  options of the command other than level() (noconstant, beta, ...). Refused when run: a collinear model.
- Left as it was: `ereturn list` after regress does not list e(beta), which Stata 19 stores (plain regress
  too); that is regress's own gap. The likelihood models agree with Stata to about 7 digits.
