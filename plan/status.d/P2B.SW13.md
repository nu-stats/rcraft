## STATUS

## P2B.SW13: type mismatch where a function fixes the kind, 2026-11-26

- "type mismatch (Stata: r(109))" now also for comparisons whose two sides are of different kinds whatever the data hold: a function that gives a number against a string (`abs(x) == "a"`), one that gives text against a number (`substr(s, 1, 1) == 3`), string concatenation against a number. A plain variable beside such a function is checked in R against its own type, as it is beside a literal. The `if` is read with the expression reader, so a comparison inside any `&`, `|` or `!` is found. `f(x) == "a"` with a function whose kind is not fixed is left to R.
