# survwgt rake and isid (2026-11-07)

YFBPT6's rake.do rakes each country's sample to age-by-gender quotas: `survwgt rake preweigh,
by(age_gender) totvars(age_gender_tot) generate(weight_rake) maxrep(1000)`. survwgt is Nick Winter's
add-on from SSC (1.2.4), installed in Stata 19 with the user's OK. Its program file was not read: what
follows was observed on made-up data (`survwgt.do` keeps the checks).

## survwgt rake

- Raking (iterative proportional fitting): each pass adjusts the by() dimensions in the order given,
  w = w * T / S with S the category's weighted total and T its control total. The weights agree with
  Stata's to 1e-14 at every pass.
- Stopping, for totals below about a thousand: after the first pass in which no category, at the step
  that adjusts it, was 0.001 or more from its total (79 of 79 such cases). One dimension stops at the
  second pass, its weights those of the first.
- With several dimensions and totals in the thousands Stata stops sooner, by a rule not pinned down:
  every case where it did had margins within about 3.3e-6 of their totals (relatively), and none of
  the candidate measures tried separates the cases (changes in the weights, absolute or relative,
  summed or largest; the margins' gaps against the category's, the dimension's or the whole total;
  Stata's reldif). R stops with a message when a pass gets within 1e-5 before the 0.001 rule fires,
  rather than guess. One dimension is exact either way.
- maxrep(): passes at most, 10 by default. "Warning: variable pre reached maximum" and "iterations
  before convergence." (two lines) whenever the passes reach maxrep(), even when the last converged.
- The new variable is a double labelled "(raked)". The data are left sorted by the last dimension
  (rows that tie in no set order in Stata; R keeps them as they were). A survwgt that stops with an
  error has sorted them too.
- A missing weight stays missing and is left out; a missing by() value is a category of its own; a
  zero weight stays 0; a string by() variable works.
- Errors: "number of by() variables must equal number of totvars()" r(198); "variable w already
  defined" r(110); "Control total t not constant within categories of dimension g" r(198) (a missing
  total counts as different); "totals across dimensions 1 and j are not equal" r(199) when the
  dimensions' totals differ by more than 0.001 or 1e-5 of the first's, whichever is larger.
- r() after it holds a summary (N, sum_w, sum, mean, min, max) of a per-observation measure of the last
  pass: its largest value is the first dimension's gap at the start of the pass over the observation's
  smallest total, the rest not decoded. Not kept.
- `if` is accepted by Stata; what it does was not observed, so it is declined.

## isid

- Nothing when the variables identify the observations. "variable a should never be missing" (the
  whole list named: "variables s and a ...", "variables a, b, and c ...") when one is missing (a string's
  is ""), unless missok; then "... do not uniquely identify the observations" (or "does not" for one).
  Both r(459). sort is declined.

## Found on the way

- Abbreviations in expressions: Stata takes `keep if country==1` for country_v13, in if, in the
  expression of generate and egen, in replace's target and expression, and in assert; a variable comes
  before a scalar of the same name; an ambiguous one is "x ambiguous abbreviation" r(111). RCraft now
  asks R for the names when an expression has a name it has not seen as a variable.
- import delimited decides a column's type from the whole file: a column of numbers with "NA" at row
  6,583 is a string. readr looked at the first 1,000 rows.
