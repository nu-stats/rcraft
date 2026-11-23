# P2A.6 RCraft manual (open → done) · 5 h · no installs

**Build:** the manual as pages inside the app, searchable from the Help box.

**The pages:** Markdown files under `docs/manual/`, compiled in with `include_str!`, written for RCraft in plain words:
1. Getting started: finding R, the panes, the first data file, the first model.
2. The menus: File, Data, Statistics, Graphs, Reports, Packages, Stata mode, Help; what each area holds.
3. The Command window: history, Tab completion, Stata mode.
4. Packages: find, install, update, project package lists.
5. Projects and reporting: Save project, the session's report, Export table, Save session as.

**How they show:**
- In `crates/ui-egui/src/help_window.rs`, as a new content kind beside R help pages: headings, paragraphs, lists and code. There are no images in the first version.
- Help > RCraft manual opens the contents page.
- `help rcraft` and a search word find manual pages first, then R's help.

**Catalog:** turn `planned("RCraft manual", Area::Help)` into `verified`, once the test below passes.

**Test:** `manual_pages_are_found_by_search`, a unit test in `help_window.rs`. Every page loads, every internal link resolves, and searching "project" returns the Projects page first.

**Done:** the entry is `Verified` and the test is live. P8.4 follows: the owner reads it.
