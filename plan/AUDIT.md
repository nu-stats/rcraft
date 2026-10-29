# Roadmap audit, 2026-10-29

Each task of `ROADMAP.md` checked against the code on `main` at `1a88d9d` (CI green on
Windows, macOS and Ubuntu), the tests, and the notes in `STATUS.md`. "Done" means built
and tested against real R; "Partial" lists what is left.

Why this was done: 36 commits (Phase 1A, P3.6 finish, P3.7, P5.4, the first three Stata
translator tasks) sat on branch `p3/join-keys-tidyverse` and were not in `main` until
today, so `main`, the coverage inventory and the top of `STATUS.md` all lagged. The branch
was fast-forwarded into `main` on 2026-10-29.

## Summary

| Phase | State |
|---|---|
| 0 Setup | Done |
| 1 R host and session | P1.1, P1.3, P1.4, P1.5 done. **P1.2 not started** (deferred on purpose). **P1.6 partial** (never logged). |
| 1A Packages | P1A.1 to P1A.9 and P1A.11 done, with four gaps below. P1A.10 partial. |
| 2 Command window, Results | P2.1, P2.3, P2.4 done (engine, terminal console, window). **P2.2 Tab completion partial** (only `rc` commands). |
| 2A Help | Not started. |
| 2B Stata mode | P2B.1 to P2B.3 done. P2B.4 to P2B.11 not started. |
| 2C Do-file runner | P2C.3 (front end vendored) done. Rest not started. |
| 3 Data | P3.1 to P3.7 done, except the leftovers below. **P3.8 not started.** |
| 4 Statistics | P4.0 to P4.5 done, except the leftovers below. |
| 5 Graphs | P5.1, P5.3, P5.4 done with leftovers. **P5.2 (ggplot2 builder) not started.** |
| 6 Depth modules | Not started (`catalog` has no panel, multilevel, survival, survey, time series, Bayesian or SEM entries). |
| 7 Reporting | Not started. `crates/report` is a one-line stub. |
| 8 Interface, packaging | P8.1 partly in place (Command, Results, Variables, Objects, Graph, Packages). Installers, example data, manual not started. |
| 9 Extensibility | Not started, though the dialog-spec format (P4.0) is its base. |

## Gaps and leftovers (the real list)

Not started:

- P1.2 portable R download (deferred until the build is near complete).
- P1.6 multiple sessions. A working directory can be set in the session config
  (`rhost::SessionConfig::working_dir`) but nothing in the window uses it.
- Phase 2A, all of it (no Help window, no `help topic`, no dialog Help button, no manual).
- P2B.4 to P2B.11, and P2C.1, P2C.2, P2C.4 to P2C.10.
- P3.8, P5.2.
- Phases 6, 7 (and `crates/report`, `crates/rgraph` are empty stubs), 8.2 to 8.5, 9.

Partly done, with what is missing:

- P1A.2: the "has a binary for my platform" filter.
- P1A.3: download counts, and a "new packages" list (CRAN's files do not support it).
- P1A.5/P1A.7: installing an older version has code but no screen.
- P1A.10: the missing-package prompt exists in dialogs (`Spec::needs`), not for typed commands.
- P2.2: Tab completion over R functions, objects, columns, file names and packages.
- P2B.1: macros, `foreach`, `forvalues` expansion; P2B.2: `_b[]`, `e()`, `r()`.
- P2B.3: `merge m:m`, merge `update`/`assert`, and other cases STATUS lists.
- P3.2 `.sas7bdat` catalogue files; P3.4 per-factor level list.
- P4.2 agreement measures (kappa); P4.3 Type II/III sums of squares and custom contrasts.
- P5.1 zoom and copy to clipboard; P5.3 heatmap and survival curves.

Stale records to fix:

- `plan/STATUS.md` header still says "Phase 0 scaffolded, not yet committed" (fixed with
  this audit).
- `crates/catalog` still marks the Packages entries (Search, Browse, Describe, Install,
  Installed, Update packages), "Translate Stata code" (data commands only) and the Stata
  mode entries as `planned`. The Packages entries are built; the coverage table needs a
  pass.
- ROADMAP P3.6 and P3.2 notes say work is "still to do" that is done (P3.6 finish, P3.7
  saves, P5.4 export, Phase 1A).

## Not checked by anyone yet

The window has been checked in rendered pictures for several screens (Packages, Find,
Export), but the data-frame chooser, the combine dialogs, the Data submenus, Browse, the
Graph tab, clicking Install in the live window, and the new Stata translator output have
not been used by a person in the running app.

Review still wanted: the plot-capture change to `rproto/bootstrap.R` (P5.1, P5.4) and the
session protocol.
