## ROADMAP

- P2A.6 RCraft manual (done 2026-11-28): five pages of plain words under `docs/manual/` (getting started, the menus, the Command window, packages, projects and reporting) and a contents page, compiled into the program and shown in the Help window. Help > RCraft manual and `help rcraft` open the contents with no R session; a search lists the manual's pages first, then R's help pages. The owner reads it in P8.4.

## STATUS

## P2A.6: RCraft manual, 2026-11-28

- **Done** (`docs/manual/*.md`; `crates/ui-egui/src/manual.rs`; `crates/ui-egui/src/help_window.rs`; the Help menu in `app.rs`; the catalog entry `RCraft manual` is now Verified):
  - The pages are Markdown, read by a small reader (headings, paragraphs, lists, code, links, bold and code in text). A link goes to another page or, as `help:topic`, to R's help page for a topic. The Help window shows a page with a Contents link at its top; Back and Forward work as for every other page.
  - `help rcraft` and `?rcraft` open the contents without asking R; a topic called `rcraft` in another package is still R's. A search shows the manual's pages before R's hits, and still shows them when R has none.
  - Every statement in the pages was checked against the menus and the code of this version: the menu entries, the tabs of the Packages window, the keys of the Command line, what Save project and Save session as write, and the report and export entries.
  - Test `manual_pages_are_found_by_search` (`help_window.rs`): every page loads and starts with its title, every link resolves, the contents list every page, searching "project" returns the Projects page first, `help rcraft`, `?rcraft` and `help(rcraft)` open the contents and ask nothing of R, a search answer puts the manual's pages before R's, and every page draws without a session. Three smaller tests in `manual.rs` cover the reader and the ranking.
- **On hold, to be built**: pictures in the pages (the first version has none); a page for each dialog area (Data, Statistics, Graphs) beyond the one-paragraph summaries in the menus page; a page on the data editor and the Properties pane; a search that matches inside R's help text and the manual together in one ranking; the manual as a PDF (P2A.10).
