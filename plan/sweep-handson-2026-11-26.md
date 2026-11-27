# Hands-on / picture pass, 2026-11-26

Second picture pass with `rcraft-desktop --picture` (the real window drawn to a PNG, the user's
R 4.6.2, made-up data), on main at dc3c075, after the first pass in
`plan/sweep-pictures-2026-11-26.md`. Sizes 1240x780, 1024x640 and 820x560; light and dark; the
accents Teal, Indigo, Plum, Forest, Amber and Rose. Nothing was installed or downloaded.

| Where | What the picture showed | Result |
|---|---|---|
| Menu bar, 820 wide | the tagline pushed Reports, Packages, Session, Help and the R version off the right edge | real, fixed (SW20) |
| Menu bar, 1024 wide | "R 4.6.2" printed on top of "Help" | real, fixed (SW20) |
| Graph tab toolbar, 820 wide | Fit, zoom, Copy, Export and Save sat on top of "Plot 1 of 1" and Newer (Newer hidden) | real, fixed (SW21): the buttons take a row of their own below 720 points |
| Do-file window, 820 wide | wider than the screen: close and Change... off the edge (the first pass's open item) | real, fixed (SW22): sized from the screen, the Runs in folder, the buttons and the missing-file rows wrap |
| Help window, 820x560 | 560 tall drawn 60 down: the last lines and bottom edge off the screen | real, fixed (SW23) |
| Left panel, 820x560 | the variable list took 42% and pushed the Properties rows off the bottom, History hidden | real, fixed (SW23): the list gives way, at least 60 points (about 2.5 rows; it scrolls) |
| Packages window (installed, find) at 1024 and 820 | tabs, list, search, buttons all reachable | fine |
| Use my file (csv) at 1024 and 820, light and dark | all rows, preview, Import/Help/Cancel reachable | fine |
| Menu dialogs (Sort rows, New variable, Recode, Value labels, Variable label, Filter rows) at 820x560 and 1024x640, light/dark, Plum, Amber | fit, Run/Cancel/Help visible | fine (SW19 holds) |
| Data tab: small, 40 rows, 200,000 rows, 41 columns with a long text column, labelled values and a column with a variable label | grid, horizontal scroll, "Show labelled values as" (Labels shows low/high), Properties, conversion report | fine |
| Menus at 820x560: File, Edit, Data (disabled with no data), Statistics and a submenu, Session | all fit; Statistics is 18 rows and ends 8 points above the bottom edge (it would not fit a window under about 540 tall) | fine, noted |
| Dark mode and the other accents on all of the above | contrast and accent colours read well (Run buttons, tabs, "ran" marks) | fine |
| Stata mode: results of a Stata session, do-file run (820, dark, Plum), differences guide (general, `regress`), at 820 and 1024 | text wraps, no cut-off beyond the Help fix above | fine |
| Data tab with `--data` and a plot in the demo | shows the Graph tab | picture tool only (a plot switches to Graph), as before |

## Not reachable by picture (listed)

- **Project open and save** (File > Open project, Save project): both go through the system
  file chooser, which the picture tool cannot draw and a headless run cannot answer; there is no
  RCraft-drawn project window. The File menu entries and the disabled "Open recent project" were
  seen. The save, open and refuse paths have the test `a_project_saved_and_opened_again_brings_the_work_back`.
  A real click-through with the chooser still needs a person.
- **Editing in the data grid**: the grid is read-only by design; edits are the Data menu dialogs
  (New variable, Recode, Value labels, Variable label, Rename, Rows), which were drawn above. There
  is no "add a row" or "add a column" by typing in the grid.
- **Tab completion list** (Stata and R): the picture tool cannot press Tab, so the popup was not
  drawn. The completion code has its own tests; a picture option (`--press KEY`) would be a
  few lines if wanted.
- **Drag and drop, Print, resizing by dragging, hover text**: not drawable.
- **Plot size in a very short pane**: at 820x560 with the new two-row Graph toolbar the plot fits
  at about 11% (a 100-point-tall pane). It stays usable (Export and Save give the full size) but
  small; dragging the Script panel shorter gives it room. Listed, low.
