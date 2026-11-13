# medeff (mediation package, SSC) (2026-11-13)

Installed from SSC with the user's yes (`mediation`: medeff, medsens) and observed on made-up data
(probes `med/m1.do` to `m10.do` in the session scratchpad) and DZ8BUS (its StudyTwo.csv and
StudyThree.csv downloaded from the package's Dataverse page with the same yes; its do-file only says
"Read in StudyTwo.csv", so the copies read them with `import delimited`). Its code was not read.
Corpus `medeff.do`.

## What it prints and leaves

- "Using 0 and 1 as treatment values"; the mediator model's regress output; the outcome model's,
  refitted with the treatment first and the mediator second (`regress y t m x`); when the data has
  fewer rows than sims, "The number of observations in the data is less than the number of
  simulations. Expanding the data to the number of simulations"; then the table (84-dash rules, the
  labels padded to 31, the values %9.0g in fields of 10, 14, 14).
- The data is as it was afterwards (60 rows, the same variables). e() is the outcome regress
  (`e(cmdline)` "regress y t m x [] if __000000 , "). r(): delta0 delta1 zeta0 zeta1 tau navg, each with
  lo and hi (delta0 = delta1 and zeta0 = zeta1 without an interaction).

## The draws (each step pinned by sims(1), sims(2), sims(3), then 40, 100, 200)

- `set seed #`; the mediator model's coefficients (Stata's order, constant last), sims rows of
  b + L z, L the lower Cholesky factor of V, z = invnormal(runiform()) taken row by row (draw s, k
  coefficients, uniforms k(s-1)+1 ... ks): sims(1)'s direct effect is the first uniform's,
  sims(2)'s second draw the fourth.
- `set seed #` again, the same for the outcome model; the generator is left after its k sims
  uniforms (runiform() after sims(40) with 4 coefficients is the 161st).
- Per draw: ACME float(a * b) (a the treatment's coefficient in the mediator model, b the
  mediator's in the outcome model), direct effect float(c), total effect float(a * b + c).
- The means; the intervals are `_pctile`'s 2.5 and 97.5 percentiles (the average of two order
  statistics when n p is whole: sims(40) and sims(200) settled it).
- The share mediated: float(mean ACME / total effect) per draw; "Mean" in the table is its median
  (sims(3): 7.777, not the mean 17.4), the interval its percentiles.

Everything in r() and on the screen matched Stata 19 exactly on the probes; DZ8BUS's four medeff
tables and regressions are Stata's.
