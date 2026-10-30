# Status

Last updated: 2026-10-29.

**Where things are:** Phases 0, 1 (except P1.2 and P1.6), 1A, 3 and 4 are built; Phase 5 and the
first three Stata translator tasks (P2B.1 to P2B.3) too. `plan/AUDIT.md` (2026-10-29) lists every
roadmap task against the code and what is left. The sections below are a dated log, oldest first.

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
