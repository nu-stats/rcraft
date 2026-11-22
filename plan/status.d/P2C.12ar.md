## ROADMAP

- P2C.12ar Censored and selection models (batch J of the StatCraft port, done 2026-11-20, in part)
  (`tobit`, `intreg`, `truncreg`, `cloglog`, `hetprobit`, `heckman`: Stata's logs and starts, fw/pw/iw weights where Stata takes them, robust and clustered errors, replay, predict, `estat ic`) Verified
  (corpus `tobit.do`, `intreg.do`, `truncreg.do`, `cloglog.do`, `hetprobit.do`, `heckman.do`; note `plan/stata/censored.md`; R in `data_r/censored/`).
  **On hold** (listed, to be built): `stepwise` and `nestreg` with tobit, `from()`, tobit/intreg with aweights, tobit with interactions, factor variables in truncreg, cloglog, hetprobit and heckman,
  `iterate()`, `noconstant`, margins after these models.
  Measured: probes 80.2% -> 80.5% (23,278 -> 23,343 translated source lines of 30,115 judged), packages 94.7% -> 94.8% (4,025 -> 4,028; the `tobit` lines of 6J6BAW).

## STATUS

## P2C.12ar: censored and selection models (batch J of the StatCraft port), 2026-11-20

- **Done** (each checked in Stata 19 on made-up data; corpus `tobit.do`, `intreg.do`, `truncreg.do`, `cloglog.do`, `hetprobit.do`, `heckman.do`, 1e-6, truncreg and heckman
  1e-5; note `plan/stata/censored.md`): `tobit` (Stata's gsem-style log, `ll`/`ul`, LR or F test, robust, clustered, fw/pw/iw, factor variables, replay, predict
  xb stdp stdf pr e ystar), `intreg` (Olsen's parameters, two logs, same weights and variances, predict), `truncreg` (rows dropped and counted, fw/aw/pw/iw, robust,
  clustered, predict xb stdp), `cloglog` (Stata's own start: the logit fit one iteration before its end; ll_0 rounded to float; eform; fw/pw/iw; predict pr xb),
  `hetprobit` (its probit fit, then the full model; LR or Wald test of lnsigma; fw/pw/iw; predict pr xb stdp sigma), `heckman` (ML from the two-step start and
  `twostep`; select(s = ...); fw/pw/iw; LR or Wald test of rho; predict xb xbsel psel mills ycond yexp). One shared Newton engine in R (`censored/cx.R`).
  `estat ic` after them (R's layout). aweights stop as Stata stops (r(101)) where it does (cloglog, hetprobit, heckman); `twostep` with weights r(198).
- **Found:** tobit's F test and t tests use N - k degrees of freedom **with clusters too** (StatCraft's note said G - 1); `e(ll_c)` is missing after robust
  hetprobit fits; Stata allows aweights in tobit, intreg and truncreg (tobit and intreg then give each row its own variance: not reproduced, refused; truncreg's
  rescaled aweights are matched); cloglog's `e(marginsok)` is `default Pr`.
- **On hold, listed:** `stepwise` and `nestreg` with tobit (their searches run on R fit objects; 3 probe lines), `from()` (probe 508: 3 lines and 4 that read
  its results), tobit/intreg with aweights, tobit with `i.g##c.x` (probe 289), factor variables in truncreg, cloglog, hetprobit, heckman and `select()`,
  `iterate()`, `noconstant`, `offset`, margins and `estat` other than `ic` after these models.
- **Shares** (dry run, `many_dofiles.rs`, same folders before and after, measured on `origin/main` 6111d87 plus this branch): probes **80.5%** (23,343 of 30,115
  source lines translated, 5,668 not, 1,104 known only when run; was 23,278 / 5,733, 80.2%); packages **94.8%** (4,028 translated, 219 not; was 4,025 / 224, 94.7%): the four
  `tobit` lines of 6J6BAW translate. nlsur (#11) landed meanwhile: its 13 lines add to these.
