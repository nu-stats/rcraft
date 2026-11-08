# winsor (SSC) (2026-11-08)

ZNMIXH's 01_analysis.do caps income and wedding spending: `winsor income_b_orig,
gen(income_b_orig_w) high p(.05)`. winsor is Nick Cox's add-on from SSC, installed in Stata 19
with the user's OK on 2026-11-08; its program file was not read. Observed on made-up data;
`winsor.do` keeps the checks (to 1e-13).

- Over the nonmissing values of the rows of `if`/`in` (N of them): the h lowest become the
  (h+1)th smallest and the h highest the (h+1)th largest, ties included; `high` (shortest `high`)
  or `low` (`low`; `hi` and `lo` are not taken) for one end only. h is `h()` or floor(`p()` N).
- The new variable keeps the old one's storage type and display format, is missing outside
  `if`/`in`, and is labelled from the variable's name (not its label): "x, Winsorized fraction
  .1" or "x, Winsorized extreme 2", then ", high only" or ", low only".
- Errors, r(198) unless said: "option generate() required"; "generate() should give new
  variable name" (r(110)); "use either h() option or p() option"; "h() or p() option required,
  h( ) or p() > 0" (also h(0), p(0)); "p() too high" (p of .5 or more); "0 values to be
  Winsorized" (floor(p N) of 0); "11 values to be Winsorized, 22 in data" (2h of N or more);
  "too many variables specified" (r(103)); "option by() not allowed".
- A string variable is taken by the add-on (its values sorted as text); not translated.
