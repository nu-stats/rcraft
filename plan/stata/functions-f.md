# Functions, batch F: word(), the noncentral distributions, old names, regexs() and more (2026-11-19)

Observed on Stata 19 SE with made-up arguments (about 7,000 distribution rows and 600 string cases
in scratch probes); the values are in the corpus files `functions_f.do` and `noncentral.do`. Gap
list: StatCraft's `crates/functions/src/library.rs` had the names; every rule below was decoded
again on Stata 19 here. Translated in `crates/statatr/src/expr_r/strfn.rs` (strings) and
`expr_r/dist.rs` (numbers), the helpers `stata_strfn` and `stata_dist`.

## Words

- `word(s, n)`, `wordcount(s)`: a word is what **blanks** separate. A tab, a new line, a no-break
  space and a quote are part of a word (`wordcount("a<tab>b")` is 1, `wordcount(`"a "b c" d"')` is 4).
  `n` is cut towards zero (1.9 is 1, -1.5 is -1); 0, missing (`.`, `.a`), past the count (either
  side) and any huge number give `""`. A negative `n` counts from the end. A number as `s`, a string
  as `n` is r(109); the wrong number of arguments r(198).
- RCraft's old `wordcount()` split at tabs too; that was wrong and is fixed.

## Strings

- `strproper(s)` (old name `proper()`): ASCII letters only. A letter after a non-letter (a digit,
  an apostrophe, a hyphen, `_`, or any non-ASCII byte) is capitalised, one after a letter is
  lower-cased: `o'neil` is `O'Neil`, `123abc` is `123Abc`, `it's` is `It'S`, `ñandú` is `ñAndú`,
  `ÉLAN` is `ÉLan`.
- `stritrim(s)` (old name `itrim()`): a run of blanks (spaces only) between two other characters
  becomes one; **leading and trailing runs stay**; tabs and new lines are not blanks.
- `strrpos(s, t)`: the byte where `t` last starts (overlapping counts: `strrpos("aaa", "aa")` is 2),
  0 if not found; an empty `t` gives the byte length of `s` (0 for an empty `s`).
- `strpos(s, t)` (old name `index()`): bytes too (`strpos("é x", "x")` is 4); an empty `t` gives 1,
  but 0 for an empty `s`. RCraft's `strpos` counted characters and gave 1 for two empty strings;
  both fixed (`ustrpos` is untouched).
- `strreverse(s)` (old name `reverse()`): reverses the **bytes** (`strreverse("é")` is two broken
  bytes).
- `subinword(s, old, new, n)`: replaces `old` where it is a whole word, meaning there is a blank or an
  end of the string on each side (a comma, a tab, a hyphen or a point is not a boundary); `old` may
  hold blanks and has to match them exactly; an empty `old` changes nothing; scanning goes on after
  each replacement. `n` missing (`.`, `.a`) replaces all, a count of 1 or more (cut towards
  zero) that many; 0, negative and **2147483648 or more** replace none.
- `regexr(s, re, t)`: replaces the first match with `t` **as it stands** (`\1`, `\\`, `$1`, `&` are
  just those characters). An invalid expression (`(`, `[`, `*`, `?`, `+`, `)`) leaves `s` whole, with
  no error. Works on bytes (`.` matches one byte of `é`). The expressions are PCRE-like: leftmost
  first (`a|ab` matches `a`), lazy `+?`, lookahead, `(?i)`, `\d \w \s \b`, back references, and `\<` and
  `\>` as word ends. R's PCRE (`perl = TRUE`, bytes) does the same; the two word ends are rewritten
  for it.
- `regexm(s, re)` is 1 or 0 (an invalid expression is 0). It keeps its match: `regexs(n)` gives group
  `n` (0 is the whole match) of the **last** `regexm()`, `""` for an unset group, `n` above 9 or below
  0 or missing, and for a failed match; `n` is cut towards zero. Before any `regexm()` it is `""`.
  In a command that goes over the rows (`generate y = regexs(1) if regexm(s, "...")`) each row sees
  its own match. RCraft keeps the groups per row and `regexs()` gives them row by row; after a
  `regexm()` of an earlier command Stata gives the last row's groups for every row, RCraft each
  row's (a caveat is shown with the code).
- `plural(n, s)`: `s` itself when `n` is 1 or -1, else `s` with an `s` added (no rules: `bus` is `buss`,
  `city` is `citys`, `""` is `s`); missing and non-whole `n` are plural. `plural(n, s, x)`: for
  `n` of 1 or -1 `s`; else `x` as it stands (`plural(2, "child", "ren")` is `ren`, not `children`); if
  `x` starts with `+` the rest is added to `s`; if it starts with `-` the rest is cut off the end of
  `s` when `s` ends with it (`plural(2, "mouse", "-ouse")` is `m`; a rest that is not there, or no
  rest, leaves `s`).
- `bsubstr(s, n1, n2)` (bytes) and `udsubstr(s, n1, n2)` (`n1` counts characters, `n2` is the
  display columns, a wide character taking two, whole characters only: `udsubstr("日本語", 1, 1)` is
  `""`, `("a日b", 3, 1)` is `b`): `n1` and `n2` are cut towards zero; a negative `n1` counts from the
  end; `n1` of 0, missing, past the end, or before the start (`-10` of 6) gives `""`; `n2` missing
  (`.`, `.a`) is the rest, 0 or below `""`. `substr()` and `usubstr()` follow the same rules (not
  retranslated here).
- `match()` is `strmatch()`.
- `matrix(exp)` is `exp` read with its names as scalars and matrices: a variable named in it is
  "not found" (r(111)); a matrix on its own is r(109); `matrix(.)` and `matrix("a")` give `.` and `a`.
  Outside the matrix `A[3,3]` is `.` in Stata (RCraft's `A[i,j]` stops with r(503), not changed).
- `cofc()` in lower case is not a Stata function (r(133), "unknown function"); `Cofc()` and `cofC()`
  are, and were done. RCraft refuses `cofc()` with that reason.

## Distributions

- Old names, same values to 1e-8 or better: `normprob(z)` is `normal()`, `invnorm(p)` is
  `invnormal()`, `fprob(a, b, f)` is `Ftail()`, `invfprob(a, b, p)` is `invFtail()` (the same
  missing values), `chiprob(df, x)` is `chi2tail()` to about 1e-11, **except** that it gives 1, not
  missing, for a degree of freedom above 2e17.
- `nt(df, np, t)` and `nttail(df, np, t)` (the noncentral t): R's `pt(t, df, ncp)` agrees to about
  1e-9 in the cdf. Missing when df is 0 or below or above 1e10, when `np` is above 1000 in size, or
  any argument is missing. R switches to a rough approximation at `|np|` above 37.62, which Stata
  does not use (the values then differ in the 3rd digit): **RCraft stops there** (an R error) and
  does not give a number. `|t|` above 1e154 breaks R; set to infinity (Stata: 1 and 0).
- `nchi2(df, np, x)`, `nchi2tail`: R's `pchisq(x, df, ncp)`. Missing for df 0 or below or above 1e6,
  `np` below 0 or above 1e4, missing arguments; `x` below 0 gives 0 (tail 1). R rounds results below
  about 1e-300 to 0 where Stata keeps them (not matched).
- `nF(a, b, np, x)`: R's `pf(x, a, b, ncp)`. Missing for a degree of freedom 0 or below or above 1e8,
  `np` below 0 or above 1e4. `nFtail()`: no limit on the degrees of freedom, `np` at most 1000.
  Results below about 1e-3 agree only to a few digits (R's series stops at 1e-9 absolute).
- Not done (no probe lines, so not decoded): `invnt()`, `invnttail()`, `invnF()`, `invnFtail()`,
  `invnchi2()`, `invnchi2tail()`, `npnt()`, `npnF()`, `npnchi2()`; they exist in Stata 19.

## Open and refused

- `nt()`/`nttail()` with `|np|` between 37.62 and 1000 (an R error, not a number); `cofc()`.
- `regexs()` after a `regexm()` of an earlier command (see above); `ustrregexm()` and its `ustrregexs()`
  are not touched (`ustrregexm` is `grepl`, `ustrregexs` is not translated).
- `strpos()` and `strreverse()` of a string with bytes that are not UTF-8 work on bytes; R shows the
  result with its bytes marked as UTF-8.
