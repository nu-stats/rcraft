# Standing decisions and rules (owner's, as of 22 Nov 2026)

These bind every contributor. `AGENTS.md` has the full licensing rules, and it wins on any conflict.

## Licensing (never break)

- **R:** drive R as a separate process only. Never link or embed R, and never bundle CRAN packages in the binary.
- **Other projects' code:** never copy code, wording or layouts from GPL, AGPL or LGPL projects, including R Commander, jamovi, JASP, RStudio and BlueSky. Never copy ACM/CALGO code either.
- **Stata:** learn only by running licensed Stata 19 on made-up data.
  - Never read Stata's own files (`.ado`, `.mata`, `.sthlp`, `.dlg`), including installed add-ons. Never even locate them with `which`.
  - Never commit Stata logs or output. Commit only numeric `@ key value` lines in corpus `.recorded` files, and keep them numeric and under about 75 characters.
- **Replication packages:** before running one in Stata, comment out every `ssc install`, `net install`, `ado update` and `, perm`.
- **Installs:** installing anything needs the owner's yes. These are already approved and installed in Stata:
  - cluster2, survwgt, isvar, center, winsor, tab_chi;
  - revrs, anovaplot, pdslasso, ivreg2, mediation, rangestat, nsplit;
  - winsor2, unique, distinct, SPost13.

## Quality

- **Matching:** a translated command must match Stata 19. If it can't, refuse with a clear reason. Never guess, and never let R crash with a raw error.
- **Precision:** numbers must agree with Stata to 8 significant digits today. The owner is being asked whether model fits (`mixed`, `menl`, `gsem`, `arima`) may match only what Stata prints, as StatCraft does.
- **On hold is not dropped:** say "on hold", list the item, and build it later.
- **`_natscale`:** ranges with no label at 0 are translated (8 digits). Ranges with a label at 0 are parked by the owner.
- **Mata and compiled plugins:** outside the first release.
- **Tab completion:** it must also work in Stata mode (P2B.12).
- **The goals:**
  - more than 95% of do-file lines translated, on the real packages and on the probes, with results matching Stata near 100%;
  - the roadmap at 98% by 5 Dec 2026.

## Working style (the owner is firm on these)

- **One working thread at a time.** No side threads, no parallel agents, no coordinator layer. Parallel threads cut output by more than half here (41 to 13 checks a day) through clashes and duplication.
- **Each next step waits for the owner's go,** except work the owner already asked for.
- **Report only results:** merges, failures, and decisions only the owner can make. Short, plain words, no jargon. No pop-up questions; a question is one plain sentence.
- **Answer the owner's questions first,** before more work.
- **Never repeat a fixed mistake.** Check this file before acting.
- **Every push:** local `cargo xtask ci` first, then a PR with auto-merge. Never push to `main`.
- **Builds:** keep one working folder with its build, so builds are incremental. Clean it weekly.
- **Model choice:** use the cheaper model for routine work, the strongest only for hard decoding.
- **Portable R (P1.2):** raise it only near the end of the build, as the owner asked.
- **RCraft's own PDF manuals (P2A.10):** wanted; planned after the in-app manual (P2A.6), see `plan/work-orders/P2A.10.md`.
- **"% complete":** means the RCraft card on the owner's Craft Suite Monitor, counted on the 108-task roadmap.

## Build and test on the owner's machine

```
R_HOME=C:/Users/jstow/R/R-4.4.3
R_LIBS_USER=C:/Users/jstow/AppData/Local/R/win-library/4.4
LOCALAPPDATA=C:/Users/jstow/R/rcraft-test-appdata
cargo xtask ci
```

## Progress

"% complete" is what `cargo xtask progress` prints, from `plan/progress.txt` and the code; CI prints it on every run. The work left is in `plan/work-orders/`, one order per task.
