# Work orders

One file per roadmap task left. Each says exactly what to build, where, how it is
proved, and what "done" means, so the work needs no exploring and no re-deciding.
"Done" is not a judgement: it is the task's line in `plan/progress.txt`, whose
checks `cargo xtask progress` runs (and CI runs on every push). Nobody types a
percentage.

## How every order is carried out

1. **Read only:** this file, the order, `plan/DECISIONS.md` and the files the
   order names. Don't survey the codebase; the order already did.
2. **Copy the pattern** the order names (for a dialog: an existing
   `crates/catalog/src/<area>.rs` spec and its `crates/engine/tests/<area>.rs`).
3. **Register:**
   - a dialog goes in `ENTRIES` (`crates/catalog/src/lib.rs`) as `verified(...)`, under
     the exact name the order gives;
   - add it to `stats::SPECS` (or the order's list);
   - add a fitted model's class to `postest::MODEL_CLASSES` when the order says
     "Model menu";
   - run `cargo xtask coverage`;
   - add the package to the `ci.yml` package list when the order adds one.
4. **Prove it**, as `crates/engine/tests/panel.rs` does:
   - against the formula written out in plain R;
   - and, where the order names a Stata command, against Stata 19 on the same made-up data.
     Run Stata from PowerShell on a scratch do-file and put only the numbers into the test as
     constants. No Stata log is committed. Never read Stata's own files.
5. **Record:**
   - in the PR that makes the checks hold, turn the task's line in `plan/progress.txt` to `done`
     (CI refuses a `done` whose checks fail);
   - for a task that stays `part`, leave the line;
   - add `plan/status.d/<id>.md` with the ROADMAP note.
6. **Ship:**
   - run `cargo xtask ci` locally (R 4.4.3 env vars as in `plan/DECISIONS.md`);
   - one PR per order, `gh pr merge --auto --squash`.
7. **Report** to the owner in five lines at most: the PR, the figure from `cargo xtask progress`,
   the next order.
8. **Time box:** each order has hours. At one and a half times that, stop and report what's left.
   Don't push on silently.

One thread works through the orders in `plan/progress.txt` order. Nothing runs in
parallel; no sessions, subagents or watchers. Keep one build folder (incremental builds).

## The owner's decisions, asked once

| # | Decision | Orders waiting on it |
|---|---|---|
| 1 | Install into the R 4.4 library, and add to CI: glmmTMB, urca, vars, forecast, rstanarm, lavaan, metafor, tidymodels | P6.2, P6.5, P6.9, P6.10, P6.11, P6.12 |
| 2 | Add the Azure Trusted Signing secrets (`AZURE_CLIENT_ID`, `AZURE_TENANT_ID`, `AZURE_SUBSCRIPTION_ID`) to `nu-stats/rcraft`, as StatCraft has. Claude can't enter credentials | P8.2 (signed Windows installers) |
| 3 | An Apple Developer ID, for a signed and notarised macOS DMG (optional) | P8.2 (otherwise the DMG ships unsigned) |
| 4 | P1.2 portable R: the owner asked to raise it only near the end; it is the second-to-last order | P1.2 |
| 5 | Download the Noto Sans and Noto Sans Mono font files (OFL licence) to embed in RCraft's PDFs | P2A.10 |

P6.1, P6.3, P6.4, P6.6, P6.7, P6.8, P7.1, P2A.6 and P9 need no decision, and come first.
