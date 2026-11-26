## STATUS

## P2B.SW5: a failed fit leaves the earlier model, 2026-11-26

- A model line keeps the last model aside while it runs (`stata_prev_fit`) and, if R stops in it, puts it back, as Stata keeps e() when a command fails. So `regress y x`, a failing `logit`, then `predict` or `_b[x]` still work. Not put back: a stop that is RCraft saying it cannot do what Stata would (its words name Stata or RCraft without Stata's own "(Stata: r(#))"), where Stata has results R does not, and a fit that posts its results on purpose before it stops (lasso and nlsur, r(430)). Every estimation command on the shared model path is covered. Regression test in `crates/engine/tests/stata_regress.rs`; the whole Stata corpus still passes.
- Not done: the translator's own state (the saved e(sample) bookkeeping) is kept from the line it translated, so a `predict ... if e(sample)` after a failed fit can differ from Stata's.
