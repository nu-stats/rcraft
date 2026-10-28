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
  Done 2026-10-28 with that go-ahead, on StatCraft branch
  `lang/shared-parsers` (not yet merged or pushed): `statcraft-lang` now
  holds the do-file reader (`script`), the text-only command syntax
  (`syntax`: `parts`, weights, `options`, `abbrev`, `wildcard`) and the
  expression parser (`expr`), and depends only on `statcraft-core`. P2C.3
  adds it as a git dependency once that branch is pushed; `statatr` (L3)
  is the crate that uses it.
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

- No HTTP client yet. `ureq` with rustls is the candidate; its dependency tree
  needs a licence check first, in a commit of its own, and `docs/network.md` gets
  the exact paths in the same commit as the first real fetch.
- No UI, no mirror setting, no command in the engine.
