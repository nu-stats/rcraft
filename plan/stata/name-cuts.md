# Names cut in tables (2026-11-13)

Observed in Stata 19 with made-up data (probes `ab/a1.do`, `a2.do` in the session scratchpad) and
DZ8BUS's regressions: `abbrev()`, `regress`, `logit`, `summarize`, `pwcorr`, `list` and `matrix list` all
cut a long name the same way.

- Cut to n (12 in the tables, 8 over pwcorr's columns, 11 for an equation name in `matrix list`, its
  colon making 12): the first n - 1 - k characters, `~`, the last k, with k the smallest that makes the
  short form match one variable of the data alone (`~` standing for any characters):
  `threatcre~ty` beside `threatcredibilitybinary` (`threatcred~y` matches both), `abcdefgh~lmn`
  beside `abcdefghijzzmn` and `abcdefghijkln`, `abcdefghi~ln`, `abcdefghij~p`.
- When no k makes it unique (`xxxxxxxxxxxxxa` beside `xxxxxxxxxxxxxxa`): the first n - 2 and `..`.
- A name that is no variable (`notavariablename`): k = 1, `notavariab~e`.
- `tabulate`'s one-way head splits a long name in pieces of 11, the last beside the heads
  (`threatcredi` / `bilitybinar` / `y`); RCraft had dropped the third piece.

RCraft's `stata_tab_abbrev` (the `stata_tables` helper) does this with the data's names, which the
translator gives before each model and the commands in `NAME_TABLES`; `summarize`, the regress tables,
`ereturn display`, `pwcorr` and `matrix list` use it.
