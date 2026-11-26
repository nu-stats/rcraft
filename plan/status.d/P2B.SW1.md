## ROADMAP

- P2B.SW1 Stata mode works on data opened outside Stata (done 2026-11-26): after File > Use my file, a Data window import, or R code that made a data frame, `summarize`, `describe`, `regress` and the rest work on the data frame shown; before, only data a Stata line (`use`, `import`) had loaded counted, and every command said no data was in memory. Picking another data frame moves the next line to it, as `use` would. Found by running the real app on the user's R 4.6.2 (plan/sweep-2026-11-26.md).

## STATUS

## P2B.SW1: Stata mode works on data opened outside Stata, 2026-11-26

- **Done** (`crates/statatr/src/live.rs`, `data_r.rs`, `dofile.rs`, `crates/ui-egui/src/app.rs`, `stata_mode.rs`): `Live::adopt(name)` starts the session on an R data frame the user opened another way; the translator begins with it in memory and reads its variables from R when a line runs. The window calls it when the data frame shown differs from the one shown at the last Stata line (a frame Stata's own `use` brought up is not mistaken for one). Adopting other data than Stata worked on starts the session over on it (macros and model were the other data's); adopting the data Stata already works on changes nothing. Tests: `data_opened_outside_stata_is_the_data_in_memory`, `adopting_other_data_starts_the_session_over_on_it` (translator) and `stata_mode_works_on_data_opened_outside_stata` (the window, real R).
