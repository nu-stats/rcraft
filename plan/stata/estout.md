# eststo and estadd (SSC estout), as they behave (P2C.9)

The estout package was already installed in Stata 19 here. Run with made-up data, 2026-11-03; only what
they did was read, never their code.

- `eststo m1` stores the active results under m1, as `estimates store` does, and `estimates restore m1`
  brings them back. Without a name: est3, est4, ... by position (after m1 and m2, the next is est3).
- `eststo m2: regress y x if x > 2` fits and stores; `e(_estimates_name)` is m2 afterwards.
- `estadd scalar a = 5` adds e(a) to the active results; after another model e(a) is missing, and after
  `estimates restore m1` it is 5 again: the stored copy got it too. The same holds for results stored with
  `estimates store`.
- `estadd scalar ym = r(mean)` then `estadd scalar ysd = r(sd)`: r() survives the first.
- `estadd local tag "yes"` adds the text e(tag). Adding a scalar again without `replace` gave no error;
  with `replace` the new value stands.
- `eststo drop m1` and `eststo clear` drop them; `estimates restore m1` then gives r(111).
- `eststo: regress y z` with z not there: r(111).
