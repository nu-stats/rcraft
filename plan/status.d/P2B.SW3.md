## STATUS

## P2B.SW3: Stata-mode messages, 2026-11-26

- A line that is not translated now gives R a warning with its reason, so Results shows it as a notice. `sysuse` and `webuse` say that StataCorp's example datasets cannot be shipped and point to `use` or File > Use my file.
- An unknown command close to a known one (`summary`) says it is not a Stata command and suggests `summarize`. `help <command>` opens RCraft's Help window on that command's differences page.
- A variable the data lack stops summarize, tabulate, tab1, list and the common models with "variable x not found (Stata: r(111))" before anything runs, so the model before stays for `predict`. dplyr's missing-column and type-mismatch errors print as r(111) and r(109). `list` no longer warns about its separator column.
- Not done: a fit that fails for another Stata reason (convergence, collinearity) still clears the earlier model.
