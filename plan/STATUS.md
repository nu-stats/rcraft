# Status

Last updated: 2026-11-12.

**Where things are:** Phases 0 to 4 are built (Help, Stata mode and Run my do-file included), but
P1.2 (a portable R download), P2A.6 and P2A.10 (RCraft's own manuals) and the leftovers
`plan/AUDIT.md` lists; Phase 5 is built but P5.2 (the ggplot2 builder); Phases 6 to 9 are not
started. `plan/AUDIT.md` (2026-11-12, with the 2026-10-29 audit kept below it) lists every phase
against the code and what is left. The sections below are a dated log, oldest first.

## Done

- P0.1 Workspace and layered crates (L0 to L7, empty except `core`,
  `catalog` and `rcraft-cli`)
- P0.2 Contributor rules, MIT and Apache-2.0 licences, `NOTICE`
- P0.3 `cargo xtask ci`: fmt, clippy, tests, layers, assets, coverage
- P0.4 Coverage inventory (`crates/catalog`, `docs/coverage.md`)
- P0.6 CI workflow for Windows, macOS and Linux (not yet run: no remote)

## P1.3 and P1.4 (R session), 2026-10-26

- `rproto`: the wire format (requests, tab-separated frames led by a random
  per-session token, escaping), and the R loop it talks to
  (`crates/rproto/src/bootstrap.R`). 8 unit tests.
- `rhost::Session`: starts R, `run`, streamed `Event`s (output, warnings,
  errors, finished, exited), `interrupt`, `restart`, `run_to_end`. Output is
  capped per run and the channel is bounded. 16 tests start a real R (skipped
  with a note when none is installed) and pass here against R 4.4.3: state
  persists, errors and parse errors, warnings versus messages, unicode, long
  lines, huge output, crash then restart. No R processes were left behind.
- `rcraft --eval "R code"` runs code through a session.
- Interrupt: on Unix it sends SIGINT (`kill -INT`); that path is written but
  untested here (this machine is Windows), so CI on Linux and macOS is its
  first check. On Windows there is no way to signal R without `unsafe`, which
  is forbidden, so interrupt stops the session and the caller restarts it;
  the objects in memory are lost. Windows is tested and works that way.
  Better ideas to weigh later: keep a checkpoint of the workspace so a stop
  can restore it, or a vetted safe wrapper for console control events.
- Output is line-buffered by R when piped, so progress text can arrive late
  unless the code calls `flush.console()`.
- Standard output and standard error are read on separate threads, so their
  relative order is approximate.
- Windows interrupt: decided 2026-10-26 to keep stop-and-restart for now.

## P1.4 supervision, 2026-10-26

- R reports its own process id in the READY frame. On Windows `Rscript`
  starts R as a child, so this is the id to measure and signal.
- `SessionConfig::max_run_time`: a run over it is interrupted
  (`Event::LimitReached`, `Limit::Time`); if R is still going 10 seconds
  later (code that never checks for interrupts) it is stopped outright.
- `SessionConfig::max_memory_bytes`: R's memory is read every 2 seconds
  during a run with `tasklist` (Windows) or `ps` (elsewhere), so there is no
  `unsafe` code; over the limit R is stopped (`Limit::Memory`) because a
  process out of memory may not answer an interrupt. Restart to go on.
- Both limits are off by default; the interface will set them.
- Tested against R 4.4.3 here: the time limit, the memory limit (a 640 MB
  vector against a 300 MB limit) and restart afterwards. The `ps` path is
  covered only by its parser tests, since this is a Windows machine; CI on
  Linux and macOS runs the real thing.
- Not covered: a hung R that is idle (not in a run) and does not answer. The
  time limit covers hung runs only.

## Not done

- CI (GitHub Actions, run on `p0/scaffold`): the first run failed on Ubuntu
  only; the cause was real, standard error text could arrive after the
  end-of-run frame and be dropped. R now sends a SYNC frame on standard
  error before END and a run finishes only when both are in. The second run
  passed on Ubuntu, Windows and macOS, so the Unix interrupt, the `ps`
  memory check and the pinned-R step all work.

## Phase 2 so far, 2026-10-26

- P2.1 history and recall: `engine::history` (`History`, `Recall`): the
  commands run, the Page Up/Down and arrow stepping that returns to the
  half-typed text, blank and repeated commands skipped, capped at 10,000.
- P2.3 results: `engine::results`, one block per command, text joined,
  errors, warnings and notices kept apart, capped at 64 MB.
- P2.4 log and script: `engine::transcript`; the log (`> code`, output) can
  also follow into a file; the script keeps only commands that ran without
  error, with a comment naming menu entries.
- `Engine` ties a session to these; `submit` and `poll` are all an interface
  needs. 17 unit tests and 9 integration tests against real R pass.
- `rcraft` with no arguments is a terminal console on the engine (one
  command per line, `:history`, `:log`, `:script`, `:restart`, `:quit`).
- Top-level errors read `Error: message`, not R's internal call.
- Not done: P2.2 Tab completion, the egui window (Command, Results,
  History panes), multi-line input (a line that is an unfinished R
  statement should wait for more, not fail), the clicking of History
  entries.
- Windows Application Control sometimes blocks a freshly built test or app
  binary (os error 4551); running it again a little later works. All checks,
  including `cargo xtask ci`, now pass here.

## Decisions made

- Architecture: RCraft **drives a real R** as a separate child process. It
  does not reimplement R and never links `libR` (keeps the licence MIT OR
  Apache-2.0).
- Layered Rust crates and the catalog-of-dialog-specs model follow StatCraft.
- Generated R code is the product: every menu action shows and logs R.

- Added 2026-10-26: Phase 1A (Packages: search, describe, install, update;
  the `ssc install` and `net search` equivalent, with `rc install` and `rc
  search` command forms) and Phase 2A (Help: R help pages, help search,
  vignettes, per-dialog help). Network rules are in AGENTS.md section 1 and
  `docs/network.md`.

- Added 2026-10-26: Phase 2B (Stata mode): a switch and button that take
  Stata code, translate it to R, show both side by side and run the R. A
  translator, not Stata; unsupported commands are marked, never guessed.
  Clean-room rules in AGENTS.md section 1; new `statatr` crate planned at L3.
- P1.1 (find installed R) is done and committed; R is not installed on this
  machine, so `rcraft --find-r` reports none.

- Researched 2026-10-26: run an existing do-file on R (`plan/do-file-run.md`,
  Phase 2C). No mature tool exists; StatCraft's parsers are reusable.
  Recommended approach: idiomatic R plus a small shipped R runtime package
  for Stata-specific rules; verified against recorded Stata output.

## Decided 2026-10-26 (open items worked)

- P0.5: `ENVIRONMENT` pins R 4.4.3 and a package snapshot dated 2025-03-15;
  CI installs that R with `r-lib/actions/setup-r`. Neither the snapshot date
  nor the CI step has run yet (no remote), so the first CI run confirms them.
- Pinokio's conda installs are found with no setting: Pinokio's home comes
  from `PINOKIO_HOME` or `~/.pinokio/config.json`, then `bin/miniforge` and
  `bin/miniconda`. `rcraft --find-r` now reports R 4.4.3 on this machine.
- Do not install packages into another app's environment. The R found here
  belongs to Husky Eye View (`hev-r`, 62 packages, no tidyverse). RCraft uses
  its own user library for installs (P1.5, Phase 1A).
- Shared parsers: extract StatCraft's do-file reader, syntax and expression
  parsers into a shared crate when P2C.3 starts, and depend on that; do not
  copy them here. That change is made in the StatCraft repository, so it is
  not done from this one and needs the maintainer's go-ahead.
  Done 2026-10-28 with that go-ahead, on StatCraft branch
  `lang/shared-parsers` (pull request nu-stats/statcraft#2): `statcraft-lang`
  now holds the do-file reader (`script`), the text-only command syntax
  (`syntax`: `parts`, weights, `options`, `abbrev`, `wildcard`) and the
  expression parser (`expr`), and depends only on `statcraft-core`.
  **Changed 2026-10-28: RCraft copies these files and does not depend on
  them.** StatCraft's repository is private and RCraft's is public, so a git
  dependency would break every outside build and RCraft's own CI. See
  "P2C.3 front end" below.
- `rcraftstata` (the shipped R runtime package) lives in this repository
  under `r/rcraftstata/` and is installed into RCraft's own user library on
  first use, with the user's yes. Created when P2C.4 starts.

## Open questions

- (Decided: Windows interrupt stays stop-and-restart; see P1.4 above.) Was: a
  checkpoint so a stop does not lose the workspace?
- Next: P1.4 watchdog and memory limit, P1.2 (download a portable R when
  none is found), then Phase 2 (Command window and Results).

- Stata mode: share StatCraft's parser (`lang` crate) or write a new one?
  Proposed: start from StatCraft's if its licence and layering allow.
- Stata mode output: R's own output first; Stata-style tables later.

- Phase 1A providers: CRAN and Bioconductor first; r-universe and GitHub
  after? Download counts from a third-party service are optional.

- Generated code style default: tidyverse with a base-R toggle (proposed).
- Public name check for "RCraft"; the repository URL in `Cargo.toml` is a
  placeholder.
- Portable R source for first-run download on each platform.
- First milestone: Phases 0 to 3 as a usable "open a file, look at it,
  clean it, run R commands" app, before any statistics menus.

## Next tasks

- Commit the scaffold (P0.1 to P0.4, P0.6)
- P0.5 pin R and the package snapshot
- P1.1 find an installed R

## Desktop window, 2026-10-26

- Layout B (Workbench) is built in egui (`crates/ui-egui`, app in
  `apps/rcraft-desktop`): menu bar, Variables over History on the left,
  Script above Results, Command line with Run and Stop, and a status bar.
  Page Up/Down and the arrows recall earlier commands, clicking a History
  entry puts it back, Ctrl+Enter in the Script runs the selection or the
  current line. Accents (7, Teal default) and light/dark are under Edit.
  Variables and the Data, Statistics, Graphs, Packages menus are
  placeholders until those phases.
- Drawing uses `wgpu` first and OpenGL as the fallback. On this machine
  neither opens (OpenGL 2.0 not available; `wgpu` finds no adapter), in the
  user's own terminal as well as mine, and StatCraft's window fails the same
  way. Cause found: Windows reports the Radeon RX 570 with device error
  Code 31 (its graphics driver does not load; the driver is dated 2023-03-29
  on Windows build 26300), so only Microsoft's basic display adapter is in
  use. Not a remote session. A reboot or a newer AMD driver should fix it;
  the user did not want to reboot yet, and Win+Ctrl+Shift+B did not help.
- So the window can be seen without a GPU:
  `rcraft-desktop --picture FILE.png [--demo FILE] [--dark] [--accent NAME]`
  draws the real window to a PNG on the CPU (`softraster.rs`, `headless.rs`),
  with a live R session. Layout B was checked this way, light and dark.
  The live window now falls back to the same CPU drawing when neither `wgpu`
  nor OpenGL can start (`softwindow.rs`, winit and softbuffer), or with
  `rcraft-desktop --software`. Checked here: it opens, titled RCraft, and
  responds; how it looks and feels on screen still needs a person to confirm.
  Dev builds optimise the drawing crates (root `Cargo.toml` profile) so it is
  usable without a release build.
- Building on this Windows machine: the Rust toolchain is the GNU one and the
  newer `windows-sys` crates need `dlltool` and `as`, which are not
  installed. `Cargo.lock` was seeded from StatCraft's, and the build worked
  with the MinGW binutils from the Husky Eye View R environment appended to
  `PATH` (`...\envs\hev-r\Library\x86_64-w64-mingw32\bin` and `...\Library\bin`).
  Installing MinGW (or the Visual Studio build tools) would be cleaner.

## Data-bridge spike and CI, 2026-10-26

- Pushed through `1fe9532`; CI: Windows and macOS passed, Ubuntu failed to
  build (no X11 development libraries on the runner). Fixed in `7a4d455`
  (apt install step in `ci.yml`), not yet pushed.
- Data bridge (`plan/data-bridge.md`): a base-R-only binary column format
  beats delimited text (write 0.38 s vs 7.4 s, read 0.09 s vs 1.1 s, 54 MB vs
  82 MB for 1M rows) and keeps types and missing values. Arrow and Parquet not
  yet measured (need the `arrow` install, which needs the user's yes);
  Application Control blocked `data.table`'s DLL, so no compiled package can be
  assumed. Phase 3 may start once the user has read the recommendation.

## Data bridge extensions and a session bug, 2026-10-26

- `rcraft-rdata` now holds the bridge: `write_code` (R code, base R only),
  `read_file`/`decode`, `Table`/`Column`/`Data` with labels and value labels.
  See `plan/data-bridge.md`. Tests: 9 unit, 3 against real R (`engine/tests/bridge.rs`).
- Found and fixed while testing: output with no final line break, such as
  `cat("hi")`, hung the run for good, because R's end-of-run frame was glued to
  the text and not recognised. The readers now find a frame after text
  (`rproto::parse_frame_after_text`). Regression test in `rhost/tests/session.rs`.
- CI on Ubuntu needed more X11 development libraries; all three systems were
  green at 425c378.

## Phase 3 started: workspace, Variables pane and Data browser, 2026-10-26

- Engine: hidden queries (`engine::workspace`, `Engine::watch/unwatch/table/
  workspace/refresh`) list R's workspace and bring watched data frames over
  through the bridge, after every command. They never reach the history, log or
  Results, and a command typed while one runs waits for it (up to 30 s) instead
  of being refused. Table files are numbered process-wide (found by a flaky test:
  engines in one process shared a file name). 5 tests against real R.
- `rdata::display`: cell text as R prints it (7 significant digits, ISO dates,
  `NA`), 4 tests.
- Window: Variables pane with a data-frame switcher, kind, missing count, label
  and value labels on hover, click to put the name in the Command line; a
  Results / Data tab switch; the Data tab is a virtualised grid (only visible
  rows and columns are drawn) with a pinned header. `--picture ... --data`
  shows it offscreen. Checked on a 5000 x 7 table.
- Known limits: the whole table is brought over (cap 200 million cells; a table
  over 5 million cells is refreshed only on request, the button for that is not
  built yet); the UI thread owns the engine, so a very large fetch can pause
  the window while a command is submitted; the History list wraps long
  commands (should truncate); no sorting, editing or column resizing yet.
- Next in Phase 3: P3.2 "Use my file" (csv first), P3.4 Objects pane, editing.

## P3.2 Use my file: delimited text, 2026-10-26

- File > Use my file… (and dropping a file on the window) opens a dialog: the
  file is looked at (`rdata::sniff`: text encoding including byte order marks
  and UTF-16, delimiter, header row, decimal mark; binary files such as .xlsx
  are refused with a reason), the choices are filled in and can all be changed
  (delimiter, decimal mark, quotes, encoding, lines to skip, missing-value text,
  header, trim spaces, text as factors, the name), the first lines of the raw
  file are shown, and so is the R it will run (`rcodegen::CsvImport`, base R
  `read.table`, no package needed). Import runs it as a menu command, so it is
  in the history, the log and the script, and the new data frame is shown in the
  Data tab; a failed import leaves the Results with R's error.
- Tests: 13 sniffing (including hostile bytes), 5 code generation, 6 end to end
  against real R (quotes with embedded line breaks, Latin-1, semicolons with
  decimal commas, UTF-8 and UTF-16 with byte order marks, `#` inside data, skip,
  no header, custom missing text, a missing file), 3 dialog, 1 app-level
  (import switches the view; a failed one does not).
- Not done in P3.2: Excel (needs `readxl`, so sheet and range choices wait for
  it), Stata/SPSS/SAS (`haven`), R data, Parquet, a Browse button (no file
  dialog library yet; paste a path or drop a file), a preview of the parsed
  table before importing (the raw lines are shown; the parsed table appears in
  the Data tab after Import), tidyverse code style (`readr`) as an option.
- Added a thin border to inactive widgets so text boxes show on white windows.
- `rcraft-desktop --picture ... --import FILE` shows the dialog offscreen.

## P3.2 continued: Excel, Stata, SPSS, SAS, R data, 2026-10-26

- Done with the user's go-ahead to install `readxl` and `haven` (21 packages
  from the CRAN cloud mirror, into RCraft's own library, nothing added to
  Pinokio's HEV R). RCraft's library (`rhost::library`) is put first on every
  session's R search path when it exists: `%LOCALAPPDATA%\RCraft\R-library`
  on Windows, `~/Library/Application Support/RCraft/R-library` on macOS,
  `~/.local/share/rcraft/R-library` on Linux. The packages loaded here, so
  Application Control did not block these compiled packages.
- Heads-up about this machine: the Claude app's sandbox redirects
  `%LOCALAPPDATA%`, so the first install went into the app's private store
  (`...\Packages\Claude_...\LocalCache\Local\RCraft`). The same folder was
  copied to the real `C:\Users\jstow\AppData\Local\RCraft` from the user's
  terminal, so RCraft run from the user's own PowerShell finds the packages.
- `rcodegen::files`: `FileKind`, `ExcelImport` (`readxl::read_excel`: sheet,
  range with a checker, header, skip, missing text, trim, guess rows),
  `HavenImport` (`read_dta`, `read_sav`, `read_por`, `read_sas`, `read_xpt`;
  labels kept as attributes or turned into factors; encoding; SPSS user-missing),
  `RObjectImport` (`readRDS`, `load`), and `Import`, one type for them all, with
  `preview_code` (R prints the first rows) and `package()`.
- Engine: `Engine::query` / `query_result`, hidden questions to R (no history,
  log or Results). The dialog uses them for the package check, the Excel sheet
  names and the preview.
- Dialog: sheet picker, range, label and encoding options, "How R reads it"
  preview (with a "choices changed" hint), and a missing-package panel that
  shows the exact `install.packages` call and the library folder and installs
  only on a click; the install runs as an ordinary command (Results, log) and the
  dialog checks again when R is free. Import and Cancel are pinned below the
  scrolling part of the window.
- Tests: 12 code generation, 5 end to end with real files (a two-sheet `.xlsx`
  built by hand in the test, and `.dta`, `.sav`, `.xpt`, `.rds`, `.RData` written by
  R), 5 dialog (preview from R for text and Stata, the missing-package flow). CI
  now installs `readxl` and `haven` so these run there too (first run of that
  step: see below).
- Found while testing: Stata and SPSS files have no missing text, so a missing
  string reads back as an empty string (the format's own rule). A "treat empty
  text as missing" choice is a candidate. Not tested against a real `.sas7bdat`
  (haven can only read, not write, them; the code path is `read_sas`).
- A table the preview reads only the first rows of says "first N rows", never a
  total.

## P3.3 Labels (first part)

- CI is green on all three systems at 0064699 (the finder prefers `R_HOME`; CI
  sets it to the pinned R).
- Data grid: "Show labelled values as" Labels / Values / Both. A numeric code with
  a value label shows the label (or `label (code)`); codes with no label, missing
  values and other columns show as they are. Labelled columns sit on the left.
- Variables pane: the variable label shows after the type, and a "Conversion
  report" under the list says what was kept (variable labels, value labels,
  SPSS user-missing), changed (columns shown as text only) and dropped
  (attributes RCraft cannot carry). Nothing is shown for a plain table.
- `rdata::display::{LabelView, labelled_text}` and `rdata::summary::Report`, with
  unit tests; checked in a picture with a `haven` labelled table.
- Not yet: editing labels, turning labelled columns into factors from the grid,
  a per-variable value-label list in the pane, labels in dialog variable pickers.

## P3.4 Variables and Objects panes

- The left pane has two tabs. Variables: the data-frame picker, a type filter (All
  types, Numbers, Text, Factors, Dates and times, Logical, With labels) with an
  "N of M variables" note, and the factor level count after the type. Objects:
  everything in R's workspace with its class and, for data frames, rows by columns.
  A data frame opens on a click (and the pane returns to Variables); other objects
  put their name in the Command line.
- `TypeFilter` and `object_list` in `ui-egui/src/data_view.rs`; unit test for the
  filter, and an R-backed app test that the Objects list sees a data frame and a
  non-frame and that the tab and filter are remembered.
- Picture options `--objects` and `--filter TYPE` added to the desktop picture mode.
- Not yet: a level list per factor, and rename/remove/save from the Objects list.

## P3.5 Data management dialogs (first part)

- The dialog-spec format (`plan/dialog-spec.md`): `rcraft_rcodegen::dialog` holds
  `Spec`, `Widget` (Variable with kind filters, Text with Name/Expression rules,
  Choice, Check), `Values`, `Context`, `Style` (base R, tidyverse via `dplyr::`),
  per-widget and per-spec checks, and `Args`, which quotes every name so code
  functions never join raw text. One renderer, `ui-egui/src/spec_dialog.rs`, draws
  any spec with its hints, problems, a code-style choice and the R that will run.
- Data menu: New variable, Filter rows, Sort rows, Rename variable
  (`catalog::data`). Each puts its result in the data frame named in the dialog
  (its own name by default, which replaces it; a new name keeps the original and
  the window switches to it).
- Tests: format checks (7), snapshot tests of the R in both styles (6), a real-R
  run of every base-R dialog on a made-up data frame, renderer tests (2) and an
  app test that opens a dialog and runs it as a menu command.
- Picture options `--dialog NAME` and `--set ID=VALUE`.
- The four entries are `partial`: the tidyverse code is snapshot-tested only, as
  CI has no dplyr. Recode, type conversion, factor, date and string tools next.

## P3 leftovers: labels, Objects actions, recent files, Browse, Parquet, 2026-10-27

- Labels (P3.3): Data menu dialogs *Variable label*, *Value labels* (`1 = Low; 2 = High`,
  replaces the old labels, empty removes them) and *Labelled to factor* (unlabelled
  values become missing, the variable label is kept). Base R only. Run against real R.
  `Labelled to factor` is a function call so its names cannot clash with the data
  frame's (a result named `f` broke the first version).
- Objects (P3.4): a right-click menu on any object: *Rename* (refuses a name that is
  taken), *Remove*, *Save* (`saveRDS`; will not replace a file unless asked). They are
  specs (`catalog::data::OBJECT_SPECS`) drawn by the same dialog, so the R is shown and
  goes into History as `Objects > ...`.
- Recent files (P3.2): `rhost::recent` keeps the last ten in `recent-files.txt` in
  RCraft's own folder (next to `R-library`); "Use my file" lists them, dimming ones
  that are gone. Pictures and tests use an in-memory list (`--recent PATH`).
- Browse (P3.2): the system's file chooser through `rfd` (UI crate only; on Linux the
  portal backend, so no GTK needed). The chooser opens where the file in the box, or the
  newest recent file, is. **Not clicked in a real window here**, only built and its
  start-folder logic tested.
- Parquet and Feather (P3.2): `arrow::read_parquet` / `read_feather` through the same
  missing-package panel as Excel and Stata; the preview reads an Arrow table and shows
  only the first rows. CI now installs `arrow` with the other packages so the real-R
  test runs there. **Not run on this machine** (no `arrow` here), only in CI.

## P3.5 value-changing dialogs, 2026-10-27

Ten new Data menu dialogs in `catalog::data_tools`, all base R (no package needed),
each applied as a small function to the column so its names (x, y, lab) cannot clash
with the data frame's, into the variable itself or a new one, keeping the variable label:

- *Recode* (`1, 2 = Low; 3 to 9 = High; NA = 0`; first matching rule wins; text news
  make a text variable; "quoted" values are text, so `"NA"` is the text NA).
- *Convert type* (number, text, factor). A factor converts to a number by what it says,
  not by its internal codes.
- *Factor* submenu: reference level, combine levels, reorder levels (R refuses a
  level that is not there, or a list that leaves one out, rather than making values missing).
- *Text* submenu: case and spaces, find and replace (literal unless the pattern box is
  ticked), extract part.
- *Date* submenu: text to date (a value that does not fit becomes missing and R warns
  how many), date part (year, month, day, weekday, quarter).

The Data menu now draws a dialog whose menu path has three parts in a submenu
(`app.rs: menu_group`). Tests: unit tests in `data_tools.rs`, and
`crates/engine/tests/data_tools.rs` runs every dialog against real R (passes on Windows;
CI runs it on the three systems). Tidyverse styles for these are not written; the
dialogs are base R only.

## P3.5 package panel and dplyr in CI, 2026-10-27

- Spec dialogs now ask R which packages the chosen code style needs
  (`Style::packages`, so dplyr for tidyverse) with `missing_packages_code`, and show
  the same install offer as "Use my file" when one is missing: the R it will run, an
  Install button (an ordinary command, `Packages > Install`, into RCraft's own library),
  Check again. Run waits until R says they are all there. Base R dialogs never wait.
  The logic is `ui-egui/src/packages.rs` (`PackageCheck`), shared with the import
  dialog's `install_code` and `library_dir`.
- A real-R test caught the first version treating R's empty answer (a bare line break)
  as a missing package; blank lines are ignored.
- CI installs dplyr with readxl, haven and arrow. `tidyverse_style_gives_the_same_tables_as_base_style`
  (engine/tests/data_dialogs.rs) runs sort, filter, compute and rename in both styles and
  compares every cell; it skips with a note where dplyr is not installed, which is the
  case on this machine, so it has only run in CI. **The panel's drawing was not looked
  at in a window**, only its logic and R's answers were tested.

## P4.1 Descriptives, 2026-10-27

Six Statistics menu dialogs in `catalog::stats`, under *Statistics > Descriptives*, all
base R (`stats` ships with R). Each prints its tables in Results and changes no data;
each is one call of an anonymous function on the chosen columns, so no helper name lands
in the user's workspace.

- *Summary statistics*: n, missing, mean, sd, min, quartiles, max for number variables,
  all together or split by a group; none chosen means every number variable; an optional
  name saves the table.
- *Frequency table*: one table per variable, percentages and missing-as-a-category optional.
- *Cross-tabulation*: counts with totals, row/column/total percentages, chi-squared test.
- *Correlation*: Pearson, Spearman or Kendall, pairwise or complete rows, rows used per
  pair, optional p-values (unadjusted).
- *Missing-data patterns*: missing count and share per variable, and the patterns.
- *Normality test*: Shapiro-Wilk; a constant or too-short variable gives NA, not an error.

The dialog-spec format gained `Widget::Variables { min }` (several columns, in order
chosen; `min` 0 means none chosen = all of the allowed kinds), `Values::list`, and
`Args::vars` / `vars_or_all`. The renderer draws a scrollable checkbox list with Select
all and Clear. The Statistics menu is drawn from `stats::SPECS` with the same submenu
logic as Data (`spec_menu`). `--set vars=a,b` sets a multi-variable widget in pictures.
Tests: unit tests in `stats.rs` and `dialog.rs`; `engine/tests/stats.rs` runs every
dialog on a small data frame in real R and checks the numbers (saved summary tables
against `sd()`, `mean()` and so on, correlations, patterns, Shapiro); an app test runs
the Correlation dialog through the menu path and finds the table in Results.

## P4.2 Tests and intervals, 2026-10-27

Ten dialogs in `catalog::inference`, under *Statistics > Tests and intervals*, base R
(`stats::`), each printing R's own test report in Results and changing no data:
one-sample, two-sample (Welch or pooled) and paired t-tests; test of proportions (one
proportion against a value, or between groups); exact binomial; Fisher's exact;
Mann-Whitney; Wilcoxon signed-rank; Kruskal-Wallis; variance tests (Brown-Forsythe,
Bartlett, F). Alternative (two-sided, less, greater) and confidence level (90, 95, 99%)
are choices. The dialog-spec format gained `TextRule::Number` (one finite number,
inserted as typed) and `Args::number`.

Tests: `engine/tests/inference.rs` runs each dialog in real R and requires its printed
report to equal what R prints for the same call written by hand (the `data:` line
aside), so the options chosen really reach R; a three-group two-sample test is refused by
R and its message shown. Unit tests give the exact code. A hand-written reference caught
a wrong expected count in my own test (the dialog was right).

Limits: a two-sample test needs exactly two groups and R checks that when it runs; the
Brown-Forsythe test is the median-centred Levene test written in base R. Agreement
measures (Cohen's kappa) are not done.

## P4.3 Linear models and ANOVA, 2026-10-27

Three dialogs in `catalog::models`, under *Statistics > Linear models*, base R (`stats::`).
Each fits a model, keeps it in R's workspace under a name the dialog asks for (default
`model`, `anova`; a name already in use is replaced, but never the data frame's own name),
prints R's own `summary`, and changes no data.

- *Linear model*: outcome, one or more number or categorical predictors, no interactions
  or all two-way ones, optional weights, confidence level for the printed intervals.
  Categorical predictors use R's treatment contrasts (first level is the reference).
- *One-way ANOVA*: the table, group means, optional Tukey comparisons.
- *Factorial ANOVA*: several grouping variables, full factorial or main effects only,
  optional Tukey comparisons. Sequential (type I) sums of squares, said in the dialog:
  with unequal group sizes the order of the variables matters. Types II and III need
  the car package; not done.

Tests: `engine/tests/models.rs` requires the saved model to equal R's for the same call
written by hand (`all.equal` on the coefficients) and the printed summary, group means and
Tukey report to equal R's own; unit tests give the exact code and the refusals.

Open design for P4.5: post-estimation needs to know "the last model". Today the model is
only an object in the workspace under the saved name; the menu will probably list the
workspace's `lm`/`glm`/`aov` objects (the Objects pane already knows their classes)
rather than track a hidden "last model".

## P4.4 Logistic and Poisson models, 2026-10-27

Two dialogs in `catalog::generalised`, under *Statistics > Generalised models*, base R
(`stats::glm`). Each keeps the fitted model in the workspace under a chosen name (never the
data frame's own), prints R's `summary` and, if ticked, the exponentiated coefficients with
confidence intervals.

- *Logistic model*: a yes/no outcome (number, logical, factor or text) with logit, probit
  or complementary log-log link; odds ratios for the logit link. The generated R first
  stops unless the outcome has exactly two different values, because `glm` would
  otherwise treat a three-level factor as first-level-versus-the-rest without saying so.
- *Poisson model*: a count outcome, Poisson or quasi-Poisson, an optional exposure that
  becomes `offset = log(exposure)`, rate ratios.

Tests: `engine/tests/glm.rs` requires the printed report (summary and ratio table) to equal
R's for the call written by hand, and the saved model's coefficients to equal that call's
(including links, a categorical predictor and the offset, which changes the coefficients);
a three-level outcome is refused with the dialog's own message and no model is saved.

Not done: negative binomial and ordinal (MASS) and multinomial (nnet) models. They come with
most R installs as recommended packages but not all (conda's R lacks them), so each needs the
package panel for its own dialog: `Spec` has no per-dialog package list yet, only the code
style's (`Style::packages`). Next design step: a `needs` field on `Spec` that the panel
reads together with the style's packages.

## P4.4 negative binomial and per-dialog packages, 2026-10-27

- `Spec` gained `needs: &[&str]`: R packages a dialog needs whatever the code style
  (`Spec::packages(style)` is the dialog's own plus the style's, each once). The package
  panel and Run gate read it, so a dialog that needs MASS shows the same install offer as the
  tidyverse style does for dplyr. The panel text now says "this dialog needs".
- *Negative binomial model* (`generalised::NEGATIVE_BINOMIAL`, needs MASS): `MASS::glm.nb`,
  an exposure becomes `offset(log(exposure))` in the formula, rate ratios optional. Its summary
  includes theta.
- `engine/tests/glm.rs` requires the printed report and ratio table to equal R's for the call
  written by hand and the saved model's coefficients to equal it (the exposure changes them).
  The test skips with a note where MASS is not installed; CI now installs MASS with the other
  packages so it runs there. MASS is present in the R used here (4.4.3). nnet is not, which
  matters for the multinomial dialog next.

## P4.4 ordinal and multinomial models, 2026-10-27

- *Ordinal model* (needs MASS): `MASS::polr(ordered(outcome) ~ ..., Hess = TRUE)` with logit,
  probit or complementary log-log link. Wrapping the outcome in `ordered()` makes the order
  explicit: a factor keeps its level order, text is alphabetical, numbers go by size. The
  generated R prints "Outcome order, lowest first: low < mid < high" so the order used is never
  silent. Optional p-values (normal approximation to each t value) and, for the logit link,
  odds ratios with profile confidence intervals.
- *Multinomial model* (needs nnet): `nnet::multinom(factor(outcome) ~ ..., trace = FALSE)`, an
  optional reference category (`relevel`; R refuses one that is not a category), p-values and
  relative risk ratios with confidence intervals. The generated R first stops unless the outcome
  has at least three different values.
- `engine/tests/glm.rs` requires each printed report to equal R's for the same call written by
  hand and each saved model's coefficients to equal that call's, plus the reference category's
  effect on the rows. The tests skip with a note where MASS or nnet is missing. nnet is not in
  the conda R used here, so it was installed from CRAN into a scratch library
  (`R_LIBS`) to run the multinomial test for real; CI installs MASS and nnet. P4.4 is done.

## P4.5 Post-estimation (first part), 2026-10-27

Decision recorded: there is no hidden "last model". A model is an object in R's workspace under
the name its dialog saved it as (`model`, `anova`, ...), and right-clicking a model in the Objects
pane (class `lm`, `glm`, `aov`, `negbin`, `polr` or `multinom`; `postest::is_model_class`) shows a
*Model* submenu. The dialogs are specs in `catalog::postest::MODEL_SPECS` with menu paths under
"Objects > Model", opened through the same object-dialog path as Rename and Save, so the model's
name is the dialog's object and the R is shown and logged as usual.

- *Fit statistics*: observations, log-likelihood, AIC, BIC; R-squared for linear models, null and
  residual deviance for generalised ones.
- *Compare models*: a table of df, log-likelihood, AIC and BIC for this model and others named in
  the dialog, and, if ticked, an F or likelihood-ratio test of nested models (R refuses or warns
  when they are not fitted to the same rows).
- *Predicted values and residuals*: new variables in a named data frame, in place; predicted
  values on the outcome's or the link scale; deviance, Pearson or raw residuals. Rows the model
  dropped for missing values get missing, matched by row name; a data frame the model was not
  fitted to is refused.
- *Diagnostics*: Shapiro-Wilk of the residuals and a Koenker version of the Breusch-Pagan test
  (linear models), Cook's distance and leverage counts and the five most influential rows (all
  linear and generalised linear models). Worded as a "Koenker version" because it is not claimed
  to equal any other program's default.
- *Robust standard errors* (needs sandwich and lmtest): `lmtest::coeftest` and `coefci` with
  `sandwich::vcovHC` HC0 to HC3. Not clustered yet.

Tests: `engine/tests/postest.rs` fits models in R by hand and checks each dialog's output or
added columns against what R gives directly (an NA row is checked to line up). The robust
standard errors test skips where sandwich or lmtest is missing, which is the case on this
machine, so it has only run in CI (which now installs both).

## P4.5 Post-estimation (rest), 2026-10-27

- *Robust standard errors* can cluster: a cluster variable and the data frame that holds it;
  `sandwich::vcovCL` with the cluster values matched to the rows the model kept by row name
  (`rownames(model.frame(model))`), so a row dropped for a missing value cannot shift the groups.
- *Test terms* (base R): each term dropped in turn (`drop1`, F test for linear models and
  likelihood-ratio for generalised ones), or several terms together against the full model
  (`update`, then `anova`); the generated R first checks the names are terms of the model.
- *Average marginal effects* (needs marginaleffects): `avg_slopes`, so a logistic model gives
  changes in probability. CI installs marginaleffects.
- Tests: test terms ran for real against R. The robust, clustered and marginal-effects tests skip
  where their packages are missing; sandwich/lmtest are missing here, so the clustered and robust
  tests have only run in CI.
- A flaky comparison fixed: `Waiting for profiling to be done...` is R's progress note on stderr,
  whose place among the stdout tables is approximate (the two streams are read on separate
  threads), so the glm tests now ignore that line. It made the logistic and ordinal tests fail on
  Ubuntu in one run and pass in others.

## P5.1 Graph capture and the Graph tab, 2026-10-27

How plots reach RCraft (this changes `rproto/bootstrap.R`, so it wants the same review as the rest
of the session protocol):

- The session creates a plot folder inside its own temp folder and names it to R in the
  environment variable `RCRAFT_PLOT_DIR`. R's default graphics device becomes one `png()` device
  with its display list enabled, **kept open between commands**, so `plot()` in one command and
  `abline()` in the next work as in any R console. (The first version closed the device at the end
  of each command and broke exactly that; a picture of the window showed it, and the design was
  changed before any more was built on it.)
- A snapshot of the page is written, with `recordPlot()` and `dev.copy()`, as `<run id>-001.png`,
  `<run id>-002.png`, ... when a command ends with the page changed (compared with the last
  snapshot's display list), and just before a new page replaces it (hooks `before.plot.new` and
  `before.grid.newpage`, so base graphics and grid/ggplot2 are both covered). One page drawn on over
  two commands gives two snapshots, the second including the added line.
- `rhost::Session` sends `Event::Plot { run, path }` for each snapshot just before `Finished`, and
  removes the folder with the session. The engine keeps them as `Item::Plot` in the Results (so
  the history is the Results), the log says "(a plot was drawn; see the Graph tab)", and the
  terminal console prints the file's path.
- A device the user opens themselves (`png("x.png")`) is left alone and not captured; `dev.off()`
  in the user's code closes the live device, and the next plot opens a new one.
- The Graph tab (`ui-egui/src/graph_view.rs`): older and newer, "Plot 2 of 3", the command that
  drew it, the picture fitted to the pane (never enlarged), Save as PNG (copies the file). It opens
  by itself when a new plot arrives and then follows the newest. PNG decoding uses the `image`
  crate (png feature only), already in the dependency tree through eframe.
- Tests: 9 new host tests against real R (one plot, three pages in order, no plot, runs not sharing
  plots, a plot before an error, a run stopped midway, folder removed with the session, a plot
  added to by the next command, the user closing or opening their own device), an engine test, an
  app test for the tab switching, and unit tests for loading and fitting. A picture of the window
  was rendered and looked at.
- Not done: zoom, copy to the clipboard, the plot dialogs of P5.3, the ggplot2 builder (P5.2).
  CI on Linux and macOS is the first run of the png device there; if a runner has no graphics
  capability for `png()` the tests will say so.

## P5.3 Graphs menu, and two bugs in the capture, 2026-10-27

Eight dialogs in `catalog::graphs` under *Graphs* (submenus One variable, Two variables, Several
variables), base graphics only: *Histogram* (R's rule, Scott, Freedman-Diaconis, or a number of
bars; optional density curve), *Density plot* (one curve, or one per group), *Box plot* (optionally
by group), *Bar chart* (counts or percent), *Normal Q-Q plot*, *Scatter plot* (optional colour by
group, straight-line fit, lowess smooth), *Line plot* (a date or number against a number) and
*Scatter plot matrix*. A title is optional and defaults to a plain one naming the variables.
*Diagnostic plots* (four panels on one page, the layout put back afterwards) is in the Objects
pane's Model menu, which finishes P4.5. Not done: heatmap, survival curves.

`engine/tests/graphs.rs` runs every dialog (with and without its options) in real R and requires it
to draw **exactly one** plot, a real PNG, and to survive missing values; a typed `abline()` after a
scatter dialog finds the plot. Those tests found two real bugs in the capture, fixed in
`bootstrap.R` with a regression test each:

- `barplot()` calls `par()` before `plot.new()`, which opens the device, so the "a new page is
  about to replace this one" hook snapshotted a blank page first. A page now counts only once
  `plot.new()` or `grid.newpage()` has begun it (hooks `plot.new` and `grid.newpage`), and the page
  signature ignores `par()` entries, which change the record but not the picture.
- `pairs()` and `par(mfrow = ...)` call `plot.new()` for each panel without starting a new page, so
  every panel was snapshotted half-drawn (nine files for one scatter plot matrix). The hook now
  fires only when `par("mfg")` says the next call begins a new page.

## Session protocol review fixes, 2026-10-27

Review of `rproto` and `rhost/src/session.rs`; each fix has a regression test
in `rhost/tests/session.rs` (or a unit test for the output reader).

- A `sink()` left on (or a message sink) no longer hangs the session: frames
  written while a sink is on wait for the run's end, sinks still on are turned
  off with a warning saying so. `capture.output()` no longer captures frames.
- Output is read in 64 KiB pieces; a piece no longer ends inside a frame marker
  (lost END: the run never finished) or inside a character (shown as `�`).
- Interrupts are held off except while the user's code runs, so one arriving
  as a request is read or a run is reported cannot lose END. RCraft signals a
  run only once it has seen its BEGIN, and R drops any interrupt still pending
  just before BEGIN, so one sent as a run ends cannot stop the next run (CI
  caught that on Ubuntu with the first version of the fix).
- An interrupt R does not answer is followed by stopping R after 10 s, as the
  limits already were; a timed-out `run_to_end` waits for that, so the session
  is free afterwards.
- `options(warn)` is respected (2 makes warnings errors, negative hides them);
  a warning raised with `signalCondition()` is no longer an error.
- R's last words on standard error when it stops are kept (the exit waits up
  to 2 s for standard error to close).
- `closeAllConnections()` in user code no longer ends the session (the
  request connection is checked by id and reopened).
- Cleanup: unused `current_id` removed; an idle R gets 0.5 s to quit on QUIT.

## P3.6 combine and reshape, 2026-10-27

`catalog::combine`, eight Data dialogs, base R only (`merge`, `aggregate`,
`reshape`), so none needs a package:

- Join (inner, left, right, full, on same-named variables), Append (a variable
  missing from one side is filled with NA), Aggregate (mean, sum, median,
  smallest, largest, standard deviation, count of values present).
- Wide to long and Long to wide, with the `reshape` bookkeeping attributes and
  row names removed; wide to long without identifiers numbers the cases in `id`
  and refuses when the data already has an `id`.
- Duplicates (drop, keep only repeats, or mark), Random sample (rows or
  percent, optional seed), Drop missing values (drop or keep the incomplete rows).

The other data frame of Join and Append is named in a text box (the dialog
cannot yet list the workspace's data frames), and its variables are matched by
name. `engine/tests/combine.rs` runs each dialog in real R and checks the
values; unit tests cover the refusals. Still to do for P3.6: a data-frame
chooser widget, joins on differently named keys, and tidyverse styles.

Data-frame chooser (follow-up to P3.6): `Widget::Frame` picks another data
frame of the workspace from a list; `Context.frames` carries the workspace's
other frames. Once one is chosen the window brings its table over (and lets go
of it when the dialog closes), so Join also refuses a match variable that the
other data frame lacks. Still to do: joins on differently named keys and
tidyverse styles.

## P3.6 finished: differently named keys and tidyverse styles, 2026-10-28

- `Widget::FrameVariables` (in the dialog-spec format): several columns of the
  data frame chosen in a `Widget::Frame`, offered only once that frame is chosen.
  Join uses it for **Match with, in the other data frame**: the partners of the
  match variables, in the same order (`merge(by.x, by.y)`, or
  `dplyr::*_join(by = c("k" = "id"))`). Left empty, the names must be the same,
  as before. The dialog refuses a different count and partners the other frame
  lacks.
- Tidyverse style (dplyr, written to give the base style's table, down to the
  order of rows and columns) for Join, Append, Aggregate, Duplicates, Random
  sample and Drop missing values. Join relocates and sorts by the match
  variables as `merge` does; Aggregate drops rows with a missing group, orders
  by the last group slowest and returns a plain data frame. Wide to long and
  Long to wide stay base R (the tidyverse way needs tidyr).
- `engine/tests/combine.rs` has a test for differently named keys and one that
  runs twelve dialog settings in both styles and requires the same tables
  (a random sample by size only, as its rows are numbered afresh).
- Known differences, not tested: text keys and group names sort in the C locale
  under dplyr and in R's locale under base R; appending a number column to a text
  column is an error in the tidyverse style and a coercion in the base style;
  many-to-many joins may order tied rows differently.
- This machine had no R, so on 2026-10-28 R 4.4.3 (signed installer from CRAN)
  was installed for the user at `%LOCALAPPDATA%\Programs\R\R-4.4.3`, and dplyr
  1.1.4 with its dependencies from the 2025-03-15 snapshot into R's user library.
  `cargo xtask ci` passes here with them, so the R-backed tests ran rather than
  skipped. MASS and nnet come with R; sandwich, lmtest, marginaleffects,
  readxl, haven and arrow are not installed here, so the tests that need them
  skip locally (they print a note) and run in CI.

## P3.7 Save and export, 2026-10-28

`catalog::save`, six dialogs under **File** (a *Save data as* submenu and *Save
workspace*), each with a *Save to* path box and a **Browse** button (the new
`TextRule::SavePath`; the system's save dialog picks the file and, since it has
already asked about replacing, ticks Replace):

- *Text file*: comma, tab or semicolon, missing as `NA` or empty, optional row
  names; UTF-8, quotes doubled (`write.table`, base R).
- *Excel workbook* (`writexl`), *Stata, SPSS or SAS* (`haven`: `.dta`, `.sav`,
  `.zsav`, `.xpt`), *Parquet or Feather* (`arrow`); each names its package in
  `Spec::needs`, so the package panel offers the install.
- *R data*: `.rds` (one object) or `.RData` (named), base R; *Save workspace*:
  `save.image()` to an `.RData`.
- A file is never written over by accident: unless **Replace** is ticked, the
  code starts with `if (file.exists(...)) stop(...)`. The file name must end in
  the format's own ending.
- Not written: `.sas7bdat` and SPSS portable `.por` (haven cannot write them).
  Stata, SPSS and SAS transport have no missing text, so a missing text value is
  saved empty (the dialog says so).

`engine/tests/save.rs` writes with each dialog and reads the file back in R
(text in four variants, the overwrite guard leaving the old file untouched,
`.rds`/`.RData`, the workspace cleared and loaded again, haven's four formats,
an Excel sheet, Parquet and Feather). It found one real bug: the tab separator
was written as a backslash and a `t`; fixed with a regression test. CI now also
installs `writexl`. This machine now has every package CI installs, from the
2025-03-15 snapshot (added 2026-10-28: dplyr, readxl, writexl, haven, arrow,
sandwich, lmtest, marginaleffects), so `cargo xtask ci` runs all 369 tests here
with none skipped for a missing package.

## P5.4 Export plot, 2026-10-28

**Graphs > Export plot** (`catalog::plot_export`), also the **Export…** button on
the Graph tab (it opens the dialog on the plot being shown). A plot is drawn
again on a `png()`, `svg()` or `pdf()` file device of the size asked for, so the
file is sharp at that size rather than the screen picture scaled. Size in inches,
centimetres or pixels (pixels are turned into inches with the resolution, for
SVG and PDF); resolution 36 to 1200 dpi; a PNG is at most 20,000 pixels on a
side, any file at most 100 inches; the file name must end in the format's
ending; a file is never written over unless **Replace** is ticked.

How it works (this changes `rproto/bootstrap.R`, so it wants the same review as
the rest of the session protocol): when `snap()` captures a page it now keeps the
`recordPlot()` under the snapshot's file name in `options("rcraft.plots")`, the
newest 30 only (a recording can hold the data it plots). The dialog's code looks
the recording up, opens the file device and `replayPlot()`s it; a name R no
longer has stops with "that plot is no longer kept". The option is not an object
in the workspace, so `ls()` and *Save workspace* do not see it.

`engine/tests/export.rs` runs it in real R: PNG sizes checked from the file's own
header (2100x1500 at 7x5 in and 300 dpi; centimetres; exactly 800x600 pixels), SVG
`width="504"`/`viewBox` and PDF `/MediaBox [0 0 504 360]` for 7x5 in and
`[0 0 595 841]` for A4, the newest plot, a plot by name, one that is gone, the
overwrite guard, a grid plot, plotting going on after an export, and the cap of 30.
Not done: viewing the dialog and the Export button in a rendered picture of the
window (the logic and the app test pass; how it looks has not been checked).
Limits: a recording replays on a differently sized device, which is right for
base and grid graphics, but a plot that fixes its own size in pixels may not scale.

## P1A.1 Package index (first part), 2026-10-28

New crate **`rcraft-rpkg`** (layer L2, listed in `AGENTS.md`; depends on `core` and
`flate2` with its pure-Rust backend). It never starts R and never touches the
network except through the `Fetcher` trait, so every test is offline. Plan and
reasons: `plan/packages.md`.

- `unpack`: gzip with a size cap (512 MB unpacked), so a small hostile file cannot
  fill memory.
- `rds`: a reader for R's `saveRDS` files, XDR format versions 2 and 3, plain or
  gzip. It handles text, number and logical vectors, lists, pairlists (the
  attributes), symbols, and the compact-sequence and wrapper ALTREP forms; any
  other object (environments, functions) or compression (xz, bzip2) is an error
  naming what it is, not a guess. Every length is checked against the bytes left
  before anything is allocated; nesting is capped at 64 and elements at 200
  million.
- `table`: a text matrix with column names (what `tools::CRAN_package_db()`
  returns) or a list of text columns (a data frame) reduced to the columns RCraft
  uses; that is the whole "trimmed copy".
- `dcf`: `PACKAGES.gz` (Debian control format, with continuation lines and CRLF).
- `index`: `Package` (17 fields), `Index` sorted by name ignoring case with exact,
  ignoring-case and prefix lookup, `Dependency` parsing, a merge that prefers the
  database and fills gaps from `PACKAGES.gz` (and adds packages only it lists),
  staleness against a clock passed in (default one week), and `sample()`, a small
  made-up index for other crates' tests.
- `cache`: one text file `cran-index.txt` in a folder the caller names; count and
  per-line field count checked, written to a `.tmp` name and renamed, so a failed
  write leaves the old cache.
- `refresh`: fetches `PACKAGES.gz` then `packages.rds` from a mirror
  (`https://cloud.r-project.org` by default). A failure or cancel changes nothing;
  a missing database gives a names-only index only when there is no full cache to
  lose; an empty list is never stored.

Tests: 40 in the crate. `tests/r_files.rs` reads files R 4.4.3 wrote itself
(`tests/fixtures`, made-up packages): default gzip v3, v2 uncompressed, a data
frame, xz and a function (both refused), every prefix of the plain and the gzip
file (all errors), and every single-byte change of the plain file (none panics).

**Checked against the real file (2026-10-28, downloaded once with the user's
go-ahead, 12,695,362 bytes, gzip):** it is a gzip text matrix with the expected
column names. It reads to 25,611 rows (25,581 packages after the index drops
unnamed and duplicate rows); every one has a title, a version and a date. Reading
takes about 1.5 s and loading the cache about 1 s in a debug build. The trimmed
cache is 22 MB (the descriptions are most of it). `Rcpp(>= 1.0.5)` with no space
before the bracket parses. Base packages such as `stats` are not in CRAN's
database, so "installed but not on CRAN" has to be handled by P1A.6. The two
tests that use the file (`the_real_*` in `tests/r_files.rs`) run only when
`RCRAFT_REAL_PACKAGES_RDS` names a downloaded copy, so CI needs no network.

**Not done, so P1A.1 is not finished:**

- The two index files have not been fetched live through the app's fetcher (the
  database was downloaded once by hand to check the reader). The HTTP client is
  in (below); its six tests run against a local plain-HTTP server (body and
  progress, error status, cap with and without Content-Length, cancel, a
  cut-short connection, plain HTTP refused by the app's fetcher). One live HTTPS
  request, made with the user's go-ahead on 2026-10-28 to
  `https://cloud.r-project.org/web/views/` (10,248 bytes), succeeded: the TLS
  handshake and certificate check against the real mirror work on the work
  computer's network. `tests/live.rs` repeats it when `RCRAFT_LIVE_URL` is set.
- No UI, no mirror setting, no command in the engine.

**HTTP client (2026-10-28).** `ureq` 3.5.0 with default features off and only
`rustls` (dbbd5c2, its own commit). Licence check of its 36-crate tree, done
before adding: MIT or Apache-2.0 (or both) for all but ring (Apache-2.0 and ISC,
no OpenSSL-licensed files), rustls-webpki and untrusted (ISC), subtle (BSD-3),
rustls (Apache-2.0 or ISC or MIT), wasi (Apache-2.0 with LLVM exception or
Apache-2.0 or MIT) and webpki-roots (CDLA-Permissive-2.0: Mozilla's CA list as
data, which wants its notice kept). It builds on Windows with no extra tools
(ring compiles its own C and assembly). `http::UreqFetcher` is the `Fetcher`:
HTTPS only (redirects too, at most 5), 20 s to connect, 15 min overall,
User-Agent `RCraft/<version>` with the repository URL, proxy settings from the
environment honoured, nothing else sent. Known limit: rustls trusts the bundled
Mozilla roots, not the operating system's store, so a network that inspects TLS
with its own certificate (some campus and company proxies) will fail the
handshake; ureq's `platform-verifier` feature would use the system store and is
the fix if that is reported.

## Phase 1A: search, describe, install, installed (first slice), 2026-10-28

**Packages menu: Find and install, Installed packages** (`ui-egui::package_window`).
CRAN only. The window works with R not running for searching and reading; install,
update and remove go through R.

- **Package list service** (`engine::package_index`): loads the cache on a worker
  thread, builds the search index there, refreshes only when the user presses the
  button (it says which host it asks and how big it is), with progress and Cancel;
  offers a refresh when the list is over a week old and never makes one; the mirror
  is checked (https, no spaces) and kept in `cran-mirror.txt` in RCraft's folder.
  A failed or cancelled refresh keeps the old list; a damaged cache is reported and a
  refresh replaces it.
- **Search** (`rpkg::search`): every word must match; order is a name equal to the whole
  query, then names (a prefix counts for less when the query has several words),
  title words, a phrase in the title, mentions in the description, and how many CRAN
  packages need the package (`Reverse depends`, `imports`, `linking to` from the
  database). Tuned on the real database: `ggplot2`, `data.table`, `survival analysis`
  and `panel data econometrics` put the obvious package first, `fixed effects` puts
  `fixest` first. Known weak spot: `mixed models` puts `glmmTMB` first and `lme4` is
  not in the top 8, because the others have the phrase in the title. A search takes
  about 60 ms in a debug build, 25,581 packages. Filters: licence, needs a compiler.
  The cache format is now version 2 (it holds who needs each package).
- **Package page** (`rpkg::describe`): version, date, licence, maintainer, authors,
  needs (Depends, Imports, LinkingTo, Suggests) with where each comes from (R, CRAN,
  not on CRAN), who needs it, system requirements, links. Links are only `http` and
  `https`, and open in the browser only on a click.
- **Install plan** (`rpkg::plan`): the packages asked for, what they need that is
  missing or too old (R's version rules), and problems named before anything runs: a
  name CRAN lacks (with a "did you mean" for a case difference), a needed package that
  was removed from CRAN, a version CRAN does not offer. A list over 2000 is cut short
  and refused. R still does the install and its own resolving.
- **Confirmation** before every install, update and remove: what will be installed with
  versions and why, the mirror host, a warning that packages run their authors' code,
  a note on packages with compiled code, and the exact R call; "Yes, install" is at the
  top. The command runs as an ordinary command, so it is in Results and the log.
- **Code** (`rcodegen::packages`): install (binary where the platform has one, or
  `type = "source"`), install an older version from CRAN's archive (no screen yet),
  remove, list installed, loaded packages, where a function comes from. Names and
  versions are checked before they go into code.
- **Installed tab**: from R's `installed.packages()`; version, loaded or attached,
  update available (only for packages in RCraft's own library: it never updates or
  removes packages in another app's library), Update all, Remove, and "where does this
  function come from".
- The old one-click panel in dialogs (a dialog needs `haven`) now installs from the
  saved mirror through the same generator.

Found on the way: on Windows, `install.packages` with a repository that lacks a binary
folder (`bin/windows/contrib/4.4`) stops with "cannot open the connection", so a local
test repository must be installed `type = "source"`. CRAN has binaries, so this does
not happen there.

Tests: `rpkg` 63 + 11 integration (real file ones opt in), `rcodegen` code generators,
`engine` service (8) and `tests/packages.rs` (3, real R against a repository built on
the spot: planned, installed with its import, listed, removed, where-from and loaded),
`ui-egui` window logic (5) and app (2). The window was looked at in pictures over the
real list: Find with a selection, the confirmation, and the Installed tab
(`rcraft-desktop --picture out.png --packages find --packages-query "fixed effects"
--packages-select fixest --packages-confirm --packages-from packages.rds`).

**Not done:** task views and the browse tree (needs `Views.rds`, 70 KB, not downloaded
yet: ask first); download counts; providers other than CRAN (Bioconductor, r-universe,
GitHub, local); readable install failures (P1A.7: missing compiler or system library, with
the fix named, and retry from an older version in a screen); the lockfile (P1A.8); command
forms `rc install`, `rc search` and Tab completion (P1A.9); "has a binary for my platform"
filter; installing or updating on a click in the real window has not been done by hand
(the flow is tested in real R and the screens are checked in pictures). The Packages
window is not a catalog entry like the dialogs are.

## P2C.3 front end (vendored from StatCraft), 2026-10-28

New crate **`rcraft-statatr`** (layer L3, already in the `AGENTS.md` table). It has
no dependencies and never starts R. Its first content is StatCraft's do-file
language front end, **copied** from StatCraft commit `a245039` (branch
`lang/shared-parsers`) because StatCraft is a private repository and RCraft a public
one (a git dependency would fail for everyone else and in CI). The user chose
vendoring over publishing `statcraft-lang` separately.

- `lang/script.rs`: do-files into logical command lines (`///` continuations, `/* */`,
  `//`, `*` comments, `#delimit`, doubled backslashes); each line keeps the physical
  lines it came from.
- `lang/syntax.rs`: a command line into main part, `if`, `in`, weight and options;
  `options`, `abbrev`, `wildcard`.
- `lang/expr.rs`: Stata expressions into a tree (`Expr`), with depth capped at 64;
  date literals become `date()` calls; `.`, `.a` to `.z` are missing values.
- `values/missing.rs`, `values/hex.rs`, `values/datetime/{mod,parse,leap}.rs`: the
  part of StatCraft's core these need. The date-format part (`display`) is left out.
- Only edits: paths (`statcraft_core::` to `crate::values::`) and the left-out part.
  Their 16 unit tests came with them and pass; `tests/front_end.rs` (3) takes a small
  do-file through all three steps and checks that bad input is an error.
- **Keep the two copies in step:** a parser bug fixed in one place is fixed in the other.
  The file `lang/script.rs` mentions `plan/stata/do-files.md`; that note is in StatCraft's
  repository, not copied here (it is observations of Stata; nothing recorded from a licensed
  Stata is committed here, AGENTS.md section 1).
- Not started: P2B.1's own reader built on these (macros, `foreach`, `by:` prefixes),
  P2B.2 expression translation to R, and everything after.

## P1A.7 Install failures made readable, 2026-10-28

When an install, update or older-version install run from the Packages window ends,
the window reads what R printed for it (output, warnings, errors) and, if it
recognises the failure, shows a banner above the lists: what went wrong in a sentence,
what to do about it on this system, and buttons that build a new list to say yes to
(nothing runs until the user does). The banner goes away when the packages asked for
are in the installed list at the version CRAN offered, so a warning on a run that
worked does not leave a banner.

`rcodegen::failure::diagnose(text, os, r_version)` finds:

- **No compiler or `make`**: Windows names the Rtools for this R (R 4.4 is Rtools44)
  and its address, and offers *Binary only*; macOS says `xcode-select --install` (and
  gfortran from the R project's tools page); Linux gives the apt and dnf commands.
- **A missing system library**: a table of 25 common libraries (libcurl, libxml2,
  OpenSSL, libpng, freetype, HarfBuzz, GDAL, PROJ, GEOS and others) gives the apt,
  dnf and Homebrew package; an unknown library is still named from the configure
  message; on Windows the advice is *Binary only*.
- **The package needs a newer R**: says which R is needed, and offers to install an
  older version (a version box, then the usual list to say yes to; built from source,
  with a caution).
- **A needed package is not offered**, **only a source package exists**, **a name CRAN
  does not have**, **the library cannot be written to**, **the mirror cannot be
  reached** (an unreachable mirror is blamed before the "not available" lines that follow
  it, and offers *Change mirror*), and a plain "R stopped installing X" when nothing
  clearer is found.
- Retries: *Try again*, *Binary only* (`type = "binary"`), *Build from source*
  (`type = "source"`), an older version, *Change mirror*. `InstallFrom` in
  `rcodegen::packages` replaces the old yes/no source flag.

Recorded from real R 4.4.3 on Windows (this machine has no Rtools, so the compiler case
is real): `'make' not found`, `unable to access index for repository`, `is not available
for this version of R` with `depends on R (>= 99.0)`, `dependency 'x' is not available for
package 'y'`, `'lib = ...' is not writable`. `engine/tests/package_failures.rs` makes
each of these happen in a real R against a repository built on the spot and checks the
diagnosis; the compiler case runs only where R has no `make`. It also caught a bug: the
dependency name was read from the wrong line when R printed it twice.

**Not verified against real output:** the macOS and Linux messages and the "available
as a source package but not as a binary" message follow what R, make, gcc and configure
scripts print there, written from knowledge and not recorded on those systems. They are
unit-tested with strings in that form only. Run an install that fails on a Mac and a
Linux machine and compare before relying on them.

Seen in a picture: `rcraft-desktop --picture out.png --packages find --packages-select
fixest --packages-failure compiler` (also `newer-r` and `network`).

Not done: a failure of an update through *Update all* offers *Try again* only for the
names of the plan; there is no per-package retry. Removal failures are not diagnosed.

## P1A.8 Package lists (reproducibility), 2026-10-28

**Packages > Project package list**, a third tab in the Packages window. A package list is
a small text file (`# RCraft package list (version 1)`, then `r`, `mirror`, `saved` and one
`package<TAB>name<TAB>version<TAB>source` line per package; source is `cran`, `base` or
`other`). It is meant to be kept with a project.

- **Save list…** records the packages loaded in this session (or, ticked, every package in
  RCraft's own library) with their versions, the R version, the mirror and the day. Where each
  came from is taken from what R records (`Repository: CRAN`, `Priority`): GitHub or local
  installs are recorded as `other`.
- **Open list…** reads a file strictly (header, tab-separated fields, valid names, versions
  and mirror, no repeats, at most 2 MB and 5000 packages; the first fault is reported with its
  line) and shows each package: listed version, installed version, and what would happen:
  already installed; install (CRAN's current version is the listed one); install this older
  version (from CRAN's archive); or left alone, with the reason (comes with R; was not from
  CRAN; no longer on CRAN; CRAN's version is older than the listed one; download the package
  list first). A note appears when the list was made with a different R (major.minor).
- **Install what is missing…** shows the usual list to say yes to with the exact R calls: the
  older versions one by one first, then the rest in one call. Failures go through the same
  banner as other installs (P1A.7).
- Nothing in the app writes a list unasked, and nothing installs without the yes.

Code: `rpkg::lockfile` (format, parse, `restore_plan`), `rcodegen::packages::restore_code`, and the
installed-package list now carries the `Repository` column. `engine::package_index::today()` gives
the date written into a list.

`engine/tests/packages.rs` runs it in real R against a repository built on the spot with an
archive folder: a list naming `pfold` 1.0 (archived, CRAN now has 2.0) and `pfcur` 1.0 is read back
from its file text, planned (one older, one current), installed with the generated calls, and
planned again (nothing left). Seen in a picture: `--packages list --packages-lock FILE`.

Found: R installs an archive tarball from a web address (https) but not from a `file://` one, so the
test names the local copy by path; the real https route to CRAN's archive has not been run (it is R's
documented way, `install.packages(url, repos = NULL, type = "source")`).

Limits, said in the window: an older version is built from source (compiler needed if it has
compiled code), and what it needs that is missing comes at its newest version, so a list does not
reproduce the whole dependency tree's versions unless every package is in the list (saving the
loaded packages includes the dependencies that are loaded). There is no `renv`-style isolation. Not
done: a list kept automatically with the session, the list in the saved R script's header, and
`rc` command forms (P1A.9).

## P1A.9 `rc` commands in the Command window, 2026-10-28

Typed forms of what the Packages window does. A line is an `rc` command only when `rc` is
followed by one of the command words, so `rc <- 3`, `rc(1)` and `rc$x` still go to R
(`rcodegen::rc::parse` returns nothing for them and the line is sent to R as before).

- `rc install PKG [PKG ...] [, version(1.2.3) from(source|binary) provider(cran)]`,
  `rc remove PKG ...`, `rc update [PKG ...]`: open the Packages window with the same list to say
  yes to (versions, why, mirror host, the exact R call). Nothing is installed, updated or removed
  until the yes. `version()` takes one package and uses CRAN's archive. `provider()` accepts only
  `cran` for now and says so. `rc update` with no names updates what has an update in RCraft's own
  library; a package in another library is named and left alone.
- `rc search WORDS`, `rc describe PKG`, `rc help`: answered at once in Results, from the package
  list on this computer (no R, no network). `rc describe` of a name that differs only in case says
  "Did you mean ...". With no list downloaded they say where to get it.
- `rc installed [PART OF A NAME]`: sent to R as ordinary `installed.packages()` code, with the typed
  line as a comment above it, so Results, the log and the script show what ran.
- A wrong line (`rc install` with no name, a bad version, an unknown option) says what is wrong, in
  Results, marked as an error.
- Tab on an `rc` line completes the command word, a CRAN package name (from the list, ignoring
  nothing: case matters), an installed package name after `rc remove` and `rc update`, and the
  options after a comma. One choice fills in; several fill in their common start and are listed
  above the line. Tab on other lines is untouched.
- `Engine::echo` records a command RCraft answered itself: History, Results and the log as usual,
  and in the script as a comment, so a saved script stays valid R.

Tests: `rcodegen::rc` (parser, R-code lookalikes, completion), four app tests against a real R
(search and describe in Results and as a script comment, wrong lines and `rc <- 41 + 1` going to
R, `rc install` opening a confirmation and recording only the typed line, Tab on rc lines).

Not done: the completion of installed names works only after the Packages window has asked R once;
`rc` lines in the Script pane's Run are not intercepted (only the Command window and the demo
runner); no `rc which` (which package a function comes from) form; a mistyped command word
(`rc instal`) goes to R and fails there.

## CI findings on the first pull request, 2026-10-28

The first GitHub run of everything on `p3/join-keys-tidyverse` failed on all three systems.

- Two were tests of mine: a Linux path in the saved-list test, and the unwritable-library test
  that used `C:/Windows/System32` (a runner is an administrator, so it was writable). Fixed; Ubuntu
  and Windows then passed.
- macOS found a real bug in P5.4 (Export plot): **SVG export reported success and wrote an empty
  file** on an R that cannot load cairo. The CRAN macOS build of R needs XQuartz's libraries
  (`/opt/X11/lib/libXrender.1.dylib`) for `svg()`; where they are missing R only warns
  ("failed to load cairo DLL") and the file is empty. Now the export code checks
  `capabilities("cairo")` first for SVG and stops with "SVG needs R's cairo graphics, which this R
  cannot load. On a Mac this usually means XQuartz is not installed (xquartz.org). PNG and PDF still
  work", and every export checks that the file was written and is not empty. PNG and PDF are not
  affected. The GitHub macOS runner has no XQuartz, so its tests now expect that message for SVG and
  compare PNGs for the choice of plot.
- Lesson kept: a graphics device that cannot open does not always raise an error; check the file.

## P1A.3 Task views (browse), 2026-10-28

CRAN's task views are part of the package list now. `src/contrib/Views.rds` (about 70 KB, 50 views,
checked once with the user's yes on 2026-10-28) is fetched as the third file of a refresh. A failure to
get or read it does not fail the refresh: the packages are stored, the saved views are kept, and a note
says the views could not be loaded. Cancelling during it cancels the whole refresh and changes nothing.
Kept in `task-views.txt` next to the index cache (same atomic write).

- `rpkg::views`: the reader (name, topic, maintainer, date, address, package table with the core flag),
  sorted views, core packages first, `members_of`, `views_of`, and the cache text. Real file: 50 views,
  Econometrics 150 packages of which 12 core (`the_real_task_views_*` in `tests/r_files.rs`, opt in with
  `RCRAFT_REAL_VIEWS_RDS`).
- **Packages > Task views** tab: the views by field, each with its packages (core ones marked), a title
  line from the package list, "not on CRAN" for listed names it lacks, a link to the view's own page (opened
  only on a click) and **Search inside this view**. Clicking a package goes to its page.
- **Find on CRAN** has a task view filter: with words they search only inside the view; with none it lists
  the view's packages core first. A package's page says which task views list it.
- Pictures: `--packages browse --packages-views Views.rds --packages-view Econometrics`.

Not done in P1A.3: download counts (needs another host, the public download-logs service, one request per
look, so it wants its own decision), and a "new packages" list, which CRAN's files do not support (only
"recently published" exists, in Find with no words).

## P1A providers: Bioconductor, r-universe, GitHub, local files, 2026-10-28

Built to `plan/providers.md` (written first, on Opus). **Packages > Other sources** tab, and
`rc install` forms. Each source has a trust label, shown with every list to say yes to; the three
nobody reviews (r-universe, GitHub, a local file) need a tick, "I understand this code has not been
reviewed", before **Yes, install** turns on.

- **Bioconductor**: *Look up* asks `bioconductor.org/config.yaml` which release is for this R
  (the newest one made for this major.minor that is not the development release), then that
  release's `bioc/VIEWS` (names, versions, titles, needs), searchable here. Install is
  `BiocManager::install(..., update = FALSE, ask = FALSE)`, installing BiocManager from CRAN first
  when it is missing, with CRAN set to the chosen mirror for the install only.
- **r-universe**: a universe name (checked as a GitHub owner name), then its
  `src/contrib/PACKAGES.gz`, searchable. Install is `install.packages(pkg, repos = c(universe, CRAN))`.
- **GitHub**: `owner/repo[@ref]` (or a github.com address), checked; *Look up* reads its
  `DESCRIPTION` from `raw.githubusercontent.com`. The list shows the CRAN packages it needs that
  are missing or too old (planned with the CRAN list; one not on CRAN stops the install with a
  reason) and the label says when the reference is `HEAD`, which can change. Install is base R:
  the CRAN needs, then `download.file()` of `github.com/<owner>/<repo>/archive/<ref>.tar.gz` and
  `install.packages(f, repos = NULL, type = "source")`.
- **A local file**: a `.tar.gz`, `.zip`, `.tgz` or a folder (File… and Folder… choosers). *Read it*
  has R print the DESCRIPTION inside it (untar/unzip into a temporary folder, nothing installed);
  the list then works as for GitHub, with the right `type` for the file.
- `rc install PKG, provider(bioc)`, `rc install PKG, provider(universe:OWNER)`,
  `rc install OWNER/REPO[@REF], provider(github)` and `rc install PATH` (a path is recognised by
  its ending or its start; `provider(local)` for a folder) open the tab, look the source up (the
  typed command is the request) and show the list once R has said what is installed.
- Addresses are in `docs/network.md` (its own commit); each *Look up* button says what it will ask.

Tests: `rpkg::provider` (names, references, labels, the release choice), `rcodegen::sources` (the code
for each source), `engine::sources` (the lookups against a stand-in for the network),
`engine/tests/sources.rs` in real R (a `.tar.gz` and a folder read and installed; a `.zip` where R can
build one, which needs Rtools; a GitHub-style archive with its `repo-ref/` top folder fetched from a
file address and installed), the window's lists and the `rc` path. Seen in a picture:
`--packages find --packages-source-demo`.

Found while building: a test that built a Windows binary with `R CMD INSTALL --build --library=.`
installed over the source folder of the same name; the binary is now built through a library of its
own. R installs a source archive whose top folder is named `repo-ref` (as GitHub's are) under the
package's own name.

**Checked against the real services (2026-10-28, with the user's go-ahead; nothing installed)**, with
`engine/tests/live_sources.rs` (opt in with `RCRAFT_LIVE_SOURCES=1`):

- Bioconductor: `config.yaml` read as written; for R 4.4.3 the release chosen is **3.20** (3.19 and 3.20
  are both for R 4.4; the newer is taken). Its `bioc/VIEWS` read as DCF: **2,289 packages, all with
  titles**.
- r-universe: `ropensci` gave **366 packages**, but its `PACKAGES.gz` has **no titles**, so a search of a
  universe matches names only. r-universe's JSON listing has titles; reading it is not done.
- GitHub: `r-lib/cli` at `HEAD` gave `cli 3.6.6.9000`, "Helpers for Developing Command Line
  Interfaces", Imports `utils`. A source DESCRIPTION has no `NeedsCompilation` field, so a GitHub
  package is always said to be built here. The archive address (asked with a header request only)
  redirects to `codeload.github.com/r-lib/cli/tar.gz/<commit>` and serves gzip; that commit could be
  shown and recorded to make `HEAD` repeatable, which is not done.
- Not run for real: BiocManager's install, an r-universe install and a GitHub install (they install,
  which was not asked for).

Limits, also in `plan/providers.md`: GitHub packages in a subfolder; annotation and experiment-data
packages of Bioconductor are not in the search (BiocManager still installs them by name); lookups are
kept in memory, not cached; package lists record these packages as `other`.
- Second macOS run (2026-10-28): `capabilities("cairo")` says TRUE on the runner even though
  `svg()` cannot load cairo there, so that check never fired; the file-size check caught it with
  the generic message. The export now notes the current device, stops with the XQuartz message
  when the SVG device did not open, and closes only the device it opened. Before, a device that
  failed to open meant `dev.off()` closed the Graph tab's own capture device. A test checks that a
  failed export leaves the Graph tab drawing, and the tests probe SVG by drawing one, not by asking
  `capabilities()`.

## P2B.1 Stata syntax reader (first part), 2026-10-28

`statatr::reader::read(text) -> Program`: a do-file read into a tree, on top of the vendored
StatCraft front end (its do-file reader, command syntax and the command-name table, also vendored now:
`lang/commands.rs` from `crates/engine/src/commands/mod.rs` at `a245039`).

- **Commands**: the word as written and its full name (`reg` is `regress`; a word the table lacks is
  kept and marked unknown), the main part, `if`, `in`, weight, options, `using` taken out of the main
  part, the line as read, and its line, column and number of physical lines.
- **Prefixes**, in order: `by varlist[, sort]:`, `bysort`, `capture`, `quietly`, `noisily` (with or
  without a colon) and `frame name:`.
- **Blocks**: `foreach x in|of varlist|newlist|numlist|local|global ... {`, `forvalues i = range {`,
  `while cond {`, `if cond {` with `} else {`, `} else if ... {` and `else` on its own line after the `}`
  (as Stata's manual writes it) or `else command`, `capture {` / `quietly {`, `frame name {`,
  `program [define] name` ... `end` (the body read as commands), and `mata`/`mata:`, `python` and
  `input varlist` ... `end`, whose lines are kept as written (they are not Stata commands). The
  one-line programming `if exp command` stays a command.
- **Macros**: the locals (`` `name' ``, nested ones by their simple parts) and globals (`$name`,
  `${name}`) each line refers to; nothing is expanded, since values exist only when the file runs.
- **Problems** never stop the reading: a `}` with nothing to close, a block left open (the line it
  opened on), a loop line without `{`, `foreach` without `in`/`of`, `forvalues` without `=`, `while`
  without a condition, `else` with no `if`, `by` without a colon, a stray `end`, blocks nested deeper
  than 64, each with its line and column. Hostile input (deep nesting, unbalanced quotes, control
  characters) is tested not to panic.

Tried on StatCraft's 230 probe do-files (ours, on made-up data; `tests/many_dofiles.rs`, opt in with
`RCRAFT_DOFILES_DIR`): 17,527 commands, no problems, after adding the program, mata, input and frame
blocks the first run showed were missing (46 problems before).

Not done in P2B.1: expanding macros, `#delimit` inside blocks is only what the vendored reader does,
`include` and `do` of other files (P2C.2), abbreviated variable names (they need the data: the R runtime
package, P2C.4), and the translation itself (P2B.2 onwards). The command table holds the 53 commands
StatCraft's engine needs plus prefixes and loops; commands outside it are read with their names as
written, which is right for any command Stata does not let you shorten.

## P2B.2 Stata expressions in R, 2026-10-28

`statatr::expr_r::translate(text, kind_of, use) -> Result<RExpr, Untranslated>`: an expression parsed by
the vendored StatCraft parser and written as R for a dplyr data-masking context (inside `mutate()` or
`filter()`, after `group_by()` for `by:`). `kind_of` gives a variable's type when the data is known;
`use` says whether a value (true and false become 1 and 0) or a condition (TRUE/FALSE, never NA) is
wanted. `RExpr` carries the code, its kind and caveats; `Untranslated` says why, and nothing is guessed.

- **Missing is larger than every number**: `x > 5` is `(is.na(x) | x > 5)`, `x < 5` is
  `(!is.na(x) & x < 5)`, `x < .` is `!is.na(x)`, and two variables compare with
  `dplyr::coalesce(., Inf)` on both sides.
- **Truth**: anything not zero is true, missing included (`(is.na(x) | x != 0)`); `&`, `|` and `!`
  work on that.
- **Arithmetic**: `x / y` divides by `dplyr::na_if(y, 0)` (Stata gives missing, R `Inf`) unless the
  divisor is a number written out; `+` on strings is `paste0`.
- **Rows**: `_n`, `_N`, `x[_n-k]` and `x[_n+k]` (`dplyr::lag`/`lead`), `x[1]` and `x[_N]`
  (`first`/`last`), all per group after `group_by()`.
- **Functions**, with StatCraft's checked rules: `round` (halfway up with Stata's allowance, so 2.345
  is 2.35), `mod` (missing for a divisor of 0 or less), `max`/`min` (skip missing), `missing`,
  `inrange`, `inlist`, `cond` (3 and 4 arguments), `sum` (running), math (`ln`, `ceil`, `int`, ...),
  strings (`strlen`, `upper`, `lower`, `trim`, `substr`, `strpos`, `subinstr`, `regexm`, `real`,
  `string`) and dates as days since 1 January 1960 (`mdy`, `year`, `month`, `day`, `dow`, `doy`;
  `td(...)` and `date()` of text written out are worked out here with the vendored date parser).
- **Caveats**: `.a` to `.z` all become `NA`; regular-expression dialects differ; `string()` formats
  numbers as R does; `round(x, y)` with `y` not written out (y = 0).

Checked in real R (`crates/engine/tests/stata_expressions.rs`, R 4.4.3 with dplyr 1.1.4): 42
expressions run through `dplyr::mutate()` on data with missing values, with and without `group_by()`,
each giving the values Stata's rules give. The first run found one bug: `strpos` kept `regexpr`'s
match attributes; fixed.

Not done in P2B.2: `_b[]`, `_se[]`, `e()` and `r()` (P2B.6), `substr` counting from the end or with
positions known only when the file runs, `date()` of a variable, `subinstr` with a count other than 1
or all, and functions outside the list above (each says so by name). Keeping the extended missings as
an attribute, as the roadmap suggested, is left until a command needs them.

## P2B.3 Stata's data commands in R, 2026-10-29

`statatr::data_r::translate(program) -> Translation`: a do-file read by P2B.1, translated command by
command into R that works on one data frame the way Stata works on the data in memory (`use
study.dta` makes `study`; the commands after it change `study`). Each line is R plus caveats and the
packages it needs, or the reason it is not translated; `Translation::script()` writes the whole R
script with each Stata line as a comment above its R, and the small helpers the lines call (float
storage, Stata's statistics rules, destring, tostring, Excel column names) defined once at the top.

Translated: `use` (varlist, if/in, nolabel), `save` (replace guard, `.dta` added), `import delimited`
(delimiters, varnames, case, encoding; empty cells missing, "NA" kept as text, names lowercased) and
`import excel` (sheet, cellrange, firstrow, allstring, case; columns named A, B, ... without firstrow),
`generate` (float storage by default, so `gen f = 0.1` holds 0.100000001490116; byte/int/long
truncate; `str#` keeps # characters; rows outside if/in get `.` or `""`; an existing name stops),
`replace` (rows outside if/in keep their value; a missing name stops), `drop`/`keep` (variables, with
`keep` keeping the data's order, or rows, where a missing condition counts as true), `rename`
(single, group, lower/upper), `recode` (first matching rule wins; ranges never match missing; min/max
over all rows; generate/prefix/copyrest), `label` (variable, define with add/modify/replace, values,
drop, list; labels kept as haven labelled columns), `sort` (stable), `gsort` (missing last even in
descending order), `egen` (mean, sd, min, max, total, count, median, pctile, iqr, std, rank with
field/track/unique, group and tag with missing, rowtotal, rowmean, rowmax, rowmin, rownonmiss,
rowmiss; floats by default; by() and the by prefix), `merge` 1:1, m:1 and 1:m (uniqueness checked
like Stata, master values kept in matched rows, `_merge` labelled 1/2/3, rows ordered master by key
then using-only by key, keep(), generate(), nogenerate, keepusing()), `append` (generate() labelled
Master / Appended dataset k), `reshape` long and wide (tidyr; rows by i then j, wide variables j by j,
Stata's errors for repeated or non-constant i), `collapse` (mean, median, p1-p99, sd, semean, sum,
rawsum, count, percent, max, min, iqr, first, last, firstnm, lastnm; by() sorted, missing groups
last; `(stat) x` labels; cw), `duplicates` (drop keeping the first row, tag, report), `encode` (byte
order codes, `""` missing), `destring` (Stata's idea of a number; without force a variable with text
is left alone with Stata's message), `tostring` (whole numbers), `order` (first, last, before, after,
alphabetic), `describe`, `list`, `count`, `set seed`, `clear`, and the prefixes `by`, `bysort`,
`capture` (as `try`) and `quietly`.

Checked in real R (`crates/engine/tests/stata_data_commands.rs`): 20 do-files over made-up data run
with every line translated. With `RCRAFT_STATCRAFT` set to a StatCraft executable (whose data
commands match Stata line for line), each do-file also runs there and the two saved datasets must have
the same variables, rows and values in the same order: all 20 match (StatCraft build of Oct 26). The
first run found one real difference, now fixed: in `merge`, a variable in both files keeps the using
file's value in rows only the using file has. It also showed that this StatCraft build needs `using`
in `import excel` (Stata does not); the test writes `using`. The two `reshape` cases (long, and long
then back to wide) were compared once tidyr 1.3.2 was installed on the work computer (2026-10-29).

Not done in P2B.3, each saying so when met: `merge m:m` (Stata pairs the k-th rows of a key), merge `update`/`replace`/`assert()`, `replace` that reads its own earlier rows
(`replace x = x[_n-1] if missing(x)` runs row by row in Stata), wildcard `rename`, recode rules with
value labels, egen `mode`, `concat`, `seq`, `cut` and the rest, `tostring` of numbers that are not
whole (Stata's `%12.0g`), weights in collapse, `duplicates list`, frames, and blocks (`foreach`,
`if`, programs: Phase 2C). `describe`, `list` and `count` print in R's layout. Abbreviated variable
names are taken as written (they need the data). A value label that comes from the data file rather
than a `label define` in the do-file is not available by name in R.

## P2.2 Tab completion, 2026-10-29

Tab in the Command window now completes any R line, not only `rc` ones. R does the
completing: `rcodegen::tab::completion_code` sends R's own console completion (`utils`) the
line as a hidden query (nothing reaches Results, the log or the history), and
`parse_answer` reads back the word being typed and its choices. So Tab offers what R's own
console would: objects and functions (a function ends in `(`), columns after `$`,
`package::name`, package names inside `library(`, and file names inside quotes, relative to
the session's working directory.

- The word is completed as far as the choices agree; several choices are shown above the line
  (at most 60, sorted). No space is added after a single choice.
- It completes the end of the line (the text box's cursor is not used), as `rc` lines do.
- An answer is used only if the line has not changed since Tab; nothing is asked while R is
  busy or the line is empty. A line R cannot complete changes nothing.
- `rc` lines still complete their own way (commands, CRAN and installed package names).
- Tests: `rcodegen::tab` (6 unit tests) and `engine/tests/tab.rs` in real R (an object, a
  column, a function, `stats::rnorm`, `library(tool`, two files that share a start, a line
  with nothing to offer, and odd lines such as an open quote or a lone backslash that must not
  upset the session).
- Not done: completing at the cursor rather than the end, argument names inside a call
  (`lm(alp` gives none), and column names inside `[["` or `df[, "`. Not yet seen in the window.

## P2A.1 to P2A.3, P2A.7, P2A.8 Help window, 2026-10-29

R's own help for the installed packages, in a **Help window** (`ui-egui::help_window`), with the
code and readers in `rcodegen::help`. Nothing is copied into RCraft: a hidden query asks the
session for the page, so it matches the packages installed here and works with no network.

- **Ways in:** the typed forms `?mean`, `?pkg::name`, `??words`, `help(mean)`,
  `help("lm", package = "stats")`, `help mean`, `help pkg::name` and `help search words` (any
  other line, such as `help <- 3`, is ordinary R); **F1** in the Command window on the word at
  the end of the line (the search box when there is none); **Help > Search R help**. The typed
  line is recorded in the history and says it opened the window.
- **A page** (`utils::help`, `tools::Rd2txt`, `tools::Rd2ex`): topic, package, title, R's text
  (description, usage, arguments, value, details, references, see also, examples), a **See also**
  row of links, **Also in** buttons when several packages have the topic, **Run examples** (sends
  the examples to the session as a typed command; the output is in Results) and **Copy examples**.
  Back and Forward walk what was shown.
- **Search** (`help.search`): each word is looked up in names, titles and keywords and a page must
  match all the words ("linear regression" finds `lm`); a topic equal to the words comes first, then
  names that start with or contain them, then base/stats/utils/graphics/methods/datasets pages, then
  the rest. At most 100 results. Words that are not valid patterns (`c++`, `(`) give an empty list,
  not an error.
- **A topic R lacks** (P2A.7, first part): the page says so, lists names that contain it, and has
  **Find a package for this…**, which opens the Packages window on a CRAN search for it.
- **Tests:** `rcodegen::help` (7 unit tests: the typed forms and what must stay ordinary R, the word
  for F1, quoting, reading a page, a missing topic and broken answers, ranking), `ui-egui::help_window`
  (3), and `engine/tests/help.rs` in real R: the `mean` page (text, examples that run, links), a
  page in one package, a symbol topic (`[`), a topic in the wrong package, odd topics, search and its
  ranking. Seen in pictures over real R: the `lm` page, a search, and a topic that does not exist.
- Found by the tests: an unknown package name makes R's `help()` an error, now a "not found".
- **Not done:** vignettes and package manuals (P2A.4), the Help button on every dialog (P2A.5), the
  RCraft manual (P2A.6), a page for a package that is not installed beyond the Packages search
  (P2A.7), clickable links inside the text of a page (only "See also" links), and Rd
  formatting beyond R's plain text. A very long page is one selectable text block.

## P2A.5 and P2A.9 (dialog pages) Help on every dialog, 2026-10-29

Every dialog has a **Help** button (next to Run and Cancel; in Use my file next to Import; in the
Packages window beside its tabs) that opens the Help window on a page for that dialog. A page is
made from the dialog's own spec (`rcodegen::dialog_help::for_spec`), so it cannot disagree with its
dialog and a dialog added tomorrow has a page the day it exists. Nothing is asked of R to show it.

- **What it holds:** the dialog's name and menu path, what it does, each option with its words
  (a variable box says which kinds it takes and how many; a list says its choices), the R packages
  it needs in each style (and that nothing installs without a yes), **the R it writes**, and **the R
  functions it uses**, each a link to R's own help page for it (`stats::lm`, `merge`, `dplyr::inner_join`).
- **The example R** is the real code the spec writes, for invented variables (`x`, `y`, `group`, a
  data frame `other`): the first values for which the dialog accepts its choices (it tries each
  starting column and a few file names, so a dialog that refuses `x` for both boxes gets two
  different ones). The functions are read from that code, outside strings and comments, without R's
  grammar words and the functions the code defines itself.
- **Use my file and Packages** are not catalog dialogs, so their pages are written by hand
  (`use_my_file()`, `packages_window()`), with notes on how they work.
- **Tests:** unit tests for the function finder (strings, escapes, comments, `pkg::name`, defined
  functions), the page of a made-up dialog, and the two hand-made pages; `catalog` checks that **every
  one of the ~80 dialogs** (`catalog::all_specs()`, new) gets a page, an example and function links;
  `engine/tests/dialog_help.rs` runs R and requires **every linked function to have a help page in R**
  (functions of packages not installed on the machine are skipped and listed; CI has them); the window
  opens the page for four kinds of dialog and stays up through frames. Seen in pictures over real R:
  Linear model, Join, Use my file (`--dialog-help NAME`).
- Found while building: `help()` treats a package held in a variable as the variable's name, so the
  app passes the package as a literal (as it always did) and the test looks up by value.
- **Not done:** the RCraft manual (P2A.6), vignettes and manuals (P2A.4), F1 inside a dialog, and
  hand-written longer explanations per dialog (the pages are generated, so they are as good as each
  dialog's own `about` and hints).

## P2A.4 Vignettes, package pages and NEWS, 2026-10-29

A package's own documents, from the installed package (`rcodegen::help_docs`, shown by the Help
window), so they match what is installed here and work with no network.

- **Ways in:** `help package NAME` or `help(package = "NAME")` (also `help manual NAME`), `help
  vignettes [NAME]` or `browseVignettes()` / `browseVignettes("NAME")`, `help news NAME`; **Help >
  Vignettes and package manuals**; a **Package page** button on every help page; the package names in
  the vignette list. Other lines (`vignette(...)`, `news()`) stay ordinary R.
- **A package page:** the DESCRIPTION fields (folded away until asked), **Vignettes (n)** and **News**
  buttons, the vignettes, and the **index of help topics** (name and title, a click opens that page
  in that package), with a box to narrow it (300 shown at a time).
- **Vignettes:** one row per vignette with a button for each file R has for it. A rendered PDF or
  HTML is opened by the system, on the click only (`file:///` address, spaces and non-ASCII escaped);
  the **R code** and the **source** (`.Rmd`, `.Rnw`, `.qmd`) are shown in the window, the code with
  **Run code**, which sends it to the session like a typed command. The list of all installed packages'
  vignettes is grouped by package.
- **NEWS:** `NEWS.md`, `NEWS` or `NEWS.Rd` (rendered to text by R), whichever the package has, cut at
  4000 lines.
- **Safe by construction:** a package name that is not a package name never reaches R (the code
  builders answer "not installed"); a file's text is read only when its path is inside an installed
  package's folder (checked in R, and the path comes from R's own vignette list, never typed).
- **Tests:** `rcodegen::help_docs` (file kinds and addresses, the code builders, reading a package page,
  missing and broken answers, titles, texts), the new typed forms in `rcodegen::help`, the window
  (every kind of answer becomes the right content), and `engine/tests/help_docs.rs` in real R: `grid`'s
  page (description, topics including `viewport`, 13 vignettes with PDF, code and source), a package that
  is not installed and odd names, the vignette list of one package and of all, a NEWS file (MASS), and a
  file outside the packages refused. Seen in pictures over real R: `grid`'s page, the list of all
  installed packages' vignettes, and MASS's NEWS.
- Found by the tests: an empty package name made R fail ("zero-length variable name"); the code
  builders now check names first.
- **Not done:** PDF reference manuals. R packages do not install them and R's own builder needs LaTeX, so RCraft will make its own, catered to RCraft (ROADMAP P2A.10, asked for 2026-10-29); links
  inside the text of a NEWS file or vignette, rendering HTML vignettes inside RCraft (the system opens
  them), and opening a vignette's PDF has not been tried by hand in the live window.

## P2A.4 reference manuals: CRAN's editions, 2026-10-29

A package page has **Reference manual (PDF)**. It opens CRAN's own PDF for the package in the user's browser,
on a click only: `<mirror>/web/packages/<package>/<package>.pdf` on the mirror chosen in the Packages window.
A package that comes with R (`Priority: base`: stats, utils, grid and so on) has no PDF of its own on CRAN, so the
button opens R's full reference manual, `<mirror>/doc/manuals/r-release/fullrefman.pdf`, which covers it. When the
package list is loaded and does not know the package (GitHub, a local file), the button is off and says why.
RCraft sends nothing; the addresses are in `docs/network.md` (both checked 2026-10-29: MASS's PDF and the full
manual exist; `grid`'s own address is a 404, which is why base packages are different).

RCraft's own manuals, catered to RCraft, are down the road (ROADMAP P2A.10; the user chose CRAN's editions for
now). Found in the walkthrough: some packages' help index has section headings (`Functions:`, `=====`), which
are no longer listed as topics; the Packages window's Help button and package count no longer overlap the tabs.
Not tried by hand: clicking the button in the live window (the address is built and tested; the browser opens it).

## Data leftovers: factor levels, kappa, Type II/III sums of squares, 2026-10-30

- **P3.4 level list.** Each factor in the Variables pane has a folded line under it ("3 levels", "3 levels,
  ordered"). Opened, it lists the levels in the factor's own order (the first is the reference level a model
  compares with) with how many rows each has; a click puts the level, quoted, in the Command line. At most 60 shown.
  Counting is tested (damaged codes are ignored, not a crash); seen in a picture folded, not opened.
- **P4.2 Agreement (kappa)**, `Statistics > Tests and intervals`: Cohen's kappa for two ratings of the same rows
  (factor, text, logical or number categories), in base R. Prints the agreement table with margins, observed and
  chance agreement, kappa, its standard error (Fleiss, Cohen and Everitt 1969), a confidence interval at 90, 95 or
  99%, and the z test of kappa = 0 (standard error under that hypothesis). Linear or quadratic weights for ordered
  categories give a weighted kappa (point estimate only, and it says so). Rows missing either rating are left out; a
  category neither rater used is dropped; all ratings in one category stops with "fewer than two different values".
  `engine/tests/kappa.rs` checks it against numbers worked by hand (2 x 2: kappa .4000, standard error .1270, z
  2.8868, p .00389, interval .1511 to .6489; 3 x 3: .6056, linear .6732, quadratic .7444).
- **P4.3 Sums of squares (Type II/III)**, in the Objects pane's Model menu on a linear model: the analysis-of-variance
  table with Type II or Type III sums of squares, in base R, so no package is needed (it works from the model's own
  model matrix; Type III recodes factors to sum to zero and says so; the saved model is unchanged). Weights, covariates
  and interactions work; a model that is not `lm`, or has a redundant (NA) coefficient, is refused in words.
  `engine/tests/sums_of_squares.rs` checks it against other computations: Type II against base R's sequential `anova()`
  with the term fitted last, Type III against a Wald test from `coef()` and `vcov()` under sum-to-zero coding, equal
  for additive models, equal to the sequential table for balanced groups, and unchanged by the order of terms. Not
  checked against the `car` package, which is not installed here; this machine's R is the only one used.
- Still to do from these three: custom contrasts (P4.3), a stronger agreement family (Fleiss' kappa for more than two
  raters, intraclass correlation), and the opened level list in a picture.

## Leftovers cleared: Fleiss' kappa, intraclass correlation, custom contrasts, car check, 2026-10-30

All in base R (no new package), each checked against published or independently computed figures.

- **Agreement of several raters (Fleiss' kappa)**, `Statistics > Tests and intervals`: one column per rater,
  rows missing any rating left out. Prints the categories with their share and their own kappa, observed and
  chance agreement, Fleiss' kappa and the z test of kappa = 0 (his 1971 standard error under that hypothesis).
  `engine/tests/reliability.rs` runs Fleiss's own example (10 subjects, 14 raters, 5 categories): kappa .2099
  (published .210), and recomputes kappa and z from the table of counts by a separate route.
- **Reliability (intraclass correlation)**: the six Shrout and Fleiss (1979) intraclass correlations from the
  two-way analysis of variance, each with its F test and an interval at 90, 95 or 99% (the F-based intervals
  of Shrout and Fleiss and McGraw and Wong, as the `psych` package gives), and the four mean squares. On the
  paper's 6 judges-by-4 targets table: mean squares 11.24, 32.49, 1.02, 6.26; ICC(1,1) .17, (2,1) .29, (3,1)
  .71, (1,k) .44, (2,k) .62, (3,k) .91, and every interval equal to psych's to two places (for example ICC(2,1)
  .019 to .76, ICC(2,k) .071 to .93). The table is one line per type with a legend below it, so it does not wrap.
- **Custom contrasts**, in the Objects pane's Model menu on a linear model: planned comparisons between the
  levels of one factor ("Treated = 1, Control = -1; Treated = 1, Low = -0.5, Control = -0.5"). The estimated mean
  of a level is the prediction averaged over every combination of the other factors (equal weights) with number
  variables at their means; a comparison's standard error comes from the model's covariance matrix and the
  residual degrees of freedom. Prints the estimated means (with standard errors) and the comparisons with
  estimate, standard error, t, df, p (unadjusted, Holm or Bonferroni) and an interval (not adjusted). Weights
  that do not add up to zero, a level the factor lacks, a name that is not a factor, a curved number variable
  (`poly`, `log`, `I(x^2)`) and a generalised model are each refused in words. `engine/tests/contrasts.rs`:
  one factor against the textbook `sqrt(MSE * sum(w^2 / n))`; additive models against `summary()`
  coefficients; an interaction model against `predict()` averaged over the other factor; adjusted p-values
  against `p.adjust`.
- **Type II/III against `car`**: CI now installs `car`, and `sums_of_squares.rs` compares both types with
  `car::Anova` (this machine has no `car`, so the test skips here and runs in CI).
- **Picture mode can click** (`--click X,Y`, repeatable): it presses the mouse at a point once everything has
  settled, so a picture can show a folded list opened or a menu used. First use: the opened factor level list
  (the position is now a dimmed number apart from the level so "1  4" no longer reads as fourteen).
- Still to do from this: a Bayesian or weighted version of agreement, contrasts of means across several
  factors at once (this takes one factor), and contrasts for generalised models.

## P1.6 working folder and sessions, P3.8 data-frame tabs, 2026-10-30

- **Working folder.** The status bar showed the program's own folder, not R's, so it was wrong as soon as a
  script called `setwd()`. The workspace listing (a hidden query after commands) now also reads `getwd()`
  (`Engine::working_dir()`), and the status bar shows that; clicking it, or **Session > Working folder…**, opens
  the system's folder chooser and runs `setwd("...")` as an ordinary menu command (in Results, the log and the
  script). The folder is remembered between runs (`working-folder.txt` in RCraft's folder, only if it is still a
  folder; pictures and tests never write it) and a Restart returns to the folder last chosen. Use my file's Browse and
  the save dialogs' Browse open in the working folder when their box is empty (before the recent files).
- **Several R sessions.** **Session > New R session** starts another R in the folder the shown one is in; the Session
  menu lists them (`R 2  (survey)`: the name and the folder) and a click switches; **Close this session…** asks first
  (its R stops and its objects are lost; the last session cannot be closed). Each session keeps its own R, objects,
  history, results, log, script text, Command line and recall, plots, tabs and the data frame shown. A command
  running in one goes on while another is shown (every session's R is served each frame). Dialogs open on one
  session are closed when you switch; the Packages and Help windows work on the shown session. The status bar says
  `Session R 2` when there is more than one. Implementation: the shown session's state lives in `App`, the others in
  `App::sessions`, swapped by `exchange`.
- **Data-frame tabs.** The Data tab has a strip of tabs for the data frames opened (clicked in Objects or the
  variable list, or imported), in the order opened; a click switches, **×** closes a tab (the data frame stays in
  R; closing the shown one goes to its left neighbour); a tab whose data frame has gone from R is dropped.
- **Tests.** Real R: `working_dir` follows `setwd` and a restart (`engine/tests/workspace.rs`); choosing a folder
  moves R and a restart keeps it; two sessions are separate and each keeps its own state while a slow command in a
  parked one finishes; a new session starts where the shown one is (even made while the last was still starting:
  each session remembers its start folder); closing an earlier session keeps the shown one; tabs and their
  neighbour rule; a tab dropped when its frame is removed. Also the file-chooser start folder order and the
  remembered-folder file (`rhost::folder`). Seen in pictures with the new `--click`: the Session menu with one and
  with two sessions, the tabs.
- **Not done:** per-session working-folder memory across runs (only the last folder chosen is remembered, and every
  new session starts in the shown one's folder); sessions are not saved or restored when RCraft closes (that is the
  project file, P7.5); a session name cannot be changed; closing the window does not ask about running sessions.

## Session rename, 2026-10-30

- **Rename.** **Session > Rename this session…** opens a small window with the name; Enter or Rename keeps it. A
  blank name keeps the old one, a name is cut to 30 characters and line breaks become spaces. The Session menu and
  the status bar show the new name. Test: `sessions_are_separate…` in `ui-egui/src/app.rs`.
- **The one "Not done" list** (this replaces the lists above): sessions are not saved or restored when RCraft closes
  (project file, P7.5); a new session starts in the shown one's folder, and only the last folder chosen is
  remembered; closing the window does not ask about running sessions; I have not driven the native window by mouse,
  only the picture mode (`--click`), because moving the real mouse takes over the user's desktop.

## P2B.4 Stata statistics commands, part one, 2026-10-30

`summarize`, `tabulate`, `correlate`, `ttest` and `regress` translated from a do-file to R
(`statatr/src/data_r/{stats,model}.rs`). The work is not RCraft's own statistics dialogs and must not
become them: those follow R's conventions, these follow Stata's, and on purpose they do not always give
the same number. Where R cannot do what Stata does, the line is refused with its reason, as everywhere
in this crate.

- **summarize.** Obs, mean, sample standard deviation, min and max; with `detail`, the variance, the
  population skewness and kurtosis (so a normal sample's kurtosis is near 3) and the nine percentiles by
  Stata's rule (`x[ceil(np)]`, or the mean of the neighbours when `np` is whole), which is R's
  `quantile(type = 2)`, not R's default. A string variable has no observations, as in Stata.
- **tabulate.** One-way (frequency, percent, cumulative) and two-way with margins; `chi2` (Pearson, no
  continuity correction), `row`, `column`, `cell`, `missing`, `nolabel`, `sort`. Value labels are shown
  in the order of the values underneath, which is Stata's order, without needing haven.
- **correlate.** Casewise, so a row with a missing value in any of the variables goes for all of them;
  `covariance` and `means`. A string variable is refused in Stata's own words.
- **ttest.** One-sample (`ttest x == 3`), paired (`ttest x == y`), two-sample (`by()` or `unpaired`),
  with `unequal` and `level()`. `welch` is refused: it asks for Welch's degrees of freedom and R's
  `t.test` offers only Satterthwaite's, which is what Stata's `unequal` means.
- **regress.** Stata's factor notation (`i.x`, `c.x`, `ib2.x`, `a#b`, `a##b`) becomes R's `factor()`,
  `:` and `*`, and the base level agrees without being told because both leave out the lowest value.
  `noconstant`, `robust`, `vce(robust|ols|cluster v)` and `level()`. The fit is kept in R as
  `stata_estimates`, which is where `predict`, `test`, `margins` and `e()` will look. Robust means HC1,
  which is Stata's, not R's default HC3; clusters add `G / (G - 1)` and `(N - 1) / (N - K)`.
- **Tests** (`engine/tests/stata_statistics.rs`, `stata_regress.rs`, 14 cases, real R). Every figure is
  checked against the same figure worked out a second way: the coefficients against the normal equations
  solved by hand, HC1 against the sandwich built from the residuals, the clustered errors against the
  clustered sandwich with both of Stata's corrections, the casewise correlation against the same
  correlation with the row removed, chi-squared against the 2x2 formula. HC0, HC3 and R's corrected
  chi-squared are shown to be different numbers, so the choices are real. A degenerate fixture was found
  this way: `x = 1:20` with four-row clusters makes the clustered variance of `x` exactly zero, so the
  test data are irregular now.
- **Not done:** the rest of P2B.4 (`logit`, `probit`, `poisson`, `nbreg`, `ologit`, `mlogit`, `anova`,
  `predict`, `test`, `lincom`, `margins`, `estat`, `xtset`/`xtreg`, `ivregress`, `stset`/`stcox`);
  weights; `by` with any of these; `tab1`/`tab2`; `pwcorr`; `summarize, meanonly`; `regress, beta`.
- **Note for later:** `ttest` is not in the vendored command table (`lang/commands.rs`), so it is matched
  by its full name only. Stata does not abbreviate it, so nothing is lost, but the commands still to come
  (`poisson`, `nbreg`, `mlogit` and the rest) will need entries there, and that table is vendored from
  StatCraft, so a change has to be made in both places.

## P2B.4 Stata models and postestimation, part two, 2026-10-30

`logit`, `probit`, `poisson`, `nbreg`, `ologit`, `oprobit`, `mlogit`, and after a model `predict`, `test`,
`lincom` and `estat` (`statatr/src/data_r/{ml,post}.rs`, with the shared pieces in `model.rs`).

- **What a model leaves in R.** Every model, `regress` included, now leaves `stata_estimates` (the fit),
  `stata_b` (`e(b)`), `stata_vcov` (`e(V)`, robust or clustered when asked) and `stata_df` (`e(df_r)`, or
  `Inf` where Stata uses z and chi-squared). The postestimation commands read only these, so `test` after
  `regress, robust` uses the robust variance, as Stata's does.
- **A fix to part one.** `regress, vce(cluster v)` had the right standard errors but took its t tests and
  intervals on R's `N - K` degrees of freedom. Stata uses `G - 1`. Fixed, and the regress tests now check
  the degrees of freedom, the interval width and the p-value, and show the `N - K` p-value is a different
  number. They also read the standard errors from `stata_vcov`, what the translation left, instead of
  recomputing the sandwich in the test. `lmtest` is no longer needed.
- **Stata's rules where R's defaults differ.** The outcome of `logit`/`probit` is 1 where it is not 0, so
  0/2 works. Standard errors are the observed information: the same as R's for logit and poisson
  (canonical links), but not for probit, where `summary(glm)` uses the expected information, nor for
  nbreg, where `glm.nb` holds theta fixed; both are worked out as Stata's. Robust errors after a
  likelihood model carry `N / (N - 1)` and clusters `G / (G - 1)`, not regress's HC1. nbreg's alpha is
  `1 / theta`, shown with `/lnalpha`. `ologit` cut points are `polr`'s zeta, renamed `/cut1`, `/cut2`.
  `mlogit`'s base is the most frequent outcome (R takes the first level); `baseoutcome()` picks another.
  Intervals are Wald, not `confint()`'s profile likelihood. `or`, `irr`, `rrr` exponentiate the estimate and
  interval and give the delta-method error, and never the cut points or `/lnalpha`. The iterations run to a
  tighter tolerance than R's default.
- **predict.** `xb` and `residuals` after regress; `pr` and `xb` after logit/probit; `n` and `xb` (with the
  exposure in, as Stata's) after poisson/nbreg; after ologit/oprobit/mlogit one variable per outcome in the
  order of the outcome values, or `outcome(v)` / `outcome(#k)`, or one variable meaning the first outcome.
  Every row with the values it needs is filled in, inside the estimation sample or not, as a float unless
  `double`. A name already in use stops, as in Stata.
- **test and lincom.** `test x w`, `test x = w`, `test 2*x + w = 1`, `test (x = 0) (w = 1)`; coefficients by
  Stata's names (`x`, `_cons`, `1.g`, `_b[x]`). An F where the model has residual degrees of freedom, a
  chi-squared otherwise. `lincom` with `level()`, `or`, `irr`, `rrr`, `eform`.
- **estat.** `ic` (degrees of freedom are the coefficients, as Stata counts them, so after regress it is 2
  below R's `AIC()`), `vce` (and `correlation`), and after regress `hettest` (on the fitted values or
  `rhs`, normal errors, Stata's default) and `ovtest` (RESET with powers 2 to 4 of the fitted values).
- **Found while testing.** `glm` returns `$weights` from the start of its last iteration, one step behind the
  estimates, so a variance built from them was off at the seventh digit. The weights and scores are now
  worked out from the final linear predictor, and the tests hold at 1e-10.
- **Tests** (`engine/tests/stata_models.rs`, 12 cases, real R). Each model's estimates are checked against
  its likelihood written out by hand (`dbinom`, `dpois`, `dnbinom`, `plogis`, `pnorm`, a softmax) and
  maximised with `optim`, and its standard errors against the Hessian of that hand-written likelihood.
  Robust and clustered logit are checked against the sandwich built by hand, and HC1's scaling is shown to
  differ. `test` after regress is checked against `anova` of nested models (the same F for least squares),
  after robust against the Wald test built by hand, and after clusters for `G - 1`. `hettest` is checked
  against the projection formula of the score test, `ovtest` against `anova` with the powers added, and
  `estat ic` against `-2 ll + 2k`. With the regress and statistics files that is 26 real-R cases for P2B.4.
- **Packages.** `MASS` for nbreg/ologit/oprobit and `nnet` for mlogit. Both are R's recommended packages,
  but this machine's conda R had no `nnet`, so it was installed into RCraft's library from the pinned
  snapshot; the tests skip with a note where they are missing.
- **The command table needed nothing.** None of the new names (`poisson`, `nbreg`, `ologit`, `oprobit`,
  `mlogit`, `predict`, `lincom`, `estat`) can be abbreviated in Stata, and the reader passes a full name
  through unchanged; `logit`, `probit` and `test` were already in the table. So nothing had to be mirrored
  into StatCraft.
- **Not done:** `margins`; `anova`; `xtset`/`xtreg`; `ivregress`; `stset`/`stcox`; weights; `by` with any of
  these; `tab1`/`tab2`; `pwcorr`; `summarize, meanonly`; `regress, beta`; `robust` after nbreg, ologit and
  mlogit; `predict, stdp` and the other predict statistics; `test` and `lincom` after ologit/mlogit (several
  equations) and with interaction coefficients; `estat` with a varlist; the LR test of alpha after nbreg;
  `e(sample)` in an `if`.

## P2B.4 Stata margins, 2026-10-30

`margins` after `regress`, `logit`, `probit`, `poisson` and `nbreg` (`statatr/src/data_r/margins.rs`), and
two fixes to the model terms found on the way.

- **Two fixes to the models.** (1) `c.x##c.x` and `c.x#c.x` became R's `x * x` and `x:x`, which R reads as
  plain `x`, so Stata's quadratic was silently fitted as a straight line. `##` is now written out term by
  term (`x + I(x^2)`, `a + b + a:b`), and a continuous variable repeated in `#` is raised to its power.
  (2) `ib2.g` became `stats::relevel(factor(g), ref = "2")`, which fails on data that lack level 2, as the
  rows `predict` and `margins` build do when they set every row to one level. It is now a small helper,
  `stata_ib(g, "2")`, that puts the base first without needing it present. A test fits the quadratic and
  checks it against the normal equations with `x^2`.
- **What margins does.** It averages the model's default prediction (`xb`, `pr`, or `n` with the exposure
  in) over the estimation sample, after setting variables: a marginlist of factor variables (`margins g`),
  `at()` (values, numlists, several `at()`s), `atmeans`. `dydx()` of a continuous variable is the average
  derivative of the prediction, worked out through the model matrix, so interactions and squares are right;
  of a factor variable it is the change from the base level to each other level; `dydx(*)` is every
  covariate. `predict(xb)` gives the linear prediction. `if`/`in` narrow the rows. The standard errors are the
  delta method's with the model's own variance (robust or clustered when the model was), on t tests where the
  model has residual degrees of freedom, as Stata 14 and later do.
- **The estimation sample.** Each single-outcome model now keeps its rows as `stata_data`. Stata's margins uses
  the *current* values of the rows in `e(sample)`, so a copy is only right while the data are unchanged. The
  translator tracks that: any command between the model and `margins` that may change the data (anything
  outside a short list that leaves values and rows alone, and any line that was not translated, since it
  still runs in Stata) makes `margins` refuse with that reason rather than give a different number.
- **Tests** (`engine/tests/stata_margins.rs`, 9 cases, real R). Every margin is checked against its formula
  worked out by hand with an analytic gradient, never against the helper's numerical derivatives: the average
  probability after logit (exactly the share of successes) and its error, `mean(p (1 - p)) b` and its gradient,
  a factor's change from its base (with `ib1.g` turning the sign), the quadratic's `b1 + 2 b2 mean(x)` on 76
  degrees of freedom, `at()` and `atmeans`, the exposure after poisson, an `if`, and the robust variance. P2B.4
  now has 35 real-R cases.
- **Not done in margins:** after ologit, oprobit and mlogit (a margin per outcome); `atmeans` with factor
  variables (Stata uses the means of their indicators); `at()` with statistics such as `(mean) x`; `post`,
  `over()`, `eyex`/`dyex`/`eydx`, `contrast`, `pwcompare`, `vce(unconditional)`; interactions in the
  marginlist; `margins` after the data changed.
- **Still not done in P2B.4:** `anova`; `xtset`/`xtreg`; `ivregress`; `stset`/`stcox`; weights; `by`;
  `tab1`/`tab2`; and the smaller items in the list above.

## P2B.4 Stata anova, 2026-10-30

`anova` (`statatr/src/data_r/anova.rs`).

- **What it is.** Stata's `anova` is a linear model in which a variable is categorical unless written `c.x`,
  so each plain variable gets `i.` before going through the same term translation as `regress` (so `g##h`,
  `ib3.g` and `c.x` work the same). The fit is kept with Stata's dummy coding, base level the lowest, with
  `stata_b`, `stata_vcov`, `stata_df` and the estimation sample, so `test`, `predict` and `margins` work after
  it as after `regress`.
- **The table.** Partial sums of squares by default, which are Type III: the same model refitted with
  sum-to-zero contrasts, each term's sum of squares being the Wald statistic of its columns. Checked against
  Stata 19 itself (see the next section), including a continuous variable interacted with a factor, to about
  15 digits; StatCraft's probe 237 (`statcraft/plan/stata/anova.md`) had already found Type III. `sequential`
  gives Type I (R's own `anova()`, which does not depend on the coding). Model, Residual and Total rows, and
  R-squared, adjusted R-squared and root MSE, as Stata shows them.
- **An empty cell.** Stata still gives partial sums of squares there (with the interaction's df cut), but by a
  rule not matched yet: tried against Stata's numbers and ruled out are Type III with the aliased column
  dropped, Type II, and a reference-coded model with every choice of base levels (each gets the interaction
  right and the main effects wrong). So the R stops and says so; sequential sums of squares work.
- **Refused, with reasons:** nested terms (`a|b`), error terms (`/`), `repeated()`.
- **Tests** (`engine/tests/stata_anova.rs`, 8 cases, real R, on an unbalanced 3 x 2 design so the kinds of sums
  of squares differ). The partial sums of squares are checked against Type III by its definition (the
  sum-to-zero design matrix built by hand, each term's sum of squares as the rise in the residual sum of squares
  when its columns are dropped), with their df, F and p, and shown to differ from the sequential ones; the
  sequential ones against the same model comparisons in order; a covariate's against its t squared times the
  MSE; `ib3.g` gives the same table; `test 2.g 3.g` after `anova y g h` gives the table's F for g; an empty cell
  stops the R; and four tables against Stata 19's own sums of squares on the same data.
- **A correction.** The first version of this note said there was no Stata on this machine to check against.
  There is: Stata 19 SE, at `C:\Program Files\Stata19`. The check it made possible is the next section.
- **Still not done in P2B.4:** `xtset`/`xtreg`; `ivregress`; `stset`/`stcox`; weights; `by`; `tab1`/`tab2`;
  and the smaller items in the lists above.

## P2B.4 checked against Stata 19, 2026-10-30

The models, postestimation, margins and anova translations were run in Stata 19 SE on the same synthetic data
the R tests make, and compared number by number.

- **How.** `"C:\Program Files\Stata19\StataSE-64.exe" /e do probe.do`, run from PowerShell (Git Bash turns
  `/e` into a drive path, and Stata then quietly does nothing). The log lands next to the do-file. Following
  StatCraft's rules (`statcraft/AGENTS.md` section 5): own synthetic data only, never StataCorp's datasets;
  results printed with `%24.17g`; logs kept out of the repository, with the licence banner, and only the
  numbers and what they show recorded here and in the tests.
- **Agree to 10 digits or more:** `regress, vce(cluster)` (b, se, and `e(df_r)` = G - 1 = 7, and `test`'s F
  on 7 df), `regress, robust` (HC1), `logit, robust` (N / (N - 1)) and clustered (G / (G - 1)), `nbreg` with
  `/lnalpha`, `mlogit`'s base outcome (the most frequent), `estat ic` (Stata too counts only the coefficients),
  both `hettest`s, `regress y c.x##c.x w`, `margins, dydx(x)` after it (and Stata does show t there),
  `margins, at()`, `lincom`, and the four anova tables.
- **Two fixes it led to.** (1) `estat ovtest` differed in the sixth digit. Stata rescales the fitted values to
  [0, 1] and stores each power as a float before the regression, which moves F; the translation now does the
  same and agrees to 13 digits (the test shows the exact-powers F is about six digits away). (2) `probit`
  agreed only to 8 digits, and the score showed it was R's fault, not Stata's: about 1e-11 at Stata's
  estimates, 3e-7 at `glm`'s, because `glm` scores with the expected information and stops on the change in
  deviance. Newton steps on the observed information now follow `glm`'s probit fit (`stata_probit_newton`),
  and it agrees to 10 digits.
- **Agree to 6 to 8 digits, and why.** `ologit`/`oprobit`: the gradient is about 5e-6 at Stata's estimates and
  2e-7 at R's, so here Stata's own fit is the looser one. The `margins` after `logit yb x i.g` and `mlogit`
  coefficients agree to 7 to 8 digits. After `poisson` with a constant the average predicted count must equal
  the mean count exactly; R gives 2.4125 to 14 digits, Stata 2.4125000019.
- **Two earlier statements were wrong.** The anova note said there was no Stata here to check against, and the
  part-two note implied R's estimates were the tighter ones throughout; for probit they were not.
- **Tests.** `the_models_agree_with_stata_19` in `engine/tests/stata_models.rs` and
  `the_sums_of_squares_are_stata_19s_own` in `stata_anova.rs` hold Stata's numbers, at 10 digits where both
  converge fully and 6 where Stata's own iterations stop earlier, each saying which.
- **Part one too.** `summarize` (with `detail`: variance, skewness, kurtosis and all nine percentiles on data
  with decimals, so the percentile rule is really exercised; with `if`; a string variable's 0 observations; a
  variable with a missing value), `tabulate` (cell counts and the chi-squared, on a 3 x 3 and a 2 x 2),
  `correlate` (casewise with `(obs=9)`, and `covariance`) and all five forms of `ttest` (one-sample, two-sample,
  `unequal` with Satterthwaite's 77.949 df, paired, `unpaired`) all agree with Stata 19 to about 15 digits.
  Nothing needed fixing; the tabulate helper now keeps its chi-squared on the table it returns, so the test can
  read it at full precision. Pinned in `the_descriptive_commands_agree_with_stata_19` (`stata_models.rs`) and
  `the_statistics_agree_with_stata_19` (`stata_statistics.rs`). P2B.4 has 46 real-R cases.
- **The last four items, also checked.**
  - `predict` stores a float for the prediction and for the residuals, exactly as the translation does (no
    row differs from the float of the double value).
  - `ttest, level()`'s intervals agree to Stata's printed seven figures (90%, 99% and 80% levels).
  - `margins, atmeans` and `at()` **differed** in about the eighth digit: Stata sets each variable to the
    *float* of the value (the mean, or `0.1`), which is why after least squares its prediction at the means is
    1.1357311297706145 and not mean(y), 1.1357311360696081. The translation now does the same and agrees to 16
    digits.
  - The messages the translated R gives where it does what Stata does: of 18, 12 already matched word for
    word, and 6 did not and now do. `correlate` with a string variable does not stop in Stata 19: it prints
    `(s ignored because string variable)` and goes on, and only with nothing left says
    `too few variables specified`; the translation stopped, and part one's note was wrong to call that Stata's
    behaviour. The others: `margins, dydx(zzz)` (`invalid dydx() option;` then
    `zzz not found in list of covariates`), the wrong number of new variables after `ologit`/`mlogit`
    (`too few`/`too many variables specified`, then the two-line explanation), `save` over a file (just
    `file x.dta already exists`, no hint), `duplicates drop` with a varlist
    (`force option required with duplicates drop varlist`), and `reshape wide` with a varying variable (one
    `variable w not constant within id` line per variable, which now means checking before the pivot).
  - All of it pinned: `engine/tests/stata_messages.rs` (19 messages), `atmeans_and_at_agree_with_stata_19`
    (`stata_margins.rs`), `ttest_intervals_at_other_levels_agree_with_stata_19` (`stata_models.rs`).
- **A flaky test found on the way.** Ubuntu CI failed once with "Unable to read from file" on `models.dta`. The
  tests in a file run at once and each fixture wrote its data to the same folder (named by the process id), so
  one test could read a file another was still writing. Every Stata test fixture now takes a fresh folder per
  call.
- **Still not checked against Stata:** only the printed layout, which is deliberately R's, not Stata's. P2B.4
  has 49 real-R cases.

## P2B.4 Stata xtset and xtreg, 2026-10-30

`xtset` and `xtreg` with `fe`, `re` (the default) and `be` (`statatr/src/data_r/xt.rs`), written in base R
rather than left to a panel package, because Stata's estimators are specific.

- **What Stata does, and so the translation.** StatCraft's probe 174 (`statcraft/plan/stata/panel.md`) had
  decoded the estimators; this was checked in Stata 19 again on an unbalanced panel with gaps. fe: least squares
  on y - ybar_i + ybar, df = N - n - k, a regressor with no variation within a panel dropped with Stata's note.
  be: least squares on the panel means. re: Swamy-Arora, sigma_e^2 from fe, sigma_u^2 = s_be^2 - sigma_e^2 /
  T-bar with T-bar the harmonic mean of the T_i, theta_i, least squares on the quasi-demeaned data, z tests.
  R-squared within, between and overall as squared correlations; sigma_u, sigma_e, rho; fe's F test that all
  u_i = 0 against the pooled fit.
- **Decoded here: the robust scaling.** `vce(robust)` clusters on the panel and `vce(cluster v)` on v; both are
  scaled by G / (G - 1) and (N - 1) / (N - k), with k the regressors and the constant, *not* counting the panel
  effects. Found by trying the candidates against Stata's numbers: that one matches to 17 digits for fe (on the
  panel and on another cluster variable) and for re; the others are 1% to 12% off. fe then tests on G - 1 df.
- **After it:** `predict` (xb, the default, as a float unless `double`), `test`, `lincom` read Stata's e(b),
  e(V), e(df_r) as after any model. `margins`, `estat ic` and `predict, u|e|ue` are refused for now.
- **Stata's words:** `xtreg` with no `xtset` is refused with Stata's `must specify panelvar; use xtset`; a
  repeated time in `xtset` stops the R with `repeated time values within panel`.
- **A bug found on the way, not in xtreg.** A test file named `repeat.dta` made the translation write
  `repeat <- ...`, and `repeat` is reserved in R. A data frame named after its file now gets `data_` in front of
  any name R reserves (`if`, `function`, `repeat` ...) and of any name starting `stata_`, which the
  translation's own objects use (a file `stata_b.dta` would have been overwritten by the next model).
- **Tests** (`engine/tests/stata_xtreg.rs`, 4 cases, real R): fe against least squares with a dummy for every
  panel (the same slopes, and with the same df the same standard errors), be against least squares on the
  means, and every estimator, robust and clustered error, R-squared, sigma, rho, theta, the F test, `predict` and
  `test` against Stata 19's own numbers, to 10 digits. P2B.4 has 53 real-R cases.
- **Not done:** `xtreg, mle` and `pa`; `margins`, `estat ic` and `predict, u|e|ue` after xtreg; `xtset` alone
  and its options (`delta()`, ...); `fe` with factor variables whose levels are absorbed; the other `xt`
  commands (`xtsum`, `xtlogit`, ...).

## P2B.4 Stata ivregress, 2026-10-30

`ivregress 2sls|liml|gmm depvar [exog] (endog = instruments)` with `small`, `vce(robust|cluster v)` (2sls),
`first` and `level()`, then `estat endogenous`, `estat overid`, `predict` (xb, residuals), `test`, `lincom`
(`statatr/src/data_r/iv.rs`), written in base R.

- **What Stata does.** StatCraft's probe 219 (`statcraft/plan/stata/ivregress.md`) had decoded the estimators,
  the default and robust variances and the `estat` tests; all of it was checked again in Stata 19 here. 2SLS is
  (X'P_Z X)^-1 X'P_Z y; LIML the k-class estimator with kappa the smallest root of |W1 - kappa W| = 0; GMM two
  steps from 2SLS with the robust weight matrix and its sandwich variance. Without `small` the variance divides
  by N and the tests are z and chi-squared; with `small`, N - k and t and F.
- **Decoded here: the clustered scaling.** Without `small`, Stata's clustered errors carry *no* G / (G - 1): the
  plain clustered sandwich. With `small`, G / (G - 1) times (N - 1) / (N - k), and t on G - 1. Found because the
  first try (with G / (G - 1) always, as `regress` does) was off by exactly sqrt(11/12) with 12 clusters. Plain
  robust is likewise unscaled without `small` and N / (N - k) with it.
- **Stata's words:** an under-identified equation stops the R with Stata's
  `equation not identified; must have at least as many instruments not in the regression as there are instrumented variables`.
- **Tests** (`engine/tests/stata_ivregress.rs`, 3 cases, real R): 2SLS against two literal stages of least
  squares (the same coefficients, and the second stage's own standard errors shown to be wrong), and every
  estimator, variance, Wald chi2 or F, kappa, R-squared, root MSE, both `estat` tests (with one and two endogenous
  variables), `predict` and `test` against Stata 19's numbers to 10 digits. P2B.4 has 56 real-R cases.
- **Not done:** `liml` with robust or clustered errors; `gmm` with its own `vce()` or `wmatrix()`, `igmm`;
  `estat firststage`; `estat endogenous` and `overid` after robust errors (Stata switches to robust score
  tests), after `liml` and `gmm`; several `(endog = inst)` groups; factor variables inside the parentheses;
  `margins` and `estat ic` after it; `noconstant`.

## P2B.4 tab1, tab2, pwcorr, meanonly and beta, 2026-10-30

The first of the small items left on the lists above (`statatr/src/data_r/{stats,model}.rs`), each run in Stata 19
first on the statistics tests' own data, so the rules below are Stata's and not guesses.

- **tab1** (`missing`, `nolabel`, `sort`): a one-way table of each variable, each on the rows where that variable is
  present. **tab2** (`firstonly`, `chi2`, `row`, `column`, `cell`, `missing`, `nolabel`): a two-way table of every
  pair, each on the rows where both of its variables are present (Stata does not drop a row for a missing value in a
  third variable); `firstonly` keeps the pairs with the first variable. One variable stops the R with Stata's
  `too few variables specified`.
- **pwcorr** (`sig`, `obs`, `star()`, `print()`, `bonferroni`, `sidak`, `listwise`/`casewise`): each correlation on
  its pair's rows, p from t on n - 2 df, Bonferroni `min(1, k p)` and Sidak `1 - (1 - p)^k` with k the number of
  pairs, and `star()`/`print()` read the adjusted p, as Stata's do. A string variable is left out with Stata's note,
  as `correlate` does; fewer than two left stops with `too few variables specified`. `bonferroni` with `sidak` is
  Stata's `invalid syntax`, refused.
- **summarize, meanonly**: prints nothing, as in Stata, and keeps Stata's r() of the last variable (N, sum_w, mean,
  min, max, sum) as `stata_r`, ready for `r()` in P2B.6. With `detail` it is refused in Stata's words
  (`only one of meanonly or detail may be specified`).
- **regress, beta**: e(beta), the coefficient times sd(x) / sd(y) over the estimation sample (the centred sd, with
  `noconstant` too; none for the constant), shown in place of the interval as Stata does, also with `robust`. With
  `vce(cluster)` refused in Stata's words (`options vce(cluster clustvar) and beta may not be combined`).
- **Tests** (`engine/tests/stata_statistics.rs`, 2 new cases, real R): table counts and both chi-squareds (g by z's is
  exactly 9 on 8 df, worked out by hand), every pwcorr correlation, p-value, count and adjustment, the e(beta) of
  four fits and the meanonly results against Stata 19's numbers to 12 digits; the p-value against `cor.test`, Sidak's
  and beta by their definitions; `star()`/`print()` in the printed table; the Stata messages. P2B.4 has 58 real-R
  cases.

## P2B.4 by with the statistics commands and the models, 2026-10-30

`by varlist:`, `bysort varlist:`, `by varlist, sort:` and `bysort g (h):` before `summarize`, `tabulate`, `tab1`, `tab2`,
`correlate`, `pwcorr`, `ttest`, `regress`, `logit`, `probit`, `poisson`, `nbreg`, `ologit`, `oprobit`, `mlogit`,
`anova`, `xtreg` and `ivregress` (`Session::by_each` in `statatr/src/data_r.rs`, `stata_by_groups` in `stats.rs`).
The data commands keep their own `by` through `dplyr::group_by`.

- **How.** The command is translated once, against a frame `stata_d`, and the R runs it in a loop at the top level over
  the groups, so after a model `stata_b`, `stata_vcov`, `stata_df` and `stata_data` are the last group's, which is
  Stata's `e()` after `by` (checked: `bysort g: regress z x` leaves the second group's b, se and N).
- **Stata's rules, from Stata 19.** The groups come in sorted order with a missing value last as a group of its own
  (`-> g = .`), strings by their bytes; the heading shows value labels (`-> g = low`) and every variable
  (`-> g = low, b = 0`); `quietly` hides the headings too. `bysort` sorts the data in memory. Plain `by` on rows out
  of order is Stata's `not sorted`; Stata decides that from the dataset's sort flag, so a file saved unsorted fails
  there even when its rows are in order, while the R checks the rows themselves and so runs the do-files that run in
  Stata. `in` is refused in Stata's words (`'in' may not be combined with 'by'`), as is any `by` option but `sort`
  (`rc0`), and a varlist that is not plain names.
- **Tests** (`by_runs_each_group_in_turn_as_stata_19_does`, real R): the headings and their order in all those cases,
  each group's figures against the same command on that group alone, Stata 19's chi-squareds, correlations, `(obs=)`
  and t inside `by`, and the last group's regress and logit estimates. The logit holds to 6 digits: on its five rows
  Stata stops with a score of about 4e-7, R's estimates have one of about 1e-15. P2B.4 has 61 real-R cases.
- **Found on the way, and done: Stata's check for perfect prediction** (`stata_perfect`, `ml.rs`). `by g: logit b x`
  on this data gives `outcome = x <= 4 predicts data perfectly` in Stata, where R's `glm` warns and prints huge
  coefficients. Decoded in Stata 19 on twelve rows built for it: before `logit` and `probit`, each regressor in turn
  (a factor variable as one indicator per level, the base included, in value order), on the rows still left, is
  tested for a cut c. The outcome 1 below and 0 above, or the reverse, with no value shared: Stata stops,
  `outcome = x <= 4 predicts data perfectly` (or `x > 8`; r(2000)). Determined on both sides and mixed at c: the
  variable is omitted and only the rows at c kept (`note: outcome = x > 4 predicts data perfectly except for x == 4
  subsample: x omitted and 10 obs not used.`). Determined on one side only: the same, worded
  `note: d != 0 predicts success perfectly; d omitted and 4 obs not used.` (`0.d != 1` for a factor's base). A factor
  left with one level, and its interactions, then go for collinearity. The translation does all of this and fits
  again on what is left; a `by` loop goes on past a group's r(2000), as Stata's `by` does. Tests
  (`stata_models.rs`, 2 cases): all eleven of the probe's fits, the kept coefficients and e(N) against Stata 19's to
  10 digits and every note and message word for word, and an ordinary logit against `glm`. Still not translated:
  the zero Stata keeps in e(b) for an omitted variable (R's coefficients leave it out).
- **Not done:** `by` with `margins`, `predict`, `test`, `lincom`, `estat`; `by`'s `rc0`.

## P2B.4 weights, 2026-10-30

`[fweight]`, `[aweight]`, `[pweight]` and `[iweight]` (and their abbreviations) for `summarize` (with `detail` and
`meanonly`), `tabulate`, `tab1`, `tab2`, `correlate`, `pwcorr`, `regress` (with `robust` and `vce(cluster)`), `logit`,
`probit` and `poisson` (`statatr/src/data_r/weights.rs`, and the helpers they feed). Every rule below was decoded
in Stata 19 on the statistics tests' data with `f = mod(id, 3) + 1` and `a = id / 3`.

- **The rows.** A missing or zero weight leaves its row out, for every kind (iweights too). A negative weight stops
  with `negative weights encountered` (not for iweights), and a frequency weight that is not whole with
  `may not use noninteger frequency weights`; both are Stata's messages, raised in R once the data are seen.
- **The kinds.** Frequency weights are copies of the row: N and the degrees of freedom are their sum (a test checks
  summarize and regress against the rows really repeated). Analytic weights are scaled to sum to the rows, so the
  count stays the rows, the variance is `sum w (x - m)^2 / sum w * n / (n - 1)` and `tabulate`'s counts are scaled
  to sum to N. Importance weights are taken as they are: variance over `sum w - 1`, regress's over `sum w - k` with
  e(df_r) the whole part of `sum w` less k. Sampling weights give analytic weights' estimates with robust errors.
- **summarize, detail with weights.** Weighted moments for skewness and kurtosis, and Stata's weighted percentile:
  the first value whose running sum of weights reaches p W, or the mean with the next where it equals it exactly
  (with unit weights that is the old rule).
- **regress.** Weighted least squares; the variance as above, the robust sandwich with scores `w e x` scaled by
  `N / (N - k)` (for fweights the meat is `w e^2 x x'`, rows as copies) and the clustered one by
  `G / (G - 1) (N - 1) / (N - k)`, N the weights' sum for fweights and the rows otherwise (`stata_lm_wvcov`).
- **logit, probit, poisson.** The weighted likelihood; fweights and iweights take the weighted information, pweights
  the sandwich scaled by `N / (N - 1)`, fweights with `robust` the sandwich with rows as copies and N their sum.
  The probit polish and the perfect-prediction check carry the weights. R's own binomial `logLik` rounds weights
  that are not whole numbers, so the log likelihood in the header is now worked out by hand (found by a weighted
  test: R said -12.33, Stata -12.55).
- **Refused in Stata's words:** `pweights not allowed` (summarize, tabulate, correlate), `iweights not allowed`
  (`summarize, detail`, correlate), `aweights not allowed` (logit, probit, poisson), `chi2 not allowed with aweights`,
  `option chi2 not allowed` (iweights). Not translated yet, with a reason: `[weight = ...]` (each command reads it
  as a different kind), a weight that is an expression, robust errors with iweights, `regress, beta` with weights,
  and `margins` and `estat` after a weighted model.
- **Tests** (`engine/tests/stata_weights.rs`, 5 cases, real R): summarize (every kind, detail, meanonly, a zero
  weight), tabulate, tab1, correlate (and covariance), pwcorr, regress (every kind, robust, two clusterings, a zero
  weight) to 12 digits; logit, probit and poisson with every kind they take to 6 digits (Stata stops earlier: after
  `logit [fw=f]` its score is about 6e-7, R's 1e-15) and their log likelihoods to 10; the messages. P2B.4 has 66
  real-R cases.
- **Not done:** weights in `nbreg`, `ologit`, `oprobit`, `mlogit`, `anova`, `xtreg`, `ivregress`, `ttest` (Stata
  takes none there) and `collapse`; `margins` and `estat` after a weighted model; robust errors with iweights.

## P2B.4 the postestimation gaps, 2026-10-30

The smaller items of part two's list, each decoded or checked in Stata 19 on the models tests' data
(`statatr/src/data_r/{ml,post}.rs`, `expr_r.rs`).

- **Robust and clustered errors after nbreg, ologit, oprobit and mlogit.** Each row's score (the gradient of its log
  likelihood: for nbreg in b and `/lnalpha`, for ologit/oprobit in b and the cut points, for mlogit in each
  equation but the base's) in a sandwich around the model's own variance, scaled by `N / (N - 1)` or
  `G / (G - 1)` (`stata_ml_sandwich`). Agrees with Stata to 6 digits or better, the bread for ologit and mlogit
  being R's numerical Hessian as before.
- **The LR test of alpha after nbreg:** the Poisson fit on the same rows against it, a chi-bar-squared(01) with p
  half of chi2(1)'s tail, printed as Stata does and, as in Stata, not with robust or clustered errors. Stata's
  chi2_c to 6 digits. (Stata's `e(p)` after nbreg is the model test's, not this one's.)
- **predict's other statistics:** `stdp` after every one-equation model (`sqrt(x V x')`, the model's own V);
  after regress `stdf`, `stdr`, `rstandard`, `rstudent`, `cooksd` and `leverage`/`hat` (s the root MSE, h from
  `(X'X)^-1`, rstudent with s worked out without the row); `score` after logit, probit and poisson; `ir` after
  poisson and nbreg (`exp(xb)` without the exposure). Stata fills them in for every row with the values they need,
  out of the sample too, and so does the translation; all agree with Stata's stored floats, in and out of the
  sample. The residual statistics after robust, clustered or weighted regress are refused for now.
- **test and lincom after ologit, oprobit and mlogit, and with interactions** (`stata_resolve`, `stata_bnames`).
  The names the translation cannot know before the model runs are looked up in R by Stata's name for each of R's
  coefficients: `1.g#c.x` in either order and with or without `c.` (Stata takes all three), `c.x#c.w`, `c.x#c.x`
  for a square, ologit's `_b[/cut1]`, mlogit's `[2]x`, `[3]_cons` and `_b[2:x]`. Stata's rules after mlogit:
  `test x` is x in every equation but the base's (chi2(2) here), `test [3]` every coefficient of equation 3 but its
  constant, `lincom x` the first equation's x, and a base outcome's coefficient is a zero (so `test [2]x = [3]x`
  with 2 the base tests `[3]x`); a test with nothing left stops and says why, where Stata prints a test with 0 df.
  A name without an equation inside an equation after mlogit is refused with a reason.
- **estat hettest with a varlist** (and with `rhs` as well: the union), over the estimation sample.
- **e(sample)** in an expression (`summarize x if e(sample)`, `gen byte es = e(sample)`): kept in R as
  `stata_esample` after regress and the likelihood models, by running the model's data expression again on the
  data with each row's number added. It is translated only while the rows are where the model found them: a sort,
  a by-sort, a command that may add or drop rows, or an untranslated line in between makes it refused with the
  reason (and so does `by` before the model).
- **Tests** (`stata_models.rs`, 3 new cases, real R): every robust and clustered standard error of the four models,
  the LR test, the seven regress statistics in rows 1 to 3 (in and out of the sample) and the logit and poisson
  ones, `stdp` against `predict(se.fit = TRUE)` and leverage against `hatvalues`, and every test, lincom, hettest
  and e(sample) figure above against Stata 19. P2B.4 has 69 real-R cases.
- **Not done:** `predict` after mlogit/ologit other than probabilities (`xb`, `stdp` per equation), `score` after
  nbreg (two equations), the residual statistics after robust or weighted regress, `e(sample)` after xtreg,
  ivregress and anova, a test with 0 df printed as Stata prints it.

## P2B.4 the rest of margins, 2026-10-30

The items left on the margins list (`statatr/src/data_r/margins.rs`), each run in Stata 19 first.

- **A fix.** `margins g h` was taken as the cells of g and h together; Stata gives the margins of g and then those of
  h. Each marginlist term is now its own, and `margins g#h` gives the cells (g outermost, as Stata lists them).
- **New:** `over(v)` (each group of the estimation sample on its own); `at((mean) x (median) z (p25) w x=1)` with
  `mean`, `median`, `min`, `max` and `p1`-`p99` of the estimation sample (Stata's percentile rule); `eyex()`,
  `dyex()`, `eydx()` (the row's derivative times x / y, x, or over y, averaged); `atmeans` with factor variables
  (each indicator at its mean, every column of the design the product of its parts' means, all as floats, so a
  derivative at the means moves x after the float); `pwcompare` and `pwcompare(effects)` (every pair, `1vs0.g`);
  `r.g` in the marginlist (each level against the first); `contrast` (Stata's joint F, or chi2, that a term's
  margins are equal); `post` (the margins become e(b) and e(V) under Stata's names, `1.h`, `0.g#1.h`, `x`, `1._at`,
  and `test`/`lincom` read them by those names, `0bn.h` included); margins after `ologit`, `oprobit` and `mlogit`
  (one per outcome, from the outcome probabilities rebuilt from e(b) so the delta method reaches the cut points
  and every equation; `predict(outcome(2))`/`outcome(#k)`; `dydx()` per outcome).
- **Margins after the data changed.** Stata averages the current values of the rows e(sample) marks. Now that
  e(sample) is kept (see the postestimation section), `margins` after a `replace`, `recode`, `generate`, `label`,
  `drop` of variables and the like uses the current values of those rows; a sort, a drop of rows, an append or an
  untranslated line in between still makes it refuse with the reason. (e(sample) now survives the commands that
  change values but not rows.)
- **Tests** (`stata_margins.rs`, 1 new case, 18 Stata figures plus the contrast and post): every margin and standard
  error above against Stata 19, to 6 digits after logit, ologit and mlogit (their fits), to 9 after regress (the
  Jacobian in b is numerical). P2B.4 has 70 real-R cases.
- **Not done:** `vce(unconditional)` (Stata allows it only after robust errors); `contrast()` and `pwcompare()` with
  suboptions (`effects` aside), and other contrast operators (`a.`, `ar.`, ...); contrasts of marginal effects;
  margins after `xtreg` and `ivregress`; margins after a weighted model; `at()` statistics other than the ones above
  (`asobserved`, `zero`, ...).

## P2B.4 the rest of xtreg, 2026-10-30

The items left on the xtreg list (`statatr/src/data_r/xt.rs`), decoded in Stata 19 on the xtreg tests' panel.

- **predict after xtreg:** `u` (fe: ybar_i - xbar_i b - _cons; re and mle: the BLUP,
  `T_i s_u^2 / (T_i s_u^2 + s_e^2)` times the panel's mean of y - xb), `e` (y - xb - u), `ue` (y - xb), `xbu`
  (xb + u) and `stdp`; u for every row of a panel that was in the sample, as Stata's manual says it fills it in.
- **estat ic after fe:** the within regression's normal log likelihood (Stata's e(ll), and e(ll_0) for the
  constant-only within model), its coefficients with the constant as df, AIC and BIC on N. After re and be Stata 19
  has none (r(321)), and the R stops saying so.
- **margins after xtreg:** the linear prediction, as Stata's default after it (`margins`, `dydx()` and the rest of the
  margins options); xtreg now keeps its estimation sample (`stata_data`) and e(sample).
- **xtset alone** shows the settings again (the time variable is remembered too), and the unit options (`yearly`,
  `delta()`, ...) are accepted: they only label the time and matter for lags, which are not translated.
- **A factor level absorbed by the panels** is dropped with Stata's own note, `note: 1.gg omitted because of
  collinearity.` (it said `factor(gg)1`).
- **xtreg, mle:** the random-effects likelihood maximised in b, ln sigma_u and ln sigma_e (analytic gradient, Newton
  steps), e(b) with `/sigma_u` and `/sigma_e` (delta-method errors), the LR model test against the constant-only
  model (Stata's e(ll_0)), rho, and the chi-bar-squared test of sigma_u = 0 against the pooled fit (e(chi2_c)).
  The estimates and log likelihood agree with Stata to 8 digits or better (R's log likelihood is the higher, so R
  is at the maximum); the standard errors to about 5: R's come from the exact Hessian, which the numerical one
  reproduces at every step size, so Stata's own is a numerical one about 1e-5 away.
- **xtreg, pa** (GEE, `corr(exchangeable)`, the default, or `independent`; `vce(robust)`): decoded by trying the
  candidates against Stata's numbers. phi = sum e^2 / N; alpha = the within-panel cross products over
  `phi sum T_i (T_i - 1)`; b by GLS with that correlation, iterated from least squares and stopped at the first
  iterate whose largest relative change is below 1e-6, which is where Stata stops (that iterate matches Stata to 16
  digits; the converged one would be 1e-7 away); the variance phi (sum X_i' R^-1 X_i)^-1 with phi from the final
  residuals (as is Stata's e(phi)); robust, the panel-clustered sandwich with G / (G - 1).
- **Tests** (`stata_xtreg.rs`, 1 new case): all of the above against Stata 19 (predict as floats, estat ic to 12
  digits, margins to 9, mle to 8 and its errors to 4, pa to 9, the note word for word). P2B.4 has 71 real-R cases.
- **Not done:** `xtreg, mle` with robust errors, `pa` with `vce(cluster)` or other correlations (`ar 1`,
  `unstructured`, ...), `predict` after pa other than xb and stdp, the other `xt` commands (`xtsum`, `xtlogit`, ...).

## P2B.4 the rest of ivregress, 2026-10-30

The items left on the ivregress list (`statatr/src/data_r/iv.rs`), each decoded in Stata 19 on the ivregress tests'
data by trying the candidate formulas against its numbers.

- **liml with robust and clustered errors:** the sandwich around (X_k' X)^-1 with the instruments' fitted X times u
  as the scores (not X_k: that one is 6e-4 away), scaled as 2SLS's.
- **gmm:** `vce(unadjusted)` is N (Q' W Q)^-1 with W the estimation weight; `wmatrix(unadjusted)` gives 2SLS's b with
  the homoskedastic variance s^2 N (Q' (Z'Z / N)^-1 Q)^-1; `wmatrix(cluster v)` the clustered weight and variance
  (no G / (G - 1), as 2SLS's clusters); `igmm` iterates the weight from the current residuals until both b and W
  change by less than 1e-6 (relatively), where Stata stops; Hansen's J = N g' W g with the estimation weight (with
  an unadjusted one, s^2 from the final residuals), kept with the fit and shown by `estat overid`.
- **estat firststage:** each endogenous variable's first-stage R-squared, adjusted R-squared, partial R-squared and F
  of the excluded instruments (after robust errors the HC1 Wald F), and the Cragg-Donald minimum eigenvalue
  statistic.
- **The estat tests after robust errors and after liml:** `estat endogenous` after robust is Wooldridge's robust
  score test (N less the residual sum of squares of 1 on the OLS residuals times v, v orthogonal to every
  regressor); `estat overid` after robust is two-step GMM's J (weight from the 2SLS residuals); after liml, the
  Anderson-Rubin chi2 N (kappa - 1) and Basmann's F (kappa - 1)(N - L)/l; after liml and gmm, `estat endogenous` is
  not available, as in Stata 19. After clustered errors these are refused for now.
- **Also:** `noconstant` (no constant in X or Z, and R-squared missing, as Stata's); factor variables inside the
  parentheses (`(x = i.zc z1)`, the base level left out); `margins` after ivregress (the linear prediction), with the
  estimation sample and e(sample) kept; `estat ic` refused in Stata's terms (r(321)); two parenthesised groups
  refused in Stata's words (`invalid syntax`; the syntax is one group).
- **Tests** (`stata_ivregress.rs`, 1 new case): seven fits' coefficients and errors, four J's, noconstant, both
  firststage F's, the robust and liml tests and margins against Stata 19 to 9 digits or better. P2B.4 has 72 real-R
  cases.
- **Not done:** gmm's `wmatrix(hac ...)` and `vce(hac ...)`, `center`; the estat tests after clustered errors;
  `estat firststage` after clustered errors; Shea's partial R-squared with several endogenous variables.

## P2B.4 unconditional margins, and anova's error terms, nesting and repeated(), 2026-10-30

The last big items of the lists above, each decoded in Stata 19.

- **margins, vce(unconditional)** after regress, logit, probit and poisson with robust or clustered errors (Stata
  allows it only then, and says so in the words the refusal now uses). Each row's part of the margin,
  `(f_i - m) / N`, plus its part through b (its influence on b times the margin's Jacobian), summed in squares or by
  cluster and scaled by the model's own small-sample factor: N / (N - k) after regress (HC1's), N / (N - 1) after
  logit, G / (G - 1) with clusters and (N - 1) / (N - k) as well after regress. A first reading from a prototype
  suggested N / (N - 1) after regress with a special case for constant effects; rerun in the test session, the
  plain rule above fits all seven of Stata's figures and needs no special case. Refused with an `if`, `over()`,
  `atmeans`, changed data, or after the other models.
- **anova with error terms (`/`):** each term before a `/` is tested against the first term after it (against the
  residual after a trailing `/`), and the first term of the last group, being an error term, is not tested (Stata's
  F is missing there). The terms keep the order written (R would put main effects first and lose the places the
  error terms are named by).
- **Nested terms (`b|a`):** the columns are sum-to-zero contrasts among the levels of b found within each level of a
  (R's own coding of `a:b` leaves empty columns when, as for subjects within groups, most combinations are absent),
  so the partial sums of squares are Stata's.
- **repeated(v):** the Huynh-Feldt, Greenhouse-Geisser and Box epsilons from the covariance of the repeated measures
  within the subjects (the between-subjects error term, `subj` or `subj|grp`), pooled over the between-subjects
  groups, HF with `N - g` in its denominator and capped at 1; the p-values of the terms with v, unadjusted and
  adjusted by each.
- **Tests** (`stata_margins.rs` and `stata_anova.rs`, 2 new cases): the seven unconditional margins, four error-term
  and nesting designs and the repeated-measures design (sums of squares, F's and the three epsilons) against Stata
  19 to 9 digits. P2B.4 has 74 real-R cases.
- **Still not done in P2B.4, of the lists above:** `stset`/`stcox` (next); anova's partial sums of squares with an
  empty cell (Stata's rule is still not matched) and with a nested design that has one; `##` together with `/` or
  `|`; `repeated()` with several variables; the other `xt` commands (`xtsum`, `xtlogit`, ...), which are commands of
  their own; and the smaller items each section above names (weights in the other models, `margins` contrasts with
  suboptions, gmm's HAC, the estat tests after clustered errors, `by` before the postestimation commands).

## P2B.4 stset and stcox, 2026-10-31

`stset` and `stcox` (`statatr/src/data_r/st.rs`), on R's `survival::coxph`, checked against Stata 19 on sixty subjects
with 42 failures at only 16 distinct times, so the handling of ties is really exercised.

- **stset** `timevar, failure(var[==values]) id() origin() enter() exit() scale()` makes Stata's `_st`, `_d`, `_t`
  and `_t0`: the time from the origin over the scale, entry at the later of origin and `enter()`, failures after
  `exit()` censored there, a subject's records under `id()` each starting where its last one ended, and a row that
  enters at or after its end not used (`_st` 0), as with Stata's `enter(t0)` here (57 of 60 rows left). It prints
  Stata's counts. `origin()`, `enter()` and `exit()` take a number or a variable (`time` before it is accepted);
  expressions and `var==value` forms are refused for now.
- **stcox** `varlist` (factor notation too), with `breslow` (the default, as Stata's; R's own default is Efron's),
  `efron`, `exactp` (R's `exact`, the exact partial likelihood; with no late entry, since R's takes no entry
  times), `vce(robust|cluster v)`, `strata()`, `nohr`, `level()`. coxph's estimates, standard errors and log
  likelihoods are Stata's to 15 digits; robust errors are coxph's sandwich times N / (N - 1) and clustered ones
  times G / (G - 1) (robust with `id()` clusters on the subject, as Stata's). The table shows hazard ratios, e(b)
  the coefficients; the header the subjects, failures, time at risk, LR (or, robust, Wald) chi2 and log likelihood.
  `predict, xb` is x b with nothing subtracted (R's `predict()` would centre it) and `hr` its exponential; `test`
  and `lincom` read e(b) and e(V) as after any model; e(sample) is kept.
- **Found on the way:** R's `coxph` wants a `cluster` or `id` for robust errors on (start, stop] data, so robust
  without clusters gives each row its own cluster (the same sandwich); and it looks for `cluster =` in the data
  first, where a variable of the same name (`cl` here) would have been taken instead, so the clusters go in a
  column of their own.
- **Refused:** `exactm` (Stata's exact marginal likelihood has no counterpart in R's survival), `stcox` before
  `stset` (Stata's `data not st; use stset`), `margins` after stcox for now.
- **Tests** (`engine/tests/stata_stcox.rs`, 3 cases, real R): all three ties methods (coefficients, errors and both
  log likelihoods), robust and clustered errors, factor variables with `nohr`, `test`, `predict hr` and `xb`, the
  header, `strata()`, and stset's `scale()`, `id()`, `enter()`, `exit()` and `origin()`, against Stata 19 to 10
  digits. P2B.4 has 77 real-R cases.
- **Not done:** `estat phtest` (Schoenfeld residuals; R's `cox.zph` transforms time differently by default), the
  other `st` commands (`sts`, `streg`, `stsplit`, ...), `margins` after stcox, `stset` with expressions or
  `var==value` in `origin()`/`enter()`/`exit()`, `exactp` with late entry.

## P2B.4 more leftovers, 2026-10-31

Items from the not-done lists above, each run in Stata 19 first.

- **Weights in nbreg, ologit, oprobit and mlogit** (`fweight`, `pweight`, `iweight`; `aweights not allowed`, as
  Stata says): the weighted likelihood (nbreg's variance from its weighted likelihood, alpha included), sampling
  weights' sandwich with the weighted scores and N / (N - 1), frequency weights counting as copies (e(N) their sum,
  161 here); mlogit's base outcome by weighted frequency. To 6 digits (numerical Hessians, as before).
- **margins after a weighted model:** the weighted mean over the estimation sample, as Stata's (logit with pweights
  and fweights, regress with aweights checked). `atmeans` and `at()` statistics after weights are refused for now.
- **estat ic after frequency weights:** N their sum and the weighted log likelihood (regress's worked out by hand:
  R's `logLik.lm` counts the rows); `estat vce` after any weights; the other `estat`s after weights refused.
- **predict:** `xb` and `stdp` after ologit and oprobit (no cut points), after mlogit for one equation
  (`outcome()`, the first by default, the base's being zero), and nbreg's two `scores` (xb's and ln(alpha)'s).
- **Stata's own words:** `by` before `predict`, `test`, `lincom`, `margins` and `estat` is Stata's error
  (`predict may not be combined with by`, ...), and after robust or clustered errors every residual statistic but
  `stdp` is too (`option rstandard not allowed after robust estimation`); the translation now says exactly that.
- **After stcox:** `estat phtest` (and `, detail`) is the Grambsch-Therneau test on analysis time, as Stata's (R's
  `cox.zph` now gives an exact score test, a different number): the global chi2 to 10 digits, each covariate's rho
  and chi2 to 6 (Stata works them out from the scaled residuals kept as floats). `margins` after stcox averages the
  relative hazard, exp(xb), or with `predict(xb)` the linear prediction.
- **stset** takes expressions in `origin()`, `enter()` and `exit()` (`exit(time 4 * 5)`, `enter(time e0 - 1)`).
- **Found on the way:** an R list loses an element set to NULL, so a weights list kept beside the margins came out
  short for an unweighted model (`subscript out of bounds` after ivregress); kept with `list()` now.
- **Tests** (`stata_models.rs`, `stata_stcox.rs`, 2 new cases): every figure above against Stata 19. P2B.4 has 79
  real-R cases.
- **Still not done:** weights in anova, xtreg and ivregress; `atmeans` and `at()` statistics after weights; the other
  `estat`s after weights; `margins` contrasts with suboptions; gmm's HAC; the estat tests after clustered iv errors;
  `xtreg, mle` robust and `pa` with other correlations; `stset` `var==value`; `exactm`; anova with an empty cell.

## P2B.5 Stata graph commands to ggplot2, 2026-10-31

`histogram`, `kdensity`, `scatter`, `line`, `connected`, `twoway`, `lowess`, `graph bar`/`hbar`, `graph twoway` and
`graph export` (`statatr/src/data_r/graph.rs`), drawn with ggplot2 3.5.1, which is now in RCraft's R library (from
the pinned 2025-03-15 snapshot, as the user approved) and in CI's package list.

- **The principle.** What a graph shows is worked out in R the way Stata 19 works it out, and only the drawing is
  left to ggplot2, since R's defaults differ at every step. Each rule was decoded in Stata 19 on the models data:
  - `histogram`: `int(min(sqrt(N), 10 log10 N))` bins (8 for 80 rows) from the minimum, width (max - min) / bins,
    each bin closed below and the last also above; `bin()`, `width()` (bins enough to reach the maximum), `start()`
    (Stata's `option start() may not be larger than minimum of y`); `discrete` (one bar per value, the width the
    smallest gap); density, `fraction`, `frequency`, `percent`; `normal` and `kdensity` overlays on the bars'
    scale. R's `hist()` would choose Sturges' bins.
  - `kdensity`: the Epanechnikov kernel on +-sqrt(5) (R's default is the Gaussian), the bandwidth
    `0.9 min(sd, IQR / 1.349) N^(-1/5)` with Stata's quartiles and the same whatever the kernel, at min(N, 50)
    points from min - h to max + h; `epan2`, `biweight`, `cosine`, `gaussian`, `parzen`, `rectangle`, `triangle`;
    `bwidth()`, `n()`, `at()`, `generate()`, `normal`; Stata's title and kernel/bandwidth note.
  - `lowess`: Stata's, a local line with tricube weights over `int((0.8 N - 0.5) / 2)` neighbours on each side, the
    window 1.0001 times its half-width, no robustness steps; R's `lowess()` is a different curve (the test shows
    it). `bwidth()`, `generate()`.
  - `lfit` and `qfit`: least-squares lines over the data's range (`qfit` at 100 points).
  - `graph bar`/`hbar`: `(mean)` unless another statistic (`median`, `p#`, `sum`, `count`, `min`, `max`, `sd`) is
    named, of each variable within each `over()` group (value labels as the categories), `stack`.
- **twoway.** Layers in parentheses or split by `||`, each with its own `if`/`in`; `line` joins the points in the
  data's order unless `sort`, as Stata's; several layers or y variables get a legend labelled as Stata's
  (the variable label, `Fitted values`, `lowess y x`, `Kernel density estimate`), and the y title is shown when
  every layer shares its y.
- **Options.** `title()`, `subtitle()`, `xtitle()`, `ytitle()`, `note()`, `caption()`, `legend(off)` and `by()` (for
  plain layers) are translated; options that only change the look (`msymbol()`, colours, `scheme()`, `xlabel()`,
  ...) are left to ggplot2's defaults and named in a note; anything else is refused with its name. Every graph is
  kept as `stata_graph` and printed, so it shows in RCraft's Graph tab.
- **graph export** writes `stata_graph` with `ggsave` (png, pdf, svg, eps, jpg, tif), `width()`/`height()` in pixels,
  and stops with Stata's `file ... already exists` unless `replace`.
- **Tests** (`engine/tests/stata_graphs.rs`, 4 cases, real R and ggplot2): the histogram bins and heights of four
  forms against Stata 19's `twoway__histogram_gen`, kdensity's bandwidth, grid and densities for all seven kernels
  and `at()`, Stata's lowess, the bar statistics against Stata's collapse, lfit against `lm`; eleven graphs built by
  ggplot2; export, its refusal to overwrite, and the refusals. Four graphs were also rendered and looked at.
- **Not done:** `graph box`, `graph pie`, `graph matrix`, `graph combine`; twoway `area`, `bar`, `rcap`, `lfitci`,
  `function`, `scatteri`; `by()` with fitted layers or histograms; `xlabel()`/`ylabel()` as axis breaks; weights in
  graphs; graphs that look like Stata's own (its schemes).

## P2B.6 Stata's returned values: e(), r(), _b[], _se[] and display, 2026-10-31

`statatr/src/data_r/returned.rs`. Every translated model now leaves `stata_e` in R and the r-class commands leave
`stata_r`: named lists holding Stata's names, numbers as numbers, macros as text, `b` and `V` as matrices. What each
holds was read off Stata 19's `ereturn list` and `return list` on the test data (probes kept outside the repository).

- **e().** `stata_ereturn()` builds it from what the model's translation already left (`stata_estimates`, `stata_b`,
  `stata_vcov`, `stata_df`, the cluster), in the order of Stata's list:
  - `regress`: N, sum_w, df_m, df_r, F (the Wald F, so robust and clustered ones too), r2, rmse, mss, rss, r2_a, ll,
    ll_0, rank, N_clust; fweights count the copies; aweights and pweights are scaled to sum to N, and aweights add
    half the sum of the log of the scaled weights to ll and ll_0 (decoded: Stata's -176.7788818 for `[aw=a]`); the
    title only when no robust or clustered errors were asked for, as Stata leaves it out then.
  - `logit`, `probit`, `poisson`, `nbreg`, `ologit`, `oprobit`, `mlogit`: rank, N, k, k_eq, k_dv, converged, rc,
    ll, k_eq_model, ll_0, df_m, chi2 (LR, or Wald when robust), p, r2_p, and each one's own (nbreg's rank0, ll_c,
    alpha and ll_0 of nbreg's constant-only model, refitted; ologit's k_cat, k_aux; mlogit's ibaseout, baseout,
    k_eq_base, k_out, eqnames).
  - `anova` (without error terms or nesting), `xtreg` fe/re/be (N_g, g_min/avg/max, sigma_u, sigma_e, rho, r2_w,
    r2_b, r2_o, corr, F_f, p_f, ll; robust clusters on the panel, as Stata's), `ivregress` 2sls/liml/gmm (kappa,
    mss, rss, rmse, r2, r2_a, chi2 or F with small, df_r, k_endog, J), `stcox` (N_sub, N_fail, risk, ll, ll_0,
    chi2, r2_p; no e(p), as in Stata) and `margins, post`.
  - The macros: cmd, cmdline, depvar, title, vce, vcetype, clustvar, wtype, wexp, properties, predict, estat_cmd,
    chi2type and the rest Stata lists, as text.
- **r().** `summarize` (plain, `detail`, `meanonly`, in Stata's order, of the last variable, `quietly` too), `count`,
  `tabulate` (N, r, c, chi2, p), `correlate` (N, rho, C), `ttest var, by()` (all 28 values, equal or unequal),
  `test` (F or chi2, df, df_r, p, drop), `lincom` (t and df, or z without df after a likelihood model; with `or`
  the odds ratio, its bounds and its delta-method se, as Stata's) and `margins` (N, level). The helpers leave
  `stata_r` in the frame that called them, so their own return values are unchanged.
- **Who holds them.** The translator follows which command left e() and r(), as Stata does: r() stays after the
  commands Stata 19 was seen to leave it alone after (generate, replace, sort, list, label, rename, format, order,
  display, predict, save, use, egen, clear) and is replaced by everything else (drop, keep, describe, the graphs,
  collapse, recode, xtset and every model, which leave r(table), were checked); e() stays until a model is fitted or
  an untranslated command that may be one runs. `e(name)` and `r(name)` are translated to `stata_eget()` and
  `stata_rget()` only when the command that left them is known; a name Stata leaves there that RCraft does not keep
  (e(ic), e(N_cdf), r(gamma), anova's ss_1, ...) is refused with that reason, never left missing; a name Stata does
  not have is missing, as in Stata. Matrices (e(b), e(V)) are refused in an expression, as Stata's type mismatch.
- **_b[] and _se[].** `stata_bget()` reads `stata_b` and `stata_vcov` through the names helper: `_b[x]`, `_b[1.g]`,
  `_b[_cons]`, `_coef[]`, `[eq]_b[x]`, `_b[eq:x]` after mlogit (the base outcome's equation, first in Stata's
  order, is zeros), `_b[/cut1]`, `_b[/lnalpha]`; a base or omitted factor level is 0; Stata's messages for a
  name or an equation that is not there (`[nosuch] not found`, `equation [y] not found`).
- **display.** Quoted text, expressions (a variable is its first row, as in Stata), `_n`/`_newline()`, `_col()`
  (which only moves forward), `_skip()`, `_dup()`, a comma as a space, `as`/`in` colours dropped with a note, and
  Stata's formats through `stata_fmt()`: `%w.dg`, `%w.df`, `%w.de`, with `c`, `-` and `0`, and `%ws`. The rules,
  decoded from Stata 19 at every width from 5 to 20: `%w.0g` fits the number in w - 1 places (one for the sign),
  fixed if its integer part fits and it shows at least as many significant digits as e notation would (which has
  max(2, w - 6) digits, one fewer with a 3-digit exponent), trailing zeros and a leading zero dropped; `%w.dg` keeps
  d digits but never rounds an integer part; a positive number in e notation keeps the sign's space even past the
  width; `%f` too wide falls back to e notation unless that is wider still; g and e round an exact half away from
  zero while f rounds it to even, as Stata's do. `display` without a format is `%10.0g` without the padding.
- **ereturn list, return list** print as Stata's, line for line (the scalars at `%18.0g`); what Stata also lists
  and RCraft does not keep (the optimizer's settings, e(ilog), e(V_modelbased), ...) is named in a note.
- **Tests** (`engine/tests/stata_returned.rs`, 7 cases, and an e() case each in the xtreg, ivregress, stcox and
  anova tests): every value against Stata 19's list, to 12 digits for least squares and 7 to 9 for the likelihood
  models; the 176 widths of Stata's `%g` grid and 66 other formats against the strings Stata printed; display and
  the lists against Stata's lines; the refusals.
- **Not done** (most of it since done; see the next section): `estimates store`/`restore` and
  `esttab`/`outreg2` (Phase 7); macros such as `` `e(cmd)' `` (Phase 2C).

## P2B.6 leftovers: more r(), scalars, matrix list, Stata's e(b), 2026-10-31

Each checked against Stata 19 on the test data (probes kept outside the repository).

- **r().** The one-sample, paired and unpaired `ttest` (each in Stata's order: the paired test on the rows with
  both, the unpaired on each variable's own rows, `sd_diff` for the paired one); `correlate, covariance` (N, cov_12,
  Var_2, Var_1, 0 for one variable) and weighted `correlate` (sum_w with aweights, fweights counted); `pwcorr`
  (the first pair's N and rho, the matrices Nobs, C and with `sig` sig; weighted too); `tab1` and `tab2` (the last
  table's); weighted `tabulate` (fweights and aweights); `describe` and `describe, short` (k, N). `quietly tab1`,
  `tab2` and `pwcorr` now run, leaving r(). `r(width)` and `r(changed)` after describe are refused with the reason
  (Stata's storage types and its record of saving, which R has neither of).
- **scalar.** `scalar [define] name = exp`, `scalar list`/`dir` (newest first at `%10.0g`; a redefinition keeps
  its place unless it changes between number and text, which makes it the newest), `scalar drop` and `_all`;
  a variable in the expression is its first row; `scalar(name)` and a bare scalar name in any expression. A
  variable comes before a scalar of the same name, as in Stata: the translator knows only the variables the do-file
  made, so before a command that reads a scalar by its bare name R checks the data's columns and stops with that
  reason if one has the name. `scalar` and `matrix list` leave r() alone (checked).
- **e(b) and e(V) as Stata keeps them** (`statatr/src/data_r/matrices.rs`, `stata_eb()`): the base level of each
  factor as a zero column (`0b.g`, `2b.cat3` in its place among the levels, `0b.g#co.x`), `o.` for a variable
  xtreg's fe drops and for lm's aliased ones, mlogit's base outcome as `o.` zeros in its place, the equation names
  (`yb:` for logit, probit, poisson, nbreg, ologit and oprobit; `y:` and `sigma_u:_cons` for xtreg's mle;
  `/:cut1`, `/:lnalpha`), each equation's constant last and the terms in the order written (R puts interactions
  after the main effects). Two factors interacted keep R's columns and `matrix list` says why.
- **matrix list** of e(b), e(V), e(R) and r(C), r(Nobs), r(sig), in Stata's layout, decoded from Stata 19: each
  column as wide as its widest entry and 2, a line of equation names and one of prefixes (`0b.`, `1.g#`) set one
  place right, names past 12 characters shortened (`averyveryv~e`), the lower half of a symmetric matrix, blocks of
  as many columns as fit in 78 (Stata's linesize 80); `format()`, `nohalf`, `noheader`, `noblank`, `title()`.
  Refused after anova and margins, whose columns are not rebuilt yet, and for matrices a do-file makes itself.
- **e() after `xtreg, mle`** (Stata's e(p) there is the test of sigma_u = 0 on 1 degree of freedom, not halved)
  and **`xtreg, pa`** with the exchangeable correlation (cmd xtgee, cmd2 xtreg, e(R) the working correlation over
  the longest panel; e(dif), the last change in b, matches Stata's, as the loop stops where Stata's does).
- **iweights:** e() after regress (N the sum of the weights rounded down, the root MSE and r2_a dividing by the
  sum itself, as decoded) and the likelihood models (N the rows).
- **Tests:** `engine/tests/stata_returned.rs` (r() of the other commands, matrix list line for line, scalars line
  for line, iweights), and a matrix list case each in the xtreg (with `o.z`), ivregress and stcox tests, e() after
  xtreg mle and pa.
- **Found:** the likelihood models' 8th digit can differ from Stata's, because Stata stops its Newton-Raphson
  iterations at its tolerance, about 1e-7 from the optimum, where R goes on (reltol 1e-14). matrix list shows 8
  digits, so for those models its numbers are checked to 5e-6 and its layout word for word; matching the last
  digit would mean taking Stata's own iteration path (its starting values and stopping rules).
- **Not done:** e() after anova with error terms or nesting, ivregress gmm with vce()/wmatrix()/igmm and xtreg pa
  with corr(independent); e(b) after anova and margins; margins' other r() (r(b), r(table)); matrices a do-file makes
  (`matrix define`, `matrix A = e(b)`).

## P2B.12 Tab completion in Stata mode, part one: the completion, 2026-10-31

`statatr/src/complete.rs`: `complete(line, &Context) -> Option<Completion>` and `apply()`, the Stata line's
counterpart of `rcodegen::tab` (same `from` and `options`, the same behaviour: complete as far as the choices
agree, the choices listed when there are several). It reads where the word being typed sits, as Stata reads the
line:

- command position, after `by ...:`, `bysort ...:`, `quietly`, `capture`, `noisily` (and `qui:` and the like):
  the commands the translator dispatches and those prefixes (`reg` gives `regress`);
- a command's subcommand (`graph bar`, `import delimited`, `merge 1:1`, `reshape long`, `estat hettest`,
  `scalar list`, `matrix list`, `label values`);
- after the comma (outside parentheses and quotes): that command's options, `level(` with its parenthesis when
  it takes an argument; inside `vce(` its kinds, then a variable; inside any other option a variable;
- after `using`, the first word of `use`/`save`/`saveold`, and inside quotes: file names (folders end in `/`);
- after a backtick a local's name with its closing quote, after `$` or `${` a global's; after `label values var`,
  `label define`, `label list`, `label drop` a value label's;
- anywhere else the variables of the data in memory, in their order, with Stata's rules: a unique abbreviation
  is the name, `inc*`, `?ge` and `inco~` (one variable only) are written out as the variables they match,
  `id-age` as the variables from id to age and `sex-` offers those after sex; `if` and `in` are offered after a
  varlist, and `i.` before a factor's name is passed over.

The caller says what the session holds (variables, locals, globals, value labels, a folder's entries), so the
module never starts R. **Tests** (in the module): the cases above; every command offered is one the translator
dispatches and every subcommand one it takes; every option offered is one it takes, by translating a line that
uses each (which found that `graph bar` and `histogram` do not take `by()` yet).

**Not done** (since done, see P2B.7): wiring it into the Command window and the Script Editor, which waits for the Stata mode switch
itself (P2B.7's two-pane view brings it): the Tab key in Stata mode calls `complete` with the data's variables
(the data viewer has them), the macros the Stata session has defined and the working folder's files.

## P2B.7 Stata mode: the switch, the two-pane view, Tab, 2026-10-31

- **The switch.** A **Stata** button on the Command and Script panes (one switch per session, kept when another
  session is shown). In Stata mode the Command line says "Type Stata and press Enter", an untouched script starts
  over as Stata (`* Type Stata here.`), and switching back restores R's.
- **A typed line** goes through `statatr::live::Live` (new): the session's Stata so far, so a line means what it
  does after the ones before it (the data in memory, the model that left e(), the scalars). It is translated, and
  what runs in R is its R with the Stata as a comment above and the notes below; the helpers R does not have yet go
  first, once, between two marker lines. Results folds them to one line (`# RCraft's Stata helpers, defined once:
  stata_summarize (104 lines, in the session script)`); History, the log and the session script keep them whole, so
  the script still runs in plain R. If R does not take the line the translator does not move on; a restart of R
  starts it over. A block or a line the reader cannot take is answered (its reason in Results) but not kept, so it
  cannot swallow the lines after it. The data a `use` or `import` brings in becomes the data shown (Variables, the
  status bar), staying on Results.
- **Page Up / Page Down** in Stata mode bring back the Stata typed, not the R it became; History shows a Stata line
  as its Stata and a click puts that line back.
- **The Script Editor** in Stata mode is two panes: the Stata on the left, and on the right each line's R under its
  Stata (with its line number), its notes in the warning colour and the reason a line was not translated in the
  error colour. The block of the line the cursor is on is marked; a click on a block selects its Stata (all of its
  lines, `///` included). **Run all** and Ctrl+Enter run the Stata through the same translator as the Command line;
  **Copy R** copies the whole script's R, helpers first.
- **Tab** in Stata mode completes from `statatr::complete` (P2B.12): the variables of the data in memory (as R last
  sent them), the macros and value labels the session's Stata has defined, and the files of R's working folder.
- Found by the new app test and fixed in the translator: `display r(mean)` after `summarize` alone called
  `stata_rget` without defining it (a model's e() brought the helper before); whatever leaves r() now brings it.
- Picture mode: `--stata` (Stata mode on, the `--demo` lines run as Stata) and `--script FILE`.
- **Tests:** `statatr::live` (lines in context, helpers once, what is not kept, macros and labels), `stata_mode`
  (the folding, a command's lines found and selected, Page Up, Tab from the session and the folder), and an app
  test with R: the switch, `use` of a .dta, `summarize`, `display r(mean)` printing Stata's 4.3333333, the helpers
  sent once, an untranslated line's reason, recall, Tab on the data's variables, a restart.
- **Not done** (done since, see the next section): a click on the R side of the Command window's Results; Stata-style
  output tables; the Script Editor's own Tab completion.

## P2B.7 finished: Stata in Results, Tab in the Script Editor, Stata's tables, 2026-10-31

- **Results and History.** A Stata line is marked in its R as `# Stata: summarize y` (`data_r::STATA_COMMENT`), so
  a Stata-mode command is told from R. Results shows it as Stata does, `. summarize y`, with its R under it (dim),
  its notes in the warning colour and the reason a line was not translated in the error colour; in Stata mode a
  click on the Stata puts it back on the Command line. History shows `. summarize y` too.
- **Tab in the Script Editor** (Stata mode): the line the cursor is on, up to the cursor, completed as on the
  Command line, the choices shown above the editor while the script is unchanged.
- **Stata's own tables** (`statatr/src/data_r/tables.rs`), read off Stata 19's output (never its files):
  - `summarize`: Obs with commas, the rest at `%9.0g`, a rule after every five variables, names past 12 shortened,
    a string variable as Obs 0 (weights and `detail` keep R's layout and say so);
  - `tabulate` one-way: the labels' column as wide as the widest label (11 at least), a long variable name split
    over two lines, percentages at `%.2f`, commas past 999; two-way: rows' labels cut at 21, columns of 10 a space
    apart in blocks that fit 80 characters (wrapped as Stata does), the column variable centred over its columns,
    `chi2` as Stata's line; `tab1`/`tab2` print Stata's `-> tabulation of` headings;
  - `regress`: the ANOVA header (SS and MS at `%11.0g`, Root MSE at `%10.5g`), the robust and clustered headers,
    `(Std. err. adjusted for 8 clusters in cl)`, the coefficient table (b, se and the interval at `%9.0g`, t at
    `%8.2f`, P at `%5.3f`) and factors as Stata groups them: one level inline (`1.g`), several, or any
    interaction with a continuous variable, under the factor's name.
  - Kept in R's layout, with a note, where the shape was not checked against Stata: string variables in tabulate
    (Stata sizes them by their storage type, which R does not keep), percentages (`row`, `column`, `cell`),
    analytic and importance weights, factors with value labels, omitted variables, two factors interacted.
- **Found and fixed on the way:** `regress y x i.g, noconstant` fitted a different model (R gives the first factor
  every level without the constant; Stata still leaves its base out) and is refused with that reason now, in every
  model; the Wald F solved e(V) unscaled, which R called singular when coefficients differ by many orders of size
  (`regress y big tiny`), and now solves it on the errors' scale; a one-coefficient model lost its name.
- **Tests:** the 26 tables of two Stata 19 runs compared character for character (`stata_returned.rs`), the
  refusal, the Results parts, Tab in the script; the old tests that read R's layout now read Stata's.

## P2B.8 The differences guide, 2026-10-31

- `statatr/src/differences.rs` (new): a page for every command Stata mode translates, and a general page on what
  differs everywhere (Stata's missing value above every number and the conditions written out for it, float
  against double, string widths, sort ties, random numbers, returned values, what is not translated). Each point
  says what Stata does, what R does by itself and what the translation does; the pages are written from the
  translator's own rules (each one checked against its code: destring leaves a variable with text as it is,
  tostring takes whole numbers, merge makes `_merge` with Stata's labels and order, reshape sorts by i and j, ...),
  never from Stata's files. `guide()` finds a page by a command or its abbreviation (`probit` shares `logit`'s,
  `tab2` `tabulate`'s, `scatter` `twoway`'s); `for_line()` by a Stata line, past `quietly`, `capture` and `by:`.
- **In the app:** a note or a reason under a Stata line, in Results and in the Script Editor's R pane, opens the
  Help window on its command's page (the general page for a command with none); Help > *Stata and R: what
  differs…* opens the general page, which lists every command's. Back and Forward work as for R's pages. Picture
  mode: `--stata-guide LINE`.
- **Tests:** every command Tab offers (the translator's) has a page with points, each page once; a line finds its
  page through prefixes and abbreviations; a note's click opens the right page.

## P2B.10 Checked against Stata: the corpus and Verified, 2026-10-31

- **The corpus** (`engine/tests/stata_corpus/`): made-up data (`corpus.csv`, 60 rows from a fixed generator, a
  random effect per panel) and five do-files of our own (`descriptives`, `regress`, `likelihood`, `panel`,
  `data`). Each names the commands it checks (`* commands:`) and its tolerance (`* tolerance:`: 1e-8, or 1e-6 for
  the likelihood models, which Stata stops iterating at its tolerance), and prints each result as
  `display "@ key " %24.17g ...`.
- **Recording:** `cargo xtask stata-record` runs each do-file on a licensed Stata (`RCRAFT_STATA`, else its usual
  places) in batch mode, keeps only the `@` lines and Stata's version in `NAME.recorded` (committed), and deletes
  Stata's log, which carries its licence banner (`*.log` is ignored there too). Without a Stata it says so; the
  tests run on what is committed.
- **The test** (`engine/tests/stata_corpus.rs`): every line of every do-file must translate; each runs in R and
  every recorded result is compared with Stata 19's: 119 results, all matching (summarize with detail and weights,
  tabulate and chi2, correlate, pwcorr, the ttests, regress plain, robust, clustered, with factors, interactions,
  fweights and aweights, test, lincom, predict, margins, logit, probit, poisson, ologit, oprobit, mlogit,
  xtreg fe/re/be/mle, ivregress 2sls/liml, anova, stcox, generate, replace, recode, sort, gsort, collapse).
- **Verified** (`statatr::verified`): each command and the corpus files that check it; a test holds it equal to the
  files' `commands:` lines, so a command is Verified only while a file that uses it passes. The differences guide
  says so on each page ("Verified: checked against Stata 19's own results").
- **Found and fixed on the way:** `import delimited` stores a column with decimals as float in Stata (decided by
  probing: whole numbers stay byte/int/long, `asdouble` keeps double), which the translation did not, so every
  result from an imported CSV could differ in the 7th digit: it now rounds them as Stata stores them (`asdouble`,
  `asfloat` accepted); `collapse (mean) y (sd) sy = y` took the sd of the new mean, as dplyr's summarise reads a
  column it has just made: each result is made under a name of its own first; `xtreg, mle` stopped with R's
  "singular" when sigma_u is at its boundary, 0: it now says so (Stata reports the pooled fit there, which is not
  matched yet).
- **Not done:** collapse of a float variable (Stata stores the mean as a float; the corpus uses `asdouble`);
  `xtreg, mle` at sigma_u = 0; the commands and options not in the corpus stay checked by their own tests.

## P2B.11 ssc install, 2026-10-31

- `ssc install name [, replace all]` translates to nothing in R, with a note, and asks the window for the package
  (`Translation::ssc`, `Step::ssc`); `ssc` with another subcommand and `net` are refused with the reason (Stata's
  add-ons are out of scope).
- In Stata mode the window then offers the R package of the same name through `rc install name` (the Packages
  window's list to say yes to, nothing installed before) when CRAN has one, and says in Results that there is none
  otherwise (`rc search name` to look); without the package list it says so. `ssc` is in Tab's commands and the
  differences guide.
- **Tests:** the translation and refusals; in the app, `ssc install fixest` opening its install list and
  `ssc install estout` saying there is no R package of that name.

## P2C part one: Run my do-file, 2026-10-31

- **`statatr::dofile`** (new): a do-file gone down the way Stata goes down it. `Run` replaces each line's macros
  (`` `name' ``, `$name`, `${name}`, nested `` `x`i'' `` inside first, `` `=exp' ``, `` `: fn' ``, `` `++i' ``, the
  escapes `` \` `` and `\$`), sets `local`/`global` (text, `= exp`, `: function`, `++`/`--`), `macro drop/shift`,
  `tempvar`/`tempname`/`tempfile`, runs `foreach` (in, of local/global/numlist/newlist/varlist), `forvalues`,
  `while`, `if`/`else if`/`else`, `capture {`/`quietly {`, `continue [, break]` and `exit`, follows `do`, `run` and
  `include` into other files (through a `Files` reader the caller gives; `do` gets its own locals and its
  arguments, `include` shares them), keeps track of `cd`, and puts the folders the user says are elsewhere where
  they are (`Options::map`). Every command it reaches goes to the translator in order (`data_r::Translator`, the
  translator a command at a time, which `translate` now wraps). Each line keeps where it came from (`Origin`:
  file, line, the text as written, the loop rounds it is in, what happened).
- **Values only R knows** (`local m = r(mean)`, `if r(N) > 0 {`, `while ...`, `local s : display %9.2f r(sd)`):
  `Run::next_event` stops with `Event::Ask`, the R that prints the value (the translator's own `display`, so
  Stata's rules: a number in %18.0g, as Stata keeps it in a macro); the caller runs it after the lines before it
  and gives back what R printed. `dry_run` gives no values: such lines say they are known only when the do-file
  runs. Values worked out here without R (numbers, text, the operators, abs/floor/ceil/int/round/mod/min/max,
  sqrt/exp/ln/log10, strlen/upper/lower/substr/word/wordcount/strtrim/string, `_pi`, `c(pi)`) are only the plain
  ones; everything else goes to R, so nothing is guessed.
- **Rules observed in Stata 19** with made-up values (probes kept out of the repository): `local x = exp` keeps
  %18.0g without its spaces (`.3333333333333333`, `1.00000000000e+20`, `1428571428.571429`); `numlist` keeps
  %15.0g and computes start + i*step (`0.7(0.1)1.0` is .7 .8 .9 1); `forvalues` adds the step each round and keeps
  %18.0g (`0(.1)1` ends at `.9999999999999999`); `string()` is %9.0g; an undefined macro is empty; `$g_x` is the
  global `g_x`; a macro's text is looked at again (an escaped `` \`i' `` kept in one is replaced where it is used);
  the loop's macro is gone after `foreach`; `local x "a" "b"` keeps `a" "b`; `: list A - B` takes the last copy of
  each element, `&` matches copies one for one, `==` is the same order and `===` the same elements; `: display`
  keeps its leading spaces; `tempvar` names are `__000000`, `__000001`, ...
- **The scan** (P2C.1, `dofile::scan`): lines that translate cleanly, with notes, not at all (each reason and how
  often), that RCraft runs itself (macros, loops) and that need a value from R; the commands used; the R packages;
  `ssc install`; every file and folder named (reads, writes, runs, `cd`), found or not, or made by the do-file
  first; and what a run on R does not do as Stata does (P2C.10: random numbers, Mata, programs, plugins,
  interactive commands).
- **The window** (`ui-egui/src/dofile_window.rs`): File > *Run do-file…*, or a `.do` file dropped on the window.
  It shows where the do-file runs (its own folder, *Change…*), the files not found on this machine (an absolute
  folder can be pointed to with *Find its folder…*; the do-file is never changed), the scan, and every line with
  its R beside it; a click shows the line in full: as written, the loop round, its R, its notes, what R printed,
  and a link to the differences guide. *Run on R* goes to the folder, then sends each line in turn (with the
  helpers it needs first, folded in Results) and asks R the values only it knows; at the first problem (a line
  not translated, or R stopping with an error) it asks: *Stop here*, *Go on*, *Go on without asking*. Afterwards
  *Save as R script…*, *Save as Quarto…* and *Save log…* (the run line by line with what R printed and each
  note, P2C.6 and P2C.7); the data the run left are shown. Picture mode: `--dofile FILE [--dofile-run]`.
- **Stata mode's typed lines** go through the same layer (without R), so `local`, `global` and loops work in the
  Command window and the Script Editor too.
- **Translator:** `cd` (setwd), `pwd`, `log`/`cmdlog`, `browse`/`edit`, `pause`, `more`, `discard` translated (the
  last ones to nothing, with a note); a `\` in a file name is read as `/`; Tab offers the new commands, and the
  differences guide has pages for `cd`, `do` (`run`, `include`), `exit`, macros (`local` and the rest) and loops
  (`foreach`, `forvalues`, `while`, `continue`).
- **The corpus:** `control.do` (with `control_setup.doh`, which it includes) checks macros, loops, `while`, `if` on
  a stored result, `numlist`, `: word count`, `tempvar` and a macro's 16 digits against Stata 19: 19 more results,
  138 in all, all matching. The corpus test now runs each do-file as the window does (line by line, R asked for
  the values only it knows). `include`, `local`, `global`, `foreach`, `forvalues`, `while` and `tempvar` are
  Verified.
- **Found and fixed on the way:** `stata_fmt` (the R helper behind `display` and every value R works out) took a
  17-digit form ending in 5 for an exact half and rounded it up, by a nudge large enough to change a 16th digit:
  `%18.0g` of .14761666666666665 gave `.1476166666666668`, Stata `.1476166666666666`. Whether a value is a half is
  now worked out from the double itself (in whole numbers, from its bits), and the nudge is one unit in the last
  place; the format grids still match and 22 macro values from Stata 19 are added to them.
- **Tests:** the expander, the formats, the static values, numlists and forvalues, files and paths, the scan, the
  three outputs (`statatr`, 87 in all); the corpus in R; in the app, a do-file run with a value from R, a loop,
  a line not translated and an R error each stopping the run to ask, the log, and the data shown.
- **Not done:** `foreach ... of varlist` with wildcards or ranges and `levelsof` (they need the data's variable
  names, which R would have to give the translator as it gives values); macro functions that read the data
  (`: variable label`, `: type`, ...); programs (`program define`), `input` blocks, Mata; in the Command window, a
  value only R knows (`local m = r(mean)` typed there says it is known only when the do-file runs); the share of
  lines translated per tier (`docs/coverage.md`, P2C.8); StatCraft's probe do-files and public replication
  packages as inputs (P2C.9).

## P2C part two: the data's names, levelsof, values from R in Stata mode, 2026-10-31

- **The data's variable names from R** (`statatr::dofile`, `Need::Names`): where a varlist has a pattern, a range or
  `_all` (`foreach v of varlist x1-x10`, `regress y x*`, `keep inc*`), the runner asks R for the names of the data
  in memory and makes the varlist Stata's way before the translator sees it (`dofile/varlist.rs`): `x*`, `x?` and
  `x~` the names that match, `a-c` the names between in the data's order, `_all` every one, an abbreviation the
  one name it stands for; factor-variable operators, `#`, parentheses and `=` are kept. Stata's errors as Stata 19
  gave them: `x ambiguous abbreviation`, `variable zz* not found`, `x~ ambiguous abbreviation`. The names are
  asked once and kept while the lines leave them alone (summarize, the models, display, ...), and asked again after
  one that may change them. The models, which refused `x*` and `x1-x3` before, now take them. A plain name is
  still taken as written (an abbreviation without a pattern is not looked up; R says so if the data has no such
  variable). A dry run has no names: such a loop is known only when the do-file runs.
- **`levelsof`** (translator): the sorted distinct values, printed and left in `r(levels)`, `r(r)` and `r(N)` as
  Stata 19 does (numbers in %18.0g, so a float 0.1 is `.1000000014901161`; text in compound quotes unless `clean`;
  `""` first and `.` last with `missing`; `separate()`); `local()` is set from `r(levels)` once the line has run in
  R, so `levelsof g, local(gs)` then `foreach k of local gs` loops over the data's own values.
- **`unab name : varlist`**, and a stored result in a macro (`` `r(levels)' ``, `` `e(cmd)' ``; Stata 19 writes a
  number there in %18.0g, as `= r()` does).
- **Macro functions that read the data:** `: variable label x` and `: label (x) #` from the labels R keeps;
  `: label set #` from the do-file's own `label define` (the value itself when the set has no text for it, as in
  Stata). `: type`, `: format` and `: value label` are refused with the reason: R does not keep Stata's storage
  types, display formats or the names of value-label sets.
- **Values from R in Stata mode:** a typed line that needs one (`local m = r(mean)`, a loop over `x1-x5`) is worked
  out by R out of sight, then the line is translated with the answer (`live::Step::ask`, `Live::give`); each later
  step runs the Stata so far again with the answers given, in order. Where lines of the same text would have to
  run first (several lines run at once from the Script Editor), the value is not asked for and the line says so.
  The Script Editor's two-pane view goes through the do-file layer too, so its loops show round by round.
- **The corpus:** `aware.do` checks a range in a loop and a model, `unab`, `levelsof` with `local()`, `if` and
  `r(levels)` in a macro, and a variable label read back against Stata 19: 14 more results, 152 in all, all
  matching. `label`, `levelsof` and `unab` are Verified.
- **Tests:** the varlist rules from Stata 19; a run whose made-up R gives names, levels and a label (names asked
  once, then again after `generate`); Live asking and replaying; in the app, a typed `local m = r(mean)` and a
  typed loop over `x-y` with a real R, and the window's do-file with a range.
- **Not done:** abbreviations in plain varlists and in expressions (`generate z = pri*2`), varlists inside options
  (`by()`, `vce(cluster ...)`), patterns inside interactions (`c.x*#i.g`), `ds`; `: type`, `: format`,
  `: value label`; programs, `input` blocks, Mata; the coverage per tier (P2C.8) and StatCraft's probe do-files as
  inputs (P2C.9).

## P2C.8 and P2C.9: coverage by tier, StatCraft's probes run on R, 2026-11-01

- **Coverage by tier (P2C.8):** `statatr::coverage` puts each command in a tier (1 data management and control
  flow, 2 core statistics, 3 panel, survival, survey, IV and the other models, 4 graphs, 5 programs and SSC
  commands) and sums dry runs. `tests/many_dofiles.rs` runs it over a folder (`RCRAFT_DOFILES_DIR`) and writes the
  table with `RCRAFT_COVERAGE_TABLE`. The table for StatCraft's 381 probe do-files is in `docs/stata-coverage.md`
  (`docs/coverage.md` is the menus' generated inventory, so it was not used).
- **Probes run on R (P2C.9):** `engine/tests/stata_probes.rs` (`RCRAFT_STATA_PROBES` = StatCraft's private corpus)
  runs each probe line by line in a fresh R, as the do-file window does. Each top-level `display`, `return list`
  and `ereturn list` before the probe's first R error is paired with the line Stata echoed with the same text
  and compared to 8 significant digits, with `%21x` read back to the double. Stata's logs stay in the corpus;
  only counts are written down.
- **What the probes found, and what was done about it:**
  - Missing features, now translated:
    - `set obs` (with no data, and to add rows);
    - one-line `if exp command` and `else command`;
    - `tokenize` (Stata 19's rules: every numbered local cleared, outer quotes off, `==` `>=` `<=` `!=` `~=` kept
      whole);
    - `display %21x`;
    - `preserve` / `restore`;
    - `expand` (copies after all the rows, in order, as Stata 19 puts them);
    - `_rc` after `capture` (0 or missing: R's errors are not Stata's numbers).
  - Silent wrong results, now refused with the reason:
    - `quietly display` and a plain `capture` printed (now Stata's rule: the innermost of quietly, noisily and
      capture wins);
    - a line that uses a variable or scalar whose `generate` / `replace` / `egen` / `predict` / `scalar` line was not
      translated ran on the old values or none;
    - a line after a refused line that replaces the data or changes its rows (`frame`, `merge`, `drop`, `sort`,
      ...) used the data from before it;
    - a `local` whose new value could not be worked out kept its old value;
    - macros a refused `gettoken` or `levelsof, local()` would set, `macval()`, class members and Stata's own `S_`
      globals expanded to nothing;
    - comparisons with `.a`–`.z` (all NA in R) gave the wrong answer;
    - time-series operators (`L.x`) reached R as they are;
    - `display` of a value from the data with no data known.
  - Lost helpers: a helper sent together with a line R could not read was never defined, though it counted as
    sent; later lines then failed with `could not find function`. The do-file window, typed Stata lines (`Live`,
    now with `Step::helpers` and `Live::unsend`) and the harness now send it again.
- **Numbers** (StatCraft's 385 recorded probes): 192 of 196 compared lines match Stata 19 to 8 significant digits
  and 1 more to 6. R errors on translated lines fell from 1,494 to 399, most of them the probes' setting (svglite
  not installed, files other probes wrote). The dry share translated is 31.8%; most of the rest follows from the
  random-number functions. `docs/stata-coverage.md` has the tables.
- **The corpus:** `made.do` (16 results, recorded with `cargo xtask stata-record`) checks `set obs`, `expand`,
  `preserve` / `restore`, `tokenize`, one-line `if` / `else`, `_rc` and `quietly` against Stata 19; all match, so
  expand, preserve, restore, set and tokenize are Verified. The corpus now checks 168 results.
- **Not done:**
  - Stata's random numbers. `runiform()`, `rnormal()` and `rgamma()` make most of the probes' data. StatCraft has
    them exactly (mt64, checked bit for bit against Stata 19), but R would need them from RCraft's side.
  - `gettoken`, programs, the date functions (`clock()`, `monthly()`, ...), `_natscale`, `bsample`, `file`,
    lasso, xtregar, and time-series operators.
  - `capture noisily` does not print R's error message.
  - Public replication packages as inputs.

## P2C.11: Stata's own random draws on R, 2026-11-01

- **What works:** `set seed`, `runiform()` (and `uniform()`), `runiform(a, b)`, `rnormal()`, `rnormal(m)`,
  `rnormal(m, s)` and `rgamma(a, b)` give Stata 19's own numbers when RCraft runs Stata on R. Where they work:
  `generate`, `replace`, `expand`, `keep`/`drop if`, `display`, `scalar`, macro values and `if` blocks, with
  `if`, `in` and `by`. A typed line in the Command window goes on from the last draw.
- **How (decided and written up in `plan/stata/random-numbers.md`):**
  - Stata's generator (the 64-bit Mersenne Twister, mt64) and StatCraft's decoding of its normal and gamma
    methods are written in R, in the helper `stata_rng` (`statatr/src/data_r/random.rs`). Each 64-bit word is
    four 16-bit pieces, since R has no unsigned 64-bit integers.
  - The state lives in R, so it follows the do-file across lines, `preserve`, `by` and typed lines, and a
    saved R script reproduces Stata's draws on its own. A Rust generator handing R the numbers through
    `Need`/`Ask` was weighed and not taken: the script would have to carry the numbers, and the runner would
    have to ask R for each line's rows and arguments.
  - A row draws only when the `if`/`in` selects it, one draw a row, in order. A draw that the R would write
    twice (the missing rule of a comparison) is drawn once and named.
- **Observed in Stata 19 SE (made-up data):**
  - Invalid or missing arguments give missing and use no uniform; `rnormal(m, 0)` is m.
  - `cond()`, `&` and `|` draw for every row.
  - `runiform(a, b)` is a + b·u − a·u.
  - A G4PE left-tail point at or below 0 is turned down before the next uniform is drawn. StatCraft draws
    that uniform first, so it differs from Stata there; noted for StatCraft.
  - `uniform()` uses another generator under `version 13`.
  - 240,000 draws (uniforms, normals, six gamma shapes, `runiform(-3, 7)`, `rnormal(10, 3)`, a float) matched
    bit for bit.
- **Refused with the reason:**
  - two draws in one expression, or in both the value and the `if`;
  - a draw in an `if` with `in`;
  - draws in other commands;
  - the other random functions;
  - draws after a line that may draw and was not translated (`bsample`, `bootstrap`, `simulate`, a program,
    `rbinomial()`, ...), until the next `set seed`;
  - draws under `version` 13 or earlier (Stata's older KISS32).
- **The corpus:** `random.do` checks 57 results exactly (tolerance 0), and passes on Windows, macOS and
  Ubuntu. `runiform`, `rnormal` and `rgamma` are Verified (a new `functions:` header and
  `verified::VERIFIED_FUNCTIONS`); `random.do` is added to `expand`, `generate`, `replace` and `set`.
- **Numbers (StatCraft's probes):**
  - The dry share translated rose from 31.8% to 68.9%. The lines refused only because a line before them was
    fell from 85,646 to 25,350.
  - On R: 205 of 210 compared lines match Stata 19 to 8 digits (192 of 196 before), and 1 more to 6. 114,916
    of 179,330 lines are translated (55,454 before).
- **Not done:**
  - `runiformint()`, `rbinomial()` and the others; `c(rngstate)`; KISS32.
  - `invnormal()` and `normal()`: `invnormal(runiform())` is how many probes make data.
  - Found on the way, not fixed here: a model the translator refuses (`qreg`) does not clear `e()`, so a later
    `estat ic` runs on the model before it (probe 315: `_rc` 0 where Stata gives 321).
  - Found on the way, not fixed here: `generate x = y if _N > 5` (an `if` with the same value for every row)
    fails in R's `dplyr::if_else`.

## P2C.12: closer to Stata on the probes, 2026-11-01

- **No compared line differs for a reason other than one `_natscale` residue.** The four lines that differed
  after P2C.11 now match or are refused with the reason:
  - `capture` gives Stata's `_rc` for a name that is not there (111) and a variable already defined (110), and
    `capture noisily` prints Stata's message (`a not found`). Any other R error keeps `_rc` missing and, under
    `noisily`, R's message (helper `stata_captured`).
  - A refused line that may fit a model (`qreg`, a refused `regress`, `estimates restore`) hides the model
    before it: `estat`, `predict`, `test`, `lincom` and `margins` are refused with the reason. Commands that
    only read the model (`testparm`, `nlcom`, `estimates store`, `margins` without `post`, ...) leave it.
    Probe 444's `estat vce` after a refused `regress ... ibn.g` had shown the earlier model's matrix.
  - A refused add-on (a command outside the tiers) or do-file program makes the globals the do-file has not
    set unknown; probe 900's `$mtable_cspec` is refused, not read as empty.
- **More lines compared:**
  - `invnormal()` and `normal()` are R's `qnorm` and `pnorm`, bit for bit against Stata on 20,000 values.
    `random.do` checks them (77 results, tolerance 0) and they are Verified.
  - The storage types' `c()` constants (`c(maxlong)`, `c(epsdouble)`, `c(pi)`, ...), read off Stata 19 in
    `%21x`, are translated.
  - `_natscale` uses StatCraft's decoded rule. It is worked out when the line is translated, and R gets
    exact hex doubles.
  - `generate x = y if _N > 5`, an `if` with one value for the data, no longer fails in `dplyr::if_else`.
- **The probe run** (`stata_probes.rs`) now also lists the lines Stata printed that were refused, by reason.
- **Numbers:**
  - 5,508 of 5,626 compared lines match Stata 19 to 8 digits (205 of 210 before) and 5 more to 6. The 113
    that differ are `_natscale` cases with n = 10 or 15, where Stata's r(min) is 2^-55 rather than 0.
  - The dry share translated is 76.1% (68.9% before).
  - 126,618 of 179,362 lines are translated on R (114,916 before).
- **Observations:** `plan/stata/errors-and-normal.md`.
- **Not done:**
  - `poisson` to 8 digits: Stata takes one Newton step from a start not yet found.
  - `_natscale`'s residue for n = 10 and 15.
  - `normalden()` and `lnnormal()`: R's agree only to about 4e-9 in the tails.
  - Collinear variables in `logit`, `poisson` and `regress` with interacted factors, reached for the first
    time: Stata omits them, R stops.

## P2C.13: collinear regressors, 2026-11-01

- **What works:** `regress`, `logit`, `probit` and `poisson` omit collinear regressors as Stata 19 does:
  - Stata's choice and its 1e-13 tolerance;
  - `note: x omitted because of collinearity.`;
  - a `0  (omitted)` row in `regress`'s table (a factor's level too: `2  |  0  (omitted)`);
  - `o.x` and `2o.g` columns in `e(b)` with zero rows in `e(V)`, and `_b[x]` of 0;
  - `e(rank)` and `e(df_m)` counting only what is estimated.
  The whole `regress y x x3 w v` of probe 288 is Stata's, line for line.
- **How:** the R helper `stata_collinear` runs after the fit. It uses StatCraft's decoding of probes 286 and
  288:
  - columns about their means are swept one at a time;
  - `regress` takes the one with the most left next, so the smallest of a collinear set goes;
  - the likelihood models take them in the order written.
  Where R's choice differs, the model is fitted again without Stata's terms, which are kept in `$omitted`
  with the order as written. R's `glm` checks with a tolerance of `epsilon / 1000`, so a float `x + 10` stayed
  in and the fit failed ("computationally singular"). Identical columns tie, and the earlier is kept, within
  1e-12 (R's cross products can differ in the last bit).
- **The corpus:** `collinear.do` (26 results from Stata 19, tolerance 1e-6 for the likelihood models)
  passes; `regress`, `logit`, `probit` and `poisson` list it.
- **Numbers:** R errors on the probes fell from 461 to 453. The compared lines are unchanged (5,508 of 5,626),
  since the probes that fit collinear models compare no `display` after them.
- **The probe trace** (`RCRAFT_PROBE_TRACE`) now prints each traced line's R and what R printed.
- **Not done:** two factors interacted (`i.g#i.h`), which stop with the reason; collinearity in the other
  models (`nbreg`, `ologit`, `mlogit`, ...); `logit`'s coefficients agree with Stata's to about 7 digits
  (Stata stops at its own tolerance).

## P2C.14: Stata's date and time functions on R, 2026-11-01

- **What works:** dates now work on columns in R, with Stata's numbers and missing values.
  - Strings read against Stata's masks: `date()` (and `daily()`), `clock()`, `weekly()`, `monthly()`,
    `quarterly()`, `halfyearly()` and `yearly()`, with the top year.
  - About 40 functions of numbers: `mdy()`, `yw()`, `ym()`, `yq()`, `yh()`, `dofm()` and the others, `year()`,
    `month()`, `day()`, `dow()`, `doy()`, `week()`, `quarter()`, `mofd()`, `wofd()`, ..., `cofd()`, `dofc()`,
    `hh()`, `mm()`, `ss()`, `hms()`, `dhms()`, `mdyhms()`, and the conversions between hours and
    milliseconds.
  Before this, only a `date()` written out in the do-file, `mdy()` and five parts of a date were translated,
  through R's own dates.
- **How:** the rules of `values/datetime` (StatCraft's decoding, already ported to Rust for dates written
  out) are written again in R as the helper `stata_dates` (`data_r/dates.rs`). Each distinct string is read
  once. An expression now carries the R helpers it calls (`RExpr::helpers`), which the line passes on.
- **Checked:** 13,177 of Stata 19's recorded answers (probes 034 and 035) were put through the R helper, and
  13,172 are the same bit for bit. Four are `wofd()` outside 0100 to 9999, where Stata's numbers are
  arbitrary; RCraft gives missing. `dates.do` in the corpus (23 results on columns, tolerance 0) passes, and
  28 date functions are Verified.
- **Numbers:**
  - The dry share translated rose from 76.1% to 79.1% (tier 1 from 82.0% to 85.4%).
  - On R, 135,462 of 179,362 probe lines are translated (126,618 before), and 5,518 of 5,636 compared lines
    match Stata to 8 digits (5,508 of 5,626 before), with no new difference.
- **Notes:** `plan/stata/dates.md`; a "Dates and times" point in the differences guide.
- **Not done:** the leap-second family (datetime/C: `Clock()`, `Cofd()`, `hhC()`, ...), which is refused with
  the reason.

## P2C.15: two factors interacted, 2026-11-01

- **What works:**
  - In an interaction a bare variable is a factor, as in Stata (`g#h` is `i.g#i.h`). Before this RCraft
    read it as a continuous variable, a different model, without saying so.
  - `i.g#i.h` without its main effects estimates every cell but the all-base one, as Stata does. The pair is
    one factor of its cells (`stata_cells`), the base cell first, so R's coding leaves it out by
    construction. A first coding, with base levels last, leaned on R to find a column collinear. Windows
    CI's `glm` missed it, so it was replaced before landing.
  - `i.g##i.h` and `i.g i.g#i.h` were already R's coding.
  - `e(b)` names every cell as Stata does (`1b.g#0b.h`, `1b.g#1.h`, `2o.g#0b.h`).
  - `regress`'s table shows the cells as `2 1`, or `2#yes` with value labels. Value labels are now shown
    everywhere in that table: a one-level factor with labels is a group, a label past 11 characters is
    shortened, a long first label in a cell goes on its own line, and an empty cell is `0  (empty)`.
  - `set linesize` is kept in R, and `matrix list` wraps at it.
- **Checked:**
  - A do-file of nine regressions run through Stata 19 and RCraft gives identical coefficient tables.
  - Probe 286's `matrix list e(b)` is Stata's, line for line.
  - The factor-variable probes (286, 287, 289, 316) have no R errors left.
  - `interactions.do` in the corpus (16 results) passes; `regress` and `logit` list it.
- **Numbers:** on the probes, 5,519 of 5,637 compared lines match Stata to 8 digits, and the R errors fell from
  453 to 449.
- **Notes:** `plan/stata/interactions.md`, and an "Interactions" point on the regress help page.
- **Not done:** three factors interacted alone (coded the same way, not checked); interactions in the
  likelihood models' tables, still in R's layout although their `e(b)` is Stata's.

## P2C.16 part one: bsample with Stata's own draws, 2026-11-01

- **What works:** `bsample`, `bsample #`, `bsample if`/`in`, and `bsample, cluster(v) idcluster(new)` draw the
  rows Stata 19 draws, in its order, from Stata's generator in R (`stata_bsample` in `data_r/random.rs`,
  StatCraft's decoding), and leave the generator where Stata leaves it.
- **Observed here (280 samples, each with the next uniform):**
  - with `if`, every row of the data still draws, and the rows selected take the first draws;
  - with `idcluster()`, each cluster's copies stay together.
  All 280 match.
- **Corpus:** `bsample.do` (18 results: rows, order, statistics after it, a `preserve`/`restore` loop, the
  draws after) passes; `bsample` is Verified.
- **Numbers:**
  - The dry share translated rose from 79.1% to 83.1%, and `bsample` is gone from the first causes.
  - On R, 158,383 of 188,989 probe lines are translated (135,471 before), as StatCraft's resampling probes
    now run their loops.
  - 5,523 of 5,641 compared lines match Stata to 8 digits, with no new difference.
- **Not done:**
  - `bsample` with `if`/`in` and a size or `cluster()` (not observed), `strata()` and `weight()`, all refused
    with the reason;
  - the `bootstrap` prefix, which is P2C.16 part two.

## P2C.16 part two: the bootstrap prefix, 2026-11-01

- **What works:** `bootstrap [exp_list] [, reps() seed() nodots cluster() level()]: command`, with `_b` after
  `regress`, `logit` or `probit`, or `r()`/`e()` values, named or not (`_bs_1`, ...). The command is translated
  once, quietly, and run in R:
  - once on the data, for the observed values;
  - then once per replication on `stata_bsample` of its estimation sample, using Stata's own draws.
  `stata_bs_report` prints Stata's messages, header and table and leaves `e()`: b, V, N, N_reps, b_bs,
  bias, se, ci_normal, ci_percentile, ci_bc and the model's scalars.
- **Observed in Stata 19 here:**
  - only the estimation sample is resampled (the next draw after three replications on 37 of 40 rows is
    the 223rd);
  - every replicate is kept as a float, `_b`'s too, and observed values are not.
  Seven `bootstrap` runs give Stata's output line for line. The exception is `logit`, whose own
  coefficients agree to about 7 digits.
- **Also:** `el(e(V), i, j)` and `el(r(...), i, j)`, through the same check as `e()`.
- **Corpus:** `bootstrap.do` (9 results, tolerance 1e-9) passes; `bootstrap` is Verified, with a guide page.
- **Numbers:** 5,524 of 5,642 compared probe lines match Stata to 8 digits; probe 180 compares and matches
  one more line.
- **Not done:** `strata()`, `saving()`, `bca`, `_b` after other models, `estat bootstrap`, `predict` after it,
  and `vce(bootstrap)`, all refused with the reason.

## P2C.11 continued: the other random functions, 2026-11-01

- **What works:** these give Stata 19's own draws on R:
  - `rexponential()`, `rlogistic()` (one argument is the scale), `rweibull()`, `rweibullph()`, `rcauchy()`,
    `rlaplace()`, `rpareto()`, `rgeometric()`;
  - `rigaussian()`, `rchi2()`, `runiformint()`;
  - `rbinomial()`, `rnbinomial()` and `rhypergeometric()` in the ranges whose method is known; past them R
    stops with the reason.
  The code is StatCraft's decoding, written in R beside the generator. Arguments outside Stata's ranges
  give missing and draw nothing.
- **Observed here:** `runiformint(a, a)` takes no draw.
- **Checked:** 34 cases of 1,500 draws each, every one followed by the next uniform, match bit for bit;
  `variates.do` in the corpus (19 results, tolerance 0) passes. Fourteen functions are Verified.
- **Numbers:** 5,526 of 5,644 compared probe lines match Stata to 8 digits; probe 182 (`runiformint`)
  matches 2 of 2. The dry share is 83.3%.
- **Not done:** `rpoisson()`, `rbeta()`, `rt()` (StatCraft has not found Stata's methods), KISS32,
  `c(rngstate)`.

## P2C.13 continued: collinear terms in nbreg, ologit, oprobit and mlogit, 2026-11-02

- **What works:** these four models omit a collinear regressor as Stata 19 does: the later one in the
  order written, with Stata's note, `o.` columns in `e(b)` (in every equation for `mlogit`, whose base
  outcome is all `o.`), `_b[]` of 0, and `e(rank)`/`e(df_m)` counting only what is estimated.
- **How:** their R fits (`glm.nb`, `polr`, `multinom`) do not leave a collinear column out themselves.
  `stata_collinear_rhs` finds Stata's omitted terms from the design before the fit, and the model is fitted
  without them and keeps them in `$omitted`.
- **Corpus:** `collinear.do` checks all four as well (35 results); `nbreg` is Verified for the first time.
- **Also:** `nbreg` on counts with no overdispersion, where Stata's alpha goes to 0 and R's `glm.nb` fails,
  now stops with that reason instead of R's "missing value where TRUE/FALSE needed".
- **Not done:** collinearity in the panel, survival and IV models; `nbreg` at alpha = 0.

## P2C.9 continued: public replication packages as inputs, 2026-11-02

- **Inputs:** 11 small public replication packages from Harvard Dataverse, each with do-files and data:
  46 do-files that run 15,147 lines. They are kept outside the repository. The first dry run translated
  40.5% of their lines; `tests/many_dofiles.rs` now also writes every refused line with its reason
  (`RCRAFT_COVERAGE_LINES`).
- **Translated from what they use:**
  - `erase` and `rm`: Stata's message and r(601) for a file that is not there; r(602) for `save` over one;
  - `compress`, `set niceness`, `set processors`, `set max_memory` and the like, and `set scheme`: nothing to do in R;
  - `append, force`: the using file's values of a variable of the other kind are missing;
  - `reshape long` without `j()` makes `_j`;
  - `fvset base #`, `base default` and `fvset clear`: later models write `i.x` (and a bare `x` in an
    interaction) as `ib#.x`; the choice stays with the data (save and use, preserve and restore keep it,
    import and clear drop it);
  - `cluster()` and `cl()` beside a model mean `vce(cluster)`, with `robust` beside them changing nothing.
- **Fixed:** the value label sets the do-file defined were kept after new data was read, so a second
  `label define` of the same set after `import ..., clear` was refused. They now go with the data, and a
  file the do-file saved brings its sets back when it is used again.
- **Observed:** `plan/stata/replication-forms.md`. **Corpus:** `replication.do` (19 results) passes;
  `append`, `erase`, `fvset` and `reshape` are Verified, with guide pages for `compress`, `erase` and `fvset`.
- **Numbers:** the packages' share is 52.1%, StatCraft's probes' 83.3% (`docs/stata-coverage.md`).
- **Not done:** add-ons from SSC (`robreg`, `outreg2`, `reghdfe`, `eststo`, ...), which are most of what is
  left; `estimates store` / `restore`, `clogit`, `tsset` and time-series operators, `gettoken`,
  `tabulate, generate()`, `xtile`, `assert`; `fvset base first/last/frequent/none` and `fvset report`;
  running the packages on R against Stata.

## P2C.9 continued: estimates store and restore, 2026-11-02

- **What works:** `estimates store` (`est sto`) keeps the model with its e(), e(sample) and what `predict`,
  `test`, `lincom` and `margins` read. `estimates restore` (`est res`) brings it all back, with Stata's note
  `(results a are active now)`. `estimates drop` and `estimates clear` forget them.
- **Rules from Stata 19:**
  - a name that is not stored is `estimation result a not found`, r(111);
  - storing with no model is refused with Stata's reason, r(301).
- **Stored models and the data:** a line that moves the rows or changes the data afterwards clears a stored
  model's e(sample) and its copy of the sample, as it does for the model held.
- **Lines that were not translated:** after one, a name not stored here may have been stored in Stata
  (`eststo`, a program), so its restore is refused rather than guessed.
- **Corpus:** `estimates.do` (9 results), with Stata 19's results recorded. `estimates` is Verified, with a
  guide page.
- **Numbers:** the packages' share is 53.1% (tier 2 from 10.4% to 17.2%); StatCraft's probes stay at 83.3%.
- **Not done:** `estimates dir`, `table`, `replay`, `save` and `use`.
- **Note:** Pinokio reinstalled its miniforge on 2026-11-02 at 12:13, which removed the R environment this
  machine used (`hev-r`). The R tests skip here until R is back; GitHub CI runs them on all three platforms.

## P2C.9 continued: assert and tabulate, generate(), 2026-11-02

- **assert:** Stata 19's messages (`23 contradictions in 60 observations`, then `assertion is false`), r(9),
  with if and in, `rc0` and `(null assertion)`. A missing value counts as large, as in Stata. `fast` and `null`
  are refused.
- **tabulate, generate():** the indicators, with Stata's labels (the number in `%11.4f`, `%11.4e` when wider),
  missing outside the table's rows, and an indicator for missing with `missing`.
- **Corpus:** `checks.do` (24 results); `assert` is Verified, and `tabulate` is also checked there.
  `replication.do` and `estimates.do` now put one value on each result line: the corpus compares only the
  first number after a key, so the lines with two or three values had been checking just the first.
- **Numbers:** the packages' share is 53.3%.
- **Machine:** a system R 4.6.2 appeared at 12:29 without the packages the corpus needs. The R tests skip
  here or fail on missing packages (the Excel import test) until it is set up; GitHub CI runs them.

## P3.2 continued: the user's own R library stays on the path, 2026-11-02

- RCraft gave its own library to R as `R_LIBS_USER`, which replaced R's own
  per-user library (`%LOCALAPPDATA%\R\win-library\4.6` and the like), so
  packages the user installed the usual way were not found. It is given as
  `R_LIBS` now: R searches it first, then the user's library, then R's own.
- Heads-up about this machine: Pinokio's reinstall removed the HEV R (R 4.4)
  and a system R 4.6.2 took its place. RCraft's library still held R 4.4
  builds, and `readr`'s compiled code would not load in 4.6. With the user's
  go-ahead all 40 were replaced with current builds for 4.6, and the 14
  packages CI uses were installed into the user's own R library. The corpus,
  format and save tests run here again.
- R keeps a library per minor version for this reason; RCraft's is one folder
  for every R, so a user who upgrades R can meet the same failure.

## P2C.9 continued: tsset and the time-series operators, 2026-11-02

- **tsset** (`data_r/ts.rs`, `stata_tsset`): Stata 19's lines (gaps, strongly or weakly balanced panels,
  delta), its errors (r(451) for repeated or non-integer times and times closer than delta, r(109) for a
  string), and the data sorted by panel and time. `xtset id t` goes the same way. The settings go with the
  data: kept by save and use and by preserve and restore, forgotten by clear, import and set obs; after
  `use` of a file this do-file did not save, they are not known and the operators are refused.
- **L., F., D., S.** (`stata_ts`): matched on panel and time, missing where no row has the time, over every
  row whatever `if` keeps; a chain is one sum over x as in Stata (`D.F.x` is F.x - x). "not sorted", r(5),
  when the data are not in time order. Under `by`, only when by names the panel.
- **Models:** `L(1/2).x`, `L.(x y)` and chains are written out in Stata's canonical names (`LD.x`, `FD.x`,
  `L2.x`), which become columns of the model's data and the coefficients' names; a lagged outcome works;
  `predict` adds the same columns.
- **replace x = L.x + 1 if ...:** done again until nothing changes, which is Stata's row-by-row result
  when the line reads only earlier times of x; `D.x` or `F.x` of the replaced variable is refused.
- **Checked:** `timeseries.do` (38 result lines, every value, tolerance 1e-6); `tsset` is Verified.
  Observations in `plan/stata/timeseries.md`.
- **Numbers:** the replication packages' share is 53.8% (53.3%); tier 3 went from 0.5% to 16.3%. The 36
  lag regressions in one package wait on a `merge m:m` before them.
- **Machine:** R 4.6.2 with the packages the tests use is set up again, so the corpus runs here.

## P2C.9 continued: clogit, and vce(r) and vce(cl), 2026-11-02

- **clogit** (`data_r/clogit.rs`, `stata_clogit`): the exact conditional likelihood, as Stata's, fitted by
  `survival::coxph` with a stratum per group and `method = "exact"`, so groups with several positives are
  right. Stata's notes and its dropping of all-positive or all-negative groups and of variables that do
  not vary within groups. survival has no robust variance for the exact method, so each row's score (x
  times its outcome less its chance of being among the group's positives, from the elementary symmetric
  sums) is worked out here: `vce(robust)` clusters on the group, `vce(cluster v)` checks the groups are
  nested (r(459)), both with G / (G - 1). e() as Stata leaves it (e(k) with the base columns, e(N_drop),
  e(N_group_drop), e(multiple)); `predict` pc1, pu0 (invlogit(xb)) and xb.
- **vce():** `vce(r)` and `vce(cl v)` at the shortest, as Stata takes them; `ols` and `oim` in full.
- **Checked:** `clogit.do` (21 result lines, every value, tolerance 1e-6); `clogit` is Verified.
  Observations in `plan/stata/clogit.md`.
- **Numbers:** the replication packages' share is 55.9% (53.8%): tier 2 26.2% (18.5%), tier 3 52.3%
  (16.3%), most of it the `vce(clu ...)` the packages write. StatCraft's probes stay at 83.3%.

## P2C.9 continued: gettoken, 2026-11-02

- **gettoken** (`dofile.rs`): the first token of a macro and what is left, as Stata 19 takes them: leading
  blanks skipped, the rest keeping its delimiter, quotes taken off, a `parse()` character a token of its
  own (`<=` and `==` together), `match()` for a token in parentheses, and `(global)` and `(local)` on any
  name. A macro not known makes the macros it sets not known. `quotes`, `qed()` and `bind` are refused.
- **Checked:** 15 cases against Stata 19 in `dofile::tests`, and three lines in `made.do` (the `|`-list
  loop the packages use, `match()`, a `while` over a list); `gettoken` is Verified. Observations in
  `plan/stata/gettoken.md`.
- **Numbers:** the replication packages' share is 56.5% (55.9%); StatCraft's probes 83.4%.

## P2C.9 continued: xtile, 2026-11-02

- **xtile** (`data_r/more.rs`, `stata_xtile`): 1 plus the number of cut points below each value (repeated
  ones each counting), the cut points Stata's percentiles of the rows `if` and `in` keep (R's type 2,
  `altdef` type 6) or every value of `cutpoints()`'s variable; missing outside the rows kept; Stata's label.
- **Checked:** `xtile.do` (12 result lines, every value, tolerance 0); `xtile` is Verified. Observations in
  `plan/stata/xtile.md`.
- **Numbers:** the replication packages' share is 56.6% (56.5%); StatCraft's probes 83.4%.

## P2C.9 continued: merge, every type, 2026-11-02

- **merge** (`data_r/more.rs`, `stata_merge`): one rule for 1:1, m:1, 1:m, m:m and 1:1 `_n`, as Stata pairs
  rows (the k-th with the k-th within a key, the shorter side's last row standing in); the master's rows
  first by key, the rest after by key; `update` (4, 5) and `replace`; `keep()` with its short words and
  match_update, match_conflict; `assert()` before `keep()`, the merged data left in memory on r(9);
  Stata's report; `nolabel`. Empty-string keys match, as in Stata (R never looks up an empty name, so the
  keys carry a mark). Rows with the same key keep their order, where Stata's sort may not (a caveat).
- **Files saved after cd:** the session keeps the folder `cd` named, so `save f` after `cd d` is the same file as `use d/f` later (case aside), and its tsset, xtset, fvset and label sets come back with it. xtreg after `use` of a file whose settings are not known says so, rather than Stata's "must specify panelvar".
- **Dry runs:** `tempfile` without a folder marks its names unknown instead of leaving them empty, and the
  coverage run gives a folder, as the app does; 10 package lines had read `using ,`.
- **Checked:** `merge.do` (16 result lines, every value, tolerance 0); `merge` is Verified. Observations in
  `plan/stata/merge.md`.
- **Numbers:** the replication packages' share is 58.8% (56.6%): tier 2 29.9%, tier 3 61.9%.
  StatCraft's probes 83.4%.

## P2C.9 continued: display options, 2026-11-03

- **Display options** (`DISPLAY_OPTIONS`, `MARGINS_DISPLAY` in `data_r.rs`): `baselevels`, `allbaselevels`,
  `noomitted`, `vsquish`, `noemptycells`, `nofvlabel`, `cformat()`, `pformat()`, `sformat()`, `nolstretch`,
  `fvwrap()`, `fvwrapon()`, `noci`, `nopvalues`, `noheader`, `notable` and `coeflegend` change only how
  Stata's table looks, so every model takes them, down to the shortest form Stata 19 takes (`base`,
  `noomit`, `cf()`, `nolstr`; `allb`, `noo` and `vsq` are refused, as Stata refuses them). `margins` takes
  them too, with `asobserved` (its default) and `noatlegend`.
- **Numbers:** the replication packages' share is 59.1% (58.8%). The 100 `margins` lines after `clogit`
  now stop at `mcomp(bonferroni)`; behind it are `margins` after `clogit` with `predict(pu0)` and
  `vce(unconditional)`, not translated yet.

## P2C.9 continued: robreg m (SSC), 2026-11-03

- **Installed** in Stata 19 here with the user's go-ahead: `moremata`, `robreg`, `ftools` and `reghdfe` from
  SSC (into `C:\Users\jstow\ado\plus`), to observe them. Their code is not read.
- **robreg m** (`data_r/robreg.rs`, `stata_robreg`): LAD starting values, the scale from their residuals
  (the normalized median, the p zero residuals left out), Huber's M estimate with that scale fixed, the
  sandwich times n / (n - p) and t tests on n - p, the empty model from the median, e() and `predict` (xb,
  residuals, weights). Base R only. When the LAD fit is not unique (indicator variables often make it so)
  robreg starts inside the LAD solutions and RCraft at a corner, so the estimates can differ a little; a
  caveat says so.
- **Checked:** `robreg.do` (12 result lines, every value, tolerance 1e-6); `robreg` is Verified.
  Observations in `plan/stata/robreg.md`.
- **Numbers:** the replication packages' share is 79.3% (59.1%): core statistics 84.8% (29.9%). The
  package's loops over robreg now stop only at one `egen rowmedian()`. StatCraft's probes 83.4%.

## P2C.9 continued: reghdfe (SSC), 2026-11-03

- **Installed** `require` too, which reghdfe asks for (SSC, with the go-ahead for reghdfe).
- **reghdfe** (`data_r/reghdfe.rs`, `stata_reghdfe`): one or two absorbed variables or `noabsorb`;
  singletons dropped (again until none are left) or kept; the absorbed means taken out, `_cons` with the
  grand means added back; reghdfe's degrees of freedom (connected groups, fixed effects nested in a cluster
  variable); unadjusted, robust, clustered and two-way clustered errors with its small-sample factors and
  Cameron, Gelbach and Miller's fix; e() and Stata's header. `predict` after it is refused for now.
- **Checked:** `reghdfe.do` (12 result lines, every value, tolerance 1e-6); `reghdfe` is Verified.
  Observations in `plan/stata/reghdfe.md`.
- **Numbers:** the replication packages' share is 80.2% (79.3%); the add-on tier 21.1% (0%). StatCraft's
  probes 83.4%.

## P2C.9 continued: eststo and estadd (SSC estout), 2026-11-03

- **eststo** (`data_r/returned.rs`, and the `eststo [name]:` prefix in `reader.rs`): stores the active
  results as `estimates store` does, under the name or est1, est2, ... by position, with
  `e(_estimates_name)`; `eststo name: command` fits and stores; `eststo clear` and `drop`.
- **estadd scalar** and **local**: add to e(), and to the stored copy when the active results are stored
  (by eststo or estimates store), as the package does; r() and e() stay.
- **regress with a constant alone**: its Wald test now gives Stata's F(0, df) = 0, where R stopped on an
  empty matrix.
- **Checked:** `estout.do` (13 result lines); `eststo` and `estadd` are Verified. Observations in
  `plan/stata/estout.md`.
- **Numbers:** the replication packages' share is 80.7% (80.2%); the add-on tier 32.3% (21.1%).

## P2C.9 continued: outreg2 (SSC), and import delimited, 2026-11-03

- **Installed** outreg2, parmest and fre from SSC with the user's go-ahead. SSC's `cluster2` turned out to be
  another program (power analysis for cluster-randomized trials) and was uninstalled; the packages use
  Mitchell Petersen's cluster2, which is not on SSC.
- **outreg2** (`data_r/outreg2.rs`, `stata_outreg2`): `outreg2 using f.xls, tstat bdec(#) tdec(#)` with
  `parenthesis(tstat)`, `adjr2`, `r2`, `keep()`, `e()`, `replace` writes f.txt and f.xls byte for byte as the
  package does, a column per model. Its own number format and its LaTeX form are refused.
- **import delimited**: `stringcols(_all)`; a line of delimiters alone is a row, as in Stata, where readr
  dropped it (`stata_read_delim`); `varnames(nonames)` named its columns with `.` inside R's pipe, which
  would have stopped in R.
- **Checked:** `outreg2.do` (the .txt read back cell by cell; the files compared byte for byte with Stata's
  by hand); `outreg2` is Verified. Observations in `plan/stata/outreg2.md`.
- **Numbers:** the replication packages' share is 81.9% (80.7%); the add-on tier 63.1% (32.3%).

## P2C.9 continued: fre (SSC), 2026-11-03

- **fre** (`data_r/fre.rs`, `stata_fre`): its tables as the package prints them (compared character by
  character with Stata's log), value labels, a string's empty value as missing, several variables; r(N),
  r(N_valid), r(N_missing), r(r), r(r_valid), r(r_missing), r(depvar).
- **Checked:** `fre.do` (its r()); `fre` is Verified. Observations in `plan/stata/fre.md`.
- **Found on the way:** R's `attr(x, "label")` matches `labels` partially when a variable has value labels
  and no variable label; `fre` asks for it exactly.

## P2C.9 continued: margins after clogit, and parmest (SSC), 2026-11-03

- **margins after clogit** (`data_r/margins.rs`, `data_r/clogit.rs`): pu0 (the default) as invlogit(xb) from
  the model as written (`stata_mfit`); `vce(unconditional)` from the rows' scores of the exact likelihood
  times the inverse information, clustered as the model was; `ar.` contrasts (each level with the one before)
  and `r.` term by term, with Stata's chi2 tests; `mcomp(bonferroni)` for the printed p-values and intervals;
  `post` keeps the base levels in e(b) as zeros, under Stata's names; r(b) and r(V) without post.
- **parmest** (`data_r/parmest.rs`): its dataset as the package saves it, for normal-deviate models.
- **Fixed on the way:** `el(e(V), i, j)` and `el(r(b), ...)` looked their matrix up as a number and were
  always refused; `display` and `matrix` between a model and `margins` no longer count as changing the rows;
  clogit's e(b) has its equation, `chosen:`, as Stata's.
- **Checked:** `clogit_margins.do` (6 result lines, the package's sequence, both parmest files read back);
  `parmest` is Verified. Observations in `plan/stata/margins-clogit.md`.
- **Numbers:** the replication packages' share is 83.5% (81.9%); core statistics 90.4%.

## P2C.9 continued: whole packages in Stata 19 and on R, 2026-11-03

- **The run** (`crates/engine/tests/replication_e2e.rs`): each package runs from its master do-file, line by
  line through the Engine, past any problem. R's output, the R it ran and each line's fate are written next to
  the copy. Stata ran the same copies with `do file, nostop`. In four packages every model line both reached
  gives Stata's numbers (ASKO93 140, FMDLII 7, TQN6ZN 231, TZEN38 252). YFBPT6's 312 differences followed an
  `xpose` that was not translated. Table and causes in `docs/stata-coverage.md`.
- **Fixed on the way:** `lincom` with base levels (0, as Stata; `0 (omitted)` when every term is one);
  `label values v name` with a set from the .dta file (read from its value-labels section, as haven drops the
  sets' names); `xtreg, fe` with `i.year` in a balanced panel stopped in R; `recode` with labels in its
  rules; `decode`; `encode`'s label set; `xpose`; `do file, nostop`; `colsof()` and `rowsof()` of e() and r()
  matrices.
- **Checked:** `panel.do` (lincom with base levels), `data.do` (recode labels, decode), `made.do` (xpose);
  `lincom` is Verified in panel, and `xpose` is Verified. Observations in `plan/stata/replication-e2e.md`.
- **_natscale:** its step is found (Stata's 10^k, not the nearest double for k < 0), but its ends are not; the
  113 probe lines stay. Observations in `plan/stata/natscale-residue.md`.
- **Numbers:** the replication packages' share is 83.7% (83.5%).

## P2B.5 continued: graphs in the replication packages, 2026-11-03

- **Graphs** (`data_r/graph.rs`): `twoway bar` (from `base()`, 0, `barwidth()` wide, 1) and `twoway rarea`;
  `twoway plot, opts || plot, opts` split at `||` with each plot's own options, which carry the graph's
  too; `graph box`/`hbox` (Stata's quartiles, whiskers to the adjacent values, the rest as points);
  `graph bar (count)` with no variable counts the rows (another statistic is r(100), as in Stata);
  `xline()` and `yline()` draw their lines; `graph drop _all`; `graph export`'s `preview()`, `fontface()`
  and `cmyk()`, which change only the file's look.
- **Look-only short forms:** the ones Stata 19 accepted when asked (`lp()`, `lc()`, `clp()`, `clc()`,
  `clw()`, `mlabv()`, `mlabp()`, `mlabs()`, `mlabc()`, `mlw()`, `bc()`, `fi()`, `blw()`, `xsc()`, `ymt()`,
  the `t1title()` family). Those that change what is drawn (`recast()`, `connect()`, `cmissing()`,
  `jitter()`, `saving()`) stay refused.
- **Checked:** `stata_graphs.rs` draws each form in R and refuses `graph drop` by name, `twoway bar,
  horizontal` and `graph bar (mean)` without a variable.
- **Numbers:** the replication packages' share is 83.9% (83.7%); graphs 16.2% (0%).
- **The probes again** (406, two more than before: `fre` and `reghdfe`): four lines had changed or were new.
  `nbreg` where alpha goes to 0 (probe 064): glm.nb now stops at a huge theta instead of failing, and printed
  numbers that were not Stata's; it is refused there, as it was. `estimates store` now names the results
  (`e(_estimates_name)`), not only `eststo` (073). `reghdfe, noabsorb` absorbs the constant as one group
  (`e(N_hdfe)`, `e(df_a_initial)`, `e(absvars)` `_cons`, 141). `fre` is refused once the do-file stores an
  extended missing value, which R keeps as one NA (131).
- **Numbers:** the probes: 159,638 of 190,347 lines translated (83.9%); 5,561 of 5,679 compared lines match Stata to 8 digits and 5 more to 6; the 113 that differ are all `_natscale`.

## P2B.5 continued: a second y axis, 2026-11-03

- **yaxis(2)** (`data_r/graph.rs`): the plots on it (scatter, line, connected, bar, rarea) are mapped onto
  the first axis's range from the values of each axis (`stata_axes`), and the right axis shows their own
  scale, titled with their variable's label. Each graph has its own mapping, since ggplot2 draws later.
  Every plot on axis 2, and fitted or density plots on it, are refused.
- **Numbers:** graphs 22.2% (16.2%); the packages 83.9%.

## P2C.9 continued: coefplot (SSC), 2026-11-03

- **coefplot** (`data_r/coefplot.rs`): installed in Stata 19 from SSC with the user's yes, and matched to
  what its `gen()` variables and SVG labels show. The rows from Stata's e(b) names (`_cons` in; base
  levels with `baselevels` and omitted terms with `omitted`, as 0), in order of first appearance with
  `_cons` last; `keep()`, `drop()`, `order()` with empty rows; t intervals with the residual df (normal
  without one) at `levels()`; several plots 1/(k+1) apart, `offset()`; `headings()` (with `nogap`),
  `coeflabels()`; subgraphs at `||` with `bylabel()`; `gen()` as floats; `saving()`; look-only options
  left to ggplot2.
- **Checked:** `coefplot.do` (53 rows of positions, estimates and interval ends from Stata 19's `gen()`,
  to 1e-7 as they are floats); `coefplot` is Verified. Observations in `plan/stata/coefplot.md`.
- **Numbers:** the packages' share is 84.6% (83.9%); graphs 62.4% (22.2%).

## P2C.9 continued: esttab (SSC), and the last of YFBPT6's differences, 2026-11-04

- **esttab** (`data_r/esttab.rs`): the screen table, `.txt`, `.csv` and `.tex` (booktabs or plain, a
  caption with `title()`), matched to Stata 19's output: a3 and fixed formats, t, se, p or none, stars and
  `star()`, `label`, base levels, N, r2, ar2, aic, bic, `scalars()`, `mtitles()`, `nonumbers`, `keep()`,
  `drop()`, `order()`, widths. 13 screen tables and 8 files were byte for byte Stata's; `esttab.do` reads
  four back. `esttab` is Verified.
- **YFBPT6 against Stata** (both now running the whole package): 4,334 model lines the same and 26
  different, from 385 and 312. What closed the gap: `destring(x)` with its variable in parentheses;
  `egen rowmedian()` and `rowpctile()` (`data.do`); `robreg`'s LAD start from an interior-point solver
  (quantreg's Frisch-Newton), polished to the vertex when the LAD fit is unique (its rows' dual weights
  inside (-1, 1)): all 200 of the package's fits match to 1e-7 (`robreg.do` adds two indicator cases).
  The remaining 26 were `display e()` after a model refused for a variable R lacks; such a line now
  loses e() and r(), as Stata ran the model.
- **Numbers:** the packages' share is 85.2% (84.6%).
- **A base level interacted** (`0.treat#1.female`, new probe 580): `test` drops a constraint of only base
  levels (`Constraint 1 dropped`, r(dropped_1), r(drop), and F(0, df) = . when none is left), `lincom`
  shows it `0 (omitted)`, both with `_rc` 0 as in Stata (`regress.do`).
- **Numbers:** the probes (412): 159,806 of 190,664 lines translated (83.8%); 5,566 of 5,684 compared lines match Stata to 8 digits, 5 more to 6; the 113 that differ are all `_natscale`.

## Step 3 begun: programs, and a second measure, 2026-11-04

- **Programs** (`dofile.rs`, `dofile/progsyntax.rs`, `data_r/returned.rs`): `program define name[, rclass]
  ... end` is kept and run where it is called, its lines translated as any, with locals of its own (`0',
  `1' ...), `args`, `syntax` (varlist, namelist or anything; [if] [in]; flags, noCONstant, typed options
  with defaults, `*`), `marksample` (as generate, replace and markout lines), `exit` returning from it,
  nested calls, its temporary variables dropped when it returns, and an r-class program's `return scalar`,
  `return local` and `return clear` as r() afterwards. A program with a line that is not translated leaves
  r() unknown; e-class programs are not translated yet.
- **Checked:** `programs.do` (9 results against Stata 19); the 28 StatCraft probes that define programs, none
  differing: probe 110 went from 4 translated lines to 607, 346 from 18 to 766, 348 from 502 to 1,463.
- **A second measure:** the packages' share counts lines as they run, so a line in a loop counts each time.
  With programs run, one package's matrix simulation (TZEN38's `learnalg`, called in loops) adds about
  200,000 run lines, most of them `matrix` commands, and that share falls to 40.0%. Counting each source
  line once (`many_dofiles` prints it too): 68.7% of 5,149 lines, against 68.8% of 5,128 before programs.
  Stata's matrix commands (`matrix A = J()`, element assignment, arithmetic, `mkmat`) are the largest gap
  left, and come next.
- **Numbers:** the probes (413): 163,251 of 207,967 run lines translated (78.5%; 159,806 of 190,664 before programs ran); 5,566 of 5,684 compared lines match Stata to 8 digits, 5 more to 6; the 113 that differ are all `_natscale`.

## P3.2 continued: compressed data files, 2026-11-04

- **Use my file opens packed files** (`rdata::unpack`, layer 2): `.zip` (stored, deflate, bzip2, Zstandard and xz
  entries; Zip64), `.gz`, `.bz2`, `.xz`, `.zst`, `.tar` and the combinations (`.tar.gz`, `.tgz`, `.tar.xz` ...).
  The kind is told from the first bytes; archives inside archives are opened (three deep); every checksum is
  checked, so a damaged file is refused; password-protected zips, 7-Zip and RAR get plain messages; the unpacked
  bytes are capped at 4 GB. `.rds`/`.RData` (gzip inside, read by R) and Office files (zip with
  `[Content_Types].xml`) are left as they are. Behaviour follows Husky Explorer's unpacker (PR 24 there).
- **Decoders:** pure Rust only. gzip and deflate through `flate2`/`miniz_oxide`, xz through `lzma-rust2`, Zstandard
  through `ruzstd`. bzip2 is ported from Husky Explorer's decoder (`unpack/bunzip.rs`): `bzip2-rs` checks no
  stream CRC, reads one stream only and can panic, and the `bzip2` crate's pure-Rust backend carries the bzip2
  licence rather than MIT/Apache. The zip and tar readers are our own and bounds-checked; a panic in a decoder
  is caught and reported as a damaged file.
- **Where the data goes:** R must read a real file, so the data file is written next to the archive under its
  own name (`survey.csv.gz` gives `survey.csv`); one already there with the same bytes is reused, a different
  one is never replaced (`survey (unpacked).csv`, then numbered). If the folder cannot be written, the copy goes
  to `rcraft-unpacked` in the system's temporary folder. The dialog says where it went, the R code starts with
  `# unpacked by RCraft: <name> from <archive>`, and the recent-files list keeps the archive. An archive with
  several data files lists them (with sizes) and the user picks one. Browse's filter takes the packed endings;
  files dropped on the window go through the same path.
- **Tests:** the xz, zstd and bzip2 files made by the real tools (Husky Explorer's fixtures, in
  `crates/rdata/fixtures/unpack/`, binary in `.gitattributes`), damaged, cut and capped variants, zips and tars
  built in the tests, every cut of a zip and tar.gz and every flipped bit of a .bz2 (an error, never a panic),
  and three dialog tests.
- **Not done:** unpacking runs on the window's thread, so a very large archive holds the window for a few
  seconds; the whole archive is read into memory (within the cap). Do-files (`.do`) are not unpacked.

## Step 3 continued: Stata's matrices, and insheet, 2026-11-04

- **Matrices** (`data_r/mexpr.rs`, `data_r/matrices.rs`): `matrix A = exp` with `J()`, `I()`, joins with `,`
  and `\`, products, sums, a number or 1x1 matrix times a matrix, `/` by a number, transposes, `invsym()`,
  `inv()`, `vecdiag()`, `diag()`, `e()` and `r()` matrices, `nullmat()`, any other function as a number;
  `matrix A[i,j] = exp` (a matrix put with its corner there); `rownames`, `colnames`, `drop`, `list` of
  the do-file's matrices; elements by number or name in any expression (`A[1, "x"]`, `el()`, `rowsof()`);
  `mkmat` with if, in, nomissing and without `matrix()`. Every name follows Stata 19 as probed (a sum
  takes the right one's names; `plan/stata/matrices.md`). A matrix a refused line may change is declined
  from then on; one never made stops R with r(111) as Stata does. Declined: `#`, submatrices,
  `cholesky()`, a singular `invsym()`.
- **insheet**, as Stata 19 reads a file (its names, labels, floats, `double`, `nonames`, `case`, r(18)).
- **svmat** (names(col), a stub, float or double, the rows it adds, r(110)), with `matrices.do`.
- **The dry run:** macros set in a loop it cannot run are known only when run; a line refused for what
  such a line leaves counts as known when run.
- **Checked:** `matrices.do` (29 results against Stata 19), and three probes compared screen for screen
  (`matrix list` of every form above matches Stata's output).
- **Numbers:** the packages' source lines, each once: 70.1% of 5,149 (68.7%). Run lines: 50.6% translated
  (40.0%); 64.6% of those a dry run judges once lines that wait on run-time values count as such.
- **Strings** (`expr_r.rs`, `strings.do`, 11 results against Stata 19): `strltrim()`, `strrtrim()`,
  `strmatch()` (`*` and `?`, every other character itself, new lines matched by `*`) and `char()` (ASCII;
  "" when missing or past 255; a single byte stops). `trim`, `strtrim`, `ltrim`, `rtrim` and the `str`
  forms took tabs and new lines as R's `trimws()` does; Stata's take spaces only, as now (the `ustr` ones
  take tabs and new lines too). `float()`. TZEN38's text cleaning now stops first at `split`.
- **Numbers:** source lines 73.0% of 5,149; run lines 64.9% of those a dry run judges.
- **Probes (415):** 163,509 of 207,839 run lines translated (78.7%); 5,569 of 5,687 compared lines match
  Stata to 8 digits, 5 more to 6; the 113 that differ are all `_natscale`.

## Step 3 done: split and file; _natscale's residue declined, 2026-11-04

- **_natscale:** a label at 0 with a step below 1 is declined (Stata's end there can be 2^-55 rather than
  0, even for 0.045 to 0.667 with 10 labels, so no narrower rule holds). None of the 46 packages calls it.
  The probes then match on every compared `_natscale` line; probe 225 declines 30 of 480 ranges, the sweeps
  of probe 226 1,479 of 4,915.
- **split** (`data_r/more.rs`, `plan/stata/split.md`): words without parse(); the strings parse() gives
  after trimming the ends (notrim), empty pieces kept, the later listed string winning a tie; limit(),
  if and in, r(k_new) and r(varlist), r(110) and r(109). destring and overlapping parse strings declined.
- **file** (`data_r/files.rs`, `plan/stata/file.md`): open (write, replace, append, text or binary), write
  (strings, _n as the system writes it, _tab, _skip, _char, (exp) as display writes it, %fmt (exp); %1bu
  and %1bs bytes in binary files) and close,
  with Stata's r(602), r(110) and r(111); the files are byte for byte Stata's. file read is not translated.
- **Checked:** `strings.do` (now 28 results against Stata 19), and probes compared line for line.
- **Numbers:** source lines 73.1% of 5,149. Probes (417): 161,600 of 208,696 run lines translated (77.4%; the
  declined `_natscale` ranges count as not translated); none of the 4,197 compared lines differs, 4,192
  match Stata to 8 digits and 5 to 6.

## Step 4 begun: xtregar, prais, newey and input from StatCraft's decoding, 2026-11-05

- **The method:** StatCraft (our own code) already matches Stata on these; its notes on Stata's behaviour
  (`plan/stata/panel.md`, `time-series.md`, `data-commands.md`) are written here in R, as `_natscale`,
  the random numbers and the dates were before (`plan/stata/arpanel.md`).
- **xtregar** (`data_r/xtregar.rs`): fe and re, every rhotype(), rhof(), twostep, lbi, and e() in
  Stata's order. re's sigma_u agrees to about 1 part in 1e8 (StatCraft accepts a few parts in 1e7).
- **prais and newey** (`data_r/tsreg.rs`): Prais-Winsten and Cochrane-Orcutt, twostep, rhotype(dw),
  vce(robust); Newey-West with any lag. Plain variable names only.
- **input** (`data_r/more.rs`), into empty data, with Stata's storage types.
- **replace row by row:** an AR(1) series built with `replace e = .5*L.e + rnormal() if t > 1` or with
  `e[_n-1]` under by now runs as Stata's (the generator put back before each round of the repeated
  replace); `x[_n]`, `x[_n+k]`, `x[_N]` and `x = x[1]` too. `replace` keeps the storage type generate
  gave (a float stays a float, row by row). Those lines built the data of the 45 xtregar probes.
- **Found on the way:** length() counts bytes; subinstr() works on bytes (char(128) to char(255) there);
  `_rc` after `capture do` of a file with a line not translated is not known; `capture include` declined.
- **Checked:** `arpanel.do` (30 results against Stata 19), `strings.do` (29), and the probes.
- **Numbers:** probes (420) 188,260 of 210,197 run lines translated (89.6%, from 77.4%); none of the 4,310
  compared lines differs, 4,299 match Stata to 8 digits and 11 to 6 (6 of them xtregar re's sigma_u).
  Packages: 73.0% of source lines (the declined char(0) and lone bytes outside subinstr() take 0.1).
- **Then:** `sign()`; a one-line `else command` after an `if { }` block inside a loop (the command was
  read again with its `else`), and `} else command` on one line. Probe 346 went from 498 lines translated
  to all 5,178.

## Date display formats, confirm, and file names in compound quotes, 2026-11-05

- **Date and time display formats** (`stata_tfmt` in `data_r/returned.rs`, from StatCraft's decoding):
  every `%t` unit but `%tC`, every code, Stata's widths and padding; probe 033 went from 311 lines
  translated to 2,066, all matching Stata byte for byte (123 cases in `stata_returned.rs`).
- **confirm** (`data_r/more.rs`): variable, numeric/string variable, new variable, [integer] number,
  file and new file with Stata's messages and return codes (`strings.do`).
- **Compound quotes around a file name** are taken off (they were kept, so `save `"`f'"'` wrote the
  wrong file). `plan/stata/date-formats-confirm.md`.
- **Numbers:** probes (421) 194,948 of 209,914 run lines translated (92.9%, from 89.6%; `sign()`, the
  one-line else, the date formats and confirm); none of the 4,320 compared lines differs, 4,309 to 8
  digits and 11 to 6. Packages 73.1% of source lines.
- **Then:** datetime/C with the leap seconds (`Clock()`, `Cofc()`, `cofC()`, `Cofd()`, `dofC()`,
  `hhC()`/`mmC()`/`ssC()`, `Chms()`, `Cdhms()`, `Cmdyhms()`, `%tC`); `invlogit()` and `logit()`; a string
  times a whole number. `dofC()` inside a leap second is the leap day, which StatCraft's notes miss.
- **Then:** a command Stata's table does not have (an add-on, or Stata's own `_get_diopts`) may set
  the caller's locals (`c_local`): when it is not translated, the locals named before its comma are not
  known after it (probe 594 showed them read as empty).
- **Numbers:** probes (422) 196,061 of 210,072 run lines translated (93.3%); none of the 4,323 compared
  lines differs, 4,312 to 8 digits and 11 to 6.

## Packages' gaps: ibn., format, postfile, graph combine, and cluster2, 2026-11-06

- **ibn.** (`data_r/model.rs`, `matrices.rs`, `post.rs`): `regress y x ibn.g, noconstant` gives every
  level its coefficient, named `kbn.g` in e(b) and `_b[]`; ibn. without noconstant is declined. No
  `e(ll_0)` after a regress without a constant, as Stata leaves none.
- **format** (`data_r/more.rs`): kept as the variable's `format.stata` attribute (haven writes it).
- **postfile, post, postclose, postutil clear** (`data_r/postfile.rs`, StatCraft's decoding checked
  in Stata 19): Stata's storage types as values are posted (float rounded, strings cut), `type(a b)`,
  `every()`, Stata's errors, each closing the postfile; a declined post leaves its handle unknown.
  Probe 579: 90 of 95 lines translated, 14/14 compared match (`postfile.do`).
- **graph name(), saving(), graph save, graph combine** (`data_r/graph.rs`): graphs kept by name or
  in files (RCraft's own, saveRDS) and laid out with gtable.
- **cluster2** (`data_r/cluster2.rs`; `plan/stata/cluster2.md`): Petersen's add-on, installed from his
  page at the user's go-ahead and observed, never read: V = clustered by each dimension less clustered
  by their cells; F and df_r from the cell-clustered (or robust) fit; Stata's header, footer and e(),
  no e(sample). Matches Stata to 14-15 digits (`cluster2.do`).
- **Numbers:** packages 78.3% of source lines (from 73.1%); probes (423) 196,229 of 210,184 run lines
  (93.4%), none of the 4,336 compared lines differs, 4,325 to 8 digits and 11 to 6.
- **An error ends the loops and programs around it** (`dofile.rs`, `abandon`): checked in Stata 19,
  a line Stata stops at ends the enclosing loops, blocks and program calls (up to a `capture`, whose
  `_rc` is then not known); the file goes on as with nostop. TZEN38's `learnalg` stops in Stata at
  `if $qtr == 0 {` (no file sets `$qtr`), so 186,028 run lines RCraft counted are not run: the
  packages are 28,953 run lines, 92.0% translated (from 64.9%), and 78.6% of 5,187 source lines.
  Probes (425) 196,796 of 210,914 (93.3%), none different.
- **Next:** outreg2's se, tex(), lab, ctitle(), addtext() (48 lines); which; estimates table's options.

## xtset kept in a .dta, xtreg's collinearity, and results after a fit R stops in, 2026-11-06

- **xtset on use** (`dofile.rs` `restore_ts`, `data_r/ts.rs` `DTA_TS_READER`): when the do-file runs,
  a .dta it did not write is asked for the `_TSpanel`/`_TStvar`/`_TSdelta` characteristics Stata keeps,
  and they are set again after `use`, as Stata does. TZEN38's 87 `xtreg` lines had been refused for this.
- **xtreg** (`data_r/xt.rs`), checked in Stata 19: collinear columns omitted, the later one; re with more
  columns than panels has sigma_u = 0; fewer clusters than slopes gives a missing Wald test (in every
  model's e()); fe with a column collinear only within the panels stops in R (Stata's choice not known).
  A variable named twice in a model is declined (Stata keeps an o. column). `xtcollinear.do`.
- **After a fit R stops in:** e() and `_b[]` read after it stop, where they showed the model before.
- **TZEN38 run whole:** 1,567 lines run (from 870), 707 model values the same as Stata's (from 252),
  none different (`plan/stata/xtset-on-use.md`).
- **Probes** (428): 197,083 of 211,500 run lines (93.2%). 11 compared lines differ, all in two probes
  StatCraft added today (510 nbreg with exposure at the alpha boundary; 596 ivprobit twostep in the
  8th digit); main's code gives the same, so they are not from this change. To fix next.
- **Next:** probes 510 and 596; outreg2's se, tex(), lab, ctitle(), addtext(), addstat() (48 lines in
  TZEN38, now reachable when it runs).

## Probes 510 and 596: exposure and offset kept in refits, probit on Stata's path, 2026-11-06

- **Offsets kept** (`data_r/model.rs` `stata_collinear_rhs`): the models fitted again without their
  collinear terms (nbreg, ologit, oprobit, ...) lost `exposure()`/`offset()` and `noconstant`, which
  are not terms of the formula; nbreg with exposure gave a likelihood below Poisson's.
- **exposure() and offset()** (`data_r/ml.rs`, probe 510): Stata's errors and codes (exposure not
  above 0 r(459), both r(198), a variable not there r(111), two r(103), a string r(109)); e(offset)
  (`ln(t)`, or the variable) in Stata's place. A line that only gives Stata's error keeps the results
  before it (the reset of e() is for a line that keeps new ones).
- **probit on Stata's path** (`stata_ml_path`, StatCraft's decoding of Stata's maximize): Newton-Raphson
  from the constant-only fit with Stata's steps and stopping rule; Stata reports that iterate, which can
  be short of the maximum (probe 596 builds ivprobit's two-step from probit fits and showed it). Used
  with a constant and no offset; otherwise the maximum as before.
- **Numbers:** probes (428) none different; 4,336 of 4,348 compared lines to 8 digits (from 4,328),
  12 to 6. Packages 78.6% of source lines.
- **Next:** outreg2's se, tex(), lab, ctitle(), addtext(), addstat() (TZEN38, now reachable when it runs).

## outreg2: se, tex(fragment), label, ctitle(), addtext(), addstat(); backslashes as written, 2026-11-06

- **outreg2** (`data_r/outreg2.rs`, `plan/stata/outreg2.md`): `se`, `.tex` with `tex(fragment)`, `label`,
  `ctitle()`, `addtext()`, `addstat()`, factor-variable models when keep() leaves them out, `a##b` in
  keep(), xtreg's Number of panels and fe's within R-squared; the statistics rows in Stata's order (one
  column, and columns added). 26 files byte for byte against Stata 19; `outreg2tex.do`.
- **TZEN38 run whole:** 27 of its 48 outreg2 lines run (20 follow fe fits R declines); each column it
  writes is the published table's, cell for cell.
- **Backslashes** (`lang/script.rs`): the reader no longer halves `\` on a line; Stata 19 keeps them
  (probe 139's own log; strlen checks). Probe 139: 17/17. A note for StatCraft is in the shared notes.
- **Numbers:** probes (429) none different; 4,338 of 4,350 compared lines to 8 digits, 12 to 6.
  Packages 78.6% of source lines (TZEN38's outreg2 lines run only when the package runs).
- **Next:** xtreg fe with a column collinear only within the panels (which one Stata omits); then the
  next gaps by impact.

## xtreg fe with within-panel collinearity fitted with a note; age age; g#c.x in e(b), 2026-11-06

- **xtreg fe** (`data_r/xt.rs`): which column Stata leaves out of a within-panel collinear set rests on
  rounding (probed on 28 models, `plan/stata/xtset-on-use.md`). RCraft now fits the model, leaving out
  the later column (Stata's choice where the collinearity is exact), and a note names the coefficients
  that may then differ (the set's and _cons); nothing else depends on the choice. Chosen with the user.
- **A variable named twice in xtreg** (`age age`): o.age after age in e(b), as Stata. **g#c.x without g**
  (`data_r/matrices.rs`): every level a column of e(b); a zero base column had been put first.
- **TZEN38 run whole:** 1,771 lines run, no R errors; 846 of 861 model values the same as Stata's (the 15
  others are age or a meeting level in those sets); Tables 1 and 5 the published ones, the others too
  but for _cons in some fe columns. `xtcollinear.do` has the exact case, age age and g#c.x.

## asclogit, drawnorm, _pctile; clogit on Stata's path, 2026-11-06

- **asclogit** (`data_r/clogit.rs`, `plan/stata/asclogit.md`): alternative-specific variables with
  noconstant, as clogit on the cases (Stata's estimates and variance to the last digit); its notes,
  header, e(), alt: in e(b), e(sample) and `estat summarize`.
- **drawnorm** (`data_r/random.rs`): Stata's draws bit for bit (a variable at a time, the deviations in
  float). **_pctile** (`data_r/stats.rs`): r(r1), r(r2), ... with Stata's default percentiles.
- **clogit** follows Stata's Newton-Raphson from an ordinary logit's slopes (one positive a group):
  estimates to 1.2e-12 of Stata's, where they were 6e-7 off.
- **YFBPT6 run whole:** 4,288 of 4,549 model values the same; the 261 others are robreg on simulated
  data, where 1e-12 upstream flips float draws; on the same data Stata's robreg gives RCraft's numbers.
  `asclogit.do` checks the chain.

## svyset and svy: clogit, 2026-11-06

- **svyset psu [pweight=w]** and **svy: clogit** (`data_r/svy.rs`, `data_r/clogit.rs`, `plan/stata/svy.md`):
  the weighted estimates on Stata's path, the variance linearized over the PSUs, the design df and the
  adjusted Wald F, e() as Stata's; predict pc1/pu0 and margins vce(unconditional) after it (weighted means,
  linearized by PSU). `svy.do`.
- **Found on the way:** the model after `svy:` lost fvset's bases (now restated as every command is);
  `ereturn list` and `return list` no longer count as changing the data for margins.
- **YFBPT6 not reached yet** (done 2026-11-07, below): its `weight_rake` comes from rake.do's `survwgt rake` (SSC), which was not
  translated; rake.do also needs `isid`. Next for that package.
- **tempfile** removes a file an earlier run left under the new name (the numbering starts again each
  run, and the app's temp folder is shared), so `save` to a tempfile without replace no longer stops
  a second run (YFBPT6 rake.do:37).

## survwgt rake, isid, abbreviations in expressions, 2026-11-07

- **survwgt rake** (SSC, installed in Stata with the user's OK; observed, its program not read;
  `data_r/survwgt.rs`, `plan/stata/survwgt.md`): raking in Stata's order, its stopping rule for totals
  below a thousand (0.001 on any category's gap; every case observed), maxrep() (10 by default) and its
  warning, the add-on's errors, missing weights and categories, the double "(raked)" variable, the data
  left sorted by the last dimension. With several dimensions and large totals Stata can stop sooner by a
  rule not pinned down: R stops with a message there. `survwgt.do`.
- **isid** with Stata's r(459) messages and missok (`data_r/more.rs`). `survwgt.do`.
- **Abbreviations in expressions**: `keep if country==1` for country_v13, in if, generate and egen's
  expression, replace and assert, as Stata 19 reads them (a variable before a scalar; ambiguous is
  r(111)). The names are asked of R when an expression has a name R has not given as a variable.
- **import delimited** decides a column's type from the whole file, as Stata does (readr looked at 1,000
  rows: a late "NA" made a column of numbers instead of text).
- **YFBPT6:** 18,106 lines ran, 214 refused, 29 stopped in R (were 17,735, 275 and 337). rake.do and
  analysis_weighted.do run every line; their 63 compared values match Stata's (to 2e-7), and the svy
  clogit tables (all nine countries, 81 rows) and Sweden's margins match Stata's to every printed digit (Stata's own reference
  rerun for those two files, since its first run had no survwgt). What is left in YFBPT6: `contract`
  (representative.do), macro.do's chain, graphs.

## contract, recast, isvar, suest, estimates table, codebook and more, 2026-11-07

- **Fixed a regression from 17242ef**: abbreviations in expressions read `egen vmean = rowmean(v*)`'s
  varlist as an expression (`v ambiguous abbreviation`, YFBPT6 macroreg.do, 10 lines). Only egen's
  functions that take an expression are read that way now.
- **contract** (freq, cfreq, percent, cpercent bit for bit, float, format, zero, nomiss, if, in,
  fweights, its errors) and **recast** to number types (truncation, float, out of range, force and
  Stata's messages). `contract.do`, `plan/stata/contract.md`.
- **isvar** (SSC, observed only; it was already in Stata's PLUS folder) with r(varlist) and
  r(badlist), which the translator works out from the data's names, so `r(varlist)' needs no ask.
  **wordcount()** in expressions. `isvar.do`.
- **A fit Stata takes as exact**: regress below about 1e-14 of the total sum of squares gives rss 0,
  R-squared 1, F and ll missing and standard errors of 0, as Stata's; the narrow band where the
  cut-off was not pinned down is refused. `regress_exact.do`. ZNMIXH's own lassoClean program relies
  on `e(r2)==1`.
- **suest** of stored clogits (e(b) with bases and name_depvar equations, the clustered sandwich
  e(V), clogit's groups without vce(cluster)), **margins after it** with dydx() of factor levels and
  an expression() of the equations' predictions, posted, and **test** after it: all match Stata to
  1e-15. **estimates table** (R's layout, r(names)). `suest.do`, `plan/stata/suest.md`.
- **margins, dydx(1.reform 3.opinion)**: the levels asked for, r(b) with their bases as Stata's; and
  **matrix list e(b) after margins, post** of effects alone (R checks the layout). `clogit_margins.do`.
- **codebook** (R's layout, no storage types), **grstyle** (style only, like set scheme), list
  **sepby()**, quoted **matrix rownames/colnames** ("18--24", "65+", spaces), `: var label`,
  **coefl** as Stata's shortest coeflegend, and **compress** and an untranslated **putexcel** no
  longer cost the model or the data that follow.
- **Abbreviations in varlists**: `drop _m` for _merge (ZNMIXH, YFBPT6 agg_merge.do), in every
  varlist command, the data's names asked when a name is not one R has given; ambiguous is Stata's
  error. The Command window asks once per new name.
- **predict after logit and probit with a perfect predictor**: missing on the rows it excluded, in
  the sample or out of it, as Stata's default (ZNMIXH 02_attrition.do's inverse-probability weights
  used 13 rows Stata leaves out); e(b) keeps the omitted variable as a column of 0 where it was
  written. `perfect_predict.do`.
- **A refused command that changes variables in place** (`center x, inplace`, `..., replace` that
  is not a file's): its variables are no longer known after it, so lines that read them are refused
  rather than run on the old values. (ZNMIXH's reference run had no `center`; Stata stopped each
  loop at it, which is why lines 860-892 and 2999-3005 differed.)
- **append** puts a file's `%td` dates in the form the data in memory have (probe 574).
- `robreg m y x` and `ivregress gmm ...`: the estimator's word is not taken as a variable.
- `estimates table` before `suest` no longer costs the stored models; `matrix rownames` with quoted
  names; list `sepby()`; `query` and `grstyle` (nothing to do in R).
- **YFBPT6:** 18,325 lines ran, 20 refused, 4 stopped in R (were 18,106, 214 and 29); all 4,553
  values compared with Stata's log match (the 63 of rake.do and analysis_weighted.do as before).
  analysis_polknow.do runs every line, suest included. Left: esttab's LaTeX options (cells(),
  mlabels() with suboptions, matrix()), cmset/cmclogit with margins over alternatives (2 files),
  rreg, twoway rspike, and two graphs that read a .gph.
- **TZEN38:** 1,923 ran, 371 refused (were 1,771 and 523): an untranslated putexcel keeps the model.
- **ZNMIXH:** isvar makes its own lassoClean program run (110,895 lines with it), but `center` (SSC,
  not translated, and not installed in the Stata that made the reference log) changes its
  variables in place, so what reads them after it is now refused: 4,823 ran. Next for ZNMIXH:
  install center in Stata with the user's OK, observe and translate it, and record Stata's log
  again with it. The loop then asks R thousands of times (26 minutes in the e2e run): worth making
  cheaper.
- Probes: 446, one differing (518-regress-bits, StatCraft's, residual bits); probe 574's append now
  runs.

## center (SSC), 2026-11-08

- **center** installed in Stata with the user's OK and observed (its program not read;
  `data_r/center.rs`, `plan/stata/center.md`, `center.do` to 1e-13): new c_ variables or
  inplace, standardize, casewise, if, aweights and fweights, double, replace, nolabel, prefix(),
  the add-on's float-rounded mean and sd, its labels, messages and errors. `in` is refused (the
  add-on leaves the rows in range unchanged there).
- **ZNMIXH with center:** Stata's reference rerun with center installed (the package copy's
  `ssc install` and `, perm` lines commented out). RCraft runs 101,360 lines and all 569 values it
  reaches match Stata's; the e2e run stops at its 30-minute limit, since lassoClean's pairwise loop
  asks R for the names and e(r2) on every pass. Left: that loop's cost; `eststo ...: reg ... if`
  at 01_analysis.do:1781 stopping in R ("logical subscript too long"); outreg (108 lines); `winsor`
  (SSC, not installed in Stata, so Stata's own run stops there too).

## winsor (SSC), and robust regress with an omitted variable, 2026-11-08

- **winsor** installed in Stata with the user's OK and observed (its program not read;
  `data_r/winsor.rs`, `plan/stata/winsor.md`, `winsor.do` to 1e-13): p() and h(), high and low,
  ties, missing values, if and in, the type, format and label it gives, its errors.
- **regress, robust with a variable omitted for collinearity** stopped in R ("logical subscript too
  long"): sandwich's variance leaves out the omitted row, and the F test indexed it by position.
  Now by name; e(F), _se and e(b) match Stata's (`regress_exact.do`). ZNMIXH 01_analysis.do:1781.
- **ZNMIXH:** 111,019 lines ran, 1,271 refused, 53 stopped in R (36 minutes, the isvar loop); all
  583 values compared match Stata's reference, rerun with center and winsor. Stata still lacks revrs,
  pdslasso, the blindschemes scheme.

## alpha, and outreg left alone, 2026-11-08

- **alpha** with generate(), min(), casewise, std, asis and if: the scale to the bit (a running
  sum rounded to float, as Stata makes it), reversed items, r(k), and with casewise r(alpha),
  r(cov) and r(rho) (`data_r/alpha.rs`, `plan/stata/alpha.md`, `alpha.do`). Without casewise
  Stata's r(alpha) and r(cov) are not matched yet and not kept.
- **outreg, frmttable and which** (still not translated) no longer cost the model, its e(sample)
  or the data after them, as putexcel: they only write a table or look something up. In ZNMIXH 51
  `summarize ... if e(sample)` lines had been refused after each outreg.
- Which items alpha reverses: the first principal factor of the complete rows' correlations
  (squared multiple correlations on the diagonal), loadings oriented to sum above 0; it matched
  Stata in 200 of 200 random data sets, and every scale ZNMIXH builds.
- **ZNMIXH:** 111,175 lines ran, 1,153 refused, 15 stopped in R; all 667 values compared match
  Stata's (were 583). Still not translated there: outreg's tables themselves (the .tex files it
  writes), esttab's LaTeX options, frame, pdslasso and revrs (not installed in Stata either).

## outreg's balance tables, 2026-11-08

- **outreg and frmttable** (SSC, observed; `data_r/outreg.rs`, `plan/stata/outreg.md`,
  `outreg.do`): the balance-table use (a model's coefficient with stars, store, replay, append,
  merge, ctitles, note(""), frmttable of a matrix) and the LaTeX fragment written byte for byte
  as outreg writes it. Other layouts are refused. A stored table is dropped before it is stored
  again, and a table whose last store was refused is not read, so no older table is written in
  its place.
- **mata: mata clear** empties outreg's tables, as in Stata (outreg keeps them in Mata);
  other Mata is still refused. **J(r, c, .)** (a matrix of missing values) in matrix expressions.
- **ZNMIXH:** 111,551 lines ran, 777 refused, 15 stopped in R; all 667 values compared match
  Stata's, and the balance tables RCraft writes (TableI1.tex, TableI2.tex) are byte for byte
  Stata's. Table 2 goes through filefilter (not translated yet) after outreg writes it.

## Merged into main, 2026-11-09

- The two entries below were made on branch `claude/step3-eclass-fileread-matrices-bootstrap` on
  2026-11-05, alongside main's entries above, and merged on 2026-11-09. Their numbers (probes, packages)
  were measured on that branch, before main's later work.

## Step 3 continued: e-class programs, file read, more matrix functions; rt(), rbeta() and the bootstrap options, 2026-11-05

- **ereturn** (`data_r/eclass.rs`, `plan/stata/eclass.md`): `post b V` with `depname()`, `obs()`, `dof()`,
  `properties()` and `esample()` (its variable marks e(sample) and leaves the data), moving the matrices as
  Stata does; `scalar`, `local` (an empty one removes it) and `matrix` (`copy`) in the order `ereturn list`
  shows (StatCraft's probe 147); `clear`; `display` (t with `dof()`, z without, equations, `level()`). `post`
  and `clear` work anywhere, the others only in an e-class program (r(152)); r(503) and r(507) as Stata's.
  e(), `_b[]`, `_se[]` and `matrix list e()` read the posted results; an e-class program with a line not
  translated leaves e() unknown. Declined: `repost`, a V that is not symmetric, predict and the rest after
  posted results. `eclass.do` (22 results); `ereturn` is Verified.
- **file read** (`data_r/files.rs`): `file open ..., read` and `file read h name`, the line in the local
  without its end, r(status) win, unix, mac or none, r(eof) and an empty local past the end, so `while
  r(eof) == 0` loops run; file open and close leave r(). `fileread.do` (15 results).
- **Matrices** (`data_r/mexpr.rs`, `plan/stata/matrices.md`): `#` (eq:name names, numbered across the
  result when the right one is typed in place), submatrices (`A[1..2, 3...]`, by name), `cholesky()`,
  `corr()`, `hadamard()`, `vec()`, `vech()`, `sweep()`, `get(_b)`, `get(VCE)`, `invsym()` of a singular
  matrix (StatCraft's sweep rule), `.` in a typed matrix; `trace()`, `det()`, `diag0cnt()`,
  `issymmetric()`, `matmissing()`, `mreldif()`, `colnumb()`, `rownumb()`. `matrix list` of each form was
  compared with Stata's screen. Found: the inverse of a matrix typed in place has default names, not
  swapped ones. `matfuncs.do` (30 results); eight functions are Verified.
- **rt()** (decoded here from Stata 19's draws): a ratio of uniforms; the next uniform is Stata's after
  every one of 116 draws over 15 df, the draws bit for bit at 11 df and within an ulp of the bound at the
  others. **rbeta()** with a shape of 1 (order statistics at (1,1), (1,2), (2,1), else one uniform
  inverted), exact; other shapes are refused (Jöhnk's method fits a + b <= 1 only in part).
  `variates2.do` (8 results); `rt` and `rbeta` are Verified. `rpoisson()` is still refused.
- **bootstrap** (`data_r/random.rs`): `strata()`, decoded on 13 samples of `bsample, strata()` (sorted by
  stratum, bins within it, a third uniform a row for the order; every sample in Stata's order), in
  `bsample` too, with a size; `saving(f [, replace double])`, the replicates' dataset as Stata writes it,
  r(602) before anything runs; `estat bootstrap` (bc, normal, percentile, all), screen for screen. Fixed:
  `capture bootstrap ...` read `bootstrap` as a statistic. `bsopts.do` (12 results).
- **Also:** a program's begin and end no longer lose e(sample); `bootstrap m = r(mean)` with spaces round
  the `=`; the corpus test takes `RCRAFT_CORPUS_ONLY`.
- **Machine:** this one has R 4.4.3 without ggplot2 and quantreg, so `coefplot.do` and `robreg.do` were
  not run here; every other corpus file passes. StatCraft's probes here (288 run) have no
  compared line differing: 4,110 of 4,114 match Stata to 8 digits and 4 more to 6.
- **Domains, from Stata's manual and checked:** `rbeta()` draws only for a in 0.05 to 1e5 and b in 0.15 to
  1e5, `rt()` for df in 1 to 2^53 - 1 (missing, with no draw, outside); RCraft had drawn outside them.
  `rt()` above 1e10 degrees of freedom is refused: at 2^53 - 1 Stata's acceptance test is not the one
  found.

## P2C.11 continued: rbeta(), from the methods Stata's manual names, 2026-11-05

- **rbeta()** (`data_r/random.rs`, `plan/stata/random-numbers.md`): Johnk's method (a + b <= 1, both below
  1), Atkinson and Whittaker's switching method split at 1/2 (both below 1, a + b > 1), their AS 134 (one
  shape on each side of 1, mirrored when a is the larger), a four-piece method laid out as Stata's G4PE
  for both shapes above 1 (Schmeiser and Babu's B4PE), and the shape-of-1 forms found before. On 556 of
  Stata 19's draws over 40 shapes: 507 bit for bit, every one within 1e-12, and the stream Stata's after
  every one; `variates2.do` checks 1,400 more on fresh seeds. Equal shapes above 1 (rbeta(2, 2)) are
  refused: their method was not matched.
- **Found:** Stata's `%21x` prints the exponent in hexadecimal; three apparent misses were the reading of it.


## filefilter, 2026-11-09

- **filefilter** (`data_r/filefilter.rs`, `plan/stata/filefilter.md`, `filefilter.do`): bytes
  replaced as Stata does, its codes, r(occurrences), r(bytes_from), r(bytes_to) and its errors;
  all probe outputs byte for byte Stata's. ZNMIXH: 111,553 lines ran; Table2.tex (outreg, then
  two filefilters), TableI1.tex and TableI2.tex, written by R from scratch, are byte for byte
  Stata's, and all 667 compared values match.

## Do-file runs faster: ZNMIXH 1,155 s to 355 s, 2026-11-09

ZNMIXH's lassoClean.ado tries every pair of its ~130 variables (`isvar`, `reg`, `if e(r2)==1`):
15,170 regressions. Each took about 70 ms in R where the fit itself takes 1 ms.

- **The estimation sample's copy** (`stata_data`, for margins and anova): `d[rows, , drop = FALSE]`
  on a 4,903 x 222 tibble from `use` went through vctrs once per column, ~17 ms. `stata_slice()`
  (`data_r/model.rs`) slices each plain column and puts its attributes back: the same object
  (`identical()` on every data frame of ZNMIXH's and on made-up columns of each kind), 4 times
  faster. All writers of `stata_data` use it; `e(sample)` over a whole data frame no longer copies
  it to number its rows.
- **The workspace listing** after each line of a script (`Source::Script`) is made at most every
  250 ms, and so is the fetch of a watched table (`Engine::refresh_waiting`); a typed command is
  listed at once as before. A run ends listed.
- **Questions** (`Engine::query`) start at once when R is free, not at the next poll.
- **The do-file window** moves the run on for up to 12 ms a frame rather than one line a frame.
- The e2e harness sends lines and questions as the window does and reports the time in R.
  ZNMIXH: same 111,553 lines, same log, all 667 compared values match, Table2/TableI1/TableI2.tex
  byte for byte Stata's.
- **Then 246 s:** the copy is made only when something reads it (`delayedAssign`, with the data
  and the rows taken at the fit, so it is the same copy): margins, anova, `estimates store` and
  the workspace listing, which now comes at most every 250 ms. A bare trip to R is 0.3 ms; what
  is left is R's own work for each fit (lm, the collinearity check, e()). Same lines, log, 667
  values and tables as before.
- **Then 117 s:** the fit's rows (`stata_fit_rows`) are read from its model frame's own row
  numbers when the data's rows are numbered 1 to n, not made into names and matched as text
  (1.2 ms to 0.03 ms, identical on 300 pairs and on named rows, `subset=`, filtered tibbles, glm
  and weights); `e(sample)` from the same rows; the old results cleared with `ls(sorted = FALSE)`.
  Timed in a live session, R was busy ~6 ms of each 7 ms `reg`: Windows' clock is too coarse to
  time a single statement, so steps are timed 300 at a time. A `reg` is now 4 ms (lm 0.6, e() 0.8,
  vcov 0.4, the collinearity check 0.3, the trip 0.7). The e2e table has each line's time in R
  (`ms`), and the summary the time in questions.
- **Review pass:** `stata_ib()` builds its factor from the distinct values for plain and
  Stata-labelled numbers (factor() wrote every row as text, again for each level margins
  computes): identical on 302 columns of YFBPT6's data and on edge cases; YFBPT6's 162 margins
  86 s to 47 s. An independent review found, and these fix: e(sample) all FALSE for a data frame
  named `e` (the shortcut's local name; now `stata_es`, test
  `a_data_frame_named_e_still_gets_its_estimation_sample`); the Data view not following a short
  do-file's data while its listing waited (`Engine::is_quiet`, used where the app waits for the
  listing); a repaint after the run moves on; parked sessions polled while a listing waits;
  `stata_slice` only for plain tibbles and integer rows (a grouped tibble keeps `[`); the copy
  made where the line runs (`parent.env(environment())`). YFBPT6 4,553 values, TZEN38 as before
  (its 15 differences are older), ZNMIXH 667 and its tables, every line's fate unchanged.

## xtreg, fe: e(r2), e(r2_a), e(rss), e(mss), e(tss), e(rmse), 2026-11-09

- Observed in Stata 19 (`plan/stata/xtreg-r2.md`, corpus `xtreg_r2.do`, all within 1e-10): the
  within R-squared, the sums of squares, and r2_a and rmse on N - k - panels, or N - k - 1 with
  robust or clustered errors. TZEN38: 2,067 lines ran (1,923 before), 227 refused (371): what is
  left is `putexcel` (220 lines), `estpost`/`esttab cells()` (6) and `set mem`. Its values as before
  (846 of 861; the 15 are the within-collinear set Stata chooses by rounding, decided earlier).

## putexcel, 2026-11-09

- **putexcel** (`data_r/putexcel.rs`, `plan/stata/putexcel.md`, `putexcel.do`): set with sheet()
  and modify or replace, text, numbers to 17 digits and ( expressions ) into cells, clear, and
  Stata's errors; R's openxlsx2 (now in CI's packages). TZEN38: 2,287 lines ran, 7 refused (227
  before; `estpost`/`esttab cells()` and `set mem` are left), its values as before, and its
  workbook, written by R from scratch, has Stata's four sheets and all 207 cells the same.
- **set memory** (`set mem`, `set memory`): Stata 19's note "set memory ignored. Memory no longer
  needs to be set in modern Statas; …", r(198) without a number or for one that is not whole.
  TZEN38's `capture set mem 2000m` now runs; its 6 left are `estpost summarize` with
  `esttab, tex cells()`.

## Value labels: attr(, "label") read exactly; label define on a set that exists, 2026-11-09

- R's `attr(v, "label")` matches partially: on a column with value labels (`labels`) and no
  variable label it returned the value labels, and a second `label values` on such a column
  stopped in R (TQN6ZN, twice; its next `tab` showed the old labels). Every read is now
  `exact = TRUE`. Corpus `labels2.do` (a copy of a labelled variable, replaces, a second value
  label, decode): Stata 19's results.
- `label define` on a set that exists without add/modify/replace is Stata's r(110) "label …
  already defined" (it was refused). TQN6ZN: 364 lines ran, 2 refused (pwcorr, o; coefplot, ci),
  the one stop Stata's own r(110); 231 values as Stata's.
- The e2e comparison joins Stata's wrapped `> ` lines first: ASKO93 (241) and TQN6ZN (231) match
  Stata in full (read without joining, 164 and 165 looked different). AXYMDI and SI3BGA never ran
  in Stata (the reference was started with the comma inside the quotes), and their do-files open
  no data; DZ8BUS and XURXZN stop in Stata too (no import; data not in the package).

## xtreg, fe: the within-collinear column Stata omits, clustered on another variable, 2026-11-09

- Observed in Stata 19 (`plan/stata/xtreg-absorbed.md`): the "collinearity with the absorbed
  variable" note, and a column other than the later one, come only with `vce(cluster v)` on a
  variable that is not the panel; Stata then omits the column its largest-pivot sweep of the within
  cross products leaves with nothing. Plain, robust or clustered on the panel it omits the later
  one (as before). RCraft now does both, with Stata's note; corpus `xtabsorbed.do`.
- Where the last two columns are left with the same (meetings of one year), Stata's rounding
  decides and was not reproduced; RCraft omits the earlier and notes the coefficients that may
  differ. TZEN38: 856 of 861 model values as Stata's (846), the 5 others in tied models.

## esttab: stats(), varlabels(), mgroups(), starlevels(), compress, nogaps; estpost summarize and cells(), 2026-11-10

- Observed in Stata 19 (`plan/stata/esttab-options.md`; four probes, RCraft's output the same in all 50
  files and on the screen; corpus `esttab2.do`): `stats()` with `fmt()` and `labels()`, `varlabels()`
  (pairs in turn; no blank lines between the coefficients), `mgroups()` with `pattern()` and, in .tex,
  `prefix() suffix() span`, `starlevels()` (stars as typed), `compress`, `nogaps`, `tex` on the screen,
  esttab's note when it takes eststo's models, and Stata's string() for `%w.df` (12344.19 in %8.3f is
  1.2e+04). `estpost summarize` posts its e() (e(N) the rows where any variable is nonmissing) and prints
  its table; `esttab, cells()` of it.
- TZEN38: every line translated (2,294 ran, none refused; 6 before), its three estpost tables and three
  `esttab, tex cells()` tables as Stata's log. ZNMIXH: 12 more of its .tex tables written, each the same
  byte for byte as the package's own (TableF11 too, its pattern() one longer than the models); the rest
  wait on lines before them, mostly comparisons with extended missing values (`== .d`).
## Extended missing values .a to .z, 2026-11-09

- Comparisons with `.` and `.a` to `.z` go by Stata's order (numbers < `.` < `.a` < ... < `.z`),
  once the data may hold them (a `.dta` read by `use`, `merge` or `append`, or one stored by
  `input`, an expression or `recode`): `stata_mv()` gives a value's place, `stata_compare()`
  compares two values that may both be missing, `inlist()` with a missing value goes value by
  value. `x < .` and `x >= .` stay `!is.na(x)` and `is.na(x)`; without tags the R is as before.
- R keeps haven's tagged NAs; as in Stata 19, copies, rows of a variable, `cond()`, `float()`,
  `int()`, `round()`, `floor()` and `ceil()` keep them, and arithmetic and other functions give
  `.` (`stata_untag()`). `stata_float()` keeps a tag; `.d` as a value is `haven::tagged_na("d")`.
  `recode` by a missing value takes that one only, `(.a/.w = 5)` a range of them, `(1 = .a)` stores
  one. Observed on made-up data (`plan/stata/extended-missing.md`); corpus `extended_missing.do`,
  76 results, all Stata 19's exactly; `cond`, `inlist`, `int`, `round`, `floor`, `ceil` Verified.
- ZNMIXH: `replace church_survey_b=. if church_survey_b==.d | church_survey_b==.w` and the chain
  after it run: 112,167 lines ran (111,553), 163 refused (775), 13 stopped in R (15, the two gone
  were after it); 926 of Stata's 947 `@E` values printed (667), every one as Stata's; Table2,
  TableI1 and TableI2 byte for byte. What is left is mostly `esttab, stats()` (21), `which`,
  `frame` and `graph export`.
- Not yet then: `egen` row functions, `collapse (max)/(min)/(first)`, `sort`, `tabulate, missing`
  and `fre` treated `.a` to `.z` as one NA, and `+x` was accepted; done 2026-11-10 (below).

## Extended missing values in commands, 2026-11-10

- Where the data may hold `.a` to `.z`, these commands now treat them as Stata 19 does
  (observed on made-up data, `plan/stata/extended-missing.md`):
  - `sort` and `gsort` order `. .a ... .z` after the numbers; `gsort -x` puts them last in
    descending order, `mfirst` first.
  - `tabulate`, `tab1` and `tab2` with `missing`: a row and column for each, a labelled one
    shown by its label. `tabulate, generate()` makes an indicator for each.
  - `fre`: a Missing row for each and their Total. It no longer refuses after one was stored.
  - `egen group(), missing` and `tag(), missing`: each missing value is a group of its own.
  - `egen rowmax()` and the new `rowfirst()`: a row with no number gives the last variable's
    value. `rowlast()` (new) gives the first's; `rowmin()` gives `.`.
  - `collapse (max)`, `(min)`, `(firstnm)` and `(lastnm)` of a group with no number give its
    largest, smallest, first and last missing value.
  - `recode` ranges from a number to a missing value go by Stata's order (`(1/.b = 9)`). From a
    missing value to a number they change nothing.
- `+x`, `2^+x`, `+(x)` are refused as Stata refuses them, r(198) and r(133). A `+` goes only
  before a number written out.
- Three bugs fixed along the way, missing values or not:
  - `egen tag(), missing` gave NA past the first row.
  - `fre` printed a grand Total when no value was valid.
  - `fre` did not pad values to one width under a label set (`1  one`, `10 ten`).
- Corpus `extended_missing_commands.do`: 75 results, all Stata 19's exactly. Verified for
  `collapse`, `egen`, `fre`, `gsort`, `input`, `recode`, `sort`, `tab1`, `tab2` and `tabulate`.
  The `tabulate` and `fre` tables were also compared with Stata's log line by line.
- Not yet then: `by` groups, `collapse, by()`, `merge` keys, `duplicates`, `levelsof` and
  `contract`; done the same day (below).

## Extended missing values as groups, 2026-11-10

- Where the data may hold `.a` to `.z`, each of `.`, `.a`, ... `.z` is a group of its own, in
  Stata's order, as Stata 19 does (`plan/stata/extended-missing.md`): `by` and `bysort` (grouped
  steps and the by-each loop of statistics and models, its `-> g = .a` headings),
  `egen ..., by()`, `collapse, by()`, `duplicates drop/tag/report`, `levelsof, missing`,
  `contract` and `merge` keys (`.a` matches `.a`; a `.z` only in the using data is not matched).
- Corpus `extended_missing_groups.do`: 27 results, all Stata 19's exactly. Verified for
  `duplicates` (new), and for `collapse`, `contract`, `egen`, `input`, `levelsof` and `merge`
  with these values.
- Not yet: `contract, zero` with `.a` to `.z` stops with a message. `isid, missok`,
  `egen rank()` and `xtile` were not observed with them.

## esttab: prehead() posthead() prefoot() postfoot(), fragment, erepeat(), mlabels(), alignment(), 2026-11-10

- Observed in Stata 19 (`plan/stata/esttab-options.md`, second part; three probes, RCraft's output the
  same in all 43 files and on the screen; corpus `esttab3.do`): the table as seven parts, each of
  prehead, posthead, prefoot and postfoot replaced by its option's lines (@title in prehead());
  `fragment` in .tex; `mgroups(..., erepeat())`; `mlabels()` with and without span, prefix() suffix(),
  none, depvars; `alignment()` in .tex's tabular; `nonumber` and `addnote` as written.
- Two differences from Stata fixed along the way: `addnotes()` with the default notes goes between the
  first note and the stars' (it came last), and `noobs` in .tex ends the coefficients with the bottom
  rule (it gave `\hline`). A single quoted `addnote("a b")` or `mtitles("a b")` is no longer split at
  its spaces.
- ZNMIXH: 136 lines refused (139); 26 of its 30 tables written, all byte for byte as Stata's (TableF4,
  F5 and F9 new); the other 4 wait on models not stored before them. YFBPT6: 17 refused (20);
  table_robreg.tex, table_robreg1.tex and table_valreg.tex written, byte for byte as Stata 19 wrote them
  from the same data. Its four `alignment()` tables now stop at `sfmt()` and `cells()` of clogit models;
  `esttab matrix()` (2) is not translated yet.

## capture which, frame create/change/drop/pwf, set obs's message, 2026-11-10

- Observed in Stata 19 (`plan/stata/which-frames.md`; corpus `which.do`, `frames.do`):
  - `capture which name` sets _rc (0 found, 111 not) and prints nothing; RCraft answers for itself
    (found when it translates the command, estout for its esttab, eststo, estadd and estpost).
  - `frame create`, `change`, `drop` and `pwf`: each frame its own data, labels and settings, e()
    shared, _rc 110, 111 and 119 as Stata gives them; preserve is each frame's own.
  - `set obs` prints "Number of observations (_N) was 0, now 5." (commas from 1,000), which RCraft had
    left out; fewer than there are is r(198).
- preserve and restore now keep the variables' storage types too (they had kept the rest).
- ZNMIXH: 97 lines refused (136 before this round, 139 before the esttab work); its package check
  and its frame blocks run; 26 of 30 tables written, all byte for byte as Stata's.

## capture drop of what is not there, string() with a format, 2026-11-10

- Observed in Stata 19 (`plan/stata/which-frames.md`; corpus `captured.do`): a `capture drop` or
  `capture keep` of a name or pattern the data does not have is r(111) and keeps r() and the data;
  RCraft now does the same where it knows the variables (it had refused a pattern and lost r() for a
  name). `string()` and `strofreal()` write Stata's format (`%9.0g` without one), as Stata does.
- `which` and `frame create/change/drop` keep r() (as Stata 19 does); `frame pwf` does not.
- ZNMIXH: 69 lines refused (97), none stopped in R (15): the `cap drop` before `g x = y > r(p50)` and
  the `cap drop Z*var` loops now run as in Stata.

## revrs and tabm (SSC), tabulate's long column labels, 2026-11-10

- revrs and tab_chi installed from SSC in Stata 19 with the user's yes; observed with made-up data
  (`plan/stata/revrs.md`, `plan/stata/tabm.md`; corpus `revrs.do`, `tabm.do`), their code not read.
  - `revrs`: a labelled variable's labelled values take the codes k ... 1 with label set rev<name>;
    without a label (max + 1) - x; `replace`; Stata's r(111), r(109), r(110), and its silence when
    rev<name> is there already for a labelled variable without replace. Values that are not whole
    numbers or halves are refused (Stata's float arithmetic there was not matched).
  - `tabm`: tabulate's table of variable by values, the first variable's value label on the columns;
    `if`, `missing`; r(N), r(r), r(c).
- tabulate: a two-way table's column labels longer than 9 characters are cut to 9 as Stata does (such
  tables had been printed in R's layout).
- ZNMIXH: 39 lines refused (69); 29 of 30 tables as Stata's. ASKO93: its tabm table as Stata's; the
  13 anovaplot lines wait on anovaplot, which is not on SSC.
- ZNMIXH's TableF2 (new with revrs) differs from the copy shipped in the package in its first two
  columns (Treatment 0.037 there, -0.120 here; R-squared 0.04, 0.09). Stata 19 itself, run on its own
  build of analysis_clean_intermediate.dta (01_analysis.do lines 1-89) and lines 2723-2785 with today's
  revrs, gives -0.12041813 (se .07312435, N 191, R-squared .09375217): what RCraft writes. The shipped
  table was made otherwise (perhaps another revrs version); RCraft matches Stata 19 here. The other 29
  tables are byte for byte as shipped.

## anovaplot (Stata Journal gr0009_2), 2026-11-10

- Installed from the Stata Journal (SJ23-1) with the user's yes; observed with made-up data, its
  graphs exported as SVG and PNG (`plan/stata/anovaplot.md`), its code not read. After anova: the
  model's fitted values over its own rows, joined along x for each value of g and marked at each cell,
  the data as points (hidden by scatter(ms(i)), which still set the y axis), x's value labels as its
  ticks, Stata's titles and "Profiles by" g's label above; drawn by ggplot2. ASKO93's first Figure 2
  panel, drawn by both from its data, shows the same lines, labels and titles.
- ASKO93: every line translated (494 ran, none refused; 27 before). Of the eight packages run, ASKO93,
  SI3BGA and TZEN38 refuse nothing.

## coefplot: levels(95 90), asequation swapnames eqrename(), mlabel(string(@b)), legend(order()), 2026-11-10

- Observed in Stata 19 (`plan/stata/coefplot.md`, last part; corpus `coefplot2.do`): two levels' intervals
  (gen()'s ll2 ul2), swapnames' rows (a group per coefficient, a row per plot), eqrename()'s row names,
  marker labels of b in a format, and legend(order()) keys (L + 1 to a plot). The probes' graphs drawn
  by both show the same rows, intervals, labels and legend.
- ZNMIXH: 23 lines refused (39); all eight of its coefplot figures translated. Left: pdslasso (2, with
  about 18 lines after it), twoway's text() and graph bar (percent).

## graph bar (percent), over(..., sort() descending), blabel(bar), hbar's order, twoway text(), 2026-11-10

- Observed in Stata 19 (`plan/stata/graph-bar-text.md`): `(percent)` without variables (each group's
  share, rows with the over() variable missing left out), `over(v, sort(var|#) descending)`,
  `blabel(bar[, format()])`, the value axis titled with a single series and no legend for it, and
  `text(y x "text")` on any graph. Two differences fixed along the way: `graph hbar` drew its first group
  at the bottom (Stata's is at the top), and over()'s sort() had been ignored. The probes' graphs drawn by
  both show the same bars, order, labels and texts.
- ZNMIXH: 20 lines refused (23), all of them pdslasso (2) and the lines in 02_attrition.do after it.

## prtest, graph bar (asis), stacked bars' order, b1title(), 2026-11-10

- Observed in Stata 19 (`plan/stata/prtest.md`; corpus `prtest.do`): `prtest var, by(g)` (Stata's
  table and its 20 r() values; r(450) and r(198) as Stata gives them; AXYMDI's two tables as in its
  Stata log), `graph bar (asis)` (r(459) for a group of several rows), `legend(order())` for bars, and
  `b1title()`. Fixed along the way: stacked bars put the first variable on top (Stata's is at the
  bottom).
- AXYMDI: every line translated (46 ran, none refused; 3 before). ASKO93, AXYMDI, SI3BGA and TZEN38
  refuse nothing.
- Builds after a clock change: the machine's clock went back an hour and twenty minutes when the session
  restarted, so cargo took older edits for built and skipped them; `cargo clean -p` of RCraft's crates
  rebuilt them (CI, building from scratch, was not affected).

## pdslasso (rigorous lasso, post-double selection), twoway rcap and rspike, m(), 2026-11-11

- pdslasso installed from SSC with ivreg2 and ranktest, which it needs (lassopack was there), each with
  the user's yes; observed with made-up data and ZNMIXH's (`plan/stata/pdslasso.md`; corpus
  `pdslasso.do`), its code not read. RCraft's rigorous lasso reproduces rlasso's loadings, lambda and
  estimates to 1e-9 and its selections exactly; pdslasso's e(b) and e(V) (OLS on the union, HC0), its
  CHS estimates and its printed output follow. On ZNMIXH's 174 controls it selects what Stata does and
  its estimates agree to 5e-11.
- twoway `rcap` and `rspike`; `m()` as msymbol().
- ZNMIXH: every line translated (112,336 ran, none refused; 139 refused on 2026-11-10 morning). Five
  of the eight packages refuse nothing: ASKO93, AXYMDI, SI3BGA, TZEN38, ZNMIXH.
- The clock went back again at a session restart (13:25, the build files at 15:23): RCraft's crates
  cleaned and rebuilt before testing.

## esttab cells() side by side, refcat(), eqlabels(none), @M; rspike horizontal; bgcolor(), 2026-11-11

- Observed in Stata 19 (`plan/stata/esttab-options.md`, last part): models' b and se side by side in
  cells(), refcat() heading rows, eqlabels(none), @M in prehead(), words outside quotes as lines, compound
  quotes in labels; esttab of one-equation models (asclogit). YFBPT6's rawestimates1.tex and
  rawgovestimates.tex are now written byte for byte as Stata's.
- twoway rcap and rspike, horizontal; bgcolor() as a look option.
- YFBPT6: 14 lines refused (17): the cmclogit chain (cmset, cmclogit, margins after it, and what reads
  them), rreg, and esttab matrix() (2). rreg: its iteration log shows a scale about 0.3% larger than
  MAD/0.6745, not yet pinned down, so it stays refused.

## P6.1 Panel and fixed effects (first part), 2026-11-12

- **Statistics > Panel models** (`catalog::panel`), four dialogs, each a spec as in Phase 4, that keep the
  model under a chosen name (but the Hausman test) and print the package's own summary:
  - **Fixed-effects regression:** `fixest::feols(y ~ x | fe1 + fe2)`, any number of absorbed fixed
    effects; errors clustered by the first fixed effect (fixest's default), conventional, robust,
    two-way clustered, or clustered by a chosen variable; confidence intervals.
  - **Fixed-effects count or binary model:** `fixest::fepois` or `fixest::feglm(family = "logit")`,
    an exposure as an offset for Poisson, rate or odds ratios with intervals.
  - **Panel model:** `plm::plm` with the unit and time as `index`: within, random, between, pooled or
    first differences, effects for units, periods or both, errors conventional or clustered by unit or
    period (`plm::vcovHC` in the summary).
  - **Hausman test:** the model fitted within and random, then `plm::phtest`, nothing kept.
  Their checks: the outcome, predictors, fixed effects, unit and time each in one role, two fixed
  effects for two-way clustering, an exposure for Poisson only, and no two-way effects for between or
  first differences.
- **Checked in real R** (`engine/tests/panel.rs`, made-up balanced panel of 6 units over 5 years): each
  report the same as the call written by hand; feols the unit and year dummy fit to 1e-10 (and its
  conventional standard error lm's), fepois and the logit the dummy-variable glm (1e-7, 1e-6), plm's
  within the unit-dummy fit, between lm on the units' means, first differences lm on the differences,
  clustered errors vcovHC's, and the Hausman statistic worked out from the two fits. Snapshot tests of
  the R for each form (`catalog::panel::tests`); every help page's function links found in R.
- **Packages:** fixest 0.12.1 and plm 2.6.5 from the pinned snapshot (2025-03-15), installed on this
  machine with the user's go-ahead (with ggplot2 and quantreg, which `coefplot.do` and `robreg.do`
  need) and added to CI's package step.
- **Not yet:** dynamic panels (GMM), instrumental variables in panels, unit-root and serial-correlation
  tests, and the Objects pane's Model menu (fit, comparison, predictions) on fixest and plm models.

