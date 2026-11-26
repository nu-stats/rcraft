## STATUS

## P2B.SW15: a line R stops in did not happen for the translator either, 2026-11-26

- When R stops in a typed Stata line, the translator goes back to where it was before that line (its data frame, its new variable, its model, its e(sample) bookkeeping), as in Stata, where a failed command changes nothing. Before, after `use nosuch.dta` failed the translator still took the missing file as the data, and after a failed `generate` it called the variable already defined. Only a step of one translated line is taken back, and not `lasso`, `sqrtlasso`, `elasticnet` or `nlsur`, which post their results before stopping with r(430). Test (real R) in `crates/ui-egui/src/app.rs`.
- Not done: a failure inside a many-line entry (or a do-file run) is not taken back: the lines before it ran, and which one failed is not tracked.
