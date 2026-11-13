# Distribution functions (2026-11-12)

Observed on Stata 19 SE with made-up arguments: 3,740 expressions in five probes (session scratchpad),
each value written by `file write` in `%21x` and `%24.17g` and set against R 4.6's `p*`, `q*` and `d*`
functions. Corpus `distributions.do`. Translated in `crates/statatr/src/expr_r/dist.rs`.

## What R gives as Stata does

- t: `t()`, `ttail()`, `tden()`, `tprob()` (= 2·ttail(df, |t|)), `invt()`, `invttail()`; non-whole df
  are fine. F: `F()`, `Ftail()`, `Fden()`. chi2: `chi2()`, `chi2tail()`, `chi2den()`, `invchi2()`,
  `invchi2tail()`. Normal: `normalden(z)`, `normalden(z, s)`, `normalden(z, m, s)`, `lnnormal()`,
  `lnnormalden()` with one to three arguments. Beta: `ibeta(a, b, x)`, `ibetatail()`, `betaden()`,
  `invibeta()`, `invibetatail()`. Gamma: `gammap(a, x)`, `gammaptail()`, `gammaden(a, b, g, x)` (shape a,
  scale b, shift g), `invgammap()`, `invgammaptail()`. `lngamma()`, `digamma()`, `trigamma()`.
- On ordinary arguments every one of them is R's to 1e-12 or closer, most bit for bit; `digamma()` and
  `trigamma()` at negative or half arguments to 1e-11.
- Below 0 or above 1 a cdf is 0 or 1 (`chi2(df, -1)` = 0, `ibeta(a, b, 1.1)` = 1), as in R.
- A probability outside 0..1 in an inverse is missing; `invt()`, `invF()`, `invchi2()` at p = 0 is the
  lower end (0, or missing for t) and at p = 1 missing.

## Where Stata gives missing and R a number

- A degree of freedom of 0 or below (t, F, chi2: R treats chi2 with df 0 as a point mass at 0), a beta or
  gamma shape of 0 or below, a gamma scale of 0 or below, a normal sd of 0 or below.
- t, F and chi2 degrees of freedom above 2e17 (they go through the gamma with df/2); beta and gamma shapes
  and the binomial n above 1e17 (exactly: 1e17 gives a number, 1.0000001e17 missing). `tden()`, `Fden()`
  and `betaden()` have no such limit.
- `lnfactorial(n)`: n must be whole and 0 or more. `comb(n, k)`: n and k whole and 0 <= k <= n;
  `comb(0, 0)` is 1.
- `binomial(n, k, p)` and `binomialtail()`: n below 0 or p outside 0..1 is missing; n and k are cut to whole
  numbers (`binomial(10.5, 3.5, .3)` = `binomial(10, 3, .3)`); k below 0 gives 0 (tail 1), k at or past n
  gives 1 (tail 0 past n). `binomialtail(n, k, p)` is P(K >= k).
- `binomialp(n, k, p)`: missing for n = 0 and for k outside 0..n (after cutting).
- `poisson(m, k)`, `poissontail()`, `poissonp()`: missing for m <= 0 and k < 0; k cut to a whole number;
  `poissontail(m, k)` is P(K >= k).
- `gammaden(a, b, g, x)` below the shift g is 0; at x = g it is 1/b for a = 1, 0 for a > 1, missing for
  a < 1 (R's infinity).
- `normalden(., s)` with a valid s is **0**, not missing (also with `.a`); `normalden(., m, s)`,
  `normalden(.)` and `lnnormalden(., s)` are missing.

## Where R is right and Stata's number differs

- R's `qf()` gives 0 at small p (`invF(1, 10, 1e-8)`) and for df2 above 4e5 swaps to chi2/df1
  (`invFtail(1, 1e6, .001)`: 10.827566 against Stata's 10.827630, which is right). The translation takes
  the F quantile from `qbeta()`: x = qbeta(p, a/2, b/2) and F = (b/a)·x/(1 - x) when x < 1/2, else the
  complement y = qbeta(p, b/2, a/2) on the other tail and F = (b/a)·(1 - y)/y. That is Stata's to 1e-9
  everywhere probed but three subnormal or overflowing results.

## Not matched (the far ends; written in the differences guide; see the limits pinned down below)

- `invt()`/`invttail()` below p = 1e-200: with 1 df Stata gives missing, with 2.5 df a number 9e-6 off the
  exact one, with 5 df 2e-9 off; R gives the exact quantile.
- Densities with degrees of freedom or shapes of 1e10 or more (`Fden(3, 1e10, 2)` 5e-6 off, wild above
  1e13), `chi2den()`, `gammaden()` and `gammap()` near 1e17: Stata loses digits.
- `poisson()` and the rest with a mean of 1e16 or more, `poissonp()` with counts from 1e10, `binomialp()`
  from n = 1e10: Stata gives missing; between 1e15 and 1e16 for the mean and between 1e8 and 1e10 for the
  counts the edge was not pinned down, so R's number stands.
- `invchi2()` and `invgammap()` at p = 1 - 1e-13: 3e-9 apart.

## The limits pinned down (2026-11-13; probes `dist/out6`–`out9`, bisected)

- `poisson()` and `poissontail()`: missing for a mean above 2^53 (9.007e15 a number, 9.008e15 missing).
- `poissonp()`: missing for a mean above 1e8 (1.0000001e8 missing) or a count above 1e9.
- `binomialp()`: missing for n, cut to a whole number, above 1e6 (1000000.5 a number, 1000001 missing);
  `binomial()` and `binomialtail()` have no such limit there.
- `invt()` and `invttail()`: missing where the quantile passes 9.3675e153 in size — bracketed between
  9.365e153 and 9.370e153 with 1 and with 0.9 degrees of freedom, so a bound on the result, not on p.
- `invttail(d, p)` is worked out as −invt(d, p): R's upper tail fails for tiny p below 1 degree of freedom.
- Left as R's exact number (Stata's loses digits): `invt()` below p = 1e-200 with 2.5 or 5 df, the
  densities with df of 1e10 or more, Poisson probabilities near a mean of 1e15 (by about 1e-9). 3,988 of
  Stata's values checked: none now differs in being missing or not.
