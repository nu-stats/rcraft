# Status

Last updated: 2026-10-26.

**Phase 0 (setup): scaffolded, not yet committed.** The Cargo workspace,
layered crates, licence files, `NOTICE`, `cargo xtask ci` and the CI workflow
exist on branch `p0/scaffold`.

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
